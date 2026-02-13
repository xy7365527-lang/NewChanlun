"""diff_strokes 差分算法 — 单元测试

覆盖笔快照差分的各种边界情况：
  - 空→空、空→候选、候选→结算+新候选
  - 候选延伸、候选不变、笔消失
  - 多笔→少笔（合并）、公共前缀保留
  - 事件序号单调递增
"""

from __future__ import annotations

import pytest

from newchan.a_stroke import Stroke
from newchan.bi_differ import diff_strokes
from newchan.events import (
    StrokeCandidate,
    StrokeExtended,
    StrokeInvalidated,
    StrokeSettled,
)


# ── 辅助函数 ──────────────────────────────────────────────────────


def _mk(
    i0: int,
    i1: int,
    direction: str = "up",
    p0: float = 10.0,
    p1: float = 20.0,
    confirmed: bool = True,
) -> Stroke:
    """快速构造一个 Stroke。high/low 从 p0/p1 推导。"""
    return Stroke(
        i0=i0,
        i1=i1,
        direction=direction,
        high=max(p0, p1),
        low=min(p0, p1),
        p0=p0,
        p1=p1,
        confirmed=confirmed,
    )


# 固定的 bar_idx / bar_ts，测试中不关心具体值
_BAR_IDX = 10
_BAR_TS = 1707800000.0


def _diff(
    prev: list[Stroke],
    curr: list[Stroke],
    seq_start: int = 0,
) -> list:
    return diff_strokes(
        prev, curr, bar_idx=_BAR_IDX, bar_ts=_BAR_TS, seq_start=seq_start
    )


# =====================================================================
# 测试类
# =====================================================================


class TestDiffStrokes:
    """diff_strokes 差分算法单元测试。"""

    def test_empty_to_empty(self):
        """空→空：无事件。"""
        events = _diff([], [])
        assert events == []

    def test_empty_to_one_candidate(self):
        """空→一笔候选：产生 StrokeCandidate。"""
        cand = _mk(0, 5, "up", 10.0, 20.0, confirmed=False)
        events = _diff([], [cand])
        assert len(events) == 1
        e = events[0]
        assert isinstance(e, StrokeCandidate)
        assert e.event_type == "stroke_candidate"
        assert e.stroke_id == 0
        assert e.direction == "up"
        assert e.i0 == 0
        assert e.i1 == 5
        assert e.p0 == 10.0
        assert e.p1 == 20.0
        assert e.bar_idx == _BAR_IDX
        assert e.bar_ts == _BAR_TS

    def test_candidate_to_settled_plus_new_candidate(self):
        """一笔候选→前一笔结算+新候选：产生 StrokeSettled + StrokeCandidate。"""
        prev = [_mk(0, 5, "up", 10.0, 20.0, confirmed=False)]
        curr = [
            _mk(0, 5, "up", 10.0, 20.0, confirmed=True),
            _mk(5, 10, "down", 20.0, 12.0, confirmed=False),
        ]
        events = _diff(prev, curr)

        # 前一笔从 unconfirmed→confirmed → 不在公共前缀中（confirmed 不同）
        # 所以：invalidate prev[0]，然后 settled curr[0]，candidate curr[1]
        assert len(events) == 3
        assert isinstance(events[0], StrokeInvalidated)
        assert events[0].stroke_id == 0
        assert isinstance(events[1], StrokeSettled)
        assert events[1].stroke_id == 0
        assert events[1].direction == "up"
        assert isinstance(events[2], StrokeCandidate)
        assert events[2].stroke_id == 1
        assert events[2].direction == "down"

    def test_candidate_extended(self):
        """候选笔延伸（i1/p1 变化）：产生 StrokeInvalidated + StrokeExtended。

        因为 high/low 随终点变化，_strokes_equal 不相等，
        所以公共前缀为 0 → 先 invalidate 旧笔，再检测到 extended。
        """
        prev = [_mk(0, 5, "up", 10.0, 20.0, confirmed=False)]
        # 同起点、同方向、未确认，但终点变了
        curr = [_mk(0, 7, "up", 10.0, 25.0, confirmed=False)]
        events = _diff(prev, curr)

        assert len(events) == 2
        # 先 invalidate
        assert isinstance(events[0], StrokeInvalidated)
        assert events[0].stroke_id == 0
        # 再 extended
        e = events[1]
        assert isinstance(e, StrokeExtended)
        assert e.event_type == "stroke_extended"
        assert e.stroke_id == 0
        assert e.direction == "up"
        assert e.old_i1 == 5
        assert e.new_i1 == 7
        assert e.old_p1 == 20.0
        assert e.new_p1 == 25.0

    def test_candidate_unchanged(self):
        """候选笔不变：无事件。"""
        cand = _mk(0, 5, "up", 10.0, 20.0, confirmed=False)
        events = _diff([cand], [cand])
        assert events == []

    def test_stroke_invalidated(self):
        """笔消失：产生 StrokeInvalidated。"""
        prev = [_mk(0, 5, "up", 10.0, 20.0, confirmed=False)]
        curr: list[Stroke] = []
        events = _diff(prev, curr)

        assert len(events) == 1
        e = events[0]
        assert isinstance(e, StrokeInvalidated)
        assert e.event_type == "stroke_invalidated"
        assert e.stroke_id == 0

    def test_multiple_strokes_to_fewer(self):
        """多笔→少笔（笔合并）：invalidate 多余的。"""
        prev = [
            _mk(0, 5, "up", 10.0, 20.0, confirmed=True),
            _mk(5, 10, "down", 20.0, 12.0, confirmed=True),
            _mk(10, 15, "up", 12.0, 22.0, confirmed=False),
        ]
        # 新快照只有一笔
        curr = [_mk(0, 15, "up", 10.0, 25.0, confirmed=False)]
        events = _diff(prev, curr)

        # 公共前缀为 0（prev[0] != curr[0]）
        # invalidate prev[0], prev[1], prev[2]
        # candidate curr[0]
        inv_events = [e for e in events if isinstance(e, StrokeInvalidated)]
        cand_events = [e for e in events if isinstance(e, StrokeCandidate)]
        assert len(inv_events) == 3
        assert len(cand_events) == 1

    def test_common_prefix_preserved(self):
        """已确认的公共前缀不产生事件。"""
        common = _mk(0, 5, "up", 10.0, 20.0, confirmed=True)
        prev = [common, _mk(5, 10, "down", 20.0, 12.0, confirmed=False)]
        curr = [common, _mk(5, 12, "down", 20.0, 8.0, confirmed=False)]
        events = _diff(prev, curr)

        # common 完全相同 → 公共前缀长度 = 1
        # prev[1] vs curr[1]：high/low 不同 → 不在公共前缀
        # 所以先 invalidate prev[1]，再因 same_origin → extended
        assert len(events) == 2
        assert isinstance(events[0], StrokeInvalidated)
        assert events[0].stroke_id == 1
        assert isinstance(events[1], StrokeExtended)
        assert events[1].stroke_id == 1

    def test_common_prefix_no_events(self):
        """公共前缀中完全相同的笔不产生任何事件。"""
        s0 = _mk(0, 5, "up", 10.0, 20.0, confirmed=True)
        s1 = _mk(5, 10, "down", 20.0, 12.0, confirmed=True)
        # prev 和 curr 前两笔完全相同，只是末笔不同
        prev = [s0, s1, _mk(10, 15, "up", 12.0, 22.0, confirmed=False)]
        curr = [s0, s1, _mk(10, 17, "up", 12.0, 25.0, confirmed=False)]
        events = _diff(prev, curr)

        # 公共前缀 = 2（s0 和 s1 完全相同）
        # 第三笔 high/low 不同 → invalidate + extended
        assert all(e.stroke_id >= 2 for e in events), (
            "Common prefix strokes (id 0, 1) should not produce events"
        )

    def test_seq_monotonic(self):
        """事件序号单调递增。"""
        prev = [
            _mk(0, 5, "up", 10.0, 20.0, confirmed=True),
            _mk(5, 10, "down", 20.0, 12.0, confirmed=False),
        ]
        curr = [
            _mk(0, 5, "up", 10.0, 20.0, confirmed=True),
            _mk(5, 10, "down", 20.0, 12.0, confirmed=True),
            _mk(10, 15, "up", 12.0, 25.0, confirmed=False),
        ]
        events = _diff(prev, curr, seq_start=100)

        assert len(events) >= 1
        seqs = [e.seq for e in events]
        assert seqs[0] == 100
        for i in range(1, len(seqs)):
            assert seqs[i] == seqs[i - 1] + 1, (
                f"seq not monotonic: {seqs}"
            )

    def test_seq_start_respected(self):
        """seq_start 参数被正确使用。"""
        cand = _mk(0, 5, "up", 10.0, 20.0, confirmed=False)
        events = _diff([], [cand], seq_start=42)
        assert events[0].seq == 42
