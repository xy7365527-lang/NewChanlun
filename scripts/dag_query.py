#!/usr/bin/env python
"""查询 dag.yaml 的信息，避免 Lead 读取整个 55KB+ 文件。

145号修复：消除 Lead 对 dag.yaml 的完整读取需求。

dag.yaml 结构:
  nodes: [{id, title, status, type, file}, ...]
  edges:
    depends_on: [{from, to}, ...]       # 有向
    triggered:  [{from, to}, ...]       # 有向
    derived:    [{from, to}, ...]       # 有向
    negates:    [{from, to}, ...]       # 有向
    negated_by: [{target, by}, ...]     # 有向
    related:    [{between: [a, b]}, ...]  # 无向
    tensions_with: [{between: [a, b], valid_until?: ...}, ...]  # 无向

用法:
  python scripts/dag_query.py --exists 143
  python scripts/dag_query.py --node 143
  python scripts/dag_query.py --edges 143
  python scripts/dag_query.py --edge-exists depends_on 143 140
  python scripts/dag_query.py --stats
  python scripts/dag_query.py --recent 5
"""
import argparse
import json
import os
import sys

import yaml


DAG_PATH = ".chanlun/genealogy/dag.yaml"

DIRECTED_TYPES = {
    "depends_on": ("from", "to"),
    "triggered": ("from", "to"),
    "derived": ("from", "to"),
    "negates": ("from", "to"),
    "negated_by": ("by", "target"),
}
UNDIRECTED_TYPES = {"related", "tensions_with"}


def load_dag():
    if not os.path.isfile(DAG_PATH):
        print(f"Error: {DAG_PATH} not found", file=sys.stderr)
        sys.exit(1)
    with open(DAG_PATH, encoding="utf-8") as f:
        return yaml.safe_load(f)


def _build_node_index(dag):
    """将 nodes list 转为 {id: node} dict"""
    nodes = dag.get("nodes", [])
    return {str(n.get("id", "")): n for n in nodes if isinstance(n, dict)}


def cmd_exists(dag, node_id):
    nodes = _build_node_index(dag)
    print("true" if str(node_id) in nodes else "false")


def cmd_node(dag, node_id):
    nodes = _build_node_index(dag)
    nid = str(node_id)
    if nid not in nodes:
        print(json.dumps({"error": f"node {nid} not found"}, ensure_ascii=False))
        sys.exit(1)
    node = nodes[nid]
    result = {
        "id": nid,
        "title": node.get("title", ""),
        "type": node.get("type", ""),
        "status": node.get("status", ""),
        "file": node.get("file", ""),
    }
    print(json.dumps(result, ensure_ascii=False))


def _get_edges_for_node(dag, node_id):
    nid = str(node_id)
    edges_dict = dag.get("edges", {})
    result = {}

    for etype, edge_list in edges_dict.items():
        if not isinstance(edge_list, list):
            continue
        connected = []
        if etype in DIRECTED_TYPES:
            src_key, tgt_key = DIRECTED_TYPES[etype]
            for edge in edge_list:
                src = str(edge.get(src_key, ""))
                tgt = str(edge.get(tgt_key, ""))
                if src == nid:
                    connected.append(tgt)
                elif tgt == nid:
                    connected.append(src)
        elif etype in UNDIRECTED_TYPES:
            for edge in edge_list:
                between = [str(x) for x in edge.get("between", [])]
                if nid in between:
                    for other in between:
                        if other != nid:
                            connected.append(other)
        if connected:
            result[etype] = connected
    return result


def cmd_edges(dag, node_id):
    result = _get_edges_for_node(dag, node_id)
    print(json.dumps(result, ensure_ascii=False))


def cmd_edge_exists(dag, edge_type, source, target):
    edges_dict = dag.get("edges", {})
    src = str(source)
    tgt = str(target)

    edge_list = edges_dict.get(edge_type, [])
    if not isinstance(edge_list, list):
        print("false")
        return

    if edge_type in DIRECTED_TYPES:
        src_key, tgt_key = DIRECTED_TYPES[edge_type]
        for edge in edge_list:
            if str(edge.get(src_key, "")) == src and str(edge.get(tgt_key, "")) == tgt:
                print("true")
                return
    elif edge_type in UNDIRECTED_TYPES:
        for edge in edge_list:
            between = {str(x) for x in edge.get("between", [])}
            if {src, tgt} == between:
                print("true")
                return

    print("false")


def cmd_stats(dag):
    nodes = _build_node_index(dag)
    edges_dict = dag.get("edges", {})
    edge_counts = {}
    total_edges = 0
    for etype, edge_list in edges_dict.items():
        if isinstance(edge_list, list):
            edge_counts[etype] = len(edge_list)
            total_edges += len(edge_list)
    numeric_ids = [int(k) for k in nodes if k.isdigit()]
    result = {
        "nodes": len(nodes),
        "total_edges": total_edges,
        "edge_types": edge_counts,
        "max_id": max(numeric_ids) if numeric_ids else 0,
    }
    print(json.dumps(result, ensure_ascii=False))


def cmd_recent(dag, n):
    nodes = _build_node_index(dag)
    sorted_ids = sorted((int(k) for k in nodes if k.isdigit()), reverse=True)[:n]
    result = []
    for nid in sorted_ids:
        node = nodes[str(nid)]
        result.append({
            "id": str(nid),
            "title": node.get("title", ""),
            "type": node.get("type", ""),
            "status": node.get("status", ""),
        })
    print(json.dumps(result, ensure_ascii=False))


def main():
    parser = argparse.ArgumentParser(description="查询 dag.yaml")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--exists", metavar="NODE_ID", help="检查节点是否存在")
    group.add_argument("--node", metavar="NODE_ID", help="获取节点信息")
    group.add_argument("--edges", metavar="NODE_ID", help="获取节点的所有边")
    group.add_argument("--edge-exists", nargs=3, metavar=("TYPE", "SOURCE", "TARGET"),
                       help="检查特定边是否存在")
    group.add_argument("--stats", action="store_true", help="获取 DAG 统计")
    group.add_argument("--recent", type=int, metavar="N", help="获取最近 N 个节点")
    args = parser.parse_args()

    dag = load_dag()

    if args.exists:
        cmd_exists(dag, args.exists)
    elif args.node:
        cmd_node(dag, args.node)
    elif args.edges:
        cmd_edges(dag, args.edges)
    elif args.edge_exists:
        cmd_edge_exists(dag, *args.edge_exists)
    elif args.stats:
        cmd_stats(dag)
    elif args.recent is not None:
        cmd_recent(dag, args.recent)


if __name__ == "__main__":
    main()
