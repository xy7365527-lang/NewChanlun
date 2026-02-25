"""A 系统 — 拓扑不变量与转换函数（195号谱系）

缠论的核心拓扑问题（001号谱系）：同一价格数据的不同合法分解是否
形成等价类？如果是，什么是这个等价类上的不变量？

本模块回答第一个问题：对同一价格数据 X，比较不同 (mode, params) 下
管线输出的拓扑指纹，判定哪些结构量在参数变化下保持。

## 拓扑语义（195号 + Gemini×Codex 共识）

### T1 公理（底层）
Extended PH 和 Zigzag PH 在中枢的区间分解意义下等价（模块同构）。
代码层面，centers_to_barcode 直接从中枢提取 bars (birth=ZD, death=ZG)，
隐含了 zigzag 3-支撑条带和中枢的一一对应（T3）。
这是公理性质——不可在代码中验证，只能声明。

### 结构映射
- 分解空间 D(X) = {所有合法分解} 是集合
- 管线 P_m: 输入空间 → D(X) 是参数化的确定性映射（mode m 决定参数）
- 等价关系 A ~ B ⇔ ∃X, m₁, m₂: P_{m₁}(X) = A ∧ P_{m₂}(X) = B
- 不变量候选 I: D(X) → V 是指纹函数，若 A ~ B ⇒ I(A) = I(B) 则 I 是不变量

### 条形码与 β₁^τ（共识修正#2）
走势空间不是流形——它是有限 CW 复形。亏格（genus）在 CW 复形上无定义，
但贝蒂数 β₁ 有定义（= H₁ 的秩）。条形码是范畴层面的对象（持续模块的
区间分解），β₁^τ = #{bars with length > τ} 是对条形码的去范畴化
（decategorification）——从 K₀(PersMod) 取秩泛函，丢弃生死时间信息。

### TDA 桥接层
本模块是 A 系统和 TDA 库之间的桥接层。
接口隔离：TDA 计算封装在 _bottleneck_distance() / _wasserstein_distance() 中。
当前后端：persim（bottleneck 基于 Hera 库，数学上正确）。
将来换库（gudhi/scikit-tda）只改这两个桥接函数，不改 A 系统。

### 映射的边界
管线 P_m 是单向的确定性算子，不可逆。
不变量是候选（待 gauge_equivalence_report 验证），不是已证明的定理。
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
import pandas as pd


# ---------------------------------------------------------------------------
# TDA 桥接层——接口隔离，将来换库只改这里
# ---------------------------------------------------------------------------

def _bottleneck_distance(
    dgm_a: np.ndarray, dgm_b: np.ndarray,
) -> float:
    """计算两个持续图（persistence diagram）之间的 bottleneck 距离。

    使用 persim 库。输入为 shape (n, 2) 的 numpy 数组，每行 (birth, death)。
    空图用 shape (0, 2) 表示。
    """
    from persim import bottleneck

    if dgm_a.shape[0] == 0 and dgm_b.shape[0] == 0:
        return 0.0
    return float(bottleneck(dgm_a, dgm_b))


def _wasserstein_distance(
    dgm_a: np.ndarray, dgm_b: np.ndarray, order: int = 1,
) -> float:
    """计算两个持续图之间的 Wasserstein-p 距离。

    默认 p=1（用于 T8 背驰检测——Layer 2）。
    """
    from persim import wasserstein

    if dgm_a.shape[0] == 0 and dgm_b.shape[0] == 0:
        return 0.0
    return float(wasserstein(dgm_a, dgm_b, order=order))


def centers_to_barcode(centers) -> tuple[tuple[float, float], ...]:
    """从中枢列表提取条形码（T3：中枢 ↔ zigzag 条带一一对应）。

    每个中枢 [ZD, ZG] 对应一个 bar (birth=ZD, death=ZG)。
    这是 T3 的直接实现：三段区间交集 [ZD, ZG] = zigzag H₀ 的一个
    3-支撑条带。
    """
    return tuple((float(c.low), float(c.high)) for c in centers)


def barcode_to_diagram(barcode: tuple[tuple[float, float], ...]) -> np.ndarray:
    """将条形码元组转为 persim 需要的 numpy 数组格式。"""
    if not barcode:
        return np.empty((0, 2), dtype=np.float64)
    return np.array(barcode, dtype=np.float64)


def compute_beta1_tau(barcode: tuple[tuple[float, float], ...], tau: float) -> int:
    """计算 β₁^τ：τ-显著的第一贝蒂数（共识修正#2）。

    β₁^τ = #{bars in barcode | death - birth > τ}

    这是条形码的去范畴化（decategorification）——从 K₀(PersMod) 取秩泛函，
    丢弃生死时间信息，只保留"有多少个独立的拓扑特征存活超过阈值 τ"。

    注意：不使用 genus（亏格）这个术语，因为走势空间是有限 CW 复形，
    不是闭可定向曲面。β₁ 在 CW 复形上有定义（= H₁ 的秩），genus 没有。
    """
    return sum(1 for b, d in barcode if d - b > tau)


@dataclass(frozen=True, slots=True)
class DecompositionFingerprint:
    """一次分解的拓扑指纹。

    不变量候选按强度分层（待 gauge_equivalence_report 验证）：
    - 强不变量候选：n_centers, trend_kinds, center_zd_zg_pairs, barcode, beta1_tau
    - 弱不变量（预期在不同模式下变化）：n_strokes, n_segments, max_level

    barcode 字段（Layer 1 共识）：
    每个中枢 [ZD, ZG] 对应一个 bar (ZD, ZG)——T3（中枢↔zigzag条带一一对应）。
    条形码是范畴层面的不变量候选，携带每个中枢的完整生死信息。

    beta1_tau 字段（共识修正#2）：
    β₁^τ = #{bars | death - birth > τ}。这是条形码的去范畴化结果。
    τ = 0 时 beta1_tau = n_centers；τ > 0 时只计长寿命中枢。
    """
    # 强不变量候选
    n_centers: int
    center_zd_zg_pairs: tuple[tuple[float, float], ...]
    trend_kinds: tuple[str, ...]
    barcode: tuple[tuple[float, float], ...]  # T3: 中枢↔条带
    beta1_tau: int  # 共识修正#2: β₁^τ（去范畴化）
    # 弱不变量
    n_strokes: int
    n_segments: int
    n_trends: int
    max_level: int


@dataclass(frozen=True, slots=True)
class StructuralDelta:
    """两次分解之间的结构差异——有结构的对象，不是标量。

    编排者修正：float structural_distance 丢失结构信息。
    StructuralDelta 是可分析的拓扑对象。

    bottleneck_distance 字段（Layer 1 共识）：
    两个条形码之间的 bottleneck 距离——gauge 不变量的核心度量。
    由瓶颈稳定性定理保证：若 δ_k 变动导致 ‖f-g‖_∞ ≤ ε，
    则 d_B(Dgm(f), Dgm(g)) ≤ ε。长度 > 2ε 的条带不受影响。
    """
    stroke_count_diff: int
    segment_count_diff: int
    center_count_diff: int
    center_interval_diffs: tuple[tuple[float, float], ...]  # 每个中枢的 (ΔZD, ΔZG)
    trend_kind_mutations: tuple[tuple[str, str], ...]  # (source_kind, target_kind) 对
    level_diff: int
    bottleneck_distance: float  # 条形码 bottleneck 距离（Layer 1 共识）
    beta1_tau_diff: int  # β₁^τ 差异


@dataclass(frozen=True, slots=True)
class TransitionResult:
    """转换函数的结果。"""
    source_mode: str
    target_mode: str
    source_fp: DecompositionFingerprint
    target_fp: DecompositionFingerprint
    delta: StructuralDelta
    strong_invariants_preserved: dict[str, bool]
    weak_invariants_preserved: dict[str, bool]


def compute_fingerprint(
    strokes, segments, centers, trends, rec_levels,
    *, tau: float = 0.0,
) -> DecompositionFingerprint:
    """计算一次分解的拓扑指纹。

    Parameters
    ----------
    tau : float
        β₁^τ 的阈值（T4：δ-Morse = gauge choice）。
        tau 就是 δ_k——Morse 函数的阈值参数。不同 tau 给出不同的
        β₁^τ，这正是 gauge choice 的体现：阈值选择是规范自由度，
        不影响长寿命拓扑特征的分类。默认 0.0 = 所有中枢都计入。
    """
    center_pairs = tuple(
        (float(c.low), float(c.high)) for c in centers
    )
    barcode = centers_to_barcode(centers)
    beta1 = compute_beta1_tau(barcode, tau)
    trend_kind_list = tuple(t.kind for t in trends)
    max_level = len(rec_levels) if rec_levels else 1
    return DecompositionFingerprint(
        n_centers=len(centers),
        center_zd_zg_pairs=center_pairs,
        trend_kinds=trend_kind_list,
        barcode=barcode,
        beta1_tau=beta1,
        n_strokes=len(strokes),
        n_segments=len(segments),
        n_trends=len(trends),
        max_level=max_level,
    )


def compute_structural_delta(
    fp_a: DecompositionFingerprint,
    fp_b: DecompositionFingerprint,
) -> StructuralDelta:
    """计算两个指纹之间的结构差异。"""
    # 中枢区间差异：对齐两组中枢（取较短的长度）
    min_centers = min(len(fp_a.center_zd_zg_pairs), len(fp_b.center_zd_zg_pairs))
    center_diffs = tuple(
        (
            fp_b.center_zd_zg_pairs[i][0] - fp_a.center_zd_zg_pairs[i][0],
            fp_b.center_zd_zg_pairs[i][1] - fp_a.center_zd_zg_pairs[i][1],
        )
        for i in range(min_centers)
    )

    # 走势类型突变：对齐两组走势
    min_trends = min(len(fp_a.trend_kinds), len(fp_b.trend_kinds))
    mutations = tuple(
        (fp_a.trend_kinds[i], fp_b.trend_kinds[i])
        for i in range(min_trends)
        if fp_a.trend_kinds[i] != fp_b.trend_kinds[i]
    )

    # 条形码 bottleneck 距离（Layer 1 共识）
    dgm_a = barcode_to_diagram(fp_a.barcode)
    dgm_b = barcode_to_diagram(fp_b.barcode)
    bn_dist = _bottleneck_distance(dgm_a, dgm_b)

    return StructuralDelta(
        stroke_count_diff=fp_b.n_strokes - fp_a.n_strokes,
        segment_count_diff=fp_b.n_segments - fp_a.n_segments,
        center_count_diff=fp_b.n_centers - fp_a.n_centers,
        center_interval_diffs=center_diffs,
        trend_kind_mutations=mutations,
        level_diff=fp_b.max_level - fp_a.max_level,
        bottleneck_distance=bn_dist,
        beta1_tau_diff=fp_b.beta1_tau - fp_a.beta1_tau,
    )


# ---------------------------------------------------------------------------
# T8：背驰 = Wasserstein-1 带容差单调下降（Layer 2）
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class T8Result:
    """T8 背驰拓扑后验验证结果。

    T8 断言：背驰 ⇔ W₁(Dgm(C)) ≤ W₁(Dgm(A)) - η
    其中 Dgm(X) 是 X 段内中枢构成的条形码。

    这是对现有 MACD 三维度 OR 判定的拓扑后验验证，
    不替代原始背驰检测。
    """
    divergence_index: int
    kind: str
    direction: str
    w1_a: float
    w1_c: float
    w1_drop: float  # w1_a - w1_c
    eta: float
    passed: bool  # w1_c <= w1_a - eta
    inconclusive: bool  # A/C 段无中枢时 True
    n_centers_a: int
    n_centers_c: int
    normalized: bool


def _normalize_barcode(
    barcode: tuple[tuple[float, float], ...],
    price_low: float,
    price_high: float,
) -> tuple[tuple[float, float], ...]:
    """仿射规范化条形码到 [0, 1] 区间。

    将每个 bar (birth, death) 线性映射：
    normalized = (value - price_low) / span，其中 span = price_high - price_low。

    如果 span ≤ 0（退化情况），返回原始条形码。
    """
    span = price_high - price_low
    if span <= 0:
        return barcode
    return tuple(
        ((b - price_low) / span, (d - price_low) / span)
        for b, d in barcode
    )


def _w1_norm(barcode: tuple[tuple[float, float], ...]) -> float:
    """计算条形码到空图的 Wasserstein-1 距离（W₁ 范数）。

    W₁(Dgm, ∅) = Σ|d_i - b_i| / 2
    即每个 bar 的半寿命之和——条形码的"总持续量"。

    空条形码返回 0.0。
    """
    if not barcode:
        return 0.0
    return sum(abs(d - b) / 2.0 for b, d in barcode)


def _centers_in_segment_range(
    centers,
    segments,
    seg_start: int,
    seg_end: int,
) -> list:
    """筛选 seg_start..seg_end 范围内的中枢子集。

    中枢的 seg0/seg1 完全落在 [seg_start, seg_end] 范围内才选入。
    centers 和 segments 分别是中枢列表和线段列表。
    """
    return [
        c for c in centers
        if c.seg0 >= seg_start and c.seg1 <= seg_end
    ]


def _segment_price_range(
    segments,
    seg_start: int,
    seg_end: int,
) -> tuple[float, float]:
    """提取 segment 范围内的 (min_low, max_high)。

    用于仿射规范化。遍历 segments[seg_start..seg_end] 的 low/high。
    """
    lows = [segments[i].low for i in range(seg_start, min(seg_end + 1, len(segments)))]
    highs = [segments[i].high for i in range(seg_start, min(seg_end + 1, len(segments)))]
    if not lows:
        return (0.0, 0.0)
    return (min(lows), max(highs))


def check_divergence_topology(
    divergences,
    segments,
    centers,
    *,
    eta: float = 0.0,
    normalize: bool = True,
) -> list[T8Result]:
    """对背驰列表逐个验证 T8 拓扑后验。

    Parameters
    ----------
    divergences : list[Divergence]
        来自 a_divergence.py 的背驰检测结果。
    segments : list[Segment]
        线段列表（用于提取价格范围和中枢筛选）。
    centers : list[Center]
        中枢列表。
    eta : float
        容差参数。背驰判定：w1_c <= w1_a - eta。
    normalize : bool
        是否做仿射规范化（默认 True）。

    Returns
    -------
    list[T8Result]
        每个背驰对应一个 T8 验证结果。
    """
    results: list[T8Result] = []

    for idx, div in enumerate(divergences):
        # 筛选 A 段和 C 段内的中枢
        centers_a = _centers_in_segment_range(
            centers, segments, div.seg_a_start, div.seg_a_end,
        )
        centers_c = _centers_in_segment_range(
            centers, segments, div.seg_c_start, div.seg_c_end,
        )

        # 任一侧无中枢 → inconclusive（域语义：无中枢 = 数据不足，非力竭）
        # 力竭 = 有中枢但中枢变窄（W₁ 下降），而非完全没有中枢。
        # Gemini×Codex Round 1 共识：选择理解B（or），拒绝理解A（and）。
        if not centers_a or not centers_c:
            results.append(T8Result(
                divergence_index=idx,
                kind=div.kind,
                direction=div.direction,
                w1_a=0.0,
                w1_c=0.0,
                w1_drop=0.0,
                eta=eta,
                passed=False,
                inconclusive=True,
                n_centers_a=len(centers_a),
                n_centers_c=len(centers_c),
                normalized=normalize,
            ))
            continue

        # 构造条形码
        bc_a = centers_to_barcode(centers_a)
        bc_c = centers_to_barcode(centers_c)

        # 仿射规范化
        if normalize and segments:
            price_low_a, price_high_a = _segment_price_range(
                segments, div.seg_a_start, div.seg_a_end,
            )
            price_low_c, price_high_c = _segment_price_range(
                segments, div.seg_c_start, div.seg_c_end,
            )
            bc_a = _normalize_barcode(bc_a, price_low_a, price_high_a)
            bc_c = _normalize_barcode(bc_c, price_low_c, price_high_c)

        # 计算 W₁ 范数
        w1_a = _w1_norm(bc_a)
        w1_c = _w1_norm(bc_c)
        w1_drop = w1_a - w1_c
        passed = w1_c <= w1_a - eta

        results.append(T8Result(
            divergence_index=idx,
            kind=div.kind,
            direction=div.direction,
            w1_a=w1_a,
            w1_c=w1_c,
            w1_drop=w1_drop,
            eta=eta,
            passed=passed,
            inconclusive=False,
            n_centers_a=len(centers_a),
            n_centers_c=len(centers_c),
            normalized=normalize,
        ))

    return results


def _check_strong_invariants(
    fp_a: DecompositionFingerprint,
    fp_b: DecompositionFingerprint,
    tolerance: float = 0.0,
) -> dict[str, bool]:
    """检查强不变量候选是否保持。

    tolerance: ZD/ZG 比较的容差（0.0 = 精确比较）。
    """
    # n_centers
    centers_preserved = fp_a.n_centers == fp_b.n_centers

    # center_zd_zg_pairs（带容差）
    intervals_preserved = False
    if centers_preserved:
        intervals_preserved = all(
            abs(a[0] - b[0]) <= tolerance and abs(a[1] - b[1]) <= tolerance
            for a, b in zip(fp_a.center_zd_zg_pairs, fp_b.center_zd_zg_pairs)
        )

    # trend_kinds
    trends_preserved = fp_a.trend_kinds == fp_b.trend_kinds

    # beta1_tau（去范畴化后的不变量）
    beta1_preserved = fp_a.beta1_tau == fp_b.beta1_tau

    # barcode bottleneck 距离（带容差）
    dgm_a = barcode_to_diagram(fp_a.barcode)
    dgm_b = barcode_to_diagram(fp_b.barcode)
    bn_dist = _bottleneck_distance(dgm_a, dgm_b)
    barcode_preserved = bn_dist <= tolerance

    return {
        "n_centers": centers_preserved,
        "center_zd_zg_pairs": intervals_preserved,
        "trend_kinds": trends_preserved,
        "beta1_tau": beta1_preserved,
        "barcode_bottleneck": barcode_preserved,
    }


def _check_weak_invariants(
    fp_a: DecompositionFingerprint,
    fp_b: DecompositionFingerprint,
) -> dict[str, bool]:
    """检查弱不变量是否保持（预期变化——记录实际情况）。"""
    return {
        "n_strokes": fp_a.n_strokes == fp_b.n_strokes,
        "n_segments": fp_a.n_segments == fp_b.n_segments,
        "n_trends": fp_a.n_trends == fp_b.n_trends,
        "max_level": fp_a.max_level == fp_b.max_level,
    }


def _run_pipeline(df_raw, mode, min_strict_sep=5, center_sustain_m=2):
    """运行 A 系统管线，返回 (strokes, segments, centers, trends, rec_levels)。"""
    from newchan.a_inclusion import merge_inclusion
    from newchan.a_fractal import fractals_from_merged
    from newchan.a_stroke import strokes_from_fractals
    from newchan.a_segment_v1 import segments_from_strokes_v1
    from newchan.a_center_v0 import centers_from_segments_v0
    from newchan.a_trendtype_v0 import trend_instances_from_centers
    from newchan.a_recursive_engine import build_recursive_levels

    df_merged, merged_to_raw = merge_inclusion(df_raw)
    fractals = fractals_from_merged(df_merged)
    strokes = strokes_from_fractals(
        df_merged, fractals, mode=mode, min_strict_sep=min_strict_sep,
        merged_to_raw=merged_to_raw if mode == "new" else None,
    )
    segments = segments_from_strokes_v1(strokes)
    rec_levels = build_recursive_levels(segments, sustain_m=center_sustain_m)
    if rec_levels:
        centers = rec_levels[0].centers
        trends = rec_levels[0].trends
    else:
        centers = centers_from_segments_v0(segments, sustain_m=center_sustain_m)
        trends = trend_instances_from_centers(segments, centers)
    return strokes, segments, centers, trends, rec_levels


def compute_transition(
    df_raw: pd.DataFrame,
    mode_a: str,
    mode_b: str,
    *,
    min_strict_sep: int = 5,
    center_sustain_m: int = 2,
    tolerance: float = 0.0,
    tau: float = 0.0,
) -> TransitionResult:
    """计算两种分解参数之间的转换函数结果。

    运行管线两次（mode_a, mode_b），比较拓扑指纹，
    分层报告强/弱不变量的保持状态，输出结构差异对象。

    Parameters
    ----------
    tau : float
        β₁^τ 的阈值。默认 0.0。
    """
    s_a, seg_a, c_a, t_a, rl_a = _run_pipeline(
        df_raw, mode_a, min_strict_sep, center_sustain_m,
    )
    s_b, seg_b, c_b, t_b, rl_b = _run_pipeline(
        df_raw, mode_b, min_strict_sep, center_sustain_m,
    )

    fp_a = compute_fingerprint(s_a, seg_a, c_a, t_a, rl_a, tau=tau)
    fp_b = compute_fingerprint(s_b, seg_b, c_b, t_b, rl_b, tau=tau)
    delta = compute_structural_delta(fp_a, fp_b)
    strong = _check_strong_invariants(fp_a, fp_b, tolerance)
    weak = _check_weak_invariants(fp_a, fp_b)

    return TransitionResult(
        source_mode=mode_a,
        target_mode=mode_b,
        source_fp=fp_a,
        target_fp=fp_b,
        delta=delta,
        strong_invariants_preserved=strong,
        weak_invariants_preserved=weak,
    )


def gauge_equivalence_report(
    df_raw: pd.DataFrame,
    modes: tuple[str, ...] = ("wide", "strict", "new"),
    *,
    min_strict_sep: int = 5,
    center_sustain_m: int = 2,
    tolerance: float = 0.0,
    tau: float = 0.0,
) -> dict:
    """对所有 mode 组合计算转换函数，输出等价类报告。

    分层报告：
    - 强不变量候选的保持率（含 barcode bottleneck 和 β₁^τ）
    - 弱不变量的变化幅度
    - 每对模式之间的 StructuralDelta（含 bottleneck 距离）

    这是 001号谱系"分解不唯一 = gauge choice"的可计算验证。

    Parameters
    ----------
    tau : float
        β₁^τ 的阈值。默认 0.0。
    """
    transitions = []
    for i, ma in enumerate(modes):
        for mb in modes[i + 1:]:
            tr = compute_transition(
                df_raw, ma, mb,
                min_strict_sep=min_strict_sep,
                center_sustain_m=center_sustain_m,
                tolerance=tolerance,
                tau=tau,
            )
            transitions.append(tr)

    # 汇总强不变量保持率
    strong_keys = [
        "n_centers", "center_zd_zg_pairs", "trend_kinds",
        "beta1_tau", "barcode_bottleneck",
    ]
    strong_summary = {}
    for k in strong_keys:
        preserved = sum(1 for t in transitions if t.strong_invariants_preserved.get(k, False))
        strong_summary[k] = {
            "preserved": preserved,
            "total": len(transitions),
            "rate": preserved / len(transitions) if transitions else 0.0,
        }

    # T7 递归条形码偏序检查（取第一个模式的管线结果）
    t7_results = []
    try:
        _, _, _, _, rl_first = _run_pipeline(
            df_raw, modes[0], min_strict_sep, center_sustain_m,
        )
        if len(rl_first) >= 2:
            t7_checks = check_recursive_barcode_order(rl_first, tau=tau)
            t7_results = [
                {
                    "level_low": r.level_low,
                    "level_high": r.level_high,
                    "passed": r.passed,
                    "trimmed_low_count": r.trimmed_low_count,
                    "high_count": r.high_count,
                }
                for r in t7_checks
            ]
    except Exception:
        pass

    # T8 背驰拓扑后验检查（取第一个模式的管线结果）
    t8_results = []
    try:
        from newchan.a_divergence import divergences_from_level
        s_first, seg_first, c_first, t_first, rl_first = _run_pipeline(
            df_raw, modes[0], min_strict_sep, center_sustain_m,
        )
        if rl_first:
            level0 = rl_first[0]
            # level0.moves 在 level=1 时是 Segment 列表，索引空间与 Center.seg0/seg1 一致
            t8_moves = level0.moves if hasattr(level0, "moves") else seg_first
            t8_centers = level0.centers
            t8_trends = level0.trends
            t8_level = level0.level
        else:
            t8_moves = seg_first
            t8_centers = c_first
            t8_trends = t_first
            t8_level = 0
        divs = divergences_from_level(
            t8_moves, t8_centers, t8_trends, t8_level,
        )
        if divs:
            # segments 参数必须与 divergences 的索引空间一致（= t8_moves）
            t8_checks = check_divergence_topology(
                divs, t8_moves, t8_centers, eta=0.0,
            )
            t8_results = [
                {
                    "divergence_index": r.divergence_index,
                    "kind": r.kind,
                    "direction": r.direction,
                    "w1_a": r.w1_a,
                    "w1_c": r.w1_c,
                    "w1_drop": r.w1_drop,
                    "passed": r.passed,
                    "inconclusive": r.inconclusive,
                }
                for r in t8_checks
            ]
    except Exception:
        pass

    return {
        "modes": list(modes),
        "n_transitions": len(transitions),
        "tau": tau,
        "transitions": [
            {
                "source": t.source_mode,
                "target": t.target_mode,
                "delta": {
                    "stroke_count_diff": t.delta.stroke_count_diff,
                    "segment_count_diff": t.delta.segment_count_diff,
                    "center_count_diff": t.delta.center_count_diff,
                    "level_diff": t.delta.level_diff,
                    "trend_mutations": list(t.delta.trend_kind_mutations),
                    "bottleneck_distance": t.delta.bottleneck_distance,
                    "beta1_tau_diff": t.delta.beta1_tau_diff,
                },
                "strong_preserved": t.strong_invariants_preserved,
                "weak_preserved": t.weak_invariants_preserved,
            }
            for t in transitions
        ],
        "strong_invariant_summary": strong_summary,
        "t7_recursive_order": t7_results,
        "t8_divergence_topology": t8_results,
    }


# ---------------------------------------------------------------------------
# T7：递归条形码偏序（B_{k+1} ⊆ Trim(B_k, τ)）
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class T7Result:
    """T7 递归条形码偏序检查结果。

    T7 断言：高级别的条形码是低级别条形码经过阈值修剪后的子集。
    B_{k+1} ⊆ Trim(B_k, τ_{k+1})
    """
    level_low: int
    level_high: int
    passed: bool
    trimmed_low_count: int
    high_count: int
    unmatched_bars: tuple[tuple[float, float], ...]


def _check_barcode_inclusion(
    bars_high: tuple[tuple[float, float], ...],
    bars_trimmed_low: tuple[tuple[float, float], ...],
    epsilon: float,
) -> tuple[bool, tuple[tuple[float, float], ...]]:
    """检查 bars_high 是否 ε-嵌入 bars_trimmed_low（真单射）。

    对 bars_high 中的每个 bar (b,d)，要求存在 bars_trimmed_low 中的
    某个 **尚未被匹配** 的 bar (b',d') 使得 |b-b'| ≤ ε 且 |d-d'| ≤ ε。
    真单射：bars_trimmed_low 中每个 bar 至多被匹配一次。

    使用最大二分匹配（增广路径法）消除贪心假阴性。
    复杂度 O(n²·m)，对 n_centers < 20 的场景足够。

    边界情况：
    - bars_high 为空 → True（空集总被包含）
    - bars_trimmed_low 为空但 bars_high 非空 → False

    Returns
    -------
    (passed, unmatched_bars) : tuple[bool, tuple[tuple[float, float], ...]]
    """
    if not bars_high:
        return (True, ())
    if not bars_trimmed_low:
        return (False, bars_high)

    n = len(bars_high)
    m = len(bars_trimmed_low)

    # 构建相容矩阵：compat[i][j] = bars_high[i] 与 bars_trimmed_low[j] 在 ε 内
    compat: list[list[bool]] = [
        [
            abs(bars_high[i][0] - bars_trimmed_low[j][0]) <= epsilon
            and abs(bars_high[i][1] - bars_trimmed_low[j][1]) <= epsilon
            for j in range(m)
        ]
        for i in range(n)
    ]

    # 最大二分匹配——增广路径法（Hungarian-style augmenting paths）
    # match_low[j] = 匹配到 bars_trimmed_low[j] 的 bars_high 索引，-1 表示未匹配
    match_low: list[int] = [-1] * m

    def _augment(i: int, visited: list[bool]) -> bool:
        for j in range(m):
            if compat[i][j] and not visited[j]:
                visited[j] = True
                if match_low[j] == -1 or _augment(match_low[j], visited):
                    match_low[j] = i
                    return True
        return False

    matched_count = 0
    for i in range(n):
        if _augment(i, [False] * m):
            matched_count += 1

    if matched_count == n:
        return (True, ())

    # 找出未匹配的 bars_high
    matched_high: set[int] = set(match_low[j] for j in range(m) if match_low[j] != -1)
    unmatched = tuple(bars_high[i] for i in range(n) if i not in matched_high)
    return (False, unmatched)


def check_recursive_barcode_order(
    levels,
    tau: float = 0.0,
    epsilon: float = 1.0,
) -> list[T7Result]:
    """检查递归层级间的条形码偏序（T7）。

    对每对相邻层级 (k, k+1)：
    - B_k = 第 k 层中枢的条形码
    - Trim(B_k, τ) = {bar in B_k | death - birth > τ}
    - 验证 B_{k+1} ε-嵌入 Trim(B_k)（真单射匹配）

    Parameters
    ----------
    levels : list[RecursiveLevel]
        从 build_recursive_levels 返回的层级列表。
    tau : float
        Trim 阈值——只保留长度 > τ 的条带。
    epsilon : float
        ε-匹配容差——birth/death 偏差在此范围内视为匹配。
    """
    results: list[T7Result] = []
    for i in range(len(levels) - 1):
        low_level = levels[i]
        high_level = levels[i + 1]
        bc_low = centers_to_barcode(low_level.centers)
        bc_high = centers_to_barcode(high_level.centers)
        trimmed_low = tuple(bar for bar in bc_low if bar[1] - bar[0] > tau)
        passed, unmatched = _check_barcode_inclusion(bc_high, trimmed_low, epsilon)
        results.append(T7Result(
            level_low=low_level.level,
            level_high=high_level.level,
            passed=passed,
            trimmed_low_count=len(trimmed_low),
            high_count=len(bc_high),
            unmatched_bars=unmatched,
        ))
    return results


# ---------------------------------------------------------------------------
# T5：走势类型 ≅ 上级笔（构造不变式验证）
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class T5Result:
    """T5 走势类型≡上级笔 构造不变式验证结果。

    T5（Trend_k ≅ E(X_{k+1})）在当前递归引擎中是构造保证：
    levels[k+1].moves 就是 levels[k].confirmed_trends 的同一引用。
    因此 T5 不是运行时约束，而是构造不变式验证——检查引擎是否
    正确传递了引用且没有混入 unconfirmed trends。
    """
    level_low: int
    level_high: int
    passed: bool
    confirmed_only: bool
    identity: bool
    n_moves: int
    n_confirmed_trends: int


def check_trend_move_equivalence(levels) -> list[T5Result]:
    """验证递归层级间的走势≡上级笔构造不变式（T5）。

    对每对相邻层级 (k, k+1) 检查：
    1. confirmed_only：levels[k+1].moves 中所有元素的 confirmed 为 True
    2. identity：levels[k+1].moves 与 levels[k].confirmed_trends 内容一致

    Parameters
    ----------
    levels : list[RecursiveLevel]
        从 build_recursive_levels 返回的层级列表。
    """
    results: list[T5Result] = []
    for i in range(len(levels) - 1):
        low_level = levels[i]
        high_level = levels[i + 1]
        confirmed_trends = [t for t in low_level.trends if t.confirmed]
        moves = high_level.moves
        confirmed_only = all(m.confirmed for m in moves)
        identity = (
            len(moves) == len(confirmed_trends)
            and all(m is ct for m, ct in zip(moves, confirmed_trends))
        )
        passed = confirmed_only and identity
        results.append(T5Result(
            level_low=low_level.level,
            level_high=high_level.level,
            passed=passed,
            confirmed_only=confirmed_only,
            identity=identity,
            n_moves=len(moves),
            n_confirmed_trends=len(confirmed_trends),
        ))
    return results
