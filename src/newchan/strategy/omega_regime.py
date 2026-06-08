"""ω regime 检测——金油比驱动的宏观信用周期判断。

ω = 金价/油价（gold/oil ratio）。
ω 上升 → 美元信用收缩 → 大宗商品做多机会 / 美股谨慎
ω 下降 → 美元信用扩张 → 美股做多机会 / 大宗商品谨慎

理论基础（卢麒元框架）：
  L = M × ω 与 GDP 协整。空转 = MCM' 对 P 的寄生。
  ω 是资本三流的温度计。

认识论标注：L2（2025-02-20 regime 断点已在真实数据中确认）。
谱系引用：用户交易方向记忆——押注金油比下降（油涨），超大级别周线线段一买。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

import numpy as np


class OmegaRegime(Enum):
    """ω regime 状态。

    BULL_EQUITY:     ω 下降（信用扩张）→ 美股做多
    BULL_COMMODITY:  ω 上升（信用收缩）→ 大宗做多
    NEUTRAL:         ω 震荡无方向
    """

    BULL_EQUITY = "bull_equity"
    BULL_COMMODITY = "bull_commodity"
    NEUTRAL = "neutral"


@dataclass(frozen=True, slots=True)
class OmegaState:
    """ω 的完整状态快照。

    Attributes
    ----------
    omega : float
        当前 ω 值（gold_price / oil_price）。
    regime : OmegaRegime
        当前 regime 判断。
    omega_ma_short : float
        短期均线值（用于趋势判断）。
    omega_ma_long : float
        长期均线值（用于趋势判断）。
    strength : float
        regime 强度（短长均线偏离率，绝对值越大越确定）。
    """

    omega: float
    regime: OmegaRegime
    omega_ma_short: float
    omega_ma_long: float
    strength: float


def compute_omega(gold_price: float, oil_price: float) -> float:
    """计算 ω = gold / oil。

    Raises
    ------
    ValueError
        oil_price <= 0 时。
    """
    if oil_price <= 0.0:
        raise ValueError(f"oil_price must be positive, got {oil_price}")
    return gold_price / oil_price


def detect_regime(
    omega_series: np.ndarray,
    short_window: int = 20,
    long_window: int = 60,
    threshold: float = 0.02,
) -> OmegaState:
    """从 ω 时间序列检测当前 regime。

    方法：双均线偏离。短期均线在长期均线之上 → ω 上升 → BULL_COMMODITY。
    偏离率绝对值小于 threshold → NEUTRAL。

    Parameters
    ----------
    omega_series : np.ndarray
        ω 的时间序列（从旧到新）。长度必须 >= long_window。
    short_window : int
        短期均线窗口（默认 20）。
    long_window : int
        长期均线窗口（默认 60）。
    threshold : float
        中性区域阈值（偏离率绝对值 < threshold → NEUTRAL）。

    Returns
    -------
    OmegaState
        包含 regime 判断和强度的完整快照。

    Raises
    ------
    ValueError
        序列长度不足 long_window 时。
    """
    if len(omega_series) < long_window:
        raise ValueError(
            f"omega_series length {len(omega_series)} "
            f"< long_window {long_window}"
        )

    current_omega = float(omega_series[-1])
    ma_short = float(np.mean(omega_series[-short_window:]))
    ma_long = float(np.mean(omega_series[-long_window:]))

    if ma_long == 0.0:
        return OmegaState(
            omega=current_omega,
            regime=OmegaRegime.NEUTRAL,
            omega_ma_short=ma_short,
            omega_ma_long=ma_long,
            strength=0.0,
        )

    deviation = (ma_short - ma_long) / ma_long

    if deviation > threshold:
        regime = OmegaRegime.BULL_COMMODITY
    elif deviation < -threshold:
        regime = OmegaRegime.BULL_EQUITY
    else:
        regime = OmegaRegime.NEUTRAL

    return OmegaState(
        omega=current_omega,
        regime=regime,
        omega_ma_short=ma_short,
        omega_ma_long=ma_long,
        strength=deviation,
    )


def regime_from_walk_direction(omega_direction: str) -> OmegaRegime:
    """从缠论 D-operator 的走势方向直接推断 regime。

    适用于已有缠论管线处理 ω 比价的场景——
    将 ω 视为 EquivalencePair("GOLD", "OIL") 跑比价走势分析，
    最终笔/线段方向即为 regime。

    Parameters
    ----------
    omega_direction : str
        "up"（ω 上升）或 "down"（ω 下降）。

    Returns
    -------
    OmegaRegime
    """
    if omega_direction == "up":
        return OmegaRegime.BULL_COMMODITY
    if omega_direction == "down":
        return OmegaRegime.BULL_EQUITY
    return OmegaRegime.NEUTRAL
