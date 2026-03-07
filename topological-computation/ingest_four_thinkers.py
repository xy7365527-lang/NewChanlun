#!/usr/bin/env python3
"""Ingest Derrida, Deleuze, Nietzsche, Foucault into k_merged.json.

Loads the hand-crafted conceptual graphs from experiment_derrida.py,
experiment_deleuze.py, experiment_nietzsche.py, experiment_foucault.py,
and merges them into the existing k_merged.json (same path and format
used by ingest_all_and_traverse.py for the 19+ thinkers).

Unlike the experiment scripts which run traversal in memory, this script
*persists* the graph to k_merged.json.

Usage:
    cd topological-computation
    python ingest_four_thinkers.py
    python ingest_four_thinkers.py --dry-run          # show stats without writing
    python ingest_four_thinkers.py --traverse          # run daemon traversal after merge
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus, compute_beta_1
from daemon import graph_to_dict, graph_from_dict

# Import all four thinkers' work collections
from experiment_derrida import ALL_WORKS as DERRIDA_WORKS
from experiment_deleuze import ALL_WORKS as DELEUZE_WORKS
from experiment_nietzsche import ALL_WORKS as NIETZSCHE_WORKS
from experiment_foucault import ALL_WORKS as FOUCAULT_WORKS


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

SWARM_DIR = Path.home() / ".swarm"
MERGED_PATH = SWARM_DIR / "k_merged.json"


# ---------------------------------------------------------------------------
# Graph merge (content-based dedup, same pattern as ingest_chanlun_blog.py)
# ---------------------------------------------------------------------------

def merge_graphs(base: Graph, addition: Graph) -> tuple[Graph, int, int]:
    """Merge addition into base, skipping duplicate vertex IDs.

    Returns (merged_graph, new_vertices_count, new_edges_count).
    """
    existing_ids = set(base.active_vertex_ids())
    # Include all vertices (not just active) to avoid re-adding folded ones
    all_ids = set(base.vertices.keys())
    existing_edges: set[tuple[str, str, str]] = {
        (e.source, e.target, e.edge_type.value)
        for e in base.edges
    }

    new_v = 0
    new_e = 0

    for vid, v in addition.vertices.items():
        if vid not in all_ids:
            base = base.add_vertex(v)
            all_ids.add(vid)
            existing_ids.add(vid)
            new_v += 1

    for e in addition.edges:
        key = (e.source, e.target, e.edge_type.value)
        if key not in existing_edges:
            if e.source in all_ids and e.target in all_ids:
                base = base.add_edge(e)
                existing_edges.add(key)
                new_e += 1

    return base, new_v, new_e


# ---------------------------------------------------------------------------
# Build graph from work functions (no traversal, just topology)
# ---------------------------------------------------------------------------

def build_graph_from_works(works: list, label: str) -> Graph:
    """Build a Graph from a list of work functions, injecting vertices
    and edges incrementally (same as the experiment runner, but without
    traversal).
    """
    graph = Graph()

    for w_fn in works:
        work_name, w_vertices, w_edges = w_fn()

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

    return graph


# ---------------------------------------------------------------------------
# Cross-thinker edges
# ---------------------------------------------------------------------------

def inject_cross_thinker_edges(graph: Graph) -> tuple[Graph, int]:
    """Add edges connecting concepts across the four thinkers.

    These are philosophically significant connections — not arbitrary.
    Only added if both endpoints exist in the graph.
    """
    cross_edges = [
        # Nietzsche -> Foucault: Foucault's genealogy is explicitly Nietzschean
        Edge("foucault_genealogy", "nietzsche_ressentiment", EdgeType.REFERENCE),
        Edge("foucault_power", "nietzsche_will_to_power", EdgeType.REFERENCE),
        Edge("foucault_death_of_man", "nietzsche_death_of_god", EdgeType.REFERENCE),

        # Nietzsche -> Deleuze: Deleuze's Nietzsche interpretation
        Edge("deleuze_difference", "nietzsche_eternal_recurrence", EdgeType.REFERENCE),
        Edge("deleuze_eternal_return", "nietzsche_eternal_recurrence", EdgeType.DEPENDENCY),
        Edge("deleuze_simulacrum", "nietzsche_perspectivism", EdgeType.REFERENCE),
        Edge("deleuze_identity_critique", "nietzsche_will_to_power", EdgeType.REFERENCE),

        # Nietzsche -> Derrida: Derrida's reading of Nietzsche
        Edge("derrida_freeplay", "nietzsche_perspectivism", EdgeType.REFERENCE),
        Edge("derrida_differance", "nietzsche_eternal_recurrence", EdgeType.REFERENCE),

        # Foucault -> Derrida: the Cogito debate
        Edge("derrida_cogito_madness", "foucault_reason_unreason", EdgeType.NEGATION),
        Edge("derrida_cogito_madness", "foucault_madness", EdgeType.REFERENCE),

        # Deleuze -> Foucault: mutual influence
        Edge("deleuze_desire_production", "foucault_power", EdgeType.REFERENCE),
        Edge("deleuze_assemblage", "foucault_discourse", EdgeType.REFERENCE),
        Edge("deleuze_smooth_space", "foucault_discipline", EdgeType.NEGATION),
        Edge("deleuze_war_machine", "foucault_state_apparatus", EdgeType.REFERENCE),
        Edge("foucault_biopolitics", "deleuze_capitalism_flows", EdgeType.REFERENCE),

        # Deleuze -> Derrida: philosophical proximity and distance
        Edge("deleuze_difference", "derrida_differance", EdgeType.REFERENCE),
        Edge("deleuze_virtual", "derrida_trace", EdgeType.REFERENCE),
        Edge("deleuze_sense", "derrida_dissemination", EdgeType.REFERENCE),
        Edge("deleuze_event", "derrida_event", EdgeType.REFERENCE),
        Edge("deleuze_repetition", "derrida_iterability", EdgeType.REFERENCE),

        # Foucault -> Deleuze: the dispositif and the assemblage
        Edge("foucault_discursive_formation", "deleuze_assemblage", EdgeType.REFERENCE),
        Edge("foucault_surveillance", "deleuze_striated_space", EdgeType.REFERENCE),
        Edge("foucault_resistance", "deleuze_line_of_flight", EdgeType.REFERENCE),

        # Derrida -> Foucault: deconstruction and archaeology
        Edge("derrida_logocentrism", "foucault_episteme", EdgeType.REFERENCE),
        Edge("derrida_absence", "foucault_silence_madness", EdgeType.REFERENCE),
    ]

    existing_vids = set(graph.vertices.keys())
    existing_edges: set[tuple[str, str, str]] = {
        (e.source, e.target, e.edge_type.value)
        for e in graph.edges
    }

    count = 0
    for e in cross_edges:
        key = (e.source, e.target, e.edge_type.value)
        if key not in existing_edges:
            if e.source in existing_vids and e.target in existing_vids:
                graph = graph.add_edge(e)
                existing_edges.add(key)
                count += 1

    return graph, count


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Ingest Derrida/Deleuze/Nietzsche/Foucault into k_merged.json"
    )
    parser.add_argument(
        "--merged-path",
        default=str(MERGED_PATH),
        help=f"k_merged.json path (default: {MERGED_PATH})",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show stats without writing to k_merged.json",
    )
    parser.add_argument(
        "--traverse",
        action="store_true",
        help="Run daemon traversal after merge",
    )

    args = parser.parse_args()
    merged_path = Path(args.merged_path)

    t0 = time.time()

    print("=" * 70)
    print("INGEST FOUR THINKERS INTO k_merged.json")
    print("  Derrida, Deleuze, Nietzsche, Foucault")
    print("=" * 70)

    # Load existing merged graph
    if merged_path.exists():
        print(f"\nLoading existing k_merged.json from {merged_path}...")
        data = json.loads(merged_path.read_text(encoding="utf-8"))
        merged = graph_from_dict(data)
        print(f"  Existing: {len(merged.active_vertex_ids())}V, "
              f"{len(merged.active_edges())}E, "
              f"beta_1={compute_beta_1(merged)}")
    else:
        print("\nNo existing k_merged.json found, starting fresh.")
        merged = Graph()

    # Build graphs for each thinker
    thinkers = [
        ("Derrida", DERRIDA_WORKS),
        ("Deleuze", DELEUZE_WORKS),
        ("Nietzsche", NIETZSCHE_WORKS),
        ("Foucault", FOUCAULT_WORKS),
    ]

    total_new_v = 0
    total_new_e = 0

    for name, works in thinkers:
        print(f"\n{'─' * 60}")
        print(f"  {name}: {len(works)} works")
        print(f"{'─' * 60}")

        sub_graph = build_graph_from_works(works, name)
        sub_v = len(sub_graph.active_vertex_ids())
        sub_e = len(sub_graph.active_edges())
        sub_beta = compute_beta_1(sub_graph)
        print(f"  Sub-graph: {sub_v}V, {sub_e}E, beta_1={sub_beta}")

        merged, new_v, new_e = merge_graphs(merged, sub_graph)
        total_new_v += new_v
        total_new_e += new_e
        print(f"  Merged: +{new_v}V, +{new_e}E -> "
              f"{len(merged.active_vertex_ids())}V, {len(merged.active_edges())}E")

    # Cross-thinker edges
    print(f"\n{'─' * 60}")
    print(f"  Cross-thinker edges")
    print(f"{'─' * 60}")
    merged, cross_count = inject_cross_thinker_edges(merged)
    total_new_e += cross_count
    print(f"  Added: {cross_count} cross-thinker edges")

    # Summary
    final_v = len(merged.active_vertex_ids())
    final_e = len(merged.active_edges())
    final_beta = compute_beta_1(merged)
    elapsed = time.time() - t0

    print(f"\n{'=' * 70}")
    print(f"  INGESTION SUMMARY")
    print(f"{'=' * 70}")
    print(f"  New vertices:    +{total_new_v}")
    print(f"  New edges:       +{total_new_e}")
    print(f"  Final graph:     {final_v}V, {final_e}E")
    print(f"  beta_1:          {final_beta}")
    print(f"  Elapsed:         {elapsed:.1f}s")

    if args.dry_run:
        print(f"\n  --dry-run: not writing to {merged_path}")
        return

    # Save
    SWARM_DIR.mkdir(parents=True, exist_ok=True)
    merged_path.write_text(
        json.dumps(graph_to_dict(merged), ensure_ascii=False),
        encoding="utf-8",
    )
    size_mb = merged_path.stat().st_size / 1024 / 1024
    print(f"  Saved to: {merged_path} ({size_mb:.1f} MB)")

    # Save report
    report = {
        "source": "four_thinkers_ingest",
        "thinkers": ["Derrida", "Deleuze", "Nietzsche", "Foucault"],
        "new_vertices": total_new_v,
        "new_edges": total_new_e,
        "cross_thinker_edges": cross_count,
        "final_vertices": final_v,
        "final_edges": final_e,
        "final_beta_1": final_beta,
        "elapsed_seconds": round(elapsed, 2),
    }
    report_path = SWARM_DIR / "output" / "four_thinkers_ingest_report.json"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(
        json.dumps(report, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )
    print(f"  Report: {report_path}")

    # Optional: run daemon traversal after merge
    if args.traverse:
        print(f"\n{'=' * 70}")
        print(f"  Running daemon traversal...")
        print(f"{'=' * 70}")

        from daemon import TopologicalDaemon
        persist_path = SWARM_DIR / "k_merged_full.jsonl"
        daemon = TopologicalDaemon(
            graph=merged,
            settlement_threshold=15,
            persist_path=str(persist_path),
        )

        t1 = time.time()
        while daemon._crystallization_count < 1:
            daemon.run(max_steps=100)
            if len(daemon.k_active.active_vertex_ids()) < 3:
                break

        elapsed2 = time.time() - t1
        post_beta = compute_beta_1(daemon.k_active)

        print(f"  Steps:       {daemon.total_steps}")
        print(f"  Time:        {elapsed2:.1f}s")
        print(f"  beta_1:      {final_beta} -> {post_beta}")
        print(f"  Events:      {daemon.total_events}")
        print(f"  Crystallized: {daemon._crystallization_count}")


if __name__ == "__main__":
    main()
