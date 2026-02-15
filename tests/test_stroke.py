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


# =====================================================================
# G) 新笔模式：mode="new"，在原始K线上计数
# =====================================================================

class TestNewBiMode:
    """新笔定义：间距在原始（raw）K线上计数，不考虑包含关系。

    关键场景：merged 间距不足（gap=3 < 4），但包含处理前
    原始K线间距足够（raw gap >= 3）。旧笔不成笔，新笔成笔。
    """

    @staticmethod
    def _make_merged_with_compression() -> tuple[pd.DataFrame, list[tuple[int, int]]]:
        """构造一组数据：merged bar 间有包含压缩。

        原始 8 根 K 线：
            raw idx:   0    1    2    3    4    5    6    7
            high:     20   19   18   15   14   13   16   18
            low:      15   14   13   10    9    8   11   13

        包含处理后（假设向下合并 raw 1-2 → merged 1，raw 3-4 → merged 2）:
            merged idx:  0    1    2    3    4
            high:       20   18   14   16   18
            low:        15   13    8   11   13

        分型：
            merged idx=0: top (20>18, 15>13 → 双条件)
            merged idx=2: bottom (8<13, 14<18 → 双条件)
                但 idx=2 的 low < idx=1 的 low AND high < idx=1 的 high
                AND idx=2 的 low < idx=3 的 low AND high < idx=3 的 high → OK

        merged gap = 2 - 0 = 2 < 4 → 旧笔不成笔
        raw gap: raw_end(merged[0])=0, raw_start(merged[2])=3
                 原始间距 = 3 - 0 - 1 = 2 ... 不够

        需要更精心的设计。让我重新设计。
        """
        # 10 根原始K线，合并后变 6 根 merged bar
        # 其中 raw[1,2] 合并为 merged[1]，raw[5,6] 合并为 merged[4]
        #
        # merged idx:   0        1       2     3       4       5
        # raw range:   (0,0)   (1,2)   (3,3) (4,4)  (5,6)   (7,7)
        # high:         20      18       14    16      19      17
        # low:          15      13        9    11      14      12
        #
        # top fractal at merged idx=0: h=20>18, l=15>13 ... need left neighbor
        # Actually merged idx=0 can't be a fractal (no left neighbor)
        #
        # Let me use a simpler approach: just create the merged df and
        # merged_to_raw directly, since we're testing strokes_from_fractals.

        # merged bar 序列（7根），idx 1 和 idx 5 各压缩了2根raw
        highs = [20, 18, 14, 12, 14, 18, 20]
        lows =  [15, 13,  9,  7,  9, 13, 15]
        n = len(highs)
        idx = pd.to_datetime([f"2025-03-{i+1:02d}" for i in range(n)])
        df = pd.DataFrame(
            {
                "open": [h - 1 for h in highs],
                "high": highs,
                "low": lows,
                "close": [l + 1 for l in lows],
            },
            index=idx,
        )
        # merged_to_raw: merged[1] 和 merged[5] 各压缩2根raw bar
        # raw indices:    0, (1,2), 3, 4, 5, (6,7), 8
        merged_to_raw = [
            (0, 0),    # merged 0
            (1, 2),    # merged 1 ← 压缩了2根raw
            (3, 3),    # merged 2
            (4, 4),    # merged 3
            (5, 5),    # merged 4
            (6, 7),    # merged 5 ← 压缩了2根raw
            (8, 8),    # merged 6
        ]
        return df, merged_to_raw

    def test_old_bi_rejects_gap3(self):
        """旧笔：merged gap=3 不成笔。"""
        df, merged_to_raw = self._make_merged_with_compression()
        # top at merged idx=1 (h=18), bottom at merged idx=3 (l=7)
        # merged gap = 3 - 1 = 2 < 4 → too small for old bi
        # Use idx=1 top and idx=5 bottom... let's pick fractals with gap=3
        fxs = [
            Fractal(idx=1, kind="top", price=18.0),
            Fractal(idx=4, kind="bottom", price=9.0),
        ]
        # merged gap = 4 - 1 = 3 < 4 → 旧笔不成笔
        strokes = strokes_from_fractals(df, fxs, mode="wide")
        assert len(strokes) == 0

    def test_new_bi_accepts_with_raw_gap(self):
        """新笔：merged gap=3，但 raw gap >= 3 → 成笔。

        merged[1] raw_end=2, merged[4] raw_start=5
        raw gap = 5 - 2 - 1 = 2 ... 还是不够。

        需要让中间的 merged bar 也有压缩。重新设计数据。
        """
        # 专门构造的数据：所有中间 merged bar 都包含压缩
        # merged 5 根，每根对应多根raw
        highs = [20, 16, 10, 14, 18]
        lows =  [15, 11,  5,  9, 13]
        n = len(highs)
        idx = pd.to_datetime([f"2025-04-{i+1:02d}" for i in range(n)])
        df = pd.DataFrame(
            {
                "open": [h - 1 for h in highs],
                "high": highs,
                "low": lows,
                "close": [l + 1 for l in lows],
            },
            index=idx,
        )
        # merged_to_raw: 中间 3 根各压缩2根raw
        # raw: 0, (1,2), (3,4), (5,6), 7
        merged_to_raw = [
            (0, 0),    # merged 0  (top fractal 无法在此，无左邻)
            (1, 2),    # merged 1
            (3, 4),    # merged 2
            (5, 6),    # merged 3
            (7, 7),    # merged 4
        ]
        # top at merged 0, bottom at merged 2
        # 但 merged 0 不能是分型（无左邻居）
        # 用 top at merged 1 (price=16), bottom at merged 3 (price=9)
        # merged gap = 3 - 1 = 2 < 4 → 旧笔不成笔
        # raw: merged_to_raw[1][1] = 2, merged_to_raw[3][0] = 5
        # raw gap = 5 - 2 - 1 = 2 ... 还是只有2

        # 需要更多压缩。让每个中间 bar 压缩 3 根。
        merged_to_raw_v2 = [
            (0, 1),     # merged 0: 2 raw bars
            (2, 4),     # merged 1: 3 raw bars (top)
            (5, 7),     # merged 2: 3 raw bars
            (8, 10),    # merged 3: 3 raw bars (bottom)
            (11, 12),   # merged 4: 2 raw bars
        ]
        fxs = [
            Fractal(idx=1, kind="top", price=16.0),
            Fractal(idx=3, kind="bottom", price=9.0),
        ]
        # merged gap = 3 - 1 = 2 < 4 → old bi fails
        strokes_old = strokes_from_fractals(df, fxs, mode="wide")
        assert len(strokes_old) == 0, "旧笔应拒绝 merged gap=2"

        # raw: merged_to_raw_v2[1][1] = 4, merged_to_raw_v2[3][0] = 8
        # raw gap = 8 - 4 - 1 = 3 >= 3 → new bi accepts
        strokes_new = strokes_from_fractals(
            df, fxs, mode="new", merged_to_raw=merged_to_raw_v2,
        )
        assert len(strokes_new) == 1, "新笔应接受 raw gap=3"
        assert strokes_new[0].direction == "down"

    def test_new_bi_rejects_insufficient_raw_gap(self):
        """新笔：即使传了 merged_to_raw，raw gap < 3 仍然不成笔。"""
        highs = [20, 16, 10, 14, 18]
        lows =  [15, 11,  5,  9, 13]
        n = len(highs)
        idx = pd.to_datetime([f"2025-05-{i+1:02d}" for i in range(n)])
        df = pd.DataFrame(
            {
                "open": [h - 1 for h in highs],
                "high": highs,
                "low": lows,
                "close": [l + 1 for l in lows],
            },
            index=idx,
        )
        # 中间 bar 只有少量压缩 → raw gap 不够
        merged_to_raw = [
            (0, 0),
            (1, 2),    # merged 1 (top): raw 1-2
            (3, 3),    # merged 2
            (4, 5),    # merged 3 (bottom): raw 4-5
            (6, 6),
        ]
        fxs = [
            Fractal(idx=1, kind="top", price=16.0),
            Fractal(idx=3, kind="bottom", price=9.0),
        ]
        # raw gap = 4 - 2 - 1 = 1 < 3 → 新笔也不成笔
        strokes = strokes_from_fractals(
            df, fxs, mode="new", merged_to_raw=merged_to_raw,
        )
        assert len(strokes) == 0, "新笔也应拒绝 raw gap=1"

    def test_new_bi_condition1_no_shared_klines(self):
        """新笔条件1：顶底分型不共用K线（与旧笔相同）。

        merged gap=1 → 分型共用K线 → 不成笔，无论 raw gap 多大。
        """
        highs = [20, 16, 10, 14, 18]
        lows =  [15, 11,  5,  9, 13]
        n = len(highs)
        idx = pd.to_datetime([f"2025-06-{i+1:02d}" for i in range(n)])
        df = pd.DataFrame(
            {
                "open": [h - 1 for h in highs],
                "high": highs,
                "low": lows,
                "close": [l + 1 for l in lows],
            },
            index=idx,
        )
        # merged gap = 1 → 分型共用K线
        merged_to_raw = [
            (0, 2),     # merged 0: 3 raw bars
            (3, 8),     # merged 1 (top): 6 raw bars
            (9, 14),    # merged 2 (bottom): 6 raw bars
            (15, 20),   # merged 3
            (21, 23),   # merged 4
        ]
        fxs = [
            Fractal(idx=1, kind="top", price=16.0),
            Fractal(idx=2, kind="bottom", price=5.0),
        ]
        # merged gap = 2 - 1 = 1 → 分型 bars 重叠 (top uses 0,1,2; bottom uses 1,2,3)
        # 即使 raw gap 很大，条件1不满足 → 不成笔
        strokes = strokes_from_fractals(
            df, fxs, mode="new", merged_to_raw=merged_to_raw,
        )
        assert len(strokes) == 0, "分型共用K线时新笔也不成笔"

    def test_new_bi_fallback_without_merged_to_raw(self):
        """mode='new' 但未传 merged_to_raw → 回退到旧笔逻辑。"""
        df = _make_merged()
        fxs = [
            Fractal(idx=4, kind="bottom", price=7.0),
            Fractal(idx=9, kind="top", price=22.0),
        ]
        # merged gap = 5 >= 4 → 旧笔成笔，新笔回退也应成笔
        strokes = strokes_from_fractals(df, fxs, mode="new")
        assert len(strokes) == 1
