"""事件指纹 — 确定性 event_id 与事件流完整性校验

event_id 设计原则：
- 确定性：同输入 → 同 event_id（无 UUID、无时间戳）
- 内容寻址：基于事件所有语义字段的 canonical JSON 哈希
- 紧凑性：16 hex = 64-bit，碰撞概率可忽略
"""

from __future__ import annotations

import hashlib
import json


def compute_event_id(
    bar_idx: int,
    bar_ts: float,
    event_type: str,
    seq: int,
    payload: dict,
) -> str:
    """计算确定性 event_id。

    Parameters
    ----------
    bar_idx : int
        触发事件的 bar 索引。
    bar_ts : float
        触发事件的 bar 时间戳（epoch 秒）。
    event_type : str
        事件类型标识符。
    seq : int
        全局单调递增事件序号。
    payload : dict
        事件的类型特定字段（如 stroke_id, direction 等）。

    Returns
    -------
    str
        16 hex chars = sha256(canonical_json)[:16]
    """
    canonical = json.dumps(
        {
            "bar_idx": bar_idx,
            "bar_ts": bar_ts,
            "event_type": event_type,
            "seq": seq,
            **payload,
        },
        sort_keys=True,
        default=str,
    )
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]


def compute_stream_fingerprint(events: list) -> str:
    """计算事件流指纹 — 验证完整性（无丢失/乱序）。

    Parameters
    ----------
    events : list
        按 seq 排序的 DomainEvent 列表（需有 event_id 属性）。

    Returns
    -------
    str
        32 hex chars = sha256(event_id_0:event_id_1:...)[:32]
    """
    parts = ":".join(ev.event_id for ev in events)
    return hashlib.sha256(parts.encode("utf-8")).hexdigest()[:32]


def compute_envelope_id(
    event_id: str,
    stream_id: str,
    parents: tuple[str, ...] = (),
) -> str:
    """计算事件信封 ID — 包含流归属和溯源信息。

    envelope_id = sha256(event_id + stream_id + sorted(parents))[:16]

    注意：envelope_id 用于跨流关联和审计，不替代 event_id。
    event_id 仍然是事件的唯一语义标识符。

    Parameters
    ----------
    event_id : str
        原始事件的 event_id。
    stream_id : str
        事件所属流（StreamId.value）。
    parents : tuple[str, ...]
        父事件 event_id 列表。

    Returns
    -------
    str
        16 hex chars
    """
    canonical = json.dumps(
        {
            "event_id": event_id,
            "stream_id": stream_id,
            "parents": sorted(parents),
        },
        sort_keys=True,
    )
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]
