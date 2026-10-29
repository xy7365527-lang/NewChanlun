# #1778 考古：domain-event 持久撤回子契约版本门控

- 票：https://github.com/xy7365527-lang/NewChanlun/issues/1778 （T 类，承接 #1768 / PR #1772）
- 代理：/root/draft1778_contract28f（claude-fable-5-1，medium），只读取证，未改码、未推送、未改 GitHub
- 基线：`origin/main` = `c0c5a369a49852e0fa5a724e57d9adbfab205975`；返修候选 `e1bd49a5b67aea729aa39ad983d93a58db16f72e`（`origin/codex/repair1772-28q`，是 PR #1772 head `89edf18f` 的快进后继，`git merge-base --is-ancestor` 通过）；PR #1772 head 仍为 `89edf18f0932d1ad43429f11f9fdca9628e19805`
- 本地 checkout 落后 origin/main（HEAD `54ce1be564`，无 ADR 0051 文件），所有正本与代码行号一律取 `git show origin/main:<path>`，e1bd49a 侧取 `git show e1bd49a…:<path>`
- 时间：2026-10-28T07:24+08:00（`date`）
- 返修轮 28H（/root/repair1778_contract28h）：按独审 `1778-INDEPENDENT-REVIEW-28G.md` F1–F7 修订 Q3 默认值、补 T15–T17 与 R21、订正行号；未改码、未推送、未改 GitHub 票

## 两路查询总表

### 第一路 tracker（含已关闭票）

| # | 命令 | 命中（节选） |
|---|---|---|
| T1 | `gh issue list --state all --search "withdraw"` | #1778、#1408、#1599(C)、#1340、#734(C)、#1560(C) |
| T2 | `… --search "撤回"` | #1768、#1380、#1651、#1534(C)、#1371 等 40 条；无「撤回 op 合同落点」裁定票 |
| T3 | `… --search "domain event"` / `"domain-event"` | #1768、#1534(C)、#1536、#1621、#1619(C)、#1323、#1488(C) |
| T4 | `… --search "schema revision"` | #1768、#1672、#1534(C)、#1537(C)、#1371(C)、#1372(C)、#1394(C)、#1612(C) |
| T5 | `… --search "子契约"` / `"子合同"` | #1768、#1657、#1716、#1715、#1717、#1718、#1527、#1406、#1620(C)、#1674(C) |
| T6 | `… --search "sr-expansion"` | #1717、#1340、#1599(C) |
| T7 | `… --search "tb03c-book"` | #1406、#1674(C)、#1620(C)、#1715、#1669(C)、#1671(C)、#1622(C) |
| T8 | `… --search "tb03b-finance"` | #1673、#1672、#1408、#1349(C)、#1406、#1394(C) |
| T9 | `… --search "economic-session/2"` | #1672、#1768、#1398、#1749、#1670、#1717、#1621、#1716、#1769、#1406 等 |
| T10 | `… --search "旧域不原地"` | 泛命中（搜索引擎分词），无专票 |
| T11 | `gh issue view 1534 --json body,comments` | 分级 T→两轮 BLOCK 转 P；用户 2026-10-27「批准」；正本 ADR 0051，PR #1766 squash `7b842b69a1`；下游 #1768 已开 |
| T12 | `gh issue view 1768 --json body,comments` | 票面 `:45`「持久表示若须新增 protocol/schema revision 或改变已签资源帽，停下交主控；不在本票内自定新修订」；`:19`「ADR 0051:374 九个 Python 读者逐个核兼容」；首条交付评论（2026-10-27T20:40:35Z）卡点 1「落盘成持久 op 需新增子契约修订——按票面禁令停下交主控」；异家族独审 BLOCK 三 HIGH |
| T13 | `gh api repos/…/issues/1768` 依赖 | blocked_by = #1534(closed)、**#1778(open)**；blocking = 3（#1361/#1363/#1399）。**#1778 → #1768 边已存在**，本考古不动边 |
| T14 | `gh issue view 1366` | TB-07 汇总票，#1768 为其子票；#1778 票面写 Part of #1323 |
| T15 | `gh issue view 1778 --json body,comments`；`gh issue view 1776` | **票面验收第 4 项（body `:25`）曾写「建立 #1776 blocks #1768」，是笔误，主控已于 2026-10-27T23:42:02Z 更正票面**：同票首行写「承接 TB-07-E #1768」，主控 2026-10-27T23:17:54Z 评论「主控已建立原生 blocking 边：#1778 blocks #1768」，T13 原生依赖只有 #1778。#1776 是无关票（OPEN「[task] #1742 Docker 方案 B 第 13 行受控验收专用票：工蜂写入探针文档」），不阻塞 #1768。实际依赖 = #1778 blocks #1768；**落地禁止新建 #1776→#1768 边** |
| T16 | `gh api repos/…/issues/1778/parent`；`gh api repos/…/issues/1323 -q .sub_issues_summary` | parent 返回 404：#1778 **不是** #1323 的原生 sub-issue；#1323 子票 total=100。N-1 适用范围只靠票面首行「Part of #1323」 |
| T17 | `gh issue view 1323 --json body` | map 索引节标题是 `## Decisions-so-far`（`:88`），同族索引行样式见 `:92`（#1534 一行）；body 70,804 字节，远低于 262,144 上限 |

### 第二路 仓内推论层 / 正本层 / 代码

| # | 命令 | 命中 |
|---|---|---|
| R1 | `rg -n "旧域不原地\|不原地升级\|不原地扩义\|不混写" analysis docs .chanlun rust/src` | `rust/src/economic_session/store.rs:18`、`:41`、`:946`；`finance.rs:637`；`.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md:115` |
| R2 | `rg -n "子合同\|子契约" docs/adr .chanlun/definitions CONTEXT.md` | **0 命中**：正本层没有「子合同」通则，机制只在代码与 SPEC 增补里 |
| R3 | `rg -n "子合同" rust/src` | `book.rs:7`、`store.rs:104/:797`、`finance.rs:18/:22/:38/:40/:81/:200/:711` |
| R4 | `rg -n "WithdrawDomainEvent\|DomainEventWithdrawn\|domain-event-withdraw" .`（main 工作树） | **0 命中**：该 op 与回执只存在于未合入的 e1bd49a |
| R5 | `rg -n "protocol_revision\|domain_revision" analysis docs .chanlun` | `issue1394-financial-owner-20260930/EVIDENCE.md:51`（显式 protocol_revision 分派，absent/null→/1，混搭拒绝）；`issue1406-owner-20261020/frontier-20261020.md:75/:90` |
| R6 | `sed -n 368,392p .chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md` | §5.8「旧 manifest、plan、grant、receipt、经济 root：不重写、不重铸」「旧库不原地改格式……本节不授权任何带未结责任会话的热迁移」「未知版本、混合域、旧 validator 收到新字段……均明确拒绝」 |
| R7 | `git show origin/main:rust/src/economic_session/store.rs` | `:108-125` `Config::load` 读同一份 manifest 文件、先判域版本与 `sr_expansion`；`:276-297` 只按 `chong` 参数区分 `B:<id>` 与 `E`，`manifest_hash: hash(&manifest)`（`:293`）对 E 与全部 B 相同——**E 与各 B 共用一份会话 manifest，没有「本域 manifest」**；`:15-20` SCHEMA_V1/V2 唯一登记点，`:17-18`「旧域不原地升级、不混写，未知值当场拒绝（§5.8）」；`:26-27` REVISION_FIELD=protocol_revision；`:64-66` domain_revision；`:207-236` 封闭已知 op 集合，`:232-233` sr_expansion 只在子合同选中时算已知；`:801-806` init 写子合同 meta：`book_revision`（`:802-803`）**不按域区分**，`sr_expansion`（`:805-806`）经 `bound_revision` 只 E；`:998-1013` audit 核 meta 与 config 一致，否则 IdentityConflict「不得在 recover 暗迁移」（书账 `:1003` 全域、SR `:1013` 只 E）；`:1099-1150` 审计按 history 末项 kind 分派（ResolveEvidence / sr_expansion::OP / finance / BindVoice），其余落 `:1132` else 分支核 `recorded` 末项（`:1139` verify_record；E `:1141` recorded_seq/commit_ns，B `:1147` b_observation.commit_ns，均报 StorageUnavailable） |
| R8 | `git show origin/main:rust/src/economic_session/sr_expansion.rs` | `:9-15`「新语义是与旧合同并列的子合同 sr-expansion/1：manifest 顶层 sr_expansion 显式选择，E 在 init 绑定、audit 核对，旧域不原地扩义。它不占财务版本号，声明在 economic-session/1 与 /2 上」；`:25-27` OP/QUERY/RECEIPT；`:33-35` CONTRACT_FIELD/CONTRACT_REVISION/META_KEY；`:281-296` registry 读 manifest 顶层字段、核 schema_revision；`:329-336` validate_config / bound_revision（只 E） |
| R9 | `git show origin/main:rust/src/economic_session/finance.rs` | `:17-25` tb03b-finance/1../4 四个版本常量，`:18`「`/1` 的子合同也不得靠改名接管新域」，`:20-22` **`tb03b-finance/3` = `economic-session/1` 域上显式选择的窄测试臂**（`:55` 登记；判例：子合同可声明在 `/1` 上），`:23-25` `/4` = `/2` 域上显式选择 + ActivateCapPolicy，「`/1`、`/2` 不接受激活」；`:67-85` contract() 由 `finance.schema_revision` 选，与域版本混写报 IdentityConflict |
| R10 | `git show origin/main:rust/src/economic_session/book.rs` + C8 报告 | `book.rs:7-13` tb03c-book/1、/2（/3）由 manifest 顶层 `book_revision` 冻结；`.chanlun/review-results/issue1406-owner-20261020/C8-REPORT.md:1-13`「`/1` 与缺省域的分派、文案和书账形状不变」「`/1` 与缺省域夹带 funding 时返回 SchemaUnsupported」 |
| R11 | `git log origin/main -S'"BindVoice"' -- store.rs` 等 | BindVoice 进封闭集合 = `c0ed0497a1` 2026-09-14（#1374）；ChongAdvance/ImportResponsibility/PrepareDecision = `4a635b7a40` 2026-09-14（#1394 wip）；**§5.8 版本登记代码 = `8f42dece50` 2026-09-30**；之后的 ActivateCapPolicy `3b1a1841cd`（#1716，finance/4）、sr-expansion `499841b592`（#1717）、book/3 `0fe8e2f3c9`（#1715）、book/2 `92dc58eafc`（#1674）全部走子合同 |
| R12 | `git show e1bd49a…:rust/src/economic_session/store.rs` | `:230` `super::domain_event::WITHDRAW_OP` 无条件进封闭集合（与 `:233-234` sr_expansion 的条件式形成对照）；`:1100-1114` 审计重放新增撤回分支；`:1228-1251` 回执审计；`:1369-1371` 回执主键；`:1426-1440` 消息绑定。四处均无版本或子合同门控 |
| R13 | `git show e1bd49a…:rust/src/economic_session/service.rs` | `:1462` `WITHDRAW_OP => withdraw(...)`，E 与全部 B 都分派，无域/版本门 |
| R14 | `git show e1bd49a…:rust/src/economic_session/domain_event.rs` | `:16` CONTRACT=domain-event/1；`:18-24` contract_record 三个可调字段默认值；`:460` withdrawn() 外壳投影（复用 ADR 0051 既有外壳）；`:510-511` WITHDRAW_OP/WITHDRAW_RECEIPT；`:531-534` 回执键 `domain-event-withdrawn:<hash(target)>`；`:605-650` withdrawal_image（写路径与审计共用，#804）；`:652-700` withdraw（纪元→审计→previous_message→回执幂等→append→回执→bind_message） |
| R15 | `git show origin/main:docs/adr/0051-…md` | `:7-8` 状态与批准；`:236`「contract 固定写 domain-event/1；改形状即新版本」；`:247` transition 含 withdrawn；`:273` 撤回 = 追加一条 kind: withdrawn 域事件，payload 带三字段；`:290-300` 三个可调字段；`:312` withdrawn_generation 属 AsKnown 事实；`:353-357` 要开的 impl 票及测试名（含 `domain_event_withdrawn_is_appended_and_old_rows_unchanged`）；`:370`「本 ADR 零代码改动」；`:374` 九个 Python 读者；`:394` 批准范围 = 外壳合同 + 13 默认值。**全文无 wire op、回执 kind、socket 接口定义**（grep `WithdrawDomainEvent\|wire\|回执` 仅命中 `:374` 读者列表语境） |
| R16 | `git show origin/main:CONTEXT.md` | `:457-469` 域事件 / 写者纪元 / 来源纪元 词条（ADR 0051 落地） |
| R17 | `git show origin/main:docs/adr/0041-…md` | `:9` 适用范围只管缠论自由度；`:13` 判别句；`:17-23` 落法四条，`:23`「改签即新版本」 |
| R18 | `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1772-CONTRACT-ROUTING-28S.md` | 主控派的只读调查：结论「改变了已批准合同、现有扩展点不覆盖」，推荐照 sr-expansion/1 另立子合同；行号与本考古 R7/R8/R12/R13 逐条复核一致 |
| R19 | `…/1772-OWNER-REPAIR-28Q.md:13-14`、`:27`、`:29` | 返修者自述「未新增表、未改既有字段名、未改 schema revision；新增的是一个 op 名与一种回执 kind」，并把「是否接受不升版本只增 op」列为需主控裁定 |
| R21 | `git show origin/main:rust/src/economic_session/store.rs \| sed -n '108,125p;969,1015p;226,236p'` | main 对 manifest 顶层键与 meta 键**不做 exact 校验**（`:108-125` 只调各子合同 `validate_config`；`:969-1015` 只核登记过的 meta 键）⟹ 旧二进制**忽略** `domain_event_withdrawal` 顶层键与新 meta 行，不能识别它；但来源 `operations` 含 `WithdrawDomainEvent` 时在 `:235` 报 `InvalidDomain：操作权限未知或重复`，加载即拒，不能执行新 op。与 sr_expansion 判例同式 |
| R20 | `grep -n "history\|kind\|receipt" s_session/economic_query.py` | `:282` 只筛 `ResolveEvidence` 的 history 项（对未知 kind 容忍）；`:772` EconomicView 顶层字段集合不变；`:860-889` original/receipt 只认 decision/grant。读者兼容仍须按 #1768 票面逐个核，本考古不替代 |

## 待决点归类

| 点 | 问题 | 归类 | 出处（可回查） | 默认值 / 去向 |
|---|---|---|---|---|
| Q1 | 持久撤回 op 落在哪一层：旧域 `/1` `/2` 原地扩义、并列子合同、还是新域版本 `/3` | **B 有先例未正式裁** | §5.8（R6，已批准 SPEC 增补）；store.rs:17-18（R7）；sr_expansion.rs:9-15（R8）；finance.rs:18/:23-25（R9）；C8-REPORT:1-13（R10）；R11 时间线：§5.8 落码（2026-09-30）之后四次新增持久 op/语义全部走子合同，原地加进封闭集合的 BindVoice 等都在之前 | 默认：并列子合同，manifest 显式选择、init 绑定、audit 核对；`/1` `/2` 已知 op 集合不变 |
| Q2 | 子合同选择子与绑定的形状 | **B** | sr_expansion.rs:33-35/:281-296/:329-336；store.rs:801-806/:998-1013；book.rs:7-13；finance.rs:67-85 | 默认：manifest 顶层 `domain_event_withdrawal = {schema_revision: "domain-event-withdrawal/1"}`；init 写 meta `domain_event_withdrawal_revision`；audit 核 meta 与 config 一致否则 IdentityConflict；未选择域中 `WithdrawDomainEvent` 不进已知集合（manifest 校验即拒），不分派 |
| Q3 | 选择粒度：仅 E（如 sr-expansion）、全会话、还是逐域 | **B** | E 与全部 B 共用一份会话 manifest（R7 `store.rs:108-125`、`:276-297`，同一 `manifest_hash`）；全会话判例 `book_revision`（`book.rs:7-11` manifest 顶层，init `store.rs:802-803`、audit `:1003` 不按域区分）；ADR 0051:273 撤回裁定覆盖 B/E；#1768 票面范围 B/E；e1bd49a service.rs:1462 对 E 与 B 分派；反例 sr_expansion `bound_revision` 只 E（`sr_expansion.rs:334-336`，该合同头注 `:9-15` 自定只 E 绑定） | 默认：**同一会话 manifest 的选择作用于其所含全部 E 与 Bᵢ 域**；E 与每个 Bᵢ 在各自 init 写 meta，audit/recover 各自核对同一 manifest 选择；不设逐域列表；S 不在范围内 |
| Q4 | `domain-event/1` 外壳要不要升版 | **A 已裁** | ADR 0051:236「改形状即新版本」；e1bd49a domain_event.rs:460 withdrawn() 复用既有外壳投影，17 字段不变 | 沿用：外壳不升版；本票只裁持久 op 的合同落点 |
| Q5 | 审计四处（history 重放、回执、回执键、消息绑定）与 history 条目 kind / 回执 kind 的归属 | **B** | store.rs:1099-1141 按 kind 分派的既有形状；sr_expansion 在 store.rs:1102-1106/:1206-1207/:1321-1322/:1385-1392 四处的接法 | 默认：四处分支都以「本会话已选择 domain-event-withdrawal/1，且本域 meta 已在 init 绑定」为前置；未绑定域里出现撤回条目或回执 = 域损坏 StorageUnavailable |
| Q6 | 旧库迁移与回放边界：已存在的 `/1` `/2` 库能否事后选上本子合同 | **A 已裁** | §5.8「旧库不原地改格式……不授权任何带未结责任会话的热迁移」；store.rs:1008-1013「不得在 recover 暗迁移」；R4：e1bd49a 未合入 main，不存在带撤回条目的生产库 | 沿用：只能在 init 时绑定；recover 不得补绑；旧库不迁移 |
| Q7 | 该机制是否属 ADR 0041 自由度 | **A 已裁** | ADR 0041:9 只管缠论自由度 | 不是：这是工程版本门，不落可调字段；但按 ADR 0041:23 同一精神「改签即新版本」 |
| Q9 | #1778 票面第 4 项的 `#1776 blocks #1768` | **A 已裁** | T13 原生依赖、T15 主控评论 2026-10-27T23:17:54Z | 笔误（票面已于 2026-10-27T23:42:02Z 更正）；实际依赖 #1778 blocks #1768（已存在）；禁止新建 #1776→#1768 |
| Q10 | 正本落点：能否改 ADR 0051 | **A 已裁** | ADR 0051:7-8「用户已批准……不以 #1323 Notes N-1 代替」、`:394` 批准范围；graph-engineering.md:77 T 类 PASS 把正本写进 `decision/<票号>` 分支 | 不改 ADR 0051；另立新 ADR，CONTEXT.md 加一句指向它；N-1 只授权新决策，不回写用户批准的旧正本 |
| Q11 | 旧二进制遇已选择子合同的域 | **A 已裁（代码事实）** | R21 | 忽略新顶层键与 meta 行，不识别；来源授予 `WithdrawDomainEvent` 时加载即拒（`:235`），不能执行新 op；撤回条目只能出现在授予该 op 的会话里，故旧二进制碰不到撤回条目 |
| Q12 | map 挂接 | **A 已裁** | graph-engineering.md:77/:90；wayfinder-workflow.md:36/:159；T16/T17 | resolution 首行 N-1；#1323 `## Decisions-so-far` 补一行索引；不伪造原生 sub-issue |
| Q8 | e1bd49a 的 HIGH-1（delta 事件集合）与 HIGH-3（snapshot/delta 读前审计）是否受本裁定影响 | **B** | 1772-CONTRACT-ROUTING-28S.md；R14 这两项不读 manifest 子合同字段 | 默认：不受影响，原 owner 实施门控时保留 |

C 类（research / probe）：**0**。所有待决点都有仓内判例或正本可援引。

## 机械校验自检

- 两路各 ≥1 条：每点 tracker 与仓内均有（Q1 T5/T12 + R6-R11；Q2 T5/T7 + R8-R10；Q3 T12 + R13/R14；Q4 T11 + R15；Q5 T5 + R7/R12；Q6 T4 + R6/R7；Q7 R17；Q8 T12 + R14/R18；Q9 T13/T15 + wayfinder-workflow.md:147「blocking 边用 tracker 原生边」；Q10 T11 + R15 `:7-8`；Q11 T5/T12 + R21；Q12 T16/T17 + graph-engineering.md:77）。
- tracker 覆盖已关闭票：T1–T10 均 `--state all`。
- A 类七点均有出处（ADR 0051:236、§5.8 + store.rs:1003/:1013、ADR 0041:9、T13/T15、ADR 0051:7-8、R21、graph-engineering.md:77）；B 类五点均有默认值；无 C 类。
- 28H 换说法复搜：tracker「并列子合同」「withdraw op」只命中 #1778 与子合同实施票；仓内 `bound_revision` 只命中 sr_expansion 判例——Q3 新默认值以 `book_revision` 全会话判例为出处，不再无源。
