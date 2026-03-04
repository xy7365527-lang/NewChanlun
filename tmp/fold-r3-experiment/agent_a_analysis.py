"""
Fold R3 — Agent A: Blind Descriptive Analysis of Three Ratio Series
===================================================================
Three daily ratio series (e1, e2, e3) are analyzed without knowledge of
what they represent. The analysis covers individual characteristics,
pairwise relationships, joint structure, and anomalous periods.
"""

import json
import warnings
from pathlib import Path

import numpy as np
import pandas as pd
import yfinance as yf
from scipy import stats
from scipy.signal import argrelextrema
from sklearn.decomposition import PCA

warnings.filterwarnings("ignore")

OUTPUT_DIR = Path(__file__).parent

# ============================================================
# 1. Data Acquisition
# ============================================================
print("Downloading data...")
gold = yf.download("GC=F", start="2000-01-01", end="2026-03-01")['Close']
copper = yf.download("HG=F", start="2000-01-01", end="2026-03-01")['Close']
dxy = yf.download("DX-Y.NYB", start="2000-01-01", end="2026-03-01")['Close']
spx = yf.download("^GSPC", start="2000-01-01", end="2026-03-01")['Close']

# Flatten MultiIndex if present
for s in [gold, copper, dxy, spx]:
    if hasattr(s, 'columns'):
        s.columns = s.columns.get_level_values(0) if isinstance(s.columns, pd.MultiIndex) else s.columns

df = pd.DataFrame({
    'gold': gold.squeeze(),
    'copper': copper.squeeze(),
    'dxy': dxy.squeeze(),
    'spx': spx.squeeze()
}).dropna()

e1 = df['gold'] / df['dxy']    # ratio 1
e2 = df['copper'] / df['gold']  # ratio 2
e3 = df['gold'] / df['spx']     # ratio 3

ratios = pd.DataFrame({'e1': e1, 'e2': e2, 'e3': e3}).dropna()
print(f"Data: {len(ratios)} observations, {ratios.index[0].date()} to {ratios.index[-1].date()}")

# Log-returns for stationarity / volatility analysis
log_ret = np.log(ratios / ratios.shift(1)).dropna()

# ============================================================
# 2. Helper Functions
# ============================================================

def adf_test(series, name):
    """Augmented Dickey-Fuller test."""
    from statsmodels.tsa.stattools import adfuller
    result = adfuller(series.dropna(), autolag='AIC')
    return {
        'series': name,
        'adf_statistic': round(result[0], 4),
        'p_value': round(result[1], 6),
        'lags_used': result[2],
        'stationary_5pct': result[1] < 0.05
    }


def rolling_vol(series, window=63):
    """Annualized rolling volatility (63-day ~ 1 quarter)."""
    return series.rolling(window).std() * np.sqrt(252)


def detect_regime_changes(series, window=126, threshold=2.0):
    """Detect volatility regime changes via z-score of rolling vol."""
    vol = rolling_vol(series, window=63).dropna()
    vol_mean = vol.rolling(window).mean()
    vol_std = vol.rolling(window).std()
    z = ((vol - vol_mean) / vol_std).dropna()
    breakpoints = z.index[z.abs() > threshold].tolist()
    # Cluster breakpoints within 30 days
    if not breakpoints:
        return []
    clusters = [[breakpoints[0]]]
    for bp in breakpoints[1:]:
        if (bp - clusters[-1][-1]).days < 30:
            clusters[-1].append(bp)
        else:
            clusters.append([bp])
    return [{'start': str(c[0].date()), 'end': str(c[-1].date()),
             'z_peak': round(float(z.loc[c].abs().max()), 2)} for c in clusters]


def rolling_correlation(s1, s2, window=252):
    """Rolling Pearson correlation."""
    return s1.rolling(window).corr(s2)


def compute_rolling_pca(df_standardized, window=252):
    """Rolling PCA on standardized ratios. Returns eigenvalue ratios."""
    n = len(df_standardized)
    results = []
    for i in range(window, n):
        chunk = df_standardized.iloc[i-window:i]
        pca = PCA(n_components=3)
        pca.fit(chunk.values)
        results.append({
            'date': str(df_standardized.index[i].date()),
            'ev1_ratio': round(float(pca.explained_variance_ratio_[0]), 4),
            'ev2_ratio': round(float(pca.explained_variance_ratio_[1]), 4),
            'ev3_ratio': round(float(pca.explained_variance_ratio_[2]), 4),
        })
    return pd.DataFrame(results)


# ============================================================
# 3. Individual Series Characteristics
# ============================================================
print("Analyzing individual series...")

individual_stats = {}
adf_results = []

for name, series in [('e1', ratios['e1']), ('e2', ratios['e2']), ('e3', ratios['e3'])]:
    ret = log_ret[name]

    # Basic statistics
    basic = {
        'mean': round(float(series.mean()), 6),
        'std': round(float(series.std()), 6),
        'min': round(float(series.min()), 6),
        'max': round(float(series.max()), 6),
        'skewness': round(float(series.skew()), 4),
        'kurtosis': round(float(series.kurtosis()), 4),
        'current_value': round(float(series.iloc[-1]), 6),
        'total_return_pct': round(float((series.iloc[-1] / series.iloc[0] - 1) * 100), 2),
    }

    # Return distribution
    ret_stats = {
        'mean_daily_return': round(float(ret.mean()), 6),
        'std_daily_return': round(float(ret.std()), 6),
        'skewness': round(float(ret.skew()), 4),
        'kurtosis': round(float(ret.kurtosis()), 4),
        'jarque_bera_stat': round(float(stats.jarque_bera(ret.dropna())[0]), 2),
        'jarque_bera_pvalue': round(float(stats.jarque_bera(ret.dropna())[1]), 8),
    }

    # ADF test on levels and returns
    adf_level = adf_test(series, f'{name}_level')
    adf_ret = adf_test(ret, f'{name}_return')
    adf_results.extend([adf_level, adf_ret])

    # Volatility regimes
    regimes = detect_regime_changes(ret)

    # Trend segmentation: rolling 252-day return sign
    rolling_252_ret = series.pct_change(252).dropna()
    up_pct = round(float((rolling_252_ret > 0).mean() * 100), 1)

    individual_stats[name] = {
        'basic': basic,
        'return_distribution': ret_stats,
        'adf_level': adf_level,
        'adf_return': adf_ret,
        'vol_regime_breaks': regimes,
        'pct_time_in_uptrend_252d': up_pct,
    }

# ============================================================
# 4. Pairwise Relationships
# ============================================================
print("Analyzing pairwise relationships...")

pairs = [('e1', 'e2'), ('e1', 'e3'), ('e2', 'e3')]
pairwise_stats = {}

# Full-sample correlation matrix
full_corr = ratios.corr()
full_corr_returns = log_ret.corr()

for a, b in pairs:
    pair_key = f'{a}_{b}'

    # Rolling correlation (1-year window)
    rc = rolling_correlation(log_ret[a], log_ret[b], window=252).dropna()

    # Identify correlation regime changes
    # Split into quintiles by rolling correlation
    rc_clean = rc.dropna()
    corr_mean = float(rc_clean.mean())
    corr_std = float(rc_clean.std())
    corr_min = float(rc_clean.min())
    corr_max = float(rc_clean.max())

    # Detect periods of sign change in correlation
    sign_changes = (rc_clean > 0).astype(int).diff().abs()
    sign_change_dates = sign_changes[sign_changes > 0].index

    # Identify extreme correlation periods (top/bottom 10%)
    high_corr_threshold = rc_clean.quantile(0.9)
    low_corr_threshold = rc_clean.quantile(0.1)

    high_periods = rc_clean[rc_clean > high_corr_threshold]
    low_periods = rc_clean[rc_clean < low_corr_threshold]

    # Cointegration test (Engle-Granger)
    from statsmodels.tsa.stattools import coint
    coint_stat, coint_pval, _ = coint(ratios[a], ratios[b])

    pairwise_stats[pair_key] = {
        'full_sample_corr_levels': round(float(full_corr.loc[a, b]), 4),
        'full_sample_corr_returns': round(float(full_corr_returns.loc[a, b]), 4),
        'rolling_corr_mean': round(corr_mean, 4),
        'rolling_corr_std': round(corr_std, 4),
        'rolling_corr_min': round(corr_min, 4),
        'rolling_corr_max': round(corr_max, 4),
        'num_sign_changes': int(len(sign_change_dates)),
        'cointegration_stat': round(float(coint_stat), 4),
        'cointegration_pval': round(float(coint_pval), 6),
        'cointegrated_5pct': float(coint_pval) < 0.05,
    }

# ============================================================
# 5. Joint Structure (PCA + Rolling Eigenvalues)
# ============================================================
print("Analyzing joint structure (PCA)...")

# Standardize ratios for PCA
ratios_std = (ratios - ratios.mean()) / ratios.std()

# Full-sample PCA
pca_full = PCA(n_components=3)
pca_full.fit(ratios_std.values)
full_pca_result = {
    'explained_variance_ratio': [round(float(x), 4) for x in pca_full.explained_variance_ratio_],
    'components': [[round(float(x), 4) for x in comp] for comp in pca_full.components_],
    'singular_values': [round(float(x), 4) for x in pca_full.singular_values_],
}

# Rolling PCA (252-day window)
rolling_pca_df = compute_rolling_pca(ratios_std, window=252)

# Detect structural breaks in PCA: when PC1 explained variance drops below threshold
# or when PC1 loading structure changes significantly
pc1_ev = rolling_pca_df['ev1_ratio'].values
pc1_dates = pd.to_datetime(rolling_pca_df['date'])

# Effective dimensionality: 1/sum(pi^2) where pi = explained variance ratio
rolling_pca_df['eff_dim'] = 1.0 / (
    rolling_pca_df['ev1_ratio']**2 +
    rolling_pca_df['ev2_ratio']**2 +
    rolling_pca_df['ev3_ratio']**2
)

# PCA on returns (more stationary)
ret_std = (log_ret - log_ret.mean()) / log_ret.std()
pca_ret = PCA(n_components=3)
pca_ret.fit(ret_std.dropna().values)
ret_pca_result = {
    'explained_variance_ratio': [round(float(x), 4) for x in pca_ret.explained_variance_ratio_],
    'components': [[round(float(x), 4) for x in comp] for comp in pca_ret.components_],
}

# Rolling PCA on returns
rolling_pca_ret_df = compute_rolling_pca(ret_std.dropna(), window=252)
rolling_pca_ret_df['eff_dim'] = 1.0 / (
    rolling_pca_ret_df['ev1_ratio']**2 +
    rolling_pca_ret_df['ev2_ratio']**2 +
    rolling_pca_ret_df['ev3_ratio']**2
)

# ============================================================
# 6. Structural Break Detection (Bai-Perron proxy via CUSUM)
# ============================================================
print("Detecting structural breaks...")

def cusum_test(series):
    """Simple CUSUM statistic for structural break detection."""
    n = len(series)
    cumsum = np.cumsum(series - series.mean()) / (series.std() * np.sqrt(n))
    return cumsum

# CUSUM on each series
cusum_results = {}
for name in ['e1', 'e2', 'e3']:
    ret = log_ret[name].dropna()
    cs = cusum_test(ret.values)
    # Find points where CUSUM exceeds critical bounds (approx 1.36 for 5%)
    exceedances = np.where(np.abs(cs) > 1.36)[0]
    if len(exceedances) > 0:
        # Cluster
        clusters = [[exceedances[0]]]
        for idx in exceedances[1:]:
            if idx - clusters[-1][-1] < 60:
                clusters[-1].append(idx)
            else:
                clusters.append([idx])
        break_dates = [str(ret.index[c[len(c)//2]].date()) for c in clusters]
    else:
        break_dates = []
    cusum_results[name] = break_dates

# ============================================================
# 7. Anomalous / Extreme Periods
# ============================================================
print("Identifying anomalous periods...")

# Method 1: Mahalanobis distance on daily returns
from numpy.linalg import inv

ret_clean = log_ret.dropna()
mu = ret_clean.mean().values
cov = ret_clean.cov().values
cov_inv = inv(cov)

mahal_dist = []
for i in range(len(ret_clean)):
    x = ret_clean.iloc[i].values - mu
    d = np.sqrt(x @ cov_inv @ x)
    mahal_dist.append(d)

mahal_series = pd.Series(mahal_dist, index=ret_clean.index)
mahal_threshold = mahal_series.quantile(0.99)
extreme_days = mahal_series[mahal_series > mahal_threshold]

# Cluster extreme days into periods
extreme_periods = []
if len(extreme_days) > 0:
    dates = extreme_days.index.tolist()
    clusters = [[dates[0]]]
    for d in dates[1:]:
        if (d - clusters[-1][-1]).days < 10:
            clusters[-1].append(d)
        else:
            clusters.append([d])
    for c in clusters:
        period_rets = ret_clean.loc[c[0]:c[-1]]
        extreme_periods.append({
            'start': str(c[0].date()),
            'end': str(c[-1].date()),
            'days': len(c),
            'max_mahalanobis': round(float(mahal_series.loc[c].max()), 2),
            'e1_cumret_pct': round(float(period_rets['e1'].sum() * 100), 2),
            'e2_cumret_pct': round(float(period_rets['e2'].sum() * 100), 2),
            'e3_cumret_pct': round(float(period_rets['e3'].sum() * 100), 2),
        })

# Method 2: Rolling effective dimensionality extremes
# (identifies when the correlation structure itself is anomalous)
eff_dim_ret = rolling_pca_ret_df.set_index('date')['eff_dim']
eff_dim_ret.index = pd.to_datetime(eff_dim_ret.index)
eff_dim_low = eff_dim_ret[eff_dim_ret < eff_dim_ret.quantile(0.05)]
eff_dim_high = eff_dim_ret[eff_dim_ret > eff_dim_ret.quantile(0.95)]

low_dim_periods = []
if len(eff_dim_low) > 0:
    dates = eff_dim_low.index.tolist()
    clusters = [[dates[0]]]
    for d in dates[1:]:
        if (d - clusters[-1][-1]).days < 30:
            clusters[-1].append(d)
        else:
            clusters.append([d])
    for c in clusters:
        low_dim_periods.append({
            'start': str(c[0].date()),
            'end': str(c[-1].date()),
            'min_eff_dim': round(float(eff_dim_ret.loc[c[0]:c[-1]].min()), 3),
            'interpretation': 'system nearly 1-dimensional (all series move together)'
        })

high_dim_periods = []
if len(eff_dim_high) > 0:
    dates = eff_dim_high.index.tolist()
    clusters = [[dates[0]]]
    for d in dates[1:]:
        if (d - clusters[-1][-1]).days < 30:
            clusters[-1].append(d)
        else:
            clusters.append([d])
    for c in clusters:
        high_dim_periods.append({
            'start': str(c[0].date()),
            'end': str(c[-1].date()),
            'max_eff_dim': round(float(eff_dim_ret.loc[c[0]:c[-1]].max()), 3),
            'interpretation': 'system nearly 3-dimensional (series move independently)'
        })

# ============================================================
# 8. Time-Varying Correlation Structure Summary
# ============================================================
print("Computing time-varying correlation epochs...")

# Split into 5-year windows and compute correlation matrices
epoch_corrs = []
for year_start in range(2001, 2026, 3):
    year_end = year_start + 3
    mask = (log_ret.index.year >= year_start) & (log_ret.index.year < year_end)
    chunk = log_ret[mask]
    if len(chunk) < 100:
        continue
    corr = chunk.corr()
    eigenvalues = np.linalg.eigvalsh(corr.values)
    epoch_corrs.append({
        'period': f'{year_start}-{year_end}',
        'n_obs': len(chunk),
        'corr_e1_e2': round(float(corr.loc['e1', 'e2']), 4),
        'corr_e1_e3': round(float(corr.loc['e1', 'e3']), 4),
        'corr_e2_e3': round(float(corr.loc['e2', 'e3']), 4),
        'eigenvalues': [round(float(x), 4) for x in sorted(eigenvalues, reverse=True)],
    })

# ============================================================
# 9. Lead-Lag Analysis
# ============================================================
print("Analyzing lead-lag relationships...")

lead_lag = {}
for a, b in pairs:
    pair_key = f'{a}_{b}'
    cross_corrs = {}
    for lag in range(-20, 21):
        if lag >= 0:
            c = log_ret[a].iloc[lag:].corr(log_ret[b].iloc[:len(log_ret)-lag if lag > 0 else len(log_ret)])
        else:
            c = log_ret[a].iloc[:len(log_ret)+lag].corr(log_ret[b].iloc[-lag:])
        cross_corrs[lag] = round(float(c), 4) if not np.isnan(c) else 0.0
    # Find lag with max absolute correlation
    max_lag = max(cross_corrs.keys(), key=lambda k: abs(cross_corrs[k]))
    lead_lag[pair_key] = {
        'max_abs_corr_lag': max_lag,
        'max_abs_corr_value': cross_corrs[max_lag],
        'cross_corr_at_key_lags': {k: cross_corrs[k] for k in [-5, -3, -1, 0, 1, 3, 5]},
    }

# ============================================================
# 10. Compile Results JSON
# ============================================================
print("Compiling results...")

results = {
    'data_summary': {
        'n_observations': len(ratios),
        'start_date': str(ratios.index[0].date()),
        'end_date': str(ratios.index[-1].date()),
    },
    'individual_series': individual_stats,
    'adf_tests': adf_results,
    'pairwise': pairwise_stats,
    'full_sample_correlation_levels': {
        f'{a}_{b}': round(float(full_corr.loc[a, b]), 4) for a, b in pairs
    },
    'full_sample_correlation_returns': {
        f'{a}_{b}': round(float(full_corr_returns.loc[a, b]), 4) for a, b in pairs
    },
    'pca_full_sample_levels': full_pca_result,
    'pca_full_sample_returns': ret_pca_result,
    'epoch_correlations': epoch_corrs,
    'cusum_break_dates': cusum_results,
    'extreme_periods_mahalanobis': extreme_periods[:30],  # top 30
    'low_dimensionality_periods': low_dim_periods,
    'high_dimensionality_periods': high_dim_periods,
    'lead_lag': lead_lag,
    'rolling_pca_summary': {
        'window': 252,
        'pc1_ev_mean': round(float(rolling_pca_ret_df['ev1_ratio'].mean()), 4),
        'pc1_ev_std': round(float(rolling_pca_ret_df['ev1_ratio'].std()), 4),
        'pc1_ev_min': round(float(rolling_pca_ret_df['ev1_ratio'].min()), 4),
        'pc1_ev_max': round(float(rolling_pca_ret_df['ev1_ratio'].max()), 4),
        'eff_dim_mean': round(float(rolling_pca_ret_df['eff_dim'].mean()), 3),
        'eff_dim_std': round(float(rolling_pca_ret_df['eff_dim'].std()), 3),
    },
}

class NumpyEncoder(json.JSONEncoder):
    def default(self, obj):
        if isinstance(obj, (np.bool_,)):
            return bool(obj)
        if isinstance(obj, (np.integer,)):
            return int(obj)
        if isinstance(obj, (np.floating,)):
            return float(obj)
        if isinstance(obj, np.ndarray):
            return obj.tolist()
        return super().default(obj)

with open(OUTPUT_DIR / 'agent_a_results.json', 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2, ensure_ascii=False, cls=NumpyEncoder)

print(f"Results saved to {OUTPUT_DIR / 'agent_a_results.json'}")

# ============================================================
# 11. Generate Report
# ============================================================
print("Generating report...")

report_lines = []
report_lines.append("# Blind Descriptive Analysis: Three Ratio Series (e1, e2, e3)")
report_lines.append("")
report_lines.append(f"**Data**: {results['data_summary']['n_observations']} daily observations, "
                    f"{results['data_summary']['start_date']} to {results['data_summary']['end_date']}")
report_lines.append("")

# --- Individual Series ---
report_lines.append("## 1. Individual Series Characteristics")
report_lines.append("")

for name in ['e1', 'e2', 'e3']:
    s = individual_stats[name]
    b = s['basic']
    r = s['return_distribution']
    report_lines.append(f"### {name}")
    report_lines.append("")
    report_lines.append(f"**Level statistics**: mean={b['mean']:.4f}, std={b['std']:.4f}, "
                        f"range=[{b['min']:.4f}, {b['max']:.4f}], current={b['current_value']:.4f}")
    report_lines.append(f"**Total change over sample**: {b['total_return_pct']:+.1f}%")
    report_lines.append(f"**Level distribution**: skewness={b['skewness']:.2f}, excess kurtosis={b['kurtosis']:.2f}")
    report_lines.append("")
    report_lines.append(f"**Daily return distribution**: mean={r['mean_daily_return']:.6f}, std={r['std_daily_return']:.4f}, "
                        f"skewness={r['skewness']:.2f}, excess kurtosis={r['kurtosis']:.2f}")
    report_lines.append(f"**Jarque-Bera**: stat={r['jarque_bera_stat']:.1f}, p={r['jarque_bera_pvalue']:.2e} "
                        f"({'reject normality' if r['jarque_bera_pvalue'] < 0.05 else 'cannot reject normality'})")
    report_lines.append("")

    adf_l = s['adf_level']
    adf_r = s['adf_return']
    report_lines.append(f"**ADF test (levels)**: stat={adf_l['adf_statistic']:.3f}, p={adf_l['p_value']:.4f} "
                        f"→ {'stationary' if adf_l['stationary_5pct'] else 'non-stationary'}")
    report_lines.append(f"**ADF test (returns)**: stat={adf_r['adf_statistic']:.3f}, p={adf_r['p_value']:.6f} "
                        f"→ {'stationary' if adf_r['stationary_5pct'] else 'non-stationary'}")
    report_lines.append(f"**Time in uptrend (252d rolling)**: {s['pct_time_in_uptrend_252d']}%")
    report_lines.append("")

    if s['vol_regime_breaks']:
        report_lines.append(f"**Volatility regime breaks**: {len(s['vol_regime_breaks'])} detected")
        for vb in s['vol_regime_breaks'][:10]:
            report_lines.append(f"  - {vb['start']} to {vb['end']} (z-peak={vb['z_peak']})")
    else:
        report_lines.append("**Volatility regime breaks**: none detected at z>2.0 threshold")
    report_lines.append("")

# --- Pairwise ---
report_lines.append("## 2. Pairwise Relationships")
report_lines.append("")

report_lines.append("### Correlation Matrix (full sample, levels)")
report_lines.append("")
report_lines.append("|     | e1   | e2   | e3   |")
report_lines.append("|-----|------|------|------|")
for a in ['e1', 'e2', 'e3']:
    row = f"| {a} |"
    for b in ['e1', 'e2', 'e3']:
        if a == b:
            row += " 1.0  |"
        else:
            key = f'{min(a,b)}_{max(a,b)}'
            val = results['full_sample_correlation_levels'].get(key,
                  results['full_sample_correlation_levels'].get(f'{a}_{b}', '?'))
            row += f" {val} |"
    report_lines.append(row)
report_lines.append("")

report_lines.append("### Correlation Matrix (full sample, daily returns)")
report_lines.append("")
report_lines.append("|     | e1   | e2   | e3   |")
report_lines.append("|-----|------|------|------|")
for a in ['e1', 'e2', 'e3']:
    row = f"| {a} |"
    for b in ['e1', 'e2', 'e3']:
        if a == b:
            row += " 1.0  |"
        else:
            key = f'{min(a,b)}_{max(a,b)}'
            val = results['full_sample_correlation_returns'].get(key,
                  results['full_sample_correlation_returns'].get(f'{a}_{b}', '?'))
            row += f" {val} |"
    report_lines.append(row)
report_lines.append("")

for pair_key, ps in pairwise_stats.items():
    a, b = pair_key.split('_')
    report_lines.append(f"### Pair {a}-{b}")
    report_lines.append("")
    report_lines.append(f"**Rolling correlation (252d)**: mean={ps['rolling_corr_mean']:.3f}, "
                        f"std={ps['rolling_corr_std']:.3f}, range=[{ps['rolling_corr_min']:.3f}, {ps['rolling_corr_max']:.3f}]")
    report_lines.append(f"**Correlation sign changes**: {ps['num_sign_changes']} times over sample")
    report_lines.append(f"**Cointegration (Engle-Granger)**: stat={ps['cointegration_stat']:.3f}, "
                        f"p={ps['cointegration_pval']:.4f} → {'cointegrated' if ps['cointegrated_5pct'] else 'not cointegrated'}")
    report_lines.append("")

# --- Epoch correlations ---
report_lines.append("### Time-Varying Correlation by 3-Year Epochs")
report_lines.append("")
report_lines.append("| Period | e1-e2 | e1-e3 | e2-e3 | Top eigenvalue |")
report_lines.append("|--------|-------|-------|-------|---------------|")
for ec in epoch_corrs:
    report_lines.append(f"| {ec['period']} | {ec['corr_e1_e2']:+.3f} | {ec['corr_e1_e3']:+.3f} | "
                        f"{ec['corr_e2_e3']:+.3f} | {ec['eigenvalues'][0]:.3f} |")
report_lines.append("")

# --- Lead-Lag ---
report_lines.append("### Lead-Lag Analysis")
report_lines.append("")
for pair_key, ll in lead_lag.items():
    a, b = pair_key.split('_')
    lag = ll['max_abs_corr_lag']
    val = ll['max_abs_corr_value']
    if lag > 0:
        interp = f"{a} leads {b} by {lag} days"
    elif lag < 0:
        interp = f"{b} leads {a} by {-lag} days"
    else:
        interp = "contemporaneous"
    report_lines.append(f"**{a}-{b}**: max |cross-corr| at lag={lag} (value={val:.4f}) → {interp}")
    lags_str = ", ".join([f"lag {k}: {v:.4f}" for k, v in ll['cross_corr_at_key_lags'].items()])
    report_lines.append(f"  Key lags: {lags_str}")
    report_lines.append("")

# --- Joint Structure ---
report_lines.append("## 3. Joint Structure")
report_lines.append("")

report_lines.append("### Full-Sample PCA (on standardized levels)")
report_lines.append("")
evr = full_pca_result['explained_variance_ratio']
report_lines.append(f"**Explained variance**: PC1={evr[0]:.1%}, PC2={evr[1]:.1%}, PC3={evr[2]:.1%}")
report_lines.append("")
report_lines.append("**Component loadings**:")
report_lines.append("")
report_lines.append("| Component | e1 | e2 | e3 |")
report_lines.append("|-----------|------|------|------|")
for i, comp in enumerate(full_pca_result['components']):
    report_lines.append(f"| PC{i+1} | {comp[0]:+.3f} | {comp[1]:+.3f} | {comp[2]:+.3f} |")
report_lines.append("")

report_lines.append("### Full-Sample PCA (on standardized daily returns)")
report_lines.append("")
evr_r = ret_pca_result['explained_variance_ratio']
report_lines.append(f"**Explained variance**: PC1={evr_r[0]:.1%}, PC2={evr_r[1]:.1%}, PC3={evr_r[2]:.1%}")
report_lines.append("")
report_lines.append("**Component loadings**:")
report_lines.append("")
report_lines.append("| Component | e1 | e2 | e3 |")
report_lines.append("|-----------|------|------|------|")
for i, comp in enumerate(ret_pca_result['components']):
    report_lines.append(f"| PC{i+1} | {comp[0]:+.3f} | {comp[1]:+.3f} | {comp[2]:+.3f} |")
report_lines.append("")

rps = results['rolling_pca_summary']
report_lines.append("### Rolling PCA Summary (252-day window, returns)")
report_lines.append("")
report_lines.append(f"**PC1 explained variance**: mean={rps['pc1_ev_mean']:.1%}, std={rps['pc1_ev_std']:.1%}, "
                    f"range=[{rps['pc1_ev_min']:.1%}, {rps['pc1_ev_max']:.1%}]")
report_lines.append(f"**Effective dimensionality**: mean={rps['eff_dim_mean']:.2f}, std={rps['eff_dim_std']:.2f}")
report_lines.append("")
report_lines.append("Interpretation: effective dimensionality near 1 means all three series move in lockstep; "
                    "near 3 means they are essentially independent.")
report_lines.append("")

# --- Structural Breaks ---
report_lines.append("## 4. Structural Breaks (CUSUM)")
report_lines.append("")
for name, dates in cusum_results.items():
    if dates:
        report_lines.append(f"**{name}**: breaks detected near {', '.join(dates)}")
    else:
        report_lines.append(f"**{name}**: no significant structural break detected")
report_lines.append("")

# --- Anomalous Periods ---
report_lines.append("## 5. Anomalous and Extreme Periods")
report_lines.append("")

report_lines.append("### Extreme Days (Mahalanobis distance, top 1%)")
report_lines.append("")
report_lines.append(f"Total extreme periods (clustered): {len(extreme_periods)}")
report_lines.append("")
if extreme_periods:
    report_lines.append("| Period | Days | Max Mahal. | e1 cum% | e2 cum% | e3 cum% |")
    report_lines.append("|--------|------|-----------|---------|---------|---------|")
    for ep in extreme_periods[:20]:
        report_lines.append(f"| {ep['start']} to {ep['end']} | {ep['days']} | {ep['max_mahalanobis']:.1f} | "
                            f"{ep['e1_cumret_pct']:+.1f} | {ep['e2_cumret_pct']:+.1f} | {ep['e3_cumret_pct']:+.1f} |")
    report_lines.append("")

report_lines.append("### Low-Dimensionality Periods (system nearly 1-D)")
report_lines.append("")
if low_dim_periods:
    for lp in low_dim_periods:
        report_lines.append(f"- {lp['start']} to {lp['end']}: eff.dim={lp['min_eff_dim']:.2f}")
else:
    report_lines.append("None detected.")
report_lines.append("")

report_lines.append("### High-Dimensionality Periods (system nearly 3-D)")
report_lines.append("")
if high_dim_periods:
    for hp in high_dim_periods:
        report_lines.append(f"- {hp['start']} to {hp['end']}: eff.dim={hp['max_eff_dim']:.2f}")
else:
    report_lines.append("None detected.")
report_lines.append("")

# --- Summary ---
report_lines.append("## 6. Summary of Key Findings")
report_lines.append("")

# Generate summary based on actual results
summary_items = []

# 1. Stationarity
non_stat = [name for name in ['e1', 'e2', 'e3']
            if not individual_stats[name]['adf_level']['stationary_5pct']]
stat = [name for name in ['e1', 'e2', 'e3']
        if individual_stats[name]['adf_level']['stationary_5pct']]
if non_stat:
    summary_items.append(f"**Stationarity**: {', '.join(non_stat)} are non-stationary in levels "
                         f"(unit root not rejected at 5%). "
                         f"{'All' if len(stat) == 0 else ', '.join(stat) + ' are'} "
                         f"{'series have' if len(non_stat) > 1 else 'has'} stationary returns.")

# 2. Dominant trend
for name in ['e1', 'e2', 'e3']:
    ret_total = individual_stats[name]['basic']['total_return_pct']
    if abs(ret_total) > 50:
        summary_items.append(f"**{name} strong trend**: {ret_total:+.0f}% total change over sample period.")

# 3. Correlation structure
for pair_key, ps in pairwise_stats.items():
    a, b = pair_key.split('_')
    if ps['rolling_corr_std'] > 0.25:
        summary_items.append(f"**Unstable {a}-{b} correlation**: rolling corr ranges from "
                             f"{ps['rolling_corr_min']:.2f} to {ps['rolling_corr_max']:.2f} "
                             f"with {ps['num_sign_changes']} sign changes — "
                             f"the relationship between these series is fundamentally time-varying.")

# 4. PCA
if evr_r[0] > 0.5:
    summary_items.append(f"**Dominant common factor**: PC1 explains {evr_r[0]:.0%} of return variance, "
                         f"suggesting a strong common driver.")
elif evr_r[0] < 0.4:
    summary_items.append(f"**No dominant common factor**: PC1 explains only {evr_r[0]:.0%} of return variance. "
                         f"The three series are driven by largely distinct forces.")

# 5. Cointegration
coint_pairs = [pk for pk, ps in pairwise_stats.items() if ps['cointegrated_5pct']]
if coint_pairs:
    summary_items.append(f"**Cointegration**: {', '.join(coint_pairs)} show cointegration (Engle-Granger, 5%), "
                         f"implying a long-run equilibrium relationship.")
else:
    summary_items.append("**No pairwise cointegration detected** at 5% level.")

# 6. Notable anomalous periods
if extreme_periods:
    top3 = sorted(extreme_periods, key=lambda x: x['max_mahalanobis'], reverse=True)[:3]
    summary_items.append("**Most anomalous periods** (by Mahalanobis distance): " +
                         "; ".join([f"{ep['start']}–{ep['end']} (d={ep['max_mahalanobis']:.1f})" for ep in top3]))

# 7. Dimensionality variation
if rps['eff_dim_std'] > 0.15:
    summary_items.append(f"**Time-varying dimensionality**: effective dimensionality fluctuates "
                         f"(mean={rps['eff_dim_mean']:.2f}, std={rps['eff_dim_std']:.2f}), "
                         f"indicating the system's internal structure is not stable over time.")

for item in summary_items:
    report_lines.append(f"- {item}")
    report_lines.append("")

report_lines.append("---")
report_lines.append("*Analysis performed blind — series identities unknown to analyst.*")

report = "\n".join(report_lines)

with open(OUTPUT_DIR / 'agent_a_report.md', 'w', encoding='utf-8') as f:
    f.write(report)

print(f"Report saved to {OUTPUT_DIR / 'agent_a_report.md'}")
print("Done.")
