"""
层内折叠检验实验（317号谱系续）
认识论等级：L2（真实数据验证）

核心假设：ratio 断裂事件序列存在层内结构（后一次断裂的动力学特征被前一次约束）。
方法：状态转移矩阵 + chi-square 检验 + 环路检测。
"""

import json
import math
import sys
from pathlib import Path
from collections import Counter
from itertools import product

# ---------------------------------------------------------------------------
# 1. 加载数据
# ---------------------------------------------------------------------------
BASE = Path(__file__).resolve().parent

with open(BASE / "agent_a_results.json") as f:
    agent_a = json.load(f)

with open(BASE / "agent_b_results.json") as f:
    agent_b = json.load(f)

with open(BASE / "agent_a_timestamps.json") as f:
    a_timestamps = json.load(f)

with open(BASE / "agent_b_timestamps.json") as f:
    b_timestamps = json.load(f)

breakpoints = agent_a["breakpoints"]
# 按时间排序（已排序，但显式确认）
breakpoints.sort(key=lambda b: b["timestamp"])

# ---------------------------------------------------------------------------
# 2. Agent A 层内结构分析
# ---------------------------------------------------------------------------

# 2a. 提取 cluster 序列
cluster_seq = [bp["cluster"] for bp in breakpoints]
n_events = len(cluster_seq)
unique_clusters = sorted(set(cluster_seq))
n_states = len(unique_clusters)

print(f"=== Agent A 层内结构 ===")
print(f"断裂事件数: {n_events}")
print(f"Cluster 序列: {cluster_seq}")
print(f"唯一状态数: {n_states}, 状态集: {unique_clusters}")
print(f"各状态频率: {Counter(cluster_seq)}")

# 2b. 状态空间分析——cluster 2 和 3 各只有 1 个成员
# 原始聚类 silhouette=0.20（弱聚类），直接用 cluster 标签做转移矩阵
# 同时用二分类（方向：level_shift 正/负）做替代分析

# --- 方案1：用原始 4-cluster 标签 ---
def build_transition_matrix(seq, states):
    """构建状态转移计数矩阵"""
    state_to_idx = {s: i for i, s in enumerate(states)}
    n = len(states)
    matrix = [[0] * n for _ in range(n)]
    for i in range(len(seq) - 1):
        fr = state_to_idx[seq[i]]
        to = state_to_idx[seq[i + 1]]
        matrix[fr][to] += 1
    return matrix

def chi_square_uniformity(matrix, states):
    """
    Chi-square 检验：转移矩阵是否偏离均匀分布。
    H0: 从任何状态出发，转移到各状态的概率相等。
    对每行独立检验，然后汇总。
    """
    n = len(states)
    results = []
    total_chi2 = 0.0
    total_df = 0

    for i, state in enumerate(states):
        row_sum = sum(matrix[i])
        if row_sum == 0:
            results.append({
                "from_state": state,
                "row_sum": 0,
                "chi2": None,
                "df": None,
                "p_value": None,
                "note": "无出发转移"
            })
            continue

        expected = row_sum / n
        chi2 = sum((matrix[i][j] - expected) ** 2 / expected for j in range(n))
        df = n - 1

        # 手动计算 chi-square p 值（不依赖 scipy）
        # 使用 Wilson-Hilferty 近似
        p_value = chi2_survival(chi2, df)

        total_chi2 += chi2
        total_df += df

        results.append({
            "from_state": state,
            "row_sum": row_sum,
            "observed": matrix[i],
            "expected": round(expected, 3),
            "chi2": round(chi2, 4),
            "df": df,
            "p_value": round(p_value, 6) if p_value is not None else None
        })

    # 汇总检验
    total_p = chi2_survival(total_chi2, total_df) if total_df > 0 else None

    return {
        "per_row": results,
        "total_chi2": round(total_chi2, 4),
        "total_df": total_df,
        "total_p_value": round(total_p, 6) if total_p is not None else None
    }

def chi2_survival(x, k):
    """
    Chi-square 生存函数 P(X > x) 的近似计算。
    使用正态近似：对 k >= 30 精度较好，k < 30 用 Wilson-Hilferty 近似。
    """
    if k <= 0 or x < 0:
        return 1.0
    if x == 0:
        return 1.0

    # Wilson-Hilferty 近似
    z = ((x / k) ** (1/3) - (1 - 2 / (9 * k))) / math.sqrt(2 / (9 * k))

    # 标准正态 CDF (Abramowitz & Stegun 近似)
    p = normal_cdf(z)
    return max(0.0, min(1.0, 1.0 - p))

def normal_cdf(x):
    """标准正态 CDF 近似（Abramowitz & Stegun 7.1.26）"""
    if x < -8:
        return 0.0
    if x > 8:
        return 1.0

    # 使用 erf 近似
    return 0.5 * (1.0 + math.erf(x / math.sqrt(2)))

# 方案1：4-cluster 转移矩阵
tm_4 = build_transition_matrix(cluster_seq, unique_clusters)
chi2_4 = chi_square_uniformity(tm_4, unique_clusters)

print(f"\n--- 方案1：4-cluster 转移矩阵 ---")
print(f"转移矩阵:")
for i, s in enumerate(unique_clusters):
    print(f"  从 cluster {s}: {tm_4[i]}")
print(f"Chi-square 总统计量: {chi2_4['total_chi2']}, df={chi2_4['total_df']}, p={chi2_4['total_p_value']}")

# --- 方案2：二分类（shift 方向：up/down）---
direction_seq = ["up" if bp["features"]["level_shift"] > 0 else "down" for bp in breakpoints]
dir_states = ["down", "up"]
tm_dir = build_transition_matrix(direction_seq, dir_states)
chi2_dir = chi_square_uniformity(tm_dir, dir_states)

print(f"\n--- 方案2：方向二分类 (up/down) ---")
print(f"方向序列: {direction_seq}")
print(f"频率: {Counter(direction_seq)}")
print(f"转移矩阵:")
for i, s in enumerate(dir_states):
    print(f"  从 {s}: {tm_dir[i]}")
print(f"Chi-square 总统计量: {chi2_dir['total_chi2']}, df={chi2_dir['total_df']}, p={chi2_dir['total_p_value']}")

# --- 方案3：三分类（shift 强度：large_down / moderate / large_up）---
def classify_shift(level_shift):
    if level_shift < -0.15:
        return "large_down"
    elif level_shift > 0.15:
        return "large_up"
    else:
        return "moderate"

intensity_seq = [classify_shift(bp["features"]["level_shift"]) for bp in breakpoints]
int_states = ["large_down", "moderate", "large_up"]
tm_int = build_transition_matrix(intensity_seq, int_states)
chi2_int = chi_square_uniformity(tm_int, int_states)

print(f"\n--- 方案3：强度三分类 ---")
print(f"强度序列: {intensity_seq}")
print(f"频率: {Counter(intensity_seq)}")
print(f"转移矩阵:")
for i, s in enumerate(int_states):
    print(f"  从 {s}: {tm_int[i]}")
print(f"Chi-square 总统计量: {chi2_int['total_chi2']}, df={chi2_int['total_df']}, p={chi2_int['total_p_value']}")

# --- 方案4：波动率分类（calm / elevated / surge）---
def classify_vol(vol_ratio):
    if vol_ratio < 1.0:
        return "calm"
    elif vol_ratio < 2.0:
        return "elevated"
    else:
        return "surge"

vol_seq = [classify_vol(bp["features"]["volatility_ratio"]) for bp in breakpoints]
vol_states = ["calm", "elevated", "surge"]
tm_vol = build_transition_matrix(vol_seq, vol_states)
chi2_vol = chi_square_uniformity(tm_vol, vol_states)

print(f"\n--- 方案4：波动率三分类 ---")
print(f"波动率序列: {vol_seq}")
print(f"频率: {Counter(vol_seq)}")
print(f"转移矩阵:")
for i, s in enumerate(vol_states):
    print(f"  从 {s}: {tm_vol[i]}")
print(f"Chi-square 总统计量: {chi2_vol['total_chi2']}, df={chi2_vol['total_df']}, p={chi2_vol['total_p_value']}")


# 2c. 环路检测——在转移图中找强连通分量
def find_sccs(adj, n_nodes):
    """Tarjan's SCC 算法"""
    index_counter = [0]
    stack = []
    on_stack = [False] * n_nodes
    indices = [-1] * n_nodes
    lowlinks = [-1] * n_nodes
    sccs = []

    def strongconnect(v):
        indices[v] = index_counter[0]
        lowlinks[v] = index_counter[0]
        index_counter[0] += 1
        stack.append(v)
        on_stack[v] = True

        for w in adj[v]:
            if indices[w] == -1:
                strongconnect(w)
                lowlinks[v] = min(lowlinks[v], lowlinks[w])
            elif on_stack[w]:
                lowlinks[v] = min(lowlinks[v], indices[w])

        if lowlinks[v] == indices[v]:
            scc = []
            while True:
                w = stack.pop()
                on_stack[w] = False
                scc.append(w)
                if w == v:
                    break
            sccs.append(scc)

    for v in range(n_nodes):
        if indices[v] == -1:
            strongconnect(v)

    return sccs

def detect_cycles(matrix, states):
    """从转移矩阵构建有向图，找环路"""
    n = len(states)
    adj = [[] for _ in range(n)]
    for i in range(n):
        for j in range(n):
            if matrix[i][j] > 0:
                adj[i].append(j)

    sccs = find_sccs(adj, n)
    # 环路 = 大小 > 1 的 SCC，或自环（matrix[i][i] > 0）
    cycles = []
    for scc in sccs:
        if len(scc) > 1:
            cycles.append({
                "type": "multi_state_cycle",
                "states": [states[i] for i in sorted(scc)],
                "size": len(scc)
            })
        elif len(scc) == 1:
            v = scc[0]
            if matrix[v][v] > 0:
                cycles.append({
                    "type": "self_loop",
                    "state": states[v],
                    "count": matrix[v][v]
                })
    return cycles

# 各方案环路检测
cycles_4 = detect_cycles(tm_4, unique_clusters)
cycles_dir = detect_cycles(tm_dir, dir_states)
cycles_int = detect_cycles(tm_int, int_states)
cycles_vol = detect_cycles(tm_vol, vol_states)

print(f"\n=== 环路检测 ===")
print(f"4-cluster: {cycles_4}")
print(f"方向二分类: {cycles_dir}")
print(f"强度三分类: {cycles_int}")
print(f"波动率三分类: {cycles_vol}")


# 2d. 连续特征的自相关分析
# 不依赖离散化，直接看相邻断裂事件特征的 lag-1 自相关
def lag1_autocorrelation(series):
    """计算 lag-1 自相关系数"""
    n = len(series)
    if n < 3:
        return None

    mean = sum(series) / n
    var = sum((x - mean) ** 2 for x in series) / n
    if var < 1e-15:
        return None

    cov = sum((series[i] - mean) * (series[i + 1] - mean) for i in range(n - 1)) / (n - 1)
    return cov / var

feature_names = ["level_shift", "volatility_ratio", "post_trend_slope",
                 "departure_speed", "mean_reversion_corr", "oscillation_rate",
                 "stabilization_days", "new_regime_distance"]

print(f"\n=== Lag-1 自相关系数（连续特征）===")
autocorrs = {}
for feat in feature_names:
    series = [bp["features"][feat] for bp in breakpoints]
    ac = lag1_autocorrelation(series)
    autocorrs[feat] = ac
    # 对 n=24，95% 临界值 ≈ ±2/sqrt(24) ≈ ±0.408
    sig = ""
    if ac is not None:
        critical = 2 / math.sqrt(n_events)
        if abs(ac) > critical:
            sig = " *显著*"
    print(f"  {feat}: {round(ac, 4) if ac is not None else 'N/A'}{sig}")

print(f"  (95% 临界值 ≈ ±{round(2/math.sqrt(n_events), 4)}，n={n_events})")


# 2e. 游程检验（Runs test）——方向序列是否随机
def runs_test(binary_seq):
    """
    Wald-Wolfowitz 游程检验。
    binary_seq: 0/1 序列
    H0: 序列是随机的
    """
    n = len(binary_seq)
    n1 = sum(binary_seq)
    n0 = n - n1

    if n1 == 0 or n0 == 0:
        return {"runs": n, "expected": None, "z": None, "p_value": None, "note": "只有一种值"}

    # 计算游程数
    runs = 1
    for i in range(1, n):
        if binary_seq[i] != binary_seq[i - 1]:
            runs += 1

    # 期望和方差
    expected_runs = 1 + 2 * n0 * n1 / n
    var_runs = 2 * n0 * n1 * (2 * n0 * n1 - n) / (n * n * (n - 1))

    if var_runs <= 0:
        return {"runs": runs, "expected": round(expected_runs, 3), "z": None, "p_value": None}

    z = (runs - expected_runs) / math.sqrt(var_runs)
    p_value = 2 * (1 - normal_cdf(abs(z)))  # 双侧检验

    return {
        "runs": runs,
        "expected": round(expected_runs, 3),
        "variance": round(var_runs, 4),
        "z": round(z, 4),
        "p_value": round(p_value, 6)
    }

binary_dir = [1 if d == "up" else 0 for d in direction_seq]
runs_result = runs_test(binary_dir)
print(f"\n=== 游程检验（方向序列）===")
print(f"  up/down 序列: {direction_seq}")
print(f"  游程数: {runs_result['runs']}, 期望: {runs_result['expected']}")
print(f"  z={runs_result.get('z')}, p={runs_result.get('p_value')}")


# ---------------------------------------------------------------------------
# 3. Agent B 层内结构分析
# ---------------------------------------------------------------------------
print(f"\n\n=== Agent B 层内结构 ===")

transitions = b_timestamps
trans_types = [(t["from_status"], t["to_status"]) for t in transitions]
print(f"转换序列: {trans_types}")
print(f"频率: {Counter(trans_types)}")

# 关键观察：每个中枢固定走 extending→expanding→newborn 路径
# 转换类型序列是完全确定性的交替：
# (ext→exp), (exp→new), (ext→exp), (exp→new), ...
# 这不是随机过程，是确定性结构——转移矩阵是确定性的

b_states = [("extending", "expanding"), ("expanding", "newborn")]
b_seq_labels = [0 if t == b_states[0] else 1 for t in trans_types]
tm_b = build_transition_matrix(b_seq_labels, [0, 1])

print(f"转移矩阵 (0=ext→exp, 1=exp→new):")
for i in range(2):
    print(f"  从 {b_states[i]}: {tm_b[i]}")

# 检查是否完全确定性交替
is_deterministic = all(
    b_seq_labels[i] != b_seq_labels[i + 1]
    for i in range(len(b_seq_labels) - 1)
)
print(f"完全确定性交替: {is_deterministic}")

# 如果完全确定性，chi-square 不适用（不是随机过程）
# 但可以检查中枢间距的规律性
zhongshu_durations = []
for zs in agent_b["zhongshus"]:
    if len(zs["transitions"]) == 2:
        # extending→expanding 间距
        ext_exp_seg_span = zs["transitions"][0]["trigger_seg_idx"] - zs["start_seg_idx"]
        # expanding→newborn 间距
        exp_new_seg_span = zs["transitions"][1]["trigger_seg_idx"] - zs["transitions"][0]["trigger_seg_idx"]
        zhongshu_durations.append({
            "index": zs["index"],
            "ext_to_exp_segs": ext_exp_seg_span,
            "exp_to_new_segs": exp_new_seg_span,
            "total_segs": zs["end_seg_idx"] - zs["start_seg_idx"]
        })

print(f"\n中枢生命周期（段数）:")
for d in zhongshu_durations:
    print(f"  中枢{d['index']}: ext→exp={d['ext_to_exp_segs']}段, exp→new={d['exp_to_new_segs']}段, 总共={d['total_segs']}段")

# 检查持续时间的自相关
if len(zhongshu_durations) >= 3:
    total_segs = [d["total_segs"] for d in zhongshu_durations]
    ext_exp_segs = [d["ext_to_exp_segs"] for d in zhongshu_durations]
    exp_new_segs = [d["exp_to_new_segs"] for d in zhongshu_durations]

    ac_total = lag1_autocorrelation(total_segs)
    ac_ext = lag1_autocorrelation(ext_exp_segs)
    ac_exp = lag1_autocorrelation(exp_new_segs)

    crit_b = 2 / math.sqrt(len(zhongshu_durations))
    print(f"\nLag-1 自相关（中枢持续时间）:")
    print(f"  总段数: {round(ac_total, 4) if ac_total else 'N/A'}")
    print(f"  ext→exp 段数: {round(ac_ext, 4) if ac_ext else 'N/A'}")
    print(f"  exp→new 段数: {round(ac_exp, 4) if ac_exp else 'N/A'}")
    print(f"  (95% 临界值 ≈ ±{round(crit_b, 4)}，n={len(zhongshu_durations)})")


# ---------------------------------------------------------------------------
# 4. 汇总结果
# ---------------------------------------------------------------------------
results = {
    "experiment": "层内折叠检验",
    "epistemological_level": "L2",
    "data_source": "317号谱系实验一的真实数据",
    "agent_a_intra_layer": {
        "n_events": n_events,
        "analyses": {
            "scheme_1_4cluster": {
                "description": "原始4-cluster标签的转移矩阵",
                "states": unique_clusters,
                "state_counts": dict(Counter(cluster_seq)),
                "transition_matrix": tm_4,
                "chi_square": chi2_4,
                "cycles": cycles_4,
                "note": "cluster 2/3 各仅1个成员，转移矩阵稀疏度高——统计力不足"
            },
            "scheme_2_direction": {
                "description": "level_shift 方向二分类 (up/down)",
                "states": dir_states,
                "sequence": direction_seq,
                "state_counts": dict(Counter(direction_seq)),
                "transition_matrix": tm_dir,
                "chi_square": chi2_dir,
                "cycles": cycles_dir,
                "runs_test": runs_result
            },
            "scheme_3_intensity": {
                "description": "level_shift 强度三分类 (large_down/moderate/large_up)",
                "states": int_states,
                "sequence": intensity_seq,
                "state_counts": dict(Counter(intensity_seq)),
                "transition_matrix": tm_int,
                "chi_square": chi2_int,
                "cycles": cycles_int
            },
            "scheme_4_volatility": {
                "description": "volatility_ratio 三分类 (calm/elevated/surge)",
                "states": vol_states,
                "sequence": vol_seq,
                "state_counts": dict(Counter(vol_seq)),
                "transition_matrix": tm_vol,
                "chi_square": chi2_vol,
                "cycles": cycles_vol
            }
        },
        "continuous_autocorrelations": {
            feat: round(ac, 4) if ac is not None else None
            for feat, ac in autocorrs.items()
        },
        "autocorrelation_critical_value_95pct": round(2 / math.sqrt(n_events), 4)
    },
    "agent_b_intra_layer": {
        "n_transitions": len(transitions),
        "transition_types": [f"{t[0]}→{t[1]}" for t in trans_types],
        "is_deterministic_alternation": is_deterministic,
        "transition_matrix": tm_b,
        "zhongshu_lifecycles": zhongshu_durations,
        "lifecycle_autocorrelations": {
            "total_segs": round(ac_total, 4) if ac_total else None,
            "ext_to_exp_segs": round(ac_ext, 4) if ac_ext else None,
            "exp_to_new_segs": round(ac_exp, 4) if ac_exp else None,
            "critical_value_95pct": round(crit_b, 4) if len(zhongshu_durations) >= 3 else None
        },
        "note": "转换序列是完全确定性的交替（extending→expanding→newborn循环），不是随机过程。chi-square 检验不适用。"
    },
    "step3_cross_layer_isomorphism": {
        "applicable": False,
        "reason": "Agent B 层内结构是确定性的（非随机），Agent A 层内结构需先确认是否显著偏离均匀——两层性质根本不同，同构检验无意义"
    }
}

# 判决
def make_verdict(results):
    """基于统计检验结果生成判决"""
    a = results["agent_a_intra_layer"]
    b = results["agent_b_intra_layer"]

    verdicts = []

    # Agent A 判决
    # 方案2（方向二分类）是最有统计力的方案
    dir_chi2 = a["analyses"]["scheme_2_direction"]["chi_square"]
    dir_runs = a["analyses"]["scheme_2_direction"]["runs_test"]

    # 检查方向转移是否偏离均匀
    dir_p = dir_chi2["total_p_value"]
    runs_p = dir_runs.get("p_value")

    # 检查自相关
    crit = a["autocorrelation_critical_value_95pct"]
    sig_autocorrs = {
        feat: ac for feat, ac in a["continuous_autocorrelations"].items()
        if ac is not None and abs(ac) > crit
    }

    verdicts.append({
        "layer": "Agent A (ratio 断裂)",
        "direction_chi2_p": dir_p,
        "direction_runs_p": runs_p,
        "significant_autocorrelations": sig_autocorrs,
        "conclusion": None  # 下面填充
    })

    # 逻辑判断
    has_transition_structure = dir_p is not None and dir_p < 0.05
    has_sequential_structure = runs_p is not None and runs_p < 0.05
    has_feature_memory = len(sig_autocorrs) > 0

    if has_transition_structure or has_sequential_structure:
        verdicts[0]["conclusion"] = "发现显著层内结构"
        verdicts[0]["fold_detected"] = True
    elif has_feature_memory:
        verdicts[0]["conclusion"] = f"转移矩阵未偏离均匀，但{len(sig_autocorrs)}个连续特征存在显著自相关——弱层内结构"
        verdicts[0]["fold_detected"] = "weak"
    else:
        verdicts[0]["conclusion"] = "无显著层内结构——断裂事件之间统计独立"
        verdicts[0]["fold_detected"] = False

    # Agent B 判决
    verdicts.append({
        "layer": "Agent B (中枢转换)",
        "is_deterministic": b["is_deterministic_alternation"],
        "conclusion": "转换序列是完全确定性交替，不存在随机性——这不是'折叠'而是'定义'（中枢生命周期是 extending→expanding→newborn 的确定性路径）",
        "fold_detected": "trivial_deterministic"
    })

    # 总判决
    a_fold = verdicts[0]["fold_detected"]
    b_fold = verdicts[1]["fold_detected"]

    if a_fold is True:
        overall = "Agent A 存在显著层内折叠。Agent B 的'结构'是定义性的（确定性交替），不是经验性发现。"
    elif a_fold == "weak":
        overall = "Agent A 存在弱层内结构（特征自相关），但转移矩阵不显著。Agent B 的结构是定义性的。"
    else:
        overall = "两层都不存在非平凡层内折叠。Agent A 断裂事件统计独立，Agent B 转换是确定性的。ratio 序列不折回自身。"

    return {
        "per_layer_verdicts": verdicts,
        "overall": overall,
        "fold_hypothesis_result": "否定" if a_fold is False else ("弱确认" if a_fold == "weak" else "确认")
    }

verdict = make_verdict(results)
results["verdict"] = verdict

print(f"\n\n{'='*60}")
print(f"=== 判决 ===")
print(f"{'='*60}")
for v in verdict["per_layer_verdicts"]:
    print(f"\n{v['layer']}:")
    print(f"  {v['conclusion']}")
print(f"\n总判决: {verdict['overall']}")
print(f"折叠假设: {verdict['fold_hypothesis_result']}")

# 保存结果
with open(BASE / "intra_layer_results.json", "w", encoding="utf-8") as f:
    json.dump(results, f, ensure_ascii=False, indent=2)

print(f"\n结果已写入 intra_layer_results.json")
