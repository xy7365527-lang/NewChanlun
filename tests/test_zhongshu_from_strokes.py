"""笔中枢 (zhongshu_from_strokes) — Golden 用例

笔中枢路径（[新缠论:选择]，谱系 settled/525）：把笔作为"最低不可分解级别的单位"，
三笔重叠构成笔中枢。算法与 zhongshu_from_segments 共享 `_scan_zhongshu`，差异仅在：
  - 过滤：confirmed（笔无 settled/candidate 二态）
  - 时间锚：i0/i1（merged idx），非 segment 的 s0/s1（stroke idx）

覆盖：
  1. 三笔重叠 → 笔中枢成立
  2. 三笔无重叠 → 无中枢
  3. 延伸（第四笔重叠）
  4. 突破 + 方向
  5. 未确认笔（最后一笔 confirmed=False）被排除
  6. first_seg_s0 / last_seg_s1 取笔 i0/i1
  7. 与 zhongshu_from_segments 算法一致性（同区间数据 → 同 zd/zg/settled）
"""

from __future__ import annotations

from newchan.a_stroke import Stroke
from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import (
    Zhongshu,
    zhongshu_from_segments,
    zhongshu_from_strokes,
)


# ── helpers ──

def _stroke(idx: int, direction: str, high: float, low: float,
            confirmed: bool = True) -> Stroke:
    """快速构造 Stroke。i0/i1 用 idx*5, idx*5+5 模拟 merged idx。"""
    i0 = idx * 5
    i1 = idx * 5 + 5
    # p0/p1 仅占位（中枢算法只用 high/low），按方向给端点价
    if direction == "up":
        p0, p1 = low, high
    else:
        p0, p1 = high, low
    return Stroke(
        i0=i0, i1=i1, direction=direction,
        high=high, low=low, p0=p0, p1=p1,
        confirmed=confirmed,
    )


# =====================================================================
# 1) 三笔重叠 → 笔中枢成立
# =====================================================================

class TestThreeStrokeOverlap:
    def test_basic_overlap(self):
        strokes = [
            _stroke(0, "up",   20, 10),   # [10, 20]
            _stroke(1, "down", 18, 8),    # [8, 18]
            _stroke(2, "up",   22, 12),   # [12, 22]
        ]
        result = zhongshu_from_strokes(strokes)
        assert len(result) == 1
        zs = result[0]
        # zd = max(10, 8, 12) = 12, zg = min(20, 18, 22) = 18
        assert zs.zd == 12
        assert zs.zg == 18
        assert zs.seg_start == 0
        assert zs.seg_end == 2
        assert zs.seg_count == 3
        assert not zs.settled


# =====================================================================
# 2) 三笔无重叠 → 无中枢
# =====================================================================

class TestThreeStrokeNoOverlap:
    def test_no_overlap(self):
        strokes = [
            _stroke(0, "up",   10, 5),
            _stroke(1, "down", 20, 15),
            _stroke(2, "up",   30, 25),
        ]
        assert zhongshu_from_strokes(strokes) == []


# =====================================================================
# 3) 延伸
# =====================================================================

class TestExtension:
    def test_fourth_stroke_extends(self):
        strokes = [
            _stroke(0, "up",   20, 10),
            _stroke(1, "down", 18, 8),
            _stroke(2, "up",   22, 12),
            _stroke(3, "down", 19, 11),  # [11, 19] 与 [12, 18] 重叠
        ]
        result = zhongshu_from_strokes(strokes)
        assert len(result) == 1
        zs = result[0]
        assert zs.seg_count == 4
        assert zs.seg_end == 3
        assert zs.zd == 12 and zs.zg == 18  # 区间不变


# =====================================================================
# 4) 突破 + 方向
# =====================================================================

class TestBreak:
    def test_break_down(self):
        strokes = [
            _stroke(0, "up",   20, 10),
            _stroke(1, "down", 18, 8),
            _stroke(2, "up",   22, 12),
            _stroke(3, "down", 10, 2),   # high=10 < zd=12 → 突破 down
        ]
        result = zhongshu_from_strokes(strokes)
        assert len(result) == 1
        zs = result[0]
        assert zs.settled
        assert zs.break_seg == 3
        assert zs.break_direction == "down"

    def test_break_up(self):
        strokes = [
            _stroke(0, "up",   20, 10),
            _stroke(1, "down", 18, 8),
            _stroke(2, "up",   22, 12),
            _stroke(3, "up",   30, 20),  # low=20 > zg=18 → 突破 up
        ]
        result = zhongshu_from_strokes(strokes)
        assert result[0].break_direction == "up"


# =====================================================================
# 5) 未确认笔被排除
# =====================================================================

class TestUnconfirmedExcluded:
    def test_last_unconfirmed_stroke_ignored(self):
        strokes = [
            _stroke(0, "up",   20, 10, confirmed=True),
            _stroke(1, "down", 18, 8, confirmed=True),
            _stroke(2, "up",   22, 12, confirmed=False),  # 延伸中
        ]
        # 只有 2 笔已确认 → 不足 3 笔
        assert zhongshu_from_strokes(strokes) == []


# =====================================================================
# 6) 时间锚取笔 i0/i1
# =====================================================================

class TestStrokeAnchors:
    def test_anchor_uses_merged_idx(self):
        strokes = [
            _stroke(0, "up",   20, 10),   # i0=0,  i1=5
            _stroke(1, "down", 18, 8),    # i0=5,  i1=10
            _stroke(2, "up",   22, 12),   # i0=10, i1=15
        ]
        result = zhongshu_from_strokes(strokes)
        assert len(result) == 1
        assert result[0].first_seg_s0 == 0    # 第一笔 i0
        assert result[0].last_seg_s1 == 15    # 第三笔 i1


# =====================================================================
# 7) 算法与线段中枢一致性
# =====================================================================

class TestAlgorithmConsistencyWithSegments:
    """相同价格区间序列下，笔路径与线段路径产出相同的 zd/zg/settled/break。"""

    def test_same_ranges_same_centers(self):
        ranges = [
            ("up", 20, 10),
            ("down", 18, 8),
            ("up", 22, 12),
            ("down", 10, 2),  # 突破
        ]
        strokes = [_stroke(i, d, h, lo) for i, (d, h, lo) in enumerate(ranges)]
        segs = [
            Segment(
                s0=i, s1=i, i0=i, i1=i, direction=d,
                high=h, low=lo, confirmed=True, kind="settled",
            )
            for i, (d, h, lo) in enumerate(ranges)
        ]
        zs_stroke = zhongshu_from_strokes(strokes)
        zs_seg = zhongshu_from_segments(segs)
        assert len(zs_stroke) == len(zs_seg) == 1
        a, b = zs_stroke[0], zs_seg[0]
        assert (a.zd, a.zg, a.settled, a.break_seg, a.break_direction) == \
               (b.zd, b.zg, b.settled, b.break_seg, b.break_direction)
