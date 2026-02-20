"""中枢状态 + 差分逻辑

ZhongshuSnapshot: 一次 zhongshu 计算后的完整快照。
diff_zhongshu: 比较前后两次 Zhongshu 列表，产生域事件。

Diff 规则（与 diff_segments 同构）：
1. 找公共前缀：(zd, zg, seg_start, seg_end, settled) 完全相同
2. prev 后缀中的中枢 → ZhongshuInvalidateV1
3. curr 后缀中的中枢：
   - 新出现 settled=False → ZhongshuCandidateV1
   - 新出现 settled=True → ZhongshuCandidateV1 + ZhongshuSettleV1
   - prev settled=False → curr settled=True → ZhongshuSettleV1
   - prev settled=False → curr seg_end 变化 → invalidate 旧 + candidate 新
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Callable

from newchan.a_zhongshu_v1 import Zhongshu
from newchan.core.diff.identity import same_zhongshu_identity
from newchan.events import (
    DomainEvent,
    ZhongshuCandidateV1,
    ZhongshuInvalidateV1,
    ZhongshuSettleV1,
)
from newchan.fingerprint import compute_event_id


@dataclass
class ZhongshuSnapshot:
    """一次 zhongshu 计算后的完整快照。"""

    bar_idx: int
    bar_ts: float
    zhongshus: list[Zhongshu]
    events: list[DomainEvent]


def _zhongshu_equal(a: Zhongshu, b: Zhongshu) -> bool:
    """严格比较两个中枢是否完全相同（用于 diff 公共前缀）。"""
    return (
        a.zd == b.zd
        and a.zg == b.zg
        and a.seg_start == b.seg_start
        and a.seg_end == b.seg_end
        and a.settled == b.settled
    )


def _emit_zhongshu_candidate(
    _append: Callable[..., None], i: int, zs: Zhongshu,
) -> None:
    """发射 ZhongshuCandidateV1 事件。"""
    _append(
        ZhongshuCandidateV1,
        zhongshu_id=i,
        zd=zs.zd,
        zg=zs.zg,
        seg_start=zs.seg_start,
        seg_end=zs.seg_end,
        seg_count=zs.seg_count,
    )


def _emit_zhongshu_settle(
    _append: Callable[..., None], i: int, zs: Zhongshu,
) -> None:
    """发射 ZhongshuSettleV1 事件。"""
    _append(
        ZhongshuSettleV1,
        zhongshu_id=i,
        zd=zs.zd,
        zg=zs.zg,
        seg_start=zs.seg_start,
        seg_end=zs.seg_end,
        seg_count=zs.seg_count,
        break_seg_id=zs.break_seg,
        break_direction=zs.break_direction,
    )


def _handle_zhongshu_same_identity(
    _append: Callable[..., None],
    i: int,
    prev_zs: Zhongshu,
    zs: Zhongshu,
) -> None:
    """同身份中枢的状态变化处理。"""
    if not prev_zs.settled and zs.settled:
        _emit_zhongshu_settle(_append, i, zs)
    elif prev_zs.seg_end != zs.seg_end:
        _emit_zhongshu_candidate(_append, i, zs)


def _handle_zhongshu_new(
    _append: Callable[..., None], i: int, zs: Zhongshu,
) -> None:
    """全新中枢的事件处理。"""
    _emit_zhongshu_candidate(_append, i, zs)
    if zs.settled:
        _emit_zhongshu_settle(_append, i, zs)


def diff_zhongshu(
    prev: list[Zhongshu],
    curr: list[Zhongshu],
    *,
    bar_idx: int,
    bar_ts: float,
    seq_start: int = 0,
) -> list[DomainEvent]:
    """比较前后两次 Zhongshu 列表，产生域事件。

    Parameters
    ----------
    prev : list[Zhongshu]
        上一次计算的中枢列表。
    curr : list[Zhongshu]
        本次计算的中枢列表。
    bar_idx : int
        当前 bar 索引。
    bar_ts : float
        当前 bar 时间戳（epoch 秒）。
    seq_start : int
        本批事件的起始序号。

    Returns
    -------
    list[DomainEvent]
        按因果顺序：先 invalidate 旧中枢，再 candidate/settle 新中枢。
    """
    events: list[DomainEvent] = []
    seq = seq_start

    # ── 找公共前缀长度 ──
    common_len = 0
    for i in range(min(len(prev), len(curr))):
        if _zhongshu_equal(prev[i], curr[i]):
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
        zs = prev[i]
        curr_zs = curr[i] if i < len(curr) else None
        if curr_zs is not None and same_zhongshu_identity(zs, curr_zs):
            continue
        _append(
            ZhongshuInvalidateV1,
            zhongshu_id=i,
            zd=zs.zd,
            zg=zs.zg,
            seg_start=zs.seg_start,
            seg_end=zs.seg_end,
        )

    # ── curr 后缀 ──
    for i in range(common_len, len(curr)):
        zs = curr[i]
        prev_zs = prev[i] if i < len(prev) else None

        if prev_zs is not None and same_zhongshu_identity(prev_zs, zs):
            _handle_zhongshu_same_identity(_append, i, prev_zs, zs)
        else:
            _handle_zhongshu_new(_append, i, zs)

    return events
