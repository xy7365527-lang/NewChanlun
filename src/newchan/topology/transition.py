"""转换拓扑 — 配置之间的转换规则与路径分析（81 边图，527号定理）。

概念溯源：254号、527号 σ 本体论冲突结算（定理/吸收）、528号折叠通道重构。

## 配置转换拓扑 = 81 边图（527号定理）

σ 是**走势方向态**，不是价格位置态。依"走势必完美"（24/29课一级权威）：下跌走势
settle 后可**直接**转上涨走势，盘整不是必经中间态——故 σ 可 −1 → +1 不经 0。

相邻 = **恰好一条独立边的走势方向改变，可改变为任意其他方向**（含 +↔− 直接跳变）。
每个节点度数 = 3 分量 × 2 个其他方向 = 6。边数 = 27 × 6 / 2 = **81**。

图的性质（重算，527号：旧 54 边性质作废）：
  - 直径 = 3（最多 3 个分量不同）
  - 半径 = 3（每个节点偏心率均为 3）
  - **非二部图**（每个分量的 3 态全连通 = K3，含长度 3 的奇环）
  - 每个分量子图 = K3（完全图），整图 = K3 □ K3 □ K3（笛卡尔积）

> 527号裁定：旧实现的 54 边图（P3 □ P3 □ P3，禁 +↔−）隐含 σ 是连续位置态，
> 是**实现错误**——以数学便利（二部/直径6/|Aut|=48）约束本体论 = 有效域膨胀。
> 该 54 边对象作为纯数学对象无误，但**不是配置转换拓扑**；本次按 no-patch 删除，
> 不保留为 fallback。若未来确需"价格位置态投影"，另建独立命名模块。

## 距离 = Hamming 距离

配置转换距离 = 走势方向不同的分量个数（config_space.hamming_distance）。
最短路径数 = k!（k 个不同分量的翻转次序任意，每个一步直达）。
"""

from __future__ import annotations

from collections import deque
from dataclasses import dataclass
from math import factorial
from typing import Iterator

from newchan.topology.config_space import (
    CENTER,
    Configuration,
    ConfigurationSpace,
    WalkDirection,
    hamming_distance,
    polarity_index,
)


def adjacent(config: Configuration) -> frozenset[Configuration]:
    """返回与 config 相邻的所有配置（81 边图，527号）。

    相邻条件：恰好一条独立边的走势方向改变，可改为任意其他方向（含 +↔− 直接跳变）。
    每个配置有 3 分量 × 2 个其他方向 = 6 个邻居。

    Parameters
    ----------
    config : Configuration
        当前配置。

    Returns
    -------
    frozenset[Configuration]
        所有相邻配置（恒 6 个）。
    """
    result: list[Configuration] = []
    vals = list(config.as_tuple)

    for i in range(3):
        original = vals[i]
        for new_val in (-1, 0, 1):
            if new_val == original:
                continue
            new_vals = list(vals)
            new_vals[i] = new_val
            result.append(Configuration(
                sigma_p=WalkDirection(new_vals[0]),
                sigma_c=WalkDirection(new_vals[1]),
                sigma_r=WalkDirection(new_vals[2]),
            ))

    return frozenset(result)


def is_adjacent(c1: Configuration, c2: Configuration) -> bool:
    """判断两个配置是否相邻（恰好一条独立边的走势方向不同）。

    Parameters
    ----------
    c1, c2 : Configuration
        两个配置。

    Returns
    -------
    bool
    """
    return hamming_distance(c1, c2) == 1


def shortest_path(c1: Configuration, c2: Configuration) -> int:
    """两个配置之间的最短路径长度 = Hamming 距离（527号 81 边图）。

    每个走势方向不同的分量一步直达（含 +↔− 直跳），故最短路径长度 =
    不同分量个数。值域 {0, 1, 2, 3}。

    Parameters
    ----------
    c1, c2 : Configuration
        起点和终点。

    Returns
    -------
    int
        最短路径长度（步数）。
    """
    return hamming_distance(c1, c2)


def shortest_path_count(c1: Configuration, c2: Configuration) -> int:
    """两个配置之间最短路径的数量 = k!（k = 不同分量个数）。

    81 边图中每个不同分量一步直达，k 个不同分量的翻转次序任意排列，
    每种排列对应一条长度 k 的最短路径，故共 k! 条。

    Parameters
    ----------
    c1, c2 : Configuration
        起点和终点。

    Returns
    -------
    int
        最短路径数量。
    """
    return factorial(hamming_distance(c1, c2))


@dataclass(frozen=True, slots=True)
class Path:
    """配置空间中的路径。

    Attributes
    ----------
    steps : tuple[Configuration, ...]
        路径上的配置序列（包含起点和终点）。
    """

    steps: tuple[Configuration, ...]

    @property
    def length(self) -> int:
        """路径长度（步数 = 节点数 - 1）。"""
        return len(self.steps) - 1

    @property
    def start(self) -> Configuration:
        """起点配置。"""
        return self.steps[0]

    @property
    def end(self) -> Configuration:
        """终点配置。"""
        return self.steps[-1]

    @property
    def polarity_sequence(self) -> tuple[int, ...]:
        """路径上的极性指数序列。"""
        return tuple(polarity_index(c) for c in self.steps)


def reachable(config: Configuration, k: int) -> frozenset[Configuration]:
    """从 config 出发 k 步内可达的所有配置。

    使用 BFS，返回距离 <= k 的所有配置。81 边图直径为 3，k >= 3 时返回全部 27 个。

    Parameters
    ----------
    config : Configuration
        起点配置。
    k : int
        最大步数。

    Returns
    -------
    frozenset[Configuration]
        k 步内可达的配置集合。
    """
    if k < 0:
        raise ValueError(f"步数 k 必须非负，收到：{k}")

    visited: set[tuple[int, int, int]] = {config.as_tuple}
    queue: deque[tuple[Configuration, int]] = deque([(config, 0)])
    result: list[Configuration] = [config]

    while queue:
        current, dist = queue.popleft()
        if dist >= k:
            continue
        for nb in adjacent(current):
            if nb.as_tuple not in visited:
                visited.add(nb.as_tuple)
                result.append(nb)
                queue.append((nb, dist + 1))

    return frozenset(result)


def all_paths(
    c1: Configuration,
    c2: Configuration,
    max_k: int,
) -> list[Path]:
    """枚举从 c1 到 c2 的所有简单路径（长度 <= max_k）。

    使用 DFS 回溯。为防止组合爆炸，max_k 上限为 12。

    Parameters
    ----------
    c1 : Configuration
        起点。
    c2 : Configuration
        终点。
    max_k : int
        最大路径长度。受硬上限 12 的限制。

    Returns
    -------
    list[Path]
        所有满足条件的路径。
    """
    hard_limit = 12
    if max_k > hard_limit:
        raise ValueError(
            f"max_k={max_k} 超过硬上限 {hard_limit}，"
            f"为防止组合爆炸请使用更小的值"
        )
    if max_k < 0:
        raise ValueError(f"max_k 必须非负，收到：{max_k}")

    results: list[Path] = []
    stack: list[Configuration] = [c1]
    visited: set[tuple[int, int, int]] = {c1.as_tuple}

    def _dfs() -> None:
        current = stack[-1]
        depth = len(stack) - 1

        if current == c2 and depth > 0:
            results.append(Path(steps=tuple(stack)))
            # 不 return，允许继续搜索更长路径

        if depth >= max_k:
            return

        for nb in sorted(adjacent(current), key=lambda c: c.as_tuple):
            if nb.as_tuple not in visited:
                visited.add(nb.as_tuple)
                stack.append(nb)
                _dfs()
                stack.pop()
                visited.discard(nb.as_tuple)

    if c1 == c2:
        results.append(Path(steps=(c1,)))
    else:
        _dfs()

    return results


def all_shortest_paths(
    c1: Configuration,
    c2: Configuration,
) -> list[Path]:
    """枚举从 c1 到 c2 的所有最短路径。

    Parameters
    ----------
    c1, c2 : Configuration
        起点和终点。

    Returns
    -------
    list[Path]
        所有最短路径。
    """
    d = shortest_path(c1, c2)
    if d == 0:
        return [Path(steps=(c1,))]
    return all_paths(c1, c2, max_k=d)


# ── 图全局性质（81 边图，527号重算）──────────────────────────────


def graph_edge_count() -> int:
    """配置转换拓扑图的边数（恒为 81，527号）。"""
    space = ConfigurationSpace()
    count = 0
    configs = list(space)
    for i, c1 in enumerate(configs):
        for c2 in configs[i + 1:]:
            if is_adjacent(c1, c2):
                count += 1
    return count


def graph_diameter() -> int:
    """配置转换拓扑图的直径（恒为 3，527号）。"""
    space = ConfigurationSpace()
    max_d = 0
    configs = list(space)
    for c1 in configs:
        for c2 in configs:
            d = shortest_path(c1, c2)
            if d > max_d:
                max_d = d
    return max_d


def graph_radius() -> int:
    """配置转换拓扑图的半径（恒为 3，527号）。

    半径 = min over v of max over u of d(v, u)。81 边图中每个节点都存在
    一个三分量全不同的对端，故偏心率恒为 3，半径 = 3（与直径相等）。
    """
    space = ConfigurationSpace()
    configs = list(space)
    min_eccentricity = len(configs)  # 上界
    for v in configs:
        ecc = max(shortest_path(v, u) for u in configs)
        if ecc < min_eccentricity:
            min_eccentricity = ecc
    return min_eccentricity


def is_bipartite() -> bool:
    """配置转换拓扑图是否二部图（恒为 False，527号）。

    每个分量的 3 个走势方向全连通 = K3（完全图），含长度 3 的奇环，
    故整图非二部。这是 527号定理对旧 54 边二部图判断的否定。

    Returns
    -------
    bool
        恒为 False。
    """
    space = ConfigurationSpace()
    color: dict[tuple[int, int, int], int] = {}
    for start in space:
        if start.as_tuple in color:
            continue
        color[start.as_tuple] = 0
        queue: deque[Configuration] = deque([start])
        while queue:
            current = queue.popleft()
            for nb in adjacent(current):
                if nb.as_tuple not in color:
                    color[nb.as_tuple] = color[current.as_tuple] ^ 1
                    queue.append(nb)
                elif color[nb.as_tuple] == color[current.as_tuple]:
                    return False
    return True


def node_degree(config: Configuration) -> int:
    """配置在转换拓扑图中的度数（81 边图中恒为 6，527号）。

    Parameters
    ----------
    config : Configuration
        配置。

    Returns
    -------
    int
        邻居数量（恒为 6）。
    """
    return len(adjacent(config))
