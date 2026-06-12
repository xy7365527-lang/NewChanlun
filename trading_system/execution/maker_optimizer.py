"""MakerOptimizer —— maker 执行状态机（在册判决的直接实装，设计 §5.2）。

在册定型（maker_fill_rate_feasibility）：
    限价挂单 ≤60s 未成交则撤单，**不追价**；撤单优于追价 27pp；
    fill rate 下界 66.6%（30s）/ 74.4%（60s）已过相对 alpha 前沿。

状态机：
    IDLE ──place──▶ PLACED(LMT @ anchor_price, 启动60s定时器)
    PLACED ──on_filled──▶ IDLE（confirm）
    PLACED ──定时器到期──▶ cancel_order → CANCELING
    CANCELING ──on_canceled──▶ IDLE（意图作废，不追价）
    PLACED ──on_rejected──▶ IDLE（计数告警）
    部分成交：on_filled(partial) 记录；到期撤余量

定时器用 Nautilus clock.set_time_alert()——回测/实盘同一时钟抽象，
60s 撤单窗口第一次可以在回测里被忠实模拟（用 Nautilus 撮合层的核心收益）。
"""

from __future__ import annotations

import enum
from datetime import timedelta
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from nautilus_trader.model.events import OrderCanceled, OrderFilled, OrderRejected

    from trading_system.execution.lmt_executor import LmtExecutor


class MakerState(enum.Enum):
    IDLE = "idle"
    PLACED = "placed"
    CANCELING = "canceling"


class MakerOptimizer:
    """单标的 maker 挂单→等候→撤单状态机。

    骨架限制：单并发挂单（PENDING 期间不发新意图——防重复开腿，设计 §4.2）。
    TODO(阶段2): per-voice 多槽并发（intent 带 voice_key，每槽独立状态机）。
    """

    def __init__(self, strategy, executor: LmtExecutor, timeout_secs: int = 60) -> None:
        self._strategy = strategy
        self._executor = executor
        self._timeout = timedelta(seconds=timeout_secs)
        self._state = MakerState.IDLE
        self._active_order_id = None
        self._alert_name: str | None = None
        # 诊断计数（阶段3验收判据：撤单计数 > 0 = 撤单逻辑在回测中可观测）
        self.fill_count = 0
        self.cancel_count = 0
        self.reject_count = 0
        self.skip_count = 0  # 因占用被跳过的意图数

    @property
    def state(self) -> MakerState:
        return self._state

    # ── 意图入口 ────────────────────────────────────────────────

    def place(self, side: str, quantity: float, anchor_price: float) -> bool:
        """挂限价单。返回是否接受（PENDING 占用时拒绝——不排队不追）。"""
        if self._state is not MakerState.IDLE:
            self.skip_count += 1
            return False
        order = self._executor.build_limit_order(
            side=side, quantity=quantity, limit_price=anchor_price,
        )
        self._executor.submit(order)
        self._active_order_id = order.client_order_id
        self._state = MakerState.PLACED
        self._alert_name = f"maker-timeout-{order.client_order_id}"
        self._strategy.clock.set_time_alert(
            name=self._alert_name,
            alert_time=self._strategy.clock.utc_now() + self._timeout,
            callback=self._on_timeout,
        )
        return True

    # ── 定时器 ──────────────────────────────────────────────────

    def _on_timeout(self, event) -> None:
        if self._state is not MakerState.PLACED:
            return  # 已成交/已拒，定时器迟到，无操作
        order = self._strategy.cache.order(self._active_order_id)
        if order is not None and order.is_open:
            self._strategy.cancel_order(order)
            self._state = MakerState.CANCELING

    # ── 订单事件 ────────────────────────────────────────────────

    def on_filled(self, event: OrderFilled) -> None:
        if event.client_order_id != self._active_order_id:
            return
        order = self._strategy.cache.order(self._active_order_id)
        if order is not None and order.is_open:
            return  # 部分成交：保持挂单，到期撤余量
        self.fill_count += 1
        self._reset()

    def on_canceled(self, event: OrderCanceled) -> None:
        if event.client_order_id != self._active_order_id:
            return
        self.cancel_count += 1
        # 撤单 = 意图永久作废，不追价（在册判决）。等待下一个结构事件。
        self._reset()

    def on_rejected(self, event: OrderRejected) -> None:
        if event.client_order_id != self._active_order_id:
            return
        self.reject_count += 1
        self._reset()

    def _reset(self) -> None:
        if self._alert_name is not None:
            try:
                self._strategy.clock.cancel_timer(self._alert_name)
            except Exception:
                pass  # 定时器已触发/已清理
            self._alert_name = None
        self._active_order_id = None
        self._state = MakerState.IDLE
