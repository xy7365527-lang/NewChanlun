# 声明-能力一致性二次审查报告（异质视角）

**审查方法**：从能力层反查声明层。Part 1 从 agent 声明出发正向追踪，本审查反向——从平台实际能力（commands、skills、hooks、ceremony_scan 输出字段）出发，检查每一项能力是否有匹配的消费者。

**与 Part 1 的关系**：Part 1 发现 17 项缺口。本审查补充以下 Part 1 未覆盖的维度：
1. Commands 声称做什么 vs 实际做什么
2. Skills 声称的触发条件 vs 实际触发路径
3. ceremony_scan.py 输出字段的消费状态
4. Hooks 声称检查什么 vs 实际检查什么
5. gangmu.yaml completion_check 类型 vs _check_completion 实现

---

## 一、Commands 声称 vs 实际

### 审查范围

13 个命令文件（`.claude/commands/*.md`）。

### 完全一致

| 命令 | 声称 | 实际 | 状态 |
|------|------|------|------|
| /ceremony | Lead 最小自举序列 | ceremony_scan.py → spawn workstations | **一致** |
| /plan | 调用 planner agent | 由 planner agent 执行 | **一致** |
| /tdd | 调用 tdd-guide agent | 由 tdd-guide agent 执行 | **一致** |
| /escalate | 生成矛盾报告写入 pending/ | 任何 agent 可调用 | **一致** |
| /inquire | 四步质询序列 | 任何 agent 可执行 | **一致** |
| /ritual | 定义广播仪式 | 任何 agent 可执行 | **一致** |
| /build-fix | 修复构建错误 | 由 build-error-resolver agent 执行 | **一致** |
| /refactor-clean | 死代码清理 | 由 refactor-cleaner agent 执行 | **一致** |
| /verify | 综合验证 | 直接执行构建/类型/测试检查 | **一致** |
| /update-docs | 文档同步 | 由 doc-updater agent 执行 | **一致** |
| /checkpoint | 创建/验证检查点 | 直接执行 git 操作 | **一致** |

### 有缺口

| 命令 | 声称 | 实际 | 缺口 |
|------|------|------|------|
| /challenge | "调用 Gemini 通过 MCP/Serena 自主导航代码库" | 执行 `python -m newchan.gemini_challenger challenge ...` | **缺口1**：前置条件声称需要 Serena MCP server 可用（`.serena/serena_config.yml`），但 gemini_challenger 实际是否依赖 Serena 需验证——settings.json 中 serena plugin 已启用，但 gemini_challenger CLI 可能独立于 Serena 运行 |
| /code-review | "触发 codex-challenger 异质审查" | 155号声称 "如果有 Python 代码变更，触发 codex-challenger" | **缺口2**：/code-review 声称自动触发 codex-challenger，但实际 spawn 机制依赖 Lead 手动执行（无 hook 自动调度）。code-review.md 第5步描述了 spawn 流程但无自动化触发路径 |

### 缺口 2 详解

`/code-review` 第 5 步声称：
> 如果有 Python 代码变更，触发 codex-challenger 的 review 模式

但这需要执行 /code-review 的 agent **主动** spawn codex-challenger。如果执行者不知道或忽略第 5 步，Codex 审查不会发生。这是 spec-execution gap（036号模式）：声明了行为但执行不受约束。

---

## 二、Skills 声称触发条件 vs 实际触发路径

### 审查范围

14 个 skill（SKILL.md），检查 `description` 中声称的触发条件是否有实际触发路径。

### 一致

| Skill | 声称触发 | 实际路径 | 状态 |
|-------|---------|---------|------|
| core-principles | ceremony 后所有工位 | CLAUDE.md 指向此 skill | **一致** |
| domain-conventions | 处理缠论内容时 | CLAUDE.md 指向此 skill | **一致** |
| domain-principles | 走势建模/分析时 | CLAUDE.md 指向此 skill | **一致** |
| swarm-architecture | 蜂群创建/架构决策时 | CLAUDE.md 指向此 skill | **一致** |
| project-topology | 定位文件/查 agent 职责时 | CLAUDE.md 指向此 skill | **一致** |
| meta-orchestration | 所有 agent | CLAUDE.md 指向此 skill | **一致** |
| sub-swarm-ceremony | teammate 创建子蜂群时 | ceremony.md 步骤 5 递归判断块指向此 skill | **一致** |
| knowledge-crystallization | 检测到稳定信号时 | crystallization-guard.sh（PostToolUse/Bash）+ ceremony_scan pattern_buffer | **一致** |
| consensus-ceremony-trigger | 质询循环收敛时 | consensus-ceremony-trigger.sh（PostToolUse/Write+Edit） | **一致** |

### 有缺口

| Skill | 声称触发 | 实际路径 | 缺口 |
|-------|---------|---------|------|
| spec-execution-gap | "声明了 X 但实际能力不匹配时触发" | **无自动触发路径** | **缺口3**：此 skill 描述了检测模式和修复模式，但无 hook、无 ceremony_scan 消费、无命令触发。完全依赖 agent 手动读取 |
| orchestrator-proxy | "选择和语法记录类决断委托给 Gemini 时激活" | **无自动触发路径**。SKILL.md 自身声明 "Task #2 负责将 decide 加入 CLI"，"在运行时就绪前，本 skill 的调用部分处于待激活状态" | **缺口4**：orchestrator-proxy decide 模式声称处理选择/语法记录类路由，但 CLI 尚未实现 `decide` 子命令。整个 decide 流程不可执行 |
| math-tools | 等价关系封闭后 | **无自动触发路径** | **缺口5**：按需读取，无 hook/scan 消费。但此 skill 是参考型（对照表），无需自动触发——标注为设计意图一致 |
| gemini-math | 形式化证明时 | **无自动触发路径** | **缺口6**：同上，参考型 skill。但 SKILL.md description 中的"形式化证明时"暗示条件触发——实际无条件检测机制 |
| plan-review | "Plan 阶段产出方案时自动激活" | **无自动触发路径** | **缺口7**：plan-review 声称"自动激活"，但无 hook 检测"Plan 阶段产出方案"事件。实际依赖使用 /plan 的 agent 主动读取此 skill 并执行多轮对审。如果 agent 不知道 plan-review skill，对审不会发生 |

### 缺口 4 详解（orchestrator-proxy decide）

这是系统中最严重的声明-能力缺口之一：
1. SKILL.md 第 102 行明确声明："截至当前，CLI 的 choices 参数尚未包含 decide"
2. 但 dispatch-dag.yaml 的 `orchestrator_proxy` section（第 585-596 行）声称 decide 已可路由
3. CLAUDE.md 的 SKILL 索引中列出 orchestrator-proxy "选择/语法记录决断时"
4. 四分法路由协议将"选择"和"语法记录"两类路由到 Gemini decide()

**结论**：四分法中 50% 的路由目标（选择/语法记录 → Gemini decide）在技术上不可执行。这不是"未来功能"——dispatch-dag 和 CLAUDE.md 声称它已在运作。

---

## 三、ceremony_scan.py 输出字段消费状态

### 审查方法

逐字段检查 ceremony_scan.py 的 JSON 输出中，哪些字段被 ceremony.md（Lead 指令）消费，哪些无消费者。

### 有消费者的字段

| 字段 | 消费者 | 消费方式 |
|------|--------|---------|
| `workstations` | ceremony.md 步骤 5 | 全部并行 spawn |
| `mode` | ceremony.md 步骤 3 | 输出摘要 |
| `session` | ceremony.md 步骤 3 | 输出摘要 |
| `settled` | ceremony.md 步骤 3 | 输出摘要 |
| `pending` | ceremony.md 步骤 3 | 输出摘要 |
| `clean_terminate` | ceremony.md 步骤 2/10 | 终止判断 |
| `head` | ceremony.md | commit 引用 |

### 无消费者的字段（死数据）

| 字段 | 产出函数 | 消费者 | 状态 |
|------|---------|--------|------|
| `required_skills` | `get_required_skills()` | **无** | **缺口8**（Part 1 已识别） |
| `pattern_buffer_candidates` | `get_pattern_buffer_candidate_count()` | **间接**：转化为 workstation | 半消费 |
| `review_results` | `get_review_results()` | **间接**：conflict/warning → workstation | 半消费 |
| `topo_context` | `get_topo_context()` | **间接**：unmapped → workstation | 半消费 |
| `encounter_context` | `get_encounter_context()` | **间接**：candidates → workstation | 半消费 |
| `research_lines` | `_scan_research_lines()` | **间接**：unblocked actions → workstation | 半消费：proposed_transitions 无消费者 |
| `genealogy_anomalies` | `detect_genealogy_anomalies()` | **间接**：anomalies → workstation | 半消费 |
| `async_self_ref` | async_self_reference 模块 | **间接**：findings → workstation | 半消费 |
| `tensions_count` | 谱系张力扫描 | **无** | **缺口9**：不转化为 workstation |
| `downstream_actions` | `downstream_audit` | **间接**：unresolved → workstation | 半消费 |
| `definitions` | definitions.yaml 读取 | **无** | **缺口10**：仅信息输出 |
| `roadmap_tasks_found` | roadmap 计数 | **无** | 信息字段 |
| `fallback_triggered` | fallback 标志 | **无** | 信息字段 |
| `suspended_workstations` | suspended 过滤 | **间接**：过滤 workstations | 有消费 |
| `suspended_filtered_count` | 过滤计数 | **无** | 信息字段 |

### 特别关注：proposed_transitions

`research_lines.proposed_transitions` 提出状态转换提议（active→blocked、active→closed），但无消费者执行这些转换。gangmu.yaml 的状态更新依赖 gangmu_update.py 手动调用。

**缺口11**：proposed_transitions 是诊断信号但无自动执行路径。

---

## 四、Hooks 声称检查什么 vs 实际检查什么

### 已注册 PostToolUse hooks 深度审查

| Hook | Matcher | 声称 | 实际行为 | 状态 |
|------|---------|------|---------|------|
| crystallization-guard.sh | Bash | git commit 时检查结晶债务 | 拦截 `git commit`，检查 `.crystallization-debt.json` 和 pattern-buffer freq>=3 | **一致** |
| post-write-edit-dispatcher.sh | Write/Edit | 5合1调度器 | result-package + downstream-action + dag-validation + topo-mutator-prompt + lead-audit | **一致** |
| consensus-ceremony-trigger.sh | Write/Edit | 质询收敛检测 | 检测 `.chanlun/review-results/*.md` 写入中的收敛信号 | **一致** |
| completion-session-guard.sh | SendMessage | shutdown 前 session 检查 | shutdown_request 时检查 `.last-session-append` 标记 | **一致** |

### 已注册 Stop hooks 深度审查

| Hook | 声称 | 实际行为 | 状态 |
|------|------|---------|------|
| meta-observer-guard.sh | 二阶反馈强制 | 检查 `.meta-observer-executed`，STRICT=1 阻断一次 | **缺口14**：注释说默认 advisory，代码默认 STRICT=1 |

### 未注册但存在的 Hooks

| Hook | 声称 | settings.json | 缺口 |
|------|------|----|------|
| source-auditor-prompt.sh | docs/ 写入时提示 | **未注册** | **缺口12**（Part 1 已识别） |
| ceremony-step-guard.sh | ceremony 步骤检查 | **已废弃** | 非缺口（文件标注已移除） |
| topology-mutator-prompt.sh | topo_effect 提示 | **未注册** | **缺口13**：功能已内联 dispatcher，但独立文件未标注废弃 |

---

## 五、gangmu.yaml completion_check 类型 vs _check_completion 实现

### gangmu.yaml 中使用的类型

| type | 使用次数 | 示例 |
|------|---------|------|
| `file_exists` | 5 | path: scripts/position_manager.py |
| `script_exists` | 2 | path: deploy/k4-monitor/setup.sh |
| `test_pass` | 6 | pattern: tests/test_xiaozhuan_da_integration.py |
| `genealogy_settled` | 5 | keyword: fugue-state-machine |

### _check_completion 支持的类型

| type | 实际行为 | 一致？ |
|------|---------|--------|
| `file_exists` | `os.path.isfile(root/path)` | **一致** |
| `script_exists` | 等同于 file_exists | **一致** |
| `test_pass` | `os.path.isfile(root/pattern)` — 只检查文件存在，**不执行测试** | **缺口15** |
| `genealogy_settled` | settled/ 目录中搜索 keyword | **一致** |

### 缺口 15 详解（test_pass 语义膨胀）

- **声称**：`test_pass` 暗示"测试通过"
- **实际**：仅检查测试文件是否存在（代码注释："只检查测试文件存在（不执行——避免扫描阻塞）"）
- **影响**：测试文件存在但全部失败时，`_check_completion` 仍返回 True
- 这是声明膨胀（090号）：type 名称声称了代码不具备的能力

---

## 六、新发现汇总（Part 2 独有）

| 编号 | 缺口 | 严重度 | 来源 | 修复状态 |
|------|------|--------|------|----------|
| 缺口1 | /challenge 前置条件声称需 Serena，实际待验证 | 低 | Commands | **已修复**：challenge.md 补充 Serena 使用说明（MCP工具层，非直接依赖） |
| 缺口2 | /code-review 声称自动触发 codex-challenger，实际需手动 | 中 | Commands | **已修复**：code-review.md 补充 D策略手动认领标注 |
| 缺口3 | spec-execution-gap skill 无自动触发 | 低 | Skills | **无需修复**：参考型 skill，手动读取是设计意图 |
| **缺口4** | **orchestrator-proxy decide 不可执行** | **高** | **Skills** | **已修复**（v157-swarm 353号）：SKILL.md 补充当前限制标注 + dispatch-dag platform_support:false |
| 缺口5 | math-tools 无自动触发（设计意图一致） | 无 | Skills | **无需修复** |
| 缺口6 | gemini-math 无自动触发 | 低 | Skills | **无需修复**：参考型 skill，手动触发是设计意图 |
| **缺口7** | **plan-review 声称自动激活无触发路径** | **中** | **Skills** | **已修复**：plan.md 补充 plan-review skill 加载步骤 + D策略标注 |
| 缺口8 | required_skills 无消费者（Part 1 已知） | 高 | scan | **已修复**（v157-swarm）：ceremony.md 步骤5b 消费 spawn_condition |
| **缺口9** | **tensions_count 死数据** | **中** | **scan** | **已修复**：tensions_count 转化为 workstation（tension_scan source） |
| 缺口10 | definitions 仅信息输出 | 低 | scan | **无需修复**：信息字段，由 ceremony 摘要步骤3消费 |
| **缺口11** | **proposed_transitions 无执行路径** | **中** | **scan** | **已修复**：proposed_transitions 转化为 workstation（gangmu_transition source） |
| 缺口12 | source-auditor hook 未注册（Part 1 已知） | 中 | Hooks | **已修复**（v157-swarm）：settings.json 已注册 |
| 缺口13 | topology-mutator-prompt 与 dispatcher 重复未标注 | 低 | Hooks | **已修复**：文件标注废弃（功能已内联 dispatcher） |
| **缺口14** | **meta-observer-guard 注释与代码不一致** | **低** | **Hooks** | **已修复**：注释对齐代码默认值（STRICT=1） |
| **缺口15** | **test_pass 只检查文件存在（声明膨胀）** | **中** | **gangmu** | **已修复**（v157-swarm）：gangmu.yaml 全量迁移到 test_file_exists |

---

## 七、系统性模式归纳

### 模式 A：声明了自动触发但实际需手动（7 个缺口）

缺口 2, 4, 7, 8, 12 + Part 1 的 code-verifier/topology-manager/topology-analyst 等。

**根因**：dispatch-dag.yaml 声明了事件→skill 映射，但 Claude Code 平台只支持工具级 hook 事件（PreToolUse/PostToolUse），不支持语义级事件（task_complete/genealogy_settlement 等）。

### 模式 B：产出了数据但无消费者（4 个缺口）

缺口 8, 9, 10, 11。

**根因**：ceremony_scan.py 输出字段持续增长，消费端（ceremony.md）不跟踪新增字段。生产者和消费者演化速度不同步。

### 模式 C：名称/注释声称超出实际能力（4 个缺口）

缺口 1, 14, 15 + orchestrator-proxy SKILL.md 声称 decide 可路由。

**根因**：命名和注释在设计阶段写入，实现阶段变更后未同步更新。
