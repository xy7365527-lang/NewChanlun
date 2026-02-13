"""EventEnvelopeV1 — 事件信封

在 DomainEvent 之上的传输包装层。
不替代 DomainEvent，而是提供跨流关联与溯源能力。

关键设计：
- event_id 直接透传自 DomainEvent（不重新计算）
- parents 不参与 event_id 计算（保护确定性红线 R3）
- event 字段是 DomainEvent 引用（不参与序列化/哈希）
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True, slots=True)
class EventEnvelopeV1:
    """事件信封 V1。

    Attributes
    ----------
    schema_version : int
        信封 schema 版本（= 1）。
    event_id : str
        原始事件的确定性 ID（来自 DomainEvent.event_id）。
    stream_id : str
        事件所属流（StreamId.value）。
    bar_time : float
        触发 bar 的时间（epoch 秒）。
    seq : int
        流内事件序号。
    subject_id : str
        事件主题标识符（如 "stroke:3"）。
    parents : tuple[str, ...]
        父事件 event_id 列表（provenance 溯源）。
    provenance : str
        来源描述（如 "bi_differ:v1"）。
    event : Any
        原始 DomainEvent 引用（不参与序列化/哈希）。
    """

    schema_version: int = 1
    event_id: str = ""
    stream_id: str = ""
    bar_time: float = 0.0
    seq: int = 0
    subject_id: str = ""
    parents: tuple[str, ...] = ()
    provenance: str = ""
    event: Any = None
