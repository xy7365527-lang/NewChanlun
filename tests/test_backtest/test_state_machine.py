"""state_machine.py 测试 — 降成本状态机 5 状态 7 事件。

认识论标注：L1（合成数据，验证管线正确性）。
"""

from __future__ import annotations

import pytest

from newchan.backtest.state_machine import (
    CostReductionStateMachine,
    map_fsm_state,
)
from newchan.backtest.types import StateMachineEvent, StateMachineState
from newchan.trading.cost_reduction_fsm import CostState


class TestFsmStateMapping:
    """FSM 状态 → 5 状态映射。"""

    def test_scanning_to_empty(self):
        assert map_fsm_state(CostState.SCANNING) == StateMachineState.EMPTY

    def test_position_open_to_base(self):
        assert map_fsm_state(CostState.POSITION_OPEN) == StateMachineState.BASE

    def test_cost_reducing_to_short_trade(self):
        assert map_fsm_state(CostState.COST_REDUCING) == StateMachineState.SHORT_TRADE

    def test_principal_withdrawn_to_full(self):
        assert map_fsm_state(CostState.PRINCIPAL_WITHDRAWN) == StateMachineState.FULL

    def test_stopped_out_to_empty(self):
        assert map_fsm_state(CostState.STOPPED_OUT) == StateMachineState.EMPTY


class TestStateMachineCreate:
    """状态机创建。"""

    def test_initial_state_is_empty(self):
        sm = CostReductionStateMachine.create()
        assert sm.state == StateMachineState.EMPTY

    def test_custom_capital(self):
        sm = CostReductionStateMachine.create(initial_capital=500_000.0)
        assert sm.fsm.own_capital == 500_000.0

    def test_no_actions_initially(self):
        sm = CostReductionStateMachine.create()
        assert sm.actions == ()
        assert sm.cost_curve == ()


class TestStateMachineTransitions:
    """状态机转移。"""

    def test_buy_signal_opens_position(self):
        """BUY_SIGNAL: EMPTY → BASE。"""
        sm = CostReductionStateMachine.create(initial_capital=100_000.0)
        sm2 = sm.process_event(
            StateMachineEvent.BUY_SIGNAL,
            price=100.0,
            bar_idx=0,
            trigger="type1_buy",
        )
        assert sm2.state == StateMachineState.BASE
        assert sm2.cost_basis == 100.0
        assert sm2.total_shares == 200_000.0 / 100.0  # 自有+融资
        assert len(sm2.actions) == 1
        assert sm2.actions[0].action == "buy"

    def test_short_entry_starts_short_trade(self):
        """SHORT_ENTRY: BASE → SHORT_TRADE。"""
        sm = CostReductionStateMachine.create(initial_capital=100_000.0)
        sm2 = sm.process_event(
            StateMachineEvent.BUY_SIGNAL,
            price=100.0, bar_idx=0,
        )
        sm3 = sm2.process_event(
            StateMachineEvent.SHORT_ENTRY,
            price=105.0, bar_idx=1,
            trigger="sub_sell",
        )
        assert sm3.state == StateMachineState.SHORT_TRADE
        assert len(sm3.actions) == 2
        assert sm3.actions[1].action == "short_sell"

    def test_short_exit_returns_to_base_or_stays(self):
        """SHORT_EXIT: 短差买回，成本降低。"""
        sm = CostReductionStateMachine.create(initial_capital=100_000.0)
        sm2 = sm.process_event(StateMachineEvent.BUY_SIGNAL, price=100.0, bar_idx=0)
        sm3 = sm2.process_event(StateMachineEvent.SHORT_ENTRY, price=105.0, bar_idx=1)
        sm4 = sm3.process_event(StateMachineEvent.SHORT_EXIT, price=95.0, bar_idx=2)
        # 买回后成本应该降低
        assert sm4.cost_basis < 100.0
        assert sm4.cumulative_recovered > 0.0
        assert len(sm4.actions) == 3
        assert sm4.actions[2].action == "short_cover"

    def test_stop_loss(self):
        """STOP_LOSS: 任何持仓 → STOPPED_OUT (→ EMPTY)。"""
        sm = CostReductionStateMachine.create()
        sm2 = sm.process_event(StateMachineEvent.BUY_SIGNAL, price=100.0, bar_idx=0)
        sm3 = sm2.process_event(StateMachineEvent.STOP_LOSS, price=80.0, bar_idx=1)
        # 止损后状态回到 EMPTY (STOPPED_OUT maps to EMPTY)
        assert sm3.state == StateMachineState.EMPTY
        assert len(sm3.actions) == 2
        assert sm3.actions[1].action == "stop_loss"

    def test_cost_curve_tracked(self):
        """每次事件都记录降成本曲线点。"""
        sm = CostReductionStateMachine.create(initial_capital=100_000.0)
        sm2 = sm.process_event(StateMachineEvent.BUY_SIGNAL, price=100.0, bar_idx=0)
        sm3 = sm2.process_event(StateMachineEvent.SHORT_ENTRY, price=105.0, bar_idx=1)
        sm4 = sm3.process_event(StateMachineEvent.SHORT_EXIT, price=95.0, bar_idx=2)
        assert len(sm4.cost_curve) == 3
        # 成本应该递减
        assert sm4.cost_curve[0].cost_basis == 100.0
        assert sm4.cost_curve[2].cost_basis < 100.0

    def test_immutability(self):
        """状态机不可变：process_event 返回新实例。"""
        sm = CostReductionStateMachine.create()
        sm2 = sm.process_event(StateMachineEvent.BUY_SIGNAL, price=100.0, bar_idx=0)
        assert sm.state == StateMachineState.EMPTY
        assert sm2.state == StateMachineState.BASE
        assert sm.actions == ()
        assert len(sm2.actions) == 1
