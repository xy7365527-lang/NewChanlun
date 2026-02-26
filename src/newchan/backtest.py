"""回测引擎 — 消费 RecursiveOrchestrator 输出，评估买卖点信号质量。

设计原则：
- 与 RecursiveOrchestrator 解耦，仅消费 RecursiveOrchestratorSnapshot
- 严格防未来函数：入场价 = BSP 确认 bar 的 close，不使用未来数据
- 不可变交易记录（frozen dataclass）
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal


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

    @property
    def pnl(self) -> float:
        """盈亏（点数）。"""
        if self.side == "long":
            return self.exit_price - self.entry_price
        return self.entry_price - self.exit_price

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


# -- 内部可变状态（不暴露） --

@dataclass
class _OpenPosition:
    """当前持仓（内部可变）。"""

    side: Literal["long", "short"]
    bsp_kind: str
    bsp_level: int
    entry_bar: int
    entry_price: float


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

    def __init__(self, config: BacktestConfig | None = None) -> None:
        self._config = config or BacktestConfig()
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
                self._position = _OpenPosition(
                    side="long",
                    bsp_kind=bp.kind,
                    bsp_level=bp.level_id,
                    entry_bar=bar_idx,
                    entry_price=price,
                )
            elif new_sells and self._config.allow_short:
                bp = new_sells[0]
                self._position = _OpenPosition(
                    side="short",
                    bsp_kind=bp.kind,
                    bsp_level=bp.level_id,
                    entry_bar=bar_idx,
                    entry_price=price,
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
        self._trades.append(Trade(
            side=pos.side,
            bsp_kind=pos.bsp_kind,
            bsp_level=pos.bsp_level,
            entry_bar=pos.entry_bar,
            exit_bar=bar_idx,
            entry_price=pos.entry_price,
            exit_price=price,
            exit_reason=reason,
        ))
        self._position = None
