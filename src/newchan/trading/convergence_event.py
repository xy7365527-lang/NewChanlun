"""收敛事件产生器 -- T(S) 变化时产生事件。

350号下游推论3：
  "当 T(S) 发生变化（新级别背驰出现或旧级别背驰消失），
   可产生收敛事件用于实时监控。"

四种事件类型：
  - STARTED: T(S) 从 0 变为正值（区间套开始收敛）
  - DEEPENED: T(S) 增大（新级别背驰出现，收敛加深）
  - WEAKENED: T(S) 减小但仍为正（旧级别背驰消失，收敛减弱）
  - LOST: T(S) 从正值变为 0（区间套收敛完全丧失）

认识论标注：L0（从350号收敛紧度函数定义直接推导）。
谱系引用：350号收敛紧度、352号多标的扫描器设计。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


# ═══════════════════════════════════════════════════════════════
# 收敛变化类型
# ═══════════════════════════════════════════════════════════════


class ConvergenceChangeType(Enum):
    """收敛事件类型（350号下游推论3）。"""

    STARTED = "started"       # T: 0 → positive
    DEEPENED = "deepened"     # T: positive → higher positive
    WEAKENED = "weakened"     # T: positive → lower positive (still > 0)
    LOST = "lost"             # T: positive → 0


# ═══════════════════════════════════════════════════════════════
# 收敛变化事件
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class ConvergenceChange:
    """收敛事件。

    Attributes
    ----------
    symbol : str
        标的代码。
    change_type : ConvergenceChangeType
        变化类型。
    prev_score : float
        变化前的 T(S) 值。
    new_score : float
        变化后的 T(S) 值。
    delta : float
        T(S) 变化量（new - prev）。
    """

    symbol: str
    change_type: ConvergenceChangeType
    prev_score: float
    new_score: float
    delta: float


# ═══════════════════════════════════════════════════════════════
# 事件检测
# ═══════════════════════════════════════════════════════════════


def detect_convergence_change(
    symbol: str,
    prev_score: float | None,
    new_score: float,
) -> ConvergenceChange | None:
    """检测 T(S) 变化并产生收敛事件。

    Parameters
    ----------
    symbol : str
        标的代码。
    prev_score : float | None
        前一次的 T(S) 值。None 表示首次计算（规范化为 0）。
    new_score : float
        本次计算的 T(S) 值。

    Returns
    -------
    ConvergenceChange | None
        有变化时返回事件，无变化返回 None。
    """
    prev = prev_score if prev_score is not None else 0.0

    if prev == new_score:
        return None

    if prev == 0.0 and new_score > 0.0:
        change_type = ConvergenceChangeType.STARTED
    elif prev > 0.0 and new_score == 0.0:
        change_type = ConvergenceChangeType.LOST
    elif new_score > prev:
        change_type = ConvergenceChangeType.DEEPENED
    else:
        change_type = ConvergenceChangeType.WEAKENED

    return ConvergenceChange(
        symbol=symbol,
        change_type=change_type,
        prev_score=prev,
        new_score=new_score,
        delta=new_score - prev,
    )
