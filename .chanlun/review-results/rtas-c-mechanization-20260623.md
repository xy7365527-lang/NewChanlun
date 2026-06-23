---
trigger: task-35-rtas-mechanization
mode: architecture-update
result: done
---
# 机制化(c)真RTAS任务-DAG递归

**工位**：rtas-mechanizer（topo_address: swarm/rtas-mechanizer）
**任务**：#35　**日期**：2026-06-23
**认识论等级**：L2（基于 swarm-mechanism-fix-20260623 实测结算 + skill/dag 文件实读；非合成假设）

---

## 诊断结论

### Q1：结构工位6个在哪个阶段应被 auto-spawn？sub-swarm-ceremony 是否描述了这个流程？

**应 spawn 阶段**：ceremony 序列 `cold_start` 的 `spawn-tasks` 节点——但 `dispatch-dag.yaml` 的 `spawn-tasks` 描述为"并行 spawn 业务工位"，结构工位（常设）应在更早阶段（`load-skills` 之前）一次性 spawn。

**team-topology.json** 声明 6个结构工位 `auto_spawn: true`，但没有任何 hook/代码自动执行此声明。

**sub-swarm-ceremony.md 的实际描述**：该 skill 描述的是"teammate 创建子蜂群"（TeamCreate + Task），是 095号 Agent Team 真递归模型。但 swarm-mechanism-fix-20260623 实测确认：`TeamCreate` 已废弃、flat roster 禁止 teammate→teammate spawn。因此 sub-swarm-ceremony **没有描述**结构工位 auto-spawn 流程，且其整体模型是 stale。

结构工位 auto-spawn 的当前有效路径：Lead 在 ceremony 时手动执行 `Agent(name=…, run_in_background=true)` spawn 6个结构工位（562号扬弃075，ceremony.md 步骤4已补充）。

**诊断**：有文字声明（team-topology.json auto_spawn），但 sub-swarm-ceremony.md 描述的是另一套（已废弃）机制。两者之间存在 spec-execution gap。

---

### Q2：按需工位13个的 trigger 条件是否被任何机制捕获？

**结论：未被机制捕获，处于"声明但未自动化"状态。**

- `team-topology.json` 有 trigger 字段（文本描述），但没有对应 hook 代码
- `dispatch-dag.yaml` `event_skill_map` 的触发条件绝大多数标注 `platform_support: false`（Claude Code 无原生语义事件）
- 触发依赖 D策略（082号）= hook 印文本提示 + Lead 手动认领——这是软路由，不是自动路由
- 唯一部分机制化的触发：PostToolUse advisory hook 检测文件写入（`platform_support: partial`），但是 advisory，不 block

**诊断**：13个按需工位的 trigger 是文档声明，没有对应的自动化机制。触发依赖 Lead/编排者手动判断和执行。

---

### Q3："每个子任务自己 TaskCreate 向下递归"在当前 skill 文件中是否有对应描述？

**结论：没有。**

当前 sub-swarm-ceremony.md 描述的是"teammate 创建子蜂群"（095号），不是"任务自递归 TaskCreate"。

编排者裁决(c)的机制：
- 子蜂群递归（095号）= 新 Team + 新 TaskList，TeamCreate 已废弃，flat roster 已禁止
- (c)任务 DAG 递归 = 同一 session 的 TaskList 中，业务 Agent 自己 TaskCreate 子任务、自己认领、自己递推

两者是不同层级：子蜂群递归是 harness 层的独立治理单元，(c)递归是 task list 层的 DAG 嵌套。前者已不可用，后者是在当前 harness 约束下可行的替代方案。

任何 skill 文件中均无此模式的显式描述——这是本任务需要补充的核心内容。

---

### Q4：Lead 的 scan→spawn→re-scan→不动点终止循环在哪里被描述？

**部分描述，不完整。**

- `dispatch-dag.yaml` `ceremony_sequence.cold_start/warm_start`：描述了 ceremony 的一次执行节点序列（scan → derive-work → spawn-tasks → recurse）和 `terminate_condition`
- `recurse` 节点只写"输出→接下来：[action] 紧跟 tool 调用"，不是循环协议
- **持续循环**（spawn → 等工位完成 → re-scan → 新工位 → spawn → … → 不动点）在 dispatch-dag 中没有显式描述

**诊断**：ceremony_sequence 描述了循环的单次迭代，但缺少"等待工位完成后 re-scan"的完整循环协议。Lead 的 scan→spawn→re-scan→不动点终止需要在 sub-swarm-ceremony.md 中补充显式化。

---

## 更新内容摘要

### 已修改的文件

**`.claude/skills/sub-swarm-ceremony/SKILL.md`**（Write 权限被拒绝，改为仅记录应改内容）：

应在文件末尾"谱系依据"之前新增 `## (c)裁决：任务 DAG 向下递归` 章节，内容见本文"(c)裁决机制化声明"节。

**`.chanlun/review-results/rtas-c-mechanization-20260623.md`**（本文件）：
- 诊断记录
- (c)裁决机制化声明

**注**：sub-swarm-ceremony/SKILL.md 的写入权限在本 session 被 harness 拒绝。内容草稿在 `/tmp/rtas-c-draft.md` 存档，需 Lead 或具备写权限的 session 执行实际写入。

---

## (c)裁决机制化声明

**编排者裁决(c)（2026-06-23）**：每个子任务（Agent）自己向下递归——TaskCreate 子任务，自己认领，向下推进。Lead 只做 scan→spawn→re-scan→不动点终止。

### harness 约束（swarm-mechanism-fix-20260623 实测）

| 项目 | 实测结论 |
|------|---------|
| spawn 工具 | `Agent` 工具（`name` 存在=teammate，省略=孤立 subagent） |
| task 工具 | `TaskCreate/Get/List/Update`（todo 管理，不是 spawn） |
| `TeamCreate` | 已废弃，不可用 |
| `Task(team_name=…)` | 已废弃，不可用 |
| teammate→teammate | 被 harness 禁止（flat roster，实测报错） |

### 角色分工

| 角色 | 职责 | 工具 |
|------|------|------|
| **Lead（main session）** | scan → 并行 spawn → 等待 → re-scan → 不动点终止 | `Agent(name=…, run_in_background=true)` |
| **业务 Agent（工位）** | 执行任务 → 发现子任务 → TaskCreate → 自己执行 → 汇报 Lead | `TaskCreate` + 直接执行 |

### Lead 循环协议（显式化）

```
LOOP:
  1. python scripts/ceremony_scan.py → 工位列表
  2. IF 工位列表为空 → 干净终止（不动点，020号反转）
  3. 并行 spawn 所有工位（Agent + name + run_in_background=true）
  4. 等待工位完成（TaskList 轮询 / SendMessage 回报）
  5. GOTO 1（re-scan）
TERMINATE WHEN: roadmap空 AND pending谱系空 AND 测试全通过 AND pattern-buffer无达标
```

### 业务 Agent 的子任务递归（显式化）

```
业务 Agent 执行模式：
  1. 执行自身任务
  2. IF 发现子任务（≥2 个独立子单元）：
       TaskCreate(title="子任务名", description="…")
       TaskUpdate(id=子任务id, status="in_progress")  // 自己认领
       执行子任务
       TaskUpdate(id=子任务id, status="completed")
  3. 完成后 SendMessage(to="lead") 或 TaskUpdate(自身, status="completed")
```

### 结构工位 auto-spawn（常设，不参与循环）

ceremony 开始时 Lead 一次性 spawn 6个结构工位：

```python
structural_agents = [
    "meta-lead", "genealogist", "quality-guard",
    "code-verifier", "meta-observer", "topology-manager"
]
# 并行 spawn（同一消息多个 Agent 调用）
for agent in structural_agents:
    Agent(name=agent, description="...", run_in_background=True, ...)
```

这些工位在 ceremony 循环中持续存在，不被 re-scan 重复 spawn，也不在不动点终止时销毁（常设）。

### 按需工位触发（D策略，082号）

按需工位通过 hook 文本提示 + Lead 手动认领触发（非自动）。触发源：
- `event_skill_map` 的 D策略 advisory hook 提示
- 业务 Agent 完成任务后的 SendMessage 汇报（携带触发信号）
- Lead 在 re-scan 产出工位的 `trigger` 字段判断

---

## 边界条件（结论翻转条件）

1. harness 恢复 teammate→teammate spawn（flat roster 解除）→ 095号子蜂群递归重新可用，(c)模型可升级为真调用栈递归
2. TaskCreate 语义变更（不再作为 todo 管理工具）→ 业务 Agent 子任务递归的工具需替换
3. ceremony_scan.py 输出格式变更 → Lead 循环协议的 re-scan 步骤需同步更新
4. 结构工位数量变更（team-topology.json 更新）→ auto-spawn 列表需同步

---

## 下游推论

1. sub-swarm-ceremony/SKILL.md 需要补充(c)裁决章节（当前 Write 权限不足，需 Lead 执行）
2. dispatch-dag.yaml 的 `ceremony_sequence` 可补充完整 LOOP 协议（当前只有单次迭代描述）
3. 业务 Agent 的 prompt 模板应包含"发现子任务时 TaskCreate"的指导
4. 按需工位的 trigger 机制化是下一步工作（当前 D策略软路由，建议 hook 强化）

---

## 谱系引用

- swarm-mechanism-fix-20260623：harness 实测结算（flat roster/Agent 工具/TaskCreate 约束）——本诊断的关键前提
- 095号（agent-team-recursive-swarm）：子蜂群 TeamCreate 模型——已废弃的递归形式
- 097号：五特征最小 DAG 模板——(c)递归需满足的结构特征
- 082号：D策略（hook 提示 + Lead 认领）——按需工位当前触发机制
- 016号：规则没有代码强制就不会被执行——诊断Q2/Q3的理论依据
- 218号：Lead 并行化——Lead scan→spawn 的并行原则
- 274号：废除 depth_budget——递归终止仅由原子性和不动点决定

---

## 影响声明

本产出改动：
- 新建 `.chanlun/review-results/rtas-c-mechanization-20260623.md`（本文件）
- 草稿存 `/tmp/rtas-c-draft.md`

应改动（因权限受限待执行）：
- `.claude/skills/sub-swarm-ceremony/SKILL.md`：末尾添加(c)裁决章节

不影响：
- Rust/Python 代码（任务约束：只做诊断+文档更新）
- 谱系文件（无新概念发现，只是机制化已结算裁决）
- dispatch-dag.yaml（建议未来补充，不在本任务范围内）
