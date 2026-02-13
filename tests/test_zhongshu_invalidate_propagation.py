"""中枢 v1 — 段否定 → 中枢否定传播

覆盖：
  1. 构成段被否定导致中枢消失 → ZhongshuInvalidateV1
  2. 同一中枢不重复 invalidate（I14）
  3. 中枢最后一段被否定 → 中枢缩小或消失
  4. 否定后新段重新形成 → 新 ZhongshuCandidateV1
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import Zhongshu, zhongshu_from_segments
from newchan.core.recursion.zhongshu_state import diff_zhongshu
from newchan.core.recursion.zhongshu_engine import ZhongshuEngine
from newchan.core.recursion.segment_state import SegmentSnapshot
from newchan.events import (
    ZhongshuCandidateV1,
    ZhongshuInvalidateV1,
)


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
# 1) 段否定导致中枢消失
# =====================================================================

class TestSegInvalidateRemovesZhongshu:
    def test_zhongshu_invalidated_when_seg_removed(self):
        """一个已确认段被否定（从列表中消失）→ 中枢消失 → InvalidateV1。"""
        # Phase 1: 三段形成中枢
        segs1 = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
        ]
        zs1 = zhongshu_from_segments(segs1)
        ev1 = diff_zhongshu([], zs1, bar_idx=10, bar_ts=100.0)
        assert len(ev1) == 1
        assert isinstance(ev1[0], ZhongshuCandidateV1)

        # Phase 2: 第三段被否定 → 只剩两段 → 中枢消失
        segs2 = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            # seg[2] 被否定，不再出现
        ]
        zs2 = zhongshu_from_segments(segs2)
        ev2 = diff_zhongshu(zs1, zs2, bar_idx=11, bar_ts=101.0, seq_start=1)
        assert len(ev2) == 1
        assert isinstance(ev2[0], ZhongshuInvalidateV1)
        assert ev2[0].zhongshu_id == 0

    def test_engine_propagates_invalidation(self):
        """通过 ZhongshuEngine 验证否定传播。"""
        engine = ZhongshuEngine()

        # Phase 1: 三段
        segs1 = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
        ]
        snap1 = SegmentSnapshot(bar_idx=10, bar_ts=100.0, segments=segs1, events=[])
        zs_snap1 = engine.process_segment_snapshot(snap1)
        assert len(zs_snap1.zhongshus) == 1

        # Phase 2: 段被否定
        segs2 = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
        ]
        snap2 = SegmentSnapshot(bar_idx=11, bar_ts=101.0, segments=segs2, events=[])
        zs_snap2 = engine.process_segment_snapshot(snap2)
        assert len(zs_snap2.zhongshus) == 0
        invalidates = [e for e in zs_snap2.events if isinstance(e, ZhongshuInvalidateV1)]
        assert len(invalidates) == 1


# =====================================================================
# 2) 同一中枢不重复 invalidate（I14 由 checker 保证，此处验证 diff 不重复产生）
# =====================================================================

class TestInvalidateIdempotent:
    def test_no_double_invalidate(self):
        """中枢已消失后再次 diff 不产生额外 invalidate。"""
        segs_full = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
        ]
        zs_full = zhongshu_from_segments(segs_full)

        segs_partial = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
        ]
        zs_partial = zhongshu_from_segments(segs_partial)

        # 第一次 diff: 中枢消失 → invalidate
        ev1 = diff_zhongshu(zs_full, zs_partial, bar_idx=11, bar_ts=101.0)
        assert len(ev1) == 1

        # 第二次 diff: 仍然无中枢 → 无事件（因为 prev 也是空）
        ev2 = diff_zhongshu(zs_partial, zs_partial, bar_idx=12, bar_ts=102.0)
        assert len(ev2) == 0


# =====================================================================
# 3) 中枢最后一段被否定 → 中枢变化
# =====================================================================

class TestPartialInvalidate:
    def test_extension_retracted(self):
        """4段中枢的最后一段被否定 → 缩回到3段。

        同身份中枢（zd/zg/seg_start 相同）的 seg_end 变化不产生 invalidate，
        只产生更新的 CandidateV1。
        """
        segs4 = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
            _seg(3, "down", 19, 11),   # 延伸段
        ]
        zs4 = zhongshu_from_segments(segs4)
        assert len(zs4) == 1
        assert zs4[0].seg_count == 4

        # 最后一段被否定
        segs3 = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
        ]
        zs3 = zhongshu_from_segments(segs3)

        events = diff_zhongshu(zs4, zs3, bar_idx=12, bar_ts=102.0)
        # 同身份中枢 seg_end 变化：不 invalidate，只发更新 candidate
        invalidates = [e for e in events if isinstance(e, ZhongshuInvalidateV1)]
        candidates = [e for e in events if isinstance(e, ZhongshuCandidateV1)]
        assert len(invalidates) == 0
        assert len(candidates) == 1
        assert candidates[0].seg_count == 3


# =====================================================================
# 4) 否定后新段重新形成中枢
# =====================================================================

class TestInvalidateThenReform:
    def test_reform_after_invalidate(self):
        """中枢被否定后，新段序列重新形成中枢。"""
        engine = ZhongshuEngine()

        # Phase 1: 三段中枢
        segs1 = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
        ]
        snap1 = SegmentSnapshot(bar_idx=10, bar_ts=100.0, segments=segs1, events=[])
        zs1 = engine.process_segment_snapshot(snap1)
        assert len(zs1.zhongshus) == 1

        # Phase 2: 全部否定
        segs2: list[Segment] = []
        snap2 = SegmentSnapshot(bar_idx=11, bar_ts=101.0, segments=segs2, events=[])
        zs2 = engine.process_segment_snapshot(snap2)
        assert len(zs2.zhongshus) == 0
        assert any(isinstance(e, ZhongshuInvalidateV1) for e in zs2.events)

        # Phase 3: 新段形成新中枢
        segs3 = [
            _seg(0, "up",   50, 30),
            _seg(1, "down", 48, 32),
            _seg(2, "up",   52, 35),
        ]
        snap3 = SegmentSnapshot(bar_idx=12, bar_ts=102.0, segments=segs3, events=[])
        zs3 = engine.process_segment_snapshot(snap3)
        assert len(zs3.zhongshus) == 1
        candidates = [e for e in zs3.events if isinstance(e, ZhongshuCandidateV1)]
        assert len(candidates) == 1
        # 新中枢 zd/zg 不同
        assert candidates[0].zd == 35
        assert candidates[0].zg == 48
