"""回测引擎 — 消费 RecursiveOrchestrator 输出，评估买卖点信号质量。

设计原则：
- 与 RecursiveOrchestrator 解耦，仅消费 RecursiveOrchestratorSnapshot
- 严格防未来函数：入场价 = BSP 确认 bar 的 close，不使用未来数据
- 不可变交易记录（frozen dataclass）
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal

from newchan.cost.config import CostConfig
from newchan.risk.config import RiskConfig
from newchan.risk.stop_loss import ATRStopLoss


@dataclass(frozen=True, slots=True)
class BacktestConfig:
    """回测配置。

    Attributes
    ----------
    stop_loss_pct : float
        固定止损百分比（0.05 = 5%）。0 表示不启用止损。
    allow_short : bool
        是否允许做空。False = 仅做多（sell BSP 仅用于平多仓）。
    """

    stop_loss_pct: float = 0.05
    allow_short: bool = False


@dataclass(frozen=True, slots=True)
class Trade:
    """一笔已完成的交易记录。"""

    side: Literal["long", "short"]
    bsp_kind: str
    bsp_level: int
    entry_bar: int
    exit_bar: int
    entry_price: float
    exit_price: float
    exit_reason: Literal["reverse_bsp", "stop_loss", "drawdown_halt"]
    entry_slippage: float = 0.0
    exit_slippage: float = 0.0
    entry_commission: float = 0.0
    exit_commission: float = 0.0
    position_size: float = 1.0

    @property
    def total_cost(self) -> float:
        """总成本 = 滑点成本 + 手续费。"""
        slippage_cost = abs(self.entry_slippage) + abs(self.exit_slippage)
        return slippage_cost + self.entry_commission + self.exit_commission

    @property
    def pnl(self) -> float:
        """盈亏（点数），已扣除成本。"""
        if self.side == "long":
            raw = self.exit_price - self.entry_price
        else:
            raw = self.entry_price - self.exit_price
        return raw - self.entry_commission - self.exit_commission

    @property
    def pnl_pct(self) -> float:
        """盈亏百分比。"""
        if self.entry_price == 0:
            return 0.0
        return self.pnl / self.entry_price

    @property
    def hold_bars(self) -> int:
        """持仓 bar 数。"""
        return self.exit_bar - self.entry_bar


@dataclass(frozen=True, slots=True)
class CostSummary:
    """成本统计摘要。"""

    total_slippage: float
    total_commission: float
    total_cost: float
    cost_to_gross_profit_ratio: float


@dataclass(frozen=True, slots=True)
class BacktestResult:
    """回测统计结果。"""

    trades: tuple[Trade, ...]
    total_bars: int

    @property
    def trade_count(self) -> int:
        return len(self.trades)

    @property
    def win_count(self) -> int:
        return sum(1 for t in self.trades if t.pnl > 0)

    @property
    def loss_count(self) -> int:
        return sum(1 for t in self.trades if t.pnl <= 0)

    @property
    def win_rate(self) -> float:
        """胜率。无交易时返回 0。"""
        if not self.trades:
            return 0.0
        return self.win_count / len(self.trades)

    @property
    def profit_loss_ratio(self) -> float:
        """盈亏比 = 平均盈利 / 平均亏损绝对值。无亏损时返回 inf，无盈利时返回 0。"""
        wins = [t.pnl for t in self.trades if t.pnl > 0]
        losses = [t.pnl for t in self.trades if t.pnl <= 0]
        if not losses:
            return float("inf") if wins else 0.0
        if not wins:
            return 0.0
        avg_win = sum(wins) / len(wins)
        avg_loss = abs(sum(losses) / len(losses))
        if avg_loss == 0:
            return float("inf")
        return avg_win / avg_loss

    @property
    def max_drawdown_pct(self) -> float:
        """最大回撤百分比（基于累计 PnL 曲线）。"""
        if not self.trades:
            return 0.0
        cumulative = 0.0
        peak = 0.0
        max_dd = 0.0
        for t in self.trades:
            cumulative += t.pnl_pct
            if cumulative > peak:
                peak = cumulative
            dd = peak - cumulative
            if dd > max_dd:
                max_dd = dd
        return max_dd

    @property
    def cost_summary(self) -> CostSummary:
        """成本统计摘要。"""
        total_slippage = sum(
            abs(t.entry_slippage) + abs(t.exit_slippage) for t in self.trades
        )
        total_commission = sum(
            t.entry_commission + t.exit_commission for t in self.trades
        )
        total_cost = total_slippage + total_commission
        # 毛利 = 不扣成本的盈利交易之和
        gross_profit = 0.0
        for t in self.trades:
            raw = (
                (t.exit_price - t.entry_price)
                if t.side == "long"
                else (t.entry_price - t.exit_price)
            )
            if raw > 0:
                gross_profit += raw
        ratio = total_cost / gross_profit if gross_profit > 0 else 0.0
        return CostSummary(
            total_slippage=total_slippage,
            total_commission=total_commission,
            total_cost=total_cost,
            cost_to_gross_profit_ratio=ratio,
        )


@dataclass(frozen=True, slots=True)
class RiskAwareResult:
    """风控增强回测结果。

    在 BacktestResult 基础上增加：净值曲线、止损触发统计、仓位分布。
    """

    base: BacktestResult
    equity_curve: tuple[tuple[int, float], ...]
    stop_loss_trigger_count: int
    drawdown_halt_count: int
    position_sizes: tuple[float, ...]

    @property
    def avg_position_size(self) -> float:
        if not self.position_sizes:
            return 0.0
        return sum(self.position_sizes) / len(self.position_sizes)

    @property
    def max_position_size(self) -> float:
        if not self.position_sizes:
            return 0.0
        return max(self.position_sizes)

    @property
    def min_position_size(self) -> float:
        if not self.position_sizes:
            return 0.0
        return min(self.position_sizes)

    @property
    def final_equity(self) -> float:
        if not self.equity_curve:
            return 0.0
        return self.equity_curve[-1][1]

    @property
    def peak_equity(self) -> float:
        if not self.equity_curve:
            return 0.0
        return max(eq for _, eq in self.equity_curve)

    @property
    def max_drawdown_from_equity(self) -> float:
        """基于净值曲线的最大回撤比例。"""
        if not self.equity_curve:
            return 0.0
        peak = 0.0
        max_dd = 0.0
        for _, eq in self.equity_curve:
            if eq > peak:
                peak = eq
            if peak > 0:
                dd = (peak - eq) / peak
                if dd > max_dd:
                    max_dd = dd
        return max_dd


# -- 内部可变状态（不暴露） --

@dataclass
class _OpenPosition:
    """当前持仓（内部可变）。"""

    side: Literal["long", "short"]
    bsp_kind: str
    bsp_level: int
    entry_bar: int
    entry_price: float
    raw_entry_price: float = 0.0
    entry_slippage: float = 0.0
    entry_commission: float = 0.0
    position_size: float = 1.0


def _bsp_identity(bp) -> tuple[int, str, str, int]:
    """BuySellPoint 身份键。"""
    return (bp.seg_idx, bp.kind, bp.side, bp.level_id)


class BacktestEngine:
    """回测引擎 — 逐 snapshot 驱动。

    用法::

        engine = BacktestEngine(config)
        for bar in bars:
            snap = orchestrator.process_bar(bar)
            engine.process_snapshot(snap, bar)
        result = engine.result()
    """

    def __init__(
        self,
        config: BacktestConfig | None = None,
        cost_config: CostConfig | None = None,
        risk_config: RiskConfig | None = None,
    ) -> None:
        self._config = config or BacktestConfig()
        self._cost_config = cost_config or CostConfig()
        self._risk_config = risk_config
        self._trades: list[Trade] = []
        self._position: _OpenPosition | None = None
        self._seen_bsp_keys: set[tuple[int, str, str, int]] = set()
        self._bar_count = 0
        # 风控状态
        self._equity = risk_config.initial_equity if risk_config else 0.0
        self._equity_curve: list[tuple[int, float]] = []
        self._stop_loss_count = 0
        self._drawdown_halt_count = 0
        self._position_sizes: list[float] = []

    def process_snapshot(self, snapshot, bar) -> None:
        """处理一个 RecursiveOrchestratorSnapshot + 对应 Bar。

        Parameters
        ----------
        snapshot : RecursiveOrchestratorSnapshot
            当前 bar 的完整快照。
        bar : Bar
            当前 K 线（用于取 close 价格）。
        """
        self._bar_count += 1
        bar_idx = snapshot.bar_idx
        price = bar.close
        rc = self._risk_config

        # 更新 ATRStopLoss 缓冲区（如果使用）
        if rc and rc.stop_loss_strategy and isinstance(rc.stop_loss_strategy, ATRStopLoss):
            rc.stop_loss_strategy.update(bar)

        # 收集本 bar 新确认的 BSP
        new_buys: list = []
        new_sells: list = []
        for bp in snapshot.bsp_snapshot.buysellpoints:
            if not bp.confirmed:
                continue
            key = _bsp_identity(bp)
            if key in self._seen_bsp_keys:
                continue
            self._seen_bsp_keys.add(key)
            if bp.side == "buy":
                new_buys.append(bp)
            else:
                new_sells.append(bp)

        # 1. 检查止损（风控策略优先，回退到 BacktestConfig.stop_loss_pct）
        if self._position is not None:
            stop_triggered = False
            if rc and rc.stop_loss_strategy:
                result = rc.stop_loss_strategy.check_stop(
                    entry_price=self._position.entry_price,
                    current_bar=bar,
                    position_side=self._position.side,
                )
                stop_triggered = result.triggered
            elif self._config.stop_loss_pct > 0:
                pos = self._position
                if pos.side == "long":
                    loss_pct = (pos.entry_price - price) / pos.entry_price
                else:
                    loss_pct = (price - pos.entry_price) / pos.entry_price
                stop_triggered = loss_pct >= self._config.stop_loss_pct
            if stop_triggered:
                self._close_position(bar_idx, price, "stop_loss")
                self._stop_loss_count += 1

        # 2. 检查反向 BSP 平仓
        if self._position is not None:
            pos = self._position
            if pos.side == "long" and new_sells:
                self._close_position(bar_idx, price, "reverse_bsp")
            elif pos.side == "short" and new_buys:
                self._close_position(bar_idx, price, "reverse_bsp")

        # 3. 开仓（受 DrawdownGuard 约束）
        if self._position is None:
            # 回撤守卫检查
            allow_open = True
            if rc and rc.drawdown_guard:
                rc.drawdown_guard.update(self._equity)
                allow_open = rc.drawdown_guard.can_open_position()
                if not allow_open:
                    self._drawdown_halt_count += 1

            if allow_open:
                opened = False
                if new_buys:
                    bp = new_buys[0]
                    entry = self._apply_slippage(price, "buy")
                    comm = self._calc_commission(entry)
                    size = self._calc_position_size(entry, bar)
                    self._position = _OpenPosition(
                        side="long",
                        bsp_kind=bp.kind,
                        bsp_level=bp.level_id,
                        entry_bar=bar_idx,
                        entry_price=entry,
                        raw_entry_price=price,
                        entry_slippage=entry - price,
                        entry_commission=comm,
                        position_size=size,
                    )
                    opened = True
                elif new_sells and self._config.allow_short:
                    bp = new_sells[0]
                    entry = self._apply_slippage(price, "sell")
                    comm = self._calc_commission(entry)
                    size = self._calc_position_size(entry, bar)
                    self._position = _OpenPosition(
                        side="short",
                        bsp_kind=bp.kind,
                        bsp_level=bp.level_id,
                        entry_bar=bar_idx,
                        entry_price=entry,
                        raw_entry_price=price,
                        entry_slippage=entry - price,
                        entry_commission=comm,
                        position_size=size,
                    )
                    opened = True
                if opened:
                    self._position_sizes.append(self._position.position_size)

        # 4. 记录净值曲线
        if rc:
            self._equity_curve.append((bar_idx, self._equity))

    def result(self) -> BacktestResult:
        """返回回测结果。未平仓头寸不计入统计。"""
        return BacktestResult(
            trades=tuple(self._trades),
            total_bars=self._bar_count,
        )

    @property
    def has_open_position(self) -> bool:
        return self._position is not None

    def _close_position(
        self,
        bar_idx: int,
        price: float,
        reason: Literal["reverse_bsp", "stop_loss", "drawdown_halt"],
    ) -> None:
        pos = self._position
        if pos is None:
            return
        exit_side = "sell" if pos.side == "long" else "buy"
        exit_price = self._apply_slippage(price, exit_side)
        exit_comm = self._calc_commission(exit_price)
        trade = Trade(
            side=pos.side,
            bsp_kind=pos.bsp_kind,
            bsp_level=pos.bsp_level,
            entry_bar=pos.entry_bar,
            exit_bar=bar_idx,
            entry_price=pos.entry_price,
            exit_price=exit_price,
            exit_reason=reason,
            entry_slippage=pos.entry_slippage,
            exit_slippage=exit_price - price,
            entry_commission=pos.entry_commission,
            exit_commission=exit_comm,
            position_size=pos.position_size,
        )
        self._trades.append(trade)
        # 更新权益
        if self._risk_config:
            self._equity += trade.pnl * pos.position_size
        self._position = None

    def _apply_slippage(self, price: float, side: str) -> float:
        """应用滑点模型。无模型时返回原价。"""
        sm = self._cost_config.slippage_model
        if sm is None:
            return price
        return sm.apply(price, side)

    def _calc_commission(self, price: float) -> float:
        """计算手续费。无模型时返回 0。"""
        cm = self._cost_config.commission_model
        if cm is None:
            return 0.0
        return cm.calculate(price, self._cost_config.quantity)

    def _calc_position_size(self, entry_price: float, bar) -> float:
        """通过 PositionSizer 计算仓位。无策略时返回 1.0。"""
        rc = self._risk_config
        if rc is None or rc.position_sizer is None:
            return 1.0
        # 止损距离：优先从 stop_loss_strategy 获取，回退到 BacktestConfig.stop_loss_pct
        stop_distance = entry_price * self._config.stop_loss_pct if self._config.stop_loss_pct > 0 else entry_price * 0.05
        if rc.stop_loss_strategy:
            # 尝试获取精确止损价
            result = rc.stop_loss_strategy.check_stop(
                entry_price=entry_price,
                current_bar=bar,
                position_side="long",  # 方向在此处不影响距离计算
            )
            if result.stop_price > 0:
                stop_distance = abs(entry_price - result.stop_price)
                if stop_distance == 0:
                    stop_distance = entry_price * 0.05  # 防零除
        return rc.position_sizer.calculate_size(
            account_equity=self._equity,
            stop_distance=stop_distance,
        )

    def risk_result(self) -> RiskAwareResult:
        """返回风控增强回测结果。

        如果未配置 RiskConfig，equity_curve 为空，统计值为 0。
        """
        return RiskAwareResult(
            base=self.result(),
            equity_curve=tuple(self._equity_curve),
            stop_loss_trigger_count=self._stop_loss_count,
            drawdown_halt_count=self._drawdown_halt_count,
            position_sizes=tuple(self._position_sizes),
        )
