# F3 定义冲突诊断报告

**诊断模式**: diagnose（严格诊断）
**诊断者**: Codex 异质审查代理
**日期**: 2026-02-26

## 1. 矛盾的精确描述

**前部声明**（ceremony.md:5, 12, 69-70）：
> 结构能力 = skill（事件驱动），不是 teammate（075号）
> 不再 spawn 结构工位。genealogist/quality-guard/meta-observer/code-verifier 等结构能力由 event_skill_map 定义

**步骤6**（ceremony.md:93-94, 102-105）：
> - 拓扑分析家 spawn（179号，上下文隔离冷读 .chanlun/block-topology/）
> - meta-observer 二阶观察
> - python scripts/gangju_analysis.py → gangju audit_needed: true → spawn 审计工位

**team-lead 的问题**：步骤6要求 Lead spawn 拓扑分析家、meta-observer、gangju-analyst，这与前部"结构能力=skill，不是teammate"矛盾。

## 2. 代码层事实验证

### 2.1 ceremony_scan.py 实际输出

`get_required_skills()` 函数（ceremony_scan.py:30-44）从 dispatch-dag.yaml 的 `event_skill_map` 中筛选 `skill_type == "structural"` 的条目。

structural skill 列表（dispatch-dag.yaml event_skill_map）：
- genealogist（structural）
- quality-guard（structural）
- meta-observer（structural）
- code-verifier（structural）
- topology-mutator（structural）

**topology-analyst 是 `skill_type: conditional`**，不在 structural 列表中。
**gangju-analyst 不存在于 event_skill_map 中**——它是 Python 脚本（`scripts/gangju_analysis.py`），不是 agent/skill。

ceremony_scan.py 的 `workstations` 列表**不包含结构工位**——只包含业务工位（来自 roadmap、session 遗留、pending 谱系等）。

### 2.2 dispatch-dag.yaml 中的分类

| id | skill_type | 触发方式 | 是 skill 还是 teammate spawn？ |
|----|-----------|---------|-------------------------------|
| genealogist | structural | task_complete, /escalate, session_end | skill（D策略：hooks提示+Lead认领） |
| quality-guard | structural | file_write .chanlun/genealogy/**, src/** | skill（D策略） |
| meta-observer | structural | session_end, swarm_cycle_end | skill（D策略） |
| code-verifier | structural | file_write src/**/*.py, tests/**/*.py | skill（D策略） |
| topology-mutator | structural | genealogy_settlement + topo_effect | skill（D策略） |
| topology-analyst | **conditional** | swarm_cycle_end, manual_invocation | skill（按需触发） |
| gemini-challenger | conditional | /challenge, escalate_choice, genealogy_settlement | skill（按需触发） |
| source-auditor | conditional | file_write docs/** | skill（按需触发） |

### 2.3 三个被指控对象的代码层实际

#### topology-analyst

- **dispatch-dag 声明**：conditional skill，触发事件 = swarm_cycle_end（dispatch-dag.yaml:267-276）
- **agent 文件**（.claude/agents/topology-analyst.md）：tools 仅 `["Read", "Grep", "Glob"]`，只读，无写入能力
- **ceremony.md 步骤6 原文**：`拓扑分析家 spawn（179号，上下文隔离冷读 .chanlun/block-topology/）`
- **实际执行方式**：Lead 在 RTAS 循环中检测到 swarm_cycle_end 条件 → 使用 Task 工具调用 → topology-analyst 运行（只读）→ 产出报告 → 完成退出
- **生命周期**：ephemeral（运行完即结束），不是 persistent

#### meta-observer

- **dispatch-dag 声明**：structural skill，触发事件 = session_end, swarm_cycle_end（dispatch-dag.yaml:193-201）
- **agent 文件**（.claude/agents/meta-observer.md）：描述为"元规则观测工位（结构工位，常设）"
- **ceremony.md 步骤6 原文**：`meta-observer 二阶观察`（注意：**没有使用"spawn"一词**）
- **实际执行方式**：Lead 在 RTAS 循环中识别到 swarm_cycle_end 条件 → 认领 meta-observer skill → 执行二阶观察 → 产出写入谱系
- **082号 D策略**：hooks 提示 + Lead 认领 = 半事件驱动

#### gangju-analyst

- **不存在于 dispatch-dag.yaml 的 event_skill_map 中**
- **不存在 .claude/agents/gangju-analyst.md 文件**（Glob 搜索确认）
- **实际是 Python 脚本**：`scripts/gangju_analysis.py`
- **ceremony.md 步骤7 原文**：`python scripts/gangju_analysis.py` → 如果 `audit_needed: true` → spawn **审计工位**（业务工位，不是结构工位）
- **结论**：gangju-analyst 作为独立 agent/skill 不存在。gangju 是脚本，其产出可能触发业务工位 spawn

## 3. 形式判定

### 3.1 075号的严格定义

075号谱系的核心区分：

| 维度 | teammate（旧） | skill（新） |
|------|---------------|------------|
| 创建时机 | ceremony 步骤4（结构 spawn） | 事件触发时按需调用 |
| 生命周期 | persistent（蜂群存续期间持续运行） | ephemeral（运行完即结束） |
| 触发方式 | ceremony 硬编码 spawn | 事件驱动（dispatch-dag event_skill_map） |
| 在 ceremony 中的位置 | 步骤4（与业务工位一起 spawn） | 不在步骤4中（由 RTAS 循环按需触发） |

### 3.2 逐项判定

**topology-analyst**：
- 不在 ceremony 步骤4 spawn（✓ skill）
- 由 swarm_cycle_end 事件触发（✓ skill）
- ephemeral 生命周期（✓ skill）
- **但 ceremony.md 步骤6 使用了"spawn"一词** → 术语不精确

**meta-observer**：
- 不在 ceremony 步骤4 spawn（✓ skill）
- 由 swarm_cycle_end / session_end 事件触发（✓ skill）
- ceremony.md 步骤6 说"二阶观察"，未使用"spawn"（✓ 术语一致）
- **但 agent 文件描述为"结构工位，常设"** → 描述与 skill 模式有张力（"常设"暗示 persistent）

**gangju-analyst**：
- **不存在**。gangju 是 Python 脚本，不是 agent/skill
- 脚本产出触发的是业务审计工位，不是结构 spawn
- **无冲突**

### 3.3 冲突的精确范围

F3 冲突**不是**"步骤6要求 spawn 三个结构 teammate"。精确范围是：

1. **术语冲突**（ceremony.md:93）：`拓扑分析家 spawn` 中的"spawn"一词与前部"不是 teammate"声明产生表面矛盾。实际执行机制是 D策略（082号）的 skill 调用，但"spawn"一词暗示 teammate 创建。
2. **描述张力**（meta-observer.md:4）：agent 文件中"常设"一词暗示 persistent 生命周期，与 skill 的 ephemeral 特征有张力。
3. **gangju-analyst 不存在**：team-lead 的问题描述中包含了一个不存在的实体。

## 4. 严格的解决方案

### 判定：075号不需要修正

075号的核心区分（ceremony-time persistent teammate vs event-driven ephemeral skill）在代码层是成立的：

- ceremony_scan.py 的 workstations 不包含结构工位（代码事实）
- ceremony.md 步骤4 只 spawn 业务工位（文本事实）
- dispatch-dag event_skill_map 定义了事件触发映射（声明事实）
- 082号 D策略定义了实际执行机制（谱系事实）

### 需要修正的是 ceremony.md 步骤6 的术语

ceremony.md:93 的 `拓扑分析家 spawn` 应修正为不使用"spawn"的表述，因为：
- "spawn"在本仓库的语境中专指 ceremony 步骤4 的 teammate 创建
- 步骤6 的操作是 D策略 skill 调用，不是 teammate spawn
- 使用"spawn"造成与前部声明的表面矛盾

**建议修正**：
```
- 拓扑分析家 spawn（179号，上下文隔离冷读 .chanlun/block-topology/）
+ 拓扑分析家冷读（179号，上下文隔离读取 .chanlun/block-topology/——conditional skill 按需触发）
```

### meta-observer.md 的"常设"描述需要对齐

meta-observer.md:4 的"结构工位，常设"应修正为与 075号 skill 模式一致的描述：
```
- 元规则观测工位（结构工位，常设）
+ 元规则观测工位（structural skill，事件触发——session_end / swarm_cycle_end）
```

## 5. 选项判定

回到 team-lead 给出的三个选项：

| 选项 | 判定 | 理由 |
|------|------|------|
| A：结构能力可作为工位 spawn → 075号需要修正 | **否决** | 代码事实证明步骤6不是 ceremony-time teammate spawn，是 RTAS 循环中的 D策略 skill 调用。075号的区分在代码层成立 |
| B：结构能力只能 skill → ceremony.md 步骤6需要删除结构 spawn | **部分采纳** | 不是"删除"，是修正术语。步骤6的操作本身合法（D策略 skill 调用），只是"spawn"一词不精确 |
| C：第三种形式 | **这就是答案** | 实际形式是082号 D策略——"hooks 提示 + Lead 认领"的半事件驱动 skill 调用。这既不是 teammate spawn（A），也不需要删除操作（B），而是修正术语使声明与实际一致 |

**结论：选项C——第三种形式。** 075号定义成立，ceremony.md 步骤6 的操作实质合法，只需修正"spawn"术语为"skill 调用/触发"以消除表面矛盾。

## 6. 边界条件

本诊断在以下条件下会翻转：
1. 如果 Lead 在 RTAS 循环中实际使用 `Task(team_name="{蜂群名}")` 为 topology-analyst 创建持久化 teammate（而非 ephemeral subagent），则 075号的 skill 区分在执行层失效 → 需要重新审视 075号
2. 如果 Claude Code 平台升级支持真正的事件驱动 skill 调用（不经过 Task tool），则 D策略自动废除，术语问题消失
