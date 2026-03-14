"""Morse terrain — BFS spanning tree + tree/critical edge marking.

Pure Python, no external dependencies.
"""

from __future__ import annotations

from collections import deque

from engine import Graph


def compute_terrain(graph: Graph) -> dict[tuple[str, str], str]:
    """Compute tree/critical marking for all active directed edges.

    BFS spanning tree from the earliest-created vertex.
    Tree edges = BFS tree edges (undirected match).
    Critical edges = all other edges (participate in cycles).

    Returns dict mapping (source, target) -> "tree" | "critical".

    Cached on Graph instance since Graph is immutable.
    """
    try:
        return graph._cached_terrain
    except AttributeError:
        pass

    active_vids = list(graph._active_ids)
    if not active_vids:
        result: dict[tuple[str, str], str] = {}
        object.__setattr__(graph, '_cached_terrain', result)
        return result

    active_edges = graph.active_edges()
    if not active_edges:
        result = {}
        object.__setattr__(graph, '_cached_terrain', result)
        return result

    # Pick root: earliest created vertex (smallest id as tiebreak)
    root = min(active_vids, key=lambda v: (graph.vertex(v).created_at, v))

    # BFS to build undirected spanning tree
    tree_undirected: set[frozenset[str]] = set()
    visited: set[str] = {root}
    queue = deque([root])

    # Build adjacency for BFS (undirected) using adj index — avoid O(E) iteration
    adj: dict[str, set[str]] = {}
    active_set = graph._active_ids
    for vid in active_vids:
        nbs: set[str] = set()
        for e in graph._adj_out.get(vid, ()):
            if e.target in active_set and e.target != vid and e.edge_type not in _MATERIAL_TYPES:
                nbs.add(e.target)
        for e in graph._adj_in.get(vid, ()):
            if e.source in active_set and e.source != vid and e.edge_type not in _MATERIAL_TYPES:
                nbs.add(e.source)
        adj[vid] = nbs

    while queue:
        cur = queue.popleft()
        for nb in adj.get(cur, set()):
            if nb not in visited:
                visited.add(nb)
                tree_undirected.add(frozenset((cur, nb)))
                queue.append(nb)

    # Handle disconnected components
    for v in active_vids:
        if v not in visited:
            visited.add(v)
            queue.append(v)
            while queue:
                cur = queue.popleft()
                for nb in adj.get(cur, set()):
                    if nb not in visited:
                        visited.add(nb)
                        tree_undirected.add(frozenset((cur, nb)))
                        queue.append(nb)

    # Mark each directed edge
    terrain: dict[tuple[str, str], str] = {}
    for e in active_edges:
        if e.source == e.target:
            terrain[(e.source, e.target)] = "critical"  # self-loops are always critical
        elif frozenset((e.source, e.target)) in tree_undirected:
            terrain[(e.source, e.target)] = "tree"
        else:
            terrain[(e.source, e.target)] = "critical"

    object.__setattr__(graph, '_cached_terrain', terrain)
    return terrain


# Material layer edge types excluded from terrain computation
from engine import EdgeType
_MATERIAL_TYPES = (EdgeType.COOCCURRENCE, EdgeType.TRAVERSAL_ASSOCIATION)


def critical_neighbors(graph: Graph, vertex: str, terrain: dict[tuple[str, str], str]) -> list[str]:
    """Return vertices reachable from `vertex` via critical edges.

    Uses adjacency index for O(deg) instead of O(|terrain|).
    """
    active = graph._active_ids
    result: set[str] = set()
    for e in graph._adj_out.get(vertex, ()):
        if e.target in active and e.target != vertex:
            if terrain.get((e.source, e.target)) == "critical":
                result.add(e.target)
    for e in graph._adj_in.get(vertex, ()):
        if e.source in active and e.source != vertex:
            if terrain.get((e.source, e.target)) == "critical":
                result.add(e.source)
    return sorted(result)


def tree_neighbors(graph: Graph, vertex: str, terrain: dict[tuple[str, str], str]) -> list[str]:
    """Return vertices reachable from `vertex` via tree edges.

    Uses adjacency index for O(deg) instead of O(|terrain|).
    """
    active = graph._active_ids
    result: set[str] = set()
    for e in graph._adj_out.get(vertex, ()):
        if e.target in active and e.target != vertex:
            if terrain.get((e.source, e.target)) == "tree":
                result.add(e.target)
    for e in graph._adj_in.get(vertex, ()):
        if e.source in active and e.source != vertex:
            if terrain.get((e.source, e.target)) == "tree":
                result.add(e.source)
    return sorted(result)
