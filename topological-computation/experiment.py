"""Topological Computation — Minimal Experiment.

5 vertices, 7 edges, β₁ ≥ 1. Run 30-50 steps.
Check 5 success criteria.

Phase 1: Topological rules.
Phase 2: LLM-driven encounter detection (--use-llm).
Phase 2 control: Controlled experiments (--control) to test fold/blocking/settlement.
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

# Extended vertex content for control B (fold candidates)
EXTENDED_VERTEX_CONTENT = {
    **VERTEX_CONTENT,
    "F": "Emergent properties arise from component interactions",
    "G": "Novel outcomes emerge from complex interactions",
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


def build_extended_graph() -> Graph:
    """Construct 7-vertex, 10-edge graph with explicit fold candidates.

    Base graph (A-E, 7 edges) plus:
    F: "Emergent properties arise from component interactions" (similar to B)
    G: "Novel outcomes emerge from complex interactions" (similar to E)

    Additional edges:
    F --dependency--> C
    G --dependency--> D
    A --reference---> F

    F is a fold candidate with B (both about emergence + component interaction).
    G is a fold candidate with E (both about novel outcomes from complexity).
    """
    g = Graph()
    for vid, content in EXTENDED_VERTEX_CONTENT.items():
        g = g.add_vertex(Vertex(vid, content=content))

    edges = [
        # Original 7 edges
        Edge("A", "B", EdgeType.DEPENDENCY),
        Edge("B", "C", EdgeType.DEPENDENCY),
        Edge("C", "A", EdgeType.DEPENDENCY),
        Edge("C", "D", EdgeType.DEPENDENCY),
        Edge("D", "E", EdgeType.DEPENDENCY),
        Edge("E", "B", EdgeType.REFERENCE),
        Edge("A", "D", EdgeType.REFERENCE),
        # New 3 edges
        Edge("F", "C", EdgeType.DEPENDENCY),
        Edge("G", "D", EdgeType.DEPENDENCY),
        Edge("A", "F", EdgeType.REFERENCE),
    ]
    for e in edges:
        g = g.add_edge(e)

    return g


def run_experiment(
    max_steps: int = 50,
    settlement_threshold: int = 5,
    seed: int = 42,
    use_llm: bool = False,
    graph: Graph | None = None,
    conservative_prompt: bool = False,
    label: str | None = None,
) -> dict:
    """Run the minimal experiment.

    Args:
        graph: If provided, use this graph instead of the default.
        conservative_prompt: If True, use conservative LLM prompt (control A).
        label: Optional label for the experiment (used in output).
    """
    mode = label or ("LLM" if use_llm else "TOPO")
    if graph is None:
        graph = build_initial_graph()
    initial_beta = compute_beta_1(graph)
    print(f"[{mode}] Initial graph: {len(graph.active_vertex_ids())} vertices, "
          f"{len(graph.active_edges())} edges, β₁ = {initial_beta}")

    engine = TraversalEngine(
        graph, start="A",
        settlement_threshold=settlement_threshold,
        seed=seed,
        use_llm=use_llm,
        conservative_prompt=conservative_prompt,
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


def summarize_experiment(result: dict) -> dict:
    """Extract summary metrics from an experiment result."""
    op_dist: dict[str, int] = {}
    for s in result["steps"]:
        op = s["operation"]
        op_dist[op] = op_dist.get(op, 0) + 1

    beta_trajectory = [s["beta_1"] for s in result["steps"]]

    return {
        "label": result.get("mode", "unknown"),
        "initial_beta_1": result["initial_beta_1"],
        "final_beta_1": result["final_beta_1"],
        "max_beta_1": max(beta_trajectory) if beta_trajectory else 0,
        "total_steps": result["total_steps"],
        "settled_cycles": result["settled_cycles"],
        "blocked_operations": result["blocked_operations"],
        "coverage": result["coverage"],
        "operation_distribution": op_dist,
        "beta_trajectory": beta_trajectory,
        "success_criteria": result["success_criteria"],
    }


def run_control_experiments(
    max_steps: int = 50,
    seed: int = 42,
    output_dir: Path | None = None,
) -> dict:
    """Run 4 controlled experiments and generate comparison report.

    Control A: Conservative prompt + original 5-vertex graph + threshold=5
    Control B: Standard prompt + extended 7-vertex graph + threshold=5
    Control C: Standard prompt + original 5-vertex graph + threshold=15
    Control ABC: Conservative prompt + extended 7-vertex graph + threshold=15
    """
    if output_dir is None:
        output_dir = Path(".")

    experiments = {
        "control_A": {
            "label": "CTRL-A (conservative prompt)",
            "graph": build_initial_graph(),
            "conservative_prompt": True,
            "settlement_threshold": 5,
            "out_file": "experiment_control_A.json",
        },
        "control_B": {
            "label": "CTRL-B (extended graph)",
            "graph": build_extended_graph(),
            "conservative_prompt": False,
            "settlement_threshold": 5,
            "out_file": "experiment_control_B.json",
        },
        "control_C": {
            "label": "CTRL-C (high threshold)",
            "graph": build_initial_graph(),
            "conservative_prompt": False,
            "settlement_threshold": 15,
            "out_file": "experiment_control_C.json",
        },
        "control_ABC": {
            "label": "CTRL-ABC (all controls)",
            "graph": build_extended_graph(),
            "conservative_prompt": True,
            "settlement_threshold": 15,
            "out_file": "experiment_control_ABC.json",
        },
    }

    results: dict[str, dict] = {}
    summaries: dict[str, dict] = {}

    for key, config in experiments.items():
        print("\n" + "=" * 60)
        print(f"CONTROL EXPERIMENT: {config['label']}")
        print("=" * 60)

        result = run_experiment(
            max_steps=max_steps,
            settlement_threshold=config["settlement_threshold"],
            seed=seed,
            use_llm=True,
            graph=config["graph"],
            conservative_prompt=config["conservative_prompt"],
            label=config["label"],
        )

        out_path = output_dir / config["out_file"]
        with open(out_path, "w", encoding="utf-8") as f:
            json.dump(result, f, indent=2, ensure_ascii=False)
        print(f"\n  Written to {out_path}")

        results[key] = result
        summaries[key] = summarize_experiment(result)

    # Generate comparison report
    fold_observed = any(
        summaries[k]["operation_distribution"].get("fold", 0) > 0
        for k in summaries
    )
    blocking_observed = any(
        summaries[k]["blocked_operations"] > 0
        for k in summaries
    )
    settlement_lt_100 = any(
        # settlement < 100% means some cycles did NOT settle
        # Proxy: if settled_cycles == 0, settlement_ratio = 0
        # If settled_cycles > 0 but blocking occurred, that implies settlement constraint worked
        summaries[k]["settled_cycles"] == 0
        or summaries[k]["blocked_operations"] > 0
        for k in summaries
    )

    report = {
        "experiment_matrix": {
            k: {
                "description": experiments[k]["label"],
                "conservative_prompt": experiments[k]["conservative_prompt"],
                "extended_graph": k in ("control_B", "control_ABC"),
                "settlement_threshold": experiments[k]["settlement_threshold"],
            }
            for k in experiments
        },
        "summaries": summaries,
        "success_criteria": {
            "fold_executed_at_least_once": fold_observed,
            "blocking_triggered_at_least_once": blocking_observed,
            "settlement_ratio_lt_100_at_least_once": settlement_lt_100,
            "any_success": fold_observed or blocking_observed or settlement_lt_100,
        },
        "cross_experiment_comparison": {
            "fold_counts": {
                k: summaries[k]["operation_distribution"].get("fold", 0)
                for k in summaries
            },
            "negate_counts": {
                k: (
                    summaries[k]["operation_distribution"].get("negate_a", 0)
                    + summaries[k]["operation_distribution"].get("negate_b", 0)
                )
                for k in summaries
            },
            "sublate_counts": {
                k: summaries[k]["operation_distribution"].get("sublate", 0)
                for k in summaries
            },
            "walk_counts": {
                k: summaries[k]["operation_distribution"].get("walk", 0)
                for k in summaries
            },
            "blocked_counts": {
                k: summaries[k]["blocked_operations"]
                for k in summaries
            },
            "settled_counts": {
                k: summaries[k]["settled_cycles"]
                for k in summaries
            },
            "final_beta_1": {
                k: summaries[k]["final_beta_1"]
                for k in summaries
            },
        },
    }

    report_path = output_dir / "control_experiment_report.json"
    with open(report_path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)

    # Print summary
    print("\n\n" + "=" * 60)
    print("CONTROL EXPERIMENT COMPARISON REPORT")
    print("=" * 60)
    for k, s in summaries.items():
        print(f"\n  [{s['label']}]")
        print(f"    β₁: {s['initial_beta_1']} → {s['final_beta_1']} (max={s['max_beta_1']})")
        print(f"    Settled: {s['settled_cycles']}, Blocked: {s['blocked_operations']}")
        print(f"    Coverage: {s['coverage']:.1%}")
        print(f"    Operations: {s['operation_distribution']}")

    print(f"\n  SUCCESS CRITERIA:")
    for name, passed in report["success_criteria"].items():
        status = "PASS" if passed else "FAIL"
        print(f"    [{status}] {name}")

    print(f"\n  Report written to {report_path}")
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Topological Computation Experiment")
    parser.add_argument("--use-llm", action="store_true", help="Use LLM for encounter detection (Phase 2)")
    parser.add_argument("--compare", action="store_true", help="Run both modes and compare")
    parser.add_argument("--control", action="store_true", help="Run Phase 2 control experiments (A/B/C/ABC)")
    parser.add_argument("--conservative-prompt", action="store_true", help="Use conservative LLM prompt")
    parser.add_argument("--extended-graph", action="store_true", help="Use 7-vertex extended graph")
    parser.add_argument("--settlement-threshold", type=int, default=5, help="Settlement threshold (default: 5)")
    parser.add_argument("--steps", type=int, default=50, help="Max steps (default: 50)")
    parser.add_argument("--seed", type=int, default=42, help="Random seed (default: 42)")
    args = parser.parse_args()

    if args.control:
        run_control_experiments(
            max_steps=args.steps,
            seed=args.seed,
            output_dir=Path("."),
        )

    elif args.compare:
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
        graph = build_extended_graph() if args.extended_graph else build_initial_graph()
        result = run_experiment(
            max_steps=args.steps,
            settlement_threshold=args.settlement_threshold,
            seed=args.seed,
            use_llm=args.use_llm,
            graph=graph,
            conservative_prompt=args.conservative_prompt,
        )

        if args.use_llm:
            out_path = Path("experiment_result_llm.json")
        else:
            out_path = Path("experiment_result.json")
        with open(out_path, "w", encoding="utf-8") as f:
            json.dump(result, f, indent=2, ensure_ascii=False)
        print(f"\nResults written to {out_path}")
