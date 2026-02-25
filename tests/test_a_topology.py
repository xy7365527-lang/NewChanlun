"""拓扑不变量与转换函数测试（195号谱系）。"""

import pytest
import pandas as pd
import numpy as np

from newchan.a_topology import (
    DecompositionFingerprint,
    StructuralDelta,
    TransitionResult,
    compute_fingerprint,
    compute_structural_delta,
    compute_transition,
    gauge_equivalence_report,
)


# ---------------------------------------------------------------------------
# 测试数据
# ---------------------------------------------------------------------------

def _make_ohlc(n=200, seed=42):
    """生成足够长的随机 OHLC 数据（保证能产生笔/线段/中枢）。"""
    rng = np.random.RandomState(seed)
    close = 100.0 + np.cumsum(rng.randn(n) * 0.5)
    high = close + rng.uniform(0.1, 1.0, n)
    low = close - rng.uniform(0.1, 1.0, n)
    opn = close + rng.uniform(-0.3, 0.3, n)
    dates = pd.date_range("2024-01-01", periods=n, freq="h")
    return pd.DataFrame(
        {"open": opn, "high": high, "low": low, "close": close},
        index=dates,
    )


@pytest.fixture
def df_raw():
    return _make_ohlc(300, seed=42)


@pytest.fixture
def df_raw_short():
    return _make_ohlc(50, seed=99)


# ---------------------------------------------------------------------------
# DecompositionFingerprint 测试
# ---------------------------------------------------------------------------

class TestDecompositionFingerprint:
    def test_fingerprint_immutable(self):
        fp = DecompositionFingerprint(
            n_centers=2,
            center_zd_zg_pairs=((99.0, 101.0), (100.0, 102.0)),
            trend_kinds=("up_trend",),
            n_strokes=10,
            n_segments=5,
            n_trends=1,
            max_level=1,
        )
        with pytest.raises(AttributeError):
            fp.n_centers = 3  # type: ignore[misc]

    def test_fingerprint_from_pipeline(self, df_raw):
        from newchan.a_topology import _run_pipeline
        strokes, segments, centers, trends, rec_levels = _run_pipeline(df_raw, "wide")
        fp = compute_fingerprint(strokes, segments, centers, trends, rec_levels)
        assert isinstance(fp, DecompositionFingerprint)
        assert fp.n_strokes == len(strokes)
        assert fp.n_segments == len(segments)
        assert fp.n_centers == len(centers)
        assert fp.n_trends == len(trends)


# ---------------------------------------------------------------------------
# StructuralDelta 测试
# ---------------------------------------------------------------------------

class TestStructuralDelta:
    def test_same_fingerprint_zero_delta(self):
        fp = DecompositionFingerprint(
            n_centers=1,
            center_zd_zg_pairs=((99.0, 101.0),),
            trend_kinds=("consolidation",),
            n_strokes=8,
            n_segments=4,
            n_trends=1,
            max_level=1,
        )
        delta = compute_structural_delta(fp, fp)
        assert delta.stroke_count_diff == 0
        assert delta.segment_count_diff == 0
        assert delta.center_count_diff == 0
        assert delta.level_diff == 0
        assert len(delta.trend_kind_mutations) == 0

    def test_delta_detects_differences(self):
        fp_a = DecompositionFingerprint(
            n_centers=2,
            center_zd_zg_pairs=((99.0, 101.0), (100.0, 102.0)),
            trend_kinds=("up_trend",),
            n_strokes=10,
            n_segments=5,
            n_trends=1,
            max_level=1,
        )
        fp_b = DecompositionFingerprint(
            n_centers=3,
            center_zd_zg_pairs=((99.0, 101.0), (100.5, 102.5), (101.0, 103.0)),
            trend_kinds=("consolidation",),
            n_strokes=12,
            n_segments=6,
            n_trends=1,
            max_level=2,
        )
        delta = compute_structural_delta(fp_a, fp_b)
        assert delta.stroke_count_diff == 2
        assert delta.segment_count_diff == 1
        assert delta.center_count_diff == 1
        assert delta.level_diff == 1
        # 第二个中枢区间变化
        assert len(delta.center_interval_diffs) == 2  # min(2, 3) = 2
        # 走势类型突变
        assert len(delta.trend_kind_mutations) == 1
        assert delta.trend_kind_mutations[0] == ("up_trend", "consolidation")


# ---------------------------------------------------------------------------
# TransitionResult 测试
# ---------------------------------------------------------------------------

class TestTransition:
    def test_transition_same_mode(self, df_raw):
        """同一模式的转换：所有差异应为零。"""
        tr = compute_transition(df_raw, "wide", "wide")
        assert isinstance(tr, TransitionResult)
        assert tr.delta.stroke_count_diff == 0
        assert tr.delta.segment_count_diff == 0
        assert tr.delta.center_count_diff == 0
        assert tr.delta.level_diff == 0
        assert all(tr.strong_invariants_preserved.values())
        assert all(tr.weak_invariants_preserved.values())

    def test_transition_wide_vs_strict(self, df_raw):
        """wide vs strict：弱不变量可能变化，结构差异有结构。"""
        tr = compute_transition(df_raw, "wide", "strict")
        assert isinstance(tr, TransitionResult)
        assert isinstance(tr.delta, StructuralDelta)
        # 不断言具体值——只断言结构完整性
        assert tr.source_mode == "wide"
        assert tr.target_mode == "strict"


# ---------------------------------------------------------------------------
# gauge_equivalence_report 测试
# ---------------------------------------------------------------------------

class TestGaugeEquivalence:
    def test_report_structure(self, df_raw):
        report = gauge_equivalence_report(df_raw, modes=("wide", "strict"))
        assert report["modes"] == ["wide", "strict"]
        assert report["n_transitions"] == 1
        assert len(report["transitions"]) == 1
        assert "strong_invariant_summary" in report
        for k in ("n_centers", "center_zd_zg_pairs", "trend_kinds"):
            assert k in report["strong_invariant_summary"]

    def test_report_three_modes(self, df_raw):
        """三模式报告应有 3 对转换。"""
        report = gauge_equivalence_report(df_raw, modes=("wide", "strict", "new"))
        assert report["n_transitions"] == 3

    def test_report_with_tolerance(self, df_raw):
        """带容差的报告。"""
        report = gauge_equivalence_report(
            df_raw, modes=("wide", "strict"), tolerance=1.0,
        )
        assert report["n_transitions"] == 1


class TestCenterIntervalsInvariant:
    def test_center_intervals_across_modes(self, df_raw):
        """验证中枢 ZD/ZG 在不同笔模式下的稳定性（经验性检验）。

        这个测试不断言 ZD/ZG 完全相等——它记录偏差幅度，
        为强/弱不变量的分类提供经验数据。
        """
        tr = compute_transition(df_raw, "wide", "strict")
        if tr.source_fp.n_centers > 0 and tr.target_fp.n_centers > 0:
            # 记录差异——不断言保持，因为这是待验证的假设
            assert isinstance(tr.delta.center_interval_diffs, tuple)
