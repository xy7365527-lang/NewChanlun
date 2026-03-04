"""
C3 补充检查：极端事件日各节点 eff.dim percentile 的交叉分析

核心问题：折叠是全局事件(K4整体)，但eff.dim是节点局部的。
当某节点处于底部0.8%时，其他节点在哪里？
"""
import warnings
import json
import numpy as np
import pandas as pd
import yfinance as yf
from scipy.spatial.distance import mahalanobis
from pathlib import Path

warnings.filterwarnings("ignore")

OUTPUT_DIR = Path(__file__).parent

def _dl(ticker, start="2000-01-01", end="2026-03-03"):
    raw = yf.download(ticker, start=start, end=end)["Close"]
    if isinstance(raw, pd.DataFrame):
        raw = raw.iloc[:, 0]
    return raw

# ============================================================
# 1. Data
# ============================================================
print("Downloading data...")
gold = _dl("GC=F")
dxy = _dl("DX-Y.NYB")
spx = _dl("^GSPC")
oil = _dl("CL=F")

df = pd.DataFrame({"gold": gold, "dxy": dxy, "spx": spx, "oil": oil}).dropna()
df = df[df["oil"] > 0]
print(f"Data: {len(df)} obs, {df.index[0].date()} to {df.index[-1].date()}")

NODE_RATIOS = {
    "Au": {"e1": ("gold", "dxy"), "e2": ("gold", "spx"), "e3": ("gold", "oil")},
    "Dollar": {"e1": ("dxy", "gold"), "e2": ("dxy", "spx"), "e3": ("dxy", "oil")},
    "Equity": {"e1": ("spx", "dxy"), "e2": ("spx", "gold"), "e3": ("spx", "oil")},
    "Commodity": {"e1": ("oil", "dxy"), "e2": ("oil", "gold"), "e3": ("oil", "spx")},
}

WINDOW = 252
NODES = ["Au", "Dollar", "Equity", "Commodity"]

# ============================================================
# 2. Compute eff.dim for all nodes
# ============================================================
print("Computing eff.dim for all nodes...")
node_effdim = {}

for node_name, edges in NODE_RATIOS.items():
    ratios = {}
    for ek, (n, d) in edges.items():
        ratios[ek] = df[n] / df[d]
    rdf = pd.DataFrame(ratios)
    lr = np.log(rdf).diff().dropna()

    vals, dates = [], []
    for i in range(WINDOW, len(lr)):
        w = lr.iloc[i - WINDOW : i]
        cov = w.cov().values
        ev = np.linalg.eigvalsh(cov)
        ev = ev[ev > 0]
        p = ev / ev.sum()
        vals.append(np.exp(-np.sum(p * np.log(p))))
        dates.append(lr.index[i])
    node_effdim[node_name] = pd.Series(vals, index=dates)
    print(f"  {node_name}: {len(vals)} pts, mean={np.mean(vals):.4f}")

# ============================================================
# 3. Compute Mahalanobis (same for all nodes - K4 symmetry)
# ============================================================
print("\nComputing Mahalanobis distances...")
ratios_au = {}
for ek, (n, d) in NODE_RATIOS["Au"].items():
    ratios_au[ek] = df[n] / df[d]
lr_au = np.log(pd.DataFrame(ratios_au)).diff().dropna()
mu = lr_au.mean().values
cov_full = lr_au.cov().values
cov_inv = np.linalg.inv(cov_full)

mahal_vals = []
for i in range(len(lr_au)):
    x = lr_au.iloc[i].values
    mahal_vals.append(mahalanobis(x, mu, cov_inv))
mahal_s = pd.Series(mahal_vals, index=lr_au.index)

thresh = np.percentile(mahal_s, 99)
extreme_mask = mahal_s > thresh
print(f"Mahalanobis threshold (99pct): {thresh:.4f}")
print(f"Extreme days: {extreme_mask.sum()}")

# ============================================================
# 4. Common index
# ============================================================
common = node_effdim["Au"].index
for n in NODES[1:]:
    common = common.intersection(node_effdim[n].index)
common = common.intersection(mahal_s.index)
print(f"\nCommon index: {len(common)} days")

extreme_common = extreme_mask.loc[common]
extreme_days = common[extreme_common]
print(f"Extreme days in common: {len(extreme_days)}")

# ============================================================
# 5. Core check: percentile of each node at each extreme day
# ============================================================
# Pre-compute percentile function for each node
node_ed = {n: node_effdim[n].loc[common] for n in NODES}
node_pct_08 = {n: np.percentile(node_ed[n], 0.8) for n in NODES}

print("\n" + "=" * 70)
print("EXTREME DAYS WHERE ANY NODE IS BELOW ITS 0.8th PERCENTILE")
print("=" * 70)

# For each extreme day, compute percentile rank for each node
extreme_pcts = {n: [] for n in NODES}
below_08_events = []

for d in extreme_days:
    pcts = {}
    below_nodes = []
    for n in NODES:
        val = node_ed[n].loc[d]
        pct = (node_ed[n] < val).sum() / len(node_ed[n]) * 100
        pcts[n] = {"value": float(val), "percentile": float(pct)}
        extreme_pcts[n].append(float(pct))
        if val <= node_pct_08[n]:
            below_nodes.append(n)

    if below_nodes:
        below_08_events.append({
            "date": str(d.date()),
            "mahalanobis": float(mahal_s.loc[d]),
            "below_nodes": below_nodes,
            "all_pcts": pcts,
        })
        print(f"\n  {d.date()} | Mahal={mahal_s.loc[d]:.2f} | Below 0.8pct: {below_nodes}")
        for n in NODES:
            marker = " <<<" if n in below_nodes else ""
            print(f"    {n:>10}: eff.dim={pcts[n]['value']:.4f}, "
                  f"percentile={pcts[n]['percentile']:.2f}%{marker}")

print(f"\nSummary: {len(below_08_events)}/{len(extreme_days)} extreme days "
      f"have at least one node below 0.8th percentile")

# ============================================================
# 6. Distribution of node percentiles at extreme days
# ============================================================
print("\n" + "=" * 70)
print("PERCENTILE DISTRIBUTION AT ALL EXTREME DAYS")
print("=" * 70)

pct_stats = {}
for n in NODES:
    arr = np.array(extreme_pcts[n])
    stats_n = {
        "mean_pct": round(float(arr.mean()), 2),
        "median_pct": round(float(np.median(arr)), 2),
        "min_pct": round(float(arr.min()), 2),
        "max_pct": round(float(arr.max()), 2),
        "n_below_1pct": int(np.sum(arr < 1)),
        "n_below_5pct": int(np.sum(arr < 5)),
        "n_below_10pct": int(np.sum(arr < 10)),
        "n_below_20pct": int(np.sum(arr < 20)),
        "n_total": len(arr),
    }
    pct_stats[n] = stats_n
    print(f"  {n:>10}: mean={stats_n['mean_pct']:.1f}%, median={stats_n['median_pct']:.1f}%, "
          f"min={stats_n['min_pct']:.1f}%, max={stats_n['max_pct']:.1f}%, "
          f"<1%={stats_n['n_below_1pct']}, <5%={stats_n['n_below_5pct']}, "
          f"<10%={stats_n['n_below_10pct']}, <20%={stats_n['n_below_20pct']}")

# ============================================================
# 7. Cross-node synchrony: when one node is low, where are others?
# ============================================================
print("\n" + "=" * 70)
print("CROSS-NODE SYNCHRONY AT EXTREME DAYS")
print("=" * 70)

# Count: at extreme days, how many nodes are simultaneously in bottom N%
sync_counts = {t: [] for t in [1, 5, 10, 20]}
for d in extreme_days:
    for threshold in [1, 5, 10, 20]:
        n_below = 0
        for n in NODES:
            val = node_ed[n].loc[d]
            pct = (node_ed[n] < val).sum() / len(node_ed[n]) * 100
            if pct < threshold:
                n_below += 1
        sync_counts[threshold].append(n_below)

print("\nAt extreme days, how many of 4 nodes are below N-th percentile?")
sync_results = {}
for threshold in [1, 5, 10, 20]:
    arr = np.array(sync_counts[threshold])
    dist = {str(i): int(np.sum(arr == i)) for i in range(5)}
    sync_results[f"below_{threshold}pct"] = dist
    print(f"\n  Below {threshold}th percentile:")
    for i in range(5):
        count = int(np.sum(arr == i))
        if count > 0:
            print(f"    {i} nodes: {count}/{len(arr)} extreme days ({count/len(arr)*100:.1f}%)")

# ============================================================
# 8. Save results
# ============================================================
results = {
    "meta": {
        "description": "C3 cross-node check: node eff.dim percentiles at Mahalanobis extreme days",
        "question": "Is the unfoldable phase (eff.dim < 0.8pct) a node-local or K4-global state?",
        "n_extreme_days": len(extreme_days),
        "mahal_threshold_99pct": round(float(thresh), 6),
        "node_0.8pct_thresholds": {n: round(float(node_pct_08[n]), 6) for n in NODES},
    },
    "below_08_events": below_08_events,
    "n_below_08_of_total": f"{len(below_08_events)}/{len(extreme_days)}",
    "percentile_distribution_at_extreme": pct_stats,
    "cross_node_synchrony": sync_results,
}

out_path = OUTPUT_DIR / "c3_cross_node_check.json"
with open(out_path, "w", encoding="utf-8") as f:
    json.dump(results, f, indent=2, ensure_ascii=False)

print(f"\nResults saved to {out_path}")
