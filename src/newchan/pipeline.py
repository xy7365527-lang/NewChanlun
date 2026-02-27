"""交易管道 — topology + nesting + trading 三模块连接。

概念溯源：
  [操盘方法 v2] §一 完整交易管道
  配置扫描 -> 区间套定位 -> 共振仓位 -> 状态机入退场 -> 降成本追踪

管道中每个函数只做一件事，返回新的 TradingContext（不可变语义）。
CostTracker 是唯一的可变组件——通过 copy 保证 ctx 替换时旧 ctx 不受影响。
"""

from __future__ import annotations

import copy
from dataclasses import dataclass, field
from typing import Callable

from newchan.nesting.bsp import BSP, is_negated
from newchan.nesting.horizontal import (
    HorizontalNesting,
    advance,
    create_horizontal_nesting,
    is_blocked,
)
from newchan.nesting.resonance import (
    ResonanceLevel,
    ResonanceSignal,
    SignalLayer,
    classify_resonance,
    resonance_check,
    resonance_strength,
)
from newchan.topology.config_space import Configuration, polarity_index
from newchan.trading.cost_reduction import CostTracker
from newchan.trading.layer_state import LayerType
from newchan.trading.position_sizing import (
    PositionSizer,
    Signal,
)
from newchan.trading.position_sizing import Layer as SignalLayerEnum
from newchan.trading.state_machine import FourLayerStateMachine, TransitionResult


# -- SignalLayer -> position_sizing.Layer 映射 --

_RESONANCE_TO_SIGNAL_LAYER: dict[SignalLayer, SignalLayerEnum] = {
    SignalLayer.CONFIG: SignalLayerEnum.L0,
    SignalLayer.INDEPENDENT_EDGE: SignalLayerEnum.L1,
    SignalLayer.DERIVED_EDGE: SignalLayerEnum.L1,
    SignalLayer.UNDERLYING: SignalLayerEnum.L3,
}


@dataclass(slots=True)
class TradingContext:
    """交易上下文。

    除 cost_tracker 外，所有字段在管道函数中通过创建新 ctx 替换。
    cost_tracker 是可变的——管道函数在操作前 deep copy 以保证旧 ctx 不变。

    Attributes
    ----------
    config : Configuration
        当前 K4 配置。
    polarity : int
        极性指数 S in {-3,...,+3}。
    horizontal : HorizontalNesting
        横向区间套状态。
    state_machine : FourLayerStateMachine
        四层递归状态机。
    position_sizer : PositionSizer
        仓位确定性函数。
    cost_tracker : CostTracker
        降成本追踪（可变组件）。
    timestamp : float
        当前时刻。
    """

    config: Configuration
    polarity: int
    horizontal: HorizontalNesting
    state_machine: FourLayerStateMachine
    position_sizer: PositionSizer
    cost_tracker: CostTracker
    timestamp: float


def _replace_ctx(ctx: TradingContext, **kwargs: object) -> TradingContext:
    """创建新的 TradingContext，仅替换指定字段。

    cost_tracker 如果未显式传入则 deep copy，保证旧 ctx 不受影响。
    """
    return TradingContext(
        config=kwargs.get("config", ctx.config),  # type: ignore[arg-type]
        polarity=kwargs.get("polarity", ctx.polarity),  # type: ignore[arg-type]
        horizontal=kwargs.get("horizontal", ctx.horizontal),  # type: ignore[arg-type]
        state_machine=kwargs.get("state_machine", ctx.state_machine),  # type: ignore[arg-type]
        position_sizer=kwargs.get("position_sizer", ctx.position_sizer),  # type: ignore[arg-type]
        cost_tracker=kwargs.get("cost_tracker", copy.deepcopy(ctx.cost_tracker)),  # type: ignore[arg-type]
        timestamp=kwargs.get("timestamp", ctx.timestamp),  # type: ignore[arg-type]
    )


# ── 工厂 ──────────────────────────────────────────────────────


def create_context(
    config: Configuration,
    timestamp: float = 0.0,
) -> TradingContext:
    """从配置创建初始交易上下文。"""
    return TradingContext(
        config=config,
        polarity=polarity_index(config),
        horizontal=create_horizontal_nesting(),
        state_machine=FourLayerStateMachine(),
        position_sizer=PositionSizer(),
        cost_tracker=CostTracker(),
        timestamp=timestamp,
    )


# ── 配置扫描 ─────────────────────────────────────────────────


def scan_configuration(
    ctx: TradingContext,
) -> tuple[TradingContext, list[str]]:
    """扫描当前配置的极性，返回可能的交易方向。

    S > 0 -> buy (risk-on)
    S < 0 -> sell (risk-off)
    S = 0 -> neutral (等待)

    Returns
    -------
    tuple[TradingContext, list[str]]
        (极性更新后的 ctx, 方向列表)。
    """
    s = polarity_index(ctx.config)
    new_ctx = _replace_ctx(ctx, polarity=s)

    if s > 0:
        directions = ["buy"]
    elif s < 0:
        directions = ["sell"]
    else:
        directions = ["neutral"]

    return new_ctx, directions


# ── 区间套定位 ────────────────────────────────────────────────


def locate_bsp(
    ctx: TradingContext,
    bsp: BSP,
    target: str = "",
) -> TradingContext:
    """定位买卖点，推进横向区间套。

    调用 nesting.advance 更新 horizontal 状态。

    Parameters
    ----------
    ctx : TradingContext
        当前上下文。
    bsp : BSP
        当前步确认的买卖点。
    target : str
        本步产出的目标标识。
    """
    new_horizontal = advance(ctx.horizontal, bsp, target)
    return _replace_ctx(ctx, horizontal=new_horizontal)


# ── 共振仓位 ─────────────────────────────────────────────────


def _resonance_to_signal(rs: ResonanceSignal, idx: int) -> Signal:
    """将 ResonanceSignal 映射到 position_sizing.Signal。"""
    layer = _RESONANCE_TO_SIGNAL_LAYER.get(rs.layer, SignalLayerEnum.L3)
    return Signal(
        id=f"r{idx}",
        layer=layer,
        weight=0.10,  # 共振信号统一权重
        confirmed=rs.bsp.bsp_type.is_present,
    )


def compute_position(
    ctx: TradingContext,
    resonance_signals: list[ResonanceSignal],
    time_tolerance_fn: Callable[[int, int], float],
) -> tuple[float, ResonanceLevel]:
    """从共振信号计算仓位。

    1. resonance_check -> 共振是否成立
    2. resonance_strength -> 共振强度
    3. classify_resonance -> THREE_LAYER/TWO_LAYER/SINGLE
    4. ResonanceSignal -> trading.Signal 映射
    5. PositionSizer.position -> 仓位比例

    Returns
    -------
    tuple[float, ResonanceLevel]
        (仓位比例, 共振层次)。
    """
    if not resonance_signals:
        return 0.0, ResonanceLevel.SINGLE

    is_resonant = resonance_check(resonance_signals, time_tolerance_fn)
    if not is_resonant:
        return 0.0, ResonanceLevel.SINGLE

    strength = resonance_strength(resonance_signals)
    level = classify_resonance(strength)

    signals = [
        _resonance_to_signal(rs, i)
        for i, rs in enumerate(resonance_signals)
    ]
    position = ctx.position_sizer.position(signals)

    return position, level


# ── 入退场 ───────────────────────────────────────────────────


def execute_entry(
    ctx: TradingContext,
    layer_type: LayerType,
    bsp_label: str,
    position: float,
) -> tuple[TradingContext, TransitionResult]:
    """执行入场。

    调用 state_machine.entry，更新 ctx。
    在 cost_tracker 记录入场成本。

    Parameters
    ----------
    ctx : TradingContext
        当前上下文。
    layer_type : LayerType
        目标层。
    bsp_label : str
        买点标识。
    position : float
        入场仓位。
    """
    result = ctx.state_machine.entry(layer_type, bsp_label, position)
    new_tracker = copy.deepcopy(ctx.cost_tracker)
    if position > 0:
        new_tracker.add_entry(position)
    return (
        _replace_ctx(
            ctx,
            state_machine=result.machine,
            cost_tracker=new_tracker,
        ),
        result,
    )


def execute_exit(
    ctx: TradingContext,
    layer_type: LayerType,
    abort: bool = False,
) -> tuple[TradingContext, TransitionResult]:
    """执行退出。

    abort=True -> exit_abort（止损）
    abort=False -> exit_normal（正常退出）
    """
    if abort:
        result = ctx.state_machine.exit_abort(layer_type)
    else:
        result = ctx.state_machine.exit_normal(layer_type)

    new_tracker = copy.deepcopy(ctx.cost_tracker)
    if result.realized_profit != 0 and new_tracker.total_entries > 0:
        revenue = max(result.realized_profit, 0.0)
        new_tracker.add_exit(revenue, 1.0)

    return (
        _replace_ctx(
            ctx,
            state_machine=result.machine,
            cost_tracker=new_tracker,
        ),
        result,
    )


# ── 否定检查 ─────────────────────────────────────────────────


def check_negation(
    ctx: TradingContext,
    current_price: float,
    bsp: BSP,
) -> bool:
    """检查买卖点是否被否定。

    调用 nesting.bsp.is_negated。
    """
    return is_negated(bsp, current_price)


# ── 聚合管道步骤 ─────────────────────────────────────────────


def pipeline_step(
    ctx: TradingContext,
    bsp: BSP | None = None,
    target: str = "",
    resonance_signals: list[ResonanceSignal] | None = None,
    time_tolerance_fn: Callable[[int, int], float] | None = None,
) -> TradingContext:
    """单步管道推进（聚合函数）。

    1. 如果有 BSP -> locate_bsp 推进 horizontal
    2. 如果 horizontal 完成且有共振信号 -> 计算仓位 -> execute_entry
    3. 如果无 BSP -> 维持当前状态（等待）

    Returns
    -------
    TradingContext
        推进后的上下文。
    """
    if bsp is None:
        return _replace_ctx(ctx)

    # 推进区间套
    new_ctx = locate_bsp(ctx, bsp, target)

    # 区间套完成且有共振信号 -> 入场
    if new_ctx.horizontal.is_complete and resonance_signals:
        tolerance_fn = time_tolerance_fn or _default_time_tolerance
        position, _level = compute_position(
            new_ctx, resonance_signals, tolerance_fn,
        )
        if position > 0:
            bsp_label = f"{bsp.bsp_type.value}@{bsp.edge_id}"
            new_ctx, _result = execute_entry(
                new_ctx, LayerType.L0_CONFIG, bsp_label, position,
            )

    return new_ctx


def _default_time_tolerance(level_i: int, level_j: int) -> float:
    """默认时间容差：高级别间容差更大。"""
    return float(abs(level_i - level_j) + 1) * 100.0
