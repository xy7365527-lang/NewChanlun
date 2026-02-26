"""data_crypto.py 单元测试 — mock ccxt，不依赖网络。"""
from __future__ import annotations

import sys
from datetime import datetime, timezone
from types import ModuleType
from unittest.mock import AsyncMock, MagicMock, patch

import numpy as np
import pandas as pd
import pytest

from newchan.types import Bar


# ══════════ ccxt mock 注入 ══════════

_mock_ccxt = ModuleType("ccxt")
_mock_ccxt.binance = MagicMock()  # type: ignore[attr-defined]
_mock_ccxt.okx = MagicMock()  # type: ignore[attr-defined]
_mock_ccxt.pro = MagicMock()  # type: ignore[attr-defined]


@pytest.fixture(autouse=True)
def _inject_ccxt_mock():
    """在每个测试前注入 ccxt mock，测试后恢复。"""
    orig = sys.modules.get("ccxt")
    sys.modules["ccxt"] = _mock_ccxt
    # 重置 mock 状态
    _mock_ccxt.binance.reset_mock()
    _mock_ccxt.okx.reset_mock()
    _mock_ccxt.pro.reset_mock()
    yield
    if orig is not None:
        sys.modules["ccxt"] = orig
    else:
        sys.modules.pop("ccxt", None)


# ══════════ Helper: 构造 mock ccxt OHLCV 数据 ══════════


def _make_ccxt_ohlcv(n: int = 60, start_ms: int = 1704153600000) -> list[list]:
    """模拟 ccxt fetch_ohlcv 返回格式: [[ts_ms, o, h, l, c, v], ...]"""
    rng = np.random.default_rng(42)
    close = 42000.0 + rng.standard_normal(n).cumsum() * 50
    rows = []
    for i in range(n):
        ts_ms = start_ms + i * 60_000
        o = close[i] - 10
        h = close[i] + 30
        l = close[i] - 30
        c = close[i]
        v = float(rng.integers(1, 100))
        rows.append([ts_ms, o, h, l, c, v])
    return rows


# ══════════ CryptoProvider.__init__ ══════════


class TestCryptoProviderInit:
    def test_default_exchange(self):
        mock_instance = MagicMock()
        _mock_ccxt.binance.return_value = mock_instance
        from newchan.data_crypto import CryptoProvider
        provider = CryptoProvider()
        assert provider.exchange_id == "binance"

    def test_custom_exchange(self):
        mock_instance = MagicMock()
        _mock_ccxt.okx.return_value = mock_instance
        from newchan.data_crypto import CryptoProvider
        provider = CryptoProvider(exchange_id="okx")
        assert provider.exchange_id == "okx"


# ══════════ fetch_ohlcv ══════════


class TestFetchOhlcv:
    def test_returns_bars(self):
        mock_exchange = MagicMock()
        mock_exchange.fetch_ohlcv.return_value = _make_ccxt_ohlcv(10)
        _mock_ccxt.binance.return_value = mock_exchange

        from newchan.data_crypto import CryptoProvider
        provider = CryptoProvider()
        bars = provider.fetch_ohlcv("BTC/USDT", timeframe="1m", limit=10)
        assert len(bars) == 10
        assert all(isinstance(b, Bar) for b in bars)

    def test_bars_sorted_by_time(self):
        mock_exchange = MagicMock()
        mock_exchange.fetch_ohlcv.return_value = _make_ccxt_ohlcv(20)
        _mock_ccxt.binance.return_value = mock_exchange

        from newchan.data_crypto import CryptoProvider
        provider = CryptoProvider()
        bars = provider.fetch_ohlcv("BTC/USDT", timeframe="1m")
        timestamps = [b.ts for b in bars]
        assert timestamps == sorted(timestamps)

    def test_empty_result(self):
        mock_exchange = MagicMock()
        mock_exchange.fetch_ohlcv.return_value = []
        _mock_ccxt.binance.return_value = mock_exchange

        from newchan.data_crypto import CryptoProvider
        provider = CryptoProvider()
        bars = provider.fetch_ohlcv("BTC/USDT", timeframe="1m")
        assert bars == []

    def test_since_parameter(self):
        mock_exchange = MagicMock()
        mock_exchange.fetch_ohlcv.return_value = _make_ccxt_ohlcv(5)
        _mock_ccxt.binance.return_value = mock_exchange

        from newchan.data_crypto import CryptoProvider
        provider = CryptoProvider()
        provider.fetch_ohlcv("BTC/USDT", timeframe="1m", since="2025-01-01")
        call_args = mock_exchange.fetch_ohlcv.call_args
        # since should be converted to millisecond timestamp
        assert call_args[1].get("since") is not None

    def test_volume_present(self):
        mock_exchange = MagicMock()
        mock_exchange.fetch_ohlcv.return_value = _make_ccxt_ohlcv(5)
        _mock_ccxt.binance.return_value = mock_exchange

        from newchan.data_crypto import CryptoProvider
        provider = CryptoProvider()
        bars = provider.fetch_ohlcv("BTC/USDT", timeframe="1m")
        assert all(b.volume is not None for b in bars)


# ══════════ fetch_ohlcv_df (DataFrame 接口) ══════════


class TestFetchOhlcvDf:
    def test_returns_dataframe(self):
        mock_exchange = MagicMock()
        mock_exchange.fetch_ohlcv.return_value = _make_ccxt_ohlcv(10)
        _mock_ccxt.binance.return_value = mock_exchange

        from newchan.data_crypto import CryptoProvider
        provider = CryptoProvider()
        df = provider.fetch_ohlcv_df("BTC/USDT", timeframe="1m")
        assert not df.empty
        assert set(df.columns) >= {"open", "high", "low", "close", "volume"}

    def test_tz_naive_index(self):
        mock_exchange = MagicMock()
        mock_exchange.fetch_ohlcv.return_value = _make_ccxt_ohlcv(10)
        _mock_ccxt.binance.return_value = mock_exchange

        from newchan.data_crypto import CryptoProvider
        provider = CryptoProvider()
        df = provider.fetch_ohlcv_df("BTC/USDT", timeframe="1m")
        assert df.index.tz is None

    def test_empty_returns_empty_df(self):
        mock_exchange = MagicMock()
        mock_exchange.fetch_ohlcv.return_value = []
        _mock_ccxt.binance.return_value = mock_exchange

        from newchan.data_crypto import CryptoProvider
        provider = CryptoProvider()
        df = provider.fetch_ohlcv_df("BTC/USDT", timeframe="1m")
        assert df.empty


# ══════════ fetch_and_cache ══════════


class TestFetchAndCache:
    @patch("newchan.cache.load_df")
    @patch("newchan.cache.append_df")
    @patch("newchan.data_crypto.CryptoProvider.fetch_ohlcv_df")
    def test_normal_flow(self, mock_fetch, mock_append, mock_load):
        n = 10
        dates = pd.date_range("2025-01-02", periods=n, freq="1min")
        df = pd.DataFrame(
            {"open": [42000]*n, "high": [42100]*n, "low": [41900]*n, "close": [42050]*n, "volume": [5.0]*n},
            index=dates,
        )
        mock_fetch.return_value = df
        mock_load.return_value = df

        _mock_ccxt.binance.return_value = MagicMock()
        from newchan.data_crypto import CryptoProvider
        provider = CryptoProvider()
        name, count = provider.fetch_and_cache("BTC/USDT", timeframe="1m")
        assert "BTC_USDT" in name
        assert count == 10
        mock_append.assert_called_once()

    @patch("newchan.cache.load_df")
    @patch("newchan.cache.append_df")
    @patch("newchan.data_crypto.CryptoProvider.fetch_ohlcv_df")
    def test_empty_result(self, mock_fetch, mock_append, mock_load):
        mock_fetch.return_value = pd.DataFrame()
        _mock_ccxt.binance.return_value = MagicMock()
        from newchan.data_crypto import CryptoProvider
        provider = CryptoProvider()
        name, count = provider.fetch_and_cache("BTC/USDT", timeframe="1m")
        assert count == 0
        mock_append.assert_not_called()


# ══════════ _symbol_to_cache_key ══════════


class TestSymbolToCacheKey:
    def test_slash_replaced(self):
        from newchan.data_crypto import _symbol_to_cache_key
        assert _symbol_to_cache_key("BTC/USDT") == "BTC_USDT"

    def test_no_slash(self):
        from newchan.data_crypto import _symbol_to_cache_key
        assert _symbol_to_cache_key("BTCUSDT") == "BTCUSDT"


# ══════════ _timeframe_to_interval ══════════


class TestTimeframeToInterval:
    def test_common_mappings(self):
        from newchan.data_crypto import _timeframe_to_interval
        assert _timeframe_to_interval("1m") == "1min"
        assert _timeframe_to_interval("5m") == "5min"
        assert _timeframe_to_interval("1h") == "1hour"
        assert _timeframe_to_interval("1d") == "1day"


# ══════════ _parse_ws_ohlcv ══════════


class TestParseWsOhlcv:
    def test_parse_single_row(self):
        from newchan.data_crypto import _parse_ws_ohlcv
        raw = [[1704153600000, 42000.0, 42100.0, 41900.0, 42050.0, 1.5]]
        bars = _parse_ws_ohlcv(raw)
        assert len(bars) == 1
        assert isinstance(bars[0], Bar)
        assert bars[0].close == 42050.0
        assert bars[0].volume == 1.5

    def test_parse_empty(self):
        from newchan.data_crypto import _parse_ws_ohlcv
        assert _parse_ws_ohlcv([]) == []


# ══════════ WebSocket ══════════


class TestWsExchange:
    def test_create_ws_exchange(self):
        mock_ws_instance = MagicMock()
        _mock_ccxt.pro.binance.return_value = mock_ws_instance
        _mock_ccxt.binance.return_value = MagicMock()

        from newchan.data_crypto import CryptoProvider
        provider = CryptoProvider()
        ws = provider._create_ws_exchange()
        assert ws is mock_ws_instance
