"""
K4 Cluster Identification: Extended Data (FRED WTI spot + yfinance)
===================================================================
Replaces CL=F (futures) with DCOILWTICO (WTI spot) from FRED.
FRED deleted London Fix gold (GOLDAMGBD228NLBM), so Au still uses GC=F.
"""
import numpy as np
import pandas as pd
import yfinance as yf
import json
from scipy.stats import percentileofscore

print("=== K4 Cluster Identification: Extended Data ===")
print()

# ============================================================
# Step 1: Download data
# ============================================================
print("Downloading data...")

# WTI spot from FRED (1986-01-02 to present)
wti_url = "https://fred.stlouisfed.org/graph/fredgraph.csv?id=DCOILWTICO&cosd=1986-01-01&coed=2026-12-31"
wti_df = pd.read_csv(wti_url)
wti_df["observation_date"] = pd.to_datetime(wti_df["observation_date"])
wti_df = wti_df.set_index("observation_date")
wti_df = wti_df[wti_df["DCOILWTICO"] != "."]
wti_df["DCOILWTICO"] = wti_df["DCOILWTICO"].astype(float)
wti_df = wti_df.dropna()
print(f"  WTI spot: {len(wti_df)} rows, {wti_df.index[0].date()} to {wti_df.index[-1].date()}")

# Gold futures, DXY, SPX from yfinance
tickers = ["GC=F", "DX-Y.NYB", "^GSPC"]
yf_data = yf.download(tickers, start="1986-01-01", progress=False)["Close"]
yf_data.columns = [str(c) if not isinstance(c, str) else c for c in yf_data.columns]
# Flatten MultiIndex if present
if hasattr(yf_data.columns, 'nlevels') and yf_data.columns.nlevels > 1:
    yf_data.columns = yf_data.columns.get_level_values(0)
print(f"  yfinance: {len(yf_data)} rows, columns: {list(yf_data.columns)}")

# Build prices DataFrame
prices = pd.DataFrame()
for col_name in yf_data.columns:
    if "GC" in str(col_name):
        prices["Au"] = yf_data[col_name]
    elif "DX" in str(col_name):
        prices["DXY"] = yf_data[col_name]
    elif "GSPC" in str(col_name) or "SP" in str(col_name):
        prices["SPX"] = yf_data[col_name]

# Add WTI spot
prices = prices.join(wti_df.rename(columns={"DCOILWTICO": "WTI"}), how="outer")
prices = prices.dropna()
print(f"  Aligned: {len(prices)} rows, {prices.index[0].date()} to {prices.index[-1].date()}")
print()

# ============================================================
# Step 2: Compute eff.dim for each node
# ============================================================
print("Computing eff.dim for each node...")

NODES = {
    "Au":        {"col": "Au",  "peers": ["DXY", "SPX", "WTI"],
                  "edge_names": ["Au/USD", "Au/Equity", "Au/Commodity"]},
    "USD":       {"col": "DXY", "peers": ["Au", "SPX", "WTI"],
                  "edge_names": ["USD/Au", "USD/Equity", "USD/Commodity"]},
    "Equity":    {"col": "SPX", "peers": ["DXY", "Au", "WTI"],
                  "edge_names": ["Equity/USD", "Equity/Au", "Equity/Commodity"]},
    "Commodity": {"col": "WTI", "peers": ["DXY", "Au", "SPX"],
                  "edge_names": ["Commodity/USD", "Commodity/Au", "Commodity/Equity"]},
}

WINDOW = 252


def compute_log_ratio_returns(prices_df, node_col, peer_cols):
    returns = {}
    for i, peer in enumerate(peer_cols):
        ratio = prices_df[node_col] / prices_df[peer]
        log_ret = np.log(ratio).diff()
        returns[f"edge_{i}"] = log_ret
    return pd.DataFrame(returns, index=prices_df.index).dropna()


def compute_effdim_series(returns_df, window=WINDOW):
    values = []
    dates = []
    arr = returns_df.values
    for i in range(window, len(arr)):
        w = arr[i - window:i]
        cov = np.cov(w.T)
        eigvals = np.linalg.eigvalsh(cov)
        eigvals = eigvals[eigvals > 1e-15]
        if len(eigvals) == 0:
            values.append(1.0)
        else:
            p = eigvals / eigvals.sum()
            entropy = -np.sum(p * np.log(p))
            values.append(np.exp(entropy))
        dates.append(returns_df.index[i])
    return pd.Series(values, index=dates)


node_effdims = {}
for name, cfg in NODES.items():
    returns = compute_log_ratio_returns(prices, cfg["col"], cfg["peers"])
    ed = compute_effdim_series(returns)
    node_effdims[name] = ed
    print(f"  {name}: {len(ed)} data points, eff.dim range [{ed.min():.4f}, {ed.max():.4f}]")

# Align all node eff.dim series
common_dates = node_effdims["Au"].index
for name in NODES:
    common_dates = common_dates.intersection(node_effdims[name].index)
print(f"  Common dates: {len(common_dates)}")
print()

# ============================================================
# Step 3: Identify sync compression clusters
# ============================================================
print("Identifying sync compression clusters...")

# Compute 5th percentile for each node
thresholds = {}
for name in NODES:
    series = node_effdims[name].loc[common_dates]
    pct5 = np.percentile(series.values, 5)
    thresholds[name] = pct5
    print(f"  {name} 5th percentile: {pct5:.4f}")

# Find days where ALL 4 nodes are below their 5th percentile
all_below = pd.Series(True, index=common_dates)
for name in NODES:
    series = node_effdims[name].loc[common_dates]
    all_below = all_below & (series < thresholds[name])

sync_days = common_dates[all_below]
print(f"  Sync compression days: {len(sync_days)}")

# Cluster: gap <= 5 trading days = same cluster
clusters = []
if len(sync_days) > 0:
    current_cluster = [sync_days[0]]
    for i in range(1, len(sync_days)):
        gap = len(common_dates[(common_dates > current_cluster[-1]) & (common_dates < sync_days[i])])
        if gap <= 5:
            current_cluster.append(sync_days[i])
        else:
            clusters.append(current_cluster)
            current_cluster = [sync_days[i]]
    clusters.append(current_cluster)

print(f"  Clusters found: {len(clusters)}")
print()

# ============================================================
# Step 4: Detail each cluster
# ============================================================
cluster_details = []
for idx, cluster_days in enumerate(clusters):
    start = cluster_days[0]
    end = cluster_days[-1]
    duration = len(cluster_days)

    node_stats = {}
    for name in NODES:
        vals = node_effdims[name].loc[cluster_days]
        node_stats[name] = {
            "min": float(vals.min()),
            "mean": float(vals.mean()),
            "percentile_at_min": float(percentileofscore(
                node_effdims[name].loc[common_dates].values, vals.min(), kind="rank"
            ))
        }

    detail = {
        "cluster_id": idx + 1,
        "start": str(start.date()),
        "end": str(end.date()),
        "duration_days": duration,
        "node_stats": node_stats,
    }
    cluster_details.append(detail)

    print(f"  Cluster {idx+1}: {start.date()} to {end.date()} ({duration} days)")
    for name in NODES:
        s = node_stats[name]
        print(f"    {name}: min eff.dim={s['min']:.4f}, pct={s['percentile_at_min']:.1f}%")

print()

# ============================================================
# Step 5: Also run 3-node version (no Au, back to 1986)
# ============================================================
print("=== 3-Node Analysis (no Au, 1986-2025) ===")
print()

# Build 3-node prices
prices_3n = pd.DataFrame()
dx_full = yf.download("DX-Y.NYB", start="1986-01-01", progress=False)["Close"]
spx_full = yf.download("^GSPC", start="1986-01-01", progress=False)["Close"]

prices_3n["DXY"] = dx_full
prices_3n["SPX"] = spx_full
prices_3n = prices_3n.join(wti_df.rename(columns={"DCOILWTICO": "WTI"}), how="outer")
prices_3n = prices_3n.dropna()
print(f"  3-node aligned: {len(prices_3n)} rows, {prices_3n.index[0].date()} to {prices_3n.index[-1].date()}")

NODES_3N = {
    "USD":       {"col": "DXY", "peers": ["SPX", "WTI"],
                  "edge_names": ["USD/Equity", "USD/Commodity"]},
    "Equity":    {"col": "SPX", "peers": ["DXY", "WTI"],
                  "edge_names": ["Equity/USD", "Equity/Commodity"]},
    "Commodity": {"col": "WTI", "peers": ["DXY", "SPX"],
                  "edge_names": ["Commodity/USD", "Commodity/Equity"]},
}


def compute_effdim_series_2d(returns_df, window=WINDOW):
    """2-edge version for 3-node graph"""
    values = []
    dates = []
    arr = returns_df.values
    for i in range(window, len(arr)):
        w = arr[i - window:i]
        cov = np.cov(w.T)
        eigvals = np.linalg.eigvalsh(cov)
        eigvals = eigvals[eigvals > 1e-15]
        if len(eigvals) == 0:
            values.append(1.0)
        else:
            p = eigvals / eigvals.sum()
            entropy = -np.sum(p * np.log(p))
            values.append(np.exp(entropy))
        dates.append(returns_df.index[i])
    return pd.Series(values, index=dates)


node_effdims_3n = {}
for name, cfg in NODES_3N.items():
    returns = compute_log_ratio_returns(prices_3n, cfg["col"], cfg["peers"])
    ed = compute_effdim_series_2d(returns)
    node_effdims_3n[name] = ed
    print(f"  {name}: {len(ed)} data points, eff.dim range [{ed.min():.4f}, {ed.max():.4f}]")

common_dates_3n = node_effdims_3n["USD"].index
for name in NODES_3N:
    common_dates_3n = common_dates_3n.intersection(node_effdims_3n[name].index)
print(f"  Common dates: {len(common_dates_3n)}")

# 5th percentile thresholds
thresholds_3n = {}
for name in NODES_3N:
    series = node_effdims_3n[name].loc[common_dates_3n]
    pct5 = np.percentile(series.values, 5)
    thresholds_3n[name] = pct5
    print(f"  {name} 5th percentile: {pct5:.4f}")

all_below_3n = pd.Series(True, index=common_dates_3n)
for name in NODES_3N:
    series = node_effdims_3n[name].loc[common_dates_3n]
    all_below_3n = all_below_3n & (series < thresholds_3n[name])

sync_days_3n = common_dates_3n[all_below_3n]
print(f"  Sync compression days (3-node): {len(sync_days_3n)}")

clusters_3n = []
if len(sync_days_3n) > 0:
    current_cluster = [sync_days_3n[0]]
    for i in range(1, len(sync_days_3n)):
        gap = len(common_dates_3n[(common_dates_3n > current_cluster[-1]) & (common_dates_3n < sync_days_3n[i])])
        if gap <= 5:
            current_cluster.append(sync_days_3n[i])
        else:
            clusters_3n.append(current_cluster)
            current_cluster = [sync_days_3n[i]]
    clusters_3n.append(current_cluster)

print(f"  Clusters found (3-node): {len(clusters_3n)}")
print()

cluster_details_3n = []
for idx, cluster_days in enumerate(clusters_3n):
    start = cluster_days[0]
    end = cluster_days[-1]
    duration = len(cluster_days)

    node_stats = {}
    for name in NODES_3N:
        vals = node_effdims_3n[name].loc[cluster_days]
        node_stats[name] = {
            "min": float(vals.min()),
            "mean": float(vals.mean()),
            "percentile_at_min": float(percentileofscore(
                node_effdims_3n[name].loc[common_dates_3n].values, vals.min(), kind="rank"
            ))
        }

    detail = {
        "cluster_id": idx + 1,
        "start": str(start.date()),
        "end": str(end.date()),
        "duration_days": duration,
        "node_stats": node_stats,
    }
    cluster_details_3n.append(detail)

    print(f"  3N-Cluster {idx+1}: {start.date()} to {end.date()} ({duration} days)")
    for name in NODES_3N:
        s = node_stats[name]
        print(f"    {name}: min eff.dim={s['min']:.4f}, pct={s['percentile_at_min']:.1f}%")

# ============================================================
# Step 6: Save all results
# ============================================================
results = {
    "four_node_analysis": {
        "data_window": f"{prices.index[0].date()} to {prices.index[-1].date()}",
        "total_price_observations": len(prices),
        "effdim_observations": len(common_dates),
        "window": WINDOW,
        "data_sources": {
            "Au": "yfinance GC=F (gold futures, 2000-08 start)",
            "DXY": "yfinance DX-Y.NYB",
            "SPX": "yfinance ^GSPC",
            "WTI": "FRED DCOILWTICO (WTI spot, 1986-01 start, but constrained by Au)",
        },
        "thresholds_5pct": {name: float(v) for name, v in thresholds.items()},
        "sync_compression_days": len(sync_days),
        "cluster_count": len(clusters),
        "clusters": cluster_details,
    },
    "three_node_analysis": {
        "data_window": f"{prices_3n.index[0].date()} to {prices_3n.index[-1].date()}",
        "total_price_observations": len(prices_3n),
        "effdim_observations": len(common_dates_3n),
        "window": WINDOW,
        "data_sources": {
            "DXY": "yfinance DX-Y.NYB (1986 start)",
            "SPX": "yfinance ^GSPC (1986 start)",
            "WTI": "FRED DCOILWTICO (WTI spot, 1986-01 start)",
        },
        "note": "No Au node. 2-edge eff.dim per node. Covers 1986-2025 (40 years).",
        "thresholds_5pct": {name: float(v) for name, v in thresholds_3n.items()},
        "sync_compression_days": len(sync_days_3n),
        "cluster_count": len(clusters_3n),
        "clusters": cluster_details_3n,
    },
    "fred_gold_spot_status": "DELETED from FRED (Jan 2022). GOLDAMGBD228NLBM and GOLDPMGBD228NLBM no longer available. IBA/LBMA data removed.",
    "comparison": {
        "previous": "3 clusters in 25 years (yfinance CL=F, 2000-2025)",
        "four_node_current": f"{len(clusters)} clusters in ~{(prices.index[-1] - prices.index[0]).days // 365} years",
        "three_node_current": f"{len(clusters_3n)} clusters in ~{(prices_3n.index[-1] - prices_3n.index[0]).days // 365} years",
    },
}

with open("tmp/fred-k4-cluster/fred_k4_cluster_results.json", "w") as f:
    json.dump(results, f, indent=2, ensure_ascii=False)

print()
print("Results saved to tmp/fred-k4-cluster/fred_k4_cluster_results.json")
print()
print("=== SUMMARY ===")
print(f"4-node (2000-2025): {len(clusters)} clusters")
print(f"3-node (1986-2025): {len(clusters_3n)} clusters")
print(f"Previous (yfinance only): 3 clusters")
print("=== DONE ===")
