"""Hyperliquid 连接验证 —— NautilusTrader 1.228 内置 adapter 路径（mainnet）。

验证项（与 hl_verify_sdk.py 对照）：
  1. exec client 连接 + 账户余额上报（Portfolio.account）
  2. data client：BTC-USD-PERP trade tick 订阅 + 1m EXTERNAL bar 订阅
  3. nautilus 订单流：post-only LMT 买单 @ mid×0.5 → ACCEPTED → 撤单 → CANCELED

安全约束与 SDK 版一致：post_only（≡ HL tif=Alo，可成交即拒）+ 限价远低于市价 + 0.001 BTC。
运行 ~75s 后自动停机；退出码 0 = 全部验证项通过。

用法：
  HYPERLIQUID_PRIVATE_KEY=... .venv/bin/python trading_system/live/hl_verify_nautilus.py
"""

from __future__ import annotations

import asyncio
import os
import sys

from nautilus_trader.adapters.hyperliquid import (
    HYPERLIQUID,
    HyperliquidDataClientConfig,
    HyperliquidExecClientConfig,
    HyperliquidLiveDataClientFactory,
    HyperliquidLiveExecClientFactory,
)
from nautilus_trader.config import InstrumentProviderConfig, LoggingConfig, TradingNodeConfig
from nautilus_trader.core.nautilus_pyo3 import HyperliquidEnvironment
from nautilus_trader.live.node import TradingNode
from nautilus_trader.model.data import Bar, BarType, TradeTick
from nautilus_trader.model.enums import OrderSide, TimeInForce
from nautilus_trader.model.identifiers import InstrumentId, Venue
from nautilus_trader.model.orders import LimitOrder
from nautilus_trader.trading.strategy import Strategy

INSTRUMENT_ID = InstrumentId.from_str("BTC-USD-PERP.HYPERLIQUID")
BAR_TYPE = BarType.from_str("BTC-USD-PERP.HYPERLIQUID-1-MINUTE-LAST-EXTERNAL")
ORDER_SIZE = "0.001"
MAX_PX_RATIO = 0.6  # 限价必须 < 最新成交价 × 0.6
RUN_SECS = 75

results: dict[str, bool] = {
    "account": False,
    "trade_tick": False,
    "bar_1m": False,
    "order_accepted": False,
    "order_canceled": False,
}


class HlVerifyStrategy(Strategy):
    def __init__(self) -> None:
        super().__init__()
        self._tick_count = 0
        self._order: LimitOrder | None = None
        self._last_px: float | None = None

    def on_start(self) -> None:
        instrument = self.cache.instrument(INSTRUMENT_ID)
        if instrument is None:
            self.log.error(f"instrument {INSTRUMENT_ID} 未加载")
            self.stop()
            return
        self.log.info(
            f"instrument 已加载: price_precision={instrument.price_precision} "
            f"size_precision={instrument.size_precision}"
        )
        self.subscribe_trade_ticks(INSTRUMENT_ID)
        self.subscribe_bars(BAR_TYPE)

        account = self.portfolio.account(Venue(HYPERLIQUID))
        if account is not None:
            results["account"] = True
            self.log.info(f"账户状态: {account.balances_total()}")

    def on_trade_tick(self, tick: TradeTick) -> None:
        self._tick_count += 1
        self._last_px = float(tick.price)
        if self._tick_count == 1:
            results["trade_tick"] = True
            self.log.info(f"首笔 trade tick: px={tick.price} sz={tick.size}")
        if self._tick_count == 3 and self._order is None:
            self._place_probe_order()

    def on_bar(self, bar: Bar) -> None:
        results["bar_1m"] = True
        self.log.info(f"1m bar: O={bar.open} H={bar.high} L={bar.low} C={bar.close}")

    def _place_probe_order(self) -> None:
        instrument = self.cache.instrument(INSTRUMENT_ID)
        assert self._last_px is not None
        limit_px = round(self._last_px * 0.5, -2)  # mid 一半，取整百
        if limit_px >= self._last_px * MAX_PX_RATIO:
            self.log.error(f"安全检查失败: limit_px={limit_px}")
            self.stop()
            return
        self._order = self.order_factory.limit(
            instrument_id=INSTRUMENT_ID,
            order_side=OrderSide.BUY,
            quantity=instrument.make_qty(ORDER_SIZE),
            price=instrument.make_price(limit_px),
            time_in_force=TimeInForce.GTC,
            post_only=True,  # ≡ HL tif=Alo：可成交即拒，不会 taker 成交
        )
        self.log.info(f"提交探针单: BUY {ORDER_SIZE} @ {limit_px}（last={self._last_px}）")
        self.submit_order(self._order)

    def on_order_accepted(self, event) -> None:
        results["order_accepted"] = True
        self.log.info(f"订单 ACCEPTED: venue_order_id={event.venue_order_id}")
        self.cancel_order(self._order)

    def on_order_canceled(self, event) -> None:
        results["order_canceled"] = True
        self.log.info(f"订单 CANCELED: venue_order_id={event.venue_order_id}")

    def on_order_rejected(self, event) -> None:
        self.log.error(f"订单 REJECTED: {event.reason}")

    def on_stop(self) -> None:
        # 兜底：若探针单仍在册（撤单事件未到），再发一次撤单
        if self._order is not None and self._order.is_open:
            self.cancel_order(self._order)


async def main() -> None:
    pk = os.environ.get("HYPERLIQUID_PRIVATE_KEY")
    if not pk:
        print("[FAIL] 环境变量 HYPERLIQUID_PRIVATE_KEY 未设置")
        sys.exit(1)

    config = TradingNodeConfig(
        trader_id="HLVERIFY-001",
        logging=LoggingConfig(log_level="INFO"),
        data_clients={
            HYPERLIQUID: HyperliquidDataClientConfig(
                environment=HyperliquidEnvironment.MAINNET,
                instrument_provider=InstrumentProviderConfig(load_all=True),
            ),
        },
        exec_clients={
            HYPERLIQUID: HyperliquidExecClientConfig(
                environment=HyperliquidEnvironment.MAINNET,
                private_key=pk,  # adapter 默认读 HYPERLIQUID_PK，与本机变量名不同，显式传入
                instrument_provider=InstrumentProviderConfig(load_all=True),
            ),
        },
        timeout_disconnection=10.0,
        timeout_post_stop=5.0,
    )
    node = TradingNode(config=config)
    node.trader.add_strategy(HlVerifyStrategy())
    node.add_data_client_factory(HYPERLIQUID, HyperliquidLiveDataClientFactory)
    node.add_exec_client_factory(HYPERLIQUID, HyperliquidLiveExecClientFactory)
    node.build()

    try:
        run_task = asyncio.ensure_future(node.run_async())
        await asyncio.sleep(RUN_SECS)
        await node.stop_async()
        run_task.cancel()
    finally:
        node.dispose()

    print("\n========== 验证结果 ==========")
    for key, ok in results.items():
        print(f"  {key:16s}: {'PASS' if ok else 'FAIL'}")
    sys.exit(0 if all(results.values()) else 2)


if __name__ == "__main__":
    asyncio.run(main())
