---
name: sub-swarm-ceremony
description: >
  子蜂群 ceremony 标准流程。当 teammate 需要创建子蜂群时自动触发。
  确保子蜂群是完整的递归拓扑异步自指蜂群，不是简化版 agent pool。
triggers:
  - event: TeamCreate（由 teammate 而非 Lead 发起时）
---

# 子蜂群 Ceremony（095号谱系持久化，097号更新）

当一个 teammate 决定将子任务分解为子蜂群时，必须遵循此流程。
子蜂群 = 完整的递归拓扑异步自指蜂群。

**你（执行本 skill 的 teammate）是 L(n) 子蜂群的 Swarm₀。**
创世 Gap 在此瞬时表现——你制定子蜂群规则（spawn 任务时注入约束），但你自身不受子蜂群约束。
创世完成后，子蜂群 TaskList 的拓扑约束立即生效（094号：创世 Gap = 约束2 bootstrap 破缺，瞬时）。

## 何时触发

当你（作为 teammate）面对的任务满足以下条件之一：
- 子任务需要独立的状态管理（自己的 TaskList）
- 子任务可以进一步并行分解（≥2 个独立子单元）
- 子任务的复杂度超出单个 agent 的合理处理范围

**真递归是默认模式**。只有当子任务不可分解时，才在当前层直接执行（扁平退化特例）。

## 步骤

### 1. 创建子 Team

```
TeamCreate(team_name="{父team}-{子任务简称}", description="...")
```

命名规则：子 team 名 = 父 team 名 + 短横 + 子任务标识。
例：父 team = v24-swarm，子任务 = audit → 子 team = v24-swarm-audit

### 2. 创建 TaskList（四类节点，必须完整）

在子 team 中创建任务列表，按以下**最小 DAG 模板**分解子任务。

**子蜂群最小 DAG 结构（097号：四类节点，五约束 + 对象否定对象的合法实现）**：

| 节点类型 | 对应约束 | 实现方式 | 必须 |
|----------|----------|----------|------|
| **任务节点**（≥2，可并行） | 约束1a（物理持久化）+ 拓扑（DAG） | 独立的原子工作单元，产出写入文件 | ✅ |
| **审查节点**（t 时刻审查 t-1 产出） | 约束3（执行不可自观）→ 约束2（先在性）→ 异步自指 | Lead 执行：读取工位产出，验证，生成审查结论 | ✅ |
| **结晶节点**（产出凝固为外部文件） | 约束1a + 约束1b（符号可解释性） | 审查通过后，汇总写入 tmp/ 或 .chanlun/，通知父蜂群 | ✅ |
| **异质审计节点**（约束4） | 约束4（异质验证必要性） | 调用 Gemini challenger 或等效异质源，对关键产出执行异质质询 | ✅ |

**注意**：没有异质审计节点的子蜂群 = 封闭自证循环 = 违反约束4（093号）。
约束4是硬约束，失效后果是长期退化为自我确认循环。

依赖关系（DAG 边）：
- 任务节点 → 审查节点（任务完成后才能审查）
- 审查节点 → 异质审计节点（审查结论是异质审计的输入）
- 异质审计节点 → 结晶节点（异质审计通过后才能结晶）

### 3. 并行 spawn 子蜂群工位

为每个独立任务节点 spawn 一个 teammate：

```
Task(name="{工位名}", subagent_type="general-purpose", team_name="{子team名}",
     mode="bypassPermissions", run_in_background=true,
     prompt="你是 {子team名} 的工位 {工位名}。\n\n任务：{具体描述}\n\n...")
```

**数量递归规则（parallel_group 信号）**：

当 ceremony_scan 输出的工位携带 `parallel_group` 字段时：
- 同一 `parallel_group` 的所有工位**全部并行 spawn**，不评估"是否值得分解"
- 不存在"任务太多"的上限——RTAS 用数量递归解决复杂性
- Lead 读取 `parallel_group` 信号执行 spawn，不做价值判断
- `decomposition` 字段指示分解策略（`parallel` = 全部并行，`sequential` = 按序执行）
- 默认 `decomposition: parallel`

示例：ceremony_scan 输出 3 个工位均携带 `parallel_group: nested_divergence_endpoint`，
则 Lead 在同一个消息中发出 3 个并行 Task 调用。

### 4. 监控与汇总

你（作为子蜂群的 Lead）负责：
- TaskList 轮询工位状态
- 工位完成后 shutdown_request
- 执行审查节点（t 时刻读取工位产出，验证质量）
- 执行异质审计节点（调用 Gemini 对关键产出执行质询）
- 汇总所有工位产出
- 将汇总结果通过 SendMessage 回传给父蜂群的 Lead

**异质审计节点的最小实现**：
子蜂群 Lead 在汇总阶段，对关键产出执行异质质询。方式：
- 使用 `/challenge` skill 对产出进行 Gemini 异质质询
- 或直接 spawn 一个 gemini-challenger teammate 执行 decide/challenge 模式
- 如果产出是纯操作性（行动类，018号四分法），可免除异质验证

如果 Gemini 不可用：记录降级状态，向父蜂群 Lead 报告（审计层断裂 Gap 扩大，094号）。

### 5. Session 标注与清理

所有工位完成后：
1. **标注 session 文件**：在 `.chanlun/sessions/` 最新 session 文件中追加蜂群完成记录：
   - 蜂群名称（team_name）
   - 完成时间
   - 工位数量与状态摘要
   - 产出文件列表（谱系/定义/代码变更）
   - 异质审计结论（通过/降级/不适用）
2. TeamDelete 清理子 team
3. 向父蜂群 Lead 发送最终汇总（含异质审计结论）

## 子蜂群的完整结构（不可省略）

子蜂群必须具备：

| 特征 | 实现方式 | 约束来源 | 不可省略 |
|------|----------|----------|----------|
| **拓扑** | TaskList 作为 DAG 路由（四类节点 + 依赖边） | 069号：递归拓扑定义 | ✅ |
| **异步自指** | Lead 在审查节点执行 t 时刻审查 | 约束2 × 约束3 → 视差 Gap | ✅ |
| **结晶** | 产出写入文件（tmp/ 或 .chanlun/） | 约束1a + 约束1b | ✅ |
| **状态管理** | TaskList 独立于父蜂群 | 095号：真递归 | ✅ |
| **异质验证** | 异质审计节点调用 Gemini challenger | 约束4（093号） | ✅ |

**097号新增**：五特征（在095号四特征基础上，将约束4/异质验证明确为第五特征）。

## 创世 Gap 的递归化（097号）

当你执行本 skill 时，你是 L(n) 子蜂群的 Swarm₀。这意味着：

1. **你制定子蜂群的规则**（通过 TaskCreate 定义四类节点）
2. **你自身不受子蜂群规则约束**（创世 Gap：约束2 bootstrap 破缺）
3. **创世完成后**，子蜂群的规则立即生效，你变为子蜂群的监管者（父蜂群视角的 Lead）

这不是 bug，是每层递归的结构性条件（094号：创世 Gap 是瞬时的，不是永久奇点）。

## 递归深度

递归终止条件仅两个（274号）：
1. **原子性**：当前任务不可分解为 ≥2 个独立子任务 → 直接执行
2. **不动点**：rescan 未产生新工位 → ceremony 终止

无外部计数器（depth_budget 已废除）。无全局截断。递归深度由任务结构自然决定。
context window 耗尽 → 触发 compaction → 下一轮恢复继续。不是终止，是暂停。

与缠论同构：笔是原子性终止（不可再分解），不动点是区间套收敛终止。

每层递归都引入视差 Gap 累积（约束3 × 约束2 的历时效应，094号）。这是结构性代价，不是截断理由。


## (c)裁决：任务 DAG 向下递归——每个子任务自己 TaskCreate（不依赖 Lead 转发）

**编排者裁决(c)（2026-06-23）**：每个子任务（Agent）自己向下递归——TaskCreate 子任务，自己认领，向下推进。Lead 只做 scan→spawn→re-scan→不动点终止。

### 当前 harness 约束（swarm-mechanism-fix-20260623 实测结算）

- `TeamCreate` + `Task(team_name=…)` 已废弃，不可用
- flat roster：teammate 不能 spawn teammate（harness 硬禁）
- 有效工具：`Agent`（spawn）+ `TaskCreate/Get/List/Update`（todo 管理）

### 角色分工

| 角色 | 职责 | 工具 |
|------|------|------|
| **Lead（main session）** | scan 工位列表 → 并行 spawn 工位 → 等待完成 → re-scan → 不动点终止 | `Agent(name=…, run_in_background=true)` + TaskList |
| **业务 Agent（spawn 的工位）** | 执行自身任务 → 如发现子任务 → TaskCreate 子任务 → 执行（自己认领）→ 完成后汇报 Lead | `TaskCreate` + 直接执行 |

### Lead 的 scan→spawn→re-scan→不动点循环

```
LOOP:
  1. python scripts/ceremony_scan.py → 工位列表
  2. IF 工位列表为空 → 干净终止（不动点，020号反转）
  3. 并行 spawn 所有工位（Agent + name + run_in_background=true）
  4. 等待工位完成（TaskList 轮询 / SendMessage 回报）
  5. GOTO 1（re-scan）
TERMINATE WHEN: roadmap空 AND pending谱系空 AND 测试全通过 AND pattern-buffer无达标
```

### 与"子蜂群递归"的区别

| 维度 | 子蜂群递归（095号，TeamCreate 模型） | (c)任务 DAG 递归（当前 harness） |
|------|--------------------------------------|----------------------------------|
| 层级 | 新 Team + 新 TaskList（独立治理） | 同一 session 的 TaskList 嵌套 |
| 可用性 | TeamCreate 已废弃，不可用 | TaskCreate 有效（todo 管理工具） |
| 递归主体 | teammate spawn 子 team（flat roster 禁止） | 业务 Agent 自己 TaskCreate 子任务 |
| Lead 职责 | 管理层级（已不可行） | 只 scan/spawn/re-scan，不越级管理 |

### 约束

1. Lead 不自行执行任务——只分派（spawn）和汇总（re-scan）
2. 业务 Agent 发现子任务 → 自己 TaskCreate → 自己执行——不反抛给 Lead
3. 结构工位（6个）在 ceremony 开始时由 Lead 一次性 spawn，不参与 re-scan 循环（常设）
4. 按需工位由业务 Agent 或 Lead 按触发条件 spawn（event_skill_map D策略，082号）

## 谱系依据

- 097号：真严格递归拓扑异步自指蜂群完整架构——五特征最小 DAG 模板
- 095号：Agent Team 子蜂群真递归正式化
- 094号：创世 Gap 的递归化（L(n-1) 充当 L(n) Swarm₀）
- 093号：五约束有向依赖图（约束4异质验证为硬约束）
- 069号：递归拓扑异步自指蜂群定义
- 056号：递归是存在方式，线性是退化特例
- 016号：规则没有代码强制就不会被执行
