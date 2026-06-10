"""gateway.py 多 TF 回放端点测试。

覆盖：
  - 多 TF 会话创建（POST /api/replay/start + timeframes）
  - 多 TF 步进（POST /api/replay/step）返回正确快照
  - 多 TF seek 重置（POST /api/replay/seek）
  - WsSnapshot 字段完整性
  - 边界情况：单 TF 退化、空数据
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone
from unittest.mock import patch

import pytest
import pandas as pd
from fastapi.testclient import TestClient

from newchan.gateway import app, _load_bars, _sessions, _orchestrators
from newchan.types import Bar


# ── 辅助 ──────────────────────────────────────────────────────


def _make_bars(n: int = 120) -> list[Bar]:
    """生成 n 根 1min 锯齿形 bars（16-bar 周期）。"""
    bars: list[Bar] = []
    base_time = datetime(2024, 1, 1, tzinfo=timezone.utc)
    for i in range(n):
        cycle_pos = i % 16
        if cycle_pos < 8:
            base = 100 - cycle_pos * 5
        else:
            base = 60 + (cycle_pos - 8) * 5
        bars.append(Bar(
            ts=base_time + timedelta(minutes=i),
            open=base + 0.5,
            high=base + 1.5,
            low=base - 1.5,
            close=base - 0.5,
        ))
    return bars


@pytest.fixture(autouse=True)
def _clean_gateway_state():
    """每个测试前后清理 gateway 全局状态。"""
    _sessions.clear()
    _orchestrators.clear()
    yield
    _sessions.clear()
    _orchestrators.clear()


@pytest.fixture()
def client():
    return TestClient(app)


@pytest.fixture()
def mock_load_bars():
    """mock _load_bars 返回合成数据。"""
    bars = _make_bars(120)
    with patch("newchan.gateway._load_bars", return_value=bars) as m:
        m._bars = bars
        yield m


# =====================================================================
# 多 TF 会话创建
# =====================================================================


class TestMultiTFSessionCreate:
    """POST /api/replay/start 多 TF 路径。"""

    def test_creates_orchestrator(self, client, mock_load_bars):
        """timeframes 含多个 TF → 创建 TFOrchestrator。"""
        resp = client.post("/api/replay/start", json={
            "symbol": "TEST",
            "tf": "5m",
            "timeframes": ["5m", "30m"],
        })
        assert resp.status_code == 200
        body = resp.json()
        assert body["status"] == "ready"
        assert body["total_bars"] == 120
        assert body["timeframes"] == ["5m", "30m"]

        sid = body["session_id"]
        assert sid in _orchestrators
        assert sid in _sessions

    def test_rejects_timeframe_mismatch_before_loading(self, client):
        """timeframes[0] 必须与 tf 一致，避免 base TF 标签污染。"""
        with patch("newchan.gateway._load_bars") as load_bars:
            resp = client.post("/api/replay/start", json={
                "symbol": "TEST",
                "tf": "1m",
                "timeframes": ["5m", "30m"],
            })

        assert resp.status_code == 400
        assert load_bars.call_count == 0
        assert _sessions == {}
        assert _orchestrators == {}

    def test_single_tf_no_orchestrator(self, client, mock_load_bars):
        """timeframes 仅一个 TF → 不创建 TFOrchestrator。"""
        resp = client.post("/api/replay/start", json={
            "symbol": "TEST",
            "tf": "5m",
            "timeframes": ["5m"],
        })
        body = resp.json()
        sid = body["session_id"]
        assert sid not in _orchestrators
        assert sid in _sessions

    def test_empty_timeframes_uses_tf(self, client, mock_load_bars):
        """timeframes 为空 → 退化为单 TF（使用 tf 字段）。"""
        resp = client.post("/api/replay/start", json={
            "symbol": "TEST",
            "tf": "5m",
        })
        body = resp.json()
        assert body["timeframes"] == ["5m"]
        assert body["session_id"] not in _orchestrators

    def test_empty_data_returns_error(self):
        """数据为空 → 返回 404。"""
        with patch("newchan.gateway._load_bars", return_value=[]):
            c = TestClient(app, raise_server_exceptions=False)
            resp = c.post("/api/replay/start", json={
                "symbol": "EMPTY",
                "tf": "5m",
                "timeframes": ["5m", "30m"],
            })
        assert resp.status_code == 404

    def test_load_failure_returns_error(self):
        """_load_bars 抛异常 → 返回 404。"""
        with patch("newchan.gateway._load_bars", side_effect=ValueError("缓存不存在")):
            c = TestClient(app, raise_server_exceptions=False)
            resp = c.post("/api/replay/start", json={
                "symbol": "MISSING",
                "tf": "5m",
            })
        assert resp.status_code == 404

    def test_load_bars_propagates_resample_errors(self):
        """非法目标周期不能静默回退到原始 interval 数据。"""
        df = pd.DataFrame(
            {
                "open": [1.0, 2.0],
                "high": [2.0, 3.0],
                "low": [0.5, 1.5],
                "close": [1.5, 2.5],
            },
            index=pd.date_range("2024-01-01", periods=2, freq="1min"),
        )
        with patch("newchan.cache.load_df", return_value=df):
            with pytest.raises(ValueError, match="不支持"):
                _load_bars("TEST", "1min", "3m")


# =====================================================================
# 多 TF 步进
# =====================================================================


class TestMultiTFStep:
    """POST /api/replay/step 多 TF 路径。"""

    def _create_session(self, client, mock_load_bars, timeframes=None):
        """创建会话并返回 session_id。"""
        if timeframes is None:
            timeframes = ["5m", "30m"]
        resp = client.post("/api/replay/start", json={
            "symbol": "TEST",
            "tf": "5m",
            "timeframes": timeframes,
        })
        return resp.json()["session_id"]

    def test_step_returns_bar_idx(self, client, mock_load_bars):
        """步进 1 → 返回 bar_idx。"""
        sid = self._create_session(client, mock_load_bars)
        resp = client.post("/api/replay/step", json={
            "session_id": sid,
            "count": 1,
        })
        assert resp.status_code == 200
        body = resp.json()
        assert "bar_idx" in body

    def test_step_returns_bar(self, client, mock_load_bars):
        """步进 → 返回 WsBar。"""
        sid = self._create_session(client, mock_load_bars)
        resp = client.post("/api/replay/step", json={
            "session_id": sid,
            "count": 1,
        })
        body = resp.json()
        bar = body.get("bar")
        assert bar is not None
        assert "idx" in bar
        assert "o" in bar
        assert "h" in bar
        assert "l" in bar
        assert "c" in bar

    def test_step_multiple(self, client, mock_load_bars):
        """步进 10 → current_idx 前进 10。"""
        sid = self._create_session(client, mock_load_bars)
        client.post("/api/replay/step", json={
            "session_id": sid,
            "count": 10,
        })
        orch = _orchestrators[sid]
        assert orch.current_idx == 10

    def test_step_events_have_tf_tag(self, client, mock_load_bars):
        """步进足够多 bar 后，events 应携带 tf 标签。"""
        sid = self._create_session(client, mock_load_bars)
        # 步进足够多以产生事件
        resp = client.post("/api/replay/step", json={
            "session_id": sid,
            "count": 60,
        })
        body = resp.json()
        events = body.get("events", [])
        for ev in events:
            assert "tf" in ev
            assert ev["tf"] in ("5m", "30m", "")

    def test_step_single_tf_degradation(self, client, mock_load_bars):
        """单 TF 会话步进走原有路径。"""
        sid = self._create_session(client, mock_load_bars, timeframes=["5m"])
        resp = client.post("/api/replay/step", json={
            "session_id": sid,
            "count": 5,
        })
        assert resp.status_code == 200
        body = resp.json()
        assert "bar_idx" in body

    def test_step_invalid_session(self):
        """不存在的 session_id → 返回 404。"""
        c = TestClient(app, raise_server_exceptions=False)
        resp = c.post("/api/replay/step", json={
            "session_id": "nonexistent",
            "count": 1,
        })
        assert resp.status_code == 404


# =====================================================================
# 多 TF Seek
# =====================================================================


class TestMultiTFSeek:
    """POST /api/replay/seek 多 TF 路径。"""

    def _create_and_step(self, client, mock_load_bars, step_count=30):
        """创建多 TF 会话并步进。"""
        resp = client.post("/api/replay/start", json={
            "symbol": "TEST",
            "tf": "5m",
            "timeframes": ["5m", "30m"],
        })
        sid = resp.json()["session_id"]
        client.post("/api/replay/step", json={
            "session_id": sid,
            "count": step_count,
        })
        return sid

    def test_seek_returns_snapshot(self, client, mock_load_bars):
        """seek → 返回 WsSnapshot。"""
        sid = self._create_and_step(client, mock_load_bars)
        resp = client.post("/api/replay/seek", json={
            "session_id": sid,
            "target_idx": 20,
        })
        assert resp.status_code == 200
        body = resp.json()
        assert "snapshot" in body
        snap = body["snapshot"]
        assert snap["type"] == "snapshot"
        assert "bar_idx" in snap
        assert "strokes" in snap
        assert "event_count" in snap

    def test_seek_resets_position(self, client, mock_load_bars):
        """seek 后 current_idx 对应 target。"""
        sid = self._create_and_step(client, mock_load_bars, step_count=50)
        client.post("/api/replay/seek", json={
            "session_id": sid,
            "target_idx": 10,
        })
        orch = _orchestrators[sid]
        # seek(10) → current_idx = 11（已处理 0..10）
        assert orch.current_idx == 11

    def test_seek_to_zero(self, client, mock_load_bars):
        """seek(0) → 重置到起始。"""
        sid = self._create_and_step(client, mock_load_bars)
        resp = client.post("/api/replay/seek", json={
            "session_id": sid,
            "target_idx": 0,
        })
        body = resp.json()
        assert body["bar_idx"] == 0

    def test_seek_consistency_with_step(self, client, mock_load_bars):
        """seek(N) 的快照 strokes === 从头 step(N+1) 的 strokes。"""
        bars = mock_load_bars._bars

        # 路径 1：step 到 bar 40
        resp1 = client.post("/api/replay/start", json={
            "symbol": "TEST", "tf": "5m", "timeframes": ["5m", "30m"],
        })
        sid1 = resp1.json()["session_id"]
        client.post("/api/replay/step", json={
            "session_id": sid1, "count": 40,
        })
        strokes_step = [
            (s.i0, s.i1, s.direction)
            for s in _orchestrators[sid1].sessions["5m"].engine._bi_engine.current_strokes
        ]

        # 路径 2：seek 到 bar 39
        resp2 = client.post("/api/replay/start", json={
            "symbol": "TEST", "tf": "5m", "timeframes": ["5m", "30m"],
        })
        sid2 = resp2.json()["session_id"]
        client.post("/api/replay/seek", json={
            "session_id": sid2, "target_idx": 39,
        })
        strokes_seek = [
            (s.i0, s.i1, s.direction)
            for s in _orchestrators[sid2].sessions["5m"].engine._bi_engine.current_strokes
        ]

        assert strokes_step == strokes_seek

    def test_seek_invalid_session(self):
        """不存在的 session_id → 返回 404。"""
        c = TestClient(app, raise_server_exceptions=False)
        resp = c.post("/api/replay/seek", json={
            "session_id": "nonexistent",
            "target_idx": 10,
        })
        assert resp.status_code == 404


# =====================================================================
# WsSnapshot 字段完整性
# =====================================================================


class TestWsSnapshotFields:
    """验证 WsSnapshot 消息包含全链路字段。"""

    def test_snapshot_has_required_fields(self, client, mock_load_bars):
        """seek 返回的 snapshot 包含 type/bar_idx/strokes/event_count。"""
        resp = client.post("/api/replay/start", json={
            "symbol": "TEST", "tf": "5m", "timeframes": ["5m", "30m"],
        })
        sid = resp.json()["session_id"]
        # 步进足够多以产生 strokes
        client.post("/api/replay/step", json={
            "session_id": sid, "count": 60,
        })
        resp = client.post("/api/replay/seek", json={
            "session_id": sid, "target_idx": 59,
        })
        snap = resp.json()["snapshot"]
        assert snap["type"] == "snapshot"
        assert isinstance(snap["bar_idx"], int)
        assert isinstance(snap["strokes"], list)
        assert isinstance(snap["event_count"], int)

    def test_snapshot_strokes_structure(self, client, mock_load_bars):
        """snapshot.strokes 中每个 stroke 包含 i0/i1/direction/confirmed。"""
        resp = client.post("/api/replay/start", json={
            "symbol": "TEST", "tf": "5m", "timeframes": ["5m", "30m"],
        })
        sid = resp.json()["session_id"]
        client.post("/api/replay/step", json={
            "session_id": sid, "count": 60,
        })
        resp = client.post("/api/replay/seek", json={
            "session_id": sid, "target_idx": 59,
        })
        snap = resp.json()["snapshot"]
        if snap["strokes"]:
            stroke = snap["strokes"][0]
            assert "i0" in stroke
            assert "i1" in stroke
            assert "direction" in stroke
            assert "confirmed" in stroke


# =====================================================================
# 边界情况
# =====================================================================


class TestEdgeCases:
    """边界情况测试。"""

    def test_step_beyond_end(self, client, mock_load_bars):
        """步进超过总 bar 数 → 不崩溃，mode 变为 done。"""
        resp = client.post("/api/replay/start", json={
            "symbol": "TEST", "tf": "5m", "timeframes": ["5m", "30m"],
        })
        sid = resp.json()["session_id"]
        total = resp.json()["total_bars"]
        # 步进超过总数
        resp = client.post("/api/replay/step", json={
            "session_id": sid, "count": total + 10,
        })
        assert resp.status_code == 200
        session = _sessions[sid]
        assert session.mode == "done"

    def test_seek_beyond_end(self, client, mock_load_bars):
        """seek 超过总 bar 数 → 不崩溃。"""
        resp = client.post("/api/replay/start", json={
            "symbol": "TEST", "tf": "5m", "timeframes": ["5m", "30m"],
        })
        sid = resp.json()["session_id"]
        total = resp.json()["total_bars"]
        resp = client.post("/api/replay/seek", json={
            "session_id": sid, "target_idx": total + 100,
        })
        assert resp.status_code == 200

    def test_three_tf_session(self, client, mock_load_bars):
        """三级别 TF 会话创建和步进。"""
        resp = client.post("/api/replay/start", json={
            "symbol": "TEST", "tf": "5m",
            "timeframes": ["5m", "15m", "30m"],
        })
        assert resp.status_code == 200
        body = resp.json()
        sid = body["session_id"]
        assert len(body["timeframes"]) == 3

        resp = client.post("/api/replay/step", json={
            "session_id": sid, "count": 30,
        })
        assert resp.status_code == 200
        orch = _orchestrators[sid]
        assert "15m" in orch.sessions
        assert "30m" in orch.sessions
