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
    """
    active_vids = graph.active_vertex_ids()
    if not active_vids:
        return {}

    active_edges = graph.active_edges()
    if not active_edges:
        return {}

    # Pick root: earliest created vertex (smallest id as tiebreak)
    root = min(active_vids, key=lambda v: (graph.vertex(v).created_at, v))

    # BFS to build undirected spanning tree
    tree_undirected: set[frozenset[str]] = set()
    visited: set[str] = {root}
    queue = deque([root])

    # Build adjacency for BFS (undirected)
    adj: dict[str, set[str]] = {v: set() for v in active_vids}
    for e in active_edges:
        if e.source != e.target:  # skip self-loops
            adj[e.source].add(e.target)
            adj[e.target].add(e.source)

    while queue:
        cur = queue.popleft()
        for nb in sorted(adj.get(cur, set())):
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
                for nb in sorted(adj.get(cur, set())):
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

    return terrain


def critical_neighbors(graph: Graph, vertex: str, terrain: dict[tuple[str, str], str]) -> list[str]:
    """Return vertices reachable from `vertex` via critical edges."""
    result: set[str] = set()
    for (src, tgt), mark in terrain.items():
        if mark == "critical":
            if src == vertex and tgt != vertex:
                result.add(tgt)
            elif tgt == vertex and src != vertex:
                result.add(src)
    return sorted(result)


def tree_neighbors(graph: Graph, vertex: str, terrain: dict[tuple[str, str], str]) -> list[str]:
    """Return vertices reachable from `vertex` via tree edges."""
    result: set[str] = set()
    for (src, tgt), mark in terrain.items():
        if mark == "tree":
            if src == vertex and tgt != vertex:
                result.add(tgt)
            elif tgt == vertex and src != vertex:
                result.add(src)
    return sorted(result)
