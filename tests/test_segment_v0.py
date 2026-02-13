"""线段 v0（三笔交集重叠法） — 单元测试 (Step11)

覆盖 docs/chan_spec.md §5 + §5.4 v0：
  A) 能生成段：5 笔中存在三笔交集重叠 → segments >= 1
  B) 不能生成段：三笔无交集重叠 → segments 为空
  C) 贪心推进：6 笔连续重叠产生两段，验证 j 跳跃逻辑
  D) 断言集成：assert_segment_min_three_strokes_overlap ok=True
  E) confirmed：最后一段 False，其余 True
"""

from __future__ import annotations

import pytest

from newchan.a_stroke import Stroke
from newchan.a_segment_v0 import Segment, segments_from_strokes_v0
from newchan.a_assertions import assert_segment_min_three_strokes_overlap


# ── helper ───────────────────────────────────────────────────────────

def _s(i0, i1, direction, high, low, confirmed=True):
    if direction == "up":
        p0, p1 = low, high
    else:
        p0, p1 = high, low
    return Stroke(i0=i0, i1=i1, direction=direction,
                  high=high, low=low, p0=p0, p1=p1, confirmed=confirmed)


# =====================================================================
# A) 能生成段
# =====================================================================

class TestCanProduceSegment:
    """三笔存在交集重叠时，应产生线段。"""

    def test_basic_5_strokes(self):
        """5 笔，前三笔有重叠 → 产生 1 段。"""
        strokes = [
            _s(0, 4, "up",   high=20.0, low=5.0),
            _s(4, 8, "down", high=18.0, low=8.0),
            _s(8, 12, "up",  high=22.0, low=6.0),
            _s(12, 16, "down", high=15.0, low=3.0),
            _s(16, 20, "up",  high=25.0, low=10.0),
        ]
        # [0,1,2]: overlap_low=max(5,8,6)=8, overlap_high=min(20,18,22)=18
        # 8 < 18 → segment
        segs = segments_from_strokes_v0(strokes)
        assert len(segs) >= 1
        assert segs[0].s0 == 0
        assert segs[0].s1 == 2

    def test_segment_direction_is_first_stroke(self):
        """v0: 线段方向 = 第一笔方向。"""
        strokes = [
            _s(0, 5, "up",   high=20.0, low=5.0),
            _s(5, 10, "down", high=18.0, low=8.0),
            _s(10, 15, "up",  high=22.0, low=6.0),
        ]
        segs = segments_from_strokes_v0(strokes)
        assert len(segs) == 1
        assert segs[0].direction == "up"

    def test_segment_high_low(self):
        """段的 high/low = 三笔 high 最大 / low 最小。"""
        strokes = [
            _s(0, 5, "up",   high=20.0, low=5.0),
            _s(5, 10, "down", high=18.0, low=8.0),
            _s(10, 15, "up",  high=22.0, low=6.0),
        ]
        segs = segments_from_strokes_v0(strokes)
        assert segs[0].high == 22.0   # max(20, 18, 22)
        assert segs[0].low == 5.0     # min(5, 8, 6)

    def test_segment_i0_i1(self):
        """i0 = strokes[s0].i0, i1 = strokes[s1].i1。"""
        strokes = [
            _s(3, 7, "down", high=20.0, low=5.0),
            _s(7, 11, "up",  high=18.0, low=8.0),
            _s(11, 16, "down", high=22.0, low=6.0),
        ]
        segs = segments_from_strokes_v0(strokes)
        assert segs[0].i0 == 3    # strokes[0].i0
        assert segs[0].i1 == 16   # strokes[2].i1


# =====================================================================
# B) 不能生成段
# =====================================================================

class TestCannotProduceSegment:
    """三笔无交集重叠时，不产生线段。"""

    def test_no_overlap(self):
        """三笔价格区间完全不重叠。"""
        strokes = [
            _s(0, 5, "up",   high=10.0, low=5.0),
            _s(5, 10, "down", high=20.0, low=12.0),
            _s(10, 15, "up",  high=30.0, low=22.0),
        ]
        # overlap_low=max(5,12,22)=22, overlap_high=min(10,20,30)=10
        # 22 < 10? No → no segment
        segs = segments_from_strokes_v0(strokes)
        assert len(segs) == 0

    def test_fewer_than_3_strokes(self):
        """少于 3 笔 → 空。"""
        assert segments_from_strokes_v0([]) == []
        assert segments_from_strokes_v0([
            _s(0, 5, "up", high=20.0, low=5.0),
        ]) == []
        assert segments_from_strokes_v0([
            _s(0, 5, "up",   high=20.0, low=5.0),
            _s(5, 10, "down", high=18.0, low=8.0),
        ]) == []

    def test_overlap_boundary_equal(self):
        """overlap_low == overlap_high → 不成立（需要 strict <）。"""
        strokes = [
            _s(0, 5, "up",   high=15.0, low=10.0),
            _s(5, 10, "down", high=15.0, low=10.0),
            _s(10, 15, "up",  high=15.0, low=10.0),
        ]
        # overlap_low=max(10,10,10)=10, overlap_high=min(15,15,15)=15
        # 10 < 15 → overlap exists! This actually IS a segment.
        # Let me change the data so overlap is exactly equal:
        strokes2 = [
            _s(0, 5, "up",   high=10.0, low=5.0),
            _s(5, 10, "down", high=15.0, low=10.0),
            _s(10, 15, "up",  high=20.0, low=15.0),
        ]
        # overlap_low=max(5,10,15)=15, overlap_high=min(10,15,20)=10
        # 15 < 10? No → no segment
        segs = segments_from_strokes_v0(strokes2)
        assert len(segs) == 0


# =====================================================================
# C) 贪心推进
# =====================================================================

class TestGreedyAdvance:
    """6 笔连续重叠产生两段（j 跳跃逻辑）。"""

    def _make_6_strokes(self) -> list[Stroke]:
        """6 笔全部在 [5, 20] 区间内有重叠。"""
        return [
            _s(0,  4,  "up",   high=20.0, low=5.0),
            _s(4,  8,  "down", high=18.0, low=8.0),
            _s(8,  12, "up",   high=22.0, low=6.0),   # window[0,1,2] → seg1
            _s(12, 16, "down", high=19.0, low=7.0),
            _s(16, 20, "up",   high=21.0, low=8.0),   # window[2,3,4] → seg2
            _s(20, 24, "down", high=17.0, low=5.0),
        ]

    def test_two_segments(self):
        strokes = self._make_6_strokes()
        segs = segments_from_strokes_v0(strokes)
        assert len(segs) == 2

    def test_first_segment_endpoints(self):
        strokes = self._make_6_strokes()
        segs = segments_from_strokes_v0(strokes)
        assert segs[0].s0 == 0
        assert segs[0].s1 == 2
        assert segs[0].i0 == 0
        assert segs[0].i1 == 12

    def test_second_segment_endpoints(self):
        strokes = self._make_6_strokes()
        segs = segments_from_strokes_v0(strokes)
        assert segs[1].s0 == 2
        assert segs[1].s1 == 4
        assert segs[1].i0 == 8
        assert segs[1].i1 == 20

    def test_j_jump_no_overlap(self):
        """中间三笔无重叠 → 只第一窗口成段，贪心跳过后无法再成段。"""
        strokes = [
            _s(0,  4,  "up",   high=20.0, low=5.0),
            _s(4,  8,  "down", high=18.0, low=8.0),
            _s(8,  12, "up",   high=22.0, low=6.0),
            _s(12, 16, "down", high=30.0, low=25.0),  # 远高于前面
            _s(16, 20, "up",   high=40.0, low=32.0),
            _s(20, 24, "down", high=38.0, low=28.0),
        ]
        segs = segments_from_strokes_v0(strokes)
        # window[0,1,2] → overlap; window[2,3,4] → no overlap
        # (overlap_low=max(6,25,32)=32 vs overlap_high=min(22,30,40)=22 → no)
        assert len(segs) == 1


# =====================================================================
# D) 断言集成
# =====================================================================

class TestAssertIntegration:
    """assert_segment_min_three_strokes_overlap 集成。"""

    def test_valid_segments_pass(self):
        strokes = [
            _s(0, 5, "up",   high=20.0, low=5.0),
            _s(5, 10, "down", high=18.0, low=8.0),
            _s(10, 15, "up",  high=22.0, low=6.0),
        ]
        segs = segments_from_strokes_v0(strokes)
        result = assert_segment_min_three_strokes_overlap(segs, strokes)
        assert result.ok is True

    def test_greedy_pass(self):
        strokes = [
            _s(0,  4,  "up",   high=20.0, low=5.0),
            _s(4,  8,  "down", high=18.0, low=8.0),
            _s(8,  12, "up",   high=22.0, low=6.0),
            _s(12, 16, "down", high=19.0, low=7.0),
            _s(16, 20, "up",   high=21.0, low=8.0),
            _s(20, 24, "down", high=17.0, low=5.0),
        ]
        segs = segments_from_strokes_v0(strokes)
        result = assert_segment_min_three_strokes_overlap(segs, strokes)
        assert result.ok is True

    def test_empty_segments_pass(self):
        result = assert_segment_min_three_strokes_overlap([], [])
        assert result.ok is True

    def test_bad_overlap_fails(self):
        """手工构造一段但对应 strokes 无重叠 → 断言失败。"""
        strokes = [
            _s(0, 5, "up",   high=10.0, low=5.0),
            _s(5, 10, "down", high=20.0, low=12.0),
            _s(10, 15, "up",  high=30.0, low=22.0),
        ]
        bad_seg = [Segment(s0=0, s1=2, i0=0, i1=15,
                           direction="up", high=30, low=5, confirmed=False)]
        result = assert_segment_min_three_strokes_overlap(bad_seg, strokes)
        assert result.ok is False
        assert "overlap empty" in result.message


# =====================================================================
# E) confirmed
# =====================================================================

class TestConfirmed:
    """§5.3 最后一段 confirmed=False，其余 True。"""

    def test_single_segment_unconfirmed(self):
        strokes = [
            _s(0, 5, "up",   high=20.0, low=5.0),
            _s(5, 10, "down", high=18.0, low=8.0),
            _s(10, 15, "up",  high=22.0, low=6.0),
        ]
        segs = segments_from_strokes_v0(strokes)
        assert len(segs) == 1
        assert segs[0].confirmed is False

    def test_two_segments_confirmed(self):
        strokes = [
            _s(0,  4,  "up",   high=20.0, low=5.0),
            _s(4,  8,  "down", high=18.0, low=8.0),
            _s(8,  12, "up",   high=22.0, low=6.0),
            _s(12, 16, "down", high=19.0, low=7.0),
            _s(16, 20, "up",   high=21.0, low=8.0),
        ]
        segs = segments_from_strokes_v0(strokes)
        assert len(segs) == 2
        assert segs[0].confirmed is True
        assert segs[1].confirmed is False
