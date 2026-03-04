"""
C3-invariance: percentile 阈值的 QE/非QE 不变性检验
=====================================================

假说：阈值本身（~0.8th percentile）在 QE 和非 QE 期间相同。
如果成立，阈值是 K4 拓扑不变量，不受货币政策制度影响。

实验设计：
  QE 期：2008-11-25 至 2022-03-16
  非 QE 期：其余所有日期

  对四个节点分别：
  1. 将 eff.dim 时间序列分为 QE 期和非 QE 期
  2. 将 Mahalanobis 极端事件（top 1%，全样本计算）也分为 QE 期和非 QE 期
  3. QE 期内极端事件：计算其 eff.dim 在 QE 期 eff.dim 分布中的 percentile
  4. 非 QE 期内极端事件：计算其 eff.dim 在非 QE 期 eff.dim 分布中的 percentile
  5. 找到各期的"事件地板"——极端事件的 eff.dim 最低值及其 percentile

判决标准：
  - 四节点 QE/非QE 地板 percentile 差值均 ±0.5% 以内 → CONVERGENT
  - 任一节点差值超过 2% → NOT_CONVERGENT
  - 中间 → INCONCLUSIVE

认识论等级：L2（真实数据验证，可能产出否定性结果）
"""

import json
import warnings
from pathlib import Path

import numpy as np
import pandas as pd
import yfinance as yf
from scipy.spatial.distance import mahalanobis

warnings.filterwarnings("ignore")

OUTPUT_DIR = Path(__file__).parent

# ============================================================
# 1. 参数
# ============================================================
WINDOW = 252
MAHAL_EXTREME_PCT = 99  # top 1%

# QE 期定义（美联储资产负债表扩张期）
QE_START = pd.Timestamp("2008-11-25")
QE_END = pd.Timestamp("2022-03-16")

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
# 4. eff.dim 计算 (252天滚动窗口)
# ============================================================
def compute_effdim(returns_df, window=WINDOW):
    """Compute rolling eff.dim using covariance matrix eigenvalue entropy."""
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
    """Compute Mahalanobis distance using full-sample covariance.

    Mahalanobis 距离跨四节点恒等（K4 对称性），但这里仍按节点的
    三条边计算——结果应一致（交叉检查在 c3_cross_node_check.py 已做）。
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
# 6. QE/非QE 分期掩码
# ============================================================
def make_regime_masks(index):
    """Return (qe_mask, nonqe_mask) boolean Series for the given DatetimeIndex."""
    qe_mask = (index >= QE_START) & (index <= QE_END)
    nonqe_mask = ~qe_mask
    return qe_mask, nonqe_mask


# ============================================================
# 7. percentile 位置计算
# ============================================================
def percentile_of_value(distribution, value):
    """Compute the percentile rank of `value` within `distribution`.

    Returns the fraction of `distribution` values that are <= `value`,
    expressed as a percentage (0-100).
    """
    return float(np.mean(distribution <= value) * 100.0)


# ============================================================
# 8. 核心计算：单节点
# ============================================================
def analyze_node(node_name, edges, df):
    """Run QE/nonQE invariance analysis for one node.

    Returns a dict with results.
    """
    # Build ratio log returns
    ratios = {}
    for edge_key, (num, den) in edges.items():
        ratios[edge_key] = df[num] / df[den]
    ratio_df = pd.DataFrame(ratios)
    log_returns = np.log(ratio_df).diff().dropna()

    # eff.dim (252d rolling window)
    effdim = compute_effdim(log_returns)

    # Mahalanobis (full-sample covariance)
    mahal = compute_mahalanobis(log_returns)

    # Extreme events: top 1% Mahalanobis (全样本)
    mahal_threshold = np.percentile(mahal, MAHAL_EXTREME_PCT)
    extreme_mask = mahal > mahal_threshold

    # Align: only dates where both eff.dim and mahal exist
    common_idx = effdim.index.intersection(mahal.index)
    effdim_a = effdim.loc[common_idx]
    extreme_a = extreme_mask.loc[common_idx]

    # Regime masks
    qe_mask, nonqe_mask = make_regime_masks(common_idx)

    # Split eff.dim by regime
    effdim_qe = effdim_a[qe_mask]
    effdim_nonqe = effdim_a[nonqe_mask]

    # Split extreme events by regime
    extreme_qe = extreme_a[qe_mask]
    extreme_nonqe = extreme_a[nonqe_mask]

    n_extreme_qe = int(extreme_qe.sum())
    n_extreme_nonqe = int(extreme_nonqe.sum())

    result = {
        "n_total": len(common_idx),
        "effdim_stats": {
            "mean": round(float(effdim_a.mean()), 6),
            "std": round(float(effdim_a.std()), 6),
            "min": round(float(effdim_a.min()), 6),
            "max": round(float(effdim_a.max()), 6),
        },
        "mahal_threshold_99pct": round(float(mahal_threshold), 6),
    }

    # --- QE period ---
    qe_result = {
        "n_days": int(qe_mask.sum()),
        "n_extreme": n_extreme_qe,
        "effdim_mean": round(float(effdim_qe.mean()), 6) if len(effdim_qe) > 0 else None,
        "effdim_std": round(float(effdim_qe.std()), 6) if len(effdim_qe) > 0 else None,
    }

    if n_extreme_qe > 0:
        extreme_effdim_qe = effdim_qe[extreme_qe]
        floor_val_qe = float(extreme_effdim_qe.min())
        floor_pct_qe = percentile_of_value(effdim_qe.values, floor_val_qe)

        # Also compute percentile of each extreme event
        extreme_pcts_qe = [
            percentile_of_value(effdim_qe.values, v)
            for v in extreme_effdim_qe.values
        ]

        qe_result["event_floor_effdim"] = round(floor_val_qe, 6)
        qe_result["event_floor_percentile_in_qe"] = round(floor_pct_qe, 4)
        qe_result["extreme_effdim_median"] = round(float(np.median(extreme_effdim_qe)), 6)
        qe_result["extreme_percentile_median"] = round(float(np.median(extreme_pcts_qe)), 4)
        qe_result["extreme_percentile_mean"] = round(float(np.mean(extreme_pcts_qe)), 4)
    else:
        qe_result["event_floor_effdim"] = None
        qe_result["event_floor_percentile_in_qe"] = None
        qe_result["extreme_effdim_median"] = None
        qe_result["extreme_percentile_median"] = None
        qe_result["extreme_percentile_mean"] = None

    # --- Non-QE period ---
    nonqe_result = {
        "n_days": int(nonqe_mask.sum()),
        "n_extreme": n_extreme_nonqe,
        "effdim_mean": round(float(effdim_nonqe.mean()), 6) if len(effdim_nonqe) > 0 else None,
        "effdim_std": round(float(effdim_nonqe.std()), 6) if len(effdim_nonqe) > 0 else None,
    }

    if n_extreme_nonqe > 0:
        extreme_effdim_nonqe = effdim_nonqe[extreme_nonqe]
        floor_val_nonqe = float(extreme_effdim_nonqe.min())
        floor_pct_nonqe = percentile_of_value(effdim_nonqe.values, floor_val_nonqe)

        extreme_pcts_nonqe = [
            percentile_of_value(effdim_nonqe.values, v)
            for v in extreme_effdim_nonqe.values
        ]

        nonqe_result["event_floor_effdim"] = round(floor_val_nonqe, 6)
        nonqe_result["event_floor_percentile_in_nonqe"] = round(floor_pct_nonqe, 4)
        nonqe_result["extreme_effdim_median"] = round(float(np.median(extreme_effdim_nonqe)), 6)
        nonqe_result["extreme_percentile_median"] = round(float(np.median(extreme_pcts_nonqe)), 4)
        nonqe_result["extreme_percentile_mean"] = round(float(np.mean(extreme_pcts_nonqe)), 4)
    else:
        nonqe_result["event_floor_effdim"] = None
        nonqe_result["event_floor_percentile_in_nonqe"] = None
        nonqe_result["extreme_effdim_median"] = None
        nonqe_result["extreme_percentile_median"] = None
        nonqe_result["extreme_percentile_mean"] = None

    # --- Percentile diff ---
    if (qe_result["event_floor_percentile_in_qe"] is not None
            and nonqe_result["event_floor_percentile_in_nonqe"] is not None):
        pct_diff = (qe_result["event_floor_percentile_in_qe"]
                    - nonqe_result["event_floor_percentile_in_nonqe"])
    else:
        pct_diff = None

    result["qe"] = qe_result
    result["nonqe"] = nonqe_result
    result["percentile_diff"] = round(pct_diff, 4) if pct_diff is not None else None

    return result


# ============================================================
# 9. 运行四节点
# ============================================================
print("\n" + "=" * 70)
print("C3-invariance: percentile 阈值的 QE/非QE 不变性检验")
print(f"QE period: {QE_START.date()} to {QE_END.date()}")
print("=" * 70)

per_node = {}
for node_name, edges in NODE_RATIOS.items():
    print(f"\n--- {node_name} ---")
    result = analyze_node(node_name, edges, df)

    per_node[node_name] = result

    qe = result["qe"]
    nonqe = result["nonqe"]
    print(f"  Total aligned: {result['n_total']} days")
    print(f"  QE:    {qe['n_days']} days, {qe['n_extreme']} extreme events")
    print(f"  NonQE: {nonqe['n_days']} days, {nonqe['n_extreme']} extreme events")

    if qe["event_floor_percentile_in_qe"] is not None:
        print(f"  QE event floor:    eff.dim={qe['event_floor_effdim']:.4f}, "
              f"pct_in_qe={qe['event_floor_percentile_in_qe']:.2f}%")
    else:
        print(f"  QE event floor:    N/A (no extreme events in QE period)")

    if nonqe["event_floor_percentile_in_nonqe"] is not None:
        print(f"  NonQE event floor: eff.dim={nonqe['event_floor_effdim']:.4f}, "
              f"pct_in_nonqe={nonqe['event_floor_percentile_in_nonqe']:.2f}%")
    else:
        print(f"  NonQE event floor: N/A (no extreme events in non-QE period)")

    if result["percentile_diff"] is not None:
        print(f"  → percentile diff = {result['percentile_diff']:+.4f}%")
    else:
        print(f"  → percentile diff = N/A")


# ============================================================
# 10. 综合判定
# ============================================================
print("\n" + "=" * 70)
print("综合判定")
print("=" * 70)

max_abs_diff = 0.0
has_null = False
diffs = {}

for node_name in NODE_RATIOS:
    d = per_node[node_name]["percentile_diff"]
    if d is None:
        has_null = True
        diffs[node_name] = None
        print(f"  {node_name}: N/A (insufficient extreme events)")
    else:
        diffs[node_name] = d
        abs_d = abs(d)
        if abs_d > max_abs_diff:
            max_abs_diff = abs_d
        print(f"  {node_name}: diff = {d:+.4f}%")

if has_null:
    verdict = "INCONCLUSIVE"
    interpretation = "一个或多个节点在某一制度期内没有极端事件，无法比较。"
elif max_abs_diff <= 0.5:
    verdict = "CONVERGENT"
    interpretation = (
        f"四节点 QE/非QE 地板 percentile 差值均在 ±0.5% 以内 "
        f"(max |diff| = {max_abs_diff:.4f}%)。"
        f"阈值 (~0.8th percentile) 是 K4 拓扑不变量，不受 QE 制度影响。"
    )
elif max_abs_diff > 2.0:
    verdict = "NOT_CONVERGENT"
    interpretation = (
        f"存在节点 QE/非QE 地板 percentile 差值超过 2% "
        f"(max |diff| = {max_abs_diff:.4f}%)。"
        f"制度干预移动了阈值，阈值不是拓扑不变量。"
    )
else:
    verdict = "INCONCLUSIVE"
    interpretation = (
        f"差值在 0.5%-2% 之间 (max |diff| = {max_abs_diff:.4f}%)，不可判决。"
    )

print(f"\n  Verdict: {verdict}")
print(f"  {interpretation}")


# ============================================================
# 11. 保存结果
# ============================================================
output = {
    "meta": {
        "description": "C3-invariance: percentile 阈值的 QE/非QE 不变性检验",
        "hypothesis": "阈值 (~0.8th percentile) 在 QE 和非 QE 期间相同 → K4 拓扑不变量",
        "qe_period": f"{QE_START.date()} to {QE_END.date()}",
        "effdim_source": f"{WINDOW}d rolling covariance matrix",
        "mahalanobis_source": "Full-sample covariance matrix",
        "extreme_threshold": f"top {100 - MAHAL_EXTREME_PCT}% Mahalanobis",
        "epistemological_level": "L2",
        "data_range": f"{df.index[0].date()} to {df.index[-1].date()}",
        "n_obs_raw": int(len(df)),
        "convergence_criterion": "±0.5% floor percentile diff across all 4 nodes",
        "divergence_criterion": "any node > 2% floor percentile diff",
    },
    "per_node": per_node,
    "verdict": {
        "overall": verdict,
        "max_abs_diff": round(max_abs_diff, 4),
        "per_node_diffs": {k: round(v, 4) if v is not None else None for k, v in diffs.items()},
        "interpretation": interpretation,
    },
}

results_path = OUTPUT_DIR / "c3_invariance_results.json"
with open(results_path, "w", encoding="utf-8") as f:
    json.dump(output, f, indent=2, ensure_ascii=False)
print(f"\nResults saved to {results_path}")
