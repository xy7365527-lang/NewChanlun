"""traverse.py — 穿越基础设施查询接口（组件一）

三个查询函数：
- query_block: 区块完整穿越视图
- query_pair: 两区块间 BFS 最短路径（references 无向图）
- query_concept: 概念注册表查询

无状态，依赖 block_topology / morse_landscape / concept_registry。
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import deque
from pathlib import Path
from typing import Any

# scripts 目录内互相 import
sys.path.insert(0, str(Path(__file__).parent))

from block_topology import (
    DEFAULT_BASE,
    read_all_relations,
    read_block,
    get_block_mapping,
)
from concept_registry import ConceptRegistry, load_registry
from morse_landscape import MorseLandscape, build_morse_landscape


def query_block(
    block_id: str,
    morse: MorseLandscape,
    registry: ConceptRegistry,
    base: Path = DEFAULT_BASE,
) -> dict | None:
    """返回区块的完整穿越视图。区块不存在时返回 None。"""
    blk = read_block(block_id, base)
    if blk is None:
        return None

    content = blk.get("content", {})
    title = content.get("title", "")
    block_type = blk.get("type", "")
    status = content.get("status", "")

    # genealogy_number
    id2num, _ = get_block_mapping(base)
    genealogy_number = id2num.get(block_id)

    # 边收集
    all_relations = read_all_relations(base)
    out_edges: dict[str, list[dict]] = {}
    in_edges: dict[str, list[dict]] = {}

    for rel in all_relations:
        rtype = rel.get("relation", "")
        if rel["from"] == block_id:
            edge_entry: dict[str, Any] = {"to": rel["to"]}
            if rtype == "references":
                edge_key = f"{rel['from']}:{rel['to']}"
                mark = morse.edge_marks.get(edge_key)
                if mark is not None:
                    edge_entry["mark"] = mark
            out_edges.setdefault(rtype, []).append(edge_entry)

        if rel["to"] == block_id:
            edge_entry = {"from": rel["from"]}
            if rtype == "references":
                edge_key = f"{rel['from']}:{rel['to']}"
                mark = morse.edge_marks.get(edge_key)
                if mark is not None:
                    edge_entry["mark"] = mark
            in_edges.setdefault(rtype, []).append(edge_entry)

    # concepts
    concepts: list[dict] = []
    for cid, entry in registry.entries.items():
        if block_id in entry.defining_blocks:
            concepts.append({
                "concept_id": cid,
                "term": entry.term,
                "authoritative": entry.authoritative,
            })

    # morse_summary
    critical_out = 0
    critical_in = 0
    for edge_list in out_edges.get("references", []):
        if edge_list.get("mark") == "critical":
            critical_out += 1
    for edge_list in in_edges.get("references", []):
        if edge_list.get("mark") == "critical":
            critical_in += 1

    return {
        "block_id": block_id,
        "genealogy_number": genealogy_number,
        "title": title,
        "type": block_type,
        "status": status,
        "out_edges": out_edges,
        "in_edges": in_edges,
        "concepts": concepts,
        "morse_summary": {
            "critical_out": critical_out,
            "critical_in": critical_in,
        },
    }


def query_pair(
    src: str,
    dst: str,
    base: Path = DEFAULT_BASE,
) -> dict | None:
    """BFS 最短路径（references 无向图）。不可达返回 None。"""
    if src == dst:
        return {"nodes": [src], "edges": [], "marks": [], "length": 0}

    # 构建 references 无向邻接表
    all_relations = read_all_relations(base)
    # adjacency: node -> list of (neighbor, edge_key)
    adj: dict[str, list[tuple[str, str]]] = {}
    nodes_in_graph: set[str] = set()

    for rel in all_relations:
        if rel.get("relation") != "references":
            continue
        from_id = rel["from"]
        to_id = rel["to"]
        edge_key = f"{from_id}:{to_id}"
        nodes_in_graph.add(from_id)
        nodes_in_graph.add(to_id)
        # 无向：双方向都可走
        adj.setdefault(from_id, []).append((to_id, edge_key))
        adj.setdefault(to_id, []).append((from_id, edge_key))

    if src not in nodes_in_graph or dst not in nodes_in_graph:
        return None

    # BFS
    visited: set[str] = {src}
    # queue entries: (current_node, path_nodes, path_edges)
    queue: deque[tuple[str, list[str], list[str]]] = deque()
    queue.append((src, [src], []))

    while queue:
        current, path_nodes, path_edges = queue.popleft()
        for neighbor, edge_key in adj.get(current, []):
            if neighbor in visited:
                continue
            new_nodes = path_nodes + [neighbor]
            new_edges = path_edges + [edge_key]
            if neighbor == dst:
                # 构建 morse marks（需要 morse landscape，但签名中无 morse 参数）
                # 从 relations 中重建 edge_marks
                morse = build_morse_landscape(
                    relations_path=base / "relations.jsonl",
                    cache_path=None,
                )
                marks = [
                    morse.edge_marks.get(ek, "tree") for ek in new_edges
                ]
                return {
                    "nodes": new_nodes,
                    "edges": new_edges,
                    "marks": marks,
                    "length": len(new_edges),
                }
            visited.add(neighbor)
            queue.append((neighbor, new_nodes, new_edges))

    return None


def query_concept(
    concept_id: str,
    registry: ConceptRegistry,
) -> dict:
    """从概念注册表查询。概念不存在返回空结果。"""
    entry = registry.entries.get(concept_id)
    if entry is None:
        return {
            "concept_id": concept_id,
            "term": "",
            "authoritative": False,
            "defining_blocks": [],
            "reference_count": 0,
        }
    return {
        "concept_id": concept_id,
        "term": entry.term,
        "authoritative": entry.authoritative,
        "defining_blocks": entry.defining_blocks,
        "reference_count": entry.reference_count,
    }


# --- CLI formatting ---

def _format_block_table(data: dict) -> str:
    lines = [
        f"Block: {data['block_id']}",
        f"  genealogy_number: {data['genealogy_number']}",
        f"  title: {data['title']}",
        f"  type: {data['type']}",
        f"  status: {data['status']}",
        f"  morse: critical_out={data['morse_summary']['critical_out']}, "
        f"critical_in={data['morse_summary']['critical_in']}",
    ]
    if data["out_edges"]:
        lines.append("  out_edges:")
        for rtype, edges in data["out_edges"].items():
            for e in edges:
                mark = f" [{e['mark']}]" if "mark" in e else ""
                lines.append(f"    {rtype} -> {e['to']}{mark}")
    if data["in_edges"]:
        lines.append("  in_edges:")
        for rtype, edges in data["in_edges"].items():
            for e in edges:
                mark = f" [{e['mark']}]" if "mark" in e else ""
                lines.append(f"    {rtype} <- {e['from']}{mark}")
    if data["concepts"]:
        lines.append("  concepts:")
        for c in data["concepts"]:
            auth = " [authoritative]" if c["authoritative"] else ""
            lines.append(f"    {c['concept_id']}: {c['term']}{auth}")
    return "\n".join(lines)


def _format_pair_table(data: dict) -> str:
    lines = [
        f"Path length: {data['length']}",
        f"Nodes: {' -> '.join(data['nodes'])}",
    ]
    if data["edges"]:
        lines.append("Edges:")
        for ek, mk in zip(data["edges"], data["marks"]):
            lines.append(f"  {ek} [{mk}]")
    return "\n".join(lines)


def _format_concept_table(data: dict) -> str:
    lines = [
        f"Concept: {data['concept_id']}",
        f"  term: {data['term']}",
        f"  authoritative: {data['authoritative']}",
        f"  defining_blocks: {', '.join(data['defining_blocks']) or '(none)'}",
        f"  reference_count: {data['reference_count']}",
    ]
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description="穿越基础设施查询接口")
    sub = parser.add_subparsers(dest="command")

    # block 子命令
    p_block = sub.add_parser("block", help="查询区块穿越视图")
    p_block.add_argument("block_id")
    p_block.add_argument("--format", choices=["json", "table"], default="table")

    # pair 子命令
    p_pair = sub.add_parser("pair", help="查询两区块间最短路径")
    p_pair.add_argument("src")
    p_pair.add_argument("dst")
    p_pair.add_argument("--format", choices=["json", "table"], default="table")

    # concept 子命令
    p_concept = sub.add_parser("concept", help="查询概念")
    p_concept.add_argument("concept_id_or_term")
    p_concept.add_argument("--format", choices=["json", "table"], default="table")

    args = parser.parse_args()

    if args.command is None:
        parser.print_help()
        sys.exit(1)

    if args.command == "block":
        morse = build_morse_landscape()
        registry = load_registry()
        result = query_block(args.block_id, morse, registry)
        if result is None:
            print(f"Block not found: {args.block_id}", file=sys.stderr)
            sys.exit(1)
        if args.format == "json":
            print(json.dumps(result, ensure_ascii=False, indent=2))
        else:
            print(_format_block_table(result))

    elif args.command == "pair":
        result = query_pair(args.src, args.dst)
        if result is None:
            print(
                f"unreachable: {args.src} -> {args.dst}",
                file=sys.stderr,
            )
            sys.exit(0)
        if args.format == "json":
            print(json.dumps(result, ensure_ascii=False, indent=2))
        else:
            print(_format_pair_table(result))

    elif args.command == "concept":
        registry = load_registry()
        # 尝试先按 concept_id 查，找不到再按 term 查
        result = query_concept(args.concept_id_or_term, registry)
        if not result["term"]:
            # 按 term 搜索
            for cid, entry in registry.entries.items():
                if entry.term == args.concept_id_or_term:
                    result = query_concept(cid, registry)
                    break
        if args.format == "json":
            print(json.dumps(result, ensure_ascii=False, indent=2))
        else:
            print(_format_concept_table(result))


if __name__ == "__main__":
    main()
