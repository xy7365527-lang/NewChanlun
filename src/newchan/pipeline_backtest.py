"""策略级回测引擎 — pipeline + backtest 集成。

与 BacktestEngine 的区别：
- BacktestEngine: 见 BSP 就入场（信号级）
- PipelineBacktestEngine: 配置扫描 -> 区间套 -> 共振 -> 状态机入场（策略级）

概念溯源：
  [操盘方法 v2] §一 完整交易管道
  回测时每 bar 推进 TradingContext，根据状态机状态决策入退场。

241号谱系：_build_resonance_signals 从双层简化升级为双模型并行共振构建。
  直积版：ConfigurationSpace polarity → CONFIG 层方向，BSP 本身 → INDEPENDENT_EDGE 层。
  纤维丛版：FiberSignalFilter polarity 分歧检测 → CONFIG 层方向修正。
  两版并行输出，在 DualResonanceSignals 中同时可用。
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Callable, Literal

from newchan.backtest import BacktestResult, Trade
from newchan.cost.config import CostConfig
from newchan.nesting.bsp import BSP, BSPType, is_negated
from newchan.nesting.resonance import ResonanceSignal, SignalLayer
from newchan.pipeline import (
    TradingContext,
    compute_position,
    create_context,
    execute_entry,
    execute_exit,
    locate_bsp,
    scan_configuration,
)
from newchan.topology.config_space import Configuration, polarity_index
from newchan.topology.fiber_pipeline_adapter import (
    FiberSignalFilter,
    create_fiber_context,
)
from newchan.trading.layer_state import LayerState, LayerType


@dataclass(frozen=True, slots=True)
class PipelineBacktestConfig:
    """策略级回测配置。

    Attributes
    ----------
    config : Configuration
        K4 配置（固定）。
    time_tolerance_fn : Callable[[int, int], float] | None
        共振时间容差函数。None = 使用默认。
    cost_config : CostConfig | None
        成本配置。None = 不计成本。
    allow_short : bool
        是否允许做空。
    fiber_filter : FiberSignalFilter | None
        纤维丛信号过滤器。None = 使用默认。
    """

    config: Configuration
    time_tolerance_fn: Callable[[int, int], float] | None = None
    cost_config: CostConfig | None = None
    allow_short: bool = False
    fiber_filter: FiberSignalFilter | None = None


def _default_time_tolerance(level_i: int, level_j: int) -> float:
    """默认时间容差。"""
    return float(abs(level_i - level_j) + 1) * 100.0


def _extract_new_bsps(
    snapshot,
    seen_keys: set[tuple[int, str, str, int]],
) -> list[BSP]:
    """从 snapshot 提取新确认的 BSP，转换为 nesting.bsp.BSP 类型。

    返回新 BSP 列表（同时更新 seen_keys）。
    """
    result: list[BSP] = []
    for bp in snapshot.bsp_snapshot.buysellpoints:
        if not bp.confirmed:
            continue
        key = (bp.seg_idx, bp.kind, bp.side, bp.level_id)
        if key in seen_keys:
            continue
        seen_keys.add(key)

        if bp.side == "buy":
            bsp_type = _kind_to_buy_type(bp.kind)
        else:
            bsp_type = _kind_to_sell_type(bp.kind)

        result.append(BSP(
            edge_id=f"bt_{bp.seg_idx}",
            level=bp.level_id,
            time=float(bp.bar_idx),
            bsp_type=bsp_type,
            price=bp.price,
        ))
    return result


def _kind_to_buy_type(kind: str) -> BSPType:
    """BuySellPoint.kind -> BSPType (buy side)。"""
    mapping = {"type1": BSPType.B1, "type2": BSPType.B2, "type3": BSPType.B3}
    return mapping.get(kind, BSPType.B1)


def _kind_to_sell_type(kind: str) -> BSPType:
    """BuySellPoint.kind -> BSPType (sell side)。"""
    mapping = {"type1": BSPType.S1, "type2": BSPType.S2, "type3": BSPType.S3}
    return mapping.get(kind, BSPType.S1)


@dataclass(frozen=True, slots=True)
class DualResonanceSignals:
    """双模型共振信号（直积 + 纤维丛并行输出）。

    241号谱系：两套组装方式各生成一份共振信号。
    243号谱系：fiber_scan_direction 使纤维丛管道 scan 层与共振层使用同一 polarity 来源。

    Attributes
    ----------
    product_signals : tuple[ResonanceSignal, ...]
        直积版共振信号（ConfigurationSpace polarity 决定 CONFIG 层方向）。
    fiber_signals : tuple[ResonanceSignal, ...]
        纤维丛版共振信号（FiberSignalFilter polarity 决定 CONFIG 层方向）。
    product_polarity : int
        直积假设下的 polarity_index。
    fiber_polarity : int
        纤维丛联络下的 polarity_index。
    polarity_divergence : bool
        两种 polarity 是否不同。
    fiber_scan_direction : str
        纤维丛版的 scan direction（由 fiber_polarity 决定）。
        "buy" / "sell" / "neutral"。与 fiber_signals 的 CONFIG 层方向来源一致。
    """

    product_signals: tuple[ResonanceSignal, ...]
    fiber_signals: tuple[ResonanceSignal, ...]
    product_polarity: int
    fiber_polarity: int
    polarity_divergence: bool
    fiber_scan_direction: str


def _polarity_to_scan_direction(polarity: int) -> str:
    """polarity -> scan direction 字符串。

    与 pipeline.scan_configuration 的方向逻辑一致：
    S > 0 -> "buy", S < 0 -> "sell", S = 0 -> "neutral"。

    243号谱系：提取为独立函数，使纤维丛版可以从 fiber_polarity 推导 scan direction。
    """
    if polarity > 0:
        return "buy"
    if polarity < 0:
        return "sell"
    return "neutral"


def _polarity_to_bsp(polarity: int, bsp: BSP) -> BSP:
    """根据 polarity 方向生成 CONFIG 层的方向 BSP。

    polarity > 0 → buy 方向（使用 B1 类型）
    polarity < 0 → sell 方向（使用 S1 类型）
    polarity = 0 → 使用 BSP 自身方向（neutral 时 CONFIG 层不提供独立方向信息）

    返回新 BSP，保留原始 BSP 的 edge_id/level/time/price，仅修改 bsp_type。
    """
    if polarity > 0:
        config_type = BSPType.B1
    elif polarity < 0:
        config_type = BSPType.S1
    else:
        config_type = bsp.bsp_type
    return BSP(
        edge_id=f"{bsp.edge_id}_config",
        level=bsp.level,
        time=bsp.time,
        bsp_type=config_type,
        price=bsp.price,
    )


def _build_signals_for_polarity(
    polarity: int,
    bsp: BSP,
) -> tuple[ResonanceSignal, ...]:
    """从 polarity 和 BSP 构建一组共振信号。

    CONFIG 层：polarity 方向（配置层全局方向确认）。
    INDEPENDENT_EDGE 层：BSP 自身方向（独立边级别方向确认）。

    共振条件：CONFIG 层方向与 INDEPENDENT_EDGE 层方向一致时，
    信号通过 resonance_check。不一致时 resonance_check 返回 False——
    这是真实的共振筛选：配置不支持的方向不入场。
    """
    config_bsp = _polarity_to_bsp(polarity, bsp)
    return (
        ResonanceSignal(
            edge_id=config_bsp.edge_id,
            level=bsp.level,
            bsp=config_bsp,
            layer=SignalLayer.CONFIG,
            time=bsp.time,
        ),
        ResonanceSignal(
            edge_id=bsp.edge_id,
            level=bsp.level,
            bsp=bsp,
            layer=SignalLayer.INDEPENDENT_EDGE,
            time=bsp.time,
        ),
    )


def _build_resonance_signals(
    bsp: BSP,
    ctx: TradingContext,
    fiber_filter: FiberSignalFilter | None = None,
) -> DualResonanceSignals:
    """从 BSP + TradingContext 构建双模型共振信号。

    241号谱系：替换双层简化，使用真实共振构建。

    直积版：polarity_index(config) 决定 CONFIG 层方向。
    纤维丛版：FiberSignalFilter 检测 polarity 分歧，
    polarity_divergence=True 且 KL > threshold 时使用 fiber_polarity。

    Parameters
    ----------
    bsp : BSP
        当前步确认的买卖点。
    ctx : TradingContext
        当前交易上下文（携带 Configuration）。
    fiber_filter : FiberSignalFilter | None
        纤维丛信号过滤器。None 时使用默认。

    Returns
    -------
    DualResonanceSignals
        双模型共振信号。
    """
    # 直积版：ConfigurationSpace polarity
    product_pol = polarity_index(ctx.config)
    product_signals = _build_signals_for_polarity(product_pol, bsp)

    # 纤维丛版：FiberTradingContext polarity
    ff = fiber_filter if fiber_filter is not None else FiberSignalFilter()
    fiber_ctx = create_fiber_context(ctx)
    if ff.should_override_polarity(fiber_ctx):
        fiber_pol = fiber_ctx.fiber_polarity
    else:
        fiber_pol = product_pol
    fiber_signals = _build_signals_for_polarity(fiber_pol, bsp)

    # 243号：fiber_scan_direction 由 fiber_pol 决定（与 fiber_signals CONFIG 层同源）
    fiber_scan_dir = _polarity_to_scan_direction(fiber_pol)

    return DualResonanceSignals(
        product_signals=product_signals,
        fiber_signals=fiber_signals,
        product_polarity=product_pol,
        fiber_polarity=fiber_ctx.fiber_polarity,
        polarity_divergence=fiber_ctx.polarity_divergence,
        fiber_scan_direction=fiber_scan_dir,
    )


@dataclass
class _OpenTrade:
    """当前持仓（内部可变跟踪）。"""

    side: Literal["long", "short"]
    entry_bar: int
    entry_price: float
    bsp: BSP
    entry_slippage: float = 0.0
    entry_commission: float = 0.0


class PipelineBacktestEngine:
    """策略级回测引擎 -- 基于 TradingContext 的完整交易管道。

    用法::

        engine = PipelineBacktestEngine(config)
        for bar in bars:
            snap = orchestrator.process_bar(bar)
            engine.process_snapshot(snap, bar)
        result = engine.result()
    """

    def __init__(self, config: PipelineBacktestConfig) -> None:
        self._config = config
        self._cost_config = config.cost_config or CostConfig()
        self._ctx: TradingContext = create_context(config.config)
        self._time_tolerance_fn = config.time_tolerance_fn or _default_time_tolerance
        self._fiber_filter = config.fiber_filter
        self._trades: list[Trade] = []
        self._open_trade: _OpenTrade | None = None
        self._seen_bsp_keys: set[tuple[int, str, str, int]] = set()
        self._bar_count = 0
        self._last_dual_signals: DualResonanceSignals | None = None

    @property
    def has_open_position(self) -> bool:
        """是否有未平仓头寸。"""
        return self._open_trade is not None

    @property
    def ctx(self) -> TradingContext:
        """当前 TradingContext（只读访问）。"""
        return self._ctx

    @property
    def last_dual_signals(self) -> DualResonanceSignals | None:
        """最近一次共振信号构建的双模型结果（只读访问）。"""
        return self._last_dual_signals

    def process_snapshot(self, snapshot, bar) -> None:
        """处理一个 RecursiveOrchestratorSnapshot + 对应 Bar。

        每 bar 推进 pipeline，根据 TradingContext 状态决策。

        流程：
        1. 从 snapshot 提取新确认的 BSP
        2. 如果持仓且 BSP 被否定 -> 退出
        3. 如果持仓且出现反向 BSP -> 退出
        4. 如果无持仓，有买方向 BSP 且极性允许 -> 通过 pipeline 入场
        """
        self._bar_count += 1
        bar_idx = snapshot.bar_idx
        price = bar.close

        new_bsps = _extract_new_bsps(snapshot, self._seen_bsp_keys)
        buy_bsps = [b for b in new_bsps if b.bsp_type.is_buy]
        sell_bsps = [b for b in new_bsps if b.bsp_type.is_sell]

        # 1. 否定检查：如果持仓的入场 BSP 被否定 -> 止损退出
        if self._open_trade is not None:
            if is_negated(self._open_trade.bsp, price):
                self._close_position(bar_idx, price, "bsp_negated")

        # 2. 反向 BSP 平仓
        if self._open_trade is not None:
            if self._open_trade.side == "long" and sell_bsps:
                self._close_position(bar_idx, price, "reverse_bsp")
            elif self._open_trade.side == "short" and buy_bsps:
                self._close_position(bar_idx, price, "reverse_bsp")

        # 3. 开仓：通过 pipeline 逻辑
        if self._open_trade is None:
            self._try_entry(buy_bsps, sell_bsps, bar_idx, price)

    def _try_entry(
        self,
        buy_bsps: list[BSP],
        sell_bsps: list[BSP],
        bar_idx: int,
        price: float,
    ) -> None:
        """尝试通过 pipeline 入场。"""
        # 配置扫描确定方向
        self._ctx, directions = scan_configuration(self._ctx)

        if "buy" in directions and buy_bsps:
            bsp = buy_bsps[0]
            self._pipeline_enter(bsp, "long", bar_idx, price)
        elif "sell" in directions and sell_bsps and self._config.allow_short:
            bsp = sell_bsps[0]
            self._pipeline_enter(bsp, "short", bar_idx, price)

    def _pipeline_enter(
        self,
        bsp: BSP,
        side: Literal["long", "short"],
        bar_idx: int,
        price: float,
    ) -> None:
        """通过 pipeline 组件推进并入场。

        流程：
        1. locate_bsp: 推进横向区间套
        2. _build_resonance_signals: 双模型共振信号构建
        3. compute_position: 使用直积版信号计算共振仓位
        4. execute_entry: 仓位 > 0 时入场

        241号谱系：共振信号从双层简化升级为真实构建。
        直积版信号用于入场决策（与现有管道语义一致）。
        纤维丛版信号通过 last_dual_signals 暴露给调用方做对比分析。
        """
        # 1. 推进区间套
        self._ctx = locate_bsp(self._ctx, bsp)

        # 2. 双模型共振信号构建
        dual = _build_resonance_signals(bsp, self._ctx, self._fiber_filter)
        self._last_dual_signals = dual

        # 3. 使用直积版信号计算共振仓位
        resonance_signals = list(dual.product_signals)
        position, _level = compute_position(
            self._ctx, resonance_signals, self._time_tolerance_fn,
        )

        if position <= 0:
            return

        # 3. 入场
        l0_state = self._ctx.state_machine.get_state(LayerType.L0_CONFIG)
        if l0_state is not LayerState.ACTIVE:
            bsp_label = f"{bsp.bsp_type.value}@{bsp.edge_id}"
            self._ctx, _result = execute_entry(
                self._ctx, LayerType.L0_CONFIG, bsp_label, position,
            )

            entry_price = self._apply_slippage(price, "buy" if side == "long" else "sell")
            commission = self._calc_commission(entry_price)
            self._open_trade = _OpenTrade(
                side=side,
                entry_bar=bar_idx,
                entry_price=entry_price,
                bsp=bsp,
                entry_slippage=entry_price - price,
                entry_commission=commission,
            )

    def _close_position(
        self,
        bar_idx: int,
        price: float,
        reason: Literal["reverse_bsp", "bsp_negated"],
    ) -> None:
        """平仓并记录交易。"""
        pos = self._open_trade
        if pos is None:
            return

        exit_side = "sell" if pos.side == "long" else "buy"
        exit_price = self._apply_slippage(price, exit_side)
        exit_comm = self._calc_commission(exit_price)

        # 通过 pipeline 退出状态机
        abort = reason == "bsp_negated"
        l0_state = self._ctx.state_machine.get_state(LayerType.L0_CONFIG)
        if l0_state is LayerState.ACTIVE:
            self._ctx, _ = execute_exit(self._ctx, LayerType.L0_CONFIG, abort=abort)

        self._trades.append(Trade(
            side=pos.side,
            bsp_kind=pos.bsp.bsp_type.value,
            bsp_level=pos.bsp.level,
            entry_bar=pos.entry_bar,
            exit_bar=bar_idx,
            entry_price=pos.entry_price,
            exit_price=exit_price,
            exit_reason=reason,
            entry_slippage=pos.entry_slippage,
            exit_slippage=exit_price - price,
            entry_commission=pos.entry_commission,
            exit_commission=exit_comm,
        ))
        self._open_trade = None

    def result(self) -> BacktestResult:
        """返回回测结果。未平仓头寸不计入统计。"""
        return BacktestResult(
            trades=tuple(self._trades),
            total_bars=self._bar_count,
        )

    def _apply_slippage(self, price: float, side: str) -> float:
        sm = self._cost_config.slippage_model
        if sm is None:
            return price
        return sm.apply(price, side)

    def _calc_commission(self, price: float) -> float:
        cm = self._cost_config.commission_model
        if cm is None:
            return 0.0
        return cm.calculate(price, self._cost_config.quantity)
