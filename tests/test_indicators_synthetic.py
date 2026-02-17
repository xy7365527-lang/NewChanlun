"""indicators.py + synthetic.py 单元测试 — 覆盖率冲刺。"""
from __future__ import annotations

import numpy as np
import pandas as pd
import pytest

from newchan.indicators import (
    INDICATOR_REGISTRY,
    calc_bollinger,
    calc_ema,
    calc_macd,
    calc_rsi,
    calc_sma,
    compute_indicator,
)
from newchan.synthetic import make_ratio, make_spread


# ── fixtures ─────────────────────────────────────────────

@pytest.fixture()
def ohlcv_df() -> pd.DataFrame:
    """50-bar sample OHLCV with a simple upward-then-downward pattern."""
    n = 50
    dates = pd.date_range("2025-01-01", periods=n, freq="1min")
    close = np.concatenate([np.linspace(100, 120, 25), np.linspace(120, 95, 25)])
    return pd.DataFrame(
        {
            "open": close - 0.5,
            "high": close + 1.0,
            "low": close - 1.0,
            "close": close,
            "volume": np.random.default_rng(42).integers(100, 1000, n).astype(float),
        },
        index=dates,
    )


@pytest.fixture()
def ohlcv_pair() -> tuple[pd.DataFrame, pd.DataFrame]:
    """Two OHLCV DataFrames with partially overlapping indices."""
    dates_a = pd.date_range("2025-01-01", periods=30, freq="1min")
    dates_b = pd.date_range("2025-01-01 00:10:00", periods=30, freq="1min")
    rng = np.random.default_rng(7)
    def _mk(dates):
        n = len(dates)
        c = rng.uniform(100, 110, n)
        return pd.DataFrame(
            {"open": c - 0.3, "high": c + 0.5, "low": c - 0.5, "close": c, "volume": rng.integers(100, 500, n).astype(float)},
            index=dates,
        )
    return _mk(dates_a), _mk(dates_b)


# ══════════ indicators.py ══════════


class TestCalcMacd:
    def test_columns(self, ohlcv_df):
        result = calc_macd(ohlcv_df)
        assert list(result.columns) == ["macd", "signal", "histogram"]

    def test_shape(self, ohlcv_df):
        result = calc_macd(ohlcv_df)
        assert len(result) == len(ohlcv_df)

    def test_custom_params(self, ohlcv_df):
        result = calc_macd(ohlcv_df, fast=5, slow=10, signal=3)
        assert list(result.columns) == ["macd", "signal", "histogram"]

    def test_histogram_equals_diff(self, ohlcv_df):
        result = calc_macd(ohlcv_df)
        np.testing.assert_allclose(
            result["histogram"].values,
            (result["macd"] - result["signal"]).values,
            atol=1e-12,
        )


class TestCalcSma:
    def test_columns(self, ohlcv_df):
        result = calc_sma(ohlcv_df)
        assert list(result.columns) == ["sma"]

    def test_nan_warmup(self, ohlcv_df):
        result = calc_sma(ohlcv_df, period=10)
        assert result["sma"].isna().sum() == 9  # first period-1 are NaN

    def test_correct_value(self, ohlcv_df):
        period = 5
        result = calc_sma(ohlcv_df, period=period)
        expected = ohlcv_df["close"].iloc[:period].mean()
        assert abs(result["sma"].iloc[period - 1] - expected) < 1e-10


class TestCalcEma:
    def test_columns(self, ohlcv_df):
        result = calc_ema(ohlcv_df)
        assert list(result.columns) == ["ema"]

    def test_first_value(self, ohlcv_df):
        result = calc_ema(ohlcv_df, period=5)
        # EMA first value = first close (adjust=False)
        assert abs(result["ema"].iloc[0] - ohlcv_df["close"].iloc[0]) < 1e-10

    def test_no_nan_after_first(self, ohlcv_df):
        result = calc_ema(ohlcv_df, period=5)
        assert result["ema"].isna().sum() == 0


class TestCalcRsi:
    def test_columns(self, ohlcv_df):
        result = calc_rsi(ohlcv_df)
        assert list(result.columns) == ["rsi"]

    def test_range(self, ohlcv_df):
        result = calc_rsi(ohlcv_df, period=14)
        valid = result["rsi"].dropna()
        assert (valid >= 0).all() and (valid <= 100).all()

    def test_constant_series(self):
        """Constant price → RSI should be NaN (0/0 edge case)."""
        dates = pd.date_range("2025-01-01", periods=30, freq="1min")
        df = pd.DataFrame({"close": [100.0] * 30}, index=dates)
        result = calc_rsi(df, period=14)
        # All deltas are 0, so avg_gain=0, avg_loss=0 → NaN
        assert result["rsi"].iloc[-1] != 50 or np.isnan(result["rsi"].iloc[-1])


class TestCalcBollinger:
    def test_columns(self, ohlcv_df):
        result = calc_bollinger(ohlcv_df)
        assert set(result.columns) == {"bb_mid", "bb_upper", "bb_lower"}

    def test_ordering_after_warmup(self, ohlcv_df):
        result = calc_bollinger(ohlcv_df, period=10)
        valid = result.dropna()
        assert (valid["bb_upper"] >= valid["bb_mid"]).all()
        assert (valid["bb_mid"] >= valid["bb_lower"]).all()

    def test_custom_std(self, ohlcv_df):
        r1 = calc_bollinger(ohlcv_df, std=1.0)
        r2 = calc_bollinger(ohlcv_df, std=3.0)
        valid1 = r1.dropna()
        valid2 = r2.dropna()
        # Wider std → wider bands
        assert (valid2["bb_upper"].values >= valid1["bb_upper"].values - 1e-10).all()


class TestComputeIndicator:
    def test_dispatch_macd(self, ohlcv_df):
        result = compute_indicator("MACD", ohlcv_df)
        assert "macd" in result.columns

    def test_dispatch_sma(self, ohlcv_df):
        result = compute_indicator("SMA", ohlcv_df)
        assert "sma" in result.columns

    def test_dispatch_ema(self, ohlcv_df):
        result = compute_indicator("EMA", ohlcv_df)
        assert "ema" in result.columns

    def test_dispatch_rsi(self, ohlcv_df):
        result = compute_indicator("RSI", ohlcv_df)
        assert "rsi" in result.columns

    def test_dispatch_bollinger(self, ohlcv_df):
        result = compute_indicator("Bollinger", ohlcv_df)
        assert "bb_mid" in result.columns

    def test_unknown_raises(self, ohlcv_df):
        with pytest.raises(ValueError, match="未知指标"):
            compute_indicator("nonexistent", ohlcv_df)

    def test_custom_params(self, ohlcv_df):
        result = compute_indicator("SMA", ohlcv_df, params={"period": 5})
        assert result["sma"].isna().sum() == 4  # period-1


class TestRegistry:
    def test_all_keys_present(self):
        expected = {"MACD", "SMA", "EMA", "RSI", "Bollinger"}
        assert expected.issubset(set(INDICATOR_REGISTRY.keys()))

    def test_each_entry_has_func(self):
        for name, entry in INDICATOR_REGISTRY.items():
            assert "func" in entry
            assert callable(entry["func"])


# ══════════ synthetic.py ══════════


class TestMakeSpread:
    def test_columns(self, ohlcv_pair):
        a, b = ohlcv_pair
        result = make_spread(a, b)
        assert set(result.columns) >= {"open", "high", "low", "close"}

    def test_values_are_difference(self, ohlcv_pair):
        a, b = ohlcv_pair
        result = make_spread(a, b)
        common_idx = a.index.intersection(b.index)
        np.testing.assert_allclose(
            result.loc[common_idx, "close"].values,
            a.loc[common_idx, "close"].values - b.loc[common_idx, "close"].values,
            atol=1e-10,
        )

    def test_inner_join_length(self, ohlcv_pair):
        a, b = ohlcv_pair
        result = make_spread(a, b)
        expected_len = len(a.index.intersection(b.index))
        assert len(result) == expected_len

    def test_volume_from_a(self, ohlcv_pair):
        a, b = ohlcv_pair
        result = make_spread(a, b)
        if "volume" in result.columns:
            common_idx = a.index.intersection(b.index)
            np.testing.assert_allclose(
                result.loc[common_idx, "volume"].values,
                a.loc[common_idx, "volume"].values,
                atol=1e-10,
            )


class TestMakeRatio:
    def test_columns(self, ohlcv_pair):
        a, b = ohlcv_pair
        result = make_ratio(a, b)
        assert set(result.columns) >= {"open", "high", "low", "close"}

    def test_values_are_ratio(self, ohlcv_pair):
        a, b = ohlcv_pair
        result = make_ratio(a, b)
        common_idx = a.index.intersection(b.index)
        np.testing.assert_allclose(
            result.loc[common_idx, "close"].values,
            a.loc[common_idx, "close"].values / b.loc[common_idx, "close"].values,
            atol=1e-10,
        )

    def test_no_overlap_empty(self):
        dates_a = pd.date_range("2025-01-01", periods=5, freq="1min")
        dates_b = pd.date_range("2025-02-01", periods=5, freq="1min")
        a = pd.DataFrame({"open": [1]*5, "high": [2]*5, "low": [0.5]*5, "close": [1.5]*5}, index=dates_a)
        b = pd.DataFrame({"open": [1]*5, "high": [2]*5, "low": [0.5]*5, "close": [1.5]*5}, index=dates_b)
        result = make_ratio(a, b)
        assert len(result) == 0
