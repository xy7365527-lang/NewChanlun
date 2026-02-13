"""走势类型不变量测试 — I18-I21 正/反例（MVP-D0）

覆盖 8 个场景：
  1. I18 正例：zs_count >= 1 无违规
  2. I18 反例：zs_count < 1 触发违规
  3. I19 正例：candidate → settle 无违规
  4. I19 反例：settle 无先行 candidate
  5. I20 正例：zs_end >= zs_start 无违规
  6. I20 反例：zs_end < zs_start 触发违规
  7. I21 正例：invalidate 后无同身份事件
  8. I21 反例：invalidate 后出现同身份 candidate
"""

from __future__ import annotations

import pytest

from newchan.audit.move_checker import MoveInvariantChecker
from newchan.events import (
    MoveCandidateV1,
    MoveInvalidateV1,
    MoveSettleV1,
)
from newchan.fingerprint import compute_event_id


# ── helpers ──

def _make_candidate(
    bar_idx: int = 10,
    bar_ts: float = 100.0,
    seq: int = 0,
    *,
    move_id: int = 0,
    kind: str = "consolidation",
    direction: str = "up",
    seg_start: int = 0,
    seg_end: int = 2,
    zs_start: int = 0,
    zs_end: int = 0,
    zs_count: int = 1,
) -> MoveCandidateV1:
    payload = {
        "move_id": move_id, "kind": kind, "direction": direction,
        "seg_start": seg_start, "seg_end": seg_end,
        "zs_start": zs_start, "zs_end": zs_end, "zs_count": zs_count,
    }
    eid = compute_event_id(bar_idx, bar_ts, "move_candidate", seq, payload)
    return MoveCandidateV1(
        bar_idx=bar_idx, bar_ts=bar_ts, seq=seq, event_id=eid,
        move_id=move_id, kind=kind, direction=direction,
        seg_start=seg_start, seg_end=seg_end,
        zs_start=zs_start, zs_end=zs_end, zs_count=zs_count,
    )


def _make_settle(
    bar_idx: int = 10,
    bar_ts: float = 100.0,
    seq: int = 1,
    *,
    move_id: int = 0,
    kind: str = "consolidation",
    direction: str = "up",
    seg_start: int = 0,
    seg_end: int = 2,
    zs_start: int = 0,
    zs_end: int = 0,
    zs_count: int = 1,
) -> MoveSettleV1:
    payload = {
        "move_id": move_id, "kind": kind, "direction": direction,
        "seg_start": seg_start, "seg_end": seg_end,
        "zs_start": zs_start, "zs_end": zs_end, "zs_count": zs_count,
    }
    eid = compute_event_id(bar_idx, bar_ts, "move_settle", seq, payload)
    return MoveSettleV1(
        bar_idx=bar_idx, bar_ts=bar_ts, seq=seq, event_id=eid,
        move_id=move_id, kind=kind, direction=direction,
        seg_start=seg_start, seg_end=seg_end,
        zs_start=zs_start, zs_end=zs_end, zs_count=zs_count,
    )


def _make_invalidate(
    bar_idx: int = 10,
    bar_ts: float = 100.0,
    seq: int = 0,
    *,
    move_id: int = 0,
    kind: str = "consolidation",
    direction: str = "up",
    seg_start: int = 0,
    seg_end: int = 2,
) -> MoveInvalidateV1:
    payload = {
        "move_id": move_id, "kind": kind, "direction": direction,
        "seg_start": seg_start, "seg_end": seg_end,
    }
    eid = compute_event_id(bar_idx, bar_ts, "move_invalidate", seq, payload)
    return MoveInvalidateV1(
        bar_idx=bar_idx, bar_ts=bar_ts, seq=seq, event_id=eid,
        move_id=move_id, kind=kind, direction=direction,
        seg_start=seg_start, seg_end=seg_end,
    )


# =====================================================================
# 1) I18 正例：zs_count >= 1 无违规
# =====================================================================

class TestI18Positive:
    def test_valid_zs_count(self):
        checker = MoveInvariantChecker()
        ev = _make_candidate(zs_count=1)
        violations = checker.check([ev], 10, 100.0)
        i18 = [v for v in violations if "I18" in v.code]
        assert len(i18) == 0


# =====================================================================
# 2) I18 反例：zs_count < 1 触发违规
# =====================================================================

class TestI18Negative:
    def test_invalid_zs_count(self):
        checker = MoveInvariantChecker()
        ev = _make_candidate(zs_count=0)
        violations = checker.check([ev], 10, 100.0)
        i18 = [v for v in violations if "I18" in v.code]
        assert len(i18) == 1


# =====================================================================
# 3) I19 正例：candidate → settle 无违规
# =====================================================================

class TestI19Positive:
    def test_candidate_then_settle(self):
        checker = MoveInvariantChecker()
        cand = _make_candidate(seq=0, move_id=0)
        settle = _make_settle(seq=1, move_id=0)
        violations = checker.check([cand, settle], 10, 100.0)
        i19 = [v for v in violations if "I19" in v.code]
        assert len(i19) == 0

    def test_candidate_prior_bar_then_settle(self):
        """candidate 在前一 bar，settle 在后一 bar → 无违规。"""
        checker = MoveInvariantChecker()
        cand = _make_candidate(bar_idx=10, bar_ts=100.0, seq=0, move_id=0)
        v1 = checker.check([cand], 10, 100.0)
        assert len(v1) == 0

        settle = _make_settle(bar_idx=11, bar_ts=101.0, seq=1, move_id=0)
        v2 = checker.check([settle], 11, 101.0)
        i19 = [v for v in v2 if "I19" in v.code]
        assert len(i19) == 0


# =====================================================================
# 4) I19 反例：settle 无先行 candidate
# =====================================================================

class TestI19Negative:
    def test_settle_without_candidate(self):
        checker = MoveInvariantChecker()
        settle = _make_settle(move_id=99)
        violations = checker.check([settle], 10, 100.0)
        i19 = [v for v in violations if "I19" in v.code]
        assert len(i19) == 1


# =====================================================================
# 5) I20 正例：zs_end >= zs_start 无违规
# =====================================================================

class TestI20Positive:
    def test_valid_parents(self):
        checker = MoveInvariantChecker()
        ev = _make_candidate(zs_start=0, zs_end=1, zs_count=2)
        violations = checker.check([ev], 10, 100.0)
        i20 = [v for v in violations if "I20" in v.code]
        assert len(i20) == 0


# =====================================================================
# 6) I20 反例：zs_end < zs_start 触发违规
# =====================================================================

class TestI20Negative:
    def test_invalid_parents(self):
        checker = MoveInvariantChecker()
        ev = _make_candidate(zs_start=5, zs_end=3, zs_count=1)
        violations = checker.check([ev], 10, 100.0)
        i20 = [v for v in violations if "I20" in v.code]
        assert len(i20) == 1


# =====================================================================
# 7) I21 正例：invalidate 后无同身份事件
# =====================================================================

class TestI21Positive:
    def test_invalidate_then_different_identity(self):
        """invalidate 后出现不同身份 candidate → 无违规。"""
        checker = MoveInvariantChecker()
        inv = _make_invalidate(bar_idx=10, bar_ts=100.0, seg_start=0)
        v1 = checker.check([inv], 10, 100.0)
        assert len(v1) == 0

        # 不同身份 candidate
        cand = _make_candidate(bar_idx=11, bar_ts=101.0, seg_start=6)
        v2 = checker.check([cand], 11, 101.0)
        i21 = [v for v in v2 if "I21" in v.code]
        assert len(i21) == 0


# =====================================================================
# 8) I21 反例：invalidate 后出现同身份 candidate
# =====================================================================

class TestI21Negative:
    def test_invalidate_then_same_identity_candidate(self):
        """invalidate 后出现同身份 candidate → I21 违规。"""
        checker = MoveInvariantChecker()
        inv = _make_invalidate(bar_idx=10, bar_ts=100.0, seg_start=0)
        v1 = checker.check([inv], 10, 100.0)
        assert len(v1) == 0

        # 同身份 candidate → 违规
        cand = _make_candidate(bar_idx=11, bar_ts=101.0, seg_start=0)
        v2 = checker.check([cand], 11, 101.0)
        i21 = [v for v in v2 if "I21" in v.code]
        assert len(i21) == 1
