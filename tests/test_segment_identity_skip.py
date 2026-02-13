"""线段 diff 身份跳过测试 — PR-C0.5

覆盖 8 个场景：
  1. 末段 s1 延伸 → 0 个 InvalidateV1
  2. 末段新增 break_evidence → BreakPending 但无 Invalidate
  3. break_evidence.trigger_stroke_k 变化 → 新 BreakPending（无 Invalidate）
  4. unconfirmed → confirmed+settled 升级 → BreakPending + Settle（无 Invalidate）
  5. 不同身份段替换 → 正常 Invalidate
  6. 段消失 → 正常 Invalidate
  7. prev=[] 首次 diff → 与现有行为一致
  8. 确定性：同输入两次 → 相同事件流
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v0 import BreakEvidence, Segment
from newchan.core.recursion.segment_state import diff_segments
from newchan.events import (
    SegmentBreakPendingV1,
    SegmentInvalidateV1,
    SegmentSettleV1,
)


# ── helpers ──

def _seg(
    s0: int,
    s1: int,
    direction: str,
    *,
    confirmed: bool = True,
    kind: str = "settled",
    break_evidence: BreakEvidence | None = None,
    ep0_price: float = 0.0,
    ep1_price: float = 0.0,
) -> Segment:
    high = 20.0 if direction == "up" else 15.0
    low = 5.0 if direction == "up" else 10.0
    return Segment(
        s0=s0,
        s1=s1,
        i0=s0 * 5,
        i1=s1 * 5,
        direction=direction,
        high=high,
        low=low,
        confirmed=confirmed,
        kind=kind,
        ep0_price=ep0_price,
        ep1_price=ep1_price,
        break_evidence=break_evidence,
    )


def _be(trigger_k: int, gap: str = "none") -> BreakEvidence:
    return BreakEvidence(
        trigger_stroke_k=trigger_k,
        fractal_abc=(trigger_k - 1, trigger_k, trigger_k + 1),
        gap_type=gap,
    )


# =====================================================================
# 1) 末段 s1 延伸（纯延伸，无 break_evidence）→ 无 Invalidate
# =====================================================================

class TestExtendNoInvalidate:
    def test_s1_extend_no_invalidate(self):
        """段 s1 从 4 延伸到 6，同身份 → 0 个 InvalidateV1，0 个事件。"""
        prev = [_seg(0, 4, "up", confirmed=False, kind="candidate")]
        curr = [_seg(0, 6, "up", confirmed=False, kind="candidate")]

        events = diff_segments(prev, curr, bar_idx=1, bar_ts=101.0)
        invalidates = [e for e in events if isinstance(e, SegmentInvalidateV1)]
        assert len(invalidates) == 0
        # 纯延伸无 break_evidence → 不产生任何事件
        assert len(events) == 0


# =====================================================================
# 2) 末段新增 break_evidence → BreakPending 但无 Invalidate
# =====================================================================

class TestNewBreakEvidenceNoInvalidate:
    def test_new_break_evidence_emits_pending(self):
        """段 s1 延伸且新增 be → BreakPending，但无 Invalidate。

        注意：_segments_equal 不检查 break_evidence，因此 s1 也需变化
        才能触发 diff 后缀逻辑。这与实际场景一致：新笔产生时 s1 总会延伸。
        """
        prev = [_seg(0, 4, "up", confirmed=False, kind="candidate")]
        curr = [_seg(0, 6, "up", confirmed=False, kind="candidate", break_evidence=_be(3))]

        events = diff_segments(prev, curr, bar_idx=1, bar_ts=101.0)
        invalidates = [e for e in events if isinstance(e, SegmentInvalidateV1)]
        pendings = [e for e in events if isinstance(e, SegmentBreakPendingV1)]
        assert len(invalidates) == 0
        assert len(pendings) == 1
        assert pendings[0].break_at_stroke == 3


# =====================================================================
# 3) break_evidence 变化 → 新 BreakPending（无 Invalidate）
# =====================================================================

class TestBreakEvidenceUpdateEmitsPending:
    def test_trigger_stroke_change(self):
        """be.trigger_stroke_k 从 3 变为 5 → 新 BreakPending。"""
        prev = [_seg(0, 4, "up", confirmed=False, kind="candidate", break_evidence=_be(3))]
        curr = [_seg(0, 6, "up", confirmed=False, kind="candidate", break_evidence=_be(5))]

        events = diff_segments(prev, curr, bar_idx=2, bar_ts=102.0)
        invalidates = [e for e in events if isinstance(e, SegmentInvalidateV1)]
        pendings = [e for e in events if isinstance(e, SegmentBreakPendingV1)]
        assert len(invalidates) == 0
        assert len(pendings) == 1
        assert pendings[0].break_at_stroke == 5

    def test_s1_extend_with_same_be_emits_pending(self):
        """s1 延伸但 be 的 trigger_stroke_k 不变 → 仍 emit BreakPending（因 s1 变了）。"""
        prev = [_seg(0, 4, "up", confirmed=False, kind="candidate", break_evidence=_be(3))]
        curr = [_seg(0, 6, "up", confirmed=False, kind="candidate", break_evidence=_be(3))]

        events = diff_segments(prev, curr, bar_idx=2, bar_ts=102.0)
        invalidates = [e for e in events if isinstance(e, SegmentInvalidateV1)]
        pendings = [e for e in events if isinstance(e, SegmentBreakPendingV1)]
        assert len(invalidates) == 0
        assert len(pendings) == 1


# =====================================================================
# 4) unconfirmed → confirmed+settled 升级 → BreakPending + Settle
# =====================================================================

class TestUpgradeToSettled:
    def test_upgrade_emits_pending_settle(self):
        """段从 unconfirmed → confirmed+settled → BreakPending + Settle，无 Invalidate。"""
        prev = [_seg(0, 4, "up", confirmed=False, kind="candidate", break_evidence=_be(3))]
        curr = [_seg(0, 4, "up", confirmed=True, kind="settled",
                      break_evidence=_be(3), ep0_price=5.0, ep1_price=20.0)]

        events = diff_segments(prev, curr, bar_idx=2, bar_ts=102.0)
        invalidates = [e for e in events if isinstance(e, SegmentInvalidateV1)]
        pendings = [e for e in events if isinstance(e, SegmentBreakPendingV1)]
        settles = [e for e in events if isinstance(e, SegmentSettleV1)]
        assert len(invalidates) == 0
        assert len(pendings) == 1
        assert len(settles) == 1
        assert pendings[0].seq < settles[0].seq


# =====================================================================
# 5) 不同身份段替换 → 正常 Invalidate
# =====================================================================

class TestDifferentIdentityInvalidates:
    def test_different_s0_invalidates(self):
        """s0 不同 → 身份不同 → 正常 Invalidate。"""
        prev = [_seg(0, 4, "up", confirmed=False, kind="candidate")]
        curr = [_seg(3, 6, "down", confirmed=False, kind="candidate")]

        events = diff_segments(prev, curr, bar_idx=1, bar_ts=101.0)
        invalidates = [e for e in events if isinstance(e, SegmentInvalidateV1)]
        assert len(invalidates) == 1
        assert invalidates[0].s0 == 0


# =====================================================================
# 6) 段消失 → 正常 Invalidate
# =====================================================================

class TestEntityDisappearedInvalidates:
    def test_segment_disappeared(self):
        """段从 prev 消失（curr 更短）→ Invalidate。"""
        prev = [
            _seg(0, 2, "up"),
            _seg(3, 5, "down", confirmed=False, kind="candidate"),
        ]
        curr = [_seg(0, 2, "up")]

        events = diff_segments(prev, curr, bar_idx=1, bar_ts=101.0)
        invalidates = [e for e in events if isinstance(e, SegmentInvalidateV1)]
        assert len(invalidates) == 1
        assert invalidates[0].s0 == 3


# =====================================================================
# 7) prev=[] 首次 diff → 现有行为不变
# =====================================================================

class TestFirstDiffFromEmpty:
    def test_from_empty(self):
        """prev=[] → 全新段，behavior 与之前一致。"""
        curr = [_seg(0, 4, "up", confirmed=True, kind="settled",
                      break_evidence=_be(3), ep0_price=5.0, ep1_price=20.0)]

        events = diff_segments([], curr, bar_idx=0, bar_ts=100.0)
        pendings = [e for e in events if isinstance(e, SegmentBreakPendingV1)]
        settles = [e for e in events if isinstance(e, SegmentSettleV1)]
        assert len(pendings) == 1
        assert len(settles) == 1


# =====================================================================
# 8) 确定性
# =====================================================================

class TestDeterminismWithIdentitySkip:
    def test_two_runs_same_events(self):
        """同输入两次 diff → 完全相同 event_id。"""
        prev = [_seg(0, 4, "up", confirmed=False, kind="candidate")]
        curr = [_seg(0, 6, "up", confirmed=False, kind="candidate", break_evidence=_be(5))]

        ev1 = diff_segments(prev, curr, bar_idx=1, bar_ts=101.0)
        ev2 = diff_segments(prev, curr, bar_idx=1, bar_ts=101.0)

        assert len(ev1) == len(ev2)
        for a, b in zip(ev1, ev2):
            assert a.event_id == b.event_id
            assert a.event_type == b.event_type
            assert a.seq == b.seq
