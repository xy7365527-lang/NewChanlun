"""线段不变量测试 — I6, I7, I9, I17 正/反例

覆盖 8 个场景：
  1. I6  正例：pending → settle 无违规
  2. I6  反例：直接 settle（无先行 pending_break）
  3. I7  正例：s1 == new_segment_s0 - 1 无违规
  4. I7  反例：s1 != new_segment_s0 - 1 触发违规
  5. I9  正例：一次 invalidate 无违规
  6. I9  反例：重复 invalidate 同一段（中间无 settle）
  7. I17 正例：invalidate 后无同身份后续事件
  8. I17 反例：invalidate 后出现同身份 candidate（pending）

注意：I8 (GAP_NEEDS_SEQ2) 由算法内部保证，checker 未实现检查，跳过。
"""

from typing import Literal

from newchan.audit.segment_checker import SegmentInvariantChecker
from newchan.audit.invariants import (
    I6_PENDING_DIRECT_SETTLE,
    I7_SETTLE_ANCHOR,
    I9_INVALIDATE_IDEMPOTENT,
    I17_INVALIDATE_IS_TERMINAL,
)
from newchan.events import (
    SegmentBreakPendingV1,
    SegmentInvalidateV1,
    SegmentSettleV1,
)
from newchan.fingerprint import compute_event_id


# ── helpers ──


def _make_pending(
    bar_idx: int = 10,
    bar_ts: float = 100.0,
    seq: int = 0,
    *,
    segment_id: int = 1,
    direction: Literal["up", "down"] = "up",
    break_at_stroke: int = 4,
    s0: int = 0,
    s1: int = 3,
) -> SegmentBreakPendingV1:
    """构造 SegmentBreakPendingV1 事件。"""
    payload = {
        "segment_id": segment_id,
        "direction": direction,
        "break_at_stroke": break_at_stroke,
        "s0": s0,
        "s1": s1,
    }
    eid = compute_event_id(bar_idx, bar_ts, "segment_break_pending", seq, payload)
    return SegmentBreakPendingV1(
        bar_idx=bar_idx,
        bar_ts=bar_ts,
        seq=seq,
        event_id=eid,
        segment_id=segment_id,
        direction=direction,
        break_at_stroke=break_at_stroke,
        s0=s0,
        s1=s1,
    )


def _make_settle(
    bar_idx: int = 10,
    bar_ts: float = 100.0,
    seq: int = 1,
    *,
    segment_id: int = 1,
    direction: Literal["up", "down"] = "up",
    s0: int = 0,
    s1: int = 3,
    new_segment_s0: int = 4,
    new_segment_direction: Literal["up", "down"] = "down",
) -> SegmentSettleV1:
    """构造 SegmentSettleV1 事件。"""
    payload = {
        "segment_id": segment_id,
        "direction": direction,
        "s0": s0,
        "s1": s1,
        "new_segment_s0": new_segment_s0,
    }
    eid = compute_event_id(bar_idx, bar_ts, "segment_settle", seq, payload)
    return SegmentSettleV1(
        bar_idx=bar_idx,
        bar_ts=bar_ts,
        seq=seq,
        event_id=eid,
        segment_id=segment_id,
        direction=direction,
        s0=s0,
        s1=s1,
        new_segment_s0=new_segment_s0,
        new_segment_direction=new_segment_direction,
    )


def _make_invalidate(
    bar_idx: int = 10,
    bar_ts: float = 100.0,
    seq: int = 0,
    *,
    segment_id: int = 1,
    direction: Literal["up", "down"] = "up",
    s0: int = 0,
    s1: int = 3,
) -> SegmentInvalidateV1:
    """构造 SegmentInvalidateV1 事件。"""
    payload = {
        "segment_id": segment_id,
        "direction": direction,
        "s0": s0,
        "s1": s1,
    }
    eid = compute_event_id(bar_idx, bar_ts, "segment_invalidate", seq, payload)
    return SegmentInvalidateV1(
        bar_idx=bar_idx,
        bar_ts=bar_ts,
        seq=seq,
        event_id=eid,
        segment_id=segment_id,
        direction=direction,
        s0=s0,
        s1=s1,
    )


# =====================================================================
# 1) I6 正例：pending → settle 无违规
# =====================================================================


def test_i6_pass_pending_then_settle():
    """I6 正例：先发 pending_break 再 settle，无违规。"""
    checker = SegmentInvariantChecker()
    pending = _make_pending(seq=0, segment_id=1)
    settle = _make_settle(seq=1, segment_id=1, s0=0, s1=3, new_segment_s0=4)
    violations = checker.check([pending, settle], bar_idx=10, bar_ts=100.0)
    assert not any(v.code == I6_PENDING_DIRECT_SETTLE for v in violations)


def test_i6_pass_pending_prior_bar_then_settle():
    """I6 正例：pending 在前一 bar，settle 在后一 bar → 无违规。"""
    checker = SegmentInvariantChecker()
    pending = _make_pending(bar_idx=10, bar_ts=100.0, seq=0, segment_id=1)
    v1 = checker.check([pending], bar_idx=10, bar_ts=100.0)
    assert not any(v.code == I6_PENDING_DIRECT_SETTLE for v in v1)

    settle = _make_settle(
        bar_idx=11, bar_ts=101.0, seq=1,
        segment_id=1, s0=0, s1=3, new_segment_s0=4,
    )
    v2 = checker.check([settle], bar_idx=11, bar_ts=101.0)
    assert not any(v.code == I6_PENDING_DIRECT_SETTLE for v in v2)


# =====================================================================
# 2) I6 反例：直接 settle（无先行 pending_break）
# =====================================================================


def test_i6_fail_settle_without_pending():
    """I6 反例：直接 settle 而无 pending_break → 违规。"""
    checker = SegmentInvariantChecker()
    settle = _make_settle(
        seq=0, segment_id=99,
        s0=0, s1=3, new_segment_s0=4,
    )
    violations = checker.check([settle], bar_idx=10, bar_ts=100.0)
    assert any(v.code == I6_PENDING_DIRECT_SETTLE for v in violations)


# =====================================================================
# 3) I7 正例：s1 == new_segment_s0 - 1 无违规
# =====================================================================


def test_i7_pass_correct_anchor():
    """I7 正例：s1=3, new_segment_s0=4 → s1 == new_segment_s0-1，无违规。"""
    checker = SegmentInvariantChecker()
    # 先发 pending 避免 I6 违规
    pending = _make_pending(seq=0, segment_id=1)
    settle = _make_settle(
        seq=1, segment_id=1,
        s0=0, s1=3, new_segment_s0=4,  # 3 == 4-1 ✓
    )
    violations = checker.check([pending, settle], bar_idx=10, bar_ts=100.0)
    assert not any(v.code == I7_SETTLE_ANCHOR for v in violations)


# =====================================================================
# 4) I7 反例：s1 != new_segment_s0 - 1 触发违规
# =====================================================================


def test_i7_fail_wrong_anchor():
    """I7 反例：s1=3, new_segment_s0=6 → s1 != new_segment_s0-1，违规。"""
    checker = SegmentInvariantChecker()
    # 先发 pending 避免 I6 违规
    pending = _make_pending(seq=0, segment_id=1)
    settle = _make_settle(
        seq=1, segment_id=1,
        s0=0, s1=3, new_segment_s0=6,  # 3 != 6-1=5 ✗
    )
    violations = checker.check([pending, settle], bar_idx=10, bar_ts=100.0)
    assert any(v.code == I7_SETTLE_ANCHOR for v in violations)


# =====================================================================
# 5) I9 正例：一次 invalidate 无违规
# =====================================================================


def test_i9_pass_single_invalidate():
    """I9 正例：对一个已 settle 的段做一次 invalidate，无违规。"""
    checker = SegmentInvariantChecker()
    # 先 pending + settle
    pending = _make_pending(seq=0, segment_id=1, s0=0, s1=3)
    settle = _make_settle(
        seq=1, segment_id=1, s0=0, s1=3, new_segment_s0=4,
    )
    v1 = checker.check([pending, settle], bar_idx=10, bar_ts=100.0)
    assert not any(v.code == I9_INVALIDATE_IDEMPOTENT for v in v1)

    # 一次 invalidate
    inv = _make_invalidate(
        bar_idx=11, bar_ts=101.0, seq=2,
        segment_id=1, s0=0, s1=3, direction="up",
    )
    v2 = checker.check([inv], bar_idx=11, bar_ts=101.0)
    assert not any(v.code == I9_INVALIDATE_IDEMPOTENT for v in v2)


# =====================================================================
# 6) I9 反例：重复 invalidate 同一段（中间无 settle）
# =====================================================================


def test_i9_fail_duplicate_invalidate():
    """I9 反例：同一 (s0, s1, direction) 被 invalidate 两次，中间无 settle → 违规。"""
    checker = SegmentInvariantChecker()
    # 第一次 invalidate
    inv1 = _make_invalidate(
        bar_idx=10, bar_ts=100.0, seq=0,
        segment_id=1, s0=0, s1=3, direction="up",
    )
    v1 = checker.check([inv1], bar_idx=10, bar_ts=100.0)
    assert not any(v.code == I9_INVALIDATE_IDEMPOTENT for v in v1)

    # 第二次 invalidate 同一段（无中间 settle）→ 违规
    inv2 = _make_invalidate(
        bar_idx=11, bar_ts=101.0, seq=1,
        segment_id=2, s0=0, s1=3, direction="up",  # 不同 segment_id 但相同 key
    )
    v2 = checker.check([inv2], bar_idx=11, bar_ts=101.0)
    assert any(v.code == I9_INVALIDATE_IDEMPOTENT for v in v2)


# =====================================================================
# 7) I17 正例：invalidate 后无同身份后续事件
# =====================================================================


def test_i17_pass_no_event_after_invalidate():
    """I17 正例：invalidate 后无同身份后续事件 → 无违规。"""
    checker = SegmentInvariantChecker()
    inv = _make_invalidate(
        bar_idx=10, bar_ts=100.0, seq=0,
        segment_id=1, s0=0, s1=3, direction="up",
    )
    violations = checker.check([inv], bar_idx=10, bar_ts=100.0)
    assert not any(v.code == I17_INVALIDATE_IS_TERMINAL for v in violations)


def test_i17_pass_different_identity_after_invalidate():
    """I17 正例：invalidate 后出现不同身份事件 → 无违规。"""
    checker = SegmentInvariantChecker()
    inv = _make_invalidate(
        bar_idx=10, bar_ts=100.0, seq=0,
        segment_id=1, s0=0, s1=3, direction="up",
    )
    checker.check([inv], bar_idx=10, bar_ts=100.0)

    # 不同身份 (s0=6, direction="down") 的 pending → 无违规
    pending = _make_pending(
        bar_idx=11, bar_ts=101.0, seq=1,
        segment_id=2, s0=6, s1=9, direction="down",
    )
    v2 = checker.check([pending], bar_idx=11, bar_ts=101.0)
    assert not any(v.code == I17_INVALIDATE_IS_TERMINAL for v in v2)


# =====================================================================
# 8) I17 反例：invalidate 后出现同身份 candidate（pending）
# =====================================================================


def test_i17_fail_pending_after_invalidate():
    """I17 反例：invalidate 后出现同身份 pending → 违规。"""
    checker = SegmentInvariantChecker()
    inv = _make_invalidate(
        bar_idx=10, bar_ts=100.0, seq=0,
        segment_id=1, s0=0, s1=3, direction="up",
    )
    checker.check([inv], bar_idx=10, bar_ts=100.0)

    # 同身份 (s0=0, direction="up") 的 pending → 违规
    pending = _make_pending(
        bar_idx=11, bar_ts=101.0, seq=1,
        segment_id=2, s0=0, s1=5, direction="up",
    )
    v2 = checker.check([pending], bar_idx=11, bar_ts=101.0)
    assert any(v.code == I17_INVALIDATE_IS_TERMINAL for v in v2)


def test_i17_fail_settle_after_invalidate():
    """I17 反例（补充）：invalidate 后出现同身份 settle → 违规。"""
    checker = SegmentInvariantChecker()
    inv = _make_invalidate(
        bar_idx=10, bar_ts=100.0, seq=0,
        segment_id=1, s0=0, s1=3, direction="up",
    )
    checker.check([inv], bar_idx=10, bar_ts=100.0)

    # 同身份 (s0=0, direction="up") 的 settle → 违规
    settle = _make_settle(
        bar_idx=11, bar_ts=101.0, seq=1,
        segment_id=2, s0=0, s1=5, direction="up",
        new_segment_s0=6,
    )
    v2 = checker.check([settle], bar_idx=11, bar_ts=101.0)
    assert any(v.code == I17_INVALIDATE_IS_TERMINAL for v in v2)
