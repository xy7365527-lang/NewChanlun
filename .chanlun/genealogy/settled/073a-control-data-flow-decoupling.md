---
id: "073a"
title: "控制流与数据流解耦——递归蜂群的执行模式"
类型: 语法记录
状态: 已结算
日期: "2026-02-21"
前置: ["056", "069"]
negates: ["073"]
triggered_by: "Gemini 3.1 Pro 蜂群递归审计 v2（Agent Team 模式修正）"
---

# 073a: 控制流与数据流解耦——递归蜂群的执行模式

## 否定 073 的理由

073号基于错误前提："Claude Code subagent 不能 spawn sub-subagent（递归深度限制为 1-2 层）"。

**事实**：当前系统使用 Agent Team 模式（Task tool with team_name），teammates 可以使用 Task tool spawn 新的 teammates，递归深度无平台限制。069号的分形模板实例化可以直接实现。

## 修正后的语法记录

### 核心发现

真递归（Task tool spawn）可行，但 Trampoline/黑板模式仍有架构价值——不是因为平台约束，而是因为：

1. **可观测性**：深度递归中，父节点挂起等待子节点返回，Lead 无法观测中间状态。黑板模式（TaskList/SendMessage）让所有状态对 Lead 可见。
2. **可控性**：LLM 存在上下文衰减和注意力漂移。深层递归链条中，一个节点失败会导致整个调用链崩溃。黑板模式允许 Lead 在任意节点介入。
3. **控制流与数据流解耦**：Task tool spawn = 控制流衍生（真递归），TaskList/SendMessage = 数据流同步（黑板模式）。两者解耦是在无限衍生空间中建立秩序的架构原则。

### spawn 三基因

每个通过 Task tool 衍生的节点，必须在 prompt 中携带：
1. **拓扑坐标**：当前节点在分形树中的位置
2. **深度预算**：剩余允许的递归深度或 token 预算
3. **父回调标识**：父节点等待结果的标识符（team_name + SendMessage recipient）

### 推论

- 069号"分形模板实例化"在 Agent Team 模式下可直接实现（不退化）
- Lead 角色 = Trampoline 求值器，但这是架构选择，不是平台约束
- 结构工位是 Trampoline 的 loop invariant（每次弹跳中不变）

## 边界条件

- 如果 Agent Team 模式未来限制递归深度 → 073 的"平台约束"结论恢复有效
- 如果 LLM 上下文衰减问题被解决 → 黑板模式的可控性价值降低，但可观测性价值仍在

## 下游推论

1. dispatch-dag.yaml 的 fractal_template 不是"面向未来的声明"，而是当前可执行的架构——子蜂群可以直接 spawn 自己的结构工位
2. spawn 三基因（拓扑坐标、深度预算、父回调标识）应写入 dispatch-dag 的 task_template 作为强制字段
3. ceremony 的蜂群实例化流程应验证三基因完整性

## 影响声明

- 否定 073号谱系的"平台约束"前提
- 影响 dispatch-dag.yaml 的 fractal_template 语义（从声明性变为可执行）
- 影响 ceremony 流程（需增加三基因验证）

## 谱系链接

- negates: 073
- depends_on: 056, 069
- related: 072
