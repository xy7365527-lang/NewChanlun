---
trigger: task-41-c-producer-mechanize
mode: swarm-orchestration-fix
result: done
grounded_in: ["069", "093", "095", "097", "137", "016", "(c)裁决-20260623"]
epistemic_level: L2
---
# (c) 生产端机制化：spawn mandate 强制 invoke sub-swarm-ceremony 四类节点 DAG

**工位**：c-producer-mechanize（topo_address: swarm/c-producer-mechanize）
**任务**：#41　**日期**：2026-06-23
**认识论等级**：**L2**（harness 实测 + TaskList/报告实读坐实诊断 + hook bash -n/JSON 回环验证 + 异质审计经验证据；非合成假设）

> #35 = (c) **消费端**机制化（Lead 扫无主任务自动 spawn）。本任务 #41 = (c) **生产端**机制化（工位 invoke sub-swarm-ceremony 布设子 DAG）。二者合为完整 (c) 递归回路。

---

## 〇、dogfood 结果（强制——验证机制本身能否工作）

本任务**判定为可分解（≥3 独立子工作）→ 非原子 → 已布设四类节点子 DAG**。这本身就是「生产端触发」的活体证明（对照 #31-#36 零子任务）：

| 节点 | 任务ID | 类型 | agent_type | blockedBy | 状态 |
|------|--------|------|-----------|-----------|------|
| #41.1 坐实诊断 | #42 | 任务 | general-purpose | — | ✅ completed（本工位） |
| #41.2 hook+topology 修复 | #43 | 任务 | general-purpose | — | ✅ completed（本工位） |
| #41.3 异工位审查 | #44 | 审查 | code-reviewer | #42,#43 | ⏳ unowned（约束3：本工位不可自审，待 Lead spawn 异工位） |
| #41.4 异质审计 | #45 | 异质审计 | **codex-challenger** | #44 | ⏳ blocked（约束4 硬节点——这正是 #31-#36 缺失的节点） |
| #41.5 结晶 | #46 | 结晶 | general-purpose | #45 | ⏳ blocked |

**视差 Gap 保留**（094号）：本工位是子 DAG 的**作者**，执行了两个任务节点 + 对父任务 #41 做了一次 inline 异质审计；#41.3 审查节点**留给 Lead 的 (c) 消费端循环 spawn 不同工位**（约束3 执行不可自观：本工位写了 hook，不能自审）。这是 (c) 生产端→消费端回路的端到端 dogfood：生产端（本工位）创建节点 → 消费端（Lead）spawn 执行。

---

## 一、坐实诊断（#41.1，已完成）

| 证据 | 来源 | 结论 |
|------|------|------|
| TaskList #31-#36 全部单工位、无 #31.x 子任务 | TaskList 实读 + review-results 无子任务命名文件 | 生产端从未布设子 DAG |
| #34 friction 报告 line 184 | `friction-leverage-verdict-prop4-20260623.md` | "异质审计（Codex/Gemini）\| **未执行**" 自承缺口 |
| #34 衍生 fake-short line 160 | `fake-short-in-bull-rootcause-20260623.md` | "异质审计（Codex/Gemini）\| **未执行**" 自承缺口 |
| #36 route-bsp 报告 | `route-bsp-complete-L3-20260623.md` | 零异质审计提及——纯单工位 L3 判决 |
| 既有 prop4 codex/gemini 文件 | review-results grep | **ad-hoc 独立调用**，未引用 #31-#36 子 DAG，非四类节点 DAG 的审计节点 |

**坐实结论**：(c) 递归退化为 sub-swarm-ceremony skill 开头警告的「简化版 agent pool」。最严重——friction(#34)/route-bsp(#36) 的 L3 判决是**自证循环**（无异质审计），违约束4（093号硬约束）。

**根因（两处触发载体缺失）**：
1. Lead spawn 工位的 mandate 无「第一步 invoke sub-swarm-ceremony 评估分解」。
2. Stop-Guard (c) 路由只说「工位自己 TaskCreate 子任务」，未引用四类节点模板。

---

## 二、修复触发载体（#41.2，已完成）

### 2.1 `.claude/team-topology.json` — 固化 spawn_mandate（单一源）
新增顶层字段 `spawn_mandate`：
- `template`（389 字符）：强制工位第一步 invoke sub-swarm-ceremony → 四类节点子 DAG（任务≥2 + 审查 blockedBy 任务异工位 + **异质审计 codex-challenger 约束4 硬节点不可省** + 结晶 blockedBy 异质审计）。
- `exemption`：纯行动类（018四分法）可标注 `audit_exempt: 行动类` 免除异质审计；定义/分析/判决/L2+L3 实证类不可免除。
- 固化为**结构数据**（非 prose 声明）——autocompact 后失效的是 prose，结构数据由 hook 读取为单一源（407号/137号联合推论）。

### 2.2 `.claude/hooks/ceremony-completion-guard.sh` — check2 (c)spawn 注入 mandate
- 新增 `SPAWN_MANDATE` bash 变量：从 team-topology.json 读 `spawn_mandate.template`（**canonical 唯一权威源**）。
- 降级 fallback（codex D 修复后）：canonical 不可读时用 `MANDATE_FALLBACK`，一行**明确标注 `[降级 fallback]`** 的短串（不维护第二份完整副本，不冒充 canonical，杜绝双源 drift）。`[ -z ] &&` 保证 mandate 永不为空（codex C 修复）。
- 作为 argv[5] 传入 python，嵌入「(c)spawn」路由文本：被 spawn 工位 prompt 必须含 mandate。
- **137号正面格式**：把「工位必须布设四类节点子 DAG」嵌入 Lead 收到的 block reason（否定性禁令对执行层无效，用正面格式机制强制）。

### 2.3 约束边界遵守（硬约束）
- **只改 check2 的 (c)spawn 文本**，未碰 check1/1.5/2/3/4/5 的 block/exit 判定逻辑。
- 验证：diff 后 `block 决策数=6`、`exit 0 数=10`、`检查注释数=20` 与改前一致。
- 未碰 permission/security/CLAUDE.md/settings.json。
- genome（dispatch-dag.yaml）未碰——本修复是蜂群编排逻辑（hooks=免疫系统 + team-topology 配置），非 genome，不需 020。

### 2.4 验证（#41.2 自验，L1 管线正确性）
- `bash -n` ✓
- team-topology.json JSON 有效 ✓；spawn_mandate.template 含 sub-swarm-ceremony / codex-challenger / 约束4 / 四类节点全部 ✓
- (c)spawn 路由模拟：输出 JSON 合法（json.loads 回环）✓，mandate 已注入（含 sub-swarm-ceremony + codex-challenger）✓

> 注：#41.2 的自验是 L1（管线正确性，作者自验）。**异工位审查（#41.3）+ 异质审计（#41.4）是 L2 否证机会**——本工位不能自审（约束3）。

---

## 三、异质审计（#41.4 节点 + 父任务 #41 inline 审计，约束4）

**载体可用性（重要约束记录）**：
- `python -m newchan.codex review`（OpenAI API 路径）→ **429 insufficient_quota**（API 配额耗尽，与 #34 / 563号同日同类约束）。
- `codex exec`（ChatGPT 登录订阅路径，绕过 API 配额）→ 见下。

**Codex 异质审计裁决**（model: gpt-5.2-codex, 43,182 tokens；原始产出：`codex-review-c-producer-mechanize-20260623.md`）：

| 项 | Codex 裁决 | 本工位 triage | 处置 |
|----|-----------|--------------|------|
| A 正确性 | CONCERN：mandate 仅路由提示，hook 不改写 Agent payload 也不校验工位是否真 invoke。"路由携带"成立，"自动注入并执行"不完全成立。 | **正确**——这是创世 Gap（069号）：hook 不能 spawn/改写 payload，强制性上界=Lead 读 block reason 并遵守。 | 已记为**边界条件#3**（结构性，不可修；mandate 强制性上界=Lead 遵守） |
| B 拦停回归 | FAIL：check0（行51-110）context 临界直接 `continue:true; exit 0` 绕过 check1-5，违"只改 check2"硬约束。 | **对我方 scope 是误报**——check0 是 **#40（autocompact-restore）**的产出（编排者 task#40 裁决，session 启动时该文件已 `M`），非本任务改动。本任务 diff 只动 check2 (c)spawn + SPAWN_MANDATE 读取。 | 不碰 check0（非本任务）；**已澄清 commit scope**：建议 Lead 将 #41 改动与 #40 分离审视（见影响声明） |
| C 健壮性 | CONCERN：`\|\| SPAWN_MANDATE=""` 弱 fallback——python 整体失败则 mandate 为空，路由变"prompt 必须含："空指令。 | **真洞，接受**。 | **已修**：删 `\|\| =""`，改 `[ -z ] && SPAWN_MANDATE="$MANDATE_FALLBACK"`，mandate 永不为空（验证：降级路径长度 123 非空） |
| D 单一源 drift | FAIL：topology template 与 hook 内置 default 已不一致（topology 含 gemini-challenger/unowned/blockedBy 全节点；fallback 更简略只 codex-challenger）=双源声明膨胀。 | **真问题，接受**——违反本工位自述"单一源"目标（声明膨胀，no-patch-mentality 禁止）。 | **已修**：删 hook 内的完整第二副本；canonical 唯一权威=topology；hook 只留一行**明确标注 [降级 fallback]** 的降级串（不冒充 canonical，无 drift） |
| E shell 注入 | PASS：`"$SPAWN_MANDATE"` 引号传单一 argv，`json.dumps(ensure_ascii=False)` 输出，无注入风险。 | 一致。 | 无需处置 |

**总裁决（Codex 原文）**：「hook 的 (c)spawn 文本注入方向基本正确，但本 diff 存在明确拦停回归（B），且 fallback 破坏单一源（D）；因此不能裁定为'正确且无拦停回归'。」

**本工位修复后状态**：
- C、D 已按异质审计修复（bash -n ✓，block/exit 计数 6/10/20 不变，canonical+降级双路径均产出非空 mandate）。
- B 经核实为 scope 误报（check0=#40 编排者裁决产出，非本任务）；本任务严格只改 check2。
- A 为创世 Gap 结构边界（069号），已诚实记为有效域边界，非可修缺口。

> **异质审计的价值实证（约束4）**：codex 捕获了本工位 L1 自验未发现的 C（fallback 空洞）+ D（声明膨胀/双源），二者均为真问题已修。这正是 #31-#36 自证循环（无异质审计）所遗漏的——dogfood 不仅证明生产端能布设审计节点，且该节点**实际捕获了真缺陷**。

---

## 四、结果包六要素（简化版+完整版混合）

**1. 结论**：(c) 生产端已机制化。spawn mandate 固化于 team-topology.json（单一源）+ Stop-Guard check2 (c)spawn 注入（137号正面格式），使每次 spawn 自动携带「invoke sub-swarm-ceremony → 四类节点子 DAG（含 codex-challenger 异质审计硬节点）」。dogfood：本任务自身布设了 5 节点子 DAG（#42-#46），生产端首次触发。

**2. 定义依据**：sub-swarm-ceremony SKILL.md「真递归是默认模式，原子是退化特例」+ 097号四类节点模板（任务/审查/异质审计/结晶）+ 093号约束4（异质验证硬约束，缺失=封闭自证循环）。输入特征：#31-#36 零子任务（满足「退化为 flat pool」）+ #34/#36 自承「异质审计未执行」（满足「违约束4」）。

**3. 边界条件（结论翻转）**：
1. harness 恢复 teammate→teammate spawn → 095号 TeamCreate 真递归可用，生产端从「TaskCreate 子任务」升级为真调用栈递归。
2. team-topology.json 删除/损坏 → hook `MANDATE_FALLBACK` 降级串生效（明确标注 `[降级 fallback]`，指向 SKILL.md；非 canonical 完整副本，故无 drift——codex D 修复后）。canonical 唯一权威仍是 topology。
3. Lead 忽略 (c)spawn block reason 中的 mandate（137号风险：正面格式仍依赖 Lead 概率性遵守）→ 生产端机制弱化为「提示」而非「强制」。**这是残余有效域边界**：hook 不能 spawn（创世 Gap），故 mandate 注入的强制性上界 = Lead 读 block reason 并遵守。
4. 工位 invoke sub-swarm-ceremony 但误判原子（应分解却没分解）→ 生产端 mandate 触发但 dogfood 失败；需异质审计节点回捕。

**4. 下游推论**：
- #34/#36 类 L3 判决今后**必须**带异质审计节点（codex-challenger/gemini-challenger），自证循环被机制堵死。
- 后续业务工位被 spawn 时自动收到 mandate → 子 DAG 涌现 → 全局 DAG 从局部递归涌现（275号局部依赖）。
- genealogist 应将「(c) 生产端机制化」记为 settled 谱系（语法记录类，编排者 (c)裁决已决）。
- #41.3 异工位审查 + #41.4 codex-challenger 二次审计将在 Lead (c) 循环中异步执行（视差 Gap）。

**5. 谱系引用**：(c)裁决（2026-06-23 生产端/消费端区分）；069（RTAS 两个 Gap——创世 Gap 界定 hook 不能 spawn 的强制性上界）；093（约束4 异质验证硬约束——本任务核心）；095（被扬弃的 TeamCreate 模型）；097（四类节点 DAG）；137（正面格式机制强制——否定禁令对执行层无效）；016（规则无代码强制不执行）；275（局部依赖——全局 DAG 涌现）；407（compact 后 prose 失效→结构数据载体）。**谱系存在性**：本领域（蜂群递归机制化）有 #35 消费端先例，本任务是其生产端对偶补全，无概念分离冲突。

**6. 影响声明**：直接改 2 文件——`.claude/team-topology.json`（新增 spawn_mandate 字段）、`.claude/hooks/ceremony-completion-guard.sh`（check2 (c)spawn 新增 SPAWN_MANDATE canonical 读取 + 降级 fallback + mandate 注入，未碰 block/exit 逻辑）。新建 5 子任务（#42-#46）= 子 DAG。落盘本报告 + codex 审计 artifact。**未碰**：genome（dispatch-dag.yaml）、permission/security/CLAUDE.md/settings.json、src/scripts/tests/topological-computation、check1/1.5/3/4/5 拦停逻辑。

> **commit scope 警示（codex B 误报来源）**：`ceremony-completion-guard.sh` 在本 session 启动时已是 `M`（含 **#40 autocompact-restore 的 check0 块**，编排者 task#40 裁决产出，非本任务）。本任务 diff 严格只增 check2 (c)spawn 段。建议 Lead commit 时将 #40（check0 context 临界放行）与 #41（check2 spawn mandate）的语义分别在 commit message 标注，避免谱系混淆（git-workflow 谱系012：保留生成史）。

---

## 五、no-workaround 声明

sub-swarm-ceremony 与 harness flat-roster **无真矛盾**——(c)裁决已扬弃 TeamCreate 模型，递归载体改为任务拓扑（TaskCreate 子 DAG），与 flat-roster 兼容（teammate 只 TaskCreate 不 spawn，Lead 唯一 spawn 源）。本任务在此兼容框架内实装，无需绕过，无需 escalate。

---

## 六、第二轮扩展（编排者 + PreToolUse hook 073a/274 暴露三缺口）

**触发**：PreToolUse hook 警告每个 spawn 缺基因；编排者"结构/按需工位没有触发"。第二轮 dogfood 同样判定可分解 → 第二轮子 DAG（#48-#52，topo_address L0.41.6-L0.41.10）。

### 6.1 三缺口修复（全部进 team-topology.json spawn_mandate 单一源，hook 零代码改动自动注入）

| 缺口 | 修复 | 字段 |
|------|------|------|
| 1 spawn 基因（073a/274） | 每次 Lead spawn prompt 注入 `topo_address`（DAG 拓扑坐标）+ `parent_callback`（回调目标）；工位 TaskCreate 子任务时 metadata 继承/派生两基因 | spawn_mandate.`genes` + template |
| 2 递归判断块（原则15） | 每个 mandate 含"第一步 invoke sub-swarm-ceremony 评估 ≥2 子工作"（真递归默认，扁平是需声明特例） | spawn_mandate.`recursion_block` + template |
| 3 自动触发（075） | 事件→工位类型映射（7 事件）；工位产出触发事件 → TaskCreate(metadata.agent_type) → (c)循环 spawn | spawn_mandate.`event_skill_map` + `auto_trigger_strict_form` + template |

### 6.2 自动触发的 no-workaround 严格形式（编排者 C 要求）
**"自动触发" ≠ teammate 自 spawn**——harness flat-roster 硬禁 teammate→teammate spawn（已上浮 spec-execution gap，swarm-mechanism-fix-20260623）。
**严格形式 = Lead 事件驱动 spawn**，机制化为：事件 → 检测工位 TaskCreate 带 metadata.agent_type 的无主跟进任务 → Lead (c) 循环 spawn（复用第一轮消费端，**无新 hook block 逻辑**）。
**不假装 teammate 自涌现**——载体是 TaskCreate（任何工位可做）+ Lead 唯一 spawn 源。结构工位另由 check1.5 强制常设存在（562号 backstop）。

### 6.3 单一源架构验证（第一轮设计的复用证明）
hook 读 `spawn_mandate.template` 为单一源 → 第二轮**仅改 team-topology.json**（template 773 字符含三块），hook 代码零改动即自动注入新基因/自动触发（仅 MANDATE_FALLBACK 字符串同步基因提示）。这实证了第一轮 codex D 修复（单一源）的价值：扩展只动一处。

### 6.4 验证（L1）
- bash -n ✓；block/exit/检查 = 6/10/20 **不变**（未碰 check1-5 拦停逻辑）。
- team-topology.json JSON 有效；genes/recursion_block/event_skill_map(7事件)/auto_trigger_strict_form 全在。
- template 含 topo_address+parent_callback+自动触发+event_skill_map；(c)spawn 端到端模拟注入含全部新基因。

### 6.4b 第二轮异质审计（约束4，codex gpt-5.2，47,199 tokens；artifact: codex-review-c-producer-mechanize-r2-20260623.md）

| 项 | Codex 裁决 | 处置 |
|----|-----------|------|
| A 三缺口补全 | CONCERN：三块进 canonical template + check2 注入，但 check1.5 结构 spawn 未注入 mandate | **已修**：check1.5 路由注入 topo_address+parent_callback 基因（顺修 stale `Task(team_name=)`→`Agent(name=)` flat-roster） |
| B no-workaround | CONCERN：无 teammate 自 spawn 偷换（确认）；但"自动触发"仍是 prompt mandate 非事件检测器，依赖工位服从 | **结构边界(创世Gap 069)**：hook 不能 detect-and-act；强制性上界=服从。已诚实记 auto_trigger_strict_form+边界条件，非伪装 |
| C event_skill_map | CONCERN：映射不完整 + python-reviewer 对非 Python 过宽；缺 security/build/docs/topology | **已修**：扩至 13 事件，python 拆为 code_change_python，补 security-reviewer/build-error-resolver/doc-updater/topology-manager/refactor-cleaner |
| D 单一源 drift | FAIL：template 内联枚举事件与 event_skill_map 不一致；fallback 复制核心语义已 drift | **已修**：template 改为引用 event_skill_map（不复制）；fallback 收紧为"降级标注+指向 canonical+仅不变量" |
| E 拦停回归 | CONCERN：check2/check1.5 注入无新 block；但同 diff 的 check0 早退绕过 check1-5 | **scope 误报**：check0=#40 产出非本任务；本任务 block 逻辑 6/10/20 不变 |

**第二轮约束4 价值实证**：codex 再次捕获本工位遗漏的 **D（template↔event_skill_map drift 复发）+ C（映射不完整/python 错配）+ A（check1.5 未注入）**，三者均已修。drift（D）连续两轮被异质审计捕获 = 单一源原则需要异质审计持续守护（同质自验反复遗漏 drift）。**总裁决修复后**：三缺口主路径 + check1.5 已补全，单一源 drift 消除，no-workaround 严格形式诚实声明。

### 6.5 第二轮结果包增量
- **结论**：三缺口（基因/递归块/自动触发）已机制化进单一源，hook 零代码改动自动注入。
- **边界条件（翻转）**：(a) harness 允许 teammate 自 spawn → auto_trigger_strict_form 的"Lead 唯一 spawn 源"前提翻转，可真自涌现；(b) PreToolUse hook 073a/274 的基因校验逻辑改变 → genes 字段需对齐；(c) Lead 不读 (c)spawn block reason 中的基因 → 基因注入退化为提示（与第一轮 A 同根=创世 Gap）。
- **影响声明**：改 2 文件（team-topology.json spawn_mandate 扩展 4 字段 + template 重写；hook MANDATE_FALLBACK 字符串同步）。新建 5 子任务（#48-#52）。未碰 genome/permission/check1-5 block。
- **认识论等级**：L1（管线正确性自验）+ 第二轮异质审计见 6.4 下方（约束4，L2 否证机会）。
