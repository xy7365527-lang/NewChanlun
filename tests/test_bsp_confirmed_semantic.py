"""maimai #2: 买卖点 confirmed 语义对齐测试。

核心主张 [旧缠论]：
    BSP.confirmed 应反映所属走势类型（Move）是否已完成，
    而非段（Segment）结构是否稳定。

Type 1 已正确（confirmed = div.confirmed = move.settled）。
Type 2 和 Type 3 当前使用 seg.confirmed，需改为 Move.settled。
"""

from __future__ import annotations

from dataclasses import dataclass

from newchan.a_buysellpoint_v1 import buysellpoints_from_level
from newchan.a_divergence import Divergence
from newchan.a_move_v1 import Move
from newchan.a_zhongshu_v1 import Zhongshu


# ── 辅助 Segment stub ──────────────────────────────────────


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
    confirmed: bool = False  # 段结构确认，与 Move.settled 无关


# ── 辅助构造器 ────────────────────────────────────────


def _make_trend_move(
    direction: str,
    seg_start: int,
    seg_end: int,
    zs_start: int,
    zs_end: int,
    settled: bool,
) -> Move:
    """构造最小 trend Move。"""
    return Move(
        kind="trend",
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
    direction: str,
    center_idx: int,
    seg_c_start: int,
    seg_c_end: int,
    confirmed: bool,
) -> Divergence:
    """构造最小 trend Divergence。"""
    return Divergence(
        kind="trend",
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
    seg_start: int,
    seg_end: int,
    zd: float,
    zg: float,
    settled: bool = True,
    break_direction: str = "",
    break_seg: int = -1,
) -> Zhongshu:
    """构造最小 Zhongshu。"""
    return Zhongshu(
        zd=zd,
        zg=zg,
        seg_start=seg_start,
        seg_end=seg_end,
        seg_count=seg_end - seg_start + 1,
        settled=settled,
        break_seg=break_seg,
        break_direction=break_direction,
        dd=zd - 5,
        gg=zg + 5,
    )


# ═══════════════════════════════════════════════════════════
# Type 2 confirmed 语义
# ═══════════════════════════════════════════════════════════


class TestType2ConfirmedSemantic:
    """Type 2 BSP confirmed 应来自 Move.settled，而非 Segment.confirmed。"""

    def _build_type2_scenario(
        self, move_settled: bool, callback_seg_confirmed: bool
    ) -> list:
        """构造 Type 2 Buy 场景。

        结构：
        - seg[0]: 下跌段 (趋势尾段，背驰段 C end)
        - seg[1]: 上涨段 (反弹)
        - seg[2]: 下跌段 (回调 = Type 2 Buy 所在段)

        Move 覆盖 seg[0..2]，settled 参数控制。
        callback_seg_confirmed 控制 seg[2].confirmed。
        """
        segments = [
            _Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5),
            _Seg(direction="up", high=65.0, low=50.0, i0=5, i1=10),
            _Seg(
                direction="down",
                high=62.0,
                low=52.0,
                i0=10,
                i1=15,
                confirmed=callback_seg_confirmed,
            ),
        ]

        zhongshus = [
            _make_zhongshu(seg_start=0, seg_end=2, zd=50.0, zg=60.0),
            _make_zhongshu(seg_start=2, seg_end=4, zd=48.0, zg=58.0),
        ]

        moves = [
            _make_trend_move(
                direction="down",
                seg_start=0,
                seg_end=2,
                zs_start=0,
                zs_end=1,
                settled=move_settled,
            ),
        ]

        divergences = [
            _make_divergence(
                direction="bottom",
                center_idx=1,
                seg_c_start=0,
                seg_c_end=0,
                confirmed=move_settled,  # div.confirmed tracks move.settled
            ),
        ]

        return segments, zhongshus, moves, divergences

    def test_type2_buy_confirmed_from_move_settled_true(self):
        """Move.settled=True, seg.confirmed=False → BSP.confirmed 应为 True。"""
        segments, zhongshus, moves, divergences = self._build_type2_scenario(
            move_settled=True, callback_seg_confirmed=False,
        )
        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)
        type2_bsps = [b for b in bsps if b.kind == "type2"]
        assert len(type2_bsps) >= 1, "应检测到 Type 2 Buy"
        # 核心断言：confirmed 来自 Move.settled，不是 seg.confirmed
        assert type2_bsps[0].confirmed is True

    def test_type2_buy_confirmed_from_move_settled_false(self):
        """Move.settled=False, seg.confirmed=True → BSP.confirmed 应为 False。"""
        segments, zhongshus, moves, divergences = self._build_type2_scenario(
            move_settled=False, callback_seg_confirmed=True,
        )
        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)
        type2_bsps = [b for b in bsps if b.kind == "type2"]
        assert len(type2_bsps) >= 1, "应检测到 Type 2 Buy"
        # 核心断言：即使段 confirmed=True，Move 未 settled → BSP 不 confirmed
        assert type2_bsps[0].confirmed is False

    def test_type2_sell_confirmed_from_move(self):
        """Type 2 Sell confirmed 同样应来自 Move.settled。"""
        # 上涨趋势场景
        segments = [
            _Seg(direction="up", high=80.0, low=60.0, i0=0, i1=5),
            _Seg(direction="down", high=75.0, low=55.0, i0=5, i1=10),
            _Seg(
                direction="up",
                high=78.0,
                low=58.0,
                i0=10,
                i1=15,
                confirmed=True,  # 段 confirmed
            ),
        ]

        zhongshus = [
            _make_zhongshu(seg_start=0, seg_end=2, zd=60.0, zg=70.0),
            _make_zhongshu(seg_start=2, seg_end=4, zd=62.0, zg=72.0),
        ]

        moves = [
            _make_trend_move(
                direction="up",
                seg_start=0,
                seg_end=2,
                zs_start=0,
                zs_end=1,
                settled=False,  # Move 未完成
            ),
        ]

        divergences = [
            _make_divergence(
                direction="top",
                center_idx=1,
                seg_c_start=0,
                seg_c_end=0,
                confirmed=False,  # tracks move.settled
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)
        type2_bsps = [b for b in bsps if b.kind == "type2"]
        assert len(type2_bsps) >= 1, "应检测到 Type 2 Sell"
        assert type2_bsps[0].confirmed is False


# ═══════════════════════════════════════════════════════════
# Type 3 confirmed 语义
# ═══════════════════════════════════════════════════════════


class TestType3ConfirmedSemantic:
    """Type 3 BSP confirmed 应来自 Move.settled，而非 Segment.confirmed。"""

    def _build_type3_scenario(
        self,
        move_settled: bool,
        pullback_seg_confirmed: bool,
        break_direction: str = "up",
    ) -> list:
        """构造 Type 3 Buy 场景。

        结构：
        - zhongshu[0]: settled, break_direction="up", break_seg=3
        - seg[3]: 上涨段（离开段）
        - seg[4]: 下跌段（回试段），low > ZG → 3B
        """
        segments = [
            _Seg(direction="down", high=60.0, low=50.0, i0=0, i1=5),
            _Seg(direction="up", high=65.0, low=55.0, i0=5, i1=10),
            _Seg(direction="down", high=63.0, low=52.0, i0=10, i1=15),
            _Seg(direction="up", high=75.0, low=62.0, i0=15, i1=20),  # 离开段
            _Seg(
                direction="down",
                high=72.0,
                low=66.0,  # low=66 > ZG=60 → 3B
                i0=20,
                i1=25,
                confirmed=pullback_seg_confirmed,
            ),
        ]

        zhongshus = [
            _make_zhongshu(
                seg_start=0,
                seg_end=2,
                zd=52.0,
                zg=60.0,
                settled=True,
                break_direction=break_direction,
                break_seg=3,
            ),
        ]

        # Move 覆盖中枢+后续段
        moves = [
            _make_trend_move(
                direction="up" if break_direction == "up" else "down",
                seg_start=0,
                seg_end=4,
                zs_start=0,
                zs_end=0,
                settled=move_settled,
            ),
        ]

        return segments, zhongshus, moves

    def test_type3_buy_confirmed_from_move_settled_true(self):
        """Move.settled=True, seg.confirmed=False → BSP.confirmed 应为 True。"""
        segments, zhongshus, moves = self._build_type3_scenario(
            move_settled=True, pullback_seg_confirmed=False,
        )
        bsps = buysellpoints_from_level(segments, zhongshus, moves, [], 1)
        type3_bsps = [b for b in bsps if b.kind == "type3"]
        assert len(type3_bsps) >= 1, "应检测到 Type 3 Buy"
        assert type3_bsps[0].confirmed is True

    def test_type3_buy_confirmed_from_move_settled_false(self):
        """Move.settled=False, seg.confirmed=True → BSP.confirmed 应为 False。"""
        segments, zhongshus, moves = self._build_type3_scenario(
            move_settled=False, pullback_seg_confirmed=True,
        )
        bsps = buysellpoints_from_level(segments, zhongshus, moves, [], 1)
        type3_bsps = [b for b in bsps if b.kind == "type3"]
        assert len(type3_bsps) >= 1, "应检测到 Type 3 Buy"
        assert type3_bsps[0].confirmed is False

    def test_type3_sell_confirmed_from_move(self):
        """Type 3 Sell confirmed 同样应来自 Move.settled。"""
        segments = [
            _Seg(direction="up", high=70.0, low=60.0, i0=0, i1=5),
            _Seg(direction="down", high=65.0, low=55.0, i0=5, i1=10),
            _Seg(direction="up", high=68.0, low=58.0, i0=10, i1=15),
            _Seg(direction="down", high=50.0, low=40.0, i0=15, i1=20),  # 离开段
            _Seg(
                direction="up",
                high=53.0,  # high=53 < ZD=55 → 3S
                low=45.0,
                i0=20,
                i1=25,
                confirmed=True,  # 段 confirmed
            ),
        ]

        zhongshus = [
            _make_zhongshu(
                seg_start=0,
                seg_end=2,
                zd=55.0,
                zg=65.0,
                settled=True,
                break_direction="down",
                break_seg=3,
            ),
        ]

        moves = [
            _make_trend_move(
                direction="down",
                seg_start=0,
                seg_end=4,
                zs_start=0,
                zs_end=0,
                settled=False,  # Move 未完成
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, [], 1)
        type3_bsps = [b for b in bsps if b.kind == "type3"]
        assert len(type3_bsps) >= 1, "应检测到 Type 3 Sell"
        assert type3_bsps[0].confirmed is False

    def test_type3_no_move_covers_fallback_false(self):
        """当没有 Move 覆盖回试段时，confirmed 应降级为 False。"""
        segments = [
            _Seg(direction="down", high=60.0, low=50.0, i0=0, i1=5),
            _Seg(direction="up", high=65.0, low=55.0, i0=5, i1=10),
            _Seg(direction="down", high=63.0, low=52.0, i0=10, i1=15),
            _Seg(direction="up", high=75.0, low=62.0, i0=15, i1=20),
            _Seg(
                direction="down",
                high=72.0,
                low=66.0,
                i0=20,
                i1=25,
                confirmed=True,  # 段自身 confirmed
            ),
        ]

        zhongshus = [
            _make_zhongshu(
                seg_start=0,
                seg_end=2,
                zd=52.0,
                zg=60.0,
                settled=True,
                break_direction="up",
                break_seg=3,
            ),
        ]

        # 没有 Move
        moves = []

        bsps = buysellpoints_from_level(segments, zhongshus, moves, [], 1)
        type3_bsps = [b for b in bsps if b.kind == "type3"]
        assert len(type3_bsps) >= 1, "应检测到 Type 3 Buy"
        # 无 Move 覆盖 → confirmed 安全降级为 False
        assert type3_bsps[0].confirmed is False


# ═══════════════════════════════════════════════════════════
# Type 1 confirmed 不变（回归保护）
# ═══════════════════════════════════════════════════════════


class TestType1ConfirmedRegression:
    """Type 1 confirmed 已正确来自 div.confirmed，验证不退化。"""

    def test_type1_confirmed_from_divergence(self):
        """Type 1 BSP.confirmed = Divergence.confirmed。"""
        segments = [
            _Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5),
        ]

        zhongshus = [
            _make_zhongshu(seg_start=0, seg_end=2, zd=50.0, zg=60.0),
            _make_zhongshu(seg_start=2, seg_end=4, zd=48.0, zg=58.0),
        ]

        moves = [
            _make_trend_move(
                direction="down",
                seg_start=0,
                seg_end=4,
                zs_start=0,
                zs_end=1,
                settled=True,
            ),
        ]

        divergences = [
            _make_divergence(
                direction="bottom",
                center_idx=1,
                seg_c_start=0,
                seg_c_end=0,
                confirmed=True,
            ),
        ]

        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)
        type1_bsps = [b for b in bsps if b.kind == "type1"]
        assert len(type1_bsps) >= 1
        assert type1_bsps[0].confirmed is True
