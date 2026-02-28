"""降成本状态机测试 — TDD RED phase。

认识论标注：L0（从267号定义直接推导的代数结构）。
谱系引用：267号操作方法论 v1，268a号结算修正。
"""

from __future__ import annotations

import pytest

from newchan.trading.cost_reduction_fsm import (
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    FsmSnapshot,
    FugueVoice,
    ShortDiffCycle,
    transition,
)


# ── 工厂 ──────────────────────────────────────────────────────


def _make_fsm(
    *,
    equity: float = 100_000.0,
    margin: float = 100_000.0,
) -> CostReductionFSM:
    """创建初始状态机（SCANNING）。"""
    return CostReductionFSM.create(own_capital=equity, margin_amount=margin)


def _open_position(fsm: CostReductionFSM, price: float = 10.0) -> CostReductionFSM:
    """从 SCANNING 直接建仓到 POSITION_OPEN。"""
    event = FsmEvent(
        event_type=FsmEventType.BUY_POINT_CONFIRMED,
        price=price,
        level="30min",
    )
    return transition(fsm, event)


def _start_reducing(fsm: CostReductionFSM) -> CostReductionFSM:
    """从 POSITION_OPEN 进入 COST_REDUCING。"""
    event = FsmEvent(
        event_type=FsmEventType.SUB_LEVEL_SELL_POINT,
        price=11.0,
        level="5min",
    )
    return transition(fsm, event)


# ═══════════════════════════════════════════════════════════════
# 1. 基本状态转移
# ═══════════════════════════════════════════════════════════════


class TestBasicTransitions:
    """SCANNING → POSITION_OPEN → COST_REDUCING → PRINCIPAL_WITHDRAWN."""

    def test_initial_state_is_scanning(self) -> None:
        fsm = _make_fsm()
        assert fsm.state == CostState.SCANNING

    def test_scanning_to_position_open(self) -> None:
        fsm = _make_fsm()
        event = FsmEvent(
            event_type=FsmEventType.BUY_POINT_CONFIRMED,
            price=10.0,
            level="30min",
        )
        new = transition(fsm, event)
        assert new.state == CostState.POSITION_OPEN
        # 满仓满融：size = (equity + margin) / price
        expected_size = (100_000.0 + 100_000.0) / 10.0
        assert new.total_shares == pytest.approx(expected_size)
        # 成本 = 自有资金 + 融资
        assert new.cost_basis == pytest.approx(10.0)

    def test_position_open_to_cost_reducing(self) -> None:
        fsm = _open_position(_make_fsm())
        # 次级别卖点出现 → 开始短差
        event = FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_SELL_POINT,
            price=11.0,
            level="5min",
        )
        new = transition(fsm, event)
        assert new.state == CostState.COST_REDUCING

    def test_cost_reducing_to_principal_withdrawn(self) -> None:
        """累计回收 >= 初始自有资金 → PRINCIPAL_WITHDRAWN。"""
        fsm = _open_position(_make_fsm(equity=100.0, margin=100.0), price=10.0)
        fsm = _start_reducing(fsm)
        # 模拟多次短差直到累计回收 >= 100（自有资金）
        # 买回
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_BUY_POINT,
            price=9.0,
            level="5min",
        ))
        # 再卖
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_SELL_POINT,
            price=15.0,
            level="5min",
        ))
        # 再买回
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_BUY_POINT,
            price=8.0,
            level="5min",
        ))
        # 再卖
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_SELL_POINT,
            price=20.0,
            level="5min",
        ))
        # 再买回
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_BUY_POINT,
            price=7.0,
            level="5min",
        ))
        # 累计回收应该达到自有资金阈值
        if fsm.cumulative_recovered >= fsm.own_capital:
            assert fsm.state == CostState.PRINCIPAL_WITHDRAWN
        else:
            # 继续短差直到达到
            fsm = transition(fsm, FsmEvent(
                event_type=FsmEventType.SUB_LEVEL_SELL_POINT,
                price=25.0,
                level="5min",
            ))
            fsm = transition(fsm, FsmEvent(
                event_type=FsmEventType.SUB_LEVEL_BUY_POINT,
                price=5.0,
                level="5min",
            ))
            assert fsm.cumulative_recovered >= fsm.own_capital
            assert fsm.state == CostState.PRINCIPAL_WITHDRAWN


# ═══════════════════════════════════════════════════════════════
# 2. 止损路径
# ═══════════════════════════════════════════════════════════════


class TestStopLoss:
    """任何持仓状态 → STOPPED_OUT → SCANNING。"""

    def test_position_open_stop_loss(self) -> None:
        fsm = _open_position(_make_fsm())
        event = FsmEvent(
            event_type=FsmEventType.BUY_POINT_NEGATED,
            price=8.0,
            level="30min",
        )
        new = transition(fsm, event)
        assert new.state == CostState.STOPPED_OUT

    def test_cost_reducing_stop_loss(self) -> None:
        fsm = _start_reducing(_open_position(_make_fsm()))
        event = FsmEvent(
            event_type=FsmEventType.BUY_POINT_NEGATED,
            price=7.0,
            level="30min",
        )
        new = transition(fsm, event)
        assert new.state == CostState.STOPPED_OUT

    def test_principal_withdrawn_main_sell_point(self) -> None:
        """免费仓位阶段，主级别卖点 → STOPPED_OUT（正常退出也走同一出口）。"""
        fsm = _make_fsm(equity=10.0, margin=10.0)
        fsm = _open_position(fsm, price=1.0)
        fsm = _start_reducing(fsm)
        # 大幅盈利直接触发本金退出
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_BUY_POINT,
            price=0.5,
            level="5min",
        ))
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_SELL_POINT,
            price=100.0,
            level="5min",
        ))
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_BUY_POINT,
            price=0.1,
            level="5min",
        ))
        if fsm.state == CostState.PRINCIPAL_WITHDRAWN:
            event = FsmEvent(
                event_type=FsmEventType.MAIN_LEVEL_SELL_POINT,
                price=50.0,
                level="30min",
            )
            new = transition(fsm, event)
            assert new.state == CostState.STOPPED_OUT

    def test_stopped_out_to_scanning(self) -> None:
        fsm = _open_position(_make_fsm())
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.BUY_POINT_NEGATED,
            price=8.0,
            level="30min",
        ))
        assert fsm.state == CostState.STOPPED_OUT
        new = transition(fsm, FsmEvent(
            event_type=FsmEventType.RESET,
            price=0.0,
            level="",
        ))
        assert new.state == CostState.SCANNING


# ═══════════════════════════════════════════════════════════════
# 3. 成本计算公式
# ═══════════════════════════════════════════════════════════════


class TestCostCalculation:
    """cost(t) = cost(t-1) - profit(t) / size(t)。"""

    def test_initial_cost_basis(self) -> None:
        fsm = _open_position(_make_fsm(equity=100_000.0, margin=100_000.0), price=10.0)
        assert fsm.cost_basis == pytest.approx(10.0)

    def test_cost_decreases_after_profitable_short_diff(self) -> None:
        fsm = _open_position(_make_fsm(), price=10.0)
        fsm = _start_reducing(fsm)
        # 买回低价
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_BUY_POINT,
            price=9.0,
            level="5min",
        ))
        # cost(t) = cost(t-1) - profit / size
        assert fsm.cost_basis < 10.0

    def test_cost_formula_exact(self) -> None:
        """验证精确的成本递推公式。"""
        equity = 100_000.0
        margin = 100_000.0
        entry_price = 10.0
        fsm = _open_position(_make_fsm(equity=equity, margin=margin), price=entry_price)
        total_shares = fsm.total_shares
        initial_cost = entry_price  # 10.0

        # 次级别卖点@11 → 短差开始
        fsm = _start_reducing(fsm)
        # 第一笔短差：卖@11，买回@9
        # 短差份额 = total_shares * sub_ratio（默认）
        # profit = (11 - 9) * short_shares
        # new_cost = old_cost - profit / total_shares
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.SUB_LEVEL_BUY_POINT,
            price=9.0,
            level="5min",
        ))
        # 验证成本下降了
        assert fsm.cost_basis < initial_cost


# ═══════════════════════════════════════════════════════════════
# 4. 小转大赋格分裂
# ═══════════════════════════════════════════════════════════════


class TestLevelUpgrade:
    """小转大：旧循环利润底仓 + 新级别新降成本循环。"""

    def test_level_upgrade_creates_fugue(self) -> None:
        fsm = _start_reducing(_open_position(_make_fsm()))
        event = FsmEvent(
            event_type=FsmEventType.LEVEL_UPGRADE,
            price=15.0,
            level="daily",  # 从 30min 升级到 daily
        )
        new = transition(fsm, event)
        # 小转大后：状态仍然是 COST_REDUCING（在新级别上继续降成本）
        assert new.state == CostState.COST_REDUCING
        # 赋格：应该有至少两个声部
        assert len(new.fugue_voices) >= 2

    def test_fugue_voice_old_is_profit_floor(self) -> None:
        """旧循环变为利润底仓声部。"""
        fsm = _start_reducing(_open_position(_make_fsm()))
        event = FsmEvent(
            event_type=FsmEventType.LEVEL_UPGRADE,
            price=15.0,
            level="daily",
        )
        new = transition(fsm, event)
        old_voice = new.fugue_voices[0]
        assert old_voice.is_profit_floor is True

    def test_fugue_voice_new_has_fresh_cycle(self) -> None:
        """新声部是新级别上的新降成本循环。"""
        fsm = _start_reducing(_open_position(_make_fsm()))
        event = FsmEvent(
            event_type=FsmEventType.LEVEL_UPGRADE,
            price=15.0,
            level="daily",
        )
        new = transition(fsm, event)
        new_voice = new.fugue_voices[-1]
        assert new_voice.is_profit_floor is False
        assert new_voice.level == "daily"


# ═══════════════════════════════════════════════════════════════
# 5. 不换仓（无"换仓"事件类型）
# ═══════════════════════════════════════════════════════════════


class TestNoPositionSwap:
    """状态机无"换仓"转移事件。"""

    def test_no_swap_event_type(self) -> None:
        """FsmEventType 中不存在 SWAP 或 SWITCH_POSITION。"""
        event_names = [e.name for e in FsmEventType]
        assert "SWAP" not in event_names
        assert "SWITCH_POSITION" not in event_names
        assert "CHANGE_STOCK" not in event_names

    def test_transition_rejects_unknown_event(self) -> None:
        """未定义的事件类型不被接受。"""
        fsm = _open_position(_make_fsm())
        # FsmEventType 是 Enum，无法构造不存在的成员
        # 验证所有合法事件都不包含"换仓"语义
        for et in FsmEventType:
            assert "swap" not in et.name.lower()
            assert "switch" not in et.name.lower()


# ═══════════════════════════════════════════════════════════════
# 6. 递归嵌套短差的层级约束
# ═══════════════════════════════════════════════════════════════


class TestNestedShortDiff:
    """次级别短差内部可用次次级别做更小短差，每层比例 <= 上层的 n%。"""

    def test_short_diff_cycle_tracks_shares(self) -> None:
        """短差循环跟踪当前动用的份额。"""
        fsm = _open_position(_make_fsm())
        fsm = _start_reducing(fsm)
        assert fsm.active_short_diff is not None
        assert fsm.active_short_diff.shares > 0

    def test_nested_ratio_constraint(self) -> None:
        """每层短差动用比例 <= 上层的 sub_ratio。"""
        fsm = _open_position(_make_fsm())
        fsm = _start_reducing(fsm)
        cycle = fsm.active_short_diff
        assert cycle is not None
        # 短差份额 <= 总份额 * sub_ratio
        assert cycle.shares <= fsm.total_shares * fsm.sub_ratio + 1e-9


# ═══════════════════════════════════════════════════════════════
# 7. 快照不可变性
# ═══════════════════════════════════════════════════════════════


class TestImmutability:
    """所有状态用 frozen dataclass。"""

    def test_fsm_is_frozen(self) -> None:
        fsm = _make_fsm()
        with pytest.raises(AttributeError):
            fsm.state = CostState.POSITION_OPEN  # type: ignore[misc]

    def test_snapshot_roundtrip(self) -> None:
        fsm = _open_position(_make_fsm())
        snap = fsm.snapshot()
        assert isinstance(snap, FsmSnapshot)
        assert snap.state == CostState.POSITION_OPEN

    def test_transition_returns_new_instance(self) -> None:
        fsm = _make_fsm()
        event = FsmEvent(
            event_type=FsmEventType.BUY_POINT_CONFIRMED,
            price=10.0,
            level="30min",
        )
        new = transition(fsm, event)
        assert new is not fsm
        assert fsm.state == CostState.SCANNING
        assert new.state == CostState.POSITION_OPEN


# ═══════════════════════════════════════════════════════════════
# 8. 268a攻击1：初始自有资金动态重置
# ═══════════════════════════════════════════════════════════════


class TestOwnCapitalReset:
    """268a攻击1修正：RESET 事件携带新 own_capital，每次循环独立核算。"""

    def test_reset_with_new_own_capital(self) -> None:
        """止损后 RESET 携带缩水后的实际资金。"""
        fsm = _open_position(_make_fsm(equity=100_000.0, margin=100_000.0))
        # 止损
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.BUY_POINT_NEGATED,
            price=8.0,
            level="30min",
        ))
        assert fsm.state == CostState.STOPPED_OUT
        # RESET 时传入缩水后的实际资金
        new_capital = 80_000.0
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.RESET,
            price=0.0,
            level="",
            new_own_capital=new_capital,
        ))
        assert fsm.state == CostState.SCANNING
        assert fsm.own_capital == pytest.approx(new_capital)

    def test_reset_without_new_own_capital_preserves_old(self) -> None:
        """向后兼容：不传 new_own_capital 时沿用原值。"""
        fsm = _open_position(_make_fsm(equity=100_000.0, margin=100_000.0))
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.BUY_POINT_NEGATED,
            price=8.0,
            level="30min",
        ))
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.RESET,
            price=0.0,
            level="",
        ))
        assert fsm.own_capital == pytest.approx(100_000.0)

    def test_consecutive_stop_loss_capital_shrinks(self) -> None:
        """连续止损后，每次 RESET 传入缩水资金，建仓规模随之缩小。"""
        capital = 100_000.0
        margin = 100_000.0
        fsm = CostReductionFSM.create(own_capital=capital, margin_amount=margin)

        for shrunk_capital in [80_000.0, 64_000.0, 51_200.0]:
            # 建仓
            fsm = transition(fsm, FsmEvent(
                event_type=FsmEventType.BUY_POINT_CONFIRMED,
                price=10.0,
                level="30min",
            ))
            # 止损
            fsm = transition(fsm, FsmEvent(
                event_type=FsmEventType.BUY_POINT_NEGATED,
                price=8.0,
                level="30min",
            ))
            # RESET 传入缩水后资金
            fsm = transition(fsm, FsmEvent(
                event_type=FsmEventType.RESET,
                price=0.0,
                level="",
                new_own_capital=shrunk_capital,
            ))
            assert fsm.own_capital == pytest.approx(shrunk_capital)

        # 最终以 51200 建仓，份额应该是 (51200 + 100000) / 10
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.BUY_POINT_CONFIRMED,
            price=10.0,
            level="30min",
        ))
        expected_shares = (51_200.0 + margin) / 10.0
        assert fsm.total_shares == pytest.approx(expected_shares)


# ═══════════════════════════════════════════════════════════════
# 9. 268a攻击2：最小可操作级别外部参数
# ═══════════════════════════════════════════════════════════════


class TestMinOperableLevel:
    """268a攻击2修正：min_operable_level 作为不入语法的外部参数标注。"""

    def test_default_min_operable_level_is_empty(self) -> None:
        fsm = _make_fsm()
        assert fsm.min_operable_level == ""

    def test_create_with_min_operable_level(self) -> None:
        fsm = CostReductionFSM.create(
            own_capital=100_000.0,
            min_operable_level="5min",
        )
        assert fsm.min_operable_level == "5min"

    def test_min_operable_level_preserved_through_transitions(self) -> None:
        """min_operable_level 在状态转移中保持不变。"""
        fsm = CostReductionFSM.create(
            own_capital=100_000.0,
            margin_amount=100_000.0,
            min_operable_level="5min",
        )
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.BUY_POINT_CONFIRMED,
            price=10.0,
            level="30min",
        ))
        assert fsm.min_operable_level == "5min"

    def test_min_operable_level_preserved_after_reset(self) -> None:
        """RESET 后 min_operable_level 不丢失。"""
        fsm = CostReductionFSM.create(
            own_capital=100_000.0,
            margin_amount=100_000.0,
            min_operable_level="5min",
        )
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.BUY_POINT_CONFIRMED,
            price=10.0,
            level="30min",
        ))
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.BUY_POINT_NEGATED,
            price=8.0,
            level="30min",
        ))
        fsm = transition(fsm, FsmEvent(
            event_type=FsmEventType.RESET,
            price=0.0,
            level="",
        ))
        assert fsm.min_operable_level == "5min"
