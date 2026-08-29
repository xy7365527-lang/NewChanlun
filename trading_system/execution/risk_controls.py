"""PreTradeRiskGate —— paper 试运行风控四条（#1283 Q3 / #1311）。

目的（#1283 Q3 裁定原文口径）：**paper 阶段风控目的 = 防程序事故，非防市场风险**
（市场风险正是 paper 要量的对象）。故本模块是**执行安全网**，不是缠论结构止损/
sizing 那套（那套在 Rust `theta_v0::strategy::risk`，本模块不重复）。

四条（每条一个独立判定 + 一条触发测试）：

    1. 单笔上限        single_order：单笔名义 ≤ 权益 × max_single_order_frac（默认 1%）
    2. 净持仓上限      net_position：下单后 |净持仓名义| ≤ 权益 × max_net_position_frac
    3. 频率熔断        frequency：滚动时间窗内下单次数 < max_orders_per_window
    4. kill switch    kill_switch：trip 后拒绝一切新单 + 要求撤全部挂单 + 平全部仓

纯 Python、零 nautilus 依赖：可与 Strategy/LmtExecutor/MakerOptimizer 装配
（在 `maker.place(...)` 之前调 `gate.check(...)`），也可脱离 nautilus 独立单测
（tests/test_risk_controls.py）。

装配位置（#1311 首跑链，T1 后置时落）：
    信号 → LeverageCalculator（结构杠杆） → **RiskGate.check（本模块）** → MakerOptimizer.place
    断单/异常捕获 → RiskGate.trip（kill switch）→ 撤全部挂单 + 平全部仓
"""

from __future__ import annotations

import enum
from dataclasses import dataclass

# 时间单位统一纳秒（与仓内 ts_event_ns 同口径）。
NANOS_PER_SEC = 1_000_000_000


class RiskVerdictAction(enum.Enum):
    """风控裁决动作。"""

    ALLOW = "allow"       # 放行（可通过，订单计入频率窗口）
    REJECT = "reject"     # 拒单（单笔/净持仓/频率任一超限，不计数）
    FLATTEN = "flatten"   # kill switch 已 trip：拒新单 + 必须撤单平仓


@dataclass(frozen=True)
class RiskVerdict:
    """一次下单前风控裁决（不可变，诊断可回溯）。"""

    allowed: bool
    action: RiskVerdictAction
    reason: str


@dataclass(frozen=True)
class RiskLimits:
    """风控四条阈值（全部可配置；默认值是 paper 防事故档，非市场风险档）。

    净持仓上限默认 1.0（净名义 ≤ 1× 权益）对齐 Rust RiskConfig.gamma=1.0 NAV；
    频率窗口默认 10s/10 笔、单笔 1% 为 #1283 Q3 明写或 paper 阶段占位，
    具体数值随 #1280 §5 决策点一起待编排者拍板，落定前不改默认。
    """

    max_single_order_frac: float = 0.01   # 单笔 ≤ 权益 1%（#1283 Q3 明写）
    max_net_position_frac: float = 1.0    # 净持仓名义 ≤ 1× 权益（占位，对齐 γ=1.0）
    max_orders_per_window: int = 10       # 频率熔断：窗口内下单次数上限（占位）
    rate_window_ns: int = 10 * NANOS_PER_SEC  # 频率窗口 10s（占位）

    def __post_init__(self) -> None:
        if not (0.0 < self.max_single_order_frac <= 1.0):
            raise ValueError(
                f"max_single_order_frac 须在 (0,1]: {self.max_single_order_frac}"
            )
        if not (0.0 < self.max_net_position_frac):
            raise ValueError(
                f"max_net_position_frac 须 >0: {self.max_net_position_frac}"
            )
        if self.max_orders_per_window < 1:
            raise ValueError(
                f"max_orders_per_window 须 ≥1: {self.max_orders_per_window}"
            )
        if self.rate_window_ns <= 0:
            raise ValueError(f"rate_window_ns 须 >0: {self.rate_window_ns}")


@dataclass(frozen=True)
class OrderIntent:
    """一笔待发订单的下单前描述（风控只关心这四个量，不含 venue 语义）。"""

    side: str            # "BUY" | "SELL"（与 LmtExecutor 同口径）
    qty: float           # 非负手数/币数（BTC perp = BTC 数量）
    price: float         # 限价
    contract_multiplier: float = 1.0  # 期货乘数；BTC perp = 1.0

    def __post_init__(self) -> None:
        # fail-fast：符号语义错误静默吞掉比报错更危险（signed_notional 依赖 side 极性）。
        if self.side not in ("BUY", "SELL"):
            raise ValueError(f"非法 side: {self.side!r}（须 BUY/SELL）")
        if self.qty < 0:
            raise ValueError(f"qty 须非负: {self.qty}")
        if self.price <= 0:
            raise ValueError(f"price 须 >0: {self.price}")
        if self.contract_multiplier <= 0:
            raise ValueError(f"contract_multiplier 须 >0: {self.contract_multiplier}")

    @property
    def signed_notional(self) -> float:
        """带方向名义：BUY=+，SELL=−。"""
        notional = abs(self.qty) * self.price * self.contract_multiplier
        return notional if self.side == "BUY" else -notional


class RiskGate:
    """paper 试运行风控四条的有状态闸门。

    一个实例绑定一个标的（净持仓口径 per-instrument）。状态：
      - `_order_timestamps`：频率窗口内已放行订单的时间戳（ns，滚动清理）；
      - `_tripped` / `_trip_reason`：kill switch 状态（手动 reset 才解除）。
    裁决计数器（诊断，对齐 MakerOptimizer 计数风格）：reject_by_single_order /
    reject_by_net_position / reject_by_frequency / rate_breach_count。
    """

    def __init__(self, limits: RiskLimits | None = None) -> None:
        self._limits = limits or RiskLimits()
        self._order_timestamps: list[int] = []
        self._tripped = False
        self._trip_reason: str | None = None
        # 裁决诊断计数
        self.reject_by_single_order = 0
        self.reject_by_net_position = 0
        self.reject_by_frequency = 0
        self.rate_breach_count = 0

    # ── kill switch ─────────────────────────────────────────────

    @property
    def tripped(self) -> bool:
        return self._tripped

    @property
    def trip_reason(self) -> str | None:
        return self._trip_reason

    def trip(self, reason: str) -> None:
        """急停：拒绝一切新单，要求撤全部挂单 + 平全部仓。

        幂等——重复 trip 不覆盖首个原因（首个原因是最早的现场线索）。
        解除只能 `reset()`（手动重装，不给自动恢复的隐式通道）。
        """
        if not self._tripped:
            self._tripped = True
            self._trip_reason = reason

    def reset(self) -> None:
        """手动重装（kill switch 解除）。不清空频率窗口与计数。"""
        self._tripped = False
        self._trip_reason = None

    @property
    def required_actions(self) -> tuple[str, ...]:
        """trip 状态下的强制动作清单；未 trip 为空。"""
        if self._tripped:
            return ("cancel_all_open_orders", "close_all_positions")
        return ()

    # ── 频率窗口（admission 语义：通过 check 的订单才计数）────────

    def _orders_in_window(self, now_ns: int) -> int:
        floor = now_ns - self._limits.rate_window_ns
        return sum(1 for ts in self._order_timestamps if ts > floor)

    def _prune(self, now_ns: int) -> None:
        floor = now_ns - self._limits.rate_window_ns
        self._order_timestamps = [ts for ts in self._order_timestamps if ts > floor]

    def _record(self, now_ns: int) -> None:
        self._prune(now_ns)
        self._order_timestamps.append(now_ns)

    # ── 裁决主入口 ───────────────────────────────────────────────

    def check(
        self,
        intent: OrderIntent,
        *,
        now_ns: int,
        equity: float,
        net_position_notional: float,
    ) -> RiskVerdict:
        """下单前裁决（四条按序短路：kill switch → 频率 → 单笔 → 净持仓）。

        参数：
            intent                 待发订单（side/qty/price/乘数）
            now_ns                 当前时间戳（ns）
            equity                 账户权益（美元；<=0 ⟹ 拒绝，fail-safe）
            net_position_notional  当前净持仓名义（有符号；多头 +，空头 −）

        返回 RiskVerdict；仅 ALLOW 会消耗一个频率窗口名额（admission 语义）。
        """
        # 0. kill switch —— 优先级最高，trip 期间一律 FLATTEN
        if self._tripped:
            return RiskVerdict(
                allowed=False,
                action=RiskVerdictAction.FLATTEN,
                reason=f"kill_switch_tripped: {self._trip_reason}",
            )

        # 1. 频率熔断（滚动窗口计数；本单通过才计数，先查后记）
        if self._orders_in_window(now_ns) >= self._limits.max_orders_per_window:
            self.reject_by_frequency += 1
            self.rate_breach_count += 1
            return RiskVerdict(
                allowed=False,
                action=RiskVerdictAction.REJECT,
                reason="frequency_circuit_breaker",
            )

        # 2. 单笔上限
        if equity <= 0:
            return RiskVerdict(
                allowed=False,
                action=RiskVerdictAction.REJECT,
                reason="equity_nonpositive",
            )
        single_limit = equity * self._limits.max_single_order_frac
        notional = abs(intent.signed_notional)
        if notional > single_limit:
            self.reject_by_single_order += 1
            return RiskVerdict(
                allowed=False,
                action=RiskVerdictAction.REJECT,
                reason="single_order_over_limit",
            )

        # 3. 净持仓上限
        new_net = net_position_notional + intent.signed_notional
        net_limit = equity * self._limits.max_net_position_frac
        if abs(new_net) > net_limit:
            self.reject_by_net_position += 1
            return RiskVerdict(
                allowed=False,
                action=RiskVerdictAction.REJECT,
                reason="net_position_over_limit",
            )

        # 全过 → 计数（admission）
        self._record(now_ns)
        return RiskVerdict(allowed=True, action=RiskVerdictAction.ALLOW, reason="ok")

    @property
    def stats(self) -> dict:
        """诊断计数快照（验收/日志用）。"""
        return {
            "tripped": self._tripped,
            "orders_in_window": len(self._order_timestamps),
            "reject_by_single_order": self.reject_by_single_order,
            "reject_by_net_position": self.reject_by_net_position,
            "reject_by_frequency": self.reject_by_frequency,
            "rate_breach_count": self.rate_breach_count,
        }
