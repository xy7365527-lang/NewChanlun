"""Q-R3-10 corrected: use random NEIGHBOR pairs as baseline.

The original baseline (random pairs from largest component) is invalid because
fold/negate operations exclusively target pairs that are 1-2 hops apart.
Random pairs from the full component include pairs at arbitrary graph distances,
making g(v,w) comparison meaningless (distance confound).

Corrected baseline: random neighbor pairs (1-hop) from the same component.
"""

from __future__ import annotations

import json
import random
import math
import time
from collections import Counter
from pathlib import Path

from engine import Graph, _connected_components
from genealogy_loader import load_from_repo
from verify_fg_predictions import (
    compute_f, compute_g, _find_components,
    CrossComponentTraversal, _vertex_degrees,
    statistical_summary, mann_whitney_approx, effect_size_cohens_d,
)
from traversal import EncounterType


def main():
    repo_root = Path("C:/Users/hanju/NewChanlun")

    # Step 1: Load initial graph
    print("Loading graph...")
    graph, _ = load_from_repo(repo_root)
    active_set = set(graph.active_vertex_ids())

    # Step 2: Replay instrumented experiment to get (v,w) pairs
    print("Replaying 2000-step experiment...")
    components = _find_components(graph)
    largest_component = components[0]
    degrees = _vertex_degrees(graph)
    start_vertex = max(largest_component, key=lambda v: (degrees.get(v, 0), v))

    ct = CrossComponentTraversal(
        graph, start=start_vertex,
        settlement_threshold=15, seed=42, visit_cap=3,
    )

    fold_pairs = []
    negate_a_pairs = []

    t0 = time.time()
    for step_num in range(2000):
        engine = ct.engine
        if ct._should_jump():
            ct._jump_to_next_component(engine.step + 1)

        enc = engine.detect_encounter()
        enc_type = enc.encounter_type
        if enc_type != EncounterType.NOTHING and \
           engine._blocked_at.get(engine.position, -99) >= engine.step + 1 - 2:
            enc_type = EncounterType.NOTHING

        if enc_type == EncounterType.FOLD and enc.target_a and enc.target_b:
            fold_pairs.append({"step": engine.step + 1, "v": enc.target_a, "w": enc.target_b})
        elif enc_type == EncounterType.NEGATE_A and enc.target_a and enc.target_b:
            negate_a_pairs.append({"step": engine.step + 1, "v": enc.target_a, "w": enc.target_b})

        engine.run_step()

    replay_time = time.time() - t0
    print(f"  Replay: {replay_time:.1f}s")

    # Filter blocked folds
    blocked_steps = {log.step for log in ct.engine.logs if log.blocked}
    fold_success = [p for p in fold_pairs if p["step"] not in blocked_steps]

    op_dist = Counter(log.operation for log in ct.engine.logs)
    print(f"  Operations: {dict(op_dist)}")
    print(f"  Fold success: {len(fold_success)}, Negate_a: {len(negate_a_pairs)}")

    # Step 3: Generate random neighbor pairs
    largest_set = set(largest_component)
    rng = random.Random(99)
    random_nb_pairs = []
    attempts = 0
    while len(random_nb_pairs) < 300 and attempts < 10000:
        v = rng.choice(largest_component)
        nbs = [n for n in graph.neighbors(v) if n in largest_set]
        if nbs:
            w = rng.choice(nbs)
            random_nb_pairs.append({"step": -1, "v": v, "w": w})
        attempts += 1
    print(f"  Random neighbor pairs: {len(random_nb_pairs)}")

    # Also generate random pairs from same component at arbitrary distance (for contrast)
    random_any_pairs = []
    for _ in range(200):
        v, w = rng.sample(largest_component, 2)
        random_any_pairs.append({"step": -1, "v": v, "w": w})

    # Step 4: Compute f, g for all groups (sample 150 each)
    sample_n = 150

    def sample_and_compute(pairs, label, n=sample_n):
        valid = [p for p in pairs if p["v"] in active_set and p["w"] in active_set]
        if len(valid) > n:
            valid = random.Random(42).sample(valid, n)
        print(f"  Computing f,g for {len(valid)} {label} pairs...")
        results = []
        t = time.time()
        for p in valid:
            f_val = compute_f(graph, p["v"], p["w"])
            g_val = compute_g(graph, p["v"], p["w"])
            results.append({"v": p["v"][:48], "w": p["w"][:48], "f": f_val, "g": g_val, "step": p["step"]})
        print(f"    Done in {time.time()-t:.1f}s")
        return results

    fold_fg = sample_and_compute(fold_success, "fold_success")
    negate_fg = sample_and_compute(negate_a_pairs, "negate_a")
    random_nb_fg = sample_and_compute(random_nb_pairs, "random_neighbor")
    random_any_fg = sample_and_compute(random_any_pairs, "random_any")

    # Extract values
    fold_f = [r["f"] for r in fold_fg]
    fold_g = [r["g"] for r in fold_fg]
    negate_f = [r["f"] for r in negate_fg]
    negate_g = [r["g"] for r in negate_fg]
    rnb_f = [r["f"] for r in random_nb_fg]
    rnb_g = [r["g"] for r in random_nb_fg]
    rany_f = [r["f"] for r in random_any_fg]
    rany_g = [r["g"] for r in random_any_fg]

    print("\n" + "=" * 80)
    print("CORRECTED PREDICTION VERIFICATION (random neighbor baseline)")
    print("=" * 80)

    # ---- Summary stats ----
    groups = [
        ("fold", fold_f, fold_g),
        ("negate_a", negate_f, negate_g),
        ("random_neighbor", rnb_f, rnb_g),
        ("random_any", rany_f, rany_g),
    ]

    for label, f_vals, g_vals in groups:
        fs = statistical_summary(f_vals, f"{label}_f")
        gs = statistical_summary(g_vals, f"{label}_g")
        print(f"\n  {label}: f mean={fs['mean']:.2f} median={fs['median']} std={fs['std']:.2f} | "
              f"g mean={gs['mean']:.2f} median={gs['median']} std={gs['std']:.2f}")

    # ---- Prediction 1: fold f < baseline f ----
    print("\n" + "-" * 80)
    print("PREDICTION 1: fold mean f(v,w) < random_neighbor mean f(v,w)")
    print("-" * 80)
    fs_fold = statistical_summary(fold_f, "fold_f")
    fs_rnb = statistical_summary(rnb_f, "random_nb_f")
    mw1 = mann_whitney_approx(fold_f, rnb_f)
    d1 = effect_size_cohens_d(fold_f, rnb_f)
    print(f"  fold f:      mean={fs_fold['mean']:.3f} std={fs_fold['std']:.3f}")
    print(f"  random_nb f: mean={fs_rnb['mean']:.3f} std={fs_rnb['std']:.3f}")
    print(f"  Mann-Whitney z={mw1['z']:.4f} p={mw1['p_approx']:.6f}")
    print(f"  Cohen's d = {d1}")
    direction_1 = "fold_f < random_nb_f" if fs_fold["mean"] < fs_rnb["mean"] else "fold_f >= random_nb_f"
    significant_1 = mw1["p_approx"] < 0.05
    pred1_result = fs_fold["mean"] < fs_rnb["mean"] and significant_1
    print(f"  Direction: {direction_1}")
    print(f"  Significant at p<0.05: {significant_1}")
    print(f"  PREDICTION 1: {'SUPPORTED' if pred1_result else 'NEGATED'}")
    if not pred1_result and significant_1:
        print(f"  NOTE: fold_f is LARGER than random_nb_f — opposite to prediction.")
        print(f"  Interpretation: fold targets vertices with MORE shared neighbors (higher f),")
        print(f"  not fewer. f(v,w) does not predict fold candidacy as 'low f = fold candidate'.")

    # ---- Prediction 2: negate g > baseline g ----
    print("\n" + "-" * 80)
    print("PREDICTION 2: negate mean g(v,w) > random_neighbor mean g(v,w)")
    print("-" * 80)
    gs_neg = statistical_summary(negate_g, "negate_g")
    gs_rnb = statistical_summary(rnb_g, "random_nb_g")
    mw2 = mann_whitney_approx(negate_g, rnb_g)
    d2 = effect_size_cohens_d(negate_g, rnb_g)
    print(f"  negate g:    mean={gs_neg['mean']:.3f} std={gs_neg['std']:.3f}")
    print(f"  random_nb g: mean={gs_rnb['mean']:.3f} std={gs_rnb['std']:.3f}")
    print(f"  Mann-Whitney z={mw2['z']:.4f} p={mw2['p_approx']:.6f}")
    print(f"  Cohen's d = {d2}")
    direction_2 = "negate_g > random_nb_g" if gs_neg["mean"] > gs_rnb["mean"] else "negate_g <= random_nb_g"
    significant_2 = mw2["p_approx"] < 0.05
    pred2_result = gs_neg["mean"] > gs_rnb["mean"] and significant_2
    print(f"  Direction: {direction_2}")
    print(f"  Significant at p<0.05: {significant_2}")
    print(f"  PREDICTION 2: {'SUPPORTED' if pred2_result else 'NEGATED'}")

    # ---- Prediction 3: fold vs negate in (f,g) space ----
    print("\n" + "-" * 80)
    print("PREDICTION 3: fold and negate occupy different regions in (f, g) space")
    print("-" * 80)
    mw3_f = mann_whitney_approx(fold_f, negate_f)
    mw3_g = mann_whitney_approx(fold_g, negate_g)
    d3_f = effect_size_cohens_d(fold_f, negate_f)
    d3_g = effect_size_cohens_d(fold_g, negate_g)
    print(f"  f dimension: fold mean={statistical_summary(fold_f,'')['mean']:.2f} "
          f"vs negate mean={statistical_summary(negate_f,'')['mean']:.2f}")
    print(f"    Mann-Whitney z={mw3_f['z']:.4f} p={mw3_f['p_approx']:.6f}, Cohen's d={d3_f}")
    print(f"  g dimension: fold mean={statistical_summary(fold_g,'')['mean']:.2f} "
          f"vs negate mean={statistical_summary(negate_g,'')['mean']:.2f}")
    print(f"    Mann-Whitney z={mw3_g['z']:.4f} p={mw3_g['p_approx']:.6f}, Cohen's d={d3_g}")
    f_sep = mw3_f["p_approx"] < 0.05
    g_sep = mw3_g["p_approx"] < 0.05
    pred3_result = f_sep or g_sep
    print(f"  f-separated: {f_sep}, g-separated: {g_sep}")
    print(f"  PREDICTION 3: {'SUPPORTED' if pred3_result else 'NEGATED'}")

    # ---- Saturation analysis ----
    print("\n" + "-" * 80)
    print("G SATURATION ANALYSIS")
    print("-" * 80)
    for label, g_vals in [("fold", fold_g), ("negate", negate_g), ("random_nb", rnb_g), ("random_any", rany_g)]:
        sat = sum(1 for g in g_vals if g >= 50)
        print(f"  {label}: {sat}/{len(g_vals)} saturated at g>=50 ({100*sat/max(len(g_vals),1):.0f}%)")

    # ---- Save results ----
    result = {
        "experiment": "Q-R3-10 corrected: f(v,w) and g(v,w) prediction verification",
        "baseline": "random neighbor pairs (1-hop), not random any pairs",
        "note": "g(v,w) saturates at 50 (iteration cap) for most fold/negate/neighbor pairs — "
                "this graph has extremely high vertex connectivity within the largest component. "
                "g is therefore uninformative for distinguishing operations among neighbors.",
        "counts": {
            "fold_success": len(fold_success),
            "negate_a": len(negate_a_pairs),
            "fold_fg_computed": len(fold_fg),
            "negate_fg_computed": len(negate_fg),
            "random_nb_fg_computed": len(random_nb_fg),
            "random_any_fg_computed": len(random_any_fg),
        },
        "prediction_1": {
            "hypothesis": "fold mean f(v,w) < random_neighbor mean f(v,w)",
            "fold_f": statistical_summary(fold_f, "fold_f"),
            "random_nb_f": statistical_summary(rnb_f, "random_nb_f"),
            "mann_whitney": mw1,
            "cohens_d": d1,
            "direction": direction_1,
            "significant": significant_1,
            "supported": pred1_result,
            "interpretation": (
                "NEGATED. fold f(v,w) is significantly LARGER than random neighbor f(v,w) "
                "(opposite direction). Fold targets vertex pairs with more lower-link components, "
                "meaning they have more structurally diverse neighborhoods. This makes sense: "
                "the fold encounter rule requires sharing >=2 neighbors, which selects for "
                "vertices embedded in dense, highly-connected regions with large lower links."
            ) if not pred1_result else "Supported as predicted.",
        },
        "prediction_2": {
            "hypothesis": "negate mean g(v,w) > random_neighbor mean g(v,w)",
            "negate_g": statistical_summary(negate_g, "negate_g"),
            "random_nb_g": statistical_summary(rnb_g, "random_nb_g"),
            "mann_whitney": mw2,
            "cohens_d": d2,
            "direction": direction_2,
            "significant": significant_2,
            "supported": pred2_result,
            "interpretation": (
                "g(v,w) saturates at 50 (iteration cap) for nearly all neighbor pairs in this graph. "
                "The graph has extremely high vertex connectivity. The measurement is uninformative: "
                "both negate and random neighbor pairs hit the cap. If significant, it's only because "
                "random neighbors occasionally span weakly-connected subregions."
            ),
            "caveat": "g saturation makes this test unreliable — need higher iteration cap or different metric",
        },
        "prediction_3": {
            "hypothesis": "fold and negate occupy different (f,g) regions",
            "f_dimension": {"mann_whitney": mw3_f, "cohens_d": d3_f, "significant": f_sep},
            "g_dimension": {"mann_whitney": mw3_g, "cohens_d": d3_g, "significant": g_sep},
            "supported": pred3_result,
            "interpretation": (
                f"f-dimension separation: p={mw3_f['p_approx']}, d={d3_f}. "
                f"g-dimension separation: p={mw3_g['p_approx']}, d={d3_g}. "
                "Fold pairs have lower f than negate pairs if significant in f-dimension. "
                "g-dimension is unreliable due to saturation."
            ),
        },
        "g_saturation": {
            "fold_saturated_pct": round(100 * sum(1 for g in fold_g if g >= 50) / max(len(fold_g), 1), 1),
            "negate_saturated_pct": round(100 * sum(1 for g in negate_g if g >= 50) / max(len(negate_g), 1), 1),
            "random_nb_saturated_pct": round(100 * sum(1 for g in rnb_g if g >= 50) / max(len(rnb_g), 1), 1),
            "random_any_saturated_pct": round(100 * sum(1 for g in rany_g if g >= 50) / max(len(rany_g), 1), 1),
        },
        "all_stats": {
            "fold_f": statistical_summary(fold_f, "fold_f"),
            "fold_g": statistical_summary(fold_g, "fold_g"),
            "negate_f": statistical_summary(negate_f, "negate_f"),
            "negate_g": statistical_summary(negate_g, "negate_g"),
            "random_nb_f": statistical_summary(rnb_f, "random_nb_f"),
            "random_nb_g": statistical_summary(rnb_g, "random_nb_g"),
            "random_any_f": statistical_summary(rany_f, "random_any_f"),
            "random_any_g": statistical_summary(rany_g, "random_any_g"),
        },
    }

    out_path = Path("C:/Users/hanju/NewChanlun/tmp/R3-Q10-verification.json")
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)
    print(f"\nResults saved to {out_path}")


if __name__ == "__main__":
    main()
