---
trigger: task-35-rtas-mechanization
mode: architecture-update
result: done
---
# 机制化 (c) 真 RTAS 任务-DAG 递归（已实装）

**工位**：rtas-infra-fix（topo_address: swarm/rtas-infra-fix），整合 rtas-mechanizer 诊断 + bootstrap-fix 草稿
**任务**：#35　**日期**：2026-06-23
**认识论等级**：L2（基于 harness 实测结算 + 文件实读 + hook 模拟验证；非合成假设）

> 生成史（012号保留）：本文件先由 `rtas-mechanizer` 写为诊断版（其 Write 对 skill 文件被拒，
> 改动标"待执行"），`bootstrap-fix` 给 sub-swarm-ceremony.md 追加了 (c) 段（留下旧顶部 → 自相矛盾）。
> `rtas-infra-fix`（本工位，有写权限）实装全部改动并整合为本权威版。

---

## 一、编排者裁决 (c)（真 RTAS 的最终形态）

harness 硬约束（实测）：teammate 不能 spawn teammate，roster flat。
裁决 **(c)：每个子任务自己向下递归，Lead 只做编排**。

- **递归在任务结构里**：工位在任务执行中识别子工作 → **自己 TaskCreate 子任务**（unowned）= 向下递归。
- **Lead 只做编排**：扫任务列表 → 为每个无主任务 spawn 一个工位（Agent）→ 工位干活 + 创建子任务 →
  re-scan → 直到无无主任务（不动点终止）。Lead 不做实质工作、不预规划全局 DAG（275号局部依赖）。
- **harness 兼容**：Lead 是唯一 spawn 源；teammate 只 TaskCreate，不 spawn。

---

## 二、诊断：为何蜂群循环/结构工位/按需工位没正确启动

四个根因，映射到三个症状（循环/结构/按需）+ 一个底层 stale 模型。

### 根因 D1（结构工位 auto-spawn）：bootstrap 指令 stale，与 ceremony.md 冲突

- `agent-team-bootstrap.sh`（SessionStart）原注入 `Task(team_name="swarm", subagent_type=…)` —— 但
  harness 已演化为**单一隐式 team**（无 TeamCreate、`team_name` 已废弃），且 spawn 工具是 `Agent` 不是 `Task`。
- `ceremony.md` 步骤4 已更新到隐式-team `Agent(name=…)` 模型，**bootstrap.sh 没跟上** → 两套指令冲突。
- 真正强制结构工位存在的是 Stop-Guard 检查 1.5（`member.agentType` + `leadSessionId==session_id`）。
  当前 live team（session-14c95478）6 个结构工位齐全，说明最终能 spawn，但 bootstrap 指令是 stale 的混淆源。

### 根因 D2（按需工位 trigger）：声明但无机制桥

- `team-topology.json` 13 个 on_demand 有 `trigger` 文本，但无 hook 代码捕获。
- `dispatch-dag.yaml` `event_skill_map` 触发条件绝大多数 `platform_support: false`（Claude Code 无原生语义事件）
  → 依赖 D策略（082号）软路由（hook 提示 + Lead 手动认领），**从不自动 fire**。
- 缺少"任务需要某能力 → spawn 该 agent_type"的桥 → 按需工位实际只靠 Lead 手动判断启动。

### 根因 D3（蜂群循环）：循环载体从未明确为 live TaskList

- `ceremony_scan.py`（1745 行）只从 **intent-sources** 派生 workstations（roadmap/session 中断点/
  pending 谱系/测试失败/research lines/meta-rule/downstream/topo-indicators/fallback），
  **从不读 live TaskList**（`~/.claude/tasks/`）。
- 因此工位 TaskCreate 的子任务（(c) 递归产物）对 scan-驱动的循环**不可见**：rescan 不会把它们放进
  workstations，Lead 不会 spawn 工位消费它们。
- 唯一"看见"live 任务的是 Stop-Guard 检查 2，但它只 **block 停机** + 发**泛化**路由消息
  （"待分配任务…启动工位或分配给空闲工位"），不是"为每个无主任务 spawn 一个工位"的 (c) 指令。
- 结果：两套 task 视图脱节——scan 派生的 workstations vs live TaskList。(c) 要求 live TaskList 成为循环载体，此前缺失。

### 根因 D4（stale TeamCreate 递归模型）：跨 skill/dag/ceremony 一致 stale

- `sub-swarm-ceremony.md` 步骤1 `TeamCreate(...)`、`dispatch-dag.yaml` recursion_rules
  "teammate 必须 spawn 子蜂群（TeamCreate）"、ceremony.md 步骤5 递归判断"读 sub-swarm-ceremony 执行子蜂群创建"——
  全部路由 worker 递归到 TeamCreate，**flat-roster harness 禁止**（已上浮的 spec-execution gap）。
- worker 命中此 gap → 要么无法递归，要么非法尝试 TeamCreate。(c) 正是要扬弃此模型。

### 附带缺陷 D5（Stop-Guard 跨 session 任务污染）

- `ceremony-completion-guard.sh` 检查 2 全局扫描 `~/.claude/tasks/*/`（所有 session），不限本 Lead 的 team。
- 任意其他 session 的陈旧 in_progress 任务会**误阻**本 Lead 停机
  （见 feedback_task_queue_owner_liveness / reference_stopguard_zombie_tasks 已记录事故）。
- 当前未触发（仅 live team 有活跃任务），但是 latent 正确性缺陷，(c) 循环不动点终止依赖修复它。

---

## 三、(c) 机制化方案

### 3.1 循环载体（机制核心）= live TaskList + Stop-Guard 强制

明确分工，消除 D3：

| 阶段 | 载体 | 职责 |
|------|------|------|
| **Seed（种子）** | `ceremony_scan.py` | 从 intent-sources 派生初始 workstations；Lead spawn，每个工位创建+认领自己的 Task |
| **活循环（consume）** | Lead 的 `TaskList` tool（自动 scope 到隐式 team）| 扫无主任务 → spawn 工位 per 无主任务 → 工位 TaskCreate 子任务 → re-scan |
| **强制器** | `ceremony-completion-guard.sh`（Stop hook）| 扫本 Lead team 任务目录；有无主/未阻塞 pending → block + 注入 (c)spawn 指令 → 自动再触发循环 |
| **终止** | 不动点 | TaskList 无 无主/未阻塞/in_progress 任务 AND scan intent-sources 空 → 允许停机 |

为什么 Stop-Guard 是活循环的强制器：它**已有 session_id**（hook stdin）→ 能确定性定位本 Lead 的 team 任务目录，
且每次 Stop 自动 fire → 是天然的 re-scan 触发点。

### 3.2 结构工位 auto-spawn（(c) 的常设部分）—— 消除 D1

- Lead 在 ceremony 开始一次性并行 spawn 6 个结构工位（`Agent(name=…, subagent_type=…, run_in_background=true)`，隐式 team）。
- Stop-Guard 检查 1.5 机制强制存在（562号）。结构工位常设，不参与 re-scan 循环、不在不动点销毁。
- bootstrap.sh 指令对齐到此模型（删 stale `Task(team_name=…)`）。

### 3.3 按需工位 task-triggered（(c) 的按需部分）—— 消除 D2

- 桥：工位识别到需要某能力 → `TaskCreate` 子任务，在 `metadata.agent_type` 标注所需 subagent_type。
- Lead 的 (c) 循环 spawn 时读 `metadata.agent_type`（缺省 general-purpose）→ 按需工位自然纳入循环。
- 这把按需触发从"软路由（D策略）"升格为"任务驱动（确定性）"。

### 3.4 递归模型扬弃 —— 消除 D4

- worker 递归 = `TaskCreate` 子任务（unowned），不 `TeamCreate` 子 team。
- 子蜂群最小 DAG（四类节点：任务/审查/异质审计/结晶，097号五特征）用 TaskCreate + `blockedBy` 边 + `metadata.agent_type` 表达。
- 异步自指（约束3 执行不可自观）保留：审查节点由**不同**工位执行（Lead spawn）。
- 这是 095号"真递归"的 Aufhebung：否定 TeamCreate 机制，保留递归精神 + 四类节点 + 约束4，提升载体从 team 拓扑到任务拓扑。

---

## 四、改的文件清单（已实装）

| 文件 | 改动 | 类型 |
|------|------|------|
| `.claude/hooks/agent-team-bootstrap.sh` | 删 stale `Task(team_name="swarm")`，改隐式-team `Agent(name=…)`；新增 (c) 循环 + 按需 task-trigger 描述 | hook（基因组表达） |
| `.claude/hooks/ceremony-completion-guard.sh` | (1) 任务扫描 scope 到本 Lead team（修 D5 跨 session 污染）；(2) 检查2 追踪无主任务 + (c)spawn-per-无主任务 路由消息 | hook（Stop-Guard，循环强制器） |
| `.claude/commands/ceremony.md` | 步骤6 显式化 (c) TaskList 驱动循环（spawn-per-无主）；步骤5 递归判断改 TaskCreate（非 sub-swarm TeamCreate）；步骤9/10 不动点加 TaskList 条件；不变量加 (c) 条；删 stale spec-gap 注 | command |
| `.claude/skills/sub-swarm-ceremony/SKILL.md` | **整文件严格重写**为 (c) 子任务 DAG 模型（删旧 TeamCreate 顶部，并入 bootstrap-fix 的 harness约束/角色分工/区别表） | skill（knowledge_template） |
| `.chanlun/dispatch-dag.yaml` | recursion_rules + fractal_template 改 (c)（TaskCreate 子任务，非 TeamCreate）；genealogy_ref 加 (c) 标记 | **genome**（建议 meta-observer+gemini-challenger 审查，非 020 阻断） |
| `.chanlun/review-results/rtas-c-mechanization-20260623.md` | 本文件（整合诊断+方案+清单） | review-result |

**验证**：两个 hook `bash -n` 通过；用 live lead session（14c95478）模拟 Stop hook → 正确 scope 到本 team
（仅显示本 team 2 个 in_progress，无跨 session 污染）；(c)spawn 路由消息隔离测试格式正确；bootstrap.sh 模拟输出正确。

**未改**：`ceremony_scan.py`（活循环载体改用 live TaskList + Stop-Guard，scan 保持纯 seed 角色，
避免在确定性 genome 扫描器引入 session-id 识别脆弱性——见边界条件1）；`team-topology.json`（声明已足够，
on_demand trigger 由 (c) task-driven 桥消费）；Rust/Python 业务代码。

---

## 五、结果包六要素

**1. 结论**：(c) 机制化已实装。循环载体 = live TaskList（Lead 扫无主任务 spawn）+ Stop-Guard 强制（block-per-无主任务）；
结构工位 = Lead ceremony 常设 spawn（检查1.5 强制）；按需工位 = 工位 TaskCreate + metadata.agent_type 桥；
worker 递归 = TaskCreate 子任务 DAG（扬弃 TeamCreate）。6 文件已改并验证。

**2. 定义依据**：
- (c)裁决（编排者 2026-06-23）：递归在任务结构里，Lead 只编排。
- harness 实测结算（swarm-mechanism-fix-20260623）：flat roster，teammate 不能 spawn teammate，TeamCreate 废弃。
- 输入特征满足：live team 6 结构工位齐全（检查1.5 信号），live TaskList 有 owner/status/blockedBy 字段（spawn-per-无主可判定），
  Stop hook 有 session_id（可确定性定位本 Lead team 任务目录）。

**3. 边界条件（结论翻转）**：
1. harness 恢复 teammate→teammate spawn（flat roster 解除）→ 095号子蜂群 TeamCreate 递归重新可用，(c) 可升级为真调用栈递归。
2. `TaskCreate` 语义变更（不再 todo 管理）→ worker 子任务递归工具需替换。
3. 任务目录名 ≠ team 目录名（当前实测一致）→ Stop-Guard 的 LEAD_TASK_DIR 定位需改。
4. Stop hook 不提供 session_id → LEAD_TASK_DIR 为空 → 任务队列不强制（退化为 solo，与检查1.5 同构，非 bug）。
5. 结构工位数量变更（team-topology.json）→ 检查1.5 的 required 列表 + bootstrap 列表需同步。

**4. 下游推论**：
- 蜂群循环现在对 worker 递归子任务**可见且自动**（Stop-Guard block-per-无主任务 → Lead spawn）。
- 跨 session 任务污染消除 → 不动点终止可靠（D5 修复）。
- 按需工位纳入确定性循环（metadata.agent_type 桥），不再靠手动判断。
- 业务工位 prompt 模板应包含"识别子工作 → TaskCreate（metadata.agent_type）"指导（ceremony.md 步骤5 递归判断块已注入）。
- dispatch-dag.yaml 是 genome，建议 genealogist 把 (c) 裁决记为 settled 语法记录 + meta-observer/gemini-challenger 异质审查。

**5. 谱系引用**：
- (c)裁决（2026-06-23，编排者）：本机制化的源裁决。
- swarm-mechanism-fix-20260623：harness 实测结算（flat roster/Agent/TaskCreate）——关键前提。
- 095号（agent-team-recursive-swarm）：被 (c) 扬弃机制的子蜂群 TeamCreate 模型。
- 097号：五特征最小 DAG 模板——(c) 子任务 DAG 需满足的结构。
- 562号：结构=teammate，ceremony 必 spawn 6 常设结构工位（扬弃 075）。
- 275号：局部依赖——全局 DAG 从局部子任务创建涌现，Lead 不预规划。
- 218号：Lead 并行化；224/225号：push→rescan / rescan→evaluate 原子性。
- 082号：D策略（被 (c) task-trigger 桥升格为确定性触发）。
- 016号：规则无代码强制就不会执行（D2/D3/D4 诊断的理论依据）。
- 137号：否定性禁令对行为执行层无效（Stop-Guard 用正面 (c)spawn 指令，非禁令）。
- feedback_task_queue_owner_liveness / reference_stopguard_zombie_tasks：D5 跨 session 污染事故先例。

**6. 影响声明**：改动 6 文件（2 hook + 1 command + 1 skill + 1 genome dag + 本 review-result）。
影响模块：蜂群循环载体（scan-seed → TaskList-live）、Stop-Guard 任务扫描范围（全局 → 本 Lead team）、
结构/按需工位 spawn 模型（stale TeamCreate → (c) Agent/TaskCreate）、递归定义（子 team → 子任务 DAG）。
不影响：Rust/Python 业务代码、谱系定义文件（无新概念，机制化已结算裁决）、ceremony_scan.py（保持纯 seed）。
genome 改动（dispatch-dag.yaml）已标注建议异质审查，非 020 阻断（020 仅 CLAUDE.md）。
