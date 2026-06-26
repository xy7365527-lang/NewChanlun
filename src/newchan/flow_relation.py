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

注：Σnet(V) = 0 是 K4 反对称边流的图论恒等式（每条边对两端分别贡献
+1/-1，求和必为零），不是物理资本守恒。它在封闭系统内恒成立、破缺不可观测，
无诊断价值——故不提供守恒检查函数（050 号谱系结算，方案A；222/231 号：
有效域 ≠ 定义域）。
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
    level_id: int | None = None  # 层级标识，None=不区分层级（第一层分析）

    def __post_init__(self) -> None:
        if self.vertex_a == self.vertex_b:
            raise ValueError(f"自环不合法：{self.vertex_a}")


class ResonanceStrength(Enum):
    """共振强度。"""

    NONE = "无共振"  # |net| ≤ 1
    WEAK = "弱共振"  # |net| = 2
    STRONG = "强共振"  # |net| = 3


class FlowRole(Enum):
    """顶点流转角色。

    liuzhuan.md #14 §共振判定：
      net(V) ≤ -2 → V 是流转源（资本从 V 流出）
      net(V) ≥ +2 → V 是流转汇（资本流入 V）
      |net(V)| ≤ 1 → 无明确流转方向
    """

    SOURCE = "源"   # net ≤ -2
    SINK = "汇"     # net ≥ +2
    NEUTRAL = "中性"  # |net| ≤ 1


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
    role : FlowRole
        流转角色（源/汇/中性）。
    """

    vertex: AssetVertex
    net_flow: int
    strength: ResonanceStrength
    role: FlowRole


@dataclass(frozen=True, slots=True)
class FlowRelation:
    """流转关系：有向、一对多。

    liuzhuan.md #14 §流转关系：
      当顶点 V 为流转源（net(V) ≤ -2），且顶点 W₁, W₂, ... 为流转汇
      （net(Wᵢ) ≥ +2）时，存在流转关系 Flow(V → {W₁, W₂, ...})。

    Attributes
    ----------
    source : AssetVertex
        流转源顶点。
    sinks : frozenset[AssetVertex]
        流转汇顶点集合。可以为空（有源无汇的情况——见边界条件）。
    """

    source: AssetVertex
    sinks: frozenset[AssetVertex]


# ====================================================================
# 核心函数
# ====================================================================


def _flow_contribution(edge: EdgeFlowInput, vertex: AssetVertex) -> int:
    """计算一条边对给定顶点的 flow 贡献。

    Returns +1（流入 vertex）、-1（流出 vertex）或 0（均衡/不关联）。

    Raises
    ------
    ValueError
        direction 为 UNKNOWN 或未识别的枚举值。
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
    if edge.direction == FlowDirection.B_TO_A:
        # 资本从 B → A
        if vertex == edge.vertex_a:
            return +1  # 流入 vertex
        if vertex == edge.vertex_b:
            return -1  # 流出 vertex
        return 0  # 不关联
    raise ValueError(
        f"不可聚合的方向：{edge.direction.value}（{edge.vertex_a.value}/{edge.vertex_b.value}）"
    )


def _classify_resonance(net: int) -> ResonanceStrength:
    """从 net flow 判定共振强度。"""
    abs_net = abs(net)
    if abs_net >= 3:
        return ResonanceStrength.STRONG
    if abs_net >= 2:
        return ResonanceStrength.WEAK
    return ResonanceStrength.NONE


def _classify_role(net: int) -> FlowRole:
    """从 net flow 判定流转角色。

    liuzhuan.md #14 §共振判定：
      net ≤ -2 → SOURCE
      net ≥ +2 → SINK
      |net| ≤ 1 → NEUTRAL
    """
    if net <= -2:
        return FlowRole.SOURCE
    if net >= 2:
        return FlowRole.SINK
    return FlowRole.NEUTRAL


def aggregate_vertex_flows(
    edges: list[EdgeFlowInput],
) -> list[VertexFlowState]:
    """聚合 1-6 条边的方向为 4 个顶点的流转状态。

    Parameters
    ----------
    edges : list[EdgeFlowInput]
        1 到 6 条边的流转方向输入。部分图（<6 条边）在第二层级别扫描中
        是常态——并非所有比价对都有已完成的走势。

    Returns
    -------
    list[VertexFlowState]
        4 个顶点的流转状态，按 AssetVertex 枚举顺序排列。

    Raises
    ------
    ValueError
        边数为 0 或超过 6，存在重复边，或任何边的方向为 UNKNOWN。
        UNKNOWN 边不应参与聚合——它们应在上游级别扫描阶段被过滤掉。
    """
    if not edges or len(edges) > 6:
        raise ValueError(f"需要 1-6 条边，实际 {len(edges)} 条")

    # UNKNOWN 边 fail-fast
    for e in edges:
        if e.direction == FlowDirection.UNKNOWN:
            raise ValueError(
                f"UNKNOWN 边不允许参与聚合：{e.vertex_a.value}/{e.vertex_b.value}。"
                f"调用链上游应在级别扫描阶段过滤掉未完成走势的边"
            )

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
                role=_classify_role(net),
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


def extract_flow_relations(
    states: list[VertexFlowState],
) -> list[FlowRelation]:
    """从顶点流转状态中提取流转关系 Flow(源→汇)。

    liuzhuan.md #14 §流转关系：
      当顶点 V 为流转源（net(V) ≤ -2），且顶点 W₁, W₂, ... 为流转汇
      （net(Wᵢ) ≥ +2）时，存在流转关系 Flow(V → {W₁, W₂, ...})。

    Parameters
    ----------
    states : list[VertexFlowState]
        4 个顶点的流转状态（来自 aggregate_vertex_flows）。

    Returns
    -------
    list[FlowRelation]
        所有流转关系。每个源顶点产生一个 FlowRelation。
        如果没有共振源，返回空列表。
    """
    sinks = frozenset(s.vertex for s in states if s.role == FlowRole.SINK)
    return [
        FlowRelation(source=s.vertex, sinks=sinks)
        for s in states
        if s.role == FlowRole.SOURCE
    ]


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


def _classify_cash_signal(
    cash_net: int, variance: float,
) -> tuple[str, float]:
    """根据 cash_net 和纯资产子图方差分类信号。返回 (signal_type, confidence)。"""
    if cash_net == 0:
        return "neutral", 1.0
    if variance == 0.0:
        return "metric_shift", 1.0
    ratio = min(variance / _MAX_ASSET_VARIANCE, 1.0)
    if ratio > _DIVERGENCE_THRESHOLD:
        return "genuine_flow", ratio
    return "mixed", 0.5


def disambiguate_cash_signal(
    edge_inputs: list[EdgeFlowInput],
) -> CashSignalAnalysis:
    """现金边信号消歧：用纯资产子图校验现金边信号的可信度。

    要求恰好 6 条边（完整 K4 图）。消歧需要完整的纯资产三角形来判断
    "资产在动还是尺子在动"——部分图上做消歧会静默产出错误结果。
    K4 不完整时消歧不可执行。

    Raises
    ------
    ValueError
        边数不等于 6（通过 aggregate_vertex_flows 传递）。
    """
    if len(edge_inputs) != 6:
        raise ValueError(
            f"消歧需要完整 K4 图（6 条边），实际 {len(edge_inputs)} 条。"
            f"部分图上消歧会产出错误结果"
        )
    states = aggregate_vertex_flows(edge_inputs)

    cash_net = 0
    for s in states:
        if s.vertex == AssetVertex.CASH:
            cash_net = s.net_flow
            break

    asset_directions: list[int] = [
        _direction_value(e.direction)
        for e in edge_inputs
        if not _is_cash_edge(e)
    ]
    variance = _population_variance(asset_directions)
    signal_type, confidence = _classify_cash_signal(cash_net, variance)

    return CashSignalAnalysis(
        cash_net_flow=cash_net,
        asset_subgraph_variance=variance if cash_net != 0 else 0.0,
        signal_type=signal_type,
        confidence=confidence,
    )
