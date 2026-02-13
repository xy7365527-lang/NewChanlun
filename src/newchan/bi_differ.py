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


def diff_strokes(
    prev: list[Stroke],
    curr: list[Stroke],
    *,
    bar_idx: int,
    bar_ts: float,
    seq_start: int = 0,
) -> list[DomainEvent]:
    """比较前后两次 Stroke 快照，产生域事件列表。

    Parameters
    ----------
    prev : list[Stroke]
        上一次 process_bar 后的笔列表。
    curr : list[Stroke]
        本次 process_bar 后的笔列表。
    bar_idx : int
        当前 bar 的序列索引。
    bar_ts : float
        当前 bar 的时间戳（epoch 秒）。
    seq_start : int
        本批事件的起始序号。

    Returns
    -------
    list[DomainEvent]
        按因果顺序排列：先 invalidate 旧笔，再 settle/candidate/extend 新笔。
    """
    events: list[DomainEvent] = []
    seq = seq_start

    # ── 找公共前缀长度 ──
    common_len = 0
    for i in range(min(len(prev), len(curr))):
        if _strokes_equal(prev[i], curr[i]):
            common_len = i + 1
        else:
            break

    def _append(cls: type, **kwargs: object) -> None:
        nonlocal seq
        eid = compute_event_id(
            bar_idx=bar_idx,
            bar_ts=bar_ts,
            event_type=cls.__dataclass_fields__["event_type"].default,
            seq=seq,
            payload=dict(kwargs),
        )
        events.append(cls(bar_idx=bar_idx, bar_ts=bar_ts, seq=seq, event_id=eid, **kwargs))
        seq += 1

    # ── prev 后缀 → invalidated ──
    for i in range(common_len, len(prev)):
        s = prev[i]
        _append(
            StrokeInvalidated,
            stroke_id=i,
            direction=s.direction,
            i0=s.i0,
            i1=s.i1,
            p0=s.p0,
            p1=s.p1,
        )

    # ── curr 后缀 ──
    for i in range(common_len, len(curr)):
        s = curr[i]

        if s.confirmed:
            # 已确认 → settled
            _append(
                StrokeSettled,
                stroke_id=i,
                direction=s.direction,
                i0=s.i0,
                i1=s.i1,
                p0=s.p0,
                p1=s.p1,
            )
        else:
            # 未确认 → 检查是否是延伸
            if i < len(prev) and _same_origin(prev[i], s) and not prev[i].confirmed:
                # 同起点、同方向、都是未确认 → 检查终点是否变化
                if prev[i].i1 != s.i1 or abs(prev[i].p1 - s.p1) > 1e-9:
                    _append(
                        StrokeExtended,
                        stroke_id=i,
                        direction=s.direction,
                        old_i1=prev[i].i1,
                        new_i1=s.i1,
                        old_p1=prev[i].p1,
                        new_p1=s.p1,
                    )
                # 如果终点没变 → 无事件（公共前缀应该已经覆盖这种情况，
                # 但这里作为防御处理）
            else:
                # 新笔候选
                _append(
                    StrokeCandidate,
                    stroke_id=i,
                    direction=s.direction,
                    i0=s.i0,
                    i1=s.i1,
                    p0=s.p0,
                    p1=s.p1,
                )

    return events
