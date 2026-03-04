"""
K4 Monitor: eff.dim实时监控 + basis预警 + 体制识别
==================================================
独立运行：python scripts/k4_monitor.py
输出：JSON（stdout），包含四节点eff.dim、两个basis信号、体制状态。

数据源：
  Au  = yfinance GC=F (gold futures)
  DXY = yfinance DX-Y.NYB (dollar index)
  SPX = yfinance ^GSPC (S&P 500)
  WTI = FRED DCOILWTICO (WTI spot CSV)
  GLD = yfinance GLD (gold ETF, for Au basis)
  CL=F = yfinance CL=F (crude futures, for OIL basis)

谱系引用：329号(K4定义), 337号(工程化下游)
"""
from __future__ import annotations

import json
import sys
import warnings
from datetime import datetime, timezone

import numpy as np
import pandas as pd
import yfinance as yf

warnings.filterwarnings("ignore")

# ============================================================
# Constants
# ============================================================

EFFDIM_WINDOW = 252
BASIS_ROLLING_WINDOW = 20
BASIS_Z_THRESHOLD = 2.0
PERCENTILE_THRESHOLD = 5.0
GLD_TO_OZ = 10.0
CLUSTER_GAP_DAYS = 5

# Data needs long history for stable percentile computation
DATA_START = "2000-01-01"

NODES = {
    "Au":        {"col": "Au",  "peers": ["DXY", "SPX", "WTI"]},
    "USD":       {"col": "DXY", "peers": ["Au",  "SPX", "WTI"]},
    "Equity":    {"col": "SPX", "peers": ["DXY", "Au",  "WTI"]},
    "Commodity": {"col": "WTI", "peers": ["DXY", "Au",  "SPX"]},
}


# ============================================================
# Data Download
# ============================================================

def download_prices() -> pd.DataFrame:
    """Download and align all price series."""
    # WTI spot from FRED
    wti_url = (
        "https://fred.stlouisfed.org/graph/fredgraph.csv"
        "?id=DCOILWTICO&cosd=2000-01-01&coed=2030-12-31"
    )
    wti_df = pd.read_csv(wti_url)
    wti_df["observation_date"] = pd.to_datetime(wti_df["observation_date"])
    wti_df = wti_df.set_index("observation_date")
    wti_df = wti_df[wti_df["DCOILWTICO"] != "."]
    wti_df["DCOILWTICO"] = wti_df["DCOILWTICO"].astype(float)
    wti_df = wti_df.dropna()

    # yfinance tickers
    tickers = ["GC=F", "DX-Y.NYB", "^GSPC"]
    yf_data = yf.download(tickers, start=DATA_START, progress=False)["Close"]
    if hasattr(yf_data.columns, "nlevels") and yf_data.columns.nlevels > 1:
        yf_data.columns = yf_data.columns.get_level_values(0)

    prices = pd.DataFrame()
    for col_name in yf_data.columns:
        cn = str(col_name)
        if "GC" in cn:
            prices["Au"] = yf_data[col_name]
        elif "DX" in cn:
            prices["DXY"] = yf_data[col_name]
        elif "GSPC" in cn or "SP" in cn:
            prices["SPX"] = yf_data[col_name]

    prices = prices.join(
        wti_df.rename(columns={"DCOILWTICO": "WTI"}), how="outer"
    )
    prices = prices.dropna()
    return prices


def download_basis_prices() -> tuple[pd.DataFrame, pd.DataFrame]:
    """Download GLD and CL=F for basis computation."""
    gld = yf.download("GLD", start=DATA_START, progress=False)["Close"]
    clf = yf.download("CL=F", start=DATA_START, progress=False)["Close"]
    # Flatten MultiIndex if present
    if isinstance(gld, pd.DataFrame):
        gld = gld.iloc[:, 0]
    if isinstance(clf, pd.DataFrame):
        clf = clf.iloc[:, 0]
    return gld.dropna(), clf.dropna()


# ============================================================
# eff.dim Computation
# ============================================================

def compute_log_ratio_returns(
    prices_df: pd.DataFrame, node_col: str, peer_cols: list[str]
) -> pd.DataFrame:
    """Compute log-ratio returns for each edge of a node."""
    returns = {}
    for i, peer in enumerate(peer_cols):
        ratio = prices_df[node_col] / prices_df[peer]
        log_ret = np.log(ratio).diff()
        returns[f"edge_{i}"] = log_ret
    return pd.DataFrame(returns, index=prices_df.index).dropna()


def compute_effdim_series(
    returns_df: pd.DataFrame, window: int = EFFDIM_WINDOW
) -> pd.Series:
    """Rolling eff.dim = exp(Shannon entropy of eigenvalues)."""
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
            values.append(float(np.exp(entropy)))
        dates.append(returns_df.index[i])
    return pd.Series(values, index=dates)


def compute_all_effdims(
    prices: pd.DataFrame,
) -> dict[str, pd.Series]:
    """Compute eff.dim series for all 4 nodes."""
    result = {}
    for name, cfg in NODES.items():
        returns = compute_log_ratio_returns(prices, cfg["col"], cfg["peers"])
        result[name] = compute_effdim_series(returns)
    return result


# ============================================================
# Basis Computation
# ============================================================

def compute_basis_signals(
    prices: pd.DataFrame,
    gld: pd.Series,
    clf: pd.Series,
) -> dict:
    """Compute Au basis and OIL basis with rolling z-scores."""
    # Au basis = GC=F - GLD*10
    au_df = pd.DataFrame({"gc": prices["Au"], "gld": gld}).dropna()
    au_df["basis"] = au_df["gc"] - au_df["gld"] * GLD_TO_OZ
    au_df["roll_mean"] = au_df["basis"].rolling(
        window=BASIS_ROLLING_WINDOW, min_periods=BASIS_ROLLING_WINDOW
    ).mean()
    au_df["roll_std"] = au_df["basis"].rolling(
        window=BASIS_ROLLING_WINDOW, min_periods=BASIS_ROLLING_WINDOW
    ).std()
    au_df["zscore"] = (au_df["basis"] - au_df["roll_mean"]) / au_df["roll_std"]

    # OIL basis = CL=F - DCOILWTICO
    oil_df = pd.DataFrame({"cl": clf, "wti_spot": prices["WTI"]}).dropna()
    oil_df["basis"] = oil_df["cl"] - oil_df["wti_spot"]
    oil_df["roll_mean"] = oil_df["basis"].rolling(
        window=BASIS_ROLLING_WINDOW, min_periods=BASIS_ROLLING_WINDOW
    ).mean()
    oil_df["roll_std"] = oil_df["basis"].rolling(
        window=BASIS_ROLLING_WINDOW, min_periods=BASIS_ROLLING_WINDOW
    ).std()
    oil_df["zscore"] = (oil_df["basis"] - oil_df["roll_mean"]) / oil_df["roll_std"]

    # Latest values
    au_latest = au_df.iloc[-1] if len(au_df) > 0 else None
    oil_latest = oil_df.iloc[-1] if len(oil_df) > 0 else None

    au_result = {
        "name": "Au_basis",
        "definition": "GC=F - GLD*10",
        "date": str(au_df.index[-1].date()) if au_latest is not None else None,
        "basis_value": round(float(au_latest["basis"]), 4) if au_latest is not None else None,
        "zscore_20d": round(float(au_latest["zscore"]), 4) if au_latest is not None and not np.isnan(au_latest["zscore"]) else None,
        "alert": bool(abs(au_latest["zscore"]) > BASIS_Z_THRESHOLD) if au_latest is not None and not np.isnan(au_latest["zscore"]) else False,
    }

    oil_result = {
        "name": "OIL_basis",
        "definition": "CL=F - DCOILWTICO",
        "date": str(oil_df.index[-1].date()) if oil_latest is not None else None,
        "basis_value": round(float(oil_latest["basis"]), 4) if oil_latest is not None else None,
        "zscore_20d": round(float(oil_latest["zscore"]), 4) if oil_latest is not None and not np.isnan(oil_latest["zscore"]) else None,
        "alert": bool(abs(oil_latest["zscore"]) > BASIS_Z_THRESHOLD) if oil_latest is not None and not np.isnan(oil_latest["zscore"]) else False,
    }

    return {"au_basis": au_result, "oil_basis": oil_result}


# ============================================================
# Regime Identification
# ============================================================

def identify_regime(
    node_effdims: dict[str, pd.Series],
) -> dict:
    """Determine current regime state: sync compression or not."""
    # Align dates
    common_dates = node_effdims["Au"].index
    for name in NODES:
        common_dates = common_dates.intersection(node_effdims[name].index)

    if len(common_dates) == 0:
        return {"error": "no common dates for regime identification"}

    # 5th percentile thresholds (computed over full history)
    thresholds = {}
    for name in NODES:
        series = node_effdims[name].loc[common_dates]
        thresholds[name] = float(np.percentile(series.values, PERCENTILE_THRESHOLD))

    # Find all sync compression days
    all_below = pd.Series(True, index=common_dates)
    for name in NODES:
        series = node_effdims[name].loc[common_dates]
        all_below = all_below & (series < thresholds[name])

    sync_days = common_dates[all_below]

    # Check if latest date is in sync compression
    latest_date = common_dates[-1]
    in_sync = bool(all_below.iloc[-1])

    if in_sync:
        # Count consecutive sync days ending at latest
        consecutive = 0
        for i in range(len(common_dates) - 1, -1, -1):
            if all_below.iloc[i]:
                consecutive += 1
            else:
                break

        # Find minimum eff.dim across nodes in current streak
        streak_start_idx = len(common_dates) - consecutive
        streak_dates = common_dates[streak_start_idx:]
        min_node = None
        min_val = float("inf")
        for name in NODES:
            vals = node_effdims[name].loc[streak_dates]
            node_min = float(vals.min())
            if node_min < min_val:
                min_val = node_min
                min_node = name

        return {
            "current_regime": "sync_compression",
            "regime_active": True,
            "duration_days": consecutive,
            "streak_start": str(streak_dates[0].date()),
            "min_effdim_node": min_node,
            "min_effdim_value": round(min_val, 4),
        }
    else:
        # Find most recent sync compression day
        if len(sync_days) == 0:
            return {
                "current_regime": "normal",
                "regime_active": False,
                "days_since_last_sync": None,
                "note": "no sync compression events in history",
            }

        last_sync = sync_days[-1]
        # Count trading days between last_sync and latest_date
        days_since = int(
            (common_dates > last_sync).sum()
        )

        return {
            "current_regime": "normal",
            "regime_active": False,
            "last_sync_date": str(last_sync.date()),
            "days_since_last_sync": days_since,
        }


# ============================================================
# Main
# ============================================================

def main() -> None:
    # Step 1: Download data
    print("Downloading price data...", file=sys.stderr)
    prices = download_prices()
    gld, clf = download_basis_prices()
    print(
        f"  Prices: {len(prices)} rows, "
        f"{prices.index[0].date()} to {prices.index[-1].date()}",
        file=sys.stderr,
    )

    # Step 2: Compute eff.dim for all nodes
    print("Computing eff.dim...", file=sys.stderr)
    node_effdims = compute_all_effdims(prices)

    # Align dates for percentile computation
    common_dates = node_effdims["Au"].index
    for name in NODES:
        common_dates = common_dates.intersection(node_effdims[name].index)

    # Build eff.dim output for each node
    effdim_output = {}
    for name in NODES:
        series = node_effdims[name].loc[common_dates]
        latest_val = float(series.iloc[-1])
        latest_date = series.index[-1]
        pct5 = float(np.percentile(series.values, PERCENTILE_THRESHOLD))
        # Percentile of current value within full history
        from scipy.stats import percentileofscore
        current_pct = float(
            percentileofscore(series.values, latest_val, kind="rank")
        )
        effdim_output[name] = {
            "date": str(latest_date.date()),
            "value": round(latest_val, 4),
            "percentile": round(current_pct, 1),
            "threshold_p5": round(pct5, 4),
            "below_p5": bool(latest_val < pct5),
        }
    print("  eff.dim computed for all 4 nodes.", file=sys.stderr)

    # Step 3: Basis signals
    print("Computing basis signals...", file=sys.stderr)
    basis_output = compute_basis_signals(prices, gld, clf)

    # Step 4: Regime identification
    print("Identifying regime...", file=sys.stderr)
    regime_output = identify_regime(node_effdims)

    # Assemble result
    result = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "data_range": {
            "start": str(prices.index[0].date()),
            "end": str(prices.index[-1].date()),
            "observations": len(prices),
        },
        "effdim": effdim_output,
        "basis": basis_output,
        "regime": regime_output,
    }

    print(json.dumps(result, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
