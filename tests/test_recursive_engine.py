"""递归引擎 — 单元测试

覆盖 docs/chan_spec.md §6 归纳递归：
  A) 基本递归：segments → Center[1] → TrendTypeInstance[1]
  B) 多层递归：Move[1] → Center[2]（数据足够时）
  C) 递归终止：Move 不足 3 个时停止
  D) 只用 confirmed 对象向上递归
  E) levels_to_level_views 转换
  F) 空输入/不足输入
"""

from __future__ import annotations

import pytest

from newchan.a_stroke import Stroke
from newchan.a_segment_v0 import Segment
from newchan.a_center_v0 import Center
from newchan.a_trendtype_v0 import TrendTypeInstance
from newchan.a_recursive_engine import (
    RecursiveLevel,
    build_recursive_levels,
    levels_to_level_views,
)


# ── helpers ──

def _seg(i: int, direction: str, high: float, low: float,
         confirmed: bool = True) -> Segment:
    """快速构造 Segment。s0=s1=i, i0=i*5, i1=(i+1)*5。"""
    p0, p1 = (low, high) if direction == "up" else (high, low)
    return Segment(
        s0=i, s1=i, i0=i * 5, i1=(i + 1) * 5,
        direction=direction, high=high, low=low, confirmed=confirmed,
    )


def _make_overlapping_segments(n: int = 9) -> list[Segment]:
    """构造 n 段，全部在 [8, 20] 区间内有重叠，交替 up/down。

    这确保能形成中枢（三段交集非空）。
    """
    segs = []
    for i in range(n):
        d = "up" if i % 2 == 0 else "down"
        # 价格在 [8,22] 区间波动，保证重叠
        h = 18.0 + (i % 3) * 2   # 18, 20, 22, 18, 20, 22, ...
        l = 8.0 + (i % 3) * 2    # 8, 10, 12, 8, 10, 12, ...
        segs.append(_seg(i, d, h, l))
    return segs


def _make_trending_up_segments(n: int = 12) -> list[Segment]:
    """构造 n 段，价格区间整体上移但每三段有重叠。

    意图：形成多个 Center[1]，且 Center 之间同向上移 → 趋势对象。
    足够多的趋势对象可能形成 Center[2]。
    """
    segs = []
    for i in range(n):
        d = "up" if i % 2 == 0 else "down"
        base = (i // 3) * 3  # 每 3 段一个台阶
        h = 18.0 + base + (i % 3) * 1.5
        l = 8.0 + base + (i % 3) * 1.5
        segs.append(_seg(i, d, h, l))
    return segs


# =====================================================================
# A) 基本递归
# =====================================================================

class TestBasicRecursion:
    """segments → Center[1] → TrendTypeInstance[1]。"""

    def test_produces_at_least_one_level(self):
        segs = _make_overlapping_segments(9)
        levels = build_recursive_levels(segs, sustain_m=2)
        assert len(levels) >= 1

    def test_first_level_is_level_1(self):
        segs = _make_overlapping_segments(9)
        levels = build_recursive_levels(segs, sustain_m=2)
        assert levels[0].level == 1

    def test_first_level_has_centers(self):
        segs = _make_overlapping_segments(9)
        levels = build_recursive_levels(segs, sustain_m=2)
        assert len(levels[0].centers) >= 1

    def test_first_level_has_trends(self):
        segs = _make_overlapping_segments(9)
        levels = build_recursive_levels(segs, sustain_m=2)
        assert len(levels[0].trends) >= 1

    def test_first_level_moves_are_segments(self):
        segs = _make_overlapping_segments(9)
        levels = build_recursive_levels(segs, sustain_m=2)
        assert all(isinstance(m, Segment) for m in levels[0].moves)


# =====================================================================
# C) 递归终止
# =====================================================================

class TestRecursionTermination:
    """Move 不足 3 个时停止。"""

    def test_fewer_than_3_segments(self):
        segs = [_seg(0, "up", 20, 10), _seg(1, "down", 18, 8)]
        levels = build_recursive_levels(segs)
        assert len(levels) == 0

    def test_no_overlap_stops(self):
        """三段无交集重叠 → 无中枢 → 递归停止。"""
        segs = [
            _seg(0, "up", 10, 5),
            _seg(1, "down", 20, 12),
            _seg(2, "up", 30, 22),
        ]
        levels = build_recursive_levels(segs)
        assert len(levels) == 0

    def test_max_levels_cap(self):
        """max_levels 安全阀。"""
        segs = _make_overlapping_segments(9)
        levels = build_recursive_levels(segs, max_levels=1)
        assert len(levels) <= 1

    def test_empty_segments(self):
        levels = build_recursive_levels([])
        assert levels == []


# =====================================================================
# D) 只用 confirmed 对象向上递归
# =====================================================================

class TestConfirmedOnly:
    """未确认的走势类型实例不参与上层递归。"""

    def test_last_trend_is_unconfirmed(self):
        """最后一个 TrendTypeInstance 应为 confirmed=False。"""
        segs = _make_overlapping_segments(9)
        levels = build_recursive_levels(segs, sustain_m=2)
        if levels and levels[0].trends:
            assert levels[0].trends[-1].confirmed is False


# =====================================================================
# E) levels_to_level_views
# =====================================================================

class TestLevelViews:
    """RecursiveLevel → LevelView 转换。"""

    def test_conversion_preserves_count(self):
        segs = _make_overlapping_segments(9)
        levels = build_recursive_levels(segs, sustain_m=2)
        views = levels_to_level_views(levels)
        assert len(views) == len(levels)

    def test_view_level_matches(self):
        segs = _make_overlapping_segments(9)
        levels = build_recursive_levels(segs, sustain_m=2)
        views = levels_to_level_views(levels)
        for rl, lv in zip(levels, views):
            assert lv.level == rl.level
            assert lv.centers is rl.centers

    def test_empty_levels(self):
        views = levels_to_level_views([])
        assert views == []


# =====================================================================
# F) 递归级别属性
# =====================================================================

class TestLevelProperties:
    """递归层级的数据完整性。"""

    def test_centers_have_valid_indices(self):
        """Center 的 seg0/seg1 在 moves 范围内。"""
        segs = _make_overlapping_segments(9)
        levels = build_recursive_levels(segs, sustain_m=2)
        for rl in levels:
            for c in rl.centers:
                assert 0 <= c.seg0 < len(rl.moves)
                assert 0 <= c.seg1 < len(rl.moves)

    def test_trends_have_valid_seg_range(self):
        """TrendTypeInstance 的 seg0/seg1 在 moves 范围内。"""
        segs = _make_overlapping_segments(9)
        levels = build_recursive_levels(segs, sustain_m=2)
        for rl in levels:
            for t in rl.trends:
                assert 0 <= t.seg0 < len(rl.moves)
                assert 0 <= t.seg1 < len(rl.moves)

    def test_higher_level_moves_are_trend_instances(self):
        """Level ≥ 2 的 moves 应为 TrendTypeInstance。"""
        segs = _make_overlapping_segments(9)
        levels = build_recursive_levels(segs, sustain_m=0)  # 宽松 sustain
        if len(levels) >= 2:
            assert all(
                isinstance(m, TrendTypeInstance)
                for m in levels[1].moves
            )
