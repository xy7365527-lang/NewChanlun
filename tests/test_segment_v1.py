"""线段 v1（特征序列法） — 单元测试 (Step12)

覆盖 docs/chan_spec.md §5.4 v1：
  A) 无足够反向笔 → 不触发分型 → 只输出一段
  B) 有 top fractal（up 段）→ 产生分段
  C) 有 bottom fractal（down 段）→ 产生分段
  D) confirmed 语义
  E) 断言集成 enable=False
  F) 特征序列底层单元测试
"""

from __future__ import annotations

import pytest

from newchan.a_stroke import Stroke
from newchan.a_segment_v0 import Segment
from newchan.a_segment_v1 import segments_from_strokes_v1
from newchan.a_feature_sequence import (
    FeatureBar,
    build_feature_sequence,
    merge_inclusion_feature,
)
from newchan.a_fractal_feature import FeatureFractal, fractals_from_feature
from newchan.a_assertions import assert_segment_theorem_v1


# ── helper ──

def _s(i0, i1, direction, high, low, confirmed=True):
    if direction == "up":
        p0, p1 = low, high
    else:
        p0, p1 = high, low
    return Stroke(i0=i0, i1=i1, direction=direction,
                  high=high, low=low, p0=p0, p1=p1, confirmed=confirmed)


# =====================================================================
# F) 特征序列底层单元测试
# =====================================================================

class TestFeatureSequence:
    """build_feature_sequence / merge_inclusion_feature 底层验证。"""

    def test_build_extracts_opposite(self):
        """向上段只取 down 笔。"""
        strokes = [
            _s(0, 5, "up", 15, 5),
            _s(5, 10, "down", 14, 8),
            _s(10, 15, "up", 18, 7),
            _s(15, 20, "down", 13, 6),
            _s(20, 25, "up", 20, 9),
        ]
        seq = build_feature_sequence(strokes, 0, 4, "up")
        assert len(seq) == 2
        assert all(strokes[fb.idx].direction == "down" for fb in seq)

    def test_build_down_segment_takes_up_strokes(self):
        """向下段取 up 笔。"""
        strokes = [
            _s(0, 5, "down", 25, 15),
            _s(5, 10, "up", 18, 12),
            _s(10, 15, "down", 22, 13),
        ]
        seq = build_feature_sequence(strokes, 0, 2, "down")
        assert len(seq) == 1
        assert seq[0].idx == 1

    def test_merge_no_inclusion(self):
        """无包含 → 标准序列与原序列等长。"""
        seq = [
            FeatureBar(idx=1, high=12, low=8),
            FeatureBar(idx=3, high=16, low=11),
            FeatureBar(idx=5, high=14, low=9),
        ]
        merged, m2r = merge_inclusion_feature(seq)
        assert len(merged) == 3
        assert m2r == [(0, 0), (1, 1), (2, 2)]

    def test_merge_with_inclusion(self):
        """有包含 → 合并后变短。"""
        seq = [
            FeatureBar(idx=1, high=15, low=8),
            FeatureBar(idx=3, high=14, low=9),   # 包含于 [0]: 15>=14, 8<=9
            FeatureBar(idx=5, high=10, low=5),
        ]
        merged, m2r = merge_inclusion_feature(seq)
        assert len(merged) == 2
        # 第一个 merged bar 覆盖 raw [0,1]
        assert m2r[0] == (0, 1)


class TestFeatureFractal:
    """fractals_from_feature 分型识别。"""

    def test_top_fractal(self):
        """中间 bar 的 high 和 low 都最大 → top。"""
        seq = [
            FeatureBar(idx=1, high=12, low=8),
            FeatureBar(idx=3, high=16, low=11),
            FeatureBar(idx=5, high=14, low=9),
        ]
        fxs = fractals_from_feature(seq)
        assert len(fxs) == 1
        assert fxs[0].kind == "top"
        assert fxs[0].idx == 1

    def test_bottom_fractal(self):
        """中间 bar 的 high 和 low 都最小 → bottom。"""
        seq = [
            FeatureBar(idx=1, high=18, low=12),
            FeatureBar(idx=3, high=14, low=8),
            FeatureBar(idx=5, high=16, low=10),
        ]
        fxs = fractals_from_feature(seq)
        assert len(fxs) == 1
        assert fxs[0].kind == "bottom"
        assert fxs[0].idx == 1

    def test_no_fractal_monotonic(self):
        """单调序列无分型。"""
        seq = [
            FeatureBar(idx=1, high=10, low=5),
            FeatureBar(idx=3, high=12, low=7),
            FeatureBar(idx=5, high=14, low=9),
        ]
        assert fractals_from_feature(seq) == []


# =====================================================================
# A) 无足够反向笔 → 不触发 → 单段
# =====================================================================

class TestNoTriggerSingleSegment:
    """反向笔不足 3 个，无法形成分型 → 只输出一段。"""

    def test_5_strokes_2_reverse(self):
        """5 笔（up 段），only 2 down strokes → no fractal → 1 segment."""
        strokes = [
            _s(0, 5, "up", 15, 5),
            _s(5, 10, "down", 14, 8),
            _s(10, 15, "up", 18, 7),
            _s(15, 20, "down", 13, 6),
            _s(20, 25, "up", 20, 9),
        ]
        segs = segments_from_strokes_v1(strokes)
        assert len(segs) == 1
        assert segs[0].s0 == 0
        assert segs[0].s1 == 4

    def test_3_strokes_minimum(self):
        """最小 3 笔 → 只有 1 反向笔 → 1 段。"""
        strokes = [
            _s(0, 5, "up", 20, 5),
            _s(5, 10, "down", 18, 8),
            _s(10, 15, "up", 22, 6),
        ]
        segs = segments_from_strokes_v1(strokes)
        assert len(segs) == 1


# =====================================================================
# B) top fractal（up 段）→ 产生分段
# =====================================================================

class TestTopFractalBreak:
    """向上段中 down 笔序列出现 top fractal → 段终结。"""

    def _make_strokes(self) -> list[Stroke]:
        """7 笔向上段，down 笔在 idx=3 形成 peak。

        Feature seq (down strokes at 1,3,5):
          FB(1, h=12, l=8)
          FB(3, h=16, l=11)  ← peak (both h&l highest)
          FB(5, h=14, l=9)
        → top fractal at std_seq pos 1 → trigger
        """
        return [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 11),  # peak down stroke
            _s(20, 25, "up",  20, 9),
            _s(25, 30, "down", 14, 9),
            _s(30, 35, "up",  22, 10),
        ]

    def test_produces_two_segments(self):
        segs = segments_from_strokes_v1(self._make_strokes())
        assert len(segs) == 2

    def test_first_segment_endpoints(self):
        segs = segments_from_strokes_v1(self._make_strokes())
        assert segs[0].s0 == 0
        # 分型中心 b = stroke[3]（反向笔），旧段终点 = stroke[2]
        assert segs[0].s1 == 2
        assert segs[0].direction == "up"

    def test_second_segment_starts_at_break(self):
        segs = segments_from_strokes_v1(self._make_strokes())
        # 新段从分型中心 b（stroke[3]）开始
        assert segs[1].s0 == 3
        assert segs[1].s0 == segs[0].s1 + 1  # 新拼接语义


# =====================================================================
# C) bottom fractal（down 段）→ 产生分段
# =====================================================================

class TestBottomFractalBreak:
    """向下段中 up 笔序列出现 bottom fractal → 段终结。"""

    def _make_strokes(self) -> list[Stroke]:
        """7 笔向下段，up 笔在 idx=3 形成 trough。

        Feature seq (up strokes at 1,3,5):
          FB(1, h=18, l=12)
          FB(3, h=14, l=8)   ← trough (both h&l lowest)
          FB(5, h=16, l=10)
        → bottom fractal at std_seq pos 1 → trigger
        """
        return [
            _s(0, 5, "down",  25, 15),
            _s(5, 10, "up",   18, 12),
            _s(10, 15, "down", 22, 13),
            _s(15, 20, "up",  14, 8),   # trough up stroke
            _s(20, 25, "down", 20, 11),
            _s(25, 30, "up",  16, 10),
            _s(30, 35, "down", 18, 9),
        ]

    def test_produces_two_segments(self):
        segs = segments_from_strokes_v1(self._make_strokes())
        assert len(segs) == 2

    def test_first_segment_is_down(self):
        segs = segments_from_strokes_v1(self._make_strokes())
        assert segs[0].direction == "down"
        assert segs[0].s0 == 0
        # 分型中心 b = stroke[3]（反向笔），旧段终点 = stroke[2]
        assert segs[0].s1 == 2

    def test_segments_stitched(self):
        segs = segments_from_strokes_v1(self._make_strokes())
        # 新拼接语义：s1 + 1 == s0
        assert segs[1].s0 == segs[0].s1 + 1


# =====================================================================
# D) confirmed 语义
# =====================================================================

class TestConfirmed:
    """§5.3 最后一段 confirmed=False，其余 True。"""

    def test_single_segment_unconfirmed(self):
        strokes = [
            _s(0, 5, "up", 20, 5),
            _s(5, 10, "down", 18, 8),
            _s(10, 15, "up", 22, 6),
        ]
        segs = segments_from_strokes_v1(strokes)
        assert len(segs) == 1
        assert segs[0].confirmed is False

    def test_two_segments_first_confirmed(self):
        strokes = [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 11),
            _s(20, 25, "up",  20, 9),
            _s(25, 30, "down", 14, 9),
            _s(30, 35, "up",  22, 10),
        ]
        segs = segments_from_strokes_v1(strokes)
        assert len(segs) >= 2
        assert segs[0].confirmed is True
        assert segs[-1].confirmed is False


# =====================================================================
# E) 断言集成
# =====================================================================

class TestAssertIntegration:
    """assert_segment_theorem_v1 集成测试。"""

    def test_valid_pass(self):
        strokes = [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 11),
            _s(20, 25, "up",  20, 9),
            _s(25, 30, "down", 14, 9),
            _s(30, 35, "up",  22, 10),
        ]
        segs = segments_from_strokes_v1(strokes)
        result = assert_segment_theorem_v1(strokes, segs)
        assert result.ok is True

    def test_empty_pass(self):
        result = assert_segment_theorem_v1([], [])
        assert result.ok is True

    def test_bad_stitching_fails(self):
        """手工构造不拼接的 segments → 断言失败。"""
        bad = [
            Segment(s0=0, s1=3, i0=0, i1=20, direction="up",
                    high=20, low=5, confirmed=True),
            Segment(s0=5, s1=8, i0=25, i1=40, direction="down",
                    high=25, low=10, confirmed=False),
        ]
        strokes_dummy = [_s(i * 5, (i + 1) * 5, "up" if i % 2 == 0 else "down",
                           20, 5) for i in range(9)]
        result = assert_segment_theorem_v1(strokes_dummy, bad)
        assert result.ok is False
        assert "not adjacent" in result.message or "not stitched" in result.message

    def test_bad_confirmed_fails(self):
        """最后一段 confirmed=True → 断言失败。"""
        bad = [
            Segment(s0=0, s1=3, i0=0, i1=20, direction="up",
                    high=20, low=5, confirmed=True),
        ]
        result = assert_segment_theorem_v1([], bad)
        assert result.ok is False
        assert "confirmed=False" in result.message


# =====================================================================
# 额外边界测试
# =====================================================================

class TestEdgeCases:
    """边界条件。"""

    def test_fewer_than_3_strokes(self):
        assert segments_from_strokes_v1([]) == []
        assert segments_from_strokes_v1([_s(0, 5, "up", 20, 5)]) == []

    def test_no_overlap_start(self):
        """三笔无交集重叠 → 无法启动 → 空。"""
        strokes = [
            _s(0, 5, "up",   10, 5),
            _s(5, 10, "down", 20, 12),
            _s(10, 15, "up",  30, 22),
        ]
        segs = segments_from_strokes_v1(strokes)
        assert len(segs) == 0

    def test_segment_high_low_covers_range(self):
        """段的 high/low 覆盖所有包含笔的极值。"""
        strokes = [
            _s(0, 5, "up", 15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up", 18, 7),
            _s(15, 20, "down", 13, 6),
            _s(20, 25, "up", 20, 9),
        ]
        segs = segments_from_strokes_v1(strokes)
        assert segs[0].high == 20.0
        assert segs[0].low == 5.0


class TestSegmentEndpointContract:
    """对象层端点语义契约：Segment 端点一次性确定，不依赖桥接层重算。"""

    @staticmethod
    def _endpoint_for_type(stroke: Stroke, fractal_type: str) -> tuple[int, float]:
        if fractal_type == "top":
            if stroke.direction == "down":
                return stroke.i0, stroke.p0
            return stroke.i1, stroke.p1
        if stroke.direction == "down":
            return stroke.i1, stroke.p1
        return stroke.i0, stroke.p0

    def test_segment_ep_fields_follow_direction_semantics(self):
        """ep0/ep1 取边界笔端点，保证相邻段首尾相连。"""
        strokes = [
            _s(0, 5, "up", 15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up", 18, 7),
            _s(15, 20, "down", 16, 11),
            _s(20, 25, "up", 20, 9),
            _s(25, 30, "down", 14, 9),
            _s(30, 35, "up", 22, 10),
        ]
        segs = segments_from_strokes_v1(strokes)
        assert len(segs) >= 1

        for seg in segs:
            start_type = "bottom" if seg.direction == "up" else "top"
            end_type = "top" if seg.direction == "up" else "bottom"
            exp_ep0_i, exp_ep0_price = self._endpoint_for_type(strokes[seg.s0], start_type)
            exp_ep1_i, exp_ep1_price = self._endpoint_for_type(strokes[seg.s1], end_type)

            assert seg.ep0_type == start_type
            assert seg.ep1_type == end_type
            assert seg.ep0_i == exp_ep0_i
            assert seg.ep1_i == exp_ep1_i
            assert seg.ep0_price == pytest.approx(exp_ep0_price)
            assert seg.ep1_price == pytest.approx(exp_ep1_price)
            assert seg.p0 == pytest.approx(seg.ep0_price)
            assert seg.p1 == pytest.approx(seg.ep1_price)
