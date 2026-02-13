"""Provenance 脚手架 — 事件溯源工具

提供将 DomainEvent 包装为 EventEnvelopeV1 的工具函数。
MVP-B0 阶段 parents 始终为空 tuple，脚手架为后续
线段/中枢事件的 parents 挂接预留。
"""

from __future__ import annotations

from newchan.core.envelope import EventEnvelopeV1
from newchan.events import DomainEvent


def wrap_event(
    event: DomainEvent,
    stream_id: str = "",
    parents: tuple[str, ...] = (),
    provenance: str = "",
    subject_id: str = "",
) -> EventEnvelopeV1:
    """将 DomainEvent 包装为 EventEnvelopeV1。

    Parameters
    ----------
    event : DomainEvent
        原始域事件。
    stream_id : str
        所属流标识（StreamId.value）。
    parents : tuple[str, ...]
        父事件 event_id 列表。
    provenance : str
        来源描述（如 "bi_differ:v1"）。
    subject_id : str
        事件主题标识（如 "stroke:3"）。
    """
    return EventEnvelopeV1(
        schema_version=event.schema_version,
        event_id=event.event_id,
        stream_id=stream_id,
        bar_time=event.bar_ts,
        seq=event.seq,
        subject_id=subject_id,
        parents=parents,
        provenance=provenance,
        event=event,
    )


def make_subject_id(event: DomainEvent) -> str:
    """从笔事件中提取 subject_id。

    格式: "stroke:{stroke_id}" 或 "event:{event_type}"
    """
    stroke_id = getattr(event, "stroke_id", None)
    if stroke_id is not None:
        return f"stroke:{stroke_id}"
    return f"event:{event.event_type}"
