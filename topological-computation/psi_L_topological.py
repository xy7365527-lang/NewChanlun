"""psi_L_topological: Morse-Smale linearization of K_active.

Layer three of psi_L: from graph state directly to linear narrative
via critical edge cutting + topological sort. No traversal needed.

Algorithm (Gemini R5 Q-R5-4):
1. Compute Morse terrain (tree/critical edge marking)
2. Cut all critical edges — these are "narrative turning points"
3. Topological sort the remaining DAG
4. Re-insert critical edges as "flashback/Aufhebung" annotations
"""

from __future__ import annotations

from collections import deque

from engine import Graph, EdgeType, VertexStatus, compute_beta_1
from morse import compute_terrain


# ---------------------------------------------------------------------------
# Linearize: critical edge cutting + topological sort
# ---------------------------------------------------------------------------

def _find_sccs(adj: dict[str, list[str]], vertices: list[str]) -> list[list[str]]:
    """Tarjan's SCC algorithm. Returns list of SCCs (each a list of vertex ids)."""
    index_counter = [0]
    stack: list[str] = []
    on_stack: set[str] = set()
    index_map: dict[str, int] = {}
    lowlink: dict[str, int] = {}
    result: list[list[str]] = []

    def strongconnect(v: str) -> None:
        index_map[v] = index_counter[0]
        lowlink[v] = index_counter[0]
        index_counter[0] += 1
        stack.append(v)
        on_stack.add(v)

        for w in adj.get(v, []):
            if w not in index_map:
                strongconnect(w)
                lowlink[v] = min(lowlink[v], lowlink[w])
            elif w in on_stack:
                lowlink[v] = min(lowlink[v], index_map[w])

        if lowlink[v] == index_map[v]:
            scc: list[str] = []
            while True:
                w = stack.pop()
                on_stack.discard(w)
                scc.append(w)
                if w == v:
                    break
            result.append(scc)

    for v in vertices:
        if v not in index_map:
            strongconnect(v)

    return result


def _topo_sort_dag(adj: dict[str, list[str]], vertices: list[str]) -> list[str]:
    """Kahn's algorithm for topological sort. Assumes input is a DAG."""
    in_degree: dict[str, int] = {v: 0 for v in vertices}
    for v in vertices:
        for w in adj.get(v, []):
            if w in in_degree:
                in_degree[w] = in_degree.get(w, 0) + 1

    queue = deque(sorted(v for v in vertices if in_degree[v] == 0))
    order: list[str] = []

    while queue:
        v = queue.popleft()
        order.append(v)
        for w in sorted(adj.get(v, [])):
            if w in in_degree:
                in_degree[w] -= 1
                if in_degree[w] == 0:
                    queue.append(w)

    return order


def linearize_complex(
    graph: Graph,
    terrain: dict[tuple[str, str], str] | None = None,
) -> tuple[list[str], list[tuple[str, str]], list[tuple[str, str]]]:
    """Morse-Smale linearization: cut critical edges, topological sort remainder.

    Returns:
        vertex_order: linear ordering of active vertices
        critical_cuts: list of (source, target) edges that were cut
        extra_cuts: additional edges removed to break remaining cycles (if any)
    """
    active_vids = graph.active_vertex_ids()
    if not active_vids:
        return [], [], []

    active_edges = graph.active_edges()
    if terrain is None:
        terrain = compute_terrain(graph)

    # Step 1: Identify critical edges (narrative turning points)
    critical_cuts: list[tuple[str, str]] = []
    remaining_edges: list[tuple[str, str]] = []

    for e in active_edges:
        if e.source == e.target:
            critical_cuts.append((e.source, e.target))
            continue
        mark = terrain.get((e.source, e.target), "critical")
        if mark == "critical":
            critical_cuts.append((e.source, e.target))
        else:
            remaining_edges.append((e.source, e.target))

    # Step 2: Build adjacency from remaining (tree) edges
    adj: dict[str, list[str]] = {v: [] for v in active_vids}
    for src, tgt in remaining_edges:
        if src in adj and tgt in adj:
            adj[src].append(tgt)

    # Step 3: Check for remaining cycles (SCCs with size > 1)
    sccs = _find_sccs(adj, active_vids)
    extra_cuts: list[tuple[str, str]] = []

    multi_sccs = [scc for scc in sccs if len(scc) > 1]
    if multi_sccs:
        # Break remaining cycles: remove one edge per SCC (the last in DFS order)
        for scc in multi_sccs:
            scc_set = set(scc)
            for src in scc:
                for tgt in adj.get(src, []):
                    if tgt in scc_set:
                        # Remove this edge to break the cycle
                        extra_cuts.append((src, tgt))
                        adj[src] = [t for t in adj[src] if t != tgt]
                        break
                if extra_cuts and extra_cuts[-1][0] in scc_set:
                    break

    # Step 4: Topological sort
    # After removing critical + extra cuts, the graph should be a DAG
    # Handle potential disconnected components by including all vertices
    vertex_order = _topo_sort_dag(adj, active_vids)

    # If topo sort missed some vertices (remaining cycles), append them
    ordered_set = set(vertex_order)
    for v in active_vids:
        if v not in ordered_set:
            vertex_order.append(v)

    return vertex_order, critical_cuts, extra_cuts


# ---------------------------------------------------------------------------
# Narrative generation
# ---------------------------------------------------------------------------

def _vertex_label(graph: Graph, vid: str) -> str:
    """Get readable label for a vertex."""
    v = graph.vertex(vid)
    if v and v.content:
        return v.content
    return vid


def _edge_type_label(graph: Graph, src: str, tgt: str) -> str:
    """Get the edge type between two vertices."""
    for e in graph.active_edges():
        if e.source == src and e.target == tgt:
            return e.edge_type.value
    return "unknown"


def _classify_cut(
    graph: Graph, src: str, tgt: str,
) -> str:
    """Classify a cut edge for narrative annotation."""
    edge_type = _edge_type_label(graph, src, tgt)
    if edge_type == "negation":
        return "contradiction"
    if edge_type == "sublation":
        return "Aufhebung"
    if edge_type == "fold":
        return "identification"
    return "tension"


def generate_topological_narrative(
    graph: Graph,
    terrain: dict[tuple[str, str], str] | None = None,
    concept_names: dict[str, str] | None = None,
) -> str:
    """Layer three pure topological narrative: from graph state directly to report.

    No traversal needed. Reads the current graph topology and produces:
    1. Main line (topological sort of vertices after critical edge removal)
    2. Contradiction/Aufhebung insertions (critical edges annotated at cut points)
    3. Settled cycle annotations (load-bearing structures)

    Args:
        graph: the current Graph state
        terrain: pre-computed Morse terrain (computed if None)
        concept_names: optional vertex_id -> readable name override
    """
    if terrain is None:
        terrain = compute_terrain(graph)

    # Build name resolver
    names: dict[str, str] = {}
    if concept_names:
        names.update(concept_names)
    for vid, v in graph.vertices.items():
        if vid not in names and v.content:
            names[vid] = v.content

    def name(vid: str) -> str:
        return names.get(vid, vid)

    # Linearize
    vertex_order, critical_cuts, extra_cuts = linearize_complex(graph, terrain)

    if not vertex_order:
        return "=== Topological Narrative ===\nEmpty complex. No concepts to narrate.\n"

    # Build position index for each vertex in the linear order
    pos_index: dict[str, int] = {v: i for i, v in enumerate(vertex_order)}

    # Build cut annotations: for each critical edge, determine where it falls
    # in the narrative (between its two endpoints)
    all_cuts = critical_cuts + extra_cuts
    # Group cuts by the later endpoint (insertion point)
    cut_annotations: dict[int, list[tuple[str, str, str]]] = {}
    for src, tgt in all_cuts:
        if src == tgt:
            # Self-loop: annotate at vertex position
            idx = pos_index.get(src, 0)
            annotation_type = "self-reference"
            cut_annotations.setdefault(idx, []).append((src, tgt, annotation_type))
        else:
            src_pos = pos_index.get(src, -1)
            tgt_pos = pos_index.get(tgt, -1)
            if src_pos < 0 or tgt_pos < 0:
                continue
            # Insert at the later position
            insert_pos = max(src_pos, tgt_pos)
            annotation_type = _classify_cut(graph, src, tgt)
            cut_annotations.setdefault(insert_pos, []).append(
                (src, tgt, annotation_type)
            )

    # Compute topological metrics
    beta_1 = compute_beta_1(graph)
    n_active = len(graph.active_vertex_ids())
    n_edges = len(graph.active_edges())
    n_tree = sum(1 for m in terrain.values() if m == "tree")
    n_crit = sum(1 for m in terrain.values() if m == "critical")

    # Count edge types
    edge_type_counts: dict[str, int] = {}
    for e in graph.active_edges():
        edge_type_counts[e.edge_type.value] = (
            edge_type_counts.get(e.edge_type.value, 0) + 1
        )

    # Identify contested vertices
    contested = [
        vid for vid in vertex_order
        if graph.vertex(vid) and graph.vertex(vid).status == VertexStatus.CONTESTED
    ]

    # --- Build narrative ---
    lines: list[str] = []

    # Header
    lines.append("=" * 70)
    lines.append("TOPOLOGICAL NARRATIVE (Morse-Smale Linearization)")
    lines.append("=" * 70)
    lines.append("")
    lines.append(f"Complex: {n_active} vertices, {n_edges} edges")
    lines.append(f"beta_1 = {beta_1} (irreducible cycles)")
    lines.append(f"Terrain: {n_tree} tree edges, {n_crit} critical edges")
    lines.append(f"Critical cuts: {len(critical_cuts)} edges removed for linearization")
    if extra_cuts:
        lines.append(f"Extra cuts: {len(extra_cuts)} additional edges removed (residual cycles)")
    lines.append(f"Edge types: {edge_type_counts}")
    if contested:
        lines.append(f"Contested vertices: {len(contested)}")
    lines.append("")

    # Main narrative
    lines.append("-" * 70)
    lines.append("MAIN LINE (topological order)")
    lines.append("-" * 70)
    lines.append("")

    for i, vid in enumerate(vertex_order):
        v = graph.vertex(vid)
        status_tag = ""
        if v and v.status == VertexStatus.CONTESTED:
            status_tag = " [CONTESTED]"
        elif v and v.status == VertexStatus.FOLDED:
            status_tag = " [FOLDED]"

        # Count in/out edges in original graph
        in_edges = [e for e in graph.active_edges() if e.target == vid]
        out_edges = [e for e in graph.active_edges() if e.source == vid]

        # Find tree dependencies (edges that survived the cut)
        tree_deps = []
        for e in graph.active_edges():
            if e.target == vid and terrain.get((e.source, e.target)) == "tree":
                tree_deps.append(name(e.source))

        dep_str = ""
        if tree_deps:
            dep_str = f" <- depends on: {', '.join(tree_deps[:5])}"
            if len(tree_deps) > 5:
                dep_str += f" (+{len(tree_deps) - 5} more)"

        lines.append(f"  {i+1:3d}. {name(vid)}{status_tag}{dep_str}")

        # Insert cut annotations after this vertex
        if i in cut_annotations:
            for src, tgt, atype in cut_annotations[i]:
                if src == tgt:
                    lines.append(
                        f"        [{atype.upper()}] {name(src)} references itself"
                    )
                elif pos_index.get(src, -1) < pos_index.get(tgt, -1):
                    # Forward reference that creates cycle
                    earlier_pos = pos_index.get(src, 0) + 1
                    lines.append(
                        f"        [{atype.upper()}] {name(src)} (#{earlier_pos}) "
                        f"--[{_edge_type_label(graph, src, tgt)}]--> "
                        f"{name(tgt)} (#{i+1})"
                    )
                else:
                    # Back reference (later concept points to earlier)
                    later_pos = pos_index.get(src, 0) + 1
                    lines.append(
                        f"        [{atype.upper()}] {name(src)} (#{later_pos}) "
                        f"--[{_edge_type_label(graph, src, tgt)}]--> "
                        f"{name(tgt)} (#{pos_index.get(tgt, 0) + 1}) "
                        f"[back-reference]"
                    )

    # Summary statistics
    lines.append("")
    lines.append("-" * 70)
    lines.append("STRUCTURAL SUMMARY")
    lines.append("-" * 70)
    lines.append("")

    # Cut type distribution
    cut_type_counts: dict[str, int] = {}
    for _, _, atype in [ann for anns in cut_annotations.values() for ann in anns]:
        cut_type_counts[atype] = cut_type_counts.get(atype, 0) + 1
    if cut_type_counts:
        lines.append("Cut type distribution:")
        for ctype, count in sorted(cut_type_counts.items(), key=lambda x: -x[1]):
            lines.append(f"  {ctype}: {count}")
        lines.append("")

    # Narrative density metric
    narrative_density = len(all_cuts) / max(len(vertex_order), 1)
    lines.append(f"Narrative density: {narrative_density:.3f} "
                 f"(cuts per vertex: {len(all_cuts)}/{len(vertex_order)})")
    lines.append(f"  Interpretation: {'high dialectical tension' if narrative_density > 0.5 else 'moderate tension' if narrative_density > 0.2 else 'low tension / linear progression'}")
    lines.append("")

    # Degree distribution of narrative
    lines.append("Concept connectivity (in main line order):")
    high_degree = []
    for vid in vertex_order:
        in_count = sum(1 for e in graph.active_edges() if e.target == vid)
        out_count = sum(1 for e in graph.active_edges() if e.source == vid)
        total = in_count + out_count
        if total >= 5:
            high_degree.append((vid, in_count, out_count, total))

    if high_degree:
        lines.append("  Hub concepts (degree >= 5):")
        for vid, in_c, out_c, total in sorted(high_degree, key=lambda x: -x[3]):
            lines.append(f"    {name(vid)}: in={in_c} out={out_c} total={total}")
    else:
        lines.append("  No hub concepts (all degrees < 5)")

    lines.append("")

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Experiment runner: Hegel + Spinoza comparison
# ---------------------------------------------------------------------------

def _build_hegel_complex() -> tuple[Graph, dict[str, str]]:
    """Build the Hegel Phenomenology complex (all chapters, no traversal)."""
    from experiment_phenomenology_full import ALL_CHAPTERS
    from engine import Graph

    graph = Graph()
    for ch_fn in ALL_CHAPTERS:
        _, ch_vertices, ch_edges = ch_fn()
        existing_vids = set(graph.vertices.keys())
        for v in ch_vertices:
            if v.id not in existing_vids:
                graph = graph.add_vertex(v)
        existing_edges = {(e.source, e.target, e.edge_type) for e in graph.edges}
        for e in ch_edges:
            key = (e.source, e.target, e.edge_type)
            if key not in existing_edges:
                if e.source in graph.vertices and e.target in graph.vertices:
                    graph = graph.add_edge(e)
                    existing_edges.add(key)

    concept_names = {}
    for vid, v in graph.vertices.items():
        if v.content:
            concept_names[vid] = v.content

    return graph, concept_names


def _build_spinoza_complex() -> tuple[Graph, dict[str, str]]:
    """Build the Spinoza collected works complex (all works, no traversal)."""
    from experiment_spinoza import ALL_WORKS
    from engine import Graph

    graph = Graph()
    for w_fn in ALL_WORKS:
        _, w_vertices, w_edges = w_fn()
        existing_vids = set(graph.vertices.keys())
        for v in w_vertices:
            if v.id not in existing_vids:
                graph = graph.add_vertex(v)
        existing_edges = {(e.source, e.target, e.edge_type) for e in graph.edges}
        for e in w_edges:
            key = (e.source, e.target, e.edge_type)
            if key not in existing_edges:
                if e.source in graph.vertices and e.target in graph.vertices:
                    graph = graph.add_edge(e)
                    existing_edges.add(key)

    concept_names = {}
    for vid, v in graph.vertices.items():
        if v.content:
            concept_names[vid] = v.content

    return graph, concept_names


def run_comparison():
    """Run Morse-Smale linearization on both Hegel and Spinoza, save narratives."""
    import os
    import sys

    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)
    tmp_dir = os.path.join(os.path.dirname(script_dir), "tmp")
    os.makedirs(tmp_dir, exist_ok=True)

    print("Building Hegel Phenomenology complex...", file=sys.stderr)
    hegel_graph, hegel_names = _build_hegel_complex()
    hegel_terrain = compute_terrain(hegel_graph)
    hegel_narrative = generate_topological_narrative(
        hegel_graph, hegel_terrain, hegel_names,
    )

    hegel_path = os.path.join(tmp_dir, "hegel_topo_narrative.txt")
    with open(hegel_path, "w", encoding="utf-8") as f:
        f.write(hegel_narrative)
    print(f"Hegel narrative saved to {hegel_path}", file=sys.stderr)

    print("Building Spinoza collected works complex...", file=sys.stderr)
    spinoza_graph, spinoza_names = _build_spinoza_complex()
    spinoza_terrain = compute_terrain(spinoza_graph)
    spinoza_narrative = generate_topological_narrative(
        spinoza_graph, spinoza_terrain, spinoza_names,
    )

    spinoza_path = os.path.join(tmp_dir, "spinoza_topo_narrative.txt")
    with open(spinoza_path, "w", encoding="utf-8") as f:
        f.write(spinoza_narrative)
    print(f"Spinoza narrative saved to {spinoza_path}", file=sys.stderr)

    # Print comparison summary
    print("\n" + "=" * 70)
    print("COMPARISON: Hegel vs Spinoza topological narratives")
    print("=" * 70)

    h_verts = len(hegel_graph.active_vertex_ids())
    h_edges = len(hegel_graph.active_edges())
    h_beta = compute_beta_1(hegel_graph)
    h_order, h_cuts, h_extra = linearize_complex(hegel_graph, hegel_terrain)
    h_density = len(h_cuts + h_extra) / max(len(h_order), 1)

    s_verts = len(spinoza_graph.active_vertex_ids())
    s_edges = len(spinoza_graph.active_edges())
    s_beta = compute_beta_1(spinoza_graph)
    s_order, s_cuts, s_extra = linearize_complex(spinoza_graph, spinoza_terrain)
    s_density = len(s_cuts + s_extra) / max(len(s_order), 1)

    print(f"\n{'Metric':<30} {'Hegel':>12} {'Spinoza':>12}")
    print("-" * 56)
    print(f"{'Vertices':<30} {h_verts:>12} {s_verts:>12}")
    print(f"{'Edges':<30} {h_edges:>12} {s_edges:>12}")
    print(f"{'beta_1':<30} {h_beta:>12} {s_beta:>12}")
    print(f"{'Critical cuts':<30} {len(h_cuts):>12} {len(s_cuts):>12}")
    print(f"{'Extra cuts':<30} {len(h_extra):>12} {len(s_extra):>12}")
    print(f"{'Narrative density':<30} {h_density:>12.3f} {s_density:>12.3f}")
    print(f"{'Edge/Vertex ratio':<30} {h_edges/max(h_verts,1):>12.2f} {s_edges/max(s_verts,1):>12.2f}")

    # Edge type comparison
    h_types: dict[str, int] = {}
    for e in hegel_graph.active_edges():
        h_types[e.edge_type.value] = h_types.get(e.edge_type.value, 0) + 1
    s_types: dict[str, int] = {}
    for e in spinoza_graph.active_edges():
        s_types[e.edge_type.value] = s_types.get(e.edge_type.value, 0) + 1

    all_types = sorted(set(h_types) | set(s_types))
    print(f"\n{'Edge type':<30} {'Hegel':>12} {'Spinoza':>12}")
    print("-" * 56)
    for et in all_types:
        print(f"{'  ' + et:<30} {h_types.get(et, 0):>12} {s_types.get(et, 0):>12}")

    # Negation density comparison
    h_neg = h_types.get("negation", 0) / max(h_edges, 1)
    s_neg = s_types.get("negation", 0) / max(s_edges, 1)
    h_sub = h_types.get("sublation", 0) / max(h_edges, 1)
    s_sub = s_types.get("sublation", 0) / max(s_edges, 1)

    print(f"\n{'Negation density':<30} {h_neg:>12.4f} {s_neg:>12.4f}")
    print(f"{'Sublation density':<30} {h_sub:>12.4f} {s_sub:>12.4f}")

    # Cut type comparison
    h_cut_types: dict[str, int] = {}
    for src, tgt in h_cuts + h_extra:
        ct = _classify_cut(hegel_graph, src, tgt)
        h_cut_types[ct] = h_cut_types.get(ct, 0) + 1
    s_cut_types: dict[str, int] = {}
    for src, tgt in s_cuts + s_extra:
        ct = _classify_cut(spinoza_graph, src, tgt)
        s_cut_types[ct] = s_cut_types.get(ct, 0) + 1

    all_cut_types = sorted(set(h_cut_types) | set(s_cut_types))
    if all_cut_types:
        print(f"\n{'Cut type':<30} {'Hegel':>12} {'Spinoza':>12}")
        print("-" * 56)
        for ct in all_cut_types:
            print(f"{'  ' + ct:<30} {h_cut_types.get(ct, 0):>12} {s_cut_types.get(ct, 0):>12}")

    print(f"\nNarratives written to:")
    print(f"  {hegel_path}")
    print(f"  {spinoza_path}")

    return {
        "hegel": {
            "vertices": h_verts, "edges": h_edges, "beta_1": h_beta,
            "critical_cuts": len(h_cuts), "extra_cuts": len(h_extra),
            "narrative_density": h_density,
        },
        "spinoza": {
            "vertices": s_verts, "edges": s_edges, "beta_1": s_beta,
            "critical_cuts": len(s_cuts), "extra_cuts": len(s_extra),
            "narrative_density": s_density,
        },
    }


if __name__ == "__main__":
    run_comparison()
