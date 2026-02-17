"""data_databento.py 单元测试 — mock 边界隔离，不调用真实 API。"""
from __future__ import annotations

from unittest.mock import MagicMock, patch
from datetime import datetime, timedelta

import numpy as np
import pandas as pd
import pytest

from newchan.data_databento import (
    SYMBOL_CATALOG,
    DEFAULT_SYMBOLS,
    _FUTURES_MAP,
    search_symbols,
    _resolve,
    _resample_ohlcv,
    _is_futures,
)


# ══════════ search_symbols ══════════


class TestSearchSymbols:
    def test_empty_query(self):
        assert search_symbols("") == []

    def test_whitespace_query(self):
        assert search_symbols("   ") == []

    def test_finds_known_symbol(self):
        if SYMBOL_CATALOG:
            first = SYMBOL_CATALOG[0]
            results = search_symbols(first["symbol"])
            assert len(results) >= 1
            assert any(r["symbol"] == first["symbol"] for r in results)

    def test_case_insensitive(self):
        if SYMBOL_CATALOG:
            first = SYMBOL_CATALOG[0]
            lower = first["symbol"].lower()
            results = search_symbols(lower)
            assert len(results) >= 1

    def test_no_match(self):
        results = search_symbols("ZZZZNONEXISTENT999")
        assert results == []


# ══════════ _resolve ══════════


class TestResolve:
    def test_stock_symbol(self):
        dataset, db_symbol, stype_in = _resolve("AAPL")
        assert dataset == "XNAS.ITCH"
        assert db_symbol == "AAPL"
        assert stype_in == "raw_symbol"

    def test_futures_symbol(self):
        if _FUTURES_MAP:
            sym = next(iter(_FUTURES_MAP))
            dataset, db_symbol, stype_in = _resolve(sym)
            assert dataset == "GLBX.MDP3"
            assert stype_in == "continuous"
            assert db_symbol == _FUTURES_MAP[sym]

    def test_case_insensitive(self):
        dataset, db_symbol, stype_in = _resolve("aapl")
        assert db_symbol == "AAPL"


# ══════════ _is_futures ══════════


class TestIsFutures:
    def test_stock(self):
        assert not _is_futures("AAPL")

    def test_futures(self):
        if _FUTURES_MAP:
            sym = next(iter(_FUTURES_MAP))
            assert _is_futures(sym)


# ══════════ _resample_ohlcv ══════════


class TestResampleOhlcv:
    @pytest.fixture()
    def minute_df(self) -> pd.DataFrame:
        n = 60
        dates = pd.date_range("2025-01-01 09:30", periods=n, freq="1min")
        rng = np.random.default_rng(42)
        close = 100 + rng.standard_normal(n).cumsum()
        return pd.DataFrame(
            {
                "open": close - 0.1,
                "high": close + 0.5,
                "low": close - 0.5,
                "close": close,
                "volume": rng.integers(100, 1000, n).astype(float),
            },
            index=dates,
        )

    def test_resample_5min(self, minute_df):
        result = _resample_ohlcv(minute_df, "5min")
        assert len(result) <= len(minute_df) // 5 + 1
        assert "open" in result.columns
        assert "volume" in result.columns

    def test_resample_15min(self, minute_df):
        result = _resample_ohlcv(minute_df, "15min")
        assert len(result) <= len(minute_df) // 15 + 1

    def test_resample_unknown_passthrough(self, minute_df):
        result = _resample_ohlcv(minute_df, "1hour")
        assert len(result) == len(minute_df)  # no resampling

    def test_ohlcv_aggregation(self, minute_df):
        result = _resample_ohlcv(minute_df, "5min")
        # First 5min bar should have: open=first, high=max, low=min, close=last
        first_5 = minute_df.iloc[:5]
        first_bar = result.iloc[0]
        assert abs(first_bar["open"] - first_5["open"].iloc[0]) < 1e-10
        assert abs(first_bar["high"] - first_5["high"].max()) < 1e-10
        assert abs(first_bar["low"] - first_5["low"].min()) < 1e-10
        assert abs(first_bar["close"] - first_5["close"].iloc[-1]) < 1e-10
        assert abs(first_bar["volume"] - first_5["volume"].sum()) < 1e-10


# ══════════ fetch_ohlcv (mocked) ══════════


class TestFetchOhlcv:
    def _make_mock_response(self, n=10):
        dates = pd.date_range("2025-01-02 09:30", periods=n, freq="1min", tz="UTC")
        df = pd.DataFrame(
            {
                "open": np.linspace(100, 105, n),
                "high": np.linspace(101, 106, n),
                "low": np.linspace(99, 104, n),
                "close": np.linspace(100.5, 105.5, n),
                "volume": [1000] * n,
                "rtype": [1] * n,
                "publisher_id": [1] * n,
            },
            index=dates,
        )
        mock_data = MagicMock()
        mock_data.to_df.return_value = df
        return mock_data

    @patch("newchan.data_databento._get_client")
    def test_fetch_returns_ohlcv(self, mock_get_client):
        mock_client = MagicMock()
        mock_client.timeseries.get_range.return_value = self._make_mock_response()
        mock_get_client.return_value = mock_client

        from newchan.data_databento import fetch_ohlcv
        result = fetch_ohlcv("AAPL", interval="1min", start="2025-01-02", end="2025-01-03")
        assert set(result.columns) == {"open", "high", "low", "close", "volume"}
        assert len(result) == 10
        assert result.index.tz is None  # tz-naive

    @patch("newchan.data_databento._get_client")
    def test_fetch_empty_data(self, mock_get_client):
        mock_client = MagicMock()
        empty_mock = MagicMock()
        empty_mock.to_df.return_value = pd.DataFrame()
        mock_client.timeseries.get_range.return_value = empty_mock
        mock_get_client.return_value = mock_client

        from newchan.data_databento import fetch_ohlcv
        result = fetch_ohlcv("AAPL", interval="1min", start="2025-01-02", end="2025-01-03")
        assert result.empty

    @patch("newchan.data_databento._get_client")
    def test_fetch_with_resample(self, mock_get_client):
        mock_client = MagicMock()
        mock_client.timeseries.get_range.return_value = self._make_mock_response(n=30)
        mock_get_client.return_value = mock_client

        from newchan.data_databento import fetch_ohlcv
        result = fetch_ohlcv("AAPL", interval="5min", start="2025-01-02", end="2025-01-03")
        # Should have resampled from 1min to 5min
        assert len(result) <= 7  # 30 bars / 5 + 1


# ══════════ fetch_and_cache (mocked) ══════════


class TestFetchAndCache:
    @patch("newchan.cache.load_df")
    @patch("newchan.cache.append_df")
    @patch("newchan.data_databento.fetch_ohlcv")
    def test_normal_flow(self, mock_fetch, mock_append, mock_load):
        n = 10
        dates = pd.date_range("2025-01-02", periods=n, freq="1min")
        df = pd.DataFrame(
            {"open": [1]*n, "high": [2]*n, "low": [0.5]*n, "close": [1.5]*n, "volume": [100]*n},
            index=dates,
        )
        mock_fetch.return_value = df
        mock_load.return_value = df

        from newchan.data_databento import fetch_and_cache
        name, count = fetch_and_cache("AAPL", "1min", "2025-01-02", "2025-01-03")
        assert "AAPL" in name
        assert count == 10
        mock_append.assert_called_once()
        mock_load.assert_called_once()

    @patch("newchan.cache.load_df")
    @patch("newchan.cache.append_df")
    @patch("newchan.data_databento.fetch_ohlcv")
    def test_empty_result(self, mock_fetch, mock_append, mock_load):
        mock_fetch.return_value = pd.DataFrame()
        from newchan.data_databento import fetch_and_cache
        name, count = fetch_and_cache("AAPL", "1min", "2025-01-02")
        assert count == 0
        mock_append.assert_not_called()


# ══════════ Constants ══════════


class TestConstants:
    def test_symbol_catalog_not_empty(self):
        assert len(SYMBOL_CATALOG) > 0

    def test_default_symbols_not_empty(self):
        assert len(DEFAULT_SYMBOLS) > 0

    def test_catalog_entry_schema(self):
        for item in SYMBOL_CATALOG[:3]:
            assert "symbol" in item
            assert "name" in item
