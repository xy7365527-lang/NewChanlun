# Graph engineering 流程（wayfinder → 裁决 → Sandcastle → 合入）

本文定义 #1323 这类大图从决策票到合入 main 的自动化流程。wayfinder 规则以 `docs/agents/wayfinder-workflow.md` 为准；本文只补它 v1 明写未覆盖的一层：**决策票与 SPEC 的代理裁决**（该文「v2 观察项」的落地；#1909 起不再有影子待批）。

## 五个面

| 面 | 组件 | 节奏 | 管什么 |
|---|---|---|---|
| 控制面 | `scripts/wayfinder_engine.mts`（已有） | 4 分钟 | 扫图、给合格实施票挂 `sandcastle` |
| 裁决面 | `.sandcastle/decision-claimer.mts`（新） | 10 分钟 | grilling 票：每票 MAIN 会话分级+考古并续接起草 → 异家族独审 → PASS 落地，否则 held |
| 执行面 | `claim-loop.mts` + `main.mts`（已有） | 60 秒 | 实施票：实装工蜂 + 独审工蜂 |
| 验收面 | `harvest.sh`（#1660 PR 模式）+ main 合入人闸 | 15 分钟 | 评审完成的分支推远端、基于 main 开 PR、票面登记；不合入不关票 |
| 观测面 | `health-monitor` + 进度页（新） | 5 分钟 / 1 分钟 | 停摆即 _red_、自愈、进度 |

普通研发流程的人闸只剩合入 main；真实资金、交易激活和凭据权限仍按 ADR 0041「授权来源与生效边界」取得对应授权。spec 与 P 类决策同 T 类一样，起草后经非作者独审 PASS 由代理裁定并记账（#1909，用户直接授权）；BLOCK 返修，缺有效独审不放行。

### #1742 执行面依赖（PR #1915）

保留 DeepAgents。本文的普通 SPEC 代理裁定合同是目标准入合同；现役 `deepagents-pilot/pilot1742/executor.py` / `approval.py` 仍有 SPEC_AUTHORIZATION 用户签署门，迁移由 Queued owner 的独立依赖 PR 交付。PR #1915 只交付控制面写前重验与文档，不能单独部署；依赖 PR 合入、精确集成版本通过首次队列执行验收后，才可声明新 PASS 贯通执行器。

## 裁决流程（每张 grilling 票）

### 1. 分级

主控模型 `gpt-6.1-sol` 读票面与关联票，判一类，并在票上评论一行分级理由：

- **T 类（技术取舍）**：数据结构、接口与 seam、owner 落点、状态机与事件 schema、测试锁、迁移步骤、恢复语义。有权威源或本仓判例可援引。resolution 首行「本条依 #1323 Notes N-1 预授权判定」。
- **P 类（政策）**：资金与费用参数（本金、量档、cap、分配）、风险偏好、真实场所/账户/权限、关图与 #1279 去向、Destination 改写、产品 taste、本仓无先例的新取舍。resolution 首行「本条依 #1909 代理独审裁定」。

两类走同一条起草—独审—落地链（#1909）；分级只决定授权引用与 R4/R7 的审查口径，不再增加普通人工审批。P 类中的资金/账户/权限议题可裁定方案，不能因此代签实际外部操作。拿不准一律判 P。完成标准：票上有 `分级：T|P — 理由` 评论。

### 2. 考古

用户 2026-10-20 令：「所有的裁决票都要先考古」。考古是三条既有规矩在无人值守裁决面上的执行，不另立机制：

- `/grilling`：「If a *fact* can be found by exploring the environment … look it up rather than asking me. The *decisions*, though, are mine」——事实由代理查完，决定才问人，每问附推荐答案。
- `/grill-with-docs`（= `/grilling` + `/domain-modeling`）：对照 `CONTEXT.md` 与 `docs/adr/`，已裁的直接用，与词汇表或代码冲突的当场指出。
- AGENTS.md「开之前先搜（两路）」与 ADR 0012 裁定一、四：tracker 含已关闭票与评论，仓内含推论层；推论层主张可援引，推理链搬进票内重过。ADR 0012 代价第 4 条「每张裁定票前多一个 AFK 子代理」即本步。

无人值守下没有人能当场纠正「这个早定过了」，所以胶水只补一件事：把查过的证据落成可复核的文件。主控模型 `gpt-6.1-sol` 只读取证，逐个待决点（Q1、Q2…）两路各至少一条查询，归一类：

- **A 已裁**：正本层或已关票 resolution 明写。写出处，直接沿用，不再列为待决。
- **B 有先例未正式裁**：推论层、票评论、代码常量或夹具在用。写出处与推理链，作为默认候选，以「默认值，确认或否决」呈现。
- **C 确未设计**：列出已搜关键词，标缺口去向——`decision` 由起草者按推荐答案裁定，必带推荐答案及推导依据；`research` 是仓外事实，按 wayfinder 开 research 票由 `/research` 子代理解决；`probe` 缺读数推不出推荐值，按 `wayfinder-workflow.md` 反面判据③派探针票。

决定等事实（`/wayfinder` Ticket Types：research 票「surface a fact a decision waits on」）：考古有 `research`/`probe` 缺口的票（T/P 同）不起草、不落地，进 `held`（`fact_gap`），票上列出要开的缺口票；主控按 `/graph-run` 第 3 步开票并接 blocking 边。

机械校验：两路缺一、tracker 未覆盖已关闭票、A/B 缺出处、B 缺默认值、C 缺关键词或去向、C 类 `decision` 缺推荐答案，都带问题重跑一次（重做时仍用主控模型重跑）；仍不过的问题交审查，草稿因此 BLOCK。

分级与考古由主控模型一次联合调用完成（#1742 M1，角色 `triage`），输出同一 JSON 里的 `class`/`reason` 与 `points`，仍分别落 `classify.json` 与 `archaeology.json`；分级判据与「拿不准一律判 P」不变。考古重跑（机械校验不过、重做）沿用已得分级，不再分级。上次已分级而考古未完成（含旧两段式中断）时，下一轮在 MAIN 会话仍有效时读回 `classify.json` 只补考古；没有 MAIN 记录的旧两段式断点保留原恢复方式。分级评论票上已有即不重发，认领在自己名下不重复认领。

独审指出考古路径、行号或命中表述错误时，已解析且有依据的 **R8 BLOCK** 会在既有 `redo` 轮内回流给同一 MAIN 会话的 `triage`：固定当前 T/P 分级，携带 R8 原文和机械问题，宿主解析返回值后更新考古原件，再交起草和下一轮独审。缺 R8、R8 缺依据、解析失败或仅其他规则 BLOCK 不触发此回流；已有机械不合格返修保留原恢复方式。语义返修不新增重做额度，不开放模型 Edit/Write；两轮 BLOCK 后的终态仍须走正式退回与归档流程，不提供终态同 session 追加轮次。

考古版本保存在 `archaeology.attempt<N>.json/md`（N 为产生该版本的起草轮号），旧版本与旧审查不改写。R8 返修另在 `archaeology.repair<N>/` 保存每次调用的冻结 intent、返回回执和新原件。`main-rounds.json` 先绑定完整 json/md 版本，再发布顶层别名；恢复时从检查点补齐别名，下一轮 review 使用该版本。回执已保存而原件发布中断时复用回执，不重复模型调用；仅有 intent、尚无已保存回执时停止自动续接，不以自动重试消耗第二次调用。若 runner 已抛错，宿主另存 `call<N>.error.json`：观察时间、调用序号、intent 路径、固定原因枚举（限流、超时、网络、护栏、其他上游临时错误或其他）和沿用既有 `isRetryable` 判据的布尔值；不存任意原始错误文本。恢复错误区分“已观察到调用失败”与“无回执、结果未知”，均给出 call 序号和 intent 路径。检查点仍沿用原重做与引用专修额度。这些返修原件随 MAIN 会话一起退役留档。

机械不合格返修在 `archaeology.mechanical<轮号>-<代号>/` 缓存返回回执；别名发布后、检查点提交前中断，复用同代回执恢复 json/md 与检查点，不再调用。已提交一代但仍机械不合格的恢复可开始下一代；无回执的机械失败仍沿用原有恢复条件，不套用 R8 的未知结果停点。旧检查点没有绑定考古来源时，现存别名仅标 `origin: legacy`、`attempt: null`，保存为 `archaeology.legacy.json/md`，不能推定它属于第1轮。

回执以首次落盘的 intent 为审计依据：恢复校验 model/role/cwd，不因票面新增普通分级评论导致 prompt 重算不同而重发；R8 intent 的冻结提示保持原样，不改写为当前提示。

完成标准：`/Volumes/AgentStorage/decisions/<票号>/archaeology.md` 与 `archaeology.json` 落盘，每个待决点有查询词、命令、命中与归类。只验考古：`node --import tsx .sandcastle/decision-claimer.mts --archaeology-only --ticket <号> --out <目录>`（不看票是否已关，不写 GitHub）。

### 3. 起草

起草不另起上下文（#1742 F4）。分级考古所在的本票 MAIN 会话经宿主校验分级后，用 Claude CLI `--resume <id>` 续接起草，上文的工具调用与结果都留在历史里；首稿用该会话当前模型（默认 `gpt-6.1-sol`）。会话记在 `<票号>/main-session.json`，绑定票号、取证 `sourceCommit` 与开会话时的评论水印；取证提交变化、记录损坏或水印后出现退回时旧会话改名 `main-session.retired-<k>.json` 作废，对应 `classify.json`、`archaeology.json` 与 `archaeology.md` 整组以 `.retired-<k>` 后缀留档，另起新会话重新联合分级考古，不在新检出上续接旧历史。每次重新联合分级都在模型调用前撤下旧组；新分类已落盘而考古写入前中断时，只补本轮考古，不拼接旧考古。起草中断后重入且会话有效时，沿用已落盘分级与考古，在同一会话继续起草。`main-rounds.json` 绑定同一会话保存轮次、重做与引用专修额度、反馈、草稿和独审阶段；独审中断只续审已完成草稿，解析失败不计 BLOCK，也不重发额度。轮次账本随 MAIN 退役，已完成的每轮草稿与独审分别保存在 `draft.attempt<N>.md` 和 `review.attempt<N>.json`；检查点损坏或旧会话已有轮次却缺检查点时保留原件、停止自动续接。live 认领前先校验本票 MAIN 会话；若上次取消认领失败，仅当前唯一 assignee 是本人且存在有效恢复记录时允许续做，不重加 assignee。其他人的认领或别票会话不能据此放行。模型工具仍只读，产出固定结构的 resolution 草稿：问题、权威依据、候选、裁定、验收清单、受影响票与 blocking 边变化、授权引用。

权威依据逐条带可回查位置：`docs/chanlun/text/blog/<课>.md:<行>`、ADR 路径、票评论链接。引原文按 AGENTS.md 四步核行号、逐字、作者归属、行内注。验收清单每项必须是命令、测试名、文件存在或票状态之一。

遇缠师原文无定论、有多个可行候选的待决点（`[新缠论:选择]` 或参数类），按 [ADR 0041](../../docs/adr/0041-degrees-of-freedom-configurable-not-excluded.md) 优先裁为「可调字段 + 默认值」：落进具名版本化记录，写默认值与出处、可选值及代价，改值即新版本；另一种合法取值保留为可选值。已裁定签署前不得代填的字段（资金、量档、容量、成本门 θ、风险阈值等），默认值写 `AwaitingPolicy`（不签不跑、无备用值），数值只以「推荐值＋出处」列进签署单；本票正是普通研发字段的授权票时，按考古出处与推导给出裁定值并交独审核对，来源写代理独审裁定，不写 UserSigned。真实资金、交易激活与凭据权限的签署仍须对应用户授权；此边界同样约束 decision-claimer 起草提示第 11 条的「签署票」用语。违原文、违已裁推论、被实测否证、属 #799 兜底或与现行判据构成同一判断宽严两档的候选照旧排除。

起草按考古处置待决点：A 类沿用；B 类采纳默认值，否决须写依据与替代值；C 类 `decision` 按推荐答案的推导给出裁定值。草稿本身就是裁定，不把任何待决点留给用户。草稿末尾附「考古对照」表，每个考古点号一行。覆盖检查使用 CommonMark 语法树，保留三级等从属子标题、合法嵌套列表和段落续行，在同级或更高级 ATX/setext 标题结束。节外文字、HTML 注释、真实缩进代码块和 fenced 代码块（包括引用内代码与 `canonical_edit`）中的 Q 编号不补覆盖。

七段与流程表之后只附一个 `canonical_edit` 代码块，且只声明一个 `正本落点：<路径>`。目标仍限 `.chanlun/definitions/`、`docs/adr/` 或 `CONTEXT.md`，新建和修订都可用。

新建文件给完整最终正文：

```canonical_edit
{"version":1,"operation":"create","path":"CONTEXT.md","content":"# 词汇\n\n## 词\n定义。\n"}
```

修订已有文件先读原件，再给精确替换：

```canonical_edit
{"version":1,"operation":"edit","path":"CONTEXT.md","replacements":[{"before":"旧定义。","after":"新定义。"}]}
```

`before` 非空且在每一步当前文本中唯一命中，替换按序执行；未改章节、换行和末尾字节保留。`create` 只用于不存在的文件，`edit` 只用于已存在文件。正文不得夹入「引用自证」「考古对照」或编辑合同。宿主不把 resolution 整稿追加进正本，也不做模糊匹配。教义正文的三行头、日期、票号与受影响代码清单由起草者写入，交独审核对。

新 ADR 起草用 `NNNN` 占位，宿主在独审前分配真实编号并同步草稿与编辑对象；已有 ADR 修订保留原编号。宿主对固定 `sourceCommit` 原件生成 `CanonicalPlan`（`edit/sourceCommit/before/after/diff`）。T/P 两类的这份编辑在独审 PASS 后都由宿主落地。

完成标准：草稿文件落 `/Volumes/AgentStorage/decisions/<票号>/draft.md`，七段、两张流程表与唯一编辑对象齐全，最终编辑可确定性应用。

### 4. 独审

审查者另起无会话进程（`--no-session-persistence`，不带 MAIN 会话 id），只拿草稿、`archaeology.md`、源文件以及宿主生成的最终 Git diff 和完整 JSON 转义 `before/after`，看不到起草推理。起草与独审必须属于不同模型家族，独审模型按该轮实际起草模型选择（首稿 GPT 则 Opus 审，Opus 写则 GPT 审）；按 R1–R8 逐条给 PASS 或 BLOCK：

1. R1 每条引用回查成立。
2. R2 不与 `.chanlun/definitions/`、`docs/adr/`、#799 收敛通则、#804 总缝规则、ADR 0041 自由度通则冲突。把原文无定论的合法候选当错误排除或写成常数、未落为带默认值的可调字段，判 BLOCK；把原文语义或已裁推论改成可调、把同一判断的宽严两档做成可调字段、或让可调字段运行中换值，也判 BLOCK。把代理裁定记成 UserSigned、或把普通 P/SPEC PASS 当成真实资金/交易/凭据权限，按 ADR 0041 同样判 BLOCK。
3. R3 验收项全部可执行、判据二值。
4. R4 不越出 Destination；T 类草稿里没有夹带 P 类取舍。核对实际正本 diff、完整原件和结果：编辑与裁定一致，三行头、日期、代码清单正确，无关章节未丢失；合同缺失或正文不一致即 BLOCK。
5. R5 不新增同一判断的宽严两档或失败兜底。
6. R6 受影响票与 blocking 边清单完整。
7. R7 分级正确。
8. R8 考古成立：抽查 `archaeology.md` 至少三条命中并复跑查询，另对任一 B/C 点换说法复搜。草稿把 A 类当待决、把待决点留给用户批复而不裁定、B/C 类缺查询证据、「考古对照」漏点，或 C 类 decision 的裁定值没有推导依据而在案先例未列出，判 BLOCK。

独审者的 PASS 放行本票获审文书与普通研发实施，不再等待普通人工批准；实际资金/交易/凭据权限仍适用 ADR 0041 的独立硬门。

完成标准：`review.json` 落盘，八项各有结论与依据，并保存与 `summary.json` 一致的 `canonicalPlan`。BLOCK 时在同一 MAIN 会话保留本轮实际作者模型并带上审查意见。只有 R1 引用类 BLOCK 且尚未专修时，下一轮只改引用；否则尚未重做时，重做一次。重做和引用专修各至多一次，连同首稿合计至多 `decisionMaxRounds` 轮（默认 3）；两种机会都用过或总轮次已满仍 BLOCK，进 `held`（`review_blocked`），不改判、不落地。每轮独审仍按实际作者选择异家族。审查输出两次都解析不出（`review_parse_error`）不计 BLOCK，也不计 PASS。

### 5. 落地

- **PASS（T/P 同）**：正本写进 `decision/<票号>` 分支（`.chanlun/definitions/`、`docs/adr/` 或 `CONTEXT.md`），票上发 resolution 评论，首行按分级写授权句（见「1. 分级」），并附草稿、审查路径；票上若还挂旧 `needs-approval`，发 resolution 前摘掉。决策文档按日批量开 PR；PR 合入后关票，map 的 Decisions so far 加一行，接新 blocking 边。
- **held（不放行）**：原因三种——`review_blocked`（轮次用尽仍 BLOCK）、`fact_gap`（考古事实缺口）、`invalid_landing`（获审草稿缺合法落点）。依次写入：票上发「**裁决暂停（未放行）**」评论（BLOCK 表或缺口清单）、摘旧 `needs-approval`、取消认领；每步落 `heldDone` 断点（`held_pending`），中断后只补没做完的步。held 不加任何标签，也不改判。
- **返修入口**：held 票在 `summary.json` 的评论水印之后出现编排者账号首行以「退回 / 驳回 / 打回 / 否决 / 不批准 / 不同意」开头的评论，下一轮作废旧 MAIN 会话并整链重跑（新分级、考古、起草、非作者独审）；没有这样的评论，claimer 不重跑、不调模型。

宿主确定性执行获审 `canonicalPlan`，不再调用 land 模型；triage、draft、review 的模型工具权限均只读。写入前核目标及父目录不是符号链接，当前目标与获审 `before` 逐字一致。main 无关文件前进可继续，目标漂移则保留 `reviewed_pass` 并要求重新起草、独审。提交前核工作树仅有目标改动，暂存与最终 commit 的目标 blob 均等于获审 `after`。

断点恢复使用 `commitBase` 与 `committedSha`：commit 已完成但回执中断时，核父提交、唯一改动文件、精确正文和提交消息后复用同一 SHA；每次实际 push 前重新 fetch main，核 main 上目标原件仍与获审 `before` 逐字一致，否则保留 `reviewed_pass` 并要求重审。push 回执中断时先查远端，远端已是同一 SHA 则只补记录，不重复推送，也不受之后 main 变化影响。`landedSha` 已存在则只补 resolution 评论。旧未推送 `reviewed_pass` 若缺编辑合同，明确要求迁移并重新独审，不拿旧 PASS 自动补写；旧已推送 `landedSha` 仍可只补评论。草稿、独审原件与断点合同不一致时拒绝续做。

这些检查不改变 PR 与 main 合入人闸。

### 6. 解锁

wayfinder_engine 下一轮重扫；FU blocker 全关的实施票被挂 `sandcastle`，进执行面。

放行判据（2026-10-20 图审计第 13–15 条修正）：扫描集是 open 的 `ready-for-agent` ∪ `wayfinder:impl`；实施票按 `[impl]` 标题或 `wayfinder:impl` 标签认（决策票除外）。批准态沿 sub_issues 子树递归：祖先链上有 SPEC 的票，链上每张 SPEC 都已批准才算批准；没有 SPEC 的 map 直挂票及其子票继承 map 直挂 SPEC 的批准态，且仍须带 `ready-for-agent` 才进放行队列。有任何 assignee、有 open blocker、带 `queue-policy.json` 的 hold 标签（`needs-approval`/`ready-for-human`/`needs-info`/`needs-triage`/`wontfix`）、或仍有 open 子票的汇总票，一律不挂 `sandcastle`；挂之前重读单票及其批准态依赖的全部 SPEC 正文、状态和评论，用 `isSpecApproved` 重验；嵌套 SPEC 的外层驳回不能被内层 PASS 覆盖。`needs-approval` 已无流程会添加，留在 hold 里只为不让存量票被批量放行：实施票上的旧标签由主控逐票核实依赖与 owner 后摘（#1909）。

SPEC「已批准」= 编排者账号的有效结构化 SPEC 裁定（见下节）；所有票都须绑定非作者独审 PASS，任何日期的自由文本「批准 / 通过」都不算。驳回词任何时候都撤销。

裁决面 decision-claimer 在 live 与 dry-run 下都跳过本地已落地（`done`/`land_t` 且无断点）的票、held 且水印后无退回评论的票，以及已关闭、已有 resolution、或编排者账号首行「用户 … 批复」评论的票（之后的「退回/驳回/重写」评论会重新打开）。带旧 `needs-approval` 的 grilling 票照常入选。要重跑已落地的票，先把 `/Volumes/AgentStorage/decisions/<票号>` 改名（先例 `1530.returned-20261020`）。

**旧影子模式恢复**：本地 `summary.json` 是 `shadow_p` 或断点 `shadow_p_pending` 的票，下次被选中时整目录改名为 `<票号>.shadow-p-retired-<k>` 留档，按新流程整链重跑。旧 P 稿把 C 类决定留给用户批复，不能当作代理裁定落地，旧 PASS 也不复用；票上已有「用户 … 批复」的照旧跳过。

## SPEC 裁定（#1909）

SPEC 起草后由非作者独立进程审查。独审者在原件 JSON 中记录所审票号、正文摘要、双方会话或进程身份与结论，并附 R1–R6 意见。身份使用具体会话或进程标识，同模型的两个进程可独立审查。

```json
{
  "specNumber": 1340,
  "verdict": "PASS",
  "author": "draft-session-id",
  "reviewer": "review-session-id",
  "digest": "<独审所读 GitHub body 的 specBodyDigest，64 位小写十六进制>",
  "findings": []
}
```

`verdict` 只能是 `PASS` 或 `BLOCK`。`specNumber` 是正整数；作者与独审是非空单行文本，去首尾空白、忽略大小写后必须不同。`digest` 在独审时从所读正文计算：统一 CRLF 为 LF，去末尾空白，取 UTF-8 sha256。`specBodyDigest` 导出同一算法。原件来源与身份真实性由调用者负责，发布工具检查字段形状、身份不同、票号和当前正文绑定。

发布时读取原件，不重新生成所审摘要。先生成成功，再发送评论，避免失败时提交空评论：

```bash
npx tsx scripts/wayfinder_engine.mts --spec-ruling 1340 --review-file /Volumes/AgentStorage/spec-1340/review.json \
  > /Volumes/AgentStorage/spec-1340/ruling.md && \
  gh issue comment 1340 --body-file /Volumes/AgentStorage/spec-1340/ruling.md
```

工具只读 GitHub 和原件并向 stdout 输出；`证据` 保留实际读取的原件路径。PASS 与 BLOCK 都须匹配当前正文；不匹配时无评论输出，重新独审当前版本。生成的评论格式为：

```text
**SPEC 独审裁定：PASS**
SPEC：1340
作者：draft-session-id
独审：review-session-id
正文 sha256：<独审原件中的 digest>
证据：/Volumes/AgentStorage/spec-1340/review.json
```

wayfinder_engine 只认编排者账号的结构化评论，消费时再次核对票号、非作者身份、证据位置和当前正文摘要。正文修改即使发生在生成评论后，也会使 PASS 失效。扫描结果只用于规划：拆票在解析当前正文时及每次创建前重验，正文偏离本轮计划即停止；挂队列在写标签前重验全部依赖 SPEC。已创建票的图内关联继续完成以保留可恢复身份，不因失效回滚或删除既有票；后续建票和 blocking 写入停止。GitHub 不提供跨评论读取与写操作的原子事务，这里缩短窗口，不宣称消除最后一次读之后的并发变化。按时间序最新一条结构化记录生效：BLOCK 或无效 PASS 都不放行，后续驳回评论撤销 PASS。自由文本批准没有日期兼容层，存量 SPEC 也须补齐真实独审绑定。

独审者只拿 SPEC 正文、被引用的决策票 resolution 与仓内源文件，看不到起草推理；按 R1–R6（引用、教义冲突、验收可执行、Destination、宽严两档与兜底、受影响票与边）逐条给结论，原件落 `/Volumes/AgentStorage/`。引擎起草的 SPEC 不再挂 `ready-for-human`。

## 授权记录

#1323 Notes 追加一条，comment 同步留痕：

> **N-1 技术类决策预授权**（用户 2026-10-18 原话：「其实很多技术的东西 完全不需要我来做 我完全信任你的判断」）：本图 T 类 grilling 票由代理起草、独立审查通过后直接裁定。依此判定的 resolution 首行注明「本条依 #1323 Notes N-1 预授权判定」。

**#1909 扩展**（用户直接授权本次流程改变）：P 类与 SPEC 同样经起草与非作者独审 PASS 由代理裁定并记账，删除普通用户审批门与 `/graph-approve`（真实资金与权限边界见 ADR 0041）；P 类 resolution 首行「本条依 #1909 代理独审裁定」。合入 main 仍是人闸。

`wayfinder-workflow.md` 的「不碰决策票（v1）」只约束 wayfinder_engine 本体，裁决面是独立组件；该文同步加一行指向本文。

## 调用通道

`spawn_agent` 是会话内工具，launchd 脚本调不到。裁决面与 wayfinder spec 起草保留 Claude CLI 控制通道；Sandcastle 实施/独审默认使用下述 Codex 执行通道：

```bash
ANTHROPIC_BASE_URL=http://127.0.0.1:8318 \
ANTHROPIC_AUTH_TOKEN=<运行时从 CLIProxyLocal config 读取，不落日志> \
claude -p --bare --model gpt-6.1-sol --output-format json --no-session-persistence
```

2026-10-25 用户裁定（#1617）：决策裁决面的分级、考古与首稿使用 `.sandcastle/main-model.mts` 的 `gpt-6.1-sol`；评审打回后由同模型带具体意见重做，当时保留轮次与转 P 规则；#1909 后轮次用尽转 held，不再转 P 待批。#1742 F4 将分级、考古、起草、重做与引用专修接入同一 MAIN 会话；重做与引用专修保留本轮回执记录的实际作者模型，独审取实际作者的另一家族。#1742 F2 将落地改为宿主执行获审精确编辑，不再使用模型。429 按退避重试；安全拦截的改派与异家族 FAIL-loud 规则保持不变。真实 `--bare` 冒烟见该票验收记录。

执行面 `main.mts` 与 `main-swarm.mts` 使用 Sandcastle `codex` provider，走同一模型配置。noSandbox 用 `127.0.0.1:8318`；Docker 保留隔离并用 `host.docker.internal:8318`，镜像安装 Codex CLI（Claude 仅供显式 legacy），Codex 客户端密钥只从进程/launchctl 的 `CLIPROXY_LOCAL_KEY` 经运行时 env 进入；main-swarm 在领票前做容器连通性硬检查，失败不换模型、不认领（#1617）。

## 宿主约束

Sandcastle 宿主 `NewChanlun-sandcastle-host` 开启 `SANDCASTLE_HOST_AUTO_UPDATE=1`：每次派工前 `reset --hard origin/main`，工作区脏则拒绝派工。因此执行面的任何改动都必须经 PR 合入 origin/main，宿主上不保留本地提交。

## 并行度

- 裁决面：同时处理至多 3 张互不阻塞的 grilling 票，claim 先行防重复。
- 执行面：当前单实例串行；接通后改为至多 3 个并发 main.mts，按票号加锁。
- 外盘不足时，长任务的 worktree 与日志可放 Windows 主机（Tailscale SSH）。

## 流程完成判据

1. 裁决面 dry-run：对一张 T 或 P 类票产出 `draft.md` + `review.json`，不写 GitHub。
2. 裁决面 live：一张 T 类票走完分级、起草、独审、分支落文档、PR。
3. 执行面：claim-loop 派出的实施工蜂实际产生输出（上下文非 0、有 commit 或明确 blocked 结论）。
4. health-monitor 发现停摆能变 _red_ 并自愈，进度页可看。
5. 以上改动经 PR 合入 main（人闸）。

### Codex 单票阶段交付（#1617，2026-10-25）

实施票每 run 仍是一票。作者在 workspace-write 下改源码并报告实际验证，受管宿主核完整文件集合并提交；独审读精确 commit，修正经同一宿主步骤提交。Rust 验证在封包后由宿主调用 nc-remote/CI；未验证不算验收完成。此职责分离已经用户明确批准，不扩大工蜂沙箱权限，main 合入仍是人闸。

旧段落中 Docker/swarm 的模型配置只说明路由，不代表执行可用：namespace 和 swarm 提交责任迁移未验收，默认 namespace 探针失败时入口在 planner/认领之前 fail-loud。主控不得将此候选的单票宿主冒烟计为 swarm 或全图验收。

Codex 的受管提交提示使用 codex-implement-prompt.md / codex-review-prompt.md；旧 implement-prompt.md / review-prompt.md 保持原调用合同。swarm namespace 恢复后仍须补完并验证其提交责任迁移，本票该项尚未闭合。
