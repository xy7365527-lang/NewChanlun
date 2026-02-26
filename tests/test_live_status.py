"""GET /api/live/status 端点测试 — 验证 DatabentoLiveFeeder.status() 输出结构。

覆盖：空状态、单标的、多标的、断线状态。
不依赖 Databento SDK 网络连接，直接操作 feeder 内部状态。
"""

from __future__ import annotations

import time
from unittest.mock import patch

import pytest


def _make_feeder(symbols: list[str] | None = None):
    """构造 DatabentoLiveFeeder，跳过 Databento SDK 导入。"""
    with patch("newchan.data_databento_live.DATABENTO_API_KEY", "fake-key"):
        from newchan.data_databento_live import DatabentoLiveFeeder
        return DatabentoLiveFeeder(symbols=symbols or [])


class TestLiveStatusEmpty:
    """空状态 — feeder 已创建但未启动，无数据流入。"""

    def test_default_feeder_status_structure(self):
        feeder = _make_feeder(["ES"])
        s = feeder.status()

        assert s["running"] is False
        assert len(s["symbols"]) == 1
        assert s["bar_count"] == 0
        assert s["reconnect_count"] == 0
        assert s["last_error"] is None
        # 唯一 symbol 处于 disconnected
        assert s["symbols"]["ES"]["connection"] == "disconnected"
        assert s["symbols"]["ES"]["last_update_ts"] is None


class TestLiveStatusSingleSymbol:
    """单标的状态。"""

    def test_initial_disconnected(self):
        feeder = _make_feeder(["ES"])
        s = feeder.status()

        assert "ES" in s["symbols"]
        es = s["symbols"]["ES"]
        assert es["connection"] == "disconnected"
        assert es["last_update_ts"] is None
        assert es["reconnect_count"] == 0
        assert es["queue_depth"] == 0

    def test_connected_after_simulated_start(self):
        feeder = _make_feeder(["ES"])
        # 模拟 start 中的状态变更（不启动真实线程）
        feeder._running = True
        for sym in feeder._symbols:
            feeder._symbol_states[sym]["connection"] = "connected"

        s = feeder.status()
        assert s["running"] is True
        assert s["symbols"]["ES"]["connection"] == "connected"

    def test_last_update_ts_after_bar(self):
        feeder = _make_feeder(["ES"])
        feeder._running = True
        feeder._symbol_states["ES"]["connection"] = "connected"

        now = time.time()
        feeder._symbol_states["ES"]["last_update_ts"] = now

        s = feeder.status()
        assert s["symbols"]["ES"]["last_update_ts"] == pytest.approx(now, abs=1.0)


class TestLiveStatusMultiSymbol:
    """多标的状态。"""

    def test_multiple_symbols_independent(self):
        feeder = _make_feeder(["ES", "NQ", "CL"])
        s = feeder.status()

        assert len(s["symbols"]) == 3
        for sym in ("ES", "NQ", "CL"):
            assert sym in s["symbols"]
            assert s["symbols"][sym]["connection"] == "disconnected"

    def test_mixed_connection_states(self):
        feeder = _make_feeder(["ES", "NQ"])
        feeder._running = True
        feeder._symbol_states["ES"]["connection"] = "connected"
        feeder._symbol_states["ES"]["last_update_ts"] = time.time()
        # NQ 保持 disconnected

        s = feeder.status()
        assert s["symbols"]["ES"]["connection"] == "connected"
        assert s["symbols"]["ES"]["last_update_ts"] is not None
        assert s["symbols"]["NQ"]["connection"] == "disconnected"
        assert s["symbols"]["NQ"]["last_update_ts"] is None

    def test_per_symbol_reconnect_count(self):
        feeder = _make_feeder(["ES", "NQ"])
        feeder._symbol_states["ES"]["reconnect_count"] = 3
        feeder._symbol_states["NQ"]["reconnect_count"] = 0

        s = feeder.status()
        assert s["symbols"]["ES"]["reconnect_count"] == 3
        assert s["symbols"]["NQ"]["reconnect_count"] == 0


class TestLiveStatusDisconnected:
    """断线状态。"""

    def test_stop_marks_all_disconnected(self):
        feeder = _make_feeder(["ES", "NQ"])
        feeder._running = True
        for sym in feeder._symbols:
            feeder._symbol_states[sym]["connection"] = "connected"

        feeder.stop()
        s = feeder.status()

        assert s["running"] is False
        for sym in ("ES", "NQ"):
            assert s["symbols"][sym]["connection"] == "disconnected"

    def test_error_preserved_after_stop(self):
        feeder = _make_feeder(["ES"])
        feeder._running = True
        feeder._symbol_states["ES"]["connection"] = "connected"
        feeder._last_error = "connection reset"

        feeder.stop()
        s = feeder.status()

        assert s["last_error"] == "connection reset"
        assert s["symbols"]["ES"]["connection"] == "disconnected"

    def test_reconnecting_state(self):
        feeder = _make_feeder(["ES"])
        feeder._symbol_states["ES"]["connection"] = "reconnecting"
        feeder._symbol_states["ES"]["reconnect_count"] = 2

        s = feeder.status()
        assert s["symbols"]["ES"]["connection"] == "reconnecting"
        assert s["symbols"]["ES"]["reconnect_count"] == 2
