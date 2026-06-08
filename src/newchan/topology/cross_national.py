"""跨国 K4 展开 — 多经济体资本流转结构（254号定义2 / CROSS_NATIONAL 边实装）。

概念溯源：254号多经济体资本流转本体论、292号折叠区间套顺序、528号折叠通道重构。

## $ 展开为结算尺空间 Σ，拉动 K4 整体共展开（254号定义2）

单经济体 K4 = (P, C, R, M)。当 M（货币资本 / $）展开为结算尺空间 Σ
（法定货币群 ∪ 黄金）时，整个 K4 随之共展开为 n 簇：

  - 每经济体一个截面 K4_i = (P_i, C_i, R_i, M_i)

## 边的三类（254号 + ontology-v2-push.md §二）

对 n 个经济体：

  - **国内边**：6n 条（每经济体 K4 内的 6 条边）
  - **货币边**：n(n−1)/2 条（M_i–M_j，汇率/结算尺之间，= 254号层0）
  - **跨国同类边**：3·n(n−1)/2 条（X_i–X_j，X ∈ {P,C,R}，同类资产跨经济体）

  独立自由度定理：4n − 1（4n 个节点的生成树边数）。

## 折叠区间套顺序在跨国下展开（292号§3.4）

共享折叠约束特有折叠：
  Au（全局共享，所有经济体共用同一黄金折叠）> Oil（半共享，石油美元可撕毁）
  > Bond（经济体特有，各国主权信用独立）> RE（地区特有）。

跨国边的折叠相位由 fold_channel.py 的折叠通道在各截面读出——Au 折叠全局共享意味着
所有经济体的 C_i↔M_i 折叠通道由同一黄金价格锚定。
"""

from __future__ import annotations

from dataclasses import dataclass
from itertools import combinations

from newchan.topology.graph import Edge, EdgeType, Vertex, _ALL_EDGES

# 非货币资本顶点（参与跨国同类边的资产类型）
_ASSET_VERTICES: tuple[Vertex, ...] = (Vertex.P, Vertex.C, Vertex.R)


@dataclass(frozen=True, slots=True)
class NationalVertex:
    """带经济体标签的 K4 顶点。

    Attributes
    ----------
    vertex : Vertex
        资本形态（M/P/C/R）。
    economy : str
        经济体标识（如 "US", "CN", "EU", "JP"）。
    """

    vertex: Vertex
    economy: str

    @property
    def label(self) -> str:
        """人类可读标签，如 P@US。"""
        return f"{self.vertex.name}@{self.economy}"


@dataclass(frozen=True, slots=True)
class NationalEdge:
    """跨/内经济体的比价边。

    Attributes
    ----------
    node_a : NationalVertex
        一端。
    node_b : NationalVertex
        另一端。
    edge_type : EdgeType
        边类型：INDEPENDENT/DERIVED（国内）或 CROSS_NATIONAL（跨国/货币边）。
    """

    node_a: NationalVertex
    node_b: NationalVertex
    edge_type: EdgeType

    @property
    def label(self) -> str:
        """人类可读标签，如 P@US/P@CN。"""
        return f"{self.node_a.label}/{self.node_b.label}"

    @property
    def is_currency_edge(self) -> bool:
        """是否为货币边（M_i–M_j，汇率/结算尺之间，254号层0）。"""
        return (
            self.edge_type is EdgeType.CROSS_NATIONAL
            and self.node_a.vertex is Vertex.M
            and self.node_b.vertex is Vertex.M
        )


@dataclass(frozen=True, slots=True)
class MultiEconomyGraph:
    """n 经济体共展开的 K4 结构（254号定义2）。

    Attributes
    ----------
    economies : tuple[str, ...]
        参与的经济体标识。
    domestic_edges : tuple[NationalEdge, ...]
        国内边（6n 条）。
    currency_edges : tuple[NationalEdge, ...]
        货币边（n(n−1)/2 条，M_i–M_j）。
    cross_national_edges : tuple[NationalEdge, ...]
        跨国同类边（3·n(n−1)/2 条）。
    """

    economies: tuple[str, ...]
    domestic_edges: tuple[NationalEdge, ...]
    currency_edges: tuple[NationalEdge, ...]
    cross_national_edges: tuple[NationalEdge, ...]

    @property
    def n(self) -> int:
        """经济体数量。"""
        return len(self.economies)

    @property
    def node_count(self) -> int:
        """总节点数（4n）。"""
        return 4 * self.n

    @property
    def all_edges(self) -> tuple[NationalEdge, ...]:
        """全部边（国内 + 货币 + 跨国同类）。"""
        return self.domestic_edges + self.currency_edges + self.cross_national_edges

    @property
    def independent_dof(self) -> int:
        """独立自由度（254号定理：4n − 1）。"""
        return 4 * self.n - 1


def build_multi_economy_graph(economies: tuple[str, ...]) -> MultiEconomyGraph:
    """构造 n 经济体共展开的 K4 结构（254号定义2）。

    Parameters
    ----------
    economies : tuple[str, ...]
        经济体标识序列，至少 1 个，不可重复。

    Returns
    -------
    MultiEconomyGraph

    Raises
    ------
    ValueError
        如果经济体序列为空或含重复。
    """
    if not economies:
        raise ValueError("至少需要 1 个经济体")
    if len(set(economies)) != len(economies):
        raise ValueError(f"经济体标识不可重复：{economies}")

    # 国内边：每经济体复制单体 K4 的 6 条边
    domestic: list[NationalEdge] = []
    for eco in economies:
        for e in _ALL_EDGES:
            domestic.append(
                NationalEdge(
                    node_a=NationalVertex(e.vertex_a, eco),
                    node_b=NationalVertex(e.vertex_b, eco),
                    edge_type=e.edge_type,
                )
            )

    # 货币边 + 跨国同类边：经济体两两组合
    currency: list[NationalEdge] = []
    cross: list[NationalEdge] = []
    for eco_i, eco_j in combinations(economies, 2):
        # 货币边 M_i–M_j
        currency.append(
            NationalEdge(
                node_a=NationalVertex(Vertex.M, eco_i),
                node_b=NationalVertex(Vertex.M, eco_j),
                edge_type=EdgeType.CROSS_NATIONAL,
            )
        )
        # 跨国同类边 X_i–X_j（X ∈ {P, C, R}）
        for v in _ASSET_VERTICES:
            cross.append(
                NationalEdge(
                    node_a=NationalVertex(v, eco_i),
                    node_b=NationalVertex(v, eco_j),
                    edge_type=EdgeType.CROSS_NATIONAL,
                )
            )

    return MultiEconomyGraph(
        economies=tuple(economies),
        domestic_edges=tuple(domestic),
        currency_edges=tuple(currency),
        cross_national_edges=tuple(cross),
    )
