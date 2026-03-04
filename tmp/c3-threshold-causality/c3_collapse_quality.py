"""
C3 线二: 坍缩质量分析 — 安静压缩 vs 震荡压缩
=================================================

核心问题: 向心坍缩是"安静的"还是"震荡的"?

- 安静压缩: eff.dim 下降 + 边波动率与坍缩烈度成比例
- 震荡压缩: eff.dim 下降 + 边波动率异常极端 (超出坍缩烈度的正常比例)

坍缩事件定义 (双重条件):
  - Mahalanobis > 99th percentile (K4 极端冲击)
  - 四节点同时 eff.dim < 5th percentile (all-or-nothing 同步压缩)
  → 交集: 8 天 (已由 C3 cross-node check 确认)

实验设计:
  1. 找到 Mahalanobis 极端天 (top 1%) 中, 四节点同时 eff.dim <5th pct 的天
  2. 按连续性聚类 (gap <= 7 日历天 = 同一 cluster)
  3. 对每个 cluster 计算:
     - eff.dim 压缩深度 (z-score)
     - 各节点三条边的波动率
     - 波动率 z-score (相对于全样本同长度滚动窗口)
  4. 对比: 同步压缩天 (8天) vs 非同步极端天 (56天) 的波动率差异
  5. 判决: 压缩质量一致 / 分化 / $ 特异性

认识论等级: L2 (真实数据验证, 可能产出否定性结果)
"""

import json
import warnings
from pathlib import Path

import numpy as np
import pandas as pd
import yfinance as yf
from scipy.spatial.distance import mahalanobis as mahal_dist

warnings.filterwarnings("ignore")

OUTPUT_DIR = Path(__file__).parent

# ============================================================
# 1. 参数
# ============================================================
WINDOW = 252
SYNC_PERCENTILE = 5       # 四节点同步阈值: 5th percentile
CLUSTER_GAP_DAYS = 7      # gap <= 7 日历天 = 同一 cluster
MAHAL_EXTREME_PCT = 99    # Mahalanobis top 1%

# ============================================================
# 2. 数据下载 (与 C3 主脚本一致)
# ============================================================
def _dl(ticker, start="2000-01-01", end="2026-03-04"):
    raw = yf.download(ticker, start=start, end=end)["Close"]
    if isinstance(raw, pd.DataFrame):
        raw = raw.iloc[:, 0]
    return raw


print("Downloading data...")
gold = _dl("GC=F")
dxy = _dl("DX-Y.NYB")
spx = _dl("^GSPC")
oil = _dl("CL=F")

df = pd.DataFrame({"gold": gold, "dxy": dxy, "spx": spx, "oil": oil}).dropna()
df = df[df["oil"] > 0]  # 排除负油价
print(f"Data: {len(df)} obs, {df.index[0].date()} to {df.index[-1].date()}")

# ============================================================
# 3. 四节点定义 (与 C3 主脚本一致)
# ============================================================
NODE_RATIOS = {
    "Au": {
        "e1": ("gold", "dxy"),
        "e2": ("gold", "spx"),
        "e3": ("gold", "oil"),
    },
    "Dollar": {
        "e1": ("dxy", "gold"),
        "e2": ("dxy", "spx"),
        "e3": ("dxy", "oil"),
    },
    "Equity": {
        "e1": ("spx", "dxy"),
        "e2": ("spx", "gold"),
        "e3": ("spx", "oil"),
    },
    "Commodity": {
        "e1": ("oil", "dxy"),
        "e2": ("oil", "gold"),
        "e3": ("oil", "spx"),
    },
}

NODES = ["Au", "Dollar", "Equity", "Commodity"]

# Edge labels for human-readable output
EDGE_LABELS = {
    "Au": {"e1": "Au/DXY", "e2": "Au/SPX", "e3": "Au/OIL"},
    "Dollar": {"e1": "DXY/Au", "e2": "DXY/SPX", "e3": "DXY/OIL"},
    "Equity": {"e1": "SPX/DXY", "e2": "SPX/Au", "e3": "SPX/OIL"},
    "Commodity": {"e1": "OIL/DXY", "e2": "OIL/Au", "e3": "OIL/SPX"},
}

# ============================================================
# 4. 计算各节点 eff.dim 和边对数收益率
# ============================================================
print("\n" + "=" * 70)
print("Computing eff.dim and edge log-returns for all nodes...")
print("=" * 70)

node_effdim = {}
node_edge_logreturns = {}

for node_name, edges in NODE_RATIOS.items():
    ratios = {}
    for edge_key, (num, den) in edges.items():
        ratios[edge_key] = df[num] / df[den]
    ratio_df = pd.DataFrame(ratios)
    log_returns = np.log(ratio_df).diff().dropna()
    node_edge_logreturns[node_name] = log_returns

    vals = []
    dates = []
    for i in range(WINDOW, len(log_returns)):
        w = log_returns.iloc[i - WINDOW : i]
        cov = w.cov().values
        eigvals = np.linalg.eigvalsh(cov)
        eigvals = eigvals[eigvals > 0]
        p = eigvals / eigvals.sum()
        entropy = -np.sum(p * np.log(p))
        vals.append(np.exp(entropy))
        dates.append(log_returns.index[i])
    node_effdim[node_name] = pd.Series(vals, index=dates, name=f"eff_dim_{node_name}")
    print(f"  {node_name}: eff.dim {len(vals)} pts, "
          f"mean={np.mean(vals):.4f}, min={np.min(vals):.4f}, max={np.max(vals):.4f}")

# ============================================================
# 5. Mahalanobis distance (K4 symmetry: compute once via Au node)
# ============================================================
print("\n" + "=" * 70)
print("Computing Mahalanobis distance (K4 symmetry)...")
print("=" * 70)

lr_au = node_edge_logreturns["Au"]
mu_au = lr_au.mean().values
cov_au = lr_au.cov().values
cov_inv_au = np.linalg.inv(cov_au)

mahal_vals = []
for i in range(len(lr_au)):
    x = lr_au.iloc[i].values
    mahal_vals.append(mahal_dist(x, mu_au, cov_inv_au))
mahal_s = pd.Series(mahal_vals, index=lr_au.index)

mahal_99 = np.percentile(mahal_s, MAHAL_EXTREME_PCT)
extreme_mask = mahal_s > mahal_99
n_extreme = int(extreme_mask.sum())
print(f"Mahalanobis 99th pct threshold: {mahal_99:.4f}")
print(f"Extreme days (Mahal > 99th pct): {n_extreme}")

# ============================================================
# 6. Common index and sync identification
# ============================================================
common_idx = node_effdim["Au"].index
for n in NODES[1:]:
    common_idx = common_idx.intersection(node_effdim[n].index)
common_idx = common_idx.intersection(mahal_s.index)
print(f"\nCommon index: {len(common_idx)} days")

# Node eff.dim on common index
node_ed_common = {n: node_effdim[n].loc[common_idx] for n in NODES}

# 5th percentile thresholds (full-sample)
node_pct_thresh = {}
for n in NODES:
    node_pct_thresh[n] = np.percentile(node_ed_common[n], SYNC_PERCENTILE)
    print(f"  {n}: {SYNC_PERCENTILE}th pct = {node_pct_thresh[n]:.6f}")

# Extreme days in common
extreme_common = extreme_mask.loc[common_idx]
extreme_days = common_idx[extreme_common]
print(f"\nExtreme days in common index: {len(extreme_days)}")

# Among extreme days, find those where ALL 4 nodes < 5th pct
sync_extreme_mask = pd.Series(True, index=extreme_days)
for n in NODES:
    sync_extreme_mask = sync_extreme_mask & (node_ed_common[n].loc[extreme_days] < node_pct_thresh[n])

sync_days = extreme_days[sync_extreme_mask]
nonsync_days = extreme_days[~sync_extreme_mask]
n_sync = len(sync_days)
n_nonsync = len(nonsync_days)

print(f"\n--- ALL-OR-NOTHING DECOMPOSITION ---")
print(f"Sync compression days (Mahal extreme + all 4 nodes < 5th pct): {n_sync}")
print(f"Non-sync extreme days (Mahal extreme + NOT all 4 below):       {n_nonsync}")
print(f"Total extreme days:                                             {n_extreme}")

for d in sync_days:
    vals_str = ", ".join(f"{n}={node_ed_common[n].loc[d]:.4f}" for n in NODES)
    print(f"  SYNC {d.date()}: {vals_str} | Mahal={mahal_s.loc[d]:.2f}")

# ============================================================
# 7. Cluster sync days (gap <= 7 calendar days)
# ============================================================
print("\n" + "=" * 70)
print(f"Clustering sync compression days (gap <= {CLUSTER_GAP_DAYS} calendar days)")
print("=" * 70)

clusters = []
if n_sync > 0:
    current_start = sync_days[0]
    current_end = sync_days[0]
    current_days = [sync_days[0]]

    for d in sync_days[1:]:
        gap = (d - current_end).days
        if gap <= CLUSTER_GAP_DAYS:
            current_end = d
            current_days.append(d)
        else:
            clusters.append({
                "start": current_start,
                "end": current_end,
                "days": current_days,
            })
            current_start = d
            current_end = d
            current_days = [d]
    clusters.append({
        "start": current_start,
        "end": current_end,
        "days": current_days,
    })

print(f"Number of clusters: {len(clusters)}")
for i, cl in enumerate(clusters):
    print(f"  Cluster {i+1}: {cl['start'].date()} to {cl['end'].date()}, "
          f"{len(cl['days'])} sync days")

# ============================================================
# 8. Full-sample baselines
# ============================================================
print("\n" + "=" * 70)
print("Computing full-sample baselines...")
print("=" * 70)

# 8a. eff.dim full-sample statistics (per node)
effdim_full_stats = {}
for n in NODES:
    ed = node_ed_common[n]
    effdim_full_stats[n] = {"mean": float(ed.mean()), "std": float(ed.std())}
    print(f"  {n} eff.dim: mean={ed.mean():.4f}, std={ed.std():.4f}")

# 8b. Rolling volatility baseline
def compute_rolling_vol_distribution(log_returns_series, window_size):
    """Distribution of N-day rolling volatility across full sample."""
    if window_size < 2:
        return np.array([])
    arr = log_returns_series.values
    vols = []
    for i in range(len(arr) - window_size + 1):
        w = arr[i : i + window_size]
        if len(w) >= 2:
            vols.append(np.std(w, ddof=1))
    return np.array(vols)


# ============================================================
# 9. Per-cluster analysis
# ============================================================
print("\n" + "=" * 70)
print("Per-cluster collapse quality analysis")
print("=" * 70)

cluster_results = []

for cl_idx, cl in enumerate(clusters):
    cl_id = cl_idx + 1
    cl_start = cl["start"]
    cl_end = cl["end"]
    cl_days = cl["days"]
    n_days = len(cl_days)

    print(f"\n{'='*60}")
    print(f"CLUSTER {cl_id}: {cl_start.date()} to {cl_end.date()} ({n_days} sync days)")
    print(f"{'='*60}")

    # ----------------------------------------------------------
    # 9a. eff.dim 压缩深度
    # ----------------------------------------------------------
    effdim_depth = {}
    effdim_depth_zscore = {}
    effdim_per_day = {}
    for n in NODES:
        cluster_effdim_vals = [float(node_ed_common[n].loc[d]) for d in cl_days]
        cluster_mean = np.mean(cluster_effdim_vals)
        zscore = (cluster_mean - effdim_full_stats[n]["mean"]) / effdim_full_stats[n]["std"]
        effdim_depth[n] = round(float(cluster_mean), 6)
        effdim_depth_zscore[n] = round(float(zscore), 4)
        effdim_per_day[n] = [round(v, 6) for v in cluster_effdim_vals]

    print(f"\n  eff.dim 压缩深度:")
    for n in NODES:
        print(f"    {n}: cluster_mean={effdim_depth[n]:.4f}, "
              f"z-score={effdim_depth_zscore[n]:+.4f} "
              f"(full: mean={effdim_full_stats[n]['mean']:.4f}, "
              f"std={effdim_full_stats[n]['std']:.4f})")
        print(f"         per_day: {effdim_per_day[n]}")

    # ----------------------------------------------------------
    # 9b. 边波动率 / 绝对收益率 z-score
    #     多天 cluster: 用 cluster 内日收益率 std 的 z-score
    #     单天 cluster: 用当天绝对收益率的 z-score (相对于全样本绝对收益率分布)
    # ----------------------------------------------------------
    edge_volatility = {}
    edge_vol_zscore = {}
    measure_type = "vol" if n_days >= 2 else "abs_return"

    for n in NODES:
        lr = node_edge_logreturns[n]
        cluster_lr = lr.loc[lr.index.isin(cl_days)]

        if n_days >= 2:
            # Multi-day: use volatility (std)
            node_vol = {}
            node_vol_z = {}
            for ek in ["e1", "e2", "e3"]:
                label = EDGE_LABELS[n][ek]
                cluster_vol = float(cluster_lr[ek].std(ddof=1))
                node_vol[label] = round(cluster_vol, 8)

                full_vols = compute_rolling_vol_distribution(lr[ek], n_days)
                if len(full_vols) > 1 and np.std(full_vols) > 0:
                    vol_mean = np.mean(full_vols)
                    vol_std = np.std(full_vols, ddof=1)
                    z = (cluster_vol - vol_mean) / vol_std
                    node_vol_z[label] = round(float(z), 4)
                else:
                    node_vol_z[label] = None

            edge_volatility[n] = node_vol
            edge_vol_zscore[n] = node_vol_z
        else:
            # Single day: use absolute return z-score
            node_val = {}
            node_val_z = {}
            for ek in ["e1", "e2", "e3"]:
                label = EDGE_LABELS[n][ek]
                day_return = float(cluster_lr[ek].iloc[0])
                abs_return = abs(day_return)
                node_val[label] = round(abs_return, 8)

                # z-score: abs return relative to full-sample abs return distribution
                full_abs = lr[ek].abs()
                full_mean = float(full_abs.mean())
                full_std = float(full_abs.std())
                if full_std > 0:
                    z = (abs_return - full_mean) / full_std
                    node_val_z[label] = round(float(z), 4)
                else:
                    node_val_z[label] = None

            edge_volatility[n] = node_val
            edge_vol_zscore[n] = node_val_z

    print(f"\n  边{'波动率' if measure_type == 'vol' else '绝对收益率'} "
          f"({'std of log-returns' if measure_type == 'vol' else '|log-return|'} in cluster):")
    for n in NODES:
        print(f"    {n}:")
        for ek in ["e1", "e2", "e3"]:
            label = EDGE_LABELS[n][ek]
            vol = edge_volatility[n].get(label)
            z = edge_vol_zscore[n].get(label)
            z_str = f"z={z:+.4f}" if z is not None else "z=N/A"
            vol_str = f"{vol:.8f}" if vol is not None else "N/A"
            print(f"      {label}: {'vol' if measure_type == 'vol' else '|ret|'}="
                  f"{vol_str}, {z_str}")

    # ----------------------------------------------------------
    # 9c. Mahalanobis values for cluster days
    # ----------------------------------------------------------
    cl_mahal = [float(mahal_s.loc[d]) for d in cl_days]
    mahal_info = {
        "values": [round(v, 4) for v in cl_mahal],
        "mean": round(float(np.mean(cl_mahal)), 4),
        "max": round(float(np.max(cl_mahal)), 4),
        "min": round(float(np.min(cl_mahal)), 4),
    }
    print(f"\n  Mahalanobis: values={[round(v,2) for v in cl_mahal]}, "
          f"mean={np.mean(cl_mahal):.2f}")

    cluster_results.append({
        "id": cl_id,
        "start": str(cl_start.date()),
        "end": str(cl_end.date()),
        "n_days": n_days,
        "sync_dates": [str(d.date()) for d in cl_days],
        "effdim_depth": effdim_depth,
        "effdim_depth_zscore": effdim_depth_zscore,
        "effdim_per_day": effdim_per_day,
        "edge_measure_type": measure_type,
        "edge_volatility": edge_volatility,
        "edge_vol_zscore": edge_vol_zscore,
        "mahalanobis": mahal_info,
    })

# ============================================================
# 10. Non-sync extreme days comparison
# ============================================================
print("\n" + "=" * 70)
print(f"NON-SYNC EXTREME DAYS COMPARISON ({n_nonsync} days)")
print("=" * 70)

nonsync_vol = {}
nonsync_vol_detail = {}
for n in NODES:
    lr = node_edge_logreturns[n]
    nonsync_lr = lr.loc[lr.index.isin(nonsync_days)]

    node_vol = {}
    for ek in ["e1", "e2", "e3"]:
        label = EDGE_LABELS[n][ek]
        if len(nonsync_lr) >= 2:
            v = float(nonsync_lr[ek].std(ddof=1))
            node_vol[label] = round(v, 8)
        else:
            node_vol[label] = None
    nonsync_vol[n] = node_vol

    # Also compute per-day absolute returns for comparison
    abs_returns = {}
    for ek in ["e1", "e2", "e3"]:
        label = EDGE_LABELS[n][ek]
        if len(nonsync_lr) >= 1:
            abs_r = nonsync_lr[ek].abs()
            abs_returns[label] = {
                "mean_abs_return": round(float(abs_r.mean()), 8),
                "max_abs_return": round(float(abs_r.max()), 8),
            }
    nonsync_vol_detail[n] = abs_returns

print("\nNon-sync extreme days edge volatility:")
for n in NODES:
    print(f"  {n}:")
    for label, vol in nonsync_vol[n].items():
        vol_str = f"{vol:.8f}" if vol is not None else "N/A"
        print(f"    {label}: vol={vol_str}")

# Compare sync vs non-sync
print("\n--- SYNC vs NON-SYNC edge vol comparison ---")
if cluster_results:
    # Use combined sync stats
    sync_lr_all = {}
    for n in NODES:
        lr = node_edge_logreturns[n]
        sync_lr_all[n] = lr.loc[lr.index.isin(sync_days)]

    sync_vol_all = {}
    for n in NODES:
        node_vol = {}
        for ek in ["e1", "e2", "e3"]:
            label = EDGE_LABELS[n][ek]
            if len(sync_lr_all[n]) >= 2:
                node_vol[label] = round(float(sync_lr_all[n][ek].std(ddof=1)), 8)
            else:
                node_vol[label] = None
        sync_vol_all[n] = node_vol

    print(f"\n{'Edge':>12} | {'Sync Vol':>12} | {'NonSync Vol':>12} | {'Ratio':>8}")
    print(f"{'':>12}-+-{'':>12}-+-{'':>12}-+-{'':>8}")
    sync_nonsync_ratios = {}
    for n in NODES:
        for ek in ["e1", "e2", "e3"]:
            label = EDGE_LABELS[n][ek]
            sv = sync_vol_all[n].get(label)
            nv = nonsync_vol[n].get(label)
            if sv and nv and nv > 0:
                ratio = sv / nv
                sync_nonsync_ratios[f"{n}_{label}"] = round(ratio, 4)
                print(f"{label:>12} | {sv:>12.8f} | {nv:>12.8f} | {ratio:>8.4f}")


# ============================================================
# 11. Cross-cluster eff.dim depth variation
# ============================================================
print("\n" + "=" * 70)
print("Cross-cluster eff.dim depth variation")
print("=" * 70)

if len(cluster_results) > 1:
    depth_variation = {}
    for n in NODES:
        zscores = [cr["effdim_depth_zscore"][n] for cr in cluster_results]
        mean_z = np.mean(zscores)
        std_z = np.std(zscores, ddof=1) if len(zscores) > 1 else 0.0
        cv_z = abs(std_z / mean_z) if abs(mean_z) > 1e-8 else float("inf")
        depth_variation[n] = {
            "zscores": [round(z, 4) for z in zscores],
            "mean": round(float(mean_z), 4),
            "std": round(float(std_z), 4),
            "cv": round(float(cv_z), 4),
        }
        print(f"  {n}: z-scores={[round(z,4) for z in zscores]}, CV={cv_z:.4f}")

    cvs = [depth_variation[n]["cv"] for n in NODES]
    mean_cv = np.mean(cvs)
    depth_verdict = "uniform" if mean_cv < 0.15 else "variable"
    use_q = (depth_verdict == "variable")
    print(f"\n  Verdict: {depth_verdict.upper()} (mean CV={mean_cv:.4f})")
elif len(cluster_results) == 1:
    depth_variation = {"single_cluster": True}
    depth_verdict = "single_cluster"
    mean_cv = None
    use_q = False
    print("  Single cluster — no cross-cluster variation to assess.")
    print("  → Q value not applicable; use vol z-score directly.")
else:
    depth_variation = {"no_clusters": True}
    depth_verdict = "no_clusters"
    mean_cv = None
    use_q = False
    print("  No clusters found.")

effdim_depth_variation_result = {
    "per_node": depth_variation,
    "cv_mean": round(float(mean_cv), 4) if mean_cv is not None else None,
    "verdict": depth_verdict,
}

# ============================================================
# 12. Q value computation (if applicable)
# ============================================================
if use_q and len(cluster_results) > 1:
    print("\n" + "=" * 70)
    print("Q value computation")
    print("=" * 70)
    for cr in cluster_results:
        q_vals = {}
        for n in NODES:
            depth_z = abs(cr["effdim_depth_zscore"][n])
            if depth_z < 0.01:
                q_vals[n] = "depth_near_zero"
                continue
            node_q = {}
            for label, vol_z in cr["edge_vol_zscore"][n].items():
                if vol_z is not None:
                    node_q[label] = round(vol_z / depth_z, 4)
                else:
                    node_q[label] = None
            q_vals[n] = node_q
        cr["Q_value"] = q_vals
        print(f"  Cluster {cr['id']}: Q = {q_vals}")
else:
    for cr in cluster_results:
        cr["Q_value"] = "Q_not_applicable"


# ============================================================
# 13. Dollar specificity analysis
# ============================================================
print("\n" + "=" * 70)
print("Dollar specificity analysis")
print("=" * 70)

dollar_specificity_results = []
for cr in cluster_results:
    dollar_zscores = []
    other_zscores = []

    for label, z in cr["edge_vol_zscore"].get("Dollar", {}).items():
        if z is not None:
            dollar_zscores.append(z)

    for n in ["Au", "Equity", "Commodity"]:
        for label, z in cr["edge_vol_zscore"].get(n, {}).items():
            if z is not None:
                other_zscores.append(z)

    d_mean = np.mean(dollar_zscores) if dollar_zscores else None
    o_mean = np.mean(other_zscores) if other_zscores else None

    dollar_specificity_results.append({
        "cluster_id": cr["id"],
        "dollar_mean_z": round(float(d_mean), 4) if d_mean is not None else None,
        "other_mean_z": round(float(o_mean), 4) if o_mean is not None else None,
        "dollar_minus_other": round(float(d_mean - o_mean), 4) if d_mean is not None and o_mean is not None else None,
    })

    if d_mean is not None and o_mean is not None:
        print(f"  Cluster {cr['id']}: Dollar mean vol z={d_mean:.4f}, "
              f"Others mean vol z={o_mean:.4f}, diff={d_mean - o_mean:+.4f}")

# Dollar specificity verdict
if len(dollar_specificity_results) > 0:
    diffs = [r["dollar_minus_other"] for r in dollar_specificity_results
             if r["dollar_minus_other"] is not None]
    if len(diffs) > 0:
        mean_diff = np.mean(diffs)
        if mean_diff > 2.0:
            dollar_verdict = "yes"
        elif mean_diff < -2.0:
            dollar_verdict = "no_dollar_suppressed"
        else:
            dollar_verdict = "no"
        print(f"\n  Mean (Dollar - Others) z-score diff: {mean_diff:+.4f}")
        print(f"  Dollar specificity: {dollar_verdict}")
    else:
        dollar_verdict = "inconclusive"
else:
    dollar_verdict = "inconclusive"

# ============================================================
# 14. Overall verdict
# ============================================================
print("\n" + "=" * 70)
print("OVERALL VERDICT")
print("=" * 70)

if len(cluster_results) == 0:
    overall_verdict = "NO_DATA"
    verdict_reason = "No all-or-nothing sync compression clusters found."
else:
    # Collect all vol z-scores across all clusters
    all_vol_z = []
    dollar_vol_z = []
    other_vol_z = []
    for cr in cluster_results:
        for n in NODES:
            for label, z in cr["edge_vol_zscore"].get(n, {}).items():
                if z is not None:
                    all_vol_z.append(z)
                    if n == "Dollar":
                        dollar_vol_z.append(z)
                    else:
                        other_vol_z.append(z)

    if len(all_vol_z) == 0:
        overall_verdict = "INCONCLUSIVE"
        verdict_reason = "Insufficient volatility data."
    else:
        max_abs_z = max(abs(z) for z in all_vol_z)
        mean_abs_z = np.mean([abs(z) for z in all_vol_z])

        # Separate OIL-related and non-OIL edges
        oil_z = []
        nonoil_z = []
        for cr in cluster_results:
            for n in NODES:
                for label, z in cr["edge_vol_zscore"].get(n, {}).items():
                    if z is not None:
                        if "OIL" in label.upper():
                            oil_z.append(z)
                        else:
                            nonoil_z.append(z)

        oil_mean_z = np.mean([abs(z) for z in oil_z]) if oil_z else 0.0
        nonoil_mean_z = np.mean([abs(z) for z in nonoil_z]) if nonoil_z else 0.0

        # Verdict logic
        if dollar_verdict == "yes":
            overall_verdict = "DOLLAR_SPECIFIC"
            verdict_reason = (
                f"Dollar node vol z-scores anomalously high relative to other nodes. "
                f"Dollar specificity detected."
            )
        elif oil_mean_z > 6.0 and nonoil_mean_z < 2.0:
            overall_verdict = "OIL_SPECIFIC_SHOCK"
            verdict_reason = (
                f"OIL-related edges show extreme vol z-scores (mean |z|={oil_mean_z:.2f}) "
                f"while non-OIL edges are quiet (mean |z|={nonoil_mean_z:.2f}). "
                f"Compression is quiet on non-commodity edges — OIL is the shock source, "
                f"not a K4-wide turbulence."
            )
        elif max_abs_z < 4.0 and mean_abs_z < 3.0:
            overall_verdict = "UNIFORM_QUIET"
            verdict_reason = (
                f"All edge vol z-scores in moderate range "
                f"(max |z|={max_abs_z:.2f}, mean |z|={mean_abs_z:.2f}). "
                f"Compression is quiet — volatility proportional to collapse depth."
            )
        elif max_abs_z > 6.0:
            overall_verdict = "DIFFERENTIATED"
            verdict_reason = (
                f"Edge vol z-scores extreme (max |z|={max_abs_z:.2f}, mean |z|={mean_abs_z:.2f}). "
                f"OIL edges: mean |z|={oil_mean_z:.2f}, non-OIL edges: mean |z|={nonoil_mean_z:.2f}."
            )
        else:
            overall_verdict = "MODERATE_TURBULENCE"
            verdict_reason = (
                f"Edge vol z-scores moderately elevated "
                f"(max |z|={max_abs_z:.2f}, mean |z|={mean_abs_z:.2f})."
            )

print(f"  Verdict: {overall_verdict}")
print(f"  {verdict_reason}")


# ============================================================
# 15. Save results
# ============================================================
print("\n" + "=" * 70)
print("Saving results...")
print("=" * 70)

output = {
    "meta": {
        "description": "C3 线二: 坍缩质量分析 — 安静压缩 vs 震荡压缩",
        "question": "向心坍缩事件中, 压缩是安静的还是震荡的?",
        "collapse_event_definition": (
            f"Mahalanobis > {MAHAL_EXTREME_PCT}th percentile "
            f"AND all 4 nodes eff.dim < {SYNC_PERCENTILE}th percentile"
        ),
        "cluster_gap_days": CLUSTER_GAP_DAYS,
        "effdim_window": WINDOW,
        "vol_baseline": "Full-sample rolling N-day window volatility distribution",
        "epistemological_level": "L2",
        "data_range": f"{df.index[0].date()} to {df.index[-1].date()}",
        "n_obs_raw": int(len(df)),
        "n_extreme_days_mahal": n_extreme,
        "n_sync_days": n_sync,
        "n_nonsync_days": n_nonsync,
        "n_clusters": len(clusters),
        "mahal_99pct_threshold": round(float(mahal_99), 4),
    },
    "clusters": cluster_results,
    "nonsync_comparison": {
        "n_nonsync_days": n_nonsync,
        "nonsync_edge_volatility": nonsync_vol,
        "sync_nonsync_vol_ratios": sync_nonsync_ratios if cluster_results else {},
    },
    "effdim_depth_variation": effdim_depth_variation_result,
    "dollar_specificity": dollar_verdict,
    "dollar_specificity_detail": dollar_specificity_results,
    "verdict": overall_verdict,
    "verdict_reason": verdict_reason,
}

output_path = OUTPUT_DIR / "c3_collapse_quality_results.json"
with open(output_path, "w", encoding="utf-8") as f:
    json.dump(output, f, indent=2, ensure_ascii=False, default=str)
print(f"Results saved to {output_path}")


# ============================================================
# 16. Generate report
# ============================================================
report_lines = [
    "# C3 线二: 坍缩质量分析 — 安静压缩 vs 震荡压缩",
    "",
    "## 认识论等级: L2 (真实数据验证)",
    "",
    "## 核心问题",
    "",
    "向心坍缩事件 (Mahalanobis 极端 + 四节点同步 eff.dim 压缩) 中, 压缩是\"安静的\"还是\"震荡的\"?",
    "",
    "- 安静压缩: 边波动率与坍缩烈度成比例 (vol z-score 温和)",
    "- 震荡压缩: 边波动率异常极端 (vol z-score 远超坍缩深度的合理比例)",
    "",
    "## 坍缩事件定义 (双重条件)",
    "",
    f"- Mahalanobis > {MAHAL_EXTREME_PCT}th percentile (K4 极端冲击天)",
    f"- 四节点同时 eff.dim < {SYNC_PERCENTILE}th percentile (同步压缩)",
    f"- 交集: **{n_sync} 天** (极端天 {n_extreme} 天中的子集)",
    "",
    f"## 数据: {df.index[0].date()} to {df.index[-1].date()}, {len(df)} obs",
    "",
]

# Cluster details
for cr in cluster_results:
    report_lines.append(f"### Cluster {cr['id']}: {cr['start']} to {cr['end']} ({cr['n_days']} days)")
    report_lines.append("")
    report_lines.append(f"同步天: {', '.join(cr['sync_dates'])}")
    report_lines.append("")

    # eff.dim depth table
    report_lines.append("#### eff.dim 压缩深度")
    report_lines.append("")
    report_lines.append("| Node | Cluster Mean | Full Mean | z-score |")
    report_lines.append("|------|:--:|:--:|:--:|")
    for n in NODES:
        report_lines.append(
            f"| {n} | {cr['effdim_depth'][n]:.4f} | "
            f"{effdim_full_stats[n]['mean']:.4f} | "
            f"{cr['effdim_depth_zscore'][n]:+.4f} |"
        )
    report_lines.append("")

    # Edge volatility / abs return table
    mtype = cr.get("edge_measure_type", "vol")
    mtype_label = "波动率" if mtype == "vol" else "绝对收益率"
    val_label = "Vol" if mtype == "vol" else "|Ret|"
    report_lines.append(f"#### 边{mtype_label} z-score")
    report_lines.append("")
    report_lines.append(f"| Node | Edge | {val_label} | z-score |")
    report_lines.append("|------|------|:--:|:--:|")
    for n in NODES:
        for ek in ["e1", "e2", "e3"]:
            label = EDGE_LABELS[n][ek]
            vol = cr["edge_volatility"][n].get(label)
            z = cr["edge_vol_zscore"][n].get(label)
            vol_str = f"{vol:.6f}" if vol is not None else "N/A"
            z_str = f"{z:+.4f}" if z is not None else "N/A"
            report_lines.append(f"| {n} | {label} | {vol_str} | {z_str} |")
    report_lines.append("")

    # Mahalanobis
    mc = cr["mahalanobis"]
    report_lines.append(f"#### Mahalanobis")
    report_lines.append("")
    report_lines.append(f"- 值: {mc['values']}")
    report_lines.append(f"- 均值: {mc['mean']}, 最大: {mc['max']}, 最小: {mc['min']}")
    report_lines.append("")

# Sync vs non-sync comparison
if cluster_results:
    report_lines.append("## Sync vs Non-sync 极端天波动率对比")
    report_lines.append("")
    report_lines.append(f"同步压缩天: {n_sync} 天, 非同步极端天: {n_nonsync} 天")
    report_lines.append("")
    report_lines.append("| Edge | Sync Vol | NonSync Vol | Ratio (Sync/NonSync) |")
    report_lines.append("|------|:--:|:--:|:--:|")
    for n in NODES:
        for ek in ["e1", "e2", "e3"]:
            label = EDGE_LABELS[n][ek]
            sv = sync_vol_all[n].get(label) if 'sync_vol_all' in dir() else None
            nv = nonsync_vol[n].get(label)
            key = f"{n}_{label}"
            ratio = sync_nonsync_ratios.get(key)
            sv_str = f"{sv:.6f}" if sv is not None else "N/A"
            nv_str = f"{nv:.6f}" if nv is not None else "N/A"
            r_str = f"{ratio:.4f}" if ratio is not None else "N/A"
            report_lines.append(f"| {label} | {sv_str} | {nv_str} | {r_str} |")
    report_lines.append("")

# Depth variation
report_lines.append("## eff.dim 深度跨 cluster 变异性")
report_lines.append("")
report_lines.append(f"Verdict: **{depth_verdict.upper()}**")
report_lines.append("")

# Dollar specificity
report_lines.append("## Dollar 特异性")
report_lines.append("")
report_lines.append(f"Verdict: **{dollar_verdict.upper()}**")
report_lines.append("")
if dollar_specificity_results:
    report_lines.append("| Cluster | Dollar mean z | Others mean z | Diff |")
    report_lines.append("|---------|:--:|:--:|:--:|")
    for r in dollar_specificity_results:
        d_str = f"{r['dollar_mean_z']:.4f}" if r["dollar_mean_z"] is not None else "N/A"
        o_str = f"{r['other_mean_z']:.4f}" if r["other_mean_z"] is not None else "N/A"
        diff_str = f"{r['dollar_minus_other']:+.4f}" if r["dollar_minus_other"] is not None else "N/A"
        report_lines.append(f"| {r['cluster_id']} | {d_str} | {o_str} | {diff_str} |")
report_lines.append("")

# Overall verdict
report_lines.append("## 综合判定")
report_lines.append("")
report_lines.append(f"**{overall_verdict}**")
report_lines.append("")
report_lines.append(verdict_reason)
report_lines.append("")

# Interpretation
report_lines.extend([
    "## 判决标准参照",
    "",
    "- 所有 cluster 的边 vol z-score 在同一区间 (如 2-4 sigma) → **质量一致** (全部安静压缩)",
    "- 某些 cluster 的 z-score 显著高于其他 (如一个 6 sigma 其他 3 sigma) → **质量分化** (存在震荡压缩)",
    "- $ 节点在某 cluster 中 z-score 异常高而其他节点不异常 → **$ 特异性** ($ 度量功能被特异性质疑)",
    "- 所有节点同步异常 → 只是冲击烈度大, 不特异于 $",
    "",
    "## 边界条件",
    "",
    "结论翻转的条件:",
    "1. Mahalanobis 极端阈值从 99th pct 改变 — 不同极端天子集",
    "2. all-or-nothing 同步阈值从 5th pct 改变 — 改变同步天集合",
    "3. cluster gap 从 7 天改变 — 影响 cluster 划分粒度",
    "4. 波动率 z-score 基准窗口长度 — 影响基准分布",
    "",
    "## 谱系引用",
    "",
    "- C3 v2: 阈值因果检验 (THRESHOLD_CAUSAL, 4/4 FLAT)",
    "- C3 cross-node check: all-or-nothing 同步确认 (8/64 全同步, 56/64 全不同步)",
    "- 142 号蜂群: 四节点实验框架",
])

report_path = OUTPUT_DIR / "c3_collapse_quality_report.md"
with open(report_path, "w", encoding="utf-8") as f:
    f.write("\n".join(report_lines))
print(f"Report saved to {report_path}")

print("\nDone.")
