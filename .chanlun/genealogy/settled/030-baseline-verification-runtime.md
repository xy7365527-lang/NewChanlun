---
id: "030"
title: "定义基底校验机制（运行时抽检）"
status: "已结算"
type: "语法记录"
date: "2026-02-19"
depends_on: ["013", "029"]
related: ["005"]
negated_by: []
negates: []
triggered: ["030a"]
---

# 030 — 定义基底校验机制（运行时抽检）

**类型**: 语法记录
**状态**: 已结算
**日期**: 2026-02-19
**negation_source**: homogeneous
**negation_form**: expansion（"同模型保证一致"这个假设被自身实践违反——六次 escalation 违规）
**前置**: 029-move-c-segment-coverage, 013-swarm-structural-stations
**关联**: 005-object-negation-principle

## 矛盾现象

蜂群设计隐含"同一模型 → 定义基底一致"的假设。实践证伪：Claude 实例之间六次违反 no-unnecessary-escalation 规则，同模型、同规则、不同行为。

注意：蜂群在 Agent Teams 架构下仍然是"一"（同一 Claude 模型的上下文分化）。本条不修改蜂群本体论，只修改一致性的保障方式。

## 推导链

1. 假设：同模型 + 同 SKILL.md → 理解一致
2. 反例：六次违规（同模型、同规则、不同行为）
3. 结论：一致性在同模型内大概率成立但不绝对，需要抽检机制

## 产出

1. SKILL.md 新增"定义基底校验"段落
2. 质询序列新增"第零步：基底校验（矛盾场景）"
3. 本条谱系
4. 030a（生成态）：Gemini 位置问题

## 影响范围

- **SKILL.md**：质询序列结构变更（新增第零步）
- **所有 Claude agent**：矛盾场景下需做基底校验
- **Lead**：新增定义校验职责
