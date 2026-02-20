"""四矩阵拓扑管理 — 资本容器的比价边结构。

概念溯源：
  [新缠论] 四矩阵拓扑 — 引用卢麒元框架 + ontology-v1 命题5
  规范来源：ratio_relation_v1.md §3

四个资本容器顶点（动产、不动产、商品、现金）构成 C(4,2) = 6 条比价边。
每条边代表一对等价关系，承载资本流转语义。

本模块是**拓扑结构管理**，不是分析引擎。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from itertools import combinations

from newchan.equivalence import EquivalencePair


# ── 顶点枚举 ──────────────────────────────────────────────


class AssetVertex(Enum):
    """资本容器的四个顶点。"""

    EQUITY = "动产"  # 股票、基金
    REAL_ESTATE = "不动产"  # 房地产
    COMMODITY = "商品"  # 大宗、黄金
    # CASH: 主权信用/流动性复合体（Sovereign Credit / Liquidity Complex）
    # 包含现金(M0/M1)和固定收益/国债。货币即主权债务，区别仅在久期。
    # CASH 内部的现金/债券轮动属于 Level 2（内部结构），不在拓扑层处理。
    # 参见 047号谱系。
    CASH = "现金"


# ── 比价边 ────────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class MatrixEdge:
    """四矩阵中的一条比价边。"""

    vertex_a: AssetVertex
    vertex_b: AssetVertex
    representative_pair: EquivalencePair | None = None  # 代表性等价对
    description: str = ""  # 资本流转含义描述

    @property
    def label(self) -> str:
        return f"{self.vertex_a.value}/{self.vertex_b.value}"


# ── 四矩阵 ────────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class FourMatrix:
    """四矩阵拓扑。

    管理 6 条比价边和它们的代表性等价对。
    """

    edges: tuple[MatrixEdge, ...]  # 6 条边
    region: str = ""  # 经济体/区域


# ── 全部边组合 ────────────────────────────────────────────

ALL_EDGES: tuple[tuple[AssetVertex, AssetVertex], ...] = tuple(
    combinations(AssetVertex, 2)
)
"""C(4,2) = 6 条边的全部组合。"""

# ── 默认比价边描述（ratio_relation_v1.md §3.2）──────────

_DEFAULT_DESCRIPTIONS: dict[frozenset[AssetVertex], str] = {
    frozenset([AssetVertex.EQUITY, AssetVertex.CASH]): "股市整体涨跌",
    frozenset([AssetVertex.REAL_ESTATE, AssetVertex.CASH]): "实际房价变化",
    frozenset([AssetVertex.COMMODITY, AssetVertex.CASH]): "实物通胀",
    frozenset([AssetVertex.EQUITY, AssetVertex.REAL_ESTATE]): "金融资产 vs 实物资产偏好",
    frozenset([AssetVertex.EQUITY, AssetVertex.COMMODITY]): "金融资产 vs 避险资产",
    frozenset([AssetVertex.COMMODITY, AssetVertex.REAL_ESTATE]): "实物之间的配置偏好",
}


# ── 工厂函数 ──────────────────────────────────────────────


def make_default_matrix(region: str = "") -> FourMatrix:
    """创建默认四矩阵（6 条边，无代表性等价对，含默认描述）。

    Parameters
    ----------
    region : str
        经济体/区域标识，默认空字符串。

    Returns
    -------
    FourMatrix
        包含 6 条边的默认拓扑，每条边带有 §3.2 定义的资本流转含义描述。
    """
    edges = tuple(
        MatrixEdge(
            vertex_a=a,
            vertex_b=b,
            description=_DEFAULT_DESCRIPTIONS.get(frozenset([a, b]), ""),
        )
        for a, b in ALL_EDGES
    )
    return FourMatrix(edges=edges, region=region)


def _update_edge(
    edge: MatrixEdge,
    target: frozenset[AssetVertex],
    pair: EquivalencePair,
) -> tuple[MatrixEdge, bool]:
    """如果 edge 匹配 target，返回更新后的边和 True；否则原样返回和 False。"""
    if frozenset([edge.vertex_a, edge.vertex_b]) == target:
        new_edge = MatrixEdge(
            vertex_a=edge.vertex_a,
            vertex_b=edge.vertex_b,
            representative_pair=pair,
            description=edge.description,
        )
        return new_edge, True
    return edge, False


def with_representative(
    matrix: FourMatrix,
    vertex_a: AssetVertex,
    vertex_b: AssetVertex,
    pair: EquivalencePair,
) -> FourMatrix:
    """不可变更新：为指定边设置代表性等价对。返回新 FourMatrix。

    Raises ValueError 当 vertex_a == vertex_b 或找不到对应边。
    """
    if vertex_a == vertex_b:
        raise ValueError(
            f"Same vertex on both ends: {vertex_a}. No self-loop edge exists."
        )

    target = frozenset([vertex_a, vertex_b])
    found = False
    new_edges: list[MatrixEdge] = []

    for edge in matrix.edges:
        new_edge, matched = _update_edge(edge, target, pair)
        if matched:
            found = True
        new_edges.append(new_edge)

    if not found:
        raise ValueError(
            f"No edge found between {vertex_a} and {vertex_b} in this matrix."
        )

    return FourMatrix(edges=tuple(new_edges), region=matrix.region)
