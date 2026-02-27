"""共振定义和度量。

概念溯源：
  [新缠论] levels-bsp-v2 §五：共振的精确定义和度量

共振 = 多条比价线在各自支配级别上同时出现方向一致的买卖点。
共振强度映射到仓位确定性。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from newchan.nesting.bsp import BSP


class ResonanceLevel(Enum):
    """共振层次分类。

    §5.4:
    - THREE_LAYER: 配置层+独立边+底层（满仓，ResStrength >= 6）
    - TWO_LAYER: 配置层+独立边 或 独立边+底层（重仓，ResStrength in [3,6)）
    - SINGLE: 单层信号（轻仓，ResStrength in [1,3)）
    """

    THREE_LAYER = "three_layer"
    TWO_LAYER = "two_layer"
    SINGLE = "single"


class SignalLayer(Enum):
    """信号所在层。

    §5.3: 不同层的权重：
    - CONFIG: w=3（配置层，全局方向确认）
    - INDEPENDENT_EDGE: w=2（独立边，角级别方向确认）
    - DERIVED_EDGE: w=1（派生边，一致性增强但不独立）
    - UNDERLYING: w=1（底层标的，局部确认）
    """

    CONFIG = "config"
    INDEPENDENT_EDGE = "independent_edge"
    DERIVED_EDGE = "derived_edge"
    UNDERLYING = "underlying"


_LAYER_WEIGHTS: dict[SignalLayer, float] = {
    SignalLayer.CONFIG: 3.0,
    SignalLayer.INDEPENDENT_EDGE: 2.0,
    SignalLayer.DERIVED_EDGE: 1.0,
    SignalLayer.UNDERLYING: 1.0,
}


@dataclass(frozen=True, slots=True)
class ResonanceSignal:
    """共振信号。"""

    edge_id: str  # 比价线标识
    level: int  # 支配级别
    bsp: BSP  # 买卖点
    layer: SignalLayer  # 信号所在层
    time: float  # 买卖点时刻


def _weight(layer: SignalLayer) -> float:
    return _LAYER_WEIGHTS[layer]


def _all_same_direction(signals: list[ResonanceSignal]) -> bool:
    """R2: 所有买卖点方向一致。"""
    if len(signals) <= 1:
        return True
    first_is_buy = signals[0].bsp.bsp_type.is_buy
    return all(s.bsp.bsp_type.is_buy == first_is_buy for s in signals)


def _time_overlap(
    signals: list[ResonanceSignal],
    time_tolerance_fn: callable,
) -> bool:
    """R3: 时间重叠。

    |ti - tj| <= Delta(ki, kj) for all i, j。
    Delta 由调用方提供的 time_tolerance_fn(level_i, level_j) 确定。
    """
    for i in range(len(signals)):
        for j in range(i + 1, len(signals)):
            delta = time_tolerance_fn(signals[i].level, signals[j].level)
            if abs(signals[i].time - signals[j].time) > delta:
                return False
    return True


def resonance_check(
    signals: list[ResonanceSignal],
    time_tolerance_fn: callable,
) -> bool:
    """共振条件检查。

    §5.1 三个条件全部满足：
    (R1) 每条线在各自支配级别上都有买卖点
    (R2) 所有买卖点方向一致
    (R3) 时间重叠

    Parameters
    ----------
    signals : list[ResonanceSignal]
        参与共振检查的信号列表。
    time_tolerance_fn : callable
        (level_i, level_j) -> float，返回两个级别间的时间容差。

    Returns
    -------
    bool
        True 表示共振成立。
    """
    if len(signals) < 2:
        return False

    # R1: 每条线都有买卖点
    if any(not s.bsp.bsp_type.is_present for s in signals):
        return False

    # R2: 方向一致
    if not _all_same_direction(signals):
        return False

    # R3: 时间重叠
    return _time_overlap(signals, time_tolerance_fn)


def resonance_strength(signals: list[ResonanceSignal]) -> float:
    """共振强度。

    §5.3: ResStrength = sum_i w(e_i, k_i) * I(BSP(e_i, k_i, t) != empty)

    Parameters
    ----------
    signals : list[ResonanceSignal]
        参与共振的信号列表。

    Returns
    -------
    float
        共振强度值。
    """
    return sum(
        _weight(s.layer)
        for s in signals
        if s.bsp.bsp_type.is_present
    )


def classify_resonance(strength: float) -> ResonanceLevel:
    """根据共振强度分类。

    §5.4:
    - ResStrength >= 6 -> THREE_LAYER（满仓）
    - ResStrength in [3, 6) -> TWO_LAYER（重仓）
    - ResStrength in [1, 3) -> SINGLE（轻仓）

    Parameters
    ----------
    strength : float
        共振强度值。

    Returns
    -------
    ResonanceLevel
        共振层次。
    """
    if strength >= 6.0:
        return ResonanceLevel.THREE_LAYER
    if strength >= 3.0:
        return ResonanceLevel.TWO_LAYER
    return ResonanceLevel.SINGLE
