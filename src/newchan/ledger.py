"""最小信号日志 — append-only，记录 BiEngine 域事件。

MVP-0：仅记录 StrokeSettled / StrokeInvalidated，不做仓位/下单/风控。
"""
from __future__ import annotations

from dataclasses import asdict, dataclass

from newchan.events import DomainEvent, StrokeInvalidated, StrokeSettled


@dataclass(frozen=True, slots=True)
class SignalRecord:
    """单条信号记录。"""
    event_type: str
    bar_idx: int
    bar_ts: float
    seq: int
    direction: str
    price: float
    stroke_id: int


_RECORDABLE = (StrokeSettled, StrokeInvalidated)


class SignalLedger:
    """最小信号日志 — append-only。"""

    def __init__(self) -> None:
        self._records: list[SignalRecord] = []

    def process_event(self, event: DomainEvent) -> SignalRecord | None:
        """处理一个域事件；可记录则追加并返回，否则返回 None。"""
        if not isinstance(event, _RECORDABLE):
            return None
        rec = SignalRecord(
            event_type=event.event_type,
            bar_idx=event.bar_idx,
            bar_ts=event.bar_ts,
            seq=event.seq,
            direction=event.direction,
            price=event.p1,
            stroke_id=event.stroke_id,
        )
        self._records.append(rec)
        return rec

    @property
    def records(self) -> list[SignalRecord]:
        """所有信号记录（只读副本）。"""
        return list(self._records)

    @property
    def count(self) -> int:
        return len(self._records)

    def reset(self) -> None:
        """清空日志（回放 seek 时用）。"""
        self._records.clear()

    def to_dicts(self) -> list[dict]:
        """导出为 JSON 兼容的 dict 列表。"""
        return [asdict(r) for r in self._records]

    def summary(self) -> dict:
        """统计摘要：总数、settle/invalidate 数、方向分布。"""
        s = i = u = d = 0
        for r in self._records:
            if r.event_type == "stroke_settled":
                s += 1
            else:
                i += 1
            if r.direction == "up":
                u += 1
            else:
                d += 1
        return {"total": len(self._records), "settled": s,
                "invalidated": i, "up": u, "down": d}
