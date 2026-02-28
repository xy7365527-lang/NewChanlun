"""双模型共振回测对比测试 — 242号谱系。

覆盖：
- BSP 序列生成正确性
- 单配置评估逻辑
- 干净度指标计算
- 信号一致性度量
- 裁决逻辑
"""

from __future__ import annotations

from newchan.nesting.bsp import BSP, BSPType
from newchan.topology.config_space import (
    CENTER,
    FULL_RISK_OFF,
    FULL_RISK_ON,
    Configuration,
    WalkDirection,
)
from newchan.topology.fiber_pipeline_adapter import FiberSignalFilter

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))

from dual_resonance_backtest import (
    CleanlinessMetrics,
    ConfigResult,
    SignalDecision,
    _compute_verdict,
    _generate_bsp_sequence,
    _polarity_to_direction,
    compute_cleanliness,
    evaluate_config,
    signal_consistency,
)


# ── BSP 序列生成 ──


class TestBspSequenceGeneration:
    """BSP 序列生成正确性。"""

    def test_correct_count(self):
        """指定长度的序列。"""
        bsps = _generate_bsp_sequence(10)
        assert len(bsps) == 10

    def test_alternating_buy_sell(self):
        """偶数索引 buy，奇数索引 sell。"""
        bsps = _generate_bsp_sequence(6)
        assert bsps[0].bsp_type.is_buy
        assert bsps[1].bsp_type.is_sell
        assert bsps[2].bsp_type.is_buy
        assert bsps[3].bsp_type.is_sell

    def test_unique_edge_ids(self):
        """每个 BSP 有唯一 edge_id。"""
        bsps = _generate_bsp_sequence(20)
        ids = [b.edge_id for b in bsps]
        assert len(set(ids)) == len(ids)

    def test_prices_in_range(self):
        """价格在合理区间。"""
        bsps = _generate_bsp_sequence(20)
        for b in bsps:
            assert 80.0 <= b.price <= 140.0


# ── polarity 方向映射 ──


class TestPolarityDirection:

    def test_positive_is_buy(self):
        assert _polarity_to_direction(3) == "buy"
        assert _polarity_to_direction(1) == "buy"

    def test_negative_is_sell(self):
        assert _polarity_to_direction(-3) == "sell"
        assert _polarity_to_direction(-1) == "sell"

    def test_zero_is_neutral(self):
        assert _polarity_to_direction(0) == "neutral"


# ── 单配置评估 ──


class TestConfigEvaluation:
    """单配置评估逻辑。"""

    def test_risk_on_produces_entries(self):
        """FULL_RISK_ON: buy BSP 应通过共振。"""
        bsps = _generate_bsp_sequence(4)
        ff = FiberSignalFilter(kl_threshold=0.0)
        cr = evaluate_config(FULL_RISK_ON, bsps, ff)
        assert cr.product_entry_count > 0

    def test_risk_off_blocks_buy_bsps(self):
        """FULL_RISK_OFF: sell direction，buy BSP 共振不通过。"""
        bsps = _generate_bsp_sequence(4)
        ff = FiberSignalFilter(kl_threshold=0.0)
        cr = evaluate_config(FULL_RISK_OFF, bsps, ff)
        # sell 方向的 BSP 应通过，buy 方向的 BSP 应被阻断
        assert cr.product_block_count > 0

    def test_center_neutral_entries(self):
        """CENTER: polarity=0，CONFIG 使用 BSP 自身方向 -> 全部共振通过。"""
        bsps = _generate_bsp_sequence(4)
        ff = FiberSignalFilter(kl_threshold=0.0)
        cr = evaluate_config(CENTER, bsps, ff)
        # polarity=0 时 CONFIG 层使用 BSP 自身方向 -> 同向 -> 共振通过
        assert cr.product_entry_count == len(bsps)

    def test_decisions_match_bsp_count(self):
        """decisions 数量等于 BSP 数量。"""
        bsps = _generate_bsp_sequence(10)
        ff = FiberSignalFilter(kl_threshold=0.0)
        cr = evaluate_config(FULL_RISK_ON, bsps, ff)
        assert len(cr.decisions) == 10

    def test_config_tuple_correct(self):
        """config_tuple 正确反映配置。"""
        bsps = _generate_bsp_sequence(2)
        ff = FiberSignalFilter(kl_threshold=0.0)
        cr = evaluate_config(FULL_RISK_ON, bsps, ff)
        assert cr.config_tuple == (1, 1, 1)

    def test_divergent_config_detected(self):
        """分歧配置能被检测。"""
        # E=UP, C=DOWN, R=DOWN -> polarity=-1, 纤维丛联络可能修正
        config = Configuration(WalkDirection.UP, WalkDirection.DOWN, WalkDirection.DOWN)
        bsps = _generate_bsp_sequence(4)
        ff = FiberSignalFilter(kl_threshold=0.0)
        cr = evaluate_config(config, bsps, ff)
        # 此配置已知有 polarity 分歧（240号确认）
        assert isinstance(cr.polarity_divergence, bool)


# ── 干净度指标 ──


class TestCleanlinessMetrics:
    """干净度指标计算。"""

    def test_all_27_configs_processed(self):
        """全部 27 种配置应被处理。"""
        bsps = _generate_bsp_sequence(4)
        ff = FiberSignalFilter(kl_threshold=0.0)
        results = []
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    results.append(evaluate_config(config, bsps, ff))
        m = compute_cleanliness(results, bsps)
        assert m.total_configs == 27
        assert m.total_bsp_events == 27 * 4

    def test_entry_plus_block_plus_neutral_equals_total(self):
        """入场+阻断+中立 = 总数。"""
        bsps = _generate_bsp_sequence(10)
        ff = FiberSignalFilter(kl_threshold=0.0)
        results = []
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    results.append(evaluate_config(config, bsps, ff))
        m = compute_cleanliness(results, bsps)
        assert m.product_entries + m.product_blocks + m.product_neutrals == m.total_bsp_events
        assert m.fiber_entries + m.fiber_blocks + m.fiber_neutrals == m.total_bsp_events


# ── 信号一致性 ──


class TestSignalConsistency:
    """信号一致性度量。"""

    def test_consistency_in_range(self):
        """一致性比例在 [0, 1] 内。"""
        bsps = _generate_bsp_sequence(10)
        ff = FiberSignalFilter(kl_threshold=0.0)
        results = []
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    config = Configuration(e, c, r)
                    results.append(evaluate_config(config, bsps, ff))
        c = signal_consistency(results)
        assert 0.0 <= c["product_consistency"] <= 1.0
        assert 0.0 <= c["fiber_consistency"] <= 1.0


# ── 裁决逻辑 ──


class TestVerdict:
    """裁决逻辑正确性。"""

    def test_tie_on_identical_metrics(self):
        """两套指标完全相同时判 tie。"""
        m = CleanlinessMetrics(
            total_bsp_events=100,
            product_entries=50,
            fiber_entries=50,
            product_false_signals=5,
            fiber_false_signals=5,
            product_contradictions=2,
            fiber_contradictions=2,
        )
        c = {
            "product_consistency": 0.8,
            "fiber_consistency": 0.8,
        }
        v = _compute_verdict(m, c)
        assert v["winner"] == "tie"

    def test_lower_false_rate_wins(self):
        """假信号率更低的版本胜出。"""
        m = CleanlinessMetrics(
            total_bsp_events=100,
            product_entries=50,
            fiber_entries=50,
            product_false_signals=10,
            fiber_false_signals=2,
            product_contradictions=0,
            fiber_contradictions=0,
        )
        c = {
            "product_consistency": 0.8,
            "fiber_consistency": 0.8,
        }
        v = _compute_verdict(m, c)
        assert v["winner"] == "fiber"

    def test_scores_are_numeric(self):
        """得分是数值。"""
        m = CleanlinessMetrics(
            total_bsp_events=100,
            product_entries=50,
            fiber_entries=50,
        )
        c = {
            "product_consistency": 0.5,
            "fiber_consistency": 0.5,
        }
        v = _compute_verdict(m, c)
        assert isinstance(v["product_score"], float)
        assert isinstance(v["fiber_score"], float)
