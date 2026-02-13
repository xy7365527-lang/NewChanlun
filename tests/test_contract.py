"""事件 schema 契约测试

验证：
1. 所有事件 dataclass 的字段完整性
2. event_type 默认值正确
3. WebSocket 消息模型可序列化
"""

from __future__ import annotations

import dataclasses

import pytest

from newchan.contracts.ws_messages import WsBar, WsCommand, WsEvent, WsSnapshot
from newchan.events import (
    DomainEvent,
    StrokeCandidate,
    StrokeExtended,
    StrokeInvalidated,
    StrokeSettled,
)


# ── 所有事件子类 ──

_EVENT_CLASSES = [StrokeCandidate, StrokeSettled, StrokeExtended, StrokeInvalidated]


# =====================================================================
# 事件 dataclass 契约
# =====================================================================


class TestEventContract:
    """验证事件 dataclass 字段完整性。"""

    def test_all_events_have_base_fields(self):
        """所有事件子类都包含 DomainEvent 的基础字段。"""
        for cls in _EVENT_CLASSES:
            fields = {f.name for f in dataclasses.fields(cls)}
            assert "event_type" in fields, f"{cls.__name__} missing event_type"
            assert "bar_idx" in fields, f"{cls.__name__} missing bar_idx"
            assert "bar_ts" in fields, f"{cls.__name__} missing bar_ts"
            assert "seq" in fields, f"{cls.__name__} missing seq"

    def test_event_type_values(self):
        """event_type 字段的默认值正确。"""
        assert (
            StrokeCandidate(bar_idx=0, bar_ts=0.0, seq=0).event_type
            == "stroke_candidate"
        )
        assert (
            StrokeSettled(bar_idx=0, bar_ts=0.0, seq=0).event_type
            == "stroke_settled"
        )
        assert (
            StrokeExtended(bar_idx=0, bar_ts=0.0, seq=0).event_type
            == "stroke_extended"
        )
        assert (
            StrokeInvalidated(bar_idx=0, bar_ts=0.0, seq=0).event_type
            == "stroke_invalidated"
        )

    def test_events_are_frozen(self):
        """所有事件都是 frozen dataclass（不可变）。"""
        for cls in _EVENT_CLASSES:
            assert dataclasses.fields(cls)  # 是 dataclass
            e = cls(bar_idx=0, bar_ts=0.0, seq=0)
            with pytest.raises(dataclasses.FrozenInstanceError):
                e.bar_idx = 99  # type: ignore[misc]

    def test_stroke_candidate_extra_fields(self):
        """StrokeCandidate 包含笔相关字段。"""
        fields = {f.name for f in dataclasses.fields(StrokeCandidate)}
        for name in ["stroke_id", "direction", "i0", "i1", "p0", "p1"]:
            assert name in fields, f"StrokeCandidate missing {name}"

    def test_stroke_settled_extra_fields(self):
        """StrokeSettled 包含笔相关字段。"""
        fields = {f.name for f in dataclasses.fields(StrokeSettled)}
        for name in ["stroke_id", "direction", "i0", "i1", "p0", "p1"]:
            assert name in fields, f"StrokeSettled missing {name}"

    def test_stroke_extended_extra_fields(self):
        """StrokeExtended 包含延伸前后的终点字段。"""
        fields = {f.name for f in dataclasses.fields(StrokeExtended)}
        for name in ["stroke_id", "direction", "old_i1", "new_i1", "old_p1", "new_p1"]:
            assert name in fields, f"StrokeExtended missing {name}"

    def test_stroke_invalidated_extra_fields(self):
        """StrokeInvalidated 包含被否定笔的字段。"""
        fields = {f.name for f in dataclasses.fields(StrokeInvalidated)}
        for name in ["stroke_id", "direction", "i0", "i1", "p0", "p1"]:
            assert name in fields, f"StrokeInvalidated missing {name}"

    def test_domain_event_is_base(self):
        """所有事件类都继承自 DomainEvent。"""
        for cls in _EVENT_CLASSES:
            assert issubclass(cls, DomainEvent)


# =====================================================================
# WebSocket 消息模型契约
# =====================================================================


class TestWsContract:
    """验证 WS 消息 Pydantic 模型可序列化。"""

    def test_ws_bar_serializable(self):
        """WsBar 可序列化并包含正确的 type 字段。"""
        bar = WsBar(idx=0, ts=1707800000.0, o=78.5, h=79.1, l=78.2, c=78.8)
        d = bar.model_dump()
        assert d["type"] == "bar"
        assert d["idx"] == 0
        assert d["ts"] == 1707800000.0
        assert d["o"] == 78.5
        assert d["h"] == 79.1
        assert d["l"] == 78.2
        assert d["c"] == 78.8
        assert d["v"] is None

    def test_ws_bar_with_volume(self):
        """WsBar 带 volume 字段。"""
        bar = WsBar(idx=1, ts=1707800300.0, o=79.0, h=80.0, l=78.5, c=79.5, v=12345.0)
        d = bar.model_dump()
        assert d["v"] == 12345.0

    def test_ws_event_serializable(self):
        """WsEvent 可序列化。"""
        event = WsEvent(
            event_type="stroke_settled",
            bar_idx=0,
            bar_ts=1707800000.0,
            seq=0,
            payload={"stroke_id": 1, "direction": "up"},
        )
        d = event.model_dump()
        assert d["type"] == "event"
        assert d["event_type"] == "stroke_settled"
        assert d["bar_idx"] == 0
        assert d["seq"] == 0
        assert d["payload"]["stroke_id"] == 1

    def test_ws_snapshot_serializable(self):
        """WsSnapshot 可序列化。"""
        snapshot = WsSnapshot(
            bar_idx=10,
            strokes=[
                {"i0": 0, "i1": 5, "direction": "up"},
                {"i0": 5, "i1": 10, "direction": "down"},
            ],
            event_count=5,
        )
        d = snapshot.model_dump()
        assert d["type"] == "snapshot"
        assert d["bar_idx"] == 10
        assert len(d["strokes"]) == 2
        assert d["event_count"] == 5

    def test_ws_command_serializable(self):
        """WsCommand 可序列化。"""
        cmd = WsCommand(action="replay_step", step_count=5)
        d = cmd.model_dump()
        assert d["action"] == "replay_step"
        assert d["step_count"] == 5

    def test_ws_command_actions(self):
        """WsCommand 的 action 字段只接受有效值。"""
        valid_actions = [
            "subscribe",
            "unsubscribe",
            "replay_start",
            "replay_step",
            "replay_seek",
            "replay_play",
            "replay_pause",
        ]
        for action in valid_actions:
            cmd = WsCommand(action=action)
            assert cmd.action == action

    def test_ws_event_roundtrip(self):
        """WsEvent 可以 dump -> parse 往返。"""
        event = WsEvent(
            event_type="stroke_candidate",
            bar_idx=5,
            bar_ts=1707800000.0,
            seq=42,
            payload={"i0": 0, "i1": 5, "p0": 10.0, "p1": 20.0},
        )
        json_str = event.model_dump_json()
        parsed = WsEvent.model_validate_json(json_str)
        assert parsed.event_type == event.event_type
        assert parsed.bar_idx == event.bar_idx
        assert parsed.seq == event.seq
        assert parsed.payload == event.payload
