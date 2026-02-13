"""走势类型状态 + 差分逻辑

MoveSnapshot: 一次 move 计算后的完整快照。
diff_moves: 比较前后两次 Move 列表，产生域事件。

Diff 规则（与 diff_zhongshu 同构）：
1. 找公共前缀（_move_equal 严格比较）
2. prev 后缀：同身份 → 跳过 invalidate；否则 → MoveInvalidateV1
3. curr 后缀：
   - 全新 move + settled=True → Candidate + Settle（保证 I19）
   - 全新 move + settled=False → Candidate
   - 同身份 + settled 升级(F→T) → Settle
   - 同身份 + zs_end 变化 → Candidate（延伸，含 consolidation→trend）
"""

from __future__ import annotations

from dataclasses import dataclass

from newchan.a_move_v1 import Move
from newchan.core.diff.identity import same_move_identity
from newchan.events import (
    DomainEvent,
    MoveCandidateV1,
    MoveInvalidateV1,
    MoveSettleV1,
)
from newchan.fingerprint import compute_event_id


@dataclass
class MoveSnapshot:
    """一次 move 计算后的完整快照。"""

    bar_idx: int
    bar_ts: float
    moves: list[Move]
    events: list[DomainEvent]


def _move_equal(a: Move, b: Move) -> bool:
    """严格比较两个 Move 是否完全相同（用于 diff 公共前缀）。"""
    return (
        a.kind == b.kind
        and a.direction == b.direction
        and a.seg_start == b.seg_start
        and a.zs_end == b.zs_end
        and a.settled == b.settled
    )


def diff_moves(
    prev: list[Move],
    curr: list[Move],
    *,
    bar_idx: int,
    bar_ts: float,
    seq_start: int = 0,
) -> list[DomainEvent]:
    """比较前后两次 Move 列表，产生域事件。

    Parameters
    ----------
    prev : list[Move]
        上一次计算的 Move 列表。
    curr : list[Move]
        本次计算的 Move 列表。
    bar_idx : int
        当前 bar 索引。
    bar_ts : float
        当前 bar 时间戳（epoch 秒）。
    seq_start : int
        本批事件的起始序号。

    Returns
    -------
    list[DomainEvent]
        按因果顺序：先 invalidate 旧 move，再 candidate/settle 新 move。
    """
    events: list[DomainEvent] = []
    seq = seq_start

    # ── 找公共前缀长度 ──
    common_len = 0
    for i in range(min(len(prev), len(curr))):
        if _move_equal(prev[i], curr[i]):
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
        m = prev[i]
        curr_m = curr[i] if i < len(curr) else None
        if curr_m is not None and same_move_identity(m, curr_m):
            continue
        _append(
            MoveInvalidateV1,
            move_id=i,
            kind=m.kind,
            direction=m.direction,
            seg_start=m.seg_start,
            seg_end=m.seg_end,
        )

    # ── curr 后缀 ──
    for i in range(common_len, len(curr)):
        m = curr[i]
        prev_m = prev[i] if i < len(prev) else None

        if prev_m is not None and same_move_identity(prev_m, m):
            # 同一个 move，但状态变了
            if not prev_m.settled and m.settled:
                # candidate → settle 升级
                _append(
                    MoveSettleV1,
                    move_id=i,
                    kind=m.kind,
                    direction=m.direction,
                    seg_start=m.seg_start,
                    seg_end=m.seg_end,
                    zs_start=m.zs_start,
                    zs_end=m.zs_end,
                    zs_count=m.zs_count,
                )
            elif prev_m.zs_end != m.zs_end or prev_m.kind != m.kind:
                # 延伸或 consolidation→trend 升级 → 新 candidate
                _append(
                    MoveCandidateV1,
                    move_id=i,
                    kind=m.kind,
                    direction=m.direction,
                    seg_start=m.seg_start,
                    seg_end=m.seg_end,
                    zs_start=m.zs_start,
                    zs_end=m.zs_end,
                    zs_count=m.zs_count,
                )
        else:
            # 全新 move
            if m.settled:
                # 首次出现即已 settled：先 candidate 再 settle（保证 I19）
                _append(
                    MoveCandidateV1,
                    move_id=i,
                    kind=m.kind,
                    direction=m.direction,
                    seg_start=m.seg_start,
                    seg_end=m.seg_end,
                    zs_start=m.zs_start,
                    zs_end=m.zs_end,
                    zs_count=m.zs_count,
                )
                _append(
                    MoveSettleV1,
                    move_id=i,
                    kind=m.kind,
                    direction=m.direction,
                    seg_start=m.seg_start,
                    seg_end=m.seg_end,
                    zs_start=m.zs_start,
                    zs_end=m.zs_end,
                    zs_count=m.zs_count,
                )
            else:
                # 新 candidate（未 settled）
                _append(
                    MoveCandidateV1,
                    move_id=i,
                    kind=m.kind,
                    direction=m.direction,
                    seg_start=m.seg_start,
                    seg_end=m.seg_end,
                    zs_start=m.zs_start,
                    zs_end=m.zs_end,
                    zs_count=m.zs_count,
                )

    return events
