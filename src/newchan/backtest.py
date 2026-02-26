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
    exit_reason: Literal["reverse_bsp", "stop_loss"]
    entry_slippage: float = 0.0
    exit_slippage: float = 0.0
    entry_commission: float = 0.0
    exit_commission: float = 0.0

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
    ) -> None:
        self._config = config or BacktestConfig()
        self._cost_config = cost_config or CostConfig()
        self._trades: list[Trade] = []
        self._position: _OpenPosition | None = None
        self._seen_bsp_keys: set[tuple[int, str, str, int]] = set()
        self._bar_count = 0

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

        # 1. 检查止损
        if self._position is not None and self._config.stop_loss_pct > 0:
            pos = self._position
            if pos.side == "long":
                loss_pct = (pos.entry_price - price) / pos.entry_price
            else:
                loss_pct = (price - pos.entry_price) / pos.entry_price
            if loss_pct >= self._config.stop_loss_pct:
                self._close_position(bar_idx, price, "stop_loss")

        # 2. 检查反向 BSP 平仓
        if self._position is not None:
            pos = self._position
            if pos.side == "long" and new_sells:
                self._close_position(bar_idx, price, "reverse_bsp")
            elif pos.side == "short" and new_buys:
                self._close_position(bar_idx, price, "reverse_bsp")

        # 3. 开仓
        if self._position is None:
            if new_buys:
                bp = new_buys[0]
                entry = self._apply_slippage(price, "buy")
                comm = self._calc_commission(entry)
                self._position = _OpenPosition(
                    side="long",
                    bsp_kind=bp.kind,
                    bsp_level=bp.level_id,
                    entry_bar=bar_idx,
                    entry_price=entry,
                    raw_entry_price=price,
                    entry_slippage=entry - price,
                    entry_commission=comm,
                )
            elif new_sells and self._config.allow_short:
                bp = new_sells[0]
                entry = self._apply_slippage(price, "sell")
                comm = self._calc_commission(entry)
                self._position = _OpenPosition(
                    side="short",
                    bsp_kind=bp.kind,
                    bsp_level=bp.level_id,
                    entry_bar=bar_idx,
                    entry_price=entry,
                    raw_entry_price=price,
                    entry_slippage=entry - price,
                    entry_commission=comm,
                )

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
        self, bar_idx: int, price: float, reason: Literal["reverse_bsp", "stop_loss"],
    ) -> None:
        pos = self._position
        if pos is None:
            return
        exit_side = "sell" if pos.side == "long" else "buy"
        exit_price = self._apply_slippage(price, exit_side)
        exit_comm = self._calc_commission(exit_price)
        self._trades.append(Trade(
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
        ))
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
