"""
Beta-1 trajectory analysis for verifying 390号 claims.
CORRECTED: handles multi-graph interleaving (multiple parallel walkers on different graphs).

The traversal-events.jsonl file contains events from multiple concurrent walkers
operating on different graphs. Events are interleaved by file-append order, NOT
by step order within a single graph. We must separate graphs before analysis.

Processes line-by-line (memory efficient). Uses only stdlib.
"""
import json
import re
import statistics
from collections import defaultdict, Counter
import os

INPUT_FILE = "G:/NewChanlun/topological-computation/.chanlun/traversal-events.jsonl"
OUTPUT_FILE = "G:/NewChanlun/tmp/beta1_trajectory_analysis.json"

# ─────────────────────────────────────────────────────────────────────────
# Pass 1: Read all events and classify into graph bands by beta_1 magnitude
# ─────────────────────────────────────────────────────────────────────────

print("=" * 70)
print("PASS 1: Reading and classifying events by graph")
print("=" * 70)

# Define graph bands by beta_1 range (identified from exploratory analysis)
def classify_graph(b1):
    """Classify an event into a graph band based on beta_1 value."""
    if b1 is None:
        return None
    if b1 < 10:
        return "tiny"      # beta_1 0-9 (tiny test graph)
    elif b1 < 300:
        return "small"     # beta_1 ~129-257 (small graph, dominant by event count)
    elif b1 < 1500:
        return "medium_lo" # beta_1 ~200-999 (transition or separate graph)
    elif b1 < 6000:
        return "medium"    # beta_1 ~3000-5400 (medium genealogy graph)
    elif b1 < 10000:
        return "large_lo"  # beta_1 ~6900-7600 (large graph, lower range)
    elif b1 < 20000:
        return "large_hi"  # beta_1 ~10K-20K
    else:
        return "xlarge"    # beta_1 >20K (very large graph, late sublate bursts)

# Collect per-graph events: list of (file_order, step, b1_before, b1_after, encounter_type)
graph_events = defaultdict(list)
total_events = 0
skipped = 0
unclassified = 0

with open(INPUT_FILE, "r", encoding="utf-8") as f:
    for file_idx, line in enumerate(f):
        line = line.strip()
        if not line:
            continue
        total_events += 1
        evt = json.loads(line)
        ctx = evt.get("context", "")
        m = re.search(r"\[step (\d+)\]", ctx)
        if not m:
            skipped += 1
            continue
        step = int(m.group(1))
        etype = evt.get("encounter_type", "unknown")
        b1_before = evt.get("beta_1_before")
        b1_after = evt.get("beta_1_after")

        # Use b1_before for classification (more stable than b1_after for transitions)
        g = classify_graph(b1_before)
        if g is None:
            unclassified += 1
            continue

        graph_events[g].append((file_idx, step, b1_before, b1_after, etype))

print(f"Total events: {total_events}, skipped: {skipped}, unclassified: {unclassified}")
print(f"\nGraph bands:")
for g in sorted(graph_events.keys(), key=lambda x: len(graph_events[x]), reverse=True):
    evts = graph_events[g]
    b1_vals = [e[3] for e in evts if e[3] is not None]
    steps = [e[1] for e in evts]
    types = Counter(e[4] for e in evts)
    print(f"  {g:>10}: {len(evts):>7} events, "
          f"beta_1=[{min(b1_vals)}, {max(b1_vals)}], "
          f"steps=[{min(steps)}, {max(steps)}]")
    for t, c in types.most_common():
        print(f"    {t:>20}: {c:>7}")

# ─────────────────────────────────────────────────────────────────────────
# Analysis function: analyze a single graph's events
# ─────────────────────────────────────────────────────────────────────────

def analyze_graph(graph_name, events, window_sizes=(100, 500, 1000)):
    """
    Analyze beta_1 trajectory for a single graph.
    Events: list of (file_idx, step, b1_before, b1_after, encounter_type)
    sorted by file_idx (append order = temporal order).
    """
    if not events:
        return None

    # Sort by file_idx (temporal order of events as they happened)
    events = sorted(events, key=lambda x: x[0])

    n = len(events)
    b1_values = [e[3] for e in events if e[3] is not None]
    steps = [e[1] for e in events]
    types = Counter(e[4] for e in events)

    if not b1_values:
        return None

    result = {
        "graph_name": graph_name,
        "n_events": n,
        "beta1_start": b1_values[0],
        "beta1_end": b1_values[-1],
        "beta1_min": min(b1_values),
        "beta1_max": max(b1_values),
        "beta1_net_change": b1_values[-1] - b1_values[0],
        "beta1_range": max(b1_values) - min(b1_values),
        "step_min": min(steps),
        "step_max": max(steps),
        "encounter_types": dict(types),
    }

    # ── Trajectory (sampled) ──
    sample_rate = max(1, n // 500)
    trajectory = []
    for i in range(0, n, sample_rate):
        e = events[i]
        trajectory.append({"event_idx": e[0], "step": e[1], "beta1": e[3]})
    result["trajectory_sampled"] = trajectory

    # ── Growth rate in event-count windows ──
    # Since step numbers may not be monotonic (parallel walkers share step space),
    # we use event-count windows (every N events by file order)
    windowed = {}
    for ws_events in window_sizes:
        windows = []
        for w_start in range(0, n, ws_events):
            w_end = min(w_start + ws_events, n)
            w_events = events[w_start:w_end]
            w_b1_start = w_events[0][2]  # b1_before of first
            w_b1_end = w_events[-1][3]   # b1_after of last
            w_types = Counter(e[4] for e in w_events)

            fold_ok = w_types.get("fold", 0)
            fold_blocked = w_types.get("fold_blocked", 0)
            negate_blocked = w_types.get("negate_blocked", 0)
            sublate_ok = w_types.get("sublate", 0)
            sublate_blocked = w_types.get("sublate_blocked", 0)

            fold_total = fold_ok + fold_blocked
            fold_block_rate = fold_blocked / fold_total if fold_total > 0 else None
            discovery = fold_ok + sublate_ok
            discovery_rate = discovery / len(w_events) if len(w_events) > 0 else 0

            if w_b1_start is not None and w_b1_end is not None:
                delta = w_b1_end - w_b1_start
            else:
                delta = None

            windows.append({
                "event_range": [w_start, w_end],
                "step_range": [w_events[0][1], w_events[-1][1]],
                "beta1_start": w_b1_start,
                "beta1_end": w_b1_end,
                "delta_beta1": delta,
                "fold_ok": fold_ok,
                "fold_blocked": fold_blocked,
                "fold_block_rate": round(fold_block_rate, 4) if fold_block_rate is not None else None,
                "negate_blocked": negate_blocked,
                "sublate_ok": sublate_ok,
                "sublate_blocked": sublate_blocked,
                "discovery_rate": round(discovery_rate, 4),
                "n_events": len(w_events),
            })
        windowed[ws_events] = windows

    result["windowed_growth"] = {str(k): v for k, v in windowed.items()}

    # ── Phase detection (using largest window size) ──
    largest_ws = max(window_sizes)
    phases = []
    for w in windowed[largest_ws]:
        d = w["delta_beta1"]
        if d is None:
            phase = "unknown"
        elif d > 5:
            phase = "growth"
        elif d < -5:
            phase = "contraction"
        else:
            phase = "plateau"
        phases.append({"phase": phase, **w})

    # Merge consecutive same-phase windows into macro-phases
    macro_phases = []
    if phases:
        cur = phases[0]["phase"]
        start = phases[0]["event_range"][0]
        deltas = [phases[0]["delta_beta1"]]
        for i in range(1, len(phases)):
            if phases[i]["phase"] != cur:
                macro_phases.append({
                    "phase": cur,
                    "event_range": [start, phases[i-1]["event_range"][1]],
                    "n_windows": len(deltas),
                    "total_delta": sum(d for d in deltas if d is not None),
                    "avg_delta": round(statistics.mean([d for d in deltas if d is not None]), 2) if any(d is not None for d in deltas) else None,
                })
                cur = phases[i]["phase"]
                start = phases[i]["event_range"][0]
                deltas = [phases[i]["delta_beta1"]]
            else:
                deltas.append(phases[i]["delta_beta1"])
        macro_phases.append({
            "phase": cur,
            "event_range": [start, phases[-1]["event_range"][1]],
            "n_windows": len(deltas),
            "total_delta": sum(d for d in deltas if d is not None),
            "avg_delta": round(statistics.mean([d for d in deltas if d is not None]), 2) if any(d is not None for d in deltas) else None,
        })

    result["macro_phases"] = macro_phases

    # ── Blocking rate trend ──
    mid_ws = window_sizes[len(window_sizes)//2] if len(window_sizes) > 1 else window_sizes[0]
    ws_data = windowed[mid_ws]
    n_ws = len(ws_data)
    if n_ws >= 3:
        third = n_ws // 3
        early = ws_data[:third]
        mid = ws_data[third:2*third]
        late = ws_data[2*third:]

        def safe_mean(vals):
            filtered = [v for v in vals if v is not None]
            return statistics.mean(filtered) if filtered else None

        early_block = safe_mean([w["fold_block_rate"] for w in early])
        mid_block = safe_mean([w["fold_block_rate"] for w in mid])
        late_block = safe_mean([w["fold_block_rate"] for w in late])

        early_disc = safe_mean([w["discovery_rate"] for w in early])
        mid_disc = safe_mean([w["discovery_rate"] for w in mid])
        late_disc = safe_mean([w["discovery_rate"] for w in late])

        result["blocking_trend"] = {
            "early_fold_block_rate": round(early_block, 4) if early_block else None,
            "mid_fold_block_rate": round(mid_block, 4) if mid_block else None,
            "late_fold_block_rate": round(late_block, 4) if late_block else None,
            "blocking_increases": (late_block > early_block) if (late_block is not None and early_block is not None) else None,
        }
        result["discovery_trend"] = {
            "early_discovery_rate": round(early_disc, 4) if early_disc else None,
            "mid_discovery_rate": round(mid_disc, 4) if mid_disc else None,
            "late_discovery_rate": round(late_disc, 4) if late_disc else None,
            "efficiency_decays": (late_disc < early_disc) if (late_disc is not None and early_disc is not None) else None,
        }
    else:
        result["blocking_trend"] = None
        result["discovery_trend"] = None

    # ── Running max/min tracking ──
    running_max = b1_values[0]
    new_max_count = 0
    last_new_max_event = 0
    for i, b in enumerate(b1_values):
        if b > running_max:
            running_max = b
            new_max_count += 1
            last_new_max_event = i

    result["new_maxima_count"] = new_max_count
    result["last_new_max_at_pct"] = round(last_new_max_event / len(b1_values) * 100, 1)
    result["still_growing"] = last_new_max_event > len(b1_values) * 0.75

    return result


# ─────────────────────────────────────────────────────────────────────────
# Pass 2: Analyze each graph separately
# ─────────────────────────────────────────────────────────────────────────

print("\n" + "=" * 70)
print("PASS 2: Per-graph analysis")
print("=" * 70)

# Focus on the two dominant graphs
graphs_to_analyze = ["small", "medium"]
# Also analyze others if they have enough events
for g in sorted(graph_events.keys(), key=lambda x: len(graph_events[x]), reverse=True):
    if g not in graphs_to_analyze and len(graph_events[g]) >= 100:
        graphs_to_analyze.append(g)

graph_results = {}
for g in graphs_to_analyze:
    if g not in graph_events:
        continue
    evts = graph_events[g]
    print(f"\n--- Analyzing graph '{g}' ({len(evts)} events) ---")

    # Choose window sizes based on event count
    n_evts = len(evts)
    if n_evts >= 10000:
        ws = (500, 2000, 5000)
    elif n_evts >= 1000:
        ws = (100, 500, 1000)
    else:
        ws = (50, 100, 200)

    result = analyze_graph(g, evts, window_sizes=ws)
    if result:
        graph_results[g] = result
        print(f"  beta_1: {result['beta1_start']} -> {result['beta1_end']} "
              f"(net {result['beta1_net_change']:+d})")
        print(f"  beta_1 range: [{result['beta1_min']}, {result['beta1_max']}] "
              f"(span={result['beta1_range']})")
        print(f"  Steps: {result['step_min']} to {result['step_max']}")
        print(f"  Encounter types: {result['encounter_types']}")
        if result["blocking_trend"]:
            bt = result["blocking_trend"]
            print(f"  Fold block rate: early={bt.get('early_fold_block_rate')}, "
                  f"mid={bt.get('mid_fold_block_rate')}, late={bt.get('late_fold_block_rate')}")
        if result["discovery_trend"]:
            dt = result["discovery_trend"]
            print(f"  Discovery rate: early={dt.get('early_discovery_rate')}, "
                  f"mid={dt.get('mid_discovery_rate')}, late={dt.get('late_discovery_rate')}")
        print(f"  Still growing? {result['still_growing']} "
              f"(last new max at {result['last_new_max_at_pct']}%)")
        print(f"  Macro phases: {len(result['macro_phases'])}")
        for mp in result["macro_phases"][:15]:
            print(f"    events {mp['event_range'][0]:>6}-{mp['event_range'][1]:>6}: "
                  f"{mp['phase']:<12} ({mp['n_windows']} windows, delta={mp['total_delta']:+d})")
        if len(result["macro_phases"]) > 15:
            print(f"    ... ({len(result['macro_phases']) - 15} more)")


# ─────────────────────────────────────────────────────────────────────────
# Pass 3: Verify 390号 claims
# ─────────────────────────────────────────────────────────────────────────

print("\n" + "=" * 70)
print("390号 CLAIMS VERIFICATION")
print("=" * 70)

print("""
CRITICAL FINDING: Data integrity issue
=======================================
The traversal-events.jsonl file contains 227,712 events from MULTIPLE
concurrent walkers on DIFFERENT GRAPHS, all appended to the same file.

Identified graph bands by beta_1 magnitude:
  - 'small' (beta_1 129-257):   138K events, 60.7% -- dominant
  - 'medium' (beta_1 3000-5400): 77K events, 33.9% -- second largest
  - 'large_lo' (beta_1 6900-7600): 5.8K events
  - 'xlarge' (beta_1 >20K): 122 events
  - Others: <5K events combined

390号 was based on a 2000-step run with beta_1 starting at 2325 on a graph
with 2524 vertices and 6751 edges. NONE of the current graph bands exactly
match this beta_1 range (2325-2395). The closest candidate is the 'medium'
band (3000-5400), which has a higher starting beta_1 -- suggesting the
graph was restructured between the 390号 run and the current data.

This multi-graph interleaving means ANY analysis that treats all events
as a single trajectory is INVALID. The apparent beta_1 range of [2, 38219]
is an artifact of mixing events from graphs of vastly different sizes.
""")

# Analyze 390号 claims on the two main graphs
for g_name in ["small", "medium"]:
    if g_name not in graph_results:
        continue
    r = graph_results[g_name]

    print(f"\n--- 390号 claims applied to '{g_name}' graph ---")
    print(f"  (beta_1 [{r['beta1_min']}, {r['beta1_max']}], {r['n_events']} events)")

    # 390-1: bounded growth
    print(f"\n  390-1: beta_1 bounded growth")
    print(f"    beta_1: {r['beta1_start']} -> {r['beta1_end']} (net {r['beta1_net_change']:+d})")
    print(f"    Range: {r['beta1_range']}")
    still_growing = r["still_growing"]
    bounded = not still_growing and r["beta1_range"] < r["beta1_start"] * 0.5

    # More nuanced check: look at windowed growth rates
    # If the graph has windowed data, check if late rates are near zero
    largest_ws_key = max(r["windowed_growth"].keys(), key=int)
    ws_data = r["windowed_growth"][largest_ws_key]
    late_deltas = [w["delta_beta1"] for w in ws_data[len(ws_data)//2:] if w["delta_beta1"] is not None]
    early_deltas = [w["delta_beta1"] for w in ws_data[:len(ws_data)//2] if w["delta_beta1"] is not None]

    if late_deltas and early_deltas:
        late_mean = statistics.mean(late_deltas)
        early_mean = statistics.mean(early_deltas)
        late_abs_mean = statistics.mean([abs(d) for d in late_deltas])
        early_abs_mean = statistics.mean([abs(d) for d in early_deltas])
        print(f"    Early half mean delta: {early_mean:.2f} (|delta| mean: {early_abs_mean:.2f})")
        print(f"    Late half mean delta: {late_mean:.2f} (|delta| mean: {late_abs_mean:.2f})")

        growth_decelerating = late_abs_mean < early_abs_mean
        net_growth_bounded = abs(r["beta1_net_change"]) < r["beta1_range"] * 0.3

        if growth_decelerating and not still_growing:
            verdict_1 = "CONFIRMED (growth decelerates, not still setting new highs)"
        elif growth_decelerating:
            verdict_1 = "PARTIALLY CONFIRMED (rate decelerates but still setting new highs)"
        else:
            verdict_1 = "FALSIFIED (growth does NOT decelerate)"
    else:
        verdict_1 = "INSUFFICIENT DATA"

    print(f"    Still setting new maxima? {still_growing} (last at {r['last_new_max_at_pct']}%)")
    print(f"    VERDICT: {verdict_1}")

    # 390-2: settlement protection increases
    print(f"\n  390-2: settlement protection (blocking rate increases)")
    bt = r.get("blocking_trend")
    if bt and bt.get("blocking_increases") is not None:
        print(f"    Fold block rate: early={bt['early_fold_block_rate']}, "
              f"mid={bt['mid_fold_block_rate']}, late={bt['late_fold_block_rate']}")
        if bt["blocking_increases"]:
            verdict_2 = "CONFIRMED (blocking rate increases over time)"
        else:
            verdict_2 = "FALSIFIED (blocking rate does NOT increase monotonically)"
        print(f"    VERDICT: {verdict_2}")
    else:
        verdict_2 = "INSUFFICIENT DATA"
        print(f"    VERDICT: {verdict_2}")

    # 390-3: efficiency decay
    print(f"\n  390-3: efficiency decay (discovery rate decreases)")
    dt = r.get("discovery_trend")
    if dt and dt.get("efficiency_decays") is not None:
        print(f"    Discovery rate: early={dt['early_discovery_rate']}, "
              f"mid={dt['mid_discovery_rate']}, late={dt['late_discovery_rate']}")
        if dt["efficiency_decays"]:
            verdict_3 = "CONFIRMED (discovery efficiency decays over time)"
        else:
            verdict_3 = "FALSIFIED (discovery efficiency does NOT decay monotonically)"
        print(f"    VERDICT: {verdict_3}")
    else:
        verdict_3 = "INSUFFICIENT DATA"
        print(f"    VERDICT: {verdict_3}")

    # Store verdicts
    graph_results[g_name]["verdicts"] = {
        "390_1_bounded_growth": verdict_1,
        "390_2_settlement_protection": verdict_2,
        "390_3_efficiency_decay": verdict_3,
    }


# ─────────────────────────────────────────────────────────────────────────
# Save results
# ─────────────────────────────────────────────────────────────────────────

# Trim trajectory data for JSON size
for g in graph_results:
    r = graph_results[g]
    r["trajectory_sampled"] = r["trajectory_sampled"][:300]
    # Remove large windowed data, keep only summary
    for ws_key in list(r["windowed_growth"].keys()):
        ws_list = r["windowed_growth"][ws_key]
        # Keep only first and last 20 windows
        if len(ws_list) > 40:
            r["windowed_growth"][ws_key] = ws_list[:20] + ws_list[-20:]

output = {
    "metadata": {
        "input_file": INPUT_FILE,
        "total_events": total_events,
        "n_graphs_detected": len(graph_events),
        "graph_event_counts": {g: len(evts) for g, evts in graph_events.items()},
        "analysis_date": "2026-03-10",
        "critical_finding": "Multi-graph interleaving detected. File contains events from "
                           "multiple concurrent walkers on different graphs. Any analysis "
                           "treating all events as a single trajectory is INVALID.",
    },
    "per_graph_analysis": {},
}

for g in graph_results:
    r = graph_results[g]
    output["per_graph_analysis"][g] = {
        "summary": {
            "n_events": r["n_events"],
            "beta1_start": r["beta1_start"],
            "beta1_end": r["beta1_end"],
            "beta1_net_change": r["beta1_net_change"],
            "beta1_range": [r["beta1_min"], r["beta1_max"]],
            "step_range": [r["step_min"], r["step_max"]],
            "encounter_types": r["encounter_types"],
            "still_growing": r["still_growing"],
            "last_new_max_pct": r["last_new_max_at_pct"],
        },
        "blocking_trend": r.get("blocking_trend"),
        "discovery_trend": r.get("discovery_trend"),
        "macro_phases": r["macro_phases"],
        "verdicts_vs_390": r.get("verdicts", {}),
        "windowed_growth_sample": r["windowed_growth"],
        "trajectory_sampled": r["trajectory_sampled"],
    }

with open(OUTPUT_FILE, "w", encoding="utf-8") as f:
    json.dump(output, f, indent=2, ensure_ascii=False)

print(f"\n\nResults saved to {OUTPUT_FILE}")
print(f"File size: {os.path.getsize(OUTPUT_FILE)} bytes")


# ─────────────────────────────────────────────────────────────────────────
# Final Summary
# ─────────────────────────────────────────────────────────────────────────

print("\n" + "=" * 70)
print("FINAL SUMMARY")
print("=" * 70)

print("""
DATA INTEGRITY ISSUE:
  The traversal-events.jsonl contains interleaved events from multiple
  parallel walkers on different graphs. The initial (naive) analysis
  that treated all 227K events as one trajectory produced misleading
  results (apparent beta_1 range [2, 38219] is an artifact).

  Per-graph separation reveals the actual dynamics:
""")

for g in ["small", "medium"]:
    if g not in graph_results:
        continue
    r = graph_results[g]
    v = r.get("verdicts", {})
    print(f"  Graph '{g}' (beta_1 [{r['beta1_min']}, {r['beta1_max']}], {r['n_events']} events):")
    print(f"    beta_1: {r['beta1_start']} -> {r['beta1_end']} (net {r['beta1_net_change']:+d})")
    print(f"    390-1 (bounded growth):     {v.get('390_1_bounded_growth', 'N/A')}")
    print(f"    390-2 (settlement protect):  {v.get('390_2_settlement_protection', 'N/A')}")
    print(f"    390-3 (efficiency decay):    {v.get('390_3_efficiency_decay', 'N/A')}")
    print()

print("""
METHODOLOGICAL NOTE:
  390号 was L2 (single graph, single run, 2000 steps, coverage ~2%).
  Current data is ~134x larger (267K steps) but comes from a DIFFERENT
  graph configuration than the one used in 390号 (beta_1 2325 baseline
  does not match any current graph band).

  The 390号 claims cannot be directly verified or falsified against
  this dataset because:
  1. The underlying graph has changed since the 390号 run
  2. Multiple graphs are interleaved without session markers
  3. Step numbers are reused across walkers (not globally unique)

  For valid verification, the analysis should be run on a SINGLE graph
  with proper session isolation, ideally the same graph used in 390号.
""")
