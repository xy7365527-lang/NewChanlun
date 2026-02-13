"""笔构造 — 单元测试 (Step10)

覆盖 docs/chan_spec.md §4 全部规则：
  A) 去重：top-top-bottom → 只保留更极端 top
  B) 交替：bottom-bottom-top → 只保留更极端 bottom
  C) wide 笔：idx 差 = 4 时 wide 可成笔
  D) strict 笔：idx 差 = 4 时 strict 不成笔，idx 差 = 5 时成笔
  E) confirmed：多笔时最后一笔 confirmed=False，前面 True
  F) 断言集成：assert_stroke_alternation_and_gap 返回 ok=True
"""

from __future__ import annotations

import pandas as pd
import pytest

from newchan.a_fractal import Fractal
from newchan.a_stroke import (
    Stroke,
    dedupe_fractals,
    enforce_alternation,
    strokes_from_fractals,
)
from newchan.a_assertions import assert_stroke_alternation_and_gap


# ── 通用 helper ──────────────────────────────────────────────────────

def _make_merged(n: int = 15) -> pd.DataFrame:
    """15 根 merged K 线：下行→上行→下行（V-倒V 形态）。

    idx:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14
    high: 20  18  16  14  12  14  16  18  20  22  20  18  16  14  12
    low:  15  13  11   9   7   9  11  13  15  17  15  13  11   9   7
    """
    idx = pd.to_datetime([f"2025-03-{i+1:02d}" for i in range(n)])
    highs = [20, 18, 16, 14, 12, 14, 16, 18, 20, 22, 20, 18, 16, 14, 12]
    lows  = [15, 13, 11,  9,  7,  9, 11, 13, 15, 17, 15, 13, 11,  9,  7]
    opens = [h - 1 for h in highs]
    closes = [l + 1 for l in lows]
    return pd.DataFrame(
        {"open": opens, "high": highs, "low": lows, "close": closes},
        index=idx,
    )


# =====================================================================
# A) 去重：top-top-bottom → 只保留更极端 top
# =====================================================================

class TestDedupe:
    """§4.2 分型去重。"""

    def test_top_top_keeps_higher(self):
        fxs = [
            Fractal(idx=2, kind="top", price=15.0),
            Fractal(idx=5, kind="top", price=18.0),
            Fractal(idx=8, kind="bottom", price=5.0),
        ]
        result = dedupe_fractals(fxs)
        assert len(result) == 2
        assert result[0] == Fractal(idx=5, kind="top", price=18.0)
        assert result[1] == Fractal(idx=8, kind="bottom", price=5.0)

    def test_bottom_bottom_keeps_lower(self):
        fxs = [
            Fractal(idx=2, kind="bottom", price=5.0),
            Fractal(idx=5, kind="bottom", price=3.0),
            Fractal(idx=8, kind="top", price=18.0),
        ]
        result = dedupe_fractals(fxs)
        assert len(result) == 2
        assert result[0] == Fractal(idx=5, kind="bottom", price=3.0)
        assert result[1] == Fractal(idx=8, kind="top", price=18.0)

    def test_triple_same_kind(self):
        """三连 top → 只保留最高。"""
        fxs = [
            Fractal(idx=1, kind="top", price=10.0),
            Fractal(idx=3, kind="top", price=20.0),
            Fractal(idx=5, kind="top", price=15.0),
        ]
        result = dedupe_fractals(fxs)
        assert len(result) == 1
        assert result[0].price == 20.0

    def test_empty(self):
        assert dedupe_fractals([]) == []


# =====================================================================
# B) 交替：bottom-bottom-top → 只保留更极端 bottom
# =====================================================================

class TestAlternation:
    """§4.2 顶底交替强制。"""

    def test_bottom_bottom_top(self):
        fxs = [
            Fractal(idx=2, kind="bottom", price=5.0),
            Fractal(idx=4, kind="bottom", price=3.0),
            Fractal(idx=8, kind="top", price=18.0),
        ]
        result = enforce_alternation(fxs)
        assert len(result) == 2
        assert result[0].kind == "bottom"
        assert result[0].price == 3.0
        assert result[1].kind == "top"

    def test_already_alternating(self):
        fxs = [
            Fractal(idx=2, kind="bottom", price=5.0),
            Fractal(idx=6, kind="top", price=18.0),
            Fractal(idx=10, kind="bottom", price=4.0),
        ]
        result = enforce_alternation(fxs)
        assert len(result) == 3
        assert [f.kind for f in result] == ["bottom", "top", "bottom"]

    def test_empty(self):
        assert enforce_alternation([]) == []


# =====================================================================
# C) wide 笔：idx 差 = 4 时成笔
# =====================================================================

class TestWideStroke:
    """§4.3 宽笔（mode='wide'，gap >= 4）。"""

    def test_gap_4_produces_stroke(self):
        """bottom(idx=4) → top(idx=8): gap=4 >= 4 → 成笔。"""
        df = _make_merged()
        fxs = [
            Fractal(idx=4, kind="bottom", price=7.0),
            Fractal(idx=8, kind="top", price=20.0),
        ]
        strokes = strokes_from_fractals(df, fxs, mode="wide")
        assert len(strokes) == 1
        assert strokes[0].direction == "up"
        assert strokes[0].i0 == 4
        assert strokes[0].i1 == 8

    def test_gap_3_no_stroke(self):
        """bottom(idx=4) → top(idx=7): gap=3 < 4 → 不成笔。"""
        df = _make_merged()
        fxs = [
            Fractal(idx=4, kind="bottom", price=7.0),
            Fractal(idx=7, kind="top", price=18.0),
        ]
        strokes = strokes_from_fractals(df, fxs, mode="wide")
        assert len(strokes) == 0

    def test_high_low_range(self):
        """笔的 high/low 取 i0..i1 区间极值；p0/p1 为起点/终点分型价。"""
        df = _make_merged()
        fxs = [
            Fractal(idx=4, kind="bottom", price=7.0),
            Fractal(idx=9, kind="top", price=22.0),
        ]
        strokes = strokes_from_fractals(df, fxs, mode="wide")
        assert len(strokes) == 1
        # idx 4..9: highs=[12,14,16,18,20,22] → max=22
        #           lows =[7,9,11,13,15,17]   → min=7
        assert strokes[0].high == 22.0
        assert strokes[0].low == 7.0
        # up 笔：p0=起点底分型价, p1=终点顶分型价
        assert strokes[0].p0 == 7.0
        assert strokes[0].p1 == 22.0


# =====================================================================
# D) strict 笔：idx 差 = 4 时不成笔，idx 差 = 5 时成笔
# =====================================================================

class TestStrictStroke:
    """§4.3 严笔（mode='strict'，gap >= min_strict_sep）。"""

    def test_gap_4_strict_no_stroke(self):
        """gap=4 < min_strict_sep=5 → strict 模式不成笔。"""
        df = _make_merged()
        fxs = [
            Fractal(idx=4, kind="bottom", price=7.0),
            Fractal(idx=8, kind="top", price=20.0),
        ]
        strokes = strokes_from_fractals(df, fxs, mode="strict", min_strict_sep=5)
        assert len(strokes) == 0

    def test_gap_5_strict_produces_stroke(self):
        """gap=5 >= min_strict_sep=5 → strict 模式成笔。"""
        df = _make_merged()
        fxs = [
            Fractal(idx=4, kind="bottom", price=7.0),
            Fractal(idx=9, kind="top", price=22.0),
        ]
        strokes = strokes_from_fractals(df, fxs, mode="strict", min_strict_sep=5)
        assert len(strokes) == 1
        assert strokes[0].direction == "up"
        assert strokes[0].i0 == 4
        assert strokes[0].i1 == 9


# =====================================================================
# E) confirmed：多笔时最后一笔 False，前面 True
# =====================================================================

class TestConfirmed:
    """§4.5 笔的确认语义。"""

    def test_two_strokes_confirmed(self):
        """两笔: 第一笔 confirmed=True，最后一笔 confirmed=False。"""
        df = _make_merged()
        # bottom(4) → top(9) → bottom(14)
        # gap: 9-4=5, 14-9=5, 都满足 wide
        fxs = [
            Fractal(idx=4, kind="bottom", price=7.0),
            Fractal(idx=9, kind="top", price=22.0),
            Fractal(idx=14, kind="bottom", price=7.0),
        ]
        strokes = strokes_from_fractals(df, fxs, mode="wide")
        assert len(strokes) == 2
        assert strokes[0].confirmed is True
        assert strokes[1].confirmed is False

    def test_single_stroke_unconfirmed(self):
        """单笔时也是 confirmed=False。"""
        df = _make_merged()
        fxs = [
            Fractal(idx=4, kind="bottom", price=7.0),
            Fractal(idx=9, kind="top", price=22.0),
        ]
        strokes = strokes_from_fractals(df, fxs, mode="wide")
        assert len(strokes) == 1
        assert strokes[0].confirmed is False

    def test_three_strokes_confirmed(self):
        """三笔: 前两笔 True，最后一笔 False。"""
        df = _make_merged()
        fxs = [
            Fractal(idx=0, kind="top", price=20.0),
            Fractal(idx=4, kind="bottom", price=7.0),
            Fractal(idx=9, kind="top", price=22.0),
            Fractal(idx=14, kind="bottom", price=7.0),
        ]
        strokes = strokes_from_fractals(df, fxs, mode="wide")
        assert len(strokes) == 3
        assert strokes[0].confirmed is True
        assert strokes[1].confirmed is True
        assert strokes[2].confirmed is False

    def test_directions_alternate(self):
        """多笔时方向严格交替。"""
        df = _make_merged()
        fxs = [
            Fractal(idx=0, kind="top", price=20.0),
            Fractal(idx=4, kind="bottom", price=7.0),
            Fractal(idx=9, kind="top", price=22.0),
            Fractal(idx=14, kind="bottom", price=7.0),
        ]
        strokes = strokes_from_fractals(df, fxs, mode="wide")
        dirs = [s.direction for s in strokes]
        assert dirs == ["down", "up", "down"]


# =====================================================================
# F) 断言集成
# =====================================================================

class TestAssertIntegration:
    """assert_stroke_alternation_and_gap 集成测试。"""

    def test_valid_strokes_pass(self):
        df = _make_merged()
        fxs = [
            Fractal(idx=4, kind="bottom", price=7.0),
            Fractal(idx=9, kind="top", price=22.0),
            Fractal(idx=14, kind="bottom", price=7.0),
        ]
        strokes = strokes_from_fractals(df, fxs, mode="wide")
        result = assert_stroke_alternation_and_gap(strokes, "wide")
        assert result.ok is True

    def test_strict_strokes_pass(self):
        df = _make_merged()
        fxs = [
            Fractal(idx=4, kind="bottom", price=7.0),
            Fractal(idx=9, kind="top", price=22.0),
            Fractal(idx=14, kind="bottom", price=7.0),
        ]
        strokes = strokes_from_fractals(df, fxs, mode="strict", min_strict_sep=5)
        result = assert_stroke_alternation_and_gap(strokes, "strict", 5)
        assert result.ok is True

    def test_empty_strokes_pass(self):
        result = assert_stroke_alternation_and_gap([])
        assert result.ok is True

    def test_bad_alternation_fails(self):
        """手工构造方向不交替的 strokes → 断言失败。"""
        bad = [
            Stroke(i0=0, i1=5, direction="up", high=20, low=5, p0=5, p1=20, confirmed=True),
            Stroke(i0=5, i1=10, direction="up", high=22, low=7, p0=7, p1=22, confirmed=False),
        ]
        result = assert_stroke_alternation_and_gap(bad, "wide")
        assert result.ok is False
        assert "alternation" in result.message

    def test_bad_confirmed_fails(self):
        """最后一笔 confirmed=True → 断言失败。"""
        bad = [
            Stroke(i0=0, i1=5, direction="up", high=20, low=5, p0=5, p1=20, confirmed=True),
            Stroke(i0=5, i1=10, direction="down", high=22, low=7, p0=22, p1=7, confirmed=True),
        ]
        result = assert_stroke_alternation_and_gap(bad, "wide")
        assert result.ok is False
        assert "confirmed=False" in result.message


# =====================================================================
# 连续性：strokes[i].i1 == strokes[i+1].i0，且 p0/p1 正确
# =====================================================================

class TestContinuity:
    """strokes_from_fractals 产生的笔满足首尾相连与 p0/p1 正确性。"""

    def test_strokes_connect(self):
        """strokes[i].i1 == strokes[i+1].i0 对所有相邻笔成立。"""
        df = _make_merged()
        fxs = [
            Fractal(idx=0, kind="top", price=20.0),
            Fractal(idx=4, kind="bottom", price=7.0),
            Fractal(idx=9, kind="top", price=22.0),
            Fractal(idx=14, kind="bottom", price=7.0),
        ]
        strokes = strokes_from_fractals(df, fxs, mode="wide")
        assert len(strokes) >= 2
        for i in range(len(strokes) - 1):
            assert strokes[i].i1 == strokes[i + 1].i0, (
                f"strokes[{i}].i1={strokes[i].i1} != strokes[{i+1}].i0={strokes[i+1].i0}"
            )

    def test_p0_p1_correct(self):
        """所有笔的 p0/p1 符合语义：up→p0=low,p1=high；down→p0=high,p1=low。"""
        df = _make_merged()
        fxs = [
            Fractal(idx=0, kind="top", price=20.0),
            Fractal(idx=4, kind="bottom", price=7.0),
            Fractal(idx=9, kind="top", price=22.0),
            Fractal(idx=14, kind="bottom", price=7.0),
        ]
        strokes = strokes_from_fractals(df, fxs, mode="wide")
        for s in strokes:
            if s.direction == "up":
                assert s.p0 == s.low, f"up stroke p0={s.p0} should equal low={s.low}"
                assert s.p1 == s.high, f"up stroke p1={s.p1} should equal high={s.high}"
            else:
                assert s.p0 == s.high, f"down stroke p0={s.p0} should equal high={s.high}"
                assert s.p1 == s.low, f"down stroke p1={s.p1} should equal low={s.low}"
