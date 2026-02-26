"""中枢扩展充要条件 — 测试用例

验证中心定理二中扩展（Expansion）条件的实现闭环。

定义依据（zhongshu.md §中枢三种发展 #3 扩展）：
  高级别中枢 ⟺ (后ZG < 前ZD ∧ 后GG ≥ 前DD) ∨ (后ZD > 前ZG ∧ 后DD ≤ 前GG)

对照（中心定理二完整三分类）：
  - 新生下跌：后GG < 前DD
  - 新生上涨：后DD > 前GG
  - 扩展：上述两条件均不满足，但波动区间有重叠

覆盖：
  1. 扩展触发（下方中枢）：后ZG < 前ZD ∧ 后GG ≥ 前DD → higher_center
  2. 扩展触发（上方中枢）：后ZD > 前ZG ∧ 后DD ≤ 前GG → higher_center
  3. 非扩展（新生上涨）：后DD > 前GG → up
  4. 非扩展（新生下跌）：后GG < 前DD → down
  5. 边界：后GG == 前DD → 扩展（弱不等式 ≥ 成立）
  6. 边界：后DD == 前GG → 扩展（弱不等式 ≤ 成立）
  7. GG/DD 正确性：zhongshu_from_segments 产出的 GG/DD 与手算一致
  8. 端到端：从线段构造两个中枢，验证扩展条件成立
  9. 端到端：从线段构造两个中枢，验证新生条件成立（非扩展）
"""

from __future__ import annotations

from typing import Literal

import pytest

from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import Zhongshu, zhongshu_from_segments
from newchan.a_center_v0 import Center
from newchan.a_trendtype_v0 import _centers_relation_by_gg_dd


# ── helpers ──


def _seg(
    idx: int, direction: str, high: float, low: float,
    confirmed: bool = True,
) -> Segment:
    s0 = idx * 3
    s1 = idx * 3 + 2
    return Segment(
        s0=s0, s1=s1, i0=s0 * 5, i1=s1 * 5,
        direction=direction, high=high, low=low,
        confirmed=confirmed,
    )


def _center(
    zd: float, zg: float, gg: float, dd: float,
    seg0: int = 0, seg1: int = 2,
) -> Center:
    """构造最小 Center 用于 _centers_relation_by_gg_dd 测试。

    Center.low = ZD, Center.high = ZG。
    """
    return Center(
        seg0=seg0, seg1=seg1,
        low=zd, high=zg,
        kind="settled", confirmed=True, sustain=0,
        gg=gg, dd=dd,
    )


def _expansion_holds(prev: Zhongshu, nxt: Zhongshu) -> bool:
    """直接用定义公式判定扩展条件。

    高级别中枢 ⟺ (后ZG < 前ZD ∧ 后GG ≥ 前DD) ∨ (后ZD > 前ZG ∧ 后DD ≤ 前GG)
    """
    cond_a = nxt.zg < prev.zd and nxt.gg >= prev.dd
    cond_b = nxt.zd > prev.zg and nxt.dd <= prev.gg
    return cond_a or cond_b


# =====================================================================
# 1) _centers_relation_by_gg_dd 单元测试
# =====================================================================


class TestCentersRelationExpansion:
    """中心定理二：扩展条件 → higher_center。"""

    def test_expansion_lower_center(self):
        """后ZG < 前ZD 且 后GG >= 前DD → higher_center。

        场景：后中枢区间在前中枢下方，但波动区间向上延伸到前中枢波动区间内。
        """
        prev = _center(zd=12, zg=18, gg=22, dd=8)
        nxt = _center(zd=3, zg=5, gg=10, dd=1, seg0=5, seg1=7)
        # 后ZG=5 < 前ZD=12 ✓; 后GG=10 >= 前DD=8 ✓
        assert _centers_relation_by_gg_dd(prev, nxt) == "higher_center"

    def test_expansion_upper_center(self):
        """后ZD > 前ZG 且 后DD <= 前GG → higher_center。

        场景：后中枢区间在前中枢上方，但波动区间向下延伸到前中枢波动区间内。
        """
        prev = _center(zd=12, zg=18, gg=22, dd=8)
        nxt = _center(zd=20, zg=25, gg=28, dd=15, seg0=5, seg1=7)
        # 后ZD=20 > 前ZG=18 ✓; 后DD=15 <= 前GG=22 ✓
        assert _centers_relation_by_gg_dd(prev, nxt) == "higher_center"


class TestCentersRelationNewborn:
    """中心定理二：新生条件 → up/down（非扩展）。"""

    def test_newborn_up(self):
        """后DD > 前GG → 上涨趋势延续。"""
        prev = _center(zd=12, zg=18, gg=22, dd=8)
        nxt = _center(zd=30, zg=35, gg=38, dd=25, seg0=5, seg1=7)
        # 后DD=25 > 前GG=22 ✓
        assert _centers_relation_by_gg_dd(prev, nxt) == "up"

    def test_newborn_down(self):
        """后GG < 前DD → 下跌趋势延续。"""
        prev = _center(zd=12, zg=18, gg=22, dd=8)
        nxt = _center(zd=3, zg=5, gg=6, dd=1, seg0=5, seg1=7)
        # 后GG=6 < 前DD=8 ✓
        assert _centers_relation_by_gg_dd(prev, nxt) == "down"


class TestCentersRelationBoundary:
    """中心定理二边界：GG/DD 精确等于对方 DD/GG。"""

    def test_gg_eq_dd_is_expansion(self):
        """后GG == 前DD → 弱不等式 后GG >= 前DD 成立 → 扩展（非新生）。

        定义：新生下跌要求 后GG < 前DD（严格），
        后GG == 前DD 时严格不等不满足，退回扩展判定。
        """
        prev = _center(zd=12, zg=18, gg=22, dd=8)
        nxt = _center(zd=3, zg=5, gg=8, dd=1, seg0=5, seg1=7)
        # 后GG=8 == 前DD=8 → 后GG < 前DD 不成立 → 不是 newborn down
        # 后ZG=5 < 前ZD=12 ✓; 后GG=8 >= 前DD=8 ✓ → expansion
        assert _centers_relation_by_gg_dd(prev, nxt) == "higher_center"

    def test_dd_eq_gg_is_expansion(self):
        """后DD == 前GG → 弱不等式 后DD <= 前GG 成立 → 扩展（非新生）。

        定义：新生上涨要求 后DD > 前GG（严格），
        后DD == 前GG 时严格不等不满足，退回扩展判定。
        """
        prev = _center(zd=12, zg=18, gg=22, dd=8)
        nxt = _center(zd=20, zg=25, gg=28, dd=22, seg0=5, seg1=7)
        # 后DD=22 == 前GG=22 → 后DD > 前GG 不成立 → 不是 newborn up
        # 后ZD=20 > 前ZG=18 ✓; 后DD=22 <= 前GG=22 ✓ → expansion
        assert _centers_relation_by_gg_dd(prev, nxt) == "higher_center"

    def test_gg_just_below_dd_is_newborn_down(self):
        """后GG < 前DD（严格）→ 新生下跌，不是扩展。"""
        prev = _center(zd=12, zg=18, gg=22, dd=8)
        nxt = _center(zd=3, zg=5, gg=7, dd=1, seg0=5, seg1=7)
        # 后GG=7 < 前DD=8 ✓ → newborn down
        assert _centers_relation_by_gg_dd(prev, nxt) == "down"

    def test_dd_just_above_gg_is_newborn_up(self):
        """后DD > 前GG（严格）→ 新生上涨，不是扩展。"""
        prev = _center(zd=12, zg=18, gg=22, dd=8)
        nxt = _center(zd=30, zg=35, gg=38, dd=23, seg0=5, seg1=7)
        # 后DD=23 > 前GG=22 ✓ → newborn up
        assert _centers_relation_by_gg_dd(prev, nxt) == "up"


# =====================================================================
# 2) GG/DD 正确性（zhongshu_from_segments 产出）
# =====================================================================


class TestGgDdCorrectness:
    """验证 zhongshu_from_segments 产出的 GG/DD 与手算一致。"""

    def test_gg_dd_basic_three_segments(self):
        """三段中枢：GG = max(highs), DD = min(lows)。"""
        segs = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 1
        zs = result[0]
        assert zs.gg == 22  # max(20, 18, 22)
        assert zs.dd == 8   # min(10, 8, 12)

    def test_gg_dd_with_extension(self):
        """延伸段更新 GG/DD（但不改 ZG/ZD）。"""
        segs = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
            _seg(3, "down", 25, 5),   # 延伸：high=25>GG, low=5<DD
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 1
        zs = result[0]
        # ZG/ZD 不变
        assert zs.zd == 12
        assert zs.zg == 18
        # GG/DD 更新
        assert zs.gg == 25  # max(20, 18, 22, 25)
        assert zs.dd == 5   # min(10, 8, 12, 5)
        assert zs.seg_count == 4

    def test_gg_dd_multiple_extensions(self):
        """多段延伸，GG/DD 持续更新。"""
        segs = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
            _seg(3, "down", 17, 13),   # 延伸，GG/DD 不变
            _seg(4, "up",   30, 14),   # 延伸，GG 更新到 30
            _seg(5, "down", 16, 6),    # 延伸，DD 更新到 6
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 1
        zs = result[0]
        assert zs.gg == 30  # max(20, 18, 22, 17, 30, 16)
        assert zs.dd == 6   # min(10, 8, 12, 13, 14, 6)
        assert zs.seg_count == 6


# =====================================================================
# 3) 端到端：两个中枢 + 扩展条件验证
# =====================================================================


class TestExpansionEndToEnd:
    """从线段构造两个中枢，验证扩展充要条件。"""

    def test_two_zhongshus_expansion_holds(self):
        """构造两个中枢，后中枢区间在前中枢下方但波动区间重叠 → 扩展。

        前中枢：seg0-2, ZD=12, ZG=18, GG=22, DD=8
        突破段：seg3 向下突破
        后中枢：seg3-5, ZD=3, ZG=5, GG=10, DD=1
        扩展条件：后ZG=5 < 前ZD=12 ∧ 后GG=10 ≥ 前DD=8 → 成立
        """
        segs = [
            # 前中枢：[10,20], [8,18], [12,22] → ZD=12, ZG=18
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
            # 突破段：high=5 < ZD=12 → 向下突破
            _seg(3, "down", 5, 1),
            # 后中枢组件（续进从 max(3-2,2)=2 开始扫描）
            # seg2=[12,22], seg3=[1,5], seg4=[3,10]
            # zd=max(12,1,3)=12, zg=min(22,5,10)=5 → 5<12 无重叠
            # seg3=[1,5], seg4=[3,10], seg5=[2,8]
            # zd=max(1,3,2)=3, zg=min(5,10,8)=5 → 5>3 成立
            _seg(4, "up",   10, 3),
            _seg(5, "down", 8, 2),
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 2, f"应产生 2 个中枢，实际 {len(result)}"

        prev, nxt = result[0], result[1]

        # 前中枢验证
        assert prev.zd == 12
        assert prev.zg == 18
        assert prev.gg == 22
        assert prev.dd == 8
        assert prev.settled is True

        # 后中枢验证
        assert nxt.zd == 3
        assert nxt.zg == 5
        assert nxt.gg == 10  # max(5, 10, 8)
        assert nxt.dd == 1   # min(1, 3, 2)

        # 扩展充要条件验证
        assert _expansion_holds(prev, nxt), (
            f"扩展条件应成立: 后ZG={nxt.zg} < 前ZD={prev.zd} ∧ "
            f"后GG={nxt.gg} >= 前DD={prev.dd}"
        )

    def test_two_zhongshus_newborn_down(self):
        """构造两个中枢，后中枢完全在前中枢下方 → 新生下跌（非扩展）。

        前中枢：seg0-2, ZD=12, ZG=18, GG=22, DD=8
        后中枢：seg3-5, GG=7 < 前DD=8 → 新生下跌
        """
        segs = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
            # 突破段：向下突破
            _seg(3, "down", 5, 1),
            # 后中枢：波动区间完全在前中枢下方
            # seg3=[1,5], seg4=[3,7], seg5=[2,6]
            # zd=max(1,3,2)=3, zg=min(5,7,6)=5 → 5>3 成立
            # GG=max(5,7,6)=7, DD=min(1,3,2)=1
            _seg(4, "up",   7, 3),
            _seg(5, "down", 6, 2),
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 2, f"应产生 2 个中枢，实际 {len(result)}"

        prev, nxt = result[0], result[1]

        # 后中枢 GG < 前中枢 DD → 新生下跌
        assert nxt.gg < prev.dd, (
            f"新生下跌条件应成立: 后GG={nxt.gg} < 前DD={prev.dd}"
        )
        # 扩展条件不成立
        assert not _expansion_holds(prev, nxt)

    def test_two_zhongshus_newborn_up(self):
        """构造两个中枢，后中枢完全在前中枢上方 → 新生上涨（非扩展）。

        前中枢：seg0-2, ZD=12, ZG=18, GG=22, DD=8
        后中枢：DD > 前GG=22 → 新生上涨
        """
        segs = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
            # 突破段：向上突破
            _seg(3, "up",   40, 25),
            # 续进从 max(3-2,2)=2 开始
            # seg2=[12,22], seg3=[25,40], seg4=[28,38]
            # zd=max(12,25,28)=28, zg=min(22,40,38)=22 → 22<28 无重叠
            # seg3=[25,40], seg4=[28,38], seg5=[30,35]
            # zd=max(25,28,30)=30, zg=min(40,38,35)=35 → 35>30 成立
            _seg(4, "down", 38, 28),
            _seg(5, "up",   35, 30),
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 2, f"应产生 2 个中枢，实际 {len(result)}"

        prev, nxt = result[0], result[1]

        # 后中枢 DD > 前中枢 GG → 新生上涨
        assert nxt.dd > prev.gg, (
            f"新生上涨条件应成立: 后DD={nxt.dd} > 前GG={prev.gg}"
        )
        assert not _expansion_holds(prev, nxt)

    def test_two_zhongshus_expansion_upper(self):
        """后中枢区间在前中枢上方但波动区间重叠 → 扩展。

        扩展条件：后ZD > 前ZG ∧ 后DD ≤ 前GG
        """
        segs = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
            # 突破段：向上突破
            _seg(3, "up",   35, 25),
            # 续进从 max(3-2,2)=2 开始
            # seg2=[12,22], seg3=[25,35], seg4=[20,32]
            # zd=max(12,25,20)=25, zg=min(22,35,32)=22 → 22<25 无重叠
            # seg3=[25,35], seg4=[20,32], seg5=[22,30]
            # zd=max(25,20,22)=25, zg=min(35,32,30)=30 → 30>25 成立
            # GG=max(35,32,30)=35, DD=min(25,20,22)=20
            _seg(4, "down", 32, 20),
            _seg(5, "up",   30, 22),
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 2, f"应产生 2 个中枢，实际 {len(result)}"

        prev, nxt = result[0], result[1]

        # 前中枢
        assert prev.zd == 12
        assert prev.zg == 18
        assert prev.gg == 22
        assert prev.dd == 8

        # 后中枢
        assert nxt.zd == 25
        assert nxt.zg == 30
        assert nxt.dd == 20  # min(25, 20, 22)
        assert nxt.gg == 35  # max(35, 32, 30)

        # 扩展条件：后ZD=25 > 前ZG=18 ∧ 后DD=20 ≤ 前GG=22
        assert _expansion_holds(prev, nxt), (
            f"扩展条件应成立: 后ZD={nxt.zd} > 前ZG={prev.zg} ∧ "
            f"后DD={nxt.dd} <= 前GG={prev.gg}"
        )


# =====================================================================
# 4) 端到端边界：GG/DD 精确等于对方 DD/GG
# =====================================================================


class TestExpansionBoundaryEndToEnd:
    """波动区间边界精确相切时的扩展判定。"""

    def test_boundary_gg_eq_dd_expansion(self):
        """后GG == 前DD → 扩展（弱不等式 ≥ 成立）。"""
        segs = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
            # 突破段
            _seg(3, "down", 5, 1),
            # 后中枢：需要 GG 恰好 == 前DD=8
            # seg3=[1,5], seg4=[3,8], seg5=[2,6]
            # zd=max(1,3,2)=3, zg=min(5,8,6)=5 → 5>3 成立
            # GG=max(5,8,6)=8, DD=min(1,3,2)=1
            _seg(4, "up",   8, 3),
            _seg(5, "down", 6, 2),
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 2

        prev, nxt = result[0], result[1]
        assert nxt.gg == prev.dd, (
            f"边界条件：后GG={nxt.gg} 应 == 前DD={prev.dd}"
        )
        # 后GG == 前DD → 后GG >= 前DD 成立 → 扩展
        assert _expansion_holds(prev, nxt)

    def test_boundary_dd_eq_gg_expansion(self):
        """后DD == 前GG → 扩展（弱不等式 ≤ 成立）。"""
        segs = [
            _seg(0, "up",   20, 10),
            _seg(1, "down", 18, 8),
            _seg(2, "up",   22, 12),
            # 突破段：向上
            _seg(3, "up",   35, 25),
            # 后中枢：需要 DD 恰好 == 前GG=22
            # seg3=[25,35], seg4=[22,32], seg5=[24,30]
            # zd=max(25,22,24)=25, zg=min(35,32,30)=30 → 30>25 成立
            # GG=max(35,32,30)=35, DD=min(25,22,24)=22
            _seg(4, "down", 32, 22),
            _seg(5, "up",   30, 24),
        ]
        result = zhongshu_from_segments(segs)
        assert len(result) == 2

        prev, nxt = result[0], result[1]
        assert nxt.dd == prev.gg, (
            f"边界条件：后DD={nxt.dd} 应 == 前GG={prev.gg}"
        )
        # 后DD == 前GG → 后DD <= 前GG 成立 → 扩展
        assert _expansion_holds(prev, nxt)


# =====================================================================
# 5) 三分类完备性：延伸 ∪ 新生 ∪ 扩展 = 全集
# =====================================================================


class TestThreeWayClassificationCompleteness:
    """验证中心定理一+二构成完备三分类。

    对任意后续走势段/中枢，必属于且仅属于以下之一：
    - 延伸（与当前中枢 [ZD,ZG] 重叠）
    - 新生（后GG < 前DD 或 后DD > 前GG）
    - 扩展（波动区间重叠但中枢区间不重叠）
    """

    @pytest.mark.parametrize(
        "prev_gg,prev_dd,nxt_gg,nxt_dd,nxt_zg,nxt_zd,expected",
        [
            # 新生下跌：后GG < 前DD
            (22, 8, 6, 1, 5, 3, "down"),
            # 新生上涨：后DD > 前GG
            (22, 8, 38, 25, 35, 30, "up"),
            # 扩展（下方）：后ZG < 前ZD ∧ 后GG >= 前DD
            (22, 8, 10, 1, 5, 3, "higher_center"),
            # 扩展（上方）：后ZD > 前ZG ∧ 后DD <= 前GG
            (22, 8, 28, 15, 25, 20, "higher_center"),
            # 边界：后GG == 前DD → 扩展
            (22, 8, 8, 1, 5, 3, "higher_center"),
            # 边界：后DD == 前GG → 扩展
            (22, 8, 28, 22, 25, 20, "higher_center"),
        ],
        ids=[
            "newborn-down",
            "newborn-up",
            "expansion-lower",
            "expansion-upper",
            "boundary-gg-eq-dd",
            "boundary-dd-eq-gg",
        ],
    )
    def test_classification(
        self,
        prev_gg: float, prev_dd: float,
        nxt_gg: float, nxt_dd: float,
        nxt_zg: float, nxt_zd: float,
        expected: str,
    ):
        prev = _center(zd=12, zg=18, gg=prev_gg, dd=prev_dd)
        nxt = _center(
            zd=nxt_zd, zg=nxt_zg, gg=nxt_gg, dd=nxt_dd,
            seg0=5, seg1=7,
        )
        assert _centers_relation_by_gg_dd(prev, nxt) == expected
