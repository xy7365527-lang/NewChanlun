"""Topological Computation — Minimal Experiment.

5 vertices, 7 edges, β₁ ≥ 1. Run 30-50 steps.
Check 5 success criteria.

Phase 1: Topological rules.
Phase 2: LLM-driven encounter detection (--use-llm).
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from engine import Graph, Vertex, Edge, EdgeType, compute_beta_1
from traversal import TraversalEngine


# Vertex content for Phase 2 LLM experiment
VERTEX_CONTENT = {
    "A": "All complex systems exhibit emergent behavior",
    "B": "Emergence requires interaction between components",
    "C": "Component interaction follows deterministic rules",
    "D": "Deterministic rules cannot produce true novelty",
    "E": "Complex systems produce genuinely novel outcomes",
}


def build_initial_graph() -> Graph:
    """Construct 5-vertex, 7-edge initial graph with β₁ ≥ 1.

    A --dependency--> B
    B --dependency--> C
    C --dependency--> A    (cycle 1: A-B-C)
    C --dependency--> D
    D --dependency--> E
    E --reference---> B    (cycle 2: B-C-D-E)
    A --reference---> D    (cross-edge)

    Each vertex carries a content proposition for LLM encounter detection.
    """
    g = Graph()
    for vid in ("A", "B", "C", "D", "E"):
        g = g.add_vertex(Vertex(vid, content=VERTEX_CONTENT[vid]))

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
    use_llm: bool = False,
) -> dict:
    """Run the minimal experiment."""
    mode = "LLM" if use_llm else "TOPO"
    graph = build_initial_graph()
    initial_beta = compute_beta_1(graph)
    print(f"[{mode}] Initial graph: {len(graph.active_vertex_ids())} vertices, "
          f"{len(graph.active_edges())} edges, β₁ = {initial_beta}")

    engine = TraversalEngine(
        graph, start="A",
        settlement_threshold=settlement_threshold,
        seed=seed,
        use_llm=use_llm,
    )

    for i in range(max_steps):
        log = engine.run_step()
        marker = ""
        if log.blocked:
            marker = " [BLOCKED]"
        elif log.delta_beta_1 != 0:
            marker = f" [Δβ₁={log.delta_beta_1:+d}]"
        if log.settled_count > 0:
            marker += f" [settled={log.settled_count}]"

        print(f"  [{mode}] Step {log.step:3d} @ {log.position:20s} | "
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
        "mode": mode,
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

    if use_llm and engine._llm_log:
        result["llm_log"] = engine._llm_log

    print(f"\n{'='*60}")
    print(f"EXPERIMENT RESULTS [{mode}]")
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


def compare_results(topo: dict, llm: dict) -> dict:
    """Generate comparison report between Phase 1 (topo) and Phase 2 (LLM) results."""
    # Operation distribution
    def op_distribution(result: dict) -> dict[str, int]:
        dist: dict[str, int] = {}
        for s in result["steps"]:
            op = s["operation"]
            dist[op] = dist.get(op, 0) + 1
        return dist

    # β₁ trajectory
    def beta_trajectory(result: dict) -> list[int]:
        return [s["beta_1"] for s in result["steps"]]

    topo_ops = op_distribution(topo)
    llm_ops = op_distribution(llm)
    topo_beta = beta_trajectory(topo)
    llm_beta = beta_trajectory(llm)

    comparison = {
        "topo_summary": {
            "final_beta_1": topo["final_beta_1"],
            "max_beta_1": max(topo_beta) if topo_beta else 0,
            "settled_cycles": topo["settled_cycles"],
            "blocked_operations": topo["blocked_operations"],
            "coverage": topo["coverage"],
            "all_passed": topo["all_passed"],
            "operation_distribution": topo_ops,
        },
        "llm_summary": {
            "final_beta_1": llm["final_beta_1"],
            "max_beta_1": max(llm_beta) if llm_beta else 0,
            "settled_cycles": llm["settled_cycles"],
            "blocked_operations": llm["blocked_operations"],
            "coverage": llm["coverage"],
            "all_passed": llm["all_passed"],
            "operation_distribution": llm_ops,
        },
        "differences": {
            "beta_1_final_diff": llm["final_beta_1"] - topo["final_beta_1"],
            "settled_diff": llm["settled_cycles"] - topo["settled_cycles"],
            "blocked_diff": llm["blocked_operations"] - topo["blocked_operations"],
            "coverage_diff": round(llm["coverage"] - topo["coverage"], 3),
        },
        "beta_trajectories": {
            "topo": topo_beta,
            "llm": llm_beta,
        },
    }
    return comparison


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Topological Computation Experiment")
    parser.add_argument("--use-llm", action="store_true", help="Use LLM for encounter detection (Phase 2)")
    parser.add_argument("--compare", action="store_true", help="Run both modes and compare")
    parser.add_argument("--steps", type=int, default=50, help="Max steps (default: 50)")
    parser.add_argument("--seed", type=int, default=42, help="Random seed (default: 42)")
    args = parser.parse_args()

    if args.compare:
        print("=" * 60)
        print("PHASE 1: TOPOLOGICAL RULES")
        print("=" * 60)
        topo_result = run_experiment(max_steps=args.steps, seed=args.seed, use_llm=False)

        print("\n\n")
        print("=" * 60)
        print("PHASE 2: LLM ENCOUNTER DETECTION")
        print("=" * 60)
        llm_result = run_experiment(max_steps=args.steps, seed=args.seed, use_llm=True)

        comparison = compare_results(topo_result, llm_result)

        print("\n\n")
        print("=" * 60)
        print("PHASE COMPARISON")
        print("=" * 60)
        for key in ("topo_summary", "llm_summary"):
            label = "TOPO" if "topo" in key else "LLM"
            s = comparison[key]
            print(f"\n  [{label}]")
            print(f"    Final β₁: {s['final_beta_1']}, Max β₁: {s['max_beta_1']}")
            print(f"    Settled: {s['settled_cycles']}, Blocked: {s['blocked_operations']}")
            print(f"    Coverage: {s['coverage']:.1%}, All passed: {s['all_passed']}")
            print(f"    Operations: {s['operation_distribution']}")

        diff = comparison["differences"]
        print(f"\n  [DIFF]")
        print(f"    β₁ final: {diff['beta_1_final_diff']:+d}")
        print(f"    Settled: {diff['settled_diff']:+d}")
        print(f"    Blocked: {diff['blocked_diff']:+d}")
        print(f"    Coverage: {diff['coverage_diff']:+.3f}")

        # Write results
        with open("experiment_result_llm.json", "w", encoding="utf-8") as f:
            json.dump(llm_result, f, indent=2, ensure_ascii=False)
        with open("phase_comparison.json", "w", encoding="utf-8") as f:
            json.dump(comparison, f, indent=2, ensure_ascii=False)
        print(f"\nResults written to experiment_result_llm.json and phase_comparison.json")

    else:
        result = run_experiment(max_steps=args.steps, seed=args.seed, use_llm=args.use_llm)

        if args.use_llm:
            out_path = Path("experiment_result_llm.json")
        else:
            out_path = Path("experiment_result.json")
        with open(out_path, "w", encoding="utf-8") as f:
            json.dump(result, f, indent=2, ensure_ascii=False)
        print(f"\nResults written to {out_path}")
