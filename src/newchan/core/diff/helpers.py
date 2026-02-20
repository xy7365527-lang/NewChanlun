"""Diff 共享工具 — 提取五层同构 diff 的公共骨架。

提供:
- make_appender: 构建 _append 闭包（事件工厂）
- find_common_prefix: 泛型公共前缀查找
- diff_by_prefix: 前缀式 diff 骨架（segment/zhongshu/move/level_* 共用）
"""

from __future__ import annotations

from typing import Any, Callable, Sequence, TypeVar

from newchan.events import DomainEvent
from newchan.fingerprint import compute_event_id

T = TypeVar("T")


def make_appender(
    target: list[DomainEvent],
    bar_idx: int,
    bar_ts: float,
    seq_box: list[int],
    extra_kwargs: dict[str, object] | None = None,
) -> Callable[..., None]:
    """构建 _append 闭包。

    Parameters
    ----------
    target : list
        事件追加目标列表。
    seq_box : list[int]
        单元素列表，用于跨闭包共享可变 seq 计数器。
    extra_kwargs : dict | None
        每个事件额外注入的字段（如 level_id）。
    """
    _extra = extra_kwargs or {}

    def _append(cls: type, **kwargs: object) -> None:
        kwargs.update(_extra)
        eid = compute_event_id(
            bar_idx=bar_idx,
            bar_ts=bar_ts,
            event_type=cls.__dataclass_fields__["event_type"].default,
            seq=seq_box[0],
            payload=dict(kwargs),
        )
        target.append(
            cls(bar_idx=bar_idx, bar_ts=bar_ts, seq=seq_box[0], event_id=eid, **kwargs)
        )
        seq_box[0] += 1

    return _append


def find_common_prefix(
    prev: Sequence[T],
    curr: Sequence[T],
    equal_fn: Callable[[T, T], bool],
) -> int:
    """返回 prev/curr 的公共前缀长度。"""
    common = 0
    for i in range(min(len(prev), len(curr))):
        if equal_fn(prev[i], curr[i]):
            common = i + 1
        else:
            break
    return common


def diff_by_prefix(
    prev: Sequence[T],
    curr: Sequence[T],
    *,
    bar_idx: int,
    bar_ts: float,
    seq_start: int,
    equal_fn: Callable[[T, T], bool],
    same_identity_fn: Callable[[T, T], bool],
    emit_invalidate: Callable[[Callable[..., None], int, T], None],
    handle_same_identity: Callable[[Callable[..., None], int, T, T], None],
    handle_new: Callable[[Callable[..., None], int, T], None],
    extra_kwargs: dict[str, object] | None = None,
) -> list[DomainEvent]:
    """前缀式 diff 骨架 — segment/zhongshu/move 共用。

    Returns
    -------
    list[DomainEvent]
        按因果顺序：先 invalidate，再 candidate/settle。
    """
    events: list[DomainEvent] = []
    seq_box = [seq_start]
    _append = make_appender(events, bar_idx, bar_ts, seq_box, extra_kwargs)

    common_len = find_common_prefix(prev, curr, equal_fn)

    # prev 后缀 → invalidated（跳过同身份升级项）
    for i in range(common_len, len(prev)):
        item = prev[i]
        curr_item = curr[i] if i < len(curr) else None
        if curr_item is not None and same_identity_fn(item, curr_item):
            continue
        emit_invalidate(_append, i, item)

    # curr 后缀
    for i in range(common_len, len(curr)):
        item = curr[i]
        prev_item = prev[i] if i < len(prev) else None
        if prev_item is not None and same_identity_fn(prev_item, item):
            handle_same_identity(_append, i, prev_item, item)
        else:
            handle_new(_append, i, item)

    return events
