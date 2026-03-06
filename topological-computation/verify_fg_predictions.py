"""Q-R3-10: Verify f(v,w) and g(v,w) predictions on 2000-step experiment data.

Re-runs the deterministic experiment (seed=42) to capture exact (v,w) pairs
for fold and negate_a operations, then computes f(v,w) and g(v,w) for each pair
and compares against random baseline.
"""

from __future__ import annotations

import json
import random
import time
from collections import Counter, deque
from pathlib import Path

from engine import Graph, EdgeType, compute_beta_1, _connected_components
from traversal import TraversalEngine, EncounterType


# ---------------------------------------------------------------------------
# f(v,w) and g(v,w) computation
# ---------------------------------------------------------------------------

def compute_f(graph: Graph, v: str, w: str) -> int:
    """Compute predicted delta_beta_1 for folding {v, w}.

    f(v,w) = (c - 1) + n_loop
    where c = connected components of lower link, n_loop = edges within {v,w}.
    """
    active = set(graph.active_vertex_ids())
    s = {v, w}

    # n_loop: edges within {v, w}
    n_loop = sum(
        1 for e in graph.active_edges()
        if e.source in s and e.target in s
    )

    # lower link: neighbors of S not in S
    lower_link = set()
    for x in s:
        for n in graph.neighbors(x):
            if n not in s and n in active:
                lower_link.add(n)

    # connected components of lower link subgraph
    if lower_link:
        ll_edges = []
        for e in graph.active_edges():
            if e.source in lower_link and e.target in lower_link and e.source != e.target:
                ll_edges.append(frozenset((e.source, e.target)))
        c = _connected_components(sorted(lower_link), ll_edges)
    else:
        c = 0

    return (c - 1 if c > 0 else 0) + n_loop


def compute_g(graph: Graph, v: str, w: str) -> int:
    """Compute number of vertex-disjoint paths from v to w.

    Uses iterative path-removal (equivalent to max-flow on unit-capacity vertices).
    Treats graph as undirected for path finding.
    """
    active = set(graph.active_vertex_ids())
    if v not in active or w not in active:
        return 0

    # Build undirected adjacency
    adj: dict[str, set[str]] = {vid: set() for vid in active}
    for e in graph.active_edges():
        if e.source in active and e.target in active and e.source != e.target:
            adj[e.source].add(e.target)
            adj[e.target].add(e.source)

    count = 0
    removed: set[str] = set()

    for _ in range(50):  # max 50 disjoint paths
        # BFS from v to w avoiding removed internal vertices
        visited: set[str] = set()
        parent: dict[str, str] = {}
        queue = deque([v])
        found = False

        while queue:
            cur = queue.popleft()
            if cur == w:
                found = True
                break
            if cur in visited:
                continue
            visited.add(cur)
            for nb in adj.get(cur, set()):
                if nb not in visited and nb not in removed:
                    if nb not in parent:  # first discovery only
                        parent[nb] = cur
                        queue.append(nb)

        if not found:
            break

        count += 1
        # Remove internal vertices of this path (not v or w)
        node = w
        path_internal = []
        while node in parent:
            prev = parent[node]
            if prev != v and node != w:
                path_internal.append(node)
            node = prev
        for internal in path_internal:
            removed.add(internal)

    return count


# ---------------------------------------------------------------------------
# Component finder (for start vertex selection)
# ---------------------------------------------------------------------------

def _find_components(graph: Graph) -> list[list[str]]:
    active_vids = set(graph.active_vertex_ids())
    undirected_edges = graph.undirected_active_edges()
    adj: dict[str, set[str]] = {v: set() for v in active_vids}
    for edge_set in undirected_edges:
        pair = sorted(edge_set)
        if len(pair) == 2:
            adj[pair[0]].add(pair[1])
            adj[pair[1]].add(pair[0])
    visited: set[str] = set()
    components: list[list[str]] = []
    for start in sorted(active_vids):
        if start in visited:
            continue
        component: list[str] = []
        stack = [start]
        while stack:
            v = stack.pop()
            if v in visited:
                continue
            visited.add(v)
            component.append(v)
            for nb in adj.get(v, ()):
                if nb not in visited:
                    stack.append(nb)
        components.append(sorted(component))
    components.sort(key=lambda c: -len(c))
    return components


def _vertex_degrees(graph: Graph) -> dict[str, int]:
    degrees: dict[str, int] = Counter()
    for edge_set in graph.undirected_active_edges():
        for v in edge_set:
            degrees[v] += 1
    return dict(degrees)


# ---------------------------------------------------------------------------
# CrossComponentTraversal (replicated from experiment_long_run.py)
# ---------------------------------------------------------------------------

class CrossComponentTraversal:
    def __init__(self, graph, start, settlement_threshold=15, seed=42, visit_cap=3):
        self.engine = TraversalEngine(
            graph, start=start,
            settlement_threshold=settlement_threshold,
            seed=seed, use_llm=False,
        )
        self.visit_cap = visit_cap
        self.components = _find_components(graph)
        self._component_map = {}
        for idx, comp in enumerate(self.components):
            for v in comp:
                self._component_map[v] = idx
        self.cross_jumps = []

    def _should_jump(self):
        comp_idx = self._component_map.get(self.engine.position, 0)
        comp_vids = set(self.components[comp_idx])
        active = set(self.engine.k_active.active_vertex_ids())
        comp_active = comp_vids & active
        if not comp_active:
            return True
        visit_counts = Counter(self.engine.visit_history)
        return all(visit_counts.get(v, 0) > self.visit_cap for v in comp_active)

    def _jump_to_next_component(self, step):
        visit_counts = Counter(self.engine.visit_history)
        active = set(self.engine.k_active.active_vertex_ids())
        current_comp = self._component_map.get(self.engine.position, 0)
        best_comp = -1
        best_size = 0
        best_target = ""
        for idx, comp in enumerate(self.components):
            if idx == current_comp:
                continue
            comp_active = [v for v in comp if v in active]
            if not comp_active:
                continue
            under_explored = [v for v in comp_active if visit_counts.get(v, 0) <= self.visit_cap]
            if under_explored and len(comp_active) > best_size:
                best_comp = idx
                best_size = len(comp_active)
                best_target = min(under_explored, key=lambda v: (visit_counts.get(v, 0), v))
        if best_comp >= 0:
            self.engine.position = best_target
            self.engine.visit_history.append(best_target)
            self.engine._nothing_streak = 0
            self.cross_jumps.append({
                "step": step,
                "from_component": current_comp,
                "to_component": best_comp,
            })
            return True
        return False

    def run_step(self):
        if self._should_jump():
            self._jump_to_next_component(self.engine.step + 1)
        self.engine.run_step()


# ---------------------------------------------------------------------------
# Instrumented run: capture (v, w) pairs
# ---------------------------------------------------------------------------

def run_instrumented():
    from genealogy_loader import load_from_repo

    repo_root = Path("C:/Users/hanju/NewChanlun")
    print("Loading graph...")
    graph, _ = load_from_repo(repo_root)

    # Find start vertex (same as original experiment)
    components = _find_components(graph)
    largest_component = components[0]
    degrees = _vertex_degrees(graph)
    start_vertex = max(largest_component, key=lambda v: (degrees.get(v, 0), v))

    print(f"Start vertex: {start_vertex[:32]}...")
    print(f"Vertices: {len(graph.active_vertex_ids())}")

    ct = CrossComponentTraversal(
        graph, start=start_vertex,
        settlement_threshold=15, seed=42, visit_cap=3,
    )

    # We need to intercept encounter detection to capture (v, w) pairs
    # The encounter has target_a and target_b — we capture them via patching
    fold_pairs = []     # (step, v, w, graph_snapshot_for_fg)
    negate_a_pairs = [] # (step, v, w, graph_snapshot_for_fg)

    max_steps = 2000
    print(f"Running {max_steps} instrumented steps...")
    t0 = time.time()

    for step_num in range(max_steps):
        # Detect encounter before running step
        engine = ct.engine
        if ct._should_jump():
            ct._jump_to_next_component(engine.step + 1)

        # Detect encounter
        enc = engine.detect_encounter()

        # Skip if recently blocked
        if enc.encounter_type != EncounterType.NOTHING and \
           engine._blocked_at.get(engine.position, -99) >= engine.step + 1 - 2:
            enc_type = EncounterType.NOTHING
        else:
            enc_type = enc.encounter_type

        # Capture pairs BEFORE executing (so we have the pre-operation graph)
        if enc_type == EncounterType.FOLD and enc.target_a and enc.target_b:
            fold_pairs.append({
                "step": engine.step + 1,
                "v": enc.target_a,
                "w": enc.target_b,
            })
        elif enc_type == EncounterType.NEGATE_A and enc.target_a and enc.target_b:
            negate_a_pairs.append({
                "step": engine.step + 1,
                "v": enc.target_a,
                "w": enc.target_b,
            })

        # Execute the step normally
        engine.run_step()

        if (step_num + 1) % 500 == 0:
            elapsed = time.time() - t0
            print(f"  Step {step_num + 1}: {elapsed:.1f}s "
                  f"(fold_pairs={len(fold_pairs)}, negate_a_pairs={len(negate_a_pairs)})")

    run_time = time.time() - t0
    print(f"Instrumented run complete in {run_time:.1f}s")
    print(f"Fold pairs captured: {len(fold_pairs)}")
    print(f"Negate_a pairs captured: {len(negate_a_pairs)}")

    # Verify counts match original experiment
    op_dist = Counter()
    for log in ct.engine.logs:
        op_dist[log.operation] += 1
    print(f"Operation distribution: {dict(op_dist)}")

    return ct, fold_pairs, negate_a_pairs


def compute_fg_for_pairs(graph: Graph, pairs: list[dict], label: str, sample_limit: int = 150) -> list[dict]:
    """Compute f(v,w) and g(v,w) for a list of (v,w) pairs."""
    active = set(graph.active_vertex_ids())

    # Filter to pairs where both v and w are in the initial graph
    valid = [p for p in pairs if p["v"] in active and p["w"] in active]
    if len(valid) > sample_limit:
        rng = random.Random(42)
        valid = rng.sample(valid, sample_limit)

    print(f"Computing f,g for {len(valid)} {label} pairs...")
    results = []
    t0 = time.time()
    for i, p in enumerate(valid):
        f_val = compute_f(graph, p["v"], p["w"])
        g_val = compute_g(graph, p["v"], p["w"])
        results.append({
            "step": p["step"],
            "v": p["v"][:48],
            "w": p["w"][:48],
            "f": f_val,
            "g": g_val,
        })
        if (i + 1) % 50 == 0:
            elapsed = time.time() - t0
            print(f"  {i+1}/{len(valid)} done ({elapsed:.1f}s)")

    elapsed = time.time() - t0
    print(f"  Completed {len(results)} pairs in {elapsed:.1f}s")
    return results


def generate_random_pairs(graph: Graph, n: int = 200, seed: int = 99) -> list[dict]:
    """Generate random vertex pairs from the largest connected component."""
    components = _find_components(graph)
    largest = components[0]
    rng = random.Random(seed)

    pairs = []
    for _ in range(n):
        v, w = rng.sample(largest, 2)
        pairs.append({"step": -1, "v": v, "w": w})
    return pairs


def statistical_summary(values: list[float], label: str) -> dict:
    """Compute mean, median, std for a list of values."""
    if not values:
        return {"label": label, "n": 0}
    n = len(values)
    mean = sum(values) / n
    sorted_vals = sorted(values)
    median = sorted_vals[n // 2]
    variance = sum((x - mean) ** 2 for x in values) / max(n - 1, 1)
    std = variance ** 0.5
    return {
        "label": label,
        "n": n,
        "mean": round(mean, 4),
        "median": median,
        "std": round(std, 4),
        "min": min(values),
        "max": max(values),
        "q25": sorted_vals[n // 4],
        "q75": sorted_vals[3 * n // 4],
    }


def mann_whitney_approx(group_a: list[float], group_b: list[float]) -> dict:
    """Simple Mann-Whitney U test (normal approximation for large n)."""
    import math

    combined = [(val, "a") for val in group_a] + [(val, "b") for val in group_b]
    combined.sort(key=lambda x: x[0])

    # Assign ranks (handle ties by averaging)
    ranks = []
    i = 0
    while i < len(combined):
        j = i
        while j < len(combined) and combined[j][0] == combined[i][0]:
            j += 1
        avg_rank = (i + 1 + j) / 2
        for k in range(i, j):
            ranks.append((avg_rank, combined[k][1]))
        i = j

    n_a = len(group_a)
    n_b = len(group_b)
    rank_sum_a = sum(r for r, g in ranks if g == "a")

    u_a = rank_sum_a - n_a * (n_a + 1) / 2
    u_b = n_a * n_b - u_a

    # Normal approximation
    mu = n_a * n_b / 2
    sigma = math.sqrt(n_a * n_b * (n_a + n_b + 1) / 12)
    if sigma == 0:
        return {"U_a": u_a, "U_b": u_b, "z": 0, "p_approx": 1.0}

    z = (min(u_a, u_b) - mu) / sigma

    # Two-sided p-value approximation using error function
    p_approx = 2 * (1 - 0.5 * (1 + math.erf(abs(z) / math.sqrt(2))))

    return {
        "U_a": round(u_a),
        "U_b": round(u_b),
        "z": round(z, 4),
        "p_approx": round(p_approx, 6),
        "n_a": n_a,
        "n_b": n_b,
    }


def effect_size_cohens_d(group_a: list[float], group_b: list[float]) -> float:
    """Cohen's d effect size."""
    if not group_a or not group_b:
        return 0.0
    mean_a = sum(group_a) / len(group_a)
    mean_b = sum(group_b) / len(group_b)
    var_a = sum((x - mean_a) ** 2 for x in group_a) / max(len(group_a) - 1, 1)
    var_b = sum((x - mean_b) ** 2 for x in group_b) / max(len(group_b) - 1, 1)
    pooled_std = ((var_a + var_b) / 2) ** 0.5
    if pooled_std == 0:
        return 0.0
    return round((mean_a - mean_b) / pooled_std, 4)


def main():
    # Step 1: Run instrumented experiment to capture (v, w) pairs
    ct, fold_pairs, negate_a_pairs = run_instrumented()

    # Step 2: Compute f, g on the INITIAL graph (before any operations)
    # This is the correct baseline — f and g are properties of the initial topology
    from genealogy_loader import load_from_repo
    initial_graph, _ = load_from_repo(Path("C:/Users/hanju/NewChanlun"))

    # Filter: only keep fold pairs that were NOT blocked
    # From the logs, identify which fold steps were blocked
    blocked_steps = set()
    for log in ct.engine.logs:
        if log.blocked:
            blocked_steps.add(log.step)

    fold_success = [p for p in fold_pairs if p["step"] not in blocked_steps]
    fold_blocked = [p for p in fold_pairs if p["step"] in blocked_steps]
    print(f"\nFold success: {len(fold_success)}, Fold blocked: {len(fold_blocked)}")
    print(f"Negate_a: {len(negate_a_pairs)}")

    # Step 3: Generate random pairs
    random_pairs = generate_random_pairs(initial_graph, n=200)

    # Step 4: Compute f, g for all three groups
    fold_fg = compute_fg_for_pairs(initial_graph, fold_success, "fold_success")
    negate_fg = compute_fg_for_pairs(initial_graph, negate_a_pairs, "negate_a")
    random_fg = compute_fg_for_pairs(initial_graph, random_pairs, "random")

    # Step 5: Statistical analysis
    fold_f = [r["f"] for r in fold_fg]
    fold_g = [r["g"] for r in fold_fg]
    negate_f = [r["f"] for r in negate_fg]
    negate_g = [r["g"] for r in negate_fg]
    random_f = [r["f"] for r in random_fg]
    random_g = [r["g"] for r in random_fg]

    print("\n" + "=" * 80)
    print("PREDICTION VERIFICATION RESULTS")
    print("=" * 80)

    # Prediction 1: fold f < random f
    print("\n--- Prediction 1: fold f(v,w) < random f(v,w) ---")
    fold_f_stats = statistical_summary(fold_f, "fold_f")
    random_f_stats = statistical_summary(random_f, "random_f")
    print(f"  Fold f:   {fold_f_stats}")
    print(f"  Random f: {random_f_stats}")
    mw_f = mann_whitney_approx(fold_f, random_f)
    d_f = effect_size_cohens_d(fold_f, random_f)
    print(f"  Mann-Whitney: {mw_f}")
    print(f"  Cohen's d: {d_f}")
    pred1_supported = fold_f_stats["mean"] < random_f_stats["mean"] and mw_f["p_approx"] < 0.05
    print(f"  Prediction 1 supported: {pred1_supported}")

    # Prediction 2: negate g > random g
    print("\n--- Prediction 2: negate g(v,w) > random g(v,w) ---")
    negate_g_stats = statistical_summary(negate_g, "negate_g")
    random_g_stats = statistical_summary(random_g, "random_g")
    print(f"  Negate g: {negate_g_stats}")
    print(f"  Random g: {random_g_stats}")
    mw_g = mann_whitney_approx(negate_g, random_g)
    d_g = effect_size_cohens_d(negate_g, random_g)
    print(f"  Mann-Whitney: {mw_g}")
    print(f"  Cohen's d: {d_g}")
    pred2_supported = negate_g_stats["mean"] > random_g_stats["mean"] and mw_g["p_approx"] < 0.05
    print(f"  Prediction 2 supported: {pred2_supported}")

    # Prediction 3: fold and negate occupy different (f, g) regions
    print("\n--- Prediction 3: fold vs negate in (f, g) space ---")
    fold_f_stats2 = statistical_summary(fold_f, "fold_f")
    fold_g_stats2 = statistical_summary(fold_g, "fold_g")
    negate_f_stats2 = statistical_summary(negate_f, "negate_f")
    negate_g_stats2 = statistical_summary(negate_g, "negate_g")
    print(f"  Fold:   f={fold_f_stats2['mean']:.2f}+-{fold_f_stats2['std']:.2f}, "
          f"g={fold_g_stats2['mean']:.2f}+-{fold_g_stats2['std']:.2f}")
    print(f"  Negate: f={negate_f_stats2['mean']:.2f}+-{negate_f_stats2['std']:.2f}, "
          f"g={negate_g_stats2['mean']:.2f}+-{negate_g_stats2['std']:.2f}")

    mw_fg_f = mann_whitney_approx(fold_f, negate_f)
    mw_fg_g = mann_whitney_approx(fold_g, negate_g)
    d_fg_f = effect_size_cohens_d(fold_f, negate_f)
    d_fg_g = effect_size_cohens_d(fold_g, negate_g)
    print(f"  f dimension: Mann-Whitney p={mw_fg_f['p_approx']}, Cohen's d={d_fg_f}")
    print(f"  g dimension: Mann-Whitney p={mw_fg_g['p_approx']}, Cohen's d={d_fg_g}")
    pred3_supported = mw_fg_f["p_approx"] < 0.05 or mw_fg_g["p_approx"] < 0.05
    print(f"  Prediction 3 supported: {pred3_supported}")

    # Step 6: Save results
    result = {
        "experiment": "Q-R3-10: f(v,w) and g(v,w) prediction verification",
        "method": "Replay deterministic 2000-step experiment (seed=42), compute f and g on initial graph",
        "counts": {
            "fold_success_total": len(fold_success),
            "fold_blocked_total": len(fold_blocked),
            "negate_a_total": len(negate_a_pairs),
            "fold_fg_computed": len(fold_fg),
            "negate_fg_computed": len(negate_fg),
            "random_fg_computed": len(random_fg),
        },
        "prediction_1": {
            "hypothesis": "fold mean f(v,w) < random mean f(v,w)",
            "fold_f": fold_f_stats,
            "random_f": random_f_stats,
            "mann_whitney": mw_f,
            "cohens_d": d_f,
            "supported": pred1_supported,
        },
        "prediction_2": {
            "hypothesis": "negate mean g(v,w) > random mean g(v,w)",
            "negate_g": negate_g_stats,
            "random_g": random_g_stats,
            "mann_whitney": mw_g,
            "cohens_d": d_g,
            "supported": pred2_supported,
        },
        "prediction_3": {
            "hypothesis": "fold and negate occupy different (f,g) regions",
            "fold_f_mean": fold_f_stats2["mean"],
            "fold_g_mean": fold_g_stats2["mean"],
            "negate_f_mean": negate_f_stats2["mean"],
            "negate_g_mean": negate_g_stats2["mean"],
            "f_dimension_mann_whitney": mw_fg_f,
            "f_dimension_cohens_d": d_fg_f,
            "g_dimension_mann_whitney": mw_fg_g,
            "g_dimension_cohens_d": d_fg_g,
            "supported": pred3_supported,
        },
        "raw_data": {
            "fold_fg": fold_fg[:20],  # first 20 for inspection
            "negate_fg": negate_fg[:20],
            "random_fg": random_fg[:20],
        },
    }

    out_path = Path("C:/Users/hanju/NewChanlun/tmp/R3-Q10-verification.json")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)
    print(f"\nResults saved to {out_path}")

    return result


if __name__ == "__main__":
    main()
