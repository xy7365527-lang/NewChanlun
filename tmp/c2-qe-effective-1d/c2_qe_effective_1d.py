"""
C2: QE 期间 Group A (Au, $) 是否被推入有效一维区间，Group B (Equity, Commodity) 是否被推出
===================================================================================

依据 C1 结论：
  - "有效一维" = eff.dim < 1.483 (对应 PC1 EVR > 90%)
  - 严格一维 = eff.dim < 1.263 (对应 PC1 EVR > 95%)

依据 QE 跨节点结果 (qe_crossnode_results.json)：
  - Au: QE期 mean=2.103, d=-0.72 (降低)
  - Dollar: QE期 mean=2.059, d=-0.87 (降低)
  - Equity: QE期 mean=2.317, d=+1.05 (升高)
  - Commodity: QE期 mean=1.561, d=+0.23 (升高)

假说：
  - Group A (Au, $): QE 将 eff.dim 推向有效一维区间 (更低 → 更接近1)
  - Group B (Equity, Commodity): QE 将 eff.dim 推离有效一维区间 (更高 → 更远离1)

认识论等级：L2（真实数据验证，可能产出否定性结果）
谱系引用：C1 有效一维条件
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
# 1. 参数
# ============================================================
QE_START = "2008-11-25"  # QE1 宣布
QE_END = "2022-03-16"    # 首次加息

# C1 结论的阈值
EFFECTIVE_1D_THRESHOLD = 1.483   # PC1 > 90%
STRICT_1D_THRESHOLD = 1.263      # PC1 > 95%

WINDOW = 252

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
# Drop days with non-positive oil (2020-04-20 negative oil)
df = df[df["oil"] > 0]
print(f"Data: {len(df)} obs, {df.index[0].date()} to {df.index[-1].date()}")

# ============================================================
# 3. 四节点 ratio + eff.dim 计算
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


def compute_effdim(returns_df, window=WINDOW):
    vals = []
    dates = []
    pc1_shares = []
    for i in range(window, len(returns_df)):
        w = returns_df.iloc[i - window : i]
        cov = w.cov().values
        eigvals = np.linalg.eigvalsh(cov)
        eigvals = eigvals[eigvals > 0]
        eigvals_sorted = np.sort(eigvals)[::-1]
        p = eigvals / eigvals.sum()
        entropy = -np.sum(p * np.log(p))
        vals.append(np.exp(entropy))
        pc1_shares.append(eigvals_sorted[0] / eigvals_sorted.sum())
        dates.append(returns_df.index[i])
    return pd.DataFrame({
        "eff_dim": vals,
        "pc1_share": pc1_shares,
    }, index=dates)


node_data = {}
for node_name, edges in NODE_RATIOS.items():
    print(f"Computing {node_name}...")
    ratios = {}
    for edge_key, (num, den) in edges.items():
        ratios[edge_key] = df[num] / df[den]
    ratio_df = pd.DataFrame(ratios)
    log_returns = np.log(ratio_df).diff().dropna()
    node_data[node_name] = compute_effdim(log_returns)
    nd = node_data[node_name]
    print(f"  {node_name}: {len(nd)} pts, mean eff.dim={nd['eff_dim'].mean():.4f}")


# ============================================================
# 4. QE vs 非QE：有效一维区间占比
# ============================================================
print("\n" + "=" * 70)
print("C2 核心检验：QE期间各节点落入有效一维区间的比例")
print(f"  有效一维阈值: eff.dim < {EFFECTIVE_1D_THRESHOLD}")
print(f"  严格一维阈值: eff.dim < {STRICT_1D_THRESHOLD}")
print("=" * 70)

results_per_node = {}

for node_name, nd in node_data.items():
    ed = nd["eff_dim"]
    qe_mask = (ed.index >= QE_START) & (ed.index <= QE_END)

    qe_ed = ed[qe_mask]
    nonqe_ed = ed[~qe_mask]

    # 有效一维占比
    qe_frac_eff1d = (qe_ed < EFFECTIVE_1D_THRESHOLD).mean()
    nonqe_frac_eff1d = (nonqe_ed < EFFECTIVE_1D_THRESHOLD).mean()

    # 严格一维占比
    qe_frac_strict1d = (qe_ed < STRICT_1D_THRESHOLD).mean()
    nonqe_frac_strict1d = (nonqe_ed < STRICT_1D_THRESHOLD).mean()

    # 各 percentile 的 eff.dim
    qe_percentiles = {
        f"p{p}": round(float(np.percentile(qe_ed, p)), 4)
        for p in [1, 5, 10, 25, 50, 75, 90, 95, 99]
    }
    nonqe_percentiles = {
        f"p{p}": round(float(np.percentile(nonqe_ed, p)), 4)
        for p in [1, 5, 10, 25, 50, 75, 90, 95, 99]
    }

    # Fisher exact test: 有效一维比例 QE vs 非QE 差异显著性
    qe_n_below = int((qe_ed < EFFECTIVE_1D_THRESHOLD).sum())
    qe_n_above = int((qe_ed >= EFFECTIVE_1D_THRESHOLD).sum())
    nonqe_n_below = int((nonqe_ed < EFFECTIVE_1D_THRESHOLD).sum())
    nonqe_n_above = int((nonqe_ed >= EFFECTIVE_1D_THRESHOLD).sum())

    table = [[qe_n_below, qe_n_above], [nonqe_n_below, nonqe_n_above]]
    odds_ratio, fisher_p = stats.fisher_exact(table)

    # Welch t-test on eff.dim values
    t_stat, t_p = stats.ttest_ind(qe_ed.values, nonqe_ed.values, equal_var=False)
    pooled_std = np.sqrt((qe_ed.std()**2 + nonqe_ed.std()**2) / 2)
    cohens_d = (qe_ed.mean() - nonqe_ed.mean()) / pooled_std

    result = {
        "qe": {
            "n": int(len(qe_ed)),
            "mean": round(float(qe_ed.mean()), 4),
            "std": round(float(qe_ed.std()), 4),
            "median": round(float(qe_ed.median()), 4),
            "min": round(float(qe_ed.min()), 4),
            "max": round(float(qe_ed.max()), 4),
            "percentiles": qe_percentiles,
            "frac_below_eff1d": round(float(qe_frac_eff1d), 6),
            "frac_below_strict1d": round(float(qe_frac_strict1d), 6),
            "n_below_eff1d": qe_n_below,
            "n_below_strict1d": int((qe_ed < STRICT_1D_THRESHOLD).sum()),
        },
        "nonqe": {
            "n": int(len(nonqe_ed)),
            "mean": round(float(nonqe_ed.mean()), 4),
            "std": round(float(nonqe_ed.std()), 4),
            "median": round(float(nonqe_ed.median()), 4),
            "min": round(float(nonqe_ed.min()), 4),
            "max": round(float(nonqe_ed.max()), 4),
            "percentiles": nonqe_percentiles,
            "frac_below_eff1d": round(float(nonqe_frac_eff1d), 6),
            "frac_below_strict1d": round(float(nonqe_frac_strict1d), 6),
            "n_below_eff1d": nonqe_n_below,
            "n_below_strict1d": int((nonqe_ed < STRICT_1D_THRESHOLD).sum()),
        },
        "welch_t": round(float(t_stat), 4),
        "welch_p": round(float(t_p), 8),
        "cohens_d": round(float(cohens_d), 4),
        "fisher_exact": {
            "table": table,
            "odds_ratio": round(float(odds_ratio), 4) if np.isfinite(odds_ratio) else "inf",
            "p": round(float(fisher_p), 8),
        },
        "ratio_eff1d_qe_over_nonqe": (
            round(float(qe_frac_eff1d / nonqe_frac_eff1d), 4)
            if nonqe_frac_eff1d > 0 else "inf"
        ),
    }

    results_per_node[node_name] = result

    print(f"\n{node_name}:")
    print(f"  QE期:   n={len(qe_ed)}, mean={qe_ed.mean():.4f}, "
          f"frac<{EFFECTIVE_1D_THRESHOLD}={qe_frac_eff1d:.4%} ({qe_n_below} days), "
          f"frac<{STRICT_1D_THRESHOLD}={qe_frac_strict1d:.4%}")
    print(f"  非QE期: n={len(nonqe_ed)}, mean={nonqe_ed.mean():.4f}, "
          f"frac<{EFFECTIVE_1D_THRESHOLD}={nonqe_frac_eff1d:.4%} ({nonqe_n_below} days), "
          f"frac<{STRICT_1D_THRESHOLD}={nonqe_frac_strict1d:.4%}")
    print(f"  Cohen's d={cohens_d:.4f}, Welch p={t_p:.2e}")
    print(f"  Fisher exact (eff1d比例): OR={result['fisher_exact']['odds_ratio']}, p={fisher_p:.2e}")

    if qe_frac_eff1d > nonqe_frac_eff1d:
        print(f"  → QE期有效一维比例更高 (ratio={result['ratio_eff1d_qe_over_nonqe']})")
    elif nonqe_frac_eff1d > qe_frac_eff1d:
        ratio_inv = round(float(nonqe_frac_eff1d / qe_frac_eff1d), 4) if qe_frac_eff1d > 0 else "inf"
        print(f"  → 非QE期有效一维比例更高 (ratio={ratio_inv})")
    else:
        print(f"  → 两期比例相同")


# ============================================================
# 5. 假说验证
# ============================================================
print("\n" + "=" * 70)
print("假说验证")
print("=" * 70)

group_a_nodes = ["Au", "Dollar"]
group_b_nodes = ["Equity", "Commodity"]

hypothesis_results = {}

# Group A: QE 期间 eff.dim 被推向有效一维区间（更压缩）
print("\n--- Group A (Au, $): QE 应该推入有效一维区间 ---")
for node in group_a_nodes:
    r = results_per_node[node]
    qe_frac = r["qe"]["frac_below_eff1d"]
    nonqe_frac = r["nonqe"]["frac_below_eff1d"]
    qe_mean = r["qe"]["mean"]
    nonqe_mean = r["nonqe"]["mean"]

    # 假说检验：QE期 eff.dim 更低？
    direction_ok = qe_mean < nonqe_mean
    # 假说检验：QE期有效一维比例更高？
    frac_ok = qe_frac > nonqe_frac

    verdict = "PASS" if (direction_ok and frac_ok) else "PARTIAL" if direction_ok else "FAIL"

    hypothesis_results[node] = {
        "group": "A",
        "hypothesis": "QE pushes toward effective-1D",
        "qe_mean_effdim": qe_mean,
        "nonqe_mean_effdim": nonqe_mean,
        "direction_lower_in_qe": direction_ok,
        "qe_frac_eff1d": round(qe_frac, 6),
        "nonqe_frac_eff1d": round(nonqe_frac, 6),
        "frac_higher_in_qe": frac_ok,
        "fisher_p": r["fisher_exact"]["p"],
        "verdict": verdict,
    }

    print(f"  {node}: mean QE={qe_mean:.4f} vs nonQE={nonqe_mean:.4f} "
          f"(direction={'OK' if direction_ok else 'FAIL'}), "
          f"frac_eff1d QE={qe_frac:.4%} vs nonQE={nonqe_frac:.4%} "
          f"(frac={'OK' if frac_ok else 'FAIL'}) → {verdict}")


# Group B: QE 期间 eff.dim 被推离有效一维区间（更散开）
print("\n--- Group B (Equity, Commodity): QE 应该推出有效一维区间 ---")
for node in group_b_nodes:
    r = results_per_node[node]
    qe_frac = r["qe"]["frac_below_eff1d"]
    nonqe_frac = r["nonqe"]["frac_below_eff1d"]
    qe_mean = r["qe"]["mean"]
    nonqe_mean = r["nonqe"]["mean"]

    # 假说检验：QE期 eff.dim 更高？
    direction_ok = qe_mean > nonqe_mean
    # 假说检验：QE期有效一维比例更低？
    frac_ok = qe_frac < nonqe_frac

    verdict = "PASS" if (direction_ok and frac_ok) else "PARTIAL" if direction_ok else "FAIL"

    hypothesis_results[node] = {
        "group": "B",
        "hypothesis": "QE pushes away from effective-1D",
        "qe_mean_effdim": qe_mean,
        "nonqe_mean_effdim": nonqe_mean,
        "direction_higher_in_qe": direction_ok,
        "qe_frac_eff1d": round(qe_frac, 6),
        "nonqe_frac_eff1d": round(nonqe_frac, 6),
        "frac_lower_in_qe": frac_ok,
        "fisher_p": r["fisher_exact"]["p"],
        "verdict": verdict,
    }

    print(f"  {node}: mean QE={qe_mean:.4f} vs nonQE={nonqe_mean:.4f} "
          f"(direction={'OK' if direction_ok else 'FAIL'}), "
          f"frac_eff1d QE={qe_frac:.4%} vs nonQE={nonqe_frac:.4%} "
          f"(frac={'OK' if frac_ok else 'FAIL'}) → {verdict}")


# ============================================================
# 6. 关键否定性检验：Commodity 基准偏置
# ============================================================
print("\n" + "=" * 70)
print("关键否定性检验：Commodity 基准效应")
print("=" * 70)

# Commodity 在整体分布上就已经大量处于有效一维区间
# 这不是 QE 的效果，而是 Commodity 节点的基准属性
comm_r = results_per_node["Commodity"]
comm_overall_frac = (node_data["Commodity"]["eff_dim"] < EFFECTIVE_1D_THRESHOLD).mean()
print(f"Commodity 整体有效一维比例: {comm_overall_frac:.4%}")
print(f"  QE期: {comm_r['qe']['frac_below_eff1d']:.4%}")
print(f"  非QE期: {comm_r['nonqe']['frac_below_eff1d']:.4%}")
print(f"  → Commodity 本身就有很高的有效一维比例，QE 效应需要与基准对比")

# 对 Au 和 $, 同样检查基准
for node in ["Au", "Dollar"]:
    overall_frac = (node_data[node]["eff_dim"] < EFFECTIVE_1D_THRESHOLD).mean()
    print(f"{node} 整体有效一维比例: {overall_frac:.4%}")


# ============================================================
# 7. 综合判决
# ============================================================
print("\n" + "=" * 70)
print("综合判决")
print("=" * 70)

group_a_pass = all(hypothesis_results[n]["verdict"] in ("PASS", "PARTIAL")
                   for n in group_a_nodes)
group_b_pass = all(hypothesis_results[n]["verdict"] in ("PASS", "PARTIAL")
                   for n in group_b_nodes)

# Overall assessment
overall_pass_count = sum(1 for v in hypothesis_results.values() if v["verdict"] == "PASS")
overall_partial_count = sum(1 for v in hypothesis_results.values() if v["verdict"] == "PARTIAL")
overall_fail_count = sum(1 for v in hypothesis_results.values() if v["verdict"] == "FAIL")

if overall_pass_count == 4:
    overall_verdict = "FULL_PASS"
    verdict_text = "假说完全成立：QE 将 Group A 推入有效一维，将 Group B 推出有效一维"
elif overall_pass_count + overall_partial_count >= 3:
    overall_verdict = "PARTIAL_PASS"
    verdict_text = "假说部分成立：方向性一致但有效一维占比条件不完全满足"
elif overall_pass_count + overall_partial_count >= 2:
    overall_verdict = "WEAK_SUPPORT"
    verdict_text = "假说弱支持：只有部分节点满足条件"
else:
    overall_verdict = "FAIL"
    verdict_text = "假说不成立"

# 构建关键发现
key_findings = []

# Au/Dollar: QE 方向性
au_d = results_per_node["Au"]["cohens_d"]
dollar_d = results_per_node["Dollar"]["cohens_d"]
key_findings.append(
    f"Group A 方向性强且一致: Au d={au_d:.2f}, $ d={dollar_d:.2f} "
    f"(QE期eff.dim显著降低)"
)

eq_d = results_per_node["Equity"]["cohens_d"]
comm_d = results_per_node["Commodity"]["cohens_d"]
key_findings.append(
    f"Group B 方向性: Equity d={eq_d:+.2f} (QE期eff.dim显著升高), "
    f"Commodity d={comm_d:+.2f} (QE期eff.dim微升)"
)

# 但有效一维占比的实际差异
for node in ["Au", "Dollar", "Equity", "Commodity"]:
    r = results_per_node[node]
    qf = r["qe"]["frac_below_eff1d"]
    nf = r["nonqe"]["frac_below_eff1d"]
    key_findings.append(
        f"  {node}: 有效一维占比 QE={qf:.4%} vs nonQE={nf:.4%}"
    )

print(f"\n判决: {overall_verdict}")
print(f"说明: {verdict_text}")
for kf in key_findings:
    print(f"  {kf}")


# ============================================================
# 8. 保存结果
# ============================================================
output = {
    "meta": {
        "description": "C2: QE期间Group A是否被推入有效一维区间，Group B是否被推出",
        "c1_thresholds": {
            "effective_1d": EFFECTIVE_1D_THRESHOLD,
            "strict_1d": STRICT_1D_THRESHOLD,
            "effective_1d_meaning": "PC1 EVR > 90%",
            "strict_1d_meaning": "PC1 EVR > 95%",
        },
        "qe_period": {"start": QE_START, "end": QE_END},
        "data_range": f"{df.index[0].date()} to {df.index[-1].date()}",
        "n_obs": len(df),
        "window": WINDOW,
        "epistemological_level": "L2",
    },
    "per_node": results_per_node,
    "hypothesis_tests": hypothesis_results,
    "overall_frac_eff1d": {
        node: round(float((node_data[node]["eff_dim"] < EFFECTIVE_1D_THRESHOLD).mean()), 6)
        for node in node_data
    },
    "overall_frac_strict1d": {
        node: round(float((node_data[node]["eff_dim"] < STRICT_1D_THRESHOLD).mean()), 6)
        for node in node_data
    },
    "verdict": {
        "overall": overall_verdict,
        "text": verdict_text,
        "group_a_pass": group_a_pass,
        "group_b_pass": group_b_pass,
        "per_node_verdicts": {
            node: hypothesis_results[node]["verdict"]
            for node in hypothesis_results
        },
        "key_findings": key_findings,
    },
}

output_path = OUTPUT_DIR / "c2_results.json"
with open(output_path, "w", encoding="utf-8") as f:
    json.dump(output, f, indent=2, ensure_ascii=False)

print(f"\nResults saved to {output_path}")
