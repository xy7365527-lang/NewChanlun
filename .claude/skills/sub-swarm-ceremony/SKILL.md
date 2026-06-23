---
name: sub-swarm-ceremony
description: >
  子蜂群 ceremony 标准流程。当 teammate 在任务执行中识别出可分解的子工作时触发。
  确保子蜂群是完整的递归拓扑异步自指蜂群，不是简化版 agent pool。
  (c)裁决（2026-06-23）：子蜂群 = 子任务 DAG（TaskCreate），不是子 teammate（TeamCreate）。
triggers:
  - event: teammate 在任务中识别出 ≥2 个独立子工作
---

# 子蜂群 Ceremony（095号谱系，097号 + (c)裁决更新）

当一个 teammate 在执行任务时识别出可分解的子工作，必须遵循此流程。
子蜂群 = 完整的递归拓扑异步自指蜂群。

## (c)裁决（2026-06-23，编排者）—— 递归在任务结构里，不在 team 结构里

**harness 硬约束（swarm-mechanism-fix-20260623 实测结算）**：
- `TeamCreate` + `Task(team_name=…)` 已废弃，不可用
- flat roster：teammate **不能** spawn teammate（harness 硬禁）
- 有效工具：`Agent`（spawn，仅 Lead 用）+ `TaskCreate/Get/List/Update`（所有工位的 todo 管理）

因此子蜂群**不通过 TeamCreate 创建子 team**，而是：
- **递归在任务结构里**：teammate 识别子工作 → **自己 TaskCreate 子任务**（unowned）→ 向下递归。
- **Lead 只做编排**：扫共享 TaskList → 为每个无主任务 spawn 一个工位 → re-scan → 不动点终止。
- 与 harness 兼容：Lead 是唯一 spawn 源；teammate 只 TaskCreate，不 spawn。

这扬弃（Aufhebung）了旧的 "teammate TeamCreate 子蜂群" 模型：
- **否定**：teammate 不再创建子 team（platform 不支持，095号"子 team 真递归"机制被 harness 否定）。
- **保留**：真递归是默认模式（095号精神）、四类节点最小 DAG（097号）、约束4 异质验证（093号）。
- **提升**：递归载体从 team 拓扑（roster）提升为任务拓扑（TaskList DAG）——
  全局 DAG 从工位的局部子任务创建中涌现（275号局部依赖），无人预规划。

**你（执行本 skill 的 teammate）是 L(n) 子任务 DAG 的作者（Swarm₀）。**
创世 Gap 在此瞬时表现——你制定子任务 DAG（TaskCreate 四类节点 + blockedBy 边），
但你自身不执行该 DAG（Lead spawn 工位执行）。作者 ≠ 执行者 = 视差 Gap 保留（约束3：执行不可自观）。

## 角色分工

| 角色 | 职责 | 工具 |
|------|------|------|
| **Lead（main session）** | scan→为每个无主任务 spawn 工位→等待完成→re-scan→不动点终止 | `Agent(name=…, run_in_background=true)` + `TaskList` |
| **业务 Agent（spawn 的工位）** | 执行自身任务 → 识别子工作 → TaskCreate 子任务 DAG（unowned）→ 完成后 SendMessage 父工位 | `TaskCreate` + 直接执行 |

## 何时触发

当你（作为 teammate）在任务执行中识别出满足以下条件之一的子工作：
- 子工作可以进一步并行分解（≥2 个独立子单元）
- 子工作需要独立的执行者（不同上下文，避免自观——约束3）
- 子工作的复杂度超出当前工位合理处理范围

**真递归是默认模式**。只有当任务原子性（不可分解为 ≥2 个独立子任务）时，才在当前层直接执行
（扁平退化特例，需在产出中记录理由："任务不可分解因为 [具体原因]"）。

## 步骤

### 1. 不创建子 team —— 在共享 TaskList 中布设子任务 DAG

无需 TeamCreate（隐式 team，所有工位共享同一 TaskList）。
直接 TaskCreate 子任务，用 `addBlockedBy` 表达依赖边，用 `metadata.agent_type`
标注每个子任务所需的 subagent_type（缺省 general-purpose）。

### 2. 布设四类节点（最小 DAG 模板，必须完整）

**子蜂群最小 DAG 结构（097号：四类节点，五约束 + 对象否定对象的合法实现）**：

| 节点类型 | 对应约束 | (c) 实现方式 | 必须 |
|----------|----------|----------|------|
| **任务节点**（≥2，可并行） | 约束1a（物理持久化）+ 拓扑（DAG） | `TaskCreate`（unowned，无 blockedBy），产出写入文件 | ✅ |
| **审查节点**（t 时刻审查 t-1 产出） | 约束3（执行不可自观）→ 约束2（先在性）→ 异步自指 | `TaskCreate`（blockedBy=全部任务节点），由**不同**工位执行读取产出验证 | ✅ |
| **异质审计节点**（约束4） | 约束4（异质验证必要性） | `TaskCreate`（blockedBy=审查节点，metadata.agent_type=gemini-challenger/codex-challenger） | ✅ |
| **结晶节点**（产出凝固为外部文件） | 约束1a + 约束1b（符号可解释性） | `TaskCreate`（blockedBy=异质审计节点），汇总写入 tmp/ 或 .chanlun/，SendMessage 父工位 | ✅ |

**注意**：没有异质审计节点的子任务 DAG = 封闭自证循环 = 违反约束4（093号）。
约束4是硬约束，失效后果是长期退化为自我确认循环。

依赖关系（blockedBy 边）：
- 审查节点 `blockedBy` 全部任务节点（任务完成后才能审查）
- 异质审计节点 `blockedBy` 审查节点（审查结论是异质审计的输入）
- 结晶节点 `blockedBy` 异质审计节点（异质审计通过后才能结晶）

**异步自指的 (c) 实现**：审查节点必须由与任务节点**不同的工位**执行
（Lead spawn 时为审查子任务分配新工位）——执行者与审查者分离 = 约束3（执行不可自观）的物理实现。
不允许任务节点工位自己审查自己的产出。

### 3. 不 spawn —— TaskCreate 后交给 Lead 的 (c) 循环

你（作为子任务 DAG 的作者）**不 spawn 任何工位**（flat roster：teammate 不能 spawn teammate）。
布设完子任务 DAG 后：
- 无依赖的任务节点（unowned，blockedBy 为空）→ Lead 的 (c) 循环立即扫到并 spawn 工位
- 有依赖的节点（审查/异质审计/结晶）→ blockedBy 清空后 Lead 自动 spawn

你自己可以认领（owner=自己）其中一个任务节点继续执行，或完成当前任务后退出（handoff 给 Lead 循环）。

**数量递归规则**：不存在"任务太多"的上限——RTAS 用数量递归解决复杂性，全部 TaskCreate，
不做价值判断。Lead 读取无主任务列表执行 spawn，不评估"是否值得分解"。

### 4. 监控与汇总（由结晶节点工位完成）

汇总职责落在**结晶节点工位**（不是 DAG 作者）：
- 结晶节点 blockedBy 异质审计通过后被 Lead spawn
- 读取所有任务节点产出 + 审查结论 + 异质审计结论
- 汇总写入文件（tmp/ 或 .chanlun/）
- 通过 SendMessage 将汇总结果（含异质审计结论）回传给 `parent_callback` 工位

**异质审计节点的最小实现**：
异质审计子任务的工位（metadata.agent_type=gemini-challenger 或 codex-challenger）执行异质质询：
- gemini-challenger：decide/challenge 模式对关键产出质询
- codex-challenger：review/diagnose 模式对代码产出质询
- 纯操作性产出（行动类，018号四分法）可在子任务描述中标注免除异质验证
- Gemini/Codex 不可用：审计工位记录降级状态，SendMessage 父工位（审计层断裂 Gap 扩大，094号）

### 5. Session 标注与清理

所有子任务完成后（结晶节点工位负责）：
1. **标注 session**：`bash scripts/session_append.sh "子任务DAG名: 产出摘要"` 追加完成记录
   （子任务数量与状态、产出文件列表、异质审计结论）
2. 无子 team 需要 TeamDelete（隐式 team 由 Lead 在 ceremony 终止时统一清理）
3. SendMessage 父工位最终汇总（含异质审计结论）

## 子蜂群的完整结构（不可省略，五特征）

| 特征 | (c) 实现方式 | 约束来源 | 不可省略 |
|------|----------|----------|------|
| **拓扑** | 共享 TaskList 作为 DAG（四类节点 + blockedBy 边） | 069号：递归拓扑定义 | ✅ |
| **异步自指** | 审查节点由不同工位在 t 时刻审查 t-1 产出 | 约束2 × 约束3 → 视差 Gap | ✅ |
| **结晶** | 结晶节点产出写入文件（tmp/ 或 .chanlun/） | 约束1a + 约束1b | ✅ |
| **状态管理** | 共享 TaskList（owner/status/blockedBy） | 095号：真递归（载体=任务拓扑） | ✅ |
| **异质验证** | 异质审计节点（agent_type=challenger） | 约束4（093号） | ✅ |

## 子蜂群递归（旧）vs (c) 任务 DAG 递归（当前 harness）

| 维度 | 子蜂群递归（095号，TeamCreate 模型，已废弃） | (c) 任务 DAG 递归（当前 harness） |
|------|--------------------------------------|----------------------------------|
| 层级 | 新 Team + 新 TaskList（独立治理） | 同一隐式 team 的 TaskList 嵌套 |
| 可用性 | TeamCreate 已废弃，不可用 | TaskCreate 有效（todo 管理工具） |
| 递归主体 | teammate spawn 子 team（flat roster 禁止） | 业务 Agent 自己 TaskCreate 子任务 |
| Lead 职责 | 管理层级（已不可行） | 只 scan/spawn/re-scan，不越级管理（275号） |

## 创世 Gap 的递归化（097号 + (c)）

当你执行本 skill 时，你是 L(n) 子任务 DAG 的作者（Swarm₀）：

1. **你制定子任务 DAG**（TaskCreate 四类节点 + blockedBy 边）
2. **你自身不执行该 DAG**（作者 ≠ 执行者；Lead spawn 工位执行——创世 Gap：约束2 bootstrap 破缺）
3. **布设完成后**，子任务 DAG 的拓扑约束立即生效（blockedBy 边强制执行序）

这不是 bug，是每层递归的结构性条件（094号：创世 Gap 是瞬时的，不是永久奇点）。

## 递归深度

递归终止条件仅两个（274号）：
1. **原子性**：当前任务不可分解为 ≥2 个独立子任务 → 直接执行
2. **不动点**：Lead 扫 TaskList 无无主/未阻塞/in_progress 任务 → ceremony 终止

无外部计数器（depth_budget 已废除）。无全局截断。递归深度由任务结构自然决定。
context window 耗尽 → 触发 compaction → 下一轮恢复继续。不是终止，是暂停。

与缠论同构：笔是原子性终止（不可再分解），不动点是区间套收敛终止。
每层递归引入视差 Gap 累积（约束3 × 约束2 历时效应，094号）——结构性代价，不是截断理由。

## 谱系依据

- **(c)裁决（2026-06-23，编排者）**：递归在任务结构里（TaskCreate 子任务），Lead 只编排——
  扬弃 "teammate TeamCreate 子蜂群" 模型（harness flat roster 否定子 team 真递归）
- 097号：真严格递归拓扑异步自指蜂群完整架构——五特征最小 DAG 模板
- 095号：Agent Team 子蜂群真递归正式化（机制由 (c) 从 team 拓扑改为任务拓扑）
- 094号：创世 Gap 的递归化（L(n-1) 充当 L(n) Swarm₀——(c) 下为子任务 DAG 作者）
- 093号：五约束有向依赖图（约束4异质验证为硬约束）
- 069号：递归拓扑异步自指蜂群定义
- 056号：递归是存在方式，线性是退化特例
- 016号：规则没有代码强制就不会被执行
- 275号：局部依赖——全局 DAG 从局部子任务创建中涌现，无人预规划
