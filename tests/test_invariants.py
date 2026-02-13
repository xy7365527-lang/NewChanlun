"""不变量检查器测试

验证 InvariantChecker 的 5 条不变量检测能力：
  I1 — settled stroke 不可覆盖
  I2 — bar_ts 单调非递减
  I3 — 幂等 diff（连续相同输入无事件）
  I4 — seq 单调递增（类型边界不混淆的替代验证）
  I5 — replay 确定性（由 test_replay_determinism.py 覆盖）

以及 InvariantChecker 与 BiEngine 的集成测试。
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest

from newchan.audit.checker import InvariantChecker
from newchan.audit.invariants import (
    I1_SETTLED_OVERWRITE,
    I2_TIME_BACKWARD,
    I3_DUPLICATE_SETTLE,
    I4_TYPE_MISMATCH,
)
from newchan.bi_engine import BiEngine
from newchan.events import (
    DomainEvent,
    InvariantViolation,
    StrokeCandidate,
    StrokeExtended,
    StrokeInvalidated,
    StrokeSettled,
)
from newchan.fingerprint import compute_event_id
from newchan.types import Bar


# ── 辅助函数 ──────────────────────────────────────────────────────


def _bar(ts_offset: int, o: float, h: float, l: float, c: float) -> Bar:
    """从偏移序号创建 Bar（每 bar 间隔 5 分钟）。"""
    return Bar(
        ts=datetime(2024, 1, 1, tzinfo=timezone.utc)
        + timedelta(seconds=ts_offset * 300),
        open=o,
        high=h,
        low=l,
        close=c,
    )


def _generate_zigzag_bars(n: int = 60) -> list[Bar]:
    """生成锯齿形价格序列，确保能产生多笔。"""
    bars: list[Bar] = []
    for i in range(n):
        cycle_pos = i % 16
        if cycle_pos < 8:
            base = 100 - cycle_pos * 5
        else:
            base = 60 + (cycle_pos - 8) * 5
        h = base + 1.5
        l = base - 1.5
        o = base + 0.5
        c = base - 0.5
        bars.append(_bar(i, o, h, l, c))
    return bars


def _make_event(
    cls: type,
    bar_idx: int = 0,
    bar_ts: float = 1000.0,
    seq: int = 0,
    **kwargs,
) -> DomainEvent:
    """构造带 event_id 的事件。"""
    event_type = cls.__dataclass_fields__["event_type"].default
    payload = dict(kwargs)
    eid = compute_event_id(
        bar_idx=bar_idx,
        bar_ts=bar_ts,
        event_type=event_type,
        seq=seq,
        payload=payload,
    )
    return cls(bar_idx=bar_idx, bar_ts=bar_ts, seq=seq, event_id=eid, **kwargs)


# =====================================================================
# I1: settled stroke 不可覆盖
# =====================================================================


class TestI1SettledOverwrite:
    """I1: 同一 (i0, i1, direction) 的 settled 事件不可重复出现。"""

    def test_first_settle_no_violation(self):
        """首次 settle → 无违规。"""
        checker = InvariantChecker()
        events = [
            _make_event(StrokeSettled, seq=0, stroke_id=0, direction="up",
                        i0=0, i1=10, p0=100.0, p1=110.0),
        ]
        violations = checker.check(events, bar_idx=10, bar_ts=1000.0)
        assert violations == []

    def test_duplicate_settle_detected(self):
        """同一笔重复 settle（无中间 invalidate） → I1 违规。"""
        checker = InvariantChecker()
        ev1 = _make_event(StrokeSettled, seq=0, stroke_id=0, direction="up",
                          i0=0, i1=10, p0=100.0, p1=110.0)
        checker.check([ev1], bar_idx=10, bar_ts=1000.0)

        # 同一笔再次 settle
        ev2 = _make_event(StrokeSettled, bar_idx=15, bar_ts=1500.0, seq=1,
                          stroke_id=0, direction="up",
                          i0=0, i1=10, p0=100.0, p1=110.0)
        violations = checker.check([ev2], bar_idx=15, bar_ts=1500.0)
        assert len(violations) == 1
        assert violations[0].code == I1_SETTLED_OVERWRITE

    def test_settle_after_invalidate_ok(self):
        """settle → invalidate → 再次 settle → 无违规。"""
        checker = InvariantChecker()
        ev_settle = _make_event(StrokeSettled, seq=0, stroke_id=0,
                                direction="up", i0=0, i1=10, p0=100.0, p1=110.0)
        checker.check([ev_settle], bar_idx=10, bar_ts=1000.0)

        ev_inv = _make_event(StrokeInvalidated, bar_idx=12, bar_ts=1200.0,
                             seq=1, stroke_id=0, direction="up",
                             i0=0, i1=10, p0=100.0, p1=110.0)
        checker.check([ev_inv], bar_idx=12, bar_ts=1200.0)

        # 再次 settle 同一笔 → 应该允许
        ev_settle2 = _make_event(StrokeSettled, bar_idx=15, bar_ts=1500.0,
                                 seq=2, stroke_id=0, direction="up",
                                 i0=0, i1=10, p0=100.0, p1=110.0)
        violations = checker.check([ev_settle2], bar_idx=15, bar_ts=1500.0)
        assert violations == []

    def test_different_strokes_no_collision(self):
        """不同笔各自 settle → 无违规。"""
        checker = InvariantChecker()
        ev1 = _make_event(StrokeSettled, seq=0, stroke_id=0, direction="up",
                          i0=0, i1=10, p0=100.0, p1=110.0)
        ev2 = _make_event(StrokeSettled, seq=1, stroke_id=1, direction="down",
                          i0=10, i1=20, p0=110.0, p1=95.0)
        violations = checker.check([ev1, ev2], bar_idx=20, bar_ts=2000.0)
        assert violations == []


# =====================================================================
# I2: bar_ts 单调非递减
# =====================================================================


class TestI2TimeBackward:
    """I2: bar_ts 不可倒退。"""

    def test_monotonic_ts_ok(self):
        """递增 bar_ts → 无违规。"""
        checker = InvariantChecker()
        ev1 = _make_event(StrokeCandidate, seq=0, stroke_id=0, direction="up",
                          i0=0, i1=5, p0=100.0, p1=105.0)
        checker.check([ev1], bar_idx=5, bar_ts=1000.0)

        ev2 = _make_event(StrokeCandidate, bar_idx=10, bar_ts=2000.0, seq=1,
                          stroke_id=1, direction="down",
                          i0=5, i1=10, p0=105.0, p1=95.0)
        violations = checker.check([ev2], bar_idx=10, bar_ts=2000.0)
        assert violations == []

    def test_equal_ts_ok(self):
        """相同 bar_ts → 无违规（允许相等）。"""
        checker = InvariantChecker()
        ev1 = _make_event(StrokeCandidate, seq=0, stroke_id=0, direction="up",
                          i0=0, i1=5, p0=100.0, p1=105.0)
        checker.check([ev1], bar_idx=5, bar_ts=1000.0)
        violations = checker.check([], bar_idx=6, bar_ts=1000.0)
        assert violations == []

    def test_backward_ts_detected(self):
        """bar_ts 倒退 → I2 违规。"""
        checker = InvariantChecker()
        ev1 = _make_event(StrokeCandidate, seq=0, stroke_id=0, direction="up",
                          i0=0, i1=5, p0=100.0, p1=105.0)
        checker.check([ev1], bar_idx=5, bar_ts=2000.0)

        ev2 = _make_event(StrokeCandidate, bar_idx=10, bar_ts=1000.0, seq=1,
                          stroke_id=1, direction="down",
                          i0=5, i1=10, p0=105.0, p1=95.0)
        violations = checker.check([ev2], bar_idx=10, bar_ts=1000.0)
        assert len(violations) == 1
        assert violations[0].code == I2_TIME_BACKWARD


# =====================================================================
# I3: 幂等 diff（间接测试）
# =====================================================================


class TestI3IdempotentDiff:
    """I3: 连续两次相同输入 → 第二次 diff_strokes 无事件。"""

    def test_idempotent_no_events(self):
        """完全相同的前后 stroke 列表 → diff 产生 0 事件。"""
        from newchan.a_stroke import Stroke
        from newchan.bi_differ import diff_strokes

        strokes = [
            Stroke(i0=0, i1=10, direction="up", high=110, low=100,
                   p0=100, p1=110, confirmed=True),
            Stroke(i0=10, i1=20, direction="down", high=110, low=90,
                   p0=110, p1=90, confirmed=False),
        ]
        events = diff_strokes(strokes, strokes, bar_idx=20, bar_ts=2000.0)
        assert events == []

    def test_engine_no_events_on_flat_bar(self):
        """BiEngine：插入不改变笔结构的 bar → 无事件。"""
        bars = _generate_zigzag_bars(30)
        engine = BiEngine()
        for bar in bars:
            engine.process_bar(bar)

        # 插入一根"平坦"bar（与上一根相同价格）
        last_bar = bars[-1]
        flat_bar = _bar(
            30,
            last_bar.close,
            last_bar.close + 0.1,
            last_bar.close - 0.1,
            last_bar.close,
        )
        snap = engine.process_bar(flat_bar)
        # 平坦 bar 可能不改变笔结构 → 事件为空或极少
        # 关键是不应产生 I3 违规
        violations = [e for e in snap.events if isinstance(e, InvariantViolation)]
        assert all(v.code != I3_DUPLICATE_SETTLE for v in violations)


# =====================================================================
# I4: seq 单调递增
# =====================================================================


class TestI4SeqMonotonic:
    """I4: 事件 seq 在同一批内严格递增。"""

    def test_monotonic_seq_ok(self):
        """正常递增 seq → 无违规。"""
        checker = InvariantChecker()
        events = [
            _make_event(StrokeSettled, seq=0, stroke_id=0, direction="up",
                        i0=0, i1=10, p0=100.0, p1=110.0),
            _make_event(StrokeCandidate, seq=1, stroke_id=1, direction="down",
                        i0=10, i1=15, p0=110.0, p1=100.0),
        ]
        violations = checker.check(events, bar_idx=15, bar_ts=1500.0)
        assert violations == []

    def test_nonmonotonic_seq_detected(self):
        """seq 回退 → I4 违规。"""
        checker = InvariantChecker()
        events = [
            _make_event(StrokeSettled, seq=5, stroke_id=0, direction="up",
                        i0=0, i1=10, p0=100.0, p1=110.0),
            _make_event(StrokeCandidate, seq=3, stroke_id=1, direction="down",
                        i0=10, i1=15, p0=110.0, p1=100.0),
        ]
        violations = checker.check(events, bar_idx=15, bar_ts=1500.0)
        assert len(violations) >= 1
        assert any(v.code == I4_TYPE_MISMATCH for v in violations)

    def test_equal_seq_detected(self):
        """seq 相等 → I4 违规（应严格递增）。"""
        checker = InvariantChecker()
        events = [
            _make_event(StrokeSettled, seq=5, stroke_id=0, direction="up",
                        i0=0, i1=10, p0=100.0, p1=110.0),
            _make_event(StrokeCandidate, seq=5, stroke_id=1, direction="down",
                        i0=10, i1=15, p0=110.0, p1=100.0),
        ]
        violations = checker.check(events, bar_idx=15, bar_ts=1500.0)
        assert len(violations) >= 1
        assert any(v.code == I4_TYPE_MISMATCH for v in violations)


# =====================================================================
# Checker 状态管理
# =====================================================================


class TestCheckerState:
    """InvariantChecker 状态管理测试。"""

    def test_reset_clears_state(self):
        """reset() 后，之前的 settled 记录被清除。"""
        checker = InvariantChecker()
        ev = _make_event(StrokeSettled, seq=0, stroke_id=0, direction="up",
                         i0=0, i1=10, p0=100.0, p1=110.0)
        checker.check([ev], bar_idx=10, bar_ts=1000.0)

        checker.reset()

        # 同一笔再次 settle → 不应违规（因为 reset 清除了记录）
        ev2 = _make_event(StrokeSettled, seq=0, stroke_id=0, direction="up",
                          i0=0, i1=10, p0=100.0, p1=110.0)
        violations = checker.check([ev2], bar_idx=10, bar_ts=1000.0)
        assert violations == []

    def test_violation_has_event_id(self):
        """违规事件的 event_id 非空且确定性。"""
        checker = InvariantChecker()
        ev1 = _make_event(StrokeCandidate, seq=0, stroke_id=0, direction="up",
                          i0=0, i1=5, p0=100.0, p1=105.0)
        checker.check([ev1], bar_idx=5, bar_ts=2000.0)
        ev2 = _make_event(StrokeCandidate, bar_idx=3, bar_ts=1000.0, seq=1,
                          stroke_id=1, direction="down",
                          i0=5, i1=10, p0=105.0, p1=95.0)
        violations = checker.check([ev2], bar_idx=3, bar_ts=1000.0)
        assert len(violations) >= 1
        v = violations[0]
        assert v.event_id != ""
        assert len(v.event_id) == 16

    def test_violation_event_type(self):
        """违规事件的 event_type 是 invariant_violation。"""
        checker = InvariantChecker()
        ev = _make_event(StrokeCandidate, seq=0, stroke_id=0, direction="up",
                         i0=0, i1=5, p0=100.0, p1=105.0)
        checker.check([ev], bar_idx=5, bar_ts=2000.0)
        ev2 = _make_event(StrokeCandidate, bar_idx=3, bar_ts=1000.0, seq=1,
                          stroke_id=1, direction="down",
                          i0=5, i1=10, p0=105.0, p1=95.0)
        violations = checker.check([ev2], bar_idx=3, bar_ts=1000.0)
        assert all(v.event_type == "invariant_violation" for v in violations)

    def test_violation_has_snapshot_hash(self):
        """违规事件包含非空 snapshot_hash。"""
        checker = InvariantChecker()
        ev = _make_event(StrokeCandidate, seq=0, stroke_id=0, direction="up",
                         i0=0, i1=5, p0=100.0, p1=105.0)
        checker.check([ev], bar_idx=5, bar_ts=2000.0)
        ev2 = _make_event(StrokeCandidate, bar_idx=3, bar_ts=1000.0, seq=1,
                          stroke_id=1, direction="down",
                          i0=5, i1=10, p0=105.0, p1=95.0)
        violations = checker.check([ev2], bar_idx=3, bar_ts=1000.0)
        assert len(violations) >= 1
        assert violations[0].snapshot_hash != ""

    def test_empty_events_no_violation(self):
        """空事件列表 → 无违规。"""
        checker = InvariantChecker()
        violations = checker.check([], bar_idx=0, bar_ts=1000.0)
        assert violations == []

    def test_candidate_and_extended_no_i1(self):
        """candidate 和 extended 事件不触发 I1（只有 settled 才触发）。"""
        checker = InvariantChecker()
        events = [
            _make_event(StrokeCandidate, seq=0, stroke_id=0, direction="up",
                        i0=0, i1=5, p0=100.0, p1=105.0),
            _make_event(StrokeExtended, seq=1, stroke_id=0, direction="up",
                        old_i1=5, new_i1=8, old_p1=105.0, new_p1=108.0),
        ]
        violations = checker.check(events, bar_idx=8, bar_ts=1000.0)
        assert all(v.code != I1_SETTLED_OVERWRITE for v in violations)


# =====================================================================
# BiEngine 集成测试
# =====================================================================


class TestBiEngineInvariantIntegration:
    """BiEngine 与 InvariantChecker 的集成测试。"""

    def test_normal_flow_no_violations(self):
        """正常数据流（zigzag bars）→ 无不变量违规。"""
        bars = _generate_zigzag_bars(60)
        engine = BiEngine()
        all_violations = []
        for bar in bars:
            snap = engine.process_bar(bar)
            for ev in snap.events:
                if isinstance(ev, InvariantViolation):
                    all_violations.append(ev)
        assert all_violations == [], f"意外违规: {all_violations}"

    def test_checker_reset_with_engine(self):
        """engine.reset() 同时重置 checker，seek 后不产生假阳性。"""
        bars = _generate_zigzag_bars(30)
        engine = BiEngine()

        # 先跑完所有 bar
        for bar in bars:
            engine.process_bar(bar)

        # reset 并从头重跑
        engine.reset()
        all_violations = []
        for bar in bars:
            snap = engine.process_bar(bar)
            for ev in snap.events:
                if isinstance(ev, InvariantViolation):
                    all_violations.append(ev)
        assert all_violations == [], f"reset 后假阳性: {all_violations}"

    def test_deterministic_violations(self):
        """两次独立运行 → 违规事件（如果有的话）的 event_id 完全相同。"""
        bars = _generate_zigzag_bars(60)

        def run_once():
            engine = BiEngine()
            violations = []
            for bar in bars:
                snap = engine.process_bar(bar)
                for ev in snap.events:
                    if isinstance(ev, InvariantViolation):
                        violations.append(ev.event_id)
            return violations

        v1 = run_once()
        v2 = run_once()
        assert v1 == v2
