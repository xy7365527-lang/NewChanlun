"""手续费模型测试。"""

from __future__ import annotations

import math

import pytest

from newchan.cost.commission import (
    FixedRateCommission,
    TieredCommission,
    MARKET_PRESETS,
    TIERED_PRESETS,
)
from newchan.cost.config import CommissionModel


# ── FixedRateCommission ──


class TestFixedRateCommission:
    def test_default_rate_is_a_stock(self) -> None:
        model = FixedRateCommission()
        assert model.rate == MARKET_PRESETS["a_stock"]

    def test_basic_calculation(self) -> None:
        model = FixedRateCommission(rate=0.0003)
        result = model.calculate_commission(100_000)
        assert result == pytest.approx(30.0)

    def test_custom_rate(self) -> None:
        model = FixedRateCommission(rate=0.001)
        assert model.calculate_commission(50_000) == pytest.approx(50.0)

    def test_zero_trade_value(self) -> None:
        model = FixedRateCommission(rate=0.0003)
        assert model.calculate_commission(0.0) == 0.0

    def test_monthly_volume_ignored(self) -> None:
        model = FixedRateCommission(rate=0.0003)
        a = model.calculate_commission(100_000, monthly_volume=None)
        b = model.calculate_commission(100_000, monthly_volume=999_999_999)
        assert a == b

    def test_negative_trade_value_raises(self) -> None:
        model = FixedRateCommission(rate=0.0003)
        with pytest.raises(ValueError, match="trade_value must be >= 0"):
            model.calculate_commission(-1.0)

    def test_market_presets(self) -> None:
        for market, rate in MARKET_PRESETS.items():
            model = FixedRateCommission(rate=rate)
            result = model.calculate_commission(1_000_000)
            assert result == pytest.approx(1_000_000 * rate), f"failed for {market}"


# ── TieredCommission ──


class TestTieredCommission:
    def test_default_tiers_are_a_stock(self) -> None:
        model = TieredCommission()
        assert list(model.tiers) == TIERED_PRESETS["a_stock"]

    def test_first_tier(self) -> None:
        """月交易量 < 第一档阈值 → 用第一档费率。"""
        model = TieredCommission(tiers=(
            (1_000_000, 0.0003),
            (10_000_000, 0.00025),
            (float("inf"), 0.0002),
        ))
        result = model.calculate_commission(100_000, monthly_volume=500_000)
        assert result == pytest.approx(100_000 * 0.0003)

    def test_second_tier(self) -> None:
        """月交易量 >= 第一档阈值但 < 第二档 → 用第二档费率。"""
        model = TieredCommission(tiers=(
            (1_000_000, 0.0003),
            (10_000_000, 0.00025),
            (float("inf"), 0.0002),
        ))
        result = model.calculate_commission(100_000, monthly_volume=5_000_000)
        assert result == pytest.approx(100_000 * 0.00025)

    def test_highest_tier(self) -> None:
        """月交易量 >= 最后一个有限阈值 → 用最高档费率。"""
        model = TieredCommission(tiers=(
            (1_000_000, 0.0003),
            (10_000_000, 0.00025),
            (float("inf"), 0.0002),
        ))
        result = model.calculate_commission(100_000, monthly_volume=50_000_000)
        assert result == pytest.approx(100_000 * 0.0002)

    def test_boundary_exact_threshold(self) -> None:
        """月交易量恰好等于阈值 → 进入下一档。"""
        model = TieredCommission(tiers=(
            (1_000_000, 0.0003),
            (float("inf"), 0.0002),
        ))
        # volume == 1_000_000 → NOT < 1_000_000 → 进入下一档
        result = model.calculate_commission(100_000, monthly_volume=1_000_000)
        assert result == pytest.approx(100_000 * 0.0002)

    def test_zero_monthly_volume(self) -> None:
        """monthly_volume=None 视为 0 → 用第一档。"""
        model = TieredCommission(tiers=(
            (1_000_000, 0.0003),
            (float("inf"), 0.0002),
        ))
        result = model.calculate_commission(100_000, monthly_volume=None)
        assert result == pytest.approx(100_000 * 0.0003)

    def test_zero_trade_value(self) -> None:
        model = TieredCommission()
        assert model.calculate_commission(0.0, monthly_volume=500_000) == 0.0

    def test_negative_trade_value_raises(self) -> None:
        model = TieredCommission()
        with pytest.raises(ValueError, match="trade_value must be >= 0"):
            model.calculate_commission(-1.0)

    def test_empty_tiers_raises(self) -> None:
        with pytest.raises(ValueError, match="tiers must not be empty"):
            TieredCommission(tiers=())

    def test_unsorted_tiers_raises(self) -> None:
        with pytest.raises(ValueError, match="ascending threshold order"):
            TieredCommission(tiers=(
                (10_000_000, 0.0002),
                (1_000_000, 0.0003),
            ))

    def test_negative_rate_raises(self) -> None:
        with pytest.raises(ValueError, match="rate must be >= 0"):
            TieredCommission(tiers=((1_000_000, -0.001),))

    def test_tiered_presets(self) -> None:
        for market, tiers in TIERED_PRESETS.items():
            model = TieredCommission(tiers=tuple(tiers))
            result = model.calculate_commission(100_000, monthly_volume=0)
            assert result == pytest.approx(100_000 * tiers[0][1]), f"failed for {market}"


# ── Protocol 兼容性 ──


class TestProtocolConformance:
    def test_fixed_rate_satisfies_protocol(self) -> None:
        model = FixedRateCommission(rate=0.0003)
        assert isinstance(model, CommissionModel)

    def test_tiered_satisfies_protocol(self) -> None:
        model = TieredCommission()
        assert isinstance(model, CommissionModel)

    def test_fixed_rate_calculate(self) -> None:
        """calculate(price, quantity) 应等于 calculate_commission(price * quantity)。"""
        model = FixedRateCommission(rate=0.0003)
        assert model.calculate(100.0, 1000.0) == pytest.approx(
            model.calculate_commission(100.0 * 1000.0),
        )

    def test_tiered_calculate_uses_instance_volume(self) -> None:
        """calculate() 使用实例的 monthly_volume。"""
        model = TieredCommission(
            tiers=((1_000_000, 0.0003), (float("inf"), 0.0002)),
            monthly_volume=5_000_000,
        )
        # monthly_volume=5M >= 1M → 第二档 0.0002
        assert model.calculate(100.0, 1000.0) == pytest.approx(
            100.0 * 1000.0 * 0.0002,
        )
