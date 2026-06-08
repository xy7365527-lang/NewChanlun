"""配置空间 — K4 的 27 种配置及极性指数。

概念溯源：254号、330号、527号 σ 本体论、528号折叠通道重构。

## 配置 Γ = (σ_P, σ_C, σ_R)

三条独立边（连接 M = 货币资本 = 度量基准）的走势方向三元组：
  - σ_P = σ(P/M)：生产资本（股票）相对货币资本的走势方向
  - σ_C = σ(C/M)：商品资本相对货币资本的走势方向
  - σ_R = σ(R/M)：不动产相对货币资本的走势方向

每个 σ ∈ {+1, 0, -1}，共 3^3 = 27 种配置。

> **σ_M 不参与配置**（026号 $ 双重身份）：M（货币资本）既是顶点又是度量尺。
> 三条独立边都以 M 为分母，所以 M 是基准而非第四个自由度。Γ 只有 3 个分量。
>
> 命名变更（528号）：旧 σ_E（动产）→ σ_P（生产资本金融化）。所指（股票/$ 边）不变。

## σ = 走势方向，可直接跳变（527号定理）

WalkDirection 是**走势方向态**，不是价格位置态。依"走势必完美"（24/29课一级权威）：
下跌走势 settle 后可**直接**转上涨走势，盘整（FLAT）**不是必经中间态**。
故 σ 可 −1 → +1 不经 0。配置转换拓扑因此是 81 边图（见 transition.py），
**不是** 54 边的"价格位置态投影"。

极性指数 S = sum(s(σ(e)))，s(+)=+1, s(0)=0, s(-)=-1。值域 {-3,...,+3}。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import IntEnum
from typing import Iterator, Literal


class WalkDirection(IntEnum):
    """独立边的走势方向（527号：走势方向态，可直接跳变，非价格位置态）。

    +1 = 上涨走势（资产相对货币资本升值）
     0 = 盘整走势
    -1 = 下跌走势（资产相对货币资本贬值）

    依"走势必完美"，−1 ↔ +1 可不经 0 直接转换（见 transition.py 81 边图）。
    """

    UP = 1
    FLAT = 0
    DOWN = -1


@dataclass(frozen=True, slots=True)
class Configuration:
    """K4 配置：三条独立边的走势方向三元组 Γ = (σ_P, σ_C, σ_R)。

    Attributes
    ----------
    sigma_p : WalkDirection
        P/M 边状态（生产资本 / 货币资本）。
    sigma_c : WalkDirection
        C/M 边状态（商品资本 / 货币资本）。
    sigma_r : WalkDirection
        R/M 边状态（不动产 / 货币资本）。
    """

    sigma_p: WalkDirection
    sigma_c: WalkDirection
    sigma_r: WalkDirection

    @property
    def as_tuple(self) -> tuple[int, int, int]:
        """以 (int, int, int) 形式返回。"""
        return (self.sigma_p.value, self.sigma_c.value, self.sigma_r.value)

    @property
    def zero_count(self) -> int:
        """盘整分量个数。"""
        return sum(1 for s in self.as_tuple if s == 0)

    @property
    def node_type(self) -> Literal["corner", "edge", "face", "center"]:
        """节点在 3^3 配置立方中的位置类型（按盘整分量个数）。

        corner: 0 个零分量（8 个角节点）
        edge: 1 个零分量（12 个棱节点）
        face: 2 个零分量（6 个面节点）
        center: 3 个零分量（1 个中心节点）

        注：此分类描述配置在 {-1,0,+1}^3 立方中的几何位置，与配置转换拓扑
        （81 边图，527号）的度数无关——后者每个节点度数恒为 6。
        """
        z = self.zero_count
        if z == 0:
            return "corner"
        if z == 1:
            return "edge"
        if z == 2:
            return "face"
        return "center"

    @property
    def label(self) -> str:
        """人类可读标签，如 (+,0,-)。"""
        symbols = {1: "+", 0: "0", -1: "-"}
        return "({},{},{})".format(*(symbols[s] for s in self.as_tuple))

    def __repr__(self) -> str:
        return f"Configuration{self.label}"


def polarity_index(config: Configuration) -> int:
    """计算极性指数 S。

    S = s(σ_P) + s(σ_C) + s(σ_R)，其中 s(+)=+1, s(0)=0, s(-)=-1。

    Parameters
    ----------
    config : Configuration
        配置。

    Returns
    -------
    int
        极性指数 S，值域 {-3, -2, -1, 0, +1, +2, +3}。
    """
    return config.sigma_p.value + config.sigma_c.value + config.sigma_r.value


def hamming_distance(c1: Configuration, c2: Configuration) -> int:
    """配置转换拓扑（81 边图，527号）中两配置的距离 = 走势方向不同的分量个数。

    因 σ 是走势方向态、可 −1↔+1 直接跳变（527号），任一分量改变只算一步，
    与价格位置无关。这是配置转换拓扑的正确距离度量。值域 {0, 1, 2, 3}。

    Parameters
    ----------
    c1, c2 : Configuration
        两个配置。

    Returns
    -------
    int
        Hamming 距离（不同分量个数）。
    """
    return sum(1 for a, b in zip(c1.as_tuple, c2.as_tuple) if a != b)


def _pos(val: int) -> int:
    """将 {-1, 0, +1} 映射为 {0, 1, 2} 用于价格位置投影距离计算。"""
    return val + 1


def manhattan_distance(c1: Configuration, c2: Configuration) -> int:
    """价格位置态投影（54 边图，527号）中两配置的距离。

    ⚠ 这**不是**配置转换距离（那是 hamming_distance）。曼哈顿距离假设 σ 是有连续序
    的价格位置态（涨→平→跌必经中间），属于 527号裁定的"价格位置态投影"对象——
    一个合法的纯数学度量，但不描述走势方向的配置转换。保留供位置投影分析使用。

    d(c1, c2) = sum_k |pos_k(c1) - pos_k(c2)|，pos 将 {-1,0,+1} 映射为 {0,1,2}。

    Parameters
    ----------
    c1, c2 : Configuration
        两个配置。

    Returns
    -------
    int
        价格位置投影下的曼哈顿距离。
    """
    return sum(
        abs(_pos(a) - _pos(b))
        for a, b in zip(c1.as_tuple, c2.as_tuple)
    )


class ConfigurationSpace:
    """K4 配置空间：27 种配置的完整枚举。

    不可变。创建后只读。
    """

    __slots__ = ("_configs", "_by_tuple")

    def __init__(self) -> None:
        configs: list[Configuration] = []
        by_tuple: dict[tuple[int, int, int], Configuration] = {}
        for p in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    cfg = Configuration(sigma_p=p, sigma_c=c, sigma_r=r)
                    configs.append(cfg)
                    by_tuple[cfg.as_tuple] = cfg
        self._configs: tuple[Configuration, ...] = tuple(configs)
        self._by_tuple: dict[tuple[int, int, int], Configuration] = by_tuple

    @property
    def size(self) -> int:
        """配置总数（恒为 27）。"""
        return len(self._configs)

    def __len__(self) -> int:
        return self.size

    def __iter__(self) -> Iterator[Configuration]:
        return iter(self._configs)

    def __contains__(self, config: Configuration) -> bool:
        return config.as_tuple in self._by_tuple

    def get(self, p: int, c: int, r: int) -> Configuration:
        """按数值获取配置。

        Parameters
        ----------
        p, c, r : int
            各分量值（σ_P, σ_C, σ_R），必须在 {-1, 0, +1} 内。

        Returns
        -------
        Configuration

        Raises
        ------
        KeyError
            如果 (p, c, r) 不在合法范围内。
        """
        key = (p, c, r)
        if key not in self._by_tuple:
            raise KeyError(f"非法配置：{key}，每个分量必须在 {{-1, 0, +1}} 内")
        return self._by_tuple[key]

    def configs_by_type(
        self, node_type: Literal["corner", "edge", "face", "center"]
    ) -> tuple[Configuration, ...]:
        """按节点类型筛选配置。

        Parameters
        ----------
        node_type : str
            "corner" (8), "edge" (12), "face" (6), "center" (1)。

        Returns
        -------
        tuple[Configuration, ...]
        """
        return tuple(c for c in self._configs if c.node_type == node_type)

    def configs_by_polarity(self, s: int) -> tuple[Configuration, ...]:
        """按极性指数筛选配置。

        Parameters
        ----------
        s : int
            极性指数值，必须在 [-3, +3] 内。

        Returns
        -------
        tuple[Configuration, ...]
        """
        return tuple(c for c in self._configs if polarity_index(c) == s)

    @property
    def polarity_range(self) -> tuple[int, int]:
        """极性指数的值域。"""
        return (-3, 3)


# ── 特殊配置 ──────────────────────────────────────────────────

CENTER = Configuration(WalkDirection.FLAT, WalkDirection.FLAT, WalkDirection.FLAT)
"""中心节点 (0,0,0)：全局盘整态。"""

FULL_RISK_ON = Configuration(WalkDirection.UP, WalkDirection.UP, WalkDirection.UP)
"""(+,+,+)：全面 risk-on。"""

FULL_RISK_OFF = Configuration(WalkDirection.DOWN, WalkDirection.DOWN, WalkDirection.DOWN)
"""(-,-,-)：全面 risk-off。"""
