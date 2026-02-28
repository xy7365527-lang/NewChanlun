"""双模型共振信号测试 — 241号谱系。

覆盖：
- DualResonanceSignals 结构正确性
- 直积版共振信号正确构建
- 纤维丛版共振信号正确构建
- 两版信号在中心点（FLAT,FLAT,FLAT）一致
- 两版信号在分歧点（risk-on/off 边缘）可能不同
- _polarity_to_bsp 方向映射
- _build_signals_for_polarity 信号层分配
"""

from __future__ import annotations

from newchan.nesting.bsp import BSP, BSPType
from newchan.nesting.resonance import SignalLayer
from newchan.pipeline import create_context
from newchan.pipeline_backtest import (
    DualResonanceSignals,
    _build_resonance_signals,
    _build_signals_for_polarity,
    _polarity_to_bsp,
    _polarity_to_scan_direction,
)
from newchan.topology.config_space import (
    CENTER,
    FULL_RISK_OFF,
    FULL_RISK_ON,
    Configuration,
    WalkDirection,
)
from newchan.topology.fiber_pipeline_adapter import FiberSignalFilter


def _buy_bsp(edge_id: str = "test_edge", level: int = 1) -> BSP:
    return BSP(
        edge_id=edge_id,
        level=level,
        time=100.0,
        bsp_type=BSPType.B1,
        price=100.0,
    )


def _sell_bsp(edge_id: str = "test_edge", level: int = 1) -> BSP:
    return BSP(
        edge_id=edge_id,
        level=level,
        time=100.0,
        bsp_type=BSPType.S1,
        price=100.0,
    )


# ── _polarity_to_bsp ──


class TestPolarityToBsp:
    """polarity 到 BSP 方向映射。"""

    def test_positive_polarity_gives_buy(self):
        """polarity > 0 -> B1。"""
        bsp = _buy_bsp()
        config_bsp = _polarity_to_bsp(3, bsp)
        assert config_bsp.bsp_type == BSPType.B1
        assert config_bsp.edge_id == f"{bsp.edge_id}_config"

    def test_negative_polarity_gives_sell(self):
        """polarity < 0 -> S1。"""
        bsp = _buy_bsp()
        config_bsp = _polarity_to_bsp(-2, bsp)
        assert config_bsp.bsp_type == BSPType.S1

    def test_zero_polarity_uses_bsp_direction(self):
        """polarity = 0 -> 使用 BSP 自身方向。"""
        bsp = _buy_bsp()
        config_bsp = _polarity_to_bsp(0, bsp)
        assert config_bsp.bsp_type == bsp.bsp_type

    def test_preserves_metadata(self):
        """保留原始 BSP 的 level/time/price。"""
        bsp = _buy_bsp()
        config_bsp = _polarity_to_bsp(1, bsp)
        assert config_bsp.level == bsp.level
        assert config_bsp.time == bsp.time
        assert config_bsp.price == bsp.price


# ── _build_signals_for_polarity ──


class TestBuildSignalsForPolarity:
    """从 polarity 构建信号组。"""

    def test_two_signals_produced(self):
        """始终产生两个信号。"""
        signals = _build_signals_for_polarity(3, _buy_bsp())
        assert len(signals) == 2

    def test_config_layer_first(self):
        """第一个信号是 CONFIG 层。"""
        signals = _build_signals_for_polarity(3, _buy_bsp())
        assert signals[0].layer == SignalLayer.CONFIG

    def test_edge_layer_second(self):
        """第二个信号是 INDEPENDENT_EDGE 层。"""
        signals = _build_signals_for_polarity(3, _buy_bsp())
        assert signals[1].layer == SignalLayer.INDEPENDENT_EDGE

    def test_positive_polarity_buy_bsp_same_direction(self):
        """polarity > 0 + buy BSP -> 同向（共振应通过）。"""
        signals = _build_signals_for_polarity(3, _buy_bsp())
        assert signals[0].bsp.bsp_type.is_buy
        assert signals[1].bsp.bsp_type.is_buy

    def test_negative_polarity_buy_bsp_opposite_direction(self):
        """polarity < 0 + buy BSP -> 反向（共振应失败）。"""
        signals = _build_signals_for_polarity(-3, _buy_bsp())
        assert signals[0].bsp.bsp_type.is_sell  # CONFIG 是 sell
        assert signals[1].bsp.bsp_type.is_buy  # EDGE 是 buy

    def test_positive_polarity_sell_bsp_opposite_direction(self):
        """polarity > 0 + sell BSP -> 反向（共振应失败）。"""
        signals = _build_signals_for_polarity(3, _sell_bsp())
        assert signals[0].bsp.bsp_type.is_buy  # CONFIG 是 buy
        assert signals[1].bsp.bsp_type.is_sell  # EDGE 是 sell


# ── DualResonanceSignals 结构 ──


class TestDualResonanceSignalsStructure:
    """DualResonanceSignals 不可变 + 字段完整。"""

    def test_frozen(self):
        """DualResonanceSignals 是 frozen。"""
        signals = _build_signals_for_polarity(3, _buy_bsp())
        dual = DualResonanceSignals(
            product_signals=signals,
            fiber_signals=signals,
            product_polarity=3,
            fiber_polarity=3,
            polarity_divergence=False,
            fiber_scan_direction="buy",
        )
        try:
            dual.product_polarity = 0  # type: ignore[misc]
            assert False, "Should have raised"
        except AttributeError:
            pass

    def test_all_fields_present(self):
        """六个字段全部可访问。"""
        signals = _build_signals_for_polarity(3, _buy_bsp())
        dual = DualResonanceSignals(
            product_signals=signals,
            fiber_signals=signals,
            product_polarity=3,
            fiber_polarity=1,
            polarity_divergence=True,
            fiber_scan_direction="buy",
        )
        assert dual.product_signals == signals
        assert dual.fiber_signals == signals
        assert dual.product_polarity == 3
        assert dual.fiber_polarity == 1
        assert dual.polarity_divergence is True
        assert dual.fiber_scan_direction == "buy"


# ── _build_resonance_signals 直积版 ──


class TestProductSignals:
    """直积版共振信号正确构建。"""

    def test_risk_on_buy_bsp_resonates(self):
        """FULL_RISK_ON (S=+3) + buy BSP -> CONFIG(buy) + EDGE(buy) = 同向。"""
        ctx = create_context(FULL_RISK_ON)
        dual = _build_resonance_signals(_buy_bsp(), ctx)
        # 两个信号都是 buy 方向
        assert all(s.bsp.bsp_type.is_buy for s in dual.product_signals)

    def test_risk_off_buy_bsp_conflicts(self):
        """FULL_RISK_OFF (S=-3) + buy BSP -> CONFIG(sell) + EDGE(buy) = 反向。"""
        ctx = create_context(FULL_RISK_OFF)
        dual = _build_resonance_signals(_buy_bsp(), ctx)
        config_sig = dual.product_signals[0]
        edge_sig = dual.product_signals[1]
        assert config_sig.bsp.bsp_type.is_sell
        assert edge_sig.bsp.bsp_type.is_buy

    def test_center_buy_bsp_neutral(self):
        """CENTER (S=0) + buy BSP -> CONFIG 使用 BSP 自身方向。"""
        ctx = create_context(CENTER)
        dual = _build_resonance_signals(_buy_bsp(), ctx)
        # polarity=0 时 CONFIG 层使用 BSP 自身方向
        assert dual.product_signals[0].bsp.bsp_type.is_buy
        assert dual.product_signals[1].bsp.bsp_type.is_buy

    def test_product_polarity_matches_config(self):
        """product_polarity 等于 polarity_index(config)。"""
        ctx = create_context(FULL_RISK_ON)
        dual = _build_resonance_signals(_buy_bsp(), ctx)
        assert dual.product_polarity == 3


# ── _build_resonance_signals 纤维丛版 ──


class TestFiberSignals:
    """纤维丛版共振信号正确构建。"""

    def test_fiber_polarity_reported(self):
        """fiber_polarity 字段非空。"""
        ctx = create_context(FULL_RISK_ON)
        dual = _build_resonance_signals(_buy_bsp(), ctx)
        assert isinstance(dual.fiber_polarity, int)

    def test_polarity_divergence_reported(self):
        """polarity_divergence 字段正确反映两者差异。"""
        ctx = create_context(FULL_RISK_ON)
        dual = _build_resonance_signals(_buy_bsp(), ctx)
        expected_divergence = dual.product_polarity != dual.fiber_polarity
        assert dual.polarity_divergence == expected_divergence

    def test_fiber_signals_produced(self):
        """fiber_signals 始终有两个信号。"""
        ctx = create_context(FULL_RISK_ON)
        dual = _build_resonance_signals(_buy_bsp(), ctx)
        assert len(dual.fiber_signals) == 2

    def test_custom_fiber_filter_threshold(self):
        """高 KL 阈值下 fiber_signals 退化为与 product_signals 相同。"""
        ctx = create_context(FULL_RISK_ON)
        # 设置极高阈值，纤维丛修正不被采纳
        high_threshold_filter = FiberSignalFilter(kl_threshold=1000.0)
        dual = _build_resonance_signals(_buy_bsp(), ctx, high_threshold_filter)
        # fiber_signals 应与 product_signals 方向相同
        for ps, fs in zip(dual.product_signals, dual.fiber_signals):
            assert ps.bsp.bsp_type == fs.bsp.bsp_type


# ── 中心点一致性 ──


class TestCenterPointConsistency:
    """CENTER (0,0,0) 处两版信号一致。"""

    def test_center_no_divergence(self):
        """中心点 polarity_divergence = False。"""
        ctx = create_context(CENTER)
        dual = _build_resonance_signals(_buy_bsp(), ctx)
        # 中心点联络无效应，fiber polarity 应与 product polarity 相同
        assert dual.polarity_divergence is False

    def test_center_signals_identical_direction(self):
        """中心点两版信号方向一致。"""
        ctx = create_context(CENTER)
        dual = _build_resonance_signals(_buy_bsp(), ctx)
        for ps, fs in zip(dual.product_signals, dual.fiber_signals):
            assert ps.bsp.bsp_type == fs.bsp.bsp_type


# ── 分歧点差异 ──


class TestDivergencePoints:
    """risk-on/off 边缘配置可能产生两版信号差异。"""

    def test_edge_config_reports_divergence(self):
        """边缘配置（如 UP,DOWN,-1）可能有 polarity 分歧。"""
        # E=UP, C=DOWN, R=DOWN -> polarity=-1, 但纤维丛联络可能修正
        config = Configuration(WalkDirection.UP, WalkDirection.DOWN, WalkDirection.DOWN)
        ctx = create_context(config)
        dual = _build_resonance_signals(_buy_bsp(), ctx)
        # 不断言具体值——只验证结构完整
        assert isinstance(dual.polarity_divergence, bool)
        assert isinstance(dual.fiber_polarity, int)

    def test_multiple_configs_vary(self):
        """27 种配置中，至少有一些产生分歧。"""
        divergence_count = 0
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    ctx = create_context(config)
                    dual = _build_resonance_signals(_buy_bsp(), ctx)
                    if dual.polarity_divergence:
                        divergence_count += 1
        # 240号结论：59.3% (16/27) 有分歧
        assert divergence_count > 0

    def test_divergence_may_change_signal_direction(self):
        """当 polarity 分歧时，fiber_signals CONFIG 层方向可能不同于 product_signals。"""
        # E=UP, C=DOWN, R=DOWN -> product polarity = -1 (sell)
        config = Configuration(WalkDirection.UP, WalkDirection.DOWN, WalkDirection.DOWN)
        ctx = create_context(config)
        # 使用零阈值（任何 KL > 0 都触发覆盖）
        ff = FiberSignalFilter(kl_threshold=0.0)
        dual = _build_resonance_signals(_buy_bsp(), ctx, ff)
        if dual.polarity_divergence:
            # 当有分歧时，fiber_signals 的 CONFIG 层可能与 product_signals 不同
            product_config_dir = dual.product_signals[0].bsp.bsp_type
            fiber_config_dir = dual.fiber_signals[0].bsp.bsp_type
            # 不强制不同（取决于具体纤维丛参数），但结构可观测
            assert product_config_dir.is_present
            assert fiber_config_dir.is_present


# ── _polarity_to_scan_direction ──


class TestPolarityToScanDirection:
    """polarity 到 scan direction 映射（243号）。"""

    def test_positive_gives_buy(self):
        assert _polarity_to_scan_direction(3) == "buy"
        assert _polarity_to_scan_direction(1) == "buy"

    def test_negative_gives_sell(self):
        assert _polarity_to_scan_direction(-3) == "sell"
        assert _polarity_to_scan_direction(-1) == "sell"

    def test_zero_gives_neutral(self):
        assert _polarity_to_scan_direction(0) == "neutral"


# ── fiber_scan_direction 一致性（243号）──


class TestFiberScanDirectionConsistency:
    """fiber_scan_direction 与 fiber_polarity 一致。"""

    def test_fiber_scan_direction_matches_fiber_polarity(self):
        """fiber_scan_direction 由 fiber_polarity 推导，两者方向一致。"""
        ctx = create_context(FULL_RISK_ON)
        dual = _build_resonance_signals(_buy_bsp(), ctx)
        expected = _polarity_to_scan_direction(dual.fiber_polarity)
        assert dual.fiber_scan_direction == expected

    def test_fiber_scan_direction_all_configs(self):
        """27 种配置下 fiber_scan_direction 始终与 fiber_polarity 一致。"""
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    ctx = create_context(config)
                    dual = _build_resonance_signals(_buy_bsp(), ctx)
                    expected = _polarity_to_scan_direction(dual.fiber_polarity)
                    assert dual.fiber_scan_direction == expected, (
                        f"config={config.label}: "
                        f"fiber_scan_direction={dual.fiber_scan_direction} "
                        f"!= expected={expected} "
                        f"(fiber_polarity={dual.fiber_polarity})"
                    )

    def test_fiber_scan_direction_with_filter_override(self):
        """高阈值下 fiber_scan_direction 仍由 fiber_polarity 决定（不受阈值影响）。"""
        config = Configuration(WalkDirection.UP, WalkDirection.DOWN, WalkDirection.DOWN)
        ctx = create_context(config)
        # 高阈值：纤维丛修正不被采纳 → fiber_pol 回退到 product_pol
        high_filter = FiberSignalFilter(kl_threshold=1000.0)
        dual = _build_resonance_signals(_buy_bsp(), ctx, high_filter)
        # fiber_scan_direction 仍然由实际使用的 fiber_pol 决定
        expected = _polarity_to_scan_direction(dual.fiber_polarity)
        assert dual.fiber_scan_direction == expected

    def test_divergent_config_fiber_scan_direction_differs(self):
        """当 polarity 分歧时，fiber_scan_direction 可能与直积 scan_direction 不同。"""
        config = Configuration(WalkDirection.UP, WalkDirection.DOWN, WalkDirection.DOWN)
        ctx = create_context(config)
        ff = FiberSignalFilter(kl_threshold=0.0)
        dual = _build_resonance_signals(_buy_bsp(), ctx, ff)
        product_scan = _polarity_to_scan_direction(dual.product_polarity)
        if dual.polarity_divergence:
            # 有分歧时两者方向可能不同（取决于纤维丛参数）
            assert isinstance(dual.fiber_scan_direction, str)
            assert dual.fiber_scan_direction in ("buy", "sell", "neutral")
        else:
            # 无分歧时两者一致
            assert dual.fiber_scan_direction == product_scan
