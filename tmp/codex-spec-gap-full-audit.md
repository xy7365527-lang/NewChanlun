# 声明-能力缺口全面审计

## 审计方法

逐一读取以下文件集合，提取所有声明性规则（"应该做X"/"不允许做Y"/"必须Z"），然后检查每条声明是否有对应的运行时强制机制（hook/ceremony_scan/skill触发条件）。

**审计覆盖范围**：
- `.claude/rules/` 全部 11 个规则文件
- `.claude/skills/` 全部 14 个 SKILL.md
- `CLAUDE.md` 中的所有原则声明
- `.chanlun/dispatch-dag.yaml` 中的行为规范
- `.claude/settings.json` 中的 hooks 注册
- `.claude/hooks/` 全部 27 个 hook 脚本

**强制机制分类**：
- **Hook 阻断（H-block）**：hook 输出 `decision: block` 阻止操作
- **Hook 警告（H-warn）**：hook 输出 `systemMessage` 提示但不阻断
- **Hook 拒绝（H-deny）**：hook 输出 `permissionDecision: deny` 拒绝工具调用
- **平台加载（P-load）**：Claude Code 平台自动加载 rules/CLAUDE.md（仅靠 LLM 内化）
- **Skill 触发（S-trig）**：skill 有明确触发条件
- **无强制（None）**：声明无任何运行时强制

---

## 缺口清单

| # | 声明位置 | 声明内容摘要 | 强制机制 | 缺口类型 | 严重度 |
|---|---------|------------|---------|---------|--------|
| 1 | result-package.md | 所有概念产出必须含六要素 | H-warn（post-write-edit-dispatcher 仅检查谱系文件三要素） | 检测范围不足 | P1 |
| 2 | formalization-validity-domain.md | 每次形式化操作必须标注认识论等级 L0-L3 | P-load（无 hook/无 scan） | 仅靠内化 | P1 |
| 3 | no-unnecessary-escalation.md | 输出必须以格式 A/B/C 结尾 | H-block（ceremony-completion-guard 检查四分法违规模式） | 部分覆盖——仅检测"确认请求"模式，不检测"无格式结尾" | P1 |
| 4 | lead-parallel-dispatch.md | 禁止串行执行独立操作 | P-load（无 hook/无 scan） | 仅靠内化——137号模式 | P2 |
| 5 | no-patch-mentality.md | 禁止补丁思维/声明膨胀等7种模式 | P-load（无 hook/无 scan） | 仅靠内化——137号模式 | P2 |
| 6 | no-workaround.md | 遇到概念矛盾不绕过，必须停下 | P-load（无 hook/无 scan） | 仅靠内化 | P2 |
| 7 | post-commit-flow.md | commit/push 后必须格式A（总结+行动一体） | H-block（flow-continuity-guard 注入继续指令） | **已覆盖** | -- |
| 8 | post-commit-flow.md | ceremony 中 push→rescan→evaluate 原子链 | H-block（flow-continuity-guard 覆盖 push+rescan） | **已覆盖** | -- |
| 9 | core-principles 原则1 | 概念优先于代码——定义不清楚时不写代码 | P-load | 仅靠内化 | P2 |
| 10 | core-principles 原则3 | 所有产出必须可质询（结果包） | H-warn（post-write-edit-dispatcher 部分检查） | 同缺口#1 | P1 |
| 11 | core-principles 原则4 | 谱系必须维护，先写谱系再汇总 | P-load | 仅靠内化 | P2 |
| 12 | core-principles 原则5 | 定义变更推荐通过仪式 | H-warn（pre-write-edit-dispatcher spec-write-guard） | **已覆盖（advisory）** | -- |
| 13 | core-principles 原则7 | 输出必须以格式 A/B/C 结尾（137号） | H-block（ceremony-completion-guard 四分法检测） | 同缺口#3 | P1 |
| 14 | swarm-architecture 原则10 | 蜂群是默认工作模式（≥2 即拉蜂群） | P-load | 仅靠内化 | P2 |
| 15 | swarm-architecture 原则15 | Task 调用必须携带 team_name | H-deny（agent-team-enforce） | **已覆盖** | -- |
| 16 | swarm-architecture 原则15 | 子蜂群必须是真递归蜂群（097号五特征） | P-load（spawn prompt 无递归指令） | **已知缺口——递归缺失** | P0 |
| 17 | swarm-architecture 原则17 | 严格性是蜂群语法规则 | P-load | 仅靠内化——meta 级声明 | P2 |
| 18 | dispatch-dag task_template | Lead 不自行执行任务 | H-warn（lead-audit 记录异常，不阻断） | 记录但不阻断 | P1 |
| 19 | dispatch-dag task_template | spawn 三基因（topo_address/depth_budget/parent_callback） | P-load（spawn prompt 无强制注入模板） | 仅靠内化 | P0 |
| 20 | dispatch-dag fractal_template | 子蜂群复制父蜂群完整结构（097号五特征） | P-load | 同缺口#16 | P0 |
| 21 | dispatch-dag validation | quality_gate_reachability：所有产出经 quality-guard | H-warn（post-write-edit-dispatcher 部分） | advisory 不保证执行 | P1 |
| 22 | dispatch-dag validation | crystallization_check：结晶检测已执行 | H-block（crystallization-guard on git commit） | **已覆盖** | -- |
| 23 | meta-orchestration | 质询序列四步（定义回溯→反例→推论→谱系比对） | P-load + S-trig（/inquire 命令触发） | 命令需手动调用，非自动 | P2 |
| 24 | meta-orchestration | 谱系写入必填字段（类型/状态/日期/前置） | H-warn（pre-write-edit-dispatcher genealogy-write-guard） | **已覆盖** | -- |
| 25 | meta-orchestration | 谱系 negation_form 必须标注 | P-load（genealogy-write-guard 不检查 negation_form） | 缺失检测 | P1 |
| 26 | meta-orchestration | 否定形式审计（每10条新谱系） | P-load（无自动计数/触发机制） | 仅靠内化 | P2 |
| 27 | knowledge-crystallization | 背驰+分型 = 结晶时机 | H-block（crystallization-guard 检查 pattern-buffer） | **已覆盖** | -- |
| 28 | domain-conventions | 级别=递归层级，禁止时间周期替代 | P-load | 仅靠内化 | P2 |
| 29 | domain-principles 原则8 | 对象否定对象——禁止超时/阈值否定 | P-load | 仅靠内化 | P2 |
| 30 | dispatch-dag ceremony | ceremony_scan.py 结果驱动工位 spawn | ceremony_scan.py（脚本实现） | **已覆盖** | -- |
| 31 | dispatch-dag event_edges | genealogy_settlement → gemini-challenger 概念层质询 | P-load（D策略：hooks 提示+Lead 认领） | advisory 不保证执行 | P1 |
| 32 | dispatch-dag event_edges | topology-mutator 否定事件触发拓扑操作 | H-warn（topology-mutator-prompt via post-write-edit-dispatcher） | advisory 不保证执行 | P1 |
| 33 | testing-override.md | TDD流程+80%覆盖率 | P-load | 仅靠内化 | P2 |
| 34 | dispatch-dag task_template | 子工位局部作用域（不修改全局 hook/谱系/定义） | P-load（无 hook 检查子工位写入范围） | 仅靠内化 | P1 |

---

## 详细诊断

### P0 缺口（结构性断裂——声明的核心能力完全缺失）

#### 缺口 #16/#20：子蜂群递归缺失

**声明**：原则15 + dispatch-dag fractal_template 声明"真递归蜂群是默认架构"，子蜂群必须满足五特征。

**断裂点**：spawn teammate 时，prompt 不包含递归指令。teammate 无从知道自己应该创建子蜂群。sub-swarm-ceremony skill 存在但从未被 spawn prompt 引用。

**强制机制**：无。仅靠 CLAUDE.md 中原则15的自然语言声明。

**已有诊断**：`tmp/codex-recursion-diagnosis.md`

**修复方案**：见下方修复执行。

#### 缺口 #19：spawn 三基因缺失

**声明**：dispatch-dag task_template 声明每个衍生节点必须携带三基因（topo_address、depth_budget、parent_callback）。

**断裂点**：spawn prompt 模板中无三基因注入模板。Lead spawn teammate 时不会自动携带三基因。

**强制机制**：dispatch-dag 声明了 `enforcement: "spawn 时 Lead 必须在 prompt 中注入"`，但无 hook 检查 Task 调用是否包含三基因。agent-team-enforce.sh 仅检查 team_name 存在性。

**修复方案**：扩展 agent-team-enforce.sh 检查 Task prompt 中是否包含三基因关键字。

### P1 缺口（声明有部分强制但存在漏洞）

#### 缺口 #1/#10：结果包六要素检测范围不足

**声明**：result-package.md 要求所有概念产出包含六要素。

**现状**：post-write-edit-dispatcher 的 result-package-guard 仅检查谱系文件（`.chanlun/genealogy/`），且仅检查三要素（边界条件、下游推论、影响声明）。不检查非谱系文件的概念产出，不检查结论、定义依据、谱系引用。

**修复方案**：result-package-guard 已覆盖关键路径（谱系文件）。非谱系概念产出（如定义文件）由 definition-write-guard 部分覆盖。当前覆盖度可接受——扩展为全概念产出检测的成本高于收益。保持现状，标注已知边界。

#### 缺口 #2：认识论等级标注无检测

**声明**：formalization-validity-domain.md 要求每次形式化操作标注 L0-L3 等级。

**现状**：无任何 hook 或 scan 检测。完全依赖 LLM 内化。

**修复方案**：此规则主要约束人类可读的分析产出（谱系、审计报告），难以用 hook 自动检测。137号模式——否定性禁令对行为执行层无效。建议在 result-package-guard 中增加对谱系文件的 L0-L3 标注检测（仅警告）。

#### 缺口 #3/#13：输出格式 A/B/C 检测不完整

**声明**：no-unnecessary-escalation.md + 原则7 要求输出以格式 A/B/C 结尾。

**现状**：ceremony-completion-guard 的四分法检测（检查5）仅匹配"确认请求"模式（待确认、是否现在处理等正则）。不检测"无格式结尾"（输出不以 A/B/C 任何一种结尾的情况）。

**修复方案**：Stop hook 无法读取 agent 的完整输出内容——`stop_hook_content` 字段的可靠性取决于平台实现。当前的正则匹配是最佳近似。保持现状。

#### 缺口 #18：Lead 直接执行记录但不阻断

**声明**：task_template 声明"Lead 不自行执行任务，只分派和汇总"。

**现状**：lead-audit.sh 记录 Lead 直接执行 Write/Edit/Bash 到 pattern-buffer 的 topo-anomalies.yaml，但不阻断。088号+173号设计决策：记录异常而非阻断（Lead 阻断导致项目级死锁——032号历史教训）。

**修复方案**：当前设计合理（记录而非阻断是经过谱系验证的设计决策）。保持现状。

#### 缺口 #25：谱系 negation_form 未检测

**声明**：meta-orchestration SKILL.md 要求每条谱系标注 negation_form。

**现状**：genealogy-write-guard 检查类型/状态/日期/前置四个字段，不检查 negation_form。

**修复方案**：在 pre-write-edit-dispatcher 的 genealogy-write-guard 中添加 negation_form 字段检测（警告级别）。

#### 缺口 #31/#32：D策略 advisory 不保证执行

**声明**：多处声明 genealogy_settlement → gemini-challenger 质询、topology-mutator 拓扑操作。

**现状**：D策略（082号）设计为 hooks 提示 + Lead 认领。advisory 提示可能被 Lead 忽略。

**修复方案**：D策略是经过 Gemini decide 确认的架构决策——不是缺口，是设计权衡（阻断 vs advisory 的平衡）。保持现状。

#### 缺口 #34：子工位写入范围无强制

**声明**：task_template 声明子工位为局部作用域。

**现状**：无 hook 检查子工位（`CLAUDE_AGENT_NAME` 存在时）是否写入了全局文件。lead-audit.sh 在 `CLAUDE_AGENT_NAME` 存在时直接 exit 0（跳过审计）。

**修复方案**：在 lead-audit.sh 中为子工位添加反向检查——子工位写入 .claude/hooks/、.claude/rules/、CLAUDE.md 时记录为拓扑异常。

### P2 缺口（仅靠自然语言内化，无运行时强制）

以下声明仅依赖 CLAUDE.md / rules 的平台加载，无 hook 或 scan 强制：

| # | 声明 | 性质 | 是否可自动化 |
|---|------|------|------------|
| 4 | 串行禁止 | 行为模式 | 难——需要分析多步操作间的依赖关系 |
| 5 | 补丁思维禁止 | 认知模式 | 不可——需要理解代码语义 |
| 6 | 矛盾不绕过 | 认知模式 | 不可——需要识别概念矛盾 |
| 9 | 概念优先代码 | 认知模式 | 不可——需要判断定义清晰度 |
| 11 | 先写谱系再汇总 | 行为顺序 | 可部分——检测 commit 中谱系文件是否在汇总之前 |
| 14 | 蜂群默认模式 | 行为模式 | 部分覆盖——agent-team-enforce 已强制 team_name |
| 17 | 严格性语法规则 | meta 级声明 | 不可——严格性是所有其他规则的前提 |
| 23 | 质询四步序列 | 流程 | /inquire 命令已存在，非强制执行 |
| 26 | 否定形式审计 | 周期任务 | 可——在 ceremony_scan 中添加谱系计数检测 |
| 28/29 | 缠论域语法 | 领域规则 | 不可——需要理解缠论语义 |
| 33 | TDD 流程 | 工程流程 | 可部分——检测 commit 中是否有测试文件 |

**结论**：P2 缺口大多是认知层面规则，无法用 hook 自动化。这些规则的有效形式是 CLAUDE.md 平台加载+137号强制输出格式（而非否定性禁令）。保持现状。

---

## 可立即修复的缺口（定理类/行动类）

### 修复 1：spawn 三基因注入 — 扩展 agent-team-enforce.sh

**缺口**：#19（P0）
**修复方式**：在 agent-team-enforce.sh 中，对非 Explore 类型的 Task 调用，检查 prompt 中是否包含三基因关键字（topo_address/depth_budget/parent_callback）。缺失时输出 systemMessage 警告（不阻断——避免032号死锁重演）。

### 修复 2：negation_form 检测 — 扩展 pre-write-edit-dispatcher

**缺口**：#25（P1）
**修复方式**：在 genealogy-write-guard 的 required 字段列表中添加 negation_form 检测。缺失时输出警告。

### 修复 3：子工位写入范围检测 — 扩展 lead-audit.sh

**缺口**：#34（P1）
**修复方式**：lead-audit.sh 当前在 `CLAUDE_AGENT_NAME` 存在时跳过。修改为：子工位写入基因组文件（.claude/hooks/、.claude/rules/、CLAUDE.md、.chanlun/dispatch-dag.yaml）时记录异常。

### 修复 4：否定形式审计周期触发 — ceremony_scan 谱系计数

**缺口**：#26（P2→P1）
**修复方式**：此修复需修改 ceremony_scan.py，涉及面较大。标记为建议修复。

---

## 需要编排者决断的缺口

### 缺口 #16/#20：子蜂群递归缺失

已有独立诊断（`tmp/codex-recursion-diagnosis.md`）。修复涉及 sub-swarm-ceremony skill 的 spawn prompt 注入机制——这是架构层变更，需要确认修复方向：
- 选项A：在 ceremony_scan.py 的 workstation prompt 模板中注入递归指令
- 选项B：在 agent-team-enforce.sh 中对 depth_budget > 0 的 spawn 注入递归引导
- 选项C：修改 sub-swarm-ceremony SKILL.md 的触发条件从"手动读取"改为 hook 自动检测

### 认识论等级标注强制化

formalization-validity-domain.md 的 L0-L3 标注当前无任何检测。是否需要：
- 在 genealogy-write-guard 中添加 L 等级检测？
- 仅对特定类型谱系（type: formalization / math-proof）强制？
