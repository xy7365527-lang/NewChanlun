"""四层递归状态机。

概念溯源：
  [操盘方法 v2] §三 四层递归边界条件完整枚举
  - §3.2 L0 配置层状态转换
  - §3.3 L1 跨角/边层状态转换
  - §3.4 L2 角内轮动状态转换
  - §3.5 L3 标的内操作状态转换
  - §3.6 层间关系三条公理
  - §3.8 SUSPENDED 状态精确语义
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Mapping

from newchan.trading.layer_state import (
    Layer,
    LayerState,
    LayerType,
    is_valid_transition,
)


class IllegalTransitionError(Exception):
    """非法状态转换。"""


class AxiomViolationError(Exception):
    """公理违反。"""


@dataclass(frozen=True, slots=True)
class TransitionResult:
    """状态转换结果（不可变）。

    Attributes
    ----------
    machine : FourLayerStateMachine
        转换后的新状态机实例。
    realized_profit : float
        本次转换实现的利润（正=盈利，负=亏损）。
    """

    machine: FourLayerStateMachine
    realized_profit: float


class FourLayerStateMachine:
    """四层递归状态机。

    不可变设计：每次状态转换返回新的 FourLayerStateMachine 实例。
    """

    __slots__ = ("_layers",)

    def __init__(
        self,
        layers: Mapping[LayerType, Layer] | None = None,
    ) -> None:
        if layers is not None:
            self._layers: dict[LayerType, Layer] = dict(layers)
        else:
            self._layers = {
                lt: Layer(layer_type=lt) for lt in LayerType
            }

    def _clone_with(self, **updates: Layer) -> FourLayerStateMachine:
        """返回新实例，替换指定层。"""
        new_layers = dict(self._layers)
        for lt_name, layer in updates.items():
            lt = LayerType[lt_name]
            new_layers[lt] = layer
        return FourLayerStateMachine(layers=new_layers)

    def _replace_layer(self, layer_type: LayerType, new_layer: Layer) -> FourLayerStateMachine:
        """返回新实例，替换单个层。"""
        new_layers = dict(self._layers)
        new_layers[layer_type] = new_layer
        return FourLayerStateMachine(layers=new_layers)

    def get_state(self, layer_type: LayerType) -> LayerState:
        """获取指定层的当前状态。"""
        return self._layers[layer_type].state

    def get_layer(self, layer_type: LayerType) -> Layer:
        """获取指定层的完整状态。"""
        return self._layers[layer_type]

    # ── entry ────────────────────────────────────────────────────────

    def entry(self, layer_type: LayerType, bsp: str, position: float = 0.0) -> TransitionResult:
        """入场。

        Parameters
        ----------
        layer_type : LayerType
            目标层。
        bsp : str
            买点标识（如 "配置空间一买"、"独立边 e1 一买" 等）。
        position : float
            入场仓位。

        Returns
        -------
        TransitionResult
            新的状态机和实现利润。

        Raises
        ------
        IllegalTransitionError
            如果当前状态不允许 entry。
        AxiomViolationError
            如果违反公理2（下层不创造上层入口）。
        """
        layer = self._layers[layer_type]

        # 状态合法性
        if not is_valid_transition(layer.state, LayerState.ACTIVE):
            raise IllegalTransitionError(
                f"{layer_type.name}: 不能从 {layer.state.value} 转换到 ACTIVE"
            )

        # 公理2：下层 entry 需要上层为 ACTIVE（L0 除外——它没有上层）
        if layer_type != LayerType.L0_CONFIG:
            parent_type = LayerType(layer_type.value - 1)
            parent_state = self._layers[parent_type].state
            if parent_state != LayerState.ACTIVE:
                raise AxiomViolationError(
                    f"公理2违反：{layer_type.name} entry 需要上层 "
                    f"{parent_type.name} 为 ACTIVE，当前为 {parent_state.value}"
                )

        new_layer = layer._with(
            state=LayerState.ACTIVE,
            entry_event=bsp,
            position=position,
        )
        return TransitionResult(
            machine=self._replace_layer(layer_type, new_layer),
            realized_profit=0.0,
        )

    # ── exit_normal ──────────────────────────────────────────────────

    def exit_normal(self, layer_type: LayerType) -> TransitionResult:
        """正常退出（一卖确认）。

        效果：
        - L0: 所有下层进入 SUSPENDED。
        - L1: 角切换——L2/L3 进入 SUSPENDED。L1 自身保持 ACTIVE（等待新角）。
        - L2: 标的切换——L3 回到 INACTIVE。L2 自身保持 ACTIVE（等待新标的）。
        - L3: 减仓/清仓。利润传递给 L2。

        Returns
        -------
        TransitionResult
            含实现利润。
        """
        layer = self._layers[layer_type]
        if layer.state != LayerState.ACTIVE:
            raise IllegalTransitionError(
                f"{layer_type.name}: exit_normal 需要 ACTIVE，当前 {layer.state.value}"
            )

        realized = layer.profit_buffer + layer.position * 0.1  # 简化：利润 = buffer + 仓位估值

        if layer_type == LayerType.L0_CONFIG:
            # L0 正常退出 → 自身 INACTIVE，所有下层 SUSPENDED
            new_l0 = layer._with(state=LayerState.INACTIVE, position=0.0, entry_event=None)
            new_layers = dict(self._layers)
            new_layers[LayerType.L0_CONFIG] = new_l0
            for lt in (LayerType.L1_EDGE, LayerType.L2_CORNER, LayerType.L3_STOCK):
                child = new_layers[lt]
                if child.state == LayerState.ACTIVE:
                    new_layers[lt] = child._with(state=LayerState.SUSPENDED)
            return TransitionResult(
                machine=FourLayerStateMachine(layers=new_layers),
                realized_profit=realized,
            )

        if layer_type == LayerType.L1_EDGE:
            # L1 正常退出 = 角切换。L1 保持 ACTIVE，L2/L3 进入 SUSPENDED。
            new_l1 = layer._with(
                profit_buffer=layer.profit_buffer + realized,
                entry_event=None,  # 清空旧角标识，等待新角
                position=0.0,
            )
            new_layers = dict(self._layers)
            new_layers[LayerType.L1_EDGE] = new_l1
            for lt in (LayerType.L2_CORNER, LayerType.L3_STOCK):
                child = new_layers[lt]
                if child.state == LayerState.ACTIVE:
                    new_layers[lt] = child._with(state=LayerState.SUSPENDED)
            return TransitionResult(
                machine=FourLayerStateMachine(layers=new_layers),
                realized_profit=realized,
            )

        if layer_type == LayerType.L2_CORNER:
            # L2 正常退出 = 标的切换。L2 保持 ACTIVE，L3 回到 INACTIVE。
            new_l2 = layer._with(
                profit_buffer=layer.profit_buffer + realized,
                entry_event=None,
                position=0.0,
            )
            new_layers = dict(self._layers)
            new_layers[LayerType.L2_CORNER] = new_l2
            l3 = new_layers[LayerType.L3_STOCK]
            if l3.state == LayerState.ACTIVE:
                new_layers[LayerType.L3_STOCK] = l3._with(
                    state=LayerState.INACTIVE, position=0.0, entry_event=None
                )
            return TransitionResult(
                machine=FourLayerStateMachine(layers=new_layers),
                realized_profit=realized,
            )

        # L3 正常退出
        new_l3 = layer._with(
            state=LayerState.INACTIVE,
            position=0.0,
            entry_event=None,
            profit_buffer=0.0,
        )
        # 利润传递给 L2
        l2 = self._layers[LayerType.L2_CORNER]
        new_l2 = l2._with(profit_buffer=l2.profit_buffer + realized)
        new_layers = dict(self._layers)
        new_layers[LayerType.L3_STOCK] = new_l3
        new_layers[LayerType.L2_CORNER] = new_l2
        return TransitionResult(
            machine=FourLayerStateMachine(layers=new_layers),
            realized_profit=realized,
        )

    # ── exit_abort ───────────────────────────────────────────────────

    def exit_abort(self, layer_type: LayerType) -> TransitionResult:
        """止损退出（买点被否定）。

        公理1（上层覆盖）：高层止损强制覆盖所有低层。
        exit_abort(Lk) -> 对所有 j > k, state(Lj) := INACTIVE。

        L1 止损后自身回到 ACTIVE（仍在跨角轮动中等待新角）。
        L0/L2/L3 止损后回到 INACTIVE。

        Returns
        -------
        TransitionResult
            含实现亏损（负值）。
        """
        layer = self._layers[layer_type]
        if layer.state != LayerState.ACTIVE:
            raise IllegalTransitionError(
                f"{layer_type.name}: exit_abort 需要 ACTIVE，当前 {layer.state.value}"
            )

        # 止损亏损 = 负的仓位价值（简化模型）
        loss = -layer.position

        new_layers = dict(self._layers)

        if layer_type == LayerType.L1_EDGE:
            # L1 止损：自身保持 ACTIVE（等新角），清空当前角信息
            new_layers[LayerType.L1_EDGE] = layer._with(
                position=0.0,
                entry_event=None,
                profit_buffer=layer.profit_buffer + loss,
            )
        else:
            # L0/L2/L3 止损：自身回到 INACTIVE
            new_layers[layer_type] = layer._with(
                state=LayerState.INACTIVE,
                position=0.0,
                entry_event=None,
                profit_buffer=0.0,
            )

        # 公理1：所有低层强制 INACTIVE
        for lt in LayerType:
            if lt.value > layer_type.value:
                child = new_layers[lt]
                if child.state != LayerState.INACTIVE:
                    new_layers[lt] = child._with(
                        state=LayerState.INACTIVE,
                        position=0.0,
                        entry_event=None,
                        profit_buffer=0.0,
                    )

        return TransitionResult(
            machine=FourLayerStateMachine(layers=new_layers),
            realized_profit=loss,
        )

    # ── cascade_abort ────────────────────────────────────────────────

    def cascade_abort(self, layer_type: LayerType) -> TransitionResult:
        """高层覆盖——强制所有低层 INACTIVE（公理1 的直接实现）。

        与 exit_abort 的区别：cascade_abort 只处理低层，不改变当前层状态。
        用于上层退出信号直接传递到所有低层的场景（路径 F）。
        """
        new_layers = dict(self._layers)
        total_loss = 0.0

        for lt in LayerType:
            if lt.value > layer_type.value:
                child = new_layers[lt]
                if child.state != LayerState.INACTIVE:
                    total_loss -= child.position
                    new_layers[lt] = child._with(
                        state=LayerState.INACTIVE,
                        position=0.0,
                        entry_event=None,
                        profit_buffer=0.0,
                    )

        return TransitionResult(
            machine=FourLayerStateMachine(layers=new_layers),
            realized_profit=total_loss,
        )

    # ── suspend_below ────────────────────────────────────────────────

    def suspend_below(self, layer_type: LayerType) -> FourLayerStateMachine:
        """上层退出时，所有低层中 ACTIVE 的进入 SUSPENDED。

        SUSPENDED 语义（§3.8）：
        - 不新开仓位
        - 已有仓位按止损管理
        - 出现一卖可减仓但不轮动
        - 等待上层重新 entry 后恢复 ACTIVE
        """
        new_layers = dict(self._layers)

        for lt in LayerType:
            if lt.value > layer_type.value:
                child = new_layers[lt]
                if child.state == LayerState.ACTIVE:
                    new_layers[lt] = child._with(state=LayerState.SUSPENDED)

        return FourLayerStateMachine(layers=new_layers)

    # ── 辅助 ─────────────────────────────────────────────────────────

    def all_inactive(self) -> bool:
        """所有层是否都 INACTIVE。"""
        return all(l.state == LayerState.INACTIVE for l in self._layers.values())

    def active_layers(self) -> list[LayerType]:
        """返回所有 ACTIVE 层（按层级从高到低排列）。"""
        return sorted(
            [lt for lt, l in self._layers.items() if l.state == LayerState.ACTIVE],
            key=lambda lt: lt.value,
        )

    def __repr__(self) -> str:
        states = ", ".join(
            f"{lt.name}={self._layers[lt].state.value}"
            for lt in LayerType
        )
        return f"FourLayerStateMachine({states})"
