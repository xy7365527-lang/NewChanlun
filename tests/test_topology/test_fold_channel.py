"""折叠通道测试（292号/528号：金/油是折叠通道观测量，不是顶点）。"""

import pytest

from newchan.topology.fold_channel import (
    AU,
    K4_FOLD_CHANNELS,
    OIL,
    FoldChannel,
    FoldDirection,
    omega,
)
from newchan.topology.graph import EdgeType, K4Graph, Vertex


class TestAuChannel:
    def test_au_endpoints_c_m(self):
        """Au = C↔M 折叠（黄金 ∈ Σ∩C，254号定理3）。"""
        assert AU.endpoints == frozenset({Vertex.C, Vertex.M})

    def test_au_bidirectional(self):
        assert AU.is_bidirectional
        assert AU.direction is FoldDirection.BIDIRECTIONAL

    def test_au_observable_gold(self):
        assert AU.observable_symbol == "GC"

    def test_au_weight_global_shared(self):
        """292号区间套：Au 全局共享，W=4 最外层。"""
        assert AU.weight == 4

    def test_au_folds_across_cm_independent_edge(self):
        """Au 观测于 C/M 独立边。"""
        g = K4Graph()
        edge = AU.folds_across(g)
        assert edge.vertices == frozenset({Vertex.C, Vertex.M})
        assert edge.edge_type == EdgeType.INDEPENDENT


class TestOilChannel:
    def test_oil_endpoints_c_p(self):
        """Oil = C→P 通道（产业循环物质投入，330号）。"""
        assert OIL.endpoints == frozenset({Vertex.C, Vertex.P})

    def test_oil_directed(self):
        assert not OIL.is_bidirectional
        assert OIL.direction is FoldDirection.DIRECTED
        assert OIL.source is Vertex.C
        assert OIL.target is Vertex.P

    def test_oil_observable_crude(self):
        assert OIL.observable_symbol == "CL"

    def test_oil_weight_half_shared(self):
        """292号区间套：Oil 半共享，W=3 次外层。"""
        assert OIL.weight == 3

    def test_oil_folds_across_pc_derived_edge(self):
        """Oil 观测于 P/C 派生边。"""
        g = K4Graph()
        edge = OIL.folds_across(g)
        assert edge.vertices == frozenset({Vertex.P, Vertex.C})
        assert edge.edge_type == EdgeType.DERIVED


class TestFoldChannelSet:
    def test_k4_channels_are_au_oil(self):
        assert set(K4_FOLD_CHANNELS) == {AU, OIL}

    def test_au_outranks_oil(self):
        """区间套顺序：Au（外层）权重 > Oil（内层）。"""
        assert AU.weight > OIL.weight

    def test_frozen(self):
        with pytest.raises(AttributeError):
            AU.weight = 9  # type: ignore


class TestOmega:
    def test_omega_ratio(self):
        """ω = 金价 / 油价（482号 金油比 = 剥削率）。"""
        assert omega(2000.0, 80.0) == pytest.approx(25.0)

    def test_omega_rises_when_oil_falls(self):
        assert omega(2000.0, 50.0) > omega(2000.0, 80.0)

    def test_omega_zero_oil_raises(self):
        with pytest.raises(ValueError):
            omega(2000.0, 0.0)

    def test_omega_negative_oil_raises(self):
        with pytest.raises(ValueError):
            omega(2000.0, -1.0)
