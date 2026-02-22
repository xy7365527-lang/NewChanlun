---
id: "073b"
title: "Trampoline 模式——平台约束下的递归近似"
status: "已结算"
type: "语法记录"
date: "2026-02-21"
depends_on: ["056", "069", "072"]
related: ["033", "037", "058", "068"]
negated_by: ["073a", "095"]
negates: []
---

# 073 — Trampoline 模式——平台约束下的递归近似

**类型**: 语法记录
**状态**: 已结算
**日期**: 2026-02-21
**前置**: 056-swarm-recursion-default-mode, 069-recursive-topology-async-self-referential-swarm, 072-hook-enforcement-dual-prerequisite
**关联**: 033-declarative-dispatch-spec, 037-recursive-swarm-fractal, 058-ceremony-is-swarm-zero, 068-continuous-to-discrete-paradigm-shift

**negation_form**: expansion（056号"递归是默认模式"的机制层细化——区分理想递归与实际递归）
**negation_source**: heterogeneous（Gemini 3.1 Pro Thinking 审计产出）

---

## 背景

069号严格方案描述了理想的递归拓扑：蜂群→子蜂群→子子蜂群，每层复制父层的结构模板（分形模板实例化，037号）。但 Claude Code 平台存在硬约束：subagent 不能 spawn sub-subagent。所有 agent 都是 Lead 的 Layer 1 直接子节点。

Gemini 3.1 Pro Thinking 在审计 v12 蜂群架构时辨认出：当前蜂群的递归模式实际上是函数式编程中的 **Trampoline（尾递归蹦床）**模式。

## 结论

**当前蜂群的递归不是调用栈递归，而是 Trampoline——状态层伪递归。**

### 调用栈递归 vs Trampoline

| 特征 | 调用栈递归（069号理想） | Trampoline（当前实际） |
|------|----------------------|----------------------|
| 结构 | agent → sub-agent → sub-sub-agent | 所有 agent 都是 Lead 的 L1 子节点 |
| 递归深度 | 隐含在嵌套层级中 | 外化在任务队列（DAG）中 |
| 状态保持 | 每层有自己的栈帧（局部变量） | 状态通过任务描述 + 文件系统传递 |
| 递归展开 | agent 自行 spawn 子 agent | agent 完成后写回新任务，Lead 重新 spawn |
| 终止 | 最深层返回，逐层回溯 | 任务队列清空 / 区间套收敛 |

### Trampoline 的运作机制

```
循环 {
  Lead 从任务队列取出任务
  Lead spawn agent 执行任务
  agent 执行完毕，可能产出：
    a) 结果（终止）
    b) 新的子任务写回队列（裂变 = 返回 thunk）
  Lead 回到循环顶部
}
```

这就是经典的 Trampoline：不深入调用栈，而是反复在同一层"弹跳"——每次弹跳消费一个任务、可能产出新任务。递归深度不体现在 agent 嵌套层级中，而体现在**任务裂变的代数**（一个任务裂变为多个子任务，子任务再裂变）。

## 定义依据

- 056号："递归是存在方式，线性是退化特例"——成立，但递归的实现机制是 Trampoline 而非调用栈
- 069号层面2："dispatch-spec 从线性 phase 序列重构为 DAG"——DAG 正是 Trampoline 的外化状态表示
- 072号下游推论第6条："dispatch-spec → dispatch-dag"——DAG 不仅是理论偏好（068号），更是 Trampoline 模式的平台必然

## 推导链

1. 069号定义理想递归：蜂群→子蜂群→子子蜂群（调用栈模型）
2. 平台硬约束：subagent 不能 spawn sub-subagent（所有 agent 扁平于 L1）
3. 056号确认递归是默认模式——但未区分递归的实现机制
4. 实际运行时：Lead 反复 spawn L1 agent + agent 通过任务裂变写回队列 = Trampoline
5. Trampoline 保留了递归的本质属性（任务分解的递归性），牺牲了结构属性（嵌套 agent 层级）
6. 递归深度从隐式（调用栈）转为显式（任务 DAG）——这使得 dispatch-dag 不是可选优化，而是 Trampoline 模式的结构必然

## 边界条件

- 如果平台未来支持 sub-subagent spawn → Trampoline 可升级为真调用栈递归，073号退化为历史记录
- 如果任务裂变的代数无限增长（每个子任务都裂变出更多子任务）→ 任务队列爆炸，需要区间套收敛作为终止条件（069号层面2）
- 如果 Trampoline 的状态传递（通过任务描述 + 文件系统）丢失关键上下文 → 069号矛盾#7（genealogist 无法跨层穿透子任务局部变量）的工程实例

## 下游推论

1. **069号"分形模板实例化"的退化形式**：在 Trampoline 下，分形不体现为嵌套 agent 结构，而体现为任务描述中携带的结构模板——每个裂变出的子任务继承父任务的结构约束（结构工位要求、质量标准等）
2. **dispatch-dag 是 Trampoline 的必然产物**：调用栈递归的状态隐含在栈帧中，Trampoline 的状态必须外化——DAG 就是这个外化的状态。072号→dispatch-dag 的动机不仅是拓扑约束（Dominator Node），还有 Trampoline 的状态管理需求
3. **Lead 是 Trampoline 的求值器**：Lead 的角色从"团队领导"精确化为"Trampoline 求值器"——它不做递归展开的决策（那是任务裂变的事），它只做一件事：从队列取任务、spawn agent、收结果、再取任务
4. **结构工位是 Trampoline 的不变量**：每次弹跳（spawn-execute-fission 循环）中，结构工位始终存在且不参与裂变——它们是 Trampoline 循环的 loop invariant

## 谱系引用

- 056→073 演化链：056 确认递归是默认模式，073 辨认递归的实际机制是 Trampoline
- 069号层面2：dispatch-spec → DAG 重构，现在有了额外的动机（Trampoline 状态外化）
- 037号：递归蜂群分形——Trampoline 下分形从结构层退化为状态层
- 068号：偏序集/有向图范式——DAG 作为 Trampoline 状态表示是 068号的工程实例

## 影响声明

- 为 056号的"递归是默认模式"补充机制层描述：递归 = Trampoline，不是调用栈
- 为 069号的"分形模板实例化"标注平台退化形式：结构分形 → 状态分形
- 为 dispatch-dag 增加第二动机：不仅是拓扑约束（072号），还是 Trampoline 状态管理
- Lead 角色精确化：Trampoline 求值器
- 不修改任何现有代码文件
