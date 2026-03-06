"""Query Interface — topological information for concepts.

Pure Python, no external dependencies.

CLI usage:
    python query_interface.py "genus"              # single concept query
    python query_interface.py "genus" "Che vuoi"   # concept pair query
"""

from __future__ import annotations

import json
import sys
import time
from collections import Counter
from pathlib import Path

from engine import Graph, compute_beta_1, _connected_components
from genealogy_loader import load_from_repo
from morse import compute_terrain, critical_neighbors, tree_neighbors
from concept_registry import build_registry, Registry


# ---------------------------------------------------------------------------
# f(v, w) computation (same logic as TraversalEngine._compute_f)
# ---------------------------------------------------------------------------

def _compute_f(graph: Graph, v: str, w: str) -> int:
    """Compute f(v,w) = (c-1) + n_loop.

    f = -1: no shared neighbors (topo distant)
    f = 0: topo nearly equivalent (safe fold zone)
    f > 0: gray zone to negate zone
    """
    active = set(graph.active_vertex_ids())
    s = {v, w}
    n_loop = sum(
        1 for e in graph.active_edges()
        if e.source in s and e.target in s
    )
    lower_link: set[str] = set()
    for x in s:
        for n in graph.neighbors(x):
            if n not in s and n in active:
                lower_link.add(n)
    if not lower_link:
        return -1 + n_loop
    ll_edges: list[frozenset[str]] = []
    for e in graph.active_edges():
        if e.source in lower_link and e.target in lower_link:
            ll_edges.append(frozenset((e.source, e.target)))
    c = _connected_components(sorted(lower_link), ll_edges)
    return (c - 1) + n_loop


def _compute_f_decomposition(graph: Graph, v: str, w: str) -> dict:
    """Return f(v,w) decomposed into c, n_loop, m (Q-R3-12 identity).

    f = (c - 1) + n_loop
    m = |lower_link|
    """
    active = set(graph.active_vertex_ids())
    s = {v, w}
    n_loop = sum(
        1 for e in graph.active_edges()
        if e.source in s and e.target in s
    )
    lower_link: set[str] = set()
    for x in s:
        for n in graph.neighbors(x):
            if n not in s and n in active:
                lower_link.add(n)
    if not lower_link:
        c = 0
    else:
        ll_edges: list[frozenset[str]] = []
        for e in graph.active_edges():
            if e.source in lower_link and e.target in lower_link:
                ll_edges.append(frozenset((e.source, e.target)))
        c = _connected_components(sorted(lower_link), ll_edges)

    f = (c - 1 if c > 0 else -1) + n_loop
    return {"c": c, "n_loop": n_loop, "m": len(lower_link), "f": f}


# ---------------------------------------------------------------------------
# Zone classification
# ---------------------------------------------------------------------------

def _classify_zone(avg_f: float) -> str:
    """Classify a vertex or pair into terrain zone based on average f."""
    if avg_f < 5:
        return "fold_zone"
    elif avg_f <= 12:
        return "gray_zone"
    else:
        return "negate_zone"


# ---------------------------------------------------------------------------
# Shortest path (BFS, undirected on active graph)
# ---------------------------------------------------------------------------

def _shortest_path(graph: Graph, source: str, target: str) -> list[str] | None:
    """BFS shortest path on undirected active graph. Returns vertex list or None."""
    from collections import deque

    active = set(graph.active_vertex_ids())
    if source not in active or target not in active:
        return None

    parent: dict[str, str | None] = {source: None}
    queue = deque([source])

    while queue:
        cur = queue.popleft()
        if cur == target:
            path = []
            node = target
            while node is not None:
                path.append(node)
                node = parent[node]
            return list(reversed(path))
        for nb in graph.neighbors(cur):
            if nb not in parent:
                parent[nb] = cur
                queue.append(nb)

    return None


# ---------------------------------------------------------------------------
# Settled cycle counting per vertex
# ---------------------------------------------------------------------------

def _count_settled_cycles(graph: Graph, vertex_id: str, settlement_tracker=None) -> int:
    """Count settled cycles that include vertex_id.

    Without a settlement tracker, returns 0 (we don't have traversal state).
    This is a structural query — settled cycles require a traversal run.
    """
    # In a static query context, we approximate by counting cycles in the
    # undirected graph passing through this vertex (up to small cycles).
    # For now, return 0 — settled cycles are a runtime concept.
    return 0


# ---------------------------------------------------------------------------
# query_concept
# ---------------------------------------------------------------------------

def query_concept(
    keyword: str,
    graph: Graph,
    registry: Registry,
    terrain: dict[tuple[str, str], str],
) -> dict | None:
    """Query a single concept by keyword.

    Returns dict with topological information, or None if not found.
    """
    matches = registry.lookup(keyword)
    if not matches:
        return None

    vertex_id, content = matches[0]

    # Neighbors with f values
    nbs = graph.neighbors(vertex_id)
    nb_info = []
    f_values = []
    for nb in nbs:
        f_val = _compute_f(graph, vertex_id, nb)
        nb_content = registry.reverse(nb)
        nb_info.append({
            "vertex_id": nb[:16] + "...",
            "content": nb_content,
            "f_value": f_val,
        })
        f_values.append(f_val)

    nb_info.sort(key=lambda x: x["f_value"])
    f_values.sort()

    # Shared-with: other concepts sharing most neighbors (top 5)
    nb_set = set(nbs)
    shared_counts: list[tuple[str, str, int]] = []
    for vid in graph.active_vertex_ids():
        if vid == vertex_id:
            continue
        other_nbs = set(graph.neighbors(vid))
        shared = nb_set & other_nbs
        if shared:
            shared_counts.append((vid, registry.reverse(vid), len(shared)))
    shared_counts.sort(key=lambda x: -x[2])
    shared_with = [
        {"vertex_id": vid[:16] + "...", "content": c, "shared_count": n}
        for vid, c, n in shared_counts[:5]
    ]

    # Terrain mark: critical vs tree neighbor ratio
    crit_nbs = critical_neighbors(graph, vertex_id, terrain)
    tree_nbs = tree_neighbors(graph, vertex_id, terrain)
    total_typed = len(crit_nbs) + len(tree_nbs)
    if total_typed > 0:
        terrain_mark = f"{len(crit_nbs)} critical / {len(tree_nbs)} tree ({len(crit_nbs)/total_typed:.0%} critical)"
    else:
        terrain_mark = "no edges"

    # Zone
    avg_f = sum(f_values) / len(f_values) if f_values else 0
    zone = _classify_zone(avg_f)

    return {
        "vertex_id": vertex_id[:16] + "...",
        "vertex_id_full": vertex_id,
        "content": content,
        "total_neighbors": len(nbs),
        "f_values": f_values,
        "avg_f": round(avg_f, 2),
        "neighbors": nb_info,
        "shared_with": shared_with,
        "settled_cycles": _count_settled_cycles(graph, vertex_id),
        "terrain_mark": terrain_mark,
        "zone": zone,
        "other_matches": len(matches) - 1,
    }


# ---------------------------------------------------------------------------
# query_pair
# ---------------------------------------------------------------------------

def query_pair(
    keyword_a: str,
    keyword_b: str,
    graph: Graph,
    registry: Registry,
    terrain: dict[tuple[str, str], str],
) -> dict | None:
    """Query a concept pair by keywords.

    Returns dict with pair topological information, or None if either not found.
    """
    matches_a = registry.lookup(keyword_a)
    matches_b = registry.lookup(keyword_b)

    if not matches_a or not matches_b:
        return None

    vid_a, content_a = matches_a[0]
    vid_b, content_b = matches_b[0]

    if vid_a == vid_b:
        return {"error": "Both keywords resolve to the same vertex"}

    # f value and decomposition
    decomp = _compute_f_decomposition(graph, vid_a, vid_b)
    f_val = decomp["f"]

    # Jaccard similarity of neighbor sets
    nbs_a = set(graph.neighbors(vid_a))
    nbs_b = set(graph.neighbors(vid_b))
    union = nbs_a | nbs_b
    intersection = nbs_a & nbs_b
    jaccard = len(intersection) / len(union) if union else 0.0

    # Shared neighbors
    shared = sorted(intersection - {vid_a, vid_b})
    shared_info = [
        {"vertex_id": s[:16] + "...", "content": registry.reverse(s)}
        for s in shared
    ]

    # Path (if not direct neighbors)
    are_neighbors = vid_b in nbs_a
    path = None
    if not are_neighbors:
        raw_path = _shortest_path(graph, vid_a, vid_b)
        if raw_path:
            path = [
                {"vertex_id": v[:16] + "...", "content": registry.reverse(v)}
                for v in raw_path
            ]

    # Zone
    zone = _classify_zone(f_val)

    return {
        "vertex_a": vid_a[:16] + "...",
        "vertex_b": vid_b[:16] + "...",
        "content_a": content_a,
        "content_b": content_b,
        "are_neighbors": are_neighbors,
        "f_value": f_val,
        "jaccard": round(jaccard, 4),
        "c": decomp["c"],
        "n_loop": decomp["n_loop"],
        "m": decomp["m"],
        "shared_neighbors": shared_info,
        "path": path,
        "zone": zone,
    }


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _print_result(result: dict) -> None:
    """Pretty-print a query result."""
    print(json.dumps(result, ensure_ascii=False, indent=2))


def main() -> None:
    args = sys.argv[1:]
    if not args:
        print("Usage:")
        print('  python query_interface.py "keyword"              # single concept')
        print('  python query_interface.py "keyword_a" "keyword_b"  # concept pair')
        sys.exit(1)

    here = Path(__file__).resolve().parent
    repo_root = here.parent

    t0 = time.time()
    graph, _ = load_from_repo(repo_root)
    t_load = time.time() - t0
    print(f"Graph loaded in {t_load:.3f}s ({len(graph.active_vertex_ids())} vertices, {len(graph.active_edges())} edges)")

    blocks_dir = str(repo_root / ".chanlun" / "block-topology" / "blocks")

    t0 = time.time()
    registry = build_registry(graph, blocks_dir)
    t_reg = time.time() - t0
    print(f"Registry built in {t_reg:.3f}s")

    t0 = time.time()
    terrain = compute_terrain(graph)
    t_terrain = time.time() - t0
    print(f"Terrain computed in {t_terrain:.3f}s")
    print()

    if len(args) == 1:
        result = query_concept(args[0], graph, registry, terrain)
        if result is None:
            print(f"No concept found for keyword: {args[0]}")
            sys.exit(1)
        _print_result(result)
    else:
        result = query_pair(args[0], args[1], graph, registry, terrain)
        if result is None:
            print(f"No concept found for one or both keywords: {args[0]}, {args[1]}")
            sys.exit(1)
        _print_result(result)


if __name__ == "__main__":
    main()
