"""
C3 v2: 阈值因果检验 — Mahalanobis 极端事件 vs eff.dim percentile
=================================================================

修正上一版的循环定义问题：
  - 旧版用 "running minimum of eff.dim" 定义折叠事件 → 循环
    (eff.dim创新低天然集中在eff.dim左尾，条件概率必然衰减，不携带信息)
  - 新版用 Mahalanobis 距离极端事件 (top 1%) 定义折叠事件
    → 完全不依赖 eff.dim，是独立的外部冲击度量

实验设计：
  对每个节点：
  1. 252天滚动窗口 → eff.dim (与 C1/C2 一致)
  2. 全样本协方差矩阵 → 每天 Mahalanobis 距离
  3. Mahalanobis > 99th pct → 极端事件天
  4. 连续极端天 (gap ≤ 7天) 聚类为一个事件
  5. P(mahal_extreme | eff.dim > t-th pct) 随 t 从 0.8% 到 50% 的变化
  6. 平坦性检验 (线性回归斜率是否显著异于零)

判定标准：
  - FLAT (THRESHOLD_CAUSAL): 斜率不显著 (p > 0.05)
  - INCREASING (LINEAR_CAUSAL): 斜率显著正 (p < 0.05)
  - DECREASING (ANTI_CAUSAL): 斜率显著负 (p < 0.05)
  - INCONCLUSIVE: 极端事件太少

认识论等级：L2（真实数据验证，可能产出否定性结果）
"""

import json
import warnings
from pathlib import Path

import numpy as np
import pandas as pd
import yfinance as yf
from scipy import stats
from scipy.spatial.distance import mahalanobis

warnings.filterwarnings("ignore")

OUTPUT_DIR = Path(__file__).parent

# ============================================================
# 1. 参数
# ============================================================
WINDOW = 252
THRESHOLDS_PCT = [0.8, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0]
MAHAL_EXTREME_PCT = 99  # top 1%
CLUSTER_GAP_DAYS = 7    # ≤7天 gap 聚类为一个事件

# ============================================================
# 2. 数据下载
# ============================================================
def _dl(ticker, start="2000-01-01", end="2026-03-03"):
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
df = df[df["oil"] > 0]  # Drop negative oil (2020-04-20)
print(f"Data: {len(df)} obs, {df.index[0].date()} to {df.index[-1].date()}")

# ============================================================
# 3. 四节点定义
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


# ============================================================
# 4. eff.dim 计算 (252天滚动窗口，与 C1/C2 一致)
# ============================================================
def compute_effdim(returns_df, window=WINDOW):
    """Compute rolling eff.dim using covariance matrix."""
    vals = []
    dates = []
    for i in range(window, len(returns_df)):
        w = returns_df.iloc[i - window : i]
        cov = w.cov().values
        eigvals = np.linalg.eigvalsh(cov)
        eigvals = eigvals[eigvals > 0]
        p = eigvals / eigvals.sum()
        entropy = -np.sum(p * np.log(p))
        vals.append(np.exp(entropy))
        dates.append(returns_df.index[i])
    return pd.Series(vals, index=dates, name="eff_dim")


# ============================================================
# 5. Mahalanobis 距离计算 (全样本协方差矩阵)
# ============================================================
def compute_mahalanobis(returns_df):
    """Compute Mahalanobis distance for each day using full-sample covariance.

    Returns a Series with the same index as returns_df.
    """
    mu = returns_df.mean().values
    cov = returns_df.cov().values
    cov_inv = np.linalg.inv(cov)

    distances = []
    for i in range(len(returns_df)):
        x = returns_df.iloc[i].values
        d = mahalanobis(x, mu, cov_inv)
        distances.append(d)

    return pd.Series(distances, index=returns_df.index, name="mahal_dist")


# ============================================================
# 6. 极端事件聚类
# ============================================================
def cluster_extreme_events(extreme_mask, max_gap_days=CLUSTER_GAP_DAYS):
    """Cluster consecutive extreme days (gap ≤ max_gap_days) into events.

    Returns:
        events: list of (start_date, end_date, n_days)
        event_day_mask: boolean Series, True for days that are part of an event
    """
    extreme_dates = extreme_mask.index[extreme_mask].tolist()
    if len(extreme_dates) == 0:
        return [], extreme_mask & False  # empty mask

    events = []
    current_start = extreme_dates[0]
    current_end = extreme_dates[0]

    for d in extreme_dates[1:]:
        gap = (d - current_end).days
        if gap <= max_gap_days:
            current_end = d
        else:
            events.append((current_start, current_end))
            current_start = d
            current_end = d
    events.append((current_start, current_end))

    return events, extreme_mask


# ============================================================
# 7. 核心计算
# ============================================================
print("\n" + "=" * 70)
print("Computing eff.dim and Mahalanobis distances for all nodes...")
print("=" * 70)

node_data = {}

for node_name, edges in NODE_RATIOS.items():
    print(f"\n--- {node_name} ---")

    # Build ratio log returns
    ratios = {}
    for edge_key, (num, den) in edges.items():
        ratios[edge_key] = df[num] / df[den]
    ratio_df = pd.DataFrame(ratios)
    log_returns = np.log(ratio_df).diff().dropna()

    # eff.dim (252d rolling window)
    effdim = compute_effdim(log_returns)
    print(f"  eff.dim: {len(effdim)} pts, mean={effdim.mean():.4f}, "
          f"min={effdim.min():.4f}, max={effdim.max():.4f}")

    # Mahalanobis (full-sample covariance)
    mahal = compute_mahalanobis(log_returns)
    print(f"  Mahalanobis: {len(mahal)} pts, mean={mahal.mean():.4f}, "
          f"max={mahal.max():.4f}")

    # Extreme events: top 1% Mahalanobis
    mahal_threshold = np.percentile(mahal, MAHAL_EXTREME_PCT)
    extreme_mask = mahal > mahal_threshold
    n_extreme = int(extreme_mask.sum())
    print(f"  Mahalanobis threshold (99th pct): {mahal_threshold:.4f}")
    print(f"  Extreme days: {n_extreme} ({n_extreme/len(mahal)*100:.1f}%)")

    # Cluster extreme events
    events, _ = cluster_extreme_events(extreme_mask)
    print(f"  Clustered events (gap ≤ {CLUSTER_GAP_DAYS}d): {len(events)}")

    # Align: only use dates where both eff.dim and mahal exist
    common_idx = effdim.index.intersection(mahal.index)
    effdim_aligned = effdim.loc[common_idx]
    mahal_aligned = mahal.loc[common_idx]
    extreme_aligned = extreme_mask.loc[common_idx]

    n_extreme_aligned = int(extreme_aligned.sum())
    print(f"  Aligned: {len(common_idx)} pts, {n_extreme_aligned} extreme days in overlap")

    node_data[node_name] = {
        "effdim": effdim_aligned,
        "mahal": mahal_aligned,
        "extreme_mask": extreme_aligned,
        "mahal_threshold": float(mahal_threshold),
        "n_extreme_days": n_extreme,
        "n_extreme_aligned": n_extreme_aligned,
        "n_clustered_events": len(events),
        "events": events,
        "n_aligned": len(common_idx),
    }


# ============================================================
# 8. 阈值因果检验
# ============================================================
print("\n" + "=" * 70)
print("C3 v2 核心检验: P(mahal_extreme | eff.dim > t-th pct)")
print("=" * 70)

per_node_results = {}

for node_name, nd in node_data.items():
    ed = nd["effdim"]
    extreme = nd["extreme_mask"]

    print(f"\n{'='*50}")
    print(f"{node_name} (n={len(ed)}, extreme_days={nd['n_extreme_aligned']})")
    print(f"{'='*50}")

    # --- P(extreme | eff.dim < 0.8th pct) ---
    pct_08_value = np.percentile(ed, 0.8)
    below_mask = ed <= pct_08_value
    n_below = int(below_mask.sum())
    n_extreme_below = int((below_mask & extreme).sum())
    p_below = n_extreme_below / n_below if n_below > 0 else float("nan")

    print(f"\n  P(extreme | eff.dim < 0.8th pct [{pct_08_value:.4f}]):")
    print(f"    n_below={n_below}, n_extreme_below={n_extreme_below}, "
          f"P={p_below:.6f}")

    below_result = {
        "n_below": n_below,
        "n_extreme_below": n_extreme_below,
        "p_extreme_given_below": round(float(p_below), 8),
        "effdim_at_0.8pct": round(float(pct_08_value), 6),
    }

    # --- P(extreme | eff.dim > t-th pct) for each threshold ---
    conditional_probs = {}
    print(f"\n  {'t (pct)':>10} {'eff.dim@t':>10} {'n_above':>8} {'n_ext':>8} "
          f"{'P(ext|above)':>14}")
    print(f"  {'-'*58}")

    for t in THRESHOLDS_PCT:
        pct_value = np.percentile(ed, t)
        above_mask = ed > pct_value
        n_above = int(above_mask.sum())
        n_extreme_above = int((above_mask & extreme).sum())
        p_extreme = n_extreme_above / n_above if n_above > 0 else float("nan")

        conditional_probs[str(t)] = {
            "n_above": n_above,
            "n_extreme_above": n_extreme_above,
            "p_extreme_given_above": round(float(p_extreme), 8),
            "effdim_at_percentile": round(float(pct_value), 6),
        }

        print(f"  {t:>10.1f} {pct_value:>10.4f} {n_above:>8} {n_extreme_above:>8} "
              f"{p_extreme:>14.6f}")

    # --- Flatness test ---
    probs_seq = [
        conditional_probs[str(t)]["p_extreme_given_above"]
        for t in THRESHOLDS_PCT
    ]

    # Filter valid points
    valid_pairs = [
        (t, p) for t, p in zip(THRESHOLDS_PCT, probs_seq)
        if np.isfinite(p)
    ]

    if len(valid_pairs) < 3:
        flatness = {
            "slope": float("nan"),
            "intercept": float("nan"),
            "r_squared": float("nan"),
            "p_value": float("nan"),
            "verdict": "INCONCLUSIVE",
            "reason": f"Only {len(valid_pairs)} valid data points",
        }
    else:
        x = np.array([np.log10(t) for t, _ in valid_pairs])
        y = np.array([p for _, p in valid_pairs])

        slope, intercept, r_value, p_value, std_err = stats.linregress(x, y)
        r_sq = r_value ** 2

        # CV check for FLAT verdict confidence
        y_mean = np.mean(y)
        y_std = np.std(y, ddof=1) if len(y) > 1 else 0.0
        cv = y_std / y_mean if y_mean > 0 else float("inf")

        # Monotonicity check: count sign changes in consecutive differences
        diffs = np.diff(y)
        n_sign_changes = np.sum(np.diff(np.sign(diffs)) != 0)
        is_monotonic = (n_sign_changes <= 1)  # at most 1 sign change

        if p_value < 0.05 and slope > 0:
            verdict = "INCREASING"
        elif p_value < 0.05 and slope < 0:
            verdict = "DECREASING"
        else:
            # slope not significant → FLAT
            # Extra confidence: CV < 0.3 and no monotonic trend
            verdict = "FLAT"

        flatness = {
            "slope": round(float(slope), 8),
            "intercept": round(float(intercept), 8),
            "r_squared": round(float(r_sq), 8),
            "p_value": round(float(p_value), 8),
            "std_err": round(float(std_err), 8),
            "cv": round(float(cv), 6),
            "is_monotonic": bool(is_monotonic),
            "n_sign_changes": int(n_sign_changes),
            "n_valid": len(valid_pairs),
            "verdict": verdict,
        }

        if verdict == "FLAT" and cv < 0.3 and not is_monotonic:
            flatness["flat_confidence"] = "HIGH"
        elif verdict == "FLAT":
            flatness["flat_confidence"] = "MODERATE"

    print(f"\n  平坦性检验 (P vs log10(t) 线性回归):")
    if "slope" in flatness and np.isfinite(flatness.get("slope", float("nan"))):
        print(f"    slope={flatness['slope']:.8f}, p={flatness['p_value']:.6f}, "
              f"R²={flatness['r_squared']:.6f}")
        if "cv" in flatness:
            print(f"    CV={flatness['cv']:.4f}, monotonic={flatness.get('is_monotonic', 'N/A')}")
    print(f"    → {flatness['verdict']}"
          f"{' (' + flatness.get('flat_confidence', '') + ' confidence)' if flatness.get('flat_confidence') else ''}")

    per_node_results[node_name] = {
        "n_extreme_days": nd["n_extreme_aligned"],
        "n_clustered_events": nd["n_clustered_events"],
        "mahal_threshold_99pct": round(nd["mahal_threshold"], 6),
        "below_0.8pct": below_result,
        "conditional_probs": conditional_probs,
        "flatness_test": flatness,
    }


# ============================================================
# 9. Overall verdict
# ============================================================
print("\n" + "=" * 70)
print("综合判定")
print("=" * 70)

node_verdicts = {}
for node_name in NODE_RATIOS:
    v = per_node_results[node_name]["flatness_test"]["verdict"]
    node_verdicts[node_name] = v
    ft = per_node_results[node_name]["flatness_test"]
    p_str = f"p={ft['p_value']:.4f}" if np.isfinite(ft.get("p_value", float("nan"))) else "N/A"
    print(f"  {node_name}: {v} ({p_str})")

# Count verdicts
flat_count = sum(1 for v in node_verdicts.values() if v == "FLAT")
inc_count = sum(1 for v in node_verdicts.values() if v == "INCREASING")
dec_count = sum(1 for v in node_verdicts.values() if v == "DECREASING")
incon_count = sum(1 for v in node_verdicts.values() if v == "INCONCLUSIVE")

if flat_count >= 3:
    overall = "THRESHOLD_CAUSAL"
    interpretation = (
        "P(mahal_extreme | eff.dim > t) 在 t 从 0.8% 到 50% 过程中保持平坦——"
        "阈值因果成立：一旦进入可折叠相(eff.dim > 阈值)，"
        "折叠概率与 eff.dim 具体值无关。"
    )
elif inc_count >= 3:
    overall = "LINEAR_CAUSAL"
    interpretation = (
        "P(mahal_extreme | eff.dim > t) 随 t 单调递增——"
        "线性因果成立：eff.dim 越高，折叠概率越大，阈值框架不成立。"
    )
elif dec_count >= 3:
    overall = "ANTI_CAUSAL"
    interpretation = (
        "P(mahal_extreme | eff.dim > t) 随 t 单调递减——"
        "反因果：eff.dim 越高，折叠概率越低。"
    )
elif flat_count + inc_count + dec_count < 3:
    overall = "INCONCLUSIVE"
    interpretation = "极端事件太少或结果不一致，无法判断。"
else:
    overall = "MIXED"
    interpretation = (
        f"节点间不一致：FLAT={flat_count}, INCREASING={inc_count}, "
        f"DECREASING={dec_count}, INCONCLUSIVE={incon_count}"
    )

print(f"\n  Overall: {overall}")
print(f"  {interpretation}")


# ============================================================
# 10. Save results
# ============================================================
output = {
    "meta": {
        "description": "C3 v2: 阈值因果检验 — Mahalanobis极端事件 vs eff.dim percentile",
        "fold_definition": f"Mahalanobis distance > {MAHAL_EXTREME_PCT}th percentile (per-node)",
        "effdim_source": f"{WINDOW}d rolling covariance matrix",
        "mahalanobis_source": "Full-sample covariance matrix",
        "cluster_gap_days": CLUSTER_GAP_DAYS,
        "epistemological_level": "L2",
        "data_range": f"{df.index[0].date()} to {df.index[-1].date()}",
        "n_obs_raw": int(len(df)),
        "thresholds_pct": THRESHOLDS_PCT,
        "correction_note": (
            "v1 used 'running minimum of eff.dim' as fold definition — circular. "
            "v2 uses Mahalanobis distance (independent of eff.dim) as fold definition."
        ),
    },
    "per_node": per_node_results,
    "verdict": {
        "overall": overall,
        "per_node": node_verdicts,
        "interpretation": interpretation,
        "counts": {
            "FLAT": flat_count,
            "INCREASING": inc_count,
            "DECREASING": dec_count,
            "INCONCLUSIVE": incon_count,
        },
    },
}

output_path = OUTPUT_DIR / "c3_results.json"
with open(output_path, "w", encoding="utf-8") as f:
    json.dump(output, f, indent=2, ensure_ascii=False)

print(f"\nResults saved to {output_path}")
