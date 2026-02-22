# 基础设施声明-能力审计报告

## 审计范围
- dispatch-dag.yaml: 12 个事件 / 6 个验证条件 / 9 个 event_skill_map 节点
- hooks: 21 个文件（磁盘） / settings.json 注册情况交叉核对
- skills: 7 个目录（每个含 SKILL.md）
- agents: 19 个文件
- manifest.yaml: v2.0（声称与实际 19 agents / 21 hooks / 13 commands / 6 skills 对齐）

---

## 1. dispatch-dag 事件审计（event_edges 节）

| 事件 | 声明处理 | 实际实现 | 状态 |
|------|---------|---------|------|
| `file_change spec/theorems/*` | 激活 gemini-challenger verify | double-helix-verify.sh 在 git commit 前调用 Gemini verify，但匹配的是所有 staged diff，不是专门监听 spec/theorems/ 变更 | **部分实现** — 无专门的 file_change 事件检测器，靠 pre-commit 全量 diff 间接覆盖 |
| `file_change docs/chan_spec.md` | 激活 genealogist lineage_consistency_check | 无对应 hook | **未实现** — 无 hook 监听 docs/chan_spec.md 变更 |
| `file_create **/*` | 激活 manifest_guard check_manifest_registration | topology-guard.sh 在 git commit 后检查新增文件的 manifest 注册 | **已实现**（PostToolUse/Bash on git commit） |
| `annotation @proof-required` | 激活 gemini-challenger math challenge | ceremony-completion-guard.sh 的检查4扫描 @proof-required 标签 | **部分实现** — Stop hook 扫描未验证标签并阻断，但不自动路由到 Gemini |
| `task_queue independent_tasks >= 2` | 激活 swarm_manager spawn_recursive_swarm | agent-team-enforce.sh 强制 Task 调用使用 team_name | **间接实现** — hook 强制 team 使用，但无自动检测 task_queue 数量并 spawn 的逻辑 |
| `session_start` | 激活 ceremony | session-start-ceremony.sh (SessionStart hook) | **已实现** |
| `session_end` | 激活 pattern_detector | post-session-pattern-detect.sh (Stop hook) | **已实现** |
| `tool_error gemini_api_*` | 激活 fallback_local_rules | double-helix-verify.sh 内置 Gemini 不可达降级（`except` 分支） | **部分实现** — 仅 double-helix 有降级路径，其他 Gemini 调用无统一 fallback |
| `build_failure consecutive_failures >= 3` | 激活 build_resolver auto_fix_build | 无对应 hook | **未实现** — 无 hook 检测连续构建失败并自动触发 build-error-resolver |
| `command /inquire` | 激活 quality-guard four_step_inquiry | 有 .claude/commands/inquire.md 命令文件 | **已实现**（通过 skill/command） |
| `command /escalate` | 激活 genealogist write_pending_genealogy | 有 .claude/commands/escalate.md 命令文件 | **已实现**（通过 skill/command） |
| `swarm_cycle_end` | 激活 meta-observer 元规则一致性检查 | meta-observer-guard.sh (Stop hook) 在 session 结束时检查二阶观察 | **部分实现** — Stop hook 检查，但无 swarm_cycle_end 概念的独立事件 |

---

## 2. dispatch-dag 条件审计（validation 节）

| 条件 | 声明检查 | 实际实现 | 状态 |
|------|---------|---------|------|
| `quality_gate_reachability` | 所有 task_node 产出经 quality-guard | result-package-guard.sh + downstream-action-guard.sh 在 Write/Edit 后触发 | **部分实现** — 只检查谱系文件写入，不检查所有 task 产出 |
| `single_terminal_sink` | skill-crystallizer 是唯一 terminal_sink | crystallization-guard.sh 检查结晶债务和 pattern-buffer | **已实现**（通过 hook 强制结晶路径） |
| `no_cycles` | 除 meta-observer 自环外无环 | 无运行时环路检测 | **未实现** — 纯声明，无代码检测 |
| `skills_registered` | 所有 structural skill agent 文件存在 | 无运行时检查 | **未实现** — ceremony_scan.py 不验证 agent 文件存在性 |
| `lead_audit_registered` | lead-audit hook 已注册 | lead-audit.sh 存在但**未在 settings.json 中注册** | **缺口** — 文件存在但未激活 |
| `crystallization_check` | genealogist 结晶检测已执行 | crystallization-guard.sh 在 git commit 时检查 | **部分实现** — 在 commit 时检查，不在 ceremony 后检查 |
| `roadmap_scanned` | ceremony 扫描 roadmap.yaml | ceremony_scan.py 应有 roadmap 扫描 | **待验证** — 需确认 ceremony_scan.py 实际读取 roadmap.yaml |
| `task_stations_derived` | 至少派生一个任务工位 | ceremony 流程内有 no_work_fallback | **已实现**（ceremony 流程保障） |

---

## 3. Hooks 审计

### 3.1 磁盘上存在但未在 settings.json 注册的 hooks

| Hook 文件 | 声明功能 | 注册状态 | 状态 |
|-----------|---------|---------|------|
| **lead-audit.sh** | Lead 拓扑异常对象化审计（088号） | **未注册** | **严重缺口** — 文件功能完整，但 Claude Code 不会执行它 |
| **genealogy-gemini-verify.sh** | 谱系写入后建议 Gemini verify | 已注册（PostToolUse/Write+Edit） | 已实现 |

### 3.2 manifest.yaml 声明但磁盘上不存在的 hooks

| Manifest 声明 | 实际文件 | 状态 |
|--------------|---------|------|
| **ceremony-guard.sh** (PreToolUse/Task) | **不存在** | **manifest 虚假声明** — 文件已被 agent-team-enforce.sh 替代但 manifest 未更新名称 |
| **lead-permissions.sh** (PostToolUse/Bash) | **不存在** | **manifest 虚假声明** — 已被 lead-audit.sh 替代（088号谱系），manifest 名称未更新 |
| **team-structural-inject.sh** (PostToolUse/TeamCreate) | **不存在** | **manifest 虚假声明** — 文件从未创建，manifest 声明了不存在的 hook |

### 3.3 已注册 hooks 的功能核对

| Hook 文件 | 注册事件/匹配 | 声明功能 | 实际能力 | 状态 |
|-----------|-------------|---------|---------|------|
| session-start-ceremony.sh | SessionStart | 自动 ceremony 热启动 | 检测冷/热启动，注入状态摘要 | **一致** |
| precompact-save.sh | PreCompact | 上下文压缩前状态保存 | 写入 session 文件 + 定义/谱系/git 状态 | **一致** |
| agent-team-enforce.sh | PreToolUse/Task | Task 调用强制 team_name | 阻断无 team_name 的 Task 调用 | **一致** |
| double-helix-verify.sh | PreToolUse/Bash | git commit 前 Gemini 验证 | 调用 Gemini decide() 检查 diff 一致性 | **一致** |
| definition-write-guard.sh | PreToolUse/Write+Edit | 定义文件格式验证 | 检查 status/version 字段 + 结算谱系关联 | **一致** |
| genealogy-write-guard.sh | PreToolUse/Write+Edit | 谱系文件格式强制 | 字段验证 + 前置引用 + meta-observer 检查 + 语义检查 | **一致** |
| spec-write-guard.sh | PreToolUse/Write+Edit | 核心文件修改记录 | 识别核心文件模式，输出审计提示 | **一致** |
| hub-node-impact-guard.sh | PreToolUse/Write+Edit | Hub 节点影响评估 | 检测 Top5 hub 节点修改，输出影响链 | **一致** |
| ceremony-completion-guard.sh | Stop | ceremony 完成守卫 + 通用停机阻断 | 5项检查（死寂/任务队列/生成态谱系/@proof/ceremony确认） | **功能超声明** — 名称暗示 ceremony 专用，实际是通用 Stop guard |
| post-session-pattern-detect.sh | Stop | 模式检测 | 提取 tool 调用序列，写入 pattern-buffer.yaml | **一致** |
| meta-observer-guard.sh | Stop | 二阶反馈强制 | 检查 meta-observer 执行标记，advisory 模式 | **一致**（降级为 advisory） |
| function-length-guard.sh | PostToolUse/TaskUpdate | 函数行数守卫 | 重构任务完成后扫描函数行数 | **一致** |
| flow-continuity-guard.sh | PostToolUse/Bash | 流程连续性 | git commit 后注入继续执行指令 | **一致** |
| crystallization-guard.sh | PostToolUse/Bash | 结晶守卫 | git commit 时检查结晶债务 + pattern-buffer | **一致** |
| topology-guard.sh | PostToolUse/Bash | 孤岛检测 + manifest 注册 | BFS 引用图扫描 + 新文件 manifest 检查 | **一致** |
| result-package-guard.sh | PostToolUse/Write+Edit | 结果包六要素检查 | 检查谱系文件的边界条件/下游推论/影响声明 | **一致** |
| downstream-action-guard.sh | PostToolUse/Write+Edit | 下游推论追踪 | 调用 downstream_audit.py 报告未解决下游行动 | **一致** |
| dag-validation-guard.sh | PostToolUse/Write+Edit | DAG 不变量验证 | 谱系文件写入后调用 validate_dag.py | **一致** |
| source-auditor-prompt.sh | PostToolUse/Write+Edit | 源头审计提示 | docs/ 变更后输出溯源标签验证提示 | **一致** |
| genealogy-gemini-verify.sh | PostToolUse/Write+Edit | Gemini 谱系异质审查提示 | 调用 gemini_genealogy_verify_prompt.py | **一致** |

---

## 4. Skills 审计

| Skill 目录 | dispatch-dag 声明 | 实际 SKILL.md | 调用方式 | 状态 |
|-----------|------------------|-------------|---------|------|
| gemini-math | genome_layer.knowledge_templates | 存在 | 通过 /challenge 命令触发 | **一致** |
| knowledge-crystallization | genome_layer.knowledge_templates | 存在 | 通过 skill-crystallizer agent 读取 | **一致** |
| math-tools | genome_layer.knowledge_templates | 存在 | 按需读取（数学形式化参考） | **一致** |
| meta-orchestration | genome_layer.knowledge_templates | 存在 | 通过 /ceremony 和 meta-lead agent | **一致** |
| orchestrator-proxy | genome_layer.knowledge_templates | 存在 | 通过 /escalate 命令的 Gemini decide 路径 | **一致** |
| spec-execution-gap | genome_layer.knowledge_templates | 存在 | 通过 ceremony scan 检测声明-能力缺口 | **一致** |
| sub-swarm-ceremony | genome_layer.knowledge_templates | 存在 | 095号谱系：子蜂群创建流程 | **一致** |

---

## 5. Agents 审计

### 5.1 event_skill_map 节点（9个）

| Agent 文件 | dispatch-dag 声明 | 文件存在 | 触发机制 | 状态 |
|-----------|------------------|---------|---------|------|
| genealogist.md | 谱系写入/张力检查/结晶检测 | 存在 | task_complete + /escalate + session_end（声明）；实际靠 Lead 手动调用 | **触发机制缺口** — 无 hook 自动在 task_complete 时触发 genealogist |
| quality-guard.md | 结果包检查/代码违规 | 存在 | file_write 到 genealogy/src（声明）；result-package-guard.sh + downstream-action-guard.sh 实际检查 | **部分一致** — hooks 执行部分检查，但不等同于 spawn quality-guard agent |
| meta-observer.md | 二阶反馈/元编排进化 | 存在 | session_end + swarm_cycle_end（声明）；meta-observer-guard.sh advisory | **部分一致** — advisory 模式降级 |
| code-verifier.md | 代码验证 | 存在 | file_write 到 src/tests（声明）；无对应 hook | **未实现** — 无 hook 在 Python/测试文件变更后自动触发代码验证 |
| skill-crystallizer.md | 结晶 | 存在 | pattern_buffer_ready（声明）；crystallization-guard.sh 间接触发 | **部分实现** — hook 检测但不 spawn agent |
| gemini-challenger.md | 异质否定/编排者代理 | 存在 | /challenge + spec/theorems 变更 + escalate_choice（声明）；double-helix-verify.sh + commands | **部分实现** |
| claude-challenger.md | 反向质询 | 存在 | manual_invocation（声明） | **一致**（手动调用设计） |
| source-auditor.md | 溯源标签验证 | 存在 | file_write docs/**（声明）；source-auditor-prompt.sh advisory | **部分实现** — advisory 不等同于 spawn agent |
| topology-manager.md | 拓扑分析 | 存在 | genealogy_count_threshold（声明）；无对应 hook | **未实现** — 无 hook 在谱系数量超阈值时触发 |

### 5.2 platform_layer 节点（10个）

| Agent 文件 | dispatch-dag 声明 | 文件存在 | 状态 |
|-----------|------------------|---------|------|
| architect.md | 系统架构设计 | 存在 | **一致** |
| planner.md | 实现规划 | 存在 | **一致** |
| tdd-guide.md | 测试驱动开发 | 存在 | **一致** |
| code-reviewer.md | 代码审查 | 存在 | **一致** |
| python-reviewer.md | Python 代码审查 | 存在 | **一致** |
| security-reviewer.md | 安全审查 | 存在 | **一致** |
| refactor-cleaner.md | 重构与死代码清理 | 存在 | **一致** |
| doc-updater.md | 文档更新 | 存在 | **一致** |
| meta-lead.md | 蜂群中断路由器 | 存在 | **一致** |
| build-error-resolver.md | 构建错误修复 | 存在 | **一致** |

---

## 6. dispatch-dag 与 dispatch-spec 一致性

dispatch-spec.yaml（v1.3）是 dispatch-dag.yaml（v3.1）的前身。两者存在以下差异：

| 项目 | dispatch-spec | dispatch-dag | 状态 |
|------|--------------|-------------|------|
| ceremony output_policy | "仅 recurse 节点产出输出" | "所有关键步骤向编排者展示" | **不一致** — dag 放松了输出限制 |
| scan-methodology | "读取 dispatch-spec.yaml" | "读取 dispatch-dag.yaml" | **spec 过时** — 仍引用自身 |
| structural_stations.mandatory spawn | "mandatory=true 全部 spawn" | 改为 skill 事件驱动，不 spawn | **架构变更未清理** — spec 仍有 spawn 逻辑 |
| divine-madness | "Lead 权限剥夺（032号）" | "Lead 拓扑异常审计注册（088号重设计）" | **spec 过时** |
| orchestration_protocol | 存在完整事件路由表 | 事件路由迁移到 event_edges | **重复** — spec 仍保留旧版 |
| claude-challenger triggers | re_challenge + stale_generative | manual_invocation（087号修复） | **spec 过时** — 仍有虚假事件声明 |

---

## 7. manifest.yaml 与实际文件一致性

| 类别 | manifest 声明 | 实际文件数 | 差异 | 状态 |
|------|-------------|-----------|------|------|
| agents | 19 | 19 | 无差异 | **一致** |
| hooks | 21（列出的） | 21（磁盘） | 名称映射有3个错误（见下） | **不一致** |
| commands | 13 | 需验证 | — | — |
| skills | 7（含 sub-swarm-ceremony） | 7 | 无差异 | **一致** |

manifest hook 名称错误：
1. `ceremony-guard` → 实际文件是 `agent-team-enforce.sh`（manifest 用旧名）
2. `lead-permissions` → 实际文件是 `lead-audit.sh`（manifest 用旧名）
3. `team-structural-inject` → 文件不存在（manifest 声明了幽灵 hook）

---

## 8. settings.json 注册 vs 磁盘文件

| 磁盘文件 | settings.json 注册 | 状态 |
|---------|-------------------|------|
| lead-audit.sh | **未注册** | **严重** — 088号谱系的核心实现完全失效 |
| 其余20个 hook | 全部注册 | 一致 |

---

## 严重缺口清单（按严重性排序）

### P0 — 声明的核心能力完全失效

1. **lead-audit.sh 未注册到 settings.json**
   - 影响：088号谱系（032号重设计）的核心实现——Lead 直接执行 Write/Edit/Bash 时的拓扑异常对象化审计完全不生效
   - dispatch-dag validation `lead_audit_registered` 要求此 hook 注册
   - 文件存在且功能完整，仅缺注册

2. **code-verifier 无触发机制**
   - dispatch-dag 声明 code-verifier skill 在 `file_write src/**/*.py` 和 `file_write tests/**/*.py` 时触发
   - 无对应 hook 实现此事件检测
   - code-verifier.md agent 存在但从不被事件驱动激活

3. **manifest.yaml 含3个幽灵条目**
   - `ceremony-guard.sh`、`lead-permissions.sh`、`team-structural-inject.sh` 不存在
   - manifest v2.0 声称"与实际严格对齐"——此声称为假

### P1 — 声明能力与实际实现不对称

4. **event_skill_map 的"事件驱动"大部分是声明性的**
   - dispatch-dag 声明 9 个 skill 由事件自动触发
   - 实际：hooks 执行部分检查逻辑，但不 spawn 对应 agent
   - genealogist/quality-guard/meta-observer/skill-crystallizer/topology-manager 的触发机制都是"hooks 提示 + Lead 手动认领"模式（D策略）
   - 这不是 bug——是 082号 D策略的设计意图——但 dispatch-dag 的措辞（"triggers"/"自动触发"）暗示自动化程度高于实际

5. **build_failure 事件未实现**
   - dispatch-dag 声明 `build_failure consecutive_failures >= 3` → `auto_fix_build`
   - 无 hook 检测连续构建失败
   - build-error-resolver agent 存在但只能通过 platform_layer 手动调用

6. **file_change docs/chan_spec.md 事件未实现**
   - dispatch-dag 声明此事件激活 genealogist lineage_consistency_check
   - 无 hook 监听此特定文件变更

7. **topology-manager 无触发机制**
   - dispatch-dag 声明 `genealogy_count_threshold` 条件触发
   - 无 hook 检测谱系数量阈值

### P2 — 文档/命名不一致

8. **dispatch-spec.yaml 严重过时**
   - 仍引用 032号 divine-madness（已被 088号替代）
   - 仍声明 structural_stations 的 mandatory spawn（已被 skill 事件驱动替代）
   - 仍有 claude-challenger 的虚假事件声明（已被 087号修正）
   - 仍有独立的 orchestration_protocol 节（已迁移到 dispatch-dag event_edges）

9. **ceremony-completion-guard.sh 名称误导**
   - 名称暗示 ceremony 专用，实际是通用 Stop guard（5项检查）
   - manifest 描述为"ceremony 完成守卫——6 项检查"，实际代码是 5 项检查

10. **meta-observer-guard.sh 降级为 advisory 但 dispatch-dag 未更新**
    - dispatch-dag validation 声明 `lead_audit_registered` 要求严格检查
    - 实际 hook 默认 STRICT_MODE=0（advisory 模式），仅输出提示

---

## 审计方法声明

本报告基于以下文件的逐行交叉核对：
- `.chanlun/dispatch-dag.yaml`（v3.1）
- `.chanlun/dispatch-spec.yaml`（v1.3）
- `.chanlun/manifest.yaml`（v2.0）
- `.claude/settings.json`（hooks 注册配置）
- `.claude/hooks/*.sh`（21个文件，全部逐行阅读）
- `.claude/skills/*/SKILL.md`（7个目录）
- `.claude/agents/*.md`（19个文件，存在性验证）

审计日期：2026-02-22
