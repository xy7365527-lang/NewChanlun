"""ChanlunStrategy —— NautilusTrader Strategy 子类（生命周期壳）。

设计文档 §3（analysis/nautilus_integration_design.md）：
Strategy 是薄壳，只做生命周期/订阅/订单事件路由；
缠论结构计算在 ChanlunBridge → Rust 引擎；
下单经 LmtExecutor（LMT-only）→ MakerOptimizer（≤60s 撤单不追）。

同一 Strategy 类回测/实盘零修改（官方保证：Strategy ⊂ Actor 双模式）。
"""

from __future__ import annotations

from nautilus_trader.config import StrategyConfig
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.model.events import OrderCanceled, OrderFilled, OrderRejected
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.trading.strategy import Strategy

from trading_system.execution.leverage_calculator import LeverageCalculator
from trading_system.execution.lmt_executor import LmtExecutor
from trading_system.execution.maker_optimizer import MakerOptimizer
from trading_system.strategy.signal_bridge import ChanlunBridge, FeedResult


class ChanlunStrategyConfig(StrategyConfig, frozen=True):
    """缠论策略配置。

    enable_orders=False 是阶段1（信号贯通）的只读形态——喂引擎、记录信号、不下单。
    阶段3（回测闭环）接入订单后置 True。
    """

    instrument_id: InstrumentId
    bar_type: BarType
    # ── 引擎参数（newchan_rust.RecursiveOrchestrator）──
    engine_max_levels: int = 6
    engine_stroke_mode: str = "wide"
    # ── 交易层 ──
    trading_mode: str = "fusion_tr"  # TODO(阶段2): PositionalStream mode 变体名
    enable_orders: bool = False  # 阶段1只读；阶段3+置True
    order_timeout_secs: int = 60  # maker 在册判决：≤60s 撤单不追价
    maint_margin_rate: float = 0.10  # mm；阶段3从 Instrument.margin_maint 读


class ChanlunStrategy(Strategy):
    """缠论赋格策略壳。

    数据流（每 bar）：
        on_bar(Bar) → bridge.feed(o,h,l,c) → Rust 引擎结构推进
                    → bridge.drain_signals() → [ChanlunSignal]
                    → (enable_orders) LeverageCalculator 钳制 → MakerOptimizer 挂单
    回报流：
        on_order_filled/canceled/rejected → MakerOptimizer 状态机
        TODO(阶段2): → PositionalStream.confirm_fill/reject_intent（影子账本收敛）
    """

    def __init__(self, config: ChanlunStrategyConfig) -> None:
        super().__init__(config)
        self.bridge = ChanlunBridge(
            engine_config=dict(
                max_levels=config.engine_max_levels,
                stroke_mode=config.engine_stroke_mode,
            ),
        )
        self.leverage = LeverageCalculator(maint_margin_rate=config.maint_margin_rate)
        self.lmt: LmtExecutor | None = None  # on_start 创建（需要 order_factory）
        self.maker: MakerOptimizer | None = None
        self._signal_count = 0

    # ── 生命周期 ────────────────────────────────────────────────

    def on_start(self) -> None:
        instrument = self.cache.instrument(self.config.instrument_id)
        if instrument is None:
            self.log.error(f"找不到 instrument {self.config.instrument_id}，停止")
            self.stop()
            return
        self.lmt = LmtExecutor(strategy=self, instrument=instrument)
        self.maker = MakerOptimizer(
            strategy=self,
            executor=self.lmt,
            timeout_secs=self.config.order_timeout_secs,
        )
        # TODO(阶段4 预热协议 §2.3): 冷启动 catalog 重放 → request_bars 补缺口 → 订阅。
        # 回测模式下数据全量来自 engine.add_data，直接订阅即可。
        self.subscribe_bars(self.config.bar_type)
        self.log.info(
            f"ChanlunStrategy 启动: {self.config.bar_type} "
            f"mode={self.config.trading_mode} orders={'ON' if self.config.enable_orders else 'OFF(只读)'}"
        )

    def on_bar(self, bar: Bar) -> None:
        result = self.bridge.feed(bar)
        if result is not FeedResult.ACCEPTED:
            return  # 重叠丢弃（已计数）

        signals = self.bridge.drain_signals()
        for sig in signals:
            self._signal_count += 1
            self.log.info(f"[信号] {sig.action} L{sig.level} @{sig.price:.4f} {sig.reason}")
            if self.config.enable_orders:
                self._execute_signal(sig, bar)

    def _execute_signal(self, sig, bar: Bar) -> None:
        """信号 → 杠杆钳制 → maker 挂单。"""
        assert self.maker is not None
        band = self.bridge.current_zhongshu_band()
        if band is None:
            self.log.warning("无中枢带，D_struct 不可计算，跳过信号")
            return
        zd, zg = band
        price = bar.close.as_double()
        account = self.portfolio.account(self.config.instrument_id.venue)
        if account is None:
            self.log.warning("账户不可用，跳过信号")
            return
        equity = account.balance_total().as_double()
        assert self.lmt is not None
        qty = self.leverage.max_quantity(
            equity=equity,
            price=price,
            zd=zd,
            zg=zg,
            notional_frac=sig.notional_frac,
            contract_multiplier=float(self.lmt.instrument.multiplier),
        )
        if qty <= 0:
            return
        self.maker.place(side=sig.action, quantity=qty, anchor_price=sig.price)

    # ── 订单事件路由 ────────────────────────────────────────────

    def on_order_filled(self, event: OrderFilled) -> None:
        if self.maker is not None:
            self.maker.on_filled(event)
        # TODO(阶段2): stream.confirm_fill(intent_id, fill_price, fill_qty, bar)
        #   —— 影子账本只在成交确认时更新（设计判决二：venue 是真账本）。

    def on_order_canceled(self, event: OrderCanceled) -> None:
        if self.maker is not None:
            self.maker.on_canceled(event)
        # TODO(阶段2): stream.reject_intent(intent_id) —— 撤单=意图作废，不追价。

    def on_order_rejected(self, event: OrderRejected) -> None:
        if self.maker is not None:
            self.maker.on_rejected(event)

    def on_stop(self) -> None:
        self.cancel_all_orders(self.config.instrument_id)
        snap = self.bridge.structure_snapshot()
        self.log.info(
            f"ChanlunStrategy 停止: bars={self.bridge.bar_count} "
            f"signals={self._signal_count} gaps={self.bridge.gap_count} "
            f"dups={self.bridge.dup_count} structure={snap}"
        )
