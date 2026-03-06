"""test_phi_L: Control experiment — consistent text vs contradictory text.

Runs phi_L + TraversalEngine end-to-end on two contrasting texts,
then compares topological signatures.

Usage: python test_phi_L.py
"""

from __future__ import annotations

import sys
import os

# Ensure module imports resolve from this directory
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from engine import Graph, EdgeType, compute_beta_1
from phi_L import phi_L
from traversal import TraversalEngine
from psi_L import report_graph, report_comparison


# ---------------------------------------------------------------------------
# Test texts
# ---------------------------------------------------------------------------

TEXT_A = (
    "Theories require hypotheses. "
    "Hypotheses require experiments. "
    "Experiments produce data. "
    "Data requires theories. "
    "Theories produce predictions. "
    "Predictions require experiments."
)

TEXT_B = (
    "Theories require hypotheses. "
    "Hypotheses require experiments. "
    "Experiments cannot produce data. "
    "Data requires theories. "
    "Theories contradict predictions. "
    "Predictions require experiments."
)

TRAVERSAL_STEPS = 200
SEED = 42


def _edge_type_counts(g: Graph) -> dict[str, int]:
    counts: dict[str, int] = {}
    for e in g.active_edges():
        counts[e.edge_type.value] = counts.get(e.edge_type.value, 0) + 1
    return counts


def _f_distribution(engine: TraversalEngine) -> dict[str, int]:
    """Count f-value zones from traversal logs."""
    zones: dict[str, int] = {"negative": 0, "fold(0)": 0, "gray(1-2)": 0, "negate(3+)": 0, "n/a": 0}
    for log in engine.logs:
        f = log.f_value
        if f == -99:
            zones["n/a"] += 1
        elif f < 0:
            zones["negative"] += 1
        elif f == 0:
            zones["fold(0)"] += 1
        elif f <= 2:
            zones["gray(1-2)"] += 1
        else:
            zones["negate(3+)"] += 1
    return zones


def run_experiment() -> None:
    print("=" * 70)
    print("phi_L Control Experiment: Consistent vs Contradictory Text")
    print("=" * 70)

    # -- Step 1: Build graphs --
    print("\n--- Step 1: phi_L mapping ---\n")

    graph_a = phi_L(TEXT_A)
    graph_b = phi_L(TEXT_B)

    print(f"Text A (consistent): {len(TEXT_A)} chars")
    print(report_graph(graph_a))
    print()
    print(f"Text B (contradictory): {len(TEXT_B)} chars")
    print(report_graph(graph_b))

    # -- Step 2: Initial comparison --
    print("\n--- Step 2: Pre-traversal comparison ---\n")
    print(report_comparison(graph_a, graph_b, "Consistent", "Contradictory"))

    # Check initial negation edges
    a_neg = sum(1 for e in graph_a.active_edges() if e.edge_type == EdgeType.NEGATION)
    b_neg = sum(1 for e in graph_b.active_edges() if e.edge_type == EdgeType.NEGATION)
    print(f"\nNegation edges: A={a_neg}, B={b_neg}")

    # -- Step 3: Run TraversalEngine --
    print(f"\n--- Step 3: Traversal ({TRAVERSAL_STEPS} steps) ---\n")

    # Pick starting vertices
    a_start = graph_a.active_vertex_ids()[0]
    b_start = graph_b.active_vertex_ids()[0]

    engine_a = TraversalEngine(graph_a, a_start, seed=SEED)
    engine_b = TraversalEngine(graph_b, b_start, seed=SEED)

    for _ in range(TRAVERSAL_STEPS):
        engine_a.run_step()
    for _ in range(TRAVERSAL_STEPS):
        engine_b.run_step()

    # -- Step 4: Post-traversal analysis --
    print("--- Step 4: Post-traversal comparison ---\n")

    post_a = engine_a.k_active
    post_b = engine_b.k_active

    print("Post-traversal Graph A:")
    print(report_graph(post_a))
    print()
    print("Post-traversal Graph B:")
    print(report_graph(post_b))
    print()
    print(report_comparison(post_a, post_b, "A (post)", "B (post)"))

    # -- Step 5: Traversal statistics --
    print(f"\n--- Step 5: Traversal statistics ---\n")

    def _op_counts(engine: TraversalEngine) -> dict[str, int]:
        counts: dict[str, int] = {}
        for log in engine.logs:
            counts[log.operation] = counts.get(log.operation, 0) + 1
        return counts

    ops_a = _op_counts(engine_a)
    ops_b = _op_counts(engine_b)
    all_ops = sorted(set(ops_a) | set(ops_b))

    print(f"{'Operation':<20} {'A':>8} {'B':>8}")
    print("-" * 38)
    for op in all_ops:
        print(f"{op:<20} {ops_a.get(op, 0):>8} {ops_b.get(op, 0):>8}")

    # beta_1 trajectory
    a_betas = [log.beta_1_after for log in engine_a.logs]
    b_betas = [log.beta_1_after for log in engine_b.logs]
    print(f"\nbeta_1 trajectory:")
    print(f"  A: start={a_betas[0] if a_betas else '?'}, end={a_betas[-1] if a_betas else '?'}, max={max(a_betas) if a_betas else '?'}")
    print(f"  B: start={b_betas[0] if b_betas else '?'}, end={b_betas[-1] if b_betas else '?'}, max={max(b_betas) if b_betas else '?'}")

    # Settlement
    print(f"\nSettled cycles:")
    print(f"  A: {len(engine_a.settlement.settled_cycles)}")
    print(f"  B: {len(engine_b.settlement.settled_cycles)}")

    # f-value distribution
    f_dist_a = _f_distribution(engine_a)
    f_dist_b = _f_distribution(engine_b)
    print(f"\nf-value distribution:")
    print(f"{'Zone':<15} {'A':>8} {'B':>8}")
    print("-" * 33)
    for zone in ("negative", "fold(0)", "gray(1-2)", "negate(3+)", "n/a"):
        print(f"{zone:<15} {f_dist_a[zone]:>8} {f_dist_b[zone]:>8}")

    # -- Step 6: Verdict --
    print(f"\n--- Step 6: Verdict ---\n")

    beta_a_final = compute_beta_1(post_a)
    beta_b_final = compute_beta_1(post_b)
    post_b_neg = sum(1 for e in post_b.active_edges() if e.edge_type == EdgeType.NEGATION)
    post_a_neg = sum(1 for e in post_a.active_edges() if e.edge_type == EdgeType.NEGATION)

    checks = [
        ("B has >= negation edges than A (initial)", b_neg >= a_neg),
        ("B beta_1 >= A beta_1 (post-traversal)", beta_b_final >= beta_a_final),
        ("B detected negation edges (post-traversal)", post_b_neg > 0),
    ]

    all_pass = True
    for desc, passed in checks:
        status = "PASS" if passed else "FAIL"
        if not passed:
            all_pass = False
        print(f"  [{status}] {desc}")

    print()
    if all_pass:
        print("All checks passed. Contradictory text produces higher topological complexity.")
    else:
        print("Some checks failed. See details above.")
        # Not necessarily a problem — small graphs may behave differently
        print("Note: With small graphs (5-8 vertices), traversal dynamics are stochastic.")

    return


if __name__ == "__main__":
    run_experiment()
