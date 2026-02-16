"""线段 v0 vs v1 对比测试 — 谱系 003 实证验证

基于审计工位C设计的5类场景，量化两口径差异：
1. 正常递升序列（两口径应一致）
2. 连续重叠序列（暴露贪心 vs 分型差异）
3. 退化段序列（验证 v1 是否拒绝退化）
4. 古怪线段触发点（v1 结算锚效应）
5. 方向/端点一致性（两口径通用验证）

对比指标：段数、方向一致性、退化段数、端点有效性。
"""

from __future__ import annotations

from __future__ import annotations

from typing import Literal

import pytest

from newchan.a_stroke import Stroke
from newchan.a_segment_v0 import Segment, segments_from_strokes_v0
from newchan.a_segment_v1 import segments_from_strokes_v1


# ── helper ──────────────────────────────────────────────────────────

def _s(i0: int, i1: int, direction: Literal["up", "down"], high: float, low: float,
       confirmed: bool = True) -> Stroke:
    if direction == "up":
        p0, p1 = low, high
    else:
        p0, p1 = high, low
    return Stroke(i0=i0, i1=i1, direction=direction,
                  high=high, low=low, p0=p0, p1=p1, confirmed=confirmed)


def _is_degenerate(seg: Segment) -> bool:
    """退化段：方向与端点价格矛盾。"""
    if seg.ep0_price == 0.0 or seg.ep1_price == 0.0:
        return False
    if seg.direction == "up":
        return seg.ep1_price < seg.ep0_price
    else:
        return seg.ep1_price > seg.ep0_price


def _run_both(strokes: list[Stroke]):
    """同时运行 v0 和 v1，返回 (v0_segs, v1_segs)。"""
    return segments_from_strokes_v0(strokes), segments_from_strokes_v1(strokes)


# =====================================================================
# 场景 1：正常递升序列（两口径应一致）
# =====================================================================

class TestNormalSequence:
    """简单三笔序列，两口径都应产出1段。"""

    def test_basic_3_strokes_both_produce_one_segment(self):
        strokes = [
            _s(0, 4, "up",   high=20.0, low=5.0),
            _s(4, 8, "down", high=18.0, low=8.0),
            _s(8, 12, "up",  high=22.0, low=6.0),
        ]
        v0, v1 = _run_both(strokes)
        assert len(v0) == 1
        assert len(v1) == 1
        assert v0[0].direction == v1[0].direction == "up"

    def test_basic_5_strokes_same_direction(self):
        strokes = [
            _s(0, 4, "up",   high=20.0, low=5.0),
            _s(4, 8, "down", high=18.0, low=8.0),
            _s(8, 12, "up",  high=22.0, low=6.0),
            _s(12, 16, "down", high=15.0, low=3.0),
            _s(16, 20, "up",  high=25.0, low=10.0),
        ]
        v0, v1 = _run_both(strokes)
        # 两者都应至少产出1段，方向一致
        assert len(v0) >= 1
        assert len(v1) >= 1
        assert v0[0].direction == v1[0].direction


# =====================================================================
# 场景 2：连续重叠（暴露贪心 vs 分型差异）
# =====================================================================

class TestContinuousOverlap:
    """6笔全在 [5,20] 区间重叠。
    假说：v0 贪心截断产出更多段，v1 等待分型产出更少段。
    """

    def test_v0_greedy_produces_more_segments(self):
        strokes = [
            _s(0, 4, "up",   high=20.0, low=5.0),
            _s(4, 8, "down", high=18.0, low=8.0),
            _s(8, 12, "up",  high=22.0, low=6.0),
            _s(12, 16, "down", high=19.0, low=7.0),
            _s(16, 20, "up",  high=21.0, low=8.0),
            _s(20, 24, "down", high=17.0, low=5.0),
        ]
        v0, v1 = _run_both(strokes)
        # v0 贪心：j=0 成段[0,2]，j=2 成段[2,4]，可能段数>=2
        # v1 等待分型：可能只有1段
        assert len(v0) >= len(v1), (
            f"假说违反：v0={len(v0)}段 < v1={len(v1)}段"
        )


# =====================================================================
# 场景 3：退化段检测
# =====================================================================

class TestDegenerateSegments:
    """含回头波动的序列，可能产生退化段。
    假说：v0 可能产出退化段，v1 应该不会。
    """

    def test_degenerate_count_v1_leq_v0(self):
        """v1 产出的退化段数不应多于 v0。"""
        strokes = [
            _s(0, 4, "up",   high=20.0, low=5.0),
            _s(4, 8, "down", high=18.0, low=8.0),
            _s(8, 12, "up",  high=15.0, low=6.0),   # 回头：high < 第1笔 high
            _s(12, 16, "down", high=14.0, low=4.0),
            _s(16, 20, "up",  high=22.0, low=7.0),
            _s(20, 24, "down", high=19.0, low=9.0),
            _s(24, 28, "up",  high=25.0, low=11.0),
        ]
        v0, v1 = _run_both(strokes)
        degen_v0 = sum(1 for seg in v0 if _is_degenerate(seg))
        degen_v1 = sum(1 for seg in v1 if _is_degenerate(seg))
        assert degen_v1 <= degen_v0, (
            f"假说违反：v1退化段({degen_v1}) > v0退化段({degen_v0})"
        )

    def test_no_degenerate_in_clean_sequence(self):
        """干净的递升序列，两口径都不应有退化段。"""
        strokes = [
            _s(0, 4, "up",   high=20.0, low=5.0),
            _s(4, 8, "down", high=18.0, low=10.0),
            _s(8, 12, "up",  high=25.0, low=12.0),
            _s(12, 16, "down", high=22.0, low=15.0),
            _s(16, 20, "up",  high=30.0, low=18.0),
        ]
        v0, v1 = _run_both(strokes)
        for seg in v0 + v1:
            assert not _is_degenerate(seg), (
                f"干净序列不应退化：{seg.direction} ep0={seg.ep0_price} ep1={seg.ep1_price}"
            )


# =====================================================================
# 场景 4：结算锚效应（v1 特有）
# =====================================================================

class TestSettlementAnchor:
    """v1 结算锚：新段前三笔必须重叠，否则拒绝触发。
    v0 无此机制。
    """

    def test_v0_never_has_break_evidence(self):
        """v0 永远没有 break_evidence（结构性差异）。"""
        strokes = [
            _s(0, 4, "up",   high=20.0, low=5.0),
            _s(4, 8, "down", high=18.0, low=8.0),
            _s(8, 12, "up",  high=22.0, low=6.0),
            _s(12, 16, "down", high=15.0, low=3.0),
            _s(16, 20, "up",  high=25.0, low=10.0),
            _s(20, 24, "down", high=20.0, low=7.0),
            _s(24, 28, "up",  high=28.0, low=12.0),
        ]
        v0, _v1 = _run_both(strokes)
        # v0 结构性不产生 break_evidence
        assert all(seg.break_evidence is None for seg in v0)


# =====================================================================
# 场景 5：通用性质验证
# =====================================================================

class TestUniversalProperties:
    """两口径都应满足的通用性质。"""

    @pytest.fixture
    def strokes_7(self):
        return [
            _s(0, 4, "up",   high=20.0, low=5.0),
            _s(4, 8, "down", high=18.0, low=8.0),
            _s(8, 12, "up",  high=22.0, low=6.0),
            _s(12, 16, "down", high=19.0, low=7.0),
            _s(16, 20, "up",  high=24.0, low=9.0),
            _s(20, 24, "down", high=21.0, low=6.0),
            _s(24, 28, "up",  high=26.0, low=10.0),
        ]

    def test_direction_matches_first_stroke(self, strokes_7):
        """段方向 = 第一笔方向。"""
        v0, v1 = _run_both(strokes_7)
        for segs, label in [(v0, "v0"), (v1, "v1")]:
            for seg in segs:
                first_stroke = strokes_7[seg.s0]
                assert seg.direction == first_stroke.direction, (
                    f"{label} 段[{seg.s0},{seg.s1}] 方向{seg.direction} "
                    f"!= 首笔方向{first_stroke.direction}"
                )

    def test_last_segment_not_confirmed(self, strokes_7):
        """两口径：最后一段 confirmed=False。"""
        v0, v1 = _run_both(strokes_7)
        if v0:
            assert not v0[-1].confirmed
        if v1:
            assert not v1[-1].confirmed

    def test_fewer_than_3_strokes_empty(self):
        """不足3笔 → 两口径都返回空。"""
        strokes_2 = [
            _s(0, 4, "up", high=20.0, low=5.0),
            _s(4, 8, "down", high=18.0, low=8.0),
        ]
        v0, v1 = _run_both(strokes_2)
        assert v0 == []
        assert v1 == []

    def test_segment_stroke_range_valid(self, strokes_7):
        """段的 s0/s1 必须在笔列表范围内。"""
        v0, v1 = _run_both(strokes_7)
        n = len(strokes_7)
        for segs, label in [(v0, "v0"), (v1, "v1")]:
            for seg in segs:
                assert 0 <= seg.s0 < n, f"{label} s0={seg.s0} out of range"
                assert 0 <= seg.s1 < n, f"{label} s1={seg.s1} out of range"
                assert seg.s1 >= seg.s0, f"{label} s1={seg.s1} < s0={seg.s0}"
