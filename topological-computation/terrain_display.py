"""Morse terrain display — formatted text output of topological features.

Pure Python, no external dependencies. CLI entry point at bottom.
"""

from __future__ import annotations

import sys
from pathlib import Path

from engine import Graph, compute_beta_1, _connected_components
from morse import compute_terrain, critical_neighbors, tree_neighbors


# ---------------------------------------------------------------------------
# Keyword -> vertex_id resolution
# ---------------------------------------------------------------------------

def _resolve_vertex(keyword: str, graph: Graph) -> str | None:
    """Resolve a keyword to a vertex_id by exact match, then substring on id/content."""
    # Exact match
    if graph.vertex(keyword) is not None:
        return keyword

    keyword_lower = keyword.lower()
    candidates: list[str] = []
    for vid in graph.active_vertex_ids():
        if keyword_lower in vid.lower():
            candidates.append(vid)
            continue
        v = graph.vertex(vid)
        if v and v.content and keyword_lower in v.content.lower():
            candidates.append(vid)

    if not candidates:
        return None
    # Prefer shortest id match (most specific)
    candidates.sort(key=lambda x: (len(x), x))
    return candidates[0]


# ---------------------------------------------------------------------------
# f(v,w) computation (same formula as traversal engine)
# ---------------------------------------------------------------------------

def _compute_f(v: str, w: str, graph: Graph) -> int:
    """Compute f(v,w) = (c-1) + n_loop."""
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


def _zone_label(f: int) -> str:
    """Map f value to zone name."""
    if f <= 0:
        return "fold zone"
    elif f <= 5:
        return "gray zone"
    else:
        return "negate zone"


# ---------------------------------------------------------------------------
# Display functions
# ---------------------------------------------------------------------------

def display_terrain(vertex_id: str, graph: Graph, terrain: dict) -> str:
    """Return formatted terrain info for a single vertex."""
    v = graph.vertex(vertex_id)
    if v is None:
        return f"Vertex '{vertex_id}' not found in graph."

    content = v.content or "(no content)"
    status = v.status.value

    # Degree breakdown
    crit_nbs = critical_neighbors(graph, vertex_id, terrain)
    tree_nbs = tree_neighbors(graph, vertex_id, terrain)
    total_degree = len(graph.neighbors(vertex_id))

    # Compute avg f for all neighbors
    f_values: list[tuple[int, str]] = []
    for nb in graph.neighbors(vertex_id):
        f_val = _compute_f(vertex_id, nb, graph)
        f_values.append((f_val, nb))
    f_values.sort()

    avg_f = sum(fv for fv, _ in f_values) / len(f_values) if f_values else 0.0
    zone = _zone_label(int(avg_f))

    # Settled cycle participation
    # (not directly available without SettlementTracker, show degree-based proxy)

    lines = [
        f"=== [{v.status.value}] {vertex_id} ===",
        f'Content: "{content}"',
        f"Zone: {zone} (avg_f={avg_f:.1f})",
        f"Degree: {total_degree} ({len(crit_nbs)} critical, {len(tree_nbs)} tree)",
    ]

    # Top neighbors by f
    if f_values:
        lines.append("Top neighbors by f:")
        shown = 0
        for f_val, nb_id in f_values:
            if shown >= 8:
                lines.append(f"  ... and {len(f_values) - shown} more")
                break
            nb = graph.vertex(nb_id)
            nb_content = (nb.content or "")[:60]
            nb_zone = _zone_label(f_val)
            mark = "C" if nb_id in crit_nbs else "T"
            lines.append(f"  f={f_val:3d} [{mark}]: {nb_id}")
            if nb_content:
                lines.append(f"         \"{nb_content}\"")
            shown += 1

    return "\n".join(lines)


def display_local_map(vertex_id: str, graph: Graph, terrain: dict, radius: int = 1) -> str:
    """Return local map (r-hop neighborhood) as formatted text."""
    v = graph.vertex(vertex_id)
    if v is None:
        return f"Vertex '{vertex_id}' not found in graph."

    verts, edges = graph.local_subgraph(vertex_id, radius)

    lines = [
        f"=== Local map: {vertex_id} (radius={radius}) ===",
        f"Vertices: {len(verts)}  Edges: {len(edges)}",
        "",
    ]

    # List vertices with annotations
    for vid in verts:
        vv = graph.vertex(vid)
        status = vv.status.value if vv else "?"
        content = (vv.content or "")[:50] if vv else ""
        marker = " *" if vid == vertex_id else ""
        f_val = _compute_f(vertex_id, vid, graph) if vid != vertex_id else 0
        zone = _zone_label(f_val) if vid != vertex_id else "self"
        lines.append(f"  {vid}{marker}")
        if content:
            lines.append(f"    [{status}] \"{content}\"")
        if vid != vertex_id:
            lines.append(f"    f={f_val} ({zone})")

    # List edges
    lines.append("")
    lines.append("Edges:")
    for e in edges:
        mark = terrain.get((e.source, e.target), "?")
        arrow = "-->" if mark == "tree" else "==>"  # ==> for critical
        lines.append(f"  {e.source} {arrow} {e.target}  [{e.edge_type.value}, {mark}]")

    return "\n".join(lines)


def display_pair(v1: str, v2: str, graph: Graph, terrain: dict) -> str:
    """Return detailed comparison of two vertices."""
    vx1 = graph.vertex(v1)
    vx2 = graph.vertex(v2)

    if vx1 is None:
        return f"Vertex '{v1}' not found."
    if vx2 is None:
        return f"Vertex '{v2}' not found."

    f_val = _compute_f(v1, v2, graph)
    zone = _zone_label(f_val)

    # Shared neighbors
    nbs1 = set(graph.neighbors(v1))
    nbs2 = set(graph.neighbors(v2))
    shared = sorted(nbs1 & nbs2 - {v1, v2})
    only1 = sorted(nbs1 - nbs2 - {v1, v2})
    only2 = sorted(nbs2 - nbs1 - {v1, v2})

    # Jaccard similarity
    union = nbs1 | nbs2 - {v1, v2}
    jaccard = len(shared) / len(union) if union else 0.0

    # Direct edges between them
    direct_edges = []
    for e in graph.active_edges():
        if (e.source == v1 and e.target == v2) or (e.source == v2 and e.target == v1):
            mark = terrain.get((e.source, e.target), "?")
            direct_edges.append(f"{e.source}->{e.target} [{e.edge_type.value}, {mark}]")

    c1 = (vx1.content or "(no content)")[:80]
    c2 = (vx2.content or "(no content)")[:80]

    lines = [
        f"=== Pair: {v1} vs {v2} ===",
        f'A: "{c1}" [{vx1.status.value}]',
        f'B: "{c2}" [{vx2.status.value}]',
        "",
        f"f({v1},{v2}) = {f_val}  zone: {zone}",
        f"Jaccard similarity: {jaccard:.3f}",
        f"Shared neighbors ({len(shared)}): {', '.join(shared[:10]) if shared else '(none)'}",
        f"Only in A ({len(only1)}): {', '.join(only1[:5]) if only1 else '(none)'}",
        f"Only in B ({len(only2)}): {', '.join(only2[:5]) if only2 else '(none)'}",
    ]

    if direct_edges:
        lines.append(f"Direct edges: {'; '.join(direct_edges)}")
    else:
        lines.append("Direct edges: (none)")

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> None:
    from genealogy_loader import load_from_repo

    here = Path(__file__).resolve().parent
    repo_root = here.parent

    graph, _ = load_from_repo(repo_root)
    terrain = compute_terrain(graph)

    args = sys.argv[1:]
    if not args:
        print("Usage:")
        print('  python terrain_display.py "keyword"              # single vertex terrain')
        print('  python terrain_display.py "keyword1" "keyword2"  # pair comparison')
        print('  python terrain_display.py --map "keyword"        # local map')
        sys.exit(1)

    if args[0] == "--map":
        if len(args) < 2:
            print("--map requires a keyword argument")
            sys.exit(1)
        keyword = args[1]
        radius = int(args[2]) if len(args) > 2 else 1
        vid = _resolve_vertex(keyword, graph)
        if vid is None:
            print(f"No vertex found matching '{keyword}'")
            sys.exit(1)
        print(display_local_map(vid, graph, terrain, radius))

    elif len(args) == 1:
        keyword = args[0]
        vid = _resolve_vertex(keyword, graph)
        if vid is None:
            print(f"No vertex found matching '{keyword}'")
            sys.exit(1)
        print(display_terrain(vid, graph, terrain))

    elif len(args) == 2:
        k1, k2 = args
        v1 = _resolve_vertex(k1, graph)
        v2 = _resolve_vertex(k2, graph)
        if v1 is None:
            print(f"No vertex found matching '{k1}'")
            sys.exit(1)
        if v2 is None:
            print(f"No vertex found matching '{k2}'")
            sys.exit(1)
        print(display_pair(v1, v2, graph, terrain))

    else:
        print("Too many arguments. Use 1 keyword or 2 keywords.")
        sys.exit(1)


if __name__ == "__main__":
    main()
