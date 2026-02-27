"""转换拓扑测试：K4 配置空间 P₃□P₃□P₃ 的结构性质验证。"""

from __future__ import annotations

import pytest

from newchan.transition_topology import (
    CORNERS,
    DERIVED_EDGES,
    HUB,
    INDEPENDENT_EDGES,
    EdgeState,
    K4Config,
    all_configs,
    check_algebraic_consistency,
    config_to_flow_directions,
    distance,
    edge_state_to_flow_direction,
    flow_direction_to_edge_state,
    is_adjacent,
    neighbors,
)
from newchan.capital_flow import FlowDirection
from newchan.matrix_topology import AssetVertex


# ====================================================================
# EdgeState 枚举
# ====================================================================


class TestEdgeState:
    def test_values(self):
        assert EdgeState.POSITIVE == 1
        assert EdgeState.ZERO == 0
        assert EdgeState.NEGATIVE == -1

    def test_member_count(self):
        assert len(EdgeState) == 3


# ====================================================================
# 27 种配置完备性
# ====================================================================


class TestAllConfigs:
    def test_count(self):
        configs = all_configs()
        assert len(configs) == 27

    def test_unique(self):
        configs = all_configs()
        tuples = [c.as_tuple for c in configs]
        assert len(set(tuples)) == 27

    def test_exhaustive(self):
        """每个 {-1,0,+1}^3 组合都出现。"""
        configs = all_configs()
        expected = {
            (e, c, r)
            for e in (-1, 0, 1)
            for c in (-1, 0, 1)
            for r in (-1, 0, 1)
        }
        actual = {c.as_tuple for c in configs}
        assert actual == expected


# ====================================================================
# 层级分类
# ====================================================================


class TestLayerClassification:
    def test_corner_count(self):
        configs = all_configs()
        corners = [c for c in configs if c.layer == "corner"]
        assert len(corners) == 8

    def test_edge_count(self):
        configs = all_configs()
        edges = [c for c in configs if c.layer == "edge"]
        assert len(edges) == 12

    def test_face_count(self):
        configs = all_configs()
        faces = [c for c in configs if c.layer == "face"]
        assert len(faces) == 6

    def test_center_count(self):
        configs = all_configs()
        centers = [c for c in configs if c.layer == "center"]
        assert len(centers) == 1

    def test_partition(self):
        """8 + 12 + 6 + 1 = 27。"""
        configs = all_configs()
        by_layer = {}
        for c in configs:
            by_layer.setdefault(c.layer, []).append(c)
        assert sum(len(v) for v in by_layer.values()) == 27


# ====================================================================
# 邻接关系（Hamming distance = 1 且状态差 = 1）
# ====================================================================


class TestAdjacency:
    def test_self_not_adjacent(self):
        for c in all_configs():
            assert not is_adjacent(c, c)

    def test_symmetric(self):
        configs = all_configs()
        for a in configs:
            for b in configs:
                assert is_adjacent(a, b) == is_adjacent(b, a)

    def test_no_direct_reversal(self):
        """+ 不能直接跳到 -。"""
        a = K4Config(EdgeState.POSITIVE, EdgeState.ZERO, EdgeState.ZERO)
        b = K4Config(EdgeState.NEGATIVE, EdgeState.ZERO, EdgeState.ZERO)
        assert not is_adjacent(a, b)

    def test_adjacent_pair(self):
        a = K4Config(EdgeState.POSITIVE, EdgeState.ZERO, EdgeState.ZERO)
        b = K4Config(EdgeState.ZERO, EdgeState.ZERO, EdgeState.ZERO)
        assert is_adjacent(a, b)

    def test_two_diffs_not_adjacent(self):
        a = K4Config(EdgeState.POSITIVE, EdgeState.POSITIVE, EdgeState.ZERO)
        b = K4Config(EdgeState.ZERO, EdgeState.ZERO, EdgeState.ZERO)
        assert not is_adjacent(a, b)

    def test_neighbor_counts(self):
        """corner=3, edge=4, face=5, center=6。"""
        expected = {"corner": 3, "edge": 4, "face": 5, "center": 6}
        for c in all_configs():
            nbrs = neighbors(c)
            assert len(nbrs) == expected[c.layer], (
                f"{c.label} layer={c.layer}: "
                f"expected {expected[c.layer]} neighbors, got {len(nbrs)}"
            )

    def test_neighbor_count_matches_property(self):
        for c in all_configs():
            assert len(neighbors(c)) == c.neighbor_count

    def test_neighbors_are_adjacent(self):
        for c in all_configs():
            for n in neighbors(c):
                assert is_adjacent(c, n)

    def test_neighbors_symmetric(self):
        """如果 b 是 a 的邻居，则 a 也是 b 的邻居。"""
        for c in all_configs():
            for n in neighbors(c):
                assert c in neighbors(n)

    def test_total_edges(self):
        """总边数 = Σ neighbor_count / 2。"""
        configs = all_configs()
        total = sum(len(neighbors(c)) for c in configs)
        # P₃□P₃□P₃: 每轴 2 条边 × 3^2 = 18 per axis × 3 axes = 54
        assert total // 2 == 54


# ====================================================================
# 距离函数
# ====================================================================


class TestDistance:
    def test_self_distance_zero(self):
        for c in all_configs():
            assert distance(c, c) == 0

    def test_symmetric(self):
        configs = all_configs()
        for a in configs:
            for b in configs:
                assert distance(a, b) == distance(b, a)

    def test_triangle_inequality(self):
        configs = all_configs()
        for a in configs:
            for b in configs:
                for c in configs:
                    assert distance(a, c) <= distance(a, b) + distance(b, c)

    def test_diameter(self):
        """直径 = 6：(+,+,+) 到 (-,-,-)。"""
        ppp = K4Config(EdgeState.POSITIVE, EdgeState.POSITIVE, EdgeState.POSITIVE)
        nnn = K4Config(EdgeState.NEGATIVE, EdgeState.NEGATIVE, EdgeState.NEGATIVE)
        assert distance(ppp, nnn) == 6

    def test_hub_max_distance(self):
        """(0,0,0) 到任何节点最多 3 步。"""
        for c in all_configs():
            assert distance(HUB, c) <= 3

    def test_adjacent_distance_one(self):
        for c in all_configs():
            for n in neighbors(c):
                assert distance(c, n) == 1


# ====================================================================
# 特殊配置
# ====================================================================


class TestSpecialConfigs:
    def test_hub(self):
        assert HUB.as_tuple == (0, 0, 0)
        assert HUB.layer == "center"

    def test_corners(self):
        assert len(CORNERS) == 8
        for c in CORNERS:
            assert c.layer == "corner"
            assert c.zero_count == 0


# ====================================================================
# net_cash（v4 §5.5）
# ====================================================================


class TestNetCash:
    def test_all_positive(self):
        """(+,+,+) → net($) = -3。"""
        c = K4Config(EdgeState.POSITIVE, EdgeState.POSITIVE, EdgeState.POSITIVE)
        assert c.net_cash == -3

    def test_all_negative(self):
        """(-,-,-) → net($) = +3。"""
        c = K4Config(EdgeState.NEGATIVE, EdgeState.NEGATIVE, EdgeState.NEGATIVE)
        assert c.net_cash == 3

    def test_hub(self):
        assert HUB.net_cash == 0

    def test_mixed(self):
        """(+,-,0) → net($) = 0。"""
        c = K4Config(EdgeState.POSITIVE, EdgeState.NEGATIVE, EdgeState.ZERO)
        assert c.net_cash == 0

    def test_v4_table_spot_checks(self):
        """v4 §5.5 枚举表中的 net($) 抽查。"""
        # #4: (+,+,-) → net($) = -1
        assert K4Config(EdgeState.POSITIVE, EdgeState.POSITIVE, EdgeState.NEGATIVE).net_cash == -1
        # #7: (+,-,-) → net($) = +1
        assert K4Config(EdgeState.POSITIVE, EdgeState.NEGATIVE, EdgeState.NEGATIVE).net_cash == 1
        # #10: (+,+,0) → net($) = -2
        assert K4Config(EdgeState.POSITIVE, EdgeState.POSITIVE, EdgeState.ZERO).net_cash == -2
        # #13: (-,-,0) → net($) = +2
        assert K4Config(EdgeState.NEGATIVE, EdgeState.NEGATIVE, EdgeState.ZERO).net_cash == 2


# ====================================================================
# 派生边允许状态集（v4 §5.5）
# ====================================================================


class TestDerivedEdgeAllowed:
    ALL = frozenset(EdgeState)

    def test_opposite_signs_determined(self):
        """异号 → 方向确定。#4: (+,+,-) → E/R={+}, C/R={+}。"""
        c = K4Config(EdgeState.POSITIVE, EdgeState.POSITIVE, EdgeState.NEGATIVE)
        allowed = c.all_derived_allowed()
        # E/C: same sign → indeterminate
        assert allowed[0] == self.ALL
        # E/R: + vs - → determined +
        assert allowed[1] == frozenset({EdgeState.POSITIVE})
        # C/R: + vs - → determined +
        assert allowed[2] == frozenset({EdgeState.POSITIVE})

    def test_same_signs_indeterminate(self):
        """同号 → {+,-,0}。#1: (+,+,+) → 三条派生边均不确定。"""
        c = K4Config(EdgeState.POSITIVE, EdgeState.POSITIVE, EdgeState.POSITIVE)
        for a in c.all_derived_allowed():
            assert a == self.ALL

    def test_one_zero_partial(self):
        """一端为 0 → 部分约束。#22: (+,0,0) → E/C={+,0}, E/R={+,0}, C/R={+,-,0}。"""
        c = K4Config(EdgeState.POSITIVE, EdgeState.ZERO, EdgeState.ZERO)
        allowed = c.all_derived_allowed()
        assert allowed[0] == frozenset({EdgeState.POSITIVE, EdgeState.ZERO})
        assert allowed[1] == frozenset({EdgeState.POSITIVE, EdgeState.ZERO})
        assert allowed[2] == self.ALL

    def test_double_zero_indeterminate(self):
        """双零 → {+,-,0}。(0,0,0) 的所有派生边。"""
        for a in HUB.all_derived_allowed():
            assert a == self.ALL

    def test_v4_config_16(self):
        """#16: (+,-,0) → E/C={+}, E/R={+,0}, C/R={-,0}。"""
        c = K4Config(EdgeState.POSITIVE, EdgeState.NEGATIVE, EdgeState.ZERO)
        allowed = c.all_derived_allowed()
        assert allowed[0] == frozenset({EdgeState.POSITIVE})
        assert allowed[1] == frozenset({EdgeState.POSITIVE, EdgeState.ZERO})
        assert allowed[2] == frozenset({EdgeState.NEGATIVE, EdgeState.ZERO})

    def test_v4_config_24(self):
        """#24: (0,+,0) → E/C={-,0}, E/R={+,-,0}, C/R={+,0}。"""
        c = K4Config(EdgeState.ZERO, EdgeState.POSITIVE, EdgeState.ZERO)
        allowed = c.all_derived_allowed()
        assert allowed[0] == frozenset({EdgeState.NEGATIVE, EdgeState.ZERO})
        assert allowed[1] == self.ALL
        assert allowed[2] == frozenset({EdgeState.POSITIVE, EdgeState.ZERO})


# ====================================================================
# 代数一致性检验
# ====================================================================


class TestAlgebraicConsistency:
    def test_consistent_determined(self):
        """异号配置，观测值在允许集内 → 无违规。"""
        ind = (EdgeState.POSITIVE, EdgeState.NEGATIVE, EdgeState.ZERO)
        # #16: E/C={+}, E/R={+,0}, C/R={-,0}
        obs = (EdgeState.POSITIVE, EdgeState.ZERO, EdgeState.NEGATIVE)
        assert check_algebraic_consistency(ind, obs) == []

    def test_inconsistent(self):
        """异号配置，观测值在允许集外 → 有违规。"""
        ind = (EdgeState.POSITIVE, EdgeState.NEGATIVE, EdgeState.ZERO)
        # E/C 允许 {+}，给 -
        obs = (EdgeState.NEGATIVE, EdgeState.ZERO, EdgeState.NEGATIVE)
        violations = check_algebraic_consistency(ind, obs)
        assert len(violations) == 1
        assert "E/C" in violations[0]

    def test_same_sign_always_consistent(self):
        """同号配置，任何观测值都一致（允许集 = {+,-,0}）。"""
        ind = (EdgeState.POSITIVE, EdgeState.POSITIVE, EdgeState.POSITIVE)
        for e in EdgeState:
            for c in EdgeState:
                for r in EdgeState:
                    obs = (e, c, r)
                    assert check_algebraic_consistency(ind, obs) == []


# ====================================================================
# FlowDirection 映射
# ====================================================================


class TestFlowDirectionMapping:
    def test_roundtrip(self):
        for s in EdgeState:
            fd = edge_state_to_flow_direction(s)
            assert flow_direction_to_edge_state(fd) == s

    def test_unknown_raises(self):
        with pytest.raises(ValueError):
            flow_direction_to_edge_state(FlowDirection.UNKNOWN)

    def test_config_to_flow_directions_keys(self):
        result = config_to_flow_directions(HUB)
        assert len(result) == 6
        for edge in INDEPENDENT_EDGES:
            assert edge in result
        for edge in DERIVED_EDGES:
            assert edge in result


# ====================================================================
# 独立边 / 派生边定义
# ====================================================================


class TestEdgeDefinitions:
    def test_independent_edges_count(self):
        assert len(INDEPENDENT_EDGES) == 3

    def test_derived_edges_count(self):
        assert len(DERIVED_EDGES) == 3

    def test_independent_edges_all_connect_cash(self):
        for va, vb in INDEPENDENT_EDGES:
            assert vb == AssetVertex.CASH

    def test_derived_edges_no_cash(self):
        for va, vb in DERIVED_EDGES:
            assert va != AssetVertex.CASH
            assert vb != AssetVertex.CASH

    def test_total_edges_cover_k4(self):
        """6 条边覆盖 K4 的全部 C(4,2)=6 条边。"""
        all_edges = set()
        for va, vb in INDEPENDENT_EDGES:
            all_edges.add(frozenset({va, vb}))
        for va, vb in DERIVED_EDGES:
            all_edges.add(frozenset({va, vb}))
        assert len(all_edges) == 6


# ====================================================================
# 对称性（v4 §5.5 对称性说明）
# ====================================================================


class TestSymmetry:
    def test_negation_pairing(self):
        """取反（+↔-，0不变）形成 13 对 + 1 自对称 = 14 独立配置。"""
        configs = all_configs()
        pairs = set()
        self_sym = 0
        for c in configs:
            neg = K4Config(
                e_cash=EdgeState(-c.e_cash),
                c_cash=EdgeState(-c.c_cash),
                r_cash=EdgeState(-c.r_cash),
            )
            if c.as_tuple == neg.as_tuple:
                self_sym += 1
            else:
                pair = frozenset({c.as_tuple, neg.as_tuple})
                pairs.add(pair)
        assert self_sym == 1  # (0,0,0)
        assert len(pairs) == 13
