# ADR 0053：B/E 持久撤回写口落并列子合同 `domain-event-withdrawal/1`

> 正本：本文件是 B/E 持久撤回写口（`WithdrawDomainEvent` 与回执 `DomainEventWithdrawn`）合同落点、选择、绑定、审计、拒绝与迁移边界的正本。
> 最后裁定：#1778（2026-10-28，#1323 Notes N-1 预授权 T 类判定）
> 受影响代码清单：见本文「受影响代码清单」节（`store.rs` / `service.rs` / `domain_event.rs` 门控点，实施归 #1768）

**状态**：T 类裁定，文书已合入 main（PR #1779，squash `df9feea2a9fbf7d327684d81e3e1039ca8e75ffc`）。本 ADR 只定合同落点与版本门；门控实现、测试锁与 Python 读者兼容核对归 #1768（TB-07-E）原 owner，不以本 ADR 声称实现完成。
**裁定依据**：[resolution 评论](https://github.com/xy7365527-lang/NewChanlun/issues/1778#issuecomment-6456496599)（2026-10-27T23:50:49Z）。独审第二轮 28I R1–R8 全 PASS，三项 LOW 已在 28J 修订措辞，语义不变。原件：草稿 `/Volumes/AgentStorage/decisions/1778/draft.md`，考古 `archaeology.md` / `archaeology.json`（同目录），独审 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1778-INDEPENDENT-REVIEW-28I.md`，LOW 修订 `1778-LOW-FIX-28J.md`（同目录）。
**行号基线**：裁定时固定到 main `c0c5a369`；本文落地时 main 为 `5123c9e273`，所引 `rust/src/economic_session/`、ADR 0041、ADR 0051、`CONTEXT.md`、§5.8 原件在两者间无差异，行号照用。`e1bd49a5` 是 PR #1772 返修候选（head `89edf18f` 的快进后继，未推 PR 分支、未合 main），`domain_event.rs` 只存在于该候选。
**与 ADR 0051 的关系**：ADR 0051 一字不改。它定外壳 `domain-event/1` 与撤回语义（裁定三）；本 ADR 只定 B/E 的持久撤回 op 落在哪一层合同。

## 问题

#1768（TB-07-E）在 PR #1772 的异家族独审被三项 HIGH 阻塞。返修 `e1bd49a5` 为 HIGH-2 新增了持久写口 `WithdrawDomainEvent` 与永久回执 `DomainEventWithdrawn`，并且：

- 把该 op 无条件写进 `economic-session/1`、`/2` 的封闭已知 op 集合（e1bd49a `store.rs:230`）；
- 对 E 与全部 B 无门控分派（e1bd49a `service.rs:1462`）；
- 在审计的 history 重放、回执、回执主键、消息绑定四处加了无版本门控的分支（e1bd49a `store.rs:1100-1114`、`:1228-1251`、`:1369-1371`、`:1426-1440`）。

后果已核实：一个 `/1` 或 `/2` 域只要被新二进制写过一次撤回，main 二进制审计该提交时落入 `store.rs:1132` 的 else 分支，按 `recorded` 末项核对（`:1139` `verify_record`；E 在 `:1141` 核 `recorded_seq`/`commit_ns`，B 在 `:1147` 核 `b_observation.commit_ns`），E 与 B 都报 `StorageUnavailable`。这改变了已登记修订的持久表示。

#1768 body `:45` 写明「持久表示若须新增 protocol/schema revision……停下交主控；不在本票内自定新修订」。原实现者首条交付评论（2026-10-27T20:40:35Z）卡点 1 与返修报告 `1772-OWNER-REPAIR-28Q.md:27` 都把此事交主控裁定，主控开 #1778。

本 ADR 裁的是：持久撤回 op 落在哪一层合同，以及选择、绑定、审计、拒绝与迁移边界。

不裁的：`domain-event/1` 外壳本身（ADR 0051 已裁）、撤回语义（ADR 0051 裁定三已裁）、HIGH-1/HIGH-3 的安全修复（与合同落点无关）、任何资金、场所、权限或真实启用。

## 权威依据

**已批准 SPEC 增补 #1394 §5.8**（`.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md:368-392`）：

- 「旧 manifest、plan、grant、receipt、经济 root：**不重写、不重铸。** 旧 reader 和原 native Hash 继续解释旧历史」；
- 「旧库不原地改格式，旧金融 manifest 不自动换 binary；本节不授权任何带未结责任会话的热迁移」；
- 「未知版本、混合域、旧 validator 收到新字段……均明确拒绝」；
- 同文 `:115`：「economic-session/2；只用于全新测试会话，旧域不混写」。

**正本层**：

- ADR 0051 `:236`「`contract` 固定写 `domain-event/1`；改形状即新版本」；`:273` 撤回 = 追加一条 `kind: withdrawn` 域事件；`:370`「本 ADR 零代码改动」；`:394` 批准范围 = 外壳合同与 13 项默认值。全文没有定义 wire op、回执 kind 或 socket 接口。
- ADR 0041 `:9` 本通则只管缠论自由度；`:23`「改签即新版本——改值＝新版本……不在旧版本内热改」。
- `CONTEXT.md:460` 域事件词条：「只追加，撤回也是一条新的域事件」。

**代码判例**（main）：

- `store.rs:108-125`、`:276-297`：`Config::load` 对 E 与全部 Bᵢ 读同一份会话 manifest，只按 `chong` 参数区分域，`manifest_hash`（`:293`）相同——不存在「本域 manifest」。
- `store.rs:15-20`：E/B 域版本唯一登记点，`:17-18`「旧域不原地升级、不混写，未知值当场拒绝（§5.8）」；`:26-27` 选择子 `protocol_revision`；`:207-236` 封闭 op 集合，`:232-233` `sr_expansion` 的 op 只在子合同选中时算已知；`:801-806` init 把子合同版本写 meta（`book_revision` `:802-803` 不按域区分；`sr_expansion` `:805-806` 只 E）；`:998-1013` audit 核 meta 与 config 一致，否则「IdentityConflict：……不得在 recover 暗迁移」。
- `sr_expansion.rs:9-15`：「新语义是与旧合同并列的子合同 `sr-expansion/1`：manifest 顶层 `sr_expansion` 显式选择，E 在 init 绑定、audit 核对，旧域不原地扩义。它不占财务版本号，声明在 `economic-session/1` 与内容域 `economic-session/2` 上」；`:33-35` 字段/版本/meta 键；`:281-296` registry 核 `schema_revision`；`:329-336` `bound_revision` 只认 E，是该合同头注自定的范围。
- `finance.rs:17-25`：`:18`「`/1` 的子合同也不得靠改名接管新域」；`:20-22` `tb03b-finance/3` 是 `economic-session/1` 域上显式选择的窄测试臂（`:55` 登记）——子合同可以声明在 `/1` 上，`/1` 已知语义不变；`:23-25` `/4` 在 `/2` 域上显式选择；`:67-85` 合同由 `finance.schema_revision` 选，与域版本混写报 IdentityConflict。
- `book.rs:7-13`：`tb03c-book/1`、`/2`（`/3`）由 manifest 顶层 `book_revision` 冻结，作用于该会话全部 E/B 域——全会话选择的判例。
- 时间线（`git log -S`）：§5.8 版本登记落码 `8f42dece50`（2026-09-30）之后，四次新增持久 op/语义（#1674 book/2、#1715 book/3、#1716 finance/4、#1717 sr-expansion/1）全部走子合同；直接加进封闭集合的 `BindVoice`（`c0ed0497a1`，2026-09-14）在 §5.8 之前，那时集合本身就是 `/1` 的定义。

## 候选

**C1（采纳）并列子合同 `domain-event-withdrawal/1`**，照 `sr-expansion/1` 判例。manifest 顶层显式选择，选择覆盖该会话全部 E 与 Bᵢ，各域 init 绑定 meta、audit 核对；`/1`、`/2` 的已知集合、表集合、`recorded` 判据、回执族逐字不变，main 二进制继续能审计任何未选择本子合同的域。代价：返修 head 必改 `store.rs:230`、`service.rs:1462` 与审计四处的门控，CI 与独审须在新 head 重做；manifest 多一个顶层字段，测试夹具要加。

**C2（保留为合法替代，不采纳）新域版本 `economic-session/3`**。把撤回 op 作为 `/3` 域定义的一部分，符合 §5.8「新试验域产生自身版本的新身份」。不采纳的理由是分量不相称：`finance.rs` 的 `Contract.domain` 与 `CONTRACTS` 表、`book`、`sr_expansion`（`:13` 声明只在 `/1` `/2` 上）、`TABLES`/`CONTENT_SQL` 判据、Python 读者的 `protocol_revision` 分派都要为 `/3` 重登记，而一个追加式 op 不值得开整个域版本。它没有被否，留作将来撤回语义与其他新语义一起成版时的路径。

**C3（排除）原地扩义 `/1`、`/2`（即 `e1bd49a5` 现状）**。它正面违反 §5.8「不重写、不重铸」「旧 validator 收到新字段……明确拒绝」与 `store.rs:17-18`「旧域不原地升级、不混写」，也与 §5.8 之后四次判例相反。实测后果是 main 审计撤回过的域报 `StorageUnavailable`（`store.rs:1132-1150`），等于新二进制单方面废除旧二进制对已登记修订的解释权。它不是 ADR 0041 意义上的另一种合法取值，是对已批准迁移拒绝边界的违反，照 ADR 0041 裁定四排除。

## 裁定

1. **落点**：B/E 持久撤回写口 `WithdrawDomainEvent` 与回执 `DomainEventWithdrawn` 落为与 `economic-session/1`、`/2` 并列的子合同 **`domain-event-withdrawal/1`**，不占域版本号，不改 `domain-event/1` 外壳。`/1`、`/2` 的已知 op 集合、表集合、`recorded` 判据与回执族逐字不变。
2. **选择**：manifest 顶层 `domain_event_withdrawal` 对象，exact 字段 `{"schema_revision"}`，值只认 `"domain-event-withdrawal/1"`；缺省或 `null` 为未选择；其他值或形状报 `SchemaUnsupported`。
3. **选择范围（全会话）**：E 与全部 Bᵢ 读同一份会话 manifest（`store.rs:108-125`、`:276-297`），裁定 2 的选择子作用于该 manifest 所含的 E 与每个 Bᵢ，不设逐域列表字段。判例是 `book_revision`（全会话）；`sr_expansion::bound_revision` 只认 E 是该合同头注（`sr_expansion.rs:9-15`）自定的范围，本合同覆盖 B/E（ADR 0051:273、#1768 范围），所以本合同的 `bound_revision` 不按 `config.domain` 过滤。S 不在本合同内：S 的撤回仍是 `s_structure_session` 的 `withdrawn_generation` 字段（ADR 0051 裁定三）。
4. **绑定**：选择后 E 与每个 Bᵢ 在各自 init 时在本域 `meta` 写 `domain_event_withdrawal_revision = "domain-event-withdrawal/1"`；各域 audit 与 recover 核该 meta 与同一 manifest 的选择（`bound_revision`）相等，不等报 `IdentityConflict：撤回子合同版本未由 init 显式绑定；不得在 recover 暗迁移`。未在 init 绑定的域此后不得补绑；旧库不迁移（§5.8）。
5. **已知与分派**：`WithdrawDomainEvent` 只在本会话已选择、且本域 meta 已在 init 绑定时进已知 op 集合（与 `sr_expansion` 同式），manifest `operations` 仍是来源权限闸，须显式授予；`service.rs` 只对满足该条件的域分派，否则 `InvalidDomain`。
6. **审计**：history 重放、回执审计、回执主键、消息绑定四处的撤回分支都以「本会话已选择，且本域 meta 已在 init 绑定」为前置；不满足该条件的域的持久图像里出现 `kind = WithdrawDomainEvent` 的 history 条目或 `DomainEventWithdrawn` 回执，审计报 `StorageUnavailable：域损坏`。写路径与审计共用 `withdrawal_image` 判据（#804）。
7. **语义不变**：撤回条目与回执的字段、去重键（被撤回事件身份）、同内容 no-op / 异内容 `IdentityConflict`、撤回不可再撤回、`superseded_by` 须指向已存在事件，均沿 ADR 0051 裁定三与 `e1bd49a5` 现实现，本 ADR 不改。
8. **与 HIGH-1 / HIGH-3 的关系**：delta 事件集合完整校验与 snapshot/delta 读前全域审计不依赖本子合同，原 owner 返修时原样保留。
9. **不属自由度**：本机制是工程版本门，不是 ADR 0041 的缠论自由度，不落可调字段；版本串改值即新子合同版本。
10. **旧二进制**：main 不对 manifest 顶层键与 meta 键做 exact 校验（`store.rs:108-125`、`:969-1015`），所以旧二进制忽略 `domain_event_withdrawal` 顶层键与新 meta 行，不识别本子合同。来源 `operations` 授予 `WithdrawDomainEvent` 时，旧二进制在加载时于 `store.rs:235` 报 `InvalidDomain：操作权限未知或重复`，整个会话起不来，不能识别或执行新 op。撤回条目只能写进授予了该 op 的会话（授权集合随 `manifest_hash` 冻结），所以旧二进制碰不到撤回条目。§5.8「旧 validator 收到新字段……明确拒绝」在这里由未知 op 授权间接实现，与 `sr_expansion` 判例同式；已选择但未授予该 op 的会话不含撤回条目，旧二进制按原合同可审计。

## 受影响代码清单

全部改动归 #1768 原 owner，在 `e1bd49a5` 之后的新 head 上实施（快进推至 `sandcastle/issue-1768`，不 force），PR #1772 的 CI 与异家族独审在新 head 重做。

| 文件:行（`e1bd49a5`） | 现状 | 按本 ADR 改为 | 裁定 |
|---|---|---|---|
| `rust/src/economic_session/store.rs:230` | `super::domain_event::WITHDRAW_OP` 无条件列入 `/1`、`/2` 封闭 op 集合 | 只在本会话已选择且本域 meta 已在 init 绑定时算已知，与 main `:232-233` `sr_expansion &&` 同式 | 1、5 |
| `rust/src/economic_session/store.rs`（`Config::load`，对应 main `:108-125`、`:276-297`） | 无选择子 | 解析顶层 `domain_event_withdrawal`，exact `{"schema_revision"}`，只认 `"domain-event-withdrawal/1"`，缺省/`null` 未选择，其他报 `SchemaUnsupported` | 2、3 |
| `rust/src/economic_session/store.rs`（init meta，对应 main `:801-806`） | 无 | E 与每个 Bᵢ 写 `domain_event_withdrawal_revision`，不按域过滤 | 3、4 |
| `rust/src/economic_session/store.rs`（audit meta 核对，对应 main `:998-1013`） | 无 | meta ≠ `bound_revision` 报 `IdentityConflict：撤回子合同版本未由 init 显式绑定；不得在 recover 暗迁移` | 4 |
| `rust/src/economic_session/store.rs:1100-1114`（history 重放） | 撤回分支无门控 | 以已选择且已绑定为前置；否则出现撤回条目报 `StorageUnavailable：域损坏` | 6 |
| `rust/src/economic_session/store.rs:1228-1251`（回执审计） | 同上 | 同上（`DomainEventWithdrawn` 回执） | 6 |
| `rust/src/economic_session/store.rs:1369-1371`（回执主键） | 同上 | 同上 | 6 |
| `rust/src/economic_session/store.rs:1426-1440`（消息绑定） | 同上 | 同上 | 6 |
| `rust/src/economic_session/service.rs:1462` | `WITHDRAW_OP => domain_event::withdraw(..)` 对 E 与全部 B 无门控分派 | 只对已选择且已绑定的域分派，否则 `InvalidDomain` | 5 |
| `rust/src/economic_session/domain_event.rs:16`、`:460`、`:510-511`、`:531-534`、`:605-700` | 撤回条目、回执、`withdrawal_image` 判据 | 语义不改；写路径与审计继续共用 `withdrawal_image` | 6、7 |
| main `store.rs:1132-1150`、`:207-236`、`:946`；`finance.rs`；`book.rs`；`sr_expansion.rs` | `/1`、`/2` 判据与判例来源 | 不改 | 1、10 |
| ADR 0051 `:374` 所列九个 Python 读者 | — | 逐个核兼容（#1768 body `:19`），不在本 ADR 内改 | 1 |

实施验收以 #1778 草稿第五节第 1–6 项为准：选择子缺省时各域 `Config::load` 报 `InvalidDomain：操作权限未知或重复`、加选择子后 Ok；新增测试 `domain_event_withdrawal_op_unknown_without_selection`、`domain_event_withdrawal_selection_binds_e_and_every_b`、`domain_event_withdrawal_recover_without_init_binding_is_identity_conflict`、`domain_event_withdrawal_entry_in_unbound_domain_is_storage_unavailable`、`domain_event_withdrawal_v1_v2_known_ops_and_tables_unchanged` 通过；既有 `domain_event_withdrawn_is_appended_and_old_rows_unchanged` 等三项在选择本子合同的 manifest 上通过；`cargo test --features s_session --lib economic_session` 0 failed（nc-remote 或 CI）；Python 读者核对报告与两条 pytest 命令退出码 0；PR #1772 全部必要 CI 成功后再做异家族独审。

## 受影响票与 blocking 边变化

- #1768（OPEN，TB-07-E）：直接消费本裁定，原 owner 按「受影响代码清单」改门控并补 Python 读者验收。`#1778 blocks #1768` 已存在，不重复接。
- #1778 票面验收第 4 项曾写「建立 #1776 blocks #1768」，是笔误，主控已于 2026-10-27T23:42:02Z 更正。#1776 是无关票。**禁止新建 #1776→#1768 边。**
- PR #1772：head 保持 `89edf18f` 直到新 head 产生；不推 `e1bd49a5` 原样。
- #1366（TB-07 汇总）：#1768 为其子票，不新增边。#1361、#1363、#1399：被 #1768 阻塞，不变。
- #1717 / `sr-expansion/1`、#1716 / `tb03b-finance/4`、#1406 / `tb03c-book/2`：判例来源，不改、不接边。
- #1534（CLOSED，ADR 0051）：外壳不升版，ADR 0051 一字不改。
- #1330、#1539（X 侧）：X 若需持久撤回命令原件，另按本子合同同式选择；本 ADR 不接边。

边增删汇总：增 0、删 0；禁增 #1776→#1768。

## 授权引用

- #1323 Notes N-1 技术类决策预授权（用户 2026-10-18 原话，`.sandcastle/docs/graph-engineering.md:90`）。N-1 只授权本条新决策及本 ADR，不授权回写用户批准的 ADR 0051（ADR 0051:8「本条依用户批复落地，不以 #1323 Notes N-1 代替」）。适用依据是 #1778 票面首行「Part of #1323」；#1778 不是 #1323 的原生 sub-issue（`gh api …/issues/1778/parent` 返回 404，#1323 子票已满 100），合入后由主控在 #1323 `## Decisions-so-far` 补索引行。
- 用户 2026-10-27 对 #1534 的批准（ADR 0051:8）：撤回语义已裁，本 ADR 只裁持久 op 的合同落点。
- #1768 body `:45` 停止条款与首条交付评论卡点 1：把本问题交主控裁定。
- 不含：main 合入、真实资金、场所、权限、隔离策略，以及 #1768 整票或 #1366 的关闭。
