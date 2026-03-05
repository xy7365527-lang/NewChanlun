"""
置换检验：negates 边在最优 Morse 临界集中是否过表达

背景：
- 373号 Fisher 检验发现 negates 过表达（p=0.0058）
- 374号否定 Fisher 独立性假设（DAG 边高度相关）
- 378号研究线方向1：用置换检验替代 Fisher

方法：
1. 保持图拓扑结构（所有边的位置）不变
2. 随机打乱所有边的类型标签（保持各类型数量比例不变）
3. 在原始图上运行最优 Morse（只需运行一次，临界集不变）
4. 在每次置换中记录"随机 negates"落入临界集的数量
5. 重复 10000 次，构建零分布
6. 观察真实 negates 在临界集中的数量在零分布中的位置

认识论等级：L2（真实 relations.jsonl，不依赖独立性假设）
谱系依据：374号 → 378号方向1
"""

from __future__ import annotations

import json
import random
import sys
from collections import Counter
from pathlib import Path

# Add paths
REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))
sys.path.insert(0, str(REPO_ROOT / "experiments" / "discrete_morse"))

from simplicial_complex import SimplicialComplex, build_simplicial_complex, compute_betti_numbers
from optimal_morse import build_gradient_field, GradientField


def load_relations(relations_path: Path) -> list[dict]:
    """Load relations from JSONL."""
    relations = []
    with open(relations_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            relations.append(json.loads(line))
    return relations


def get_edge_type_map(relations: list[dict]) -> dict[frozenset, str]:
    """Build mapping from edge (frozenset of 2 node hashes) to relation type."""
    edge_types: dict[frozenset, str] = {}
    for r in relations:
        src = r["from"]
        tgt = r["to"]
        edge_key = frozenset([src, tgt])
        rel_type = r.get("relation", "unknown")
        # If multiple relations between same pair, keep the one with highest "semantic weight"
        # negates > depends_on > related > others
        priority = {"negates": 10, "negated_by": 10, "depends_on": 5, "related": 3}
        existing = edge_types.get(edge_key)
        if existing is None or priority.get(rel_type, 1) > priority.get(existing, 1):
            edge_types[edge_key] = rel_type
    return edge_types


def run_permutation_test(
    sc: SimplicialComplex,
    field: GradientField,
    edge_type_map: dict[frozenset, str],
    num_permutations: int = 10000,
    seed: int = 42,
) -> dict:
    """Run permutation test for negates enrichment in Morse critical set."""

    rng = random.Random(seed)

    # Get all edges and their types
    edge_list = list(sc.edges)

    # Map each edge to its type
    edge_types = []
    for edge in edge_list:
        etype = edge_type_map.get(edge, "unknown")
        edge_types.append(etype)

    # Count types
    type_counts = Counter(edge_types)
    n_edges = len(edge_list)

    # Determine which edges are critical (unpaired_1)
    critical_set = set(field.unpaired_1)
    is_critical = [edge in critical_set for edge in edge_list]
    n_critical = sum(is_critical)

    # Count negates+negated_by edges
    negates_types = {"negates", "negated_by"}
    is_negates = [t in negates_types for t in edge_types]
    n_negates = sum(is_negates)

    # Observed: how many negates edges are critical
    observed_negates_critical = sum(
        1 for i in range(n_edges) if is_negates[i] and is_critical[i]
    )

    print(f"Graph: {n_edges} edges, {n_critical} critical ({n_critical/n_edges*100:.1f}%)")
    print(f"Negates edges: {n_negates} total, {observed_negates_critical} critical ({observed_negates_critical}/{n_negates}={observed_negates_critical/max(n_negates,1)*100:.1f}%)")
    print(f"Running {num_permutations} permutations...")

    # Permutation test
    # In each permutation: shuffle edge_types, count how many of the
    # randomly-assigned "negates" labels fall on critical edges
    null_distribution = []
    indices = list(range(n_edges))

    for p in range(num_permutations):
        # Shuffle the type labels
        shuffled_types = edge_types[:]
        rng.shuffle(shuffled_types)

        # Count negates-labeled edges that are critical
        perm_negates_critical = sum(
            1 for i in range(n_edges)
            if shuffled_types[i] in negates_types and is_critical[i]
        )
        null_distribution.append(perm_negates_critical)

        if (p + 1) % 2000 == 0:
            print(f"  {p+1}/{num_permutations} done...")

    # Compute p-value (one-sided: observed >= null)
    p_value = sum(1 for x in null_distribution if x >= observed_negates_critical) / num_permutations

    # Null distribution stats
    null_mean = sum(null_distribution) / len(null_distribution)
    null_sorted = sorted(null_distribution)
    null_median = null_sorted[len(null_sorted) // 2]
    null_95 = null_sorted[int(0.95 * len(null_sorted))]
    null_99 = null_sorted[int(0.99 * len(null_sorted))]
    null_max = max(null_distribution)

    # Distribution of null values
    null_counter = Counter(null_distribution)

    result = {
        "experiment": "Permutation Test: negates enrichment in Morse critical set",
        "epistemological_level": "L2",
        "genealogy": "374号 → 378号方向1",
        "graph": {
            "n_edges": n_edges,
            "n_critical": n_critical,
            "critical_rate": n_critical / n_edges,
        },
        "negates": {
            "n_total": n_negates,
            "n_critical_observed": observed_negates_critical,
            "critical_rate_observed": observed_negates_critical / max(n_negates, 1),
        },
        "type_distribution": dict(type_counts.most_common()),
        "permutation_test": {
            "n_permutations": num_permutations,
            "p_value_one_sided": p_value,
            "null_mean": null_mean,
            "null_median": null_median,
            "null_95_percentile": null_95,
            "null_99_percentile": null_99,
            "null_max": null_max,
            "null_distribution_counts": dict(sorted(null_counter.items())),
        },
        "conclusion": "",
    }

    # Determine conclusion
    if p_value < 0.01:
        result["conclusion"] = (
            f"negates 过表达显著（p={p_value:.4f} < 0.01）。"
            f"真实 negates 临界数={observed_negates_critical}/{n_negates}，"
            f"零分布均值={null_mean:.2f}。"
            f"373号本体论主张（negates=概念空间骨架）获得严格 L2 支持。"
        )
    elif p_value < 0.05:
        result["conclusion"] = (
            f"negates 过表达边缘显著（p={p_value:.4f}，0.01<p<0.05）。"
            f"需要更多数据或更强检验方法。"
        )
    else:
        result["conclusion"] = (
            f"negates 过表达不显著（p={p_value:.4f} >= 0.05）。"
            f"Fisher p=0.0058 为拓扑位置效应导致的伪显著性。"
            f"373号本体论主张（negates=骨架）被否定。"
        )

    return result


def main() -> None:
    relations_path = REPO_ROOT / ".chanlun" / "block-topology" / "relations.jsonl"

    if not relations_path.exists():
        print(f"Error: {relations_path} not found")
        sys.exit(1)

    print("=== 置换检验：negates 过表达 ===\n")

    # 1. Load relations and build complex
    relations = load_relations(relations_path)
    print(f"Loaded {len(relations)} relations")

    sc = build_simplicial_complex(relations)
    print(f"Complex: V={sc.num_vertices} E={sc.num_edges} T={sc.num_triangles}")

    # 2. Build edge type map
    edge_type_map = get_edge_type_map(relations)

    # 3. Compute optimal Morse (only once - topology doesn't change)
    print("\nComputing optimal Morse function (10 restarts)...")
    field = build_gradient_field(sc, num_restarts=10, seed=42)
    print(f"Critical set: {field.num_critical} "
          f"(c0={len(field.unpaired_0)} c1={len(field.unpaired_1)} c2={len(field.unpaired_2)})")

    # 4. Run permutation test
    print()
    result = run_permutation_test(
        sc, field, edge_type_map,
        num_permutations=10000, seed=42,
    )

    # 5. Print results
    print(f"\n=== 结果 ===")
    print(f"Observed: {result['negates']['n_critical_observed']}/{result['negates']['n_total']} negates critical "
          f"({result['negates']['critical_rate_observed']*100:.1f}%)")
    pt = result["permutation_test"]
    print(f"Null distribution: mean={pt['null_mean']:.2f}, "
          f"95th={pt['null_95_percentile']}, 99th={pt['null_99_percentile']}, max={pt['null_max']}")
    print(f"p-value (one-sided): {pt['p_value_one_sided']:.4f}")
    print(f"\n结论: {result['conclusion']}")

    # 6. Save
    output_path = REPO_ROOT / "tmp" / "permutation_test_report.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)
    print(f"\nReport saved to: {output_path}")


if __name__ == "__main__":
    main()
