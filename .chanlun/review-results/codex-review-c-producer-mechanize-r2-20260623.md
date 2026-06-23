# Codex 异质审计（约束4）— 任务 #41 第二轮（spawn 基因 + 递归块 + 自动触发）

> 模式：codex exec（ChatGPT 订阅登录路径；newchan.codex API 路径 429）
> 模型：gpt-5.2-codex ｜ tokens: 47,199 ｜ 日期: 2026-06-23
> 审计请求 A-E：三缺口补全 / no-workaround 严格性 / event_skill_map 合理性 / 单一源drift / 拦停回归

## Codex 逐项裁决（原文）

三块确实进入 canonical `spawn_mandate.template`，hook 也会在 check2 的无主任务 `(c)spawn` 路由中注入它：[team-topology.json](/Users/silencehan/Projects/NewChanlun/.claude/team-topology.json:153)、[ceremony-completion-guard.sh](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:367)。  
但“每次 spawn”不成立：check1.5 结构工位 bootstrap 的 spawn 指令没有注入 mandate：[ceremony-completion-guard.sh](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:264)。

**B. CONCERN**  
没有偷换成 teammate 自 spawn；文本明确是 `事件→TaskCreate(metadata.agent_type)→Lead (c)spawn`，hook 消费无主任务并要求 Lead spawn：[team-topology.json](/Users/silencehan/Projects/NewChanlun/.claude/team-topology.json:152)、[ceremony-completion-guard.sh](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:392)。  
但“自动触发”仍是 prompt mandate，不是事件检测器；事件→TaskCreate 依赖工位服从，不是 hook 可执行保证。

**C. CONCERN**  
现有映射主干合理，且对应 agent 文件存在。问题是映射不完整且有泛化错配风险：`code_change` 总是包含 `python-reviewer`，对非 Python 代码过宽；缺少明显事件如 `security_sensitive_change→security-reviewer`、`build_failure→build-error-resolver`、`docs_contract_change→doc-updater`、`topology_change→topology-manager/topology-analyst`：[team-topology.json](/Users/silencehan/Projects/NewChanlun/.claude/team-topology.json:144)。

**D. FAIL**  
单一源仍有 drift。结构字段里的 `event_skill_map.code_change` 包含 `python-reviewer`，但 template 只写 `code-verifier/code-reviewer`；template 还只枚举了 4 类事件，未注入完整 7 类映射：[team-topology.json](/Users/silencehan/Projects/NewChanlun/.claude/team-topology.json:144)、[team-topology.json](/Users/silencehan/Projects/NewChanlun/.claude/team-topology.json:153)。  
fallback 也不是纯“降级非副本”，它复制了核心语义并已与 canonical 简化不一致：[ceremony-completion-guard.sh](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:366)。

**E. CONCERN**  
check2 的 #41 注入本身没有新增 block 条件，只改路由文本和 mandate 注入。但当前实际 diff 新增了 check0 早退路径，会在 context 达阈值时绕过后续 check1-5：[ceremony-completion-guard.sh](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:51)。若把本 diff 作为一个审计单元，“未碰拦停行为”不成立。

总裁决：第二轮补上了主路径的三块语义，但还不能判定为“真补全且无拦停回归且 no-workaround 严格”；主要卡在单一源 drift、非 check2 spawn 覆盖缺口、以及同 diff 中 check0 改变拦停行为。
2026-06-23T09:35:54.663363Z ERROR codex_core::session: failed to record rollout items: thread 019ef3d4-3094-75a1-8c9f-2dc37221d3d7 not found
tokens used
47,199
**A. CONCERN**  
三块确实进入 canonical `spawn_mandate.template`，hook 也会在 check2 的无主任务 `(c)spawn` 路由中注入它：[team-topology.json](/Users/silencehan/Projects/NewChanlun/.claude/team-topology.json:153)、[ceremony-completion-guard.sh](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:367)。  
但“每次 spawn”不成立：check1.5 结构工位 bootstrap 的 spawn 指令没有注入 mandate：[ceremony-completion-guard.sh](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:264)。

**B. CONCERN**  
没有偷换成 teammate 自 spawn；文本明确是 `事件→TaskCreate(metadata.agent_type)→Lead (c)spawn`，hook 消费无主任务并要求 Lead spawn：[team-topology.json](/Users/silencehan/Projects/NewChanlun/.claude/team-topology.json:152)、[ceremony-completion-guard.sh](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:392)。  
但“自动触发”仍是 prompt mandate，不是事件检测器；事件→TaskCreate 依赖工位服从，不是 hook 可执行保证。

**C. CONCERN**  
现有映射主干合理，且对应 agent 文件存在。问题是映射不完整且有泛化错配风险：`code_change` 总是包含 `python-reviewer`，对非 Python 代码过宽；缺少明显事件如 `security_sensitive_change→security-reviewer`、`build_failure→build-error-resolver`、`docs_contract_change→doc-updater`、`topology_change→topology-manager/topology-analyst`：[team-topology.json](/Users/silencehan/Projects/NewChanlun/.claude/team-topology.json:144)。

**D. FAIL**  
单一源仍有 drift。结构字段里的 `event_skill_map.code_change` 包含 `python-reviewer`，但 template 只写 `code-verifier/code-reviewer`；template 还只枚举了 4 类事件，未注入完整 7 类映射：[team-topology.json](/Users/silencehan/Projects/NewChanlun/.claude/team-topology.json:144)、[team-topology.json](/Users/silencehan/Projects/NewChanlun/.claude/team-topology.json:153)。  
fallback 也不是纯“降级非副本”，它复制了核心语义并已与 canonical 简化不一致：[ceremony-completion-guard.sh](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:366)。

**E. CONCERN**  
check2 的 #41 注入本身没有新增 block 条件，只改路由文本和 mandate 注入。但当前实际 diff 新增了 check0 早退路径，会在 context 达阈值时绕过后续 check1-5：[ceremony-completion-guard.sh](/Users/silencehan/Projects/NewChanlun/.claude/hooks/ceremony-completion-guard.sh:51)。若把本 diff 作为一个审计单元，“未碰拦停行为”不成立。

总裁决：第二轮补上了主路径的三块语义，但还不能判定为“真补全且无拦停回归且 no-workaround 严格”；主要卡在单一源 drift、非 check2 spawn 覆盖缺口、以及同 diff 中 check0 改变拦停行为。

## 本工位 triage + 修复（codex 第二轮后）
- A CONCERN→**已修**：check1.5 结构 spawn 路由注入 topo_address+parent_callback 基因（并修 stale Task(team_name)→Agent(name)）。
- B CONCERN→**结构边界(创世Gap)**：auto-trigger 强制性上界=工位/Lead 服从 mandate；hook 不能 detect-and-act（069）。已诚实记 auto_trigger_strict_form+边界条件，非伪装自涌现（codex 确认无 teammate 自 spawn 偷换）。
- C CONCERN→**已修**：event_skill_map 扩至 13 事件，python-reviewer 拆为 code_change_python，新增 security/build/docs/topology/refactor。
- D FAIL→**已修**：template 改为引用 event_skill_map（单一源，不复制具体映射）；fallback 收紧为"降级+指向 canonical+仅不变量"。
- E CONCERN→**scope 误报**：check0 是 #40(autocompact-restore)产出，非本任务；本任务严格只改 check2 (c)spawn + check1.5 路由文本，block 逻辑 6/10/20 不变。
