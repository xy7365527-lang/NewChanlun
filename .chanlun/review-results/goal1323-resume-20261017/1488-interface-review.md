# #1488 接口候选 v2 独审

日期：2026-10-17。结论：**FAIL（概念层存在三个确定矛盾；不能按 v2 原签名冻结实施）**。

审查对象：`/private/tmp/goal1323-resume-20261017/1488-interface-candidate.md`；源码基线为 `.sandcastle/worktrees/resume-1488` 的 `c2c9c5640a99b98299456fd8b837c58035a2f4a6`。只读源码、未运行 cargo/服务/外盘。工作树 staged/unstaged diff 均为空。本报告不宣称六成功锁通过。

## 概念层质询结果

本次涉及资源所有权、父工作域、真实 SQL 材料、事务提交资格；未修改或重新解释缠论定义。定义依据为 #1488 评论 `6199095051` 已采纳的 C1/C2/C3，以及候选 §8 自选的完整 publication 强版。核查 `.chanlun/genealogy/` 中 1488/资源签发/材料签发关键词未找到覆盖以下矛盾的既有裁定。相同关键词命中另一个分类学“同父”条目，不构成接口授权。

### 阻塞一：强版在 `Recording::finish` 必然仍有 journal 强引用

候选第 39–46 行令 `ConstructionResources` 持 `Arc<Mutex<Journal>>`，`ConstructionScope` 借资源对象；第 156 行又要求 `finish_fixed_write` 在同一个 `run_resources` 闭包内执行。真实 `effects.rs:464–476` 先卸 hook/TLS，再 `Arc::try_unwrap(self.journal)`。即使所有 MaterialAccess/StageOwner 都已析构，runner 持有的 resources 仍在，至少留下第二个强引用。无法在 scope 借用期间先 drop resources；把 finish 移出 runner 又违反强版同父域。

反例：一条完全正确的普通 publication，调用 finish 时无外泄材料能力，仍返回“效果日志仍有外借”。它会把原成功合同改成必拒绝。

**最小修正**：长寿命资源 token 不持 journal 强 Arc。它持同一 Budget、私有 seal/事务身份，以及 `Weak<Mutex<Journal>>` 或不拥有 journal 的绑定身份。只在签发/校验时 upgrade，立即 drop；MaterialAccess/StageGuard 可以暂持强 Arc，但在 finish 前全部析构。`Weak::ptr_eq` 配合当前 journal/封闭 seal 核验即可，不需 native/scratch getter。`ConstructionRun` 仅含已尾检标记/返回值/身份，不含 journal。

### 阻塞二：局部 journal cap 被当成整个 publication 父帽，旧载荷没有入同一账

候选第 69 行将原 `Budget::new(j.cap,j.bytes)` 原样前移；但 `audit_owner.rs:2150–2182` 的 journal cap 是 `(limit / 8).min(limit - candidate.charge.total)`，其中 limit 已减去 raw 保留费。该账既不包含 candidate，也不是 16/8 MiB 的完整父额度。把 `validate_effects` 的新 Budget 删除并接到它上面，只会漏记旧载荷或过早拒绝。

同时 `effects.rs:1516–1524` 用 `j.cap - budget.live()` 算 stage 原件余量；若 budget 改成全局账，candidate 很容易大于 journal cap，普通 stage 会下溢。不能只改入参而保留这套“局部 cap = 父 live”算法。

**最小修正**：把 journal 子限额与全局父账明确分开。全局父账由 owner 持有；journal 只对其自身保留费/峰值实施既有局部上限，所有实际分配同时占同一父账。剩余允许量是“局部余额”和“父余额”的受检最小值，不能从局部 cap 减全局 live，也不能抬局部帽。父 limit 由 owner 已冻结合同记录，不能从私有 Ledger 读取。

**旧 candidate 能否不改禁区连续承接：可以表达，但 v2 没有实现这条接线。** `CompleteCandidate::capture` 在 `audit_owner.rs:1586–1620` 已经拿到了准确 parent：Some 分支 clone 原 cold parent，None 分支在那里建立初始预算；当前构造 Self 时把 budget 丢掉。应从这个既有构造点开始增加最后析构的资源保留字段，保存同一 Budget 及实际收费项目的归属，不在 publication 入口新建“相同数字”的账。`request_audit.rs:1602` 已有 `cold_parent_budget()`，不需修改禁区或新增 getter。

之后跨 `Ended → owner.complete → active.complete → pending → complete` 移动同一保留字段；`detach_slot` 必须在 slot/result/binding 真 drop 后才释放对应已收费项目。保留费表须区别 `Permit::commit` 已入父账的费用、NativeReservation 自行析构的费用和仅统计字段，不能把 `charge.total` 无差别重新 reserve，也不能把 `refresh_capture_charge()` 的统计差直接当可释放金额。`PendingSet.resource_hold` 只管 set，不能替代 CompleteCandidate 及 raw/source 的存活费用。预算域变化必须在 owner 合同层处理，不能静默扩大旧 Ledger，也不能把原成功用“域不同”全部拒绝。

### 阻塞三：runner 从 raw advance 后开始，仍然存在第二个父账/工作域

`AuditConnection::publish:511–526` 先调用 `with_bound_raw_advance`，再装 journal。该函数 `audit_owner.rs:1044–1076` 自建 Budget、codec_work，并把 PendingRawState 留在 active。候选只把 runner 包在后续 publish/finish 上，即使修掉 validate_effects 两个 work，也未覆盖这些存活原始来源载荷。保留“原 raw 签发顺序”不等于保留原第二套资源域。

**最小修正**：publication 的唯一 runner 在 `bind_existing` 完成后、`with_bound_raw_advance` 前启动。从已绑定 candidate 获取其 owner 保留资源，短借后 clone Budget；闭包依次执行 raw advance、before_fixed_write、journal 附接资源、生成/三 cut/材料、source 回交、capture 费用核算、finish_fixed_write。改 raw helper 为接 `&PublicationScope`，删除该路径自己的 Budget::new/codec_work.run；`PendingRawState::advance` 已接受 NativeAdmission(Some(scope))，禁区 API 无需改。

journal 此时尚不存在，所以资源根发行与 journal 附接必须分成两个不可伪造步骤；后者只能消费本次唯一发行的身份，不得再次创建资源域。不能继续保留 `issue_resources(tx)` 必须 current_journal 的原签名，又声称 runner 已包住 raw advance。

## 建议立即冻结的 R/S 修正版接口

以下是模块契约，不是声称已编译的代码。名字可统一一次调整，但责任不可分叉。

```rust
// R：owner 内唯一签发；字段私有，不 Clone。
struct PublicationResources {
    budget: native::Budget,       // 从 candidate 既有 hold clone，同 Ledger
    parent_limit: usize,          // owner 冻结合同；不查私有 Ledger
    seal: SnapshotSeal,
    // 不含 journal 强 Arc；发行/附接状态由 owner 私有状态验证
}
struct PublicationScope<'w> {
    resources: &'w PublicationResources,
    work: &'w WorkScope,
}
struct ConstructionRun<T> { value: T, before: SnapshotSeal, after: SnapshotSeal }

// owner 私有方法在 bind_existing 后调用；tx 借用不得保留到返回值。
fn issue_publication_resources(&self) -> Result<PublicationResources, String>;
fn run_resources<T>(
    resources: PublicationResources,
    f: impl FnOnce(&PublicationScope<'_>) -> Result<T, String>,
) -> Result<ConstructionRun<T>, String>;

// R：before_fixed_write 已安装 journal 后附接，检查本次资源发行身份。
fn attach_recording_resources(
    tx: &Transaction<'_>, parent: &PublicationScope<'_>,
) -> Result<(), String>;
fn with_material_access<T>(
    tx: &Transaction<'_>, parent: &PublicationScope<'_>,
    stage: Option<&StageOwner<'_>>,
    f: impl FnOnce(&MaterialAccess<'_, '_, '_>) -> Result<T, String>,
) -> Result<T, String>;

// S：stage 只长期借 work，不长期借 tx；身份用 Weak/封闭印记，不持 journal 强 Arc。
fn StageOwner::new_admitted<'w>(
    tx: &Transaction<'_>, parent: &PublicationScope<'w>,
) -> Result<StageOwner<'w>, String>;
fn StageOwner::read_scoped(
    &self, digest: &NodeDigest,
) -> Result<Option<ScopedFrame>, CodecError>;
fn StageOwner::flush_cut(
    &self, tx: &Transaction<'_>, cut: FlushCut<'_>,
) -> Result<(), String>;
fn NativeReadLease::new_admitted_staged<'a, 'db>(
    tx: &'a mut Transaction<'db>, work: &'a WorkScope,
    stage: &'a StageOwner<'_>,
) -> Result<NativeReadLease<'a, 'db>, CodecError>;
```

借用/析构顺序必须是：短借 candidate 拿资源 → 释放 RefCell guard → 进入唯一 work → raw helper 短借 complete → helper 返回释放借用 → before_fixed_write 移 candidate 到 pending → 附接 journal → pending 拆出 audited/capture，生成 lease 独占 tx → drop lease → 注入实际根 → source flush → 一次材料签发并处理 row/final → drop 材料、stage 及所有 journal 强借用 → source/install 与 charge 对账 → drop pending RefMut → finish_fixed_write（含 validate_effects，仍借同 work）→ work.run 尾检 → ConstructionRun → commit_publication。首错短路回滚；不能把 receipt 提前交给调用者。

R 的 `with_bound_raw_advance`、`finish_fixed_write`、`CompleteCandidate::validate_effects` 都应显式接 parent；begin/accept/recover 等非 publication 入口保留其原资格语义，可用独立入口，不伪造 publication token。所有继承父账的分配 helpers 只接借用，不自行再开 CodecWork。签资源/附接/签材料各自一次性状态必须有失败后不可重试锁。

`ConstructionRun` 还必须区分发行 seal 与写后 seal：`before_write_inner:1092–1103` 会推进 write_sequence，`finish_fixed_write_inner` 会更新 candidate seal。因此不能把入口 seal 原样拿去与 commit 的当前 write_sequence 比较。after 只能从 owner 完成 finish 后的真实封口状态取得，验证同 life/transaction、唯一预期写序演进及原快照约束，再经 run 尾检生成标记；不能由下游任意填 after。源/材料的 seed seal 继续使用原裁定语义，不混用两个时点。

## S 方案中可继续实施的部分

这些判断不解除上面的 FAIL，也不等于工程验收通过：

- 带许可 StageOwner 和 staged NativeReadLease 可由现接口表达；生成期间只借 stage/work 与独占 tx，lease drop 后才允许 SQL flush。保持 SameMaterial 仅覆盖 SQL，不能缓存 overlay 证明。
- source 使用注入后实际根，先真实 SQL flush，再执行现 source audit；不能用 stage.reader 代替材料 SQL 读。
- row cut 在 StageGuard 完整审计前做；先短锁确认唯一实际非 Node 行，解锁后读根/flush，再重新锁和回读审计。新增 Node 写入仍记录在实际 stage 内，不能抹 coverage。
- final 入口仍是 commit.rs:278 附近；必须显式传本代 publication/catalog/evidence/runtime 根，不能全表扫描旧历史，也不能只传 runtime。flush 之后尚未 SQL COMMIT；真实提交位置仍 audit_owner.rs:820。
- 已持有 BTree/Vec 许可不能在节点仍活时减账。返回的复制帧必须独立预留；StageOwner 自身存储许可不能借给复制帧。
- `content_store.rs`、`content_publication.rs` 纳入 S 是必要范围；原四文件权限不够。原 16/8 MiB、六成功合同、三个禁区、所有资格与尾检拒绝保持。

## 有限落地步骤与验收边界

R 先落实上面三个确定修正及 owner ledger 持有；S 可并行做许可化 stage/store 与三 cut，但不得在 R 能力接通前删除 staged guard。R/S 统一上述类型和调用职责后直接写实现，不需要再重建七份设计。

R 必须给出具体费用迁移表（冷 candidate → raw pending → journal/set → effects/capture → next candidate）：每项是旧费保留、真实 drop 释放或新费预留，以及收费/释放行号。这是排除重复收费和无账载荷的实施证据，不接受仅“同一 Budget clone”作为账目已正确的结论。

针对上述确定反例至少补三锁：finish 前 journal 强计数可 unwrap；旧 candidate 大于 journal 子帽但总工作集在原全帽内仍成功；raw→generation→validation 捕获同一 scope 身份且只有一次 run。另保留尾检失败禁止 COMMIT、复制帧许可随载荷、同 seal/journal/transaction拒伪造和原六成功锁。此轮不执行重型验证。

## 工程层审查

因概念层矛盾，按审查规则未进入工程层审批，不填写零缺陷 PASS 表。当前 FAIL 针对候选，不代表已实现源码引入了新故障。只要上述修正落在实际代码且费用迁移闭合，可以进入工程独审；远端六测结果仍须另行取得。

记忆仅用于提醒“静态审查不能替代远端/历史验收”；全部上述源码/票评论事实本次重新读取核实。
