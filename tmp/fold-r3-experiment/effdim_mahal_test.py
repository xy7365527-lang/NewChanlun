"""
321号下游推论1检验：eff.dim 与 折叠深度（Mahalanobis距离）的相关性

假设：同等外部冲击打在高 eff.dim 系统上产生更深的折叠。
      → 极端事件的 Mahalanobis 距离应与事件前的 eff.dim 正相关。

方法：
1. 从 agent_a_analysis.py 的数据管线中重建 rolling eff.dim 序列
2. 对每个极端事件，取事件前一天的 eff.dim（避免前视偏差）
3. 计算 Pearson/Spearman 相关 + 散点分析
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
print("Downloading data...")
gold = yf.download("GC=F", start="2000-01-01", end="2026-03-01")['Close']
copper = yf.download("HG=F", start="2000-01-01", end="2026-03-01")['Close']
dxy = yf.download("DX-Y.NYB", start="2000-01-01", end="2026-03-01")['Close']
spx = yf.download("^GSPC", start="2000-01-01", end="2026-03-01")['Close']

df = pd.DataFrame({'gold': gold, 'copper': copper, 'dxy': dxy, 'spx': spx}).dropna()
e1 = df['gold'] / df['dxy']
e2 = df['copper'] / df['gold']
e3 = df['gold'] / df['spx']

returns = pd.DataFrame({
    'e1': np.log(e1).diff(),
    'e2': np.log(e2).diff(),
    'e3': np.log(e3).diff()
}).dropna()

print(f"Data: {len(returns)} observations, {returns.index[0].date()} to {returns.index[-1].date()}")

# ============================================================
# 2. Rolling Effective Dimensionality (252d window)
# ============================================================
print("Computing rolling effective dimensionality...")
window = 252
eff_dims = {}

for i in range(window, len(returns)):
    w = returns.iloc[i - window:i]
    cov = w.cov().values
    eigvals = np.linalg.eigvalsh(cov)
    eigvals = eigvals[eigvals > 0]
    p = eigvals / eigvals.sum()
    entropy = -np.sum(p * np.log(p))
    eff_dim = np.exp(entropy)
    eff_dims[returns.index[i]] = eff_dim

eff_dim_series = pd.Series(eff_dims)
print(f"Eff.dim series: {len(eff_dim_series)} points, mean={eff_dim_series.mean():.3f}, std={eff_dim_series.std():.3f}")

# ============================================================
# 3. Mahalanobis Distance for Each Day
# ============================================================
print("Computing Mahalanobis distances...")
mean_ret = returns.mean().values
cov_ret = returns.cov().values
cov_inv = np.linalg.inv(cov_ret)

mahal = []
for i, (idx, row) in enumerate(returns.iterrows()):
    diff = row.values - mean_ret
    d = np.sqrt(diff @ cov_inv @ diff)
    mahal.append(d)

mahal_series = pd.Series(mahal, index=returns.index)

# ============================================================
# 4. Identify Extreme Events (top 1% Mahalanobis)
# ============================================================
threshold = mahal_series.quantile(0.99)
extreme_mask = mahal_series > threshold
print(f"Mahalanobis threshold (99th pct): {threshold:.2f}")
print(f"Extreme days: {extreme_mask.sum()}")

# Cluster extreme days (gap > 5 trading days = new event)
extreme_dates = mahal_series[extreme_mask].index.tolist()
events = []
if extreme_dates:
    current_event = {'start': extreme_dates[0], 'end': extreme_dates[0],
                     'max_mahal': mahal_series[extreme_dates[0]], 'days': 1}
    for d in extreme_dates[1:]:
        if (d - current_event['end']).days <= 7:  # 5 trading days ≈ 7 calendar days
            current_event['end'] = d
            current_event['max_mahal'] = max(current_event['max_mahal'], mahal_series[d])
            current_event['days'] += 1
        else:
            events.append(current_event)
            current_event = {'start': d, 'end': d,
                           'max_mahal': mahal_series[d], 'days': 1}
    events.append(current_event)

print(f"Clustered into {len(events)} events")

# ============================================================
# 5. For Each Event: Get Pre-Event Eff.Dim
# ============================================================
print("\nEvent-level analysis:")
print(f"{'Event Start':<14} {'Days':>5} {'MaxMahal':>9} {'Pre-EffDim':>11} {'Pre5d-EffDim':>13}")
print("-" * 60)

event_records = []
for ev in events:
    start = ev['start']

    # Get eff.dim just before the event (1 day before start)
    pre_dates = eff_dim_series.index[eff_dim_series.index < start]
    if len(pre_dates) == 0:
        continue

    # Single day before
    pre_1d = eff_dim_series[pre_dates[-1]]

    # Average of 5 days before (more stable)
    pre_5d_dates = pre_dates[-5:]
    pre_5d = eff_dim_series[pre_5d_dates].mean()

    # Average of 20 days before (monthly)
    pre_20d_dates = pre_dates[-20:]
    pre_20d = eff_dim_series[pre_20d_dates].mean()

    record = {
        'event_start': str(start.date()),
        'event_end': str(ev['end'].date()),
        'days': ev['days'],
        'max_mahal': round(float(ev['max_mahal']), 2),
        'pre_eff_dim_1d': round(float(pre_1d), 3),
        'pre_eff_dim_5d': round(float(pre_5d), 3),
        'pre_eff_dim_20d': round(float(pre_20d), 3),
    }
    event_records.append(record)

    print(f"{record['event_start']:<14} {record['days']:>5} {record['max_mahal']:>9.2f} "
          f"{record['pre_eff_dim_1d']:>11.3f} {record['pre_eff_dim_5d']:>13.3f}")

# ============================================================
# 6. Correlation Analysis
# ============================================================
print("\n" + "=" * 60)
print("CORRELATION ANALYSIS: Mahalanobis distance vs pre-event eff.dim")
print("=" * 60)

if len(event_records) >= 5:
    mahals = [r['max_mahal'] for r in event_records]
    effdims_1d = [r['pre_eff_dim_1d'] for r in event_records]
    effdims_5d = [r['pre_eff_dim_5d'] for r in event_records]
    effdims_20d = [r['pre_eff_dim_20d'] for r in event_records]

    for label, effdims in [('1d pre-event', effdims_1d),
                            ('5d pre-event avg', effdims_5d),
                            ('20d pre-event avg', effdims_20d)]:
        pearson_r, pearson_p = stats.pearsonr(effdims, mahals)
        spearman_r, spearman_p = stats.spearmanr(effdims, mahals)

        print(f"\n--- {label} eff.dim ---")
        print(f"Pearson:  r={pearson_r:+.4f}, p={pearson_p:.4f}")
        print(f"Spearman: r={spearman_r:+.4f}, p={spearman_p:.4f}")

    # Also: split into high/low eff.dim groups
    median_effdim = np.median(effdims_5d)
    high_group = [r for r in event_records if r['pre_eff_dim_5d'] >= median_effdim]
    low_group = [r for r in event_records if r['pre_eff_dim_5d'] < median_effdim]

    high_mahals = [r['max_mahal'] for r in high_group]
    low_mahals = [r['max_mahal'] for r in low_group]

    print(f"\n--- Median Split (median eff.dim = {median_effdim:.3f}) ---")
    print(f"High eff.dim group (n={len(high_group)}): mean Mahal = {np.mean(high_mahals):.2f}, std = {np.std(high_mahals):.2f}")
    print(f"Low  eff.dim group (n={len(low_group)}):  mean Mahal = {np.mean(low_mahals):.2f}, std = {np.std(low_mahals):.2f}")

    if len(high_mahals) >= 3 and len(low_mahals) >= 3:
        mwu_stat, mwu_p = stats.mannwhitneyu(high_mahals, low_mahals, alternative='greater')
        print(f"Mann-Whitney U (high > low): U={mwu_stat:.1f}, p={mwu_p:.4f}")

        ttest_stat, ttest_p = stats.ttest_ind(high_mahals, low_mahals, alternative='greater')
        print(f"Welch t-test (high > low):   t={ttest_stat:.3f}, p={ttest_p:.4f}")

    # Tertile split for more granularity
    t1 = np.percentile(effdims_5d, 33)
    t2 = np.percentile(effdims_5d, 67)

    low_t = [r for r in event_records if r['pre_eff_dim_5d'] <= t1]
    mid_t = [r for r in event_records if t1 < r['pre_eff_dim_5d'] <= t2]
    high_t = [r for r in event_records if r['pre_eff_dim_5d'] > t2]

    print(f"\n--- Tertile Split ---")
    for label, group in [('Low', low_t), ('Mid', mid_t), ('High', high_t)]:
        if group:
            gm = [r['max_mahal'] for r in group]
            ge = [r['pre_eff_dim_5d'] for r in group]
            print(f"{label} (n={len(group)}, eff.dim {min(ge):.2f}-{max(ge):.2f}): "
                  f"mean Mahal = {np.mean(gm):.2f}, median = {np.median(gm):.2f}")

# ============================================================
# 7. Save Results
# ============================================================
results = {
    'hypothesis': 'Mahalanobis distance (fold depth) positively correlates with pre-event effective dimensionality',
    'n_events': len(event_records),
    'events': event_records,
    'correlations': {},
    'group_comparison': {}
}

if len(event_records) >= 5:
    for label, effdims in [('1d', effdims_1d), ('5d', effdims_5d), ('20d', effdims_20d)]:
        pr, pp = stats.pearsonr(effdims, mahals)
        sr, sp = stats.spearmanr(effdims, mahals)
        results['correlations'][label] = {
            'pearson_r': round(pr, 4), 'pearson_p': round(pp, 4),
            'spearman_r': round(sr, 4), 'spearman_p': round(sp, 4)
        }

    results['group_comparison'] = {
        'median_effdim': round(float(median_effdim), 3),
        'high_group': {'n': len(high_group), 'mean_mahal': round(np.mean(high_mahals), 2)},
        'low_group': {'n': len(low_group), 'mean_mahal': round(np.mean(low_mahals), 2)}
    }
    if len(high_mahals) >= 3 and len(low_mahals) >= 3:
        mwu_stat, mwu_p = stats.mannwhitneyu(high_mahals, low_mahals, alternative='greater')
        results['group_comparison']['mann_whitney_u'] = round(float(mwu_stat), 1)
        results['group_comparison']['mann_whitney_p'] = round(float(mwu_p), 4)

output_path = OUTPUT_DIR / 'effdim_mahal_correlation.json'
with open(output_path, 'w') as f:
    json.dump(results, f, indent=2, ensure_ascii=False)

print(f"\nResults saved to {output_path}")
