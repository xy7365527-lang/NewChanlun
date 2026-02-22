# CLAUDE.md 原则声明-能力审计报告

## 审计方法

逐条审计 CLAUDE.md 中的所有原则（0-17）、五约束有向依赖图、递归节点默认行为、热启动机制等声明，检查每条是否有对应的：
- Hook（`.claude/hooks/`，已注册于 `.claude/settings.json`）
- Skill（`.claude/skills/`）
- Agent（`.claude/agents/`）
- 代码实现（`scripts/`、`src/`）
- dispatch-dag.yaml 中的事件映射
- Rules（`.claude/rules/`）
- Commands（`.claude/commands/`）

审计基准：声明的能力是否有**运行时可执行的机制**支撑，而非仅靠 LLM 上下文中的文本指令。

---

## 逐条审计

### 原则0：蜂群能修改一切，包括自身

- **声明**：蜂群可以修改拓扑、行为定义、免疫系统、启动协议、元编排规则、谱系、定义、仪式、核心禁令。没有任何文件不可修改。安全保障靠 git + ESC + 产出物验证。
- **实际能力**：
  - `settings.json` 的 `permissions.allow` 包含 Read/Edit/Write/Bash 等全部工具，`deny` 只限 `rm -rf *` 等极端操作
  - `spec-write-guard.sh`（PreToolUse）：对核心文件修改输出 advisory 但 allow（不阻断）
  - `definition-write-guard.sh`：定义文件写入验证 + 熔断放行机制
  - `genealogy-write-guard.sh`：谱系写入验证 + allow（不阻断）
  - git 追踪：所有修改可 diff/revert
- **状态**：✅一致
- **备注**：声明"安全靠 git + ESC + 产出物验证"——git 已实现，ESC 是人类操作（平台级），产出物验证通过 hooks 实现。三者均到位。

---

### 原则1：概念优先于代码

- **声明**：定义不清楚时不写代码。
- **实际能力**：
  - 无专门 hook 或代码强制
  - 依赖 LLM 上下文中的 CLAUDE.md 文本指令
  - `definition-write-guard.sh` 间接支持（强制定义文件有 status/version）
- **状态**：⚠️部分实现
- **缺口描述**：这是一个设计原则/纪律声明，而非可代码强制的规则。没有 hook 检测"代码写入时是否有对应定义"。但考虑到这是方法论层面的指导而非可机械化检查的规则，这种依赖 LLM 自律的方式是合理的设计选择。

---

### 原则2：不绕过矛盾

- **声明**：见 `no-workaround.md`。
- **实际能力**：
  - `.claude/rules/no-workaround.md`：Claude Code 平台强制加载到每次对话
  - `/escalate` 命令（`.claude/commands/escalate.md`）：矛盾上浮流程
  - dispatch-dag `event_edges` 中 `/escalate` → genealogist 映射
  - `genealogy-write-guard.sh`：谱系写入时检查强制字段
- **状态**：✅一致
- **备注**：规则通过 Claude Code 的 rules 自动加载机制强制，加上 /escalate 命令提供操作路径。

---

### 原则3：所有产出必须可质询

- **声明**：见 `result-package.md`。
- **实际能力**：
  - `.claude/rules/result-package.md`：六要素格式规则
  - `result-package-guard.sh`（PostToolUse）：谱系写入后检查边界条件/下游推论/影响声明
  - `/inquire` 命令：四步质询序列
- **状态**：✅一致
- **备注**：result-package-guard 只检查谱系文件（`.chanlun/genealogy/**`），不检查其他产出（代码、分析等）。但这些非谱系产出的可质询性依赖 LLM 自律 + rules 加载，符合设计意图。

---

### 原则4：谱系必须维护，且谱系优先于汇总

- **声明**：每次矛盾处理后写入谱系。先写谱系再汇总。
- **实际能力**：
  - `genealogy-write-guard.sh`：谱系写入时验证强制字段（类型/状态/日期/前置）+ 前置引用验证 + 语义一致性检查
  - `result-package-guard.sh`：谱系文件结果包字段检查
  - `downstream-action-guard.sh`：下游推论追踪
  - `genealogy-gemini-verify.sh`：谱系写入后建议 Gemini 异质审查
  - `genealogist.md` agent
  - 141 个已结算谱系（实际运行中）
- **状态**：✅一致
- **备注**："先写谱系再汇总"的顺序没有 hook 强制。但谱系基础设施（写入验证、字段检查、语义检查、审查建议）非常完整。

---

### 原则5：定义变更推荐通过仪式

- **声明**：使用 `/ritual` 是推荐流程，不是强制门控。
- **实际能力**：
  - `/ritual` 命令（`.claude/commands/ritual.md`）存在
  - `definition-write-guard.sh`：定义写入时验证格式（allow + 警告，不阻止）
  - 明确声明"不是强制门控"——设计意图就是推荐
- **状态**：✅一致

---

### 原则6：推论自动结算（四分法）

- **声明**：系统产出分为定理/选择/语法记录/行动四类，处理方式不同。
- **实际能力**：
  - `.claude/rules/no-unnecessary-escalation.md`：四分法自检序列
  - `orchestrator-proxy` skill：选择/语法记录路由到 Gemini decide
  - dispatch-dag `orchestrator_proxy`：定义了选择/语法记录的路由
  - 无自动分类 hook
- **状态**：⚠️部分实现
- **缺口描述**：四分法分类依赖 LLM 自行判断（通过 rules 加载的文本指令）。没有自动分类器。"定理自动结算"和"行动自动执行"在运行时全靠 LLM 自律。选择/语法记录路由到 Gemini 的路径存在（orchestrator-proxy skill），但触发也是 LLM 判断。这是 LLM agent 系统的固有特性——分类本身需要语义理解，难以用 hook 机械化。

---

### 原则7：ceremony 是 Swarm₀

- **声明**：ceremony 加载初始区分后直接递归进入工作。commit/push 后下一个输出必须是 `→ 接下来：[具体动作]`。不允许总结段落或等待信号。
- **实际能力**：
  - `session-start-ceremony.sh`（SessionStart hook）：自动检测冷/热启动，注入 ceremony 状态
  - `/ceremony` 命令
  - `ceremony-completion-guard.sh`（Stop hook）：检测死寂状态、pending 谱系、活跃任务等
  - `flow-continuity-guard.sh`（PostToolUse on Bash/git commit）：commit 后注入"不允许停顿"指令
  - `.claude/rules/post-commit-flow.md`：commit 后默认动作规则
  - `scripts/ceremony_scan.py`：蜂群 spawn 通用工具
- **状态**：✅一致
- **备注**：ceremony 的各个环节都有 hook 支撑：自动启动（SessionStart）、持续推进（flow-continuity-guard）、停机阻断（ceremony-completion-guard）。

---

### 原则8：对象否定对象

- **声明**：体系中一个对象被否定的唯一来源是内在否定或外部对象生成。不允许超时、阈值、或非对象来源的否定。
- **实际能力**：
  - 这是**走势描述语言的语法规则**，不是工程流程
  - 代码实现在 `src/newchan/` 的各模块中（背驰检测、分型识别等不使用超时/阈值）
  - 无专门 hook 检查"代码中是否引入了超时否定"
- **状态**：⚠️部分实现
- **缺口描述**：作为走势描述语言的语法规则，其执行体现在算法实现中（如背驰检测用区间套而非阈值）。没有 hook 自动扫描代码中是否存在违反此原则的模式（如 time-based timeout negation）。但这是概念层约束，代码审查（code-reviewer agent）间接覆盖。

---

### 原则9：热启动保障蜂群持续自动化

- **声明**：compact 发生时通过 session 记录和 ceremony 恢复蜂群状态。
- **实际能力**：
  - `precompact-save.sh`（PreCompact hook）：compact 前自动保存 session（定义状态/谱系状态/git 状态/中断点）
  - `session-start-ceremony.sh`（SessionStart hook）：新会话自动检测最新 session，注入差异报告
  - `.chanlun/sessions/` 目录：实际存在多个 session 文件
  - L0/L1/L2 三级恢复均有对应实现
- **状态**：✅一致
- **备注**：热启动是本系统最完整的实现之一。PreCompact 保存 → SessionStart 恢复的链路完整。

---

### 原则10：蜂群是默认工作模式

- **声明**：每个工作节点必须先评估可并行的独立工位数（≥2 即拉蜂群）。单线程顺序执行只在任务间有严格依赖时才允许。
- **实际能力**：
  - `agent-team-enforce.sh`（PreToolUse on Task）：强制所有 Task 调用必须通过 Agent Team，无例外
  - `.claude/rules/common/agents.md`：并行是默认模式的规则声明
  - `ceremony-completion-guard.sh`（Stop hook）：检测"有活干但没人在干"的死寂状态 → 阻断
  - dispatch-dag `event_edges`：`independent_tasks >= 2` → `spawn_recursive_swarm`
- **状态**：✅一致
- **备注**：agent-team-enforce.sh 是最严格的 hook 之一（block，无例外）。结合 ceremony-completion-guard 的死寂检测，形成"必须并行"的强制链。

---

### 原则11：上下文中的知识有走势结构

- **声明**：知识在 prompt 中的生命周期有走势结构（扩张/收缩/结晶/重新加载）。结晶的三个维度：session/skill/definitions。
- **实际能力**：
  - `crystallization-guard.sh`（PostToolUse on Bash/git commit）：检测未结晶的稳定模式
  - `post-session-pattern-detect.sh`（Stop hook）：session 结束时检测 tool 调用模式，写入 pattern-buffer
  - `knowledge-crystallization` skill
  - `skill-crystallizer.md` agent
  - `.chanlun/pattern-buffer.yaml`：实际存在并运行中
  - `precompact-save.sh`：session 维度结晶
  - `definition-write-guard.sh`：definitions 维度守卫
- **状态**：✅一致
- **备注**：结晶三维度（session/skill/definitions）都有对应机制。pattern-buffer 实际运行中（有 observed/candidate/promoted 条目）。

---

### 原则12：编排者代理（Gemini Proxy）

- **声明**：人类编排者从同步决策者变为异步审计者。选择/语法记录路由到 Gemini decide。人类保留 INTERRUPT 权。Gemini 不可用时写入 pending。
- **实际能力**：
  - `orchestrator-proxy` skill（`.claude/skills/orchestrator-proxy/SKILL.md`）
  - `gemini-challenger.md` agent
  - `src/newchan/gemini/modes.py`：Gemini decide 模式实现
  - dispatch-dag `orchestrator_proxy` 配置
  - `double-helix-verify.sh`：git commit 前调用 Gemini verify，不可达时降级放行
  - `genealogy-gemini-verify.sh`：谱系写入后建议 Gemini 审查
  - Gemini API key 通过环境变量传入
- **状态**：✅一致
- **备注**：Gemini 集成是系统中最完整的外部依赖。decide/verify/challenge 三种模式都有代码实现。降级路径（不可达时放行/pending）也有。

---

### 原则13：缠论空间是偏序集/有向图

- **声明**：走势建模为 DAG。浮点数阈值判断"背驰"是非法的。
- **实际能力**：
  - dispatch-dag.yaml `definition_partial_order`：定义偏序链
  - `src/newchan/` 代码实现：背驰检测使用区间套/面积比较而非浮点阈值
  - 无专门 hook 扫描"代码中是否使用了浮点阈值判断背驰"
- **状态**：⚠️部分实现
- **缺口描述**：定义偏序已显式声明（dispatch-dag `definition_partial_order`）。代码实现层面遵循原则。但没有自动化检测机制来防止未来代码引入浮点阈值判断。与原则8类似，这是概念层约束，靠代码审查覆盖。

---

### 原则14：合法/非法替代概率/胜率

- **声明**：走势描述语言中不存在"概率"概念。一个走势描述要么合法要么非法。
- **实际能力**：
  - 这是**走势描述语言的语法规则**
  - 代码中：买卖点判断返回布尔值（合法/非法），不返回概率
  - 无 hook 扫描"代码中是否引入了概率/胜率概念"
- **状态**：⚠️部分实现
- **缺口描述**：同原则8/13。概念层约束在代码实现中遵循，但无自动化防止违规的机制。

---

### 原则15：拓扑异步自指蜂群是默认架构

- **声明**：当前实现为扁平 Trampoline。设计意图为子蜂群嵌套。两个不可消除的 Gap。
- **实际能力**：
  - `agent-team-enforce.sh`：强制 Agent Team
  - `lead-audit.sh`（PostToolUse）：Lead 直接执行时生成拓扑异常对象（settings.local.json）
  - `sub-swarm-ceremony` skill：子蜂群创建流程结晶
  - `meta-observer-guard.sh`（Stop hook）：二阶反馈（异步自指的 t+1 审查）
  - `topology-guard.sh`（PostToolUse on Bash/git commit）：孤岛检测
  - dispatch-dag `fractal_template`：子蜂群分形模板
  - 19 个 agent 文件（meta-lead/genealogist/quality-guard/meta-observer 等）
  - **平台约束**：Claude Code 不支持 subagent spawn sub-subagent（073b号）→ Trampoline 模式
- **状态**：⚠️部分实现（平台约束）
- **缺口描述**：声明中明确标注"当前实现为 Trampoline，设计意图为真递归"。实际能力与声明一致——Trampoline 已实现，真递归受平台限制。这是**已知的平台约束**（073b号谱系），声明与能力诚实对齐。

---

### 原则16：理论洞见影响设计决策，代码自足表达

- **声明**：禁止在 `src/` 中出现哲学/精神分析命名。
- **实际能力**：
  - 无专门 hook 扫描 `src/` 中的命名
  - 实际代码文件检查：`src/newchan/` 中无 `lacan_router.py` 等命名（`gemini_challenger.py` 是工程命名）
  - 代码审查（code-reviewer agent）间接覆盖
- **状态**：⚠️部分实现
- **缺口描述**：当前依赖代码审查。可以添加一个 PostToolUse hook 在 `src/` 文件写入后扫描是否包含 lacan/sinthome 等哲学术语命名。但这是低风险缺口——命名约定靠团队纪律已足够。

---

### 原则17：严格性是蜂群的语法规则

- **声明**：所有蜂群产出必须严格。非严格的产出在蜂群中是语法不合法的。
- **实际能力**：
  - `.claude/rules/no-patch-mentality.md`：Claude Code 平台强制加载
  - `result-package-guard.sh`：检查产出完整性（间接执行严格性）
  - `quality-guard.md` agent：质量守卫
- **状态**：✅一致
- **备注**：严格性作为语法规则，通过 rules 自动加载 + quality-guard 验证。本质上是纪律性约束，通过文本指令强制是合理方式。

---

### 五约束有向依赖图（093号谱系）

#### 约束1a：物理持久化

- **声明**：agent 临时、上下文有限 → 需要物理持久化。
- **实际能力**：
  - `precompact-save.sh`：compact 前保存 session
  - `.chanlun/sessions/`：session 文件
  - `.chanlun/genealogy/`：谱系持久化
  - `.chanlun/definitions/`：定义持久化
  - `.chanlun/pattern-buffer.yaml`：模式持久化
  - git：所有文件版本控制
- **状态**：✅一致

#### 约束1b：符号可解释性

- **声明**：跨实例无共享记忆 → 符号必须自解释。
- **实际能力**：
  - 定义文件强制 status/version 字段（`definition-write-guard.sh`）
  - 谱系文件强制 类型/状态/日期/前置 字段（`genealogy-write-guard.sh`）
  - dispatch-dag.yaml 本身是声明式自解释格式
  - manifest.yaml 注册所有组件
- **状态**：✅一致

#### 约束2：规则先在性

- **声明**：spawn 在执行前完成 → 规则必须在 agent 启动前加载。
- **实际能力**：
  - `session-start-ceremony.sh`（SessionStart hook）：会话开始时自动加载
  - CLAUDE.md + `.claude/rules/`：Claude Code 平台在每次对话开始时强制加载
  - `scripts/ceremony_scan.py`：ceremony 扫描定义/谱系/skill 状态
- **状态**：✅一致

#### 约束3：执行不可自观性

- **声明**：token 单向生成 → 无法即时自修正。
- **实际能力**：
  - `meta-observer-guard.sh`（Stop hook）：session 结束时强制/建议二阶观察
  - `meta-observer.md` agent：二阶反馈
  - dispatch-dag `meta-observer → meta-observer` 自环
  - **异步自指的工程实现**：t 时刻审查 t-1 时刻的产出
- **状态**：✅一致

#### 约束4：异质验证必要性

- **声明**：同质系统盲点不可自检 → 需要异质验证。
- **实际能力**：
  - `double-helix-verify.sh`：git commit 前 Gemini 验证
  - `genealogy-gemini-verify.sh`：谱系写入后建议 Gemini 审查
  - `gemini-challenger.md` agent
  - `claude-challenger.md` agent（反向质询）
  - `src/newchan/gemini/`：Gemini API 集成
- **状态**：✅一致

---

### 递归节点默认行为（058号谱系）

#### 无阻碍执行（默认递归）

- **声明**：基于 dispatch-dag 的工位 spawn/shutdown、基于 topology 的任务路由、语法/编译/Lint 级错误的修复流程启动、quality-guard 通过后的 commit、"选择"类决断路由 Gemini。
- **实际能力**：
  - dispatch-dag 定义完整的 event_skill_map + ceremony_sequence
  - `agent-team-enforce.sh`：强制 Agent Team
  - `flow-continuity-guard.sh`：commit 后强制继续
  - `ceremony-completion-guard.sh`：死寂状态检测
  - `orchestrator-proxy` skill：选择路由到 Gemini
- **状态**：✅一致

#### 必须阻断等待（020号反转条件）

- **声明**：逻辑/断言级测试失败、dispatch-dag 未定义的异常、修改 CLAUDE.md/核心定义/已结算谱系、unclassified 否定、需要缠论领域知识的选择。
- **实际能力**：
  - `spec-write-guard.sh`：核心文件修改检测 + advisory
  - `hub-node-impact-guard.sh`：hub 节点谱系修改影响链警告
  - `definition-write-guard.sh`：定义写入验证
  - 无 hook 专门检测"unclassified 否定"
  - 无 hook 专门检测"逻辑/断言级测试失败是否为概念分离信号"
- **状态**：⚠️部分实现
- **缺口描述**：核心文件修改有 advisory（spec-write-guard + hub-node-impact-guard），但声明中说"修改 CLAUDE.md/核心定义/已结算谱系"应触发**阻断等待**（020号反转）。实际实现是 advisory（allow + 警告），不是 block。设计理由见原则0（蜂群能修改一切），但与"必须阻断等待"的声明存在**偏差**。

---

### 热启动机制

#### L0 正常

- **声明**：上下文充足时蜂群持续运行。
- **实际能力**：正常对话流程，无需特殊机制。
- **状态**：✅一致

#### L1 compact

- **声明**：PreCompact 保存 + SessionStart 恢复。
- **实际能力**：
  - `precompact-save.sh`（PreCompact hook）：已注册并运行
  - `session-start-ceremony.sh`（SessionStart hook）：已注册并运行
- **状态**：✅一致

#### L2 新对话

- **声明**：新对话执行 `/ceremony`，自动检测 session 记录，切换为热启动模式。
- **实际能力**：
  - `session-start-ceremony.sh` 自动检测冷/热启动
  - `/ceremony` 命令存在
  - 版本对比/差异报告/跳过未变更项的逻辑已实现于 hook
- **状态**：✅一致

---

### 知识仓库映射

- **声明**：速查定义→`缠论知识库.md`，域对象 schema→`definitions.yaml`，规则规范→`docs/spec/`，原文参考→`docs/chanlun/text/chan99/`
- **实际能力**：
  - `.chanlun/definitions/`：13 个定义文件（baohan/fenxing/bi/xianduan/zhongshu/zoushi/beichi/maimai 等）
  - `docs/` 目录存在
  - `缠论知识库.md` 存在
- **状态**：✅一致

---

### 分布式指令架构（014号谱系）

- **声明**：元编排指令分布在 SKILL.md/agents/commands/hooks 中。
- **实际能力**：
  - 19 个 agent 文件
  - 21 个 hook 脚本（全部注册于 settings.json）
  - 13 个 command 文件
  - 7 个 skill 目录
  - dispatch-dag.yaml 作为中心拓扑定义
  - manifest.yaml v2.0 注册所有组件
- **状态**：✅一致

---

### 可用命令

| 命令 | 声明 | 实现文件 | 状态 |
|------|------|----------|------|
| `/ceremony` | Swarm₀ 启动 | `.claude/commands/ceremony.md` | ✅ |
| `/inquire` | 四步质询序列 | `.claude/commands/inquire.md` | ✅ |
| `/escalate` | 矛盾上浮 | `.claude/commands/escalate.md` | ✅ |
| `/ritual` | 定义广播仪式 | `.claude/commands/ritual.md` | ✅ |
| `/plan` | 实现规划 | `.claude/commands/plan.md` | ✅ |
| `/tdd` | 测试驱动开发 | `.claude/commands/tdd.md` | ✅ |
| `/code-review` | 代码审查 | `.claude/commands/code-review.md` | ✅ |

---

### 结构修改模式（069号谱系）

- **声明**：meta-observer 发现矛盾 → draft-proposal → gemini-challenger 审查 → 直接执行。
- **实际能力**：
  - `meta-observer.md` agent
  - `gemini-challenger.md` agent
  - dispatch-dag edges：`meta-observer → gemini-challenger`（proposal_review）
  - `spec-write-guard.sh`：核心文件修改 advisory
  - 无自动化的"proposal → review → merge"流水线
- **状态**：⚠️部分实现
- **缺口描述**：组件都存在（meta-observer/gemini-challenger/dispatch-dag 边），但没有自动化的端到端流水线。提案-审查-合并的链路依赖 LLM 按 agent 指令手动执行。

---

### 自动化运行时基础设施

#### pattern_detection

- **声明**：session_end 时检测模式。
- **实际能力**：`post-session-pattern-detect.sh`（Stop hook）已注册。
- **状态**：✅一致

#### crystallization

- **声明**：auto_trigger + require_genealogy_settlement。
- **实际能力**：`crystallization-guard.sh`（PostToolUse on Bash/git commit）。pattern-buffer 中有 promoted 条目（实际运行过）。
- **状态**：✅一致

#### manifest

- **声明**：auto_register。
- **实际能力**：`topology-guard.sh` 中的 manifest 注册检查。manifest.yaml v2.0 存在。
- **状态**：✅一致

---

## 汇总

| 状态 | 数量 |
|------|------|
| ✅一致 | 27 |
| ⚠️部分实现 | 8 |
| ❌纯声明缺口 | 0 |
| 🔧平台约束 | 1（原则15 子蜂群嵌套，073b号） |

---

## ⚠️部分实现清单（按严重性排序）

### 1. 020号反转条件的阻断行为 vs 实际 advisory（偏差）

- **原则**：递归节点默认行为——"修改 CLAUDE.md/核心定义/已结算谱系"声明为"必须阻断等待"
- **实际**：`spec-write-guard.sh` 对核心文件修改输出 advisory（allow），不是 block
- **分析**：这是设计决策而非遗漏——原则0（蜂群能修改一切）与020号反转条件之间存在张力。当前实现选择了原则0优先（allow + 记录），牺牲了020号的阻断语义。
- **建议**：如果要严格执行020号，需要将 spec-write-guard 对 CLAUDE.md/已结算谱系的修改改为 block（保留熔断）。或者在 CLAUDE.md 中修正声明，明确"advisory 而非 block"。

### 2. 结构修改模式的端到端流水线（缺口）

- **原则**：meta-observer → draft-proposal → gemini-challenger → 执行
- **实际**：组件存在但无自动化管道
- **建议**：低优先级——当前靠 LLM 按 agent 指令执行已足够。真正的自动化管道需要平台层支持。

### 3. 原则1（概念优先于代码）无运行时强制（设计选择）

- **实际**：依赖 LLM 自律
- **建议**：这是方法论指导，不适合机械化。保持现状。

### 4. 原则8/13/14（走势语法规则）无代码扫描（设计选择）

- **实际**：概念层约束，靠代码审查覆盖
- **建议**：可选：添加 lint 规则扫描 src/ 中的 `probability`/`timeout` 等关键词。低优先级。

### 5. 原则16（禁止哲学命名）无自动扫描（设计选择）

- **实际**：靠代码审查覆盖，实际代码中无违规
- **建议**：低优先级。

### 6. 四分法无自动分类器（原则6）

- **实际**：分类依赖 LLM 语义理解
- **建议**：这是 LLM agent 系统的固有特性，无法用 hook 机械化。保持现状。

### 7. "先写谱系再汇总"顺序无强制（原则4）

- **实际**：谱系基础设施完整，但写入顺序不强制
- **建议**：低优先级。可通过 flow-continuity-guard 添加"commit 中有非谱系变更但无谱系变更"的检测。

### 8. 原则15 子蜂群嵌套受平台约束（073b号）

- **实际**：Trampoline 模式已实现，真递归受 Claude Code 限制
- **建议**：等待平台能力升级。声明已诚实标注。

---

## 总体评价

系统的声明-能力对齐度非常高。21 个 hook 全部注册于 settings.json 并实际运行，覆盖了 PreToolUse/PostToolUse/Stop/SessionStart/PreCompact 全部 hook 类型。0 个纯声明缺口（每个声明都有某种形式的实现支撑）。8 个"部分实现"中，大部分是**设计选择**（概念层约束难以机械化）或**已知平台约束**（073b号），而非遗漏。

唯一的**偏差**是020号反转条件声明的"必须阻断等待"与实际实现的"advisory"之间的不一致——这值得在 CLAUDE.md 或 hook 中对齐。
