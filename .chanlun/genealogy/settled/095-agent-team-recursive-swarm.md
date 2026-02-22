---
id: '095'
title: Agent Team 子蜂群递归——从 Trampoline 升级为真调用栈递归
type: 选择
status: 已结算
date: 2026-02-22
depends_on:
  - '056'   # 蜂群递归是默认模式
  - '069'   # 递归拓扑异步自指蜂群
  - '073b'  # Trampoline 模式——平台约束下的递归近似
  - '093'   # 五约束有向依赖图
  - '094'   # 三个 Gap 重新定位
related:
  - '037'   # 递归蜂群分形
  - '058'   # ceremony 是 Swarm₀
negation_source: 编排者决定
negation_form: expansion（073b号 Trampoline 的边界条件触发——Team 模式提供真调用栈递归的平台基础）
negates: []
negated_by: []
---

# 095号：Agent Team 子蜂群递归——从 Trampoline 升级为真调用栈递归

**类型**: 选择
**状态**: 已结算
**日期**: 2026-02-22
**前置**: 056-swarm-recursion-default-mode, 069-recursive-topology-async-self-referential-swarm, 073b-trampoline-recursion-approximation, 093-five-constraints-topology, 094-gap-reposition
**关联**: 037-recursive-swarm-fractal, 058-ceremony-is-swarm-zero

## 结论

**蜂群严格使用 Agent Team（TeamCreate + Task with team_name）实现递归，不使用孤立 subagent（Task without team_name）。**

073b号记录了 Trampoline 作为"平台约束下的递归近似"。其边界条件明确写了："如果平台未来支持 sub-subagent spawn → Trampoline 可升级为真调用栈递归"。

**本号确认：Agent Team 模式满足该边界条件。**

- `general-purpose` 类型 agent 拥有 TeamCreate 和 Task 工具
- L1 teammate 可以调用 TeamCreate 创建子 team → Task(team_name=子team) spawn L2 teammate
- L2 teammate 同样拥有完整工具集，可以继续创建 L3
- 这构成真调用栈递归的技术基础

### 两种模式共存

| 模式 | 机制 | 适用场景 |
|------|------|----------|
| **Trampoline**（默认） | 任务裂变 → Lead 重新 spawn → 同一层弹跳 | 大多数场景：任务可在同一层分解完成 |
| **真递归**（可选） | teammate 创建子 team → 子 team 内部自治 → 结果向上回传 | 子任务需要独立治理：自己的 TaskList、自己的 ceremony、自己的生命周期管理 |

**选择标准**：如果子任务只是"需要并行执行的独立工作"→ Trampoline（同一层 spawn 多个 worker）。如果子任务需要"独立编排、独立状态管理、独立生命周期"→ 真递归（创建子 team）。

### 统一约束：严格使用 Agent Team

无论 Trampoline 还是真递归，所有蜂群节点都通过 Agent Team 管理：
- TeamCreate 创建 team
- Task(team_name=xxx) spawn teammate
- 不使用 Task(无 team_name) 作为孤立 subagent
- 唯一例外：Explore 类型 agent（只读搜索，不是蜂群节点）

### 子蜂群同样是递归拓扑异步自指蜂群（编排者决定）

子 team 不是简化版 agent pool。递归的每一层都是完整的递归拓扑异步自指蜂群：
- **拓扑**：子 team 有自己的 TaskList 作为 DAG 路由
- **异步自指**：子 team 内部的 agent 用 t 时刻规则审查 t-1 时刻的产出
- **结晶**：子 team 的产出在收缩相凝固为文件，向上回传给父蜂群
- **ceremony**：子 team 的创建者作为子 Swarm₀，加载规则后递归进入工作

这是 069号核心表格"分层递归（蜂群→子蜂群→子子蜂群）"的严格实现，不是近似。037号"分形模板实例化"在此从状态层（073b Trampoline 下的退化形式）恢复为结构层——每层递归实例化完整的蜂群结构模板。

## 定义依据

- 073b号边界条件："如果平台未来支持 sub-subagent spawn → Trampoline 可升级为真调用栈递归，073号退化为历史记录"
- Task 工具定义：`general-purpose` agent 拥有 "All tools" 包括 TeamCreate 和 Task
- 069号核心表格：递归拓扑异步自指蜂群的结构 = "分层递归（蜂群→子蜂群→子子蜂群）"——Team 嵌套精确匹配此描述

## 五约束框架下的递归限制

| 约束 | 递归影响 | 限制类型 |
|------|----------|----------|
| 1a 物理持久化 | 每层独立持久化，不累积 | 无限制 |
| 1b 符号可解释性 | 深层产出逐层传递可能损耗 | 软限制 |
| 2 规则先在性 | 每层需加载规则，时间成本线性增长 | 软限制 |
| 3 执行不可自观性 | 视差 Gap 随深度累积 | 结构性限制 |
| 4 异质验证必要性 | 审计层断裂 Gap 随深度放大 | 结构性限制 |

### 创世 Gap 的递归行为

每层递归都有自己的创世 Gap——创建子 team 的 agent 制定规则但自身不受规则约束。创世 Gap × 递归深度 = 累积的 bootstrap 破缺。但这不阻止递归，只增加治理成本。

### 实际深度上限

工程上限 ≈ 3-4 层，受制于：
- 并发 agent 数（depth × width ≤ 平台并发上限）
- API 成本（随 depth × width 增长）
- 视差 Gap 累积（审查延迟正比于深度）

## 边界条件

1. 如果 Claude Code 平台限制 teammate 使用 TeamCreate 工具 → 真递归不可行，回退到纯 Trampoline
2. 如果实际运行中发现 3 层以上递归的视差 Gap 累积导致产出质量显著下降 → 需要设置硬性深度上限
3. 如果平台未来提供原生子蜂群支持（内建的嵌套 Team 机制）→ 本号的手动嵌套方案退化为历史记录

## 下游推论

1. CLAUDE.md 原则15 需要更新：从"设计意图（未实现）"改为"Team 模式已支持真递归，受深度 ≈ 3-4 层限制"
2. 073b号需要标注 negated_by: 095（Trampoline 不再是"唯一实现"，而是"默认模式"）
3. ceremony 需要考虑支持子 team 的 ceremony（子 team 的 Swarm₀）
4. dispatch-dag 可能需要层级标注（L1/L2/L3）来追踪递归深度

## 推导链

073b号记录 Trampoline 为平台约束下的唯一实现 → 编排者质疑"用 Agent Team 不行吗？" → 审计确认 general-purpose agent 拥有 TeamCreate + Task → 五约束框架逐项分析递归影响 → 确认可行但受深度限制 → 编排者决定正式化 → 095号结算

## 谱系链

056号（递归是默认模式）→ 069号（递归拓扑异步自指蜂群定义）→ 073b号（Trampoline 近似）→ **095号（Agent Team 真递归 + Trampoline 共存）**

## 影响声明

- 073b号从"当前唯一实现"变为"默认模式（与真递归共存）"
- CLAUDE.md 原则15 的"设计意图（未实现）"需要修正
- 所有 Task 调用（非 Explore 类型）应统一使用 team_name 参数
- 蜂群架构从"扁平 Trampoline"升级为"Trampoline + 可选真递归"
