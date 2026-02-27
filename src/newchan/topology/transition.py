"""转换拓扑 — 配置之间的转换规则与路径分析。

概念溯源：ontology-v2-push.md §三.8, §三.9

转换拓扑图 G = P3 [] P3 [] P3（笛卡尔积）。
27 个节点，54 条边。

相邻 = 只有一条独立边状态改变，且只在相邻态之间跳转
      （+<->0, -<->0，禁止 +<->-）。

图的性质：
  - 直径 = 6
  - 半径 = 3（中心为 (0,0,0)）
  - 围长 = 4（最短环）
  - 二部图（色数 = 2）
  - 自同构群 |Aut(G)| = 48
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
    manhattan_distance,
    polarity_index,
)


def adjacent(config: Configuration) -> frozenset[Configuration]:
    """返回与 config 相邻的所有配置。

    相邻条件：恰好一条独立边改变，且变化为相邻态（+<->0 或 -<->0）。

    Parameters
    ----------
    config : Configuration
        当前配置。

    Returns
    -------
    frozenset[Configuration]
        所有相邻配置。
    """
    result: list[Configuration] = []
    vals = list(config.as_tuple)

    for i in range(3):
        original = vals[i]
        for delta in (-1, +1):
            new_val = original + delta
            if -1 <= new_val <= 1:
                new_vals = list(vals)
                new_vals[i] = new_val
                result.append(Configuration(
                    sigma_e=WalkDirection(new_vals[0]),
                    sigma_c=WalkDirection(new_vals[1]),
                    sigma_r=WalkDirection(new_vals[2]),
                ))

    return frozenset(result)


def is_adjacent(c1: Configuration, c2: Configuration) -> bool:
    """判断两个配置是否相邻。

    Parameters
    ----------
    c1, c2 : Configuration
        两个配置。

    Returns
    -------
    bool
    """
    diffs = []
    for a, b in zip(c1.as_tuple, c2.as_tuple):
        if a != b:
            diffs.append(abs(a - b))
    return len(diffs) == 1 and diffs[0] == 1


def shortest_path(c1: Configuration, c2: Configuration) -> int:
    """两个配置之间的最短路径长度。

    等于曼哈顿距离：d = sum_k |pos_k(c1) - pos_k(c2)|。

    Parameters
    ----------
    c1, c2 : Configuration
        起点和终点。

    Returns
    -------
    int
        最短路径长度（步数）。
    """
    return manhattan_distance(c1, c2)


def shortest_path_count(c1: Configuration, c2: Configuration) -> int:
    """两个配置之间最短路径的数量。

    = (d1 + d2 + d3)! / (d1! * d2! * d3!)
    其中 dk 是各轴上需要移动的步数。

    Parameters
    ----------
    c1, c2 : Configuration
        起点和终点。

    Returns
    -------
    int
        最短路径数量。
    """
    def _pos(v: int) -> int:
        return v + 1

    d = [
        abs(_pos(a) - _pos(b))
        for a, b in zip(c1.as_tuple, c2.as_tuple)
    ]
    total = sum(d)
    return factorial(total) // (factorial(d[0]) * factorial(d[1]) * factorial(d[2]))


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

    使用 BFS，返回距离 <= k 的所有配置。

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
    """枚举从 c1 到 c2 的所有路径（长度 <= max_k）。

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
            # 不 return，允许继续搜索更长路径（经过 c2 再回来再到 c2）
            # 但只记录到达 c2 的路径

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


# ── 图全局性质 ──────────────────────────────────────────────────


def graph_edge_count() -> int:
    """转换拓扑图的边数（恒为 54）。"""
    space = ConfigurationSpace()
    count = 0
    configs = list(space)
    for i, c1 in enumerate(configs):
        for c2 in configs[i + 1:]:
            if is_adjacent(c1, c2):
                count += 1
    return count


def graph_diameter() -> int:
    """转换拓扑图的直径（恒为 6）。"""
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
    """转换拓扑图的半径（恒为 3）。

    半径 = min over v of max over u of d(v, u)。
    中心节点 (0,0,0) 到任意节点的最大距离为 3。
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
    """验证转换拓扑图是二部图。

    P3 是二部图，P3 的笛卡尔积仍是二部图。
    二部划分由 pos(a) + pos(b) + pos(c) 的奇偶性决定。

    Returns
    -------
    bool
        恒为 True。
    """
    space = ConfigurationSpace()
    for c1 in space:
        parity1 = sum(v + 1 for v in c1.as_tuple) % 2
        for c2 in adjacent(c1):
            parity2 = sum(v + 1 for v in c2.as_tuple) % 2
            if parity1 == parity2:
                return False
    return True


def bipartite_partition() -> tuple[frozenset[Configuration], frozenset[Configuration]]:
    """返回二部划分。

    按 pos 和的奇偶性划分：偶 (14 个) 和奇 (13 个)。

    Returns
    -------
    tuple[frozenset[Configuration], frozenset[Configuration]]
        (偶集, 奇集)。
    """
    space = ConfigurationSpace()
    even: list[Configuration] = []
    odd: list[Configuration] = []
    for c in space:
        parity = sum(v + 1 for v in c.as_tuple) % 2
        if parity == 0:
            even.append(c)
        else:
            odd.append(c)
    return frozenset(even), frozenset(odd)
