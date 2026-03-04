"""
相对阈值检验：frac_below 的 z-score / percentile 标准化
=======================================================
四节点的绝对阈值不收敛（Au 6.92%, $ 6.73%, Equity 0.77%, Commodity 0.81%）。
四个节点的 eff.dim 尺度不同（Commodity 均值~1.5 vs 其他~2.2）。

问题：对每个节点的 eff.dim 做 z-score 标准化后，阈值是否收敛？
如果收敛 → 全局约束存在但表现为相对位置。
如果不收敛 → 约束是节点局部的。

认识论等级：L2（真实数据验证）
谱系引用：323号（折叠理论结算）、321号（第三轮判决）
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
def _dl(ticker, start="2000-01-01", end="2026-03-03"):
    raw = yf.download(ticker, start=start, end=end)["Close"]
    if isinstance(raw, pd.DataFrame):
        raw = raw.iloc[:, 0]
    return raw


print("Downloading data...")
gc = _dl("GC=F")
dxy = _dl("DX-Y.NYB")
spx = _dl("^GSPC")
cl = _dl("CL=F")

df_all = pd.DataFrame({"gc": gc, "dxy": dxy, "spx": spx, "cl": cl}).dropna()

# Handle negative CL=F (2020-04-20)
neg_mask = df_all["cl"] <= 0
if neg_mask.any():
    neg_dates = df_all.index[neg_mask].tolist()
    print(f"WARNING: Negative/zero CL=F on {len(neg_dates)} days, dropping")
    df_all = df_all[~neg_mask]

print(f"Common data: {len(df_all)} obs, {df_all.index[0].date()} to {df_all.index[-1].date()}")


# ============================================================
# 2. Node definitions: each node has 3 ratios
# ============================================================
NODE_DEFS = {
    "Au": {
        "ratios": [
            ("gc_dxy", lambda d: d["gc"] / d["dxy"]),
            ("gc_spx", lambda d: d["gc"] / d["spx"]),
            ("gc_cl",  lambda d: d["gc"] / d["cl"]),
        ],
        "label": "GC=F",
    },
    "Dollar": {
        "ratios": [
            ("dxy_gc",  lambda d: d["dxy"] / d["gc"]),
            ("dxy_spx", lambda d: d["dxy"] / d["spx"]),
            ("dxy_cl",  lambda d: d["dxy"] / d["cl"]),
        ],
        "label": "DX-Y.NYB",
    },
    "Equity": {
        "ratios": [
            ("spx_dxy", lambda d: d["spx"] / d["dxy"]),
            ("spx_gc",  lambda d: d["spx"] / d["gc"]),
            ("spx_cl",  lambda d: d["spx"] / d["cl"]),
        ],
        "label": "^GSPC",
    },
    "Commodity": {
        "ratios": [
            ("cl_dxy", lambda d: d["cl"] / d["dxy"]),
            ("cl_gc",  lambda d: d["cl"] / d["gc"]),
            ("cl_spx", lambda d: d["cl"] / d["spx"]),
        ],
        "label": "CL=F",
    },
}

WINDOW = 252


# ============================================================
# 3. Compute eff.dim and Mahalanobis for each node
# ============================================================
def compute_node(name, node_def, df):
    """Compute eff.dim time series, Mahalanobis distances, and extreme events for one node."""
    print(f"\n{'='*60}")
    print(f"Node: {name} ({node_def['label']})")
    print(f"{'='*60}")

    # Build log-return DataFrame for 3 ratios
    ratio_series = {}
    for rname, rfunc in node_def["ratios"]:
        ratio_series[rname] = rfunc(df)
    ratios_df = pd.DataFrame(ratio_series)
    returns = ratios_df.apply(np.log).diff().dropna()
    print(f"  Returns: {len(returns)} obs")

    # Rolling eff.dim
    eff_dim_vals = []
    eff_dim_dates = []
    for i in range(WINDOW, len(returns)):
        w = returns.iloc[i - WINDOW : i]
        cov = w.cov().values
        eigvals = np.linalg.eigvalsh(cov)
        eigvals = eigvals[eigvals > 0]
        p = eigvals / eigvals.sum()
        entropy = -np.sum(p * np.log(p))
        eff_dim_vals.append(np.exp(entropy))
        eff_dim_dates.append(returns.index[i])

    eff_dim = pd.Series(eff_dim_vals, index=eff_dim_dates)
    print(f"  Eff.dim: {len(eff_dim)} pts, mean={eff_dim.mean():.3f}, std={eff_dim.std():.3f}")

    # Mahalanobis distances (full-sample covariance)
    mean_ret = returns.mean().values
    cov_ret = returns.cov().values
    cov_inv = np.linalg.inv(cov_ret)
    mahal_vals = []
    for _, row in returns.iterrows():
        diff = row.values - mean_ret
        mahal_vals.append(np.sqrt(diff @ cov_inv @ diff))
    mahal = pd.Series(mahal_vals, index=returns.index)

    # Extreme events: 99th percentile Mahalanobis, 7-day clustering
    threshold = mahal.quantile(0.99)
    extreme_dates = mahal[mahal > threshold].index.tolist()

    events = []
    if extreme_dates:
        current = {"start": extreme_dates[0], "end": extreme_dates[0],
                   "max_mahal": float(mahal[extreme_dates[0]]), "days": 1}
        for d in extreme_dates[1:]:
            if (d - current["end"]).days <= 7:
                current["end"] = d
                current["max_mahal"] = max(current["max_mahal"], float(mahal[d]))
                current["days"] += 1
            else:
                events.append(current)
                current = {"start": d, "end": d, "max_mahal": float(mahal[d]), "days": 1}
        events.append(current)

    print(f"  Extreme events: {len(events)} (Mahal threshold={threshold:.2f})")

    # Event eff.dim: eff.dim at (or just before) each event start
    event_effdims = []
    for ev in events:
        candidates = eff_dim.index[eff_dim.index <= ev["start"]]
        if len(candidates) == 0:
            continue
        ed = float(eff_dim[candidates[-1]])
        event_effdims.append({
            "event_start": str(ev["start"].date()),
            "max_mahal": ev["max_mahal"],
            "eff_dim_at_event": round(ed, 3),
        })

    return eff_dim, mahal, events, event_effdims


# Run all four nodes
node_results = {}
for name, ndef in NODE_DEFS.items():
    eff_dim, mahal, events, event_effdims = compute_node(name, ndef, df_all)
    node_results[name] = {
        "eff_dim": eff_dim,
        "mahal": mahal,
        "events": events,
        "event_effdims": event_effdims,
    }


# ============================================================
# 4. Per-node statistics and absolute threshold
# ============================================================
print(f"\n{'='*70}")
print("ABSOLUTE THRESHOLD COMPARISON (reproduction)")
print(f"{'='*70}")

per_node_stats = {}
absolute_threshold = {}

for name in NODE_DEFS:
    ed = node_results[name]["eff_dim"]
    ev_eds = [r["eff_dim_at_event"] for r in node_results[name]["event_effdims"]]

    stats_dict = {
        "mean": round(float(ed.mean()), 3),
        "std": round(float(ed.std()), 3),
        "min": round(float(ed.min()), 3),
        "max": round(float(ed.max()), 3),
        "n_pts": len(ed),
    }
    per_node_stats[name] = stats_dict

    if ev_eds:
        min_event_ed = min(ev_eds)
        frac_below = float((ed < min_event_ed).mean())
    else:
        min_event_ed = None
        frac_below = None

    absolute_threshold[name] = {
        "n_events": len(node_results[name]["event_effdims"]),
        "min_event_effdim": round(min_event_ed, 3) if min_event_ed is not None else None,
        "frac_below": round(frac_below, 4) if frac_below is not None else None,
    }

    print(f"  {name:>10}: mean={stats_dict['mean']:.3f}, std={stats_dict['std']:.3f}, "
          f"min_event_ed={absolute_threshold[name]['min_event_effdim']}, "
          f"frac_below={absolute_threshold[name]['frac_below']}")


# ============================================================
# 5. Z-score standardization and threshold
# ============================================================
print(f"\n{'='*70}")
print("Z-SCORE STANDARDIZED THRESHOLD COMPARISON")
print(f"{'='*70}")

zscore_threshold = {}

for name in NODE_DEFS:
    ed = node_results[name]["eff_dim"]
    ev_eds = [r["eff_dim_at_event"] for r in node_results[name]["event_effdims"]]

    mu = float(ed.mean())
    sigma = float(ed.std())

    # z-score of entire eff.dim series
    z_series = (ed - mu) / sigma

    # z-score of event eff.dims
    z_events = [(e - mu) / sigma for e in ev_eds]

    if z_events:
        min_event_z = min(z_events)
        frac_below_z = float((z_series < min_event_z).mean())
        mean_event_z = float(np.mean(z_events))
        median_event_z = float(np.median(z_events))
    else:
        min_event_z = None
        frac_below_z = None
        mean_event_z = None
        median_event_z = None

    zscore_threshold[name] = {
        "mu": round(mu, 3),
        "sigma": round(sigma, 3),
        "min_event_zscore": round(min_event_z, 4) if min_event_z is not None else None,
        "mean_event_zscore": round(mean_event_z, 4) if mean_event_z is not None else None,
        "median_event_zscore": round(median_event_z, 4) if median_event_z is not None else None,
        "frac_below_z": round(frac_below_z, 4) if frac_below_z is not None else None,
        "all_event_zscores": [round(z, 4) for z in z_events] if z_events else [],
    }

    print(f"  {name:>10}: mu={mu:.3f}, sigma={sigma:.3f}, "
          f"min_event_z={zscore_threshold[name]['min_event_zscore']}, "
          f"frac_below_z={zscore_threshold[name]['frac_below_z']}")


# ============================================================
# 6. Percentile threshold
# ============================================================
print(f"\n{'='*70}")
print("PERCENTILE THRESHOLD COMPARISON")
print(f"{'='*70}")

percentile_threshold = {}

for name in NODE_DEFS:
    ed = node_results[name]["eff_dim"]
    ev_eds = [r["eff_dim_at_event"] for r in node_results[name]["event_effdims"]]

    if ev_eds:
        # What percentile is the min event eff.dim in the overall distribution?
        min_event_ed = min(ev_eds)
        percentile_of_min = float(stats.percentileofscore(ed.values, min_event_ed, kind="rank"))

        # Percentile of each event eff.dim
        event_percentiles = [
            round(float(stats.percentileofscore(ed.values, e, kind="rank")), 2)
            for e in ev_eds
        ]
        min_event_percentile = min(event_percentiles)
        mean_event_percentile = float(np.mean(event_percentiles))
        median_event_percentile = float(np.median(event_percentiles))
    else:
        percentile_of_min = None
        event_percentiles = []
        min_event_percentile = None
        mean_event_percentile = None
        median_event_percentile = None

    percentile_threshold[name] = {
        "min_event_percentile": round(percentile_of_min, 2) if percentile_of_min is not None else None,
        "mean_event_percentile": round(mean_event_percentile, 2) if mean_event_percentile is not None else None,
        "median_event_percentile": round(median_event_percentile, 2) if median_event_percentile is not None else None,
        "all_event_percentiles": event_percentiles,
    }

    print(f"  {name:>10}: min_event_pctl={percentile_threshold[name]['min_event_percentile']}%, "
          f"mean_event_pctl={percentile_threshold[name]['mean_event_percentile']}%, "
          f"median_event_pctl={percentile_threshold[name]['median_event_percentile']}%")


# ============================================================
# 7. Convergence verdict
# ============================================================
print(f"\n{'='*70}")
print("CONVERGENCE ANALYSIS")
print(f"{'='*70}")

# Collect min_event_z for all nodes with data
z_values = {}
pctl_values = {}
frac_z_values = {}
frac_abs_values = {}

for name in NODE_DEFS:
    zt = zscore_threshold[name]
    pt = percentile_threshold[name]
    at = absolute_threshold[name]
    if zt["min_event_zscore"] is not None:
        z_values[name] = zt["min_event_zscore"]
    if pt["min_event_percentile"] is not None:
        pctl_values[name] = pt["min_event_percentile"]
    if zt["frac_below_z"] is not None:
        frac_z_values[name] = zt["frac_below_z"]
    if at["frac_below"] is not None:
        frac_abs_values[name] = at["frac_below"]

# Z-score convergence: all pairwise differences < 0.3
z_vals_list = list(z_values.values())
z_names_list = list(z_values.keys())

if len(z_vals_list) >= 2:
    max_z_diff = max(z_vals_list) - min(z_vals_list)
    z_converged = max_z_diff < 0.3
    z_mean = float(np.mean(z_vals_list))
    z_std = float(np.std(z_vals_list))

    print(f"\nZ-score of min event eff.dim:")
    for name, zv in z_values.items():
        print(f"  {name:>10}: {zv:.4f}")
    print(f"  Range: {max_z_diff:.4f} (threshold: 0.3)")
    print(f"  Mean: {z_mean:.4f}, Std: {z_std:.4f}")
    print(f"  CONVERGED: {'YES' if z_converged else 'NO'}")
else:
    z_converged = False
    z_mean = None
    z_std = None
    max_z_diff = None

# Percentile convergence
pctl_vals_list = list(pctl_values.values())
if len(pctl_vals_list) >= 2:
    max_pctl_diff = max(pctl_vals_list) - min(pctl_vals_list)
    pctl_mean = float(np.mean(pctl_vals_list))
    pctl_std = float(np.std(pctl_vals_list))

    print(f"\nPercentile of min event eff.dim:")
    for name, pv in pctl_values.items():
        print(f"  {name:>10}: {pv:.2f}%")
    print(f"  Range: {max_pctl_diff:.2f}pp")
    print(f"  Mean: {pctl_mean:.2f}%, Std: {pctl_std:.2f}%")
    # Converged if range < 5 percentage points
    pctl_converged = max_pctl_diff < 5.0
    print(f"  CONVERGED (range < 5pp): {'YES' if pctl_converged else 'NO'}")
else:
    pctl_converged = False
    pctl_mean = None
    pctl_std = None
    max_pctl_diff = None

# Frac_below_z convergence
frac_z_list = list(frac_z_values.values())
if len(frac_z_list) >= 2:
    max_frac_z_diff = max(frac_z_list) - min(frac_z_list)
    frac_z_mean = float(np.mean(frac_z_list))
    frac_z_std = float(np.std(frac_z_list))

    print(f"\nFrac_below (z-score standardized):")
    for name, fv in frac_z_values.items():
        print(f"  {name:>10}: {fv:.4f} ({fv*100:.2f}%)")
    print(f"  Range: {max_frac_z_diff:.4f}")
    print(f"  Mean: {frac_z_mean:.4f}, Std: {frac_z_std:.4f}")
else:
    frac_z_mean = None
    frac_z_std = None
    max_frac_z_diff = None

# Absolute frac_below comparison
print(f"\nFrac_below (absolute, for reference):")
for name, fv in frac_abs_values.items():
    print(f"  {name:>10}: {fv:.4f} ({fv*100:.2f}%)")
if len(frac_abs_values) >= 2:
    abs_range = max(frac_abs_values.values()) - min(frac_abs_values.values())
    print(f"  Range: {abs_range:.4f}")


# ============================================================
# 8. Overall verdict
# ============================================================
print(f"\n{'='*70}")
print("VERDICT")
print(f"{'='*70}")

# Determine convergence type
if z_converged and pctl_converged:
    verdict = "STRONG_CONVERGENCE"
    interpretation = (
        "Z-score 和 percentile 两个视角均收敛。"
        "全局约束存在，表现为相对位置：极端事件只在 eff.dim 处于"
        "该节点分布的特定相对区域（而非绝对值）时发生。"
        "绝对阈值不同是因为各节点 eff.dim 的尺度不同，"
        "但标准化后的阈值一致。"
    )
elif z_converged or pctl_converged:
    converged_metric = "z-score" if z_converged else "percentile"
    diverged_metric = "percentile" if z_converged else "z-score"
    verdict = "PARTIAL_CONVERGENCE"
    interpretation = (
        f"{converged_metric} 视角收敛但 {diverged_metric} 视角不收敛。"
        f"全局约束可能存在但形式依赖度量方式。"
    )
else:
    verdict = "NO_CONVERGENCE"
    interpretation = (
        "标准化后阈值仍不收敛。约束是节点局部的，"
        "各节点的 eff.dim 下界由该节点自身的结构特性决定，"
        "不存在统一的全局相对阈值。"
    )

print(f"  Convergence verdict: {verdict}")
print(f"  {interpretation}")


# ============================================================
# 9. Save results
# ============================================================
results = {
    "experiment": "relative_threshold_test",
    "description": "四节点 eff.dim 阈值的 z-score / percentile 标准化收敛检验",
    "epistemological_level": "L2",
    "data_range": f"{df_all.index[0].date()} to {df_all.index[-1].date()}",
    "n_common_obs": len(df_all),
    "methodology_notes": {
        "unified_data": (
            "本实验使用四个 ticker (GC=F, DX-Y.NYB, ^GSPC, CL=F) 的时间交集，"
            "与之前各节点独立实验的差异：(1) $ 节点之前用 HG=F(铜) 作为 commodity 代理，"
            "这里统一用 CL=F(原油)；(2) Mahalanobis 距离对每个节点独立计算（各自三条 ratio return），"
            "但时间范围一致。"
        ),
        "prior_results_comparison": {
            "Au_prior": {"min_event_effdim": "1.894 (HG=F代理)", "frac_below": "6.92%"},
            "Dollar_prior": {"min_event_effdim": "1.894 (HG=F代理)", "frac_below": "6.73%"},
            "Equity_prior": {"min_event_effdim": "1.817", "frac_below": "0.77%"},
            "Commodity_prior": {"min_event_effdim": "1.146", "frac_below": "0.81%"},
            "note": "Au和$节点之前用铜(HG=F)作为commodity代理，本实验统一用原油(CL=F)，导致绝对阈值不同",
        },
    },
    "per_node_stats": per_node_stats,
    "absolute_threshold": absolute_threshold,
    "zscore_threshold": zscore_threshold,
    "percentile_threshold": percentile_threshold,
    "convergence_analysis": {
        "zscore": {
            "values": z_values,
            "max_pairwise_diff": round(max_z_diff, 4) if max_z_diff is not None else None,
            "mean": round(z_mean, 4) if z_mean is not None else None,
            "std": round(z_std, 4) if z_std is not None else None,
            "threshold": 0.3,
            "converged": z_converged,
        },
        "percentile": {
            "values": {k: round(v, 2) for k, v in pctl_values.items()},
            "max_pairwise_diff_pp": round(max_pctl_diff, 2) if max_pctl_diff is not None else None,
            "mean": round(pctl_mean, 2) if pctl_mean is not None else None,
            "std": round(pctl_std, 2) if pctl_std is not None else None,
            "threshold_pp": 5.0,
            "converged": pctl_converged,
        },
        "frac_below_z": {
            "values": {k: round(v, 4) for k, v in frac_z_values.items()},
            "mean": round(frac_z_mean, 4) if frac_z_mean is not None else None,
            "std": round(frac_z_std, 4) if frac_z_std is not None else None,
            "max_diff": round(max_frac_z_diff, 4) if max_frac_z_diff is not None else None,
        },
        "frac_below_absolute": {k: round(v, 4) for k, v in frac_abs_values.items()},
    },
    "convergence_verdict": verdict,
    "interpretation": interpretation,
}

output_path = OUTPUT_DIR / "relative_threshold_results.json"
with open(output_path, "w", encoding="utf-8") as f:
    json.dump(results, f, indent=2, ensure_ascii=False)

print(f"\nResults saved to {output_path}")
