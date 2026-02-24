#!/usr/bin/env python3
"""拓扑前处理脚本——将 block-topology JSONL 转换为 networkx 图并计算拓扑指标。

179号-3 下游推论：分析家（topology-analyst）前处理步骤。

读取 relations.jsonl 构建有向图，计算：
- 节点入度/出度分布
- 弱连通分量数
- 最长路径（DAG 关键链）
- 承重节点（高 betweenness centrality）
- 孤立节点（无边连接）

通过 meta.json id_mapping 将 SHA256 id 翻译为可读的谱系编号。

用法:
  python scripts/topology_preprocess.py --base .chanlun/block-topology
  python scripts/topology_preprocess.py --base .chanlun/block-topology --output report.json
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def _check_networkx():
    """检查 networkx 是否可用。不可用时输出友好信息并退出。"""
    try:
        import networkx  # noqa: F401
        return True
    except ImportError:
        print(
            "错误: networkx 未安装。请执行: pip install networkx",
            file=sys.stderr,
        )
        return False


# ── 数据加载 ──


def load_relations(base: Path) -> list[dict]:
    """从 relations.jsonl 加载所有关系记录。

    Args:
        base: block-topology 根路径。

    Returns:
        关系记录列表。文件不存在时返回空列表。
    """
    jsonl_path = base / "relations.jsonl"
    if not jsonl_path.exists():
        return []
    relations = []
    for line in jsonl_path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped:
            relations.append(json.loads(stripped))
    return relations


def load_id_mapping(base: Path) -> dict[str, str]:
    """从 meta.json 加载 id_mapping（谱系编号 → SHA256）。

    Returns:
        反向映射 {SHA256: 谱系编号}。meta.json 不存在时返回空 dict。
    """
    meta_path = base / "meta.json"
    if not meta_path.exists():
        return {}
    meta = json.loads(meta_path.read_text(encoding="utf-8"))
    forward = meta.get("id_mapping", {})
    # 反转: SHA256 → 谱系编号
    return {sha: gid for gid, sha in forward.items()}


def translate_id(sha_id: str, reverse_mapping: dict[str, str]) -> str:
    """将 SHA256 id 翻译为可读的谱系编号。

    Args:
        sha_id: SHA256 区块 id。
        reverse_mapping: {SHA256: 谱系编号} 反向映射。

    Returns:
        谱系编号（如 "001"）或截断的 SHA（前12位）。
    """
    return reverse_mapping.get(sha_id, sha_id[:12])


# ── 图构建 ──


def build_graph(relations: list[dict], blocks_dir: Path):
    """从关系记录构建 networkx 有向图。

    每条关系记录生成一条有向边 from → to。
    blocks 目录中的所有区块作为节点加入（确保孤立节点也被包含）。

    Args:
        relations: 关系记录列表。
        blocks_dir: blocks 子目录路径。

    Returns:
        networkx.DiGraph。
    """
    import networkx as nx

    g = nx.DiGraph()

    # 添加 blocks 目录中的所有节点
    if blocks_dir.exists():
        for f in blocks_dir.iterdir():
            if f.suffix == ".json":
                block_id = f.stem
                g.add_node(block_id)

    # 添加边
    for rel in relations:
        g.add_edge(
            rel["from"],
            rel["to"],
            relation=rel.get("relation", "unknown"),
            order=rel.get("order", 1),
        )

    return g


# ── 指标计算 ──


def compute_graph_summary(g) -> dict:
    """计算图的基本统计摘要。

    Args:
        g: networkx.DiGraph。

    Returns:
        {nodes, edges, components, longest_path_length}
    """
    import networkx as nx

    n_nodes = g.number_of_nodes()
    n_edges = g.number_of_edges()

    if n_nodes == 0:
        return {
            "nodes": 0,
            "edges": 0,
            "components": 0,
            "longest_path_length": 0,
        }

    components = nx.number_weakly_connected_components(g)

    # 最长路径：在 DAG 上用 dag_longest_path
    # 如果图含环则降级为 0
    try:
        longest = nx.dag_longest_path(g)
        longest_len = max(0, len(longest) - 1)  # 边数 = 节点数 - 1
    except nx.NetworkXUnfeasible:
        longest_len = 0

    return {
        "nodes": n_nodes,
        "edges": n_edges,
        "components": components,
        "longest_path_length": longest_len,
    }


def find_load_bearing_nodes(g, top_n: int = 20) -> list[dict]:
    """找出承重节点（高 betweenness centrality）。

    Args:
        g: networkx.DiGraph。
        top_n: 返回前 N 个节点。

    Returns:
        按 betweenness 降序排列的节点列表。
    """
    import networkx as nx

    if g.number_of_nodes() == 0:
        return []

    betweenness = nx.betweenness_centrality(g)
    in_deg = dict(g.in_degree())
    out_deg = dict(g.out_degree())

    nodes = []
    for node_id, bc in betweenness.items():
        nodes.append({
            "id": node_id,
            "betweenness": bc,
            "in_degree": in_deg.get(node_id, 0),
            "out_degree": out_deg.get(node_id, 0),
        })

    nodes.sort(key=lambda x: x["betweenness"], reverse=True)
    return nodes[:top_n]


def find_isolated_nodes(g) -> list[dict]:
    """找出孤立节点（入度和出度均为 0）。

    Args:
        g: networkx.DiGraph。

    Returns:
        孤立节点列表。
    """
    isolated = []
    for node_id in g.nodes():
        if g.in_degree(node_id) == 0 and g.out_degree(node_id) == 0:
            isolated.append({"id": node_id})
    return isolated


def compute_relation_type_distribution(relations: list[dict]) -> dict[str, int]:
    """统计各关系类型的数量。

    Args:
        relations: 关系记录列表。

    Returns:
        {relation_type: count}。
    """
    dist: dict[str, int] = {}
    for rel in relations:
        rtype = rel.get("relation", "unknown")
        dist[rtype] = dist.get(rtype, 0) + 1
    return dist


# ── 报告生成 ──


def generate_report(base: Path) -> dict:
    """生成完整的拓扑分析报告。

    Args:
        base: block-topology 根路径。

    Returns:
        JSON 可序列化的报告 dict。
    """
    relations = load_relations(base)
    blocks_dir = base / "blocks"
    g = build_graph(relations, blocks_dir)
    reverse_mapping = load_id_mapping(base)

    summary = compute_graph_summary(g)
    load_bearing = find_load_bearing_nodes(g)
    isolated = find_isolated_nodes(g)
    distribution = compute_relation_type_distribution(relations)

    # 为承重节点和孤立节点添加可读 id
    for node in load_bearing:
        node["readable_id"] = translate_id(node["id"], reverse_mapping)

    for node in isolated:
        node["readable_id"] = translate_id(node["id"], reverse_mapping)

    return {
        "graph_summary": summary,
        "load_bearing_nodes": load_bearing,
        "isolated_nodes": isolated,
        "relation_type_distribution": distribution,
    }


# ── CLI ──


def main() -> int:
    if not _check_networkx():
        return 1

    parser = argparse.ArgumentParser(
        description="拓扑前处理——block-topology → networkx 图分析（179号-3）"
    )
    parser.add_argument(
        "--base",
        default=str(Path(".chanlun/block-topology")),
        help="block-topology 根路径（默认 .chanlun/block-topology）",
    )
    parser.add_argument(
        "--output",
        help="输出 JSON 报告路径（默认 stdout）",
    )
    args = parser.parse_args()

    base = Path(args.base)
    if not base.exists():
        print(f"错误: block-topology 目录不存在: {base}", file=sys.stderr)
        return 1

    report = generate_report(base)
    output = json.dumps(report, ensure_ascii=False, indent=2)

    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
        print(f"报告已写入: {args.output}")
    else:
        print(output)

    return 0


if __name__ == "__main__":
    sys.exit(main())
