"""折叠生命周期监控 -- 折叠通道状态标记 + 信号质量注解。

292号谱系 §4 实现：
  折叠有生命周期——不是静态分类，是动态过程。
  四阶段：稳定期 / 松动期 / 重新结算期 / 未结算期。
  每个阶段有不同的 D 算子产出签名和信号质量含义。

352号 §8 下游推论3：
  "等价类的折叠通道标记可接入300号的折叠生命周期测度论签名。
   如果某个折叠通道处于重新结算期，该等价类的信号质量应标注为可疑。"

认识论标注：
  - 四阶段定义 + 信号质量映射规则：L0（从292号定义推导）
  - 实际阶段判定（哪个通道在哪个阶段）：L2（需真实数据 D 算子签名验证）

谱系引用：292号折叠拓扑本体论、300号测度论与折叠统一、352号扫描器设计。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from newchan.trading.fold_equivalence import EquivalenceClass, FoldChannel


# ═══════════════════════════════════════════════════════════════
# 折叠生命周期四阶段（292号 §4.1）
# ═══════════════════════════════════════════════════════════════


class FoldLifecyclePhase(Enum):
    """折叠生命周期四阶段。

    292号 §4.1 定义：
    - STABLE: 折叠通道畅通，资本在两端自由流转
    - LOOSENING: 投影结构分歧，折叠通道开始淤塞
    - RESETTLING: 旧折叠解体，新折叠尚未稳定
    - UNSETTLED: 折叠尚未结晶（如 BTC）
    """

    STABLE = "stable"
    LOOSENING = "loosening"
    RESETTLING = "resettling"
    UNSETTLED = "unsettled"


# ═══════════════════════════════════════════════════════════════
# 信号质量（从生命周期阶段推导）
# ═══════════════════════════════════════════════════════════════


class SignalQuality(Enum):
    """信号质量注解。

    映射规则（352号 §8 下游推论3）：
    - RELIABLE: 折叠通道稳定，信号可信
    - CAUTION: 折叠通道松动，信号需谨慎使用
    - SUSPECT: 折叠通道重新结算中或未结算，信号可疑
    """

    RELIABLE = "reliable"
    CAUTION = "caution"
    SUSPECT = "suspect"


def annotate_signal_quality(phase: FoldLifecyclePhase) -> SignalQuality:
    """从生命周期阶段推导信号质量。

    映射：
    - STABLE → RELIABLE
    - LOOSENING → CAUTION
    - RESETTLING → SUSPECT
    - UNSETTLED → SUSPECT

    Parameters
    ----------
    phase : FoldLifecyclePhase
        折叠通道当前生命周期阶段。

    Returns
    -------
    SignalQuality
        信号质量注解。
    """
    _MAPPING = {
        FoldLifecyclePhase.STABLE: SignalQuality.RELIABLE,
        FoldLifecyclePhase.LOOSENING: SignalQuality.CAUTION,
        FoldLifecyclePhase.RESETTLING: SignalQuality.SUSPECT,
        FoldLifecyclePhase.UNSETTLED: SignalQuality.SUSPECT,
    }
    return _MAPPING[phase]


# ═══════════════════════════════════════════════════════════════
# 折叠通道状态
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class FoldLifecycleState:
    """单个折叠通道的生命周期状态。

    Attributes
    ----------
    channel : FoldChannel
        折叠通道。
    phase : FoldLifecyclePhase
        当前生命周期阶段。
    """

    channel: FoldChannel
    phase: FoldLifecyclePhase


def fold_lifecycle_state(
    channel: FoldChannel,
    phase: FoldLifecyclePhase,
) -> FoldLifecycleState:
    """构造折叠通道生命周期状态。

    Parameters
    ----------
    channel : FoldChannel
        折叠通道。
    phase : FoldLifecyclePhase
        当前生命周期阶段。

    Returns
    -------
    FoldLifecycleState
    """
    return FoldLifecycleState(channel=channel, phase=phase)


# ═══════════════════════════════════════════════════════════════
# 等价类信号质量标注
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class AnnotatedEquivalenceClass:
    """带信号质量标注的等价类。

    Attributes
    ----------
    equivalence_class : EquivalenceClass
        原始等价类。
    phase : FoldLifecyclePhase
        折叠通道生命周期阶段。
    quality : SignalQuality
        信号质量注解。
    """

    equivalence_class: EquivalenceClass
    phase: FoldLifecyclePhase
    quality: SignalQuality


def annotate_equivalence_class(
    ec: EquivalenceClass,
    lifecycle_states: dict[FoldChannel, FoldLifecycleState],
) -> AnnotatedEquivalenceClass:
    """为等价类标注信号质量。

    如果等价类的折叠通道在 lifecycle_states 中有状态记录，
    使用该状态推导信号质量。否则默认 STABLE / RELIABLE。

    Parameters
    ----------
    ec : EquivalenceClass
        待标注的等价类。
    lifecycle_states : dict[FoldChannel, FoldLifecycleState]
        各折叠通道的当前生命周期状态。

    Returns
    -------
    AnnotatedEquivalenceClass
        带信号质量标注的等价类。
    """
    state = lifecycle_states.get(ec.fold_channel)
    if state is not None:
        phase = state.phase
    else:
        phase = FoldLifecyclePhase.STABLE
    quality = annotate_signal_quality(phase)
    return AnnotatedEquivalenceClass(
        equivalence_class=ec,
        phase=phase,
        quality=quality,
    )
