"""中枢 v0 — 单元测试

严格对标缠论定理思维导图：
  A) 能生成中枢：三段有重叠，ZG/ZD 取 Z走势段
  B) 无中枢：三段无重叠
  C) 延伸：后续段与 [ZD, ZG] 重叠 → seg1 增长, sustain++
  D) 回抽确认：离开后回抽重返 → 中枢未破坏
  E) 升级：sustain >= sustain_m → settled
  F) confirmed：最后一个 False，其余 True
  G) 新字段：direction, gg, dd, g, d
  H) 断言集成
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v0 import Segment
from newchan.a_center_v0 import Center, centers_from_segments_v0
from newchan.a_assertions import (
    assert_center_definition,
    assert_non_skip,
)


# ── helper ──

def _seg(s0: int, s1: int, i0: int, i1: int, d: str,
         h: float, l: float, confirmed: bool = True) -> Segment:
    return Segment(s0=s0, s1=s1, i0=i0, i1=i1, direction=d,
                   high=h, low=l, confirmed=confirmed)


# =====================================================================
# A) 能生成中枢 — ZG/ZD 取 Z走势段
# =====================================================================

class TestCanProduceCenter:
    """三段存在重叠时，应产生中枢，ZG/ZD 由 Z走势段定义。"""

    def _make_5_segments(self) -> list[Segment]:
        """5 段，前三段价格区间重叠。

        seg0(up):   h=20, l=10
        seg1(down): h=18, l=12
        seg2(up):   h=22, l=11
        全三段: ZD_all=max(10,12,11)=12, ZG_all=min(20,18,22)=18 → 重叠
        Z走势段(s0,s2 同为up): ZD=max(10,11)=11, ZG=min(20,22)=20
        """
        return [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            _seg(6, 8, 30, 40, "down", 30.0, 25.0),   # 不重叠
            _seg(8, 10, 40, 50, "up",  35.0, 28.0),
        ]

    def test_produces_center(self):
        segs = self._make_5_segments()
        centers = centers_from_segments_v0(segs)
        assert len(centers) >= 1

    def test_center_zg_zd_uses_z_segments(self):
        """ZG/ZD 应取 Z走势段 (s0, s2)，不含中间段 s1。"""
        segs = self._make_5_segments()
        centers = centers_from_segments_v0(segs)
        c = centers[0]
        # Z走势段: s0(up h=20 l=10), s2(up h=22 l=11)
        assert c.high == 20.0   # ZG = min(20, 22) = 20
        assert c.low == 11.0    # ZD = max(10, 11) = 11

    def test_center_seg_range(self):
        segs = self._make_5_segments()
        centers = centers_from_segments_v0(segs)
        c = centers[0]
        assert c.seg0 == 0
        assert c.seg1 == 2   # 初始三段，seg3 不重叠且回抽失败 → 不延伸

    def test_center_direction(self):
        """中枢方向 = Z走势段方向 (s1 的方向)。"""
        segs = self._make_5_segments()
        centers = centers_from_segments_v0(segs)
        assert centers[0].direction == "up"


# =====================================================================
# A2) ZG/ZD 差异体现：中间段约束 vs Z走势段
# =====================================================================

class TestZSegVsAll3:
    """当中间段比 Z走势段更窄时，ZG/ZD 应不受中间段限制。"""

    def test_middle_seg_narrower_does_not_constrain(self):
        """
        s0(up): h=20, l=10
        s1(down): h=15, l=12   ← 中间段 high 偏低
        s2(up): h=18, l=11

        全三段: ZD_all=max(10,12,11)=12, ZG_all=min(20,15,18)=15 → 重叠
        Z走势段(s0,s2): ZD=max(10,11)=11, ZG=min(20,18)=18
        """
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 15.0, 12.0),
            _seg(4, 6, 20, 30, "up",   18.0, 11.0),
        ]
        centers = centers_from_segments_v0(segs)
        assert len(centers) == 1
        c = centers[0]
        assert c.high == 18.0   # ZG = min(20, 18)，不是 15
        assert c.low == 11.0    # ZD = max(10, 11)，不是 12


# =====================================================================
# B) 无中枢
# =====================================================================

class TestNoCenter:
    """三段无重叠 → 无中枢。"""

    def test_no_overlap(self):
        segs = [
            _seg(0, 2, 0, 10, "up",   10.0, 5.0),
            _seg(2, 4, 10, 20, "down", 20.0, 12.0),
            _seg(4, 6, 20, 30, "up",   30.0, 22.0),
        ]
        # ZD_all = max(5,12,22) = 22, ZG_all = min(10,20,30) = 10 → 不重叠
        centers = centers_from_segments_v0(segs)
        assert len(centers) == 0

    def test_fewer_than_3_segments(self):
        assert centers_from_segments_v0([]) == []
        assert centers_from_segments_v0([
            _seg(0, 2, 0, 10, "up", 20.0, 10.0),
        ]) == []
        assert centers_from_segments_v0([
            _seg(0, 2, 0, 10, "up", 20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
        ]) == []

    def test_boundary_equal(self):
        """ZG_all == ZD_all → 不成立（需要 strict >）。"""
        segs = [
            _seg(0, 2, 0, 10, "up",   10.0, 5.0),
            _seg(2, 4, 10, 20, "down", 15.0, 10.0),
            _seg(4, 6, 20, 30, "up",   20.0, 15.0),
        ]
        # ZD_all = max(5,10,15) = 15, ZG_all = min(10,15,20) = 10 → 不重叠
        centers = centers_from_segments_v0(segs)
        assert len(centers) == 0


# =====================================================================
# C) 延伸
# =====================================================================

class TestExtension:
    """后续段与 [ZD, ZG] 重叠 → seg1 增长, sustain++。"""

    def _make_extended(self) -> list[Segment]:
        """5 段，前三段形成中枢（ZD=11, ZG=20），seg3/seg4 继续重叠。"""
        return [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            _seg(6, 8, 30, 40, "down", 19.0, 13.0),  # overlap [13,19]∩[11,20] → yes
            _seg(8, 10, 40, 50, "up",  17.0, 11.0),  # overlap [11,17]∩[11,20] → yes
        ]

    def test_seg1_extended(self):
        segs = self._make_extended()
        centers = centers_from_segments_v0(segs)
        assert len(centers) == 1
        assert centers[0].seg1 == 4   # 延伸到 seg4

    def test_sustain_count(self):
        segs = self._make_extended()
        centers = centers_from_segments_v0(segs)
        assert centers[0].sustain == 2   # seg3 + seg4

    def test_zg_zd_unchanged_after_extension(self):
        """延伸不改变 ZG/ZD（仍以初始 Z走势段 为核）。"""
        segs = self._make_extended()
        centers = centers_from_segments_v0(segs)
        # Z走势段(s0, s2): ZD=max(10,11)=11, ZG=min(20,22)=20
        assert centers[0].high == 20.0
        assert centers[0].low == 11.0


# =====================================================================
# D) 回抽确认（中枢破坏定理）
# =====================================================================

class TestPullbackConfirmation:
    """理论：中枢被破坏，当且仅当一个次级别走势离开中枢，
    其后的次级别回抽走势不重新回到该走势中枢内。"""

    def test_exit_then_pullback_success(self):
        """离开后回抽成功 → 中枢未破坏，继续延伸。"""
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            # Z走势段: ZD=11, ZG=20
            _seg(6, 8, 30, 40, "down", 30.0, 25.0),   # EXIT: [25,30]∩[11,20] = 空
            _seg(8, 10, 40, 50, "up",  19.0, 13.0),   # PULLBACK: [13,19]∩[11,20] → 重返
            _seg(10, 12, 50, 60, "down", 40.0, 35.0),  # EXIT again
        ]
        centers = centers_from_segments_v0(segs, sustain_m=2)
        assert len(centers) == 1
        c = centers[0]
        # seg3 离开、seg4 回抽成功 → 中枢延伸到至少 seg4
        assert c.seg1 >= 4
        # sustain 包含 exit + pullback 两段
        assert c.sustain >= 2

    def test_exit_then_pullback_fail(self):
        """离开后回抽失败 → 中枢破坏。"""
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            # Z走势段: ZD=11, ZG=20
            _seg(6, 8, 30, 40, "down", 30.0, 25.0),   # EXIT
            _seg(8, 10, 40, 50, "up",  35.0, 28.0),   # PULLBACK FAIL: [28,35]∩[11,20]=空
        ]
        centers = centers_from_segments_v0(segs)
        assert len(centers) >= 1
        c = centers[0]
        assert c.seg1 == 2   # 只有初始三段

    def test_multiple_exit_pullback_cycles(self):
        """多次离开-回抽 → 中枢持续延伸。"""
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            # cycle 1: exit + pullback
            _seg(6, 8, 30, 40, "down", 30.0, 25.0),   # EXIT
            _seg(8, 10, 40, 50, "up",  19.0, 12.0),   # PULLBACK OK
            # cycle 2: exit + pullback
            _seg(10, 12, 50, 60, "down", 8.0, 5.0),   # EXIT (below)
            _seg(12, 14, 60, 70, "up",  17.0, 13.0),  # PULLBACK OK
            # final exit (no pullback)
            _seg(14, 16, 70, 80, "down", 40.0, 35.0),
        ]
        centers = centers_from_segments_v0(segs, sustain_m=2)
        assert len(centers) >= 1
        c = centers[0]
        # 经过两轮 exit-pullback，中枢延伸到 seg6
        assert c.seg1 >= 6
        # sustain = 4 (两轮 × 2段)
        assert c.sustain >= 4
        assert c.kind == "settled"


# =====================================================================
# E) 升级
# =====================================================================

class TestUpgrade:
    """sustain >= sustain_m → settled。"""

    def test_settled_default_m2(self):
        """sustain=2 >= sustain_m=2 → settled。"""
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            _seg(6, 8, 30, 40, "down", 19.0, 13.0),
            _seg(8, 10, 40, 50, "up",  17.0, 11.0),
        ]
        centers = centers_from_segments_v0(segs, sustain_m=2)
        assert centers[0].kind == "settled"

    def test_candidate_sustain_1(self):
        """sustain=1 < sustain_m=2 → candidate。"""
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            _seg(6, 8, 30, 40, "down", 19.0, 13.0),   # overlap → sustain=1
            _seg(8, 10, 40, 50, "up",  30.0, 25.0),   # no overlap & no pullback → stop
        ]
        centers = centers_from_segments_v0(segs, sustain_m=2)
        assert centers[0].kind == "candidate"
        assert centers[0].sustain == 1

    def test_custom_sustain_m(self):
        """sustain_m=1 时，sustain=1 即 settled。"""
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            _seg(6, 8, 30, 40, "down", 19.0, 13.0),
        ]
        centers = centers_from_segments_v0(segs, sustain_m=1)
        assert centers[0].kind == "settled"


# =====================================================================
# F) confirmed
# =====================================================================

class TestConfirmed:
    """最后一个中枢 confirmed=False，其余 True。"""

    def test_single_center_unconfirmed(self):
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
        ]
        centers = centers_from_segments_v0(segs)
        assert len(centers) == 1
        assert centers[0].confirmed is False

    def test_two_centers_first_confirmed(self):
        """两个中枢：第一个 True，最后一个 False。"""
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            # gap: seg3 no overlap, seg4 no pullback
            _seg(6, 8, 30, 40, "down", 50.0, 40.0),
            _seg(8, 10, 40, 50, "up",  55.0, 42.0),   # pullback fail
            # new center
            _seg(10, 12, 50, 60, "down", 48.0, 42.0),
            _seg(12, 14, 60, 70, "up",  52.0, 41.0),
            _seg(14, 16, 70, 80, "down", 50.0, 43.0),
        ]
        centers = centers_from_segments_v0(segs)
        assert len(centers) >= 2
        assert centers[0].confirmed is True
        assert centers[-1].confirmed is False


# =====================================================================
# G) 新字段 direction, gg, dd, g, d
# =====================================================================

class TestNewFields:
    """direction, gg, dd, g, d 统计量。"""

    def test_direction_from_z_segments(self):
        """中枢方向 = 初始 Z走势段的方向。"""
        segs = [
            _seg(0, 2, 0, 10, "down", 20.0, 10.0),
            _seg(2, 4, 10, 20, "up",   18.0, 12.0),
            _seg(4, 6, 20, 30, "down", 22.0, 11.0),
        ]
        centers = centers_from_segments_v0(segs)
        assert len(centers) == 1
        assert centers[0].direction == "down"

    def test_gg_dd_g_d_initial(self):
        """初始中枢的 GG/DD/G/D。"""
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
        ]
        centers = centers_from_segments_v0(segs)
        c = centers[0]
        # Z走势段: seg0(up h=20 l=10), seg2(up h=22 l=11)
        assert c.gg == 22.0   # GG = max(20, 22)
        assert c.dd == 10.0   # DD = min(10, 11)
        assert c.g == 20.0    # G = min(20, 22) = ZG
        assert c.d == 11.0    # D = max(10, 11) = ZD

    def test_gg_dd_g_d_updated_on_extension(self):
        """延伸时更新 GG/DD/G/D（仅 Z走势段 参与）。"""
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            # Z走势段 ZD=11, ZG=20
            _seg(6, 8, 30, 40, "down", 19.0, 13.0),   # overlap, direction=down → 非Z走势段
            _seg(8, 10, 40, 50, "up",  25.0, 9.0),    # overlap, direction=up → Z走势段!
        ]
        centers = centers_from_segments_v0(segs)
        assert len(centers) == 1
        c = centers[0]
        # Z走势段: seg0(h=20,l=10), seg2(h=22,l=11), seg4(h=25,l=9)
        assert c.gg == 25.0   # GG = max(20, 22, 25)
        assert c.dd == 9.0    # DD = min(10, 11, 9)
        assert c.g == 20.0    # G = min(20, 22, 25)
        assert c.d == 11.0    # D = max(10, 11, 9) = 11
        # ZG/ZD 不变
        assert c.high == 20.0
        assert c.low == 11.0


# =====================================================================
# H) 断言集成
# =====================================================================

class TestAssertIntegration:
    """assert_center_definition + assert_non_skip 集成。"""

    def test_valid_pass(self):
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
            _seg(6, 8, 30, 40, "down", 19.0, 13.0),
            _seg(8, 10, 40, 50, "up",  17.0, 11.0),
        ]
        centers = centers_from_segments_v0(segs)
        result = assert_center_definition(segs, centers, 2)
        assert result.ok is True

    def test_non_skip_pass(self):
        segs = [
            _seg(0, 2, 0, 10, "up",   20.0, 10.0),
            _seg(2, 4, 10, 20, "down", 18.0, 12.0),
            _seg(4, 6, 20, 30, "up",   22.0, 11.0),
        ]
        centers = centers_from_segments_v0(segs)
        result = assert_non_skip(segs, centers)
        assert result.ok is True

    def test_empty_pass(self):
        assert assert_center_definition([], []).ok is True
        assert assert_non_skip([], []).ok is True

    def test_bad_zg_zd_fails(self):
        """手工构造 ZG<=ZD 的 center → 断言失败。"""
        bad = [Center(seg0=0, seg1=2, low=20.0, high=10.0,
                      kind="candidate", confirmed=False, sustain=0)]
        segs = [
            _seg(0, 2, 0, 10, "up", 10.0, 5.0),
            _seg(2, 4, 10, 20, "down", 20.0, 12.0),
            _seg(4, 6, 20, 30, "up", 30.0, 22.0),
        ]
        result = assert_center_definition(segs, bad)
        assert result.ok is False
        assert "ZG" in result.message

    def test_bad_settled_sustain_fails(self):
        """kind=settled 但 sustain < sustain_m → 断言失败。"""
        bad = [Center(seg0=0, seg1=2, low=12.0, high=18.0,
                      kind="settled", confirmed=False, sustain=1)]
        segs = [_seg(i, i+2, i*10, (i+2)*10, "up", 20.0, 10.0) for i in range(3)]
        result = assert_center_definition(segs, bad, 2)
        assert result.ok is False
        assert "sustain" in result.message


# =====================================================================
# I) 向下兼容：旧的构造调用仍有效（新字段有默认值）
# =====================================================================

class TestBackwardCompat:
    """旧代码直接 Center(...) 不传新字段时不报错。"""

    def test_old_style_construction(self):
        c = Center(seg0=0, seg1=2, low=12.0, high=18.0,
                   kind="candidate", confirmed=False, sustain=0)
        assert c.direction == ""
        assert c.gg == 0.0
        assert c.dd == 0.0
        assert c.g == 0.0
        assert c.d == 0.0
