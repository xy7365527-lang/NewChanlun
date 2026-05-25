"""回测引擎 — 消费 RecursiveOrchestrator 输出，评估买卖点信号质量。

设计原则：
- 与 RecursiveOrchestrator 解耦，仅消费 RecursiveOrchestratorSnapshot
- 严格防未来函数：入场价 = BSP 确认 bar 的 close，不使用未来数据
- 不可变交易记录（frozen dataclass）
- 缠论语言封闭性：退出条件基于走势结构，不引入外部金融工程概念
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Literal

from newchan.cost.config import CostConfig


# ── 仓位阶段（220号概念2） ──


class PositionPhase(Enum):
    """仓位阶段。

    COST_REDUCTION: 成本>0，短差降成本
    COST_ZERO: 成本归零（翻倍出半仓触发）— 过渡态，立即转入 EARN_SHARES
    EARN_SHARES: 成本=0，短差挣股票
    """

    COST_REDUCTION = "cost_reduction"
    COST_ZERO = "cost_zero"
    EARN_SHARES = "earn_shares"


# ── 短差记录 ──


@dataclass(frozen=True, slots=True)
class ShortDiffRecord:
    """一次短差操作记录。"""

    bar_idx: int
    side: Literal["reduce", "replenish"]
    price: float
    quantity: float
    sub_bsp_kind: str
    sub_bsp_level: int
    profit: float


# ── 退出条件参数 ──


@dataclass(frozen=True, slots=True)
class _ExitParams:
    """退出条件参数（按 BSP 类型不同）。

    Type1: 监控上涨中是否形成新同级别中枢
    Type2: 监控是否跌破前一下跌趋势最低点
    Type3: 监控回试是否跌破 ZG
    """

    bsp_kind: Literal["type1", "type2", "type3"]
    entry_zs_count: int | None = None
    prev_downtrend_low: float | None = None
    center_zg: float | None = None


# ── 配置 ──


@dataclass(frozen=True, slots=True)
class BacktestConfig:
    """回测配置。

    Attributes
    ----------
    operation_level : int
        操作级别（大级别买卖点）。
    short_diff_level : int | None
        短差级别（次级别），None = 不启用短差。
    allow_short : bool
        是否允许做空。False = 仅做多。
    initial_capital : float
        初始资金。
    position_fraction : float
        短差用机动资金比例（原文 1/10）。
    """

    operation_level: int = 1
    short_diff_level: int | None = None
    allow_short: bool = False
    initial_capital: float = 100_000.0
    position_fraction: float = 0.1


# ── 交易记录 ──


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
    exit_reason: Literal["reverse_bsp", "bsp_negated"]
    phase_at_exit: PositionPhase | None = None
    short_diff_count: int = 0
    cost_reduction: float = 0.0
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


# ── 成本统计 ──


@dataclass(frozen=True, slots=True)
class CostSummary:
    """成本统计摘要（纯执行成本事实，L1 管线度量；不含成本/毛利比=策略评价）。"""

    total_slippage: float
    total_commission: float
    total_cost: float


# ── 回测结果 ──


@dataclass(frozen=True, slots=True)
class BacktestResult:
    """回测统计结果。"""

    trades: tuple[Trade, ...]
    total_bars: int

    @property
    def trade_count(self) -> int:
        return len(self.trades)

    # 注：win_rate/profit_loss_ratio/max_drawdown_pct/win_count/loss_count 已删除
    # （pending-004 结算，2026-05-24）。这些是"测策略好不好"的绩效指标，违反
    # SKILL.md § 2.2（仅允许测代码对不对）、220号（风控非独立度量）、295号（第四层
    # 决断质量不可形式化）、蓝图第一原则（不回测策略）。回测只产 Trade 单笔事实 +
    # 成本统计（L1 管线度量），不评价策略优劣。

    @property
    def cost_summary(self) -> CostSummary:
        total_slippage = sum(
            abs(t.entry_slippage) + abs(t.exit_slippage) for t in self.trades
        )
        total_commission = sum(
            t.entry_commission + t.exit_commission for t in self.trades
        )
        total_cost = total_slippage + total_commission
        return CostSummary(
            total_slippage=total_slippage,
            total_commission=total_commission,
            total_cost=total_cost,
        )


# -- 内部可变状态 --


@dataclass
class _OpenPosition:
    """当前持仓（内部可变）。"""

    side: Literal["long", "short"]
    bsp_kind: str
    bsp_level: int
    entry_bar: int
    entry_price: float
    cost_basis: float
    phase: PositionPhase
    short_diffs: list[ShortDiffRecord]
    exit_params: _ExitParams
    raw_entry_price: float = 0.0
    entry_slippage: float = 0.0
    entry_commission: float = 0.0


def _bsp_identity(bp) -> tuple[int, str, str, int]:
    """BuySellPoint 身份键。"""
    return (bp.seg_idx, bp.kind, bp.side, bp.level_id)


def _build_exit_params(bp, zs_count: int) -> _ExitParams:
    """从 BuySellPoint 构建退出条件参数。"""
    kind = bp.kind
    if kind == "type1":
        return _ExitParams(
            bsp_kind="type1",
            entry_zs_count=zs_count,
        )
    elif kind == "type2":
        return _ExitParams(
            bsp_kind="type2",
            prev_downtrend_low=getattr(bp, "center_zd", None),
        )
    elif kind == "type3":
        return _ExitParams(
            bsp_kind="type3",
            center_zg=getattr(bp, "center_zg", None),
        )
    else:
        # 非标准 kind（如旧测试中的 "1st"）→ 无结构性退出条件
        return _ExitParams(bsp_kind="type1")


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
        self._seen_sub_bsp_keys: set[tuple[int, str, str, int]] = set()
        self._bar_count = 0

    def process_snapshot(self, snapshot, bar) -> None:
        """处理一个 RecursiveOrchestratorSnapshot + 对应 Bar。"""
        self._bar_count += 1
        bar_idx = snapshot.bar_idx
        price = bar.close

        # 收集本 bar 新确认的操作级别 BSP
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

        # 1. 检查退出条件（结构性退出，替代百分比止损）
        if self._position is not None and hasattr(snapshot, "zs_snapshot"):
            exit_reason = self._check_exit_condition(
                self._position, snapshot, price,
            )
            if exit_reason is not None:
                self._close_position(bar_idx, price, exit_reason)

        # 2. 检查反向 BSP 平仓
        if self._position is not None:
            pos = self._position
            if pos.side == "long" and new_sells:
                self._close_position(bar_idx, price, "reverse_bsp")
            elif pos.side == "short" and new_buys:
                self._close_position(bar_idx, price, "reverse_bsp")

        # 3. 短差程序
        if self._position is not None and hasattr(snapshot, "recursive_snapshots"):
            self._check_short_diff(self._position, snapshot, price, bar_idx)

        # 4. 开仓
        if self._position is None:
            zs_snap = getattr(snapshot, "zs_snapshot", None)
            zs_count = len(zs_snap.zhongshus) if zs_snap is not None else 0
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
                    cost_basis=entry,
                    phase=PositionPhase.COST_REDUCTION,
                    short_diffs=[],
                    exit_params=_build_exit_params(bp, zs_count),
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
                    cost_basis=entry,
                    phase=PositionPhase.COST_REDUCTION,
                    short_diffs=[],
                    exit_params=_build_exit_params(bp, zs_count),
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

    # ── 退出条件判定（220号概念1） ──

    def _check_exit_condition(
        self,
        pos: _OpenPosition,
        snapshot,
        price: float,
    ) -> Literal["bsp_negated", "reverse_bsp"] | None:
        """检查退出条件。

        退出条件 = 买入程序的判断条件被否定，不是价格偏离度量。
        """
        ep = pos.exit_params

        if ep.bsp_kind == "type1" and pos.side == "long":
            # Type1 买点退出：上涨中再次形成同级别中枢
            current_zs_count = len(snapshot.zs_snapshot.zhongshus)
            if (
                ep.entry_zs_count is not None
                and current_zs_count > ep.entry_zs_count
            ):
                return "bsp_negated"

        elif ep.bsp_kind == "type2" and pos.side == "long":
            # Type2 买点退出：跌破前一下跌趋势最低点
            if ep.prev_downtrend_low is not None and price < ep.prev_downtrend_low:
                return "bsp_negated"

        elif ep.bsp_kind == "type3" and pos.side == "long":
            # Type3 买点退出：回试跌破 ZG
            if ep.center_zg is not None and price < ep.center_zg:
                return "bsp_negated"

        return None

    # ── 短差程序（220号概念3） ──

    def _check_short_diff(
        self,
        pos: _OpenPosition,
        snapshot,
        price: float,
        bar_idx: int,
    ) -> None:
        """检查次级别 BSP 信号，执行短差操作。

        短差级别 = short_diff_level 的 BSP 信号。
        成本>0阶段：减仓量=回补量（不改变总仓位）
        成本=0阶段：回补金额=减仓金额（仓位增加）
        """
        if self._config.short_diff_level is None:
            return

        target_level = self._config.short_diff_level
        sub_bsp_snapshot = None

        # 从 recursive_snapshots 中找到次级别的 BSP 快照
        for rs in snapshot.recursive_snapshots:
            if rs.level_id == target_level and hasattr(rs, "bsp_snapshot") and rs.bsp_snapshot is not None:
                sub_bsp_snapshot = rs.bsp_snapshot
                break

        if sub_bsp_snapshot is None:
            return

        # 收集次级别新确认的 BSP
        for bp in sub_bsp_snapshot.buysellpoints:
            if not bp.confirmed:
                continue
            key = _bsp_identity(bp)
            if key in self._seen_sub_bsp_keys:
                continue
            self._seen_sub_bsp_keys.add(key)

            fraction = self._config.position_fraction

            if pos.side == "long" and bp.side == "sell":
                # 次级别卖点 → 减仓
                record = ShortDiffRecord(
                    bar_idx=bar_idx,
                    side="reduce",
                    price=price,
                    quantity=fraction,
                    sub_bsp_kind=bp.kind,
                    sub_bsp_level=bp.level_id,
                    profit=0.0,
                )
                pos.short_diffs.append(record)

            elif pos.side == "long" and bp.side == "buy":
                # 次级别买点 → 回补
                profit = 0.0
                if pos.short_diffs and pos.short_diffs[-1].side == "reduce":
                    profit = (pos.short_diffs[-1].price - price) * fraction
                    if pos.phase == PositionPhase.COST_REDUCTION:
                        pos.cost_basis = max(0.0, pos.cost_basis - profit)
                        if pos.cost_basis == 0.0:
                            pos.phase = PositionPhase.EARN_SHARES

                record = ShortDiffRecord(
                    bar_idx=bar_idx,
                    side="replenish",
                    price=price,
                    quantity=fraction,
                    sub_bsp_kind=bp.kind,
                    sub_bsp_level=bp.level_id,
                    profit=profit,
                )
                pos.short_diffs.append(record)

    # ── 平仓 ──

    def _close_position(
        self,
        bar_idx: int,
        price: float,
        reason: Literal["reverse_bsp", "bsp_negated"],
    ) -> None:
        pos = self._position
        if pos is None:
            return
        exit_side = "sell" if pos.side == "long" else "buy"
        exit_price = self._apply_slippage(price, exit_side)
        exit_comm = self._calc_commission(exit_price)
        total_cost_reduction = sum(
            r.profit for r in pos.short_diffs if r.side == "replenish"
        )
        self._trades.append(Trade(
            side=pos.side,
            bsp_kind=pos.bsp_kind,
            bsp_level=pos.bsp_level,
            entry_bar=pos.entry_bar,
            exit_bar=bar_idx,
            entry_price=pos.entry_price,
            exit_price=exit_price,
            exit_reason=reason,
            phase_at_exit=pos.phase,
            short_diff_count=len(pos.short_diffs),
            cost_reduction=total_cost_reduction,
            entry_slippage=pos.entry_slippage,
            exit_slippage=exit_price - price,
            entry_commission=pos.entry_commission,
            exit_commission=exit_comm,
        ))
        self._position = None

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
