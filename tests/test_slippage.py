"""滑点模型测试。"""

from __future__ import annotations

import math

import pytest

from newchan.cost.slippage import (
    FixedSlippage,
    MarketImpactSlippage,
    PercentSlippage,
)


# -- FixedSlippage --

class TestFixedSlippage:
    def test_buy_adds_points(self) -> None:
        model = FixedSlippage(slippage_points=0.5)
        assert model.apply_slippage(100.0, "buy") == 100.5

    def test_sell_subtracts_points(self) -> None:
        model = FixedSlippage(slippage_points=0.5)
        assert model.apply_slippage(100.0, "sell") == 99.5

    def test_zero_slippage(self) -> None:
        model = FixedSlippage(slippage_points=0.0)
        assert model.apply_slippage(100.0, "buy") == 100.0
        assert model.apply_slippage(100.0, "sell") == 100.0

    def test_buy_sell_symmetry(self) -> None:
        model = FixedSlippage(slippage_points=2.0)
        buy_diff = model.apply_slippage(100.0, "buy") - 100.0
        sell_diff = 100.0 - model.apply_slippage(100.0, "sell")
        assert buy_diff == pytest.approx(sell_diff)


# -- PercentSlippage --

class TestPercentSlippage:
    def test_buy_increases_price(self) -> None:
        model = PercentSlippage(slippage_pct=0.001)
        assert model.apply_slippage(1000.0, "buy") == pytest.approx(1001.0)

    def test_sell_decreases_price(self) -> None:
        model = PercentSlippage(slippage_pct=0.001)
        assert model.apply_slippage(1000.0, "sell") == pytest.approx(999.0)

    def test_zero_slippage(self) -> None:
        model = PercentSlippage(slippage_pct=0.0)
        assert model.apply_slippage(500.0, "buy") == 500.0
        assert model.apply_slippage(500.0, "sell") == 500.0

    def test_buy_sell_direction(self) -> None:
        model = PercentSlippage(slippage_pct=0.01)
        assert model.apply_slippage(100.0, "buy") > 100.0
        assert model.apply_slippage(100.0, "sell") < 100.0


# -- MarketImpactSlippage --

class TestMarketImpactSlippage:
    def test_small_participation_has_small_nonzero_impact(self) -> None:
        # #920：旧版在参与率 < 1% 时冲击恒 0——与平方根律相反。0.5% 参与率必须非零。
        model = MarketImpactSlippage(y=0.9, sigma_daily=0.02)
        result = model.apply_slippage(100.0, "buy", volume=500.0, avg_daily_volume=100_000.0)
        expected = 100.0 * (1.0 + 0.9 * 0.02 * math.sqrt(0.005))
        assert result == pytest.approx(expected)
        assert result > 100.0

    def test_buy_impact_sqrt_law(self) -> None:
        model = MarketImpactSlippage(y=0.9, sigma_daily=0.02)
        result = model.apply_slippage(100.0, "buy", volume=5000.0, avg_daily_volume=100_000.0)
        expected = 100.0 * (1.0 + 0.9 * 0.02 * math.sqrt(0.05))
        assert result == pytest.approx(expected)

    def test_sell_impact_sqrt_law(self) -> None:
        model = MarketImpactSlippage(y=0.9, sigma_daily=0.02)
        result = model.apply_slippage(100.0, "sell", volume=5000.0, avg_daily_volume=100_000.0)
        expected = 100.0 * (1.0 - 0.9 * 0.02 * math.sqrt(0.05))
        assert result == pytest.approx(expected)

    def test_missing_volume_no_impact(self) -> None:
        model = MarketImpactSlippage(y=0.9, sigma_daily=0.02)
        assert model.apply_slippage(100.0, "buy", volume=None, avg_daily_volume=100_000.0) == 100.0

    def test_missing_adv_no_impact(self) -> None:
        model = MarketImpactSlippage(y=0.9, sigma_daily=0.02)
        assert model.apply_slippage(100.0, "buy", volume=5000.0, avg_daily_volume=None) == 100.0

    def test_zero_adv_no_impact(self) -> None:
        model = MarketImpactSlippage(y=0.9, sigma_daily=0.02)
        assert model.apply_slippage(100.0, "buy", volume=5000.0, avg_daily_volume=0.0) == 100.0

    def test_zero_sigma_daily_is_honest_no_impact(self) -> None:
        # sigma_daily 默认 0 = 无波动率信息 ⟹ 不臆造冲击。
        model = MarketImpactSlippage()
        assert model.apply_slippage(100.0, "buy", volume=5000.0, avg_daily_volume=100_000.0) == 100.0

    def test_buy_sell_symmetry(self) -> None:
        model = MarketImpactSlippage(y=0.9, sigma_daily=0.02)
        buy = model.apply_slippage(100.0, "buy", volume=5000.0, avg_daily_volume=100_000.0)
        sell = model.apply_slippage(100.0, "sell", volume=5000.0, avg_daily_volume=100_000.0)
        assert buy > 100.0
        assert sell < 100.0
        assert (buy - 100.0) == pytest.approx(100.0 - sell)
