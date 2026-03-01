"""全流程回测引擎 — K4 配置 → 选股扫描 → 降成本状态机。

认识论标注：
  - 引擎框架：L0（从架构定义直接推导）
  - 信号提取规则（何时产生 SUB_LEVEL_SELL_POINT 等）：L2（需回测验证）

谱系引用：267号操作方法论 v1、268a号结算修正。

bar-by-bar 推进逻辑：
  1. 推进 K4 三条比价线的 RecursiveOrchestrator
  2. 调用 K4 Scanner → Configuration + polarity
  3. SCANNING 阶段：推进候选标的 → Stock Scanner → 选股命中则建仓
  4. POSITION_OPEN / COST_REDUCING：推进持仓标的 → 检测次级别买卖点
  5. PRINCIPAL_WITHDRAWN：检测主级别卖点
  6. STOPPED_OUT：自动 RESET
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime

from newchan.orchestrator.recursive import RecursiveOrchestrator, RecursiveOrchestratorSnapshot
from newchan.trading.cost_reduction_fsm import (
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    transition,
)
from newchan.types import Bar

# ── 依赖模块的条件导入（K4Scanner / StockScanner 可能尚未就绪） ──

try:
    from newchan.topology.k4_scanner import K4ScanResult, scan_k4  # type: ignore[import-not-found]
    _HAS_K4_SCANNER = True
except ImportError:
    _HAS_K4_SCANNER = False

try:
    from newchan.trading.stock_scanner import ScanResult, scan_candidates  # type: ignore[import-not-found]
    _HAS_STOCK_SCANNER = True
except ImportError:
    _HAS_STOCK_SCANNER = False

from newchan.topology.config_space import Configuration, WalkDirection, polarity_index


# ═══════════════════════════════════════════════════════════════
# 数据结构
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class FullPipelineConfig:
    """全流程回测配置。"""

    initial_capital: float = 1_000_000.0
    margin_ratio: float = 1.0
    short_diff_ratio: float = 0.1
    min_operable_level: str = "5min"
    k4_symbols: tuple[str, str, str] = ("SPY", "GLD", "TLT")

    @property
    def margin_amount(self) -> float:
        return self.initial_capital * self.margin_ratio


@dataclass(frozen=True, slots=True)
class BarStep:
    """单 bar 步进记录。"""

    bar_idx: int
    bar_ts: float
    k4_config_label: str
    k4_polarity: int
    scanner_selected: str | None
    fsm_state: str
    cost_basis: float
    cumulative_recovered: float
    total_shares: float
    missed_signals_count: int = 0


@dataclass
class FullPipelineResult:
    """完整回测结果。"""

    config: FullPipelineConfig
    steps: list[BarStep]
    fsm_transitions: list[tuple[int, str, str]]
    final_fsm: CostReductionFSM
    total_bars: int


# ═══════════════════════════════════════════════════════════════
# 引擎
# ═══════════════════════════════════════════════════════════════


class FullPipelineEngine:
    """全流程回测引擎。

    驱动逻辑（每 bar）：
    1. 推进 K4 三条比价线的 RecursiveOrchestrator
    2. 调用 K4 Scanner → Configuration
    3. 如果 FSM 在 SCANNING：
       a. 推进所有候选标的的 RecursiveOrchestrator
       b. 调用 Stock Scanner → ScanResult
       c. 如果 selected 非空 → 生成 BUY_POINT_CONFIRMED 事件 → transition
    4. 如果 FSM 在 POSITION_OPEN/COST_REDUCING：
       a. 推进当前持仓标的的 RecursiveOrchestrator
       b. 从 bsp_snapshot 检测次级别买卖点 → 生成相应 FSM 事件
       c. 检测买点失效 → BUY_POINT_NEGATED
    5. 如果 FSM 在 PRINCIPAL_WITHDRAWN：
       a. 检测主级别卖点 → MAIN_LEVEL_SELL_POINT
    6. 如果 FSM 在 STOPPED_OUT：
       a. 自动 RESET（own_capital = 实际剩余资金）
    7. 记录 BarStep
    """

    def __init__(self, config: FullPipelineConfig) -> None:
        self._config = config
        self._bar_idx = 0
        self._steps: list[BarStep] = []
        self._fsm_transitions: list[tuple[int, str, str]] = []

        # FSM 初始化
        self._fsm = CostReductionFSM.create(
            own_capital=config.initial_capital,
            margin_amount=config.margin_amount,
            sub_ratio=config.short_diff_ratio,
            min_operable_level=config.min_operable_level,
        )

        # K4 三条比价线的 orchestrator
        self._k4_orchestrators: tuple[RecursiveOrchestrator, RecursiveOrchestrator, RecursiveOrchestrator] = (
            RecursiveOrchestrator(stream_id=config.k4_symbols[0]),
            RecursiveOrchestrator(stream_id=config.k4_symbols[1]),
            RecursiveOrchestrator(stream_id=config.k4_symbols[2]),
        )

        # 候选标的 orchestrators（按需创建）
        self._candidate_orchestrators: dict[str, RecursiveOrchestrator] = {}

        # 当前持仓标的
        self._held_symbol: str | None = None
        self._held_entry_price: float = 0.0

        # BSP 跟踪（去重）
        self._seen_bsp_keys: set[tuple[int, str, str, int]] = set()

        # 机会成本跟踪（不换仓原则，267号§五）
        self._missed_bsp_keys: set[tuple[str, int, str, str, int]] = set()
        self._current_bar_missed: int = 0

    def _get_or_create_orchestrator(self, symbol: str) -> RecursiveOrchestrator:
        """获取或创建候选标的的 orchestrator。"""
        if symbol not in self._candidate_orchestrators:
            self._candidate_orchestrators[symbol] = RecursiveOrchestrator(stream_id=symbol)
        return self._candidate_orchestrators[symbol]

    def _transition_fsm(self, event: FsmEvent) -> None:
        """执行 FSM 转移并记录。"""
        old_state = self._fsm.state.name
        self._fsm = transition(self._fsm, event)
        new_state = self._fsm.state.name
        if old_state != new_state:
            self._fsm_transitions.append((self._bar_idx, old_state, new_state))

    def _scan_k4(
        self,
        snapshots: tuple[RecursiveOrchestratorSnapshot, RecursiveOrchestratorSnapshot, RecursiveOrchestratorSnapshot],
    ) -> tuple[str, int]:
        """K4 扫描：从三条比价线 snapshot 提取配置。

        返回 (config_label, polarity)。

        如果 k4_scanner 模块可用，调用 scan_k4()；
        否则使用 lstar 推导走势方向作为 fallback。
        """
        if _HAS_K4_SCANNER:
            result = scan_k4(snapshots[0], snapshots[1], snapshots[2], level=1)
            cfg = result.config
            label = f"({_dir_char(cfg.sigma_e)},{_dir_char(cfg.sigma_c)},{_dir_char(cfg.sigma_r)})"
            return (label, result.polarity)

        # Fallback：从 lstar 推导方向
        directions = []
        for snap in snapshots:
            if snap.lstar is not None:
                directions.append(_lstar_to_direction(snap.lstar))
            else:
                directions.append(WalkDirection.FLAT)

        cfg = Configuration(sigma_e=directions[0], sigma_c=directions[1], sigma_r=directions[2])
        pol = polarity_index(cfg)
        label = f"({_dir_char(cfg.sigma_e)},{_dir_char(cfg.sigma_c)},{_dir_char(cfg.sigma_r)})"
        return (label, pol)

    def _scan_candidates(
        self,
        config_label: str,
        polarity: int,
        candidate_snapshots: dict[str, RecursiveOrchestratorSnapshot],
    ) -> tuple[str, float, str] | None:
        """选股扫描。

        返回 (symbol, price, level) 或 None。

        如果 stock_scanner 模块可用，调用 scan_candidates()；
        否则从 candidate_snapshots 中检测买点作为 fallback。
        """
        if _HAS_STOCK_SCANNER:
            result = scan_candidates(
                Configuration(
                    sigma_e=WalkDirection.FLAT,
                    sigma_c=WalkDirection.FLAT,
                    sigma_r=WalkDirection.FLAT,
                ),
                candidate_snapshots,
            )
            if result.selected is not None:
                return (result.selected.symbol, result.selected.price, result.selected.level)
            return None

        # Fallback：检测买点
        if polarity <= 0:
            return None

        for sym, snap in candidate_snapshots.items():
            for bp in snap.bsp_snapshot.buysellpoints:
                if bp.confirmed and bp.side == "buy":
                    key = (bp.seg_idx, bp.kind, bp.side, bp.level_id)
                    if key not in self._seen_bsp_keys:
                        self._seen_bsp_keys.add(key)
                        return (sym, bp.price, str(bp.level_id))
        return None

    def _detect_bsp_signal(
        self,
        snap: RecursiveOrchestratorSnapshot,
        current_price: float,
    ) -> tuple[str, float, str] | None:
        """从持仓标的 snapshot 检测买卖点信号。

        返回 (event_type_name, price, level) 或 None。
        event_type_name: "SUB_LEVEL_SELL_POINT" / "SUB_LEVEL_BUY_POINT" /
                         "BUY_POINT_NEGATED" / "MAIN_LEVEL_SELL_POINT"
        """
        # 检测买点失效（价格跌破入场价）
        if self._held_entry_price > 0 and current_price < self._held_entry_price * 0.9:
            return ("BUY_POINT_NEGATED", current_price, self._fsm.entry_level)

        # 检测次级别买卖点
        for bp in snap.bsp_snapshot.buysellpoints:
            if not bp.confirmed:
                continue
            key = (bp.seg_idx, bp.kind, bp.side, bp.level_id)
            if key in self._seen_bsp_keys:
                continue
            self._seen_bsp_keys.add(key)

            if bp.side == "sell":
                return ("SUB_LEVEL_SELL_POINT", bp.price, str(bp.level_id))
            if bp.side == "buy":
                return ("SUB_LEVEL_BUY_POINT", bp.price, str(bp.level_id))

        return None

    def process_bar(
        self,
        k4_bars: tuple[Bar, Bar, Bar],
        candidate_bars: dict[str, Bar],
    ) -> BarStep:
        """处理单 bar。"""
        bar_ts = k4_bars[0].ts.timestamp()

        # 1. 推进 K4 三条比价线
        k4_snaps = tuple(
            orch.process_bar(bar)
            for orch, bar in zip(self._k4_orchestrators, k4_bars)
        )

        # 2. K4 扫描
        config_label, polarity = self._scan_k4(k4_snaps)  # type: ignore[arg-type]

        scanner_selected: str | None = None

        # 3-6. 根据 FSM 状态分派
        self._current_bar_missed = 0

        if self._fsm.state == CostState.SCANNING:
            self._handle_scanning(polarity, candidate_bars, config_label)
            if self._fsm.state == CostState.POSITION_OPEN:
                scanner_selected = self._held_symbol

        elif self._fsm.state in (CostState.POSITION_OPEN, CostState.COST_REDUCING):
            self._handle_position(candidate_bars)

        elif self._fsm.state == CostState.PRINCIPAL_WITHDRAWN:
            self._handle_principal_withdrawn(candidate_bars)

        elif self._fsm.state == CostState.STOPPED_OUT:
            self._handle_stopped_out()

        # 7. 记录 BarStep
        step = BarStep(
            bar_idx=self._bar_idx,
            bar_ts=bar_ts,
            k4_config_label=config_label,
            k4_polarity=polarity,
            scanner_selected=scanner_selected,
            fsm_state=self._fsm.state.name,
            cost_basis=self._fsm.cost_basis,
            cumulative_recovered=self._fsm.cumulative_recovered,
            total_shares=self._fsm.total_shares,
            missed_signals_count=self._current_bar_missed,
        )
        self._steps.append(step)
        self._bar_idx += 1
        return step

    def _handle_scanning(
        self,
        polarity: int,
        candidate_bars: dict[str, Bar],
        config_label: str,
    ) -> None:
        """SCANNING 状态处理。"""
        # 推进所有候选标的
        candidate_snaps: dict[str, RecursiveOrchestratorSnapshot] = {}
        for sym, bar in candidate_bars.items():
            orch = self._get_or_create_orchestrator(sym)
            candidate_snaps[sym] = orch.process_bar(bar)

        # 选股扫描
        scan_result = self._scan_candidates(config_label, polarity, candidate_snaps)
        if scan_result is not None:
            symbol, price, level = scan_result
            self._held_symbol = symbol
            self._held_entry_price = price
            self._transition_fsm(FsmEvent(
                event_type=FsmEventType.BUY_POINT_CONFIRMED,
                price=price,
                level=level,
            ))

    def _handle_position(self, candidate_bars: dict[str, Bar]) -> None:
        """POSITION_OPEN / COST_REDUCING 状态处理。"""
        if self._held_symbol is None:
            return

        # 机会成本跟踪：推进其他标的并检测买点（267号§五不换仓原则）
        self._track_missed_signals(candidate_bars)

        bar = candidate_bars.get(self._held_symbol)
        if bar is None:
            return

        orch = self._get_or_create_orchestrator(self._held_symbol)
        snap = orch.process_bar(bar)

        signal = self._detect_bsp_signal(snap, bar.close)
        if signal is None:
            return

        event_name, price, level = signal
        event_type = FsmEventType[event_name]
        self._transition_fsm(FsmEvent(
            event_type=event_type,
            price=price,
            level=level,
        ))

    def _handle_principal_withdrawn(self, candidate_bars: dict[str, Bar]) -> None:
        """PRINCIPAL_WITHDRAWN 状态处理。"""
        if self._held_symbol is None:
            return

        # 机会成本跟踪
        self._track_missed_signals(candidate_bars)

        bar = candidate_bars.get(self._held_symbol)
        if bar is None:
            return

        orch = self._get_or_create_orchestrator(self._held_symbol)
        snap = orch.process_bar(bar)

        signal = self._detect_bsp_signal(snap, bar.close)
        if signal is None:
            return

        event_name, price, level = signal
        event_type = FsmEventType[event_name]
        self._transition_fsm(FsmEvent(
            event_type=event_type,
            price=price,
            level=level,
        ))

    def _track_missed_signals(self, candidate_bars: dict[str, Bar]) -> None:
        """跟踪持仓期间其他标的出现的买点信号（不换仓原则机会成本）。

        267号§五：持仓期间不因外部机会换仓。
        此方法不影响交易决策，仅记录错过的信号用于事后分析。
        """
        self._current_bar_missed = 0
        for sym, bar in candidate_bars.items():
            if sym == self._held_symbol:
                continue
            orch = self._get_or_create_orchestrator(sym)
            snap = orch.process_bar(bar)
            for bp in snap.bsp_snapshot.buysellpoints:
                if not bp.confirmed or bp.side != "buy":
                    continue
                key = (sym, bp.seg_idx, bp.kind, bp.side, bp.level_id)
                if key in self._missed_bsp_keys:
                    continue
                self._missed_bsp_keys.add(key)
                self._current_bar_missed += 1

    def _handle_stopped_out(self) -> None:
        """STOPPED_OUT 状态处理：自动 RESET。"""
        remaining_capital = self._estimate_remaining_capital()
        self._transition_fsm(FsmEvent(
            event_type=FsmEventType.RESET,
            price=0.0,
            level="",
            new_own_capital=remaining_capital,
        ))
        self._held_symbol = None
        self._held_entry_price = 0.0
        self._seen_bsp_keys.clear()

    def _estimate_remaining_capital(self) -> float:
        """估算止损后剩余资金。

        简化计算：own_capital - (entry_price - current_cost) * total_shares。
        如果 FSM 中有持仓信息，使用 FSM 数据。
        """
        fsm = self._fsm
        if fsm.total_shares <= 0 or fsm.entry_price <= 0:
            return fsm.own_capital

        loss_per_share = max(0.0, fsm.entry_price - fsm.cost_basis)
        total_loss = loss_per_share * fsm.total_shares
        remaining = max(0.0, fsm.own_capital - total_loss + fsm.cumulative_recovered)
        return remaining

    def result(self) -> FullPipelineResult:
        """返回回测结果。"""
        return FullPipelineResult(
            config=self._config,
            steps=list(self._steps),
            fsm_transitions=list(self._fsm_transitions),
            final_fsm=self._fsm,
            total_bars=self._bar_idx,
        )


# ═══════════════════════════════════════════════════════════════
# 便捷入口
# ═══════════════════════════════════════════════════════════════


def run_full_pipeline(
    config: FullPipelineConfig,
    k4_bar_streams: tuple[list[Bar], list[Bar], list[Bar]],
    candidate_bar_streams: dict[str, list[Bar]],
) -> FullPipelineResult:
    """给定全部 bar 数据，运行完整回测。

    k4_bar_streams: (SPY_bars, GLD_bars, TLT_bars)，同步对齐
    candidate_bar_streams: {symbol: bars}，所有候选标的

    如果 K4 流长度不一致，取最短长度。
    """
    engine = FullPipelineEngine(config)

    # 取最短长度
    k4_len = min(len(s) for s in k4_bar_streams) if k4_bar_streams[0] else 0
    if k4_len == 0:
        return engine.result()

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
        engine.process_bar(k4_bars, candidate_bars)

    return engine.result()


# ═══════════════════════════════════════════════════════════════
# 辅助函数
# ═══════════════════════════════════════════════════════════════


def _dir_char(d: WalkDirection) -> str:
    """WalkDirection → "+"/"-"/"0" 字符。"""
    if d == WalkDirection.UP:
        return "+"
    if d == WalkDirection.DOWN:
        return "-"
    return "0"


def _lstar_to_direction(lstar) -> WalkDirection:
    """从 LStar 推导 WalkDirection（L2 近似）。

    lstar.direction 不一定存在，fallback 到 FLAT。
    """
    direction = getattr(lstar, "direction", None)
    if direction is None:
        return WalkDirection.FLAT
    if direction > 0:
        return WalkDirection.UP
    if direction < 0:
        return WalkDirection.DOWN
    return WalkDirection.FLAT
