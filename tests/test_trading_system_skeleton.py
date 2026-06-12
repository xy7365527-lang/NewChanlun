"""trading_system 骨架最小守卫测试。

覆盖：杠杆纯函数、LMT-only 断言、桥接层时间守卫。
（生成态例外不适用——这三块是纯技术层，定义已结算。）
"""

import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT))

from trading_system.execution.leverage_calculator import LeverageCalculator


class TestLeverageCalculator:
    def test_l_max_formula(self):
        """L_max = 1/(D_struct + mm)；OSC 相位 P_neg = ZD。"""
        calc = LeverageCalculator(maint_margin_rate=0.10)
        # price=100, ZD=90 → D_struct = (100-90)/100 = 0.10 → L_max = 1/0.20 = 5.0
        r = calc.compute(price=100.0, zd=90.0, zg=95.0)
        assert r.d_struct == pytest.approx(0.10)
        assert r.l_max == pytest.approx(5.0)

    def test_up_move_phase_uses_zg(self):
        """MOVE↑ 相位 P_neg = ZG。"""
        calc = LeverageCalculator(maint_margin_rate=0.10)
        r = calc.compute(price=100.0, zd=90.0, zg=95.0, phase_is_up_move=True)
        assert r.d_struct == pytest.approx(0.05)
        assert r.l_max == pytest.approx(1.0 / 0.15)

    def test_price_below_p_neg_floors_at_mm(self):
        """价格已破 P_neg：D_struct=0，L_max=1/mm（仅保证金兜底）。"""
        calc = LeverageCalculator(maint_margin_rate=0.05)
        r = calc.compute(price=80.0, zd=90.0, zg=95.0)
        assert r.d_struct == 0.0
        assert r.l_max == pytest.approx(20.0)

    def test_max_quantity_respects_contract_multiplier(self):
        """期货乘数：BZ 1000桶/手——qty 按每手名义 price×1000 折算。"""
        calc = LeverageCalculator(maint_margin_rate=0.10)
        qty = calc.max_quantity(
            equity=1_000_000.0, price=100.0, zd=90.0, zg=95.0,
            notional_frac=1.0, contract_multiplier=1000.0,
        )
        # intent = 1e6/(100×1000) = 10 手；cap = 5×1e6/1e5 = 50 手 → 10
        assert qty == pytest.approx(10.0)

    def test_invalid_mm_rejected(self):
        with pytest.raises(ValueError):
            LeverageCalculator(maint_margin_rate=1.5)


class TestChanlunBridge:
    def _bar(self, ts_ns: int, px: float = 100.0):
        """构造最小 Nautilus Bar。"""
        from nautilus_trader.model.data import Bar, BarType
        from nautilus_trader.model.objects import Price, Quantity

        bt = BarType.from_str("BZ.GLBX-1-MINUTE-LAST-EXTERNAL")
        p = Price(px, 2)
        return Bar(
            bar_type=bt, open=p, high=p, low=p, close=p,
            volume=Quantity.from_int(1), ts_event=ts_ns, ts_init=ts_ns,
        )

    def test_feed_monotonic_and_duplicate(self):
        from trading_system.strategy.signal_bridge import ChanlunBridge, FeedResult

        bridge = ChanlunBridge()
        m = 60_000_000_000  # 1min ns
        assert bridge.feed(self._bar(1 * m)) is FeedResult.ACCEPTED
        assert bridge.feed(self._bar(2 * m)) is FeedResult.ACCEPTED
        # 重复 ts → DUPLICATE 丢弃计数
        assert bridge.feed(self._bar(2 * m)) is FeedResult.DUPLICATE
        assert bridge.dup_count == 1
        assert bridge.bar_count == 2

    def test_feed_backwards_ts_fails_fast(self):
        from trading_system.strategy.signal_bridge import ChanlunBridge

        bridge = ChanlunBridge()
        m = 60_000_000_000
        bridge.feed(self._bar(2 * m))
        with pytest.raises(RuntimeError, match="倒退"):
            bridge.feed(self._bar(1 * m))

    def test_gap_counted_not_filled(self):
        from trading_system.strategy.signal_bridge import ChanlunBridge

        bridge = ChanlunBridge()
        m = 60_000_000_000
        bridge.feed(self._bar(1 * m))
        bridge.feed(self._bar(2 * m))   # 建立 interval
        bridge.feed(self._bar(10 * m))  # gap
        assert bridge.gap_count == 1
        assert bridge.bar_count == 3    # 不填充


class TestLmtOnly:
    def test_market_order_forbidden(self):
        """非 LIMIT 订单 submit 前抛异常（fail-fast，不是过滤）。"""
        from unittest.mock import MagicMock

        from trading_system.execution.lmt_executor import (
            LmtExecutor,
            MarketOrderForbidden,
        )
        from nautilus_trader.model.enums import OrderType

        executor = LmtExecutor(strategy=MagicMock(), instrument=MagicMock())
        fake_mkt = MagicMock()
        fake_mkt.order_type = OrderType.MARKET
        with pytest.raises(MarketOrderForbidden):
            executor.submit(fake_mkt)
