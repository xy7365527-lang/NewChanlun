"""流转关系（FlowRelation）测试。

验证 liuzhuan.md #14 定义：
  - 顶点流量聚合 net(V)
  - 共振判定 |net(V)| ≥ 2
  - 守恒约束 Σnet(V) = 0
  - 强共振/弱共振区分
"""

from __future__ import annotations

import pytest

from newchan.capital_flow import FlowDirection
from newchan.flow_relation import (
    EdgeFlowInput,
    ResonanceStrength,
    VertexFlowState,
    aggregate_vertex_flows,
    detect_resonance,
)
from newchan.matrix_topology import AssetVertex


# ── helpers ──────────────────────────────────────────────

V = AssetVertex  # 简写


def _edge_input(
    a: AssetVertex, b: AssetVertex, direction: FlowDirection
) -> EdgeFlowInput:
    return EdgeFlowInput(vertex_a=a, vertex_b=b, direction=direction)


# ── 守恒约束 ─────────────────────────────────────────────


class TestConservation:
    """守恒约束：Σnet(V) = 0，对任何合法的 6 条边方向输入。"""

    def test_all_equilibrium(self) -> None:
        """全部均衡 → 所有 net = 0 → 守恒。"""
        edges = [
            _edge_input(a, b, FlowDirection.EQUILIBRIUM)
            for a in V
            for b in V
            if a.value < b.value
        ]
        states = aggregate_vertex_flows(edges)
        for s in states:
            assert s.net_flow == 0

    def test_single_edge_active(self) -> None:
        """单条边有方向，其余均衡 → 守恒。"""
        edges = []
        for a in V:
            for b in V:
                if a.value < b.value:
                    if a == V.EQUITY and b == V.COMMODITY:
                        edges.append(
                            _edge_input(a, b, FlowDirection.A_TO_B)
                        )
                    else:
                        edges.append(
                            _edge_input(a, b, FlowDirection.EQUILIBRIUM)
                        )
        states = aggregate_vertex_flows(edges)

    def test_all_edges_same_direction_still_conserves(self) -> None:
        """所有边 A→B → 守恒（拓扑不变量，不依赖方向模式）。"""
        edges = [
            _edge_input(a, b, FlowDirection.A_TO_B)
            for a in V
            for b in V
            if a.value < b.value
        ]
        states = aggregate_vertex_flows(edges)


# ── 顶点流量聚合 ─────────────────────────────────────────


class TestVertexAggregation:
    """net(V) = Σflow(eᵢ, V), i=1..3"""

    def test_vertex_with_all_inflow(self) -> None:
        """一个顶点的 3 条边全部流入 → net = +3。"""
        # CASH 的 3 条边全部向 CASH 流入
        edges = [
            # EQUITY/CASH: A_TO_B → 资本从 EQUITY 流向 CASH
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            # REAL_ESTATE/CASH: A_TO_B → 资本从 REAL_ESTATE 流向 CASH
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.A_TO_B),
            # COMMODITY/CASH: A_TO_B → 资本从 COMMODITY 流向 CASH
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.A_TO_B),
            # 其余 3 条边均衡
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.EQUILIBRIUM),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        cash_state = _find_vertex(states, V.CASH)
        assert cash_state.net_flow == 3

    def test_vertex_with_all_outflow(self) -> None:
        """一个顶点的 3 条边全部流出 → net = -3。"""
        # COMMODITY 的 3 条边全部从 COMMODITY 流出
        edges = [
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.B_TO_A),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.B_TO_A),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.CASH, FlowDirection.EQUILIBRIUM),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        commodity_state = _find_vertex(states, V.COMMODITY)
        assert commodity_state.net_flow == -3

    def test_mixed_flow(self) -> None:
        """2 入 1 出 → net = +1。"""
        edges = [
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),  # → CASH
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.A_TO_B),  # → CASH
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.B_TO_A),  # ← CASH
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.EQUILIBRIUM),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        cash_state = _find_vertex(states, V.CASH)
        assert cash_state.net_flow == 1


# ── 共振判定 ─────────────────────────────────────────────


class TestResonance:
    """共振：|net(V)| ≥ 2。"""

    def test_strong_resonance_sink(self) -> None:
        """|net| = 3 → 强共振汇。"""
        edges = [
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.EQUILIBRIUM),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        resonances = detect_resonance(states)
        cash_r = _find_vertex(resonances, V.CASH)
        assert cash_r.strength == ResonanceStrength.STRONG
        assert cash_r.net_flow == 3

    def test_strong_resonance_source(self) -> None:
        """|net| = 3 → 强共振源。"""
        edges = [
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.B_TO_A),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.B_TO_A),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.CASH, FlowDirection.EQUILIBRIUM),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        resonances = detect_resonance(states)
        commodity_r = _find_vertex(resonances, V.COMMODITY)
        assert commodity_r.strength == ResonanceStrength.STRONG
        assert commodity_r.net_flow == -3

    def test_weak_resonance(self) -> None:
        """|net| = 2 → 弱共振。"""
        edges = [
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.EQUILIBRIUM),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        resonances = detect_resonance(states)
        cash_r = _find_vertex(resonances, V.CASH)
        assert cash_r.strength == ResonanceStrength.WEAK
        assert cash_r.net_flow == 2

    def test_no_resonance(self) -> None:
        """|net| ≤ 1 → 无共振。"""
        edges = [
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.B_TO_A),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.EQUILIBRIUM),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        resonances = detect_resonance(states)
        cash_r = _find_vertex(resonances, V.CASH)
        assert cash_r.strength == ResonanceStrength.NONE

    def test_all_equilibrium_no_resonance(self) -> None:
        """全部均衡 → 全部无共振。"""
        edges = [
            _edge_input(a, b, FlowDirection.EQUILIBRIUM)
            for a in V
            for b in V
            if a.value < b.value
        ]
        states = aggregate_vertex_flows(edges)
        resonances = detect_resonance(states)
        for r in resonances:
            assert r.strength == ResonanceStrength.NONE
            assert r.net_flow == 0


# ── 边输入验证 ───────────────────────────────────────────


class TestEdgeInputValidation:
    """输入校验。"""

    def test_wrong_edge_count(self) -> None:
        """非 6 条边 → 报错。"""
        with pytest.raises(ValueError, match="6"):
            aggregate_vertex_flows([])

    def test_duplicate_edge(self) -> None:
        """重复边 → 报错。"""
        edges = [
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.CASH, FlowDirection.B_TO_A),  # 重复
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.EQUILIBRIUM),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.EQUILIBRIUM),
        ]
        with pytest.raises(ValueError, match="重复"):
            aggregate_vertex_flows(edges)

    def test_self_loop(self) -> None:
        """自环 → 报错。"""
        with pytest.raises(ValueError, match="自环"):
            _edge_input(V.CASH, V.CASH, FlowDirection.A_TO_B)


# ── helpers ──────────────────────────────────────────────


def _find_vertex(
    states: list[VertexFlowState], vertex: AssetVertex
) -> VertexFlowState:
    for s in states:
        if s.vertex == vertex:
            return s
    raise ValueError(f"未找到顶点 {vertex}")
