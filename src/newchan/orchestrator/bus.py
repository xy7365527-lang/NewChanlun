"""EventBus — 按 TF 分区的事件收集器

从各 TF 的 BiEngine 快照中收集事件，附加 TF 标签，
支持按 TF 或全局排序取出。
"""

from __future__ import annotations

from dataclasses import dataclass

from newchan.events import DomainEvent


@dataclass(frozen=True, slots=True)
class TaggedEvent:
    """带 TF / stream_id 标签的事件。"""

    tf: str
    event: DomainEvent
    stream_id: str = ""  # MVP-B0: 流标识（空串 = 未指定）


class EventBus:
    """按 TF / stream_id 分区的事件收集器。

    Usage::

        bus = EventBus()
        bus.push("5m", snapshot_5m.events, stream_id="CME:BZ/1min@5m:L0/replay")
        bus.push("30m", snapshot_30m.events, stream_id="CME:BZ/1min@30m:L0/replay")
        all_events = bus.drain()       # 取出全部并清空
        tf_events = bus.drain_by_tf("5m")  # 仅取指定 TF
        stream_events = bus.drain_by_stream("CME:BZ/1min@5m:L0/replay")
    """

    def __init__(self) -> None:
        self._events: list[TaggedEvent] = []

    def push(self, tf: str, events: list[DomainEvent], stream_id: str = "") -> None:
        """添加一批事件，标记所属 TF 和 stream_id。"""
        for ev in events:
            self._events.append(TaggedEvent(tf=tf, event=ev, stream_id=stream_id))

    def drain(self) -> list[TaggedEvent]:
        """取出全部事件并清空缓冲。"""
        result = list(self._events)
        self._events.clear()
        return result

    def drain_by_tf(self, tf: str) -> list[DomainEvent]:
        """取出指定 TF 的事件，保留其它 TF 事件。"""
        matched: list[DomainEvent] = []
        remaining: list[TaggedEvent] = []
        for te in self._events:
            if te.tf == tf:
                matched.append(te.event)
            else:
                remaining.append(te)
        self._events = remaining
        return matched

    def drain_by_stream(self, stream_id: str) -> list[DomainEvent]:
        """取出指定 stream_id 的事件，保留其它事件。"""
        matched: list[DomainEvent] = []
        remaining: list[TaggedEvent] = []
        for te in self._events:
            if te.stream_id == stream_id:
                matched.append(te.event)
            else:
                remaining.append(te)
        self._events = remaining
        return matched

    @property
    def count(self) -> int:
        """当前缓冲中的事件数。"""
        return len(self._events)
