# #1419 M8D：第 3 片生产接线准备

名分：绑定 #1419 的工作草稿，可交后续实施，不是新合同或生产通过收据。取证基线为 `sandcastle/issue-1419@b99d8e3e71a49c146659f41afc273e44dc4a0020`。除明确标注旧提交者外，源码行号均以该 HEAD 为准，路径相对仓根。本文中的新类型、方法和锁名全部是拟实施项。

输入已读：本目录 M7D-DESIGN、M8A-SLICE1、M8B-SLICE2、M8C-SHARDING，M2-NOTES 的 M7C 至 M8C；非作者报告 `/Volumes/AgentStorage/m7d-design-review48x/REPORT.md`、`/Volumes/AgentStorage/m8a-review48z/REPORT.md`、`/Volumes/AgentStorage/m8b-review48ad/REPORT.md`，以及首拒原件 `/Volumes/AgentStorage/budget-diag46z2/REPORT.md`。四项条件的直接来源是 m8b-review48ad `:57-62`。本文按 writing-for-agents 的可执行交棒口径组织，按 unslop 校订文字。

本阶段只新增本文并追加 M2-NOTES。实施沿已批方案 ii，不启用淘汰或回收，不改 J/R、判据、错误文本、effects、证书/资格、角色收费、deadline、W2 规则串、认证、历史规模或期限。M8A/B 仍为测试后端，M8C 只改分片。依本轮主控提供的状态，`2c6cb4e3` 的 CI 仍在跑，M8A/B/C 尚无 CI 收据；本文不将其推断为成功。以下“通过”均为后续验收目标。

## 1. Access 生涯封印与描述符

### 1.1 现有组成与实际缺口

这里的 Access 指 S owner 的实际材料访问能力。源码并没有一个包办全部生涯检查的 `Access` 结构：`s_content::access::Access` 是消费者 Job 访问 trait，`rust/src/s_content/access.rs:14-42` 不持 FD、WAL 或写者生涯。生产材料能力是 `MaterialAccess`，它依赖 `AuditConnection/AuditTxn/history_clamp` 的组合。不得把已有字段清单写成 M8B 已经封闭了这些能力。

| 实际材料 | HEAD 源码证据 | 现有约束及接线要求 |
|---|---|---|
| 写者生涯、退役、事务/写序 | `rust/src/bin/s_session_v3/audit_owner.rs:308-328`、`:438-450`、`:379-389` | `life` 由 owner 私有计数签发；`retired`、事务 ID、write_sequence 属 owner，不从路径或 digest 推导。life 数字本身不可作为跨进程身份，同库新进程必须重新认证。 |
| 快照封印 | 同文件 `:3256-3276` | `SnapshotSeal={life,transaction_id,write_sequence,data_version,schema_version,domain:[i64;6]}`；跨事务继承有 `same_snapshot` 条件。不能把快照封印误称含 FD 的完整生涯封印。 |
| 同连接 data_version | 同文件 `:1142-1156`、`:2684-2698`；`audit_owner/history_clamp.rs:229-275` | 真读 `main.meta` Btree 建快照后取版本；仅在该 owner 同连接上比较。`check` 对照生涯基线；自身 COMMIT 后不刷新版本来吸收外部提交，见 clamp `:415-417`。 |
| 主库/WAL FD 与路径身份/stat | `audit_owner/history_clamp.rs:18-67`、`:94-105`、`:144-169` | `WatchedFile{path,File}`；`Stamp={dev,ino,len,mtime秒/纳秒,ctime秒/纳秒}`。每次 held FD metadata 与 named path metadata 全等，FD 活着不等于路径仍指同库。 |
| WAL 头及观测双夹持 | 同文件 `:70-84`、`:203-227` | `Files{main,wal,wal_header}`，头取至多前 32 B；读头前后再次比两文件 stat。不是只比较 WAL 路径字符串。 |
| 连接域、计数、控制原件 | 同文件 `:78-84`、`:229-255`、`:429-433`；`audit_owner.rs:3173-3205` | domain 核 TEMP 对象/附加库为空，并记 read_uncommitted、foreign_keys、recursive_triggers、trusted_schema、query_only、main.encoding；另核 schema、total_changes、meta/s_protocol_meta 实际行规范字节。固定查询条数不代表控制正文是零字节。 |
| 自身 effects 与物理提交收据 | `audit_owner/history_clamp.rs:277-302`、`:304-339`、`:347-417`；`audit_owner/history_physical.rs:11-41`、`:63-98`、`:100-115` | candidate 必须有同父且完整 coverage，控制差异由 effects 解释；自身 WAL append/reset/checkpoint 依实际帧/回填进度核，不能用读前重新取 baseline 掩盖外写。 |
| 材料 Access | `audit_owner/effects.rs:1179-1187`、`:1223-1250`、`:1663-1718`、`:1803-1813` | 持真实 tx/work/budget、journal、SnapshotSeal、initial_material_epoch。`check` 比实际 TLS journal 与保存 journal、事务、seed seal、epoch、unexpected/overflow；目前没有把 clamp/FD 借入 reader。 |
| 父身份与活动域 | `rust/src/s_content/capture_allocation/scratch.rs:55-61`、`:105-113`；`audit_owner/history_clamp.rs:174-186` | 原许可的父 Arc 身份与当前 WorkScope 健康分别核；旧 origin 可以不同。rebind 只能同父；释放必须回原 origin。 |
| M8B 描述符 | `rust/src/s_content/step_reader/managed_sql.rs:11-22`、`:26-77`、`:84`、`:174` | 只持 tx/work/sample/0 B identity、changes/schema。`check_domain(self.tx,self.work)` 内的 ptr/父比较自比较，不能证明与 S Access 同源；外部 WAL 生涯目前由调用方 sample 承担。 |

### 1.2 拟定数据形状和接口

生产描述符放在 **bin owner 私有模块** `rust/src/bin/s_session_v3/audit_owner/step_reader.rs`，由 `AuditTxn` 工厂签发，不能从 library 向上依赖 bin。library 保留 codec/租约机制；将 M8B SQL 复制/核帧机制抽为单一路径，由 owner 描述符实现 NodeSource。以下是接口草图，生命周期参数可按 Rust 借用检查细化，能力来源及费用义务不得变化。

```text
OwnerLifeGuard<'owner> {
    owner: &'owner AuditConnection,        // 私有、不可 Clone，不接调用者 life 数字
    clamp: 借用原 Clamp 的只读核能力,       // 留在原 owner，持原 FD/基线/物理进度
    parent_identity: 原 Budget 身份见证,
}
MaterialReadSeal<'step> {
    life: OwnerLifeGuard<'step>,
    active: &'step AuditTxn,
    snapshot: SnapshotSeal,
    journal: 原 Journal 身份借用,
    phase_epoch: u64,
    phase_observation: 原 Observation 的受管本切面见证,
    cap: JournalCap<'step>,
}
OwnerStepReader<'step> {
    access: &'step MaterialAccess,
    seal: MaterialReadSeal<'step>,
    work: &'step WorkScope,
    prior: QualifiedSuccessView<'step>,
    pending: StepSuccesses,                // 仅本步新成功，实际容量收费
    sqlite: SqliteDomainLease<'step>,
    failure: Option<原 typed error>,
}
AuditTxn::with_step_reader(material, 原请求门, consume) -> Result<T, PublishError>
OwnerStepReader::read_checked(&mut self, RefView, Mode) -> Result<ReadLease<'_>, CodecError>
MaterialReadSeal::check_live(&self, 原请求门) -> Result<(), 原错误类型>
```

`原请求门`为借原 Job/绝对截止时刻的具名访问对象，只负责原取消/期限，不接替生涯核，不传任意 `Fn()->Ok(())`。library 不能自行构造生涯 guard 或接受“已认证=true”。对 library 必须新增的桥只传已经封闭的 reader/成功查询实现，生产工厂仍只在 owner 中；测试注入留 test cfg。

现 MaterialAccess 只提供 `&Transaction`，见 effects `:1182`；不能硬将它转成 M8B 的 `&mut Transaction`。owner 工厂借住真实 AuditTxn 与材料切面，reader 不暴露 tx/Connection getter，所有写入口由材料 epoch 和原 owner 控制；reader/lease 未结束不能 Commit、换事务或推进 stage。`with_step_reader` 的闭包只限定借用寿命，不承担生涯检查。实现不得用 unsafe 别名绕过此边界。

检查时点及完成判据：

1. **签发前**沿原入事务门建真实快照、`check_history`、父域健康及 Access/journal/资源绑定。必须从 owner 的真实 seal 和 FD 观察建立见证；0 B reservation 只作父见证，不能作内容库见证。
2. **签发本切面**时锁定 Access seal、epoch 和当前合法自身写已发生后的 Observation。该见证不是 `WriteCandidate`，没有历史/来源资格。生产材料读取发生在自身 node 写之后，不能每读一次把当前 Observation 与写前旧 baseline 直接全等比较，也不能直接用当前观察刷新生涯 baseline。只能沿已有受控 journal/effects 的写序转换解释本连接变化；未解释的变化走原 stale。与 `candidate/check_candidate/committed` 同用原观察核和物理核，不另写宽松判据。
3. **每次原件 I/O 前、成功点查/暖命中前**，内建 `check_live` 核 owner 未 retired、真实 owner/txn/seal、Access journal/epoch、原父身份、活动域、同连接版本/FD/WAL/stat/域及本切面观测。父比较改为 `seal.parent_identity` 对当前 access.work；连接比较若保留则比真实 owner 持有连接与 Access tx，删除传 self.tx 给 self.tx 的恒真核。自身写只能在原 owner 允许的阶段边界结束旧 reader 后重新签发，不能在 reader 中更新 epoch。
4. **取行复制前、核 F 后、交付租约前与步出口**采样原 deadline/取消，执行原必要身份核。已锁的逻辑门顺序、manifest 先核帧后解码、控制 rebind 先于暖观察不变；不得凭本表调整错误优先级。构造绑定失败只拒本次；进入原 owner 核后失败仍 revoke。第一错误保存原对象/origin，失活后节点 SELECT 为零，无换源重读。
5. **Commit 路径**原 candidate/effects/物理提交核照跑，内容库 Commit → 台账 Commit → 资格/前沿交接；读描述符出口无权推进 clamp baseline。回收前仍保全部 backing。

构造和每次域核无节点全表扫描。新增 FD/stat/控制观察、metadata SQL 的实际频率必须进入 §3 度量，不能把固定条数解释为免费。若本切面自身 effects 转换不能复用原核且保持原门序，3A 停在测试证据，按 §7 交主控。

### 1.3 真实负控

| 场景 | 建立方式与应锁行为 |
|---|---|
| 外连接真实 WAL 提交 | 真实文件库、独立 Connection COMMIT，分别插在 descriptor 签发前、首个节点读前、旧成功命中前；原 stale/拒绝，后续 SELECT 不增长，不用回调伪造版本。 |
| 主库换库/同路径改名替换 | 另一真实 SQLite 文件替换命名路径；原 FD 继续活着，held/named dev/ino/stat 不等即拒。另核相同 SQL 内容也不能接纳新文件身份。 |
| WAL 替换/截短/不完整头 | 保持主库，换实际 WAL inode，及原有截短/头故障；原 FD/stat/头核拒，不用 mock tx 指针代替。 |
| 同库同内容新连接/新进程 | 新连接不能接旧描述符；新进程只序列化公开库和旧台账，不序列化 guard，从 g1 冷认证。旧资格文件或重合 life 数字均不能绕冷折叠。正常新进程冷认证后续写是正控。 |
| 自身写与 checkpoint 正控 | 自身 journal 覆盖 node 写、合法 COMMIT/原阈值 checkpoint/WAL reset 继续通过；descriptor 只在原许可阶段重签，不将这些全部误报 foreign。 |
| 错父/错请求/期限/取消 | 用真实 Budget 与原 WorkScope，旧 origin 同父可读，健康异父在描述符入核前拒且不撤已有 owner 成功；入核后的原撤销照旧。原绝对期限到期不重置，首错与 typed bridge 保真。 |

## 2. j.cap 入口、接口与费用流

首拒已经实测，不再列为候选。独立诊断 run [41434824853](https://github.com/xy7365527-lang/NewChanlun/actions/runs/41434824853) 中，C/D g23、E/F g26 在 effects codec 分支 `rust/src/bin/s_session_v3/audit_owner/effects.rs:1477-1485` 追加时，累计表从 114688 到 229376 申请 `103222800 B`；父 reserve 成功后，分别被 `J=52612123/52145259 B` 拒绝，Allocation，未执行 allocator try_reserve，未到 validate_effects/后续保留门。旧报告的 `s_content.rs:3564-3567` 对应 HEAD `:3603-3607`，不能照抄旧行号当当前行号。

现入口仍是 `append_original_codec_for_owner(.., cap, work)`，HEAD `s_content.rs:3546-3567`；同摘要异正文实际比较在 `:3579-3591`，局部 growth 门在 `:3597-3612`。尾部 `audit_owner.rs:3911-3913` 的 `usize::MAX` 是另一既有入口，不覆盖来源准备 J 门。M8B `content_store/m8b_production_diff.rs:264-270/:433-439` 用真实原语重组 effects 序列且传 MAX，不能据此签 J 不变，也不是调用 `effects::audit_native_source` 的完整差分。

拟实施接口把入口费用能力显式带入，不能让默认 cap 产生 MAX：

```text
JournalCap<'step> { value: 原 j.cap, seal: 原 journal/phase 身份 } // 只有 owner 工厂构造
OwnerStepReader::read_scoped(cap: &JournalCap, digest) -> ScopedFrame
OwnerOriginals::append_from_effects(
    cap: &JournalCap, work: &WorkScope, digest, tag, body: CodecVec<u8>
) -> Result<bool, CodecError>
OwnerOriginals::verify_and_lease(cap, 完整 RefView, Mode, QualifiedSuccessView) -> ReadLease
```

来源准备先从同事务真实 j.nodes 读回，Native 正文原许可随 body 移交；追加前核实际 frame，再按原规则比较同摘要的实际 tag/body；替换索引布局所需**一次逻辑增长操作的完整申请**仍以 j.cap 比较，父 reserve 与局部失败的 typed 顺序沿原核。一个增长批次不得拆成多笔小申请绕 J。J 仍是原局部增长门，不改成“全请求所有历史 SQL 字节累计限额”；正文读取的原 WorkScope/父/资源门同样存在。没有增长也必须经过能力检查，不能因 growth=0 就绕同父/健康核。

费用流固定为原 R → 原 O/U/A 冻结 → 原 candidate 父 Budget/WorkScope → `P=min(A,parent.remaining())` → `J=floor(P/8)` → journal 与本次 append/read。descriptor/台账查询 scratch、Rust SQL 副本、真实证明副本及新索引均在这一个父账内。现有正文 backing 移交不再物理收费一次；真实 SQL 副本是另一 allocation，单独收费。SourceClaim 的 `len+97` 仍是角色额，不能抵掉物理 reserve。§4 的连接域许可也必须在该父账和旧 O/U 划分中对账，不另造额度。

### 2.1 差分锁与原失败场景

| 臂/时点 | 对照及预期 | 必要证据 |
|---|---|---|
| 有限 j.cap，小成功/重复插入 | 实际 effects 来源入口 vs 新入口，使用真实 Recording 签发 cap；逐位正文、拒绝类别、角色和业务 events 相同，F/SQL/物理事件分列。 | `h1_m8d_jcap_effects_append_read_diff`；必须调用真实 `effects::audit_native_source`，不能只复制序列。 |
| 精确局部增长与少 1B | 真实同父有余量；在确定的 append 操作增长量 G 上，以 G/G−1 两 cap 对照。不得通过制造父不足掩盖局部门；正文、proof 已存活量另计。 | `h1_m8d_jcap_exact_growth_and_one_less_keep_origin`；锁先 reserve 成功再局部 Allocation、首错、桥与无候选。 |
| MAX 对照 | 仅保原尾部对照说明局部 cap 不同，不是生产替代臂；按同父核其真实行为。 | 与有限 cap 臂并列，检查生产来源调用图没有流向此入口。 |
| C/D g23、E/F g26 原失败 | 3A/B/C/D 均未回收，若累计表保持，原失败仍可能存在；用固定原夹具在真实 effects J 门复现/记录，保持原四锁成功断言，不能改成“失败就是通过”。 | P3 的 `h1_m8d_jcap_cd_g23_ef_g26_first_refusal` 是嵌入原两锁的诊断断言 helper，不另注册完整重放测试；原 C/D、E/F 完整冷锁仍记录红/绿。不得预置新 J 等于旧 J。 |
| 完成 ii 的边界回收/冷恢复之后 | 累计整块翻倍原因消失，预期 g23/g26 及 C/D43、E/F107 全程通过；须经过原 j.cap，而非扩帽、转 MAX 或调扩容时机。 | M7D 第 4/5 片，M8C native-1/-2 的原完整锁 + owned release `h1_` 逐名 ok，真实新冻结账和首拒/无拒读数，非作者独审。 |

“消除整块翻倍后预期通过”是方案 ii 完成后的待测行为变化，不是本准备片、无回收的接线片已经修复。若仍失败，先按同一请求记录 `g/phase/入口、G/J、O/U/A/P、scope live/peak/limit、父 remaining/live/L0+A、origin/trace、驻留索引 len/capacity、SQL/证明/资格共存量`。区分新前沿/Δ 自身撞 J、父 reserve、allocator、validate_effects、各保留分项及 deadline，再修持有/共存/重复复制；帽、代数和错误文本保持。没有执行的后门标“未到达”。诊断收据不能覆盖原锁红灯。

## 3. 成功继承、接口与 SQL/F/metadata 读数门

### 3.1 已有能力与接入位置

M8B `managed_sql.rs:91-98` 每次新建空 AuditMemo，SQL 核完整闭包，成功 memo 随 verifier 析构；这不能直接进入生产替代 owner 的 Δ 成功继承。

现有可用组件分别是：

- `rust/src/bin/s_session_v3/audit_ledger.rs:272-306` 的 Qualification，含 through、roots、fold_digest、verifier_revision、rule_revision、content_identity、binary_commit，有读前字符串保留上界。`:634-658` 的 bounded qualification 和 `contains_qualified_unit` 真核已提交资格；`:699-704` 按完整 digest/flavor/mode/first_generation 点查。不是不收费的 bool 集。
- `rust/src/bin/s_session_v3/prepared.rs:398-416` 已有 Runtime 的 bounded qualification/qualified unit 委托。`:715` 消费 prior qualification。owner 不再建立全历史内存复制表。
- `rust/src/s_content.rs:5591-5625` 的 verifier 已有完整单位止步与 owner 成功树复用位置，`:6235-6277` 先核完整根 header 再 run。四字段、Mode 及 JsonInner 派生仍由原核负责。
- `rust/src/s_content/closure_delta.rs:37-45` 的 Frontier 只管完整键点查，不证明 U-A；`:119-149` 将完整 VerifiedUnits 与诊断 Probe 分开。文件 `:1-2` 明写探针不进准入。不能直接把 `probe_four_roots` 改名后接生产；`audit_ledger.rs:722-774` 的 `stage_verified` 当前只接严格核签发的 VerifiedUnits，Probe 不能冒充它。

拟定私有数据与方法：

```text
QualifiedSuccessView<'step> {
    material: &'step MaterialReadSeal,
    prior: &'step 已持票 Qualification,       // 既有完整已提交水位
    ledger: 借 Runtime 的只读完整单位点查口,
    position: 原 CommittedSet/PendingSet 变化证据借用,
    revision: 原 codec/四根判据版本,
}
StepSuccesses { Public/Template 分开的受管本步新单位与必要聚合 }
QualifiedSuccessView::contains(Unit {digest, Flavor, Mode}) -> Result<bool, 原错误>
OwnerVerifier::verify_delta(完整四根, QualifiedSuccessView) -> VerifiedDeltaUnits
Ledger::stage_verified_delta(token, delta, VerifiedDeltaUnits, work, prior)
```

`VerifiedDeltaUnits` 为拟新增的核签发类型，字段私有，带完整四根、prior 水位/fold/版本绑定和本步新增完整单位；只有同一个 verifier 全部成功后产生。新 stage 入口验证这些绑定，原严格 VerifiedUnits 入口保留原有合法使用域；两者进入同一个台账事务写核，没有失败后改走另一入口的分派。冷折叠仍从空 g1 调同一个 Δ 步，不能另接每代全闭包路径。台账字段若需扩展，仅在既有旁挂格式升版本后丢弃重建，不改内容库 schema 或公开根。

现 Verifier 位于 library 私有模块，bin 不可直接构造它。实施须新增 `s_incremental` 下的具名机制桥，返回四根/单位/版本的核验收据，沿用现 verifier 的 header、push_work、单位检查；机制桥本身不签 U-A、来源或历史资格。owner-private 的 `verify_delta` 只从自身 Runtime/Qualification 构造 QualifiedSuccessView，再把机制收据与同一个 MaterialReadSeal/prior 绑定成 VerifiedDeltaUnits；stage 消费时再次核绑定。不能导出“传任意 Frontier/成功闭包即可签 owner 资格”的接口，不能让调用方把诊断 Probe 或 self-reported bool 包装成生产成功。

查找顺序固定：原门与描述符有效 → 根的真实 header/四字段 → 本步已完整成功 → 当前生涯 prior 合格的完整单位点查 → miss 按原 verifier 核该单位和必要序列 → 全根/该步成功才晋升/准备台账。lookup Err 立即失败并保首错，不当成 miss，不调用完整审计重判。首达的旧落库不可达节点仍是新单位；Public/Template、Value/JsonInner 不互借；序列 digest 不进认证单位表，UTF-8/数字进态、字段顺序和 extent 聚合仍核。

#1481:24 的位置继承与 AMENDMENT 的单位认证分两层，避免新造口径：对象/历史结论用原身份坐标、根角色、字段/路径的 before/after Retain 证据证明“同位置同摘要”；异位置的领域约束在新位置重核，不能拿数组下标或同 digest 证明对象身份不变。codec 子树止步按 AMENDMENT:25/:29/:60 的完整 `(digest,Flavor,Mode)`；同单位在另一个容器出现不自动撤销已批 codec 资格，但不能连带继承该容器的来源/历史结论。位置变化带来 Flavor/Mode 改变时一定 miss。不得给 node 表强加未经批准的裸位置策略，也不得以 node 命中替代位置判据。

新步只持本步 memo/聚合与活跃成功描述符，不复制全史 node 集。byte/elem/field/key_cmp 不是任意序列的通用证书：原上下文键、聚合值和版本必须保留；不能移走 seen 后宣称其余 AVL/HashSet 已有界。完整闭包失败不发布任何部分成功，owner 原 revoke 仍清两模式能力，台账 pending/bound/stale 不作为当前 prior 命中来源；本步已成功但尚未整代提交的材料只在原收据允许的本步域内复用。冷启动/recover 丢旧准入权，从空 g1 同一 Δ 步重建，旧台账只诊断。

### 3.2 测量方法及阈值原文

本轮只读取得 #1481 正文/评论，命令为 `env -u GH_TOKEN -u GITHUB_TOKEN gh issue view 1481 --repo xy7365527-lang/NewChanlun --json body,comments`，退出 0。临时原件 `/tmp/issue1419-m8d-1481.json` 不入仓。按正文 splitlines 编号：

| 原文来源 | 逐字摘引/公式 | 实施读数义务 |
|---|---|---|
| [#1481:24](https://github.com/xy7365527-lang/NewChanlun/issues/1481) | “未变子树按「同位置同摘要」继承上一代结论，异位置同摘要按新上下文重核；不常驻全部原件” | 新单位/变化对象与继承命中分桶，禁止用整闭包重核代替继承。 |
| 同票 :27 比例门 | “系数在三个大 g 点相对变化 ≤20%，残差对 g 斜率 95% CI 含 0，允许 SQLite 规模项 ≤ G^0.2。” | 普通非 test 二进制，原 g∈{86,1000,4000,9011} 或原 M2 选点；审计耗时与读量对 Δ_R 拟合，保留样本/拟合方式/区间，不能用 STRING 微锁代替。 |
| 同票 :27 上界门 | `Δ_R(g) ≤ Σ_f(u+e+⌈log₂count_f(g)⌉+1) + C₀_base + C_bi·bi_births(g) + C₁·⌈log₂N(g)⌉`；“五常数沿用 #1491 在 S86 前缀上实测冻结的值并写进运行 manifest，超界代数 0。” | u/e/C₀_base/C_bi/C₁ 的原实测冻结值和 manifest 来源必须随收据；不重新拟合常数吸收新增查询。原文数值缺件就标待前提，不填写推导值。 |
| 同票 :27 期限 | “30 s 调用门不放宽。前置：#1488（段 A）与 #1491（段 B）落地。” | 保留原 deadline 与采样。审计门之外的原整体性能门也保留，见 M7D §5 与 ADR0042。 |
| AMENDMENT:25/:29/:60 | 单位为“(节点摘要, 上下文 Flavor{Value, JsonInner}, 核验模式 Mode{Public, Template})”；“序列节点……不是认证单位” | 冷/暖/首达/上下文变化分别量 F、真实读回及台账查询，不能只报单位命中率。 |

每个测量代/请求取同一窗口的前后差，保原 `Δ_R`、F、节点 reads 定义；旁列如下计数，别把新增 PRAGMA 或台账 SQL 藏进旧节点 SELECT 名称：

| 计数 | 来源与采集办法 | 阈值及解释 |
|---|---|---|
| `node_selects`、实际 F/核帧字节、逻辑 lookup、first_reachable/new_units | 既有 SQL_READS/codec 观察与测试限定 NodeSource 计数；重型普通二进制用原具名读数入口 | 沿原 Δ_R/F 定义进入 :27 两门；本步重读不能报为新认证单位。 |
| `metadata_sql` | 测试只读调用计数分别记构造、每次 guard、根头、取行、qualification 校验/水位/cert。SELECT/PRAGMA 均计，syscall stat/FD 另列 | 不新增“每次≤21”政策。M8B STRING 5 节点 SQL 对应 `3×(5+2)=21`、BOOL 3 对应 15 次 metadata，仅源码推导；新 Access 核、旧 TxStore::new 格式查询及台账资格查询都会改变此数。CI 必须逐臂实测后冻结微锁精确值。 |
| `ledger_sql`、已提交命中/本步命中/miss | 点查完整单位、qualification 校验和 stage 写入分桶；失败查询也记尝试 | `contains_qualified_unit` 内会重核 qualification/through，不是单次 EXISTS 的成本。原 :27 没有单列 metadata 的数值帽；全部 SQL 的真实耗时/读量要在原比例门评估，按原定义进入 Δ_R 的项仍受原上界，新增项另列对账，不能改定义免验或自定一个新阈值。 |
| deadline/取消、FD/stat/控制原件读量 | 保原采样观察与测试受管门计数，窗口绑定同一绝对期限 | 不重起计时，不减少采样凑性能。固定 metadata 次数也可能处理非固定字节控制行，需同时记字节。 |

差分收据须给 raw 样本、构建 SHA/特性/profile、输入摘要、资源档、g/各根/台账水位、五常数及来源、count/bi_births/N、Δ_R/上界/超界代数、各类 SQL/F/耗时。普通 binary 的 :27、S86 全前缀/9011、恢复窗口全部是原具名 CI/重型前提，本准备片未运行。M4 的 13377 和 ordinary memo 排除门保持独立，不能以接台账为由开启全局 memo。

## 4. SQLite 内部字节⑥与同父共存峰

### 4.1 字节来源

版本取自 `rust/Cargo.lock:838-839/:1638-1639`，libsqlite3-sys=0.38.2、rusqlite=0.40.2。SQLite 文档使用这一本 bundled sqlite3.h；本机可读路径为 `/Users/silencehan/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libsqlite3-sys-0.38.2/sqlite3/`，后续 CI 按锁版本在其 registry 取证，不依赖此绝对路径。

| 来源 | 文件:行/文档 | 与 Rust 许可的关系 |
|---|---|---|
| prepare 的 VM、schema、绑定等 | `rust/src/sqlite_checked.rs:44-64` 调 sqlite3_prepare_v2；rusqlite `src/raw_statement.rs:152/:175` 调 step | 已进入 SQLite C allocator，发生在 Rust body Vec reserve 以前。现读路径主要走项目 CheckedStatement，不能只看 rusqlite row.get<Vec>。 |
| pager 页缓存/溢出页读取、VDBE 行材料 | `sqlite3.c:57119` sqlite3PcacheFetch，`:86951` sqlite3VdbeMemFromBtree；项目 `sqlite_checked.rs:179` 调 step | SELECT 已经取页、组行。页可跨 statement/请求存活，finalize 不等于清 pager。 |
| SQLite BLOB 展开/格式转换 | `sqlite3.c:94938` sqlite3_column_blob、`:85903` sqlite3VdbeMemExpandBlob；`sqlite3.h:5510-5574` | 项目 `sqlite_checked.rs:319-332` 先 require Blob 再 column_blob/column_bytes，避免随意 Text 转换；真实 BLOB 仍可能展开 zero-blob/跨页材料。文档允许转换分配失败并使旧指针失效，不能以 `&[u8]` 判零费用。 |
| Rust 行副本与 proof | `managed_sql.rs:186-209`，`rust/src/s_content/job.rs:380-390/:523-543` | Rust Vec 复制前 reserve；CheckedFrame 按值接 Vec 不再复制正文。SQLite 行仍活时 Vec 可与其共存。以后如额外保 proof 正文，按实际新 backing 再计。 |
| finalize/drop 与缓存存活 | `sqlite_checked.rs:230-236`；`sqlite3.h:5552-5557`；`audit_owner.rs:4511-4520` | column 指针只活到 step/reset/finalize；Rust 拷贝可继续活。原 cache_flush 在 effects finish 前保持，不能因新 lease drop 提前释放页缓存费用。 |

### 4.2 选择连接域整体归属，保留 Rust 逐次许可

拟选组合：**SQLite 内部由连接域整体预留，Rust 正文/独立 proof/临时容器继续逐次原许可**。不选“查出 row.len 后才预留 SQLite 内部”方案，因为 prepare/step/取页已经发生。主库连接及旁挂台账连接分别有归属域，但都在原 owner 同父总账内；不能漏掉新台账查询的 SQLite 内存。

```text
SqliteDomainLease<'life> {
    owner_connection: 真实连接域身份,
    parent: 原父账身份,
    permit: 原父账对 B_sql 的许可,
    bound: SqliteBoundProof,       // 来源/布局/允许语句/最大并发/上限覆盖证明，非观测 bool
}
SqliteDomainLease::before_sql(原域/请求门) -> Result<(), 原 typed error>
```

`B_sql` 是需证明的覆盖额，不是新增资源档字段或新帽。必须指出已有物理费用中哪些已经计在 O/U 或原许可，建一份 allocation 归属映射去重；同一 SQLite allocation 不能既算 U 又完整 reserve 两次。连接已有存活费用须在请求冻结前归入旧池/未覆盖费用；新增长的覆盖许可必须在可能分配的 prepare/step/blob **之前**取得，并计一次完整操作需求，保原 J 的适用范围。无法证明的内部字节不能用负数差额抵账。

连接域许可须覆盖该域全部 SQLite 存活及瞬时申请，允许的并发 statement、pager、schema、lookaside、WAL 关联材料、行展开/临时分配均需列出来源；不是只加 CACHE_USED+STMT_USED。连接域随原 owner/父身份活到真实缓存/连接资源析构；仅 finalize statement 只能释放可证 statement 部分。旧 origin 释放只减 total live，不返当前 remaining，冷/新请求冻结沿原式。不能为 pool 另签父账或提高 R。

当前证据**还不能给出 B_sql 的可执行硬上界**。cache_size 是建议值，见 `rust/src/bin/s_session_v3/sqlite_pager_probe.rs:1-3`；测得峰×经验系数也不是覆盖证明。3B 须先交同父覆盖证明与 CI 读数；若只能用新 allocator 拦截/全局 hard limit/新资源政策才能成立，转 §7，3C 不接生产。这一缺证不以本文 status=ready 消除。

### 4.3 共存峰与只读实测

在同一时间 t 记 `S_main(t),S_ledger(t)` 为实际 SQLite allocation 去重总量，`R_body(t)` 为活 Rust 正文 backing，`R_proof(t)` 为独立 proof backing，`R_other(t)` 为 descriptor/memo/前沿/索引/渲染等，其余 owner/audited/Q/ExactRows/journal/资格为 `H(t)`：

```text
M_actual(t) = H(t) + R_body(t) + R_proof(t) + R_other(t)
              + S_main(t) + S_ledger(t)
L_accounted(t) = H_accounted(t) + R_body_permits(t) + R_proof_permits(t)
                 + R_other_permits(t) + B_sql_main(t) + B_sql_ledger(t)
要求：SQLite 可归属 allocation ≤ 对应 B_sql，其他 allocation ≤ 各自原许可；
      max_t L_accounted(t) ≤ 原 total_limit，且每次新增申请 ≤ 当时 remaining；
      局部增长仍核原 j.cap。
```

角色 SourceClaim/资格语义收费单列原合同，不将其简单相加当真实堆；账目按原 CompleteCharge 规则对账。实测峰取上述同一 t 总和最大值，不能加不同时间的历史高水位。pool 覆盖额已计物理许可，db_status 实测不能再加一次到父 live。RSS 只观察系统效果，不用 RSS 减法签同父许可正确。

只读 CI/test-only 测点为：连接已有静止点 → 原 metadata/qualification 前后 → prepare 后 → step 得行 → blob 后 → Rust Vec+SQLite 行共存 → CheckedFrame → finalize → lease drop → effects finish/cache_flush → 原请求边界/连接关闭。使用现有 pager snapshot `sqlite_pager_probe.rs:11-29/:147-168`，必要的细测只在 test-only 用现有 sqlite3_db_status、sqlite3_memory_used/memory_highwater API；不增加生产日志字段、环境开关、分配 hook 或测量拒门，不改 cache_size/SQLite 配置。

- `CACHE_USED/SCHEMA_USED/STMT_USED` 均为 approximate，sqlite3.h `:9104-9136` 明写其 highwater 恒 0。须对 current 自己逐测点取 max，0 不能当峰。STMT 含部分 lookaside，与其他状态求和先核重叠；shared cache 需报告 CACHE_USED_SHARED，避免跨连接重复归属。
- `sqlite3_memory_used/highwater` 是进程 SQLite allocator 汇总，sqlite3.h `:3332-3344`；包含 SQLite 内开销但不含系统 malloc 开销。独立串行测试进程、记录所有连接和基线，用 `highwater(0)` 不重置共享状态。逐点差值/进程高水位只能给实测下界/总量，不能凭其证明每连接硬上界，不能减去其他活动连接未知量。
- 需覆盖冷/暖、首次可达、大 BLOB/zero-blob、缺失/损坏拒绝前、两种上下文、metadata/台账点查、取消/少 1B、合法 checkpoint、C/D g23/E/F g26 和实际调用嵌套。测点间短暂峰可能漏掉，标明采样下界；额外证明由源代码上界给出。原 CACHE_SPILL 计数不作压力 spill 或容量证明。

上述全部字节、pool 对账和生产共存读数待 CI；M8B 40/39 B 行复制锁只证明 Rust Vec 许可，不覆盖本节。B_sql 无证明或实测穿界就不能首次接生产，不把穿界值调成新帽。

## 5. 验收矩阵与执行路径

### 5.1 四项条件逐格对照

矩阵中的锁均为 §5.2 的拟新增项；本阶段没有新增或执行 Rust 锁。

| 条件 | 源码证据 | test-only 锁 | 生产差分锁 | CI 实测读数 | 原验收保留项 |
|---|---|---|---|---|---|
| 生涯封印 | §1.1 的 owner/clamp/MaterialAccess，特别是非自比较的身份来源 | L1/L2/L3/L4/L5 | P1 经真实 MaterialAccess/owner 核，外连接/换库/WAL/新进程与自身写正控 | 每拒绝点 SELECT 零增长、原 typed 首错/桥、stat/metadata 量、原父 live/release | foreign stale、main/WAL/stat、own commit/checkpoint、admission/recover、certified/qualified |
| 局部 J | effects `:1477-1485` → append `s_content.rs:3546/:3603`；原 cap 冻结公式 | L6 精确/少1B，保原 MAX 对照限定 | P2/P3 调真实 effects，含有限 j.cap；原四锁不替换 | 原/新 J/G/父账全字段、g23/g26 首拒/通过阶段，未到门标未到 | M3 四锁、原 J/R、同父、SourceClaim、预算锁、effects事件 |
| 成功继承及读数门 | ledger资格/完整单位点查、verifier止步、根头；Probe非准入边界 | L7/L8/L9 | P4/P5 冷/暖/位置变化、台账失效/失败撤销对完整 oracle | 分桶 SQL/F/metadata/命中/耗时；:27 五常数与两门；21/15 只作推导对照 | admission六代、full canonical、全史冷/recover/台账丢弃、ordinary full_some、M4排除门 |
| SQLite⑥ | CheckedStatement/SQLite C 机制、真实 reservation 与析构顺序 | L10/L11 | P6 用真实 native Budget 和嵌套 owner，Rust+SQLite+proof 同时在场 | db_status current、allocator总量、采样限制、B_sql来源/覆盖额、峰/remaining、旧origin不退款 | 少1B/首错、父总门、后续保留门、flush门序、冷/RSS原读数 |

### 5.2 拟新增锁的名称、文件和可执行断言

L 组中 owner 边界放拟新增 `rust/src/bin/s_session_v3/audit_owner/step_reader_tests.rs`，作为 `construction_tests` 子树挂载，完整前缀拟为 `v3::audit_owner::construction_tests::step_reader_tests::`。该处全部 L 锁限定 `#[cfg(all(test, feature="s_capture_native_testonly"))]`，避免 library 伪造 bin 生涯。纯 SQL/租约机制锁仍放已有 `rust/src/s_content/step_reader/managed_sql/tests.rs`，前缀 `s_content::step_reader::managed_sql::tests::`。下表指定文件后据前缀组成完整测试名。

| ID | 拟新增短锁名 | 文件/运行组 | 断言 |
|---|---|---|---|
| L1 | `m8d_descriptor_real_foreign_commit_stops_before_hit` | owner step_reader_tests / native delta | 真实外连接提交，miss/暖命中都先过生涯门，首错后零 SELECT。 |
| L2 | `m8d_descriptor_main_wal_replace_and_stat_refuse` | 同上 | 两真实库、真实 WAL 替换/截短/头、无 SQL 的 main stat 变化均由原门拒。 |
| L3 | `m8d_descriptor_new_process_cannot_import_old_authority` | 同上 | 真子进程冷启动重建，旧 guard无导入口，公开结果对拍；不将相等 life 数字当资格。 |
| L4 | `m8d_descriptor_wrong_parent_keeps_prior_success` | 同上 | 真实父账错域入核前拒，原父能力仍可用；入核后错误撤销原锁保留。 |
| L5 | `m8d_descriptor_own_write_checkpoint_and_epoch_boundaries` | 同上 | 合法自身写不误 stale；读期间不能推进 stage，结束后依原写序转换。 |
| L6 | `m8d_jcap_exact_growth_one_less_and_max_control` | 同上 | 同父余量足够，局部 G/G−1 精确门与首错；MAX 仅原尾部对照。 |
| L7 | `m8d_success_complete_unit_context_position_and_first_reachable` | 同上 | 同位置继承/异位置领域重核，Flavor/Mode 分离，首达旧节点重核，序列不能签单位。 |
| L8 | `m8d_success_partial_failure_and_ledger_phase_never_promote` | 同上 | 失败不留下半成功；pending/bound/stale/旧水位/版本/根/身份资格均拒。 |
| L9 | `m8d_metadata_node_f_and_qualified_lookup_receipts` | 同上 | STRING/BOOL 冷暖、qualification查询逐桶实测，核旧21/15推导与实际差值原因，原计数定义不变。 |
| L10 | `m8d_sqlite_internal_and_rust_copy_coexist_receipt` | managed_sql/tests / native delta lib | test-only只读取现成SQLite状态；行/Vec/证明共存实测，缺读数不签0，实测≤已证明覆盖额。 |
| L11 | `m8d_sqlite_domain_permit_lives_until_resource_destroyed` | owner step_reader_tests / native delta | 真实预算/连接域，finalize后cache仍有归属；正文先析构，旧origin不返当前remaining；准确少1B门。 |

P 组放拟新增 `rust/src/bin/s_session_v3/audit_owner/step_reader_production_tests.rs`，作为 `construction_tests` 子树挂载，完整前缀拟为 `v3::audit_owner::construction_tests::step_reader_production_tests::`。除 P3 helper 外，用 `#[cfg(all(test,feature="s_incremental"))]` 注册测试，因此同时在生产增量测试与 native delta 可执行。名称含 `h1_`，自然被现有 owned release `h1_` 过滤命中：

| ID | 拟新增短锁名 | 生产差分对象 |
|---|---|---|
| P1 | `h1_m8d_access_descriptor_real_life_diff` | 真实 owner/MaterialAccess 生涯与新描述符，同正文/错误，保原截止/取消/首错。 |
| P2 | `h1_m8d_jcap_effects_append_read_diff` | 真实 `effects::audit_native_source` 有限cap，非拼装序列；同digest异实际正文/缺失/损坏/角色与来源资格全比。 |
| P3 | `h1_m8d_jcap_cd_g23_ef_g26_first_refusal` | 不注册独立重放测试，作为原 C/D/E/F 完整锁调用的诊断断言 helper；在原43/107代输入、原资源档上保 g23/g26 收据，完成ii后登记为无该拒绝并核后门。 |
| P4 | `h1_m8d_success_delta_and_full_canonical_diff` | 完整 oracle仅测试，冷/暖/变化对象/异位置，同canonical与拒绝类别；新增F/SQL单列。 |
| P5 | `h1_m8d_success_ledger_commit_failure_and_cold_rebuild_diff` | 内容Commit/台账Commit/交接、失败stale、丢台账/新进程折叠，资格语义不变。 |
| P6 | `h1_m8d_sqlite_parent_peak_and_first_error_receipts` | 实际 Budget/WorkScope，各源同时在场，精确覆盖/少1B、取消和首错不被后续读抹掉。 |

P3 随原 C/D/E/F 完整锁在 native-1/-2 及 owned release 执行，避免在普通片重复增加两条重放；其诊断检查不能消费、改写原错误或改变原成功断言。每一子片先据实际迁移面登记应到阶段；完成 ii 后只能期待完整通过，不加运行时双轨或 env 选择生产实现。P 组先以真实生产入口对测试候选 adapter 对拍；只有 §6 的 3C 才让 adapter 获得非测试调用者。

### 5.3 CI 运行步骤和待 CI 前提

以下是下游 CI 命令，本机禁止执行。先跑现有检查，再执行锁，不以编译代替执行：

1. 原 `RUSTFLAGS=-D warnings cargo check --all-targets --features s_session,s_incremental` 和对应 `s_session,s_capture_native_testonly`，见 `.github/workflows/ci.yml:220-235`。后者必须包含非 test 编译；若 M8B import 告警成立，只做原 owner cfg 小修，不改判定。原禁串和无测试钩子检查同跑。
2. 在 `rust/` 中以现有 `../.github/scripts/test-feature-delta.sh s_capture_native_testonly <index> 8` 跑 index=0..7。每片必须有 `verified=true`、完整真实清单和逐名结果；L/P 新锁应出现在 base=`s_session` 与 feature 集差中。测试特性/模块cfg决定自然覆盖，不能只看名字推断已跑。
3. 新 L/P 注册普通项自然进 M8C 4..7；P3 helper随原锁，不独占新清单项。四个原完整 W2 锁固定 0 bounded2、1 C/D、2 E/F、3 W。具体普通片取实际清单，不手冻索引；无 removed、并集覆盖、重复为空、60分钟原门。原单参数入口继续可用。
4. P 组同时由现 owned 的 release、`s_session,s_incremental`、`h1_ --test-threads 1 --show-output` 命中，现配置见 ci.yml `:500-515`。靶向复查用 `cargo test --locked --features s_session,s_capture_native_testonly --lib --jobs 1 -- --exact <L的库完整名>` 或 `--bin s_structure_session` 加 owner完整名；P组注册项另用 `--release --features s_session,s_incremental --bin s_structure_session -- --exact <P完整名> --test-threads 1 --show-output`，P3使用原C/D/E/F完整名。新增文件必须挂入实际 construction_tests 模块，不能孤立只编译。
5. ordinary full_some、预算等不在新增差集的同名锁，仍依原 content/owned/bin shards 等完整覆盖；native差集不替代 base测试。:27的普通非test二进制读数、S86/9011及真杀恢复在原具名窗口交付，微锁不替代重型规模。

待 CI 前提清单：上游 M8A 的7锁、M8B的16锁逐名执行和双形态 warnings；M8C真实分区/实际耗时；新 L/P 实际编译/执行；B_sql覆盖和真实共存峰；新节点/metadata/台账 SQL/F及 :27全部读数；原四锁/六代/资格/effects/预算/首错/恢复实跑。当前各项均无本阶段通过收据。若前片CI不通过，先修前片并独审再交3C，不用静态PASS代替。

### 5.4 原验收保留方式

原清单详见 M7D-DESIGN §5；此处列实施必须确认的保留动作，源锁不删、不减代、不改语义断言：

| 保留项 | 运行/对拍要求 |
|---|---|
| M3四锁 | 原W2完整生产串、cursor/seed/center/pair/revision/window/raw/canonical、全部冷点、每cut真H1资格、full audit准入零增长；C/D43、E/F107与其他原代数全留。M8C四固定片及owned release分别取收据。 |
| admission六代 | `audit_owner/admission_tests.rs` 的g1..g5、冷3/冷5、真实recover、epoch2 pending g6、最终冷6，完整canonical/台账逐位；负例类别、own commit/foreign stale/never_full_audit、写前失败释放全保。 |
| certified/qualified | `audit_owner/history_clamp_tests.rs` certified六代六时钟、qualified三代及资格1/2/3；规则/内容库/二进制/roots/units、旧资格释放/失败stale，不以reader迁移重签或延寿。 |
| effects/codec | 同文件原effects锁与 `s_content.rs` 成功/部分失败/模式/角色/版本/头部/同摘要异正文锁；before-SQL/hooks、有序seal/stage、receipt一次性、未来stage不能补缺失、取消和revoke不变。 |
| 少1B/首错/预算 | `history.rs` publication_native_cut_scope_real_budget_preserves_origin、`history_clamp_tests.rs` 控制/资格预算及原精确峰值门等继续执行。只对实际布局影响部分按实测重冻，少1B门、同父、原origin与bridge、无写/无slot、释放和原门序不变。 |
| ordinary full_some | `source_audit.rs` 的 `full_some_audit_keeps_frozen_read_and_budget_trace` 保 counts=(91,13,11,14,0,0)、reads=227347、F=248860，read_hash=`13bd2c89eae9fa7590d79dcf172f3ee2419ced7b826191ecc73a888f6c3052c2`、budget_hash=`20e6e506fdbbe095e5838c68df99add568377e9881a26460ae97540da00da37d`。此ordinary路径不在迁移面，不允许重冻；M4 13377独立。 |
| 全史/恢复/性能 | 原三层对拍、台账丢弃/格式重建/重建+k步、独立进程9011双跑、g86/9011冷与真杀恢复、内容schema不变/回退正控、原期限和性能门；按原重型窗口留证，不扩窗、不缩历史。 |

## 6. 下一实施片切分

这是 M7D 第3片内部的4个可独审子片，3A → 3B → 3C → 3D。3A/B只在测试驱动，3C首次触及非测试调用点。3D结束也不等于M7D第4片回收已获放行。

| 子片 | 输入 | 输出/实施落点 | 禁止项 | 独审与CI完成判据 |
|---|---|---|---|---|
| 3A 生涯封印 | §1现核、上游接口/CI缺件清单 | owner私有guard/phase seal/factory与测试adapter；复用原clamp核，L1..L5/P1测试对拍，证明同父/非自比较/自身写phase门序 | 不接非测试reader，不推进clamp baseline，不重签资格，不用closure保生涯，不回收 | 原门序/受控写phase源码证明，真实外连接/换库/WAL/新进程与自身写正控实际执行；双warnings、M8A/B前提收据归档；非作者独审PASS。无法解释phase转换则留在此片。 |
| 3B 成功继承与SQL费用证据 | 3A guard；现Qualification/单位核；锁版本SQLite机制 | QualifiedSuccessView/核签发Delta类型/台账stage接口在测试中对拍；L6..L11、P2..P6；j.cap费用传递和B_sql同父方案证明/只读读数，新增SQL/F分桶表 | 不调用诊断Probe作准入，不把经验峰作硬界，不造新预算池/帽/生产测量，不回收 | 单位/位置/根头/失败/台账phase全锁；真实J精确/少1B与原失败场景；实测与同父账相符，B_sql覆盖证据可复核。原比例/上界所需普通binary读数无法在测试片取得的保留为3C受控CI硬前提，不写通过。SQLite需新政策则按§7交主控。 |
| 3C 首次生产调用点 | 3A/B非作者PASS、实际CI收据；SQLite覆盖证明；原资源档/读数manifest | `effects::audit_native_source :1462-1495` 来源准备与owner核接唯一reader，仍显式j.cap；旧capture暂保，真实生产构建运行新路径并交新增SQL/F/metadata/峰收据 | 无旧新运行时开关/失败回旧核，无MAX迁移，无淘汰/回收，不改业务effects/角色/资格/错误/期限/读数定义 | 源码可达图、P组真实调用差分、原four/六代/资格/effects/预算结果逐项登记；本片受影响物理观测按下段重冻。:27与重型前提未过则本片不签生产接线完成，不能以微锁替代。C/D原累计失败如仍在，照实保留原红灯，不宣称整票完成。 |
| 3D 引用与能力消费迁移 | 3C已验收的reader、M8A持有者表与成功台账 | NativeCountsSubsetReader、SourceFields/payload、字段/标量/身份消费者和资格交接用受管描述符/租约，原业务核仍同一份；逐调用生命周期/容量表更新 | 不drop旧capture backing，不减仍存活角色费，不把全史成功表藏进前沿，不自动重建失败台账 | certified/qualified、canonical、Counts三闭包/rollback、同父释放/首错及冷在线对拍；八项证明逐项给缺口/收据，特别①成功表与⑧冷折叠。原读取迁移完整后，另由主控按M7D第4片门放行回收。 |

3C及之后的物理重冻手续：先枚举受影响调用点和观察字段，仅限 Native SQL/F/metadata、分配/释放/驻留/峰、必要descriptor/证书载入费用及采样次数；保原统计定义。取得原命令、构建SHA/feature/profile、输入/资源档摘要、原始输出和旧/新逐项差异；精确预算锁由新实测峰定位其少1B边界。非作者用独立进程只读对拍通过，再交主控接受重冻。ordinary full_some与无关局部锁不动，业务effects序列/资格/拒绝义务不能混进物理差异。仅在测试候选分支发生的变化也要如实记录，不能据其与生产相等签零变化。

本准备阶段没有要求立即重跑完整历史。实施先靶向/差异枚举再对拍；全量9011等依原具名窗口，原验收不豁免。3C的接线候选可供受控CI验证，但依赖实测的生产完成声明必须等待真实结果。

## 7. 交主控与明确未决项

1. **SQLite内部覆盖额仍缺硬证据。** 已选整体连接域归属接口，但当前代码/收据没有可直接填写的B_sql上界。3B可在原预算内先做只读CI实测和源代码覆盖证明；如果必须增加allocator拦截、SQLite硬限额/全局配置或新资源政策，已越本轮范围，交主控另裁。此处不自选数值、不用cache_size或经验倍率代替证明，未解不交3C生产调用点。
2. **本切面生涯核须保原自身写语义。** MaterialAccess/Clamp现分持，直接在自身写后跑写前baseline全等会误拒。3A必须用原journal/effects与原物理核证明phase转换，不签发新的历史资格；若需改变effects定义/认证时点/原首错顺序才能接通，交主控，不自增宽松核。
3. **:27及上游CI是待CI前提。** 没有五常数冻结manifest/普通binary比例与上界/实际锁收据就不签通过。若新SQL读数过门失败，先查重复查询、单位未止步、资格重复验证和工作集共存；如必须改F/Δ_R定义、阈值、deadline或原规模才能过，交主控，不能以物理值可重冻授权它。
4. **第3片不会自行消除累计持有首拒。** 本轮禁止回收，现 `finish_owner_step` 保全部正文。完成ii并移除累计翻倍后的g23/g26通过证明归M7D第4/5片；3C/D原锁若仍红只能报告未完成，不改断言、不加ignore，不据本文开启回收。八项有界证明仍以M8B §4为待结清清单，局部k/h不替代①/⑥/⑦/⑧。

除以上实施缺证/可能越界分支，本文未发现必须修改已批合同才能描述的接口变化，也未作任何新政策裁定。status=ready仅表示准备文档和交棒切片齐备。

## 8. 本阶段检查收据

本机只做只读源码/文档/合同取证、文档写入和静态检查；未执行cargo/rustc、Rust测试或任何间接启动它们的脚本。最终空白/差异/历史前缀检查记录追加在M2-NOTES「M8D」；其通过不替代本文件各处明确列出的待CI前提。不git add/commit，不回退并行改动。
