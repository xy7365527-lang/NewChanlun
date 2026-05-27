"""A 系统 — 几何动量 over 拓扑特征：对 MACD 柱（hist）序列做 H0 持续同调。

⚠ 命名与定位（521号已结算定理，强制阅读——避免声明膨胀 090号）
--------------------------------------------------------------
521号定理（L0）：**纯拓扑因果动量不变量不存在**——动量（背驰）= 红绿柱面积的时间
积分（第24课），需要时间度量；拓扑不变量定义上在时间重参数化下不变（丢弃时间）。
两者直接冲突 → 任何纯 PH 构造不能编码动量。本模块原名"动量拓扑"被 521号判为
workaround 陷阱（声称拓扑能产生动量），故重命名为 521号术语 **「几何动量 over
拓扑特征」**，并显式标注几何非拓扑：

  1. **动量来自 MACD（几何，非拓扑）**：输入是**全局 MACD 的 hist**，hist 已编码
     跨笔连续 EMA 的时间积分 + 0 轴跨段基准（§5 原因二）——这是几何/度量结构。
  2. **PH 只提取 hist 的拓扑结构**（摆动 prominence 的 persistence diagram）——
     拓扑提供"何时/何物"，几何（MACD）提供"多快/多强"。
  3. 故 `total_persistence(hist)` 是**几何动量序列上的拓扑泛函**，能 work 完全因为
     MACD 已编码动量；它**不是**也**不可能是**纯拓扑动量（521号）。

与 520号否决的区别：520 否决"路径空间 (t,p) PH 从笔内几何**重建**动量"（信息论上
访问不到跨笔 EMA）。本模块不重建动量——它读取 MACD **已算出**的动量序列的拓扑形态，
是 §6 允许的"PH 作监视层读取动量结构"。

硬约束：必须用**全局 MACD 的 hist**（保留跨笔记忆），再按段切片——绝不在段内重置
EMA（段内重置 = 退回 §5/520 否决的退化版本，丢掉 0 轴跨段基准）。

合法 vs 非法用途（521号裁定）
-----------------------------
- ✅ **合法**：生成态证伪探针（testing-override + 231号否定性结果价值）；§6 监视层
  读取 MACD 动量结构；研究"拓扑提取泛函 vs 积分泛函"的参数鲁棒性差异（模块3）。
- ❌ **非法**：作为"纯拓扑统一框架"的动量层（521号定理证否）；声称可独立于 MACD
  产出背驰判据。背驰的**动量闸必须 MACD**（521号下游推论，§6/§14）。

几何动量的两个 over-hist 泛函（模块3 验证，与定理不冲突）
-------------------------------------------------------
缠论背驰（第24课）：C 段 MACD 柱面积 < A 段 → 背驰。面积 = Σ|hist|（积分泛函）。
本模块提供另一个 over-hist 泛函：persistence（hist 摆动 prominence）：
- `total_persistence(hist)` = hist 摆动结构累加（面积的拓扑类比）。
- `max_persistence(hist)` = 主导动量摆动 prominence。

可验证增量（不与 521号冲突——两个泛函都 over 同一 MACD 几何动量）：persistence
泛函对 MACD 参数（fast/slow/signal）扰动是否比积分泛函（面积）更鲁棒。这比较的是
"同一几何动量的两个 over-hist 泛函的参数稳定性"，**不是**"拓扑 vs 几何"。模块3
在 700 上 L2 实测。本模块只提供确定性算法（L0）。

认识论等级（formalization-validity-domain 规则）
-----------------------------------------------
- hist 计算 + H0 + 力度比值：**L0**（纯算法，确定性，零信息增量）。
- "persistence 泛函比面积泛函更抗参数"经验断言：腾讯 700 L2（模块3，可否证）。

概念溯源标签
-----------
- 几何动量 over 拓扑特征 [新缠论:521号——动量是 MACD 几何，PH 仅提取其拓扑结构]
- persistence 泛函 vs 面积泛函 [新缠论:候选——同一 MACD 动量的两个 over-hist 泛函]
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

import numpy as np
import pandas as pd

from newchan.a_macd import compute_macd
from newchan.a_persistence_barcode import Bar, sublevel_h0_bars

__all__ = [
    "MomentumBarcode",
    "MomentumDivergence",
    "macd_hist",
    "momentum_barcode",
    "momentum_divergence",
]

ForceMetric = Literal["total_persistence", "max_persistence"]


# ====================================================================
# 数据类型（frozen + slots，immutable）
# ====================================================================


@dataclass(frozen=True, slots=True)
class MomentumBarcode:
    """一段 MACD hist 序列的 H0 持续同调（动量结构的拓扑表达）。

    Attributes
    ----------
    bars : tuple[Bar, ...]
        hist 序列的 H0 特征（persistence 降序）。每个 bar 的 persistence = 一个
        动量摆动的 prominence（hist 局部极值的显著性）。
    n_points : int
        hist 切片的样本数。
    macd_params : tuple[int, int, int]
        (fast, slow, signal)——产出此 barcode 的 MACD 参数（鲁棒性对比用）。
    hist_slice : tuple[float, ...]
        参与计算的 hist 值（保留以供 diagram / 面积对照）。
    """

    bars: tuple[Bar, ...]
    n_points: int
    macd_params: tuple[int, int, int]
    hist_slice: tuple[float, ...]

    @property
    def total_persistence(self) -> float:
        """总持续度 = Σ persistence ≈ MACD 面积的拓扑类比（累加动量结构）。"""
        return sum(b.persistence for b in self.bars)

    @property
    def max_persistence(self) -> float:
        """主导动量摆动 prominence（单波最强动量）。无特征为 0。"""
        return max((b.persistence for b in self.bars), default=0.0)

    @property
    def macd_area(self) -> float:
        """传统 MACD 面积 |Σ hist|——同切片上的对照量（非拓扑）。"""
        return float(abs(sum(self.hist_slice)))

    def diagram(self) -> np.ndarray:
        """persistence diagram 的 (birth, death) 数组（persim/bottleneck 用）。

        空 barcode 返回形状 (0, 2) 的数组——persim.bottleneck 接受空图。
        """
        if not self.bars:
            return np.empty((0, 2), dtype=float)
        return np.asarray([[b.birth, b.death] for b in self.bars], dtype=float)

    def force(self, metric: ForceMetric = "total_persistence") -> float:
        """按指定度量取该段动量力度。"""
        return (
            self.total_persistence
            if metric == "total_persistence"
            else self.max_persistence
        )


@dataclass(frozen=True, slots=True)
class MomentumDivergence:
    """A 段 vs C 段的动量拓扑背驰判定（与 §3 MACD 面积背驰同构）。

    Attributes
    ----------
    force_a, force_c : float
        A 段、C 段的动量力度（按 metric）。
    ratio : float
        force_c / force_a（< 1 = C 段动量更弱 = 背驰方向）。force_a=0 时为 inf。
    is_divergent : bool
        是否背驰 = (force_a > noise_floor) ∧ (force_c < force_a)。
    metric : str
        所用力度度量。
    area_a, area_c : float
        同段 MACD 面积（|Σhist|）对照——供"动量拓扑 vs 面积"一致性验证。
    area_is_divergent : bool
        传统面积判据：area_c < area_a（且 area_a > 0）。
    agrees_with_area : bool
        拓扑背驰与面积背驰是否给出相同 bool（一致性，模块4/L2 核心观测量）。
    """

    force_a: float
    force_c: float
    ratio: float
    is_divergent: bool
    metric: str
    area_a: float
    area_c: float
    area_is_divergent: bool
    agrees_with_area: bool


# ====================================================================
# MACD hist（全局，保留跨笔 EMA 记忆——§5 硬约束）
# ====================================================================


def macd_hist(
    prices, *, fast: int = 12, slow: int = 26, signal: int = 9
) -> tuple[float, ...]:
    """全局 MACD 柱序列 hist = macd_line − signal_line（用全序列连续 EMA）。

    **必须传入完整价格序列**——hist 的每个值依赖跨笔连续 EMA 与 0 轴跨段基准
    （§5 原因二）。切片在 hist 上做（momentum_barcode 的 i0/i1），不在价格上重置 EMA。
    """
    df = pd.DataFrame({"close": [float(p) for p in prices]})
    hist = compute_macd(df, fast=fast, slow=slow, signal=signal)["hist"]
    return tuple(float(v) for v in hist.to_numpy())


# ====================================================================
# 动量 barcode（全局 MACD → hist 切片 → H0）
# ====================================================================


def momentum_barcode(
    prices,
    *,
    i0: int = 0,
    i1: int | None = None,
    fast: int = 12,
    slow: int = 26,
    signal: int = 9,
) -> MomentumBarcode:
    """对 [i0, i1] 段的全局 MACD hist 做 H0 持续同调。

    流程：全序列 `prices` → 全局 MACD（保留跨笔 EMA，§5）→ hist[i0:i1+1] →
    sublevel_h0_bars。

    Parameters
    ----------
    prices : Sequence[float]
        **完整**价格序列（不是预切片段——切片在 hist 上做以保留全局 EMA）。
    i0, i1 : int
        段的闭区间索引（i1=None → 序列末端）。
    fast, slow, signal : int
        MACD 参数（鲁棒性实验在此扫描）。

    Returns
    -------
    MomentumBarcode

    认识论等级：L0（纯算法）。
    """
    prices = [float(p) for p in prices]
    n = len(prices)
    hist = macd_hist(prices, fast=fast, slow=slow, signal=signal)
    if i1 is None:
        i1 = n - 1
    i0 = max(0, min(i0, n - 1))
    i1 = max(0, min(i1, n - 1))
    seg = hist[i0 : i1 + 1] if i1 >= i0 else ()
    bars = sublevel_h0_bars(seg) if seg else ()
    return MomentumBarcode(
        bars=tuple(bars),
        n_points=len(seg),
        macd_params=(fast, slow, signal),
        hist_slice=tuple(seg),
    )


# ====================================================================
# 动量背驰（A 段 vs C 段，与 §3 MACD 面积背驰同构）
# ====================================================================


def momentum_divergence(
    prices,
    a_range: tuple[int, int],
    c_range: tuple[int, int],
    *,
    fast: int = 12,
    slow: int = 26,
    signal: int = 9,
    metric: ForceMetric = "total_persistence",
    noise_floor: float = 0.0,
) -> MomentumDivergence:
    """A 段（在前）vs C 段（在后）的动量拓扑背驰 + 与 MACD 面积判据的一致性。

    背驰 = C 段动量力度 < A 段（且 A 段力度 > noise_floor，排除对噪声的算术比较）。
    同时计算传统 MACD 面积判据，输出 `agrees_with_area`——这是"动量拓扑背驰 vs
    传统面积背驰是否一致"（任务模块2/4 的 L2 核心观测量）。

    全局 MACD 只算一次（同一把 EMA 尺子），A/C 段在同一 hist 上切片——保证两段的
    0 轴基准一致（§5），背驰对比才有意义。

    Parameters
    ----------
    prices : Sequence[float]
        完整价格序列。
    a_range, c_range : tuple[int, int]
        A 段、C 段的闭区间索引 [i0, i1]。
    metric : ForceMetric
        力度度量（默认 total_persistence ≈ 面积类比）。
    noise_floor : float
        A 段力度下限（低于此值视为无实质动量，背驰判定退化）。

    Returns
    -------
    MomentumDivergence

    认识论等级：算法 L0；"拓扑背驰≡面积背驰"经验断言 L1（待 700 L2）。
    """
    prices = [float(p) for p in prices]
    hist = macd_hist(prices, fast=fast, slow=slow, signal=signal)

    def _seg_barcode(rng: tuple[int, int]) -> MomentumBarcode:
        i0, i1 = rng
        i0 = max(0, min(i0, len(prices) - 1))
        i1 = max(0, min(i1, len(prices) - 1))
        seg = hist[i0 : i1 + 1] if i1 >= i0 else ()
        return MomentumBarcode(
            bars=tuple(sublevel_h0_bars(seg)) if seg else (),
            n_points=len(seg),
            macd_params=(fast, slow, signal),
            hist_slice=tuple(seg),
        )

    bc_a = _seg_barcode(a_range)
    bc_c = _seg_barcode(c_range)

    force_a = bc_a.force(metric)
    force_c = bc_c.force(metric)
    ratio = force_c / force_a if force_a > 0 else float("inf")
    is_div = force_a > noise_floor and force_c < force_a

    area_a, area_c = bc_a.macd_area, bc_c.macd_area
    area_div = area_a > 0 and area_c < area_a

    return MomentumDivergence(
        force_a=force_a,
        force_c=force_c,
        ratio=ratio,
        is_divergent=is_div,
        metric=metric,
        area_a=area_a,
        area_c=area_c,
        area_is_divergent=area_div,
        agrees_with_area=(is_div == area_div),
    )
