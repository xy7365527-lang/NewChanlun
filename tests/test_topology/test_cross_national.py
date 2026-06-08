"""跨国 K4 展开测试（254号定义2 / CROSS_NATIONAL 边实装）。"""

import pytest

from newchan.topology.cross_national import (
    MultiEconomyGraph,
    NationalEdge,
    NationalVertex,
    build_multi_economy_graph,
)
from newchan.topology.graph import EdgeType, Vertex


class TestSingleEconomy:
    def test_one_economy_is_plain_k4(self):
        g = build_multi_economy_graph(("US",))
        assert g.n == 1
        assert g.node_count == 4
        assert len(g.domestic_edges) == 6  # 单体 K4 六条边
        assert len(g.currency_edges) == 0
        assert len(g.cross_national_edges) == 0

    def test_one_economy_dof(self):
        """独立自由度 4n-1 = 3。"""
        g = build_multi_economy_graph(("US",))
        assert g.independent_dof == 3


class TestTwoEconomies:
    def setup_method(self):
        self.g = build_multi_economy_graph(("US", "CN"))

    def test_node_count(self):
        assert self.g.node_count == 8

    def test_domestic_edges_6n(self):
        """国内边 = 6n = 12。"""
        assert len(self.g.domestic_edges) == 12

    def test_currency_edges(self):
        """货币边 = n(n-1)/2 = 1。"""
        assert len(self.g.currency_edges) == 1

    def test_cross_national_edges(self):
        """跨国同类边 = 3·n(n-1)/2 = 3。"""
        assert len(self.g.cross_national_edges) == 3

    def test_currency_edge_is_money_to_money(self):
        ce = self.g.currency_edges[0]
        assert ce.is_currency_edge
        assert ce.node_a.vertex is Vertex.M
        assert ce.node_b.vertex is Vertex.M
        assert ce.edge_type is EdgeType.CROSS_NATIONAL

    def test_cross_national_are_same_type_non_cash(self):
        """跨国同类边连接同类资产（P/C/R），不含 M。"""
        for e in self.g.cross_national_edges:
            assert e.node_a.vertex == e.node_b.vertex
            assert e.node_a.vertex is not Vertex.M
            assert e.node_a.economy != e.node_b.economy

    def test_dof(self):
        """独立自由度 4n-1 = 7。"""
        assert self.g.independent_dof == 7


class TestThreeEconomies:
    def test_edge_counts(self):
        g = build_multi_economy_graph(("US", "CN", "EU"))
        assert len(g.domestic_edges) == 18       # 6·3
        assert len(g.currency_edges) == 3         # C(3,2)
        assert len(g.cross_national_edges) == 9   # 3·C(3,2)
        assert g.independent_dof == 11            # 4·3 - 1


class TestValidation:
    def test_empty_raises(self):
        with pytest.raises(ValueError):
            build_multi_economy_graph(())

    def test_duplicate_raises(self):
        with pytest.raises(ValueError):
            build_multi_economy_graph(("US", "US"))
