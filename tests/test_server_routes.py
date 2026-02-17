"""Unit tests for newchan.server route handlers.

All IO (cache, indicators, bottle.request) is mocked so no real
filesystem / network access occurs.
"""

from __future__ import annotations

import json
from unittest.mock import MagicMock, patch

import pandas as pd
import pytest

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_ohlcv_df(n: int = 5) -> pd.DataFrame:
    """Return a tiny OHLCV DataFrame with a DatetimeIndex (tz-naive)."""
    idx = pd.date_range("2024-01-01", periods=n, freq="1min")
    return pd.DataFrame(
        {
            "open": range(1, n + 1),
            "high": range(2, n + 2),
            "low": range(0, n),
            "close": range(1, n + 1),
            "volume": [100] * n,
        },
        index=idx,
    )


def _parse(raw: str) -> dict | list:
    return json.loads(raw)


def _make_query(params: dict | None = None):
    """Build a mock that behaves like bottle.request.query."""
    store = params or {}
    mock = MagicMock()
    mock.get = lambda key, default="": store.get(key, default)
    return mock


# ===================================================================
# api_symbols
# ===================================================================

class TestApiSymbols:
    @patch("newchan.server.list_cached")
    def test_returns_cached_list(self, mock_list):
        mock_list.return_value = [
            {"name": "CL_1min_raw", "symbol": "CL", "interval": "1min"}
        ]
        from newchan.server import api_symbols

        result = _parse(api_symbols())
        assert result == [{"name": "CL_1min_raw", "symbol": "CL", "interval": "1min"}]
        mock_list.assert_called_once()


# ===================================================================
# api_indicators
# ===================================================================

class TestApiIndicators:
    def test_returns_indicator_list(self):
        from newchan.server import api_indicators

        result = _parse(api_indicators())
        assert isinstance(result, list)
        assert len(result) > 0
        first = result[0]
        assert "name" in first
        assert "display" in first
        assert "params" in first
        assert "series" in first


# ===================================================================
# api_timeframes
# ===================================================================

class TestApiTimeframes:
    def test_returns_supported_tf(self):
        from newchan.server import api_timeframes
        from newchan.b_timeframe import SUPPORTED_TF

        result = _parse(api_timeframes())
        assert result == SUPPORTED_TF


# ===================================================================
# api_ohlcv
# ===================================================================

class TestApiOhlcv:
    @patch("newchan.server.request")
    def test_missing_symbol_returns_400(self, mock_req):
        mock_req.query = _make_query({"symbol": ""})
        from newchan.server import api_ohlcv, response

        result = _parse(api_ohlcv())
        assert "error" in result
        assert response.status_code == 400

    @patch("newchan.server.load_df", return_value=None)
    @patch("newchan.server.request")
    def test_no_cache_returns_404(self, mock_req, mock_load):
        mock_req.query = _make_query({"symbol": "CL", "interval": "1min", "tf": "1m"})
        from newchan.server import api_ohlcv, response

        result = _parse(api_ohlcv())
        assert "error" in result
        assert response.status_code == 404

    @patch("newchan.server.resample_ohlc")
    @patch("newchan.server.load_df")
    @patch("newchan.server.request")
    def test_success_returns_data(self, mock_req, mock_load, mock_resample):
        df = _make_ohlcv_df(3)
        mock_req.query = _make_query({"symbol": "CL", "interval": "1min", "tf": "1m"})
        mock_load.return_value = df
        mock_resample.return_value = df

        from newchan.server import api_ohlcv

        result = _parse(api_ohlcv())
        assert "data" in result
        assert "count" in result
        assert result["count"] == 3

    @patch("newchan.server.resample_ohlc")
    @patch("newchan.server.load_df")
    @patch("newchan.server.request")
    def test_countback_pagination(self, mock_req, mock_load, mock_resample):
        df = _make_ohlcv_df(10)
        mock_req.query = _make_query({
            "symbol": "CL", "interval": "1min", "tf": "1m",
            "countBack": "3",
        })
        mock_load.return_value = df
        mock_resample.return_value = df

        from newchan.server import api_ohlcv

        result = _parse(api_ohlcv())
        assert result["count"] == 3

    @patch("newchan.server.resample_ohlc")
    @patch("newchan.server.load_df")
    @patch("newchan.server.request")
    def test_resample_valueerror_returns_400(self, mock_req, mock_load, mock_resample):
        mock_req.query = _make_query({"symbol": "CL", "interval": "1min", "tf": "bad"})
        mock_load.return_value = _make_ohlcv_df()
        mock_resample.side_effect = ValueError("unsupported tf")

        from newchan.server import api_ohlcv, response

        result = _parse(api_ohlcv())
        assert "error" in result
        assert response.status_code == 400


# ===================================================================
# api_indicator
# ===================================================================

class TestApiIndicator:
    @patch("newchan.server.request")
    def test_missing_symbol_returns_400(self, mock_req):
        mock_req.query = _make_query({"symbol": "", "name": "MACD"})
        from newchan.server import api_indicator, response

        result = _parse(api_indicator())
        assert "error" in result
        assert response.status_code == 400

    @patch("newchan.server.request")
    def test_missing_name_returns_400(self, mock_req):
        mock_req.query = _make_query({"symbol": "CL", "name": ""})
        from newchan.server import api_indicator, response

        result = _parse(api_indicator())
        assert "error" in result
        assert response.status_code == 400

    @patch("newchan.server.load_df", return_value=None)
    @patch("newchan.server.request")
    def test_no_cache_returns_404(self, mock_req, mock_load):
        mock_req.query = _make_query({"symbol": "CL", "name": "MACD"})
        from newchan.server import api_indicator, response

        result = _parse(api_indicator())
        assert "error" in result
        assert response.status_code == 404

    @patch("newchan.server.compute_indicator")
    @patch("newchan.server.resample_ohlc")
    @patch("newchan.server.load_df")
    @patch("newchan.server.request")
    def test_success_returns_data(self, mock_req, mock_load, mock_resample, mock_compute):
        df = _make_ohlcv_df(5)
        indicator_df = pd.DataFrame({"macd": [1.0] * 5}, index=df.index)
        mock_req.query = _make_query({"symbol": "CL", "name": "MACD", "tf": "1m"})
        mock_load.return_value = df
        mock_resample.return_value = df
        mock_compute.return_value = indicator_df

        from newchan.server import api_indicator

        result = _parse(api_indicator())
        assert "data" in result
        assert len(result["data"]) == 5

    @patch("newchan.server.compute_indicator")
    @patch("newchan.server.resample_ohlc")
    @patch("newchan.server.load_df")
    @patch("newchan.server.request")
    def test_valueerror_returns_400(self, mock_req, mock_load, mock_resample, mock_compute):
        df = _make_ohlcv_df()
        mock_req.query = _make_query({"symbol": "CL", "name": "UNKNOWN", "tf": "1m"})
        mock_load.return_value = df
        mock_resample.return_value = df
        mock_compute.side_effect = ValueError("未知指标")

        from newchan.server import api_indicator, response

        result = _parse(api_indicator())
        assert "error" in result
        assert response.status_code == 400

    @patch("newchan.server.compute_indicator")
    @patch("newchan.server.resample_ohlc")
    @patch("newchan.server.load_df")
    @patch("newchan.server.request")
    def test_keyerror_returns_400(self, mock_req, mock_load, mock_resample, mock_compute):
        df = _make_ohlcv_df()
        mock_req.query = _make_query({"symbol": "CL", "name": "BAD", "tf": "1m"})
        mock_load.return_value = df
        mock_resample.return_value = df
        mock_compute.side_effect = KeyError("missing key")

        from newchan.server import api_indicator, response

        result = _parse(api_indicator())
        assert "error" in result
        assert response.status_code == 400


# ===================================================================
# api_fetch (POST)
# ===================================================================

class TestApiFetch:
    @patch("newchan.server.request")
    def test_missing_symbol_returns_400(self, mock_req):
        mock_req.json = {"symbol": ""}
        from newchan.server import api_fetch, response

        result = _parse(api_fetch())
        assert "error" in result
        assert response.status_code == 400

    @patch("newchan.server.threading.Thread")
    @patch("newchan.server.request")
    def test_starts_fetch_thread(self, mock_req, mock_thread_cls):
        # Clear any leftover status from other tests
        import newchan.server as srv
        srv._fetch_status.clear()

        mock_req.json = {"symbol": "CL", "interval": "1min", "start": "2024-01-01"}
        mock_thread = MagicMock()
        mock_thread_cls.return_value = mock_thread

        result = _parse(srv.api_fetch())
        assert result["status"] == "running"
        assert "task_id" in result
        mock_thread.start.assert_called_once()

    @patch("newchan.server.request")
    def test_already_running_returns_running(self, mock_req):
        import newchan.server as srv

        srv._fetch_status["CL_1min"] = {"status": "running"}
        mock_req.json = {"symbol": "CL", "interval": "1min"}

        result = _parse(srv.api_fetch())
        assert result["status"] == "running"

        # cleanup
        srv._fetch_status.clear()


# ===================================================================
# api_synthetic (POST)
# ===================================================================

class TestApiSynthetic:
    @patch("newchan.server.request")
    def test_missing_a_returns_400(self, mock_req):
        mock_req.json = {"a": "", "b": "GC"}
        from newchan.server import api_synthetic, response

        result = _parse(api_synthetic())
        assert "error" in result
        assert response.status_code == 400

    @patch("newchan.server.request")
    def test_missing_b_returns_400(self, mock_req):
        mock_req.json = {"a": "CL", "b": ""}
        from newchan.server import api_synthetic, response

        result = _parse(api_synthetic())
        assert "error" in result
        assert response.status_code == 400

    @patch("newchan.server.load_df")
    @patch("newchan.server.request")
    def test_no_cache_a_returns_404(self, mock_req, mock_load):
        mock_req.json = {"a": "CL", "b": "GC", "op": "spread"}
        mock_load.return_value = None

        from newchan.server import api_synthetic, response

        result = _parse(api_synthetic())
        assert "error" in result
        assert response.status_code == 404

    @patch("newchan.server.load_df")
    @patch("newchan.server.request")
    def test_no_cache_b_returns_404(self, mock_req, mock_load):
        df = _make_ohlcv_df()
        mock_req.json = {"a": "CL", "b": "GC", "op": "spread"}
        mock_load.side_effect = lambda name: df if "CL" in name else None

        from newchan.server import api_synthetic, response

        result = _parse(api_synthetic())
        assert "error" in result
        assert response.status_code == 404

    @patch("newchan.server.save_df")
    @patch("newchan.server.load_df")
    @patch("newchan.server.request")
    def test_spread_success(self, mock_req, mock_load, mock_save):
        df = _make_ohlcv_df(5)
        mock_req.json = {"a": "CL", "b": "GC", "op": "spread", "interval": "1min"}
        mock_load.return_value = df
        mock_save.return_value = None

        with patch("newchan.synthetic.make_spread", return_value=df) as mock_spread:
            from newchan.server import api_synthetic

            result = _parse(api_synthetic())
            assert result["name"] == "CL_GC_spread"
            assert result["count"] == 5
            mock_spread.assert_called_once()

    @patch("newchan.server.save_df")
    @patch("newchan.server.load_df")
    @patch("newchan.server.request")
    def test_unknown_op_returns_400(self, mock_req, mock_load, mock_save):
        df = _make_ohlcv_df()
        mock_req.json = {"a": "CL", "b": "GC", "op": "INVALID"}
        mock_load.return_value = df

        from newchan.server import api_synthetic, response

        result = _parse(api_synthetic())
        assert "error" in result
        assert response.status_code == 400


# ===================================================================
# api_newchan_overlay
# ===================================================================

class TestApiNewchanOverlay:
    @patch("newchan.server.request")
    def test_missing_symbol_returns_400(self, mock_req):
        mock_req.query = _make_query({"symbol": ""})
        from newchan.server import api_newchan_overlay, response

        result = _parse(api_newchan_overlay())
        assert "error" in result
        assert response.status_code == 400

    @patch("newchan.server.load_df", return_value=None)
    @patch("newchan.server.request")
    def test_no_cache_returns_404(self, mock_req, mock_load):
        mock_req.query = _make_query({"symbol": "CL"})
        from newchan.server import api_newchan_overlay, response

        result = _parse(api_newchan_overlay())
        assert "error" in result
        assert response.status_code == 404

    @patch("newchan.server.resample_ohlc")
    @patch("newchan.server.load_df")
    @patch("newchan.server.request")
    def test_success_returns_overlay(self, mock_req, mock_load, mock_resample):
        df = _make_ohlcv_df(10)
        mock_req.query = _make_query({
            "symbol": "CL", "interval": "1min", "tf": "1m",
            "detail": "full", "segment_algo": "v1",
            "stroke_mode": "wide", "min_strict_sep": "5",
            "center_sustain_m": "2", "limit": "",
        })
        mock_load.return_value = df
        mock_resample.return_value = df

        overlay_data = {"strokes": [], "segments": []}
        with patch("newchan.ab_bridge_newchan.build_overlay_newchan", return_value=overlay_data) as mock_build:
            from newchan.server import api_newchan_overlay

            result = _parse(api_newchan_overlay())
            assert result == overlay_data
            mock_build.assert_called_once()

    @patch("newchan.server.resample_ohlc")
    @patch("newchan.server.load_df")
    @patch("newchan.server.request")
    def test_exception_returns_500(self, mock_req, mock_load, mock_resample):
        df = _make_ohlcv_df()
        mock_req.query = _make_query({
            "symbol": "CL", "interval": "1min", "tf": "1m",
            "detail": "full", "segment_algo": "v1",
            "stroke_mode": "wide", "min_strict_sep": "5",
            "center_sustain_m": "2", "limit": "",
        })
        mock_load.return_value = df
        mock_resample.return_value = df

        with patch("newchan.ab_bridge_newchan.build_overlay_newchan", side_effect=RuntimeError("boom")):
            from newchan.server import api_newchan_overlay, response

            result = _parse(api_newchan_overlay())
            assert "error" in result
            assert response.status_code == 500
