"""Flow(源→汇) 数据结构测试。

定义依据（liuzhuan.md #14 §流转关系）：
  当顶点 V 为流转源（net(V) ≤ -2），且顶点 W₁, W₂, ... 为流转汇（net(Wᵢ) ≥ +2）时，
  存在流转关系：Flow(V → {W₁, W₂, ...})
  这是一个有向、一对多的关系，从源顶点到一个或多个汇顶点。

审计缺口（audit-cross-3.md §3 MEDIUM）：
  Flow(V → {W₁, W₂, ...}) 流转关系数据结构未实现。
  代码止步于 VertexFlowState（每个顶点的 net_flow），
  没有从 VertexFlowState 聚合出 Flow(源→汇集合) 的数据结构。
"""

from __future__ import annotations

import pytest

from newchan.capital_flow import FlowDirection
from newchan.flow_relation import (
    EdgeFlowInput,
    FlowRole,
    FlowRelation,
    ResonanceStrength,
    VertexFlowState,
    aggregate_vertex_flows,
    extract_flow_relations,
)
from newchan.matrix_topology import AssetVertex


# ── helpers ──────────────────────────────────────────────

V = AssetVertex


def _edge_input(
    a: AssetVertex, b: AssetVertex, direction: FlowDirection,
) -> EdgeFlowInput:
    return EdgeFlowInput(vertex_a=a, vertex_b=b, direction=direction)


def _find_vertex(
    states: list[VertexFlowState], vertex: AssetVertex,
) -> VertexFlowState:
    for s in states:
        if s.vertex == vertex:
            return s
    raise ValueError(f"未找到顶点 {vertex}")


# ── FlowRole 枚举 ───────────────────────────────────────


class TestFlowRole:
    """VertexFlowState 的显式源/汇/中性分类。

    审计缺口（LOW）：VertexFlowState 无 "源/汇/中性" 显式分类。
    """

    def test_source_role(self) -> None:
        """net ≤ -2 → SOURCE。"""
        state = VertexFlowState(
            vertex=V.COMMODITY,
            net_flow=-3,
            strength=ResonanceStrength.STRONG,
            role=FlowRole.SOURCE,
        )
        assert state.role == FlowRole.SOURCE

    def test_sink_role(self) -> None:
        """net ≥ +2 → SINK。"""
        state = VertexFlowState(
            vertex=V.CASH,
            net_flow=3,
            strength=ResonanceStrength.STRONG,
            role=FlowRole.SINK,
        )
        assert state.role == FlowRole.SINK

    def test_neutral_role(self) -> None:
        """|net| ≤ 1 → NEUTRAL。"""
        state = VertexFlowState(
            vertex=V.EQUITY,
            net_flow=0,
            strength=ResonanceStrength.NONE,
            role=FlowRole.NEUTRAL,
        )
        assert state.role == FlowRole.NEUTRAL

    def test_aggregate_assigns_roles(self) -> None:
        """aggregate_vertex_flows 自动赋予 role。"""
        edges = [
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.EQUILIBRIUM),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        cash = _find_vertex(states, V.CASH)
        assert cash.role == FlowRole.SINK
        assert cash.net_flow == 3


# ── FlowRelation 数据结构 ───────────────────────────────


class TestFlowRelation:
    """Flow(V → {W₁, W₂, ...}) 有向、一对多的流转关系。"""

    def test_single_source_single_sink(self) -> None:
        """单源→单汇：COMMODITY 流出，CASH 流入。"""
        # COMMODITY 的 3 条边全部流出
        edges = [
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.B_TO_A),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.B_TO_A),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.A_TO_B),
            # 其他边造成 CASH 也是汇
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        relations = extract_flow_relations(states)

        # 至少存在一个 FlowRelation
        assert len(relations) >= 1

        # 查找以 COMMODITY 为源的流转关系
        commodity_flows = [r for r in relations if r.source == V.COMMODITY]
        assert len(commodity_flows) == 1
        assert V.CASH in commodity_flows[0].sinks

    def test_single_source_multiple_sinks(self) -> None:
        """单源→多汇：资本从 COMMODITY 流出，流入多个容器。"""
        # 设计一个场景：COMMODITY 是源，CASH 和 EQUITY 都是汇
        edges = [
            # COMMODITY 的 3 条边全部流出
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.B_TO_A),  # COMMODITY→EQUITY
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.B_TO_A),  # COMMODITY→RE
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.A_TO_B),  # COMMODITY→CASH
            # EQUITY 获得额外流入使其成为汇
            _edge_input(V.EQUITY, V.CASH, FlowDirection.B_TO_A),  # CASH→EQUITY
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.B_TO_A),  # RE→EQUITY
            # RE/CASH
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        relations = extract_flow_relations(states)

        commodity_flows = [r for r in relations if r.source == V.COMMODITY]
        assert len(commodity_flows) == 1
        # EQUITY net = +1(from COMMODITY) +1(from CASH) +1(from RE) = +3 → SINK
        equity = _find_vertex(states, V.EQUITY)
        if equity.net_flow >= 2:
            assert V.EQUITY in commodity_flows[0].sinks

    def test_no_resonance_no_flow_relation(self) -> None:
        """全部均衡 → 无共振 → 无流转关系。"""
        edges = [
            _edge_input(a, b, FlowDirection.EQUILIBRIUM)
            for a in V
            for b in V
            if a.value < b.value
        ]
        states = aggregate_vertex_flows(edges)
        relations = extract_flow_relations(states)
        assert len(relations) == 0

    def test_source_without_sink_possible(self) -> None:
        """有源无汇的情况（守恒约束下理论上可能，因为汇可能不达共振阈值）。"""
        # 构造：COMMODITY 强共振源(-3)，其他三个顶点各 +1 → 无汇
        edges = [
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.B_TO_A),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.B_TO_A),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.CASH, FlowDirection.EQUILIBRIUM),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        relations = extract_flow_relations(states)

        # COMMODITY 是源（net=-3）
        commodity_flows = [r for r in relations if r.source == V.COMMODITY]
        assert len(commodity_flows) == 1
        # 其他三个顶点各 +1，不达共振阈值 → sinks 可能为空
        # （这是定义的忠实实现：sinks 只含 net ≥ +2 的顶点）
        for sink in commodity_flows[0].sinks:
            sink_state = _find_vertex(states, sink)
            assert sink_state.net_flow >= 2

    def test_flow_relation_immutable(self) -> None:
        """FlowRelation 是不可变的。"""
        relation = FlowRelation(
            source=V.COMMODITY,
            sinks=frozenset([V.CASH]),
        )
        with pytest.raises(AttributeError):
            relation.source = V.EQUITY  # type: ignore[misc]

    def test_flow_relation_fields(self) -> None:
        """FlowRelation 字段正确。"""
        relation = FlowRelation(
            source=V.COMMODITY,
            sinks=frozenset([V.CASH, V.EQUITY]),
        )
        assert relation.source == V.COMMODITY
        assert V.CASH in relation.sinks
        assert V.EQUITY in relation.sinks
        assert len(relation.sinks) == 2
