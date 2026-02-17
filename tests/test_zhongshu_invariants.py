"""中枢不变量测试 — I11-I14, I17 正/反例（MVP-C0）

覆盖 10 个场景：
  1.  I11 正例：ZG > ZD（candidate）→ 无违规
  2.  I11 反例：ZG <= ZD（candidate）→ 触发违规
  3.  I11 反例：ZG <= ZD（settle）→ 触发违规
  4.  I12 正例：candidate → settle 无违规
  5.  I12 反例：直接 settle 无先行 candidate → 违规
  6.  I13 正例：seg_start < seg_end 且 seg_count >= 3 → 无违规
  7.  I13 反例：seg_end < seg_start → 违规
  8.  I13 反例：seg_count < 3 → 违规
  9.  I14 正例：单次 invalidate → 无违规
  10. I14 反例：重复 invalidate 同一中枢 → 违规
  11. I17 正例：invalidate 后无同身份事件 → 无违规
  12. I17 反例：invalidate 后出现同身份 candidate → 违规
  13. I17 反例：invalidate 后出现同身份 settle → 违规
"""

from typing import Literal

from newchan.audit.invariants import (
    I11_ZHONGSHU_OVERLAP,
    I12_CANDIDATE_BEFORE_SETTLE,
    I13_PARENTS_TRACEABLE,
    I14_ZHONGSHU_INVALIDATE_IDEMPOTENT,
    I17_INVALIDATE_IS_TERMINAL,
)
from newchan.audit.zhongshu_checker import ZhongshuInvariantChecker
from newchan.events import (
    ZhongshuCandidateV1,
    ZhongshuInvalidateV1,
    ZhongshuSettleV1,
)
from newchan.fingerprint import compute_event_id


# ── helpers ──


def _make_candidate(
    bar_idx: int = 10,
    bar_ts: float = 100.0,
    seq: int = 0,
    *,
    zhongshu_id: int = 1,
    zd: float = 90.0,
    zg: float = 110.0,
    seg_start: int = 0,
    seg_end: int = 2,
    seg_count: int = 3,
    level_id: int = 1,
) -> ZhongshuCandidateV1:
    """构造 ZhongshuCandidateV1 事件，使用 compute_event_id 生成确定性 event_id。"""
    payload = {
        "zhongshu_id": zhongshu_id,
        "zd": zd,
        "zg": zg,
        "seg_start": seg_start,
        "seg_end": seg_end,
        "seg_count": seg_count,
    }
    eid = compute_event_id(bar_idx, bar_ts, "zhongshu_candidate", seq, payload)
    return ZhongshuCandidateV1(
        bar_idx=bar_idx,
        bar_ts=bar_ts,
        seq=seq,
        event_id=eid,
        zhongshu_id=zhongshu_id,
        zd=zd,
        zg=zg,
        seg_start=seg_start,
        seg_end=seg_end,
        seg_count=seg_count,
        level_id=level_id,
    )


def _make_settle(
    bar_idx: int = 10,
    bar_ts: float = 100.0,
    seq: int = 1,
    *,
    zhongshu_id: int = 1,
    zd: float = 90.0,
    zg: float = 110.0,
    seg_start: int = 0,
    seg_end: int = 2,
    seg_count: int = 3,
    break_seg_id: int = 3,
    break_direction: Literal["up", "down"] = "up",
    level_id: int = 1,
) -> ZhongshuSettleV1:
    """构造 ZhongshuSettleV1 事件。"""
    payload = {
        "zhongshu_id": zhongshu_id,
        "zd": zd,
        "zg": zg,
        "seg_start": seg_start,
        "seg_end": seg_end,
        "seg_count": seg_count,
        "break_seg_id": break_seg_id,
    }
    eid = compute_event_id(bar_idx, bar_ts, "zhongshu_settle", seq, payload)
    return ZhongshuSettleV1(
        bar_idx=bar_idx,
        bar_ts=bar_ts,
        seq=seq,
        event_id=eid,
        zhongshu_id=zhongshu_id,
        zd=zd,
        zg=zg,
        seg_start=seg_start,
        seg_end=seg_end,
        seg_count=seg_count,
        break_seg_id=break_seg_id,
        break_direction=break_direction,
        level_id=level_id,
    )


def _make_invalidate(
    bar_idx: int = 10,
    bar_ts: float = 100.0,
    seq: int = 0,
    *,
    zhongshu_id: int = 1,
    zd: float = 90.0,
    zg: float = 110.0,
    seg_start: int = 0,
    seg_end: int = 2,
    level_id: int = 1,
) -> ZhongshuInvalidateV1:
    """构造 ZhongshuInvalidateV1 事件。"""
    payload = {
        "zhongshu_id": zhongshu_id,
        "zd": zd,
        "zg": zg,
        "seg_start": seg_start,
        "seg_end": seg_end,
    }
    eid = compute_event_id(bar_idx, bar_ts, "zhongshu_invalidate", seq, payload)
    return ZhongshuInvalidateV1(
        bar_idx=bar_idx,
        bar_ts=bar_ts,
        seq=seq,
        event_id=eid,
        zhongshu_id=zhongshu_id,
        zd=zd,
        zg=zg,
        seg_start=seg_start,
        seg_end=seg_end,
        level_id=level_id,
    )


# =====================================================================
# I11: ZHONGSHU_OVERLAP — 中枢 ZG > ZD
# =====================================================================


def test_i11_pass_candidate_zg_gt_zd():
    """I11 正例：candidate 的 ZG > ZD → 无违规。"""
    checker = ZhongshuInvariantChecker()
    ev = _make_candidate(zd=90.0, zg=110.0)
    violations = checker.check([ev], bar_idx=10, bar_ts=100.0)
    assert not any(v.code == I11_ZHONGSHU_OVERLAP for v in violations)


def test_i11_fail_candidate_zg_le_zd():
    """I11 反例：candidate 的 ZG <= ZD → 违规。"""
    checker = ZhongshuInvariantChecker()
    # ZG == ZD
    ev_eq = _make_candidate(zd=100.0, zg=100.0)
    violations = checker.check([ev_eq], bar_idx=10, bar_ts=100.0)
    assert any(v.code == I11_ZHONGSHU_OVERLAP for v in violations)

    # ZG < ZD
    checker2 = ZhongshuInvariantChecker()
    ev_lt = _make_candidate(zd=110.0, zg=90.0)
    violations2 = checker2.check([ev_lt], bar_idx=10, bar_ts=100.0)
    assert any(v.code == I11_ZHONGSHU_OVERLAP for v in violations2)


def test_i11_fail_settle_zg_le_zd():
    """I11 反例：settle 的 ZG <= ZD → 违规（settle 同样检查 I11）。"""
    checker = ZhongshuInvariantChecker()
    # 先给一个合法 candidate 以避免 I12 干扰
    cand = _make_candidate(seq=0, zhongshu_id=1, zd=90.0, zg=110.0)
    checker.check([cand], bar_idx=10, bar_ts=100.0)

    # settle 时 ZG <= ZD
    settle = _make_settle(
        bar_idx=11, bar_ts=101.0, seq=0,
        zhongshu_id=1, zd=100.0, zg=100.0,
    )
    violations = checker.check([settle], bar_idx=11, bar_ts=101.0)
    assert any(v.code == I11_ZHONGSHU_OVERLAP for v in violations)


# =====================================================================
# I12: CANDIDATE_BEFORE_SETTLE — settle 前须有 candidate
# =====================================================================


def test_i12_pass_candidate_then_settle():
    """I12 正例：candidate → settle（同批次）→ 无违规。"""
    checker = ZhongshuInvariantChecker()
    cand = _make_candidate(seq=0, zhongshu_id=1)
    settle = _make_settle(seq=1, zhongshu_id=1)
    violations = checker.check([cand, settle], bar_idx=10, bar_ts=100.0)
    assert not any(v.code == I12_CANDIDATE_BEFORE_SETTLE for v in violations)


def test_i12_pass_candidate_prior_bar_then_settle():
    """I12 正例：candidate 在前一 bar，settle 在后一 bar → 无违规。"""
    checker = ZhongshuInvariantChecker()
    cand = _make_candidate(bar_idx=10, bar_ts=100.0, seq=0, zhongshu_id=1)
    checker.check([cand], bar_idx=10, bar_ts=100.0)

    settle = _make_settle(
        bar_idx=11, bar_ts=101.0, seq=0, zhongshu_id=1,
    )
    violations = checker.check([settle], bar_idx=11, bar_ts=101.0)
    assert not any(v.code == I12_CANDIDATE_BEFORE_SETTLE for v in violations)


def test_i12_fail_settle_without_candidate():
    """I12 反例：直接 settle 无先行 candidate → 违规。"""
    checker = ZhongshuInvariantChecker()
    settle = _make_settle(seq=0, zhongshu_id=99)
    violations = checker.check([settle], bar_idx=10, bar_ts=100.0)
    assert any(v.code == I12_CANDIDATE_BEFORE_SETTLE for v in violations)


# =====================================================================
# I13: PARENTS_TRACEABLE — seg_start < seg_end 且 seg_count >= 3
# =====================================================================


def test_i13_pass_valid_parents():
    """I13 正例：seg_start=0, seg_end=2, seg_count=3 → 无违规。"""
    checker = ZhongshuInvariantChecker()
    ev = _make_candidate(seg_start=0, seg_end=2, seg_count=3)
    violations = checker.check([ev], bar_idx=10, bar_ts=100.0)
    assert not any(v.code == I13_PARENTS_TRACEABLE for v in violations)


def test_i13_fail_seg_end_lt_seg_start():
    """I13 反例：seg_end < seg_start → 违规。"""
    checker = ZhongshuInvariantChecker()
    ev = _make_candidate(seg_start=5, seg_end=3, seg_count=3)
    violations = checker.check([ev], bar_idx=10, bar_ts=100.0)
    assert any(v.code == I13_PARENTS_TRACEABLE for v in violations)


def test_i13_fail_seg_count_lt_3():
    """I13 反例：seg_count < 3 → 违规。"""
    checker = ZhongshuInvariantChecker()
    ev = _make_candidate(seg_start=0, seg_end=1, seg_count=2)
    violations = checker.check([ev], bar_idx=10, bar_ts=100.0)
    assert any(v.code == I13_PARENTS_TRACEABLE for v in violations)


# =====================================================================
# I14: ZHONGSHU_INVALIDATE_IDEMPOTENT — invalidate 幂等
# =====================================================================


def test_i14_pass_single_invalidate():
    """I14 正例：单次 invalidate → 无违规。"""
    checker = ZhongshuInvariantChecker()
    inv = _make_invalidate(seq=0, zhongshu_id=1, zd=90.0, zg=110.0)
    violations = checker.check([inv], bar_idx=10, bar_ts=100.0)
    assert not any(v.code == I14_ZHONGSHU_INVALIDATE_IDEMPOTENT for v in violations)


def test_i14_fail_duplicate_invalidate():
    """I14 反例：重复 invalidate 同一中枢（相同 key）→ 违规。"""
    checker = ZhongshuInvariantChecker()
    inv1 = _make_invalidate(
        bar_idx=10, bar_ts=100.0, seq=0,
        zhongshu_id=1, zd=90.0, zg=110.0, seg_start=0, seg_end=2,
    )
    checker.check([inv1], bar_idx=10, bar_ts=100.0)

    # 第二次 invalidate 同一 key
    inv2 = _make_invalidate(
        bar_idx=11, bar_ts=101.0, seq=0,
        zhongshu_id=2, zd=90.0, zg=110.0, seg_start=0, seg_end=2,
    )
    violations = checker.check([inv2], bar_idx=11, bar_ts=101.0)
    assert any(v.code == I14_ZHONGSHU_INVALIDATE_IDEMPOTENT for v in violations)


# =====================================================================
# I17: INVALIDATE_IS_TERMINAL — invalidate 终态
# =====================================================================


def test_i17_pass_invalidate_then_different_identity():
    """I17 正例：invalidate 后出现不同身份 candidate → 无违规。"""
    checker = ZhongshuInvariantChecker()
    inv = _make_invalidate(
        bar_idx=10, bar_ts=100.0, seq=0,
        zd=90.0, zg=110.0, seg_start=0,
    )
    checker.check([inv], bar_idx=10, bar_ts=100.0)

    # 不同身份（seg_start 不同）
    cand = _make_candidate(
        bar_idx=11, bar_ts=101.0, seq=0,
        zhongshu_id=2, zd=90.0, zg=110.0, seg_start=5,
    )
    violations = checker.check([cand], bar_idx=11, bar_ts=101.0)
    assert not any(v.code == I17_INVALIDATE_IS_TERMINAL for v in violations)


def test_i17_fail_candidate_after_invalidate():
    """I17 反例：invalidate 后出现同身份 candidate → 违规。"""
    checker = ZhongshuInvariantChecker()
    inv = _make_invalidate(
        bar_idx=10, bar_ts=100.0, seq=0,
        zd=90.0, zg=110.0, seg_start=0,
    )
    checker.check([inv], bar_idx=10, bar_ts=100.0)

    # 同身份 candidate (zd, zg, seg_start) 相同
    cand = _make_candidate(
        bar_idx=11, bar_ts=101.0, seq=0,
        zhongshu_id=2, zd=90.0, zg=110.0, seg_start=0,
    )
    violations = checker.check([cand], bar_idx=11, bar_ts=101.0)
    assert any(v.code == I17_INVALIDATE_IS_TERMINAL for v in violations)


def test_i17_fail_settle_after_invalidate():
    """I17 反例：invalidate 后出现同身份 settle → 违规。"""
    checker = ZhongshuInvariantChecker()
    inv = _make_invalidate(
        bar_idx=10, bar_ts=100.0, seq=0,
        zd=90.0, zg=110.0, seg_start=0,
    )
    checker.check([inv], bar_idx=10, bar_ts=100.0)

    # 同身份 settle (zd, zg, seg_start) 相同
    settle = _make_settle(
        bar_idx=11, bar_ts=101.0, seq=0,
        zhongshu_id=2, zd=90.0, zg=110.0, seg_start=0,
    )
    violations = checker.check([settle], bar_idx=11, bar_ts=101.0)
    assert any(v.code == I17_INVALIDATE_IS_TERMINAL for v in violations)
