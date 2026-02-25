---
id: "207"
number: 207
type: 概念发现
status: 已结算
date: 2026-02-25
trigger: Gemini Round 1 异质否定——W₁ 单调性假设与缠论递归放大矛盾
depends_on:
  - "204"
  - "205"
rule_version_baseline:
  claude_md_commit: "97f3ba3186f1919ff6abf02dea4247724d47c79a"
  rules_dir_mtime: "2026-02-23 21:10:15 +0000"
---

# 207号：W₁ 单调性假设与缠论递归放大的矛盾——T6 指标修正

## 结论

T6 原始设计假设"粗粒化后 W₁ 单调递减"（W₁(B_{k+1}) ≤ W₁(B_k)）。
Gemini Round 1 审核指出：缠论高级别中枢价格跨度更大，W₁ 可以增加。

反例：低级别 9 个跨度 5 的中枢 → W₁=22.5，高级别 1 个跨度 50 的中枢 → W₁=25 > 22.5。

这是概念层矛盾：W₁(Dgm, ∅) = Σ|d-b|/2 是总持续量（绝对值），不是归一化量。
缠论递归构造中高级别中枢代表更大价格区间，W₁ 增加是正常的。

## 修正

W₁ 单调性 → W₁ 比率有界：W₁(B_{k+1}) / W₁(B_k) ≤ λ

- λ 默认 3.0（允许高级别 W₁ 最多为低级别的 3 倍）
- 不要求递减，但增长比率应有界——无界增长暗示递归构造创造了虚假信息
- 同时修复小样本 KL inconclusive（min_bars < 5 时不参与判定）

## 边界条件

- 如果缠论递归构造中高级别中枢数量远少于低级别（常见），W₁ 比率可能 < 1（仍 bounded）
- 如果高级别中枢跨度极大（如跨越整个价格区间），W₁ 比率可能 > λ → T6 fail（正确行为）
- λ=3.0 是起始默认值，需从真实数据校准

## 下游推论

1. T6 指标从 w1_monotone 改为 w1_ratio_bounded → [resolved: 代码已修改]
2. 小样本 KL inconclusive 机制 → [resolved: min_bars_kl=5 参数已实现]
3. λ 参数需从真实数据校准（与 T1 ε、T6 δ/κ 同类问题）→ 长期工程项

## 谱系引用

- 204号：搁置模式否定——T6 可计算近似的来源
- 205号：clean_terminate 停顿模式——T6 审核的触发
- Gemini Round 1 审核报告：`.chanlun/review-results/plan-review-layer3-gemini-round1.md`
