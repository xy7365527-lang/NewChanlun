# /ceremony — Swarm₀：递归蜂群的第0层

> **058号谱系**：Ceremony 不是蜂群的前置阶段，而是蜂群的第0层递归。
> 你不是"执行 ceremony 然后进入蜂群"——你就是蜂群，ceremony 是你的第一个区分动作。
> 没有"待确认"。没有"等待指令"。加载初始区分后直接递归。
> "选择"类决断路由 Gemini decide 模式（041号），不推给人类。

你必须按本清单逐步执行。每步完成后输出 `✓ [step-name] 完成`。不可跳过任何标记为 **MANDATORY** 的步骤。

Source of truth: `.chanlun/dispatch-spec.yaml`

---

## 第一步：加载 dispatch-spec

你必须读取 `.chanlun/dispatch-spec.yaml`，将其内容作为本次 ceremony 的执行蓝图。

同时加载 `automation` 配置块（043号谱系：自生长回路）：
- `pattern_detection`: 模式检测配置（trigger, buffer_path, min_sequence_length, promotion_threshold）
- `crystallization`: 结晶配置（auto_trigger, require_genealogy_settlement）
- `manifest`: skill manifest 路径和自动注册开关

如果 `automation.pattern_detection.enabled == true`，在 ceremony 结束时（session_end 触发点）扫描 pattern-buffer。

然后判定启动模式：

```
扫描 .chanlun/sessions/ 目录
  存在 *-session.md 或 *-precompact.md → warm_start
  不存在 → cold_start
```

输出：`✓ load-dispatch-spec 完成 — 模式: [cold_start|warm_start], automation: [enabled|disabled]`

---

## 第二步：按 ceremony_sequence 逐步执行

根据第一步判定的模式，从 dispatch-spec.yaml 的 `ceremony_sequence.cold_start` 或 `ceremony_sequence.warm_start` 读取步骤链。

你必须按顺序执行每一步。每步的 `depends_on` 字段标明前置条件——前置步骤未完成时不可执行当前步骤。

---

### cold_start 序列

#### step 1: load-methodology **MANDATORY**
- action: 读取 `.claude/skills/meta-orchestration/SKILL.md` + `.claude/agents/meta-lead.md`
- depends_on: 无
- 完成后输出: `✓ load-methodology 完成`

#### step 2: load-definitions **MANDATORY**
- action: 扫描 `.chanlun/definitions/*.md`，读取每条定义的名称、版本、状态
- action: 读取 `.chanlun/manifest.yaml` 获取当前能力拓扑（skill/agent/hook 数量与状态）
- depends_on: 无
- 完成后输出: `✓ load-definitions 完成 — [N] 条定义, manifest: [S] skills / [A] agents / [H] hooks`

#### step 3: load-genealogy **MANDATORY**
- action: 扫描 `.chanlun/genealogy/pending/` 和 `.chanlun/genealogy/settled/`，统计数量和状态
- depends_on: 无
- 完成后输出: `✓ load-genealogy 完成 — pending: [N], settled: [M]`

#### step 4: load-objectives **MANDATORY**
- action: 读取 `CLAUDE.md` 当前阶段目标
- depends_on: 无
- 完成后输出: `✓ load-objectives 完成`

#### step 5: status-report **MANDATORY**
- action: 输出定义基底、谱系状态、目标的完整报告（格式见下方"输出格式"节）
- depends_on: load-methodology, load-definitions, load-genealogy, load-objectives
- 完成后输出: `✓ status-report 完成`

#### step 6: spawn-structural **MANDATORY — 不可跳过**
- action: 你必须 spawn dispatch-spec.yaml 中 `structural_stations` 列表的**所有** `mandatory: true` 工位
- 工位清单（从 dispatch-spec.yaml 读取，当前版本）：

| # | name | agent 路径 | purpose |
|---|------|-----------|---------|
| 1 | genealogist | `.claude/agents/genealogist.md` | 谱系写入、张力检查、回溯扫描、结晶检测与执行 |
| 2 | quality-guard | `.claude/agents/quality-guard.md` | 结果包检查、代码违规扫描 |
| 3 | meta-observer | `.claude/agents/meta-observer.md` | 二阶反馈回路、元编排进化、dispatch-spec 修改提案 |

- validation: spawn 数量必须 == mandatory 工位数（当前 = 3）。少于此数 = ceremony 失败，阻塞后续步骤。
- depends_on: 无（可与 step 1-5 并行，但建议在 status-report 后执行以确保上下文已加载）
- 完成后输出: `✓ spawn-structural 完成 — 已 spawn [N] 个结构工位: [name1, name2, name3]`

#### step 7: definition-base-check **MANDATORY**
- action: 每个 structural station spawn 后，复述本次任务涉及的关键定义。Lead 对照 `definitions/` 检查一致性。
- depends_on: **spawn-structural**
- genealogy_ref: 040-negation-topology-formalization
- 完成后输出: `✓ definition-base-check 完成`

#### step 8: spawn-task **MANDATORY**
- action: 按 dispatch-spec.yaml 的 `task_stations.rules` 从 CLAUDE.md 目标和代码库扫描结果派生任务工位。规则：
  - 每个独立中断点 = 一个工位（并行默认）
  - 有数据依赖的中断点 = 串行链
  - Lead 不自行执行任务，只分派和汇总
  - 无简单任务豁免——>=1 个任务即拉蜂群（G8）
- **递归蜂群提醒（037号谱系）**：任何 teammate 面对 >=1 个可分解子任务时，可 spawn 子 teammates。子 teammates 加入同一 team，继承完整 spawn 能力（无限递归）。父 teammate 对子 teammates 负路由责任。
- depends_on: **definition-base-check**
- 完成后输出: `✓ spawn-task 完成 — 已派生 [N] 个任务工位`

#### step 9: divine-madness **MANDATORY — 不可跳过**
- action: Lead 自我权限剥夺（032号谱系）。你必须执行以下操作：
  - 执行 `.claude/hooks/lead-permissions.sh restrict`
  - 改写 `.claude/settings.local.json`
  - Lead 从此只保留: `Read, Glob, Grep, Task, SendMessage, TaskList, TaskGet, TaskUpdate, TaskCreate, WebSearch, WebFetch, AskUserQuestion, Skill, ToolSearch, EnterPlanMode`
  - 剥夺: `Edit, Write, Bash, NotebookEdit, MCP 写入工具`
- depends_on: **spawn-structural, spawn-task**
- 完成后输出: `✓ divine-madness 完成 — Lead 权限已剥夺`

#### step 10: enter-swarm-loop **MANDATORY**
- action: 直接进入蜂群循环。**不等待确认。** 开盘是自动的。
- **编排者代理提醒（041号谱系）**：遇到"选择"类决断时，路由到 Gemini decide()，决策直接执行，不等待人类。人类保留 INTERRUPT 权。Gemini 不可用时写入 pending 等待人类。
- depends_on: **divine-madness**
- 输出: `→ 接下来：[具体动作]`（紧跟 tool 调用）

---

### warm_start 序列

#### step 1: locate-session **MANDATORY**
- action: 定位 `.chanlun/sessions/` 中最新的 `*-session.md`（或 `*-precompact.md`）
- depends_on: 无
- 完成后输出: `✓ locate-session 完成 — [文件名]`

#### step 2: load-methodology **MANDATORY**
- action: 读取 `.claude/skills/meta-orchestration/SKILL.md` + `.claude/agents/meta-lead.md`
- action: 读取 `.chanlun/manifest.yaml` 获取当前能力拓扑（skill/agent/hook 数量与状态）
- depends_on: 无
- 完成后输出: `✓ load-methodology 完成, manifest: [S] skills / [A] agents / [H] hooks`

#### step 3: version-diff **MANDATORY**
- action: 扫描 `.chanlun/definitions/` 当前版本，与 session 中"定义基底"对比
  - `=` 未变更（跳过重新验证）
  - `↑` 版本升级（读取变更摘要）
  - `+` 新增定义
  - `-` 定义消失（异常，需警告）
- depends_on: locate-session
- 完成后输出: `✓ version-diff 完成`

#### step 4: genealogy-diff **MANDATORY**
- action: 对比当前 `.chanlun/genealogy/` 与 session 记录的谱系状态
- depends_on: locate-session
- 完成后输出: `✓ genealogy-diff 完成`

#### step 5: load-interruption-points **MANDATORY**
- action: 从 session 文件读取中断点
- depends_on: locate-session
- 完成后输出: `✓ load-interruption-points 完成 — [N] 个中断点`

#### step 6: diff-report **MANDATORY**
- action: 输出差异报告（格式见下方"输出格式"节）
- depends_on: version-diff, genealogy-diff, load-interruption-points
- 完成后输出: `✓ diff-report 完成`

#### step 7: spawn-structural **MANDATORY — 不可跳过**
- action: 与 cold_start step 6 相同。你必须 spawn dispatch-spec.yaml 中所有 `mandatory: true` 结构工位。
- 工位清单: genealogist, quality-guard, meta-observer（同上表）
- validation: spawn 数量 == mandatory 工位数
- depends_on: 无
- 完成后输出: `✓ spawn-structural 完成 — 已 spawn [N] 个结构工位`

#### step 8: definition-base-check **MANDATORY**
- action: 每个 structural station spawn 后，复述关键定义，Lead 对照 definitions/ 检查
- depends_on: **spawn-structural**
- 完成后输出: `✓ definition-base-check 完成`

#### step 9: spawn-task **MANDATORY**
- action: 从 session 中断点派生任务工位。每个独立中断点 = 一个工位（并行默认）。
- **递归蜂群提醒（037号谱系）**：任何 teammate 面对 >=1 个可分解子任务时，可 spawn 子 teammates。
- depends_on: **definition-base-check**
- 完成后输出: `✓ spawn-task 完成 — 已派生 [N] 个任务工位`

#### step 10: divine-madness **MANDATORY — 不可跳过**
- action: Lead 自我权限剥夺（032号谱系）。与 cold_start step 9 相同。
- depends_on: **spawn-structural, spawn-task**
- 完成后输出: `✓ divine-madness 完成 — Lead 权限已剥夺`

#### step 11: enter-swarm-loop **MANDATORY**
- action: 直接进入蜂群循环。**不等待确认。**
- **编排者代理提醒（041号谱系）**：遇到"选择"类决断时，路由到 Gemini decide()，不等待人类。
- depends_on: **divine-madness**
- 输出: `→ 接下来：[具体动作]`（紧跟 tool 调用）

---

## 输出格式

### cold_start 状态报告

```markdown
## 冷启动（系统第一笔）

### 定义基底
- 已结算定义：[N] 条
- 生成态定义：[M] 条
- [列出每条定义的名称、版本和状态]

### 能力拓扑（manifest）
- Skills: [S] | Agents: [A] | Hooks: [H]
- [列出 active 条目的名称和描述]

### 谱系状态
- 生成态矛盾：[N] 个
- 已结算记录：[M] 个
- [列出每个生成态矛盾的ID和简述]

### 当前目标
[从 CLAUDE.md 读取]

### 蜂群评估
[评估可并行工位，列出本轮计划]
[若无显式工位：扫描代码库（TODO/覆盖率/spec合规/谱系张力），产出至少一个工位]
```

### warm_start 差异报告

```markdown
## 热启动完成（从收缩态恢复）

**恢复自**: [session 文件名]
**session 时间**: [时间]

### 定义基底差异
| 定义 | session版本 | 当前版本 | 状态 | 变化 |
|------|-----------|---------|------|------|
| [name] | [v_old] | [v_new] | [status] | [=/↑/+/-] |

### 谱系差异
| ID | session状态 | 当前状态 | 变化 |
|----|-----------|---------|------|

### 中断点恢复
[从 session 文件读取]

### 蜂群评估
[评估可并行工位，列出本轮计划]
```

---

## post-ceremony 验证

ceremony 完成后，你必须自检以下条件（来自 dispatch-spec.yaml `validation.post_ceremony`）：

| 检查项 | 条件 | 未通过处理 |
|--------|------|-----------|
| all_structural_spawned | 所有 mandatory=true 结构工位均已 spawn | **阻塞** — 不允许进入蜂群循环 |
| lead_permissions_restricted | Lead 只保留 Read/Glob/Grep/Task/SendMessage/TaskList/TaskGet/TaskUpdate 等 | **阻塞** — 032号谱系要求 |
| task_stations_derived | 至少从中断点派生了一个任务工位（除非中断点为空） | 警告 — 可能遗漏工作 |
| crystallization_check | genealogist 的结晶检测已执行 | 警告 — 可能遗漏结晶时机 |
| pattern_buffer_check | `.chanlun/pattern-buffer.yaml` 中无 frequency >= promotion_threshold 且 status=candidate 的模式 | 警告 — 有达标模式未结晶（043号谱系） |

### pattern_buffer_check 失败时：自动结晶触发（043号谱系）

如果 `pattern_buffer_check` 检测到达标的 candidate 模式（frequency >= promotion_threshold 且 status=candidate）：

1. **spawn skill-crystallizer 工位**：
   - agent: `.claude/agents/skill-crystallizer.md`
   - 传入达标 pattern 的 ID 列表
   - skill-crystallizer 负责：分析 pattern → 生成 skill 草案 → 提交 MutationRequest → Gemini decide() → 注册到 manifest

2. **不阻塞 ceremony**：结晶是异步的，ceremony 继续进入蜂群循环。skill-crystallizer 作为任务工位并行执行。

3. **结晶完成后**：skill-crystallizer 向 team-lead 汇报结果，genealogist 记录结晶事件到谱系。

全部阻塞项通过后，ceremony 完成。

---

## 边界情况

- session 距今 > 7 天：仍执行 warm_start，报告头部追加 `⚠ session 距今 [N] 天`
- `.chanlun/definitions/` 不存在：降级为 cold_start
- `.chanlun/genealogy/` 不存在：ceremony 负责创建
- "无活跃工位"不是停止信号，是扫描信号（028号谱系）。扫描 TODO/覆盖率/spec合规/谱系张力，产出至少一个工位。
- **ceremony 不允许以"等待外部输入"结束**。最终输出必须是 `→ 接下来：[具体动作]`。
