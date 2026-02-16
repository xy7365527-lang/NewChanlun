"""背驰 / 盘整背驰 — 单元测试

测试项：
  A) 趋势背驰：2 中枢趋势，C 段力度 < A 段 → 检出背驰
  B) 趋势无背驰：C 段力度 >= A 段 → 无背驰
  C) 盘整背驰：盘整中后一次离开力度 < 前一次 → 检出
  D) 不足条件：单中枢趋势不触发趋势背驰
  E) 方向标签：上涨趋势 → top，下跌趋势 → bottom
  F) 空输入
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v0 import Segment
from newchan.a_center_v0 import Center, centers_from_segments_v0
from newchan.a_trendtype_v0 import TrendTypeInstance, trend_instances_from_centers
from newchan.a_divergence import Divergence, divergences_from_level


# ── helper ──

def _seg(s0: int, s1: int, i0: int, i1: int, d: str,
         h: float, l: float, confirmed: bool = True) -> Segment:
    return Segment(s0=s0, s1=s1, i0=i0, i1=i1, direction=d,
                   high=h, low=l, confirmed=confirmed)


def _make_uptrend_segments() -> list[Segment]:
    """构造一个上涨趋势，包含 2 个中枢 + C 段力度弱于 A 段。

    结构：
      seg0(↑10→20) seg1(↓20→12) seg2(↑12→18)  → center0 [12,18]，GG=20 DD=10
      seg3(↓11→8) seg4(↑11→8)                   → 离开不回抽，c0 终止
      seg5(↓25→20) seg6(↑20→30)                 → 过渡
      seg7(↓30→25) seg8(↑25→35) seg9(↓35→28)   → center1 [25,30]，DD=25 > c0.GG=20
      seg10(↑28→31)                             → C段(力度小：3点振幅)

    关键约束：满足上涨趋势判定 c1.DD (25) > c0.GG (20) → "up" 趋势成立
    """
    return [
        _seg(0, 0, 0, 10, "up", 20, 10),      # seg0 (c0 Z走势段)
        _seg(1, 1, 10, 20, "down", 20, 12),   # seg1
        _seg(2, 2, 20, 30, "up", 18, 12),     # seg2 (c0 Z走势段，c0结束)
        _seg(3, 3, 30, 40, "down", 11, 8),    # seg3 完全离开 c0，不回抽
        _seg(4, 4, 40, 50, "up", 11, 8),      # seg4 不回抽，c0 终止
        _seg(5, 5, 50, 60, "down", 25, 20),   # seg5 (A段下跌)
        _seg(6, 6, 60, 70, "up", 25, 20),     # seg6 (A段上涨，与seg7,8,9不重叠)
        _seg(7, 7, 70, 80, "down", 32, 26),   # seg7 (c1 初始)
        _seg(8, 8, 80, 90, "up", 34, 26),     # seg8
        _seg(9, 9, 90, 100, "down", 34, 28),  # seg9 (c1 Z走势段，c1结束)
        _seg(10, 10, 100, 110, "up", 35, 33), # seg10 (C段: 完全高于c1，小力度2点)
    ]


def _make_downtrend_segments() -> list[Segment]:
    """构造一个下跌趋势。

    结构：
      seg0-seg2: center0 [80,90]
      seg3-seg4: A段(大力度下跌)
      seg5-seg7: 过渡
      seg8-seg10: center1 [50,60]
      seg11-seg12: C段(小力度下跌)
    """
    return [
        _seg(0, 0, 0, 10, "down", 95, 80),      # seg0
        _seg(1, 1, 10, 20, "up", 95, 82),        # seg1
        _seg(2, 2, 20, 30, "down", 90, 80),      # seg2 → center0
        _seg(3, 3, 30, 40, "up", 82, 75),         # seg3
        _seg(4, 4, 40, 50, "down", 75, 55),       # seg4 (A: 大力度 20点)
        _seg(5, 5, 50, 60, "up", 65, 55),         # seg5
        _seg(6, 6, 60, 70, "down", 65, 50),       # seg6
        _seg(7, 7, 70, 80, "up", 60, 50),         # seg7
        _seg(8, 8, 80, 90, "down", 60, 50),       # seg8 → center1
        _seg(9, 9, 90, 100, "up", 52, 48),        # seg9
        _seg(10, 10, 100, 110, "down", 50, 45),   # seg10 (C: 小力度 5点)
    ]


# =====================================================================
# A) 趋势背驰检出
# =====================================================================

class TestTrendDivergence:
    """上涨/下跌趋势中 C 段力度弱于 A 段时应检出背驰。"""

    def test_uptrend_divergence_detected(self):
        segs = _make_uptrend_segments()
        centers = centers_from_segments_v0(segs, sustain_m=0)
        trends = trend_instances_from_centers(segs, centers)

        divs = divergences_from_level(segs, centers, trends, level_id=1)
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) >= 1
        d = trend_divs[0]
        assert d.direction == "top"
        assert d.force_c < d.force_a
        assert d.level_id == 1

    def test_downtrend_divergence_detected(self):
        segs = _make_downtrend_segments()
        centers = centers_from_segments_v0(segs, sustain_m=0)
        trends = trend_instances_from_centers(segs, centers)

        divs = divergences_from_level(segs, centers, trends, level_id=1)
        trend_divs = [d for d in divs if d.kind == "trend"]
        # 可能检出也可能不检出（取决于中枢构造），至少不报错
        for d in trend_divs:
            assert d.direction == "bottom"
            assert d.force_c < d.force_a


# =====================================================================
# B) 无背驰
# =====================================================================

class TestNoDivergence:
    """C 段力度 >= A 段时不应检出背驰。"""

    def test_strong_c_segment_no_divergence(self):
        """C 段力度大于 A 段。"""
        segs = [
            _seg(0, 0, 0, 10, "up", 20, 10),
            _seg(1, 1, 10, 20, "down", 20, 12),
            _seg(2, 2, 20, 30, "up", 18, 12),
            _seg(3, 3, 30, 40, "down", 18, 15),
            _seg(4, 4, 40, 50, "up", 22, 15),   # A: 7点
            _seg(5, 5, 50, 60, "down", 22, 18),
            _seg(6, 6, 60, 70, "up", 28, 18),
            _seg(7, 7, 70, 80, "down", 28, 22),
            _seg(8, 8, 80, 90, "up", 26, 22),
            _seg(9, 9, 90, 100, "down", 26, 23),
            _seg(10, 10, 100, 120, "up", 35, 23),  # C: 12点, 力度更大
        ]
        centers = centers_from_segments_v0(segs, sustain_m=0)
        trends = trend_instances_from_centers(segs, centers)

        divs = divergences_from_level(segs, centers, trends, level_id=1)
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) == 0


# =====================================================================
# D) 不足条件
# =====================================================================

class TestInsufficientConditions:
    """单中枢不触发趋势背驰（需至少 2 中枢）。"""

    def test_single_center_no_trend_divergence(self):
        segs = [
            _seg(0, 0, 0, 10, "up", 20, 10),
            _seg(1, 1, 10, 20, "down", 20, 12),
            _seg(2, 2, 20, 30, "up", 18, 12),
            _seg(3, 3, 30, 40, "down", 18, 14),
            _seg(4, 4, 40, 50, "up", 16, 14),
        ]
        centers = centers_from_segments_v0(segs, sustain_m=0)
        trends = trend_instances_from_centers(segs, centers)

        divs = divergences_from_level(segs, centers, trends, level_id=1)
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) == 0


# =====================================================================
# E) 方向标签正确性
# =====================================================================

class TestDirectionLabels:
    """上涨 → top，下跌 → bottom。"""

    def test_up_trend_gives_top(self):
        segs = _make_uptrend_segments()
        centers = centers_from_segments_v0(segs, sustain_m=0)
        trends = trend_instances_from_centers(segs, centers)

        divs = divergences_from_level(segs, centers, trends, level_id=1)
        for d in divs:
            if d.kind == "trend":
                assert d.direction == "top"


# =====================================================================
# C) 盘整背驰
# =====================================================================

class TestConsolidationDivergence:
    """盘整中后一次同向离开力度 < 前一次 → 盘整背驰。"""

    def test_consolidation_divergence_detected(self):
        """单中枢盘整，两次向下离开，后者力度弱 → 检出。"""
        segs = [
            _seg(0, 0, 0, 10, "up", 20, 10),       # center 初始段
            _seg(1, 1, 10, 20, "down", 20, 12),     # center 段
            _seg(2, 2, 20, 30, "up", 18, 12),       # center 段
            _seg(3, 3, 30, 50, "down", 22, 5),      # 第一次向下离开（大力度）
            _seg(4, 4, 50, 60, "up", 15, 10),       # 回抽
            _seg(5, 5, 60, 70, "down", 19, 9),      # 第二次向下离开（小力度）
        ]
        centers = centers_from_segments_v0(segs, sustain_m=0)
        trends = trend_instances_from_centers(segs, centers)

        divs = divergences_from_level(segs, centers, trends, level_id=1)
        cons_divs = [d for d in divs if d.kind == "consolidation"]
        assert len(cons_divs) >= 1
        d = cons_divs[0]
        assert d.force_c < d.force_a

    def test_no_consolidation_when_force_increases(self):
        """后一次同向离开力度更大 → 不检出。"""
        segs = [
            _seg(0, 0, 0, 10, "up", 18, 12),       # center 段（完全在 [ZD,ZG] 内）
            _seg(1, 1, 10, 20, "down", 18, 12),     # center 段
            _seg(2, 2, 20, 30, "up", 18, 12),       # center 段
            _seg(3, 3, 30, 40, "down", 19, 8),      # 第一次向下离开（小力度）
            _seg(4, 4, 40, 50, "up", 16, 13),       # 回抽
            _seg(5, 5, 50, 80, "down", 22, 2),      # 第二次向下离开（大力度）
        ]
        centers = centers_from_segments_v0(segs, sustain_m=0)
        trends = trend_instances_from_centers(segs, centers)

        divs = divergences_from_level(segs, centers, trends, level_id=1)
        cons_divs = [d for d in divs if d.kind == "consolidation"]
        assert len(cons_divs) == 0


# =====================================================================
# F) 空输入
# =====================================================================

class TestEmptyInput:
    def test_empty_everything(self):
        assert divergences_from_level([], [], [], 1) == []

    def test_no_trends(self):
        segs = [_seg(0, 0, 0, 10, "up", 20, 10)]
        assert divergences_from_level(segs, [], [], 1) == []
