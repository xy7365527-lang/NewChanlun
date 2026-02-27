"""配置空间 — K4 的 27 种配置及极性指数。

概念溯源：ontology-v2-push.md §三

配置 Gamma = (sigma(E/$), sigma(C/$), sigma(R/$))
每个 sigma 属于 {+1, 0, -1}，共 3^3 = 27 种配置。

极性指数 S = sum(s(sigma(e)))，其中 s(+) = +1, s(0) = 0, s(-) = -1。
值域 {-3, -2, -1, 0, +1, +2, +3}。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import IntEnum
from typing import Iterator, Literal


class WalkDirection(IntEnum):
    """独立边的走势方向。

    +1 = 上涨（资产相对现金升值）
     0 = 盘整
    -1 = 下跌（资产相对现金贬值）
    """

    UP = 1
    FLAT = 0
    DOWN = -1


@dataclass(frozen=True, slots=True)
class Configuration:
    """K4 配置：三条独立边的状态三元组。

    Attributes
    ----------
    sigma_e : WalkDirection
        E/$ 边状态。
    sigma_c : WalkDirection
        C/$ 边状态。
    sigma_r : WalkDirection
        R/$ 边状态。
    """

    sigma_e: WalkDirection
    sigma_c: WalkDirection
    sigma_r: WalkDirection

    @property
    def as_tuple(self) -> tuple[int, int, int]:
        """以 (int, int, int) 形式返回。"""
        return (self.sigma_e.value, self.sigma_c.value, self.sigma_r.value)

    @property
    def zero_count(self) -> int:
        """盘整分量个数。"""
        return sum(1 for s in self.as_tuple if s == 0)

    @property
    def node_type(self) -> Literal["corner", "edge", "face", "center"]:
        """节点在 P3^3 中的类型。

        corner: 0 个零分量（8 个角节点）
        edge: 1 个零分量（12 个棱节点）
        face: 2 个零分量（6 个面节点）
        center: 3 个零分量（1 个中心节点）
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
    def degree(self) -> int:
        """在转换拓扑图中的度数（邻居数量）。

        corner: 3, edge: 4, face: 5, center: 6
        """
        count = 0
        for s in self.as_tuple:
            if s == 0:
                count += 2  # 0 可以向 + 或 - 突破
            else:
                count += 1  # + 或 - 只能退化为 0
        return count

    @property
    def label(self) -> str:
        """人类可读标签，如 (+,0,-)。"""
        symbols = {1: "+", 0: "0", -1: "-"}
        return "({},{},{})".format(*(symbols[s] for s in self.as_tuple))

    def __repr__(self) -> str:
        return f"Configuration{self.label}"


def polarity_index(config: Configuration) -> int:
    """计算极性指数 S。

    S = s(sigma_E) + s(sigma_C) + s(sigma_R)
    其中 s(+) = +1, s(0) = 0, s(-) = -1。

    Parameters
    ----------
    config : Configuration
        配置。

    Returns
    -------
    int
        极性指数 S，值域 {-3, -2, -1, 0, +1, +2, +3}。
    """
    return config.sigma_e.value + config.sigma_c.value + config.sigma_r.value


def _pos(val: int) -> int:
    """将 {-1, 0, +1} 映射为 {0, 1, 2} 用于距离计算。"""
    return val + 1


def manhattan_distance(c1: Configuration, c2: Configuration) -> int:
    """两个配置之间的曼哈顿距离（= 最短路径长度）。

    d(c1, c2) = sum_k |pos_k(c1) - pos_k(c2)|
    其中 pos 将 {-1, 0, +1} 映射为 {0, 1, 2}。

    Parameters
    ----------
    c1, c2 : Configuration
        两个配置。

    Returns
    -------
    int
        最短路径长度。
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
        for e in WalkDirection:
            for c in WalkDirection:
                for r in WalkDirection:
                    cfg = Configuration(sigma_e=e, sigma_c=c, sigma_r=r)
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

    def get(self, e: int, c: int, r: int) -> Configuration:
        """按数值获取配置。

        Parameters
        ----------
        e, c, r : int
            各分量值，必须在 {-1, 0, +1} 内。

        Returns
        -------
        Configuration

        Raises
        ------
        KeyError
            如果 (e, c, r) 不在合法范围内。
        """
        key = (e, c, r)
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
