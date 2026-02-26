"""API 端点错误处理测试。

覆盖：
- gateway.py REST 端点：session 不存在 → 404，数据缺失 → 404，引擎异常 → 500
- server.py Bottle 端点：参数无效 → 400，feeder 异常 → 500，搜索降级
"""

from __future__ import annotations

import json
from unittest.mock import MagicMock, patch

import pytest
from fastapi.testclient import TestClient

from newchan.gateway import app as gw_app, _sessions, _orchestrators


# ═══════════════════════════════════════════════════
# Gateway (FastAPI) 错误处理
# ═══════════════════════════════════════════════════


@pytest.fixture(autouse=True)
def _clean_gw_state():
    _sessions.clear()
    _orchestrators.clear()
    yield
    _sessions.clear()
    _orchestrators.clear()


@pytest.fixture()
def gw_client():
    return TestClient(gw_app, raise_server_exceptions=False)


class TestGatewayReplayStartErrors:
    """POST /api/replay/start 错误路径。"""

    def test_data_not_found_returns_404(self, gw_client):
        with patch("newchan.gateway._load_bars", side_effect=ValueError("缓存不存在")):
            resp = gw_client.post("/api/replay/start", json={
                "symbol": "MISSING", "tf": "5m",
            })
        assert resp.status_code == 404
        assert "detail" in resp.json()

    def test_empty_data_returns_404(self, gw_client):
        with patch("newchan.gateway._load_bars", return_value=[]):
            resp = gw_client.post("/api/replay/start", json={
                "symbol": "EMPTY", "tf": "5m",
            })
        assert resp.status_code == 404

    def test_invalid_request_body_returns_422(self, gw_client):
        resp = gw_client.post("/api/replay/start", json={})
        assert resp.status_code == 422


class TestGatewaySessionNotFound:
    """各端点 session 不存在 → 404。"""

    def test_step_404(self, gw_client):
        resp = gw_client.post("/api/replay/step", json={
            "session_id": "nonexistent", "count": 1,
        })
        assert resp.status_code == 404

    def test_seek_404(self, gw_client):
        resp = gw_client.post("/api/replay/seek", json={
            "session_id": "nonexistent", "target_idx": 0,
        })
        assert resp.status_code == 404

    def test_play_404(self, gw_client):
        resp = gw_client.post("/api/replay/play", json={
            "session_id": "nonexistent", "speed": 1.0,
        })
        assert resp.status_code == 404

    def test_pause_404(self, gw_client):
        resp = gw_client.post("/api/replay/pause", json={
            "session_id": "nonexistent",
        })
        assert resp.status_code == 404

    def test_status_404(self, gw_client):
        resp = gw_client.get("/api/replay/status", params={
            "session_id": "nonexistent",
        })
        assert resp.status_code == 404


class TestGatewayGlobalExceptionHandler:
    """全局异常兜底 → 500 JSON。"""

    def test_unexpected_engine_error_returns_500(self, gw_client):
        """引擎内部异常被全局 handler 捕获。"""
        from datetime import datetime, timezone
        from newchan.types import Bar
        from newchan.replay import ReplaySession
        from newchan.orchestrator.recursive import RecursiveOrchestrator

        bars = [Bar(ts=datetime(2024, 1, 1, tzinfo=timezone.utc),
                    open=100, high=101, low=99, close=100)]
        engine = RecursiveOrchestrator()
        sess = ReplaySession(session_id="test-err", bars=bars, engine=engine)
        _sessions["test-err"] = sess

        with patch.object(sess, "step", side_effect=RuntimeError("引擎崩溃")):
            resp = gw_client.post("/api/replay/step", json={
                "session_id": "test-err", "count": 1,
            })
        assert resp.status_code == 500
        body = resp.json()
        assert body["code"] == "internal_error"


# ═══════════════════════════════════════════════════
# Server (Bottle) 错误处理
# ═══════════════════════════════════════════════════


def _make_query(params: dict | None = None):
    store = params or {}
    mock = MagicMock()
    mock.get = lambda key, default="": store.get(key, default)
    return mock


def _parse(raw: str) -> dict | list:
    return json.loads(raw)


class TestServerOhlcvParamErrors:
    """api_ohlcv 分页参数无效 → 400。"""

    @patch("newchan.server.resample_ohlc")
    @patch("newchan.server.load_df")
    @patch("newchan.server.request")
    def test_bad_countback_returns_400(self, mock_req, mock_load, mock_resample):
        import pandas as pd
        df = pd.DataFrame(
            {"open": [1], "high": [2], "low": [0], "close": [1], "volume": [100]},
            index=pd.date_range("2024-01-01", periods=1, freq="1min"),
        )
        mock_req.query = _make_query({
            "symbol": "CL", "interval": "1min", "tf": "1m",
            "countBack": "not_a_number",
        })
        mock_load.return_value = df
        mock_resample.return_value = df

        from newchan.server import api_ohlcv, response
        result = _parse(api_ohlcv())
        assert "error" in result
        assert response.status_code == 400

    @patch("newchan.server.resample_ohlc")
    @patch("newchan.server.load_df")
    @patch("newchan.server.request")
    def test_bad_to_returns_400(self, mock_req, mock_load, mock_resample):
        import pandas as pd
        df = pd.DataFrame(
            {"open": [1], "high": [2], "low": [0], "close": [1], "volume": [100]},
            index=pd.date_range("2024-01-01", periods=1, freq="1min"),
        )
        mock_req.query = _make_query({
            "symbol": "CL", "interval": "1min", "tf": "1m",
            "to": "abc",
        })
        mock_load.return_value = df
        mock_resample.return_value = df

        from newchan.server import api_ohlcv, response
        result = _parse(api_ohlcv())
        assert "error" in result
        assert response.status_code == 400


class TestServerOverlayParamErrors:
    """api_newchan_overlay 参数无效 → 400。"""

    @patch("newchan.server.request")
    def test_bad_min_strict_sep_returns_400(self, mock_req):
        mock_req.query = _make_query({
            "symbol": "CL", "interval": "1min", "tf": "1m",
            "detail": "full", "segment_algo": "v1",
            "stroke_mode": "wide", "min_strict_sep": "abc",
            "center_sustain_m": "2", "limit": "",
        })
        from newchan.server import api_newchan_overlay, response
        result = _parse(api_newchan_overlay())
        assert "error" in result
        assert response.status_code == 400


class TestServerNestedDivergenceParamErrors:
    """api_nested_divergence 参数无效 → 400。"""

    @patch("newchan.server.request")
    def test_bad_max_levels_returns_400(self, mock_req):
        mock_req.query = _make_query({
            "symbol": "CL", "interval": "1min", "tf": "1m",
            "stroke_mode": "wide", "min_strict_sep": "5",
            "max_levels": "xyz", "limit": "3000",
        })
        from newchan.server import api_nested_divergence, response
        result = _parse(api_nested_divergence())
        assert "error" in result
        assert response.status_code == 400


class TestServerLiveEndpointErrors:
    """Live 端点异常 → 500。"""

    @patch("newchan.server._get_live_feeder", side_effect=RuntimeError("feeder 初始化失败"))
    def test_live_status_error(self, _mock):
        from newchan.server import api_live_status
        result = _parse(api_live_status())
        assert "error" in result

    @patch("newchan.server._get_live_feeder", side_effect=RuntimeError("feeder 初始化失败"))
    def test_live_start_error(self, _mock):
        from newchan.server import api_live_start
        result = _parse(api_live_start())
        assert "error" in result

    @patch("newchan.server._get_live_feeder", side_effect=RuntimeError("feeder 初始化失败"))
    def test_connection_error(self, _mock):
        from newchan.server import api_connection
        result = _parse(api_connection())
        assert "error" in result
        assert result["connected"] is False


class TestServerSearchDegradation:
    """api_search — Databento 搜索失败时降级为仅缓存结果。"""

    @patch("newchan.server.request")
    @patch("newchan.server.list_cached", return_value=[])
    def test_search_degrades_on_databento_error(self, _mock_cached, mock_req):
        mock_req.query = _make_query({"q": "CL"})
        with patch("newchan.data_databento.search_symbols", side_effect=RuntimeError("网络错误")):
            from newchan.server import api_search
            result = _parse(api_search())
        assert isinstance(result, list)
