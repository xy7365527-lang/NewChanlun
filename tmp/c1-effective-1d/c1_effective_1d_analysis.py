"""
C1: 从相关矩阵数学结构推导"有效一维"条件
=============================================
目标：
1. 数学推导 eff.dim 与特征值/PC1 explained variance 的精确关系
2. 定义"有效一维"判定条件（PC1 > 0.9 或 0.95）
3. 计算对应的 eff.dim 临界值
4. 在四个节点上验证该临界值的 percentile 位置是否收敛到 ~1%

认识论等级：L0（数学推导）+ L3（四节点交叉验证）
"""

import json
import warnings
from pathlib import Path

import numpy as np
import pandas as pd
import yfinance as yf
from scipy import stats
from scipy.optimize import brentq

warnings.filterwarnings("ignore")

OUTPUT_DIR = Path(__file__).parent


# ================================================================
# Part 1: Pure Mathematics (L0)
# ================================================================
def eff_dim_from_eigenvalues(eigvals):
    """Compute effective dimensionality from eigenvalues.

    eff.dim = exp(H), where H = -sum(p_i * log(p_i)),
    p_i = lambda_i / sum(lambda_j).

    For a correlation matrix: sum(lambda_i) = d (dimensionality).
    For d=3: lambda_1 + lambda_2 + lambda_3 = 3.
    """
    eigvals = np.array(eigvals, dtype=float)
    eigvals = eigvals[eigvals > 0]
    p = eigvals / eigvals.sum()
    entropy = -np.sum(p * np.log(p))
    return np.exp(entropy)


def pc1_explained_variance(lambda1, d=3):
    """PC1 explained variance ratio = lambda_1 / d."""
    return lambda1 / d


def eff_dim_from_pc1_evr(evr, d=3):
    """Given PC1 explained variance ratio, compute the MINIMUM eff.dim.

    When PC1 explains fraction `evr` of total variance, the remaining
    (1-evr) is split among d-1 components. The minimum eff.dim (maximum
    concentration) occurs when remaining variance is EQUALLY split.

    lambda_1 = evr * d
    lambda_2 = ... = lambda_d = (1-evr)*d/(d-1)

    This gives the minimum eff.dim for that evr level.
    """
    lambda1 = evr * d
    lambda_rest = (1 - evr) * d / (d - 1)
    eigvals = [lambda1] + [lambda_rest] * (d - 1)
    return eff_dim_from_eigenvalues(eigvals)


def eff_dim_from_pc1_evr_max(evr, d=3):
    """Given PC1 explained variance ratio, compute the MAXIMUM eff.dim.

    Maximum eff.dim for given PC1 occurs when remaining variance is
    maximally UNEQUAL: one component gets all remaining, others get epsilon.

    lambda_1 = evr * d
    lambda_2 = (1-evr)*d
    lambda_3 = ... = lambda_d = epsilon

    In the limit epsilon->0, this reduces to a 2D system.
    """
    lambda1 = evr * d
    lambda2 = (1 - evr) * d
    # Use small epsilon for remaining components
    eps = 1e-10
    eigvals = [lambda1, lambda2] + [eps] * (d - 2)
    return eff_dim_from_eigenvalues(eigvals)


# --- Key theoretical values ---
print("=" * 70)
print("PART 1: PURE MATHEMATICS (L0)")
print("=" * 70)

print("\n--- Boundary values for 3x3 correlation matrix ---")
print(f"Complete 1D (lambda=[3,0,0]):  eff.dim = {eff_dim_from_eigenvalues([3, 0, 0]):.6f}")
print(f"Complete 3D (lambda=[1,1,1]):  eff.dim = {eff_dim_from_eigenvalues([1, 1, 1]):.6f}")

print("\n--- eff.dim as function of PC1 explained variance (minimum, equal-split residual) ---")
evr_targets = [0.80, 0.85, 0.90, 0.91, 0.92, 0.93, 0.94, 0.95, 0.96, 0.97, 0.98, 0.99]
print(f"{'PC1_EVR':>10} {'lambda_1':>10} {'lambda_rest':>12} {'eff.dim_min':>12} {'eff.dim_max':>12}")
theory_table = []
for evr in evr_targets:
    ed_min = eff_dim_from_pc1_evr(evr, d=3)
    ed_max = eff_dim_from_pc1_evr_max(evr, d=3)
    l1 = evr * 3
    l_rest = (1 - evr) * 3 / 2
    theory_table.append({
        "pc1_evr": evr,
        "lambda_1": round(l1, 4),
        "lambda_rest": round(l_rest, 4),
        "eff_dim_min": round(ed_min, 6),
        "eff_dim_max": round(ed_max, 6),
    })
    print(f"{evr:>10.2f} {l1:>10.4f} {l_rest:>12.4f} {ed_min:>12.6f} {ed_max:>12.6f}")


# --- Inverse: given eff.dim target, find PC1 EVR ---
print("\n--- Inverse mapping: eff.dim -> PC1 EVR (equal-split residual) ---")
def find_pc1_evr_for_effdim(target_effdim, d=3):
    """Find PC1 explained variance ratio that gives target eff.dim (min case)."""
    def objective(evr):
        return eff_dim_from_pc1_evr(evr, d) - target_effdim
    # Search between 1/d (uniform) and 1.0 (degenerate)
    try:
        return brentq(objective, 1/d + 1e-10, 1.0 - 1e-10)
    except ValueError:
        return None


effdim_targets = [1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.0]
print(f"{'eff.dim':>10} {'PC1_EVR':>10} {'lambda_1':>10}")
inverse_table = []
for ed_target in effdim_targets:
    evr = find_pc1_evr_for_effdim(ed_target)
    if evr is not None:
        inverse_table.append({
            "eff_dim": ed_target,
            "pc1_evr": round(evr, 6),
            "lambda_1": round(evr * 3, 6),
        })
        print(f"{ed_target:>10.1f} {evr:>10.6f} {evr*3:>10.6f}")


# ================================================================
# Part 2: "Effective 1D" Definition (L0)
# ================================================================
print("\n" + "=" * 70)
print("PART 2: 'EFFECTIVE 1D' DEFINITION (L0)")
print("=" * 70)

# Definition: "effective 1D" = PC1 explains >= 95% of total variance
# This means lambda_1 >= 2.85 (for d=3), lambda_2 + lambda_3 <= 0.15
EFF_1D_THRESHOLD_95 = 0.95
EFF_1D_THRESHOLD_90 = 0.90

ed_at_95 = eff_dim_from_pc1_evr(EFF_1D_THRESHOLD_95, d=3)
ed_at_90 = eff_dim_from_pc1_evr(EFF_1D_THRESHOLD_90, d=3)

print(f"\n'Effective 1D' at PC1 >= 95%:")
print(f"  lambda_1 >= {EFF_1D_THRESHOLD_95 * 3:.2f}")
print(f"  eff.dim <= {ed_at_95:.6f} (equal-split residual)")
print(f"  Interpretation: one factor explains 95%+ of variance;")
print(f"    three edges are 95%+ collinear; only one effective role")

print(f"\n'Effective 1D' at PC1 >= 90%:")
print(f"  lambda_1 >= {EFF_1D_THRESHOLD_90 * 3:.2f}")
print(f"  eff.dim <= {ed_at_90:.6f} (equal-split residual)")
print(f"  Interpretation: one factor explains 90%+ of variance")

# More relaxed: what if eff.dim at event floor?
print("\n--- Empirical event floor from relative_threshold_results ---")
event_floors = {
    "Au": 1.534,
    "Dollar": 1.493,
    "Equity": 1.684,
    "Commodity": 1.146,
}
for node, floor in event_floors.items():
    evr = find_pc1_evr_for_effdim(floor)
    print(f"  {node:>10}: min_event_effdim = {floor:.3f} -> PC1 EVR = {evr:.4f} ({evr*100:.1f}%)")


# ================================================================
# Part 3: Data Verification (L3)
# ================================================================
print("\n" + "=" * 70)
print("PART 3: DATA VERIFICATION (L3)")
print("=" * 70)

# Download data
print("\nDownloading data...")
def _dl(ticker, start="2000-01-01", end="2026-03-03"):
    raw = yf.download(ticker, start=start, end=end)["Close"]
    if isinstance(raw, pd.DataFrame):
        raw = raw.iloc[:, 0]
    return raw


gc = _dl("GC=F")
dxy = _dl("DX-Y.NYB")
spx = _dl("^GSPC")
cl = _dl("CL=F")

df_all = pd.DataFrame({"gc": gc, "dxy": dxy, "spx": spx, "cl": cl}).dropna()
neg_mask = df_all["cl"] <= 0
if neg_mask.any():
    df_all = df_all[~neg_mask]
print(f"Common data: {len(df_all)} obs, {df_all.index[0].date()} to {df_all.index[-1].date()}")

NODE_DEFS = {
    "Au": [
        ("gc_dxy", lambda d: d["gc"] / d["dxy"]),
        ("gc_spx", lambda d: d["gc"] / d["spx"]),
        ("gc_cl",  lambda d: d["gc"] / d["cl"]),
    ],
    "Dollar": [
        ("dxy_gc",  lambda d: d["dxy"] / d["gc"]),
        ("dxy_spx", lambda d: d["dxy"] / d["spx"]),
        ("dxy_cl",  lambda d: d["dxy"] / d["cl"]),
    ],
    "Equity": [
        ("spx_dxy", lambda d: d["spx"] / d["dxy"]),
        ("spx_gc",  lambda d: d["spx"] / d["gc"]),
        ("spx_cl",  lambda d: d["spx"] / d["cl"]),
    ],
    "Commodity": [
        ("cl_dxy", lambda d: d["cl"] / d["dxy"]),
        ("cl_gc",  lambda d: d["cl"] / d["gc"]),
        ("cl_spx", lambda d: d["cl"] / d["spx"]),
    ],
}

WINDOW = 252


def compute_rolling_effdim_and_eigenvalues(node_name, ratio_defs, df):
    """Compute rolling eff.dim AND eigenvalue time series for a node."""
    ratio_series = {}
    for rname, rfunc in ratio_defs:
        ratio_series[rname] = rfunc(df)
    ratios_df = pd.DataFrame(ratio_series)
    returns = ratios_df.apply(np.log).diff().dropna()

    eff_dim_vals = []
    pc1_evr_vals = []
    lambda1_vals = []
    dates = []

    for i in range(WINDOW, len(returns)):
        w = returns.iloc[i - WINDOW : i]
        # Use correlation matrix (not covariance) for comparable eigenvalues
        corr = w.corr().values
        eigvals = np.sort(np.linalg.eigvalsh(corr))[::-1]  # descending
        eigvals_pos = eigvals[eigvals > 0]

        p = eigvals_pos / eigvals_pos.sum()
        entropy = -np.sum(p * np.log(p))
        ed = np.exp(entropy)

        eff_dim_vals.append(ed)
        pc1_evr_vals.append(eigvals[0] / eigvals.sum())
        lambda1_vals.append(eigvals[0])
        dates.append(returns.index[i])

    return pd.DataFrame({
        "eff_dim": eff_dim_vals,
        "pc1_evr": pc1_evr_vals,
        "lambda_1": lambda1_vals,
    }, index=dates)


# Compute for all nodes
node_data = {}
for name, rdefs in NODE_DEFS.items():
    df_node = compute_rolling_effdim_and_eigenvalues(name, rdefs, df_all)
    node_data[name] = df_node
    print(f"\n  {name}: {len(df_node)} pts")
    print(f"    eff.dim: mean={df_node['eff_dim'].mean():.3f}, std={df_node['eff_dim'].std():.3f}, "
          f"min={df_node['eff_dim'].min():.3f}, max={df_node['eff_dim'].max():.3f}")
    print(f"    PC1 EVR: mean={df_node['pc1_evr'].mean():.4f}, max={df_node['pc1_evr'].max():.4f}")
    print(f"    lambda_1: mean={df_node['lambda_1'].mean():.4f}, max={df_node['lambda_1'].max():.4f}")


# ================================================================
# Part 3a: Verify eff.dim from covariance vs correlation
# ================================================================
print("\n--- NOTE on covariance vs correlation ---")
print("Prior experiments used COVARIANCE matrix eigenvalues.")
print("For 'effective 1D' analysis, CORRELATION matrix is canonical")
print("because eigenvalue sum = d (dimension), making PC1 EVR directly interpretable.")
print("We compute both and compare.")

# Recompute using covariance (as in prior experiments)
def compute_rolling_effdim_cov(node_name, ratio_defs, df):
    """Using covariance matrix (as in prior experiments)."""
    ratio_series = {}
    for rname, rfunc in ratio_defs:
        ratio_series[rname] = rfunc(df)
    ratios_df = pd.DataFrame(ratio_series)
    returns = ratios_df.apply(np.log).diff().dropna()

    eff_dim_vals = []
    dates = []
    for i in range(WINDOW, len(returns)):
        w = returns.iloc[i - WINDOW : i]
        cov = w.cov().values
        eigvals = np.linalg.eigvalsh(cov)
        eigvals_pos = eigvals[eigvals > 0]
        p = eigvals_pos / eigvals_pos.sum()
        entropy = -np.sum(p * np.log(p))
        eff_dim_vals.append(np.exp(entropy))
        dates.append(returns.index[i])
    return pd.Series(eff_dim_vals, index=dates)


node_data_cov = {}
for name, rdefs in NODE_DEFS.items():
    ed_cov = compute_rolling_effdim_cov(name, rdefs, df_all)
    node_data_cov[name] = ed_cov

# Compare: they should be almost identical since eff.dim is scale-invariant
print("\nCorrelation between cov-based and corr-based eff.dim:")
for name in NODE_DEFS:
    ed_corr = node_data[name]["eff_dim"]
    ed_cov = node_data_cov[name]
    common_idx = ed_corr.index.intersection(ed_cov.index)
    r = np.corrcoef(ed_corr[common_idx].values, ed_cov[common_idx].values)[0, 1]
    max_diff = np.abs(ed_corr[common_idx].values - ed_cov[common_idx].values).max()
    print(f"  {name}: corr={r:.8f}, max_abs_diff={max_diff:.6f}")


# ================================================================
# Part 3b: "Effective 1D" threshold verification
# ================================================================
print("\n" + "=" * 70)
print("PART 3b: 'EFFECTIVE 1D' THRESHOLD VERIFICATION")
print("=" * 70)

# For each PC1 EVR threshold, compute corresponding eff.dim critical value
# and check what percentile it falls at in each node's distribution
thresholds = [0.90, 0.91, 0.92, 0.93, 0.94, 0.95, 0.96, 0.97, 0.98, 0.99]

results_by_threshold = {}
for thresh in thresholds:
    ed_crit_equal_split = eff_dim_from_pc1_evr(thresh, d=3)

    node_percentiles = {}
    for name in NODE_DEFS:
        ed_series = node_data[name]["eff_dim"]
        # What fraction of days have eff.dim <= ed_crit?
        frac_below = (ed_series <= ed_crit_equal_split).mean()
        # Equivalent percentile
        percentile = frac_below * 100
        node_percentiles[name] = {
            "frac_below": round(float(frac_below), 6),
            "percentile": round(float(percentile), 4),
        }

    results_by_threshold[str(thresh)] = {
        "pc1_evr_threshold": thresh,
        "eff_dim_critical": round(ed_crit_equal_split, 6),
        "node_percentiles": node_percentiles,
    }

    print(f"\nPC1 EVR >= {thresh*100:.0f}% -> eff.dim <= {ed_crit_equal_split:.4f}")
    for name, vals in node_percentiles.items():
        print(f"  {name:>10}: {vals['percentile']:>8.4f}% of days below threshold")


# ================================================================
# Part 3c: Reverse approach — what PC1 EVR corresponds to each
#           node's event floor percentile (~0.7%)?
# ================================================================
print("\n" + "=" * 70)
print("PART 3c: REVERSE — PERCENTILE-MATCHED PC1 EVR")
print("=" * 70)

target_percentiles = [0.5, 0.6, 0.7, 0.77, 0.81, 1.0, 1.5, 2.0]

reverse_results = {}
for pct in target_percentiles:
    node_vals = {}
    for name in NODE_DEFS:
        ed_series = node_data[name]["eff_dim"]
        ed_at_pct = np.percentile(ed_series, pct)
        evr = find_pc1_evr_for_effdim(ed_at_pct)
        node_vals[name] = {
            "eff_dim_at_pct": round(float(ed_at_pct), 4),
            "pc1_evr": round(float(evr), 6) if evr else None,
            "pc1_pct": round(float(evr * 100), 2) if evr else None,
        }

    reverse_results[str(pct)] = {
        "target_percentile": pct,
        "nodes": node_vals,
    }

    print(f"\nAt {pct}th percentile of eff.dim distribution:")
    for name, vals in node_vals.items():
        if vals["pc1_evr"]:
            print(f"  {name:>10}: eff.dim = {vals['eff_dim_at_pct']:.4f}, "
                  f"PC1 EVR = {vals['pc1_pct']:.2f}%")


# ================================================================
# Part 3d: Direct verification — at what percentile does eff.dim
#           drop below 1.5 (strong 1D) for each node?
# ================================================================
print("\n" + "=" * 70)
print("PART 3d: DIRECT — EMPIRICAL PERCENTILE AT eff.dim THRESHOLDS")
print("=" * 70)

effdim_thresholds = [1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8]

direct_results = {}
for ed_thresh in effdim_thresholds:
    evr_at_thresh = find_pc1_evr_for_effdim(ed_thresh)
    node_pcts = {}
    for name in NODE_DEFS:
        ed_series = node_data[name]["eff_dim"]
        frac = (ed_series <= ed_thresh).mean()
        pct = stats.percentileofscore(ed_series, ed_thresh)
        node_pcts[name] = {
            "frac_below": round(float(frac), 6),
            "percentile": round(float(pct), 4),
        }

    direct_results[str(ed_thresh)] = {
        "eff_dim_threshold": ed_thresh,
        "pc1_evr_at_threshold": round(float(evr_at_thresh), 6) if evr_at_thresh else None,
        "node_percentiles": node_pcts,
    }

    print(f"\neff.dim <= {ed_thresh:.1f} (PC1 >= {evr_at_thresh*100:.1f}%):")
    pcts = [v["percentile"] for v in node_pcts.values()]
    spread = max(pcts) - min(pcts) if pcts else 0
    for name, vals in node_pcts.items():
        print(f"  {name:>10}: percentile = {vals['percentile']:>8.4f}%")
    print(f"  Spread: {spread:.4f} pp")


# ================================================================
# Part 3e: Find the eff.dim threshold where all four nodes
#           have the SAME percentile (~1%)
# ================================================================
print("\n" + "=" * 70)
print("PART 3e: FIND CONVERGENCE POINT")
print("=" * 70)

def percentile_spread_at_effdim(ed_thresh):
    """Return the max pairwise difference in percentile at a given eff.dim threshold."""
    pcts = []
    for name in NODE_DEFS:
        ed_series = node_data[name]["eff_dim"]
        pct = stats.percentileofscore(ed_series, ed_thresh)
        pcts.append(pct)
    return max(pcts) - min(pcts), pcts


# Scan a range
scan_range = np.arange(1.0, 2.5, 0.01)
convergence_scan = []
for ed_thresh in scan_range:
    spread, pcts = percentile_spread_at_effdim(ed_thresh)
    convergence_scan.append({
        "eff_dim": round(float(ed_thresh), 2),
        "spread_pp": round(float(spread), 4),
        "pcts": {name: round(float(p), 4) for name, p in zip(NODE_DEFS.keys(), pcts)},
    })

# The key insight from prior experiments: convergence happens NOT at a
# fixed eff.dim threshold, but at each node's OWN distribution floor.
# The convergence is in PERCENTILE SPACE, not in eff.dim space.
print("\nConvergence scan (eff.dim -> spread in percentile):")
print(f"{'eff.dim':>8} {'spread_pp':>10} {'Au':>8} {'Dollar':>8} {'Equity':>8} {'Commodity':>8}")
for row in convergence_scan:
    if row["spread_pp"] < 10:  # Only show reasonably converged values
        p = row["pcts"]
        print(f"{row['eff_dim']:>8.2f} {row['spread_pp']:>10.4f} "
              f"{p['Au']:>8.4f} {p['Dollar']:>8.4f} {p['Equity']:>8.4f} {p['Commodity']:>8.4f}")


# ================================================================
# Part 4: Mathematical Explanation of ~1% Convergence
# ================================================================
print("\n" + "=" * 70)
print("PART 4: MATHEMATICAL EXPLANATION")
print("=" * 70)

# The convergence at ~0.7% percentile means:
# For each node, ~0.7% of the time, all three edges become nearly collinear.
#
# Mathematical model:
# 1. In a 252-day rolling window, sample correlation matrix C has eigenvalues
#    that follow the Marchenko-Pastur distribution in the null case.
# 2. Under the null (i.i.d. returns), the probability of observing
#    lambda_1/trace(C) > 0.95 is determined by the Tracy-Widom distribution.
# 3. But these edges are NOT i.i.d. — they share common factors.
#    The question is: what fraction of 252-day windows have the correlation
#    structure so extreme that effectively only one factor operates?

# Compute the empirical distribution of PC1 EVR for each node
print("\nEmpirical PC1 EVR distribution:")
for name in NODE_DEFS:
    pc1 = node_data[name]["pc1_evr"]
    print(f"\n  {name}:")
    for p_thresh in [0.90, 0.92, 0.94, 0.95, 0.96, 0.97, 0.98, 0.99]:
        frac = (pc1 >= p_thresh).mean()
        print(f"    PC1 >= {p_thresh*100:.0f}%: {frac*100:.3f}% of days")

# What PC1 EVR is at the 0.7th percentile of each node's eff.dim?
print("\n--- PC1 EVR at event-floor percentile (~0.7%) ---")
convergence_summary = {}
for name in NODE_DEFS:
    ed_series = node_data[name]["eff_dim"]
    pc1_series = node_data[name]["pc1_evr"]

    # Get the eff.dim value at ~0.7th percentile
    ed_at_0_7 = np.percentile(ed_series, 0.7)
    # Find days where eff.dim is at or below this
    mask = ed_series <= ed_at_0_7
    pc1_at_floor = pc1_series[mask]

    convergence_summary[name] = {
        "eff_dim_at_0.7pct": round(float(ed_at_0_7), 4),
        "pc1_evr_mean_at_floor": round(float(pc1_at_floor.mean()), 6),
        "pc1_evr_min_at_floor": round(float(pc1_at_floor.min()), 6),
        "n_days_at_floor": int(mask.sum()),
    }
    print(f"  {name:>10}: eff.dim@0.7pct = {ed_at_0_7:.4f}, "
          f"PC1 EVR = {pc1_at_floor.mean():.4f} ({pc1_at_floor.mean()*100:.1f}%)")


# ================================================================
# Part 5: The ~1% Theorem
# ================================================================
print("\n" + "=" * 70)
print("PART 5: THE ~1% THEOREM")
print("=" * 70)

# Theorem: For a 3x3 correlation matrix computed from 252-day rolling windows
# of log-ratio returns, the probability of entering the "effective 1D" regime
# (where the system's three edges become nearly collinear) converges to
# approximately 0.7% across all four nodes.
#
# This is NOT because the nodes have the same correlation structure.
# It IS because:
# 1. Each node's eff.dim distribution has a hard lower bound (1.0)
# 2. The distribution near the lower bound follows a similar tail shape
#    across nodes, determined by the 252-day window statistics
# 3. The "event floor" (minimum eff.dim observed during extreme events)
#    falls at the same relative position (~0.7th percentile) because:
#    - Events require CORRELATION BREAKDOWN (edges diverge abnormally)
#    - But the most extreme events require CORRELATION COMPRESSION first
#      (the "spring-loading" mechanism)
#    - The probability of this pre-compression is governed by the tail
#      of the eff.dim distribution, which is a function of sample size (252)
#      and dimension (3)

# Verify: use the prior experiments' event floor percentiles
prior_event_floor_percentiles = {
    "Au": 0.77,
    "Dollar": 0.60,
    "Equity": 0.78,
    "Commodity": 0.81,
}

# Compute event-floor percentiles with our correlation-based eff.dim
print("\nEvent-floor percentiles (correlation-based eff.dim):")
our_event_floor_pcts = {}
for name in NODE_DEFS:
    ed_series = node_data[name]["eff_dim"]
    # Use the min_event_effdim from prior experiments
    min_event_ed = event_floors[name]
    pct = stats.percentileofscore(ed_series, min_event_ed)
    our_event_floor_pcts[name] = round(float(pct), 4)
    print(f"  {name:>10}: min_event_effdim = {min_event_ed:.3f} -> percentile = {pct:.4f}%")

print(f"\nMean percentile: {np.mean(list(our_event_floor_pcts.values())):.4f}%")
print(f"Std percentile:  {np.std(list(our_event_floor_pcts.values())):.4f}%")
print(f"Spread:          {max(our_event_floor_pcts.values()) - min(our_event_floor_pcts.values()):.4f}%")


# ================================================================
# Part 6: Compile Results
# ================================================================
results = {
    "meta": {
        "description": "C1: effective-1D condition mathematical derivation and cross-validation",
        "epistemological_levels": {
            "part_1_theory": "L0 (pure mathematics, no data dependency)",
            "part_2_definition": "L0 (definition)",
            "part_3_verification": "L3 (four-node cross-validation on real data)",
        },
    },
    "theory": {
        "eff_dim_formula": "exp(-sum(p_i * log(p_i))), p_i = lambda_i / sum(lambda_j)",
        "boundaries": {
            "complete_1d": {"eigenvalues": [3, 0, 0], "eff_dim": 1.0},
            "complete_3d": {"eigenvalues": [1, 1, 1], "eff_dim": 3.0},
        },
        "pc1_evr_to_effdim": theory_table,
        "effdim_to_pc1_evr": inverse_table,
    },
    "effective_1d_definition": {
        "pc1_threshold_95pct": {
            "threshold": 0.95,
            "eff_dim_critical": round(ed_at_95, 6),
            "interpretation": "One factor explains 95%+ of variance; three edges 95%+ collinear",
        },
        "pc1_threshold_90pct": {
            "threshold": 0.90,
            "eff_dim_critical": round(ed_at_90, 6),
        },
    },
    "threshold_verification": results_by_threshold,
    "reverse_percentile": reverse_results,
    "direct_effdim_thresholds": direct_results,
    "convergence_summary": convergence_summary,
    "event_floor_percentiles": {
        "prior_experiment": prior_event_floor_percentiles,
        "correlation_based": our_event_floor_pcts,
        "mean": round(float(np.mean(list(our_event_floor_pcts.values()))), 4),
        "std": round(float(np.std(list(our_event_floor_pcts.values()))), 4),
        "spread_pp": round(float(max(our_event_floor_pcts.values()) - min(our_event_floor_pcts.values())), 4),
    },
}

output_file = OUTPUT_DIR / "c1_results.json"
with open(output_file, "w", encoding="utf-8") as f:
    json.dump(results, f, indent=2, ensure_ascii=False, default=str)
print(f"\nResults saved to {output_file}")
