"""Tests for CSR view consistency with Python Graph.

Verifies that GraphCSR produces identical results to Graph for:
- neighbors (undirected)
- connected_components
- beta_1
Also includes micro-benchmarks for query performance comparison.
"""

from __future__ import annotations

import time
import sys
import os

sys.path.insert(0, os.path.dirname(__file__))

from engine import (
    Graph, Vertex, Edge, EdgeType, VertexStatus,
    compute_beta_1, _connected_components, CONCEPT_EDGE_TYPES,
)
from graph_csr import GraphCSR, from_graph


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_triangle() -> Graph:
    """A -> B -> C -> A (beta_1 = 1)."""
    g = Graph()
    for v in "ABC":
        g = g.add_vertex(Vertex(v))
    g = g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY))
    g = g.add_edge(Edge("B", "C", EdgeType.DEPENDENCY))
    g = g.add_edge(Edge("C", "A", EdgeType.DEPENDENCY))
    return g


def _make_line() -> Graph:
    """A -> B -> C (beta_1 = 0)."""
    g = Graph()
    for v in "ABC":
        g = g.add_vertex(Vertex(v))
    g = g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY))
    g = g.add_edge(Edge("B", "C", EdgeType.DEPENDENCY))
    return g


def _make_two_cycles() -> Graph:
    """A->B->C->A + C->D->E->B (beta_1 = 2)."""
    g = Graph()
    for v in "ABCDE":
        g = g.add_vertex(Vertex(v))
    for s, t in [("A", "B"), ("B", "C"), ("C", "A"), ("C", "D"), ("D", "E"), ("E", "B")]:
        g = g.add_edge(Edge(s, t, EdgeType.DEPENDENCY))
    return g


def _make_self_loop() -> Graph:
    """Single vertex with self-loop (beta_1 = 1)."""
    g = Graph().add_vertex(Vertex("A"))
    g = g.add_edge(Edge("A", "A", EdgeType.DEPENDENCY))
    return g


def _make_mixed_layers() -> Graph:
    """Graph with both concept and material edges."""
    g = Graph()
    for v in "ABCDE":
        g = g.add_vertex(Vertex(v))
    # Concept edges: A->B->C->A (cycle)
    g = g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY))
    g = g.add_edge(Edge("B", "C", EdgeType.NEGATION))
    g = g.add_edge(Edge("C", "A", EdgeType.REFERENCE))
    # Material edges: D->E, A->D
    g = g.add_edge(Edge("D", "E", EdgeType.COOCCURRENCE))
    g = g.add_edge(Edge("A", "D", EdgeType.TRAVERSAL_ASSOCIATION))
    return g


def _make_disconnected() -> Graph:
    """Two disconnected components: {A,B,C} and {D,E}."""
    g = Graph()
    for v in "ABCDE":
        g = g.add_vertex(Vertex(v))
    g = g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY))
    g = g.add_edge(Edge("B", "C", EdgeType.DEPENDENCY))
    g = g.add_edge(Edge("D", "E", EdgeType.DEPENDENCY))
    return g


def _make_with_folded() -> Graph:
    """Graph with a folded vertex (should be excluded from active)."""
    g = Graph()
    g = g.add_vertex(Vertex("A"))
    g = g.add_vertex(Vertex("B"))
    g = g.add_vertex(Vertex("C", status=VertexStatus.FOLDED))
    g = g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY))
    g = g.add_edge(Edge("B", "C", EdgeType.DEPENDENCY))  # C is folded, edge inactive
    return g


def _make_large_graph(n_vertices: int, n_edges_per_vertex: int) -> Graph:
    """Generate a larger graph for benchmarking."""
    import random
    random.seed(42)
    verts = [f"v{i}" for i in range(n_vertices)]
    g = Graph()
    for v in verts:
        g = g.add_vertex(Vertex(v))
    edges = []
    for v in verts:
        targets = random.sample(verts, min(n_edges_per_vertex, n_vertices))
        for t in targets:
            if t != v:
                edges.append(Edge(v, t, EdgeType.DEPENDENCY))
    g = g.add_edges_batch(edges)
    return g


# ---------------------------------------------------------------------------
# Neighbor consistency tests
# ---------------------------------------------------------------------------

def test_neighbors_triangle():
    g = _make_triangle()
    csr = from_graph(g)
    for vid in "ABC":
        py_neighbors = g.neighbor_set(vid)
        csr_neighbors = csr.neighbors_csr(vid)
        assert py_neighbors == csr_neighbors, (
            f"Triangle neighbors mismatch for {vid}: "
            f"py={sorted(py_neighbors)}, csr={sorted(csr_neighbors)}"
        )


def test_neighbors_line():
    g = _make_line()
    csr = from_graph(g)
    for vid in "ABC":
        py_neighbors = g.neighbor_set(vid)
        csr_neighbors = csr.neighbors_csr(vid)
        assert py_neighbors == csr_neighbors, (
            f"Line neighbors mismatch for {vid}: "
            f"py={sorted(py_neighbors)}, csr={sorted(csr_neighbors)}"
        )


def test_neighbors_two_cycles():
    g = _make_two_cycles()
    csr = from_graph(g)
    for vid in "ABCDE":
        py_neighbors = g.neighbor_set(vid)
        csr_neighbors = csr.neighbors_csr(vid)
        assert py_neighbors == csr_neighbors, (
            f"Two-cycles neighbors mismatch for {vid}: "
            f"py={sorted(py_neighbors)}, csr={sorted(csr_neighbors)}"
        )


def test_neighbors_mixed_layers():
    """Material edges should be visible in neighbors (used by traversal)."""
    g = _make_mixed_layers()
    csr = from_graph(g)
    for vid in "ABCDE":
        py_neighbors = g.neighbor_set(vid)
        csr_neighbors = csr.neighbors_csr(vid)
        assert py_neighbors == csr_neighbors, (
            f"Mixed-layer neighbors mismatch for {vid}: "
            f"py={sorted(py_neighbors)}, csr={sorted(csr_neighbors)}"
        )


def test_out_neighbors():
    g = _make_triangle()
    csr = from_graph(g)
    for vid in "ABC":
        py_out = g.out_neighbor_set(vid)
        csr_out = csr.out_neighbors_csr(vid)
        assert py_out == csr_out, (
            f"Out-neighbors mismatch for {vid}: "
            f"py={sorted(py_out)}, csr={sorted(csr_out)}"
        )


def test_in_neighbors():
    g = _make_triangle()
    csr = from_graph(g)
    for vid in "ABC":
        py_in = g.in_neighbor_set(vid)
        csr_in = csr.in_neighbors_csr(vid)
        assert py_in == csr_in, (
            f"In-neighbors mismatch for {vid}: "
            f"py={sorted(py_in)}, csr={sorted(csr_in)}"
        )


def test_neighbors_folded_excluded():
    """Folded vertex should not appear in CSR view."""
    g = _make_with_folded()
    csr = from_graph(g)
    assert csr.n_vertices == 2, f"Expected 2 active vertices, got {csr.n_vertices}"
    assert not csr.index.has("C"), "Folded vertex C should not be in CSR index"
    # B's neighbors: only A (edge B->C is inactive because C is folded)
    py_neighbors = g.neighbor_set("B")
    csr_neighbors = csr.neighbors_csr("B")
    assert py_neighbors == csr_neighbors, (
        f"Folded neighbors mismatch for B: py={sorted(py_neighbors)}, csr={sorted(csr_neighbors)}"
    )


def test_neighbors_nonexistent():
    """Querying a vertex not in the graph returns empty set."""
    g = _make_triangle()
    csr = from_graph(g)
    assert csr.neighbors_csr("Z") == set()


# ---------------------------------------------------------------------------
# Connected components consistency tests
# ---------------------------------------------------------------------------

def test_connected_components_empty():
    g = Graph()
    csr = from_graph(g)
    assert csr.connected_components_csr() == 0


def test_connected_components_single():
    g = Graph().add_vertex(Vertex("A"))
    csr = from_graph(g)
    assert csr.connected_components_csr() == 1


def test_connected_components_triangle():
    g = _make_triangle()
    csr = from_graph(g)
    # Triangle is one connected component
    py_cc = _connected_components(
        g.active_vertex_ids(),
        g.undirected_active_edges(),
    )
    assert csr.connected_components_csr() == py_cc == 1


def test_connected_components_disconnected():
    g = _make_disconnected()
    csr = from_graph(g)
    py_cc = _connected_components(
        g.active_vertex_ids(),
        g.undirected_active_edges(),
    )
    assert csr.connected_components_csr() == py_cc == 2


def test_connected_components_isolated_vertex():
    """An isolated vertex (no concept edges) is its own component."""
    g = Graph()
    for v in "AB":
        g = g.add_vertex(Vertex(v))
    # No edges at all
    csr = from_graph(g)
    assert csr.connected_components_csr() == 2


# ---------------------------------------------------------------------------
# beta_1 consistency tests
# ---------------------------------------------------------------------------

def test_beta1_empty():
    g = Graph()
    csr = from_graph(g)
    assert csr.beta_1_csr() == compute_beta_1(g) == 0


def test_beta1_single():
    g = Graph().add_vertex(Vertex("A"))
    csr = from_graph(g)
    assert csr.beta_1_csr() == compute_beta_1(g) == 0


def test_beta1_line():
    g = _make_line()
    csr = from_graph(g)
    py_b1 = compute_beta_1(g)
    csr_b1 = csr.beta_1_csr()
    assert py_b1 == csr_b1 == 0, f"Line beta_1: py={py_b1}, csr={csr_b1}"


def test_beta1_triangle():
    g = _make_triangle()
    csr = from_graph(g)
    py_b1 = compute_beta_1(g)
    csr_b1 = csr.beta_1_csr()
    assert py_b1 == csr_b1 == 1, f"Triangle beta_1: py={py_b1}, csr={csr_b1}"


def test_beta1_two_cycles():
    g = _make_two_cycles()
    csr = from_graph(g)
    py_b1 = compute_beta_1(g)
    csr_b1 = csr.beta_1_csr()
    assert py_b1 == csr_b1 == 2, f"Two-cycles beta_1: py={py_b1}, csr={csr_b1}"


def test_beta1_self_loop():
    g = _make_self_loop()
    csr = from_graph(g)
    py_b1 = compute_beta_1(g)
    csr_b1 = csr.beta_1_csr()
    assert py_b1 == csr_b1 == 1, f"Self-loop beta_1: py={py_b1}, csr={csr_b1}"


def test_beta1_mixed_layers():
    """Material edges should NOT affect beta_1."""
    g = _make_mixed_layers()
    csr = from_graph(g)
    py_b1 = compute_beta_1(g)
    csr_b1 = csr.beta_1_csr()
    assert py_b1 == csr_b1, f"Mixed-layer beta_1: py={py_b1}, csr={csr_b1}"


def test_beta1_disconnected():
    g = _make_disconnected()
    csr = from_graph(g)
    py_b1 = compute_beta_1(g)
    csr_b1 = csr.beta_1_csr()
    assert py_b1 == csr_b1, f"Disconnected beta_1: py={py_b1}, csr={csr_b1}"


def test_beta1_with_folded():
    g = _make_with_folded()
    csr = from_graph(g)
    py_b1 = compute_beta_1(g)
    csr_b1 = csr.beta_1_csr()
    assert py_b1 == csr_b1, f"Folded beta_1: py={py_b1}, csr={csr_b1}"


def test_beta1_all_edge_types():
    """Test with every concept edge type to verify none are accidentally excluded."""
    g = Graph()
    for v in "ABCDEFG":
        g = g.add_vertex(Vertex(v))
    # Create a cycle using different edge types
    g = g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY))
    g = g.add_edge(Edge("B", "C", EdgeType.NEGATION))
    g = g.add_edge(Edge("C", "D", EdgeType.SUBLATION))
    g = g.add_edge(Edge("D", "E", EdgeType.REFERENCE))
    g = g.add_edge(Edge("E", "F", EdgeType.FOLD))
    g = g.add_edge(Edge("F", "G", EdgeType.ARTICULATED))
    g = g.add_edge(Edge("G", "A", EdgeType.DEPENDENCY))
    csr = from_graph(g)
    py_b1 = compute_beta_1(g)
    csr_b1 = csr.beta_1_csr()
    assert py_b1 == csr_b1 == 1, (
        f"All-types cycle beta_1: py={py_b1}, csr={csr_b1}"
    )


# ---------------------------------------------------------------------------
# Concept-only neighbor tests
# ---------------------------------------------------------------------------

def test_concept_neighbors_exclude_material():
    """concept_neighbors_csr should exclude material edges."""
    g = _make_mixed_layers()
    csr = from_graph(g)
    # A has concept edges to B and from C, plus material edge to D
    concept_nbrs = csr.concept_neighbors_csr("A")
    assert "D" not in concept_nbrs, "Material neighbor D should not be in concept neighbors"
    assert "B" in concept_nbrs, "Concept neighbor B should be present"
    assert "C" in concept_nbrs, "Concept neighbor C should be present"


# ---------------------------------------------------------------------------
# Benchmark (only runs when invoked with --benchmark flag)
# ---------------------------------------------------------------------------

def _benchmark():
    """Compare CSR vs Python dict query performance."""
    sizes = [100, 500, 1000, 2000]
    print(f"\n{'='*70}")
    print(f"{'Benchmark: CSR vs Python dict query performance':^70}")
    print(f"{'='*70}")
    print(f"{'N':>6} | {'edges':>7} | {'py_neighbors':>14} | {'csr_neighbors':>14} | {'py_beta1':>10} | {'csr_beta1':>10}")
    print(f"{'-'*6}-+-{'-'*7}-+-{'-'*14}-+-{'-'*14}-+-{'-'*10}-+-{'-'*10}")

    for n in sizes:
        g = _make_large_graph(n, min(5, n))
        n_edges = len(g.edges)
        active_ids = g.active_vertex_ids()

        # Build CSR (timed)
        t0 = time.perf_counter()
        csr = from_graph(g)
        build_time = time.perf_counter() - t0

        # Python neighbors (all vertices)
        t0 = time.perf_counter()
        for vid in active_ids:
            g.neighbor_set(vid)
        py_nbr_time = time.perf_counter() - t0

        # CSR neighbors (all vertices)
        t0 = time.perf_counter()
        for vid in active_ids:
            csr.neighbors_csr(vid)
        csr_nbr_time = time.perf_counter() - t0

        # Python beta_1
        t0 = time.perf_counter()
        py_b1 = compute_beta_1(g)
        py_b1_time = time.perf_counter() - t0

        # CSR beta_1
        t0 = time.perf_counter()
        csr_b1 = csr.beta_1_csr()
        csr_b1_time = time.perf_counter() - t0

        assert py_b1 == csr_b1, f"beta_1 mismatch at N={n}: py={py_b1}, csr={csr_b1}"

        print(
            f"{n:>6} | {n_edges:>7} | {py_nbr_time*1000:>11.2f} ms | {csr_nbr_time*1000:>11.2f} ms | "
            f"{py_b1_time*1000:>7.2f} ms | {csr_b1_time*1000:>7.2f} ms"
        )

    print(f"{'='*70}\n")


if __name__ == "__main__":
    import pytest
    args = [__file__, "-v"]
    if "--benchmark" in sys.argv:
        _benchmark()
    else:
        pytest.main(args)
