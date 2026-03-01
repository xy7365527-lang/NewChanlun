"""OQ2 E矩阵内部拓扑实验 — 行业ETF比价10条边。

5个行业ETF（XLK, XLF, XLE, XLV, XLY）两两比价产生10条边。
每条边跑 RecursiveOrchestrator（口径A：从日线递归）+ D算子读数，
探测E矩阵内部耦合结构，并与K4完全图六条边的拓扑比较。

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
ROOT = Path(__file__).resolve().parent.parent.parent
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
for _i in range(len(ETF_SYMBOLS)):
    for _j in range(_i + 1, len(ETF_SYMBOLS)):
        EDGES.append((ETF_SYMBOLS[_i], ETF_SYMBOLS[_j]))

RESULTS_JSON = ROOT / "tmp" / "oq2-equity-topology-results.json"
REPORT_MD = ROOT / "tmp" / "oq2-equity-topology-report.md"


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
    """单条边的运行结果。"""

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
    """对一条边跑 RecursiveOrchestrator（口径A）。"""
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

    - 方向态边（up/down）= 活跃边
    - flat 边 = ker(D) 边
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

    n_active = len(active_edges)
    n_vertices = len(vertices)

    if n_active == 0:
        topo_type = "empty"
    elif n_active == 10:
        topo_type = "complete_K5"
    else:
        max_degree = max(len(adj) for adj in adjacency.values()) if adjacency else 0
        if max_degree == n_vertices - 1 and n_active == n_vertices - 1:
            center = [v for v, adj in adjacency.items() if len(adj) == max_degree][0]
            topo_type = f"star(center={center})"
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


def compute_ker_d_boundary(results: list[EdgeRunResult]) -> dict[str, Any]:
    """计算每条边进入/离开 ker(D) 的边界时刻。"""
    boundaries: dict[str, dict] = {}

    for r in results:
        transitions: list[dict] = []
        prev_dir = None
        for entry in r.direction_series:
            curr_dir = entry["d_direction"]
            if prev_dir is not None and curr_dir != prev_dir:
                transitions.append({
                    "bar_idx": entry["bar_idx"],
                    "ts": entry["ts"],
                    "from": prev_dir,
                    "to": curr_dir,
                })
            prev_dir = curr_dir

        flat_count = sum(1 for e in r.direction_series if e["d_direction"] == "flat")
        total = len(r.direction_series)

        boundaries[r.edge_label] = {
            "num_transitions": len(transitions),
            "flat_ratio": round(flat_count / total, 4) if total > 0 else 0,
            "flat_bars": flat_count,
            "total_bars": total,
        }

    return boundaries


# ═══════════════════════════════════════════════════════════════
# K4 拓扑比较
# ═══════════════════════════════════════════════════════════════


def compare_with_k4(topology: dict[str, Any]) -> dict[str, Any]:
    """E矩阵内部拓扑与K4拓扑的比较。

    K4完全图：4顶点（E/Au/R/$），6条边。
    E矩阵内部：5顶点（XLK/XLF/XLE/XLV/XLY），10条边。

    比较维度：
    1. 顶点数/边数
    2. 活跃边比例
    3. 拓扑类型是否同构
    """
    e_vertices = 5
    e_total_edges = 10
    e_active = topology["num_active"]
    e_topo = topology["topology_type"]

    k4_vertices = 4
    k4_total_edges = 6

    is_complete_e = (e_active == e_total_edges)

    comparison = {
        "e_matrix": {
            "vertices": e_vertices,
            "total_edges": e_total_edges,
            "active_edges": e_active,
            "active_ratio": round(e_active / e_total_edges, 4),
            "topology_type": e_topo,
            "is_complete": is_complete_e,
        },
        "k4": {
            "vertices": k4_vertices,
            "total_edges": k4_total_edges,
            "description": "K4完全图（E/Au/R/$四顶点）",
        },
        "structural_comparison": {
            "same_graph_type": False,
            "reason": "",
        },
    }

    # E矩阵是K5的子图（5顶点完全图有10条边）
    # K4是4顶点完全图（6条边）
    # 顶点数不同 → 不可能图同构
    comparison["structural_comparison"]["same_graph_type"] = False
    comparison["structural_comparison"]["reason"] = (
        f"E矩阵是5顶点图（最多K5=10边），K4是4顶点图（6边）。"
        f"顶点数不同，不可能图同构。"
        f"E矩阵当前活跃边{e_active}条（{e_topo}），"
        f"与K4的6条边在结构上不可比。"
        f"但两者的D算子读数语义一致：方向态=资本流向。"
    )

    # 活跃比例比较
    if is_complete_e:
        comparison["structural_comparison"]["active_comparison"] = (
            "E矩阵所有10条边均为活跃态 → "
            "行业间比价全部有方向性走势，无ker(D)边"
        )
    else:
        comparison["structural_comparison"]["active_comparison"] = (
            f"E矩阵{e_active}/10条边活跃 → "
            f"部分行业间比价处于盘整/无方向态"
        )

    return comparison


# ═══════════════════════════════════════════════════════════════
# 报告生成
# ═══════════════════════════════════════════════════════════════


def generate_report(
    results: list[EdgeRunResult],
    topology: dict,
    ker_d: dict,
    k4_comparison: dict,
    data_range: str,
) -> str:
    """生成 Markdown 报告。"""
    lines: list[str] = []
    lines.append("# OQ2 E矩阵内部拓扑分析报告")
    lines.append("")
    lines.append("认识论标注：L2（真实数据验证）")
    lines.append("谱系引用：279号")
    lines.append("")

    lines.append("## 1. 数据概览")
    lines.append("")
    lines.append(f"- ETF：{', '.join(ETF_SYMBOLS)}")
    lines.append(f"- 数据范围：{data_range}")
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
    lines.append(f"- 活跃边（方向态 != flat）：{topology['num_active']} 条")
    if topology['active_edges']:
        lines.append(f"  - {', '.join(topology['active_edges'])}")
    lines.append(f"- ker(D) 边（flat）：{topology['num_ker_d']} 条")
    if topology['ker_d_edges']:
        lines.append(f"  - {', '.join(topology['ker_d_edges'])}")
    lines.append("")
    if topology['vertex_degrees']:
        lines.append("- 顶点度数：")
        for v, deg in sorted(topology['vertex_degrees'].items(), key=lambda x: -x[1]):
            lines.append(f"  - {v}: {deg}")
    lines.append("")

    lines.append("## 4. 方向态分类")
    lines.append("")
    up_edges = [r.edge_label for r in results if r.final_direction == "up"]
    down_edges = [r.edge_label for r in results if r.final_direction == "down"]
    flat_edges = [r.edge_label for r in results if r.final_direction == "flat"]
    lines.append(f"- 上涨方向态（{len(up_edges)}条）：{', '.join(up_edges) if up_edges else '无'}")
    lines.append(f"- 下跌方向态（{len(down_edges)}条）：{', '.join(down_edges) if down_edges else '无'}")
    lines.append(f"- ker(D)/flat（{len(flat_edges)}条）：{', '.join(flat_edges) if flat_edges else '无'}")
    lines.append("")

    lines.append("## 5. ker(D) 边界分析")
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

    lines.append("## 6. 与 K4 拓扑比较")
    lines.append("")
    ec = k4_comparison["e_matrix"]
    lines.append(f"- E矩阵：{ec['vertices']}顶点 {ec['total_edges']}条边，"
                 f"活跃{ec['active_edges']}条（{ec['active_ratio']:.0%}），"
                 f"拓扑={ec['topology_type']}")
    k4c = k4_comparison["k4"]
    lines.append(f"- K4：{k4c['vertices']}顶点 {k4c['total_edges']}条边（{k4c['description']}）")
    lines.append("")
    sc = k4_comparison["structural_comparison"]
    lines.append(f"- 同构判断：{'是' if sc['same_graph_type'] else '否'}")
    lines.append(f"- 理由：{sc['reason']}")
    if "active_comparison" in sc:
        lines.append(f"- 活跃度比较：{sc['active_comparison']}")
    lines.append("")

    lines.append("## 7. 下游推论")
    lines.append("")
    lines.append("- E矩阵内部拓扑结构反映行业间的相对强弱关系")
    lines.append("- 活跃边 = 行业间存在显著的方向性资本流动")
    lines.append("- ker(D) 边 = 行业间比价处于盘整/无方向态")
    lines.append("- E矩阵（5顶点K5子图）与K4（4顶点完全图）不可图同构，但D算子语义一致")
    lines.append("- 两者的对比意义在于：E矩阵是K4中E顶点的内部展开，展示了'权益类资产'内部的行业分化结构")
    lines.append("")

    lines.append("## 8. 边界条件")
    lines.append("")
    lines.append("- 以上结论基于行业ETF日线数据，级别由递归深度决定")
    lines.append("- 如果行业ETF选择不同，拓扑结构会不同")
    lines.append("- 时变拓扑显示行业轮动的结构性变化，但本报告仅展示终态")
    lines.append("")

    return "\n".join(lines)


# ═══════════════════════════════════════════════════════════════
# 主流程
# ═══════════════════════════════════════════════════════════════


def main() -> None:
    print("=" * 60)
    print("OQ2 E矩阵内部拓扑实验 — 行业ETF比价10条边")
    print("口径A：RecursiveOrchestrator 从日线递归")
    print("=" * 60)

    # 1. 拉取数据
    print("\n[1/6] 拉取数据")
    raw_data = fetch_data(ETF_SYMBOLS)

    # 2. 对齐
    print("\n[2/6] 对齐交易日")
    common_dates, aligned = align_data(raw_data)
    data_range = (
        f"{common_dates[0].strftime('%Y-%m-%d')} ~ "
        f"{common_dates[-1].strftime('%Y-%m-%d')}"
    )
    print(f"  公共交易日：{len(common_dates)} bars")
    print(f"  范围：{data_range}")

    # 3. 转换 Bar
    print("\n[3/6] 转换 Bar 数据")
    bar_streams: dict[str, list[Bar]] = {}
    for sym in ETF_SYMBOLS:
        bar_streams[sym] = df_to_bars(aligned[sym])
        print(f"  {sym}: {len(bar_streams[sym])} bars")

    # 4. 逐条边跑 D 算子
    print("\n[4/6] 逐条边跑 RecursiveOrchestrator + D 算子")
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
            f"dir={result.final_direction} amp={result.final_amplitude:.4f} "
            f"({elapsed:.1f}s)"
        )
        edge_results.append(result)

    # 5. 分析
    print("\n[5/6] 拓扑分析")

    topology = analyze_topology(edge_results)
    print(f"  拓扑类型：{topology['topology_type']}")
    print(f"  活跃边：{topology['num_active']}, ker(D)边：{topology['num_ker_d']}")

    ker_d = compute_ker_d_boundary(edge_results)

    k4_comparison = compare_with_k4(topology)
    print(f"  K4同构：{k4_comparison['structural_comparison']['same_graph_type']}")

    # 6. 保存
    print("\n[6/6] 保存结果")

    # 构造结果 JSON（不含庞大的 direction_series）
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

    output = {
        "experiment": "OQ2 E矩阵内部拓扑",
        "pipeline": "口径A（RecursiveOrchestrator 从日线递归）",
        "epistemology_level": "L2",
        "genealogy_ref": "279号",
        "timestamp": datetime.now().isoformat(),
        "data_range": data_range,
        "symbols": list(ETF_SYMBOLS),
        "edges": edge_structure,
        "topology": topology,
        "ker_d_boundary": ker_d,
        "k4_comparison": k4_comparison,
        "direction_classification": {
            "up": [r.edge_label for r in edge_results if r.final_direction == "up"],
            "down": [r.edge_label for r in edge_results if r.final_direction == "down"],
            "flat": [r.edge_label for r in edge_results if r.final_direction == "flat"],
        },
    }

    RESULTS_JSON.parent.mkdir(parents=True, exist_ok=True)
    with open(RESULTS_JSON, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False, default=str)
    print(f"  结果 JSON: {RESULTS_JSON}")

    # 报告
    report = generate_report(edge_results, topology, ker_d, k4_comparison, data_range)
    with open(REPORT_MD, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"  报告 MD: {REPORT_MD}")

    print("\n完成。")


if __name__ == "__main__":
    main()
