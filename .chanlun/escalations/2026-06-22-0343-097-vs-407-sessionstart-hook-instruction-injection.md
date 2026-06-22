---
date: 2026-06-22
time: "03:43"
trigger: heterogeneous-challenge-request
subject: "097号 hook 纯化 vs 407号 compact 恢复锚点——SessionStart hook 注入行为指令的边界冲突"
status: settled
verdict: B（编排者 2026-06-22 裁决——追认 SessionStart hook 为 097 结构性例外）
crystallized_to: .chanlun/genealogy/settled/548-hook-action-injection-vs-instruction-index.md
challenger: gemini-challenger (降级为同质质询，Gemini API 429 RESOURCE_EXHAUSTED)
---

# Escalation：097↔407 概念冲突——SessionStart hook 注入行为指令是否越界

## 触发事件

异质质询任务（challenge 模式）：对 session-start-ceremony.sh 修复的 097号边界判断进行对抗性质询。

被质询产出：
1. 输出通道改为 `hookSpecificOutput.additionalContext`
2. 新增 `source==compact` 分支专用 header
3. `FIRST_ACTION` 变量注入（"本回合第一个动作=ceremony 差异检查……spawn 蜂群"）

产出者声称：此修复在 097号 hook 纯化边界**之内**。

质询判定：**声称不成立**，存在真实的 097↔407 概念冲突。

---

## 冲突两端（精确定义）

### 端A：097号精神——hook 不做提示/教学

**已结算原则**（097号，2026-02-22）：
> "hook 层的动作语义必须是阻断/放行，绝不能是提示/教学"
> "彻底消除'第四层（提示层）'"

**当前 hook 注入内容**（FIRST_ACTION）：
```
"本回合第一个动作=ceremony 差异检查：对比下方状态快照(session vs 当前 git/谱系)，
确定本轮工作目标；若识别出≥2个可并行独立工位，用 TaskCreate+Task spawn 拉起蜂群循环。"
```

**结构同构判断**：此 FIRST_ACTION 与被097号删除的 `team-structural-inject.sh` 内容（"必须读取 sub-swarm-ceremony skill"）属同一功能形式：均是"hook 告诉 agent 应该做什么"的教学型行为指令，不是状态数据。

**若端A成立**：FIRST_ACTION 必须从 hook 中删除，迁入 CLAUDE.md 或 swarm-architecture skill 声明层。

### 端B：407号已裁定 + 137号约束——compact 后行为层必须通过 hook 正面格式恢复

**已结算事实**（137号，已结算）：
- 否定性禁令对行为执行层无效（RLHF 基底约束）
- CLAUDE.md 声明层在 compact 后效力假设为零

**已结算裁定**（407号修复1，已结算）：
> "在热启动系统消息追加角色边界锚点……这是正面格式（'Lead 做 X'），不是否定性禁令"
> "compact 后每次热启动都会注入，使行为层在每次恢复后都收到角色边界 few-shot"

**推论**：如果 compact 后 CLAUDE.md 的"执行 ceremony"声明效力为零，而 hook 是唯一在 compact 后仍然可靠注入的执行层通道，则不注入"第一动作=X"意味着 compact 后 ceremony 永远不会自动执行。

**若端B成立**：FIRST_ACTION 必须留在 hook 中（因为声明层效力为零，这是唯一有效通道）。

---

## 不可弥合点

端A 和端B 的矛盾可以精确表述为：

**命题P**："hook 注入行为指令（教学型内容）违反 097号"

- 端A 要求 P = True（hook 不做指令）
- 端B 要求 P = False（因为声明层对行为执行层无效，hook 是唯一有效通道）

这不是边界模糊，是两个已结算原则（097号和 137号+407号联合）对同一对象（SessionStart hook 的内容类型）产出直接矛盾的规定。

---

## 推导链

### 质询1：理由链(b)是否诡辩

产出者论点："SessionStart 没有 tool 可阻断/放行，因此 097号的 hook 纯化要求无法施加于 SessionStart。"

**诡辩结构**：
- 正确前提：SessionStart 确实没有 tool 可阻断（技术事实）
- 错误推论：因此 097号的深层精神（hook 不做指令层）不适用于 SessionStart

097号的精神有两层：
- 表层（技术层）：hook 的执行语义是阻断/放行
- 深层（概念层）：hook 不是提示/教学层，不做指令分发

产出者的论点只证伪了表层在 SessionStart 上的不适用性，没有触及深层。用表层技术约束的不适用性，推出深层概念约束也不适用——跨层推论无效。

**判定**：理由链(b) 是诡辩形式，但其背后有一个真实的困境（端B），该困境本身是合法的 escalation 材料。

### 质询2：状态快照 vs 行为指令的本体论区分

- 状态快照（git commit hash、谱系计数、session 时间戳）：世界状态描述，模型自主判断响应
- FIRST_ACTION（"第一个动作=X"）：行为指令，直接规定动作

两者在当前 hook 里被打包。前者合法，后者合法性是本次冲突的核心。

**判定**：两者是本体论上不同的两类事物，产出者以"动态 bootstrap"统一覆盖，存在概念混淆。

### 质询3：137号授权范围

137号约束的对象是 Lead 的输出格式（每次完成 phase 后输出"→ 接下来：X"）。

产出者推论：hook 注入的内容也必须是正面格式 → 正面格式合法。

**误用结构**：137号说"正面格式在 Lead 输出中有效"，不等于"正面格式在 hook 内容中合法"。约束施力对象不同，推论不能直接迁移。

**判定**：137号不授权 hook 注入行为指令，但 407号的修复1确实要求 hook 注入正面格式锚点——两者必须区分"角色约束声明"（407号明确裁定）vs "具体第一动作指令"（407号未裁定）。

### 质询4：更严格方案

**存在。** 以下分离方案能同时满足 097号和 137号+407号：

| 内容类型 | 当前位置 | 严格解位置 |
|---------|---------|----------|
| 状态快照（git/谱系/session） | hook | hook（合法） |
| ROLE_BOUNDARY（角色约束） | hook | hook（407号明确裁定） |
| compact/热启动 header（事实声明） | hook | hook（可接受，无指令含义） |
| FIRST_ACTION（第一动作指令） | hook | CLAUDE.md/skill 声明层 |

**代价**：如果 FIRST_ACTION 迁入声明层，其效力在 compact 后假设为零（137号），ceremony 差异检查的自动触发能力消失——这正是端B的困境。

---

## 需要编排者裁决的问题

**核心问题**：097号和 137号+407号在 SessionStart hook 的内容类型问题上产出矛盾。这是价值判断，不是逻辑推导：

**选项A**（优先 097号）：hook 不注入行为指令，FIRST_ACTION 迁入声明层，接受 compact 后 ceremony 差异检查不自动执行的代价。

**选项B**（优先 137号+407号）：承认 SessionStart hook 注入行为指令是对 097号的有意扩展（expansion），因为 compact 恢复是唯一需要"行为层主动注入"的场景，其他 hook 仍遵守 097号纯化原则。需要写入新谱系记录，明确 SessionStart hook 是 097号的结构性例外。

**选项C**（严格分离）：FIRST_ACTION 的内容从 hook 迁出，同时承认这带来的 compact 自动性损失，并讨论是否可以通过其他机制（precompact-save.sh、write_session.sh 字段、多位置锚点）弥补。

---

## 谱系关联

- 097号（已结算）：hook 纯化——hook 不做提示/索引/教学
- 137号（已结算）：否定性禁令对行为执行层无效
- 407号（已结算）：compact 后行为层恢复不对等，修复1要求注入角色边界锚点
- 本 escalation 是 407号修复1的边界判断争议

---

## 降级标注

Gemini 异质质询因 API 429 RESOURCE_EXHAUSTED 降级为同质质询（Claude 自身）。
本结论需在 Gemini 配额恢复后补充异质验证（父蜂群事后审计）。
降级依据：097号谱系边界条件第三条"异质验证（Gemini）在子蜂群中因 MCP 工具不可用，约束4降级为父蜂群事后审计"。

---

## 产出者补充（事实校正，供编排者裁决参考）

质询后由产出者（hook-compact-fix 工位）补充两条事实，质询过程未覆盖：

### 校正1：FIRST_ACTION 指令注入是**既存条件**，非本次 diff 引入

命题P 适用于 hook 的**既有设计**，不是本次修改新增的行为：
- 修改**前**该 hook 已注入 ceremony 触发指令：冷启动 `"请执行完整 /ceremony 确认定义基底"`、热启动 `"⚡自动进入蜂群循环：先评估可并行工位数(≥2即拉蜂群)…"`。
- hook 文件头注释（既有）："效果等同于用户手动输入 /ceremony，但零人工干预"——即"自动触发 ceremony"本就是该 hook 的设计目的（关联 028号 ceremony-default-action）。
- 407号修复1 已向该 hook 注入 ROLE_BOUNDARY 行为内容。
- 本次 diff 只做两件事：(1) 通道修复 systemMessage→additionalContext；(2) 把既存软请求重写为 137号正面格式。

**对裁决的影响**：编排者面对的不是"是否接受一个新功能"，而是"该 hook 自创世起就在 097 边界上，且 407号已推进过——现在是否显式裁定其存在形式"。选项B（结构性例外+写谱系）因此更接近"追认既存事实"而非"新增扩展"。

### 校正2：通道修复（systemMessage→additionalContext）独立于本冲突，应无条件保留

命题P 只针对**内容类型**（FIRST_ACTION 是否越界），不针对**通道**：
- 通道修复是纯交付可靠性修复——无论 hook 注入什么合法内容（状态快照、ROLE_BOUNDARY、header），都应经 additionalContext 而非 systemMessage（后者不进模型 context）。
- 同链 sibling `session-start-fable5.sh` 早已用 additionalContext，并在注释中写明同一根因——通道选择已是既定先例，本次只是消除 ceremony hook 与其的不一致。
- **即使编排者选 A/C（把 FIRST_ACTION 迁出 hook）**，状态快照与 ROLE_BOUNDARY 仍需经 additionalContext 注入——通道修复在三个选项下都成立。

**结论**：冲突的标的是 FIRST_ACTION 的**内容**，不是通道。通道修复与 source==compact 分支（事实声明，无指令含义）不在冲突范围内。

---

## 产出者补充2（谱系检查发现，决定性——重框定冲突）

应 Stop-Guard"谱系检查"指令，产出者读取 `407-lead-role-boundary-compact-regression.md`（已结算，2026-03-09，type=meta-rule，negation_form=expansion，negation_source=**homogeneous**）全文，发现三条改变冲突框定的事实：

### 发现1：通道 bug 一直掩盖着 097↔407 张力（核心）

407号修复1（行 136-151）明确：把 ROLE_BOUNDARY 锚点**追加到热启动系统消息**（即 `systemMessage`，407 引用的正是 `session-start-ceremony.sh:147` 那条 `⚡自动进入蜂群循环` 消息）。

→ **407号修复1 用的是同一个死通道（systemMessage）**。在本次通道修复（systemMessage→additionalContext）之前：
- ROLE_BOUNDARY 与既存的"进入蜂群循环"指令**从未真正到达模型 context**（只进了用户横幅）。
- 因此 097号"hook 不教学"在**效果上从未被违反**——没有任何指令被"教"给模型（内容流向了用户，不是模型）。
- 推论：407号修复1 自 2026-03-09 起**实际上是非功能的**——它试图修复的 C2 角色回归，因通道 bug 从未真正注入锚点，从未真正被修复。

→ 本次通道修复使该 hook **首次真正向模型 context 注入行为指令**（ROLE_BOUNDARY + FIRST_ACTION）。这才让 097↔407 张力从潜伏变为实在。**通道 bug 是这个张力此前未爆发的唯一原因。**

### 发现2：冲突不是"本次改动 vs 097"，而是"407号 从未与 097号 reconcile"

407号 depends_on=[226,137,057,218]，谱系关联=[226,137,057,218,143]——**完全不提 097号**。407号 negation_source=homogeneous（未经异质验证）。407号以 expansion 形式扩展了该 hook 的行为内容（追加 ROLE_BOUNDARY），却从未对照 097号的 hook 纯化原则做 reconcile。

→ 两个已结算记录（097 hook纯化 vs 407 hook行为锚点注入）**共存且互不引用**，张力自 407 结算起潜伏至今。本次工作（通道修复使 407 实际生效）是这个潜伏张力的**触发器**，不是制造者。

### 发现3：选项A 与 407号已结算推论直接矛盾（非"有代价"，是矛盾）

407号下游推论5（已 covered:v215）："**所有 rules/skills 中关于 Lead 行为的声明，在 compact 后的效力需要假设为'零'——不是'降低'，是'零'**"。且三个 rule 文件（no-unnecessary-escalation.md / lead-parallel-dispatch.md / post-commit-flow.md）的"compact 后效力声明"均写明："行为层恢复**依赖** `session-start-ceremony.sh` 注入的角色边界锚点"。

→ 选项A（把 FIRST_ACTION 迁入 CLAUDE.md/声明层）会把行为指令迁到一个**已被 407号结算为"此用途效力=零"**的层。这不是质询4 所说的"代价"，而是与 407号已结算推论5 的**直接矛盾**。选项A 实质要求**否定 407号推论5**。

### 对编排者裁决的影响（修订）

- 通道修复（systemMessage→additionalContext）不仅无争议，而且是 **407号修复1 实际生效的必要前提**——它修复的是 407 自身实现的潜在 bug。建议无条件保留（否则 407号修复1 继续非功能）。
- 真正的 CHOICE 收窄为：FIRST_ACTION/ROLE_BOUNDARY 这类**行为指令内容**经（已修复的）可靠通道注入模型，是否需要 097 重新裁定。
- 选项A 因与 407号推论5 直接矛盾，若选 A 须同时**重开 407号**。选项B（追认 SessionStart hook 为 097 结构性例外 + 写谱系明确 097↔407 reconcile）在谱系一致性上代价最小——它把一个潜伏的未 reconcile 张力显式化为已结算的例外。
- 强烈建议：本裁决待 Gemini 配额恢复后补**异质验证**，因为 407号本身是 homogeneous 结算，而 097↔407 的 reconcile 恰是需要异质视角的概念层裁定。

---

## 产出者补充3（谱系检查发现，决定性——冲突重分类为语法记录）

应 Stop-Guard"谱系检查"指令，产出者扫描全部 settled 谱系中"hook 注入"相关记录，发现 097 的真实分界线，**冲突在很大程度上消解**。

### 发现：存在 ≥3 个"hook 注入行为指令"的已结算先例，097 一个都没否定

| 记录 | 内容 | hook | 注入物 | status / negated_by |
|------|------|------|--------|---------------------|
| **042号**（Hook 网络模式） | 行48/56：hook 动作语义 = "block **或注入 systemMessage**"；flow-continuity-guard 注入"commit 后不允许停顿"(028号) | flow-continuity-guard（PostToolUse，**当前仍注册**） | 行为指令（反停顿） | 已结算 / `negated_by: []` |
| **048号**（Universal Stop-Guard） | 行37：引 027号"不是'禁止等待'，而是'stop 时**自动注入下一步扫描指令**'"；行42/52：阻止停止并**注入扫描指令** | ceremony-completion-guard（Stop，**当前仍注册**，即正在给本工位发消息的 Stop-Guard） | 正面格式下一步行动指令 | 已结算 / `negated_by: []` |
| **407号** | 修复1：注入 ROLE_BOUNDARY | session-start-ceremony（SessionStart） | 行为恢复锚点 | 已结算 |

097号 `negates: []`——它**只删除了 team-structural-inject.sh**（指向 skill 路径的**索引**），没有否定上述任何一个"注入行为指令"的 hook。

### 097 的真实分界线（被质询误读）

质询将 097 的深层精神读作"hook 不做任何指令分发"。但 042/048/407 三个已结算先例证明：**hook 注入正面格式的行动指令是已结算合法的 hook 动作语义**（042 明列"注入 systemMessage"，048 明列"注入扫描指令"）。

097 真正纯化的是**指向指令层的索引/教学**（"必须读取 skill X"——可迁入 CLAUDE.md promoter 的静态指针），**不是行动指令注入本身**：

| 097 删除的（索引/教学） | 097 保留的（行动注入，042/048/407 已结算） |
|------------------------|------------------------------------------|
| "必须读取 sub-swarm-ceremony skill"（指向声明层的指针，冗余） | flow-continuity-guard "不允许停顿"（行为强制） |
| → 迁入 CLAUDE.md（promoter） | Stop-Guard "注入下一步扫描指令"（正面行动） |
| | ROLE_BOUNDARY / FIRST_ACTION（compact 行为恢复） |

**FIRST_ACTION（"第一动作=ceremony 差异检查…spawn 蜂群"）= 048 Stop-Guard"注入下一步扫描指令"的 session-start 同构物**——同为 hook 注入的正面格式下一步行动指令。048 已结算且未被 097 否定 ⇒ FIRST_ACTION 落在 097 保留侧，不是删除侧。

### 重分类：选择 → 语法记录

冲突不再是"两个已结算原则不可弥合"，而是：**"hook 注入正面格式行动指令合法、097 纯化只针对指令层索引"这条分界线已跨 042/048/407 三记录运作，但从未被显式结晶**。这是**语法记录**（四分法：已在运作但未显式化的规则），不是需要价值权衡的纯选择。

- 选项A（迁 FIRST_ACTION 入声明层）现在不仅与 407推论5 矛盾，还与 **042/048 已结算的"注入行动指令"模式**矛盾——若选 A 须同时重开 042+048+407。
- 选项B（追认 SessionStart hook 注入行为恢复指令合法 + 写谱系结晶"行动注入 vs 指令索引"分界）= **把已运作的语法记录显式化**，谱系一致性代价最小，且与 042/048/407 三先例对齐。

→ 修订建议：本 escalation 的实质是**结晶一条已运作的语法记录**（hook 动作语义包含"注入正面格式行动指令"，097 纯化边界是"不做指令层索引"），而非在矛盾的两端做价值取舍。仍建议异质验证确认该分界线读法，但裁决重心从"A/B/C 价值选择"转为"是否追认并结晶 B"。
