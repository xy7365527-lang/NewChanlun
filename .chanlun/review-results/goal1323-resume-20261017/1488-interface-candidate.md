# #1488 R/S 精确接口重建候选 v2（2026-10-17）

状态：**新候选，待独审；不是丢失七件设计的恢复件，不是已实现/已通过的冻结接口。**

基线 `/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1488`，`c2c9c5640a99b98299456fd8b837c58035a2f4a6`。只读源码，未 cargo、未外盘、未改源码。唯一写出本文件。依据：本目录 `1488-recovery.md`、`1488.json` 最末 `6199095051` 评论以及下列实码。已读根 CLAUDE、Serena manual；图工具没有本恢复树索引，故回落文件阅读。谱系 pending 的相关关键词搜索没有找到另行裁定本资源接口的条目；这不是全域无矛盾证明。本候选不改缠论定义，C1/C2/C3 取已裁评论；新 Rust 接口和跨账交接选择仍暂定。

## 1. 先给可执行判断

三个禁区 `construction_state.rs`、`audit_owner/native.rs`、`s_content/capture_allocation/scratch.rs` **不必改 API 才能表达本候选**：已有 `Budget::clone/reserve/codec_work/release`、`WorkScope::reserve_native`、`NativeReservation`、`ScopedFrame` 和 `PendingRawState::close_set(PendingSet, actual_root)` 足够承载。

但只改 `effects.rs/audit_owner.rs/staged_nodes.rs/prepared.rs` 四文件 **不够**。至少还必须改共享 `v2/content_store.rs` 和 `v2/content_publication.rs`、`v3/commit.rs`，以及用于连续持有资源账的 `v3/construction_set.rs`。这不是可选重构：现 staged store 明确丢读许可、NativeReadLease 明确不支持 staged、发布 resolver 明确传 None scope。若文件权限被限制为原四文件，则本候选为 BLOCKED，不能开工后暗扩面。

目前不能声称新候选一定满足原 16/8 MiB 成功合同。必须在相同预算实跑六项；容器全额计量后如确实超帽，优化真实载荷/消废，不能释放尚活载荷的账或升帽。这是验证条件，不是把成功改拒绝的备选。

## 2. 现码直接证据

| 位置 | 已核事实与必要动作 |
|---|---|
| `effects.rs:1169-1318` | with_material_access 同时发资源和材料；`:1186` 用 resources.is_some 判断重复；`:1269` 无外 work 时自开 CodecWork，闭包结束即关域。拆开两次不同能力签发，不能删除一次性限制。 |
| `effects.rs:198-204, :573` | journal 载荷先 Drop、outer_cap 最后；finish 把 outer_cap move 给 PendingSet。新方案须保留同等连续持有，不能用新 budget 替换已有载荷的许可。 |
| `prepared.rs:328-336, :564` | staged+work 尚未接入的拒绝和 staged/native lease 拒绝确实存在。只有新 typed staged lease 实际接通后才能把这两条能力缺失检查改成“同 parent + 同 journal + staged lease 已装配”校验。 |
| `prepared.rs:576-619` | 第一段 NativeReadLease 独占 tx；drop 后运行 before_native_source 注入；因此资源能力不得长期借 tx，材料能力必须在这一借用结束后签。 |
| `effects.rs:870-939` | MaterialAccess reader 固定从实际 tx 读，不接受外来 NodeSource；保留。 |
| `content_store.rs:588-750` | NativeReadLease 内部 `&mut Transaction`、SameMaterial、poison、insert-preserve 检查。staged 不能绕回普通旧读核。 |
| `content_store.rs:841-850` | staged checked_read_scoped 当前把 CheckedFrame 包成 permit=None；`:874/:893` stage.read 又克隆 Vec 无许可。这是连续计量硬缺口。 |
| `staged_nodes.rs:21,54,84` | owner 只认 handle，put_new 无父账；flush 最后全表扫根，并真 TxStore.put。需复用 SQL 落库门，改为选择性 cut。 |
| `effects.rs:1465-1547` | StageGuard.finish 锁 journal 后用实际行根审计。flush 不能在持锁时调用 TxStore.put，否则 note_node(:760) 回同锁死锁。 |
| `commit.rs:85-105` | staged 发布把 workspace 和 work 都传 None；`:278` 最后 flush；**这个函数没有 COMMIT**。 |
| `audit_owner.rs:585/:774-820` | 真提交在 finish_fixed_write 后的 commit_publication/commit_as。Runtime 测试兼容入口真提交在 prepared.rs:191/217。 |
| `construction_set.rs:255-325` | PendingSet 的 parent_charge 是唯一 NativeReservation，不能直接承接同一个 Budget 的所有已入账载荷；需要增加最后析构的保留句柄。 |
| `audit_owner.rs:1751-1776,1890附近` | validate_effects 另建 Budget，并有 node_work、append cut work。不得把它当成新父账已全面闭合；见 §8 边界。 |

## 3. R 接口与资源所有权

以下是需实现的 Rust 签名契约，名字是本次新命名。私有字段不允许调用者伪造。所有严格分支仍由真实 Journal/Seal 签发。

```rust
// effects.rs；不借 Transaction，允许随后 NativeReadLease 独占 tx。
pub(crate) struct ConstructionResources {
    journal: Arc<Mutex<Journal>>,
    seal: SnapshotSeal,
    budget: native::Budget, // 与 journal 中同一 Arc<Ledger>，不是第二份 Budget::new
}
pub(crate) struct ConstructionScope<'w> {
    resources: &'w ConstructionResources,
    work: &'w WorkScope,
}
// 不能 Clone，不能裸构造；result 仅在 CodecWork.run 成功后生成。
pub(crate) struct ConstructionRun<T> {
    value: T,
    seal: SnapshotSeal,
}
fn issue_resources(tx: &Transaction<'_>)
    -> Result<Option<ConstructionResources>, String>;
fn run_resources<T>(
    resources: ConstructionResources,
    f: impl FnOnce(&ConstructionScope<'_>) -> Result<T, String>,
) -> Result<ConstructionRun<T>, String>;
fn with_material_access<T>(
    tx: &Transaction<'_>,
    parent: &ConstructionScope<'_>,
    stage: Option<&StageOwner>,
    f: impl FnOnce(&MaterialAccess<'_, '_, '_>) -> Result<T, String>,
) -> Result<T, String>;
```

`issue_resources` 从 current_journal 取得真实 Arc，核 operation/set_required/seal/事务未结束以及 unexpected/overflow。资源发行用单独 `resources_issued`，材料发行用单独 `material_issued`；两者都在尝试前置为 true，失败置 unexpected，不能重试洗账。原 `resources.is_some()` 不能再兼任 material_issued。

正式 owner 原本传入 None work 的入口：把原 with_material_access 里的唯一 `Budget::new(j.cap,j.bytes)` 移到 issue_resources，journal 和 ConstructionResources 共享该 budget；再由这唯一 budget 的 codec_work 开唯一 scope，覆盖 generate/source/row/final。不再 reserve 整个 j.cap 后又让同一 scope 为载荷重复收费。

**不把裸 `Option<&WorkScope>` 当父账证明。** 现唯一外部显式 admitted 调用是 prepared.rs:1946 的测试，它自行 parent.codec_work.run 后再 tx.commit。其正常语义保留；若该入口用于严格 owner，迁移为传不可伪造 ConstructionScope，不能从 WorkScope 反查私有 Ledger，也不能另造 scope。非 owner 原有 admitted 入口可保留，调用者外层尾检/提交顺序原样；它不能被升级为 strict owner 凭据。不要为兼容签名偷偷创建替代 scope。需要复用外部父资源时，由同一个 `run_resources` 所发 ConstructionScope 向下借用；不能提供公开 `(budget, scope)` 任意配对构造器。

MaterialAccess 仍持实际 tx，只增加 stage 的受限借用。预算引用来自 parent.resources.budget，work 引用来自 parent.work。签材料核 Arc::ptr_eq、seal、material_issued=false、set/source/stages 仍空；签后整个 source+finish products+所有 row stages 使用同一 MaterialAccess，不二次签发。材料闭包返回检查 set 已封、active_stage 空、无 unexpected/overflow；此函数绝不自行 codec_work.run。

`ConstructionRun` 仅由 run_resources 在 work.run 成功后构造，附真实 seal。owner 的 publication commit 增加消费这个标记的私有入口，核 life/transaction_id/write_sequence，再走原 commit_as；原其它 operation 的 commit 不受影响。若 parent scope 尚在运行，不能制造 ConstructionRun。原 Runtime::commit 测试适配同样在完整资源 runner 返回之后 tx.commit。

## 4. S StageOwner 与 store 精确接口

StageOwner 不长期借 tx，只保留 connection identity，并在严格路径绑定资源 seal/journal identity；每个 cut 都核当前活动事务。借同一 WorkScope 的生命周期，声明所有载荷字段在许可前：

```rust
struct ChargedNode {
    tag: u8,
    body: Vec<u8>,
    body_permit: Option<NativeReservation>,
}
struct StageStorage {
    nodes: BTreeMap<NodeDigest, ChargedNode>,
    tree_permit: Option<NativeReservation>,
}
struct StageOwner<'w> {
    storage: RefCell<StageStorage>,
    work: Option<&'w WorkScope>,
    // connection, seal/identity, phase/poison, 原诊断计数
}
enum FlushCut<'a> {
    Source(sc::RefView<'a>),
    Row(&'a [NodeDigest]),
    Final(&'a [NodeDigest]),
}
fn new_admitted(tx: &Transaction<'_>, parent: &ConstructionScope<'w>)
    -> Result<StageOwner<'w>, String>;
fn read_scoped(&self, digest: &NodeDigest)
    -> Result<Option<sc::job::ScopedFrame>, sc::CodecError>;
fn flush_cut(&self, tx: &Transaction<'_>, cut: FlushCut<'_>)
    -> Result<(), String>;
```

旧 StageOwner::new 只服务原无 scope 非 strict fixture，保持现成功行为；strict owner 必须 new_admitted，不能依靠 mode bool 猜资格。

put_new：先查重复并逐字验证；新 body 用 `native_body_reserved(Some(work),body)` 先许可再分配；新整树上界用已有 native_tree_storage，扩容时旧 tree permit 仍持有。将 body、body permit move 到 ChargedNode；插入完成后先替换 tree permit 再 Drop 旧 permit。失败先 drop 已分配 body 再 drop 其许可；不能 tuple 解构让 permit 抢先析构。整树上界不会改称严格 heap 实测。

read_scoped：必须为这次返回的 **复制** 单独预留 body，`CheckedFrame::checked` 后包 ScopedFrame 携许可，不能借 stored body 许可给第二份 Vec。读到 SQLite 与暂存同摘要时核原件相等；不从 overlay 遮住真实库破坏。TxStore.checked_read_scoped 改走这个接口，删除的是 permit=None 漏账实现，而不是资格 guard。非 scoped read/checked_read 若必须返回无 RAII 帧，只能保守 retain 许可到父账结束；主链选择 scoped 路径，避免每次读永久累加。

NativeReadLease 新增 `new_admitted_staged(&mut Transaction, &WorkScope, &StageOwner)` 和私有 optional stage；仍执行原 read_uncommitted/format/poison 检查。SQL SameMaterial 只缓存真实 SQL；暂存读用 stage.read_scoped，暂存写走同一个 TxStore::new_with_stage(...).with_work(Some(work))，不假称物理 INSERT 保持缓存。任何真实 flush 在该 lease drop 后执行，避免 SQL/material epoch 改变时还活着缓存 lease。

## 5. 三个 cut 的真实 SQL 与锁顺序

flush_cut 复用原 tag/长度/子边检查，但只以传入 cut 根闭包遍历；通过 SQL+overlay 读验证当前闭包，对其中仍暂存且 SQL 不存在的原件调用 **无 stage 的 TxStore::new(tx).with_work(Some(work)).put**。禁止裸 INSERT 绕 note_persisted_node。重复 cut 对已有一致 SQL 原件不再次 put；仅真实新插入进 journal，避免无意义 Noop 反复推进 material_epoch。缺节点、冲突、任何 SQL/预算失败均 poison，父 journal unexpected，原始错误短路，事务最终回滚。

每次成功 INSERT 后可移除对应暂存条目：`remove_entry` 得到 ChargedNode，先 drop body 后释放 body_permit；其余未落库未来根继续保留。BTree 容量上界可以保守保留到整个 storage drop；不把 `len` 下降当物理树分配已经下降。todo/seen/roots 也必须是同 scope 的 ScratchVec 或先许可后分配并随真实容器持有，不能只计算 body。

- **source cut**：prepared.rs:616 注入完成后，以 `generated.generated_state` 实际值 `flush_cut(Source(actual_root))`，随后签发 MaterialAccess 并调用 audit_native_source_cut，传同一 actual_root。source_binding 仍由 effects.rs:1101附近 Declaration::read(root) 从该实参构造，不缓存注入前根。capture 仍从 j.nodes 的实际 SQL 读追加，不允许从 StageOwner 直接供“真实材料”。
- **row cut**：不能把整个 row plan 一次性全量 flush。每个 StageGuard 的真实 SQL 行已写后、现有 finish 的实际闭包验证之前，读该行 kind.roots() 列的全部 digest，flush Row。实现优先在 StageGuard::finish 顶部增加私有 prepare_row_cut：短锁取 stage 标识/唯一真实非 Node key → 释放 mutex → ExactRows 从 actual tx 读取本行、取固定大小根数组 → flush → 再进入原 finish 完整校验。原 begin_stage、重复身份、kind 顺序、唯一 SQL 行、实际 tuple、set 双向判据全部保留。flush 期间不持 mutex、不持 NativeReadLease。它新增 Node keys 是允许记录的本 stage effects，不能清空或跳过 coverage。若第一遍行读取之后任何注入改变该行，第二遍原 finish 检验仍必须拒绝；不复用未核读数。
- **final cut**：在 commit.rs 原 flush 点，以当次 manifest/snapshot/delta 根、catalog/evidence/控制行引用，以及 visit_runtime_roots 完整集合为根。row cuts 已确认的持久闭包不再全历史 SELECT 扫根。只取本代新控制引用，旧库完整性由已有旧审计保证；不能漏掉 runtime 独有根。最终 flush 完成后才结束资源 work；SQL COMMIT 仍在外层。

这三 cut 是不同可达集合，不是假设三套 source 判据；语义核/材料验证保持同一个。

## 6. 调用位置与 borrow / move / drop 顺序

1. `AuditConnection::publish :511-526` 原 bind_existing/签 raw/before_fixed_write 保留；在 before_fixed_write 已装 journal 后 issue_resources。该句只短借 tx，返回资源对象不含 tx 引用。
2. 将 `PublishPhase::publish :96-116` → `Runtime::publish_impl_bound :254` 增加 `&ConstructionScope` 下传；资源 runner 只包运行期构造与发布，返回 `ConstructionRun<CommitReceipt>`。
3. `prepared.rs:332` 在 runner 内创建 StageOwner；`:576` 生成 lease 借 tx 与 stage，返回 owned generated/objects/axes，lease drop；运行注入；source flush；材料一次签发；finish_node_products、row stage、目录按原顺序。
4. `prepared.rs:620/764/832` 三个材料调用点统一改借既发资源（各路径仍只一次材料签发）。普通非 owner 分支保留 None 材料；不能把 strict 缺能力当非 owner。
5. `prepared.rs:369` staged publish 新增 work/workspace 参数。`commit.rs:85` 传到底，`content_publication.rs:67` staged resolver 新增 work，不能继续硬写 None。若已有 PreparedManifestWorkspace 不支持 staged，同步使其通过已落库 final 所需模板闭包或 staged NodeStore 接口消费；不得静默丢 workspace 造成重做/未计量。
6. 源状态/Prepared/manifest 的载荷与对应许可在原 owner 或 runner 中继续活着；StageOwner 在 report 读取后 drop，先所有 nodes 再许可证。不能出 runner 后还留 `&WorkScope`，但 owning NativeReservation / ScratchVec 可按已有协议离域（只读/析构，不能扩容）。
7. run_resources 返回后 WorkScope 已由 scratch.rs:293-317 做 active=false 和 parent.health 尾检，才产生 ConstructionRun。owner 保留其 value 不向用户交成功；后续 install_source/refresh_capture_charge/finish_fixed_write 原校验照跑，任一错误销毁 receipt 并回滚。最后消费同 seal 标记调用 commit_publication。
8. finish_fixed_write `Recording::finish` 要 `Arc::try_unwrap(journal)`；因此 ConstructionResources/MaterialAccess/StageOwner 的 Arc 借用在此之前全部 drop，仅 journal 自己保留预算句柄。这一点不能把整个资源 token 放 PendingSet（会阻止 try_unwrap）。应把不含 journal 的纯预算保留句柄移动给 PendingSet。

## 7. 连续许可交接，禁止“换账等于释放”

增加 `construction_set.rs` 最后字段 `resource_hold: Option<ResourceHold>`，保留旧 parent_charge 兼容其它现调用。ResourceHold 定义在 effects 或独立内模块，**只含同一个 native::Budget 和它拥有的未释放保留费记录，不含 journal Arc/tx/work 引用**；不是 Budget::new。

Recording::finish 在仍持预算时构造 changes、核 set；把存活 set 与对应 ResourceHold 一起 move，填充不能在 drop 原持有者之后。Journal 的已真实释放容器费用可从原 ledger.release，转移后的载荷费用不得释放。错误出口同样通过字段顺序或显式 Drop：载荷先、费用句柄后。`construction_state.rs:426 close_set` 按值接整个 PendingSet，已经足够把新字段连同载荷放入 pending_set，无需改它。

ResourceHold 的精确 bytes 必须来自原实际增量费用（journal keys/nodes/stages/consumed、set.retained_bytes、rows 的实际 capacity），而不是重新计算一笔“类似的预算”。资源元数据费保留在同一 Budget，许可迁移不是 reserve→release 一次换账。`NativeReservation::retain` 是保守不释放，不是给活载荷提前减账；不得用它当无限账逃逸。

新增 ConstructionRun 只增加尾检资格，不改变 `check_commit`、`close_set`、`close_effects`、实际 total_changes、owner 退役或回滚门；任何原检查失败仍失败，不能凭新 token 越过。

此处的实现验收必须抓住存活集合：run 内、run 返回、Recording.finish、PendingRawState.close_set、commit成功/rollback分别断言 surviving payload 的原 parent 仍被 ResourceHold/许可持有；不只断言最终 live=0。

## 8. 不得模糊的范围与未封口技术风险

- 本候选解决 strict 构造的 generate/source/row/final 同父域，**不能据此宣称完整 Native owner 单一父账已完成**。`CompleteCandidate::validate_effects :1766` 现确实 `Budget::new(limit,self.charge.total+effects.bytes)`，还自开两次 work。若 C3 被解释为包含这个后续全投影验证，R 必须继续把 validate_effects 改为接 `&ConstructionScope`，去掉该路径新 Budget/new work，并把 finish_fixed_write 放到同一个 runner 内；同时要先 drop 材料/StageOwner/资源 journal Arc，再允许 Recording.try_unwrap。仅给函数增加参数而保留新 Budget 是违规。
- 强版迁移需要记录“旧 candidate 已有父账费用”与“新 journal/set 费用”精确相加一次；现 WorkScope 不提供父账获取接口，禁止从私有 native/scratch 增 getter。这必须由上层签同一 ConstructionScope，再顺传，不能下层猜 parent。同一 scope 全程下传在三个禁区约束下可表达，但不是四文件小补丁。
- **本候选选择全 publication 强版**：后验证也纳入 runner，不采用把 C3 收窄到 Runtime 的解释。§6 第7步的外层顺序应落实为“source移交/保留费校验/finish_fixed_write 都仍在同一次 run 内 → run 尾检 → 消费 ConstructionRun → 原 commit_as”。§6 的分层位置描述用于说明现码差异，不能把它当成允许后验证另建账的例外。未实现强版前，不得派“删 staged guard”的实施。
- 即使强版接口类型可表达，旧 live 账向同父账承接/退役顺序与 16/8 MiB 六成功尚未实证；独审必须在这个新候选上审，不能继承旧七文件“已通过”结论。

## 9. R/S 分工、可审查完成条件

R 拥有 effects.rs、audit_owner.rs、construction_set.rs：资源/材料一次发行、能力品牌、尾检→提交 token、保留费移交；若采用全 publication 同父域，R 同时改 validate_effects 与 finish_fixed_write 接线；本候选已选择强版。

S 拥有 staged_nodes.rs、content_store.rs、content_publication.rs、prepared.rs、commit.rs：staged NativeReadLease、带许可读写、三个选择性真实 SQL cut、actual root、scope/workspace 贯通。StageGuard row cut 属 R 文件，R 调用 S 的 `StageOwner::flush_cut`，S 不并行写 effects.rs。任何接口更改先向对方更新，不各猜一版。

独审必须至少拒绝以下变异：重复 resources/material；错 journal/transaction/seal/scope；注入前根绑定；SQL 未落库仅 overlay 证明；flush 持锁递归；permit 先于 body Drop；尾检错误却发生 COMMIT；第二连接/新 life 复用旧资格；行重复、孤儿、错根；预算放大；原成功改拒绝；旧核兜底。

六个原成功锁名称与既有 runner 从 recovery.md 原样使用，新增失败注入只补充；16/8 MiB 不改。先实现/独审封候选及新 source hash，再跑六个 exact；没有运行结果前不能填 PASS。S86/MIN12、#1491/#1481、完整历史/双跑、64 GiB 每库/148 GiB 外盘及 #1279 门全部原样。
