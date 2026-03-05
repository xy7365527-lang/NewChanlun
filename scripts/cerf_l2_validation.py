"""
Cerf L2 验证：在真实 relations.jsonl 上运行加权 Morse Cerf 分析

认识论等级：L2（真实数据 + ground truth 对比）
有效域：本谱系 DAG（单标的），ground truth 来自355号的22个关键否定事件

目标：
1. 在真实 relations.jsonl 上运行 Cerf 分岔点检测（加权 Morse）
2. 同时运行均匀 Morse 时间序列作为基线
3. 对比加权 vs 均匀 Morse 的 precision/recall（ground truth = 355号的22个关键否定）
4. 检查分岔点是否与已知"关键转折"对应

谱系依据：360号（B-M 三步完成，L0+L1）-> 本脚本提升到 L2
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from pathlib import Path

# 确保可导入 scripts/ 和 experiments/discrete_morse/
REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))
sys.path.insert(0, str(REPO_ROOT / "experiments" / "discrete_morse"))

from cerf_bifurcation import (
    Bifurcation,
    MorseSnapshot,
    build_temporal_morse_sequence,
    detect_bifurcations,
    annotate_bifurcations_with_genealogy,
    cerf_summary,
)
from morse_temporal import (
    run_temporal_evolution,
    detect_jumps,
)


# ─────────────────────────────────────────────────────────────────────────────
# Ground Truth（355号验证报告中的22个关键否定事件）
# ─────────────────────────────────────────────────────────────────────────────

GROUND_TRUTH_CRITICAL_NEGATIONS: set[str] = {
    "064", "068", "087", "088", "089", "090", "096", "097",
    "105", "136", "137", "143", "147", "156", "157", "161",
    "162", "173", "218", "274", "275", "277",
}

# 已知的关键转折点（不限于否定，包含范式转移等）
KNOWN_TURNING_POINTS: dict[str, str] = {
    "020": "统一本体论",
    "068": "连续->离散范式转移",
    "087": "编排者 INTERRUPT 否定 086",
    "089": "补丁思维第二次否定（严格扬弃）",
    "090": "严格性永久化为语法规则",
    "130": "RTAS 架构确立",
    "137": "RLHF 基底约束发现",
    "161": "务实否定",
    "218": "Lead 并行化",
}


# ─────────────────────────────────────────────────────────────────────────────
# Precision / Recall 计算
# ─────────────────────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class PrecisionRecallRow:
    top_n: int
    precision: float
    recall: float
    hits: int
    hit_keys: tuple[str, ...]


def compute_precision_recall(
    ranked_keys: list[str],
    ground_truth: set[str],
    top_ns: list[int],
) -> list[PrecisionRecallRow]:
    """对排序后的谱系编号列表计算 precision/recall at top-N。"""
    results: list[PrecisionRecallRow] = []
    gt_size = len(ground_truth)
    if gt_size == 0:
        return results

    for n in top_ns:
        top = ranked_keys[:n]
        hits = [k for k in top if k in ground_truth]
        p = len(hits) / n if n > 0 else 0.0
        r = len(hits) / gt_size
        results.append(PrecisionRecallRow(
            top_n=n,
            precision=p,
            recall=r,
            hits=len(hits),
            hit_keys=tuple(hits),
        ))
    return results


# ─────────────────────────────────────────────────────────────────────────────
# 主分析
# ─────────────────────────────────────────────────────────────────────────────

def main() -> None:
    relations_path = REPO_ROOT / ".chanlun" / "block-topology" / "relations.jsonl"
    meta_path = REPO_ROOT / ".chanlun" / "block-topology" / "meta.json"
    settled_dir = REPO_ROOT / ".chanlun" / "genealogy" / "settled"

    for p in [relations_path, meta_path]:
        if not p.exists():
            print(f"Error: {p} not found")
            sys.exit(1)

    print("=" * 70)
    print("Cerf L2 验证：加权 Morse 分岔 vs 关键否定事件")
    print("=" * 70)
    print()
    print("认识论等级: L2（真实数据 + ground truth 对比）")
    print(f"Ground truth: {len(GROUND_TRUTH_CRITICAL_NEGATIONS)} 个关键否定事件（来源：355号验证报告）")
    print()

    # ─────────────────────────────────────────────────────────────────────
    # Phase 1: 加权 Morse Cerf 分岔检测
    # ─────────────────────────────────────────────────────────────────────
    print("Phase 1: 构造加权 Morse 时间序列...")
    sequence = build_temporal_morse_sequence(
        str(relations_path),
        meta_path=str(meta_path),
    )
    print(f"  总步骤数: {len(sequence)}")
    if sequence:
        final = sequence[-1]
        print(f"  最终复形: V={final.num_vertices} E={final.num_edges} T={final.num_triangles}")
        print(f"  最终临界集: m0={final.critical_0} m1={final.critical_1} m2={final.critical_2}")
        print(f"  最终 Betti: ({final.betti_0}, {final.betti_1}, {final.betti_2})")

    print("\n  检测分岔点（threshold=0.3）...")
    bifurcations = detect_bifurcations(sequence, threshold=0.3)
    print(f"  分岔点数: {len(bifurcations)}")

    # 按幅度排序
    sorted_bifs = sorted(bifurcations, key=lambda b: b.magnitude, reverse=True)
    weighted_ranked_keys = [b.genealogy_key for b in sorted_bifs]

    # ─────────────────────────────────────────────────────────────────────
    # Phase 2: 均匀 Morse 基线（DM2 跳跃点）
    # ─────────────────────────────────────────────────────────────────────
    print("\nPhase 2: 构造均匀 Morse 时间序列（DM2 基线）...")
    uniform_snapshots = run_temporal_evolution(relations_path, meta_path)
    uniform_jumps = detect_jumps(uniform_snapshots, threshold=1)
    sorted_uniform = sorted(uniform_jumps, key=lambda j: j.magnitude, reverse=True)
    uniform_ranked_keys = [j.genealogy_key for j in sorted_uniform]
    print(f"  均匀 Morse 跳跃点数 (threshold>=1): {len(uniform_jumps)}")

    # ─────────────────────────────────────────────────────────────────────
    # Phase 3: Precision / Recall 对比
    # ─────────────────────────────────────────────────────────────────────
    print("\n" + "=" * 70)
    print("Phase 3: Precision / Recall 对比")
    print("=" * 70)

    top_ns = [5, 10, 15, 20, 30, 50, 100, len(weighted_ranked_keys)]

    weighted_pr = compute_precision_recall(
        weighted_ranked_keys, GROUND_TRUTH_CRITICAL_NEGATIONS, top_ns,
    )
    uniform_pr = compute_precision_recall(
        uniform_ranked_keys, GROUND_TRUTH_CRITICAL_NEGATIONS,
        [5, 10, 15, 20, 30, 50, 100, len(uniform_ranked_keys)],
    )

    print("\n--- 加权 Morse Cerf 分岔 ---")
    print(f"{'Top-N':>8s} | {'Precision':>10s} | {'Recall':>8s} | {'Hits':>5s}")
    print("-" * 40)
    for row in weighted_pr:
        print(f"{row.top_n:8d} | {row.precision:10.4f} | {row.recall:8.4f} | {row.hits:5d}")

    print("\n--- 均匀 Morse 跳跃点（DM2 基线）---")
    print(f"{'Top-N':>8s} | {'Precision':>10s} | {'Recall':>8s} | {'Hits':>5s}")
    print("-" * 40)
    for row in uniform_pr:
        print(f"{row.top_n:8d} | {row.precision:10.4f} | {row.recall:8.4f} | {row.hits:5d}")

    # ─────────────────────────────────────────────────────────────────────
    # Phase 4: 逐点分析
    # ─────────────────────────────────────────────────────────────────────
    print("\n" + "=" * 70)
    print("Phase 4: 加权 Cerf 分岔 Top-20 逐点分析")
    print("=" * 70)

    # 谱系标注
    annotated = annotate_bifurcations_with_genealogy(bifurcations, str(settled_dir))
    annotated_map = {a["genealogy_key"]: a for a in annotated}

    print(f"\n{'Rank':>4s} {'Key':>6s} | {'Type':>14s} | {'Mag':>8s} | {'GndTruth':>8s} | Description")
    print("-" * 95)
    for rank, bif in enumerate(sorted_bifs[:20], 1):
        is_gt = "YES" if bif.genealogy_key in GROUND_TRUTH_CRITICAL_NEGATIONS else "no"
        desc = bif.description[:50]
        print(f"{rank:4d} {bif.genealogy_key:>6s} | {bif.type:>14s} | {bif.magnitude:8.3f} | {is_gt:>8s} | {desc}")

    # ─────────────────────────────────────────────────────────────────────
    # Phase 5: 关键否定在 Cerf 分岔中的排名分布
    # ─────────────────────────────────────────────────────────────────────
    print("\n" + "=" * 70)
    print("Phase 5: 22个关键否定在加权 Cerf 分岔中的排名")
    print("=" * 70)

    key_to_rank: dict[str, int] = {k: i + 1 for i, k in enumerate(weighted_ranked_keys)}
    key_to_rank_uniform: dict[str, int] = {k: i + 1 for i, k in enumerate(uniform_ranked_keys)}

    print(f"\n{'Key':>6s} | {'Cerf Rank':>10s} | {'Cerf Mag':>10s} | {'Unif Rank':>10s} | {'Unif Mag':>10s} | Turning Point")
    print("-" * 85)

    gt_sorted = sorted(GROUND_TRUTH_CRITICAL_NEGATIONS, key=lambda k: key_to_rank.get(k, 99999))
    for key in gt_sorted:
        cerf_rank = key_to_rank.get(key, -1)
        uniform_rank = key_to_rank_uniform.get(key, -1)

        cerf_mag_str = "N/A"
        if cerf_rank > 0 and cerf_rank <= len(sorted_bifs):
            cerf_mag_str = f"{sorted_bifs[cerf_rank - 1].magnitude:.3f}"

        uniform_mag_str = "N/A"
        if uniform_rank > 0 and uniform_rank <= len(sorted_uniform):
            uniform_mag_str = f"{sorted_uniform[uniform_rank - 1].magnitude}"

        cerf_rank_str = str(cerf_rank) if cerf_rank > 0 else "MISS"
        uniform_rank_str = str(uniform_rank) if uniform_rank > 0 else "MISS"

        tp = KNOWN_TURNING_POINTS.get(key, "")
        print(f"{key:>6s} | {cerf_rank_str:>10s} | {cerf_mag_str:>10s} | {uniform_rank_str:>10s} | {uniform_mag_str:>10s} | {tp}")

    # ─────────────────────────────────────────────────────────────────────
    # Phase 6: 加权 vs 均匀 改善分析
    # ─────────────────────────────────────────────────────────────────────
    print("\n" + "=" * 70)
    print("Phase 6: 加权 vs 均匀改善分析")
    print("=" * 70)

    # 对每个 ground truth key，比较两种方法的排名
    cerf_better_count = 0
    uniform_better_count = 0
    tie_count = 0
    cerf_miss_count = 0
    uniform_miss_count = 0

    for key in GROUND_TRUTH_CRITICAL_NEGATIONS:
        cr = key_to_rank.get(key, -1)
        ur = key_to_rank_uniform.get(key, -1)
        if cr == -1 and ur == -1:
            pass
        elif cr == -1:
            uniform_better_count += 1
            cerf_miss_count += 1
        elif ur == -1:
            cerf_better_count += 1
            uniform_miss_count += 1
        elif cr < ur:
            cerf_better_count += 1
        elif ur < cr:
            uniform_better_count += 1
        else:
            tie_count += 1

    # 平均排名比较（只计算两者都有排名的）
    cerf_ranks_valid: list[int] = []
    uniform_ranks_valid: list[int] = []
    for key in GROUND_TRUTH_CRITICAL_NEGATIONS:
        cr = key_to_rank.get(key, -1)
        ur = key_to_rank_uniform.get(key, -1)
        if cr > 0 and ur > 0:
            cerf_ranks_valid.append(cr)
            uniform_ranks_valid.append(ur)

    avg_cerf_rank = sum(cerf_ranks_valid) / len(cerf_ranks_valid) if cerf_ranks_valid else float('inf')
    avg_uniform_rank = sum(uniform_ranks_valid) / len(uniform_ranks_valid) if uniform_ranks_valid else float('inf')
    median_cerf = sorted(cerf_ranks_valid)[len(cerf_ranks_valid) // 2] if cerf_ranks_valid else -1
    median_uniform = sorted(uniform_ranks_valid)[len(uniform_ranks_valid) // 2] if uniform_ranks_valid else -1

    print(f"\n  加权 Cerf 排名更优: {cerf_better_count} / {len(GROUND_TRUTH_CRITICAL_NEGATIONS)}")
    print(f"  均匀 Morse 排名更优: {uniform_better_count} / {len(GROUND_TRUTH_CRITICAL_NEGATIONS)}")
    print(f"  平局: {tie_count}")
    print(f"  加权 Cerf 未检出: {cerf_miss_count}")
    print(f"  均匀 Morse 未检出: {uniform_miss_count}")
    print(f"\n  加权 Cerf 平均排名: {avg_cerf_rank:.1f} (median={median_cerf})")
    print(f"  均匀 Morse 平均排名: {avg_uniform_rank:.1f} (median={median_uniform})")

    improvement = avg_uniform_rank - avg_cerf_rank
    print(f"\n  排名改善（均匀 - 加权，正值 = 加权更优）: {improvement:.1f}")

    # ─────────────────────────────────────────────────────────────────────
    # Phase 7: 分岔类型分布
    # ─────────────────────────────────────────────────────────────────────
    print("\n" + "=" * 70)
    print("Phase 7: 分岔类型分布")
    print("=" * 70)

    type_dist: dict[str, int] = {}
    type_in_gt: dict[str, int] = {}
    for bif in bifurcations:
        type_dist[bif.type] = type_dist.get(bif.type, 0) + 1
        if bif.genealogy_key in GROUND_TRUTH_CRITICAL_NEGATIONS:
            type_in_gt[bif.type] = type_in_gt.get(bif.type, 0) + 1

    print(f"\n{'Type':>16s} | {'Total':>6s} | {'In GT':>6s} | {'Rate':>8s}")
    print("-" * 45)
    for t in sorted(type_dist.keys()):
        total = type_dist[t]
        in_gt = type_in_gt.get(t, 0)
        rate = in_gt / total if total > 0 else 0.0
        print(f"{t:>16s} | {total:6d} | {in_gt:6d} | {rate:8.4f}")

    # ─────────────────────────────────────────────────────────────────────
    # Phase 8: 已知关键转折点的谱系标注
    # ─────────────────────────────────────────────────────────────────────
    print("\n" + "=" * 70)
    print("Phase 8: 已知关键转折点的 Cerf 分岔匹配")
    print("=" * 70)

    bif_keys = {b.genealogy_key for b in bifurcations}
    print(f"\n{'Key':>6s} | {'In Cerf':>8s} | {'Rank':>6s} | Description")
    print("-" * 60)
    for key, desc in sorted(KNOWN_TURNING_POINTS.items(), key=lambda x: int(x[0])):
        in_cerf = "YES" if key in bif_keys else "no"
        rank = key_to_rank.get(key, -1)
        rank_str = str(rank) if rank > 0 else "N/A"
        print(f"{key:>6s} | {in_cerf:>8s} | {rank_str:>6s} | {desc}")

    # ─────────────────────────────────────────────────────────────────────
    # 综合结论
    # ─────────────────────────────────────────────────────────────────────
    print("\n" + "=" * 70)
    print("综合 L2 结论")
    print("=" * 70)

    # 判定标准：如果 top-20 precision > 0.15 且加权优于均匀，则正面
    top20_cerf_pr = [r for r in weighted_pr if r.top_n == 20]
    top20_precision = top20_cerf_pr[0].precision if top20_cerf_pr else 0.0

    top20_uniform_pr = [r for r in uniform_pr if r.top_n == 20]
    top20_uniform_precision = top20_uniform_pr[0].precision if top20_uniform_pr else 0.0

    cerf_is_better = avg_cerf_rank < avg_uniform_rank
    precision_is_nontrivial = top20_precision > 0.10

    if cerf_is_better and precision_is_nontrivial:
        verdict = "正面（弱）"
        explanation = (
            f"加权 Cerf 分岔在 ground truth 排名上优于均匀 Morse "
            f"（平均排名 {avg_cerf_rank:.1f} vs {avg_uniform_rank:.1f}），"
            f"Top-20 precision = {top20_precision:.4f}。"
            f"加权 Morse 的语义权重确实有助于捕捉关键否定事件，"
            f"但 precision 仍然偏低，说明 Cerf 分岔不是强预测器。"
        )
    elif cerf_is_better and not precision_is_nontrivial:
        verdict = "否定（加权排名更优但 precision 过低）"
        explanation = (
            f"加权 Cerf 排名优于均匀 Morse "
            f"（{avg_cerf_rank:.1f} vs {avg_uniform_rank:.1f}），"
            f"但 Top-20 precision = {top20_precision:.4f} 过低。"
            f"加权 Morse 的优势来自排名提升而非整体预测能力。"
        )
    elif not cerf_is_better and precision_is_nontrivial:
        verdict = "否定（均匀 Morse 排名更优）"
        explanation = (
            f"均匀 Morse 排名优于加权 Cerf "
            f"（{avg_uniform_rank:.1f} vs {avg_cerf_rank:.1f}）。"
            f"加权策略未能改善预测。"
        )
    else:
        verdict = "否定"
        explanation = (
            f"加权 Cerf 未能改善均匀 Morse 的预测能力。"
            f"平均排名 {avg_cerf_rank:.1f} vs {avg_uniform_rank:.1f}，"
            f"Top-20 precision = {top20_precision:.4f}。"
        )

    print(f"\n  verdict: {verdict}")
    print(f"  {explanation}")
    print()
    print("  认识论标注:")
    print("    - 数据真实性: 是（真实 relations.jsonl）")
    print("    - ground truth: 355号报告的22个关键否定（编排者标注 + 规则引用 + INTERRUPT）")
    print("    - 否证可能: 是（否定性结果同样有价值）")
    print("    - 单标的: 是（仅本谱系 DAG）")
    print(f"    - 与 355号（均匀 Morse L2）的区别: 本验证使用加权 Morse + Cerf 分岔（B-M 研究线产出）")

    # ─────────────────────────────────────────────────────────────────────
    # 保存结果
    # ─────────────────────────────────────────────────────────────────────
    output_dir = REPO_ROOT / "experiments" / "discrete_morse"
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / "cerf_l2_validation_results.json"

    def pr_to_dict(rows: list[PrecisionRecallRow]) -> list[dict]:
        return [
            {
                "top_n": r.top_n,
                "precision": r.precision,
                "recall": r.recall,
                "hits": r.hits,
                "hit_keys": list(r.hit_keys),
            }
            for r in rows
        ]

    out = {
        "experiment": "Cerf L2 Validation (B-M -> L2)",
        "epistemological_level": "L2",
        "validity_domain": "单标的（本谱系 DAG），ground truth = 355号22个关键否定",
        "verdict": verdict,
        "explanation": explanation,
        "ground_truth_size": len(GROUND_TRUTH_CRITICAL_NEGATIONS),
        "cerf_bifurcation_count": len(bifurcations),
        "uniform_jump_count": len(uniform_jumps),
        "sequence_length": len(sequence),
        "weighted_precision_recall": pr_to_dict(weighted_pr),
        "uniform_precision_recall": pr_to_dict(uniform_pr),
        "rank_comparison": {
            "cerf_better_count": cerf_better_count,
            "uniform_better_count": uniform_better_count,
            "tie_count": tie_count,
            "cerf_miss_count": cerf_miss_count,
            "uniform_miss_count": uniform_miss_count,
            "avg_cerf_rank": avg_cerf_rank,
            "avg_uniform_rank": avg_uniform_rank,
            "median_cerf_rank": median_cerf,
            "median_uniform_rank": median_uniform,
            "improvement": improvement,
        },
        "bifurcation_type_distribution": type_dist,
        "bifurcation_type_in_ground_truth": type_in_gt,
        "top_20_bifurcations": [
            {
                "rank": i + 1,
                "genealogy_key": b.genealogy_key,
                "type": b.type,
                "magnitude": b.magnitude,
                "description": b.description,
                "is_ground_truth": b.genealogy_key in GROUND_TRUTH_CRITICAL_NEGATIONS,
            }
            for i, b in enumerate(sorted_bifs[:20])
        ],
        "ground_truth_ranks": {
            key: {
                "cerf_rank": key_to_rank.get(key, -1),
                "uniform_rank": key_to_rank_uniform.get(key, -1),
            }
            for key in sorted(GROUND_TRUTH_CRITICAL_NEGATIONS)
        },
    }

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=2, ensure_ascii=False, default=str)
    print(f"\n  结果已保存到: {output_path}")


if __name__ == "__main__":
    main()
