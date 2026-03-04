"""
Equity节点折叠内在性检验（复制$节点方法论）
==================================================
K4拓扑中Equity节点连着三条边：Equity/$, Equity/Au, Equity/Commodity
SPX是Equity节点的直接代理变量。

问题1（不对称性来源 - 323号推论3）：下降压缩/上升散开是Au特殊性质还是K4一般性质？
问题2（阈值效应确认 - 323号推论2）：Equity节点是否存在类似的eff.dim下界？

认识论等级：L2（真实数据验证）
谱系引用：323号（折叠理论结算）、321号（第三轮判决）、324号（$节点meta-observation）
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
# 1. Data Acquisition
# ============================================================
def _dl(ticker, start="2000-01-01", end="2026-03-01"):
    raw = yf.download(ticker, start=start, end=end)["Close"]
    if isinstance(raw, pd.DataFrame):
        raw = raw.iloc[:, 0]
    return raw

print("Downloading data for Equity node...")
spx = _dl("^GSPC")
dxy = _dl("DX-Y.NYB")
gold = _dl("GC=F")
crude = _dl("CL=F")

df = pd.DataFrame({"spx": spx, "dxy": dxy, "gold": gold, "crude": crude}).dropna()

# Equity node edges: ratios with SPX as numerator
e1 = df["spx"] / df["dxy"]     # Equity/$
e2 = df["spx"] / df["gold"]    # Equity/Au
e3 = df["spx"] / df["crude"]   # Equity/Commodity

returns = pd.DataFrame({
    "e1_spx_dxy": np.log(e1).diff(),
    "e2_spx_gold": np.log(e2).diff(),
    "e3_spx_crude": np.log(e3).diff(),
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
# 4. SPX Trend Identification (TWO METHODS)
# ============================================================
print("\n" + "=" * 70)
print("SPX TREND IDENTIFICATION")
print("=" * 70)

# --- Method A: 252-day MA crossover (objective, automatic) ---
spx_aligned = df["spx"].reindex(eff_dim_series.index)
ma252 = spx_aligned.rolling(252).mean()
ma252_slope = ma252.diff(20)  # 20-day slope of MA

# T = price above rising MA OR price below falling MA
# N = everything else
auto_labels = pd.Series("N", index=eff_dim_series.index)
above_rising = (spx_aligned > ma252) & (ma252_slope > 0)
below_falling = (spx_aligned < ma252) & (ma252_slope < 0)
auto_labels[above_rising | below_falling] = "T"

# Filter out first 252 days where MA not yet available
auto_labels[ma252.isna()] = "X"

auto_t_count = (auto_labels == "T").sum()
auto_n_count = (auto_labels == "N").sum()
print(f"\nMethod A (MA crossover): T={auto_t_count} days, N={auto_n_count} days")

# --- Method B: Manual trend annotation (from task description) ---
manual_trend_periods = [
    ("2000-09-01", "2002-10-09", "T", "下降趋势1(科技泡沫破裂)"),
    ("2002-10-10", "2007-10-09", "T", "上升趋势1(复苏+房地产泡沫)"),
    ("2007-10-10", "2009-03-09", "T", "下降趋势2(金融危机)"),
    ("2009-03-10", "2020-02-19", "T", "上升趋势2(QE+长牛)"),
    ("2020-02-20", "2020-03-23", "T", "下降趋势3(COVID暴跌)"),
    ("2020-03-24", "2021-12-31", "T", "上升趋势3(疫后反弹)"),
    ("2022-01-01", "2022-10-12", "T", "下降趋势4(加息)"),
    ("2022-10-13", "2025-02-19", "T", "上升趋势4(AI牛市)"),
    ("2025-02-20", "2026-02-28", "N", "盘整/回调"),
]

# Separate rising and falling for asymmetry analysis
manual_rising_periods = [
    ("2002-10-10", "2007-10-09", "上升趋势1"),
    ("2009-03-10", "2020-02-19", "上升趋势2"),
    ("2020-03-24", "2021-12-31", "上升趋势3"),
    ("2022-10-13", "2025-02-19", "上升趋势4"),
]
manual_falling_periods = [
    ("2000-09-01", "2002-10-09", "下降趋势1"),
    ("2007-10-10", "2009-03-09", "下降趋势2"),
    ("2020-02-20", "2020-03-23", "下降趋势3"),
    ("2022-01-01", "2022-10-12", "下降趋势4"),
]

manual_labels = pd.Series("X", index=eff_dim_series.index)
manual_trend_labels = pd.Series("X", index=eff_dim_series.index)
for start, end, state, name in manual_trend_periods:
    mask = (eff_dim_series.index >= start) & (eff_dim_series.index <= end)
    manual_labels[mask] = state
    manual_trend_labels[mask] = name

manual_t_count = (manual_labels == "T").sum()
manual_n_count = (manual_labels == "N").sum()
print(f"Method B (manual annotation): T={manual_t_count} days, N={manual_n_count} days")

# ============================================================
# 5. PROPOSITION 1: eff.dim x SPX trend state (BOTH METHODS)
# ============================================================
def run_proposition_1(labels_series, method_name):
    """Run proposition 1 analysis with given labels."""
    print(f"\n{'=' * 70}")
    print(f"PROPOSITION 1 ({method_name}): Effective dimensionality as measure of level")
    print(f"{'=' * 70}")

    labeled = eff_dim_series[labels_series.isin(["T", "N"])]
    labeled_labels = labels_series[labels_series.isin(["T", "N"])]

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

    verdict = t_p_one < 0.05 and cohens_d < -0.2
    print(f"\nPROPOSITION 1 VERDICT ({method_name}): {'PASS' if verdict else 'FAIL'}")
    print(f"  p < 0.05: {t_p_one < 0.05}, Cohen's d magnitude > 0.2: {abs(cohens_d) > 0.2}")

    return {
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
        "verdict": "PASS" if verdict else "FAIL",
    }, t_group, n_group

prop1_auto, t_auto, n_auto = run_proposition_1(auto_labels, "MA crossover")
prop1_manual, t_manual, n_manual = run_proposition_1(manual_labels, "Manual annotation")

# ============================================================
# 6. Per-segment analysis (manual annotation)
# ============================================================
print("\n--- Per-segment eff.dim ---")
for start, end, state, name in manual_trend_periods:
    mask = (eff_dim_series.index >= start) & (eff_dim_series.index <= end)
    seg = eff_dim_series[mask]
    if len(seg) > 0:
        print(f"  {name:<28} ({state}): n={len(seg):>4}, median={seg.median():.3f}, "
              f"mean={seg.mean():.3f}, range=[{seg.min():.3f}, {seg.max():.3f}]")

# ============================================================
# 7. KEY TEST: Rising vs Falling trend asymmetry
# ============================================================
print("\n" + "=" * 70)
print("ASYMMETRY TEST: Rising vs Falling SPX trends")
print("=" * 70)

rising_data = pd.concat([eff_dim_series[(eff_dim_series.index >= s) & (eff_dim_series.index <= e)]
                          for s, e, _ in manual_rising_periods])
falling_data = pd.concat([eff_dim_series[(eff_dim_series.index >= s) & (eff_dim_series.index <= e)]
                           for s, e, _ in manual_falling_periods])

print(f"\nRising SPX (equity bull):   n={len(rising_data)}, median={rising_data.median():.3f}, "
      f"mean={rising_data.mean():.3f}, IQR=[{rising_data.quantile(0.25):.3f}, {rising_data.quantile(0.75):.3f}]")
print(f"Falling SPX (equity bear):  n={len(falling_data)}, median={falling_data.median():.3f}, "
      f"mean={falling_data.mean():.3f}, IQR=[{falling_data.quantile(0.25):.3f}, {falling_data.quantile(0.75):.3f}]")

if len(rising_data) >= 10 and len(falling_data) >= 10:
    rf_t, rf_p = stats.ttest_ind(rising_data.values, falling_data.values, equal_var=False)
    rf_u, rf_u_p = stats.mannwhitneyu(rising_data.values, falling_data.values, alternative="two-sided")
    rf_d = (rising_data.mean() - falling_data.mean()) / np.sqrt((rising_data.std()**2 + falling_data.std()**2) / 2)
    print(f"\nWelch t-test (rising vs falling): t={rf_t:.4f}, p={rf_p:.6f}")
    print(f"Mann-Whitney U: U={rf_u:.1f}, p={rf_u_p:.6f}")
    print(f"Cohen's d (rising - falling): {rf_d:.4f}")

    # Comparison with Au node:
    # Au: rising trend -> higher eff.dim (散开), falling trend -> lower eff.dim (压缩)
    # $ node: SAME_AS_AU (rising DXY -> higher eff.dim, falling DXY -> lower eff.dim)
    # Equity prediction if "普通节点": same pattern — rising SPX -> higher eff.dim, falling SPX -> lower eff.dim
    if rf_p < 0.05:
        if rf_d > 0:
            asym_verdict = "SAME_AS_AU"
            asym_desc = "上升散开/下降压缩：与Au节点同向 -> K4一般性质"
        else:
            asym_verdict = "OPPOSITE_TO_AU"
            asym_desc = "上升压缩/下降散开：与Au节点反向 -> 节点特异性"
    else:
        asym_verdict = "NO_ASYMMETRY"
        asym_desc = f"无显著不对称 (p={rf_p:.4f}) -> 不对称可能是Au特异或效应量不足"
    print(f"\nASYMMETRY VERDICT: {asym_verdict}")
    print(f"  {asym_desc}")
else:
    rf_t, rf_p, rf_d = float('nan'), float('nan'), float('nan')
    asym_verdict = "INSUFFICIENT_DATA"
    asym_desc = "数据不足"

# Within-trend drift (each trend segment)
print("\n--- Within-trend eff.dim drift ---")
drift_records = []
all_trend_segs = [(s, e, n, "rising") for s, e, n in manual_rising_periods] + \
                 [(s, e, n, "falling") for s, e, n in manual_falling_periods]
for start, end, name, direction in all_trend_segs:
    mask = (eff_dim_series.index >= start) & (eff_dim_series.index <= end)
    seg = eff_dim_series[mask]
    if len(seg) < 20:
        print(f"  {name} ({direction}): n={len(seg)}, SKIPPED (< 20 obs)")
        drift_records.append({
            "segment": name,
            "direction": direction,
            "n": len(seg),
            "first_half_mean": None,
            "second_half_mean": None,
            "spearman_r": None,
            "spearman_p": None,
            "note": "skipped (< 20 obs)",
        })
        continue
    mid = len(seg) // 2
    first_half = seg.iloc[:mid]
    second_half = seg.iloc[mid:]
    time_idx = np.arange(len(seg))
    sp_r, sp_p = stats.spearmanr(time_idx, seg.values)
    print(f"  {name} ({direction}):")
    print(f"    First half:  n={len(first_half)}, mean={first_half.mean():.3f}")
    print(f"    Second half: n={len(second_half)}, mean={second_half.mean():.3f}")
    print(f"    Spearman (eff.dim vs time): r={sp_r:+.4f}, p={sp_p:.4f}")
    drift_records.append({
        "segment": name,
        "direction": direction,
        "n": len(seg),
        "first_half_mean": round(float(first_half.mean()), 3),
        "second_half_mean": round(float(second_half.mean()), 3),
        "spearman_r": round(float(sp_r), 4),
        "spearman_p": round(float(sp_p), 4),
    })

# ============================================================
# 8. PROPOSITION 2: eff.dim x fold depth (Mahalanobis)
# ============================================================
print("\n" + "=" * 70)
print("PROPOSITION 2: Level modulates fold depth")
print("=" * 70)

# Extreme events (top 1% Mahalanobis, 7-day clustering)
threshold = mahal_series.quantile(0.99)
extreme_mask = mahal_series > threshold
extreme_dates = mahal_series[extreme_mask].index.tolist()

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
corr_results = {}
for label, ed, mh in [("pre_5d", effdims_5d, mahals),
                        ("pre_10d", effdims_10d, mahals_10d),
                        ("pre_20d", effdims_20d, mahals_20d)]:
    if len(ed) < 5:
        continue
    sp_r, sp_p_two = stats.spearmanr(ed, mh)
    sp_p_one = sp_p_two / 2 if sp_r > 0 else 1 - sp_p_two / 2
    pr_r, pr_p_two = stats.pearsonr(ed, mh)
    pr_p_one = pr_p_two / 2 if pr_r > 0 else 1 - pr_p_two / 2
    print(f"  {label}: Spearman rho={sp_r:+.4f}, p(one-sided)={sp_p_one:.4f} | "
          f"Pearson r={pr_r:+.4f}, p(one-sided)={pr_p_one:.4f}")
    corr_results[label] = {
        "spearman_rho": round(sp_r, 4),
        "spearman_p_one": round(sp_p_one, 4),
        "pearson_r": round(pr_r, 4),
        "pearson_p_one": round(pr_p_one, 4),
    }

# Robustness: remove top 2 extreme
print("\n--- Robustness: Remove 2 most extreme Mahal events ---")
if len(event_records) >= 3:
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

# Median split
if len(effdims_5d) >= 4:
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
else:
    median_ed = float('nan')
    high_m, low_m = [], []

# Proposition 2 verdict
sp5 = corr_results.get("pre_5d", {})
prop2_pass = sp5.get("spearman_rho", 0) > 0 and sp5.get("spearman_p_one", 1) < 0.05
print(f"\nPROPOSITION 2 VERDICT: {'PASS' if prop2_pass else 'FAIL'}")

# ============================================================
# 9. EVENT DENSITY: T vs N periods
# ============================================================
print("\n" + "=" * 70)
print("EVENT DENSITY: T vs N periods (manual annotation)")
print("=" * 70)

# Label returns by manual trend
ret_labels = pd.Series("X", index=returns.index)
for start, end, state, name in manual_trend_periods:
    mask = (returns.index >= start) & (returns.index <= end)
    ret_labels[mask] = state

t_days = (ret_labels == "T").sum()
n_days = (ret_labels == "N").sum()
print(f"\nTrading days: T={t_days}, N={n_days}")

# Classify events
t_events_list = []
n_events_list = []
other_events_list = []

for ev in events:
    start = ev["start"]
    label = ret_labels.get(start, "X")
    if label == "T":
        t_events_list.append(ev)
    elif label == "N":
        n_events_list.append(ev)
    else:
        other_events_list.append(ev)

print(f"\nEvents in T: {len(t_events_list)}")
for ev in t_events_list:
    print(f"  {ev['start'].date()} (Mahal={ev['max_mahal']:.1f})")
print(f"\nEvents in N: {len(n_events_list)}")
for ev in n_events_list:
    print(f"  {ev['start'].date()} (Mahal={ev['max_mahal']:.1f})")
if other_events_list:
    print(f"\nEvents unlabeled: {len(other_events_list)}")

t_rate = len(t_events_list) / t_days * 1000 if t_days > 0 else 0
n_rate = len(n_events_list) / n_days * 1000 if n_days > 0 else 0

print(f"\nEvent rate (per 1000 trading days):")
print(f"  T periods: {t_rate:.2f}")
print(f"  N periods: {n_rate:.2f}")
if t_rate > 0:
    print(f"  Ratio N/T: {n_rate/t_rate:.2f}")

# Fisher exact test
table = [[len(t_events_list), t_days - len(t_events_list)],
         [len(n_events_list), n_days - len(n_events_list)]]
odds_ratio, fisher_p = stats.fisher_exact(table, alternative="less")
print(f"\nFisher exact test (T rate < N rate):")
print(f"  odds ratio = {odds_ratio:.4f}, p = {fisher_p:.4f}")

chi2, chi_p, dof, expected = stats.chi2_contingency(table)
print(f"Chi-square: chi2={chi2:.4f}, p={chi_p:.4f}")

# Per-segment breakdown
print("\n--- Per-segment event count ---")
for start, end, state, name in manual_trend_periods:
    seg_mask = (returns.index >= start) & (returns.index <= end)
    seg_days = seg_mask.sum()
    seg_ev_count = 0
    for ev in events:
        if pd.Timestamp(start) <= ev["start"] <= pd.Timestamp(end):
            seg_ev_count += 1
    rate = seg_ev_count / seg_days * 1000 if seg_days > 0 else 0
    print(f"  {name:<28} ({state}): {seg_days:>4} days, {seg_ev_count} events, rate={rate:.2f}/1000d")

# ============================================================
# 10. THRESHOLD EFFECT: eff.dim lower bound for extreme events
# ============================================================
print("\n" + "=" * 70)
print("THRESHOLD EFFECT: eff.dim at time of extreme events")
print("=" * 70)

event_effdims = []
for ev in events:
    start = ev["start"]
    candidates = eff_dim_series.index[eff_dim_series.index <= start]
    if len(candidates) == 0:
        continue
    ed = float(eff_dim_series[candidates[-1]])
    event_effdims.append({
        "event_start": str(start.date()),
        "max_mahal": ev["max_mahal"],
        "eff_dim_at_event": round(ed, 3),
        "label": ret_labels.get(start, "?"),
    })

print(f"\n{'Event':<14} {'Mahal':>7} {'EffDim':>7} {'State':>6}")
print("-" * 40)
for r in event_effdims:
    print(f"{r['event_start']:<14} {r['max_mahal']:>7.1f} {r['eff_dim_at_event']:>7.3f} {r['label']:>6}")

eds = [r["eff_dim_at_event"] for r in event_effdims]
print(f"\nEff.dim at extreme events:")
print(f"  min = {min(eds):.3f}")
print(f"  max = {max(eds):.3f}")
print(f"  mean = {np.mean(eds):.3f}")
print(f"  median = {np.median(eds):.3f}")
print(f"  std = {np.std(eds):.3f}")

overall_min = eff_dim_series.min()
overall_max = eff_dim_series.max()
overall_mean = eff_dim_series.mean()
print(f"\nOverall eff.dim distribution:")
print(f"  min = {overall_min:.3f}")
print(f"  max = {overall_max:.3f}")
print(f"  mean = {overall_mean:.3f}")

min_event_ed = min(eds)
frac_below = (eff_dim_series < min_event_ed).mean()
print(f"\nThreshold analysis:")
print(f"  Minimum eff.dim at any extreme event: {min_event_ed:.3f}")
print(f"  Fraction of all days with eff.dim < {min_event_ed:.3f}: {frac_below:.4f} ({frac_below*100:.2f}%)")

for pct in [0, 5, 10, 25]:
    if pct == 0:
        threshold_ed = min_event_ed
    else:
        threshold_ed = np.percentile(eds, pct)
    frac = (eff_dim_series < threshold_ed).mean()
    print(f"  Event eff.dim p{pct}: {threshold_ed:.3f} -> {frac*100:.2f}% of all days below this")

# Statistical test: event eff.dims vs overall
overall_sample = eff_dim_series.values
thresh_t_stat, thresh_t_p_two = stats.ttest_ind(eds, overall_sample, equal_var=False)
thresh_t_p_one = thresh_t_p_two / 2 if thresh_t_stat > 0 else 1 - thresh_t_p_two / 2
print(f"\nWelch t-test (event eff.dim > overall): t={thresh_t_stat:.4f}, p(one-sided)={thresh_t_p_one:.4f}")

mw_stat, mw_p_thresh = stats.mannwhitneyu(eds, overall_sample, alternative="greater")
print(f"Mann-Whitney U (event eff.dim > overall): U={mw_stat:.1f}, p={mw_p_thresh:.4f}")

print("\n--- Cumulative event count by eff.dim threshold ---")
for thresh in [1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.0, 2.1, 2.2]:
    n_below = sum(1 for e in eds if e < thresh)
    frac_overall_below = (eff_dim_series < thresh).mean() * 100
    print(f"  eff.dim < {thresh:.1f}: {n_below}/{len(eds)} events ({n_below/len(eds)*100:.0f}%), "
          f"but {frac_overall_below:.1f}% of all days")

# Au and $ comparison
print(f"\n--- Cross-node comparison (Au vs $ vs Equity) ---")
print(f"  Au node:     min event eff.dim = 1.894, frac below = 6.92%")
print(f"  $  node:     min event eff.dim = 1.894, frac below = 6.73%")
print(f"  Equity node: min event eff.dim = {min_event_ed:.3f}, frac below = {frac_below*100:.2f}%")

# ============================================================
# 11. SAVE RESULTS
# ============================================================
results = {
    "node": "equity",
    "edges": ["SPX/DXY (Equity/$)", "SPX/GC=F (Equity/Au)", "SPX/CL=F (Equity/Commodity)"],
    "data_range": f"{returns.index[0].date()} to {returns.index[-1].date()}",
    "n_obs": len(returns),
    "eff_dim_summary": {
        "n_pts": len(eff_dim_series),
        "mean": round(float(eff_dim_series.mean()), 3),
        "std": round(float(eff_dim_series.std()), 3),
        "min": round(float(eff_dim_series.min()), 3),
        "max": round(float(eff_dim_series.max()), 3),
    },
    "proposition_1": {
        "method_a_ma_crossover": prop1_auto,
        "method_b_manual": prop1_manual,
        "robustness": "PASS" if prop1_auto["verdict"] == prop1_manual["verdict"] else "SENSITIVE_TO_METHOD",
    },
    "asymmetry": {
        "rising_spx": {
            "n": len(rising_data),
            "mean": round(float(rising_data.mean()), 3),
            "median": round(float(rising_data.median()), 3),
        },
        "falling_spx": {
            "n": len(falling_data),
            "mean": round(float(falling_data.mean()), 3),
            "median": round(float(falling_data.median()), 3),
        },
        "welch_t": round(float(rf_t), 4) if not np.isnan(rf_t) else None,
        "welch_p": round(float(rf_p), 6) if not np.isnan(rf_p) else None,
        "cohens_d": round(float(rf_d), 4) if not np.isnan(rf_d) else None,
        "drift_by_segment": drift_records,
        "verdict": asym_verdict,
        "description": asym_desc,
        "au_dollar_comparison": {
            "au_pattern": "rising_gold=散开(higher eff.dim), falling_gold=压缩(lower eff.dim)",
            "dollar_pattern": "rising_dxy=散开(higher eff.dim), falling_dxy=压缩(lower eff.dim) [SAME_AS_AU]",
            "equity_pattern": f"rising_spx mean={rising_data.mean():.3f}, falling_spx mean={falling_data.mean():.3f}",
        },
    },
    "proposition_2": {
        "n_events": len(event_records),
        "events": event_records,
        "correlations": corr_results,
        "median_split": {
            "median_effdim": round(float(median_ed), 3) if not np.isnan(median_ed) else None,
            "high_group_mean_mahal": round(float(np.mean(high_m)), 2) if high_m else None,
            "low_group_mean_mahal": round(float(np.mean(low_m)), 2) if low_m else None,
        },
        "verdict": "PASS" if prop2_pass else "FAIL",
    },
    "event_density": {
        "T_days": int(t_days),
        "N_days": int(n_days),
        "T_events": len(t_events_list),
        "N_events": len(n_events_list),
        "T_rate_per_1000d": round(t_rate, 2),
        "N_rate_per_1000d": round(n_rate, 2),
        "ratio_N_over_T": round(n_rate / t_rate, 2) if t_rate > 0 else None,
        "fisher_exact_p": round(fisher_p, 4),
        "chi2_p": round(chi_p, 4),
    },
    "threshold_effect": {
        "event_effdims": event_effdims,
        "min_event_effdim": round(min(eds), 3),
        "max_event_effdim": round(max(eds), 3),
        "mean_event_effdim": round(float(np.mean(eds)), 3),
        "overall_mean_effdim": round(float(overall_mean), 3),
        "frac_days_below_min_event": round(float(frac_below), 4),
        "welch_t_p_one_sided": round(float(thresh_t_p_one), 4),
        "mann_whitney_p": round(float(mw_p_thresh), 4),
        "three_node_comparison": {
            "au_min_event_effdim": 1.894,
            "au_frac_below": 0.0692,
            "dollar_min_event_effdim": 1.894,
            "dollar_frac_below": 0.0673,
            "equity_min_event_effdim": round(min(eds), 3),
            "equity_frac_below": round(float(frac_below), 4),
        },
    },
}

# Final verdicts
p1_auto_v = prop1_auto["verdict"]
p1_manual_v = prop1_manual["verdict"]
p2v = results["proposition_2"]["verdict"]

# Combined verdict
if p1_auto_v == "PASS" and p1_manual_v == "PASS":
    p1_combined = "PASS (robust across methods)"
elif p1_auto_v == "PASS" or p1_manual_v == "PASS":
    p1_combined = f"PARTIAL (MA={p1_auto_v}, Manual={p1_manual_v})"
else:
    p1_combined = "FAIL"

# Threshold convergence across 3 nodes
threshold_converge = (
    "THREE_NODE_CONVERGENCE"
    if frac_below > 0.03  # meaningful exclusion zone exists
    else "EQUITY_NO_THRESHOLD"
)

results["final_verdicts"] = {
    "asymmetry": asym_verdict,
    "asymmetry_description": asym_desc,
    "threshold_min_event_effdim": round(min(eds), 3),
    "threshold_convergence": threshold_converge,
    "proposition_1": p1_combined,
    "proposition_2": p2v,
    "question_1_answer": (
        "K4一般性质（三节点一致）" if asym_verdict == "SAME_AS_AU" else
        "Equity节点特异" if asym_verdict == "OPPOSITE_TO_AU" else
        "证据不足（无显著不对称或数据不足）"
    ),
    "question_2_answer": (
        "三节点汇合确认" if frac_below > 0.03 else
        "Au/$特有，Equity不具备" if frac_below < 0.01 else
        "证据不足"
    ),
}

output_path = OUTPUT_DIR / "r3_equity_node_results.json"
with open(output_path, "w", encoding="utf-8") as f:
    json.dump(results, f, indent=2, ensure_ascii=False)

print(f"\n{'='*70}")
print("FINAL VERDICTS")
print(f"{'='*70}")
print(f"Proposition 1 (eff.dim is level measure):")
print(f"  MA crossover: {p1_auto_v}")
print(f"  Manual annotation: {p1_manual_v}")
print(f"  Combined: {p1_combined}")
print(f"\nProposition 2 (level modulates fold depth): {p2v}")
print(f"\nAsymmetry (Q1 - 不对称性来源):")
print(f"  Verdict: {asym_verdict}")
print(f"  {asym_desc}")
print(f"  Au:     rising=散开, falling=压缩 (SAME_AS_AU)")
print(f"  $:      rising=散开, falling=压缩 (SAME_AS_AU)")
print(f"  Equity: rising mean={rising_data.mean():.3f}, falling mean={falling_data.mean():.3f}")
print(f"\nThreshold (Q2 - 阈值效应确认):")
print(f"  Au node:     min event eff.dim = 1.894, frac below = 6.92%")
print(f"  $  node:     min event eff.dim = 1.894, frac below = 6.73%")
print(f"  Equity node: min event eff.dim = {min_event_ed:.3f}, frac below = {frac_below*100:.2f}%")
print(f"  Convergence: {threshold_converge}")
print(f"  Answer: {results['final_verdicts']['question_2_answer']}")
print(f"\nResults saved to {output_path}")
