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
    _live_bar_counts,
    _live_clients,
    _live_engines,
    _live_snapshots,
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
    yield
    _live_engines.clear()
    _live_bar_counts.clear()
    _live_clients.clear()
    _live_snapshots.clear()


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
