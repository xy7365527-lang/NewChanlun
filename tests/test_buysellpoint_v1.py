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
  H) Type 2 买点：1B 后第一次回调 → 2B
  I) Type 2 卖点：1S 后第一次反弹 → 2S
  J) Type 2 不成立：1B 后没有足够段
  K) 2B+3B 重合：同一 seg_idx 上同时产生 2B 和 3B
  L) 2B+3B 不重合：不同 seg_idx
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


# =====================================================================
# H) Type 2 买点：1B 后第一次回调 → 2B
# =====================================================================

class TestType2Buy:
    def test_type2_buy_after_1b(self):
        """1B 出现后，第一次反弹后的第一次回调 → 2B。

        扩展下跌趋势 fixture：
          seg10 = 1B（下跌趋势背驰底部）
          seg11 (up) = 反弹
          seg12 (down) = 回调 → 2B at seg_idx=12
        """
        segs, zss, mvs, divs = _make_downtrend_with_divergence()
        # 追加反弹和回调段
        segs.extend([
            _seg(11, 11, 110, 130, "up", 60, 46),     # 反弹
            _seg(12, 12, 130, 140, "down", 55, 48),    # 回调 → 2B
        ])

        bsps = buysellpoints_from_level(segs, zss, mvs, divs, level_id=1)
        t2_buys = [bp for bp in bsps if bp.kind == "type2" and bp.side == "buy"]
        assert len(t2_buys) >= 1
        bp = t2_buys[0]
        assert bp.seg_idx == 12  # 回调段
        assert bp.divergence_key is not None  # 继承自 type1
        assert bp.price == 48.0  # 回调段 low


# =====================================================================
# I) Type 2 卖点：1S 后第一次反弹 → 2S
# =====================================================================

class TestType2Sell:
    def test_type2_sell_after_1s(self):
        """1S 出现后，第一次回调后的第一次反弹 → 2S。

        扩展上涨趋势 fixture：
          seg10 = 1S（上涨趋势背驰顶部）
          seg11 (down) = 回调
          seg12 (up) = 反弹 → 2S at seg_idx=12
        """
        segs, zss, mvs, divs = _make_uptrend_with_divergence()
        segs.extend([
            _seg(11, 11, 110, 130, "down", 32, 25),   # 回调
            _seg(12, 12, 130, 140, "up", 30, 26),      # 反弹 → 2S
        ])

        bsps = buysellpoints_from_level(segs, zss, mvs, divs, level_id=1)
        t2_sells = [bp for bp in bsps if bp.kind == "type2" and bp.side == "sell"]
        assert len(t2_sells) >= 1
        bp = t2_sells[0]
        assert bp.seg_idx == 12
        assert bp.side == "sell"
        assert bp.divergence_key is not None
        assert bp.price == 30.0  # 反弹段 high


# =====================================================================
# J) Type 2 不成立：1B 后没有足够段
# =====================================================================

class TestType2NotEnoughSegments:
    def test_no_type2_when_no_rebound(self):
        """1B 之后没有反弹和回调段 → 不产生 2B。"""
        segs, zss, mvs, divs = _make_downtrend_with_divergence()
        # 不追加额外段
        bsps = buysellpoints_from_level(segs, zss, mvs, divs, level_id=1)
        t2 = [bp for bp in bsps if bp.kind == "type2"]
        assert len(t2) == 0

    def test_no_type2_when_only_rebound(self):
        """1B 之后只有反弹没有回调 → 不产生 2B。"""
        segs, zss, mvs, divs = _make_downtrend_with_divergence()
        segs.append(_seg(11, 11, 110, 130, "up", 60, 46))
        bsps = buysellpoints_from_level(segs, zss, mvs, divs, level_id=1)
        t2 = [bp for bp in bsps if bp.kind == "type2"]
        assert len(t2) == 0


# =====================================================================
# K) 2B+3B 重合
# =====================================================================

class TestOverlap2B3B:
    def test_2b_3b_overlap_same_seg(self):
        """V型反转：1B后凌厉上破最后中枢，回试不触及ZG → 2B+3B重合。

        扩展下跌趋势：
          seg10 = 1B
          seg11 (up) = 凌厉反弹，突破 zhongshu1 的 ZG=60
          seg12 (down) = 回试，low=62 > ZG=60 → 3B
                         同时也是 1B 后的第一次回调 → 2B
          → seg12 同时是 2B 和 3B → overlaps_with 标记
        """
        segs, zss, mvs, divs = _make_downtrend_with_divergence()

        # 修改 zhongshu1：break_direction="up" 表示上方突破
        zss[1] = Zhongshu(
            zd=50.0, zg=60.0, seg_start=7, seg_end=9, seg_count=3,
            settled=True, break_seg=11, break_direction="up",
            first_seg_s0=7, last_seg_s1=9, gg=60.0, dd=50.0,
        )

        segs.extend([
            _seg(11, 11, 110, 130, "up", 75, 46),     # 凌厉上破 ZG=60
            _seg(12, 12, 130, 140, "down", 70, 62),    # 回试 low=62 > ZG=60 → 3B
        ])

        bsps = buysellpoints_from_level(segs, zss, mvs, divs, level_id=1)
        t2_buys = [bp for bp in bsps if bp.kind == "type2" and bp.side == "buy"]
        t3_buys = [bp for bp in bsps if bp.kind == "type3" and bp.side == "buy"]

        # 都应在 seg12
        assert len(t2_buys) >= 1
        assert len(t3_buys) >= 1
        assert t2_buys[0].seg_idx == 12
        assert t3_buys[0].seg_idx == 12

        # 重合标记
        assert t2_buys[0].overlaps_with == "type3"
        assert t3_buys[0].overlaps_with == "type2"


# =====================================================================
# L) 2B+3B 不重合
# =====================================================================

class TestOverlapNone:
    def test_no_overlap_different_seg(self):
        """2B 和 3B 不在同一段 → 无重合标记。"""
        segs, zss, mvs, divs = _make_downtrend_with_divergence()

        # zhongshu1 保持 break_direction="down"（不触发 3B）
        segs.extend([
            _seg(11, 11, 110, 130, "up", 60, 46),
            _seg(12, 12, 130, 140, "down", 55, 48),
        ])

        bsps = buysellpoints_from_level(segs, zss, mvs, divs, level_id=1)
        t2_buys = [bp for bp in bsps if bp.kind == "type2" and bp.side == "buy"]
        # 有 2B 但 zhongshu1 的 break_direction="down" 不产生 3B
        # 所以不会重合
        for bp in t2_buys:
            assert bp.overlaps_with is None


# =====================================================================
# G) 探索性测试 — TBD 定义矛盾暴露
# （testing-override 生成态例外：目的是暴露问题，非覆盖率指标）
# =====================================================================

class TestTBD2ConfirmedPropagation:
    """[TBD-2] confirmed 状态从 Divergence 传递到 Type1 BSP。

    当前决策：BSP.confirmed = div.confirmed（跟随背驰确认态）。
    翻转条件：若走势完成与背驰不等价，需引入独立的 move_completed 状态。
    """

    def test_type1_buy_confirmed_true(self):
        """div.confirmed=True → BSP.confirmed=True。"""
        segs, zss, mvs, divs = _make_downtrend_with_divergence()

        # 替换 divergence 为 confirmed=True 版本
        div_confirmed = Divergence(
            kind="trend", direction="bottom", level_id=1,
            seg_a_start=3, seg_a_end=6, seg_c_start=10, seg_c_end=10,
            center_idx=1, force_a=500.0, force_c=50.0, confirmed=True,
        )

        bsps = buysellpoints_from_level(segs, zss, mvs, [div_confirmed], level_id=1)
        t1_buys = [bp for bp in bsps if bp.kind == "type1" and bp.side == "buy"]
        assert len(t1_buys) == 1
        assert t1_buys[0].confirmed is True

    def test_type1_buy_confirmed_false(self):
        """div.confirmed=False → BSP.confirmed=False。"""
        segs, zss, mvs, divs = _make_downtrend_with_divergence()
        # 原始数据中 div.confirmed=False
        bsps = buysellpoints_from_level(segs, zss, mvs, divs, level_id=1)
        t1_buys = [bp for bp in bsps if bp.kind == "type1" and bp.side == "buy"]
        assert len(t1_buys) == 1
        assert t1_buys[0].confirmed is False


class TestTBD3Type2ConfirmedIndependence:
    """[TBD-3] Type2 的 confirmed 跟随回调段而非 Type1。

    当前决策：type2.confirmed = callback_seg.confirmed。
    翻转条件：若确认时机需要后续走势验证，需引入额外状态机。
    """

    def test_type2_confirmed_follows_callback_not_type1(self):
        """Type1.confirmed=True，但回调段 confirmed=False → Type2.confirmed=False。

        这验证了 Type2 的确认是独立路径，不从 Type1 继承。
        """
        segs, zss, mvs, divs = _make_downtrend_with_divergence()

        # 让 divergence confirmed=True → Type1 将是 confirmed
        div_confirmed = Divergence(
            kind="trend", direction="bottom", level_id=1,
            seg_a_start=3, seg_a_end=6, seg_c_start=10, seg_c_end=10,
            center_idx=1, force_a=500.0, force_c=50.0, confirmed=True,
        )

        # 添加后续段用于 Type2 检测（默认 confirmed=True）
        segs_ext = list(segs) + [
            _seg(11, 11, 110, 130, "up", 60, 46, confirmed=True),     # 反弹段
            _seg(12, 12, 130, 140, "down", 55, 48, confirmed=False),  # 回调段: confirmed=False
        ]

        bsps = buysellpoints_from_level(segs_ext, zss, mvs, [div_confirmed], level_id=1)
        t1_buys = [bp for bp in bsps if bp.kind == "type1" and bp.side == "buy"]
        t2_buys = [bp for bp in bsps if bp.kind == "type2" and bp.side == "buy"]

        assert len(t1_buys) == 1
        assert t1_buys[0].confirmed is True   # Type1 跟随 div

        assert len(t2_buys) == 1
        assert t2_buys[0].confirmed is False   # Type2 跟随回调段，非 Type1


class TestTBD1StrictCriterion:
    """[TBD-1] 严格口径：盘整 Move 不产生 Type 1 买卖点。

    当前决策：assoc_move.kind != "trend" → 跳过。
    翻转条件：若宽松口径（盘整背驰也算），需删除此守卫。
    """

    def test_consolidation_move_with_trend_div_no_type1(self):
        """即使 Divergence.kind=="trend"，若关联 Move 是盘整，也不产生 Type1。

        这是防御测试：验证 TBD-1 的严格口径守卫工作。
        """
        segments = [
            _seg(0, 0, 0, 10, "down", 95, 80),
            _seg(1, 1, 10, 20, "up", 90, 82),
            _seg(2, 2, 20, 30, "down", 88, 80),
            _seg(3, 3, 30, 40, "up", 82, 75),
            _seg(4, 4, 40, 50, "down", 75, 55),
            _seg(5, 5, 50, 60, "up", 60, 50),
            _seg(6, 6, 60, 70, "down", 60, 50),
            _seg(7, 7, 70, 80, "up", 55, 48),
            _seg(8, 8, 80, 90, "down", 55, 45),
        ]

        zhongshus = [
            Zhongshu(zd=80.0, zg=90.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="down",
                     first_seg_s0=0, last_seg_s1=2, gg=95.0, dd=80.0),
            Zhongshu(zd=50.0, zg=60.0, seg_start=5, seg_end=7, seg_count=3,
                     settled=True, break_seg=8, break_direction="down",
                     first_seg_s0=5, last_seg_s1=7, gg=60.0, dd=48.0),
        ]

        # Move 标记为 consolidation（而非 trend）
        moves = [
            Move(kind="consolidation", direction="down", seg_start=0, seg_end=8,
                 zs_start=0, zs_end=1, zs_count=2, settled=False,
                 high=90.0, low=50.0, first_seg_s0=0, last_seg_s1=8),
        ]

        # 手工构造趋势背驰（绕过 _detect_trend_divergence 的 move.kind 检查）
        divergences = [
            Divergence(kind="trend", direction="bottom", level_id=1,
                       seg_a_start=3, seg_a_end=4, seg_c_start=8, seg_c_end=8,
                       center_idx=1, force_a=400.0, force_c=50.0, confirmed=False),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)
        t1_points = [bp for bp in bsps if bp.kind == "type1"]
        # 严格口径：consolidation Move → 无 Type1
        assert len(t1_points) == 0
