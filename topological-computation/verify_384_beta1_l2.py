"""384号 L2验证：Δβ₁=(c-1)+n_loop 公式在真实380区块数据上的验证。

认识论等级：L2（真实数据，单标的/单时段）。
验证方法：
1. 从 block-topology 加载真实图
2. 对图中随机采样的活跃顶点对执行 fold 操作
3. 比较公式预测值 vs 实际 β₁ 差值
4. 记录所有匹配/不匹配，包括否定性结果

否定性结果比确认性结果更有价值——否定性结果缩小有效域边界。
"""

from __future__ import annotations

import json
import random
import time
from collections import Counter
from pathlib import Path

from engine import (
    Graph, Edge, EdgeType, Vertex, VertexStatus,
    compute_beta_1, _connected_components,
    SettlementTracker, fold, OperationResult,
)
from genealogy_loader import load_from_repo


def compute_lower_link_info(graph: Graph, vertices: list[str]) -> dict:
    """Compute lower link details for a fold operation.

    Returns dict with c (connected components), n_loop (internal edges),
    lower_link_vids, and lower_link_edges for inspection.
    """
    active = set(graph.active_vertex_ids())
    s = set(vertices)

    # n_loop: edges within S (including self-loops)
    n_loop = sum(
        1 for e in graph.active_edges()
        if e.source in s and e.target in s
    )

    # lower link: vertices adjacent to S but not in S
    lower_link_vids: set[str] = set()
    for v in s:
        for n in graph.neighbors(v):
            if n not in s and n in active:
                lower_link_vids.add(n)

    # connected components of lower link
    if lower_link_vids:
        ll_edges: list[frozenset[str]] = []
        for e in graph.active_edges():
            if (e.source in lower_link_vids and e.target in lower_link_vids
                    and e.source != e.target):
                ll_edges.append(frozenset((e.source, e.target)))
        c = _connected_components(sorted(lower_link_vids), ll_edges)
    else:
        c = 0

    predicted = (c - 1 if c > 0 else 0) + n_loop

    return {
        "c": c,
        "n_loop": n_loop,
        "predicted_delta": predicted,
        "lower_link_size": len(lower_link_vids),
        "lower_link_edges": len(set(ll_edges)) if lower_link_vids else 0,
    }


def find_fold_candidates(graph: Graph, max_pairs: int = 200, seed: int = 42) -> list[tuple[str, str]]:
    """Find candidate vertex pairs for fold verification.

    Strategy: sample pairs that share at least one neighbor (fold-eligible).
    """
    active_vids = graph.active_vertex_ids()
    if len(active_vids) < 2:
        return []

    rng = random.Random(seed)
    candidates: list[tuple[str, str]] = []
    seen: set[frozenset[str]] = set()

    # Build neighbor sets for fast lookup
    neighbor_map: dict[str, set[str]] = {}
    active_set = set(active_vids)
    for v in active_vids:
        neighbor_map[v] = set(graph.neighbors(v)) & active_set

    attempts = 0
    max_attempts = max_pairs * 20

    while len(candidates) < max_pairs and attempts < max_attempts:
        v = rng.choice(active_vids)
        nbs = list(neighbor_map.get(v, set()))
        if not nbs:
            attempts += 1
            continue
        w = rng.choice(nbs)
        key = frozenset((v, w))
        if key not in seen and v != w:
            seen.add(key)
            candidates.append((v, w))
        attempts += 1

    return candidates


def verify_single_fold(graph: Graph, v: str, w: str, step: int) -> dict:
    """Verify Δβ₁=(c-1)+n_loop for a single fold of (v, w).

    Returns detailed result dict including match/mismatch.
    """
    # Compute prediction using the formula
    info = compute_lower_link_info(graph, [v, w])
    predicted = info["predicted_delta"]

    # Compute actual β₁ change
    beta_before = compute_beta_1(graph)

    # Execute fold manually (without settlement blocking)
    keep, remove = v, w
    result_graph = graph.merge_vertices(keep, remove)

    beta_after = compute_beta_1(result_graph)
    actual = beta_after - beta_before

    match = predicted == actual

    return {
        "v": v[:64],
        "w": w[:64],
        "step": step,
        "beta_before": beta_before,
        "beta_after": beta_after,
        "predicted": predicted,
        "actual": actual,
        "match": match,
        "c": info["c"],
        "n_loop": info["n_loop"],
        "lower_link_size": info["lower_link_size"],
        "lower_link_edges": info["lower_link_edges"],
        "mismatch_delta": actual - predicted if not match else 0,
    }


def main():
    repo_root = Path("G:/NewChanlun")
    print("=" * 80)
    print("384号 L2验证: Δβ₁=(c-1)+n_loop")
    print("认识论等级: L2 (真实380区块数据)")
    print("=" * 80)

    # Step 1: Load real graph
    print("\n[1] Loading graph from block-topology...")
    t0 = time.time()
    graph, rel_counts = load_from_repo(repo_root)
    load_time = time.time() - t0

    active_vids = graph.active_vertex_ids()
    active_edges = graph.active_edges()
    undirected = graph.undirected_active_edges()
    self_loops = graph.self_loops()
    beta_1 = compute_beta_1(graph)
    components = _connected_components(active_vids, undirected)

    print(f"  Loaded in {load_time:.1f}s")
    print(f"  Active vertices: {len(active_vids)}")
    print(f"  Active edges: {len(active_edges)} (directed)")
    print(f"  Undirected edges: {len(undirected)}")
    print(f"  Self-loops: {len(self_loops)}")
    print(f"  β₁: {beta_1}")
    print(f"  Connected components: {components}")
    print(f"  Relation types: {rel_counts}")

    # Step 2: Find fold candidates
    print("\n[2] Finding fold candidates...")
    candidates = find_fold_candidates(graph, max_pairs=200, seed=42)
    print(f"  Found {len(candidates)} candidate pairs")

    # Step 3: Verify each fold
    print("\n[3] Verifying Δβ₁ formula on each pair...")
    results = []
    t0 = time.time()

    for i, (v, w) in enumerate(candidates):
        result = verify_single_fold(graph, v, w, step=i + 1)
        results.append(result)
        if (i + 1) % 50 == 0:
            elapsed = time.time() - t0
            matches = sum(1 for r in results if r["match"])
            print(f"  {i+1}/{len(candidates)}: "
                  f"{matches} match, {i+1-matches} mismatch ({elapsed:.1f}s)")

    verify_time = time.time() - t0

    # Step 4: Analyze results
    print("\n" + "=" * 80)
    print("RESULTS")
    print("=" * 80)

    total = len(results)
    matches = sum(1 for r in results if r["match"])
    mismatches = [r for r in results if not r["match"]]

    print(f"\n  Total pairs tested: {total}")
    print(f"  Matches (predicted == actual): {matches}")
    print(f"  Mismatches: {len(mismatches)}")
    print(f"  Match rate: {100*matches/max(total,1):.1f}%")
    print(f"  Verification time: {verify_time:.1f}s")

    # Distribution of predicted values
    pred_dist = Counter(r["predicted"] for r in results)
    print(f"\n  Predicted Δβ₁ distribution: {dict(sorted(pred_dist.items()))}")

    actual_dist = Counter(r["actual"] for r in results)
    print(f"  Actual Δβ₁ distribution: {dict(sorted(actual_dist.items()))}")

    # Distribution of c and n_loop
    c_dist = Counter(r["c"] for r in results)
    print(f"  c (lower link components) distribution: {dict(sorted(c_dist.items()))}")

    nloop_dist = Counter(r["n_loop"] for r in results)
    print(f"  n_loop distribution: {dict(sorted(nloop_dist.items()))}")

    # Mismatch analysis
    if mismatches:
        print(f"\n  --- MISMATCH DETAILS (否定性结果) ---")
        for m in mismatches[:20]:
            print(f"    v={m['v'][:32]}... w={m['w'][:32]}...")
            print(f"      c={m['c']}, n_loop={m['n_loop']}, "
                  f"predicted={m['predicted']}, actual={m['actual']}, "
                  f"delta={m['mismatch_delta']}")
            print(f"      lower_link: {m['lower_link_size']} verts, "
                  f"{m['lower_link_edges']} edges")

    # Verdict
    print("\n" + "=" * 80)
    if len(mismatches) == 0:
        print("VERDICT: 公式 Δβ₁=(c-1)+n_loop 在全部 "
              f"{total} 对真实数据上完全匹配。")
        print("L2验证结果: 未否证（公式在380区块数据有效域内成立）")
    else:
        print(f"VERDICT: 公式 Δβ₁=(c-1)+n_loop 在 {len(mismatches)}/{total} "
              f"对上不匹配。")
        print("L2验证结果: 部分否证——公式有效域需收窄")
        # Analyze mismatch patterns
        mismatch_patterns = Counter()
        for m in mismatches:
            pattern = f"c={m['c']},n_loop={m['n_loop']},delta={m['mismatch_delta']}"
            mismatch_patterns[pattern] += 1
        print(f"  不匹配模式: {dict(mismatch_patterns.most_common(10))}")
    print("=" * 80)

    # Save results
    output = {
        "experiment": "384号 L2验证: Δβ₁=(c-1)+n_loop",
        "epistemological_level": "L2",
        "data_source": "真实380区块 block-topology",
        "graph_stats": {
            "active_vertices": len(active_vids),
            "active_edges_directed": len(active_edges),
            "undirected_edges": len(undirected),
            "self_loops": len(self_loops),
            "beta_1": beta_1,
            "connected_components": components,
        },
        "verification": {
            "total_pairs": total,
            "matches": matches,
            "mismatches": len(mismatches),
            "match_rate": round(100 * matches / max(total, 1), 2),
        },
        "distributions": {
            "predicted": dict(sorted(pred_dist.items())),
            "actual": dict(sorted(actual_dist.items())),
            "c": dict(sorted(c_dist.items())),
            "n_loop": dict(sorted(nloop_dist.items())),
        },
        "mismatch_details": mismatches[:50],
        "sample_matches": [r for r in results if r["match"]][:20],
    }

    out_path = Path("G:/NewChanlun/tmp/384-L2-verification.json")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)
    print(f"\nResults saved to {out_path}")

    return output


if __name__ == "__main__":
    main()

