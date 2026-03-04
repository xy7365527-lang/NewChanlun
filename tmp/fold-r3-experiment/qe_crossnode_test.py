"""
QE跨节点验证：四节点折叠实验中QE异常的系统性检验
================================================================
背景：四节点折叠实验发现三个"异常"段高度重叠于QE时代（2008-2021）：
  1. $节点下降趋势4（2020-2021 QE期）r=+0.28，下降中散开
  2. Equity节点上升趋势2（2009-2020长牛）r=-0.56，上升中压缩
  3. Commodity节点下降趋势4（需求放缓期）r=+0.67，下降中散开

假设：QE同时扭曲了多个节点的正常模式。

认识论等级：L2（真实数据验证）
谱系引用：323号（折叠理论结算）
"""

import json
import warnings
from pathlib import Path
from itertools import combinations

import numpy as np
import pandas as pd
import yfinance as yf
from scipy import stats

warnings.filterwarnings("ignore")

OUTPUT_DIR = Path(__file__).parent

# ============================================================
# 1. QE时段定义
# ============================================================
QE_START = "2008-11-25"  # QE1宣布
QE_END = "2022-03-16"    # 首次加息

# ============================================================
# 2. 数据下载
# ============================================================
def _dl(ticker, start="2000-01-01", end="2026-03-03"):
    raw = yf.download(ticker, start=start, end=end)["Close"]
    if isinstance(raw, pd.DataFrame):
        raw = raw.iloc[:, 0]
    return raw

print("Downloading data for 4 nodes...")
gold = _dl("GC=F")
dxy = _dl("DX-Y.NYB")
spx = _dl("^GSPC")
oil = _dl("CL=F")

df = pd.DataFrame({"gold": gold, "dxy": dxy, "spx": spx, "oil": oil}).dropna()
print(f"Data: {len(df)} obs, {df.index[0].date()} to {df.index[-1].date()}")

# ============================================================
# 3. 四节点ratio定义 + eff.dim计算
# ============================================================
NODE_RATIOS = {
    "Au": {
        "e1": ("gold", "dxy"),     # GC=F/DXY
        "e2": ("gold", "spx"),     # GC=F/^GSPC
        "e3": ("gold", "oil"),     # GC=F/CL=F
    },
    "Dollar": {
        "e1": ("dxy", "gold"),     # DXY/GC=F
        "e2": ("dxy", "spx"),      # DXY/^GSPC
        "e3": ("dxy", "oil"),      # DXY/CL=F
    },
    "Equity": {
        "e1": ("spx", "dxy"),     # ^GSPC/DXY
        "e2": ("spx", "gold"),    # ^GSPC/GC=F
        "e3": ("spx", "oil"),     # ^GSPC/CL=F
    },
    "Commodity": {
        "e1": ("oil", "dxy"),     # CL=F/DXY
        "e2": ("oil", "gold"),    # CL=F/GC=F
        "e3": ("oil", "spx"),     # CL=F/^GSPC
    },
}

WINDOW = 252

def compute_effdim(returns_df, window=WINDOW):
    """计算滚动eff.dim时间序列"""
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
    return pd.Series(vals, index=dates)


# 计算四个节点的eff.dim
node_effdim = {}
for node_name, edges in NODE_RATIOS.items():
    print(f"Computing eff.dim for {node_name} node...")
    ratios = {}
    for edge_key, (num, den) in edges.items():
        ratios[edge_key] = df[num] / df[den]
    ratio_df = pd.DataFrame(ratios)
    log_returns = np.log(ratio_df).diff().dropna()
    node_effdim[node_name] = compute_effdim(log_returns)
    ed = node_effdim[node_name]
    print(f"  {node_name}: {len(ed)} pts, mean={ed.mean():.3f}, std={ed.std():.3f}")


# ============================================================
# 4. QE vs 非QE eff.dim 对比（每个节点）
# ============================================================
print("\n" + "=" * 70)
print("TEST 1: QE vs 非QE eff.dim 对比")
print("=" * 70)

per_node_qe_vs_nonqe = {}

for node_name, ed in node_effdim.items():
    qe_mask = (ed.index >= QE_START) & (ed.index <= QE_END)
    nonqe_mask = ~qe_mask

    qe_data = ed[qe_mask]
    nonqe_data = ed[nonqe_mask]

    # Welch t检验
    t_stat, t_p = stats.ttest_ind(qe_data.values, nonqe_data.values, equal_var=False)
    # Cohen's d
    pooled_std = np.sqrt((qe_data.std()**2 + nonqe_data.std()**2) / 2)
    cohens_d = (qe_data.mean() - nonqe_data.mean()) / pooled_std

    result = {
        "qe": {
            "n": len(qe_data),
            "mean": round(float(qe_data.mean()), 4),
            "std": round(float(qe_data.std()), 4),
            "median": round(float(qe_data.median()), 4),
        },
        "nonqe": {
            "n": len(nonqe_data),
            "mean": round(float(nonqe_data.mean()), 4),
            "std": round(float(nonqe_data.std()), 4),
            "median": round(float(nonqe_data.median()), 4),
        },
        "welch_t": round(float(t_stat), 4),
        "welch_p": round(float(t_p), 6),
        "cohens_d": round(float(cohens_d), 4),
    }
    per_node_qe_vs_nonqe[node_name] = result

    print(f"\n{node_name}:")
    print(f"  QE期:   n={len(qe_data)}, mean={qe_data.mean():.4f}, std={qe_data.std():.4f}")
    print(f"  非QE期: n={len(nonqe_data)}, mean={nonqe_data.mean():.4f}, std={nonqe_data.std():.4f}")
    print(f"  Welch t={t_stat:.4f}, p={t_p:.6f}, Cohen's d={cohens_d:.4f}")
    sig = "***" if t_p < 0.001 else "**" if t_p < 0.01 else "*" if t_p < 0.05 else "n.s."
    print(f"  显著性: {sig}")


# ============================================================
# 5. 趋势内漂移方向（关键检验）
# ============================================================
print("\n" + "=" * 70)
print("TEST 2: 趋势内漂移方向 — QE vs 非QE对比")
print("=" * 70)

# 手工标注趋势段（按节点）
TREND_SEGMENTS = {
    "Au": [
        ("2001-04-01", "2011-09-30", "上升1", "rising"),
        ("2011-10-01", "2015-12-31", "下降1", "falling"),
        ("2018-08-01", "2020-08-31", "上升2", "rising"),
    ],
    "Dollar": [
        ("2001-07-01", "2008-03-31", "下降1", "falling"),
        ("2008-03-01", "2009-03-31", "上升1", "rising"),
        ("2009-03-01", "2011-04-30", "下降2", "falling"),
        ("2011-04-01", "2017-01-31", "上升2", "rising"),
        ("2017-01-01", "2018-02-28", "下降3", "falling"),
        ("2018-02-01", "2020-03-31", "上升3", "rising"),
        ("2020-03-01", "2021-01-31", "下降4", "falling"),
        ("2021-01-01", "2022-10-31", "上升4", "rising"),
    ],
    "Equity": [
        ("2003-03-01", "2007-10-31", "上升1", "rising"),
        ("2007-10-01", "2009-03-31", "下降1", "falling"),
        ("2009-03-01", "2020-02-29", "上升2", "rising"),
        ("2020-02-01", "2020-03-31", "下降2", "falling"),
        ("2020-03-01", "2021-12-31", "上升3", "rising"),
        ("2022-01-01", "2022-10-31", "下降3", "falling"),
        ("2022-10-01", "2025-12-31", "上升4", "rising"),
    ],
    "Commodity": [
        ("2001-11-01", "2008-07-31", "上升1", "rising"),
        ("2008-07-01", "2009-02-28", "下降1", "falling"),
        ("2009-02-01", "2011-04-30", "上升2", "rising"),
        ("2014-06-01", "2016-02-29", "下降2", "falling"),
        ("2016-02-01", "2018-10-31", "上升3", "rising"),
        ("2018-10-01", "2020-04-30", "下降3", "falling"),
        ("2020-04-01", "2022-03-31", "上升4", "rising"),
        ("2022-03-01", "2023-06-30", "下降4", "falling"),
    ],
}


def is_in_qe(start_str, end_str):
    """判断趋势段是否主要落在QE时段内"""
    seg_start = pd.Timestamp(start_str)
    seg_end = pd.Timestamp(end_str)
    qe_s = pd.Timestamp(QE_START)
    qe_e = pd.Timestamp(QE_END)
    overlap_start = max(seg_start, qe_s)
    overlap_end = min(seg_end, qe_e)
    if overlap_start > overlap_end:
        return "nonqe"
    overlap_days = (overlap_end - overlap_start).days
    total_days = (seg_end - seg_start).days
    if total_days == 0:
        return "nonqe"
    overlap_frac = overlap_days / total_days
    if overlap_frac >= 0.5:
        return "qe"
    return "nonqe"


drift_comparison = {}

for node_name, segments in TREND_SEGMENTS.items():
    ed = node_effdim[node_name]
    node_drift = []
    print(f"\n{node_name}:")

    for start, end, name, direction in segments:
        mask = (ed.index >= start) & (ed.index <= end)
        seg = ed[mask]
        if len(seg) < 30:
            print(f"  {name} ({direction}): n={len(seg)} — 数据不足，跳过")
            continue

        time_idx = np.arange(len(seg))
        sp_r, sp_p = stats.spearmanr(time_idx, seg.values)
        qe_label = is_in_qe(start, end)

        record = {
            "segment": name,
            "direction": direction,
            "qe_label": qe_label,
            "n": len(seg),
            "spearman_r": round(float(sp_r), 4),
            "spearman_p": round(float(sp_p), 4),
            "mean_effdim": round(float(seg.mean()), 4),
        }
        node_drift.append(record)

        drift_dir = "散开(+)" if sp_r > 0 else "压缩(-)"
        sig = "*" if sp_p < 0.05 else ""
        print(f"  {name} ({direction}, {qe_label}): r={sp_r:+.4f}{sig} {drift_dir}, "
              f"n={len(seg)}, mean_ed={seg.mean():.3f}")

    drift_comparison[node_name] = node_drift


# 汇总：QE时段的漂移方向是否系统性偏离？
print("\n--- 汇总：QE vs 非QE 漂移方向对比 ---")

all_qe_drifts = []
all_nonqe_drifts = []
qe_rising_drifts = []
qe_falling_drifts = []
nonqe_rising_drifts = []
nonqe_falling_drifts = []

for node_name, records in drift_comparison.items():
    for r in records:
        if r["qe_label"] == "qe":
            all_qe_drifts.append(r["spearman_r"])
            if r["direction"] == "rising":
                qe_rising_drifts.append(r["spearman_r"])
            else:
                qe_falling_drifts.append(r["spearman_r"])
        else:
            all_nonqe_drifts.append(r["spearman_r"])
            if r["direction"] == "rising":
                nonqe_rising_drifts.append(r["spearman_r"])
            else:
                nonqe_falling_drifts.append(r["spearman_r"])

print(f"QE时段趋势段: n={len(all_qe_drifts)}, 平均r={np.mean(all_qe_drifts):.4f}")
print(f"非QE时段趋势段: n={len(all_nonqe_drifts)}, 平均r={np.mean(all_nonqe_drifts):.4f}")

if len(qe_rising_drifts) >= 2 and len(nonqe_rising_drifts) >= 2:
    print(f"\n上升趋势中:")
    print(f"  QE: n={len(qe_rising_drifts)}, 平均r={np.mean(qe_rising_drifts):+.4f}")
    print(f"  非QE: n={len(nonqe_rising_drifts)}, 平均r={np.mean(nonqe_rising_drifts):+.4f}")

if len(qe_falling_drifts) >= 2 and len(nonqe_falling_drifts) >= 2:
    print(f"\n下降趋势中:")
    print(f"  QE: n={len(qe_falling_drifts)}, 平均r={np.mean(qe_falling_drifts):+.4f}")
    print(f"  非QE: n={len(nonqe_falling_drifts)}, 平均r={np.mean(nonqe_falling_drifts):+.4f}")

# Mann-Whitney U 检验（QE vs 非QE 漂移方向差异）
drift_test = {}
if len(all_qe_drifts) >= 3 and len(all_nonqe_drifts) >= 3:
    u_stat, u_p = stats.mannwhitneyu(all_qe_drifts, all_nonqe_drifts, alternative="two-sided")
    print(f"\n全部趋势段 Mann-Whitney U (QE vs 非QE): U={u_stat:.1f}, p={u_p:.4f}")
    drift_test["all_segments"] = {
        "qe_n": len(all_qe_drifts),
        "nonqe_n": len(all_nonqe_drifts),
        "qe_mean_r": round(float(np.mean(all_qe_drifts)), 4),
        "nonqe_mean_r": round(float(np.mean(all_nonqe_drifts)), 4),
        "mann_whitney_U": round(float(u_stat), 1),
        "mann_whitney_p": round(float(u_p), 4),
    }


# ============================================================
# 6. 四节点同步性分析
# ============================================================
print("\n" + "=" * 70)
print("TEST 3: 四节点eff.dim跨节点相关性 — QE vs 非QE")
print("=" * 70)

# 对齐四节点的eff.dim到共同日期
aligned = pd.DataFrame({name: ed for name, ed in node_effdim.items()}).dropna()
print(f"共同日期: {len(aligned)} 天, {aligned.index[0].date()} to {aligned.index[-1].date()}")

qe_mask_aligned = (aligned.index >= QE_START) & (aligned.index <= QE_END)
nonqe_mask_aligned = ~qe_mask_aligned

aligned_qe = aligned[qe_mask_aligned]
aligned_nonqe = aligned[nonqe_mask_aligned]

print(f"QE期: {len(aligned_qe)} 天")
print(f"非QE期: {len(aligned_nonqe)} 天")

# 静态相关矩阵（QE vs 非QE）
node_pairs = list(combinations(node_effdim.keys(), 2))

print("\n--- 静态Spearman相关 ---")
static_corr = {"qe": {}, "nonqe": {}}

for n1, n2 in node_pairs:
    pair_key = f"{n1}_vs_{n2}"

    # QE期
    qe_r, qe_p = stats.spearmanr(aligned_qe[n1].values, aligned_qe[n2].values)
    static_corr["qe"][pair_key] = {
        "rho": round(float(qe_r), 4),
        "p": round(float(qe_p), 6),
    }

    # 非QE期
    nonqe_r, nonqe_p = stats.spearmanr(aligned_nonqe[n1].values, aligned_nonqe[n2].values)
    static_corr["nonqe"][pair_key] = {
        "rho": round(float(nonqe_r), 4),
        "p": round(float(nonqe_p), 6),
    }

    diff = abs(qe_r) - abs(nonqe_r)
    print(f"  {pair_key}: QE rho={qe_r:+.4f}, 非QE rho={nonqe_r:+.4f}, |diff|={diff:+.4f}")


# 滚动相关（120天窗口）
ROLL_WINDOW = 120
print(f"\n--- 滚动相关（{ROLL_WINDOW}天窗口）---")

rolling_corr = {}
for n1, n2 in node_pairs:
    pair_key = f"{n1}_vs_{n2}"
    s1 = aligned[n1]
    s2 = aligned[n2]

    corr_vals = []
    corr_dates = []
    for i in range(ROLL_WINDOW, len(aligned)):
        w1 = s1.iloc[i - ROLL_WINDOW : i].values
        w2 = s2.iloc[i - ROLL_WINDOW : i].values
        r, _ = stats.spearmanr(w1, w2)
        corr_vals.append(r)
        corr_dates.append(aligned.index[i])

    roll_series = pd.Series(corr_vals, index=corr_dates)
    rolling_corr[pair_key] = roll_series

    # QE vs 非QE 滚动相关分布
    roll_qe_mask = (roll_series.index >= QE_START) & (roll_series.index <= QE_END)
    roll_qe = roll_series[roll_qe_mask]
    roll_nonqe = roll_series[~roll_qe_mask]

    if len(roll_qe) > 0 and len(roll_nonqe) > 0:
        t_stat, t_p = stats.ttest_ind(roll_qe.values, roll_nonqe.values, equal_var=False)
        print(f"  {pair_key}: QE滚动mean={roll_qe.mean():.4f}, 非QE滚动mean={roll_nonqe.mean():.4f}, "
              f"Welch p={t_p:.4f}")


# 跨节点同步性指标：平均绝对相关
print("\n--- 跨节点同步性指标 ---")

sync_qe_vals = []
sync_nonqe_vals = []

for pair_key, roll_series in rolling_corr.items():
    roll_qe_mask = (roll_series.index >= QE_START) & (roll_series.index <= QE_END)
    sync_qe_vals.extend(roll_series[roll_qe_mask].abs().values.tolist())
    sync_nonqe_vals.extend(roll_series[~roll_qe_mask].abs().values.tolist())

sync_qe_mean = np.mean(sync_qe_vals)
sync_nonqe_mean = np.mean(sync_nonqe_vals)
sync_t, sync_p = stats.ttest_ind(sync_qe_vals, sync_nonqe_vals, equal_var=False)
sync_d = (sync_qe_mean - sync_nonqe_mean) / np.sqrt(
    (np.std(sync_qe_vals)**2 + np.std(sync_nonqe_vals)**2) / 2
)

print(f"QE期平均|相关|: {sync_qe_mean:.4f}")
print(f"非QE期平均|相关|: {sync_nonqe_mean:.4f}")
print(f"Welch t={sync_t:.4f}, p={sync_p:.6f}, Cohen's d={sync_d:.4f}")

cross_node_correlation = {
    "static_corr": static_corr,
    "rolling_corr_summary": {},
    "sync_metric": {
        "qe_mean_abs_corr": round(float(sync_qe_mean), 4),
        "nonqe_mean_abs_corr": round(float(sync_nonqe_mean), 4),
        "welch_t": round(float(sync_t), 4),
        "welch_p": round(float(sync_p), 6),
        "cohens_d": round(float(sync_d), 4),
    },
}

# 滚动相关摘要
for pair_key, roll_series in rolling_corr.items():
    roll_qe_mask = (roll_series.index >= QE_START) & (roll_series.index <= QE_END)
    roll_qe = roll_series[roll_qe_mask]
    roll_nonqe = roll_series[~roll_qe_mask]

    t_stat_r, t_p_r = stats.ttest_ind(roll_qe.values, roll_nonqe.values, equal_var=False)
    cross_node_correlation["rolling_corr_summary"][pair_key] = {
        "qe_mean": round(float(roll_qe.mean()), 4),
        "qe_std": round(float(roll_qe.std()), 4),
        "nonqe_mean": round(float(roll_nonqe.mean()), 4),
        "nonqe_std": round(float(roll_nonqe.std()), 4),
        "welch_t": round(float(t_stat_r), 4),
        "welch_p": round(float(t_p_r), 6),
    }


# ============================================================
# 7. 综合判决
# ============================================================
print("\n" + "=" * 70)
print("综合判决")
print("=" * 70)

# 判决1：QE是否系统性改变eff.dim水平？
effdim_shift_count = sum(
    1 for v in per_node_qe_vs_nonqe.values()
    if v["welch_p"] < 0.05
)
effdim_shift_verdict = (
    f"显著（{effdim_shift_count}/4节点p<0.05）" if effdim_shift_count >= 3
    else f"部分显著（{effdim_shift_count}/4节点p<0.05）" if effdim_shift_count >= 1
    else "不显著"
)

# 判决2：QE时段漂移方向是否系统性偏离？
drift_verdict_text = "证据不足"
if drift_test.get("all_segments"):
    if drift_test["all_segments"]["mann_whitney_p"] < 0.05:
        drift_verdict_text = "系统性偏离"
    elif drift_test["all_segments"]["mann_whitney_p"] < 0.10:
        drift_verdict_text = "边际显著偏离"
    else:
        drift_verdict_text = "无系统性偏离"

# 判决3：QE是否增强跨节点同步性？
if sync_p < 0.05 and sync_d > 0:
    sync_verdict = "QE增强了跨节点同步性"
elif sync_p < 0.05 and sync_d < 0:
    sync_verdict = "QE降低了跨节点同步性"
else:
    sync_verdict = "QE未显著改变跨节点同步性"

# 综合
anomaly_nodes = []
for node_name, v in per_node_qe_vs_nonqe.items():
    if v["welch_p"] < 0.05:
        direction = "升高" if v["cohens_d"] > 0 else "降低"
        anomaly_nodes.append(f"{node_name}({direction}, d={v['cohens_d']:.2f})")

overall_verdict = (
    f"QE系统性扭曲了{effdim_shift_count}个节点的eff.dim分布。"
    f"漂移方向：{drift_verdict_text}。"
    f"跨节点同步性：{sync_verdict}。"
    f"受影响节点：{', '.join(anomaly_nodes) if anomaly_nodes else '无'}。"
)

print(f"\n1. eff.dim水平变化: {effdim_shift_verdict}")
for node_name, v in per_node_qe_vs_nonqe.items():
    d = v["cohens_d"]
    p = v["welch_p"]
    direction = "QE期升高" if d > 0 else "QE期降低"
    print(f"   {node_name}: {direction}, d={d:.4f}, p={p:.6f}")

print(f"\n2. 漂移方向: {drift_verdict_text}")
print(f"\n3. 跨节点同步性: {sync_verdict}")
print(f"   QE期|corr|={sync_qe_mean:.4f} vs 非QE期|corr|={sync_nonqe_mean:.4f}, d={sync_d:.4f}")

print(f"\n综合判决: {overall_verdict}")


# ============================================================
# 8. 保存结果
# ============================================================
results = {
    "experiment": "QE跨节点验证",
    "qe_period": {"start": QE_START, "end": QE_END},
    "data_range": f"{df.index[0].date()} to {df.index[-1].date()}",
    "n_obs": len(df),
    "per_node_qe_vs_nonqe": per_node_qe_vs_nonqe,
    "drift_comparison": {
        "per_node": drift_comparison,
        "summary": {
            "qe_segments": {
                "n": len(all_qe_drifts),
                "mean_r": round(float(np.mean(all_qe_drifts)), 4) if all_qe_drifts else None,
                "values": [round(float(x), 4) for x in all_qe_drifts],
            },
            "nonqe_segments": {
                "n": len(all_nonqe_drifts),
                "mean_r": round(float(np.mean(all_nonqe_drifts)), 4) if all_nonqe_drifts else None,
                "values": [round(float(x), 4) for x in all_nonqe_drifts],
            },
            "qe_rising": {
                "n": len(qe_rising_drifts),
                "mean_r": round(float(np.mean(qe_rising_drifts)), 4) if qe_rising_drifts else None,
            },
            "qe_falling": {
                "n": len(qe_falling_drifts),
                "mean_r": round(float(np.mean(qe_falling_drifts)), 4) if qe_falling_drifts else None,
            },
            "nonqe_rising": {
                "n": len(nonqe_rising_drifts),
                "mean_r": round(float(np.mean(nonqe_rising_drifts)), 4) if nonqe_rising_drifts else None,
            },
            "nonqe_falling": {
                "n": len(nonqe_falling_drifts),
                "mean_r": round(float(np.mean(nonqe_falling_drifts)), 4) if nonqe_falling_drifts else None,
            },
            "test": drift_test,
        },
    },
    "cross_node_correlation": cross_node_correlation,
    "verdict": {
        "effdim_shift": effdim_shift_verdict,
        "drift_direction": drift_verdict_text,
        "sync_change": sync_verdict,
        "anomaly_nodes": anomaly_nodes,
        "overall": overall_verdict,
    },
}

output_path = OUTPUT_DIR / "qe_crossnode_results.json"
with open(output_path, "w", encoding="utf-8") as f:
    json.dump(results, f, indent=2, ensure_ascii=False)

print(f"\nResults saved to {output_path}")
