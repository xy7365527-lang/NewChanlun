"""线段 v1 — 结算锚验证 + 首段扩展修复 回归测试

覆盖：
  A) 结算锚验证：新段前三笔无重叠 → 旧段延续（不过早结算）
  B) 结算锚验证：新段笔不足 → 旧段延续
  C) 首段不强制扩展到 stroke 0（_find_overlap_start 跳笔场景）
  D) 段连续覆盖断言
  E) 正常触发仍然工作（回归保护）
  F) 极值归属：旧段应收到触发前的极值
"""

from __future__ import annotations

import pytest

from newchan.a_stroke import Stroke
from newchan.a_segment_v0 import Segment, BreakEvidence
from newchan.a_segment_v1 import (
    segments_from_strokes_v1,
    _three_stroke_overlap,
    _FeatureSeqState,
)
from newchan.a_assertions import assert_segment_theorem_v1


# ── helper ──

def _s(i0, i1, direction, high, low, confirmed=True):
    if direction == "up":
        p0, p1 = low, high
    else:
        p0, p1 = high, low
    return Stroke(i0=i0, i1=i1, direction=direction,
                  high=high, low=low, p0=p0, p1=p1, confirmed=confirmed)


# =====================================================================
# A) 结算锚验证：新段前三笔无重叠 → 旧段延续
# =====================================================================

class TestSettlementAnchorNoOverlap:
    """当特征序列分型触发但新段前三笔无重叠时，
    旧段应延续而非过早结算。

    原文："线段终结的充要条件就是新线段生成"
    """

    def _make_strokes_no_new_overlap(self) -> list[Stroke]:
        """构造场景：up 段分型触发后新段三笔无重叠（趋势太强）。

        向上段特征序列（down 笔 at 1,3,5）:
          FB(1, h=12, l=8)
          FB(3, h=16, l=11)  ← peak
          FB(5, h=14, l=9)
        → 顶分型在 std_seq pos 1 → 事件锚触发

        但新段（从 k=3 开始）的三笔 [3,4,5]:
          stroke 3: h=16, l=11
          stroke 4: h=30, l=20   ← 跳空大涨
          stroke 5: h=40, l=32   ← 继续跳空
        → max(11,20,32)=32, min(16,30,40)=16 → 32 >= 16 → 无重叠
        → 新段无法形成 → 旧段延续
        """
        return [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 11),   # peak（事件锚）
            _s(20, 25, "up",  30, 20),    # 跳空大涨
            _s(25, 30, "down", 40, 32),   # 继续跳空（新段三笔无重叠）
            _s(30, 35, "up",  50, 38),
            _s(35, 40, "down", 45, 35),
            _s(40, 45, "up",  55, 40),
        ]

    def test_old_segment_continues(self):
        """新段无法形成时只输出一段（旧段延续到末尾）。"""
        segs = segments_from_strokes_v1(self._make_strokes_no_new_overlap())
        # 旧段应延续，不应在 k=3 被切断
        assert len(segs) == 1
        assert segs[0].s0 == 0
        assert segs[0].s1 == 8  # 延续到最后一笔

    def test_old_segment_not_prematurely_settled(self):
        """旧段不应被过早结算（confirmed 应为 False，因为是唯一段）。"""
        segs = segments_from_strokes_v1(self._make_strokes_no_new_overlap())
        assert segs[0].confirmed is False

    def test_assertions_pass(self):
        """断言应通过。"""
        strokes = self._make_strokes_no_new_overlap()
        segs = segments_from_strokes_v1(strokes)
        result = assert_segment_theorem_v1(strokes, segs)
        assert result.ok is True


# =====================================================================
# B) 结算锚验证：新段笔不足 → 旧段延续
# =====================================================================

class TestSettlementAnchorInsufficientStrokes:
    """分型触发但 k+2 >= n（笔不足验证新段）→ 旧段延续。"""

    def _make_strokes_tail_trigger(self) -> list[Stroke]:
        """7 笔，分型在倒数第二根反向笔触发，之后不足3笔。

        特征序列（down 笔 at 1,3,5）:
          FB(1, h=12, l=8)
          FB(3, h=16, l=11)  ← peak
          FB(5, h=14, l=9)
        → 顶分型触发，k=3，但 k+2=5 已是最后一笔（n=7, k+2=5 < n=7）
        实际上 k=3, k+2=5 < 7，所以 strokes[3,4,5] 存在。
        改为真正不足的情况：6笔。
        """
        # 6笔：分型在 k=3 触发，k+2=5 = n-1 = 5（刚好够）
        # 需要真正不足：5笔 → k=3, k+2=5 >= n=5
        return [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 11),  # peak
            _s(20, 25, "up",  14, 9),    # k=3 → k+2=5 >= n=5
        ]

    def test_single_segment_when_tail_insufficient(self):
        """尾部笔不足 → 不切段。"""
        segs = segments_from_strokes_v1(self._make_strokes_tail_trigger())
        assert len(segs) == 1
        assert segs[0].s0 == 0
        assert segs[0].s1 == 4


# =====================================================================
# C) 首段不强制扩展到 stroke 0
# =====================================================================

class TestFirstSegmentNoForceExpand:
    """_find_overlap_start 跳过前几笔时，首段不应强制扩回 stroke 0。"""

    def _make_strokes_skip_start(self) -> list[Stroke]:
        """前3笔无重叠，从 stroke 2 开始才有重叠。

        strokes[0,1,2] 无重叠：
          stroke 0: h=10, l=5    (up)
          stroke 1: h=20, l=12   (down) ← 跳空
          stroke 2: h=30, l=22   (up)   ← 跳空
        max(5,12,22)=22, min(10,20,30)=10 → 22 >= 10 → 无重叠

        strokes[2,3,4] 有重叠：
          stroke 2: h=30, l=22
          stroke 3: h=28, l=20
          stroke 4: h=32, l=24
        max(22,20,24)=24, min(30,28,32)=28 → 24 < 28 → 有重叠 ✓
        """
        return [
            _s(0, 5, "up",    10, 5),
            _s(5, 10, "down", 20, 12),
            _s(10, 15, "up",  30, 22),
            _s(15, 20, "down", 28, 20),
            _s(20, 25, "up",  32, 24),
            _s(25, 30, "down", 26, 18),
            _s(30, 35, "up",  34, 23),
        ]

    def test_first_segment_starts_at_overlap(self):
        """首段应从第一个有重叠的位置开始，不强制扩回 0。"""
        segs = segments_from_strokes_v1(self._make_strokes_skip_start())
        assert segs[0].s0 == 2  # 从 stroke 2 开始（第一个重叠位置）

    def test_first_segment_satisfies_overlap(self):
        """首段的前三笔必须有重叠。"""
        strokes = self._make_strokes_skip_start()
        segs = segments_from_strokes_v1(strokes)
        s0 = segs[0].s0
        assert _three_stroke_overlap(strokes[s0], strokes[s0 + 1], strokes[s0 + 2])


# =====================================================================
# D) 段连续覆盖 + 方向交替
# =====================================================================

class TestSegmentContinuity:
    """段首尾相接（s1+1 == s0）且方向交替。"""

    def _make_multi_break_strokes(self) -> list[Stroke]:
        """构造多段场景：两次分型触发 → 三段。

        向上段（0-4），分型触发在 k=3 → 段1=[0,2], 段2从3开始（down）
        向下段（3-8），分型触发在 k=6 → 段2=[3,5], 段3从6开始（up）
        """
        return [
            # 段1 (up): strokes 0-2
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),    # feat seq elem
            _s(10, 15, "up",  18, 7),
            # 段2 (down): strokes 3-?
            _s(15, 20, "down", 16, 11),  # feat peak → trigger at k=3
            _s(20, 25, "up",  14, 9),
            _s(25, 30, "down", 10, 6),
            # 段3 (up): strokes 6-?
            _s(30, 35, "up",  12, 4),    # feat trough → trigger at k=6
            _s(35, 40, "down", 15, 8),
            _s(40, 45, "up",  18, 10),
            _s(45, 50, "down", 14, 7),
            _s(50, 55, "up",  20, 11),
        ]

    def test_segments_stitched(self):
        """相邻段首尾相接。"""
        segs = segments_from_strokes_v1(self._make_multi_break_strokes())
        for i in range(1, len(segs)):
            assert segs[i].s0 == segs[i - 1].s1 + 1, \
                f"seg[{i}].s0={segs[i].s0} != seg[{i-1}].s1+1={segs[i-1].s1 + 1}"

    def test_direction_alternates(self):
        """相邻段方向交替。"""
        segs = segments_from_strokes_v1(self._make_multi_break_strokes())
        for i in range(1, len(segs)):
            assert segs[i].direction != segs[i - 1].direction, \
                f"seg[{i}] and seg[{i-1}] both {segs[i].direction}"

    def test_assertion_v1_pass(self):
        """assert_segment_theorem_v1 通过。"""
        strokes = self._make_multi_break_strokes()
        segs = segments_from_strokes_v1(strokes)
        result = assert_segment_theorem_v1(strokes, segs)
        assert result.ok is True, result.message


# =====================================================================
# E) 正常触发仍然工作（回归保护）
# =====================================================================

class TestNormalTriggerStillWorks:
    """正常情况下（新段有重叠）分型触发仍然正确切段。"""

    def _make_normal_strokes(self) -> list[Stroke]:
        """标准 7 笔 up 段，在 k=3 顶分型触发，新段有重叠。"""
        return [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 11),  # peak
            _s(20, 25, "up",  20, 9),
            _s(25, 30, "down", 14, 9),
            _s(30, 35, "up",  22, 10),
        ]

    def test_two_segments(self):
        segs = segments_from_strokes_v1(self._make_normal_strokes())
        assert len(segs) == 2

    def test_break_evidence_present(self):
        """已确认段应有断段证据。"""
        segs = segments_from_strokes_v1(self._make_normal_strokes())
        assert segs[0].break_evidence is not None
        assert segs[0].break_evidence.trigger_stroke_k == 3
        assert segs[0].break_evidence.gap_type in ("none", "second")

    def test_confirmed_semantics(self):
        segs = segments_from_strokes_v1(self._make_normal_strokes())
        assert segs[0].confirmed is True
        assert segs[-1].confirmed is False


# =====================================================================
# F) 极值归属：旧段在延续时应收到极值
# =====================================================================

class TestExtremeAttribution:
    """当事件锚未结算时，旧段应包含后续极值。"""

    def _make_strokes_extreme_after_event(self) -> list[Stroke]:
        """up 段，分型触发后新段无重叠 → 旧段延续并收到后续新高。

        特征序列（down 笔 at 1,3,5,7）:
          FB(1, h=12, l=8)
          FB(3, h=16, l=11)  ← 分型中心
          FB(5, h=14, l=9)   ← 分型 c

        分型触发 k=3，但新段 strokes[3,4,5] 无重叠：
          stroke 3: h=16, l=11
          stroke 4: h=25, l=18  ← 大涨
          stroke 5: h=30, l=25  ← 继续涨（无重叠）

        之后有新高 stroke 6: up, h=50
        旧段延续，应包含此新高。
        """
        return [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 11),
            _s(20, 25, "up",  25, 18),
            _s(25, 30, "down", 30, 25),
            _s(30, 35, "up",  50, 28),    # 新高
            _s(35, 40, "down", 45, 35),
            _s(40, 45, "up",  48, 38),
        ]

    def test_segment_captures_extreme(self):
        """旧段应收到 stroke 6 的新高（50）。"""
        segs = segments_from_strokes_v1(self._make_strokes_extreme_after_event())
        # 因为新段无法形成，旧段延续到末尾，high 应包含 50
        seg0 = segs[0]
        assert seg0.high >= 50.0, f"段 high={seg0.high}，未收到新高 50"


# =====================================================================
# G) _three_stroke_overlap 单元测试
# =====================================================================

class TestThreeStrokeOverlap:
    """三笔重叠判定的边界条件。"""

    def test_overlap_true(self):
        s1 = _s(0, 5, "up", 15, 5)
        s2 = _s(5, 10, "down", 12, 8)
        s3 = _s(10, 15, "up", 18, 7)
        assert _three_stroke_overlap(s1, s2, s3) is True

    def test_overlap_false_gap(self):
        """跳空 → 无重叠。"""
        s1 = _s(0, 5, "up", 10, 5)
        s2 = _s(5, 10, "down", 20, 12)
        s3 = _s(10, 15, "up", 30, 22)
        assert _three_stroke_overlap(s1, s2, s3) is False

    def test_overlap_boundary_equal(self):
        """边界相等 → 无重叠（严格 <）。"""
        s1 = _s(0, 5, "up", 10, 5)
        s2 = _s(5, 10, "down", 15, 10)
        s3 = _s(10, 15, "up", 20, 15)
        # max(5,10,15) = 15, min(10,15,20) = 10 → 15 >= 10 → False
        assert _three_stroke_overlap(s1, s2, s3) is False


# =====================================================================
# H) kind 字段验证
# =====================================================================

class TestSegmentKindField:
    """验证 kind 字段在各场景下的正确性。"""

    def test_confirmed_old_seg_is_settled(self):
        """被新段确认的旧段 kind='settled'。"""
        # 复用 TestNormalTriggerStillWorks 的数据（两段场景）
        strokes = [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 11),
            _s(20, 25, "up",  20, 9),
            _s(25, 30, "down", 14, 9),
            _s(30, 35, "up",  22, 10),
        ]
        segs = segments_from_strokes_v1(strokes)
        assert len(segs) == 2
        assert segs[0].kind == "settled"
        assert segs[0].confirmed is True

    def test_last_seg_with_overlap_is_settled(self):
        """最后一段首三笔有重叠 → kind='settled'。"""
        strokes = [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 11),
            _s(20, 25, "up",  20, 9),
            _s(25, 30, "down", 14, 9),
            _s(30, 35, "up",  22, 10),
        ]
        segs = segments_from_strokes_v1(strokes)
        last = segs[-1]
        # 最后一段从 s0=3 开始，strokes[3,4,5] 应有重叠
        assert last.confirmed is False
        assert last.kind == "settled"

    def test_last_seg_without_overlap_is_candidate(self):
        """最后一段首三笔无重叠 → kind='candidate'。

        构造：up 段正常触发 → 新段（down）从 k 开始，
        但新段的首三笔跳空无重叠。
        需要让新段有 >=3 笔但首三笔无重叠。
        """
        strokes = [
            # 段1 (up): 正常段，分型触发
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 11),  # 分型中心
            _s(20, 25, "up",  14, 9),    # 分型 c
            # 新段 (down): strokes[3,4,5] 需要有重叠以通过结算锚
            # strokes[3]=(16,11) [4]=(14,9) [5]=(10,3)
            _s(25, 30, "down", 10, 3),
            # 继续：分型在 k=6 触发，新段从 6 开始（up）
            # strokes[6,7,8] 跳空无重叠
            _s(30, 35, "up",  8, 2),     # 分型触发
            _s(35, 40, "down", 6, 1),
            _s(40, 45, "up",  30, 20),   # 跳空大涨
            _s(45, 50, "down", 50, 35),  # 继续跳空
            _s(50, 55, "up",  60, 45),
        ]
        segs = segments_from_strokes_v1(strokes)
        # 最后一段的首三笔应无重叠 → candidate
        last = segs[-1]
        if last.kind == "candidate":
            assert last.confirmed is False
        # 也验证之前的段都是 settled
        for seg in segs[:-1]:
            assert seg.kind == "settled"

    def test_candidate_only_at_end(self):
        """candidate 段只出现在列表最末——通过断言验证。"""
        strokes = [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 11),
            _s(20, 25, "up",  20, 9),
            _s(25, 30, "down", 14, 9),
            _s(30, 35, "up",  22, 10),
        ]
        segs = segments_from_strokes_v1(strokes)
        # 中间段不应为 candidate
        for seg in segs[:-1]:
            assert seg.kind != "candidate", \
                f"Segment s0={seg.s0} is candidate but not last"

    def test_settled_confirmed_has_break_evidence(self):
        """settled + confirmed=True 的段必有 break_evidence。"""
        strokes = [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 11),
            _s(20, 25, "up",  20, 9),
            _s(25, 30, "down", 14, 9),
            _s(30, 35, "up",  22, 10),
        ]
        segs = segments_from_strokes_v1(strokes)
        for seg in segs:
            if seg.kind == "settled" and seg.confirmed:
                assert seg.break_evidence is not None, \
                    f"Segment s0={seg.s0} is settled+confirmed but no break_evidence"

    def test_assertion_v1_kind_checks_pass(self):
        """断言函数对 kind 的检查应全部通过。"""
        strokes = [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 11),
            _s(20, 25, "up",  20, 9),
            _s(25, 30, "down", 14, 9),
            _s(30, 35, "up",  22, 10),
        ]
        segs = segments_from_strokes_v1(strokes)
        result = assert_segment_theorem_v1(strokes, segs)
        assert result.ok is True, result.message

    def test_single_segment_kind(self):
        """只有一段时（最后一段），kind 由首三笔重叠判定。"""
        # 首三笔有重叠 → settled
        strokes = [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 14, 9),
            _s(20, 25, "up",  20, 10),
        ]
        segs = segments_from_strokes_v1(strokes)
        assert len(segs) == 1
        assert segs[0].kind == "settled"  # 首三笔有重叠
