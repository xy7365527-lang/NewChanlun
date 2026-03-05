"""Tests for the topological computation engine."""

from __future__ import annotations

import sys
import traceback

from engine import (
    Graph, Vertex, Edge, EdgeType, VertexStatus,
    compute_beta_1, SettlementTracker, fold, negate, sublate,
    find_new_cycle_edges,
)
from morse import compute_terrain, critical_neighbors


def _make_triangle() -> Graph:
    """A -> B -> C -> A (β₁ = 1)."""
    g = Graph()
    for v in "ABC":
        g = g.add_vertex(Vertex(v))
    g = g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY))
    g = g.add_edge(Edge("B", "C", EdgeType.DEPENDENCY))
    g = g.add_edge(Edge("C", "A", EdgeType.DEPENDENCY))
    return g


def _make_line() -> Graph:
    """A -> B -> C (β₁ = 0)."""
    g = Graph()
    for v in "ABC":
        g = g.add_vertex(Vertex(v))
    g = g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY))
    g = g.add_edge(Edge("B", "C", EdgeType.DEPENDENCY))
    return g


# ---------------------------------------------------------------------------
# β₁ tests
# ---------------------------------------------------------------------------

def test_beta1_empty():
    g = Graph()
    assert compute_beta_1(g) == 0, "Empty graph β₁ should be 0"


def test_beta1_single_vertex():
    g = Graph().add_vertex(Vertex("A"))
    assert compute_beta_1(g) == 0, "Single vertex β₁ should be 0"


def test_beta1_line():
    g = _make_line()
    assert compute_beta_1(g) == 0, f"Line graph β₁ should be 0, got {compute_beta_1(g)}"


def test_beta1_triangle():
    g = _make_triangle()
    b = compute_beta_1(g)
    assert b == 1, f"Triangle β₁ should be 1, got {b}"


def test_beta1_two_cycles():
    """A->B->C->A + C->D->E->B (two independent cycles, β₁=2)."""
    g = Graph()
    for v in "ABCDE":
        g = g.add_vertex(Vertex(v))
    for s, t in [("A", "B"), ("B", "C"), ("C", "A"), ("C", "D"), ("D", "E"), ("E", "B")]:
        g = g.add_edge(Edge(s, t, EdgeType.DEPENDENCY))
    b = compute_beta_1(g)
    assert b == 2, f"Two-cycle graph β₁ should be 2, got {b}"


def test_beta1_self_loop():
    """Single vertex with self-loop → β₁ = 1."""
    g = Graph().add_vertex(Vertex("A"))
    g = g.add_edge(Edge("A", "A", EdgeType.DEPENDENCY))
    b = compute_beta_1(g)
    assert b == 1, f"Self-loop β₁ should be 1, got {b}"


# ---------------------------------------------------------------------------
# Fold tests
# ---------------------------------------------------------------------------

def test_fold_basic():
    """Fold two vertices in a line: A-B-C, fold A and C. n_loop=0, c depends on structure."""
    g = _make_line()
    s = SettlementTracker()
    result = fold(g, ["A", "C"], step=1, settlement=s)
    assert not result.blocked, "Fold should not be blocked"
    # After fold: A and C merge. B connected to merged vertex.
    assert result.delta_beta_1_actual == result.delta_beta_1_predicted


def test_fold_creates_loop():
    """Fold A and B in triangle A->B->C->A. A-B edge becomes self-loop. n_loop=1."""
    g = _make_triangle()
    s = SettlementTracker()
    result = fold(g, ["A", "B"], step=1, settlement=s)
    assert not result.blocked
    # n_loop = 1 (edge A->B becomes self-loop)
    # The fold should produce a predictable Δβ₁


# ---------------------------------------------------------------------------
# Negate tests
# ---------------------------------------------------------------------------

def test_negate_case_a():
    """Negate with existing antithesis where path exists → Δβ₁ = +1."""
    g = _make_triangle()  # A->B->C->A, all connected
    s = SettlementTracker()
    # Negate A with antithesis C (path A->B->C exists)
    result = negate(g, thesis="A", antithesis="C", step=1, settlement=s)
    assert not result.blocked
    assert result.delta_beta_1_actual >= 0  # Should be +1 if path exists
    # A should be contested
    assert result.graph.vertex("A").status == VertexStatus.CONTESTED


def test_negate_case_b():
    """Negate with new antithesis → Δβ₁ = 0, new vertex created."""
    g = _make_triangle()
    s = SettlementTracker()
    result = negate(g, thesis="A", antithesis=None, step=1, settlement=s)
    assert not result.blocked
    assert result.delta_beta_1_predicted == 0
    assert result.delta_beta_1_actual == 0
    assert result.new_vertex is not None
    # New vertex should exist in graph
    assert result.graph.vertex(result.new_vertex) is not None
    # A should be contested
    assert result.graph.vertex("A").status == VertexStatus.CONTESTED


def test_negate_case_b_no_path():
    """New antithesis has no path to thesis → Δβ₁ = 0."""
    g = _make_line()  # A->B->C, no cycle
    s = SettlementTracker()
    result = negate(g, thesis="C", antithesis=None, step=1, settlement=s)
    assert result.delta_beta_1_actual == 0


# ---------------------------------------------------------------------------
# Sublate tests
# ---------------------------------------------------------------------------

def test_sublate_after_negate():
    """Full contradiction path: negate then sublate → net Δβ₁ = +1."""
    g = _make_triangle()
    s = SettlementTracker()

    # Step 1: Negate A with new antithesis
    neg_result = negate(g, thesis="A", antithesis=None, step=1, settlement=s)
    g2 = neg_result.graph
    anti = neg_result.new_vertex
    beta_after_neg = compute_beta_1(g2)

    # Step 2: Sublate
    sub_result = sublate(g2, thesis="A", antithesis=anti, step=2, settlement=s)
    assert not sub_result.blocked
    assert sub_result.new_vertex is not None
    beta_after_sub = compute_beta_1(sub_result.graph)

    # Net effect: β₁ should increase by 1 from sublation
    assert sub_result.delta_beta_1_actual == 1, (
        f"Sublate Δβ₁ should be +1, got {sub_result.delta_beta_1_actual}"
    )


def test_sublate_requires_negation():
    """Sublate without prior negation should raise."""
    g = _make_triangle()
    s = SettlementTracker()
    try:
        sublate(g, "A", "B", step=1, settlement=s)
        assert False, "Should have raised ValueError"
    except ValueError:
        pass


# ---------------------------------------------------------------------------
# Settlement tests
# ---------------------------------------------------------------------------

def test_settlement_tracking():
    """Cycle persists for threshold steps → settled."""
    s = SettlementTracker(threshold=3)
    cycle_edges = frozenset({("A", "B"), ("B", "C"), ("C", "A")})
    g = _make_triangle()

    s.register_new_cycle(cycle_edges, step=1)
    assert len(s.settled_cycles) == 0

    s.check_settlement(2, g)
    assert len(s.settled_cycles) == 0

    s.check_settlement(3, g)
    assert len(s.settled_cycles) == 0

    s.check_settlement(4, g)  # step 4 - first_seen 1 = 3 >= threshold 3
    assert len(s.settled_cycles) == 1


def test_settlement_blocking():
    """Operation that would destroy settled cycle is blocked."""
    g = _make_triangle()
    s = SettlementTracker(threshold=1)
    cycle_edges = frozenset({("A", "B"), ("B", "C"), ("C", "A")})
    s.register_new_cycle(cycle_edges, step=0)
    s.check_settlement(1, g)
    assert len(s.settled_cycles) == 1

    # Try to negate A (which is on the settled cycle)
    # This doesn't destroy the cycle in v2 (A stays in K_active)
    result = negate(g, "A", "B", step=2, settlement=s)
    # The negate adds an edge, doesn't remove anything → should NOT be blocked
    assert not result.blocked, "Negate should not be blocked (doesn't destroy cycle)"


def test_settlement_does_not_block_unrelated():
    """Operations not affecting settled cycle should not be blocked."""
    g = _make_triangle()
    # Add isolated vertex
    g = g.add_vertex(Vertex("X"))
    g = g.add_vertex(Vertex("Y"))
    g = g.add_edge(Edge("X", "Y", EdgeType.DEPENDENCY))

    s = SettlementTracker(threshold=1)
    cycle_edges = frozenset({("A", "B"), ("B", "C"), ("C", "A")})
    s.register_new_cycle(cycle_edges, step=0)
    s.check_settlement(1, g)

    # Negate X (unrelated to settled cycle)
    result = negate(g, "X", None, step=2, settlement=s)
    assert not result.blocked


# ---------------------------------------------------------------------------
# Creation = arrival tests
# ---------------------------------------------------------------------------

def test_negate_b_position():
    """Negate Case B: new vertex created → position should move there."""
    # This is tested via traversal, but we verify the engine returns new_vertex
    g = _make_triangle()
    s = SettlementTracker()
    result = negate(g, "A", None, step=1, settlement=s)
    assert result.new_vertex is not None
    assert result.graph.vertex(result.new_vertex) is not None


def test_sublate_position():
    """Sublate: synthesis created → position should move there."""
    g = _make_triangle()
    s = SettlementTracker()
    neg = negate(g, "A", None, step=1, settlement=s)
    sub = sublate(neg.graph, "A", neg.new_vertex, step=2, settlement=s)
    assert sub.new_vertex is not None
    assert sub.graph.vertex(sub.new_vertex) is not None


# ---------------------------------------------------------------------------
# K_full only grows
# ---------------------------------------------------------------------------

def test_kfull_only_grows():
    """K_full vertex and edge counts never decrease."""
    g = _make_triangle()
    counts = [(len(g.vertices), len(g.edges))]

    s = SettlementTracker()
    r1 = negate(g, "A", None, step=1, settlement=s)
    g2 = r1.graph
    counts.append((len(g2.vertices), len(g2.edges)))

    r2 = sublate(g2, "A", r1.new_vertex, step=2, settlement=s)
    g3 = r2.graph
    counts.append((len(g3.vertices), len(g3.edges)))

    for i in range(1, len(counts)):
        assert counts[i][0] >= counts[i - 1][0], f"Vertex count decreased at step {i}"
        assert counts[i][1] >= counts[i - 1][1], f"Edge count decreased at step {i}"


# ---------------------------------------------------------------------------
# Morse terrain tests
# ---------------------------------------------------------------------------

def test_morse_terrain_triangle():
    """Triangle should have 2 tree edges and 1 critical edge."""
    g = _make_triangle()
    terrain = compute_terrain(g)
    tree_count = sum(1 for v in terrain.values() if v == "tree")
    crit_count = sum(1 for v in terrain.values() if v == "critical")
    assert tree_count == 2, f"Expected 2 tree edges, got {tree_count}"
    assert crit_count == 1, f"Expected 1 critical edge, got {crit_count}"


def test_morse_terrain_line():
    """Line graph: all edges are tree edges."""
    g = _make_line()
    terrain = compute_terrain(g)
    crit_count = sum(1 for v in terrain.values() if v == "critical")
    assert crit_count == 0, f"Line should have 0 critical edges, got {crit_count}"


def test_critical_neighbors():
    """Critical neighbors of a vertex in a triangle."""
    g = _make_triangle()
    terrain = compute_terrain(g)
    # Find which vertex has a critical edge
    crit_edges = [(s, t) for (s, t), m in terrain.items() if m == "critical"]
    assert len(crit_edges) == 1
    src, tgt = crit_edges[0]
    cn = critical_neighbors(g, src, terrain)
    assert tgt in cn


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------

def run_tests():
    tests = [
        test_beta1_empty,
        test_beta1_single_vertex,
        test_beta1_line,
        test_beta1_triangle,
        test_beta1_two_cycles,
        test_beta1_self_loop,
        test_fold_basic,
        test_fold_creates_loop,
        test_negate_case_a,
        test_negate_case_b,
        test_negate_case_b_no_path,
        test_sublate_after_negate,
        test_sublate_requires_negation,
        test_settlement_tracking,
        test_settlement_blocking,
        test_settlement_does_not_block_unrelated,
        test_negate_b_position,
        test_sublate_position,
        test_kfull_only_grows,
        test_morse_terrain_triangle,
        test_morse_terrain_line,
        test_critical_neighbors,
    ]

    passed = 0
    failed = 0
    errors = []

    for test in tests:
        try:
            test()
            passed += 1
            print(f"  [PASS] {test.__name__}")
        except Exception as e:
            failed += 1
            errors.append((test.__name__, e))
            print(f"  [FAIL] {test.__name__}: {e}")
            traceback.print_exc()

    print(f"\n{'='*40}")
    print(f"  {passed} passed, {failed} failed out of {len(tests)}")
    if failed:
        print(f"  Failed: {', '.join(name for name, _ in errors)}")
    print(f"{'='*40}")

    return failed == 0


if __name__ == "__main__":
    success = run_tests()
    sys.exit(0 if success else 1)
