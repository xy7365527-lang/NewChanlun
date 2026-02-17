"""三锚体系 L* 裁决 — 单元测试 (Step14 新缠论语汇)

覆盖 docs/chan_spec.md §9 + 三锚判定：
  A) SETTLE_ANCHOR_IN_CORE：price in core
  B) SETTLE_ANCHOR_IN_CORE：cur seg 重叠且 idx<=seg1
  C) RUN_ANCHOR_POST_EXIT：离开段后未见回抽
  D) EVENT_ANCHOR_FIRST_PULLBACK：出现反向段/触碰核，未再确认
  E) DEAD_NEGATION_SETTLED：回抽后再确认创新高/新低
  F) 对象驱动：无超时，离开后多段无对象否定仍保持alive
  G) DEAD_NOT_SETTLED：candidate center
  H) select_lstar_newchan：双 level / 无 alive
  I) assert_single_lstar 集成
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v0 import Segment
from newchan.a_center_v0 import Center
from newchan.a_level_fsm_newchan import (
    Regime,
    ExitSide,
    AliveCenter,
    AnchorSet,
    LevelView,
    LStar,
    classify_center_practical_newchan,
    select_lstar_newchan,
    overlap,
)
from newchan.a_assertions import assert_single_lstar


# ── helpers ──

def _seg(s0, s1, i0, i1, d, h, l, confirmed=True):
    return Segment(s0=s0, s1=s1, i0=i0, i1=i1, direction=d,
                   high=h, low=l, confirmed=confirmed)

def _center(seg0, seg1, low, high, kind="settled", confirmed=True, sustain=2):
    return Center(seg0=seg0, seg1=seg1, low=low, high=high,
                  kind=kind, confirmed=confirmed, sustain=sustain)


# 通用 segments: 10 段交替 up/down
#   seg0..seg2 形成核 [12,18]
#   seg3 向上离开（high=25, low=20 → ABOVE core 18）
#   seg4 反向 down（回抽）
#   seg5 再次 up（可能创新高）
def _make_segments(n: int = 10) -> list[Segment]:
    base = [
        _seg(0, 2, 0, 10, "up",   20.0, 10.0),    # seg0
        _seg(2, 4, 10, 20, "down", 18.0, 12.0),    # seg1
        _seg(4, 6, 20, 30, "up",   22.0, 11.0),    # seg2
        _seg(6, 8, 30, 40, "down", 19.0, 13.0),    # seg3 (延伸)
        _seg(8, 10, 40, 50, "up",  17.0, 14.0),    # seg4 (延伸)
        _seg(10, 12, 50, 60, "up",  25.0, 20.0),   # seg5 离开 ABOVE
        _seg(12, 14, 60, 70, "down", 21.0, 16.0),  # seg6 回抽（反向）
        _seg(14, 16, 70, 80, "up",  27.0, 19.0),   # seg7 再次 up 创新高
        _seg(16, 18, 80, 90, "down", 24.0, 17.0),  # seg8
        _seg(18, 20, 90, 100, "up", 26.0, 21.0),   # seg9
    ]
    return base[:n]

def _make_center_settled(seg0=0, seg1=4) -> Center:
    """settled center: core [12, 18], seg0..seg1=4, sustain=2."""
    return _center(seg0, seg1, low=12.0, high=18.0,
                   kind="settled", sustain=2)


# =====================================================================
# A) SETTLE_ANCHOR_IN_CORE — price in core
# =====================================================================

class TestSettleAnchorPriceInCore:

    def test_price_inside_core(self):
        segs = _make_segments(6)
        c = _make_center_settled(seg1=4)
        ac = classify_center_practical_newchan(c, 0, segs, last_price=15.0)
        assert ac.is_alive is True
        assert ac.regime == Regime.SETTLE_ANCHOR_IN_CORE

    def test_price_at_core_boundary(self):
        """价格 == 核上沿，仍在核内（闭区间）。"""
        segs = _make_segments(6)
        c = _make_center_settled(seg1=4)
        ac = classify_center_practical_newchan(c, 0, segs, last_price=18.0)
        assert ac.is_alive is True
        assert ac.regime == Regime.SETTLE_ANCHOR_IN_CORE

    def test_price_at_core_low(self):
        segs = _make_segments(6)
        c = _make_center_settled(seg1=4)
        ac = classify_center_practical_newchan(c, 0, segs, last_price=12.0)
        assert ac.is_alive is True
        assert ac.regime == Regime.SETTLE_ANCHOR_IN_CORE


# =====================================================================
# B) SETTLE_ANCHOR_IN_CORE — cur seg 重叠且 idx<=seg1
# =====================================================================

class TestSettleAnchorCurSegOverlap:

    def test_cur_seg_overlaps_and_within_seg1(self):
        """cur segment (最后一段) 与核重叠且 index <= center.seg1。"""
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            _seg(6, 8, 30, 40, "down", 19.0, 13.0),
            _seg(8, 10, 40, 50, "up",  17.0, 14.0),  # idx=4, overlaps core [12,18]
        ]
        c = _center(0, 4, low=12.0, high=18.0, kind="settled", sustain=2)
        # last_price outside core, but cur_seg (idx=4) overlaps and 4<=4
        ac = classify_center_practical_newchan(c, 0, segs, last_price=25.0)
        assert ac.is_alive is True
        assert ac.regime == Regime.SETTLE_ANCHOR_IN_CORE


# =====================================================================
# C) RUN_ANCHOR_POST_EXIT — 离开段后未见回抽
# =====================================================================

class TestRunAnchorPostExit:

    def test_just_exited_above(self):
        """仅有离开段，无后续段。"""
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            _seg(6, 8, 30, 40, "down", 19.0, 13.0),
            _seg(8, 10, 40, 50, "up",  17.0, 14.0),
            _seg(10, 12, 50, 60, "up", 25.0, 20.0),  # exit ABOVE
        ]
        c = _make_center_settled(seg1=4)
        ac = classify_center_practical_newchan(c, 0, segs, last_price=24.0)
        assert ac.is_alive is True
        assert ac.regime == Regime.RUN_ANCHOR_POST_EXIT
        assert ac.anchors.run_exit_side == ExitSide.ABOVE
        assert ac.anchors.run_exit_extreme == 25.0

    def test_exit_below(self):
        """向下离开。"""
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            _seg(6, 8, 30, 40, "down", 19.0, 13.0),
            _seg(8, 10, 40, 50, "up",  17.0, 14.0),
            _seg(10, 12, 50, 60, "down", 10.0, 5.0),  # exit BELOW (high=10 <= low=12? 10<=12 yes)
        ]
        c = _make_center_settled(seg1=4)
        ac = classify_center_practical_newchan(c, 0, segs, last_price=4.0)
        assert ac.is_alive is True
        assert ac.regime == Regime.RUN_ANCHOR_POST_EXIT
        assert ac.anchors.run_exit_side == ExitSide.BELOW


# =====================================================================
# D) EVENT_ANCHOR_FIRST_PULLBACK
# =====================================================================

class TestEventAnchorFirstPullback:

    def test_reverse_direction_pullback(self):
        """离开后出现反向段但尚未创新高 → 事件锚。"""
        segs = _make_segments(7)  # seg5=exit ABOVE, seg6=down(回抽)
        c = _make_center_settled(seg1=4)
        ac = classify_center_practical_newchan(c, 0, segs, last_price=20.0)
        assert ac.is_alive is True
        assert ac.regime == Regime.EVENT_ANCHOR_FIRST_PULLBACK
        assert ac.anchors.event_seen_pullback is True
        assert ac.anchors.event_pullback_settled is False

    def test_touch_core_pullback(self):
        """离开后有段触碰核 → 事件锚。"""
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            _seg(6, 8, 30, 40, "down", 19.0, 13.0),
            _seg(8, 10, 40, 50, "up",  17.0, 14.0),
            _seg(10, 12, 50, 60, "up",  25.0, 20.0),   # exit ABOVE
            # seg6: same dir as exit but overlaps core [12,18]
            _seg(12, 14, 60, 70, "up",  19.0, 15.0),
        ]
        c = _make_center_settled(seg1=4)
        ac = classify_center_practical_newchan(c, 0, segs, last_price=19.0)
        assert ac.is_alive is True
        assert ac.regime == Regime.EVENT_ANCHOR_FIRST_PULLBACK


# =====================================================================
# E) DEAD_NEGATION_SETTLED — 回抽后再创新高/新低
# =====================================================================

class TestDeadNegationSettled:

    def test_pullback_then_new_high(self):
        """离开 ABOVE → 回抽 → 再 up 创新高 → 死亡。"""
        segs = _make_segments(8)
        # seg5=exit ABOVE(h=25), seg6=down(回抽), seg7=up h=27>25 → settled
        c = _make_center_settled(seg1=4)
        ac = classify_center_practical_newchan(c, 0, segs, last_price=26.0)
        assert ac.is_alive is False
        assert ac.regime == Regime.DEAD_NEGATION_SETTLED
        assert ac.anchors.event_pullback_settled is True

    def test_no_exit_segment(self):
        """seg1+1 越界 → 无离开段 → 死亡。"""
        # 3 段，center 覆盖 seg0..seg2，exit_idx=3 越界
        # seg2 不与核 [12,18] 重叠 → 不会被 IN_CORE 拦截
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   11.0, 9.0),   # h=11 < core_low=12 → 无重叠
        ]
        c = _center(0, 2, low=12.0, high=18.0, kind="settled", sustain=2)
        ac = classify_center_practical_newchan(c, 0, segs, last_price=25.0)
        assert ac.is_alive is False
        assert ac.regime == Regime.DEAD_NEGATION_SETTLED
        assert ac.anchors.death_reason == "no_exit_segment"


# =====================================================================
# F) 对象驱动：无超时，离开后多段无对象否定仍保持alive
# =====================================================================

class TestNoTimeout:
    """005a/005b: 对象否定对象——不允许超时否定，只有对象事件能杀死中枢。"""

    def test_many_segments_no_object_negation_stays_alive(self):
        """离开后经过多段，但无回抽+再确认 → 中枢仍活。"""
        # 构造离开后多段同向、无回抽的场景
        segs_no_negation = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),    # seg0
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),    # seg1
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),    # seg2
            _seg(6, 8, 30, 40, "down", 19.0, 13.0),    # seg3
            _seg(8, 10, 40, 50, "up",  17.0, 14.0),    # seg4
            _seg(10, 12, 50, 60, "up", 25.0, 20.0),    # seg5 exit ABOVE
            # seg6..seg9: 全部同向 up，无回抽，无创新高
            _seg(12, 14, 60, 70, "up", 24.0, 21.0),    # seg6 同向，h=24<25
            _seg(14, 16, 70, 80, "up", 23.0, 20.0),    # seg7 同向
            _seg(16, 18, 80, 90, "up", 24.5, 19.0),    # seg8 同向
            _seg(18, 20, 90, 100, "up", 24.8, 20.0),   # seg9 同向
        ]
        c2 = _make_center_settled(seg1=4)
        ac = classify_center_practical_newchan(c2, 0, segs_no_negation, last_price=24.0)
        # 无反向段 → 无回抽 → 仍在 RUN_ANCHOR_POST_EXIT
        assert ac.is_alive is True
        assert ac.regime == Regime.RUN_ANCHOR_POST_EXIT
        assert ac.anchors.death_reason is None


# =====================================================================
# G) DEAD_NOT_SETTLED — candidate center
# =====================================================================

class TestDeadNotSettled:

    def test_candidate_center(self):
        segs = _make_segments(5)
        c = _center(0, 2, low=12.0, high=18.0, kind="candidate", sustain=0)
        ac = classify_center_practical_newchan(c, 0, segs, last_price=15.0)
        assert ac.is_alive is False
        assert ac.regime == Regime.DEAD_NOT_SETTLED
        assert ac.anchors.death_reason == "not_settled"


# =====================================================================
# H) select_lstar_newchan
# =====================================================================

class TestSelectLStar:

    def test_single_level_alive(self):
        segs = _make_segments(6)
        c = _make_center_settled(seg1=4)
        lv = LevelView(level=0, segments=segs, centers=[c])
        result = select_lstar_newchan([lv], last_price=15.0)
        assert result is not None
        assert result.level == 0
        assert result.regime == Regime.SETTLE_ANCHOR_IN_CORE

    def test_no_alive_returns_none(self):
        segs = _make_segments(5)
        c = _center(0, 2, low=12.0, high=18.0, kind="candidate", sustain=0)
        lv = LevelView(level=0, segments=segs, centers=[c])
        result = select_lstar_newchan([lv], last_price=15.0)
        assert result is None

    def test_higher_level_preferred(self):
        """有两个 level，优先取更高 level。"""
        segs0 = _make_segments(6)
        c0 = _make_center_settled(seg1=4)
        lv0 = LevelView(level=0, segments=segs0, centers=[c0])

        segs1 = _make_segments(6)
        c1 = _make_center_settled(seg1=4)
        lv1 = LevelView(level=1, segments=segs1, centers=[c1])

        result = select_lstar_newchan([lv0, lv1], last_price=15.0)
        assert result is not None
        assert result.level == 1

    def test_multiple_alive_picks_newest(self):
        """同一 level 多个 alive center → 取 seg1 最大。"""
        segs = _make_segments(10)
        c0 = _center(0, 4, low=12.0, high=18.0, kind="settled", sustain=2)
        c1 = _center(5, 9, low=14.0, high=19.0, kind="settled", sustain=2)
        # c0 的 seg1=4 价格 in core → alive
        # c1 的 seg1=9 价格 in core → alive
        # 但 c1 的 seg1=9 > c0 的 seg1=4 → c1 优先
        lv = LevelView(level=0, segments=segs, centers=[c0, c1])
        result = select_lstar_newchan([lv], last_price=15.0)
        assert result is not None
        assert result.center_idx == 1  # c1


# =====================================================================
# I) assert_single_lstar
# =====================================================================

class TestAssertSingleLStar:

    def test_valid_pass(self):
        segs = _make_segments(6)
        c = _make_center_settled(seg1=4)
        lv = LevelView(level=0, segments=segs, centers=[c])
        result = assert_single_lstar([lv], 15.0)
        assert result.ok is True

    def test_empty_pass(self):
        result = assert_single_lstar([], 15.0)
        assert result.ok is True

    def test_duplicate_alive_center_fails(self):
        """同 (seg0,seg1) 出现两次 alive → 断言失败。"""
        segs = _make_segments(6)
        # 两个完全相同的 settled center
        c0 = _make_center_settled(seg1=4)
        c1 = _make_center_settled(seg1=4)
        lv = LevelView(level=0, segments=segs, centers=[c0, c1])
        result = assert_single_lstar([lv], 15.0)
        assert result.ok is False
        assert "duplicate" in result.message


# =====================================================================
# 边界 / overlap 工具
# =====================================================================

class TestOverlapUtil:

    def test_overlap_true(self):
        assert overlap(10, 20, 15, 25) is True

    def test_overlap_false(self):
        assert overlap(10, 20, 25, 30) is False

    def test_overlap_touching_false(self):
        """端点相触不算重叠（strict <）。"""
        assert overlap(10, 20, 20, 30) is False
