"""OQ1 层间时序关系实验 — 层1比价事件 vs 层2个股事件的时序统计。

口径 A 管线（RecursiveOrchestrator 从日线递归）。
数据源：yfinance + FRED 组合，最长最纯净原则。

层1边（比价日线）：
  E/Au = SPY/GLD（SPY 用黄金计价）
  E/R  = SPY/TLT
  Au/R = GLD/TLT

层2（个股/ETF日线）：SPY, GLD, TLT
层2扩展（行业ETF日线）：XLK, XLF, XLE

实验内容：
  1. 在所有层1和层2输入上跑 RecursiveOrchestrator
  2. 记录每个买卖点和走势转折的精确时间戳
  3. 统计层1事件 vs 层2事件的时序先后关系
  4. 用 cross-correlation 和置换检验验证时序关系的统计显著性

认识论等级：L2（真实数据验证）
谱系引用：279号（OQ1 层间关系假说）
"""

from __future__ import annotations

import json
import sys
import time
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "src"))

import numpy as np
import pandas as pd
import yfinance as yf
from scipy import stats as scipy_stats

from newchan.orchestrator.recursive import (
    RecursiveOrchestrator,
    RecursiveOrchestratorSnapshot,
)
from newchan.events import DomainEvent
from newchan.types import Bar


# ═══════════════════════════════════════════════════════════════
# 标的与比价定义
# ═══════════════════════════════════════════════════════════════

L2_TICKERS: dict[str, str] = {
    "SPY": "SPY",
    "GLD": "GLD",
    "TLT": "TLT",
    "XLK": "XLK",
    "XLF": "XLF",
    "XLE": "XLE",
}

# 层1比价边：(stream_id, 分子ticker, 分母ticker)
L1_RATIO_EDGES: list[tuple[str, str, str]] = [
    ("E/Au", "SPY", "GLD"),
    ("E/R", "SPY", "TLT"),
    ("Au/R", "GLD", "TLT"),
]


# ═══════════════════════════════════════════════════════════════
# 数据拉取与转换
# ═══════════════════════════════════════════════════════════════


def fetch_daily(ticker: str, label: str) -> pd.DataFrame | None:
    """拉取日线数据。失败返回 None。"""
    print(f"  [{label}] 拉取 {ticker} ...", end=" ", flush=True)
    try:
        df = yf.download(ticker, period="max", interval="1d", progress=False)
        if isinstance(df.columns, pd.MultiIndex):
            df.columns = df.columns.get_level_values(0)
        if df.empty:
            print("无数据")
            return None
        if df.index.tz is not None:
            df.index = df.index.tz_localize(None)
        print(f"{len(df)} bars ({df.index[0].date()} ~ {df.index[-1].date()})")
        return df
    except Exception as e:
        print(f"失败: {e}")
        return None


def df_to_bars(df: pd.DataFrame) -> list[Bar]:
    """DataFrame -> Bar list。"""
    bars: list[Bar] = []
    for idx, row in df.iterrows():
        ts = idx.to_pydatetime() if hasattr(idx, "to_pydatetime") else idx
        bars.append(Bar(
            ts=ts,
            open=float(row["Open"]),
            high=float(row["High"]),
            low=float(row["Low"]),
            close=float(row["Close"]),
            volume=float(row["Volume"]) if "Volume" in row.index else None,
        ))
    return bars


def make_ratio_bar(bar_a: Bar, bar_b: Bar) -> Bar:
    """构造比价 Bar。与 backtest/orchestrator.py 保持一致。"""
    ratio_open = bar_a.open / bar_b.open
    ratio_high = bar_a.high / bar_b.high
    ratio_low = bar_a.low / bar_b.low
    ratio_close = bar_a.close / bar_b.close
    all_ratios = (ratio_open, ratio_high, ratio_low, ratio_close)
    return Bar(
        ts=bar_a.ts,
        open=ratio_open,
        high=max(all_ratios),
        low=min(all_ratios),
        close=ratio_close,
        volume=None,
    )


# ═══════════════════════════════════════════════════════════════
# 结构事件提取
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class StructureEvent:
    """单个结构事件（可序列化）。"""
    stream_id: str
    bar_idx: int
    timestamp: str   # ISO 格式
    event_type: str  # domain event type from events.py
    detail: dict


# 关注的结构事件类型——代表走势结构变化的关键节点
STRUCTURAL_EVENT_TYPES = frozenset({
    "segment_settle",       # 线段结算（结构确认）
    "zhongshu_candidate",   # 中枢形成
    "zhongshu_settle",      # 中枢闭合
    "move_candidate",       # 走势类型候选
    "move_settle",          # 走势类型结算
    "bsp_confirm",          # 买卖点确认
    "bsp_settle",           # 买卖点结算
})


def run_orchestrator_and_extract(
    stream_id: str,
    bars: list[Bar],
) -> tuple[list[StructureEvent], dict]:
    """对一条流运行 RecursiveOrchestrator 并提取全部结构事件。

    使用 RecursiveOrchestratorSnapshot.all_events 收集所有层级的事件，
    然后过滤出结构性事件。

    Returns (events, stats_dict)
    """
    orch = RecursiveOrchestrator(stream_id=stream_id)
    events: list[StructureEvent] = []
    event_counts: dict[str, int] = defaultdict(int)
    seen_event_ids: set[str] = set()

    for bar in bars:
        snap = orch.process_bar(bar)

        for de in snap.all_events:
            if de.event_type not in STRUCTURAL_EVENT_TYPES:
                continue

            # 使用 event_id 去重（如果有的话），否则用 (event_type, bar_idx, seq)
            eid = de.event_id if de.event_id else f"{de.event_type}_{de.bar_idx}_{de.seq}"
            if eid in seen_event_ids:
                continue
            seen_event_ids.add(eid)

            # 构造 detail dict
            detail: dict = {"seq": de.seq}
            for attr in ("direction", "kind", "side", "level_id",
                         "segment_id", "zhongshu_id", "move_id", "bsp_id",
                         "price", "seg_start", "seg_end",
                         "break_direction", "zd", "zg"):
                if hasattr(de, attr):
                    val = getattr(de, attr)
                    if val is not None:
                        detail[attr] = val

            ts_str = bar.ts.isoformat() if isinstance(bar.ts, datetime) else str(bar.ts)
            events.append(StructureEvent(
                stream_id=stream_id,
                bar_idx=snap.bar_idx,
                timestamp=ts_str,
                event_type=de.event_type,
                detail=detail,
            ))
            event_counts[de.event_type] += 1

    stats = {
        "stream_id": stream_id,
        "total_bars": len(bars),
        "total_events": len(events),
        "event_breakdown": dict(event_counts),
    }
    return events, stats


# ═══════════════════════════════════════════════════════════════
# 时序分析：事件级配对
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class EventPair:
    """一对 L1-L2 事件的时序关系。"""
    l1_stream: str
    l1_event_type: str
    l1_bar_idx: int
    l1_timestamp: str
    l2_stream: str
    l2_event_type: str
    l2_bar_idx: int
    l2_timestamp: str
    lag_bars: int   # l2_bar_idx - l1_bar_idx (正=L1先, 负=L2先)


def find_nearest_pairs(
    l1_events: list[StructureEvent],
    l2_events: list[StructureEvent],
    max_window: int = 60,
) -> list[EventPair]:
    """为每个 L1 事件找最近的 L2 事件（双向，窗口内）。

    对每个 L1 事件，找窗口内最近的 L2 事件（可以在前或在后）。
    返回所有配对（允许正负 lag）。
    """
    if not l1_events or not l2_events:
        return []

    pairs: list[EventPair] = []
    l2_bar_indices = np.array([e.bar_idx for e in l2_events])

    for l1_ev in l1_events:
        diffs = l2_bar_indices - l1_ev.bar_idx
        abs_diffs = np.abs(diffs)
        within_window = abs_diffs <= max_window
        if not within_window.any():
            continue

        # 找窗口内最近的
        masked = np.where(within_window, abs_diffs, max_window + 1)
        best_idx = int(np.argmin(masked))
        l2_ev = l2_events[best_idx]
        lag = l2_ev.bar_idx - l1_ev.bar_idx

        pairs.append(EventPair(
            l1_stream=l1_ev.stream_id,
            l1_event_type=l1_ev.event_type,
            l1_bar_idx=l1_ev.bar_idx,
            l1_timestamp=l1_ev.timestamp,
            l2_stream=l2_ev.stream_id,
            l2_event_type=l2_ev.event_type,
            l2_bar_idx=l2_ev.bar_idx,
            l2_timestamp=l2_ev.timestamp,
            lag_bars=lag,
        ))

    return pairs


# ═══════════════════════════════════════════════════════════════
# 时序分析：事件时间序列 + Cross-Correlation
# ═══════════════════════════════════════════════════════════════


def build_event_time_series(
    events: list[StructureEvent],
    total_bars: int,
) -> np.ndarray:
    """将事件列表转换为 0/1 时间序列（某 bar 是否有事件）。"""
    ts = np.zeros(total_bars, dtype=np.float64)
    for ev in events:
        if 0 <= ev.bar_idx < total_bars:
            ts[ev.bar_idx] = 1.0
    return ts


def cross_correlation(x: np.ndarray, y: np.ndarray, max_lag: int = 60) -> dict:
    """计算两个二进制时间序列的归一化互相关。

    正 lag = x 领先 y（x 的事件先于 y 的事件）。

    Returns dict with lag values, correlations, peak lag, peak correlation.
    """
    n = len(x)
    if n == 0:
        return {"peak_lag": 0, "peak_corr": 0.0, "lags": [], "correlations": []}

    # 去均值
    x_centered = x - x.mean()
    y_centered = y - y.mean()

    # 归一化因子
    norm = np.sqrt(np.sum(x_centered ** 2) * np.sum(y_centered ** 2))
    if norm < 1e-12:
        return {"peak_lag": 0, "peak_corr": 0.0, "lags": [], "correlations": []}

    lags_range = range(-max_lag, max_lag + 1)
    correlations = []
    for lag in lags_range:
        if lag >= 0:
            corr = np.sum(x_centered[:n - lag] * y_centered[lag:]) / norm
        else:
            corr = np.sum(x_centered[-lag:] * y_centered[:n + lag]) / norm
        correlations.append(float(corr))

    lags_list = list(lags_range)
    peak_idx = int(np.argmax(np.abs(correlations)))

    return {
        "peak_lag": lags_list[peak_idx],
        "peak_corr": round(correlations[peak_idx], 6),
        "lags": lags_list,
        "correlations": [round(c, 6) for c in correlations],
    }


def permutation_test_lead_lag(
    lags: list[int],
    n_permutations: int = 10000,
) -> dict:
    """置换检验：检验 lag 的均值是否显著偏离零。

    H0: lag 均值 = 0（无稳定先后关系，随机）
    H1: lag 均值 != 0（存在稳定先后关系）

    Returns dict with observed_mean, p_value, conclusion.
    """
    if len(lags) < 3:
        return {
            "observed_mean": round(np.mean(lags), 4) if lags else 0,
            "p_value": 1.0,
            "n_pairs": len(lags),
            "conclusion": "样本不足（<3对），无法判断",
        }

    arr = np.array(lags, dtype=np.float64)
    observed = float(np.mean(arr))
    abs_observed = abs(observed)

    # 置换：随机翻转符号
    rng = np.random.default_rng(42)
    count_extreme = 0
    for _ in range(n_permutations):
        signs = rng.choice([-1, 1], size=len(arr))
        perm_mean = float(np.mean(arr * signs))
        if abs(perm_mean) >= abs_observed:
            count_extreme += 1

    p_value = (count_extreme + 1) / (n_permutations + 1)

    if p_value < 0.01:
        sig = "高度显著 (p<0.01)"
    elif p_value < 0.05:
        sig = "显著 (p<0.05)"
    elif p_value < 0.10:
        sig = "边缘显著 (p<0.10)"
    else:
        sig = "不显著 (p>=0.10)"

    direction = ""
    if p_value < 0.10:
        if observed > 0:
            direction = "L1 领先 L2（比价事件先于个股事件）"
        else:
            direction = "L2 领先 L1（个股事件先于比价事件）"

    return {
        "observed_mean": round(observed, 4),
        "p_value": round(p_value, 4),
        "n_pairs": len(lags),
        "significance": sig,
        "direction": direction,
        "n_permutations": n_permutations,
    }


def compute_lead_lag_stats(lags: list[int]) -> dict:
    """计算 lag 分布的描述统计。"""
    if not lags:
        return {"count": 0}
    arr = np.array(lags)
    return {
        "count": len(arr),
        "mean": round(float(np.mean(arr)), 2),
        "median": round(float(np.median(arr)), 2),
        "std": round(float(np.std(arr)), 2),
        "min": int(np.min(arr)),
        "max": int(np.max(arr)),
        "p25": round(float(np.percentile(arr, 25)), 2),
        "p75": round(float(np.percentile(arr, 75)), 2),
        "l1_leads_count": int(np.sum(arr > 0)),
        "l2_leads_count": int(np.sum(arr < 0)),
        "simultaneous_count": int(np.sum(arr == 0)),
        "l1_leads_pct": round(float(np.sum(arr > 0)) / len(arr) * 100, 1),
        "l2_leads_pct": round(float(np.sum(arr < 0)) / len(arr) * 100, 1),
        "simultaneous_pct": round(float(np.sum(arr == 0)) / len(arr) * 100, 1),
    }


# ═══════════════════════════════════════════════════════════════
# 主流程
# ═══════════════════════════════════════════════════════════════


def main() -> None:
    print("=" * 70)
    print("OQ1 层间时序关系实验")
    print("口径 A (RecursiveOrchestrator 从日线递归)")
    print("数据源: yfinance, 最长最纯净原则")
    print("认识论等级: L2（真实数据验证）")
    print("=" * 70)

    # ── 1. 数据拉取 ──
    print("\n[1/6] 拉取数据")
    raw_data: dict[str, pd.DataFrame] = {}
    for label, ticker in L2_TICKERS.items():
        df = fetch_daily(ticker, label)
        if df is not None:
            raw_data[label] = df

    required = ["SPY", "GLD", "TLT"]
    for r in required:
        if r not in raw_data:
            print(f"错误: 缺少必需数据 {r}")
            return

    # ── 2. 对齐到公共交易日 ──
    print("\n[2/6] 数据对齐")

    # 核心三标的取交集
    common_dates = raw_data["SPY"].index
    for t in ["GLD", "TLT"]:
        common_dates = common_dates.intersection(raw_data[t].index)
    common_dates = common_dates.sort_values()
    print(f"  核心公共交易日: {len(common_dates)} "
          f"({common_dates[0].date()} ~ {common_dates[-1].date()})")

    # 对齐所有层2标的
    aligned_l2: dict[str, pd.DataFrame] = {}
    for name in L2_TICKERS:
        if name in raw_data:
            overlap = common_dates.intersection(raw_data[name].index).sort_values()
            aligned_l2[name] = raw_data[name].loc[overlap]
            print(f"  {name}: {len(aligned_l2[name])} bars")

    # 构造 Bar 列表
    l2_bars: dict[str, list[Bar]] = {}
    for name, df in aligned_l2.items():
        l2_bars[name] = df_to_bars(df)

    # ── 3. 构造层1比价 Bar ──
    print("\n[3/6] 构造层1比价序列")
    l1_bars: dict[str, list[Bar]] = {}
    for stream_id, num_ticker, den_ticker in L1_RATIO_EDGES:
        if num_ticker not in l2_bars or den_ticker not in l2_bars:
            print(f"  跳过 {stream_id}: 缺少 {num_ticker} 或 {den_ticker}")
            continue
        num_b = l2_bars[num_ticker]
        den_b = l2_bars[den_ticker]
        min_len = min(len(num_b), len(den_b))
        ratio_bars = [make_ratio_bar(num_b[i], den_b[i]) for i in range(min_len)]
        l1_bars[stream_id] = ratio_bars
        print(f"  {stream_id}: {len(ratio_bars)} bars")

    total_bars = min(
        min(len(b) for b in l1_bars.values()),
        min(len(b) for b in l2_bars.values()),
    )
    print(f"  公共 bar 数量: {total_bars}")

    # ── 4. 运行 D 算子 + 提取结构事件 ──
    print("\n[4/6] 运行 RecursiveOrchestrator + 提取结构事件")

    all_l1_events: dict[str, list[StructureEvent]] = {}
    all_l2_events: dict[str, list[StructureEvent]] = {}
    all_stats: dict[str, dict] = {}
    t0 = time.time()

    print("  --- 层1 (比价序列) ---")
    for stream_id, bars in l1_bars.items():
        print(f"  {stream_id} ({len(bars)} bars) ...", end=" ", flush=True)
        t_start = time.time()
        events, stats = run_orchestrator_and_extract(stream_id, bars)
        elapsed = time.time() - t_start
        all_l1_events[stream_id] = events
        all_stats[f"L1_{stream_id}"] = stats
        print(f"{elapsed:.1f}s | {len(events)} events "
              f"({stats['event_breakdown']})")

    print("  --- 层2 (个股/ETF) ---")
    for name, bars in l2_bars.items():
        print(f"  {name} ({len(bars)} bars) ...", end=" ", flush=True)
        t_start = time.time()
        events, stats = run_orchestrator_and_extract(name, bars)
        elapsed = time.time() - t_start
        all_l2_events[name] = events
        all_stats[f"L2_{name}"] = stats
        print(f"{elapsed:.1f}s | {len(events)} events "
              f"({stats['event_breakdown']})")

    total_elapsed = time.time() - t0
    print(f"\n  全部流完成: {total_elapsed:.1f}s")

    # ── 5. 时序分析 ──
    print("\n[5/6] 时序分析")

    # 5a. 事件配对 + lead-lag 统计
    print("  --- 事件配对分析 ---")
    pairwise_results: dict[str, dict] = {}
    all_lags: list[int] = []

    for l1_name, l1_events in all_l1_events.items():
        for l2_name, l2_events in all_l2_events.items():
            pair_key = f"{l1_name} vs {l2_name}"
            pairs = find_nearest_pairs(l1_events, l2_events, max_window=60)
            lags = [p.lag_bars for p in pairs]
            all_lags.extend(lags)

            lag_stats = compute_lead_lag_stats(lags)
            perm_test = permutation_test_lead_lag(lags)

            pairwise_results[pair_key] = {
                "l1_events_count": len(l1_events),
                "l2_events_count": len(l2_events),
                "pairs_count": len(pairs),
                "lag_stats": lag_stats,
                "permutation_test": perm_test,
            }

            if pairs:
                print(f"  {pair_key}: {len(pairs)} pairs | "
                      f"mean_lag={lag_stats['mean']} | "
                      f"L1先={lag_stats['l1_leads_pct']}% | "
                      f"p={perm_test['p_value']} ({perm_test['significance']})")
            else:
                print(f"  {pair_key}: 0 pairs")

    # 5b. 总体 lead-lag
    overall_lag_stats = compute_lead_lag_stats(all_lags)
    overall_perm_test = permutation_test_lead_lag(all_lags)
    print(f"\n  总体: {len(all_lags)} pairs | "
          f"mean_lag={overall_lag_stats.get('mean', 'N/A')} | "
          f"p={overall_perm_test['p_value']} ({overall_perm_test['significance']})")

    # 5c. Cross-correlation 分析
    print("\n  --- Cross-Correlation 分析 ---")
    xcorr_results: dict[str, dict] = {}
    for l1_name, l1_events in all_l1_events.items():
        l1_ts = build_event_time_series(l1_events, total_bars)
        for l2_name, l2_events in all_l2_events.items():
            l2_ts = build_event_time_series(l2_events, total_bars)
            pair_key = f"{l1_name} vs {l2_name}"
            xcorr = cross_correlation(l1_ts, l2_ts, max_lag=60)
            # 只保存 peak 信息，不保存完整 correlation 数组（太大）
            xcorr_results[pair_key] = {
                "peak_lag": xcorr["peak_lag"],
                "peak_corr": xcorr["peak_corr"],
            }
            print(f"  {pair_key}: peak_lag={xcorr['peak_lag']}, "
                  f"peak_corr={xcorr['peak_corr']}")

    # 5d. 按事件类型分组分析
    print("\n  --- 按事件类型分组 ---")
    by_event_type: dict[str, list[int]] = defaultdict(list)
    for l1_name, l1_events in all_l1_events.items():
        for l2_name, l2_events in all_l2_events.items():
            pairs = find_nearest_pairs(l1_events, l2_events, max_window=60)
            for p in pairs:
                key = f"{p.l1_event_type} -> {p.l2_event_type}"
                by_event_type[key].append(p.lag_bars)

    event_type_analysis: dict[str, dict] = {}
    for key, lags in sorted(by_event_type.items()):
        lag_stats = compute_lead_lag_stats(lags)
        perm = permutation_test_lead_lag(lags)
        event_type_analysis[key] = {
            "lag_stats": lag_stats,
            "permutation_test": perm,
        }
        if lag_stats["count"] >= 3:
            print(f"  {key}: n={lag_stats['count']} | "
                  f"mean={lag_stats['mean']} | p={perm['p_value']}")

    # ── 6. 保存结果 ──
    print("\n[6/6] 保存结果")

    # 6a. JSON 结果
    results = {
        "metadata": {
            "experiment": "OQ1 层间时序关系",
            "pipeline": "口径 A (RecursiveOrchestrator)",
            "epistemological_level": "L2（真实数据验证）",
            "genealogy_ref": "279号",
            "generated_at": datetime.now().isoformat(),
            "total_bars": total_bars,
            "common_dates_start": str(common_dates[0].date()),
            "common_dates_end": str(common_dates[-1].date()),
        },
        "stream_stats": all_stats,
        "pairwise_analysis": pairwise_results,
        "overall": {
            "lag_stats": overall_lag_stats,
            "permutation_test": overall_perm_test,
        },
        "cross_correlation": xcorr_results,
        "by_event_type": event_type_analysis,
        "event_timeline": {},
    }

    # 事件时间线（每个流的前 200 个事件作为样本）
    for name, events in {**all_l1_events, **all_l2_events}.items():
        results["event_timeline"][name] = [
            {
                "stream_id": ev.stream_id,
                "bar_idx": ev.bar_idx,
                "timestamp": ev.timestamp,
                "event_type": ev.event_type,
                "detail": ev.detail,
            }
            for ev in events[:200]
        ]

    results_path = ROOT / "tmp" / "oq1-interlayer-timing-results.json"
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False)
    print(f"  结果 -> {results_path}")

    # 6b. 报告
    report_lines = _build_report(
        common_dates, all_stats, pairwise_results,
        overall_lag_stats, overall_perm_test,
        xcorr_results, event_type_analysis, total_bars,
    )
    report_path = ROOT / "tmp" / "oq1-interlayer-timing-report.md"
    with open(report_path, "w", encoding="utf-8") as f:
        f.write("\n".join(report_lines))
    print(f"  报告 -> {report_path}")

    print("\n" + "=" * 70)
    print("OQ1 层间时序关系实验完成")
    print("=" * 70)


def _build_report(
    common_dates: pd.DatetimeIndex,
    all_stats: dict,
    pairwise_results: dict,
    overall_lag_stats: dict,
    overall_perm_test: dict,
    xcorr_results: dict,
    event_type_analysis: dict,
    total_bars: int,
) -> list[str]:
    """构建人类可读报告。"""
    lines = [
        "# OQ1 层间时序关系实验报告",
        "",
        f"生成时间: {datetime.now().isoformat()}",
        f"认识论等级: L2（真实数据验证）",
        f"谱系引用: 279号（OQ1 层间关系假说）",
        "",
        "## 1. 实验设计",
        "",
        "- **管线**: 口径 A（RecursiveOrchestrator 从日线递归）",
        "- **层1比价边**: E/Au (SPY/GLD), E/R (SPY/TLT), Au/R (GLD/TLT)",
        "- **层2个股/ETF**: SPY, GLD, TLT, XLK, XLF, XLE",
        f"- **公共交易日**: {len(common_dates)} "
        f"({common_dates[0].date()} ~ {common_dates[-1].date()})",
        f"- **总 bar**: {total_bars}",
        "",
        "## 2. 结构事件统计",
        "",
        "| 流 | 层 | 总 bar | 总事件 | 事件明细 |",
        "|---|---|---|---|---|",
    ]

    for name, stats in sorted(all_stats.items()):
        layer = "L1" if name.startswith("L1_") else "L2"
        short_name = name[3:]
        breakdown = stats.get("event_breakdown", {})
        detail_str = ", ".join(f"{k}={v}" for k, v in sorted(breakdown.items()))
        lines.append(
            f"| {short_name} | {layer} | {stats['total_bars']} | "
            f"{stats['total_events']} | {detail_str} |"
        )

    lines.extend([
        "",
        "## 3. 层间时序配对分析",
        "",
        "正 mean_lag = L1 领先 L2（比价事件先于个股事件）",
        "负 mean_lag = L2 领先 L1（个股事件先于比价事件）",
        "",
        "| L1 vs L2 | 配对数 | mean lag | median lag | L1先% | L2先% | 同时% | p-value | 显著性 |",
        "|---|---|---|---|---|---|---|---|---|",
    ])

    for pair_key, r in sorted(pairwise_results.items()):
        ls = r["lag_stats"]
        pt = r["permutation_test"]
        if ls.get("count", 0) > 0:
            lines.append(
                f"| {pair_key} | {ls['count']} | {ls['mean']} | {ls['median']} | "
                f"{ls['l1_leads_pct']} | {ls['l2_leads_pct']} | "
                f"{ls['simultaneous_pct']} | {pt['p_value']} | {pt['significance']} |"
            )
        else:
            lines.append(f"| {pair_key} | 0 | - | - | - | - | - | - | - |")

    lines.extend([
        "",
        "### 总体统计",
        "",
    ])
    if overall_lag_stats.get("count", 0) > 0:
        lines.extend([
            f"- 总配对数: {overall_lag_stats['count']}",
            f"- mean lag: {overall_lag_stats['mean']} bars",
            f"- median lag: {overall_lag_stats['median']} bars",
            f"- std: {overall_lag_stats['std']} bars",
            f"- L1 领先比例: {overall_lag_stats['l1_leads_pct']}%",
            f"- L2 领先比例: {overall_lag_stats['l2_leads_pct']}%",
            f"- 同时比例: {overall_lag_stats['simultaneous_pct']}%",
            f"- 置换检验 p-value: {overall_perm_test['p_value']}",
            f"- 显著性: {overall_perm_test['significance']}",
        ])
        if overall_perm_test.get("direction"):
            lines.append(f"- 方向: {overall_perm_test['direction']}")
    else:
        lines.append("- 无足够数据")

    lines.extend([
        "",
        "## 4. Cross-Correlation 峰值",
        "",
        "| L1 vs L2 | peak lag (bars) | peak correlation |",
        "|---|---|---|",
    ])
    for pair_key, xc in sorted(xcorr_results.items()):
        lines.append(f"| {pair_key} | {xc['peak_lag']} | {xc['peak_corr']} |")

    lines.extend([
        "",
        "## 5. 按事件类型分组",
        "",
        "| 事件类型对 | 配对数 | mean lag | p-value | 显著性 |",
        "|---|---|---|---|---|",
    ])
    for key, r in sorted(event_type_analysis.items()):
        ls = r["lag_stats"]
        pt = r["permutation_test"]
        if ls.get("count", 0) >= 3:
            lines.append(
                f"| {key} | {ls['count']} | {ls['mean']} | "
                f"{pt['p_value']} | {pt['significance']} |"
            )

    lines.extend([
        "",
        "## 6. 结论",
        "",
        "### 结果包",
        "",
        "1. **结论**: 见上表——层1比价事件与层2个股事件之间是否存在稳定的时序先后关系",
        "2. **定义依据**: OQ1 层间关系假说（279号谱系）— 资本流动方向态在比价层先于个股层",
        "3. **边界条件**:",
        "   - ETF 上市日期不同导致公共数据长度受限",
        "   - 结构事件频率受管线参数影响（笔模式、递归深度）",
        "   - 60 bar 窗口是匹配阈值（可调整）",
        "4. **下游推论**: 如果 L1 显著领先 L2 且 p<0.05，K4 配置可作为层2操作的先行指标",
        "5. **谱系引用**: 279号（OQ1 层间关系假说）",
        "6. **影响声明**: 实验结果，不修改现有代码或定义",
    ])

    return lines


if __name__ == "__main__":
    main()
