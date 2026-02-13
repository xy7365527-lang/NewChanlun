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

from newchan.a_segment_v0 import Segment
from newchan.core.diff.identity import same_segment_identity
from newchan.events import (
    DomainEvent,
    SegmentBreakPendingV1,
    SegmentInvalidateV1,
    SegmentSettleV1,
)
from newchan.fingerprint import compute_event_id


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


def diff_segments(
    prev: list[Segment],
    curr: list[Segment],
    *,
    bar_idx: int,
    bar_ts: float,
    seq_start: int = 0,
) -> list[DomainEvent]:
    """比较前后两次 Segment 列表，产生域事件。

    Parameters
    ----------
    prev : list[Segment]
        上一次计算的线段列表。
    curr : list[Segment]
        本次计算的线段列表。
    bar_idx : int
        当前 bar 索引。
    bar_ts : float
        当前 bar 时间戳（epoch 秒）。
    seq_start : int
        本批事件的起始序号。

    Returns
    -------
    list[DomainEvent]
        按因果顺序排列：先 invalidate 旧段，再 settle/pending 新段。
    """
    events: list[DomainEvent] = []
    seq = seq_start

    # ── 找公共前缀长度 ──
    common_len = 0
    for i in range(min(len(prev), len(curr))):
        if _segments_equal(prev[i], curr[i]):
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

    # ── prev 后缀 → invalidated（跳过同身份升级项） ──
    for i in range(common_len, len(prev)):
        seg = prev[i]
        # 检查 curr 中同位是否有"同身份"段（延伸或升级）
        curr_seg = curr[i] if i < len(curr) else None
        if curr_seg is not None and same_segment_identity(seg, curr_seg):
            # 同一段的状态更新（延伸、break_evidence 变化、结算升级），不发 invalidate
            continue
        _append(
            SegmentInvalidateV1,
            segment_id=i,
            direction=seg.direction,
            s0=seg.s0,
            s1=seg.s1,
        )

    # ── curr 后缀 ──
    for i in range(common_len, len(curr)):
        seg = curr[i]

        # 检查 prev 中是否有"同位同身份"段（可能是延伸或升级）
        prev_seg = prev[i] if i < len(prev) else None

        if prev_seg is not None and same_segment_identity(prev_seg, seg):
            # ── 同身份段：状态变化产生升级事件 ──

            if (
                not prev_seg.confirmed
                and seg.confirmed
                and seg.kind == "settled"
                and seg.break_evidence is not None
            ):
                # 升级路径：unconfirmed → confirmed+settled（结算升级）
                be = seg.break_evidence
                gap_cls = "gap" if be.gap_type == "second" else "none"
                opposite = "down" if seg.direction == "up" else "up"

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

            elif not seg.confirmed and seg.break_evidence is not None:
                # break_evidence 变化/新增 + s1 可能延伸
                prev_be = prev_seg.break_evidence
                if (
                    prev_be is None
                    or prev_be.trigger_stroke_k != seg.break_evidence.trigger_stroke_k
                    or prev_seg.s1 != seg.s1
                ):
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

            # else: 纯延伸（s1 变但无 break_evidence）或状态未变 → 不产生事件

        else:
            # ── 全新段（或身份不同的替换段） ──

            if seg.confirmed and seg.kind == "settled" and seg.break_evidence is not None:
                # 旧段被结算：先发 pending，再发 settle
                be = seg.break_evidence
                gap_cls = "gap" if be.gap_type == "second" else "none"
                opposite = "down" if seg.direction == "up" else "up"

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

            elif not seg.confirmed:
                # 未确认的最后一段 → SegmentBreakPendingV1
                if seg.break_evidence is not None:
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
                # 无 break_evidence 的新段（纯延伸）→ 不产生事件

    return events
