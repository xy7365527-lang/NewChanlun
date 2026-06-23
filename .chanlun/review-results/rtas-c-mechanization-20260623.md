---
trigger: task-35-rtas-mechanization
mode: architecture-update
result: done
grounded_in: ["069", "092", "097", "153", "177", "272"]
---
# 机制化 (c) 真 RTAS 任务-DAG 递归（已实装，ground 在 RTAS 谱系）

**工位**：rtas-infra-fix（topo_address: swarm/rtas-infra-fix），整合 rtas-mechanizer 诊断 + bootstrap-fix 草稿
**任务**：#35　**日期**：2026-06-23
**认识论等级**：L2（harness 实测结算 + 谱系实读 + 文件实读 + hook 模拟验证；非合成假设）

> 生成史（012号保留）：本文件先由 `rtas-mechanizer` 写诊断版（其 skill Write 被拒，改动标"待执行"），
> `bootstrap-fix` 提交检查1.5 结构工位强制（562号）。`rtas-infra-fix`（本工位，有写权限）实装配套
> hook/command/skill 改动，并 ground 在编排者指定的 RTAS 谱系（069/153 等），整合为本权威版。

---

## 〇、第一性依据：先 ground 在 RTAS 谱系（编排者要求）

| 谱系 | 对本机制化的约束 |
|------|------------------|
| **069**（RTAS 定义 + 5层 + 两个 Gap）| **创世 Gap**：Swarm₀/ceremony/CC/Lead 是 bootstrap 节点——制定拓扑但不在拓扑内，**禁止纳入循环**；**视差 Gap**：异步时间差是结构性的，禁止消除；**层面3**：结构修改走提案模式（proposals/）+ meta-observer 注入任务（不自行 spawn）+ gemini-challenger 高阶审查；**矛盾6/8**：结构工位降级→meta-observer 注入任务、系统默认拉起 |
| **153**（持久化断裂）| 持久化不变量：每条退出路径以 session + commit + push 结束；ceremony_scan 只读 session——循环持久化必须靠 session 载体 + bootstrap 自举 + Stop-Guard 保活 |
| **097**（严格 RTAS 架构）| 五特征最小 DAG（任务/审查/异质审计/结晶 + 异步自指） |
| **092**（RTAS 严格审计）| 严格性约束 |
| **177**（topo-effect 执行）| 谱系结算携带 topo_effect 的执行机制 |
| **272**（RTAS 三议题裁决）| 局部依赖 + 运动形式约束 |

**核心 ground**：(c) 的"Lead 扫无主任务自动 spawn"机制，其"自动"的上界由**创世 Gap**界定——
hook **不能** spawn（平台约束：只有 bootstrap 节点 Lead/CC 能用 Agent tool spawn）。因此循环的最大机制化 =
**Stop-Guard 检测+保活+注入正面格式指令（137号）→ bootstrap 节点 spawn**。残余的"Lead 执行 spawn 动作"
不是待修复的缺口，而是创世 Gap 的结构性显形（069号：bootstrap 节点游离拓扑外，禁止纳入循环）。

---

## 一、编排者裁决 (c) + 两个追加硬要求

**(c)**：每个子任务自己向下递归（TaskCreate 子任务），Lead 只做编排（scan 无主任务→spawn 工位→re-scan→不动点终止）。
**追加1 循环持久化（自持续）**：RTAS 循环必须自持续，机制化"有无主任务⟹自动 spawn"为 137号机制强制。
**追加2 所有工位 auto-appearance**：结构工位 proactive spawn、按需工位事件触发、业务工位无主⟹spawn。

---

## 二、诊断：为何蜂群循环/结构工位/按需工位没正确启动（4 根因 + 1 缺陷）

- **D1 结构工位**：`agent-team-bootstrap.sh` 发 stale `Task(team_name="swarm")`，与 ceremony.md 已更新的隐式-team `Agent(name=…)` 冲突。
- **D2 按需工位**：13 个 trigger 是文本声明，无机制桥（`event_skill_map` 多数 `platform_support:false`，从不 fire）。
- **D3 蜂群循环**：`ceremony_scan.py` 只从 intent-sources 派生 workstations，**从不读 live TaskList** → 工位 TaskCreate 的子任务对循环不可见；唯一看见 live 任务的 Stop-Guard 检查2 只 block + 发泛化消息，非"为每个无主任务 spawn"的 (c) 指令。
- **D4 stale TeamCreate 模型**：sub-swarm-ceremony.md / dispatch-dag.yaml / ceremony.md 步骤5 全把 worker 递归路由到 TeamCreate（flat-roster 禁止，已上浮 spec-execution gap）。
- **D5 缺陷**：Stop-Guard 检查2 全局扫 `~/.claude/tasks/*/` → 跨 session 污染误阻停机（feedback_task_queue_owner_liveness 事故）。

---

## 三、(c) 机制化方案（ground 在 069/153）

### 3.1 循环持久化（自持续）—— 三支柱，尊重创世 Gap

| 支柱 | 载体 | 作用 | 谱系 ground |
|------|------|------|------------|
| **bootstrap 自举节点** | Lead/CC（Agent tool）| 唯一能 spawn 的节点；循环的 source 节点 | 069 创世 Gap（禁止纳入循环） |
| **保活强制** | `ceremony-completion-guard.sh`（Stop hook）| 每次 Stop 自动扫本 Lead team；有无主任务→block + 注入 (c)spawn 正面格式指令 | 137号机制强制 + 153 持久化 |
| **任务-DAG 递归** | 共享 TaskList | 工位 TaskCreate 子任务（unowned）= 向下递归；下一轮 Stop-Guard/TaskList 扫到 | (c)裁决 + 275 局部依赖 |
| **状态持久化** | session + commit + push | 跨 compaction/context 的状态载体 | 153 持久化不变量 |

**确定性循环步**（"有无主任务⟹自动 spawn"）的机制化 = Stop-Guard 检测无主任务 → block（保活）→
注入正面格式 `(c)spawn：为每个无主任务 spawn 一个工位 Agent(...)`（137号：否定性禁令对执行层无效，用正面格式）。
**视差 Gap 保留**：worker TaskCreate 与 Lead spawn 之间有一个 Stop 周期的时间差，结构性，不消除。

### 3.2 所有工位 auto-appearance —— ground 在 069 矛盾6/8

| 工位类 | 出现机制 | primary / backstop | 谱系 ground |
|--------|---------|--------------------|------------|
| **结构工位（6 常设）** | Lead ceremony 第一动作 proactive 并行 spawn `Agent(name=…)` | primary=proactive；backstop=Stop-Guard 检查1.5 机制强制 | 562（结构=teammate 必 spawn）+ 069 矛盾6（系统默认拉起） |
| **按需工位（challengers 等）** | 事件→工位/meta-observer `TaskCreate`（metadata.agent_type=<工位类型>）→ Lead 扫无主任务 spawn | 事件触发（task-driven 确定性桥，非 D策略软路由） | 069 矛盾8（meta-observer 注入任务、系统默认拉起）+ (c) |
| **业务工位** | ceremony_scan seed → Lead spawn；或工位递归 TaskCreate→Lead spawn | scan seed + 无主⟹spawn | (c) + 275 |

**562 扬弃 075 后的触发机制厘清**：结构工位是常设 teammate（ceremony proactive spawn，不靠事件）；
按需 challengers 是 conditional——其"出现"= 触发事件发生时，检测到需求的工位（或 meta-observer，069矛盾8
不自行 spawn 任务工位）`TaskCreate` 一个带 `metadata.agent_type` 的子任务，Lead 的 (c) 循环 spawn 之。
这把 D策略（082号 hook 提示 + 手动认领）升格为任务驱动的确定性出现。

### 3.3 递归模型扬弃（消除 D4）

worker 递归 = `TaskCreate` 子任务 DAG（四类节点 + blockedBy + metadata.agent_type），不 TeamCreate。
异步自指（约束3）保留：审查节点由**不同**工位执行。这是 095号"真递归"的 Aufhebung（否定 TeamCreate 机制、
保留递归精神+四类节点+约束4、提升载体从 team 拓扑到任务拓扑）。

### 3.4 自指=自我进化：genome 走提案模式（069 层面3，不擅改）

`dispatch-dag.yaml` 是 genome 拓扑定义。编排者要求"genome 走 020-gated 提案，不擅改"。
rtas-infra-fix 曾直接编辑 → **已 git revert** → 改为 **PROP-002**（`.chanlun/proposals/PROP-002-dispatch-dag-c-task-recursion.md`），
经 meta-observer 注入 + gemini-challenger 高阶审查（069矛盾5：避免 quality-guard t/t+1 保守死锁）。
批准前 genome 保持 stale（020-gated 不擅改的代价，符合 069层面3）。

---

## 四、改的文件清单

### 已直接实装（非 genome；hooks=immune_system，command/skill；建议 quality-guard + 审查链复核，069层面3）

| 文件 | 改动 | 类型 | 验证 |
|------|------|------|------|
| `.claude/hooks/agent-team-bootstrap.sh` | 删 stale `Task(team_name)`，改隐式-team `Agent`；新增 (c) 循环 + structural proactive + 按需 task-trigger 描述 | hook（免疫系统） | `bash -n` ✓ + 模拟输出 ✓ |
| `.claude/hooks/ceremony-completion-guard.sh` | (1) 任务扫描 scope 到本 Lead team（修 D5）；(2) 追踪无主任务 + (c)spawn-per-无主任务 正面格式路由消息（137号） | hook（Stop-Guard，保活强制器） | `bash -n` ✓ + live session 模拟正确 scope ✓ + (c)spawn 消息隔离测试 ✓ |
| `.claude/commands/ceremony.md` | 步骤6 显式 (c) TaskList 驱动循环（spawn-per-无主）；步骤5 递归判断改 TaskCreate；步骤9/10 不动点加 TaskList 条件；不变量加 (c) 条 | command | 一致性 grep ✓ |
| `.claude/skills/sub-swarm-ceremony/SKILL.md` | 整文件严格重写为 (c) 子任务 DAG（删旧 TeamCreate 顶部，并入 bootstrap-fix 的 harness约束/角色分工/区别表） | skill（knowledge_template） | grep 无 stale 指令 ✓ |

### genome 改动 → 提案（不擅改，069层面3）

| 文件 | 状态 | 提案 |
|------|------|------|
| `.chanlun/dispatch-dag.yaml` | **已 revert（保持原状）** | `.chanlun/proposals/PROP-002-dispatch-dag-c-task-recursion.md`（pending_review，待 meta-observer 注入 + gemini-challenger 审查） |

### 报告
- `.chanlun/review-results/rtas-c-mechanization-20260623.md`（本文件）

**ceremony_scan.py 不改**（genome，保持纯 seed 角色）：活循环载体改用 live TaskList + Stop-Guard（有 session_id），
避免在确定性 bootloader 引入 session-id 识别脆弱性，且符合 069（ceremony_scan = bootloader，Stop-Guard = 免疫系统，
bootstrap 节点 = spawner，三者分离尊重创世 Gap）。若未来需 ceremony_scan 读 live 任务，走提案模式。

---

## 五、结果包六要素

**1. 结论**：(c) 已实装（5 直接文件 + 1 genome 提案）。循环持久化 = bootstrap 自举节点 + Stop-Guard 保活强制（137号）+
任务-DAG 递归 + session 持久化（153），尊重创世 Gap（CC 不纳入循环）+ 视差 Gap（时间差保留）。
auto-appearance：结构=proactive ceremony spawn（检查1.5 backstop）、按需=事件→TaskCreate(metadata.agent_type)→spawn（069矛盾8）、业务=无主⟹spawn。
genome 改动走 PROP-002 提案（069层面3 不擅改）。

**2. 定义依据**：(c)裁决 + 追加要求（编排者 2026-06-23）；harness 实测（flat roster）；069（两个 Gap + 层面3 + 矛盾6/8）；153（持久化不变量）；097（五特征 DAG）；562（结构=teammate）。输入特征满足：live team 6 结构工位齐全（检查1.5 信号）、live TaskList 有 owner/blockedBy（spawn-per-无主可判定）、Stop hook 有 session_id（确定性定位本 Lead team）。

**3. 边界条件（结论翻转）**：
1. harness 恢复 teammate→teammate spawn → 095号 TeamCreate 递归重新可用，(c) 升级为真调用栈递归。
2. 平台允许 hook 直接 spawn → 循环可完全脱离 bootstrap 节点（创世 Gap 物质形态改变）——但 069 断言创世 Gap 不可消除，故此翻转存疑。
3. `TaskCreate` 不再 todo 管理 → worker 递归工具需替换。
4. 任务目录名 ≠ team 目录名（当前实测一致）→ Stop-Guard LEAD_TASK_DIR 定位需改。
5. Stop hook 不提供 session_id → LEAD_TASK_DIR 空 → 任务队列不强制（退化 solo，与检查1.5 同构，非 bug）。
6. PROP-002 被 gemini-challenger 否决 → genome 保持 stale，配套实装与 genome 不一致需回滚。

**4. 下游推论**：
- 蜂群循环对 worker 递归子任务可见且保活（Stop-Guard block-per-无主 → bootstrap 节点 spawn）。
- 跨 session 污染消除（D5）→ 不动点终止可靠。
- 按需工位纳入确定性出现（metadata.agent_type 桥），不再靠手动判断。
- genealogist 应将 (c) 裁决记为 settled 谱系（语法记录/选择类，编排者已决）。
- meta-observer 应注入 PROP-002 审查任务（069矛盾6 不自行 spawn）；gemini-challenger 高阶审查。
- quality-guard 应复核已直接实装的 hook/command/skill 结构改动（069层面3）。

**5. 谱系引用**：(c)裁决（2026-06-23）；069（RTAS 定义/两个 Gap/层面3/矛盾5/6/8——本机制化第一性依据）；
153（持久化断裂——循环持久化要求对应）；097（五特征 DAG）；092（严格审计）；177（topo-effect）；272（三议题）；
095（被扬弃的 TeamCreate 模型）；562（结构=teammate proactive spawn）；275（局部依赖）；137（正面格式机制强制）；
082（D策略，被升格为 task-trigger）；016（规则无代码强制不执行）；
feedback_task_queue_owner_liveness / reference_stopguard_zombie_tasks（D5 事故先例）；swarm-mechanism-fix-20260623（harness 实测）。

**6. 影响声明**：直接改 5 文件（2 hook + 1 command + 1 skill + 本 report）；genome（dispatch-dag.yaml）改动走 PROP-002 提案（已 revert 直接编辑）。
影响模块：蜂群循环载体（scan-seed + live-TaskList/Stop-Guard 保活）、Stop-Guard 任务扫描范围（全局→本 Lead team）、
工位 auto-appearance（structural proactive + 按需 task-trigger + 业务无主⟹spawn）、递归定义（子 team→子任务 DAG）。
不影响：ceremony_scan.py（纯 seed）、Rust/Python 业务代码、谱系定义文件。
genome 改动经提案模式（069层面3），未擅改；未触发 CLAUDE.md 020 阻断（仅 CLAUDE.md 触发 020）。
自指=自我进化合规：本工位修蜂群结构，genome 走提案、配套实装建议审查链复核。
