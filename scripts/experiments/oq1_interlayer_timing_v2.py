"""OQ1 v2 层间结构事件的时序关系实验。

284号谱系（OQ v2）：D 算子 + 背驰判断的联合阅读。

层 1（K4 六条比价边的走势结构和背驰判断）和层 2（个股走势结构和背驰判断）
之间，是否存在稳定的先后关系？

两层各自跑 RecursiveOrchestrator + 背驰判断（管线内部已整合）。
层 1 的结构事件（中枢破坏、走势完成、背驰/买卖点）和层 2 的结构事件
在时间上的关系是什么？有没有稳定的领先/滞后模式？

数据源：yfinance，最长最纯净原则。
认识论等级：L2（真实数据验证）
谱系引用：284号（OQ v2）、283号、279号
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

RESULTS_DIR = ROOT / "tmp" / "oq1-v2-results"

# ═══════════════════════════════════════════════════════════════
# K4 六条比价边 + 层2标的
# ═══════════════════════════════════════════════════════════════

# 层1：K4 完全图六条比价边
# 顶点：E(权益)=SPY, Au(黄金)=GLD, R(债券)=TLT, $(美元隐含基准)
# 顶点边（原始品种,相对$）
L1_VERTEX_EDGES: list[tuple[str, str | None, str | None]] = [
    ("E/$", "SPY", None),
    ("Au/$", "GLD", None),
    ("R/$", "TLT", None),
]
# 比价边
L1_RATIO_EDGES: list[tuple[str, str, str]] = [
    ("E/Au", "SPY", "GLD"),
    ("Au/R", "GLD", "TLT"),
    ("E/R", "SPY", "TLT"),
]

# 层2 个股/ETF
L2_TICKERS: dict[str, str] = {
    "SPY": "SPY",
    "GLD": "GLD",
    "TLT": "TLT",
    "XLK": "XLK",
    "XLF": "XLF",
    "XLE": "XLE",
}


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
    """构造比价 Bar。high/low 取四种极端比值的 max/min。"""
    ratio_open = bar_a.open / bar_b.open
    ratio_close = bar_a.close / bar_b.close
    all_ratios = [
        ratio_open,
        ratio_close,
        bar_a.high / bar_b.low,
        bar_a.low / bar_b.high,
    ]
    return Bar(
        ts=bar_a.ts,
        open=ratio_open,
        high=max(all_ratios),
        low=min(all_ratios),
        close=ratio_close,
        volume=None,
    )


# ═══════════════════════════════════════════════════════════════
# 结构事件提取（含背驰/买卖点）
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class StructureEvent:
    """单个结构事件。"""
    stream_id: str
    bar_idx: int
    timestamp: str
    event_type: str
    detail: dict


# 关注的结构事件类型——走势结构 + 背驰/买卖点
STRUCTURAL_EVENT_TYPES = frozenset({
    "segment_settle",
    "zhongshu_candidate",
    "zhongshu_settle",
    "move_candidate",
    "move_settle",
    "bsp_candidate",
    "bsp_confirm",
    "bsp_settle",
})


def run_orchestrator_and_extract(
    stream_id: str,
    bars: list[Bar],
) -> tuple[list[StructureEvent], dict]:
    """对一条流运行 RecursiveOrchestrator 并提取全部结构事件。

    RecursiveOrchestrator 内部已整合 D 算子 + 背驰判断 + 买卖点，
    不存在"形态学层"和"动力学层"的分离。

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

            eid = de.event_id if de.event_id else f"{de.event_type}_{de.bar_idx}_{de.seq}"
            if eid in seen_event_ids:
                continue
            seen_event_ids.add(eid)

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
# 时序分析
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
    lag_bars: int


def find_nearest_pairs(
    l1_events: list[StructureEvent],
    l2_events: list[StructureEvent],
    max_window: int = 60,
) -> list[EventPair]:
    """为每个 L1 事件找窗口内最近的 L2 事件。"""
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


def build_event_time_series(
    events: list[StructureEvent],
    total_bars: int,
) -> np.ndarray:
    """将事件列表转换为 0/1 时间序列。"""
    ts = np.zeros(total_bars, dtype=np.float64)
    for ev in events:
        if 0 <= ev.bar_idx < total_bars:
            ts[ev.bar_idx] = 1.0
    return ts


def cross_correlation(x: np.ndarray, y: np.ndarray, max_lag: int = 60) -> dict:
    """归一化互相关。正 lag = x 领先 y。"""
    n = len(x)
    if n == 0:
        return {"peak_lag": 0, "peak_corr": 0.0, "lags": [], "correlations": []}

    x_centered = x - x.mean()
    y_centered = y - y.mean()

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
    """置换检验：检验 lag 均值是否显著偏离零。"""
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
    """lag 分布描述统计。"""
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
    print("OQ1 v2 层间结构事件时序关系实验")
    print("284号谱系：D 算子 + 背驰判断的联合阅读")
    print("管线：RecursiveOrchestrator（形态+背驰+买卖点一体）")
    print("数据源: yfinance, 最长最纯净原则")
    print("认识论等级: L2（真实数据验证）")
    print("=" * 70)

    # ── 1. 数据拉取 ──
    print("\n[1/6] 拉取数据")
    raw_data: dict[str, pd.DataFrame] = {}
    all_tickers = {**L2_TICKERS}
    for label, ticker in all_tickers.items():
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
    common_dates = raw_data["SPY"].index
    for t in ["GLD", "TLT"]:
        common_dates = common_dates.intersection(raw_data[t].index)
    common_dates = common_dates.sort_values()
    print(f"  核心公共交易日: {len(common_dates)} "
          f"({common_dates[0].date()} ~ {common_dates[-1].date()})")

    aligned_l2: dict[str, pd.DataFrame] = {}
    for name in L2_TICKERS:
        if name in raw_data:
            overlap = common_dates.intersection(raw_data[name].index).sort_values()
            aligned_l2[name] = raw_data[name].loc[overlap]
            print(f"  {name}: {len(aligned_l2[name])} bars")

    l2_bars: dict[str, list[Bar]] = {}
    for name, df in aligned_l2.items():
        l2_bars[name] = df_to_bars(df)

    # ── 3. 构造层1（K4 六条边） ──
    print("\n[3/6] 构造层1 K4 六条边")

    l1_bars: dict[str, list[Bar]] = {}

    # 顶点边（原始品种 vs $）
    for edge_label, ticker, _ in L1_VERTEX_EDGES:
        if ticker in l2_bars:
            l1_bars[edge_label] = l2_bars[ticker]
            print(f"  {edge_label} = {ticker}: {len(l1_bars[edge_label])} bars")

    # 比价边
    for edge_label, num_ticker, den_ticker in L1_RATIO_EDGES:
        if num_ticker not in l2_bars or den_ticker not in l2_bars:
            print(f"  跳过 {edge_label}: 缺少 {num_ticker} 或 {den_ticker}")
            continue
        num_b = l2_bars[num_ticker]
        den_b = l2_bars[den_ticker]
        min_len = min(len(num_b), len(den_b))
        ratio_bars = [make_ratio_bar(num_b[i], den_b[i]) for i in range(min_len)]
        l1_bars[edge_label] = ratio_bars
        print(f"  {edge_label}: {len(ratio_bars)} bars")

    total_bars = min(
        min(len(b) for b in l1_bars.values()),
        min(len(b) for b in l2_bars.values()),
    )
    print(f"  公共 bar 数量: {total_bars}")

    # ── 4. 运行 RecursiveOrchestrator + 提取结构事件（含背驰/BSP）──
    print("\n[4/6] 运行 RecursiveOrchestrator + 提取结构事件（含背驰/买卖点）")

    all_l1_events: dict[str, list[StructureEvent]] = {}
    all_l2_events: dict[str, list[StructureEvent]] = {}
    all_stats: dict[str, dict] = {}
    t0 = time.time()

    print("  --- 层1 (K4 六条边) ---")
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
            xcorr_results[pair_key] = {
                "peak_lag": xcorr["peak_lag"],
                "peak_corr": xcorr["peak_corr"],
            }
            print(f"  {pair_key}: peak_lag={xcorr['peak_lag']}, "
                  f"peak_corr={xcorr['peak_corr']}")

    # 5d. 按事件类型分组分析——区分结构事件和背驰/BSP事件
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
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    results = {
        "metadata": {
            "experiment": "OQ1 v2 层间结构事件时序关系",
            "pipeline": "RecursiveOrchestrator（D算子+背驰+买卖点一体）",
            "epistemological_level": "L2（真实数据验证）",
            "genealogy_ref": ["284号（OQ v2）", "283号", "279号"],
            "generated_at": datetime.now().isoformat(),
            "total_bars": total_bars,
            "common_dates_start": str(common_dates[0].date()),
            "common_dates_end": str(common_dates[-1].date()),
            "l1_edges": list(l1_bars.keys()),
            "l2_tickers": list(l2_bars.keys()),
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

    # 事件时间线样本
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

    results_path = RESULTS_DIR / "oq1-v2-results.json"
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False)
    print(f"  结果 -> {results_path}")

    # 报告
    report_lines = _build_report(
        common_dates, all_stats, pairwise_results,
        overall_lag_stats, overall_perm_test,
        xcorr_results, event_type_analysis, total_bars,
    )
    report_path = RESULTS_DIR / "oq1-v2-report.md"
    with open(report_path, "w", encoding="utf-8") as f:
        f.write("\n".join(report_lines))
    print(f"  报告 -> {report_path}")

    print("\n" + "=" * 70)
    print("OQ1 v2 层间结构事件时序关系实验完成")
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
        "# OQ1 v2 层间结构事件时序关系实验报告",
        "",
        f"生成时间: {datetime.now().isoformat()}",
        "认识论等级: L2（真实数据验证）",
        "谱系引用: 284号（OQ v2）、283号、279号",
        "",
        "## 1. 实验设计",
        "",
        "### 284号变更",
        "",
        "- 背驰判断（走势延续 + MACD 面积缩小）是缠论内部的统一判断",
        "- 形态学和动力学不是两层独立的东西",
        "- D 算子 + 背驰判断一起跑在每条边上",
        "- RecursiveOrchestrator 内部已整合形态+背驰+买卖点",
        "",
        "### 管线",
        "",
        "- RecursiveOrchestrator 从日线递归",
        "- 层1：K4 完全图六条边（顶点边 E/$, Au/$, R/$ + 比价边 E/Au, Au/R, E/R）",
        "- 层2：SPY, GLD, TLT, XLK, XLF, XLE",
        f"- 公共交易日: {len(common_dates)} "
        f"({common_dates[0].date()} ~ {common_dates[-1].date()})",
        f"- 总 bar: {total_bars}",
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
        "正 mean_lag = L1 领先 L2（比价/顶点边事件先于个股事件）",
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
        "## 6. 结果包",
        "",
        "1. **结论**: 见上表——层1 K4六条边和层2个股/ETF之间的时序先后关系",
        "2. **定义依据**: 284号 OQ v2——背驰是形态和力度的交叉判断，不是独立层",
        "3. **边界条件**:",
        "   - ETF 上市日期限制公共窗口长度",
        "   - RecursiveOrchestrator 管线参数（笔模式、递归深度）影响事件频率",
        "   - 60 bar 匹配窗口是可调参数",
        "4. **下游推论**: 如果 L1 显著领先 L2 且 p<0.05，K4 配置变化可作为层2操作的先行指标",
        "5. **谱系引用**: 284号、283号、279号",
        "6. **影响声明**: 实验结果，不修改现有代码或定义",
    ])

    return lines


if __name__ == "__main__":
    main()
