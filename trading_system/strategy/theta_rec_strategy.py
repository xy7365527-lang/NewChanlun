"""ThetaRecStrategy —— theta_v0 π 回路（ThetaStream）的 NautilusTrader 策略（目标敞口跟踪，NETTING）。

#951 建新件：`rec_t_strategy.py`（recursive_t 旧流式出口）的继任者。继任 seam =
`ThetaStream.push_bar(o, h, l, c, p_t, nav) -> f64`（返回目标净敞口 p_star）——与旧
`recursive_t` 流式出口的 `push_bar(o,h,l,c) -> f64`（返回 lu−su）同范畴同量纲（#808 D1 出口 ①）。

与旧策略的差异（设计稿 #951 §3.2 处置 #1）：
- 引擎换 `ThetaStream`（构造行 + push_bar 参数表补 `p_t`/`nav`）；`_rebalance` 的
  `delta = target − cur` 逻辑逐字不变。
- `mode`(structural/and/or) 实验自变量在 theta_v0 零命中（#808 B3）⟹ 本策略**删去 mode 轴**
  （旧缓存 `t_fugue_*.json` 不可复现，见 #808）。
- 收尾：`finish_full()`（per-leg 账本 dict）替代旧 `op_counts()`/`finish()`（引擎内部模拟 nav）。

状态裁定（设计稿 #951 §1.3）：`p_t`（真实净持仓）每 bar 显式传入，成交回执不回填 Rust。
"""

from __future__ import annotations

import newchan_rust
from nautilus_trader.config import StrategyConfig
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.model.enums import OrderSide
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.trading.strategy import Strategy


class ThetaRecStrategyConfig(StrategyConfig, frozen=True):
    """theta_v0 π 目标敞口跟踪策略配置。

    min_rebalance_units : 目标敞口变化阈值——小于此不提单（避免微小 churn / size 精度噪声）。
    initial_nav         : `push_bar` 的 `nav<=0` 时兜底净值（正常路径 nav 取账户真余额）。
    """

    instrument_id: InstrumentId
    bar_type: BarType
    min_rebalance_units: float = 1e-4
    initial_nav: float = 1.0e6


class ThetaRecStrategy(Strategy):
    """theta_v0 π 目标敞口跟踪策略（NETTING）。

    数据流（每 bar）：
        on_bar(Bar) → engine.push_bar(o,h,l,c,p_t,nav) → 目标净敞口 p_star（有符号手数）
                   → _rebalance: delta = target − NT净仓位 → market order 提单
    """

    def __init__(self, config: ThetaRecStrategyConfig) -> None:
        super().__init__(config)
        self.engine = newchan_rust.ThetaStream(initial_nav=config.initial_nav)
        self.instrument = None
        self._watermark_ns: int | None = None
        self.n_bars = 0
        self.n_orders = 0
        self.n_dups = 0
        self.last_target: float = 0.0
        # 诊断（§9 质询）：敞口符号分布。
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
        self.log.info(f"ThetaRecStrategy 启动: {self.config.bar_type}")

    def on_bar(self, bar: Bar) -> None:
        ts = bar.ts_event
        if self._watermark_ns is not None and ts <= self._watermark_ns:
            self.n_dups += 1
            return  # 单调性守卫：重复/倒退丢弃
        self._watermark_ns = ts
        self.n_bars += 1
        # p_t = NT 真实净持仓（手数，有符号）；nav = 账户总余额（NAV/价 = 可建名义手数）。
        p_t = float(self.portfolio.net_position(self.config.instrument_id))
        target = self.engine.push_bar(
            bar.open.as_double(),
            bar.high.as_double(),
            bar.low.as_double(),
            bar.close.as_double(),
            p_t,
            self._nav(),
        )
        self.last_target = target
        if target > 1e-9:
            self.n_bars_long += 1
        elif target < -1e-9:
            self.n_bars_short += 1
        else:
            self.n_bars_flat += 1
        self._rebalance(target)

    def _nav(self) -> float:
        """账户总余额（净值真值源）；账户不可用 ⟹ 构造期 initial_nav 兜底。"""
        account = self.portfolio.account(self.config.instrument_id.venue)
        if account is None:
            return self.config.initial_nav
        nav = account.balance_total().as_double()
        return nav if nav > 0.0 else self.config.initial_nav

    def _rebalance(self, target_units: float) -> None:
        """提单使 NT 净仓位 = 引擎目标净敞口（NETTING，delta 提单）。与旧策略逐字同逻辑。"""
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
        d = self.engine.finish_full()
        tot = max(1, self.n_bars)
        self.log.info(
            f"ThetaRecStrategy 停止: bars={self.n_bars} orders={self.n_orders} dups={self.n_dups} "
            f"n_orders(delta≠0)={d.get('n_orders')} reconcile_residual={d.get('reconcile_residual')}"
        )
        self.log.info(
            f"敞口分布: 多={self.n_bars_long}({100*self.n_bars_long/tot:.1f}%) "
            f"空={self.n_bars_short}({100*self.n_bars_short/tot:.1f}%) "
            f"平={self.n_bars_flat}({100*self.n_bars_flat/tot:.1f}%)"
        )
