"""比价分析调度器（RatioEngine）测试。

概念溯源：[旧缠论] 第9/72/73课 — 比价关系为三个独立系统之一
         [新缠论] 等价关系严格定义 — ratio_relation_v1.md §2
"""

from __future__ import annotations

import pandas as pd
import pytest

from newchan.equivalence import EquivalencePair
from newchan.ratio_engine import (
    RatioAnalysis,
    RatioAnalysisError,
    analyze_batch,
    analyze_pair,
)


# ── 测试数据工厂 ─────────────────────────────────────────


def _ohlcv(prices: list[float], start: str = "2024-01-01") -> pd.DataFrame:
    """从 close 列表生成简单 OHLCV（open=close, high=close+1, low=close-1）。"""
    n = len(prices)
    idx = pd.date_range(start, periods=n, freq="D")
    return pd.DataFrame(
        {
            "open": prices,
            "high": [p + 1 for p in prices],
            "low": [p - 1 for p in prices],
            "close": prices,
            "volume": [1000] * n,
        },
        index=idx,
    )


def _trending_ohlcv(
    n: int = 30,
    start_price: float = 100.0,
    step: float = 2.0,
    volatility: float = 3.0,
    phase_offset: int = 0,
    start: str = "2024-01-01",
) -> pd.DataFrame:
    """生成带趋势和波动的 OHLCV 数据，能产生分型/笔/线段。

    Parameters
    ----------
    phase_offset : int
        波动相位偏移，用于让 A/B 产生不同的波动模式以避免退化比价。
    """
    idx = pd.date_range(start, periods=n, freq="D")
    closes = []
    for i in range(n):
        # 锯齿形波动：奇数 step 上涨，偶数 step 回调
        j = (i + phase_offset) % 4
        base = start_price + i * step
        if j < 2:
            closes.append(base + volatility * j)
        else:
            closes.append(base - volatility * (j - 2))

    highs = [c + volatility for c in closes]
    lows = [c - volatility for c in closes]
    opens = [closes[0]] + closes[:-1]

    return pd.DataFrame(
        {
            "open": opens,
            "high": highs,
            "low": lows,
            "close": closes,
            "volume": [1000] * n,
        },
        index=idx,
    )


# ── RatioAnalysis / RatioAnalysisError 不可变性 ──────────


class TestDataclassImmutable:
    def test_ratio_analysis_error_frozen(self):
        pair = EquivalencePair(sym_a="A", sym_b="B")
        err = RatioAnalysisError(pair=pair, reason="test")
        with pytest.raises(AttributeError):
            err.reason = "changed"  # type: ignore[misc]

    def test_ratio_analysis_error_fields(self):
        pair = EquivalencePair(sym_a="GLD", sym_b="SLV")
        err = RatioAnalysisError(pair=pair, reason="no overlap")
        assert err.pair is pair
        assert err.reason == "no overlap"


# ── analyze_pair 验证失败 → RatioAnalysisError ───────────


class TestAnalyzePairValidationFail:
    def test_no_overlap_returns_error(self):
        """时间窗口不重叠 → RatioAnalysisError。"""
        pair = EquivalencePair(sym_a="A", sym_b="B")
        df_a = _ohlcv([100, 102], start="2024-01-01")
        df_b = _ohlcv([50, 51], start="2025-01-01")
        result = analyze_pair(pair, df_a, df_b)
        assert isinstance(result, RatioAnalysisError)
        assert "overlap" in result.reason.lower()

    def test_zero_price_returns_error(self):
        """B 含零价格 → RatioAnalysisError。"""
        pair = EquivalencePair(sym_a="A", sym_b="B")
        df_a = _ohlcv([100, 102, 101, 105, 103])
        df_b = _ohlcv([50, 0, 49, 52, 50])
        result = analyze_pair(pair, df_a, df_b)
        assert isinstance(result, RatioAnalysisError)

    def test_constant_ratio_returns_error(self):
        """常数比价（退化）→ RatioAnalysisError。"""
        pair = EquivalencePair(sym_a="A", sym_b="B")
        df_a = _ohlcv([100, 200, 300, 400, 500])
        df_b = _ohlcv([50, 100, 150, 200, 250])  # 完美2x
        result = analyze_pair(pair, df_a, df_b)
        assert isinstance(result, RatioAnalysisError)


# ── analyze_pair 正常流程 → RatioAnalysis ────────────────


class TestAnalyzePairSuccess:
    def test_valid_pair_returns_analysis(self):
        """有效等价对 → RatioAnalysis，包含所有管线产物。"""
        pair = EquivalencePair(sym_a="A", sym_b="B")
        df_a = _trending_ohlcv(n=30, start_price=100, step=2, volatility=3)
        df_b = _trending_ohlcv(n=30, start_price=50, step=1, volatility=1.5, phase_offset=2)
        result = analyze_pair(pair, df_a, df_b)
        assert isinstance(result, RatioAnalysis)
        assert result.pair is pair

    def test_result_has_ratio_kline(self):
        """结果包含比价K线 DataFrame。"""
        pair = EquivalencePair(sym_a="A", sym_b="B")
        df_a = _trending_ohlcv(n=30, start_price=100, step=2, volatility=3)
        df_b = _trending_ohlcv(n=30, start_price=50, step=1, volatility=1.5, phase_offset=2)
        result = analyze_pair(pair, df_a, df_b)
        assert isinstance(result, RatioAnalysis)
        assert isinstance(result.ratio_kline, pd.DataFrame)
        assert len(result.ratio_kline) > 0
        for col in ["open", "high", "low", "close"]:
            assert col in result.ratio_kline.columns

    def test_result_fractals_are_list(self):
        """fractals 字段是列表。"""
        pair = EquivalencePair(sym_a="A", sym_b="B")
        df_a = _trending_ohlcv(n=30, start_price=100, step=2, volatility=3)
        df_b = _trending_ohlcv(n=30, start_price=50, step=1, volatility=1.5, phase_offset=2)
        result = analyze_pair(pair, df_a, df_b)
        assert isinstance(result, RatioAnalysis)
        assert isinstance(result.fractals, list)

    def test_result_strokes_are_list(self):
        """strokes 字段是列表。"""
        pair = EquivalencePair(sym_a="A", sym_b="B")
        df_a = _trending_ohlcv(n=30, start_price=100, step=2, volatility=3)
        df_b = _trending_ohlcv(n=30, start_price=50, step=1, volatility=1.5, phase_offset=2)
        result = analyze_pair(pair, df_a, df_b)
        assert isinstance(result, RatioAnalysis)
        assert isinstance(result.strokes, list)

    def test_result_segments_are_list(self):
        """segments 字段是列表。"""
        pair = EquivalencePair(sym_a="A", sym_b="B")
        df_a = _trending_ohlcv(n=30, start_price=100, step=2, volatility=3)
        df_b = _trending_ohlcv(n=30, start_price=50, step=1, volatility=1.5, phase_offset=2)
        result = analyze_pair(pair, df_a, df_b)
        assert isinstance(result, RatioAnalysis)
        assert isinstance(result.segments, list)


# ── analyze_pair 管线异常 → RatioAnalysisError ───────────


class TestAnalyzePairPipelineError:
    def test_empty_data_returns_error(self):
        """空数据 → RatioAnalysisError（管线无法产出）。"""
        pair = EquivalencePair(sym_a="A", sym_b="B")
        df_a = _ohlcv([])
        df_b = _ohlcv([])
        result = analyze_pair(pair, df_a, df_b)
        assert isinstance(result, RatioAnalysisError)


# ── analyze_batch ────────────────────────────────────────


class TestAnalyzeBatch:
    def test_empty_batch(self):
        """空列表 → 空列表。"""
        result = analyze_batch([])
        assert result == []

    def test_batch_preserves_order(self):
        """批量结果与输入顺序一致。"""
        pair1 = EquivalencePair(sym_a="A1", sym_b="B1")
        pair2 = EquivalencePair(sym_a="A2", sym_b="B2")
        df_a = _trending_ohlcv(n=30, start_price=100, step=2, volatility=3)
        df_b = _trending_ohlcv(n=30, start_price=50, step=1, volatility=1.5, phase_offset=2)
        items = [
            (pair1, df_a, df_b),
            (pair2, df_a, df_b),
        ]
        results = analyze_batch(items)
        assert len(results) == 2
        assert results[0].pair is pair1  # type: ignore[union-attr]
        assert results[1].pair is pair2  # type: ignore[union-attr]

    def test_batch_mixed_results(self):
        """批量中可以混合成功和失败结果。"""
        pair_ok = EquivalencePair(sym_a="OK_A", sym_b="OK_B")
        pair_bad = EquivalencePair(sym_a="BAD_A", sym_b="BAD_B")
        df_a_ok = _trending_ohlcv(n=30, start_price=100, step=2, volatility=3)
        df_b_ok = _trending_ohlcv(n=30, start_price=50, step=1, volatility=1.5)
        df_a_bad = _ohlcv([100, 102], start="2024-01-01")
        df_b_bad = _ohlcv([50, 51], start="2025-01-01")  # 不重叠
        items = [
            (pair_ok, df_a_ok, df_b_ok),
            (pair_bad, df_a_bad, df_b_bad),
        ]
        results = analyze_batch(items)
        assert len(results) == 2
        # 第一个成功或者至少有 pair 字段
        assert results[0].pair is pair_ok  # type: ignore[union-attr]
        # 第二个失败
        assert isinstance(results[1], RatioAnalysisError)

    def test_batch_single_item(self):
        """批量单元素 → 单结果列表。"""
        pair = EquivalencePair(sym_a="A", sym_b="B")
        df_a = _trending_ohlcv(n=30, start_price=100, step=2, volatility=3)
        df_b = _trending_ohlcv(n=30, start_price=50, step=1, volatility=1.5, phase_offset=2)
        results = analyze_batch([(pair, df_a, df_b)])
        assert len(results) == 1
