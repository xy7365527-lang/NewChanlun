"""convert.py + b_timeframe.py 单元测试。"""
from __future__ import annotations

from datetime import datetime, timedelta

import numpy as np
import pandas as pd
import pytest

from newchan.convert import bars_to_df
from newchan.b_timeframe import resample_ohlc, SUPPORTED_TF
from newchan.types import Bar


# ── fixtures ─────────────────────────────────────────────


@pytest.fixture()
def sample_bars() -> list[Bar]:
    """20 bars at 1min intervals."""
    bars = []
    base = datetime(2025, 1, 2, 9, 30)
    for i in range(20):
        ts = base + timedelta(minutes=i)
        c = 100.0 + i * 0.5
        bars.append(Bar(ts=ts, open=c - 0.2, high=c + 1.0, low=c - 1.0, close=c, volume=float(100 + i * 10)))
    return bars


@pytest.fixture()
def ohlcv_1m() -> pd.DataFrame:
    """60-bar 1min OHLCV DataFrame."""
    n = 60
    dates = pd.date_range("2025-01-02 09:30", periods=n, freq="1min")
    close = np.linspace(100, 110, n)
    return pd.DataFrame(
        {
            "open": close - 0.1,
            "high": close + 0.5,
            "low": close - 0.5,
            "close": close,
            "volume": np.full(n, 500.0),
        },
        index=dates,
    )


# ══════════ convert.py ══════════


class TestBarsTodf:
    def test_returns_dataframe(self, sample_bars):
        df = bars_to_df(sample_bars)
        assert isinstance(df, pd.DataFrame)

    def test_columns(self, sample_bars):
        df = bars_to_df(sample_bars)
        for col in ["open", "high", "low", "close"]:
            assert col in df.columns

    def test_index_is_datetime(self, sample_bars):
        df = bars_to_df(sample_bars)
        assert isinstance(df.index, pd.DatetimeIndex)

    def test_length(self, sample_bars):
        df = bars_to_df(sample_bars)
        assert len(df) == len(sample_bars)

    def test_sorted_by_time(self, sample_bars):
        # Reverse the input, should still come out sorted
        reversed_bars = list(reversed(sample_bars))
        df = bars_to_df(reversed_bars)
        assert df.index.is_monotonic_increasing

    def test_values_correct(self, sample_bars):
        df = bars_to_df(sample_bars)
        assert abs(df["close"].iloc[0] - sample_bars[0].close) < 1e-10


# ══════════ b_timeframe.py ══════════


class TestResampleOhlc:
    def test_supported_tf_not_empty(self):
        assert len(SUPPORTED_TF) > 0

    def test_resample_5m(self, ohlcv_1m):
        result = resample_ohlc(ohlcv_1m, "5m")
        assert len(result) <= len(ohlcv_1m) // 5 + 1
        assert "open" in result.columns

    def test_resample_aggregation(self, ohlcv_1m):
        result = resample_ohlc(ohlcv_1m, "5m")
        first_5 = ohlcv_1m.iloc[:5]
        first_bar = result.iloc[0]
        assert abs(first_bar["open"] - first_5["open"].iloc[0]) < 1e-10
        assert abs(first_bar["high"] - first_5["high"].max()) < 1e-10
        assert abs(first_bar["low"] - first_5["low"].min()) < 1e-10
        assert abs(first_bar["close"] - first_5["close"].iloc[-1]) < 1e-10

    def test_volume_summed(self, ohlcv_1m):
        result = resample_ohlc(ohlcv_1m, "5m")
        first_5 = ohlcv_1m.iloc[:5]
        assert abs(result.iloc[0]["volume"] - first_5["volume"].sum()) < 1e-10

    def test_unsupported_tf_raises(self, ohlcv_1m):
        with pytest.raises(ValueError, match="不支持"):
            resample_ohlc(ohlcv_1m, "999xyz")

    def test_all_supported_tf(self, ohlcv_1m):
        """Verify each SUPPORTED_TF doesn't error on valid data."""
        for tf in SUPPORTED_TF:
            result = resample_ohlc(ohlcv_1m, tf)
            assert len(result) > 0
