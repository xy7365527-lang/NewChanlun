# #1778 裁定草稿：B/E 持久撤回 op 的合同落点（domain-event-withdrawal/1 并列子合同）

- 票：https://github.com/xy7365527-lang/NewChanlun/issues/1778 ｜ 分级：T（graph-engineering.md:23「接口与 seam、状态机与事件 schema、恢复语义」）
- 起草：/root/draft1778_contract28f（claude-fable-5-1，medium），只读取证；考古见同目录 `archaeology.md` / `archaeology.json`
- 基线：`origin/main` `c0c5a369`；返修候选 `e1bd49a5`（PR #1772 head `89edf18f` 的快进后继，未推 PR 分支、未合 main）
- 本草稿不发 resolution、不改依赖、不改码；待异家族独审（Opus 5.5）R1–R8 后按 N-1 落地
- 返修轮 28H：按 `1778-INDEPENDENT-REVIEW-28G.md` F1–F7 修订（选择粒度改为全会话、#1776 笔误、验收第 1 项二值化、正本改落新 ADR、旧二进制行为、行号与判例、map 挂接）；修复对照见 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1778-CONTRACT-REPAIR-28H.md`
- 返修轮 28J：按 `1778-INDEPENDENT-REVIEW-28I.md`（PASS）三项 LOW 修订措辞与验收细节，语义不变；见 `1778-LOW-FIX-28J.md`

## 一、问题

#1768（TB-07-E）在 PR #1772 的异家族独审被三项 HIGH 阻塞后，返修 `e1bd49a` 为 HIGH-2 新增了持久写口 `WithdrawDomainEvent` 与永久回执 `DomainEventWithdrawn`。它把这个 op 无条件写进 `economic-session/1`、`/2` 的封闭已知 op 集合（e1bd49a `store.rs:230`），对 E 与全部 B 分派（`service.rs:1462`），并在审计的 history 重放、回执、回执主键、消息绑定四处加了无版本门控的分支（`store.rs:1100-1114`、`:1228-1251`、`:1369-1371`、`:1426-1440`）。

后果已核实：一个 `/1` 或 `/2` 域只要被新二进制写过一次撤回，main 二进制审计该提交时会落入 `store.rs:1132` 的 else 分支，按 `recorded` 末项核对（`:1139` `verify_record`；E 在 `:1141` 核 `recorded_seq`/`commit_ns`，B 在 `:1147` 核 `b_observation.commit_ns`），E 与 B 都报 `StorageUnavailable`。这改变了已登记修订的持久表示。

票面 #1768:45 写明「持久表示若须新增 protocol/schema revision……停下交主控；不在本票内自定新修订」；原实现者首条交付评论卡点 1 与返修报告 `1772-OWNER-REPAIR-28Q.md:27` 都把这件事交给主控裁定。本票裁的是：**持久撤回 op 落在哪一层合同，以及选择、绑定、审计、拒绝与迁移边界。**

不裁的：`domain-event/1` 外壳本身（ADR 0051 已裁）、撤回语义（ADR 0051 裁定三已裁）、HIGH-1/HIGH-3 的安全修复（与合同落点无关）、任何资金、场所、权限或真实启用。

## 二、权威依据

**已批准 SPEC 增补 #1394 §5.8**（`.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md:368-392`）
- 「旧 manifest、plan、grant、receipt、经济 root：**不重写、不重铸。** 旧 reader 和原 native Hash 继续解释旧历史」
- 「旧库不原地改格式，旧金融 manifest 不自动换 binary；本节不授权任何带未结责任会话的热迁移」
- 「未知版本、混合域、旧 validator 收到新字段……均明确拒绝」
- 同文 `:115`：「economic-session/2；只用于全新测试会话，旧域不混写」

**正本层**
- ADR 0051 `:236`「`contract` 固定写 `domain-event/1`；改形状即新版本」；`:273` 撤回 = 追加一条 `kind: withdrawn` 域事件；`:370`「本 ADR 零代码改动」；`:394` 批准范围 = 外壳合同与 13 项默认值。全文没有定义 wire op、回执 kind 或 socket 接口。
- ADR 0041 `:9` 本通则只管缠论自由度；`:23`「改签即新版本——改值＝新版本……不在旧版本内热改」。
- CONTEXT.md `:460` 域事件词条：「只追加，撤回也是一条新的域事件」。

**代码判例**（行号固定到 `origin/main` `c0c5a369`）
- `store.rs:108-125`、`:276-297`：`Config::load` 对 E 与全部 Bᵢ 读同一份会话 manifest，只按 `chong` 参数区分域，`manifest_hash`（`:293`）相同——**不存在「本域 manifest」**。
- `store.rs:15-20`：E/B 域版本唯一登记点，`:17-18`「旧域不原地升级、不混写，未知值当场拒绝（§5.8）」；`:26-27` 选择子 `protocol_revision`；`:207-236` 封闭 op 集合，`:232-233` `sr_expansion` 的 op 只在子合同选中时算已知；`:801-806` init 把子合同版本写 meta（`book_revision` `:802-803` 不按域区分；`sr_expansion` `:805-806` 只 E）；`:998-1013` audit 核 meta 与 config 一致，否则「IdentityConflict：……不得在 recover 暗迁移」（书账 `:1003` 全域、SR `:1013` 只 E）。
- `sr_expansion.rs:9-15`：「新语义是与旧合同并列的子合同 `sr-expansion/1`：manifest 顶层 `sr_expansion` 显式选择，E 在 init 绑定、audit 核对，旧域不原地扩义。它不占财务版本号，声明在 `economic-session/1` 与内容域 `economic-session/2` 上」；`:33-35` 字段/版本/meta 键；`:281-296` registry 核 `schema_revision`；`:329-336` `bound_revision`——只认 E（`:334-336`），合同头注 `:9-15` 本就写「E 在 init 绑定」。
- `finance.rs:17-25`：`tb03b-finance/1..4`，`:18`「`/1` 的子合同也不得靠改名接管新域」，`:20-22` **`tb03b-finance/3` 是 `economic-session/1` 域上显式选择的窄测试臂**（`:55` 登记）——子合同可以声明在 `/1` 上，`/1` 已知语义不变，`:23-25` `/4` 在 `/2` 域上显式选择并加 `ActivateCapPolicy`，「`/1`、`/2` 不接受激活」；`:67-85` 合同由 `finance.schema_revision` 选，与域版本混写报 IdentityConflict。
- `book.rs:7-13`：`tb03c-book/1`、`/2`（`/3`）由 manifest 顶层 `book_revision` 冻结，作用于该会话全部 E/B 域——**全会话选择的判例**；C8 报告 `.chanlun/review-results/issue1406-owner-20261020/C8-REPORT.md:1-13`「`/1` 与缺省域的分派、文案和书账形状不变」。
- 时间线（`git log -S`）：§5.8 版本登记落码 `8f42dece50`（2026-09-30）之后，四次新增持久 op/语义（#1674 book/2、#1715 book/3、#1716 finance/4、#1717 sr-expansion/1）全部走子合同；直接加进封闭集合的 `BindVoice`（`c0ed0497a1`，2026-09-14）等都在 §5.8 之前，且那时集合本身就是 `/1` 的定义。

**票面与评论**
- #1768 body `:45` 停止条款；`:19` ADR 0051:374 九个 Python 读者逐个核兼容。
- #1768 首条交付评论（xy7365527-lang，2026-10-27T20:40:35Z）卡点 1：「落盘成持久 op 需新增子契约修订——按票面禁令停下交主控」。
- PR #1772 独审 BLOCK（`1772-INDEPENDENT-REVIEW-28P.md`）：HIGH-2 要求 withdrawn 持久化。
- 主控派的只读调查 `1772-CONTRACT-ROUTING-28S.md`：结论与本草稿一致；其引用行号已逐条复核。
- tracker：`#1778 blocks #1768` 已存在（API `blocked_by` = #1534(closed)、#1778(open)；主控 2026-10-27T23:17:54Z 评论「主控已建立原生 blocking 边：#1778 blocks #1768」），本草稿不增删。#1778 票面验收第 4 项曾写成「建立 #1776 blocks #1768」，是笔误，主控已于 2026-10-27T23:42:02Z 更正，见第六节。

## 三、候选

**C1（推荐）并列子合同 `domain-event-withdrawal/1`**，照 `sr-expansion/1` 判例。
- manifest 顶层字段 `domain_event_withdrawal = {"schema_revision": "domain-event-withdrawal/1"}`，缺省/`null` = 未选择。
- 选择作用于该会话 manifest 所含全部 E 与 Bᵢ 域（同 `book_revision`）。选择后：`WithdrawDomainEvent` 进已知 op 集合（与 `store.rs:232-233` 同式），`service.rs` 只对「本会话已选择且本域 meta 已在 init 绑定」的域分派；E 与每个 Bᵢ 各自 init 写 meta `domain_event_withdrawal_revision`；audit 核 meta 与 config 一致，否则 IdentityConflict「不得在 recover 暗迁移」。
- 未选择域：manifest 校验即拒（`InvalidDomain：操作权限未知`），不分派；若其持久图像里出现撤回条目或 `DomainEventWithdrawn` 回执，审计判域损坏 `StorageUnavailable`。
- `/1`、`/2` 的已知集合、表集合、`recorded` 判据、回执族全部逐字不变；main 二进制继续能审计任何未选择本子合同的域。
- 旧二进制：忽略新顶层键，靠来源 `operations` 里的未知 op 在加载时拒绝（裁定 10）。
- 代价：返修 head 必改 `store.rs:230`、`service.rs:1462` 与审计四处的门控，CI 与独审须在新 head 重做；manifest 多一个顶层字段，测试夹具要加。

**C2 新域版本 `economic-session/3`**（合法替代，不推荐）
- 把撤回 op 作为 `/3` 域定义的一部分。符合 §5.8「新试验域产生自身版本的新身份」。
- 代价：`finance.rs` 的 `Contract.domain` 与 `CONTRACTS` 表、`book`、`sr_expansion`（`:13` 声明只在 `/1` `/2` 上）、`TABLES`/`CONTENT_SQL` 判据、Python 读者的 `protocol_revision` 分派都要为 `/3` 重登记；为一个追加式 op 开整个域版本与 §5.8「只开隔离的新测试会话」的分量不相称。不写成「被否」，留作将来撤回语义与其他新语义一起成版时的路径。

**C3 原地扩义 `/1` `/2`（即 e1bd49a 现状）**（排除，真错）
- 正面违反 §5.8「不重写、不重铸」「旧 validator 收到新字段……明确拒绝」与 `store.rs:17-18`「旧域不原地升级、不混写」；与 §5.8 之后四次判例相反。
- 实测后果：main 审计撤回过的域报 `StorageUnavailable`（`store.rs:1132-1150`，E `:1141`、B `:1147`），等于新二进制单方面废除旧二进制对已登记修订的解释权。
- 它不是 ADR 0041 意义上的「另一种合法取值」，是对已批准迁移拒绝边界的违反，照 ADR 0041 裁定四排除。

## 四、裁定

本条依 #1323 Notes N-1 预授权判定（独审 PASS 后生效）。

1. **落点**：B/E 持久撤回写口 `WithdrawDomainEvent` 与回执 `DomainEventWithdrawn` 落为与 `economic-session/1`、`/2` 并列的子合同 **`domain-event-withdrawal/1`**，不占域版本号，不改 `domain-event/1` 外壳。`/1`、`/2` 的已知 op 集合、表集合、`recorded` 判据与回执族逐字不变。
2. **选择**：manifest 顶层 `domain_event_withdrawal` 对象，exact 字段 `{"schema_revision"}`，值只认 `"domain-event-withdrawal/1"`；缺省或 `null` 为未选择；其他值或形状 `SchemaUnsupported`。
3. **选择范围（全会话）**：E 与全部 Bᵢ 读同一份会话 manifest（`store.rs:108-125`、`:276-297`），裁定 2 的选择子作用于该 manifest 所含的 E 与每个 Bᵢ，不设逐域列表字段。判例是 `book_revision`（全会话）；`sr_expansion::bound_revision` 只认 E 是该合同头注（`sr_expansion.rs:9-15`）自定的范围，本合同覆盖 B/E（ADR 0051:273、#1768 范围），所以 `bound_revision` 不按 `config.domain` 过滤。S 不在本合同内（S 的撤回仍是 `s_structure_session` 的 `withdrawn_generation` 字段，ADR 0051 裁定三）。
4. **绑定**：选择后 E 与每个 Bᵢ 在各自 init 时在本域 `meta` 写 `domain_event_withdrawal_revision = "domain-event-withdrawal/1"`；各域 audit 与 recover 核该 meta 与同一 manifest 的选择（`bound_revision`）相等，不等报 `IdentityConflict：撤回子合同版本未由 init 显式绑定；不得在 recover 暗迁移`。未在 init 绑定的域此后不得补绑；旧库不迁移（§5.8）。
5. **已知与分派**：`WithdrawDomainEvent` 只在本会话已选择、且本域 meta 已在 init 绑定时进已知 op 集合（与 `sr_expansion` 同式），manifest `operations` 仍是来源权限闸，须显式授予；`service.rs` 只对满足该条件的域分派，否则 `InvalidDomain`。
6. **审计**：history 重放、回执审计、回执主键、消息绑定四处的撤回分支都以「本会话已选择，且本域 meta 已在 init 绑定」为前置；不满足该条件的域的持久图像里出现 `kind = WithdrawDomainEvent` 的 history 条目或 `DomainEventWithdrawn` 回执，审计报 `StorageUnavailable：域损坏`。写路径与审计共用 `withdrawal_image` 判据保持（#804）。
7. **语义不变**：撤回条目与回执的字段、去重键（被撤回事件身份）、同内容 no-op / 异内容 `IdentityConflict`、撤回不可再撤回、`superseded_by` 须指向已存在事件，均沿 ADR 0051 裁定三与 e1bd49a 现实现，本票不改。
8. **与 HIGH-1 / HIGH-3 的关系**：delta 事件集合完整校验与 snapshot/delta 读前全域审计不依赖本子合同，原 owner 返修时原样保留。
9. **不属自由度**：本机制是工程版本门，不是 ADR 0041 的缠论自由度，不落可调字段；版本串改值即新子合同版本。
10. **旧二进制**：main 不对 manifest 顶层键与 meta 键做 exact 校验（`store.rs:108-125`、`:969-1015`），所以旧二进制**忽略** `domain_event_withdrawal` 顶层键与新 meta 行，不识别本子合同。来源 `operations` 授予 `WithdrawDomainEvent` 时，旧二进制在加载时于 `store.rs:235` 报 `InvalidDomain：操作权限未知或重复`，整个会话起不来，**不能识别或执行新 op**。撤回条目只能写进授予了该 op 的会话（授权集合随 `manifest_hash` 冻结），所以旧二进制碰不到撤回条目。§5.8「旧 validator 收到新字段……明确拒绝」在这里由未知 op 授权间接实现，与 `sr_expansion` 判例同式；已选择但未授予该 op 的会话不含撤回条目，旧二进制按原合同可审计。

## 五、验收清单（判据二值）

1. `cargo test --features s_session domain_event_withdrawal_op_unknown_without_selection` 通过：同一份 manifest 中某来源 `operations` 授予 `WithdrawDomainEvent`，去掉顶层 `domain_event_withdrawal` 时，对 E 与每个 Bᵢ 调 `Config::load` 都返回以 `InvalidDomain：操作权限未知或重复` 开头的错误；加上选择子后都返回 Ok。另 `cat rust/src/economic_session/*.rs | grep -c '"domain-event-withdrawal/1"'` 输出 ≥ 1（先合并再计数，单一数值）。
2. `cargo test --features s_session --lib domain_event` 通过，且新增以下测试名（`cargo test --features s_session <名>` 通过）：
   - `domain_event_withdrawal_selection_binds_e_and_every_b`（选择后 E 与每个 Bᵢ init 后本域 meta 都有 `domain_event_withdrawal_revision = domain-event-withdrawal/1`；未选择时所有域都无此行）
   - `domain_event_withdrawal_recover_without_init_binding_is_identity_conflict`（E 与一个 Bᵢ 各测：手工写入 meta 或删 meta 后 recover 报 `IdentityConflict`）
   - `domain_event_withdrawal_entry_in_unbound_domain_is_storage_unavailable`（未绑定域图像注入撤回条目/回执后 audit 报 `StorageUnavailable`）
   - `domain_event_withdrawal_v1_v2_known_ops_and_tables_unchanged`（未选择时 `/1` 恰好 10 表、`/2` 表集合与已知 op 集合逐字等于 main 基线）
3. 既有 `domain_event_withdrawn_is_appended_and_old_rows_unchanged`、`be_snapshot_delta_resume_equals_same_cut_snapshot_fieldwise_and_gap_on_lag`、`recovery_replay_root_equals_original_commit_root_or_storage_unavailable` 在选择本子合同的 manifest 上通过。
4. `cargo test --features s_session --lib economic_session` 0 failed（nc-remote 或 CI，不在 macOS 编译）。
5. #1768 票面 `:19` 九个 Python 读者逐个核兼容的报告落 `/Volumes/AgentStorage/issue1768-*/`；以下两条命令退出码均为 0：`python -m pytest -q s_session/tests/test_*.py s_session/tests/issue1527_recovery/test_python_r4.py`（导入上述读者模块的既有测试集）与 CI 同口径的 `pytest -m "not slow" -q`（`tests/`）。
6. 新 head 快进推至 `sandcastle/issue-1768`（不 force），PR #1772 全部必要 CI 成功，再做异家族独审。
7. 正本：在 `decision/1778` 分支新建 ADR `docs/adr/<NNNN>-domain-event-withdrawal-subcontract.md`（`<NNNN>` 取落地时 main 与在飞 `decision/*` 分支都未占用的下一个号），带三行头（正本／最后裁定 #1778／受影响代码清单），`grep -c 'domain-event-withdrawal/1' docs/adr/<NNNN>-*.md` ≥ 1；`CONTEXT.md` 「域事件」词条后新增一句「B/E 的持久撤回写口由子合同 domain-event-withdrawal/1 按会话 manifest 显式选择，见 ADR <NNNN>」，不改原句（`git diff --numstat origin/main -- CONTEXT.md` 的删除列为 0）；`git diff --quiet origin/main -- docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md` 退出码 0（ADR 0051 零改动）。
8. #1778 票上 resolution 评论首行「本条依 #1323 Notes N-1 预授权判定」，附本草稿与 `review.json` 路径；PR 合入后关票，#1323 `## Decisions-so-far` 有含 `#1778` 的一行（挂接方案见第八节）。

## 六、受影响票与 blocking 边

- #1768（OPEN，TB-07-E）：直接消费本裁定；原 owner 按第四节改门控并补 Python 读者验收。`#1778 blocks #1768` **已存在**，不重复接。
- **#1778 票面验收第 4 项曾写的「#1776 blocks #1768」是笔误，主控已于 2026-10-27T23:42:02Z 更正票面**：同票首行「承接 TB-07-E #1768」、主控 2026-10-27T23:17:54Z 评论与原生依赖实测都是 #1778 blocks #1768。#1776 是无关票（OPEN「[task] #1742 Docker 方案 B 第 13 行受控验收专用票：工蜂写入探针文档」）。**落地禁止新建 #1776→#1768 边**。
- PR #1772：head 保持 `89edf18f` 直到新 head 产生；不推 e1bd49a 原样。
- #1366（TB-07 汇总）：#1768 为其子票，不新增边。
- #1361、#1363、#1399：被 #1768 阻塞的三票不变。
- #1717 / `sr-expansion/1`、#1716 / `tb03b-finance/4`、#1406 / `tb03c-book/2`：判例来源，不改、不接边。
- #1534（CLOSED，ADR 0051）：外壳不升版；ADR 0051 一字不改，子合同落新 ADR。
- #1330、#1539（X 侧）：X 若需持久撤回命令原件，另按本子合同同式选择；本票不接边。

**边增删汇总**：增 0、删 0；禁增 #1776→#1768。

## 七、授权引用

- #1323 Notes N-1 技术类决策预授权（用户 2026-10-18 原话，`.sandcastle/docs/graph-engineering.md:90`）。N-1 只授权本票这条新决策及其新 ADR，**不授权回写用户批准的 ADR 0051**（ADR 0051:8「本条依用户批复落地，不以 #1323 Notes N-1 代替」）。N-1 的适用依据是 #1778 票面首行「Part of #1323」；#1778 不是 #1323 的原生 sub-issue（见第八节）。
- 用户 2026-10-27 对 #1534 的「批准」（ADR 0051:8）：撤回语义已裁，本票只裁持久 op 的合同落点。
- #1768 body `:45` 停止条款与首条交付评论卡点 1：把本问题交主控裁定。
- 主控 2026-10-28 开票 #1778 并指定推荐方案为并列子合同。
- 不含：main 合入、真实资金、场所、权限、隔离策略，以及 #1768 整票或 #1366 的关闭。

## 八、map 挂接

#1778 不是 #1323 的原生 sub-issue（`gh api …/issues/1778/parent` 返回 404；#1323 子票 `sub_issues_summary.total` = 100）。落地时不伪造原生挂接，按 graph-engineering.md:77 做两件事：

1. #1778 resolution 评论首行写「本条依 #1323 Notes N-1 预授权判定」；
2. PR 合入后由主控在 #1323 `## Decisions-so-far`（body `:88`，同族样式见 `:92` #1534 行）补一行索引，建议文字：「- #1778 B/E 持久撤回写口落并列子合同 `domain-event-withdrawal/1`（会话 manifest 显式选择，E/B 各自 init 绑定；T 类 N-1，新 ADR <NNNN>）；实施归 #1768。」#1323 body 当前 70,804 字节，远低于 262,144 上限。

必要时在 #1778 加一条评论说明未挂原生子票的原因（#1323 子票数已到 100）。

## 引用自证

| 引用 | 复核命令 | 结果 |
|---|---|---|
| §5.8 `:368-392`、`:115` | `sed -n 368,392p .chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md` | 逐字可见「不重写、不重铸」「不授权……热迁移」「均明确拒绝」 |
| store.rs `:15-20/:26-27/:64-66/:108-125/:207-236/:276-297/:801-806/:969-1015/:998-1013/:1099-1150`（else `:1132`，E `:1141`，B `:1147`） | `git show origin/main:rust/src/economic_session/store.rs \| sed -n …` | 与正文引述逐字相符；`:232-233` 为 `sr_expansion &&` 条件式 |
| sr_expansion.rs `:9-15/:33-35/:281-296/:329-336` | 同上 | 相符 |
| finance.rs `:17-25/:20-22/:55/:67-85`；book.rs `:7-13` | 同上 | 相符 |
| e1bd49a `store.rs:230/:1100-1114/:1228-1251/:1369-1371/:1426-1440`、`service.rs:1462`、`domain_event.rs:16/:460/:510-511/:531-534/:605-700` | `git show e1bd49a5b67aea729aa39ad983d93a58db16f72e:<path> \| sed -n …` | 相符；`:230` 无条件列入 |
| ADR 0051 `:7-8/:236/:273/:370/:374/:394` | `git show origin/main:docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md \| sed -n …` | 相符；本地 checkout 无此文件，须用 origin/main |
| ADR 0041 `:9/:13/:17-23` | 同上 | 相符 |
| CONTEXT.md `:457-469`（「只追加，撤回也是一条新的域事件」在 `:460`） | `git show origin/main:CONTEXT.md \| sed -n 457,469p` | 相符 |
| 提交 `8f42dece50`(09-30)、`c0ed0497a1`(09-14)、`3b1a1841cd`、`499841b592`、`0fe8e2f3c9`、`92dc58eafc` | `git log -1 --format='%h %ad %s' --date=short <sha>` | 相符 |
| #1768 body `:19/:45`、首条评论 2026-10-27T20:40:35Z | `gh issue view 1768 --json body,comments` | 相符 |
| #1778 body `:25`（#1776 笔误，2026-10-27T23:42:02Z 已更正）与 2026-10-27T23:17:54Z 评论；#1776 标题 | `gh issue view 1778 --json body,comments`；`gh issue view 1776` | 相符 |
| #1778 parent 404；#1323 `## Decisions-so-far` `:88` | `gh api …/issues/1778/parent`；`gh issue view 1323 --json body` | 相符 |
| #1768 blocked_by 含 #1778 | `gh api repos/xy7365527-lang/NewChanlun/issues/1768/dependencies/blocked_by` | 1534(closed)、1778(open) |
| `1772-CONTRACT-ROUTING-28S.md`、`1772-OWNER-REPAIR-28Q.md:13-14/:27/:29` | `cat` / `sed -n 10,30p` | 相符 |

## 考古对照

| 点 | 归类 | 落点 |
|---|---|---|
| Q1 落在哪一层 | B | 裁定 1（C1），C2 保留为合法替代，C3 排除 |
| Q2 选择子与绑定形状 | B | 裁定 2、4 |
| Q3 选择粒度 | B | 裁定 3、4（全会话） |
| Q4 外壳是否升版 | A | 裁定 1 末句，沿 ADR 0051:236 |
| Q5 审计四处归属 | B | 裁定 5、6 |
| Q6 迁移与回放边界 | A | 裁定 4，沿 §5.8 与 store.rs:1003/:1013 |
| Q7 是否自由度 | A | 裁定 9，沿 ADR 0041:9 |
| Q8 HIGH-1/HIGH-3 | B | 裁定 8 |
| Q9 #1776 笔误 | A | 第六节 |
| Q10 正本落点 | A | 验收 7、第七节 |
| Q11 旧二进制 | A | 裁定 10 |
| Q12 map 挂接 | A | 第八节、验收 8 |

C 类 0，无 research / probe 票要开。
