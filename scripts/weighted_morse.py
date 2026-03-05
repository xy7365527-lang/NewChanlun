"""
加权离散 Morse 函数构造（阶段 B-M 第二步）

在 optimal_morse.py（均匀权重）基础上，按关系类型加权构造 Morse 函数。
核心假设：否定性关系携带更大信息差，应优先保留为临界边。

加权策略：
- 配对优先级：优先配对低权重边（低语义重量 = 结构性骨架，可配对消除）
- 保留高权重边为临界（高语义重量 = 否定/扬弃，是拓扑信息的载体）

认识论等级：L0（加权定义是代数构造）+ L1（合成数据验证管线正确性）
有效域：加权复形上代数正确，加权 vs 均匀的差异是否预测"关键否定"待 L2 验证

谱系依据：355号（L2 否定性结果）→ 阶段 B-M → optimal_morse.py → 本模块
"""

from __future__ import annotations

import json
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import FrozenSet

# DM1 模块复用
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "experiments" / "discrete_morse"))
from simplicial_complex import (
    BettiNumbers,
    SimplicialComplex,
    build_simplicial_complex,
    compute_betti_numbers,
)

from optimal_morse import (
    GradientField,
    OptimalMorseResult,
    _build_complex_index,
    _coreduction_with_order,
    build_gradient_field,
    optimal_morse_function,
    verify_gradient_field,
)


# ─────────────────────────────────────────────────────────────────────────────
# 默认权重
# ─────────────────────────────────────────────────────────────────────────────

DEFAULT_WEIGHTS: dict[str, int] = {
    "negates": 4,
    "negated_by": 4,
    "supersedes": 3,
    "splits": 3,
    "modifies": 2,
    "refines": 2,
    "tensions_with": 2,
    "reopens": 2,
    "depends_on": 1,
    "references": 1,
    "defines": 1,
    "related": 1,
    "records": 1,
    "extends": 1,
    "residue_of": 1,
}


# ─────────────────────────────────────────────────────────────────────────────
# 加权复形数据结构
# ─────────────────────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class WeightedComplex:
    """加权单纯复形：在 SimplicialComplex 基础上附加边权重。"""
    simplicial_complex: SimplicialComplex
    edge_weights: dict[FrozenSet[str], float]  # edge -> weight (>= 1)
    relation_type_counts: dict[str, int]  # 统计信息


@dataclass(frozen=True)
class WeightedMorseResult:
    """加权 Morse 函数结果。"""
    gradient_field: GradientField
    betti: BettiNumbers
    is_perfect: bool
    best_critical_count: int
    critical_edge_weights: list[float]  # 临界边的权重列表


@dataclass(frozen=True)
class ComparisonResult:
    """加权 vs 均匀 Morse 对比结果。"""
    uniform_critical_count: int
    weighted_critical_count: int
    uniform_critical_edges: tuple[FrozenSet[str], ...]
    weighted_critical_edges: tuple[FrozenSet[str], ...]
    only_in_weighted: tuple[FrozenSet[str], ...]   # 仅在加权中为临界
    only_in_uniform: tuple[FrozenSet[str], ...]    # 仅在均匀中为临界
    avg_weight_weighted_critical: float  # 加权临界边的平均权重
    avg_weight_uniform_critical: float   # 均匀临界边的平均权重（用加权值衡量）
    betti: BettiNumbers


# ─────────────────────────────────────────────────────────────────────────────
# 加权复形构造
# ─────────────────────────────────────────────────────────────────────────────

def build_weighted_complex(
    relations_path: str,
    weights: dict[str, int] | None = None,
) -> WeightedComplex:
    """从 relations.jsonl 构造加权单纯复形。

    边的权重 = 该边上所有关系类型权重的最大值。
    （同一对节点可能有多种关系类型，取最重的——信息差最大的关系决定边的语义重量。）

    Parameters
    ----------
    relations_path : str
        relations.jsonl 文件路径。
    weights : dict[str, int] | None
        关系类型 -> 权重映射。None 时使用 DEFAULT_WEIGHTS。

    Returns
    -------
    WeightedComplex
    """
    if weights is None:
        weights = DEFAULT_WEIGHTS

    relations: list[dict] = []
    with open(relations_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            relations.append(json.loads(line))

    return build_weighted_complex_from_relations(relations, weights)


def build_weighted_complex_from_relations(
    relations: list[dict],
    weights: dict[str, int] | None = None,
) -> WeightedComplex:
    """从关系列表构造加权单纯复形（不需要文件 I/O）。

    Parameters
    ----------
    relations : list[dict]
        关系记录列表，每条记录至少有 from, to 字段，可选 relation/type 字段。
    weights : dict[str, int] | None
        关系类型 -> 权重映射。None 时使用 DEFAULT_WEIGHTS。

    Returns
    -------
    WeightedComplex
    """
    if weights is None:
        weights = DEFAULT_WEIGHTS

    # 收集边的最大权重
    edge_max_weight: dict[FrozenSet[str], float] = {}
    type_counts: dict[str, int] = defaultdict(int)

    for rel in relations:
        src = rel["from"]
        tgt = rel["to"]
        if src == tgt:
            continue
        edge = frozenset([src, tgt])
        rel_type = rel.get("relation", rel.get("type", "unknown"))
        w = weights.get(rel_type, 1)
        type_counts[rel_type] += 1
        edge_max_weight[edge] = max(edge_max_weight.get(edge, 0), w)

    sc = build_simplicial_complex(relations)

    # 为复形中的边赋权（复形可能有三角形闭包引入的边，
    # 这些边如果没有直接关系记录则权重为 1）
    final_weights: dict[FrozenSet[str], float] = {}
    for e in sc.edges:
        final_weights[e] = edge_max_weight.get(e, 1.0)

    return WeightedComplex(
        simplicial_complex=sc,
        edge_weights=final_weights,
        relation_type_counts=dict(type_counts),
    )


# ─────────────────────────────────────────────────────────────────────────────
# 加权 Morse 函数构造
# ─────────────────────────────────────────────────────────────────────────────

def weighted_morse_function(complex_data: WeightedComplex) -> WeightedMorseResult:
    """构造加权离散 Morse 函数。

    策略：配对优先级由边权重决定——优先配对低权重边，保留高权重边为临界。

    实现：利用 coreduction 算法的种子顺序注入点。
    将单形按"期望配对优先级"排序：
    - 0-单形（顶点）：按连接的最大边权重升序（低权重邻域优先配对）
    - 1-单形（边）：按边权重升序（低权重边优先配对 = 高权重边更可能残留为临界）
    - 2-单形（三角形）：按三边权重之和升序

    同维度内按期望配对优先级排序；维度间按低维优先（coreduction 自然顺序）。
    """
    sc = complex_data.simplicial_complex
    ew = complex_data.edge_weights

    all_simplices, simplex_to_idx, facets, cofacets = _build_complex_index(sc)
    n = len(all_simplices)

    if n == 0:
        betti = compute_betti_numbers(sc)
        return WeightedMorseResult(
            gradient_field=GradientField(
                pairs=(), unpaired_0=(), unpaired_1=(), unpaired_2=(),
            ),
            betti=betti,
            is_perfect=True,
            best_critical_count=0,
            critical_edge_weights=[],
        )

    # 为每个单形计算排序键
    def sort_key(idx: int) -> tuple[int, float]:
        dim, simplex = all_simplices[idx]
        if dim == 0:
            # 顶点：按其所连边的最大权重升序
            max_w = 0.0
            for cf_idx in cofacets[idx]:
                cf_simplex = all_simplices[cf_idx][1]
                max_w = max(max_w, ew.get(cf_simplex, 1.0))
            return (0, max_w)
        elif dim == 1:
            # 边：按边权重升序（低权重优先配对）
            return (1, ew.get(simplex, 1.0))
        else:
            # 三角形：按三边权重之和升序
            total_w = 0.0
            for f_idx in facets[idx]:
                f_simplex = all_simplices[f_idx][1]
                total_w += ew.get(f_simplex, 1.0)
            return (2, total_w)

    order = sorted(range(n), key=sort_key)
    field = _coreduction_with_order(all_simplices, facets, cofacets, order)

    betti = compute_betti_numbers(sc)

    # 收集临界边的权重
    critical_edge_ws = [
        ew.get(e, 1.0) for e in field.unpaired_1
    ]

    return WeightedMorseResult(
        gradient_field=field,
        betti=betti,
        is_perfect=field.is_perfect(betti),
        best_critical_count=field.num_critical,
        critical_edge_weights=sorted(critical_edge_ws, reverse=True),
    )


# ─────────────────────────────────────────────────────────────────────────────
# 加权 vs 均匀对比
# ─────────────────────────────────────────────────────────────────────────────

def compare_weighted_vs_uniform(complex_data: WeightedComplex) -> ComparisonResult:
    """对比加权 vs 均匀 Morse 的临界集差异。

    返回：哪些边在加权中成为临界但在均匀中不是（以及反向）。
    """
    sc = complex_data.simplicial_complex
    ew = complex_data.edge_weights

    # 加权 Morse
    weighted_result = weighted_morse_function(complex_data)
    weighted_field = weighted_result.gradient_field

    # 均匀 Morse（使用 optimal_morse 的最优策略）
    uniform_field = build_gradient_field(sc, num_restarts=50, seed=42)

    betti = weighted_result.betti

    # 临界边集合
    weighted_critical_edges = set(weighted_field.unpaired_1)
    uniform_critical_edges = set(uniform_field.unpaired_1)

    only_weighted = weighted_critical_edges - uniform_critical_edges
    only_uniform = uniform_critical_edges - weighted_critical_edges

    # 计算平均权重
    def avg_weight(edges: set[FrozenSet[str]]) -> float:
        if not edges:
            return 0.0
        return sum(ew.get(e, 1.0) for e in edges) / len(edges)

    return ComparisonResult(
        uniform_critical_count=uniform_field.num_critical,
        weighted_critical_count=weighted_field.num_critical,
        uniform_critical_edges=tuple(sorted(uniform_field.unpaired_1, key=str)),
        weighted_critical_edges=tuple(sorted(weighted_field.unpaired_1, key=str)),
        only_in_weighted=tuple(sorted(only_weighted, key=str)),
        only_in_uniform=tuple(sorted(only_uniform, key=str)),
        avg_weight_weighted_critical=avg_weight(weighted_critical_edges),
        avg_weight_uniform_critical=avg_weight(uniform_critical_edges),
        betti=betti,
    )


# ─────────────────────────────────────────────────────────────────────────────
# 主入口
# ─────────────────────────────────────────────────────────────────────────────

def main() -> None:
    repo_root = Path(__file__).resolve().parent.parent
    relations_path = repo_root / ".chanlun" / "block-topology" / "relations.jsonl"

    if not relations_path.exists():
        print(f"Error: {relations_path} not found")
        sys.exit(1)

    print("=== 加权离散 Morse 函数实验 ===\n")
    print(f"认识论等级: L0（加权定义）+ L1（合成验证）")
    print(f"有效域: 加权 vs 均匀差异的预测能力待 L2 验证\n")

    # 1. 构造加权复形
    print("构造加权复形...")
    wc = build_weighted_complex(str(relations_path))
    sc = wc.simplicial_complex
    print(f"  复形规模: V={sc.num_vertices} E={sc.num_edges} T={sc.num_triangles}")
    print(f"  关系类型分布: {wc.relation_type_counts}")
    print(f"  权重表: {DEFAULT_WEIGHTS}")

    # 2. 加权 Morse
    print("\n构造加权 Morse 函数...")
    w_result = weighted_morse_function(wc)
    wf = w_result.gradient_field
    print(f"  加权 Morse: m0={len(wf.unpaired_0)} m1={len(wf.unpaired_1)} "
          f"m2={len(wf.unpaired_2)} (total={wf.num_critical})")
    print(f"  Betti 下界: b0={w_result.betti.beta_0} b1={w_result.betti.beta_1} "
          f"b2={w_result.betti.beta_2}")
    print(f"  完美: {w_result.is_perfect}")
    if w_result.critical_edge_weights:
        print(f"  临界边权重分布: "
              f"max={max(w_result.critical_edge_weights):.0f} "
              f"min={min(w_result.critical_edge_weights):.0f} "
              f"avg={sum(w_result.critical_edge_weights)/len(w_result.critical_edge_weights):.2f}")

    # 3. 验证
    print("\n验证加权梯度向量场...")
    v = verify_gradient_field(sc, wf)
    print(f"  全部合法: {v['all_valid']}")

    # 4. 对比
    print("\n对比加权 vs 均匀...")
    comp = compare_weighted_vs_uniform(wc)
    print(f"  均匀临界总数: {comp.uniform_critical_count}")
    print(f"  加权临界总数: {comp.weighted_critical_count}")
    print(f"  仅在加权中为临界的边数: {len(comp.only_in_weighted)}")
    print(f"  仅在均匀中为临界的边数: {len(comp.only_in_uniform)}")
    print(f"  加权临界边平均权重: {comp.avg_weight_weighted_critical:.2f}")
    print(f"  均匀临界边平均权重(加权值): {comp.avg_weight_uniform_critical:.2f}")

    # 5. 保存结果
    output_dir = repo_root / "experiments" / "discrete_morse"
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / "weighted_morse_results.json"

    out = {
        "experiment": "Weighted Morse Function (B-M Step 2)",
        "epistemological_level": "L0+L1",
        "validity_domain": "加权复形上代数正确，加权 vs 均匀差异是否预测关键否定待 L2 验证",
        "weights": DEFAULT_WEIGHTS,
        "complex": {
            "vertices": sc.num_vertices,
            "edges": sc.num_edges,
            "triangles": sc.num_triangles,
        },
        "relation_type_counts": wc.relation_type_counts,
        "betti": {
            "beta_0": w_result.betti.beta_0,
            "beta_1": w_result.betti.beta_1,
            "beta_2": w_result.betti.beta_2,
        },
        "weighted_morse": {
            "m0": len(wf.unpaired_0),
            "m1": len(wf.unpaired_1),
            "m2": len(wf.unpaired_2),
            "is_perfect": w_result.is_perfect,
            "critical_edge_weight_stats": {
                "count": len(w_result.critical_edge_weights),
                "max": max(w_result.critical_edge_weights) if w_result.critical_edge_weights else 0,
                "min": min(w_result.critical_edge_weights) if w_result.critical_edge_weights else 0,
                "avg": (sum(w_result.critical_edge_weights) / len(w_result.critical_edge_weights)
                        if w_result.critical_edge_weights else 0),
            },
        },
        "comparison": {
            "uniform_critical_count": comp.uniform_critical_count,
            "weighted_critical_count": comp.weighted_critical_count,
            "only_in_weighted_count": len(comp.only_in_weighted),
            "only_in_uniform_count": len(comp.only_in_uniform),
            "avg_weight_weighted_critical": comp.avg_weight_weighted_critical,
            "avg_weight_uniform_critical": comp.avg_weight_uniform_critical,
        },
        "verification": {k: v_val for k, v_val in v.items()
                         if k not in ("missing", "extra", "conflicts", "dim_errors")},
    }

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=2, ensure_ascii=False)
    print(f"\n结果已保存到: {output_path}")


if __name__ == "__main__":
    main()
