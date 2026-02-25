"""拓扑不变量与转换函数测试（195号谱系 + Layer 1 共识）。"""

import pytest
import pandas as pd
import numpy as np

from newchan.a_topology import (
    DecompositionFingerprint,
    StructuralDelta,
    TransitionResult,
    T7Result,
    T5Result,
    T8Result,
    T6Result,
    centers_to_barcode,
    barcode_to_diagram,
    compute_beta1_tau,
    compute_fingerprint,
    compute_structural_delta,
    compute_transition,
    gauge_equivalence_report,
    _bottleneck_distance,
    _check_barcode_inclusion,
    _w1_norm,
    _normalize_barcode,
    check_divergence_topology,
    check_recursive_barcode_order,
    check_trend_move_equivalence,
    check_cross_level_leray,
    _bar_length_distribution,
    _kl_divergence,
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
# TDA 桥接层测试
# ---------------------------------------------------------------------------

class TestTDABridge:
    def test_bottleneck_empty_diagrams(self):
        """空条形码之间的 bottleneck 距离应为 0。"""
        empty = np.empty((0, 2), dtype=np.float64)
        assert _bottleneck_distance(empty, empty) == 0.0

    def test_bottleneck_identical_diagrams(self):
        """相同条形码之间的 bottleneck 距离应为 0。"""
        dgm = np.array([[1.0, 3.0], [2.0, 5.0]], dtype=np.float64)
        assert _bottleneck_distance(dgm, dgm) == 0.0

    def test_bottleneck_different_diagrams(self):
        """不同条形码之间的 bottleneck 距离应 > 0。"""
        dgm_a = np.array([[1.0, 3.0]], dtype=np.float64)
        dgm_b = np.array([[1.0, 5.0]], dtype=np.float64)
        dist = _bottleneck_distance(dgm_a, dgm_b)
        assert dist > 0.0

    def test_barcode_to_diagram_empty(self):
        """空条形码转为 (0,2) 数组。"""
        dgm = barcode_to_diagram(())
        assert dgm.shape == (0, 2)

    def test_barcode_to_diagram_nonempty(self):
        """非空条形码转为正确形状的数组。"""
        bc = ((99.0, 101.0), (100.0, 102.0))
        dgm = barcode_to_diagram(bc)
        assert dgm.shape == (2, 2)
        assert dgm[0, 0] == 99.0
        assert dgm[1, 1] == 102.0


# ---------------------------------------------------------------------------
# beta1_tau 测试（共识修正#2）
# ---------------------------------------------------------------------------

class TestBeta1Tau:
    def test_beta1_tau_zero_threshold(self):
        """τ=0 时 β₁^τ = 所有 bar 的数量。"""
        bc = ((99.0, 101.0), (100.0, 105.0))
        assert compute_beta1_tau(bc, tau=0.0) == 2

    def test_beta1_tau_filters_short_bars(self):
        """τ>0 时只计长寿命条带。"""
        bc = ((99.0, 101.0), (100.0, 105.0))  # 长度: 2.0, 5.0
        assert compute_beta1_tau(bc, tau=3.0) == 1  # 只有 5.0 > 3.0

    def test_beta1_tau_all_filtered(self):
        """τ 大于所有条带长度时 β₁^τ = 0。"""
        bc = ((99.0, 101.0), (100.0, 102.0))  # 长度: 2.0, 2.0
        assert compute_beta1_tau(bc, tau=5.0) == 0

    def test_beta1_tau_empty(self):
        """空条形码的 β₁^τ = 0。"""
        assert compute_beta1_tau((), tau=0.0) == 0


# ---------------------------------------------------------------------------
# DecompositionFingerprint 测试
# ---------------------------------------------------------------------------

class TestDecompositionFingerprint:
    def test_fingerprint_immutable(self):
        fp = DecompositionFingerprint(
            n_centers=2,
            center_zd_zg_pairs=((99.0, 101.0), (100.0, 102.0)),
            trend_kinds=("up_trend",),
            barcode=((99.0, 101.0), (100.0, 102.0)),
            beta1_tau=2,
            n_strokes=10,
            n_segments=5,
            n_trends=1,
            max_level=1,
        )
        with pytest.raises(AttributeError):
            fp.n_centers = 3  # type: ignore[misc]

    def test_fingerprint_has_barcode_fields(self):
        """指纹包含 barcode 和 beta1_tau 字段。"""
        fp = DecompositionFingerprint(
            n_centers=1,
            center_zd_zg_pairs=((99.0, 101.0),),
            trend_kinds=("consolidation",),
            barcode=((99.0, 101.0),),
            beta1_tau=1,
            n_strokes=8,
            n_segments=4,
            n_trends=1,
            max_level=1,
        )
        assert fp.barcode == ((99.0, 101.0),)
        assert fp.beta1_tau == 1

    def test_fingerprint_from_pipeline(self, df_raw):
        from newchan.a_topology import _run_pipeline
        strokes, segments, centers, trends, rec_levels = _run_pipeline(df_raw, "wide")
        fp = compute_fingerprint(strokes, segments, centers, trends, rec_levels)
        assert isinstance(fp, DecompositionFingerprint)
        assert fp.n_strokes == len(strokes)
        assert fp.n_segments == len(segments)
        assert fp.n_centers == len(centers)
        assert fp.n_trends == len(trends)
        # 新字段存在且一致
        assert len(fp.barcode) == len(centers)
        assert fp.beta1_tau >= 0
        # barcode 与 center_zd_zg_pairs 一致
        assert fp.barcode == fp.center_zd_zg_pairs

    def test_fingerprint_beta1_tau_with_threshold(self, df_raw):
        """tau > 0 时 beta1_tau ≤ n_centers。"""
        from newchan.a_topology import _run_pipeline
        strokes, segments, centers, trends, rec_levels = _run_pipeline(df_raw, "wide")
        fp = compute_fingerprint(strokes, segments, centers, trends, rec_levels, tau=1.0)
        assert fp.beta1_tau <= fp.n_centers


# ---------------------------------------------------------------------------
# StructuralDelta 测试
# ---------------------------------------------------------------------------

class TestStructuralDelta:
    def test_same_fingerprint_zero_delta(self):
        fp = DecompositionFingerprint(
            n_centers=1,
            center_zd_zg_pairs=((99.0, 101.0),),
            trend_kinds=("consolidation",),
            barcode=((99.0, 101.0),),
            beta1_tau=1,
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
        assert delta.bottleneck_distance == 0.0
        assert delta.beta1_tau_diff == 0

    def test_delta_detects_differences(self):
        fp_a = DecompositionFingerprint(
            n_centers=2,
            center_zd_zg_pairs=((99.0, 101.0), (100.0, 102.0)),
            trend_kinds=("up_trend",),
            barcode=((99.0, 101.0), (100.0, 102.0)),
            beta1_tau=2,
            n_strokes=10,
            n_segments=5,
            n_trends=1,
            max_level=1,
        )
        fp_b = DecompositionFingerprint(
            n_centers=3,
            center_zd_zg_pairs=((99.0, 101.0), (100.5, 102.5), (101.0, 103.0)),
            trend_kinds=("consolidation",),
            barcode=((99.0, 101.0), (100.5, 102.5), (101.0, 103.0)),
            beta1_tau=3,
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
        assert len(delta.center_interval_diffs) == 2  # min(2, 3) = 2
        assert len(delta.trend_kind_mutations) == 1
        assert delta.trend_kind_mutations[0] == ("up_trend", "consolidation")
        # 新字段
        assert delta.bottleneck_distance > 0.0
        assert delta.beta1_tau_diff == 1

    def test_delta_bottleneck_symmetric(self):
        """bottleneck 距离是对称的。"""
        fp_a = DecompositionFingerprint(
            n_centers=1,
            center_zd_zg_pairs=((99.0, 101.0),),
            trend_kinds=(),
            barcode=((99.0, 101.0),),
            beta1_tau=1,
            n_strokes=5,
            n_segments=2,
            n_trends=0,
            max_level=1,
        )
        fp_b = DecompositionFingerprint(
            n_centers=1,
            center_zd_zg_pairs=((99.0, 103.0),),
            trend_kinds=(),
            barcode=((99.0, 103.0),),
            beta1_tau=1,
            n_strokes=5,
            n_segments=2,
            n_trends=0,
            max_level=1,
        )
        d_ab = compute_structural_delta(fp_a, fp_b)
        d_ba = compute_structural_delta(fp_b, fp_a)
        assert abs(d_ab.bottleneck_distance - d_ba.bottleneck_distance) < 1e-10


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
        assert tr.delta.bottleneck_distance == 0.0
        assert tr.delta.beta1_tau_diff == 0
        assert all(tr.strong_invariants_preserved.values())
        assert all(tr.weak_invariants_preserved.values())

    def test_transition_wide_vs_strict(self, df_raw):
        """wide vs strict：结构差异有结构，含 bottleneck 距离。"""
        tr = compute_transition(df_raw, "wide", "strict")
        assert isinstance(tr, TransitionResult)
        assert isinstance(tr.delta, StructuralDelta)
        assert tr.source_mode == "wide"
        assert tr.target_mode == "strict"
        # bottleneck_distance 有值（≥ 0）
        assert tr.delta.bottleneck_distance >= 0.0
        # 强不变量报告包含新字段
        assert "beta1_tau" in tr.strong_invariants_preserved
        assert "barcode_bottleneck" in tr.strong_invariants_preserved


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
        # 新增字段检查
        for k in ("n_centers", "center_zd_zg_pairs", "trend_kinds",
                   "beta1_tau", "barcode_bottleneck"):
            assert k in report["strong_invariant_summary"]
        # tau 参数记录在报告中
        assert "tau" in report
        # transition 包含 bottleneck_distance
        t0 = report["transitions"][0]
        assert "bottleneck_distance" in t0["delta"]
        assert "beta1_tau_diff" in t0["delta"]

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

    def test_report_with_tau(self, df_raw):
        """带 τ 阈值的报告。"""
        report = gauge_equivalence_report(
            df_raw, modes=("wide", "strict"), tau=2.0,
        )
        assert report["tau"] == 2.0


class TestCenterIntervalsInvariant:
    def test_center_intervals_across_modes(self, df_raw):
        """验证中枢 ZD/ZG 在不同笔模式下的稳定性（经验性检验）。"""
        tr = compute_transition(df_raw, "wide", "strict")
        if tr.source_fp.n_centers > 0 and tr.target_fp.n_centers > 0:
            assert isinstance(tr.delta.center_interval_diffs, tuple)
            # bottleneck 距离作为更精确的稳定性度量
            assert tr.delta.bottleneck_distance >= 0.0

    def test_barcode_consistency_t3(self, df_raw):
        """T3 验证：barcode 与 center_zd_zg_pairs 一致（中枢↔条带一一对应）。"""
        tr = compute_transition(df_raw, "wide", "wide")
        # 同一模式下，barcode 应等于 center_zd_zg_pairs
        assert tr.source_fp.barcode == tr.source_fp.center_zd_zg_pairs

    def test_beta1_tau_gauge_candidate(self, df_raw):
        """β₁^τ 作为 gauge 不变量候选的经验检验。"""
        tr = compute_transition(df_raw, "wide", "strict", tau=1.0)
        # 记录差异——β₁^τ 是否在不同模式下保持
        # 不做强断言：这是待验证的假设
        assert isinstance(tr.source_fp.beta1_tau, int)
        assert isinstance(tr.target_fp.beta1_tau, int)


# ---------------------------------------------------------------------------
# T7：递归条形码偏序测试（ε-真单射匹配）
# ---------------------------------------------------------------------------

class TestBarcodeInclusion:
    def test_empty_high_always_passes(self):
        """空 bars_high 总被包含。"""
        passed, unmatched = _check_barcode_inclusion((), ((1.0, 3.0),), 1.0)
        assert passed is True
        assert unmatched == ()

    def test_empty_low_nonempty_high_fails(self):
        """非空 bars_high 不被空 bars_trimmed_low 包含。"""
        passed, unmatched = _check_barcode_inclusion(((1.0, 3.0),), (), 1.0)
        assert passed is False
        assert unmatched == ((1.0, 3.0),)

    def test_exact_match(self):
        """精确匹配通过。"""
        bars = ((1.0, 3.0), (2.0, 5.0))
        passed, unmatched = _check_barcode_inclusion(bars, bars, 0.01)
        assert passed is True
        assert unmatched == ()

    def test_epsilon_tolerance(self):
        """ε 容差内的匹配通过。"""
        high = ((1.0, 3.0),)
        low = ((1.05, 3.05),)
        passed, _ = _check_barcode_inclusion(high, low, 0.1)
        assert passed is True

    def test_outside_epsilon_fails(self):
        """超出 ε 容差不匹配。"""
        high = ((1.0, 3.0),)
        low = ((2.0, 4.0),)
        passed, unmatched = _check_barcode_inclusion(high, low, 0.5)
        assert passed is False
        assert len(unmatched) == 1

    def test_true_injective_no_double_match(self):
        """真单射：bars_trimmed_low 中每个 bar 至多被匹配一次。"""
        high = ((1.0, 3.0), (1.0, 3.0))  # 两个相同的 bar
        low = ((1.0, 3.0),)  # 只有一个
        passed, unmatched = _check_barcode_inclusion(high, low, 0.1)
        assert passed is False
        assert len(unmatched) == 1

    def test_surplus_low_bars_ok(self):
        """bars_trimmed_low 有多余 bar 不影响通过。"""
        high = ((1.0, 3.0),)
        low = ((1.0, 3.0), (5.0, 8.0), (10.0, 15.0))
        passed, _ = _check_barcode_inclusion(high, low, 0.1)
        assert passed is True

    def test_exact_match_epsilon_zero(self):
        """epsilon=0.0 时精确相同的 bar 应匹配。"""
        bars = ((1.0, 3.0),)
        passed, unmatched = _check_barcode_inclusion(bars, bars, 0.0)
        assert passed is True
        assert unmatched == ()

    def test_bipartite_matching_avoids_greedy_false_negative(self):
        """ε-邻域交叠场景：贪心会假阴性，二分匹配应通过。"""
        # Gemini 反例：H1 邻域覆盖 L1 和 L2，H2 邻域只覆盖 L1
        # 贪心 H1→L1 锁定后 H2→L2 失败；最优 H1→L2, H2→L1 通过
        high = ((1.0, 3.0), (1.08, 3.08))
        low = ((1.05, 3.05), (0.95, 2.95))
        passed, unmatched = _check_barcode_inclusion(high, low, 0.1)
        assert passed is True
        assert unmatched == ()


class TestT7RecursiveOrder:
    def test_single_level_no_check(self):
        """只有一层时 T7 无需检查，返回空列表。"""
        from newchan.a_topology import _run_pipeline
        from newchan.a_recursive_engine import build_recursive_levels
        df = _make_ohlc(50, seed=99)
        from newchan.a_inclusion import merge_inclusion
        from newchan.a_fractal import fractals_from_merged
        from newchan.a_stroke import strokes_from_fractals
        from newchan.a_segment_v1 import segments_from_strokes_v1
        df_m, m2r = merge_inclusion(df)
        fractals = fractals_from_merged(df_m)
        strokes = strokes_from_fractals(df_m, fractals, mode="wide")
        segments = segments_from_strokes_v1(strokes)
        levels = build_recursive_levels(segments)
        if len(levels) <= 1:
            results = check_recursive_barcode_order(levels)
            assert results == []

    def test_t7_from_pipeline(self, df_raw):
        """用真实数据运行管线，如果产生多层则验证 T7。"""
        from newchan.a_topology import _run_pipeline
        from newchan.a_recursive_engine import build_recursive_levels
        from newchan.a_inclusion import merge_inclusion
        from newchan.a_fractal import fractals_from_merged
        from newchan.a_stroke import strokes_from_fractals
        from newchan.a_segment_v1 import segments_from_strokes_v1
        df_m, m2r = merge_inclusion(df_raw)
        fractals = fractals_from_merged(df_m)
        strokes = strokes_from_fractals(df_m, fractals, mode="wide")
        segments = segments_from_strokes_v1(strokes)
        levels = build_recursive_levels(segments)
        if len(levels) >= 2:
            results = check_recursive_barcode_order(levels, tau=0.0, epsilon=5.0)
            assert len(results) == len(levels) - 1
            for r in results:
                assert isinstance(r, T7Result)


# ---------------------------------------------------------------------------
# T5：走势类型 ≅ 上级笔 构造不变式验证测试
# ---------------------------------------------------------------------------

class TestT5TrendMoveEquivalence:
    def test_single_level_no_check(self):
        """只有一层时 T5 无需检查，返回空列表。"""
        from newchan.a_recursive_engine import build_recursive_levels
        from newchan.a_inclusion import merge_inclusion
        from newchan.a_fractal import fractals_from_merged
        from newchan.a_stroke import strokes_from_fractals
        from newchan.a_segment_v1 import segments_from_strokes_v1
        df = _make_ohlc(50, seed=99)
        df_m, m2r = merge_inclusion(df)
        fractals = fractals_from_merged(df_m)
        strokes = strokes_from_fractals(df_m, fractals, mode="wide")
        segments = segments_from_strokes_v1(strokes)
        levels = build_recursive_levels(segments)
        if len(levels) <= 1:
            results = check_trend_move_equivalence(levels)
            assert results == []

    def test_t5_from_pipeline(self, df_raw):
        """用真实数据运行管线，验证 T5 构造不变式。"""
        from newchan.a_recursive_engine import build_recursive_levels
        from newchan.a_inclusion import merge_inclusion
        from newchan.a_fractal import fractals_from_merged
        from newchan.a_stroke import strokes_from_fractals
        from newchan.a_segment_v1 import segments_from_strokes_v1
        df_m, m2r = merge_inclusion(df_raw)
        fractals = fractals_from_merged(df_m)
        strokes = strokes_from_fractals(df_m, fractals, mode="wide")
        segments = segments_from_strokes_v1(strokes)
        levels = build_recursive_levels(segments)
        if len(levels) >= 2:
            results = check_trend_move_equivalence(levels)
            assert len(results) == len(levels) - 1
            for r in results:
                assert isinstance(r, T5Result)
                assert r.confirmed_only is True
                assert r.identity is True
                assert r.passed is True


class TestT5FailurePaths:
    def test_t5_fails_with_unconfirmed_move(self):
        """moves 中含 unconfirmed trend 时 confirmed_only 应为 False。"""
        from types import SimpleNamespace as NS
        # Mock: level 0 has one confirmed trend, level 1 moves has one unconfirmed
        trend_confirmed = NS(confirmed=True, kind="up_trend")
        trend_unconfirmed = NS(confirmed=False, kind="down_trend")
        level_0 = NS(level=0, trends=[trend_confirmed], centers=[], moves=[])
        level_1 = NS(level=1, trends=[], centers=[], moves=[trend_unconfirmed])
        results = check_trend_move_equivalence([level_0, level_1])
        assert len(results) == 1
        assert results[0].confirmed_only is False
        assert results[0].passed is False

    def test_t5_fails_with_identity_mismatch(self):
        """moves 与 confirmed_trends 引用不同时 identity 应为 False。"""
        from types import SimpleNamespace as NS
        trend_a = NS(confirmed=True, kind="up_trend")
        trend_b = NS(confirmed=True, kind="up_trend")  # 不同对象
        level_0 = NS(level=0, trends=[trend_a], centers=[], moves=[])
        level_1 = NS(level=1, trends=[], centers=[], moves=[trend_b])
        results = check_trend_move_equivalence([level_0, level_1])
        assert len(results) == 1
        assert results[0].confirmed_only is True
        assert results[0].identity is False  # trend_b is not trend_a
        assert results[0].passed is False


# ---------------------------------------------------------------------------
# T8：背驰拓扑后验验证测试（Layer 2）
# ---------------------------------------------------------------------------

class TestT8DivergenceTopology:
    def test_t8_w1_norm_basic(self):
        """W₁ 范数：Σ|d-b|/2 求和。"""
        bc = ((1.0, 3.0), (2.0, 6.0))
        # |3-1|/2 + |6-2|/2 = 1.0 + 2.0 = 3.0
        assert _w1_norm(bc) == pytest.approx(3.0)

    def test_t8_w1_norm_empty(self):
        """空条形码 → W₁ = 0.0。"""
        assert _w1_norm(()) == 0.0

    def test_t8_w1_norm_tau_trim(self):
        """tau-trim 过滤短条带后计算 W₁。"""
        bc = ((1.0, 3.0), (2.0, 2.5), (0.0, 10.0))
        # 无 trim: |3-1|/2 + |2.5-2|/2 + |10-0|/2 = 1.0 + 0.25 + 5.0 = 6.25
        assert _w1_norm(bc, tau=0.0) == pytest.approx(6.25)
        # tau=1.0: 过滤 |2.5-2|=0.5 <= 1.0 → 1.0 + 5.0 = 6.0
        assert _w1_norm(bc, tau=1.0) == pytest.approx(6.0)
        # tau=3.0: 过滤 |3-1|=2.0 和 |2.5-2|=0.5 → 只剩 |10-0|/2 = 5.0
        assert _w1_norm(bc, tau=3.0) == pytest.approx(5.0)

    def test_t8_normalize_barcode(self):
        """仿射规范化将条形码映射到 [0, 1] 区间。"""
        bc = ((100.0, 104.0), (102.0, 106.0))
        normalized = _normalize_barcode(bc, price_low=100.0, price_high=110.0)
        # span = 10, (100-100)/10=0.0, (104-100)/10=0.4, (102-100)/10=0.2, (106-100)/10=0.6
        assert normalized[0] == pytest.approx((0.0, 0.4))
        assert normalized[1] == pytest.approx((0.2, 0.6))

    def test_t8_normalize_barcode_zero_span(self):
        """span=0 退化情况返回原始条形码。"""
        bc = ((5.0, 5.0),)
        result = _normalize_barcode(bc, price_low=5.0, price_high=5.0)
        assert result == bc

    def test_t8_synthetic_divergence(self):
        """合成背驰：W₁(C) < W₁(A) → passed=True。"""
        from types import SimpleNamespace as NS

        # 构造 segments: 10 个线段
        segments = [
            NS(s0=i, s1=i, i0=i*10, i1=(i+1)*10, direction="up",
               high=100.0 + i * 2, low=98.0 + i * 2)
            for i in range(10)
        ]
        # A 段中枢（宽带——高持续量）
        center_a = NS(seg0=1, seg1=3, low=99.0, high=103.0,
                       kind="settled", confirmed=True, sustain=0,
                       direction="up", gg=103.0, dd=99.0, g=100.0, d=101.0,
                       development="", level_id=0, terminated=False)
        # C 段中枢（窄带——低持续量 = 力竭信号）
        center_c = NS(seg0=7, seg1=9, low=112.0, high=113.0,
                       kind="settled", confirmed=True, sustain=0,
                       direction="up", gg=113.0, dd=112.0, g=112.5, d=112.5,
                       development="", level_id=0, terminated=False)
        centers = [center_a, center_c]

        # 构造一个 Divergence
        div = NS(kind="trend", direction="top", level_id=0,
                 seg_a_start=0, seg_a_end=4, seg_c_start=6, seg_c_end=9,
                 center_idx=1, force_a=100.0, force_c=50.0, confirmed=True)

        results = check_divergence_topology(
            [div], segments, centers, eta=0.0, normalize=False,
        )
        assert len(results) == 1
        r = results[0]
        assert isinstance(r, T8Result)
        assert r.inconclusive is False
        assert r.n_centers_a == 1
        assert r.n_centers_c == 1
        # A 段中枢 [99,103] → |103-99|/2 = 2.0
        assert r.w1_a == pytest.approx(2.0)
        # C 段中枢 [112,113] → |113-112|/2 = 0.5
        assert r.w1_c == pytest.approx(0.5)
        assert r.passed is True
        assert r.w1_drop == pytest.approx(1.5)

    def test_t8_inconclusive(self):
        """A/C 段无中枢且无 strokes → inconclusive=True。"""
        from types import SimpleNamespace as NS

        segments = [
            NS(s0=i, s1=i, i0=i*10, i1=(i+1)*10, direction="up",
               high=100.0, low=98.0)
            for i in range(10)
        ]
        div = NS(kind="trend", direction="top", level_id=0,
                 seg_a_start=0, seg_a_end=3, seg_c_start=6, seg_c_end=9,
                 center_idx=0, force_a=100.0, force_c=50.0, confirmed=True)

        results = check_divergence_topology(
            [div], segments, [], eta=0.0,  # 空中枢列表，无 strokes
        )
        assert len(results) == 1
        assert results[0].inconclusive is True
        assert results[0].passed is False

    def test_t8_stroke_fallback(self):
        """A/C 段无中枢但有 strokes → 用笔振荡构造条形码（208号谱系）。"""
        from types import SimpleNamespace as NS

        # 构造 strokes：A 段笔振荡大，C 段笔振荡小 → 背驰
        strokes = []
        # A 段 strokes (index 0..9): 大振荡
        for i in range(10):
            strokes.append(NS(low=90.0 + i, high=100.0 + i, direction="up"))
        # B 段 strokes (index 10..19): 中间
        for i in range(10):
            strokes.append(NS(low=95.0, high=105.0, direction="up"))
        # C 段 strokes (index 20..22): 小振荡
        for i in range(3):
            strokes.append(NS(low=98.0, high=100.0, direction="up"))

        # segments: A=[seg0], C=[seg2]
        segments = [
            NS(s0=0, s1=9, i0=0, i1=100, direction="up", high=109.0, low=90.0),
            NS(s0=10, s1=19, i0=100, i1=200, direction="up", high=105.0, low=95.0),
            NS(s0=20, s1=22, i0=200, i1=230, direction="up", high=100.0, low=98.0),
        ]
        div = NS(kind="consolidation", direction="top", level_id=0,
                 seg_a_start=0, seg_a_end=0, seg_c_start=2, seg_c_end=2,
                 center_idx=0, force_a=100.0, force_c=30.0, confirmed=True)

        results = check_divergence_topology(
            [div], segments, [], strokes=strokes, eta=0.0, normalize=False,
        )
        assert len(results) == 1
        r = results[0]
        assert r.inconclusive is False  # 不再 inconclusive
        assert r.w1_a > 0.0  # A 段有笔振荡
        assert r.w1_c > 0.0  # C 段有笔振荡
        assert r.w1_a > r.w1_c  # A 段振荡 > C 段 → 背驰
        assert r.passed is True
        assert r.macd_agrees is True  # force_a=100 > force_c=30, w1_a > w1_c → 一致

    def test_t8_inconclusive_a_only(self):
        """A 段有中枢但 C 段无中枢 → inconclusive=True（数据不足，非力竭）。"""
        from types import SimpleNamespace as NS

        segments = [
            NS(s0=i, s1=i, i0=i*10, i1=(i+1)*10, direction="up",
               high=100.0 + i * 2, low=98.0 + i * 2)
            for i in range(10)
        ]
        center_a = NS(seg0=1, seg1=3, low=99.0, high=103.0,
                       kind="settled", confirmed=True, sustain=0,
                       direction="up", gg=103.0, dd=99.0, g=100.0, d=101.0,
                       development="", level_id=0, terminated=False)
        div = NS(kind="trend", direction="top", level_id=0,
                 seg_a_start=0, seg_a_end=4, seg_c_start=6, seg_c_end=9,
                 center_idx=0, force_a=100.0, force_c=50.0, confirmed=True)

        results = check_divergence_topology(
            [div], segments, [center_a], eta=0.0,
        )
        assert len(results) == 1
        assert results[0].inconclusive is True
        assert results[0].passed is False
        assert results[0].n_centers_a == 1
        assert results[0].n_centers_c == 0

    def test_t8_inconclusive_c_only(self):
        """C 段有中枢但 A 段无中枢 → inconclusive=True。"""
        from types import SimpleNamespace as NS

        segments = [
            NS(s0=i, s1=i, i0=i*10, i1=(i+1)*10, direction="up",
               high=100.0 + i * 2, low=98.0 + i * 2)
            for i in range(10)
        ]
        center_c = NS(seg0=7, seg1=9, low=112.0, high=113.0,
                       kind="settled", confirmed=True, sustain=0,
                       direction="up", gg=113.0, dd=112.0, g=112.5, d=112.5,
                       development="", level_id=0, terminated=False)
        div = NS(kind="trend", direction="top", level_id=0,
                 seg_a_start=0, seg_a_end=4, seg_c_start=6, seg_c_end=9,
                 center_idx=0, force_a=100.0, force_c=50.0, confirmed=True)

        results = check_divergence_topology(
            [div], segments, [center_c], eta=0.0,
        )
        assert len(results) == 1
        assert results[0].inconclusive is True
        assert results[0].passed is False
        assert results[0].n_centers_a == 0
        assert results[0].n_centers_c == 1

    def test_t8_eta_tolerance(self):
        """eta>0 需要更大的 W₁ drop 才能 pass。"""
        from types import SimpleNamespace as NS

        segments = [
            NS(s0=i, s1=i, i0=i*10, i1=(i+1)*10, direction="up",
               high=100.0 + i, low=98.0 + i)
            for i in range(10)
        ]
        # 两个中枢，A 段的比 C 段的略宽
        center_a = NS(seg0=1, seg1=3, low=99.0, high=102.0,
                       kind="settled", confirmed=True, sustain=0,
                       direction="up", gg=102.0, dd=99.0, g=100.0, d=100.0,
                       development="", level_id=0, terminated=False)
        center_c = NS(seg0=7, seg1=9, low=105.0, high=107.5,
                       kind="settled", confirmed=True, sustain=0,
                       direction="up", gg=107.5, dd=105.0, g=106.0, d=106.0,
                       development="", level_id=0, terminated=False)
        centers = [center_a, center_c]

        div = NS(kind="trend", direction="top", level_id=0,
                 seg_a_start=0, seg_a_end=4, seg_c_start=6, seg_c_end=9,
                 center_idx=1, force_a=100.0, force_c=80.0, confirmed=True)

        # 无规范化，A: |102-99|/2=1.5, C: |107.5-105|/2=1.25
        # w1_drop = 0.25
        # eta=0 → passed (0.25 > 0)
        results_no_eta = check_divergence_topology(
            [div], segments, centers, eta=0.0, normalize=False,
        )
        assert results_no_eta[0].passed is True

        # eta=0.5 → not passed (w1_c=1.25 > w1_a - eta = 1.5 - 0.5 = 1.0? 1.25 > 1.0 → False)
        results_with_eta = check_divergence_topology(
            [div], segments, centers, eta=0.5, normalize=False,
        )
        assert results_with_eta[0].passed is False

    def test_t8_from_pipeline(self, df_raw):
        """真实数据端到端：运行管线 → 检测背驰 → T8 验证。"""
        from newchan.a_topology import _run_pipeline
        from newchan.a_divergence import divergences_from_level

        strokes, segments, centers, trends, rec_levels = _run_pipeline(df_raw, "wide")

        # 获取背驰
        if rec_levels:
            level0 = rec_levels[0]
            divs = divergences_from_level(
                level0.moves if hasattr(level0, "moves") else segments,
                level0.centers,
                level0.trends,
                level0.level,
            )
            used_centers = level0.centers
        else:
            from newchan.a_center_v0 import centers_from_segments_v0
            from newchan.a_trendtype_v0 import trend_instances_from_centers
            used_centers = centers_from_segments_v0(segments)
            used_trends = trend_instances_from_centers(segments, used_centers)
            divs = divergences_from_level(segments, used_centers, used_trends, 0)

        if divs:
            results = check_divergence_topology(
                divs, segments, used_centers, strokes=strokes, eta=0.0,
            )
            assert len(results) == len(divs)
            for r in results:
                assert isinstance(r, T8Result)
                assert r.w1_a >= 0.0
                assert r.w1_c >= 0.0
                assert isinstance(r.passed, bool)
                assert isinstance(r.inconclusive, bool)


# ---------------------------------------------------------------------------
# T6：Leray 条件可计算近似（204号谱系）
# ---------------------------------------------------------------------------

class TestT6CrossLevelLeray:
    """T6 跨层 Leray 可计算近似测试。"""

    def test_t6_bar_length_distribution_basic(self):
        """条带长度直方图基本功能。"""
        bc = ((0.0, 10.0), (0.0, 5.0), (0.0, 2.0))
        dist = _bar_length_distribution(bc)
        assert dist.shape == (10,)
        assert abs(dist.sum() - 1.0) < 1e-9  # 归一化

    def test_t6_bar_length_distribution_empty(self):
        """空条形码返回均匀分布。"""
        dist = _bar_length_distribution(())
        assert dist.shape == (10,)
        assert abs(dist.sum() - 1.0) < 1e-9
        # 均匀分布：每个 bin 相等
        assert abs(dist[0] - dist[-1]) < 1e-9

    def test_t6_kl_divergence_same(self):
        """相同分布 KL = 0。"""
        p = np.array([0.25, 0.25, 0.25, 0.25])
        assert abs(_kl_divergence(p, p)) < 1e-9

    def test_t6_kl_divergence_different(self):
        """不同分布 KL > 0。"""
        p = np.array([0.5, 0.3, 0.1, 0.1])
        q = np.array([0.25, 0.25, 0.25, 0.25])
        kl = _kl_divergence(p, q)
        assert kl > 0

    def test_t6_synthetic_two_levels(self):
        """合成两层递归——T6 检查应产出结果。"""
        from types import SimpleNamespace

        # 低级别：3 个中枢
        centers_low = [
            SimpleNamespace(low=10.0, high=20.0, seg0=0, seg1=2),
            SimpleNamespace(low=25.0, high=35.0, seg0=3, seg1=5),
            SimpleNamespace(low=40.0, high=50.0, seg0=6, seg1=8),
        ]
        # 高级别：1 个中枢（粗粒化后）
        centers_high = [
            SimpleNamespace(low=15.0, high=45.0, seg0=0, seg1=2),
        ]
        level0 = SimpleNamespace(level=0, centers=centers_low)
        level1 = SimpleNamespace(level=1, centers=centers_high)

        results = check_cross_level_leray([level0, level1])
        assert len(results) == 1
        r = results[0]
        assert isinstance(r, T6Result)
        assert r.level_low == 0
        assert r.level_high == 1
        assert r.n_bars_low == 3
        assert r.n_bars_high == 1
        assert r.w1_low >= 0
        assert r.w1_high >= 0
        assert isinstance(r.passed, bool)
        assert isinstance(r.w1_ratio_bounded, bool)
        assert r.w1_ratio >= 0
        assert isinstance(r.bottleneck_bounded, bool)
        assert isinstance(r.kl_bounded, bool)
        # 小样本（1 bar high）→ inconclusive
        assert r.inconclusive is True

    def test_t6_w1_ratio_bounded(self):
        """高级别 W₁ 在 λ 倍以内 → w1_ratio_bounded = True。"""
        from types import SimpleNamespace

        # 低级别：宽条带
        centers_low = [
            SimpleNamespace(low=0.0, high=100.0, seg0=0, seg1=2),
            SimpleNamespace(low=10.0, high=90.0, seg0=3, seg1=5),
        ]
        # 高级别：窄条带（粗粒化后信息量减少）
        centers_high = [
            SimpleNamespace(low=30.0, high=50.0, seg0=0, seg1=1),
        ]
        level0 = SimpleNamespace(level=0, centers=centers_low)
        level1 = SimpleNamespace(level=1, centers=centers_high)

        results = check_cross_level_leray([level0, level1])
        assert results[0].w1_ratio_bounded is True
        assert results[0].w1_ratio <= 3.0

    def test_t6_w1_ratio_unbounded(self):
        """高级别 W₁ 远超低级别 → w1_ratio_bounded = False（λ=1.0 时）。"""
        from types import SimpleNamespace

        # 低级别：窄条带
        centers_low = [
            SimpleNamespace(low=10.0, high=15.0, seg0=0, seg1=2),
        ]
        # 高级别：宽条带（缠论递归放大）
        centers_high = [
            SimpleNamespace(low=0.0, high=100.0, seg0=0, seg1=1),
        ]
        level0 = SimpleNamespace(level=0, centers=centers_low)
        level1 = SimpleNamespace(level=1, centers=centers_high)

        results = check_cross_level_leray([level0, level1], w1_lambda=1.0)
        assert results[0].w1_ratio > 1.0
        assert results[0].w1_ratio_bounded is False

    def test_t6_single_level_no_result(self):
        """单层递归无 T6 结果。"""
        from types import SimpleNamespace
        level0 = SimpleNamespace(level=0, centers=[])
        results = check_cross_level_leray([level0])
        assert results == []

    def test_t6_delta_kappa_params(self):
        """自定义 delta/kappa 参数。"""
        from types import SimpleNamespace

        centers_low = [
            SimpleNamespace(low=0.0, high=50.0, seg0=0, seg1=2),
        ]
        centers_high = [
            SimpleNamespace(low=0.0, high=50.0, seg0=0, seg1=1),
        ]
        level0 = SimpleNamespace(level=0, centers=centers_low)
        level1 = SimpleNamespace(level=1, centers=centers_high)

        # 极严格参数
        results_strict = check_cross_level_leray(
            [level0, level1], delta=0.001, kappa=0.001,
        )
        assert len(results_strict) == 1
        assert results_strict[0].delta == 0.001
        assert results_strict[0].kappa == 0.001

        # 极宽松参数
        results_lenient = check_cross_level_leray(
            [level0, level1], delta=1000.0, kappa=1000.0,
        )
        assert results_lenient[0].bottleneck_bounded is True
        assert results_lenient[0].kl_bounded is True
