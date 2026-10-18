# #1527 持久锚接口接线提案（未应用）

本文件已按独评 BLOCK 的 M-01 修正；前轮检查记录仍为原件，本轮证据以 `../issue1527-r4-repair-20261018/REPAIR.md`、CHECKS、MANIFEST 为准。

本文件绑定固定 5cb7253d46d973669e7d10cd3331e12c9ef313b1；是 OPEN #1527 工作草稿，不是 main producer 准入。所有下表行号、blob、现有签名均从该 Git 对象读取，并与工作区原文件逐字比较。新模块无 Git blob，交付 SHA256 见 MANIFEST。

## 当前接口事实

5cb 没有 `s_session_v3/admission.rs`、`WriterScope` 或 `Job::new_started`。S3 Driver 在 `process_lock` 后直接 `AuditConnection::open`；S2 直接持 File 锁和 `open_db`。当前 Python 也没有恢复 R4 发布器。不能把历史 v2/v3 候选的函数名和行号写成这些接口已经存在。

新模块直接复用 `newchan_rust::s_content::job::JobLimits` 和 `session_protocol::{strict_json, sha256_hex}`，不引入新的 crate/Cargo feature。类型构造、解析及路径打开使用现有 std/libc/serde_json 依赖；本轮仅 rustfmt 语法检查，类型/ABI 仍 NOT_RUN。

| 固定文件 | Git blob OID |
|---|---|
| `rust/src/s_content/job.rs` | `643f8963118da91674387ae452fc9482e2ebc2c2` |
| `rust/src/bin/s_session_v3/mod.rs` | `6e63b439e07957ecde205fa721c76337c96ee041` |
| `rust/src/bin/s_session_v3/writer_cli.rs` | `6a5c14f8c14fff31f329c0f54c787d48e45b3772` |
| `rust/src/bin/s_session_v3/driver.rs` | `33d386b6592c28515705cf6f2ed2c621360ff896` |
| `rust/src/bin/s_session_v2/mod.rs` | `f9c9a9936fb10887186009f102d3f1d8e8fc3662` |
| `rust/src/bin/s_structure_session.rs` | `1b30590f9bd90827a5b5e1d714b1cac57a53ca11` |
| `rust/src/session_protocol.rs` | `6689c7d17d0b5de0ff7e878caa0988ec66e07d5e` |
| `rust/src/session_transport.rs` | `f2f4e927ae9c4261c2eed6e416a57d6047271ddb` |
| `s_session/s_service_control.py` | `c597d1e293fad7805ce7f4af6a5579534176c932` |

函数级签名及所有当前行号见下表；完整逐项原文与文件 SHA 在 SOURCE-EVIDENCE.json。表中只选生产相关函数，不把测试构造器当调用点。

| 位置（固定5cb） | 现有签名/作用 | 精确改动归属 |
|---|---|---|
| `s_content/job.rs:97` | `JobLimits` 八个 pub u64 字段 | 新模块直接返回这一类型，无新 schema |
| `s_content/job.rs:112` | `JobLimits::from_canonical_json(&JsonNode) -> Result<JobLimits, CodecError>` | strict_json 之后调用；错误原文被稳定类别替换 |
| `s_content/job.rs:599` | `Job::new(JobLimits, Existing, Box<dyn Clock>) -> Result<Job, CodecError>` | 原 new 不改；仅提案新增 new_started |
| `s_content/job.rs:719` | `Job::charge_wire_bytes(&mut self, u64) -> Result<(), CodecError>` | 先实际入账；当前 wire 先于 deadline，保留原函数行为 |
| `s_content/job.rs:1021` | `Job::finish_check(&mut self) -> Result<(), CodecError>` | 提案在 E8 调用；不重起钟 |
| `s_session_v3/mod.rs:123` | `dispatch(&[String]) -> Result<bool, String>` | 保持 selector；模块声明提案显式公开到 crate 内供 S2 消费 |
| `s_session_v3/writer_cli.rs:192` | `run(&[String]) -> Result<Option<Value>, String>` | 完整旗标解析之后，每个写子命令只 load 一次 |
| `s_session_v3/writer_cli.rs:205` | `parse_flags(&str, &[String], &[&str]) -> Result<BTreeMap<String,String>, String>` | 原严格解析不改；三 allowlist 只增已有设计的 --admission-job-limits |
| `s_session_v3/writer_cli.rs:467` | `serve(ServeOptions) -> Result<Option<Value>, String>` | 追加 PreparedAdmission 按值参数，转发到互斥 Driver 分支 |
| `s_session_v3/writer_cli.rs:565` | `advance(AdvanceOptions) -> Result<Option<Value>, String>` | 同上；一次性参数沿调用链转移 |
| `s_session_v3/writer_cli.rs:593` | `recover(RecoverOptions) -> Result<Option<Value>, String>` | 同上；不得仅给 recover 一条路径接锚 |
| `s_session_v3/writer_cli.rs:375` | `preflight_input(&AdvanceOptions, &ClockPlan, &Value) -> Result<(), String>` | 现有源只读开库前资格仍需根协调；见下文未闭合项 |
| `s_session_v3/driver.rs:97` | `Driver::open_for_serve(&Path,&Path,&str,ClockPlan,usize) -> Result<Self,String>` | 追加 PreparedAdmission，无默认参数 |
| `s_session_v3/driver.rs:114` | `open_for_serve_with_allocation(..., Option<q_allocation::Profile>)` | 同一 token 转发 |
| `s_session_v3/driver.rs:132` | `open_for_serve_with_mode(..., s_allocation::Mode)` | 同一 token 转发 |
| `s_session_v3/driver.rs:147` | `open_for_serve_with_identity(..., s_allocation::Mode)` | Native 分支也接同一 token；不碰 audit_owner/native 算法 |
| `s_session_v3/driver.rs:197` | `open(&Path,&Path,&str,ClockPlan) -> Result<Self,String>` | 追加 PreparedAdmission；现有测试不得静默造默认锚 |
| `s_session_v3/driver.rs:207` | `open_with_request(..., Option<&Value>)` | 同一 token 转发 |
| `s_session_v3/driver.rs:218` | `open_with_request_and_allocation(..., Option<q_allocation::Profile>)` | 同一 token 转发 |
| `s_session_v3/driver.rs:231` | `open_with_request_and_mode(..., s_allocation::Mode)` | 同一 token 转发 |
| `s_session_v3/driver.rs:242` | `open_with_request_and_mode_context(..., Option<&mut RequestAuditContext>)` | 根供 WriterScope::admit；Driver 保存 scope 而非裸 File |
| `s_session_v3/driver.rs:456` | `recover(&Path,&str,&ClockPlan,&str) -> Result<Value,String>` | 用同一 scope；在 open 原件前 require_admitted |
| `s_session_v2/mod.rs:2469` | `serve(&Path,&Path,&Path,&str,&Path,Bounds) -> Result<(),String>` | 按值追加 token，scope 存活到服务退出；handler 及 pending 前复核 |
| `s_session_v2/mod.rs:2535` | `dispatch(&[String]) -> Result<bool,String>` | serve/recover 各加载一次，不能借 env 缺失跳过 |
| `s_session_v2/mod.rs:2607` | `legacy_write_lock(&[String]) -> Result<Option<File>,String>` | 提案返回持 scope 的 guard，accept/advance/recover 使用相同合同 |
| `s_structure_session.rs:5394` | `run() -> Result<(),String>` | :5405 原锁局部变量改持 guard，并保留到命令结束 |
| `s_structure_session.rs:319/2182/3081/5283` | `open_db` / `accept_core` / `advance_core` / `recover_core` | 最终源资格与每次开库复核归根；本轮不复制这些算法 |
| `session_transport.rs:43` | `process_lock(&Path,&str) -> Result<File,String>` | 只引用其 canonical 语义；锁与维护 failfast 保持 |

## 精确补丁和依赖

`INTERFACE-JOB.patch` 是两个文件的精确 unified diff：固定 Job 文件新增显式真实起钟构造口；本轮新消费者新增 `ConsumedAdmission::into_job(self, measured: Existing)` 与 `StartedAdmission::finish(self)`。补丁没有应用。new_started 校验真实时刻不在未来、checked deadline 与当前未过期，然后沿用原 new 的实测基线检查，设置同一个 started。into_job 先用冻结限额核原 t0/wire，创建成功后一次 charge 实际前导量；构造或回灌失败不退还可重用 token。finish 在私有工作完成后复核原 lease、原 Job deadline/finish，最后交出 Arc；源身份及 admitted 标记由 scope 完成。原 `job_mut() -> &mut Job` 已删除，改为 `accounting() -> AdmissionAccounting<'_>`；下节定义受限门面及调用者约束。失败的 JobAdmissionError 增加原 JobSnapshot，保留 wire、停止原因、两类预留及基线。

`INTERFACE-CALLERS.patch` 是五个既存文件的精确 unified diff，覆盖 S3 三 CLI 和所有 Driver 包装、S2 serve/recover、legacy accept/advance/recover 的移动传递及持有者。该补丁明确依赖 **根另行交付已具完整资格的 admission::WriterScope**。这份提案不能单独构建：5cb 无 provider，现存测试调用也没有新增 token 参数。补丁仅锁定调用者变化形状，不附一个全面拒绝或绕过检查的替身 WriterScope。

根协调的接口形状如下；这是合同，未在当前源码添加空壳实现：

```rust
// 根持有的 admission.rs；具体算法沿既有批准的恢复/资源链合流。
pub(crate) struct WriterScope { /* 原 writer/维护锁、源资格、Arc<BudgetLease> */ }
impl WriterScope {
    pub(crate) fn admit(
        requested_db: &Path,
        prepared: admission_anchor::PreparedAdmission,
    ) -> Result<Self, String>;
    pub(crate) fn db_path(&self) -> &Path;
    pub(crate) fn require_admitted(&self) -> Result<(), String>;
}
```

`admit` 必须消费准备态，requested_db 与 lease 内实际 DB 一致，否则拒绝，不能用另一 DB 搭配预算。E5 先核、取唯一 writer/maintenance failfast、再核，任何 E6 清理前保证原 lease；E6–E7 的既存全域基线、allocation_upper、clone/私有 SQLite 资格继续用批准原算法。测出 Existing 才 into_job，不能补 0。E8 私有操作全部收尾后调用 StartedAdmission::finish，再核源身份，最后才能设置 admitted；scope 保留 Arc。E9 require_admitted 必须核标记、原 lease、源，不能重新解析 path 替换限额，不能创建新 Job。

## 这些提案尚未闭合的接线面

1. 5cb 没有恢复 scope、maintenance/private admission 或可信 allocation provider，不能从本次 loader 正控推出可打开原件 SQLite。消费者有效成功返回仅是 R4 准备成功。
2. Caller 提案从 loader 起保留真实时间，但 `SResources::load`、`ClockPlan::load`、profile/input 读取、preflight、CLI stdout/stderr 等原 I/O 的实际累计还需由根合流到同一 Job/前导账；目前补丁没有谎称这些已计。E6 前后的 I/O 必须逐段检查单一期限，不能只在最后补一门。
3. 当前 `Job::charge_wire_bytes` wire 首错早于 deadline。消费者的 E4–E5 只处理控制三件，已按 deadline 优先；它们在可信后不再读 payload。JOB 提案的前导回灌先经同一 gate，因此控制量不会靠此原方法降级首错。其它真实 I/O 继续严格保留共享 Job 的 wire 先累计、wire 帽后 deadline；门面不加前置 gate 包住 charge_wire_bytes，不转译原 CodecError。截止优先仅属于可信 prelude。
4. `preflight_input` 当前自己开只读源库；legacy core 可在一个命令里多次开原件。提案中的入口复核不足以证明所有 E9 开库点：根必须把实际 scope 引用传到 `open_db`/core 或其已批准的唯一开库门，核完资格才调用。SQLite pathname 在检查后仍有竞态，本片不证明连续性。
5. S2 的旧参数读取器并不是 S3 完整 allowlist 解析器。本模块只严格提取 admission 标志，不能当作所有 S2/legacy CLI token 已全审。根需保持各原 CLI 合同并明确旧入口重复/缺值首错。
6. `init/reset` 的原 File 锁分支在 Caller 提案中被原样包入 `LegacyWriteGuard::Initialization`；这只是保留旧行为，不授予 R4 bootstrap、重置或既存无锚接管资格。新 prepare/迁移路径仍不实现，不把该分支当成已受 R4 保护的恢复入口。
7. 原生产/测试调用者需要逐个适配新增必填 token；锚测试只能通过真实 loader 建立合法临时锚。不能增加生产 TestOnly 绕过参数。需由后续获准构建执行当前 Rust 测试及原真实写入口回归，Rust NOT_RUN 不转换为通过。

上述缺口属于根保留的生产接线/合流，不是本轮独立消费者的隐式批准。两个 patch 仅检查能否匹配固定文本、能否 rustfmt 解析；不是编译结果，也不是 main 合入方案已通过。生产 v4 保持 BLOCK_NOT_MERGEABLE。


## M-01 修正：只借记账门面，原账留在父持有者

`StartedAdmission` 的 job/lease/first_read_at 字段仍私有，不实现 Clone、Copy、Default、Deref、AsMut 或提取 Job 的入口。它仅公开 `accounting(&mut self) -> AdmissionAccounting<'_>` 与消费型 `finish(self) -> Result<Arc<BudgetLease>, JobAdmissionError>`。门面仅有私有 `job: &mut Job`，没有公开构造口。根持有的 WriterScope 准入流程保留 StartedAdmission 本体，不向 provider 传递本体或 `&mut StartedAdmission`。

门面 API 与共享 Job 一对一委托（返回原 CodecError，不新增宽松判据）：

| 门面签名（均 pub(crate)） | 原 Job 委托/调用责任 |
|---|---|
| `check_deadline(&self) -> Result<(), CodecError>` | 已停止优先沿用原错，再检查原期限 |
| `first_read_deadline(&mut self) -> Result<Instant, CodecError>` | 原 started 已设，不续期 |
| `pre_io_check(&mut self) -> Result<(), CodecError>` | 真实 I/O 前原门 |
| `charge_wire_bytes(&mut self, u64) -> Result<(), CodecError>` | 已发生量先累计，wire 帽后 deadline；不额外前置期限检查 |
| `charge_records/charge_requests(&mut self, u64) -> Result<(), CodecError>` | 原累计语义 |
| `accept_node(&mut self, &NodeDigest, u8, &[u8]) -> Result<NodeAcceptance, CodecError>` | 原 F/节点去重/预算门 |
| `accept_frame(&mut self, &NodeDigest, u8, Vec<u8>) -> Result<CheckedFrame, CodecError>` | 原 owned frame 路径 |
| `accept_checked_usage(&mut self, &CheckedNodeUsage) -> Result<NodeAcceptance, CodecError>` | 原已核用量；provider 仍须证明来源/复用合法 |
| `reserve_staged/reserve_store(&mut self, u64) -> Result<Grant, CodecError>` | 同一原 Job；暂存同步占总账 |
| `reconcile(&mut self, &Grant, u64) -> Result<(), CodecError>` | 实测分配；超预留也先入 actual |
| `release(&mut self, &Grant) -> Result<(), CodecError>` | 仅退出原未决预留；Grant 的原 Job 身份校验不变 |
| `reclaim_staged(&mut self, u64) -> Result<(), CodecError>` | 只核实际已删除的本 job 暂存，不能减历史基线 |
| `snapshot(&self) -> JobSnapshot` | 拷贝快照，无修改原账能力 |
| `node_source<'b>(&'b mut self, &'b mut dyn NodeSource) -> JobNodeSource<'b>` | 现有只读包装私有持借用，无 Job 提取口；本身不计 wire/request |

根协调 provider 的精确边界：在原签名中将准入记账参数 `job: &mut Job` 替换为 `account: &mut AdmissionAccounting<'_>`，其余 source、allocation、私有 SQLite、锁和清理参数/返回值维持原恢复合同。固定 5cb 没有这些 provider 函数，故不能编造其名字或“已修改”行号。其**全部角色**为 E7 写前预留、clone/私有 SQLite I/O 适配、内容节点/record/request 记账、写后 allocation 对账、失败取消/实际删除清理、stdout/stderr 外围实量适配；所有角色只能收门面，不得再调用 `job_mut`、闭包 `&mut Job` 或重造 Job。E6 的完整 Existing 测量在 into_job 之前，必须仍受原 prelude 和来源门，不因门面改动伪造空基线。

调用顺序必须是 `consumed.into_job(measured)` → 父流程持 `started` → 局部 `let mut account = started.accounting()` → provider 使用 `&mut account` 完成原算法及失败清理 → 借用结束 → `started.finish()` → WriterScope 核源身份并设置 admitted。门面不能 finish 或取得 lease。把门面替换成另一份门面只交换借用视图；原 started 中的 Job、未结预留、停止状态不动。另一本 Job 的 Grant 也不能对账原 Job。安全 Rust 不能把新 `Job` 填进门面字段或通过解引用取得原槽位。模块内部的实现代码仍是可信边界；此设计不承诺防止修改实现源码或 unsafe 破坏。

固定生产调用者仍是前表全部 S3/S2/legacy 移动传递链，签名不因门面再变；CALLERS.patch 只补模块依赖合同注释。`capture.rs:2468/2499/2940`、`q_executor.rs:890`、`q_runtime.rs:52` 现有 Job 使用属于自己的请求/测试域，未引用新 StartedAdmission，不迁移、不改其 Job API。若后续 provider 要复用要求裸 `&mut Job` 的适配器，须经 `node_source` 或另行受限适配；不得以“现有函数需要”为由重开裸借用。现有测试调用与五文件提案的 token 适配义务不变，尚未执行。

失败与析构：consume/into_job 失败消费旧 token，局部 File/Arc 与新建 Job 按所有权析构，不回送可重试准备态；构造/回灌失败的 AdmissionError 保留原 t0 和 prelude 实读量。provider 失败先沿原清理算法关闭私有句柄、按实际 reconcile/release/reclaim，停止后这些清理口仍可用；丢弃 Grant 或门面不会替代对账/清理。finish 按值消费，先复核 lease，再调用原 check_deadline，继而处理路径错、原 finish_check；失败返回原 t0/u128 wire/完整 JobSnapshot，不返回 lease 或 holder。成功只交 Arc，原 Job 账不再可用，零未结预留须由原 finish_check 证明。Drop 不执行物理清理、不重签、不自动释放未结预留。所有私有 SQLite/文件清理应在 finish 前完成；finish 后路径/期限失败不授权原件开库。WriterScope 的源身份后置核验失败也不得设置 admitted。

固定基线的调用/定义/不变转发逐行清单见返修目录 `CALLER-INVENTORY.json`（76 处词法位置，含 Driver 测试区及 recover 原方法；不把全部命中都算签名变化）。前表给出实际变更签名，清单补齐直接测试构造与 CLI argv 适配位置。没有可用图工具，采用定向搜索及逐行固定源码核对，不声称此清单是 Rust 语义引用分析。
