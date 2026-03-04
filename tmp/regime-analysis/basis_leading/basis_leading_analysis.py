"""
Basis Leading Signal Test: 2020-21 Synchronous Compression Regime
=================================================================

Core question: Did Au basis and OIL basis diverge BEFORE eff.dim
synchronous compression, or only during/after?

Regime definition (from fred_k4_cluster_report.md):
  - Single continuous cluster: 2020-04-03 to 2021-04-23 (263 sync days)
  - NOT 3 separate events as in the old au-basis-experiment

Data:
  Au basis  = GC=F (gold futures) - GLD*10 (spot proxy)
  OIL basis = CL=F (crude futures) - DCOILWTICO (WTI spot from FRED)
  eff.dim   = 4-node K4 (Au, DXY, SPX, WTI), 252d rolling window

Epistemological level: L2 (real data, single regime event)

Genealogy references:
  - 329: K4 sync compression definition
  - Previous: tmp/au-basis-experiment/ (old 3-cluster version)
"""

from __future__ import annotations

import json
import sys
import warnings
from datetime import datetime
from pathlib import Path
from typing import Any

import numpy as np
import pandas as pd
import yfinance as yf

warnings.filterwarnings("ignore")

# ============================================================
# Constants
# ============================================================

OUTPUT_DIR = Path(__file__).parent

# Regime dates (from fred_k4_cluster_report.md)
REGIME_ENTRY = pd.Timestamp("2020-04-03")
REGIME_EXIT = pd.Timestamp("2021-04-23")

# Analysis windows
ENTRY_WINDOW_START = pd.Timestamp("2020-01-01")
ENTRY_WINDOW_END = pd.Timestamp("2020-06-01")
EXIT_WINDOW_START = pd.Timestamp("2021-02-01")
EXIT_WINDOW_END = pd.Timestamp("2021-07-01")

# Normal reference period (no sync compression)
NORMAL_START = pd.Timestamp("2019-01-01")
NORMAL_END = pd.Timestamp("2019-12-31")

# Data download window (wide enough for rolling warmup)
DATA_START = "2018-06-01"
DATA_END = "2021-09-01"

# Rolling parameters
ROLLING_WINDOW = 20
Z_THRESHOLD = 2.0

# eff.dim parameters
EFFDIM_WINDOW = 252
EFFDIM_DATA_START = "2000-01-01"  # Need long history for percentile computation

# GLD conversion
GLD_TO_OZ = 10.0

# Notable date
NEGATIVE_OIL_DATE = pd.Timestamp("2020-04-20")


# ============================================================
# Step 1: Data Download
# ============================================================

def download_yf_close(ticker: str, start: str, end: str | None = None) -> pd.Series:
    """Download close price from Yahoo Finance."""
    kwargs = {"start": start, "progress": False, "auto_adjust": True}
    if end:
        kwargs["end"] = end
    df = yf.download(ticker, **kwargs)
    if df.empty:
        raise RuntimeError(f"yfinance returned empty data for {ticker}")
    if isinstance(df.columns, pd.MultiIndex):
        close = df["Close"]
        if isinstance(close, pd.DataFrame):
            close = close.iloc[:, 0]
    else:
        close = df["Close"]
    s = close.copy()
    s.index = pd.DatetimeIndex(s.index)
    s.name = ticker
    return s.dropna()


def load_wti_fred(csv_path: Path) -> pd.Series:
    """Load DCOILWTICO from local FRED CSV."""
    df = pd.read_csv(csv_path, parse_dates=["observation_date"],
                     index_col="observation_date")
    s = df["DCOILWTICO"].replace(".", np.nan).astype(float)
    s.index = pd.DatetimeIndex(s.index)
    s.name = "DCOILWTICO"
    return s.dropna()


def download_all_data() -> dict[str, pd.Series]:
    """Download all required price series."""
    print("=== Step 1: Download Data ===")

    # GC=F (gold futures)
    print("  Downloading GC=F (gold futures)...")
    gc = download_yf_close("GC=F", DATA_START, DATA_END)
    print(f"    {gc.index[0].date()} to {gc.index[-1].date()}, {len(gc)} rows")

    # GLD (gold spot proxy)
    print("  Downloading GLD (gold ETF)...")
    gld = download_yf_close("GLD", DATA_START, DATA_END)
    print(f"    {gld.index[0].date()} to {gld.index[-1].date()}, {len(gld)} rows")

    # CL=F (crude futures)
    print("  Downloading CL=F (crude futures)...")
    cl = download_yf_close("CL=F", DATA_START, DATA_END)
    print(f"    {cl.index[0].date()} to {cl.index[-1].date()}, {len(cl)} rows")

    # DCOILWTICO (WTI spot from FRED local CSV)
    wti_csv = Path(__file__).parent.parent.parent / "fred-k4-cluster" / "wti_fred.csv"
    print(f"  Loading DCOILWTICO from {wti_csv}...")
    wti = load_wti_fred(wti_csv)
    # Trim to analysis window
    wti = wti.loc[DATA_START:DATA_END]
    print(f"    {wti.index[0].date()} to {wti.index[-1].date()}, {len(wti)} rows")

    # For eff.dim: need full history from 2000
    print("  Downloading full history for eff.dim computation...")
    gc_full = download_yf_close("GC=F", EFFDIM_DATA_START)
    dxy_full = download_yf_close("DX-Y.NYB", EFFDIM_DATA_START)
    spx_full = download_yf_close("^GSPC", EFFDIM_DATA_START)
    wti_full = load_wti_fred(wti_csv)
    print(f"    GC=F full: {gc_full.index[0].date()} to {gc_full.index[-1].date()}, {len(gc_full)} rows")
    print(f"    DXY full:  {dxy_full.index[0].date()} to {dxy_full.index[-1].date()}, {len(dxy_full)} rows")
    print(f"    SPX full:  {spx_full.index[0].date()} to {spx_full.index[-1].date()}, {len(spx_full)} rows")
    print(f"    WTI full:  {wti_full.index[0].date()} to {wti_full.index[-1].date()}, {len(wti_full)} rows")

    return {
        "gc": gc, "gld": gld, "cl": cl, "wti": wti,
        "gc_full": gc_full, "dxy_full": dxy_full, "spx_full": spx_full, "wti_full": wti_full,
    }


# ============================================================
# Step 2: Compute Basis Series
# ============================================================

def compute_au_basis(gc: pd.Series, gld: pd.Series) -> pd.DataFrame:
    """Au basis = GC=F - GLD*10."""
    df = pd.DataFrame({"gc": gc, "gld": gld}).dropna()
    df["gld_oz"] = df["gld"] * GLD_TO_OZ
    df["basis"] = df["gc"] - df["gld_oz"]
    df["basis_pct"] = (df["basis"] / df["gld_oz"]) * 100.0
    return df


def compute_oil_basis(cl: pd.Series, wti: pd.Series) -> pd.DataFrame:
    """OIL basis = CL=F - DCOILWTICO."""
    df = pd.DataFrame({"cl": cl, "wti_spot": wti}).dropna()
    df["basis"] = df["cl"] - df["wti_spot"]
    # basis_pct relative to spot (handle negative spot)
    df["basis_pct"] = np.where(
        df["wti_spot"].abs() > 0.01,
        (df["basis"] / df["wti_spot"].abs()) * 100.0,
        np.nan
    )
    return df


def add_rolling_zscore(df: pd.DataFrame, col: str = "basis",
                       window: int = ROLLING_WINDOW) -> pd.DataFrame:
    """Add rolling mean, std, z-score for a given column."""
    result = df.copy()
    result[f"{col}_roll_mean"] = df[col].rolling(window=window, min_periods=window).mean()
    result[f"{col}_roll_std"] = df[col].rolling(window=window, min_periods=window).std()
    result[f"{col}_zscore"] = (
        (df[col] - result[f"{col}_roll_mean"]) / result[f"{col}_roll_std"]
    )
    return result


# ============================================================
# Step 3: Compute eff.dim Sync State
# ============================================================

NODES = {
    "Au": {"col": "Au", "peers": ["DXY", "SPX", "WTI"]},
    "$": {"col": "DXY", "peers": ["Au", "SPX", "WTI"]},
    "Equity": {"col": "SPX", "peers": ["DXY", "Au", "WTI"]},
    "Commodity": {"col": "WTI", "peers": ["DXY", "Au", "SPX"]},
}


def compute_log_ratio_returns(prices: pd.DataFrame, node_col: str,
                              peer_cols: list[str]) -> pd.DataFrame:
    """Log-ratio returns between node and its peers."""
    returns = {}
    for i, peer in enumerate(peer_cols):
        ratio = prices[node_col] / prices[peer]
        log_ret = np.log(ratio).diff()
        returns[f"edge_{i}"] = log_ret
    return pd.DataFrame(returns, index=prices.index).dropna()


def compute_effdim_series(returns: pd.DataFrame,
                          window: int = EFFDIM_WINDOW) -> pd.Series:
    """Rolling eff.dim computation (Shannon entropy of eigenvalue spectrum)."""
    values = []
    dates = []
    arr = returns.values
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
        dates.append(returns.index[i])
    return pd.Series(values, index=pd.DatetimeIndex(dates))


def compute_sync_state(prices_full: pd.DataFrame) -> pd.DataFrame:
    """Compute per-day sync compression state for all 4 nodes.

    Returns DataFrame with columns: Au_effdim, $_effdim, Equity_effdim,
    Commodity_effdim, all_below_5pct (bool).
    """
    print("\n=== Step 3: Compute eff.dim Sync State ===")

    node_effdims = {}
    for name, cfg in NODES.items():
        returns = compute_log_ratio_returns(prices_full, cfg["col"], cfg["peers"])
        ed = compute_effdim_series(returns)
        node_effdims[name] = ed
        print(f"  {name}: {len(ed)} eff.dim values, "
              f"range [{ed.min():.4f}, {ed.max():.4f}]")

    # Align to common dates
    eddf = pd.DataFrame({
        f"{name}_effdim": series for name, series in node_effdims.items()
    }).dropna()
    print(f"  Common dates: {len(eddf)}")

    # Compute 5th percentile thresholds (full sample)
    thresholds = {}
    for name in NODES:
        col = f"{name}_effdim"
        pct5 = float(np.percentile(eddf[col].values, 5))
        thresholds[name] = pct5
        eddf[f"{name}_below_5pct"] = eddf[col] < pct5
        print(f"  {name} 5th pct threshold: {pct5:.6f}")

    # All 4 nodes below 5th percentile simultaneously
    eddf["all_below_5pct"] = (
        eddf["Au_below_5pct"] &
        eddf["$_below_5pct"] &
        eddf["Equity_below_5pct"] &
        eddf["Commodity_below_5pct"]
    )

    sync_days = eddf["all_below_5pct"].sum()
    print(f"  Total sync compression days: {sync_days}")

    return eddf, thresholds


# ============================================================
# Step 4: Leading Signal Analysis
# ============================================================

def find_first_extreme_zscore(zscore_series: pd.Series,
                              start: pd.Timestamp, end: pd.Timestamp,
                              threshold: float = Z_THRESHOLD) -> dict:
    """Find first date where |z-score| exceeds threshold in [start, end)."""
    window = zscore_series.loc[start:end].dropna()
    above = window.abs() > threshold
    if above.any():
        first_date = window.index[above][0]
        return {
            "found": True,
            "first_date": str(first_date.date()),
            "first_zscore": float(window.loc[first_date]),
            "max_abs_zscore": float(window.abs().max()),
            "days_above": int(above.sum()),
            "total_days": int(len(window)),
        }
    return {
        "found": False,
        "max_abs_zscore": float(window.abs().max()) if len(window) > 0 else None,
        "total_days": int(len(window)),
    }


def find_first_sync_compression(eddf: pd.DataFrame,
                                start: pd.Timestamp, end: pd.Timestamp) -> dict:
    """Find first date where all 4 nodes are in sync compression."""
    window = eddf.loc[start:end]
    sync = window["all_below_5pct"]
    if sync.any():
        first_date = window.index[sync][0]
        return {
            "found": True,
            "first_date": str(first_date.date()),
            "total_sync_days_in_window": int(sync.sum()),
        }
    return {
        "found": False,
        "total_sync_days_in_window": 0,
    }


def find_last_sync_compression(eddf: pd.DataFrame,
                                start: pd.Timestamp, end: pd.Timestamp) -> dict:
    """Find last date where all 4 nodes are in sync compression."""
    window = eddf.loc[start:end]
    sync = window["all_below_5pct"]
    if sync.any():
        last_date = window.index[sync][-1]
        return {
            "found": True,
            "last_date": str(last_date.date()),
            "total_sync_days_in_window": int(sync.sum()),
        }
    return {
        "found": False,
        "total_sync_days_in_window": 0,
    }


def find_zscore_normalization(zscore_series: pd.Series,
                              start: pd.Timestamp, end: pd.Timestamp,
                              threshold: float = Z_THRESHOLD) -> dict:
    """Find last date where |z-score| exceeds threshold, then when it normalizes."""
    window = zscore_series.loc[start:end].dropna()
    above = window.abs() > threshold
    if not above.any():
        return {
            "last_extreme_date": None,
            "normalization_date": None,
            "detail": "No extreme z-score in window",
        }

    # Find last extreme date
    last_extreme_date = window.index[above][-1]

    # Find normalization: first date AFTER last_extreme_date where |z| < threshold
    # for at least 5 consecutive days
    after = window.loc[last_extreme_date:]
    normal = after.abs() <= threshold

    norm_run_start = None
    run_len = 0
    for idx, is_normal in normal.items():
        if is_normal:
            if norm_run_start is None:
                norm_run_start = idx
            run_len += 1
            if run_len >= 5:
                return {
                    "last_extreme_date": str(last_extreme_date.date()),
                    "normalization_date": str(norm_run_start.date()),
                    "run_length": run_len,
                }
        else:
            norm_run_start = None
            run_len = 0

    return {
        "last_extreme_date": str(last_extreme_date.date()),
        "normalization_date": None,
        "detail": "z-score did not normalize in window",
    }


def analyze_entry_window(au_basis: pd.DataFrame, oil_basis: pd.DataFrame,
                         eddf: pd.DataFrame) -> dict:
    """Analyze leading signal in the regime entry window."""
    print("\n=== Step 4a: Entry Window Analysis ===")
    print(f"  Window: {ENTRY_WINDOW_START.date()} to {ENTRY_WINDOW_END.date()}")
    print(f"  Regime entry: {REGIME_ENTRY.date()}")

    # Au basis first extreme z-score BEFORE regime entry
    au_pre = find_first_extreme_zscore(
        au_basis["basis_zscore"],
        ENTRY_WINDOW_START,
        REGIME_ENTRY - pd.Timedelta(days=1)
    )
    print(f"  Au basis pre-entry extreme: {au_pre}")

    # OIL basis first extreme z-score BEFORE regime entry
    oil_pre = find_first_extreme_zscore(
        oil_basis["basis_zscore"],
        ENTRY_WINDOW_START,
        REGIME_ENTRY - pd.Timedelta(days=1)
    )
    print(f"  OIL basis pre-entry extreme: {oil_pre}")

    # eff.dim first sync compression date
    effdim_first = find_first_sync_compression(eddf, ENTRY_WINDOW_START, ENTRY_WINDOW_END)
    print(f"  eff.dim first sync: {effdim_first}")

    # Compute lead times
    au_lead = None
    oil_lead = None

    if au_pre["found"] and effdim_first["found"]:
        au_first = pd.Timestamp(au_pre["first_date"])
        effdim_date = pd.Timestamp(effdim_first["first_date"])
        au_lead = (effdim_date - au_first).days
        print(f"  Au basis leads eff.dim by {au_lead} calendar days")

    if oil_pre["found"] and effdim_first["found"]:
        oil_first = pd.Timestamp(oil_pre["first_date"])
        effdim_date = pd.Timestamp(effdim_first["first_date"])
        oil_lead = (effdim_date - oil_first).days
        print(f"  OIL basis leads eff.dim by {oil_lead} calendar days")

    # Au basis during entry window (full)
    au_entry_full = find_first_extreme_zscore(
        au_basis["basis_zscore"],
        ENTRY_WINDOW_START,
        ENTRY_WINDOW_END
    )
    oil_entry_full = find_first_extreme_zscore(
        oil_basis["basis_zscore"],
        ENTRY_WINDOW_START,
        ENTRY_WINDOW_END
    )

    return {
        "window": f"{ENTRY_WINDOW_START.date()} to {ENTRY_WINDOW_END.date()}",
        "regime_entry_date": str(REGIME_ENTRY.date()),
        "au_basis_pre_entry": au_pre,
        "oil_basis_pre_entry": oil_pre,
        "effdim_first_sync": effdim_first,
        "au_lead_days": au_lead,
        "oil_lead_days": oil_lead,
        "au_basis_full_window": au_entry_full,
        "oil_basis_full_window": oil_entry_full,
    }


def analyze_exit_window(au_basis: pd.DataFrame, oil_basis: pd.DataFrame,
                        eddf: pd.DataFrame) -> dict:
    """Analyze signal behavior in the regime exit window."""
    print("\n=== Step 4b: Exit Window Analysis ===")
    print(f"  Window: {EXIT_WINDOW_START.date()} to {EXIT_WINDOW_END.date()}")
    print(f"  Regime exit: {REGIME_EXIT.date()}")

    # eff.dim last sync compression
    effdim_last = find_last_sync_compression(eddf, EXIT_WINDOW_START, EXIT_WINDOW_END)
    print(f"  eff.dim last sync: {effdim_last}")

    # Au basis normalization in exit window
    au_norm = find_zscore_normalization(
        au_basis["basis_zscore"],
        EXIT_WINDOW_START,
        EXIT_WINDOW_END
    )
    print(f"  Au basis normalization: {au_norm}")

    # OIL basis normalization in exit window
    oil_norm = find_zscore_normalization(
        oil_basis["basis_zscore"],
        EXIT_WINDOW_START,
        EXIT_WINDOW_END
    )
    print(f"  OIL basis normalization: {oil_norm}")

    # Check if basis normalized BEFORE or AFTER eff.dim last sync
    au_exit_lead = None
    oil_exit_lead = None

    if au_norm["normalization_date"] and effdim_last["found"]:
        au_norm_date = pd.Timestamp(au_norm["normalization_date"])
        effdim_date = pd.Timestamp(effdim_last["last_date"])
        au_exit_lead = (effdim_date - au_norm_date).days
        print(f"  Au basis normalizes {au_exit_lead} days before eff.dim last sync")

    if oil_norm["normalization_date"] and effdim_last["found"]:
        oil_norm_date = pd.Timestamp(oil_norm["normalization_date"])
        effdim_date = pd.Timestamp(effdim_last["last_date"])
        oil_exit_lead = (effdim_date - oil_norm_date).days
        print(f"  OIL basis normalizes {oil_exit_lead} days before eff.dim last sync")

    return {
        "window": f"{EXIT_WINDOW_START.date()} to {EXIT_WINDOW_END.date()}",
        "regime_exit_date": str(REGIME_EXIT.date()),
        "effdim_last_sync": effdim_last,
        "au_basis_normalization": au_norm,
        "oil_basis_normalization": oil_norm,
        "au_exit_lead_days": au_exit_lead,
        "oil_exit_lead_days": oil_exit_lead,
    }


# ============================================================
# Step 5: Normal vs Pre-Entry Comparison
# ============================================================

def compute_period_stats(df: pd.DataFrame, start: pd.Timestamp,
                         end: pd.Timestamp, label: str) -> dict:
    """Compute basis statistics for a period."""
    window = df.loc[start:end].dropna(subset=["basis"])
    if len(window) == 0:
        return {"label": label, "n": 0}

    basis = window["basis"]
    zscore = window["basis_zscore"].dropna()

    extreme_days = int((zscore.abs() > Z_THRESHOLD).sum()) if len(zscore) > 0 else 0
    extreme_freq = extreme_days / len(zscore) if len(zscore) > 0 else 0

    return {
        "label": label,
        "n": int(len(basis)),
        "mean": float(basis.mean()),
        "std": float(basis.std()) if len(basis) > 1 else 0.0,
        "min": float(basis.min()),
        "max": float(basis.max()),
        "median": float(basis.median()),
        "extreme_zscore_days": extreme_days,
        "extreme_zscore_freq": round(extreme_freq, 4),
    }


def compare_periods(au_basis: pd.DataFrame, oil_basis: pd.DataFrame) -> dict:
    """Compare normal period vs pre-entry period basis behavior."""
    print("\n=== Step 5: Period Comparison ===")

    au_normal = compute_period_stats(au_basis, NORMAL_START, NORMAL_END, "normal_2019")
    au_pre = compute_period_stats(au_basis, ENTRY_WINDOW_START,
                                  REGIME_ENTRY - pd.Timedelta(days=1), "pre_entry_2020")
    au_during = compute_period_stats(au_basis, REGIME_ENTRY, REGIME_EXIT, "during_regime")

    oil_normal = compute_period_stats(oil_basis, NORMAL_START, NORMAL_END, "normal_2019")
    oil_pre = compute_period_stats(oil_basis, ENTRY_WINDOW_START,
                                   REGIME_ENTRY - pd.Timedelta(days=1), "pre_entry_2020")
    oil_during = compute_period_stats(oil_basis, REGIME_ENTRY, REGIME_EXIT, "during_regime")

    print(f"  Au normal 2019: mean={au_normal['mean']:.2f}, std={au_normal['std']:.2f}, "
          f"extreme freq={au_normal['extreme_zscore_freq']:.4f}")
    print(f"  Au pre-entry:   mean={au_pre['mean']:.2f}, std={au_pre['std']:.2f}, "
          f"extreme freq={au_pre['extreme_zscore_freq']:.4f}")
    print(f"  Au during:      mean={au_during['mean']:.2f}, std={au_during['std']:.2f}, "
          f"extreme freq={au_during['extreme_zscore_freq']:.4f}")

    print(f"  OIL normal 2019: mean={oil_normal['mean']:.2f}, std={oil_normal['std']:.2f}, "
          f"extreme freq={oil_normal['extreme_zscore_freq']:.4f}")
    print(f"  OIL pre-entry:   mean={oil_pre['mean']:.2f}, std={oil_pre['std']:.2f}, "
          f"extreme freq={oil_pre['extreme_zscore_freq']:.4f}")
    print(f"  OIL during:      mean={oil_during['mean']:.2f}, std={oil_during['std']:.2f}, "
          f"extreme freq={oil_during['extreme_zscore_freq']:.4f}")

    return {
        "au_basis": {"normal_2019": au_normal, "pre_entry_2020": au_pre, "during_regime": au_during},
        "oil_basis": {"normal_2019": oil_normal, "pre_entry_2020": oil_pre, "during_regime": oil_during},
    }


# ============================================================
# Step 6: Negative Oil Price Special Annotation
# ============================================================

def annotate_negative_oil(oil_basis: pd.DataFrame, cl: pd.Series,
                          wti: pd.Series) -> dict:
    """Document 2020-04-20 negative oil price impact."""
    print("\n=== Step 6: Negative Oil Price Annotation ===")

    target = NEGATIVE_OIL_DATE

    # Get raw prices around that date
    window_start = target - pd.Timedelta(days=5)
    window_end = target + pd.Timedelta(days=5)

    cl_window = cl.loc[window_start:window_end]
    wti_window = wti.loc[window_start:window_end]
    basis_window = oil_basis.loc[window_start:window_end]

    daily = []
    for date in basis_window.index:
        row = {
            "date": str(date.date()),
            "cl_close": float(cl.loc[date]) if date in cl.index else None,
            "wti_spot": float(wti.loc[date]) if date in wti.index else None,
        }
        if date in basis_window.index:
            row["basis"] = float(basis_window.loc[date, "basis"])
            row["basis_zscore"] = (
                float(basis_window.loc[date, "basis_zscore"])
                if pd.notna(basis_window.loc[date, "basis_zscore"]) else None
            )
        daily.append(row)

    # Check if 2020-04-20 is actually in our data
    has_date = target in oil_basis.index
    if has_date:
        basis_val = float(oil_basis.loc[target, "basis"])
        z_val = (
            float(oil_basis.loc[target, "basis_zscore"])
            if pd.notna(oil_basis.loc[target, "basis_zscore"]) else None
        )
    else:
        basis_val = None
        z_val = None

    result = {
        "date": str(target.date()),
        "in_data": has_date,
        "basis_value": basis_val,
        "basis_zscore": z_val,
        "note": (
            "2020-04-20: WTI May futures (CL=F) closed at approximately -$37.63, "
            "DCOILWTICO spot at approximately -$36.98. This is a real market event "
            "(storage cost exceeding commodity value), NOT a data artifact. "
            "The extreme basis value reflects genuine market dislocation."
        ),
        "daily_context": daily,
    }

    print(f"  2020-04-20 in data: {has_date}")
    if has_date:
        print(f"  OIL basis: {basis_val:.2f}, z-score: {z_val}")

    return result


# ============================================================
# Step 7: Daily Time Series for Key Windows
# ============================================================

def generate_daily_timeseries(au_basis: pd.DataFrame, oil_basis: pd.DataFrame,
                              eddf: pd.DataFrame,
                              start: pd.Timestamp, end: pd.Timestamp) -> list[dict]:
    """Generate daily time series combining basis and eff.dim data."""
    dates = pd.date_range(start, end, freq="B")  # business days
    rows = []
    for d in dates:
        row = {"date": str(d.date())}

        # Au basis
        if d in au_basis.index:
            row["au_basis"] = round(float(au_basis.loc[d, "basis"]), 2)
            row["au_basis_zscore"] = (
                round(float(au_basis.loc[d, "basis_zscore"]), 3)
                if pd.notna(au_basis.loc[d, "basis_zscore"]) else None
            )
        else:
            row["au_basis"] = None
            row["au_basis_zscore"] = None

        # OIL basis
        if d in oil_basis.index:
            row["oil_basis"] = round(float(oil_basis.loc[d, "basis"]), 2)
            row["oil_basis_zscore"] = (
                round(float(oil_basis.loc[d, "basis_zscore"]), 3)
                if pd.notna(oil_basis.loc[d, "basis_zscore"]) else None
            )
        else:
            row["oil_basis"] = None
            row["oil_basis_zscore"] = None

        # eff.dim sync state
        if d in eddf.index:
            row["all_below_5pct"] = bool(eddf.loc[d, "all_below_5pct"])
            row["au_effdim"] = round(float(eddf.loc[d, "Au_effdim"]), 4)
            row["commodity_effdim"] = round(float(eddf.loc[d, "Commodity_effdim"]), 4)
        else:
            row["all_below_5pct"] = None

        rows.append(row)

    return rows


# ============================================================
# Step 8: Verdict
# ============================================================

def compute_verdict(entry_results: dict, exit_results: dict,
                    comparison: dict) -> dict:
    """Determine whether basis is a leading signal."""
    print("\n=== Step 8: Verdict ===")

    # Entry analysis
    au_leads_entry = (entry_results["au_lead_days"] is not None and
                      entry_results["au_lead_days"] > 0)
    oil_leads_entry = (entry_results["oil_lead_days"] is not None and
                       entry_results["oil_lead_days"] > 0)

    # Exit analysis
    au_leads_exit = (exit_results["au_exit_lead_days"] is not None and
                     exit_results["au_exit_lead_days"] > 0)
    oil_leads_exit = (exit_results["oil_exit_lead_days"] is not None and
                      exit_results["oil_exit_lead_days"] > 0)

    # Regime behavior change
    au_normal_freq = comparison["au_basis"]["normal_2019"]["extreme_zscore_freq"]
    au_pre_freq = comparison["au_basis"]["pre_entry_2020"]["extreme_zscore_freq"]
    au_during_freq = comparison["au_basis"]["during_regime"]["extreme_zscore_freq"]

    oil_normal_freq = comparison["oil_basis"]["normal_2019"]["extreme_zscore_freq"]
    oil_pre_freq = comparison["oil_basis"]["pre_entry_2020"]["extreme_zscore_freq"]
    oil_during_freq = comparison["oil_basis"]["during_regime"]["extreme_zscore_freq"]

    # Classification
    au_classification = "UNKNOWN"
    if au_leads_entry:
        au_classification = "LEADING_SIGNAL"
    elif entry_results["au_basis_pre_entry"]["found"]:
        au_classification = "CONCURRENT_SIGNAL"
    else:
        au_classification = "LAGGING_OR_NO_SIGNAL"

    oil_classification = "UNKNOWN"
    if oil_leads_entry:
        oil_classification = "LEADING_SIGNAL"
    elif entry_results["oil_basis_pre_entry"]["found"]:
        oil_classification = "CONCURRENT_SIGNAL"
    else:
        oil_classification = "LAGGING_OR_NO_SIGNAL"

    verdict = {
        "au_basis": {
            "entry_classification": au_classification,
            "entry_lead_days": entry_results["au_lead_days"],
            "exit_leads": au_leads_exit,
            "exit_lead_days": exit_results["au_exit_lead_days"],
            "regime_change": {
                "normal_extreme_freq": au_normal_freq,
                "pre_entry_extreme_freq": au_pre_freq,
                "during_regime_extreme_freq": au_during_freq,
            },
        },
        "oil_basis": {
            "entry_classification": oil_classification,
            "entry_lead_days": entry_results["oil_lead_days"],
            "exit_leads": oil_leads_exit,
            "exit_lead_days": exit_results["oil_exit_lead_days"],
            "regime_change": {
                "normal_extreme_freq": oil_normal_freq,
                "pre_entry_extreme_freq": oil_pre_freq,
                "during_regime_extreme_freq": oil_during_freq,
            },
        },
        "combined_assessment": "",
    }

    # Combined narrative
    parts = []
    if au_classification == "LEADING_SIGNAL":
        parts.append(f"Au basis is a LEADING signal (leads eff.dim sync by "
                     f"{entry_results['au_lead_days']} calendar days at entry)")
    elif au_classification == "CONCURRENT_SIGNAL":
        parts.append("Au basis shows extreme behavior before regime, but eff.dim "
                     "sync occurs first or simultaneously")
    else:
        parts.append("Au basis does NOT show pre-regime extreme behavior")

    if oil_classification == "LEADING_SIGNAL":
        parts.append(f"OIL basis is a LEADING signal (leads eff.dim sync by "
                     f"{entry_results['oil_lead_days']} calendar days at entry)")
    elif oil_classification == "CONCURRENT_SIGNAL":
        parts.append("OIL basis shows extreme behavior before regime, but eff.dim "
                     "sync occurs first or simultaneously")
    else:
        parts.append("OIL basis does NOT show pre-regime extreme behavior")

    verdict["combined_assessment"] = "; ".join(parts)

    print(f"  Au basis: {au_classification}")
    print(f"  OIL basis: {oil_classification}")
    print(f"  {verdict['combined_assessment']}")

    return verdict


# ============================================================
# Main
# ============================================================

def main() -> int:
    print("=" * 72)
    print("Basis Leading Signal Test: 2020-21 Synchronous Compression Regime")
    print("=" * 72)

    # Step 1: Download data
    try:
        data = download_all_data()
    except Exception as e:
        print(f"\nData download failed: {e}")
        import traceback
        traceback.print_exc()
        return 1

    # Step 2: Compute basis series
    print("\n=== Step 2: Compute Basis Series ===")

    au_basis = compute_au_basis(data["gc"], data["gld"])
    au_basis = add_rolling_zscore(au_basis)
    print(f"  Au basis: {len(au_basis)} rows, "
          f"{au_basis.index[0].date()} to {au_basis.index[-1].date()}")
    print(f"  Au basis range: [{au_basis['basis'].min():.2f}, {au_basis['basis'].max():.2f}]")

    oil_basis = compute_oil_basis(data["cl"], data["wti"])
    oil_basis = add_rolling_zscore(oil_basis)
    print(f"  OIL basis: {len(oil_basis)} rows, "
          f"{oil_basis.index[0].date()} to {oil_basis.index[-1].date()}")
    print(f"  OIL basis range: [{oil_basis['basis'].min():.2f}, {oil_basis['basis'].max():.2f}]")

    # Step 3: Compute eff.dim sync state
    prices_full = pd.DataFrame({
        "Au": data["gc_full"],
        "DXY": data["dxy_full"],
        "SPX": data["spx_full"],
        "WTI": data["wti_full"],
    }).dropna()
    print(f"\n  Aligned prices for eff.dim: {len(prices_full)} rows, "
          f"{prices_full.index[0].date()} to {prices_full.index[-1].date()}")

    eddf, thresholds = compute_sync_state(prices_full)

    # Step 4: Leading signal analysis
    entry_results = analyze_entry_window(au_basis, oil_basis, eddf)
    exit_results = analyze_exit_window(au_basis, oil_basis, eddf)

    # Step 5: Period comparison
    comparison = compare_periods(au_basis, oil_basis)

    # Step 6: Negative oil price annotation
    neg_oil = annotate_negative_oil(oil_basis, data["cl"], data["wti"])

    # Step 7: Daily time series for key windows
    print("\n=== Step 7: Daily Time Series ===")
    entry_ts = generate_daily_timeseries(
        au_basis, oil_basis, eddf,
        ENTRY_WINDOW_START, ENTRY_WINDOW_END
    )
    exit_ts = generate_daily_timeseries(
        au_basis, oil_basis, eddf,
        EXIT_WINDOW_START, EXIT_WINDOW_END
    )
    print(f"  Entry window: {len(entry_ts)} rows")
    print(f"  Exit window: {len(exit_ts)} rows")

    # Step 8: Verdict
    verdict = compute_verdict(entry_results, exit_results, comparison)

    # ============================================================
    # Compile and save results
    # ============================================================
    results = {
        "metadata": {
            "analysis": "Basis leading signal test: 2020-21 synchronous compression regime",
            "run_time": datetime.now().isoformat(),
            "epistemological_level": "L2 (real data, single regime event, falsifiable)",
            "regime": {
                "entry_date": str(REGIME_ENTRY.date()),
                "exit_date": str(REGIME_EXIT.date()),
                "duration_days": (REGIME_EXIT - REGIME_ENTRY).days,
                "source": "fred_k4_cluster_report.md (263 sync days, single cluster)",
            },
            "data_sources": {
                "au_basis": "GC=F (gold futures) - GLD*10 (gold ETF proxy for spot)",
                "oil_basis": "CL=F (crude futures) - DCOILWTICO (WTI spot from FRED)",
                "effdim": "4-node K4 (Au=GC=F, DXY, SPX, WTI=DCOILWTICO), 252d rolling",
            },
            "parameters": {
                "rolling_window": ROLLING_WINDOW,
                "z_threshold": Z_THRESHOLD,
                "effdim_window": EFFDIM_WINDOW,
                "effdim_threshold_percentile": 5.0,
            },
        },
        "entry_window_analysis": entry_results,
        "exit_window_analysis": exit_results,
        "period_comparison": comparison,
        "negative_oil_annotation": neg_oil,
        "effdim_thresholds": {k: round(v, 6) for k, v in thresholds.items()},
        "verdict": verdict,
        "daily_timeseries": {
            "entry_window": entry_ts,
            "exit_window": exit_ts,
        },
    }

    # Save JSON
    results_path = OUTPUT_DIR / "basis_leading_results.json"
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False, default=str)
    print(f"\nResults saved to {results_path}")

    # ============================================================
    # Generate report
    # ============================================================
    report = generate_report(results, verdict, entry_results, exit_results,
                             comparison, neg_oil)
    report_path = OUTPUT_DIR / "basis_leading_report.md"
    with open(report_path, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"Report saved to {report_path}")

    print("\n" + "=" * 72)
    print("DONE")
    print("=" * 72)

    return 0


# ============================================================
# Report Generation
# ============================================================

def generate_report(results: dict, verdict: dict,
                    entry: dict, exit_res: dict,
                    comparison: dict, neg_oil: dict) -> str:
    """Generate six-element report."""
    lines = []

    lines.append("# Basis Leading Signal Test: 2020-21 Synchronous Compression Regime")
    lines.append("")
    lines.append(f"Run time: {results['metadata']['run_time']}")
    lines.append(f"Epistemological level: **L2** (real data, single regime event, falsifiable)")
    lines.append("")

    # 1. Conclusion
    lines.append("## 1. Conclusion")
    lines.append("")
    lines.append(f"**Regime**: 2020-04-03 to 2021-04-23 (263 sync compression days, single cluster)")
    lines.append("")
    lines.append("### Au Basis (GC=F - GLD*10)")
    lines.append("")
    au_v = verdict["au_basis"]
    lines.append(f"- Entry classification: **{au_v['entry_classification']}**")
    if au_v["entry_lead_days"] is not None:
        lines.append(f"- Lead time at entry: {au_v['entry_lead_days']} calendar days")
    lines.append(f"- Exit leads eff.dim: {au_v['exit_leads']}")
    if au_v["exit_lead_days"] is not None:
        lines.append(f"- Exit lead time: {au_v['exit_lead_days']} calendar days")
    lines.append(f"- Normal period (2019) extreme z-score frequency: {au_v['regime_change']['normal_extreme_freq']:.4f}")
    lines.append(f"- Pre-entry extreme z-score frequency: {au_v['regime_change']['pre_entry_extreme_freq']:.4f}")
    lines.append(f"- During-regime extreme z-score frequency: {au_v['regime_change']['during_regime_extreme_freq']:.4f}")
    lines.append("")

    lines.append("### OIL Basis (CL=F - DCOILWTICO)")
    lines.append("")
    oil_v = verdict["oil_basis"]
    lines.append(f"- Entry classification: **{oil_v['entry_classification']}**")
    if oil_v["entry_lead_days"] is not None:
        lines.append(f"- Lead time at entry: {oil_v['entry_lead_days']} calendar days")
    lines.append(f"- Exit leads eff.dim: {oil_v['exit_leads']}")
    if oil_v["exit_lead_days"] is not None:
        lines.append(f"- Exit lead time: {oil_v['exit_lead_days']} calendar days")
    lines.append(f"- Normal period (2019) extreme z-score frequency: {oil_v['regime_change']['normal_extreme_freq']:.4f}")
    lines.append(f"- Pre-entry extreme z-score frequency: {oil_v['regime_change']['pre_entry_extreme_freq']:.4f}")
    lines.append(f"- During-regime extreme z-score frequency: {oil_v['regime_change']['during_regime_extreme_freq']:.4f}")
    lines.append("")

    lines.append("### 2020-04-20 Negative Oil Price")
    lines.append("")
    lines.append(f"- In data: {neg_oil['in_data']}")
    if neg_oil["basis_value"] is not None:
        lines.append(f"- OIL basis value: {neg_oil['basis_value']:.2f}")
    if neg_oil["basis_zscore"] is not None:
        lines.append(f"- OIL basis z-score: {neg_oil['basis_zscore']:.3f}")
    lines.append(f"- Note: {neg_oil['note']}")
    lines.append("")

    lines.append("### Combined Assessment")
    lines.append("")
    lines.append(f"{verdict['combined_assessment']}")
    lines.append("")

    lines.append("### Comparison with Previous Experiment")
    lines.append("")
    lines.append("The previous Au basis experiment (`tmp/au-basis-experiment/`) used the old 3-cluster model:")
    lines.append("- Cluster 1: 2020-04-22 to 2020-05-05 (6 days)")
    lines.append("- Cluster 2: 2020-06-11 (1 day)")
    lines.append("- Cluster 3: 2020-11-09 (1 day)")
    lines.append("")
    lines.append("This experiment uses the corrected single-regime model:")
    lines.append("- Single regime: 2020-04-03 to 2021-04-23 (263 sync days)")
    lines.append("")
    lines.append("The old model had a narrower cluster start (2020-04-22), which made leading signal detection easier. ")
    lines.append("The corrected regime start (2020-04-03) is earlier, which may change the lead time calculation.")
    lines.append("")

    # 2. Definition basis
    lines.append("## 2. Definition Basis")
    lines.append("")
    lines.append("- **Au basis** = GC=F - GLD*10. GLD (SPDR Gold Trust) tracks gold spot price. 1 share of GLD represents approximately 1/10 troy ounce of gold. Tracking error is small relative to basis deviations.")
    lines.append("- **OIL basis** = CL=F - DCOILWTICO. DCOILWTICO is WTI spot price from FRED (Cushing, Oklahoma delivery). CL=F is CME WTI futures continuous contract.")
    lines.append("- **Sync compression** = all 4 node eff.dim simultaneously < their respective full-sample 5th percentile (329 settlement).")
    lines.append("- **Z-score** = (basis - 20d rolling mean) / 20d rolling std. |z| > 2 = extreme.")
    lines.append("- **Leading signal** = basis z-score breaches |z| > 2 before eff.dim first enters sync compression.")
    lines.append("")

    # 3. Boundary conditions
    lines.append("## 3. Boundary Conditions")
    lines.append("")
    lines.append("The following changes would alter the conclusion:")
    lines.append("")
    lines.append("1. **Z-score threshold**: Using |z| > 1.5 or |z| > 3 would change the \"first extreme\" date.")
    lines.append("2. **Rolling window**: 20d is arbitrary. 10d (more sensitive) or 60d (more stable) would shift detection dates.")
    lines.append("3. **GLD as spot proxy**: GLD has tracking error, creation/redemption costs, and is an ETF (not physical gold). If LBMA London Fix were available, basis computation would differ.")
    lines.append("4. **Regime boundary**: 2020-04-03 is the first day ALL 4 nodes are below 5th percentile. A different threshold (e.g., 10th percentile) would shift this date.")
    lines.append("5. **Single regime**: This is N=1 analysis. The conclusion does not generalize to other regimes without additional L3-level testing.")
    lines.append("6. **Futures roll artifacts**: Both GC=F and CL=F are continuous contracts with roll artifacts. OIL basis partially mitigates this (spot side from FRED), but Au basis (both sides futures-linked) retains this issue.")
    lines.append("")

    # 4. Downstream implications
    lines.append("## 4. Downstream Implications")
    lines.append("")
    lines.append("- If basis is a leading signal: basis monitoring can be added to the K4 eff.dim dashboard as an early warning indicator.")
    lines.append("- If basis is concurrent/lagging: basis is a symptom of sync compression, not a predictor. No actionable signal.")
    lines.append("- The OIL basis has the 2020-04-20 negative oil price event within the analysis window. If OIL basis leads, the negative oil price is part of the leading signal (market dislocation precedes statistical sync compression).")
    lines.append("- This is L2 (single regime). Upgrading to L3 would require finding other sync compression regimes (none in 2000-2025 data) or using different asset classes.")
    lines.append("")

    # 5. Genealogy references
    lines.append("## 5. Genealogy References")
    lines.append("")
    lines.append("- 329: K4 sync compression definition + threshold causality + QE regime dependency")
    lines.append("- 330: K4 topology and capital flow mapping")
    lines.append("- 231: Formalization validity domain rule (L2 annotation)")
    lines.append("- Previous experiment: `tmp/au-basis-experiment/` (3-cluster model, superseded)")
    lines.append("")

    # 6. Impact statement
    lines.append("## 6. Impact Statement")
    lines.append("")
    lines.append("- No code or definition changes.")
    lines.append("- Three new files in `tmp/regime-analysis/basis_leading/`:")
    lines.append("  - `basis_leading_analysis.py` (this script)")
    lines.append("  - `basis_leading_results.json` (structured results)")
    lines.append("  - `basis_leading_report.md` (this report)")
    lines.append("- Supersedes `tmp/au-basis-experiment/` conclusions for the 2020-21 regime (corrected from 3-cluster to single-regime model).")
    lines.append("- OIL basis analysis is new (not in previous experiment).")
    lines.append("")

    return "\n".join(lines)


if __name__ == "__main__":
    sys.exit(main())
