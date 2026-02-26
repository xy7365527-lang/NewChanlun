"""成本模型单元测试 — 滑点模型 + 手续费模型。

覆盖：
- FixedSlippage / PercentSlippage / MarketImpactSlippage 的 Protocol 兼容性与计算正确性
- FixedRateCommission / TieredCommission 的计算正确性与边界条件
- 所有模型满足 frozen dataclass 不可变约束
"""

from __future__ import annotations

import math

import pytest

from newchan.cost.config import CommissionModel, CostConfig, SlippageModel
from newchan.cost.slippage import (
    FixedSlippage,
    MarketImpactSlippage,
    PercentSlippage,
)
from newchan.cost.commission import (
    FixedRateCommission,
    MARKET_PRESETS,
    TieredCommission,
    TIERED_PRESETS,
)


# ── FixedSlippage ─────────────────────────────────────────────────


class TestFixedSlippage:
    def test_satisfies_protocol(self) -> None:
        m = FixedSlippage(slippage_points=1.0)
        assert isinstance(m, SlippageModel)

    def test_buy_increases_price(self) -> None:
        m = FixedSlippage(slippage_points=0.5)
        assert m.apply(100.0, "buy") == 100.5

    def test_sell_decreases_price(self) -> None:
        m = FixedSlippage(slippage_points=0.5)
        assert m.apply(100.0, "sell") == 99.5

    def test_zero_slippage(self) -> None:
        m = FixedSlippage(slippage_points=0.0)
        assert m.apply(50.0, "buy") == 50.0
        assert m.apply(50.0, "sell") == 50.0

    def test_frozen(self) -> None:
        m = FixedSlippage(slippage_points=1.0)
        with pytest.raises(AttributeError):
            m.slippage_points = 2.0  # type: ignore[misc]

    def test_apply_slippage_still_works(self) -> None:
        """丰富接口 apply_slippage 保持可用。"""
        m = FixedSlippage(slippage_points=0.5)
        assert m.apply_slippage(100.0, "buy") == 100.5


# ── PercentSlippage ───────────────────────────────────────────────


class TestPercentSlippage:
    def test_satisfies_protocol(self) -> None:
        m = PercentSlippage(slippage_pct=0.001)
        assert isinstance(m, SlippageModel)

    def test_buy_increases_price(self) -> None:
        m = PercentSlippage(slippage_pct=0.001)
        assert m.apply(1000.0, "buy") == pytest.approx(1001.0)

    def test_sell_decreases_price(self) -> None:
        m = PercentSlippage(slippage_pct=0.001)
        assert m.apply(1000.0, "sell") == pytest.approx(999.0)

    def test_zero_pct(self) -> None:
        m = PercentSlippage(slippage_pct=0.0)
        assert m.apply(100.0, "buy") == 100.0

    def test_frozen(self) -> None:
        m = PercentSlippage(slippage_pct=0.001)
        with pytest.raises(AttributeError):
            m.slippage_pct = 0.01  # type: ignore[misc]


# ── MarketImpactSlippage ──────────────────────────────────────────


class TestMarketImpactSlippage:
    def test_satisfies_protocol(self) -> None:
        m = MarketImpactSlippage()
        assert isinstance(m, SlippageModel)

    def test_no_volume_returns_original_price(self) -> None:
        """Protocol apply() 无 volume 参数时，无法计算冲击，返回原价。"""
        m = MarketImpactSlippage()
        assert m.apply(100.0, "buy") == 100.0
        assert m.apply(100.0, "sell") == 100.0

    def test_below_threshold_no_impact(self) -> None:
        m = MarketImpactSlippage(threshold_pct=0.01, impact_coeff=0.1)
        # participation = 50 / 10000 = 0.005 < 0.01
        result = m.apply_slippage(100.0, "buy", volume=50.0, avg_daily_volume=10000.0)
        assert result == 100.0

    def test_above_threshold_buy_impact(self) -> None:
        m = MarketImpactSlippage(threshold_pct=0.01, impact_coeff=0.1)
        # participation = 500 / 10000 = 0.05 > 0.01
        # impact = 0.1 * sqrt(0.05) ≈ 0.02236
        result = m.apply_slippage(100.0, "buy", volume=500.0, avg_daily_volume=10000.0)
        expected = 100.0 * (1.0 + 0.1 * math.sqrt(0.05))
        assert result == pytest.approx(expected)

    def test_above_threshold_sell_impact(self) -> None:
        m = MarketImpactSlippage(threshold_pct=0.01, impact_coeff=0.1)
        result = m.apply_slippage(100.0, "sell", volume=500.0, avg_daily_volume=10000.0)
        expected = 100.0 * (1.0 - 0.1 * math.sqrt(0.05))
        assert result == pytest.approx(expected)

    def test_zero_avg_volume_returns_original(self) -> None:
        m = MarketImpactSlippage()
        assert m.apply_slippage(100.0, "buy", volume=100.0, avg_daily_volume=0.0) == 100.0

    def test_frozen(self) -> None:
        m = MarketImpactSlippage()
        with pytest.raises(AttributeError):
            m.threshold_pct = 0.5  # type: ignore[misc]


# ── FixedRateCommission ───────────────────────────────────────────


class TestFixedRateCommission:
    def test_satisfies_protocol(self) -> None:
        m = FixedRateCommission(rate=0.001)
        assert isinstance(m, CommissionModel)

    def test_calculate_protocol(self) -> None:
        m = FixedRateCommission(rate=0.001)
        # price=100, quantity=200 -> trade_value=20000 -> 20000*0.001=20
        assert m.calculate(100.0, 200.0) == pytest.approx(20.0)

    def test_calculate_commission_direct(self) -> None:
        m = FixedRateCommission(rate=0.0003)
        assert m.calculate_commission(100_000.0) == pytest.approx(30.0)

    def test_default_rate_is_a_stock(self) -> None:
        m = FixedRateCommission()
        assert m.rate == MARKET_PRESETS["a_stock"]

    def test_negative_trade_value_raises(self) -> None:
        m = FixedRateCommission(rate=0.001)
        with pytest.raises(ValueError, match="trade_value must be >= 0"):
            m.calculate_commission(-100.0)

    def test_zero_trade_value(self) -> None:
        m = FixedRateCommission(rate=0.001)
        assert m.calculate_commission(0.0) == 0.0

    def test_frozen(self) -> None:
        m = FixedRateCommission(rate=0.001)
        with pytest.raises(AttributeError):
            m.rate = 0.01  # type: ignore[misc]


# ── TieredCommission ──────────────────────────────────────────────


class TestTieredCommission:
    def test_satisfies_protocol(self) -> None:
        m = TieredCommission()
        assert isinstance(m, CommissionModel)

    def test_first_tier(self) -> None:
        tiers = ((1_000_000, 0.0003), (float("inf"), 0.0001))
        m = TieredCommission(tiers=tiers, monthly_volume=0.0)
        # volume=0 < 1M -> rate=0.0003
        assert m.calculate_commission(100_000.0, monthly_volume=0.0) == pytest.approx(30.0)

    def test_second_tier(self) -> None:
        tiers = ((1_000_000, 0.0003), (float("inf"), 0.0001))
        m = TieredCommission(tiers=tiers, monthly_volume=2_000_000.0)
        # volume=2M >= 1M -> rate=0.0001
        assert m.calculate_commission(100_000.0, monthly_volume=2_000_000.0) == pytest.approx(10.0)

    def test_calculate_protocol_uses_instance_volume(self) -> None:
        tiers = ((1_000_000, 0.0003), (float("inf"), 0.0001))
        m = TieredCommission(tiers=tiers, monthly_volume=2_000_000.0)
        # Protocol: calculate(price, quantity) uses self.monthly_volume
        assert m.calculate(100.0, 1000.0) == pytest.approx(10.0)

    def test_empty_tiers_raises(self) -> None:
        with pytest.raises(ValueError, match="tiers must not be empty"):
            TieredCommission(tiers=())

    def test_non_ascending_tiers_raises(self) -> None:
        with pytest.raises(ValueError, match="ascending"):
            TieredCommission(tiers=((100, 0.001), (50, 0.0005)))

    def test_negative_rate_raises(self) -> None:
        with pytest.raises(ValueError, match="rate must be >= 0"):
            TieredCommission(tiers=((100, -0.001),))

    def test_negative_trade_value_raises(self) -> None:
        m = TieredCommission()
        with pytest.raises(ValueError, match="trade_value must be >= 0"):
            m.calculate_commission(-1.0)

    def test_frozen(self) -> None:
        m = TieredCommission()
        with pytest.raises(AttributeError):
            m.monthly_volume = 999.0  # type: ignore[misc]

    def test_default_tiers_are_a_stock(self) -> None:
        m = TieredCommission()
        assert len(m.tiers) == len(TIERED_PRESETS["a_stock"])


# ── CostConfig 组装 ──────────────────────────────────────────────


class TestCostConfigAssembly:
    def test_with_real_models(self) -> None:
        cfg = CostConfig(
            slippage_model=FixedSlippage(slippage_points=0.5),
            commission_model=FixedRateCommission(rate=0.001),
            quantity=100.0,
        )
        assert cfg.slippage_model is not None
        assert cfg.commission_model is not None
        assert cfg.slippage_model.apply(100.0, "buy") == 100.5
        assert cfg.commission_model.calculate(100.0, 100.0) == pytest.approx(10.0)

    def test_frozen(self) -> None:
        cfg = CostConfig()
        with pytest.raises(AttributeError):
            cfg.quantity = 999.0  # type: ignore[misc]
