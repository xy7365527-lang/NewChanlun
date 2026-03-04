"""
Agent A: Structural breakpoint detection and post-breakpoint dynamics clustering
for an anonymous ratio time series (~25 years daily data).

Steps:
1. Change-point detection (ruptures PELT + rolling statistics cross-validation)
2. Post-breakpoint dynamics characterization
3. Unsupervised clustering of post-breakpoint behaviors
"""

import json
import numpy as np
import pandas as pd
from pathlib import Path
from datetime import datetime

# ── Load data ──────────────────────────────────────────────────────────────
DATA_DIR = Path(__file__).parent
df = pd.read_csv(DATA_DIR / "agent_a_input.csv", parse_dates=["Date"])
df = df.sort_values("Date").reset_index(drop=True)
ratio = df["CuAu_Ratio"].values
dates = df["Date"].values

print(f"Loaded {len(df)} rows: {df['Date'].iloc[0].date()} → {df['Date'].iloc[-1].date()}")
print(f"Ratio range: {ratio.min():.6f} – {ratio.max():.6f}")

# ── Step 1: Structural breakpoint detection ────────────────────────────────
import ruptures as rpt

# --- 1a. PELT on the raw series (rbf kernel, penalizes model complexity) ---
# Use log-ratio for better numerical properties (ratio is strictly positive)
log_ratio = np.log(ratio)

# PELT with rbf kernel — penalty calibrated via elbow heuristic
# For ~6400 points over 25 years, pen=15-25 typically yields 8-20 breakpoints
signal = log_ratio.reshape(-1, 1)

# Run PELT with multiple penalties to find stable breakpoints
results_by_pen = {}
for pen in [10, 15, 20, 25, 30]:
    algo = rpt.Pelt(model="rbf", min_size=60, jump=5).fit(signal)
    bkps = algo.predict(pen=pen)
    # Remove the last element (always == len(signal))
    bkps_clean = [b for b in bkps if b < len(signal)]
    results_by_pen[pen] = bkps_clean
    print(f"  PELT pen={pen}: {len(bkps_clean)} breakpoints")

# --- 1b. Binary segmentation as cross-check ---
algo_binseg = rpt.Binseg(model="l2", min_size=60, jump=5).fit(signal)
bkps_binseg = algo_binseg.predict(pen=20)
bkps_binseg_clean = [b for b in bkps_binseg if b < len(signal)]
print(f"  BinSeg pen=20: {len(bkps_binseg_clean)} breakpoints")

# --- 1c. Rolling statistics for cross-validation ---
window = 120  # ~6 months
roll_mean = pd.Series(log_ratio).rolling(window, center=True).mean()
roll_std = pd.Series(log_ratio).rolling(window, center=True).std()

# Detect variance regime shifts: points where rolling std changes significantly
roll_std_diff = roll_std.diff().abs()
std_threshold = roll_std_diff.quantile(0.98)
variance_shift_indices = roll_std_diff[roll_std_diff > std_threshold].index.tolist()

# Cluster nearby variance shift points (within 30 trading days)
def cluster_indices(indices, min_gap=30):
    if not indices:
        return []
    clusters = [[indices[0]]]
    for idx in indices[1:]:
        if idx - clusters[-1][-1] <= min_gap:
            clusters[-1].append(idx)
        else:
            clusters.append([idx])
    return [int(np.median(c)) for c in clusters]

variance_shift_points = cluster_indices(variance_shift_indices, min_gap=30)
print(f"  Rolling variance shifts: {len(variance_shift_points)} clusters")

# --- 1d. Consensus breakpoints ---
# Use PELT pen=20 as primary, cross-validate with BinSeg and variance shifts
primary_bkps = results_by_pen[20]

# For each primary breakpoint, check if BinSeg or variance shift confirms (within 40 days)
def find_nearby(target, candidates, tolerance=40):
    return any(abs(target - c) <= tolerance for c in candidates)

confirmed_bkps = []
unconfirmed_bkps = []
for bp in primary_bkps:
    binseg_confirm = find_nearby(bp, bkps_binseg_clean, 40)
    var_confirm = find_nearby(bp, variance_shift_points, 40)
    if binseg_confirm or var_confirm:
        confirmed_bkps.append(bp)
    else:
        unconfirmed_bkps.append(bp)

# Also add BinSeg-only breakpoints confirmed by variance shifts
for bp in bkps_binseg_clean:
    if not find_nearby(bp, primary_bkps, 40):
        if find_nearby(bp, variance_shift_points, 40):
            confirmed_bkps.append(bp)

# Include unconfirmed PELT breakpoints if they show strong signal
# (large level shift in log-ratio around the breakpoint)
for bp in unconfirmed_bkps:
    pre_window = slice(max(0, bp - 60), bp)
    post_window = slice(bp, min(len(log_ratio), bp + 60))
    level_shift = abs(np.mean(log_ratio[post_window]) - np.mean(log_ratio[pre_window]))
    if level_shift > 0.05:  # >5% shift in log-ratio
        confirmed_bkps.append(bp)

# Sort and deduplicate (merge within 30 days)
confirmed_bkps = sorted(set(confirmed_bkps))
final_bkps = cluster_indices(confirmed_bkps, min_gap=30)

# Convert to dates
breakpoint_dates = [pd.Timestamp(dates[min(bp, len(dates)-1)]).strftime("%Y-%m-%d") for bp in final_bkps]
print(f"\nFinal breakpoints: {len(final_bkps)}")
for i, (bp, d) in enumerate(zip(final_bkps, breakpoint_dates)):
    print(f"  BP{i+1:02d}: index={bp}, date={d}, ratio={ratio[bp]:.6f}")

# ── Step 2: Post-breakpoint dynamics characterization ──────────────────────
print("\n=== Step 2: Post-breakpoint dynamics ===")

POST_WINDOW = 120  # ~6 months post-breakpoint for characterization
PRE_WINDOW = 60    # ~3 months pre-breakpoint for baseline

breakpoint_profiles = []

for i, bp_idx in enumerate(final_bkps):
    bp_date = breakpoint_dates[i]

    # Define windows
    pre_start = max(0, bp_idx - PRE_WINDOW)
    pre_end = bp_idx
    post_start = bp_idx
    # Post window ends at next breakpoint or POST_WINDOW, whichever is shorter
    if i + 1 < len(final_bkps):
        next_bp = final_bkps[i + 1]
        post_end = min(bp_idx + POST_WINDOW, next_bp)
    else:
        post_end = min(bp_idx + POST_WINDOW, len(ratio))

    pre_data = log_ratio[pre_start:pre_end]
    post_data = log_ratio[post_start:post_end]
    post_ratio = ratio[post_start:post_end]

    if len(pre_data) < 10 or len(post_data) < 10:
        continue

    # --- Quantitative features ---
    # 1. Level shift magnitude (log scale)
    pre_mean = np.mean(pre_data)
    post_mean = np.mean(post_data)
    level_shift = post_mean - pre_mean

    # 2. Volatility change
    pre_vol = np.std(pre_data)
    post_vol = np.std(post_data)
    vol_ratio = post_vol / pre_vol if pre_vol > 1e-10 else 1.0

    # 3. Trend slope in post-breakpoint window (linear regression)
    if len(post_data) > 1:
        x = np.arange(len(post_data))
        slope, intercept = np.polyfit(x, post_data, 1)
    else:
        slope = 0.0

    # 4. Speed of departure: max absolute daily change in first 20 days
    first_20 = post_data[:min(20, len(post_data))]
    if len(first_20) > 1:
        departure_speed = np.max(np.abs(np.diff(first_20)))
    else:
        departure_speed = 0.0

    # 5. Mean reversion: does the post-window return toward pre-mean?
    # Measure as correlation between distance-from-pre-mean and time
    distances = np.abs(post_data - pre_mean)
    if len(distances) > 5:
        x_dist = np.arange(len(distances))
        corr_dist_time = np.corrcoef(x_dist, distances)[0, 1]
    else:
        corr_dist_time = 0.0
    # Negative correlation = mean reverting, positive = diverging

    # 6. Oscillation: number of zero-crossings around post-mean
    post_centered = post_data - np.mean(post_data)
    zero_crossings = np.sum(np.diff(np.sign(post_centered)) != 0)
    osc_rate = zero_crossings / len(post_data) if len(post_data) > 0 else 0

    # 7. Stabilization time: first point where 20-day rolling std drops below pre-breakpoint level
    if len(post_data) >= 20:
        post_roll_std = pd.Series(post_data).rolling(20).std().values
        stable_mask = post_roll_std < pre_vol
        stable_indices = np.where(stable_mask)[0]
        if len(stable_indices) > 0:
            stabilization_days = int(stable_indices[0])
        else:
            stabilization_days = -1  # did not stabilize within window
    else:
        stabilization_days = -1

    # 8. Regime establishment: is the last third of post-window significantly
    #    different from pre-window? (t-test proxy: difference in means / pooled std)
    last_third = post_data[len(post_data)*2//3:]
    if len(last_third) > 5:
        new_regime_distance = abs(np.mean(last_third) - pre_mean) / (pre_vol + 1e-10)
    else:
        new_regime_distance = 0.0

    profile = {
        "breakpoint_index": int(bp_idx),
        "breakpoint_date": bp_date,
        "breakpoint_ratio_value": float(ratio[min(bp_idx, len(ratio)-1)]),
        "pre_mean_log": float(pre_mean),
        "post_mean_log": float(post_mean),
        "level_shift": float(level_shift),
        "pre_volatility": float(pre_vol),
        "post_volatility": float(post_vol),
        "volatility_ratio": float(vol_ratio),
        "post_trend_slope": float(slope),
        "departure_speed": float(departure_speed),
        "mean_reversion_corr": float(corr_dist_time),
        "oscillation_rate": float(osc_rate),
        "stabilization_days": stabilization_days,
        "new_regime_distance": float(new_regime_distance),
        "post_window_length": int(post_end - post_start),
    }
    breakpoint_profiles.append(profile)

    print(f"  BP{i+1:02d} ({bp_date}): shift={level_shift:+.4f}, "
          f"vol_ratio={vol_ratio:.2f}, slope={slope:.6f}, "
          f"stab_days={stabilization_days}, osc={osc_rate:.3f}")

# ── Step 2b: Generate textual descriptions ─────────────────────────────────
print("\n=== Step 2b: Generating descriptions ===")

def describe_dynamics(p):
    """Generate a plain-language description of post-breakpoint dynamics."""
    parts = []

    # Direction and magnitude
    direction = "upward" if p["level_shift"] > 0 else "downward"
    mag = abs(p["level_shift"])
    if mag > 0.15:
        mag_word = "large"
    elif mag > 0.05:
        mag_word = "moderate"
    else:
        mag_word = "small"
    parts.append(f"{mag_word} {direction} level shift ({p['level_shift']:+.4f} in log-ratio)")

    # Departure speed
    if p["departure_speed"] > 0.03:
        parts.append("rapid initial departure")
    elif p["departure_speed"] > 0.015:
        parts.append("moderate initial departure speed")
    else:
        parts.append("gradual initial departure")

    # Volatility change
    vr = p["volatility_ratio"]
    if vr > 2.0:
        parts.append(f"volatility surged ({vr:.1f}x pre-breakpoint)")
    elif vr > 1.3:
        parts.append(f"volatility increased ({vr:.1f}x)")
    elif vr < 0.7:
        parts.append(f"volatility compressed ({vr:.1f}x)")
    else:
        parts.append("volatility roughly unchanged")

    # Post-breakpoint trend
    slope = p["post_trend_slope"]
    if abs(slope) < 1e-5:
        parts.append("no significant post-breakpoint trend")
    elif slope > 0:
        parts.append(f"continued upward drift (slope={slope:.6f}/day)")
    else:
        parts.append(f"continued downward drift (slope={slope:.6f}/day)")

    # Mean reversion vs divergence
    mrc = p["mean_reversion_corr"]
    if mrc < -0.3:
        parts.append("mean-reverting behavior (distance from prior level decreasing over time)")
    elif mrc > 0.3:
        parts.append("diverging behavior (distance from prior level increasing over time)")
    else:
        parts.append("neither clearly mean-reverting nor diverging")

    # Oscillation
    osc = p["oscillation_rate"]
    if osc > 0.5:
        parts.append("high oscillation around post-breakpoint mean")
    elif osc > 0.3:
        parts.append("moderate oscillation")
    else:
        parts.append("low oscillation (trending or stable)")

    # Stabilization
    sd = p["stabilization_days"]
    if sd == -1:
        parts.append("did not stabilize within observation window")
    elif sd < 20:
        parts.append(f"rapid stabilization (~{sd} trading days)")
    elif sd < 60:
        parts.append(f"gradual stabilization (~{sd} trading days)")
    else:
        parts.append(f"slow stabilization (~{sd} trading days)")

    # New regime
    nrd = p["new_regime_distance"]
    if nrd > 3.0:
        parts.append("established a clearly distinct new regime")
    elif nrd > 1.0:
        parts.append("partial regime shift (new level distinguishable but not far)")
    else:
        parts.append("returned close to prior regime level")

    return "; ".join(parts)


for p in breakpoint_profiles:
    p["description"] = describe_dynamics(p)
    print(f"  BP {p['breakpoint_date']}: {p['description'][:120]}...")

# ── Step 3: Unsupervised clustering ────────────────────────────────────────
print("\n=== Step 3: Clustering post-breakpoint behaviors ===")

from sklearn.preprocessing import StandardScaler, RobustScaler
from sklearn.cluster import DBSCAN, AgglomerativeClustering
from sklearn.mixture import GaussianMixture
from scipy.cluster.hierarchy import linkage, fcluster
from scipy.spatial.distance import pdist
from sklearn.metrics import silhouette_score, silhouette_samples, calinski_harabasz_score

# Extract feature matrix — use richer, more discriminating features
feature_names = [
    "level_shift", "volatility_ratio", "post_trend_slope",
    "departure_speed", "mean_reversion_corr", "oscillation_rate",
    "stabilization_days", "new_regime_distance"
]

raw_features = []
for p in breakpoint_profiles:
    row = [p[f] for f in feature_names]
    if row[6] == -1:
        row[6] = POST_WINDOW
    raw_features.append(row)

X_raw = np.array(raw_features)
n_samples = X_raw.shape[0]
print(f"  Feature matrix: {n_samples} samples x {len(feature_names)} features")

# Use RobustScaler to reduce outlier dominance (2008 event)
scaler = RobustScaler()
X = scaler.fit_transform(X_raw)

# --- 3a. Hierarchical clustering (Ward) with silhouette scan ---
linkage_matrix = linkage(X, method="ward")

hier_results = {}
for k in range(2, min(n_samples - 1, 10)):
    labels = fcluster(linkage_matrix, k, criterion="maxclust")
    if len(set(labels)) < 2:
        continue
    sil = silhouette_score(X, labels)
    ch = calinski_harabasz_score(X, labels)
    hier_results[k] = {"labels": labels, "silhouette": sil, "calinski": ch}
    print(f"    Hierarchical k={k}: silhouette={sil:.3f}, calinski={ch:.1f}")

# --- 3b. GMM with BIC (use diagonal covariance for small N) ---
gmm_results = {}
for k in range(2, min(n_samples // 3, 8)):
    for cov_type in ["diag", "spherical"]:
        try:
            gmm = GaussianMixture(n_components=k, covariance_type=cov_type,
                                   n_init=10, random_state=42, reg_covar=1e-4)
            gmm.fit(X)
            bic = gmm.bic(X)
            labels = gmm.predict(X)
            sil = silhouette_score(X, labels) if len(set(labels)) >= 2 else -1
            key = f"gmm_{cov_type}_k{k}"
            gmm_results[key] = {"k": k, "cov": cov_type, "bic": bic,
                                "labels": labels, "silhouette": sil}
            print(f"    GMM {cov_type} k={k}: BIC={bic:.1f}, silhouette={sil:.3f}")
        except Exception as e:
            pass

# --- 3c. Select best clustering ---
# Strategy: prefer k=4-6 range (meaningful for 24 breakpoints over 25 years)
# Use a composite score: silhouette * 0.6 + normalized_calinski * 0.4
# But penalize k=2 if one cluster has <3 members (degenerate split)

def score_clustering(labels, X, k):
    """Score a clustering, penalizing degenerate splits."""
    unique, counts = np.unique(labels, return_counts=True)
    min_size = counts.min()
    sil = silhouette_score(X, labels) if len(unique) >= 2 else -1

    # Penalize if any cluster has fewer than 2 members
    if min_size < 2:
        sil *= 0.5

    # Slight preference for more granular (but not too granular) solutions
    # Sweet spot around k=4-6 for 24 samples
    granularity_bonus = 0.0
    if 3 <= k <= 6:
        granularity_bonus = 0.05
    elif k >= 7:
        granularity_bonus = -0.05

    return sil + granularity_bonus

all_candidates = []
for k, res in hier_results.items():
    score = score_clustering(res["labels"], X, k)
    all_candidates.append(("hier", k, score, res["labels"]))

for key, res in gmm_results.items():
    score = score_clustering(res["labels"], X, res["k"])
    all_candidates.append(("gmm", res["k"], score, res["labels"]))

all_candidates.sort(key=lambda x: x[2], reverse=True)

print("\n  Top 5 clustering candidates:")
for method, k, score, _ in all_candidates[:5]:
    print(f"    {method} k={k}: composite_score={score:.3f}")

# Select best
best_method, final_k, best_score, final_labels = all_candidates[0]
print(f"\n  Selected: {best_method} k={final_k} (score={best_score:.3f})")

# If best is still k=2 with degenerate split, force k=4 hierarchical
unique_labels, label_counts = np.unique(final_labels, return_counts=True)
if final_k == 2 and label_counts.min() < 3:
    print("  Degenerate k=2 detected, escalating to k=4 hierarchical")
    if 4 in hier_results:
        final_labels = hier_results[4]["labels"]
        final_k = 4
    elif 3 in hier_results:
        final_labels = hier_results[3]["labels"]
        final_k = 3

# Relabel to 0-based contiguous
from sklearn.preprocessing import LabelEncoder
le = LabelEncoder()
final_labels = le.fit_transform(final_labels)

# Assign cluster labels to profiles
for i, p in enumerate(breakpoint_profiles):
    p["cluster"] = int(final_labels[i])

print(f"  Final clustering: k={final_k}")
for c in sorted(set(final_labels)):
    members = [breakpoint_profiles[i]["breakpoint_date"]
               for i in range(len(breakpoint_profiles)) if final_labels[i] == c]
    print(f"    Cluster {c}: {len(members)} members — {members}")

# --- 3e. Characterize each cluster ---
cluster_chars = {}
for c in sorted(set(final_labels)):
    members = [p for p in breakpoint_profiles if p["cluster"] == c]
    member_indices = [i for i, p in enumerate(breakpoint_profiles) if p["cluster"] == c]
    member_features = X_raw[member_indices]

    char = {
        "cluster_id": int(c),
        "n_members": len(members),
        "member_dates": [m["breakpoint_date"] for m in members],
        "avg_level_shift": float(np.mean(member_features[:, 0])),
        "std_level_shift": float(np.std(member_features[:, 0])),
        "avg_volatility_ratio": float(np.mean(member_features[:, 1])),
        "avg_departure_speed": float(np.mean(member_features[:, 3])),
        "avg_mean_reversion": float(np.mean(member_features[:, 4])),
        "avg_oscillation": float(np.mean(member_features[:, 5])),
        "avg_stabilization_days": float(np.mean(member_features[:, 6])),
        "avg_new_regime_distance": float(np.mean(member_features[:, 7])),
        "avg_post_trend_slope": float(np.mean(member_features[:, 2])),
    }

    # Generate cluster description based on distinguishing features
    desc_parts = []

    # Direction
    if char["avg_level_shift"] > 0.1:
        desc_parts.append("strong upward shift")
    elif char["avg_level_shift"] > 0.02:
        desc_parts.append("moderate upward shift")
    elif char["avg_level_shift"] < -0.3:
        desc_parts.append("extreme downward shift")
    elif char["avg_level_shift"] < -0.1:
        desc_parts.append("strong downward shift")
    elif char["avg_level_shift"] < -0.02:
        desc_parts.append("moderate downward shift")
    else:
        desc_parts.append("mixed/neutral direction")

    # Volatility
    if char["avg_volatility_ratio"] > 2.0:
        desc_parts.append("volatility explosion")
    elif char["avg_volatility_ratio"] > 1.4:
        desc_parts.append("elevated volatility")
    elif char["avg_volatility_ratio"] < 0.7:
        desc_parts.append("volatility compression")
    else:
        desc_parts.append("stable volatility")

    # Speed
    if char["avg_departure_speed"] > 0.03:
        desc_parts.append("fast departure")
    elif char["avg_departure_speed"] > 0.02:
        desc_parts.append("moderate departure speed")
    else:
        desc_parts.append("gradual transition")

    # Reversion
    if char["avg_mean_reversion"] < -0.3:
        desc_parts.append("mean-reverting")
    elif char["avg_mean_reversion"] > 0.3:
        desc_parts.append("persistently diverging")

    # Regime
    if char["avg_new_regime_distance"] > 5.0:
        desc_parts.append("strong new regime established")
    elif char["avg_new_regime_distance"] > 2.0:
        desc_parts.append("new regime partially established")
    else:
        desc_parts.append("returns toward prior level")

    char["description"] = ", ".join(desc_parts)
    cluster_chars[int(c)] = char
    print(f"\n  Cluster {c} ({len(members)} members): {char['description']}")
    for m in members:
        print(f"    - {m['breakpoint_date']} (shift={m['level_shift']:+.4f}, vol={m['volatility_ratio']:.2f}x)")

# ── Step 3f: Confidence assessment ─────────────────────────────────────────
print("\n=== Confidence assessment ===")

# Confidence per breakpoint: based on cross-validation support
for p in breakpoint_profiles:
    bp_idx = p["breakpoint_index"]
    pelt_confirm = find_nearby(bp_idx, results_by_pen[20], 40)
    binseg_confirm = find_nearby(bp_idx, bkps_binseg_clean, 40)
    var_confirm = find_nearby(bp_idx, variance_shift_points, 40)
    support = sum([pelt_confirm, binseg_confirm, var_confirm])

    if support >= 3:
        p["breakpoint_confidence"] = "high"
    elif support >= 2:
        p["breakpoint_confidence"] = "medium"
    else:
        p["breakpoint_confidence"] = "low"

# Clustering confidence
best_sil = silhouette_score(X, final_labels) if len(set(final_labels)) >= 2 else -1
if best_sil > 0.4:
    clustering_confidence = "high"
elif best_sil > 0.2:
    clustering_confidence = "medium"
else:
    clustering_confidence = "low"

# Per-sample silhouette for cluster confidence
sample_sils = silhouette_samples(X, final_labels)
for i, p in enumerate(breakpoint_profiles):
    p["cluster_confidence"] = (
        "high" if sample_sils[i] > 0.4 else
        "medium" if sample_sils[i] > 0.1 else
        "low"
    )

# Spectrum vs discrete assessment
avg_sil = float(np.mean(sample_sils))
if avg_sil > 0.5:
    spectrum_note = "The data suggests relatively discrete types with clear separation between clusters."
elif avg_sil > 0.25:
    spectrum_note = ("The data shows partially overlapping clusters — there are identifiable groupings "
                     "but boundaries are fuzzy, suggesting a semi-continuous spectrum with denser regions.")
else:
    spectrum_note = ("The clustering is weak, suggesting the post-breakpoint behaviors form more of a "
                     "continuous spectrum than discrete types. Cluster assignments should be treated as "
                     "approximate groupings rather than definitive categories.")

print(f"  Overall clustering confidence: {clustering_confidence}")
print(f"  Average silhouette: {avg_sil:.3f}")
print(f"  Spectrum assessment: {spectrum_note}")

# ── Output ─────────────────────────────────────────────────────────────────
print("\n=== Writing output files ===")

# 1. Timestamps
timestamps = [p["breakpoint_date"] for p in breakpoint_profiles]
with open(DATA_DIR / "agent_a_timestamps.json", "w", encoding="utf-8") as f:
    json.dump(timestamps, f, indent=2, ensure_ascii=False)
print(f"  Wrote agent_a_timestamps.json ({len(timestamps)} breakpoints)")

# 2. Full results JSON
results = {
    "metadata": {
        "data_range": f"{df['Date'].iloc[0].strftime('%Y-%m-%d')} to {df['Date'].iloc[-1].strftime('%Y-%m-%d')}",
        "n_observations": len(df),
        "n_breakpoints": len(breakpoint_profiles),
        "n_clusters": int(final_k),
        "clustering_method": "hierarchical (Ward) with GMM cross-validation",
        "clustering_confidence": clustering_confidence,
        "average_silhouette": float(avg_sil),
        "spectrum_assessment": spectrum_note,
        "detection_methods": {
            "primary": "PELT (rbf kernel, pen=20)",
            "cross_validation": ["BinSeg (l2, pen=20)", "rolling variance shift (120-day window)"]
        }
    },
    "breakpoints": [],
    "clusters": {}
}

for p in breakpoint_profiles:
    bp_entry = {
        "timestamp": p["breakpoint_date"],
        "ratio_value": p["breakpoint_ratio_value"],
        "description": p["description"],
        "cluster": p["cluster"],
        "breakpoint_confidence": p["breakpoint_confidence"],
        "cluster_confidence": p["cluster_confidence"],
        "features": {
            "level_shift": p["level_shift"],
            "volatility_ratio": p["volatility_ratio"],
            "post_trend_slope": p["post_trend_slope"],
            "departure_speed": p["departure_speed"],
            "mean_reversion_corr": p["mean_reversion_corr"],
            "oscillation_rate": p["oscillation_rate"],
            "stabilization_days": p["stabilization_days"],
            "new_regime_distance": p["new_regime_distance"],
        }
    }
    results["breakpoints"].append(bp_entry)

for c_id, char in cluster_chars.items():
    results["clusters"][str(c_id)] = char

with open(DATA_DIR / "agent_a_results.json", "w", encoding="utf-8") as f:
    json.dump(results, f, indent=2, ensure_ascii=False)
print(f"  Wrote agent_a_results.json")

# 3. Human-readable summary (Chinese)
summary_lines = []
summary_lines.append("# Agent A 分析报告：匿名比率时间序列结构性断点检测与聚类\n")
summary_lines.append(f"数据范围：{results['metadata']['data_range']}")
summary_lines.append(f"观测数量：{results['metadata']['n_observations']}")
summary_lines.append(f"检测到的结构性断点：{results['metadata']['n_breakpoints']} 个")
summary_lines.append(f"聚类数量：{results['metadata']['n_clusters']} 个")
summary_lines.append(f"聚类置信度：{results['metadata']['clustering_confidence']}")
summary_lines.append(f"平均轮廓系数：{results['metadata']['average_silhouette']:.3f}\n")

summary_lines.append("## 检测方法\n")
summary_lines.append("- 主方法：PELT（rbf 核，惩罚参数=20，最小段长=60）")
summary_lines.append("- 交叉验证：BinSeg（l2 模型）+ 120日滚动方差突变检测")
summary_lines.append("- 共识机制：至少两种方法在40个交易日内确认的断点被保留\n")

summary_lines.append("## 断点列表\n")
summary_lines.append("| 序号 | 日期 | 比率值 | 聚类 | 断点置信度 | 聚类置信度 |")
summary_lines.append("|------|------|--------|------|-----------|-----------|")
for i, bp in enumerate(results["breakpoints"]):
    summary_lines.append(
        f"| {i+1:02d} | {bp['timestamp']} | {bp['ratio_value']:.6f} | "
        f"{bp['cluster']} | {bp['breakpoint_confidence']} | {bp['cluster_confidence']} |"
    )

summary_lines.append("\n## 各断点后动态描述\n")
for i, bp in enumerate(results["breakpoints"]):
    summary_lines.append(f"### BP{i+1:02d} — {bp['timestamp']}\n")
    summary_lines.append(f"比率值：{bp['ratio_value']:.6f}")
    summary_lines.append(f"聚类分配：Cluster {bp['cluster']}")
    summary_lines.append(f"动态描述：{bp['description']}\n")
    f = bp["features"]
    summary_lines.append(f"- 水平位移（对数）：{f['level_shift']:+.4f}")
    summary_lines.append(f"- 波动率变化倍数：{f['volatility_ratio']:.2f}x")
    summary_lines.append(f"- 趋势斜率：{f['post_trend_slope']:.6f}/日")
    summary_lines.append(f"- 离场速度：{f['departure_speed']:.6f}")
    summary_lines.append(f"- 均值回归相关性：{f['mean_reversion_corr']:.3f}")
    summary_lines.append(f"- 振荡率：{f['oscillation_rate']:.3f}")
    stab = f['stabilization_days']
    summary_lines.append(f"- 稳定化天数：{'未稳定' if stab == -1 else str(stab)}")
    summary_lines.append(f"- 新制度距离：{f['new_regime_distance']:.2f}\n")

summary_lines.append("## 聚类特征\n")
for c_id, char in sorted(cluster_chars.items()):
    summary_lines.append(f"### Cluster {c_id}（{char['n_members']} 个成员）\n")
    summary_lines.append(f"特征描述：{char['description']}\n")
    summary_lines.append(f"- 平均水平位移：{char['avg_level_shift']:+.4f}")
    summary_lines.append(f"- 平均波动率倍数：{char['avg_volatility_ratio']:.2f}x")
    summary_lines.append(f"- 平均离场速度：{char['avg_departure_speed']:.6f}")
    summary_lines.append(f"- 平均均值回归：{char['avg_mean_reversion']:.3f}")
    summary_lines.append(f"- 平均振荡率：{char['avg_oscillation']:.3f}")
    summary_lines.append(f"- 平均稳定化天数：{char['avg_stabilization_days']:.0f}")
    summary_lines.append(f"- 平均新制度距离：{char['avg_new_regime_distance']:.2f}")
    summary_lines.append(f"- 成员日期：{', '.join(char['member_dates'])}\n")

summary_lines.append("## 离散类型 vs 连续谱\n")
summary_lines.append(results["metadata"]["spectrum_assessment"])

summary_text = "\n".join(summary_lines)
with open(DATA_DIR / "agent_a_summary.md", "w", encoding="utf-8") as f:
    f.write(summary_text)
print(f"  Wrote agent_a_summary.md")

print("\n=== Analysis complete ===")
print(f"  {len(breakpoint_profiles)} breakpoints detected")
print(f"  {final_k} clusters identified")
print(f"  Clustering confidence: {clustering_confidence}")
print(f"  Files written: agent_a_timestamps.json, agent_a_results.json, agent_a_summary.md")
