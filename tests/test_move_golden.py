"""走势类型 golden 测试 — MVP-D0

覆盖 15 个场景：
  1. 单中枢 → 盘整
  2. 两个上涨中枢 → 上涨趋势
  3. 两个下跌中枢 → 下跌趋势
  4. 两个重叠中枢 → 两个独立盘整
  5. 三中枢混合：C1↑C2, C2↓C3 → 趋势 + 盘整
  6. 盘整→趋势升级（同身份 Candidate 更新）
  7. move settle（新 move 出现确认前一个）
  8. 无 settled 中枢 → 空
  9. 盘整方向 = break_direction
  10. high/low 来自中枢区间
  11. diff([], [m]) → Candidate
  12. diff([m_unsettled], [m_settled]) → Settle
  13. diff 同身份更新无 Invalidate
  14. diff 不同身份 → Invalidate
  15. diff 确定性
"""

from __future__ import annotations

import pytest

from newchan.a_move_v1 import Move, moves_from_zhongshus
from newchan.a_zhongshu_v1 import Zhongshu
from newchan.core.recursion.move_state import diff_moves
from newchan.events import (
    MoveCandidateV1,
    MoveInvalidateV1,
    MoveSettleV1,
)


# ── helpers ──

def _zs(
    seg_start: int,
    seg_end: int,
    zd: float,
    zg: float,
    *,
    settled: bool = True,
    break_direction: str = "up",
    break_seg: int = -1,
    first_seg_s0: int = 0,
    last_seg_s1: int = 0,
    gg: float | None = None,
    dd: float | None = None,
) -> Zhongshu:
    seg_count = seg_end - seg_start + 1
    if settled and break_seg == -1:
        break_seg = seg_end + 1
    return Zhongshu(
        zd=zd,
        zg=zg,
        seg_start=seg_start,
        seg_end=seg_end,
        seg_count=seg_count,
        settled=settled,
        break_seg=break_seg,
        break_direction=break_direction if settled else "",
        first_seg_s0=first_seg_s0,
        last_seg_s1=last_seg_s1,
        gg=gg if gg is not None else zg,
        dd=dd if dd is not None else zd,
    )


# =====================================================================
# 1) 单中枢 → 盘整
# =====================================================================

class TestSingleCenterConsolidation:
    def test_one_settled_center(self):
        """1 个 settled 中枢 → 1 个盘整，settled=False。"""
        zhongshus = [_zs(0, 2, 10.0, 18.0, break_direction="up")]
        moves = moves_from_zhongshus(zhongshus)
        assert len(moves) == 1
        m = moves[0]
        assert m.kind == "consolidation"
        assert m.direction == "up"
        assert m.zs_count == 1
        assert m.settled is False


# =====================================================================
# 2) 两个上涨中枢 → 上涨趋势
# =====================================================================

class TestTwoAscendingCentersTrend:
    def test_ascending_trend(self):
        """C2.zd > C1.zg → 上涨趋势。"""
        zhongshus = [
            _zs(0, 2, 10.0, 18.0, break_direction="up"),    # [10, 18]
            _zs(6, 8, 20.0, 28.0, break_direction="up"),    # [20, 28] — zd=20 > zg=18
        ]
        moves = moves_from_zhongshus(zhongshus)
        assert len(moves) == 1
        m = moves[0]
        assert m.kind == "trend"
        assert m.direction == "up"
        assert m.zs_count == 2
        assert m.settled is False


# =====================================================================
# 3) 两个下跌中枢 → 下跌趋势
# =====================================================================

class TestTwoDescendingCentersTrend:
    def test_descending_trend(self):
        """C2.zg < C1.zd → 下跌趋势。"""
        zhongshus = [
            _zs(0, 2, 20.0, 30.0, break_direction="down"),  # [20, 30]
            _zs(6, 8, 5.0, 18.0, break_direction="down"),   # [5, 18] — zg=18 < zd=20
        ]
        moves = moves_from_zhongshus(zhongshus)
        assert len(moves) == 1
        m = moves[0]
        assert m.kind == "trend"
        assert m.direction == "down"
        assert m.zs_count == 2


# =====================================================================
# 4) 两个重叠中枢 → 两个独立盘整
# =====================================================================

class TestTwoOverlappingCentersSeparate:
    def test_overlapping_centers(self):
        """C1=[10,18], C2=[15,25] — 区间重叠 → 2 个盘整。"""
        zhongshus = [
            _zs(0, 2, 10.0, 18.0, break_direction="up"),
            _zs(6, 8, 15.0, 25.0, break_direction="up"),    # zd=15 < zg=18 → NOT ascending
        ]
        moves = moves_from_zhongshus(zhongshus)
        assert len(moves) == 2
        assert moves[0].kind == "consolidation"
        assert moves[0].settled is True
        assert moves[1].kind == "consolidation"
        assert moves[1].settled is False


# =====================================================================
# 5) 三中枢混合：C1↑C2, C2↓C3
# =====================================================================

class TestThreeCentersMixed:
    def test_mixed_directions(self):
        """C1→C2 ascending, C2→C3 descending → 2 个 move。"""
        zhongshus = [
            _zs(0, 2, 10.0, 18.0, break_direction="up"),    # [10, 18]
            _zs(6, 8, 20.0, 28.0, break_direction="down"),  # [20, 28] — ascending from C1
            _zs(12, 14, 5.0, 18.0, break_direction="down"), # [5, 18] — descending from C2
        ]
        moves = moves_from_zhongshus(zhongshus)
        assert len(moves) == 2
        assert moves[0].kind == "trend"
        assert moves[0].direction == "up"
        assert moves[0].zs_count == 2
        assert moves[0].settled is True
        assert moves[1].kind == "consolidation"
        assert moves[1].settled is False


# =====================================================================
# 6) 盘整→趋势升级（同身份 Candidate 更新）
# =====================================================================

class TestConsolidationToTrendUpgrade:
    def test_upgrade_same_identity(self):
        """1 中枢 → 2 同向中枢 = 同身份 Candidate 更新（kind 变化）。"""
        # Phase 1: 1 中枢
        zs_phase1 = [_zs(0, 2, 10.0, 18.0, break_direction="up")]
        m1 = moves_from_zhongshus(zs_phase1)
        assert m1[0].kind == "consolidation"

        # Phase 2: 2 上涨中枢
        zs_phase2 = [
            _zs(0, 2, 10.0, 18.0, break_direction="up"),
            _zs(6, 8, 20.0, 28.0, break_direction="up"),
        ]
        m2 = moves_from_zhongshus(zs_phase2)
        assert m2[0].kind == "trend"

        # 同身份（seg_start=0）→ diff 产生 Candidate 而非 Invalidate
        events = diff_moves(m1, m2, bar_idx=10, bar_ts=100.0)
        invalidates = [e for e in events if isinstance(e, MoveInvalidateV1)]
        candidates = [e for e in events if isinstance(e, MoveCandidateV1)]
        assert len(invalidates) == 0
        assert len(candidates) == 1
        assert candidates[0].kind == "trend"


# =====================================================================
# 7) move settle（新 move 出现确认前一个）
# =====================================================================

class TestMoveSettleWhenNextAppears:
    def test_settle_on_next_move(self):
        """新 move 出现 → 前一个 move settled。"""
        # Phase 1: 1 个 unsettled 盘整
        zs1 = [_zs(0, 2, 10.0, 18.0, break_direction="up")]
        m1 = moves_from_zhongshus(zs1)
        assert m1[0].settled is False

        # Phase 2: 2 个盘整（重叠中枢）
        zs2 = [
            _zs(0, 2, 10.0, 18.0, break_direction="up"),
            _zs(6, 8, 15.0, 25.0, break_direction="up"),
        ]
        m2 = moves_from_zhongshus(zs2)
        assert m2[0].settled is True
        assert m2[1].settled is False

        # diff 产生 Settle（前一个从 unsettled → settled）
        events = diff_moves(m1, m2, bar_idx=11, bar_ts=101.0)
        settles = [e for e in events if isinstance(e, MoveSettleV1)]
        candidates = [e for e in events if isinstance(e, MoveCandidateV1)]
        assert len(settles) == 1
        assert settles[0].seg_start == 0
        assert len(candidates) == 1  # 新 move


# =====================================================================
# 8) 无 settled 中枢 → 空
# =====================================================================

class TestNoSettledCentersEmpty:
    def test_all_unsettled(self):
        """全是 candidate 中枢 → 空。"""
        zhongshus = [_zs(0, 2, 10.0, 18.0, settled=False)]
        moves = moves_from_zhongshus(zhongshus)
        assert len(moves) == 0


# =====================================================================
# 9) 盘整方向 = break_direction
# =====================================================================

class TestDirectionFromBreak:
    def test_consolidation_direction(self):
        """盘整方向 = 中枢的 break_direction。"""
        zs_up = [_zs(0, 2, 10.0, 18.0, break_direction="up")]
        zs_down = [_zs(0, 2, 10.0, 18.0, break_direction="down")]

        m_up = moves_from_zhongshus(zs_up)
        m_down = moves_from_zhongshus(zs_down)

        assert m_up[0].direction == "up"
        assert m_down[0].direction == "down"


# =====================================================================
# 10) high/low 来自中枢区间
# =====================================================================

class TestHighLowFromCenters:
    def test_trend_high_low(self):
        """趋势 high/low = max(zg)/min(zd)。"""
        zhongshus = [
            _zs(0, 2, 10.0, 18.0, break_direction="up"),    # zd=10, zg=18
            _zs(6, 8, 20.0, 30.0, break_direction="up"),    # zd=20, zg=30
        ]
        moves = moves_from_zhongshus(zhongshus)
        assert moves[0].high == 30.0  # max(18, 30)
        assert moves[0].low == 10.0   # min(10, 20)


# =====================================================================
# 11) diff([], [m]) → Candidate
# =====================================================================

class TestDiffNewConsolidation:
    def test_first_move(self):
        """首次出现 move → Candidate。"""
        curr = [Move(
            kind="consolidation", direction="up",
            seg_start=0, seg_end=2, zs_start=0, zs_end=0,
            zs_count=1, settled=False,
        )]
        events = diff_moves([], curr, bar_idx=10, bar_ts=100.0)
        candidates = [e for e in events if isinstance(e, MoveCandidateV1)]
        assert len(candidates) == 1
        assert candidates[0].kind == "consolidation"


# =====================================================================
# 12) diff([m_unsettled], [m_settled]) → Settle
# =====================================================================

class TestDiffSettleUpgrade:
    def test_settle_upgrade(self):
        """unsettled → settled（同身份）→ Settle。"""
        prev = [Move(
            kind="consolidation", direction="up",
            seg_start=0, seg_end=2, zs_start=0, zs_end=0,
            zs_count=1, settled=False,
        )]
        curr = [Move(
            kind="consolidation", direction="up",
            seg_start=0, seg_end=2, zs_start=0, zs_end=0,
            zs_count=1, settled=True,
        )]
        events = diff_moves(prev, curr, bar_idx=11, bar_ts=101.0)
        settles = [e for e in events if isinstance(e, MoveSettleV1)]
        invalidates = [e for e in events if isinstance(e, MoveInvalidateV1)]
        assert len(settles) == 1
        assert len(invalidates) == 0


# =====================================================================
# 13) diff 同身份更新无 Invalidate
# =====================================================================

class TestDiffIdentitySkip:
    def test_extend_no_invalidate(self):
        """zs_end 变化（延伸）→ Candidate 但无 Invalidate。"""
        prev = [Move(
            kind="consolidation", direction="up",
            seg_start=0, seg_end=2, zs_start=0, zs_end=0,
            zs_count=1, settled=False,
        )]
        curr = [Move(
            kind="trend", direction="up",
            seg_start=0, seg_end=8, zs_start=0, zs_end=1,
            zs_count=2, settled=False,
        )]
        events = diff_moves(prev, curr, bar_idx=12, bar_ts=102.0)
        invalidates = [e for e in events if isinstance(e, MoveInvalidateV1)]
        candidates = [e for e in events if isinstance(e, MoveCandidateV1)]
        assert len(invalidates) == 0
        assert len(candidates) == 1
        assert candidates[0].kind == "trend"


# =====================================================================
# 14) diff 不同身份 → Invalidate
# =====================================================================

class TestDiffDifferentIdentity:
    def test_different_seg_start_invalidates(self):
        """seg_start 不同 → 身份不同 → Invalidate。"""
        prev = [Move(
            kind="consolidation", direction="up",
            seg_start=0, seg_end=2, zs_start=0, zs_end=0,
            zs_count=1, settled=False,
        )]
        curr = [Move(
            kind="consolidation", direction="down",
            seg_start=6, seg_end=8, zs_start=1, zs_end=1,
            zs_count=1, settled=False,
        )]
        events = diff_moves(prev, curr, bar_idx=13, bar_ts=103.0)
        invalidates = [e for e in events if isinstance(e, MoveInvalidateV1)]
        candidates = [e for e in events if isinstance(e, MoveCandidateV1)]
        assert len(invalidates) == 1
        assert invalidates[0].seg_start == 0
        assert len(candidates) == 1
        assert candidates[0].seg_start == 6


# =====================================================================
# 15) diff 确定性
# =====================================================================

class TestDiffDeterminism:
    def test_two_runs_same_events(self):
        """同输入两次 diff → 完全相同 event_id。"""
        prev = [Move(
            kind="consolidation", direction="up",
            seg_start=0, seg_end=2, zs_start=0, zs_end=0,
            zs_count=1, settled=False,
        )]
        curr = [Move(
            kind="trend", direction="up",
            seg_start=0, seg_end=8, zs_start=0, zs_end=1,
            zs_count=2, settled=False,
        )]
        ev1 = diff_moves(prev, curr, bar_idx=12, bar_ts=102.0)
        ev2 = diff_moves(prev, curr, bar_idx=12, bar_ts=102.0)

        assert len(ev1) == len(ev2)
        for a, b in zip(ev1, ev2):
            assert a.event_id == b.event_id
            assert a.event_type == b.event_type
            assert a.seq == b.seq


# =====================================================================
# 16) GG/DD 波动区间区分趋势（中心定理二）
# =====================================================================

class TestGGDDTrendDistinction:
    """GG/DD vs ZG/ZD 的区别：波动区间重叠时非趋势。"""

    def test_zd_above_zg_but_gg_dd_overlap(self):
        """ZD/ZG 不重叠但 GG/DD 重叠 → 非趋势（盘整）。

        C1: zd=10, zg=18, dd=5, gg=25
        C2: zd=20, zg=28, dd=15, gg=35
        ZD比较: C2.zd=20 > C1.zg=18 → 旧逻辑认为上涨
        GG/DD比较: C2.dd=15 < C1.gg=25 → 波动区间重叠 → 非趋势
        """
        zhongshus = [
            _zs(0, 2, 10.0, 18.0, break_direction="up", dd=5.0, gg=25.0),
            _zs(6, 8, 20.0, 28.0, break_direction="up", dd=15.0, gg=35.0),
        ]
        moves = moves_from_zhongshus(zhongshus)
        # 波动区间重叠 → 两个独立盘整，不是趋势
        assert len(moves) == 2
        assert moves[0].kind == "consolidation"
        assert moves[1].kind == "consolidation"

    def test_gg_dd_no_overlap_ascending(self):
        """GG/DD 完全不重叠 → 真正的上涨趋势。

        C1: zd=10, zg=18, dd=5, gg=22
        C2: zd=25, zg=33, dd=23, gg=38
        C2.dd=23 > C1.gg=22 → 波动区间不重叠 → 上涨
        """
        zhongshus = [
            _zs(0, 2, 10.0, 18.0, break_direction="up", dd=5.0, gg=22.0),
            _zs(6, 8, 25.0, 33.0, break_direction="up", dd=23.0, gg=38.0),
        ]
        moves = moves_from_zhongshus(zhongshus)
        assert len(moves) == 1
        assert moves[0].kind == "trend"
        assert moves[0].direction == "up"

    def test_gg_dd_no_overlap_descending(self):
        """GG/DD 完全不重叠 → 真正的下跌趋势。

        C1: zd=20, zg=30, dd=18, gg=35
        C2: zd=5, zg=15, dd=3, gg=17
        C2.gg=17 < C1.dd=18 → 波动区间不重叠 → 下跌
        """
        zhongshus = [
            _zs(0, 2, 20.0, 30.0, break_direction="down", dd=18.0, gg=35.0),
            _zs(6, 8, 5.0, 15.0, break_direction="down", dd=3.0, gg=17.0),
        ]
        moves = moves_from_zhongshus(zhongshus)
        assert len(moves) == 1
        assert moves[0].kind == "trend"
        assert moves[0].direction == "down"
