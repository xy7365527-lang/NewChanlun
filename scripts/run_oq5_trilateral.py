"""OQ5 三重联立检测 -- 多边联立 bar-by-bar 滑动窗口。

279号谱系延续：从单品种结构统计升级为多边联立检测。
口径 A：RecursiveOrchestrator 从日线递归。

三重联立判据：
  1. 中心失构：美元腿 D 算子方向态衰减或进入盘整
  2. 外围升构：交叉盘从无走势变为有走势（从 ker(D) 移出）
  3. 法币/黄金共振：多条法币/黄金边同步获得方向态

实现方式：
  - 所有边各自创建一个 RecursiveOrchestrator
  - 逐 bar 推进，在每个 bar 提取方向态
  - 在每个 bar 检查三个条件是否同时满足

认识论等级：L2（真实数据验证）
谱系引用：279号（OQ5 口径A验证）
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

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.topology.k4_scanner import walk_direction_from_snapshot
from newchan.topology.config_space import WalkDirection
from newchan.types import Bar


# ===================================================================
# 边定义
# ===================================================================

# 中心边（美元腿）: DXY 或美元计价对
CENTER_TICKERS: dict[str, str] = {
    "DXY": "DX-Y.NYB",
    "XAU_USD": "GC=F",
    "USD_JPY": "JPY=X",
    "GBP_USD": "GBPUSD=X",
    "USD_CHF": "CHF=X",
}

# 外围边（交叉盘）: 直接 ticker 或合成标记
PERIPHERY_TICKERS: dict[str, str] = {
    "GBP_JPY": "GBPJPY=X",
    "EUR_CHF": "EURCHF=X",
}

# 合成交叉盘（从两条美元腿计算）
SYNTHETIC_EDGES: dict[str, tuple[str, str, str]] = {
    # name: (numerator_ticker, denominator_ticker, operation)
    # XAU/JPY = GC=F * JPY=X (gold_usd * usd_jpy)
    "XAU_JPY": ("GC=F", "JPY=X", "multiply"),
    # XAU/GBP = GC=F / GBPUSD=X
    "XAU_GBP": ("GC=F", "GBPUSD=X", "divide"),
    # XAU/CHF = GC=F * CHF=X (gold_usd * usd_chf)
    "XAU_CHF": ("GC=F", "CHF=X", "multiply"),
}

# 边分类
CENTER_EDGES = ["DXY", "USD_JPY", "GBP_USD", "USD_CHF"]
GOLD_EDGES = ["XAU_USD", "XAU_JPY", "XAU_GBP", "XAU_CHF"]
PERIPHERY_EDGES = ["GBP_JPY", "EUR_CHF"]

RESULTS_DIR = ROOT / "tmp" / "oq5-trilateral-results"


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
        # 去时区
        if df.index.tz is not None:
            df.index = df.index.tz_localize(None)
        print(f"{len(df)} bars ({df.index[0].date()} ~ {df.index[-1].date()})")
        return df
    except Exception as e:
        print(f"失败: {e}")
        return None


def synthesize_cross(
    df_a: pd.DataFrame,
    df_b: pd.DataFrame,
    operation: str,
) -> pd.DataFrame:
    """从两条美元腿合成交叉盘 OHLC。

    multiply: cross = A * B (e.g. XAU/JPY = GC=F * JPY=X)
    divide:   cross = A / B (e.g. XAU/GBP = GC=F / GBPUSD=X)
    """
    common = df_a.index.intersection(df_b.index).sort_values()
    a = df_a.loc[common]
    b = df_b.loc[common]

    if operation == "multiply":
        op = lambda x, y: x * y
    else:
        op = lambda x, y: x / y

    result = pd.DataFrame(index=common)
    result["Open"] = op(a["Open"].values, b["Open"].values)
    result["High"] = op(a["High"].values, b["High"].values)
    result["Low"] = op(a["Low"].values, b["Low"].values)
    result["Close"] = op(a["Close"].values, b["Close"].values)
    # Volume not meaningful for synthetic pairs
    result["Volume"] = 0.0
    return result


def df_to_bars(df: pd.DataFrame) -> list[Bar]:
    """DataFrame -> Bar list."""
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
# 方向态时间序列
# ===================================================================


@dataclass
class EdgeTimeSeries:
    """单条边的逐 bar 方向态时间序列。"""
    edge_name: str
    timestamps: list[datetime] = field(default_factory=list)
    directions: list[WalkDirection] = field(default_factory=list)
    # 统计
    total_bars: int = 0
    move_count_l1: int = 0
    seg_count_l1: int = 0
    zs_count_l1: int = 0


def run_edge(
    edge_name: str,
    bars: list[Bar],
) -> EdgeTimeSeries:
    """对单条边运行 RecursiveOrchestrator 并提取逐 bar 方向态。"""
    orch = RecursiveOrchestrator(stream_id=edge_name)
    ts_list: list[datetime] = []
    dir_list: list[WalkDirection] = []
    last_snap = None

    for bar in bars:
        snap = orch.process_bar(bar)
        direction = walk_direction_from_snapshot(snap, level=0)
        ts_list.append(bar.ts)
        dir_list.append(direction)
        last_snap = snap

    result = EdgeTimeSeries(
        edge_name=edge_name,
        timestamps=ts_list,
        directions=dir_list,
        total_bars=len(bars),
    )

    if last_snap is not None:
        result.move_count_l1 = len(last_snap.move_snapshot.moves)
        result.seg_count_l1 = len(last_snap.seg_snapshot.segments)
        result.zs_count_l1 = len(last_snap.zs_snapshot.zhongshus)

    return result


# ===================================================================
# 三重联立检测
# ===================================================================


@dataclass(frozen=True, slots=True)
class TrilateralEvent:
    """三重联立事件：三个条件同时满足的时间点。"""
    bar_idx: int
    timestamp: datetime
    center_state: dict[str, str]     # edge -> direction
    periphery_state: dict[str, str]  # edge -> direction
    gold_state: dict[str, str]       # edge -> direction


def detect_trilateral(
    edge_series: dict[str, EdgeTimeSeries],
    common_dates: pd.DatetimeIndex,
) -> tuple[list[TrilateralEvent], list[dict]]:
    """逐 bar 三重联立检测。

    条件:
    1. 中心失构: 美元腿方向态衰减 (DXY FLAT or DOWN, 或多条美元对 FLAT/DOWN)
    2. 外围升构: 交叉盘从 FLAT 变为 UP 或 DOWN (从 ker(D) 移出)
    3. 法币/黄金共振: 多条法币/黄金边同步获得方向态 (非 FLAT)

    Returns
    -------
    events : list[TrilateralEvent]
        满足三重联立的事件列表
    per_bar_diagnostics : list[dict]
        每个 bar 的逐条件诊断 (仅保存关键 bar, 不全存以免过大)
    """
    events: list[TrilateralEvent] = []
    diagnostics: list[dict] = []

    # 建立日期到索引的映射
    date_index: dict[str, dict[datetime, int]] = {}
    for name, series in edge_series.items():
        date_index[name] = {ts: i for i, ts in enumerate(series.timestamps)}

    n_bars = len(common_dates)
    prev_center_state: dict[str, WalkDirection] = {}
    prev_periphery_state: dict[str, WalkDirection] = {}

    for bar_i, dt in enumerate(common_dates):
        dt_py = dt.to_pydatetime() if hasattr(dt, "to_pydatetime") else dt

        # 读取各边在此 bar 的方向态
        center_state: dict[str, WalkDirection] = {}
        for edge_name in CENTER_EDGES:
            if edge_name in edge_series:
                idx_map = date_index[edge_name]
                if dt_py in idx_map:
                    center_state[edge_name] = edge_series[edge_name].directions[idx_map[dt_py]]

        periphery_state: dict[str, WalkDirection] = {}
        for edge_name in PERIPHERY_EDGES:
            if edge_name in edge_series:
                idx_map = date_index[edge_name]
                if dt_py in idx_map:
                    periphery_state[edge_name] = edge_series[edge_name].directions[idx_map[dt_py]]

        gold_state: dict[str, WalkDirection] = {}
        for edge_name in GOLD_EDGES:
            if edge_name in edge_series:
                idx_map = date_index[edge_name]
                if dt_py in idx_map:
                    gold_state[edge_name] = edge_series[edge_name].directions[idx_map[dt_py]]

        # ── 条件 1: 中心失构 ──
        # DXY 方向态衰减 = DXY 从 UP/DOWN 变为 FLAT，或当前为 FLAT/DOWN
        # 操作化: 多数美元腿处于 FLAT 或 DOWN
        center_flat_or_down = sum(
            1 for d in center_state.values()
            if d in (WalkDirection.FLAT, WalkDirection.DOWN)
        )
        total_center = len(center_state)
        center_breakdown = total_center > 0 and center_flat_or_down > total_center / 2

        # ── 条件 2: 外围升构 ──
        # 交叉盘从 FLAT 变为 UP/DOWN (ker(D) 移出)
        periphery_active = sum(
            1 for d in periphery_state.values()
            if d != WalkDirection.FLAT
        )
        periphery_construction = periphery_active > 0

        # ── 条件 3: 法币/黄金共振 ──
        # 多条法币/黄金边同步获得方向态
        gold_active = sum(
            1 for d in gold_state.values()
            if d != WalkDirection.FLAT
        )
        total_gold = len(gold_state)
        gold_resonance = total_gold > 0 and gold_active >= max(1, total_gold // 2)

        triple_met = center_breakdown and periphery_construction and gold_resonance

        if triple_met:
            events.append(TrilateralEvent(
                bar_idx=bar_i,
                timestamp=dt_py,
                center_state={k: v.name for k, v in center_state.items()},
                periphery_state={k: v.name for k, v in periphery_state.items()},
                gold_state={k: v.name for k, v in gold_state.items()},
            ))

        # 每 500 bar 或三重联立时记录诊断
        if bar_i % 500 == 0 or triple_met:
            diagnostics.append({
                "bar_idx": bar_i,
                "timestamp": dt_py.isoformat(),
                "center": {k: v.name for k, v in center_state.items()},
                "periphery": {k: v.name for k, v in periphery_state.items()},
                "gold": {k: v.name for k, v in gold_state.items()},
                "center_breakdown": center_breakdown,
                "periphery_construction": periphery_construction,
                "gold_resonance": gold_resonance,
                "triple_met": triple_met,
            })

        prev_center_state = center_state
        prev_periphery_state = periphery_state

    return events, diagnostics


# ===================================================================
# 主流程
# ===================================================================


def main() -> None:
    print("=" * 70)
    print("OQ5 三重联立检测 -- 多边联立 bar-by-bar")
    print("口径 A (RecursiveOrchestrator 从日线递归)")
    print("=" * 70)

    # ── 1. 数据拉取 ──
    print("\n[1/5] 拉取数据")
    raw_data: dict[str, pd.DataFrame] = {}

    # 直接 ticker
    all_tickers = {**CENTER_TICKERS, **PERIPHERY_TICKERS}
    for label, ticker in all_tickers.items():
        df = fetch_daily(ticker, label)
        if df is not None:
            raw_data[label] = df

    # 合成交叉盘
    print("\n  合成交叉盘:")
    for synth_name, (ticker_a, ticker_b, op) in SYNTHETIC_EDGES.items():
        # 找到已拉取的数据
        df_a = None
        df_b = None
        for label, ticker in all_tickers.items():
            if ticker == ticker_a and label in raw_data:
                df_a = raw_data[label]
            if ticker == ticker_b and label in raw_data:
                df_b = raw_data[label]
        if df_a is not None and df_b is not None:
            synth_df = synthesize_cross(df_a, df_b, op)
            raw_data[synth_name] = synth_df
            print(f"  [{synth_name}] 合成 {len(synth_df)} bars "
                  f"({synth_df.index[0].date()} ~ {synth_df.index[-1].date()})")
        else:
            print(f"  [{synth_name}] 跳过 (缺少组件数据)")

    available_edges = list(raw_data.keys())
    print(f"\n  可用边: {available_edges} ({len(available_edges)} 条)")

    # ── 2. 转换为 Bar 列表 ──
    print("\n[2/5] 转换 Bar 数据")
    bar_streams: dict[str, list[Bar]] = {}
    for edge_name, df in raw_data.items():
        bars = df_to_bars(df)
        bar_streams[edge_name] = bars
        print(f"  {edge_name}: {len(bars)} bars")

    # ── 3. 逐边运行 D 算子 ──
    print("\n[3/5] 运行 D 算子 (逐边)")
    edge_results: dict[str, EdgeTimeSeries] = {}
    t0 = time.time()

    for i, (edge_name, bars) in enumerate(bar_streams.items()):
        print(f"  [{i+1}/{len(bar_streams)}] {edge_name} ({len(bars)} bars) ...",
              end=" ", flush=True)
        t_edge = time.time()
        result = run_edge(edge_name, bars)
        elapsed = time.time() - t_edge
        edge_results[edge_name] = result
        print(f"{elapsed:.1f}s | "
              f"moves={result.move_count_l1} segs={result.seg_count_l1} "
              f"zs={result.zs_count_l1}")

    total_elapsed = time.time() - t0
    print(f"\n  全部边完成: {total_elapsed:.1f}s")

    # ── 4. 三重联立检测 ──
    print("\n[4/5] 三重联立检测")

    # 找公共日期范围
    all_dates: set[datetime] = set()
    for edge_name, series in edge_results.items():
        all_dates.update(series.timestamps)
    common_dates = pd.DatetimeIndex(sorted(all_dates))
    print(f"  日期范围: {common_dates[0].date()} ~ {common_dates[-1].date()} "
          f"({len(common_dates)} 个交易日)")

    events, diagnostics = detect_trilateral(edge_results, common_dates)
    print(f"  三重联立事件: {len(events)} 个")

    if events:
        print("\n  三重联立事件列表:")
        for ev in events[:50]:  # 前 50 个
            print(f"    [{ev.timestamp.date()}] "
                  f"center={ev.center_state} "
                  f"periphery={ev.periphery_state} "
                  f"gold={ev.gold_state}")
        if len(events) > 50:
            print(f"    ... ({len(events) - 50} more)")

    # ── 4a. 2002-2007 窗口诊断 ──
    print("\n  2002-2007 窗口逐条件诊断:")
    window_events = [
        ev for ev in events
        if 2002 <= ev.timestamp.year <= 2007
    ]
    print(f"    2002-2007 窗口内三重联立事件: {len(window_events)} 个")

    window_diag = [
        d for d in diagnostics
        if d["timestamp"][:4] in ("2002", "2003", "2004", "2005", "2006", "2007")
    ]
    if window_diag:
        print("    抽样诊断:")
        for d in window_diag[:20]:
            print(f"      [{d['timestamp'][:10]}] "
                  f"center_breakdown={d['center_breakdown']} "
                  f"periphery_construction={d['periphery_construction']} "
                  f"gold_resonance={d['gold_resonance']} "
                  f"triple={d['triple_met']}")

    # ── 5. 保存结果 ──
    print("\n[5/5] 保存结果")
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    # 5a. 每条边的走势产出摘要
    edge_summary = {}
    for name, series in edge_results.items():
        # 方向态分布
        dir_counts = {"UP": 0, "DOWN": 0, "FLAT": 0}
        for d in series.directions:
            dir_counts[d.name] += 1

        # 方向态变化计数
        transitions = 0
        for i in range(1, len(series.directions)):
            if series.directions[i] != series.directions[i - 1]:
                transitions += 1

        edge_summary[name] = {
            "total_bars": series.total_bars,
            "move_count_l1": series.move_count_l1,
            "seg_count_l1": series.seg_count_l1,
            "zs_count_l1": series.zs_count_l1,
            "direction_distribution": dir_counts,
            "direction_transitions": transitions,
            "date_range": (
                f"{series.timestamps[0].date()} ~ {series.timestamps[-1].date()}"
                if series.timestamps else "N/A"
            ),
        }

    summary_path = RESULTS_DIR / "edge_summary.json"
    with open(summary_path, "w", encoding="utf-8") as f:
        json.dump(edge_summary, f, indent=2, ensure_ascii=False)
    print(f"  边摘要 -> {summary_path}")

    # 5b. 三重联立事件
    events_data = [
        {
            "bar_idx": ev.bar_idx,
            "timestamp": ev.timestamp.isoformat(),
            "center_state": ev.center_state,
            "periphery_state": ev.periphery_state,
            "gold_state": ev.gold_state,
        }
        for ev in events
    ]
    events_path = RESULTS_DIR / "trilateral_events.json"
    with open(events_path, "w", encoding="utf-8") as f:
        json.dump(events_data, f, indent=2, ensure_ascii=False)
    print(f"  三重联立事件 -> {events_path}")

    # 5c. 逐条件诊断
    diag_path = RESULTS_DIR / "diagnostics.json"
    with open(diag_path, "w", encoding="utf-8") as f:
        json.dump(diagnostics, f, indent=2, ensure_ascii=False)
    print(f"  诊断数据 -> {diag_path}")

    # 5d. 时间窗口统计
    window_stats = {}
    # 按年份分段统计三重联立事件
    for ev in events:
        year = ev.timestamp.year
        if year not in window_stats:
            window_stats[year] = {"count": 0, "dates": []}
        window_stats[year]["count"] += 1
        window_stats[year]["dates"].append(ev.timestamp.strftime("%Y-%m-%d"))

    stats_path = RESULTS_DIR / "window_stats.json"
    with open(stats_path, "w", encoding="utf-8") as f:
        json.dump(window_stats, f, indent=2, ensure_ascii=False)
    print(f"  窗口统计 -> {stats_path}")

    # 5e. 完整结果包
    result_package = {
        "verification_time": datetime.now().isoformat(),
        "pipeline": "口径A（RecursiveOrchestrator 从日线递归）",
        "epistemological_level": "L2（真实数据验证）",
        "genealogy_ref": "279号（OQ5 口径A验证）",
        "available_edges": available_edges,
        "total_bars_range": (
            f"{common_dates[0].date()} ~ {common_dates[-1].date()}"
        ),
        "total_trading_days": len(common_dates),
        "trilateral_event_count": len(events),
        "window_2002_2007": {
            "event_count": len(window_events),
            "expected": "反例（三重联立应不满足）",
            "actual": (
                "不满足（符合预期）" if len(window_events) == 0
                else f"满足 {len(window_events)} 次（不符合预期）"
            ),
        },
        "trilateral_conditions": {
            "center_breakdown": "美元腿多数处于 FLAT 或 DOWN",
            "periphery_construction": "至少一条交叉盘处于非 FLAT",
            "gold_resonance": "法币/黄金边半数以上处于非 FLAT",
        },
        "edge_summary": edge_summary,
    }
    result_path = RESULTS_DIR / "result_package.json"
    with open(result_path, "w", encoding="utf-8") as f:
        json.dump(result_package, f, indent=2, ensure_ascii=False)
    print(f"  结果包 -> {result_path}")

    # ── 打印总结 ──
    print("\n" + "=" * 70)
    print("OQ5 三重联立检测结果")
    print("=" * 70)
    print(f"  可用边: {len(available_edges)} 条")
    print(f"  总交易日: {len(common_dates)}")
    print(f"  三重联立事件: {len(events)} 个")
    if window_stats:
        print(f"  按年份分布:")
        for year in sorted(window_stats.keys()):
            print(f"    {year}: {window_stats[year]['count']} 次")
    print(f"  2002-2007 窗口: {len(window_events)} 个事件")
    print(f"\n  结果目录: {RESULTS_DIR}")
    print("\n完成。")


if __name__ == "__main__":
    main()
