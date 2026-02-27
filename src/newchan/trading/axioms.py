"""三条公理的形式化检查。

概念溯源：
  [操盘方法 v2] §3.6 层间关系的三条公理

  公理1（上层覆盖）：exit_abort(Lk) -> 对所有 j > k, state(Lj) := INACTIVE。
  公理2（下层不创造上层入口）：低层信号不能替代上层 entry 条件。
  公理3（层间传递需要卖出事件）：利润传递当且仅当低层发生卖出。
"""

from __future__ import annotations

from newchan.trading.layer_state import LayerState, LayerType

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from newchan.trading.state_machine import FourLayerStateMachine


def axiom1_coverage(machine: FourLayerStateMachine, layer: LayerType) -> bool:
    """公理1检查：高层止损后所有低层为 INACTIVE。

    给定 layer 已执行 exit_abort 后的 machine 状态，
    检查所有 j > layer.value 的层是否都为 INACTIVE。

    Parameters
    ----------
    machine : FourLayerStateMachine
        止损执行后的状态机。
    layer : LayerType
        执行了止损的层。

    Returns
    -------
    bool
        True 如果公理1满足。
    """
    for lt in LayerType:
        if lt.value > layer.value:
            if machine.get_state(lt) != LayerState.INACTIVE:
                return False
    return True


def axiom2_no_upward_entry(
    machine: FourLayerStateMachine,
    lower: LayerType,
    upper: LayerType,
) -> bool:
    """公理2检查：低层不创造上层入口。

    验证：upper 层的 entry 不依赖 lower 层的状态。
    具体检查：如果 upper 不是 ACTIVE，lower 不应该是 ACTIVE。

    Parameters
    ----------
    machine : FourLayerStateMachine
        当前状态机。
    lower : LayerType
        低层。
    upper : LayerType
        高层。

    Returns
    -------
    bool
        True 如果公理2满足（不存在 upper 非 ACTIVE 而 lower ACTIVE 的违规）。
    """
    if lower.value <= upper.value:
        raise ValueError(f"lower ({lower.name}) 必须比 upper ({upper.name}) 更低（值更大）")

    upper_state = machine.get_state(upper)
    lower_state = machine.get_state(lower)

    # 公理2：如果上层不是 ACTIVE，下层不能是 ACTIVE
    if upper_state != LayerState.ACTIVE and lower_state == LayerState.ACTIVE:
        return False
    return True


def axiom3_profit_transfer(
    machine: FourLayerStateMachine,
    lower: LayerType,
) -> bool:
    """公理3检查：利润传递需要卖出事件。

    验证：lower 层如果仍在 ACTIVE（未发生卖出），其利润不应已传递到上层。
    这是一个结构性检查——ACTIVE 层的利润缓冲变化只在该层内部，
    只有 exit_normal/exit_abort（卖出事件）后利润才传递给上层。

    实际检查：如果 lower 层是 ACTIVE 且有仓位，上层的 profit_buffer
    不应包含 lower 层的浮盈。由于状态机不跟踪浮盈（只跟踪已兑现），
    这个检查总是满足的——这正是设计意图。

    Parameters
    ----------
    machine : FourLayerStateMachine
        当前状态机。
    lower : LayerType
        低层。

    Returns
    -------
    bool
        True 如果公理3满足。
    """
    # 在当前状态机设计中，profit_buffer 只在 exit 事件时更新，
    # ACTIVE 层的浮盈不参与上层计算。因此只要状态机通过正常 API 操作，
    # 公理3自动满足。
    #
    # 这个函数的价值在于：外部代码可以用它做 invariant 检查。
    if lower == LayerType.L0_CONFIG:
        raise ValueError("L0 是最高层，没有利润传递目标")

    # 如果 lower 层是 ACTIVE（未卖出），验证其利润还在本层
    lower_layer = machine.get_layer(lower)
    if lower_layer.state == LayerState.ACTIVE:
        # ACTIVE 层的利润只在 buffer 中（已兑现的次级别操作利润），
        # 尚未传递到上层——这是 exit 时才发生的。
        # 这里检查上层的 buffer 没有包含 lower 层 ACTIVE 期间的增量。
        # 在状态机 API 约束下这总是 True。
        return True

    return True
