"""RecTStrategy —— 递归 T 引擎的 NautilusTrader Strategy（目标敞口跟踪，NETTING）。

设计：docs/recursive_t_architecture_v2.md §5（单 Strategy 内部递归树，否决 actor-per-level）。
递归 T 引擎（newchan_rust.RecTStream）是**位置权威**——每 bar 输出目标净敞口（signed units）；
Strategy 提单使 NT 净仓位 = 目标（NETTING，delta = target − current）。NT 加真实撮合/滑点/佣金/
保证金（v2 §5.2：T step 降级为目标敞口，venue 是真账本）。同一 Strategy 类回测/实盘零修改。

与 ChanlunStrategy（BSP 信号→订单）的范畴差：递归引擎产出**仓位**不是信号，故走目标敞口跟踪
（NETTING 镜像），不走 BSP→BUY/SELL 映射。
"""

from __future__ import annotations

import newchan_rust
from nautilus_trader.config import StrategyConfig
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.model.enums import OrderSide
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.trading.strategy import Strategy


class RecTStrategyConfig(StrategyConfig, frozen=True):
    """递归 T 策略配置。

    mode                : 步骤c 走势完美判定 structural/and/or（受控实验唯一变量）。
    min_rebalance_units : 目标敞口变化阈值——小于此不提单（避免微小 churn / size 精度噪声）。
    """

    instrument_id: InstrumentId
    bar_type: BarType
    mode: str = "structural"
    min_rebalance_units: float = 1e-4


class RecTStrategy(Strategy):
    """递归 T 目标敞口跟踪策略（NETTING）。

    数据流（每 bar）：
        on_bar(Bar) → engine.push_bar(o,h,l,c) → 目标净敞口 signed units
                    → _rebalance: delta = target − NT净仓位 → market order 提单
    """

    def __init__(self, config: RecTStrategyConfig) -> None:
        super().__init__(config)
        self.engine = newchan_rust.RecTStream(config.mode)
        self.instrument = None
        self._watermark_ns: int | None = None
        self.n_bars = 0
        self.n_orders = 0
        self.n_dups = 0
        self.last_target: float = 0.0
        # 诊断（§9 质询）：敞口符号分布——验证"持有宏观空头"假说。
        self.n_bars_short = 0
        self.n_bars_long = 0
        self.n_bars_flat = 0

    # ── 生命周期 ──────────────────────────────────────────────

    def on_start(self) -> None:
        self.instrument = self.cache.instrument(self.config.instrument_id)
        if self.instrument is None:
            self.log.error(f"找不到 instrument {self.config.instrument_id}，停止")
            self.stop()
            return
        self.subscribe_bars(self.config.bar_type)
        self.log.info(f"RecTStrategy 启动: {self.config.bar_type} mode={self.config.mode}")

    def on_bar(self, bar: Bar) -> None:
        ts = bar.ts_event
        if self._watermark_ns is not None and ts <= self._watermark_ns:
            self.n_dups += 1
            return  # 单调性守卫：重复/倒退丢弃
        self._watermark_ns = ts
        self.n_bars += 1
        target = self.engine.push_bar(
            bar.open.as_double(),
            bar.high.as_double(),
            bar.low.as_double(),
            bar.close.as_double(),
        )
        self.last_target = target
        if target > 1e-9:
            self.n_bars_long += 1
        elif target < -1e-9:
            self.n_bars_short += 1
        else:
            self.n_bars_flat += 1
        self._rebalance(target)

    def _rebalance(self, target_units: float) -> None:
        """提单使 NT 净仓位 = 引擎目标净敞口（NETTING，delta 提单）。"""
        assert self.instrument is not None
        cur = float(self.portfolio.net_position(self.config.instrument_id))
        delta = target_units - cur
        if abs(delta) < self.config.min_rebalance_units:
            return
        side = OrderSide.BUY if delta > 0 else OrderSide.SELL
        qty = self.instrument.make_qty(abs(delta))
        if qty == 0:
            return  # 小于 size 精度，跳过
        order = self.order_factory.market(
            instrument_id=self.config.instrument_id,
            order_side=side,
            quantity=qty,
        )
        self.submit_order(order)
        self.n_orders += 1

    def on_stop(self) -> None:
        self.close_all_positions(self.config.instrument_id)
        ec = self.engine.op_counts()  # (enter, sink, recover, spawn, reruns)
        eng_nav = self.engine.finish()  # 引擎内部模拟 final_nav（对照 NT 真账本）
        tot = max(1, self.n_bars)
        self.log.info(
            f"RecTStrategy 停止: bars={self.n_bars} orders={self.n_orders} dups={self.n_dups} "
            f"引擎操作(enter/sink/recover/spawn/reruns)={ec} 引擎模拟final_nav={eng_nav:.2f}"
        )
        self.log.info(
            f"敞口分布: 多={self.n_bars_long}({100*self.n_bars_long/tot:.1f}%) "
            f"空={self.n_bars_short}({100*self.n_bars_short/tot:.1f}%) "
            f"平={self.n_bars_flat}({100*self.n_bars_flat/tot:.1f}%) "
            f"[空头占比高=持有宏观空头被牛市轧的直接证据]"
        )
