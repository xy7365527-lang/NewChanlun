"""买卖点状态 + 差分逻辑

BuySellPointSnapshot: 一次 BSP 计算后的完整快照。
diff_buysellpoints: 比较前后两次 BSP 列表，产生域事件。

Diff 规则（身份键映射 diff，非位置对位）：
1. 构建 prev/curr 的身份键映射 {(seg_idx, kind, side, level_id) → BuySellPoint}
2. prev_only → BuySellPointInvalidateV1
3. curr_only → BuySellPointCandidateV1 (+ ConfirmV1 if confirmed，保证 I24)
4. both → 检查状态变化（confirmed/settled/price/overlaps_with）
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass

from newchan.a_buysellpoint_v1 import BuySellPoint
from newchan.core.diff.identity import bsp_identity_key
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


def _stable_bsp_id(key: tuple[int, str, str, int]) -> int:
    """身份键 → 稳定的正整数 bsp_id（跨运行确定性）。"""
    raw = f"{key[0]}|{key[1]}|{key[2]}|{key[3]}"
    digest = hashlib.sha256(raw.encode("utf-8")).digest()
    return int.from_bytes(digest[:4], "big") & 0x7FFFFFFF


def diff_buysellpoints(
    prev: list[BuySellPoint],
    curr: list[BuySellPoint],
    *,
    bar_idx: int,
    bar_ts: float,
    seq_start: int = 0,
) -> list[DomainEvent]:
    """比较前后两次 BSP 列表，产生域事件。

    使用身份键映射 diff（非位置对位），正确处理中间删除/插入。

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
    invalidate_events: list[DomainEvent] = []
    update_events: list[DomainEvent] = []
    seq = seq_start

    # ── 构建身份键映射 ──
    prev_map: dict[tuple[int, str, str, int], BuySellPoint] = {
        bsp_identity_key(bp): bp for bp in prev
    }
    curr_map: dict[tuple[int, str, str, int], BuySellPoint] = {
        bsp_identity_key(bp): bp for bp in curr
    }

    def _append_inv(cls: type, **kwargs: object) -> None:
        nonlocal seq
        eid = compute_event_id(
            bar_idx=bar_idx,
            bar_ts=bar_ts,
            event_type=cls.__dataclass_fields__["event_type"].default,
            seq=seq,
            payload=dict(kwargs),
        )
        invalidate_events.append(
            cls(bar_idx=bar_idx, bar_ts=bar_ts, seq=seq, event_id=eid, **kwargs)
        )
        seq += 1

    def _append_upd(cls: type, **kwargs: object) -> None:
        nonlocal seq
        eid = compute_event_id(
            bar_idx=bar_idx,
            bar_ts=bar_ts,
            event_type=cls.__dataclass_fields__["event_type"].default,
            seq=seq,
            payload=dict(kwargs),
        )
        update_events.append(
            cls(bar_idx=bar_idx, bar_ts=bar_ts, seq=seq, event_id=eid, **kwargs)
        )
        seq += 1

    prev_keys = set(prev_map.keys())
    curr_keys = set(curr_map.keys())

    # ── 消失的 BSP → Invalidate（按 key 排序确保确定性）──
    for key in sorted(prev_keys - curr_keys):
        bp = prev_map[key]
        _append_inv(
            BuySellPointInvalidateV1,
            bsp_id=_stable_bsp_id(key),
            kind=bp.kind,
            side=bp.side,
            level_id=bp.level_id,
            seg_idx=bp.seg_idx,
        )

    # ── 新增或状态变化的 BSP（按 key 排序确保确定性）──
    for key in sorted(curr_keys):
        bp = curr_map[key]
        bid = _stable_bsp_id(key)
        old = prev_map.get(key)

        if old is None:
            # 全新 BSP
            _append_upd(
                BuySellPointCandidateV1,
                bsp_id=bid,
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
                _append_upd(
                    BuySellPointConfirmV1,
                    bsp_id=bid,
                    kind=bp.kind,
                    side=bp.side,
                    level_id=bp.level_id,
                    seg_idx=bp.seg_idx,
                    price=bp.price,
                )
        else:
            # 同身份 BSP — 检查状态变化
            if not old.confirmed and bp.confirmed:
                _append_upd(
                    BuySellPointConfirmV1,
                    bsp_id=bid,
                    kind=bp.kind,
                    side=bp.side,
                    level_id=bp.level_id,
                    seg_idx=bp.seg_idx,
                    price=bp.price,
                )
            elif not old.settled and bp.settled:
                _append_upd(
                    BuySellPointSettleV1,
                    bsp_id=bid,
                    kind=bp.kind,
                    side=bp.side,
                    level_id=bp.level_id,
                    seg_idx=bp.seg_idx,
                    price=bp.price,
                )
            elif old.price != bp.price or old.overlaps_with != bp.overlaps_with:
                _append_upd(
                    BuySellPointCandidateV1,
                    bsp_id=bid,
                    kind=bp.kind,
                    side=bp.side,
                    level_id=bp.level_id,
                    seg_idx=bp.seg_idx,
                    price=bp.price,
                    move_seg_start=bp.move_seg_start,
                    center_seg_start=bp.center_seg_start,
                    overlaps_with=bp.overlaps_with or "",
                )

    # 因果序：invalidate 在前，candidate/confirm/settle 在后
    return invalidate_events + update_events
