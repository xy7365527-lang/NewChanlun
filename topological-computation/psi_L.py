"""psi_L: Graph state -> structured text report.

Templated output layer. Reads Graph state and produces human-readable reports.
"""

from __future__ import annotations

from engine import Graph, EdgeType, VertexStatus, compute_beta_1
from morse import compute_terrain, critical_neighbors


def report_graph(graph: Graph) -> str:
    """Generate a structured text report of graph state."""
    lines: list[str] = []
    verts = graph.vertices
    active_ids = set(graph.active_vertex_ids())
    active_edges = graph.active_edges()

    # -- Vertices --
    active_verts = {vid: v for vid, v in verts.items() if vid in active_ids}
    folded_verts = {vid: v for vid, v in verts.items() if vid not in active_ids}

    lines.append(f"=== Graph Report ===")
    lines.append(f"Active vertices: {len(active_verts)}  |  Folded: {len(folded_verts)}  |  Total: {len(verts)}")
    lines.append(f"Active edges: {len(active_edges)}  |  Total: {len(graph.edges)}")
    lines.append("")

    lines.append("-- Vertices --")
    for vid in sorted(active_verts):
        v = active_verts[vid]
        status_tag = f" [{v.status.value}]" if v.status != VertexStatus.ACTIVE else ""
        content_tag = f" '{v.content}'" if v.content else ""
        lines.append(f"  {vid}{content_tag}{status_tag}")

    if folded_verts:
        lines.append("")
        lines.append("-- Folded vertices --")
        for vid in sorted(folded_verts):
            v = folded_verts[vid]
            content_tag = f" '{v.content}'" if v.content else ""
            lines.append(f"  {vid}{content_tag} [folded]")

    # -- Edges --
    lines.append("")
    lines.append("-- Edges --")
    edge_by_type: dict[str, list[str]] = {}
    for e in active_edges:
        src_label = _vertex_label(verts, e.source)
        tgt_label = _vertex_label(verts, e.target)
        entry = f"  {src_label} --> {tgt_label}"
        edge_by_type.setdefault(e.edge_type.value, []).append(entry)

    for etype in ("dependency", "negation", "sublation", "reference", "fold"):
        entries = edge_by_type.get(etype, [])
        if entries:
            lines.append(f"  [{etype}] ({len(entries)})")
            for entry in entries:
                lines.append(f"    {entry.strip()}")

    # -- beta_1 --
    beta = compute_beta_1(graph)
    lines.append("")
    lines.append(f"-- Topology --")
    lines.append(f"  beta_1 = {beta}")

    # -- Terrain summary --
    terrain = compute_terrain(graph)
    n_tree = sum(1 for m in terrain.values() if m == "tree")
    n_crit = sum(1 for m in terrain.values() if m == "critical")
    lines.append(f"  tree edges: {n_tree}  |  critical edges: {n_crit}")

    # -- Contested vertices --
    contested = [vid for vid, v in active_verts.items() if v.status == VertexStatus.CONTESTED]
    if contested:
        lines.append(f"  contested: {', '.join(sorted(contested))}")

    return "\n".join(lines)


def report_encounter(
    encounter_type: str,
    vertex_a: str,
    vertex_b: str | None,
    f_value: int,
    context: str = "",
) -> str:
    """Generate a human-readable encounter report."""
    parts: list[str] = []
    parts.append(f"Encounter: {encounter_type}")
    if vertex_b is not None:
        parts.append(f"Vertices: {vertex_a} vs {vertex_b}")
    else:
        parts.append(f"Vertex: {vertex_a}")

    zone = _f_zone(f_value)
    parts.append(f"f = {f_value} ({zone})")

    if context:
        parts.append(f"Context: {context}")

    return " | ".join(parts)


def report_comparison(
    graph_a: Graph,
    graph_b: Graph,
    label_a: str = "A",
    label_b: str = "B",
) -> str:
    """Compare two graphs side by side."""
    lines: list[str] = []
    lines.append(f"=== Comparison: {label_a} vs {label_b} ===")
    lines.append("")

    a_active = len(graph_a.active_vertex_ids())
    b_active = len(graph_b.active_vertex_ids())
    a_edges = len(graph_a.active_edges())
    b_edges = len(graph_b.active_edges())
    a_beta = compute_beta_1(graph_a)
    b_beta = compute_beta_1(graph_b)

    lines.append(f"{'Metric':<25} {label_a:>10} {label_b:>10} {'Delta':>10}")
    lines.append("-" * 57)
    lines.append(f"{'Active vertices':<25} {a_active:>10} {b_active:>10} {b_active - a_active:>+10}")
    lines.append(f"{'Active edges':<25} {a_edges:>10} {b_edges:>10} {b_edges - a_edges:>+10}")
    lines.append(f"{'beta_1':<25} {a_beta:>10} {b_beta:>10} {b_beta - a_beta:>+10}")

    # Edge type breakdown
    def _edge_type_counts(g: Graph) -> dict[str, int]:
        counts: dict[str, int] = {}
        for e in g.active_edges():
            counts[e.edge_type.value] = counts.get(e.edge_type.value, 0) + 1
        return counts

    a_counts = _edge_type_counts(graph_a)
    b_counts = _edge_type_counts(graph_b)
    all_types = sorted(set(a_counts) | set(b_counts))

    if all_types:
        lines.append("")
        lines.append("-- Edge type breakdown --")
        for etype in all_types:
            ac = a_counts.get(etype, 0)
            bc = b_counts.get(etype, 0)
            lines.append(f"{'  ' + etype:<25} {ac:>10} {bc:>10} {bc - ac:>+10}")

    # Terrain comparison
    terrain_a = compute_terrain(graph_a)
    terrain_b = compute_terrain(graph_b)
    a_crit = sum(1 for m in terrain_a.values() if m == "critical")
    b_crit = sum(1 for m in terrain_b.values() if m == "critical")
    lines.append("")
    lines.append(f"{'Critical edges':<25} {a_crit:>10} {b_crit:>10} {b_crit - a_crit:>+10}")

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _vertex_label(verts: dict, vid: str) -> str:
    """Return a compact label for a vertex."""
    v = verts.get(vid)
    if v and v.content:
        return f"{vid}({v.content})"
    return vid


def _f_zone(f: int) -> str:
    """Classify f value into zone."""
    if f < 0:
        return "no shared neighbors"
    if f == 0:
        return "fold zone"
    if f <= 2:
        return "gray zone"
    return "negate zone"
