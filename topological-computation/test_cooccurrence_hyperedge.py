"""test_cooccurrence_hyperedge.py — 共现超边单元测试。

测试覆盖：
  1. CooccurrenceHyperedge 不可变性
  2. SNet 超边 add/query/index
  3. derive_pairwise_edges 正确性
  4. ingest_text_passage_batch 产出超边（不产出成对边）
  5. 序列化/反序列化
"""

import sys
import os

# 确保 topological-computation 在 sys.path 中
_THIS_DIR = os.path.dirname(os.path.abspath(__file__))
if _THIS_DIR not in sys.path:
    sys.path.insert(0, _THIS_DIR)

from cooccurrence_hyperedge import (
    CooccurrenceHyperedge,
    hyperedge_to_dict,
    hyperedge_from_dict,
)
from signifier_net import SNet, Signifier, SignifierEdge, AxisType


# ---------------------------------------------------------------------------
# 1. CooccurrenceHyperedge 不可变性
# ---------------------------------------------------------------------------

def test_hyperedge_frozen():
    """CooccurrenceHyperedge 是 frozen dataclass，不可修改字段。"""
    he = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B", "C"}),
        source="test.md",
        domain="test",
        timestamp="123.0",
    )
    try:
        he.source = "changed"  # type: ignore
        assert False, "应该抛出 FrozenInstanceError"
    except AttributeError:
        pass  # 预期行为


def test_hyperedge_vertices_frozenset():
    """vertices 是 frozenset，不可修改。"""
    he = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B"}),
        source="test.md",
        domain="test",
        timestamp="0",
    )
    assert isinstance(he.vertices, frozenset)
    assert len(he.vertices) == 2


def test_hyperedge_equality():
    """相同字段的超边相等。"""
    he1 = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B"}),
        source="s",
        domain="d",
        timestamp="t",
    )
    he2 = CooccurrenceHyperedge(
        vertices=frozenset({"B", "A"}),
        source="s",
        domain="d",
        timestamp="t",
    )
    assert he1 == he2


# ---------------------------------------------------------------------------
# 2. SNet 超边 add/query/index
# ---------------------------------------------------------------------------

def _make_snet_with_signifiers(*ids: str) -> SNet:
    """创建包含指定 signifier IDs 的 SNet。"""
    snet = SNet()
    for sid in ids:
        snet = snet.add_signifier(Signifier(id=sid))
    return snet


def test_snet_add_hyperedge():
    """add_hyperedge 返回新 SNet，不修改原始。"""
    snet = _make_snet_with_signifiers("A", "B", "C")
    he = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B", "C"}),
        source="test.md",
        domain="test",
        timestamp="0",
    )
    new_snet = snet.add_hyperedge(he)
    assert len(snet.hyperedges) == 0
    assert len(new_snet.hyperedges) == 1
    assert new_snet.hyperedges[0] == he


def test_snet_add_hyperedges_batch():
    """add_hyperedges 批量添加。"""
    snet = _make_snet_with_signifiers("A", "B", "C", "D")
    he1 = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B"}),
        source="s1",
        domain="d",
        timestamp="0",
    )
    he2 = CooccurrenceHyperedge(
        vertices=frozenset({"C", "D"}),
        source="s2",
        domain="d",
        timestamp="1",
    )
    new_snet = snet.add_hyperedges([he1, he2])
    assert len(new_snet.hyperedges) == 2


def test_snet_add_hyperedges_empty():
    """add_hyperedges([]) 返回 self。"""
    snet = _make_snet_with_signifiers("A")
    result = snet.add_hyperedges([])
    assert result is snet


def test_snet_hyperedges_containing():
    """hyperedges_containing 返回包含指定顶点的超边。"""
    snet = _make_snet_with_signifiers("A", "B", "C", "D")
    he1 = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B", "C"}),
        source="s1",
        domain="d",
        timestamp="0",
    )
    he2 = CooccurrenceHyperedge(
        vertices=frozenset({"B", "D"}),
        source="s2",
        domain="d",
        timestamp="1",
    )
    snet = snet.add_hyperedges([he1, he2])

    assert len(snet.hyperedges_containing("A")) == 1
    assert len(snet.hyperedges_containing("B")) == 2
    assert len(snet.hyperedges_containing("D")) == 1
    assert len(snet.hyperedges_containing("X")) == 0


def test_snet_cooccurrence_context():
    """cooccurrence_context 返回同时包含两个顶点的超边。"""
    snet = _make_snet_with_signifiers("A", "B", "C", "D")
    he1 = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B", "C"}),
        source="s1",
        domain="d",
        timestamp="0",
    )
    he2 = CooccurrenceHyperedge(
        vertices=frozenset({"B", "D"}),
        source="s2",
        domain="d",
        timestamp="1",
    )
    he3 = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B"}),
        source="s3",
        domain="d",
        timestamp="2",
    )
    snet = snet.add_hyperedges([he1, he2, he3])

    ctx_ab = snet.cooccurrence_context("A", "B")
    assert len(ctx_ab) == 2  # he1 and he3
    ctx_bd = snet.cooccurrence_context("B", "D")
    assert len(ctx_bd) == 1  # he2
    ctx_ad = snet.cooccurrence_context("A", "D")
    assert len(ctx_ad) == 0


# ---------------------------------------------------------------------------
# 3. derive_pairwise_edges 正确性
# ---------------------------------------------------------------------------

def test_derive_pairwise_edges():
    """从 3-顶点超边派生 C(3,2)=3 条成对边。"""
    snet = _make_snet_with_signifiers("A", "B", "C")
    he = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B", "C"}),
        source="test.md",
        domain="test",
        timestamp="0",
        evidence_tag="corpus:test:test.md",
    )
    edges = snet.derive_pairwise_edges(he)
    assert len(edges) == 3  # C(3,2) = 3
    # 所有边都是 SYNTAGMATIC
    assert all(e.axis == AxisType.SYNTAGMATIC for e in edges)
    # 检查所有对
    pairs = {(e.source, e.target) for e in edges}
    assert ("A", "B") in pairs
    assert ("A", "C") in pairs
    assert ("B", "C") in pairs


def test_derive_pairwise_edges_two_vertices():
    """2-顶点超边派生 1 条成对边。"""
    snet = SNet()
    he = CooccurrenceHyperedge(
        vertices=frozenset({"X", "Y"}),
        source="s",
        domain="d",
        timestamp="0",
    )
    edges = snet.derive_pairwise_edges(he)
    assert len(edges) == 1
    assert edges[0].source == "X"
    assert edges[0].target == "Y"


def test_derive_pairwise_edges_deterministic():
    """derive_pairwise_edges 的输出是确定性的（按字典序）。"""
    snet = SNet()
    he = CooccurrenceHyperedge(
        vertices=frozenset({"C", "A", "B"}),
        source="s",
        domain="d",
        timestamp="0",
    )
    edges1 = snet.derive_pairwise_edges(he)
    edges2 = snet.derive_pairwise_edges(he)
    assert [(e.source, e.target) for e in edges1] == [(e.source, e.target) for e in edges2]


# ---------------------------------------------------------------------------
# 4. ingest_text_passage_batch 产出超边
# ---------------------------------------------------------------------------

def test_ingest_batch_produces_hyperedges():
    """ingest_text_passage_batch 产出超边，不产出成对边。"""
    from signifier_net_ingest import ingest_text_passage_batch

    snet = _make_snet_with_signifiers("走势", "级别", "中枢")
    paragraphs = ["走势由多个级别的中枢构成。"]

    new_snet, log_entries = ingest_text_passage_batch(
        snet, paragraphs, "chanlun", "test.md",
    )

    # 应产出超边
    assert len(new_snet.hyperedges) >= 1
    # 超边包含匹配到的术语
    he = new_snet.hyperedges[0]
    assert "走势" in he.vertices
    assert "级别" in he.vertices
    assert "中枢" in he.vertices

    # 不应产出成对边
    assert len(new_snet.edges) == 0

    # 日志类型是 hyperedge
    assert all(e["type"] == "hyperedge" for e in log_entries)


def test_ingest_batch_single_term_no_hyperedge():
    """段落中只有一个术语时不产出超边。"""
    from signifier_net_ingest import ingest_text_passage_batch

    snet = _make_snet_with_signifiers("走势", "级别")
    paragraphs = ["这是关于走势的段落。"]

    new_snet, log_entries = ingest_text_passage_batch(
        snet, paragraphs, "chanlun", "test.md",
    )

    assert len(new_snet.hyperedges) == 0
    assert len(log_entries) == 0


def test_ingest_batch_multiple_paragraphs():
    """多段落批量摄入产出多个超边。"""
    from signifier_net_ingest import ingest_text_passage_batch

    snet = _make_snet_with_signifiers("走势", "级别", "中枢", "买卖点")
    paragraphs = [
        "走势由多个级别构成。",
        "中枢与买卖点的关系。",
    ]

    new_snet, log_entries = ingest_text_passage_batch(
        snet, paragraphs, "chanlun", "test.md",
    )

    assert len(new_snet.hyperedges) == 2
    assert len(log_entries) == 2


def test_ingest_single_passage_produces_hyperedge():
    """ingest_text_passage 产出超边。"""
    from signifier_net_ingest import ingest_text_passage

    snet = _make_snet_with_signifiers("走势", "级别", "中枢")

    new_snet, log_entries = ingest_text_passage(
        snet, "走势由多个级别的中枢构成。", "chanlun", "test.md",
    )

    assert len(new_snet.hyperedges) >= 1
    assert len(new_snet.edges) == 0
    assert all(e["type"] == "hyperedge" for e in log_entries)


# ---------------------------------------------------------------------------
# 5. 序列化/反序列化
# ---------------------------------------------------------------------------

def test_hyperedge_serialization():
    """hyperedge_to_dict / hyperedge_from_dict 往返一致。"""
    he = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B", "C"}),
        source="test.md",
        domain="test",
        timestamp="123.456",
        ingest_param_refs=("snet_param:alpha=0.5",),
        evidence_tag="corpus:test:test.md",
    )
    d = hyperedge_to_dict(he)
    restored = hyperedge_from_dict(d)
    assert restored == he
    assert restored.vertices == he.vertices
    assert restored.ingest_param_refs == he.ingest_param_refs


def test_snet_serialization_with_hyperedges():
    """SNet.to_dict / SNet.from_dict 包含超边。"""
    snet = _make_snet_with_signifiers("A", "B", "C")
    he = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B", "C"}),
        source="test.md",
        domain="test",
        timestamp="0",
        evidence_tag="corpus:test:test.md",
    )
    snet = snet.add_hyperedge(he)

    d = snet.to_dict()
    restored = SNet.from_dict(d)

    assert len(restored.hyperedges) == 1
    assert restored.hyperedges[0] == he
    assert len(restored.hyperedges_containing("A")) == 1


def test_snet_serialization_empty_hyperedges():
    """无超边的 SNet 序列化/反序列化正常。"""
    snet = _make_snet_with_signifiers("A")
    d = snet.to_dict()
    restored = SNet.from_dict(d)
    assert len(restored.hyperedges) == 0


def test_snet_serialization_backward_compatible():
    """旧格式（无 hyperedges 键）的 dict 反序列化正常。"""
    old_data = {
        "signifiers": {
            "A": {"id": "A", "surface_forms": [], "source": "k_active_projection",
                   "lang": "", "domain": ""},
        },
        "edges": [],
        "morphemes": {},
        # 没有 "hyperedges" 键
    }
    snet = SNet.from_dict(old_data)
    assert len(snet.hyperedges) == 0
    assert len(snet.signifiers) == 1


# ---------------------------------------------------------------------------
# SNet 不可变操作保留超边
# ---------------------------------------------------------------------------

def test_add_signifier_preserves_hyperedges():
    """add_signifier 返回的新 SNet 保留已有超边。"""
    snet = _make_snet_with_signifiers("A", "B")
    he = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B"}),
        source="s",
        domain="d",
        timestamp="0",
    )
    snet = snet.add_hyperedge(he)
    new_snet = snet.add_signifier(Signifier(id="C"))
    assert len(new_snet.hyperedges) == 1


def test_add_edge_preserves_hyperedges():
    """add_edge 返回的新 SNet 保留已有超边。"""
    snet = _make_snet_with_signifiers("A", "B")
    he = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B"}),
        source="s",
        domain="d",
        timestamp="0",
    )
    snet = snet.add_hyperedge(he)
    new_snet = snet.add_edge(SignifierEdge(
        source="A", target="B", axis=AxisType.SYNTAGMATIC,
    ))
    assert len(new_snet.hyperedges) == 1


def test_merge_edge_weights_preserves_hyperedges():
    """merge_edge_weights 保留超边。"""
    snet = _make_snet_with_signifiers("A", "B")
    he = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B"}),
        source="s",
        domain="d",
        timestamp="0",
    )
    snet = snet.add_hyperedge(he)
    snet = snet.add_edge(SignifierEdge(
        source="A", target="B", axis=AxisType.SYNTAGMATIC, weight=1.0,
    ))
    snet = snet.add_edge(SignifierEdge(
        source="A", target="B", axis=AxisType.SYNTAGMATIC, weight=2.0,
    ))
    merged = snet.merge_edge_weights()
    assert len(merged.hyperedges) == 1
    assert len(merged.edges) == 1
    assert merged.edges[0].weight == 3.0


def test_repr_includes_hyperedges():
    """SNet repr 包含 hyperedges 计数。"""
    snet = _make_snet_with_signifiers("A", "B")
    he = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B"}),
        source="s",
        domain="d",
        timestamp="0",
    )
    snet = snet.add_hyperedge(he)
    r = repr(snet)
    assert "hyperedges=1" in r


if __name__ == "__main__":
    import pytest
    pytest.main([__file__, "-v"])
