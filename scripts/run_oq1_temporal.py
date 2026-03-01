"""OQ1 层间时序关系 -- 层1比价事件 vs 层2个股事件的时序统计。

279号谱系延续：多层同步跑 D 算子，统计层1结构事件和层2结构事件之间的时序关系。

层1边（比价序列）：SPY/GLD, SPY/TLT, GLD/TLT
层2（个股价格序列）：SPY, GLD, TLT
层2扩展：XLK, XLF, XLE

结构事件定义：
  - 走势转折 = settled move 的方向与前一个 settled move 不同
  - 买卖点 = bsp_snapshot.buysellpoints 中 confirmed=True 的项
  - 方向态变化 = walk_direction_from_snapshot 的值变化

认识论等级：L2（真实数据验证）
谱系引用：279号（OQ1 层间关系假说）
"""

from __future__ import annotations

import json
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

import pandas as pd
import yfinance as yf

from newchan.orchestrator.recursive import RecursiveOrchestrator, RecursiveOrchestratorSnapshot
from newchan.topology.k4_scanner import walk_direction_from_snapshot
from newchan.topology.config_space import WalkDirection
from newchan.types import Bar


# ===================================================================
# 标的与比价定义
# ===================================================================

L2_TICKERS: dict[str, str] = {
    "SPY": "SPY",
    "GLD": "GLD",
    "TLT": "TLT",
    "XLK": "XLK",
    "XLF": "XLF",
    "XLE": "XLE",
}

# 层1比价边：(名称, 分子ticker, 分母ticker)
L1_RATIO_EDGES: list[tuple[str, str, str]] = [
    ("SPY_GLD", "SPY", "GLD"),
    ("SPY_TLT", "SPY", "TLT"),
    ("GLD_TLT", "GLD", "TLT"),
]

RESULTS_DIR = ROOT / "tmp" / "oq1-temporal-results"


# ===================================================================
# 数据拉取
# ===================================================================


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


def make_ratio_bar(bar_a: Bar, bar_b: Bar) -> Bar:
    """从两个标的的同日 Bar 构造比价 Bar。"""
    ratio_open = bar_a.open / bar_b.open
    ratio_close = bar_a.close / bar_b.close
    all_ratios = [ratio_open, ratio_close,
                  bar_a.high / bar_b.low, bar_a.low / bar_b.high]
    return Bar(
        ts=bar_a.ts,
        open=ratio_open,
        high=max(all_ratios),
        low=min(all_ratios),
        close=ratio_close,
        volume=None,
    )


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


# ===================================================================
# 结构事件提取
# ===================================================================


@dataclass(frozen=True, slots=True)
class StructureEvent:
    """单个结构事件。"""
    stream_id: str
    bar_idx: int
    timestamp: datetime
    event_type: str   # "move_reversal" | "bsp_confirmed" | "direction_change"
    detail: dict


def extract_events_from_stream(
    stream_id: str,
    bars: list[Bar],
) -> tuple[list[StructureEvent], dict]:
    """对一条流运行 RecursiveOrchestrator 并提取所有结构事件。

    Returns (events, stats_dict)
    """
    orch = RecursiveOrchestrator(stream_id=stream_id)
    events: list[StructureEvent] = []

    prev_settled_moves: list[tuple[str, str]] = []  # [(kind, direction), ...]
    prev_walk_dir: WalkDirection = WalkDirection.FLAT
    seen_bsp_keys: set[tuple[int, str, str, int]] = set()

    move_reversal_count = 0
    bsp_confirmed_count = 0
    direction_change_count = 0

    for bar in bars:
        snap = orch.process_bar(bar)

        # --- 走势转折检测 ---
        settled_moves = [
            (m.kind, m.direction)
            for m in snap.move_snapshot.moves if m.settled
        ]
        if (settled_moves and prev_settled_moves
                and settled_moves[-1] != prev_settled_moves[-1]):
            last_prev = prev_settled_moves[-1]
            last_curr = settled_moves[-1]
            events.append(StructureEvent(
                stream_id=stream_id,
                bar_idx=snap.bar_idx,
                timestamp=bar.ts,
                event_type="move_reversal",
                detail={
                    "prev_kind": last_prev[0],
                    "prev_direction": last_prev[1],
                    "curr_kind": last_curr[0],
                    "curr_direction": last_curr[1],
                },
            ))
            move_reversal_count += 1
        if settled_moves:
            prev_settled_moves = settled_moves

        # --- 买卖点确认检测 ---
        for bp in snap.bsp_snapshot.buysellpoints:
            if bp.confirmed:
                bsp_key = (bp.seg_idx, bp.kind, bp.side, bp.level_id)
                if bsp_key not in seen_bsp_keys:
                    seen_bsp_keys.add(bsp_key)
                    events.append(StructureEvent(
                        stream_id=stream_id,
                        bar_idx=snap.bar_idx,
                        timestamp=bar.ts,
                        event_type="bsp_confirmed",
                        detail={
                            "kind": bp.kind,
                            "side": bp.side,
                            "level_id": bp.level_id,
                            "price": bp.price,
                        },
                    ))
                    bsp_confirmed_count += 1

        # --- 方向态变化检测 ---
        curr_walk_dir = walk_direction_from_snapshot(snap, level=0)
        if curr_walk_dir != prev_walk_dir:
            events.append(StructureEvent(
                stream_id=stream_id,
                bar_idx=snap.bar_idx,
                timestamp=bar.ts,
                event_type="direction_change",
                detail={
                    "prev_direction": prev_walk_dir.name,
                    "curr_direction": curr_walk_dir.name,
                },
            ))
            direction_change_count += 1
        prev_walk_dir = curr_walk_dir

    stats = {
        "stream_id": stream_id,
        "total_bars": len(bars),
        "move_reversals": move_reversal_count,
        "bsp_confirmed": bsp_confirmed_count,
        "direction_changes": direction_change_count,
        "total_events": len(events),
    }
    return events, stats


# ===================================================================
# 时序分析
# ===================================================================


@dataclass
class TemporalPair:
    """一对层1-层2事件的时序关系。"""
    l1_stream: str
    l1_event_type: str
    l1_bar_idx: int
    l1_timestamp: datetime
    l2_stream: str
    l2_event_type: str
    l2_bar_idx: int
    l2_timestamp: datetime
    lag_bars: int       # l2_bar_idx - l1_bar_idx (正=层1领先, 负=层2领先)
    lag_days: int       # 日历天差


def compute_temporal_pairs(
    l1_events: list[StructureEvent],
    l2_events: list[StructureEvent],
    max_window_bars: int = 60,
) -> list[TemporalPair]:
    """为每个层1事件，找最近的层2事件（窗口内），反之亦然。

    双向匹配：每个 L1 事件找最近的后续 L2 事件，每个 L2 事件找最近的前序 L1 事件。
    """
    pairs: list[TemporalPair] = []
    seen_pair_keys: set[tuple[str, int, str, int]] = set()

    # L1 → 最近后续 L2
    for l1_ev in l1_events:
        best_l2: StructureEvent | None = None
        best_lag: int = max_window_bars + 1
        for l2_ev in l2_events:
            lag = l2_ev.bar_idx - l1_ev.bar_idx
            if 0 <= lag <= max_window_bars and lag < best_lag:
                best_lag = lag
                best_l2 = l2_ev
        if best_l2 is not None:
            pair_key = (l1_ev.stream_id, l1_ev.bar_idx,
                        best_l2.stream_id, best_l2.bar_idx)
            if pair_key not in seen_pair_keys:
                seen_pair_keys.add(pair_key)
                lag_days = (best_l2.timestamp - l1_ev.timestamp).days
                pairs.append(TemporalPair(
                    l1_stream=l1_ev.stream_id,
                    l1_event_type=l1_ev.event_type,
                    l1_bar_idx=l1_ev.bar_idx,
                    l1_timestamp=l1_ev.timestamp,
                    l2_stream=best_l2.stream_id,
                    l2_event_type=best_l2.event_type,
                    l2_bar_idx=best_l2.bar_idx,
                    l2_timestamp=best_l2.timestamp,
                    lag_bars=best_lag,
                    lag_days=lag_days,
                ))

    # L2 → 最近前序 L1
    for l2_ev in l2_events:
        best_l1: StructureEvent | None = None
        best_lag: int = max_window_bars + 1
        for l1_ev in l1_events:
            lag = l2_ev.bar_idx - l1_ev.bar_idx
            if 0 <= lag <= max_window_bars and lag < best_lag:
                best_lag = lag
                best_l1 = l1_ev
        if best_l1 is not None:
            pair_key = (best_l1.stream_id, best_l1.bar_idx,
                        l2_ev.stream_id, l2_ev.bar_idx)
            if pair_key not in seen_pair_keys:
                seen_pair_keys.add(pair_key)
                lag_days = (l2_ev.timestamp - best_l1.timestamp).days
                pairs.append(TemporalPair(
                    l1_stream=best_l1.stream_id,
                    l1_event_type=best_l1.event_type,
                    l1_bar_idx=best_l1.bar_idx,
                    l1_timestamp=best_l1.timestamp,
                    l2_stream=l2_ev.stream_id,
                    l2_event_type=l2_ev.event_type,
                    l2_bar_idx=l2_ev.bar_idx,
                    l2_timestamp=l2_ev.timestamp,
                    lag_bars=best_lag,
                    lag_days=lag_days,
                ))

    return pairs


def compute_aggregate_stats(
    pairs: list[TemporalPair],
) -> dict:
    """从配对中计算汇总统计。"""
    if not pairs:
        return {"total_pairs": 0}

    lags = [p.lag_bars for p in pairs]
    lag_days_list = [p.lag_days for p in pairs]

    # 按 L1 边分组
    by_l1: dict[str, list[int]] = {}
    for p in pairs:
        by_l1.setdefault(p.l1_stream, []).append(p.lag_bars)

    # 按 L2 边分组
    by_l2: dict[str, list[int]] = {}
    for p in pairs:
        by_l2.setdefault(p.l2_stream, []).append(p.lag_bars)

    # 按事件类型分组
    by_event_type: dict[str, list[int]] = {}
    for p in pairs:
        key = f"{p.l1_event_type}->{p.l2_event_type}"
        by_event_type.setdefault(key, []).append(p.lag_bars)

    def _stats(vals: list[int]) -> dict:
        if not vals:
            return {"count": 0}
        s = sorted(vals)
        n = len(s)
        return {
            "count": n,
            "mean": round(sum(s) / n, 2),
            "median": s[n // 2],
            "min": s[0],
            "max": s[-1],
            "p25": s[n // 4],
            "p75": s[3 * n // 4],
            "zero_lag_count": sum(1 for v in s if v == 0),
            "within_5_bars": sum(1 for v in s if v <= 5),
            "within_10_bars": sum(1 for v in s if v <= 10),
        }

    return {
        "total_pairs": len(pairs),
        "overall_lag_bars": _stats(lags),
        "overall_lag_days": _stats(lag_days_list),
        "by_l1_stream": {k: _stats(v) for k, v in sorted(by_l1.items())},
        "by_l2_stream": {k: _stats(v) for k, v in sorted(by_l2.items())},
        "by_event_type_pair": {k: _stats(v) for k, v in sorted(by_event_type.items())},
    }


def compute_lead_lag_summary(
    all_l1_events: dict[str, list[StructureEvent]],
    all_l2_events: dict[str, list[StructureEvent]],
) -> dict:
    """计算每对 L1-L2 之间谁先谁后的统计。

    对每对 (L1边, L2标的)，在公共 bar_idx 范围内，统计：
    - L1 事件先出现的比例
    - L2 事件先出现的比例
    - 同时出现的比例
    """
    summary: dict[str, dict] = {}

    for l1_name, l1_events in all_l1_events.items():
        for l2_name, l2_events in all_l2_events.items():
            # 提取该 L2 标的的成分（从比价边名称推断）
            pair_key = f"{l1_name} vs {l2_name}"

            l1_bar_set = {ev.bar_idx for ev in l1_events}
            l2_bar_set = {ev.bar_idx for ev in l2_events}

            # 逐 L1 事件找最近 L2 事件
            l1_leads = 0
            l2_leads = 0
            simultaneous = 0
            total_matched = 0
            lead_lags: list[int] = []

            l2_sorted = sorted(l2_events, key=lambda e: e.bar_idx)
            l2_bar_indices = [e.bar_idx for e in l2_sorted]

            for l1_ev in l1_events:
                # 二分搜索找最近的 L2 事件
                import bisect
                pos = bisect.bisect_left(l2_bar_indices, l1_ev.bar_idx)
                candidates = []
                if pos < len(l2_bar_indices):
                    candidates.append(l2_bar_indices[pos])
                if pos > 0:
                    candidates.append(l2_bar_indices[pos - 1])

                if not candidates:
                    continue

                nearest = min(candidates, key=lambda x: abs(x - l1_ev.bar_idx))
                lag = nearest - l1_ev.bar_idx  # 正=L1先, 负=L2先

                if abs(lag) <= 60:  # 60 bar 窗口
                    total_matched += 1
                    lead_lags.append(lag)
                    if lag > 0:
                        l1_leads += 1
                    elif lag < 0:
                        l2_leads += 1
                    else:
                        simultaneous += 1

            summary[pair_key] = {
                "l1_events": len(l1_events),
                "l2_events": len(l2_events),
                "matched": total_matched,
                "l1_leads": l1_leads,
                "l2_leads": l2_leads,
                "simultaneous": simultaneous,
                "l1_lead_pct": round(l1_leads / total_matched * 100, 1) if total_matched else 0,
                "l2_lead_pct": round(l2_leads / total_matched * 100, 1) if total_matched else 0,
                "simultaneous_pct": round(simultaneous / total_matched * 100, 1) if total_matched else 0,
                "mean_lag": round(sum(lead_lags) / len(lead_lags), 2) if lead_lags else 0,
                "median_lag": sorted(lead_lags)[len(lead_lags) // 2] if lead_lags else 0,
            }

    return summary


# ===================================================================
# 主流程
# ===================================================================


def main() -> None:
    print("=" * 70)
    print("OQ1 层间时序关系 -- 层1比价事件 vs 层2个股事件")
    print("口径 A (RecursiveOrchestrator 从日线递归)")
    print("=" * 70)

    # ── 1. 数据拉取 ──
    print("\n[1/5] 拉取数据")
    raw_data: dict[str, pd.DataFrame] = {}
    for label, ticker in L2_TICKERS.items():
        df = fetch_daily(ticker, label)
        if df is not None:
            raw_data[label] = df

    if len(raw_data) < 3:
        print("错误: 至少需要 SPY, GLD, TLT 数据")
        return

    # ── 2. 对齐到公共交易日 + 构造比价 Bar ──
    print("\n[2/5] 数据对齐 + 构造比价序列")

    # 找公共日期（所有层2标的）
    core_tickers = ["SPY", "GLD", "TLT"]
    common_dates = raw_data[core_tickers[0]].index
    for t in core_tickers[1:]:
        if t in raw_data:
            common_dates = common_dates.intersection(raw_data[t].index)
    common_dates = common_dates.sort_values()
    print(f"  核心公共交易日: {len(common_dates)} "
          f"({common_dates[0].date()} ~ {common_dates[-1].date()})")

    # 对齐层2数据
    aligned_l2: dict[str, pd.DataFrame] = {}
    for name in L2_TICKERS:
        if name in raw_data:
            df = raw_data[name]
            overlap = common_dates.intersection(df.index).sort_values()
            aligned_l2[name] = df.loc[overlap]
            print(f"  {name}: {len(aligned_l2[name])} bars (对齐后)")

    # 构造层2 Bar 列表
    l2_bars: dict[str, list[Bar]] = {}
    for name, df in aligned_l2.items():
        l2_bars[name] = df_to_bars(df)

    # 构造层1比价 Bar
    l1_bars: dict[str, list[Bar]] = {}
    for ratio_name, num_ticker, den_ticker in L1_RATIO_EDGES:
        if num_ticker not in l2_bars or den_ticker not in l2_bars:
            print(f"  跳过 {ratio_name}: 缺少 {num_ticker} 或 {den_ticker}")
            continue
        num_bars = l2_bars[num_ticker]
        den_bars = l2_bars[den_ticker]
        # 使用核心公共日期确保对齐
        min_len = min(len(num_bars), len(den_bars))
        ratio_bars = [
            make_ratio_bar(num_bars[i], den_bars[i])
            for i in range(min_len)
        ]
        l1_bars[ratio_name] = ratio_bars
        print(f"  {ratio_name}: {len(ratio_bars)} bars (比价)")

    # ── 3. 运行 D 算子并提取结构事件 ──
    print("\n[3/5] 运行 D 算子 + 提取结构事件")

    all_l1_events: dict[str, list[StructureEvent]] = {}
    all_l2_events: dict[str, list[StructureEvent]] = {}
    all_stats: dict[str, dict] = {}
    t0 = time.time()

    # 层1
    print("  --- 层1 (比价序列) ---")
    for i, (name, bars) in enumerate(l1_bars.items()):
        print(f"  [{i+1}/{len(l1_bars)}] {name} ({len(bars)} bars) ...",
              end=" ", flush=True)
        t_edge = time.time()
        events, stats = extract_events_from_stream(name, bars)
        elapsed = time.time() - t_edge
        all_l1_events[name] = events
        all_stats[f"L1_{name}"] = stats
        print(f"{elapsed:.1f}s | "
              f"reversals={stats['move_reversals']} "
              f"bsp={stats['bsp_confirmed']} "
              f"dir_changes={stats['direction_changes']}")

    # 层2
    print("  --- 层2 (个股/ETF序列) ---")
    for i, (name, bars) in enumerate(l2_bars.items()):
        print(f"  [{i+1}/{len(l2_bars)}] {name} ({len(bars)} bars) ...",
              end=" ", flush=True)
        t_edge = time.time()
        events, stats = extract_events_from_stream(name, bars)
        elapsed = time.time() - t_edge
        all_l2_events[name] = events
        all_stats[f"L2_{name}"] = stats
        print(f"{elapsed:.1f}s | "
              f"reversals={stats['move_reversals']} "
              f"bsp={stats['bsp_confirmed']} "
              f"dir_changes={stats['direction_changes']}")

    total_elapsed = time.time() - t0
    print(f"\n  全部流完成: {total_elapsed:.1f}s")

    # ── 4. 时序分析 ──
    print("\n[4/5] 时序分析")

    # 4a. 对每对 (L1, L2) 计算配对统计
    all_pairs: list[TemporalPair] = []
    for l1_name, l1_events in all_l1_events.items():
        for l2_name, l2_events in all_l2_events.items():
            pairs = compute_temporal_pairs(l1_events, l2_events, max_window_bars=60)
            all_pairs.extend(pairs)
            print(f"  {l1_name} <-> {l2_name}: {len(pairs)} 配对")

    aggregate = compute_aggregate_stats(all_pairs)
    print(f"  总配对数: {aggregate['total_pairs']}")
    if aggregate["total_pairs"] > 0:
        overall = aggregate["overall_lag_bars"]
        print(f"  lag (bars): mean={overall['mean']}, "
              f"median={overall['median']}, "
              f"min={overall['min']}, max={overall['max']}")

    # 4b. 谁领先谁的汇总
    lead_lag = compute_lead_lag_summary(all_l1_events, all_l2_events)
    print("\n  领先/滞后汇总:")
    for pair_key, stats in lead_lag.items():
        print(f"    {pair_key}: "
              f"L1先={stats['l1_lead_pct']}% "
              f"L2先={stats['l2_lead_pct']}% "
              f"同时={stats['simultaneous_pct']}% "
              f"(mean_lag={stats['mean_lag']} bars)")

    # ── 5. 保存结果 ──
    print("\n[5/5] 保存结果")
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    # 5a. 结构事件列表
    events_output: dict[str, list[dict]] = {}
    for name, events in {**all_l1_events, **all_l2_events}.items():
        events_output[name] = [
            {
                "stream_id": ev.stream_id,
                "bar_idx": ev.bar_idx,
                "timestamp": ev.timestamp.isoformat(),
                "event_type": ev.event_type,
                "detail": ev.detail,
            }
            for ev in events
        ]
    events_path = RESULTS_DIR / "structure_events.json"
    with open(events_path, "w", encoding="utf-8") as f:
        json.dump(events_output, f, indent=2, ensure_ascii=False)
    print(f"  结构事件 -> {events_path}")

    # 5b. 时序分析
    temporal_output = {
        "aggregate_stats": aggregate,
        "lead_lag_summary": lead_lag,
        "stream_stats": all_stats,
        "pairs_sample": [
            {
                "l1_stream": p.l1_stream,
                "l1_event_type": p.l1_event_type,
                "l1_bar_idx": p.l1_bar_idx,
                "l1_timestamp": p.l1_timestamp.isoformat(),
                "l2_stream": p.l2_stream,
                "l2_event_type": p.l2_event_type,
                "l2_bar_idx": p.l2_bar_idx,
                "l2_timestamp": p.l2_timestamp.isoformat(),
                "lag_bars": p.lag_bars,
                "lag_days": p.lag_days,
            }
            for p in all_pairs[:500]  # 前 500 对样本
        ],
        "total_pairs": len(all_pairs),
    }
    temporal_path = RESULTS_DIR / "temporal_analysis.json"
    with open(temporal_path, "w", encoding="utf-8") as f:
        json.dump(temporal_output, f, indent=2, ensure_ascii=False)
    print(f"  时序分析 -> {temporal_path}")

    # 5c. 人类可读摘要
    summary_lines = [
        "# OQ1 层间时序关系分析摘要",
        "",
        f"生成时间: {datetime.now().isoformat()}",
        f"认识论等级: L2（真实数据验证）",
        f"谱系引用: 279号（OQ1 层间关系假说）",
        "",
        "## 输入",
        "",
        f"- 层1比价边: {', '.join(l1_bars.keys())}",
        f"- 层2个股/ETF: {', '.join(l2_bars.keys())}",
        f"- 公共交易日: {len(common_dates)} "
        f"({common_dates[0].date()} ~ {common_dates[-1].date()})",
        "",
        "## 结构事件统计",
        "",
        "| 流 | 层 | 总 bar | 走势转折 | 买卖点确认 | 方向态变化 |",
        "|---|---|---|---|---|---|",
    ]
    for name, stats in sorted(all_stats.items()):
        layer = "L1" if name.startswith("L1_") else "L2"
        short_name = name[3:]
        summary_lines.append(
            f"| {short_name} | {layer} | {stats['total_bars']} | "
            f"{stats['move_reversals']} | {stats['bsp_confirmed']} | "
            f"{stats['direction_changes']} |"
        )

    summary_lines.extend([
        "",
        "## 时序配对统计",
        "",
        f"- 总配对数: {aggregate['total_pairs']}",
    ])
    if aggregate["total_pairs"] > 0:
        overall = aggregate["overall_lag_bars"]
        summary_lines.extend([
            f"- lag (bars): mean={overall['mean']}, median={overall['median']}, "
            f"min={overall['min']}, max={overall['max']}",
            f"- 0-bar lag: {overall['zero_lag_count']} 对 "
            f"({round(overall['zero_lag_count']/aggregate['total_pairs']*100,1)}%)",
            f"- <=5-bar lag: {overall['within_5_bars']} 对 "
            f"({round(overall['within_5_bars']/aggregate['total_pairs']*100,1)}%)",
            f"- <=10-bar lag: {overall['within_10_bars']} 对 "
            f"({round(overall['within_10_bars']/aggregate['total_pairs']*100,1)}%)",
        ])

    summary_lines.extend([
        "",
        "## 领先/滞后分析",
        "",
        "正 mean_lag = L1 领先 L2（比价事件先于个股事件）",
        "负 mean_lag = L2 领先 L1（个股事件先于比价事件）",
        "",
        "| L1边 vs L2标的 | L1先% | L2先% | 同时% | mean lag (bars) | median lag |",
        "|---|---|---|---|---|---|",
    ])
    for pair_key, stats in sorted(lead_lag.items()):
        summary_lines.append(
            f"| {pair_key} | {stats['l1_lead_pct']} | {stats['l2_lead_pct']} | "
            f"{stats['simultaneous_pct']} | {stats['mean_lag']} | "
            f"{stats['median_lag']} |"
        )

    summary_lines.extend([
        "",
        "## 结论（待数据填充）",
        "",
        "1. **结论**: 层1和层2事件的时序关系——需根据实际数据判断是否存在稳定先后关系",
        "2. **定义依据**: OQ1 层间关系假说（279号）",
        "3. **边界条件**: 数据长度限制（ETF 上市日期不同）、结构事件稀疏性",
        "4. **下游推论**: 如果层1领先层2，K4配置可作为层2操作的先行指标",
        "5. **谱系引用**: 279号",
        "6. **影响声明**: 实验结果，不影响现有代码",
    ])

    summary_path = RESULTS_DIR / "summary.md"
    with open(summary_path, "w", encoding="utf-8") as f:
        f.write("\n".join(summary_lines))
    print(f"  摘要 -> {summary_path}")

    # ── 打印总结 ──
    print("\n" + "=" * 70)
    print("OQ1 层间时序关系分析完成")
    print("=" * 70)
    print(f"  结果目录: {RESULTS_DIR}")
    print(f"  文件: structure_events.json, temporal_analysis.json, summary.md")
    print("\n完成。")


if __name__ == "__main__":
    main()
