"""小转大编排集成测试 — 跨级别联动。

I1 completion_check: tests/test_xiaozhuan_da_integration.py

测试小转大在 orchestrator 层的集成：
- 从 RecursiveOrchestratorSnapshot 提取跨级别信号
- 小转大检测与 CostReductionFSM 的联动
- 信号聚合与无效信号过滤

原文依据：
- 第43课：小级别背驰引发大级别转折
- 第53课：小转大时买卖点选择
- 267号§三：小转大处理——旧级别短差停止，新级别上继续
- 338号修正4：强趋势浅回调下短差规则，显式关联小转大

概念溯源: [旧缠论] 第43课 + [267号操作方法论]
"""

from __future__ import annotations

import pytest
from dataclasses import replace

from newchan.a_divergence import Divergence
from newchan.a_move_v1 import Move
from newchan.a_segment_v0 import Segment
from newchan.a_zhongshu_v1 import Zhongshu
from newchan.a_xiaozhuan_da import XiaozhuanDa
from newchan.orchestrator.xiaozhuan_da_orchestrator import (
    CrossLevelSignal,
    extract_cross_level_signals,
    filter_actionable_signals,
    signal_to_fsm_events,
)
from newchan.trading.cost_reduction_fsm import (
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    transition,
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


def _make_uptrend_with_sub_divergence():
    """构造 level-1 上涨趋势 + 次级别背驰的完整数据。

    结构：
      seg0-2  → zhongshu0 [ZD=12,ZG=18]
      seg6-8  → zhongshu1 [ZD=28,ZG=32]
      seg9-10 → C 段（本级别不背驰，次级别有背驰）
    """
    segments = [
        _seg(0, 0, 0, 10, "up", 20, 10),
        _seg(1, 1, 11, 20, "down", 18, 12),
        _seg(2, 2, 21, 30, "up", 19, 13),
        _seg(3, 3, 31, 40, "down", 16, 11),
        _seg(4, 4, 41, 50, "up", 25, 15),
        _seg(5, 5, 51, 60, "down", 22, 20),
        _seg(6, 6, 61, 70, "up", 34, 26),
        _seg(7, 7, 71, 80, "down", 32, 28),
        _seg(8, 8, 81, 90, "up", 33, 29),
        _seg(9, 9, 91, 100, "down", 30, 27),
        _seg(10, 10, 101, 120, "up", 42, 28, confirmed=False),
    ]

    zhongshus = [
        _zs(0, 2, 12, 18, 10, 20, settled=True),
        _zs(6, 8, 28, 32, 26, 34, settled=True),
    ]

    moves = [
        _move("trend", "up", 0, 10, 0, 1, 2, settled=False, high=42, low=10),
    ]

    # 本级别无背驰
    level1_divergences: list[Divergence] = []

    # 次级别在 C 段内有背驰
    sub_divergence = _divergence(
        "trend", "top", 0,
        seg_a_start=4, seg_a_end=5,
        seg_c_start=9, seg_c_end=10,
        center_idx=1, force_a=100.0, force_c=50.0,
    )

    return segments, zhongshus, moves, level1_divergences, [sub_divergence]


# ═══════════════════════════════════════════════════════════════
# 一、跨级别信号提取
# ═══════════════════════════════════════════════════════════════


class TestExtractCrossLevelSignals:
    """测试从走势结构数据提取跨级别小转大信号。"""

    def test_basic_signal_extraction(self):
        """有效的小转大结构 → 产出 CrossLevelSignal。"""
        segs, zss, moves, l1_divs, sub_divs = _make_uptrend_with_sub_divergence()

        signals = extract_cross_level_signals(
            segments=segs,
            zhongshus=zss,
            moves=moves,
            level_divergences=l1_divs,
            sub_divergences=sub_divs,
            level_id=1,
        )

        assert len(signals) == 1
        sig = signals[0]
        assert isinstance(sig, CrossLevelSignal)
        assert sig.xiaozhuan_da.level_id == 1
        assert sig.xiaozhuan_da.side == "sell"
        assert sig.source_level == 1
        assert sig.trigger_level == 0

    def test_no_signal_when_level_has_divergence(self):
        """本级别有背驰 → 不产出小转大信号（走正常背驰路径）。"""
        segs, zss, moves, _, sub_divs = _make_uptrend_with_sub_divergence()

        # 本级别有背驰
        l1_div = _divergence(
            "trend", "top", 1,
            seg_a_start=3, seg_a_end=5,
            seg_c_start=9, seg_c_end=10,
            center_idx=1, force_a=100.0, force_c=50.0,
        )

        signals = extract_cross_level_signals(
            segments=segs,
            zhongshus=zss,
            moves=moves,
            level_divergences=[l1_div],
            sub_divergences=sub_divs,
            level_id=1,
        )

        assert len(signals) == 0

    def test_no_signal_when_no_sub_divergence(self):
        """次级别无背驰 → 不产出信号。"""
        segs, zss, moves, l1_divs, _ = _make_uptrend_with_sub_divergence()

        signals = extract_cross_level_signals(
            segments=segs,
            zhongshus=zss,
            moves=moves,
            level_divergences=l1_divs,
            sub_divergences=[],
            level_id=1,
        )

        assert len(signals) == 0

    def test_empty_input(self):
        """空输入 → 空结果。"""
        signals = extract_cross_level_signals(
            segments=[], zhongshus=[], moves=[],
            level_divergences=[], sub_divergences=[],
            level_id=1,
        )
        assert signals == []

    def test_consolidation_excluded(self):
        """盘整走势不产生小转大信号。"""
        segments = [
            _seg(0, 0, 0, 10, "up", 20, 10),
            _seg(1, 1, 11, 20, "down", 18, 12),
            _seg(2, 2, 21, 30, "up", 19, 13),
        ]
        zhongshus = [_zs(0, 2, 12, 18, 10, 20, settled=True)]
        moves = [_move("consolidation", "up", 0, 2, 0, 0, 1, settled=False)]

        sub_div = _divergence(
            "trend", "top", 0,
            seg_a_start=0, seg_a_end=0,
            seg_c_start=2, seg_c_end=2,
            center_idx=0, force_a=100.0, force_c=50.0,
        )

        signals = extract_cross_level_signals(
            segments=segments, zhongshus=zhongshus, moves=moves,
            level_divergences=[], sub_divergences=[sub_div],
            level_id=1,
        )

        assert len(signals) == 0

    def test_pure_function(self):
        """纯函数性：输入不变。"""
        segs, zss, moves, l1_divs, sub_divs = _make_uptrend_with_sub_divergence()
        segs_before = list(segs)
        zss_before = list(zss)
        moves_before = list(moves)

        extract_cross_level_signals(
            segments=segs, zhongshus=zss, moves=moves,
            level_divergences=l1_divs, sub_divergences=sub_divs,
            level_id=1,
        )

        assert segs == segs_before
        assert zss == zss_before
        assert moves == moves_before


# ═══════════════════════════════════════════════════════════════
# 二、信号过滤（无效信号排除）
# ═══════════════════════════════════════════════════════════════


class TestFilterActionableSignals:
    """测试信号过滤——只保留可操作的小转大信号。"""

    def test_confirmed_sub_divergence_passes(self):
        """次级别背驰已确认 → 信号可操作。"""
        segs, zss, moves, l1_divs, sub_divs = _make_uptrend_with_sub_divergence()

        signals = extract_cross_level_signals(
            segments=segs, zhongshus=zss, moves=moves,
            level_divergences=l1_divs, sub_divergences=sub_divs,
            level_id=1,
        )

        actionable = filter_actionable_signals(signals)
        assert len(actionable) == 1

    def test_unconfirmed_sub_divergence_filtered(self):
        """次级别背驰未确认 → 信号被过滤。"""
        segs, zss, moves, l1_divs, _ = _make_uptrend_with_sub_divergence()

        # 未确认的次级别背驰
        sub_div_unconfirmed = _divergence(
            "trend", "top", 0,
            seg_a_start=4, seg_a_end=5,
            seg_c_start=9, seg_c_end=10,
            center_idx=1, force_a=100.0, force_c=50.0,
            confirmed=False,
        )

        signals = extract_cross_level_signals(
            segments=segs, zhongshus=zss, moves=moves,
            level_divergences=l1_divs, sub_divergences=[sub_div_unconfirmed],
            level_id=1,
        )

        actionable = filter_actionable_signals(signals)
        assert len(actionable) == 0

    def test_empty_signals(self):
        """空信号列表 → 空结果。"""
        assert filter_actionable_signals([]) == []


# ═══════════════════════════════════════════════════════════════
# 三、小转大 → FSM 事件联动
# ═══════════════════════════════════════════════════════════════


class TestSignalToFsmEvents:
    """测试小转大信号转换为 CostReductionFSM 事件。"""

    def test_sell_signal_produces_level_upgrade(self):
        """上涨趋势小转大(sell) → LEVEL_UPGRADE 事件。"""
        segs, zss, moves, l1_divs, sub_divs = _make_uptrend_with_sub_divergence()

        signals = extract_cross_level_signals(
            segments=segs, zhongshus=zss, moves=moves,
            level_divergences=l1_divs, sub_divergences=sub_divs,
            level_id=1,
        )
        assert len(signals) == 1

        fsm_events = signal_to_fsm_events(signals[0], current_price=42.0)

        assert len(fsm_events) == 1
        ev = fsm_events[0]
        assert ev.event_type == FsmEventType.LEVEL_UPGRADE
        assert ev.price == 42.0

    def test_buy_signal_produces_level_upgrade(self):
        """下跌趋势小转大(buy) → LEVEL_UPGRADE 事件。"""
        # 构造下跌趋势
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
            _zs(0, 2, 12, 18, 8, 20, settled=True),
            _zs(6, 8, 2, 4, 0.5, 6, settled=True),
        ]
        moves = [
            _move("trend", "down", 0, 10, 0, 1, 2, settled=False, high=20, low=-2),
        ]
        sub_div = _divergence(
            "trend", "bottom", 0,
            seg_a_start=4, seg_a_end=5,
            seg_c_start=9, seg_c_end=10,
            center_idx=1, force_a=100.0, force_c=50.0,
        )

        signals = extract_cross_level_signals(
            segments=segments, zhongshus=zhongshus, moves=moves,
            level_divergences=[], sub_divergences=[sub_div],
            level_id=1,
        )
        assert len(signals) == 1

        fsm_events = signal_to_fsm_events(signals[0], current_price=1.5)
        assert len(fsm_events) == 1
        assert fsm_events[0].event_type == FsmEventType.LEVEL_UPGRADE
        assert fsm_events[0].price == 1.5


# ═══════════════════════════════════════════════════════════════
# 四、FSM 级别升级完整流程
# ═══════════════════════════════════════════════════════════════


class TestFsmLevelUpgradeIntegration:
    """测试 CostReductionFSM 在收到 LEVEL_UPGRADE 后的完整行为。"""

    def test_level_upgrade_from_cost_reducing(self):
        """COST_REDUCING 状态收到 LEVEL_UPGRADE → 赋格分裂。"""
        fsm = CostReductionFSM.create(own_capital=100000.0, margin_amount=100000.0)

        # 建仓
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.BUY_POINT_CONFIRMED,
            price=10.0, level="L1",
        ))
        assert fsm.state == CostState.POSITION_OPEN

        # 开始降成本
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_SELL_POINT,
            price=11.0, level="sub-L1",
        ))
        assert fsm.state == CostState.COST_REDUCING

        # 小转大 → LEVEL_UPGRADE
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.LEVEL_UPGRADE,
            price=12.0, level="L2",
        ))

        # 验证赋格分裂
        assert fsm.state == CostState.COST_REDUCING
        assert fsm.entry_level == "L2"
        assert fsm.active_short_diff is None  # 旧短差停止
        assert len(fsm.fugue_voices) == 2
        assert fsm.fugue_voices[0].is_profit_floor  # 旧声部：利润底仓
        assert not fsm.fugue_voices[1].is_profit_floor  # 新声部：新循环

    def test_signal_to_fsm_full_chain(self):
        """完整链路：走势数据 → 信号提取 → 过滤 → FSM 事件 → 状态转移。"""
        segs, zss, moves, l1_divs, sub_divs = _make_uptrend_with_sub_divergence()

        # 1. 提取信号
        signals = extract_cross_level_signals(
            segments=segs, zhongshus=zss, moves=moves,
            level_divergences=l1_divs, sub_divergences=sub_divs,
            level_id=1,
        )
        assert len(signals) == 1

        # 2. 过滤
        actionable = filter_actionable_signals(signals)
        assert len(actionable) == 1

        # 3. 转为 FSM 事件
        fsm_events = signal_to_fsm_events(actionable[0], current_price=42.0)
        assert len(fsm_events) == 1

        # 4. 驱动 FSM
        fsm = CostReductionFSM.create(own_capital=100000.0, margin_amount=100000.0)
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.BUY_POINT_CONFIRMED,
            price=10.0, level="L1",
        ))
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_SELL_POINT,
            price=11.0, level="sub-L1",
        ))

        # 应用小转大事件
        fsm = transition(fsm, fsm_events[0])

        assert fsm.entry_level == "L2"
        assert len(fsm.fugue_voices) == 2


# ═══════════════════════════════════════════════════════════════
# 五、强趋势浅回调信号（338号修正4）
# ═══════════════════════════════════════════════════════════════


class TestShallowPullbackPrecursor:
    """338号修正4：强趋势浅回调 → 小转大前兆。

    "次级别卖出后，若次级别买点价格 >= 前次卖出价格，不执行加回。"
    这是结构层面的前兆信号检测，不是操作指令。
    """

    def test_shallow_pullback_detected(self):
        """次级别买回价格 >= 卖出价格 → 标记为浅回调前兆。"""
        from newchan.orchestrator.xiaozhuan_da_orchestrator import (
            detect_shallow_pullback_precursor,
        )

        # 卖出 @11.0，买回 @11.5（高于卖出价）
        is_precursor = detect_shallow_pullback_precursor(
            sell_price=11.0, buy_price=11.5,
        )
        assert is_precursor is True

    def test_normal_pullback_not_precursor(self):
        """次级别买回价格 < 卖出价格 → 正常短差，非前兆。"""
        from newchan.orchestrator.xiaozhuan_da_orchestrator import (
            detect_shallow_pullback_precursor,
        )

        is_precursor = detect_shallow_pullback_precursor(
            sell_price=11.0, buy_price=10.5,
        )
        assert is_precursor is False

    def test_equal_price_is_precursor(self):
        """买回价格 == 卖出价格 → 也是浅回调（338号：>=）。"""
        from newchan.orchestrator.xiaozhuan_da_orchestrator import (
            detect_shallow_pullback_precursor,
        )

        is_precursor = detect_shallow_pullback_precursor(
            sell_price=11.0, buy_price=11.0,
        )
        assert is_precursor is True


# ═══════════════════════════════════════════════════════════════
# 六、CrossLevelSignal 数据完整性
# ═══════════════════════════════════════════════════════════════


class TestCrossLevelSignalIntegrity:
    """CrossLevelSignal 结构字段的完整性和不可变性。"""

    def test_signal_is_frozen(self):
        """CrossLevelSignal 应该是不可变的。"""
        segs, zss, moves, l1_divs, sub_divs = _make_uptrend_with_sub_divergence()

        signals = extract_cross_level_signals(
            segments=segs, zhongshus=zss, moves=moves,
            level_divergences=l1_divs, sub_divergences=sub_divs,
            level_id=1,
        )
        assert len(signals) == 1

        sig = signals[0]
        with pytest.raises(AttributeError):
            sig.source_level = 2  # type: ignore[misc]

    def test_signal_contains_xiaozhuan_da(self):
        """CrossLevelSignal 内嵌完整的 XiaozhuanDa。"""
        segs, zss, moves, l1_divs, sub_divs = _make_uptrend_with_sub_divergence()

        signals = extract_cross_level_signals(
            segments=segs, zhongshus=zss, moves=moves,
            level_divergences=l1_divs, sub_divergences=sub_divs,
            level_id=1,
        )

        sig = signals[0]
        xzd = sig.xiaozhuan_da
        assert isinstance(xzd, XiaozhuanDa)
        assert xzd.c_seg_start == 9
        assert xzd.c_seg_end == 10
        assert xzd.sub_divergence.confirmed is True

    def test_signal_level_relationship(self):
        """source_level 和 trigger_level 的关系：trigger = source - 1。"""
        segs, zss, moves, l1_divs, sub_divs = _make_uptrend_with_sub_divergence()

        signals = extract_cross_level_signals(
            segments=segs, zhongshus=zss, moves=moves,
            level_divergences=l1_divs, sub_divergences=sub_divs,
            level_id=1,
        )

        sig = signals[0]
        assert sig.trigger_level == sig.source_level - 1
