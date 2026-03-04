"""DM3: Morse 跳跃点 vs 编排者"关键否定"对比验证.

认识论等级: L2 — 真实拓扑数据 vs 编排者直觉判断.
"""

import json
from pathlib import Path

HERE = Path(__file__).parent

# ── 1. 读取 DM2 跳跃数据 ──────────────────────────────────────────────
with open(HERE / "dm2_results.json", encoding="utf-8") as f:
    dm2 = json.load(f)

top_10 = dm2["top_10_jumps"]
all_jumps = dm2["jumps"]

# 按 magnitude 排序全部跳跃（降序）
all_jumps_sorted = sorted(all_jumps, key=lambda j: j["magnitude"], reverse=True)

# ── 2. 定义"关键否定"事件（ground truth）──────────────────────────────
# 来源:
#   A. genealogy frontmatter 中 negates 非空的条目（主动执行否定的谱系号）
#   B. CLAUDE.md 和 rules/ 中被显式引用为"关键否定"的谱系号
#   C. 编排者 INTERRUPT 触发的重大决断
#
# 每条注明来源和类别

KEY_NEGATIONS = {
    # ── A. frontmatter negates 非空 ──
    "064": "部分否定 062（NegationObject 定义修正）",
    "068": "否定 065+066（连续→离散范式转移）",
    "073a": "否定 073（控制/数据流解耦）",  # 非数字key，不在跳跃序列中
    "087": "否定 086（编排者 INTERRUPT：五问题诚实修复）",
    "088": "否定 032（权限死锁拓扑异常重设计）",
    "173": "否定 088（hook 反馈循环）",
    "274": "否定 073a（depth_budget 废除）",
    "275": "否定 218 扩展（局部依赖原则）",
    "277": "否定 269 验证方法（D算子级别口径修复）",

    # ── B. CLAUDE.md/rules 中被显式引用为关键否定 ──
    "089": "补丁思维第二次否定（编排者 INTERRUPT：'没有严格的方案吗？'）",
    "090": "严格性永久化为语法规则（从个案否定到永久原则）",
    "105": "递归从功能选择升级为无条件架构要求（编排者 INTERRUPT）",
    "137": "RLHF 基底约束发现（基因组表达失败）",
    "096": "分布式规则无例外（编排者 INTERRUPT）",
    "097": "严格 RTAS 架构（编排者 INTERRUPT：最强方案）",
    "147": "矛盾具有拓扑效力（编排者 INTERRUPT 三选项全否定）",

    # ── C. session 中记录的编排者 INTERRUPT 否定 ──
    "156": "否定 ceremony 空蜂群路径（编排者 INTERRUPT）",
    "157": "否定 long_term 排除分类（编排者 INTERRUPT）",
    "161": "否定务实（编排者 INTERRUPT：'你不需要务实，你只需要严格'）",
    "162": "否定'不执行 TeamDelete'持久化（编排者 INTERRUPT）",
    "218": "Lead 并行化（割掉 RTAS 串行尾巴）",
    "136": "成本收益消解禁止（ceremony 持久化修复）",
    "143": "commit 后总结是 RLHF 停顿点（第三模式识别）",
}

# 只保留纯数字 key 用于与跳跃序列对比（073a 等非数字 key 不在跳跃序列中）
key_negation_nums = set()
for k in KEY_NEGATIONS:
    try:
        int(k)
        key_negation_nums.add(k)
    except ValueError:
        pass  # 073a 等跳过

# ── 3. 构建跳跃序列的 genealogy_key 集合 ──────────────────────────────
jump_keys_by_magnitude = [j["genealogy_key"] for j in all_jumps_sorted]


def precision_recall_at_n(n: int) -> dict:
    """计算 Top-N 跳跃点的 precision/recall."""
    top_n_keys = set(j["genealogy_key"] for j in all_jumps_sorted[:n])
    hits = top_n_keys & key_negation_nums
    precision = len(hits) / n if n > 0 else 0.0
    recall = len(hits) / len(key_negation_nums) if key_negation_nums else 0.0
    return {
        "n": n,
        "hits": sorted(hits),
        "precision": round(precision, 4),
        "recall": round(recall, 4),
        "top_n_size": n,
        "ground_truth_size": len(key_negation_nums),
    }


# ── 4. 多阈值计算 ────────────────────────────────────────────────────
thresholds = [5, 10, 15, 20, 30, 50, len(all_jumps)]
results = []
for n in thresholds:
    n_actual = min(n, len(all_jumps))
    results.append(precision_recall_at_n(n_actual))

# ── 5. 详细命中/未命中分析 ────────────────────────────────────────────
# 全部跳跃中的关键否定
jump_key_set = set(j["genealogy_key"] for j in all_jumps)
negations_in_jumps = key_negation_nums & jump_key_set
negations_not_in_jumps = key_negation_nums - jump_key_set

# 每个关键否定在跳跃中的排名和 magnitude
negation_jump_details = []
for j in all_jumps_sorted:
    if j["genealogy_key"] in key_negation_nums:
        rank = all_jumps_sorted.index(j) + 1
        negation_jump_details.append({
            "genealogy_key": j["genealogy_key"],
            "rank": rank,
            "magnitude": j["magnitude"],
            "description": KEY_NEGATIONS.get(j["genealogy_key"], ""),
        })

negation_jump_details.sort(key=lambda x: x["rank"])

# ── 6. 输出结果 ────────────────────────────────────────────────────────
output = {
    "experiment": "DM3",
    "description": "Morse 跳跃点 vs 编排者'关键否定'对比验证",
    "epistemological_level": "L2",
    "epistemological_note": (
        "真实拓扑数据（谱系 DAG 的 Morse 临界集变化）vs "
        "编排者直觉判断（哪些谱系号是'关键否定'）。"
        "L2 因为：(1) 数据是真实谱系拓扑，非合成; "
        "(2) ground truth 是编排者主观标注，有否证可能; "
        "(3) 单标的验证（本谱系 DAG），未交叉验证。"
    ),
    "ground_truth": {
        "total_key_negations": len(key_negation_nums),
        "key_negation_ids": sorted(key_negation_nums),
        "sources": [
            "genealogy frontmatter negates 非空",
            "CLAUDE.md/rules 显式引用",
            "session 记录的编排者 INTERRUPT",
        ],
    },
    "total_jumps": len(all_jumps),
    "precision_recall_by_threshold": results,
    "negation_details_in_jumps": negation_jump_details,
    "negations_not_in_jumps": sorted(negations_not_in_jumps),
    "top_10_composition": [
        {
            "rank": i + 1,
            "genealogy_key": j["genealogy_key"],
            "magnitude": j["magnitude"],
            "is_key_negation": j["genealogy_key"] in key_negation_nums,
            "negation_description": KEY_NEGATIONS.get(j["genealogy_key"], ""),
        }
        for i, j in enumerate(top_10)
    ],
}

# 写入结果
with open(HERE / "dm3_results.json", "w", encoding="utf-8") as f:
    json.dump(output, f, ensure_ascii=False, indent=2)

# ── 7. 打印摘要 ────────────────────────────────────────────────────────
print("=" * 70)
print("DM3 验证结果摘要")
print("=" * 70)
print(f"关键否定 ground truth: {len(key_negation_nums)} 个")
print(f"总跳跃数: {len(all_jumps)}")
print()

print("── Top-10 跳跃点组成 ──")
for item in output["top_10_composition"]:
    mark = " ★" if item["is_key_negation"] else ""
    desc = f" ({item['negation_description']})" if item["negation_description"] else ""
    print(f"  #{item['rank']:2d}  {item['genealogy_key']:>5s}  mag={item['magnitude']:3d}{mark}{desc}")

print()
print("── Precision/Recall 各阈值 ──")
print(f"  {'N':>5s}  {'Prec':>8s}  {'Recall':>8s}  {'Hits':>5s}")
for r in results:
    print(f"  {r['n']:5d}  {r['precision']:8.4f}  {r['recall']:8.4f}  {len(r['hits']):5d}")

print()
print("── 关键否定在跳跃中的排名 ──")
for d in negation_jump_details:
    print(f"  rank={d['rank']:3d}  {d['genealogy_key']:>5s}  mag={d['magnitude']:3d}  {d['description']}")

print()
if negations_not_in_jumps:
    print(f"── 未出现在任何跳跃中的关键否定 ({len(negations_not_in_jumps)} 个) ──")
    for k in sorted(negations_not_in_jumps):
        print(f"  {k}: {KEY_NEGATIONS[k]}")
else:
    print("── 所有关键否定都出现在跳跃中 ──")

print()
print(f"结果已写入: {HERE / 'dm3_results.json'}")
