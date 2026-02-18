"""四矩阵拓扑管理（MatrixTopology）测试。

概念溯源：[新缠论] 四矩阵拓扑 — 引用卢麒元框架 + ontology-v1 命题5
规范来源：ratio_relation_v1.md §3
"""

from __future__ import annotations

import pytest

from newchan.equivalence import EquivalencePair
from newchan.matrix_topology import (
    ALL_EDGES,
    AssetVertex,
    FourMatrix,
    MatrixEdge,
    make_default_matrix,
    with_representative,
)


# ── AssetVertex 枚举 ─────────────────────────────────────


class TestAssetVertex:
    def test_four_vertices(self):
        """四个资本容器顶点。"""
        assert len(AssetVertex) == 4

    def test_vertex_values(self):
        """每个顶点的中文值。"""
        assert AssetVertex.EQUITY.value == "动产"
        assert AssetVertex.REAL_ESTATE.value == "不动产"
        assert AssetVertex.COMMODITY.value == "商品"
        assert AssetVertex.CASH.value == "现金"


# ── MatrixEdge 不可变性与属性 ─────────────────────────────


class TestMatrixEdge:
    def test_frozen(self):
        """MatrixEdge 是不可变的。"""
        edge = MatrixEdge(
            vertex_a=AssetVertex.EQUITY,
            vertex_b=AssetVertex.CASH,
        )
        with pytest.raises(AttributeError):
            edge.vertex_a = AssetVertex.COMMODITY  # type: ignore[misc]

    def test_label(self):
        """label 属性返回 '顶点A值/顶点B值'。"""
        edge = MatrixEdge(
            vertex_a=AssetVertex.EQUITY,
            vertex_b=AssetVertex.CASH,
        )
        assert edge.label == "动产/现金"

    def test_default_representative_is_none(self):
        """默认无代表性等价对。"""
        edge = MatrixEdge(
            vertex_a=AssetVertex.COMMODITY,
            vertex_b=AssetVertex.REAL_ESTATE,
        )
        assert edge.representative_pair is None

    def test_with_representative_pair(self):
        """可附带代表性等价对。"""
        pair = EquivalencePair(sym_a="SPY", sym_b="USD", category="四矩阵边")
        edge = MatrixEdge(
            vertex_a=AssetVertex.EQUITY,
            vertex_b=AssetVertex.CASH,
            representative_pair=pair,
            description="股市整体涨跌",
        )
        assert edge.representative_pair is pair
        assert edge.description == "股市整体涨跌"

    def test_with_description(self):
        """description 字段记录资本流转含义。"""
        edge = MatrixEdge(
            vertex_a=AssetVertex.EQUITY,
            vertex_b=AssetVertex.COMMODITY,
            description="金融资产 vs 避险资产",
        )
        assert edge.description == "金融资产 vs 避险资产"


# ── ALL_EDGES ─────────────────────────────────────────────


class TestAllEdges:
    def test_six_edges(self):
        """C(4,2) = 6 条边。"""
        assert len(ALL_EDGES) == 6

    def test_all_edges_are_tuples_of_vertex_pairs(self):
        """每条边是 (AssetVertex, AssetVertex) 元组。"""
        for edge in ALL_EDGES:
            assert len(edge) == 2
            assert isinstance(edge[0], AssetVertex)
            assert isinstance(edge[1], AssetVertex)

    def test_no_self_loops(self):
        """没有自环（同一顶点到自己的边）。"""
        for a, b in ALL_EDGES:
            assert a != b

    def test_no_duplicates(self):
        """没有重复边（无序比较）。"""
        normalized = [frozenset(e) for e in ALL_EDGES]
        assert len(normalized) == len(set(normalized))

    def test_all_vertex_pairs_covered(self):
        """覆盖所有 C(4,2)=6 种组合。"""
        all_vertices = list(AssetVertex)
        expected_pairs = set()
        for i in range(len(all_vertices)):
            for j in range(i + 1, len(all_vertices)):
                expected_pairs.add(frozenset([all_vertices[i], all_vertices[j]]))
        actual_pairs = {frozenset(e) for e in ALL_EDGES}
        assert actual_pairs == expected_pairs


# ── FourMatrix 不可变性 ───────────────────────────────────


class TestFourMatrixImmutable:
    def test_frozen(self):
        """FourMatrix 是不可变的。"""
        m = make_default_matrix()
        with pytest.raises(AttributeError):
            m.region = "US"  # type: ignore[misc]

    def test_edges_is_tuple(self):
        """edges 是 tuple 不是 list。"""
        m = make_default_matrix()
        assert isinstance(m.edges, tuple)


# ── make_default_matrix 工厂 ──────────────────────────────


class TestMakeDefaultMatrix:
    def test_has_six_edges(self):
        """默认矩阵包含 6 条边。"""
        m = make_default_matrix()
        assert len(m.edges) == 6

    def test_default_region_empty(self):
        """默认 region 为空。"""
        m = make_default_matrix()
        assert m.region == ""

    def test_custom_region(self):
        """可指定区域。"""
        m = make_default_matrix(region="US")
        assert m.region == "US"

    def test_all_representatives_none(self):
        """默认所有边无代表性等价对。"""
        m = make_default_matrix()
        for edge in m.edges:
            assert edge.representative_pair is None

    def test_covers_all_vertex_pairs(self):
        """默认矩阵覆盖全部 6 种顶点组合。"""
        m = make_default_matrix()
        actual_pairs = {frozenset([e.vertex_a, e.vertex_b]) for e in m.edges}
        all_vertices = list(AssetVertex)
        expected_pairs = set()
        for i in range(len(all_vertices)):
            for j in range(i + 1, len(all_vertices)):
                expected_pairs.add(frozenset([all_vertices[i], all_vertices[j]]))
        assert actual_pairs == expected_pairs

    def test_each_edge_has_description(self):
        """默认矩阵每条边都有资本流转含义描述。"""
        m = make_default_matrix()
        for edge in m.edges:
            assert edge.description != "", f"Edge {edge.label} missing description"


# ── with_representative 不可变更新 ────────────────────────


class TestWithRepresentative:
    def test_returns_new_matrix(self):
        """返回新对象，不修改原对象。"""
        original = make_default_matrix()
        pair = EquivalencePair(sym_a="SPY", sym_b="USD", category="四矩阵边")
        updated = with_representative(
            original, AssetVertex.EQUITY, AssetVertex.CASH, pair
        )
        assert updated is not original

    def test_original_unchanged(self):
        """原矩阵不被修改。"""
        original = make_default_matrix()
        pair = EquivalencePair(sym_a="SPY", sym_b="USD", category="四矩阵边")
        with_representative(original, AssetVertex.EQUITY, AssetVertex.CASH, pair)
        # 原矩阵所有边仍无代表性等价对
        for edge in original.edges:
            assert edge.representative_pair is None

    def test_target_edge_updated(self):
        """目标边的 representative_pair 被更新。"""
        pair = EquivalencePair(sym_a="SPY", sym_b="USD", category="四矩阵边")
        m = with_representative(
            make_default_matrix(), AssetVertex.EQUITY, AssetVertex.CASH, pair
        )
        # 找到目标边
        target = _find_edge(m, AssetVertex.EQUITY, AssetVertex.CASH)
        assert target is not None
        assert target.representative_pair is pair

    def test_other_edges_unchanged(self):
        """非目标边不受影响。"""
        pair = EquivalencePair(sym_a="SPY", sym_b="USD", category="四矩阵边")
        m = with_representative(
            make_default_matrix(), AssetVertex.EQUITY, AssetVertex.CASH, pair
        )
        for edge in m.edges:
            if frozenset([edge.vertex_a, edge.vertex_b]) != frozenset(
                [AssetVertex.EQUITY, AssetVertex.CASH]
            ):
                assert edge.representative_pair is None

    def test_edge_order_symmetric(self):
        """vertex_a/vertex_b 顺序不影响匹配（对称性）。"""
        pair = EquivalencePair(sym_a="SPY", sym_b="USD", category="四矩阵边")
        # 用 CASH, EQUITY 顺序（和边定义中可能不同）
        m = with_representative(
            make_default_matrix(), AssetVertex.CASH, AssetVertex.EQUITY, pair
        )
        target = _find_edge(m, AssetVertex.EQUITY, AssetVertex.CASH)
        assert target is not None
        assert target.representative_pair is pair

    def test_nonexistent_edge_raises(self):
        """对同一个顶点操作应报错（无自环）。"""
        pair = EquivalencePair(sym_a="SPY", sym_b="SPY")
        with pytest.raises(ValueError, match="[Nn]o.*edge|[Ss]ame.*vertex"):
            with_representative(
                make_default_matrix(), AssetVertex.EQUITY, AssetVertex.EQUITY, pair
            )

    def test_chain_updates(self):
        """链式更新多条边。"""
        m = make_default_matrix()
        pair1 = EquivalencePair(sym_a="SPY", sym_b="USD", category="四矩阵边")
        pair2 = EquivalencePair(sym_a="SPY", sym_b="GLD", category="四矩阵边")
        m = with_representative(m, AssetVertex.EQUITY, AssetVertex.CASH, pair1)
        m = with_representative(m, AssetVertex.EQUITY, AssetVertex.COMMODITY, pair2)
        e1 = _find_edge(m, AssetVertex.EQUITY, AssetVertex.CASH)
        e2 = _find_edge(m, AssetVertex.EQUITY, AssetVertex.COMMODITY)
        assert e1 is not None and e1.representative_pair is pair1
        assert e2 is not None and e2.representative_pair is pair2

    def test_preserves_region(self):
        """更新边不丢失 region。"""
        m = make_default_matrix(region="CN")
        pair = EquivalencePair(sym_a="SSE", sym_b="CNY")
        m2 = with_representative(m, AssetVertex.EQUITY, AssetVertex.CASH, pair)
        assert m2.region == "CN"


# ── 辅助函数 ──────────────────────────────────────────────


def _find_edge(
    matrix: FourMatrix, va: AssetVertex, vb: AssetVertex
) -> MatrixEdge | None:
    """在矩阵中查找指定两个顶点之间的边（无序匹配）。"""
    target = frozenset([va, vb])
    for edge in matrix.edges:
        if frozenset([edge.vertex_a, edge.vertex_b]) == target:
            return edge
    return None
