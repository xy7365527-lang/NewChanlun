"""
R3 附加检验——编排者追加分析
=================================
检查1：T时段 vs N时段的极端事件密度差异
检查2：阈值效应——极端事件发生时 eff.dim 是否全部高于某个下界

数据来源：r3_addendum_test.py 已重建的 eff_dim_series + mahal_series + trend_periods
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
# 1. Data Acquisition (same pipeline)
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

# ============================================================
# 3. Mahalanobis Distance
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
# 4. Trend periods (Agent B)
# ============================================================
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

# Label each day
labels = pd.Series(index=returns.index, dtype=str)
for start, end, state, name in trend_periods:
    mask = (returns.index >= start) & (returns.index <= end)
    labels[mask] = state

# ============================================================
# 5. Extreme events
# ============================================================
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

# ============================================================
# CHECK 1: Event density in T vs N periods
# ============================================================
print("\n" + "=" * 70)
print("CHECK 1: Extreme event density in T vs N periods")
print("=" * 70)

# Count trading days in T and N
t_days = labels[labels == "T"].shape[0]
n_days = labels[labels == "N"].shape[0]
unlabeled_days = labels[~labels.isin(["T", "N"])].shape[0]

print(f"\nTrading days: T={t_days}, N={n_days}, unlabeled={unlabeled_days}")

# Classify each event into T or N (by start date)
t_events = []
n_events = []
other_events = []

for ev in events:
    start = ev["start"]
    if labels.get(start, "") == "T":
        t_events.append(ev)
    elif labels.get(start, "") == "N":
        n_events.append(ev)
    else:
        other_events.append(ev)

print(f"\nEvents in T: {len(t_events)}")
for ev in t_events:
    print(f"  {ev['start'].date()} (Mahal={ev['max_mahal']:.1f})")

print(f"\nEvents in N: {len(n_events)}")
for ev in n_events:
    print(f"  {ev['start'].date()} (Mahal={ev['max_mahal']:.1f})")

if other_events:
    print(f"\nEvents unlabeled: {len(other_events)}")
    for ev in other_events:
        print(f"  {ev['start'].date()} (Mahal={ev['max_mahal']:.1f})")

# Event rate per 1000 trading days
t_rate = len(t_events) / t_days * 1000 if t_days > 0 else 0
n_rate = len(n_events) / n_days * 1000 if n_days > 0 else 0

print(f"\nEvent rate (per 1000 trading days):")
print(f"  T periods: {t_rate:.2f}")
print(f"  N periods: {n_rate:.2f}")
print(f"  Ratio N/T: {n_rate/t_rate:.2f}" if t_rate > 0 else "  T rate is 0")

# Fisher exact test (2x2: events/non-events × T/N)
# T: t_events events, t_days - t_events non-events
# N: n_events events, n_days - n_events non-events
table = [[len(t_events), t_days - len(t_events)],
         [len(n_events), n_days - len(n_events)]]
odds_ratio, fisher_p = stats.fisher_exact(table, alternative="less")
print(f"\nFisher exact test (T rate < N rate):")
print(f"  odds ratio = {odds_ratio:.4f}, p = {fisher_p:.4f}")

# Also try chi-square
chi2, chi_p, dof, expected = stats.chi2_contingency(table)
print(f"Chi-square: χ²={chi2:.4f}, p={chi_p:.4f}")

# Per-segment breakdown
print("\n--- Per-segment event count ---")
for start, end, state, name in trend_periods:
    seg_mask = (returns.index >= start) & (returns.index <= end)
    seg_days = seg_mask.sum()
    seg_ev_count = 0
    for ev in events:
        if pd.Timestamp(start) <= ev["start"] <= pd.Timestamp(end):
            seg_ev_count += 1
    rate = seg_ev_count / seg_days * 1000 if seg_days > 0 else 0
    print(f"  {name:<16} ({state}): {seg_days:>4} days, {seg_ev_count} events, rate={rate:.2f}/1000d")

# ============================================================
# CHECK 2: Threshold effect — eff.dim lower bound for events
# ============================================================
print("\n" + "=" * 70)
print("CHECK 2: Threshold effect — eff.dim at time of extreme events")
print("=" * 70)

# For each event, get eff.dim AT the event (not pre-event)
event_effdims = []
for ev in events:
    start = ev["start"]
    # Get closest eff.dim at or before event start
    candidates = eff_dim_series.index[eff_dim_series.index <= start]
    if len(candidates) == 0:
        continue
    ed = float(eff_dim_series[candidates[-1]])
    event_effdims.append({
        "event_start": str(start.date()),
        "max_mahal": ev["max_mahal"],
        "eff_dim_at_event": round(ed, 3),
        "label": labels.get(start, "?"),
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

# Compare with overall distribution
overall_min = eff_dim_series.min()
overall_max = eff_dim_series.max()
overall_mean = eff_dim_series.mean()

print(f"\nOverall eff.dim distribution:")
print(f"  min = {overall_min:.3f}")
print(f"  max = {overall_max:.3f}")
print(f"  mean = {overall_mean:.3f}")

# Threshold analysis: what fraction of overall eff.dim observations
# are below the minimum event eff.dim?
min_event_ed = min(eds)
frac_below = (eff_dim_series < min_event_ed).mean()
print(f"\nThreshold analysis:")
print(f"  Minimum eff.dim at any extreme event: {min_event_ed:.3f}")
print(f"  Fraction of all days with eff.dim < {min_event_ed:.3f}: {frac_below:.4f} ({frac_below*100:.2f}%)")

# More granular: check 5th, 10th, 25th percentile of event eff.dims
for pct in [0, 5, 10, 25]:
    if pct == 0:
        threshold_ed = min_event_ed
    else:
        threshold_ed = np.percentile(eds, pct)
    frac = (eff_dim_series < threshold_ed).mean()
    print(f"  Event eff.dim p{pct}: {threshold_ed:.3f} → {frac*100:.2f}% of all days below this")

# One-sided test: are event eff.dims drawn from a distribution
# with higher mean than overall?
overall_sample = eff_dim_series.values
t_stat, t_p_two = stats.ttest_ind(eds, overall_sample, equal_var=False)
t_p_one = t_p_two / 2 if t_stat > 0 else 1 - t_p_two / 2
print(f"\nWelch t-test (event eff.dim > overall): t={t_stat:.4f}, p(one-sided)={t_p_one:.4f}")

mw_stat, mw_p = stats.mannwhitneyu(eds, overall_sample, alternative="greater")
print(f"Mann-Whitney U (event eff.dim > overall): U={mw_stat:.1f}, p={mw_p:.4f}")

# Permutation test: are there any events below eff.dim = 1.7? 1.8? 1.9?
print("\n--- Cumulative event count by eff.dim threshold ---")
for thresh in [1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.0, 2.1, 2.2]:
    n_below = sum(1 for e in eds if e < thresh)
    frac_overall_below = (eff_dim_series < thresh).mean() * 100
    print(f"  eff.dim < {thresh:.1f}: {n_below}/{len(eds)} events ({n_below/len(eds)*100:.0f}%), "
          f"but {frac_overall_below:.1f}% of all days")

# ============================================================
# 7. Save Results
# ============================================================
results = {
    "check_1_event_density": {
        "T_days": t_days,
        "N_days": n_days,
        "T_events": len(t_events),
        "N_events": len(n_events),
        "T_rate_per_1000d": round(t_rate, 2),
        "N_rate_per_1000d": round(n_rate, 2),
        "ratio_N_over_T": round(n_rate / t_rate, 2) if t_rate > 0 else None,
        "fisher_exact_p": round(fisher_p, 4),
        "chi2_p": round(chi_p, 4),
    },
    "check_2_threshold": {
        "event_effdims": event_effdims,
        "min_event_effdim": round(min(eds), 3),
        "max_event_effdim": round(max(eds), 3),
        "mean_event_effdim": round(float(np.mean(eds)), 3),
        "overall_mean_effdim": round(float(overall_mean), 3),
        "frac_days_below_min_event": round(float(frac_below), 4),
        "welch_t_p_one_sided": round(float(t_p_one), 4),
        "mann_whitney_p": round(float(mw_p), 4),
    },
}

output_path = OUTPUT_DIR / "r3_density_threshold_results.json"
with open(output_path, "w", encoding="utf-8") as f:
    json.dump(results, f, indent=2, ensure_ascii=False)

print(f"\n{'='*70}")
print("SUMMARY")
print(f"{'='*70}")
print(f"Check 1 - Event density: T={t_rate:.2f}/1000d, N={n_rate:.2f}/1000d, "
      f"ratio N/T={n_rate/t_rate:.2f}" if t_rate > 0 else "T rate=0")
print(f"  Fisher p={fisher_p:.4f}")
print(f"Check 2 - Threshold: min event eff.dim={min(eds):.3f}, "
      f"{frac_below*100:.2f}% of days below this")
print(f"  Event eff.dim vs overall: Welch p={t_p_one:.4f}")
print(f"\nResults saved to {output_path}")
