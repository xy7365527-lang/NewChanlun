"""背驰 v1 管线 — 单元测试（探索性：beichi 定义仍为生成态）

使用 v1 的 Zhongshu + Move 类型替代 v0 的 Center + TrendTypeInstance。
Divergence 输出类型不变。

测试项：
  A) 趋势背驰：2 中枢趋势 Move，C 段力度 < A 段 → 检出背驰
  B) 无背驰：C 段力度 >= A 段 → 无背驰
  C) 盘整背驰：单中枢盘整 Move，后一次离开力度 < 前一次
  D) 不足条件：单中枢 trend 不触发
  E) 方向标签：up → top，down → bottom
  F) 空输入
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import Zhongshu
from newchan.a_move_v1 import Move
from newchan.a_divergence_v1 import Divergence, divergences_from_moves_v1


# ── helpers ──

def _seg(s0: int, s1: int, i0: int, i1: int, d: str,
         h: float, l: float, confirmed: bool = True) -> Segment:
    return Segment(s0=s0, s1=s1, i0=i0, i1=i1, direction=d,
                   high=h, low=l, confirmed=confirmed)


def _make_uptrend_v1():
    """构造上涨趋势数据（v1 类型）。

    结构（同 test_divergence.py 但用 Zhongshu/Move）：
      seg0-2  → zhongshu0 [ZD=12,ZG=18], GG=20, DD=10
      seg3-6  → 过渡段
      seg7-9  → zhongshu1 [ZD=28,ZG=32], GG=34, DD=26
      seg10   → C 段（力度小: 2 点振幅）

    上涨判定: zhongshu1.dd(26) > zhongshu0.gg(20) → up
    """
    segments = [
        _seg(0, 0, 0, 10, "up", 20, 10),       # seg0 (zs0)
        _seg(1, 1, 10, 20, "down", 20, 12),     # seg1 (zs0)
        _seg(2, 2, 20, 30, "up", 18, 12),       # seg2 (zs0)
        _seg(3, 3, 30, 40, "down", 11, 8),      # seg3 离开
        _seg(4, 4, 40, 50, "up", 11, 8),        # seg4
        _seg(5, 5, 50, 60, "down", 25, 20),     # seg5 (A 段)
        _seg(6, 6, 60, 70, "up", 25, 20),       # seg6 (A 段)
        _seg(7, 7, 70, 80, "down", 32, 26),     # seg7 (zs1)
        _seg(8, 8, 80, 90, "up", 34, 26),       # seg8 (zs1)
        _seg(9, 9, 90, 100, "down", 34, 28),    # seg9 (zs1)
        _seg(10, 10, 100, 110, "up", 35, 33),   # seg10 (C 段: 2 点)
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


def _make_downtrend_v1():
    """构造下跌趋势数据（v1 类型）。

      seg0-2  → zhongshu0 [ZD=82,ZG=90], GG=95, DD=80
      seg3-4  → A 段（大力度 20 点）
      seg5-6  → 过渡
      seg7-9  → zhongshu1 [ZD=50,ZG=60], GG=60, DD=50
      seg10   → C 段（小力度 5 点）

    下跌判定: zhongshu1.gg(60) < zhongshu0.dd(80) → down
    """
    segments = [
        _seg(0, 0, 0, 10, "down", 95, 80),      # seg0
        _seg(1, 1, 10, 20, "up", 95, 82),        # seg1
        _seg(2, 2, 20, 30, "down", 90, 80),      # seg2 → zs0
        _seg(3, 3, 30, 40, "up", 82, 75),        # seg3
        _seg(4, 4, 40, 50, "down", 75, 55),      # seg4 (A 段: 20 点)
        _seg(5, 5, 50, 60, "up", 65, 55),        # seg5
        _seg(6, 6, 60, 70, "down", 65, 50),      # seg6
        _seg(7, 7, 70, 80, "up", 60, 50),        # seg7 → zs1
        _seg(8, 8, 80, 90, "down", 60, 50),      # seg8
        _seg(9, 9, 90, 100, "up", 52, 48),       # seg9 → zs1
        _seg(10, 10, 100, 110, "down", 50, 45),  # seg10 (C: 5 点)
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

    return segments, zhongshus, moves


def _make_consolidation_v1():
    """构造盘整数据（v1 类型）— 单中枢，两次同向离开。

      seg0-2  → zhongshu0 [ZD=12,ZG=18], GG=20, DD=10
      seg3    → 第一次向上离开（大力度）
      seg4    → 回抽
      seg5    → 第二次向上离开（小力度）→ 盘整背驰
    """
    segments = [
        _seg(0, 0, 0, 10, "up", 20, 10),       # seg0 (zs0)
        _seg(1, 1, 10, 20, "down", 20, 12),     # seg1 (zs0)
        _seg(2, 2, 20, 30, "up", 18, 12),       # seg2 (zs0)
        _seg(3, 3, 30, 50, "down", 22, 5),      # seg3: 第一次离开 (high>ZG, 力度=17*20=340)
        _seg(4, 4, 50, 60, "up", 15, 10),       # seg4: 回抽入中枢
        _seg(5, 5, 60, 70, "down", 19, 9),      # seg5: 第二次离开 (high>ZG, 力度=10*10=100)
    ]

    zhongshus = [
        Zhongshu(zd=12.0, zg=18.0, seg_start=0, seg_end=2, seg_count=3,
                 settled=True, break_seg=5, break_direction="down",
                 first_seg_s0=0, last_seg_s1=5, gg=20.0, dd=10.0),
    ]

    moves = [
        Move(kind="consolidation", direction="down", seg_start=0, seg_end=5,
             zs_start=0, zs_end=0, zs_count=1, settled=False,
             high=18.0, low=12.0, first_seg_s0=0, last_seg_s1=5),
    ]

    return segments, zhongshus, moves


# =====================================================================
# A) 趋势背驰检出
# =====================================================================

class TestTrendDivergenceV1:
    """v1 管线趋势背驰检测。"""

    def test_uptrend_divergence_detected(self):
        segs, zss, mvs = _make_uptrend_v1()
        divs = divergences_from_moves_v1(segs, zss, mvs, level_id=1)
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) >= 1
        d = trend_divs[0]
        assert d.direction == "top"
        assert d.force_c < d.force_a
        assert d.level_id == 1
        # A段=[3,6]（zs0.seg_end+1 ~ zs1.seg_start-1），C段=[10,10]
        assert d.seg_a_start == 3
        assert d.seg_a_end == 6
        assert d.seg_c_start == 10
        assert d.seg_c_end == 10
        assert d.center_idx == 1  # 最后中枢索引
        assert d.confirmed is False  # move.settled=False

    def test_downtrend_divergence_detected(self):
        segs, zss, mvs = _make_downtrend_v1()
        divs = divergences_from_moves_v1(segs, zss, mvs, level_id=1)
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) >= 1
        d = trend_divs[0]
        assert d.direction == "bottom"
        assert d.force_c < d.force_a
        # 同结构：A=[3,6], C=[10,10]
        assert d.seg_a_start == 3
        assert d.seg_a_end == 6
        assert d.seg_c_start == 10
        assert d.seg_c_end == 10

    def test_trend_confirmed_from_settled_move(self):
        """move.settled=True → divergence.confirmed=True。"""
        segs, zss, _ = _make_uptrend_v1()
        mvs = [
            Move(kind="trend", direction="up", seg_start=0, seg_end=10,
                 zs_start=0, zs_end=1, zs_count=2, settled=True,
                 high=32.0, low=12.0, first_seg_s0=0, last_seg_s1=10),
        ]
        divs = divergences_from_moves_v1(segs, zss, mvs, level_id=1)
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) >= 1
        assert trend_divs[0].confirmed is True

    def test_c_segment_not_formed_no_divergence(self):
        """C段未形成（zs_last.seg_end == move.seg_end）→ 无背驰。"""
        segs = [
            _seg(0, 0, 0, 10, "up", 20, 10),
            _seg(1, 1, 10, 20, "down", 20, 12),
            _seg(2, 2, 20, 30, "up", 18, 12),
            _seg(3, 3, 30, 40, "down", 11, 8),
            _seg(4, 4, 40, 50, "up", 25, 20),
            _seg(5, 5, 50, 60, "down", 32, 26),
            _seg(6, 6, 60, 70, "up", 34, 26),
            _seg(7, 7, 70, 80, "down", 34, 28),  # move 终止于 zs1.seg_end
        ]
        zhongshus = [
            Zhongshu(zd=12.0, zg=18.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="up",
                     first_seg_s0=0, last_seg_s1=2, gg=20.0, dd=10.0),
            Zhongshu(zd=28.0, zg=32.0, seg_start=5, seg_end=7, seg_count=3,
                     settled=True, break_seg=-1, break_direction="",
                     first_seg_s0=5, last_seg_s1=7, gg=34.0, dd=26.0),
        ]
        moves = [
            Move(kind="trend", direction="up", seg_start=0, seg_end=7,
                 zs_start=0, zs_end=1, zs_count=2, settled=False,
                 high=32.0, low=12.0, first_seg_s0=0, last_seg_s1=7),
        ]
        divs = divergences_from_moves_v1(segs, zhongshus, moves, level_id=1)
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) == 0


# =====================================================================
# B) 无背驰
# =====================================================================

class TestNoDivergenceV1:
    """C 段力度 >= A 段时不检出。"""

    def test_strong_c_segment_no_divergence(self):
        """C 段力度大于 A 段。"""
        segs = [
            _seg(0, 0, 0, 10, "up", 20, 10),
            _seg(1, 1, 10, 20, "down", 20, 12),
            _seg(2, 2, 20, 30, "up", 18, 12),
            _seg(3, 3, 30, 40, "down", 18, 15),
            _seg(4, 4, 40, 50, "up", 22, 15),   # A 段: 7 点
            _seg(5, 5, 50, 60, "down", 22, 18),
            _seg(6, 6, 60, 70, "up", 28, 18),
            _seg(7, 7, 70, 80, "down", 28, 22),
            _seg(8, 8, 80, 90, "up", 26, 22),
            _seg(9, 9, 90, 100, "down", 26, 23),
            _seg(10, 10, 100, 140, "up", 50, 23),  # C: 27 点 x 40 bars = 1080 >> A
        ]

        zhongshus = [
            Zhongshu(zd=12.0, zg=18.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="up",
                     first_seg_s0=0, last_seg_s1=2, gg=20.0, dd=10.0),
            Zhongshu(zd=22.0, zg=26.0, seg_start=7, seg_end=9, seg_count=3,
                     settled=True, break_seg=10, break_direction="up",
                     first_seg_s0=7, last_seg_s1=9, gg=28.0, dd=22.0),
        ]

        moves = [
            Move(kind="trend", direction="up", seg_start=0, seg_end=10,
                 zs_start=0, zs_end=1, zs_count=2, settled=False,
                 high=26.0, low=12.0, first_seg_s0=0, last_seg_s1=10),
        ]

        divs = divergences_from_moves_v1(segs, zhongshus, moves, level_id=1)
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) == 0


# =====================================================================
# C) 盘整背驰
# =====================================================================

class TestConsolidationDivergenceV1:
    """盘整中后一次同向离开力度 < 前一次 → 盘整背驰。"""

    def test_consolidation_divergence_detected(self):
        segs, zss, mvs = _make_consolidation_v1()
        divs = divergences_from_moves_v1(segs, zss, mvs, level_id=1)
        cons_divs = [d for d in divs if d.kind == "consolidation"]
        assert len(cons_divs) >= 1
        d = cons_divs[0]
        assert d.force_c < d.force_a

    def test_no_consolidation_when_force_increases(self):
        """后一次离开力度更大 → 不检出。"""
        segments = [
            _seg(0, 0, 0, 10, "up", 18, 12),       # seg0 中枢内 (h<=ZG, l>=ZD)
            _seg(1, 1, 10, 20, "down", 18, 12),     # seg1 中枢内
            _seg(2, 2, 20, 30, "up", 18, 12),       # seg2 中枢内
            _seg(3, 3, 30, 40, "down", 19, 8),      # 第一次 down 离开 (force=11*10=110)
            _seg(4, 4, 40, 50, "up", 16, 13),       # 回抽（up 方向，仅 1 次，无法比较）
            _seg(5, 5, 50, 80, "down", 22, 2),      # 第二次 down 离开 (force=20*30=600 >> 110)
        ]

        zhongshus = [
            Zhongshu(zd=12.0, zg=18.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=5, break_direction="down",
                     first_seg_s0=0, last_seg_s1=5, gg=20.0, dd=10.0),
        ]

        moves = [
            Move(kind="consolidation", direction="down", seg_start=0, seg_end=5,
                 zs_start=0, zs_end=0, zs_count=1, settled=False,
                 high=18.0, low=12.0, first_seg_s0=0, last_seg_s1=5),
        ]

        divs = divergences_from_moves_v1(segments, zhongshus, moves, level_id=1)
        cons_divs = [d for d in divs if d.kind == "consolidation"]
        assert len(cons_divs) == 0

    # ── C1: 上行离开盘整背驰 → top ──

    def test_upward_exit_divergence_top(self):
        """两次向上离开中枢，第二次力度弱 → top 盘整背驰。

        中枢内段严格在 [ZD,ZG] 内以避免意外 exit。
        exit_segs_by_dir["up"] = [3, 5]
        force(seg3) = (80-55) * (50-30) = 25*20 = 500
        force(seg5) = (65-58) * (70-60) = 7*10 = 70
        70 < 500 → top divergence
        """
        segments = [
            _seg(0, 0, 0, 10, "up", 58, 52),       # 中枢内 (h<ZG=60, l>ZD=50)
            _seg(1, 1, 10, 20, "down", 58, 52),     # 中枢内
            _seg(2, 2, 20, 30, "up", 58, 52),       # 中枢内
            _seg(3, 3, 30, 50, "up", 80, 55),       # 第一次 up 离开（大力度）
            _seg(4, 4, 50, 60, "down", 62, 48),     # 回抽 (down 仅 1 次)
            _seg(5, 5, 60, 70, "up", 65, 58),       # 第二次 up 离开（小力度）
        ]

        zhongshus = [
            Zhongshu(zd=50.0, zg=60.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="up",
                     first_seg_s0=0, last_seg_s1=5, gg=60.0, dd=50.0),
        ]

        moves = [
            Move(kind="consolidation", direction="up", seg_start=0, seg_end=5,
                 zs_start=0, zs_end=0, zs_count=1, settled=False,
                 high=60.0, low=50.0, first_seg_s0=0, last_seg_s1=5),
        ]

        divs = divergences_from_moves_v1(segments, zhongshus, moves, level_id=1)
        cons_divs = [d for d in divs if d.kind == "consolidation"]
        assert len(cons_divs) == 1
        d = cons_divs[0]
        assert d.direction == "top"
        assert d.force_c < d.force_a
        assert d.seg_a_start == 3
        assert d.seg_c_start == 5

    # ── C2: 下行离开盘整背驰 → bottom ──

    def test_downward_exit_divergence_bottom(self):
        """两次向下离开中枢，第二次力度弱 → bottom 盘整背驰。

        exit_segs_by_dir["down"] = [3, 5]
        force(seg3) = (55-30) * (50-30) = 25*20 = 500
        force(seg5) = (52-45) * (70-60) = 7*10 = 70
        70 < 500 → bottom divergence
        """
        segments = [
            _seg(0, 0, 0, 10, "down", 58, 52),      # 中枢内
            _seg(1, 1, 10, 20, "up", 58, 52),        # 中枢内
            _seg(2, 2, 20, 30, "down", 58, 52),      # 中枢内
            _seg(3, 3, 30, 50, "down", 55, 30),      # 第一次 down 离开（大力度）
            _seg(4, 4, 50, 60, "up", 62, 48),        # 回抽 (up 仅 1 次)
            _seg(5, 5, 60, 70, "down", 52, 45),      # 第二次 down 离开（小力度）
        ]

        zhongshus = [
            Zhongshu(zd=50.0, zg=60.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="down",
                     first_seg_s0=0, last_seg_s1=5, gg=60.0, dd=50.0),
        ]

        moves = [
            Move(kind="consolidation", direction="down", seg_start=0, seg_end=5,
                 zs_start=0, zs_end=0, zs_count=1, settled=False,
                 high=60.0, low=50.0, first_seg_s0=0, last_seg_s1=5),
        ]

        divs = divergences_from_moves_v1(segments, zhongshus, moves, level_id=1)
        cons_divs = [d for d in divs if d.kind == "consolidation"]
        assert len(cons_divs) == 1
        d = cons_divs[0]
        assert d.direction == "bottom"
        assert d.force_c < d.force_a
        assert d.seg_a_start == 3
        assert d.seg_c_start == 5

    # ── C3: confirmed 状态 = move.settled ──

    def test_confirmed_true_from_settled_move(self):
        """move.settled=True → divergence.confirmed=True。"""
        segments = [
            _seg(0, 0, 0, 10, "down", 58, 52),
            _seg(1, 1, 10, 20, "up", 58, 52),
            _seg(2, 2, 20, 30, "down", 58, 52),
            _seg(3, 3, 30, 50, "down", 55, 30),
            _seg(4, 4, 50, 60, "up", 62, 48),
            _seg(5, 5, 60, 70, "down", 52, 45),
        ]

        zhongshus = [
            Zhongshu(zd=50.0, zg=60.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="down",
                     first_seg_s0=0, last_seg_s1=5, gg=60.0, dd=50.0),
        ]

        moves = [
            Move(kind="consolidation", direction="down", seg_start=0, seg_end=5,
                 zs_start=0, zs_end=0, zs_count=1, settled=True,
                 high=60.0, low=50.0, first_seg_s0=0, last_seg_s1=5),
        ]

        divs = divergences_from_moves_v1(segments, zhongshus, moves, level_id=1)
        cons_divs = [d for d in divs if d.kind == "consolidation"]
        assert len(cons_divs) == 1
        assert cons_divs[0].confirmed is True

    def test_confirmed_false_from_unsettled_move(self):
        """move.settled=False → divergence.confirmed=False。"""
        segments = [
            _seg(0, 0, 0, 10, "down", 58, 52),
            _seg(1, 1, 10, 20, "up", 58, 52),
            _seg(2, 2, 20, 30, "down", 58, 52),
            _seg(3, 3, 30, 50, "down", 55, 30),
            _seg(4, 4, 50, 60, "up", 62, 48),
            _seg(5, 5, 60, 70, "down", 52, 45),
        ]

        zhongshus = [
            Zhongshu(zd=50.0, zg=60.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="down",
                     first_seg_s0=0, last_seg_s1=5, gg=60.0, dd=50.0),
        ]

        moves = [
            Move(kind="consolidation", direction="down", seg_start=0, seg_end=5,
                 zs_start=0, zs_end=0, zs_count=1, settled=False,
                 high=60.0, low=50.0, first_seg_s0=0, last_seg_s1=5),
        ]

        divs = divergences_from_moves_v1(segments, zhongshus, moves, level_id=1)
        cons_divs = [d for d in divs if d.kind == "consolidation"]
        assert len(cons_divs) == 1
        assert cons_divs[0].confirmed is False

    # ── C4: 等力度 → 无背驰 ──

    def test_equal_force_no_divergence(self):
        """两次离开力度相同 → force_c 不小于 force_a → 无背驰。

        force(seg3) = (55-45) * (40-30) = 10*10 = 100
        force(seg5) = (55-45) * (70-60) = 10*10 = 100
        100 不 < 100 → 无背驰
        """
        segments = [
            _seg(0, 0, 0, 10, "down", 58, 52),
            _seg(1, 1, 10, 20, "up", 58, 52),
            _seg(2, 2, 20, 30, "down", 58, 52),
            _seg(3, 3, 30, 40, "down", 55, 45),     # 第一次 down (force=100)
            _seg(4, 4, 40, 60, "up", 62, 48),        # 回抽
            _seg(5, 5, 60, 70, "down", 55, 45),      # 第二次 down (force=100)
        ]

        zhongshus = [
            Zhongshu(zd=50.0, zg=60.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="down",
                     first_seg_s0=0, last_seg_s1=5, gg=60.0, dd=50.0),
        ]

        moves = [
            Move(kind="consolidation", direction="down", seg_start=0, seg_end=5,
                 zs_start=0, zs_end=0, zs_count=1, settled=False,
                 high=60.0, low=50.0, first_seg_s0=0, last_seg_s1=5),
        ]

        divs = divergences_from_moves_v1(segments, zhongshus, moves, level_id=1)
        cons_divs = [d for d in divs if d.kind == "consolidation"]
        assert len(cons_divs) == 0

    # ── C6: 每方向仅一次离开 → 无背驰 ──

    def test_single_exit_per_direction_no_divergence(self):
        """每个方向只有 1 次离开（需 >= 2 次同向），无背驰。"""
        segments = [
            _seg(0, 0, 0, 10, "down", 58, 52),
            _seg(1, 1, 10, 20, "up", 58, 52),
            _seg(2, 2, 20, 30, "down", 58, 52),
            _seg(3, 3, 30, 40, "down", 55, 30),     # 仅 1 次 down
            _seg(4, 4, 40, 50, "up", 70, 55),       # 仅 1 次 up
        ]

        zhongshus = [
            Zhongshu(zd=50.0, zg=60.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="down",
                     first_seg_s0=0, last_seg_s1=4, gg=60.0, dd=50.0),
        ]

        moves = [
            Move(kind="consolidation", direction="down", seg_start=0, seg_end=4,
                 zs_start=0, zs_end=0, zs_count=1, settled=False,
                 high=60.0, low=50.0, first_seg_s0=0, last_seg_s1=4),
        ]

        divs = divergences_from_moves_v1(segments, zhongshus, moves, level_id=1)
        cons_divs = [d for d in divs if d.kind == "consolidation"]
        assert len(cons_divs) == 0


# =====================================================================
# D) 不足条件
# =====================================================================

class TestInsufficientConditionsV1:
    """单中枢不触发趋势背驰。"""

    def test_single_center_no_trend_divergence(self):
        segs = [
            _seg(0, 0, 0, 10, "up", 20, 10),
            _seg(1, 1, 10, 20, "down", 20, 12),
            _seg(2, 2, 20, 30, "up", 18, 12),
            _seg(3, 3, 30, 40, "down", 18, 14),
            _seg(4, 4, 40, 50, "up", 16, 14),
        ]

        zhongshus = [
            Zhongshu(zd=12.0, zg=18.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="down",
                     first_seg_s0=0, last_seg_s1=2, gg=20.0, dd=10.0),
        ]

        moves = [
            Move(kind="consolidation", direction="down", seg_start=0, seg_end=4,
                 zs_start=0, zs_end=0, zs_count=1, settled=False,
                 high=18.0, low=12.0, first_seg_s0=0, last_seg_s1=4),
        ]

        divs = divergences_from_moves_v1(segs, zhongshus, moves, level_id=1)
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) == 0


# =====================================================================
# G) force_a <= 0 守卫（退化 A 段）
# =====================================================================

class TestZeroForceGuard:
    """A 段力度为零（价格平坦退化）时守卫应阻止误报。"""

    def test_trend_zero_force_a_no_divergence(self):
        """趋势背驰：A 段所有线段 high == low → force_a=0 → 无背驰。

        结构：
          seg0-seg2: zs0 (settled, [12,18])
          seg3:      A 段 — high=low=15 → force=(15-15)*10 = 0
          seg4-seg6: zs1 (settled, [22,28])
          seg7:      C 段 — 正常力度
        """
        segments = [
            _seg(0, 0, 0, 10, "up", 18, 12),
            _seg(1, 1, 10, 20, "down", 18, 12),
            _seg(2, 2, 20, 30, "up", 18, 12),       # zs0 结束
            _seg(3, 3, 30, 40, "up", 15, 15),        # A 段：flat price → force=0
            _seg(4, 4, 40, 50, "down", 28, 22),
            _seg(5, 5, 50, 60, "up", 28, 22),
            _seg(6, 6, 60, 70, "down", 28, 22),      # zs1 结束
            _seg(7, 7, 70, 80, "up", 35, 30),        # C 段：5*10=50
        ]

        zhongshus = [
            Zhongshu(zd=12.0, zg=18.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="up",
                     first_seg_s0=0, last_seg_s1=2, gg=20.0, dd=10.0),
            Zhongshu(zd=22.0, zg=28.0, seg_start=4, seg_end=6, seg_count=3,
                     settled=True, break_seg=7, break_direction="up",
                     first_seg_s0=4, last_seg_s1=6, gg=30.0, dd=20.0),
        ]

        moves = [
            Move(kind="trend", direction="up", seg_start=0, seg_end=7,
                 zs_start=0, zs_end=1, zs_count=2, settled=False,
                 high=28.0, low=12.0, first_seg_s0=0, last_seg_s1=7),
        ]

        divs = divergences_from_moves_v1(segments, zhongshus, moves, level_id=1)
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) == 0

    def test_consolidation_zero_force_a_no_divergence(self):
        """盘整背驰：第一次离开段 high == low → force_a=0 → 跳过 → 无背驰。

        exit_segs_by_dir["down"] = [3, 5]
        force(seg3) = (50-50) * 10 = 0   → force_a <= 0 → continue
        结果：无盘整背驰
        """
        segments = [
            _seg(0, 0, 0, 10, "down", 58, 52),      # 中枢内
            _seg(1, 1, 10, 20, "up", 58, 52),        # 中枢内
            _seg(2, 2, 20, 30, "down", 58, 52),      # 中枢内
            _seg(3, 3, 30, 40, "down", 50, 50),      # 第一次 down 离开：flat → force=0
            _seg(4, 4, 40, 50, "up", 58, 52),        # 回抽
            _seg(5, 5, 50, 60, "down", 52, 45),      # 第二次 down 离开：正常力度
        ]

        zhongshus = [
            Zhongshu(zd=50.0, zg=60.0, seg_start=0, seg_end=2, seg_count=3,
                     settled=True, break_seg=3, break_direction="down",
                     first_seg_s0=0, last_seg_s1=5, gg=60.0, dd=50.0),
        ]

        moves = [
            Move(kind="consolidation", direction="down", seg_start=0, seg_end=5,
                 zs_start=0, zs_end=0, zs_count=1, settled=False,
                 high=60.0, low=50.0, first_seg_s0=0, last_seg_s1=5),
        ]

        divs = divergences_from_moves_v1(segments, zhongshus, moves, level_id=1)
        cons_divs = [d for d in divs if d.kind == "consolidation"]
        assert len(cons_divs) == 0


# =====================================================================
# E) 方向标签
# =====================================================================

class TestDirectionLabelsV1:
    """up → top, down → bottom."""

    def test_up_trend_gives_top(self):
        segs, zss, mvs = _make_uptrend_v1()
        divs = divergences_from_moves_v1(segs, zss, mvs, level_id=1)
        for d in divs:
            if d.kind == "trend":
                assert d.direction == "top"

    def test_down_trend_gives_bottom(self):
        segs, zss, mvs = _make_downtrend_v1()
        divs = divergences_from_moves_v1(segs, zss, mvs, level_id=1)
        for d in divs:
            if d.kind == "trend":
                assert d.direction == "bottom"


# =====================================================================
# F) 空输入
# =====================================================================

class TestEmptyInputV1:
    def test_empty_everything(self):
        assert divergences_from_moves_v1([], [], [], 1) == []

    def test_no_moves(self):
        segs = [_seg(0, 0, 0, 10, "up", 20, 10)]
        assert divergences_from_moves_v1(segs, [], [], 1) == []


# =====================================================================
# T4) MACD 黄白线回拉 0 轴前提检查（beichi.md §T4）
#
# 方案3: B 段内黄白线穿越 0 轴 = has_positive AND has_negative
# 原文第25课："背驰需要多少个条件？光柱子缩短就背驰？前面的黄白线有回拉0轴吗？"
# =====================================================================

import numpy as np
import pandas as pd


def _make_df_macd(macd_values: list[float]) -> pd.DataFrame:
    """构造 MACD DataFrame，macd 列为黄白线 (DIF)，其余补零。"""
    n = len(macd_values)
    return pd.DataFrame({
        "macd": macd_values,
        "signal": [0.0] * n,
        "hist": [0.1] * n,  # 非零 hist 确保力度计算有值
    })


def _make_merged_to_raw_identity(n: int) -> list[tuple[int, int]]:
    """1:1 映射: merged bar i → raw bar (i, i)。"""
    return [(i, i) for i in range(n)]


class TestT4ZeroCrossPositive:
    """T4_zero_cross_positive: B 段 MACD 黄白线有正有负 → 满足前提 → 正常检出趋势背驰。"""

    def test_b_segment_crosses_zero_divergence_detected(self):
        """B 段（最后中枢 zs1）覆盖的 raw bar 内，MACD macd 列同时有正有负。
        力度比较 C < A → 应检出趋势背驰。

        结构: 上涨趋势，2中枢
          seg0-2  → zs0, merged bar [0..30]
          seg3-6  → A段 过渡, merged bar [30..70]
          seg7-9  → zs1 (B段), merged bar [70..100]
          seg10   → C段, merged bar [100..110]

        MACD macd 列:
          bar 70-100 (zs1范围): 交替正负 → 穿越 0 轴 ✓
        """
        segs, zss, mvs = _make_uptrend_v1()

        # 构造 111 根 bar 的 MACD 数据
        # B 段 (zs1) 覆盖 merged bar 70..100 → raw bar 70..100
        macd_vals = [0.0] * 111
        # A段之前: 正值（上涨初段）
        for i in range(0, 70):
            macd_vals[i] = 2.0
        # B段 (zs1, bar 70-100): 穿越 0 轴 — 有正有负
        for i in range(70, 101):
            macd_vals[i] = 1.5 if (i % 2 == 0) else -0.5
        # C段 (bar 100-110): 弱正值
        for i in range(101, 111):
            macd_vals[i] = 0.3

        df_macd = _make_df_macd(macd_vals)
        merged_to_raw = _make_merged_to_raw_identity(111)

        divs = divergences_from_moves_v1(
            segs, zss, mvs, level_id=1,
            df_macd=df_macd, merged_to_raw=merged_to_raw,
        )
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) >= 1
        d = trend_divs[0]
        assert d.direction == "top"
        assert d.force_c < d.force_a

    def test_no_macd_data_fallback_still_detects(self):
        """df_macd=None 时，T4 不作为前提检查（fallback 逻辑），
        力度用价格振幅，仍然检出背驰。"""
        segs, zss, mvs = _make_uptrend_v1()
        divs = divergences_from_moves_v1(
            segs, zss, mvs, level_id=1,
            df_macd=None, merged_to_raw=None,
        )
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) >= 1


class TestT4ZeroCrossNegative:
    """T4_zero_cross_negative: B 段 MACD 黄白线全正或全负 → 不满足前提 → 不产生趋势背驰。"""

    def test_b_segment_all_positive_no_divergence(self):
        """B 段 MACD macd 列全正（黄白线未回拉 0 轴）→ 前提不满足 → 无趋势背驰。

        即使 C 段力度 < A 段力度，也因 T4 前提不满足而不产生背驰。
        """
        segs, zss, mvs = _make_uptrend_v1()

        macd_vals = [0.0] * 111
        # 全程正值 — B 段 (bar 70-100) 黄白线没有回拉 0 轴
        for i in range(111):
            macd_vals[i] = 2.0 + i * 0.01  # 全正，远离 0 轴
        # C 段较弱
        for i in range(100, 111):
            macd_vals[i] = 0.1

        df_macd = _make_df_macd(macd_vals)
        merged_to_raw = _make_merged_to_raw_identity(111)

        divs = divergences_from_moves_v1(
            segs, zss, mvs, level_id=1,
            df_macd=df_macd, merged_to_raw=merged_to_raw,
        )
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) == 0

    def test_b_segment_all_negative_no_divergence(self):
        """B 段 MACD macd 列全负（下跌趋势中黄白线未回拉 0 轴）→ 无趋势背驰。"""
        segs, zss, mvs = _make_downtrend_v1()

        macd_vals = [0.0] * 111
        # 全程负值 — B 段黄白线没有回拉 0 轴
        for i in range(111):
            macd_vals[i] = -2.0 - i * 0.01  # 全负
        # C 段较弱（接近 0 但仍负）
        for i in range(100, 111):
            macd_vals[i] = -0.1

        df_macd = _make_df_macd(macd_vals)
        # hist 列也需为负（下跌看 area_neg）
        df_macd["hist"] = -0.1
        merged_to_raw = _make_merged_to_raw_identity(111)

        divs = divergences_from_moves_v1(
            segs, zss, mvs, level_id=1,
            df_macd=df_macd, merged_to_raw=merged_to_raw,
        )
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) == 0


class TestT4ZeroCrossEdge:
    """T4_zero_cross_edge: B 段 MACD 黄白线恰好触及 0 轴（有 0 值）的边界处理。

    设计决策：恰好为 0 不算穿越。穿越要求 strictly positive AND strictly negative。
    理由：0 值是黄白线恰好在 0 轴上，不算"回拉到 0 轴另一侧"。
    方案3 的精确语义: has_positive = any(v > 0), has_negative = any(v < 0)
    """

    def test_b_segment_has_zero_and_positive_only_no_cross(self):
        """B 段 MACD: [0, 0, 0.5, 1.0, 0, 0.3] — 有零和正值但无负值。
        穿越 0 轴要求有负值，此处不满足 → 无背驰。"""
        segs, zss, mvs = _make_uptrend_v1()

        macd_vals = [2.0] * 111
        # B 段 (bar 70-100): 全为 0 或正
        for i in range(70, 101):
            macd_vals[i] = 0.0 if (i % 3 == 0) else 0.5
        # C 段弱
        for i in range(100, 111):
            macd_vals[i] = 0.1

        df_macd = _make_df_macd(macd_vals)
        merged_to_raw = _make_merged_to_raw_identity(111)

        divs = divergences_from_moves_v1(
            segs, zss, mvs, level_id=1,
            df_macd=df_macd, merged_to_raw=merged_to_raw,
        )
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) == 0

    def test_b_segment_has_zero_and_negative_only_no_cross(self):
        """B 段 MACD: [0, -0.5, 0, -1.0] — 有零和负值但无正值 → 无穿越 → 无背驰。"""
        segs, zss, mvs = _make_downtrend_v1()

        macd_vals = [-2.0] * 111
        # B 段: 0 和负值，无正值
        for i in range(70, 101):
            macd_vals[i] = 0.0 if (i % 3 == 0) else -0.5

        df_macd = _make_df_macd(macd_vals)
        df_macd["hist"] = -0.1  # 下跌需负 hist
        merged_to_raw = _make_merged_to_raw_identity(111)

        divs = divergences_from_moves_v1(
            segs, zss, mvs, level_id=1,
            df_macd=df_macd, merged_to_raw=merged_to_raw,
        )
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) == 0

    def test_b_segment_has_positive_negative_and_zero_still_crosses(self):
        """B 段 MACD: [0.5, 0, -0.3, 0, 0.2] — 同时有正和负值（也有零）→ 穿越成立。"""
        segs, zss, mvs = _make_uptrend_v1()

        macd_vals = [2.0] * 111
        # B 段: 有正、有负、有零
        b_values = [0.5, 0.0, -0.3, 0.0, 0.2, -0.1, 0.8, 0.0, -0.2, 0.3]
        for i in range(70, 101):
            macd_vals[i] = b_values[(i - 70) % len(b_values)]
        # C 段弱
        for i in range(100, 111):
            macd_vals[i] = 0.1

        df_macd = _make_df_macd(macd_vals)
        merged_to_raw = _make_merged_to_raw_identity(111)

        divs = divergences_from_moves_v1(
            segs, zss, mvs, level_id=1,
            df_macd=df_macd, merged_to_raw=merged_to_raw,
        )
        trend_divs = [d for d in divs if d.kind == "trend"]
        assert len(trend_divs) >= 1

    def test_t4_only_applies_to_trend_not_consolidation(self):
        """T4 零轴穿越是趋势背驰的前提，盘整背驰不受此约束。

        盘整背驰（单中枢）即使没有 MACD 数据也可以正常检出。
        """
        segs, zss, mvs = _make_consolidation_v1()
        divs = divergences_from_moves_v1(
            segs, zss, mvs, level_id=1,
            df_macd=None, merged_to_raw=None,
        )
        cons_divs = [d for d in divs if d.kind == "consolidation"]
        assert len(cons_divs) >= 1
