# 独立审查：codex/decision-archaeology-gate-20261020

**总结论：REQUEST_CHANGES**（无 HIGH；4 条 MEDIUM 需改后合入，主要是「C 类 decision 推荐值可空」与提示自相矛盾、考古解析对畸形输出会直接抛错绕过重试、C 类 research/probe 缺口无人执行且不阻 T 类落地、升档重跑考古不带反馈）。

测试：`node --import tsx --test .sandcastle/decision-claimer.test.ts` → **56 tests / 56 pass / 0 fail**。

> 注：`origin/main..HEAD` 两点 diff 里几十个 `.chanlun/**` 删除是 origin/main 已前进（merge-base `bc401fc5`）所致，本分支实际改动只有 5 个文件（`git diff origin/main...HEAD --stat`）。合入前需 rebase。

## 需求逐条

| # | 结论 | 证据 |
|---|---|---|
| 1 考古阶段 + archaeology.md 逐点 查询词/命中/归类，A/B/C 处置口径 | **PASS** | `decision-claimer.mts:1138`（分级后 `runArchaeology`）；`:712-740` 机械校验两路、`--state all`、A/B 出处、B 默认值、C 关键词；`:745-765` `renderArchaeology` 逐点表；`:873` 起草硬要求 9「只有 C 类 decision 列为真正待决；B 类……确认或否决」。prompt `:807-811` 两路命令覆盖正本层/推论层/代码常量/夹具。 |
| 2 R8 | **PASS** | `RULES` 含 R8（`:36`）；`reviewPrompt:911` 写明「抽查至少三条命中并复跑」「A 当待决」「B/C 缺查询证据」「P 类凭空要数值而在案先例未列」均 BLOCK；`graph-engineering.md:67`。机械侧 `:1179-1183` 把考古问题与「考古对照」漏点并入 BLOCK。 |
| 3 SKILL / graph-engineering / wayfinder 指针 | **PASS（带 MEDIUM-1 的保留）** | `graph-approve/SKILL.md:13` 只问 B/C、每问附出处与推荐值、引用 `/grilling` 分工；`graph-engineering.md:28-45,76`；`wayfinder-workflow.md:113` 指向「2. 考古」。但代码侧 C 类推荐值可空（见 MEDIUM-1），SKILL 承诺的「推荐 <值>」无法保证。 |
| 4 测试可跑；`--archaeology-only` 对已关票验形状 | **PASS** | 56/56；`parseArgs:93` 守卫只允许 dry-run+单轮+--ticket；`archaeologyOnly:1061-1068` 不走 `resolvedReason`、不写 summary；测试 `decision-claimer.test.ts:893-910` 用 `state: "CLOSED"` 票验证且 0 次 gh 写。 |
| 5 复用 Matt Pocock skill，只补胶水 | **PASS** | `archaeologyPrompt:800-804` 让子代理先读 `grilling/SKILL.md`（已核 `:10` 原句存在）、`domain-modeling/SKILL.md`（`:44,:56` 两节存在）、`wayfinder/SKILL.md` Ticket Types（`:73-77`）、AGENTS.md `:155` 两路搜、ADR 0012 `:64,:130`。胶水 = A/B/C 归类 + JSON 形状 + 机械校验，没有另造 grilling 流程。 |

## 发现

### MEDIUM

**MEDIUM-1 C 类 `decision` 推荐值不强制，三处口径自相矛盾**
- `decision-claimer.mts:818`（JSON 说明「recommended：B 必填，C 可选」）、`:737`（校验只对 B 查 `recommended`）vs `:873` 起草硬要求 9「只有 C 类 decision 列为真正待决，每问附推荐答案」vs `graph-approve/SKILL.md:13`「C 类问『推荐 <值>（推导依据）』」vs `/grilling` 原则「每个决定附推荐答案」。
- 后果：`archaeologyAsks:838` 对缺值的 C 点打印「见草稿「候选」」，用户在影子评论里看到的是指针不是值——正是需求 2「要求用户凭空提供数值」要挡的形态（测试 `:874` 甚至把这当正例断言）。
- 建议：`archaeologyProblems` 增加 `p.klass === "C" && p.gap === "decision" && !p.recommended → "C 类 decision 缺推荐答案"`；prompt 改「C decision 必填」；删掉 `|| "见草稿「候选」"` 兜底（它本身也是 R5 意义上的失败兜底）。

**MEDIUM-2 `parseArchaeology` 对畸形 JSON 抛 TypeError，绕过「带问题重跑一次」**
- `:689-706`：`points:[null]` 或 `hits:[null]` → `Cannot read properties of null`（已实测）。异常从 `runArchaeology` 直冲 `processTicket` 的 catch → `phase: "error"`，不是设计中的「解析不出 → 带问题重跑一次」。
- 建议：`map` 前过滤非对象元素（`points.filter(p => p && typeof p === "object")`，queries/hits 同理），或把 `parseArchaeology` 包 try/catch 返回 `null`。补一个畸形输入测试。

**MEDIUM-3 C 类 `research`/`probe` 缺口没有执行者，也不阻 T 类落地**
- `:873` 要起草者把缺口「写进受影响票与 blocking 边变化，作为要开的 research 票或探针票」；`graph-engineering.md:42` 说「开 research 票由 /research 子代理解决」；`SKILL.md:13` 说「开票补」。但代码没有任何一处开票（`plannedWrites`/live 写只有 comment/label/assign/branch），`nextAfterReview` 也不因 C-research/probe 存在而拦截 T 类 `land_t`。按 wayfinder `SKILL.md:77`，research 票是「a fact a decision waits on」——决定等事实，这里却是事实缺着、裁定先落正本。
- 建议二选一并写进 graph-engineering.md：(a) 考古有 `gap ∈ {research, probe}` 时 T 类自动降 P（或 `nextAfterReview` 置 `shadow_p`），由 `/graph-approve` 人开票；(b) 裁决面在 live 下按 wayfinder 口径开子票并加 blocking 边，T 类落地前等其关闭。当前「写进清单」是无人消费的 no-op。

**MEDIUM-4 升档重做时重跑考古不带上一次问题、不升档、覆盖时长**
- `:1150`：`redo` 时 `ar = await runArchaeology(t, d, dir, summary)`，`fb` 从 `undefined` 起（`:1021`），两次机会里第一次是盲跑；模型固定 `MODEL_FABLE`（`:1026`）而起草者已升 Opus；`summary.durationsMs.archaeology1/2` 被覆盖（`:1027`）。测试 `:844` 只断言次数 4，没断言第三次 prompt 含反馈。
- 建议：`runArchaeology(t, d, dir, summary, initialFeedback?, model?)`，redo 时传上轮 `ar.problems` 与 `s.drafterModel`；时长键按轮次计（`archaeology${attempt}_${i}`）。

### LOW

**LOW-1 tracker「覆盖已关闭票」校验有漏洞**
`:710` `TRACKER_ALL_STATES = /--state[\s=]+all|gh search issues/`：`gh search issues "x" --state open` 通过校验（已实测返回 `[]`）。建议：`gh search issues` 分支额外排除 `--state\s+(open|closed)`。

**LOW-2 同一段设计理由三处重复（writing-for-agents「单一事实源」）**
`graph-engineering.md:30-34`、`decision-claimer.mts:660-671` 注释、`archaeologyPrompt:800-804` 三份几乎相同的 grilling/domain-modeling/ADR 0012 引文。文件头 `:5` 已声明「设计正本：graph-engineering.md」。建议：代码注释缩成一行指针；prompt 保留（它是给子代理的 context pointer，属合理重复）。

**LOW-3 `reviewPrompt` 的「本票没有 archaeology.md」分支不可达**
`:917`：`processTicket` 永远传 `ar.md`，考古失败早已抛错；R8「考古缺失按 BLOCK」只在直接调用 `reviewPrompt` 时才可能触发。建议把 `arch` 改为必填参数，删掉分支。

**LOW-4 R8 在无 C 类点时的指令悬空**
`:911`「另自选一个 C 类点换一个说法再搜一次」——全 A/B 票时审查者无法执行；建议改为「对任一 B/C 点换说法复搜」。

**LOW-5 `landPrompt` 把「考古对照」表连同草稿整段写进正本**
`:930-936` 原样贴草稿；原先「引用自证」已有此问题，本次再加一段流程产物。建议 landPrompt 明示「只写七段，`引用自证`、`考古对照` 不入正本」，或落地前裁掉。

**LOW-6 `graph-approve/SKILL.md:13`「票上没有 archaeology.md 时先退回补考古」缺动作**
现有 `needs-approval` 存量票都没有考古；SKILL 没说怎么补。建议补一句：`node --import tsx .sandcastle/decision-claimer.mts --archaeology-only --ticket <号> --out /Volumes/AgentStorage/decisions`，并把生成的问表贴回票。

**LOW-7 `archaeologyOnly` 不取 `.lock`**
`:1061` 与 `processTicket:1076` 同目录写 `archaeology.md/json`，并行时互相覆盖。建议复用同一把锁或写到 `archaeology-only/` 子目录。

**LOW-8 续做路径对旧票静默无考古**
`:1256` `loadArchaeology(dir)` 读不到即 `null`，影子评论不带问表且无提示；与 R8「考古缺失按 BLOCK」口径不一致。建议 resume 且无 `archaeology.json` 时在影子评论首行写「本票无考古（旧流程），请先 `--archaeology-only`」。

## 正确性结论摘要

- 状态机：`nextAfterReview` 未动，考古问题只经 `extra` 并入 BLOCK（`:1179-1190`），路径正确；缺的是 MEDIUM-3 的 C-gap 门。
- 续做：`resume` 分支跳过考古、`loadArchaeology` 只读 `parsed:true` 文件，安全；见 LOW-8。
- dry-run 守卫：`--archaeology-only` 在 `parseArgs` 层拒 live/loop，`guardGh` 继续生效，测试证 0 写；新增 READ_TOOLS `gh issue list`/`gh search issues` 均只读。
- 解析健壮性：MEDIUM-2 + LOW-1。
- 提示自相矛盾：MEDIUM-1（C 推荐值）、LOW-3/LOW-4。
- 孤立机制：未发现另造 grilling；A/B/C 与 JSON 形状属最少胶水。唯一偏离既有 skill 的是 C-research/probe「写进清单」却无 wayfinder 开票动作（MEDIUM-3）。
- 文档风格：正向陈述为主；重复见 LOW-2；no-op 见 MEDIUM-3 的「写进清单」句。