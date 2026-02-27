"""转换拓扑：K4 配置空间的 P₃ □ P₃ □ P₃ 格点图。

27 个 K4 配置节点，每个节点是三条独立边（连接 CASH 的三条边）的状态三元组。
自然转换只在相邻态之间（+↔0、-↔0），不允许直接反转（+↔-）。

结构性质：
  - 8 个角节点（无0）：3 个邻居
  - 12 个棱节点（一个0）：4 个邻居
  - 6 个面节点（两个0）：5 个邻居
  - 1 个中心节点（0,0,0）：6 个邻居
  - 直径 = 6（从 (+,+,+) 到 (-,-,-) 需要 6 步）
  - (0,0,0) 是枢纽：到任何节点最多 3 步

概念溯源：
  [新缠论] 策略层——转换拓扑（v4 §2.5 两层结构的操作化）
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import IntEnum
from typing import Literal

from newchan.capital_flow import FlowDirection
from newchan.matrix_topology import AssetVertex


# ====================================================================
# 边状态枚举
# ====================================================================


class EdgeState(IntEnum):
    """独立边的走势状态。

    +1 = 趋势上行（A 相对 B 升值，资本从 B 流向 A）
     0 = 盘整（走势类型已确定为盘整）
    -1 = 趋势下行（A 相对 B 贬值，资本从 A 流向 B）
    """

    POSITIVE = 1
    ZERO = 0
    NEGATIVE = -1


# ====================================================================
# 独立边定义
# ====================================================================

# K4 的 3 条独立边：连接 CASH 的三条边。
# 约定：vertex_a 是资产端，vertex_b 是 CASH。
# POSITIVE = 资产相对现金升值 = 资本从 CASH 流向资产。
INDEPENDENT_EDGES: tuple[tuple[AssetVertex, AssetVertex], ...] = (
    (AssetVertex.EQUITY, AssetVertex.CASH),       # E/$
    (AssetVertex.COMMODITY, AssetVertex.CASH),     # C/$
    (AssetVertex.REAL_ESTATE, AssetVertex.CASH),   # R/$
)

# 3 条派生边：纯资产边。
DERIVED_EDGES: tuple[tuple[AssetVertex, AssetVertex], ...] = (
    (AssetVertex.EQUITY, AssetVertex.COMMODITY),    # E/C
    (AssetVertex.EQUITY, AssetVertex.REAL_ESTATE),  # E/R
    (AssetVertex.COMMODITY, AssetVertex.REAL_ESTATE),  # C/R
)


# ====================================================================
# K4 配置
# ====================================================================


@dataclass(frozen=True, slots=True)
class K4Config:
    """K4 配置：三条独立边的状态三元组。

    Attributes
    ----------
    e_cash : EdgeState
        EQUITY/CASH 边状态。
    c_cash : EdgeState
        COMMODITY/CASH 边状态。
    r_cash : EdgeState
        REAL_ESTATE/CASH 边状态。
    """

    e_cash: EdgeState
    c_cash: EdgeState
    r_cash: EdgeState

    @property
    def as_tuple(self) -> tuple[int, int, int]:
        return (self.e_cash.value, self.c_cash.value, self.r_cash.value)

    @property
    def zero_count(self) -> int:
        """含 0 的分量个数。"""
        return sum(1 for s in self.as_tuple if s == 0)

    @property
    def layer(self) -> Literal["corner", "edge", "face", "center"]:
        """节点在立方体中的层级。"""
        z = self.zero_count
        if z == 0:
            return "corner"
        if z == 1:
            return "edge"
        if z == 2:
            return "face"
        return "center"

    @property
    def neighbor_count(self) -> int:
        """自然邻居数量。"""
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

    @property
    def net_cash(self) -> int:
        """现金顶点的净流入计数（v4 §5.5）。

        X/$=+ → 资本从 $ 流向 X → $ 贡献 -1
        X/$=- → 资本从 X 流向 $ → $ 贡献 +1
        X/$=0 → 贡献 0
        """
        return -(self.e_cash + self.c_cash + self.r_cash)

    def derived_edge_state(self, idx: int) -> EdgeState:
        """计算第 idx 条派生边的离散算术状态。

        派生边 = 两条独立边的"差"：
          E/C = E/$ - C/$  （E 相对 C 的走势 = E 相对 $ 减去 C 相对 $）
          E/R = E/$ - R/$
          C/R = C/$ - R/$

        返回值截断到 {-1, 0, +1}。

        注意：这是离散算术结果。当两条独立边同号时，实际派生边方向
        不确定（v4 §5.5）。如需允许状态集，使用 derived_edge_allowed。
        """
        pairs = ((0, 1), (0, 2), (1, 2))  # (E/$-C/$, E/$-R/$, C/$-R/$)
        i, j = pairs[idx]
        t = self.as_tuple
        diff = t[i] - t[j]
        if diff > 0:
            return EdgeState.POSITIVE
        if diff < 0:
            return EdgeState.NEGATIVE
        return EdgeState.ZERO

    def derived_edge_allowed(self, idx: int) -> frozenset[EdgeState]:
        """计算第 idx 条派生边的允许状态集（v4 §5.5）。

        约束规则：
          异号 → 方向确定 {sign(a-b)}
          同号 → 不确定 {+,-,0}
          一端为 0 → 部分约束 {dominant, 0}
          双零 → 不确定 {+,-,0}
        """
        _PAIRS = ((0, 1), (0, 2), (1, 2))
        _ALL = frozenset(EdgeState)
        i, j = _PAIRS[idx]
        t = self.as_tuple
        a, b = t[i], t[j]

        if a == 0 and b == 0:
            return _ALL
        if a != 0 and b != 0:
            if a == b:
                return _ALL
            return frozenset({EdgeState.POSITIVE if a - b > 0 else EdgeState.NEGATIVE})
        # 一端为 0：dominant = sign(a) if a != 0 else sign(-b)
        dominant = EdgeState(a) if a != 0 else EdgeState(-b)
        return frozenset({dominant, EdgeState.ZERO})

    def all_derived_states(self) -> tuple[EdgeState, EdgeState, EdgeState]:
        """返回三条派生边的离散算术状态 (E/C, E/R, C/R)。"""
        return (
            self.derived_edge_state(0),
            self.derived_edge_state(1),
            self.derived_edge_state(2),
        )

    def all_derived_allowed(
        self,
    ) -> tuple[frozenset[EdgeState], frozenset[EdgeState], frozenset[EdgeState]]:
        """返回三条派生边的允许状态集 (E/C, E/R, C/R)。"""
        return (
            self.derived_edge_allowed(0),
            self.derived_edge_allowed(1),
            self.derived_edge_allowed(2),
        )


# ====================================================================
# 转换拓扑图
# ====================================================================


def all_configs() -> list[K4Config]:
    """枚举全部 27 个 K4 配置。"""
    configs = []
    for e in EdgeState:
        for c in EdgeState:
            for r in EdgeState:
                configs.append(K4Config(e_cash=e, c_cash=c, r_cash=r))
    return configs


def is_adjacent(a: K4Config, b: K4Config) -> bool:
    """判断两个配置是否为自然邻居。

    自然转换条件：
    1. 恰好一条独立边的状态不同
    2. 该边的状态变化是相邻的（+↔0 或 -↔0，不允许 +↔-）
    """
    diffs = []
    for sa, sb in zip(a.as_tuple, b.as_tuple):
        if sa != sb:
            diffs.append((sa, sb))
    if len(diffs) != 1:
        return False
    sa, sb = diffs[0]
    # 相邻 = 差的绝对值为 1（+↔0 或 -↔0）
    return abs(sa - sb) == 1


def neighbors(config: K4Config) -> list[K4Config]:
    """返回给定配置的所有自然邻居。"""
    result = []
    t = list(config.as_tuple)
    for i in range(3):
        original = t[i]
        for delta in (-1, +1):
            new_val = original + delta
            if -1 <= new_val <= 1:
                t[i] = new_val
                result.append(K4Config(
                    e_cash=EdgeState(t[0]),
                    c_cash=EdgeState(t[1]),
                    r_cash=EdgeState(t[2]),
                ))
                t[i] = original
    return result


def distance(a: K4Config, b: K4Config) -> int:
    """计算两个配置之间的最短路径长度（曼哈顿距离）。

    在 P₃ □ P₃ □ P₃ 上，最短路径 = 各轴距离之和。
    每轴距离：|state_a - state_b|（因为 +↔- 必须经过 0，距离为 2）。
    """
    return sum(abs(sa - sb) for sa, sb in zip(a.as_tuple, b.as_tuple))


# ====================================================================
# 特殊配置
# ====================================================================

HUB = K4Config(EdgeState.ZERO, EdgeState.ZERO, EdgeState.ZERO)
"""枢纽配置 (0,0,0)：全局盘整态，配置空间的十字路口。"""

CORNERS: list[K4Config] = [
    c for c in all_configs() if c.layer == "corner"
]
"""8 个角节点：方向极其清晰但退出方式少。"""


# ====================================================================
# FlowDirection 映射
# ====================================================================


def edge_state_to_flow_direction(state: EdgeState) -> FlowDirection:
    """EdgeState → FlowDirection 映射。

    POSITIVE → B_TO_A（资本从 CASH 流向资产）
    NEGATIVE → A_TO_B（资本从资产流向 CASH）
    ZERO → EQUILIBRIUM
    """
    if state == EdgeState.POSITIVE:
        return FlowDirection.B_TO_A
    if state == EdgeState.NEGATIVE:
        return FlowDirection.A_TO_B
    return FlowDirection.EQUILIBRIUM


def flow_direction_to_edge_state(direction: FlowDirection) -> EdgeState:
    """FlowDirection → EdgeState 映射。

    Raises
    ------
    ValueError
        direction 为 UNKNOWN。
    """
    if direction == FlowDirection.B_TO_A:
        return EdgeState.POSITIVE
    if direction == FlowDirection.A_TO_B:
        return EdgeState.NEGATIVE
    if direction == FlowDirection.EQUILIBRIUM:
        return EdgeState.ZERO
    raise ValueError(f"UNKNOWN 方向不能映射为 EdgeState：{direction}")


def config_to_flow_directions(
    config: K4Config,
) -> dict[tuple[AssetVertex, AssetVertex], FlowDirection]:
    """将 K4Config 展开为全部 6 条边的 FlowDirection。

    返回 dict，key 是 (vertex_a, vertex_b) 元组。
    """
    result: dict[tuple[AssetVertex, AssetVertex], FlowDirection] = {}

    # 3 条独立边
    states = config.as_tuple
    for i, (va, vb) in enumerate(INDEPENDENT_EDGES):
        result[(va, vb)] = edge_state_to_flow_direction(EdgeState(states[i]))

    # 3 条派生边
    derived = config.all_derived_states()
    for i, (va, vb) in enumerate(DERIVED_EDGES):
        result[(va, vb)] = edge_state_to_flow_direction(derived[i])

    return result


# ====================================================================
# 代数一致性检验
# ====================================================================


def check_algebraic_consistency(
    independent: tuple[EdgeState, EdgeState, EdgeState],
    observed_derived: tuple[EdgeState, EdgeState, EdgeState],
) -> list[str]:
    """检验观测到的派生边状态是否与独立边状态代数一致。

    使用 v4 §5.5 的允许状态集（而非离散算术点估计）。
    观测值落在允许集内 = 一致，落在允许集外 = 违规。

    Parameters
    ----------
    independent : (E/$, C/$, R/$) 的 EdgeState
    observed_derived : (E/C, E/R, C/R) 的观测 EdgeState

    Returns
    -------
    list[str]
        不一致项的描述。空列表 = 完全一致。
    """
    config = K4Config(
        e_cash=independent[0],
        c_cash=independent[1],
        r_cash=independent[2],
    )
    allowed = config.all_derived_allowed()
    edge_names = ("E/C", "E/R", "C/R")
    symbols = {1: "+", 0: "0", -1: "-"}

    violations = []
    for i in range(3):
        if observed_derived[i] not in allowed[i]:
            allowed_str = "{" + ",".join(
                symbols[s.value] for s in sorted(allowed[i], key=lambda x: -x.value)
            ) + "}"
            violations.append(
                f"{edge_names[i]}: 允许 {allowed_str}，"
                f"观测 {symbols[observed_derived[i].value]}"
            )
    return violations
