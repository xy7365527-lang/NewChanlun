"""
共享边假说检验
==============
K4完全图四节点（Au, $, Equity, Commodity）的 eff.dim 时间序列相关性
与 frac_below 距离的对比。

假说：eff.dim 时间序列相关性高的节点对，frac_below 也接近。

认识论等级：L2（真实数据验证）
谱系引用：323号（折叠理论结算）
"""

import json
import warnings
from pathlib import Path

import numpy as np
import pandas as pd
import yfinance as yf
from scipy import stats

warnings.filterwarnings("ignore")

OUTPUT_DIR = Path(__file__).parent

# ============================================================
# 0. Known frac_below from node experiments
# ============================================================
FRAC_BELOW = {
    "Au": 0.0692,
    "$": 0.0673,
    "Equity": 0.0077,
    "Commodity": 0.0081,
}

NODE_NAMES = ["Au", "$", "Equity", "Commodity"]

# ============================================================
# 1. Data Acquisition
# ============================================================
def _dl(ticker, start="2000-01-01", end="2026-03-03"):
    raw = yf.download(ticker, start=start, end=end)["Close"]
    if isinstance(raw, pd.DataFrame):
        raw = raw.iloc[:, 0]
    return raw


print("Downloading data for all four nodes...")
gold = _dl("GC=F")
dxy = _dl("DX-Y.NYB")
spx = _dl("^GSPC")
clf = _dl("CL=F")
copper = _dl("HG=F")

# Align all series
df = pd.DataFrame({
    "gold": gold, "dxy": dxy, "spx": spx, "clf": clf, "copper": copper
}).dropna()

# Drop negative/zero CL=F (2020-04-20)
df = df[df["clf"] > 0]

print(f"Aligned data: {len(df)} obs, {df.index[0].date()} to {df.index[-1].date()}")

# ============================================================
# 2. Compute eff.dim for each node (252d rolling window)
# ============================================================
def compute_effdim(returns_df, window=252):
    """Compute rolling effective dimensionality from 3-edge return covariance."""
    vals = []
    dates = []
    for i in range(window, len(returns_df)):
        w = returns_df.iloc[i - window : i]
        cov = w.cov().values
        eigvals = np.linalg.eigvalsh(cov)
        eigvals = eigvals[eigvals > 0]
        p = eigvals / eigvals.sum()
        entropy = -np.sum(p * np.log(p))
        vals.append(np.exp(entropy))
        dates.append(returns_df.index[i])
    return pd.Series(vals, index=dates)


# Au node edges: Au/DXY, Au/SPX, Au/Commodity
# (using copper for commodity as in addendum; dollar node used copper too)
au_ratios = pd.DataFrame({
    "e1_au_dxy": np.log(df["gold"] / df["dxy"]).diff(),
    "e2_au_spx": np.log(df["gold"] / df["spx"]).diff(),
    "e3_au_clf": np.log(df["gold"] / df["clf"]).diff(),
}).dropna()

# $ node edges: $/Au, $/SPX, $/Commodity
dol_ratios = pd.DataFrame({
    "e1_dxy_gold": np.log(df["dxy"] / df["gold"]).diff(),
    "e2_dxy_spx": np.log(df["dxy"] / df["spx"]).diff(),
    "e3_dxy_copper": np.log(df["dxy"] / copper.reindex(df.index).dropna()).diff(),
}).dropna()
# Note: dollar node used HG=F for commodity edge in original experiment
# Re-download copper aligned to df
copper_aligned = copper.reindex(df.index).dropna()
dol_ratios = pd.DataFrame({
    "e1_dxy_gold": np.log(df["dxy"] / df["gold"]).diff(),
    "e2_dxy_spx": np.log(df["dxy"] / df["spx"]).diff(),
    "e3_dxy_copper": np.log(df["dxy"] / copper_aligned).diff(),
}).dropna()

# Equity node edges: SPX/$, SPX/Au, SPX/Commodity
eq_ratios = pd.DataFrame({
    "e1_spx_dxy": np.log(df["spx"] / df["dxy"]).diff(),
    "e2_spx_gold": np.log(df["spx"] / df["gold"]).diff(),
    "e3_spx_clf": np.log(df["spx"] / df["clf"]).diff(),
}).dropna()

# Commodity node edges: CL/$, CL/Au, CL/SPX
com_ratios = pd.DataFrame({
    "e1_clf_dxy": np.log(df["clf"] / df["dxy"]).diff(),
    "e2_clf_gold": np.log(df["clf"] / df["gold"]).diff(),
    "e3_clf_spx": np.log(df["clf"] / df["spx"]).diff(),
}).dropna()

print("Computing eff.dim for Au node...")
ed_au = compute_effdim(au_ratios)
print(f"  Au: {len(ed_au)} pts, mean={ed_au.mean():.3f}")

print("Computing eff.dim for $ node...")
ed_dol = compute_effdim(dol_ratios)
print(f"  $: {len(ed_dol)} pts, mean={ed_dol.mean():.3f}")

print("Computing eff.dim for Equity node...")
ed_eq = compute_effdim(eq_ratios)
print(f"  Equity: {len(ed_eq)} pts, mean={ed_eq.mean():.3f}")

print("Computing eff.dim for Commodity node...")
ed_com = compute_effdim(com_ratios)
print(f"  Commodity: {len(ed_com)} pts, mean={ed_com.mean():.3f}")

# ============================================================
# 3. Align all four eff.dim series
# ============================================================
ed_all = pd.DataFrame({
    "Au": ed_au,
    "$": ed_dol,
    "Equity": ed_eq,
    "Commodity": ed_com,
}).dropna()

print(f"\nAligned eff.dim: {len(ed_all)} common dates")

# ============================================================
# 4. Compute 4x4 eff.dim correlation matrix
# ============================================================
pearson_corr = ed_all.corr(method="pearson")
spearman_corr = ed_all.corr(method="spearman")

print("\n=== Pearson correlation of eff.dim time series ===")
print(pearson_corr.round(4).to_string())

print("\n=== Spearman correlation of eff.dim time series ===")
print(spearman_corr.round(4).to_string())

# ============================================================
# 5. Compute 4x4 frac_below distance matrix
# ============================================================
n = len(NODE_NAMES)
frac_dist = pd.DataFrame(np.zeros((n, n)), index=NODE_NAMES, columns=NODE_NAMES)
for i, ni in enumerate(NODE_NAMES):
    for j, nj in enumerate(NODE_NAMES):
        frac_dist.iloc[i, j] = abs(FRAC_BELOW[ni] - FRAC_BELOW[nj])

print("\n=== frac_below distance matrix ===")
print(frac_dist.round(4).to_string())

# ============================================================
# 6. Extract upper triangle pairs for correlation analysis
# ============================================================
pairs = []
for i in range(n):
    for j in range(i + 1, n):
        ni, nj = NODE_NAMES[i], NODE_NAMES[j]
        pair = f"{ni}-{nj}"
        fb_dist = frac_dist.iloc[i, j]
        pc = pearson_corr.iloc[i, j]
        sc = spearman_corr.iloc[i, j]

        # Compute p-values for correlations
        s1 = ed_all[ni]
        s2 = ed_all[nj]
        pr, pp = stats.pearsonr(s1, s2)
        sr, sp = stats.spearmanr(s1, s2)

        pairs.append({
            "pair": pair,
            "frac_below_distance": round(float(fb_dist), 4),
            "pearson_r": round(float(pr), 4),
            "pearson_p": round(float(pp), 6),
            "spearman_r": round(float(sr), 4),
            "spearman_p": round(float(sp), 6),
        })

print("\n=== Pair-level analysis ===")
print(f"{'Pair':<20} {'|Δfrac|':>8} {'Pearson r':>10} {'Spearman r':>11}")
print("-" * 55)
for p in pairs:
    print(f"{p['pair']:<20} {p['frac_below_distance']:>8.4f} "
          f"{p['pearson_r']:>10.4f} {p['spearman_r']:>11.4f}")

# ============================================================
# 7. Test: correlation between eff.dim similarity and frac_below proximity
# ============================================================
print("\n=== HYPOTHESIS TEST ===")
print("H0: eff.dim相关性与frac_below接近度无关")
print("H1: eff.dim相关性越高的节点对，frac_below差值越小")

fb_dists = [p["frac_below_distance"] for p in pairs]
ed_corrs_p = [p["pearson_r"] for p in pairs]
ed_corrs_s = [p["spearman_r"] for p in pairs]

# With only 6 pairs, use Spearman rank correlation
# Negative correlation expected: higher eff.dim corr → smaller frac_below distance
if len(pairs) >= 4:
    r_p, p_p = stats.spearmanr(ed_corrs_p, fb_dists)
    r_s, p_s = stats.spearmanr(ed_corrs_s, fb_dists)
    print(f"\nSpearman(Pearson_r vs frac_distance): rho={r_p:+.4f}, p={p_p:.4f}")
    print(f"Spearman(Spearman_r vs frac_distance): rho={r_s:+.4f}, p={p_s:.4f}")
    print("(负相关 = 假说成立：eff.dim越相关的节点对，frac_below差值越小)")
else:
    r_p, p_p, r_s, p_s = float('nan'), float('nan'), float('nan'), float('nan')

# ============================================================
# 8. Cluster analysis: 2-group structure
# ============================================================
print("\n=== CLUSTER STRUCTURE ===")
print("从frac_below看，存在明显的2-group结构：")
print(f"  Group A (高frac_below): Au={FRAC_BELOW['Au']:.4f}, $={FRAC_BELOW['$']:.4f}")
print(f"  Group B (低frac_below): Equity={FRAC_BELOW['Equity']:.4f}, Commodity={FRAC_BELOW['Commodity']:.4f}")
print(f"  组内距离A: {abs(FRAC_BELOW['Au'] - FRAC_BELOW['$']):.4f}")
print(f"  组内距离B: {abs(FRAC_BELOW['Equity'] - FRAC_BELOW['Commodity']):.4f}")
print(f"  组间距离(mean): {((FRAC_BELOW['Au'] + FRAC_BELOW['$'])/2 - (FRAC_BELOW['Equity'] + FRAC_BELOW['Commodity'])/2):.4f}")

# Check: does eff.dim correlation show the same 2-group structure?
# Within-group correlations
within_a = pearson_corr.loc["Au", "$"]
within_b = pearson_corr.loc["Equity", "Commodity"]

# Between-group correlations
between = []
for ni in ["Au", "$"]:
    for nj in ["Equity", "Commodity"]:
        between.append(pearson_corr.loc[ni, nj])
between_mean = np.mean(between)

print(f"\nEff.dim Pearson相关性：")
print(f"  组内A (Au-$): {within_a:.4f}")
print(f"  组内B (Equity-Commodity): {within_b:.4f}")
print(f"  组间平均: {between_mean:.4f}")
print(f"  组内平均: {(within_a + within_b) / 2:.4f}")

cluster_consistent = (within_a + within_b) / 2 > between_mean
print(f"\n2-group结构一致性: {'YES' if cluster_consistent else 'NO'}")
print(f"  (组内相关性 > 组间相关性 → frac_below分群由eff.dim共变结构支持)")

# ============================================================
# 9. Rolling correlation analysis
# ============================================================
print("\n=== ROLLING CORRELATION (252d window) ===")
rolling_corrs = {}
for i in range(n):
    for j in range(i + 1, n):
        ni, nj = NODE_NAMES[i], NODE_NAMES[j]
        pair = f"{ni}-{nj}"
        rc = ed_all[ni].rolling(252).corr(ed_all[nj]).dropna()
        rolling_corrs[pair] = rc
        print(f"  {pair}: mean={rc.mean():.4f}, std={rc.std():.4f}, "
              f"min={rc.min():.4f}, max={rc.max():.4f}")

# ============================================================
# 10. Summary and results
# ============================================================
# Determine verdict
if not np.isnan(r_p):
    if r_p < -0.5 and p_p < 0.10:  # 6 pairs = low power, relax threshold
        hyp_verdict = "SUPPORTED"
    elif r_p < 0:
        hyp_verdict = "WEAK_SUPPORT"
    else:
        hyp_verdict = "NOT_SUPPORTED"
else:
    hyp_verdict = "INSUFFICIENT_DATA"

# Alternative: direct comparison of within vs between group correlation
if cluster_consistent:
    cluster_verdict = "2-GROUP_CONFIRMED"
    cluster_desc = "eff.dim共变结构与frac_below分群一致：Au-$高相关+高frac_below，Equity-Commodity另成一群"
else:
    cluster_verdict = "2-GROUP_NOT_CONFIRMED"
    cluster_desc = "eff.dim共变结构与frac_below分群不一致"

results = {
    "hypothesis": "共享边假说：eff.dim时间序列相关性高的节点对，frac_below更接近",
    "frac_below": FRAC_BELOW,
    "frac_below_distance_matrix": {
        f"{NODE_NAMES[i]}-{NODE_NAMES[j]}": round(float(frac_dist.iloc[i, j]), 4)
        for i in range(n) for j in range(i + 1, n)
    },
    "effdim_summary": {
        name: {
            "n_pts": len(ed_all),
            "mean": round(float(ed_all[name].mean()), 3),
            "std": round(float(ed_all[name].std()), 3),
            "min": round(float(ed_all[name].min()), 3),
            "max": round(float(ed_all[name].max()), 3),
        }
        for name in NODE_NAMES
    },
    "effdim_pearson_correlation": {
        f"{NODE_NAMES[i]}-{NODE_NAMES[j]}": round(float(pearson_corr.iloc[i, j]), 4)
        for i in range(n) for j in range(i + 1, n)
    },
    "effdim_spearman_correlation": {
        f"{NODE_NAMES[i]}-{NODE_NAMES[j]}": round(float(spearman_corr.iloc[i, j]), 4)
        for i in range(n) for j in range(i + 1, n)
    },
    "pair_details": pairs,
    "hypothesis_test": {
        "method": "Spearman rank correlation between eff.dim correlation and frac_below distance (6 pairs)",
        "spearman_pearson_r_vs_frac": {
            "rho": round(float(r_p), 4) if not np.isnan(r_p) else None,
            "p": round(float(p_p), 4) if not np.isnan(p_p) else None,
        },
        "spearman_spearman_r_vs_frac": {
            "rho": round(float(r_s), 4) if not np.isnan(r_s) else None,
            "p": round(float(p_s), 4) if not np.isnan(p_s) else None,
        },
        "verdict": hyp_verdict,
        "note": "6个数据点（6对节点）统计功效低，结论谨慎解读",
    },
    "cluster_analysis": {
        "group_a": {"nodes": ["Au", "$"], "frac_below_mean": round((FRAC_BELOW["Au"] + FRAC_BELOW["$"]) / 2, 4)},
        "group_b": {"nodes": ["Equity", "Commodity"], "frac_below_mean": round((FRAC_BELOW["Equity"] + FRAC_BELOW["Commodity"]) / 2, 4)},
        "within_group_a_corr": round(float(within_a), 4),
        "within_group_b_corr": round(float(within_b), 4),
        "between_group_mean_corr": round(float(between_mean), 4),
        "within_vs_between": round(float((within_a + within_b) / 2 - between_mean), 4),
        "verdict": cluster_verdict,
        "description": cluster_desc,
    },
    "rolling_correlation_summary": {
        pair: {
            "mean": round(float(rc.mean()), 4),
            "std": round(float(rc.std()), 4),
            "min": round(float(rc.min()), 4),
            "max": round(float(rc.max()), 4),
        }
        for pair, rc in rolling_corrs.items()
    },
}

# Final verdict
print(f"\n{'='*70}")
print("FINAL VERDICTS")
print(f"{'='*70}")
print(f"假说检验（Spearman rank）: {hyp_verdict}")
print(f"  rho(Pearson_r vs frac_dist) = {r_p:+.4f}, p = {p_p:.4f}")
print(f"  rho(Spearman_r vs frac_dist) = {r_s:+.4f}, p = {p_s:.4f}")
print(f"\n2-group分群一致性: {cluster_verdict}")
print(f"  组内平均相关: {(within_a + within_b) / 2:.4f}")
print(f"  组间平均相关: {between_mean:.4f}")
print(f"  {cluster_desc}")

output_path = OUTPUT_DIR / "shared_edge_results.json"
with open(output_path, "w", encoding="utf-8") as f:
    json.dump(results, f, indent=2, ensure_ascii=False)

print(f"\nResults saved to {output_path}")
