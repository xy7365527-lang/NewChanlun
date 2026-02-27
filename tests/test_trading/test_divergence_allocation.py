"""背驰力度分配测试。"""

from __future__ import annotations

import math

import pytest

from newchan.trading.divergence_allocation import (
    GammaByLayer,
    allocate,
    divergence_ratio,
)


# ── divergence_ratio ──


class TestDivergenceRatio:
    def test_strong_divergence(self) -> None:
        """A_后 远小于 A_前 → d 接近 1。"""
        d = divergence_ratio(100.0, 10.0)
        assert abs(d - 0.9) < 1e-9

    def test_weak_divergence(self) -> None:
        """A_后 接近 A_前 → d 接近 0。"""
        d = divergence_ratio(100.0, 95.0)
        assert abs(d - 0.05) < 1e-9

    def test_no_divergence(self) -> None:
        """A_后 >= A_前 → d = 0（不是负数）。"""
        d = divergence_ratio(100.0, 100.0)
        assert d == 0.0

    def test_no_divergence_exceeds(self) -> None:
        """A_后 > A_前 → d = 0。"""
        d = divergence_ratio(100.0, 120.0)
        assert d == 0.0

    def test_zero_curr(self) -> None:
        """A_后 = 0 → d = 1。"""
        d = divergence_ratio(100.0, 0.0)
        assert d == 1.0

    def test_a_prev_zero_raises(self) -> None:
        with pytest.raises(ValueError, match="a_prev must be > 0"):
            divergence_ratio(0.0, 50.0)

    def test_a_prev_negative_raises(self) -> None:
        with pytest.raises(ValueError, match="a_prev must be > 0"):
            divergence_ratio(-10.0, 50.0)

    def test_a_curr_negative_raises(self) -> None:
        with pytest.raises(ValueError, match="a_curr must be >= 0"):
            divergence_ratio(100.0, -1.0)


# ── allocate ──


class TestAllocate:
    def test_normalization(self) -> None:
        """分配结果的归一化：sum(w_alloc) == W（§八.3）。"""
        total = 0.5
        ratios = [0.8, 0.5, 0.3]
        result = allocate(total, ratios, gamma=1.5)
        assert abs(sum(result) - total) < 1e-9

    def test_normalization_gamma_2(self) -> None:
        total = 0.75
        ratios = [0.9, 0.6, 0.2, 0.1]
        result = allocate(total, ratios, gamma=2.0)
        assert abs(sum(result) - total) < 1e-9

    def test_single_target(self) -> None:
        """单个标的拿全部仓位。"""
        result = allocate(0.5, [0.7], gamma=2.0)
        assert len(result) == 1
        assert abs(result[0] - 0.5) < 1e-9

    def test_higher_divergence_gets_more(self) -> None:
        """背驰力度更大的标的分配更多仓位。"""
        ratios = [0.9, 0.3]
        result = allocate(1.0, ratios, gamma=1.5)
        assert result[0] > result[1]

    def test_gamma_increases_concentration(self) -> None:
        """gamma 越大，分配越集中到高背驰标的。"""
        ratios = [0.9, 0.3]
        result_low = allocate(1.0, ratios, gamma=1.0)
        result_high = allocate(1.0, ratios, gamma=3.0)
        # 高 gamma 下第一个的份额更大
        assert result_high[0] / sum(result_high) > result_low[0] / sum(result_low)

    def test_gamma_1_linear(self) -> None:
        """gamma=1 时线性分配。"""
        ratios = [0.6, 0.4]
        result = allocate(1.0, ratios, gamma=1.0)
        assert abs(result[0] - 0.6) < 1e-9
        assert abs(result[1] - 0.4) < 1e-9

    def test_equal_divergence_equal_allocation(self) -> None:
        """相同背驰力度 → 平均分配。"""
        ratios = [0.5, 0.5, 0.5]
        result = allocate(0.9, ratios, gamma=2.0)
        for w in result:
            assert abs(w - 0.3) < 1e-9

    def test_empty_raises(self) -> None:
        with pytest.raises(ValueError, match="must not be empty"):
            allocate(0.5, [], gamma=1.0)

    def test_gamma_below_1_raises(self) -> None:
        with pytest.raises(ValueError, match="gamma must be >= 1"):
            allocate(0.5, [0.5], gamma=0.5)

    def test_all_zero_ratios_raises(self) -> None:
        with pytest.raises(ValueError, match="all divergence ratios are 0"):
            allocate(0.5, [0.0, 0.0], gamma=1.0)


# ── GammaByLayer ──


class TestGammaByLayer:
    def test_defaults(self) -> None:
        g = GammaByLayer()
        assert g.l1 == 1.5
        assert g.l2 == 2.0
        assert g.l3 == 1.0

    def test_custom(self) -> None:
        g = GammaByLayer(l1=2.0, l2=3.0, l3=1.5)
        assert g.l1 == 2.0

    def test_invalid_raises(self) -> None:
        with pytest.raises(ValueError, match="l1 must be >= 1"):
            GammaByLayer(l1=0.5)
        with pytest.raises(ValueError, match="l2 must be >= 1"):
            GammaByLayer(l2=0.0)

    def test_immutable(self) -> None:
        g = GammaByLayer()
        with pytest.raises(AttributeError):
            g.l1 = 3.0  # type: ignore[misc]
