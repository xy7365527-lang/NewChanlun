"""G1 仓位管理模块测试。

覆盖：
- 三阶段状态机（IDLE → COST_POSITIVE → COST_ZERO → FREE_POSITION → IDLE）
- 退出条件（买入程序否定）
- 短差程序（减仓/回补）
- 338号修正4（强趋势浅回调不加回）
- 短差摩擦成本截止
- 批量处理
"""

from __future__ import annotations

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

from scripts.position_manager import (
    Action,
    Phase,
    Position,
    PositionConfig,
    ShortDiffRecord,
    Signal,
    check_exit_condition,
    process_signal,
    run_signals,
    short_diff_feasible,
)


# ── 配置 ──

DEFAULT_CONFIG = PositionConfig(
    own_capital=100000.0,
    leverage_ratio=2.0,
    short_diff_ratio=0.3,
)


def _buy_signal(
    kind: str = "type1",
    level: int = 0,
    price: float = 10.0,
    confirmed: bool = True,
    invalidated: bool = False,
    ts: float = 0.0,
) -> Signal:
    return Signal(
        kind=kind, side="buy", level=level, price=price,
        timestamp=ts, confirmed=confirmed, invalidated=invalidated,
    )


def _sell_signal(
    kind: str = "type1",
    level: int = 0,
    price: float = 10.0,
    confirmed: bool = True,
    invalidated: bool = False,
    ts: float = 0.0,
) -> Signal:
    return Signal(
        kind=kind, side="sell", level=level, price=price,
        timestamp=ts, confirmed=confirmed, invalidated=invalidated,
    )


# ── 阶段1：建仓 ──


class TestEntry:
    """建仓测试。"""

    def test_idle_buy_confirmed_enters(self):
        """操作级别确认买点 → 建仓。"""
        pos = Position()
        sig = _buy_signal(price=10.0)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert new_pos.phase == Phase.COST_POSITIVE
        assert new_pos.entry_kind == "type1"
        assert new_pos.entry_price == 10.0
        assert new_pos.cost == 10.0
        assert new_pos.shares == 20000.0  # 200000 / 10
        assert new_pos.own_capital == 100000.0
        assert action.action_type == "enter"
        assert action.shares == 20000.0

    def test_idle_sell_no_action(self):
        """IDLE + 卖出信号 → 无操作。"""
        pos = Position()
        sig = _sell_signal(price=10.0)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert new_pos.phase == Phase.IDLE
        assert action.action_type == "hold"

    def test_idle_unconfirmed_no_action(self):
        """IDLE + 未确认买点 → 无操作。"""
        pos = Position()
        sig = _buy_signal(confirmed=False)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert new_pos.phase == Phase.IDLE
        assert action.action_type == "hold"

    def test_idle_sublevel_buy_no_action(self):
        """IDLE + 次级别买点 → 无操作（只在操作级别建仓）。"""
        pos = Position()
        sig = _buy_signal(level=1)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert new_pos.phase == Phase.IDLE
        assert action.action_type == "hold"

    def test_entry_type2_buy(self):
        """type2 买点建仓。"""
        pos = Position()
        sig = _buy_signal(kind="type2", price=15.0)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert new_pos.entry_kind == "type2"
        assert action.action_type == "enter"

    def test_entry_type3_buy(self):
        """type3 买点建仓。"""
        pos = Position()
        sig = _buy_signal(kind="type3", price=20.0)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert new_pos.entry_kind == "type3"
        assert action.action_type == "enter"


# ── 退出条件 ──


class TestExitCondition:
    """退出条件测试——买入程序否定。"""

    def test_invalidated_exits(self):
        """操作级别信号 invalidated → 结构退出。"""
        pos = Position(
            phase=Phase.COST_POSITIVE,
            entry_kind="type1",
            entry_price=10.0,
            shares=20000.0,
            own_capital=100000.0,
            cost=10.0,
        )
        sig = _sell_signal(price=9.0, invalidated=True)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert new_pos.phase == Phase.IDLE
        assert action.action_type == "exit_structure"
        assert action.shares == 20000.0

    def test_non_invalidated_holds(self):
        """操作级别信号非 invalidated → 不退出。"""
        pos = Position(
            phase=Phase.COST_POSITIVE,
            entry_kind="type1",
            entry_price=10.0,
            shares=20000.0,
            own_capital=100000.0,
            cost=10.0,
        )
        sig = _sell_signal(price=9.0, invalidated=False)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert new_pos.phase == Phase.COST_POSITIVE
        assert action.action_type == "hold"

    def test_sublevel_invalidated_no_exit(self):
        """次级别 invalidated → 不触发退出（退出只看操作级别）。"""
        pos = Position(
            phase=Phase.COST_POSITIVE,
            entry_kind="type1",
            entry_price=10.0,
            shares=20000.0,
            own_capital=100000.0,
            cost=10.0,
        )
        sig = _sell_signal(level=1, price=9.0, invalidated=True)

        assert not check_exit_condition(pos, sig)

    def test_exit_from_free_position(self):
        """FREE_POSITION + invalidated → 也退出。"""
        pos = Position(
            phase=Phase.FREE_POSITION,
            entry_kind="type1",
            entry_price=10.0,
            shares=10000.0,
            cost=0.0,
        )
        sig = _sell_signal(price=8.0, invalidated=True)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert new_pos.phase == Phase.IDLE
        assert action.action_type == "exit_structure"


# ── 短差程序 ──


class TestShortDiff:
    """短差程序测试。"""

    def test_sublevel_sell_reduces(self):
        """次级别卖点 → 短差减仓。"""
        pos = Position(
            phase=Phase.COST_POSITIVE,
            entry_kind="type1",
            entry_price=10.0,
            shares=20000.0,
            own_capital=100000.0,
            cost=10.0,
        )
        sig = _sell_signal(level=1, price=11.0)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert action.action_type == "short_sell"
        assert action.shares == 6000.0  # 20000 * 0.3
        assert new_pos.pending_sell_price == 11.0

    def test_sublevel_buy_recovers(self):
        """次级别买点（价格低于卖出价）→ 短差回补。"""
        pos = Position(
            phase=Phase.COST_POSITIVE,
            entry_kind="type1",
            entry_price=10.0,
            shares=20000.0,
            own_capital=100000.0,
            cost=10.0,
            pending_sell_price=11.0,
            pending_sell_level=1,
        )
        sig = _buy_signal(level=1, price=10.2)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert action.action_type == "short_buy"
        expected_profit = (11.0 - 10.2) * 6000.0  # 4800
        assert new_pos.accumulated_recovery == expected_profit
        assert new_pos.pending_sell_price is None
        assert len(new_pos.short_diffs) == 1

    def test_no_double_sell(self):
        """已有未回补的短差 → 不再减仓。"""
        pos = Position(
            phase=Phase.COST_POSITIVE,
            entry_kind="type1",
            entry_price=10.0,
            shares=20000.0,
            own_capital=100000.0,
            cost=10.0,
            pending_sell_price=11.0,
        )
        sig = _sell_signal(level=1, price=12.0)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert action.action_type == "hold"

    def test_buyback_no_pending(self):
        """无待回补短差时买点 → 无操作。"""
        pos = Position(
            phase=Phase.COST_POSITIVE,
            entry_kind="type1",
            entry_price=10.0,
            shares=20000.0,
            own_capital=100000.0,
            cost=10.0,
        )
        sig = _buy_signal(level=1, price=9.0)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert action.action_type == "hold"


# ── 338号修正4：强趋势浅回调不加回 ──


class TestSkipBuyback:
    """338号修正4测试。"""

    def test_skip_when_price_higher(self):
        """买点价格 >= 卖出价格 → skip_buyback。"""
        pos = Position(
            phase=Phase.COST_POSITIVE,
            entry_kind="type1",
            entry_price=10.0,
            shares=20000.0,
            own_capital=100000.0,
            cost=10.0,
            pending_sell_price=11.0,
            pending_sell_level=1,
        )
        sig = _buy_signal(level=1, price=11.5)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert action.action_type == "skip_buyback"
        assert new_pos.pending_sell_price is None

    def test_skip_when_price_equal(self):
        """买点价格 == 卖出价格 → skip_buyback。"""
        pos = Position(
            phase=Phase.COST_POSITIVE,
            entry_kind="type1",
            entry_price=10.0,
            shares=20000.0,
            own_capital=100000.0,
            cost=10.0,
            pending_sell_price=11.0,
            pending_sell_level=1,
        )
        sig = _buy_signal(level=1, price=11.0)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert action.action_type == "skip_buyback"


# ── 阶段推进 ──


class TestPhaseTransition:
    """阶段推进测试。"""

    def test_cost_positive_to_cost_zero(self):
        """累计回收 >= 自有资金 → COST_ZERO。"""
        pos = Position(
            phase=Phase.COST_POSITIVE,
            entry_kind="type1",
            entry_price=10.0,
            shares=20000.0,
            own_capital=100000.0,
            cost=8.0,
            accumulated_recovery=100000.0,
        )
        sig = _sell_signal(level=1, price=12.0)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert new_pos.phase == Phase.COST_ZERO
        assert action.action_type == "hold"
        assert "成本归零" in action.reason

    def test_cost_zero_to_free_position(self):
        """COST_ZERO + 操作级别卖点 → 退出本金。"""
        pos = Position(
            phase=Phase.COST_ZERO,
            entry_kind="type1",
            entry_price=10.0,
            shares=20000.0,
            own_capital=100000.0,
            cost=5.0,
            accumulated_recovery=100000.0,
        )
        sig = _sell_signal(level=0, price=20.0)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert new_pos.phase == Phase.FREE_POSITION
        assert new_pos.shares == 10000.0  # 半仓
        assert new_pos.cost == 0.0
        assert action.action_type == "extract_capital"
        assert action.shares == 10000.0

    def test_free_position_final_exit(self):
        """FREE_POSITION + 主级别卖点 → 清仓。"""
        pos = Position(
            phase=Phase.FREE_POSITION,
            entry_kind="type1",
            entry_price=10.0,
            shares=10000.0,
            cost=0.0,
        )
        sig = _sell_signal(level=0, price=25.0)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert new_pos.phase == Phase.IDLE
        assert action.action_type == "final_exit"
        assert action.shares == 10000.0


# ── 免费仓位短差 ──


class TestFreePositionShortDiff:
    """阶段三短差测试。"""

    def test_free_position_short_sell(self):
        """FREE_POSITION + 次级别卖点 → 短差。"""
        pos = Position(
            phase=Phase.FREE_POSITION,
            entry_kind="type1",
            entry_price=10.0,
            shares=10000.0,
            cost=0.0,
        )
        sig = _sell_signal(level=1, price=25.0)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert action.action_type == "short_sell"
        assert new_pos.pending_sell_price == 25.0

    def test_free_position_short_buy_increases_shares(self):
        """FREE_POSITION 短差回补 → 仓位增加（第31课阶段三）。"""
        pos = Position(
            phase=Phase.FREE_POSITION,
            entry_kind="type1",
            entry_price=10.0,
            shares=10000.0,
            cost=0.0,
            pending_sell_price=25.0,
            pending_sell_level=1,
        )
        # 买点价格20 < 卖出价25 → 正常回补
        sig = _buy_signal(level=1, price=20.0)

        new_pos, action = process_signal(pos, sig, DEFAULT_CONFIG)

        assert action.action_type == "short_buy"
        # 卖出3000份@25=75000, 买入75000/20=3750份
        # 新总份额 = 10000 - 3000 + 3750 = 10750
        assert new_pos.shares > pos.shares  # 仓位增加


# ── 摩擦成本检查 ──


class TestFrictionCost:
    """摩擦成本截止条件测试。"""

    def test_feasible_spread(self):
        """价差足够 → 可行。"""
        assert short_diff_feasible(DEFAULT_CONFIG, 1, 0.01)

    def test_infeasible_spread(self):
        """价差不足 → 不可行。"""
        assert not short_diff_feasible(DEFAULT_CONFIG, 1, 0.003)

    def test_infeasible_level(self):
        """级别超过最小可操作级别 → 不可行。"""
        assert not short_diff_feasible(DEFAULT_CONFIG, 3, 0.05)


# ── 批量处理 ──


class TestBatchProcessing:
    """批量处理测试。"""

    def test_run_signals_sequence(self):
        """完整信号序列处理。"""
        signals = [
            _buy_signal(price=10.0, ts=1000.0),
            _sell_signal(level=1, price=11.0, ts=2000.0),
            _buy_signal(level=1, price=10.2, ts=3000.0),
        ]
        results = run_signals(signals, DEFAULT_CONFIG)

        assert len(results) == 3
        assert results[0][1].action_type == "enter"
        assert results[1][1].action_type == "short_sell"
        assert results[2][1].action_type == "short_buy"

    def test_empty_signals(self):
        """空信号序列。"""
        results = run_signals([], DEFAULT_CONFIG)
        assert results == []


# ── 不可变性 ──


class TestImmutability:
    """不可变性测试。"""

    def test_position_immutable(self):
        """Position 是 frozen dataclass。"""
        pos = Position()
        try:
            pos.phase = Phase.COST_POSITIVE  # type: ignore
            assert False, "Should raise"
        except AttributeError:
            pass

    def test_signal_immutable(self):
        """Signal 是 frozen dataclass。"""
        sig = _buy_signal()
        try:
            sig.price = 20.0  # type: ignore
            assert False, "Should raise"
        except AttributeError:
            pass

    def test_process_does_not_mutate_input(self):
        """process_signal 不修改输入。"""
        pos = Position()
        sig = _buy_signal(price=10.0)

        pos_before_phase = pos.phase
        process_signal(pos, sig, DEFAULT_CONFIG)

        assert pos.phase == pos_before_phase
