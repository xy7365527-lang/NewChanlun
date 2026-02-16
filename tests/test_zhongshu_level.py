"""泛化中枢构造 (LevelZhongshu) — 单元测试

覆盖：
  1.  基本三组件中枢
  2.  无重叠 → 空列表
  3.  中枢延伸（4组件）
  4.  突破（4组件，第4个不重叠）
  5.  两个中枢
  6.  续进策略（从 break_comp - 2 开始）
  7.  过滤 completed=False
  8.  空输入
  9.  交叉验证：与 zhongshu_from_segments() 对比
  10. moves_from_level_zhongshus 基本：1中枢 → consolidation
  11. moves_from_level_zhongshus 趋势：2 ascending 中枢 → uptrend
  12. moves_from_level_zhongshus 最后 move unsettled
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

import pytest

from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import Zhongshu, zhongshu_from_segments
from newchan.a_level_protocol import (
    MoveProtocol,
    SegmentAsComponent,
    adapt_segments,
)
from newchan.a_zhongshu_level import (
    LevelZhongshu,
    zhongshu_from_components,
    moves_from_level_zhongshus,
)


# ── helpers ──


@dataclass(frozen=True, slots=True)
class FakeComponent:
    """用于测试的 MoveProtocol 实现。"""

    component_idx: int
    high: float
    low: float
    direction: Literal["up", "down"]
    completed: bool
    level_id: int = 0


def _comp(
    idx: int,
    high: float,
    low: float,
    direction: str = "up",
    completed: bool = True,
    level_id: int = 0,
) -> FakeComponent:
    return FakeComponent(
        component_idx=idx,
        high=high,
        low=low,
        direction=direction,
        completed=completed,
        level_id=level_id,
    )


def _seg(
    s0: int,
    s1: int,
    direction: str,
    high: float,
    low: float,
    confirmed: bool = True,
    kind: str = "settled",
) -> Segment:
    return Segment(
        s0=s0, s1=s1, i0=s0 * 5, i1=s1 * 5,
        direction=direction, high=high, low=low,
        confirmed=confirmed, kind=kind,
    )


# ====================================================================
# zhongshu_from_components 测试
# ====================================================================


class TestZhongshuFromComponents:
    """zhongshu_from_components 算法测试。"""

    def test_basic_three_component_zhongshu(self) -> None:
        """3个重叠组件 → 1个中枢。"""
        comps = [
            _comp(0, high=20.0, low=10.0, direction="up"),
            _comp(1, high=18.0, low=8.0, direction="down"),
            _comp(2, high=22.0, low=12.0, direction="up"),
        ]
        result = zhongshu_from_components(comps)

        assert len(result) == 1
        zs = result[0]
        assert zs.zd == 12.0  # max(10, 8, 12)
        assert zs.zg == 18.0  # min(20, 18, 22)
        assert zs.comp_start == 0
        assert zs.comp_end == 2
        assert zs.comp_count == 3
        assert zs.settled is False  # 没有突破组件
        assert zs.gg == 22.0  # max(20, 18, 22)
        assert zs.dd == 8.0  # min(10, 8, 12)
        assert zs.level_id == 1  # 组件 level_id=0，中枢 +1

    def test_no_overlap(self) -> None:
        """3个无重叠组件 → 空列表。"""
        comps = [
            _comp(0, high=10.0, low=5.0, direction="up"),
            _comp(1, high=25.0, low=15.0, direction="down"),
            _comp(2, high=40.0, low=30.0, direction="up"),
        ]
        result = zhongshu_from_components(comps)
        assert result == []

    def test_extension(self) -> None:
        """4个组件，第4个与[ZD,ZG]重叠 → 中枢延伸到4组件。"""
        comps = [
            _comp(0, high=20.0, low=10.0, direction="up"),
            _comp(1, high=18.0, low=8.0, direction="down"),
            _comp(2, high=22.0, low=12.0, direction="up"),
            _comp(3, high=19.0, low=11.0, direction="down"),  # 与 [12, 18] 重叠
        ]
        result = zhongshu_from_components(comps)

        assert len(result) == 1
        zs = result[0]
        assert zs.comp_count == 4
        assert zs.comp_end == 3
        assert zs.gg == 22.0  # max(20, 18, 22, 19)
        assert zs.dd == 8.0  # min(10, 8, 12, 11)
        assert zs.settled is False

    def test_breakthrough(self) -> None:
        """4个组件，第4个不重叠 → 中枢 settled + break。"""
        comps = [
            _comp(0, high=20.0, low=10.0, direction="up"),
            _comp(1, high=18.0, low=8.0, direction="down"),
            _comp(2, high=22.0, low=12.0, direction="up"),
            _comp(3, high=40.0, low=25.0, direction="up"),  # 突破上方
        ]
        result = zhongshu_from_components(comps)

        assert len(result) == 1
        zs = result[0]
        assert zs.comp_count == 3
        assert zs.settled is True
        assert zs.break_comp == 3
        assert zs.break_direction == "up"

    def test_two_zhongshus(self) -> None:
        """足够组件产生2个中枢。"""
        # 第一个中枢：idx 0-2，重叠区 [12, 18]；idx 3 突破上方
        # 第二个中枢：续进从 idx 1 开始（max(3-2, 2) = 2 → 实际从 completed[2] 开始）
        # 需要更多组件来确保两个中枢
        comps = [
            # 第一个中枢：0, 1, 2 重叠
            _comp(0, high=20.0, low=10.0, direction="up"),
            _comp(1, high=18.0, low=8.0, direction="down"),
            _comp(2, high=22.0, low=12.0, direction="up"),
            # 突破
            _comp(3, high=40.0, low=25.0, direction="up"),
            # 第二个中枢：需要从续进点开始的三个重叠组件
            _comp(4, high=38.0, low=28.0, direction="down"),
            _comp(5, high=42.0, low=30.0, direction="up"),
            _comp(6, high=36.0, low=26.0, direction="down"),
        ]
        result = zhongshu_from_components(comps)

        assert len(result) >= 2
        # 第一个中枢
        assert result[0].comp_start == 0
        assert result[0].settled is True
        # 第二个中枢存在
        assert result[1].comp_count >= 3

    def test_continuation_from_break(self) -> None:
        """续进策略：从 break 位置 - 2 开始扫描下一个中枢。

        设计：
        - 中枢 1: comp 0,1,2 重叠在 [12, 18]
        - comp 3 突破上方 (break at offset 3)
        - 续进: max(3-2, 2) = 2, 从 offset 2 开始
        - 中枢 2: comp 2,3,4 重叠在 [25, 35]
        """
        comps = [
            _comp(0, high=20.0, low=10.0, direction="up"),
            _comp(1, high=18.0, low=8.0, direction="down"),
            _comp(2, high=35.0, low=12.0, direction="up"),  # 跨度大
            _comp(3, high=40.0, low=25.0, direction="up"),   # 突破 zg=18 上方，但与 comp[2] 重叠
            _comp(4, high=38.0, low=28.0, direction="down"),  # 与 comp[2,3] 重叠
        ]
        result = zhongshu_from_components(comps)

        # 至少产生 2 个中枢
        assert len(result) >= 2
        # 第二个中枢应从续进点开始
        assert result[1].comp_start <= result[0].break_comp

    def test_filter_incomplete(self) -> None:
        """completed=False 的组件被过滤。"""
        comps = [
            _comp(0, high=20.0, low=10.0, direction="up", completed=True),
            _comp(1, high=18.0, low=8.0, direction="down", completed=True),
            _comp(2, high=22.0, low=12.0, direction="up", completed=False),  # 被过滤
        ]
        result = zhongshu_from_components(comps)
        assert result == []  # 只有 2 个 completed，不够形成中枢

    def test_empty_input(self) -> None:
        """空列表 → 空结果。"""
        assert zhongshu_from_components([]) == []


# ====================================================================
# 交叉验证
# ====================================================================


class TestCrossValidation:
    """用相同数据对比 zhongshu_from_segments() 和 zhongshu_from_components()。"""

    def test_cross_validation_with_segments(self) -> None:
        """确保两个函数在相同输入下产生一致的 ZD/ZG/settled/break_direction。"""
        segments = [
            _seg(0, 2, "up", high=20.0, low=10.0),
            _seg(2, 4, "down", high=18.0, low=8.0),
            _seg(4, 6, "up", high=22.0, low=12.0),
            _seg(6, 8, "down", high=40.0, low=25.0),  # 突破
            _seg(8, 10, "up", high=38.0, low=28.0),
            _seg(10, 12, "down", high=36.0, low=26.0),
            _seg(12, 14, "up", high=42.0, low=30.0),
        ]

        # 原始方法
        old_result = zhongshu_from_segments(segments)
        # 泛化方法
        comps = adapt_segments(segments)
        new_result = zhongshu_from_components(comps)

        assert len(old_result) == len(new_result), (
            f"中枢数量不一致: old={len(old_result)}, new={len(new_result)}"
        )

        for i, (old, new) in enumerate(zip(old_result, new_result)):
            assert old.zd == new.zd, f"中枢 {i}: ZD 不一致"
            assert old.zg == new.zg, f"中枢 {i}: ZG 不一致"
            assert old.settled == new.settled, f"中枢 {i}: settled 不一致"
            assert old.break_direction == new.break_direction, (
                f"中枢 {i}: break_direction 不一致"
            )
            assert old.gg == new.gg, f"中枢 {i}: GG 不一致"
            assert old.dd == new.dd, f"中枢 {i}: DD 不一致"
            assert old.seg_count == new.comp_count, (
                f"中枢 {i}: count 不一致"
            )


# ====================================================================
# moves_from_level_zhongshus 测试
# ====================================================================


class TestMovesFromLevelZhongshus:
    """moves_from_level_zhongshus 算法测试。"""

    def test_moves_from_level_zhongshus_basic(self) -> None:
        """1个 settled 中枢 → 1个 consolidation move。"""
        zhongshus = [
            LevelZhongshu(
                zd=12.0, zg=18.0,
                comp_start=0, comp_end=2, comp_count=3,
                settled=True,
                break_comp=3, break_direction="up",
                gg=22.0, dd=8.0, level_id=1,
            ),
        ]
        result = moves_from_level_zhongshus(zhongshus)

        assert len(result) == 1
        m = result[0]
        assert m.kind == "consolidation"
        assert m.direction == "up"  # 从 break_direction 来
        assert m.zs_count == 1
        assert m.seg_start == 0  # comp_start
        assert m.seg_end == 2  # comp_end
        # 最后一个 move → unsettled
        assert m.settled is False

    def test_moves_from_level_zhongshus_trend(self) -> None:
        """2个 ascending settled 中枢 → 1个 uptrend move。"""
        zhongshus = [
            LevelZhongshu(
                zd=10.0, zg=15.0,
                comp_start=0, comp_end=2, comp_count=3,
                settled=True,
                break_comp=3, break_direction="up",
                gg=18.0, dd=5.0, level_id=1,  # GG=18
            ),
            LevelZhongshu(
                zd=25.0, zg=30.0,
                comp_start=3, comp_end=5, comp_count=3,
                settled=True,
                break_comp=6, break_direction="up",
                gg=35.0, dd=20.0, level_id=1,  # DD=20 > GG=18 → ascending
            ),
        ]
        result = moves_from_level_zhongshus(zhongshus)

        assert len(result) == 1
        m = result[0]
        assert m.kind == "trend"
        assert m.direction == "up"
        assert m.zs_count == 2
        assert m.high == 30.0  # max(zg: 15, 30)
        assert m.low == 10.0  # min(zd: 10, 25)

    def test_moves_from_level_zhongshus_last_unsettled(self) -> None:
        """最后一个 move 的 settled 必须为 False。"""
        zhongshus = [
            LevelZhongshu(
                zd=10.0, zg=15.0,
                comp_start=0, comp_end=2, comp_count=3,
                settled=True,
                break_comp=3, break_direction="up",
                gg=18.0, dd=5.0, level_id=1,
            ),
            LevelZhongshu(
                zd=20.0, zg=25.0,
                comp_start=3, comp_end=5, comp_count=3,
                settled=True,
                break_comp=6, break_direction="down",
                gg=28.0, dd=15.0, level_id=1,
            ),
        ]
        result = moves_from_level_zhongshus(zhongshus)
        assert len(result) >= 1
        # 最后一个 move 必须 unsettled
        assert result[-1].settled is False
        # 第一个之前的都是 settled（如果有多个 move 的话）
        for m in result[:-1]:
            assert m.settled is True
