"""级别递归层 — 快照与差分。

RecursiveLevelSnapshot 包含单个递归级别的中枢和走势计算结果。
diff 函数复用 LevelZhongshu 的身份规则产生域事件。

概念溯源: [旧缠论:选择] — settled 作为递归输入过滤条件
"""

from __future__ import annotations

from dataclasses import dataclass

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


# ── diff_level_zhongshu ──


def diff_level_zhongshu(
    prev: list[LevelZhongshu],
    curr: list[LevelZhongshu],
    *,
    bar_idx: int,
    bar_ts: float,
    seq_start: int = 0,
) -> list[DomainEvent]:
    """比较前后两次 LevelZhongshu 列表，产生域事件。

    结构与 zhongshu_state.diff_zhongshu 同构，适配 LevelZhongshu 的字段。

    Returns
    -------
    list[DomainEvent]
        按因果顺序：先 invalidate 旧中枢，再 candidate/settle 新中枢。
    """
    events: list[DomainEvent] = []
    seq = seq_start

    # 找公共前缀长度
    common_len = 0
    for i in range(min(len(prev), len(curr))):
        if _level_zhongshu_equal(prev[i], curr[i]):
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
            if not prev_zs.settled and zs.settled:
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
            elif prev_zs.comp_end != zs.comp_end:
                _append(
                    ZhongshuCandidateV1,
                    zhongshu_id=i,
                    zd=zs.zd,
                    zg=zs.zg,
                    seg_start=zs.comp_start,
                    seg_end=zs.comp_end,
                    seg_count=zs.comp_count,
                )
        else:
            if zs.settled:
                _append(
                    ZhongshuCandidateV1,
                    zhongshu_id=i,
                    zd=zs.zd,
                    zg=zs.zg,
                    seg_start=zs.comp_start,
                    seg_end=zs.comp_end,
                    seg_count=zs.comp_count,
                )
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
            else:
                _append(
                    ZhongshuCandidateV1,
                    zhongshu_id=i,
                    zd=zs.zd,
                    zg=zs.zg,
                    seg_start=zs.comp_start,
                    seg_end=zs.comp_end,
                    seg_count=zs.comp_count,
                )

    return events


# ── diff_level_moves ──


def diff_level_moves(
    prev: list[Move],
    curr: list[Move],
    *,
    bar_idx: int,
    bar_ts: float,
    seq_start: int = 0,
) -> list[DomainEvent]:
    """比较前后两次递归级别 Move 列表，产生域事件。

    结构与 move_state.diff_moves 同构。

    Returns
    -------
    list[DomainEvent]
        按因果顺序：先 invalidate，再 candidate/settle。
    """
    events: list[DomainEvent] = []
    seq = seq_start

    def _move_equal(a: Move, b: Move) -> bool:
        return (
            a.kind == b.kind
            and a.direction == b.direction
            and a.seg_start == b.seg_start
            and a.zs_end == b.zs_end
            and a.settled == b.settled
        )

    def _same_identity(a: Move, b: Move) -> bool:
        return a.seg_start == b.seg_start

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

    # prev 后缀 → invalidated
    for i in range(common_len, len(prev)):
        m = prev[i]
        curr_m = curr[i] if i < len(curr) else None
        if curr_m is not None and _same_identity(m, curr_m):
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

        if prev_m is not None and _same_identity(prev_m, m):
            if not prev_m.settled and m.settled:
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
            if m.settled:
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
