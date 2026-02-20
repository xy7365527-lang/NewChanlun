"""中枢不变量检查器 — I11-I14

在 ZhongshuEngine.process_segment_snapshot() 返回后调用，
检测中枢层不变量。I15 由测试覆盖，不做实时检查。
"""

from __future__ import annotations

from newchan.audit.invariants import (
    I11_ZHONGSHU_OVERLAP,
    I12_CANDIDATE_BEFORE_SETTLE,
    I13_PARENTS_TRACEABLE,
    I14_ZHONGSHU_INVALIDATE_IDEMPOTENT,
    I17_INVALIDATE_IS_TERMINAL,
)
from newchan.events import (
    DomainEvent,
    InvariantViolation,
    ZhongshuCandidateV1,
    ZhongshuInvalidateV1,
    ZhongshuSettleV1,
)
from newchan.fingerprint import compute_event_id


class ZhongshuInvariantChecker:
    """Per-bar 中枢不变量检查器。

    维护跨 bar 的累积状态，对每个 ZhongshuSnapshot 的事件做 I11-I14 检查。

    Usage::

        checker = ZhongshuInvariantChecker()
        for bar in bars:
            zs_snap = zs_engine.process_segment_snapshot(seg_snap)
            violations = checker.check(zs_snap.events, snap.bar_idx, snap.bar_ts)
    """

    def __init__(self) -> None:
        # zhongshu_id → 已见 candidate（等待 settle 或 invalidate）
        self._candidate_ids: set[int] = set()
        # (zd, zg, seg_start, seg_end) → 已 invalidate 的中枢 key
        self._invalidated_keys: set[tuple[float, float, int, int]] = set()
        # I17: (zd, zg, seg_start) → 已 invalidate 的身份键（终态）
        self._terminal_identities: set[tuple[float, float, int]] = set()
        self._violation_seq: int = 0

    def reset(self) -> None:
        """重置（回放 seek 时调用）。"""
        self._candidate_ids.clear()
        self._invalidated_keys.clear()
        self._terminal_identities.clear()
        self._violation_seq = 0

    def check(
        self,
        events: list[DomainEvent],
        bar_idx: int,
        bar_ts: float,
    ) -> list[InvariantViolation]:
        """检查一组中枢事件的不变量。"""
        violations: list[InvariantViolation] = []
        batch_candidate_ids: set[int] = set()

        for ev in events:
            if isinstance(ev, ZhongshuCandidateV1):
                violations.extend(
                    self._check_candidate(ev, bar_idx, bar_ts),
                )
                self._candidate_ids.add(ev.zhongshu_id)
                batch_candidate_ids.add(ev.zhongshu_id)

            elif isinstance(ev, ZhongshuSettleV1):
                violations.extend(
                    self._check_settle(ev, bar_idx, bar_ts, batch_candidate_ids),
                )

            elif isinstance(ev, ZhongshuInvalidateV1):
                violations.extend(
                    self._check_invalidate(ev, bar_idx, bar_ts),
                )

        return violations

    def _check_candidate(
        self,
        ev: ZhongshuCandidateV1,
        bar_idx: int,
        bar_ts: float,
    ) -> list[InvariantViolation]:
        """I17 终态 + I11 重叠 + I13 可追溯。"""
        violations: list[InvariantViolation] = []
        identity = (ev.zd, ev.zg, ev.seg_start)

        # I17: invalidate 后不得出现同身份 candidate
        if identity in self._terminal_identities:
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I17_INVALIDATE_IS_TERMINAL,
                reason=(
                    f"zhongshu {ev.zhongshu_id}: candidate after "
                    f"invalidate for identity {identity}"
                ),
            ))

        # I11: candidate 的 zg > zd（三段重叠成立条件）
        if ev.zg <= ev.zd:
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I11_ZHONGSHU_OVERLAP,
                reason=(
                    f"zhongshu {ev.zhongshu_id}: "
                    f"zg={ev.zg} <= zd={ev.zd}, overlap not satisfied"
                ),
            ))

        # I13: seg_start < seg_end 且 seg_count >= 3
        if ev.seg_end < ev.seg_start or ev.seg_count < 3:
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I13_PARENTS_TRACEABLE,
                reason=(
                    f"zhongshu {ev.zhongshu_id}: "
                    f"seg_start={ev.seg_start}, seg_end={ev.seg_end}, "
                    f"seg_count={ev.seg_count} invalid"
                ),
            ))

        return violations

    def _check_settle(
        self,
        ev: ZhongshuSettleV1,
        bar_idx: int,
        bar_ts: float,
        batch_candidate_ids: set[int],
    ) -> list[InvariantViolation]:
        """I17 终态 + I12 前置 candidate + I11 重叠。"""
        violations: list[InvariantViolation] = []
        identity = (ev.zd, ev.zg, ev.seg_start)

        # I17: invalidate 后不得出现同身份 settle
        if identity in self._terminal_identities:
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I17_INVALIDATE_IS_TERMINAL,
                reason=(
                    f"zhongshu {ev.zhongshu_id}: settle after "
                    f"invalidate for identity {identity}"
                ),
            ))

        # I12: settle 前必须有 candidate
        if (
            ev.zhongshu_id not in self._candidate_ids
            and ev.zhongshu_id not in batch_candidate_ids
        ):
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I12_CANDIDATE_BEFORE_SETTLE,
                reason=(
                    f"zhongshu {ev.zhongshu_id} settled without "
                    f"prior candidate"
                ),
            ))

        # I11: settle 的 zg > zd 同样成立
        if ev.zg <= ev.zd:
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I11_ZHONGSHU_OVERLAP,
                reason=(
                    f"zhongshu {ev.zhongshu_id}: "
                    f"zg={ev.zg} <= zd={ev.zd} in settle event"
                ),
            ))

        # 更新状态
        self._candidate_ids.discard(ev.zhongshu_id)

        return violations

    def _check_invalidate(
        self,
        ev: ZhongshuInvalidateV1,
        bar_idx: int,
        bar_ts: float,
    ) -> list[InvariantViolation]:
        """I17 终态记录 + I14 幂等检查。"""
        violations: list[InvariantViolation] = []
        identity = (ev.zd, ev.zg, ev.seg_start)
        self._terminal_identities.add(identity)

        key = (ev.zd, ev.zg, ev.seg_start, ev.seg_end)
        if key in self._invalidated_keys:
            violations.append(self._make_violation(
                bar_idx=bar_idx, bar_ts=bar_ts,
                code=I14_ZHONGSHU_INVALIDATE_IDEMPOTENT,
                reason=(
                    f"zhongshu (zd={ev.zd}, zg={ev.zg}, "
                    f"seg_start={ev.seg_start}, seg_end={ev.seg_end}) "
                    f"already invalidated"
                ),
            ))
        self._invalidated_keys.add(key)
        self._candidate_ids.discard(ev.zhongshu_id)

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
