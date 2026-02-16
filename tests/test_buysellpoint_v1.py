"""买卖点 v1 — 探索性单元测试（maimai 定义仍为生成态）

依赖：a_divergence_v1.py (v1), a_move_v1.py, a_zhongshu_v1.py
产出：a_buysellpoint_v1.py

测试项：
  A) Type 1 买点：下跌趋势背驰 → 1B
  B) Type 1 卖点：上涨趋势背驰 → 1S
  C) Type 3 买点：中枢突破后回试不跌破 ZG → 3B
  D) Type 3 卖点：中枢突破后回抽不升破 ZD → 3S
  E) Type 3 不成立：回试跌破 ZG
  F) 空输入
  G) 盘整背驰不产生 Type 1
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import Zhongshu
from newchan.a_move_v1 import Move
from newchan.a_divergence import Divergence
from newchan.a_buysellpoint_v1 import BuySellPoint, buysellpoints_from_level


# ── helpers ──

def _seg(s0: int, s1: int, i0: int, i1: int, d: str,
         h: float, l: float, confirmed: bool = True) -> Segment:
    return Segment(s0=s0, s1=s1, i0=i0, i1=i1, direction=d,
                   high=h, low=l, confirmed=confirmed)


# ── fixture: 下跌趋势背驰 → 1B ──

def _make_downtrend_with_divergence():
    """下跌趋势 + 趋势背驰 → 应产生 1B。

    seg0-2  → zhongshu0 [ZD=82,ZG=90], DD=80
    seg3-6  → A 段
    seg7-9  → zhongshu1 [ZD=50,ZG=60], DD=50
    seg10   → C 段（力度弱 → 背驰）

    趋势背驰: direction="bottom" → side="buy" → 1B
    """
    segments = [
        _seg(0, 0, 0, 10, "down", 95, 80),
        _seg(1, 1, 10, 20, "up", 95, 82),
        _seg(2, 2, 20, 30, "down", 90, 80),
        _seg(3, 3, 30, 40, "up", 82, 75),
        _seg(4, 4, 40, 50, "down", 75, 55),
        _seg(5, 5, 50, 60, "up", 65, 55),
        _seg(6, 6, 60, 70, "down", 65, 50),
        _seg(7, 7, 70, 80, "up", 60, 50),
        _seg(8, 8, 80, 90, "down", 60, 50),
        _seg(9, 9, 90, 100, "up", 52, 48),
        _seg(10, 10, 100, 110, "down", 50, 45),
    ]

    zhongshus = [
        Zhongshu(zd=82.0, zg=90.0, seg_start=0, seg_end=2, seg_count=3,
                 settled=True, break_seg=3, break_direction="down",
                 first_seg_s0=0, last_seg_s1=2, gg=95.0, dd=80.0),
        Zhongshu(zd=50.0, zg=60.0, seg_start=7, seg_end=9, seg_count=3,
                 settled=True, break_seg=10, break_direction="down",
                 first_seg_s0=7, last_seg_s1=9, gg=60.0, dd=50.0),
    ]

    moves = [
        Move(kind="trend", direction="down", seg_start=0, seg_end=10,
             zs_start=0, zs_end=1, zs_count=2, settled=False,
             high=90.0, low=50.0, first_seg_s0=0, last_seg_s1=10),
    ]

    divergences = [
        Divergence(kind="trend", direction="bottom", level_id=1,
                   seg_a_start=3, seg_a_end=6, seg_c_start=10, seg_c_end=10,
                   center_idx=1, force_a=500.0, force_c=50.0, confirmed=False),
    ]

    return segments, zhongshus, moves, divergences


# ── fixture: 上涨趋势背驰 → 1S ──

def _make_uptrend_with_divergence():
    """上涨趋势 + 趋势背驰 → 应产生 1S。"""
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

    divergences = [
        Divergence(kind="trend", direction="top", level_id=1,
                   seg_a_start=3, seg_a_end=6, seg_c_start=10, seg_c_end=10,
                   center_idx=1, force_a=500.0, force_c=20.0, confirmed=False),
    ]

    return segments, zhongshus, moves, divergences


# ── fixture: 中枢突破 + 回试 → 3B ──

def _make_type3_buy():
    """中枢向上突破后回试不跌破 ZG → 3B。

    seg0-2 → zhongshu [ZD=12, ZG=18]
    seg3   → 向上突破（break_direction="up"）
    seg4   → 回试，low=19 > ZG=18 → 3B 成立
    """
    segments = [
        _seg(0, 0, 0, 10, "up", 18, 12),
        _seg(1, 1, 10, 20, "down", 18, 12),
        _seg(2, 2, 20, 30, "up", 18, 12),
        _seg(3, 3, 30, 50, "up", 30, 19),     # 向上突破
        _seg(4, 4, 50, 60, "down", 25, 19),    # 回试: low=19 > ZG=18 ✓
    ]

    zhongshus = [
        Zhongshu(zd=12.0, zg=18.0, seg_start=0, seg_end=2, seg_count=3,
                 settled=True, break_seg=3, break_direction="up",
                 first_seg_s0=0, last_seg_s1=2, gg=18.0, dd=12.0),
    ]

    moves = [
        Move(kind="consolidation", direction="up", seg_start=0, seg_end=4,
             zs_start=0, zs_end=0, zs_count=1, settled=False,
             high=18.0, low=12.0, first_seg_s0=0, last_seg_s1=4),
    ]

    return segments, zhongshus, moves


# ── fixture: 中枢突破 + 回试跌破 ZG → 3B 不成立 ──

def _make_type3_buy_fail():
    """回试跌破 ZG → 3B 不成立。"""
    segments = [
        _seg(0, 0, 0, 10, "up", 18, 12),
        _seg(1, 1, 10, 20, "down", 18, 12),
        _seg(2, 2, 20, 30, "up", 18, 12),
        _seg(3, 3, 30, 50, "up", 30, 19),
        _seg(4, 4, 50, 60, "down", 25, 16),    # 回试: low=16 < ZG=18 → 失败
    ]

    zhongshus = [
        Zhongshu(zd=12.0, zg=18.0, seg_start=0, seg_end=2, seg_count=3,
                 settled=True, break_seg=3, break_direction="up",
                 first_seg_s0=0, last_seg_s1=2, gg=18.0, dd=12.0),
    ]

    return segments, zhongshus


# =====================================================================
# A) Type 1 买点
# =====================================================================

class TestType1Buy:
    def test_downtrend_divergence_produces_1b(self):
        segs, zss, mvs, divs = _make_downtrend_with_divergence()
        bsps = buysellpoints_from_level(segs, zss, mvs, divs, level_id=1)
        t1_buys = [bp for bp in bsps if bp.kind == "type1" and bp.side == "buy"]
        assert len(t1_buys) >= 1
        bp = t1_buys[0]
        assert bp.seg_idx == 10  # C 段终段
        assert bp.level_id == 1
        assert bp.divergence_key is not None


# =====================================================================
# B) Type 1 卖点
# =====================================================================

class TestType1Sell:
    def test_uptrend_divergence_produces_1s(self):
        segs, zss, mvs, divs = _make_uptrend_with_divergence()
        bsps = buysellpoints_from_level(segs, zss, mvs, divs, level_id=1)
        t1_sells = [bp for bp in bsps if bp.kind == "type1" and bp.side == "sell"]
        assert len(t1_sells) >= 1
        bp = t1_sells[0]
        assert bp.seg_idx == 10
        assert bp.side == "sell"


# =====================================================================
# C) Type 3 买点
# =====================================================================

class TestType3Buy:
    def test_pullback_above_zg_produces_3b(self):
        segs, zss, mvs = _make_type3_buy()
        bsps = buysellpoints_from_level(segs, zss, mvs, [], level_id=1)
        t3_buys = [bp for bp in bsps if bp.kind == "type3" and bp.side == "buy"]
        assert len(t3_buys) >= 1
        bp = t3_buys[0]
        assert bp.seg_idx == 4  # 回试段索引
        assert bp.center_zg == 18.0


# =====================================================================
# D) Type 3 卖点
# =====================================================================

class TestType3Sell:
    def test_pullback_below_zd_produces_3s(self):
        """中枢向下突破后回抽不升破 ZD → 3S。"""
        segments = [
            _seg(0, 0, 0, 10, "down", 60, 50),
            _seg(1, 1, 10, 20, "up", 60, 50),
            _seg(2, 2, 20, 30, "down", 60, 50),
            _seg(3, 3, 30, 50, "down", 48, 30),    # 向下突破
            _seg(4, 4, 50, 60, "up", 48, 42),       # 回抽: high=48 < ZD=50 ✓
        ]

        zhongshus = [
            Zhongshu(zd=50.0, zg=60.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="down",
                     first_seg_s0=0, last_seg_s1=2, gg=60.0, dd=50.0),
        ]

        moves = [
            Move(kind="consolidation", direction="down", seg_start=0, seg_end=4,
                 zs_start=0, zs_end=0, zs_count=1, settled=False,
                 high=60.0, low=50.0, first_seg_s0=0, last_seg_s1=4),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, [], level_id=1)
        t3_sells = [bp for bp in bsps if bp.kind == "type3" and bp.side == "sell"]
        assert len(t3_sells) >= 1
        assert t3_sells[0].center_zd == 50.0


# =====================================================================
# E) Type 3 不成立
# =====================================================================

class TestType3Fail:
    def test_pullback_breaks_zg(self):
        segs, zss = _make_type3_buy_fail()
        moves = [
            Move(kind="consolidation", direction="up", seg_start=0, seg_end=4,
                 zs_start=0, zs_end=0, zs_count=1, settled=False,
                 high=18.0, low=12.0, first_seg_s0=0, last_seg_s1=4),
        ]
        bsps = buysellpoints_from_level(segs, zss, moves, [], level_id=1)
        t3_buys = [bp for bp in bsps if bp.kind == "type3" and bp.side == "buy"]
        assert len(t3_buys) == 0


# =====================================================================
# F) 空输入
# =====================================================================

class TestEmptyInputBSP:
    def test_empty(self):
        assert buysellpoints_from_level([], [], [], [], 1) == []


# =====================================================================
# G) 盘整背驰不产生 Type 1
# =====================================================================

class TestConsolidationDivergenceNoType1:
    def test_consolidation_divergence_no_type1(self):
        """盘整背驰（kind="consolidation"）不产生第一类买卖点。"""
        segs = [_seg(i, i, i * 10, (i + 1) * 10, "up", 20, 10) for i in range(5)]

        zhongshus = [
            Zhongshu(zd=12.0, zg=18.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="up",
                     first_seg_s0=0, last_seg_s1=2, gg=18.0, dd=12.0),
        ]

        moves = [
            Move(kind="consolidation", direction="up", seg_start=0, seg_end=4,
                 zs_start=0, zs_end=0, zs_count=1, settled=False,
                 high=18.0, low=12.0, first_seg_s0=0, last_seg_s1=4),
        ]

        # 盘整背驰
        divs = [
            Divergence(kind="consolidation", direction="top", level_id=1,
                       seg_a_start=0, seg_a_end=0, seg_c_start=3, seg_c_end=3,
                       center_idx=0, force_a=100.0, force_c=50.0, confirmed=False),
        ]

        bsps = buysellpoints_from_level(segs, zhongshus, moves, divs, level_id=1)
        t1 = [bp for bp in bsps if bp.kind == "type1"]
        assert len(t1) == 0
