"""买卖点不变量测试 — I23-I27 正/反例（MVP-E0）

覆盖 10 个场景：
  1. I24 正例：candidate → confirm 无违规
  2. I24 反例：confirm 无先行 candidate
  3. I25 正例：candidate → confirm → settle 无违规
  4. I25 反例：settle 无先行 confirm
  5. I26 正例：type2+type3 共存无违规（2B+3B 重合）
  6. I26 反例：type1+type2 同 seg_idx 触发互斥
  7. I26 反例：type1+type3 同 seg_idx 触发互斥
  8. I27 正例：invalidate 后无同身份事件
  9. I27 反例：invalidate 后出现同身份 candidate
 10. I23 反例：type3 缺 center_seg_start
"""

from __future__ import annotations

from newchan.audit.bsp_checker import BspInvariantChecker
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
)
from newchan.fingerprint import compute_event_id


# ── helpers ──

def _make_candidate(
    bar_idx: int = 10,
    bar_ts: float = 100.0,
    seq: int = 0,
    *,
    bsp_id: int = 1,
    kind: str = "type1",
    side: str = "buy",
    level_id: int = 1,
    seg_idx: int = 5,
    price: float = 99.0,
    move_seg_start: int = 0,
    center_seg_start: int = 3,
) -> BuySellPointCandidateV1:
    payload = {
        "bsp_id": bsp_id, "kind": kind, "side": side,
        "level_id": level_id, "seg_idx": seg_idx,
    }
    eid = compute_event_id(bar_idx, bar_ts, "bsp_candidate", seq, payload)
    return BuySellPointCandidateV1(
        bar_idx=bar_idx, bar_ts=bar_ts, seq=seq, event_id=eid,
        bsp_id=bsp_id, kind=kind, side=side, level_id=level_id,
        seg_idx=seg_idx, price=price,
        move_seg_start=move_seg_start, center_seg_start=center_seg_start,
    )


def _make_confirm(
    bar_idx: int = 10,
    bar_ts: float = 100.0,
    seq: int = 1,
    *,
    bsp_id: int = 1,
    kind: str = "type1",
    side: str = "buy",
    level_id: int = 1,
    seg_idx: int = 5,
    price: float = 99.0,
) -> BuySellPointConfirmV1:
    payload = {
        "bsp_id": bsp_id, "kind": kind, "side": side,
        "level_id": level_id, "seg_idx": seg_idx,
    }
    eid = compute_event_id(bar_idx, bar_ts, "bsp_confirm", seq, payload)
    return BuySellPointConfirmV1(
        bar_idx=bar_idx, bar_ts=bar_ts, seq=seq, event_id=eid,
        bsp_id=bsp_id, kind=kind, side=side, level_id=level_id,
        seg_idx=seg_idx, price=price,
    )


def _make_settle(
    bar_idx: int = 10,
    bar_ts: float = 100.0,
    seq: int = 2,
    *,
    bsp_id: int = 1,
    kind: str = "type1",
    side: str = "buy",
    level_id: int = 1,
    seg_idx: int = 5,
    price: float = 99.0,
) -> BuySellPointSettleV1:
    payload = {
        "bsp_id": bsp_id, "kind": kind, "side": side,
        "level_id": level_id, "seg_idx": seg_idx,
    }
    eid = compute_event_id(bar_idx, bar_ts, "bsp_settle", seq, payload)
    return BuySellPointSettleV1(
        bar_idx=bar_idx, bar_ts=bar_ts, seq=seq, event_id=eid,
        bsp_id=bsp_id, kind=kind, side=side, level_id=level_id,
        seg_idx=seg_idx, price=price,
    )


def _make_invalidate(
    bar_idx: int = 10,
    bar_ts: float = 100.0,
    seq: int = 0,
    *,
    bsp_id: int = 1,
    kind: str = "type1",
    side: str = "buy",
    level_id: int = 1,
    seg_idx: int = 5,
) -> BuySellPointInvalidateV1:
    payload = {
        "bsp_id": bsp_id, "kind": kind, "side": side,
        "level_id": level_id, "seg_idx": seg_idx,
    }
    eid = compute_event_id(bar_idx, bar_ts, "bsp_invalidate", seq, payload)
    return BuySellPointInvalidateV1(
        bar_idx=bar_idx, bar_ts=bar_ts, seq=seq, event_id=eid,
        bsp_id=bsp_id, kind=kind, side=side, level_id=level_id,
        seg_idx=seg_idx,
    )


# ── I24: candidate before confirm ──

def test_i24_pass_candidate_then_confirm():
    """I24 正例：candidate → confirm 无违规。"""
    checker = BspInvariantChecker()
    cand = _make_candidate(seq=0)
    conf = _make_confirm(seq=1)
    violations = checker.check([cand, conf], bar_idx=10, bar_ts=100.0)
    assert not any(v.code == I24_BSP_CANDIDATE_BEFORE_CONFIRM for v in violations)


def test_i24_fail_confirm_without_candidate():
    """I24 反例：confirm 无先行 candidate → 违规。"""
    checker = BspInvariantChecker()
    conf = _make_confirm(seq=0, bsp_id=99)
    violations = checker.check([conf], bar_idx=10, bar_ts=100.0)
    assert any(v.code == I24_BSP_CANDIDATE_BEFORE_CONFIRM for v in violations)


# ── I25: confirm before settle ──

def test_i25_pass_candidate_confirm_settle():
    """I25 正例：candidate → confirm → settle 无违规。"""
    checker = BspInvariantChecker()
    events = [
        _make_candidate(seq=0),
        _make_confirm(seq=1),
        _make_settle(seq=2),
    ]
    violations = checker.check(events, bar_idx=10, bar_ts=100.0)
    assert not any(v.code == I25_BSP_CONFIRM_BEFORE_SETTLE for v in violations)


def test_i25_fail_settle_without_confirm():
    """I25 反例：settle 无先行 confirm → 违规。"""
    checker = BspInvariantChecker()
    events = [
        _make_candidate(seq=0),
        _make_settle(seq=1),  # 跳过 confirm
    ]
    violations = checker.check(events, bar_idx=10, bar_ts=100.0)
    assert any(v.code == I25_BSP_CONFIRM_BEFORE_SETTLE for v in violations)


# ── I26: mutual exclusion ──

def test_i26_pass_type2_type3_coexist():
    """I26 正例：type2+type3 同 seg_idx 共存（2B+3B 重合），无违规。"""
    checker = BspInvariantChecker()
    t2 = _make_candidate(seq=0, bsp_id=1, kind="type2", seg_idx=5)
    t3 = _make_candidate(seq=1, bsp_id=2, kind="type3", seg_idx=5)
    violations = checker.check([t2, t3], bar_idx=10, bar_ts=100.0)
    assert not any(v.code == I26_BSP_MUTUAL_EXCLUSION for v in violations)


def test_i26_fail_type1_type2_same_seg():
    """I26 反例：type1+type2 同 seg_idx → 互斥违规。"""
    checker = BspInvariantChecker()
    t1 = _make_candidate(seq=0, bsp_id=1, kind="type1", seg_idx=5)
    t2 = _make_candidate(seq=1, bsp_id=2, kind="type2", seg_idx=5)
    violations = checker.check([t1, t2], bar_idx=10, bar_ts=100.0)
    assert any(v.code == I26_BSP_MUTUAL_EXCLUSION for v in violations)


def test_i26_fail_type1_type3_same_seg():
    """I26 反例：type1+type3 同 seg_idx → 互斥违规。"""
    checker = BspInvariantChecker()
    t1 = _make_candidate(seq=0, bsp_id=1, kind="type1", seg_idx=5)
    t3 = _make_candidate(seq=1, bsp_id=2, kind="type3", seg_idx=5)
    violations = checker.check([t1, t3], bar_idx=10, bar_ts=100.0)
    assert any(v.code == I26_BSP_MUTUAL_EXCLUSION for v in violations)


# ── I27: invalidate terminal ──

def test_i27_pass_no_event_after_invalidate():
    """I27 正例：invalidate 后无同身份事件 → 无违规。"""
    checker = BspInvariantChecker()
    inv = _make_invalidate(seq=0)
    violations = checker.check([inv], bar_idx=10, bar_ts=100.0)
    assert not any(v.code == I27_BSP_INVALIDATE_TERMINAL for v in violations)


def test_i27_fail_candidate_after_invalidate():
    """I27 反例：invalidate 后出现同身份 candidate → 违规。"""
    checker = BspInvariantChecker()
    inv = _make_invalidate(seq=0, bsp_id=1, seg_idx=5)
    checker.check([inv], bar_idx=10, bar_ts=100.0)

    # 下一个 bar：同身份 candidate 出现
    cand = _make_candidate(bar_idx=11, bar_ts=101.0, seq=0, bsp_id=2, seg_idx=5)
    violations = checker.check([cand], bar_idx=11, bar_ts=101.0)
    assert any(v.code == I27_BSP_INVALIDATE_TERMINAL for v in violations)


# ── I23: type constraint ──

def test_i23_fail_type3_missing_center():
    """I23 反例：type3 的 center_seg_start=0（非零 seg_idx）→ 违规。"""
    checker = BspInvariantChecker()
    cand = _make_candidate(
        seq=0, bsp_id=1, kind="type3", seg_idx=5, center_seg_start=0,
    )
    violations = checker.check([cand], bar_idx=10, bar_ts=100.0)
    assert any(v.code == I23_BSP_TYPE_CONSTRAINT for v in violations)
