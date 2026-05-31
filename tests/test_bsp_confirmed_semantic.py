"""买卖点 confirmed / settled 语义测试（confirmed-fix，maimai #2 v0.7 翻转）。

核心主张 [旧缠论]（修订后）：
    BSP.confirmed = 买卖点**自身**的确认条件（可操作信号），与走势是否完成无关。
    BSP.settled   = 覆盖该买卖点的走势类型（Move）是否已完成（事后验证）。

旧设计（maimai #2）将 confirmed 绑定到 Move.settled，导致走势末端（实盘当下）
的所有买卖点 confirmed 恒为 False —— 买卖点的目的恰恰是在走势完成**之前**入场。
本次修订将两个概念解耦：

- Type1（第17/24课）：candidate = 背驰存在（面积开始缩小）；
  confirmed = 面积比 force_c/force_a ≤ TYPE1_CONFIRM_RATIO（面积比低于阈值）。
- Type2（第17/21课）：candidate = confirmed = 次级别回调/反弹不创新极值（同步）。
- Type3（第20课）：candidate = 中枢突破后回试不破边界；
  confirmed = 回试段之后出现 break_direction 方向的延续段（回试结束、走势延续）。
- 三者 settled 一律 = 覆盖段的 Move.settled（无 Move 覆盖时降级 False）。

谱系：candidate-fix（candidate/confirmed 时间分离）；confirmed-fix（maimai #2 翻转）；
005b 对象否定对象。
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
    confirmed: bool = False  # 段结构确认，与 BSP confirmed/settled 均无关


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
    confirmed: bool = True,
) -> Divergence:
    """构造最小 trend Divergence（confirmed 现恒为 True：背驰存在即确认）。"""
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
# Type 2 confirmed / settled 语义
# ═══════════════════════════════════════════════════════════


class TestType2ConfirmedSemantic:
    """Type 2: confirmed = 回调/反弹不创新极值；settled = Move.settled。"""

    def _build_type2_buy_scenario(
        self, move_settled: bool, callback_low: float,
    ) -> tuple[list, list, list, list]:
        """构造 Type 2 Buy 场景。

        结构：
        - seg[0]: 下跌段（趋势尾段，背驰段 C end）→ 1B，price = seg[0].low = 45
        - seg[1]: 上涨段（反弹）
        - seg[2]: 下跌段（回调 = 2B 所在段），callback_low 控制是否创新低

        callback_low > 45 → 不创新低 → 2B confirmed=True；否则 confirmed=False。
        move_settled 控制 settled（与 confirmed 解耦）。
        """
        segments = [
            _Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5),
            _Seg(direction="up", high=65.0, low=50.0, i0=5, i1=10),
            _Seg(direction="down", high=62.0, low=callback_low, i0=10, i1=15),
        ]
        zhongshus = [
            _make_zhongshu(seg_start=0, seg_end=2, zd=50.0, zg=60.0),
            _make_zhongshu(seg_start=2, seg_end=4, zd=48.0, zg=58.0),
        ]
        moves = [
            _make_trend_move(
                direction="down", seg_start=0, seg_end=2,
                zs_start=0, zs_end=1, settled=move_settled,
            ),
        ]
        divergences = [
            _make_divergence(
                direction="bottom", center_idx=1,
                seg_c_start=0, seg_c_end=0,
            ),
        ]
        return segments, zhongshus, moves, divergences

    def test_type2_buy_confirmed_decoupled_from_settled(self):
        """回调不创新低 → confirmed=True，即使走势未完成（move.settled=False）。"""
        segments, zhongshus, moves, divergences = self._build_type2_buy_scenario(
            move_settled=False, callback_low=52.0,  # 52 > 45 → 不创新低
        )
        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)
        type2 = [b for b in bsps if b.kind == "type2"]
        assert len(type2) == 1
        # 核心：confirmed 来自买卖点自身条件（不创新低），与 Move.settled 解耦
        assert type2[0].confirmed is True
        assert type2[0].settled is False  # 走势未完成

    def test_type2_buy_settled_tracks_move(self):
        """走势完成 → settled=True；confirmed 同样 True。"""
        segments, zhongshus, moves, divergences = self._build_type2_buy_scenario(
            move_settled=True, callback_low=52.0,
        )
        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)
        type2 = [b for b in bsps if b.kind == "type2"]
        assert len(type2) == 1
        assert type2[0].confirmed is True
        assert type2[0].settled is True

    def test_type2_buy_unconfirmed_when_new_low(self):
        """回调创新低（low < 1B low）→ confirmed=False（候选但未确认）。"""
        segments, zhongshus, moves, divergences = self._build_type2_buy_scenario(
            move_settled=True, callback_low=40.0,  # 40 < 45 → 创新低
        )
        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)
        type2 = [b for b in bsps if b.kind == "type2"]
        assert len(type2) == 1
        # 创新低 → 不满足"不创新低"确认条件
        assert type2[0].confirmed is False
        # settled 仍跟随 Move（走势完成验证独立于 confirmed）
        assert type2[0].settled is True

    def test_type2_sell_confirmed_decoupled_from_settled(self):
        """2S：反弹不创新高 → confirmed=True，settled 跟随 Move（此处未完成）。"""
        segments = [
            _Seg(direction="up", high=80.0, low=60.0, i0=0, i1=5),
            _Seg(direction="down", high=75.0, low=55.0, i0=5, i1=10),
            _Seg(direction="up", high=78.0, low=58.0, i0=10, i1=15),  # high 78 < 1S high 80
        ]
        zhongshus = [
            _make_zhongshu(seg_start=0, seg_end=2, zd=60.0, zg=70.0),
            _make_zhongshu(seg_start=2, seg_end=4, zd=62.0, zg=72.0),
        ]
        moves = [
            _make_trend_move(
                direction="up", seg_start=0, seg_end=2,
                zs_start=0, zs_end=1, settled=False,
            ),
        ]
        divergences = [
            _make_divergence(
                direction="top", center_idx=1, seg_c_start=0, seg_c_end=0,
            ),
        ]
        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)
        type2 = [b for b in bsps if b.kind == "type2"]
        assert len(type2) == 1
        assert type2[0].confirmed is True   # 78 <= 80，不创新高
        assert type2[0].settled is False    # 走势未完成


# ═══════════════════════════════════════════════════════════
# Type 3 confirmed / settled 语义
# ═══════════════════════════════════════════════════════════


class TestType3ConfirmedSemantic:
    """Type 3 (candidate-fix): confirmed = 回试后延续段存在；settled = Move.settled。

    回试不破（low>ZG / high<ZD）使 BSP 进入列表作为 candidate；回试段之后出现
    break_direction 方向的延续段时才 confirmed（回试结束、走势延续）。
    """

    def _build_type3_scenario(
        self, move_settled: bool, break_direction: str = "up",
        with_move: bool = True, with_continuation: bool = False,
    ) -> tuple[list, list, list]:
        """构造 Type 3 Buy 场景（回试段 low=66 > ZG=60 → 3B 成立）。

        with_continuation=True 时追加回试段后的向上延续段（seg[5]）→ 触发 confirmed。
        """
        segments = [
            _Seg(direction="down", high=60.0, low=50.0, i0=0, i1=5),
            _Seg(direction="up", high=65.0, low=55.0, i0=5, i1=10),
            _Seg(direction="down", high=63.0, low=52.0, i0=10, i1=15),
            _Seg(direction="up", high=75.0, low=62.0, i0=15, i1=20),  # 离开段
            _Seg(direction="down", high=72.0, low=66.0, i0=20, i1=25),  # 回试段
        ]
        if with_continuation:
            segments.append(_Seg(direction="up", high=82.0, low=70.0, i0=25, i1=30))  # 延续段
        zhongshus = [
            _make_zhongshu(
                seg_start=0, seg_end=2, zd=52.0, zg=60.0,
                settled=True, break_direction=break_direction, break_seg=3,
            ),
        ]
        moves = []
        if with_move:
            moves = [
                _make_trend_move(
                    direction="up" if break_direction == "up" else "down",
                    seg_start=0, seg_end=len(segments) - 1, zs_start=0, zs_end=0,
                    settled=move_settled,
                ),
            ]
        return segments, zhongshus, moves

    def test_type3_candidate_when_no_continuation(self):
        """回试不破但无延续段 → candidate（confirmed=False），即使走势未完成。"""
        segments, zhongshus, moves = self._build_type3_scenario(move_settled=False)
        bsps = buysellpoints_from_level(segments, zhongshus, moves, [], 1)
        type3 = [b for b in bsps if b.kind == "type3"]
        assert len(type3) == 1
        assert type3[0].confirmed is False  # 候选：延续段未出现
        assert type3[0].settled is False    # 走势未完成

    def test_type3_confirmed_when_continuation_exists(self):
        """回试后出现向上延续段 → confirmed=True。"""
        segments, zhongshus, moves = self._build_type3_scenario(
            move_settled=False, with_continuation=True,
        )
        bsps = buysellpoints_from_level(segments, zhongshus, moves, [], 1)
        type3 = [b for b in bsps if b.kind == "type3"]
        assert len(type3) == 1
        assert type3[0].confirmed is True

    def test_type3_settled_tracks_move(self):
        """走势完成 → settled=True（独立于 confirmed）。"""
        segments, zhongshus, moves = self._build_type3_scenario(
            move_settled=True, with_continuation=True,
        )
        bsps = buysellpoints_from_level(segments, zhongshus, moves, [], 1)
        type3 = [b for b in bsps if b.kind == "type3"]
        assert len(type3) == 1
        assert type3[0].confirmed is True
        assert type3[0].settled is True

    def test_type3_no_move_candidate_settled_false(self):
        """无 Move 覆盖 + 无延续段 → confirmed=False（候选），settled 降级 False。"""
        segments, zhongshus, moves = self._build_type3_scenario(
            move_settled=False, with_move=False,
        )
        bsps = buysellpoints_from_level(segments, zhongshus, moves, [], 1)
        type3 = [b for b in bsps if b.kind == "type3"]
        assert len(type3) == 1
        assert type3[0].confirmed is False
        assert type3[0].settled is False

    def test_type3_sell_candidate_then_confirmed(self):
        """3S：回抽不破（high<ZD）→ candidate；出现向下延续段 → confirmed。"""
        segments = [
            _Seg(direction="up", high=70.0, low=60.0, i0=0, i1=5),
            _Seg(direction="down", high=65.0, low=55.0, i0=5, i1=10),
            _Seg(direction="up", high=68.0, low=58.0, i0=10, i1=15),
            _Seg(direction="down", high=50.0, low=40.0, i0=15, i1=20),  # 离开段
            _Seg(direction="up", high=53.0, low=45.0, i0=20, i1=25),  # 回抽 high 53 < ZD 55
        ]
        zhongshus = [
            _make_zhongshu(
                seg_start=0, seg_end=2, zd=55.0, zg=65.0,
                settled=True, break_direction="down", break_seg=3,
            ),
        ]
        moves = [
            _make_trend_move(
                direction="down", seg_start=0, seg_end=4,
                zs_start=0, zs_end=0, settled=False,
            ),
        ]
        # 无延续段 → candidate
        bsps = buysellpoints_from_level(segments, zhongshus, moves, [], 1)
        type3 = [b for b in bsps if b.kind == "type3"]
        assert len(type3) == 1
        assert type3[0].confirmed is False
        assert type3[0].settled is False

        # 追加向下延续段（seg[5]）→ confirmed
        segments.append(_Seg(direction="down", high=44.0, low=35.0, i0=25, i1=30))
        bsps2 = buysellpoints_from_level(segments, zhongshus, moves, [], 1)
        type3b = [b for b in bsps2 if b.kind == "type3"]
        assert len(type3b) == 1
        assert type3b[0].confirmed is True


# ═══════════════════════════════════════════════════════════
# Type 1 confirmed / settled 语义
# ═══════════════════════════════════════════════════════════


class TestType1ConfirmedSemantic:
    """Type 1: confirmed = 背驰确认（div 存在即成立）；settled = Move.settled。"""

    def _build_type1_scenario(self, move_settled: bool):
        segments = [
            _Seg(direction="down", high=60.0, low=45.0, i0=0, i1=5),
        ]
        zhongshus = [
            _make_zhongshu(seg_start=0, seg_end=2, zd=50.0, zg=60.0),
            _make_zhongshu(seg_start=2, seg_end=4, zd=48.0, zg=58.0),
        ]
        moves = [
            _make_trend_move(
                direction="down", seg_start=0, seg_end=4,
                zs_start=0, zs_end=1, settled=move_settled,
            ),
        ]
        divergences = [
            _make_divergence(
                direction="bottom", center_idx=1, seg_c_start=0, seg_c_end=0,
            ),
        ]
        return segments, zhongshus, moves, divergences

    def test_type1_confirmed_from_divergence_decoupled(self):
        """背驰存在 → confirmed=True，即使走势未完成（settled=False）。"""
        segments, zhongshus, moves, divergences = self._build_type1_scenario(
            move_settled=False,
        )
        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)
        type1 = [b for b in bsps if b.kind == "type1"]
        assert len(type1) >= 1
        assert type1[0].confirmed is True
        assert type1[0].settled is False

    def test_type1_settled_tracks_move(self):
        """走势完成 → settled=True。"""
        segments, zhongshus, moves, divergences = self._build_type1_scenario(
            move_settled=True,
        )
        bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)
        type1 = [b for b in bsps if b.kind == "type1"]
        assert len(type1) >= 1
        assert type1[0].confirmed is True
        assert type1[0].settled is True
