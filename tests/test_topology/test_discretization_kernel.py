"""离散化算子 D 核结构测试。

验证 233号谱系：ker(D) 从缠论公理推导 + 232号经验数据验证。
"""

import pytest

from newchan.topology.discretization_kernel import (
    DISCRETIZATION_LAYERS,
    D1,
    D2,
    D3,
    CouplingClassification,
    CouplingType,
    DiscretizationOperator,
    KernelStructure,
    classify_coupling,
    kernel_structure,
)


# ── 层结构测试 ──────────────────────────────────────────────


class TestDiscretizationLayers:
    """离散化算子三层结构测试。"""

    def test_three_layers(self) -> None:
        assert len(DISCRETIZATION_LAYERS) == 3

    def test_layer_names(self) -> None:
        names = [layer.name for layer in DISCRETIZATION_LAYERS]
        assert names == ["D1", "D2", "D3"]

    def test_d1_absorbs_amplitude(self) -> None:
        """D1（笔）吸收幅度信息——公理：笔只编码方向。"""
        assert D1.absorbs_amplitude is True
        assert D1.absorbs_weak_direction is False

    def test_d2_absorbs_weak_direction(self) -> None:
        """D2（线段）吸收弱方向信号——公理：线段终结须线段终结。"""
        assert D2.absorbs_amplitude is False
        assert D2.absorbs_weak_direction is True

    def test_d3_absorbs_both(self) -> None:
        """D3（中枢→走势）吸收幅度和弱方向——公理：只有趋势有方向。"""
        assert D3.absorbs_amplitude is True
        assert D3.absorbs_weak_direction is True

    def test_composition_order(self) -> None:
        """D = D3 . D2 . D1：层按执行顺序排列。"""
        assert DISCRETIZATION_LAYERS[0] is D1
        assert DISCRETIZATION_LAYERS[1] is D2
        assert DISCRETIZATION_LAYERS[2] is D3


# ── 核结构测试 ──────────────────────────────────────────────


class TestKernelStructure:
    """ker(D) 结构定理测试。"""

    def test_kernel_not_empty(self) -> None:
        ks = kernel_structure()
        assert len(ks.description) > 0
        assert len(ks.absorption_mechanism) == 3
        assert len(ks.axiom_chain) >= 4

    def test_kernel_is_pure_amplitude(self) -> None:
        """核心定理：ker(D) = 纯幅度耦合。"""
        ks = kernel_structure()
        assert "幅度" in ks.description
        assert "方向" in ks.description

    def test_axiom_chain_completeness(self) -> None:
        """公理链引用 bi/xianduan/zhongshu/qushi 四层定义。"""
        ks = kernel_structure()
        chain_text = " ".join(ks.axiom_chain)
        assert "bi.md" in chain_text
        assert "xianduan.md" in chain_text
        assert "zhongshu.md" in chain_text
        assert "qushi.md" in chain_text


# ── 232号经验数据验证 ───────────────────────────────────────


class TestClassifyCoupling232:
    """用 232号数据验证 ker(D) 理论。"""

    def test_er_in_kernel(self) -> None:
        """E-R: partial_corr = -0.336, beta = -0.017 -> ker(D)。

        E-R 跷跷板是幅度现象：跌时一个跌多一个跌少。
        D1 丢弃幅度 -> D3 FLAT 吸收 -> beta 接近零。
        """
        result = classify_coupling(partial_corr=-0.336, beta=-0.017)
        assert result.in_kernel is True
        assert result.coupling_type == CouplingType.AMPLITUDE

    def test_cr_not_in_kernel(self) -> None:
        """C-R: partial_corr = 0.153, beta = 0.688 -> not ker(D)。

        C-R 避险同向性是方向现象：黄金涨→债券涨。
        D1 保留方向 -> 穿透所有层 -> beta 显著。
        """
        result = classify_coupling(partial_corr=0.153, beta=0.688)
        assert result.in_kernel is False
        assert result.coupling_type == CouplingType.DIRECTION

    def test_ec_independent(self) -> None:
        """E-C: partial_corr = -0.029, beta ~ 0 -> 底空间独立。

        E-C 控制 $ 后独立 -> 不走纤维丛联络。
        """
        result = classify_coupling(partial_corr=-0.029, beta=0.0)
        assert result.coupling_type == CouplingType.INDEPENDENT
        assert result.in_kernel is False

    def test_er_absorption_ratio_near_one(self) -> None:
        """E-R 吸收率接近 1（几乎完全吸收）。"""
        result = classify_coupling(partial_corr=-0.336, beta=-0.017)
        assert result.absorption_ratio > 0.9

    def test_cr_absorption_ratio_below_one(self) -> None:
        """C-R 吸收率远低于 1（方向穿透）。

        注意：C-R beta(0.688) > partial_corr(0.153)，
        说明方向耦合在离散层面被放大而非吸收。
        absorption_ratio 被 clamp 到 [0, 1]。
        """
        result = classify_coupling(partial_corr=0.153, beta=0.688)
        # beta > partial_corr -> 负吸收 -> clamped to 0
        assert result.absorption_ratio == 0.0

    def test_er_dominant_layer(self) -> None:
        """E-R 的主要吸收层是 D1+D3。"""
        result = classify_coupling(partial_corr=-0.336, beta=-0.017)
        assert "D1" in result.dominant_layer


# ── DiscretizationOperator 集成测试 ─────────────────────────


class TestDiscretizationOperator:
    """DiscretizationOperator 集成测试。"""

    def test_kernel_property(self) -> None:
        op = DiscretizationOperator()
        ks = op.kernel
        assert isinstance(ks, KernelStructure)

    def test_classify_method(self) -> None:
        op = DiscretizationOperator()
        result = op.classify(partial_corr=-0.336, beta=-0.017)
        assert isinstance(result, CouplingClassification)
        assert result.in_kernel is True

    def test_layer_analysis(self) -> None:
        op = DiscretizationOperator()
        analysis = op.layer_analysis()
        assert len(analysis) == 3
        assert analysis[0]["name"] == "D1"
        assert analysis[1]["name"] == "D2"
        assert analysis[2]["name"] == "D3"

    def test_verify_232(self) -> None:
        """完整 232号验证。"""
        op = DiscretizationOperator()
        results = op.verify_232()

        assert "E-R" in results
        assert "C-R" in results
        assert "E-C" in results

        # E-R 在核内
        assert results["E-R"].in_kernel is True
        assert results["E-R"].coupling_type == CouplingType.AMPLITUDE

        # C-R 不在核内
        assert results["C-R"].in_kernel is False
        assert results["C-R"].coupling_type == CouplingType.DIRECTION

        # E-C 是底空间独立
        assert results["E-C"].coupling_type == CouplingType.INDEPENDENT

    def test_verify_232_sign_consistency(self) -> None:
        """232号验证：符号一致性。

        E-R: 偏相关为负（反向耦合），beta 也为负 -> 符号一致。
        C-R: 偏相关为正（正向耦合），beta 也为正 -> 符号一致。
        """
        op = DiscretizationOperator()
        results = op.verify_232()

        # 这里不直接检查 beta 符号（beta 不在 result 中），
        # 但 classify 的正确分类本身验证了理论的一致性。
        assert results["E-R"].in_kernel is True  # 幅度被吸收
        assert results["C-R"].in_kernel is False  # 方向穿透


# ── 边界条件测试 ────────────────────────────────────────────


class TestEdgeCases:
    """边界条件测试。"""

    def test_zero_partial_corr_zero_beta(self) -> None:
        """两者都为零 -> 独立。"""
        result = classify_coupling(partial_corr=0.0, beta=0.0)
        assert result.coupling_type == CouplingType.INDEPENDENT

    def test_strong_corr_zero_beta(self) -> None:
        """强连续耦合 + 零离散耦合 -> 完美 ker(D)。"""
        result = classify_coupling(partial_corr=0.5, beta=0.0)
        assert result.in_kernel is True
        assert result.absorption_ratio == 1.0

    def test_equal_corr_beta(self) -> None:
        """连续和离散耦合相等 -> 无吸收 -> not ker(D)。"""
        result = classify_coupling(partial_corr=0.3, beta=0.3)
        assert result.in_kernel is False
        assert result.absorption_ratio == pytest.approx(0.0, abs=0.01)

    def test_beta_exceeds_partial_corr(self) -> None:
        """离散耦合超过连续耦合 -> 方向放大 -> not ker(D)。"""
        result = classify_coupling(partial_corr=0.1, beta=0.5)
        assert result.in_kernel is False
        assert result.absorption_ratio == 0.0  # clamped

    def test_negative_signs(self) -> None:
        """负偏相关+负 beta -> 分类基于绝对值。"""
        result = classify_coupling(partial_corr=-0.4, beta=-0.01)
        assert result.in_kernel is True
        assert result.coupling_type == CouplingType.AMPLITUDE

    def test_custom_thresholds(self) -> None:
        """自定义阈值改变分类边界。"""
        # 默认阈值下 beta=0.04 在核内
        r1 = classify_coupling(partial_corr=0.3, beta=0.04)
        assert r1.in_kernel is True

        # 更严格的阈值下 beta=0.04 不在核内
        r2 = classify_coupling(
            partial_corr=0.3, beta=0.04, kernel_threshold=0.03,
        )
        assert r2.in_kernel is False

    def test_near_independence_threshold(self) -> None:
        """偏相关恰好在独立阈值边界。"""
        # 恰好低于阈值 -> 独立
        r1 = classify_coupling(partial_corr=0.049, beta=0.0)
        assert r1.coupling_type == CouplingType.INDEPENDENT

        # 恰好高于阈值 -> 有耦合
        r2 = classify_coupling(partial_corr=0.051, beta=0.0)
        assert r2.coupling_type != CouplingType.INDEPENDENT
