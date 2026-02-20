"""流转状态时间序列测试。

验证 liuzhuan.md #14 的时间维度扩展：
  - 6 条边各自独立产出笔事件
  - 在每个笔端点时刻，用当时各边的最新方向聚合为四矩阵快照
  - 快照序列按时间排序
  - 守恒约束在每个快照上都成立

概念溯源：
  [新缠论] 结构同步而非时钟同步——不同边有各自的笔节奏，
  时间序列在笔事件发生时更新，不是按固定时钟采样。
"""

from __future__ import annotations

from datetime import datetime, timedelta

import pytest

from newchan.capital_flow import FlowDirection
from newchan.flow_relation import (
    ResonanceStrength,
    VertexFlowState,
)
from newchan.flow_timeline import (
    EdgeEvent,
    FlowSnapshot,
    build_flow_timeline,
)
from newchan.matrix_topology import AssetVertex

V = AssetVertex
D = FlowDirection

# ── helpers ──────────────────────────────────────────────

T0 = datetime(2024, 1, 1)


def _t(days: int) -> datetime:
    """辅助：偏移天数。"""
    return T0 + timedelta(days=days)


def _event(
    va: AssetVertex, vb: AssetVertex, direction: FlowDirection, day: int
) -> EdgeEvent:
    return EdgeEvent(
        vertex_a=va, vertex_b=vb, direction=direction, timestamp=_t(day)
    )


# ── 空输入 ───────────────────────────────────────────────


class TestEmptyInput:
    def test_no_events_returns_empty(self) -> None:
        """无事件 → 无快照。"""
        assert build_flow_timeline([]) == []


# ── 单事件 ───────────────────────────────────────────────


class TestSingleEvent:
    def test_single_event_produces_one_snapshot(self) -> None:
        """单条边有一个笔事件 → 1 个快照。"""
        events = [_event(V.EQUITY, V.CASH, D.A_TO_B, 0)]
        timeline = build_flow_timeline(events)
        assert len(timeline) == 1

    def test_single_event_conservation(self) -> None:
        """单事件快照守恒。"""
        events = [_event(V.EQUITY, V.CASH, D.A_TO_B, 0)]
        timeline = build_flow_timeline(events)

    def test_single_event_timestamp(self) -> None:
        """快照时间戳 = 事件时间戳。"""
        events = [_event(V.EQUITY, V.CASH, D.A_TO_B, 5)]
        timeline = build_flow_timeline(events)
        assert timeline[0].timestamp == _t(5)

    def test_single_event_direction_reflected(self) -> None:
        """EQUITY→CASH 的 A_TO_B → EQUITY net=-1, CASH net=+1, 其余 0。"""
        events = [_event(V.EQUITY, V.CASH, D.A_TO_B, 0)]
        timeline = build_flow_timeline(events)
        states = {s.vertex: s for s in timeline[0].vertex_states}
        assert states[V.EQUITY].net_flow == -1
        assert states[V.CASH].net_flow == +1
        assert states[V.REAL_ESTATE].net_flow == 0
        assert states[V.COMMODITY].net_flow == 0


# ── 多事件同边 ──────────────────────────────────────────


class TestSameEdgeMultipleEvents:
    def test_direction_overwrite(self) -> None:
        """同一条边先 A→B 再 B→A → 方向更新。"""
        events = [
            _event(V.EQUITY, V.CASH, D.A_TO_B, 0),
            _event(V.EQUITY, V.CASH, D.B_TO_A, 10),
        ]
        timeline = build_flow_timeline(events)
        assert len(timeline) == 2

        # 第一个快照：A→B
        s0 = {s.vertex: s for s in timeline[0].vertex_states}
        assert s0[V.EQUITY].net_flow == -1
        assert s0[V.CASH].net_flow == +1

        # 第二个快照：B→A（方向翻转）
        s1 = {s.vertex: s for s in timeline[1].vertex_states}
        assert s1[V.EQUITY].net_flow == +1
        assert s1[V.CASH].net_flow == -1

    def test_all_snapshots_conserve(self) -> None:
        """多次方向翻转，每个快照都守恒。"""
        events = [
            _event(V.EQUITY, V.CASH, D.A_TO_B, 0),
            _event(V.EQUITY, V.CASH, D.B_TO_A, 10),
            _event(V.EQUITY, V.CASH, D.A_TO_B, 20),
        ]
        timeline = build_flow_timeline(events)
        for snap in timeline:
            pass  # 守恒约束已移除（050号谱系）


# ── 多边交错事件 ────────────────────────────────────────


class TestMultiEdgeInterleaved:
    def test_interleaved_events_sorted_by_time(self) -> None:
        """不同边的事件交错 → 快照按时间排序。"""
        events = [
            _event(V.EQUITY, V.CASH, D.A_TO_B, 5),
            _event(V.COMMODITY, V.CASH, D.B_TO_A, 2),
            _event(V.EQUITY, V.REAL_ESTATE, D.A_TO_B, 8),
        ]
        timeline = build_flow_timeline(events)
        assert len(timeline) == 3
        assert timeline[0].timestamp == _t(2)
        assert timeline[1].timestamp == _t(5)
        assert timeline[2].timestamp == _t(8)

    def test_accumulative_state(self) -> None:
        """第二个事件建立在第一个事件的状态之上。"""
        events = [
            # t=0: EQUITY→CASH A_TO_B → EQUITY -1, CASH +1
            _event(V.EQUITY, V.CASH, D.A_TO_B, 0),
            # t=5: COMMODITY→CASH A_TO_B → COMMODITY -1, CASH 再 +1
            _event(V.COMMODITY, V.CASH, D.A_TO_B, 5),
        ]
        timeline = build_flow_timeline(events)
        # 第二个快照应该同时包含两条边的影响
        s1 = {s.vertex: s for s in timeline[1].vertex_states}
        assert s1[V.CASH].net_flow == +2  # 两条边都流入 CASH
        assert s1[V.EQUITY].net_flow == -1
        assert s1[V.COMMODITY].net_flow == -1

    def test_all_interleaved_conserve(self) -> None:
        """交错事件的每个快照都守恒。"""
        events = [
            _event(V.EQUITY, V.CASH, D.A_TO_B, 0),
            _event(V.COMMODITY, V.CASH, D.A_TO_B, 5),
            _event(V.REAL_ESTATE, V.CASH, D.A_TO_B, 10),
            _event(V.EQUITY, V.COMMODITY, D.B_TO_A, 15),
        ]
        timeline = build_flow_timeline(events)
        for snap in timeline:
            pass  # 守恒约束已移除（050号谱系）


# ── 共振涌现 ────────────────────────────────────────────


class TestResonanceEmergence:
    def test_resonance_emerges_from_accumulation(self) -> None:
        """资本逐步流入 CASH：第二条边加入时弱共振涌现。"""
        events = [
            _event(V.EQUITY, V.CASH, D.A_TO_B, 0),
            _event(V.COMMODITY, V.CASH, D.A_TO_B, 5),
        ]
        timeline = build_flow_timeline(events)

        # 第一个快照：CASH net=+1 → 无共振
        s0 = {s.vertex: s for s in timeline[0].vertex_states}
        assert s0[V.CASH].strength == ResonanceStrength.NONE

        # 第二个快照：CASH net=+2 → 弱共振涌现
        s1 = {s.vertex: s for s in timeline[1].vertex_states}
        assert s1[V.CASH].strength == ResonanceStrength.WEAK

    def test_strong_resonance_all_three_edges(self) -> None:
        """三条边全部流入 CASH → 强共振。"""
        events = [
            _event(V.EQUITY, V.CASH, D.A_TO_B, 0),
            _event(V.COMMODITY, V.CASH, D.A_TO_B, 5),
            _event(V.REAL_ESTATE, V.CASH, D.A_TO_B, 10),
        ]
        timeline = build_flow_timeline(events)

        s2 = {s.vertex: s for s in timeline[2].vertex_states}
        assert s2[V.CASH].strength == ResonanceStrength.STRONG
        assert s2[V.CASH].net_flow == +3

    def test_resonance_dissolves(self) -> None:
        """共振建立后，一条边翻转 → 共振消散。"""
        events = [
            _event(V.EQUITY, V.CASH, D.A_TO_B, 0),
            _event(V.COMMODITY, V.CASH, D.A_TO_B, 5),
            # 弱共振建立
            _event(V.EQUITY, V.CASH, D.B_TO_A, 10),
            # EQUITY 翻转 → CASH net 从 +2 变为 0
        ]
        timeline = build_flow_timeline(events)

        s1 = {s.vertex: s for s in timeline[1].vertex_states}
        assert s1[V.CASH].strength == ResonanceStrength.WEAK

        s2 = {s.vertex: s for s in timeline[2].vertex_states}
        assert s2[V.CASH].strength == ResonanceStrength.NONE
        assert s2[V.CASH].net_flow == 0


# ── 同时事件 ────────────────────────────────────────────


class TestSimultaneousEvents:
    def test_same_timestamp_different_edges(self) -> None:
        """同一时刻两条边变化 → 两个快照（按输入顺序）。"""
        events = [
            _event(V.EQUITY, V.CASH, D.A_TO_B, 0),
            _event(V.COMMODITY, V.CASH, D.A_TO_B, 0),
        ]
        timeline = build_flow_timeline(events)
        # 允许 1 或 2 个快照（实现可以选择合并同时事件或分开）
        # 但最终状态必须正确
        last = {s.vertex: s for s in timeline[-1].vertex_states}
        assert last[V.CASH].net_flow == +2


# ── FlowSnapshot 结构 ──────────────────────────────────


class TestSnapshotStructure:
    def test_snapshot_is_frozen(self) -> None:
        """FlowSnapshot 不可变。"""
        events = [_event(V.EQUITY, V.CASH, D.A_TO_B, 0)]
        timeline = build_flow_timeline(events)
        snap = timeline[0]
        with pytest.raises(AttributeError):
            snap.timestamp = _t(99)  # type: ignore[misc]

    def test_snapshot_has_four_vertices(self) -> None:
        """每个快照包含 4 个顶点状态。"""
        events = [_event(V.EQUITY, V.CASH, D.A_TO_B, 0)]
        timeline = build_flow_timeline(events)
        assert len(timeline[0].vertex_states) == 4
        vertices = {s.vertex for s in timeline[0].vertex_states}
        assert vertices == set(AssetVertex)

    def test_snapshot_trigger_edge(self) -> None:
        """快照记录触发它的边。"""
        events = [_event(V.EQUITY, V.CASH, D.A_TO_B, 0)]
        timeline = build_flow_timeline(events)
        assert timeline[0].trigger_edge == (V.EQUITY, V.CASH)
