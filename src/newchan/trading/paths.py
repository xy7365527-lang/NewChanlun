"""异常路径枚举与执行。

概念溯源：
  [操盘方法 v2] §3.7 异常路径枚举

  路径 A：正常全周期
  路径 B：底层止损，上层不受影响
  路径 C：角级别止损，切换角
  路径 D：配置层止损，全面清仓
  路径 E：底层预警触发逐层减仓
  路径 F：层间时序错位
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from newchan.trading.layer_state import LayerState, LayerType

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from newchan.trading.state_machine import FourLayerStateMachine, TransitionResult


class PathType(Enum):
    """六条路径类型。"""

    NORMAL_FULL = "A"          # 正常全周期
    BOTTOM_STOP = "B"          # 底层止损，上层不受影响
    CORNER_STOP = "C"          # 角级别止损，切换角
    CONFIG_STOP = "D"          # 配置层止损，全面清仓
    EARLY_WARNING = "E"        # 底层预警触发逐层减仓
    TIME_MISMATCH = "F"        # 层间时序错位


@dataclass(frozen=True, slots=True)
class Event:
    """事件。

    Attributes
    ----------
    layer : LayerType
        事件发生的层。
    action : str
        事件类型: "entry", "exit_normal", "exit_abort"。
    """

    layer: LayerType
    action: str


def classify_path(event_sequence: list[Event]) -> PathType:
    """根据事件序列分类路径。

    Parameters
    ----------
    event_sequence : list[Event]
        按时间顺序排列的事件列表。

    Returns
    -------
    PathType
        路径类型。
    """
    if not event_sequence:
        raise ValueError("事件序列不能为空")

    abort_layers = [e.layer for e in event_sequence if e.action == "exit_abort"]
    exit_normal_layers = [e.layer for e in event_sequence if e.action == "exit_normal"]

    # 路径 D：配置层止损
    if LayerType.L0_CONFIG in abort_layers:
        return PathType.CONFIG_STOP

    # 路径 C：角级别止损（L1 abort）
    if LayerType.L1_EDGE in abort_layers:
        return PathType.CORNER_STOP

    # 路径 B：底层止损（L3 abort），上层无 abort
    if LayerType.L3_STOCK in abort_layers and LayerType.L1_EDGE not in abort_layers:
        return PathType.BOTTOM_STOP

    # 路径 F：层间时序错位 —— L1 exit_normal 但 L3 未先 exit
    # 检测：L1 exit_normal 出现时，序列中此前没有 L3 exit
    for i, e in enumerate(event_sequence):
        if e.layer == LayerType.L1_EDGE and e.action == "exit_normal":
            prior_l3_exits = [
                ev for ev in event_sequence[:i]
                if ev.layer == LayerType.L3_STOCK
                and ev.action in ("exit_normal", "exit_abort")
            ]
            if not prior_l3_exits:
                return PathType.TIME_MISMATCH

    # 路径 E：多个底层 exit_normal（预警信号）后上层 exit_normal
    l3_normal_count = sum(
        1 for e in event_sequence
        if e.layer == LayerType.L3_STOCK and e.action == "exit_normal"
    )
    l0_normal = any(
        e.layer == LayerType.L0_CONFIG and e.action == "exit_normal"
        for e in event_sequence
    )
    if l3_normal_count >= 2 and l0_normal:
        return PathType.EARLY_WARNING

    # 路径 A：正常全周期 —— 有完整的 entry-exit_normal 序列，无 abort
    if not abort_layers:
        return PathType.NORMAL_FULL

    # 默认：底层止损
    return PathType.BOTTOM_STOP


def execute_path(
    machine: FourLayerStateMachine,
    path_type: PathType,
) -> TransitionResult:
    """执行路径对应的状态转换。

    根据路径类型，在状态机上执行相应的转换序列。
    这是一个便捷函数——每条路径的转换最终都通过状态机的 entry/exit_normal/exit_abort 方法完成。

    Parameters
    ----------
    machine : FourLayerStateMachine
        当前状态机。
    path_type : PathType
        路径类型。

    Returns
    -------
    TransitionResult
        转换结果。
    """
    from newchan.trading.state_machine import TransitionResult

    if path_type == PathType.CONFIG_STOP:
        # 路径 D：配置层止损 → 全面清仓
        result = machine.exit_abort(LayerType.L0_CONFIG)
        return result

    if path_type == PathType.CORNER_STOP:
        # 路径 C：角级别止损 → L2/L3 INACTIVE，L1 等新角
        result = machine.exit_abort(LayerType.L1_EDGE)
        return result

    if path_type == PathType.BOTTOM_STOP:
        # 路径 B：底层止损 → L3 INACTIVE，L2 寻找新标的
        result = machine.exit_abort(LayerType.L3_STOCK)
        return result

    if path_type == PathType.TIME_MISMATCH:
        # 路径 F：L1 退出信号直接传递给低层 → cascade_abort
        result = machine.cascade_abort(LayerType.L1_EDGE)
        return result

    if path_type == PathType.EARLY_WARNING:
        # 路径 E：逐层减仓 → 从 L3 开始逐层 exit_normal
        total_profit = 0.0
        current = machine

        # 逐层退出（如果该层 ACTIVE）
        for lt in reversed(list(LayerType)):  # L3 → L2 → L1 → L0
            if current.get_state(lt) == LayerState.ACTIVE:
                result = current.exit_normal(lt)
                total_profit += result.realized_profit
                current = result.machine

        return TransitionResult(machine=current, realized_profit=total_profit)

    if path_type == PathType.NORMAL_FULL:
        # 路径 A：正常退出序列 L3 → L2 → L1 → L0
        total_profit = 0.0
        current = machine

        for lt in reversed(list(LayerType)):
            if current.get_state(lt) == LayerState.ACTIVE:
                result = current.exit_normal(lt)
                total_profit += result.realized_profit
                current = result.machine

        return TransitionResult(machine=current, realized_profit=total_profit)

    raise ValueError(f"未知路径类型: {path_type}")
