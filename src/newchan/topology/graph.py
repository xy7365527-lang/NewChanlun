"""K4 完全图结构 — 四顶点六边的资本容器拓扑。

概念溯源：ontology-v2-push.md §一

四个顶点 E(动产), C(商品), R(不动产), $(现金) 构成 K4 完全图。
6 条边分为：
  - 3 条独立边：E/$, C/$, R/$ （连接 CASH 的边）
  - 3 条派生边：E/C, E/R, C/R （纯资产边）

K4 包含 4 个 K3 子图（三角形）：
  - ECR, EC$, ER$, CR$
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from itertools import combinations
from typing import FrozenSet, Tuple


class Vertex(Enum):
    """资本容器的四个顶点。"""

    E = "动产"       # Equity: 股票、基金
    C = "商品"       # Commodity: 大宗、黄金
    R = "不动产"     # Real estate: 房地产
    CASH = "现金"    # Cash: 主权信用/流动性复合体


class EdgeType(Enum):
    """边的类型。"""

    INDEPENDENT = "独立"   # 连接 CASH 的边，构成配置空间的自由度
    DERIVED = "派生"       # 纯资产边，可由独立边代数推导
    CROSS_NATIONAL = "跨国同类"  # 不同经济体同类资产间的边


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
        """人类可读标签，如 E/$。"""
        return f"{self.vertex_a.name}/{self.vertex_b.name}"

    @property
    def vertices(self) -> FrozenSet[Vertex]:
        """边连接的两个顶点（无序集合）。"""
        return frozenset({self.vertex_a, self.vertex_b})


# ── 标准 K4 的 6 条边 ──────────────────────────────────────────

# 3 条独立边：连接 CASH
_INDEPENDENT_EDGES: tuple[Edge, ...] = (
    Edge(Vertex.E, Vertex.CASH, EdgeType.INDEPENDENT),
    Edge(Vertex.C, Vertex.CASH, EdgeType.INDEPENDENT),
    Edge(Vertex.R, Vertex.CASH, EdgeType.INDEPENDENT),
)

# 3 条派生边：纯资产
_DERIVED_EDGES: tuple[Edge, ...] = (
    Edge(Vertex.E, Vertex.C, EdgeType.DERIVED),
    Edge(Vertex.E, Vertex.R, EdgeType.DERIVED),
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
        """返回 3 条独立边（连接 CASH）。"""
        return _INDEPENDENT_EDGES

    def get_derived_edges(self) -> tuple[Edge, ...]:
        """返回 3 条派生边（纯资产边）。"""
        return _DERIVED_EDGES

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
            键为三角形标签（如 "ECR"），值为三条边。
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
