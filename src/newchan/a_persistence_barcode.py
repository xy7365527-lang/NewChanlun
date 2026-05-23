"""A 系统 — 持续同调 barcode：缠论递归的并行力度/级别引擎。

存在论位置
----------
缠论对象（笔→线段→中枢→走势）提供**形态学递归**；持续同调在递归的
每一层提供**动力学度量**（力度、级别）。两者不是替代关系——拓扑计算
嵌入在缠论递归的内部，随递归层级自动展开。

与 ker(D) 的关系（239号谱系，必读）
-----------------------------------
235/237/239号确立：**振幅力度 ∈ ker(D)**——离散化算子 D（笔→线段→中枢）
系统性吸收幅度信息，因此在**离散化输出**（Segment/Move）上计算振幅力度是
重新捞取 D 已经丢弃的信息，概念不自洽。`directional_force.py` 据此用方向性
力度（∉ ker(D)）替代。

持续同调与上述批判**不冲突**，因为它作用在 **D 之前的原始价格序列**上，是
一条**与 D 正交的并行测量通道**，不经过 D，因此"是否 ∈ ker(D)"对它是范畴错误
（它不在 D 的定义域内）。但必须诚实标注：
- **H0 持续度 ≈ 摆动 prominence ≈ 幅度维度**——它测量的正是 D 选择性吸收的那个
  维度。作为力度，它与方向性力度互补而非等价（见 a_divergence_topo 的边界条件）。
- **H1 持续度（相空间 loop）携带振荡的几何信息（中枢的循环性）**——比纯幅度多，
  是真正的新信号。

认识论等级（formalization-validity-domain 规则）
-----------------------------------------------
- barcode 计算本身：**L0**（纯算法，从价格序列确定性推导，零信息增量）。
- "persistence = 力度""dominant = 主导级别"等**经验断言**：在真实数据上做过
  L2/L3 假设检验之前，停留在 **L1**（管线正确性）。本模块不声称 L2+。

维度↔级别映射
-------------
- **H0（连通分量，sublevel set filtration）**：1D 价格的摆动 prominence。
  单调趋势 → 单一大 bar（persistence = 趋势全幅）；震荡 → 多个小 bar。
  ≈ 笔/趋势级力度。
- **H1（loop，time-delay 嵌入 + Vietoris-Rips）**：相空间中价格往返形成的环。
  中枢 = 价格在区间内往返 = 相空间的 1-cycle。persistence = 振荡的几何持续性。
  ≈ 中枢级力度。

> 技术记录（语法记录，非补丁）：1D 函数的 sublevel set filtration 只产生 H0
> （区间上函数的下水平集无 1 维环）。要得到 H1 必须先做 time-delay 嵌入把时间
> 序列升维成点云。因此**两个维度用两种不同的 filtration**，各取所长——这是严格
> 的设计，不是 workaround。

概念溯源标签
-----------
- 持续同调力度引擎 [新缠论:候选——并行动力学度量]
- ker(D) 与幅度吸收 [新缠论:235号/237号/239号]
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Literal, Protocol, Sequence, runtime_checkable

__all__ = [
    "Bar",
    "PersistenceBarcode",
    "sublevel_h0_bars",
    "rips_h1_bars",
    "barcode_from_prices",
    "active_bars",
    "dominant_bar",
    "atr",
    "atr_noise_threshold",
    "PriceRanged",
    "BarcodeAnnotation",
    "attach_barcodes",
]


# ====================================================================
# 数据类型（frozen + slots，immutable）
# ====================================================================


@dataclass(frozen=True, slots=True)
class Bar:
    """persistence diagram 中的单个特征（一条 barcode）。

    Attributes
    ----------
    birth : float
        特征诞生的 filtration 值（H0：局部极小价位；H1：loop 出现的尺度）。
    death : float
        特征消亡的 filtration 值。
    persistence : float
        death - birth。特征的拓扑显著性 = 该维度上的"力度"。
    dimension : int
        同调维度。0 = 连通分量（笔/趋势级），1 = loop（中枢级）。
    """

    birth: float
    death: float
    persistence: float
    dimension: int


@dataclass(frozen=True, slots=True)
class PersistenceBarcode:
    """一段价格序列的持续同调结果（所有维度的 Bar 的不可变集合）。

    Attributes
    ----------
    bars : tuple[Bar, ...]
        全部特征，按 persistence 降序排列（便于直接读出主导级别）。
    n_points : int
        计算所用的价格样本数（可观测性：barcode 对应多少根 K 线）。
    """

    bars: tuple[Bar, ...]
    n_points: int

    def by_dimension(self, dimension: int) -> tuple[Bar, ...]:
        """取某一维度的全部 bar（保持 persistence 降序）。"""
        return tuple(b for b in self.bars if b.dimension == dimension)

    def total_persistence(self, dimension: int | None = None) -> float:
        """总持续度 = sum(persistence)。

        等价于 persistence diagram 到对角线（空图）的 Wasserstein-1 力度，
        是"这段走势在该维度上的总力度"的拓扑表达。
        """
        return sum(
            b.persistence
            for b in self.bars
            if dimension is None or b.dimension == dimension
        )

    def max_persistence(self, dimension: int | None = None) -> float:
        """最大持续度 = 主导特征的力度（无特征时为 0.0）。"""
        candidates = [
            b.persistence
            for b in self.bars
            if dimension is None or b.dimension == dimension
        ]
        return max(candidates) if candidates else 0.0


# ====================================================================
# H0：1D sublevel set filtration（union-find merge tree）
# ====================================================================


class _UnionFind:
    """按诞生值合并的并查集——elder rule：诞生早（值小）的分量存活。"""

    __slots__ = ("parent", "birth")

    def __init__(self) -> None:
        self.parent: dict[int, int] = {}
        self.birth: dict[int, float] = {}

    def add(self, x: int, value: float) -> None:
        self.parent[x] = x
        self.birth[x] = value

    def find(self, x: int) -> int:
        root = x
        while self.parent[root] != root:
            root = self.parent[root]
        # 路径压缩
        while self.parent[x] != root:
            self.parent[x], x = root, self.parent[x]
        return root


def sublevel_h0_bars(
    values: Sequence[float],
    *,
    finite_cap: float | None = None,
) -> tuple[Bar, ...]:
    """1D 价格序列的 sublevel set filtration 的 H0 持续同调。

    在路径图（相邻样本相连）上做下水平集滤波：阈值 t 从低到高扫描，
    局部极小处诞生连通分量，分量在相遇的极大值处合并——年轻分量（诞生值
    更高）死亡，persistence = 死亡值 - 诞生值 = 该摆动的 prominence。

    这是 1D 持续同调的标准 merge-tree 算法（O(n log n)，精确，无外部依赖）。

    Parameters
    ----------
    values : Sequence[float]
        价格序列（通常是 close）。
    finite_cap : float | None
        全局（最久）分量的死亡值本应为 +inf；用此值封顶以得到有限 persistence。
        默认 max(values)，使全局 bar 的 persistence = 价格全幅。

    Returns
    -------
    tuple[Bar, ...]
        H0 特征，persistence 降序。
    """
    n = len(values)
    if n == 0:
        return ()
    vals = [float(v) for v in values]
    if n == 1:
        return (Bar(vals[0], vals[0], 0.0, 0),)

    cap = max(vals) if finite_cap is None else float(finite_cap)

    order = sorted(range(n), key=lambda i: vals[i])
    uf = _UnionFind()
    active = [False] * n
    bars: list[Bar] = []

    for i in order:
        vi = vals[i]
        uf.add(i, vi)
        active[i] = True

        neighbor_roots: list[int] = []
        for j in (i - 1, i + 1):
            if 0 <= j < n and active[j]:
                r = uf.find(j)
                if r not in neighbor_roots:
                    neighbor_roots.append(r)

        if not neighbor_roots:
            continue  # 局部极小：新分量诞生

        comps = [uf.find(i), *neighbor_roots]
        comps = list(dict.fromkeys(comps))
        elder = min(comps, key=lambda r: uf.birth[r])
        for r in comps:
            if r == elder:
                continue
            b = uf.birth[r]
            if b < vi:  # 真实特征（年轻分量在 vi 处死亡）；诞生于 vi 的奇点 persistence=0，跳过
                bars.append(Bar(b, vi, vi - b, 0))
            uf.parent[r] = elder

    # 仍存活的分量（路径图最终全连通 → 恰好一个）死亡值封顶
    alive_roots = {uf.find(i) for i in range(n)}
    for r in alive_roots:
        b = uf.birth[r]
        bars.append(Bar(b, cap, cap - b, 0))

    bars.sort(key=lambda b: b.persistence, reverse=True)
    return tuple(bars)


# ====================================================================
# H1：time-delay 嵌入 + Vietoris-Rips（ripser）
# ====================================================================


def _takens_embedding(
    values: Sequence[float],
    dim: int,
    delay: int,
) -> list[list[float]]:
    """Takens time-delay 嵌入：x(t) → [x(t), x(t-τ), ..., x(t-(d-1)τ)]。

    把 1D 时间序列升维成相空间点云，使振荡（中枢）显现为环。
    """
    vals = [float(v) for v in values]
    span = (dim - 1) * delay
    n_pts = len(vals) - span
    if n_pts <= 0:
        return []
    return [
        [vals[t + k * delay] for k in range(dim)]
        for t in range(n_pts)
    ]


def rips_h1_bars(
    values: Sequence[float],
    *,
    embedding_dim: int = 3,
    embedding_delay: int = 1,
    max_dimension: int = 1,
) -> tuple[Bar, ...]:
    """time-delay 嵌入 + Vietoris-Rips 的 H1（loop）持续同调。

    中枢 = 价格在区间内往返振荡 = 相空间轨迹的 1-cycle。loop 的 persistence
    度量振荡在相空间中的几何持续性（半径 × 规整度），≈ 中枢级力度。

    Parameters
    ----------
    values : Sequence[float]
        价格序列。
    embedding_dim : int
        嵌入维度（默认 3）。
    embedding_delay : int
        嵌入延迟 τ（默认 1）。
    max_dimension : int
        ripser 计算的最高同调维度（默认 1，即算到 H1）。

    Returns
    -------
    tuple[Bar, ...]
        H1 特征（dimension=1），persistence 降序。点云过小时返回空。
    """
    cloud = _takens_embedding(values, embedding_dim, embedding_delay)
    if len(cloud) < embedding_dim + 2:
        return ()

    import numpy as np
    from ripser import ripser

    diagrams = ripser(np.asarray(cloud, dtype=float), maxdim=max_dimension)["dgms"]
    if len(diagrams) < 2:
        return ()

    bars: list[Bar] = []
    for birth, death in diagrams[1]:
        if math.isinf(death):
            continue
        bars.append(Bar(float(birth), float(death), float(death - birth), 1))

    bars.sort(key=lambda b: b.persistence, reverse=True)
    return tuple(bars)


# ====================================================================
# 主入口：从价格序列直接得到 barcode
# ====================================================================


def barcode_from_prices(
    prices: Sequence[float],
    *,
    maxdim: int = 1,
    embedding_dim: int = 3,
    embedding_delay: int = 1,
    finite_cap: float | None = None,
) -> PersistenceBarcode:
    """从一段价格序列计算 PersistenceBarcode（不预选 K 线周期）。

    - H0 永远计算（sublevel set，笔/趋势级 prominence）。
    - H1 当 maxdim >= 1 时计算（time-delay 嵌入 + Rips，中枢级 loop）。

    Parameters
    ----------
    prices : Sequence[float]
        价格序列（通常 close）。
    maxdim : int
        最高同调维度（0 = 只算 H0；>=1 = H0 + H1）。
    embedding_dim, embedding_delay : int
        H1 的 Takens 嵌入参数。
    finite_cap : float | None
        H0 全局分量死亡值封顶（默认 max(prices)）。

    Returns
    -------
    PersistenceBarcode
        bars 按 persistence 降序。

    认识论等级：L0（纯算法）。
    """
    h0 = sublevel_h0_bars(prices, finite_cap=finite_cap)
    if maxdim >= 1:
        h1 = rips_h1_bars(
            prices,
            embedding_dim=embedding_dim,
            embedding_delay=embedding_delay,
            max_dimension=maxdim,
        )
    else:
        h1 = ()

    all_bars = sorted([*h0, *h1], key=lambda b: b.persistence, reverse=True)
    return PersistenceBarcode(bars=tuple(all_bars), n_points=len(prices))


# ====================================================================
# 级别判断（active bars / dominant level / 噪声阈值）
# ====================================================================


def active_bars(
    barcode: PersistenceBarcode,
    tau: float,
    *,
    dimension: int | None = None,
) -> tuple[Bar, ...]:
    """当前所有"在运作"的级别 = persistence 超过噪声阈值 τ 的 bar。

    噪声以下的特征视为不构成级别（震荡/毛刺）。返回按 persistence 降序，
    即"从主导级别到最次级别"的有序级别谱。

    Parameters
    ----------
    barcode : PersistenceBarcode
    tau : float
        噪声阈值（建议用 atr_noise_threshold 计算）。
    dimension : int | None
        限定维度（None = 所有维度一起排序）。
    """
    selected = [
        b
        for b in barcode.bars
        if b.persistence > tau and (dimension is None or b.dimension == dimension)
    ]
    selected.sort(key=lambda b: b.persistence, reverse=True)
    return tuple(selected)


def dominant_bar(
    barcode: PersistenceBarcode,
    *,
    dimension: int | None = None,
    tau: float = 0.0,
) -> Bar | None:
    """当前主导级别 = argmax(persistence)（超过 τ 的特征中持续度最大者）。

    无满足条件的特征时返回 None。
    """
    actives = active_bars(barcode, tau, dimension=dimension)
    return actives[0] if actives else None


def atr(
    highs: Sequence[float],
    lows: Sequence[float],
    closes: Sequence[float],
    *,
    period: int = 14,
) -> float:
    """Average True Range——噪声阈值的量纲基准。

    TR_t = max(high-low, |high-prev_close|, |low-prev_close|)。
    ATR = 最近 period 个 TR 的均值（数据不足时用全部可用 TR）。

    Parameters
    ----------
    highs, lows, closes : Sequence[float]
        等长价格序列。
    period : int
        平滑窗口。

    Returns
    -------
    float
        ATR（≥ 0）。序列过短时返回 0.0。
    """
    n = len(closes)
    if n < 2 or len(highs) != n or len(lows) != n:
        return 0.0

    trs: list[float] = []
    for t in range(1, n):
        prev_close = float(closes[t - 1])
        tr = max(
            float(highs[t]) - float(lows[t]),
            abs(float(highs[t]) - prev_close),
            abs(float(lows[t]) - prev_close),
        )
        trs.append(tr)

    window = trs[-period:] if len(trs) >= period else trs
    return sum(window) / len(window) if window else 0.0


def atr_noise_threshold(
    highs: Sequence[float],
    lows: Sequence[float],
    closes: Sequence[float],
    *,
    period: int = 14,
    multiple: float = 1.0,
) -> float:
    """噪声阈值 τ = multiple × ATR。

    persistence 低于 τ 的特征视为噪声，不计入级别谱。multiple 可配置：
    越大越保守（只认大级别），越小越敏感。
    """
    return multiple * atr(highs, lows, closes, period=period)


# ====================================================================
# 挂载到缠论递归的每一层（Protocol-based，不修改现有类）
# ====================================================================


@runtime_checkable
class PriceRanged(Protocol):
    """可定位到原始 bar 区间的缠论对象的最小接口。

    任何能给出 [raw_i0, raw_i1] 原始 K 线索引区间的对象（笔/线段/Move 经映射后）
    都可挂载 barcode。本协议不要求对象暴露价格——价格由调用方的序列提供。
    """

    @property
    def raw_i0(self) -> int:
        """对象覆盖的原始 bar 起始索引。"""
        ...

    @property
    def raw_i1(self) -> int:
        """对象覆盖的原始 bar 终止索引（闭区间）。"""
        ...


@dataclass(frozen=True, slots=True)
class BarcodeAnnotation:
    """一个缠论对象的 barcode 标注（不侵入原对象）。

    Attributes
    ----------
    component_idx : int
        对象在其层级序列中的位置。
    raw_i0, raw_i1 : int
        对象对应的原始价格区间。
    barcode : PersistenceBarcode
        该区间价格序列的持续同调。
    """

    component_idx: int
    raw_i0: int
    raw_i1: int
    barcode: PersistenceBarcode


def _default_resolve_range(obj: object) -> tuple[int, int] | None:
    """默认区间解析：依次尝试 (raw_i0,raw_i1) / (i0,i1)。

    线段(Segment) 用 i0/i1（merged bar 索引）；若调用方已映射到 raw，
    可让对象暴露 raw_i0/raw_i1。解析不到返回 None（跳过该对象）。
    """
    for lo, hi in (("raw_i0", "raw_i1"), ("i0", "i1")):
        if hasattr(obj, lo) and hasattr(obj, hi):
            return int(getattr(obj, lo)), int(getattr(obj, hi))
    return None


def attach_barcodes(
    components: Sequence[object],
    prices: Sequence[float],
    *,
    resolve_range=_default_resolve_range,
    min_points: int = 4,
    **barcode_kwargs,
) -> tuple[BarcodeAnnotation, ...]:
    """为一层缠论对象批量挂载 barcode——递归每层调用一次即可。

    这是"barcode 随递归自动生成"的集成点：把它放进递归循环的每一层
    （每产出一层 Move/Segment 就调用一次），无需逐对象手工调用。设计为
    **可组合的 overlay**，不修改 RecursiveLevelEngine 等现有引擎（immutable
    + Protocol，符合项目 coding-style）。

    Parameters
    ----------
    components : Sequence[object]
        一层的缠论对象（线段 / Move-as-component / 经映射的笔）。
    prices : Sequence[float]
        原始价格序列（barcode 在 D 之前的原始价格上计算）。
    resolve_range : Callable[[object], tuple[int,int] | None]
        把对象解析为 [raw_i0, raw_i1]。默认尝试 raw_i0/i0 等属性。
    min_points : int
        区间样本数下限，低于此值跳过（barcode 无意义）。
    **barcode_kwargs
        透传给 barcode_from_prices（maxdim / embedding_dim / ...）。

    Returns
    -------
    tuple[BarcodeAnnotation, ...]
        每个可解析且样本充足的对象一条标注。

    认识论等级：L0（barcode 计算）。挂载关系本身是确定性映射。
    """
    n = len(prices)
    annotations: list[BarcodeAnnotation] = []
    for idx, obj in enumerate(components):
        rng = resolve_range(obj)
        if rng is None:
            continue
        i0, i1 = rng
        i0 = max(0, min(i0, n - 1))
        i1 = max(0, min(i1, n - 1))
        if i1 < i0 or (i1 - i0 + 1) < min_points:
            continue
        segment_prices = prices[i0 : i1 + 1]
        barcode = barcode_from_prices(segment_prices, **barcode_kwargs)
        annotations.append(
            BarcodeAnnotation(
                component_idx=idx,
                raw_i0=i0,
                raw_i1=i1,
                barcode=barcode,
            )
        )
    return tuple(annotations)
