"""beichi #5: divergences_in_bar_range 单元测试。

区间套的单级别检测入口 — 在指定 merged bar 索引范围内检测背驰。

测试场景来源：R23-C 接口设计（agent a9cc516）。

概念溯源：[旧缠论] 第27课 区间套定理
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import Zhongshu
from newchan.a_move_v1 import Move
from newchan.a_divergence_v1 import (
    divergences_from_moves_v1,
    divergences_in_bar_range,
)


# ── helpers ──────────────────────────────────────


def _seg(s0: int, s1: int, i0: int, i1: int, d: str,
         h: float, l: float, confirmed: bool = True) -> Segment:
    return Segment(s0=s0, s1=s1, i0=i0, i1=i1, direction=d,
                   high=h, low=l, confirmed=confirmed)


def _make_uptrend_v1():
    """上涨趋势：2中枢，C段力度弱 → 应检出背驰。

    seg[0-2]  → zs0 [ZD=12,ZG=18]  i0=0..30
    seg[3-6]  → 过渡段              i0=30..70
    seg[7-9]  → zs1 [ZD=28,ZG=32]  i0=70..100
    seg[10]   → C段 (2点振幅)       i0=100..110

    Move: seg[0..10], i0=0, i1=110
    """
    segments = [
        _seg(0, 0, 0, 10, "up", 20, 10),
        _seg(1, 1, 10, 20, "down", 20, 12),
        _seg(2, 2, 20, 30, "up", 18, 12),
        _seg(3, 3, 30, 40, "down", 11, 8),
        _seg(4, 4, 40, 50, "up", 11, 8),
        _seg(5, 5, 50, 60, "down", 25, 20),
        _seg(6, 6, 60, 70, "up", 25, 20),
        _seg(7, 7, 70, 80, "down", 32, 26),
        _seg(8, 8, 80, 90, "up", 34, 26),
        _seg(9, 9, 90, 100, "down", 34, 28),
        _seg(10, 10, 100, 110, "up", 35, 33),
    ]

    zhongshus = [
        Zhongshu(zd=12.0, zg=18.0, seg_start=0, seg_end=2, seg_count=3,
                 settled=True, break_seg=3, break_direction="up",
                 first_seg_s0=0, last_seg_s1=2, gg=20.0, dd=10.0),
        Zhongshu(zd=28.0, zg=32.0, seg_start=7, seg_end=9, seg_count=3,
                 settled=True, break_seg=10, break_direction="up",
                 first_seg_s0=7, last_seg_s1=9, gg=34.0, dd=26.0),
    ]

    moves = [
        Move(kind="trend", direction="up", seg_start=0, seg_end=10,
             zs_start=0, zs_end=1, zs_count=2, settled=False,
             high=32.0, low=12.0, first_seg_s0=0, last_seg_s1=10),
    ]

    return segments, zhongshus, moves


def _make_two_moves_v1():
    """两个独立 Move，方便测试过滤。

    Move 0 (趋势上涨): seg[0..10], bar range [0, 110]  — 有背驰
    Move 1 (趋势下跌): seg[11..21], bar range [110, 220] — 有背驰
    """
    segs_up = [
        _seg(0, 0, 0, 10, "up", 20, 10),
        _seg(1, 1, 10, 20, "down", 20, 12),
        _seg(2, 2, 20, 30, "up", 18, 12),
        _seg(3, 3, 30, 40, "down", 11, 8),
        _seg(4, 4, 40, 50, "up", 11, 8),
        _seg(5, 5, 50, 60, "down", 25, 20),
        _seg(6, 6, 60, 70, "up", 25, 20),
        _seg(7, 7, 70, 80, "down", 32, 26),
        _seg(8, 8, 80, 90, "up", 34, 26),
        _seg(9, 9, 90, 100, "down", 34, 28),
        _seg(10, 10, 100, 110, "up", 35, 33),
    ]

    segs_down = [
        _seg(11, 11, 110, 120, "down", 95, 80),
        _seg(12, 12, 120, 130, "up", 95, 82),
        _seg(13, 13, 130, 140, "down", 90, 80),
        _seg(14, 14, 140, 150, "up", 82, 75),
        _seg(15, 15, 150, 160, "down", 75, 55),
        _seg(16, 16, 160, 170, "up", 65, 55),
        _seg(17, 17, 170, 180, "down", 65, 50),
        _seg(18, 18, 180, 190, "up", 60, 50),
        _seg(19, 19, 190, 200, "down", 60, 50),
        _seg(20, 20, 200, 210, "up", 52, 48),
        _seg(21, 21, 210, 220, "down", 50, 45),
    ]

    segments = segs_up + segs_down

    zhongshus = [
        # zs0 for Move 0
        Zhongshu(zd=12.0, zg=18.0, seg_start=0, seg_end=2, seg_count=3,
                 settled=True, break_seg=3, break_direction="up",
                 first_seg_s0=0, last_seg_s1=2, gg=20.0, dd=10.0),
        # zs1 for Move 0
        Zhongshu(zd=28.0, zg=32.0, seg_start=7, seg_end=9, seg_count=3,
                 settled=True, break_seg=10, break_direction="up",
                 first_seg_s0=7, last_seg_s1=9, gg=34.0, dd=26.0),
        # zs2 for Move 1
        Zhongshu(zd=82.0, zg=90.0, seg_start=11, seg_end=13, seg_count=3,
                 settled=True, break_seg=14, break_direction="down",
                 first_seg_s0=11, last_seg_s1=13, gg=95.0, dd=80.0),
        # zs3 for Move 1
        Zhongshu(zd=50.0, zg=60.0, seg_start=18, seg_end=20, seg_count=3,
                 settled=True, break_seg=21, break_direction="down",
                 first_seg_s0=18, last_seg_s1=20, gg=60.0, dd=50.0),
    ]

    moves = [
        Move(kind="trend", direction="up", seg_start=0, seg_end=10,
             zs_start=0, zs_end=1, zs_count=2, settled=True,
             high=32.0, low=12.0, first_seg_s0=0, last_seg_s1=10),
        Move(kind="trend", direction="down", seg_start=11, seg_end=21,
             zs_start=2, zs_end=3, zs_count=2, settled=False,
             high=90.0, low=50.0, first_seg_s0=11, last_seg_s1=21),
    ]

    return segments, zhongshus, moves


# ═══════════════════════════════════════════════════════════
# A. 基本功能测试
# ═══════════════════════════════════════════════════════════


class TestBarRangeBasic:
    """基本功能测试 (A1-A6)。"""

    def test_a1_empty_moves(self):
        """A1: 空 Move 列表 → 返回空。"""
        segs, zss, _ = _make_uptrend_v1()
        result = divergences_in_bar_range(segs, zss, [], 1, bar_range=(0, 999))
        assert result == []

    def test_a2_no_move_in_range(self):
        """A2: bar_range 与所有 Move 不重叠 → 返回空。"""
        segs, zss, mvs = _make_uptrend_v1()
        # Move 0 covers bar [0, 110], use range [500, 600]
        result = divergences_in_bar_range(segs, zss, mvs, 1, bar_range=(500, 600))
        assert result == []

    def test_a3_full_range_equivalence(self):
        """A3: bar_range 足够大 → 结果与全量检测一致。"""
        segs, zss, mvs = _make_uptrend_v1()
        full = divergences_from_moves_v1(segs, zss, mvs, level_id=1)
        ranged = divergences_in_bar_range(segs, zss, mvs, 1, bar_range=(0, 99999))
        assert len(ranged) == len(full)
        for a, b in zip(ranged, full):
            assert a.kind == b.kind
            assert a.direction == b.direction
            assert a.force_a == b.force_a
            assert a.force_c == b.force_c

    def test_a4_single_trend_move_in_range(self):
        """A4: 单个趋势 Move 落入范围 → 检出趋势背驰。"""
        segs, zss, mvs = _make_uptrend_v1()
        result = divergences_in_bar_range(segs, zss, mvs, 1, bar_range=(0, 110))
        trend_divs = [d for d in result if d.kind == "trend"]
        assert len(trend_divs) >= 1
        assert trend_divs[0].direction == "top"

    def test_a6_move_in_range_no_divergence(self):
        """A6: Move 落入范围但力度不满足 → 返回空。"""
        # 构造一个C段力度强于A段的场景
        segments = [
            _seg(0, 0, 0, 10, "up", 20, 10),
            _seg(1, 1, 10, 20, "down", 20, 12),
            _seg(2, 2, 20, 30, "up", 18, 12),
            _seg(3, 3, 30, 40, "down", 11, 8),
            _seg(4, 4, 40, 50, "up", 11, 8),
            _seg(5, 5, 50, 60, "down", 25, 20),
            _seg(6, 6, 60, 70, "up", 25, 20),
            _seg(7, 7, 70, 80, "down", 32, 26),
            _seg(8, 8, 80, 90, "up", 34, 26),
            _seg(9, 9, 90, 100, "down", 34, 28),
            # C段力度大于A段: 振幅50点 vs A段约20点
            _seg(10, 10, 100, 150, "up", 90, 33),
        ]
        zhongshus = [
            Zhongshu(zd=12.0, zg=18.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="up",
                     first_seg_s0=0, last_seg_s1=2, gg=20.0, dd=10.0),
            Zhongshu(zd=28.0, zg=32.0, seg_start=7, seg_end=9, seg_count=3,
                     settled=True, break_seg=10, break_direction="up",
                     first_seg_s0=7, last_seg_s1=9, gg=34.0, dd=26.0),
        ]
        moves = [
            Move(kind="trend", direction="up", seg_start=0, seg_end=10,
                 zs_start=0, zs_end=1, zs_count=2, settled=False,
                 high=32.0, low=12.0, first_seg_s0=0, last_seg_s1=10),
        ]
        result = divergences_in_bar_range(segments, zhongshus, moves, 1,
                                          bar_range=(0, 150))
        assert result == []


# ═══════════════════════════════════════════════════════════
# B. 边界范围测试
# ═══════════════════════════════════════════════════════════


class TestBarRangeBoundary:
    """边界范围测试 (B1-B5)。"""

    def test_b1_move_exceeds_left(self):
        """B1: Move 左侧超出 bar_range → 被排除。"""
        segs, zss, mvs = _make_uptrend_v1()
        # Move covers [0, 110], bar_range starts at 50 → Move left extends beyond
        result = divergences_in_bar_range(segs, zss, mvs, 1, bar_range=(50, 200))
        assert result == []

    def test_b2_move_exceeds_right(self):
        """B2: Move 右侧超出 bar_range → 被排除。"""
        segs, zss, mvs = _make_uptrend_v1()
        # Move covers [0, 110], bar_range ends at 80 → Move right extends beyond
        result = divergences_in_bar_range(segs, zss, mvs, 1, bar_range=(0, 80))
        assert result == []

    def test_b3_move_exactly_equals_range(self):
        """B3: Move 恰好等于 bar_range 边界 → 被包含。"""
        segs, zss, mvs = _make_uptrend_v1()
        result = divergences_in_bar_range(segs, zss, mvs, 1, bar_range=(0, 110))
        assert len(result) >= 1

    def test_b5_inverted_range(self):
        """B5: bar_range 反转 (start > end) → 返回空。"""
        segs, zss, mvs = _make_uptrend_v1()
        result = divergences_in_bar_range(segs, zss, mvs, 1, bar_range=(200, 100))
        assert result == []


# ═══════════════════════════════════════════════════════════
# C. 过滤精确性测试
# ═══════════════════════════════════════════════════════════


class TestBarRangeFiltering:
    """过滤精确性测试 (C1-C2)。"""

    def test_c1_multiple_moves_partial_in_range(self):
        """C1: 3个Move中只有1个完全落入范围。"""
        segs, zss, mvs = _make_two_moves_v1()
        # 全量应有2个背驰
        full = divergences_from_moves_v1(segs, zss, mvs, level_id=1)
        assert len(full) >= 2, f"全量应有≥2个背驰，实际{len(full)}"

        # 只包含 Move 0 (bar [0, 110])
        result = divergences_in_bar_range(segs, zss, mvs, 1, bar_range=(0, 110))
        assert len(result) == 1
        assert result[0].direction == "top"  # 上涨背驰

    def test_c2_only_second_move_in_range(self):
        """C2: 只有第二个 Move 在范围内。"""
        segs, zss, mvs = _make_two_moves_v1()
        # 只包含 Move 1 (bar [110, 220])
        result = divergences_in_bar_range(segs, zss, mvs, 1, bar_range=(110, 220))
        assert len(result) == 1
        assert result[0].direction == "bottom"  # 下跌背驰


# ═══════════════════════════════════════════════════════════
# D. 与全量检测一致性测试
# ═══════════════════════════════════════════════════════════


class TestBarRangeConsistency:
    """与全量检测一致性 (D1-D2)。"""

    def test_d1_full_range_equivalence_two_moves(self):
        """D1: 全范围 → 结果与全量检测完全一致。"""
        segs, zss, mvs = _make_two_moves_v1()
        full = divergences_from_moves_v1(segs, zss, mvs, level_id=1)
        ranged = divergences_in_bar_range(segs, zss, mvs, 1, bar_range=(0, 99999))
        assert len(ranged) == len(full)

    def test_d2_ranged_subset_of_full(self):
        """D2: 范围限定结果 ⊆ 全量结果。"""
        segs, zss, mvs = _make_two_moves_v1()
        full = divergences_from_moves_v1(segs, zss, mvs, level_id=1)
        ranged = divergences_in_bar_range(segs, zss, mvs, 1, bar_range=(0, 110))
        # 范围限定的每个结果都应在全量中存在
        full_keys = {(d.kind, d.direction, d.seg_c_start, d.seg_c_end) for d in full}
        for d in ranged:
            assert (d.kind, d.direction, d.seg_c_start, d.seg_c_end) in full_keys


# ═══════════════════════════════════════════════════════════
# F. 纯函数性测试
# ═══════════════════════════════════════════════════════════


class TestBarRangePurity:
    """纯函数性测试 (F1-F2)。"""

    def test_f1_input_not_mutated(self):
        """F1: 调用前后输入对象未被修改。"""
        segs, zss, mvs = _make_uptrend_v1()
        segs_copy = list(segs)
        zss_copy = list(zss)
        mvs_copy = list(mvs)

        divergences_in_bar_range(segs, zss, mvs, 1, bar_range=(0, 110))

        assert segs == segs_copy
        assert zss == zss_copy
        assert mvs == mvs_copy

    def test_f2_idempotent(self):
        """F2: 对同一输入调用两次，返回结果完全相等。"""
        segs, zss, mvs = _make_uptrend_v1()
        r1 = divergences_in_bar_range(segs, zss, mvs, 1, bar_range=(0, 110))
        r2 = divergences_in_bar_range(segs, zss, mvs, 1, bar_range=(0, 110))
        assert len(r1) == len(r2)
        for a, b in zip(r1, r2):
            assert a == b
