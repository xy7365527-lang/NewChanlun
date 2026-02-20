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
from typing import Callable

from newchan.a_move_v1 import Move
from newchan.core.diff.identity import same_move_identity
from newchan.events import (
    DomainEvent,
    MoveCandidateV1,
    MoveInvalidateV1,
    MoveSettleV1,
)
from newchan.core.diff.helpers import diff_by_prefix


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


def _move_kwargs(i: int, m: Move) -> dict[str, object]:
    """构建 Move 事件的公共 kwargs。"""
    return dict(
        move_id=i,
        kind=m.kind,
        direction=m.direction,
        seg_start=m.seg_start,
        seg_end=m.seg_end,
        zs_start=m.zs_start,
        zs_end=m.zs_end,
        zs_count=m.zs_count,
    )


def _handle_move_same_identity(
    _append: Callable[..., None],
    i: int,
    prev_m: Move,
    m: Move,
) -> None:
    """同身份 move 的状态变化处理。"""
    if not prev_m.settled and m.settled:
        _append(MoveSettleV1, **_move_kwargs(i, m))
    elif prev_m.zs_end != m.zs_end or prev_m.kind != m.kind:
        _append(MoveCandidateV1, **_move_kwargs(i, m))


def _handle_move_new(
    _append: Callable[..., None], i: int, m: Move,
) -> None:
    """全新 move 的事件处理。"""
    _append(MoveCandidateV1, **_move_kwargs(i, m))
    if m.settled:
        _append(MoveSettleV1, **_move_kwargs(i, m))


def _emit_move_invalidate(
    _append: Callable[..., None], i: int, m: Move,
) -> None:
    """发射 MoveInvalidateV1 事件。"""
    _append(
        MoveInvalidateV1,
        move_id=i,
        kind=m.kind,
        direction=m.direction,
        seg_start=m.seg_start,
        seg_end=m.seg_end,
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

    Returns
    -------
    list[DomainEvent]
        按因果顺序：先 invalidate 旧 move，再 candidate/settle 新 move。
    """
    return diff_by_prefix(
        prev,
        curr,
        bar_idx=bar_idx,
        bar_ts=bar_ts,
        seq_start=seq_start,
        equal_fn=_move_equal,
        same_identity_fn=same_move_identity,
        emit_invalidate=_emit_move_invalidate,
        handle_same_identity=_handle_move_same_identity,
        handle_new=_handle_move_new,
    )
