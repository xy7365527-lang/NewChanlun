"""走势类型实例构造 — 单元测试

覆盖 docs/chan_spec.md §8 全部规则：
  A) 单中枢 → 盘整对象 (kind="consolidation")
  B) ≥2 同向中枢 → 趋势对象 (kind="trend")
  C) 方向变化 → 切分为多个实例
  D) 连续性：instances[i].seg1 == instances[i+1].seg0
  E) confirmed：最后一个 False，其余 True
  F) 无 settled 中枢 → 空列表
  G) 去除 trend_leg：不再有该概念
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v0 import Segment
from newchan.a_center_v0 import Center
from newchan.a_trendtype_v0 import (
    TrendTypeInstance,
    _centers_relation,
    trend_instances_from_centers,
    _centers_same_direction,
    _group_centers_by_direction,
)


# ── helpers ──

def _seg(s0_i0: int, s0_i1: int, direction: str,
         high: float, low: float) -> Segment:
    """快速构造 Segment（s0/s1 不重要，用 i0 当 s0）。"""
    return Segment(
        s0=s0_i0, s1=s0_i1, i0=s0_i0 * 5, i1=s0_i1 * 5,
        direction=direction, high=high, low=low, confirmed=True,
    )


def _center(seg0: int, seg1: int, low: float, high: float,
            kind: str = "settled", sustain: int = 2,
            gg: float = 0.0, dd: float = 0.0) -> Center:
    return Center(
        seg0=seg0, seg1=seg1, low=low, high=high,
        kind=kind, confirmed=True, sustain=sustain,
        gg=gg, dd=dd,
    )


def _make_segments(n: int = 12) -> list[Segment]:
    """构造 n 段，交替 up/down，价格在 [5,25] 区间波动。"""
    segs = []
    for i in range(n):
        d = "up" if i % 2 == 0 else "down"
        h = 20.0 + (i % 3) * 2
        l = 8.0 + (i % 3) * 2
        segs.append(Segment(
            s0=i, s1=i, i0=i * 5, i1=(i + 1) * 5,
            direction=d, high=h, low=l, confirmed=True,
        ))
    return segs


# =====================================================================
# 方向判定
# =====================================================================

class TestDirectionDetection:
    """§8.2 中枢方向判定。"""

    def test_up_direction(self):
        c1 = _center(0, 2, low=10, high=18)
        c2 = _center(4, 6, low=12, high=20)
        assert _centers_same_direction(c1, c2) == "up"

    def test_down_direction(self):
        c1 = _center(0, 2, low=12, high=20)
        c2 = _center(4, 6, low=10, high=18)
        assert _centers_same_direction(c1, c2) == "down"

    def test_divergent_not_same(self):
        """high 升 low 降 → 不同向。"""
        c1 = _center(0, 2, low=12, high=18)
        c2 = _center(4, 6, low=10, high=20)
        assert _centers_same_direction(c1, c2) is None

    def test_convergent_not_same(self):
        """high 降 low 升 → 不同向。"""
        c1 = _center(0, 2, low=10, high=20)
        c2 = _center(4, 6, low=12, high=18)
        assert _centers_same_direction(c1, c2) is None


# =====================================================================
# A0) 前后中枢关系（GG/DD 判定，思维导图 #32-35）
# =====================================================================

class TestCenterRelation:
    """前后同级别中枢关系判定，使用 GG/DD。"""

    def test_up_relation(self):
        """后DD > 前GG → 上涨及其延续。"""
        c1 = _center(0, 2, low=10, high=18, gg=20, dd=8)
        c2 = _center(4, 6, low=22, high=28, gg=30, dd=21)
        assert _centers_relation(c1, c2) == "up"

    def test_down_relation(self):
        """后GG < 前DD → 下跌及其延续。"""
        c1 = _center(0, 2, low=20, high=28, gg=30, dd=18)
        c2 = _center(4, 6, low=10, high=15, gg=17, dd=8)
        assert _centers_relation(c1, c2) == "down"

    def test_higher_center_zg_lt_zd(self):
        """后ZG < 前ZD 且 后GG >= 前DD → 形成高级别中枢。"""
        # 前：ZD=20, ZG=28, DD=18
        # 后：ZD=10, ZG=15, GG=19 (>=18)
        c1 = _center(0, 2, low=20, high=28, gg=30, dd=18)
        c2 = _center(4, 6, low=10, high=15, gg=19, dd=8)
        assert _centers_relation(c1, c2) == "higher_center"

    def test_higher_center_zd_gt_zg(self):
        """后ZD > 前ZG 且 后DD <= 前GG → 形成高级别中枢。"""
        # 前：ZD=10, ZG=18, GG=20
        # 后：ZD=19, ZG=25, DD=15 (<=20)
        c1 = _center(0, 2, low=10, high=18, gg=20, dd=8)
        c2 = _center(4, 6, low=19, high=25, gg=28, dd=15)
        assert _centers_relation(c1, c2) == "higher_center"

    def test_none_relation(self):
        """不满足任何条件 → none。"""
        c1 = _center(0, 2, low=10, high=20, gg=22, dd=8)
        c2 = _center(4, 6, low=12, high=18, gg=19, dd=11)
        assert _centers_relation(c1, c2) == "none"

    def test_backward_compat_same_direction(self):
        """_centers_same_direction 仍返回 up/down/None。"""
        c1 = _center(0, 2, low=10, high=18, gg=20, dd=8)
        c2 = _center(4, 6, low=22, high=28, gg=30, dd=21)
        assert _centers_same_direction(c1, c2) == "up"

    def test_backward_compat_higher_center_returns_none(self):
        """higher_center 在旧接口中返回 None。"""
        c1 = _center(0, 2, low=20, high=28, gg=30, dd=18)
        c2 = _center(4, 6, low=10, high=15, gg=19, dd=8)
        assert _centers_same_direction(c1, c2) is None

    def test_fallback_when_gg_dd_zero(self):
        """GG/DD 为 0（旧 Center 无新字段）时 fallback 到 ZG/ZD 弱化判定。"""
        c1 = _center(0, 2, low=10, high=18)  # gg=0, dd=0 → fallback
        c2 = _center(4, 6, low=12, high=20)  # gg=0, dd=0 → fallback
        # fallback 用 ZG/ZD：ZG2(20)>ZG1(18) AND ZD2(12)>ZD1(10) → "up"
        rel = _centers_relation(c1, c2)
        assert rel == "up"
        assert _centers_same_direction(c1, c2) == "up"


# =====================================================================
# A) 单中枢 → 盘整
# =====================================================================

class TestConsolidation:
    """单个 settled 中枢 → 盘整对象。"""

    def test_single_center_produces_consolidation(self):
        segs = _make_segments(6)
        centers = [_center(1, 3, low=10, high=18)]
        result = trend_instances_from_centers(segs, centers)
        assert len(result) == 1
        assert result[0].kind == "consolidation"

    def test_consolidation_covers_all_segments(self):
        segs = _make_segments(6)
        centers = [_center(1, 3, low=10, high=18)]
        result = trend_instances_from_centers(segs, centers)
        assert result[0].seg0 == 0
        assert result[0].seg1 == 5  # last segment

    def test_consolidation_center_indices(self):
        segs = _make_segments(6)
        centers = [_center(1, 3, low=10, high=18)]
        result = trend_instances_from_centers(segs, centers)
        assert result[0].center_indices == (0,)


# =====================================================================
# B) ≥2 同向中枢 → 趋势
# =====================================================================

class TestTrend:
    """两个同向中枢 → 趋势对象。"""

    def test_two_up_centers_produce_trend(self):
        segs = _make_segments(10)
        centers = [
            _center(1, 3, low=10, high=18),   # ZD=10, ZG=18
            _center(5, 7, low=12, high=20),   # ZD=12, ZG=20 → up
        ]
        result = trend_instances_from_centers(segs, centers)
        assert len(result) == 1
        assert result[0].kind == "trend"
        assert result[0].direction == "up"

    def test_two_down_centers_produce_trend(self):
        segs = _make_segments(10)
        centers = [
            _center(1, 3, low=12, high=20),
            _center(5, 7, low=10, high=18),   # → down
        ]
        result = trend_instances_from_centers(segs, centers)
        assert len(result) == 1
        assert result[0].kind == "trend"
        assert result[0].direction == "down"

    def test_trend_contains_both_centers(self):
        segs = _make_segments(10)
        centers = [
            _center(1, 3, low=10, high=18),
            _center(5, 7, low=12, high=20),
        ]
        result = trend_instances_from_centers(segs, centers)
        assert result[0].center_indices == (0, 1)

    def test_three_same_dir_centers(self):
        """三个连续同向中枢 → 一个趋势对象。"""
        segs = _make_segments(12)
        centers = [
            _center(0, 2, low=10, high=18),
            _center(4, 6, low=12, high=20),
            _center(8, 10, low=14, high=22),
        ]
        result = trend_instances_from_centers(segs, centers)
        assert len(result) == 1
        assert result[0].kind == "trend"
        assert result[0].center_indices == (0, 1, 2)


# =====================================================================
# C) 方向变化 → 切分
# =====================================================================

class TestDirectionChange:
    """中枢方向变化 → 切分为多个走势类型实例。"""

    def test_up_then_down(self):
        """先上后下 → 趋势(up) + 盘整(down center alone)。"""
        segs = _make_segments(12)
        centers = [
            _center(0, 2, low=10, high=18),
            _center(4, 6, low=12, high=20),    # up trend
            _center(8, 10, low=8, high=16),    # down → new instance
        ]
        result = trend_instances_from_centers(segs, centers)
        assert len(result) == 2
        assert result[0].kind == "trend"
        assert result[0].direction == "up"
        assert result[1].kind == "consolidation"

    def test_alternating_directions(self):
        """三个中枢交替方向 → 三个盘整。"""
        segs = _make_segments(12)
        centers = [
            _center(0, 2, low=10, high=18),
            _center(4, 6, low=14, high=16),    # high↓ low↑ → convergent, not same as C0
            _center(8, 10, low=10, high=20),   # high↑ low↓ → divergent, not same as C1
        ]
        result = trend_instances_from_centers(segs, centers)
        assert len(result) == 3
        assert all(inst.kind == "consolidation" for inst in result)

    def test_adjacent_same_direction_trends_are_merged(self):
        """若出现 trend(up)+trend(up) 紧邻，应合并为一个极大趋势对象。"""
        segs = _make_segments(20)
        centers = [
            _center(0, 2, low=10, high=18),   # c0
            _center(4, 6, low=12, high=20),   # c1: c0->c1 up
            _center(8, 10, low=11, high=21),  # c2: c1->c2 非同向（高升低降）
            _center(12, 14, low=13, high=23), # c3: c2->c3 up
        ]
        result = trend_instances_from_centers(segs, centers)
        assert len(result) == 1
        assert result[0].kind == "trend"
        assert result[0].direction == "up"
        assert result[0].seg0 == 0
        assert result[0].seg1 == len(segs) - 1


# =====================================================================
# D) 连续性
# =====================================================================

class TestContinuity:
    """走势类型实例之间首尾相连。"""

    def test_instances_stitched(self):
        """instances[i].seg1 == instances[i+1].seg0。"""
        segs = _make_segments(12)
        centers = [
            _center(0, 2, low=10, high=18),
            _center(4, 6, low=12, high=20),
            _center(8, 10, low=8, high=16),
        ]
        result = trend_instances_from_centers(segs, centers)
        assert len(result) >= 2
        for i in range(len(result) - 1):
            assert result[i].seg1 == result[i + 1].seg0, (
                f"Gap: instances[{i}].seg1={result[i].seg1} "
                f"!= instances[{i+1}].seg0={result[i+1].seg0}"
            )

    def test_first_starts_at_zero(self):
        segs = _make_segments(10)
        centers = [_center(2, 4, low=10, high=18)]
        result = trend_instances_from_centers(segs, centers)
        assert result[0].seg0 == 0

    def test_last_ends_at_last_segment(self):
        segs = _make_segments(10)
        centers = [_center(2, 4, low=10, high=18)]
        result = trend_instances_from_centers(segs, centers)
        assert result[-1].seg1 == 9  # len(segs) - 1


# =====================================================================
# E) confirmed 语义
# =====================================================================

class TestConfirmed:
    """最后一个实例 confirmed=False，其余 True。"""

    def test_single_instance_unconfirmed(self):
        segs = _make_segments(6)
        centers = [_center(1, 3, low=10, high=18)]
        result = trend_instances_from_centers(segs, centers)
        assert result[0].confirmed is False

    def test_two_instances_first_confirmed(self):
        segs = _make_segments(12)
        centers = [
            _center(0, 2, low=10, high=18),
            _center(4, 6, low=12, high=20),
            _center(8, 10, low=8, high=16),
        ]
        result = trend_instances_from_centers(segs, centers)
        assert len(result) >= 2
        assert result[0].confirmed is True
        assert result[-1].confirmed is False


# =====================================================================
# F) 边界情况
# =====================================================================

class TestEdgeCases:
    """边界条件。"""

    def test_no_centers(self):
        segs = _make_segments(6)
        assert trend_instances_from_centers(segs, []) == []

    def test_no_segments(self):
        centers = [_center(0, 2, low=10, high=18)]
        assert trend_instances_from_centers([], centers) == []

    def test_only_candidate_centers(self):
        """只有 candidate 中枢 → 空列表。"""
        segs = _make_segments(6)
        centers = [_center(1, 3, low=10, high=18, kind="candidate")]
        assert trend_instances_from_centers(segs, centers) == []

    def test_high_low_covers_range(self):
        """实例的 high/low 覆盖所有包含段的极值。"""
        segs = _make_segments(6)
        centers = [_center(1, 3, low=10, high=18)]
        result = trend_instances_from_centers(segs, centers)
        seg_slice = segs[result[0].seg0 : result[0].seg1 + 1]
        assert result[0].high == max(s.high for s in seg_slice)
        assert result[0].low == min(s.low for s in seg_slice)


# =====================================================================
# G) 不再有 trend_leg
# =====================================================================

class TestNoTrendLeg:
    """确认 trend_leg 概念已被删除。"""

    def test_no_trend_leg_kind(self):
        """任何输出都不会出现 'trend_leg'。"""
        segs = _make_segments(12)
        centers = [
            _center(0, 2, low=10, high=18),
            _center(4, 6, low=12, high=20),
            _center(8, 10, low=8, high=16),
        ]
        result = trend_instances_from_centers(segs, centers)
        for inst in result:
            assert inst.kind in ("trend", "consolidation")
            assert inst.kind != "trend_leg"
