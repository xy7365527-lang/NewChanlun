"""A 系统 — 买点质量评分：纯 PH（形态学层）的无参数买点打分。

存在论位置（§17.1 形态学层的聚合 + §17.3 边界）
------------------------------------------------
`persistence_theory.md` §17.1 列出 PH 无参数覆盖的形态学判据（级别、中枢计数、区间套、
alive/settled、走势完成度）。本模块把这些判据**聚合为一个 0-100 的买点质量分**。

**严格边界（§17.3 / 521号，违反=声明膨胀）**：本评分是**形态学层**产出，只能是买点的
**结构前置/必要条件**，**不是充分条件**。充分条件需动力学层（MACD 力度背驰，第24课），
PH 定义上无法产生（521号：纯拓扑动量不存在）。高分 ≠ 买入信号，只表示"结构上已具备
买点候选的形态前提"。

第5维（力度衰竭）的诚实标注（521号 / 239号 ker(D)，必读）
--------------------------------------------------------
第5维用连续 settled 摆动的 persistence 递减度量。**persistence ≈ 幅度 ∈ ker(D)**
（`a_persistence_barcode` 顶部 235/237/239号）——它测的是**结构振幅在缩小**=缠论的
**"振幅收窄"**，是**形态学层的振幅衰减信号**，**不是动力学层的力度背驰**（MACD 面积）。
故字段命名为 `amplitude_decay`（非 momentum/force_divergence），并在解释里标注：
这一维**不能**替代真背驰确认（521号），它只补充"摆动幅度是否在收敛"的形态前提。

认识论等级
----------
- 五维计算 + 归一化映射：**L0**（从 barcode 确定性推导；归一化的具体映射是显式值判断）。
- 综合分的组合方式（加权/乘积）：**值判断**（learning-mode 决策点，ScorePolicy 暴露）。
- "高分 = 买点候选"经验断言：**L2**（单标的可否证；跨标的 L3 未做）。

设计约束（coding-style）
------------------------
- BuyPointScore / ScorePolicy：frozen + slots，immutable。
- 归一化映射是显式值判断（saturating 函数），不伪装成客观阈值。

概念溯源标签
-----------
- 形态学买点评分 [新缠论:候选——§17.1 判据聚合]
- 振幅衰减 ≠ 力度背驰 [新缠论:521号/239号 ker(D)]
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Sequence

from newchan.a_level_detection import detect_levels
from newchan.a_online_persistence import MergeBar, OnlineMergeTree
from newchan.a_ph_zhongshu import ZhongshuPolicy, build_containment_forest, detect_zhongshu

__all__ = [
    "ScorePolicy",
    "BuyPointScore",
    "score_buypoint",
    "score_buypoint_from_prices",
    "containment_max_depth",
]


# ====================================================================
# 评分策略（值判断 —— 塑造各维权重与综合方式）
# ====================================================================


@dataclass(frozen=True, slots=True)
class ScorePolicy:
    """综合评分策略（值判断，learning-mode 决策点）。

    Attributes
    ----------
    weights : tuple[float, float, float, float, float]
        五维权重（结构完成度、中枢计数、区间套深度、alive干净度、振幅衰减），
        加权模式下归一化后加权平均。默认等权。
    mode : str
        "weighted"（加权平均）或 "product"（几何乘积 = 五维都好才高分，
        任一维短板拖累全局——更保守）。
    noise_multiple : float
        噪声阈值 = noise_multiple × ATR（中枢/级别检测的 noise_floor）。
    zhongshu_target : float
        中枢计数归一化目标（缠论：≥2 中枢=趋势）。count/target 截断到 [0,1]。
    """

    weights: tuple[float, float, float, float, float] = (1.0, 1.0, 1.0, 1.0, 1.0)
    mode: str = "weighted"
    noise_multiple: float = 1.0
    zhongshu_target: float = 2.0


# ====================================================================
# 评分结果（immutable）
# ====================================================================


@dataclass(frozen=True, slots=True)
class BuyPointScore:
    """五维形态学买点评分（每维 0-1）+ 综合分（0-100）+ 缠论解释。

    Attributes
    ----------
    structure_completion : float
        结构完成度 = 最近 settled 下跌分量 persistence / 全局最大 persistence。
    zhongshu_count : int
        中枢个数（detect_zhongshu）。
    zhongshu_score : float
        中枢计数归一化（count / target 截断）。
    nesting_depth : int
        区间套最大嵌套层数（包含森林最长链）。
    nesting_score : float
        嵌套深度归一化（1 − 0.5**depth）。
    alive_cleanliness : float
        1 − alive_count / total_count（有多少走势还没完成）。
    amplitude_decay : float
        **形态学层振幅衰减**（非力度背驰，521号）= clamp(1 − recent/prev, 0, 1)，
        最近两个 settled 摆动的 persistence 比值。
    composite_weighted : float
        加权平均综合分（0-100）。
    composite_product : float
        几何乘积综合分（0-100）。
    explanations : tuple[str, ...]
        各维的缠论解释（含第5维的诚实标注）。
    """

    structure_completion: float
    zhongshu_count: int
    zhongshu_score: float
    nesting_depth: int
    nesting_score: float
    alive_cleanliness: float
    amplitude_decay: float
    composite_weighted: float
    composite_product: float
    explanations: tuple[str, ...] = field(default_factory=tuple)

    @property
    def dimensions(self) -> tuple[float, float, float, float, float]:
        """五维归一化值（0-1），顺序与 ScorePolicy.weights 对齐。"""
        return (
            self.structure_completion,
            self.zhongshu_score,
            self.nesting_score,
            self.alive_cleanliness,
            self.amplitude_decay,
        )


# ====================================================================
# 区间套深度（包含森林最长链）
# ====================================================================


def containment_max_depth(bars: Sequence[MergeBar]) -> int:
    """价格区间包含森林的最大嵌套层数（缠论：区间套确认的级别数）。

    用 `build_containment_forest`（直接父→子映射，-1 为虚根）做 DFS 求最长链。
    深度 = 从某根到最深叶的边数（单个 bar 无嵌套 → 0）。

    认识论等级：L0（确定性，从区间包含推导）。
    """
    if not bars:
        return 0
    forest = build_containment_forest(bars)

    def depth(node: int) -> int:
        children = forest.get(node, [])
        if not children:
            return 0
        return 1 + max(depth(c) for c in children)

    # -1 是虚根，其直接子是"顶层 bar"（depth 0 起算）
    roots = forest.get(-1, [])
    if not roots:
        return 0
    return max(depth(r) for r in roots)


# ====================================================================
# 核心评分
# ====================================================================


def _clamp01(x: float) -> float:
    return 0.0 if x < 0.0 else 1.0 if x > 1.0 else x


def _recent_settled_down_persistence(settled: Sequence[MergeBar]) -> float:
    """最近 settled 分量的 persistence（death_idx 最大者；全 H0 sublevel 特征皆为
    valley-prominence=下跌摆动幅度）。无 settled 返回 0。"""
    finite = [b for b in settled if b.death_idx is not None]
    if not finite:
        return 0.0
    recent = max(finite, key=lambda b: b.death_idx)  # type: ignore[arg-type]
    return recent.persistence


def score_buypoint(
    tree: OnlineMergeTree,
    *,
    highs: Sequence[float] | None = None,
    lows: Sequence[float] | None = None,
    closes: Sequence[float] | None = None,
    policy: ScorePolicy | None = None,
) -> BuyPointScore:
    """从活的 OnlineMergeTree（current_barcode 快照，因果）计算买点质量分。

    用 `current_barcode()` 而非 `finalize()`——买点是**当前因果决策**，不能用未来
    （finalize 含 hindsight，§7.5）。alive 分量 death 用 cap 估计（current_barcode 约定）。

    Parameters
    ----------
    highs, lows, closes : Sequence[float] | None
        用于 ATR 噪声阈值（中枢/级别检测的 noise_floor）。缺省则 noise_floor=0。
    policy : ScorePolicy | None
        评分策略（默认 ScorePolicy()）。

    认识论等级：五维 L0；组合方式值判断；"高分=买点候选" L2（调用方标注）。
    """
    policy = policy or ScorePolicy()
    snap = tree.current_barcode()
    bars = snap.all_bars  # settled + alive（因果快照，persistence 降序）
    expl: list[str] = []

    # 噪声阈值
    if highs is not None and lows is not None and closes is not None:
        from newchan.a_persistence_barcode import atr_noise_threshold

        noise = atr_noise_threshold(highs, lows, closes, multiple=policy.noise_multiple)
    else:
        noise = 0.0

    # 维度1：结构完成度
    global_max = bars[0].persistence if bars else 0.0
    recent_down = _recent_settled_down_persistence(snap.settled_bars)
    structure_completion = _clamp01(recent_down / global_max) if global_max > 0 else 0.0
    expl.append(
        f"结构完成度={structure_completion:.3f}：最近 settled 下跌摆动幅度 "
        f"{recent_down:.2f} / 全局最大 {global_max:.2f}（走势完成多少；低=大级别下跌未完成）"
    )

    # 维度2：中枢计数
    zhongshus = detect_zhongshu(bars, noise_floor=noise, policy=ZhongshuPolicy())
    zcount = len(zhongshus)
    zscore = _clamp01(zcount / policy.zhongshu_target) if policy.zhongshu_target > 0 else 0.0
    expl.append(
        f"中枢计数={zcount}（归一{zscore:.3f}）：缠论 ≥2 同向中枢=趋势、1=盘整、0=无结构"
    )

    # 维度3：区间套深度
    depth = containment_max_depth(bars)
    nesting_score = _clamp01(1.0 - 0.5 ** depth)
    expl.append(
        f"区间套深度={depth}（归一{nesting_score:.3f}=1−0.5^depth）：嵌套层数=区间套确认的级别数"
    )

    # 维度4：alive 干净度
    alive_n = len(snap.alive_bars)
    total_n = alive_n + len(snap.settled_bars)
    alive_clean = _clamp01(1.0 - alive_n / total_n) if total_n > 0 else 0.0
    expl.append(
        f"alive干净度={alive_clean:.3f}=1−{alive_n}/{total_n}：有多少走势还没完成"
        f"（注：全局分量恒 alive，故 <1 是常态）"
    )

    # 维度5：振幅衰减（形态学层，非力度背驰——521号）
    finite_settled = sorted(
        (b for b in snap.settled_bars if b.death_idx is not None),
        key=lambda b: b.death_idx,  # type: ignore[arg-type,return-value]
    )
    if len(finite_settled) >= 2:
        prev_p = finite_settled[-2].persistence
        recent_p = finite_settled[-1].persistence
        amplitude_decay = _clamp01(1.0 - recent_p / prev_p) if prev_p > 0 else 0.0
        decay_note = (
            f"振幅衰减={amplitude_decay:.3f}=clamp(1−{recent_p:.2f}/{prev_p:.2f})："
            f"最近两个 settled 摆动的幅度比 [形态学层振幅收窄，**非**MACD力度背驰，521号]"
        )
    else:
        amplitude_decay = 0.0
        decay_note = (
            "振幅衰减=0.000：settled 摆动 <2 个，无法评估 "
            "[形态学层振幅信号，**非**MACD力度背驰；真力度背驰需动力学层，521号]"
        )
    expl.append(decay_note)

    # 综合
    dims = (structure_completion, zscore, nesting_score, alive_clean, amplitude_decay)
    w = policy.weights
    wsum = sum(w)
    weighted = (sum(d * wi for d, wi in zip(dims, w)) / wsum) if wsum > 0 else 0.0
    # 几何乘积（任一维短板拖累）：用 w 作指数权重
    product = 1.0
    for d, wi in zip(dims, w):
        product *= max(d, 1e-9) ** (wi / wsum if wsum > 0 else 0.0)

    return BuyPointScore(
        structure_completion=structure_completion,
        zhongshu_count=zcount,
        zhongshu_score=zscore,
        nesting_depth=depth,
        nesting_score=nesting_score,
        alive_cleanliness=alive_clean,
        amplitude_decay=amplitude_decay,
        composite_weighted=round(weighted * 100.0, 2),
        composite_product=round(product * 100.0, 2),
        explanations=tuple(expl),
    )


def score_buypoint_from_prices(
    closes: Sequence[float],
    *,
    highs: Sequence[float] | None = None,
    lows: Sequence[float] | None = None,
    policy: ScorePolicy | None = None,
) -> BuyPointScore:
    """逐根 update closes 后评分（不 finalize，因果）。"""
    tree = OnlineMergeTree()
    for p in closes:
        tree.update(p)
    return score_buypoint(
        tree, highs=highs, lows=lows, closes=closes, policy=policy
    )
