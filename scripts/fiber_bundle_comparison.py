#!/usr/bin/env python3
"""纤维丛 vs 直积配置空间对比脚本。

236号谱系：纤维丛集成——直积近似 vs 纤维丛精确解的策略偏差量化。

获取四标的（SPY/GLD/TLT/UUP）日线数据，用 BiEngine 计算逐 bar 走势方向，
构建直积配置和纤维丛配置，量化两者的策略信号差异。

输出 JSON 到 tmp/fiber-bundle-comparison.json。
"""

from __future__ import annotations

import json
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

# 确保 src 在路径中
_PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_PROJECT_ROOT / "src"))

import math

import yfinance as yf

from newchan.bi_engine import BiEngine
from newchan.types import Bar
from newchan.topology.config_space import Configuration, WalkDirection, polarity_index
from newchan.topology.fiber_bundle import (
    BasePoint,
    default_connection,
    default_fiber_bundle,
)
from newchan.topology.fiber_pipeline_adapter import (
    FiberPipelineAdapter,
    compute_fiber_correction,
)


# ── 标的定义 ──────────────────────────────────────────────

TICKERS = {
    "E": "SPY",   # 权益
    "C": "GLD",   # 商品（黄金）
    "R": "TLT",   # 利率（长债）
}
UUP_TICKER = "UUP"  # 美元指数（现金代理，用于交易日对齐）

START_DATE = "2007-03-01"
END_DATE = "2026-02-27"


# ── 数据获取 ──────────────────────────────────────────────


def fetch_daily_data(ticker: str) -> list[Bar]:
    """获取日线数据并转换为 Bar 序列。"""
    print(f"  获取 {ticker} 日线数据 [{START_DATE} ~ {END_DATE}]...")
    df = yf.download(ticker, start=START_DATE, end=END_DATE, auto_adjust=True)
    if df.empty:
        raise RuntimeError(f"无法获取 {ticker} 数据")
    # yfinance 可能返回 MultiIndex columns — 扁平化
    if hasattr(df.columns, "levels") and len(df.columns.levels) > 1:
        df.columns = df.columns.get_level_values(0)
    bars: list[Bar] = []
    for ts, row in df.iterrows():
        bars.append(Bar(
            ts=ts.to_pydatetime().replace(tzinfo=timezone.utc),
            open=float(row.iloc[row.index.get_loc("Open")]) if "Open" in row.index else float(row.iloc[0]),
            high=float(row.iloc[row.index.get_loc("High")]) if "High" in row.index else float(row.iloc[1]),
            low=float(row.iloc[row.index.get_loc("Low")]) if "Low" in row.index else float(row.iloc[2]),
            close=float(row.iloc[row.index.get_loc("Close")]) if "Close" in row.index else float(row.iloc[3]),
        ))
    print(f"    → {len(bars)} bars")
    return bars


# ── 走势方向序列 ──────────────────────────────────────────


def compute_direction_series(bars: list[Bar]) -> list[int]:
    """用 BiEngine（new 笔模式）计算逐 bar 走势方向序列。

    方向编码：
      +1 = 当前所处笔方向为 up
      -1 = 当前所处笔方向为 down
       0 = 笔尚未形成（数据初始阶段）

    返回长度等于 bars 的方向序列。
    """
    engine = BiEngine(stroke_mode="new")
    directions: list[int] = []
    for bar in bars:
        snap = engine.process_bar(bar)
        if snap.strokes:
            last_stroke = snap.strokes[-1]
            if last_stroke.direction == "up":
                directions.append(1)
            else:
                directions.append(-1)
        else:
            directions.append(0)
    return directions


# ── 交易日对齐 ──────────────────────────────────────────


def align_trading_days(
    bars_dict: dict[str, list[Bar]],
    directions_dict: dict[str, list[int]],
) -> list[dict]:
    """按交易日对齐所有标的的方向。

    返回列表，每项 = {"date": str, "E": int, "C": int, "R": int}。
    """
    # 构建 date→index 映射
    date_maps: dict[str, dict[str, int]] = {}
    for key, bars in bars_dict.items():
        date_maps[key] = {
            b.ts.strftime("%Y-%m-%d"): i for i, b in enumerate(bars)
        }

    # 取交集日期
    all_dates = set(date_maps["E"].keys())
    for key in ("C", "R"):
        all_dates &= set(date_maps[key].keys())
    sorted_dates = sorted(all_dates)

    aligned: list[dict] = []
    for date_str in sorted_dates:
        row = {"date": date_str}
        for key in ("E", "C", "R"):
            idx = date_maps[key][date_str]
            row[key] = directions_dict[key][idx]
        aligned.append(row)
    return aligned


# ── 比较分析 ──────────────────────────────────────────────


def run_comparison(aligned_data: list[dict]) -> dict:
    """运行直积 vs 纤维丛比较分析。"""
    adapter = FiberPipelineAdapter()
    fb = adapter.fb

    # 统计变量
    total_days = len(aligned_data)
    polarity_diff_count = 0
    kl_values: list[float] = []
    correction_magnitudes: list[float] = []
    config_counts: Counter = Counter()
    polarity_diff_dates: list[str] = []
    base_point_kl: dict[str, float] = {}

    for row in aligned_data:
        e, c, r = row["E"], row["C"], row["R"]

        # 构建直积配置
        config = Configuration(
            sigma_e=WalkDirection(e),
            sigma_c=WalkDirection(c),
            sigma_r=WalkDirection(r),
        )
        config_counts[(e, c, r)] += 1

        # 纤维丛修正
        correction = adapter.correct(config)

        # KL 散度
        kl_values.append(correction.kl_divergence)
        correction_magnitudes.append(correction.correction_magnitude)

        # polarity 差异
        if correction.polarity_flipped:
            polarity_diff_count += 1
            polarity_diff_dates.append(row["date"])

        # 按底空间点汇总 KL
        base_key = f"({e},{c})"
        if base_key not in base_point_kl:
            base_point_kl[base_key] = correction.kl_divergence

    # 全局度量
    global_kl = adapter.global_kl_divergence()
    d_eff = adapter.effective_dimension()

    # 差异最大的市场状态
    base_kl_sorted = sorted(base_point_kl.items(), key=lambda x: -x[1])

    # 配置频率
    config_freq = {
        str(k): v for k, v in sorted(
            config_counts.items(), key=lambda x: -x[1],
        )
    }

    # KL 统计
    kl_mean = sum(kl_values) / len(kl_values) if kl_values else 0.0
    kl_max = max(kl_values) if kl_values else 0.0

    # 修正幅度统计
    mag_mean = (
        sum(correction_magnitudes) / len(correction_magnitudes)
        if correction_magnitudes
        else 0.0
    )
    mag_max = max(correction_magnitudes) if correction_magnitudes else 0.0

    return {
        "metadata": {
            "start_date": START_DATE,
            "end_date": END_DATE,
            "total_trading_days": total_days,
            "tickers": {"E": "SPY", "C": "GLD", "R": "TLT"},
        },
        "global_metrics": {
            "fiber_bundle_kl_from_product": round(global_kl, 6),
            "effective_dimension_D_eff": round(d_eff, 4),
            "connection_beta_er": round(fb.connection.beta_er, 4),
            "connection_beta_cr": round(fb.connection.beta_cr, 4),
        },
        "polarity_comparison": {
            "days_with_different_polarity": polarity_diff_count,
            "diff_percentage": round(
                polarity_diff_count / total_days * 100, 2,
            ) if total_days > 0 else 0.0,
            "sample_diff_dates": polarity_diff_dates[:20],
        },
        "kl_divergence_stats": {
            "mean": round(kl_mean, 6),
            "max": round(kl_max, 6),
        },
        "correction_magnitude_stats": {
            "mean": round(mag_mean, 6),
            "max": round(mag_max, 6),
        },
        "base_point_kl_ranking": [
            {"base_point": k, "kl": round(v, 6)}
            for k, v in base_kl_sorted
        ],
        "config_frequency_top10": dict(list(config_freq.items())[:10]),
    }


# ── 主流程 ──────────────────────────────────────────────


def main() -> None:
    print("=" * 60)
    print("236号 纤维丛 vs 直积配置空间对比")
    print("=" * 60)

    # 1. 获取数据
    print("\n[1/4] 获取四标的日线数据...")
    bars_dict: dict[str, list[Bar]] = {}
    for key, ticker in TICKERS.items():
        bars_dict[key] = fetch_daily_data(ticker)

    # 2. 计算走势方向
    print("\n[2/4] 用 BiEngine（new笔模式）计算走势方向...")
    directions_dict: dict[str, list[int]] = {}
    for key in ("E", "C", "R"):
        ticker = TICKERS[key]
        print(f"  计算 {ticker} 走势方向...")
        directions_dict[key] = compute_direction_series(bars_dict[key])
        n_up = sum(1 for d in directions_dict[key] if d == 1)
        n_down = sum(1 for d in directions_dict[key] if d == -1)
        n_flat = sum(1 for d in directions_dict[key] if d == 0)
        print(f"    → UP={n_up}, DOWN={n_down}, FLAT={n_flat}")

    # 3. 对齐交易日
    print("\n[3/4] 对齐交易日...")
    aligned = align_trading_days(bars_dict, directions_dict)
    print(f"  对齐后 {len(aligned)} 交易日")

    # 4. 运行比较
    print("\n[4/4] 运行直积 vs 纤维丛比较分析...")
    result = run_comparison(aligned)

    # 输出
    output_path = _PROJECT_ROOT / "tmp" / "fiber-bundle-comparison.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)
    print(f"\n结果已写入：{output_path}")

    # 摘要
    print("\n" + "=" * 60)
    print("摘要：")
    gm = result["global_metrics"]
    pc = result["polarity_comparison"]
    kl = result["kl_divergence_stats"]
    cm = result["correction_magnitude_stats"]
    print(f"  总交易日：{result['metadata']['total_trading_days']}")
    print(f"  D_eff = {gm['effective_dimension_D_eff']}")
    print(f"  全局 KL = {gm['fiber_bundle_kl_from_product']}")
    print(f"  Polarity 差异日：{pc['days_with_different_polarity']} ({pc['diff_percentage']}%)")
    print(f"  KL 均值/最大值：{kl['mean']} / {kl['max']}")
    print(f"  修正幅度均值/最大值：{cm['mean']} / {cm['max']}")
    print(f"\n  差异最大的底空间点（KL 排序）：")
    for item in result["base_point_kl_ranking"][:5]:
        print(f"    {item['base_point']}: KL = {item['kl']}")
    print("=" * 60)


if __name__ == "__main__":
    main()
