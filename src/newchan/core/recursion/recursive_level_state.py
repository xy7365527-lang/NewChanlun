"""级别递归层 — 快照与差分。

RecursiveLevelSnapshot 包含单个递归级别的中枢和走势计算结果。
diff 函数复用 LevelZhongshu 的身份规则产生域事件。

概念溯源: [旧缠论:选择] — settled 作为递归输入过滤条件
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Callable

from newchan.a_move_v1 import Move
from newchan.a_zhongshu_level import LevelZhongshu
from newchan.events import (
    DomainEvent,
    MoveCandidateV1,
    MoveInvalidateV1,
    MoveSettleV1,
    ZhongshuCandidateV1,
    ZhongshuInvalidateV1,
    ZhongshuSettleV1,
)
from newchan.fingerprint import compute_event_id


@dataclass
class RecursiveLevelSnapshot:
    """一次递归级别计算后的完整快照。"""

    bar_idx: int
    bar_ts: float
    level_id: int
    zhongshus: list[LevelZhongshu]
    moves: list[Move]
    zhongshu_events: list[DomainEvent]
    move_events: list[DomainEvent]


# ── 身份和比较 ──


def _level_zhongshu_equal(a: LevelZhongshu, b: LevelZhongshu) -> bool:
    """严格比较两个 LevelZhongshu 是否完全相同（用于 diff 公共前缀）。"""
    return (
        a.zd == b.zd
        and a.zg == b.zg
        and a.comp_start == b.comp_start
        and a.comp_end == b.comp_end
        and a.settled == b.settled
        and a.level_id == b.level_id
    )


def _same_level_zhongshu_identity(a: LevelZhongshu, b: LevelZhongshu) -> bool:
    """两个 LevelZhongshu 是否具有同一身份（zd + zg + comp_start + level_id）。"""
    return (
        a.zd == b.zd
        and a.zg == b.zg
        and a.comp_start == b.comp_start
        and a.level_id == b.level_id
    )


# ── LevelZhongshu 辅助 ──


def _emit_lzs_candidate(
    _append: Callable[..., None], i: int, zs: LevelZhongshu,
) -> None:
    _append(
        ZhongshuCandidateV1,
        zhongshu_id=i,
        zd=zs.zd,
        zg=zs.zg,
        seg_start=zs.comp_start,
        seg_end=zs.comp_end,
        seg_count=zs.comp_count,
    )


def _emit_lzs_settle(
    _append: Callable[..., None], i: int, zs: LevelZhongshu,
) -> None:
    _append(
        ZhongshuSettleV1,
        zhongshu_id=i,
        zd=zs.zd,
        zg=zs.zg,
        seg_start=zs.comp_start,
        seg_end=zs.comp_end,
        seg_count=zs.comp_count,
        break_seg_id=zs.break_comp,
        break_direction=zs.break_direction,
    )


def _handle_lzs_same_identity(
    _append: Callable[..., None],
    i: int,
    prev_zs: LevelZhongshu,
    zs: LevelZhongshu,
) -> None:
    """同身份 LevelZhongshu 的状态变化处理。"""
    if not prev_zs.settled and zs.settled:
        _emit_lzs_settle(_append, i, zs)
    elif prev_zs.comp_end != zs.comp_end:
        _emit_lzs_candidate(_append, i, zs)


def _handle_lzs_new(
    _append: Callable[..., None], i: int, zs: LevelZhongshu,
) -> None:
    """全新 LevelZhongshu 的事件处理。"""
    _emit_lzs_candidate(_append, i, zs)
    if zs.settled:
        _emit_lzs_settle(_append, i, zs)


# ── diff_level_zhongshu ──


def diff_level_zhongshu(
    prev: list[LevelZhongshu],
    curr: list[LevelZhongshu],
    *,
    bar_idx: int,
    bar_ts: float,
    seq_start: int = 0,
    level_id: int = 1,
) -> list[DomainEvent]:
    """比较前后两次 LevelZhongshu 列表，产生域事件。

    结构与 zhongshu_state.diff_zhongshu 同构，适配 LevelZhongshu 的字段。

    Parameters
    ----------
    level_id : int
        递归级别 ID，注入到所有产出事件的 level_id 字段（P6 扩展）。

    Returns
    -------
    list[DomainEvent]
        按因果顺序：先 invalidate 旧中枢，再 candidate/settle 新中枢。
    """
    events: list[DomainEvent] = []
    seq = seq_start

    common_len = 0
    for i in range(min(len(prev), len(curr))):
        if _level_zhongshu_equal(prev[i], curr[i]):
            common_len = i + 1
        else:
            break

    def _append(cls: type, **kwargs: object) -> None:
        nonlocal seq
        kwargs["level_id"] = level_id
        eid = compute_event_id(
            bar_idx=bar_idx,
            bar_ts=bar_ts,
            event_type=cls.__dataclass_fields__["event_type"].default,
            seq=seq,
            payload=dict(kwargs),
        )
        events.append(cls(bar_idx=bar_idx, bar_ts=bar_ts, seq=seq, event_id=eid, **kwargs))
        seq += 1

    # prev 后缀 → invalidated（跳过同身份升级项）
    for i in range(common_len, len(prev)):
        zs = prev[i]
        curr_zs = curr[i] if i < len(curr) else None
        if curr_zs is not None and _same_level_zhongshu_identity(zs, curr_zs):
            continue
        _append(
            ZhongshuInvalidateV1,
            zhongshu_id=i,
            zd=zs.zd,
            zg=zs.zg,
            seg_start=zs.comp_start,
            seg_end=zs.comp_end,
        )

    # curr 后缀
    for i in range(common_len, len(curr)):
        zs = curr[i]
        prev_zs = prev[i] if i < len(prev) else None

        if prev_zs is not None and _same_level_zhongshu_identity(prev_zs, zs):
            _handle_lzs_same_identity(_append, i, prev_zs, zs)
        else:
            _handle_lzs_new(_append, i, zs)

    return events


# ── Level Move 辅助 ──


def _level_move_equal(a: Move, b: Move) -> bool:
    return (
        a.kind == b.kind
        and a.direction == b.direction
        and a.seg_start == b.seg_start
        and a.zs_end == b.zs_end
        and a.settled == b.settled
    )


def _same_level_move_identity(a: Move, b: Move) -> bool:
    return a.seg_start == b.seg_start


def _level_move_kwargs(i: int, m: Move) -> dict[str, object]:
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


def _handle_level_move_same_identity(
    _append: Callable[..., None],
    i: int,
    prev_m: Move,
    m: Move,
) -> None:
    """同身份 level move 的状态变化处理。"""
    if not prev_m.settled and m.settled:
        _append(MoveSettleV1, **_level_move_kwargs(i, m))
    elif prev_m.zs_end != m.zs_end or prev_m.kind != m.kind:
        _append(MoveCandidateV1, **_level_move_kwargs(i, m))


def _handle_level_move_new(
    _append: Callable[..., None], i: int, m: Move,
) -> None:
    """全新 level move 的事件处理。"""
    _append(MoveCandidateV1, **_level_move_kwargs(i, m))
    if m.settled:
        _append(MoveSettleV1, **_level_move_kwargs(i, m))


# ── diff_level_moves ──


def diff_level_moves(
    prev: list[Move],
    curr: list[Move],
    *,
    bar_idx: int,
    bar_ts: float,
    seq_start: int = 0,
    level_id: int = 1,
) -> list[DomainEvent]:
    """比较前后两次递归级别 Move 列表，产生域事件。

    结构与 move_state.diff_moves 同构。

    Parameters
    ----------
    level_id : int
        递归级别 ID，注入到所有产出事件的 level_id 字段（P6 扩展）。

    Returns
    -------
    list[DomainEvent]
        按因果顺序：先 invalidate，再 candidate/settle。
    """
    events: list[DomainEvent] = []
    seq = seq_start

    common_len = 0
    for i in range(min(len(prev), len(curr))):
        if _level_move_equal(prev[i], curr[i]):
            common_len = i + 1
        else:
            break

    def _append(cls: type, **kwargs: object) -> None:
        nonlocal seq
        kwargs["level_id"] = level_id
        eid = compute_event_id(
            bar_idx=bar_idx,
            bar_ts=bar_ts,
            event_type=cls.__dataclass_fields__["event_type"].default,
            seq=seq,
            payload=dict(kwargs),
        )
        events.append(cls(bar_idx=bar_idx, bar_ts=bar_ts, seq=seq, event_id=eid, **kwargs))
        seq += 1

    # prev 后缀 → invalidated
    for i in range(common_len, len(prev)):
        m = prev[i]
        curr_m = curr[i] if i < len(curr) else None
        if curr_m is not None and _same_level_move_identity(m, curr_m):
            continue
        _append(
            MoveInvalidateV1,
            move_id=i,
            kind=m.kind,
            direction=m.direction,
            seg_start=m.seg_start,
            seg_end=m.seg_end,
        )

    # curr 后缀
    for i in range(common_len, len(curr)):
        m = curr[i]
        prev_m = prev[i] if i < len(prev) else None

        if prev_m is not None and _same_level_move_identity(prev_m, m):
            _handle_level_move_same_identity(_append, i, prev_m, m)
        else:
            _handle_level_move_new(_append, i, m)

    return events
