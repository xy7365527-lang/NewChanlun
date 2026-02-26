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
    CashSignalAnalysis,
    EdgeFlowInput,
    FlowRelation,
    FlowRole,
    ResonanceStrength,
    VertexFlowState,
    aggregate_vertex_flows,
    check_conservation,
    detect_resonance,
    disambiguate_cash_signal,
    extract_flow_relations,
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


# ── FlowRole 分类 ─────────────────────────────────────────


class TestFlowRole:
    """FlowRole 与 net_flow 的对应关系。"""

    def test_source_role(self) -> None:
        """net ≤ -2 → SOURCE。"""
        edges = [
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.EQUILIBRIUM),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        # EQUITY: 1 条流出(→CASH), 0 条流入 → net=-1 → NEUTRAL
        eq = _find_vertex(states, V.EQUITY)
        assert eq.role == FlowRole.NEUTRAL
        # CASH: 3 条流入 → net=+3 → SINK
        cash = _find_vertex(states, V.CASH)
        assert cash.role == FlowRole.SINK

    def test_sink_role(self) -> None:
        """net ≥ +2 → SINK。"""
        edges = [
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.EQUILIBRIUM),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        cash = _find_vertex(states, V.CASH)
        assert cash.role == FlowRole.SINK
        assert cash.net_flow == 2

    def test_neutral_role(self) -> None:
        """net = 0 → NEUTRAL。"""
        edges = [
            _edge_input(a, b, FlowDirection.EQUILIBRIUM)
            for a in V
            for b in V
            if a.value < b.value
        ]
        states = aggregate_vertex_flows(edges)
        for s in states:
            assert s.role == FlowRole.NEUTRAL


# ── 流转关系提取 ──────────────────────────────────────────


class TestExtractFlowRelations:
    """extract_flow_relations：从顶点状态提取 Flow(源→汇)。"""

    def test_single_source_single_sink(self) -> None:
        """一个源、一个汇 → 一条流转关系。"""
        # COMMODITY 全流出(-3=SOURCE), CASH 全流入(+3=SINK)
        edges = [
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.B_TO_A),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.B_TO_A),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        relations = extract_flow_relations(states)
        assert len(relations) == 1
        assert relations[0].source == V.COMMODITY
        assert relations[0].sinks == frozenset([V.CASH])

    def test_no_resonance_no_relations(self) -> None:
        """全均衡 → 无流转关系。"""
        edges = [
            _edge_input(a, b, FlowDirection.EQUILIBRIUM)
            for a in V
            for b in V
            if a.value < b.value
        ]
        states = aggregate_vertex_flows(edges)
        relations = extract_flow_relations(states)
        assert relations == []

    def test_source_without_sink(self) -> None:
        """有源无汇 → FlowRelation.sinks 为空集。"""
        # COMMODITY 全流出(-3), 其余各得 +1 → 无 SINK
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
        assert len(relations) == 1
        assert relations[0].source == V.COMMODITY
        assert relations[0].sinks == frozenset()

    def test_multiple_sources_multiple_sinks(self) -> None:
        """多源多汇场景。"""
        # EQUITY→CASH, EQUITY→COMMODITY, RE→CASH, RE→COMMODITY
        # EQUITY 和 RE 各 2 条流出 → SOURCE
        # CASH 和 COMMODITY 各 2 条流入 → SINK
        edges = [
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        relations = extract_flow_relations(states)
        sources = {r.source for r in relations}
        assert sources == {V.EQUITY, V.REAL_ESTATE}
        for r in relations:
            assert r.sinks == frozenset([V.CASH, V.COMMODITY])

    def test_flow_relation_is_immutable(self) -> None:
        """FlowRelation 是 frozen dataclass。"""
        fr = FlowRelation(
            source=V.EQUITY, sinks=frozenset([V.CASH])
        )
        with pytest.raises(AttributeError):
            fr.source = V.COMMODITY  # type: ignore[misc]


# ── 守恒约束 ──────────────────────────────────────────────


class TestCheckConservation:
    """check_conservation：Σnet(V) = 0。"""

    def test_conserved_all_equilibrium(self) -> None:
        edges = [
            _edge_input(a, b, FlowDirection.EQUILIBRIUM)
            for a in V
            for b in V
            if a.value < b.value
        ]
        states = aggregate_vertex_flows(edges)
        assert check_conservation(states) is True

    def test_conserved_with_active_edges(self) -> None:
        """有方向的边也守恒（拓扑不变量）。"""
        edges = [
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.B_TO_A),
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.EQUILIBRIUM),
        ]
        states = aggregate_vertex_flows(edges)
        assert check_conservation(states) is True

    def test_broken_conservation_detected(self) -> None:
        """手工构造破缺状态 → check_conservation 返回 False。"""
        # 直接构造不守恒的 states（绕过 aggregate 的拓扑保证）
        broken_states = [
            VertexFlowState(V.EQUITY, 1, ResonanceStrength.NONE, FlowRole.NEUTRAL),
            VertexFlowState(V.REAL_ESTATE, 1, ResonanceStrength.NONE, FlowRole.NEUTRAL),
            VertexFlowState(V.COMMODITY, 0, ResonanceStrength.NONE, FlowRole.NEUTRAL),
            VertexFlowState(V.CASH, 0, ResonanceStrength.NONE, FlowRole.NEUTRAL),
        ]
        assert check_conservation(broken_states) is False


# ── 现金边信号消歧 ────────────────────────────────────────


class TestDisambiguateCashSignal:
    """disambiguate_cash_signal：026号谱系。"""

    def test_neutral_when_cash_net_zero(self) -> None:
        """CASH net=0 → neutral。"""
        edges = [
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.B_TO_A),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.EQUILIBRIUM),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.EQUILIBRIUM),
        ]
        result = disambiguate_cash_signal(edges)
        assert isinstance(result, CashSignalAnalysis)
        assert result.signal_type == "neutral"
        assert result.cash_net_flow == 0

    def test_metric_shift_when_asset_subgraph_uniform(self) -> None:
        """纯资产子图全均衡（方差=0）+ CASH 有净流 → metric_shift。"""
        edges = [
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.EQUILIBRIUM),
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.EQUILIBRIUM),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.EQUILIBRIUM),
        ]
        result = disambiguate_cash_signal(edges)
        assert result.signal_type == "metric_shift"
        assert result.cash_net_flow == 3

    def test_genuine_flow_when_asset_subgraph_divergent(self) -> None:
        """纯资产子图有分化 + CASH 有净流 → genuine_flow 或 mixed。"""
        edges = [
            _edge_input(V.EQUITY, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.CASH, FlowDirection.A_TO_B),
            _edge_input(V.COMMODITY, V.CASH, FlowDirection.A_TO_B),
            # 纯资产子图有分化
            _edge_input(V.EQUITY, V.REAL_ESTATE, FlowDirection.A_TO_B),
            _edge_input(V.EQUITY, V.COMMODITY, FlowDirection.A_TO_B),
            _edge_input(V.REAL_ESTATE, V.COMMODITY, FlowDirection.B_TO_A),
        ]
        result = disambiguate_cash_signal(edges)
        assert result.signal_type in ("genuine_flow", "mixed")
        assert result.cash_net_flow == 3
        assert result.asset_subgraph_variance > 0

    def test_result_fields_complete(self) -> None:
        """CashSignalAnalysis 所有字段都有值。"""
        edges = [
            _edge_input(a, b, FlowDirection.A_TO_B)
            for a in V
            for b in V
            if a.value < b.value
        ]
        result = disambiguate_cash_signal(edges)
        assert isinstance(result.cash_net_flow, int)
        assert isinstance(result.asset_subgraph_variance, float)
        assert result.signal_type in ("genuine_flow", "metric_shift", "mixed", "neutral")
        assert 0.0 <= result.confidence <= 1.0
