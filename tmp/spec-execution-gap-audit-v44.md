# 声明-实现缺口全量审计 v44

> 审计时间：2026-02-23
> 审计范围：递归总方针 + 174号谱系 + 行为原则 + 三步战略 + dispatch-dag.yaml + 163-174号谱系
> 方法：逐条验证声明与实际状态的一致性

---

## 缺口清单

| # | 来源 | 声明内容 | 实际状态 | 严重性 | 修复方案 |
|---|------|---------|---------|--------|---------|
| 1 | 递归总方针 L37 | "topology_operator.py 已实现（35个测试全绿）" | **属实**。pytest tests/test_topology_operator.py → 35 passed in 0.04s | -- | 无需修复 |
| 2 | 递归总方针 L37 | "dispatch-dag topology-mutator skill 已注册" | **属实**。dispatch-dag.yaml L279 有 topology-mutator skill 定义，含 triggers/condition/negation_topo_map | -- | 无需修复 |
| 3 | 递归总方针 L43 | "异步自指机制已有（Gemini challenge + Codex review + claude-challenger）" | **部分属实**。三个 agent 文件存在（gemini-challenger.md, codex-challenger.md, claude-challenger.md）。但 codex-challenger 的 4 个事件触发（/code-review, test_failure, plan_review, genealogy_settlement）均无对应 hook，全靠 D-策略手动认领。genealogy_settlement 触发是 v42-swarm 新增但尚未有 hook 支持 | P2 | codex-challenger 的 genealogy_settlement 触发需要 hook 支持或在 ceremony_scan 中集成自动检测 |
| 4 | 递归总方针 L43 | "审查链的不可通约性积累尚未自动化为拓扑操作" | **属实声明的缺口**。递归总方针自身已声明这是缺口 | P1 | 已知缺口，需要设计不可通约性检测 → 自动 puncture 机制 |
| 5 | 递归总方针 L50-53 | 158号第三步"取消编排者作为最终决断者的位置" | **部分属实**。Gemini decide 存在（orchestrator_proxy 在 dispatch-dag.yaml L539-550），plan-review skill 存在。但编排者 INTERRUPT 权实质上仍是 gatekeeper——所有重大谱系变更仍由编排者 INTERRUPT 触发（161/156/157/162号全部由编排者 INTERRUPT 产生） | P2 | 结构性缺口：编排者 INTERRUPT 权在实践中不是"异步审计"而是"同步决策触发器"。需要度量 INTERRUPT 频率和类型，区分"异步审计"和"实时审批" |
| 6 | 递归总方针 "收缩相与承重点" | "系统有权判定某个谱系条目为承重点并拒绝对其进行否定" | **纯文档声明，无代码实现**。grep "承重点" 在所有 .py/.sh 文件中零匹配。没有任何代码机制阻止对标记为承重点的谱系条目执行否定操作 | P1 | 需要在 genealogy-write-guard.sh 或 topology_operator.py 中增加承重点标记检查，拒绝对承重点条目写入 negates 边 |
| 7 | 递归总方针 "谱系的生成性" | "谱系不只记录和发现，更是概念/概念分离的生产引擎" | **纯认识论声明，无代码实现**。grep "生成性" 在所有 .py/.sh 文件中零匹配。ceremony_scan 的 discover_business_tasks() 只扫描测试失败，不从谱系推导新概念 | P2 | 认识论层面声明。"生成"由 LLM 在谱系分析中完成（蜂群行为），不需要独立代码实现。但 ceremony_scan 可以扩展：从谱系 tensions_with 边自动推导"生成态工位" |
| 8 | 递归总方针 "RTAS是谱系的实现" | RTAS 循环以谱系为中心 | **部分属实**。ceremony_scan 从 session 遗留 + roadmap + pending 谱系推导工位，但不从已结算谱系的下游推论直接推导。downstream_audit 模块存在但作为二阶反馈，不是核心循环 | P2 | ceremony_scan 的核心循环应以谱系为主要数据源，当前以 roadmap/session 为主 |
| 9 | 递归总方针 "严格=反熵" | "忠实严格地生成谱系，熵就暂时排除于系统之外" | **纯认识论声明，无度量机制**。grep "反熵" 在所有 .py/.sh 文件中零匹配。没有任何指标衡量系统是否在"反熵"还是"兜圈子" | P2 | 可选度量：谱系增长率 / session 循环数、每循环新结算谱系数、声明-实现缺口趋势。但这是认识论声明而非工程要求 |
| 10 | 174号-2 下游推论 | "ceremony_scan 异常检测不应局限于编号冲突——任何谱系/代码/DAG的不一致都应被检测" | **声明膨胀**。ceremony_scan.py detect_genealogy_anomalies() 只检测 2 类：(1) duplicate_number（编号重复）(2) id_mismatch（文件名 vs 内部 id）。不检测：depends_on 断裂引用、dag.yaml 节点缺失、frontmatter schema 违规、DAG 环路 | **P0** | ceremony_scan 异常检测需扩展：增加 dag.yaml 节点完整性检查（当前 159-162 号节点缺失就是活证据）、depends_on 边引用验证 |
| 11 | dag.yaml 完整性 | 所有已结算谱系文件应在 dag.yaml 中有对应节点 | **159、160、161、162 号节点缺失于 dag.yaml**。文件存在于 settled/ 目录，但 dag.yaml nodes 列表中无对应条目。174号的 depends_on 引用 161/162 导致**两条断裂边** | **P0** | 立即在 dag.yaml 中补充 159/160/161/162 号节点及其 depends_on/negates 边 |
| 12 | dispatch-dag event_skill_map | genealogist skill 声明 3 个触发事件 | **无对应 hook**。genealogist 的 task_complete、/escalate、session_end 三个触发均无 hook 实现。完全依赖 D-策略（Lead 手动认领）。这意味着 genealogist skill 在实践中从未被自动触发 | P2 | D-策略是设计意图（082号），但 genealogist 作为 structural skill 是否应有至少一个自动触发 hook 值得评估 |
| 13 | dispatch-dag event_skill_map | quality-guard skill 声明 3 个触发事件 | **无直接对应 hook**。quality-guard 的 file_write 触发无 hook。result-package-guard.sh 和 genealogy-write-guard.sh 覆盖部分功能但不是 quality-guard skill 的直接触发。完全依赖 D-策略 | P2 | 同上，D-策略是设计意图 |
| 14 | dispatch-dag event_skill_map | code-verifier skill 声明 2 个触发事件 | **无对应 hook**。code-verifier 的 file_write src/**/*.py 和 tests/**/*.py 触发无 hook | P2 | D-策略覆盖。但 code-verifier 作为 structural skill 缺少自动触发是一个事实 |
| 15 | dispatch-dag event_skill_map | skill-crystallizer 声明 pattern_buffer_ready 触发 | crystallization-guard.sh 存在。**部分覆盖**——hook 检测 candidate 模式，但不直接执行结晶 | P2 | crystallization-guard.sh 是 advisory hook（提示），结晶执行仍需 Lead 认领 |
| 16 | 行为原则 1 | "严格，不务实" | no-patch-mentality.md 存在。**无代码强制**——这是 rules 文件，由 Claude Code 平台强制加载到 LLM context 中。没有 hook 或脚本在代码层面检测"务实行为" | P2 | 规则通过 LLM context injection 执行，不需要代码强制。这是平台机制的设计意图 |
| 17 | 行为原则 2 | "矛盾是最有价值的产出" | no-workaround.md 存在。**无代码强制**。同上 | P2 | 同上 |
| 18 | 行为原则 3 | "否定史=谱系拓扑" | genealogy/ 目录存在，dag.yaml 存在。**DAG 不完整**——159-162 号节点缺失（见缺口 #11） | P0 | 修复 dag.yaml |
| 19 | 行为原则 6 | "承重点不可否定——系统有权判定承重点并拒绝否定" | **无代码实现**（见缺口 #6）。没有任何谱系条目被标记为承重点，没有机制阻止对任何条目执行否定 | P1 | 需要：(1) 在谱系 frontmatter 中增加 load_bearing: true 字段 (2) genealogy-write-guard.sh 检查 negates 目标是否为承重点 |
| 20 | 行为原则 7 | "谱系是生产引擎" | **纯认识论声明**（见缺口 #7） | P2 | 认识论声明，LLM 行为层实现 |
| 21 | 行为原则 8 | "严格=反熵" | **纯认识论声明，无度量**（见缺口 #9） | P2 | 认识论声明 |
| 22 | 三步战略第一步 | topology_operator.py + dispatch-dag topology-mutator | **属实**。35 测试全绿 + dispatch-dag 注册完整 | -- | 无需修复 |
| 23 | 三步战略第二步 | "审查链不可通约性积累尚未自动化" | **已声明缺口**（见缺口 #4） | P1 | 已知缺口 |
| 24 | 三步战略第三步 | "Gemini decide、plan-review 无需编排者审批" | **部分属实**（见缺口 #5）。Gemini decide 可独立运行，plan-review skill 定义完整。但编排者 INTERRUPT 在实践中仍是主要决策触发方式 | P2 | 结构性观察 |
| 25 | 163-174号谱系 | 所有 depends_on 引用应存在 | **174号→161/162 断裂**。dag.yaml 中 174 的 depends_on 边指向 161/162，但这两个节点不存在于 dag.yaml | **P0** | 补充 159-162 号节点到 dag.yaml |
| 26 | 163-174号谱系 | 所有下游推论应在 downstream-action-overrides.yaml 中有记录 | **属实**。174-1/174-2/174-3 全部标记为 resolved。172-1/172-2/172-3 全部有记录。173-1/173-2/173-3 全部有记录 | -- | 无需修复 |
| 27 | 163-165号谱系 | 文件应有 frontmatter | **163/164/165 号无 YAML frontmatter**。这三个文件缺少标准 frontmatter（---/id/depends_on/...），导致 ceremony_scan 和 topology_operator 无法从中提取元数据 | P1 | 为 163/164/165 号添加标准 frontmatter |
| 28 | dispatch-dag validation | "event_skill_map 中所有 structural skill 的 agent 文件存在" | **属实**。genealogist.md、quality-guard.md、meta-observer.md、code-verifier.md、skill-crystallizer.md、topology-manager.md 全部存在 | -- | 无需修复 |
| 29 | dispatch-dag genealogical_coordinates | 引用的谱系编号应存在 | **属实**。所有引用（012、033、004、019d、030a、041、062、043、155）均存在于 settled/ | -- | 无需修复 |
| 30 | 174号谱系文件 | "更新 .chanlun/genealogy/dag.yaml：增加173号、174号节点及边" | **属实**。dag.yaml 中 173/174 节点存在，depends_on 边存在（174→069/161/162，其中161/162目标节点缺失——见缺口 #11） | P0 | 缺口 #11 的子集 |

---

## 按严重性分类汇总

### P0（阻塞性缺口）

| # | 缺口 | 修复方案 |
|---|------|---------|
| 11 | dag.yaml 缺少 159/160/161/162 号节点 → 174号 depends_on 边断裂 | 立即补充节点和边 |
| 10 | ceremony_scan 异常检测声明覆盖"一切不一致"但实际只检测编号冲突/id不匹配（159-162缺失就是活证据——如果 ceremony_scan 检测 dag.yaml 完整性，这个缺口早就被发现了） | 扩展 detect_genealogy_anomalies() 增加 dag.yaml 节点完整性 + depends_on 引用有效性检查 |
| 25 | 174号→161/162 断裂边（#11 的子集） | #11 修复后自动解决 |

### P1（功能性缺口）

| # | 缺口 | 修复方案 |
|---|------|---------|
| 4 | 审查链不可通约性未自动化为拓扑操作 | 已知缺口，递归总方针已声明 |
| 6 | 承重点保护无代码实现 | genealogy-write-guard.sh 增加承重点检查 |
| 19 | 同上（行为原则6的声明-实现缺口） | 同上 |
| 27 | 163/164/165号谱系无 frontmatter | 补充标准 frontmatter |

### P2（设计意图内的差距 / 认识论声明）

| # | 缺口 | 性质 |
|---|------|------|
| 3 | codex-challenger genealogy_settlement 无 hook | D-策略设计意图，但新增的 genealogy_settlement 触发尚无工具链 |
| 5 | 编排者 INTERRUPT 权实质上仍是同步决策触发器 | 结构性观察，非工程缺口 |
| 7 | "谱系是生产引擎"无代码实现 | 认识论声明，由 LLM 行为层实现 |
| 8 | RTAS 循环未以谱系为核心数据源 | ceremony_scan 以 roadmap/session 为主，谱系为辅 |
| 9 | "严格=反熵"无度量 | 认识论声明 |
| 12-15 | structural skill 无自动触发 hook | D-策略（082号）设计意图 |
| 16-17 | 行为原则无代码强制 | 通过 LLM context injection 执行 |
| 20-21 | 行为原则7/8纯认识论 | 认识论声明 |
| 24 | 编排者异步审计 vs 实时审批 | 结构性观察 |

---

## 补充审计：Codex/Gemini 三个位置的声明-实现缺口

> 来源：Lead 补充指令——编排者在上一个对话中将 Codex 放在三个位置上，全部声明未实现

### Codex 三个位置

| # | 来源 | 声明内容 | 实际状态 | 严重性 |
|---|------|---------|---------|--------|
| 31 | 155号：代码层异质审查者 | Codex 作为代码层异质审查者，三种模式（review/diagnose/decide） | **代码模块存在**（src/newchan/codex/ 含 engine.py/modes.py/registry.py/__main__.py），agent 定义存在（codex-challenger.md）。**执行证据极少**：.chanlun/review-results/ 中仅有 1 条记录（codex-diagnose-20260223-2122.md），且为 diagnose 模式。review 模式零执行、decide 模式零执行。触发完全靠手动——5 个事件触发（/code-review, test_failure, plan_review, genealogy_settlement, manual）全部无对应 hook | **P0** |
| 32 | 159号：写代码标配 | "Codex 异质审查从'条件触发'提升为'蜂群写代码标配'"——dispatch-dag codex-challenger 增加 file_write 触发 | **声明与实现矛盾**。159号谱系声明"增加 file_write 触发（src/**/*.py）"，但 dispatch-dag.yaml 中 codex-challenger 的 triggers 列表无 file_write 事件（只有 /code-review、test_failure、plan_review、genealogy_settlement、manual_invocation）。file_write → Codex 自动触发**从未实现** | **P0** |
| 33 | 160号：Plan 阶段多模型对审 | plan-review skill 定义 Opus 出方案 + Codex 评审多轮对审 | **skill 文件完整**（.claude/skills/plan-review/SKILL.md 定义完整工作流）。**零执行证据**：.chanlun/review-results/ 中无任何 plan-review-*.md 文件。plan-review 流程从未被执行过 | **P0** |

**Codex 总结**：编排者声明 Codex 在三个位置（异质审查/写代码标配/Plan对审），实际执行证据仅 1 次 diagnose 调用。review 零次、decide 零次、plan-review 零次。159号声明的 file_write 触发未写入 dispatch-dag.yaml。

### Gemini 位置审计

| # | 来源 | 声明内容 | 实际状态 | 严重性 |
|---|------|---------|---------|--------|
| 34 | dispatch-dag L220-233 | gemini-challenger 声明 3 个触发：/challenge、spec/theorems/* 变更、escalate_choice | **有 hook 支持**：genealogy-gemini-verify.sh 在谱系写入后输出 advisory 提示。但 spec/theorems/ 触发从未发生——项目中 spec/theorems/ 目录不存在 | P1 |
| 35 | dispatch-dag L440-445（175号新增） | genealogy_settlement → gemini-challenger 概念层异质否定 | **刚声明，尚无执行证据**。dispatch-dag.yaml 中 skill_flow_edges 和 event_edges 都有此映射（L440-445, L543-547），但无对应 hook 自动触发——依赖 D-策略手动认领 | P1 |
| 36 | 041号 orchestrator_proxy | Gemini decide 模式——选择/语法记录类决断 | **有执行证据**。tmp/ 目录下有 35+ 个 gemini-*.md 文件，包括多次 challenge、bilateral discussion、audit 结果。Gemini decide 是三个位置中执行证据最充分的 | -- |
| 37 | dispatch-dag L61 | "dispatch-dag 修改经 meta-observer + gemini-challenger 审查" | **无证据表明 dispatch-dag 修改时 Gemini 被自动触发审查**。所有 dispatch-dag 修改都是 Lead 直接执行 | P2 |
| 38 | gemini-challenger.md L8-12 | 声明 5 种模式：challenge/verify/derive/decide/手动 | **derive 模式零执行证据**。verify 模式零执行证据（spec/theorems/ 不存在）。仅 challenge 和 decide 有执行记录 | P1 |

**Gemini 总结**：Gemini 的 decide 模式是三个异质否定源中执行证据最充分的（35+ tmp/ 文件）。但 verify/derive 模式从未执行，spec/theorems/ 触发目标目录不存在。新增的 genealogy_settlement 触发尚无 hook。

### Codex vs Gemini 对比

| 维度 | Codex | Gemini |
|------|-------|--------|
| 声明的模式数 | 3（review/diagnose/decide） | 5（challenge/verify/derive/decide/manual） |
| 有执行证据的模式 | 1（diagnose × 1次） | 2（challenge/decide × 35+次） |
| 零执行的模式 | review, decide | verify, derive |
| file_write 自动触发 | 159号声明但未实现 | 不适用 |
| plan-review | 定义完整但零执行 | 不适用 |
| genealogy_settlement | 175号新增，无 hook | 175号新增，无 hook |
| hook 支持 | 零 | genealogy-gemini-verify.sh（advisory） |

---

## 属实/无需修复项

以下声明经验证属实，无缺口：

1. topology_operator.py 35个测试全绿 ✓
2. dispatch-dag topology-mutator skill 注册完整 ✓
3. 三个 challenger agent 文件存在 ✓
4. dispatch-dag event_skill_map structural skill agent 文件全部存在 ✓
5. genealogical_coordinates 引用的谱系编号全部存在 ✓
6. 163-174号谱系文件全部存在于 settled/ ✓
7. 174号下游推论全部在 downstream-action-overrides.yaml 中标记为 resolved ✓
8. 172/173号下游推论全部有记录 ✓
9. downstream-action-overrides.yaml 引用的 63 个谱系 ID 全部在 settled/ 中存在 ✓
10. dag.yaml 163-174号节点存在（除 159-162 缺失外） ✓
11. dag.yaml 边引用完整性：仅 2 条断裂边（174→161, 174→162） ✓

---

## 审计结论

**6 个 P0 缺口**：

| # | 缺口 | 类型 |
|---|------|------|
| 11 | dag.yaml 缺少 159-162 号节点 → 边断裂 | 数据完整性 |
| 10 | ceremony_scan 异常检测范围不足 | 声明-实现 |
| 25 | 174号→161/162 断裂边（#11子集） | 数据完整性 |
| 31 | Codex 异质审查者：review/decide 零执行，仅 1 次 diagnose | 声明-实现 |
| 32 | 159号声明 file_write→Codex 自动触发：未写入 dispatch-dag | 声明-实现 |
| 33 | plan-review 零执行（skill 完整但从未运行） | 声明-实现 |

**7 个 P1 缺口**：

| # | 缺口 |
|---|------|
| 4 | 审查链不可通约性未自动化（已声明缺口） |
| 6/19 | 承重点保护无代码实现 |
| 27 | 163-165号谱系无 frontmatter |
| 34 | Gemini spec/theorems/* 触发目标不存在 |
| 35 | Gemini genealogy_settlement 触发无 hook |
| 38 | Gemini verify/derive 模式零执行 |

**16 个 P2 缺口**：D-策略设计意图或认识论声明。

### 关键发现

1. **P0 缺口 #10 和 #11 互为证据**——如果 ceremony_scan 的异常检测足够完整（174-2 的声明），它应该已经检测到 159-162 号节点缺失于 dag.yaml 的事实。159-162 号的缺失是 ceremony_scan 异常检测不足的活证据。

2. **Codex 三个位置全部声明远超实现**——编排者声明 Codex 在三个位置（异质审查/写代码标配/Plan对审），实际执行证据仅 1 次 diagnose 调用。159号声明的 file_write 触发甚至未写入 dispatch-dag.yaml——这是**声明了实现但实现本身不存在**的缺口。

3. **Gemini 比 Codex 好但也有缺口**——Gemini decide 有 35+ 执行记录，是三个异质否定源中最活跃的。但 verify/derive 模式零执行，spec/theorems/ 触发目标目录不存在（声明了永远不会触发的事件）。

4. **异质否定自动化程度**：三个异质否定源（Gemini/Codex/Claude-challenger）全部依赖 D-策略手动认领。genealogy_settlement 触发（175号新增）对 Gemini 和 Codex 都已声明但无 hook 支持。自动化程度 = 0。
