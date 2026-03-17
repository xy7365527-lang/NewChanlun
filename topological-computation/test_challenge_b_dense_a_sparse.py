"""test_challenge_b_dense_a_sparse.py — 439号-3: B密A疏→质疑机制测试.

测试场景：
  S_net 有 paradigmatic 边（sig_A ↔ sig_B），K_active 有概念边（A → B），
  但 K_active 无 COOCCURRENCE 边 → B密A疏质疑 → negate(A, B)
"""
from engine import Graph, Vertex, Edge, EdgeType, VertexStatus
from signifier_net import SNet, Signifier, SignifierEdge, AxisType
from snet_activation import SNetActivation
from traversal import TraversalEngine, EncounterType


def _build_scenario():
    """构建 B密A疏 测试场景：paradigmatic 有，cooccurrence 无，concept edge 有."""
    # K_active: A --REFERENCE--> B (概念边存在)
    g = Graph()
    g = g.add_vertex(Vertex("A", VertexStatus.ACTIVE, created_at=0))
    g = g.add_vertex(Vertex("B", VertexStatus.ACTIVE, created_at=0))
    g = g.add_edge(Edge("A", "B", EdgeType.REFERENCE, created_at=0))

    # S_net: sig_A <--paradigmatic--> sig_B
    snet = SNet()
    snet = snet.add_signifier(Signifier("sig_A"))
    snet = snet.add_signifier(Signifier("sig_B"))
    snet = snet.add_edge(SignifierEdge("sig_A", "sig_B", AxisType.PARADIGMATIC, weight=1.0))

    # 映射: A ↔ sig_A, B ↔ sig_B
    c2s = {"A": "sig_A", "B": "sig_B"}
    s2c = {"sig_A": ["A"], "sig_B": ["B"]}
    activation = SNetActivation(snet, c2s, s2c)

    return g, activation


def test_check_articulation_returns_negate_for_b_dense_a_sparse_with_concept_edge():
    """B密A疏 + 有概念边 → 返回 NEGATE_A encounter."""
    g, activation = _build_scenario()

    engine = TraversalEngine(g, start="A", seed=42)
    engine._snet_activation = activation

    result = engine._check_articulation_encounter()
    assert result is not None, "应检测到 B密A疏 不一致"

    enc, score, meta = result
    assert enc.encounter_type == EncounterType.NEGATE_A, (
        f"B密A疏+有概念边应返回 NEGATE_A，实际返回 {enc.encounter_type}"
    )
    assert meta["imbalance_type"] == "B_dense_A_sparse_challenge"
    assert "challenge" in enc.reason


def test_check_articulation_returns_articulate_for_b_dense_a_sparse_no_concept_edge():
    """B密A疏 + 无概念边 → 返回 ARTICULATE encounter（涌现）."""
    # K_active: A 和 B 存在但无概念边
    g = Graph()
    g = g.add_vertex(Vertex("A", VertexStatus.ACTIVE, created_at=0))
    g = g.add_vertex(Vertex("B", VertexStatus.ACTIVE, created_at=0))

    # S_net: paradigmatic 边
    snet = SNet()
    snet = snet.add_signifier(Signifier("sig_A"))
    snet = snet.add_signifier(Signifier("sig_B"))
    snet = snet.add_edge(SignifierEdge("sig_A", "sig_B", AxisType.PARADIGMATIC, weight=1.0))

    c2s = {"A": "sig_A", "B": "sig_B"}
    s2c = {"sig_A": ["A"], "sig_B": ["B"]}
    activation = SNetActivation(snet, c2s, s2c)

    engine = TraversalEngine(g, start="A", seed=42)
    engine._snet_activation = activation

    result = engine._check_articulation_encounter()
    assert result is not None, "应检测到 B密A疏 不一致"

    enc, score, meta = result
    assert enc.encounter_type == EncounterType.ARTICULATE, (
        f"B密A疏+无概念边应返回 ARTICULATE，实际返回 {enc.encounter_type}"
    )
    assert meta["imbalance_type"] == "B_dense_A_sparse"


def test_no_trigger_when_cooccurrence_exists():
    """有 cooccurrence 边时不触发 B密A疏."""
    g = Graph()
    g = g.add_vertex(Vertex("A", VertexStatus.ACTIVE, created_at=0))
    g = g.add_vertex(Vertex("B", VertexStatus.ACTIVE, created_at=0))
    g = g.add_edge(Edge("A", "B", EdgeType.REFERENCE, created_at=0))
    g = g.add_edge(Edge("A", "B", EdgeType.COOCCURRENCE, created_at=0))

    snet = SNet()
    snet = snet.add_signifier(Signifier("sig_A"))
    snet = snet.add_signifier(Signifier("sig_B"))
    snet = snet.add_edge(SignifierEdge("sig_A", "sig_B", AxisType.PARADIGMATIC, weight=1.0))

    c2s = {"A": "sig_A", "B": "sig_B"}
    s2c = {"sig_A": ["A"], "sig_B": ["B"]}
    activation = SNetActivation(snet, c2s, s2c)

    engine = TraversalEngine(g, start="A", seed=42)
    engine._snet_activation = activation

    result = engine._check_articulation_encounter()
    assert result is None, "有 cooccurrence 时不应触发 B密A疏"
