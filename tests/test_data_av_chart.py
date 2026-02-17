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
