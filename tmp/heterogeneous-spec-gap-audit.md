# 声明-能力一致性二次审查（异质视角）

**审计方法**：从能力层出发反查声明层——不问"声明了什么"，问"实际能做什么"。

## 一、Commands 审查（`.claude/commands/*.md`）

| 命令 | 声称功能 | 实际能力 | 缺口 |
|------|---------|---------|------|
| /ceremony | Lead 最小自举序列（10步） | ceremony_scan.py + TeamCreate + Task spawn | **消费缺口**：步骤中无 required_skills 消费步骤 |
| /challenge | 调用 Gemini 异质质询 | 需 GOOGLE_API_KEY + Serena MCP | **一致**（手动触发） |
| /inquire | 四步质询序列 | 纯指令，agent 按步骤执行 | **一致** |
| /escalate | 矛盾上浮 | 写入 pending/ 谱系 | **一致** |
| /ritual | 定义广播仪式 | 写定义 + 写谱系 + 回溯扫描 + 通知 | **一致** |
| /plan | 调用 planner agent | spawn planner subagent | **一致** |
| /tdd | 调用 tdd-guide agent | spawn tdd-guide subagent | **一致** |
| /code-review | 代码审查 + Codex | 代码审查逻辑 + 声称触发 codex-challenger | **部分缺口**：Codex 触发声明在命令文档中但无自动 spawn 机制 |
| /build-fix | 构建错误修复 | 通用构建修复流程 | **一致** |
| /refactor-clean | 死代码清理 | 通用清理流程 | **一致** |
| /update-docs | 文档更新 | 通用文档流程 | **一致** |
| /verify | 综合验证 | 构建+类型+lint+测试 | **一致** |
| /checkpoint | 工作点检查 | git stash/commit + 日志 | **一致** |

### 命令层缺口汇总

1. `/ceremony` 不消费 `required_skills`——这是最严重的缺口，意味着结构工位永远不会被 ceremony 自动 spawn。
2. `/code-review` 声称触发 codex-challenger 但无自动 spawn 机制——需要 Lead 手动认领。

## 二、Skills 审查（`.claude/skills/*/SKILL.md`）

| Skill | 声称触发条件 | 实际触发路径 | 缺口 |
|-------|------------|------------|------|
| core-principles | ceremony 后（所有工位） | CLAUDE.md 索引列出，agent 自行读取 | **一致**（被动加载） |
| domain-conventions | 处理缠论内容时 | agent 自行读取 | **一致** |
| domain-principles | 走势建模/分析时 | agent 自行读取 | **一致** |
| swarm-architecture | 蜂群创建/架构决策时 | agent 自行读取 | **一致** |
| project-topology | 定位文件/查 agent 职责时 | agent 自行读取 | **一致** |
| meta-orchestration | 所有 agent | agent 自行读取 | **一致** |
| orchestrator-proxy | 选择/语法记录决断时 | 声称通过 Gemini decide | **缺口**：orchestrator_proxy 在 dag 中定义了路由但无 hook 自动触发 |
| sub-swarm-ceremony | teammate 创建子蜂群时 | 递归判断块注入引用 | **一致** |
| knowledge-crystallization | 检测到稳定信号时 | crystallization-guard.sh 检测 | **部分实现** |
| spec-execution-gap | 声明与能力不匹配时 | 无自动检测 | **缺口**：无触发机制 |
| math-tools | 等价关系封闭后 | 手动引用 | **一致**（被动资料） |
| gemini-math | 形式化证明时 | 手动引用 | **一致** |
| plan-review | Plan 阶段产出方案时 | 声称 Opus 方案 + Codex 评审 | **缺口**：无自动触发 Codex 评审的机制 |
| consensus-ceremony-trigger | 质询循环收敛时 | consensus-ceremony-trigger.sh（PostToolUse/Write+Edit） | **部分实现**：hook 存在且注册，但仅检测 review-results 写入 |

### Skill 层缺口汇总

1. **orchestrator-proxy**：声称在遇到"选择/语法记录"决断时路由到 Gemini decide，但无 hook 或代码自动检测这两类决断并路由。
2. **spec-execution-gap**：声称在"声明与能力不匹配"时触发，但这正是本次审计暴露的问题——工具本身没有自动检测机制。
3. **plan-review**：声称 Plan 阶段自动触发 Codex 评审（160号），但无 hook 或 ceremony 步骤实现此触发。

## 三、ceremony_scan.py 输出字段消费审查

| 输出字段 | 有消费者? | 消费者 | 状态 |
|---------|----------|--------|------|
| `required_skills` | **无** | 应由 Lead 消费但 ceremony.md 无此步骤 | **死数据** |
| `workstations` | 有 | Lead spawn 工位 | **活跃** |
| `mode` | 有 | Lead 输出摘要 | **活跃** |
| `session` | 有 | Lead 输出摘要 | **活跃** |
| `roadmap_tasks_found` | 有 | Lead 输出摘要 | **活跃** |
| `pattern_buffer_candidates` | 有 | 生成工位到 workstations | **活跃** |
| `review_results` | 有 | 生成工位到 workstations | **活跃** |
| `topo_context` | 有 | 生成工位到 workstations | **活跃** |
| `encounter_context` | 有 | 生成工位到 workstations | **活跃** |
| `research_lines` | 有 | 生成工位到 workstations | **活跃** |
| `genealogy_anomalies` | 有 | 生成工位到 workstations | **活跃** |
| `async_self_ref` | 有 | 生成工位到 workstations | **活跃** |
| `clean_terminate` | 有 | Lead 终止判断 | **活跃** |
| `definitions` | 有 | Lead 输出摘要 | **活跃** |
| `pending` | 有 | Lead 输出摘要 | **活跃** |
| `settled` | 有 | Lead 输出摘要 | **活跃** |
| `tensions_count` | 弱 | Lead 输出摘要（如有） | **弱活跃** |
| `downstream_actions` | 有 | 生成工位到 workstations | **活跃** |
| `head` | 有 | 元数据 | **活跃** |
| `fallback_triggered` | 有 | Lead 判断 | **活跃** |
| `suspended_workstations` | 有 | 工位过滤 | **活跃** |
| `suspended_filtered_count` | 弱 | 信息性 | **弱活跃** |

### 关键发现

`required_skills` 是唯一的死数据字段。所有其他字段要么直接消费，要么转化为 workstations 间接消费。

## 四、Hooks 声称 vs 实际审查

### 已注册且工作的 hooks

| Hook | Matcher | 实际检查内容 | 声称 vs 实际 |
|------|---------|-------------|-------------|
| session-start-ceremony.sh | SessionStart | ceremony 状态初始化 | **一致** |
| agent-team-bootstrap.sh | SessionStart | 读 team-topology.json 输出提示 | **一致** |
| precompact-save.sh | PreCompact | 保存上下文到 session | **一致** |
| agent-team-enforce.sh | PreToolUse/Task | 强制使用 Agent Team（禁孤立 subagent） | **一致** |
| double-helix-verify.sh | PreToolUse/Bash | 双螺旋验证（git 操作前检查） | **一致** |
| pre-write-edit-dispatcher.sh | PreToolUse/Write+Edit | 分派到 definition/genealogy/spec/hub-node 守卫 | **一致** |
| ceremony-completion-guard.sh | Stop | 检查 ceremony 是否正常完成 | **一致** |
| post-session-pattern-detect.sh | Stop | session 结束时模式检测 | **一致** |
| meta-observer-guard.sh | Stop | 提醒 meta-observer 执行 | **一致**（但只是提醒，不 spawn） |
| function-length-guard.sh | PostToolUse/TaskUpdate | 检查函数行数 | **一致** |
| flow-continuity-guard.sh | PostToolUse/Bash | 流程连续性检查 | **一致** |
| crystallization-guard.sh | PostToolUse/Bash | pattern-buffer 候选检测 | **一致**（检测+警告，不 spawn） |
| topology-guard.sh | PostToolUse/Bash | 拓扑变更检查 | **一致** |
| post-write-edit-dispatcher.sh | PostToolUse/Write+Edit | 分派到 result-package/downstream/dag-validation/topo-mutator | **一致** |
| consensus-ceremony-trigger.sh | PostToolUse/Write+Edit | 检测 review-results 写入触发共识仪式 | **一致** |
| lead-audit.sh | PostToolUse/Bash | Lead 操作审计 | **一致** |
| completion-session-guard.sh | PostToolUse/SendMessage | shutdown 前检查 session 写入 | **一致** |

### 存在但未注册的 hooks

| Hook 文件 | 声称功能 | 状态 |
|-----------|---------|------|
| source-auditor-prompt.sh | 文档写入时提示 source-auditor 审查 | **死代码**——文件存在但未注册到 settings.json |
| ceremony-step-guard.sh | ceremony 步骤守卫 | **死代码**——文件存在但未注册到 settings.json |

### Hooks 层缺口汇总

1. `source-auditor-prompt.sh` 未注册——source-auditor agent 的文件触发路径完全断裂。
2. `ceremony-step-guard.sh` 未注册——ceremony 步骤检查无效。
3. 多个 hooks（meta-observer-guard、crystallization-guard）只做**提醒/警告**但不 spawn agent——D策略082号的"Lead 认领"步骤在 Lead 指令（ceremony.md）中无明确对应。

## 五、gangmu.yaml completion_check 审查

gangmu.yaml 中使用了以下 completion_check 类型：

| check 类型 | _check_completion 实现? | 使用次数 | 状态 |
|-----------|------------------------|---------|------|
| file_exists | 有（line 614） | 多处 | **一致** |
| script_exists | 有（与 file_exists 等同，line 614） | 1处 | **一致** |
| genealogy_settled | 有（line 617-625） | 多处 | **一致** |
| test_pass | 有（line 626-629，只检查文件存在不执行测试） | 多处 | **声明膨胀**：名为 test_pass 但实际只检查文件是否存在 |

### gangmu 缺口

1. **test_pass 语义偏差**：`_check_completion` 中 `test_pass` 类型只检查测试文件是否存在（`os.path.isfile`），不执行测试。名为"test_pass"暗示测试通过，但实际只验证文件存在。这是声明膨胀（090号）。
   - 注释中已说明"只检查测试文件存在（不执行——避免扫描阻塞）"，但 check 类型名 `test_pass` 与实际行为不符。

## 六、跨系统一致性缺口

### dispatch-dag 事件 vs 实际事件产生

dispatch-dag.yaml 的 `event_edges` 定义了大量事件触发边，但以下事件**没有 hook 或代码产生**：

| 事件 | 声称触发 | 产生者 | 状态 |
|------|---------|--------|------|
| task_complete | genealogist skill | 无——Task 完成没有 hook 产生此事件 | **无产生者** |
| file_change(spec/theorems/*) | gemini-challenger verify | 无——settings.json 中无此路径的 hook | **无产生者** |
| escalate_choice | gemini-challenger decide | 无——/escalate 写谱系但不产生此事件 | **无产生者** |
| genealogy_settlement | gemini-challenger + topology-mutator | 谱系写入到 settled/ 时无自动事件 | **无产生者** |
| test_failure | codex-challenger diagnose | 无——测试失败没有 hook 产生此事件 | **无产生者** |
| genealogy_count_threshold | topology-manager | 无——ceremony_scan 不检查此阈值 | **无产生者** |
| build_failure(>=3) | build_resolver | 无——构建失败没有计数器 | **无产生者** |
| swarm_cycle_end | meta-observer + topology-analyst | 无——蜂群循环结束没有产生此事件的 hook | **无产生者** |
| pattern_buffer_ready | skill-crystallizer | crystallization-guard.sh 检测到但不产生正式事件 | **弱产生** |

### 核心诊断

**事件驱动架构的根本性缺口**：dispatch-dag.yaml 声明了事件驱动的 skill 触发机制，但 Claude Code 平台的 hook 系统**只支持工具调用级别的事件**（PreToolUse/PostToolUse/Stop/SessionStart/PreCompact），不支持语义级别的事件（task_complete、genealogy_settlement、test_failure 等）。

这意味着 dispatch-dag.yaml 中声明的大部分事件→skill 映射是**声明层的理想设计**，但在当前 Claude Code 平台约束下无法自动执行。D策略（082号）的"Lead 手动认领"是对这一平台限制的 workaround，但 Lead 指令（ceremony.md）中缺少系统化的认领步骤。

## 七、审计结论

### 严重缺口（3个）

1. **required_skills 字段无消费者**：ceremony_scan 输出了 required_skills 但 ceremony 序列不消费
2. **事件产生链断裂**：dispatch-dag 声明的 10+ 个语义事件中 9 个无产生者
3. **source-auditor-prompt.sh 未注册**：hook 文件存在但永远不执行

### 中等缺口（5个）

4. code-verifier 从未被自动触发
5. topology-manager 阈值触发无实现
6. topology-analyst swarm_cycle_end 触发无实现
7. test_pass completion_check 语义偏差
8. orchestrator-proxy 自动路由无实现

### 设计层问题（1个）

9. D策略（082号）"Lead 认领"需要 ceremony.md 中的系统化消费步骤——当前缺失
