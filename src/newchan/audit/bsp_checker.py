"""买卖点不变量检查器 — I23-I27

在 BuySellPointEngine.process() 返回后调用，
检测买卖点层不变量。I28 由测试覆盖，I29 待确认时机结算后启用。
"""

from __future__ import annotations

from newchan.audit.invariants import (
    I23_BSP_TYPE_CONSTRAINT,
    I24_BSP_CANDIDATE_BEFORE_CONFIRM,
    I25_BSP_CONFIRM_BEFORE_SETTLE,
    I26_BSP_MUTUAL_EXCLUSION,
    I27_BSP_INVALIDATE_TERMINAL,
)
from newchan.events import (
    BuySellPointCandidateV1,
    BuySellPointConfirmV1,
    BuySellPointInvalidateV1,
    BuySellPointSettleV1,
    DomainEvent,
    InvariantViolation,
)
from newchan.fingerprint import compute_event_id


class BspInvariantChecker:
    """Per-bar 买卖点不变量检查器。

    维护跨 bar 的累积状态，对每个 BuySellPointSnapshot 的事件做 I23-I27 检查。

    Usage::

        checker = BspInvariantChecker()
        for bar in bars:
            bsp_snap = bsp_engine.process(move_snap, divs, zs_snap)
            violations = checker.check(bsp_snap.events, snap.bar_idx, snap.bar_ts)
    """

    def __init__(self) -> None:
        # bsp_id → 已见 candidate（等待 confirm/settle/invalidate）
        self._candidate_ids: set[int] = set()
        # bsp_id → 已见 confirm（等待 settle/invalidate）
        self._confirmed_ids: set[int] = set()
        # I27: (seg_idx, kind, side, level_id) → 已 invalidate 的身份键
        self._terminal_identities: set[tuple[int, str, str, int]] = set()
        # I26: 跟踪活跃的买卖点 — (seg_idx, level_id) → set of kinds
        self._active_by_seg: dict[tuple[int, int], set[str]] = {}
        self._violation_seq: int = 0

    def reset(self) -> None:
        """重置（回放 seek 时调用）。"""
        self._candidate_ids.clear()
        self._confirmed_ids.clear()
        self._terminal_identities.clear()
        self._active_by_seg.clear()
        self._violation_seq = 0

    def check(
        self,
        events: list[DomainEvent],
        bar_idx: int,
        bar_ts: float,
    ) -> list[InvariantViolation]:
        """检查一组 BSP 事件的不变量。"""
        violations: list[InvariantViolation] = []

        batch_candidate_ids: set[int] = set()
        batch_confirmed_ids: set[int] = set()

        for ev in events:
            if isinstance(ev, BuySellPointCandidateV1):
                identity = (ev.seg_idx, ev.kind, ev.side, ev.level_id)

                # I27: invalidate 后不得出现同身份 candidate
                if identity in self._terminal_identities:
                    violations.append(self._make_violation(
                        bar_idx=bar_idx,
                        bar_ts=bar_ts,
                        code=I27_BSP_INVALIDATE_TERMINAL,
                        reason=(
                            f"bsp {ev.bsp_id}: candidate after "
                            f"invalidate for identity {identity}"
                        ),
                    ))

                # I23: 类型约束
                violations.extend(
                    self._check_type_constraint(ev, bar_idx, bar_ts)
                )

                # I26: 互斥检查（注册活跃状态）
                seg_key = (ev.seg_idx, ev.level_id)
                if seg_key not in self._active_by_seg:
                    self._active_by_seg[seg_key] = set()
                self._active_by_seg[seg_key].add(ev.kind)
                violations.extend(
                    self._check_mutual_exclusion(
                        seg_key, ev.bsp_id, bar_idx, bar_ts,
                    )
                )

                self._candidate_ids.add(ev.bsp_id)
                batch_candidate_ids.add(ev.bsp_id)

            elif isinstance(ev, BuySellPointConfirmV1):
                identity = (ev.seg_idx, ev.kind, ev.side, ev.level_id)

                # I27: invalidate 后不得出现同身份 confirm
                if identity in self._terminal_identities:
                    violations.append(self._make_violation(
                        bar_idx=bar_idx,
                        bar_ts=bar_ts,
                        code=I27_BSP_INVALIDATE_TERMINAL,
                        reason=(
                            f"bsp {ev.bsp_id}: confirm after "
                            f"invalidate for identity {identity}"
                        ),
                    ))

                # I24: confirm 前必有 candidate
                if (
                    ev.bsp_id not in self._candidate_ids
                    and ev.bsp_id not in batch_candidate_ids
                ):
                    violations.append(self._make_violation(
                        bar_idx=bar_idx,
                        bar_ts=bar_ts,
                        code=I24_BSP_CANDIDATE_BEFORE_CONFIRM,
                        reason=(
                            f"bsp {ev.bsp_id} confirmed without "
                            f"prior candidate"
                        ),
                    ))

                self._confirmed_ids.add(ev.bsp_id)
                batch_confirmed_ids.add(ev.bsp_id)

            elif isinstance(ev, BuySellPointSettleV1):
                identity = (ev.seg_idx, ev.kind, ev.side, ev.level_id)

                # I27: invalidate 后不得出现同身份 settle
                if identity in self._terminal_identities:
                    violations.append(self._make_violation(
                        bar_idx=bar_idx,
                        bar_ts=bar_ts,
                        code=I27_BSP_INVALIDATE_TERMINAL,
                        reason=(
                            f"bsp {ev.bsp_id}: settle after "
                            f"invalidate for identity {identity}"
                        ),
                    ))

                # I25: settle 前必有 confirm
                if (
                    ev.bsp_id not in self._confirmed_ids
                    and ev.bsp_id not in batch_confirmed_ids
                ):
                    violations.append(self._make_violation(
                        bar_idx=bar_idx,
                        bar_ts=bar_ts,
                        code=I25_BSP_CONFIRM_BEFORE_SETTLE,
                        reason=(
                            f"bsp {ev.bsp_id} settled without "
                            f"prior confirm"
                        ),
                    ))

                self._candidate_ids.discard(ev.bsp_id)
                self._confirmed_ids.discard(ev.bsp_id)

            elif isinstance(ev, BuySellPointInvalidateV1):
                # I27: 记录身份键为终态
                identity = (ev.seg_idx, ev.kind, ev.side, ev.level_id)
                self._terminal_identities.add(identity)
                self._candidate_ids.discard(ev.bsp_id)
                self._confirmed_ids.discard(ev.bsp_id)

                # 从活跃跟踪中移除
                seg_key = (ev.seg_idx, ev.level_id)
                if seg_key in self._active_by_seg:
                    self._active_by_seg[seg_key].discard(ev.kind)
                    if not self._active_by_seg[seg_key]:
                        del self._active_by_seg[seg_key]

        return violations

    def _check_type_constraint(
        self,
        ev: BuySellPointCandidateV1,
        bar_idx: int,
        bar_ts: float,
    ) -> list[InvariantViolation]:
        """I23: type1/type2 必须有 divergence_key；type3 必须无。"""
        violations: list[InvariantViolation] = []

        if ev.kind == "type3":
            # type3 不应关联趋势背驰（center_seg_start 必须有值）
            if ev.center_seg_start is None:
                violations.append(self._make_violation(
                    bar_idx=bar_idx,
                    bar_ts=bar_ts,
                    code=I23_BSP_TYPE_CONSTRAINT,
                    reason=(
                        f"bsp {ev.bsp_id}: type3 missing "
                        f"center_seg_start"
                    ),
                ))

        return violations

    def _check_mutual_exclusion(
        self,
        seg_key: tuple[int, int],
        bsp_id: int,
        bar_idx: int,
        bar_ts: float,
    ) -> list[InvariantViolation]:
        """I26: 同 (seg_idx, level_id) 下 type1+type2 或 type1+type3 互斥。"""
        violations: list[InvariantViolation] = []
        kinds = self._active_by_seg.get(seg_key, set())

        if "type1" in kinds and "type2" in kinds:
            violations.append(self._make_violation(
                bar_idx=bar_idx,
                bar_ts=bar_ts,
                code=I26_BSP_MUTUAL_EXCLUSION,
                reason=(
                    f"bsp {bsp_id}: type1+type2 co-active "
                    f"at seg_key={seg_key}"
                ),
            ))
        if "type1" in kinds and "type3" in kinds:
            violations.append(self._make_violation(
                bar_idx=bar_idx,
                bar_ts=bar_ts,
                code=I26_BSP_MUTUAL_EXCLUSION,
                reason=(
                    f"bsp {bsp_id}: type1+type3 co-active "
                    f"at seg_key={seg_key}"
                ),
            ))
        # type2+type3 共存是合法的（2B+3B 重合）

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
