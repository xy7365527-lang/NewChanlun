"""纤维丛 pipeline 适配器测试。

验证：
1. FiberCorrection 正确计算
2. 当 E×C = (FLAT, FLAT) 时修正最小（底空间中心，联络影响弱）
3. 当 E = UP, C = DOWN 时修正显著（risk-on/off 分歧点）
4. 向后兼容：adapter 输出的 config 与原始一致
5. KL 散度非负
6. FiberTradingContext 正确构造（240号）
7. polarity_divergence 检测正确（240号）
8. FiberSignalFilter 逻辑正确（240号）
"""

import math

import pytest

from newchan.topology.config_space import (
    CENTER,
    FULL_RISK_OFF,
    FULL_RISK_ON,
    Configuration,
    WalkDirection,
    polarity_index,
)
from newchan.topology.fiber_bundle import (
    Connection,
    FiberBundleConfigSpace,
    default_fiber_bundle,
)
from newchan.topology.fiber_pipeline_adapter import (
    FiberCorrection,
    FiberPipelineAdapter,
    FiberSignalFilter,
    FiberTradingContext,
    compute_fiber_correction,
    create_fiber_context,
)
from newchan.pipeline import TradingContext, create_context


class TestFiberCorrection:
    """FiberCorrection 正确计算。"""

    def test_product_probabilities_uniform(self):
        """直积概率始终均匀 1/3。"""
        config = Configuration(WalkDirection.UP, WalkDirection.DOWN, WalkDirection.FLAT)
        correction = compute_fiber_correction(config)
        for r in (-1, 0, 1):
            assert abs(correction.r_prob_product[r] - 1.0 / 3.0) < 1e-10

    def test_fiber_probabilities_sum_to_one(self):
        """纤维丛条件概率和为 1。"""
        config = Configuration(WalkDirection.UP, WalkDirection.UP, WalkDirection.UP)
        correction = compute_fiber_correction(config)
        total = sum(correction.r_prob_fiber.values())
        assert abs(total - 1.0) < 1e-10

    def test_kl_divergence_nonnegative(self):
        """KL 散度非负（信息论基本性质）。"""
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    correction = compute_fiber_correction(config)
                    assert correction.kl_divergence >= -1e-15

    def test_correction_magnitude_nonnegative(self):
        """修正幅度非负。"""
        config = FULL_RISK_ON
        correction = compute_fiber_correction(config)
        assert correction.correction_magnitude >= 0.0

    def test_correction_magnitude_bounded(self):
        """修正幅度不超过 2/3（概率差最大值）。"""
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    correction = compute_fiber_correction(config)
                    assert correction.correction_magnitude <= 2.0 / 3.0 + 1e-10

    def test_original_config_preserved(self):
        """原始配置在修正结果中保留不变。"""
        config = Configuration(WalkDirection.DOWN, WalkDirection.UP, WalkDirection.FLAT)
        correction = compute_fiber_correction(config)
        assert correction.original_config is config
        assert correction.original_config.as_tuple == config.as_tuple


class TestCenterPointMinimalCorrection:
    """E×C = (FLAT, FLAT) 时修正最小（底空间中心，联络影响弱）。"""

    def test_center_kl_zero(self):
        """底空间原点 (0,0) 上联络无效应 → KL = 0。"""
        config = CENTER  # (0, 0, 0)
        correction = compute_fiber_correction(config)
        assert abs(correction.kl_divergence) < 1e-10

    def test_center_correction_magnitude_zero(self):
        """底空间原点上修正幅度为 0。"""
        config = CENTER
        correction = compute_fiber_correction(config)
        assert abs(correction.correction_magnitude) < 1e-10

    def test_center_polarity_not_flipped(self):
        """底空间原点上 polarity 不翻转。"""
        config = CENTER
        correction = compute_fiber_correction(config)
        assert not correction.polarity_flipped

    def test_flat_base_all_r_values(self):
        """底空间 (FLAT, FLAT) 上所有 R 值的修正幅度为 0。"""
        for r in WalkDirection:
            config = Configuration(WalkDirection.FLAT, WalkDirection.FLAT, r)
            correction = compute_fiber_correction(config)
            assert abs(correction.correction_magnitude) < 1e-10


class TestDivergencePointSignificantCorrection:
    """E = UP, C = DOWN 时修正显著（risk-on/off 分歧点）。"""

    def test_up_down_kl_positive(self):
        """E=UP, C=DOWN → 联络产生非均匀纤维分布 → KL > 0。"""
        config = Configuration(
            WalkDirection.UP, WalkDirection.DOWN, WalkDirection.FLAT,
        )
        correction = compute_fiber_correction(config)
        assert correction.kl_divergence > 0.0

    def test_up_down_correction_nonzero(self):
        """E=UP, C=DOWN → 修正幅度 > 0。"""
        config = Configuration(
            WalkDirection.UP, WalkDirection.DOWN, WalkDirection.FLAT,
        )
        correction = compute_fiber_correction(config)
        assert correction.correction_magnitude > 0.01

    def test_divergence_exceeds_center(self):
        """分歧点的 KL 散度严格大于中心点。"""
        center = compute_fiber_correction(CENTER)
        divergence = compute_fiber_correction(
            Configuration(WalkDirection.UP, WalkDirection.DOWN, WalkDirection.FLAT),
        )
        assert divergence.kl_divergence > center.kl_divergence


class TestBackwardCompatibility:
    """adapter 输出的 config 与原始一致。"""

    def test_fiber_point_config_matches_original(self):
        """FiberBundlePoint.config 返回与原始 Configuration 等值的对象。"""
        config = Configuration(WalkDirection.UP, WalkDirection.DOWN, WalkDirection.UP)
        correction = compute_fiber_correction(config)
        assert correction.fiber_point.config.as_tuple == config.as_tuple

    def test_all_27_configs_compatible(self):
        """全部 27 种配置的 fiber_point.config 与原始一致。"""
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    correction = compute_fiber_correction(config)
                    assert correction.fiber_point.config.as_tuple == config.as_tuple

    def test_polarity_index_matches_original(self):
        """直积 polarity 计算与原始一致。"""
        config = FULL_RISK_ON
        correction = compute_fiber_correction(config)
        assert polarity_index(correction.original_config) == polarity_index(config)


class TestKLDivergenceProperties:
    """KL 散度的数学性质。"""

    def test_kl_zero_for_flat_connection(self):
        """平坦联络 → 所有配置的 KL = 0。"""
        flat_fb = FiberBundleConfigSpace(Connection(0.0, 0.0))
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    correction = compute_fiber_correction(config, flat_fb)
                    assert abs(correction.kl_divergence) < 1e-10

    def test_kl_nonnegative_for_all_configs(self):
        """默认联络下所有 27 种配置的 KL ≥ 0。"""
        fb = default_fiber_bundle()
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    correction = compute_fiber_correction(config, fb)
                    assert correction.kl_divergence >= -1e-15

    def test_kl_symmetric_by_base_point(self):
        """同一底空间点上不同 R 值的 KL 相同（KL 仅依赖底空间点）。"""
        fb = default_fiber_bundle()
        for e in WalkDirection:
            for c in WalkDirection:
                kls = []
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    correction = compute_fiber_correction(config, fb)
                    kls.append(correction.kl_divergence)
                # 同一底空间点上的 KL 应该相同
                assert abs(kls[0] - kls[1]) < 1e-10
                assert abs(kls[1] - kls[2]) < 1e-10


class TestFiberPipelineAdapter:
    """FiberPipelineAdapter 集成测试。"""

    @pytest.fixture
    def adapter(self):
        return FiberPipelineAdapter()

    @pytest.fixture
    def flat_adapter(self):
        return FiberPipelineAdapter(FiberBundleConfigSpace(Connection(0.0, 0.0)))

    def test_correct_returns_fiber_correction(self, adapter):
        correction = adapter.correct(FULL_RISK_ON)
        assert isinstance(correction, FiberCorrection)

    def test_batch_correct_length(self, adapter):
        configs = [FULL_RISK_ON, FULL_RISK_OFF, CENTER]
        corrections = adapter.batch_correct(configs)
        assert len(corrections) == 3

    def test_batch_correct_matches_single(self, adapter):
        configs = [FULL_RISK_ON, CENTER]
        batch = adapter.batch_correct(configs)
        for config, correction in zip(configs, batch):
            single = adapter.correct(config)
            assert abs(correction.kl_divergence - single.kl_divergence) < 1e-10

    def test_effective_dimension(self, adapter):
        dim = adapter.effective_dimension()
        assert 2.0 < dim < 3.0

    def test_flat_effective_dimension(self, flat_adapter):
        dim = flat_adapter.effective_dimension()
        assert abs(dim - 3.0) < 0.01

    def test_global_kl_divergence_positive(self, adapter):
        kl = adapter.global_kl_divergence()
        assert kl > 0.0

    def test_flat_global_kl_zero(self, flat_adapter):
        kl = flat_adapter.global_kl_divergence()
        assert abs(kl) < 1e-10


# ── 240号：FiberTradingContext 测试 ─────────────────────────────


class TestFiberTradingContext:
    """FiberTradingContext 正确构造。"""

    @pytest.fixture
    def risk_on_ctx(self):
        """FULL_RISK_ON 配置的 TradingContext。"""
        return create_context(FULL_RISK_ON, timestamp=1000.0)

    @pytest.fixture
    def center_ctx(self):
        """CENTER 配置的 TradingContext。"""
        return create_context(CENTER, timestamp=2000.0)

    @pytest.fixture
    def divergence_ctx(self):
        """E=UP, C=DOWN 分歧点的 TradingContext。"""
        config = Configuration(WalkDirection.UP, WalkDirection.DOWN, WalkDirection.FLAT)
        return create_context(config, timestamp=3000.0)

    def test_wraps_original_ctx(self, risk_on_ctx):
        """fiber_ctx.ctx 是原始 TradingContext 的引用。"""
        fiber_ctx = create_fiber_context(risk_on_ctx)
        assert fiber_ctx.ctx is risk_on_ctx

    def test_config_delegation(self, risk_on_ctx):
        """config 属性委托给原始 ctx。"""
        fiber_ctx = create_fiber_context(risk_on_ctx)
        assert fiber_ctx.config is risk_on_ctx.config

    def test_timestamp_delegation(self, risk_on_ctx):
        """timestamp 属性委托给原始 ctx。"""
        fiber_ctx = create_fiber_context(risk_on_ctx)
        assert fiber_ctx.timestamp == 1000.0

    def test_product_polarity_matches_ctx(self, risk_on_ctx):
        """product_polarity 与原始 ctx.polarity 一致。"""
        fiber_ctx = create_fiber_context(risk_on_ctx)
        assert fiber_ctx.product_polarity == risk_on_ctx.polarity

    def test_fiber_correction_present(self, risk_on_ctx):
        """fiber_correction 是 FiberCorrection 实例。"""
        fiber_ctx = create_fiber_context(risk_on_ctx)
        assert isinstance(fiber_ctx.fiber_correction, FiberCorrection)

    def test_fiber_polarity_is_int(self, risk_on_ctx):
        """fiber_polarity 是整数。"""
        fiber_ctx = create_fiber_context(risk_on_ctx)
        assert isinstance(fiber_ctx.fiber_polarity, int)

    def test_correction_confidence_is_kl(self, risk_on_ctx):
        """correction_confidence 等于 KL 散度。"""
        fiber_ctx = create_fiber_context(risk_on_ctx)
        assert fiber_ctx.correction_confidence == fiber_ctx.fiber_correction.kl_divergence

    def test_center_no_divergence(self, center_ctx):
        """底空间中心无 polarity 分歧。"""
        fiber_ctx = create_fiber_context(center_ctx)
        assert not fiber_ctx.polarity_divergence

    def test_center_zero_confidence(self, center_ctx):
        """底空间中心的修正置信度为 0。"""
        fiber_ctx = create_fiber_context(center_ctx)
        assert abs(fiber_ctx.correction_confidence) < 1e-10

    def test_immutability(self, risk_on_ctx):
        """FiberTradingContext 是不可变的（frozen=True）。"""
        fiber_ctx = create_fiber_context(risk_on_ctx)
        with pytest.raises(AttributeError):
            fiber_ctx.fiber_polarity = 999  # type: ignore[misc]

    def test_custom_fiber_bundle(self, risk_on_ctx):
        """可以传入自定义纤维丛。"""
        flat_fb = FiberBundleConfigSpace(Connection(0.0, 0.0))
        fiber_ctx = create_fiber_context(risk_on_ctx, flat_fb)
        assert abs(fiber_ctx.correction_confidence) < 1e-10
        assert not fiber_ctx.polarity_divergence


class TestPolarityDivergenceDetection:
    """polarity_divergence 检测正确。"""

    def test_all_flat_base_no_divergence(self):
        """底空间 (FLAT, FLAT) 上所有 R 值无 polarity 分歧。"""
        for r in WalkDirection:
            config = Configuration(WalkDirection.FLAT, WalkDirection.FLAT, r)
            ctx = create_context(config)
            fiber_ctx = create_fiber_context(ctx)
            assert not fiber_ctx.polarity_divergence

    def test_flat_connection_no_divergence(self):
        """平坦联络下所有配置无 polarity 分歧。"""
        flat_fb = FiberBundleConfigSpace(Connection(0.0, 0.0))
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    ctx = create_context(config)
                    fiber_ctx = create_fiber_context(ctx, flat_fb)
                    assert not fiber_ctx.polarity_divergence

    def test_divergence_possible_for_non_flat(self):
        """非平坦底空间上存在 polarity 分歧的配置。"""
        divergence_found = False
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    ctx = create_context(config)
                    fiber_ctx = create_fiber_context(ctx)
                    if fiber_ctx.polarity_divergence:
                        divergence_found = True
                        break
                if divergence_found:
                    break
            if divergence_found:
                break
        assert divergence_found, "默认联络下应存在至少一个 polarity 分歧的配置"


class TestFiberSignalFilter:
    """FiberSignalFilter 逻辑正确。"""

    @pytest.fixture
    def default_filter(self):
        return FiberSignalFilter()

    @pytest.fixture
    def high_threshold_filter(self):
        return FiberSignalFilter(kl_threshold=100.0)

    def test_no_override_when_no_divergence(self, default_filter):
        """无 polarity 分歧时不覆盖。"""
        ctx = create_context(CENTER)
        fiber_ctx = create_fiber_context(ctx)
        assert not default_filter.should_override_polarity(fiber_ctx)

    def test_no_override_with_high_threshold(self, high_threshold_filter):
        """高阈值时即使有分歧也不覆盖。"""
        # 找一个有分歧的配置
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    ctx = create_context(config)
                    fiber_ctx = create_fiber_context(ctx)
                    # 即使有分歧，阈值太高也不覆盖
                    assert not high_threshold_filter.should_override_polarity(fiber_ctx)

    def test_override_when_divergence_and_sufficient_confidence(self, default_filter):
        """有分歧且置信度足够时覆盖。"""
        override_found = False
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    ctx = create_context(config)
                    fiber_ctx = create_fiber_context(ctx)
                    if default_filter.should_override_polarity(fiber_ctx):
                        assert fiber_ctx.polarity_divergence
                        assert fiber_ctx.correction_confidence > 0.0
                        override_found = True
        assert override_found, "默认联络下应存在被覆盖的配置"

    def test_correction_report_structure(self, default_filter):
        """修正报告包含所有必要字段。"""
        ctx = create_context(FULL_RISK_ON, timestamp=42.0)
        fiber_ctx = create_fiber_context(ctx)
        report = default_filter.correction_report(fiber_ctx)

        required_keys = {
            "product_polarity",
            "fiber_polarity",
            "polarity_divergence",
            "should_override",
            "kl_divergence",
            "correction_magnitude",
            "r_prob_product",
            "r_prob_fiber",
            "config",
            "timestamp",
        }
        assert set(report.keys()) == required_keys

    def test_correction_report_values(self, default_filter):
        """修正报告的值与 FiberTradingContext 一致。"""
        ctx = create_context(FULL_RISK_ON, timestamp=42.0)
        fiber_ctx = create_fiber_context(ctx)
        report = default_filter.correction_report(fiber_ctx)

        assert report["product_polarity"] == fiber_ctx.product_polarity
        assert report["fiber_polarity"] == fiber_ctx.fiber_polarity
        assert report["polarity_divergence"] == fiber_ctx.polarity_divergence
        assert report["kl_divergence"] == fiber_ctx.fiber_correction.kl_divergence
        assert report["timestamp"] == 42.0
        assert report["config"] == FULL_RISK_ON.as_tuple

    def test_kl_threshold_property(self):
        """kl_threshold 属性可读。"""
        f = FiberSignalFilter(kl_threshold=0.5)
        assert f.kl_threshold == 0.5
