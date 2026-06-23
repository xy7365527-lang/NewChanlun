---
id: "pending-auto-spawn-gap"
title: "team-topology.json auto_spawn:true 声明-实装 gap（结构工位实际靠 Lead 手动 spawn）"
type: "语法记录"
status: "生成态"
date: "2026-06-23"
depends_on: ["095", "096", "562", "551"]
negation_source: "rtas-mechanizer 诊断（Task #35，2026-06-23）"
negation_form: "spec-execution-gap"
---

# pending：team-topology auto_spawn 声明-实装 gap

## 矛盾

`.claude/team-topology.json` 中 6 个结构工位（meta-lead/genealogist/quality-guard/code-verifier/meta-observer/topology-manager）声明 `auto_spawn: true`。

但实际执行路径是：Lead 在 ceremony 时**手动** spawn 6 个结构工位（在 agent-team-bootstrap.sh 中写死）。`auto_spawn: true` 字段没有消费者——没有任何机制读取该字段并自动触发 spawn。

## 来源

rtas-mechanizer（Task #35）在审计 sub-swarm-ceremony SKILL 时发现，并记录于：
`.chanlun/review-results/rtas-c-mechanization-20260623.md`（诊断要点1）

## 边界条件

- 实际行为与声明不一致：声明"自动"，实现"手动"
- 不影响当前功能（Lead 手动 spawn 仍然工作）
- 但违反 090号严格性：声明膨胀——声明代码不具备的能力

## 四分法分类

**语法记录**：`auto_spawn: true` 字段已在 team-topology.json 中运作多轮（被写入但从未被读取），是"已在代码中存在但从未显式化其无效性"的隐性规则——需要让这个无效性显式化，要么删除字段（诚实），要么实现消费者（机制化）。

## 待裁决

选择 A：删除 `auto_spawn` 字段（诚实：声明不具备的能力，违 090号，直接删除）
选择 B：保留字段 + 在 agent-team-bootstrap.sh 中添加读取逻辑（机制化：让声明变成现实）

这是 **选择** 类型——需要编排者价值判断（或 Gemini decide() 代理）。
