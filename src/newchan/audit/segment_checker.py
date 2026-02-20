"""线段不变量检查器 — I6-I10, I17

在 SegmentEngine.process_snapshot() 返回后调用，
检测线段层不变量。I10 由测试覆盖，不做实时检查。
I17（invalidate 终态）跨层适用。
"""

from __future__ import annotations

from newchan.audit.invariants import (
    I6_PENDING_DIRECT_SETTLE,
    I7_SETTLE_ANCHOR,
    I9_INVALIDATE_IDEMPOTENT,
    I17_INVALIDATE_IS_TERMINAL,
)
from newchan.events import (
    DomainEvent,
    InvariantViolation,
    SegmentBreakPendingV1,
    SegmentInvalidateV1,
    SegmentSettleV1,
)
from newchan.fingerprint import compute_event_id


class SegmentInvariantChecker:
    """Per-bar 线段不变量检查器。

    维护跨 bar 的累积状态，对每个 SegmentSnapshot 的事件做 I6-I9, I17 检查。

    Usage::

        checker = SegmentInvariantChecker()
        for bar in bars:
            seg_snap = seg_engine.process_snapshot(bi_snap)
            violations = checker.check(seg_snap.events, snap.bar_idx, snap.bar_ts)
    """

    def __init__(self) -> None:
        # segment_id → 已见 pending 的 segment（等待 settle 确认）
        self._pending_ids: set[int] = set()
        # (s0, s1, direction) → 已 settle 且未被 invalidate 的段
        self._settled_keys: set[tuple[int, int, str]] = set()
        # I9: (s0, s1, direction) → 已被 invalidate 且未重新 settle 的段
        self._invalidated_keys: set[tuple[int, int, str]] = set()
        # I17: (s0, direction) → 已 invalidate 的身份键（终态）
        self._terminal_identities: set[tuple[int, str]] = set()
        self._violation_seq: int = 0

    def reset(self) -> None:
        """重置（回放 seek 时调用）。"""
        self._pending_ids.clear()
        self._settled_keys.clear()
        self._invalidated_keys.clear()
        self._terminal_identities.clear()
        self._violation_seq = 0

    def check(
        self,
        events: list[DomainEvent],
        bar_idx: int,
        bar_ts: float,
    ) -> list[InvariantViolation]:
        """检查一组线段事件的不变量。"""
        violations: list[InvariantViolation] = []
        batch_pending_ids: set[int] = set()

        for ev in events:
            if isinstance(ev, SegmentBreakPendingV1):
                violations.extend(
                    self._check_pending(ev, bar_idx, bar_ts),
                )
                self._pending_ids.add(ev.segment_id)
                batch_pending_ids.add(ev.segment_id)

            elif isinstance(ev, SegmentSettleV1):
                violations.extend(
                    self._check_settle(ev, bar_idx, bar_ts, batch_pending_ids),
                )

            elif isinstance(ev, SegmentInvalidateV1):
                violations.extend(
                    self._check_invalidate(ev, bar_idx, bar_ts),
                )

        return violations

    def _check_pending(
        self,
        ev: SegmentBreakPendingV1,
        bar_idx: int,
        bar_ts: float,
    ) -> list[InvariantViolation]:
        """I17: invalidate 后不得出现同身份 pending。"""
        identity = (ev.s0, ev.direction)
        if identity in self._terminal_identities:
            return [self._make_violation(
                bar_idx=bar_idx,
                bar_ts=bar_ts,
                code=I17_INVALIDATE_IS_TERMINAL,
                reason=(
                    f"segment {ev.segment_id}: pending after "
                    f"invalidate for identity {identity}"
                ),
            )]
        return []

    def _check_settle(
        self,
        ev: SegmentSettleV1,
        bar_idx: int,
        bar_ts: float,
        batch_pending_ids: set[int],
    ) -> list[InvariantViolation]:
        """I17 终态 + I6 前置 pending + I7 结算锚。"""
        violations: list[InvariantViolation] = []
        violations.extend(self._check_settle_invariants(ev, bar_idx, bar_ts, batch_pending_ids))
        self._update_settle_state(ev)
        return violations

    def _check_settle_invariants(
        self,
        ev: SegmentSettleV1,
        bar_idx: int,
        bar_ts: float,
        batch_pending_ids: set[int],
    ) -> list[InvariantViolation]:
        """I17/I6/I7 settle 不变量检查（不修改状态）。"""
        violations: list[InvariantViolation] = []
        identity = (ev.s0, ev.direction)

        # I17: invalidate 后不得出现同身份 settle
        if identity in self._terminal_identities:
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I17_INVALIDATE_IS_TERMINAL,
                reason=(
                    f"segment {ev.segment_id}: settle after "
                    f"invalidate for identity {identity}"
                ),
            ))

        # I6: settle 前必须有 pending
        if (
            ev.segment_id not in self._pending_ids
            and ev.segment_id not in batch_pending_ids
        ):
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I6_PENDING_DIRECT_SETTLE,
                reason=(
                    f"segment {ev.segment_id} settled without "
                    f"prior pending_break"
                ),
            ))

        # I7: 结算锚 s1 == new_segment_s0 - 1
        if ev.s1 != ev.new_segment_s0 - 1:
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I7_SETTLE_ANCHOR,
                reason=(
                    f"segment {ev.segment_id}: "
                    f"s1={ev.s1} != new_segment_s0-1={ev.new_segment_s0 - 1}"
                ),
            ))

        return violations

    def _update_settle_state(self, ev: SegmentSettleV1) -> None:
        """settle 后更新累积状态。"""
        self._pending_ids.discard(ev.segment_id)
        key = (ev.s0, ev.s1, ev.direction)
        self._settled_keys.add(key)
        self._invalidated_keys.discard(key)

    def _check_invalidate(
        self,
        ev: SegmentInvalidateV1,
        bar_idx: int,
        bar_ts: float,
    ) -> list[InvariantViolation]:
        """I17 终态记录 + I9 幂等检查。"""
        violations: list[InvariantViolation] = []
        identity = (ev.s0, ev.direction)
        self._terminal_identities.add(identity)

        key = (ev.s0, ev.s1, ev.direction)
        if key in self._invalidated_keys:
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I9_INVALIDATE_IDEMPOTENT,
                reason=(
                    f"segment ({ev.s0}, {ev.s1}, {ev.direction}): "
                    f"duplicate invalidate without intervening settle"
                ),
            ))
        self._invalidated_keys.add(key)
        self._settled_keys.discard(key)

        return violations

    def _make_violation(
        self,
        *,
        bar_idx: int,
        bar_ts: float,
        code: str,
        reason: str,
    ) -> InvariantViolation:
        """创建带确定性 event_id 的 InvariantViolation。"""
        payload = {"code": code, "reason": reason, "snapshot_hash": ""}
        eid = compute_event_id(
            bar_idx=bar_idx,
            bar_ts=bar_ts,
            event_type="invariant_violation",
            seq=self._violation_seq,
            payload=payload,
        )
        v = InvariantViolation(
            bar_idx=bar_idx,
            bar_ts=bar_ts,
            seq=self._violation_seq,
            event_id=eid,
            code=code,
            reason=reason,
            snapshot_hash="",
        )
        self._violation_seq += 1
        return v
