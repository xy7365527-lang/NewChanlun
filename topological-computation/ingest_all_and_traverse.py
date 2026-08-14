#!/usr/bin/env python3
"""Merge all ingested topologies and run daemon traversal.

Loads:
1. Existing K_active from ~/.swarm/ (genealogy + 19 thinkers)
2. DeepSeek-V3 code topology
3. DeepSeek-R1 paper topology (if available)
4. NewChanlun code topology
5. topological-computation self topology
6. DeepSeek papers text topology

Then: generate cross-domain edges, run daemon traversal, report.

Usage:
    python ingest_all_and_traverse.py
    python ingest_all_and_traverse.py --max-crystallizations 3
    python ingest_all_and_traverse.py --skip-ingest  # use cached graphs
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
from daemon import TopologicalDaemon, graph_to_dict, graph_from_dict
from code_ingest import ingest_tree
from block_topology_persistence import load_graph_from_block_topology, DAEMON_BT_BASE


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

SWARM_DIR = Path.home() / ".swarm"
CACHE_DIR = SWARM_DIR / "ingest_cache"
MERGED_PATH = SWARM_DIR / "k_merged.json"

SOURCES = {
    "deepseek-v3": Path.home() / "feeds" / "deepseek-v3",
    "deepseek-r1": Path.home() / "feeds" / "deepseek-r1",
    "newchanlun": Path.home() / "NewChanlun",
    "topo-self": Path.home() / "NewChanlun" / "topological-computation",
}

EXCLUDE_DIRS = {
    "__pycache__", ".venv", "venv", "env", "node_modules", ".git",
    ".tox", ".mypy_cache", ".pytest_cache", "dist", "build",
    "site-packages", ".chanlun",
}


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _print_section(title: str) -> None:
    print(f"\n{'='*60}")
    print(f"  {title}")
    print(f"{'='*60}")


def merge_graphs(base: Graph, addition: Graph) -> Graph:
    """Merge addition into base, skipping duplicate vertex IDs."""
    existing_ids = set(base.active_vertex_ids())
    existing_edges: set[tuple[str, str, str]] = {
        (e.source, e.target, e.edge_type.value)
        for e in base.edges
    }

    for vid in addition.active_vertex_ids():
        if vid not in existing_ids:
            v = addition.vertex(vid)
            base = base.add_vertex(v)
            existing_ids.add(vid)

    for e in addition.edges:
        key = (e.source, e.target, e.edge_type.value)
        if key not in existing_edges:
            if e.source in existing_ids and e.target in existing_ids:
                base = base.add_edge(e)
                existing_edges.add(key)

    return base


def save_graph_cache(graph: Graph, name: str) -> None:
    """Cache an ingested graph to JSON."""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    path = CACHE_DIR / f"{name}.json"
    data = graph_to_dict(graph)
    path.write_text(json.dumps(data, ensure_ascii=False), encoding="utf-8")
    print(f"  [cache] Saved {name}: {len(graph.active_vertex_ids())}V, {len(graph.active_edges())}E")


def load_graph_cache(name: str) -> Graph | None:
    """Load a cached graph if available."""
    path = CACHE_DIR / f"{name}.json"
    if not path.exists():
        return None
    data = json.loads(path.read_text(encoding="utf-8"))
    return graph_from_dict(data)


# ---------------------------------------------------------------------------
# Ingest functions
# ---------------------------------------------------------------------------

def ingest_code_source(source_name: str, source_path: Path, skip_ingest: bool) -> Graph:
    """Ingest a code source, using cache if available and skip_ingest=True."""
    if skip_ingest:
        cached = load_graph_cache(f"code_{source_name}")
        if cached:
            print(f"  [cache] Loaded {source_name}: "
                  f"{len(cached.active_vertex_ids())}V, {len(cached.active_edges())}E")
            return cached

    if not source_path.exists():
        print(f"  [skip] {source_name}: path not found ({source_path})")
        return Graph()

    print(f"  Ingesting {source_name} from {source_path}...")
    t0 = time.time()
    graph = ingest_tree(str(source_path), source=source_name)
    elapsed = time.time() - t0
    n_v = len(graph.active_vertex_ids())
    n_e = len(graph.active_edges())
    print(f"  [OK] {source_name}: {n_v}V, {n_e}E in {elapsed:.1f}s")

    save_graph_cache(graph, f"code_{source_name}")
    return graph


def load_existing_k_active() -> Graph:
    """Load existing K_active from JSONL/snapshot first, then bounded legacy blocks."""
    persist_path = SWARM_DIR / "k_full.jsonl"
    bt_graph, _ = load_graph_from_block_topology(
        DAEMON_BT_BASE, jsonl_path=persist_path,
    )
    bt_vids = bt_graph.active_vertex_ids()
    if bt_vids:
        n_v = len(bt_vids)
        n_e = len(bt_graph.active_edges())
        print(f"  [OK] Existing K_active: {n_v}V, {n_e}E")
        return bt_graph
    return Graph()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description="Merge all topologies and traverse")
    parser.add_argument("--max-crystallizations", type=int, default=1,
                        help="Stop after all regions crystallize this many times (default: 1)")
    parser.add_argument("--skip-ingest", action="store_true",
                        help="Use cached ingested graphs (skip re-parsing)")
    parser.add_argument("--no-traverse", action="store_true",
                        help="Only merge, don't run daemon")
    args = parser.parse_args()

    # -----------------------------------------------------------------------
    # Phase 1: Load existing K_active
    # -----------------------------------------------------------------------
    _print_section("Phase 1: Load existing K_active")
    merged = load_existing_k_active()
    total_v = len(merged.active_vertex_ids())
    total_e = len(merged.active_edges())

    # -----------------------------------------------------------------------
    # Phase 2: Ingest all code sources
    # -----------------------------------------------------------------------
    _print_section("Phase 2: Ingest code sources")
    for name, path in SOURCES.items():
        sub = ingest_code_source(name, path, args.skip_ingest)
        if sub.active_vertex_ids():
            pre_v = len(merged.active_vertex_ids())
            merged = merge_graphs(merged, sub)
            post_v = len(merged.active_vertex_ids())
            print(f"  [merge] +{post_v - pre_v} vertices from {name}")

    # -----------------------------------------------------------------------
    # Phase 3: Load paper topologies (if cached by paper-worker)
    # -----------------------------------------------------------------------
    _print_section("Phase 3: Load paper topologies")
    for paper_cache in sorted(CACHE_DIR.glob("paper_*.json")) if CACHE_DIR.exists() else []:
        name = paper_cache.stem
        data = json.loads(paper_cache.read_text(encoding="utf-8"))
        paper_graph = graph_from_dict(data)
        if paper_graph.active_vertex_ids():
            pre_v = len(merged.active_vertex_ids())
            merged = merge_graphs(merged, paper_graph)
            post_v = len(merged.active_vertex_ids())
            print(f"  [merge] +{post_v - pre_v} vertices from {name}")

    # -----------------------------------------------------------------------
    # Phase 4: Cross-domain edges
    # -----------------------------------------------------------------------
    _print_section("Phase 4: Cross-domain edge detection")
    try:
        from cross_domain import inject_cross_domain_edges
        pre_e = len(merged.active_edges())
        merged = inject_cross_domain_edges(merged, min_score=0.12, max_edges=1000)
        post_e = len(merged.active_edges())
        print(f"  [OK] +{post_e - pre_e} cross-domain edges injected")
    except ImportError:
        print("  [skip] cross_domain.py not available yet")
    except Exception as exc:
        print(f"  [warn] cross-domain edge detection failed: {exc}")

    # -----------------------------------------------------------------------
    # Summary
    # -----------------------------------------------------------------------
    final_v = len(merged.active_vertex_ids())
    final_e = len(merged.active_edges())
    b1 = compute_beta_1(merged)

    _print_section("Merged K_active Summary")
    print(f"  Vertices:  {final_v}")
    print(f"  Edges:     {final_e}")
    print(f"  beta_1:    {b1}")

    # Save merged graph
    MERGED_PATH.write_text(
        json.dumps(graph_to_dict(merged), ensure_ascii=False),
        encoding="utf-8",
    )
    print(f"  Saved to:  {MERGED_PATH}")
    print(f"  Size:      {MERGED_PATH.stat().st_size / 1024 / 1024:.1f} MB")

    if args.no_traverse:
        print("\n  --no-traverse specified, stopping here.")
        return

    # -----------------------------------------------------------------------
    # Phase 5: Daemon traversal (crystallization-driven exit)
    # -----------------------------------------------------------------------
    _print_section(f"Phase 5: Daemon traversal (until {args.max_crystallizations} crystallization(s))")

    persist_path = SWARM_DIR / "k_merged_full.jsonl"
    daemon = TopologicalDaemon(
        graph=merged,
        settlement_threshold=15,
        persist_path=str(persist_path),
    )

    target_crystallizations = args.max_crystallizations
    t0 = time.time()

    # Run in step batches, checking crystallization count
    while daemon._crystallization_count < target_crystallizations:
        daemon.run(max_steps=100)
        # Safety: if graph is trivially small, avoid infinite loop
        if len(daemon.k_active.active_vertex_ids()) < 3:
            break

    elapsed = time.time() - t0

    final_b1 = compute_beta_1(daemon.k_active)
    final_v2 = len(daemon.k_active.active_vertex_ids())
    final_e2 = len(daemon.k_active.active_edges())

    _print_section("Traversal Results")
    print(f"  Steps:          {daemon.total_steps}")
    print(f"  Time:           {elapsed:.1f}s")
    print(f"  K_active:       {final_v2}V, {final_e2}E")
    print(f"  beta_1:         {b1} -> {final_b1} (delta={final_b1 - b1})")
    print(f"  Events:         {daemon.total_events}")
    print(f"  Gaps detected:  {daemon.total_gaps_detected}")
    print(f"  Feeds:          {daemon.total_feeds}")
    print(f"  Crystallized:   {daemon._crystallization_count}")

    # Show last 20 events
    if daemon.event_log:
        print(f"\n  --- Last 20 events ---")
        for line in daemon.event_log[-20:]:
            print(f"  {line}")

    # Save report
    report = {
        "max_crystallizations": args.max_crystallizations,
        "actual_crystallizations": daemon._crystallization_count,
        "steps": daemon.total_steps,
        "time_seconds": round(elapsed, 1),
        "merged_vertices": final_v,
        "merged_edges": final_e,
        "merged_beta_1": b1,
        "final_vertices": final_v2,
        "final_edges": final_e2,
        "final_beta_1": final_b1,
        "total_events": daemon.total_events,
        "total_gaps": daemon.total_gaps_detected,
        "total_feeds": daemon.total_feeds,
        "last_events": daemon.event_log[-20:] if daemon.event_log else [],
    }
    report_path = SWARM_DIR / "output" / "ingest_traverse_report.json"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"\n  Report: {report_path}")


if __name__ == "__main__":
    main()
