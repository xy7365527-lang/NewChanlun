"""LmtExecutor —— LMT-only 硬约束执行器（设计 §5.1）。

本类是**唯一**调用 order_factory 的地方，且只调用 order_factory.limit()。
防御深度：RiskEngine 之外加本地断言——任何 OrderType != LIMIT 的订单在
submit 前抛异常。不是过滤，是 fail-fast：出现 MKT 单意味着代码有 bug。

挂单价位（maker 被动侧）：
    买入：挂 BID（或 MID 取整到 tick）
    卖出：挂 ASK（或 MID 取整到 tick）
骨架阶段以信号锚定价（结构价）为限价基准——bar 数据无盘口；
阶段4 实盘接 QuoteTick 后切换被动侧报价。
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from nautilus_trader.model.enums import OrderSide, OrderType, TimeInForce
from nautilus_trader.model.objects import Price, Quantity

if TYPE_CHECKING:
    from nautilus_trader.model.instruments import Instrument
    from nautilus_trader.model.orders import LimitOrder
    from nautilus_trader.trading.strategy import Strategy


class MarketOrderForbidden(RuntimeError):
    """出现非 LIMIT 订单 = 代码 bug，立即崩溃，不静默过滤。"""


class LmtExecutor:
    """LMT 单构造 + 提交前断言。"""

    def __init__(self, strategy: Strategy, instrument: Instrument) -> None:
        self._strategy = strategy
        self._instrument = instrument

    def build_limit_order(
        self,
        side: str,
        quantity: float,
        limit_price: float,
        post_only: bool = False,
    ) -> LimitOrder:
        """构造限价单。side: "BUY" | "SELL"。

        post_only: Binance 侧可置 True（taker 即拒单，交易所级保证 maker）；
        IBKR 无 post-only 等价物，靠限价价位选择保证（保持 False）。
        """
        if side not in ("BUY", "SELL"):
            raise ValueError(f"非法 side: {side!r}")
        order_side = OrderSide.BUY if side == "BUY" else OrderSide.SELL
        order = self._strategy.order_factory.limit(
            instrument_id=self._instrument.id,
            order_side=order_side,
            quantity=Quantity(quantity, self._instrument.size_precision),
            price=Price(limit_price, self._instrument.price_precision),
            time_in_force=TimeInForce.GTC,  # 生命周期由 MakerOptimizer 定时器管理
            post_only=post_only,
        )
        return order

    def submit(self, order) -> None:
        """提交前 LMT-only 断言（fail-fast，绝对禁止 MKT）。"""
        if order.order_type != OrderType.LIMIT:
            raise MarketOrderForbidden(
                f"禁止非 LIMIT 订单: {order.order_type}。"
                "出现此异常说明有代码绕过了 LmtExecutor——这是 bug，不是可恢复错误。"
            )
        self._strategy.submit_order(order)

    @property
    def instrument(self) -> Instrument:
        return self._instrument
