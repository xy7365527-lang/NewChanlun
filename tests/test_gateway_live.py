"""Live WebSocket 推送测试。

覆盖：
- /ws/live/{symbol} 连接 → 收到 snapshot
- 多客户端订阅同一标的
- 客户端断连后不再收到消息
- _on_live_bar 回调驱动广播
"""

from __future__ import annotations

import asyncio
from datetime import datetime, timezone
from unittest.mock import patch

import pytest
from fastapi.testclient import TestClient

from newchan.gateway import (
    _ensure_live_engine,
    _live_bar_counts,
    _live_bp_queues,
    _live_bp_tasks,
    _live_clients,
    _live_engines,
    _live_pending_bars,
    _live_snapshots,
    _live_warming,
    _on_live_bar,
    app,
)
from newchan.types import Bar


@pytest.fixture(autouse=True)
def _clean_live_state():
    """每个测试前后清理 live 全局状态。"""
    _live_engines.clear()
    _live_bar_counts.clear()
    _live_clients.clear()
    _live_snapshots.clear()
    _live_bp_queues.clear()
    _live_bp_tasks.clear()
    _live_warming.clear()
    _live_pending_bars.clear()
    yield
    _live_engines.clear()
    _live_bar_counts.clear()
    _live_clients.clear()
    _live_snapshots.clear()
    _live_bp_queues.clear()
    _live_bp_tasks.clear()
    _live_warming.clear()
    _live_pending_bars.clear()


def _make_bar(idx: int, price: float = 100.0) -> Bar:
    """构造测试用 Bar。"""
    return Bar(
        ts=datetime(2025, 1, 1, 0, idx, tzinfo=timezone.utc),
        open=price,
        high=price + 1,
        low=price - 1,
        close=price + 0.5,
        volume=1000,
    )


def _make_bars(n: int) -> list[Bar]:
    """构造 n 根递增价格的 bar。"""
    return [_make_bar(i, price=100.0 + i) for i in range(n)]


class TestWsLiveConnect:
    """测试 /ws/live/{symbol} 连接和 snapshot 推送。"""

    def test_connect_no_cache(self):
        """无缓存数据时连接成功，收到空 snapshot。"""
        client = TestClient(app)
        with patch("newchan.gateway._load_bars", side_effect=ValueError("无缓存")):
            with client.websocket_connect("/ws/live/TEST") as ws:
                msg = ws.receive_json()
                assert msg["type"] == "snapshot"
                assert msg["bar_idx"] == 0
                assert msg["strokes"] == []

    def test_connect_with_cache(self):
        """有缓存数据时连接成功，收到预热后的 snapshot。"""
        bars = _make_bars(30)
        client = TestClient(app)
        with patch("newchan.gateway._load_bars", return_value=bars):
            with client.websocket_connect("/ws/live/BZ") as ws:
                msg = ws.receive_json()
                assert msg["type"] == "snapshot"
                # bar_idx 来自 snapshot，是 0-indexed 的最后一根 bar 索引
                assert msg["bar_idx"] == 29
                # 引擎已注册
                assert "BZ" in _live_engines

    def test_connect_symbol_case_insensitive(self):
        """symbol 大小写不敏感。"""
        client = TestClient(app)
        with patch("newchan.gateway._load_bars", side_effect=ValueError("无缓存")):
            with client.websocket_connect("/ws/live/bz") as ws:
                msg = ws.receive_json()
                assert msg["type"] == "snapshot"
                # 内部存储为大写
                assert "BZ" in _live_engines


class TestWsLiveDisconnect:
    """测试客户端断连清理。"""

    def test_disconnect_removes_client(self):
        """断连后客户端从 _live_clients 中移除。"""
        client = TestClient(app)
        with patch("newchan.gateway._load_bars", side_effect=ValueError("无缓存")):
            with client.websocket_connect("/ws/live/TEST") as ws:
                ws.receive_json()  # snapshot
                assert len(_live_clients.get("TEST", set())) == 1
            # 退出 with 后连接关闭
        assert len(_live_clients.get("TEST", set())) == 0


class TestWsLivePing:
    """测试 ping/pong。"""

    def test_ping_pong(self):
        """客户端发送 ping 收到 pong。"""
        client = TestClient(app)
        with patch("newchan.gateway._load_bars", side_effect=ValueError("无缓存")):
            with client.websocket_connect("/ws/live/TEST") as ws:
                ws.receive_json()  # snapshot
                ws.send_json({"action": "ping"})
                msg = ws.receive_json()
                assert msg["type"] == "pong"


class TestOnLiveBar:
    """测试 _on_live_bar 回调逻辑。"""

    def test_no_engine_noop(self):
        """无引擎时回调不报错。"""
        bar = _make_bar(0)
        _on_live_bar("UNKNOWN", bar)  # 不应抛异常

    def test_engine_processes_bar(self):
        """有引擎时 bar 被处理，bar_count 递增。"""
        from newchan.orchestrator.recursive import RecursiveOrchestrator

        engine = RecursiveOrchestrator(stream_id="live-TEST")
        _live_engines["TEST"] = engine
        _live_bar_counts["TEST"] = 0

        bar = _make_bar(0)
        _on_live_bar("TEST", bar)

        assert _live_bar_counts["TEST"] == 1
        assert "TEST" in _live_snapshots

    def test_multiple_bars_increment(self):
        """连续多根 bar 正确递增计数。"""
        from newchan.orchestrator.recursive import RecursiveOrchestrator

        engine = RecursiveOrchestrator(stream_id="live-TEST")
        _live_engines["TEST"] = engine
        _live_bar_counts["TEST"] = 0

        for i in range(5):
            _on_live_bar("TEST", _make_bar(i, price=100.0 + i))

        assert _live_bar_counts["TEST"] == 5


class TestLiveEngineReuse:
    """测试引擎复用 — 多次连接同一标的不重新创建引擎。"""

    def test_engine_reused(self):
        """第二次连接复用已有引擎。"""
        bars = _make_bars(10)
        client = TestClient(app)
        with patch("newchan.gateway._load_bars", return_value=bars):
            with client.websocket_connect("/ws/live/BZ") as ws:
                ws.receive_json()
            engine_id = id(_live_engines["BZ"])
            with client.websocket_connect("/ws/live/BZ") as ws:
                ws.receive_json()
            assert id(_live_engines["BZ"]) == engine_id


class TestLiveStatusEndpoint:
    """测试 GET /api/live/status 端点。"""

    def test_empty_status(self):
        """无活跃标的时返回空状态。"""
        client = TestClient(app)
        resp = client.get("/api/live/status")
        assert resp.status_code == 200
        data = resp.json()
        assert data["active_symbols"] == 0
        assert data["symbols"] == {}

    def test_status_with_engine(self):
        """有引擎时返回标的状态。"""
        from newchan.orchestrator.recursive import RecursiveOrchestrator

        engine = RecursiveOrchestrator(stream_id="live-TEST")
        _live_engines["TEST"] = engine
        _live_bar_counts["TEST"] = 42

        client = TestClient(app)
        resp = client.get("/api/live/status")
        assert resp.status_code == 200
        data = resp.json()
        assert data["active_symbols"] == 1
        assert data["symbols"]["TEST"]["bar_count"] == 42
        assert data["symbols"]["TEST"]["connected_clients"] == 0

    def test_status_reflects_connected_client(self):
        """有 WS 客户端连接时 connected_clients > 0。"""
        client = TestClient(app)
        with patch("newchan.gateway._load_bars", side_effect=ValueError("无缓存")):
            with client.websocket_connect("/ws/live/TEST") as ws:
                ws.receive_json()  # snapshot
                resp = client.get("/api/live/status")
                data = resp.json()
                assert data["symbols"]["TEST"]["connected_clients"] == 1


class TestBackpressureIntegration:
    """测试背压队列在 WS live 路径中的集成。"""

    def test_bp_queue_created_on_connect(self):
        """WS 连接时创建背压队列。"""
        client = TestClient(app)
        with patch("newchan.gateway._load_bars", side_effect=ValueError("无缓存")):
            with client.websocket_connect("/ws/live/TEST") as ws:
                ws.receive_json()  # snapshot
                assert len(_live_bp_queues) == 1

    def test_bp_queue_cleaned_on_disconnect(self):
        """WS 断连后背压队列被清理。"""
        client = TestClient(app)
        with patch("newchan.gateway._load_bars", side_effect=ValueError("无缓存")):
            with client.websocket_connect("/ws/live/TEST") as ws:
                ws.receive_json()
        assert len(_live_bp_queues) == 0
        assert len(_live_bp_tasks) == 0


class TestLiveWarmupRace:
    """预热期间 feeder bar 不得静默丢失。"""

    def test_bars_during_warmup_are_flushed(self):
        """预热慢路径期间到达的 live bar，结束后必须并入引擎。"""
        import threading
        import time

        warm_bars = _make_bars(15)
        live_bar = _make_bar(30, price=200.0)  # ts 晚于 warm_bars

        def slow_load(*_a, **_k):
            time.sleep(0.15)
            return warm_bars

        def feeder():
            # 等预热进入 warming 状态
            for _ in range(50):
                if "CL" in _live_warming:
                    break
                time.sleep(0.005)
            _on_live_bar("CL", live_bar)

        with patch("newchan.gateway._load_bars", side_effect=slow_load):
            t = threading.Thread(target=feeder)
            t.start()
            engine = _ensure_live_engine("CL")
            t.join(timeout=2.0)

        assert "CL" not in _live_warming
        assert "CL" in _live_engines
        # 预热 15 + 去重后应用 1 根 live bar
        assert _live_bar_counts["CL"] == 16
        assert engine._bi_engine.bar_count == 16  # noqa: SLF001
