"""K4 完全图结构 — 四顶点六边的资本容器拓扑（折叠通道模型）。

概念溯源：254号多经济体资本流转本体论、292号折叠拓扑本体论、330号 K4 资本循环映射。

## 顶点 = 卢麒元资本流转四矩阵（330号）

四个顶点对应《资本论》第二卷资本循环的四种资本形态：

  - M（货币资本 / $）：现金、主权信用。循环起点/终点/度量基准。
  - P（生产资本金融化 / Equity）：股票。对生产资本 P 的金融索取权，**不是 P 本身**。
  - C（商品资本）：大宗商品整体。物质投入 + 实物价值储备。
  - R（不动产）：固定资本容器。

> 命名变更（528号重构）：旧枚举 E/C/R/CASH 重命名为 P/C/R/M。所指（标的）不变——
> 股票仍是股票、$ 仍是 $——仅名称对齐 330号资本循环语义。M 既是顶点又是度量尺
> （026号 $ 双重身份）。

## 折叠通道不是顶点（292号 / 528号）

金（Au）和油（Oil）**不是独立顶点**，而是顶点之间的**折叠通道**——同一对象在不同
截面呈现不同范畴身份（不同相位），由数据结构 `FoldChannel` 承载（见 fold_channel.py）：

  - Au = C↔M 折叠通道（黄金既是商品又是货币替代，254号：黄金 ∈ Σ∩C）。
  - Oil = C→P 通道（油是产业循环 M→C→P 的物质投入，330号）。

> 528号失效模式：把折叠对象（金/油）压平成单顶点标签，导致回测（金=C）与监控（金=$）
> 读到相反顶点。折叠通道模型让"同一对象的两个相位"可表达，消除该冲突。

## 六条边

  - 3 条独立边（连接 M，配置空间自由度）：P/M, C/M, R/M
  - 3 条派生边（纯资产边，= 独立边之差）：P/C, P/R, C/R

K4 含 4 个 K3 子图（三角形）：PCR, PCM, PRM, CRM。

跨国展开（254号定义2、CROSS_NATIONAL 边类型）见 cross_national.py。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from itertools import combinations
from typing import FrozenSet


class Vertex(Enum):
    """资本容器的四个顶点 —— 资本流转四矩阵（330号）。"""

    M = "货币资本"   # Money capital / $: 现金、主权信用、度量基准
    P = "生产资本"   # Production capital (financialized) / Equity: 股票
    C = "商品资本"   # Commodity capital: 大宗商品整体
    R = "不动产"     # Real estate: 固定资本容器


class EdgeType(Enum):
    """边的类型。"""

    INDEPENDENT = "独立"   # 连接 M（货币资本）的边，构成配置空间的自由度
    DERIVED = "派生"       # 纯资产边，可由独立边代数推导
    CROSS_NATIONAL = "跨国同类"  # 不同经济体同类资产间的边（254号、cross_national.py）


@dataclass(frozen=True, slots=True)
class Edge:
    """K4 图中的一条比价边。

    Attributes
    ----------
    vertex_a : Vertex
        边的一端。
    vertex_b : Vertex
        边的另一端。
    edge_type : EdgeType
        边的类型（独立/派生/跨国同类）。
    """

    vertex_a: Vertex
    vertex_b: Vertex
    edge_type: EdgeType

    @property
    def label(self) -> str:
        """人类可读标签，如 P/M。"""
        return f"{self.vertex_a.name}/{self.vertex_b.name}"

    @property
    def vertices(self) -> FrozenSet[Vertex]:
        """边连接的两个顶点（无序集合）。"""
        return frozenset({self.vertex_a, self.vertex_b})


# ── 标准 K4 的 6 条边 ──────────────────────────────────────────

# 3 条独立边：连接 M（货币资本 / 度量基准）
_INDEPENDENT_EDGES: tuple[Edge, ...] = (
    Edge(Vertex.P, Vertex.M, EdgeType.INDEPENDENT),
    Edge(Vertex.C, Vertex.M, EdgeType.INDEPENDENT),
    Edge(Vertex.R, Vertex.M, EdgeType.INDEPENDENT),
)

# 3 条派生边：纯资产
_DERIVED_EDGES: tuple[Edge, ...] = (
    Edge(Vertex.P, Vertex.C, EdgeType.DERIVED),
    Edge(Vertex.P, Vertex.R, EdgeType.DERIVED),
    Edge(Vertex.C, Vertex.R, EdgeType.DERIVED),
)

_ALL_EDGES: tuple[Edge, ...] = _INDEPENDENT_EDGES + _DERIVED_EDGES


class K4Graph:
    """K4 完全图：4 顶点、6 条边。

    不可变。创建后只读。
    """

    __slots__ = ("_edges", "_vertices")

    def __init__(self) -> None:
        self._vertices: tuple[Vertex, ...] = tuple(Vertex)
        self._edges: tuple[Edge, ...] = _ALL_EDGES

    @property
    def vertices(self) -> tuple[Vertex, ...]:
        """全部 4 个顶点。"""
        return self._vertices

    @property
    def edges(self) -> tuple[Edge, ...]:
        """全部 6 条边。"""
        return self._edges

    @property
    def edge_count(self) -> int:
        """边数（恒为 6）。"""
        return len(self._edges)

    @property
    def vertex_count(self) -> int:
        """顶点数（恒为 4）。"""
        return len(self._vertices)

    def get_independent_edges(self) -> tuple[Edge, ...]:
        """返回 3 条独立边（连接 M）。"""
        return _INDEPENDENT_EDGES

    def get_derived_edges(self) -> tuple[Edge, ...]:
        """返回 3 条派生边（纯资产边）。"""
        return _DERIVED_EDGES

    def edge_between(self, v1: Vertex, v2: Vertex) -> Edge:
        """返回连接 v1 与 v2 的边。

        Parameters
        ----------
        v1, v2 : Vertex
            两个不同的顶点。

        Returns
        -------
        Edge
            连接这两个顶点的边。

        Raises
        ------
        ValueError
            如果 v1 == v2 或两顶点间无边（K4 中不会发生）。
        """
        if v1 == v2:
            raise ValueError(f"需要两个不同的顶点，收到：{v1}, {v2}")
        target = frozenset({v1, v2})
        for e in self._edges:
            if e.vertices == target:
                return e
        raise ValueError(f"K4 中找不到连接 {v1} 与 {v2} 的边")

    def get_triangle(
        self, v1: Vertex, v2: Vertex, v3: Vertex
    ) -> tuple[Edge, ...]:
        """返回由三个顶点构成的 K3 子图（三角形）的边。

        Parameters
        ----------
        v1, v2, v3 : Vertex
            三个不同的顶点。

        Returns
        -------
        tuple[Edge, ...]
            三条边。

        Raises
        ------
        ValueError
            如果顶点不是三个不同的值。
        """
        tri_vertices = {v1, v2, v3}
        if len(tri_vertices) != 3:
            raise ValueError(
                f"需要 3 个不同的顶点，收到：{v1}, {v2}, {v3}"
            )
        if not tri_vertices.issubset(set(Vertex)):
            raise ValueError(
                f"顶点必须是 Vertex 枚举值：{tri_vertices}"
            )

        tri_edges = tuple(
            e for e in self._edges if e.vertices.issubset(tri_vertices)
        )
        return tri_edges

    def get_all_triangles(self) -> dict[str, tuple[Edge, ...]]:
        """返回 K4 的全部 4 个 K3 子图。

        Returns
        -------
        dict[str, tuple[Edge, ...]]
            键为三角形标签（如 "PCR"），值为三条边。
        """
        result: dict[str, tuple[Edge, ...]] = {}
        for combo in combinations(self._vertices, 3):
            label = "".join(v.name for v in combo)
            result[label] = self.get_triangle(*combo)
        return result

    def edges_incident_to(self, vertex: Vertex) -> tuple[Edge, ...]:
        """返回与指定顶点关联的所有边。

        Parameters
        ----------
        vertex : Vertex
            目标顶点。

        Returns
        -------
        tuple[Edge, ...]
            与该顶点关联的边（K4 中每个顶点关联 3 条边）。
        """
        return tuple(
            e for e in self._edges
            if vertex in e.vertices
        )
