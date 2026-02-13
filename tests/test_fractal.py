"""分型识别 — 单元测试 (Step9)

覆盖 docs/chan_spec.md §3 全部规则：
  A) 顶分型样本（双条件：high 居中最高 AND low 居中最高）
  B) 底分型样本（双条件：low 居中最低 AND high 居中最低）
  C) 边界：第一根和最后一根不可能是分型
  D) 断言集成：assert_fractal_definition 对合法分型返回 ok=True
"""

from __future__ import annotations

import pandas as pd
import pytest

from newchan.a_fractal import Fractal, fractals_from_merged
from newchan.a_assertions import assert_fractal_definition


# =====================================================================
# A) 顶分型样本
# =====================================================================
#
# 5 根 merged K 线（已经过包含处理，无相邻包含）：
#
#   i=0: H=10, L=5
#   i=1: H=12, L=7     ← 上升
#   i=2: H=15, L=10    ← 顶分型！ high=15 最大, low=10 最大
#   i=3: H=13, L=8     ← 下降
#   i=4: H=11, L=6
#
# i=2 满足: 15>12, 15>13, 10>7, 10>8 → top, price=15

def _make_top_df() -> pd.DataFrame:
    idx = pd.to_datetime([
        "2025-01-01", "2025-01-02", "2025-01-03",
        "2025-01-04", "2025-01-05",
    ])
    return pd.DataFrame({
        "open":  [6, 8, 11, 9, 7],
        "high":  [10, 12, 15, 13, 11],
        "low":   [5, 7, 10, 8, 6],
        "close": [9, 11, 14, 12, 10],
    }, index=idx)


class TestTopFractal:
    """A) 顶分型识别。"""

    @pytest.fixture(autouse=True)
    def setup(self):
        self.df = _make_top_df()
        self.fractals = fractals_from_merged(self.df)

    def test_finds_one_top(self):
        assert len(self.fractals) == 1

    def test_kind_is_top(self):
        assert self.fractals[0].kind == "top"

    def test_idx_is_2(self):
        assert self.fractals[0].idx == 2

    def test_price_is_high(self):
        assert self.fractals[0].price == 15.0


# =====================================================================
# B) 底分型样本
# =====================================================================
#
# 5 根 merged K 线：
#
#   i=0: H=15, L=10
#   i=1: H=13, L=8     ← 下降
#   i=2: H=11, L=5     ← 底分型！ low=5 最小, high=11 最小
#   i=3: H=13, L=7     ← 上升
#   i=4: H=15, L=9
#
# i=2 满足: 5<8, 5<7, 11<13, 11<13 → bottom, price=5

def _make_bottom_df() -> pd.DataFrame:
    idx = pd.to_datetime([
        "2025-02-01", "2025-02-02", "2025-02-03",
        "2025-02-04", "2025-02-05",
    ])
    return pd.DataFrame({
        "open":  [12, 10, 7, 9, 11],
        "high":  [15, 13, 11, 13, 15],
        "low":   [10, 8, 5, 7, 9],
        "close": [14, 12, 10, 12, 14],
    }, index=idx)


class TestBottomFractal:
    """B) 底分型识别。"""

    @pytest.fixture(autouse=True)
    def setup(self):
        self.df = _make_bottom_df()
        self.fractals = fractals_from_merged(self.df)

    def test_finds_one_bottom(self):
        assert len(self.fractals) == 1

    def test_kind_is_bottom(self):
        assert self.fractals[0].kind == "bottom"

    def test_idx_is_2(self):
        assert self.fractals[0].idx == 2

    def test_price_is_low(self):
        assert self.fractals[0].price == 5.0


# =====================================================================
# C) 边界：首尾不产生分型 + 少于 3 根无分型
# =====================================================================

class TestBoundary:
    """C) 第一根和最后一根不可能是分型；少于 3 根返回空。"""

    def test_no_fractal_at_index_0_or_last(self):
        """即使首尾数值极端，也不会被识别为分型。"""
        df = pd.DataFrame({
            "open":  [1, 5, 3, 5, 1],
            "high":  [99, 10, 8, 10, 99],  # 首尾 high 最大
            "low":   [0, 5, 3, 5, 0],      # 首尾 low 最小
            "close": [50, 8, 6, 8, 50],
        })
        fxs = fractals_from_merged(df)
        idxs = [f.idx for f in fxs]
        assert 0 not in idxs, "第一根不应是分型"
        assert len(df) - 1 not in idxs, "最后一根不应是分型"

    def test_less_than_3_bars(self):
        """少于 3 根 merged bar 时返回空列表。"""
        df2 = pd.DataFrame({"high": [10, 12], "low": [5, 7]})
        assert fractals_from_merged(df2) == []
        df1 = pd.DataFrame({"high": [10], "low": [5]})
        assert fractals_from_merged(df1) == []
        df0 = pd.DataFrame(columns=["high", "low"])
        assert fractals_from_merged(df0) == []

    def test_monotonic_no_fractal(self):
        """严格单调序列（如逐步升高），不会产生任何分型。"""
        df = pd.DataFrame({
            "open":  [1, 3, 5, 7, 9],
            "high":  [2, 4, 6, 8, 10],
            "low":   [1, 3, 5, 7, 9],
            "close": [2, 4, 6, 8, 10],
        })
        assert fractals_from_merged(df) == []

    def test_multiple_fractals_sorted(self):
        """含多个分型时，返回列表按 idx 递增排序。"""
        # 构造: bottom at i=1, top at i=3
        #   i=0: H=15, L=10
        #   i=1: H=11, L=5   ← bottom (11<15, 11<13, 5<10, 5<7)
        #   i=2: H=13, L=7
        #   i=3: H=16, L=11  ← top (16>13, 16>14, 11>7, 11>9)
        #   i=4: H=14, L=9
        df = pd.DataFrame({
            "open":  [12, 7, 9, 13, 10],
            "high":  [15, 11, 13, 16, 14],
            "low":   [10, 5, 7, 11, 9],
            "close": [14, 10, 12, 15, 13],
        })
        fxs = fractals_from_merged(df)
        assert len(fxs) == 2
        assert fxs[0].kind == "bottom" and fxs[0].idx == 1
        assert fxs[1].kind == "top" and fxs[1].idx == 3
        # 确保按 idx 排序
        assert fxs[0].idx < fxs[1].idx


# =====================================================================
# D) 断言集成
# =====================================================================

class TestAssertIntegration:
    """D) assert_fractal_definition 对合法分型返回 ok=True。"""

    def test_top_fractal_assertion_ok(self):
        df = _make_top_df()
        fxs = fractals_from_merged(df)
        result = assert_fractal_definition(df, fxs)
        assert result.ok is True

    def test_bottom_fractal_assertion_ok(self):
        df = _make_bottom_df()
        fxs = fractals_from_merged(df)
        result = assert_fractal_definition(df, fxs)
        assert result.ok is True

    def test_bad_fractal_fails(self):
        """手工构造一个不满足双条件的 Fractal，断言应返回 ok=False。"""
        df = pd.DataFrame({
            "high": [10, 12, 11, 13, 14],
            "low":  [5, 7, 8, 6, 9],
        })
        # i=2: high=11 不满足 11>12（左边更高）→ 不是合法顶分型
        bad_fx = [Fractal(idx=2, kind="top", price=11.0)]
        result = assert_fractal_definition(df, bad_fx)
        assert result.ok is False
        assert "fails double condition" in result.message

    def test_empty_fractals_ok(self):
        """空分型列表 → ok。"""
        df = pd.DataFrame({"high": [10, 12], "low": [5, 7]})
        result = assert_fractal_definition(df, [])
        assert result.ok is True
