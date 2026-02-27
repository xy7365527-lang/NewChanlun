"""层状态定义。

概念溯源：
  [操盘方法 v2] §3.1 形式化符号
  state(Lk) in {INACTIVE, ACTIVE, SUSPENDED}

  [操盘方法 v2] §二 四层降成本递归
  L0 配置层 / L1 跨角边层 / L2 角内轮动 / L3 标的内操作
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Optional


class LayerState(Enum):
    """层运行状态。

    INACTIVE: 该层未启动或已清仓。重新启动需要完整的 entry 条件。
    ACTIVE: 该层正在运行。
    SUSPENDED: 该层的上层已退出，该层暂停等待新的上层入口。
      -- 不新开仓位，已有仓位按止损管理，出现一卖可减仓但不轮动。
    """

    INACTIVE = "INACTIVE"
    ACTIVE = "ACTIVE"
    SUSPENDED = "SUSPENDED"


class LayerType(Enum):
    """四层类型，从上到下排列。

    值越小层级越高。L0 是最高层（配置层），L3 是最底层（标的内操作）。
    """

    L0_CONFIG = 0   # 配置层
    L1_EDGE = 1     # 跨角/边层
    L2_CORNER = 2   # 角内轮动
    L3_STOCK = 3    # 标的内操作


# 合法的状态转换表
# key: (当前状态, 目标状态), value: 是否合法
_VALID_TRANSITIONS: frozenset[tuple[LayerState, LayerState]] = frozenset({
    (LayerState.INACTIVE, LayerState.ACTIVE),      # entry
    (LayerState.ACTIVE, LayerState.INACTIVE),       # exit_normal / exit_abort
    (LayerState.ACTIVE, LayerState.SUSPENDED),      # 上层退出
    (LayerState.SUSPENDED, LayerState.ACTIVE),      # 上层重新 entry
    (LayerState.SUSPENDED, LayerState.INACTIVE),    # 上层 abort / 超时清仓
    # ACTIVE -> ACTIVE 不在此表：角切换/标的切换是语义层面的"切换"，
    # 状态机层面 state 不变。
})


def is_valid_transition(from_state: LayerState, to_state: LayerState) -> bool:
    """检查状态转换是否合法。"""
    if from_state == to_state:
        return True
    return (from_state, to_state) in _VALID_TRANSITIONS


@dataclass(slots=True)
class Layer:
    """单层状态。

    Attributes
    ----------
    layer_type : LayerType
        层类型。
    state : LayerState
        当前状态。
    entry_event : str | None
        入场事件描述（买点标识）。
    profit_buffer : float
        该层累积的利润缓冲（已兑现利润 - 已消耗止损）。
    position : float
        当前仓位（归一化到 [0, 1]）。
    """

    layer_type: LayerType
    state: LayerState = LayerState.INACTIVE
    entry_event: Optional[str] = None
    profit_buffer: float = 0.0
    position: float = 0.0

    def _with(self, **kwargs: object) -> Layer:
        """返回新 Layer 实例，仅替换指定字段（不可变语义）。"""
        return Layer(
            layer_type=kwargs.get("layer_type", self.layer_type),  # type: ignore[arg-type]
            state=kwargs.get("state", self.state),  # type: ignore[arg-type]
            entry_event=kwargs.get("entry_event", self.entry_event),  # type: ignore[arg-type]
            profit_buffer=kwargs.get("profit_buffer", self.profit_buffer),  # type: ignore[arg-type]
            position=kwargs.get("position", self.position),  # type: ignore[arg-type]
        )
