"""data_av.py + b_chart.py 单元测试 — mock 隔离。"""
from __future__ import annotations

from datetime import datetime
from unittest.mock import MagicMock, patch, PropertyMock
import time

import pytest

from newchan.types import Bar


# ══════════ AlphaVantageProvider ══════════


class TestAlphaVantageProvider:
    def _make_provider(self, api_key="test-key", rate_limit=0.0):
        """Create provider with no rate limiting for fast tests."""
        from newchan.data_av import AlphaVantageProvider
        return AlphaVantageProvider(api_key=api_key, rate_limit=rate_limit)

    def test_init_defaults(self):
        prov = self._make_provider()
        assert prov.api_key == "test-key"
        assert prov.rate_limit == 0.0

    def test_throttle_no_wait(self):
        prov = self._make_provider(rate_limit=0.0)
        start = time.monotonic()
        prov._throttle()
        prov._throttle()
        elapsed = time.monotonic() - start
        assert elapsed < 0.1

    @patch("newchan.data_av.requests.get")
    def test_get_success(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"data": []}
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        prov = self._make_provider()
        result = prov._get({"function": "TEST"})
        assert result == {"data": []}
        mock_get.assert_called_once()

    @patch("newchan.data_av.requests.get")
    def test_get_error_message(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"Error Message": "bad request"}
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        prov = self._make_provider()
        with pytest.raises(RuntimeError, match="Alpha Vantage error"):
            prov._get({"function": "TEST"})

    @patch("newchan.data_av.requests.get")
    def test_get_information_field(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"Information": "rate limited"}
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        prov = self._make_provider()
        with pytest.raises(RuntimeError, match="Alpha Vantage info"):
            prov._get({"function": "TEST"})

    @patch("newchan.data_av.requests.get")
    def test_fetch_brent_daily(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "data": [
                {"date": "2025-01-02", "value": "75.50"},
                {"date": "2025-01-03", "value": "76.00"},
                {"date": "2025-01-04", "value": "."},  # missing value
            ]
        }
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        prov = self._make_provider()
        bars = prov.fetch_brent_daily()
        assert len(bars) == 2  # missing value skipped
        assert isinstance(bars[0], Bar)
        assert bars[0].close == 75.50
        assert bars[1].close == 76.00

    @patch("newchan.data_av.requests.get")
    def test_fetch_brent_empty(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"data": []}
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        prov = self._make_provider()
        bars = prov.fetch_brent_daily()
        assert bars == []


# ══════════ b_chart.py ══════════


class TestBuildAppHtml:
    @patch("newchan.b_chart._load_js")
    def test_returns_html_string(self, mock_load_js):
        mock_load_js.return_value = "// mock JS content"
        from newchan.b_chart import build_app_html
        html = build_app_html()
        assert isinstance(html, str)
        assert "// mock JS content" in html

    @patch("newchan.b_chart._load_js")
    def test_contains_html_structure(self, mock_load_js):
        mock_load_js.return_value = "var x = 1;"
        from newchan.b_chart import build_app_html
        html = build_app_html()
        # Should contain basic HTML tags from the template
        assert "<" in html  # At minimum has HTML tags


# ══════════ fetch_intraday / fetch_daily / fetch_macd ══════════


class TestAlphaVantageIntraday:
    def _make_provider(self, api_key="test-key", rate_limit=0.0):
        from newchan.data_av import AlphaVantageProvider
        return AlphaVantageProvider(api_key=api_key, rate_limit=rate_limit)

    @patch("newchan.data_av.requests.get")
    def test_fetch_intraday(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "Meta Data": {},
            "Time Series (1min)": {
                "2025-06-01 09:31:00": {
                    "1. open": "100.00", "2. high": "101.00",
                    "3. low": "99.50", "4. close": "100.50", "5. volume": "1000",
                },
                "2025-06-01 09:32:00": {
                    "1. open": "100.50", "2. high": "101.50",
                    "3. low": "100.00", "4. close": "101.00", "5. volume": "1500",
                },
            },
        }
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        prov = self._make_provider()
        bars = prov.fetch_intraday("SPY", interval="1min")
        assert len(bars) == 2
        assert bars[0].open == 100.00
        assert bars[1].close == 101.00
        assert bars[0].ts < bars[1].ts  # 正序

    @patch("newchan.data_av.requests.get")
    def test_fetch_intraday_empty(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.json.return_value = {"Meta Data": {}, "Time Series (1min)": {}}
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        prov = self._make_provider()
        bars = prov.fetch_intraday("SPY")
        assert bars == []


class TestAlphaVantageDaily:
    def _make_provider(self, api_key="test-key", rate_limit=0.0):
        from newchan.data_av import AlphaVantageProvider
        return AlphaVantageProvider(api_key=api_key, rate_limit=rate_limit)

    @patch("newchan.data_av.requests.get")
    def test_fetch_daily(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "Meta Data": {},
            "Time Series (Daily)": {
                "2025-06-02": {
                    "1. open": "200.00", "2. high": "205.00",
                    "3. low": "198.00", "4. close": "203.00", "5. volume": "50000",
                },
                "2025-06-01": {
                    "1. open": "195.00", "2. high": "201.00",
                    "3. low": "194.00", "4. close": "200.00", "5. volume": "40000",
                },
            },
        }
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        prov = self._make_provider()
        bars = prov.fetch_daily("SPY")
        assert len(bars) == 2
        assert bars[0].ts < bars[1].ts  # sorted ascending
        assert bars[0].close == 200.00  # June 1 first
        assert bars[1].close == 203.00  # June 2 second


class TestAlphaVantageMACD:
    def _make_provider(self, api_key="test-key", rate_limit=0.0):
        from newchan.data_av import AlphaVantageProvider
        return AlphaVantageProvider(api_key=api_key, rate_limit=rate_limit)

    @patch("newchan.data_av.requests.get")
    def test_fetch_macd_returns_dataframe(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "Meta Data": {},
            "Technical Analysis: MACD": {
                "2025-06-01": {"MACD": "1.5", "MACD_Signal": "1.2", "MACD_Hist": "0.3"},
                "2025-06-02": {"MACD": "1.8", "MACD_Signal": "1.4", "MACD_Hist": "0.4"},
                "2025-06-03": {"MACD": "1.3", "MACD_Signal": "1.3", "MACD_Hist": "0.0"},
            },
        }
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        prov = self._make_provider()
        df = prov.fetch_macd("SPY")
        assert list(df.columns) == ["macd", "signal", "hist"]
        assert len(df) == 3
        assert df.index.is_monotonic_increasing  # sorted ascending

    @patch("newchan.data_av.requests.get")
    def test_fetch_macd_values_correct(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "Meta Data": {},
            "Technical Analysis: MACD": {
                "2025-06-01": {"MACD": "2.5", "MACD_Signal": "2.0", "MACD_Hist": "0.5"},
            },
        }
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        prov = self._make_provider()
        df = prov.fetch_macd("SPY")
        assert df.iloc[0]["macd"] == 2.5
        assert df.iloc[0]["signal"] == 2.0
        assert df.iloc[0]["hist"] == 0.5

    @patch("newchan.data_av.requests.get")
    def test_fetch_macd_empty(self, mock_get):
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "Meta Data": {},
            "Technical Analysis: MACD": {},
        }
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        prov = self._make_provider()
        df = prov.fetch_macd("SPY")
        assert len(df) == 0
        assert list(df.columns) == ["macd", "signal", "hist"]

    @patch("newchan.data_av.requests.get")
    def test_fetch_macd_compatible_with_pipeline(self, mock_get):
        """AV MACD 的 df 格式与 a_macd.compute_macd() 输出兼容。"""
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "Meta Data": {},
            "Technical Analysis: MACD": {
                "2025-06-01": {"MACD": "1.0", "MACD_Signal": "0.8", "MACD_Hist": "0.2"},
                "2025-06-02": {"MACD": "1.5", "MACD_Signal": "1.1", "MACD_Hist": "0.4"},
            },
        }
        mock_resp.raise_for_status = MagicMock()
        mock_get.return_value = mock_resp

        prov = self._make_provider()
        df = prov.fetch_macd("SPY")
        # 管线期望的列名
        assert "macd" in df.columns
        assert "signal" in df.columns
        assert "hist" in df.columns
        # 管线通过 .iloc 按位置访问
        from newchan.a_macd import macd_area_for_range
        area = macd_area_for_range(df, 0, 1)
        assert area["n_bars"] == 2
        assert area["area_total"] == round(0.2 + 0.4, 6)
