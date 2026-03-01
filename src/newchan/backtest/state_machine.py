"""降成本状态机 — 5 状态 7 事件，包装 cost_reduction_fsm。

将任务指定的 5 状态（EMPTY/BASE/ADDED/SHORT_TRADE/FULL）
映射到现有 CostReductionFSM 的状态空间。

两种状态机的映射：
  任务定义          →  现有 FSM
  EMPTY             →  SCANNING
  BASE              →  POSITION_OPEN
  ADDED/SHORT_TRADE →  COST_REDUCING（短差进行中）
  FULL              →  PRINCIPAL_WITHDRAWN（本金已退出=满仓满融完成）

认识论标注：L0（映射关系从定义推导）。
谱系引用：267号操作方法论 v1。
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from newchan.backtest.types import (
    CostCurvePoint,
    StateMachineEvent,
    StateMachineState,
    TradeAction,
)
from newchan.trading.cost_reduction_fsm import (
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    transition,
)


# ═══════════════════════════════════════════════════════════════
# 状态映射
# ═══════════════════════════════════════════════════════════════


_FSM_TO_SM: dict[CostState, StateMachineState] = {
    CostState.SCANNING: StateMachineState.EMPTY,
    CostState.POSITION_OPEN: StateMachineState.BASE,
    CostState.COST_REDUCING: StateMachineState.SHORT_TRADE,
    CostState.PRINCIPAL_WITHDRAWN: StateMachineState.FULL,
    CostState.STOPPED_OUT: StateMachineState.EMPTY,
}

_EVENT_TO_FSM: dict[StateMachineEvent, FsmEventType] = {
    StateMachineEvent.BUY_SIGNAL: FsmEventType.BUY_POINT_CONFIRMED,
    StateMachineEvent.SELL_SIGNAL: FsmEventType.MAIN_LEVEL_SELL_POINT,
    StateMachineEvent.SHORT_ENTRY: FsmEventType.SUB_LEVEL_SELL_POINT,
    StateMachineEvent.SHORT_EXIT: FsmEventType.SUB_LEVEL_BUY_POINT,
    StateMachineEvent.STOP_LOSS: FsmEventType.BUY_POINT_NEGATED,
    StateMachineEvent.FULL_TRIGGER: FsmEventType.LEVEL_UPGRADE,
    StateMachineEvent.CLEAR_TRIGGER: FsmEventType.MAIN_LEVEL_SELL_POINT,
}


def map_fsm_state(state: CostState) -> StateMachineState:
    """将内部 FSM 状态映射到任务定义的 5 状态。"""
    return _FSM_TO_SM.get(state, StateMachineState.EMPTY)


# ═══════════════════════════════════════════════════════════════
# 状态机包装
# ═══════════════════════════════════════════════════════════════


@dataclass
class CostReductionStateMachine:
    """降成本状态机 — 包装 CostReductionFSM，提供 5 状态 7 事件接口。

    不可变内核：每次 process_event 返回新的 CostReductionStateMachine。

    Attributes
    ----------
    fsm : CostReductionFSM
        内部 FSM（不可变）。
    actions : tuple[TradeAction, ...]
        已记录的操作。
    cost_curve : tuple[CostCurvePoint, ...]
        降成本曲线。
    """

    fsm: CostReductionFSM
    actions: tuple[TradeAction, ...]
    cost_curve: tuple[CostCurvePoint, ...]

    @staticmethod
    def create(
        initial_capital: float = 1_000_000.0,
        margin_ratio: float = 1.0,
        short_diff_ratio: float = 0.1,
    ) -> CostReductionStateMachine:
        """创建初始状态机。"""
        margin_amount = initial_capital * margin_ratio
        fsm = CostReductionFSM.create(
            own_capital=initial_capital,
            margin_amount=margin_amount,
            sub_ratio=short_diff_ratio,
        )
        return CostReductionStateMachine(
            fsm=fsm,
            actions=(),
            cost_curve=(),
        )

    @property
    def state(self) -> StateMachineState:
        """当前状态（映射后）。"""
        return map_fsm_state(self.fsm.state)

    @property
    def cost_basis(self) -> float:
        """当前持仓成本。"""
        return self.fsm.cost_basis

    @property
    def total_shares(self) -> float:
        """总持仓。"""
        return self.fsm.total_shares

    @property
    def cumulative_recovered(self) -> float:
        """累计回收金额。"""
        return self.fsm.cumulative_recovered

    def process_event(
        self,
        event: StateMachineEvent,
        price: float,
        bar_idx: int,
        level: str = "1",
        trigger: str = "",
    ) -> CostReductionStateMachine:
        """处理事件，返回新状态机。

        Parameters
        ----------
        event : StateMachineEvent
            状态机事件。
        price : float
            成交价。
        bar_idx : int
            当前 bar 索引。
        level : str
            级别标识。
        trigger : str
            触发原因描述。

        Returns
        -------
        CostReductionStateMachine
            新状态机实例。
        """
        fsm_event_type = _EVENT_TO_FSM.get(event)
        if fsm_event_type is None:
            return self

        fsm_event = FsmEvent(
            event_type=fsm_event_type,
            price=price,
            level=level,
        )

        new_fsm = transition(self.fsm, fsm_event)

        # 记录操作
        action_type = _event_to_action_type(event)
        quantity = _compute_quantity(self.fsm, new_fsm, event)
        action = TradeAction(
            bar_idx=bar_idx,
            action=action_type,
            price=price,
            quantity=quantity,
            cost_basis=new_fsm.cost_basis,
            trigger=trigger,
        )

        # 更新降成本曲线
        curve_point = CostCurvePoint(
            bar_idx=bar_idx,
            cost_basis=new_fsm.cost_basis,
            total_shares=new_fsm.total_shares,
            cumulative_recovered=new_fsm.cumulative_recovered,
        )

        return CostReductionStateMachine(
            fsm=new_fsm,
            actions=self.actions + (action,),
            cost_curve=self.cost_curve + (curve_point,),
        )


def _event_to_action_type(event: StateMachineEvent) -> str:
    """事件 → 操作类型字符串。"""
    mapping = {
        StateMachineEvent.BUY_SIGNAL: "buy",
        StateMachineEvent.SELL_SIGNAL: "sell",
        StateMachineEvent.SHORT_ENTRY: "short_sell",
        StateMachineEvent.SHORT_EXIT: "short_cover",
        StateMachineEvent.STOP_LOSS: "stop_loss",
        StateMachineEvent.FULL_TRIGGER: "full_trigger",
        StateMachineEvent.CLEAR_TRIGGER: "clear",
    }
    return mapping.get(event, "unknown")


def _compute_quantity(
    old_fsm: CostReductionFSM,
    new_fsm: CostReductionFSM,
    event: StateMachineEvent,
) -> float:
    """计算操作的数量。"""
    if event == StateMachineEvent.BUY_SIGNAL:
        return new_fsm.total_shares
    if event in (StateMachineEvent.SHORT_ENTRY, StateMachineEvent.SHORT_EXIT):
        return old_fsm.total_shares * old_fsm.sub_ratio
    if event in (StateMachineEvent.SELL_SIGNAL, StateMachineEvent.STOP_LOSS,
                 StateMachineEvent.CLEAR_TRIGGER):
        return old_fsm.total_shares
    return 0.0
