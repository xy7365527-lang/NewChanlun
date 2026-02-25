---
id: "209"
number: 209
type: 概念发现
status: 已结算
date: 2026-02-25
trigger: 208号下游——gauge 不变量保持率模式对分析揭示不变量层级结构
depends_on:
  - "195"
  - "208"
audit_targets:
  - "gauge 不变量层级"
---

# 209号：gauge 不变量层级结构——从保持率数据中涌现

## 结论

5 个强不变量候选在 3 种模式对（wide→new, wide→strict, strict→new）× 5 只股票上的保持率揭示了清晰的层级结构：

| 不变量 | wide→new | wide→strict | strict→new | 总体 | 层级 |
|--------|----------|-------------|------------|------|------|
| n_centers | 100% | 60% | 60% | 73% | Tier 1 |
| beta1_tau | 100% | 60% | 60% | 73% | Tier 1 |
| trend_kinds | 80% | 60% | 40% | 60% | Tier 2 |
| center_zd_zg_pairs | 60% | 0% | 0% | 20% | Tier 3 |
| barcode_bottleneck | 60% | 0% | 0% | 20% | Tier 3 |

### 层级解读

- Tier 1（n_centers, beta1_tau）：wide→new 100% 保持 = 真正的 gauge 不变量。wide/new 模式只改变笔的数量，不改变中枢拓扑。strict 模式的 min_strict_sep 过滤器改变笔数量和位置，导致 60% 保持。
- Tier 2（trend_kinds）：走势类型（趋势/盘整）对笔模式变化中等敏感。
- Tier 3（center_zd_zg_pairs, barcode_bottleneck）：涉及 strict 时 0% = 不是 gauge 不变量。中枢的精确 ZD/ZG 值和条形码的精确形状对笔模式高度敏感。

### 物理类比

Tier 1 = 拓扑不变量（同胚下保持）
Tier 2 = 同伦不变量（同伦等价下保持）
Tier 3 = 度量不变量（等距下保持）

gauge 等价性 ≈ 同胚级别——保持中枢数量和 β₁^τ，不保持精确度量。

## 定义依据

- gauge 等价性（195号）：不同笔模式下拓扑不变量的保持
- 208号：强不变量保持率 49.3% 的结构性解释

## 边界条件

- 5 只股票 × 7 年日线数据的统计量有限（每对 5 个样本）
- 更高频数据可能改变层级排序（更多线段 → 更稳定的中枢结构）
- strict 模式的 min_strict_sep 参数值影响 Tier 1 的 60%——参数越小，保持率越高

## 下游推论

1. gauge_equivalence_report 应区分 Tier 1/2/3 不变量 → [resolved: tier_map 已实现]
2. 强不变量通过率应按 Tier 加权报告（Tier 1 权重高于 Tier 3）→ [resolved: _tier_weighted_rate 已实现]
3. center_zd_zg_pairs 和 barcode_bottleneck 应从"强不变量"降级为"弱不变量" → [resolved: 209号-3 已执行]

## 谱系引用

- 195号：缠论代码拓扑化
- 208号：gauge 不变量保持率模式分析
- 验证报告：`.chanlun/review-results/real-market-data-verification-report.md`
