"""W 权重敏感度分析——L2 验证（真实数据，单时段多标的）。

目标：评估 FOLD_WEIGHT {AU:4, OIL:3, BOND:2, RE:1, EQUITY:1} 的选择
是否对商空间排序结果具有敏感性。如果排序对 W 值高度敏感，则当前
权重的选择需要更强的理论/经验支撑；如果不敏感，则权重选择是稳健的。

方法：
1. 从 L2 验证的缓存数据读取真实标的属性（24个A股，5年日线）
2. 构造多组权重方案（均匀、极端、局部扰动、反转等）
3. 对每组权重计算商空间排序
4. 用 Kendall tau 距离衡量排序稳定性
5. 分析哪些排序变化是实质性的（top-3 变化）

认识论等级：L2（真实数据，但单时段单市场）。
不是 L3——L3 需要多标的/多时段/多市场交叉验证。
诚实标注：使用的是 L2 验证的同一批数据（24个A股 2020-2026），
未引入新数据源。严格来说是对 L2 数据的敏感度分析，不是独立的 L3。

谱系引用：366号 L2 验证、352号多标的扫描器设计、292号折叠拓扑本体论。
"""

from __future__ import annotations

import itertools
import json
import sys
from dataclasses import dataclass
from pathlib import Path

# 确保项目 src 在 path 上
_project_root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_project_root / "src"))

from newchan.trading.fold_equivalence import (
    DTriState,
    EquivalenceClass,
    FoldChannel,
    FOLD_WEIGHT,
    TargetAttributes,
    build_quotient_space,
    equivalence_key,
)

# ═══════════════════════════════════════════════════════════════
# 数据：从 L2 验证报告中提取标的属性
# ═══════════════════════════════════════════════════════════════

DATA_DIR = _project_root / "data" / "scanner_l2"
L2_REPORT_PATH = DATA_DIR / "l2_validation_report.json"

# 从 L2 报告中提取的标的属性（精确复原）
# 每个标的的 tightness 和 liquidity 从 L2 验证过程中获取
# 这里使用报告中的数据重建 TargetAttributes

# 标的元数据（与 scanner_l2_validation.py 一致）
SYMBOL_META: dict[str, dict] = {
    # AU 通道
    "600547": {"name": "山东黄金", "channel": FoldChannel.AU, "sector": ""},
    "002155": {"name": "湖南黄金", "channel": FoldChannel.AU, "sector": ""},
    "600489": {"name": "中金黄金", "channel": FoldChannel.AU, "sector": ""},
    # OIL 通道
    "600028": {"name": "中国石化", "channel": FoldChannel.OIL, "sector": ""},
    "601857": {"name": "中国石油", "channel": FoldChannel.OIL, "sector": ""},
    "600688": {"name": "上海石化", "channel": FoldChannel.OIL, "sector": ""},
    # BOND 通道
    "601398": {"name": "工商银行", "channel": FoldChannel.BOND, "sector": ""},
    "601288": {"name": "农业银行", "channel": FoldChannel.BOND, "sector": ""},
    # RE 通道
    "600048": {"name": "保利发展", "channel": FoldChannel.RE, "sector": ""},
    "000002": {"name": "万科A", "channel": FoldChannel.RE, "sector": ""},
    "001979": {"name": "招商蛇口", "channel": FoldChannel.RE, "sector": ""},
    # EQUITY 通道 — 多板块
    "002230": {"name": "科大讯飞", "channel": FoldChannel.EQUITY, "sector": "tech"},
    "300059": {"name": "东方财富", "channel": FoldChannel.EQUITY, "sector": "tech"},
    "000725": {"name": "京东方A", "channel": FoldChannel.EQUITY, "sector": "tech"},
    "601318": {"name": "中国平安", "channel": FoldChannel.EQUITY, "sector": "finance"},
    "600036": {"name": "招商银行", "channel": FoldChannel.EQUITY, "sector": "finance"},
    "601166": {"name": "兴业银行", "channel": FoldChannel.EQUITY, "sector": "finance"},
    "600519": {"name": "贵州茅台", "channel": FoldChannel.EQUITY, "sector": "consumer"},
    "000858": {"name": "五粮液", "channel": FoldChannel.EQUITY, "sector": "consumer"},
    "002304": {"name": "洋河股份", "channel": FoldChannel.EQUITY, "sector": "consumer"},
    "300760": {"name": "迈瑞医疗", "channel": FoldChannel.EQUITY, "sector": "pharma"},
    "600276": {"name": "恒瑞医药", "channel": FoldChannel.EQUITY, "sector": "pharma"},
    "601985": {"name": "中国核电", "channel": FoldChannel.EQUITY, "sector": "energy"},
    "600900": {"name": "长江电力", "channel": FoldChannel.EQUITY, "sector": "energy"},
}


def _load_target_attributes_from_l2_report() -> tuple[TargetAttributes, ...]:
    """从 L2 验证报告重建 TargetAttributes。

    L2 报告中 flat_ranking 包含每个标的的 tightness。
    liquidity 信息不在报告中，使用序号作为 tie-breaker proxy
    （L2 验证中 liquidity 仅用于同 tightness 标的的排序打破）。
    """
    report = json.loads(L2_REPORT_PATH.read_text(encoding="utf-8"))
    flat = report["flat_ranking"]

    targets: list[TargetAttributes] = []
    for i, entry in enumerate(flat):
        symbol = entry["symbol"]
        meta = SYMBOL_META.get(symbol)
        if meta is None:
            continue
        targets.append(TargetAttributes(
            symbol=symbol,
            fold_channel=meta["channel"],
            sector=meta["sector"],
            d_tri_state=DTriState.RETAIN,  # L2 简化
            tightness=entry["tightness"],
            level_magnitude=1.0,  # L2 报告中未区分
            liquidity=float(len(flat) - i),  # 原始排序序号作为 proxy
        ))
    return tuple(targets)


# ═══════════════════════════════════════════════════════════════
# 权重方案
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class WeightScheme:
    """一组权重方案。"""
    name: str
    description: str
    weights: dict[FoldChannel, int]


# 基准方案
BASELINE = WeightScheme(
    name="baseline",
    description="当前权重 {AU:4, OIL:3, BOND:2, RE:1, EQUITY:1}",
    weights={
        FoldChannel.AU: 4,
        FoldChannel.OIL: 3,
        FoldChannel.BOND: 2,
        FoldChannel.RE: 1,
        FoldChannel.EQUITY: 1,
    },
)

# 实验方案集
WEIGHT_SCHEMES: list[WeightScheme] = [
    BASELINE,
    # --- 均匀权重 ---
    WeightScheme(
        name="uniform",
        description="均匀权重 {all:1}——退化为纯 T 排序（商空间仅合并等价类）",
        weights={ch: 1 for ch in FoldChannel},
    ),
    WeightScheme(
        name="uniform-3",
        description="均匀权重 {all:3}——缩放不影响排序（验证缩放不变性）",
        weights={ch: 3 for ch in FoldChannel},
    ),
    # --- 极端权重 ---
    WeightScheme(
        name="extreme-au",
        description="AU 极端高权 {AU:10, others:1}",
        weights={
            FoldChannel.AU: 10,
            FoldChannel.OIL: 1,
            FoldChannel.BOND: 1,
            FoldChannel.RE: 1,
            FoldChannel.EQUITY: 1,
        },
    ),
    WeightScheme(
        name="extreme-equity",
        description="EQUITY 极端高权 {EQUITY:10, others:1}",
        weights={
            FoldChannel.AU: 1,
            FoldChannel.OIL: 1,
            FoldChannel.BOND: 1,
            FoldChannel.RE: 1,
            FoldChannel.EQUITY: 10,
        },
    ),
    # --- 反转权重 ---
    WeightScheme(
        name="inverted",
        description="内外层反转 {AU:1, OIL:1, BOND:2, RE:3, EQUITY:4}",
        weights={
            FoldChannel.AU: 1,
            FoldChannel.OIL: 1,
            FoldChannel.BOND: 2,
            FoldChannel.RE: 3,
            FoldChannel.EQUITY: 4,
        },
    ),
    # --- 局部扰动 ---
    WeightScheme(
        name="perturb-au-down",
        description="AU 降 1: {AU:3, OIL:3, BOND:2, RE:1, EQUITY:1}",
        weights={
            FoldChannel.AU: 3,
            FoldChannel.OIL: 3,
            FoldChannel.BOND: 2,
            FoldChannel.RE: 1,
            FoldChannel.EQUITY: 1,
        },
    ),
    WeightScheme(
        name="perturb-au-up",
        description="AU 升 1: {AU:5, OIL:3, BOND:2, RE:1, EQUITY:1}",
        weights={
            FoldChannel.AU: 5,
            FoldChannel.OIL: 3,
            FoldChannel.BOND: 2,
            FoldChannel.RE: 1,
            FoldChannel.EQUITY: 1,
        },
    ),
    WeightScheme(
        name="perturb-oil-down",
        description="OIL 降 1: {AU:4, OIL:2, BOND:2, RE:1, EQUITY:1}",
        weights={
            FoldChannel.AU: 4,
            FoldChannel.OIL: 2,
            FoldChannel.BOND: 2,
            FoldChannel.RE: 1,
            FoldChannel.EQUITY: 1,
        },
    ),
    WeightScheme(
        name="perturb-bond-up",
        description="BOND 升 1: {AU:4, OIL:3, BOND:3, RE:1, EQUITY:1}",
        weights={
            FoldChannel.AU: 4,
            FoldChannel.OIL: 3,
            FoldChannel.BOND: 3,
            FoldChannel.RE: 1,
            FoldChannel.EQUITY: 1,
        },
    ),
    # --- 理论驱动：292号折叠嵌套序 ---
    WeightScheme(
        name="log-shared",
        description="对数共享性: {AU:3, OIL:2, BOND:2, RE:1, EQUITY:1}（更平缓梯度）",
        weights={
            FoldChannel.AU: 3,
            FoldChannel.OIL: 2,
            FoldChannel.BOND: 2,
            FoldChannel.RE: 1,
            FoldChannel.EQUITY: 1,
        },
    ),
    WeightScheme(
        name="steep-gradient",
        description="陡峭梯度: {AU:8, OIL:4, BOND:2, RE:1, EQUITY:1}（指数增长）",
        weights={
            FoldChannel.AU: 8,
            FoldChannel.OIL: 4,
            FoldChannel.BOND: 2,
            FoldChannel.RE: 1,
            FoldChannel.EQUITY: 1,
        },
    ),
]


# ═══════════════════════════════════════════════════════════════
# 用自定义权重构造商空间排序
# ═══════════════════════════════════════════════════════════════


def build_quotient_space_with_weights(
    targets: tuple[TargetAttributes, ...],
    weights: dict[FoldChannel, int],
) -> list[tuple[float, str, FoldChannel, float]]:
    """用指定权重构造商空间排序，返回 [(rank, symbol, channel, tightness)]。

    复用 fold_equivalence.py 的等价类构造（等价关系不依赖 W），
    但用自定义权重计算 rank。
    """
    # 二值筛选
    filtered = tuple(t for t in targets if t.tightness > 0.0)
    if not filtered:
        return []

    # 分组（复用 equivalence_key）
    groups: dict[tuple, list[TargetAttributes]] = {}
    for t in filtered:
        key = equivalence_key(t)
        if key not in groups:
            groups[key] = []
        groups[key].append(t)

    # 构造等价类 + 用自定义权重计算 rank
    results: list[tuple[float, str, FoldChannel, float]] = []
    for key, group in groups.items():
        sorted_members = sorted(group, key=lambda x: (-x.tightness, -x.liquidity))
        representative = sorted_members[0]
        max_t = representative.tightness
        channel = key[0]
        w = weights.get(channel, 1)
        rank = max_t * w
        results.append((rank, representative.symbol, channel, max_t))

    # 排序：rank desc → tightness desc
    results.sort(key=lambda x: (-x[0], -x[3]))
    return results


# ═══════════════════════════════════════════════════════════════
# Kendall tau 距离
# ═══════════════════════════════════════════════════════════════


def kendall_tau_distance(order_a: list[str], order_b: list[str]) -> tuple[int, int, float]:
    """计算两个排列的 Kendall tau 距离。

    只考虑两个排列中都出现的元素。

    Returns
    -------
    tuple[int, int, float]
        (discordant_pairs, total_pairs, normalized_distance)
        normalized_distance in [0, 1]，0=完全一致，1=完全反转。
    """
    common = [s for s in order_a if s in set(order_b)]
    if len(common) < 2:
        return (0, 0, 0.0)

    rank_a = {s: i for i, s in enumerate(order_a) if s in set(common)}
    rank_b = {s: i for i, s in enumerate(order_b) if s in set(common)}

    n = len(common)
    total_pairs = n * (n - 1) // 2
    discordant = 0
    for i in range(n):
        for j in range(i + 1, n):
            si, sj = common[i], common[j]
            if (rank_a[si] - rank_a[sj]) * (rank_b[si] - rank_b[sj]) < 0:
                discordant += 1

    normalized = discordant / total_pairs if total_pairs > 0 else 0.0
    return (discordant, total_pairs, normalized)


def top_k_overlap(order_a: list[str], order_b: list[str], k: int) -> float:
    """Top-k 重叠率：两个排序的 top-k 集合的 Jaccard 相似度。

    Returns
    -------
    float
        Jaccard similarity in [0, 1]。
    """
    set_a = set(order_a[:k])
    set_b = set(order_b[:k])
    if not set_a and not set_b:
        return 1.0
    return len(set_a & set_b) / len(set_a | set_b)


# ═══════════════════════════════════════════════════════════════
# 主实验
# ═══════════════════════════════════════════════════════════════


@dataclass
class SchemeResult:
    """单个权重方案的结果。"""
    scheme: WeightScheme
    ranking: list[tuple[float, str, FoldChannel, float]]
    order: list[str]  # symbol 顺序


def run_experiment() -> dict:
    """执行 W 权重敏感度实验。"""
    # 加载数据
    targets = _load_target_attributes_from_l2_report()
    print(f"加载标的: {len(targets)} 个")

    # 数据质量检查
    tightness_values = sorted(set(t.tightness for t in targets))
    print(f"Tightness 值域: {tightness_values}")
    tightness_dist = {}
    for t in targets:
        tightness_dist[t.tightness] = tightness_dist.get(t.tightness, 0) + 1
    print(f"Tightness 分布: {tightness_dist}")

    # 运行所有权重方案
    results: list[SchemeResult] = []
    for scheme in WEIGHT_SCHEMES:
        ranking = build_quotient_space_with_weights(targets, scheme.weights)
        order = [r[1] for r in ranking]
        results.append(SchemeResult(scheme=scheme, ranking=ranking, order=order))

    baseline_result = results[0]
    baseline_order = baseline_result.order

    # 计算每个方案相对于 baseline 的距离
    print("\n" + "=" * 80)
    print("W 权重敏感度分析")
    print("=" * 80)
    print(f"\n基准方案: {BASELINE.description}")
    print(f"基准排序: {' → '.join(baseline_order)}")

    pairwise_results = []
    for sr in results:
        tau_disc, tau_total, tau_norm = kendall_tau_distance(baseline_order, sr.order)
        top3_overlap = top_k_overlap(baseline_order, sr.order, 3)
        top5_overlap = top_k_overlap(baseline_order, sr.order, 5)
        pairwise_results.append({
            "scheme": sr.scheme.name,
            "description": sr.scheme.description,
            "weights": {ch.name: w for ch, w in sr.scheme.weights.items()},
            "order": sr.order,
            "kendall_tau_discordant": tau_disc,
            "kendall_tau_total_pairs": tau_total,
            "kendall_tau_normalized": round(tau_norm, 4),
            "top3_jaccard": round(top3_overlap, 4),
            "top5_jaccard": round(top5_overlap, 4),
            "ranking_details": [
                {
                    "rank_score": round(r[0], 4),
                    "symbol": r[1],
                    "name": SYMBOL_META.get(r[1], {}).get("name", "?"),
                    "channel": r[2].name,
                    "tightness": round(r[3], 4),
                }
                for r in sr.ranking
            ],
        })

    # 打印详细结果
    print("\n" + "-" * 80)
    print(f"{'方案':<22} {'Kendall tau':>12} {'Top-3 Jaccard':>14} {'Top-5 Jaccard':>14} 排序")
    print("-" * 80)
    for pr in pairwise_results:
        print(f"{pr['scheme']:<22} {pr['kendall_tau_normalized']:>12.4f} "
              f"{pr['top3_jaccard']:>14.4f} {pr['top5_jaccard']:>14.4f} "
              f"{' → '.join(pr['order'][:5])}...")

    # 分析：哪些方案改变了 top-3
    print("\n" + "-" * 80)
    print("Top-3 变化分析：")
    print("-" * 80)
    baseline_top3 = set(baseline_order[:3])
    for pr in pairwise_results:
        order = pr["order"]
        top3 = set(order[:3])
        if top3 != baseline_top3:
            entered = top3 - baseline_top3
            exited = baseline_top3 - top3
            entered_names = [SYMBOL_META.get(s, {}).get("name", s) for s in entered]
            exited_names = [SYMBOL_META.get(s, {}).get("name", s) for s in exited]
            print(f"  {pr['scheme']}: 进入 {entered_names}, 退出 {exited_names}")
        else:
            print(f"  {pr['scheme']}: Top-3 不变")

    # 分析：排序稳定性分类
    print("\n" + "-" * 80)
    print("排序稳定性分类：")
    print("-" * 80)

    stable_schemes = []
    moderate_schemes = []
    unstable_schemes = []
    for pr in pairwise_results:
        if pr["scheme"] == "baseline":
            continue
        tau = pr["kendall_tau_normalized"]
        if tau == 0.0:
            stable_schemes.append(pr["scheme"])
        elif tau <= 0.2:
            moderate_schemes.append(pr["scheme"])
        else:
            unstable_schemes.append(pr["scheme"])

    print(f"  完全稳定 (tau=0): {stable_schemes}")
    print(f"  中等敏感 (0<tau<=0.2): {moderate_schemes}")
    print(f"  高度敏感 (tau>0.2): {unstable_schemes}")

    # 关键发现：tightness 量化效应
    print("\n" + "-" * 80)
    print("关键发现：Tightness 量化效应")
    print("-" * 80)
    print(f"  Tightness 值域: {tightness_values}")
    print(f"  Tightness 分布: {tightness_dist}")
    print(f"  {tightness_dist.get(0.8, 0)}/{len(targets)} 标的的 tightness=0.8")
    print("  → 当大部分标的 tightness 相同时，排序几乎完全由 W 决定")
    print("  → W 权重的选择在当前数据上具有决定性影响")
    print("  → 这不意味着 W 应该等于 1（均匀权重），而是意味着：")
    print("    1. 当前 tightness proxy 的分辨率太低，需要改进")
    print("    2. 或者，W 权重的理论依据（292号区间套顺序）比 tightness 数值更重要")

    # 交叉 Kendall tau 矩阵（所有方案两两对比）
    print("\n" + "-" * 80)
    print("全方案两两 Kendall tau 距离矩阵：")
    print("-" * 80)
    scheme_names = [sr.scheme.name for sr in results]
    tau_matrix = []
    for i, sr_i in enumerate(results):
        row = []
        for j, sr_j in enumerate(results):
            _, _, tau = kendall_tau_distance(sr_i.order, sr_j.order)
            row.append(round(tau, 4))
        tau_matrix.append(row)

    # 打印矩阵
    header = f"{'':>22}" + "".join(f"{n:>14}" for n in scheme_names)
    print(header)
    for i, name in enumerate(scheme_names):
        row_str = f"{name:>22}" + "".join(f"{tau_matrix[i][j]:>14.4f}" for j in range(len(scheme_names)))
        print(row_str)

    # 构造报告
    report = {
        "experiment": "W 权重敏感度分析",
        "epistemology_level": "L2（真实数据，单时段多标的——复用366号L2数据）",
        "data_source": "366号 L2 验证缓存（24个A股，2020-2026日线）",
        "tightness_distribution": tightness_dist,
        "num_targets": len(targets),
        "num_equivalence_classes": len(baseline_result.ranking),
        "num_weight_schemes": len(WEIGHT_SCHEMES),
        "pairwise_vs_baseline": pairwise_results,
        "kendall_tau_matrix": {
            "scheme_names": scheme_names,
            "matrix": tau_matrix,
        },
        "stability_classification": {
            "stable": stable_schemes,
            "moderate": moderate_schemes,
            "unstable": unstable_schemes,
        },
        "conclusion": None,  # 运行后填充
    }

    # 生成结论
    total_non_baseline = len(WEIGHT_SCHEMES) - 1
    unstable_count = len(unstable_schemes)
    stable_count = len(stable_schemes)

    if unstable_count > total_non_baseline * 0.5:
        conclusion = "否定性：W 权重对排序高度敏感，当前 {4,3,2,1,1} 的选择需要更强理论支撑"
    elif stable_count > total_non_baseline * 0.5:
        conclusion = "确认性：排序对 W 权重不敏感，当前 {4,3,2,1,1} 是稳健选择"
    else:
        conclusion = "混合：排序对 W 权重有中等敏感性，局部扰动稳定但极端变化导致排序翻转"

    # 检查 tightness 量化效应对结论的影响
    max_tightness_count = max(tightness_dist.values())
    tightness_quantization_ratio = max_tightness_count / len(targets)
    if tightness_quantization_ratio > 0.7:
        conclusion += (
            f"。但注意：{tightness_quantization_ratio:.0%} 的标的共享同一 tightness 值，"
            "排序敏感性可能被 tightness 的低分辨率放大——"
            "如果 tightness 有更好的分辨率，W 权重的敏感性可能降低"
        )

    report["conclusion"] = conclusion
    print(f"\n{'=' * 80}")
    print(f"结论: {conclusion}")
    print(f"{'=' * 80}")

    return report


def main() -> None:
    report = run_experiment()

    # 保存报告
    output_path = _project_root / "tmp" / "w_weight_sensitivity_report.json"
    output_path.write_text(
        json.dumps(report, ensure_ascii=False, indent=2, default=str),
        encoding="utf-8",
    )
    print(f"\n报告已保存: {output_path}")


if __name__ == "__main__":
    main()
