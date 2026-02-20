"""走势类型不变量检查器 — I18-I21

在 MoveEngine.process_zhongshu_snapshot() 返回后调用，
检测走势类型层不变量。I22 由测试覆盖，不做实时检查。
I21（invalidate 终态）跨层适用。
"""

from __future__ import annotations

from newchan.audit.invariants import (
    I18_MOVE_MIN_CENTER,
    I19_MOVE_CANDIDATE_BEFORE_SETTLE,
    I20_MOVE_PARENTS_TRACEABLE,
    I21_MOVE_INVALIDATE_TERMINAL,
)
from newchan.events import (
    DomainEvent,
    InvariantViolation,
    MoveCandidateV1,
    MoveInvalidateV1,
    MoveSettleV1,
)
from newchan.fingerprint import compute_event_id


class MoveInvariantChecker:
    """Per-bar 走势类型不变量检查器。

    维护跨 bar 的累积状态，对每个 MoveSnapshot 的事件做 I18-I21 检查。

    Usage::

        checker = MoveInvariantChecker()
        for bar in bars:
            move_snap = move_engine.process_zhongshu_snapshot(zs_snap)
            violations = checker.check(move_snap.events, snap.bar_idx, snap.bar_ts)
    """

    def __init__(self) -> None:
        # move_id → 已见 candidate（等待 settle 或 invalidate）
        self._candidate_ids: set[int] = set()
        # I21: (seg_start,) → 已 invalidate 的身份键（终态）
        self._terminal_identities: set[tuple[int]] = set()
        self._violation_seq: int = 0

    def reset(self) -> None:
        """重置（回放 seek 时调用）。"""
        self._candidate_ids.clear()
        self._terminal_identities.clear()
        self._violation_seq = 0

    def check(
        self,
        events: list[DomainEvent],
        bar_idx: int,
        bar_ts: float,
    ) -> list[InvariantViolation]:
        """检查一组 Move 事件的不变量。"""
        violations: list[InvariantViolation] = []
        batch_candidate_ids: set[int] = set()

        for ev in events:
            if isinstance(ev, MoveCandidateV1):
                violations.extend(
                    self._check_candidate(ev, bar_idx, bar_ts),
                )
                self._candidate_ids.add(ev.move_id)
                batch_candidate_ids.add(ev.move_id)

            elif isinstance(ev, MoveSettleV1):
                violations.extend(
                    self._check_settle(ev, bar_idx, bar_ts, batch_candidate_ids),
                )

            elif isinstance(ev, MoveInvalidateV1):
                self._check_invalidate(ev)

        return violations

    def _check_candidate(
        self,
        ev: MoveCandidateV1,
        bar_idx: int,
        bar_ts: float,
    ) -> list[InvariantViolation]:
        """I21 终态 + I18 最小中枢 + I20 可追溯。"""
        violations: list[InvariantViolation] = []
        identity = (ev.seg_start,)

        # I21: invalidate 后不得出现同身份 candidate
        if identity in self._terminal_identities:
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I21_MOVE_INVALIDATE_TERMINAL,
                reason=(
                    f"move {ev.move_id}: candidate after "
                    f"invalidate for identity {identity}"
                ),
            ))

        # I18: zs_count >= 1
        if ev.zs_count < 1:
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I18_MOVE_MIN_CENTER,
                reason=f"move {ev.move_id}: zs_count={ev.zs_count} < 1",
            ))

        # I20: zs_end >= zs_start 且 zs_count >= 1
        if ev.zs_end < ev.zs_start or ev.zs_count < 1:
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I20_MOVE_PARENTS_TRACEABLE,
                reason=(
                    f"move {ev.move_id}: "
                    f"zs_start={ev.zs_start}, zs_end={ev.zs_end}, "
                    f"zs_count={ev.zs_count} invalid"
                ),
            ))

        return violations

    def _check_settle(
        self,
        ev: MoveSettleV1,
        bar_idx: int,
        bar_ts: float,
        batch_candidate_ids: set[int],
    ) -> list[InvariantViolation]:
        """I21 终态 + I19 前置 candidate。"""
        violations: list[InvariantViolation] = []
        identity = (ev.seg_start,)

        # I21: invalidate 后不得出现同身份 settle
        if identity in self._terminal_identities:
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I21_MOVE_INVALIDATE_TERMINAL,
                reason=(
                    f"move {ev.move_id}: settle after "
                    f"invalidate for identity {identity}"
                ),
            ))

        # I19: settle 前必须有 candidate
        if (
            ev.move_id not in self._candidate_ids
            and ev.move_id not in batch_candidate_ids
        ):
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I19_MOVE_CANDIDATE_BEFORE_SETTLE,
                reason=(
                    f"move {ev.move_id} settled without "
                    f"prior candidate"
                ),
            ))

        # 更新状态
        self._candidate_ids.discard(ev.move_id)
        return violations

    def _check_invalidate(self, ev: MoveInvalidateV1) -> None:
        """I21: 记录身份键为终态。"""
        identity = (ev.seg_start,)
        self._terminal_identities.add(identity)
        self._candidate_ids.discard(ev.move_id)

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
