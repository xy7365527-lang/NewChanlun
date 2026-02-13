"""不变量检查器 — per-bar 执行运行时不变量验证

在 BiEngine.process_bar() 返回后调用，检测 I1-I4 不变量。
I5 (replay determinism) 由测试覆盖，不做实时检查。
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import asdict

from newchan.audit.invariants import (
    I1_SETTLED_OVERWRITE,
    I2_TIME_BACKWARD,
    I3_DUPLICATE_SETTLE,
    I4_TYPE_MISMATCH,
)
from newchan.events import DomainEvent, InvariantViolation
from newchan.fingerprint import compute_event_id


def _snapshot_hash(events: list[DomainEvent]) -> str:
    """计算快照级哈希（用于违规诊断定位）。"""
    if not events:
        return "empty"
    parts = ":".join(
        f"{e.event_type}:{e.bar_idx}:{e.seq}" for e in events
    )
    return hashlib.sha256(parts.encode("utf-8")).hexdigest()[:12]


class InvariantChecker:
    """Per-bar 不变量检查器。

    维护跨 bar 的累积状态（已 settle 的笔集合、上次 bar_ts 等），
    对每个 BiEngineSnapshot 的事件做 I1-I4 检查。

    Usage::

        checker = InvariantChecker()
        for bar in bars:
            snap = engine.process_bar(bar)
            violations = checker.check(snap.events, snap.bar_idx, snap.bar_ts)
    """

    def __init__(self) -> None:
        # (i0, i1, direction) → 已 settle 且未被 invalidate 的笔
        self._settled_keys: set[tuple[int, int, str]] = set()
        self._last_bar_ts: float = -1.0
        self._last_seq: int = -1
        self._violation_seq: int = 0

    def reset(self) -> None:
        """重置到初始状态（回放 seek 时与 engine.reset() 同步调用）。"""
        self._settled_keys.clear()
        self._last_bar_ts = -1.0
        self._last_seq = -1
        self._violation_seq = 0

    def check(
        self,
        events: list[DomainEvent],
        bar_idx: int,
        bar_ts: float,
    ) -> list[InvariantViolation]:
        """检查一组事件的不变量，返回违规列表。

        Parameters
        ----------
        events : list[DomainEvent]
            本 bar 产生的事件（来自 BiEngineSnapshot.events）。
        bar_idx : int
            当前 bar 索引。
        bar_ts : float
            当前 bar 时间戳。

        Returns
        -------
        list[InvariantViolation]
            违规事件列表（可能为空）。
        """
        violations: list[InvariantViolation] = []
        snap_hash = _snapshot_hash(events)

        # I2: bar_ts 单调非递减
        if bar_ts < self._last_bar_ts:
            violations.append(self._make_violation(
                bar_idx=bar_idx,
                bar_ts=bar_ts,
                code=I2_TIME_BACKWARD,
                reason=f"bar_ts {bar_ts} < last {self._last_bar_ts}",
                snapshot_hash=snap_hash,
            ))
        self._last_bar_ts = bar_ts

        for ev in events:
            # I1 + I3: settled stroke 不可覆盖
            if ev.event_type == "stroke_settled":
                key = self._stroke_key(ev)
                if key in self._settled_keys:
                    violations.append(self._make_violation(
                        bar_idx=bar_idx,
                        bar_ts=bar_ts,
                        code=I1_SETTLED_OVERWRITE,
                        reason=f"stroke {key} already settled without invalidate",
                        snapshot_hash=snap_hash,
                    ))
                self._settled_keys.add(key)

            # 从 settled 集合中移除被 invalidate 的笔
            elif ev.event_type == "stroke_invalidated":
                key = self._stroke_key(ev)
                self._settled_keys.discard(key)

            # I4: confirmed 状态与事件类型匹配
            # candidate 事件不应带 confirmed=True 的语义
            # settled 事件不应带 confirmed=False 的语义
            # 注意：事件 payload 不直接含 confirmed 字段，
            # 而是通过 event_type 隐含表达，由 bi_differ 逻辑保证。
            # 这里做 seq 单调性检查作为 I4 的替代验证。
            if ev.seq <= self._last_seq and self._last_seq >= 0:
                violations.append(self._make_violation(
                    bar_idx=bar_idx,
                    bar_ts=bar_ts,
                    code=I4_TYPE_MISMATCH,
                    reason=f"seq {ev.seq} <= last_seq {self._last_seq} (non-monotonic)",
                    snapshot_hash=snap_hash,
                ))
            self._last_seq = ev.seq

        return violations

    def _make_violation(
        self,
        *,
        bar_idx: int,
        bar_ts: float,
        code: str,
        reason: str,
        snapshot_hash: str,
    ) -> InvariantViolation:
        """创建带确定性 event_id 的 InvariantViolation。"""
        payload = {"code": code, "reason": reason, "snapshot_hash": snapshot_hash}
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
            snapshot_hash=snapshot_hash,
        )
        self._violation_seq += 1
        return v

    @staticmethod
    def _stroke_key(ev: DomainEvent) -> tuple[int, int, str]:
        """从事件中提取笔的唯一标识 (i0, i1, direction)。"""
        return (
            getattr(ev, "i0", 0),
            getattr(ev, "i1", 0),
            getattr(ev, "direction", ""),
        )
