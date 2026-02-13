"""WebSocket 消息格式 — Pydantic 模型

定义服务端↔客户端之间的所有 WS 消息类型。
所有时间戳使用 epoch 秒（number），与 overlay 坐标系一致。
"""

from __future__ import annotations

from typing import Any, Literal

from pydantic import BaseModel


# ════════════════════════════════════════════════
# 服务端 → 客户端
# ════════════════════════════════════════════════


class WsBar(BaseModel):
    """每根新 bar 的推送数据。"""

    type: Literal["bar"] = "bar"
    idx: int
    ts: float  # epoch 秒
    o: float
    h: float
    l: float
    c: float
    v: float | None = None
    tf: str = ""  # 多 TF 标识（空串 = base TF）
    stream_id: str = ""  # MVP-B0: 流标识（空串 = 未指定）


class WsEvent(BaseModel):
    """域事件推送。"""

    type: Literal["event"] = "event"
    event_type: str  # stroke_candidate / stroke_settled / ...
    bar_idx: int
    bar_ts: float  # epoch 秒
    seq: int
    payload: dict[str, Any]
    event_id: str = ""
    schema_version: int = 1
    tf: str = ""  # 多 TF 标识（空串 = base TF）
    stream_id: str = ""  # MVP-B0: 流标识（空串 = 未指定）


class WsSnapshot(BaseModel):
    """快照消息 — 连接或 seek 后发送完整状态。"""

    type: Literal["snapshot"] = "snapshot"
    bar_idx: int
    strokes: list[dict[str, Any]]
    event_count: int


class WsReplayStatus(BaseModel):
    """回放状态推送。"""

    type: Literal["replay_status"] = "replay_status"
    mode: Literal["idle", "playing", "paused", "done"]
    current_idx: int
    total_bars: int
    speed: float


class WsError(BaseModel):
    """错误消息。"""

    type: Literal["error"] = "error"
    message: str
    code: str = "unknown"


# ════════════════════════════════════════════════
# 客户端 → 服务端
# ════════════════════════════════════════════════


class WsCommand(BaseModel):
    """客户端控制命令。"""

    action: Literal[
        "subscribe",
        "unsubscribe",
        "replay_start",
        "replay_step",
        "replay_seek",
        "replay_play",
        "replay_pause",
    ]
    symbol: str = ""
    tf: str = "5m"
    step_count: int = 1
    seek_idx: int = 0
    speed: float = 1.0


# ════════════════════════════════════════════════
# REST API 模型
# ════════════════════════════════════════════════


class ReplayStartRequest(BaseModel):
    symbol: str
    tf: str = "5m"
    interval: str = "1min"
    stroke_mode: str = "wide"
    min_strict_sep: int = 5
    timeframes: list[str] = []  # 多 TF 列表，空 = 仅 tf 单级别


class ReplayStartResponse(BaseModel):
    session_id: str
    total_bars: int
    timeframes: list[str] = []  # 实际启用的 TF 列表
    status: Literal["ready"] = "ready"


class ReplayStepRequest(BaseModel):
    session_id: str
    count: int = 1


class ReplayStepResponse(BaseModel):
    bar_idx: int
    bar: WsBar | None = None
    events: list[WsEvent] = []


class ReplaySeekRequest(BaseModel):
    session_id: str
    target_idx: int


class ReplaySeekResponse(BaseModel):
    bar_idx: int
    snapshot: WsSnapshot


class ReplayPlayRequest(BaseModel):
    session_id: str
    speed: float = 1.0


class ReplayPauseRequest(BaseModel):
    session_id: str


class ReplayStatusResponse(BaseModel):
    session_id: str
    mode: Literal["idle", "playing", "paused", "done"]
    current_idx: int
    total_bars: int
    speed: float
