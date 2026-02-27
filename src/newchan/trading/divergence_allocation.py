"""背驰力度与仓位分配量化。

文档三 §八 的实现。

核心公式：
    d(e) = 1 - A_后 / A_前          （背驰力度比）
    w_alloc(e_j) = W * d(e_j)^gamma / sum(d(e_i)^gamma)  （分配函数）
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Sequence


def divergence_ratio(a_prev: float, a_curr: float) -> float:
    """计算背驰力度比 d(e) = 1 - A_后 / A_前。

    Parameters
    ----------
    a_prev : float
        前段推动的 MACD 面积（A_前），必须 > 0。
    a_curr : float
        后段推动的 MACD 面积（A_后），必须 >= 0。

    Returns
    -------
    float
        d(e) in (0, 1)。a_curr >= a_prev 时返回 0（无背驰）。

    Raises
    ------
    ValueError
        a_prev <= 0。
    """
    if a_prev <= 0:
        raise ValueError(f"a_prev must be > 0, got {a_prev}")
    if a_curr < 0:
        raise ValueError(f"a_curr must be >= 0, got {a_curr}")
    ratio = 1.0 - a_curr / a_prev
    return max(ratio, 0.0)


def allocate(
    total_position: float,
    divergence_ratios: Sequence[float],
    gamma: float,
) -> list[float]:
    """将总仓位按背驰力度分配到多个标的。

    公式：w_alloc(e_j) = W * d(e_j)^gamma / sum(d(e_i)^gamma)

    Parameters
    ----------
    total_position : float
        总仓位 W（来自仓位确定性函数）。
    divergence_ratios : Sequence[float]
        各标的的背驰力度比 d(e_i)，每个值 in [0, 1)。
    gamma : float
        集中度参数，gamma >= 1。

    Returns
    -------
    list[float]
        各标的的分配仓位，sum = total_position。

    Raises
    ------
    ValueError
        divergence_ratios 为空、gamma < 1、或所有 d 为 0。
    """
    if not divergence_ratios:
        raise ValueError("divergence_ratios must not be empty")
    if gamma < 1:
        raise ValueError(f"gamma must be >= 1, got {gamma}")

    powered = [d ** gamma for d in divergence_ratios]
    total_power = sum(powered)

    if total_power == 0:
        raise ValueError(
            "all divergence ratios are 0 — cannot allocate"
        )

    return [total_position * p / total_power for p in powered]


@dataclass(frozen=True, slots=True)
class GammaByLayer:
    """各层级的集中度参数 gamma。

    文档三 §八.4：
    - L1（跨角分配）：gamma_1 = 1.5，角间分散化有价值
    - L2（角内标的分配）：gamma_2 = 2.0，角内风险同源，集中到最确定的
    - L3（标的内加仓分配）：gamma_3 = 1.0，同标的不同级别买点互补
    """

    l1: float = 1.5
    l2: float = 2.0
    l3: float = 1.0

    def __post_init__(self) -> None:
        for name, val in [("l1", self.l1), ("l2", self.l2), ("l3", self.l3)]:
            if val < 1:
                raise ValueError(f"{name} must be >= 1, got {val}")
