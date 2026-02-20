"""线段状态 + 差分逻辑

SegmentSnapshot: 一次 segment 计算后的完整快照。
diff_segments: 比较前后两次 Segment 列表，产生域事件。

Diff 规则（类比 zhongshu_state 的 diff_zhongshu，三层同构）：
1. 找公共前缀：(s0, s1, direction, confirmed, kind) 完全相同
2. prev 后缀中的段：
   - 若 curr 同位段具有相同身份 (s0, direction) → 跳过 invalidate（升级/延伸）
   - 否则 → SegmentInvalidateV1
3. curr 后缀中的段：
   - 同身份升级：unconfirmed→confirmed+settled → BreakPending + Settle
   - 同身份新增 break_evidence → BreakPending
   - 同身份 break_evidence 变化或 s1 变化（有 be）→ BreakPending（更新）
   - 同身份纯延伸（无 break_evidence 变化）→ 无事件
   - 全新段 confirmed=True + settled + break_evidence → BreakPending + Settle
   - 全新段 confirmed=False + break_evidence → BreakPending
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Callable

from newchan.a_segment_v0 import Segment
from newchan.core.diff.identity import same_segment_identity
from newchan.events import (
    DomainEvent,
    SegmentBreakPendingV1,
    SegmentInvalidateV1,
    SegmentSettleV1,
)
from newchan.core.diff.helpers import diff_by_prefix


@dataclass
class SegmentSnapshot:
    """一次 segment 计算后的完整快照。"""

    bar_idx: int
    bar_ts: float
    segments: list[Segment]
    events: list[DomainEvent]


def _segments_equal(a: Segment, b: Segment) -> bool:
    """严格比较两段是否完全相同（用于 diff 公共前缀）。"""
    return (
        a.s0 == b.s0
        and a.s1 == b.s1
        and a.direction == b.direction
        and a.confirmed == b.confirmed
        and a.kind == b.kind
    )


def _emit_break_pending(
    _append: Callable[..., None], i: int, seg: Segment,
) -> None:
    """发射 SegmentBreakPendingV1 事件。"""
    be = seg.break_evidence
    gap_cls = "gap" if be.gap_type == "second" else "none"
    _append(
        SegmentBreakPendingV1,
        segment_id=i,
        direction=seg.direction,
        break_at_stroke=be.trigger_stroke_k,
        gap_class=gap_cls,
        fractal_type="top" if seg.direction == "up" else "bottom",
        s0=seg.s0,
        s1=seg.s1,
    )


def _emit_settle(
    _append: Callable[..., None], i: int, seg: Segment,
) -> None:
    """发射 SegmentSettleV1 事件。"""
    be = seg.break_evidence
    gap_cls = "gap" if be.gap_type == "second" else "none"
    opposite = "down" if seg.direction == "up" else "up"
    _append(
        SegmentSettleV1,
        segment_id=i,
        direction=seg.direction,
        s0=seg.s0,
        s1=seg.s1,
        ep0_price=seg.ep0_price,
        ep1_price=seg.ep1_price,
        gap_class=gap_cls,
        new_segment_s0=be.trigger_stroke_k,
        new_segment_direction=opposite,
    )


def _handle_seg_same_identity(
    _append: Callable[..., None],
    i: int,
    prev_seg: Segment,
    seg: Segment,
) -> None:
    """同身份段：状态变化产生升级事件。"""
    if (
        not prev_seg.confirmed
        and seg.confirmed
        and seg.kind == "settled"
        and seg.break_evidence is not None
    ):
        _emit_break_pending(_append, i, seg)
        _emit_settle(_append, i, seg)
    elif not seg.confirmed and seg.break_evidence is not None:
        prev_be = prev_seg.break_evidence
        if (
            prev_be is None
            or prev_be.trigger_stroke_k != seg.break_evidence.trigger_stroke_k
            or prev_seg.s1 != seg.s1
        ):
            _emit_break_pending(_append, i, seg)


def _handle_seg_new(
    _append: Callable[..., None], i: int, seg: Segment,
) -> None:
    """全新段（或身份不同的替换段）的事件处理。"""
    if seg.confirmed and seg.kind == "settled" and seg.break_evidence is not None:
        _emit_break_pending(_append, i, seg)
        _emit_settle(_append, i, seg)
    elif not seg.confirmed and seg.break_evidence is not None:
        _emit_break_pending(_append, i, seg)


def _emit_seg_invalidate(
    _append: Callable[..., None], i: int, seg: Segment,
) -> None:
    """发射 SegmentInvalidateV1 事件。"""
    _append(
        SegmentInvalidateV1,
        segment_id=i,
        direction=seg.direction,
        s0=seg.s0,
        s1=seg.s1,
    )


def diff_segments(
    prev: list[Segment],
    curr: list[Segment],
    *,
    bar_idx: int,
    bar_ts: float,
    seq_start: int = 0,
) -> list[DomainEvent]:
    """比较前后两次 Segment 列表，产生域事件。

    Returns
    -------
    list[DomainEvent]
        按因果顺序排列：先 invalidate 旧段，再 settle/pending 新段。
    """
    return diff_by_prefix(
        prev,
        curr,
        bar_idx=bar_idx,
        bar_ts=bar_ts,
        seq_start=seq_start,
        equal_fn=_segments_equal,
        same_identity_fn=same_segment_identity,
        emit_invalidate=_emit_seg_invalidate,
        handle_same_identity=_handle_seg_same_identity,
        handle_new=_handle_seg_new,
    )
