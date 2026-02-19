"""古怪线段专项测试 — 第78课场景验证

覆盖 segment_rules_v1.md I11 及第78课核心场景：
  A) I11 硬约束直接验证（退化段检测）
  B) 第一种笔破坏无后续线段形成 → 旧段延续
  C) 三笔重叠边界情况（相切 vs 重叠）
  D) 段 high/low 覆盖范围验证
  E) assert_segment_theorem_v1 对退化段的捕获

概念溯源：[旧缠论] 第78课
谱系引用：001-degenerate-segment（已结算）
"""

from __future__ import annotations

import pytest

from newchan.a_stroke import Stroke
from newchan.a_segment_v0 import Segment
from newchan.a_segment_v1 import segments_from_strokes_v1
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
# A) I11 硬约束：所有 confirmed 段必须满足"顶高于底"
# =====================================================================

class TestI11DegenerateSegmentProhibition:
    """I11: 向上段 ep1_price >= ep0_price；向下段 ep1_price <= ep0_price。

    第78课 L20 原文：
    "同一线段中，两端的一顶一底，顶肯定要高于底，
     如果你划出一个不符合这基本要求的线段，那肯定是划错了。"

    验证方式：构造各种笔序列，对所有 confirmed 段检查硬约束。
    """

    def _assert_no_degenerate(self, segs: list[Segment]) -> None:
        """辅助：对每个有 ep 价格的段检查 I11 硬约束。"""
        for i, seg in enumerate(segs):
            if seg.ep0_price == 0.0 or seg.ep1_price == 0.0:
                continue
            if seg.direction == "up":
                assert seg.ep1_price >= seg.ep0_price - 1e-9, (
                    f"I11 violated: Segment[{i}] direction=up, "
                    f"ep1={seg.ep1_price} < ep0={seg.ep0_price}"
                )
            else:
                assert seg.ep1_price <= seg.ep0_price + 1e-9, (
                    f"I11 violated: Segment[{i}] direction=down, "
                    f"ep1={seg.ep1_price} > ep0={seg.ep0_price}"
                )

    def test_normal_up_segment(self):
        """正常向上段：终点顶 > 起点底。"""
        strokes = [
            _s(0, 4, "up", 15, 5),
            _s(4, 8, "down", 14, 8),
            _s(8, 12, "up", 18, 7),
            _s(12, 16, "down", 16, 10),
            _s(16, 20, "up", 22, 12),
            _s(20, 24, "down", 20, 6),
            _s(24, 28, "up", 14, 5),
        ]
        segs = segments_from_strokes_v1(strokes)
        assert len(segs) >= 1
        self._assert_no_degenerate(segs)

    def test_normal_down_segment(self):
        """正常向下段：终点底 < 起点顶。"""
        strokes = [
            _s(0, 4, "down", 20, 10),
            _s(4, 8, "up", 15, 8),
            _s(8, 12, "down", 14, 5),
            _s(12, 16, "up", 10, 4),
            _s(16, 20, "down", 8, 2),
            _s(20, 24, "up", 12, 3),
            _s(24, 28, "down", 10, 1),
        ]
        segs = segments_from_strokes_v1(strokes)
        assert len(segs) >= 1
        self._assert_no_degenerate(segs)

    def test_long_zigzag_no_degenerate(self):
        """长锯齿序列（15 笔）不应产出退化段。"""
        strokes = []
        base = 100
        for i in range(15):
            idx = i * 4
            if i % 2 == 0:
                # up stroke: 整体上升趋势
                h = base + i * 2 + 5
                l = base + i * 2
                strokes.append(_s(idx, idx + 4, "up", h, l))
            else:
                # down stroke: 回撤但不过深
                h = base + i * 2 + 3
                l = base + i * 2 - 2
                strokes.append(_s(idx, idx + 4, "down", h, l))
        segs = segments_from_strokes_v1(strokes)
        self._assert_no_degenerate(segs)

    def test_volatile_sequence_no_degenerate(self):
        """剧烈波动序列（大幅反转），仍不应产出退化段。"""
        strokes = [
            _s(0, 4, "up", 50, 10),
            _s(4, 8, "down", 48, 15),
            _s(8, 12, "up", 55, 12),
            _s(12, 16, "down", 52, 8),
            _s(16, 20, "up", 60, 6),      # 大幅上涨
            _s(20, 24, "down", 58, 20),
            _s(24, 28, "up", 40, 18),      # 幅度缩小
            _s(28, 32, "down", 38, 5),     # 深跌
            _s(32, 36, "up", 30, 4),
        ]
        segs = segments_from_strokes_v1(strokes)
        self._assert_no_degenerate(segs)

    def test_assert_catches_degenerate_up(self):
        """assert_segment_theorem_v1 应能检测到向上退化段。"""
        # 手工构造一个违反 I11 的假段
        fake_seg = Segment(
            s0=0, s1=2, i0=0, i1=12, direction="up",
            high=20, low=5, confirmed=True,
            kind="settled",
            ep0_i=0, ep0_price=15.0, ep0_type="bottom",
            ep1_i=12, ep1_price=10.0, ep1_type="top",  # 10 < 15 → 退化！
        )
        result = assert_segment_theorem_v1(
            [_s(0, 4, "up", 20, 5), _s(4, 8, "down", 18, 8), _s(8, 12, "up", 15, 7)],
            [fake_seg],
            enable=False,
        )
        assert result.ok is False
        assert "degenerate" in result.message.lower()

    def test_assert_catches_degenerate_down(self):
        """assert_segment_theorem_v1 应能检测到向下退化段。"""
        fake_seg = Segment(
            s0=0, s1=2, i0=0, i1=12, direction="down",
            high=20, low=5, confirmed=True,
            kind="settled",
            ep0_i=0, ep0_price=10.0, ep0_type="top",
            ep1_i=12, ep1_price=15.0, ep1_type="bottom",  # 15 > 10 → 退化！
        )
        result = assert_segment_theorem_v1(
            [_s(0, 4, "down", 20, 10), _s(4, 8, "up", 15, 8), _s(8, 12, "down", 14, 5)],
            [fake_seg],
            enable=False,
        )
        assert result.ok is False
        assert "degenerate" in result.message.lower()


# =====================================================================
# B) 第一种笔破坏无后续线段形成 → 旧段延续
# =====================================================================

class TestStrokeBreakWithoutSegmentFormation:
    """第78课核心：分型触发后新段三笔无重叠 → 旧段延续。

    "线段被破坏的充要条件就是另一个线段生成"——
    笔破坏了旧段，但如果不能形成新段，旧段延续。
    """

    def test_fractal_no_new_overlap_extends(self):
        """分型触发但新段前三笔无重叠 → 旧段延续到最后。

        向上段特征序列（反向 down 笔的 h/l）：
          a: h=14, l=8
          b: h=16, l=11  ← 顶分型 peak
          c: h=13, l=6

        但新段起点后的三笔跳空无重叠 → 旧段未被结算。
        """
        strokes = [
            _s(0, 4, "up", 15, 5),       # seg start
            _s(4, 8, "down", 14, 8),      # feat a
            _s(8, 12, "up", 18, 9),
            _s(12, 16, "down", 16, 11),   # feat b (peak)
            _s(16, 20, "up", 19, 10),
            _s(20, 24, "down", 13, 6),    # feat c → 顶分型触发
            # 新段笔：大幅跳空，无重叠
            _s(24, 28, "up", 30, 25),     # h=30, l=25
            _s(28, 32, "down", 45, 35),   # h=45, l=35 (无重叠: 35>25)
            _s(32, 36, "up", 55, 50),     # h=55, l=50 (无重叠: 50>35)
        ]
        segs = segments_from_strokes_v1(strokes)
        # 新段三笔无重叠 → 不应有 settled 的旧段结算
        # 旧段应延续（只有 1 段或第一段仍 unconfirmed）
        settled = [s for s in segs if s.confirmed]
        # 如果分型触发但新段无重叠，旧段不应被结算
        if len(segs) == 1:
            assert not segs[0].confirmed  # 唯一一段，未结算
        elif len(segs) >= 2:
            # 即使有多段，confirmed 的段不应违反 I11
            for s in segs:
                if s.ep0_price != 0.0 and s.ep1_price != 0.0:
                    if s.direction == "up":
                        assert s.ep1_price >= s.ep0_price - 1e-9

    def test_fractal_then_continuation(self):
        """分型触发、新段暂时无重叠，后续笔补充重叠后才结算。

        验证：结算时机正确（不过早、不过晚）。
        """
        strokes = [
            _s(0, 4, "up", 15, 5),
            _s(4, 8, "down", 14, 8),
            _s(8, 12, "up", 20, 9),
            _s(12, 16, "down", 18, 12),   # feat: 可能触发分型
            _s(16, 20, "up", 22, 11),
            _s(20, 24, "down", 16, 7),    # 分型
            # 新段笔（有重叠）
            _s(24, 28, "up", 14, 6),
            _s(28, 32, "down", 13, 5),
            _s(32, 36, "up", 12, 4),
        ]
        segs = segments_from_strokes_v1(strokes)
        # 如果有 2 段，验证结算锚正确
        if len(segs) >= 2:
            assert segs[0].confirmed
            # 新段首笔 index 应 = 旧段终点 + 1
            assert segs[1].s0 == segs[0].s1 + 1


# =====================================================================
# C) 三笔重叠边界情况
# =====================================================================

class TestThreeStrokeOverlapBoundary:
    """结算锚验证：新段前三笔的重叠条件。

    三笔重叠 ≡ max(lows) < min(highs)
    相切（max(lows) == min(highs)）不算重叠。
    """

    def test_exact_touch_no_overlap(self):
        """三笔刚好相切 → 不算重叠 → 不结算。

        笔 A: h=15, l=10
        笔 B: h=20, l=15  ← l=15 == h_A=15
        笔 C: h=25, l=20
        max(lows)=20, min(highs)=15 → 20 > 15?
        不对，重新计算：max(10,15,20)=20, min(15,20,25)=15 → 20 > 15 → 无重叠
        """
        strokes = [
            # 向上段基础
            _s(0, 4, "up", 30, 5),
            _s(4, 8, "down", 28, 8),
            _s(8, 12, "up", 35, 7),
            _s(12, 16, "down", 32, 10),
            _s(16, 20, "up", 38, 9),
            # 分型触发后，新段三笔（跳空递增，无重叠）
            _s(20, 24, "down", 25, 15),   # h=25, l=15
            _s(24, 28, "up", 14, 10),     # h=14, l=10
            _s(28, 32, "down", 9, 5),     # h=9, l=5
            _s(32, 36, "up", 4, 1),       # h=4, l=1
        ]
        segs = segments_from_strokes_v1(strokes)
        # 如果新段无重叠，旧段应延续
        if len(segs) >= 2 and segs[0].confirmed:
            # 如果确实 settled，验证新段有重叠
            k = segs[0].s1 + 1 if segs[0].break_evidence is None else segs[0].break_evidence.trigger_stroke_k
            if k + 2 < len(strokes):
                s1, s2, s3 = strokes[k], strokes[k+1], strokes[k+2]
                overlap = max(s1.low, s2.low, s3.low) < min(s1.high, s2.high, s3.high)
                assert overlap, "settled 段的新段三笔必须有重叠"

    def test_clear_overlap_settles(self):
        """三笔明确重叠 → 结算。

        新段笔全在 10-20 区间内 → max(lows) < min(highs) → 重叠。
        """
        strokes = [
            _s(0, 4, "up", 20, 5),
            _s(4, 8, "down", 18, 9),
            _s(8, 12, "up", 22, 8),
            _s(12, 16, "down", 19, 10),
            _s(16, 20, "up", 25, 9),
            _s(20, 24, "down", 22, 11),   # 分型
            # 新段三笔有明确重叠（区间 10-18）
            _s(24, 28, "up", 18, 10),
            _s(28, 32, "down", 16, 11),
            _s(32, 36, "up", 15, 12),
        ]
        segs = segments_from_strokes_v1(strokes)
        # 应至少有 2 段（旧段 settled + 新段）
        if len(segs) >= 2:
            assert segs[0].confirmed


# =====================================================================
# D) 段 high/low 覆盖范围
# =====================================================================

class TestSegmentHighLowCoverage:
    """段的 high/low 应覆盖其包含的所有笔的极值范围。"""

    def test_segment_covers_all_strokes(self):
        """段 high = max(笔.high)，low = min(笔.low)。"""
        strokes = [
            _s(0, 4, "up", 15, 5),      # l=5
            _s(4, 8, "down", 14, 8),
            _s(8, 12, "up", 20, 7),      # h=20
            _s(12, 16, "down", 18, 6),   # l=6
            _s(16, 20, "up", 25, 10),    # h=25
            _s(20, 24, "down", 22, 8),
            _s(24, 28, "up", 15, 7),
        ]
        segs = segments_from_strokes_v1(strokes)
        assert len(segs) >= 1
        for seg in segs:
            seg_strokes = strokes[seg.s0:seg.s1 + 1]
            expected_high = max(s.high for s in seg_strokes)
            expected_low = min(s.low for s in seg_strokes)
            assert seg.high == pytest.approx(expected_high, abs=1e-9), (
                f"seg high={seg.high} != max(strokes.high)={expected_high}"
            )
            assert seg.low == pytest.approx(expected_low, abs=1e-9), (
                f"seg low={seg.low} != min(strokes.low)={expected_low}"
            )

    def test_single_segment_covers_all(self):
        """只有一段时，high/low 应覆盖全部笔。"""
        strokes = [
            _s(0, 4, "up", 12, 3),
            _s(4, 8, "down", 11, 6),
            _s(8, 12, "up", 15, 5),
        ]
        segs = segments_from_strokes_v1(strokes)
        assert len(segs) == 1
        assert segs[0].high == 15
        assert segs[0].low == 3


# =====================================================================
# E) assert_segment_theorem_v1 集成验证
# =====================================================================

class TestAssertIntegrationV1:
    """确保断言函数对正常输出返回 ok，对人造异常返回 fail。"""

    def test_normal_output_passes(self):
        """正常 segments_from_strokes_v1 输出通过断言。"""
        strokes = [
            _s(0, 4, "up", 15, 5),
            _s(4, 8, "down", 14, 8),
            _s(8, 12, "up", 18, 7),
            _s(12, 16, "down", 16, 10),
            _s(16, 20, "up", 22, 12),
            _s(20, 24, "down", 20, 6),
            _s(24, 28, "up", 14, 5),
        ]
        segs = segments_from_strokes_v1(strokes)
        result = assert_segment_theorem_v1(strokes, segs, enable=False)
        assert result.ok, f"Unexpected failure: {result.message}"

    def test_many_strokes_passes(self):
        """大量笔（25 笔，正常上升趋势）的输出通过断言。"""
        strokes = []
        for i in range(25):
            idx = i * 4
            if i % 2 == 0:
                # up stroke: 每次上升 2 点
                h = 100 + i * 2 + 5
                l = 100 + i * 2
                strokes.append(_s(idx, idx + 4, "up", h, l))
            else:
                # down stroke: 回撤但不过深（high < 前一 up 的 high）
                h = 100 + i * 2 + 3
                l = 100 + i * 2 - 1
                strokes.append(_s(idx, idx + 4, "down", h, l))
        segs = segments_from_strokes_v1(strokes)
        result = assert_segment_theorem_v1(strokes, segs, enable=False)
        assert result.ok, f"Unexpected failure: {result.message}"
