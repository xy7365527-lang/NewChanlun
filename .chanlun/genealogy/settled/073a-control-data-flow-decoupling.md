---
id: "073a"
title: "控制流与数据流解耦——递归蜂群的执行模式"
status: "已结算"
type: "语法记录"
date: "2026-02-21"
depends_on: ["056", "069"]
related: ["072"]
negated_by: []
negates: ["073"]
类型: "语法记录"
状态: "已结算"
日期: "2026-02-21"
前置: ["056", "069"]
triggered_by: "Gemini 3.1 Pro 蜂群递归审计 v2（Agent Team 模式修正）"
---

# 073a: 控制流与数据流解耦——递归蜂群的执行模式

## 否定 073 的理由

073号基于错误前提："Claude Code subagent 不能 spawn sub-subagent（递归深度限制为 1-2 层）"。

**事实**：当前系统使用 Agent Team 模式（Task tool with team_name），teammates 可以使用 Task tool spawn 新的 teammates，递归深度无平台限制。

## 修正后的语法记录

### 核心发现

真递归可行，但 Trampoline/黑板模式仍有架构价值——不是因为平台约束，而是因为：

1. **可观测性**：黑板模式让所有状态对 Lead 可见
2. **可控性**：黑板模式允许 Lead 在任意节点介入
3. **控制流与数据流解耦**：Task tool spawn = 控制流，TaskList/SendMessage = 数据流

### spawn 三基因

1. **拓扑坐标**：当前节点在分形树中的位置
2. **深度预算**：剩余递归深度或 token 预算
3. **父回调标识**：父节点标识符（team_name + recipient）

## 边界条件

- 如果 Agent Team 模式未来限制递归深度 → 073 的"平台约束"结论恢复有效
- 如果 LLM 上下文衰减问题被解决 → 黑板模式的可控性价值降低

## 下游推论

1. dispatch-dag.yaml 的 fractal_template 是当前可执行的架构
2. spawn 三基因应写入 dispatch-dag 的 task_template 作为强制字段
3. ceremony 的蜂群实例化流程应验证三基因完整性

## 影响声明

- 否定 073号谱系的"平台约束"前提
- 影响 dispatch-dag.yaml 的 fractal_template 语义
- 影响 ceremony 流程（需增加三基因验证）
