"""
折叠内在性检验 第三轮附加：级别与折叠的关系
============================================
命题一：eff.dim 是级别的度量（趋势在场时 eff.dim 低于趋势缺席时）
命题二：级别调制折叠深度（事件前 eff.dim 与 Mahalanobis 距离正相关）

数据来源：第三轮实验 Agent A + Agent B 现有结果
"""

import json
import warnings
from pathlib import Path

import numpy as np
import pandas as pd
import yfinance as yf
from scipy import stats
from sklearn.decomposition import PCA

warnings.filterwarnings("ignore")

OUTPUT_DIR = Path(__file__).parent

# ============================================================
# 1. Data Acquisition (same as agent_a)
# ============================================================
def _dl(ticker, start="2000-01-01", end="2026-03-01"):
    raw = yf.download(ticker, start=start, end=end)["Close"]
    if isinstance(raw, pd.DataFrame):
        raw = raw.iloc[:, 0]
    return raw

print("Downloading data...")
gold = _dl("GC=F")
copper = _dl("HG=F")
dxy = _dl("DX-Y.NYB")
spx = _dl("^GSPC")

df = pd.DataFrame({"gold": gold, "copper": copper, "dxy": dxy, "spx": spx}).dropna()
e1 = df["gold"] / df["dxy"]
e2 = df["copper"] / df["gold"]
e3 = df["gold"] / df["spx"]

returns = pd.DataFrame({
    "e1": np.log(e1).diff(),
    "e2": np.log(e2).diff(),
    "e3": np.log(e3).diff(),
}).dropna()

print(f"Data: {len(returns)} obs, {returns.index[0].date()} to {returns.index[-1].date()}")

# ============================================================
# 2. Rolling Effective Dimensionality (252d window)
# ============================================================
print("Computing rolling effective dimensionality...")
window = 252
eff_dim_vals = []
eff_dim_dates = []

for i in range(window, len(returns)):
    w = returns.iloc[i - window : i]
    cov = w.cov().values
    eigvals = np.linalg.eigvalsh(cov)
    eigvals = eigvals[eigvals > 0]
    p = eigvals / eigvals.sum()
    entropy = -np.sum(p * np.log(p))
    eff_dim_vals.append(np.exp(entropy))
    eff_dim_dates.append(returns.index[i])

eff_dim_series = pd.Series(eff_dim_vals, index=eff_dim_dates)
print(f"Eff.dim: {len(eff_dim_series)} pts, mean={eff_dim_series.mean():.3f}, std={eff_dim_series.std():.3f}")

# ============================================================
# 3. Mahalanobis Distance for Each Day
# ============================================================
print("Computing Mahalanobis distances...")
mean_ret = returns.mean().values
cov_ret = returns.cov().values
cov_inv = np.linalg.inv(cov_ret)

mahal_vals = []
for _, row in returns.iterrows():
    diff = row.values - mean_ret
    mahal_vals.append(np.sqrt(diff @ cov_inv @ diff))

mahal_series = pd.Series(mahal_vals, index=returns.index)

# ============================================================
# 4. PROPOSITION 1: eff.dim × Chanlun trend state
# ============================================================
print("\n" + "=" * 70)
print("PROPOSITION 1: Effective dimensionality as a measure of level")
print("=" * 70)

# Agent B trend periods
trend_periods = [
    ("2001-07-31", "2011-12-29", "T", "上升趋势1"),
    ("2011-12-30", "2013-04-16", "N", "间隔1"),
    ("2013-04-17", "2015-02-09", "T", "下降趋势2"),
    ("2015-02-10", "2016-02-11", "N", "间隔2"),
    ("2016-02-12", "2017-12-12", "T", "下降趋势3"),
    ("2017-12-13", "2021-03-07", "N", "间隔3"),
    ("2021-03-08", "2023-11-17", "T", "上升趋势4"),
    ("2023-11-18", "2026-02-27", "N", "间隔4"),
]

# Label each eff.dim observation
labels = pd.Series(index=eff_dim_series.index, dtype=str)
trend_labels = pd.Series(index=eff_dim_series.index, dtype=str)
for start, end, state, name in trend_periods:
    mask = (eff_dim_series.index >= start) & (eff_dim_series.index <= end)
    labels[mask] = state
    trend_labels[mask] = name

# Drop unlabeled (before first period)
labeled = eff_dim_series[labels.isin(["T", "N"])]
labeled_labels = labels[labels.isin(["T", "N"])]

t_group = labeled[labeled_labels == "T"]
n_group = labeled[labeled_labels == "N"]

print(f"\nT group (trend present):  n={len(t_group)}, median={t_group.median():.3f}, "
      f"mean={t_group.mean():.3f}, IQR=[{t_group.quantile(0.25):.3f}, {t_group.quantile(0.75):.3f}]")
print(f"N group (trend absent):   n={len(n_group)}, median={n_group.median():.3f}, "
      f"mean={n_group.mean():.3f}, IQR=[{n_group.quantile(0.25):.3f}, {n_group.quantile(0.75):.3f}]")

# Welch t-test (one-sided: T < N)
t_stat, t_p_two = stats.ttest_ind(t_group.values, n_group.values, equal_var=False)
t_p_one = t_p_two / 2 if t_stat < 0 else 1 - t_p_two / 2
print(f"\nWelch t-test (T < N): t={t_stat:.4f}, p={t_p_one:.6f}")

# Mann-Whitney U (one-sided: T < N)
u_stat, u_p_two = stats.mannwhitneyu(t_group.values, n_group.values, alternative="less")
print(f"Mann-Whitney U (T < N): U={u_stat:.1f}, p={u_p_two:.6f}")

# Cohen's d
pooled_std = np.sqrt((t_group.std() ** 2 + n_group.std() ** 2) / 2)
cohens_d = (t_group.mean() - n_group.mean()) / pooled_std
print(f"Cohen's d: {cohens_d:.4f}")

prop1_pass = t_p_one < 0.05 and cohens_d < -0.2
print(f"\nPROPOSITION 1 VERDICT: {'PASS' if prop1_pass else 'FAIL'}")
print(f"  p < 0.05: {t_p_one < 0.05}, Cohen's d magnitude > 0.2: {abs(cohens_d) > 0.2}")

# ---- Supplementary: per-trend-segment analysis ----
print("\n--- Supplementary: Per-segment eff.dim ---")
for start, end, state, name in trend_periods:
    mask = (eff_dim_series.index >= start) & (eff_dim_series.index <= end)
    seg = eff_dim_series[mask]
    if len(seg) > 0:
        print(f"  {name:<16} ({state}): n={len(seg):>4}, median={seg.median():.3f}, "
              f"mean={seg.mean():.3f}, range=[{seg.min():.3f}, {seg.max():.3f}]")

# ---- Supplementary: rising vs falling trend ----
print("\n--- Supplementary: Rising vs Falling trends ---")
rising_periods = [("2001-07-31", "2011-12-29"), ("2021-03-08", "2023-11-17")]
falling_periods = [("2013-04-17", "2015-02-09"), ("2016-02-12", "2017-12-12")]

rising_data = pd.concat([eff_dim_series[(eff_dim_series.index >= s) & (eff_dim_series.index <= e)]
                          for s, e in rising_periods])
falling_data = pd.concat([eff_dim_series[(eff_dim_series.index >= s) & (eff_dim_series.index <= e)]
                           for s, e in falling_periods])

print(f"Rising trends:  n={len(rising_data)}, median={rising_data.median():.3f}, mean={rising_data.mean():.3f}")
print(f"Falling trends: n={len(falling_data)}, median={falling_data.median():.3f}, mean={falling_data.mean():.3f}")

if len(rising_data) >= 10 and len(falling_data) >= 10:
    rf_t, rf_p = stats.ttest_ind(rising_data.values, falling_data.values, equal_var=False)
    print(f"Welch t-test (rising vs falling): t={rf_t:.4f}, p={rf_p:.4f}")

# ---- Supplementary: within-trend drift ----
print("\n--- Supplementary: Within-trend eff.dim drift ---")
for start, end, state, name in trend_periods:
    if state != "T":
        continue
    mask = (eff_dim_series.index >= start) & (eff_dim_series.index <= end)
    seg = eff_dim_series[mask]
    if len(seg) < 20:
        continue
    # Split into first half / second half
    mid = len(seg) // 2
    first_half = seg.iloc[:mid]
    second_half = seg.iloc[mid:]
    print(f"  {name}:")
    print(f"    First half:  n={len(first_half)}, mean={first_half.mean():.3f}, median={first_half.median():.3f}")
    print(f"    Second half: n={len(second_half)}, mean={second_half.mean():.3f}, median={second_half.median():.3f}")
    # Linear trend (Spearman rank correlation with time index)
    time_idx = np.arange(len(seg))
    sp_r, sp_p = stats.spearmanr(time_idx, seg.values)
    print(f"    Spearman (eff.dim vs time): r={sp_r:+.4f}, p={sp_p:.4f}")

# ============================================================
# 5. PROPOSITION 2: eff.dim × fold depth
# ============================================================
print("\n" + "=" * 70)
print("PROPOSITION 2: Level modulates fold depth")
print("=" * 70)

# Identify extreme events (same as agent_a: top 1% Mahalanobis)
threshold = mahal_series.quantile(0.99)
extreme_mask = mahal_series > threshold
extreme_dates = mahal_series[extreme_mask].index.tolist()

# Cluster extreme days
events = []
if extreme_dates:
    current = {"start": extreme_dates[0], "end": extreme_dates[0],
               "max_mahal": float(mahal_series[extreme_dates[0]]), "days": 1}
    for d in extreme_dates[1:]:
        if (d - current["end"]).days <= 7:
            current["end"] = d
            current["max_mahal"] = max(current["max_mahal"], float(mahal_series[d]))
            current["days"] += 1
        else:
            events.append(current)
            current = {"start": d, "end": d, "max_mahal": float(mahal_series[d]), "days": 1}
    events.append(current)

print(f"Extreme events: {len(events)} (threshold Mahal={threshold:.2f})")

# For each event: get pre-event eff.dim
event_records = []
for ev in events:
    start = ev["start"]
    # Pre-5d: 5 trading days before event start
    pre_candidates = eff_dim_series.index[eff_dim_series.index < start]
    if len(pre_candidates) < 5:
        continue

    pre_5d = float(eff_dim_series[pre_candidates[-5]])
    pre_10d = float(eff_dim_series[pre_candidates[-10]]) if len(pre_candidates) >= 10 else None
    pre_20d = float(eff_dim_series[pre_candidates[-20]]) if len(pre_candidates) >= 20 else None

    record = {
        "event_start": str(start.date()),
        "event_end": str(ev["end"].date()),
        "days": ev["days"],
        "max_mahal": round(ev["max_mahal"], 2),
        "eff_dim_pre5d": round(pre_5d, 3),
        "eff_dim_pre10d": round(pre_10d, 3) if pre_10d else None,
        "eff_dim_pre20d": round(pre_20d, 3) if pre_20d else None,
    }
    event_records.append(record)

print(f"\nEvent-level data ({len(event_records)} events with pre-event eff.dim):\n")
print(f"{'Event':<14} {'Days':>5} {'Mahal':>7} {'pre5d':>7} {'pre10d':>7} {'pre20d':>7}")
print("-" * 55)
for r in event_records:
    p10 = f"{r['eff_dim_pre10d']:.3f}" if r['eff_dim_pre10d'] is not None else "N/A"
    p20 = f"{r['eff_dim_pre20d']:.3f}" if r['eff_dim_pre20d'] is not None else "N/A"
    print(f"{r['event_start']:<14} {r['days']:>5} {r['max_mahal']:>7.2f} "
          f"{r['eff_dim_pre5d']:>7.3f} {p10:>7} {p20:>7}")

# Correlation analysis
mahals = [r["max_mahal"] for r in event_records]
effdims_5d = [r["eff_dim_pre5d"] for r in event_records]
effdims_10d = [r["eff_dim_pre10d"] for r in event_records if r["eff_dim_pre10d"] is not None]
effdims_20d = [r["eff_dim_pre20d"] for r in event_records if r["eff_dim_pre20d"] is not None]
mahals_10d = [r["max_mahal"] for r in event_records if r["eff_dim_pre10d"] is not None]
mahals_20d = [r["max_mahal"] for r in event_records if r["eff_dim_pre20d"] is not None]

print("\n--- Primary: Spearman rank correlation (one-sided: positive) ---")
for label, ed, mh in [("pre-5d", effdims_5d, mahals),
                        ("pre-10d", effdims_10d, mahals_10d),
                        ("pre-20d", effdims_20d, mahals_20d)]:
    if len(ed) < 5:
        continue
    sp_r, sp_p_two = stats.spearmanr(ed, mh)
    sp_p_one = sp_p_two / 2 if sp_r > 0 else 1 - sp_p_two / 2
    pr_r, pr_p_two = stats.pearsonr(ed, mh)
    pr_p_one = pr_p_two / 2 if pr_r > 0 else 1 - pr_p_two / 2
    print(f"  {label}: Spearman rho={sp_r:+.4f}, p(one-sided)={sp_p_one:.4f} | "
          f"Pearson r={pr_r:+.4f}, p(one-sided)={pr_p_one:.4f}")

# Robustness: remove top 2 extreme Mahalanobis events
print("\n--- Robustness: Remove 2 most extreme Mahal events ---")
sorted_by_mahal = sorted(event_records, key=lambda r: r["max_mahal"], reverse=True)
excluded = {sorted_by_mahal[0]["event_start"], sorted_by_mahal[1]["event_start"]}
print(f"  Excluded: {excluded}")
robust_records = [r for r in event_records if r["event_start"] not in excluded]
rob_mahals = [r["max_mahal"] for r in robust_records]
rob_effdims = [r["eff_dim_pre5d"] for r in robust_records]

if len(rob_effdims) >= 5:
    sp_r, sp_p_two = stats.spearmanr(rob_effdims, rob_mahals)
    sp_p_one = sp_p_two / 2 if sp_r > 0 else 1 - sp_p_two / 2
    print(f"  Spearman (n={len(robust_records)}): rho={sp_r:+.4f}, p(one-sided)={sp_p_one:.4f}")

# Robustness: different pre-event windows already covered above (5d, 10d, 20d)

# Median split
median_ed = np.median(effdims_5d)
high_group = [r for r in event_records if r["eff_dim_pre5d"] >= median_ed]
low_group = [r for r in event_records if r["eff_dim_pre5d"] < median_ed]
high_m = [r["max_mahal"] for r in high_group]
low_m = [r["max_mahal"] for r in low_group]

print(f"\n--- Median split (median eff.dim = {median_ed:.3f}) ---")
print(f"  High eff.dim (n={len(high_group)}): mean Mahal={np.mean(high_m):.2f}, median={np.median(high_m):.2f}")
print(f"  Low  eff.dim (n={len(low_group)}):  mean Mahal={np.mean(low_m):.2f}, median={np.median(low_m):.2f}")

if len(high_m) >= 3 and len(low_m) >= 3:
    mw_u, mw_p = stats.mannwhitneyu(high_m, low_m, alternative="greater")
    print(f"  Mann-Whitney U (high > low): U={mw_u:.1f}, p={mw_p:.4f}")

# ============================================================
# 6. Save Results
# ============================================================
results = {
    "proposition_1": {
        "T_group": {
            "n": len(t_group),
            "median": round(float(t_group.median()), 3),
            "mean": round(float(t_group.mean()), 3),
            "iqr": [round(float(t_group.quantile(0.25)), 3), round(float(t_group.quantile(0.75)), 3)],
        },
        "N_group": {
            "n": len(n_group),
            "median": round(float(n_group.median()), 3),
            "mean": round(float(n_group.mean()), 3),
            "iqr": [round(float(n_group.quantile(0.25)), 3), round(float(n_group.quantile(0.75)), 3)],
        },
        "welch_t": {"t": round(float(t_stat), 4), "p_one_sided": round(float(t_p_one), 6)},
        "mann_whitney_u": {"U": round(float(u_stat), 1), "p_one_sided": round(float(u_p_two), 6)},
        "cohens_d": round(float(cohens_d), 4),
        "verdict": "PASS" if prop1_pass else "FAIL",
    },
    "proposition_2": {
        "n_events": len(event_records),
        "events": event_records,
        "correlations": {},
        "median_split": {
            "median_effdim": round(float(median_ed), 3),
            "high_group_mean_mahal": round(float(np.mean(high_m)), 2),
            "low_group_mean_mahal": round(float(np.mean(low_m)), 2),
        },
    },
}

# Fill in correlations
for label, ed, mh in [("pre_5d", effdims_5d, mahals),
                        ("pre_10d", effdims_10d, mahals_10d),
                        ("pre_20d", effdims_20d, mahals_20d)]:
    if len(ed) < 5:
        continue
    sp_r, sp_p2 = stats.spearmanr(ed, mh)
    sp_p1 = sp_p2 / 2 if sp_r > 0 else 1 - sp_p2 / 2
    pr_r, pr_p2 = stats.pearsonr(ed, mh)
    pr_p1 = pr_p2 / 2 if pr_r > 0 else 1 - pr_p2 / 2
    results["proposition_2"]["correlations"][label] = {
        "spearman_rho": round(sp_r, 4),
        "spearman_p_one": round(sp_p1, 4),
        "pearson_r": round(pr_r, 4),
        "pearson_p_one": round(pr_p1, 4),
    }

# Proposition 2 verdict
sp5 = results["proposition_2"]["correlations"].get("pre_5d", {})
prop2_pass = sp5.get("spearman_rho", 0) > 0 and sp5.get("spearman_p_one", 1) < 0.05
results["proposition_2"]["verdict"] = "PASS" if prop2_pass else "FAIL"

# Combined verdict
p1v = results["proposition_1"]["verdict"]
p2v = results["proposition_2"]["verdict"]
if p1v == "PASS" and p2v == "PASS":
    combined = "有效维度是级别的度量，级别调制折叠深度。走势通过改变有效维度决定系统对折叠的易感性。"
elif p1v == "PASS" and p2v == "FAIL":
    combined = "有效维度是级别的度量，但级别不调制折叠深度。走势和折叠之间的关系不经过易感性机制。"
elif p1v == "FAIL" and p2v == "PASS":
    combined = "有效维度不是级别的正确度量，但它与折叠深度相关。eff.dim可能度量的是其他东西。"
else:
    combined = "有效维度既不是级别也不调制折叠。eff.dim的年度漂移与折叠无关。"

results["combined_verdict"] = combined

output_path = OUTPUT_DIR / "r3_addendum_results.json"
with open(output_path, "w", encoding="utf-8") as f:
    json.dump(results, f, indent=2, ensure_ascii=False)

print(f"\n{'='*70}")
print(f"COMBINED VERDICT")
print(f"{'='*70}")
print(f"Proposition 1: {p1v}")
print(f"Proposition 2: {p2v}")
print(f"Meaning: {combined}")
print(f"\nResults saved to {output_path}")
