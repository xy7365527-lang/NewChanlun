---
id: '432'
number: 432
title: "元观察——v231-swarm（ARTICULATE拓扑判据重构 + 索绪尔两轴对应 + API通道不稳定下的降级策略）"
type: meta-rule
status: 已结算
date: 2026-03-12
source: meta-observer（二阶观察，v231-swarm session 触发，Lead接手——076号模式）
depends_on:
  - '431'   # ARTICULATE 拓扑判据
  - '430'   # v230-swarm 元观察
  - '076'   # fractal execution gap
---

# 432号：元观察——v231-swarm

## 观察

### 观察1：编排者否定驱动的判据类型范畴变更（收敛——090号严格性）

编排者否定 ARTICULATE 的度量阈值（`score >= 2.0`），要求拓扑内在判据。这不是"优化阈值"，是判据类型的范畴变更：从度量（多少）到拓扑（有/无）。与 231号（有效域规则）同构——度量阈值的有效域依赖数据分布，拓扑判据的有效域等于定义域。090号严格性的又一实例：度量阈值是"大致能工作"的补丁，拓扑判据是严格的形式。

### 观察2：076号 fractal execution gap 持续（收敛——076号）

本 session 中 genealogist 和 meta-observer 两个工位均因 API 通道不稳定 + context 耗尽而无产出，Lead 接手完成。这是 076号的第 N 次确认。新变量：API 通道不稳定（503 错误）加剧了 context 耗尽的概率——agent 在等待 API 响应时消耗 context window。

### 观察3：API 通道容量与并发 agent 数的矛盾（首次出现）

6 个并行 agent spawn 时全部卡住（API 分发器 503），降为 2+2 批次后成功。这暴露了一个基础设施约束：并发 agent 数受限于 API 通道容量，不仅受限于 context window。ceremony 的并行默认原则（218号）在 API 通道不足时需要降级策略。

语法记录候选：**并发降级规则**——当 API 通道返回 503 时，自动将并发数减半重试，而不是全部失败。

### 观察4：scan 消费标记缺失导致推论重复检出（结构性缺口）

rescan 时 431号下游推论 1-3（status: executed）和 415/412/426号推论（已四分法消费）被重复检出为 "not_covered"。原因：ceremony_scan.py 的 downstream_audit 不读取推论的 status 字段，只检查 coverage_status。这是 scan 的结构性缺口，不是本轮特有问题。

## 自环检查

- 观察1：090号收敛
- 观察2：076号收敛（第N次）
- 观察3：新发现，语法记录候选（并发降级规则）
- 观察4：结构性缺口（scan 消费标记）

## 结论

无需 `/escalate`。一条语法记录候选（观察3：并发降级规则）。一条结构性缺口（观察4：scan 消费标记）。
