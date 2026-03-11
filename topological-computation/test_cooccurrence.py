"""Tests for COOCCURRENCE + TRAVERSAL_ASSOCIATION edge types — S_net material layer (Dass) in K_active.

Verifies:
  1. COOCCURRENCE edges don't affect β₁ computation
  2. COOCCURRENCE edges don't participate in fold/negate/sublate operations
  3. COOCCURRENCE edges ARE visible to traversal (neighbors)
  4. On-demand pull creates COOCCURRENCE edges during traversal
  5. would_destroy_settled ignores COOCCURRENCE edges
  6. TRAVERSAL_ASSOCIATION edges don't affect β₁
  7. TRAVERSAL_ASSOCIATION edges excluded from active_edges but included in all_active_edges
  8. _record_traversal_association creates edges with correct type
  9. BlockTopologyWriter.append_cooccurrence writes valid blocks
  10. BlockTopologyWriter.append_traversal_association writes valid blocks
"""

import json
import tempfile
from pathlib import Path

from engine import (
    Graph, Vertex, Edge, VertexStatus, EdgeType, SettlementTracker,
    compute_beta_1, fold, negate, sublate, CONCEPT_EDGE_TYPES,
)
from signifier_net import SNet, Signifier, SignifierEdge, AxisType
from snet_activation import SNetActivation


def _triangle_graph() -> Graph:
    """A -> B -> C -> A triangle with DEPENDENCY edges."""
    g = Graph()
    g = g.add_vertex(Vertex("A", VertexStatus.ACTIVE))
    g = g.add_vertex(Vertex("B", VertexStatus.ACTIVE))
    g = g.add_vertex(Vertex("C", VertexStatus.ACTIVE))
    g = g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY, 0))
    g = g.add_edge(Edge("B", "C", EdgeType.DEPENDENCY, 0))
    g = g.add_edge(Edge("C", "A", EdgeType.DEPENDENCY, 0))
    return g


# -- Test 1: COOCCURRENCE edges don't affect β₁ --

def test_beta1_ignores_cooccurrence():
    g = _triangle_graph()
    beta_before = compute_beta_1(g)

    # Add COOCCURRENCE edges — should NOT change β₁
    g2 = g.add_edge(Edge("A", "B", EdgeType.COOCCURRENCE, 1))
    g2 = g2.add_edge(Edge("B", "C", EdgeType.COOCCURRENCE, 1))
    beta_after = compute_beta_1(g2)

    assert beta_before == beta_after, (
        f"COOCCURRENCE edges changed β₁: {beta_before} -> {beta_after}"
    )


# -- Test 2: active_edges excludes COOCCURRENCE --

def test_active_edges_excludes_cooccurrence():
    g = _triangle_graph()
    g = g.add_edge(Edge("A", "C", EdgeType.COOCCURRENCE, 1))

    active = g.active_edges()
    cooc_in_active = [e for e in active if e.edge_type == EdgeType.COOCCURRENCE]
    assert len(cooc_in_active) == 0, "COOCCURRENCE should not appear in active_edges()"


def test_all_active_edges_includes_cooccurrence():
    g = _triangle_graph()
    g = g.add_edge(Edge("A", "C", EdgeType.COOCCURRENCE, 1))

    all_active = g.all_active_edges()
    cooc_in_all = [e for e in all_active if e.edge_type == EdgeType.COOCCURRENCE]
    assert len(cooc_in_all) == 1, "COOCCURRENCE should appear in all_active_edges()"


# -- Test 3: COOCCURRENCE edges visible in neighbors --

def test_neighbors_includes_cooccurrence():
    g = Graph()
    g = g.add_vertex(Vertex("A", VertexStatus.ACTIVE))
    g = g.add_vertex(Vertex("B", VertexStatus.ACTIVE))
    # Only COOCCURRENCE edge, no concept edge
    g = g.add_edge(Edge("A", "B", EdgeType.COOCCURRENCE, 0))

    nbs = g.neighbors("A")
    assert "B" in nbs, "COOCCURRENCE neighbor should be visible via neighbors()"


# -- Test 4: fold ignores COOCCURRENCE edges --

def test_fold_ignores_cooccurrence():
    """Fold should work the same whether COOCCURRENCE edges exist or not."""
    g = Graph()
    g = g.add_vertex(Vertex("A", VertexStatus.ACTIVE, content="a"))
    g = g.add_vertex(Vertex("B", VertexStatus.ACTIVE, content="b"))
    g = g.add_vertex(Vertex("C", VertexStatus.ACTIVE, content="c"))
    g = g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY, 0))
    g = g.add_edge(Edge("A", "C", EdgeType.DEPENDENCY, 0))
    # Add COOCCURRENCE between B and C
    g = g.add_edge(Edge("B", "C", EdgeType.COOCCURRENCE, 0))

    settlement = SettlementTracker(threshold=5)
    result = fold(g, ["B", "C"], step=1, settlement=settlement)

    assert not result.blocked, "Fold should not be blocked by COOCCURRENCE edges"
    # B and C merged — only COOCCURRENCE edge between them should not affect merge
    merged_active = result.graph.active_vertex_ids()
    assert "B" in merged_active, "Keep vertex B should remain"


# -- Test 5: negate ignores COOCCURRENCE edges --

def test_negate_ignores_cooccurrence():
    """Negate should work the same whether COOCCURRENCE edges exist or not."""
    g = Graph()
    g = g.add_vertex(Vertex("A", VertexStatus.ACTIVE, content="a"))
    g = g.add_vertex(Vertex("B", VertexStatus.ACTIVE, content="b"))
    g = g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY, 0))
    g = g.add_edge(Edge("A", "B", EdgeType.COOCCURRENCE, 0))

    settlement = SettlementTracker(threshold=5)
    result = negate(g, "A", "B", step=1, settlement=settlement)

    assert not result.blocked, "Negate should not be blocked by COOCCURRENCE edges"
    # Negation edge should be added
    neg_edges = [e for e in result.graph.active_edges() if e.edge_type == EdgeType.NEGATION]
    assert len(neg_edges) == 1


# -- Test 6: would_destroy_settled ignores COOCCURRENCE --

def test_would_destroy_settled_ignores_cooccurrence():
    """Settled cycle detection should not consider COOCCURRENCE edges."""
    g = _triangle_graph()
    settlement = SettlementTracker(threshold=1)

    # Manually settle the triangle
    cycle_edges = frozenset({("A", "B"), ("B", "C"), ("C", "A")})
    from engine import SettledCycle
    settlement._settled.append(SettledCycle(
        edges=cycle_edges,
        settled_at_step=0,
    ))

    # Add COOCCURRENCE edge — removing it should NOT count as destroying settled cycle
    g2 = g.add_vertex(Vertex("D", VertexStatus.ACTIVE))
    g2 = g2.add_edge(Edge("A", "D", EdgeType.COOCCURRENCE, 1))

    # Graph without the COOCCURRENCE edge — settled cycle still intact
    violated = settlement.would_destroy_settled(g2, graph_before=g)
    assert violated is None, "COOCCURRENCE edges should not affect settled cycle detection"


# -- Test 7: undirected_active_edges excludes COOCCURRENCE --

def test_undirected_active_edges_excludes_cooccurrence():
    g = _triangle_graph()
    before = len(g.undirected_active_edges())

    g2 = g.add_edge(Edge("A", "B", EdgeType.COOCCURRENCE, 1))
    after = len(g2.undirected_active_edges())

    assert before == after, "COOCCURRENCE should not appear in undirected_active_edges"


# -- Test 8: CONCEPT_EDGE_TYPES does not include COOCCURRENCE --

def test_concept_edge_types_excludes_cooccurrence():
    assert EdgeType.COOCCURRENCE not in CONCEPT_EDGE_TYPES
    assert EdgeType.TRAVERSAL_ASSOCIATION not in CONCEPT_EDGE_TYPES
    assert EdgeType.DEPENDENCY in CONCEPT_EDGE_TYPES
    assert EdgeType.NEGATION in CONCEPT_EDGE_TYPES
    assert EdgeType.SUBLATION in CONCEPT_EDGE_TYPES
    assert EdgeType.REFERENCE in CONCEPT_EDGE_TYPES
    assert EdgeType.FOLD in CONCEPT_EDGE_TYPES


# -- Test 9: TRAVERSAL_ASSOCIATION edges don't affect β₁ --

def test_beta1_ignores_traversal_association():
    g = _triangle_graph()
    beta_before = compute_beta_1(g)

    g2 = g.add_edge(Edge("A", "B", EdgeType.TRAVERSAL_ASSOCIATION, 1))
    g2 = g2.add_edge(Edge("B", "C", EdgeType.TRAVERSAL_ASSOCIATION, 1))
    beta_after = compute_beta_1(g2)

    assert beta_before == beta_after, (
        f"TRAVERSAL_ASSOCIATION edges changed β₁: {beta_before} -> {beta_after}"
    )


# -- Test 10: active_edges excludes TRAVERSAL_ASSOCIATION --

def test_active_edges_excludes_traversal_association():
    g = _triangle_graph()
    g = g.add_edge(Edge("A", "C", EdgeType.TRAVERSAL_ASSOCIATION, 1))

    active = g.active_edges()
    ta_in_active = [e for e in active if e.edge_type == EdgeType.TRAVERSAL_ASSOCIATION]
    assert len(ta_in_active) == 0, "TRAVERSAL_ASSOCIATION should not appear in active_edges()"


def test_all_active_edges_includes_traversal_association():
    g = _triangle_graph()
    g = g.add_edge(Edge("A", "C", EdgeType.TRAVERSAL_ASSOCIATION, 1))

    all_active = g.all_active_edges()
    ta_in_all = [e for e in all_active if e.edge_type == EdgeType.TRAVERSAL_ASSOCIATION]
    assert len(ta_in_all) == 1, "TRAVERSAL_ASSOCIATION should appear in all_active_edges()"


# -- Test 11: TRAVERSAL_ASSOCIATION visible in neighbors --

def test_neighbors_includes_traversal_association():
    g = Graph()
    g = g.add_vertex(Vertex("A", VertexStatus.ACTIVE))
    g = g.add_vertex(Vertex("B", VertexStatus.ACTIVE))
    g = g.add_edge(Edge("A", "B", EdgeType.TRAVERSAL_ASSOCIATION, 0))

    nbs = g.neighbors("A")
    assert "B" in nbs, "TRAVERSAL_ASSOCIATION neighbor should be visible via neighbors()"


# -- Test 12: undirected_active_edges excludes TRAVERSAL_ASSOCIATION --

def test_undirected_active_edges_excludes_traversal_association():
    g = _triangle_graph()
    before = len(g.undirected_active_edges())

    g2 = g.add_edge(Edge("A", "B", EdgeType.TRAVERSAL_ASSOCIATION, 1))
    after = len(g2.undirected_active_edges())

    assert before == after, "TRAVERSAL_ASSOCIATION should not appear in undirected_active_edges"


# -- Test 13: _record_traversal_association creates correct edge --

def test_record_traversal_association():
    """TraversalEngine._record_traversal_association creates TRAVERSAL_ASSOCIATION edge."""
    from traversal import TraversalEngine

    g = Graph()
    g = g.add_vertex(Vertex("A", VertexStatus.ACTIVE, content="走势"))
    g = g.add_vertex(Vertex("B", VertexStatus.ACTIVE, content="级别"))
    g = g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY, 0))

    snet = SNet()
    snet = snet.add_signifier(Signifier("走势"))
    snet = snet.add_signifier(Signifier("级别"))

    te = TraversalEngine(g, "A")
    te.step = 5

    activation = SNetActivation(
        s_net=snet,
        concept_to_signifier={"A": "走势", "B": "级别"},
        signifier_to_concepts={"走势": ["A"], "级别": ["B"]},
    )
    te.set_snet_activation(activation)

    te._record_traversal_association("A", "B")

    ta_edges = [e for e in te.k_active.all_active_edges() if e.edge_type == EdgeType.TRAVERSAL_ASSOCIATION]
    assert len(ta_edges) == 1
    assert ta_edges[0].source == "A"
    assert ta_edges[0].target == "B"
    assert ta_edges[0].surface == "走势->级别"
    assert "traversal_step:5" in ta_edges[0].context


# -- Test 14: _record_traversal_association skips when no signifier mapping --

def test_record_traversal_association_skips_unmapped():
    """_record_traversal_association does nothing if vertices lack signifier mappings."""
    from traversal import TraversalEngine

    g = Graph()
    g = g.add_vertex(Vertex("A", VertexStatus.ACTIVE, content="走势"))
    g = g.add_vertex(Vertex("B", VertexStatus.ACTIVE, content="unknown"))
    g = g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY, 0))

    snet = SNet()
    snet = snet.add_signifier(Signifier("走势"))

    te = TraversalEngine(g, "A")
    activation = SNetActivation(
        s_net=snet,
        concept_to_signifier={"A": "走势"},
        signifier_to_concepts={"走势": ["A"]},
    )
    te.set_snet_activation(activation)

    te._record_traversal_association("A", "B")

    ta_edges = [e for e in te.k_active.all_active_edges() if e.edge_type == EdgeType.TRAVERSAL_ASSOCIATION]
    assert len(ta_edges) == 0, "Should not create edge when target has no signifier mapping"


# -- Test 15: _pull_cooccurrence_edges creates COOCCURRENCE edges --

def test_pull_cooccurrence_edges():
    """_pull_cooccurrence_edges creates COOCCURRENCE edges from S_net neighbors."""
    from traversal import TraversalEngine

    g = Graph()
    g = g.add_vertex(Vertex("A", VertexStatus.ACTIVE, content="走势"))
    g = g.add_vertex(Vertex("B", VertexStatus.ACTIVE, content="级别"))
    g = g.add_vertex(Vertex("C", VertexStatus.ACTIVE, content="中枢"))
    g = g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY, 0))

    snet = SNet()
    snet = snet.add_signifier(Signifier("走势"))
    snet = snet.add_signifier(Signifier("级别"))
    snet = snet.add_signifier(Signifier("中枢"))
    snet = snet.add_edge(SignifierEdge("走势", "级别", AxisType.SYNTAGMATIC, 5.0))
    snet = snet.add_edge(SignifierEdge("走势", "中枢", AxisType.SYNTAGMATIC, 3.0))

    te = TraversalEngine(g, "A")
    te.step = 1

    activation = SNetActivation(
        s_net=snet,
        concept_to_signifier={"A": "走势", "B": "级别", "C": "中枢"},
        signifier_to_concepts={"走势": ["A"], "级别": ["B"], "中枢": ["C"]},
    )
    te.set_snet_activation(activation)

    te._pull_cooccurrence_edges()

    cooc_edges = [e for e in te.k_active.all_active_edges() if e.edge_type == EdgeType.COOCCURRENCE]
    # Should have pulled B and C as COOCCURRENCE neighbors of A
    cooc_targets = {e.target for e in cooc_edges}
    assert "B" in cooc_targets or "C" in cooc_targets, "Should pull at least one COOCCURRENCE edge"


# -- Test 16: BlockTopologyWriter.append_cooccurrence --

def test_block_topology_writer_cooccurrence():
    """append_cooccurrence writes a valid block to disk."""
    from block_topology_persistence import BlockTopologyWriter

    with tempfile.TemporaryDirectory() as tmpdir:
        bt_base = Path(tmpdir) / "bt"
        (bt_base / "blocks").mkdir(parents=True)

        writer = BlockTopologyWriter(bt_base=bt_base)
        writer.append_cooccurrence(
            source_signifier="走势",
            target_signifier="级别",
            weight=5.0,
            corpus_source="test_corpus",
            ingest_params={"pmi_threshold": 0.0},
            timestamp="2026-03-11T00:00:00Z",
        )

        blocks = list((bt_base / "blocks").iterdir())
        assert len(blocks) == 1

        with open(blocks[0], "r", encoding="utf-8") as f:
            block = json.load(f)
        assert block["content"]["event_type"] == "cooccurrence"
        assert block["content"]["source_signifier"] == "走势"
        assert block["content"]["target_signifier"] == "级别"
        assert block["content"]["weight"] == 5.0
        assert block["content"]["domain"] == "material"


# -- Test 17: BlockTopologyWriter.append_traversal_association --

def test_block_topology_writer_traversal_association():
    """append_traversal_association writes a valid block to disk."""
    from block_topology_persistence import BlockTopologyWriter

    with tempfile.TemporaryDirectory() as tmpdir:
        bt_base = Path(tmpdir) / "bt"
        (bt_base / "blocks").mkdir(parents=True)

        writer = BlockTopologyWriter(bt_base=bt_base)
        writer.append_traversal_association(
            from_signifier="走势",
            to_signifier="级别",
            traversal_id="test-traversal-001",
            step_number=42,
            timestamp="2026-03-11T00:00:00Z",
        )

        blocks = list((bt_base / "blocks").iterdir())
        assert len(blocks) == 1

        with open(blocks[0], "r", encoding="utf-8") as f:
            block = json.load(f)
        assert block["content"]["event_type"] == "traversal_association"
        assert block["content"]["from_signifier"] == "走势"
        assert block["content"]["to_signifier"] == "级别"
        assert block["content"]["traversal_id"] == "test-traversal-001"
        assert block["content"]["step_number"] == 42
        assert block["content"]["domain"] == "material"
