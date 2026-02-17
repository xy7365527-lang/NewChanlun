"""P6 事件 level_id 扩展测试

验证：
  - 6 个中枢/走势事件类新增 level_id 字段（默认值 1，向后兼容）
  - RecursiveLevelEngine 产生的事件携带正确的 level_id
  - EventBus push_level / drain_by_level 按级别路由事件
"""

from __future__ import annotations

import pytest

from newchan.events import (
    DomainEvent,
    MoveCandidateV1,
    MoveInvalidateV1,
    MoveSettleV1,
    ZhongshuCandidateV1,
    ZhongshuInvalidateV1,
    ZhongshuSettleV1,
)
from newchan.orchestrator.bus import EventBus


# ── 辅助 ──


def _ev(cls: type, **kwargs: object) -> DomainEvent:
    """用默认 bar_idx/bar_ts/seq 构造事件。"""
    defaults = {"bar_idx": 0, "bar_ts": 0.0, "seq": 0}
    defaults.update(kwargs)
    return cls(**defaults)


# 6 个需要 level_id 的事件类
LEVEL_EVENT_CLASSES = [
    ZhongshuCandidateV1,
    ZhongshuSettleV1,
    ZhongshuInvalidateV1,
    MoveCandidateV1,
    MoveSettleV1,
    MoveInvalidateV1,
]


# ── 事件 level_id 字段测试 ──


class TestEventLevelIdField:
    """6 个中枢/走势事件类的 level_id 字段。"""

    @pytest.mark.parametrize("cls", LEVEL_EVENT_CLASSES)
    def test_default_level_id_is_1(self, cls: type) -> None:
        """默认 level_id=1，保证向后兼容。"""
        ev = _ev(cls)
        assert ev.level_id == 1

    @pytest.mark.parametrize("cls", LEVEL_EVENT_CLASSES)
    def test_explicit_level_id(self, cls: type) -> None:
        """可以显式指定 level_id。"""
        ev = _ev(cls, level_id=3)
        assert ev.level_id == 3

    @pytest.mark.parametrize("cls", LEVEL_EVENT_CLASSES)
    def test_frozen_level_id(self, cls: type) -> None:
        """level_id 不可变（frozen dataclass）。"""
        ev = _ev(cls, level_id=2)
        with pytest.raises(AttributeError):
            ev.level_id = 5  # type: ignore[misc]


# ── 递归层事件 level_id 传递测试 ──


class TestRecursiveLevelEngineEventLevelId:
    """RecursiveLevelEngine 产生的事件携带正确的 level_id。"""

    def test_level2_events_carry_level_id_2(self) -> None:
        """level_id=2 引擎产生的中枢/走势事件 level_id=2。"""
        from newchan.core.recursion.move_state import MoveSnapshot
        from newchan.core.recursion.recursive_level_engine import RecursiveLevelEngine
        from newchan.a_move_v1 import Move

        # 构造 3 个价格重叠的 settled Move 以触发中枢
        moves = [
            Move(kind="consolidation", direction="up", seg_start=0, seg_end=1,
                 zs_start=0, zs_end=0, zs_count=1, settled=True,
                 high=10.0, low=5.0, first_seg_s0=0, last_seg_s1=1),
            Move(kind="consolidation", direction="down", seg_start=2, seg_end=3,
                 zs_start=0, zs_end=0, zs_count=1, settled=True,
                 high=12.0, low=6.0, first_seg_s0=2, last_seg_s1=3),
            Move(kind="consolidation", direction="up", seg_start=4, seg_end=5,
                 zs_start=0, zs_end=0, zs_count=1, settled=True,
                 high=11.0, low=7.0, first_seg_s0=4, last_seg_s1=5),
        ]
        snap = MoveSnapshot(bar_idx=100, bar_ts=1000.0, moves=moves, events=[])

        engine = RecursiveLevelEngine(level_id=2)
        result = engine.process_move_snapshot(snap)

        # 收集所有有 level_id 属性的事件
        all_events = result.zhongshu_events + result.move_events
        events_with_level = [e for e in all_events if hasattr(e, "level_id")]

        # 必须有事件产出（3 个重叠 Move 至少产生 ZhongshuCandidateV1）
        assert len(events_with_level) > 0, "Expected at least one event with level_id"
        # 且它们的 level_id 应该是 2
        for ev in events_with_level:
            assert ev.level_id == 2, f"{type(ev).__name__}.level_id = {ev.level_id}, expected 2"

    def test_level3_events_carry_level_id_3(self) -> None:
        """level_id=3 引擎产生的事件 level_id=3。"""
        from newchan.core.recursion.move_state import MoveSnapshot
        from newchan.core.recursion.recursive_level_engine import RecursiveLevelEngine
        from newchan.a_move_v1 import Move

        # 同样 3 个重叠 Move
        moves = [
            Move(kind="consolidation", direction="up", seg_start=0, seg_end=1,
                 zs_start=0, zs_end=0, zs_count=1, settled=True,
                 high=10.0, low=5.0, first_seg_s0=0, last_seg_s1=1),
            Move(kind="consolidation", direction="down", seg_start=2, seg_end=3,
                 zs_start=0, zs_end=0, zs_count=1, settled=True,
                 high=12.0, low=6.0, first_seg_s0=2, last_seg_s1=3),
            Move(kind="consolidation", direction="up", seg_start=4, seg_end=5,
                 zs_start=0, zs_end=0, zs_count=1, settled=True,
                 high=11.0, low=7.0, first_seg_s0=4, last_seg_s1=5),
        ]
        snap = MoveSnapshot(bar_idx=200, bar_ts=2000.0, moves=moves, events=[])

        engine = RecursiveLevelEngine(level_id=3)
        result = engine.process_move_snapshot(snap)

        all_events = result.zhongshu_events + result.move_events
        events_with_level = [e for e in all_events if hasattr(e, "level_id")]

        assert len(events_with_level) > 0, "Expected at least one event with level_id"
        for ev in events_with_level:
            assert ev.level_id == 3, f"{type(ev).__name__}.level_id = {ev.level_id}, expected 3"


# ── EventBus level 路由测试 ──


class TestEventBusLevelRouting:
    """EventBus push_level / drain_by_level 按级别路由。"""

    def test_push_level_and_drain_by_level(self) -> None:
        """push_level 推入的事件可以通过 drain_by_level 取回。"""
        bus = EventBus()
        ev1 = _ev(ZhongshuCandidateV1, level_id=2)
        ev2 = _ev(MoveCandidateV1, level_id=3)

        bus.push_level(2, [ev1])
        bus.push_level(3, [ev2])

        level2_events = bus.drain_by_level(2)
        assert len(level2_events) == 1
        assert level2_events[0] is ev1

        level3_events = bus.drain_by_level(3)
        assert len(level3_events) == 1
        assert level3_events[0] is ev2

    def test_drain_by_level_empty(self) -> None:
        """drain_by_level 对空级别返回空列表。"""
        bus = EventBus()
        assert bus.drain_by_level(5) == []

    def test_push_level_preserves_other_levels(self) -> None:
        """drain_by_level 只消费指定级别，保留其他。"""
        bus = EventBus()
        ev1 = _ev(ZhongshuCandidateV1, level_id=2)
        ev2 = _ev(ZhongshuCandidateV1, level_id=3)
        bus.push_level(2, [ev1])
        bus.push_level(3, [ev2])

        _ = bus.drain_by_level(2)
        assert bus.count == 1  # level 3 still present

    def test_push_level_with_stream_id(self) -> None:
        """push_level 支持 stream_id 参数。"""
        bus = EventBus()
        ev = _ev(MoveCandidateV1, level_id=2)
        bus.push_level(2, [ev], stream_id="test_stream")

        events = bus.drain_by_level(2)
        assert len(events) == 1

    def test_push_level_coexists_with_push_tf(self) -> None:
        """push_level 和 push (tf) 共存，互不干扰。"""
        bus = EventBus()
        ev_level = _ev(ZhongshuCandidateV1, level_id=2)
        ev_tf = _ev(ZhongshuCandidateV1)

        bus.push_level(2, [ev_level])
        bus.push("5m", [ev_tf])

        assert bus.count == 2

        level_events = bus.drain_by_level(2)
        assert len(level_events) == 1

        tf_events = bus.drain_by_tf("5m")
        assert len(tf_events) == 1

        assert bus.count == 0
