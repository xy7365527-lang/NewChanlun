"""双 TF 确定性测试

验证 TFOrchestrator 的核心行为：
  - 各 TF 事件流与单独运行完全一致（无串扰）
  - 多 TF seek 后状态 === 逐步推进的状态
  - EventBus 正确分区
  - 时间戳对齐正确性
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest

from newchan.bi_engine import BiEngine
from newchan.events import DomainEvent, InvariantViolation
from newchan.orchestrator.bus import EventBus, TaggedEvent
from newchan.orchestrator.timeframes import TFOrchestrator
from newchan.replay import ReplaySession
from newchan.types import Bar


# ── 辅助函数 ──────────────────────────────────────────────────────


def _bar(ts_offset: int, o: float, h: float, l: float, c: float) -> Bar:
    """从偏移序号创建 Bar（每 bar 间隔 1 分钟）。"""
    return Bar(
        ts=datetime(2024, 1, 1, tzinfo=timezone.utc)
        + timedelta(minutes=ts_offset),
        open=o,
        high=h,
        low=l,
        close=c,
    )


def _generate_1m_bars(n: int = 120) -> list[Bar]:
    """生成 1m 锯齿形 bars，足够产生多笔。

    使用 16-bar 周期：8 bar 下降 + 8 bar 上升。
    """
    bars: list[Bar] = []
    for i in range(n):
        cycle_pos = i % 16
        if cycle_pos < 8:
            base = 100 - cycle_pos * 5
        else:
            base = 60 + (cycle_pos - 8) * 5
        h = base + 1.5
        l = base - 1.5
        o = base + 0.5
        c = base - 0.5
        bars.append(_bar(i, o, h, l, c))
    return bars


# =====================================================================
# EventBus 单元测试
# =====================================================================


class TestEventBus:
    """EventBus 事件收集器测试。"""

    def test_push_and_drain(self):
        """push → drain 返回所有事件并清空。"""
        bus = EventBus()
        from newchan.events import StrokeCandidate
        from newchan.fingerprint import compute_event_id

        ev = StrokeCandidate(
            bar_idx=0, bar_ts=1000.0, seq=0,
            event_id=compute_event_id(
                bar_idx=0, bar_ts=1000.0, event_type="stroke_candidate",
                seq=0, payload={"stroke_id": 0, "direction": "up",
                                "i0": 0, "i1": 5, "p0": 100.0, "p1": 105.0},
            ),
            stroke_id=0, direction="up",
            i0=0, i1=5, p0=100.0, p1=105.0,
        )
        bus.push("5m", [ev])
        bus.push("30m", [ev])

        all_events = bus.drain()
        assert len(all_events) == 2
        assert all_events[0].tf == "5m"
        assert all_events[1].tf == "30m"
        assert bus.count == 0

    def test_drain_by_tf(self):
        """drain_by_tf 只取指定 TF，保留其它。"""
        bus = EventBus()
        from newchan.events import StrokeCandidate

        ev = StrokeCandidate(
            bar_idx=0, bar_ts=1000.0, seq=0, event_id="test",
            stroke_id=0, direction="up",
            i0=0, i1=5, p0=100.0, p1=105.0,
        )
        bus.push("5m", [ev])
        bus.push("30m", [ev])

        five_m = bus.drain_by_tf("5m")
        assert len(five_m) == 1
        assert bus.count == 1  # 30m 还在

    def test_empty_bus(self):
        """空 bus → drain 返回空列表。"""
        bus = EventBus()
        assert bus.drain() == []
        assert bus.count == 0


# =====================================================================
# TFOrchestrator 核心测试
# =====================================================================


class TestTFOrchestrator:
    """TFOrchestrator 多级别调度测试。"""

    def test_single_tf_equivalent(self):
        """单 TF orchestrator === 直接用 ReplaySession。"""
        bars = _generate_1m_bars(60)
        orch = TFOrchestrator("sid", bars, ["5m"])
        direct_engine = BiEngine()
        direct_session = ReplaySession("sid_direct", bars, direct_engine)

        # 步进完所有 bar
        for _ in range(len(bars)):
            result = orch.step(1)
            direct_session.step(1)

        orch_strokes = orch.base_session.engine.current_strokes
        direct_strokes = direct_session.engine.current_strokes
        assert len(orch_strokes) == len(direct_strokes)
        for a, b in zip(orch_strokes, direct_strokes):
            assert a.i0 == b.i0
            assert a.i1 == b.i1
            assert a.direction == b.direction

    def test_two_tf_no_crosstalk(self):
        """5m + 30m 同时运行，各 TF 事件互不干扰。"""
        bars = _generate_1m_bars(120)
        orch = TFOrchestrator("sid", bars, ["5m", "30m"])

        all_5m_events: list[TaggedEvent] = []
        all_30m_events: list[TaggedEvent] = []

        for _ in range(len(bars)):
            orch.step(1)
            tagged = orch.bus.drain()
            for te in tagged:
                if te.tf == "5m":
                    all_5m_events.append(te)
                elif te.tf == "30m":
                    all_30m_events.append(te)

        # 两个 TF 都应该有事件（如果数据足够产生笔的话）
        # 不做数量断言（取决于具体数据），但确保标签正确
        for te in all_5m_events:
            assert te.tf == "5m"
        for te in all_30m_events:
            assert te.tf == "30m"

    def test_two_tf_independent_strokes(self):
        """双 TF 的最终 strokes 互相独立。"""
        bars = _generate_1m_bars(120)
        orch = TFOrchestrator("sid", bars, ["5m", "30m"])

        for _ in range(len(bars)):
            orch.step(1)

        strokes_5m = orch.sessions["5m"].engine.current_strokes
        strokes_30m = orch.sessions["30m"].engine.current_strokes

        # 30m 的笔数应该 ≤ 5m（更少的 bar 产生更少的笔）
        assert len(strokes_30m) <= len(strokes_5m) + 1  # +1 容差

    def test_orchestrator_seek_consistency(self):
        """seek 后状态 === 从头逐步推进到该位置的状态。"""
        bars = _generate_1m_bars(120)

        # 方式 1：逐步推进到 bar 80
        orch1 = TFOrchestrator("sid1", bars, ["5m", "30m"])
        for _ in range(80):
            orch1.step(1)
        strokes_step = orch1.sessions["5m"].engine.current_strokes

        # 方式 2：seek 到 bar 79（0-based，含该 bar）
        orch2 = TFOrchestrator("sid2", bars, ["5m", "30m"])
        orch2.seek(79)
        strokes_seek = orch2.sessions["5m"].engine.current_strokes

        assert len(strokes_step) == len(strokes_seek)
        for a, b in zip(strokes_step, strokes_seek):
            assert a.i0 == b.i0
            assert a.i1 == b.i1
            assert a.direction == b.direction

    def test_orchestrator_deterministic(self):
        """两次独立运行 → 各 TF 事件流完全相同。"""
        bars = _generate_1m_bars(120)

        def run_once():
            orch = TFOrchestrator("sid", bars, ["5m", "30m"])
            event_ids: dict[str, list[str]] = {"5m": [], "30m": []}
            for _ in range(len(bars)):
                orch.step(1)
                for te in orch.bus.drain():
                    event_ids[te.tf].append(te.event.event_id)
            return event_ids

        r1 = run_once()
        r2 = run_once()
        assert r1["5m"] == r2["5m"]
        assert r1["30m"] == r2["30m"]

    def test_no_violations_in_normal_flow(self):
        """正常数据流 → 无不变量违规。"""
        bars = _generate_1m_bars(120)
        orch = TFOrchestrator("sid", bars, ["5m", "30m"])

        violations = []
        for _ in range(len(bars)):
            orch.step(1)
            for te in orch.bus.drain():
                if isinstance(te.event, InvariantViolation):
                    violations.append(te)

        assert violations == [], f"意外违规: {violations}"

    def test_higher_tf_has_fewer_bars(self):
        """30m TF 的 bar 数应该少于 5m TF。"""
        bars = _generate_1m_bars(120)
        orch = TFOrchestrator("sid", bars, ["5m", "30m"])

        assert orch.sessions["30m"].total_bars < orch.sessions["5m"].total_bars

    def test_empty_timeframes_raises(self):
        """空 timeframes 列表 → ValueError。"""
        bars = _generate_1m_bars(10)
        with pytest.raises(ValueError):
            TFOrchestrator("sid", bars, [])

    def test_step_count_respected(self):
        """step(5) 步进 5 根 base TF bar。"""
        bars = _generate_1m_bars(60)
        orch = TFOrchestrator("sid", bars, ["5m", "30m"])
        result = orch.step(5)
        base_snaps = result["5m"]
        assert len(base_snaps) == 5
        assert orch.current_idx == 5

    def test_get_status(self):
        """get_status 返回各 TF 的状态。"""
        bars = _generate_1m_bars(60)
        orch = TFOrchestrator("sid", bars, ["5m", "30m"])
        orch.step(10)
        status = orch.get_status()
        assert "5m" in status
        assert "30m" in status
        assert status["5m"]["current_idx"] == 10
