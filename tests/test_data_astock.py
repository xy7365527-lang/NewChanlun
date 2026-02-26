"""data_astock.py 单元测试 — mock akshare，不依赖网络。"""
from __future__ import annotations

from datetime import datetime
from unittest.mock import MagicMock, patch

import numpy as np
import pandas as pd
import pytest

from newchan.types import Bar


# ══════════ Helper: 构造 mock akshare 返回的 DataFrame ══════════


def _make_ak_minute_df(n: int = 60, start: str = "2025-01-02 09:31:00") -> pd.DataFrame:
    """模拟 akshare stock_zh_a_hist_min_em 返回格式。"""
    dates = pd.date_range(start, periods=n, freq="1min")
    rng = np.random.default_rng(42)
    close = 10.0 + rng.standard_normal(n).cumsum() * 0.05
    return pd.DataFrame({
        "时间": [d.strftime("%Y-%m-%d %H:%M:%S") for d in dates],
        "开盘": close - 0.02,
        "收盘": close,
        "最高": close + 0.05,
        "最低": close - 0.05,
        "成交量": rng.integers(100, 5000, n).astype(float),
        "成交额": rng.integers(10000, 500000, n).astype(float),
    })


def _make_ak_daily_df(n: int = 30, start: str = "2025-01-02") -> pd.DataFrame:
    """模拟 akshare stock_zh_a_hist 返回格式（日线，前复权）。"""
    dates = pd.bdate_range(start, periods=n)
    rng = np.random.default_rng(42)
    close = 10.0 + rng.standard_normal(n).cumsum() * 0.1
    return pd.DataFrame({
        "日期": [d.strftime("%Y-%m-%d") for d in dates],
        "开盘": close - 0.05,
        "收盘": close,
        "最高": close + 0.1,
        "最低": close - 0.1,
        "成交量": rng.integers(10000, 500000, n).astype(float),
        "成交额": rng.integers(1000000, 50000000, n).astype(float),
    })


# ══════════ AStockProvider.__init__ ══════════


class TestAStockProviderInit:
    @patch("newchan.data_astock.ak", new_callable=MagicMock)
    def test_default_adjust(self, _mock_ak):
        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        assert provider.adjust == "qfq"

    @patch("newchan.data_astock.ak", new_callable=MagicMock)
    def test_custom_adjust(self, _mock_ak):
        from newchan.data_astock import AStockProvider
        provider = AStockProvider(adjust="hfq")
        assert provider.adjust == "hfq"


# ══════════ fetch_minute ══════════


class TestFetchMinute:
    @patch("newchan.data_astock.ak")
    def test_returns_bars(self, mock_ak):
        mock_ak.stock_zh_a_hist_min_em.return_value = _make_ak_minute_df(10)
        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        bars = provider.fetch_minute("000001", period="1")
        assert len(bars) == 10
        assert all(isinstance(b, Bar) for b in bars)

    @patch("newchan.data_astock.ak")
    def test_bars_sorted_by_time(self, mock_ak):
        mock_ak.stock_zh_a_hist_min_em.return_value = _make_ak_minute_df(20)
        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        bars = provider.fetch_minute("000001", period="1")
        timestamps = [b.ts for b in bars]
        assert timestamps == sorted(timestamps)

    @patch("newchan.data_astock.ak")
    def test_empty_result(self, mock_ak):
        mock_ak.stock_zh_a_hist_min_em.return_value = pd.DataFrame()
        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        bars = provider.fetch_minute("000001", period="1")
        assert bars == []

    @patch("newchan.data_astock.ak")
    def test_suspension_day_empty(self, mock_ak):
        """停牌日 akshare 返回空 DataFrame。"""
        mock_ak.stock_zh_a_hist_min_em.return_value = pd.DataFrame()
        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        bars = provider.fetch_minute("000001", period="1")
        assert bars == []

    @patch("newchan.data_astock.ak")
    def test_5min_period(self, mock_ak):
        mock_ak.stock_zh_a_hist_min_em.return_value = _make_ak_minute_df(10)
        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        bars = provider.fetch_minute("000001", period="5")
        assert len(bars) == 10


# ══════════ fetch_daily ══════════


class TestFetchDaily:
    @patch("newchan.data_astock.ak")
    def test_returns_bars(self, mock_ak):
        mock_ak.stock_zh_a_hist.return_value = _make_ak_daily_df(10)
        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        bars = provider.fetch_daily("000001", start_date="20250102", end_date="20250115")
        assert len(bars) == 10
        assert all(isinstance(b, Bar) for b in bars)

    @patch("newchan.data_astock.ak")
    def test_daily_bars_sorted(self, mock_ak):
        mock_ak.stock_zh_a_hist.return_value = _make_ak_daily_df(15)
        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        bars = provider.fetch_daily("000001", start_date="20250102", end_date="20250120")
        timestamps = [b.ts for b in bars]
        assert timestamps == sorted(timestamps)

    @patch("newchan.data_astock.ak")
    def test_empty_result(self, mock_ak):
        mock_ak.stock_zh_a_hist.return_value = pd.DataFrame()
        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        bars = provider.fetch_daily("000001", start_date="20250102", end_date="20250115")
        assert bars == []

    @patch("newchan.data_astock.ak")
    def test_adjust_qfq(self, mock_ak):
        """前复权参数透传。"""
        mock_ak.stock_zh_a_hist.return_value = _make_ak_daily_df(5)
        from newchan.data_astock import AStockProvider
        provider = AStockProvider(adjust="qfq")
        provider.fetch_daily("000001", start_date="20250102", end_date="20250110")
        call_kwargs = mock_ak.stock_zh_a_hist.call_args
        assert call_kwargs.kwargs.get("adjust") == "qfq" or call_kwargs[1].get("adjust") == "qfq"


# ══════════ fetch_ohlcv (统一接口) ══════════


class TestFetchOhlcv:
    @patch("newchan.data_astock.ak")
    def test_1min_delegates_to_fetch_minute(self, mock_ak):
        mock_ak.stock_zh_a_hist_min_em.return_value = _make_ak_minute_df(10)
        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        df = provider.fetch_ohlcv("000001", interval="1min")
        assert not df.empty
        assert set(df.columns) >= {"open", "high", "low", "close", "volume"}

    @patch("newchan.data_astock.ak")
    def test_1day_delegates_to_fetch_daily(self, mock_ak):
        mock_ak.stock_zh_a_hist.return_value = _make_ak_daily_df(10)
        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        df = provider.fetch_ohlcv("000001", interval="1day")
        assert not df.empty

    @patch("newchan.data_astock.ak")
    def test_ohlcv_tz_naive_index(self, mock_ak):
        mock_ak.stock_zh_a_hist_min_em.return_value = _make_ak_minute_df(10)
        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        df = provider.fetch_ohlcv("000001", interval="1min")
        assert df.index.tz is None

    @patch("newchan.data_astock.ak")
    def test_ohlcv_empty(self, mock_ak):
        mock_ak.stock_zh_a_hist_min_em.return_value = pd.DataFrame()
        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        df = provider.fetch_ohlcv("000001", interval="1min")
        assert df.empty


# ══════════ fetch_and_cache ══════════


class TestFetchAndCache:
    @patch("newchan.cache.load_df")
    @patch("newchan.cache.append_df")
    @patch("newchan.data_astock.AStockProvider.fetch_ohlcv")
    def test_normal_flow(self, mock_fetch, mock_append, mock_load):
        n = 10
        dates = pd.date_range("2025-01-02 09:31", periods=n, freq="1min")
        df = pd.DataFrame(
            {"open": [10]*n, "high": [11]*n, "low": [9]*n, "close": [10.5]*n, "volume": [1000]*n},
            index=dates,
        )
        mock_fetch.return_value = df
        mock_load.return_value = df

        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        name, count = provider.fetch_and_cache("000001", interval="1min")
        assert "000001" in name
        assert count == 10
        mock_append.assert_called_once()

    @patch("newchan.cache.load_df")
    @patch("newchan.cache.append_df")
    @patch("newchan.data_astock.AStockProvider.fetch_ohlcv")
    def test_empty_result(self, mock_fetch, mock_append, mock_load):
        mock_fetch.return_value = pd.DataFrame()
        from newchan.data_astock import AStockProvider
        provider = AStockProvider()
        name, count = provider.fetch_and_cache("000001", interval="1min")
        assert count == 0
        mock_append.assert_not_called()


# ══════════ _normalize_symbol ══════════


class TestNormalizeSymbol:
    def test_six_digit_passthrough(self):
        from newchan.data_astock import _normalize_symbol
        assert _normalize_symbol("000001") == "000001"

    def test_strips_prefix(self):
        from newchan.data_astock import _normalize_symbol
        assert _normalize_symbol("SZ000001") == "000001"
        assert _normalize_symbol("SH600000") == "600000"

    def test_strips_dot_prefix(self):
        from newchan.data_astock import _normalize_symbol
        assert _normalize_symbol("000001.SZ") == "000001"
        assert _normalize_symbol("600000.SH") == "600000"
