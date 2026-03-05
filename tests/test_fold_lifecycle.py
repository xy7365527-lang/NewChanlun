"""折叠生命周期监控测试。

验证：
1. 折叠通道四阶段状态标记
2. 信号质量注解（基于生命周期阶段）
3. 等价类信号质量标注
4. 边界条件：未知阶段、空输入

认识论等级：L0（从292号定义推导）+ L2（四阶段签名需真实数据验证）。
谱系引用：292号折叠拓扑本体论、300号测度论与折叠统一、352号扫描器设计§8下游推论3。
"""

from __future__ import annotations

import pytest

from newchan.trading.fold_equivalence import (
    DTriState,
    EquivalenceClass,
    FoldChannel,
    TargetAttributes,
    build_quotient_space,
)
from newchan.trading.fold_lifecycle import (
    FoldLifecyclePhase,
    FoldLifecycleState,
    SignalQuality,
    annotate_equivalence_class,
    annotate_signal_quality,
    fold_lifecycle_state,
)


# ═══════════════════════════════════════════════════════════════
# Fixtures
# ═══════════════════════════════════════════════════════════════


def _target(
    symbol: str,
    fold: FoldChannel,
    *,
    tightness: float = 2.0,
    liquidity: float = 100.0,
) -> TargetAttributes:
    return TargetAttributes(
        symbol=symbol,
        fold_channel=fold,
        sector="",
        d_tri_state=DTriState.RETAIN,
        tightness=tightness,
        level_magnitude=1.0,
        liquidity=liquidity,
    )


# ═══════════════════════════════════════════════════════════════
# FoldLifecyclePhase
# ═══════════════════════════════════════════════════════════════


class TestFoldLifecyclePhase:
    """验证四阶段枚举。"""

    def test_four_phases_exist(self) -> None:
        """292号 §4.1 定义四阶段。"""
        assert len(FoldLifecyclePhase) == 4
        assert FoldLifecyclePhase.STABLE.value == "stable"
        assert FoldLifecyclePhase.LOOSENING.value == "loosening"
        assert FoldLifecyclePhase.RESETTLING.value == "resettling"
        assert FoldLifecyclePhase.UNSETTLED.value == "unsettled"


# ═══════════════════════════════════════════════════════════════
# fold_lifecycle_state
# ═══════════════════════════════════════════════════════════════


class TestFoldLifecycleState:
    """验证从通道和阶段构造生命周期状态。"""

    def test_stable_au(self) -> None:
        state = fold_lifecycle_state(FoldChannel.AU, FoldLifecyclePhase.STABLE)
        assert state.channel is FoldChannel.AU
        assert state.phase is FoldLifecyclePhase.STABLE

    def test_loosening_oil(self) -> None:
        state = fold_lifecycle_state(FoldChannel.OIL, FoldLifecyclePhase.LOOSENING)
        assert state.channel is FoldChannel.OIL
        assert state.phase is FoldLifecyclePhase.LOOSENING

    def test_frozen(self) -> None:
        """FoldLifecycleState 是不可变的。"""
        state = fold_lifecycle_state(FoldChannel.AU, FoldLifecyclePhase.STABLE)
        with pytest.raises(AttributeError):
            state.phase = FoldLifecyclePhase.LOOSENING  # type: ignore[misc]


# ═══════════════════════════════════════════════════════════════
# annotate_signal_quality
# ═══════════════════════════════════════════════════════════════


class TestAnnotateSignalQuality:
    """验证信号质量注解规则。"""

    def test_stable_is_reliable(self) -> None:
        """稳定期 → 信号可靠。"""
        quality = annotate_signal_quality(FoldLifecyclePhase.STABLE)
        assert quality is SignalQuality.RELIABLE

    def test_loosening_is_caution(self) -> None:
        """松动期 → 信号需谨慎。"""
        quality = annotate_signal_quality(FoldLifecyclePhase.LOOSENING)
        assert quality is SignalQuality.CAUTION

    def test_resettling_is_suspect(self) -> None:
        """重新结算期 → 信号可疑。"""
        quality = annotate_signal_quality(FoldLifecyclePhase.RESETTLING)
        assert quality is SignalQuality.SUSPECT

    def test_unsettled_is_suspect(self) -> None:
        """未结算期 → 信号可疑。"""
        quality = annotate_signal_quality(FoldLifecyclePhase.UNSETTLED)
        assert quality is SignalQuality.SUSPECT


# ═══════════════════════════════════════════════════════════════
# annotate_equivalence_class
# ═══════════════════════════════════════════════════════════════


class TestAnnotateEquivalenceClass:
    """验证等价类的信号质量标注。"""

    def test_stable_channel_annotated(self) -> None:
        """稳定期通道的等价类标注为 RELIABLE。"""
        targets = (
            _target("GLD", FoldChannel.AU, tightness=3.0),
            _target("GC=F", FoldChannel.AU, tightness=2.0),
        )
        eqs = build_quotient_space(targets)
        states = {
            FoldChannel.AU: fold_lifecycle_state(
                FoldChannel.AU, FoldLifecyclePhase.STABLE
            ),
        }
        annotated = annotate_equivalence_class(eqs[0], states)
        assert annotated.quality is SignalQuality.RELIABLE
        assert annotated.phase is FoldLifecyclePhase.STABLE

    def test_resettling_channel_annotated_suspect(self) -> None:
        """重新结算期通道标注为 SUSPECT。"""
        targets = (_target("TLT", FoldChannel.BOND, tightness=2.0),)
        eqs = build_quotient_space(targets)
        states = {
            FoldChannel.BOND: fold_lifecycle_state(
                FoldChannel.BOND, FoldLifecyclePhase.RESETTLING
            ),
        }
        annotated = annotate_equivalence_class(eqs[0], states)
        assert annotated.quality is SignalQuality.SUSPECT

    def test_unknown_channel_defaults_reliable(self) -> None:
        """未在 states 字典中的通道默认 RELIABLE。"""
        targets = (_target("XOM", FoldChannel.EQUITY, tightness=2.0),)
        eqs = build_quotient_space(targets)
        annotated = annotate_equivalence_class(eqs[0], {})
        assert annotated.quality is SignalQuality.RELIABLE
        assert annotated.phase is FoldLifecyclePhase.STABLE

    def test_annotated_carries_original_class(self) -> None:
        """标注结果携带原始等价类。"""
        targets = (_target("GLD", FoldChannel.AU, tightness=3.0),)
        eqs = build_quotient_space(targets)
        annotated = annotate_equivalence_class(eqs[0], {})
        assert annotated.equivalence_class is eqs[0]
