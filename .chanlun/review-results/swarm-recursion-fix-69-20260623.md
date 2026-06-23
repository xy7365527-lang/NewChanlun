# 蜂群递归机制修复：(c)裁决传播到 stale 文档 + spawn 基因 present-by-instruction 机制化

**工位**：swarm-infra（topo_address: swarm/swarm-infra）
**任务**：#69 蜂群递归机制修复（teammate 不能 spawn teammate 的 spec-execution gap "也要修复"）
**日期**：2026-06-23　**parent_callback**：main
**认识论等级**：L2（真实 harness 实测核实 spawn/enforce 行为逐场景验证；非合成/纯代数）

---

## 1. 结论

四项任务，三项已完成实装，一项确认为已完成（#41 前置）。**无矛盾上浮**——(c) 与 275 张力可调和（见 §3.1）。

| 任务 | 结论 | 实装 |
|------|------|------|
| **#1 重写 sub-swarm-ceremony.md 为 (c) 模型** | **已由 #41 完成**（commit 870fe40588/9ea113a76e）。复核确认：skill 已是干净 (c) 模型——递归在任务结构（TaskCreate 子任务），Lead 只编排，保留四类节点 DAG。残留 `TeamCreate` 引用全部是"已废弃/不可用"文档（声明**不该**做什么），**非** stale 指令。 | 无需改动（验证完成） |
| **#2 核实 (c) 机制完整工作** | enforce hook 此前对 teammate 无-name Agent 调用只给泛化 advisory，**未显式导向 TaskCreate**（(c) 路径）；且 header 仍把 teammate 递归框定为"未决 spec-execution gap"（post-(c) 已 stale）。**已修复**：新增 (c) 路径 advisory + 更新 header。 | ✅ `agent-team-enforce.sh` |
| **#3 spawn 基因机制化** | 基因（topo_address/parent_callback/递归判断块）是 prompt **自由文本**——harness Agent 工具**无结构化基因字段**，hook 只能启发式 substring 检测（检字符串存在性，非语义有效性）→ 启发式 block 会误杀（同义表述）→ 违反 fail-open 不变量（032）。故基因在 enforce 层**保持 advisory**。**诚实声明（codex F2/090号）**：基因**无 hard structural 机制化路径**；实际形式 = **present-by-instruction**（bootstrap 注入含占位符 template）+ **advisory-detected**（enforce 字符串检测），**非** present-by-construction（不保证 Lead 填充语义有效值）。**已实装**模板注入。 | ✅ `agent-team-bootstrap.sh` |
| **#4 flag dispatch-dag.yaml genome stale** | dispatch-dag.yaml 在 `genome_layer`（line 25/50），020-gated（line 58）。**不擅改**。stale 行（与 562 矛盾）：**7, 11, 160, 633, 784**。flag 给 Lead 走 020-gated genome 修改流程。 | ⚠ flag only（见 §6） |

---

## 2. 定义依据

- **562号**（`.chanlun/genealogy/settled/562-bootstrap-structural-enforce.md`，`negates: ["075"]`）：结构能力是 teammate（spawn），非纯 skill 事件驱动。→ dispatch-dag.yaml 的 "结构=skill（075）" 行是 stale 文档（562 已结算，非开放冲突）。
- **(c)裁决（2026-06-23，编排者）**：teammate 识别子工作 → TaskCreate 子任务 → Lead spawn。递归载体从 team 拓扑（roster）改为任务拓扑（TaskList DAG）。这把 #30 诊断中的"teammate→teammate spec-execution gap"从**未决**改为**已结算的递归实现**。enforce hook 的 (c) 路径 advisory 是此裁决在 enforce 层的落地。
- **137号**：否定性文本对行为执行层无效 → 机制强制。**关键边界（codex F2/090号修正）**：137 的机制化形式取决于被强制对象的可结构化性。基因是自由文本 ∧ harness Agent 工具无结构化基因字段 → **无 hard structural 机制化路径**。可达上限 = present-by-instruction（bootstrap template 注入）+ advisory-detected（enforce 字符串存在性检测，非语义有效性）。这**不是** present-by-construction（不保证占位符被填实），但仍是当前 harness 下最强可达形式——**诚实标注其上限（090：不声明代码不具备的能力）**。post-hoc 启发式 block 被排除（引入 032 误杀风险 = 用一个违规换另一个违规 = 补丁思维）。
- **032号 / fail-open 不变量**：本 hook 仅在 POSITIVE 结构性违规（is_lead ∧ 无 name = 确定的 subagent）时 block；启发式判据不满足"POSITIVE 结构性"标准（substring 检测可被同义改写绕过/误伤）。
- **096号**：无孤立 subagent。teammate 的 Agent(无 name) = 孤立 subagent（违 096），除只读 Explore 搜索豁免。
- **016号**：规则无代码强制就不被执行——enforce hook matcher="Agent"（#30 已修）是该规则的载体。
- **020号**：修改 genome_layer（含 dispatch-dag.yaml）触发阻断等待编排者——故 #4 只 flag 不改。

**输入数据满足定义的哪些条件**：实测 7 场景全绿（§5）——name 是结构化 param（block 判据 POSITIVE 可靠），基因/递归块是 prompt 自由文本（advisory，fail-open）。bootstrap 输出 additionalContext 含基因 template（present-by-instruction 验证通过——template 注入存在性，非占位符填充语义）。

---

## 3. 边界条件（结论翻转条件）

1. **harness 恢复 teammate→teammate spawn**：(c) 路径 advisory 失去必要性——teammate 可直接命名 spawn，递归回到 team 拓扑，sub-swarm-ceremony 旧模型复活。
2. **PreToolUse 获得 prompt-rewrite 能力**：基因可从"bootstrap 模板注入"升级为"enforce hook 自动注入"，§1#3 的"hook 不能 inject"前提翻转。
3. **基因变为结构化 param**（非 prompt 自由文本）：启发式误杀风险消失 → block 成为安全的 POSITIVE 强制，§1#3 的"保持 advisory"结论翻转为 block。
4. **562 被否定（075 复辟）**：skill + bootstrap + enforce 三层 (c) 编码同步回退；dispatch-dag.yaml 的 075 行反而正确。
5. **(c) 被编排者改判 (a)/(b)**：若改判为"接受 LEAD 以下递归用 subagent"(a) 或"等 harness 恢复"(b)，enforce 的 (c) 路径 advisory 须改导向。

### 3.0a (c) 循环时延（已知特性，codex F3 — 非 bug，文档化备查）

(c) 模型把 spawn 中心化到 Lead 后引入一个**结构性时延**：teammate TaskCreate 的子任务**不会立即 spawn**——须等 Lead 完成当前轮、回到循环步骤1（TaskList scan）才被扫到并 spawn。Lead 执行长任务期间，无主子任务在队列中等待。这不是缺陷，是 (c) 中心化 spawn（harness flat-roster 物理约束）的必然代价：递归不是同步函数调用，是异步任务投递 + 轮询消费。与 275 局部依赖一致（子任务由 Lead 轮询消费，非 teammate 直接同步 spawn）。调试含义：子任务"未启动"可能只是 Lead 尚未 re-scan，非 spawn 失败——查 Lead 循环位置而非报错。post-commit-flow 的 push→rescan 原子链（224/225号）正是为压缩此时延而设。

**两场景区分（codex F3 §1.5 精确化）——避免把"已覆盖"误读为"全覆盖"**：

| 场景 | 状态 | 机制 |
|------|------|------|
| **Lead compact 后恢复**（context 饱和→autocompact→刷新） | **已覆盖** | `session-start-ceremony.sh` + `agent-team-bootstrap.sh` 在每次 SessionStart（含 compact 后）**重新注入**基因模板 + (c) 循环指令到 additionalContext → 基因/循环不丢失（bootstrap 存在的根本目的，见 hook 内 compact 注释） |
| **Lead 长任务期间子任务等待**（Lead 忙→子任务积压等下轮 scan） | **已知特性（非 bug，无 heartbeat/timeout）** | (c) 循环固有时延 = Lead 当前操作完成时间；无 Lead 心跳机制。若 re-scan 频率与 Lead 任务粒度严重不对齐（极长任务期大量积压）→ 吞吐下降，需观测（codex 边界条件 §3.3） |

**未实装 Lead heartbeat/timeout**——诚实声明（090）：(c) 模型当前依赖 Lead 主动 re-scan，无独立心跳保证子任务及时被消费。这是已知设计边界，非缺陷，但极端长任务下需观测吞吐。

### 3.1 (c) × 275 张力检查（任务边界明确要求）

任务边界：若 (c)（Lead 中心化 spawn）与 275（局部依赖）不可调和 → /escalate。**核查结论：可调和，不上浮。**

- **275 禁止的是**：(i) 全局优先级排序、(ii) 越级管理子蜂群、(iii) 中心化**依赖调度**、(iv) 全局工位评估。
- **(c) 做的是**：Lead scan → 为每个 unowned ∧ unblocked 任务**无排序**地 spawn → re-scan（033：Lead 是 DAG 解释器非决策者）。**依赖拓扑仍是局部的**——每个工位自己 TaskCreate 子任务 + blockedBy 边，全局 DAG 从局部创建中涌现（275 核心命题）。
- **分离**：(c) 中心化的是**物理 spawn 系统调用**（harness flat-roster 物理约束：只有 Lead 能 spawn），**不是依赖逻辑/排序**。275 约束依赖与排序层（保持局部），(c) 约束 spawn 执行层（被 harness 强制中心化）。两者**正交**——spawn 谁 ≠ 谁依赖谁。
- 故非不可弥合矛盾，是 harness 物理约束施加的"依赖局部 / spawn 中心"分工。**无 escalate。**

---

## 4. 下游推论

1. **genome 对齐缺口**：dispatch-dag.yaml stale 行（7/11/160/633/784）与 562 矛盾，须走 020-gated genome 修改（编排者批准）对齐。在此之前，genome_layer 文档与 structural_layer 实装（hook + skill）不一致——但这是 stale 文档非运行时冲突（hook/skill 是实际执行路径，dispatch-dag.yaml 是声明式索引）。
2. **(c) 三层一致**：(c) 路径现已一致编码于 skill（sub-swarm-ceremony）+ enforce hook（advisory 导向）+ bootstrap hook（(c) 循环 + 基因模板）——三层互证。dispatch-dag.yaml 是第四层（声明式），待 genome 流程补齐。
3. **基因机制化范式**：present-by-instruction（模板注入）+ advisory-detected（字符串检测）> post-hoc 启发式 block，确立为**自由文本类、无结构化字段**基因的可达机制化上限。可作为后续类似"声明强制"的参照（避免用误杀风险的启发式 block 冒充机制强制；并诚实标注其非 hard structural 的上限——090）。

---

## 5. 验证（L2 实测，7 场景）

`bash -n` 两 hook 全过。enforce hook 7 场景（真实 leadSessionId=938cba26…）：

| # | 输入 | 期望 | 实测 |
|---|------|------|------|
| 1 | 非 Agent 工具（Read） | 静默放行 exit 0 | ✅ |
| 2 | Agent(Explore) | advisory（096 优先 Glob/Grep） | ✅ |
| 3 | teammate 无 name 非 Explore（缺基因） | 基因 advisory + 递归块 advisory + **(c) 路径导向 TaskCreate（新）** | ✅ |
| 4 | teammate 无 name 含基因+递归块 | 仅 (c) 路径导向（无基因 advisory） | ✅ |
| 5 | **LEAD 无 name 非 Explore** | **BLOCK（deny）** | ✅ |
| 6 | LEAD 有 name+基因+递归块 | 干净放行（无输出） | ✅ |
| 7 | LEAD 有 name 缺基因 | advisory only（**不 block，fail-open**） | ✅ |

bootstrap hook 输出：valid JSON ∧ additionalContext 含基因模板（`spawn 两基因` ∧ `topo_address` ∧ `递归判断块`）= present-by-instruction 验证通过（template 注入存在性，**非**占位符填充语义）。

**认识论诚实修正（codex F2 表 line 120）**：上述 7 场景由 swarm-infra **自行设计**（输入是作者构造的，验证的是"管线按我设计的判据工作"）→ 严格说更接近 **L1（作者自证管线正确性）** 而非真正 adversarial L2。真正的 L2 否证性来自**异质**对抗——本修复的 L2 成分实际由 codex #72 的逐行异质审计提供（发现 F1 死变量 / F2 命名膨胀 / F3 文档缺口，皆我自测未覆盖）。故本工位的 L2 标注应理解为"L1 自测 + codex 异质 L2 复核"的组合，不是单方 L2。

---

## 6. 影响声明 + genome flag

**改动文件（交 Lead 决定 commit）**：

1. `.claude/hooks/agent-team-enforce.sh`：
   - header（§harness 真实机制 第4点）：把 teammate→teammate gap 从"未决 spec-execution gap（上浮编排者）"更新为"(c)裁决已结算的递归实现（TaskCreate 子任务→Lead spawn）"。
   - advisory 段：(a) 新增机制化边界注释（为何基因保持 advisory：自由文本→启发式 block 违反 fail-open）；(b) 简化递归块 advisory（删 stale "已上浮" 框定）；(c) **新增 (c) 路径 advisory**——teammate 无-name Agent → 显式导向 TaskCreate 子任务（metadata.agent_type）→ Lead spawn，并提示只读搜索用 Explore。
2. `.claude/hooks/agent-team-bootstrap.sh`：
   - 结构工位 spawn 块后新增"=== spawn 两基因 + 递归判断块（present-by-instruction）===" 段：注入基因模板（含占位符）到 additionalContext，指示 Lead 的每个 Agent prompt 携带两基因 + 递归判断块（137 机制化在无结构化字段下的可达形式；诚实标注非 hard structural）。

**未改动（明确不碰）**：
- `.claude/skills/sub-swarm-ceremony/SKILL.md`：已由 #41 完成 (c) 重写，复核干净，无需改动。
- `.claude/settings.json`：matcher 已为 "Agent"（#30 已修）。

### ⚠ genome flag（交 Lead 走 020-gated 修改流程，不擅改）

`.chanlun/dispatch-dag.yaml`（genome_layer，line 25/50；020-gated line 58）以下行与 **562号（结构=teammate，negates 075）** 矛盾，须经编排者 020 批准后修改：

| 行 | 当前内容（stale，075） | 应改为（562） |
|----|----------------------|--------------|
| 7 | `结构能力 = skill（事件驱动），不是 teammate（075号）` | 结构能力 = teammate（spawn，562 扬弃 075）；skill 层为轻量事件守卫并存 |
| 11 | `dispatch-dag 定义"事件→skill 映射"，不是"必须 spawn 的 agent 列表"（075号）` | 结构工位是 ceremony 必 spawn 的 teammate（562 + bootstrap auto_spawn）；event_skill_map 是按需 skill 层 |
| 160 | `事件→Skill 映射（075号：结构能力从 teammate 转为事件驱动 skill）` | 562 扬弃：结构=teammate；本节为按需 skill 层（非结构能力载体） |
| 633 | `075号：skill 是全局的，子蜂群不再 spawn 结构 teammates` | (c)裁决：子蜂群=子任务 DAG（TaskCreate）；结构 teammate 由 Lead 在 ceremony 必 spawn（562） |
| 784 | `运行时验证（075号更新：从 dominator spawn 检查改为 skill 可用性检查）` | 562：运行时验证 = Stop-Guard 检查 1.5（结构工位 spawn 检查），非 skill 可用性 |

> 任务消息指明 7/11/160/633-634；额外发现 **784**（同根 075 stale）。建议一并纳入 genome 修改批次。

---

## 7. 谱系引用

- **075→562 扬弃链**（结构工位 teammate↔skill）：本修复消费此链——562 把 075 的"结构=skill"扬弃为"结构=teammate"。dispatch-dag.yaml 是该链尚未传播到的最后一层（genome，020-gated）。
- **(c)裁决（2026-06-23，编排者）**：teammate→teammate gap 的递归解——从 #30 诊断的"未决上浮"结算为"任务拓扑递归（TaskCreate→Lead spawn）"。本工位把 (c) 传播到 enforce hook advisory（此前只有 skill + bootstrap 编码了 (c)）。
- **137/016/032/090**：机制化方法论——137 的机制化形式受被强制对象可结构化性约束（自由文本 ∧ 无结构化字段→present-by-instruction + advisory-detected，非启发式 block 避免 032 误杀；并按 090 诚实标注其非 hard structural 上限）。
- **073a/274**：spawn 两基因（topo_address, parent_callback；depth_budget 已废）。
- **原则15**：真递归默认（递归判断块）。
- **095/096/097**：Agent Team 真递归 + 无孤立 subagent + 五特征子蜂群（(c) 在新 harness 的落地）。
- **020号**：genome 自我保护——dispatch-dag.yaml 修改阻断等待编排者。
- **不确定是否有相关谱系的领域**：(c)裁决本身可能值得结晶为语法记录（"harness 物理约束 vs 蜂群架构应然"的调和模式，含 §3.1 的 (c)×275 正交性论证）——但应由 genealogist 工位判断，本工位仅 flag。

---

## 8. codex-challenger #72 异质审计处置（约束4 闭环）

异质审计判定 **CONDITIONAL PASS**（报告 `codex-audit-swarm-recursion-72-20260623.md`）。通过项：(c) 模型传播无活跃 stale 指令 / no-workaround（TaskCreate 是架构重构非补丁）/ genome flag 处理正确。3 发现全部处置（非阻断，但按 no-patch-mentality「完整一次修完」+ 090 处理）：

| 发现 | 等级 | 处置 |
|------|------|------|
| **F1** `lead_known` 死变量（enforce hook） | MINOR | **删除**（no-patch 第5条：无用代码直接删除不注释保留）。验证 0 refs。 |
| **F2** "present-by-construction" 命名精度（template 含占位符 + enforce 字符串检测≠语义有效性） | METHODOLOGICAL | **采纳**。enforce/bootstrap 注释 + 报告全文改为 **present-by-instruction + advisory-detected**，并诚实标注**无 hard structural 路径**（harness Agent 工具无结构化基因字段）。对齐 090（不声明代码不具备的能力）。 |
| **F3** (c) 循环时延未文档化 | ARCH-DOC | **采纳**。新增 §3.0a 文档化（已知特性，非 bug）。 |

F2 修正后实测复验：bash -n 全过，7 场景全绿（LEAD-block / clean-pass / (c)-redirect 不变），bootstrap 输出含 present-by-instruction 措辞、无遗留 present-by-construction 过度声明。

**第二轮：核对 codex 书面报告（team-lead 指向 `codex-audit-swarm-recursion-72-20260623.md`）后的增量闭环**（首轮基于 codex 内联消息，书面报告 F3 §1.5 更精确）：
- **F3 精确化**：书面报告区分"compact 后恢复=已覆盖（bootstrap SessionStart 重注入）"vs"Lead 长任务子任务等待=已知特性（无 heartbeat）"。→ §3.0a 补两场景区分表 + `agent-team-bootstrap.sh` 补 compact-recovery 注释（team-lead 明确请求）。
- **L2 诚实修正**：书面报告 line 120 指 7 场景=作者自designed≈L1。→ §5 补认识论修正（L1 自测 + codex 异质 L2 组合）。
- F1/F2 首轮已闭环（删死变量 / 命名修正），书面报告确认无新增。

---

## 结论摘要（给 Lead）

- 改 2 文件（enforce hook header+advisory / bootstrap 基因模板），7 场景实测全绿，建议 commit。
- skill #1 已由 #41 完成，无需改。
- genome flag：dispatch-dag.yaml line 7/11/160/633/784 stale（075↔562），交 Lead 走 020-gated 流程。
- 无矛盾上浮：(c)×275 正交可调和（§3.1）。
- 异质审计：codex-challenger #72 **CONDITIONAL PASS**，3 发现（F1 死变量删除 / F2 命名精度修正 / F3 时延文档化）全部处置闭环（§8）。
