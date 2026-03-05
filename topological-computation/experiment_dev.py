"""Topological Computation — Developmental Experiment.

Large synthetic graph (30-40 vertices, two subgraphs + bridge).
500 steps, pure topological rules (use_llm=False).
Tests the developmental hypothesis: 4 phases emerge naturally.

1. Infancy: false positive folds (shared neighbors but semantically different)
2. Crisis: wrong folds negated, beta_1 rapid growth
3. Stabilization: fold candidates protected by settlement become more accurate
4. Maturity: settlement rate stabilizes, autonomous perception

CLI: python experiment_dev.py [--steps N] [--seed N] [--threshold N]
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import deque
from pathlib import Path

from engine import (
    Graph, Vertex, Edge, EdgeType, VertexStatus,
    compute_beta_1, SettlementTracker,
)
from traversal import TraversalEngine


# ---------------------------------------------------------------------------
# Graph construction
# ---------------------------------------------------------------------------

def _build_subgraph_a() -> tuple[list[Vertex], list[Edge]]:
    """Subgraph A: 18 vertices, domain = 'logic'.

    Contains 4 internal cycles via bidirectional and cross edges.
    """
    names = {
        "a01": "Formal systems require axioms",
        "a02": "Axioms are unprovable assumptions",
        "a03": "Assumptions may contain hidden contradictions",
        "a04": "Contradictions invalidate formal proofs",
        "a05": "Proofs establish truth within a system",
        "a06": "Truth depends on consistency of axioms",
        "a07": "Consistency cannot be self-proven (Goedel)",
        "a08": "Self-reference creates paradox",
        "a09": "Paradox reveals limits of formalization",
        "a10": "Limits motivate meta-level reasoning",
        "a11": "Meta-level reasoning requires its own axioms",
        "a12": "Infinite regress of justification",
        "a13": "Regress is halted by pragmatic acceptance",
        "a14": "Pragmatic acceptance is not logical proof",
        "a15": "Logical proof requires complete axioms",
        "a16": "Complete axiom sets are impossible (Goedel)",
        "a17": "Impossibility drives creative formalization",
        "a18": "Creative formalization produces new axioms",
    }
    vertices = [Vertex(vid, content=c) for vid, c in names.items()]
    edges = [
        # Chain
        Edge("a01", "a02", EdgeType.DEPENDENCY),
        Edge("a02", "a03", EdgeType.DEPENDENCY),
        Edge("a03", "a04", EdgeType.DEPENDENCY),
        Edge("a04", "a05", EdgeType.REFERENCE),
        Edge("a05", "a06", EdgeType.DEPENDENCY),
        Edge("a06", "a07", EdgeType.DEPENDENCY),
        Edge("a07", "a08", EdgeType.DEPENDENCY),
        Edge("a08", "a09", EdgeType.DEPENDENCY),
        Edge("a09", "a10", EdgeType.DEPENDENCY),
        Edge("a10", "a11", EdgeType.DEPENDENCY),
        Edge("a11", "a12", EdgeType.DEPENDENCY),
        Edge("a12", "a13", EdgeType.DEPENDENCY),
        Edge("a13", "a14", EdgeType.DEPENDENCY),
        Edge("a14", "a15", EdgeType.REFERENCE),
        Edge("a15", "a16", EdgeType.DEPENDENCY),
        Edge("a16", "a17", EdgeType.DEPENDENCY),
        Edge("a17", "a18", EdgeType.DEPENDENCY),
        # Cycle 1: a01-a02-a03-a04-a05-a06-a01
        Edge("a06", "a01", EdgeType.REFERENCE),
        # Cycle 2: a07-a08-a09-a10-a11-a07
        Edge("a11", "a07", EdgeType.REFERENCE),
        # Cycle 3: a15-a16-a17-a18-a01-a02-...-a15 (long)
        Edge("a18", "a01", EdgeType.DEPENDENCY),
        # Bidirectional pairs (contradiction signals)
        Edge("a03", "a06", EdgeType.REFERENCE),   # assumption <-> consistency
        Edge("a08", "a10", EdgeType.REFERENCE),   # self-ref <-> meta-level
        Edge("a13", "a15", EdgeType.REFERENCE),   # pragmatic <-> logical proof
        # Cross edges (more cycle-forming potential)
        Edge("a04", "a09", EdgeType.REFERENCE),   # contradiction <-> paradox
        Edge("a12", "a16", EdgeType.REFERENCE),   # regress <-> impossibility
    ]
    return vertices, edges


def _build_subgraph_b() -> tuple[list[Vertex], list[Edge]]:
    """Subgraph B: 17 vertices, domain = 'empirics'.

    Contains 3 internal cycles via bidirectional and cross edges.
    """
    names = {
        "b01": "Observation requires theoretical framework",
        "b02": "Theory shapes what counts as data",
        "b03": "Data confirms or refutes theory",
        "b04": "Refutation requires alternative theory",
        "b05": "Alternative theories compete for evidence",
        "b06": "Evidence is theory-laden",
        "b07": "Theory-ladenness threatens objectivity",
        "b08": "Objectivity requires inter-subjective agreement",
        "b09": "Agreement can be socially constructed",
        "b10": "Social construction undermines truth claims",
        "b11": "Truth claims require empirical grounding",
        "b12": "Empirical grounding depends on instruments",
        "b13": "Instruments embody theoretical assumptions",
        "b14": "Assumptions create circular justification",
        "b15": "Circular justification is a logical fallacy",
        "b16": "Fallacies can still produce useful knowledge",
        "b17": "Useful knowledge is pragmatically validated",
    }
    vertices = [Vertex(vid, content=c) for vid, c in names.items()]
    edges = [
        # Chain
        Edge("b01", "b02", EdgeType.DEPENDENCY),
        Edge("b02", "b03", EdgeType.DEPENDENCY),
        Edge("b03", "b04", EdgeType.DEPENDENCY),
        Edge("b04", "b05", EdgeType.DEPENDENCY),
        Edge("b05", "b06", EdgeType.DEPENDENCY),
        Edge("b06", "b07", EdgeType.DEPENDENCY),
        Edge("b07", "b08", EdgeType.DEPENDENCY),
        Edge("b08", "b09", EdgeType.DEPENDENCY),
        Edge("b09", "b10", EdgeType.DEPENDENCY),
        Edge("b10", "b11", EdgeType.DEPENDENCY),
        Edge("b11", "b12", EdgeType.DEPENDENCY),
        Edge("b12", "b13", EdgeType.DEPENDENCY),
        Edge("b13", "b14", EdgeType.DEPENDENCY),
        Edge("b14", "b15", EdgeType.DEPENDENCY),
        Edge("b15", "b16", EdgeType.DEPENDENCY),
        Edge("b16", "b17", EdgeType.DEPENDENCY),
        # Cycle 1: b01-b02-b03-...-b06-b01
        Edge("b06", "b01", EdgeType.REFERENCE),
        # Cycle 2: b07-b08-b09-b10-b11-b07
        Edge("b11", "b07", EdgeType.REFERENCE),
        # Cycle 3: b13-b14-b15-b16-b17-b01-...-b13 (long)
        Edge("b17", "b01", EdgeType.DEPENDENCY),
        # Bidirectional pairs
        Edge("b02", "b06", EdgeType.REFERENCE),   # theory <-> theory-laden
        Edge("b09", "b11", EdgeType.REFERENCE),   # agreement <-> truth
        Edge("b14", "b17", EdgeType.REFERENCE),   # circular <-> pragmatic
        # Cross edges
        Edge("b03", "b07", EdgeType.REFERENCE),   # data <-> objectivity
        Edge("b10", "b15", EdgeType.REFERENCE),   # social <-> fallacy
    ]
    return vertices, edges


def build_dev_graph() -> Graph:
    """Build the developmental experiment graph.

    Two subgraphs (A: logic, B: empirics) connected by 3 bridge edges.
    Total: 35 vertices, ~50 edges, initial beta_1 ~ 8-15.
    """
    verts_a, edges_a = _build_subgraph_a()
    verts_b, edges_b = _build_subgraph_b()

    g = Graph()
    for v in verts_a + verts_b:
        g = g.add_vertex(v)
    for e in edges_a + edges_b:
        g = g.add_edge(e)

    # Bridge edges (cross-domain connections)
    bridges = [
        Edge("a03", "b14", EdgeType.REFERENCE),   # hidden contradictions <-> circular justification
        Edge("a13", "b17", EdgeType.REFERENCE),   # pragmatic acceptance <-> pragmatic validation
        Edge("b07", "a09", EdgeType.REFERENCE),   # objectivity threat <-> limits of formalization
    ]
    for e in bridges:
        g = g.add_edge(e)

    return g


# ---------------------------------------------------------------------------
# False positive fold tracking
# ---------------------------------------------------------------------------

class FoldTracker:
    """Track folds and detect false positives.

    A false positive fold = a fold whose target vertex gets negated within
    `window` steps after the fold.
    """

    def __init__(self, window: int = 10) -> None:
        self.window = window
        self._fold_log: list[dict] = []       # all folds with step + targets
        self._negate_log: list[dict] = []     # all negations with step + targets
        self.total_folds = 0
        self.false_positive_folds = 0

    def record_fold(self, step: int, target_a: str, target_b: str) -> None:
        self._fold_log.append({
            "step": step, "a": target_a, "b": target_b,
        })
        self.total_folds += 1

    def record_negate(self, step: int, thesis: str, antithesis: str | None) -> None:
        self._negate_log.append({
            "step": step, "thesis": thesis, "antithesis": antithesis,
        })

    def compute_false_positive_rate(self) -> float:
        """Compute false positive fold rate.

        For each fold at step S targeting vertex V (the kept vertex),
        check if V gets negated in steps [S+1, S+window].
        """
        if self.total_folds == 0:
            return 0.0

        fp_count = 0
        for f in self._fold_log:
            fold_step = f["step"]
            kept = f["a"]  # target_a is the kept vertex in fold
            for n in self._negate_log:
                if fold_step < n["step"] <= fold_step + self.window:
                    if n["thesis"] == kept or n["antithesis"] == kept:
                        fp_count += 1
                        break

        self.false_positive_folds = fp_count
        return fp_count / self.total_folds


# ---------------------------------------------------------------------------
# Phase detection
# ---------------------------------------------------------------------------

def detect_phases(
    step_logs: list[dict],
    initial_beta_1: int,
) -> dict:
    """Detect developmental phases from step-level metrics.

    Phase transitions detected via beta_1 growth rate changes
    and false positive fold patterns.

    Returns phase analysis dict with boundaries and characteristics.
    """
    n = len(step_logs)
    if n < 20:
        return {"detected": False, "reason": "too_few_steps"}

    # Compute beta_1 growth rate (rolling window = 20 steps)
    window = 20
    growth_rates: list[float] = []
    for i in range(window, n):
        beta_start = step_logs[i - window]["beta_1"]
        beta_end = step_logs[i]["beta_1"]
        rate = (beta_end - beta_start) / window
        growth_rates.append(rate)

    if not growth_rates:
        return {"detected": False, "reason": "insufficient_growth_data"}

    # Find first negate that undoes a previous fold (infancy end)
    infancy_end = None
    fold_targets: list[tuple[int, str]] = []  # (step, kept_vertex)
    for s in step_logs:
        if s["operation"] == "fold":
            fold_targets.append((s["step"], s["position"]))
        elif "negate" in s["operation"] and not s["blocked"]:
            # Check if this negate targets a vertex that was previously folded into
            for fs, fv in fold_targets:
                if s["step"] > fs and fv in (s.get("thesis", ""), s.get("antithesis", "")):
                    infancy_end = s["step"]
                    break
                # Also check position match (negate at a previously fold-created location)
                if s["step"] > fs and s["position"] == fv:
                    infancy_end = s["step"]
                    break
            if infancy_end is not None:
                break

    # If no direct fold-undo detected, use first negate_a as proxy
    if infancy_end is None:
        for s in step_logs:
            if s["operation"] == "negate_a" and not s["blocked"]:
                infancy_end = s["step"]
                break

    # Crisis end: beta_1 growth rate drops below 50% of initial growth rate
    crisis_end = None
    if growth_rates:
        initial_rate = max(growth_rates[:min(10, len(growth_rates))])
        if initial_rate > 0:
            threshold = initial_rate * 0.5
            for i, rate in enumerate(growth_rates):
                if i > 10 and rate < threshold:
                    crisis_end = step_logs[i + window]["step"]
                    break

    # Stabilization end: 50 consecutive steps with no false positive fold
    # (approximated: 50 consecutive steps where no fold is followed by negate within 10 steps)
    stabilization_end = None
    clean_streak = 0
    fold_steps_set = {s["step"] for s in step_logs if s["operation"] == "fold"}
    negate_steps_with_target: list[tuple[int, str]] = []
    for s in step_logs:
        if "negate" in s["operation"] and not s["blocked"]:
            negate_steps_with_target.append((s["step"], s.get("position", "")))

    for s in step_logs:
        step = s["step"]
        is_fp = False
        if step in fold_steps_set:
            for ns, nt in negate_steps_with_target:
                if step < ns <= step + 10:
                    is_fp = True
                    break
        if is_fp:
            clean_streak = 0
        else:
            clean_streak += 1
        if clean_streak >= 50 and stabilization_end is None:
            stabilization_end = step

    # Build phase boundaries
    phases = []
    if infancy_end is not None:
        phases.append({
            "name": "infancy",
            "start": 1,
            "end": infancy_end,
            "description": "Topological rules produce folds, some false positives",
        })

    if crisis_end is not None and infancy_end is not None:
        phases.append({
            "name": "crisis",
            "start": infancy_end + 1,
            "end": crisis_end,
            "description": "Wrong folds negated, beta_1 rapid growth",
        })
    elif infancy_end is not None:
        # Crisis may extend to end
        phases.append({
            "name": "crisis",
            "start": infancy_end + 1,
            "end": step_logs[-1]["step"],
            "description": "Wrong folds negated, beta_1 growth (no clear end detected)",
        })

    if stabilization_end is not None:
        stab_start = (crisis_end + 1) if crisis_end else (
            (infancy_end + 1) if infancy_end else 1
        )
        phases.append({
            "name": "stabilization",
            "start": stab_start,
            "end": stabilization_end,
            "description": "Fold accuracy improves, settlement protects stable cycles",
        })
        phases.append({
            "name": "maturity",
            "start": stabilization_end + 1,
            "end": step_logs[-1]["step"],
            "description": "Settlement rate stable, autonomous perception",
        })

    return {
        "detected": len(phases) > 0,
        "phase_count": len(phases),
        "phases": phases,
        "infancy_end_step": infancy_end,
        "crisis_end_step": crisis_end,
        "stabilization_end_step": stabilization_end,
        "growth_rate_max": max(growth_rates) if growth_rates else 0.0,
        "growth_rate_min": min(growth_rates) if growth_rates else 0.0,
        "growth_rate_final": growth_rates[-1] if growth_rates else 0.0,
    }


# ---------------------------------------------------------------------------
# Main experiment
# ---------------------------------------------------------------------------

def run_dev_experiment(
    steps: int = 500,
    seed: int = 42,
    settlement_threshold: int = 10,
) -> dict:
    """Run the developmental experiment."""
    graph = build_dev_graph()
    initial_beta = compute_beta_1(graph)
    n_verts = len(graph.active_vertex_ids())
    n_edges = len(graph.active_edges())

    print(f"[DEV] Initial graph: {n_verts} vertices, {n_edges} edges, "
          f"beta_1 = {initial_beta}")

    engine = TraversalEngine(
        graph, start="a01",
        settlement_threshold=settlement_threshold,
        seed=seed,
        use_llm=False,
    )
    fold_tracker = FoldTracker(window=10)

    step_data: list[dict] = []

    for i in range(steps):
        log = engine.run_step()

        # Track folds and negations for false positive analysis
        if log.operation == "fold":
            fold_tracker.record_fold(log.step, log.position, log.position)
        elif "negate" in log.operation and not log.blocked:
            fold_tracker.record_negate(log.step, log.position, None)

        # Compute cumulative counts
        fold_cum = sum(1 for s in engine.logs if s.operation == "fold")
        negate_cum = sum(
            1 for s in engine.logs
            if "negate" in s.operation and not s.blocked
        )
        sublate_cum = sum(1 for s in engine.logs if s.operation == "sublate")
        blocked_cum = sum(1 for s in engine.logs if s.blocked)

        # Settlement rate: settled / total registered
        total_registered = (
            len(engine.settlement.settled_cycles)
            + len(engine.settlement._pending)
        )
        settlement_rate = (
            len(engine.settlement.settled_cycles) / total_registered
            if total_registered > 0 else 0.0
        )

        entry = {
            "step": log.step,
            "position": log.position,
            "encounter": log.encounter,
            "operation": log.operation,
            "beta_1": log.beta_1_after,
            "delta_beta_1": log.delta_beta_1,
            "blocked": log.blocked,
            "v_active": log.vertices_active,
            "e_active": log.edges_active,
            "settled_count": log.settled_count,
            "fold_count_cumulative": fold_cum,
            "negate_count_cumulative": negate_cum,
            "sublate_count_cumulative": sublate_cum,
            "blocked_count_cumulative": blocked_cum,
            "settlement_rate": round(settlement_rate, 4),
        }
        step_data.append(entry)

        # Progress output every 50 steps
        if log.step % 50 == 0 or log.step == 1:
            marker = ""
            if log.blocked:
                marker = " [BLOCKED]"
            elif log.delta_beta_1 != 0:
                marker = f" [d_b1={log.delta_beta_1:+d}]"
            print(f"  [DEV] Step {log.step:4d} @ {log.position:12s} | "
                  f"b1={log.beta_1_after:3d} fold={fold_cum} neg={negate_cum} "
                  f"sub={sublate_cum} settled={log.settled_count} "
                  f"sr={settlement_rate:.2f}{marker}")

    # -- Summary metrics ---------------------------------------------------

    final_beta = compute_beta_1(engine.k_active)
    max_beta = max(s["beta_1"] for s in step_data)
    fp_rate = fold_tracker.compute_false_positive_rate()

    # Operation distribution
    op_dist: dict[str, int] = {}
    for s in step_data:
        op = s["operation"]
        op_dist[op] = op_dist.get(op, 0) + 1

    # Coverage
    all_visited = {s["position"] for s in step_data}
    all_active = set(engine.k_active.active_vertex_ids())
    coverage = len(all_visited & all_active) / max(len(all_active), 1)

    # Phase analysis
    phase_analysis = detect_phases(step_data, initial_beta)

    # Blocked log details
    blocked_details = [
        {
            "step": b["step"],
            "operation": b["operation"],
            "detail": {
                k: v for k, v in b["detail"].items()
            },
            "blocked_by_edges": len(b["blocked_by"]),
        }
        for b in engine.settlement.blocked_log
    ]

    summary = {
        "initial_beta_1": initial_beta,
        "final_beta_1": final_beta,
        "max_beta_1": max_beta,
        "total_steps": steps,
        "vertices_initial": n_verts,
        "edges_initial": n_edges,
        "vertices_final_active": len(engine.k_active.active_vertex_ids()),
        "vertices_final_full": len(engine.k_full.vertices),
        "edges_final_active": len(engine.k_active.active_edges()),
        "edges_final_full": len(engine.k_full.edges),
        "settled_cycles": len(engine.settlement.settled_cycles),
        "blocked_operations": len(engine.settlement.blocked_log),
        "coverage": round(coverage, 4),
        "false_positive_fold_rate": round(fp_rate, 4),
        "total_folds": fold_tracker.total_folds,
        "false_positive_folds": fold_tracker.false_positive_folds,
        "operation_distribution": op_dist,
    }

    result = {
        "config": {
            "steps": steps,
            "seed": seed,
            "settlement_threshold": settlement_threshold,
            "graph": "dev_35v_two_subgraphs_bridge",
            "use_llm": False,
        },
        "summary_metrics": summary,
        "phase_analysis": phase_analysis,
        "blocked_details": blocked_details,
        "step_logs": step_data,
    }

    # -- Print summary -----------------------------------------------------

    print(f"\n{'=' * 60}")
    print("DEVELOPMENTAL EXPERIMENT RESULTS")
    print(f"{'=' * 60}")
    print(f"  beta_1: {initial_beta} -> {final_beta} (max={max_beta})")
    print(f"  Steps: {steps}")
    print(f"  Vertices: {summary['vertices_final_active']} active / "
          f"{summary['vertices_final_full']} full")
    print(f"  Settled cycles: {summary['settled_cycles']}")
    print(f"  Blocked operations: {summary['blocked_operations']}")
    print(f"  Coverage: {coverage:.1%}")
    print(f"  False positive fold rate: {fp_rate:.1%} "
          f"({fold_tracker.false_positive_folds}/{fold_tracker.total_folds})")
    print(f"  Operations: {op_dist}")
    print()

    if phase_analysis["detected"]:
        print(f"  PHASE ANALYSIS ({phase_analysis['phase_count']} phases detected):")
        for p in phase_analysis["phases"]:
            print(f"    [{p['name']:15s}] steps {p['start']:4d}-{p['end']:4d} : "
                  f"{p['description']}")
        print(f"    Growth rate: max={phase_analysis['growth_rate_max']:.3f} "
              f"min={phase_analysis['growth_rate_min']:.3f} "
              f"final={phase_analysis['growth_rate_final']:.3f}")
    else:
        print(f"  PHASE ANALYSIS: not detected — "
              f"{phase_analysis.get('reason', 'unknown')}")

    return result


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Topological Computation — Developmental Experiment",
    )
    parser.add_argument(
        "--steps", type=int, default=500,
        help="Number of steps (default: 500)",
    )
    parser.add_argument(
        "--seed", type=int, default=42,
        help="Random seed (default: 42)",
    )
    parser.add_argument(
        "--threshold", type=int, default=10,
        help="Settlement threshold (default: 10)",
    )
    args = parser.parse_args()

    result = run_dev_experiment(
        steps=args.steps,
        seed=args.seed,
        settlement_threshold=args.threshold,
    )

    out_path = Path("experiment_dev_result.json")
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)
    print(f"\nResults written to {out_path}")
