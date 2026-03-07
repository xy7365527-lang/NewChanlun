#!/usr/bin/env python3
"""Verify encounter fix: run 100 steps and check density > 0.

Loads k_merged.json (or genealogy graph), runs 100 traversal steps,
and asserts that encounter density (non-walk operations / total steps) > 0.

A density of 0 means the walker never triggers any encounter in 100 steps,
which indicates a bug in encounter detection or an extremely sparse graph.

Usage:
    python verify_encounter_density.py
    python verify_encounter_density.py --steps 200
    python verify_encounter_density.py --use-genealogy
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from engine import Graph, compute_beta_1
from traversal import TraversalEngine


DEFAULT_MERGED_PATH = Path.home() / ".swarm" / "k_merged.json"


def _load_graph(use_genealogy: bool, merged_path: Path) -> Graph:
    """Load graph from k_merged.json or genealogy."""
    if use_genealogy:
        from genealogy_loader import load_from_repo
        repo_root = Path("C:/Users/hanju/NewChanlun")
        graph, _ = load_from_repo(repo_root)
        return graph
    else:
        if not merged_path.exists():
            raise FileNotFoundError(f"k_merged.json not found: {merged_path}")
        from daemon import graph_from_dict
        data = json.loads(merged_path.read_text(encoding="utf-8"))
        return graph_from_dict(data)


def _find_start_vertex(graph: Graph) -> str:
    """Find start vertex: highest degree in largest component."""
    from engine import _connected_components

    active_vids = graph.active_vertex_ids()
    undirected = graph.undirected_active_edges()

    # Build adjacency
    adj: dict[str, set[str]] = {v: set() for v in active_vids}
    for edge_set in undirected:
        pair = sorted(edge_set)
        if len(pair) == 2:
            adj[pair[0]].add(pair[1])
            adj[pair[1]].add(pair[0])

    # Find largest component via BFS
    visited: set[str] = set()
    largest: list[str] = []
    for start in sorted(active_vids):
        if start in visited:
            continue
        component: list[str] = []
        stack = [start]
        while stack:
            v = stack.pop()
            if v in visited:
                continue
            visited.add(v)
            component.append(v)
            for nb in adj.get(v, ()):
                if nb not in visited:
                    stack.append(nb)
        if len(component) > len(largest):
            largest = component

    if not largest:
        return active_vids[0]

    # Highest degree vertex in largest component
    degrees: Counter = Counter()
    for edge_set in undirected:
        for v in edge_set:
            degrees[v] += 1

    return max(largest, key=lambda v: (degrees.get(v, 0), v))


def verify_encounter_density(
    steps: int = 100,
    use_genealogy: bool = False,
    merged_path: Path = DEFAULT_MERGED_PATH,
    seed: int = 42,
) -> dict:
    """Run N steps and verify encounter density > 0.

    Returns a result dict with density and operation breakdown.
    """
    t0 = time.time()

    print("Loading graph...")
    graph = _load_graph(use_genealogy, merged_path)
    n_v = len(graph.active_vertex_ids())
    n_e = len(graph.active_edges())
    beta_1 = compute_beta_1(graph)
    print(f"  {n_v} vertices, {n_e} edges, beta_1={beta_1}")

    if n_v == 0:
        raise ValueError("Graph has no active vertices")

    start = _find_start_vertex(graph)
    sv = graph.vertex(start)
    start_content = (sv.content or "")[:60] if sv else ""
    print(f"  Start: {start[:48]} ({start_content})")

    # Run traversal
    print(f"\nRunning {steps} steps (seed={seed})...")
    engine = TraversalEngine(
        graph, start,
        settlement_threshold=15,
        seed=seed,
    )

    for _ in range(steps):
        engine.run_step()

    # Compute density
    op_counts: Counter = Counter()
    for log in engine.logs:
        op_counts[log.operation] += 1

    walk_count = op_counts.get("walk", 0)
    non_walk = steps - walk_count
    density = non_walk / max(steps, 1)

    elapsed = time.time() - t0

    result = {
        "steps": steps,
        "density": round(density, 4),
        "walk": walk_count,
        "fold": op_counts.get("fold", 0),
        "negate_a": op_counts.get("negate_a", 0),
        "negate_b": op_counts.get("negate_b", 0),
        "sublate": op_counts.get("sublate", 0),
        "fold_blocked": op_counts.get("fold_blocked", 0),
        "negate_blocked": op_counts.get("negate_blocked", 0),
        "sublate_blocked": op_counts.get("sublate_blocked", 0),
        "initial_beta_1": beta_1,
        "final_beta_1": compute_beta_1(engine.k_active),
        "settled_cycles": len(engine.settlement.settled_cycles),
        "elapsed_seconds": round(elapsed, 3),
        "passed": density > 0,
    }

    # Print results
    print(f"\n{'='*60}")
    print(f"  Encounter Density Verification")
    print(f"{'='*60}")
    print(f"  Steps:             {steps}")
    print(f"  Encounter density: {density:.1%} ({non_walk}/{steps})")
    print(f"  Operation breakdown:")
    for op, count in sorted(op_counts.items()):
        print(f"    {op:<20} {count:>4}")
    print(f"  beta_1:            {beta_1} -> {result['final_beta_1']}")
    print(f"  Settled cycles:    {result['settled_cycles']}")
    print(f"  Time:              {elapsed:.2f}s")

    if density > 0:
        print(f"\n  PASS: density={density:.1%} > 0")
    else:
        print(f"\n  FAIL: density=0 (no encounters in {steps} steps)")

    return result


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Verify encounter density > 0 over N traversal steps"
    )
    parser.add_argument(
        "--steps", "-n",
        type=int, default=100,
        help="Number of steps to run (default: 100)",
    )
    parser.add_argument(
        "--use-genealogy",
        action="store_true",
        help="Load from genealogy instead of k_merged.json",
    )
    parser.add_argument(
        "--merged-path",
        default=str(DEFAULT_MERGED_PATH),
        help=f"k_merged.json path (default: {DEFAULT_MERGED_PATH})",
    )
    parser.add_argument(
        "--seed",
        type=int, default=42,
        help="Random seed (default: 42)",
    )

    args = parser.parse_args()

    result = verify_encounter_density(
        steps=args.steps,
        use_genealogy=args.use_genealogy,
        merged_path=Path(args.merged_path),
        seed=args.seed,
    )

    sys.exit(0 if result["passed"] else 1)


if __name__ == "__main__":
    main()
