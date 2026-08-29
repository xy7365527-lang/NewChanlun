"""试运行风控四条（#1283 Q3 / #1311）触发测试。

四条各有一组触发测试（验收子句「风控四条各有触发测试」）：
  1. 单笔 ≤ 权益 1%       TestSingleOrderLimit
  2. 净持仓上限           TestNetPositionLimit
  3. 频率熔断             TestFrequencyCircuitBreaker
  4. kill switch          TestKillSwitch

纯逻辑层，零 nautilus 依赖——import trading_system.execution 不强制 nautilus
（execution/__init__.py 对 LmtExecutor/MakerOptimizer 惰性导入）。
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT))

from trading_system.execution import (  # noqa: E402 —— 验证惰性 __init__ 导出路径
    OrderIntent,
    RiskGate,
    RiskLimits,
    RiskVerdictAction,
)
from trading_system.execution.risk_controls import NANOS_PER_SEC  # noqa: E402

T0 = 1_700_000_000_000_000_000  # 任意大 ns 纪元锚，避开浮点误差


def _intent(side: str, qty: float, price: float, multiplier: float = 1.0) -> OrderIntent:
    return OrderIntent(side=side, qty=qty, price=price, contract_multiplier=multiplier)


class TestSingleOrderLimit:
    """单笔 ≤ 权益 1%（#1283 Q3 ①）。"""

    def test_over_1pct_rejected(self):
        gate = RiskGate()  # max_single_order_frac = 0.01
        # 权益 100_000 → 单笔上限 1_000；本单名义 2_000 超限
        v = gate.check(
            _intent("BUY", qty=0.02, price=100_000),
            now_ns=T0,
            equity=100_000.0,
            net_position_notional=0.0,
        )
        assert v.allowed is False
        assert v.action is RiskVerdictAction.REJECT
        assert v.reason == "single_order_over_limit"
        assert gate.reject_by_single_order == 1

    def test_within_1pct_allowed(self):
        gate = RiskGate()
        v = gate.check(
            _intent("BUY", qty=0.005, price=100_000),  # 名义 500 ≤ 1_000
            now_ns=T0,
            equity=100_000.0,
            net_position_notional=0.0,
        )
        assert v.allowed is True
        assert v.action is RiskVerdictAction.ALLOW

    def test_futures_multiplier_counts_into_notional(self):
        # 期货乘数 1000：0.001 手 × 100 × 1000 = 100 名义，超过 1%×5000=50
        gate = RiskGate()
        v = gate.check(
            _intent("BUY", qty=0.001, price=100.0, multiplier=1000.0),
            now_ns=T0,
            equity=5_000.0,
            net_position_notional=0.0,
        )
        assert v.allowed is False
        assert v.reason == "single_order_over_limit"


class TestNetPositionLimit:
    """净持仓上限（#1283 Q3 ②）。

    用自定义阈值隔离本检查：max_single_order_frac=1.0（单笔不抢先触发）、
    max_net_position_frac=0.5（净帽 = 0.5×权益，比单笔更紧，才能单独触发）。
    """

    def _gate(self) -> RiskGate:
        return RiskGate(
            RiskLimits(max_single_order_frac=1.0, max_net_position_frac=0.5)
        )

    def test_buy_pushes_net_over_cap_rejected(self):
        gate = self._gate()
        # 权益 100_000 → 净帽 50_000；现有 +40_000，再买 20_000 → +60_000 超帽
        v = gate.check(
            _intent("BUY", qty=0.20, price=100_000),
            now_ns=T0,
            equity=100_000.0,
            net_position_notional=+40_000.0,
        )
        assert v.allowed is False
        assert v.reason == "net_position_over_limit"
        assert gate.reject_by_net_position == 1

    def test_buy_net_within_cap_allowed(self):
        gate = self._gate()
        # +10_000 → net=+50_000 恰好到顶
        v = gate.check(
            _intent("BUY", qty=0.10, price=100_000),
            now_ns=T0,
            equity=100_000.0,
            net_position_notional=+40_000.0,
        )
        assert v.allowed is True

    def test_sell_flips_net_short_over_cap_rejected(self):
        gate = self._gate()
        # 现有 +40_000，卖 100_000 → net = −60_000，|net| > 50_000
        v = gate.check(
            _intent("SELL", qty=1.00, price=100_000),
            now_ns=T0,
            equity=100_000.0,
            net_position_notional=+40_000.0,
        )
        assert v.allowed is False
        assert v.reason == "net_position_over_limit"


class TestFrequencyCircuitBreaker:
    """频率熔断（#1283 Q3 ③）：滚动窗口内下单次数上限。"""

    def _gate(self) -> RiskGate:
        return RiskGate(
            RiskLimits(max_orders_per_window=3, rate_window_ns=10 * NANOS_PER_SEC)
        )

    def test_fourth_order_within_window_rejected(self):
        gate = self._gate()
        for i in range(3):
            v = gate.check(
                _intent("BUY", qty=0.001, price=100_000),
                now_ns=T0 + i * NANOS_PER_SEC,
                equity=100_000.0,
                net_position_notional=0.0,
            )
            assert v.allowed is True
        # 第 4 笔仍在 10s 窗口内 → 熔断
        v = gate.check(
            _intent("BUY", qty=0.001, price=100_000),
            now_ns=T0 + 3 * NANOS_PER_SEC,
            equity=100_000.0,
            net_position_notional=0.0,
        )
        assert v.allowed is False
        assert v.reason == "frequency_circuit_breaker"
        assert gate.reject_by_frequency == 1
        assert gate.rate_breach_count == 1

    def test_window_slides_and_recovers(self):
        gate = self._gate()
        for i in range(3):
            assert gate.check(
                _intent("BUY", qty=0.001, price=100_000),
                now_ns=T0 + i * NANOS_PER_SEC,
                equity=100_000.0,
                net_position_notional=0.0,
            ).allowed
        # 窗口滑过：T0 那笔（t=0s）已滑出 10s 窗口，可再放行
        v = gate.check(
            _intent("BUY", qty=0.001, price=100_000),
            now_ns=T0 + 11 * NANOS_PER_SEC,
            equity=100_000.0,
            net_position_notional=0.0,
        )
        assert v.allowed is True


class TestKillSwitch:
    """kill switch（#1283 Q3 ④）：trip 后拒一切新单 + 强制撤单平仓。"""

    def test_trip_blocks_everything_and_demands_flatten(self):
        gate = RiskGate()
        gate.trip("manual_stop: 操作员急停")
        v = gate.check(
            _intent("BUY", qty=0.001, price=100_000),
            now_ns=T0,
            equity=100_000.0,
            net_position_notional=0.0,
        )
        assert v.allowed is False
        assert v.action is RiskVerdictAction.FLATTEN
        assert "manual_stop" in v.reason
        assert gate.required_actions == ("cancel_all_open_orders", "close_all_positions")

    def test_trip_is_idempotent_keeps_first_reason(self):
        gate = RiskGate()
        gate.trip("first")
        gate.trip("second")
        assert gate.trip_reason == "first"

    def test_reset_rearms(self):
        gate = RiskGate()
        gate.trip("manual_stop")
        gate.reset()
        assert gate.tripped is False
        assert gate.required_actions == ()
        v = gate.check(
            _intent("BUY", qty=0.001, price=100_000),
            now_ns=T0,
            equity=100_000.0,
            net_position_notional=0.0,
        )
        assert v.allowed is True


class TestFailSafe:
    """边界与 fail-safe（非正权益、非法配置）。"""

    def test_nonpositive_equity_rejected(self):
        gate = RiskGate()
        v = gate.check(
            _intent("BUY", qty=0.001, price=100_000),
            now_ns=T0,
            equity=0.0,
            net_position_notional=0.0,
        )
        assert v.allowed is False
        assert v.reason == "equity_nonpositive"

    def test_invalid_limits_rejected(self):
        with pytest.raises(ValueError):
            RiskLimits(max_single_order_frac=1.5)
        with pytest.raises(ValueError):
            RiskLimits(max_net_position_frac=0.0)
        with pytest.raises(ValueError):
            RiskLimits(max_orders_per_window=0)
        with pytest.raises(ValueError):
            RiskLimits(rate_window_ns=0)

    def test_signed_notional_sign(self):
        assert _intent("BUY", 2.0, 100.0).signed_notional == pytest.approx(+200.0)
        assert _intent("SELL", 2.0, 100.0).signed_notional == pytest.approx(-200.0)

    def test_invalid_intent_rejected_fast(self):
        with pytest.raises(ValueError):
            OrderIntent(side="hold", qty=1.0, price=100.0)
        with pytest.raises(ValueError):
            OrderIntent(side="BUY", qty=-1.0, price=100.0)
        with pytest.raises(ValueError):
            OrderIntent(side="BUY", qty=1.0, price=0.0)
        with pytest.raises(ValueError):
            OrderIntent(side="BUY", qty=1.0, price=100.0, contract_multiplier=0.0)
