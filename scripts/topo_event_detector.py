"""Topology event detector -- read-only analysis of block-topology relations.

Detects:
1. Cycles in the relation graph (new cycle formation)
2. Cycle rank changes (before/after comparison)
3. New connected components

Usage:
    python scripts/topo_event_detector.py --last-n 10
    python scripts/topo_event_detector.py --since <commit_hash>
    python scripts/topo_event_detector.py --last-n 10 --base .chanlun/block-topology
    python scripts/topo_event_detector.py --last-n 10 --layer 1
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

# Import LAYER_MAP from block_topology if available, else inline fallback
try:
    from scripts.block_topology import LAYER_MAP
except ImportError:
    LAYER_MAP: dict[str, int] = {
        "depends_on": 1, "negates": 1, "negated_by": 1, "supersedes": 1,
        "residue_of": 1, "reopens": 1, "tensions_with": 1,
        "freezes": 1, "splits": 1, "severs": 1,
        "references": 2, "defines": 2, "modifies": 2,
        "refines": 2, "revises": 2, "annotates": 2,
        "records": 3, "related": 3,
    }

DEFAULT_BASE = Path(".chanlun/block-topology")


# ---------------------------------------------------------------------------
# Relation parsing (handles heterogeneous schemas in relations.jsonl)
# ---------------------------------------------------------------------------

def parse_relation(rec: dict[str, Any]) -> tuple[str, str, str]:
    """Normalize a relation record to (source, target, relation_type).

    Handles multiple schema variants:
    - Old: {from, to, relation, ...}
    - New: {from, to, type, ...}
    - source/target keys alongside from/to
    """
    src = str(rec.get("from", ""))
    tgt = str(rec.get("to", ""))
    rtype = rec.get("relation") or rec.get("type") or "unknown"
    return src, tgt, str(rtype)


# ---------------------------------------------------------------------------
# Loading
# ---------------------------------------------------------------------------

def load_relations(
    path: Path,
    last_n: int | None = None,
) -> list[tuple[str, str, str]]:
    """Load relations from a jsonl file, optionally taking only last N lines."""
    if not path.exists():
        return []
    lines = path.read_text(encoding="utf-8").splitlines()
    if last_n is not None and last_n > 0:
        lines = lines[-last_n:]
    result: list[tuple[str, str, str]] = []
    for line in lines:
        line = line.strip()
        if not line:
            continue
        rec = json.loads(line)
        result.append(parse_relation(rec))
    return result


def load_relations_by_layer(
    path: Path,
    layer: int,
    last_n: int | None = None,
) -> list[tuple[str, str, str]]:
    """Load relations filtered to a specific layer."""
    all_rels = load_relations(path, last_n=last_n)
    return [
        (s, t, rt) for s, t, rt in all_rels
        if LAYER_MAP.get(rt, 0) == layer
    ]


# ---------------------------------------------------------------------------
# Graph construction
# ---------------------------------------------------------------------------

def build_graph(
    rels: list[tuple[str, str, str]],
) -> dict[str, set[str]]:
    """Build directed adjacency dict from relation tuples."""
    g: dict[str, set[str]] = defaultdict(set)
    for src, tgt, _ in rels:
        g[src].add(tgt)
        if tgt not in g:
            g[tgt] = g[tgt]  # ensure key exists
    return dict(g)


# ---------------------------------------------------------------------------
# Cycle detection (Johnson's simplified -- find all elementary cycles)
# ---------------------------------------------------------------------------

def detect_cycles(graph: dict[str, set[str]]) -> list[list[str]]:
    """Find all elementary cycles in a directed graph.

    Uses iterative DFS with coloring. Returns list of cycles,
    each cycle as a list of node ids in traversal order.
    For large graphs, limits to first 100 cycles to avoid explosion.
    """
    MAX_CYCLES = 100
    cycles: list[list[str]] = []
    nodes = sorted(graph.keys())

    for start in nodes:
        if len(cycles) >= MAX_CYCLES:
            break
        visited: set[str] = set()
        stack: list[tuple[str, list[str]]] = [(start, [start])]
        while stack and len(cycles) < MAX_CYCLES:
            node, path = stack.pop()
            for neighbor in sorted(graph.get(node, set())):
                if neighbor == start and len(path) >= 1:
                    cycle = _normalize_cycle(path)
                    if cycle not in cycles:
                        cycles.append(cycle)
                elif neighbor not in visited and neighbor > start:
                    visited.add(neighbor)
                    stack.append((neighbor, path + [neighbor]))

    # Also detect self-loops explicitly
    for node in nodes:
        if node in graph.get(node, set()):
            sl = [node]
            if sl not in cycles:
                cycles.append(sl)

    return cycles


def _normalize_cycle(path: list[str]) -> list[str]:
    """Normalize cycle representation: rotate so min element is first."""
    if not path:
        return path
    min_idx = path.index(min(path))
    return path[min_idx:] + path[:min_idx]


# ---------------------------------------------------------------------------
# Cycle rank
# ---------------------------------------------------------------------------

def _cycle_rank(graph: dict[str, set[str]]) -> int:
    """Compute cycle rank = |E| - |V| + number_of_weakly_connected_components.

    This is a standard graph-theoretic measure of "cyclicity".
    """
    num_edges = sum(len(nbrs) for nbrs in graph.values())
    num_vertices = len(graph)
    num_components = len(_weakly_connected_components(graph))
    return num_edges - num_vertices + num_components


def diff_cycle_rank(
    old_graph: dict[str, set[str]],
    new_graph: dict[str, set[str]],
) -> tuple[int, int, int]:
    """Compare cycle_rank between two graph states.

    Returns (old_rank, new_rank, delta).
    """
    old_rank = _cycle_rank(old_graph)
    new_rank = _cycle_rank(new_graph)
    return old_rank, new_rank, new_rank - old_rank


# ---------------------------------------------------------------------------
# Connected components (weakly connected, treating directed as undirected)
# ---------------------------------------------------------------------------

def _weakly_connected_components(
    graph: dict[str, set[str]],
) -> list[set[str]]:
    """Find weakly connected components (ignoring edge direction)."""
    if not graph:
        return []
    # Build undirected adjacency
    undirected: dict[str, set[str]] = defaultdict(set)
    for node, nbrs in graph.items():
        undirected[node]  # ensure present
        for nbr in nbrs:
            undirected[node].add(nbr)
            undirected[nbr].add(node)

    visited: set[str] = set()
    components: list[set[str]] = []
    for node in undirected:
        if node in visited:
            continue
        component: set[str] = set()
        queue = [node]
        while queue:
            current = queue.pop()
            if current in visited:
                continue
            visited.add(current)
            component.add(current)
            for nbr in undirected[current]:
                if nbr not in visited:
                    queue.append(nbr)
        components.append(component)
    return components


def detect_new_components(
    old_graph: dict[str, set[str]],
    new_graph: dict[str, set[str]],
) -> list[list[str]]:
    """Find components in new_graph whose nodes are entirely absent from old_graph.

    A "new component" = a weakly connected component where EVERY node
    is absent from old_graph. Merging existing components is not reported.
    """
    old_nodes = set(old_graph.keys())
    new_comps = _weakly_connected_components(new_graph)
    result: list[list[str]] = []
    for comp in new_comps:
        if comp.isdisjoint(old_nodes):
            result.append(sorted(comp))
    return result


# ---------------------------------------------------------------------------
# Event assembly
# ---------------------------------------------------------------------------

def _assemble_events(
    cycles: list[list[str]],
    rank_delta: tuple[int, int, int],
    new_components: list[list[str]],
) -> list[dict[str, Any]]:
    """Assemble topology events into a list of event dicts."""
    events: list[dict[str, Any]] = []

    for cycle in cycles:
        events.append({
            "type": "cycle_detected",
            "nodes": cycle,
            "length": len(cycle),
        })

    old_rank, new_rank, delta = rank_delta
    if delta != 0:
        events.append({
            "type": "cycle_rank_change",
            "old_rank": old_rank,
            "new_rank": new_rank,
            "delta": delta,
        })

    for comp in new_components:
        events.append({
            "type": "new_component",
            "nodes": comp,
            "size": len(comp),
        })

    return events


# ---------------------------------------------------------------------------
# Git-based relation slicing (--since <commit>)
# ---------------------------------------------------------------------------

def _count_relations_at_commit(
    commit: str, rel_path: str,
) -> int:
    """Count lines in relations.jsonl at a given commit."""
    try:
        result = subprocess.run(
            ["git", "show", f"{commit}:{rel_path}"],
            capture_output=True, text=True, check=True,
        )
        return len(result.stdout.strip().splitlines())
    except subprocess.CalledProcessError:
        return 0


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _build_parser() -> argparse.ArgumentParser:
    """Build the CLI argument parser."""
    parser = argparse.ArgumentParser(
        description="Detect topology events in block-topology relations.",
    )
    parser.add_argument(
        "--since", type=str, default=None,
        help="Git commit hash: analyze relations added after this commit.",
    )
    parser.add_argument(
        "--last-n", type=int, default=None,
        help="Analyze the last N relations only.",
    )
    parser.add_argument(
        "--base", type=str, default=str(DEFAULT_BASE),
        help="Base directory for block-topology data.",
    )
    parser.add_argument(
        "--layer", type=int, default=None,
        help="Filter relations to a specific layer (1/2/3).",
    )
    return parser


def _split_relations(
    all_rels: list[tuple[str, str, str]],
    rel_path: Path,
    since: str | None,
    last_n: int | None,
) -> tuple[list[tuple[str, str, str]], list[tuple[str, str, str]]]:
    """Split relations into old/new based on --since or --last-n."""
    split = 0
    if since:
        git_rel_path = str(rel_path).replace("\\", "/")
        split = _count_relations_at_commit(since, git_rel_path)
    elif last_n:
        split = max(0, len(all_rels) - last_n)
    return all_rels[:split], all_rels


def _filter_by_layer(
    rels: list[tuple[str, str, str]], layer: int | None,
) -> list[tuple[str, str, str]]:
    """Filter relations to a specific layer, or return all if layer is None."""
    if layer is None:
        return rels
    return [(s, t, rt) for s, t, rt in rels if LAYER_MAP.get(rt, 0) == layer]


def main() -> None:
    args = _build_parser().parse_args()
    base = Path(args.base)
    rel_path = base / "relations.jsonl"

    if not rel_path.exists():
        json.dump({"events": [], "meta": {"error": "relations.jsonl not found"}},
                   sys.stdout, indent=2)
        sys.stdout.write("\n")
        return

    all_rels = load_relations(rel_path)
    old_rels, new_rels = _split_relations(all_rels, rel_path, args.since, args.last_n)
    old_rels = _filter_by_layer(old_rels, args.layer)
    new_rels = _filter_by_layer(new_rels, args.layer)

    old_graph = build_graph(old_rels)
    new_graph = build_graph(new_rels)
    cycles = detect_cycles(new_graph)
    rank_delta = diff_cycle_rank(old_graph, new_graph)
    new_comps = detect_new_components(old_graph, new_graph)
    events = _assemble_events(cycles, rank_delta, new_comps)

    output = {
        "events": events,
        "meta": {
            "total_relations": len(all_rels),
            "old_relations": len(old_rels),
            "new_relations": len(new_rels),
            "old_cycle_rank": rank_delta[0],
            "new_cycle_rank": rank_delta[1],
        },
    }
    json.dump(output, sys.stdout, indent=2, ensure_ascii=False)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
