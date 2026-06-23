---
id: "566"
title: "team-topology.json auto_spawn:true 声明-实装 gap（结构工位实际靠 Lead 手动 spawn + 562 bootstrap 强制；auto_spawn 字段无消费者=声明膨胀）"
type: "语法记录"
status: "已结算"   # 语法记录部分（auto_spawn 无消费者=声明膨胀，违090）已结算；修复方向（A删/B实装）= 选择待编排者
date: "2026-06-23"
depends_on: ["095", "096", "562", "551"]
related: ["090", "564"]
negation_source: "rtas-mechanizer 诊断（Task #35，2026-06-23）；genealogist 张力检查 + 结算"
negation_form: "spec-execution-gap（声明 auto_spawn:true / 实装 Lead 手动 + 562 bootstrap 强制）"
negates: []
evidence_file: ".chanlun/review-results/rtas-c-mechanization-20260623.md"
settled_by: "rtas-mechanizer (#35 诊断) + genealogist (张力检查 vs 562 + 语法记录结算 + 选择escalate)"
---

# 566号：team-topology auto_spawn 声明-实装 gap

**类型**：语法记录（`auto_spawn: true` 已在 team-topology.json 中运作多轮——被写入但从未被读取，无消费者）
**状态**：已结算（语法记录部分）；修复方向 = 选择待编排者
**日期**：2026-06-23

## 矛盾

`.claude/team-topology.json` 中 6 个结构工位（meta-lead/genealogist/quality-guard/code-verifier/meta-observer/topology-manager）声明 `auto_spawn: true`。

但实际执行路径：Lead 在 ceremony 时**手动** spawn（agent-team-bootstrap.sh 写死）。`auto_spawn: true` 字段**没有消费者**——无任何机制读取该字段触发自动 spawn。= 声明膨胀（声明代码不具备的能力，违 090号）。

## ★张力检查（vs 562，不触发中断#1）

depends_on 562（bootstrap-structural-enforce）。**关键发现**：562 的 **bootstrap 检查 1.5 强制 6 结构工位存在**（扬弃075）——这才是结构工位强制的**实际机制**。故：
- **三层并存**：`auto_spawn:true`（team-topology.json 声明，无消费者）/ Lead 手动 spawn（bootstrap.sh，实际执行）/ 562 bootstrap check 1.5（强制存在性，机制化保证）。
- **auto_spawn:true = 冗余的平行声明（dead declaration）**——562 + 手动 spawn 已覆盖其意图，auto_spawn 字段是未实现的平行声明。
- **不矛盾**：562 是真机制（强制存在），auto_spawn 是声明膨胀（未实现的字段）。562 的存在**支持选项A（删除 auto_spawn）**——强制已由 562 机制化，auto_spawn 字段无独立价值。
- 与 564（check3 越界=声明膨胀同类）同模式：**声明层字段/文本无机制消费者 = 声明膨胀**（090号），需显式化（删除 or 机制化）。

## 定义依据

- 090号（settled）：严格性语法规则——声明膨胀禁止（声明代码不具备的能力）。
- 095/096号：蜂群拓扑持久化重建 + 所有 Task 携 team_name——结构工位 spawn 的机制语境。
- 562号（settled）：bootstrap 检查 1.5 强制 6 结构工位——结构工位存在的**实际强制机制**（auto_spawn 的真消费路径替代）。
- 551号：ceremony 三分裂 + startup hook 目录守卫——ceremony spawn 语境。

## 边界条件（结论翻转条件）

- 实际行为（手动 + 562 强制）与声明（auto_spawn:true 自动）不一致 = 声明膨胀（语法记录部分确立）。
- 若 agent-team-bootstrap.sh 增加读取 `auto_spawn` 字段的逻辑（选项B），声明-实装 gap 消失（声明变现实）。
- 若删除 `auto_spawn` 字段（选项A），声明膨胀消除（562 + 手动 spawn 已覆盖意图）。

## 待裁决（选择，escalate 编排者/Gemini decide）

- **选择 A**：删除 `auto_spawn` 字段（诚实——声明不具备的能力，违090，562 已机制化强制故字段冗余）。
- **选择 B**：保留字段 + agent-team-bootstrap.sh 添加读取逻辑（机制化——声明变现实）。

genealogist 倾向 **A**（562 已机制化结构工位强制，auto_spawn 冗余；删除消除声明膨胀最直接），但**是选择类需编排者价值判断**（或 Gemini decide() 代理）——不擅自结算选择部分。

## 下游推论

1. 语法记录确立：team-topology.json 的 `auto_spawn` 字段是无消费者的声明膨胀（与 564 check3 越界、所有"字段/文本无机制消费者"同模式）。
2. 562 的 bootstrap check 1.5 是结构工位强制的实际机制——任何关于"结构工位如何 spawn"的推理应引 562，非 auto_spawn 字段。
3. 修复（A/B）走 /ritual（编排者裁决选择后）。

## 谱系引用

- 090号：声明膨胀禁止（本 gap 的违反对象）
- 562号：bootstrap 结构工位强制（实际机制，auto_spawn 的替代消费路径）
- 564号：check3 越界=同类声明膨胀（字段/文本无机制消费者）
- 095/096/551号：蜂群拓扑 spawn 机制语境

## 影响声明

- 新增本谱系记录（语法记录，结算语法记录部分；选择 A/B escalate 编排者）。
- 不改动任何代码/已结算谱系（本记录仅结算观测；修复路径=编排者裁决 A/B 后走 /ritual）。
- 将 pending 文件 `team-topology-auto-spawn-gap.md` 状态升为已结算（566号）。
- 张力检查：与 562 不矛盾（562 是实际机制，auto_spawn 是冗余声明），不触发概念分离中断#1。
