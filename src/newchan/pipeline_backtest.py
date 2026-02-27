"""策略级回测引擎 — pipeline + backtest 集成。

与 BacktestEngine 的区别：
- BacktestEngine: 见 BSP 就入场（信号级）
- PipelineBacktestEngine: 配置扫描 -> 区间套 -> 共振 -> 状态机入场（策略级）

概念溯源：
  [操盘方法 v2] §一 完整交易管道
  回测时每 bar 推进 TradingContext，根据状态机状态决策入退场。
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
from newchan.topology.config_space import Configuration
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
    """

    config: Configuration
    time_tolerance_fn: Callable[[int, int], float] | None = None
    cost_config: CostConfig | None = None
    allow_short: bool = False


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


def _build_resonance_signals(bsp: BSP) -> list[ResonanceSignal]:
    """从单个 BSP 构建最小共振信号集。

    策略级回测的简化：用单 BSP 构建 CONFIG + INDEPENDENT_EDGE 双层信号，
    使 resonance_check 能通过（需要 >= 2 个同方向信号）。
    """
    return [
        ResonanceSignal(
            edge_id=bsp.edge_id,
            level=bsp.level,
            bsp=bsp,
            layer=SignalLayer.CONFIG,
            time=bsp.time,
        ),
        ResonanceSignal(
            edge_id=f"{bsp.edge_id}_edge",
            level=bsp.level,
            bsp=bsp,
            layer=SignalLayer.INDEPENDENT_EDGE,
            time=bsp.time,
        ),
    ]


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
        self._trades: list[Trade] = []
        self._open_trade: _OpenTrade | None = None
        self._seen_bsp_keys: set[tuple[int, str, str, int]] = set()
        self._bar_count = 0

    @property
    def has_open_position(self) -> bool:
        """是否有未平仓头寸。"""
        return self._open_trade is not None

    @property
    def ctx(self) -> TradingContext:
        """当前 TradingContext（只读访问）。"""
        return self._ctx

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
        2. compute_position: 计算共振仓位
        3. execute_entry: 仓位 > 0 时入场

        横向区间套六步在回测中逐步推进（每个新 BSP 推进一步），
        但入场决策由配置极性 + 共振仓位决定，不以六步全部完成为前提。
        """
        # 1. 推进区间套
        self._ctx = locate_bsp(self._ctx, bsp)

        # 2. 计算共振仓位
        resonance_signals = _build_resonance_signals(bsp)
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
