"""MoveProtocol + 适配器 — 单元测试

覆盖：
  1. SegmentAsComponent 满足 MoveProtocol
  2. MoveAsComponent 满足 MoveProtocol
  3. SegmentAsComponent 各字段映射正确
  4. MoveAsComponent 各字段映射正确
  5. SegmentAsComponent.completed 逻辑（confirmed + kind 组合）
  6. adapt_segments 批量转换
  7. adapt_moves 批量转换
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v0 import Segment
from newchan.a_move_v1 import Move
from newchan.a_level_protocol import (
    MoveProtocol,
    SegmentAsComponent,
    MoveAsComponent,
    adapt_segments,
    adapt_moves,
)


# ── helpers ──


def _seg(
    s0: int = 0,
    s1: int = 2,
    direction: str = "up",
    high: float = 20.0,
    low: float = 10.0,
    confirmed: bool = True,
    kind: str = "settled",
) -> Segment:
    """快速构造 Segment。"""
    return Segment(
        s0=s0, s1=s1, i0=s0 * 5, i1=s1 * 5,
        direction=direction, high=high, low=low,
        confirmed=confirmed, kind=kind,
    )


def _move(
    kind: str = "consolidation",
    direction: str = "up",
    seg_start: int = 0,
    seg_end: int = 2,
    settled: bool = True,
    high: float = 25.0,
    low: float = 8.0,
) -> Move:
    """快速构造 Move。"""
    return Move(
        kind=kind,
        direction=direction,
        seg_start=seg_start,
        seg_end=seg_end,
        zs_start=0,
        zs_end=0,
        zs_count=1,
        settled=settled,
        high=high,
        low=low,
        first_seg_s0=0,
        last_seg_s1=2,
    )


# ── 1. Protocol 满足性 ──


class TestProtocolSatisfaction:
    """验证适配器满足 runtime_checkable Protocol。"""

    def test_segment_satisfies_protocol(self) -> None:
        seg = _seg()
        comp = SegmentAsComponent(_segment=seg, _component_idx=0)
        assert isinstance(comp, MoveProtocol)

    def test_move_satisfies_protocol(self) -> None:
        m = _move()
        comp = MoveAsComponent(_move=m, _component_idx=0, _level_id=1)
        assert isinstance(comp, MoveProtocol)


# ── 2. 字段映射 ──


class TestSegmentAdapterFields:
    """SegmentAsComponent 各字段映射正确。"""

    def test_segment_adapter_fields(self) -> None:
        seg = _seg(s0=3, s1=5, direction="down", high=30.0, low=12.0,
                   confirmed=True, kind="settled")
        comp = SegmentAsComponent(_segment=seg, _component_idx=7)

        assert comp.component_idx == 7
        assert comp.high == 30.0
        assert comp.low == 12.0
        assert comp.direction == "down"
        assert comp.completed is True
        assert comp.level_id == 0


class TestMoveAdapterFields:
    """MoveAsComponent 各字段映射正确。"""

    def test_move_adapter_fields(self) -> None:
        m = _move(direction="down", high=35.0, low=5.0, settled=True)
        comp = MoveAsComponent(_move=m, _component_idx=3, _level_id=2)

        assert comp.component_idx == 3
        assert comp.high == 35.0
        assert comp.low == 5.0
        assert comp.direction == "down"
        assert comp.completed is True
        assert comp.level_id == 2


# ── 3. completed 逻辑 ──


class TestSegmentCompletedLogic:
    """confirmed=True + kind="settled" → True；其他组合 → False。"""

    @pytest.mark.parametrize(
        "confirmed, kind, expected",
        [
            (True, "settled", True),
            (True, "candidate", False),
            (False, "settled", False),
            (False, "candidate", False),
        ],
    )
    def test_segment_completed_logic(
        self, confirmed: bool, kind: str, expected: bool
    ) -> None:
        seg = _seg(confirmed=confirmed, kind=kind)
        comp = SegmentAsComponent(_segment=seg, _component_idx=0)
        assert comp.completed is expected


# ── 4. 批量适配 ──


class TestAdaptHelpers:
    """adapt_segments / adapt_moves 批量转换正确。"""

    def test_adapt_segments_helper(self) -> None:
        segs = [
            _seg(s0=0, s1=2, direction="up", high=20.0, low=10.0),
            _seg(s0=2, s1=4, direction="down", high=18.0, low=8.0),
            _seg(s0=4, s1=6, direction="up", high=22.0, low=12.0),
        ]
        comps = adapt_segments(segs)

        assert len(comps) == 3
        for i, c in enumerate(comps):
            assert isinstance(c, SegmentAsComponent)
            assert isinstance(c, MoveProtocol)
            assert c.component_idx == i
            assert c.high == segs[i].high
            assert c.low == segs[i].low
            assert c.direction == segs[i].direction

    def test_adapt_moves_helper(self) -> None:
        moves = [
            _move(direction="up", high=25.0, low=8.0, settled=True),
            _move(direction="down", high=22.0, low=6.0, settled=False),
        ]
        comps = adapt_moves(moves, level_id=1)

        assert len(comps) == 2
        for i, c in enumerate(comps):
            assert isinstance(c, MoveAsComponent)
            assert isinstance(c, MoveProtocol)
            assert c.component_idx == i
            assert c.level_id == 1
            assert c.high == moves[i].high
            assert c.low == moves[i].low
            assert c.direction == moves[i].direction
            assert c.completed == moves[i].settled
