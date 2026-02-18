"""比价K线端到端管线测试。

验证比价K线可以直接输入已有缠论管线，产出完整走势结构。
这是 ratio_relation_v1.md §1.3 IR-3（完备性）的验证。

概念溯源：[旧缠论] 第9课 — 比价变动构成独立买卖系统
"""

from __future__ import annotations

import pandas as pd
import pytest

from newchan.equivalence import make_ratio_kline
from newchan.a_inclusion import merge_inclusion
from newchan.a_fractal import fractals_from_merged
from newchan.a_stroke import strokes_from_fractals
from newchan.a_segment_v1 import segments_from_strokes_v1


def _make_wave(
    base: float,
    amplitude: float,
    periods: int,
    waves: int,
    start: str = "2024-01-01",
) -> pd.DataFrame:
    """生成正弦波 OHLCV 数据。"""
    import numpy as np

    n = periods * waves
    t = np.linspace(0, 2 * np.pi * waves, n)
    closes = base + amplitude * np.sin(t)
    idx = pd.date_range(start, periods=n, freq="h")
    noise = np.random.RandomState(42).uniform(-0.5, 0.5, n)
    return pd.DataFrame(
        {
            "open": closes + noise * 0.3,
            "high": closes + abs(noise) + 0.5,
            "low": closes - abs(noise) - 0.5,
            "close": closes,
            "volume": [1000] * n,
        },
        index=idx,
    )


def _run_pipeline(df: pd.DataFrame):
    """跑完整管线到线段，返回中间结果。"""
    df_merged, _m2r = merge_inclusion(df)
    fxs = fractals_from_merged(df_merged)
    strokes = strokes_from_fractals(df_merged, fxs)
    segs = segments_from_strokes_v1(strokes)
    return df_merged, fxs, strokes, segs


class TestRatioPipelineE2E:
    """比价K线 → 完整管线 → 走势结构。"""

    def test_ratio_produces_fractals(self):
        """比价K线能产出分型。"""
        df_a = _make_wave(base=100, amplitude=10, periods=50, waves=4)
        df_b = _make_wave(base=50, amplitude=3, periods=50, waves=3)
        ratio = make_ratio_kline(df_a, df_b)

        _, fxs, _, _ = _run_pipeline(ratio)
        assert len(fxs) > 0, "比价K线应产出至少一个分型"

    def test_ratio_produces_strokes(self):
        """比价K线能产出笔。"""
        df_a = _make_wave(base=100, amplitude=15, periods=60, waves=5)
        df_b = _make_wave(base=80, amplitude=5, periods=60, waves=3)
        ratio = make_ratio_kline(df_a, df_b)

        _, _, strokes, _ = _run_pipeline(ratio)
        assert len(strokes) >= 3, "比价K线应产出至少3笔（走势必完美最小要求）"

    def test_ratio_produces_segments(self):
        """比价K线能产出线段。"""
        df_a = _make_wave(base=100, amplitude=20, periods=80, waves=8)
        df_b = _make_wave(base=60, amplitude=8, periods=80, waves=5)
        ratio = make_ratio_kline(df_a, df_b)

        _, _, _, segs = _run_pipeline(ratio)
        assert len(segs) >= 1, "比价K线应产出至少1条线段"

    def test_symmetry_structure(self):
        """IR-1 结构验证：A/B 和 B/A 的笔数应相近。"""
        df_a = _make_wave(base=100, amplitude=15, periods=60, waves=5)
        df_b = _make_wave(base=80, amplitude=5, periods=60, waves=3)

        ratio_ab = make_ratio_kline(df_a, df_b)
        ratio_ba = make_ratio_kline(df_b, df_a)

        def _count_strokes(df: pd.DataFrame) -> int:
            _, _, strokes, _ = _run_pipeline(df)
            return len(strokes)

        n_ab = _count_strokes(ratio_ab)
        n_ba = _count_strokes(ratio_ba)
        # 笔数不一定完全相同（因为包含处理的非线性），但应接近
        assert abs(n_ab - n_ba) <= max(n_ab, n_ba) * 0.3, (
            f"A/B ({n_ab}笔) 和 B/A ({n_ba}笔) 结构差异过大"
        )
