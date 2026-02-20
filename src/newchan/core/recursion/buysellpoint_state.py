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
from typing import Callable

from newchan.a_buysellpoint_v1 import BuySellPoint
from newchan.core.diff.helpers import make_appender
from newchan.core.diff.identity import bsp_identity_key
from newchan.events import (
    BuySellPointCandidateV1,
    BuySellPointConfirmV1,
    BuySellPointInvalidateV1,
    BuySellPointSettleV1,
    DomainEvent,
)


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


def _handle_bsp_new(
    _append: Callable[..., None],
    bp: BuySellPoint,
    bid: int,
) -> None:
    """全新 BSP 的事件处理。"""
    _append(
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
        _append(
            BuySellPointConfirmV1,
            bsp_id=bid,
            kind=bp.kind,
            side=bp.side,
            level_id=bp.level_id,
            seg_idx=bp.seg_idx,
            price=bp.price,
        )


def _handle_bsp_state_change(
    _append: Callable[..., None],
    old: BuySellPoint,
    bp: BuySellPoint,
    bid: int,
) -> None:
    """同身份 BSP 的状态变化处理。"""
    if not old.confirmed and bp.confirmed:
        _append(
            BuySellPointConfirmV1,
            bsp_id=bid,
            kind=bp.kind,
            side=bp.side,
            level_id=bp.level_id,
            seg_idx=bp.seg_idx,
            price=bp.price,
        )
    elif not old.settled and bp.settled:
        _append(
            BuySellPointSettleV1,
            bsp_id=bid,
            kind=bp.kind,
            side=bp.side,
            level_id=bp.level_id,
            seg_idx=bp.seg_idx,
            price=bp.price,
        )
    elif old.price != bp.price or old.overlaps_with != bp.overlaps_with:
        _append(
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


def _emit_bsp_invalidates(
    _append: Callable[..., None],
    prev_map: dict[tuple[int, str, str, int], BuySellPoint],
    removed_keys: set[tuple[int, str, str, int]],
) -> None:
    """消失的 BSP → Invalidate（按 key 排序确保确定性）。"""
    for key in sorted(removed_keys):
        bp = prev_map[key]
        _append(
            BuySellPointInvalidateV1,
            bsp_id=_stable_bsp_id(key),
            kind=bp.kind,
            side=bp.side,
            level_id=bp.level_id,
            seg_idx=bp.seg_idx,
        )


def diff_buysellpoints(
    prev: list[BuySellPoint],
    curr: list[BuySellPoint],
    *,
    bar_idx: int,
    bar_ts: float,
    seq_start: int = 0,
) -> list[DomainEvent]:
    """比较前后两次 BSP 列表，产生域事件（身份键映射 diff）。"""
    invalidate_events: list[DomainEvent] = []
    update_events: list[DomainEvent] = []
    seq_box = [seq_start]

    _append_inv = make_appender(invalidate_events, bar_idx, bar_ts, seq_box)
    _append_upd = make_appender(update_events, bar_idx, bar_ts, seq_box)

    prev_map = {bsp_identity_key(bp): bp for bp in prev}
    curr_map = {bsp_identity_key(bp): bp for bp in curr}

    _emit_bsp_invalidates(
        _append_inv, prev_map, set(prev_map.keys()) - set(curr_map.keys()),
    )

    for key in sorted(curr_map.keys()):
        bp = curr_map[key]
        bid = _stable_bsp_id(key)
        old = prev_map.get(key)
        if old is None:
            _handle_bsp_new(_append_upd, bp, bid)
        else:
            _handle_bsp_state_change(_append_upd, old, bp, bid)

    return invalidate_events + update_events
