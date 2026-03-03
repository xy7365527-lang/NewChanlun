"""
折叠内在性检验：Agent A / Agent B 交叉比对
============================================

认识论等级：L2（单标的/单时段真实数据验证）

判据一：聚类数量（N_clusters = 4 vs 预期 3）
判据二：聚类特征与中枢状态转换的对应（Fisher 精确检验）
判据三：时间方向性（Layer 1 断裂是否系统性领先 Layer 2 转换）
"""

import json
import math
from datetime import date, timedelta
from pathlib import Path
from itertools import product

# ─── 路径 ────────────────────────────────────────────────────────
BASE = Path(__file__).parent
AGENT_A = BASE / "agent_a_results.json"
AGENT_B = BASE / "agent_b_results.json"
AGENT_A_TS = BASE / "agent_a_timestamps.json"
AGENT_B_TS = BASE / "agent_b_timestamps.json"
OUT_JSON = BASE / "cross_compare_results.json"
OUT_MD = BASE / "cross_compare_summary.md"

ALPHA = 0.05  # 锁定的显著性水平


# ─── 工具函数 ─────────────────────────────────────────────────────

def parse_date(s: str) -> date:
    """YYYY-MM-DD → date"""
    parts = s.split("-")
    return date(int(parts[0]), int(parts[1]), int(parts[2]))


def days_between(d1: date, d2: date) -> int:
    """d2 - d1，日历天"""
    return (d2 - d1).days


def factorial(n: int) -> int:
    if n <= 1:
        return 1
    result = 1
    for i in range(2, n + 1):
        result *= i
    return result


def comb(n: int, k: int) -> int:
    if k < 0 or k > n:
        return 0
    return factorial(n) // (factorial(k) * factorial(n - k))


def hypergeometric_pmf(k: int, N: int, K: int, n: int) -> float:
    """
    P(X=k) in hypergeometric distribution.
    N = population size, K = success states in population,
    n = draws, k = observed successes.
    """
    num = comb(K, k) * comb(N - K, n - k)
    den = comb(N, n)
    if den == 0:
        return 0.0
    return num / den


def fisher_exact_2x2(table):
    """
    Fisher exact test for a 2x2 contingency table [[a,b],[c,d]].
    Returns two-sided p-value.
    Uses the hypergeometric distribution.
    """
    a, b = table[0]
    c, d = table[1]
    N = a + b + c + d
    K = a + c  # column 1 total
    n = a + b  # row 1 total

    # Observed probability
    p_obs = hypergeometric_pmf(a, N, K, n)

    # Two-sided: sum probabilities <= p_obs
    p_value = 0.0
    for k in range(max(0, n - (N - K)), min(n, K) + 1):
        p_k = hypergeometric_pmf(k, N, K, n)
        if p_k <= p_obs + 1e-15:  # tolerance for floating point
            p_value += p_k

    return min(p_value, 1.0)


def fisher_exact_rxc(table):
    """
    For tables larger than 2x2, we use a Monte Carlo permutation approach
    or fall back to chi-squared. Given small samples, we use an exhaustive
    approach for small tables, or chi-squared approximation.

    For this experiment: we compute chi-squared with Yates correction
    and also provide a note about sample size limitations.
    """
    rows = len(table)
    cols = len(table[0]) if rows > 0 else 0
    N = sum(sum(row) for row in table)

    if N == 0:
        return 1.0, "empty table"

    row_totals = [sum(row) for row in table]
    col_totals = [sum(table[r][c] for r in range(rows)) for c in range(cols)]

    # Chi-squared statistic
    chi2 = 0.0
    min_expected = float("inf")
    for r in range(rows):
        for c in range(cols):
            expected = row_totals[r] * col_totals[c] / N
            if expected > 0:
                chi2 += (table[r][c] - expected) ** 2 / expected
            if expected < min_expected:
                min_expected = expected

    # Degrees of freedom
    df = (rows - 1) * (cols - 1)

    if df == 0:
        return 1.0, "degenerate (df=0)"

    # Chi-squared p-value approximation using regularized incomplete gamma
    p_value = chi2_survival(chi2, df)

    warning = ""
    if min_expected < 5:
        warning = f"WARNING: min expected cell count = {min_expected:.2f} < 5; chi-squared approximation unreliable"

    return p_value, warning


def chi2_survival(x: float, k: int) -> float:
    """
    P(X > x) for chi-squared distribution with k degrees of freedom.
    Uses the regularized incomplete gamma function approximation.
    """
    if x <= 0:
        return 1.0

    # Use series expansion for regularized lower incomplete gamma
    # P(a, x) = gamma_lower(a, x) / Gamma(a)
    a = k / 2.0
    x2 = x / 2.0

    # For small x, use series expansion
    # For large x, this converges slowly but our values are moderate
    return 1.0 - regularized_gamma_lower(a, x2)


def regularized_gamma_lower(a: float, x: float) -> float:
    """
    Regularized lower incomplete gamma function P(a, x) = gamma(a,x)/Gamma(a).
    Series expansion: P(a,x) = e^(-x) * x^a * sum_{n=0}^{inf} x^n / Gamma(a+n+1)
    """
    if x < 0:
        return 0.0
    if x == 0:
        return 0.0

    # Use series: P(a,x) = e^{-x} * sum_{n=0}^{inf} x^{a+n} / Gamma(a+n+1)
    # = e^{-x} * x^a / Gamma(a) * sum_{n=0}^{inf} x^n / prod_{j=1}^{n}(a+j)

    # Better: use the series P(a,x) = 1 - Q(a,x)
    # Series: P(a,x) = e^{-x} * x^a * sum_{n=0}^{inf} x^n / (a * (a+1) * ... * (a+n))  / Gamma(a)

    # Simplest accurate approach: series sum
    term = 1.0 / a
    total = term
    for n in range(1, 300):
        term *= x / (a + n)
        total += term
        if abs(term) < 1e-15 * abs(total):
            break

    try:
        result = math.exp(-x + a * math.log(x) - math.lgamma(a)) * total
    except (OverflowError, ValueError):
        result = 1.0 if x > a else 0.0

    return max(0.0, min(1.0, result))


def monte_carlo_fisher_rxc(table, n_perms: int = 50000) -> float:
    """
    Monte Carlo permutation test for RxC contingency table.
    More reliable than chi-squared for small expected cell counts.
    """
    import random

    rows = len(table)
    cols = len(table[0])
    N = sum(sum(row) for row in table)

    if N == 0:
        return 1.0

    row_totals = [sum(row) for row in table]
    col_totals = [sum(table[r][c] for r in range(rows)) for c in range(cols)]

    # Observed chi-squared
    chi2_obs = 0.0
    for r in range(rows):
        for c in range(cols):
            expected = row_totals[r] * col_totals[c] / N
            if expected > 0:
                chi2_obs += (table[r][c] - expected) ** 2 / expected

    # Build flat data: each element is a (row_label, col_label)
    data_rows = []
    data_cols = []
    for r in range(rows):
        for c in range(cols):
            for _ in range(table[r][c]):
                data_rows.append(r)
                data_cols.append(c)

    # Permutation test: shuffle column labels, recompute chi-squared
    count_ge = 0
    for _ in range(n_perms):
        random.shuffle(data_cols)
        # Rebuild table
        perm_table = [[0] * cols for _ in range(rows)]
        for i in range(len(data_rows)):
            perm_table[data_rows[i]][data_cols[i]] += 1

        chi2_perm = 0.0
        for r in range(rows):
            for c in range(cols):
                expected = row_totals[r] * col_totals[c] / N
                if expected > 0:
                    chi2_perm += (perm_table[r][c] - expected) ** 2 / expected

        if chi2_perm >= chi2_obs - 1e-10:
            count_ge += 1

    return count_ge / n_perms


def sign_test_p_value(values, test_median_gt_zero: bool = True) -> float:
    """
    Sign test: test whether median > 0 (one-sided).
    Returns p-value.
    """
    n_pos = sum(1 for v in values if v > 0)
    n_neg = sum(1 for v in values if v < 0)
    n = n_pos + n_neg  # exclude zeros

    if n == 0:
        return 1.0

    # Under H0: median = 0, P(positive) = 0.5
    # One-sided: P(X >= n_pos) where X ~ Binomial(n, 0.5)
    if test_median_gt_zero:
        # p = P(X >= n_pos)
        p = 0.0
        for k in range(n_pos, n + 1):
            p += comb(n, k) * (0.5 ** n)
        return p
    else:
        # Two-sided
        p = 0.0
        for k in range(n_pos, n + 1):
            p += comb(n, k) * (0.5 ** n)
        return min(2.0 * p, 1.0)


# ─── 数据加载 ─────────────────────────────────────────────────────

with open(AGENT_A, "r", encoding="utf-8") as f:
    a_data = json.load(f)

with open(AGENT_B, "r", encoding="utf-8") as f:
    b_data = json.load(f)

with open(AGENT_A_TS, "r", encoding="utf-8") as f:
    T_A_raw = json.load(f)

with open(AGENT_B_TS, "r", encoding="utf-8") as f:
    T_B_raw = json.load(f)

# 解析
T_A = [parse_date(s) for s in T_A_raw]  # 24 breakpoints
T_B_records = T_B_raw  # 12 transitions with metadata
T_B = [parse_date(rec["date"]) for rec in T_B_records]  # 12 dates

breakpoints = a_data["breakpoints"]
clusters_info = a_data["clusters"]


# ─── 判据一：聚类数量 ─────────────────────────────────────────────

n_clusters = a_data["metadata"]["n_clusters"]
silhouette = a_data["metadata"]["average_silhouette"]
cluster_confidence = a_data["metadata"]["clustering_confidence"]

criterion_1 = {
    "name": "聚类数量",
    "expected": 3,
    "observed": n_clusters,
    "passed": n_clusters == 3,
    "silhouette_score": silhouette,
    "clustering_confidence": cluster_confidence,
    "analysis": None,
}

# 分析：4个聚类中有2个是单例（cluster 2: 2008-10-03, cluster 3: 2011-08-05）
singleton_clusters = [
    cid for cid, info in clusters_info.items()
    if info["n_members"] == 1
]
non_singleton_clusters = [
    cid for cid, info in clusters_info.items()
    if info["n_members"] > 1
]

analysis_lines = []
analysis_lines.append(f"N_clusters = {n_clusters}，不等于预期的 3。判据一未通过。")
analysis_lines.append(f"轮廓系数 = {silhouette:.3f}，聚类置信度 = {cluster_confidence}。")
analysis_lines.append(f"单例聚类：{len(singleton_clusters)} 个（Cluster {', '.join(singleton_clusters)}）")
analysis_lines.append(f"非单例聚类：{len(non_singleton_clusters)} 个（Cluster {', '.join(non_singleton_clusters)}）")
analysis_lines.append(
    "轮廓系数 0.20 表明数据不支持离散聚类——断裂后行为更接近连续谱。"
    "4 个聚类中 2 个是单例（极端事件：2008 金融危机、2011 欧债危机），"
    "实质上有意义的聚类只有 2 个（Cluster 0 和 Cluster 1）。"
    "这既否定了 N=3 的理论预期，也否定了 N=4 的实质性——"
    "数据更可能是连续谱上的人为切割。"
)
criterion_1["analysis"] = "\n".join(analysis_lines)


# ─── 判据二：聚类特征与中枢状态转换的对应 ──────────────────────────

# 策略：对每个 Agent A 断点，找到时间上最近的 Agent B 状态转换，
# 然后构建 (cluster, transition_type) 列联表。

# 先定义对齐窗口：我们不预设窗口，而是先计算所有 delta，
# 从分布中确定自然窗口。

# 对每个 t_a，找最近的 t_b（不限方向）
alignments = []
for i, t_a in enumerate(T_A):
    bp = breakpoints[i]
    best_delta = None
    best_j = None
    for j, t_b in enumerate(T_B):
        delta = days_between(t_a, t_b)  # t_b - t_a
        if best_delta is None or abs(delta) < abs(best_delta):
            best_delta = delta
            best_j = j
    alignments.append({
        "breakpoint_date": T_A_raw[i],
        "cluster": bp["cluster"],
        "nearest_transition_date": T_B_records[best_j]["date"],
        "nearest_transition_type": f"{T_B_records[best_j]['from_status']}→{T_B_records[best_j]['to_status']}",
        "nearest_transition_zhongshu": T_B_records[best_j]["zhongshu_index"],
        "delta_days": best_delta,
        "abs_delta_days": abs(best_delta),
    })

# 统计 delta 分布
deltas = [a["delta_days"] for a in alignments]
abs_deltas = [a["abs_delta_days"] for a in alignments]

# 分位数（手动计算，不依赖 numpy）
def percentile(sorted_data, p):
    """p in [0, 100]"""
    n = len(sorted_data)
    k = (n - 1) * p / 100.0
    f = math.floor(k)
    c = math.ceil(k)
    if f == c:
        return sorted_data[int(k)]
    return sorted_data[int(f)] * (c - k) + sorted_data[int(c)] * (k - f)


sorted_deltas = sorted(deltas)
sorted_abs_deltas = sorted(abs_deltas)

delta_stats = {
    "n": len(deltas),
    "min": min(deltas),
    "max": max(deltas),
    "mean": sum(deltas) / len(deltas),
    "median": percentile(sorted_deltas, 50),
    "p25": percentile(sorted_deltas, 25),
    "p75": percentile(sorted_deltas, 75),
    "abs_mean": sum(abs_deltas) / len(abs_deltas),
    "abs_median": percentile(sorted_abs_deltas, 50),
    "abs_p25": percentile(sorted_abs_deltas, 25),
    "abs_p75": percentile(sorted_abs_deltas, 75),
    "n_positive": sum(1 for d in deltas if d > 0),
    "n_negative": sum(1 for d in deltas if d < 0),
    "n_zero": sum(1 for d in deltas if d == 0),
    "fraction_positive": sum(1 for d in deltas if d > 0) / len(deltas),
}

# 自然窗口：使用 |delta| 的 75th percentile
natural_window = percentile(sorted_abs_deltas, 75)

# 构建列联表：cluster × transition_type
# 只使用 |delta| <= natural_window 的对齐对
transition_types = sorted(set(a["nearest_transition_type"] for a in alignments))
cluster_ids = sorted(set(a["cluster"] for a in alignments))

# 全量列联表（不过滤窗口）
contingency_full = [[0] * len(transition_types) for _ in range(len(cluster_ids))]
for a in alignments:
    r = cluster_ids.index(a["cluster"])
    c = transition_types.index(a["nearest_transition_type"])
    contingency_full[r][c] += 1

# 窗口内列联表
contingency_windowed = [[0] * len(transition_types) for _ in range(len(cluster_ids))]
n_in_window = 0
for a in alignments:
    if a["abs_delta_days"] <= natural_window:
        r = cluster_ids.index(a["cluster"])
        c = transition_types.index(a["nearest_transition_type"])
        contingency_windowed[r][c] += 1
        n_in_window += 1

# Fisher 精确检验（或 Monte Carlo 替代）
# 对全量表
if len(cluster_ids) == 2 and len(transition_types) == 2:
    p_full = fisher_exact_2x2(contingency_full)
    test_method_full = "Fisher exact (2x2)"
    warning_full = ""
else:
    # Use both chi-squared and Monte Carlo
    p_chi2, warning_full = fisher_exact_rxc(contingency_full)
    p_mc = monte_carlo_fisher_rxc(contingency_full, n_perms=100000)
    p_full = p_mc  # prefer Monte Carlo for small samples
    test_method_full = f"Monte Carlo permutation (100k perms); chi2 p={p_chi2:.4f}"

# 对窗口内表
if n_in_window > 0:
    # Filter out empty rows/cols for windowed table
    non_empty_rows = [r for r in range(len(cluster_ids)) if sum(contingency_windowed[r]) > 0]
    non_empty_cols = [c for c in range(len(transition_types)) if sum(contingency_windowed[r][c] for r in range(len(cluster_ids))) > 0]
    contingency_windowed_filtered = [[contingency_windowed[r][c] for c in non_empty_cols] for r in non_empty_rows]
    cluster_ids_filtered = [cluster_ids[r] for r in non_empty_rows]
    transition_types_filtered = [transition_types[c] for c in non_empty_cols]

    if len(cluster_ids_filtered) >= 2 and len(transition_types_filtered) >= 2:
        if len(cluster_ids_filtered) == 2 and len(transition_types_filtered) == 2:
            p_windowed = fisher_exact_2x2(contingency_windowed_filtered)
            test_method_windowed = "Fisher exact (2x2)"
            warning_windowed = ""
        else:
            p_chi2_w, warning_windowed = fisher_exact_rxc(contingency_windowed_filtered)
            p_mc_w = monte_carlo_fisher_rxc(contingency_windowed_filtered, n_perms=100000)
            p_windowed = p_mc_w
            test_method_windowed = f"Monte Carlo permutation (100k perms); chi2 p={p_chi2_w:.4f}"
    else:
        p_windowed = 1.0
        test_method_windowed = "degenerate (< 2 rows or cols after filtering)"
        warning_windowed = "table too sparse for meaningful test"
else:
    p_windowed = 1.0
    test_method_windowed = "no observations in window"
    warning_windowed = ""

# 还做一个简化版本：把 cluster 简化为 2 组（有意义的分组）
# Cluster 0 (elevated volatility, trending) vs Cluster 1 (stable, mixed)
# Cluster 2,3 归入 Cluster 0（都是高波动、大位移）
simplified_cluster_map = {0: "high_vol", 1: "stable", 2: "high_vol", 3: "high_vol"}
simplified_transition_map = {
    "extending→expanding": "ext→exp",
    "expanding→newborn": "exp→new",
}

simplified_table = [[0, 0], [0, 0]]  # rows: high_vol, stable; cols: ext→exp, exp→new
simplified_row_labels = ["high_vol", "stable"]
simplified_col_labels = ["ext→exp", "exp→new"]

for a in alignments:
    sc = simplified_cluster_map[a["cluster"]]
    r = simplified_row_labels.index(sc)
    c = simplified_col_labels.index(
        simplified_transition_map.get(a["nearest_transition_type"], a["nearest_transition_type"])
    )
    simplified_table[r][c] += 1

p_simplified = fisher_exact_2x2(simplified_table)

criterion_2 = {
    "name": "聚类特征与中枢状态转换对应",
    "natural_window_days": natural_window,
    "contingency_table_full": {
        "rows": [f"Cluster {c}" for c in cluster_ids],
        "cols": transition_types,
        "data": contingency_full,
    },
    "contingency_table_windowed": {
        "rows": [f"Cluster {c}" for c in cluster_ids],
        "cols": transition_types,
        "data": contingency_windowed,
        "n_observations": n_in_window,
    },
    "test_full": {
        "method": test_method_full,
        "p_value": p_full,
        "significant": p_full < ALPHA,
        "warning": warning_full,
    },
    "test_windowed": {
        "method": test_method_windowed,
        "p_value": p_windowed,
        "significant": p_windowed < ALPHA if isinstance(p_windowed, float) else False,
        "warning": warning_windowed if isinstance(warning_windowed, str) else "",
    },
    "simplified_2x2": {
        "rows": simplified_row_labels,
        "cols": simplified_col_labels,
        "data": simplified_table,
        "p_value_fisher": p_simplified,
        "significant": p_simplified < ALPHA,
    },
    "passed": (p_full < ALPHA) or (p_windowed < ALPHA) or (p_simplified < ALPHA),
    "analysis": None,
}

analysis_2 = []
analysis_2.append(f"自然对齐窗口（|delta| 75th percentile）：{natural_window:.0f} 天")
analysis_2.append(f"全量列联表 p = {p_full:.4f}（{'显著' if p_full < ALPHA else '不显著'}，alpha={ALPHA}）")
analysis_2.append(f"窗口内列联表 p = {p_windowed:.4f}（{'显著' if p_windowed < ALPHA else '不显著'}）" if isinstance(p_windowed, float) else f"窗口内列联表：{test_method_windowed}")
analysis_2.append(f"简化 2x2 表 Fisher p = {p_simplified:.4f}（{'显著' if p_simplified < ALPHA else '不显著'}）")

if not criterion_2["passed"]:
    analysis_2.append(
        "所有检验均不显著。聚类类型与中枢状态转换类型之间不存在统计显著的对应关系。"
        "这是预期内的否定性结果——Agent A 的聚类本身就不稳定（轮廓系数 0.20），"
        "且 24 个断点对 12 个转换的匹配本身就是多对一映射，信息损失大。"
    )
else:
    analysis_2.append(
        "至少一个检验达到显著水平。需要进一步检查效应量和实际含义。"
    )

criterion_2["analysis"] = "\n".join(analysis_2)


# ─── 判据三：时间方向性 ───────────────────────────────────────────

# 按协议：对每个 t_a ∈ T_A，计算 δ = min_{t_b ∈ T_B}(t_b - t_a)
# 注意：这里取最近的 t_b，保留符号
# δ > 0 表示 Layer 1 领先（断裂先于转换）
# δ < 0 表示 Layer 2 领先（转换先于断裂）

directed_deltas = []
for t_a in T_A:
    best_delta = None
    for t_b in T_B:
        delta = days_between(t_a, t_b)  # t_b - t_a
        if best_delta is None or abs(delta) < abs(best_delta):
            best_delta = delta
    directed_deltas.append(best_delta)

sorted_directed = sorted(directed_deltas)
median_delta = percentile(sorted_directed, 50)
fraction_positive = sum(1 for d in directed_deltas if d > 0) / len(directed_deltas)

# 符号检验：H0: median = 0, H1: median > 0
p_sign = sign_test_p_value(directed_deltas, test_median_gt_zero=True)

# 条件：中位数 > 0 且 75% 以上落在正侧
direction_test_passed = (median_delta > 0) and (fraction_positive >= 0.75)

criterion_3 = {
    "name": "时间方向性",
    "directed_deltas": directed_deltas,
    "delta_distribution": {
        "n": len(directed_deltas),
        "min": min(directed_deltas),
        "max": max(directed_deltas),
        "mean": sum(directed_deltas) / len(directed_deltas),
        "median": median_delta,
        "p25": percentile(sorted_directed, 25),
        "p75": percentile(sorted_directed, 75),
        "fraction_positive": fraction_positive,
        "fraction_negative": sum(1 for d in directed_deltas if d < 0) / len(directed_deltas),
        "n_positive": sum(1 for d in directed_deltas if d > 0),
        "n_negative": sum(1 for d in directed_deltas if d < 0),
        "n_zero": sum(1 for d in directed_deltas if d == 0),
    },
    "sign_test_p_value": p_sign,
    "passed": direction_test_passed,
    "analysis": None,
}

analysis_3 = []
analysis_3.append(f"delta 中位数 = {median_delta} 天（{'> 0' if median_delta > 0 else '<= 0'}）")
analysis_3.append(f"正侧比例 = {fraction_positive:.2%}（{'≥ 75%' if fraction_positive >= 0.75 else '< 75%'}）")
analysis_3.append(f"符号检验 p = {p_sign:.4f}（{'显著' if p_sign < ALPHA else '不显著'}，单侧 H1: median > 0）")

if direction_test_passed:
    analysis_3.append(
        "方向性成立：Layer 1 断裂系统性领先 Layer 2 中枢状态转换。"
    )
else:
    if median_delta > 0 and fraction_positive < 0.75:
        analysis_3.append(
            f"中位数为正但正侧比例仅 {fraction_positive:.1%}，不满足 75% 阈值。"
            "方向性部分存在但不稳定。"
        )
    elif median_delta <= 0:
        analysis_3.append(
            "中位数非正——不存在 Layer 1 系统性领先 Layer 2 的方向性。"
            "可能是双向耦合或 Layer 2 领先。"
        )

criterion_3["analysis"] = "\n".join(analysis_3)


# ─── 综合结论 ─────────────────────────────────────────────────────

overall = {
    "criterion_1_passed": criterion_1["passed"],
    "criterion_2_passed": criterion_2["passed"],
    "criterion_3_passed": criterion_3["passed"],
    "n_passed": sum([criterion_1["passed"], criterion_2["passed"], criterion_3["passed"]]),
    "overall_verdict": None,
    "epistemological_level": "L2",
    "validity_domain": "FCX daily / Cu-Au ratio, 2000-08-30 to 2026-03-02",
}

n_pass = overall["n_passed"]
if n_pass == 0:
    overall["overall_verdict"] = (
        "三个判据全部未通过。在当前数据（单标的/单时段）下，"
        "Layer 1（Cu/Au比值统计断裂）与 Layer 2（FCX日线中枢状态转换）"
        "之间不存在可检测的内在对应。这是有信息量的否定性结果。"
    )
elif n_pass == 1:
    overall["overall_verdict"] = (
        f"仅 {n_pass}/3 个判据通过。证据不足以支持内在对应假设。"
    )
elif n_pass == 2:
    overall["overall_verdict"] = (
        f"{n_pass}/3 个判据通过。存在部分证据支持内在对应，但不完整。"
        "需要 L3 级别（多标的/多时段）交叉验证。"
    )
else:
    overall["overall_verdict"] = (
        "三个判据全部通过。在当前数据下支持内在对应假设。"
        "但这仅是 L2 级别——单标的/单时段，有效域严格受限。"
    )


# ─── 输出 JSON ────────────────────────────────────────────────────

results = {
    "experiment": "折叠内在性检验 - Agent A/B 交叉比对",
    "epistemological_level": "L2",
    "significance_level": ALPHA,
    "agent_a": {
        "n_breakpoints": len(T_A),
        "n_clusters": n_clusters,
        "silhouette": silhouette,
    },
    "agent_b": {
        "n_zhongshus": len(b_data["zhongshus"]),
        "n_transitions": len(T_B),
    },
    "alignments": alignments,
    "delta_statistics": delta_stats,
    "criterion_1": criterion_1,
    "criterion_2": criterion_2,
    "criterion_3": criterion_3,
    "overall": overall,
}

with open(OUT_JSON, "w", encoding="utf-8") as f:
    json.dump(results, f, ensure_ascii=False, indent=2, default=str)

print(f"[OK] Results written to {OUT_JSON}")


# ─── 输出 Markdown 摘要 ──────────────────────────────────────────

md_lines = []
md_lines.append("# 折叠内在性检验：交叉比对结果")
md_lines.append("")
md_lines.append(f"**认识论等级**：L2（单标的/单时段真实数据验证）")
md_lines.append(f"**有效域**：{overall['validity_domain']}")
md_lines.append(f"**显著性水平**：alpha = {ALPHA}")
md_lines.append("")

md_lines.append("## 数据概要")
md_lines.append("")
md_lines.append(f"- Agent A：{len(T_A)} 个结构性断点，{n_clusters} 个聚类（轮廓系数 {silhouette:.3f}）")
md_lines.append(f"- Agent B：{len(b_data['zhongshus'])} 个中枢，{len(T_B)} 次状态转换")
md_lines.append("")

md_lines.append("## 时间对齐")
md_lines.append("")
md_lines.append(f"| Agent A 断点 | 聚类 | 最近 Agent B 转换 | 转换类型 | delta (天) |")
md_lines.append(f"|-------------|------|------------------|---------|-----------|")
for a in alignments:
    md_lines.append(
        f"| {a['breakpoint_date']} | {a['cluster']} | {a['nearest_transition_date']} "
        f"| {a['nearest_transition_type']} | {a['delta_days']:+d} |"
    )
md_lines.append("")

md_lines.append(f"**delta 分布**：")
md_lines.append(f"- 范围：[{min(deltas)}, {max(deltas)}] 天")
md_lines.append(f"- 中位数：{delta_stats['median']:.0f} 天")
md_lines.append(f"- 25th / 75th percentile：{delta_stats['p25']:.0f} / {delta_stats['p75']:.0f} 天")
md_lines.append(f"- |delta| 中位数：{delta_stats['abs_median']:.0f} 天")
md_lines.append(f"- 自然窗口（|delta| 75th pct）：{natural_window:.0f} 天")
md_lines.append("")

md_lines.append("---")
md_lines.append("")

# 判据一
md_lines.append("## 判据一：聚类数量")
md_lines.append("")
md_lines.append(f"**结果：{'通过' if criterion_1['passed'] else '未通过'}**")
md_lines.append("")
md_lines.append(criterion_1["analysis"])
md_lines.append("")

# 判据二
md_lines.append("## 判据二：聚类特征与中枢状态转换对应")
md_lines.append("")
md_lines.append(f"**结果：{'通过' if criterion_2['passed'] else '未通过'}**")
md_lines.append("")

# 全量列联表
md_lines.append("### 全量列联表")
md_lines.append("")
header = "| |" + "|".join(f" {t} " for t in transition_types) + "| 合计 |"
md_lines.append(header)
md_lines.append("|" + "|".join(["---"] * (len(transition_types) + 2)) + "|")
for i, cid in enumerate(cluster_ids):
    row = contingency_full[i]
    row_total = sum(row)
    md_lines.append(f"| Cluster {cid} |" + "|".join(f" {v} " for v in row) + f"| {row_total} |")
col_totals = [sum(contingency_full[r][c] for r in range(len(cluster_ids))) for c in range(len(transition_types))]
md_lines.append(f"| 合计 |" + "|".join(f" {v} " for v in col_totals) + f"| {sum(col_totals)} |")
md_lines.append("")
md_lines.append(f"Monte Carlo permutation p = {p_full:.4f}")
md_lines.append("")

# 简化 2x2 表
md_lines.append("### 简化 2x2 列联表（high_vol vs stable × ext→exp vs exp→new）")
md_lines.append("")
md_lines.append("| | ext→exp | exp→new | 合计 |")
md_lines.append("|---|---|---|---|")
for i, label in enumerate(simplified_row_labels):
    row = simplified_table[i]
    md_lines.append(f"| {label} | {row[0]} | {row[1]} | {sum(row)} |")
md_lines.append(f"| 合计 | {simplified_table[0][0]+simplified_table[1][0]} | {simplified_table[0][1]+simplified_table[1][1]} | {sum(sum(r) for r in simplified_table)} |")
md_lines.append("")
md_lines.append(f"Fisher exact p = {p_simplified:.4f}")
md_lines.append("")

md_lines.append(criterion_2["analysis"])
md_lines.append("")

# 判据三
md_lines.append("## 判据三：时间方向性")
md_lines.append("")
md_lines.append(f"**结果：{'通过' if criterion_3['passed'] else '未通过'}**")
md_lines.append("")
md_lines.append(criterion_3["analysis"])
md_lines.append("")

# delta 直方图（文本版）
md_lines.append("### delta 分布直方图（文本版）")
md_lines.append("")
md_lines.append("```")
# 分桶
bin_width = 60  # 天
min_d = min(directed_deltas)
max_d = max(directed_deltas)
bin_start = (min_d // bin_width) * bin_width
bin_end = ((max_d // bin_width) + 1) * bin_width
bins = {}
for d in directed_deltas:
    b = (d // bin_width) * bin_width
    bins[b] = bins.get(b, 0) + 1

for b in range(bin_start, bin_end + bin_width, bin_width):
    count = bins.get(b, 0)
    bar = "#" * count
    label = f"[{b:+5d}, {b+bin_width:+5d})"
    md_lines.append(f"{label} | {bar} ({count})")
md_lines.append("```")
md_lines.append("")

# 综合
md_lines.append("---")
md_lines.append("")
md_lines.append("## 综合结论")
md_lines.append("")
md_lines.append(f"| 判据 | 结果 |")
md_lines.append(f"|------|------|")
md_lines.append(f"| 一：聚类数量 (N=3?) | {'通过' if criterion_1['passed'] else '未通过'} (N={n_clusters}) |")
md_lines.append(f"| 二：聚类-转换对应 | {'通过' if criterion_2['passed'] else '未通过'} (p={p_full:.4f}) |")
md_lines.append(f"| 三：时间方向性 | {'通过' if criterion_3['passed'] else '未通过'} (median={median_delta:.0f}d, {fraction_positive:.0%}+) |")
md_lines.append(f"| **总计** | **{n_pass}/3** |")
md_lines.append("")
md_lines.append(overall["overall_verdict"])
md_lines.append("")

md_lines.append("## 方法论注记")
md_lines.append("")
md_lines.append("- 认识论等级 L2：单标的（FCX）、单时段（2000-2026）的真实数据验证")
md_lines.append("- 否定性结果的信息量：否定缩小了有效域边界，比确认性结果更有价值")
md_lines.append("- 聚类不稳定性：轮廓系数 0.20 表明 Agent A 的聚类本身就是脆弱的，下游任何基于聚类的检验都受此限制")
md_lines.append("- 多对一映射偏差：24 个断点匹配 12 个转换，必然存在多个断点匹配同一转换的情况")
md_lines.append("- 该实验的有效域严格限于上述参数组合，不可外推至其他标的/时段/级别")
md_lines.append("")

with open(OUT_MD, "w", encoding="utf-8") as f:
    f.write("\n".join(md_lines))

print(f"[OK] Summary written to {OUT_MD}")
print(f"\n=== 快速结果 ===")
print(f"判据一（聚类数量 N=3?）：{'通过' if criterion_1['passed'] else '未通过'} — N={n_clusters}")
print(f"判据二（聚类-转换对应）：{'通过' if criterion_2['passed'] else '未通过'} — p={p_full:.4f}")
print(f"判据三（时间方向性）：{'通过' if criterion_3['passed'] else '未通过'} — median={median_delta:.0f}d, {fraction_positive:.1%} positive")
print(f"总计：{n_pass}/3 通过")
print(f"\n{overall['overall_verdict']}")
