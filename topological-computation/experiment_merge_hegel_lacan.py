"""Hegel + Lacan (Ecrits + Seminars) — three-complex merge experiment.

Builds three independent complexes from existing experiments:
1. Hegel's Phenomenology of Spirit (15 chapters)
2. Lacan's Ecrits (16 papers)
3. Lacan's Seminars (23 seminars)

Merges them by identifying shared vertex IDs across complexes.
Records pre/post merge topological metrics, then runs 2000 steps
of traversal on the merged complex, tracking cross-complex behavior.
"""

from __future__ import annotations

import json
import sys
import time

sys.path.insert(0, ".")

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus, compute_beta_1
from traversal import TraversalEngine
from morse import compute_terrain

# ---------------------------------------------------------------------------
# Import chapter/paper/seminar builders from existing experiments
# ---------------------------------------------------------------------------

from experiment_phenomenology_full import ALL_CHAPTERS
from experiment_lacan_ecrits import ALL_PAPERS
from experiment_lacan_seminars import ALL_SEMINARS


# ---------------------------------------------------------------------------
# Build a single graph from a list of unit-builder functions (no traversal)
# ---------------------------------------------------------------------------

def build_complex(unit_fns: list) -> tuple[Graph, set[str]]:
    """Build a graph by injecting all units. Return (graph, vertex_id_set)."""
    graph = Graph()
    all_vids: set[str] = set()
    for fn in unit_fns:
        _name, vertices, edges = fn()
        for v in vertices:
            if v.id not in graph.vertices:
                graph = graph.add_vertex(v)
                all_vids.add(v.id)
        existing_edges = {(e.source, e.target, e.edge_type) for e in graph.edges}
        for e in edges:
            key = (e.source, e.target, e.edge_type)
            if key not in existing_edges:
                if e.source in graph.vertices and e.target in graph.vertices:
                    graph = graph.add_edge(e)
                    existing_edges.add(key)
    return graph, all_vids


def graph_stats(graph: Graph) -> dict:
    """Return basic stats for a graph."""
    active_v = graph.active_vertex_ids()
    active_e = graph.active_edges()
    beta_1 = compute_beta_1(graph)
    type_counts: dict[str, int] = {}
    for e in active_e:
        type_counts[e.edge_type.value] = type_counts.get(e.edge_type.value, 0) + 1
    return {
        "vertices": len(active_v),
        "edges": len(active_e),
        "beta_1": beta_1,
        "edge_type_distribution": type_counts,
    }


# ---------------------------------------------------------------------------
# Merge logic: identify shared vertex IDs across complexes
# ---------------------------------------------------------------------------

def merge_complexes(
    graphs: list[Graph],
    vid_sets: list[set[str]],
    labels: list[str],
) -> tuple[Graph, dict]:
    """Merge multiple graphs by unifying vertices with the same ID.

    Returns (merged_graph, merge_info).
    """
    merged = Graph()
    # Track which vertices came from which complex(es)
    vertex_origin: dict[str, list[str]] = {}  # vid -> list of source labels

    # Pass 1: collect all vertices, noting origins
    for graph, vids, label in zip(graphs, vid_sets, labels):
        for vid in vids:
            v = graph.vertex(vid)
            if v is None:
                continue
            if vid not in vertex_origin:
                vertex_origin[vid] = []
                merged = merged.add_vertex(v)
            vertex_origin[vid].append(label)

    # Pass 2: collect all edges (skip duplicates)
    existing_edges: set[tuple[str, str, str]] = set()
    cross_complex_edges = 0
    for graph, _vids, label in zip(graphs, vid_sets, labels):
        for e in graph.edges:
            key = (e.source, e.target, e.edge_type.value)
            if key not in existing_edges:
                if e.source in merged.vertices and e.target in merged.vertices:
                    merged = merged.add_edge(e)
                    existing_edges.add(key)
                    # Check if this edge connects vertices from different complexes
                    src_origins = set(vertex_origin.get(e.source, []))
                    tgt_origins = set(vertex_origin.get(e.target, []))
                    if src_origins != tgt_origins and len(src_origins | tgt_origins) > 1:
                        cross_complex_edges += 1

    # Identify shared vertices (appear in 2+ complexes)
    shared_vertices = {
        vid: origins
        for vid, origins in vertex_origin.items()
        if len(origins) > 1
    }

    merge_info = {
        "shared_vertices": {
            vid: origins for vid, origins in sorted(shared_vertices.items())
        },
        "shared_vertex_count": len(shared_vertices),
        "cross_complex_edge_count": cross_complex_edges,
        "vertex_origin": {
            vid: origins for vid, origins in sorted(vertex_origin.items())
        },
    }
    return merged, merge_info


# ---------------------------------------------------------------------------
# Main experiment
# ---------------------------------------------------------------------------

def run_experiment():
    print("=" * 70)
    print("HEGEL + LACAN MERGE — THREE-COMPLEX TOPOLOGICAL EXPERIMENT")
    print("Hegel Phenomenology + Lacan Ecrits + Lacan Seminars")
    print("=" * 70)

    t0 = time.time()

    # 1. Build three independent complexes
    print("\n--- Building independent complexes ---")

    hegel_graph, hegel_vids = build_complex(ALL_CHAPTERS)
    hegel_stats = graph_stats(hegel_graph)
    print(f"  Hegel Phenomenology: {hegel_stats['vertices']} V, "
          f"{hegel_stats['edges']} E, beta_1={hegel_stats['beta_1']}")

    ecrits_graph, ecrits_vids = build_complex(ALL_PAPERS)
    ecrits_stats = graph_stats(ecrits_graph)
    print(f"  Lacan Ecrits:       {ecrits_stats['vertices']} V, "
          f"{ecrits_stats['edges']} E, beta_1={ecrits_stats['beta_1']}")

    seminars_graph, seminars_vids = build_complex(ALL_SEMINARS)
    seminars_stats = graph_stats(seminars_graph)
    print(f"  Lacan Seminars:     {seminars_stats['vertices']} V, "
          f"{seminars_stats['edges']} E, beta_1={seminars_stats['beta_1']}")

    independent_beta_1_sum = (
        hegel_stats["beta_1"]
        + ecrits_stats["beta_1"]
        + seminars_stats["beta_1"]
    )
    print(f"\n  Independent beta_1 sum: {independent_beta_1_sum}")

    # 2. Merge
    print("\n--- Merging complexes ---")
    merged_graph, merge_info = merge_complexes(
        [hegel_graph, ecrits_graph, seminars_graph],
        [hegel_vids, ecrits_vids, seminars_vids],
        ["hegel", "ecrits", "seminars"],
    )
    merged_stats = graph_stats(merged_graph)
    beta_1_delta = merged_stats["beta_1"] - independent_beta_1_sum

    print(f"  Merged complex: {merged_stats['vertices']} V, "
          f"{merged_stats['edges']} E, beta_1={merged_stats['beta_1']}")
    print(f"  beta_1 delta (merged - independent sum): {beta_1_delta}")
    print(f"  Shared vertices: {merge_info['shared_vertex_count']}")
    print(f"  Cross-complex edges: {merge_info['cross_complex_edge_count']}")

    # Print shared vertices
    print(f"\n  Shared vertex details:")
    for vid, origins in sorted(merge_info["shared_vertices"].items()):
        v = merged_graph.vertex(vid)
        content = v.content if v else "?"
        print(f"    {vid}: {origins} — \"{content}\"")

    # 3. Identify fold candidates: shared vertices with low f-value
    print("\n--- Cross-complex fold candidates (shared vertices) ---")
    active_vids = set(merged_graph.active_vertex_ids())
    fold_candidates = []
    shared_list = list(merge_info["shared_vertices"].keys())
    for i, v1 in enumerate(shared_list):
        if v1 not in active_vids:
            continue
        for v2 in shared_list[i + 1:]:
            if v2 not in active_vids:
                continue
            # Compute f(v1, v2)
            s = {v1, v2}
            n_loop = sum(
                1 for e in merged_graph.active_edges()
                if e.source in s and e.target in s
            )
            lower_link: set[str] = set()
            for x in s:
                for n in merged_graph.neighbors(x):
                    if n not in s and n in active_vids:
                        lower_link.add(n)
            if not lower_link:
                f_val = -1 + n_loop
            else:
                from engine import _connected_components
                ll_edges: list[frozenset[str]] = []
                for e in merged_graph.active_edges():
                    if e.source in lower_link and e.target in lower_link:
                        ll_edges.append(frozenset((e.source, e.target)))
                c = _connected_components(sorted(lower_link), ll_edges)
                f_val = (c - 1) + n_loop
            fold_candidates.append({
                "v1": v1,
                "v2": v2,
                "f_value": f_val,
                "origins_v1": merge_info["shared_vertices"][v1],
                "origins_v2": merge_info["shared_vertices"][v2],
            })

    fold_candidates.sort(key=lambda x: x["f_value"])
    for fc in fold_candidates[:20]:
        print(f"    f={fc['f_value']:3d}  {fc['v1']} ({fc['origins_v1']}) <-> "
              f"{fc['v2']} ({fc['origins_v2']})")

    # 4. Run 2000-step traversal on merged complex
    print("\n--- Running 2000-step traversal on merged complex ---")
    start_vertex = merged_graph.active_vertex_ids()[0]
    engine = TraversalEngine(
        merged_graph, start=start_vertex, settlement_threshold=10, seed=42,
    )

    # Track which complex each vertex belongs to for cross-complex analysis
    def vertex_complex(vid: str) -> set[str]:
        return set(merge_info["vertex_origin"].get(vid, ["unknown"]))

    total_steps = 2000
    cross_complex_ops = []  # operations that span complex boundaries
    op_counts: dict[str, int] = {}
    beta_1_snapshots = []

    for step_i in range(total_steps):
        log = engine.run_step()
        op_counts[log.operation] = op_counts.get(log.operation, 0) + 1

        # Record beta_1 at intervals
        if (step_i + 1) % 100 == 0:
            beta_1_snapshots.append({
                "step": step_i + 1,
                "beta_1": log.beta_1_after,
                "vertices_active": log.vertices_active,
                "edges_active": log.edges_active,
                "settled_count": log.settled_count,
            })

        # Detect cross-complex operations
        if log.operation not in ("walk", "nothing"):
            pos_complex = vertex_complex(log.position)
            # Check if the operation involved vertices from different complexes
            # by looking at the encounter targets in the step log
            step_logs = engine.logs
            if step_logs:
                last_log = step_logs[-1]
                # For fold/negate operations, check if position is a shared vertex
                is_cross = len(pos_complex) > 1
                if is_cross or log.delta_beta_1 != 0:
                    cross_complex_ops.append({
                        "step": log.step,
                        "operation": log.operation,
                        "position": log.position,
                        "position_complexes": sorted(pos_complex),
                        "beta_1_before": log.beta_1_before,
                        "beta_1_after": log.beta_1_after,
                        "delta_beta_1": log.delta_beta_1,
                    })

    # 5. Post-traversal analysis
    print(f"\n  Traversal complete: {total_steps} steps")
    final_beta_1 = compute_beta_1(engine.k_active)
    final_v = len(engine.k_active.active_vertex_ids())
    final_e = len(engine.k_active.active_edges())
    final_settled = len(engine.settlement.settled_cycles)

    print(f"  Final beta_1: {final_beta_1}")
    print(f"  Final complex: {final_v} V, {final_e} E")
    print(f"  Settled cycles: {final_settled}")

    print(f"\n  Operation counts:")
    for op, count in sorted(op_counts.items(), key=lambda x: -x[1]):
        print(f"    {op}: {count}")

    print(f"\n  Cross-complex operations: {len(cross_complex_ops)}")
    for cop in cross_complex_ops[:20]:
        print(f"    step={cop['step']:4d} {cop['operation']:15s} "
              f"at {cop['position']} ({cop['position_complexes']}) "
              f"delta_beta_1={cop['delta_beta_1']}")
    if len(cross_complex_ops) > 20:
        print(f"    ... and {len(cross_complex_ops) - 20} more")

    print(f"\n  beta_1 evolution (every 100 steps):")
    for snap in beta_1_snapshots:
        bar = "#" * min(snap["beta_1"], 80)
        print(f"    step={snap['step']:4d}: beta_1={snap['beta_1']:4d} "
              f"V={snap['vertices_active']} E={snap['edges_active']} "
              f"settled={snap['settled_count']} {bar}")

    # Cross-complex tension pairs: shared vertices with high topological tension
    print(f"\n  Cross-complex tension analysis:")
    final_terrain = compute_terrain(engine.k_active)
    tension_pairs = []
    shared_vids_active = [
        vid for vid in merge_info["shared_vertices"]
        if vid in set(engine.k_active.active_vertex_ids())
    ]
    for vid in shared_vids_active:
        crit_count = 0
        for nb in engine.k_active.neighbors(vid):
            fwd = final_terrain.get((vid, nb), "critical")
            rev = final_terrain.get((nb, vid), "critical")
            if fwd == "critical" or rev == "critical":
                crit_count += 1
        if crit_count > 0:
            tension_pairs.append({
                "vertex": vid,
                "complexes": merge_info["shared_vertices"][vid],
                "critical_neighbor_count": crit_count,
            })
    tension_pairs.sort(key=lambda x: -x["critical_neighbor_count"])
    for tp in tension_pairs[:15]:
        print(f"    {tp['vertex']} ({tp['complexes']}): "
              f"{tp['critical_neighbor_count']} critical neighbors")

    # Final terrain stats
    ft_counts: dict[str, int] = {"tree": 0, "critical": 0}
    for mark in final_terrain.values():
        ft_counts[mark] = ft_counts.get(mark, 0) + 1
    crit_ratio = ft_counts["critical"] / (ft_counts["tree"] + ft_counts["critical"]) \
        if (ft_counts["tree"] + ft_counts["critical"]) > 0 else 0

    # Edge type distribution after traversal
    final_type_counts: dict[str, int] = {}
    for e in engine.k_active.active_edges():
        final_type_counts[e.edge_type.value] = \
            final_type_counts.get(e.edge_type.value, 0) + 1

    elapsed = time.time() - t0
    print(f"\n  Elapsed: {elapsed:.1f}s")

    # ---------------------------------------------------------------------------
    # Save results
    # ---------------------------------------------------------------------------
    output = {
        "experiment": "merge_hegel_lacan",
        "description": "Three-complex merge: Hegel Phenomenology + Lacan Ecrits + Lacan Seminars",
        "pre_merge": {
            "hegel_phenomenology": hegel_stats,
            "lacan_ecrits": ecrits_stats,
            "lacan_seminars": seminars_stats,
            "independent_beta_1_sum": independent_beta_1_sum,
        },
        "merge_info": {
            "shared_vertices": merge_info["shared_vertices"],
            "shared_vertex_count": merge_info["shared_vertex_count"],
            "cross_complex_edge_count": merge_info["cross_complex_edge_count"],
        },
        "post_merge": {
            "vertices": merged_stats["vertices"],
            "edges": merged_stats["edges"],
            "beta_1": merged_stats["beta_1"],
            "beta_1_delta_vs_independent": beta_1_delta,
            "edge_type_distribution": merged_stats["edge_type_distribution"],
        },
        "fold_candidates": fold_candidates[:30],
        "traversal": {
            "total_steps": total_steps,
            "final_beta_1": final_beta_1,
            "final_vertices_active": final_v,
            "final_edges_active": final_e,
            "final_settled_cycles": final_settled,
            "operation_counts": op_counts,
            "beta_1_snapshots": beta_1_snapshots,
            "cross_complex_operations": cross_complex_ops,
            "cross_complex_tension_pairs": tension_pairs[:30],
            "final_terrain_distribution": ft_counts,
            "critical_edge_ratio": crit_ratio,
            "final_edge_type_distribution": final_type_counts,
        },
        "settled_cycles_detail": [
            {"edges": sorted(sc.edges), "settled_at_step": sc.settled_at_step}
            for sc in engine.settlement.settled_cycles
        ],
        "elapsed_seconds": elapsed,
    }

    output_path = "experiment_merge_hegel_lacan.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
