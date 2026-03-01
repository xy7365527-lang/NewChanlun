"""OQ2 E矩阵内部拓扑——行业ETF比价10条边。

5个行业ETF（XLK, XLF, XLE, XLV, XLY）两两比价产生10条边。
每条边跑 RecursiveOrchestrator + D算子，探测E矩阵内部耦合结构。

认识论标注：L2（真实数据验证）。
谱系引用：279号。
"""

from __future__ import annotations

import json
import sys
import time
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any

# 项目根
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

import yfinance as yf
import pandas as pd

from newchan.orchestrator.recursive import (
    RecursiveOrchestrator,
    RecursiveOrchestratorSnapshot,
)
from newchan.topology.k4_scanner import walk_direction_from_snapshot
from newchan.topology.config_space import WalkDirection
from newchan.backtest.types import extract_d_reading, DOperatorReading
from newchan.types import Bar


# ═══════════════════════════════════════════════════════════════
# 配置
# ═══════════════════════════════════════════════════════════════

ETF_SYMBOLS = ("XLK", "XLF", "XLE", "XLV", "XLY")

# 10条边（两两组合）
EDGES: list[tuple[str, str]] = []
for i in range(len(ETF_SYMBOLS)):
    for j in range(i + 1, len(ETF_SYMBOLS)):
        EDGES.append((ETF_SYMBOLS[i], ETF_SYMBOLS[j]))

RESULTS_DIR = ROOT / "tmp" / "oq2-equity-topo-results"


# ═══════════════════════════════════════════════════════════════
# 数据拉取与对齐
# ═══════════════════════════════════════════════════════════════


def fetch_data(symbols: tuple[str, ...]) -> dict[str, pd.DataFrame]:
    """拉取所有标的的日线 OHLCV 数据（最大历史）。"""
    data: dict[str, pd.DataFrame] = {}
    for sym in symbols:
        print(f"  拉取 {sym} ...", end=" ", flush=True)
        df = yf.download(sym, period="max", interval="1d", progress=False)
        if isinstance(df.columns, pd.MultiIndex):
            df.columns = df.columns.get_level_values(0)
        print(f"{len(df)} bars")
        data[sym] = df
    return data


def align_data(
    data: dict[str, pd.DataFrame],
) -> tuple[pd.DatetimeIndex, dict[str, pd.DataFrame]]:
    """对齐所有标的到公共交易日。"""
    common_index: pd.DatetimeIndex | None = None
    for df in data.values():
        idx = df.index
        if common_index is None:
            common_index = idx
        else:
            common_index = common_index.intersection(idx)
    if common_index is None or len(common_index) == 0:
        raise ValueError("无公共交易日")
    common_index = common_index.sort_values()
    return common_index, {sym: df.loc[common_index] for sym, df in data.items()}


def df_to_bars(df: pd.DataFrame) -> list[Bar]:
    """DataFrame -> Bar 列表。"""
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


# ═══════════════════════════════════════════════════════════════
# 比价 Bar 构造
# ═══════════════════════════════════════════════════════════════


def make_ratio_bar(bar_a: Bar, bar_b: Bar) -> Bar:
    """构造比价 Bar。

    high = max(四种极端比值), low = min(四种极端比值)。
    """
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
# 单条边跑 D 算子
# ═══════════════════════════════════════════════════════════════


@dataclass
class EdgeRunResult:
    """单条边的跑完结果。"""

    edge_label: str
    sym_a: str
    sym_b: str
    total_bars: int
    num_strokes: int
    num_segments: int
    num_zhongshus: int
    num_moves: int
    num_settled_moves: int
    num_bsp: int
    final_direction: str
    final_amplitude: float
    final_absorption: bool
    # 时变方向态序列（每bar的方向态 + D读数）
    direction_series: list[dict]


def run_edge(
    sym_a: str,
    sym_b: str,
    bars_a: list[Bar],
    bars_b: list[Bar],
) -> EdgeRunResult:
    """对一条边跑 RecursiveOrchestrator。"""
    edge_label = f"{sym_a}/{sym_b}"
    orch = RecursiveOrchestrator(stream_id=f"{sym_a}_{sym_b}_ratio")

    direction_series: list[dict] = []
    last_snapshot: RecursiveOrchestratorSnapshot | None = None

    for i in range(len(bars_a)):
        ratio_bar = make_ratio_bar(bars_a[i], bars_b[i])
        snapshot = orch.process_bar(ratio_bar)
        last_snapshot = snapshot

        # 每 bar 记录方向态
        wd = walk_direction_from_snapshot(snapshot, level=0)
        d_reading = extract_d_reading(snapshot)
        direction_series.append({
            "bar_idx": i,
            "ts": bars_a[i].ts.strftime("%Y-%m-%d"),
            "direction": wd.name,
            "d_direction": d_reading.direction.value,
            "amplitude": round(d_reading.amplitude, 6),
            "absorption": d_reading.absorption,
        })

    # 从最终快照提取统计
    if last_snapshot is None:
        raise ValueError(f"边 {edge_label} 无数据")

    d_reading = extract_d_reading(last_snapshot)
    bi_snap = last_snapshot.bi_snapshot
    seg_snap = last_snapshot.seg_snapshot
    zs_snap = last_snapshot.zs_snapshot
    move_snap = last_snapshot.move_snapshot
    bsp_snap = last_snapshot.bsp_snapshot

    return EdgeRunResult(
        edge_label=edge_label,
        sym_a=sym_a,
        sym_b=sym_b,
        total_bars=len(bars_a),
        num_strokes=len(bi_snap.strokes),
        num_segments=len(seg_snap.segments),
        num_zhongshus=len(zs_snap.zhongshus),
        num_moves=len(move_snap.moves),
        num_settled_moves=len([m for m in move_snap.moves if m.settled]),
        num_bsp=len(bsp_snap.buysellpoints),
        final_direction=d_reading.direction.value,
        final_amplitude=d_reading.amplitude,
        final_absorption=d_reading.absorption,
        direction_series=direction_series,
    )


# ═══════════════════════════════════════════════════════════════
# 拓扑分析
# ═══════════════════════════════════════════════════════════════


def analyze_topology(results: list[EdgeRunResult]) -> dict[str, Any]:
    """分析10条边构成的拓扑。

    - 方向态边（UP/DOWN）= 活跃边
    - FLAT 边 = ker(D) 边
    - 活跃边构成的子图拓扑
    """
    active_edges: list[str] = []
    ker_d_edges: list[str] = []
    vertices: set[str] = set()
    adjacency: dict[str, list[str]] = defaultdict(list)

    for r in results:
        if r.final_direction != "flat":
            active_edges.append(r.edge_label)
            vertices.add(r.sym_a)
            vertices.add(r.sym_b)
            adjacency[r.sym_a].append(r.sym_b)
            adjacency[r.sym_b].append(r.sym_a)
        else:
            ker_d_edges.append(r.edge_label)

    # 拓扑分类
    n_active = len(active_edges)
    n_vertices = len(vertices)

    if n_active == 0:
        topo_type = "empty"
    elif n_active == 10:
        topo_type = "complete_K5"
    else:
        # 检查是否星形（一个顶点连所有其他顶点）
        max_degree = max(len(adj) for adj in adjacency.values()) if adjacency else 0
        if max_degree == n_vertices - 1 and n_active == n_vertices - 1:
            center = [v for v, adj in adjacency.items() if len(adj) == max_degree][0]
            topo_type = f"star(center={center})"
        # 检查是否链
        elif all(len(adj) <= 2 for adj in adjacency.values()):
            topo_type = "chain/path"
        else:
            topo_type = f"general({n_active}edges/{n_vertices}vertices)"

    return {
        "active_edges": active_edges,
        "ker_d_edges": ker_d_edges,
        "num_active": n_active,
        "num_ker_d": len(ker_d_edges),
        "vertices_in_active": sorted(vertices),
        "adjacency": {k: sorted(v) for k, v in adjacency.items()},
        "topology_type": topo_type,
        "vertex_degrees": {v: len(adjacency[v]) for v in sorted(vertices)},
    }


def analyze_temporal_topology(
    results: list[EdgeRunResult],
    window_size: int = 60,
) -> list[dict]:
    """时变拓扑分析：滑动窗口内活跃边变化。

    用最近 window_size 个 bar 的方向态判断边是否活跃。
    在窗口末端看"最终"方向态。
    """
    if not results or not results[0].direction_series:
        return []

    total_bars = len(results[0].direction_series)
    temporal_snapshots: list[dict] = []

    # 每 window_size 个 bar 采样一次
    for end_idx in range(window_size, total_bars, window_size):
        ts = results[0].direction_series[end_idx - 1]["ts"]
        active: list[str] = []
        ker_d: list[str] = []

        for r in results:
            direction = r.direction_series[end_idx - 1]["direction"]
            if direction != "FLAT":
                active.append(r.edge_label)
            else:
                ker_d.append(r.edge_label)

        temporal_snapshots.append({
            "window_end_ts": ts,
            "window_end_idx": end_idx - 1,
            "active_edges": active,
            "ker_d_edges": ker_d,
            "num_active": len(active),
            "num_ker_d": len(ker_d),
        })

    return temporal_snapshots


def compute_ker_d_boundary(results: list[EdgeRunResult]) -> dict[str, Any]:
    """计算每条边进入/离开 ker(D) 的边界时刻。

    ker(D) = FLAT 方向态。
    记录每条边方向态的变化点。
    """
    boundaries: dict[str, list[dict]] = {}

    for r in results:
        transitions: list[dict] = []
        prev_dir = None
        for entry in r.direction_series:
            curr_dir = entry["direction"]
            if prev_dir is not None and curr_dir != prev_dir:
                transitions.append({
                    "bar_idx": entry["bar_idx"],
                    "ts": entry["ts"],
                    "from": prev_dir,
                    "to": curr_dir,
                })
            prev_dir = curr_dir

        # 统计在 FLAT 中停留的时间占比
        flat_count = sum(1 for e in r.direction_series if e["direction"] == "FLAT")
        total = len(r.direction_series)

        boundaries[r.edge_label] = {
            "transitions": transitions,
            "num_transitions": len(transitions),
            "flat_ratio": round(flat_count / total, 4) if total > 0 else 0,
            "flat_bars": flat_count,
            "total_bars": total,
        }

    return boundaries


# ═══════════════════════════════════════════════════════════════
# 输出
# ═══════════════════════════════════════════════════════════════


def generate_summary(
    results: list[EdgeRunResult],
    topology: dict,
    temporal: list[dict],
    ker_d: dict,
) -> str:
    """生成人类可读摘要。"""
    lines: list[str] = []
    lines.append("# OQ2 E矩阵内部拓扑分析")
    lines.append("")
    lines.append(f"认识论标注：L2（真实数据验证）")
    lines.append(f"谱系引用：279号")
    lines.append("")

    lines.append("## 1. 数据概览")
    lines.append("")
    lines.append(f"- ETF：{', '.join(ETF_SYMBOLS)}")
    lines.append(f"- 总 bar 数：{results[0].total_bars}")
    lines.append(f"- 比价边数：{len(results)}")
    lines.append("")

    lines.append("## 2. 各边走势产出")
    lines.append("")
    lines.append("| 边 | 笔数 | 线段数 | 中枢数 | 走势数 | 已结走势 | 买卖点 | 终态方向 | 幅度 | 吸收态 |")
    lines.append("|---|------|--------|--------|--------|----------|--------|----------|------|--------|")
    for r in results:
        lines.append(
            f"| {r.edge_label} | {r.num_strokes} | {r.num_segments} | "
            f"{r.num_zhongshus} | {r.num_moves} | {r.num_settled_moves} | "
            f"{r.num_bsp} | {r.final_direction} | {r.final_amplitude:.4f} | "
            f"{'Y' if r.final_absorption else 'N'} |"
        )
    lines.append("")

    lines.append("## 3. 拓扑结构（终态）")
    lines.append("")
    lines.append(f"- 拓扑类型：{topology['topology_type']}")
    lines.append(f"- 活跃边（方向态 != FLAT）：{topology['num_active']} 条")
    if topology['active_edges']:
        lines.append(f"  - {', '.join(topology['active_edges'])}")
    lines.append(f"- ker(D) 边（FLAT）：{topology['num_ker_d']} 条")
    if topology['ker_d_edges']:
        lines.append(f"  - {', '.join(topology['ker_d_edges'])}")
    lines.append("")
    if topology['vertex_degrees']:
        lines.append("- 顶点度数：")
        for v, deg in sorted(topology['vertex_degrees'].items(), key=lambda x: -x[1]):
            lines.append(f"  - {v}: {deg}")
    lines.append("")

    lines.append("## 4. ker(D) 边界分析")
    lines.append("")
    lines.append("| 边 | FLAT占比 | 方向变化次数 | FLAT bar数 |")
    lines.append("|---|---------|-------------|-----------|")
    for edge_label in sorted(ker_d.keys()):
        info = ker_d[edge_label]
        lines.append(
            f"| {edge_label} | {info['flat_ratio']:.1%} | "
            f"{info['num_transitions']} | {info['flat_bars']} |"
        )
    lines.append("")

    lines.append("## 5. 时变拓扑")
    lines.append("")
    if temporal:
        lines.append(f"- 采样窗口：60 bar")
        lines.append(f"- 采样点数：{len(temporal)}")
        lines.append("")

        # 活跃边数量变化
        active_counts = [t["num_active"] for t in temporal]
        lines.append(f"- 活跃边数量范围：{min(active_counts)} ~ {max(active_counts)}")
        avg_active = sum(active_counts) / len(active_counts)
        lines.append(f"- 平均活跃边数：{avg_active:.1f}")
        lines.append("")

        # 最近5个时段的拓扑
        lines.append("最近5个采样点：")
        lines.append("")
        for t in temporal[-5:]:
            lines.append(f"- {t['window_end_ts']}: {t['num_active']} 活跃, {t['num_ker_d']} ker(D)")
            if t['active_edges']:
                lines.append(f"  活跃：{', '.join(t['active_edges'])}")
    else:
        lines.append("（数据不足，无时变分析）")
    lines.append("")

    lines.append("## 6. 下游推论")
    lines.append("")
    lines.append("- E矩阵内部拓扑结构反映行业间的相对强弱关系")
    lines.append("- 活跃边 = 行业间存在显著的方向性资本流动")
    lines.append("- ker(D) 边 = 行业间比价处于盘整/无方向态")
    lines.append("- 拓扑的时变性 = 行业轮动的结构性描述")
    lines.append("")

    return "\n".join(lines)


# ═══════════════════════════════════════════════════════════════
# 主流程
# ═══════════════════════════════════════════════════════════════


def main() -> None:
    print("=" * 60)
    print("OQ2 E矩阵内部拓扑 — 行业ETF比价10条边")
    print("=" * 60)

    # 1. 拉取数据
    print("\n[1/5] 拉取数据")
    raw_data = fetch_data(ETF_SYMBOLS)

    # 2. 对齐
    print("\n[2/5] 对齐交易日")
    common_dates, aligned = align_data(raw_data)
    print(f"  公共交易日：{len(common_dates)} bars")
    print(f"  起止：{common_dates[0].strftime('%Y-%m-%d')} -> {common_dates[-1].strftime('%Y-%m-%d')}")

    # 3. 转换 Bar
    print("\n[3/5] 转换 Bar 数据")
    bar_streams: dict[str, list[Bar]] = {}
    for sym in ETF_SYMBOLS:
        bar_streams[sym] = df_to_bars(aligned[sym])
        print(f"  {sym}: {len(bar_streams[sym])} bars")

    # 4. 逐条边跑 D 算子
    print("\n[4/5] 逐条边跑 D 算子")
    edge_results: list[EdgeRunResult] = []

    for edge_idx, (sym_a, sym_b) in enumerate(EDGES):
        edge_label = f"{sym_a}/{sym_b}"
        print(f"  [{edge_idx + 1}/10] {edge_label} ...", end=" ", flush=True)
        t0 = time.time()
        result = run_edge(sym_a, sym_b, bar_streams[sym_a], bar_streams[sym_b])
        elapsed = time.time() - t0
        print(
            f"{result.num_strokes}笔 {result.num_segments}段 "
            f"{result.num_zhongshus}中枢 {result.num_moves}走势 "
            f"dir={result.final_direction} ({elapsed:.1f}s)"
        )
        edge_results.append(result)

    # 5. 分析
    print("\n[5/5] 拓扑分析")

    topology = analyze_topology(edge_results)
    print(f"  拓扑类型：{topology['topology_type']}")
    print(f"  活跃边：{topology['num_active']}, ker(D)边：{topology['num_ker_d']}")

    temporal = analyze_temporal_topology(edge_results)
    print(f"  时变采样点：{len(temporal)}")

    ker_d = compute_ker_d_boundary(edge_results)

    # 输出
    print("\n[保存结果]")
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    # edge_structure.json
    edge_structure = []
    for r in edge_results:
        edge_structure.append({
            "edge_label": r.edge_label,
            "sym_a": r.sym_a,
            "sym_b": r.sym_b,
            "total_bars": r.total_bars,
            "num_strokes": r.num_strokes,
            "num_segments": r.num_segments,
            "num_zhongshus": r.num_zhongshus,
            "num_moves": r.num_moves,
            "num_settled_moves": r.num_settled_moves,
            "num_bsp": r.num_bsp,
            "final_direction": r.final_direction,
            "final_amplitude": r.final_amplitude,
            "final_absorption": r.final_absorption,
        })
    with open(RESULTS_DIR / "edge_structure.json", "w", encoding="utf-8") as f:
        json.dump(edge_structure, f, indent=2, ensure_ascii=False)
    print(f"  edge_structure.json")

    # topology_analysis.json
    with open(RESULTS_DIR / "topology_analysis.json", "w", encoding="utf-8") as f:
        json.dump(topology, f, indent=2, ensure_ascii=False)
    print(f"  topology_analysis.json")

    # ker_d_boundary.json
    with open(RESULTS_DIR / "ker_d_boundary.json", "w", encoding="utf-8") as f:
        json.dump(ker_d, f, indent=2, ensure_ascii=False)
    print(f"  ker_d_boundary.json")

    # temporal_topology.json
    with open(RESULTS_DIR / "temporal_topology.json", "w", encoding="utf-8") as f:
        json.dump(temporal, f, indent=2, ensure_ascii=False)
    print(f"  temporal_topology.json")

    # summary.md
    summary = generate_summary(edge_results, topology, temporal, ker_d)
    with open(RESULTS_DIR / "summary.md", "w", encoding="utf-8") as f:
        f.write(summary)
    print(f"  summary.md")

    # 方向态时间序列（大文件，单独存放）
    direction_data = {}
    for r in edge_results:
        direction_data[r.edge_label] = r.direction_series
    with open(RESULTS_DIR / "direction_series.json", "w", encoding="utf-8") as f:
        json.dump(direction_data, f, indent=2, ensure_ascii=False)
    print(f"  direction_series.json")

    print("\n完成。")


if __name__ == "__main__":
    main()
