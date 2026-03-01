"""全流程编排 — K4 配置 → 选股扫描 → 状态机 → 成本跟踪。

逐 bar 同步推进，所有判断基于当前已确认结构（不用未来数据）。

复用 backtest.full_pipeline.FullPipelineEngine 的逐 bar 架构，
在其基础上附加 D 算子读数和降成本曲线追踪。

认识论标注：
  - 编排逻辑：L0（从架构定义推导）
  - 信号提取规则：L2（需回测验证）

谱系引用：267号操作方法论 v1。
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime

from newchan.backtest.k4_config import read_k4_config
from newchan.backtest.scanner import scan_stocks
from newchan.backtest.state_machine import CostReductionStateMachine, StateMachineEvent
from newchan.backtest.types import (
    CostCurvePoint,
    Direction,
    K4State,
    ScannerResult,
    StateMachineState,
    TradeAction,
)
from newchan.orchestrator.recursive import RecursiveOrchestrator, RecursiveOrchestratorSnapshot
from newchan.types import Bar


# ═══════════════════════════════════════════════════════════════
# 配置
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class BacktestOrchestratorConfig:
    """回测编排器配置。

    Attributes
    ----------
    initial_capital : float
        初始自有资金。
    margin_ratio : float
        融资比例（1:1 = 1.0）。
    short_diff_ratio : float
        短差比例 n%。
    k4_symbols : tuple[str, str, str]
        K4 三标的。
    candidate_symbols : tuple[str, ...]
        候选标的列表。
    """

    initial_capital: float = 1_000_000.0
    margin_ratio: float = 1.0
    short_diff_ratio: float = 0.1
    k4_symbols: tuple[str, str, str] = ("SPY", "GLD", "TLT")
    candidate_symbols: tuple[str, ...] = (
        "XLK", "XLF", "XLE", "XLV", "XLY",
        "XLI", "XLB", "XLP", "XLU", "XLRE", "XLC",
    )

    @property
    def max_capital(self) -> float:
        """最大可用资金 = 自有 + 融资。"""
        return self.initial_capital * (1 + self.margin_ratio)


# ═══════════════════════════════════════════════════════════════
# 步进记录
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class OrchestratorStep:
    """单 bar 编排结果。

    Attributes
    ----------
    bar_idx : int
        bar 索引。
    bar_ts : datetime
        bar 时间戳。
    k4_state : K4State
        K4 配置状态。
    scanner_result : ScannerResult
        选股扫描结果。
    sm_state : StateMachineState
        状态机状态。
    cost_basis : float
        当前持仓成本。
    cumulative_recovered : float
        累计回收金额。
    held_symbol : str | None
        当前持仓标的。
    action : TradeAction | None
        本 bar 执行的操作。
    """

    bar_idx: int
    bar_ts: datetime
    k4_state: K4State
    scanner_result: ScannerResult
    sm_state: StateMachineState
    cost_basis: float
    cumulative_recovered: float
    held_symbol: str | None
    action: TradeAction | None


# ═══════════════════════════════════════════════════════════════
# 回测结果
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class BacktestOrchestratorResult:
    """全流程回测结果。

    Attributes
    ----------
    config : BacktestOrchestratorConfig
        配置。
    steps : tuple[OrchestratorStep, ...]
        所有步进记录。
    actions : tuple[TradeAction, ...]
        所有操作记录。
    cost_curve : tuple[CostCurvePoint, ...]
        降成本曲线。
    k4_changes : tuple[tuple[int, str], ...]
        K4 配置变化日志：(bar_idx, new_label)。
    stock_changes : tuple[tuple[int, str | None], ...]
        选股日志：(bar_idx, symbol)。
    """

    config: BacktestOrchestratorConfig
    steps: tuple[OrchestratorStep, ...]
    actions: tuple[TradeAction, ...]
    cost_curve: tuple[CostCurvePoint, ...]
    k4_changes: tuple[tuple[int, str], ...]
    stock_changes: tuple[tuple[int, str | None], ...]


# ═══════════════════════════════════════════════════════════════
# 编排器
# ═══════════════════════════════════════════════════════════════


class BacktestOrchestrator:
    """全流程回测编排器。

    用法::

        orch = BacktestOrchestrator(config)
        for bar_idx in range(num_bars):
            orch.process_bar(
                k4_bars=(spy_bars[bar_idx], gld_bars[bar_idx], tlt_bars[bar_idx]),
                candidate_bars={sym: bars[bar_idx] for sym, bars in ...},
            )
        result = orch.result()
    """

    def __init__(self, config: BacktestOrchestratorConfig) -> None:
        self._config = config
        self._bar_idx = 0

        # K4 三条比价线的 orchestrator
        self._k4_orchestrators = (
            RecursiveOrchestrator(stream_id=config.k4_symbols[0]),
            RecursiveOrchestrator(stream_id=config.k4_symbols[1]),
            RecursiveOrchestrator(stream_id=config.k4_symbols[2]),
        )

        # 候选标的 orchestrators（按需创建）
        self._candidate_orchestrators: dict[str, RecursiveOrchestrator] = {}

        # 状态机
        self._sm = CostReductionStateMachine.create(
            initial_capital=config.initial_capital,
            margin_ratio=config.margin_ratio,
            short_diff_ratio=config.short_diff_ratio,
        )

        # 持仓标的
        self._held_symbol: str | None = None

        # BSP 去重
        self._seen_bsp_keys: set[tuple[int, str, str, int]] = set()

        # 步进记录
        self._steps: list[OrchestratorStep] = []

        # 日志
        self._k4_changes: list[tuple[int, str]] = []
        self._stock_changes: list[tuple[int, str | None]] = []
        self._prev_k4_label: str = ""

    def _get_or_create_orchestrator(self, symbol: str) -> RecursiveOrchestrator:
        """获取或创建候选标的的 orchestrator。"""
        if symbol not in self._candidate_orchestrators:
            self._candidate_orchestrators[symbol] = RecursiveOrchestrator(
                stream_id=symbol,
            )
        return self._candidate_orchestrators[symbol]

    def process_bar(
        self,
        k4_bars: tuple[Bar, Bar, Bar],
        candidate_bars: dict[str, Bar],
    ) -> OrchestratorStep:
        """处理单 bar。

        Parameters
        ----------
        k4_bars : tuple[Bar, Bar, Bar]
            K4 三标的的当前 bar (SPY, GLD, TLT)。
        candidate_bars : dict[str, Bar]
            候选标的的当前 bar。

        Returns
        -------
        OrchestratorStep
            本 bar 的编排结果。
        """
        bar_ts = k4_bars[0].ts

        # 1. 推进 K4 三条比价线
        k4_snapshots = tuple(
            orch.process_bar(bar)
            for orch, bar in zip(self._k4_orchestrators, k4_bars)
        )

        # 2. 读取 K4 配置（含 D 算子读数）
        k4_state = read_k4_config(
            k4_snapshots[0], k4_snapshots[1], k4_snapshots[2],
            e_symbol=self._config.k4_symbols[0],
            au_symbol=self._config.k4_symbols[1],
            r_symbol=self._config.k4_symbols[2],
        )

        # K4 变化日志
        k4_label = k4_state.config.label
        if k4_label != self._prev_k4_label:
            self._k4_changes.append((self._bar_idx, k4_label))
            self._prev_k4_label = k4_label

        # 3. 推进所有候选标的
        candidate_snapshots: dict[str, RecursiveOrchestratorSnapshot] = {}
        for sym, bar in candidate_bars.items():
            orch = self._get_or_create_orchestrator(sym)
            candidate_snapshots[sym] = orch.process_bar(bar)

        # 4. 选股扫描
        scanner_result = scan_stocks(k4_state, candidate_snapshots)

        # 5. 状态机驱动
        action: TradeAction | None = None

        if self._sm.state == StateMachineState.EMPTY:
            # 空仓：检查是否有选股命中
            action = self._handle_empty(
                scanner_result, candidate_snapshots, candidate_bars,
            )
        elif self._sm.state in (
            StateMachineState.BASE,
            StateMachineState.SHORT_TRADE,
            StateMachineState.ADDED,
        ):
            # 持仓：检查次级别买卖点
            action = self._handle_position(candidate_snapshots, candidate_bars)
        elif self._sm.state == StateMachineState.FULL:
            # 满仓：检查主级别卖点
            action = self._handle_full(candidate_snapshots, candidate_bars)

        # 6. 记录步进
        step = OrchestratorStep(
            bar_idx=self._bar_idx,
            bar_ts=bar_ts,
            k4_state=k4_state,
            scanner_result=scanner_result,
            sm_state=self._sm.state,
            cost_basis=self._sm.cost_basis,
            cumulative_recovered=self._sm.cumulative_recovered,
            held_symbol=self._held_symbol,
            action=action,
        )
        self._steps.append(step)
        self._bar_idx += 1
        return step

    def _handle_empty(
        self,
        scanner_result: ScannerResult,
        candidate_snapshots: dict[str, RecursiveOrchestratorSnapshot],
        candidate_bars: dict[str, Bar],
    ) -> TradeAction | None:
        """空仓：检查选股命中 + 买点确认。"""
        if scanner_result.selected_symbol is None:
            return None

        symbol = scanner_result.selected_symbol
        snap = candidate_snapshots.get(symbol)
        if snap is None:
            return None

        # 检查是否有已确认买点
        for bp in snap.bsp_snapshot.buysellpoints:
            if not bp.confirmed or bp.side != "buy":
                continue
            key = (bp.seg_idx, bp.kind, bp.side, bp.level_id)
            if key in self._seen_bsp_keys:
                continue
            self._seen_bsp_keys.add(key)

            # 建仓
            bar = candidate_bars.get(symbol)
            if bar is None:
                continue

            price = bar.close
            self._sm = self._sm.process_event(
                StateMachineEvent.BUY_SIGNAL,
                price=price,
                bar_idx=self._bar_idx,
                trigger=f"buy_{bp.kind}_L{bp.level_id}",
            )
            self._held_symbol = symbol
            self._stock_changes.append((self._bar_idx, symbol))

            return self._sm.actions[-1] if self._sm.actions else None

        return None

    def _handle_position(
        self,
        candidate_snapshots: dict[str, RecursiveOrchestratorSnapshot],
        candidate_bars: dict[str, Bar],
    ) -> TradeAction | None:
        """持仓：检查次级别买卖点 → 短差操作。"""
        if self._held_symbol is None:
            return None

        snap = candidate_snapshots.get(self._held_symbol)
        bar = candidate_bars.get(self._held_symbol)
        if snap is None or bar is None:
            return None

        price = bar.close

        # 检查买点失效（价格跌破入场价 10%）
        if (self._sm.fsm.entry_price > 0
                and price < self._sm.fsm.entry_price * 0.9):
            self._sm = self._sm.process_event(
                StateMachineEvent.STOP_LOSS,
                price=price,
                bar_idx=self._bar_idx,
                trigger="price_below_entry_10pct",
            )
            old_symbol = self._held_symbol
            self._held_symbol = None
            self._stock_changes.append((self._bar_idx, None))
            self._seen_bsp_keys.clear()
            # STOPPED_OUT → RESET
            self._sm = CostReductionStateMachine.create(
                initial_capital=max(1.0, self._sm.fsm.own_capital),
                margin_ratio=self._config.margin_ratio,
                short_diff_ratio=self._config.short_diff_ratio,
            )
            return self._sm.actions[-1] if self._sm.actions else None

        # 检查次级别卖点 → 短差卖出
        for bp in snap.bsp_snapshot.buysellpoints:
            if not bp.confirmed:
                continue
            key = (bp.seg_idx, bp.kind, bp.side, bp.level_id)
            if key in self._seen_bsp_keys:
                continue
            self._seen_bsp_keys.add(key)

            if bp.side == "sell":
                event = StateMachineEvent.SHORT_ENTRY
                trigger = f"sub_sell_{bp.kind}_L{bp.level_id}"
            elif bp.side == "buy":
                event = StateMachineEvent.SHORT_EXIT
                trigger = f"sub_buy_{bp.kind}_L{bp.level_id}"
            else:
                continue

            try:
                self._sm = self._sm.process_event(
                    event,
                    price=price,
                    bar_idx=self._bar_idx,
                    trigger=trigger,
                )
                return self._sm.actions[-1] if self._sm.actions else None
            except Exception:
                continue

        return None

    def _handle_full(
        self,
        candidate_snapshots: dict[str, RecursiveOrchestratorSnapshot],
        candidate_bars: dict[str, Bar],
    ) -> TradeAction | None:
        """满仓（本金已退出）：检查主级别卖点 → 退出。"""
        if self._held_symbol is None:
            return None

        snap = candidate_snapshots.get(self._held_symbol)
        bar = candidate_bars.get(self._held_symbol)
        if snap is None or bar is None:
            return None

        price = bar.close

        for bp in snap.bsp_snapshot.buysellpoints:
            if not bp.confirmed or bp.side != "sell":
                continue
            key = (bp.seg_idx, bp.kind, bp.side, bp.level_id)
            if key in self._seen_bsp_keys:
                continue
            self._seen_bsp_keys.add(key)

            self._sm = self._sm.process_event(
                StateMachineEvent.SELL_SIGNAL,
                price=price,
                bar_idx=self._bar_idx,
                trigger=f"main_sell_{bp.kind}_L{bp.level_id}",
            )
            self._held_symbol = None
            self._stock_changes.append((self._bar_idx, None))
            self._seen_bsp_keys.clear()
            # STOPPED_OUT → RESET
            self._sm = CostReductionStateMachine.create(
                initial_capital=max(1.0, self._sm.fsm.own_capital),
                margin_ratio=self._config.margin_ratio,
                short_diff_ratio=self._config.short_diff_ratio,
            )
            return self._sm.actions[-1] if self._sm.actions else None

        return None

    def result(self) -> BacktestOrchestratorResult:
        """返回回测结果。"""
        return BacktestOrchestratorResult(
            config=self._config,
            steps=tuple(self._steps),
            actions=self._sm.actions,
            cost_curve=self._sm.cost_curve,
            k4_changes=tuple(self._k4_changes),
            stock_changes=tuple(self._stock_changes),
        )


# ═══════════════════════════════════════════════════════════════
# 便捷入口
# ═══════════════════════════════════════════════════════════════


def run_backtest(
    config: BacktestOrchestratorConfig,
    k4_bar_streams: tuple[list[Bar], list[Bar], list[Bar]],
    candidate_bar_streams: dict[str, list[Bar]],
) -> BacktestOrchestratorResult:
    """给定全部 bar 数据，运行完整回测。

    Parameters
    ----------
    config : BacktestOrchestratorConfig
        回测配置。
    k4_bar_streams : tuple[list[Bar], list[Bar], list[Bar]]
        K4 三标的 bar 流 (SPY, GLD, TLT)，同步对齐。
    candidate_bar_streams : dict[str, list[Bar]]
        候选标的 bar 流。

    Returns
    -------
    BacktestOrchestratorResult
        回测结果。
    """
    orch = BacktestOrchestrator(config)

    k4_len = min(len(s) for s in k4_bar_streams) if k4_bar_streams[0] else 0
    if k4_len == 0:
        return orch.result()

    for i in range(k4_len):
        k4_bars = (
            k4_bar_streams[0][i],
            k4_bar_streams[1][i],
            k4_bar_streams[2][i],
        )
        candidate_bars: dict[str, Bar] = {}
        for sym, bars in candidate_bar_streams.items():
            if i < len(bars):
                candidate_bars[sym] = bars[i]
        orch.process_bar(k4_bars, candidate_bars)

    return orch.result()
