"""小转大检测 — 单元测试（TDD RED → GREEN）。

原文依据：
- 第43课：小级别背驰引发大级别转折的两种转折方式
- 第53课：小转大时没有本级别第一类买卖点，第二类买卖点最佳
- 第29课答疑：各级别小转大的存在使各种走势都成为可能
- 第66课：小转大一般都有一个小平台

概念溯源: [旧缠论] 第43课 + 第29课 + 第53课

测试项：
  A) 基本检测：level-1 趋势无背驰 + level-1 c段内 sub_divergence 存在 → 检出
  B) 无小转大：level-1 趋势本身有背驰 → 不检出（走正常背驰路径）
  C) 无小转大：level-1 次级别无背驰 → 不检出
  D) 方向正确：上涨趋势小转大 → sell 方向
  E) 下跌趋势小转大 → buy 方向
  F) 空输入 → 空结果
  G) 纯函数性：输入不变
"""

from __future__ import annotations

import pytest

from newchan.a_divergence import Divergence
from newchan.a_move_v1 import Move
from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import Zhongshu
from newchan.a_xiaozhuan_da import (
    XiaozhuanDa,
    detect_xiaozhuan_da,
)


# ── helpers ──


def _seg(s0: int, s1: int, i0: int, i1: int, d: str,
         h: float, l: float, confirmed: bool = True) -> Segment:
    return Segment(s0=s0, s1=s1, i0=i0, i1=i1, direction=d,
                   high=h, low=l, confirmed=confirmed)


def _zs(seg_start, seg_end, zd, zg, dd, gg, settled=True,
        break_seg=-1, break_direction="") -> Zhongshu:
    return Zhongshu(
        seg_start=seg_start, seg_end=seg_end,
        seg_count=seg_end - seg_start + 1,
        zd=zd, zg=zg, dd=dd, gg=gg,
        settled=settled,
        break_seg=break_seg, break_direction=break_direction,
        first_seg_s0=0, last_seg_s1=0,
    )


def _move(kind, direction, seg_start, seg_end,
          zs_start, zs_end, zs_count,
          settled=False, high=0.0, low=0.0) -> Move:
    return Move(
        kind=kind, direction=direction,
        seg_start=seg_start, seg_end=seg_end,
        zs_start=zs_start, zs_end=zs_end,
        zs_count=zs_count, settled=settled,
        high=high, low=low,
    )


def _divergence(kind, direction, level_id,
                seg_a_start, seg_a_end,
                seg_c_start, seg_c_end,
                center_idx, force_a, force_c,
                confirmed=True) -> Divergence:
    return Divergence(
        kind=kind, direction=direction, level_id=level_id,
        seg_a_start=seg_a_start, seg_a_end=seg_a_end,
        seg_c_start=seg_c_start, seg_c_end=seg_c_end,
        center_idx=center_idx,
        force_a=force_a, force_c=force_c,
        confirmed=confirmed,
    )


def _make_uptrend_no_beichi():
    """构造 level-1 上涨趋势，本级别无背驰。

    结构：
      seg0-2  → zhongshu0 [ZD=12,ZG=18], GG=20, DD=10
      seg3-5  → 过渡段（A 段）
      seg6-8  → zhongshu1 [ZD=28,ZG=32], GG=34, DD=26
      seg9-10 → C 段（力度大: 不背驰）

    上涨判定: zhongshu1.dd(26) > zhongshu0.gg(20) → up
    """
    segments = [
        _seg(0, 0, 0, 10, "up", 20, 10),       # seg0 (zs0)
        _seg(1, 1, 11, 20, "down", 18, 12),     # seg1 (zs0)
        _seg(2, 2, 21, 30, "up", 19, 13),       # seg2 (zs0)
        _seg(3, 3, 31, 40, "down", 16, 11),     # seg3 (A段)
        _seg(4, 4, 41, 50, "up", 25, 15),       # seg4 (A段)
        _seg(5, 5, 51, 60, "down", 22, 20),     # seg5 (A段)
        _seg(6, 6, 61, 70, "up", 34, 26),       # seg6 (zs1)
        _seg(7, 7, 71, 80, "down", 32, 28),     # seg7 (zs1)
        _seg(8, 8, 81, 90, "up", 33, 29),       # seg8 (zs1)
        _seg(9, 9, 91, 100, "down", 30, 27),    # seg9 (C段)
        _seg(10, 10, 101, 120, "up", 42, 28, confirmed=False),  # seg10 (C段，力度大)
    ]

    zhongshus = [
        _zs(0, 2, 12, 18, 10, 20, settled=True),
        _zs(6, 8, 28, 32, 26, 34, settled=True),
    ]

    # 趋势 Move: 2 中枢, 上涨, 未 settled（最后一个）
    moves = [
        _move("trend", "up", 0, 10, 0, 1, 2, settled=False, high=42, low=10),
    ]

    return segments, zhongshus, moves


class TestDetectXiaozhuanDa:
    """基本小转大检测。"""

    def test_basic_detection(self):
        """A) level-1 趋势无背驰 + c段内有次级别背驰 → 检出小转大。"""
        segments, zhongshus, moves = _make_uptrend_no_beichi()

        # 本级别无背驰
        level1_divergences: list[Divergence] = []

        # 次级别在 c 段范围内有背驰
        # c 段 = seg9-10, bar范围 [91, 120]
        # 次级别背驰的 seg_c_end=10 → seg10.i1=120，落在 c 段内
        sub_divergence = _divergence(
            "trend", "top", 0,  # level_id=0 表示次级别
            seg_a_start=4, seg_a_end=5,
            seg_c_start=9, seg_c_end=10,
            center_idx=1, force_a=100.0, force_c=50.0,
        )

        results = detect_xiaozhuan_da(
            segments=segments,
            zhongshus=zhongshus,
            moves=moves,
            level_divergences=level1_divergences,
            sub_divergences=[sub_divergence],
            level_id=1,
        )

        assert len(results) == 1
        xzd = results[0]
        assert isinstance(xzd, XiaozhuanDa)
        assert xzd.level_id == 1
        assert xzd.side == "sell"  # 上涨趋势的小转大 → 卖出信号
        assert xzd.move_seg_start == 0
        assert xzd.sub_divergence is sub_divergence

    def test_no_xiaozhuan_when_level_has_beichi(self):
        """B) 本级别趋势有背驰 → 走正常背驰路径，不检出小转大。"""
        segments, zhongshus, moves = _make_uptrend_no_beichi()

        # 本级别有背驰
        level1_div = _divergence(
            "trend", "top", 1,
            seg_a_start=3, seg_a_end=5,
            seg_c_start=9, seg_c_end=10,
            center_idx=1, force_a=100.0, force_c=50.0,
        )

        sub_divergence = _divergence(
            "trend", "top", 0,
            seg_a_start=4, seg_a_end=5,
            seg_c_start=7, seg_c_end=8,
            center_idx=1, force_a=100.0, force_c=50.0,
        )

        results = detect_xiaozhuan_da(
            segments=segments,
            zhongshus=zhongshus,
            moves=moves,
            level_divergences=[level1_div],
            sub_divergences=[sub_divergence],
            level_id=1,
        )

        assert len(results) == 0

    def test_no_xiaozhuan_when_no_sub_divergence(self):
        """C) 本级别无背驰 + 次级别也无背驰 → 不检出。"""
        segments, zhongshus, moves = _make_uptrend_no_beichi()

        results = detect_xiaozhuan_da(
            segments=segments,
            zhongshus=zhongshus,
            moves=moves,
            level_divergences=[],
            sub_divergences=[],
            level_id=1,
        )

        assert len(results) == 0

    def test_direction_sell_for_uptrend(self):
        """D) 上涨趋势的小转大 → side="sell"。"""
        segments, zhongshus, moves = _make_uptrend_no_beichi()

        sub_div = _divergence(
            "trend", "top", 0,
            seg_a_start=4, seg_a_end=5,
            seg_c_start=9, seg_c_end=10,
            center_idx=1, force_a=100.0, force_c=50.0,
        )

        results = detect_xiaozhuan_da(
            segments=segments,
            zhongshus=zhongshus,
            moves=moves,
            level_divergences=[],
            sub_divergences=[sub_div],
            level_id=1,
        )

        assert len(results) == 1
        assert results[0].side == "sell"

    def test_direction_buy_for_downtrend(self):
        """E) 下跌趋势的小转大 → side="buy"。"""
        segments = [
            _seg(0, 0, 0, 10, "down", 20, 10),
            _seg(1, 1, 11, 20, "up", 18, 12),
            _seg(2, 2, 21, 30, "down", 19, 8),
            _seg(3, 3, 31, 40, "up", 16, 11),
            _seg(4, 4, 41, 50, "down", 10, 5),
            _seg(5, 5, 51, 60, "up", 8, 4),
            _seg(6, 6, 61, 70, "down", 6, 1),
            _seg(7, 7, 71, 80, "up", 4, 2),
            _seg(8, 8, 81, 90, "down", 3, 0.5),
            _seg(9, 9, 91, 100, "up", 2, 1),
            _seg(10, 10, 101, 120, "down", 1.5, -2, confirmed=False),
        ]

        zhongshus = [
            _zs(0, 2, 12, 18, 8, 20, settled=True),   # 高位中枢
            _zs(6, 8, 2, 4, 0.5, 6, settled=True),      # 低位中枢
        ]

        # 下跌趋势: zs1.gg(6) < zs0.dd(8)
        moves = [
            _move("trend", "down", 0, 10, 0, 1, 2, settled=False, high=20, low=-2),
        ]

        sub_div = _divergence(
            "trend", "bottom", 0,
            seg_a_start=4, seg_a_end=5,
            seg_c_start=9, seg_c_end=10,
            center_idx=1, force_a=100.0, force_c=50.0,
        )

        results = detect_xiaozhuan_da(
            segments=segments,
            zhongshus=zhongshus,
            moves=moves,
            level_divergences=[],
            sub_divergences=[sub_div],
            level_id=1,
        )

        assert len(results) == 1
        assert results[0].side == "buy"

    def test_empty_input(self):
        """F) 空输入 → 空结果。"""
        results = detect_xiaozhuan_da(
            segments=[],
            zhongshus=[],
            moves=[],
            level_divergences=[],
            sub_divergences=[],
            level_id=1,
        )
        assert results == []

    def test_pure_function(self):
        """G) 纯函数性：输入不变。"""
        segments, zhongshus, moves = _make_uptrend_no_beichi()
        level_divs: list[Divergence] = []
        sub_div = _divergence(
            "trend", "top", 0,
            seg_a_start=4, seg_a_end=5,
            seg_c_start=9, seg_c_end=10,
            center_idx=1, force_a=100.0, force_c=50.0,
        )
        sub_divs = [sub_div]

        # 保存输入快照
        segs_before = list(segments)
        zs_before = list(zhongshus)
        moves_before = list(moves)

        detect_xiaozhuan_da(
            segments=segments,
            zhongshus=zhongshus,
            moves=moves,
            level_divergences=level_divs,
            sub_divergences=sub_divs,
            level_id=1,
        )

        assert segments == segs_before
        assert zhongshus == zs_before
        assert moves == moves_before


class TestXiaozhuanDaEdgeCases:
    """边界条件。"""

    def test_consolidation_move_not_detected(self):
        """盘整走势（只有1个中枢）不产生小转大。

        小转大的前提是趋势走势（≥2中枢），盘整走势中
        没有"c段"概念。
        """
        segments = [
            _seg(0, 0, 0, 10, "up", 20, 10),
            _seg(1, 1, 11, 20, "down", 18, 12),
            _seg(2, 2, 21, 30, "up", 19, 13),
            _seg(3, 3, 31, 40, "down", 15, 11),
            _seg(4, 4, 41, 50, "up", 22, 14, confirmed=False),
        ]

        zhongshus = [
            _zs(0, 2, 12, 18, 10, 20, settled=True,
                break_seg=3, break_direction="up"),
        ]

        moves = [
            _move("consolidation", "up", 0, 4, 0, 0, 1, settled=False),
        ]

        sub_div = _divergence(
            "consolidation", "top", 0,
            seg_a_start=0, seg_a_end=0,
            seg_c_start=2, seg_c_end=2,
            center_idx=0, force_a=100.0, force_c=50.0,
        )

        results = detect_xiaozhuan_da(
            segments=segments,
            zhongshus=zhongshus,
            moves=moves,
            level_divergences=[],
            sub_divergences=[sub_div],
            level_id=1,
        )

        assert len(results) == 0

    def test_settled_move_not_detected(self):
        """已 settled 的 Move 不产生小转大。

        小转大是"进行中"的走势判断，settled 意味着走势已完成，
        完成方式已确定（正常背驰或其他），不再需要小转大检测。
        """
        segments, zhongshus, moves = _make_uptrend_no_beichi()
        # 将 move 改为 settled
        from dataclasses import replace
        moves = [replace(moves[0], settled=True)]

        sub_div = _divergence(
            "trend", "top", 0,
            seg_a_start=4, seg_a_end=5,
            seg_c_start=7, seg_c_end=8,
            center_idx=1, force_a=100.0, force_c=50.0,
        )

        results = detect_xiaozhuan_da(
            segments=segments,
            zhongshus=zhongshus,
            moves=moves,
            level_divergences=[],
            sub_divergences=[sub_div],
            level_id=1,
        )

        assert len(results) == 0

    def test_c_segment_bar_range(self):
        """检验 XiaozhuanDa 的 c_seg_start 和 c_seg_end 字段。"""
        segments, zhongshus, moves = _make_uptrend_no_beichi()

        sub_div = _divergence(
            "trend", "top", 0,
            seg_a_start=4, seg_a_end=5,
            seg_c_start=9, seg_c_end=10,
            center_idx=1, force_a=100.0, force_c=50.0,
        )

        results = detect_xiaozhuan_da(
            segments=segments,
            zhongshus=zhongshus,
            moves=moves,
            level_divergences=[],
            sub_divergences=[sub_div],
            level_id=1,
        )

        assert len(results) == 1
        xzd = results[0]
        # C 段 = 最后中枢之后到 Move 终点
        # zhongshu1.seg_end = 8, move.seg_end = 10
        assert xzd.c_seg_start == 9   # zs_last.seg_end + 1
        assert xzd.c_seg_end == 10     # move.seg_end

    def test_sub_divergence_in_c_range_filtering(self):
        """次级别背驰必须在 c 段 bar 范围内才能触发小转大。

        如果次级别背驰发生在 a 段或 b 段，不算小转大。
        """
        segments, zhongshus, moves = _make_uptrend_no_beichi()

        # 这个次级别背驰的 seg_c_end 对应的 bar 范围在 A 段
        # seg5 的 i1 = 60，c 段起点 seg9 的 i0 = 91
        # 所以这个背驰在 c 段之前，不应触发小转大
        sub_div_outside = _divergence(
            "trend", "top", 0,
            seg_a_start=0, seg_a_end=1,
            seg_c_start=3, seg_c_end=4,  # seg4.i1 = 50, 在 c 段 [91,120] 之外
            center_idx=0, force_a=100.0, force_c=50.0,
        )

        results = detect_xiaozhuan_da(
            segments=segments,
            zhongshus=zhongshus,
            moves=moves,
            level_divergences=[],
            sub_divergences=[sub_div_outside],
            level_id=1,
        )

        assert len(results) == 0
