"""Type 1 买卖点 golden case 测试。

覆盖 _detect_type1 的核心检测逻辑：
- 标准买/卖点产出
- 盘整背驰跳过
- 非趋势 Move 跳过
- confirmed 语义传递
- 多背驰 → 多买卖点
- 无 Move 关联 → 跳过

定义依据: maimai_rules_v1.md, 缠论知识库 §17 第一类买卖点
"""

from dataclasses import dataclass
from typing import Literal

from newchan.a_buysellpoint_v1 import buysellpoints_from_level
from newchan.a_divergence import Divergence
from newchan.a_move_v1 import Move
from newchan.a_zhongshu_v1 import Zhongshu


# ── Segment stub ──


@dataclass
class _Seg:
    """最小化 Segment stub，只保留 BSP 检测所需字段。"""

    direction: str
    high: float
    low: float
    i0: int = 0
    i1: int = 0
    s0: int = 0
    s1: int = 0
    confirmed: bool = False


# ── 辅助构造器 ──


def _make_trend_move(
    direction: Literal["up", "down"] = "down",
    seg_start: int = 0,
    seg_end: int = 4,
    zs_start: int = 0,
    zs_end: int = 1,
    settled: bool = True,
    kind: Literal["consolidation", "trend"] = "trend",
) -> Move:
    """构造 Move，默认为 trend。"""
    return Move(
        kind=kind,
        direction=direction,
        seg_start=seg_start,
        seg_end=seg_end,
        zs_start=zs_start,
        zs_end=zs_end,
        zs_count=2,
        settled=settled,
        high=100.0,
        low=50.0,
        first_seg_s0=0,
        last_seg_s1=10,
    )


def _make_divergence(
    direction: Literal["top", "bottom"] = "bottom",
    center_idx: int = 1,
    seg_c_start: int = 0,
    seg_c_end: int = 0,
    confirmed: bool = True,
    kind: Literal["trend", "consolidation"] = "trend",
) -> Divergence:
    """构造 Divergence，默认为 trend。"""
    return Divergence(
        kind=kind,
        direction=direction,
        level_id=1,
        seg_a_start=0,
        seg_a_end=1,
        seg_c_start=seg_c_start,
        seg_c_end=seg_c_end,
        center_idx=center_idx,
        force_a=100.0,
        force_c=80.0,
        confirmed=confirmed,
    )


def _make_zhongshu(
    seg_start: int = 0,
    seg_end: int = 2,
    zd: float = 50.0,
    zg: float = 60.0,
) -> Zhongshu:
    """构造最小 Zhongshu。"""
    return Zhongshu(
        zd=zd,
        zg=zg,
        seg_start=seg_start,
        seg_end=seg_end,
        seg_count=seg_end - seg_start + 1,
        settled=True,
        break_seg=-1,
        break_direction="",
        dd=zd - 5,
        gg=zg + 5,
    )


# ═══════════════════════════════════════════════════════════
# TestType1Detection — 检测正确性
# ═══════════════════════════════════════════════════════════


class TestType1Detection:
    """Type 1 买卖点检测正确性。"""

    def test_type1_buy_standard(self) -> None:
        """标准下跌趋势背驰 -> 产生 1B。"""
        segments = [_Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5)]
        zhongshus = [_make_zhongshu(0, 2, 50.0, 60.0), _make_zhongshu(2, 4, 48.0, 58.0)]
        moves = [_make_trend_move(direction="down", zs_start=0, zs_end=1)]
        divergences = [_make_divergence(direction="bottom", center_idx=1, seg_c_end=0)]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)

        type1 = [b for b in bsps if b.kind == "type1"]
        assert len(type1) == 1
        assert type1[0].side == "buy"
        assert type1[0].seg_idx == 0
        assert type1[0].price == 45.0
        assert type1[0].confirmed is True

    def test_type1_sell_standard(self) -> None:
        """标准上涨趋势背驰 -> 产生 1S。"""
        segments = [_Seg(direction="up", high=80.0, low=60.0, i0=0, i1=5)]
        zhongshus = [_make_zhongshu(0, 2, 60.0, 70.0), _make_zhongshu(2, 4, 62.0, 72.0)]
        moves = [_make_trend_move(direction="up", zs_start=0, zs_end=1)]
        divergences = [_make_divergence(direction="top", center_idx=1, seg_c_end=0)]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)

        type1 = [b for b in bsps if b.kind == "type1"]
        assert len(type1) == 1
        assert type1[0].side == "sell"
        assert type1[0].price == 80.0

    def test_type1_skip_consolidation_divergence(self) -> None:
        """盘整背驰（div.kind='consolidation'）应跳过，无 type1 产出。"""
        segments = [_Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5)]
        zhongshus = [_make_zhongshu(0, 2, 50.0, 60.0), _make_zhongshu(2, 4, 48.0, 58.0)]
        moves = [_make_trend_move(direction="down", zs_start=0, zs_end=1)]
        divergences = [_make_divergence(direction="bottom", center_idx=1, seg_c_end=0, kind="consolidation")]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)

        type1 = [b for b in bsps if b.kind == "type1"]
        assert len(type1) == 0

    def test_type1_skip_non_trend_move(self) -> None:
        """Move.kind='consolidation' 应跳过，无 type1 产出。"""
        segments = [_Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5)]
        zhongshus = [_make_zhongshu(0, 2, 50.0, 60.0), _make_zhongshu(2, 4, 48.0, 58.0)]
        moves = [_make_trend_move(direction="down", zs_start=0, zs_end=1, kind="consolidation")]
        divergences = [_make_divergence(direction="bottom", center_idx=1, seg_c_end=0)]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)

        type1 = [b for b in bsps if b.kind == "type1"]
        assert len(type1) == 0

    def test_type1_confirmed_true(self) -> None:
        """div.confirmed=True 时 BSP.confirmed=True。"""
        segments = [_Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5)]
        zhongshus = [_make_zhongshu(0, 2, 50.0, 60.0), _make_zhongshu(2, 4, 48.0, 58.0)]
        moves = [_make_trend_move(direction="down", zs_start=0, zs_end=1)]
        divergences = [_make_divergence(direction="bottom", center_idx=1, seg_c_end=0, confirmed=True)]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)

        type1 = [b for b in bsps if b.kind == "type1"]
        assert len(type1) == 1
        assert type1[0].confirmed is True

    def test_type1_confirmed_false(self) -> None:
        """div.confirmed=False 时 BSP.confirmed=False。"""
        segments = [_Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5)]
        zhongshus = [_make_zhongshu(0, 2, 50.0, 60.0), _make_zhongshu(2, 4, 48.0, 58.0)]
        moves = [_make_trend_move(direction="down", zs_start=0, zs_end=1)]
        divergences = [_make_divergence(direction="bottom", center_idx=1, seg_c_end=0, confirmed=False)]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)

        type1 = [b for b in bsps if b.kind == "type1"]
        assert len(type1) == 1
        assert type1[0].confirmed is False


# ═══════════════════════════════════════════════════════════
# TestType1EdgeCases — 边界情况
# ═══════════════════════════════════════════════════════════


class TestType1EdgeCases:
    """Type 1 买卖点边界情况。"""

    def test_type1_multiple_divergences(self) -> None:
        """2 个 trend divergence -> 2 个 type1。"""
        segments = [
            _Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5),
            _Seg(direction="down", high=55.0, low=40.0, i0=5, i1=10),
        ]
        zhongshus = [
            _make_zhongshu(0, 2, 50.0, 60.0),
            _make_zhongshu(2, 4, 48.0, 58.0),
            _make_zhongshu(4, 6, 46.0, 56.0),
        ]
        moves = [
            _make_trend_move(direction="down", zs_start=0, zs_end=1),
            _make_trend_move(direction="down", seg_start=2, seg_end=6, zs_start=1, zs_end=2),
        ]
        divergences = [
            _make_divergence(direction="bottom", center_idx=1, seg_c_end=0),
            _make_divergence(direction="bottom", center_idx=2, seg_c_end=1),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)

        type1 = [b for b in bsps if b.kind == "type1"]
        assert len(type1) == 2

    def test_type1_no_move_association(self) -> None:
        """center_idx 对应的中枢没有被任何 Move 覆盖 -> 无 type1 产出。"""
        segments = [_Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5)]
        zhongshus = [_make_zhongshu(0, 2, 50.0, 60.0), _make_zhongshu(2, 4, 48.0, 58.0)]
        # Move 只覆盖 zs_start=0, zs_end=0，不覆盖 center_idx=1
        moves = [_make_trend_move(direction="down", zs_start=0, zs_end=0)]
        divergences = [_make_divergence(direction="bottom", center_idx=1, seg_c_end=0)]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id=1)

        type1 = [b for b in bsps if b.kind == "type1"]
        assert len(type1) == 0
