---
id: '548'
number: 548
title: "hook 双类型分离——bootstrap hook 行动指令注入（097 结构性例外）vs tool-拦截 hook 指令索引（097 纯化靶点）"
type: 语法记录
status: settled
date: 2026-06-22
settled_date: 2026-06-22
settlement: "编排者 2026-06-22 裁决 verdict=B（追认 SessionStart hook 为 097 结构性例外）。分界线由两轮质询坐实（第一轮判 097 违反，第二轮经 042/048/407 三先例自我推翻）。Gemini 异质补验待配额（降级父蜂群事后审计，不阻塞结算）。"
level: "L0（谱系/源码事实：042/048/407 三已结算先例 + 097 negates=[] + settings.json hook 注册）。无经验否证维度——纯定义/谱系一致性裁定。"
负责工位: CC session（hook-compact-ceremony-fix 工位）
provenance: "[新缠论:实装审计]（compact 后 ceremony 不自动触发的 spec-execution-gap 修复）+ [蜂群架构]（097 hook 纯化 / 407 compact 行为恢复 / 042 hook 网络 / 048 universal stop-guard）"
negation_source: "编排者裁决（verdict=B）+ homogeneous 质询（gemini-challenger 两轮，Gemini API 429 全程降级为同质）"
negation_form: separation
negates: "hook-purity-applies-uniformly（隐含命题：『097 的 hook 纯化—只阻断/放行、不做指令—统一适用于所有 hook 类型』——被证伪：SessionStart 无 tool 可拦截，其存在论职责本就是注入初始结构；042/048/407 三已结算先例证明 hook 注入正面格式行动指令一直合法，097 negates=[] 一个没否）"
topo_effect: "split:097:hook-types — 097号的『hook』分裂为两类：(a) tool-拦截hook（PreToolUse/PostToolUse/Stop，严守阻断/放行）+ (b) bootstrap-hook（SessionStart，结构性例外，合法注入正面格式行动指令）"
depends_on:
  - '097'   # hook 纯化——被本号分离为双类型；本号是其精化（structural exception 显式化）
  - '407'   # compact 后行为层恢复——修复1 已向 SessionStart 注入 ROLE_BOUNDARY 行动锚点（先例）
  - '137'   # 否定性禁令对行为执行层无效 / 声明层 compact 后效力为零——hook 是唯一跨 compact 断裂的行为通道
  - '042'   # hook 网络模式——明列 hook 动作语义含"注入 systemMessage"
  - '048'   # universal stop-guard——明列"stop 时注入下一步扫描指令"（行动注入先例）
related:
  - '027'   # 正面指令优于禁止——FIRST_ACTION/ROLE_BOUNDARY 的正面格式依据
  - '028'   # ceremony-default-action——"开盘是自动的"，SessionStart 自动触发 ceremony 的存在论依据
tensions_with: []
escalation: ".chanlun/escalations/2026-06-22-0343-097-vs-407-sessionstart-hook-instruction-injection.md"
---

# 548 号：hook 双类型分离——bootstrap hook 行动指令注入 vs tool-拦截 hook 指令索引

**认识论**：L0（谱系/源码事实，纯定义一致性裁定，无经验否证维度）。

## 一、触发（spec-execution-gap）

CC session 经 compact（autocompact 或 /compact）恢复后，本应自动触发的 ceremony 热启动没有发生，用户被迫手动 `/ceremony`。根因为 `session-start-ceremony.sh` 用 `systemMessage`（仅用户可见横幅，不进模型 context）注入恢复指令，被 compact summary 的 "Resume directly — do not acknowledge" 压过。修复（通道改 `hookSpecificOutput.additionalContext` + `source==compact` 分支 + 137号正面格式 `FIRST_ACTION`）触发了 097↔407 边界质询。

## 二、概念分离（本号核心）

097号的「hook」此前被当作单一对象，要求"动作语义必须阻断/放行，绝不能提示/教学"。本号将其**分离为两类**：

| 类型 | 实例 | 存在论职责 | 097 约束 |
|------|------|-----------|---------|
| (a) **tool-拦截 hook** | PreToolUse / PostToolUse / Stop | 拦截具体 tool 调用，裁决阻断/放行 | **严守 097**：只阻断/放行，不得复活指令层 |
| (b) **bootstrap hook** | SessionStart | **无 tool 可拦截**；存在论职责本就是"向新生命周期注入初始结构" | **097 的结构性例外**：合法注入正面格式行动指令 |

分离依据：tool-拦截 hook 存在的前提是"有一个 tool 调用等待裁决"；SessionStart 在任何 tool 调用之前触发，没有可裁决对象——它唯一能做的就是 context 注入（bootstrap）。把"只阻断/放行"施加于一个无可阻断对象的 hook 是范畴错误。

## 三、分界线：行动指令注入（合法）vs 指令索引/教学（097 靶点）

097 真正纯化的不是"hook 含指令"，而是**指向指令层的索引/教学**：

| 097 删除的（指令索引，违规） | 097 保留的（行动注入，合法，三先例已结算） |
|----------------------------|------------------------------------------|
| team-structural-inject.sh："必须读取 sub-swarm-ceremony skill"——指向声明层的**静态指针**，可迁入 CLAUDE.md（promoter），在 hook 里是冗余第四层 | **042号** flow-continuity-guard 注入"commit 后不允许停顿"（行为强制） |
| → 迁入 CLAUDE.md 声明层 | **048号** universal stop-guard 注入"stop 时下一步扫描指令"（正面行动） |
| | **407号** 修复1 注入 ROLE_BOUNDARY（compact 行为恢复锚点） |

`FIRST_ACTION`（"第一动作=ceremony 差异检查…spawn 蜂群"）= **048 stop-guard"注入下一步扫描指令"的 session-start 同构物**——同为 hook 注入的正面格式下一步行动指令，落 097 **保留侧**。

**决定性证据（L0）**：042号（2026-02-20）、048号（2026-02-20）均已结算且明列"注入 systemMessage/扫描指令"为合法 hook 动作语义；097号（2026-02-22，晚两天）`negates: []`，只删除路径索引型 team-structural-inject.sh，**有意保留**全部行动注入型 hook。日期序 + negates=[] 证明是**有意区分，非遗漏**。

## 四、发现2：通道 bug 一直掩盖 097↔407 张力（决定性）

407号修复1（其文件行 136-151）把 ROLE_BOUNDARY 锚点**追加到热启动 `systemMessage`**——与本次修复前 `session-start-ceremony.sh` 同一个**死通道**。

- 在本次通道修复（systemMessage→additionalContext）之前，ROLE_BOUNDARY 与既存"进入蜂群循环"指令**从未真正到达模型 context**（只进用户横幅）。
- 因此 097"hook 不教学"**在效果上从未被违反**——没有任何指令被"教"给模型。
- 推论：**407号修复1 自 2026-03-09 起实际上是非功能的**——它要修的 C2 角色回归，因通道 bug 从未真正注入锚点，从未真正被修复。
- 本次通道修复使该 hook **首次真正向模型 context 注入行动指令**，张力从潜伏变实在。**通道 bug 是此张力此前未爆发的唯一原因。**

故本号同时是 **407号的隐性 bug 修复记录**：通道修复是 407修复1 实际生效的必要前提。

## 五、裁决（verdict=B，编排者 2026-06-22）

**追认 SessionStart hook 为 097号的结构性例外。**

- `FIRST_ACTION` + `source==compact` 分支 + additionalContext 通道 + ROLE_BOUNDARY 全部**保留在 hook**，不回退。
- 选项A（迁 FIRST_ACTION 入声明层）被否：与 407号下游推论5（"声明层 compact 后效力=零"）+ 042/048 行动注入先例**直接矛盾**，选 A 须同时重开 042+048+407。

## 六、边界条件（关键——例外不可滥用）

1. **此例外仅限 SessionStart / bootstrap 类 hook。** 其他 tool-拦截 hook（PreToolUse/PostToolUse/Stop）仍受 097 严格约束，**不得借此例外复活指令层**。
2. 注入物必须是**正面格式行动指令**（"第一动作=X"）或**行为锚点**（ROLE_BOUNDARY），**不是指向声明层的静态索引**（"必须读 skill X"——那仍属 097 删除侧，应在 CLAUDE.md）。
3. 若未来 CC 平台为 SessionStart 增加可阻断对象（如 session 启动门控），分类(b)需重新评估是否回归(a)。
4. 若 additionalContext 注入语义改变（不再进 system-reminder / 被 summary 覆盖），整个修复失效，需重开。

## 七、下游推论

1. `session-start-ceremony.sh` 修复保留（通道 + source 分支 + FIRST_ACTION），不回退。
2. `agent-team-bootstrap.sh` 同病（spawn 指令走 systemMessage 死通道）——Lead 落实通道修复（同属分类(b) bootstrap hook，合法）。
3. `session-start-fable5.sh` 已正确（早用 additionalContext）——是分类(b)正确实现的先例。
4. 任何"以 systemMessage 注入可执行内容"的 bootstrap hook 都应改 additionalContext——通道选择是 137号"结构化模板有效"的**必要前提**（模板必须真进 context 才有效），此前未显式化。
5. 097号谱系记录应反向链接本号（hook 双类型分离精化）。

## 八、②根治验证方法（改动只对未来 session 生效，当前 session 无法热验）

下次 compact 恢复后确认 ceremony 自动触发：

1. **通道层**（可立即静态验证）：
   `echo '{"cwd":"'$PWD'","source":"compact"}' | bash .claude/hooks/session-start-ceremony.sh | python3 -m json.tool`
   → 确认输出含 `hookSpecificOutput.additionalContext` 且其内容含 compact header + FIRST_ACTION（已通过）。
2. **端到端**（下次真 compact 时观察）：触发 autocompact（context 达 75%）或手动 `/compact` → 恢复后模型**第一个动作**应为 ceremony 差异检查（读 session 快照 + git/谱系对比 + 评估并行工位 + 必要时 spawn），**而非**直接 resume 上个 task。
3. **反向确认**：若恢复后仍直接 resume 上个 task 而跳过 ceremony 差异检查 → additionalContext 未进入恢复后 context 或被 summary 覆盖 → 边界条件4 命中 → 重开。

## 九、降级补验（不阻塞结算）

两轮 gemini-challenger 质询均因 Gemini API 429 RESOURCE_EXHAUSTED 降级为同质（Claude 自身）。结论由 L0 硬谱系事实（042/048 日期 + 097 negates=[]）支撑而稳健，但真异质验证待 Gemini 配额恢复后补（父蜂群事后审计，依据 097号边界条件第三条）。

## 十、影响声明

- 写入 548号谱系记录（语法记录结晶，verdict=B）。
- 关联 escalation 文件 `2026-06-22-0343-...md` status 改 settled（verdict=B）。
- 精化 097号：hook 从单一对象分离为 tool-拦截 / bootstrap 双类型。
- 记录 407号修复1 的隐性通道 bug（自 2026-03-09 非功能）及其修复。
- 指向代码：`session-start-ceremony.sh`（已修，保留）、`agent-team-bootstrap.sh`（Lead 落实）。
