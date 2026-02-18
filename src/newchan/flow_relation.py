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
from typing import Literal

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


# ====================================================================
# 现金边信号消歧（026号谱系）
# ====================================================================

# 纯资产边方向值最大方差（3 个值取自 {-1, 0, +1} 的总体方差上界）。
# 极端分化组合如 [+1, +1, -1]：均值 = 1/3，方差 = 8/9。
_MAX_ASSET_VARIANCE: float = 8.0 / 9.0

# 分化比率阈值：超过此值判定为 genuine_flow，低于（且 > 0）为 mixed。
_DIVERGENCE_THRESHOLD: float = 0.5


@dataclass(frozen=True, slots=True)
class CashSignalAnalysis:
    """现金边信号消歧分析结果。

    概念溯源：[新缠论] 026号谱系——现金角双重身份
    用纯资产子图（3 条无货币因子的边）校验现金边信号的可信度。

    Attributes
    ----------
    cash_net_flow : int
        CASH 顶点的 net_flow（正=汇，负=源）。
    asset_subgraph_variance : float
        纯资产子图 3 条边方向值的总体方差（0=全均衡或全同向，高=有分化）。
    signal_type : Literal["genuine_flow", "metric_shift", "mixed", "neutral"]
        消歧后的信号类型。
    confidence : float
        置信度（0.0 ~ 1.0）。
    """

    cash_net_flow: int
    asset_subgraph_variance: float
    signal_type: Literal["genuine_flow", "metric_shift", "mixed", "neutral"]
    confidence: float


def _direction_value(direction: FlowDirection) -> int:
    """将 FlowDirection 映射为数值：A_TO_B → +1, B_TO_A → -1, EQUILIBRIUM → 0。"""
    if direction == FlowDirection.A_TO_B:
        return 1
    if direction == FlowDirection.B_TO_A:
        return -1
    return 0


def _population_variance(values: list[int]) -> float:
    """计算总体方差（除以 N，非 N-1）。"""
    n = len(values)
    if n == 0:
        return 0.0
    mean = sum(values) / n
    return sum((v - mean) ** 2 for v in values) / n


def _is_cash_edge(edge: EdgeFlowInput) -> bool:
    """判断边是否为现金边（至少有一个端点是 CASH）。"""
    return edge.vertex_a == AssetVertex.CASH or edge.vertex_b == AssetVertex.CASH


def disambiguate_cash_signal(
    edge_inputs: list[EdgeFlowInput],
) -> CashSignalAnalysis:
    """现金边信号消歧：用纯资产子图校验现金边信号的可信度。

    概念溯源：[新缠论] 026号谱系
    推导链：
      1. 现金边信号 = 资产运动 + 货币运动（混合）
      2. 纯资产边消去了货币因子（纯信号）
      3. 对比两者可消歧：分化 → 真实流转，均衡 → 度量变动

    Parameters
    ----------
    edge_inputs : list[EdgeFlowInput]
        恰好 6 条边的流转方向输入。

    Returns
    -------
    CashSignalAnalysis
        消歧分析结果。

    Raises
    ------
    ValueError
        边数不等于 6，或存在重复边（由 aggregate_vertex_flows 抛出）。
    """
    # 委托给 aggregate_vertex_flows 做输入验证和顶点聚合
    states = aggregate_vertex_flows(edge_inputs)

    # 1. 提取 CASH 的 net_flow
    cash_net = 0
    for s in states:
        if s.vertex == AssetVertex.CASH:
            cash_net = s.net_flow
            break

    # 2. 如果 CASH 无信号 → neutral
    if cash_net == 0:
        return CashSignalAnalysis(
            cash_net_flow=0,
            asset_subgraph_variance=0.0,
            signal_type="neutral",
            confidence=1.0,
        )

    # 3. 提取纯资产边的方向值
    asset_directions: list[int] = [
        _direction_value(e.direction)
        for e in edge_inputs
        if not _is_cash_edge(e)
    ]

    # 4. 计算纯资产子图方差
    variance = _population_variance(asset_directions)

    # 5. 分类
    if variance == 0.0:
        # 纯资产边无分化 → 度量基准变动
        return CashSignalAnalysis(
            cash_net_flow=cash_net,
            asset_subgraph_variance=0.0,
            signal_type="metric_shift",
            confidence=1.0,
        )

    ratio = min(variance / _MAX_ASSET_VARIANCE, 1.0)

    if ratio > _DIVERGENCE_THRESHOLD:
        # 纯资产边有明显分化 → 真实流转
        return CashSignalAnalysis(
            cash_net_flow=cash_net,
            asset_subgraph_variance=variance,
            signal_type="genuine_flow",
            confidence=ratio,
        )

    # 中间情况
    return CashSignalAnalysis(
        cash_net_flow=cash_net,
        asset_subgraph_variance=variance,
        signal_type="mixed",
        confidence=0.5,
    )
