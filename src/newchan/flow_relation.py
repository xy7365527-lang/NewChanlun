"""流转关系：四矩阵上的有向流场。

定义编号：#14（liuzhuan.md）
概念层级：跨标的维度·拓扑层（L2）

概念溯源：
  [旧缠论] 第9课："比价关系的变动…和市场资金的流向相关的"
  [新缠论] 流转关系 = 四矩阵有向流场（025号谱系）

核心操作：
  1. 每条边有一个 FlowDirection（来自比价走势当前笔方向）
  2. 对每个顶点 V，聚合其 3 条关联边的方向 → net(V)
  3. |net(V)| ≥ 2 → 共振（V 是流转源或汇）
  4. Σnet(V) = 0 → 守恒约束（拓扑不变量）
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from newchan.capital_flow import FlowDirection
from newchan.matrix_topology import AssetVertex


# ====================================================================
# 数据结构
# ====================================================================


@dataclass(frozen=True, slots=True)
class EdgeFlowInput:
    """一条边的当前流转方向输入。

    vertex_a/vertex_b 的顺序与 FlowDirection 的语义一致：
      A_TO_B = 资本从 vertex_a 流向 vertex_b
      B_TO_A = 资本从 vertex_b 流向 vertex_a
    """

    vertex_a: AssetVertex
    vertex_b: AssetVertex
    direction: FlowDirection

    def __post_init__(self) -> None:
        if self.vertex_a == self.vertex_b:
            raise ValueError(f"自环不合法：{self.vertex_a}")


class ResonanceStrength(Enum):
    """共振强度。"""

    NONE = "无共振"  # |net| ≤ 1
    WEAK = "弱共振"  # |net| = 2
    STRONG = "强共振"  # |net| = 3


@dataclass(frozen=True, slots=True)
class VertexFlowState:
    """顶点流转状态。

    Attributes
    ----------
    vertex : AssetVertex
        顶点。
    net_flow : int
        净流量。正 = 净流入（汇），负 = 净流出（源）。
    strength : ResonanceStrength
        共振强度。
    """

    vertex: AssetVertex
    net_flow: int
    strength: ResonanceStrength


# ====================================================================
# 核心函数
# ====================================================================


def _flow_contribution(edge: EdgeFlowInput, vertex: AssetVertex) -> int:
    """计算一条边对给定顶点的 flow 贡献。

    Returns +1（流入 vertex）、-1（流出 vertex）或 0（均衡）。
    """
    if edge.direction == FlowDirection.EQUILIBRIUM:
        return 0
    if edge.direction == FlowDirection.A_TO_B:
        # 资本从 A → B
        if vertex == edge.vertex_b:
            return +1  # 流入 vertex
        if vertex == edge.vertex_a:
            return -1  # 流出 vertex
        return 0  # 不关联
    # B_TO_A: 资本从 B → A
    if vertex == edge.vertex_a:
        return +1  # 流入 vertex
    if vertex == edge.vertex_b:
        return -1  # 流出 vertex
    return 0  # 不关联


def _classify_resonance(net: int) -> ResonanceStrength:
    """从 net flow 判定共振强度。"""
    abs_net = abs(net)
    if abs_net >= 3:
        return ResonanceStrength.STRONG
    if abs_net >= 2:
        return ResonanceStrength.WEAK
    return ResonanceStrength.NONE


def aggregate_vertex_flows(
    edges: list[EdgeFlowInput],
) -> list[VertexFlowState]:
    """聚合 6 条边的方向为 4 个顶点的流转状态。

    Parameters
    ----------
    edges : list[EdgeFlowInput]
        恰好 6 条边的流转方向输入。

    Returns
    -------
    list[VertexFlowState]
        4 个顶点的流转状态，按 AssetVertex 枚举顺序排列。

    Raises
    ------
    ValueError
        边数不等于 6，或存在重复边。
    """
    if len(edges) != 6:
        raise ValueError(f"需要恰好 6 条边，实际 {len(edges)} 条")

    # 检查重复
    seen: set[frozenset[AssetVertex]] = set()
    for e in edges:
        key = frozenset([e.vertex_a, e.vertex_b])
        if key in seen:
            raise ValueError(f"重复边：{e.vertex_a.value}/{e.vertex_b.value}")
        seen.add(key)

    states: list[VertexFlowState] = []
    for vertex in AssetVertex:
        net = sum(_flow_contribution(e, vertex) for e in edges)
        states.append(
            VertexFlowState(
                vertex=vertex,
                net_flow=net,
                strength=_classify_resonance(net),
            )
        )
    return states


def detect_resonance(
    states: list[VertexFlowState],
) -> list[VertexFlowState]:
    """返回所有顶点的流转状态（含共振强度标注）。

    这是一个透传函数——aggregate_vertex_flows 已经计算了 strength。
    保留此函数作为公开 API 以匹配定义文件中的接口契约。
    """
    return states


def check_conservation(states: list[VertexFlowState]) -> bool:
    """检查守恒约束：Σnet(V) = 0。

    这是拓扑不变量——对任何合法输入恒为 True。
    返回 False 意味着输入或计算有误。
    """
    return sum(s.net_flow for s in states) == 0
