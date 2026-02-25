---
id: '194'
title: 笔定义代码忠实度审计——10 Key 并行诊断
type: 概念发现
status: 结算态
date: 2026-02-25
source: v53-swarm（3工位并行审计 bi_engine.py / a_stroke.py / ab_bridge_newchan.py）
settlement: 吸收
depends_on:
  - '193'   # 第一次真实多轮质询（产出 10 Key）
  - '113'   # 包含关系工程选择
  - '114'   # 分型形式化
  - '001'   # 退化线段
---

# 194号：笔定义代码忠实度审计——10 Key 并行诊断

## 来源标注

v53-swarm 蜂群审计。编排者决断"全部 10 条轨迹并行推进"（188-3 resolved），
3 个工位并行诊断 193号质询产出的 10 个 Key。

## 审计结果

| Key | 组别 | 忠实度 | 严重性 |
|-----|------|--------|--------|
| counting_anchor_ambiguity | 锚点 | 忠实 | - |
| definition_anchor_collapse | 锚点 | 忠实 | - |
| lesson81_anchor_ambiguity | 锚点 | 有歧义 | LOW |
| new_bi_anchor_ambiguity | 锚点 | 忠实 | - |
| dual_baseline_counting | 计数 | 忠实 | - |
| kline_count_metric | 计数 | 忠实 | - |
| extreme_value_tie_breaker | 计数 | 有歧义 | LOW |
| extremum_matching_logic | 计数 | 忠实 | - |
| new_old_bi_compatibility | 兼容性 | 有歧义 | **HIGH** |
| missing_code_context | 兼容性 | 有歧义 | LOW |

**总结**：7 忠实 / 3 有歧义（2 LOW + 1 HIGH）/ 0 不忠实 / 0 定义冲突。

## HIGH 问题：ab_bridge 新笔静默退化

`ab_bridge_newchan.py:98-100` 的 `_run_a_pipeline` 调用 `strokes_from_fractals` 时
未传递 `merged_to_raw` 参数，导致 `use_new_bi = mode == "new" and merged_to_raw is not None`
为 False，新笔模式静默退化为旧笔宽模式。

**修复**：b3b5a9a — 当 `stroke_mode="new"` 时传递 `merged_to_raw`。

BiEngine 路径（`bi_engine.py:112-121`）一直正确传递该参数，只有 ab_bridge 路径遗漏。

## 歧义登记（非代码 bug）

### lesson81_anchor_ambiguity（LOW）

原文"不包括这两K线"中"K线"的指称有歧义：
- 解读A：指 raw K线 → 只排除极值那1根 raw K线
- 解读B：指 merged K线 → 排除整个 merged bar（可能对应多根 raw K线）

代码选择了解读B（偏严）。两种解读在绝大多数情况下等价，
仅在极值 merged bar 吞噬多根 raw K线时有差异。

### extreme_value_tie_breaker（LOW）

同类分型极值相等时，代码保留先出现者（时间优先）。
知识库 §4.4 步骤2 说"其他情况（例如相等）可先保留"，措辞模糊。
实际影响极小：两个相等的同类分型保留哪个，只影响分型位置不影响极值。

### missing_code_context（LOW）

BiEngine 采用全量重算策略（O(n²)），自包含所有上下文，
不依赖外部全局状态。唯一的上下文缺失在 ab_bridge 路径
（与 new_old_bi_compatibility 同一根源，已修复）。

## 下游推论

1. ~~ab_bridge 新笔退化修复~~ **已完成**（b3b5a9a）
2. lesson81_anchor_ambiguity 定义歧义——需要回溯缠师原文确认"K线"在该语境下的指称（依赖原文精读）
3. BiEngine 全量重算 O(n²) 性能优化——当前无正确性影响，实盘数据量大时可能成为瓶颈（非紧急）

## 谱系引用

- 193号：第一次真实多轮质询（本次审计的 10 Key 来源）
- 188号：多轮质询管道收敛算法（188-3 编排者全轨迹决断）
- 113号：包含关系工程选择（merged_to_raw 映射的来源）
- 114号：分型形式化（分型作为 Morse 临界点）
- 001号：退化线段（分解不唯一性 = gauge choice）
