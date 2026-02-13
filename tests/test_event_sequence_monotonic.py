"""事件序列单调性测试 — I17 验证（PR-C0.5）

覆盖 4 个场景：
  1. 中枢：invalidate 后无同身份事件
  2. 线段：invalidate 后无同身份事件
  3. checker 检测 I17 违规（手动构造）
  4. 全管线运行 → checker 无 I17 违规
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import Zhongshu, zhongshu_from_segments
from newchan.audit.segment_checker import SegmentInvariantChecker
from newchan.audit.zhongshu_checker import ZhongshuInvariantChecker
from newchan.core.recursion.segment_state import diff_segments
from newchan.core.recursion.zhongshu_state import diff_zhongshu
from newchan.events import (
    SegmentBreakPendingV1,
    SegmentInvalidateV1,
    ZhongshuCandidateV1,
    ZhongshuInvalidateV1,
)
from newchan.fingerprint import compute_event_id


# ── helpers ──

def _seg(idx: int, direction: str, high: float, low: float,
         confirmed: bool = True) -> Segment:
    s0 = idx * 3
    s1 = idx * 3 + 2
    return Segment(
        s0=s0, s1=s1,
        i0=s0 * 5, i1=s1 * 5,
        direction=direction,
        high=high, low=low,
        confirmed=confirmed,
    )


# =====================================================================
# 1) 中枢：invalidate 后无同身份事件
# =====================================================================

class TestZhongshuNoEventAfterInvalidate:
    def test_invalidate_then_no_candidate(self):
        """中枢被 invalidate 后，不应在后续 diff 中重新出现同身份 candidate。

        场景：3段→2段（中枢消失）→3段（新的不同中枢）
        """
        segs3 = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
        ]
        segs2 = segs3[:2]
        segs3_new = [
            _seg(0, "up",   50, 30),
            _seg(1, "down", 48, 32),
            _seg(2, "up",   52, 35),
        ]

        zs3 = zhongshu_from_segments(segs3)
        zs2 = zhongshu_from_segments(segs2)
        zs3_new = zhongshu_from_segments(segs3_new)

        # Step 1: 中枢出现
        ev1 = diff_zhongshu([], zs3, bar_idx=10, bar_ts=100.0)
        # Step 2: 中枢消失 → invalidate
        ev2 = diff_zhongshu(zs3, zs2, bar_idx=11, bar_ts=101.0)
        assert any(isinstance(e, ZhongshuInvalidateV1) for e in ev2)
        # Step 3: 新中枢（不同身份）
        ev3 = diff_zhongshu(zs2, zs3_new, bar_idx=12, bar_ts=102.0)

        # 验证所有事件通过 checker
        checker = ZhongshuInvariantChecker()
        for evs, idx, ts in [(ev1, 10, 100.0), (ev2, 11, 101.0), (ev3, 12, 102.0)]:
            violations = checker.check(evs, idx, ts)
            assert len(violations) == 0, f"I17 violation at bar {idx}: {violations}"


# =====================================================================
# 2) 线段：invalidate 后无同身份事件
# =====================================================================

class TestSegmentNoEventAfterInvalidate:
    def test_segment_invalidate_terminal(self):
        """段被 invalidate 后，diff 不应为同身份段再产生事件。

        段身份 = (s0, direction)。
        """
        seg_a = Segment(
            s0=0, s1=4, i0=0, i1=20,
            direction="up", high=20, low=5,
            confirmed=False,
        )
        seg_b = Segment(
            s0=3, s1=6, i0=15, i1=30,
            direction="down", high=25, low=10,
            confirmed=False,
        )

        # Step 1: seg_a 存在
        ev1 = diff_segments([], [seg_a], bar_idx=0, bar_ts=100.0)
        # Step 2: seg_a → seg_b（不同身份） → invalidate seg_a
        ev2 = diff_segments([seg_a], [seg_b], bar_idx=1, bar_ts=101.0)
        invalidates = [e for e in ev2 if isinstance(e, SegmentInvalidateV1)]
        assert len(invalidates) == 1
        assert invalidates[0].s0 == 0

        # 验证通过 checker
        checker = SegmentInvariantChecker()
        for evs, idx, ts in [(ev1, 0, 100.0), (ev2, 1, 101.0)]:
            violations = checker.check(evs, idx, ts)
            assert len(violations) == 0


# =====================================================================
# 3) checker 检测 I17 违规（手动构造事件）
# =====================================================================

class TestCheckerDetectsI17Violation:
    def test_zhongshu_i17_violation(self):
        """手动构造 invalidate 后跟同身份 candidate → checker 报 I17。"""
        checker = ZhongshuInvariantChecker()

        # 构造 invalidate 事件
        inv_ev = ZhongshuInvalidateV1(
            bar_idx=10, bar_ts=100.0, seq=0,
            event_id=compute_event_id(10, 100.0, "zhongshu_invalidate", 0,
                                       {"zhongshu_id": 0, "zd": 10.0, "zg": 18.0,
                                        "seg_start": 0, "seg_end": 2}),
            zhongshu_id=0, zd=10.0, zg=18.0, seg_start=0, seg_end=2,
        )
        v1 = checker.check([inv_ev], 10, 100.0)
        assert len(v1) == 0  # invalidate 本身无违规

        # 构造同身份 candidate → 应触发 I17
        cand_ev = ZhongshuCandidateV1(
            bar_idx=11, bar_ts=101.0, seq=1,
            event_id=compute_event_id(11, 101.0, "zhongshu_candidate", 1,
                                       {"zhongshu_id": 0, "zd": 10.0, "zg": 18.0,
                                        "seg_start": 0, "seg_end": 3, "seg_count": 4}),
            zhongshu_id=0, zd=10.0, zg=18.0, seg_start=0, seg_end=3, seg_count=4,
        )
        v2 = checker.check([cand_ev], 11, 101.0)
        assert len(v2) == 1
        assert "I17" in v2[0].code

    def test_segment_i17_violation(self):
        """手动构造 segment invalidate 后跟同身份 pending → checker 报 I17。"""
        checker = SegmentInvariantChecker()

        # invalidate
        inv_ev = SegmentInvalidateV1(
            bar_idx=10, bar_ts=100.0, seq=0,
            event_id=compute_event_id(10, 100.0, "segment_invalidate", 0,
                                       {"segment_id": 0, "direction": "up",
                                        "s0": 0, "s1": 4}),
            segment_id=0, direction="up", s0=0, s1=4,
        )
        v1 = checker.check([inv_ev], 10, 100.0)
        assert len(v1) == 0

        # 同身份 pending → I17
        pend_ev = SegmentBreakPendingV1(
            bar_idx=11, bar_ts=101.0, seq=1,
            event_id=compute_event_id(11, 101.0, "segment_break_pending", 1,
                                       {"segment_id": 0, "direction": "up",
                                        "break_at_stroke": 3, "gap_class": "none",
                                        "fractal_type": "top", "s0": 0, "s1": 6}),
            segment_id=0, direction="up", break_at_stroke=3,
            gap_class="none", fractal_type="top", s0=0, s1=6,
        )
        v2 = checker.check([pend_ev], 11, 101.0)
        assert len(v2) == 1
        assert "I17" in v2[0].code


# =====================================================================
# 4) 全管线运行 → checker 无 I17 违规
# =====================================================================

class TestIntegratedPipelineMonotonic:
    def test_zhongshu_multi_phase_no_i17(self):
        """多阶段中枢生命周期通过 checker 无 I17 违规。"""
        checker = ZhongshuInvariantChecker()

        # Phase 1: 3段 → 中枢出现
        segs1 = [_seg(0, "up", 20, 10), _seg(1, "down", 18, 8), _seg(2, "up", 22, 12)]
        zs1 = zhongshu_from_segments(segs1)
        ev1 = diff_zhongshu([], zs1, bar_idx=10, bar_ts=100.0)
        assert len(checker.check(ev1, 10, 100.0)) == 0

        # Phase 2: 4段 → 延伸（同身份更新）
        segs2 = segs1 + [_seg(3, "down", 19, 11)]
        zs2 = zhongshu_from_segments(segs2)
        ev2 = diff_zhongshu(zs1, zs2, bar_idx=11, bar_ts=101.0)
        assert len(checker.check(ev2, 11, 101.0)) == 0

        # Phase 3: 段否定 → 中枢消失
        zs3 = zhongshu_from_segments(segs1[:2])
        ev3 = diff_zhongshu(zs2, zs3, bar_idx=12, bar_ts=102.0)
        assert len(checker.check(ev3, 12, 102.0)) == 0

        # Phase 4: 全新中枢（不同身份） → 无 I17 问题
        segs4 = [_seg(5, "up", 50, 30), _seg(6, "down", 48, 32), _seg(7, "up", 52, 35)]
        zs4 = zhongshu_from_segments(segs4)
        ev4 = diff_zhongshu(zs3, zs4, bar_idx=13, bar_ts=103.0)
        assert len(checker.check(ev4, 13, 103.0)) == 0
