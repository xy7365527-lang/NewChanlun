"""Topological Computation — Minimal Experiment.

5 vertices, 7 edges, β₁ ≥ 1. Run 30-50 steps.
Check 5 success criteria.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

from engine import Graph, Vertex, Edge, EdgeType, compute_beta_1
from traversal import TraversalEngine


def build_initial_graph() -> Graph:
    """Construct 5-vertex, 7-edge initial graph with β₁ ≥ 1.

    A --dependency--> B
    B --dependency--> C
    C --dependency--> A    (cycle 1: A-B-C)
    C --dependency--> D
    D --dependency--> E
    E --reference---> B    (cycle 2: B-C-D-E)
    A --reference---> D    (cross-edge)
    """
    g = Graph()
    for vid in ("A", "B", "C", "D", "E"):
        g = g.add_vertex(Vertex(vid))

    edges = [
        Edge("A", "B", EdgeType.DEPENDENCY),
        Edge("B", "C", EdgeType.DEPENDENCY),
        Edge("C", "A", EdgeType.DEPENDENCY),
        Edge("C", "D", EdgeType.DEPENDENCY),
        Edge("D", "E", EdgeType.DEPENDENCY),
        Edge("E", "B", EdgeType.REFERENCE),
        Edge("A", "D", EdgeType.REFERENCE),
    ]
    for e in edges:
        g = g.add_edge(e)

    return g


def run_experiment(
    max_steps: int = 50,
    settlement_threshold: int = 5,
    seed: int = 42,
) -> dict:
    """Run the minimal experiment."""
    graph = build_initial_graph()
    initial_beta = compute_beta_1(graph)
    print(f"Initial graph: {len(graph.active_vertex_ids())} vertices, "
          f"{len(graph.active_edges())} edges, β₁ = {initial_beta}")

    engine = TraversalEngine(graph, start="A", settlement_threshold=settlement_threshold, seed=seed)

    for i in range(max_steps):
        log = engine.run_step()
        marker = ""
        if log.blocked:
            marker = " [BLOCKED]"
        elif log.delta_beta_1 != 0:
            marker = f" [Δβ₁={log.delta_beta_1:+d}]"
        if log.settled_count > 0:
            marker += f" [settled={log.settled_count}]"

        print(f"  Step {log.step:3d} @ {log.position:20s} | "
              f"{log.encounter:12s} → {log.operation:16s} | "
              f"β₁={log.beta_1_after} V={log.vertices_active} E={log.edges_active}"
              f"{marker}")

    # -- Success criteria ---------------------------------------------------

    final_beta = compute_beta_1(engine.k_active)
    max_beta = max(log.beta_1_after for log in engine.logs)
    all_visited = set()
    for log in engine.logs:
        all_visited.add(log.position)
    all_active = set(engine.k_active.active_vertex_ids())
    coverage = len(all_visited & all_active) / max(len(all_active), 1)

    contradiction_path = False
    for i, log in enumerate(engine.logs):
        if "negate" in log.operation and not log.blocked:
            # Check if sublation follows for the same pair
            for j in range(i + 1, len(engine.logs)):
                if engine.logs[j].operation == "sublate":
                    contradiction_path = True
                    break
            if contradiction_path:
                break

    settlement_occurred = len(engine.settlement.settled_cycles) > 0
    blocking_occurred = len(engine.settlement.blocked_log) > 0

    criteria = {
        "1_beta_1_grew": max_beta > initial_beta,
        "2_contradiction_path": contradiction_path,
        "3_settlement": settlement_occurred,
        "4_blocking": blocking_occurred,
        "5_coverage_80pct": coverage >= 0.80,
    }

    result = {
        "initial_beta_1": initial_beta,
        "final_beta_1": final_beta,
        "total_steps": len(engine.logs),
        "vertices_final_active": len(engine.k_active.active_vertex_ids()),
        "vertices_final_full": len(engine.k_full.vertices),
        "edges_final_active": len(engine.k_active.active_edges()),
        "edges_final_full": len(engine.k_full.edges),
        "settled_cycles": len(engine.settlement.settled_cycles),
        "blocked_operations": len(engine.settlement.blocked_log),
        "coverage": round(coverage, 3),
        "success_criteria": criteria,
        "all_passed": all(criteria.values()),
        "steps": [
            {
                "step": log.step,
                "position": log.position,
                "encounter": log.encounter,
                "operation": log.operation,
                "beta_1": log.beta_1_after,
                "delta_beta_1": log.delta_beta_1,
                "settled": log.settled_count,
                "blocked": log.blocked,
                "v_active": log.vertices_active,
                "e_active": log.edges_active,
            }
            for log in engine.logs
        ],
    }

    print(f"\n{'='*60}")
    print(f"EXPERIMENT RESULTS")
    print(f"{'='*60}")
    print(f"  β₁: {initial_beta} → {final_beta} (max={max_beta}, Δmax={max_beta - initial_beta})")
    print(f"  Steps: {len(engine.logs)}")
    print(f"  Vertices: {len(engine.k_active.active_vertex_ids())} active / "
          f"{len(engine.k_full.vertices)} full")
    print(f"  Settled cycles: {len(engine.settlement.settled_cycles)}")
    print(f"  Blocked operations: {len(engine.settlement.blocked_log)}")
    print(f"  Coverage: {coverage:.1%}")
    print()
    print("  Success Criteria:")
    for name, passed in criteria.items():
        status = "PASS" if passed else "FAIL"
        print(f"    [{status}] {name}")
    print()
    overall = "ALL PASSED" if result["all_passed"] else "SOME FAILED"
    print(f"  Overall: {overall}")

    return result


if __name__ == "__main__":
    result = run_experiment()

    out_path = Path("experiment_result.json")
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)
    print(f"\nResults written to {out_path}")
