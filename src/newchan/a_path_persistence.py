"""A 系统 — 路径空间 (t,p) 持续同调：时间盲的候选解 + 实验性证伪通道。

存在论位置（诚实声明，非补丁）
--------------------------------
本模块是 **实验性证伪通道**，不是生产引擎。它实现编排者提出的方案——
把每根 K 线视为 (t, p) 联合平面上的点，对点云做 Vietoris-Rips filtration，
使 persistence "天然编码时间+价格"，试图消除 sublevel H0 的时间盲（239号）。

Gemini decide（2026-05-27，选项 C 拒绝）给出三个可证伪的 L0 预言，本模块
为在腾讯 700 真实数据上做 **L2 经验检验**而存在：

1. **信息论否决**：路径空间 Rips 只看单笔内部点云，无法访问笔外全局 EMA
   记忆（§5 的 MACD 差异来源）→ 不能复现 §5 的 #33/#37 区分。
2. **自由参数陷阱**：σ_p/σ_t 比值控制 (t,p) 平面纵横比，是伪装成数据内生的
   自由参数；换 std/MAD/range 会改变 Rips 连通顺序 → persistence 改变。
   → 故 `normalization` 是**显式可插拔参数**，用于经验检验此预言。
3. **数学退化**：单调笔的路径空间 H1 恒空；H0 Rips 退化为相邻点最大距离
   （≈ 最大单根速度），语义与缠论"累积动量"错位。
   → `max_step_distance()` 实现这个退化预测子，用于验证 H0≈max gap。

**否定性结果在此模块是合法且有价值的产出**（231号有效域规则）：若实测确认
路径空间不提供独立于 H0+MACD 的新信息，则 L2 证实 Gemini 的 L0 否决，
缩小有效域边界——比"未能否证"更有价值。

认识论等级
----------
- point cloud 构造 / Rips 调用 / max_step_distance：**L0**（纯算法，确定性）。
- "路径空间能/不能区分 #33/#37""与 MACD 相关性"：需腾讯 700 真实数据 → **L2**
  （由 `scripts/tencent_path_persistence.py` 产出，本模块不声称 L2）。

概念溯源标签
-----------
- 路径空间 (t,p) Rips [新缠论:候选——时间盲解，编排者提出 2026-05-27]
- Gemini 否决 [新缠论:orchestrator-proxy/decide 2026-05-27 选项C]
- ker(D) 时间盲 [新缠论:239号]
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Literal, Sequence

__all__ = [
    "PathBar",
    "PathPersistenceResult",
    "scale_estimate",
    "path_point_cloud",
    "path_persistence",
    "max_step_distance",
]

Normalization = Literal["std", "mad", "range"]


# ====================================================================
# 数据类型（frozen + slots，immutable）
# ====================================================================


@dataclass(frozen=True, slots=True)
class PathBar:
    """路径空间 persistence diagram 中的单个特征。

    与 a_persistence_barcode.Bar 同构，但 birth/death 的量纲是**归一化后的
    联合距离**（无量纲），不是价格。这是与 sublevel H0 不可直接比较的根本原因。
    """

    birth: float
    death: float
    persistence: float
    dimension: int


@dataclass(frozen=True, slots=True)
class PathPersistenceResult:
    """一段走势在 (t,p) 路径空间上的 Rips 持续同调结果。

    Attributes
    ----------
    bars : tuple[PathBar, ...]
        全部特征，persistence 降序。
    n_points : int
        点云点数（= K 线根数）。
    sigma_t, sigma_p : float
        实际使用的时间/价格尺度因子（暴露出来以便审计自由参数效应）。
    normalization : str
        使用的归一化方法（std/mad/range）。
    """

    bars: tuple[PathBar, ...]
    n_points: int
    sigma_t: float
    sigma_p: float
    normalization: str

    def by_dimension(self, dimension: int) -> tuple[PathBar, ...]:
        return tuple(b for b in self.bars if b.dimension == dimension)

    def total_persistence(self, dimension: int | None = None) -> float:
        return sum(
            b.persistence
            for b in self.bars
            if dimension is None or b.dimension == dimension
        )

    def max_persistence(self, dimension: int | None = None) -> float:
        cands = [
            b.persistence
            for b in self.bars
            if dimension is None or b.dimension == dimension
        ]
        return max(cands) if cands else 0.0


# ====================================================================
# 尺度估计（自由参数审计的核心——Gemini 论断2 的检验入口）
# ====================================================================


def scale_estimate(values: Sequence[float], method: Normalization) -> float:
    """估计一组值的尺度（用于归一化）。

    三种方法对应 Gemini 论断2 中点名的三个候选；它们给出不同的 σ，
    从而（若论断2 成立）给出不同的 Rips persistence。这是把"自由参数效应"
    做成**可经验测量**的设计，不是补丁。

    - std   : 标准差（提案默认的"数据内生标准差"）。
    - mad   : 中位绝对偏差 × 1.4826（稳健，抗极端跳空）。
    - range : (max − min) / √12（与均匀分布 std 同量纲的极差估计）。

    Returns
    -------
    float
        尺度估计（≥ 0）。常数序列返回 0.0（调用方须处理退化）。
    """
    vals = [float(v) for v in values]
    n = len(vals)
    if n < 2:
        return 0.0
    if method == "std":
        mean = sum(vals) / n
        var = sum((v - mean) ** 2 for v in vals) / n
        return math.sqrt(var)
    if method == "mad":
        srt = sorted(vals)
        med = srt[n // 2] if n % 2 else (srt[n // 2 - 1] + srt[n // 2]) / 2
        devs = sorted(abs(v - med) for v in vals)
        mad = devs[n // 2] if n % 2 else (devs[n // 2 - 1] + devs[n // 2]) / 2
        return 1.4826 * mad
    if method == "range":
        return (max(vals) - min(vals)) / math.sqrt(12.0)
    raise ValueError(f"unknown normalization method: {method!r}")


# ====================================================================
# (t,p) 点云构造
# ====================================================================


def path_point_cloud(
    prices: Sequence[float],
    times: Sequence[float] | None = None,
    *,
    normalization: Normalization = "std",
    sigma_t: float | None = None,
    sigma_p: float | None = None,
) -> tuple[list[list[float]], float, float]:
    """构造归一化的 (t,p) 点云。

    P_i = ( t_i / σ_t , p_i / σ_p )，σ 由 normalization 决定（或显式给定）。
    时间默认取 0..n-1（均匀索引）——注意此时相邻 Δt 恒=1，
    "Δt 的标准差"=0，故 σ_t 只能取**时间值**的尺度（≈ 仅依赖 n）。
    这正是 Gemini 论断2"σ_t 仅依赖 n"的代数根源。

    **全局 σ 覆盖（段间对比的前提，2026-05-27 段间实验新增）**
    -------------------------------------------------------------
    `sigma_t` / `sigma_p` 显式给定时跳过 `scale_estimate`，直接用传入值归一化。
    动机：段间背驰对比（A段力度 vs C段力度）要求各段在**同一把尺子**下度量。
    若每段用自身的 std 归一化（sigma=None），则每段被缩放到单位方差，**幅度信息
    被抹除**——一个跌 100 点的段和跌 10 点的段几何上趋同 → 段间对比失效。
    传入全局 σ（整条序列的 std/mad/range）保留跨段幅度差，使段间对比有意义。
    代价：全局 σ 的 σ_p/σ_t 比值是显式自由参数（Gemini 论断2 的张力不消除，
    只是从"伪装成数据内生"变为"显式声明的全局选择"）。

    Returns
    -------
    (cloud, sigma_t, sigma_p)
        cloud: list[[t_norm, p_norm], ...]；sigma_t/sigma_p: 实际尺度因子。

    Raises
    ------
    ValueError
        价格或时间尺度退化为 0（常数序列），联合度量无法定义。
    """
    p = [float(v) for v in prices]
    n = len(p)
    if times is None:
        t = [float(i) for i in range(n)]
    else:
        t = [float(v) for v in times]
        if len(t) != n:
            raise ValueError("times 与 prices 长度不一致")

    st = scale_estimate(t, normalization) if sigma_t is None else float(sigma_t)
    sp = scale_estimate(p, normalization) if sigma_p is None else float(sigma_p)
    if st <= 0.0 or sp <= 0.0:
        raise ValueError(
            f"尺度退化 (sigma_t={st}, sigma_p={sp})：常数序列无法定义联合度量"
        )

    cloud = [[ti / st, pi / sp] for ti, pi in zip(t, p)]
    return cloud, st, sp


# ====================================================================
# 路径空间 Rips 持续同调
# ====================================================================


def path_persistence(
    prices: Sequence[float],
    times: Sequence[float] | None = None,
    *,
    normalization: Normalization = "std",
    maxdim: int = 1,
    sigma_t: float | None = None,
    sigma_p: float | None = None,
) -> PathPersistenceResult:
    """(t,p) 联合空间上的 Vietoris-Rips 持续同调。

    与现有 a_persistence_barcode 的两点根本区别：
    1. sublevel H0 在 1D 价格的下水平集上滤波（时间盲）；本函数在 2D (t,p)
       点云上做 Rips（含时）。
    2. sublevel H0 的 birth/death 是价格；本函数是归一化联合距离（无量纲）。

    `sigma_t` / `sigma_p` 见 `path_point_cloud`——给定时用全局 σ（段间对比的前提）。

    Returns
    -------
    PathPersistenceResult
        bars 按 persistence 降序。点云过小（< 3）时返回空 bars。

    认识论等级：L0（纯算法）。
    """
    cloud, sigma_t, sigma_p = path_point_cloud(
        prices, times, normalization=normalization,
        sigma_t=sigma_t, sigma_p=sigma_p,
    )
    n = len(cloud)
    if n < 3:
        return PathPersistenceResult((), n, sigma_t, sigma_p, normalization)

    import numpy as np
    from ripser import ripser

    diagrams = ripser(np.asarray(cloud, dtype=float), maxdim=maxdim)["dgms"]

    bars: list[PathBar] = []
    for dim, dgm in enumerate(diagrams):
        for birth, death in dgm:
            if math.isinf(death):
                continue  # essential class（H0 全局分量）：不计入有限 persistence
            bars.append(PathBar(float(birth), float(death), float(death - birth), dim))

    bars.sort(key=lambda b: b.persistence, reverse=True)
    return PathPersistenceResult(
        tuple(bars), n, sigma_t, sigma_p, normalization
    )


# ====================================================================
# 退化预测子（Gemini 论断3 的检验入口）
# ====================================================================


def max_step_distance(
    prices: Sequence[float],
    times: Sequence[float] | None = None,
    *,
    normalization: Normalization = "std",
    sigma_t: float | None = None,
    sigma_p: float | None = None,
) -> float:
    """相邻点在归一化 (t,p) 空间中的最大步距。

    Gemini 论断3：单调路径上，H0 Rips 的最大有限 persistence 退化为
    路径图中相邻点的最大边长（单连接树的最长边）。本函数直接计算该量，
    用于经验检验"path-H0 max persistence ≈ max_step_distance"。

    若两者高度相等 → 论断3 在 L2 成立 → 路径空间 H0 仅是"最大单根速度"
    的拓扑包装，可被简单差分平替（杀鸡用牛刀）。

    `sigma_t` / `sigma_p` 见 `path_point_cloud`——给定时用全局 σ。

    认识论等级：L0（纯算法）。
    """
    cloud, _, _ = path_point_cloud(
        prices, times, normalization=normalization,
        sigma_t=sigma_t, sigma_p=sigma_p,
    )
    if len(cloud) < 2:
        return 0.0
    return max(
        math.dist(cloud[i], cloud[i + 1]) for i in range(len(cloud) - 1)
    )
