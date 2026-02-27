"""反向传递。

概念溯源：
  [新缠论] levels-bsp-v2 §三：反向传递形式化
  标的层卖点汇聚 -> 板块层力度衰竭 -> 角层力度衰竭 -> 配置层卖点

反向传递与正向下钻（区间套）的对偶：
  正向下钻回答：在哪里入场
  反向传递回答：什么时候开始退出
"""

from __future__ import annotations

from dataclasses import dataclass

from newchan.nesting.bsp import DivergenceType


@dataclass(frozen=True, slots=True)
class DivergenceRecord:
    """单条边/标的的背驰记录。"""

    entity_id: str  # 标的/板块/角标识
    level: int
    time: float
    divergence_type: DivergenceType
    weight: float = 1.0  # 在所属组中的权重


def div_count(
    records: list[DivergenceRecord],
    div_type: DivergenceType,
    level: int,
    time: float,
    time_tolerance: float = 0.0,
) -> int:
    """背驰计数。

    §3.1: DivCount(Group, k, t) = |{entity : D(entity, k, t) = div_type}|

    Parameters
    ----------
    records : list[DivergenceRecord]
        当前组内所有实体的背驰记录。
    div_type : DivergenceType
        要统计的背驰类型（TOP_DIV 或 BOT_DIV）。
    level : int
        递归层级。
    time : float
        时刻。
    time_tolerance : float
        时间容差（允许的时间偏差）。

    Returns
    -------
    int
        满足条件的背驰实体数量。
    """
    count = 0
    for r in records:
        if (
            r.divergence_type is div_type
            and r.level == level
            and abs(r.time - time) <= time_tolerance
        ):
            count += 1
    return count


def exhaustion_check(
    records: list[DivergenceRecord],
    div_type: DivergenceType,
    level: int,
    time: float,
    threshold: float,
    time_tolerance: float = 0.0,
) -> bool:
    """力度衰竭判定。

    §3.1: DivCount(Group, k, t) / n >= theta

    Parameters
    ----------
    records : list[DivergenceRecord]
        当前组内所有实体的背驰记录。
    div_type : DivergenceType
        背驰类型。
    level : int
        递归层级。
    time : float
        时刻。
    threshold : float
        汇聚阈值 theta。
    time_tolerance : float
        时间容差。

    Returns
    -------
    bool
        True 表示力度衰竭条件成立。
    """
    if len(records) == 0:
        return False
    total = len(records)
    count = div_count(records, div_type, level, time, time_tolerance)
    return count / total >= threshold


def threshold_lower_bound(weights: list[float]) -> float:
    """汇聚阈值下界。

    §3.2: theta 的下界 = min theta 使得背驰实体的权重总和 > 1/2。
    等权情况下 theta > 1/2。

    给定权重列表（降序排列效率最高），返回满足权重和 > 0.5 所需的最小比例。

    Parameters
    ----------
    weights : list[float]
        组内各实体的权重（正数，总和归一化为 1）。

    Returns
    -------
    float
        theta 的下界。如果权重为空返回 1.0（不可能满足）。
    """
    if len(weights) == 0:
        return 1.0

    total_weight = sum(weights)
    if total_weight <= 0:
        return 1.0

    # 归一化
    normalized = sorted((w / total_weight for w in weights), reverse=True)

    cumulative = 0.0
    for i, w in enumerate(normalized):
        cumulative += w
        if cumulative > 0.5:
            return (i + 1) / len(normalized)

    return 1.0
