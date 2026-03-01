"""OQ4 黄金截面 K3 -- 三条边走势 vs K4 六条边对比。

279号谱系延续：用黄金作为尺子，在 K3 截面下跑 D 算子，与 K4 六条边对比。

K3 截面（黄金计价）的三条边：
  1. SPY/GLD — E 用黄金量
  2. TLT/GLD — R 用黄金量
  3. SPY/TLT — E/R 不经过黄金，保留

K4 退化为 K3：E、R、Au/$ 三个顶点，三条边。

K4 对照组（美元计价）六条边：
  顶点边：SPY/$、GLD/$、TLT/$
  比价边：SPY/GLD、TLT/GLD、SPY/TLT

对比分析：K3 三条边 vs K4 六条边的方向态一致性。

认识论等级：L2（真实数据验证）
谱系引用：279号（OQ4 黄金截面假说）
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
# 常量
# ===================================================================

TICKERS = {
    "SPY": "SPY",
    "GLD": "GLD",
    "TLT": "TLT",
}

# K3 边（黄金计价）
K3_EDGES = ["SPY_GLD", "TLT_GLD", "SPY_TLT"]

# K4 边（美元计价 + 比价）
K4_VERTEX_EDGES = ["SPY", "GLD", "TLT"]               # 顶点→现金
K4_RATIO_EDGES = ["SPY_GLD", "TLT_GLD", "SPY_TLT"]    # 比价边
K4_ALL_EDGES = K4_VERTEX_EDGES + K4_RATIO_EDGES

RESULTS_DIR = ROOT / "tmp" / "oq4-gold-k3-results"


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


def make_ratio_bars(
    bars_a: list[Bar],
    bars_b: list[Bar],
    ts_a: list[datetime],
    ts_b: list[datetime],
) -> tuple[list[Bar], list[datetime]]:
    """构造比价 Bar 序列。

    按编排者指令：
      ratio_open  = a.open / b.open
      ratio_close = a.close / b.close
      all_ratios  = [ratio_open, ratio_close, a.high/b.low, a.low/b.high]
      high = max(all_ratios), low = min(all_ratios)
    """
    # 建立日期映射
    idx_b = {ts: i for i, ts in enumerate(ts_b)}
    result_bars: list[Bar] = []
    result_ts: list[datetime] = []

    for i, ts in enumerate(ts_a):
        if ts not in idx_b:
            continue
        j = idx_b[ts]
        a = bars_a[i]
        b = bars_b[j]

        if b.open == 0 or b.close == 0 or b.high == 0 or b.low == 0:
            continue

        ratio_open = a.open / b.open
        ratio_close = a.close / b.close
        all_ratios = [
            ratio_open,
            ratio_close,
            a.high / b.low,
            a.low / b.high,
        ]
        result_bars.append(Bar(
            ts=ts,
            open=ratio_open,
            high=max(all_ratios),
            low=min(all_ratios),
            close=ratio_close,
            volume=None,
        ))
        result_ts.append(ts)

    return result_bars, result_ts


def df_to_bars(df: pd.DataFrame) -> tuple[list[Bar], list[datetime]]:
    """DataFrame -> (Bar list, timestamp list)."""
    bars: list[Bar] = []
    timestamps: list[datetime] = []
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
        timestamps.append(ts)
    return bars, timestamps


# ===================================================================
# D 算子运行
# ===================================================================


@dataclass
class EdgeResult:
    """单条边的逐 bar 方向态时间序列。"""
    edge_name: str
    timestamps: list[datetime] = field(default_factory=list)
    directions: list[WalkDirection] = field(default_factory=list)
    total_bars: int = 0
    move_count_l1: int = 0
    seg_count_l1: int = 0
    zs_count_l1: int = 0


def run_edge(edge_name: str, bars: list[Bar]) -> EdgeResult:
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

    result = EdgeResult(
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
# 对比分析
# ===================================================================


def direction_distribution(directions: list[WalkDirection]) -> dict[str, int]:
    """方向态分布统计。"""
    counts = {"UP": 0, "DOWN": 0, "FLAT": 0}
    for d in directions:
        counts[d.name] += 1
    return counts


def direction_transitions(directions: list[WalkDirection]) -> int:
    """方向态变化次数。"""
    n = 0
    for i in range(1, len(directions)):
        if directions[i] != directions[i - 1]:
            n += 1
    return n


def compare_k3_k4(
    k3_results: dict[str, EdgeResult],
    k4_results: dict[str, EdgeResult],
    common_dates: list[datetime],
) -> dict:
    """K3 vs K4 逐 bar 对比。

    对比维度：
    1. 共享边（SPY/GLD, TLT/GLD, SPY/TLT）方向态是否一致
    2. K4 多出的顶点边（SPY, GLD, TLT）信息量
    3. 方向态转换时间差
    """
    # 建立日期→索引映射
    k3_idx: dict[str, dict[datetime, int]] = {}
    for name, r in k3_results.items():
        k3_idx[name] = {ts: i for i, ts in enumerate(r.timestamps)}

    k4_idx: dict[str, dict[datetime, int]] = {}
    for name, r in k4_results.items():
        k4_idx[name] = {ts: i for i, ts in enumerate(r.timestamps)}

    # 共享边一致性
    shared_edges = ["SPY_GLD", "TLT_GLD", "SPY_TLT"]
    agreement_counts: dict[str, int] = {}
    disagreement_counts: dict[str, int] = {}
    disagreement_samples: dict[str, list[dict]] = {}
    total_comparable: dict[str, int] = {}

    for edge in shared_edges:
        agree = 0
        disagree = 0
        total = 0
        samples: list[dict] = []

        if edge not in k3_idx or edge not in k4_idx:
            continue

        k3_map = k3_idx[edge]
        k4_map = k4_idx[edge]

        for dt in common_dates:
            if dt not in k3_map or dt not in k4_map:
                continue
            total += 1
            d_k3 = k3_results[edge].directions[k3_map[dt]]
            d_k4 = k4_results[edge].directions[k4_map[dt]]

            if d_k3 == d_k4:
                agree += 1
            else:
                disagree += 1
                if len(samples) < 100:
                    samples.append({
                        "date": dt.strftime("%Y-%m-%d"),
                        "k3": d_k3.name,
                        "k4": d_k4.name,
                    })

        agreement_counts[edge] = agree
        disagreement_counts[edge] = disagree
        total_comparable[edge] = total
        disagreement_samples[edge] = samples

    # K4 顶点边信息（K3 不含）
    vertex_info: dict[str, dict] = {}
    for edge in K4_VERTEX_EDGES:
        if edge in k4_results:
            r = k4_results[edge]
            vertex_info[edge] = {
                "direction_distribution": direction_distribution(r.directions),
                "transitions": direction_transitions(r.directions),
                "total_bars": r.total_bars,
            }

    # 方向态同步性：在每个 bar 统计 K3 和 K4 各有多少边非 FLAT
    k3_active_counts: list[int] = []
    k4_active_counts: list[int] = []
    for dt in common_dates:
        k3_active = 0
        for edge in K3_EDGES:
            if edge in k3_idx and dt in k3_idx[edge]:
                d = k3_results[edge].directions[k3_idx[edge][dt]]
                if d != WalkDirection.FLAT:
                    k3_active += 1
        k3_active_counts.append(k3_active)

        k4_active = 0
        for edge in K4_ALL_EDGES:
            if edge in k4_idx and dt in k4_idx[edge]:
                d = k4_results[edge].directions[k4_idx[edge][dt]]
                if d != WalkDirection.FLAT:
                    k4_active += 1
        k4_active_counts.append(k4_active)

    return {
        "shared_edge_agreement": {
            edge: {
                "agree": agreement_counts.get(edge, 0),
                "disagree": disagreement_counts.get(edge, 0),
                "total": total_comparable.get(edge, 0),
                "agreement_pct": (
                    round(agreement_counts.get(edge, 0) / total_comparable[edge] * 100, 2)
                    if total_comparable.get(edge, 0) > 0 else 0
                ),
            }
            for edge in shared_edges
        },
        "disagreement_samples": disagreement_samples,
        "k4_vertex_info": vertex_info,
        "active_edge_stats": {
            "k3_mean_active": round(sum(k3_active_counts) / max(len(k3_active_counts), 1), 3),
            "k4_mean_active": round(sum(k4_active_counts) / max(len(k4_active_counts), 1), 3),
            "k3_max_active": max(k3_active_counts) if k3_active_counts else 0,
            "k4_max_active": max(k4_active_counts) if k4_active_counts else 0,
        },
    }


# ===================================================================
# 结果序列化
# ===================================================================


def serialize_edge_result(r: EdgeResult) -> dict:
    """EdgeResult -> JSON-serializable dict."""
    return {
        "edge_name": r.edge_name,
        "total_bars": r.total_bars,
        "move_count_l1": r.move_count_l1,
        "seg_count_l1": r.seg_count_l1,
        "zs_count_l1": r.zs_count_l1,
        "direction_distribution": direction_distribution(r.directions),
        "direction_transitions": direction_transitions(r.directions),
        "date_range": (
            f"{r.timestamps[0].date()} ~ {r.timestamps[-1].date()}"
            if r.timestamps else "N/A"
        ),
    }


# ===================================================================
# 主流程
# ===================================================================


def main() -> None:
    print("=" * 70)
    print("OQ4 黄金截面 K3 -- 三条边走势 vs K4 六条边对比")
    print("口径 A (RecursiveOrchestrator 从日线递归)")
    print("=" * 70)

    # ── 1. 拉取原始数据 ──
    print("\n[1/6] 拉取数据")
    raw_data: dict[str, pd.DataFrame] = {}
    for label, ticker in TICKERS.items():
        df = fetch_daily(ticker, label)
        if df is not None:
            raw_data[label] = df

    missing = [k for k in TICKERS if k not in raw_data]
    if missing:
        print(f"\n  缺失数据: {missing}，无法继续。")
        sys.exit(1)

    # ── 2. 对齐到公共交易日 ──
    print("\n[2/6] 对齐公共交易日")
    common_idx = raw_data["SPY"].index
    for label in ("GLD", "TLT"):
        common_idx = common_idx.intersection(raw_data[label].index)
    common_idx = common_idx.sort_values()
    print(f"  公共交易日: {len(common_idx)} "
          f"({common_idx[0].date()} ~ {common_idx[-1].date()})")

    # 截取公共日期
    aligned: dict[str, pd.DataFrame] = {}
    for label in TICKERS:
        aligned[label] = raw_data[label].loc[common_idx]

    # ── 3. 构造 Bar 流 ──
    print("\n[3/6] 构造 Bar 流")

    # 原始品种 Bars
    raw_bars: dict[str, tuple[list[Bar], list[datetime]]] = {}
    for label in TICKERS:
        bars, ts = df_to_bars(aligned[label])
        raw_bars[label] = (bars, ts)
        print(f"  {label}: {len(bars)} bars")

    # K3 比价 Bars（黄金计价）
    k3_bars: dict[str, tuple[list[Bar], list[datetime]]] = {}
    spy_bars, spy_ts = raw_bars["SPY"]
    gld_bars, gld_ts = raw_bars["GLD"]
    tlt_bars, tlt_ts = raw_bars["TLT"]

    # SPY/GLD
    ratio_bars, ratio_ts = make_ratio_bars(spy_bars, gld_bars, spy_ts, gld_ts)
    k3_bars["SPY_GLD"] = (ratio_bars, ratio_ts)
    print(f"  SPY/GLD (K3): {len(ratio_bars)} bars")

    # TLT/GLD
    ratio_bars, ratio_ts = make_ratio_bars(tlt_bars, gld_bars, tlt_ts, gld_ts)
    k3_bars["TLT_GLD"] = (ratio_bars, ratio_ts)
    print(f"  TLT/GLD (K3): {len(ratio_bars)} bars")

    # SPY/TLT（共享边）
    ratio_bars, ratio_ts = make_ratio_bars(spy_bars, tlt_bars, spy_ts, tlt_ts)
    k3_bars["SPY_TLT"] = (ratio_bars, ratio_ts)
    print(f"  SPY/TLT (K3): {len(ratio_bars)} bars")

    # K4 比价 Bars（与 K3 使用相同的比价构造法）
    k4_ratio_bars: dict[str, tuple[list[Bar], list[datetime]]] = {}
    # SPY/GLD（重复构造以确保 K4 用同一逻辑）
    ratio_bars, ratio_ts = make_ratio_bars(spy_bars, gld_bars, spy_ts, gld_ts)
    k4_ratio_bars["SPY_GLD"] = (ratio_bars, ratio_ts)
    # TLT/GLD
    ratio_bars, ratio_ts = make_ratio_bars(tlt_bars, gld_bars, tlt_ts, gld_ts)
    k4_ratio_bars["TLT_GLD"] = (ratio_bars, ratio_ts)
    # SPY/TLT
    ratio_bars, ratio_ts = make_ratio_bars(spy_bars, tlt_bars, spy_ts, tlt_ts)
    k4_ratio_bars["SPY_TLT"] = (ratio_bars, ratio_ts)

    # ── 4. 运行 D 算子 ──
    print("\n[4/6] 运行 D 算子")

    # K3: 三条比价边
    print("\n  --- K3 截面（黄金计价三条边）---")
    k3_results: dict[str, EdgeResult] = {}
    t0 = time.time()
    for edge_name in K3_EDGES:
        bars_for_edge, _ = k3_bars[edge_name]
        print(f"  {edge_name} ({len(bars_for_edge)} bars) ...", end=" ", flush=True)
        t_edge = time.time()
        result = run_edge(f"K3_{edge_name}", bars_for_edge)
        elapsed = time.time() - t_edge
        k3_results[edge_name] = result
        print(f"{elapsed:.1f}s | "
              f"moves={result.move_count_l1} segs={result.seg_count_l1} "
              f"zs={result.zs_count_l1}")
    k3_time = time.time() - t0
    print(f"  K3 完成: {k3_time:.1f}s")

    # K4: 三条顶点边 + 三条比价边
    print("\n  --- K4 对照组（美元计价六条边）---")
    k4_results: dict[str, EdgeResult] = {}
    t0 = time.time()

    # 顶点边（原始品种）
    for label in K4_VERTEX_EDGES:
        bars_for_edge, _ = raw_bars[label]
        print(f"  {label} ({len(bars_for_edge)} bars) ...", end=" ", flush=True)
        t_edge = time.time()
        result = run_edge(f"K4_{label}", bars_for_edge)
        elapsed = time.time() - t_edge
        k4_results[label] = result
        print(f"{elapsed:.1f}s | "
              f"moves={result.move_count_l1} segs={result.seg_count_l1} "
              f"zs={result.zs_count_l1}")

    # 比价边
    for edge_name in K4_RATIO_EDGES:
        bars_for_edge, _ = k4_ratio_bars[edge_name]
        print(f"  {edge_name} ({len(bars_for_edge)} bars) ...", end=" ", flush=True)
        t_edge = time.time()
        result = run_edge(f"K4_{edge_name}", bars_for_edge)
        elapsed = time.time() - t_edge
        k4_results[edge_name] = result
        print(f"{elapsed:.1f}s | "
              f"moves={result.move_count_l1} segs={result.seg_count_l1} "
              f"zs={result.zs_count_l1}")

    k4_time = time.time() - t0
    print(f"  K4 完成: {k4_time:.1f}s")

    # ── 5. 对比分析 ──
    print("\n[5/6] 对比分析")

    # 公共日期（取 K3 和 K4 共享边的交集）
    all_ts_sets = []
    for edge in K3_EDGES:
        if edge in k3_results:
            all_ts_sets.append(set(k3_results[edge].timestamps))
    for edge in K4_ALL_EDGES:
        if edge in k4_results:
            all_ts_sets.append(set(k4_results[edge].timestamps))
    if all_ts_sets:
        common_ts = sorted(set.intersection(*all_ts_sets))
    else:
        common_ts = []
    print(f"  对比窗口: {len(common_ts)} 个交易日")
    if common_ts:
        print(f"  范围: {common_ts[0].date()} ~ {common_ts[-1].date()}")

    comparison = compare_k3_k4(k3_results, k4_results, common_ts)

    # 打印共享边一致性
    print("\n  共享边方向态一致性:")
    for edge, stats in comparison["shared_edge_agreement"].items():
        print(f"    {edge}: {stats['agreement_pct']}% 一致 "
              f"({stats['agree']}/{stats['total']} bars)")

    # 打印 K4 顶点边信息
    print("\n  K4 顶点边（K3 不含）:")
    for edge, info in comparison["k4_vertex_info"].items():
        dist = info["direction_distribution"]
        print(f"    {edge}: UP={dist['UP']} DOWN={dist['DOWN']} FLAT={dist['FLAT']} "
              f"transitions={info['transitions']}")

    # 打印活跃边统计
    stats = comparison["active_edge_stats"]
    print(f"\n  活跃边平均数: K3={stats['k3_mean_active']}/3 "
          f"K4={stats['k4_mean_active']}/6")

    # ── 6. 保存结果 ──
    print("\n[6/6] 保存结果")
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    # 6a. K3 结构
    k3_data = {name: serialize_edge_result(r) for name, r in k3_results.items()}
    k3_path = RESULTS_DIR / "k3_structure.json"
    with open(k3_path, "w", encoding="utf-8") as f:
        json.dump(k3_data, f, indent=2, ensure_ascii=False)
    print(f"  K3 结构 -> {k3_path}")

    # 6b. K4 参照
    k4_data = {name: serialize_edge_result(r) for name, r in k4_results.items()}
    k4_path = RESULTS_DIR / "k4_reference.json"
    with open(k4_path, "w", encoding="utf-8") as f:
        json.dump(k4_data, f, indent=2, ensure_ascii=False)
    print(f"  K4 参照 -> {k4_path}")

    # 6c. 对比
    comp_path = RESULTS_DIR / "comparison.json"
    with open(comp_path, "w", encoding="utf-8") as f:
        json.dump(comparison, f, indent=2, ensure_ascii=False)
    print(f"  对比结果 -> {comp_path}")

    # 6d. 分析摘要
    summary_lines = [
        "# OQ4 黄金截面 K3 vs K4 对比分析",
        "",
        "## 实验参数",
        "",
        f"- 数据范围: {common_ts[0].date()} ~ {common_ts[-1].date()}" if common_ts else "- 数据范围: N/A",
        f"- 公共交易日: {len(common_ts)}",
        f"- K3 边数: {len(K3_EDGES)} (SPY/GLD, TLT/GLD, SPY/TLT)",
        f"- K4 边数: {len(K4_ALL_EDGES)} (SPY, GLD, TLT, SPY/GLD, TLT/GLD, SPY/TLT)",
        "",
        "## 比价 Bar 构造",
        "",
        "```",
        "ratio_open  = a.open / b.open",
        "ratio_close = a.close / b.close",
        "all_ratios  = [ratio_open, ratio_close, a.high/b.low, a.low/b.high]",
        "high = max(all_ratios), low = min(all_ratios)",
        "```",
        "",
        "## K3 截面结构",
        "",
    ]
    for name, r in k3_results.items():
        dist = direction_distribution(r.directions)
        trans = direction_transitions(r.directions)
        summary_lines.append(
            f"- **{name}**: {r.total_bars} bars | "
            f"UP={dist['UP']} DOWN={dist['DOWN']} FLAT={dist['FLAT']} | "
            f"transitions={trans} | "
            f"moves={r.move_count_l1} segs={r.seg_count_l1} zs={r.zs_count_l1}"
        )

    summary_lines.extend([
        "",
        "## K4 对照组结构",
        "",
    ])
    for name, r in k4_results.items():
        dist = direction_distribution(r.directions)
        trans = direction_transitions(r.directions)
        tag = "(顶点边)" if name in K4_VERTEX_EDGES else "(比价边)"
        summary_lines.append(
            f"- **{name}** {tag}: {r.total_bars} bars | "
            f"UP={dist['UP']} DOWN={dist['DOWN']} FLAT={dist['FLAT']} | "
            f"transitions={trans} | "
            f"moves={r.move_count_l1} segs={r.seg_count_l1} zs={r.zs_count_l1}"
        )

    summary_lines.extend([
        "",
        "## 共享边方向态一致性",
        "",
        "K3 和 K4 都包含三条比价边（SPY/GLD, TLT/GLD, SPY/TLT）。",
        "由于两个截面使用相同的比价 Bar 构造法和相同的 RecursiveOrchestrator，",
        "理论上共享边应完全一致。不一致则说明流 ID 等非数据因素影响了引擎行为。",
        "",
    ])
    for edge, s in comparison["shared_edge_agreement"].items():
        summary_lines.append(
            f"- **{edge}**: {s['agreement_pct']}% 一致 "
            f"({s['agree']}/{s['total']})"
        )
        if s["disagree"] > 0:
            samples = comparison["disagreement_samples"].get(edge, [])
            summary_lines.append(f"  - 不一致样本 (前{min(len(samples), 10)}个):")
            for sample in samples[:10]:
                summary_lines.append(
                    f"    - {sample['date']}: K3={sample['k3']} K4={sample['k4']}"
                )

    summary_lines.extend([
        "",
        "## K3 少了什么（相对 K4）",
        "",
        "K4 有六条边，K3 只有三条边。K3 缺少的三条顶点边（SPY/$, GLD/$, TLT/$）",
        "携带的方向态信息：",
        "",
    ])
    for edge, info in comparison["k4_vertex_info"].items():
        dist = info["direction_distribution"]
        summary_lines.append(
            f"- **{edge}**: UP={dist['UP']} DOWN={dist['DOWN']} FLAT={dist['FLAT']} "
            f"transitions={info['transitions']}"
        )

    summary_lines.extend([
        "",
        "## K3 多了什么（黄金计价的优势）",
        "",
        "黄金计价消除了美元噪声。在 K4 中，SPY/$ 和 GLD/$ 的方向态可能同时被",
        "美元波动驱动（假方向态），但 SPY/GLD 直接反映权益相对黄金的真实购买力变化。",
        "",
        f"活跃边平均数: K3={stats['k3_mean_active']}/3 "
        f"K4={stats['k4_mean_active']}/6",
        "",
    ])

    # K3 截面完全分类规则初稿
    summary_lines.extend([
        "## K3 截面完全分类规则（初稿）",
        "",
        "K3 退化为三顶点（E, R, Au）三条边的完备图 K3。",
        "每条边有三种方向态 {UP, DOWN, FLAT}，理论配置空间 3^3 = 27。",
        "",
        "### 约束",
        "",
        "1. SPY/GLD * GLD/TLT = SPY/TLT（传递性约束）",
        "   - 但 D 算子在每条边独立运行，不保证代数传递性",
        "   - 因此 27 种配置中，违反传递性的配置可能在实际中出现",
        "",
        "### 语义分类",
        "",
        "| 配置 | SPY/GLD | TLT/GLD | SPY/TLT | 含义 |",
        "|------|---------|---------|---------|------|",
        "| A1   | UP      | DOWN    | UP      | 权益强于黄金强于债券 |",
        "| A2   | DOWN    | UP      | DOWN    | 债券强于黄金强于权益 |",
        "| A3   | UP      | UP      | FLAT    | 权益和债券都弱于黄金（避险） |",
        "| A4   | DOWN    | DOWN    | FLAT    | 权益和债券都强于黄金（风险偏好） |",
        "| A5   | FLAT    | FLAT    | FLAT    | 三者均衡 |",
        "| ...  | ...     | ...     | ...     | 其余配置待实际统计后分类 |",
        "",
        "### 实际配置分布",
        "",
    ])

    # 统计实际出现的配置
    config_counts: dict[tuple[str, str, str], int] = {}
    for dt in common_ts:
        dirs: dict[str, str] = {}
        all_found = True
        for edge in K3_EDGES:
            if edge not in k3_results:
                all_found = False
                break
            idx_map = {ts: i for i, ts in enumerate(k3_results[edge].timestamps)}
            if dt not in idx_map:
                all_found = False
                break
            dirs[edge] = k3_results[edge].directions[idx_map[dt]].name
        if all_found:
            key = (dirs["SPY_GLD"], dirs["TLT_GLD"], dirs["SPY_TLT"])
            config_counts[key] = config_counts.get(key, 0) + 1

    total_configs = sum(config_counts.values())
    sorted_configs = sorted(config_counts.items(), key=lambda x: -x[1])

    summary_lines.append("| SPY/GLD | TLT/GLD | SPY/TLT | 频次 | 占比 |")
    summary_lines.append("|---------|---------|---------|------|------|")
    for (sg, tg, st), count in sorted_configs:
        pct = round(count / total_configs * 100, 1) if total_configs > 0 else 0
        summary_lines.append(f"| {sg:7s} | {tg:7s} | {st:7s} | {count:4d} | {pct:5.1f}% |")

    summary_lines.extend([
        "",
        "## 结果包",
        "",
        "1. **结论**: K3 vs K4 方向态一致性/差异分析（见上）",
        "2. **定义依据**: OQ4 黄金截面假说——用黄金消除美元噪声",
        f"3. **边界条件**: GLD 数据起始日期 ({common_ts[0].date() if common_ts else 'N/A'})；"
        "  比价 Bar 构造法假设分母 OHLC 不为零",
        "4. **下游推论**: K3 截面是否可作为 K4 的有效简化——取决于共享边一致性和顶点边信息量",
        "5. **谱系引用**: 279号",
        "6. **影响声明**: 实验结果，不修改仓库代码",
        "",
        "## 认识论等级",
        "",
        "L2（真实数据验证）",
    ])

    summary_path = RESULTS_DIR / "summary.md"
    with open(summary_path, "w", encoding="utf-8") as f:
        f.write("\n".join(summary_lines))
    print(f"  分析摘要 -> {summary_path}")

    # ── 打印完成 ──
    print("\n" + "=" * 70)
    print("OQ4 黄金截面 K3 实验完成")
    print("=" * 70)
    print(f"  结果目录: {RESULTS_DIR}")
    print(f"  K3 耗时: {k3_time:.1f}s")
    print(f"  K4 耗时: {k4_time:.1f}s")
    print("\n完成。")


if __name__ == "__main__":
    main()
