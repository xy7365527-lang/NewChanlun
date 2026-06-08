"""折叠通道 — K4 顶点之间的折叠观测量（不是顶点，不是边）。

概念溯源：292号折叠拓扑本体论、254号黄金∈Σ∩C / 油=结算通道、330号资本循环、
          528号顶点映射折叠压平冲突。

## 折叠通道 ≠ 顶点 ≠ 边

292号核心命题："矩阵划分是折叠的推论，不是前提。" 折叠 = **同一对象在不同截面呈现
不同范畴身份（不同相位）**，不是"既 A 又 B"。

金和油不是 K4 的第五、第六个顶点，也不是 K4 的边——它们是**横跨两个顶点的折叠**，
在边上被**观测**：

  - **Au（黄金）= C↔M 折叠通道**（双向）。
    黄金既是商品（C 相位：可交割、储存）又是货币替代（M 相位：价值储藏、结算锚）。
    254号定理3：黄金 ∈ Σ∩C；在黄金截面 C=$，K4 退化为 K3。
    观测于 C/M 独立边。

  - **Oil（石油）= C→P 通道**（定向）。
    油是产业循环 M→C→P 的物质投入（330号）。292号§5：油的扩张面=燃烧（不可逆
    消耗，进入生产）。观测于 P/C 派生边。

    > ⚠ 已知未建模边界（292号 / 528号张力，诚实标注）：292号折叠盘点同时记录
    > Oil 的 C↔$ 石油美元相位（收缩面=回到货币秩序）。本模块按 330号产业循环视角
    > 建模 C→P 扩张相位；C↔$ 石油美元相位为**已知但本次未建模**的折叠相位，
    > 留待未来扩展（结构上 FoldChannel 支持，见下）。不静默覆盖 292号。

## 折叠通道是观测量，不进入配置空间主轴

配置空间 Γ=(σ_P, σ_C, σ_R)（config_space.py）由三条独立边的走势方向构成。
折叠通道状态（ω = Au/Oil）是**附加信号**，不进入 Γ 主轴——它度量折叠生命周期
（292号§4），用于长期方向判断，**不接回实时入场门控**（Omega Regime 已被 L2 证伪，
见 auto-memory `project_omega_regime_falsified`）。

## 共享性权重 W（292号§3.3 区间套顺序）

共享折叠是外层，特有折叠是内层，外层约束内层：
  Au（全局共享，W=4）> Oil（半共享，W=3）> Bond（经济体特有，W=2）> RE（地区特有，W=1）

本模块仅承载 K4 主轴的两个 C 类折叠（Au/Oil）。完整五类折叠等价类（含 Bond/RE/
Equity）见 trading/fold_equivalence.py（商空间排序层，352号）——本模块是其在**顶点
映射层**的对应（528号裂隙1指出二者此前未连线）。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from newchan.topology.graph import Edge, K4Graph, Vertex


class FoldDirection(Enum):
    """折叠通道的方向性。"""

    BIDIRECTIONAL = "bidirectional"  # 双向折叠：同一对象在两个顶点间往返（如 Au: C↔M）
    DIRECTED = "directed"            # 定向通道：单向物质/价值流（如 Oil: C→P）


@dataclass(frozen=True, slots=True)
class FoldChannel:
    """K4 顶点之间的折叠通道。

    折叠通道是观测量，在 K4 的一条边上被读出，但它本身不是边——它表达"同一对象
    （金/油）在 source 与 target 两个顶点上的不同相位"。

    Attributes
    ----------
    name : str
        通道名（"Au" / "Oil"）。
    source : Vertex
        折叠的一端。定向通道中为流出端（如 Oil 的 C）。
    target : Vertex
        折叠的另一端。定向通道中为流入端（如 Oil 的 P）。
    direction : FoldDirection
        双向折叠（BIDIRECTIONAL）或定向通道（DIRECTED）。
    observable_symbol : str
        折叠通道的可观测标的（其 $ 相位价格代理），如 GC（黄金）、CL（原油）。
    weight : int
        292号折叠共享性权重 W（区间套顺序，外层约束内层）。
    """

    name: str
    source: Vertex
    target: Vertex
    direction: FoldDirection
    observable_symbol: str
    weight: int

    @property
    def endpoints(self) -> frozenset[Vertex]:
        """折叠横跨的两个顶点（无序）。"""
        return frozenset({self.source, self.target})

    @property
    def is_bidirectional(self) -> bool:
        """是否为双向折叠。"""
        return self.direction is FoldDirection.BIDIRECTIONAL

    def folds_across(self, graph: K4Graph) -> Edge:
        """返回该折叠通道被观测的 K4 边。

        折叠通道横跨两个顶点，其状态在连接这两个顶点的 K4 边上被读出。
        例如 Au（C↔M）观测于 C/M 独立边，Oil（C→P）观测于 P/C 派生边。

        Parameters
        ----------
        graph : K4Graph
            K4 完全图。

        Returns
        -------
        Edge
            该折叠通道横跨的边。
        """
        return graph.edge_between(self.source, self.target)


# ── 292号 K4 主轴的两个 C 类折叠通道 ──────────────────────────────

AU = FoldChannel(
    name="Au",
    source=Vertex.C,
    target=Vertex.M,
    direction=FoldDirection.BIDIRECTIONAL,
    observable_symbol="GC",
    weight=4,  # 全局共享，区间套最外层
)
"""黄金折叠通道：C↔M 双向（黄金 ∈ Σ∩C，254号定理3）。观测于 C/M 边。"""

OIL = FoldChannel(
    name="Oil",
    source=Vertex.C,
    target=Vertex.P,
    direction=FoldDirection.DIRECTED,
    observable_symbol="CL",
    weight=3,  # 半共享（石油美元可撕毁），次外层
)
"""石油定向通道：C→P（产业循环物质投入，330号）。观测于 P/C 边。

⚠ C↔$ 石油美元相位（292号收缩面）为已知未建模边界，见模块 docstring。"""

K4_FOLD_CHANNELS: tuple[FoldChannel, ...] = (AU, OIL)
"""K4 主轴的折叠通道集合（Au, Oil）。完整五类见 fold_equivalence.py。"""


# ── ω 金油比：折叠通道的状态度量 ──────────────────────────────────


def omega(gold_price: float, oil_price: float) -> float:
    """金油比 ω = Au 的 $ 相位 / Oil 的 $ 相位（= 两个 C↔$ 折叠对象 $ 相位之比）。

    482号：ω = Price(Gold) / Price(Oil) = 金融循环（MCM′）与产业循环（P）的力量
    对比 = 剥削率（角速度）。金是退出 MCM′ 的沉没物，油是被 MCM′ 压价的 P 输入。

    认识论等级：ω 本身是 L0（纯比值）。其作为长期方向指标 = L2（482号协整 p=0.033）。
    作为实时入场门控 = 已被 L2 证伪（project_omega_regime_falsified），不在此提供。

    Parameters
    ----------
    gold_price : float
        黄金 $ 相位价格（AU.observable_symbol 的价格）。
    oil_price : float
        石油 $ 相位价格（OIL.observable_symbol 的价格），必须 > 0。

    Returns
    -------
    float
        金油比 ω。

    Raises
    ------
    ValueError
        如果 oil_price <= 0。
    """
    if oil_price <= 0:
        raise ValueError(f"油价必须为正，收到：{oil_price}")
    return gold_price / oil_price
