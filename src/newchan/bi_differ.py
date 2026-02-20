"""Stroke 列表差分 — 从快照对比产生域事件

核心算法：
1. 找 prev 和 curr 的公共前缀（confirmed 且字段完全相同的笔）
2. prev 后缀中的笔 → StrokeInvalidated
3. curr 后缀中：
   - confirmed=True → StrokeSettled
   - confirmed=False 且是延伸 → StrokeExtended
   - confirmed=False 且是新笔 → StrokeCandidate
"""

from __future__ import annotations

from newchan.a_stroke import Stroke
from newchan.events import (
    DomainEvent,
    StrokeCandidate,
    StrokeExtended,
    StrokeInvalidated,
    StrokeSettled,
)
from newchan.fingerprint import compute_event_id


def _strokes_equal(a: Stroke, b: Stroke) -> bool:
    """严格比较两笔是否完全相同（包含 confirmed 状态）。"""
    return (
        a.i0 == b.i0
        and a.i1 == b.i1
        and a.direction == b.direction
        and a.confirmed == b.confirmed
        and abs(a.p0 - b.p0) < 1e-9
        and abs(a.p1 - b.p1) < 1e-9
    )


def _same_origin(a: Stroke, b: Stroke) -> bool:
    """两笔是否来自同一起点（i0 和 direction 相同）。"""
    return a.i0 == b.i0 and a.direction == b.direction


def _find_common_prefix_len(prev: list[Stroke], curr: list[Stroke]) -> int:
    """找 prev 和 curr 的公共前缀长度（confirmed 且字段完全相同）。"""
    common_len = 0
    for i in range(min(len(prev), len(curr))):
        if _strokes_equal(prev[i], curr[i]):
            common_len = i + 1
        else:
            break
    return common_len


class _EventEmitter:
    """封装事件创建的序号管理和 event_id 计算。"""

    def __init__(self, bar_idx: int, bar_ts: float, seq_start: int) -> None:
        self.bar_idx = bar_idx
        self.bar_ts = bar_ts
        self.seq = seq_start
        self.events: list[DomainEvent] = []

    def emit(self, cls: type, **kwargs: object) -> None:
        eid = compute_event_id(
            bar_idx=self.bar_idx, bar_ts=self.bar_ts,
            event_type=cls.__dataclass_fields__["event_type"].default,
            seq=self.seq, payload=dict(kwargs),
        )
        self.events.append(cls(
            bar_idx=self.bar_idx, bar_ts=self.bar_ts,
            seq=self.seq, event_id=eid, **kwargs,
        ))
        self.seq += 1


def _classify_curr_stroke(
    emitter: _EventEmitter, s: Stroke, i: int, prev: list[Stroke],
) -> None:
    """将 curr 后缀中的一笔分类为 settled / extended / candidate 并发射事件。"""
    if s.confirmed:
        emitter.emit(StrokeSettled, stroke_id=i, direction=s.direction,
                     i0=s.i0, i1=s.i1, p0=s.p0, p1=s.p1)
        return

    if i < len(prev) and _same_origin(prev[i], s) and not prev[i].confirmed:
        if prev[i].i1 != s.i1 or abs(prev[i].p1 - s.p1) > 1e-9:
            emitter.emit(StrokeExtended, stroke_id=i, direction=s.direction,
                         old_i1=prev[i].i1, new_i1=s.i1,
                         old_p1=prev[i].p1, new_p1=s.p1)
        return

    emitter.emit(StrokeCandidate, stroke_id=i, direction=s.direction,
                 i0=s.i0, i1=s.i1, p0=s.p0, p1=s.p1)


def diff_strokes(
    prev: list[Stroke],
    curr: list[Stroke],
    *,
    bar_idx: int,
    bar_ts: float,
    seq_start: int = 0,
) -> list[DomainEvent]:
    """比较前后两次 Stroke 快照，产生域事件列表。

    按因果顺序：先 invalidate 旧笔，再 settle/candidate/extend 新笔。
    """
    common_len = _find_common_prefix_len(prev, curr)
    emitter = _EventEmitter(bar_idx, bar_ts, seq_start)

    for i in range(common_len, len(prev)):
        s = prev[i]
        emitter.emit(StrokeInvalidated, stroke_id=i, direction=s.direction,
                     i0=s.i0, i1=s.i1, p0=s.p0, p1=s.p1)

    for i in range(common_len, len(curr)):
        _classify_curr_stroke(emitter, curr[i], i, prev)

    return emitter.events
