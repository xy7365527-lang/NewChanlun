"""OQ2 v2 权益矩阵内部拓扑实验 — 行业ETF比价10条边。

284号谱系（OQ v2）：D 算子 + 背驰判断的联合阅读。

5 个行业 ETF（XLK/XLF/XLE/XLV/XLY）产生 10 条比价边。
每条边跑 RecursiveOrchestrator + 背驰判断（管线内部已整合）。

观察：
- 哪些边有走势结构（活跃边），哪些落入 ker(D)（沉寂边）
- 形态和背驰一致的边 vs 形态延续但已背驰的边
- 后者意味着板块间的相对流转即将反转
- 记录活跃拓扑的时变结构

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
from typing import Any

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "src"))

import pandas as pd
import yfinance as yf

from newchan.orchestrator.recursive import (
    RecursiveOrchestrator,
    RecursiveOrchestratorSnapshot,
)
from newchan.events import DomainEvent
from newchan.types import Bar

RESULTS_DIR = ROOT / "tmp" / "oq2-v2-results"

# ═══════════════════════════════════════════════════════════════
# 配置
# ═══════════════════════════════════════════════════════════════

ETF_SYMBOLS = ("XLK", "XLF", "XLE", "XLV", "XLY")

EDGES: list[tuple[str, str]] = []
for _i in range(len(ETF_SYMBOLS)):
    for _j in range(_i + 1, len(ETF_SYMBOLS)):
        EDGES.append((ETF_SYMBOLS[_i], ETF_SYMBOLS[_j]))


# ═══════════════════════════════════════════════════════════════
# 数据拉取与对齐
# ═══════════════════════════════════════════════════════════════


def fetch_data(symbols: tuple[str, ...]) -> dict[str, pd.DataFrame]:
    """拉取所有标的的日线 OHLCV。"""
    data: dict[str, pd.DataFrame] = {}
    for sym in symbols:
        print(f"  拉取 {sym} ...", end=" ", flush=True)
        df = yf.download(sym, period="max", interval="1d", progress=False)
        if isinstance(df.columns, pd.MultiIndex):
            df.columns = df.columns.get_level_values(0)
        if df.index.tz is not None:
            df.index = df.index.tz_localize(None)
        print(f"{len(df)} bars")
        data[sym] = df
    return data


def align_data(
    data: dict[str, pd.DataFrame],
) -> tuple[pd.DatetimeIndex, dict[str, pd.DataFrame]]:
    """对齐到公共交易日。"""
    common_index: pd.DatetimeIndex | None = None
    for df in data.values():
        idx = df.index
        common_index = idx if common_index is None else common_index.intersection(idx)
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
# 单条边运行 + 背驰/BSP 提取
# ═══════════════════════════════════════════════════════════════


# 结构事件类型
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

    # 最后一个走势的状态
    last_move_kind: str = ""
    last_move_direction: str = ""
    last_move_settled: bool = False

    # 背驰/买卖点事件计数
    bsp_events_count: int = 0

    # 活跃/沉寂判断
    is_active: bool = False  # 是否有已结算走势
    has_divergence_bsp: bool = False  # 最新走势是否伴随背驰/BSP

    # 时变方向态序列（每 bar 的走势状态摘要）
    direction_series: list[dict] = field(default_factory=list)


def run_edge(
    sym_a: str,
    sym_b: str,
    bars_a: list[Bar],
    bars_b: list[Bar],
) -> EdgeRunResult:
    """对一条比价边跑 RecursiveOrchestrator（D算子+背驰+BSP一体）。"""
    edge_label = f"{sym_a}/{sym_b}"
    orch = RecursiveOrchestrator(stream_id=f"{sym_a}_{sym_b}_ratio")

    direction_series: list[dict] = []
    last_snapshot: RecursiveOrchestratorSnapshot | None = None
    bsp_events_count = 0

    for i in range(len(bars_a)):
        ratio_bar = make_ratio_bar(bars_a[i], bars_b[i])
        snapshot = orch.process_bar(ratio_bar)
        last_snapshot = snapshot

        # 计数 BSP 事件
        for ev in snapshot.all_events:
            if ev.event_type in ("bsp_candidate", "bsp_confirm", "bsp_settle"):
                bsp_events_count += 1

        # 每 bar 记录方向态摘要
        moves = snapshot.move_snapshot.moves
        if moves:
            last_m = moves[-1]
            direction_series.append({
                "bar_idx": i,
                "ts": bars_a[i].ts.strftime("%Y-%m-%d"),
                "move_kind": last_m.kind,
                "move_direction": last_m.direction,
                "move_settled": last_m.settled,
                "move_count": len(moves),
            })
        else:
            direction_series.append({
                "bar_idx": i,
                "ts": bars_a[i].ts.strftime("%Y-%m-%d"),
                "move_kind": "none",
                "move_direction": "none",
                "move_settled": False,
                "move_count": 0,
            })

    if last_snapshot is None:
        raise ValueError(f"边 {edge_label} 无数据")

    bi_snap = last_snapshot.bi_snapshot
    seg_snap = last_snapshot.seg_snapshot
    zs_snap = last_snapshot.zs_snapshot
    move_snap = last_snapshot.move_snapshot
    bsp_snap = last_snapshot.bsp_snapshot

    moves = move_snap.moves
    settled_moves = [m for m in moves if m.settled]
    is_active = len(settled_moves) > 0

    last_move_kind = moves[-1].kind if moves else ""
    last_move_direction = moves[-1].direction if moves else ""
    last_move_settled = moves[-1].settled if moves else False

    # 最新走势是否有背驰/BSP（形态延续但已背驰 = 即将反转信号）
    has_divergence_bsp = len(bsp_snap.buysellpoints) > 0

    return EdgeRunResult(
        edge_label=edge_label,
        sym_a=sym_a,
        sym_b=sym_b,
        total_bars=len(bars_a),
        num_strokes=len(bi_snap.strokes),
        num_segments=len(seg_snap.segments),
        num_zhongshus=len(zs_snap.zhongshus),
        num_moves=len(moves),
        num_settled_moves=len(settled_moves),
        num_bsp=len(bsp_snap.buysellpoints),
        last_move_kind=last_move_kind,
        last_move_direction=last_move_direction,
        last_move_settled=last_move_settled,
        bsp_events_count=bsp_events_count,
        is_active=is_active,
        has_divergence_bsp=has_divergence_bsp,
        direction_series=direction_series,
    )


# ═══════════════════════════════════════════════════════════════
# 拓扑分析
# ═══════════════════════════════════════════════════════════════


def analyze_topology(results: list[EdgeRunResult]) -> dict[str, Any]:
    """分析10条边构成的拓扑。

    活跃边：有已结算走势的边（is_active=True）
    ker(D)：无已结算走势的边（is_active=False）
    """
    active_edges: list[str] = []
    ker_d_edges: list[str] = []
    diverging_edges: list[str] = []  # 形态延续但已背驰
    vertices: set[str] = set()
    adjacency: dict[str, list[str]] = defaultdict(list)

    for r in results:
        if r.is_active:
            active_edges.append(r.edge_label)
            vertices.add(r.sym_a)
            vertices.add(r.sym_b)
            adjacency[r.sym_a].append(r.sym_b)
            adjacency[r.sym_b].append(r.sym_a)
            if r.has_divergence_bsp and not r.last_move_settled:
                diverging_edges.append(r.edge_label)
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
        "diverging_edges": diverging_edges,
        "num_active": n_active,
        "num_ker_d": len(ker_d_edges),
        "num_diverging": len(diverging_edges),
        "vertices_in_active": sorted(vertices),
        "adjacency": {k: sorted(v) for k, v in adjacency.items()},
        "topology_type": topo_type,
        "vertex_degrees": {v: len(adjacency[v]) for v in sorted(vertices)},
    }


def compute_time_varying_topology(
    results: list[EdgeRunResult],
    sample_interval: int = 250,
) -> list[dict]:
    """按时间采样记录活跃拓扑的时变结构。"""
    if not results:
        return []

    total_bars = results[0].total_bars
    snapshots: list[dict] = []

    for bar_idx in range(0, total_bars, sample_interval):
        active = []
        ker_d = []
        diverging = []
        for r in results:
            if bar_idx >= len(r.direction_series):
                continue
            entry = r.direction_series[bar_idx]
            if entry["move_count"] > 0 and entry["move_kind"] != "none":
                active.append(r.edge_label)
            else:
                ker_d.append(r.edge_label)

        snapshots.append({
            "bar_idx": bar_idx,
            "ts": results[0].direction_series[bar_idx]["ts"] if bar_idx < len(results[0].direction_series) else "",
            "active_count": len(active),
            "ker_d_count": len(ker_d),
            "active_edges": active,
        })

    return snapshots


# ═══════════════════════════════════════════════════════════════
# 报告生成
# ═══════════════════════════════════════════════════════════════


def generate_report(
    results: list[EdgeRunResult],
    topology: dict,
    time_topo: list[dict],
    data_range: str,
) -> str:
    """生成 Markdown 报告。"""
    lines: list[str] = []
    lines.append("# OQ2 v2 权益矩阵内部拓扑分析报告")
    lines.append("")
    lines.append("认识论等级：L2（真实数据验证）")
    lines.append("谱系引用：284号（OQ v2）、283号、279号")
    lines.append("")

    lines.append("## 1. 284号变更")
    lines.append("")
    lines.append("- 背驰判断是缠论内部的统一判断，形态学和动力学不分离")
    lines.append("- D 算子 + 背驰判断一起跑在每条边上")
    lines.append("- 新增观察维度：形态和背驰一致 vs 形态延续但已背驰")
    lines.append("- 后者意味着板块间的相对流转即将反转")
    lines.append("")

    lines.append("## 2. 数据概览")
    lines.append("")
    lines.append(f"- ETF：{', '.join(ETF_SYMBOLS)}")
    lines.append(f"- 数据范围：{data_range}")
    lines.append(f"- 总 bar 数：{results[0].total_bars}")
    lines.append(f"- 比价边数：{len(results)}")
    lines.append("")

    lines.append("## 3. 各边走势产出")
    lines.append("")
    lines.append("| 边 | 笔 | 线段 | 中枢 | 走势 | settled | BSP | 末走势 | 方向 | 活跃 | 背驰BSP |")
    lines.append("|---|---|---|---|---|---|---|---|---|---|---|")
    for r in results:
        active_mark = "Y" if r.is_active else "N"
        div_mark = "Y" if r.has_divergence_bsp else "N"
        lines.append(
            f"| {r.edge_label} | {r.num_strokes} | {r.num_segments} | "
            f"{r.num_zhongshus} | {r.num_moves} | {r.num_settled_moves} | "
            f"{r.num_bsp} | {r.last_move_kind} | {r.last_move_direction} | "
            f"{active_mark} | {div_mark} |"
        )
    lines.append("")

    lines.append("## 4. 拓扑结构（终态）")
    lines.append("")
    lines.append(f"- 拓扑类型：{topology['topology_type']}")
    lines.append(f"- 活跃边：{topology['num_active']} 条")
    if topology['active_edges']:
        lines.append(f"  - {', '.join(topology['active_edges'])}")
    lines.append(f"- ker(D) 边：{topology['num_ker_d']} 条")
    if topology['ker_d_edges']:
        lines.append(f"  - {', '.join(topology['ker_d_edges'])}")
    lines.append(f"- 形态延续但已背驰的边：{topology['num_diverging']} 条")
    if topology['diverging_edges']:
        lines.append(f"  - {', '.join(topology['diverging_edges'])}")
        lines.append("  - 这些边的板块间相对流转可能即将反转")
    lines.append("")

    if topology['vertex_degrees']:
        lines.append("- 顶点度数：")
        for v, deg in sorted(topology['vertex_degrees'].items(), key=lambda x: -x[1]):
            lines.append(f"  - {v}: {deg}")
    lines.append("")

    lines.append("## 5. 边分类（284号核心观察）")
    lines.append("")
    lines.append("| 类别 | 边 | 含义 |")
    lines.append("|------|---|------|")
    for r in results:
        if r.is_active and not r.has_divergence_bsp:
            lines.append(f"| 形态+背驰一致 | {r.edge_label} | 走势延续中，无背驰信号 |")
    for r in results:
        if r.is_active and r.has_divergence_bsp:
            lines.append(f"| 形态延续已背驰 | {r.edge_label} | 走势可能即将反转 |")
    for r in results:
        if not r.is_active:
            lines.append(f"| ker(D) | {r.edge_label} | 无方向性走势 |")
    lines.append("")

    lines.append("## 6. 时变拓扑")
    lines.append("")
    if time_topo:
        lines.append("| 时间 | 活跃边数 | ker(D)边数 |")
        lines.append("|------|---------|----------|")
        for snap in time_topo:
            lines.append(f"| {snap['ts']} | {snap['active_count']} | {snap['ker_d_count']} |")
    lines.append("")

    lines.append("## 7. 结果包")
    lines.append("")
    lines.append("1. **结论**: 见上表——10条比价边的走势结构、背驰状态、活跃拓扑")
    lines.append("2. **定义依据**: 284号 OQ v2——D算子+背驰判断的联合阅读")
    lines.append("3. **边界条件**: 行业ETF选择不同则拓扑不同；级别由递归深度决定")
    lines.append("4. **下游推论**: 形态延续但已背驰的边 = 板块轮动反转信号（层1.5）")
    lines.append("5. **谱系引用**: 284号、283号、279号")
    lines.append("6. **影响声明**: 实验结果，不修改现有代码或定义")
    lines.append("")

    return "\n".join(lines)


# ═══════════════════════════════════════════════════════════════
# 主流程
# ═══════════════════════════════════════════════════════════════


def main() -> None:
    print("=" * 60)
    print("OQ2 v2 权益矩阵内部拓扑实验 — 行业ETF比价10条边")
    print("284号谱系：D 算子 + 背驰判断的联合阅读")
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

    # 4. 逐条边跑 RecursiveOrchestrator
    print("\n[4/6] 逐条边跑 RecursiveOrchestrator（D算子+背驰+BSP一体）")
    edge_results: list[EdgeRunResult] = []

    for edge_idx, (sym_a, sym_b) in enumerate(EDGES):
        edge_label = f"{sym_a}/{sym_b}"
        print(f"  [{edge_idx + 1}/10] {edge_label} ...", end=" ", flush=True)
        t0 = time.time()
        result = run_edge(sym_a, sym_b, bar_streams[sym_a], bar_streams[sym_b])
        elapsed = time.time() - t0
        active = "active" if result.is_active else "ker(D)"
        div = "+div" if result.has_divergence_bsp else ""
        print(
            f"{result.num_strokes}笔 {result.num_segments}段 "
            f"{result.num_zhongshus}中枢 {result.num_moves}走势 "
            f"{result.num_bsp}BSP | {active}{div} "
            f"({elapsed:.1f}s)"
        )
        edge_results.append(result)

    # 5. 分析
    print("\n[5/6] 拓扑分析")

    topology = analyze_topology(edge_results)
    print(f"  拓扑类型：{topology['topology_type']}")
    print(f"  活跃边：{topology['num_active']}, ker(D)边：{topology['num_ker_d']}")
    print(f"  形态延续已背驰：{topology['num_diverging']}")

    time_topo = compute_time_varying_topology(edge_results, sample_interval=250)

    # 6. 保存
    print("\n[6/6] 保存结果")
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

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
            "last_move_kind": r.last_move_kind,
            "last_move_direction": r.last_move_direction,
            "last_move_settled": r.last_move_settled,
            "bsp_events_count": r.bsp_events_count,
            "is_active": r.is_active,
            "has_divergence_bsp": r.has_divergence_bsp,
        })

    output = {
        "experiment": "OQ2 v2 权益矩阵内部拓扑",
        "pipeline": "RecursiveOrchestrator（D算子+背驰+BSP一体）",
        "epistemology_level": "L2",
        "genealogy_ref": ["284号（OQ v2）", "283号", "279号"],
        "timestamp": datetime.now().isoformat(),
        "data_range": data_range,
        "symbols": list(ETF_SYMBOLS),
        "edges": edge_structure,
        "topology": topology,
        "time_varying_topology": time_topo,
        "edge_classification": {
            "consistent": [r.edge_label for r in edge_results
                          if r.is_active and not r.has_divergence_bsp],
            "diverging": [r.edge_label for r in edge_results
                         if r.is_active and r.has_divergence_bsp],
            "ker_d": [r.edge_label for r in edge_results if not r.is_active],
        },
    }

    results_path = RESULTS_DIR / "oq2-v2-results.json"
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False, default=str)
    print(f"  结果 JSON: {results_path}")

    report = generate_report(edge_results, topology, time_topo, data_range)
    report_path = RESULTS_DIR / "oq2-v2-report.md"
    with open(report_path, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"  报告 MD: {report_path}")

    print("\n完成。")


if __name__ == "__main__":
    main()
