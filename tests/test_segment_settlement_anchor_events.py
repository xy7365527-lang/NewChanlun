"""线段结算锚事件测试 — 事件驱动 I6/I7 验证

覆盖 4 个场景：
  1. settle 事件 s1 == k-1（旧段终点 = 分型中心前一笔）
  2. settle 事件 new_segment_s0 == k（新段起点 = 分型中心）
  3. pending 事件 seq < settle 事件 seq（I6 两阶段顺序）
  4. 新段无重叠 → 只有 pending，无 settle
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v1 import segments_from_strokes_v1
from newchan.a_stroke import Stroke
from newchan.bi_engine import BiEngineSnapshot
from newchan.core.recursion.segment_engine import SegmentEngine
from newchan.core.recursion.segment_state import diff_segments
from newchan.events import (
    SegmentBreakPendingV1,
    SegmentInvalidateV1,
    SegmentSettleV1,
)


# ── helpers ──

def _s(i0, i1, direction, high, low, confirmed=True):
    if direction == "up":
        p0, p1 = low, high
    else:
        p0, p1 = high, low
    return Stroke(i0=i0, i1=i1, direction=direction,
                  high=high, low=low, p0=p0, p1=p1, confirmed=confirmed)


def _snap(strokes, bar_idx=0, bar_ts=1000.0):
    return BiEngineSnapshot(
        bar_idx=bar_idx,
        bar_ts=bar_ts,
        strokes=strokes,
        events=[],
        n_merged=0,
        n_fractals=0,
    )


# 共用的可结算数据集（确保三笔重叠 anchor 成立）
def _settleable_strokes():
    """向上段 + 顶分型触发 + 新段三笔重叠 → 可结算。"""
    return [
        _s(0, 5, "up",    15, 5),
        _s(5, 10, "down", 12, 8),    # feat a
        _s(10, 15, "up",  18, 7),
        _s(15, 20, "down", 16, 9),   # feat b (顶分型中心), k=3
        _s(20, 25, "up",  14, 8),    # feat c
        _s(25, 30, "down", 13, 7),
        _s(30, 35, "up",  12, 4),
    ]


# =====================================================================
# 1) settle 事件 s1 == k-1
# =====================================================================

class TestSettleEventOldEndKMinus1:
    """SegmentSettleV1.s1 应等于 break_at_stroke - 1。"""

    def test_s1_equals_k_minus_1(self):
        segs = segments_from_strokes_v1(_settleable_strokes())
        events = diff_segments([], segs, bar_idx=0, bar_ts=1000.0)
        settles = [e for e in events if isinstance(e, SegmentSettleV1)]
        assert len(settles) >= 1
        for s in settles:
            # 从 pending 中找到对应 segment_id 的 break_at_stroke
            pendings = [
                p for p in events
                if isinstance(p, SegmentBreakPendingV1) and p.segment_id == s.segment_id
            ]
            assert len(pendings) == 1
            assert s.s1 == pendings[0].break_at_stroke - 1, (
                f"s1={s.s1} should == break_at_stroke-1={pendings[0].break_at_stroke - 1}"
            )

    def test_via_engine(self):
        engine = SegmentEngine()
        snap = engine.process_snapshot(_snap(_settleable_strokes()))
        settles = [e for e in snap.events if isinstance(e, SegmentSettleV1)]
        pendings = [e for e in snap.events if isinstance(e, SegmentBreakPendingV1)]
        assert len(settles) >= 1
        for s in settles:
            p = [x for x in pendings if x.segment_id == s.segment_id]
            assert len(p) == 1
            assert s.s1 == p[0].break_at_stroke - 1


# =====================================================================
# 2) settle 事件 new_segment_s0 == k
# =====================================================================

class TestSettleEventNewStartK:
    """SegmentSettleV1.new_segment_s0 应等于 break_at_stroke。"""

    def test_new_s0_equals_k(self):
        segs = segments_from_strokes_v1(_settleable_strokes())
        events = diff_segments([], segs, bar_idx=0, bar_ts=1000.0)
        settles = [e for e in events if isinstance(e, SegmentSettleV1)]
        assert len(settles) >= 1
        for s in settles:
            pendings = [
                p for p in events
                if isinstance(p, SegmentBreakPendingV1) and p.segment_id == s.segment_id
            ]
            assert len(pendings) == 1
            assert s.new_segment_s0 == pendings[0].break_at_stroke

    def test_new_direction_flipped(self):
        """新段方向应与旧段相反。"""
        segs = segments_from_strokes_v1(_settleable_strokes())
        events = diff_segments([], segs, bar_idx=0, bar_ts=1000.0)
        settles = [e for e in events if isinstance(e, SegmentSettleV1)]
        assert len(settles) >= 1
        for s in settles:
            expected = "down" if s.direction == "up" else "up"
            assert s.new_segment_direction == expected


# =====================================================================
# 3) pending seq < settle seq （I6）
# =====================================================================

class TestPendingBeforeSettle:
    """同 segment_id 的 BreakPending.seq 必须 < Settle.seq。"""

    def test_pending_seq_less_than_settle_seq(self):
        segs = segments_from_strokes_v1(_settleable_strokes())
        events = diff_segments([], segs, bar_idx=0, bar_ts=1000.0)

        settles = [e for e in events if isinstance(e, SegmentSettleV1)]
        pendings = [e for e in events if isinstance(e, SegmentBreakPendingV1)]
        assert len(settles) >= 1 and len(pendings) >= 1

        for s in settles:
            matching = [p for p in pendings if p.segment_id == s.segment_id]
            assert len(matching) == 1, "每个 settle 必须对应恰好一个 pending"
            assert matching[0].seq < s.seq, (
                f"pending.seq={matching[0].seq} should < settle.seq={s.seq}"
            )

    def test_no_settle_without_pending(self):
        """settle 不能没有对应的 pending（I6 保证）。"""
        engine = SegmentEngine()
        snap = engine.process_snapshot(_snap(_settleable_strokes()))
        settles = [e for e in snap.events if isinstance(e, SegmentSettleV1)]
        pendings = [e for e in snap.events if isinstance(e, SegmentBreakPendingV1)]
        settle_ids = {s.segment_id for s in settles}
        pending_ids = {p.segment_id for p in pendings}
        assert settle_ids.issubset(pending_ids), (
            f"settle ids {settle_ids} should be subset of pending ids {pending_ids}"
        )


# =====================================================================
# 4) 新段无重叠 → 无 settle
# =====================================================================

class TestNoSettleWithoutAnchor:
    """新段三笔无重叠时，旧段不应结算。"""

    def _strokes(self):
        return [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 11),   # 分型中心
            _s(20, 25, "up",  30, 20),    # 跳空大涨（三笔无重叠）
            _s(25, 30, "down", 40, 32),
            _s(30, 35, "up",  50, 38),
        ]

    def test_no_settle_event(self):
        engine = SegmentEngine()
        snap = engine.process_snapshot(_snap(self._strokes()))
        settles = [e for e in snap.events if isinstance(e, SegmentSettleV1)]
        assert len(settles) == 0

    def test_single_segment(self):
        segs = segments_from_strokes_v1(self._strokes())
        assert len(segs) == 1, f"应只有一段，实际有 {len(segs)} 段"
        assert segs[0].confirmed is False


# =====================================================================
# Invalidate 幂等性 (I9)
# =====================================================================

class TestInvalidateIdempotent:
    """逐步喂入 stroke 快照，invalidate 不应重复发出。"""

    def test_no_duplicate_invalidate(self):
        """同一 (s0, s1, direction) 不应被 invalidate 两次。"""
        engine = SegmentEngine()

        # 阶段 1：足够的笔产生一段
        strokes_v1 = [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 9),
            _s(20, 25, "up",  14, 8),
        ]
        snap1 = engine.process_snapshot(_snap(strokes_v1, bar_idx=0))

        # 阶段 2：笔被否定，段消失
        strokes_v2 = [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
        ]
        snap2 = engine.process_snapshot(_snap(strokes_v2, bar_idx=1))

        # 阶段 3：再次变化（不应再产生相同 invalidate）
        strokes_v3 = [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
        ]
        snap3 = engine.process_snapshot(_snap(strokes_v3, bar_idx=2))

        # 统计 invalidate 事件
        all_inv = []
        for snap in [snap1, snap2, snap3]:
            all_inv.extend([e for e in snap.events if isinstance(e, SegmentInvalidateV1)])

        # 同 (s0, s1, direction) 的 invalidate 不应重复
        seen = set()
        for inv in all_inv:
            key = (inv.s0, inv.s1, inv.direction)
            assert key not in seen, f"重复 invalidate: {key}"
            seen.add(key)


# =====================================================================
# 事件确定性（I10 部分覆盖）
# =====================================================================

class TestSegmentReplayDeterminism:
    """同输入两次独立回放 → 事件序列完全一致。"""

    def test_two_runs_same_events(self):
        strokes = _settleable_strokes()

        engine1 = SegmentEngine()
        snap1 = engine1.process_snapshot(_snap(strokes))

        engine2 = SegmentEngine()
        snap2 = engine2.process_snapshot(_snap(strokes))

        assert len(snap1.events) == len(snap2.events), (
            f"事件数不一致: {len(snap1.events)} vs {len(snap2.events)}"
        )
        for e1, e2 in zip(snap1.events, snap2.events):
            assert e1.event_id == e2.event_id
            assert e1.event_type == e2.event_type
            assert e1.seq == e2.seq
