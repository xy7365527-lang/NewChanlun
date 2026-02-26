#!/usr/bin/env python3
"""信号时效性分析 — 买卖点信号确认后的走势完成度评估。

分析维度：
- 第一类买卖点后走势类型是否按预期完成
- 第二类买卖点的次级别回调/反弹幅度分布
- 第三类买卖点后中枢离开段的力度分布
- 各类信号的 MAE/MFE 分布

用法：
    python scripts/signal_timeliness.py

输出：tmp/signal-timeliness-report.json
"""

from __future__ import annotations

import json
import sys
from collections import defaultdict
from datetime import datetime
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

import pandas as pd

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

CACHE_DIR = Path(__file__).resolve().parent.parent / ".cache"
OUTPUT_PATH = Path(__file__).resolve().parent.parent / "tmp" / "signal-timeliness-report.json"
HORIZONS = [5, 10, 20, 50]


# ── 数据加载 ──


def load_bars(symbol: str, interval: str = "1min") -> list[Bar]:
    """从 parquet 缓存加载 Bar 列表。"""
    path = CACHE_DIR / f"{symbol}_{interval}_raw.parquet"
    if not path.exists():
        raise FileNotFoundError(f"缓存不存在: {path}")
    df = pd.read_parquet(path)
    bars: list[Bar] = []
    for ts, row in df.iterrows():
        bars.append(Bar(
            ts=ts.to_pydatetime() if hasattr(ts, "to_pydatetime") else ts,
            open=float(row["open"]),
            high=float(row["high"]),
            low=float(row["low"]),
            close=float(row["close"]),
            volume=float(row["volume"]) if "volume" in row and pd.notna(row.get("volume")) else None,
        ))
    return bars


def discover_symbols(interval: str = "1min") -> list[str]:
    """扫描 .cache 目录，发现可用标的。"""
    suffix = f"_{interval}_raw.parquet"
    return sorted(
        p.name.removesuffix(suffix)
        for p in CACHE_DIR.glob(f"*{suffix}")
    )


# ── 信号收集 ──


def collect_signals(bars: list[Bar], symbol: str) -> list[dict]:
    """运行 RecursiveOrchestrator，收集所有已确认买卖点信号。"""
    orch = RecursiveOrchestrator(stream_id=symbol)
    signals: list[dict] = []
    seen_keys: set[tuple] = set()

    for bar in bars:
        snap = orch.process_bar(bar)
        for bsp in snap.bsp_snapshot.buysellpoints:
            key = (bsp.seg_idx, bsp.kind, bsp.side, bsp.level_id)
            if key in seen_keys:
                continue
            seen_keys.add(key)
            signals.append({
                "symbol": symbol,
                "kind": bsp.kind,
                "side": bsp.side,
                "level_id": bsp.level_id,
                "seg_idx": bsp.seg_idx,
                "bar_idx": bsp.bar_idx,
                "price": bsp.price,
                "confirmed": bsp.confirmed,
            })

    return signals


# ── 前向窗口统计（纯函数，可测试） ──


def forward_window_stats(
    signal: dict,
    bars: list[Bar],
    horizons: list[int] | None = None,
) -> dict[str, Any]:
    """计算信号后 N bar 的前向收益、MAE、MFE。

    Parameters
    ----------
    signal : dict
        信号记录，需含 bar_idx, price, side。
    bars : list[Bar]
        完整 K 线序列。
    horizons : list[int]
        观测窗口列表。

    Returns
    -------
    dict
        fwd_{h}, mae_{h}, mfe_{h} 键值对。
    """
    if horizons is None:
        horizons = HORIZONS

    bi = signal["bar_idx"]
    price = signal["price"]
    is_buy = signal["side"] == "buy"
    n_bars = len(bars)
    stats: dict[str, Any] = {}

    for h in horizons:
        target_idx = bi + h
        if target_idx >= n_bars or price <= 0:
            stats[f"fwd_{h}"] = None
            stats[f"mae_{h}"] = None
            stats[f"mfe_{h}"] = None
            continue

        # 前向收益
        future_close = bars[target_idx].close
        raw_return = (future_close - price) / price
        if not is_buy:
            raw_return = -raw_return
        stats[f"fwd_{h}"] = raw_return

        # MAE / MFE: 扫描 [bi+1, bi+h] 区间
        min_excursion = 0.0
        max_excursion = 0.0
        for j in range(bi + 1, min(bi + h + 1, n_bars)):
            bar_close = bars[j].close
            excursion = (bar_close - price) / price
            if not is_buy:
                excursion = -excursion
            if excursion < min_excursion:
                min_excursion = excursion
            if excursion > max_excursion:
                max_excursion = excursion

        stats[f"mae_{h}"] = min_excursion
        stats[f"mfe_{h}"] = max_excursion

    return stats


# ── 走势完成度分类（纯函数，可测试） ──


def classify_completion(stats: dict[str, Any], horizon: int = 20) -> str:
    """根据 MFE/MAE/fwd 判断走势完成度。

    Returns
    -------
    str
        "full" | "partial" | "failed" | "unknown"
    """
    mfe = stats.get(f"mfe_{horizon}")
    mae = stats.get(f"mae_{horizon}")
    fwd = stats.get(f"fwd_{horizon}")

    if mfe is None or mae is None or fwd is None:
        return "unknown"

    abs_mae = abs(mae)

    # full: MFE 显著且最终保留大部分利润
    if mfe > 0.02 and fwd > mfe * 0.5:
        return "full"

    # partial: MFE 可观但最终回吐大部分
    if mfe > 0.02 and fwd <= mfe * 0.5 and fwd > 0:
        return "partial"

    # failed: 方向错误或几乎无正向偏移
    return "failed"


# ── 按类型聚合（纯函数，可测试） ──


def aggregate_by_kind(records: list[dict]) -> dict[str, Any]:
    """按 kind_side 分组聚合统计。"""
    if not records:
        return {}

    groups: dict[str, list[dict]] = defaultdict(list)
    for rec in records:
        key = f"{rec['kind']}_{rec['side']}"
        groups[key].append(rec)

    result: dict[str, Any] = {}
    for group_key in sorted(groups.keys()):
        items = groups[group_key]
        total = len(items)

        # 完成度分布
        comp_dist: dict[str, int] = defaultdict(int)
        fwd_values: list[float] = []
        for item in items:
            comp = item.get("completion", "unknown")
            comp_dist[comp] += 1
            fwd = item.get("fwd_20")
            if fwd is not None:
                fwd_values.append(fwd)

        mean_fwd = sum(fwd_values) / len(fwd_values) if fwd_values else None

        result[group_key] = {
            "count": total,
            "completion_dist": dict(comp_dist),
            "mean_fwd_20": round(mean_fwd, 6) if mean_fwd is not None else None,
        }

    return result


# ── 主流程 ──


def main() -> None:
    symbols = discover_symbols()
    if not symbols:
        print("未找到 1min 缓存数据。")
        return

    print(f"发现 {len(symbols)} 个标的: {', '.join(symbols)}")

    all_records: list[dict] = []
    per_symbol: dict[str, Any] = {}

    for sym in symbols:
        print(f"\n处理 {sym} ...")
        bars = load_bars(sym)
        signals = collect_signals(bars, sym)
        print(f"  {len(bars)} bars, {len(signals)} 个信号")

        records: list[dict] = []
        for sig in signals:
            stats = forward_window_stats(sig, bars, HORIZONS)
            completion = classify_completion(stats, horizon=20)
            rec = {**sig, **stats, "completion": completion}
            records.append(rec)

        all_records.extend(records)
        per_symbol[sym] = aggregate_by_kind(records)

        for gk, gs in per_symbol[sym].items():
            dist = gs["completion_dist"]
            print(f"  {gk}: count={gs['count']}, "
                  f"full={dist.get('full', 0)}, partial={dist.get('partial', 0)}, "
                  f"failed={dist.get('failed', 0)}")

    global_agg = aggregate_by_kind(all_records)

    report = {
        "generated_at": datetime.now().isoformat(),
        "symbols": symbols,
        "horizons": HORIZONS,
        "total_signals": len(all_records),
        "global_stats": global_agg,
        "per_symbol": per_symbol,
    }

    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT_PATH.write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"\n报告已写入: {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
