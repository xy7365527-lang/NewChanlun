"""A 系统 — 拓扑背驰：Wasserstein-1 距离替代 MACD 面积对比。

存在论位置
----------
这是力度计算的**第四种方法**，与 a_divergence.py 的 MACD 面积、价格振幅
fallback、以及 directional_force.py 的方向性力度并列。接口对齐
a_divergence._compute_force（输入一段走势，输出 float 力度），可作为其
force 函数的替换。

力度的拓扑表达
-------------
一段走势的"力度" = 其 PersistenceBarcode 的**总持续度** sum(persistence)。
这恰好等于 persistence diagram 到对角线（空图）的 **Wasserstein-1 距离**——
即把所有特征都"抹平到无特征"所需的最小搬运量。它替代 MACD 柱子面积作为
力度标量：

    MACD：force_a = area_A,            force_c = area_C
    拓扑：force_a = totalpers(bar_A),  force_c = totalpers(bar_C)
    背驰：force_c < force_a（力量衰竭），两者同构。

此外提供 A、C 两段 diagram 之间的**直接 Wasserstein-1 距离** wasserstein_ac，
度量两段走势的**结构重组**程度（不止幅度差异）——这是 MACD 面积差给不出的信息。

与 ker(D) 的边界条件（239号谱系）
--------------------------------
H0 总持续度 ≈ 幅度维度（D 选择性吸收的维度）。因此当 dimension=0 时，拓扑
力度与振幅力度高度相关，可能与方向性力度（∉ ker(D)）**判定不一致**——
此时方向性力度更可信（239号）。dimension=1（中枢 loop）携带振荡几何，
信息超出纯幅度，是拓扑力度相对独立的部分。

**结论翻转的边界**：若 dimension=0 且走势近似单调（H0 退化为单一全幅 bar），
拓扑力度 ≡ 振幅力度 ∈ ker(D)——此时 is_divergent 不应作为独立证据。

认识论等级
----------
- Wasserstein 距离 / 总持续度计算：**L0**（纯算法）。
- "拓扑背驰 → 买卖点"等经验断言：真实数据 L2/L3 验证前停留 **L1**。

概念溯源标签
-----------
- 拓扑背驰（Wasserstein-1 力度）[新缠论:候选——并行动力学度量]
- MACD 面积 [缠论知识库 §9.3]
- 振幅力度 ∈ ker(D) [新缠论:239号]
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Sequence

from newchan.a_persistence_barcode import (
    PersistenceBarcode,
    barcode_from_prices,
)

__all__ = [
    "TopoDivergence",
    "topo_force",
    "topo_divergence",
    "topo_divergence_from_prices",
]


@dataclass(frozen=True, slots=True)
class TopoDivergence:
    """拓扑背驰判定结果（接口对齐 a_divergence.Divergence 的力度语义）。

    Attributes
    ----------
    force_a : float
        A 段力度 = barcode_A 在指定维度上的总持续度。
    force_c : float
        C 段力度 = barcode_C 在指定维度上的总持续度。
    ratio : float
        force_c / force_a（force_a=0 时为 0.0）。< 1 表示力度衰减。
    is_divergent : bool
        是否背驰（力度衰竭）。
    dominant_level_persistence : float
        C 段（当前段）的主导级别持续度 = max(persistence)。用于级别判断：
        当前在运作的最大级别有多强。
    wasserstein_ac : float
        A、C 两段 diagram 之间的 Wasserstein-1 距离 = 结构重组程度。
    dimension : int
        判定所用的同调维度（0 = 笔/趋势级，1 = 中枢级）。
    """

    force_a: float
    force_c: float
    ratio: float
    is_divergent: bool
    dominant_level_persistence: float
    wasserstein_ac: float
    dimension: int


def topo_force(
    barcode: PersistenceBarcode,
    *,
    dimension: int | None = 1,
) -> float:
    """一段走势的拓扑力度 = 总持续度（= W1 到对角线）。

    第四种 force 方法。可直接替换 a_divergence._compute_force 作为力度标量。

    Parameters
    ----------
    barcode : PersistenceBarcode
        该段走势原始价格的持续同调。
    dimension : int | None
        取哪个维度的力度。1 = 中枢 loop（推荐，信息最独立于 ker(D)）；
        0 = 笔/趋势 prominence；None = 所有维度合计。

    Returns
    -------
    float
        总持续度（≥ 0）。
    """
    return barcode.total_persistence(dimension)


def _diagram_array(barcode: PersistenceBarcode, dimension: int):
    """把指定维度的 bar 转成 persim 需要的 (n, 2) [birth, death] 数组。

    barcode 中所有 death 均有限（H0 已封顶、H1 已丢弃 inf），可直接喂给
    persim.wasserstein。
    """
    import numpy as np

    pairs = [
        (b.birth, b.death) for b in barcode.bars if b.dimension == dimension
    ]
    if not pairs:
        return np.empty((0, 2), dtype=float)
    return np.asarray(pairs, dtype=float)


def topo_divergence(
    barcode_a: PersistenceBarcode,
    barcode_c: PersistenceBarcode,
    *,
    dimension: int = 1,
    noise_floor: float = 0.0,
) -> TopoDivergence:
    """用持续同调判定 A→C 两段走势是否背驰（力度衰竭）。

    判定准则（与 MACD 面积同构）：A 段有力度（> noise_floor）且 C 段总持续度
    严格小于 A 段 → 背驰。dimension 默认 1（中枢 loop，信息最独立于 ker(D)）。

    Parameters
    ----------
    barcode_a, barcode_c : PersistenceBarcode
        A、C 两段走势各自原始价格的持续同调。
    dimension : int
        判定所用同调维度。
    noise_floor : float
        A 段力度需超过此下限才进入判定（避免在噪声段误报）。建议传入
        与 dimension 量纲一致的阈值（H0 用价格量纲，可由 ATR 导出）。

    Returns
    -------
    TopoDivergence
    """
    force_a = topo_force(barcode_a, dimension=dimension)
    force_c = topo_force(barcode_c, dimension=dimension)

    ratio = force_c / force_a if force_a > 0 else 0.0
    is_divergent = force_a > noise_floor and force_c < force_a

    dominant = barcode_c.max_persistence(dimension)

    diagram_a = _diagram_array(barcode_a, dimension)
    diagram_c = _diagram_array(barcode_c, dimension)
    if diagram_a.shape == diagram_c.shape and (diagram_a == diagram_c).all():
        # W(D, D) 按定义精确为 0。绕过 Hungarian 求解器，避免不同
        # scipy/persim 平台对同一 diagram 留下非零浮点残差。
        w_ac = 0.0
    else:
        import persim

        w_ac = float(persim.wasserstein(diagram_a, diagram_c))

    return TopoDivergence(
        force_a=force_a,
        force_c=force_c,
        ratio=ratio,
        is_divergent=is_divergent,
        dominant_level_persistence=dominant,
        wasserstein_ac=w_ac,
        dimension=dimension,
    )


def topo_divergence_from_prices(
    prices_a: Sequence[float],
    prices_c: Sequence[float],
    *,
    dimension: int = 1,
    noise_floor: float = 0.0,
    **barcode_kwargs,
) -> TopoDivergence:
    """便捷入口：直接从 A、C 两段价格序列判定拓扑背驰。

    内部对两段各算一次 barcode 再比较。barcode_kwargs 透传给
    barcode_from_prices（maxdim / embedding_dim / embedding_delay / ...）。

    Parameters
    ----------
    prices_a, prices_c : Sequence[float]
        A、C 两段的原始价格序列。
    dimension : int
        判定维度。
    noise_floor : float
        A 段力度下限。
    **barcode_kwargs
        透传给 barcode_from_prices。

    Returns
    -------
    TopoDivergence
    """
    barcode_a = barcode_from_prices(prices_a, **barcode_kwargs)
    barcode_c = barcode_from_prices(prices_c, **barcode_kwargs)
    return topo_divergence(
        barcode_a,
        barcode_c,
        dimension=dimension,
        noise_floor=noise_floor,
    )
