"""买卖点状态 + 差分逻辑

BuySellPointSnapshot: 一次 BSP 计算后的完整快照。
diff_buysellpoints: 比较前后两次 BSP 列表，产生域事件。

Diff 规则（与 diff_moves 同构，新增 confirm 状态转换）：
1. 找公共前缀（_bsp_equal 严格比较身份键 + 状态字段）
2. prev 后缀：同身份 → 跳过 invalidate；否则 → BuySellPointInvalidateV1
3. curr 后缀：
   - 全新 BSP + confirmed=True → Candidate + Confirm（保证 I24）
   - 全新 BSP + confirmed=False → Candidate
   - 同身份 + confirmed 升级(F→T) → Confirm
   - 同身份 + settled 升级(F→T) → Settle
   - 同身份 + price/overlaps_with 变化 → Candidate（更新）
"""

from __future__ import annotations

from dataclasses import dataclass

from newchan.a_buysellpoint_v1 import BuySellPoint
from newchan.core.diff.identity import same_bsp_identity
from newchan.events import (
    BuySellPointCandidateV1,
    BuySellPointConfirmV1,
    BuySellPointInvalidateV1,
    BuySellPointSettleV1,
    DomainEvent,
)
from newchan.fingerprint import compute_event_id


@dataclass
class BuySellPointSnapshot:
    """一次 BSP 计算后的完整快照。"""

    bar_idx: int
    bar_ts: float
    buysellpoints: list[BuySellPoint]
    events: list[DomainEvent]


def _bsp_equal(a: BuySellPoint, b: BuySellPoint) -> bool:
    """严格比较两个 BSP（身份键 + 所有状态字段）。"""
    return (
        a.seg_idx == b.seg_idx
        and a.kind == b.kind
        and a.side == b.side
        and a.level_id == b.level_id
        and a.confirmed == b.confirmed
        and a.settled == b.settled
        and a.price == b.price
        and a.overlaps_with == b.overlaps_with
    )


def diff_buysellpoints(
    prev: list[BuySellPoint],
    curr: list[BuySellPoint],
    *,
    bar_idx: int,
    bar_ts: float,
    seq_start: int = 0,
) -> list[DomainEvent]:
    """比较前后两次 BSP 列表，产生域事件。

    Parameters
    ----------
    prev, curr : list[BuySellPoint]
        前后两次计算的买卖点列表。
    bar_idx, bar_ts : int, float
        当前 bar 的索引和时间戳。
    seq_start : int
        本批事件的起始序号。

    Returns
    -------
    list[DomainEvent]
        按因果顺序：先 invalidate，再 candidate/confirm/settle。
    """
    events: list[DomainEvent] = []
    seq = seq_start

    # ── 找公共前缀 ──
    common_len = 0
    for i in range(min(len(prev), len(curr))):
        if _bsp_equal(prev[i], curr[i]):
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

    # ── prev 后缀 → invalidated（跳过同身份升级项）──
    for i in range(common_len, len(prev)):
        bp = prev[i]
        curr_bp = curr[i] if i < len(curr) else None
        if curr_bp is not None and same_bsp_identity(bp, curr_bp):
            continue  # 同身份 → 状态变化，不 invalidate
        _append(
            BuySellPointInvalidateV1,
            bsp_id=i,
            kind=bp.kind,
            side=bp.side,
            level_id=bp.level_id,
            seg_idx=bp.seg_idx,
        )

    # ── curr 后缀 ──
    for i in range(common_len, len(curr)):
        bp = curr[i]
        prev_bp = prev[i] if i < len(prev) else None

        if prev_bp is not None and same_bsp_identity(prev_bp, bp):
            # 同身份但状态变了
            if not prev_bp.confirmed and bp.confirmed:
                _append(
                    BuySellPointConfirmV1,
                    bsp_id=i,
                    kind=bp.kind,
                    side=bp.side,
                    level_id=bp.level_id,
                    seg_idx=bp.seg_idx,
                    price=bp.price,
                )
            elif not prev_bp.settled and bp.settled:
                _append(
                    BuySellPointSettleV1,
                    bsp_id=i,
                    kind=bp.kind,
                    side=bp.side,
                    level_id=bp.level_id,
                    seg_idx=bp.seg_idx,
                    price=bp.price,
                )
            elif prev_bp.price != bp.price or prev_bp.overlaps_with != bp.overlaps_with:
                _append(
                    BuySellPointCandidateV1,
                    bsp_id=i,
                    kind=bp.kind,
                    side=bp.side,
                    level_id=bp.level_id,
                    seg_idx=bp.seg_idx,
                    price=bp.price,
                    move_seg_start=bp.move_seg_start,
                    center_seg_start=bp.center_seg_start,
                    overlaps_with=bp.overlaps_with or "",
                )
        else:
            # 全新 BSP
            _append(
                BuySellPointCandidateV1,
                bsp_id=i,
                kind=bp.kind,
                side=bp.side,
                level_id=bp.level_id,
                seg_idx=bp.seg_idx,
                price=bp.price,
                move_seg_start=bp.move_seg_start,
                center_seg_start=bp.center_seg_start,
                overlaps_with=bp.overlaps_with or "",
            )
            if bp.confirmed:
                # 首次出现即已 confirmed → Candidate + Confirm（保证 I24）
                _append(
                    BuySellPointConfirmV1,
                    bsp_id=i,
                    kind=bp.kind,
                    side=bp.side,
                    level_id=bp.level_id,
                    seg_idx=bp.seg_idx,
                    price=bp.price,
                )

    return events
