#!/usr/bin/env python3
"""Query traversal interface for block-topology.

Usage:
    python query_traverse.py "概念名"           # exact or substring match
    python query_traverse.py --block 069        # query by genealogy_id
    python query_traverse.py --path 069 090     # find shortest path between two blocks
    python query_traverse.py --neighbors 069    # show direct neighbors
    python query_traverse.py --stats            # show topology statistics

Outputs human-readable text with block titles, relation types, and path lengths.
"""

import json
import sys
from collections import defaultdict, deque
from pathlib import Path

TOPO_DIR = Path(__file__).resolve().parent.parent
BLOCKS_DIR = TOPO_DIR / "blocks"
META_PATH = TOPO_DIR / "meta.json"
REGISTRY_PATH = TOPO_DIR / "concept_registry.json"
LANDSCAPE_PATH = TOPO_DIR / "morse_landscape.json"
RELATIONS_PATH = TOPO_DIR / "relations.jsonl"


def load_json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def load_relations():
    relations = []
    with open(RELATIONS_PATH, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    relations.append(json.loads(line))
                except json.JSONDecodeError:
                    continue
    return relations


def build_graph(relations):
    """Build adjacency lists (directed)."""
    forward = defaultdict(list)   # from -> [(to, relation)]
    backward = defaultdict(list)  # to -> [(from, relation)]
    for rel in relations:
        src = rel.get("from", "")
        dst = rel.get("to", "")
        rtype = rel.get("relation", "unknown")
        if src and dst:
            forward[src].append((dst, rtype))
            backward[dst].append((src, rtype))
    return forward, backward


def resolve_id(query, meta):
    """Resolve a genealogy_id to block hash."""
    id_mapping = meta.get("id_mapping", {})
    if query in id_mapping:
        return id_mapping[query]
    # Try with zero-padding
    for k, v in id_mapping.items():
        if k.lstrip("0") == query.lstrip("0"):
            return v
    return None


def get_block_label(block_hash, hash_to_id):
    """Get a human-readable label for a block."""
    gid = hash_to_id.get(block_hash, "")
    if gid:
        return f"[{gid}号]"
    return f"[{block_hash[:12]}...]"


def get_block_info(block_hash, hash_to_id):
    """Load and return block info."""
    gid = hash_to_id.get(block_hash, "")
    block_path = BLOCKS_DIR / f"{block_hash}.json"
    title = ""
    btype = ""
    if block_path.exists():
        try:
            with open(block_path, encoding="utf-8") as f:
                block = json.load(f)
            btype = block.get("type", "")
            content = block.get("content", {})
            if isinstance(content, dict):
                title = content.get("title", "")
                if not title:
                    ca = content.get("content_analysis", {})
                    if isinstance(ca, dict):
                        title = f"谱系{ca.get('genealogy_id', gid)}号内容充实"
        except (json.JSONDecodeError, UnicodeDecodeError):
            pass
    return {"genealogy_id": gid, "title": title, "type": btype}


def bfs_shortest_path(start, end, forward, backward):
    """BFS for shortest undirected path between two blocks."""
    if start == end:
        return [start]
    visited = {start}
    queue = deque([(start, [start])])
    while queue:
        current, path = queue.popleft()
        # Forward neighbors
        for neighbor, rtype in forward.get(current, []):
            if neighbor not in visited:
                new_path = path + [neighbor]
                if neighbor == end:
                    return new_path
                visited.add(neighbor)
                queue.append((neighbor, new_path))
        # Backward neighbors
        for neighbor, rtype in backward.get(current, []):
            if neighbor not in visited:
                new_path = path + [neighbor]
                if neighbor == end:
                    return new_path
                visited.add(neighbor)
                queue.append((neighbor, new_path))
    return None


def get_path_relations(path, forward):
    """Get relation types along a path."""
    edges = []
    for i in range(len(path) - 1):
        src, dst = path[i], path[i + 1]
        # Check forward
        for neighbor, rtype in forward.get(src, []):
            if neighbor == dst:
                edges.append((src, dst, rtype, "->"))
                break
        else:
            # Check reverse
            for neighbor, rtype in forward.get(dst, []):
                if neighbor == src:
                    edges.append((src, dst, rtype, "<-"))
                    break
            else:
                edges.append((src, dst, "?", "--"))
    return edges


def cmd_concept_search(query, registry, meta, hash_to_id, forward, backward):
    """Search for a concept and show related blocks + paths."""
    matches = []
    for term, data in registry.items():
        if query == term:
            matches.insert(0, (term, data))
        elif query.lower() in term.lower():
            matches.append((term, data))

    if not matches:
        print(f"概念 '{query}' 未找到。")
        # Suggest similar concepts
        suggestions = [
            t for t in registry
            if any(c in t for c in query)
        ][:5]
        if suggestions:
            print(f"相近概念：{', '.join(suggestions)}")
        return

    print(f"找到 {len(matches)} 个匹配概念：\n")
    for term, data in matches[:10]:
        blocks = data["blocks"]
        print(f"  概念: {term}")
        print(f"  出现次数: {data['count']} 个区块")
        print(f"  首次出现: {data['first_seen']}")
        print(f"  最后出现: {data['last_seen']}")
        if data.get("is_new_concept"):
            print(f"  标记: 新概念")

        print(f"  相关区块:")
        for bh in blocks[:10]:
            info = get_block_info(bh, hash_to_id)
            label = get_block_label(bh, hash_to_id)
            neighbors_out = len(forward.get(bh, []))
            neighbors_in = len(backward.get(bh, []))
            title_str = f" — {info['title']}" if info["title"] else ""
            print(f"    {label}{title_str}  (度: {neighbors_in + neighbors_out})")

        if len(blocks) > 10:
            print(f"    ... 还有 {len(blocks) - 10} 个区块")

        # Show connections between concept blocks
        if len(blocks) >= 2:
            print(f"  概念内部连接:")
            block_set = set(blocks)
            internal_edges = 0
            for bh in blocks:
                for neighbor, rtype in forward.get(bh, []):
                    if neighbor in block_set:
                        src_label = get_block_label(bh, hash_to_id)
                        dst_label = get_block_label(neighbor, hash_to_id)
                        if internal_edges < 5:
                            print(f"    {src_label} --{rtype}--> {dst_label}")
                        internal_edges += 1
            if internal_edges == 0:
                print(f"    (无直接连接)")
            elif internal_edges > 5:
                print(f"    ... 还有 {internal_edges - 5} 条连接")

        print()


def cmd_block_query(gid, meta, hash_to_id, forward, backward):
    """Show details and neighbors of a specific block."""
    block_hash = resolve_id(gid, meta)
    if not block_hash:
        print(f"谱系号 '{gid}' 未找到。")
        return

    info = get_block_info(block_hash, hash_to_id)
    label = get_block_label(block_hash, hash_to_id)
    title_str = f" — {info['title']}" if info["title"] else ""
    print(f"区块: {label}{title_str}")
    print(f"类型: {info['type']}")
    print(f"哈希: {block_hash}")

    out_edges = forward.get(block_hash, [])
    in_edges = backward.get(block_hash, [])
    print(f"\n出度: {len(out_edges)}, 入度: {len(in_edges)}, 总度: {len(out_edges) + len(in_edges)}")

    if out_edges:
        print(f"\n→ 出向关系 ({len(out_edges)}):")
        by_type = defaultdict(list)
        for dst, rtype in out_edges:
            by_type[rtype].append(dst)
        for rtype in sorted(by_type.keys()):
            dsts = by_type[rtype]
            print(f"  [{rtype}] ({len(dsts)}):")
            for dst in dsts[:5]:
                dst_label = get_block_label(dst, hash_to_id)
                print(f"    -> {dst_label}")
            if len(dsts) > 5:
                print(f"    ... 还有 {len(dsts) - 5} 条")

    if in_edges:
        print(f"\n← 入向关系 ({len(in_edges)}):")
        by_type = defaultdict(list)
        for src, rtype in in_edges:
            by_type[rtype].append(src)
        for rtype in sorted(by_type.keys()):
            srcs = by_type[rtype]
            print(f"  [{rtype}] ({len(srcs)}):")
            for src in srcs[:5]:
                src_label = get_block_label(src, hash_to_id)
                print(f"    <- {src_label}")
            if len(srcs) > 5:
                print(f"    ... 还有 {len(srcs) - 5} 条")


def cmd_path(gid1, gid2, meta, hash_to_id, forward, backward):
    """Find shortest path between two blocks."""
    h1 = resolve_id(gid1, meta)
    h2 = resolve_id(gid2, meta)
    if not h1:
        print(f"谱系号 '{gid1}' 未找到。")
        return
    if not h2:
        print(f"谱系号 '{gid2}' 未找到。")
        return

    path = bfs_shortest_path(h1, h2, forward, backward)
    if not path:
        print(f"{gid1}号 与 {gid2}号 之间无路径（不连通）。")
        return

    print(f"最短路径: {gid1}号 → {gid2}号, 长度 = {len(path) - 1}\n")
    edges = get_path_relations(path, forward)
    for i, (src, dst, rtype, direction) in enumerate(edges):
        src_label = get_block_label(src, hash_to_id)
        dst_label = get_block_label(dst, hash_to_id)
        if direction == "->":
            print(f"  {src_label} --{rtype}--> {dst_label}")
        else:
            print(f"  {src_label} <--{rtype}-- {dst_label}")


def cmd_neighbors(gid, meta, hash_to_id, forward, backward):
    """Show direct neighbors with relation types."""
    cmd_block_query(gid, meta, hash_to_id, forward, backward)


def cmd_stats(meta, hash_to_id, forward, backward, registry):
    """Show topology statistics."""
    landscape = None
    if LANDSCAPE_PATH.exists():
        landscape = load_json(LANDSCAPE_PATH)

    if landscape:
        stats = landscape.get("statistics", {})
        print("拓扑统计:")
        print(f"  区块总数: {stats.get('total_blocks', '?')}")
        print(f"  关系总数: {stats.get('total_relations', '?')}")
        print(f"  平均度数: {stats.get('mean_height', '?')}")
        print(f"  度数标准差: {stats.get('std_height', '?')}")
        print(f"  最大度数: {stats.get('max_height', '?')}")

        peaks = landscape.get("peaks", [])
        if peaks:
            print(f"\n  山峰（高密度中心, 前10）:")
            for p in peaks[:10]:
                gid = p["genealogy_id"] or "?"
                print(f"    {gid:>5s}号 | 度={p['height']:3d} (入={p['in_degree']}, 出={p['out_degree']})")

    print(f"\n  概念总数: {len(registry)}")
    top_concepts = sorted(registry.items(), key=lambda x: -x[1]["count"])[:10]
    if top_concepts:
        print(f"  高频概念（前10）:")
        for term, data in top_concepts:
            print(f"    {data['count']:3d} 区块 | {term}")


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return

    meta = load_json(META_PATH)
    hash_to_id = {v: k for k, v in meta.get("id_mapping", {}).items()}
    registry = load_json(REGISTRY_PATH) if REGISTRY_PATH.exists() else {}
    relations = load_relations()
    forward, backward = build_graph(relations)

    if sys.argv[1] == "--block":
        if len(sys.argv) < 3:
            print("用法: python query_traverse.py --block <谱系号>")
            return
        cmd_block_query(sys.argv[2], meta, hash_to_id, forward, backward)
    elif sys.argv[1] == "--path":
        if len(sys.argv) < 4:
            print("用法: python query_traverse.py --path <谱系号1> <谱系号2>")
            return
        cmd_path(sys.argv[2], sys.argv[3], meta, hash_to_id, forward, backward)
    elif sys.argv[1] == "--neighbors":
        if len(sys.argv) < 3:
            print("用法: python query_traverse.py --neighbors <谱系号>")
            return
        cmd_neighbors(sys.argv[2], meta, hash_to_id, forward, backward)
    elif sys.argv[1] == "--stats":
        cmd_stats(meta, hash_to_id, forward, backward, registry)
    else:
        query = " ".join(sys.argv[1:])
        cmd_concept_search(query, registry, meta, hash_to_id, forward, backward)


if __name__ == "__main__":
    main()
