"""
Au edge internal structure test: spot/futures basis around 2020-04 synchronous compression.

Core question: Did the Au basis (GC=F - GLD*10) begin diverging BEFORE the K4
synchronous compression cluster (2020-04-22 to 2020-05-05), or only during/after?

Epistemological level: L2 (real data, single asset, single event).
"""

import json
import sys
from datetime import datetime, timedelta
from pathlib import Path

import numpy as np
import pandas as pd
import yfinance as yf


# ── Constants ──────────────────────────────────────────────────────────────

OUTPUT_DIR = Path(__file__).parent

# Synchronous compression cluster days (329号 settlement)
CLUSTER_1_DAYS = [
    "2020-04-22", "2020-04-23", "2020-04-27",
    "2020-04-29", "2020-04-30", "2020-05-05",
]
CLUSTER_2_DAYS = ["2020-06-11"]
CLUSTER_3_DAYS = ["2020-11-09"]

# Data window: wide enough for rolling stats warmup + context
DATA_START = "2019-07-01"
DATA_END = "2020-12-31"

# Analysis window
ANALYSIS_START = "2020-01-01"
ANALYSIS_END = "2020-07-01"

# Rolling window for statistics
ROLLING_WINDOW = 20

# Z-score threshold for "significant"
Z_THRESHOLD = 2.0

# How many days before cluster to check for leading signal
LEAD_CHECK_DAYS = [5, 10, 15, 20, 30]

# GLD to ounce conversion factor
GLD_TO_OZ = 10.0


def download_data():
    """Download GC=F and GLD daily data from yfinance."""
    print("Downloading GC=F (gold futures)...")
    gc = yf.download("GC=F", start=DATA_START, end=DATA_END, auto_adjust=True)
    print(f"  Got {len(gc)} rows")

    print("Downloading GLD (gold spot ETF proxy)...")
    gld = yf.download("GLD", start=DATA_START, end=DATA_END, auto_adjust=True)
    print(f"  Got {len(gld)} rows")

    return gc, gld


def compute_basis(gc_df, gld_df):
    """
    Compute basis = GC=F_close - GLD_close * 10.
    Also compute basis_pct = basis / (GLD_close * 10) * 100.
    Returns a DataFrame with columns: gc_close, gld_close, gld_oz, basis, basis_pct.
    """
    # Extract close prices, flatten multi-level columns if needed
    gc_close = gc_df["Close"].copy()
    gld_close = gld_df["Close"].copy()

    # If columns are multi-level (ticker as second level), flatten
    if hasattr(gc_close, "columns"):
        gc_close = gc_close.iloc[:, 0] if gc_close.ndim > 1 else gc_close
    if hasattr(gld_close, "columns"):
        gld_close = gld_close.iloc[:, 0] if gld_close.ndim > 1 else gld_close

    # Align on common dates
    df = pd.DataFrame({
        "gc_close": gc_close,
        "gld_close": gld_close,
    })
    df = df.dropna()

    # Compute basis
    df["gld_oz"] = df["gld_close"] * GLD_TO_OZ
    df["basis"] = df["gc_close"] - df["gld_oz"]
    df["basis_pct"] = (df["basis"] / df["gld_oz"]) * 100.0

    return df


def compute_rolling_stats(df, window=ROLLING_WINDOW):
    """Add rolling mean, std, and z-score for basis."""
    df = df.copy()
    df["basis_roll_mean"] = df["basis"].rolling(window=window, min_periods=window).mean()
    df["basis_roll_std"] = df["basis"].rolling(window=window, min_periods=window).std()
    df["basis_zscore"] = (df["basis"] - df["basis_roll_mean"]) / df["basis_roll_std"]

    # Same for basis_pct
    df["basis_pct_roll_mean"] = df["basis_pct"].rolling(window=window, min_periods=window).mean()
    df["basis_pct_roll_std"] = df["basis_pct"].rolling(window=window, min_periods=window).std()
    df["basis_pct_zscore"] = (df["basis_pct"] - df["basis_pct_roll_mean"]) / df["basis_pct_roll_std"]

    return df


def check_leading_signal(df, cluster_start_str, lead_days_list, z_threshold=Z_THRESHOLD):
    """
    Check if basis z-score exceeds threshold before cluster start.
    Returns dict with results for each lead_days value.
    """
    cluster_start = pd.Timestamp(cluster_start_str)

    results = {}
    for n in lead_days_list:
        check_start = cluster_start - pd.Timedelta(days=n)
        window = df.loc[check_start:cluster_start - pd.Timedelta(days=1)]

        if len(window) == 0:
            results[f"lead_{n}d"] = {
                "trading_days": 0,
                "max_abs_zscore": None,
                "any_above_threshold": False,
                "days_above_threshold": 0,
            }
            continue

        abs_z = window["basis_zscore"].abs()
        above = abs_z > z_threshold

        results[f"lead_{n}d"] = {
            "trading_days": int(len(window)),
            "max_abs_zscore": float(abs_z.max()) if len(abs_z) > 0 else None,
            "any_above_threshold": bool(above.any()),
            "days_above_threshold": int(above.sum()),
            "first_breach_date": str(window.index[above].min().date()) if above.any() else None,
        }

    return results


def analyze_cluster_window(df, cluster_days_str, label, context_days=10):
    """
    Analyze basis behavior around a specific cluster.
    Returns structured results.
    """
    cluster_dates = [pd.Timestamp(d) for d in cluster_days_str]
    cluster_start = min(cluster_dates)
    cluster_end = max(cluster_dates)

    # Extended window for context
    window_start = cluster_start - pd.Timedelta(days=context_days * 2)  # extra for weekends
    window_end = cluster_end + pd.Timedelta(days=context_days * 2)

    window = df.loc[window_start:window_end].copy()
    if len(window) == 0:
        return {"label": label, "error": "No data in window"}

    # Mark cluster days
    window["is_cluster"] = window.index.isin(cluster_dates)

    # Pre-cluster, during-cluster, post-cluster stats
    pre = window.loc[window.index < cluster_start]
    during = window.loc[window["is_cluster"]]
    post = window.loc[window.index > cluster_end]

    def _stats(sub, name):
        if len(sub) == 0:
            return {"phase": name, "n": 0}
        return {
            "phase": name,
            "n": int(len(sub)),
            "basis_mean": float(sub["basis"].mean()),
            "basis_std": float(sub["basis"].std()) if len(sub) > 1 else 0.0,
            "basis_min": float(sub["basis"].min()),
            "basis_max": float(sub["basis"].max()),
            "basis_pct_mean": float(sub["basis_pct"].mean()),
            "basis_pct_max": float(sub["basis_pct"].max()),
            "zscore_mean": float(sub["basis_zscore"].mean()) if sub["basis_zscore"].notna().any() else None,
            "zscore_max_abs": float(sub["basis_zscore"].abs().max()) if sub["basis_zscore"].notna().any() else None,
        }

    # Find basis peak in the entire window
    peak_idx = window["basis"].abs().idxmax()
    peak_phase = "pre" if peak_idx < cluster_start else ("during" if peak_idx <= cluster_end else "post")

    # Detailed daily values for cluster days and surrounding days
    detail_window = df.loc[
        (df.index >= cluster_start - pd.Timedelta(days=15)) &
        (df.index <= cluster_end + pd.Timedelta(days=10))
    ]
    daily_detail = []
    for idx, row in detail_window.iterrows():
        daily_detail.append({
            "date": str(idx.date()),
            "gc_close": round(float(row["gc_close"]), 2),
            "gld_oz": round(float(row["gld_oz"]), 2),
            "basis": round(float(row["basis"]), 2),
            "basis_pct": round(float(row["basis_pct"]), 4),
            "zscore": round(float(row["basis_zscore"]), 3) if pd.notna(row["basis_zscore"]) else None,
            "is_cluster_day": bool(idx in cluster_dates),
        })

    return {
        "label": label,
        "cluster_start": str(cluster_start.date()),
        "cluster_end": str(cluster_end.date()),
        "cluster_days_count": len(cluster_dates),
        "pre_cluster": _stats(pre, "pre"),
        "during_cluster": _stats(during, "during"),
        "post_cluster": _stats(post, "post"),
        "basis_peak": {
            "date": str(peak_idx.date()),
            "value": round(float(window.loc[peak_idx, "basis"]), 2),
            "phase": peak_phase,
        },
        "daily_detail": daily_detail,
    }


def analyze_non_sync_mahalanobis_days(df, cluster_days_all):
    """
    Check basis behavior on Mahalanobis-extreme days that are NOT part of
    synchronous compression clusters.

    We don't have the full list of 64 Mahalanobis-extreme days, so we
    approximate: look at dates where basis z-score is extreme and check
    whether they overlap with known cluster days.
    """
    cluster_set = set(pd.Timestamp(d) for d in cluster_days_all)
    analysis_window = df.loc[ANALYSIS_START:ANALYSIS_END].copy()

    # Days with |z-score| > 2
    extreme = analysis_window[analysis_window["basis_zscore"].abs() > Z_THRESHOLD].copy()
    extreme["is_cluster"] = extreme.index.isin(cluster_set)

    non_cluster_extreme = extreme[~extreme["is_cluster"]]
    cluster_extreme = extreme[extreme["is_cluster"]]

    return {
        "total_extreme_basis_days": int(len(extreme)),
        "extreme_in_cluster": int(len(cluster_extreme)),
        "extreme_not_in_cluster": int(len(non_cluster_extreme)),
        "non_cluster_extreme_dates": [
            {
                "date": str(idx.date()),
                "basis": round(float(row["basis"]), 2),
                "zscore": round(float(row["basis_zscore"]), 3),
            }
            for idx, row in non_cluster_extreme.iterrows()
        ],
    }


def compute_basis_regime_change(df):
    """
    Detect when basis regime changed by looking at structural breaks
    in the rolling mean. Uses simple heuristic: when rolling mean
    crosses its own long-term mean by >1 std.
    """
    analysis = df.loc["2020-01-01":"2020-07-01"].copy()
    if len(analysis) == 0:
        return {}

    long_mean = analysis["basis"].mean()
    long_std = analysis["basis"].std()

    # Find first date where basis persistently exceeds long_mean + long_std
    elevated = analysis["basis"] > (long_mean + long_std)
    if not elevated.any():
        return {
            "regime_change_detected": False,
            "long_term_mean": round(float(long_mean), 2),
            "long_term_std": round(float(long_std), 2),
        }

    # Find first sustained elevation (3+ consecutive days)
    elevated_runs = []
    run_start = None
    run_len = 0
    for idx, val in elevated.items():
        if val:
            if run_start is None:
                run_start = idx
            run_len += 1
        else:
            if run_len >= 3:
                elevated_runs.append((run_start, run_len))
            run_start = None
            run_len = 0
    if run_len >= 3:
        elevated_runs.append((run_start, run_len))

    return {
        "regime_change_detected": len(elevated_runs) > 0,
        "long_term_mean": round(float(long_mean), 2),
        "long_term_std": round(float(long_std), 2),
        "elevated_threshold": round(float(long_mean + long_std), 2),
        "sustained_elevated_runs": [
            {"start": str(s.date()), "days": d} for s, d in elevated_runs
        ],
        "first_sustained_elevation": str(elevated_runs[0][0].date()) if elevated_runs else None,
    }


def generate_text_timeseries(df, start, end, cluster_days_all):
    """Generate a text-based visualization of basis and z-score."""
    cluster_set = set(pd.Timestamp(d) for d in cluster_days_all)
    window = df.loc[start:end]

    lines = []
    lines.append(f"{'Date':>12} {'GC=F':>8} {'GLD*10':>8} {'Basis':>8} {'Basis%':>8} {'Z-score':>8} {'':>3}")
    lines.append("-" * 65)

    for idx, row in window.iterrows():
        marker = " **" if idx in cluster_set else ""
        z_str = f"{row['basis_zscore']:>8.2f}" if pd.notna(row["basis_zscore"]) else "     N/A"
        lines.append(
            f"{str(idx.date()):>12} "
            f"{row['gc_close']:>8.2f} "
            f"{row['gld_oz']:>8.2f} "
            f"{row['basis']:>8.2f} "
            f"{row['basis_pct']:>8.4f} "
            f"{z_str}"
            f"{marker}"
        )

    return "\n".join(lines)


def main():
    # ── Step 1: Download data ──
    gc_df, gld_df = download_data()

    # ── Step 2: Compute basis ──
    df = compute_basis(gc_df, gld_df)
    print(f"\nBasis DataFrame: {len(df)} rows, {df.index.min().date()} to {df.index.max().date()}")

    # ── Step 3: Rolling statistics ──
    df = compute_rolling_stats(df)

    # ── Step 4: Text time series (around cluster 1) ──
    all_cluster_days = CLUSTER_1_DAYS + CLUSTER_2_DAYS + CLUSTER_3_DAYS
    ts_text = generate_text_timeseries(df, "2020-03-01", "2020-06-15", all_cluster_days)
    print("\n=== Basis Time Series (2020-03 to 2020-06) ===")
    print(ts_text)

    # ── Step 5: Leading signal check ──
    leading = check_leading_signal(df, CLUSTER_1_DAYS[0], LEAD_CHECK_DAYS)
    print("\n=== Leading Signal Check (before 2020-04-22) ===")
    for k, v in leading.items():
        print(f"  {k}: max|z|={v['max_abs_zscore']}, above_threshold={v['any_above_threshold']}, "
              f"days_above={v['days_above_threshold']}")

    # ── Step 6: Cluster analysis ──
    c1 = analyze_cluster_window(df, CLUSTER_1_DAYS, "Cluster 1 (2020-04-22 to 2020-05-05)")
    c2 = analyze_cluster_window(df, CLUSTER_2_DAYS, "Cluster 2 (2020-06-11)")
    c3 = analyze_cluster_window(df, CLUSTER_3_DAYS, "Cluster 3 (2020-11-09)")

    print(f"\n=== Cluster 1 Basis Peak ===")
    print(f"  Peak date: {c1['basis_peak']['date']}, value: {c1['basis_peak']['value']}, "
          f"phase: {c1['basis_peak']['phase']}")

    # ── Step 7: Regime change detection ──
    regime = compute_basis_regime_change(df)
    print(f"\n=== Regime Change Detection ===")
    print(f"  Detected: {regime.get('regime_change_detected')}")
    if regime.get("first_sustained_elevation"):
        print(f"  First sustained elevation: {regime['first_sustained_elevation']}")

    # ── Step 8: Non-sync Mahalanobis days ──
    non_sync = analyze_non_sync_mahalanobis_days(df, all_cluster_days)
    print(f"\n=== Non-Sync Extreme Basis Days ===")
    print(f"  Total extreme: {non_sync['total_extreme_basis_days']}")
    print(f"  In cluster: {non_sync['extreme_in_cluster']}")
    print(f"  Not in cluster: {non_sync['extreme_not_in_cluster']}")

    # ── Step 9: Verdict ──
    # Determine the verdict based on leading signal results
    cluster_start = pd.Timestamp(CLUSTER_1_DAYS[0])

    # Check: was basis already elevated before the cluster?
    pre_cluster_30d = df.loc[
        (df.index >= cluster_start - pd.Timedelta(days=45)) &
        (df.index < cluster_start)
    ]
    if len(pre_cluster_30d) > 0 and pre_cluster_30d["basis_zscore"].notna().any():
        pre_max_z = float(pre_cluster_30d["basis_zscore"].abs().max())
        pre_first_breach = pre_cluster_30d[pre_cluster_30d["basis_zscore"].abs() > Z_THRESHOLD]
        if len(pre_first_breach) > 0:
            first_breach_date = pre_first_breach.index[0]
            lead_calendar_days = (cluster_start - first_breach_date).days
            lead_trading_days = len(pre_cluster_30d.loc[first_breach_date:cluster_start]) - 1
            verdict = "LEADING_SIGNAL"
            verdict_detail = (
                f"Basis z-score first breached {Z_THRESHOLD}σ on {first_breach_date.date()}, "
                f"{lead_calendar_days} calendar days ({lead_trading_days} trading days) "
                f"before cluster start."
            )
        else:
            verdict = "NO_LEADING_SIGNAL"
            verdict_detail = (
                f"Basis z-score max was {pre_max_z:.2f} in 30d before cluster, "
                f"below {Z_THRESHOLD}σ threshold."
            )
    else:
        verdict = "INSUFFICIENT_DATA"
        verdict_detail = "Not enough data to determine leading signal."

    # During-cluster stats
    cluster_dates = [pd.Timestamp(d) for d in CLUSTER_1_DAYS]
    during_cluster = df.loc[df.index.isin(cluster_dates)]
    during_max_z = float(during_cluster["basis_zscore"].abs().max()) if len(during_cluster) > 0 else None

    print(f"\n=== VERDICT ===")
    print(f"  {verdict}: {verdict_detail}")
    if during_max_z:
        print(f"  During cluster max |z|: {during_max_z:.2f}")

    # ── Step 10: Compile results ──
    results = {
        "experiment": "Au edge internal structure: basis leading signal test",
        "epistemological_level": "L2",
        "data": {
            "futures": "GC=F (gold futures, continuous contract)",
            "spot_proxy": "GLD * 10 (SPDR Gold Trust ETF, 1 share ≈ 1/10 oz)",
            "period": f"{DATA_START} to {DATA_END}",
            "rolling_window": ROLLING_WINDOW,
            "z_threshold": Z_THRESHOLD,
        },
        "cluster_1_analysis": c1,
        "cluster_2_analysis": c2,
        "cluster_3_analysis": c3,
        "leading_signal_check": leading,
        "regime_change": regime,
        "non_sync_extreme_days": non_sync,
        "verdict": {
            "classification": verdict,
            "detail": verdict_detail,
            "during_cluster_max_abs_zscore": during_max_z,
            "criteria": {
                "LEADING_SIGNAL": f"basis |z| > {Z_THRESHOLD} at least 5 days before cluster",
                "SYNC_SIGNAL": f"basis |z| > {Z_THRESHOLD} only during cluster",
                "NO_SIGNAL": f"basis |z| < {Z_THRESHOLD} throughout",
            },
        },
        "text_timeseries": ts_text,
    }

    # ── Step 11: Save results ──
    results_path = OUTPUT_DIR / "au_basis_results.json"
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False, default=str)
    print(f"\nResults saved to {results_path}")

    return results


if __name__ == "__main__":
    results = main()
