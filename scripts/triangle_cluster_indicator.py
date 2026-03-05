#!/usr/bin/env python3
"""三角簇密度指标——从 block-topology/relations.jsonl 提取三角簇并分析密度。

三角簇定义：三个节点 A, B, C 满足 A→B, B→C, A→C（有向三角形）。
三角簇密度 = 实际三角簇数 / 可能三角簇数。

来源：362号洞察2 + 365号拓扑分析。
- 362号-4：三角簇密度是"概念硬核心"的度量
- 365号-1：可纳入 ceremony_scan 健康度指标
- 365号-2：triangle-exclusive 节点是"被忽视的概念群"发现线索
- 365号-3：聚类系数可对接 Morse 临界点类型判据
- 365号-4：三角簇 + 入度双指标体系

认识论等级：L0（纯拓扑计算，从关系数据推导）

用法：
    python scripts/triangle_cluster_indicator.py
    python scripts/triangle_cluster_indicator.py --json          # JSON 输出
    python scripts/triangle_cluster_indicator.py --layer 1       # 仅逻辑层边
    python scripts/triangle_cluster_indicator.py --top 20        # top-N 节点
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_BASE = ROOT / ".chanlun" / "block-topology"

# 复用 block_topology.py 的层分类
LAYER_MAP: dict[str, int] = {
    "depends_on": 1, "negates": 1, "negated_by": 1, "supersedes": 1,
    "residue_of": 1, "reopens": 1, "tensions_with": 1,
    "freezes": 1, "splits": 1, "severs": 1,
    "references": 2, "defines": 2, "modifies": 2,
    "refines": 2, "revises": 2, "annotates": 2,
    "records": 3, "related": 3,
}


def load_relations(
    base: Path = DEFAULT_BASE,
    layer: int | None = None,
) -> list[dict]:
    """读取 relations.jsonl，可选按层过滤。"""
    jsonl_path = base / "relations.jsonl"
    if not jsonl_path.exists():
        return []
    relations = []
    for line in jsonl_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        rec = json.loads(line)
        # 兼容旧字段名
        if "source" in rec and "from" not in rec:
            rec["from"] = rec.pop("source")
        if "target" in rec and "to" not in rec:
            rec["to"] = rec.pop("target")
        if "type" in rec and "relation" not in rec:
            rec["relation"] = rec.pop("type")
        if layer is not None:
            rel_type = rec.get("relation", "")
            if LAYER_MAP.get(rel_type, 0) != layer:
                continue
        relations.append(rec)
    return relations


def build_adjacency(relations: list[dict]) -> dict[str, set[str]]:
    """构建有向邻接表 {from_node: {to_nodes}}。"""
    adj: dict[str, set[str]] = defaultdict(set)
    for rec in relations:
        src = rec.get("from", "")
        tgt = rec.get("to", "")
        if src and tgt and src != tgt:
            adj[src].add(tgt)
    return adj


def find_triangles(adj: dict[str, set[str]]) -> list[tuple[str, str, str]]:
    """查找所有有向三角簇 (A, B, C)：A→B, B→C, A→C。

    对每条边 A→B，检查 B 的所有邻居 C，若 A→C 也存在则形成三角簇。
    去重：只保留 A < B < C（字典序）的三元组。
    """
    triangles: list[tuple[str, str, str]] = []
    for a, a_neighbors in adj.items():
        for b in a_neighbors:
            if b <= a:
                continue
            b_neighbors = adj.get(b, set())
            # A→B 存在，检查 B→C 且 A→C
            common = a_neighbors & b_neighbors
            for c in common:
                if c <= b:
                    continue
                triangles.append((a, b, c))
    return triangles


def compute_node_triangle_count(
    triangles: list[tuple[str, str, str]],
) -> dict[str, int]:
    """每个节点参与的三角簇数量。"""
    counts: dict[str, int] = defaultdict(int)
    for a, b, c in triangles:
        counts[a] += 1
        counts[b] += 1
        counts[c] += 1
    return dict(counts)


def compute_clustering_coefficients(
    adj: dict[str, set[str]],
    node_triangle_count: dict[str, int],
) -> dict[str, float]:
    """计算每个节点的局部聚类系数。

    对有向图，节点 v 的聚类系数 = triangles(v) / (deg(v) * (deg(v)-1) / 2)
    其中 deg(v) 是 v 的（出）邻居数。
    """
    coefficients: dict[str, float] = {}
    for node in adj:
        deg = len(adj[node])
        if deg < 2:
            coefficients[node] = 0.0
            continue
        possible = deg * (deg - 1) // 2
        actual = node_triangle_count.get(node, 0)
        coefficients[node] = actual / possible if possible > 0 else 0.0
    return coefficients


def find_triangle_exclusive_nodes(
    triangles: list[tuple[str, str, str]],
    adj: dict[str, set[str]],
    in_degree: dict[str, int],
) -> list[str]:
    """找出仅在三角簇中出现、入度为0的节点——"被忽视的概念群"(365号-2)。

    这些节点只因为参与三角关系才可见，不被其他节点直接引用。
    """
    triangle_nodes = set()
    for a, b, c in triangles:
        triangle_nodes.update([a, b, c])
    return sorted(n for n in triangle_nodes if in_degree.get(n, 0) == 0)


def compute_in_degree(relations: list[dict]) -> dict[str, int]:
    """计算每个节点的入度。"""
    in_deg: dict[str, int] = defaultdict(int)
    for rec in relations:
        tgt = rec.get("to", "")
        if tgt:
            in_deg[tgt] += 1
    return dict(in_deg)


def get_block_label(block_id: str, base: Path = DEFAULT_BASE) -> str:
    """尝试获取区块的人类可读标签。"""
    block_path = base / "blocks" / f"{block_id}.json"
    if not block_path.exists():
        return block_id[:12] + "..."
    try:
        blk = json.loads(block_path.read_text(encoding="utf-8"))
        content = blk.get("content", {})
        num = content.get("genealogy_number") or content.get("number") or content.get("id")
        title = content.get("title", "")
        if num is not None:
            label = f"#{num}"
            if title:
                label += f" {title[:40]}"
            return label
        if title:
            return title[:50]
    except (json.JSONDecodeError, OSError):
        pass
    return block_id[:12] + "..."


def analyze(
    base: Path = DEFAULT_BASE,
    layer: int | None = None,
    top_n: int = 15,
) -> dict:
    """执行完整三角簇分析，返回结构化结果。"""
    relations = load_relations(base, layer=layer)
    if not relations:
        return {"error": "No relations found", "triangles": [], "density": 0.0}

    adj = build_adjacency(relations)
    in_degree = compute_in_degree(relations)
    all_nodes = set(adj.keys())
    for rec in relations:
        tgt = rec.get("to", "")
        if tgt:
            all_nodes.add(tgt)

    triangles = find_triangles(adj)
    node_tri_count = compute_node_triangle_count(triangles)
    clustering = compute_clustering_coefficients(adj, node_tri_count)
    triangle_exclusive = find_triangle_exclusive_nodes(triangles, adj, in_degree)

    # 全局密度：实际三角簇 / 可能三角簇 (n*(n-1)*(n-2)/6)
    n = len(all_nodes)
    possible_triangles = n * (n - 1) * (n - 2) // 6 if n >= 3 else 0
    global_density = len(triangles) / possible_triangles if possible_triangles > 0 else 0.0

    # 按三角簇参与度排序的 top-N 节点
    top_nodes = sorted(
        node_tri_count.items(), key=lambda x: x[1], reverse=True
    )[:top_n]

    # 按聚类系数排序的 top-N 节点（仅有 >=2 条出边的）
    top_clustering = sorted(
        ((n, c) for n, c in clustering.items() if c > 0),
        key=lambda x: x[1],
        reverse=True,
    )[:top_n]

    # 双指标：三角簇数 + 入度 (365号-4)
    dual_indicator = []
    for node_id, tri_count in top_nodes:
        dual_indicator.append({
            "node": node_id,
            "triangle_count": tri_count,
            "in_degree": in_degree.get(node_id, 0),
            "clustering_coefficient": round(clustering.get(node_id, 0.0), 4),
        })

    return {
        "total_nodes": len(all_nodes),
        "total_edges": len(relations),
        "triangle_count": len(triangles),
        "possible_triangles": possible_triangles,
        "global_density": global_density,
        "avg_clustering_coefficient": (
            sum(clustering.values()) / len(clustering) if clustering else 0.0
        ),
        "top_triangle_nodes": [
            {"node": nid, "count": cnt, "label": get_block_label(nid, base)}
            for nid, cnt in top_nodes
        ],
        "top_clustering_nodes": [
            {"node": nid, "coefficient": round(coeff, 4),
             "label": get_block_label(nid, base)}
            for nid, coeff in top_clustering
        ],
        "triangle_exclusive_nodes": [
            {"node": nid, "label": get_block_label(nid, base)}
            for nid in triangle_exclusive[:top_n]
        ],
        "dual_indicator": dual_indicator,
        "layer_filter": layer,
    }


def print_report(result: dict) -> None:
    """输出人类可读报告。"""
    print("=" * 70)
    print("  三角簇密度分析报告")
    if result.get("layer_filter") is not None:
        print(f"  (仅 Layer {result['layer_filter']} 边)")
    print("=" * 70)

    if "error" in result:
        print(f"\n  {result['error']}")
        return

    print(f"\n  节点: {result['total_nodes']}  边: {result['total_edges']}")
    print(f"  三角簇: {result['triangle_count']}  "
          f"(可能: {result['possible_triangles']})")
    print(f"  全局密度: {result['global_density']:.6e}")
    print(f"  平均聚类系数: {result['avg_clustering_coefficient']:.4f}")

    if result["top_triangle_nodes"]:
        print(f"\n  概念硬核心（三角簇参与 Top-{len(result['top_triangle_nodes'])}）:")
        for item in result["top_triangle_nodes"]:
            print(f"    {item['count']:>5}  {item['label']}")

    if result["top_clustering_nodes"]:
        print(f"\n  高聚类系数节点（Top-{len(result['top_clustering_nodes'])}）:")
        for item in result["top_clustering_nodes"]:
            print(f"    {item['coefficient']:.4f}  {item['label']}")

    if result["triangle_exclusive_nodes"]:
        print(f"\n  被忽视的概念群（仅三角簇可见，入度=0）:")
        for item in result["triangle_exclusive_nodes"]:
            print(f"    {item['label']}")
    else:
        print("\n  无 triangle-exclusive 节点")

    if result["dual_indicator"]:
        print(f"\n  双指标体系（三角簇数 + 入度）:")
        print(f"    {'三角簇':>6}  {'入度':>4}  {'聚类系数':>8}  节点")
        for item in result["dual_indicator"]:
            print(f"    {item['triangle_count']:>6}  "
                  f"{item['in_degree']:>4}  "
                  f"{item['clustering_coefficient']:>8.4f}  "
                  f"{get_block_label(item['node'])}")

    print(f"\n{'=' * 70}")


def health_summary(base: Path = DEFAULT_BASE) -> dict:
    """精简的健康度指标，可嵌入 ceremony_scan。

    返回：
      triangle_count: 三角簇总数
      global_density: 全局三角簇密度
      avg_clustering: 平均聚类系数
      top3_hubs: 三角簇参与度最高的3个节点
    """
    result = analyze(base, top_n=3)
    if "error" in result:
        return {"triangle_count": 0, "global_density": 0.0,
                "avg_clustering": 0.0, "top3_hubs": []}
    return {
        "triangle_count": result["triangle_count"],
        "global_density": result["global_density"],
        "avg_clustering": result["avg_clustering_coefficient"],
        "top3_hubs": [
            {"label": item["label"], "count": item["count"]}
            for item in result["top_triangle_nodes"][:3]
        ],
    }


def main():
    parser = argparse.ArgumentParser(description="三角簇密度分析")
    parser.add_argument("--json", action="store_true", help="JSON 输出")
    parser.add_argument("--layer", type=int, choices=[1, 2, 3],
                        help="仅分析指定层的边 (1=逻辑, 2=导航, 3=元数据)")
    parser.add_argument("--top", type=int, default=15, help="Top-N 节点数 (默认 15)")
    parser.add_argument("--base", type=str, default=None,
                        help="block-topology 基础路径")
    parser.add_argument("--health", action="store_true",
                        help="精简健康度输出 (可嵌入 ceremony_scan)")
    args = parser.parse_args()

    base = Path(args.base) if args.base else DEFAULT_BASE

    if args.health:
        summary = health_summary(base)
        print(json.dumps(summary, ensure_ascii=False, indent=2))
        return

    result = analyze(base, layer=args.layer, top_n=args.top)

    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        print_report(result)


if __name__ == "__main__":
    main()
