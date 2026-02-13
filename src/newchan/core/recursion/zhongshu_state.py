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
        # 检查 curr 中同位是否有"同身份"中枢（candidate→settle 升级或延伸）
        curr_zs = curr[i] if i < len(curr) else None
        if curr_zs is not None and same_zhongshu_identity(zs, curr_zs):
            # 同一个中枢的状态更新（闭合升级或延伸），不发 invalidate
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

        # 检查 prev 中是否有"同位"但不完全相同的中枢（可能是延伸或闭合升级）
        prev_zs = prev[i] if i < len(prev) else None

        if prev_zs is not None and same_zhongshu_identity(prev_zs, zs):
            # 同一个中枢，但 seg_end 或 settled 状态变了
            if not prev_zs.settled and zs.settled:
                # candidate → settle 升级
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
            elif prev_zs.seg_end != zs.seg_end:
                # 延伸：seg_end 变化 → 新 candidate（旧的已在 prev 后缀 invalidated）
                _append(
                    ZhongshuCandidateV1,
                    zhongshu_id=i,
                    zd=zs.zd,
                    zg=zs.zg,
                    seg_start=zs.seg_start,
                    seg_end=zs.seg_end,
                    seg_count=zs.seg_count,
                )
        else:
            # 全新中枢
            if zs.settled:
                # 首次出现即已闭合：先 candidate 再 settle（保证 I12）
                _append(
                    ZhongshuCandidateV1,
                    zhongshu_id=i,
                    zd=zs.zd,
                    zg=zs.zg,
                    seg_start=zs.seg_start,
                    seg_end=zs.seg_end,
                    seg_count=zs.seg_count,
                )
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
            else:
                # 新 candidate（未闭合）
                _append(
                    ZhongshuCandidateV1,
                    zhongshu_id=i,
                    zd=zs.zd,
                    zg=zs.zg,
                    seg_start=zs.seg_start,
                    seg_end=zs.seg_end,
                    seg_count=zs.seg_count,
                )

    return events
