"""比价走势到资本流转方向推断。

语义映射层：将管线产出的比价走势结构映射为资本流转语义。

概念溯源：
  [旧缠论:隐含] 比价走势语义（从第9课"资金流向"推出）
  [旧缠论] 第9课："比价关系的变动，也可以构成一个买卖系统，
    这个买卖系统是和市场资金的流向相关的"

规范引用：ratio_relation_v1.md §1.2 语义映射表

语义映射表（ratio_relation_v1.md §1.2）：
  比价一笔向上 → A 相对于 B 升值 = 资本从 B 流向 A
  比价一笔向下 → A 相对于 B 贬值 = 资本从 A 流向 B
  比价中枢     → A、B 之间资本流转暂时均衡
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from newchan.a_stroke import Stroke
from newchan.equivalence import EquivalencePair


# ====================================================================
# 枚举：资本流转方向
# ====================================================================


class FlowDirection(Enum):
    """资本流转方向。

    概念溯源：[旧缠论:隐含] 比价走势语义
    """

    A_TO_B = "A→B"  # 资本从 A 流向 B（比价下跌）
    B_TO_A = "B→A"  # 资本从 B 流向 A（比价上涨）
    EQUILIBRIUM = "均衡"  # 中枢区间内，暂时均衡


# ====================================================================
# 数据类型：单笔资本流转
# ====================================================================


@dataclass(frozen=True, slots=True)
class StrokeFlow:
    """单笔资本流转。

    Attributes
    ----------
    stroke_index : int
        笔在序列中的位置（从 0 开始）。
    direction : FlowDirection
        流转方向。
    sym_from : str
        资本来源标的。
    sym_to : str
        资本去向标的。
    magnitude : float
        比价变动幅度（abs(p1 - p0) / p0）。
    """

    stroke_index: int
    direction: FlowDirection
    sym_from: str
    sym_to: str
    magnitude: float


# ====================================================================
# 核心映射函数
# ====================================================================


def _resolve_flow_direction(
    pair: EquivalencePair,
    stroke_direction: str,
) -> tuple[FlowDirection, str, str]:
    """根据笔方向解析资本流转方向和来源/去向标的。"""
    if stroke_direction == "up":
        return FlowDirection.B_TO_A, pair.sym_b, pair.sym_a
    return FlowDirection.A_TO_B, pair.sym_a, pair.sym_b


def _map_stroke(
    pair: EquivalencePair,
    stroke: Stroke,
    index: int,
) -> StrokeFlow:
    """将单根比价笔映射为资本流转语义。"""
    if stroke.p0 == 0.0:
        raise ValueError(
            f"stroke[{index}].p0 is zero — magnitude undefined "
            f"(division by zero)"
        )

    direction, sym_from, sym_to = _resolve_flow_direction(
        pair, stroke.direction,
    )
    magnitude = abs(stroke.p1 - stroke.p0) / abs(stroke.p0)

    return StrokeFlow(
        stroke_index=index,
        direction=direction,
        sym_from=sym_from,
        sym_to=sym_to,
        magnitude=magnitude,
    )


def strokes_to_flows(
    pair: EquivalencePair,
    strokes: list[Stroke],
) -> list[StrokeFlow]:
    """将比价笔序列映射为资本流转序列。

    概念溯源：[旧缠论:隐含] 比价走势语义（第9课"资金流向"推出）
    规范引用：ratio_relation_v1.md §1.2

    Parameters
    ----------
    pair : EquivalencePair
        等价对，提供 sym_a / sym_b 标的名。
    strokes : list[Stroke]
        在比价K线上构造的笔序列。

    Returns
    -------
    list[StrokeFlow]
        与输入笔序列等长的资本流转序列。

    Raises
    ------
    ValueError
        当任意笔的 p0 == 0 时。
    """
    return [
        _map_stroke(pair, stroke, index)
        for index, stroke in enumerate(strokes)
    ]
