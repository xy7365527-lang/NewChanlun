# #1481 H1 受管阶段 S1 交付记录

> 名分：#1481 活期工作草稿，仅记录本阶段候选与未完成项，不是教义或架构正本。
> 状态：blocked。核心第 1 项未完成；第 5 项已写测试及 CI 接线，但没有 Rust 执行证据。当前 WIP 不具备准入验收条件。

## 工位与范围

- 工位：`/Users/silencehan/Projects/NewChanlun-sandcastle-host/.sandcastle/worktrees/sandcastle-issue-1481`。
- 分支：`sandcastle/issue-1481`；HEAD/实施基线：`b0db7a8d71f929318e539a57c9dd42161f4fbec6`。所有候选均为未提交工作区改动，没有新 commit 或 PR。
- 最后只读观察时 `origin/main` 已为 `5c341d8047722ca4dd4e3c25b78f2c6d637af7f9`；本工蜂未 fetch、切分支、重置或追并该引用，候选基线仍是上述 HEAD。
- 已读主控 brief 43K、裁决 43I、调查 43G/43G-2、39P INTERFACE、#1481/#1647/#1644 原件及 AMENDMENT、PR1756 文件名单、实际源码和两个 workflow。
- 本阶段遵用户 S1 范围。43K/43I 的资源 `/3`、scrub、launcher/5、Python、ADR 留 S2，不因 brief 全合同而提前实现。
- 原 brief 指定的 Opus 模型调用未被当前工具接受，实施代理实际沿本会话默认模型执行；独立只读评审使用 `gpt-6-astra`。未把工具拒绝的路由记为实际执行。

## 阻塞与唯一接续动作

独立评审确认以下两个 P1，实施方复核同意，尚未修复。

1. **冷认证真实 wire 未接原准入账。** `rust/src/bin/s_session_v3/admission_private/mod.rs:421` 把增量私有前缀改为 schema 检查后，正式冷折叠从普通 `open_db` 连接读取。`rust/src/bin/s_session_v3/admission.rs:200` 已把节点、记录及原首读期限接回同一个 `FinishedAdmission`，但该门面没有 SQLite 实际读取的 `after_read`。`rust/src/bin/s_structure_session.rs:343` 的默认 SQLite 连接没有原 FD/VFS 计量，原 `admission_private/mod.rs` 的 FD/VFS 注册只覆盖私有前缀会话。前面的 quick_check 实读不能抵消冷折叠再次读取；SQL body 长度也不能冒充实际 wire。
2. **迁移可能先于历史认证写源库。** `rust/src/bin/s_structure_session.rs:351` 的 `migrate_legacy_policy_records` 位于 owner 开门及 H1 冷折叠之前。原私有完整审计曾在此之前拒绝坏历史；现在仅 schema/epoch/rule 预检，对 epoch 正确而历史损坏的 `PrePolicyRecords` 库可能先迁移、后拒绝。既有错 epoch 的“源库不迁移”锁不能覆盖这个负例。

唯一下一步是把同核冷取证接入现有私有 FD/VFS 的只读准入阶段，原账核完实际 I/O、节点、记录与期限后，再把已核 capture、来源贡献态、已发布 F、CommittedSet 和资格安全交接给正式 owner；迁移必须在完整认证之后。随后补 `PrePolicyRecords + 坏节点/manifest + 正确 epoch` 的源库/WAL 不变锁与真实 wire 短预算锁，再执行当前 CI。不能恢复旧 `state::audit` 作兜底，也不能以本报告代替这段实现。

## 原合同逐项映射

| 验收 | 本阶段结果 | 证据及限度 |
|---|---|---|
| 接纳、Begin、Prepare、identity、summary、recover 改为 H1 资格 | 部分完成，阻塞 | 正式入口、重建及在线推进已写入；上述 wire/迁移前置仍未闭合，不能签正式准入完成 |
| 新生涯/recover 从 g1 冷认证，完成前不接写 | 已写候选，未运行 | 同核来源贡献步重建 fresh P6，再从空前沿消费资格；真实 Driver 锁含重启续写及 R>F |
| 内容 COMMIT → 台账 COMMIT → advance_qualified，失败 stale | 已写候选，未运行 | 复用 PR1816 推进及 P6 stage 前核，禁止缺候选/资源不足重试全审计 |
| 旧审计对照臂与退场条件、两处旧 allow 删除 | 已完成源码修改 | 注释带 #1481；原公开组件 wrapper 改为 cfg(test)，生产消费抽出的共同 seeded 核，不能声称原 wrapper 已有生产调用者 |
| 写者准入 Memo 退役、读侧不动 | 已完成源码修改，未运行 | 增量正式接线不再调用旧审计/Memo 配置；cache.rs 文件头改口径带 #1481；旧 v2 serve 与 Q/E/B 读侧保留 |
| 准入三层完整规范对拍及正式入口无旧审计锁 | 已写 8 锁并接 CI，未实际执行 | 接受时比完整规范字节，负例比拒绝类别；没有运行 PASS、逐名 CI 日志或 Rust 编译证据 |
| 保持 Ordinary、Q/c2、资金、隔离、64 GiB、30 秒、FU16 | 未完成整体验证 | Ordinary 新交接均 cfg 增量、旧控制流保留；Q/c2/时钟/资金/隔离未改。64 GiB 使用原每库帽，首读沿原期限；实际 wire 与迁移前置仍有回归，不能宣称全部保持 |
| 资源 `/3`、cold_start_window、scrub 两字段 | 留 S2 | writer_cli 资源 schema、版本、字段和值不变；冷入口显式 deadline 并返回 elapsed，无新默认冷窗口 |
| launcher/5、ADR 修订与其余 Python 服务控制 | 留 S2；S1K 同步现役调用方 | Ordinary `/1` 保留。s_incremental 写者 recover/advance 已要求显式 `--resource-config`，不能声称未迁移的 launcher/4 恢复合同仍兼容；S1K 为 launcher/4 recover 传入 start-s 同份、已核身份的冻结资源原件，launcher/4 其余参数、阶段、错误与返回帧保持。 |
| 两字段解析/AwaitingPolicy/超窗/scrub 轮转验证 | 留 S2 | 本阶段没有这些资源字段，不把现有 deadline 等同于 S2 冷窗口验收 |
| #1419 g77 消费、9011/G4、默认切换、整票关闭 | 未做 | 不跑 28/106/9011/S86；未作性能或生产资源签值，不关票、不评论、不改默认构建 |

## 当前真实调用链

以下是候选源码路径，不是已运行证明。

| 阶段 | 调用链 |
|---|---|
| serve/一次性 advance | writer_cli 显式消费既有 `/2` 第八门 → Driver 冻结 Native context → `rust/src/bin/s_session_v3/driver.rs:566` → `rust/src/bin/s_session_v3/audit_owner.rs:449` |
| recover | `rust/src/bin/s_session_v3/driver.rs:815` → 新 Runtime → open_rebuilt_history → owner.recover |
| 共同冷开门核 | `rust/src/bin/s_session_v3/audit_owner.rs:388` → `rust/src/bin/s_session_v3/audit_owner.rs:1501` → `rust/src/bin/s_session_v3/state.rs:1766` |
| 冷来源/P6 | HistoricalSourceFold 逐代核 cut → ColdSource/LazySet → `rust/src/bin/s_session_v3/source_contributions.rs:1962` → receipt 的同字段导出核 → Ledger.stage_verified/bind_view/content_committed |
| H1 资格 | 同一 cold capture 的 cuts → `rust/src/bin/s_session_v3/state.rs:1692` → QualifiedCertification::fold → 与在线共用 step；没有依赖旧台账资格 |
| 重启后续写 | `rust/src/bin/s_session_v3/construction_state.rs:256` 同时承接 F/raw binding、NativeSourceState 和 CommittedSet；R>F 尾部留下一次在线 Δ |
| 接纳/Begin/identity/summary | owner.bind_admitted_history → 当前事务 bind_existing；缺完整资格即 stale；Prepare 要求已绑 H1 |
| 在线发布 | `rust/src/bin/s_session_v3/audit_owner.rs:1012` 在内容提交后提交台账 → `rust/src/bin/s_session_v3/audit_owner.rs:1022`；失败清候选并停止写者 |

`open_certified_history_component(.., Some(QualifiedSource))` 旧接口本阶段保留为测试 wrapper。生产重建入口与它调用同一个 `open_certified_history_component_seeded`，以便一次捕获中承接冷来源状态，避免二次开 owner 丢失 F/set。此接口形状与用户点名的字面调用有差别，且仍有上述资源/迁移阻塞，不能按原接口调用验收完成。

固定 `.audit-ledger.sqlite` 缓存每次锁内冷重建，旧文件不作权威输入；删除仅接受同用户、单链接普通文件。台账及内容库共用原 `max_store_bytes`，上界仍为 64 GiB。候选没有删除完整重算入口。

## 原准入调用替换清单

| 基线调用 | 当前候选 |
|---|---|
| audit_owner.rs:567 Native startup 的 audit_native_in_request / audit_startup | 认证 owner 先 bind_admitted_history；旧调用只留未装正式资格的 Ordinary/诊断分支 |
| audit_owner.rs:609、612 identity 两个 audit 分支 | 认证 owner 绑当前 H1；旧分支不作失败兜底 |
| audit_owner.rs:632 summary 的 state::audit | 认证 owner 绑当前 H1；保留 Ordinary/诊断对照分支 |
| audit_owner.rs:1161 recover 的 state::audit | 认证 owner 重新绑定/消费资格；监督器从新 Runtime 冷建后才调用 recover |
| admission_private/mod.rs:423、431、439 三处 measured/full audit | cfg 非 s_incremental 原样保留；增量改拒绝前缀 schema/epoch/rule；此替换正是未闭合 wire/迁移前置的阻塞点 |
| writer_cli::preflight_input 的 state::audit | cfg 非 s_incremental；增量只预检请求身份，完整认证交持锁 Driver |
| prepared::Runtime 完整审计备用分支 | 已装完整正式台账时缺 H1 bound_audit 立即拒绝，不回落；测试 oracle/离线及 Ordinary 路径保留 |

注释退场条件见 `rust/src/bin/s_session_v3/audit_owner.rs:539` 与 `rust/src/bin/s_session_v3/state.rs:158`。三层锁在 CI 真执行且 9011 双跑通过之前，不删除完整对照入口。v2 的 `cache::configure` 仍服务旧 v2/读侧；v3 中剩余 configure 命中均为测试/离线诊断，未把它重接正式 H1 路径。

## 既有覆盖与本阶段补锁

PR1816 既有 `h1_qualified_*` 实际为 6 条，43G 的“四条”数量不准确：

| 既有测试后缀 | 已覆盖 |
|---|---|
| online_consumes_real_ledger_and_cold_rebuild_matches | 在线资格、冷/在线完整历史规范序列化与前沿相同 |
| cold_rejects_missing_partial_or_foreign_qualification | 缺资格、部分台账、未安装台账 |
| cold_rejects_identity_rule_root_and_unit_mismatch | 内容/二进制身份、规则、cert roots、根单位缺失 |
| ledger_commit_failure_and_external_identity_write_are_stale | 台账 COMMIT 失败、外写身份 stale |
| stage_rejects_externally_rewritten_fold_digest_or_rule_revision | stage 前外写 fold_digest/rule 拒绝 |
| transition_short_budget_rejects_before_write_and_releases | 资格交接短 Native 保留预算、写前拒绝及释放 |

36T 既有 `h1_cert_*` 为 3 条：空史/修订赢家/未发布尾部完整规范；控制代号/manifest batch/索引前沿/catalog 状态/删除全部节点拒绝；deadline/scratch 失败释放。其负例原先只比 `is_err`，没有准入拒绝类别及真实 Driver 重建续写证明。

新增测试全部无 ignore：前 7 条位于 `rust/src/bin/s_session_v3/audit_owner/admission_tests.rs`，完整模块前缀 `v3::audit_owner::construction_tests::`；第 8 条前缀 `v3::writer_cli::tests::`。

| 测试名 | 本阶段缺口 |
|---|---|
| h1_admission_cold_prefixes_match_full_canonical | 空史至多代前缀、从 g1 折叠、接受时完整规范字节 |
| h1_admission_cold_negative_classes_match_full_oracle | 坏节点/缺节点/同 ID 换内容/索引/clock/manifest 的普通 audit、Native oracle、正式冷准入拒绝类别一致 |
| h1_admission_online_own_commits_match_full_canonical | 自身每代提交完整规范与旧 Native oracle 逐位相同，完整资格一致 |
| h1_admission_foreign_sqlite_changes_retire_writer | 独立 SQLite 连接真实 UPDATE/DELETE/COMMIT；相同六负例导致 stale 或拒绝，不能重判 |
| h1_admission_formal_accept_begin_prepare_recover_never_full_audit | 正式 owner startup/identity/summary/接纳/Begin/Prepare/recover 计数区间无旧审计，oracle 正控在区间外 |
| h1_admission_driver_cold_recover_rebuilds_ledger_bitwise | 新 Driver/新 Runtime、污染旧台账后重建、冷后继续修订/撤回、再次冷、R>F recover、epoch2 pending 续写再冷；全台账 dump 与完整历史逐字节对拍 |
| h1_admission_driver_cold_preserves_original_node_record_gates | 原 Job 的 max_total_nodes/max_total_records 独立短预算，正式 Driver 冷入口拒绝且不写、无旧审计 |
| h1_admission_one_shot_requires_explicit_retention_resource_file | 仅增量 advance/recover 必须显式既有 `/2` 资源文件，缺文件不默认签第八门 |

完整规范调用 `ImmutableHistoricalAudit::h1_canonical`，穷尽解构全部字段。Native oracle 与 cold 比完整规范字节；Ordinary 不携可选 Native 事实，因此负例额外比较 Ordinary 接受/拒绝及类别，未声称 Ordinary 与 Native 可选事实字节相同。没有只比 fingerprint。

仍缺：实际 wire 短预算锁、正确 epoch 的坏 PrePolicyRecords 库迁移前无写锁，以及所有新增锁的 Rust/CI 运行证据。故第 5 项只能记“已写”，不能记“验收通过”。

## CI 接线与实际检查

- `.github/workflows/ci.yml:428` 的 `rust-test-s-session`、`matrix.group == owned` 新步骤执行 release、`s_session,s_incremental` 的 `h1_admission_`。随后逐名 `grep -Fx` 核 7 条 owner 锁及 1 条 CLI 锁，每条必须 `... ok`，零测试或缺名均失败。上传 `h1-admission-S1` 日志 artifact。
- `history36h.yml` 新增本分支 push 触发；现有 `H1 life clamp and history locks` 的 `h1_` 过滤实际包含新锁，也保留 SQLite COMMIT 失败、Native 及 Ordinary 步骤。
- 保留现行三档 `-D warnings`、增量 all-targets、生产产物无钩子/禁串步骤。本次不改变默认构建。
- 本机实际执行：20 个改动 Rust 文件直接 `rustfmt --edition 2021 --check` 通过；`git diff --check`；两个 workflow 的 PyYAML 解析；8 个新锁测试名唯一性及 CI 逐名存在检查；生产无测试钩子扫描；禁串清单生成及一致性检查通过，生成条目 1635；生产无测试钩子扫描 `violations=0`。
- 本机禁止 cargo/rustc，均未执行。没有 CI run/head 日志，没有测试 PASS。远端重型窗口、28/106/9011/S86、g77 性能测量均未执行。
- 独审已纠正的源码问题：冷来源态存在但 CommittedSet 缺失导致下一代 bootstrap；冷节点/记录漏原 Job；旧组件 wrapper 失去生产调用的 dead-code 风险，以及 test_prepare_caps 的 Ordinary cfg 告警风险。修订均只作静态复核，未编译证明。

## PR1756 交集

只消费 `ac52446c14a441b81af8f9e875cf7e0e203f9c82` 的文件名单，没有读取、复制或合入候选代码。实际文件交集：

- `rust/src/bin/s_session_v2/cache.rs`
- `rust/src/bin/s_session_v3/audit_owner.rs`
- `rust/src/bin/s_session_v3/driver.rs`
- `rust/src/bin/s_session_v3/prepared.rs`

共享状态机由 core 单写者实施；CLI 与测试代理仅在明确划分的 helper/文件区协作。未回退其他工作，未 git add/commit/push，未写 Git metadata，未评论或关闭 issue。

## S1b

> 状态：ready，可交主控封包。此状态覆盖上文 S1 的 blocked；S1 原记录保留。两项 P1 已在候选源码中闭合，新增锁及 CI 接线已完成。Rust 未编译、未执行，待 CI；ready 不代表整票验收通过。

### 重放与边界

按用户原命令执行 `git apply /Volumes/AgentStorage/h1-managed43r/s1-wip/tracked.patch` 与 `tar xf /Volumes/AgentStorage/h1-managed43r/s1-wip/untracked.tar`，重放后的 short status 与保存的 `status.txt` 一致。HEAD 复核仍为 `b0db7a8d71f929318e539a57c9dd42161f4fbec6`，分支未切换，没有 Git 暂存、提交、stash、reset 或其他 metadata 写操作。

本轮继续使用 `/2` 原资源合同，未改 `/3`、scrub、launcher、Python 或 ADR。Ordinary 的 `open_db` 开门顺序保留；原迁移体仅抽成共用的事务函数，非增量路径仍在原位置调用。未运行重型窗口、远端清理、默认切换、真实资金或整票关闭。

### 两项 P1 的闭合

1. **真实冷读取进入原准入账。** `admission.rs:181` 的 `WriterScope::certify_cold` 借用既有 `FinishedAdmission`，不创建新 Job、不重置首读时钟。`admission_private/mod.rs:580` 的冷事务与 `:624` 的 `read_session` 复用原固定 FD/VFS、`SharedAccounting`、原预留和全部收尾门。SQLite 的真实 `fd_wal.rs:300` 读取经 `admission_private/mod.rs:353` 的 `after_read(actual)` 入 wire，节点与记录经同一门面记账；没有用 SQL 字符串长度计算 wire。

   `audit_owner.rs:468` 的正式冷入口在这个私有会话中完成 H1/P6 同核折叠、完整 SQL/Q 捕获、资格消费、已发布 F、来源贡献态以及 `CommittedSet`。`construction_state.rs:292` 产生无 owner 的 `ColdRawState`，不把私有 seal 冒充正式 seal。PrePolicyRecords 确定补列所需的线性内存变换和费用核算也在私有阶段完成，见 `audit_owner.rs:2981` 附近的 `prepare_cold_policy_migration`。回滚、关闭连接、撤销 VFS、结原预留和期限门全部成功后，才允许返回这些载荷。临时 Runtime 的台账最后才交正式 Runtime，见 `prepared.rs:254`、`:258`。

2. **完整认证先于迁移。** `s_structure_session.rs:357` 新增专用无迁移开门，仅供已核冷载荷交接，不执行 `journal_mode` 切换。它启用并核对 `NO_CKPT_ON_CLOSE`，避免交接拒绝后的最后连接关闭改写源库/WAL。`audit_owner.rs:641` 先取得 SQLite `IMMEDIATE` 锁，在锁内经 `admission.rs:177` → `admission_provider.rs:349` 核源主库/WAL 完整 stamp，随后才调用 `s_structure_session.rs:434` 的受控补列事务核。因此正确 epoch 的坏 PrePolicyRecords 历史在只读阶段拒绝，不会进入迁移或正式写连接。

   第一事务提交后，在事务外初始化夹持所需的 `wal_checkpoint(NOOP)`；第二个 `IMMEDIATE` 事务核同连接 `data_version`，锁内建立实际夹持基线及 seal。这样既拒绝私有认证到正式绑定之间的外连接 COMMIT，也保留 MainOnly 的合法空 WAL 输入。实际 owner 只刷新 schema 与三份固定配置原件，历史节点和业务行沿已核 capture 移交，见 `audit_owner.rs:3032` 附近的 `rebind_cold_connection`。

   `construction_state.rs:262` 的 `into_owner` 先以既有 `BudgetStamp::advance` 校验原父账与正式冻结域，再一次性重绑 raw、来源态、Set 和 seal；不重新遍历或计费。正式 owner 的交接入口为 `audit_owner.rs:1638`。任何失败都不安装正式资格，不回落 `state::audit`。

### 更新后的真实调用链

| 阶段 | 调用链，路径前缀为 `rust/src/bin/` |
|---|---|
| serve / advance | `s_session_v3/driver.rs:566` → `audit_owner.rs:468` → `admission.rs:181` → `admission_private/mod.rs:580` → `read_session:624` → 既有 FD/VFS |
| recover 冷起步 | `s_session_v3/driver.rs:836` → 同一个 `open_rebuilt_history`，完成资格移交后才 Recover 写入 |
| 私有认证 | `audit_owner.rs:526` → `HistoricalCertificate::cold_for_request_rebuilding` → 同核 P6 来源步/资格步 → CompleteCandidate/capture/Q → `construction_state.rs:292` 的 F/来源态/Set |
| 结账后交接 | 私有 rollback/close/registration/reconcile/deadline → 专用无迁移开门 → 第一 IMMEDIATE 核完整 stamp 并受控迁移 → 事务外 Clamp 初始化 → 第二 IMMEDIATE 核 data_version → `seed_rebuilt_history:1638` → `ColdRawState::into_owner:262` → Runtime adopt |

订正 S1 的接口说明：`open_certified_history_component_seeded` 现在只供测试，已加 `cfg(test)`。正式生产入口直接在原私有只读会话中调用共同的 `HistoricalCertificate` 冷认证核，并移交已核载荷；不再声称正式路径与测试共用同一个 owner 开门包装。旧审计仍只在 Ordinary、测试 oracle 或离线对照分支，不给正式 H1 失败兜底。其他 S1 行为判断经源码复核继续成立，其执行证据仍待 CI。

### 新锁与 CI

三条均位于 `rust/src/bin/s_session_v3/audit_owner/admission_tests.rs`，无 ignore，模块前缀为 `v3::audit_owner::construction_tests::`。

| 锁 | 覆盖 |
|---|---|
| `h1_admission_cold_real_wire_short_budget_rejects_before_write`，`:699` | 用成功正控校准原账前缀用量，再让同形新库通过前缀后在真实冷 FD/VFS read 消耗 wire 时拒绝；检查源库/WAL 字节不变、无请求/Begin、无旧审计 |
| `h1_admission_prepolicy_bad_history_rejected_before_migration`，`:759` | 正确 epoch、缺 policy_records 列，分别破坏节点与 manifest；拒绝后源库/WAL 逐字节不变且仍缺列。随后只修复坏历史，要求同库冷开成功、完成迁移并与完整 Native oracle 规范字节相同，防止缩小输入域 |
| `h1_admission_cold_handoff_foreign_sqlite_commit_rejects`，`:325` | 私有冷认证完成后，独立 SQLite 连接真实 UPDATE/COMMIT；交接必须拒绝，不重认证、不调用旧审计，外写后的源库/WAL 不再变化 |

`.github/workflows/ci.yml:428` 的现有 H1 release 步骤执行 `--features s_session,s_incremental ... h1_admission_`，逐名清单现含 10 条 owner 锁及 1 条 CLI 锁，各自要求完整 `test ... ok`。新三锁已逐名列入，日志仍上传 `h1-admission-S1`。`history36h.yml:19` 的现有 `h1_` 过滤也覆盖它们。没有用 `--no-run` 代替执行。

### 实际检查与独审

- 全部 23 个改动 Rust 文件通过 `rustfmt --edition 2021 --check --config skip_children=true`；此参数只限制检查显式列出的改动文件。独审返修后，对再改文件重跑通过。
- `git diff --check` 通过；两个 workflow 的 PyYAML 解析通过；11 条锁的名称唯一、test 属性、无 ignore、执行过滤及逐名清单检查通过。
- `python3 .github/scripts/check-prod-no-test-hooks.py` 通过，`violations=0`。
- `python3 .github/scripts/gen-prod-banned-literals.py .` 重生成 1636 条；随后 `python3 .github/scripts/gen-prod-banned-literals.py --check .` 通过。最终核心返修后重跑扫描与一致性检查。
- 独立只读评审为非作者 `gpt-6-astra` 会话。发现并修复了 CStr 参数类型、Budget trait 作用域、事务内 NOOP、交接外写间隙、冷 Set 构造顺序与父账戳交接问题；最终静态复核未发现剩余确定阻塞。
- 本机未运行 cargo/rustc，也没有新 CI/head 的逐名测试日志。没有声称 Rust 编译或测试 PASS。

### 仍未闭合

本轮两项 P1 的源码及锁已闭合。三构建 `-D warnings`、11 条 H1 准入锁的实际执行、生产 release 产物禁钩子检查仍待现有 CI，尤其真实 wire 校准和合法 PrePolicyRecords 正控须以运行结果验收。资源 `/3`、冷窗口字段、scrub、launcher、Python、ADR 继续留 S2；9011/G4、g77 真实消费、默认切换与整票关闭继续留原实施链。

### 本次封包完整文件清单

以下包含恢复的全部 S1 WIP 与本轮修改，按最终 `git status --porcelain --untracked-files=all` 生成。

- `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md`
- `.github/prod-banned-literals.txt`
- `.github/workflows/ci.yml`
- `.github/workflows/history36h.yml`
- `rust/src/bin/s_session_v2/cache.rs`
- `rust/src/bin/s_session_v3/admission.rs`
- `rust/src/bin/s_session_v3/admission_anchor.rs`
- `rust/src/bin/s_session_v3/admission_private/fd_wal.rs`
- `rust/src/bin/s_session_v3/admission_private/mod.rs`
- `rust/src/bin/s_session_v3/admission_provider.rs`
- `rust/src/bin/s_session_v3/audit_ledger.rs`
- `rust/src/bin/s_session_v3/audit_owner.rs`
- `rust/src/bin/s_session_v3/audit_owner/admission_tests.rs`
- `rust/src/bin/s_session_v3/audit_owner/cold_source.rs`
- `rust/src/bin/s_session_v3/audit_owner/history_clamp.rs`
- `rust/src/bin/s_session_v3/construction_set.rs`
- `rust/src/bin/s_session_v3/construction_state.rs`
- `rust/src/bin/s_session_v3/driver.rs`
- `rust/src/bin/s_session_v3/history.rs`
- `rust/src/bin/s_session_v3/prepared.rs`
- `rust/src/bin/s_session_v3/source_audit.rs`
- `rust/src/bin/s_session_v3/source_contributions.rs`
- `rust/src/bin/s_session_v3/source_facts.rs`
- `rust/src/bin/s_session_v3/source_receipt.rs`
- `rust/src/bin/s_session_v3/state.rs`
- `rust/src/bin/s_session_v3/writer_cli.rs`
- `rust/src/bin/s_structure_session.rs`


## S1c

> 状态：ready，可交主控封包。基线为 `sandcastle/issue-1481` / `07f531a1bde8883126f54bcca8df651a780b23ba`。本节覆盖 S1b 的封包结论；历史章节原样保留。第 1–9 项已形成源码候选、锁与 CI 接线；Rust 未验证，待 CI，不能据此宣布 PR 或整票验收通过。

### 原始证据与范围

全文读取 `/Volumes/AgentStorage/h1-managed43r/S1-PROMPT.md`、`S1B-PROMPT.md`、本报告及 `/Volumes/AgentStorage/h1-source-review44g/REPORT.md`；复核 43K、43I、43G/43G-2、39P 接口。S1c 已由 #1481 原票及本轮派单授权，无新票、无新增政策裁定。

五份原始日志位于 `/Volumes/AgentStorage/h1-managed43r/ci-07f531a1/`，已逐文件读取。以下位置按原文件物理行计；ANSI 或日志内字面的 `^[[...m` 不参与错误判断，Native job 的 JSON 编译消息另解码读取 `rendered`：

| 失败 | 原日志位置 | 根因 |
|---|---|---|
| 1，E0282 | `history36h-124763297258.log:497`；`ci-124763670473.log:820`；`ci-124763670596.log:596`、`:4745` | 工作闭包返回值经 `?` 可转为多种错误，`WorkError<E>` 无足够信息推定 `E` |
| 2，E0597 | history `:511`；rust-check `:834`；Native `:597`、`:4746` | 块尾 `AdmissionAccounting` 临时对象可能晚于外层 `RefMut` 析构 |
| 3，E0507 | history `:538`；rust-check `:861`；Native `:598`、`:4747` | `SourceInput::root(&self)` 试图移出非 Copy 的 `Built` |
| 4，Python 锁 | `ci-124763670712.log:324`–`:329`；`admission-124763669655.log:521`–`:526` | include 外部的 cfg 不在文件内调用前文，既有锁要求显式外层属性 |
| 5，P1-B | 44G 报告“台账旁挂文件让下一次准入恒拒” | 台账生成后被既有未知旁挂枚举拒绝，H1 来不及重建 |
| 6，P1-A | 44G 报告“H1 写连接整生涯禁用关闭检查点” | 成功交接后仍保留 `NO_CKPT_ON_CLOSE`，最后连接不清理 WAL/SHM |
| 7，输入域与时机 | 44G 两段 P2，及本轮主控追加裁决 | 非 WAL 合法库未经原 `open_db` 模式转换即进 Clamp；热日志必须先按既有 medium 判据处置 |
| 8，Ordinary 次序 | 44G “testonly 构建的 Ordinary 臂次序变了” | feature 级分派误将 Ordinary 前缀也缩成仅 schema 预检 |
| 9，触发分支 | 44G P3 | 本轮保留分支 push 以取返修 CI，合入前须移除 |

history36h 是精确 head 运行；四个 PR CI 日志显示 checkout 为 `0932709`，即把该 head 合入 `a2d1d63766f367d064510341bd27c20dc7ce8763` 的测试合并提交。没有把 PR 合并提交冒记为分支 head。本轮没有新 commit，没有新精确 head CI 结果。

仍不改资源 `/3`、scrub、launcher、Python 服务控制或 ADR；不运行 cargo/rustc、28/106/9011/S86，不操作 Git metadata，不推送、不评论、不关票。44G 已接受的真实 FD/VFS wire 及完整认证先于迁移结构继续保留。


### 第 1–6 项修复

下列 Rust 路径均相对 `rust/src/bin/`。

1. **E0282**：`s_session_v3/audit_owner.rs:637` 明定闭包错误参数为 `scratch::WorkError<String>`。冷核、捕获、贡献态构造的业务错误本来就是 `String`，只补足推断信息，仍经过原 `native::Failure::from` / `cold_failure`，不改拒绝行为。
2. **E0597**：`s_session_v3/admission.rs:423` 把 `first_read_deadline` 的结果先存局部变量再返回，让 `AdmissionAccounting` 临时值先于外层 `RefMut` 析构，错误和期限不变。
3. **E0507 与真实所有权**：`s_session_v3/audit_owner/cold_source.rs:15` 保存可 Copy 的 `construction_set::Declaration`，`:104` 按值构造 `Built`。`SourceInput::root(&self) -> Built` 的原调用者需要独立根描述，材料随后仍用于可变读取和差分，不能消费 `self`，也不宜改成跨可变调用持借用的接口。这里只复制固定的 digest/kind/extent，没有复制节点、集合或历史，没有 heap clone 或隐性大拷贝。
4. **Python cfg 锁**：`s_session_v3/audit_owner/admission_tests.rs:449` 等实际调用所在 helper/test 项加显式外层 `#[cfg(test)]`；三 feature 专用项用 `#[cfg(all(test, feature = "s_capture_native_testonly"))]`。不修改 Python 锁，也不靠文件内层属性或 include 外部 cfg 欺骗它。
5. **P1-B**：`s_session_v3/admission_provider.rs:118` 精确登记 `.audit-ledger.sqlite`、`.audit-ledger.sqlite-wal`、`.audit-ledger.sqlite-shm`、`.audit-ledger.sqlite-journal`，`:297` 接入原完整枚举，不用通配或跳过枚举。四名必须是普通单链接文件，仍经原 `Pinned` FD/路径身份核验及 `:191` 的 `st_blocks * 512` 原 store 账、原 `max_store_bytes` 和 64 GiB 帽；它们不决定内容库的 medium，不进入主库/WAL 内容权威。`:358` 删除前再核固定身份及在场集合；`audit_owner.rs:516`–`:531` 仅在已有 WriterScope 双锁内精确清理这四名，再释放旧缓存 FD，随后才 fresh 新台账。释放仅作用于独立缓存 pins，不动主库、WAL、控制文件；避免 unlink 后旧 inode 仍被 FD 持有而与新台账叠加占盘。真正未知名字继续拒绝，原 `known_control_lock_does_not_allow_unknown_sidecars` 保留。
6. **P1-A**：`s_session_v3/audit_owner.rs:708` 在 seed、资格移交及 Runtime adopt 成功后，把 `NO_CKPT_ON_CLOSE` 设回 false 并核返回值；拒绝路径继续保持 true，关闭连接不能顺手 checkpoint 源库。`audit_owner/admission_tests.rs:40` 移除常驻 `f.conn`，`:48` 改为作用域短连接。此前 `f.conn` 自己会建立并保留 `-shm`，与 `admission_provider.rs` 对源库既存 `-shm` 的生产拒绝判据冲突，不能代表“全连接退出后的重启”。现在正式 Driver 每次重新开门前检查 `-wal/-shm` 均不存在；新锁 `:880` 用真实子进程写入、正常退出，再用另一子进程重启同库，并核子进程实际执行 1 test 的收据。旧交接拒绝锁 `:331` 强化为外写连接先真正关闭，留下非空 WAL，再核拒绝后的主库/WAL 原字节。

第 5 项依据为 `.chanlun/review-results/issue1644-p6-audit-ledger-20261021/AMENDMENT.md:37` 的写者旁挂、`:39` 的同一存储帽、`:45` 及 `:48`–`:49` 的旧台账非权威/丢弃重建合同。未扩大可删除文件集合到这四名以外。

### 第 7–9 项修复与依据

**合法输入域。** 原 `s_structure_session.rs:346` 的 `open_db` 合同是把合法库转为 WAL，再迁移；Clamp 的 `s_session_v3/audit_owner/history_clamp.rs:115`–`:118` 则要求进入时已经是 WAL。S1b 缺了二者之间的合法转换，不能把 Clamp 的内部前置误写成“生产只收 WAL”。本轮在 `s_structure_session.rs:387` 增加 `prepare_h1_wal_handoff`：

- 全部 H1 历史认证仍先在原 FD/VFS、原 Job 内完成。坏历史、正确 epoch 的坏 PrePolicyRecords 等在改写源库之前拒绝；合法转换和迁移不能替代认证。
- 对非 WAL 库，一次性连接先取得 SQLite EXCLUSIVE 事务，在锁内核原内容完整 stamp，并检查需要补列时的既有迁移授权。空 COMMIT 保持排他锁，然后才执行旧合同的 WAL 模式转换。
- 转换连接继续持排他锁，建立空 WAL 并实际读库；`admission_provider.rs:388` 在锁内取得主库和 WAL 两份 opaque `Pinned` 完整物理戳。它只证明认证之后受控转换的物理连续性，不签发历史资格，不读取新历史或重认证。
- `audit_owner.rs:654` 关闭这条 `NO_CKPT_ON_CLOSE=true` 的一次性连接，再开普通正式连接。`:675` 在正式连接第一把 IMMEDIATE 锁内对两份 FD/path 全 stamp 复核；窗口内外部 COMMIT、主库替换或 WAL 替换均拒绝。通过后才在 `:679` 迁移，然后建立 Clamp、seed 和正常写者关闭语义。WriterScope 原 OS 双锁贯穿整个过程；正式连接不继承永久 EXCLUSIVE。

不能用“自身转换让 data_version 加一”抵消变化：首次 WAL snapshot 可能把转换与外部 COMMIT 合并成同一次缓存失效。轻原型已反证，最终实现不采用该办法，也不在释放锁后无条件重取新基线。固定依赖 `libsqlite3-sys 0.38.2` 的 `sqlite3.c:62849` 附近 `pagerBeginReadTransaction` 与 `:63788`、`:69964` 的关闭路径已核读。Python 3.14 / SQLite 3.53.4 临时原型分别覆盖 DELETE/TRUNCATE/PERSIST/OFF/MEMORY：空 COMMIT 不改源；锁内 token 采样、空 rollback、一次性连接关闭、普通连接首 IMMEDIATE 之后，主库/WAL 的 bytes 及 dev/ino/mode/uid/nlink/len/blocks/mtime/ctime 均不变；恢复普通关闭后 WAL/SHM 消失。这只是 SQLite 机制原型，不能代替本仓 Rust 运行证据。

**RollbackJournal 热介质。** 依据仍是 `s_session_v3/admission_provider.rs` 的 `MeasuredDomain::medium`，以及 `admission_private/mod.rs:644` 已有 `D2ProductionCapacityMissing` 判据。它在私有只读会话的分派入口、任何会触发 SQLite 热回滚的读写连接打开前拒绝；不先改主库再报身份变化，不新增 WAL-only 输入限制。新锁 `admission_tests.rs:1079` 由真实子进程在 DELETE 模式创建并遗留热日志，核 magic，父进程走正式 Driver；拒绝类别及主库/热日志原字节都入断言。

**Ordinary 次序。** `admission_private/mod.rs:433` 按 `RejectionPrefix::OrdinaryStartup` 选择原完整审计，保持先审计、后 `open_db` 迁移；Native/Recover 继续完整 H1 路径，不以 Ordinary 分支给失败兜底。测试 `admission_tests.rs:952` 只在 `s_capture_native_testonly` 下存在，正确 epoch 的坏旧形状库须先拒绝且不迁移；同库修好历史后须成功迁移，防止恒拒的空洞通过。非增量分支保留原顺序。

**分支触发。** `.github/workflows/history36h.yml:4` 的 `sandcastle/issue-1481` push 触发本轮保留，供主控封包后取返修 CI。合入前由主控移除这一临时分支触发；本轮没有把它删掉而阻断返修 CI。

### cfg 与同类问题自检

检查范围为 S1 原提交与 S1c 的全部 23 个 Rust 改动文件，先看 diff，再沿调用和返回类型追踪。检查了工作闭包的 `?`/错误类型及 trait 转换，所有新增 `RefCell`/事务的临时析构次序，从共享借用移出 owned 值的位置，短连接/游标的作用域，及新增导入、变量和 private 方法的实际调用。

| feature 组合 | 非 test | test |
|---|---|---|
| `s_session` | 没有 H1 生产冷门、`ColdRawState` 或增量资源字段引用；Ordinary 原审计顺序保留 | 不引用增量专用 caps helper 或新增 H1 测试项 |
| `s_session,s_incremental` | 真实 Native/H1 调用链、两种介质交接、错误类型、缓存资源及关闭门均逐项阅读；无生产测试钩子 | 冷核、Driver、7 条 provider 锁及 CLI 锁实际项/模块路径匹配；Ordinary 专用锁不编入 |
| `s_session,s_incremental,s_capture_native_testonly` | Native 与 Ordinary 按 prefix 分流，未用 feature 一刀切撤审计；所有注入仍在 test 门内 | Ordinary 迁移前拒绝锁存在，继续使用同一 cold/helper 类型与资源合同 |

上表每格又对 unix/非 unix 分支做源码核对。修正 `audit_owner::cold_source` 为 `s_incremental` 共享模块，避免非 unix 的历史折叠引用不存在模块；只供 unix owner 交接的 `ColdRawState`、Set/来源态重绑、Runtime 移交及冷重建方法加正确门控。`driver.rs:557` 的可变 Runtime 只在实际需要 `&mut` 的 unix 增量分支声明，冷调用与 recover 的非 unix 分支明确拒绝，避免缺项和 `unused_mut`。共享冷差分仍需要的来源方法不误删。未加泛化 `allow` 掩盖新问题。

限制：原有 admission/provider 等无门控使用 `std::os::unix`，本二进制基线就不是非 unix 可编目标。本轮核的是新增引用是否闭合，未宣称全仓非 unix 能编译；三档 feature 的 test/非 test `-D warnings` 均仍须以 CI 复验。静态非作者交叉复核覆盖 owner、provider、转换 helper 与锁，未发现剩余确定阻塞。

### 新锁、CI 与实际检查

CI H1 步骤为 `.github/workflows/ci.yml:428`，位于 `rust-test-s-session-extra-groups` 的 owned 组。原 11 条保留，新增 12 条，总共 23 条唯一 `#[test]`，全部无 ignore。每条都要求完整模块路径的 `test ... ok`，不以编译或零测试替代执行。

| 新锁，全名后缀 | 文件:行 | 所属执行配置 |
|---|---|---|
| `h1_admission_normal_exit_same_database_new_process_restarts` | `audit_owner/admission_tests.rs:880` | `s_session,s_incremental` / `h1_admission_` |
| `h1_admission_mainonly_nonwal_prepolicy_preserves_domain` | 同上 `:1018` | 同上 |
| `h1_admission_hot_rollback_journal_rejects_before_rw_open` | 同上 `:1079` | 同上 |
| `h1_admission_nonwal_conversion_handoff_foreign_commit_rejects` | 同上 `:1137` | 同上 |
| `h1_admission_ordinary_prepolicy_bad_history_rejected_before_migration` | 同上 `:952` | `s_session,s_incremental,s_capture_native_testonly` / 单独精确名 |
| `h1_ledger_cache_sidecars_are_pinned_and_physically_counted` | `admission_provider.rs:589` | `s_session,s_incremental` / `h1_ledger_cache_` |
| `h1_ledger_cache_registration_still_rejects_unknown_sidecars` | 同上 `:621` | 同上 |
| `h1_ledger_cache_substitutes_are_rejected` | 同上 `:639` | 同上 |
| `h1_ledger_cache_uses_original_store_cap_before_discard` | 同上 `:658` | 同上 |
| `h1_ledger_cache_replacement_or_new_member_is_rejected_before_discard` | 同上 `:680` | 同上 |
| `h1_ledger_cache_discard_releases_only_cache_pins_before_rebuild` | 同上 `:703` | 同上 |
| `h1_ledger_cache_conversion_stamp_rejects_changed_main_or_wal_frames` | 同上 `:725` | 同上 |

owner 锁模块前缀为 `v3::audit_owner::construction_tests::`；provider 为 `v3::admission_provider::control_lock_tests::`。CI 上传 `1481-admission.log`、`1481-ledger.log`、`1481-ordinary.log` 三份逐名日志。既有交接外写锁同步强化，不重复新造同义锁；既有 `h1_admission_driver_cold_recover_rebuilds_ledger_bitwise` 现在不再被台账未知旁挂或夹具常驻 SHM 挡在认证前，实际是否通过仍待 CI。

本机实际检查：

- `rustfmt --edition 2021 --check --config skip_children=true`，显式检查 S1 与 S1c 全部 23 个 Rust 文件，通过。`skip_children` 只限制隐式递归，列表内文件均检查。
- `git diff --check` 通过；ci/history36h 两个 workflow 的 PyYAML 解析通过；静态提取 23 条 test 核唯一、无 ignore、CI 过滤、模块路径及逐名核对清单一致，通过。
- `python3 -B -m unittest s_session.tests.test_tb01c_r4_anchor_lock -v`，3 tests 全部通过，含原失败锁；Python 锁源文件未改。
- `python3 .github/scripts/check-prod-no-test-hooks.py`，`violations=0`。
- `python3 .github/scripts/gen-prod-banned-literals.py .`，1636 条；随后 `python3 .github/scripts/gen-prod-banned-literals.py --check .` 一致。生成物与基线相同，因此不列入本轮改动文件。

本机没有运行 cargo/rustc，没有新的 Rust 编译、23 锁 PASS 或生产 release 产物检查收据。主控封包后须取新精确 head 的三档 `-D warnings`、23 条逐名 `... ok`、history36h 与生产 release 无钩子/禁串产物日志。资源 `/3`、S2、9011/G4、g77 消费及整票关闭义务不在本节结清。

### 本轮改动文件

相对精确 head `07f531a1` 的实际未提交改动如下，不重复列出已在基线中的 S1/S1b 文件：

- `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md`
- `.github/workflows/ci.yml`
- `rust/src/bin/s_session_v3/admission.rs`
- `rust/src/bin/s_session_v3/admission_private/mod.rs`
- `rust/src/bin/s_session_v3/admission_provider.rs`
- `rust/src/bin/s_session_v3/audit_owner.rs`
- `rust/src/bin/s_session_v3/audit_owner/admission_tests.rs`
- `rust/src/bin/s_session_v3/audit_owner/cold_source.rs`
- `rust/src/bin/s_session_v3/construction_set.rs`
- `rust/src/bin/s_session_v3/construction_state.rs`
- `rust/src/bin/s_session_v3/driver.rs`
- `rust/src/bin/s_session_v3/prepared.rs`
- `rust/src/bin/s_session_v3/source_contributions.rs`
- `rust/src/bin/s_session_v3/state.rs`
- `rust/src/bin/s_structure_session.rs`

## S1D

> 状态：ready，可交主控封包取 CI。精确基线为 `sandcastle/issue-1481` / `bda5958779569869b3624e7962a77666ee0f03bc`，已合并 H1 `822309387d` 与 E6 #1527 `784bc6d7`。本节为未提交返修的源码与静态检查收据；**Rust 未验证，待 CI**。不能据此宣布 10 项已实跑通过、44G 运行验收通过或整票完成。

### 证据、授权与范围

已全文读取 43K 实施简报、44G BLOCK 报告、S1/S1B/S1C 提示（含追加第 5–9 项）、S1D-MERGE-NOTES、本报告既有章节，以及 `/Volumes/AgentStorage/h1-recover45c/REPORT.md` 和**原始** `ci-41349093849-failed.log`。下表的日志位置均为该原始文件物理行，不是转述报告的行号。run `41349093849` 对应合并前精确 head `822309387d`；原日志 `:419` 为 61 passed / 10 failed / 0 ignored。因此这 10 项原始失败均先于 E6 合并存在；另行查出的 E6 共享核回归单列，不把它误记为原失败原因。

本轮沿 #1481 已有实施授权工作；未新增票、未改政策或 ADR。未运行 cargo/rustc、重型窗口或资源 `/3`；未做 scrub、launcher/5、Python 服务控制，未操作 Git metadata、暂存、提交或推送。所有行号除原日志位置外，均指本轮最终工作区；Rust 路径在下文简称中相对 `rust/src/bin/`。

### 授权假说：否定缺 scope，确认固定 SQL 与 SQLite 回调不匹配

`not authorized` 不是历史读取缺少生产授权对象，也不是漏登记旁挂文件。真实调用链为：

1. `s_session_v3/admission.rs:193` 的 `WriterScope::certify_cold` 借用原启动账，进入 `admission_private/mod.rs:618` 的私有认证，再调用 `:662` 的 `read_session`。
2. `admission_private/mod.rs:694` 注册原 FD/WAL VFS；`fd_wal.rs:1171` 给真实连接安装固定 authorizer。不是夹具绕开生产授权构造。
3. `audit_owner.rs:3346` 附近的 `CompleteCandidate::capture` 生成全表原件 SQL，交 `request_audit.rs:2653` 的 `ExactRows::read_optional_bounded_params_in_parent`；`:2661` 在 `tx.prepare(sql)` 首先失败，`:958` 的 `storage` 包装成“历史原件绑定读取失败”。
4. 失败 SQL 是 `SELECT rowid,* FROM main."s_delivery_refs" ORDER BY rowid`。该表只有 `INTEGER PRIMARY KEY`。SQLite 3.53.4 在通常的列读取之外，补发 `(SQLITE_READ=20, a="s_delivery_refs", b="", db="main", origin=None)`。`fd_wal.rs:678` 的既有策略只允许白名单业务表 **db=None** 的空列读取，故正确拒绝此事件；不是整份授权没有安装。

修法是 `audit_owner.rs:3352` 仅对这张表生成无 `main.` 的同义 SQL；`audit_owner/effects.rs:120` 同步其固定 SQL 身份。没有改动生产 `audit_sql_allowed`、空列白名单或授权范围。私有连接原来就禁止 TEMP/ATTACH；正式 owner 的 `audit_owner.rs:2914` `read_domain` 拒绝 TEMP 对象及附加库，因此不引入同名影子表解释空间。

`fd_wal.rs:873` 的既有授权锁增加实际 SQLite 正负控（`:925` 起）：原 main 限定 SQL 仍拒、TEMP 同名仍拒、未知整数主键表仍拒；去掉 TEMP 后，新 SQL 必须完整返回 `[[1,1],[4,4]]`。没有把拒绝断言改成成功或跳过登记。独立 Python SQLite 3.53.4 原型从生产 SCHEMA 抽取表，扫描历史读取及候选捕获的 44 条 SQL：旧 SQL 是唯一拒绝，新 SQL 全部接受。该原型是机制证据，不能代替本仓 bundled SQLite/Rust 锁的 CI 运行。

### 10 项失败逐项处理

下表“共同 SQL 根因”均指上一节已证明的 prepare/authorizer 首错及两处 SQL 修复；“冷缓存修复”见下一节。保留所有关键断言，尤其不以删去交接命中断言使测试通过。

| 项 / 测试后缀 | 原始日志位置与首个失败 | 证明的根因、修法及最终位置 | E6 合并关系 |
|---|---|---|---|
| 1 `control_lock_tests::h1_ledger_cache_substitutes_are_rejected` | `:79`–`:80`，`test_support.rs:87` unwrap，Identity/E0 | 原 `admission_provider.rs:639` 测试把源 DB 自身 hardlink 到缓存名，源 DB nlink=2；真实 loader 在 `admission_anchor.rs:1310` 正确拒绝，未到缓存测量。`:648` 改由独立 `cache-source` 建硬链接，显式核缓存 nlink=2、源 DB nlink=1，真实 `consumed` 先成功，再要求 measure 拒绝。Python 4 个缓存名×3 种替身共 12 例通过。 | 无；夹具在合并前已污染源库，未改生产身份门。 |
| 2 `h1_admission_cold_handoff_foreign_sqlite_commit_rejects` | `:91`–`:95`，原测试 `:380`，实际命中数 0 | `audit_owner.rs:635` 之前的候选捕获被共同 SQL 首错挡住，尚未返回冷认证；`COLD_AFTER_CERTIFICATION` 的 thread-local 注册/取出和 `#[cfg(test)]` 均存在，非漏接或跨线程。修共同 SQL 后，继续由 `audit_owner.rs:668` 在认证结束、正式开库前调用原 hook。`admission_tests.rs:333` 原测试体与命中断言未改。 | 无；原日志来自合并前 head。 |
| 3 `h1_admission_cold_prefixes_match_full_canonical` | `:112`–`:114`，原测试 `:151`，raw sequence 与 owner 前代 prefix 不一致 | 旧夹具 `f.open` 只构造组件 complete 状态，却跨 owner 复用旧 Runtime：旧生成器 raw 序列为 1，新 owner construction 前沿为 0；`generation.rs:550` 的一致性门正确拒绝。`admission_tests.rs:157` 改走正式 Driver，每轮 drop 后冷建配套 owner/Runtime，保留 canonical 字节、逐代 cuts、旧审计零调用断言；同时补齐正式冷续写所需缓存。 | 无；夹具与生产缓存缺口均在 H1 原实现中，未放宽 prefix 门。 |
| 4 `h1_admission_cold_real_wire_short_budget_rejects_before_write` | `:131`–`:133`，原测试 `:727`，历史原件 not authorized | 共同 SQL 首错发生在校准所需的合法 Driver 冷开，尚未验证目标 wire 短预算门；只修 SQL（`audit_owner.rs:3352`、`effects.rs:120`）。`admission_tests.rs:723` 原校准、真实冷读、预算错误及源字节断言保持。 | 无；固定 authorizer 相对 `822309387d` 的合并差异为空。 |
| 5 `h1_admission_driver_cold_recover_rebuilds_ledger_bitwise` | `:148`–`:150`，原测试 `:537`，历史原件 not authorized | 首次正式 Driver open 即被共同 SQL 拒绝；修 SQL，并在 `audit_owner.rs:635`、`prepared.rs:258`/`:308` 补配套冷生成器。`admission_tests.rs:537` 原锁不改，继续验证逐分区逐字节重建、下一次写与 recover。 | 无；原始首错在合并前。 |
| 6 `h1_admission_mainonly_nonwal_prepolicy_preserves_domain` | `:182`–`:184`，原测试 `:1060`，合法 MainOnly/DELETE 旧库 not authorized | 共同 SQL 首错在合法库完成只读认证前，尚未到 WAL 转换和迁移。修 SQL；`admission_tests.rs:1021` 的合法介质/模式正控保留，生产转换次序不改。 | 无；未缩为只接受 WAL。 |
| 7 `h1_admission_prepolicy_bad_history_rejected_before_migration` | `:228`–`:230`，原测试 `:855`，修复后的合法旧形状正控 not authorized | 坏历史的拒绝不构成此项失败，失败在同库修好后的成功正控；共同 SQL 首错使合法正控仍拒。修 SQL，保留 `admission_tests.rs:784` 坏历史写前拒与修好后迁移成功的全部断言。 | 无；不通过恒拒旧形状来避错。 |
| 8 `h1_admission_formal_accept_begin_prepare_recover_never_full_audit` | `:165`–`:167`，原测试 `:412`，StartupScopeMissing | 原夹具直接 `owner.startup`，未进入 WriterScope；`admission.rs:603` 的 current_scope 门正确报错。`admission_tests.rs:400` 用正式 Driver（`driver.rs:554` enter、`:598` startup），recover 用 `:822` 的正式 scoped retention 入口；finish/drop scope 后新 Driver 验身份、summary、canonical、cuts 与旧审计零调用。只改夹具，未放宽 scope。 | 无；原夹具漏 scope，在合并前已失败。 |
| 9 `h1_admission_nonwal_conversion_handoff_foreign_commit_rejects` | `:192`–`:193`，原测试 `:1191`，未命中转换交接窗口 | 同第 2 项，被共同 SQL 首错挡在转换之前。`audit_owner.rs:681` 的转换 hook 在原受控连接关闭后、正式写者开库前，注册、调用与 test cfg 均闭合。`admission_tests.rs:1140` 原测试体、命中要求及源字节拒绝断言完全保留。 | 无；不是 E6 导致 hook 门控失效。 |
| 10 `h1_admission_normal_exit_same_database_new_process_restarts` | `:196`–`:210`，父 `:931`，write 子进程首次 open 在 `:901` not authorized | 原日志证明子进程尚未完成首次 open，不能据此推断正常关闭仍有缺陷。修共同 SQL及正式冷续写缓存；`audit_owner.rs:742` 成功交接后恢复正常关闭的代码保持，拒绝路径保持 NO_CKPT。`admission_tests.rs:883` 原真实双进程锁不改，仍核 WAL/SHM 清理与同库新进程重启。 | 无；原始首错在合并前，重启的实际通过待 CI。 |

第 1、3、8 项夹具调整代表生产的理由分别是：保留真实 loader 并只污染被测缓存对象；采用真实 Driver 的成对 owner/Runtime 生涯；采用真实 Driver/scoped recover 的启动授权对象。没有另造宽松授权对象或测试专用准入捷径。

### 正式冷续写缓存连续性

修第 3 项不能只让夹具新建 Runtime。沿生产路径还发现：既有 `Runtime::adopt_cold_audit_runtime` 只移交 ledger，而新 Runtime 的 generator 从空开始；owner 已认证前沿为 F 时，首次续写仍会因 generator 前沿 0 与 F 不符而失败。

- `audit_owner.rs:635` 在原私有只读认证、原 Job、原 deadline/work 内恢复必要进程缓存；输入是同核已核的 raw 与最后一代 generated_state。`construction_state.rs:262`/`:303` 仅导出 binding/F，不签发可发布 token。
- `prepared.rs:258` 使用正式 `v2::protocol_identity`（`:272`）与既有 profile/rule/catalog 字段配方构造缓存绑定；没有把 session_generation 当成 meta key。`generation.rs:467` 恢复 DeltaLane、raw/effective SequenceMemo 和 producer；`:1609` 从已核序列根恢复 key/root。
- `s_session_v2/s_fact_cache.rs:401` 只解析已发布 **F** 内的赢家、来源坐标与 bar，R>F 的原始尾部留给下一次在线 Δ；`:443` 要求冷 life=0、实际 owner 初始 binding、前沿相等且无 poisoned/未授权历史。`content_producer.rs:1633` 恢复 sources/prepared keys，`:1676` 校验旧冷域后重绑。
- `audit_owner.rs:665` 保存冷 binding/F；两锁源连续性通过后，`:727`–`:737` 再核实际 owner binding/F 并 adopt。`prepared.rs:308` 拒绝覆盖运行中的 generator/binding/ledger；所有失败直接返回，无旧审计 fallback、无 reset 后重试、无 stale 门放宽。
- 新 rows、SequenceMemo 与 HashReader overlay 沿原 Native work 分配；生成节点只写内存 overlay，不写源 SQLite。`s_fact_cache.rs:568` 仅对已经 Native 的 rows 传当前 work；`:598` 缺活 work 硬拒，`:700` 附近的 reserve 与 `:738` 的 push 使用原生接口。旧 Plain 车道继续原分配合同。

内存边界照实：parser 的 IncrFactsState、producer 普通性能缓存沿既有 Plain 合同，本节不声称整个生成器堆都已纳入 Native 计量。上述恢复不是第二套认证判据，只消费同核已核的 F；没有因为严格判据失败切换到宽松路径。

### E6 合并专属回归

另行检查 S1D-MERGE-NOTES 提醒的 E6 N1 两条 S3 锁，发现共享 `audit_core` 把增量 `RejectionPrefix::None` 统一缩为 schema-only 后，N1 独立组件也失去了原完整 S3 审计及原 Job 的 nodes/records 计费。这是 **E6 合并交互产生的行为回归**，与上述合并前 10 项首错分开处理。

`admission_private/mod.rs:379` 增加 `NativeComponentAudit`，唯一构造方为 `admission_anchor/native_n1.rs:1802` 的独立组件审计入口。共享核 `mod.rs:442` 让 OrdinaryStartup 和该组件各自保留原完整审计；`:484` 保持组件原来 None 的无 startup-prefix 合同。H1 None/NativeStartup/Recover 仍走原 schema-prefix 加 H1 冷认证，失败不转组件审计。没有简单把所有 None 改回全审，因为正式 writer preflight 也使用 None。E6 的事务边界、预算对象、迁移核及两条测试断言均不回退。

### 44G 义务逐项状态

本表“源码保留/修复”只表示当前候选接线；涉及 Rust 的运行验收统一待新精确 head CI。

| 44G 项 | S1D 状态与证据 |
|---|---|
| 原 P1：冷认证真实 wire 入原准入账 | 源码保留：`admission.rs:193` → 私有 `certify_cold/read_session` 共用原 FD/VFS、SharedAccounting、首读 t0、Job 与收尾门；修 SQL 后不再提前止于 not authorized。原短 wire 锁仍要求冷读实际 >0、冷阶段超限、改写前拒绝。 |
| 原 P1：完整认证先于迁移 | 源码保留：候选与本轮 seed 都在 `audit_owner.rs:539` 私有只读认证中；认证结束后才转换/开正式连接、首 IMMEDIATE 内核 stamp 后迁移，第二锁拒外写。坏历史与合法正控锁均保留。 |
| P1-B：台账旁挂与重启 | S1c 精确登记的 `.audit-ledger.sqlite`、`-wal/-shm/-journal` 保留，仍钉身份、计物理字节及原 store/64 GiB 帽；锁内精确丢弃、释放旧缓存 pins 后重建，真正未知旁挂继续拒。S1D 只修 hardlink 夹具与首错，不扩集合。 |
| P1-A：成功后正常关闭、新进程可重启 | 成功 adopt 后 `audit_owner.rs:742` 恢复 NO_CKPT=false 并核返回值；拒绝路径不 checkpoint。短连接夹具及两真实进程退出/重启锁完整保留；原失败是首次 open 先被 SQL 拒，实际重启运行收据待 CI。 |
| P2：非 WAL 合法输入域、旧 open_db 合同 | 保留 S1c 的 MainOnly/非 WAL 先认证、锁内合法转换、严格转换 stamp 与首锁迁移，不采用 44G 中曾建议但用户已否决的 WAL-only 缩域。DELETE/TRUNCATE/PERSIST/OFF/MEMORY 正控不删。 |
| P2：RollbackJournal 热介质写前拒绝 | 保留 MeasuredDomain medium 的原拒门，在可能热回滚的读写连接之前拒；真实热日志子进程锁及主库/journal 原字节断言保留。 |
| P2：TestOnly Ordinary 先审后迁 | `admission_private/mod.rs:442` 独立 OrdinaryStartup 完整审计分支保持；三 feature 的正确 epoch 坏旧形状拒绝、修好后迁移正控保持。 |
| P3：临时分支 push 触发 | `history36h.yml` 未改，返修取 CI 的 `sandcastle/issue-1481` 触发保留；合入前由主控移除，本工蜂不自行推送或移除触发。 |
| P3：整读可执行文件仍受原首读期限 | 原 deadline 不重置，本轮 seed 也在同期限内。S2 cold_start_window 仍属后续阶段。 |
| 运行收据与增量独审 | 已做两名非作者对最终 diff 的只读交叉复核，覆盖编译可见性/cfg/错误转换/借用、授权、预算、副作用、E6 分流及锁。无剩余确定阻断；本机禁 Rust，23+4 条逐名 PASS 与三档编译/产物仍须主控封包后 CI 提供，不能记成已验收。 |
| 44G 明列 S2 与整票后续 | 资源 `/3`、scrub、launcher/5、Python 服务控制、ADR、#1641 签署值、g77、9011 双跑及默认切换均未做，未据 S1D 宣告结清。 |

### cfg、错误与生产构建自检

对最终 12 个 Rust 改动文件及调用点逐项源码阅读，覆盖闭包 `?` 的错误类型/From 转换、`RefCell` 与游标临时借用、ColdRawState 可见性/reexport、fresh 限制、绑定移交、Native 容器扩容及未使用项。没有加泛化 allow 来掩盖警告。

| feature 组合 | 非 test 源码检查 | test 源码检查 |
|---|---|---|
| `s_session` | 新冷 seed/rebind 均在 `all(s_incremental, unix)` 内，无增量专用类型泄漏；共享核非增量完整审计保持 | 新冷夹具只在既有增量测试模块内；不引用不存在的冷方法 |
| `s_session,s_incremental` | 冷调用、类型可见性、错误转换、借用及活 work 链闭合；生产只用 `advance_owner_bound_with_work`，无 hook 引用 | 旧包装 `advance_owner_bound` 仅 cfg(test)，授权新增实际 SQLite 正负控；两条交接 hook 在 test 门内 |
| `s_session,s_incremental,s_capture_native_testonly` | Ordinary 与 H1/组件按语义 prefix 分流，三者不互相兜底 | 原 Ordinary 单独锁保持，新增 test_prepare 调用有显式外层 test cfg，R4 锁未改 |

以上六格均为**源码检查**，不是编译收据；各格 `-D warnings` 的实际编译及 Rust 测试仍待 CI。新增 unix 门与调用点对应，未宣称基线二进制支持非 unix。

本机实际执行：

- `rustfmt --edition 2021 --check --config skip_children=true` 显式检查全部 12 个改动 Rust 文件，通过。
- `git diff --check` 通过；ci/history36h 的 YAML 解析及 H1 shell 的 `bash -n` 通过。
- 静态提取基线和候选的测试名/CI 逐名清单：原 23 条逐字保留、唯一、无 ignore；新增 4 个执行项的完整模块路径与源测试一致。两条交接测试及新进程重启测试的函数体与基线完全一致。
- `python3 -B -m unittest s_session.tests.test_tb01c_r4_anchor_lock -v`，3 tests 通过，锁源文件未改。
- `python3 -B .github/scripts/check-prod-no-test-hooks.py`，`violations=0`。
- `python3 -B .github/scripts/gen-prod-banned-literals.py .`，1638 条；随后 `--check .` 一致。相对精确基线新增 1 条测试专用文字，已同步 `.github/prod-banned-literals.txt`。该 count 与历史 S1c 节不同，历史章节不追改。
- 独立 Python 的 SQLite 44 SQL 原型及缓存替身 12 例通过；没有把原型文件入仓。

没有运行 cargo/rustc，没有新 CI 或生产 release 产物无钩子扫描收据。后续须在主控提交后的精确 head 上取三档 test/非 test `-D warnings`、全部具名测试 `... ok`、history36h 与生产 release 无钩子/禁串产物证据。

### ci.yml 锁清单差异

原 11 条及 S1c 新增 12 条合计 **23 条 H1 锁完全保留**，名称、过滤、无 ignore 和逐名 grep 要求不变。新增的是 4 条已有测试的显式执行，不新增同义测试名：

| 新增完整测试名 | CI 位置 / 日志 |
|---|---|
| `v3::admission_private::fd_wal::authorization_tests::full_audit_read_actions_do_not_enable_write_attach_temp_or_unknown_exits` | `ci.yml:441` 独立 FD/WAL 步骤，`1481-fd-wal.log` |
| `v3::admission_private::fd_wal::authorization_tests::count_scan_reads_are_limited_to_business_tables` | 同上，内部增加本次真实 SQLite 回归断言 |
| `v3::admission_anchor::native_n1::tests::native_n1_s3_real_audit_charges_original_job_and_keeps_s2_accept_version_gate` | `ci.yml:487`，`1481-e6.log` |
| `v3::admission_anchor::native_n1::tests::native_n1_vfs32f_real_s3_audit_seal_destroy_finish_stays_same_job` | 同上 |

四条均以 `s_session,s_incremental`、`--exact`、实际 `cargo test` 加逐名 `test ... ok` 核验，不以编译或零测试替代。现在合计 **27 条**具名锁；原 `1481-admission.log`、`1481-ledger.log`、`1481-ordinary.log` 上传保留，并新增 FD/WAL 与 E6 两份日志。`history36h.yml` 无本轮差异。

### 本轮实际改动文件

相对精确基线 `bda5958779569869b3624e7962a77666ee0f03bc`：

- `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md`
- `.github/prod-banned-literals.txt`
- `.github/workflows/ci.yml`
- `rust/src/bin/s_session_v2/content_producer.rs`
- `rust/src/bin/s_session_v2/s_fact_cache.rs`
- `rust/src/bin/s_session_v3/admission_anchor/native_n1.rs`
- `rust/src/bin/s_session_v3/admission_private/fd_wal.rs`
- `rust/src/bin/s_session_v3/admission_private/mod.rs`
- `rust/src/bin/s_session_v3/admission_provider.rs`
- `rust/src/bin/s_session_v3/audit_owner.rs`
- `rust/src/bin/s_session_v3/audit_owner/admission_tests.rs`
- `rust/src/bin/s_session_v3/audit_owner/effects.rs`
- `rust/src/bin/s_session_v3/construction_state.rs`
- `rust/src/bin/s_session_v3/generation.rs`
- `rust/src/bin/s_session_v3/prepared.rs`

## S1E

> 状态：ready，可交主控封包取 CI。基线为 `sandcastle/issue-1481` / `a0d85e33a105f40965e3a1438eb6a34959c967fa`，开工时工作区干净。本节只交付 6 个编译错误的最小可见性修复及静态核查；**Rust 未验证，待 CI**，不能据此宣布 H1 测试或三档 `-D warnings` 已通过。

### 原始日志与逐项修复

已读取 S1D 提示、所引 S1/S1B/S1C 的范围与禁止项、本报告 S1D，以及原始日志 `/Volumes/AgentStorage/h1-recover45c/ci-41355496782-s1d-failed.log` 全文。该日志属于 GitHub run `41355496782`、精确 head `a0d85e33`；物理行 `:1` 的命令是 `cargo test --release --locked --features s_session,s_incremental --bin s_structure_session h1_ --jobs 4 -- --test-threads 1 --nocapture`，`:147` 明报 6 个编译错误。测试尚未执行。下表日志行号均指这份原件；源码路径相对 `rust/src/bin/`，行号指本轮最终工作区。

| 错误 | 原日志物理行与原文 | 原调用点 | 修法与最终位置 |
|---|---|---|---|
| E0603，类型不可见 | `:69` ``error[E0603]: struct `RawPrefixIndex` is private``；`:70` 指向调用，`:76` 指向定义 | `s_session_v2/s_fact_cache.rs:412` | `s_session_v3/business_json.rs:429` 的 `RawPrefixIndex` 从 `pub(super)` 改为 `pub(crate)`。 |
| E0603，模块私有 | `:81` ``error[E0603]: module `history` is private``；`:82` 指向调用，`:85` 明报 `check_deadline` 未再导出，`:91` 指向模块定义 | `s_session_v2/s_fact_cache.rs:423` | `s_session_v3/history.rs:3724` 的既有函数改为 `pub(crate)`；`s_session_v3/mod.rs:75`–`:76` 按冷调用的相同 cfg 单独再导出；调用改用 `crate::v3::check_deadline`。`mod history` 仍为私有。 |
| E0624，构造函数不可见 | `:95` ``error[E0624]: associated function `new` is private``；`:96` 指向调用，`:101` 指向定义 | `s_session_v2/s_fact_cache.rs:412` | `s_session_v3/business_json.rs:520` 的 `new` 改为 `pub(crate)`。 |
| E0624，推进方法不可见 | `:106` ``error[E0624]: method `advance_measured` is private``；`:107` 指向调用，`:112` 指向定义 | `s_session_v2/s_fact_cache.rs:413` | `s_session_v3/business_json.rs:624` 的 `advance_measured` 改为 `pub(crate)`。 |
| E0624，第一次有序遍历不可见 | `:123` ``error[E0624]: method `ordered` is private``；`:124` 指向调用，`:129` 指向定义 | `s_session_v2/s_fact_cache.rs:417` | `s_session_v3/business_json.rs:589` 的 `ordered` 改为 `pub(crate)`，保留迭代器类型与函数体。 |
| E0624，第二次有序遍历不可见 | `:134` ``error[E0624]: method `ordered` is private``；`:135` 指向调用，`:140` 指向同一定义 | `s_session_v2/s_fact_cache.rs:422` | 复用上一项的同一修复，没有新增另一套遍历接口。 |

原日志中的 `pub(super)` 只把这些项开放给 v3 及其后代。`s_fact_cache` 虽然物理路径在 `s_session_v2/`，实际由 `s_structure_session.rs:72`–`:73` 挂在 crate 根，与 v3 是兄弟模块，因此不可访问。上述项原本都存在于失败 cfg 中；根因是可见范围，不是缺少 test 定义。

同时处理两处接口连带风险，未等 CI 再暴露下一轮错误：

- `business_json.rs:461` 的 `impl Deref for RawPrefixIndex` 以 `RawPrefixFacts` 为关联 `Target`。`:435` 同步将该类型改为 `pub(crate)`，避免更可见的索引泄漏更私有的关联类型。内部字段和方法可见性不变。
- `advance_measured` 返回 `RawPrefixChanges`，`:482` 同步将返回类型改为 `pub(crate)`，避免 `private_interfaces` 及调用端私有返回类型问题。其字段类型为公开 `CodecVec`、基础类型或原已 `pub(crate)` 的 `RawWinnerEdit` / `RawWinner`，没有下一层更私有类型。入参 `RawEventsView`、`CodecRead`、`Budget` 分别已在 `business_json.rs:182`、`audit_store.rs:163`、`audit_budget.rs:18` 声明 `pub(crate)`。

没有新增裸 `pub`、泛化 `allow`、宽松接口或实现副本。`pub(crate)` 不宣称 crate 外可达，不新增 `unreachable_pub` 风险。`RawPrefixFixture` 仍仅 `cfg(test)`；其既有 #1488 注释描述测试 wrapper 自身的局部边界，本轮生产索引的 crate 内提升以上表为准。

### cfg 与告警核对

以下是源码可达性核查，不是编译证明。`s_session` 通过 Cargo 的 required feature 进入 `s_structure_session`；`RawPrefixIndex`、关联类型和三个方法原先均无 test/feature cfg，提升可见性不改变其是否编入。

| feature 组合 | 非 test | test |
|---|---|---|
| `s_session` | 新冷调用 `s_fact_cache.rs:400` 的 `all(s_incremental, unix)` 为假，根再导出 `mod.rs:75` 同时消失，无未使用导入。索引方法仍有既有历史审计生产调用。 | 同左；既有 `RawPrefixFixture` 仍只在测试中编入，不依赖新增冷方法。 |
| `s_session,s_incremental` | Unix 下冷调用与单项再导出同时存在；所有目标及签名类型可达。生产调用不依赖 test wrapper，不需 dead-code allow。 | 同左，测试仅补原有 fixture/钩子，不改变修复项的定义可见性。 |
| `s_session,s_incremental,s_capture_native_testonly` | 新修复无 `s_capture_native_testonly` 分叉，与上一格同路；未改变 Ordinary 诊断分流。 | 同左；原测试 cfg 和显式 test 属性均保留。 |

非 Unix 时，冷方法及根再导出均被同一个 `all(s_incremental, unix)` 排除，没有新路径悬空；这只是门控配对，不声称基线整体已支持非 Unix。实施方另对三档 × test/非 test × Unix/非 Unix 共 12 个组合做源码门控配对。

索引方法的既有生产引用不靠测试保活：`history.rs:1085` 调 `RawPrefixIndex::new`，`:1562` 调 `advance_admitted`，后者在 `business_json.rs:622` 调 `advance_measured`；`history.rs:1602` 调 `ordered`。`check_deadline` 也有 `history.rs:925`、`:1217` 等既有无 cfg 调用。因此单档 `s_session` 不会因新增冷方法被排除而新增 unused/dead_code 项。六格实际 `-D warnings` 编译仍待 CI。

### 本轮实际验证与边界

- `rustfmt --edition 2021 --check rust/src/bin/s_session_v2/s_fact_cache.rs rust/src/bin/s_session_v3/business_json.rs rust/src/bin/s_session_v3/history.rs rust/src/bin/s_session_v3/mod.rs` 通过。
- Python 精确比对 HEAD 原件与工作区 4 个 Rust 文件，只允许表列的 7 处 `pub(super)` → `pub(crate)`、两行 cfg 再导出及一处调用路径替换，全部相等。函数体、字段、判据与资源合同没有改动。
- `python3 -B -m unittest s_session.tests.test_tb01c_r4_anchor_lock -v`，3 tests 通过。
- `python3 -B .github/scripts/check-prod-no-test-hooks.py`，`violations=0`。
- `python3 -B .github/scripts/gen-prod-banned-literals.py --check .`，禁串清单与源码一致，未改清单。
- `git diff --check` 通过。未运行 cargo/rustc、Rust 测试、远端重型窗口或新 CI；未 git add/commit/push，未写 Git metadata。资源 `/3`、scrub、launcher/5、Python 服务控制与 ADR 均未改。

### S1D 新增／修改跨模块调用穷举（独立只读核查）

核查输入为完整 `git diff bda5958779 a0d85e33 -- rust/`，12 个 Rust 文件、全部新增行和被改写调用的上下文；已全文读 S1D 提示、其引用的 S1/S1B/S1C 范围、本报告 S1D 节与 run `41355496782` 原始日志。本清单记录源码可达性，不等于 Rust 编译收据。下文路径相对 `rust/src/`；调用行号为精确 head `a0d85e33`，S1E 没有插入这些调用文件中的新行，因此仍适用于修补后的调用点。定义行号以当前 S1E 候选为准。

**模块树先核实。** `bin/s_structure_session.rs:30–37` 挂 `v2/v3`，`:60–77` 挂 `content_producer/s_fact_cache`；后两者虽然物理放在 `s_session_v2/`，实际是 crate 根的子模块。故其 `pub(super)` 表示 crate 内可达，而 `v3/business_json.rs` 的 `pub(super)` 只覆盖 `v3` 子树，不能供根同级模块使用。`v3/mod.rs:59` 的 `pub(super) mod business_json` 可由 crate 内路径到达；`:74` 的 `mod history` 只覆盖 v3 子树，S1E 保持该模块私有。

**公共依赖与 receiver 核查。** 各条中包括通过既有 import、父模块 `use super::*`、trait 方法或字段 receiver 发生的调用，没有仅按 `::` grep。`sc` 指 `newchan_rust::s_content`（公开模块）；`NodeStore` 是其 `sequence.rs:110` 的公开 trait，经 `s_content.rs:66–71` 重导出；`codec_work` 来自公开 `NodeSource`（`s_content.rs:471,487`），`WorkScope::check_current` 在 `s_content/capture_allocation/scratch.rs:461` 为公开方法。`CodecVec` 经 `s_content.rs:44` 公开重导出，其 `try_reserve_native/try_reserve_exact/is_native_bound/try_push_native/try_push` 均为公开方法（`capture_allocation/work.rs:203,381,486,497,547`）。这些库接口不需要本轮提升可见性。

| S1D 文件与调用位置 | 全部跨模块路径／receiver 与定义、可达性 | cfg 核查 |
|---|---|---|
| `bin/s_session_v2/content_producer.rs:1635–1638` | 参数 `NodeStore` 为上述公开库 trait；`crate::v3::business_json::RawEventsView` 在 `business_json.rs:182` 为 `pub(crate)`；`super::s_fact_cache::DeltaRawRow` 在 `s_fact_cache.rs:245` 为 `pub(super)`（真实父为 crate），字段 `ordinal/bar` 为 public；`crate::v3::audit_owner::RawAdvanceBinding` 经 `audit_owner.rs:139` `pub(crate) use`，原类型在 `construction_state.rs:22` 为 `pub(crate)`。 | `seed_cold_delta` 整项 `all(s_incremental,unix)`（`:1632`），类型与方法在该 cfg 下存在；`RawEventsView`、行类型、binding 类型本身无 test 限定。 |
| 同文件 `:1640,1649–1653,1681–1683` | binding `is_unbound_cold`、`is_initial` 在 `construction_state.rs:42,45` 为 `pub(crate)`；前者仅 `all(s_incremental,unix)`，与调用一致。`store.codec_work`、`work.check_current` 为上述公开库方法；`raw.get` 在 `business_json.rs:208` 为 public。 | 两个冷 seed/rebind 函数同为 `all(s_incremental,unix)`（`:1632,:1675`），test 与生产均可达。 |
| 同文件 `:1647–1666` | `Vec::with_capacity/push` 等是标准库公开接口；`RawRefSpec::from_event`、`fractal_unit_key`、`self.bind_source` 都在本文件定义，不是新增跨模块可见性边。`row.bar.source_index` 是公开 `Bar` 字段。 | 无新 import、无仅 test 的生产依赖。 |
| `bin/s_session_v2/s_fact_cache.rs:402,404–407,409,445–451` | `bj::RawEventsView` 同上；`RawAdvanceBinding` 及两方法同上；`crate::v3::audit_store::CodecRead` 在 `audit_store.rs:163` 为 `pub(crate)`，模块在 `v3/mod.rs:57` 为 `pub(crate)`；`crate::v3::audit_budget::Budget` 在 `audit_budget.rs:18` 为 `pub(crate)`，模块在 `v3/mod.rs:51` 为 `pub(crate)`；`Instant` 是既有 `std::time::Instant` import（文件 `:24`）。`raw.len/budget` 在 `business_json.rs:195,230` 为 public。 | `cold_seed/rebind_cold_owner` 整项 `all(s_incremental,unix)`（`:400,:442`）；引用的 `is_unbound_cold` 同门。 |
| 同文件 `:412–413,417,422` | `bj::RawPrefixIndex::new`、`advance_measured`、`ordered` 的 5 个失败点（类型、新建、推进、有序读取两次）全部原属 `pub(super)`，S1E 改为 `pub(crate)`（`business_json.rs:429,520,624,589`）。该 struct 的 `Deref::Target=RawPrefixFacts`（`:461–464`）及 `advance_measured -> RawPrefixChanges` 返回值（`:630`）也同步为 `pub(crate)`（`:435,:482`），消除伴生接口泄漏／private_interfaces 风险；字段继续原可见性。 | 类型、上述三方法不受 cfg(test) 门控，三 feature 组合均有定义；冷调用只在增量 unix 存在。旧 v3 生产 `RawPrefixIndex::new/ordered/advance_measured` 使用仍在，不新增 dead_code。 |
| 同文件 `:415–417,430–435` | `admission.codec_work` 沿 `CodecRead: NodeSource` 到公开 `NodeSource::codec_work`；rows `try_reserve_native/try_push_native` 为上述公开 `CodecVec` 方法。 | 调用与 cold_seed 同门。 |
| 同文件 `:423–427` | 原 `crate::v3::history::check_deadline` 穿过私有模块，是第 6 个错误；S1E 改 `crate::v3::check_deadline`。原实现 `history.rs:3724` 改 `pub(crate)`，`v3/mod.rs:75–76` 仅在 `all(s_incremental,unix)` 做 `pub(crate) use`；不复制实现、不公开整个模块。`budget.precheck` 属 `Budget` trait；`raw.get` 同上；`lane.state.append_with_changes` 和 `change.shared_available` 在 `theta_v0/parser/inclusion.rs:1040,1020` 为 public。`bar_of` 是本文件函数。 | 重导出门与唯一新增根路径调用完全相同，非增量不产生 unused_imports。原 v3 内部 `history::check_deadline` 路径保持存在，不受重导出门限制。 |
| 同文件 `:530–531,572,594` | 参数 `crate::v3::audit_owner::BoundRawAdvance` 在 `construction_state.rs:117` 为 `pub(crate)`，经 `audit_owner.rs:139` 重导出；`newchan_rust::s_content::capture_allocation::scratch::WorkScope` 为公开库类型。 | 新带 work 的正式推进方法无 cfg(test)，保留普通构建；旧无 work 包装 `advance_owner_bound` 新加 `#[cfg(test)]`（`:520`）。实际生产调用 `generation.rs:554` 已换到带 work 方法。 |
| 同文件 `:544–545,576,597,714–715,743–744` | `raw.get` 同上；rows `is_native_bound/try_reserve_native/try_reserve_exact/try_push_native/try_push` 均为公开库方法。其余新增 `advance_bound_checked/advance_bound_inner` 是同一 impl 内私有调用，`bound.ensure_current/invalidate_after_consumer_failure/raw` 等既有调用定义均 `pub(crate)`（`construction_state.rs:182,208,211`）。 | 所有新增 work 参数在普通／增量构建均有公开库类型；没有把某个 cfg(test) 接口带入生产。 |
| `bin/s_session_v3/admission_anchor/native_n1.rs:1802` | `super::super::admission_private::RejectionPrefix::NativeComponentAudit`：模块为 v3 私有子模块，调用方是 v3 后代；enum `admission_private/mod.rs:376` 为 `pub(super)`（v3 范围），变体 `:379` 随 enum 可见。`native_n1` 自身由 `admission_anchor.rs:24` `pub(crate) mod` 挂载。 | 变体没有 feature/test 限定，三档 test/非 test 均可解析；没有只在增量定义而普通调用的问题。 |
| `bin/s_session_v3/admission_private/mod.rs:442,484`（相关执行 `:446`） | `RejectionPrefix::{OrdinaryStartup,NativeComponentAudit,None}` 均为本模块 enum/变体，不是穿过另一个模块。S1D 改分派后执行的 `super::state::audit_with_budget` 在 `state.rs` 为既有 `pub(super)`，仍在 v3 后代内可达；本轮没有新增私有外穿路径。 | `matches` 的完整审计选择块限定 `s_incremental`（`:438`）；末尾 match 无该限制，变体在所有组合均存在。非增量既有完整审计路径与无 test 钩子特征不变。 |
| `bin/s_session_v3/admission_private/fd_wal.rs:925–984` | 通过既有父模块 imports `std::ffi::{c_void,c_int,c_char}`（`:11`）、`rusqlite::{ffi,Connection}`（`:8`）、`std::ptr`（`:16`）；新增 `ffi::{SQLITE_OK,SQLITE_DENY,sqlite3_set_authorizer}` 与 `ptr::null_mut` 全为依赖公开接口。`Connection::open_in_memory/execute_batch/handle/prepare`，`Statement::query_map`、`Row::get`、`rusqlite::Result`、iterator `collect` 同为公开依赖／标准库接口。`fixed_authorizer`、`bytes` 与 `audit_sql_allowed` 都在当前模块或其后代可见的父作用域，不新设模块穿透。 | 整个 `authorization_tests` 在 `#[cfg(test)]`（`:760`），生产完全不含新增回调和连接。测试模块沿 `use super::*` 复用 imports；未新增仅 test 使用的模块级 import，不引入生产 unused_imports。 |
| `bin/s_session_v3/admission_provider.rs:648–658` | `f.root.join` 用公开 `PathBuf/Path` 方法；`std::fs::{write,hard_link,metadata}` 全为标准库公开函数；metadata `.nlink` 沿文件 `:8` 的既有 `std::os::unix::fs::MetadataExt` 公开 trait。`f.consumed` 在同一测试模块 `:512`，`MeasuredDomain::measure` 在父模块 `:159` 为 `pub(super)`，后代测试可达。 | 整个 `control_lock_tests` 在 `#[cfg(test)]`（`:485`）；文件已有 unix 文件域依赖，本次未增加非 unix 构建承诺。三档测试均可达；生产不含新夹具。 |
| `bin/s_session_v3/audit_owner.rs:133–134` | 新 `pub(crate) use construction_state::ColdRawState`：私有 construction_state 模块里类型在 `construction_state.rs:259` 改为 `pub(crate)`；重导出的可见性与源类型相等。 | 源类型、源 impl、重导出均 `all(s_incremental,unix)`；非增量与非 unix 不产生空重导出或导入警告。 |
| 同文件 `:635–648,665,728,732–737` | `cold_runtime.seed_cold_generation` 与 `runtime.adopt_cold_audit_runtime` 在 `prepared.rs:258,308` 为 `pub(super)`（v3 范围）；`construction.generation_binding` 对冷态在 `construction_state.rs:262` 为 `pub(crate)`，实际 committed 态在 `:303` 为 `pub(super)`（audit_owner 范围），调用就在 audit_owner。`candidate.audited.raw.view` 通过 `state::AuditedState` 的 public raw 字段（`state.rs:46,48`）与 `business_json::RawEvents::view`（`:79`）；`cuts.last` 是公开 CodecVec/Deref，cut 的 roots 字段在 `history.rs:208–210` 公开，generated_state 的 `.view` 经 `audit_store::RefValue::view`（`:1159`）公开。`construction.borrow` 是标准库 RefCell 接口。 | 整体位于 `AuditConnection::open_rebuilt_history` 的 `all(s_incremental,unix)`，与被调两个 Runtime 方法、ColdRawState、binding 导出一致；非 test 真实调用存在。 |
| 同文件 `:3352–3381` | S1D 仅改固定 SQL prefix 的字符串选择与 `String::push_str` 标准库调用；没有新增业务模块路径。SQL 中 `main` 是 SQLite schema 名，不是 Rust 模块。 | 不新增 cfg 相关名字。 |
| `bin/s_session_v3/audit_owner/admission_tests.rs:164–190,408–447` | 新／改 receiver `driver.handle` 在 `driver.rs:636` 为 `pub(super)`（v3），`driver.h1_owner_for_test` 在 `driver.rs:515` 为 `pub(super)`；`state::full_audit_calls_for_test/traced` 在 `state.rs:154,2166` 为 `pub(super)`；`driver::recover_scoped_with_retention` 在 `driver.rs:822` 为 `pub(super)`；`super::super::admission_anchor::test_prepare` 在 `admission_anchor.rs:1641` 为 `pub(crate)`；`scope.finish_startup` 在 `admission.rs:449` 为 `pub(crate)`。owner identity/summary、fixture/full_canonical/context/h1_admission_driver/h1_admission_canonical 属既有生产方法或同 include 模块 helper；`Result::unwrap`、JSON 索引、`drop` 为依赖／标准库公开接口。 | include 的父 `construction_tests` 是 `all(test,s_incremental)`（`audit_owner.rs:5464`），include 自身另 `#[cfg(unix)]`（`:5471–5472`）；H1 owner helper 同为 `all(test,s_incremental,unix)`，full_audit counter 是 `all(test,s_incremental)`，traced/test_prepare 是 cfg(test)。第 2 个测试另有显式 `#[cfg(test)]`（`:398`）。普通 s_session 不载入；增量与 testonly 的测试均可达；所有生产格均不含新测试调用。 |
| `bin/s_session_v3/audit_owner/effects.rs:120–121` | 仅 `Self::Delivery` 固定 SQL 文本改动，没有新增 Rust 跨模块路径。 | 无新增 cfg 名字。 |
| `bin/s_session_v3/construction_state.rs:259–264,302–304` | `ColdRawState` 提升、其 `generation_binding` 返回 `RawAdvanceBinding`（同文件 `:22`，pub(crate)）与 usize，无更窄返回类型；`.facts.prefix_len` 定义 `business_json.rs:456` 为 `pub(super)`（v3 范围），此模块是 v3 后代，能到达。CommittedRawState 的同名方法仍限 audit_owner 范围，调用在父 audit_owner。 | 类型／冷态 impl／新 committed 方法均 `all(s_incremental,unix)`，prefix_len 本体不删，只通过既有 cfg_attr 抑制非测试未用告警；生产调用并未被 cfg 掉。 |
| `bin/s_session_v3/generation.rs:469–476,482–500` | 参数 `super::history::HashReader` 在 `history.rs:3181` 为 `pub(crate)`，该私有模块在 v3 内可见；`RawEventsView` 同上；`super::audit_owner::ColdRawState` 同重导出；`sc::RefView` 经 `s_content.rs:50` 公开重导出，`std::time::Instant` 公开，`Budget` 同上。冷 binding 方法同上；`s_fact_cache::DeltaLane::cold_seed/rows` 分别 `s_fact_cache.rs:401,490` 为 `pub(super)`（crate 范围）。`store.lookup_field_ref` 来自 `audit_store::CodecRead`（trait `:163`、方法 `:475`，HashReader 实现 `history.rs:3997`），经本文件既有 trait import 可达；返回 `RefValue::view` 为 public（`audit_store.rs:1159`）。`self.producer.seed_cold_delta` 为 `content_producer.rs:1633` 的 crate 可达方法。 | 新 seed 整项 `all(s_incremental,unix)`（`:466`），全部冷类型和被调方法同门。lookup trait/method 与 rows 不限 test。 |
| 同文件 `:508–519,554` | `RawAdvanceBinding` 同上；lane `.rebind_cold_owner` 在 `s_fact_cache.rs:443` 为 crate 可达；producer `.rebind_cold_delta` 在 `content_producer.rs:1676` 为 crate 可达。生产 `.advance_owner_bound_with_work` 在 `s_fact_cache.rs:528` 为 crate 可达，`store.codec_work` 为库 NodeSource 方法。`keys.as_slice` 在本文件 SequenceKeys 实现，不是跨模块边。 | rebind 冷方法整项 `all(s_incremental,unix)`；`:554` 普通推进方法无新增门，其所调带 work 方法与公开 WorkScope 类型也不限 feature/test。 |
| 同文件 `:1611–1628` | `HashReader`、`sc::RefView` 同上；`Kind::Array` 为公开 enum/变体（`s_content.rs:185`）；root `.kind/.extent/.digest` 在公开 RefView impl（`s_content/ref_owner.rs:226,232,238`）；`sc::path::array_sequence_refview` 在公开 path 模块 `path.rs:1166` 为 public；store `.array_cursor/.next_array_member` 来自 crate 可达 `CodecRead` trait（`audit_store.rs:408,418`）；reference `.view` 同上。`keys.reserve_admitted/splice_reserved/as_slice` 和 `codec_error` 为本文件实现／函数。 | `SequenceMemo::seed_cold` 整项 `all(s_incremental,unix)`（`:1608`），库方法与 trait 方法不受 test 限定。 |
| `bin/s_session_v3/prepared.rs:260–267,272–302` | `rusqlite::Transaction` 通过既有 import（文件 `:13`）为公开类型；`Path` 由父 import 为 std 公开类型；`history::HashReader`、business_json::RawEventsView、audit_owner::ColdRawState、sc::RefView、Instant、Budget 同上。`v2::protocol_identity` 在 `s_session_v2/mod.rs:2805` 为 `pub(super)`（crate 范围），从 v3 可达；`history::check_deadline` 在 v3 子树内本来可达，S1E 仅把函数提升为 crate；`budget.precheck` 为 Budget 方法；`meta_get_opt` 在 crate 根 `s_structure_session.rs:569` 为私有项，可供全部 crate 后代使用；`s_fact_cache::lane_key` 在 `s_fact_cache.rs:1246` 为 crate 可达；`GenerationRuntime::with_config/seed_cold` 在 `generation.rs:413,467` 为 v3 可达；`db.to_string_lossy/into_owned`、theta config Clone 是标准库／公开库接口。 | 新 Runtime seed 整项 `all(s_incremental,unix)`（`:257`），其所有冷项同门。v2 identity、lane_key 与 Runtime::with_config 无 test 门，不影响普通构建。 |
| 同文件 `:311–321` | 返回／参数 RawAdvanceBinding 同上；`cold.generator…rebind_cold_owner` 在 `generation.rs:506` 为 `pub(super)`（v3）；`install_audit_ledger` 是本 impl 方法，无新模块穿透。 | adopt 冷方法整项 `all(s_incremental,unix)`（`:307`），与调用侧 `audit_owner::open_rebuilt_history`、被调 generation rebind 同门。 |

标准库 `Option/Result/Vec/String`、迭代器的 `map/map_err/ok_or/unwrap/count/collect`、切片／索引／比较／Clone 等常规 receiver 在上表相关行一并按公开 trait／类型接口检查；它们不经过仓内私有模块。没有新增 `use` 漏列：S1D Rust diff 中唯一新增重导出是 `audit_owner.rs:134` 的 ColdRawState，上表已单列。

**六格源码结论。** `rust/Cargo.toml:84,87–88,108–111` 声明增量依赖 s_session、native_testonly 依赖增量，且此 bin 要求 s_session。在既有 Unix 平台前提下：

| feature | 非 test | test |
|---|---|---|
| `s_session` | 所有新冷 seed/rebind／ColdRawState／根 check_deadline 重导出都被同一增量 unix 门剔除；RawPrefixIndex 三方法及其返回／关联类型仍有旧 v3 使用；带 work 推进与其公开参数类型均存在。 | H1 admission_tests 被父增量门剔除；旧 advance_owner_bound 包装只在 test 编译且仍由既有测试使用。FD/WAL 与缓存替身新增测试是 cfg(test)，其依赖全为现有公开 API。 |
| `s_session,s_incremental` | 冷调用链从 open_rebuilt_history → prepared → generation → 根 s_fact_cache/content_producer 的每条可见性和 cfg 均配对；根 check_deadline 重导出有真实生产调用；无只在 test 定义的依赖。 | 六个原日志错误点经最小 crate 可见性／单函数重导出可达；H1 test helpers 的 test+增量+unix 门闭合。 |
| `s_session,s_incremental,s_capture_native_testonly` | native_testonly 不额外限制本轮暴露的类型／方法；等同上一格可达性，不新增诊断臂准入接口。 | H1 与 Ordinary 的测试定义保持各自原门；上述新增测试调用仍在 test 门内，test_prepare 显式外层 cfg 保留。 |

**告警风险复核。** S1E 当前 diff 仅使用 pub(crate)，没有新增裸 pub 接口，因此没有新 unreachable_pub 风险；RawPrefixIndex 的 Deref 关联 Target 与 advance_measured 返回类型同步为 pub(crate)，参数 RawEventsView/CodecRead/Budget 原本已是 pub(crate)，返回类型内的 CodecVec/RawWinnerEdit 均足够公开，字段保留原样。重导出 check_deadline 精确门控，不会在 s_session 单档生成 unused_imports；三方法原有 v3 使用持续存在，不因扩可见性制造 dead_code。没有新增 allow 规避告警。此结论是源码复核，`-D warnings`、E0446/private_interfaces 的实际编译裁定和 Rust 测试均待精确候选 CI；本核查没有调用 cargo/rustc，也没有改 Git metadata。

### S1E 实际改动文件

- `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md`
- `rust/src/bin/s_session_v2/s_fact_cache.rs`
- `rust/src/bin/s_session_v3/business_json.rs`
- `rust/src/bin/s_session_v3/history.rs`
- `rust/src/bin/s_session_v3/mod.rs`

## S1F

> 状态：ready，候选可交主控封包取 CI。基线为 `sandcastle/issue-1481` / `7332d8014a3c859f80ad0f58eaa285206b33288d`，开工工作区干净。以下是根因证明、源码修复及静态检查收据；**Rust 未验证，待 CI**。不把候选 ready 记作三项 Rust 测试已经转绿。

### 原日志与三项首错

已读取 S1D/S1E 提示及引用的范围与禁止项、本报告两节、43K 简报、44G BLOCK 和指定原始日志 `/Volumes/AgentStorage/h1-recover45c/ci-41356487293-s1e-failed.log`。下述日志位置均是该原件物理行。run `41356487293` 的命令在 `:1`，`:69` 编译完成，`:72` 开始 71 项；`:405` 是 `68 passed; 3 failed; 0 ignored`。原日志不含 RawCoords 内部字段 dump，下面的字段值由失败现场输入与构造/调用链推导，未冒充新增运行读数。

| 测试后缀 | 原日志与前后输出 | 证明的根因 | S1E / E6 关系 |
|---|---|---|---|
| `h1_admission_cold_prefixes_match_full_canonical` | `:103–106`，原测试 `:165` 的 handle 报 RawCoords 错。紧前 `:100` 是 `g=1 nodes=1848 object_effects=4 aux_effects=[0,0,2]`，`:101` 的 work 为 `live_peak_meta=(0,790048,96,192)`、`native_upper_live_peak=(3605204,4575603)`、`total_peak=5111707`；失败后 `:107–108` 已是下一测试的 g1/work。 | 第一代发布并冷开后，第二代首次续写需要 1 个旧赢家，RawCoords 却仍为空；严格门正确拒绝。 | S1D 新冷恢复漏 ContentBi。S1E 只处理可见性；E6 未改此缓存链。 |
| `h1_admission_driver_cold_recover_rebuilds_ledger_bitwise` | `:162–164`，原测试 `:496` 的 commit helper 报同错。紧前 `:155–160` 是该锁 g1/g2/g3，g3 为 `nodes=5564 object_effects=11 aux_effects=[3,7,3]`，work 为 `live_peak_meta=(0,2428576,96,192)`、`native_upper_live_peak=(7822547,12032480)`、`total_peak=13275044`；`:165–166` 已是下一锁输出。 | 三代后冷开成功，首次修订 n4 需要 3 个旧赢家，RawCoords 仍为空。另穷举发现同一失败锁后面的 R>F 夹具也另造零旧池合同，本轮同时修正，见预算节。 | 缓存缺项属 S1D；后续裸 context 夹具问题与第 3 项同族。均非 S1E/E6 引入。 |
| `h1_admission_formal_accept_begin_prepare_recover_never_full_audit` | `:179–181`，原测试 `:409` 的第一次 identity 报预算交接拒绝。`:165–176` 的 suffix/work 属前一外写锁，`:177` 已明确它通过；本失败锁尚未产生自己的首代 suffix。 | 正式 Driver 已有活父账，夹具却创建 `old_pool=0` 的独立新冻结域，预算门正确拒绝换域。 | S1D 将正式 Driver 与原裸 context 混用；S1E/E6 未改该合同及拒门。 |

### RawCoords 逐字段证明与修复

确切拒门为 `rust/src/bin/s_session_v2/content_bi.rs:430–436`，其中 `:430` 检查 `self.coords.len() != view.previous_len()`，`:436` 产生原错误。绑定分支在 `:420–423`。S1D 的 `generation.rs:467` 原 seed 只恢复 DeltaLane、raw/effective 两个 SequenceMemo 和 producer；`ContentBi::new` 在 `content_bi.rs:1126` 仍创建 `RawCoords::new`，没有任何冷恢复赋值。

| 字段 | S1D 冷恢复实际值 | 在线首次续写要求与 S1F 恢复值 |
|---|---|---|
| `coords` | `[]` | prefixes 的 F=1、N=1，为 `[10]`；driver 锁的 F=3、N=3，为 `[0,1,2]`。坐标分别来自 `audit_owner.rs:5796–5804` 的 b2 输入和本锁前三次独立源输入。 |
| `refs_leaves` | `[]` | 按同一赢家顺序取得 producer 已恢复的 `source_root(coord)`，长度 N，内容就是在线同源引用叶摘要。 |
| `text` | `SequenceRef::empty()`，count=0 | 在线同一个 `coord_text_leaf` 逐坐标生成叶，再以同一个规范 M 序列核构造，count=N，根为同一有序叶序列的规范根。 |
| `refs` | `SequenceRef::empty()`，count=0 | 将上述 `source_root` 有序叶交同一个规范 M 序列核，count=N，根与在线相同。 |
| `delta_binding` | `None` | 先存同核 life=0 冷 binding，再与 lane/producer 一起严格重绑为新实际 owner 的初始 binding；下一 bound 的 previous_binding 即该值。 |

实际触发首错的是 **`0 != N` 的 coords 长度**，不是门中每个子条件都失败。旧 `None + initial + 空 coords` 分支本来合法，另外三个长度比较相对空 coords 都是 `0==0`；不能把错误文字中的 binding/计数/根统称为三个独立实测失配。

修法位于 `content_bi.rs:1133` 的 `seed_cold_delta`：仅接受 fresh 缓存和未绑定冷域，消费已核 F 的赢家行，严格核升序，用在线相同的 `coord_text_leaf`、`source_root`、`sequence_rebuild_suffix` 恢复全部字段。`:1185` 的 `rebind_cold_delta` 核旧冷域、新实际 owner 初始域、原绑定及全部长度，再只替换 binding。`generation.rs:501–503` 将它接在 producer seed 后；`:523` 在 lane/producer 原严格交接后交接 bi。没有放宽原 sync_delta 门，没有 reset 后重试，也没有旧审计兜底。

冷 HashReader 的 overlay 仍可在 `audit_owner.rs:648` 原位置丢弃，不能仅凭根相同来断言节点可读，节点闭包证据如下：`content_bi.rs:1275` 每代无条件建立 `whole`，`:1690–1700` 无条件发布带 `&whole` 的 CC-056.version；`:1724–1730` 进入 build_object，`object_records.rs:797–798` 写入 input_refs/source_coords 内容边；`generation.rs:1243–1252`、`:1287–1298` 把该对象纳入 generated_state。`content_producer.rs:3199–3216` 的数组帧引用完整 M 根，因此末代已发布版本对象的闭包已含两棵完整序列树。`s_content/sequence.rs:1–8` 的唯一规范分解保证冷重建全部 M 节点也相同。空 F 的 count=0 根在 `sequence.rs:137–149` 不要求读取节点。

ContentBi 唯一跨代字段就是 coords，已全部恢复。其他生成缓存也已穷举：objects/witnesses/relations/observations 的序列按本代完整键 sync，可从 fresh 建立；event_nodes 是可重算 memo；fractal_histories 从真实 bars 建立；空 development_ranges 使本代 ranges 全脏并重建 batches。R>F 仍只 seed 已发布 F 的赢家，未发布尾部交下一在线 Δ；修订后 F 与 N 不同不改变此规则。pending 分支 `generation.rs:1127` 不提前返回，仍走 ContentBi 和版本对象闭包。原失败锁保留修订后再冷、R>F、epoch2 pending 续写再冷的全部对拍。

### 预算交接判据、生命周期与夹具修复

错误文本位于 `request_audit.rs:2578/2581`。真实链为 `audit_owner.rs:979 identity` → `:853 begin_in_request` → `:794 bind_history_parent` → `request_audit.rs:2571 bind_frozen_mapped` → `audit_owner/native.rs:603–650`。确切失败门在 `native.rs:637`，换域要求 `live <= old_pool`，并同时要求序号与活动 scope 合法；本轮不修改这些条件。

`driver.rs:466–470` 的正式 cold_context 冻结 `T=536870912, legacy=0, old_pool=0`，以 D₁ 的唯一 BootstrapGrant 出生并绑定实际父账 P。启动后的 Clamp/候选驻留 P。旧测试另调用 `audit_owner.rs:6685–6691 context()`，冻结独立 D₂，仍声明 `old_pool=0`、uncovered=0；P 的 live 至少有 `native.rs:538–556` 的 metadata_bytes，严格大于零。因此 D₂ 接管 P 时必触发 `live > 0`，不是生产交接拒门缺陷，也不是 recover scope 丢失。

`admission_tests.rs:405–409`、`:434–435` 改用无测试特权的生产 `Driver::identity()` / `state_summary()`。`driver.rs:325–337` 的前者沿当前父域调用真实 identity，后者执行真实 summary 并给结果补 policy_records。后续 `handle` 仍在 `driver.rs:675–678` 调生产 entry_old_pool_partition 并冻结逐项旧费，未绕过请求预算。正式 scoped recover、scope.finish_startup/drop、Committed、cuts、旧审计零调用及最终 canonical 断言均保留。

同一第 2 项失败锁在 R>F 构造处原来也将裸 `context()` 传给已冷建 Driver 的 owner，修首错后会遇到同类换域失败。本轮 `admission_tests.rs:576–595` 复用生产 `entry_old_pool_partition`（`:584`）、`freeze_manifest_workspace_partition`（`:588`）和原 keeper，再调用真实 `owner.admit/Begin`（`:594`），故仍在第六条真实原件已接纳、Begin 完成而未发布的状态停止。保留 raw_count=6、generation=5、recover 只认证 g1..g5、台账逐字节一致、epoch2 自动发布 g6 的所有断言。Driver 的 stop_after 字段私有且生产 open 无该参数；本轮未扩大测试字段可见性，也未添加新钩子。

### S1E / E6 与保持项

只读差异核查：`a0d85e33..7332d801` 对本轮三个 Rust 文件无差异；`822309387d..bda5958779` 对 native.rs、request_audit.rs、content_bi.rs 无差异；S1D 对 native.rs/request_audit.rs/driver.rs 也无差异。故三项不是 S1E 可见性或 E6 共享审计/迁移改动造成。S1D 漏了一份冷缓存，又让已有夹具的冷合同接管正式暖父账。

原严格 RawCoords 函数、全部生产预算拒门保持。已通过 68 项的断言不改；正常退出同库新进程重启、未知旁挂拒绝、MainOnly/非 WAL、坏旧史迁移前拒绝、热日志不改源、Ordinary 先审后迁及 E6 锁均未修改。ci.yml/history36h 的原逐名清单与触发不变，无新增或 ignore 测试。仍不做资源 `/3`、scrub、launcher/5、Python 服务控制、ADR、默认切换、重型窗口或整票关闭。

### cfg、告警与实际自检

| feature 组合 | 非 test | test |
|---|---|---|
| `s_session` | 两个新 ContentBi 方法与 generation 调用均由相同 `all(s_incremental,unix)` 排除，无新增 import 或类型泄漏。 | H1 admission_tests 由父增量 cfg 排除。 |
| `s_session,s_incremental` | Unix 下 seed/rebind 都有真实生产调用，NodeStore、DeltaRawRow、RawAdvanceBinding 等签名类型可达；无测试钩子依赖。 | H1 夹具经既有 test+增量+unix include 编入，调用原生产 Driver/owner/分区接口。 |
| `s_session,s_incremental,s_capture_native_testonly` | 新代码无 testonly 分叉，与上一格相同；Ordinary 诊断臂次序不改。 | 同上，原 test_prepare 外层 test 属性保留。 |

非 Unix 时两个方法及两处调用一并排除。不新增导入、公开类型、allow 或环境开关；两个新方法是根模块的 pub(super)，可供 v3 调用；SequenceRef 已 derive PartialEq/Eq；BiError 已有 From<CodecError> 和 From<ProducerError>；dyn NodeStore 可顺序重借用，生成器错误转换返回 String。六格 `-D warnings`、借用与错误类型仅作源码核查，实际编译待 CI。

已实际执行 R4 Python 锁 3 项通过、生产无测试钩子扫描 `violations=0`、禁串 `--check` 与源码一致、三个改动 Rust 文件 rustfmt check 和 `git diff --check`。严格 RawCoords 的整个 sync_delta 函数另与 HEAD 原件逐字比较，未变。非作者独审核对字段连续性、节点持久闭包、预算生命周期、cfg 和下游 pending/recover 路径。未运行 cargo/rustc、Rust 测试或新 CI；未 git add/commit/push，未写 Git metadata。

### S1F 实际改动文件

- `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md`
- `rust/src/bin/s_session_v2/content_bi.rs`
- `rust/src/bin/s_session_v3/audit_owner/admission_tests.rs`
- `rust/src/bin/s_session_v3/generation.rs`

## S1G

> 状态：ready，源码返修候选可交主控封包取 CI。基线为 `sandcastle/issue-1481` / `5a2068a76f82d936ed12da60dcd3c6b234149799`，开工工作区干净。以下区分原 CI 现场、源码调用链及尺寸估算；**Rust 未验证，待 CI**。未把静态检查或 release 已绿记作 debug 修复已通过。

### 原日志与故障区间

已读取 S1D/S1E/S1F 提示、所引 S1/S1B/S1C 范围与禁止项、本报告对应章节、43K 简报、44G BLOCK，以及 `/Volumes/AgentStorage/h1-recover45c/ci-41357971195-s1f-failed.log` 原件。日志属于 run `41357971195`，精确 head `5a2068a7`。以下日志行号为原件物理行，源码行号无特殊声明时为本轮最终工作区。

- `:1–2`：`cargo test --locked --features s_session,s_incremental --bin s_structure_session --jobs 4 -- --test-threads 1 --nocapture v3::driver::tests::audit_owner_incremental_normal_writes_full_oracle_and_suffix v3::source_audit::source_receipt::tests::p6_raw_receipt_expired_`。
- `:69–72`：`Finished test profile [unoptimized + debuginfo]`，开始 4 tests。
- `:73`：`owner actual suffix g=1 nodes=1845 object_effects=4 aux_effects=[0, 0, 2]`。
- `:75–76`：`thread 'v3::driver::tests::audit_owner_incremental_normal_writes_full_oracle_and_suffix' (5118) has overflowed its stack`；`fatal runtime error: stack overflow, aborting`。
- `:77–82`：SIGABRT、exit 101；没有随后的 `NATIVE_CODEC_WORK`，没有测试成功收据。

`audit_owner.rs:3770` 输出 suffix，`:3776–3798` 立即调用 `append_cut`，`:3801` 才打印 `NATIVE_CODEC_WORK`。因此不是后面的 `same()`/完整 oracle 或测试 reopen 发生溢出；故障落在两条日志之间的在线后缀调用内。此前 H1 release、SQLite COMMIT 锁已通过是主控提供的同一 run 状态，本节没有自行运行或重新签署它们。

### 根因：共享在线验证器被塞入冷状态及冷分支 debug 栈槽

对照 `git diff b0db7a8d HEAD -- rust/src/bin/s_session_v3/state.rs`，新增点在 S1 接线的 `HistoricalSourceFold` 与其 `verify`，不是 S1F 的 RawCoords 规则。精确旧 head 的 `state.rs:1251–1258` 为共同 fold 新增 `ledger`、`cold_source: Option<NativeSourceState>`、`cold_raw`、`cold_root`；`:1276–1336` 又把整个冷台账重建块放入共同 `verify`。在线 `append_owner_cut` 仍在旧 `:3732–3757` 构造该结构，虽然这四个字段给的是 None/空索引，类型尺寸和 debug 函数的临时槽并不因此消失。

suffix 后的有界调用链（旧 head 行号）为：

1. `audit_owner.rs:3776` 的 `append_cut` 闭包 → `state.rs:3688` `AuditedState::append_owner_cut`，按值接管已核历史，构造共同 `sources`。
2. `state.rs:3758` → `history.rs:8491` `append_owner_cut` → `history.rs:1189` `audit_one_published_cut` → `history.rs:1863` 动态调用 `CutSourceVerifier::verify`。
3. 旧 `state.rs:1263` `HistoricalSourceFold::verify`：在线的 `ledger=None`，不执行冷重建，但该函数体新增的 `source`、`Result<(Counts, NativeSourceState, SourceExports), String>`、`RawReceipt`、`GenerationDelta` 等按值局部/返回槽属于同一个 debug 帧。
4. 旧 `state.rs:1337` → `audit_one_cut_sources`；在线通过 `OwnerSourceCheck` 取得 counts，继续既有 raw/目录核验。该链没有回到 cold seed、Driver open 或自身 append 的调用。

尺寸证据来自实际类型定义：`source_contributions.rs:493–543` 的 `NativeSourceState` 内嵌 **23 个 Map、11 个 Tree、3 个 Order**；Map/Tree 包装 `NativeTree`/`NativeSet`，后者包含 BTreeMap、high_water 以及带 NativeReservation 的 CodecVec（`source_contributions.rs:132/239`、`capture_allocation/native_tree.rs:6/181`、`capture_allocation/work.rs:168`、`scratch.rs:30`）。这是容器外壳本身，不是把堆中的全部历史计成栈。

本机 Python 按源码逐字段计数，并用明确的 64 位常见布局模型计算：CodecVec≈96 B，NativeTree≈128 B，Map/Tree≈136 B，Order≈160 B；连同 Built、binding、TextValue、4 个 Option<SequenceRef>、其余标量及 SourceWork，得到 **NativeSourceState≈5672 B**。仅 `Option<NativeSourceState>` 这个 None 槽也必须容纳完整载荷；旧 verify 里的 source/tuple Result 至少又各容纳一份同量级载荷，另有冷收据和 delta 等槽。具体 LLVM 是否合并某些移动槽、全部调用帧的机器字节数和撞 guard 的指令，本机禁 Rust，均未实测；5672 是布局估算，不冒充 `size_of` 或反汇编收据。

这解释的是**有界调用链的固定 debug 帧膨胀**，不是 1845 个节点形成 1845 层递归。新增冷 `audit_source_step` 的 phases 顺序执行，SourceInput 转发显式调用 SourceMaterial 固有方法，未发现新增自调用或互调环。在线现场 `ledger=None` 更明确排除了实际执行冷来源步、冷 seed 递归重入的说法。S1D/S1F 的 seed 位于 `audit_owner.rs:635` 的私有冷入口，此时尚未重新进入它；不得把它写成此次动态执行的首错。原调用链的树/codec 遍历未改，其有界深度上叠加的冷壳与冷临时帧才是本分支新增的栈负担。

main/旧头通过该锁是任务给定的对照证据；本轮进一步以 git 对象核对，当前在线 fold 的字段列表恢复到 `b0db7a8d` 的同一列表。没有证据支持“main 自己失败、与本分支无关”，不据此转 blocked。最终消除溢出的运行证据仍须新精确 head debug CI。

### 修法与语义保持

仅改 `rust/src/bin/s_session_v3/state.rs`：

- `:1242` 的 `HistoricalSourceFold` 恢复只包含共同来源核所需字段，在线 `:3782–3800` 不再构造四项冷字段。
- `:1256` 新增私有 `ColdHistoricalSourceFold<'borrow,'audit,'ledger>`，冷状态只在 `:1935` 冷入口构造。它**借用**共同 fold 与原 ledger/parent，不按值复制共同核，不新增堆分配或 Native 许可。借用避免把冷状态搬进在线局部量；没有引入未计量 Box。
- `:1310` 的在线 trait 实现只调用 `require_next_cut` 与 `verify_sources`；冷大对象和冷台账块留在 `:1325` 的另一份适配器实现，在线不可能调用它。两实现共用 `:1269` 连续代号判据与 `:1276` 来源验证核，没有两档判据、失败改道或宽松重判。
- `:1964–1973` 仍先导出 raw facts，再移交 cold source；显式 drop 冷适配器后结束共同 fold 的借用，保持冷生成器 seed/rebind 的原接续。冷台账核验 → 来源核验 → deadline → 更新 generation/previous 的次序不变。

实际执行 Python 文本对拍：冷台账整个块在 `self.sources.raw/previous/deadline` 接收者归一化后，与 HEAD 原件逐字相同；共同来源调用、deadline、前沿更新块逐字相同。既有故障测试就是正确的生产调用栈回归锁，未新增浅层替代测试，未删断言。driver.rs、history_clamp_tests.rs、admission_tests.rs、ci.yml、history36h.yml 均与 HEAD 逐字相同。没有 workflow、栈大小、线程、profile、ignore 或跳过测试的改动。

### H1 release 已绿路径的 debug 同类风险排查

按实际调用者分组检查，release 结果不能证明 debug 栈足够。下表名字以源文件为准，均保留原锁，不修改 CI 配置。

| 路径 / 文件 | 已检查的同类测试 | 结论 |
|---|---|---|
| 在线 `publish → append_owner_cut → verify`；`audit_owner/history_clamp_tests.rs:24/239/254/278/373/460/591/798` | `h1_real_own_accept_begin_publish_continues_same_life`、`h1_own_checkpoint_and_wal_reset_preserve_full_history`、`h1_partial_checkpoint_waits_for_reader_then_resets`、`h1_zero_autocheckpoint_keeps_main_and_wal_round`、`h1_certified_online_consumes_source_receipt_and_matches_cold`、`h1_certified_open_empty_and_reopen_from_originals`、`h1_control_parent_rebind_precedes_warm_request_observation`、`h1_qualified_online_consumes_real_ledger_and_cold_rebuild_matches` | 与故障 Native 锁共用同一在线冷壳/冷帧，受本轮修复覆盖；需要 debug 运行收据。 |
| 先成功写、再做资格负例；同文件 `:899/991/1064/1128/1213` | `h1_qualified_cold_rejects_identity_rule_root_and_unit_mismatch`、`h1_qualified_ledger_commit_failure_and_external_identity_write_are_stale`、`h1_qualified_stage_rejects_externally_rewritten_fold_digest_or_rule_revision`、`h1_qualified_transition_short_budget_rejects_before_write_and_releases`、`h1_roots_json_matches_real_cert_roots` | 前置成功写同样承担原在线帧，修复覆盖；不能因后续测试目的是负例而漏查。 |
| 正式冷后续写/修订/recover；`audit_owner/admission_tests.rs:157/274/299/400/534/899` | `h1_admission_cold_prefixes_match_full_canonical`、`h1_admission_online_own_commits_match_full_canonical`、`h1_admission_foreign_sqlite_changes_retire_writer`、`h1_admission_formal_accept_begin_prepare_recover_never_full_audit`、`h1_admission_driver_cold_recover_rebuilds_ledger_bitwise`、`h1_admission_normal_exit_same_database_new_process_restarts` | 成功在线写由同一修复覆盖；其中冷折叠实际执行冷来源步，见下一行，不能只复跑 Native 一条就声称所有 debug 安全。 |
| 正式只读冷认证；上述 admission 锁及同文件 `:333/663/739/800/1037/1156` 的 handoff、node/record、wire、prepolicy、non-WAL 锁 | `h1_admission_cold_handoff_foreign_sqlite_commit_rejects`、`h1_admission_driver_cold_preserves_original_node_record_gates`、`h1_admission_cold_real_wire_short_budget_rejects_before_write`、`h1_admission_prepolicy_bad_history_rejected_before_migration`、`h1_admission_mainonly_nonwal_prepolicy_preserves_domain`、`h1_admission_nonwal_conversion_handoff_foreign_commit_rejects` | `cold_captured → fold_certified_history → ColdHistoricalSourceFold::verify → audit_source_step` 有界，冷 NativeSourceState 的按值返回仍有固定大槽，未改其语义/资源合同。它现在只在真实冷步骤存在，不再压到在线栈；这些锁仍须 debug `h1_` 验证，不宣称不存在剩余栈风险。 |
| 低层冷组件；`state.rs:4138/4161/4184` 与 history_clamp_tests 的 certified/qualified open | 三条 `h1_cert_*`，缺/坏 qualification、identity/rule/root/unit、deadline/scratch 等组件锁 | ledger=None，不执行冷来源步；共用代号和来源判据保持。预算/身份/隔离等在 append 前拒绝的 H1 锁没有进入事故帧；原断言全部保留。 |

另检查 `audit_owner.rs` 的 `h1_request_closure_real_owner_multicut_reuses_resolved_roots`：包装真实 multicut writer/rebuild，也消费相同后缀核，覆盖于本轮修复。原 Native 锁后面的 `p6_raw_receipt_expired_*` 不构造共同历史 fold，未发现本次引入的冷槽；原 run 因进程 abort 无法替它们签 PASS。

建议主控在新精确 head 先原样执行现有 Native debug step，再以相同 features/debug、`h1_ -- --test-threads 1 --nocapture` 运行 H1 靶向集合，尤其包含冷恢复逐字节锁；这里仅列待 CI 动作，没有本机偷跑或改 workflow。

### cfg、告警与实际自检

| 组合 | 非 test 源码穷举 | test 源码穷举 |
|---|---|---|
| `s_session` | 两个 fold、两个 impl、共同方法与所有调用都在原 `s_incremental` 门内，整体排除；未引入 import、dead_code 或缺失类型引用。 | 原 Ordinary/测试核及断言未改；无新增 test 专用引用。 |
| `s_session,s_incremental` | 冷 wrapper 由 cold_captured 构造且交 trait 对象；在线 wrapper 由 append 构造；两个共同方法均实际调用。三种生命周期分别绑定共同核借用、capture/raw、ledger，不要求互相延长。 | 同一生产实现，所有故障/H1 锁原样保留，无新钩子。 |
| `s_session,s_incremental,s_capture_native_testonly` | 与上一格同核；Ordinary 诊断分流、先审后迁、Native 准入不变。 | 原 testonly 锁原样保留，无新的 feature 依赖。 |

非 unix 构建保留原组件冷入口，wrapper 本身不引用 unix-only API；cfg 未额外收窄。`_auxiliary` 显式标记在线不用的 trait 参数，冷实现仍实际用 auxiliary；无新增 import、allow、错误转换或外部可见性。冷适配器析构后才能释放/消费共同核，Native 内容、许可与父账身份均未复制。六格 `-D warnings` 是源码核对，**不是已编译通过**。

本机实际检查：

- `rustfmt --edition 2021 --check --config skip_children=true rust/src/bin/s_session_v3/state.rs` 通过；`git diff --check` 通过。
- `python3 -B -m unittest s_session.tests.test_tb01c_r4_anchor_lock -v`：3 tests 通过，锁文件未改。
- `python3 -B .github/scripts/check-prod-no-test-hooks.py`：`violations=0`。
- `python3 -B .github/scripts/gen-prod-banned-literals.py --check .`：禁串与源码一致，无需修改禁串表。
- Python 源码对拍通过：冷块/共同块逐字一致、在线无冷字段/冷调用、上述测试与 workflow 未变；布局模型与字段数量输出见根因节。
- 非作者独立代理只读审查最终 diff，核对借用、生命周期、cfg、判据顺序和同类 H1 调用者；按其意见恢复 raw facts 在 cold source 之前的提取次序。

本机未运行 cargo/rustc、Rust 测试、重型窗口或生产产物扫描；未 git add/commit/push、未写 Git metadata。资源 `/3`、scrub、launcher/5、Python 服务控制、ADR、E6、FU16、输入域和写前拒绝门均未改。

### S1G 实际改动文件

- `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md`
- `rust/src/bin/s_session_v3/state.rs`


## S1H

> 状态：ready，可交主控封包取 CI。开工分支 `sandcastle/issue-1481`，精确 HEAD `8ec1995667dee36bfb9ba8b0813e9b577ad93678`，工作区干净。本节记录 run `41357975939` 两处回归的返修与静态核查；**Rust 未验证，待 CI**，不代表编译、具名 Rust 锁或整票验收通过。

### 原日志、根因与修法

已读取 S1D 提示及其引用的范围、S1E/S1F/S1G 提示、本报告 S1D–S1G，以及 `/Volumes/AgentStorage/h1-recover45c/ci-41357975939/` 下五份原始日志。run 的精确 head 为 `5a2068a7`；S1G 只改 state.rs，没有触及本节两处事故代码。以下日志行号为原件物理行，源码行号为当前工作区。

| 原日志 | 原文位置与结果 | 根因与修法 |
|---|---|---|
| `125002119534.log`，rust-check | `:764`/`:789` 为 `cargo check --all-targets --features s_session`；`:801` 为 ``error: function `migrate_policy_records_in_transaction` is never used``；`:802` 指向 `s_structure_session.rs:501:4`；`:807` 明报 `-D dead-code` 来自 `-D warnings`。 | S1D-M 保留了无 cfg 的兼容薄包装，但唯一调用者 `audit_owner.rs:696` 在 `:477` 的 `all(s_incremental, unix)` 内。按 Migrate Callers Then Delete，将该调用直接改为 `crate::migrate_in(migration.tx())?`，删除根文件原 `:499` 起六行包装和注释。 |
| `125002119586.log`，rust-test-default | `:4195`–`:4196`：`no_stray_bsp_binding_rewrites_outside_registry` 在 bsp.rs:834 panic，命中 `bin/s_session_v2/content_producer.rs:3411`；`:4202` 为 2996 passed / 1 failed。 | `raw_bar_at` 是既有原始 Bar 精确坐标查询，不是新增 BSP 绑定规则。S1D 在它之前净增 62 行冷 seed/rebind，令 main 的 `:3349` 漂到 `:3411`，而 file:line 白名单仍为 3349。修法见下文。 |
| `125002119568.log`，backtest-bin | `:4248`–`:4249` 同一锁、同一命中；`:4255` 为 3007 passed / 1 failed。 | 同一白名单漂移，无独立的 backtest 缺陷。 |
| `125002119152.log`，#1488 Owned | `:5196`–`:5197` 同一锁、同一命中；`:5204` 为 0 passed / 1 failed / 3624 filtered。 | 同一源码扫描失败，不是 Owned 核行为失败。 |
| `125002119491.log`，#1488 Plain | `:5069`–`:5070` 同一锁、同一命中；`:5077` 为 0 passed / 1 failed / 3594 filtered。 | 同一源码扫描失败，不是 Plain 核行为失败。 |

迁移核 `rust/src/bin/s_structure_session.rs:485` 的 `migrate_in` 整个函数体与 HEAD 逐字相同，`:487` 的 `require_policy_records_migration()` 授权门完整保留。H1 仍先核源连续性，再在已持 IMMEDIATE 事务内调用共用核，随后原位 commit。旧入口 `s_structure_session.rs:478` 与 N1 `s_session_v3/admission_anchor/native_n1.rs:1842` 原已使用同一核。没有加 allow/expect、放宽授权、复制迁移判据或改变事务边界；旧包装 Rust 定义及调用已归零。

BSP 返修位于 `rust/src/bin/s_session_v2/content_producer.rs:3404` 的 `raw_bar_at`：`:3408` 保留 `partition_point(|bar| bar.source_index < coord)`，`:3409` 改为 `match raw_bars.get(index)`，`:3410` 仅精确坐标相等才返回原引用，否则仍报 `MissingSource(coord as i64)`。它的调用者 `:1499` 和 `:2224` 消费原始 Bar/OHLC，没有访问 BspPoint、买卖位或绑定规则。保留二分复杂度、缺失错误、越界错误及重复坐标取第一条的行为，不改用语义不同的 BSP API。

`rust/src/theta_v0/classifier/bsp.rs:675` 接缝仅移除已不再命中的旧 `content_producer.rs:3349` 白名单及其 13 行历史说明；没有新增或重锚豁免。`:787` 起的原 grep 表达式、自身排除、命中集与 `:821`/`:827` 的 stray/stale 两个断言与 HEAD 逐字相同，锁判据未改。仅改写查询却保留旧条目会触发 stale，因此必须同时删除消失项。精确原 grep 原型修前为 hits=24、whitelist=24、stray=3411/stale=3349；修后为 23/23，stray/stale 均空。

已排除日志噪声：Owned 的四条 fixed_radix 测试在 `:864/:865/:867/:869`、Plain 在 `:863/:865/:867/:869` 均明确 `... ok`，panic 输出来自测试主动捕获的 panic 合同，不是失败。Owned `:355/:670/:674`、Plain `:384/:679/:683` 的 `::error::Missing ...` 都是 shell `echo` 源文本，不是实际缺失报告。

### cfg 与 dead_code/unused 同类穷举

输入为完整 `git diff 784bc6d7 HEAD -- rust/`，共 31 文件、4214 新增行，全部在 `rust/src/bin/`；逐项阅读新增声明、导入、字段、局部绑定及调用上下文。本轮另复核 bsp.rs 的白名单删除。该清单是源码可达性核对，不能替代编译。

`.github/workflows/ci.yml:180`–`:235` 的 rust-check 实际为五档，均 `--all-targets`、`RUSTFLAGS=-D warnings`，覆盖 test 与非 test；不能只按注释里的“三档”理解。

| CI feature 配置 | test / 非 test 静态核对 |
|---|---|
| 默认，无 feature | `Cargo.toml` 的 required-features 排除 S 会话 bin；31 个差异文件不编入默认目标。BSP 锁在默认库测试中仍读取磁盘上的 bin 源码，故默认测试也能因扫描而失败。 |
| `backtest_bin` | 同样不启用 `s_session`，不编入这些 S bin 模块；库扫描锁仍覆盖它们。 |
| `s_session` | H1 冷方法及调用整体由 incremental 门排除；原无门迁移包装成为死码，本轮删除。常驻迁移核、Ordinary 路径及非增量 RefCell 使用仍存在。 |
| `s_session,s_incremental` | Unix 冷入口、类型、seed/rebind、再导出和调用同门；生产调用不依赖 test helper。测试钩子及夹具保留 test 门。 |
| `s_session,s_capture_native_testonly` | Cargo feature 传递启用 `s_incremental`，等价于用户列出的三 feature 组合。Ordinary 诊断、H1、N1 仍按原 prefix 分流，新增调用不依赖额外测试接口。 |

逐文件覆盖清单如下；路径均相对 `rust/src/bin/`，`inc` 指 `s_incremental`。

| 差异文件 | 新增项与实际引用／门控核对 |
|---|---|
| `s_session_v2/cache.rs` | 仅文档，无新增声明。 |
| `s_session_v2/content_bi.rs` | seed/rebind 为 `all(inc,unix)`，由 generation 调用。 |
| `s_session_v2/content_producer.rs` | seed/rebind 同门且由 generation 消费；bar 查询保持原有两个调用。 |
| `s_session_v2/s_fact_cache.rs` | cold_seed/rebind 同门；旧 advance_owner_bound 改 test-only；with_work 由 generation 生产调用，新 work/reserve/push 参数均使用。 |
| `s_session_v3/admission.rs` | max_store_bytes 字段/构造/getter 同 inc；冷 require/release/capture/certify/first_deadline 供 owner/driver 使用，with_cold_account 由 Budget 实现调用。 |
| `s_session_v3/admission_anchor.rs` | test_prepare_caps 为 `all(test,inc,unix)`，由 provider 预算锁使用。 |
| `s_session_v3/admission_anchor/native_n1.rs` | 既有 NativeOwner 改用 NativeComponentAudit，无新增孤项。 |
| `s_session_v3/admission_private/fd_wal.rs` | cfg! pragma 不增声明，新增 authorizer 夹具位于既有测试函数内。 |
| `s_session_v3/admission_private/mod.rs` | read_session 由 validate_prefix 与 inc 的 certify_cold 共用；NativeComponentAudit 有 N1 使用；cold_start_wire 外层 test+内层 inc，trace 冷字段由 snapshot 消费。 |
| `s_session_v3/admission_provider.rs` | RefCell、ledger_cache、AUDIT_LEDGER_SUFFIXES 被非增量也存在的 measure/members/verify_before_open 使用；冷转换类型/四方法为 inc，供 admission/owner 使用；七条新增锁均在 test，增量锁另加门。 |
| `s_session_v3/audit_ledger.rs` | from_receipt_view 为 inc；两个 bind_view 被原包装及冷 fold 使用，原包装仍有调用。 |
| `s_session_v3/audit_owner.rs` | cold_source 为 inc；cold_failure/open_rebuilt_history/bind_admitted_history/seed_rebuilt_history/Candidate 冷 helper 在 inc+unix 生产链；组件测试入口及两个 hook 在 test+inc+unix；新增 binding 全部使用；唯一迁移包装调用已转共用核。 |
| `s_session_v3/audit_owner/admission_tests.rs` | construction_tests 父模块为 test+inc，include 另要求 unix；H1 helper 均用于具名锁，Ordinary 锁另有 testonly 门，无生产 import 泄漏。 |
| `s_session_v3/audit_owner/cold_source.rs` | 父 inc；ColdSource 构造/字段及各 trait 转发均由冷 fold 实际使用。 |
| `s_session_v3/audit_owner/effects.rs` | 固定 SQL 与注释变动，无新声明。 |
| `s_session_v3/audit_owner/history_clamp.rs` | open 改 test-only，生产继续 open_in_parent。 |
| `s_session_v3/business_json.rs` | 仅既有 RawPrefix 类型与方法提升至 crate 内可见，原历史审计调用仍存在。 |
| `s_session_v3/construction_set.rs` | CommittedSet cold/rebind 在父 inc+unix，construction_state 使用；LazySet cold 在 inc，ColdSource 使用；previous/knowledge/bootstrap/Seed 在 delta/finish 消费。 |
| `s_session_v3/construction_state.rs` | is_unbound_cold、ColdRawState、generation_binding、cold_seed/into_owner 均 inc+unix，owner/prepared/generation 调用配对。 |
| `s_session_v3/driver.rs` | cold_context、retention open/recover 为 inc，有 serve/CLI 调用；h1_owner_for_test 为 test+inc+unix；runtime mut 按 inc+unix 配对，非 unix 显式拒绝分支消费参数。 |
| `s_session_v3/generation.rs` | seed/rebind/SequenceMemo seed 在 inc+unix，prepared 调用；with_work 消费生产 work。 |
| `s_session_v3/history.rs` | CutSourceVerifier 的 auxiliary 传实际 Index；check_deadline 只提升可见性，原调用仍存在。 |
| `s_session_v3/mod.rs` | check_deadline 再导出与 s_fact_cache 调用同为 inc+unix。 |
| `s_session_v3/prepared.rs` | 冷 Runtime seed/adopt 为 inc+unix，owner 调用；install_audit_ledger 增量生产可达，新增 bound 分支消费原字段。 |
| `s_session_v3/source_audit.rs` | SourceInput/SourceRaw/audit_source_step 再导出在 inc，由 cold_source/state 使用。 |
| `s_session_v3/source_contributions.rs` | 父 inc；SourceInput 由在线 SourceMaterial 和冷 ColdSource 实现；SourceRaw/accessor/step 两方使用，bind/rebind 为 unix；cold_knowledge_ordinal 有两个 cold 调用；stats 前后同 test 门。 |
| `s_session_v3/source_facts.rs` | 父 receipt 为 inc；export_history 由 receipt 使用，export_parts 与原 export 共用。 |
| `s_session_v3/source_receipt.rs` | 父 inc；receipt_from_cold_step/export_history_business 由冷 fold 调用。 |
| `s_session_v3/state.rs` | full_audit_calls/thread_local 为 test；两个 fold/impl、共同方法及冷核为 inc，install_cold_qualification/rebuilding 有 owner 调用；对照入口改 test；在线未用参数显式为 _auxiliary。 |
| `s_session_v3/writer_cli.rs` | flag、resources 定义/构造/解析/消费按 inc/not(inc) 互斥配对；新增资源锁为 test+inc，无闲置 binding。 |
| `s_structure_session.rs` | 两个 H1 开库/交接 helper 同 inc+unix，owner 调用；删除无门薄包装，共用 migrate_in 常驻且仍有调用，授权门逐字保留。 |

Unix 的五档 test/非 test 源码核查除本次包装外，未发现另一项确定 unused/dead_code 回归。非 Unix 另核新增项与调用门控、明确拒绝分支；原包装在非 Unix 的增量构建也无调用。基线 `admission_provider.rs:8` 已无条件依赖 `std::os::unix`，因此不宣称整个 bin 支持 Windows 或非 Unix 编译。所有实际 `-D warnings` 结论仍须 CI。

### 源码扫描锁清单（独立只读复核）

输入为完整 `git diff 784bc6d7 HEAD -- rust/`：31 个文件、4214 个新增行，全部属于 `rust/src/bin/`。逐一枚举 `rust/src`、`rust/tests` 的 `include_str!`、`include_bytes!`、`read_to_string`、`fs::read`、`read_dir`、`File::open`、`read_to_end` 和 grep 子进程读取点，结合测试体区分源码守卫、JSON/数据夹具、运行产物读取。`include_str!` 读 Rust 源码的来源只有 funding_record、env_registry、config、policy_records 四个来源族；源码递归读取另有 BSP、certificate-chain、nest。未发现遗漏的、扫描本分支新增代码的 Rust 源码扫描类测试锁。`state.rs:2495–2496` 的 `include_bytes!("state.rs"/"driver.rs")` 只把实际源码摘要写入测试证据 binding，没有固定摘要相等断言，故不属于会因本轮源码变化而报红的扫描锁。

下表的结果来自仓外临时 Python 按原条件复现或原 grep 原样执行，**不是 Rust 测试 PASS；Rust 未验证，待 CI**。脚本未入仓，未运行 cargo/rustc。

| 锁名与来源 | 原扫描域／条件 | 本分支及当前返修结果 |
|---|---|---|
| `theta_v0::classifier::bsp::tests::no_stray_bsp_binding_rewrites_outside_registry`，`rust/src/theta_v0/classifier/bsp.rs:655` | 递归 grep `rust/src/**/*.rs`，原正则 `\.(find\|any\|filter)\(\s*\|[a-zA-Z_]+\|[^)]*\.source_index\s*==`；排除 bsp.rs 自身；命中必须与 file:line WHITELIST 双向一致（stray/stale 都为空）。 | 原候选 hits=24/whitelist=24，仅 stray `bin/s_session_v2/content_producer.rs:3411` 与 stale `:3349`。当前返修改写后 hits=23/whitelist=23，stray/stale 均为空；判据逐字不变，仅删去消失的同一白名单条目及其说明。 |
| `theta_v0::policy_records::tests::source_guard_no_constants_or_in_place_writes`，`rust/src/theta_v0/policy_records.rs:672` | config.rs 禁七种常量字面量；segment.rs 去 `//` 后禁 TAIL_WINDOW；递归 `rust/src/**/*.rs` 去 `//` 后查 `.level.l_max/.macd.fast/.macd.slow/.macd.signal/.tail_window/.second_seq_scan_window/.tangency` 后单 `=`，后三项的 `self` 内部写入除外。 | 新增面没有命中；全扫描违规 0。config/segment 不在差异面，固定字面量子锁亦为 0。 |
| `theta_v0::policy_records::tests::source_guard_tangency_only_from_record`，同文件 `:812` | 递归 Rust 生产源码（按原 `#[cfg(test)]\nmod tests {` 截断），跳过 `*_tests.rs` 及 theta_v0/config.rs、policy_records.rs，去 `//` 后禁 `Tangency::Closed/Open`。 | 新增面没有命中；全扫描违规 0。 |
| `candidate_and_strict_predicates_have_one_canonical_implementation`，`rust/tests/certificate_chain_single_source.rs:37` | formal/Origin 三个 Lean def 各一 canonical；递归 `rust/src` 恰一 `fn strict_chain_verdict(` 且位于 theta_v0/backtest/admission.rs；该文件委托调用与闭合条件文本各一次。 | Rust 全扫描仅 `rust/src/theta_v0/backtest/admission.rs:508` 一份，委托/闭合各 1；新增面没有同名实现。Lean/固定 admission 体与差异面无交集，未改。 |
| `theta_v0::config::tests::source_guard_no_record_constants`，`rust/src/theta_v0/config.rs:1198` | config.rs 首个 cfg(test) 前禁 15 个记录常量；backtest/data.rs 同范围禁 `0.80`。 | 两个扫描源均与差异面无交集；按原条件核对违规 0。 |
| `production_sources_hold_no_funding_constants`，`rust/src/funding_record.rs:725` | SOURCES 的 31 个固定 Rust 源文件，按原生产段截断、去 `//`，禁 INITIAL_CAPITAL/DEFAULT_OP_LADDER/SUB_EXPIRY/INITIAL_NAV_PLACEHOLDER/1_000_000/1.0e6/1e6，按字母数字下划线及点的边界判断。 | 31 个扫描源均与差异面无交集；复现全域违规 0。 |
| `theta_v0::env_registry::tests::no_stray_env_literals_outside_registry`，`rust/src/theta_v0/env_registry.rs:699` | 原 grep 递归 src/theta_v0，禁 env::{var,var_os,set_var,remove_var}/env_flag 的大写裸字符串；排除 env_registry.rs。 | 与本分支 31 文件差异面无交集；原 grep 全域违规 0。 |
| `theta_v0::env_registry::tests::const_declarations_match_registry`，同文件 `:663` | 只解析自身 `pub const ...: &str = "...";`，须与静态 REGISTRY 一致。 | 自身源码与差异面无交集；没有新增风险；仅源码/差异核对，不冒充重新执行 Rust 数组匹配。 |
| `classifier_production_code_never_imports_nest_artifacts`，`rust/tests/nest_isolation_guard.rs:79` | classifier 目录直属 `.rs`；依据精确 GUARD-ROLE 自述豁免，首个 cfg(test) 前去行注释及 mod nest 挂载后禁八种 nest 类型/路径/产物文本；需扫描数大于 10。 | 本分支 HEAD 差异与该域无交集；S1H 只删 bsp.rs 既有白名单，不增加禁串。复现 scanned=32、违规 0。 |
| R4 `AnchorLockTests::test_rust_test_prepare_exists_only_under_cfg_test`，`s_session/tests/test_tb01c_r4_anchor_lock.py:31` | admission_anchor.rs 只有一个 test_prepare 定义且邻接 cfg(test)；递归 Rust 调用点前文存在 test cfg；另两条锁 Python 生产引用与无锚 CLI 拒绝。 | 此锁确与本分支新增测试调用有交集；要求保留，父工蜂执行该现有三项测试。S1H 不新增或移动 test_prepare。 |
| V2 Reader 源 SQL 顺序锁，`s_session/tests/test_tb01c_query_integrity_layout.py:41,57` | 从 s_structure_session.rs 提取四个 OBJECT_*_SQL 常量，核 active 为 window_start、dead 为 first_known_generation；与 Python 查询次序对拍。 | 四个 SQL 常量均不在差异面。独立实际执行 `python3 -B -m unittest s_session.tests.test_tb01c_query_integrity_layout -v`，5 tests 通过。 |
| 生产无测试钩子／禁串清单，`.github/scripts/check-prod-no-test-hooks.py:232,290`、`gen-prod-banned-literals.py:273,291` | 生产可达模块树与 feature/cfg 推演、测试 API/环境变量检查及禁串提取；与本分支生产/测试代码均有交集。 | 要求保留，父工蜂实际执行原扫描与 `--check`；本复核未修改规则、源扫描脚本或禁串清单。 |

BSP 的精确语义核对：`content_producer.rs:3404` 的 `raw_bar_at` 返回 `&Bar`，对 `RawBars` 升序表用 `partition_point(source_index < coord)` 定位，再要求精确坐标相等，缺失或插入点越界均报 `MissingSource(coord as i64)`；没有访问 BspPoint、bits 或买卖点规则。基线 `784bc6d7:3349` 和原 HEAD `:3411` 的 `.filter` 本体逐字相同，diff 的新增行映射证明它不是新增语句，而是 S1D 的两个冷缓存方法净增 62 行造成 file:line 白名单漂移。显式 match 的三个分支（精确命中、不等、None）与 Option::filter 的原三分支严格相同，继续二分取值，无近似回落。

原日志已回看：`125002119534.log:801–808` 为 dead_code；BSP 四份失败原文分别为 `125002119586.log:4195–4196`、`125002119568.log:4248–4249`、`125002119152.log:5196–5197`、`125002119491.log:5069–5070`。这些行均点名同一 BSP 锁及 `content_producer.rs:3411`。


### 实际自检、独审与边界

主控对最终四个 Rust 文件完整 diff 验收，并实际执行：

- `rustfmt --edition 2021 --check --config skip_children=true rust/src/bin/s_session_v2/content_producer.rs rust/src/bin/s_session_v3/audit_owner.rs rust/src/bin/s_structure_session.rs rust/src/theta_v0/classifier/bsp.rs`，通过。
- `git diff --check`，通过。
- `python3 -B -m unittest s_session.tests.test_tb01c_r4_anchor_lock -v`，3 tests 通过。
- `python3 -B -m unittest s_session.tests.test_tb01c_query_integrity_layout -v`，5 tests 通过。
- `python3 -B .github/scripts/check-prod-no-test-hooks.py`，`violations=0`；这是源码扫描，未生成生产产物。
- `python3 -B .github/scripts/gen-prod-banned-literals.py --check .`，禁串清单与源码一致，清单未改。
- `python3 -B /private/tmp/issue1481-s1h-bsp-scan.py`，原 grep 及双向集合复现为 23/23，stray/stale 均空。
- `python3 -B /private/tmp/s1h-audit-source-locks.py`，输出 31 文件/4214 新增行；上表 policy/config/env/funding/nest 全域静态复现无违规，strict canonical/委托/闭合各 1。Rust 测试未运行，以上不冒充原具名 Rust 锁 PASS。
- `python3 -B /private/tmp/issue1481-s1h-lookup-equivalence.py`，18,018 例空/稀疏/重复/缺失/大坐标的旧查询与显式 match 对拍通过。此为 Python 机制原型，不是 Rust 运行收据。
- Python 与 HEAD 逐字比对：migrate_in 整函数不变；BSP 从 let src_dir 起的 grep、过滤、stray/stale 断言全不变；旧包装 Rust 定义及调用归零。

非作者代理分别只读穷举 31 文件 cfg 与源码锁，并独立审阅最终四文件 diff，未发现剩余确定阻断。仓外临时原型未入仓。REPORT 原 S1D–S1G 保留为历史收据，仅追加本节。

本轮未运行 cargo/rustc、Rust 测试、重型窗口或新 CI；未 git add/commit/push，未写 Git metadata。MainOnly/非 WAL 合法域、写前拒绝、成功后正常关闭/重启、台账四种精确旁挂登记与 64 GiB 帽、TestOnly Ordinary 先审后迁、既有全部锁/断言/逐名 CI 清单、E6 与 FU16 均保持。资源 `/3`、scrub、launcher/5、Python 服务控制、ADR、workflow 与测试源文件未改。后续须在主控封包的精确 head 取得 rust-check 五档与原 BSP 具名锁的 CI 收据。

### S1H 实际改动文件

- `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md`
- `rust/src/bin/s_session_v2/content_producer.rs`
- `rust/src/bin/s_session_v3/audit_owner.rs`
- `rust/src/bin/s_structure_session.rs`
- `rust/src/theta_v0/classifier/bsp.rs`

## S1I

> 状态：ready，可交主控封包取 CI。开工分支 `sandcastle/issue-1481`，精确 HEAD `fb06cf4d6a40460063ff3d28c7b600e52d949064`，工作区干净。本节记录 rust-check 死码返修及逐档源码核查；**Rust 未验证，待 CI**，不代表新候选已编译或整票完成。

### 原日志与首错

已读取 S1D–S1H 提示及引用的范围、禁止项、本报告对应章节和原始日志 `/Volumes/AgentStorage/h1-recover45c/ci-41360454221/125009585550.log`。以下日志位置为该原件物理行。run `41360454221` 的 rust-check job `125009585550` 对应 PR head `fb06cf4d`；`:172` 同时明确 checkout 的是合成 merge `62e8ff7`，即将该 head 合入 `784bc6d7` 的检查对象。

- `:555`、`:724`、`:776` 分别记录 default、backtest_bin、s_session 三档 `Finished`。
- `:786–787` 是第四档 `cargo check --all-targets --features s_session,s_incremental`，`:798` 的 `RUSTFLAGS` 为 `-D warnings`。
- `:801` 原文为 ``error: function `recover_scoped` is never used``，`:802–804` 指向原 `src/bin/s_session_v3/driver.rs:868:15`，`:807` 明确 `-D dead-code` 来自 `-D warnings`；`:810–812` 为 bin 编译失败、退出 101。第五档没有执行，不记作已验证。
- `:189–190` 的 shard 文本来自 `test_check_partition_rejects_missing_and_overlap`。源码 `.github/scripts/test_test_shard.py:85–89` 两次用 `assertRaises(SystemExit)` 验证缺项与重叠拒绝；日志 `:191` 为 `ok`，`:209–211` 为 16 tests / OK，故不是该 job 的失败原因。

### 修法与逐档调用者表

唯一代码改动是 `rust/src/bin/s_session_v3/driver.rs:868` 新增 `#[cfg(any(test, not(feature = "s_incremental")))]`，函数定义现为 `:869`。这是实际调用者门控的精确并集：唯一生产调用者 `writer_cli.rs:998` 受 `not(s_incremental)` 门控；测试包装 `driver.rs:894` 在 `:901` 调用，历史规则测试在 `:1327` 直接调用，两者均在 test 内。原包装声明 `:893` 的 `#[cfg(test)]` 保留。

不能删除旧入口或把所有测试改走 H1：`driver.rs:1005` 的 Fixture::open 以及 `:1014` 的 open_with_config 构造 Ordinary 测试臂，后者显式使用 Ordinary/None context；recover 包装的既有调用覆盖旧规则首错、PrePolicyRecords、pending 和时钟合同。`recover_scoped_with_retention` 则在 `driver.rs:844–850` 强制冷 context、CapturedNative、正式 ThetaConfig 和 H1 重建，二者并非可互换的测试入口。非增量 CLI 也确实仍需要旧入口。因此采用请求允许的精确 cfg 保留方案，不迁移这些必要调用者。

`.github/workflows/ci.yml:180–235` 五档均为 `--all-targets`、`RUSTFLAGS=-D warnings`。`rust/Cargo.toml:111` 要求 S bin 的 `s_session`；`:84` 的 inc 启用 session，`:87` 的 testonly 又启用 inc，所以第五档与显式三 feature 组合的 recover 门控相同。表中路径相对 `rust/src/bin/s_session_v3/`，行号为最终候选。

| cargo check feature 档位 | 非 test 的定义与调用者 | test 的定义与调用者 |
|---|---|---|
| 默认，无 feature | S bin 不编入，三个 recover 入口都不存在 | 同左；库 BSP 锁仍可扫描磁盘上的 S 源码 |
| `backtest_bin` | 未启用 session，S bin 不编入 | 同左 |
| `s_session` | 旧 scoped `driver:869` ← CLI `writer_cli:998`；retention 入口及测试包装不编入 | 旧 scoped ← CLI、包装 `driver:901`、规则锁 `driver:1327`；包装 `:894` 供既有合同测试调用；retention 入口不编入 |
| `s_session,s_incremental` | retention `driver:822` ← CLI `writer_cli:989`；旧 scoped 和测试包装均不编入，不再留下无调用旧函数 | retention ← CLI 及 `audit_owner/admission_tests:416,608`；旧 scoped ← 包装 `driver:901` 和规则锁 `:1327`；Ordinary 包装仍仅供测试 |
| `s_session,s_capture_native_testonly` | 与上一格相同；既有诊断臂不改变 recover cfg | 与上一格相同；包括 testonly 隐含的 inc，即显式三 feature 组合 |

以上为 Ubuntu/Unix 的源码调用者核对，不是编译证明。非 Unix 下新 cfg 同样移除增量非 test 的旧 scoped；retention 入口原有非 Unix 显式错误分支仍在 `driver.rs:830–834`。冷 helper 的 unix 门与调用方保持配对；基线已有无条件 Unix 模块依赖，不宣称整个 bin 支持非 Unix。

### 被替换／包装的旧函数穷举

核查输入为完整 `git diff 784bc6d7 HEAD -- rust/`，精确基线共 **32 文件、4212 新增行、280 删除行**。从全部差异提取旧调用改写、包装抽核及 cold 平行入口，再在实际模块树回查新旧调用者，不以名称出现次数代替 cfg 核查。下表 `F` 为仅 session，`I` 为两档增量配置，`T` 为 test；默认和 backtest 档不编入这些 S 模块。未标 test 的保留调用链覆盖该 feature 下 test/非 test 两格；标 `T` 的旧入口在非 test 排除。v3 路径省略 `rust/src/bin/s_session_v3/`，v2 及根路径另写。

| 旧入口／抽核关系 | 实际调用者、逐档结论 |
|---|---|
| `recover_scoped` → `recover_scoped_with_retention` | 本次唯一确定失去生产调用的档为 I 非 T；按上表精确门控修复，F 与 T 的必要旧调用保留 |
| `Driver::open_production_with_request_and_mode` → retention one-shot | 旧 `driver:414` 仍由生产 serve 包装 `:181` 调，CLI serve `writer_cli:781,813` 保留；F one-shot `:941` 用旧、I `:928` 用新。旧入口 F/I 均不可删 |
| `Driver::open_with_request_and_mode` → retention 包装 | 旧 `driver:390` 已为 T；`:165,376` 和 F+T 的 CLI `writer_cli:953` 调用。非 T 排除，F/I T 仍有调用 |
| `open_with_request_and_mode_context`、default context → `cold_context` | 共同核 `driver:520` 仍由 bound `:451`、serve `:283`、retention `:497` 调；default context 仍在共同路径，冷 context `:466` 仅 I，由 `:282,496,844` 消费 |
| `AuditConnection::open` → `open_rebuilt_history` | 旧 `audit_owner:337` 仍被 `driver:587,590` 的对照/F 分支及组件调用；新 `audit_owner:478` 为 I+unix，由 `driver:568,850` 调，原入口未孤立 |
| `open_history_component`、`Clamp::open` → `open_in_parent` | 旧 owner `:371`、Clamp `audit_owner/history_clamp:96` 已为 T+I+unix / T；组件测试和 owner `:373` 仍调。新 Clamp `:103` 由 owner `:419,700` 调，生产链存在 |
| `open_certified_history_component` → seeded / 正式 rebuilt | 旧 owner `:385`、seeded `:403`、admitted `:443` 全为 T+I+unix；旧→seeded `:392`，admitted→旧 `:451`，history_clamp_tests 多处及 admission_tests 仍调用；生产改用 rebuilt，不遗留生产旧包装 |
| `seed_historical_certificate` → `seed_rebuilt_history` | 旧 owner `:1654` 为 T+I+unix，由组件 `:397` 调；新 `:1716` 为 I+unix，由正式入口 `:714` 调 |
| `HistoricalCertificate::cold/cold_for_request` → rebuilding / inner | 旧 state `:1788,1798` 已为 T，前者由组件锁、后者由 owner `:1681` 调；新 `state:1809` 为 I+unix，由 owner `:557` 调；inner `:1832`、captured `:1881` 继续承载生产冷链 |
| `HistoricalCertificate::historical` 旧访问口 | state `:2028` 已为 T，调用 `:4107,4156,4203` 均在 I 测试中；I 非 T 不编入，没有靠旧访问口维持生产资格 |
| `HistoricalSourceFold::verify` → 共同方法 + 冷适配器 | 在线 trait 实现 `state:1310` 仍由 append verifier 调；冷实现 `:1325` 仍由 cold fold 调；共同 `require_next_cut:1269/verify_sources:1276` 两侧均用，I T/非 T 都有实际调用 |
| `state::audit/audit_native_in_request`、`s_allocation::audit_startup` → admitted bind | 旧入口仍有 Ordinary/对照调用，例如 owner `:937,939,988,991,1020,1579`；`audit_with_budget` 仍由 admission_private `:446,480` 和 state `:177,541` 调。F/I 保留各自调用，正式 H1 不以旧审计兜底 |
| `validate_prefix_inner` → `read_session` | admission_private 旧 `:605` ← `:592`；旧在 `:610` 调新核 `:662`，新增 certify_cold 也在 `:628` 调新核。F/I 原前缀包装继续生产使用 |
| 根 `open_db` → H1 专用开库／WAL handoff | 旧仍由 owner `:338` 及 v1/v2 调；新根 `s_structure_session:357,387` 为 I+unix，由 owner `:671,672,684` 调，未替掉旧合法输入域 |
| `migrate_policy_records_in_transaction` → `migrate_in` | 旧包装已在 S1H 删除，当前 HEAD 定义与调用为零；共同核仍由根旧迁移入口、owner `:696`、native_n1 `:1842` 调，授权门保留 |
| v2 `advance_owner_bound` → `advance_owner_bound_with_work` | s_fact_cache 旧 `:521` 已为 T，由 `:847,907,917,950,968,986` 测试调；新 `:528` ← generation `:558` 及旧包装 `:525`。F/I 非 T 不编旧，生产新链存在 |
| v2 `advance_bound_checked/inner` 新增 work 参数 | 旧名未退役：checked `s_fact_cache:568` ← `:541,565`，inner `:590` ← `:581`；F/I 的 owner 生产链仍经它们 |
| `QueuedFacts::from_receipt` → `from_receipt_view` | audit_ledger 旧 `:135` ← prepared `:1613`；旧→新 `:139`，冷 state `:1383` 也调新。两者均 I，T/非 T 均有调用；F 按原 cfg 排除 |
| `QueuedFacts::bind` → `bind_view` | audit_ledger 旧 `:223` ← prepared `:698`；新 `:230` ← 旧 `:228` 及冷 state `:1384`。I 的在线旧链和冷新链均有调用；F 下该包装原本不供生产使用，audit_ledger.rs:5 的既有模块 allow 未动，不是此次失调用回归 |
| `Ledger::bind` → `bind_view` | audit_ledger 旧 `:1114` ← prepared `:707` 及 fold_step `:1202`；新 `:1122` ← 旧 `:1120` 及 state `:1396`，各档无新失调用 |
| `audit_native_bound` → `audit_source_step` | source_contributions 旧 `:1936` ← effects `:1523`；新 `:1975` ← 旧 `:1958` 及 state `:1353`。父模块为 I，在线与冷核都仍生产可达 |
| `SourceMaterial` 固有读取方法 → `SourceInput` 转发 | effects 的 root/sequence_delta/request_delta/array_member/request_member `:1337,1343,1350,1358,1367` 仍被 contributions `:32,38,44,47,50` 显式转发；ColdSource 对应实现为 `:104,107,113,116,123`，I 两侧都用，未遗弃旧方法 |
| `member` 与 NativeSourceState 阶段泛化 | contributions `member:729` 仍由 `:1115,1486,1770` 调，empty/raw/cover/object/request/shape/witness/observation 阶段继续由共同 step 顺序调。I T/非 T 同门，未另留一份旧实现 |
| `receipt_from_native/export_raw_receipt` → cold receipt | source_receipt 旧 `:196` ← prepared `:1093`；共核 `:123` ← 旧 `:207`、新 `:238`；cold `:227` ← state `:1370`。父模块为 I，在线旧链和冷新链均在 |
| `RawReceipt::export_business` → history business；facts export 抽核 | 旧 receipt `:256` ← prepared `:1612,1629`；旧 facts `:155` ← receipt `:273`；parts `:202` 由原 export 与 history export `:174` 共用，新 history business `:301` ← state `:1381`。全部为 I，T/非 T 都配对 |
| `LazySet::new` / 在线 CommittedSet → cold 构造 | LazySet 旧 construction_set `:707` ← effects `:1517`，新 `:729` ← cold_source `:32`；CommittedSet cold `:235` ← construction_state `:338`，在线 finish 仍建原 set。父 I，cold CommittedSet 与唯一调用同为 unix |
| CommittedRawState 在线构造 → cold_seed / into_owner | 原在线 PendingRawState::advance `construction_state:519` 仍由 owner `:2530,2607` 调用；其 committed `:859` 在 owner `:1917` 被消费并于 `:860` 构造 CommittedRawState；cold_seed `:308` ← owner `:622`，into_owner `:265` ← owner `:1733`，冷边均 I+unix |
| v2 producer / ContentBi 的 new → 冷 seed/rebind | 旧 content_producer `:771`、content_bi `:1124` ← generation `:421,422`，F/I 都在；新 seed `:1633/:1133` ← generation `:499,502`，rebind `:1676/:1185` ← `:522,523`，均 I+unix 同门 |
| v2 DeltaLane::new / SequenceMemo::default → cold seed | 旧 lane new `s_fact_cache:464` ← generation `:551` 及自身冷 seed；旧 SequenceMemo default 在 generation `:428–433`。新 lane cold_seed `:401` ← generation `:482`，memo seed `generation:1613` ← `:488,493`，新增均 I+unix，原在线构造未失调用 |
| GenerationRuntime / Runtime::with_config → cold seed/adopt | 旧 generation `:413` ← prepared `:296,944`，Runtime 旧 `prepared:166` ← driver `:558,560,845`。新 gen seed/rebind `:467,509` ← prepared `:302,321`；冷 Runtime/seed/adopt `prepared:254,258,308` ← owner `:537,635,732`，均 I+unix；F/I 原构造继续使用 |
| `Runtime::install_audit_ledger` 接入生产 | prepared `:334` ← owner `:566` 及 prepared `:322`，I+unix 有实际生产调用，原组件测试也保留。非 I 既有 cfg_attr 不在本轮修改范围，没有新加 allow |
| RawPrefixIndex 的 new/ordered/advance_measured 提升可见性 | 旧 new `business_json:520` ← construction_state `:573` 等原链；ordered `:589` 原 history 生产调用仍在，advance_measured `:624` ← advance_admitted `:622`；新增 v2 cold 调用不替换这些旧入口，F/I 均有原引用 |
| `raw_bar_at` 的 filter → match | v2 content_producer `:3404` 仍被 `:1499,2224` 调用；S1H 的既有修法不动，各档原行为保持 |

其余差异包括 admission/provider 的新 cold 门面、预算 trait 委托、新冷载荷与 binding 方法、deadline 再导出、字段/签名可见性、固定 SQL、测试、注释及 BSP 白名单清理；没有另一个被新包装替代后遗弃的旧函数。新 cold 门面沿 owner 冷入口到原私有账，相关 unix/inc 门与调用保持配对。实施代理与非作者评审分别独立查阅差异与调用边，除本次 recover 门控外未发现第二项确定的 dead_code/unused 阻断；此结论限于静态阅读，五档实际 Rust 检查仍待 CI。

### 实际自检与保持项

- `rustfmt --edition 2021 --check rust/src/bin/s_session_v3/driver.rs` 通过。
- `git diff --check` 通过。
- `python3 -B -m unittest s_session.tests.test_tb01c_r4_anchor_lock -v`，3 tests 通过。
- `python3 -B .github/scripts/check-prod-no-test-hooks.py`，`violations=0`。
- `python3 -B .github/scripts/gen-prod-banned-literals.py --check .`，清单与源码一致，清单未改。
- `python3 -B /private/tmp/issue1481-s1h-bsp-scan.py`，读取当前 bsp.rs 的原正则及 WHITELIST，以原 grep 和双向集合复现，hits=23、whitelist=23、stray=[]、stale=[]；这是静态复现，不冒充 Rust 测试执行。
- Python 将 HEAD 的 driver.rs 仅插入上述 cfg 后与工作区逐字比对，相等；所有函数体、调用点、测试断言和原 cfg 均未改。报告历史部分保留，仅追加本节。

没有新增 allow/expect；recover 的错误顺序、retention 参数及资源解析、scope 的 enter/finish/drop 合同逐字保持。Ordinary/非 inc、H1 冷认证、输入域、写前拒绝、正常关闭/新进程重启、台账旁挂登记及 64 GiB 帽、TestOnly Ordinary 先审后迁、原锁和 CI 清单、E6、FU16 都不变。未运行 cargo/rustc、重型窗口、新 CI 或产物构建；未 git add/commit/push、未写 Git metadata。资源 `/3`、scrub、launcher/5、Python 服务控制、ADR 和 workflow 未改。

### S1I 实际改动文件

- `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md`
- `rust/src/bin/s_session_v3/driver.rs`


## S1J

> 状态：ready，交主控封包取 debug CI。基线为 `sandcastle/issue-1481` / `7a170a103387ce2217cc9d7495e12b2d046b802a`，开工工作区干净。本节记录源码返修与静态检查；**Rust 未验证，待 CI**。实际机器帧尺寸、撞 guard 的指令和修复后具名测试结果均未在本机测得。

### 原始日志与证据边界

已读 S1D 提示及其引用的 S1/S1B/S1C 范围、禁止项、43K 简报与 44G BLOCK，S1E–S1I 提示，以及本报告 S1D–S1I，特别是 S1G 的在线/冷帧隔离。本轮沿 #1481 已有实施授权，不另开票。下面日志位置均为 `/Volumes/AgentStorage/h1-recover45c/ci-41360454221/125009585993.log` 原件物理行；源码除标记“基线”外均为最终工作区，相对 `rust/src/bin/`。

- run `41360454221`，job `125009585993`；任务给定精确 head `fb06cf4d`，当前 S1I 基线仅增加 recover 的 cfg，不改变本条冷链。
- `:470–471`：`run-cargo-with-telemetry.sh ../.github/scripts/test-feature-delta.sh s_capture_native_testonly`。`rust/Cargo.toml:87` 的 testonly feature 隐含 `s_incremental`，后者隐含 `s_session`。
- `:890/893`：`Finished test profile [unoptimized + debuginfo]`；`:896` 为 `running 535 tests`。这是 debug 运行，不是 history36h 的 release 收据。
- `:1183`：`h1_admission_cold_negative_classes_match_full_oracle ... ok`。任务所写前一成功测试位置近似为 :1191；原件实际在 :1183，:1184–1190 是 telemetry，:1191 为空。
- `:1192–1193`：`thread 'v3::audit_owner::construction_tests::h1_admission_cold_handoff_foreign_sqlite_commit_rejects' (4195) has overflowed its stack`；`fatal runtime error: stack overflow, aborting`。
- `:1197`：同一个 `target/debug/deps/s_structure_session-ffdf887457b646bf --exact ...` 进程 SIGABRT，包含下列 110 个关注测试。命令没有 `--test-threads 1`，不能把“同进程”写成“单线程”。

原日志没有冷认证内部逐步骤栈回溯，故不能指定溢出发生于某一条机器指令、断言外写 hook 已执行或编造 `size_of` 读数。下面按 S1G 方法，以新增调用边、类型外壳和 debug 返回槽证明固定栈负担；任务给定的 release 通过只用作对照，不能替代修复后的 debug 验证。

### 根因：真实冷链多层携带大值，叠加冷专用局部槽

故障测试 `s_session_v3/audit_owner/admission_tests.rs:333` 先用 `h1_admission_seed_three` 写三代，再在 `:377` 经正式 Driver 冷开；成功认证后才会运行外连接 COMMIT hook。其生产调用链为：

1. `s_session_v3/driver.rs:520/568` → `audit_owner.rs:502 open_rebuilt_history`。外层持临时冷 Runtime、候选和 ColdRawState，基线 `audit_owner.rs:504` 还把 `[u8;16384]` 哈希缓冲放在这个长寿命 debug 帧。
2. `audit_owner.rs:551` → `admission.rs:193 WriterScope::certify_cold<T>` → `admission_private/mod.rs:628 certify_cold<T>` → `:672 read_session<T>`。基线业务闭包返回 **`(CompleteCandidate, ColdRawState)`**，所以包装层的 `Result<T,String>`、`Option<T>`、`Result<(T,DeltaShape),String>` 以及 `map`/`and_then`/闭包移动槽都必须能容纳同一大载荷；值尚未实际返回并不能使 debug 帧的类型槽变小。`Work::run` 当时也携带同一个 tuple。
3. 冷认证在原私有 FD/VFS 事务中，经 `state.rs:1849 cold_for_request_rebuilding` → `:1871 cold_for_request_inner` → `:1920 cold_captured` → `history::fold_certified_history` → 冷 `CutSourceVerifier::verify`。基线 rebuilding 另按值返回 `(AuditedState, Option<NativeSourceState>)`，基线 `construction_state.rs:308 cold_seed` 又按值接 source、返回 ColdRawState，两段都增加同型槽。
4. 冷 fold 经 `state.rs:1327 advance_cold_sources` 调在线同一个 `source_contributions.rs:1975 audit_source_step`。基线冷 verify 将 source/tuple、RawReceipt、台账 delta、四根闭包校验结果全部放在同一个函数体；来源步的深层 codec 运行时也背负后面阶段尚未执行的槽。S1G 只把这段从**在线** HistoricalSourceFold 拆走，没有缩小真实**冷** fold 的帧。
5. 后续 `audit_owner.rs:635` → `CommittedRawState::cold_seed`，`:649` → `prepared.rs:258 seed_cold_generation` → `generation.rs:467 seed_cold` → `s_fact_cache.rs:401 DeltaLane::cold_seed`、`content_producer.rs:1633 seed_cold_delta`、`content_bi.rs:1133 seed_cold_delta`。基线 prepared 的局部 generator 构造与 seed 深链共帧，adopt 又按值接整份 Runtime，均是同类固定外壳叠加。

尺寸模型沿 S1G 的 64 位常见布局，未运行 Rust：

| 载荷 | 源码与估算 |
|---|---|
| `NativeSourceState` | `source_contributions.rs:493–543` 实数为 **23 Map、11 Tree、3 Order**。按 Map/Tree≈136 B、Order≈160 B，单容器壳小计 `(23+11)×136+3×160=5104 B`；加 Built、binding、TextValue、4 个 Option<SequenceRef>、标量与 SourceWork，约 **5672 B**。Python 已从最终源码重新计数。 |
| `ColdRawState` | `construction_state.rs:222/259` 包装 CommittedRawState，内嵌上述可选来源态以及 RawPrefixFacts、约152 B 的 CommittedSet、seal/许可/父账戳，约 **6–7 KiB**。`None` 仍须容纳完整变体；原认证 tuple 还另带 CompleteCandidate，绝非几个指针。 |
| `SourceExports` / 收据 | `source_facts.rs:33` 的 3 个 CodecVec 加 usize，按 CodecVec≈96 B 为 **296 B**；`source_receipt.rs:17` 的 RawReceipt 另有 rows、BusinessRows、NativeValue 与 metadata 壳。vector 内容在堆上，不按历史条数计入栈；仍应与5.7 KiB来源状态的移动槽分帧。 |
| Runtime / generator | `generation.rs:354` 的 GenerationRuntime 约3–4 KiB，`prepared.rs:120` 的 Runtime 约4–5 KiB；auxiliary 另含8份 SequenceMemo。`fixed_radix.rs:197` 只内嵌 root/len/Ownership，`:57` 的 Block 是指针及许可，**不把 radix 树堆节点误算为内联数组**。此估算是数量级，不是 ABI 测量。 |
| 哈希缓冲 | `[u8;16384]` 精确 **16 KiB**，原来与冷认证/生成 seed 共帧，修后只在短哈希调用内存活。 |

不能把上表简单相加成“实测总栈”：编译器是否合并移动槽、各槽是否共享空间，须由 CI/反汇编验证。但大载荷在多层泛型函数和冷阶段帧中按值传播的结构是可由源码直接证实的本分支新增负担。

无界递归假说不成立于新增冷链：逐 cut/逐 raw/逐赢家均为循环；新增 seed、rebind、认证门面和来源适配器没有自调用或互调环。`ColdSource`/在线 SourceMaterial 的 trait 转发显式落到固有读取方法，没有递归重入 Driver 或冷认证。codec/sequence/radix/AVL 原有树遍历深度受结构高度约束，本轮未改。这里修的是**有界调用链固定大帧**，不是把三代历史解释成无界递归。

### 修法与收尾合同

| 最终文件:行 | 修改与保留的合同 |
|---|---|
| `audit_owner.rs:57/521` | 将可执行文件哈希移到 `cold_binary_commit`，原每次 read 前的 deadline、原准入预算 precheck、16 KiB分块与SHA-256配方不变；哈希返回后缓冲帧已退出。 |
| `audit_owner.rs:549–550/670/684–692` | candidate/construction 留在借用的外槽，业务闭包及 `codec_work.run` 返回 `()`；因此 certify_cold/read_session 实例 **T=()**。原 FD/VFS 包装源码完全未动；仅全部 rollback、close、registration、reconcile、期限、源连续性门成功后才取槽并走正式开库。失败先丢两槽，再清 context。没有提前公开载荷。 |
| `state.rs:1849/2009` | rebuilding 以借用 source_out 移交来源态，取消携带来源态的返回 tuple；cold_captured 用 swap 移交给唯一传入空槽的调用者，减少 take/赋值移动槽。raw facts 先提取、cold source 后移交的 S1G 次序保留。 |
| `construction_state.rs:308/349` | cold_seed 借用来源态、写入借用 output，返回 `()`，成功末端才 take 来源。原 `size_of::<Self>()` 许可仍对同一个未改类型计费；fresh/binding、已发布F、来源/根配对、Set与父账戳判据不变。 |
| `audit_owner.rs:649/663–667` | cold generation seed 失败在业务闭包内清 construction 槽，按原局部变量失败路径及时退许可，不额外保活到只读收尾。candidate 只在全部业务成功末端写槽。 |
| `audit_owner.rs:565/677–682` | `CodecWork::run` 还可能在业务成功后因scope/parent健康后验拒绝，见 `rust/src/s_content/capture_allocation/scratch.rs:428–450`。将run结果存为小型Result，任何Err均在业务operation返回前按candidate→construction清槽，保持原tuple在run失败时的析构时机；不把大tuple重新带回返回链。 |
| `state.rs:1327/1373/1421` | 冷 verify 拆为顺序的来源推进及收据/台账闭合，返回 verify 的只是 counts/小型 exports；来源大槽不再与后半段大局部共帧。顺序仍为代号门→同核来源Δ→receipt/business→四根闭包→stage/bind/台账commit→共同来源核→期限/前沿更新。在线 HistoricalSourceFold 的字段及实现不变，共同 audit_source_step 不变。 |
| `prepared.rs:258/312/321`、`audit_owner.rs:760` | generator 构造槽放独立初始化函数；seed只借用临时 cold Runtime 内的生成器，失败清None，成功后才装binding。adopt借用 cold Runtime，仍先严格rebind，再 take 台账/binding/generator；不用整份Runtime按值参数。临时Runtime一直独占且不逃逸。 |
| `admission_private/mod.rs:419/456/490` | `audit_complete_prefix` 独帧丢弃完整 AuditedState，隔离共用 audit_core 里未执行分支的返回槽；增量Ordinary/NativeComponent及非增量原完整分支仍调用同一审计、同一预算。所有prefix分流、epoch/rule、旧协议审后拒绝及N1事务边界不变。 |

没有 Box 或新分配/费用合同，没有测试钩子、allow/expect lint豁免、环境开关或两档判据。新增 `.expect(...)` 是唯一成功赋槽和ledger存在的内部不变量断言，不是 lint `#[expect(...)]`；调用前置在同一私有路径内建立。

### native-testonly 分支核对

`Cargo.toml:84/87` 使本次三feature共享同一增量冷核；`state`、冷source、prepared、generation并无一套 native-testonly 冷算法。`s_allocation.rs:153–226` 的区别是生产增量固定Native，而testonly保留环境选择；`driver.rs:219/248` 的VenueProfile/诊断fixture仅用于对应诊断入口。故障 `admission_tests.rs:450` helper **显式 CapturedNative**，不经过这些选择分支，不能把feature名当作已证实的动态根因。

仅testonly编入的 `admission_tests.rs:971 h1_admission_ordinary_prepolicy_bad_history_rejected_before_migration` 仍经过 audit_core 的完整 Ordinary 判据，先审后迁保持；它在 :1197 的15个admission测试中。相邻Native正式前缀的schema门也保持，失败不转Ordinary/组件重判。`audit_complete_prefix` 专门隔离二者共享帧上的完整结果槽，不改变选择。其余 native-only 观测/故障注入锁没有进入本条正式冷核的调用边。

### S1以来新增或加大冷帧穷举

以完整 `git diff 784bc6d7 HEAD -- rust/` 的32文件差异为输入，枚举冷声明、局部及其实际调用边，再加本轮5文件；未仅grep故障测试名。下表全部覆盖 test/非test 和显式三feature配置，风险结论限源码。

| 文件/路径与本轮最终入口 | 栈上对象/递归与处理 |
|---|---|
| `admission_provider.rs` 的 measure、verify_cold_ledger_cache、`:375 release`、`:388 capture_cold_conversion`、`:459 verify_cold_content`；admission.rs `:177–225` 门面/with_cold_account | Vec、BTreeSet、固定stamp/FD/引用壳，遍历迭代，未内嵌文件字节。通用certify的T传播已由调用实例改unit；其他cold门面返回小型stamp/标量，保留。 |
| `admission_private/mod.rs:430/628/672` audit_core/certify/read_session | 完整结果槽分帧；冷T=unit。SharedAccounting、首错记录、VFS注册/连接/回滚/关闭及原首读时钟不变。 |
| `audit_owner.rs:502/1744/3089/3140` open、seed_rebuilt_history、候选迁移、重绑 | 修hash与认证载荷传播；seed_rebuilt接收已核载荷仍按原封闭交接一次消费。迁移/重绑用游标、Vec/ExactRows壳与逐行循环，没有额外历史内联数组；源连续性、两把IMMEDIATE锁及拒绝顺序不动。 |
| `audit_owner/cold_source.rs:17` / `construction_set.rs:729 LazySet::cold` | LazySet含3个Option<SequenceDelta>及声明壳，约KiB量级；仅借用store和parent，按实际差分迭代。来源推进已与receipt阶段隔离，未改读取判据。 |
| `construction_set.rs:222/235` rebind/cold；`construction_state.rs:265/308` into_owner/cold_seed | Set约152B；ColdRawState6–7KiB。大source输入及cold载荷Result输出改借用；已有实际owner状态不改变类型、不新增Box或预算，不放宽rebind。 |
| `prepared.rs:254/258/312/321` cold_runtime/seed/初始化/adopt | 冷Runtime固定壳在冷入口持有；generator大构造与seed分帧，adopt改引用。其余Runtime构造在深层冷fold开始前已结束，无新增递归；失败恢复fresh。 |
| `generation.rs:467/509/1613` seed/rebind/SequenceMemo::seed_cold | 只借用生成器；lane及键序列为容器壳。末代根、raw/effective序列、F、前代binding按原严格门核验；sequence遍历为原结构有界遍历。 |
| `s_session_v2/s_fact_cache.rs:401/443` DeltaLane冷seed/rebind | lane约KiB固定壳；赢家索引和rows在堆，逐赢家append。无历史条数大小栈数组，未发现本轮需再改的新增大返回链；与generator构造槽叠加已解除。 |
| `s_session_v2/content_producer.rs:1633/1676` seed/rebind | Vec外壳24B及RawRefSpec/摘要固定局部；逐winner恢复源缓存，非递归。没有完整NativeSourceState或Runtime按值返回，保留。 |
| `s_session_v2/content_bi.rs:1133/1185` seed/rebind | 3×Vec外壳加2×SequenceRef，百字节量级；leaf内容在堆。用相同coord_text_leaf/source_root/sequence核恢复，循环严格升序；旧sync_delta严格门不变。 |
| `state.rs` cold/cold_for_request/inner/captured与冷fold | 大source tuple改借用；冷verify拆两阶段，S1G在线/冷隔离保留。ledger=None组件不执行冷来源步，但类型仍是同一冷wrapper，亦经本轮薄verify。 |
| `source_contributions.rs:1975` / `source_receipt.rs:227` / `source_facts.rs:33/174/202` 同核来源、冷收据、business导出；audit_ledger from_receipt_view/bind_view | 共同来源步按值状态的实现是原在线核，本轮不改其签名/判据；新增冷调用者已拆后续大槽。Exports约296B、BusinessRows4个CodecVec；receipt/delta壳固定，payload在堆。receipt/ledger阶段不再与来源状态移动帧并存。 |
| `driver.rs` cold_context、retention open/recover；`s_structure_session.rs` H1开库/转换/迁移 | 小型参数、配置/Path/事务引用及固定 stamp；按正式同一冷入口认证，未另建递归路径。真正源写入仍在认证及全部只读收尾成功之后。 |

另穷举差异新增固定数组：生产大数组仅旧hash的16KiB；domain `[0;6]`为48B，flag/suffix清单为常量。provider测试里的4KiB文件块是各自独立物理门测试局部，不在来源深链。原sequence/radix/AVL的栈深不是S1新增递归，本轮不重写。

### 原日志 :1197 的110项精确清单与覆盖映射

从原始process命令提取完整测试名并核全部唯一、全部能在Rust源码找到唯一函数定义：**69 h1、11 b2_native、30 p6，共110**。这里列的是实选集合，不能用源码中整个族的测试数代替。

| 实选模块/数量 | 共同路径与本轮覆盖 |
|---|---|
| provider/control_lock_tests 4 | 缓存精确登记、换件/bytes/转换stamp；物理门即可拒绝，不进入冷source深链。 |
| audit_ledger/tests 3 | 低层完整资格/分区/闭包delta，固定投影壳；无新Runtime seed。 |
| audit_owner/construction_tests 62＝50 h1+11 b2_native+1 p6 | 其中admission文件实选15条走正式Driver冷认证/重启/交接或对应拒门；其他真实owner写入、冷重开及oracle同核，覆盖在线S1G隔离及本轮冷fold隔离。先成功写再做负例的锁也计入。 |
| driver/tests 1、prepared/tests 16 | 真owner生成、ledger、recover/冷重建以及失败清理；经过同核来源/收据投影，正式冷路径消费本轮seed/adopt修复。 |
| history/effective_sequence_tests 4、request_closure_tests 5、history/tests 2 | RawPrefix/请求闭包/历史cut直接路径，调用相同源容器/codec，通常不进入正式Runtime冷seed；核新增壳与有界遍历，无额外修复。 |
| source_audit/source_receipt/tests 9 | 原始收据有效期、四根/请求替换、费用释放；核exports与导出代码，不构造正式coldRuntime。 |
| state/h1_certificate_tests 3 | 组件直接cold_captured，ledger=None；薄冷verify及同核来源判据保持，不引入正式认证outslot。 |
| writer_cli/tests 1 | retention资源档显式配置门，不进入来源冷fold。 |

下列完整名均原样保留，测试体/断言无改动，未添加ignore或改CI过滤：

```text
v3::admission_provider::control_lock_tests::h1_ledger_cache_conversion_stamp_rejects_changed_main_or_wal_frames
v3::admission_provider::control_lock_tests::h1_ledger_cache_discard_releases_only_cache_pins_before_rebuild
v3::admission_provider::control_lock_tests::h1_ledger_cache_replacement_or_new_member_is_rejected_before_discard
v3::admission_provider::control_lock_tests::h1_ledger_cache_uses_original_store_cap_before_discard
v3::audit_ledger::tests::p6_raw_partition_cannot_reopen_or_append_to_complete_certified_ledger
v3::audit_ledger::tests::p6_two_object_generations_commit_disjoint_closure_deltas
v3::audit_ledger::tests::p6_verified_units_difference_keeps_mode_and_first_generation
v3::audit_owner::construction_tests::b2_native_carry_budget_rejects_without_losing_committed_frontier
v3::audit_owner::construction_tests::b2_native_formal_failure_rollback_and_commit_failure_revoke_all_state
v3::audit_owner::construction_tests::b2_native_owner_restore_bounded_multicut_matches_writer_and_rebuild
v3::audit_owner::construction_tests::b2_native_owner_restore_wrong_library_layout_rejects_in_append_owner_cut
v3::audit_owner::construction_tests::b2_native_source_future_duplicates_preserve_earlier_actual_fault
v3::audit_owner::construction_tests::b2_native_source_global_dependencies_match_full_oracle_and_rollback
v3::audit_owner::construction_tests::b2_native_source_phase_errors_precede_later_bad_material
v3::audit_owner::construction_tests::b2_native_source_unsettled_rechecks_added_nonzero_group_before_partition
v3::audit_owner::construction_tests::b2_native_two_nonempty_formal_rounds_match_full_oracle_at_every_commit
v3::audit_owner::construction_tests::b2_native_warm_cold_and_ordinary_match_all_rows_nodes_roots_receipts_and_counts
v3::audit_owner::construction_tests::b2_native_warm_missing_or_stale_originals_reject_before_full_source
v3::audit_owner::construction_tests::h1_admission_cold_handoff_foreign_sqlite_commit_rejects
v3::audit_owner::construction_tests::h1_admission_cold_negative_classes_match_full_oracle
v3::audit_owner::construction_tests::h1_admission_cold_prefixes_match_full_canonical
v3::audit_owner::construction_tests::h1_admission_cold_real_wire_short_budget_rejects_before_write
v3::audit_owner::construction_tests::h1_admission_driver_cold_preserves_original_node_record_gates
v3::audit_owner::construction_tests::h1_admission_driver_cold_recover_rebuilds_ledger_bitwise
v3::audit_owner::construction_tests::h1_admission_foreign_sqlite_changes_retire_writer
v3::audit_owner::construction_tests::h1_admission_formal_accept_begin_prepare_recover_never_full_audit
v3::audit_owner::construction_tests::h1_admission_hot_rollback_journal_rejects_before_rw_open
v3::audit_owner::construction_tests::h1_admission_mainonly_nonwal_prepolicy_preserves_domain
v3::audit_owner::construction_tests::h1_admission_nonwal_conversion_handoff_foreign_commit_rejects
v3::audit_owner::construction_tests::h1_admission_normal_exit_same_database_new_process_restarts
v3::audit_owner::construction_tests::h1_admission_online_own_commits_match_full_canonical
v3::audit_owner::construction_tests::h1_admission_ordinary_prepolicy_bad_history_rejected_before_migration
v3::audit_owner::construction_tests::h1_admission_prepolicy_bad_history_rejected_before_migration
v3::audit_owner::construction_tests::h1_certified_commit_failure_releases_controls_before_return
v3::audit_owner::construction_tests::h1_certified_missing_candidate_and_mode_switch_are_sticky_stale
v3::audit_owner::construction_tests::h1_certified_online_consumes_source_receipt_and_matches_cold
v3::audit_owner::construction_tests::h1_certified_online_without_source_receipt_rejects_without_full_fallback
v3::audit_owner::construction_tests::h1_certified_open_empty_and_reopen_from_originals
v3::audit_owner::construction_tests::h1_certified_open_failures_leave_no_candidate_or_write
v3::audit_owner::construction_tests::h1_complete_canonical_empty_history_is_deterministic
v3::audit_owner::construction_tests::h1_control_bytes_change_in_same_connection_is_stale
v3::audit_owner::construction_tests::h1_control_observation_budget_failure_retires_without_writing
v3::audit_owner::construction_tests::h1_control_originals_are_retained_on_the_same_parent_and_released
v3::audit_owner::construction_tests::h1_control_parent_rebind_precedes_warm_request_observation
v3::audit_owner::construction_tests::h1_external_checkpoint_is_not_own_effect
v3::audit_owner::construction_tests::h1_main_identity_replacement_is_stale
v3::audit_owner::construction_tests::h1_missing_commit_receipt_retires_after_real_commit
v3::audit_owner::construction_tests::h1_other_connection_commit_is_sticky_stale
v3::audit_owner::construction_tests::h1_own_checkpoint_and_wal_reset_preserve_full_history
v3::audit_owner::construction_tests::h1_partial_checkpoint_waits_for_reader_then_resets
v3::audit_owner::construction_tests::h1_qualified_cold_rejects_identity_rule_root_and_unit_mismatch
v3::audit_owner::construction_tests::h1_qualified_cold_rejects_missing_partial_or_foreign_qualification
v3::audit_owner::construction_tests::h1_qualified_ledger_commit_failure_and_external_identity_write_are_stale
v3::audit_owner::construction_tests::h1_qualified_online_consumes_real_ledger_and_cold_rebuild_matches
v3::audit_owner::construction_tests::h1_qualified_stage_rejects_externally_rewritten_fold_digest_or_rule_revision
v3::audit_owner::construction_tests::h1_qualified_transition_short_budget_rejects_before_write_and_releases
v3::audit_owner::construction_tests::h1_raw_suffix_probe_detects_linear_skip_regression
v3::audit_owner::construction_tests::h1_real_own_accept_begin_publish_continues_same_life
v3::audit_owner::construction_tests::h1_real_write_does_not_explain_external_main_change_before_checkpoint
v3::audit_owner::construction_tests::h1_request_closure_real_owner_multicut_reuses_resolved_roots
v3::audit_owner::construction_tests::h1_roots_json_matches_real_cert_roots
v3::audit_owner::construction_tests::h1_schema_and_domain_changes_are_stale
v3::audit_owner::construction_tests::h1_sqlite_commit_failure_does_not_advance_clamp
v3::audit_owner::construction_tests::h1_unchanged_snapshot_survives_read_end
v3::audit_owner::construction_tests::h1_undeclared_write_rejected_before_sql_and_commit
v3::audit_owner::construction_tests::h1_unexplained_main_stat_is_stale
v3::audit_owner::construction_tests::h1_wal_identity_replacement_is_stale
v3::audit_owner::construction_tests::h1_zero_autocheckpoint_keeps_main_and_wal_round
v3::audit_owner::construction_tests::p6_native_source_context_preserves_real_owner_multicut_contract
v3::driver::tests::p6_complete_ledger_cover_matches_across_delta_and_full_lease_owner_paths
v3::history::effective_sequence_tests::h1_effective_blocks_match_independent_array_each_cut
v3::history::effective_sequence_tests::h1_effective_final_winner_edits_and_coordinate_swaps
v3::history::effective_sequence_tests::h1_effective_native_parent_exact_peak_rejection_and_release
v3::history::effective_sequence_tests::h1_effective_revision_work_tracks_height_then_append_reuses_blocks
v3::history::request_closure_tests::h1_request_closure_native_exact_peak_reject_and_release
v3::history::request_closure_tests::h1_request_closure_overlay_conflict_and_failure_do_not_escape_revocation
v3::history::request_closure_tests::h1_request_closure_rechecks_all_root_fields_and_revokes_both_modes
v3::history::request_closure_tests::h1_request_closure_shares_only_same_request_mode_and_keeps_canonical
v3::history::request_closure_tests::h1_request_closure_template_and_readset_negatives_match_full_oracle
v3::history::tests::h1_native_history_prefix_real_multicut_roots_and_linear_leaf_visits
v3::history::tests::h1_owner_retained_prefix_normal_append_and_revision_matches_oracle
v3::prepared::tests::p6_complete_ledger_real_owner_cover_tracks_revisions_and_rebuild
v3::prepared::tests::p6_complete_ledger_real_owner_records_pending_without_group
v3::prepared::tests::p6_complete_ledger_rejects_partial_and_wrong_rule_without_commit
v3::prepared::tests::p6_native_read_lease_boundary_cold_publish_fault_matrix_discards_runtime
v3::prepared::tests::p6_production_manifest_source_receipt_consumed_without_third_assembly
v3::prepared::tests::p6_production_manifest_source_receipt_rejects_request_substitution
v3::prepared::tests::p6_queued_units_ledger_commit_failure_keeps_content_and_never_retries
v3::prepared::tests::p6_queued_units_prepare_failure_discards_facts_and_stales
v3::prepared::tests::p6_queued_units_real_owner_commits_only_verified_complete_keys
v3::prepared::tests::p6_raw_partition_real_owner_keeps_revisions_without_full_certificate
v3::prepared::tests::p6_raw_partition_sidecar_commit_failure_keeps_content_and_stales
v3::prepared::tests::p6_real_prepare_closure_probe_stages_and_rolls_back_without_certification
v3::prepared::tests::p6_real_prepare_raw_receipt_keeps_old_revision_and_binds_request
v3::prepared::tests::p6_source_partitions_fees_release_after_refusal_commit_and_binding_failure
v3::prepared::tests::p6_source_partitions_real_owner_matches_originals_and_withdrawals
v3::prepared::tests::p6_source_partitions_sidecar_commit_failure_keeps_content_and_stales
v3::source_audit::source_receipt::tests::p6_business_source_coordinates_release_native_fees_on_success_and_refusal
v3::source_audit::source_receipt::tests::p6_raw_partition_borrows_original_payload_and_releases_pending_parent_fees
v3::source_audit::source_receipt::tests::p6_raw_receipt_expired_before_binding_releases_budget
v3::source_audit::source_receipt::tests::p6_raw_receipt_expired_before_consumption_releases_budget
v3::source_audit::source_receipt::tests::p6_raw_receipt_expired_during_closure_releases_budget
v3::source_audit::source_receipt::tests::p6_raw_receipt_fourth_root_failure_issues_no_prepared_receipt
v3::source_audit::source_receipt::tests::p6_raw_receipt_preserves_full_event_and_only_requested_suffix
v3::source_audit::source_receipt::tests::p6_raw_receipt_rejects_request_and_four_root_substitution
v3::source_audit::source_receipt::tests::p6_raw_receipt_source_failure_issues_nothing
v3::state::h1_certificate_tests::h1_cert_cold_empty_revision_winner_and_unpublished_tail
v3::state::h1_certificate_tests::h1_cert_deadline_and_scratch_failure_release
v3::state::h1_certificate_tests::h1_cert_rejects_control_manifest_current_sql_and_missing_original
v3::writer_cli::tests::h1_admission_one_shot_requires_explicit_retention_resource_file
```

### cfg、告警、实际检查与保持项

| feature档 | 非test源码核查 | test源码核查 |
|---|---|---|
| `s_session` | 冷helper/签名调用整体按原inc+unix门排除；audit_complete_prefix常驻且有非增量完整分支调用，不新增unused/dead_code。 | 原Ordinary完整判据及测试不动；无冷类型泄漏。 |
| `s_session,s_incremental` | source_out/output/adopt引用的定义与唯一调用均inc+unix；冷fold两helper均inc且由verify调用。可见性未扩大；SourceExports及其模块原已有crate内可见性。 | 同一生产核，原test hook仍仅test，原test_prepare外层cfg保留。 |
| `s_session,s_incremental,s_capture_native_testonly` | inc冷核同上；Ordinary/组件/Native仍按prefix分派。新helper不依赖testonly类型或钩子。 | native-only Ordinary先审后迁锁保留；精确110项及其原断言/故障注入门不动。 |

另核rust-check实际五档：默认与backtest_bin不编入S bin；上表三档由`--all-targets`覆盖test/非test。非Unix保持冷入口原门控配对，不宣称基线整体支持非Unix。没有新增allow/expect lint豁免、未使用导入或需要泛化推断的错误转换。原WorkError<String>标注保留；外槽类型由唯一赋值和消费确定，闭包捕获为借用；seed输出后才借construction，store退出后才移candidate；run后验失败也先清槽再交只读收尾。以上仅是 **`-D warnings`风险源码穷举，不是已通过编译**。

实际执行：

- `rustfmt --edition 2021 --check --config skip_children=true rust/src/bin/s_session_v3/admission_private/mod.rs rust/src/bin/s_session_v3/audit_owner.rs rust/src/bin/s_session_v3/construction_state.rs rust/src/bin/s_session_v3/prepared.rs rust/src/bin/s_session_v3/state.rs`，5文件通过；skip_children仅限制隐式递归。
- `git diff --check`，通过。
- `python3 -B -m unittest s_session.tests.test_tb01c_r4_anchor_lock -v`，3 tests通过，原Python锁未改。
- `python3 -B .github/scripts/check-prod-no-test-hooks.py`，`violations=0`。
- `python3 -B .github/scripts/gen-prod-banned-literals.py --check .`，禁串与源码一致，未改禁串表。
- `python3 -B /private/tmp/issue1481-s1h-bsp-scan.py`，读取现行BSP原正则及WHITELIST复现原grep：hits=23、whitelist=23、stray=[]、stale=[]。静态复现，不冒充Rust测试运行。
- 独立Python重新核NativeSourceState的23/11/3字段数，抽取原日志110实选测试及唯一源码定义；对拍原所有具名测试体、workflow/telemetry/scripts、原认证收尾实现、在线fold、共同来源步、RawCoords严格门与HEAD均未变。历史REPORT前缀逐字保留，只追加本节。
- 非作者独立代理只读审查全部5个Rust文件，重点复核outslot失败析构/预算生涯、冷fold步骤与首错顺序、借用、cfg/告警及保持项；未发现确定阻断，独立复跑rustfmt与diff检查通过。另一只读代理独立做32文件冷帧及110项调用组排查。

没有运行cargo/rustc、Rust测试、重型窗口或新CI；没有修改workflow/telemetry、设置RUST_MIN_STACK、另开大栈线程、改release、跳过或ignore。未git add/commit/push、未写Git metadata。

MainOnly/非WAL合法域与旧open_db合同、热RollbackJournal写前拒绝、坏历史迁移前拒绝、外写交接拒绝、成功后正常关闭/同库新进程可重启、台账四文件精确登记/真实字节/64GiB帽、真正未知旁挂拒绝、TestOnly Ordinary先审后迁、原全部锁与CI逐名清单、E6/N1及FU16均保持。资源/3、scrub、launcher/5、Python服务控制、ADR及整票关票不在本轮范围。主控须在新精确head运行现行native debug列表，并取全部h1/b2_native/p6具名结果；现有失败锁承担真实生产调用栈回归，不另造浅层替代锁。

### S1J实际改动文件

- `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md`
- `rust/src/bin/s_session_v3/admission_private/mod.rs`
- `rust/src/bin/s_session_v3/audit_owner.rs`
- `rust/src/bin/s_session_v3/construction_state.rs`
- `rust/src/bin/s_session_v3/prepared.rs`
- `rust/src/bin/s_session_v3/state.rs`

## S1K

> 状态：ready，交主控封包取精确新 head 的 CI。基线为 `sandcastle/issue-1481` / `c5a1ba0d8d655cab469989620732e40eef63ba43`，已含 main `eab5dd10`；开工工作区干净。本节只闭合非作者独审 46L 的三项 P1。Rust 未验证，待 CI，不能据源码核查宣称 Rust 编译、重启锁或整票验收通过。

已读取 `/Volumes/AgentStorage/h1-review46l/REPORT.md` 全文、S1D 及引用、S1E–S1J 提示和本报告对应章节。沿 #1481 既有票及本轮明确授权实施；Python 调用方迁移是本轮 P1-1 明确授权的范围。未改 spec、launcher 版本号、资源 `/3`、scrub 或 ADR；46L 建议项留后续。下文 Rust 简称相对 `rust/src/bin/`。

### P1-1：同步 launcher/4 的 recover 调用方

根因是 `s_session/s_service_control.py:845` 原先只取 stopped 状态，丢弃了 status 已核验的资源身份，原 `:859` 构造 recover 时也未带 s_incremental 写者必填的 `--resource-config`。start-s 已有 `:733` 参数与 `:449` 身份验证核，无需另建资源文件路径。

修法在 `s_session/s_service_control.py:845` 保留已核记录，`:862` 仅对 `s-launcher/4` 复用其中 `resource_identity`，给 recover 传恰一次原 start-s 冻结文件路径，并再次调用原 `validate_resource_identity`。仍用原 `require_current=False` 身份合同，允许当前 launcher 配置改动后恢复旧生涯；inode、权限、内容摘要和命令绑定仍须相符。阶段名、其余参数、错误收据和返回帧不变。`s-launcher/3` 对应非增量二进制，不接受此 recover 参数，保留其原命令；`/2` 也不变。Rust 必填与审计门均未放宽。

对 Python 调用面搜索 `advance/recover/resource-config` 后确认：现役控制器没有一次性 advance CLI 调用，`s_socket_client.py` 的 advance 是 socket 请求，经济域 recover 调的是各自二进制；没有第二个需要迁移的现役 s_incremental CLI 调用方。测试、实验驱动中的旧协议命令没有改为生产部署入口。

更正本报告原第 36 行：不能再写“旧二进制 launcher/4 合同保留”。准确表述是“s_incremental recover/advance 要求显式资源原件；本 PR 已同步 launcher/4 recover，复用已核 start-s 冻结文件；其余 launcher/4 使用合同保持”。该表行已改，其余历史章节不追改。

### P1-2：按失败原因选择关闭语义

根因是 H1 正式连接先开启 `NO_CKPT_ON_CLOSE=true`，直到整个交接、seed、adopt 成功才恢复；中途所有 `?` 都继承禁止检查点的关闭方式，明确的自身失败也会留下 SHM/WAL，下一次准入被 `UnsupportedWalShmJournalCombination` 拒绝。

`s_structure_session.rs:389` 的 `h1_normal_close` 将该配置恢复 false 并核 SQLite 返回值；`:421` 的 `h1_close_after_failure` 仅对自身失败调用它，正常恢复时逐字返回原错误。恢复配置本身失败时，仍返回错误，保留原错误作为前缀并附加关闭失败证据。开门的连接本地 PRAGMA 移至禁止检查点配置之前，配置安装失败也走检查后的正常关闭；转换 helper 和 owner 构造的失败同样覆盖。

`audit_owner.rs:752` 将正式交接收为返回 `()` 的闭包，所有自身早退经过统一关闭处理；candidate/construction 仍借用原外槽，到 seed 才 take，不再新增大载荷返回层。`audit_owner/history_clamp.rs` 用内联 Cell 标志记录实际比较失败及无法维持源连续性的错误；构造失败经借用的外 Cell 反馈，成功后由 Clamp 自持 Cell，两者在收口处合并，不新增 Rc 堆分配。其拒绝仍使用原具名错误，不按共同的 `HistoricalAuditStale` 外观把资源失败当完整性拒绝。首锁继承的 data_version 在 Clamp 首次检查中先于控制原件/预算读取核验；迁移后任一失败还在正常关闭前统一复核该版本，避免已发生的外部提交被资源错误掩盖。原 begin 实现和在线核不变。`history_physical.rs:39` 只同步新增观测字段的初始化；旧 test 包装补 `#[cfg(test)]`，不加 lint 豁免。

完整性拒绝的运维合同也写入 `audit_owner.rs:743` 的 #1481 注释：保留 fail-stop，拒绝动作不 checkpoint、不再改写源主库/WAL；SHM/WAL 可作为失败证据保留，直接重试可能仍报 `UnsupportedWalShmJournalCombination`。操作者先确认 S/Q 和所有外部连接已停止，保全主库、WAL、SHM 与错误证据，查明并处理外部写入或身份冲突；需要恢复可信原件时沿原有原件恢复手续处理，不能丢弃已持久事实。介质收尾使用既有 SQLite 普通连接正常关闭，不能手工删除旁挂、不能跳过准入。核无 SHM 后走既有监督器 `s_service_control.py:842` recover，再 start-s；入口合同见 `s_session/README.md:111`、ADR 0042:40。这里没有新清理工具，也不声称 recover 能越过 SHM 拒门或修复坏历史。

### 新增与加强的行为锁

Rust 锁均位于 `s_session_v3/audit_owner/admission_tests.rs`，模块前缀为 `v3::audit_owner::construction_tests::`，无 ignore。

| 锁与源码位置 | 行为与 CI 实际执行位置 |
|---|---|
| `h1_admission_temporary_seed_failure_same_database_new_process_restarts`，`:912` | 既有 cfg(test) thread-local 机制在第二锁、真实迁移 COMMIT 之后注入一次 seed 暂时失败；原错误逐字返回、钩子确实消费。失败返回后先断言无 SHM/WAL，再开任何检查连接；另一独立新进程对同库冷启成功，迁移已提交且 generation=3，不调用旧审计。`ci.yml:467` 逐名，owned 的 H1 release step `:449`，日志 `1481-admission.log`。 |
| `h1_admission_cold_handoff_foreign_sqlite_commit_rejects`，`:333`，加强原锁 | 原 UPDATE 外写及新增 DROP meta 两子例均保留真实 COMMIT、命中计数、规范 cuts、旧审计零调用与主库/WAL 字节相等；增加 SourceIdentityChanged/SourceWalChanged 具名断言，DROP 子例覆盖 begin 的固定 SQL 先失败。原 owned H1 逐名项保持。 |
| `h1_admission_nonwal_conversion_handoff_foreign_commit_rejects`，`:1264`，加强原锁 | 非 WAL 转换交接间隙外写要求 SourceIdentityChanged，原全部字节保护断言保持。原 owned H1 逐名项保持。 |
| `test_h1_recover_preserves_versioned_resource_contract_after_config_change`，`s_session/tests/test_tb01c_s3_service_control.py:437` | 真走 start-s 记录后恢复，/4 参数恰一次且与 start-s 同路径；配置改动后不另 freeze；/3 argv 和 recover 返回帧保持。既有 `s-observer` 的 Python observer and lifecycle tests，`ci.yml:1038`。 |
| `test_h1_recover_refuses_changed_resource_identity_before_process`，同文件 `:480` | inode、内容、登记命令和 status 后交接窗口替换都在恢复子进程前拒绝。与上一锁同一 Python discover step。 |

16 个原 S3 Python 锁体与 HEAD 的 AST 逐位相同，原 recover 锁全部保留。原 H1、Ordinary、FD/WAL、E6 逐名清单未删除。

### P1-3：移除临时触发与 PR 覆盖对照

根因是 history36h 的临时 `sandcastle/issue-1481` push 触发仍在，而仅靠“其他 CI 可能也跑到”不足以证明同 feature/profile 覆盖。`history36h.yml:4` 现恢复 main `eab5dd10` 的两分支原样，`:5` 保留 workflow_dispatch。

| history36h 原 step | PR `ci.yml` 覆盖与本轮处理 |
|---|---|
| H1 life clamp and history locks，`:19`，release `s_session,s_incremental` / `h1_` | 既有 extras(owned) H1 step `:449`，`:454` 过滤从 `h1_admission_` 扩为 `h1_`，承接全部原 H1 release 域，原逐名收据保留并加入新锁。 |
| Existing SQLite commit failure locks，`:21`，release 同 features | 新增现有 owned job 内步骤 `ci.yml:494`，`:498` 执行同过滤，逐名核 `h1_sqlite_commit_failure_does_not_advance_clamp`、`prepared_commit_sqlite_commit_failure_returns_no_receipt_and_rolls_back`、`sqlite_commit_failure_cannot_return_a_receipt_or_keep_real_prepared_state`、`native_capture_sqlite_commit_failure_retries_with_new_current_capture` 四项。 |
| Native debug source suffix and receipt expiry locks，`:23`，debug 同 features | 新增 owned 步骤 `ci.yml:508`，`:512` 保持 debug，逐名核 `audit_owner_incremental_normal_writes_full_oracle_and_suffix` 与 `p6_raw_receipt_expired_before_binding_releases_budget`、`p6_raw_receipt_expired_during_closure_releases_budget`、`p6_raw_receipt_expired_before_consumption_releases_budget`。 |
| Ordinary build contract，`:25`，`cargo check --features s_session` | 既有 rust-check `ci.yml:207`，`:215` 的 `cargo check --all-targets --features s_session` 并启用 `-D warnings`，无需重复。 |

新增两份 Rust 日志随原 H1 artifact 一起上传，`ci.yml:533`、`:534`；每项要求真实 `test ... ok`，不是只编译。Python 两新锁由现有 `test_tb01c_*.py` discover 覆盖，不新增重复步骤。上述是工作流接线核查，实际 Rust 收据须新 head CI 提供。

### Python 实跑结果与边界

使用 python3 实跑新增两锁、原 recover 锁与 R4 anchor 锁，均通过。非作者独立复跑新两锁及旧 recover 三锁共 5/5、R4 3/3 通过。

完整 `test_tb01c_*.py` 发现 100 项，95 通过、5 error；只读动态加载 HEAD 基线发现 98 项，93 通过、同 5 error，错误集合与文本逐项相同，均为本沙盒执行 `ps` 的 `PermissionError: Operation not permitted`。新增两锁全部通过，没有修改旧锁回避环境限制。相关 `issue1527_recovery/test_python_r4.py` 48 项当前与 HEAD 同为 2 fail / 9 error，主要是旧夹具缺 main #1657 的 `AdmissionContractVersionMissing` 及其收据级联，失败集合逐项相同；未扩修这些基线问题。完整 Python 面不能记成全绿。

实际对拍收据位于仓外 `/private/tmp/s1k-launcher-{current,baseline}-{tb01c,r4}.json`，临时执行器 `/private/tmp/s1k-launcher-verify.py` 未入仓。这些是本机实际测试记录，不代替 CI。

### 正式开门到关闭恢复的失败点分类表

表中“自身”均指错误仍须返回、清除本轮候选并退还原预算许可，随后核 false、正常关闭；“完整性”均指保留 true、fail-stop 及上面的受控恢复合同。没有将拒绝改成成功，也没有重新认证或旧审计兜底。

| 失败点，最终文件:行 | 分类与处理 |
|---|---|
| `s_structure_session.rs:358` 原 scope 门、`:359` open、`:364`/`:367` 同步 PRAGMA | 准入或自身初始化失败，尚未设置 true；沿普通连接关闭，不会遗留本轮禁止检查点配置。 |
| 同文件 `:370`–`:383` 安装 NO_CKPT 配置 | 自身；失败也由 `h1_close_after_failure` 核恢复 false。 |
| 同文件 `:448` 日志模式读取、`:454`–`:461` 排他模式/锁、`:466`–`:480` policy 授权/只读提交/合法 WAL 转换、`:481`–`:487` 固定 meta 读及 rollback | 自身或授权错误；转换 helper `:491` 统一核正常关闭后返回原错。完整认证、迁移授权和合法输入域保持。 |
| 同文件 `:463` authenticate、`:484` snapshot；`audit_owner.rs:769`–`:776` 首锁 token/原件核验 | `s_structure_session.rs:406` 按现有具名类别分类：Identity/Wal/Type/ChangedDuringPin/Medium 实际失配，以及 PathLost/FdLost/MetadataUnavailable 身份失证，保守按完整性处理。SourcePinFailed 是元数据已核后的本进程开 FD 失败，按自身处理。底层 errno 已丢的 metadata 失败不假称自身。 |
| `audit_owner.rs:724`、`:739` 两次开门，`:734` 转换连接成功关闭 | 开门遵上面两行；转换成功仍禁止关闭检查点，以保留交接凭据，EXCLUSIVE 的 heap 索引不产生 SHM。两交接 hook 仅 test。 |
| 同文件 `:743` Self::new；`:382`–`:395` 生涯编号及接管前置 | 自身；失败在接管连接前核 false，不增加 owner 大返回槽。 |
| 同文件 `:754`–`:768` 第一锁 begin 的 SQL/锁错误 | 默认自身；外部 DROP/坏 schema 可使 SQL 先失败，因此失败回调仍有界复核原 source/token，失配或失证则保留原 SQL 错并附具名完整性错误，禁止 checkpoint。 |
| 同文件 `:779` migrate_in、`:782` migration COMMIT/continue | 认证后的本写者合法迁移或自身事务失败，默认自身；`:781` 保存继承版本，后续任何失败收口再查版本，外部提交不得夹带自身错误正常关闭。迁移已提交的事实不回退。 |
| 同文件 `:785` Clamp 初始化；`history_clamp.rs:139`–`:164` | 父账、普通 PRAGMA、阈值及物理 NOOP 初始化错误属自身；模式实际非 WAL、被观测路径 open/stat/header 无法维持身份可用性则按完整性保全。文件读取分类见同文件 `:44`–`:64`、`:203`–`:221`。 |
| `audit_owner.rs:796` 第二锁 begin、`:797` check、`:802` seed、`:808` read_end、`:809` deadline、`:810` continue、`:815` history_result | 明确的资源、首读期限、seed、回滚和候选自身错误正常关闭；`history_clamp.rs:258` 首检先核继承版本，外部提交或文件/控制原件/连接域实际失配置完整性标志。history_result 只传播和清候选，retire 不清完整性标志。 |
| 同文件 `:816`–`:823` construction/actual_prefix、`:825` adopt | 本写者自身交接或资源错误；全部进入统一错误收口，不再带 true 直接早退。 |
| 同文件 `:831` 成功恢复、`:841`–`:860` 错误出口连续性复核 | 成功核 false；记录了继承版本的任一失败在正常关闭前只读再查 data_version。匹配才按自身关闭；不符附 HistoricalAuditStale 外部提交，读不到版本附 HistoricalAuditStale 失败后源连续性失证，两者 fail-stop。 |
| 同文件 `:864` 最终错误收口；`s_structure_session.rs:389`、`:429` | set_db_config(false) 必须核返回值。成功时原错误逐字返回；关闭配置也失败则报“原错误；关闭错误”，保留两份证据，绝不成功返回。 |

### cfg、告警风险和最终自检

| feature 档 | 非 test 源码穷举 | test 源码穷举 |
|---|---|---|
| `s_session` | H1 helper、Clamp 与正式冷入口均按原 inc+unix 门排除。Self::new 的普通错误行为保持，没有增量类型泄漏。 | 新锁文件沿原 test+inc+unix include 排除，原 Ordinary 锁保持。 |
| `s_session,s_incremental` | 关闭 helper、分类器、外 Cell/Clamp 内 Cell、继承版本字段及实际调用配对；无新堆分配、allow/expect、未用导入。原 begin 体逐字保持，T 仅 unit/u64/小转换凭据，冷大载荷仍在外槽。 | 新注入声明为 `all(test,s_incremental,unix)`，消费另有 `#[cfg(test)]`；测试调用显式 test 属性，无 ignore。open_in_parent 旧包装仅 test，防止生产 dead_code。 |
| `s_session,s_incremental,s_capture_native_testonly` | 与上档同一冷核、同一错误收口；不引入 native-testonly 生产捷径或环境钩子。 | 原 TestOnly Ordinary 先审后迁保持；新失败锁和原所有 H1 锁均编入并由 CI 执行。 |

默认与 backtest_bin 不编入 S bin，仍保留库源码扫描；非 Unix 维持原门控。上述六格是 `-D warnings` 风险源码核查，实际三档 test/非 test 编译及告警结论待 CI，未将“源码可读”写成“编译通过”。

最终实际检查：

- 5 个改动 Rust 文件 `rustfmt --edition 2021 --check --config skip_children=true` 通过，参数仅限制隐式递归；`git diff --check` 通过。
- 两个 workflow YAML 解析、全部 run 块 `bash -n` 通过，ci 71 块、history 4 块；触发分支与 dispatch、原 26 个相关具名项保留、新锁唯一性和八条 SQLite/Native 源码定义及完整路径核对通过。
- 仓库根 R4 anchor 锁 3/3；在 `s_session/tests` 目录以 python3 运行新增两锁及旧 recover 三锁 5/5。首次从根用包名运行该五锁因夹具不在 sys.path 而未导入，随后按测试目录入口重跑通过；完整 Python 实跑边界见上节。
- 生产无测试钩子源码扫描 `violations=0`。禁串重生成 1639 条，唯一新增 `h1_cold_seed: injected temporary failure`，随后 `--check .` 与源码一致；没有生产产物构建收据。
- 原 BSP grep/WHITELIST 双向集合静态复现 hits=23、whitelist=23、stray=[]、stale=[]，未修改判据。
- 两名非作者代理分别核 Python 合同/锁、Rust 失败分类/cfg/借用/帧与 CI 覆盖；最终差异无剩余确定阻断。其证据限源码及上述 Python/静态工具，不代替 Rust 运行。

没有运行 cargo/rustc、重型窗口、CI 或生产构建，没有设置 RUST_MIN_STACK、加大栈线程、ignore 或跳过测试；没有 git add/commit/push 或写 Git metadata，HEAD 仍为上述基线。MainOnly/非 WAL 合法域、热 journal 写前拒绝、完整性拒绝源字节保护、正常关闭重启、台账四文件精确登记/真实字节/64 GiB 帽、未知旁挂拒绝、Ordinary、资金、E6 和 FU16 保持。46L 的 CapturedNative 无 cache context 显式拒绝、structure_source 组合锁与其他建议留后续，不记为本轮完成。

### S1K 实际改动文件

- `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md`
- `.github/prod-banned-literals.txt`
- `.github/workflows/ci.yml`
- `.github/workflows/history36h.yml`
- `rust/src/bin/s_session_v3/audit_owner.rs`
- `rust/src/bin/s_session_v3/audit_owner/admission_tests.rs`
- `rust/src/bin/s_session_v3/audit_owner/history_clamp.rs`
- `rust/src/bin/s_session_v3/audit_owner/history_physical.rs`
- `rust/src/bin/s_structure_session.rs`
- `s_session/s_service_control.py`
- `s_session/tests/test_tb01c_s3_service_control.py`


## S1L

> 状态：ready，交主控封包取精确新 head 的 CI。开工 HEAD 为 `614823a0b2cbed5ddac5f4de0748ca511e7e3a9c`，分支 `sandcastle/issue-1481`，工作区干净；该 head 已含 S1K。46M 报告及本轮 native 原日志的冻结对象为 `c5a1ba0d8d655cab469989620732e40eef63ba43`，本节已在实际工位重定位。以下是源码修复、测试接线及静态检查收据，**Rust 未验证，待 CI**。

本轮沿 #1481 的 S1L 明确授权，已读取 S1D 及所引 S1/S1B/S1C 边界、S1E–S1K 提示、工位报告对应节、43K 简报、44G 及 46M 全文。只处理 P1-4、P2 冷历史对拍缺口与追加的 native debug 栈问题；不做资源 `/3`、scrub、launcher/5、ADR、性能或资源帽变更。未运行 cargo/rustc、重型窗口，未暂存/提交/推送，也未写 Git metadata。下文 Rust 简称相对 `rust/src/bin/`，除明确标为基线/日志外，行号均为本轮最终候选。

### P1-4：真正旧写入口在只读前缀拒绝 v3

根因重新定位：`s_session_v3/admission_private/mod.rs:445` 先按 protocol_revision 分派，原 None 前缀在增量构建执行 schema 核后直接成功；`control_audit.rs:195–210` 的 schema 核见缺列便在 `:208` 写 `DeltaShape::PrePolicyRecords`。这就是 `admission.rs:561–576` 的迁移许可。旧处理器的根 `open_db` 在 `s_structure_session.rs:332–354` 先转 WAL、补列，`recover_core:5873–5876` 才判规则拒绝；accept/advance 原样同族，未改它们来延后拒绝。

**先区分入口再改的证据**：`s_session_v3/mod.rs:220–231` 无选择器即 `Ok(false)`，`:248–262` 只有显式 s-session/3 选择器进入 writer_cli。v3 `writer_cli.rs:928` advance 在该独立分派内直接 `WriterScope::admit`，继续得到 None；旧 v1 则由根 `s_structure_session.rs:5993` 经 `v2::legacy_write_lock:3395/3405`，旧 v2 clock recover 则由 `v2/mod.rs:3336–3354` 独立分支调用 scope。两个旧调用点是可靠的调用链分叉点，不靠命令名称或 None 值猜测。

修法：`admission_private/mod.rs:379` 新增 LegacyWrite；`admission.rs:306–315` 用 `admit_legacy_write` 只签这个前缀；`v2/mod.rs:3354/3405` 两个真正旧入口迁移到它。`admission_private/mod.rs:447–451` 在只读连接读出 PROTOCOL_REVISION 后立即返回 `VersionMismatch：s-session/3 写入口必须显式使用 --protocol-revision s-session/3`。返回发生在 schema 核之前，未调用 before_write、更未把 PrePolicyRecords 写进 scope；也到不了 scope seal、正式写连接、根 open_db、WAL 转换及迁移。未恢复完整旧审计。v3 advance 仍是 None，未改 writer_cli。

| 入口 | 修后前缀/拒绝点 | 拒绝前是否写源库 |
|---|---|---|
| v1 recover | LegacyWrite，`admission_private/mod.rs:450` VersionMismatch | 否；仅私有只读 FD/VFS，尚未进根 open_db |
| v1 accept | 同上 | 否 |
| v1 advance | 同上 | 否 |
| v2 recover --clock-plan | `v2/mod.rs:3354` → LegacyWrite →同一 VersionMismatch | 否；clock 原件仍按旧次序只读加载 |
| LegacyServe（无选择器） | 仍 LegacyServe，`v2/mod.rs:3112–3124` verify_serve_startup→active，active `:196–203` 只读拒 v3 | 否；本轮未改 |
| v3 advance（显式选择器） | writer_cli:928 仍 None→H1 冷认证→认证后迁移 | 合法认证完成后才转 WAL/补列；正控还提交 g4 |
| v3 serve/recover（显式选择器） | NativeStartup/UnboundRecover，原 H1 | 本轮未改其准入前缀语义 |

**旧域逐位不变的源码证据**：新增提前拒绝仅 `cfg(s_incremental)` 且仅 PROTOCOL_REVISION 分支；None/PREVIOUS_REVISION 原 `audit_private_startup`、LegacyServe 唯一条件、原预算适配拒绝均原样。非增量的完整审计 wildcard 仍处理 LegacyWrite，最终 match 把 LegacyWrite 与原 None 同列。必要相邻面 `admission_private/mixed.rs:18` 也把 LegacyWrite 与 None 同列，避免旧 v1/v2 mixed 成功封存后被 wildcard 错拒；mixed 私有核 `:65–76` 对 v3 仍按原 MixedProtocolUnsupported 只读拒绝，未扩大 mixed SQL 合同。

穷举所有 RejectionPrefix 匹配：audit_core 的增量完整审计选择保留两原变体；schema work 的 NativeStartup 专项+wildcard 不变；非增量 NativeStartup 专项+wildcard 不变；末尾完整 match 新增 LegacyWrite 同 None；旧协议 LegacyServe 的 if-let 不变；validate_prefix NativeStartup 的 context 清理 if-let 不变；mixed::SealedPrefix::require 已同步。无遗漏穷尽 match，也无新 allow。

**新增锁**均在 `audit_owner/legacy_admission_tests.rs`；由 `audit_owner.rs:5639` 的 unix include 引入既有 test+s_incremental 模块：

| 锁名 | 源位置 | CI 逐名位置 |
|---|---|---|
| h1_admission_legacy_v1_recover_prepolicy_mainonly_rejects_unchanged | :135 | ci.yml:469 |
| h1_admission_legacy_v1_accept_prepolicy_mainonly_rejects_unchanged | :141 | ci.yml:470 |
| h1_admission_legacy_v1_advance_prepolicy_mainonly_rejects_unchanged | :147 | ci.yml:471 |
| h1_admission_legacy_v2_clock_recover_prepolicy_mainonly_rejects_unchanged | :153 | ci.yml:472 |
| h1_admission_v3_selector_advance_prepolicy_mainonly_authenticates_before_migration | :159 | ci.yml:473 |

CI step 保持 `.github/workflows/ci.yml:449`「H1 正式准入三层对拍与调用路径锁（#1481 S1）」原 release h1_ 实跑与逐名 grep；只在既有逐名清单追加5项，原清单不删，无 ignore。

夹具 `legacy_admission_tests.rs:4–17`：先真实生成3代，再正式 Driver 冷重建留下存在的受管 `.audit-ledger.sqlite`，随后 DROP policy_records 并转 DELETE；无常驻 SQLite 连接，明确 MainOnly（源 -wal/-shm/-journal 全不存在）。`:29–52` 精确逐文件保存主库、-wal/-shm/-journal、台账及其3旁挂，共8份 Option<bytes>，不存在者须仍不存在。`:79–130` 依根 run 实际分派前缀分别执行旧入口，断言具名拒绝及正确选择器提示、full_audit_calls 不增、8文件字节不变；用只读连接再次核未补列、DELETE，核后再比8文件避免检查本身掩盖写入。

正控 `:159`：**同一物理库**在 `:183–191` 先经四种旧入口逐一拒绝并核字节，再交显式 selector advance。子进程仅冻结 native testonly 环境，未增栈；走真实 v3 dispatch→writer_cli，断言 full_audit_calls 不变，资格轨迹 `[1,2,3,4]`（冷 g1..3 + 在线 g4），成功补列、模式 WAL、generation=4、正常关闭。保留父进程对真实单锁运行与 PASS 收据的断言。

**cfg 六格源码核查（非 Rust 编译）**：

| features | 非 test | test |
|---|---|---|
| s_session | LegacyWrite 与 wrapper 两个生产调用者均存在；提前拒绝 cfg 排除，原完整审计及最终 None 同列行为不变；mixed match 齐全 | 新文件不 include，不泄漏增量类型/调用 |
| s_session,s_incremental | 旧入口独立前缀，guard 在 schema 及迁移授权前；v3 None 不变；所有入参引用就地借用，无大按值类型或新闭包类型推断 | 新5锁仅 test+s_incremental+unix；test_prepare 所在helper显式 cfg(test)；子进程只在测试中；显式 String 避免 Extend 推断歧义 |
| s_session,s_incremental,s_capture_native_testonly | 同上；新生产逻辑不按 native testonly 变更 | 四负例直接真实入口；正控独立子进程选择 CapturedNative，不污染共享环境；同一源码锁及判据 |

实际执行收据：改动5个 Rust 实装/新测试文件 `rustfmt --edition 2021 --check --config skip_children=true` 通过（audit_owner include由主控全局format）；`git diff --check` 通过；R4 `python3 -B -m unittest s_session.tests.test_tb01c_r4_anchor_lock -v` 3 tests 通过；`python3 -B .github/scripts/check-prod-no-test-hooks.py` violations=0。静态核5新增名在 H1 CI 清单各唯一且无 ignore。禁串生成/一致性由主控统一执行，避免并行生成覆盖。未运行 cargo/rustc，六格 -D warnings 与5个Rust锁均待精确head CI。

### P2：冷点之前包含源修订与历史插入

扩展原 `s_session_v3/audit_owner/admission_tests.rs:157` 的 `h1_admission_cold_prefixes_match_full_canonical`，保留原 g0..g3 升序场景，并在同一测试增加 g0..g5 场景，不增加嵌套 Driver 大栈帧或第二组同时存活的 Driver：

| 代 | 正式接纳输入 | 要锁住的变化 |
|---|---|---|
| g1 | a，坐标0，revision=1，OHLC=(6,12,3,7) | 初始历史 |
| g2 | b，坐标20，revision=1 | 第二个历史坐标 |
| g3 | a，坐标0，revision=2，OHLC=(4,9,1,6) | 同身份同坐标的真实源修订，载荷也改变 |
| g4 | c，坐标10，revision=1 | 插在0和20之间的历史插入，不能退化成升序追加 |
| g5 | d，坐标30，revision=1 | g4 已冷开后再真实追加至少一代 |

`:183–187` 每代都要求 `Committed`、真实 meta generation 与 `full_audit_calls` 不增。`:189–214` 在 g4 冷点之前逐行核 raw 原件确实保留 a/rev1、b/rev1、a/rev2、c/rev1 及相应源坐标。`:216` 的独立 full canonical oracle 在计数区间外，在线与冷开结果都以完整 canonical 字节比较；`:223–239` 冷开期间旧审计计数必须不变，资格 trace 必须从 g1 逐代到当前代。g5 的在线续写与再次冷开继续同样对拍，不只比较 fingerprint。

未构造坐标互换。`s_session_v3/raw_input.rs:154–160` 的正式生产门在同一业务身份修订换坐标时返回 `IdentityConflict`，原文明确“同一业务身份不得换源位置”；不绕过接纳门制造不可达历史。新锁仍沿正式 Driver 入口，源修订和历史插入均合法。

原测试名与原 CI 逐名位置 `.github/workflows/ci.yml:456` 保持。父 `construction_tests` 为 test+s_incremental，include 另受 unix 门；无新增 helper/import/allow/ignore。Python 字节对照确认 admission_tests.rs 除此一个测试体外全文件与 HEAD 相同，原 S1K 的交接/临时失败/重启锁及全部断言保持。未改 46M 已审 PASS 的冷 seed 语义；本机不能执行这把 Rust 锁，最终对拍结果待 CI。

### S1L native debug 残余栈：原件、根因与隔离修复

原件 `/Volumes/AgentStorage/h1-recover45c/ci-41369194207/125035893237.log`：`:939–945` 明写 unoptimized + debuginfo 与 535 tests；`:1355` S1J 目标 `h1_admission_cold_handoff_foreign_sqlite_commit_rejects ... ok`；`:1475–1476` 则是 `h1_admission_driver_cold_recover_rebuilds_ledger_bitwise` 栈溢出；`:1480` 为全部 535 个 `--exact` 名的 SIGABRT 命令。日志没有逐步骤机器栈回溯，不能声称已定位撞 guard 的指令，也不能把这个失败写成 S1J 目标仍失败。失败收据锚 c5a1ba0d；本次源码对拍以当前 S1K HEAD 614823a0 为基线。

新增固定负担是按值拥有的外壳及未执行后半段的 debug 移动槽：`driver.rs:837` 的受管 recover 原帧同时持 context、Runtime、owner、大 `Result<(AuditConnection,Duration)>`；`audit_owner.rs:526` 原长函数同时含冷 fold/seed 与后续 owner、两把 AuditTxn、Ended、Self 返回槽。冷认证尚未进入后半段，也会为这些类型留槽。`audit_owner.rs:324` 的 AuditTxn 实数包含一个 PendingRawState、**两个** CompleteCandidate 和 Recording，绝非“小型参数”；AuditConnection (`:257`) 还内含 CommittedRawState 和 CompleteCandidate。三代/五代历史均是循环，无新无界自调用；本轮不把 codec 原有有界树遍历当作新增递归。

| 最终文件:行 | 隔离及逐句合同 |
|---|---|
| `rust/src/bin/s_session_v3/audit_owner.rs:526` | `open_rebuilt_history_into` 仅经 `&mut Option<Self>` 移交 owner，返回 Duration 小值；正式 Driver/recover 两调用点输出槽均初始 None。原认证/seed/外槽失败清理整个业务块逐字保留，原started传入finish，在最终swap后、converted等局部析构前取elapsed；保持原tuple的移交→计时→局部析构顺序。 |
| 同文件 `:740` | 把第一写连接、转换、迁移、两锁、seed_rebuilt_history、adopt、源连续性复核、正常/拒绝关闭整体移进 `finish_rebuilt_history`。冷 fold/seed 执行时这些 owner/事务槽不再在它的共用帧里。所有回调、deadline、错误先后、candidate/construction 消费顺序不动。 |
| 同文件 `:910` | `initialize_rebuilt_owner` 独帧消费 `Self::new` 的按值构造结果；浅构造返回后才进交接事务。finish 本地只持 owner_out 一槽与借用，最终 h1_close 成功且 require_historical_certificate 置真后才 swap 交给调用方；错误时仍先析构 owner，再由上层析构冷载荷。 |
| `rust/src/bin/s_session_v3/driver.rs:582` | 正式启动直接提供 None 槽，取消大 `Result<Self>` 解包链；后续 startup、Driver 装配及 pending 恢复次序不动。 |
| 同文件 `:837/:867/:881` | recover 的 context/Runtime 构造及借槽开库放 `recover_rebuilt_history`；该 helper 只返回 Value。Runtime/context **仍覆盖** conn.recover 与显式 drop(conn)，上层 scope guard 仍覆盖全部过程；没有提早析构冷台账或父账。 |

实际执行 `/private/tmp/s1l-stack-semantics.py`：① 认证/seed 块逐字相同；② 写交接与失败收口块仅归一化已借用的 canonical/cold_runtime 接收者和 owner 构造三行后逐字相同；③ 两个 None 输出槽与唯一接口、recover 的 context→runtime→开库→recover→drop(conn) 顺序锁通过。独立非作者逐句阅读也未发现语义、cfg 或借用阻断。未改 construction_state/into_owner、S1G/S1J 来源核、seed 等价判据、任何费用/性能/资源帽；未增加 Box 或其他分配，未设置栈大小、修改 workflow、ignore 或删断言。

### 535 项穷举方法与源码尺寸估算边界

从 `:1480` 原始 `--exact` 命令重抽集合，与保存的 535 行名单逐字同序、去重后仍 535；SHA-256 为 `be1e8aec09b4bd51a4e05198213a49bf223f5f70829c99156fc1db9c825a4f8f`。每个名字均重新定位到唯一源码定义。表生成脚本只负责枚举、定位和排表，**不是 Rust 调用图分析器或栈测量工具**。入口映射另人工核了全仓 `open_rebuilt_history_into`（仅 Driver 两处）、retention startup、scoped retention recover、open_for_serve_with_identity、open_admitted_history、component/cold_for_request 的全部调用点和测试 helper；非 h1/b2/p6 名称同样纳入。

64 位外壳模型沿 S1G/S1J：NativeSourceState 约 5.7 KiB（23 Map、11 Tree、3 Order）；ColdRawState/CommittedRawState 取 6–8 KiB；含 PendingSet/RecoverHandoff 的 PendingRawState 取 8–12 KiB；CompleteCandidate 连 AuditedState 的最大 Unique 变体、Q/slot/capture 外壳取 6–10 KiB；AuditTxn 的两候选+pending+Recording/事务取 24–40 KiB；AuditConnection 取 16–24 KiB；Runtime 连 generator/auxiliary 取 5–9 KiB，context 取 1–3 KiB。这里计容器壳、枚举最大变体和许可壳，**不把树/Vec 堆中的历史内容按条数算栈**。`None` 仍预留最大变体。

为逐项估算，按拥有值约 **2–8 份** debug 参数/局部/Result/移动槽，给各函数族一份工程包络后求和；引用只记指针，并将测试/helper 外壳另留 384 KiB（低层锁 128 KiB）。这是假设，不是编译器保证；不能由 Rust 源码证明 LLVM 必定落在这个份数区间。下面列的是**可归属到源码外壳的模型帧和**，不含无法在禁 cargo/rustc 条件下测定的旧 codec 树遍历、libtest/SQLite 调用栈及机器 spill。采用默认 libtest 常见 2 MiB 作对照，把 1.5 MiB 作为模型分诊上限，另留 512 KiB 给未测部分；**未测部分也没有已证明的 512 KiB 上界**。因此表的“低于分诊线”不能写成 Rust 已通过、真实栈安全或 535 全绿，最终由精确新 head native debug CI 裁定。

| 链代号 | 源码可达最深阶段（顺序阶段取最大，不把先后退出的帧相加） | 修后模型帧和 | 是否处理 |
|---|---|---:|---|
| H | Driver 冻结 context→open_rebuilt_history_into→certify_cold/read_session(unit)→cold_for_request_rebuilding/cold_captured→fold→advance_cold_sources/audit_source_step→原 codec；与 finish→两 AuditTxn/seed_rebuilt→adopt 分支取 max | 约 1,376 KiB＝测试/helper384+Driver192+冷载荷192+只读门64+state192+fold192+同核来源160 | 原长帧模型约 1,888 KiB，超1.5MiB；共用正式入口已拆帧/借槽。finish 分支另算约1,344 KiB＝测试384+Driver192+外层冷载荷192+owner_out128+单阶段事务/移动槽256+seed/adopt192；H取max(1,376,1,344)=1,376。 |
| R | scoped retention recover→recover_rebuilt_history→H（ledger fresh、seed/adopt）→同owner recover；后一个事务与前一个深 fold 取 max | 约 1,440 KiB＝max(H 1,376+scope薄包装64, 后续同owner Recover 1,088)；后者约测试384+helper context/Runtime与owner192+txn256+候选/pending128+浅包装128 | 原模型约 2,016 KiB，且日志实际溢出；共用 H 和 recover 包装均处理。 |
| H0 | 两 native_bootstrap identity 测试→open_for_serve_with_identity→cold_context→open_rebuilt_history_into→prepare_parent/metadata预算拒门；不到 fold/seed | 约 832 KiB＝测试384+Driver192+冷载荷192+预算门64 | 共用借槽/分帧覆盖；没有为测试提高 retention=1。 |
| C | h1_certified_owner/h1_qualified_owner 或 AdmissionFixture::open→open_certified_history_component/seed_historical_certificate→cold_for_request→cold_captured；ledger=None，之后可能 qualified fold；与真实在线阶段取max | 约 1,056 KiB＝测试384+组件owner/txn192+候选64+state192+fold96+原来源128 | 未超模型分诊线；S1G/S1J组件拆帧保留，本轮不改其判据。 |
| S | HistoricalCertificate::cold→cold_captured→fold；ledger=None，不走正式冷 seed | 约 832 KiB＝测试384+state192+fold128+原来源128 | 原组件壳；未超模型分诊线。 |
| B | complete_driver/recover_lock_driver→同 owner AuditConnection::recover→完整候选绑定/PendingRawState 换代→真实 Recover commit；不走 supervisor H/R | 约 896 KiB＝测试384+txn192+候选128+pending64+原调用128 | 未超模型分诊线；没有误算成新 H1 冷重建。 |
| P | AuditConnection::open 或 Fixture::publish→在线来源/真实owner→prepare/ledger/旧 full oracle；测试存在多个分支时保守取这几段最大 | 约 928 KiB＝测试384+txn192+Runtime64+在线共同核160+原codec128 | 无新增正式冷 seed；按共同核最大包络纳入，不按测试名字排除。 |
| O | Driver default/未冻结 context 或 request_audit 完整旧cold→原完整 oracle；不是 source_cache_enabled 的 H/R | 约 1,056 KiB，保守取旧完整来源与 C/P 模型最大 | 无本轮新增 H1 深链；模型未超分诊线，不冒充运行PASS。 |
| L | 直接 source receipt/ledger stage/bind/资格闭合，没有正式 cold Runtime/adopt | 约 384 KiB＝测试128+收据128+台账128 | 不含新冷seed；低层包络未超分诊线。 |
| V | 只读物理/缓存登记/替换/费用门即拒，未进入cold来源深链 | 约 192 KiB＝测试128+只读门64 | 保留只读门。 |
| X | 无上述 S1 冷/recover/重建/seed 调用边的低层/既有完整核；全仓调用点穷举及非作者复核排除，不用名称猜测可达性 | 新增链 0；既有帧未测 | 不宣称既有代码真实零栈或所有测试已PASS。 |

两个 H0 还揭示一项**既有静态测试风险**：`native_bootstrap_idle_identity_hits_then_drops_and_ready_is_new_cold_default_paths_unchanged` 用 incremental_retention_bytes=1 后仍 unwrap；`cold_context` 冻结1字节，`prepare_parent(None)`→`Budget::bootstrap/build` 的父账元数据费大于1，按现有资源合同会先拒绝。另一项 epoch/profile/idle/illegal_request 测试也带1，但部分分支本就期望拒绝。没有为使它们通过而改资源帽、断言或跳过测试。该发现与真正栈溢出分开记录；本轮 ready 只表示指定修复候选可审，不代表这535项已全绿。

以下每行保留原完整测试名、定位、最深链模型及处理；P/O 等多分支采用保守最大阶段而不声称取得动态回溯。X 逐项列出以便核排除面。所有数字均带上述未实测边界。

| 原 --exact 测试名 | 当前定义（相对 rust/src/bin/） | 最深链 | 模型帧和/处理 |
|---|---|---|---|
| `cache::tests::mixed_cache_failed_publication_restores_prior_parent_tables` | `s_session_v2/cache.rs:2714` | X | 新增链0／旧帧未测 |
| `cache::tests::mixed_cache_second_eviction_plan_failure_preserves_both_old_groups` | `s_session_v2/cache.rs:2672` | X | 新增链0／旧帧未测 |
| `cache::tests::remove_cert_owned_insert_failure_keeps_payload_accounted` | `s_session_v2/cache.rs:2759` | X | 新增链0／旧帧未测 |
| `content_manifest::tests::native_manifest_copy_failure_preserves_inputs_and_does_not_validate_early` | `s_session_v2/content_manifest.rs:2141` | X | 新增链0／旧帧未测 |
| `content_manifest::tests::native_manifest_metadata_and_roots_outlive_scope_and_drop_independently` | `s_session_v2/content_manifest.rs:2111` | X | 新增链0／旧帧未测 |
| `content_manifest::tests::native_manifest_originals_tree_permit_is_incremental_without_whole_tree_coexistence` | `s_session_v2/content_manifest.rs:2251` | X | 新增链0／旧帧未测 |
| `content_manifest::tests::native_manifest_recorded_body_and_puts_keep_original_permits_after_scope` | `s_session_v2/content_manifest.rs:2196` | X | 新增链0／旧帧未测 |
| `content_manifest::tests::native_publication_material_owner_normal_puts_keep_base_and_read_targets_cold` | `s_session_v2/content_manifest.rs:2304` | X | 新增链0／旧帧未测 |
| `content_manifest::tests::r9a_manifest_scope_preserves_identity_and_accounts_outputs` | `s_session_v2/content_manifest.rs:2586` | X | 新增链0／旧帧未测 |
| `content_producer::tests::native_local_fractal_relations_preserve_full_material_and_render_with_fewer_puts` | `s_session_v2/content_producer.rs:5926` | X | 新增链0／旧帧未测 |
| `content_publication::tests::admitted_sql_text_and_publication_path_keep_their_own_permits` | `s_session_v2/content_publication.rs:1549` | X | 新增链0／旧帧未测 |
| `content_publication::tests::native_publication_begin_and_consumers_share_parent_scope` | `s_session_v2/content_publication.rs:2820` | X | 新增链0／旧帧未测 |
| `content_publication::tests::native_publication_fields_exact_declaration_failures_and_overlay_not_cached` | `s_session_v2/content_publication.rs:2496` | X | 新增链0／旧帧未测 |
| `content_publication::tests::native_publication_fields_retains_per_cut_rejection_matrix` | `s_session_v2/content_publication.rs:2573` | X | 新增链0／旧帧未测 |
| `content_publication::tests::native_publication_fields_same_roots_output_and_fewer_real_lookups` | `s_session_v2/content_publication.rs:2381` | X | 新增链0／旧帧未测 |
| `content_publication::tests::native_publication_fresh_sql_vs_current_capture_diagnostic` | `s_session_v2/content_publication.rs:1598` | X | 新增链0／旧帧未测 |
| `content_publication::tests::ordinary_proof_publication_committed_active_fields_rejected_at_withdrawal_cut` | `s_session_v2/content_publication.rs:2234` | X | 新增链0／旧帧未测 |
| `content_publication::tests::ordinary_proof_publication_complete_outputs_two_frontiers_and_release` | `s_session_v2/content_publication.rs:2088` | X | 新增链0／旧帧未测 |
| `content_publication::tests::ordinary_proof_publication_exact_declarations_never_hit_good_root_entry` | `s_session_v2/content_publication.rs:2332` | X | 新增链0／旧帧未测 |
| `content_publication::tests::ordinary_proof_publication_keeps_per_cut_rejection_matrix` | `s_session_v2/content_publication.rs:2178` | X | 新增链0／旧帧未测 |
| `content_store::tests::native_insert_actual_constraint_and_insert_deny_errors_poison_unchanged` | `s_session_v2/content_store.rs:1722` | X | 新增链0／旧帧未测 |
| `content_store::tests::native_insert_counter_witness_rejects_every_unproven_effect` | `s_session_v2/content_store.rs:1883` | X | 新增链0／旧帧未测 |
| `content_store::tests::native_insert_effects_cover_main_temp_indirect_delete_replace_and_ignore` | `s_session_v2/content_store.rs:1551` | X | 新增链0／旧帧未测 |
| `content_store::tests::native_insert_hidden_loaded_trigger_uses_actual_effects` | `s_session_v2/content_store.rs:1595` | X | 新增链0／旧帧未测 |
| `content_store::tests::native_insert_ignored_pragma_metadata_is_error_including_temp_replace_shadow` | `s_session_v2/content_store.rs:1777` | X | 新增链0／旧帧未测 |
| `content_store::tests::native_insert_loaded_shape_and_temp_shadow_reject_preservation` | `s_session_v2/content_store.rs:1641` | X | 新增链0／旧帧未测 |
| `content_store::tests::native_insert_pk_replace_misread_missing_removes_only_target_proof` | `s_session_v2/content_store.rs:1827` | X | 新增链0／旧帧未测 |
| `content_store::tests::native_insert_preserves_only_prior_other_reads_and_keeps_put_cold` | `s_session_v2/content_store.rs:1511` | X | 新增链0／旧帧未测 |
| `content_store::tests::native_insert_trigger_replace_delete_ignore_still_counts_inner_insert` | `s_session_v2/content_store.rs:1695` | X | 新增链0／旧帧未测 |
| `content_store::tests::native_insert_unqualified_unique_replace_keeps_old_full_clear` | `s_session_v2/content_store.rs:1669` | X | 新增链0／旧帧未测 |
| `content_store::tests::native_read_lease_isolation_activity_and_actual_interrupt` | `s_session_v2/content_store.rs:1978` | X | 新增链0／旧帧未测 |
| `content_store::tests::native_read_lease_put_trigger_errors_and_poison_do_not_repair` | `s_session_v2/content_store.rs:1899` | X | 新增链0／旧帧未测 |
| `content_store::tests::native_read_lease_sql_f_headers_and_cold_lifecycle` | `s_session_v2/content_store.rs:1435` | X | 新增链0／旧帧未测 |
| `content_store::tests::native_read_lease_wal_snapshot_and_upgrade_failure` | `s_session_v2/content_store.rs:2063` | X | 新增链0／旧帧未测 |
| `object_diff::tests::admitted_changes_keep_permits_past_scope_and_release_on_drop` | `s_session_v3/object_diff.rs:889` | X | 新增链0／旧帧未测 |
| `object_diff::tests::admitted_diff_partial_output_growth_failure_releases_all_buffers` | `s_session_v3/object_diff.rs:1015` | X | 新增链0／旧帧未测 |
| `object_diff::tests::admitted_diff_preserves_lazy_read_order_and_budget_failure_atomicity` | `s_session_v3/object_diff.rs:926` | X | 新增链0／旧帧未测 |
| `object_diff::tests::admitted_diff_rejects_two_allocation_domains_before_growth` | `s_session_v3/object_diff.rs:988` | X | 新增链0／旧帧未测 |
| `object_diff::tests::admitted_slice_and_indexed_share_order_and_failure_semantics` | `s_session_v3/object_diff.rs:828` | X | 新增链0／旧帧未测 |
| `staged_nodes::staged_native_lease_uses_overlay_without_sql_material_cache_and_poison_cannot_retry` | `s_session_v2/staged_nodes.rs:745` | X | 新增链0／旧帧未测 |
| `staged_nodes::staged_nodes_capacity_counts_unique_owned_vecs_without_per_put_heap_sampling` | `s_session_v2/staged_nodes.rs:538` | X | 新增链0／旧帧未测 |
| `staged_nodes::staged_nodes_flush_counts_rows_and_traverses_old_parent_to_new_child` | `s_session_v2/staged_nodes.rs:572` | X | 新增链0／旧帧未测 |
| `staged_nodes::staged_nodes_preserve_frame_and_current_sql_error_gates` | `s_session_v2/staged_nodes.rs:624` | X | 新增链0／旧帧未测 |
| `staged_nodes::staged_scoped_copies_hold_distinct_permits_until_payload_drop` | `s_session_v2/staged_nodes.rs:660` | X | 新增链0／旧帧未测 |
| `staged_nodes::staged_selective_cut_follows_sql_parent_and_keeps_unselected_payload_charged` | `s_session_v2/staged_nodes.rs:695` | X | 新增链0／旧帧未测 |
| `tb02a_facts::catalog_axis_owner_tests::catalog_root_outlives_scope_and_every_capacity_failure_releases_partial_axes` | `s_session_v2/tb02a_facts.rs:1664` | X | 新增链0／旧帧未测 |
| `tests::native_batch_header_shares_original_text_permits_until_last_drop` | `s_structure_session.rs:8277` | X | 新增链0／旧帧未测 |
| `tests::native_cold_raw_ohlc_and_revision_matches_plain` | `s_structure_session.rs:8467` | X | 新增链0／旧帧未测 |
| `tests::native_cold_raw_sql_json_hash_receipt_revision_matches_plain` | `s_structure_session.rs:8357` | X | 新增链0／旧帧未测 |
| `tests::native_raw_column_errors_precede_ohlc_cardinality_and_earlier_row_faults` | `s_structure_session.rs:8380` | X | 新增链0／旧帧未测 |
| `v3::admission_anchor::ram::tests::mixed_ram_parser_refusal_allocates_no_error_string` | `s_session_v3/admission_anchor/ram.rs:620` | X | 新增链0／旧帧未测 |
| `v3::admission_provider::control_lock_tests::h1_ledger_cache_conversion_stamp_rejects_changed_main_or_wal_frames` | `s_session_v3/admission_provider.rs:733` | V | 192 KiB／只读拒门 |
| `v3::admission_provider::control_lock_tests::h1_ledger_cache_discard_releases_only_cache_pins_before_rebuild` | `s_session_v3/admission_provider.rs:711` | V | 192 KiB／只读拒门 |
| `v3::admission_provider::control_lock_tests::h1_ledger_cache_replacement_or_new_member_is_rejected_before_discard` | `s_session_v3/admission_provider.rs:688` | V | 192 KiB／只读拒门 |
| `v3::admission_provider::control_lock_tests::h1_ledger_cache_uses_original_store_cap_before_discard` | `s_session_v3/admission_provider.rs:666` | V | 192 KiB／只读拒门 |
| `v3::audit_ledger::tests::p6_raw_partition_cannot_reopen_or_append_to_complete_certified_ledger` | `s_session_v3/audit_ledger.rs:1332` | L | 384 KiB／低层闭合 |
| `v3::audit_ledger::tests::p6_two_object_generations_commit_disjoint_closure_deltas` | `s_session_v3/audit_ledger.rs:1557` | L | 384 KiB／低层闭合 |
| `v3::audit_ledger::tests::p6_verified_units_difference_keeps_mode_and_first_generation` | `s_session_v3/audit_ledger.rs:1406` | L | 384 KiB／低层闭合 |
| `v3::audit_owner::audit_owner_ended_cannot_promote_earlier_context_or_fixture_life` | `s_session_v3/audit_owner.rs:3685` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::audit_owner_q_itemized_normal_cold_growth_and_typed_limit` | `s_session_v3/audit_owner.rs:4647` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::construction_set::tests::owner_set_append_skips_shared_prefix_and_preserves_duplicate_occurrence` | `s_session_v3/construction_set.rs:1020` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::construction_set::tests::owner_set_diff_budget_failure_does_not_reissue_success` | `s_session_v3/construction_set.rs:1178` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::construction_set::tests::owner_set_full_material_diff_covers_witness_observation_and_nested_requests` | `s_session_v3/construction_set.rs:1104` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::construction_set::tests::owner_set_sequence_exhaustive_occurrences_and_order` | `s_session_v3/construction_set.rs:955` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::construction_set::tests::owner_set_wrong_root_owner_and_unclosed_domain_reject` | `s_session_v3/construction_set.rs:1145` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::construction_set::tests::sequence_delta_retains_charge_and_drop_does_not_refill_next_request` | `s_session_v3/construction_set.rs:932` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::construction_tests::b2_native_carry_budget_rejects_without_losing_committed_frontier` | `s_session_v3/audit_owner.rs:6813` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::b2_native_formal_failure_rollback_and_commit_failure_revoke_all_state` | `s_session_v3/audit_owner.rs:6637` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::b2_native_owner_restore_bounded_multicut_matches_writer_and_rebuild` | `s_session_v3/audit_owner.rs:6352` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::b2_native_owner_restore_wrong_library_layout_rejects_in_append_owner_cut` | `s_session_v3/audit_owner.rs:6485` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::b2_native_source_future_duplicates_preserve_earlier_actual_fault` | `s_session_v3/audit_owner/b2_source_fault_tests.rs:580` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::construction_tests::b2_native_source_global_dependencies_match_full_oracle_and_rollback` | `s_session_v3/audit_owner/b2_source_fault_tests.rs:568` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::construction_tests::b2_native_source_phase_errors_precede_later_bad_material` | `s_session_v3/audit_owner/b2_source_fault_tests.rs:557` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::construction_tests::b2_native_source_unsettled_rechecks_added_nonzero_group_before_partition` | `s_session_v3/audit_owner/b2_source_fault_tests.rs:589` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::construction_tests::b2_native_two_nonempty_formal_rounds_match_full_oracle_at_every_commit` | `s_session_v3/audit_owner.rs:6578` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::b2_native_warm_cold_and_ordinary_match_all_rows_nodes_roots_receipts_and_counts` | `s_session_v3/audit_owner.rs:6103` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::b2_native_warm_missing_or_stale_originals_reject_before_full_source` | `s_session_v3/audit_owner.rs:6758` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::cross_request_actual_commit_failure_cannot_promote` | `s_session_v3/audit_owner.rs:7049` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::cross_request_publication_tail_failure_cannot_commit` | `s_session_v3/audit_owner.rs:7045` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::cross_request_ready_first_publication_preserves_real_parent` | `s_session_v3/audit_owner.rs:7041` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_admission_cold_handoff_foreign_sqlite_commit_rejects` | `s_session_v3/audit_owner/admission_tests.rs:381` | H | 1,376 KiB／共用H已拆帧 |
| `v3::audit_owner::construction_tests::h1_admission_cold_negative_classes_match_full_oracle` | `s_session_v3/audit_owner/admission_tests.rs:271` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_admission_cold_prefixes_match_full_canonical` | `s_session_v3/audit_owner/admission_tests.rs:157` | H | 1,376 KiB／共用H已拆帧 |
| `v3::audit_owner::construction_tests::h1_admission_cold_real_wire_short_budget_rejects_before_write` | `s_session_v3/audit_owner/admission_tests.rs:800` | H | 1,376 KiB／共用H已拆帧 |
| `v3::audit_owner::construction_tests::h1_admission_driver_cold_preserves_original_node_record_gates` | `s_session_v3/audit_owner/admission_tests.rs:724` | H | 1,376 KiB／共用H已拆帧 |
| `v3::audit_owner::construction_tests::h1_admission_driver_cold_recover_rebuilds_ledger_bitwise` | `s_session_v3/audit_owner/admission_tests.rs:595` | R | 1,440 KiB／H+recover已拆帧 |
| `v3::audit_owner::construction_tests::h1_admission_foreign_sqlite_changes_retire_writer` | `s_session_v3/audit_owner/admission_tests.rs:347` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_admission_formal_accept_begin_prepare_recover_never_full_audit` | `s_session_v3/audit_owner/admission_tests.rs:461` | R | 1,440 KiB／H+recover已拆帧 |
| `v3::audit_owner::construction_tests::h1_admission_hot_rollback_journal_rejects_before_rw_open` | `s_session_v3/audit_owner/admission_tests.rs:1254` | H | 1,376 KiB／共用H已拆帧 |
| `v3::audit_owner::construction_tests::h1_admission_mainonly_nonwal_prepolicy_preserves_domain` | `s_session_v3/audit_owner/admission_tests.rs:1193` | H | 1,376 KiB／共用H已拆帧 |
| `v3::audit_owner::construction_tests::h1_admission_nonwal_conversion_handoff_foreign_commit_rejects` | `s_session_v3/audit_owner/admission_tests.rs:1312` | H | 1,376 KiB／共用H已拆帧 |
| `v3::audit_owner::construction_tests::h1_admission_normal_exit_same_database_new_process_restarts` | `s_session_v3/audit_owner/admission_tests.rs:1055` | H | 1,376 KiB／共用H已拆帧 |
| `v3::audit_owner::construction_tests::h1_admission_online_own_commits_match_full_canonical` | `s_session_v3/audit_owner/admission_tests.rs:322` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_admission_ordinary_prepolicy_bad_history_rejected_before_migration` | `s_session_v3/audit_owner/admission_tests.rs:1127` | O | 1,056 KiB／C建库+Ordinary旧核最大 |
| `v3::audit_owner::construction_tests::h1_admission_prepolicy_bad_history_rejected_before_migration` | `s_session_v3/audit_owner/admission_tests.rs:861` | H | 1,376 KiB／共用H已拆帧 |
| `v3::audit_owner::construction_tests::h1_certified_commit_failure_releases_controls_before_return` | `s_session_v3/audit_owner/history_clamp_tests.rs:652` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_certified_missing_candidate_and_mode_switch_are_sticky_stale` | `s_session_v3/audit_owner/history_clamp_tests.rs:482` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_certified_online_consumes_source_receipt_and_matches_cold` | `s_session_v3/audit_owner/history_clamp_tests.rs:373` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_certified_online_without_source_receipt_rejects_without_full_fallback` | `s_session_v3/audit_owner/history_clamp_tests.rs:432` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_certified_open_empty_and_reopen_from_originals` | `s_session_v3/audit_owner/history_clamp_tests.rs:460` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_certified_open_failures_leave_no_candidate_or_write` | `s_session_v3/audit_owner/history_clamp_tests.rs:513` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_complete_canonical_empty_history_is_deterministic` | `s_session_v3/audit_owner/history_clamp_tests.rs:190` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_control_bytes_change_in_same_connection_is_stale` | `s_session_v3/audit_owner/history_clamp_tests.rs:86` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_control_observation_budget_failure_retires_without_writing` | `s_session_v3/audit_owner/history_clamp_tests.rs:623` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_control_originals_are_retained_on_the_same_parent_and_released` | `s_session_v3/audit_owner/history_clamp_tests.rs:565` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_control_parent_rebind_precedes_warm_request_observation` | `s_session_v3/audit_owner/history_clamp_tests.rs:591` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_external_checkpoint_is_not_own_effect` | `s_session_v3/audit_owner/history_clamp_tests.rs:294` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_main_identity_replacement_is_stale` | `s_session_v3/audit_owner/history_clamp_tests.rs:53` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_missing_commit_receipt_retires_after_real_commit` | `s_session_v3/audit_owner/history_clamp_tests.rs:337` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_other_connection_commit_is_sticky_stale` | `s_session_v3/audit_owner/history_clamp_tests.rs:44` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_own_checkpoint_and_wal_reset_preserve_full_history` | `s_session_v3/audit_owner/history_clamp_tests.rs:239` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_partial_checkpoint_waits_for_reader_then_resets` | `s_session_v3/audit_owner/history_clamp_tests.rs:254` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_qualified_cold_rejects_identity_rule_root_and_unit_mismatch` | `s_session_v3/audit_owner/history_clamp_tests.rs:899` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_qualified_cold_rejects_missing_partial_or_foreign_qualification` | `s_session_v3/audit_owner/history_clamp_tests.rs:862` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_qualified_ledger_commit_failure_and_external_identity_write_are_stale` | `s_session_v3/audit_owner/history_clamp_tests.rs:991` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_qualified_online_consumes_real_ledger_and_cold_rebuild_matches` | `s_session_v3/audit_owner/history_clamp_tests.rs:798` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_qualified_stage_rejects_externally_rewritten_fold_digest_or_rule_revision` | `s_session_v3/audit_owner/history_clamp_tests.rs:1064` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_qualified_transition_short_budget_rejects_before_write_and_releases` | `s_session_v3/audit_owner/history_clamp_tests.rs:1128` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_raw_suffix_probe_detects_linear_skip_regression` | `s_session_v3/audit_owner/history_clamp_tests.rs:706` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_real_own_accept_begin_publish_continues_same_life` | `s_session_v3/audit_owner/history_clamp_tests.rs:24` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_real_write_does_not_explain_external_main_change_before_checkpoint` | `s_session_v3/audit_owner/history_clamp_tests.rs:309` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_request_closure_real_owner_multicut_reuses_resolved_roots` | `s_session_v3/audit_owner.rs:6339` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_roots_json_matches_real_cert_roots` | `s_session_v3/audit_owner/history_clamp_tests.rs:1213` | C | 1,056 KiB／保留既有隔离 |
| `v3::audit_owner::construction_tests::h1_schema_and_domain_changes_are_stale` | `s_session_v3/audit_owner/history_clamp_tests.rs:99` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_sqlite_commit_failure_does_not_advance_clamp` | `s_session_v3/audit_owner/history_clamp_tests.rs:129` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_unchanged_snapshot_survives_read_end` | `s_session_v3/audit_owner/history_clamp_tests.rs:14` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_undeclared_write_rejected_before_sql_and_commit` | `s_session_v3/audit_owner/history_clamp_tests.rs:112` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_unexplained_main_stat_is_stale` | `s_session_v3/audit_owner/history_clamp_tests.rs:73` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_wal_identity_replacement_is_stale` | `s_session_v3/audit_owner/history_clamp_tests.rs:63` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::h1_zero_autocheckpoint_keeps_main_and_wal_round` | `s_session_v3/audit_owner/history_clamp_tests.rs:278` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::owner_cold_candidate_born_from_grant_before_first_allocation` | `s_session_v3/audit_owner.rs:7239` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::owner_raw_commit_hook_failure_never_promotes` | `s_session_v3/audit_owner.rs:7220` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::owner_raw_external_commit_rejects_equal_length_material` | `s_session_v3/audit_owner.rs:7276` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::owner_raw_formal_accept_append_commits_before_nonempty_delta` | `s_session_v3/audit_owner.rs:5951` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::owner_raw_formal_accept_probe_is_scoped_to_owner_and_thread` | `s_session_v3/audit_owner.rs:5886` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::owner_raw_formal_accept_revision_commits_before_nonempty_delta` | `s_session_v3/audit_owner.rs:5955` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::owner_raw_nonempty_consumer_failure_stales_after_formal_accept_commit` | `s_session_v3/audit_owner.rs:5959` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::owner_raw_pending_state_holds_its_fee_until_drop` | `s_session_v3/audit_owner.rs:7259` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::owner_raw_real_effects_only_promote_after_real_commit` | `s_session_v3/audit_owner.rs:7132` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::owner_raw_rollback_unknown_write_and_unclosed_commit_revoke` | `s_session_v3/audit_owner.rs:7168` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::owner_raw_schema_read_domain_and_consumer_failure_revoke` | `s_session_v3/audit_owner.rs:7307` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::owner_set_real_publish_commit_failure_revokes_everything` | `s_session_v3/audit_owner.rs:7124` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::owner_set_real_publish_stage_effects_commit_promotes` | `s_session_v3/audit_owner.rs:7120` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::owner_set_real_publish_unscoped_same_value_update_rejects` | `s_session_v3/audit_owner.rs:7128` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::p6_native_source_context_preserves_real_owner_multicut_contract` | `s_session_v3/audit_owner.rs:6331` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::publication_resource_tail_failure_prevents_actual_sql_commit` | `s_session_v3/audit_owner.rs:7058` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::construction_tests::publication_single_parent_scope_reaches_raw_runtime_material_and_validation` | `s_session_v3/audit_owner.rs:7054` | P | 928 KiB／共同核包络 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_effects_absent_budget_never_yields_coverage_receipt` | `s_session_v3/audit_owner/effects.rs:2488` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_effects_node_insert_and_repeated_noop_use_actual_write_counts` | `s_session_v3/audit_owner/effects.rs:2436` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_effects_strict_schema_rejects_rowid_node_substitute` | `s_session_v3/audit_owner/effects.rs:2475` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_effects_unhooked_without_rowid_write_and_trigger_side_effect_reject` | `s_session_v3/audit_owner/effects.rs:2454` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_set_actual_transaction_progress_cancellation_is_preserved` | `s_session_v3/audit_owner/effects.rs:3192` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_set_distinct_withdrawals_each_consume_one_actual_row` | `s_session_v3/audit_owner/effects.rs:3025` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_set_duplicate_withdrawal_same_final_row_poisons_ordered_seal` | `s_session_v3/audit_owner/effects.rs:3004` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_set_exact_work_cap_lives_through_effects_then_releases_only_its_charge` | `s_session_v3/audit_owner/effects.rs:3108` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_set_failed_source_drops_new_cap_without_erasing_existing_codec_retain` | `s_session_v3/audit_owner/effects.rs:3159` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_set_future_stage_cannot_repair_missing_current_original` | `s_session_v3/audit_owner/effects.rs:2914` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_set_merged_receipts_reject_dropped_misplaced_or_miscounted_event` | `s_session_v3/audit_owner/effects.rs:2792` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_set_none_scope_shares_journal_cap_and_failure_cannot_reset` | `s_session_v3/audit_owner/effects.rs:3085` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_set_ordered_node_receipts_keep_repeated_noop_and_unreachable_nodes` | `s_session_v3/audit_owner/effects.rs:2742` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_set_receipt_events_grow_alone_and_overflow_like_keys` | `s_session_v3/audit_owner/effects.rs:2852` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_set_source_cut_rejects_foreign_only_closure_and_cannot_reissue` | `s_session_v3/audit_owner/effects.rs:3052` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::owner_set_stage_drop_foreign_connection_and_omitted_business_row_reject` | `s_session_v3/audit_owner/effects.rs:2948` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::p2_2_journal_permit_taken_before_recording_allocation` | `s_session_v3/audit_owner/effects.rs:2504` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::p2_2_private_journal_parent_born_before_allocation_and_material_adds_no_fee` | `s_session_v3/audit_owner/effects.rs:2578` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::p3b_start_on_keeps_pre_p2_2_result_categories` | `s_session_v3/audit_owner/effects.rs:2538` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::publication_resource_attachment_is_once_and_foreign_scope_rejects` | `s_session_v3/audit_owner/effects.rs:2695` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::construction_coverage_tests::publication_weak_identity_does_not_block_finish_with_large_parent_retention` | `s_session_v3/audit_owner/effects.rs:2637` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::e_owner_move_tests::e_apply_native_insert_is_ordered_and_releases_only_manual_outer_charge` | `s_session_v3/audit_owner/effects.rs:3244` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::e_owner_move_tests::e_apply_native_replace_and_delete_keep_remaining_rows_owned` | `s_session_v3/audit_owner/effects.rs:3322` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::e_owner_move_tests::e_stage_into_row_keeps_charge_after_header_drop` | `s_session_v3/audit_owner/effects.rs:3214` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::native_journal_only_reserves_the_container_that_grows` | `s_session_v3/audit_owner/effects.rs:2358` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::u2_row_owned_tests::u2_effects_row_owned_accepts_cells_born_under_parent` | `s_session_v3/audit_owner/effects.rs:3408` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::effects::u2_row_owned_tests::u2_effects_row_owned_rejects_plain_cell_or_foreign_parent_without_adopt` | `s_session_v3/audit_owner/effects.rs:3425` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::cross_request_budget_tests::cross_request_denied_growth_preserves_old_payload_and_origin` | `s_session_v3/audit_owner/native.rs:2168` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::cross_request_budget_tests::handoff_active_scope_overflow_and_changed_contract_are_atomic` | `s_session_v3/audit_owner/native.rs:2202` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::cross_request_budget_tests::initial_original_fee_can_be_adopted_only_once` | `s_session_v3/audit_owner/native.rs:2258` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::cross_request_budget_tests::native_growth_across_two_requests_keeps_payload_and_charge_origin` | `s_session_v3/audit_owner/native.rs:2136` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::cross_request_budget_tests::native_reservation_split_merge_rejects_foreign_and_new_origin` | `s_session_v3/audit_owner/native.rs:2071` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::cross_request_budget_tests::old_release_does_not_refill_allowance_with_real_native_payload` | `s_session_v3/audit_owner/native.rs:2104` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::cross_request_budget_tests::old_tree_remove_and_drop_return_only_their_own_origin` | `s_session_v3/audit_owner/native.rs:2240` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::cross_request_budget_tests::permit_split_merge_foreign_source_and_duplicate_consumption_reject` | `s_session_v3/audit_owner/native.rs:2222` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::cross_request_budget_tests::retained_scratch_layout_drops_in_old_origin_after_scope_closes` | `s_session_v3/audit_owner/native.rs:2044` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::lifecycle_repair_v2_tests::business_map_rejects_foreign_parent_before_spare_capacity_or_growth` | `s_session_v3/audit_owner/native.rs:2425` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::lifecycle_repair_v2_tests::expired_budget_rejects_spare_vec_hash_and_tree_replacement` | `s_session_v3/audit_owner/native.rs:2397` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::lifecycle_repair_v2_tests::expired_domain_cannot_adopt_plain_vec_even_without_growth` | `s_session_v3/audit_owner/native.rs:2328` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::lifecycle_repair_v2_tests::frozen_increment_accepts_700_old_and_20_new_with_200_allowance` | `s_session_v3/audit_owner/native.rs:2285` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::lifecycle_repair_v2_tests::frozen_old_pool_must_include_actual_parent_metadata_before_handoff` | `s_session_v3/audit_owner/native.rs:2307` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::lifecycle_repair_v2_tests::insufficient_scope_metadata_keeps_plain_vec_and_original_fee_unchanged` | `s_session_v3/audit_owner/native.rs:2363` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::lifecycle_repair_v2_tests::temporary_native_text_and_codec_capacity_release_on_actual_owner_drop` | `s_session_v3/audit_owner/native.rs:2475` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::r6_codec_work_tests::r6_codec_completed_scope_keeps_readable_bytes_and_rejects_writes` | `s_session_v3/audit_owner/native.rs:29` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::r6_codec_work_tests::r6_codec_parent_metadata_growth_refusal_and_real_drop` | `s_session_v3/audit_owner/native.rs:209` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::r6_codec_work_tests::r6_codec_real_verify_render_lookup_writer_plain_and_scoped_memo` | `s_session_v3/audit_owner/native.rs:63` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::r6_codec_work_tests::r6_codec_render_output_refusal_keeps_write_cause` | `s_session_v3/audit_owner/native.rs:177` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::r8_native_admission_tests::r8_native_business_map_growth_uses_parent_ledger` | `s_session_v3/audit_owner/native.rs:1994` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::r8_native_admission_tests::r8_native_frame_body_release_and_preallocation_refusal` | `s_session_v3/audit_owner/native.rs:1868` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::r8_native_admission_tests::r8_native_local_material_memo_consumes_work_and_releases` | `s_session_v3/audit_owner/native.rs:1925` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::tests::native_hash_bucket_bound_covers_actual_growth_and_json_shapes_cover_heap` | `s_session_v3/audit_owner/native.rs:1833` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::tests::native_permits_release_and_growth_keeps_old_block_until_reserve` | `s_session_v3/audit_owner/native.rs:1789` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::u2_exit_adopt_tests::p2_3_rebirth_codec_vec_peak_counts_old_and_new_blocks` | `s_session_v3/audit_owner/native.rs:2837` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::u2_exit_adopt_tests::p2_3_rebirth_vec_peak_counts_old_and_new_blocks` | `s_session_v3/audit_owner/native.rs:2810` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::u2_exit_adopt_tests::p3a_production_reserve_native_rejects_allocated_plain_without_adopting` | `s_session_v3/audit_owner/native.rs:2876` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::u2_exit_adopt_tests::p3g_codec_vec_grow_and_scratch_never_double_reserve_same_allocation` | `s_session_v3/audit_owner/native.rs:2914` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::u2_exit_adopt_tests::u2_effects_rows_accept_same_parent_born_without_charge` | `s_session_v3/audit_owner/native.rs:2733` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::u2_exit_adopt_tests::u2_effects_rows_reject_plain_and_foreign_without_adopt` | `s_session_v3/audit_owner/native.rs:2741` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::u2_exit_adopt_tests::u2_fresh_parent_codec_vec_grow_never_adopts_allocated_plain` | `s_session_v3/audit_owner/native.rs:3005` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::u2_exit_adopt_tests::u2_history_prefix_rebirth_binds_new_block_under_parent` | `s_session_v3/audit_owner/native.rs:2758` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::u2_exit_adopt_tests::u2_history_prefix_rebirth_never_draws_moved_in_or_foreign` | `s_session_v3/audit_owner/native.rs:2776` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::u2_exit_adopt_tests::u2_material_parent_born_empty_and_journal_fee_reserved` | `s_session_v3/audit_owner/native.rs:2677` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::u2_exit_adopt_tests::u2_material_parent_rejects_fee_beyond_real_room_instead_of_moved_in` | `s_session_v3/audit_owner/native.rs:2685` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::u2_exit_adopt_tests::u2_q_raw_rebirth_never_draws_moved_in` | `s_session_v3/audit_owner/native.rs:2716` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::u2_exit_adopt_tests::u2_q_raw_rebirth_reserves_new_block_and_fee_moves_with_vec` | `s_session_v3/audit_owner/native.rs:2697` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::unified_ownership_v4_tests::handoff_ends_move_in_and_old_rest_never_refills_new_allowance` | `s_session_v3/audit_owner/native.rs:2601` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::unified_ownership_v4_tests::move_in_rejects_legacy_moved_in_without_mutation` | `s_session_v3/audit_owner/native.rs:2627` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::unified_ownership_v4_tests::move_in_reserves_splits_and_returns_rest_without_partial_mutation` | `s_session_v3/audit_owner/native.rs:2566` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::unified_ownership_v4_tests::observer_rejects_when_sponsor_exceeds_allowance_without_mutation` | `s_session_v3/audit_owner/native.rs:2553` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::native::unified_ownership_v4_tests::observer_sponsor_counts_in_allowance_and_old_drop_never_refills` | `s_session_v3/audit_owner/native.rs:2517` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::p1_2_keeper_pool_on_other_ledger_counts_physical_live_after_discard` | `s_session_v3/audit_owner.rs:5132` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::p1_alpha_stray_on_keeper_ledger_survives_discard_retry_and_is_rejected` | `s_session_v3/audit_owner.rs:5238` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::p2_4_seal_census_flags_duplicate_permit_on_real_seal_path` | `s_session_v3/audit_owner.rs:5008` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::publication_degrade_verdict_tests::publication_degrade_verdict_only_for_pure_budget_shortfall` | `s_session_v3/audit_owner.rs:7754` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::q::actual_nonempty_native_q_accept_preserves_full_raw_without_manual_double_charge` | `s_session_v3/audit_owner/q.rs:961` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::q::pending_normal_last_remove_releases_root_before_charge` | `s_session_v3/audit_owner/q.rs:935` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::r13_cold_candidate_and_q_share_parent_and_preserve_complete_output` | `s_session_v3/audit_owner.rs:4749` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::r13_optional_sql_rows_share_parent_without_changing_cells` | `s_session_v3/audit_owner.rs:4859` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::r14_source_keeper_seal_and_next_request_share_explicit_parent` | `s_session_v3/audit_owner.rs:5306` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::raw_native_move_ownership_tests::pending_and_bound_raw_moves_keep_original_permits_until_payload_drop` | `s_session_v3/audit_owner.rs:7362` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::sqlite_pager_probe::tests::pager_probe_startup_line_keeps_field_set` | `s_session_v3/sqlite_pager_probe.rs:313` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::sqlite_pager_probe::tests::pager_probe_testonly_cache_kib_parse` | `s_session_v3/sqlite_pager_probe.rs:343` | X | 新增链0／旧帧未测 |
| `v3::audit_owner::sqlite_pager_probe::tests::pager_probe_testonly_cache_request_sets_cache_size` | `s_session_v3/sqlite_pager_probe.rs:380` | X | 新增链0／旧帧未测 |
| `v3::audit_store::ref_value_tests::native_tree_removed_entry_keeps_payload_charge_after_tree_and_scope_drop` | `s_session_v3/audit_store.rs:1248` | X | 新增链0／旧帧未测 |
| `v3::audit_store::ref_value_tests::r9a_optional_fields_preserve_absence_kind_and_scoped_reads` | `s_session_v3/audit_store.rs:1347` | X | 新增链0／旧帧未测 |
| `v3::audit_store::ref_value_tests::r9a_ref_outputs_charge_plain_text_and_preserve_values` | `s_session_v3/audit_store.rs:1287` | X | 新增链0／旧帧未测 |
| `v3::aux_index::tests::native_aux_failed_table_construction_releases_partial_owners` | `s_session_v3/aux_index.rs:2621` | X | 新增链0／旧帧未测 |
| `v3::aux_index::tests::native_aux_owner_prefix_duplicates_reference_and_overlay_matrix` | `s_session_v3/aux_index.rs:2863` | X | 新增链0／旧帧未测 |
| `v3::aux_index::tests::native_aux_tables_retain_owners_across_scope_and_release_on_drop` | `s_session_v3/aux_index.rs:2596` | X | 新增链0／旧帧未测 |
| `v3::aux_index::tests::p50_legacy_to_native_index_permit_counts_control_byte_per_slot` | `s_session_v3/aux_index.rs:2804` | X | 新增链0／旧帧未测 |
| `v3::aux_index::tests::r12_cold_object_aux_shared_scope_matches_complete_plain_outputs` | `s_session_v3/aux_index.rs:2665` | X | 新增链0／旧帧未测 |
| `v3::business_json::native_consumer_tests::admitted_json_and_hash_match_normal_with_payload_raii` | `s_session_v3/business_json.rs:3343` | X | 新增链0／旧帧未测 |
| `v3::business_json::native_consumer_tests::admitted_sql_text_clone_and_json_parsers_match_plain` | `s_session_v3/business_json.rs:3304` | X | 新增链0／旧帧未测 |
| `v3::business_json::native_consumer_tests::raw_native_owners_move_out_of_scope_and_release_only_on_drop` | `s_session_v3/business_json.rs:3242` | X | 新增链0／旧帧未测 |
| `v3::business_json::native_consumer_tests::raw_native_tree_reuses_high_water_and_retains_buffer_until_drop` | `s_session_v3/business_json.rs:3278` | X | 新增链0／旧帧未测 |
| `v3::business_json::plain_raii_tests::native_json_cannot_be_mutated_or_detached_without_admission` | `s_session_v3/business_json.rs:3462` | X | 新增链0／旧帧未测 |
| `v3::business_json::plain_raii_tests::native_plain_values_keep_original_permits_past_scope_and_drop_independently` | `s_session_v3/business_json.rs:3387` | X | 新增链0／旧帧未测 |
| `v3::business_json::plain_raii_tests::native_strict_parse_preserves_diagnostics_and_failed_parse_returns_permit` | `s_session_v3/business_json.rs:3437` | X | 新增链0／旧帧未测 |
| `v3::business_json::raw_events_layout_tests::aggregate_tree_conversion_and_direct_append_refuse_without_releasing_live_children` | `s_session_v3/business_json.rs:3762` | X | 新增链0／旧帧未测 |
| `v3::business_json::raw_events_layout_tests::itemized_array_multiple_growths_retain_exact_buffers_across_work_scopes` | `s_session_v3/business_json.rs:3812` | X | 新增链0／旧帧未测 |
| `v3::business_json::raw_events_native_append_tests::failed_work_and_capacity_refusal_preserve_raw_payload_and_balance` | `s_session_v3/business_json.rs:3709` | X | 新增链0／旧帧未测 |
| `v3::business_json::raw_events_native_append_tests::same_parent_new_work_appends_and_foreign_parent_refuses_without_mutation` | `s_session_v3/business_json.rs:3667` | X | 新增链0／旧帧未测 |
| `v3::business_json::scoped_json_array_tests::scoped_array_each_preparation_refusal_preserves_parent_payload_capacity_and_live` | `s_session_v3/business_json.rs:3580` | X | 新增链0／旧帧未测 |
| `v3::business_json::scoped_json_array_tests::scoped_array_growth_accounts_old_new_payload_and_metadata_together` | `s_session_v3/business_json.rs:3536` | X | 新增链0／旧帧未测 |
| `v3::business_json::scoped_json_array_tests::scoped_array_outlives_scope_retains_metadata_and_releases_at_last_owner_drop` | `s_session_v3/business_json.rs:3495` | X | 新增链0／旧帧未测 |
| `v3::business_json::scoped_json_array_tests::scoped_array_refuses_dual_parent_and_finished_native_rejects_unscoped_mutation` | `s_session_v3/business_json.rs:3622` | X | 新增链0／旧帧未测 |
| `v3::capture::retained::native_boundary_tests::plain_capture_roundtrip_stays_plain_and_unreachable_native_injection_is_rejected` | `s_session_v3/q_retained.rs:896` | X | 新增链0／旧帧未测 |
| `v3::catalog_evidence::native_catalog_temp_tests::reference_text_keeps_native_permit_after_scope_and_matches_canonical_oracle` | `s_session_v3/catalog_evidence.rs:960` | X | 新增链0／旧帧未测 |
| `v3::catalog_evidence::native_catalog_temp_tests::reference_text_refusal_keeps_source_and_releases_partial_output` | `s_session_v3/catalog_evidence.rs:999` | X | 新增链0／旧帧未测 |
| `v3::control_audit::tests::r12_native_control_cold_matches_plain_empty_domain_on_parent_scope` | `s_session_v3/control_audit.rs:544` | X | 新增链0／旧帧未测 |
| `v3::control_audit::tests::r12_native_control_cold_matches_plain_published_facts_on_parent_scope` | `s_session_v3/control_audit.rs:486` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::counts_footprint_tests::counts_footprint_actual_owned_normal_diagnostic` | `s_session_v3/cross_request_source_cache.rs:2265` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::codec_root_success_handler_legal_growth_matches_complete_cold` | `s_session_v3/cross_request_source_cache.rs:1782` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::counts_subset_normal_multicut_bounded_groups_match_complete_cold` | `s_session_v3/cross_request_source_cache.rs:1543` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::counts_subset_normal_owner_releases_unselected_originals_and_keeps_exact_tuples` | `s_session_v3/cross_request_source_cache.rs:1652` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::counts_subset_normal_unrelated_legal_append_keeps_complete_counts` | `s_session_v3/cross_request_source_cache.rs:1623` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::cross_request_source_cache_actual_owned_snapshot_diagnostic` | `s_session_v3/cross_request_source_cache.rs:2114` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::cross_request_source_cache_complete_key_and_exact_rows_never_hash_only` | `s_session_v3/cross_request_source_cache.rs:1888` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::cross_request_source_cache_damaged_current_material_rejects_and_clears` | `s_session_v3/cross_request_source_cache.rs:1938` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::cross_request_source_cache_later_revision_and_withdrawal_still_use_current_index` | `s_session_v3/cross_request_source_cache.rs:1848` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::cross_request_source_cache_measured_cap_exact_minus_one_and_one` | `s_session_v3/cross_request_source_cache.rs:1871` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::cross_request_source_cache_real_append_matches_complete_cold` | `s_session_v3/cross_request_source_cache.rs:1814` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::cross_request_source_cache_sql_interrupt_never_cold_retries` | `s_session_v3/cross_request_source_cache.rs:2038` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::cross_request_source_cache_subset_requires_unique_complete_old_cover` | `s_session_v3/cross_request_source_cache.rs:2060` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::cross_request_source_cache_tail_and_late_catalog_fault_cannot_be_hidden` | `s_session_v3/cross_request_source_cache.rs:1972` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::cross_request_source_cache_unrelated_original_change_outside_counts_closure_still_hits` | `s_session_v3/cross_request_source_cache.rs:1997` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::p2a_source_claim_points_to_charged_body_and_duplicate_reserve_rejected` | `s_session_v3/cross_request_source_cache.rs:954` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::p3b_seal_rejects_missing_source_claim_on_charged_body` | `s_session_v3/cross_request_source_cache.rs:1046` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::r11_source_binding_new_and_seal_use_shared_scope` | `s_session_v3/cross_request_source_cache.rs:1111` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::r13_source_trial_cold_hit_miss_record_and_seal_use_shared_scope` | `s_session_v3/cross_request_source_cache.rs:1403` | X | 新增链0／旧帧未测 |
| `v3::cross_request_source_cache::tests::r14_source_pool_normal_growth_and_sparse_cap_match_none` | `s_session_v3/cross_request_source_cache.rs:1169` | X | 新增链0／旧帧未测 |
| `v3::driver::tests::audit_owner_complete_accept_tail_begin_recover_retry_and_revision_qualification` | `s_session_v3/driver.rs:3097` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::audit_owner_complete_normal_second_connection_and_new_life_are_cold` | `s_session_v3/driver.rs:4845` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::audit_owner_complete_optional_budget_releases_without_changing_reply` | `s_session_v3/driver.rs:4224` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::audit_owner_complete_ready_receipt_repost_keep_owned_state_and_match_ordinary` | `s_session_v3/driver.rs:3026` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::audit_owner_incremental_normal_writes_full_oracle_and_suffix` | `s_session_v3/driver.rs:7614` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::audit_owner_native_no_write_must_not_repeat_all_old_cut_semantics` | `s_session_v3/driver.rs:2956` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::audit_owner_new_message_raw_replay_matches_ordinary_complete_tables` | `s_session_v3/driver.rs:2870` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::audit_owner_normal_append_keeps_candidate_below_old_multiplier_gate` | `s_session_v3/driver.rs:4695` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::audit_owner_staged_normal_append_consumes_actual_nodes_and_suffix` | `s_session_v3/driver.rs:7550` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::audit_owner_write_budget_retains_candidate_within_retention_cap_and_parent_limit` | `s_session_v3/driver.rs:4770` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::counts_subset_driver_normal_rows_append_repost_restart_and_shared_s7` | `s_session_v3/driver.rs:5545` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::cross_request_ready_first_write_and_two_requests_preserve_parent` | `s_session_v3/driver.rs:4666` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::diagnostic_fixture_serve_entry_reaches_ready_but_refuses_every_advance` | `s_session_v3/driver.rs:1228` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::incremental_build_full_branch_only_at_birth` | `s_session_v3/driver.rs:7998` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_bootstrap_epoch_profile_idle_and_illegal_request_reject_before_pending_resume_and_release` | `s_session_v3/driver.rs:5174` | H0 | 832 KiB／H前段已覆盖 |
| `v3::driver::tests::native_bootstrap_idle_identity_hits_then_drops_and_ready_is_new_cold_default_paths_unchanged` | `s_session_v3/driver.rs:4935` | H0 | 832 KiB／H前段已覆盖 |
| `v3::driver::tests::native_bootstrap_pending_drops_startup_slot_before_resume_then_identity_cold_matches_current_frontier` | `s_session_v3/driver.rs:5038` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_bootstrap_two_transactions_reject_current_damage_or_sql_error_without_old_proof` | `s_session_v3/driver.rs:5109` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_capture_readonly_current_source_rejects_fresh_damage_without_writes` | `s_session_v3/driver.rs:6198` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_capture_real_sql_failures_preserve_each_persistent_phase_boundary` | `s_session_v3/driver.rs:6155` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_capture_three_commits_and_restart_match_ordinary_current_state` | `s_session_v3/driver.rs:6076` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_delta_explicit_policy_sql_receipt_and_failed_commit_retry` | `s_session_v3/driver.rs:1818` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_mode_is_frozen_before_pending_resume_captures_advance_and_prepare` | `s_session_v3/driver.rs:6131` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_prefix_context_rechecks_real_connection_current_material` | `s_session_v3/driver.rs:1505` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_prefix_final_deadline_preserves_business_and_clears_retained_context` | `s_session_v3/driver.rs:1665` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_prefix_outer_epoch_failure_clears_retained_slot_without_reviving_pool` | `s_session_v3/driver.rs:1590` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_prefix_populated_capture_type_error_precedes_frame_and_epoch` | `s_session_v3/driver.rs:1469` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_prefix_preserves_business_epoch_rule_order_and_clears_context` | `s_session_v3/driver.rs:1417` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_prefix_success_keeps_actual_startup_validation` | `s_session_v3/driver.rs:1739` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_startup_full_audit_and_pending_resume_before_write_guards` | `s_session_v3/driver.rs:5268` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_startup_rejects_each_current_damage_before_pending_resume` | `s_session_v3/driver.rs:5312` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::native_startup_sql_errors_propagate_without_ordinary_fallback` | `s_session_v3/driver.rs:5366` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::ordinary_material_multicut_wire_comparison_keeps_same_proofs_and_all_cuts` | `s_session_v3/driver.rs:8437` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::ordinary_proof_multicut_complete_outputs_match_frozen_oracle` | `s_session_v3/driver.rs:8336` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::p1_1_driver_warm_old_pool_partition_counts_l0_once` | `s_session_v3/driver.rs:4294` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::p24adj_cold_root_renewal_keeps_real_revocations` | `s_session_v3/driver.rs:4586` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::p24adj_cold_root_rerun_renews_lease_and_larger_cap_ingest_commits` | `s_session_v3/driver.rs:4462` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::p3_1_cold_root_audit_failure_leaves_no_permit_on_destroyed_root` | `s_session_v3/driver.rs:4346` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::p3_d_cold_root_residue_after_clear_handles_is_rejected` | `s_session_v3/driver.rs:4395` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::p6_complete_ledger_cover_matches_across_delta_and_full_lease_owner_paths` | `s_session_v3/driver.rs:8708` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::pager_probe_configure_keeps_carry_lease_open` | `s_session_v3/driver.rs:8124` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::publication_candidate_budget_shortfall_degrades_and_keeps_admitting` | `s_session_v3/driver.rs:7989` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::publication_ledger_after_budget_failure_rejects_without_degrade` | `s_session_v3/driver.rs:8113` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::publication_prepare_wrapped_allocation_failure_degrades` | `s_session_v3/driver.rs:8105` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::publication_real_error_after_budget_failure_rejects_without_degrade` | `s_session_v3/driver.rs:8083` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::publication_real_tail_failure_still_rejects_without_degrade` | `s_session_v3/driver.rs:8074` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::publication_source_append_allocation_failure_degrades` | `s_session_v3/driver.rs:8096` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::recover_lock_budget_live_and_peak_continue_across_recover` | `s_session_v3/driver.rs:3600` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_commit_failure_and_rollback_never_promote` | `s_session_v3/driver.rs:3503` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_external_write_and_read_domain_change_still_revoke` | `s_session_v3/driver.rs:3532` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_fee_permit_moves_on_same_parent_ledger` | `s_session_v3/driver.rs:3632` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_handoff_failure_revokes` | `s_session_v3/driver.rs:3585` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_holds_gate_rejects_fee_permit_on_foreign_ledger` | `s_session_v3/driver.rs:4195` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_old_raw_token_and_begin_token_rejected_after_promotion` | `s_session_v3/driver.rs:3714` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_promotes_after_duplicate_ingest_and_chained_rebinds` | `s_session_v3/driver.rs:4134` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_promotes_after_new_request_readonly_rebind` | `s_session_v3/driver.rs:4011` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_promotes_after_new_request_receipt_rebind` | `s_session_v3/driver.rs:4105` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_rebind_does_not_advance_foreign_or_stale_qualification_record` | `s_session_v3/driver.rs:4063` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_rejects_confirm_epoch_mismatch` | `s_session_v3/driver.rs:3862` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_rejects_foreign_ledger_budget_handoff` | `s_session_v3/driver.rs:3881` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_rejects_missing_candidate_or_parent_ledger` | `s_session_v3/driver.rs:3682` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_rejects_missing_same_snapshot_complete` | `s_session_v3/driver.rs:4176` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_rejects_non_increasing_epoch` | `s_session_v3/driver.rs:3440` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_rejects_qualification_recorded_on_other_parent_or_domain` | `s_session_v3/driver.rs:3937` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_rejects_qualification_without_parent_record` | `s_session_v3/driver.rs:4042` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_rejects_stale_qualification_stamp` | `s_session_v3/driver.rs:3834` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_rejects_unclosed_recover_effects` | `s_session_v3/driver.rs:3475` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_same_owner_promotes_and_voids_old_epoch_token_and_lease` | `s_session_v3/driver.rs:3382` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::recover_lock_schema_and_material_change_revoke` | `s_session_v3/driver.rs:3772` | B | 896 KiB／无新增冷seed |
| `v3::driver::tests::request_history_handler_sql_and_commit_failures_release_context` | `s_session_v3/driver.rs:6023` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::request_history_real_driver_three_stages_reuse_and_repost_starts_cold` | `s_session_v3/driver.rs:5967` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::s4_generation_incremental_matches_cold_full_six_stages` | `s_session_v3/driver.rs:8660` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::s4_stage_measurement_from_bars` | `s_session_v3/driver.rs:8679` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::s_system_clock_budget_degrade_reuses_first_commit_reading` | `s_session_v3/driver.rs:2809` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::source_keeper_driver_append_repost_restart_preserves_results_and_request_reuse` | `s_session_v3/driver.rs:5707` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::source_keeper_driver_measured_cap_only_changes_retention` | `s_session_v3/driver.rs:5919` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::source_keeper_driver_workspace_shares_s7_with_nonempty_old_pool_and_exact_rows` | `s_session_v3/driver.rs:5815` | O | 1,056 KiB／旧核保守包络 |
| `v3::driver::tests::staged_next106_cold_driver_observation` | `s_session_v3/driver.rs:5391` | O | 1,056 KiB／旧核保守包络 |
| `v3::generation::tests::event_radix_memo_hit_requires_current_native_scope_and_keeps_root_recipe` | `s_session_v3/generation.rs:2614` | X | 新增链0／旧帧未测 |
| `v3::generation::tests::finish_axes_resource_rejection_preserves_published_roots` | `s_session_v3/generation.rs:2900` | X | 新增链0／旧帧未测 |
| `v3::generation::tests::native_generated_results_match_plain_and_keep_permits_until_drop` | `s_session_v3/generation.rs:2813` | X | 新增链0／旧帧未测 |
| `v3::generation::tests::native_sequence_cross_generation_same_parent_foreign_refusal_and_discard` | `s_session_v3/generation.rs:2742` | X | 新增链0／旧帧未测 |
| `v3::generation::tests::native_stable_sort_matches_old_equal_key_order_and_refuses_atomically` | `s_session_v3/generation.rs:2697` | X | 新增链0／旧帧未测 |
| `v3::history::effective_sequence_tests::h1_effective_blocks_match_independent_array_each_cut` | `s_session_v3/history.rs:9409` | X | 新增链0／旧帧未测 |
| `v3::history::effective_sequence_tests::h1_effective_final_winner_edits_and_coordinate_swaps` | `s_session_v3/history.rs:9458` | X | 新增链0／旧帧未测 |
| `v3::history::effective_sequence_tests::h1_effective_native_parent_exact_peak_rejection_and_release` | `s_session_v3/history.rs:9574` | X | 新增链0／旧帧未测 |
| `v3::history::effective_sequence_tests::h1_effective_revision_work_tracks_height_then_append_reuses_blocks` | `s_session_v3/history.rs:9493` | X | 新增链0／旧帧未测 |
| `v3::history::native_reader_owner_tests::parsed_plain_owner_outlives_reader_and_scope_without_retaining_failed_parse` | `s_session_v3/history.rs:8699` | X | 新增链0／旧帧未测 |
| `v3::history::native_record_fields::tests::record_sidecar_failed_replacement_keeps_original_and_releases_temporaries` | `s_session_v3/native_record_fields.rs:134` | X | 新增链0／旧帧未测 |
| `v3::history::native_record_fields::tests::record_sidecar_shared_scope_seed_subset_matches_plain` | `s_session_v3/native_record_fields.rs:27` | X | 新增链0／旧帧未测 |
| `v3::history::native_record_fields::tests::record_sidecar_subset_outlives_scope_and_drops_its_own_permits` | `s_session_v3/native_record_fields.rs:88` | X | 新增链0／旧帧未测 |
| `v3::history::request_closure_tests::h1_request_closure_native_exact_peak_reject_and_release` | `s_session_v3/history.rs:9813` | X | 新增链0／旧帧未测 |
| `v3::history::request_closure_tests::h1_request_closure_overlay_conflict_and_failure_do_not_escape_revocation` | `s_session_v3/history.rs:9751` | X | 新增链0／旧帧未测 |
| `v3::history::request_closure_tests::h1_request_closure_rechecks_all_root_fields_and_revokes_both_modes` | `s_session_v3/history.rs:9668` | X | 新增链0／旧帧未测 |
| `v3::history::request_closure_tests::h1_request_closure_shares_only_same_request_mode_and_keeps_canonical` | `s_session_v3/history.rs:9630` | X | 新增链0／旧帧未测 |
| `v3::history::request_closure_tests::h1_request_closure_template_and_readset_negatives_match_full_oracle` | `s_session_v3/history.rs:9705` | X | 新增链0／旧帧未测 |
| `v3::history::tests::h1_native_history_prefix_real_multicut_roots_and_linear_leaf_visits` | `s_session_v3/history.rs:6725` | X | 新增链0／旧帧未测 |
| `v3::history::tests::h1_owner_retained_prefix_normal_append_and_revision_matches_oracle` | `s_session_v3/history.rs:4438` | X | 新增链0／旧帧未测 |
| `v3::history::tests::history_layout_multicut_native_projection_equals_plain_audit` | `s_session_v3/history.rs:6636` | X | 新增链0／旧帧未测 |
| `v3::history::tests::native_cut_projection_full_history_matches_ordinary_and_rejects_fresh_damage` | `s_session_v3/history.rs:6970` | X | 新增链0／旧帧未测 |
| `v3::history::tests::native_cut_projection_gate_excludes_reader_dynamic_owned_and_deadline_native_publication_fields` | `s_session_v3/history.rs:4797` | X | 新增链0／旧帧未测 |
| `v3::history::tests::native_dual_actual_template_negative_matrix` | `s_session_v3/history.rs:7198` | X | 新增链0／旧帧未测 |
| `v3::history::tests::native_dual_real_multicut_roots_and_scoped_projection` | `s_session_v3/history.rs:7028` | X | 新增链0／旧帧未测 |
| `v3::history::tests::native_full_audit_duplicate_put_excludes_non_native_owned_bounded_and_gates` | `s_session_v3/history.rs:4694` | X | 新增链0／旧帧未测 |
| `v3::history::tests::native_full_audit_duplicate_put_keeps_conflict_bad_f_missing_and_existing_overlay` | `s_session_v3/history.rs:4579` | X | 新增链0／旧帧未测 |
| `v3::history::tests::native_full_audit_duplicate_put_keeps_current_source_vec_and_first_f` | `s_session_v3/history.rs:4514` | X | 新增链0／旧帧未测 |
| `v3::history::tests::native_history_prefix_bad_cut_and_unpublished_future_material_still_reject` | `s_session_v3/history.rs:6823` | X | 新增链0／旧帧未测 |
| `v3::history::tests::native_history_prefix_effective_topology_keeps_last_coord_overwrite` | `s_session_v3/history.rs:4472` | X | 新增链0／旧帧未测 |
| `v3::history::tests::native_measured_reader_retains_source_role_and_actual_frame_observation` | `s_session_v3/history.rs:4639` | X | 新增链0／旧帧未测 |
| `v3::history::tests::ordinary_material_whole_readonly_audit_heap_releases` | `s_session_v3/history.rs:6150` | X | 新增链0／旧帧未测 |
| `v3::history::tests::ordinary_proof_owner_commits_only_after_whole_cut_and_reopen_starts_empty` | `s_session_v3/history.rs:6181` | X | 新增链0／旧帧未测 |
| `v3::history::tests::r9a_new_cut_manifest_consumes_scoped_sql_source_and_publication` | `s_session_v3/history.rs:6665` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::cold_relation_index_sort_ties_keep_complete_projection_roots` | `s_session_v3/object_index.rs:5642` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::history_public_coverage_ordinary_bound_births_preserves_record_budget_before_reuse` | `s_session_v3/object_index.rs:5826` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_candidate_fields_component_exact_metadata_slot_and_late_null_gates` | `s_session_v3/object_index.rs:4465` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_candidate_fields_internal_record_json_edge_gate_and_real_wire_sql_failure` | `s_session_v3/object_index.rs:4592` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_candidate_union_real_generations_match_plan_and_reduce_sql_reads` | `s_session_v3/object_index.rs:4796` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_candidate_union_second_root_errors_have_no_put_or_future_wire_repair_and_budget_refuses` | `s_session_v3/object_index.rs:4686` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_changes_use_current_scope_and_return_owned_json_permits` | `s_session_v3/object_index.rs:5316` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_cut_projection_endpoints_sort_ties_records_and_overlay_lifetime` | `s_session_v3/object_index.rs:5716` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_cut_projection_life_checked_on_hits_and_first_failure_not_cached` | `s_session_v3/object_index.rs:5874` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_cut_projection_missing_corrupt_or_overlay_only_closure_never_cached` | `s_session_v3/object_index.rs:5911` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_cut_projection_real_generations_reuse_exact_wire_after_overlay_clear` | `s_session_v3/object_index.rs:6008` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_event_cursors_admitted_cuts_match_scan_and_drop_to_baseline` | `s_session_v3/object_index.rs:5382` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_event_failed_consuming_candidate_drops_cursor_and_all_permits` | `s_session_v3/object_index.rs:5441` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_event_failed_wire_replacement_preserves_saved_owner` | `s_session_v3/object_index.rs:5467` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_event_owners_share_once_and_release_after_last_scope_owner` | `s_session_v3/object_index.rs:5346` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_history_projection_sweep_positive_real_generations_complete_roots_and_material` | `s_session_v3/object_index.rs:6013` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_plan_capacity_refusal_preserves_input_and_budget` | `s_session_v3/object_index.rs:5290` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_plan_late_missing_wire_releases_partial_plan_without_mutating_input` | `s_session_v3/object_index.rs:5263` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_plan_owners_outlive_scope_and_match_independent_change_rows` | `s_session_v3/object_index.rs:5167` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_shape_leaf_complete_declarations_error_clear_and_capture_owned_closure` | `s_session_v3/object_index.rs:4084` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_shape_leaf_real_load_rejects_changed_projection_and_shared_entry_damage` | `s_session_v3/object_index.rs:3992` | X | 新增链0／旧帧未测 |
| `v3::object_index::tests::native_shape_leaf_real_load_reuses_across_distinct_births_and_excludes_gates` | `s_session_v3/object_index.rs:3919` | X | 新增链0／旧帧未测 |
| `v3::object_records::tests::native_wire_bad_early_record_precedes_bad_late_life_and_releases_all_owners` | `s_session_v3/object_records.rs:2117` | X | 新增链0／旧帧未测 |
| `v3::object_records::tests::native_wire_records_and_birth_reference_keep_permits_after_scope` | `s_session_v3/object_records.rs:2065` | X | 新增链0／旧帧未测 |
| `v3::object_records::tests::native_wire_valid_record_late_life_error_and_early_resource_keep_priority` | `s_session_v3/object_records.rs:2165` | X | 新增链0／旧帧未测 |
| `v3::ordinary_materials::tests::ordinary_material_pool_actual_heap_released_after_readonly_scope` | `s_session_v3/ordinary_materials_tests.rs:445` | X | 新增链0／旧帧未测 |
| `v3::prepared::tests::assemble_manifest_reads_beyond_closure_verify_stay_constant` | `s_session_v3/prepared.rs:4105` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::native_candidate_union_four_real_commits_match_ordinary_roots_rows_and_revision` | `s_session_v3/prepared.rs:4017` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::native_capture_sqlite_commit_failure_retries_with_new_current_capture` | `s_session_v3/prepared.rs:4536` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::native_local_material_memo_cannot_publish_trigger_damaged_deep_key` | `s_session_v3/prepared.rs:4376` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::native_manifest_roots_keep_three_permits_after_scope_until_owner_drop` | `s_session_v3/prepared.rs:1883` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::native_manifest_workspace_four_normal_commits_match_cold_and_ordinary_full_rows` | `s_session_v3/prepared.rs:4144` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_complete_ledger_real_owner_cover_tracks_revisions_and_rebuild` | `s_session_v3/prepared.rs:2896` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_complete_ledger_real_owner_records_pending_without_group` | `s_session_v3/prepared.rs:2988` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_complete_ledger_rejects_partial_and_wrong_rule_without_commit` | `s_session_v3/prepared.rs:3040` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_native_read_lease_boundary_cold_publish_fault_matrix_discards_runtime` | `s_session_v3/prepared.rs:4339` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_production_manifest_source_receipt_consumed_without_third_assembly` | `s_session_v3/prepared.rs:3169` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_production_manifest_source_receipt_rejects_request_substitution` | `s_session_v3/prepared.rs:3183` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_queued_units_ledger_commit_failure_keeps_content_and_never_retries` | `s_session_v3/prepared.rs:3124` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_queued_units_prepare_failure_discards_facts_and_stales` | `s_session_v3/prepared.rs:3078` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_queued_units_real_owner_commits_only_verified_complete_keys` | `s_session_v3/prepared.rs:2689` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_raw_partition_real_owner_keeps_revisions_without_full_certificate` | `s_session_v3/prepared.rs:2504` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_raw_partition_sidecar_commit_failure_keeps_content_and_stales` | `s_session_v3/prepared.rs:2612` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_real_prepare_closure_probe_stages_and_rolls_back_without_certification` | `s_session_v3/prepared.rs:3331` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_real_prepare_raw_receipt_keeps_old_revision_and_binds_request` | `s_session_v3/prepared.rs:3250` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_source_partitions_fees_release_after_refusal_commit_and_binding_failure` | `s_session_v3/prepared.rs:2364` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_source_partitions_real_owner_matches_originals_and_withdrawals` | `s_session_v3/prepared.rs:2111` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::p6_source_partitions_sidecar_commit_failure_keeps_content_and_stales` | `s_session_v3/prepared.rs:2618` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::r10_native_manifest_workspace_shared_scope_normal_commits_match_full_rows` | `s_session_v3/prepared.rs:4149` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::runtime_roots_append_revision_error_observation` | `s_session_v3/prepared.rs:3545` | P | 928 KiB／共同核包络 |
| `v3::prepared::tests::staged_new_nodes_complete_commit_observation` | `s_session_v3/prepared.rs:3433` | P | 928 KiB／共同核包络 |
| `v3::request_audit::handoff_domain_tests::bootstrap_grant_from_other_freeze_domain_is_rejected` | `s_session_v3/request_audit.rs:3187` | X | 新增链0／旧帧未测 |
| `v3::request_audit::handoff_domain_tests::bootstrap_grant_is_consumed_exactly_once` | `s_session_v3/request_audit.rs:3169` | X | 新增链0／旧帧未测 |
| `v3::request_audit::handoff_domain_tests::frozen_request_rejects_foreign_parent_and_expired_domain` | `s_session_v3/request_audit.rs:3133` | X | 新增链0／旧帧未测 |
| `v3::request_audit::handoff_domain_tests::p1_1_same_parent_pool_counts_l0_once_and_items_keep_contract` | `s_session_v3/request_audit.rs:3222` | X | 新增链0／旧帧未测 |
| `v3::request_audit::handoff_domain_tests::p1_1_unmappable_old_pool_is_rejected_without_mutation` | `s_session_v3/request_audit.rs:3335` | X | 新增链0／旧帧未测 |
| `v3::request_audit::handoff_domain_tests::used_foreign_cold_parent_cannot_be_replaced_by_handoff` | `s_session_v3/request_audit.rs:3154` | X | 新增链0／旧帧未测 |
| `v3::request_audit::handoff_domain_tests::warm_prepare_binds_real_parent_and_voids_grant` | `s_session_v3/request_audit.rs:3415` | X | 新增链0／旧帧未测 |
| `v3::request_audit::r_owner_move_tests::r_optional_bad_sql_precedes_scope_and_rejected_read_refunds` | `s_session_v3/request_audit.rs:2991` | X | 新增链0／旧帧未测 |
| `v3::request_audit::r_owner_move_tests::r_sql_row_pop_and_blob_move_keep_exact_parent_charge` | `s_session_v3/request_audit.rs:2932` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::measured_capture_and_rebind_preserve_late_type_error_over_early_bad_frame` | `s_session_v3/request_audit.rs:2340` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::measured_default_context_cold_hit_and_limit_clear_slot_without_parent` | `s_session_v3/request_audit.rs:1593` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::measured_native_admitted_cold_hit_append_preserves_full_output_and_refunds` | `s_session_v3/request_audit.rs:1403` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::r11_exact_rows_admitted_preserves_all_sql_cells_scope_and_empty_columns` | `s_session_v3/request_audit.rs:1106` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::r11_request_native_shared_scope_cold_warm_append_full_output_equal` | `s_session_v3/request_audit.rs:1398` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_actual_current_three_fresh_transactions_diagnostic` | `s_session_v3/request_audit.rs:2091` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_binding_sql_interrupt_once_rejects_without_cold_retry` | `s_session_v3/request_audit.rs:1921` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_current_control_and_exact_old_material_damage_never_reuses` | `s_session_v3/request_audit.rs:1869` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_current_tail_damage_and_extra_ohlc_cannot_be_hidden_by_prefix_binding` | `s_session_v3/request_audit.rs:2074` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_exact_cells_preserve_type_null_and_raw_bytes` | `s_session_v3/request_audit.rs:2383` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_fresh_transactions_bind_exactly_and_drop_at_request_end` | `s_session_v3/request_audit.rs:1726` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_legal_node_set_change_builds_fresh_then_rebinds` | `s_session_v3/request_audit.rs:1802` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_node_error_precedes_current_control_error_and_clears_slot` | `s_session_v3/request_audit.rs:1977` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_node_mismatch_does_not_hide_later_decode_or_sql_error` | `s_session_v3/request_audit.rs:2290` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_node_rebind_complete_set_rejects_duplicate_digest_and_omission` | `s_session_v3/request_audit.rs:2269` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_node_sql_interrupt_once_rejects_without_capture_retry` | `s_session_v3/request_audit.rs:1946` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_one_capture_three_complete_current_node_scans` | `s_session_v3/request_audit.rs:1741` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_ordinary_and_zero_generation_do_not_gain_shared_source` | `s_session_v3/request_audit.rs:2012` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_original_tuple_comparison_in_bin_preserves_bytes_and_sets` | `s_session_v3/request_audit.rs:2220` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_rowid_reinsertion_is_material_change` | `s_session_v3/request_audit.rs:1909` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_tail_and_normal_begin_keys_reuse_but_complete_raw_stays_fresh` | `s_session_v3/request_audit.rs:1851` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_history_unknown_meta_changes_miss_even_if_original_gate_accepts` | `s_session_v3/request_audit.rs:1835` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_native_scan_scratch_cold_warm_and_legal_append_full_output_equal` | `s_session_v3/request_audit.rs:1393` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_native_scan_scratch_reuses_capacity_across_complete_transactions` | `s_session_v3/request_audit.rs:1345` | X | 新增链0／旧帧未测 |
| `v3::request_audit::tests::request_native_three_capture_entries_borrow_digest_and_match_owned_sql_baseline` | `s_session_v3/request_audit.rs:1259` | X | 新增链0／旧帧未测 |
| `v3::source_audit::ordinary_source::tests::ordinary_proof_checked_refs_hit_registers_coord_after_preceding_object_withdraws` | `s_session_v3/ordinary_source.rs:1033` | X | 新增链0／旧帧未测 |
| `v3::source_audit::ordinary_source::tests::ordinary_proof_complete_ref_and_profile_rule_are_not_digest_only_keys` | `s_session_v3/ordinary_source.rs:1379` | X | 新增链0／旧帧未测 |
| `v3::source_audit::ordinary_source::tests::ordinary_proof_duplicate_global_effect_precedes_later_malformed_record` | `s_session_v3/ordinary_source.rs:954` | X | 新增链0／旧帧未测 |
| `v3::source_audit::ordinary_source::tests::ordinary_proof_group_dependency_changes_invalidate_same_shape_root` | `s_session_v3/ordinary_source.rs:1125` | X | 新增链0／旧帧未测 |
| `v3::source_audit::ordinary_source::tests::ordinary_proof_raw_wire_bar_and_original_changes_match_frozen_first_error` | `s_session_v3/ordinary_source.rs:926` | X | 新增链0／旧帧未测 |
| `v3::source_audit::ordinary_source::tests::ordinary_proof_request_negative_subject_dependency_cannot_be_reused` | `s_session_v3/ordinary_source.rs:1186` | X | 新增链0／旧帧未测 |
| `v3::source_audit::ordinary_source::tests::ordinary_proof_root_declaration_errors_match_independent_oracle` | `s_session_v3/ordinary_source.rs:1358` | X | 新增链0／旧帧未测 |
| `v3::source_audit::ordinary_source::tests::ordinary_proof_step_bar_and_witness_shape_dependencies_match_old_local_kernel` | `s_session_v3/ordinary_source.rs:1249` | X | 新增链0／旧帧未测 |
| `v3::source_audit::ordinary_source::tests::ordinary_proof_success_reuse_oracle_is_independent_and_two_frontiers_release` | `s_session_v3/ordinary_source.rs:881` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_contributions::component_tests::b2_source_duplicate_key_is_reported_at_second_current_occurrence` | `s_session_v3/source_contributions.rs:2142` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_contributions::component_tests::b2_source_nested_reverse_index_charges_each_member_and_clears_empty_root` | `s_session_v3/source_contributions.rs:2120` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_contributions::component_tests::b2_source_retained_tree_uses_actual_string_capacity_and_symmetric_release` | `s_session_v3/source_contributions.rs:2091` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_contributions::source_order::tests::b2_order_avl_stable_handles_survive_prefix_middle_and_reverse_edits` | `s_session_v3/source_order.rs:383` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_contributions::source_order::tests::b2_order_rejects_wrong_original_and_stale_handle_before_reuse` | `s_session_v3/source_order.rs:414` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_contributions::source_order::tests::native_order_admission_failure_preserves_avl_and_next_handle` | `s_session_v3/source_order.rs:469` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_contributions::source_order::tests::native_order_reclaimed_node_and_weak_tail_keep_original_parent_charge` | `s_session_v3/source_order.rs:429` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_contributions::source_order::tests::native_order_rotations_remove_reinsert_and_drop_release_all_nodes` | `s_session_v3/source_order.rs:532` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_prefix::tests::cached_coords_preserve_order_lexemes_negative_and_domain_rules` | `s_session_v3/source_prefix.rs:357` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_prefix::tests::future_invalid_or_colliding_raw_disables_only_reference_reuse` | `s_session_v3/source_prefix.rs:325` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_prefix::tests::native_invalid_future_identity_drops_partial_certificate_owners` | `s_session_v3/source_prefix.rs:143` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_prefix::tests::native_prefix_certificates_and_shared_coords_follow_actual_owners` | `s_session_v3/source_prefix.rs:72` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_prefix::tests::references_hit_shared_coords_then_reject_same_price_revision_and_future_sources` | `s_session_v3/source_prefix.rs:197` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_prefix::tests::winner_sweep_matches_every_effective_prefix_with_revisions_and_sparse_coords` | `s_session_v3/source_prefix.rs:169` | X | 新增链0／旧帧未测 |
| `v3::source_audit::source_receipt::tests::p6_business_source_coordinates_release_native_fees_on_success_and_refusal` | `s_session_v3/source_receipt.rs:666` | L | 384 KiB／低层闭合 |
| `v3::source_audit::source_receipt::tests::p6_raw_partition_borrows_original_payload_and_releases_pending_parent_fees` | `s_session_v3/source_receipt.rs:824` | L | 384 KiB／低层闭合 |
| `v3::source_audit::source_receipt::tests::p6_raw_receipt_expired_before_binding_releases_budget` | `s_session_v3/source_receipt.rs:811` | L | 384 KiB／低层闭合 |
| `v3::source_audit::source_receipt::tests::p6_raw_receipt_expired_before_consumption_releases_budget` | `s_session_v3/source_receipt.rs:819` | L | 384 KiB／低层闭合 |
| `v3::source_audit::source_receipt::tests::p6_raw_receipt_expired_during_closure_releases_budget` | `s_session_v3/source_receipt.rs:815` | L | 384 KiB／低层闭合 |
| `v3::source_audit::source_receipt::tests::p6_raw_receipt_fourth_root_failure_issues_no_prepared_receipt` | `s_session_v3/source_receipt.rs:1032` | L | 384 KiB／低层闭合 |
| `v3::source_audit::source_receipt::tests::p6_raw_receipt_preserves_full_event_and_only_requested_suffix` | `s_session_v3/source_receipt.rs:928` | L | 384 KiB／低层闭合 |
| `v3::source_audit::source_receipt::tests::p6_raw_receipt_rejects_request_and_four_root_substitution` | `s_session_v3/source_receipt.rs:987` | L | 384 KiB／低层闭合 |
| `v3::source_audit::source_receipt::tests::p6_raw_receipt_source_failure_issues_nothing` | `s_session_v3/source_receipt.rs:965` | L | 384 KiB／低层闭合 |
| `v3::source_audit::tests::cold_raw_index_normal_revisions_unordered_duplicate_prices_and_replay_view` | `s_session_v3/source_audit.rs:3085` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::cold_source_normal_fixture_oracle` | `s_session_v3/source_audit.rs:3239` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_field_memo_scoped_hit_copies_clear_and_drop_keep_original_permits` | `s_session_v3/source_audit.rs:3675` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_fields_exact_headers_names_and_overlay_isolation` | `s_session_v3/source_audit.rs:3584` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_fields_multi_cut_reuses_only_decoding_and_error_clears` | `s_session_v3/source_audit.rs:3527` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_history_record_handoff_eliminates_both_source_field_lookups` | `s_session_v3/source_audit.rs:3474` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_raw_facts_keep_original_bucket_permits_across_scope_and_move` | `s_session_v3/source_audit.rs:3155` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_raw_ledger_incremental_winners_rewind_and_overlay_clear` | `s_session_v3/source_audit.rs:3254` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_raw_ledger_invalid_source_only_checked_for_final_winner` | `s_session_v3/source_audit.rs:3356` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_raw_ledger_jump_skips_invalid_transients_and_checks_final_collisions_first` | `s_session_v3/source_audit.rs:3296` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_raw_ledger_measured_visits_hits_rewind_and_failure` | `s_session_v3/source_audit.rs:2989` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_raw_ledger_real_multicut_matches_full_source_audit` | `s_session_v3/source_audit.rs:3378` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_source_measured_preserves_fields_witness_and_failure_cleanup` | `s_session_v3/source_audit.rs:3909` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_witness_identity_exact_inputs_and_success_only` | `s_session_v3/source_audit.rs:3841` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_witness_identity_keeps_current_cut_faults` | `s_session_v3/source_audit.rs:3987` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_witness_identity_requires_original_complete_closure` | `s_session_v3/source_audit.rs:4130` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::native_witness_memo_scoped_hits_clear_and_drop_keep_original_permits` | `s_session_v3/source_audit.rs:3772` | X | 新增链0／旧帧未测 |
| `v3::source_audit::tests::source_identity_producer_outlives_scope_without_freeing_initial_strings` | `s_session_v3/source_audit.rs:3196` | X | 新增链0／旧帧未测 |
| `v3::state::audit_capture_gate_tests::cold_owned_facts_keep_unpublished_raw_separate_after_transaction_end` | `s_session_v3/state.rs:3164` | X | 新增链0／旧帧未测 |
| `v3::state::audit_capture_gate_tests::cold_relation_indexes_normal_multicut_full_state_parity` | `s_session_v3/state.rs:3214` | X | 新增链0／旧帧未测 |
| `v3::state::audit_capture_gate_tests::cold_relation_indexes_replay_message_keeps_raw_and_cuts` | `s_session_v3/state.rs:3319` | X | 新增链0／旧帧未测 |
| `v3::state::audit_capture_gate_tests::native_frame_reuse_opt_in_respects_render_and_budget_gates` | `s_session_v3/state.rs:3362` | X | 新增链0／旧帧未测 |
| `v3::state::h1_certificate_tests::h1_cert_cold_empty_revision_winner_and_unpublished_tail` | `s_session_v3/state.rs:4178` | S | 832 KiB／保留既有隔离 |
| `v3::state::h1_certificate_tests::h1_cert_deadline_and_scratch_failure_release` | `s_session_v3/state.rs:4224` | S | 832 KiB／保留既有隔离 |
| `v3::state::h1_certificate_tests::h1_cert_rejects_control_manifest_current_sql_and_missing_original` | `s_session_v3/state.rs:4201` | S | 832 KiB／保留既有隔离 |
| `v3::writer_cli::tests::diagnostic_fixture_switch_accepts_only_the_fixture_record_id` | `s_session_v3/writer_cli.rs:1089` | X | 新增链0／旧帧未测 |
| `v3::writer_cli::tests::diagnostic_fixture_tags_only_ready_inside_hashed_payload` | `s_session_v3/writer_cli.rs:1108` | X | 新增链0／旧帧未测 |
| `v3::writer_cli::tests::h1_admission_one_shot_requires_explicit_retention_resource_file` | `s_session_v3/writer_cli.rs:1020` | X | 新增链0／旧帧未测 |

**新增P1/P2锁补审：** 新5把legacy/positive控制锁的建库helper先经 `h1_admission_seed_three→f.open` 组件C，再显式 `drop(h1_admission_driver(...))` 正式H留台账缓存，主线程按H最大模型1,376 KiB；显式v3 advance正控另进子进程 `dispatch_fixture→writer_cli advance(None)→正式Driver→H`，同H预算且和父进程栈不相加。扩展的冷前缀修订/插入锁仍是原H锁，多轮顺序执行，不增加调用深度；业务源修订/历史插入的负担是堆中数据和迭代，模型不按代数乘栈。所有新增锁无ignore。

实际静态验证：两生产文件 rustfmt --edition 2021 --check 通过；逐字归一化语义对拍通过；只读核对三档cfg（test/非test）接口和unix/non-unix分支；新增helper仅s_incremental+unix调用，无未用生产测试钩子或额外allow。未运行cargo/rustc、Rust测试或重型窗口；Rust未验证，待CI。

冻结日志以外的当前新增锁另逐项补齐，避免把535误写成当前测试动态全集：

| 新增全名（均为v3::audit_owner::construction_tests::前缀） | 最深链/模型帧和 | 处理 |
|---|---|---|
| `h1_admission_temporary_seed_failure_same_database_new_process_restarts`（S1K） | 子进程正式H；约1,376 KiB；包含finish第二锁内注入失败后正常关闭及下一进程重启 | 共用H拆帧，断言/失败收口不动 |
| `h1_admission_legacy_v1_recover_prepolicy_mainonly_rejects_unchanged`（S1L） | C建库后显式H留真实缓存；约1,376 KiB；真正旧写入口只读VersionMismatch | 共用H拆帧，旧入口无写副作用 |
| `h1_admission_legacy_v1_accept_prepolicy_mainonly_rejects_unchanged`（S1L） | 同上H最大约1,376 KiB | 同上 |
| `h1_admission_legacy_v1_advance_prepolicy_mainonly_rejects_unchanged`（S1L） | 同上H最大约1,376 KiB | 同上 |
| `h1_admission_legacy_v2_clock_recover_prepolicy_mainonly_rejects_unchanged`（S1L） | 同上H最大约1,376 KiB | 同上 |
| `h1_admission_v3_selector_advance_prepolicy_mainonly_authenticates_before_migration`（S1L） | 父进程C/H夹具；子进程dispatch_fixture→writer_cli advance→正式H；两进程各约1,376 KiB，不相加 | 真入口共用H拆帧；None前缀不受旧写拒门影响 |

### 三档 cfg、所有权及告警风险穷举

| feature 档 | 非 test | test |
|---|---|---|
| `s_session` | LegacyWrite 及其两个旧调用点常驻，增量提前拒绝被排除，完整审计及旧协议分支保持；新冷帧 helper 与全部调用由 inc+unix 同门排除 | H1 两个测试 include 随父增量门排除，没有增量类型或 helper 泄漏 |
| `s_session,s_incremental` | LegacyWrite 的只读拒门有真实调用；v3 advance None 不变。open_rebuilt_history_into、finish_rebuilt_history、initialize_rebuilt_owner、recover_rebuilt_history 均有对应生产调用，无遗留旧函数符号。认证闭包仍返回 unit、WorkError<String> 原标注保留，输出槽均为借用且两个调用点初值 None | 原测试与新增5锁仅 test+inc+unix；test_prepare 调用所在 helper 有显式外层 test 属性。P2 未增 helper/import。原 hook 的声明/消费仍各在 test 门内 |
| `s_session,s_incremental,s_capture_native_testonly` | 新生产逻辑不新增 testonly 分支；同一 H1 核、同一 Ordinary/组件前缀分派及失败收口 | 新正控用原默认栈的独立进程冻结 Native 对照环境；不改变并行进程全局环境。原 Ordinary 先审后迁锁与所有原断言保留 |

另外核对 rust-check 实际默认、backtest_bin 两档：S bin 仍受 Cargo required-features 限制，不编入这些修改；BSP 等磁盘源码扫描仍需通过。非 Unix 按原 inc+unix 配对，不宣称基线整个 bin 已支持非 Unix。

没有新增 allow/expect lint 豁免、未使用导入或按值大对象泛型返回链。新增 expect 是同路径成功赋槽后的内部不变量断言；输出值不从借用逃逸。finish 中 owner 的借用在最终 swap 前结束，scope/enter guard 仍覆盖 recover helper 全程。以上是三档×test/非 test 的源码穷举，`-D warnings` 的实际编译裁定待 CI。

### 实际自检、独审及保持项

- 对全部8个改动/新增 Rust 文件执行 `rustfmt --edition 2021 --check --config skip_children=true`，通过。skip_children 只限制隐式模块递归；新 include 文件已显式列入。
- `git diff --check` 通过；REPORT 历史前缀与 HEAD 逐字相同，仅追加 S1L。
- `.github/workflows/ci.yml`、`history36h.yml` YAML 解析通过，所有 bash run 块 `bash -n` 通过，分别71/4块。H1 两组具名循环原22项全部保留且唯一，追加5项后27项；新增完整测试名与源码定义一一对应、无 ignore。history36h 本轮没有修改。
- `python3 -B -m unittest s_session.tests.test_tb01c_r4_anchor_lock s_session.tests.test_tb01c_query_integrity_layout -v`，8/8通过。另在 `s_session/tests` 目录实跑 S3 两条 H1 recover 合同锁、原 S3 recover 分派锁和旧生命周期 recover 两锁，5/5通过。首次调用误用测试类名，只产生 loader AttributeError，按真实 S3LifecycleTests 更正后上述5项均通过；没有修改 Python 实现或测试。
- `python3 -B .github/scripts/check-prod-no-test-hooks.py`，`violations=0`；`python3 -B .github/scripts/gen-prod-banned-literals.py --check .` 与源码一致，禁串清单本轮未改。这里只是源码检查，生产 release 产物无钩子/禁串收据仍待 CI。
- 按现行 BSP 锁原 grep/WHITELIST 双向集合静态复现，hits=23、whitelist=23、stray=[]、stale=[]，未改锁判据。
- 原 admission_tests 除 P2 单个函数体外逐字保持；v2/mod.rs 精确对拍只有两个 WriterScope 调用目标替换。state/prepared/generation、ContentBi/producer、根 open_db/迁移/关闭 helper、Clamp/physical、Python 调用方及测试、history36h 共11个保护文件与 HEAD 逐字相同。
- 已由两名非作者代理独立只读审查入口区分、mixed 兼容、五新锁、P2 输入、两锁次序、S1K 失败分类与析构、输出槽借用/cfg。审查发现的正控 trace 少算在线 g4 已修成 `[1,2,3,4]`，同库正控也已加强。native 名单另作独立535项提取和别名入口复核，详见上方栈表。

本机未运行 cargo/rustc、Rust 测试、新 CI、生产构建或重型窗口；没有扩大默认线程栈、设置 RUST_MIN_STACK、另起大栈线程、改 profile、ignore、删断言或更改资源帽。Rust 编译、真实帧尺寸和具名测试结果仍须主控封包后在精确新 head 验证；ready 仅表示本轮指定修复候选可审，不等于 native535项或整票验收已通过。表中另列的旧 bootstrap 1字节预算夹具问题未扩修。

MainOnly/非 WAL 合法输入域、热 journal 写前拒绝、完整性拒绝源字节保全、自身失败正常关闭/新进程重启、台账四文件精确登记/实际字节/64 GiB帽、未知旁挂拒绝、TestOnly Ordinary 先审后迁、E6 与 FU16 保持。S1K 的 Python recover 参数迁移、失败关闭分类和移除临时 push 触发均保持。

### S1L 实际改动文件

- `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md`
- `.github/workflows/ci.yml`
- `rust/src/bin/s_session_v2/mod.rs`
- `rust/src/bin/s_session_v3/admission.rs`
- `rust/src/bin/s_session_v3/admission_private/mixed.rs`
- `rust/src/bin/s_session_v3/admission_private/mod.rs`
- `rust/src/bin/s_session_v3/audit_owner.rs`
- `rust/src/bin/s_session_v3/audit_owner/admission_tests.rs`
- `rust/src/bin/s_session_v3/audit_owner/legacy_admission_tests.rs`
- `rust/src/bin/s_session_v3/driver.rs`

## S1M

> 状态：ready，供主控封包后取 CI。基线为 `sandcastle/issue-1481` / `97cc94eea9ccbeac3763e5e6624aeec6801f65b5`，开工工作区干净。仅修 S1L 测试取参编译错误并核查同类引用；**Rust 未验证，待 CI**。

### 原始证据与修法

已读本报告 S1L、S1D–S1L 提示及所引 S1/S1B/S1C 范围与禁止项，并直接读取 `/Volumes/AgentStorage/h1-recover45c/ci-97cc94ee/125057523022.log`、`125057418479.log` 原日志。以下日志位置为原件物理行；源码行号为本轮候选，Rust 简称相对 `rust/src/bin/`。

| 原日志 | 首错与证据边界 |
|---|---|
| `125057523022.log:780,792,795–813`，rust-check | `cargo check --all-targets --features s_session,s_incremental`，`RUSTFLAGS=-D warnings`，报 `legacy_admission_tests.rs:163:39` E0425，最终只有 1 个编译错误。`:549/:718/:770` 分别是 default、backtest_bin、s_session 档完成；后续 native-testonly 档尚未执行。checkout `:167` 是合成 merge `ec5079b`，由 `97cc94ee` 合入 `eab5dd10`，不是把工位 HEAD 当作 CI checkout。 |
| `125057418479.log:428–429,496–514`，history36h | release、`s_session,s_incremental`、`--bin s_structure_session h1_` 的 test 构建报同一 E0425。测试尚未执行，不能据此记任何 H1 PASS。 |

根因是测试把取参函数写成 `crate::arg`，根模块没有该项；日志列出的 `s_session_v2/mod.rs:3247` 是 v2 私有函数，不能从测试跨模块调用。`legacy_admission_tests.rs:163–168` 改为在本测试分支内用 `args.windows(2).find(|pair| pair[0] == "--db").map(|pair| pair[1].as_str()).unwrap()` 取值，再构造拥有路径的 PathBuf。参数由同文件 `:55` 的夹具生成，`--db` 与路径相邻且唯一，随子进程环境序列化传入；保留原来取值失败即 unwrap 的行为。借用只用于构造 PathBuf，不逃逸。

没有提升 `v2::arg` 可见性，没有改生产函数、导入、cfg、测试名或断言。字节对照确认 legacy 全文件仅此一条语句替换；S1L 另一个改写测试所在 `admission_tests.rs` 全文件与 HEAD 逐字相同。

### 显式路径与私有项穷举

范围由 `git diff 614823a0b2 97cc94ee -- rust/` 确定：S1L 新测试面是 legacy 全文件和 `admission_tests.rs:157` 的冷前缀测试体。其余改动为生产代码及 include 接线，没有第三份新增测试。原错误路径消除后，legacy 剩余 `crate::` / `super::` 共 6 处，逐一如下。

| legacy 引用位置 | 定义与可达性 |
|---|---|
| `:22/:179`，`crate::policy_records_missing` | 根 `s_structure_session.rs:521` 私有函数；调用者是根模块后代，可达。 |
| `:57`，`super::super::admission_anchor::test_prepare` | 两级 super 回到 v3；模块在 `s_session_v3/mod.rs:47` 为 pub(crate)，函数 `admission_anchor.rs:1641` 为 pub(crate)、`:1640` 限 cfg(test)。调用项 `:54` 也显式 cfg(test)。 |
| `:114/:171`，`super::super::dispatch_fixture` | `s_session_v3/mod.rs:203` 为 pub(super)，可见范围为根，`:202` 限 cfg(test)，覆盖本测试。 |
| `:199`，`super::super::writer_cli::GATE_FIELDS` | writer_cli 私有模块 `s_session_v3/mod.rs:110` 对 v3 后代可见；`writer_cli.rs:41` 常量 pub(super)，范围为 v3，`:40` 限 s_incremental，正好与 include 配对。 |

以下包括通过 glob 导入或同作用域使用的受限项，不把“没写 crate/super”当作已核可达。

| legacy 调用或访问位置 | 定义与可达性 |
|---|---|
| `:116/:120` v2::dispatch_fixture；`:121` v2::legacy_write_lock | `s_session_v2/mod.rs:3273` pub fn（`:3272` cfg(test)）、`:3395` pub fn；根的私有 v2 模块对根后代可见。 |
| `:112/:127/:170/:173` state::full_audit_calls_for_test；`:171` state::traced | `state.rs:154` pub(super)、cfg(test+s_incremental)；`:2252` pub(super)、cfg(test)。可见范围均为 v3，门控覆盖调用。 |
| `:180` meta_i64；`:85/:205/:209` canonical_json；`:85/:209` b2_envelope | meta_i64 为根 `s_structure_session.rs:1226` 私有函数；canonical_json 由根 `:86` 导入；b2_envelope 为 `audit_owner.rs:5964` 同一 construction_tests 模块私有函数。 |
| `:4/:5/:55/:79` H1AdmissionFixture 与 db/profile、new/conn | `admission_tests.rs:4` struct、`:6/:7` 字段、`:10` impl、`:11/:51` 方法；include 后与 legacy 同作用域，私有项可达。 |
| `:6/:8/:9/:15/:176/:245/:175` 共享 helper | seed_three→`admission_tests.rs:779`；driver→`:511`（cfg(test)）；ledger_path→`:525`；assert_clean_close→`:947`；qualified_cuts→`history_clamp_tests.rs:783`。均在同一个 include 作用域。 |
| 本文件全部私有 helper 调用 | prepolicy_mainonly `:74/:187`→`:4`；assert_prepolicy_delete `:16/:129/:169`→`:20`；all_source_bytes `:108/:128/:130`→`:29`；cli_args `:80/:197`→`:55`；write_rejects_before_migration `:136/:142/:148/:154`→`:73`；assert_write_rejected `:75/:195`→`:79`。后三项有显式 cfg(test)，均有实际测试调用。CHILD 是同函数 `:160` 的局部常量。 |

`admission_tests.rs:157–243` 的 S1L 冷前缀测试体没有 `crate::` 或 `super::`，其全部受限引用及共享 helper 继续核到定义：

| 调用或访问位置 | 定义与可达性 |
|---|---|
| `:161/:164/:223` fixture::new、h1_admission_driver；`:168/:181` 两种 envelope | 同文件 `:11/:511/:530`，b2_envelope 在 `audit_owner.rs:5964`；均同 construction_tests 作用域。revision_envelope 的 `:537` `crate::tests::located_ohlc_input` 定义在根 `s_structure_session.rs:6694`，pub(super) 范围为根，tests 模块 `:6084` 受 `:6083` cfg(test) 限制；driver helper `:523` 的 test_prepare 与上表相同。 |
| `:183/:186/:222/:226` state 计数；`:223` state::traced；`:187` meta_i64 | 定义及门控同 legacy 表，均可达。 |
| `:184` driver.handle；`:218/:237` driver.h1_owner_for_test | `driver.rs:651/:528` 均 pub(super)，范围 v3；owner 方法 `:527` 限 test+s_incremental+unix，与 include 一致。 |
| `:187/:190/:216` fixture.db/conn/full_canonical | 同文件 `:6/:51/:82` 私有项，同模块可达。full_canonical 的 state::audit_native_current 在 `state.rs:183` 为 pub(super)，其 h1_canonical 在 `:4111` 为 pub(super)、cfg(test+s_incremental)。 |
| `:218/:237` h1_admission_canonical；`:232` h1_qualified_cuts | 同文件 `:98` 与 history_clamp_tests `:783`。canonical helper 访问 owner.complete（`audit_owner.rs:270`）及 complete.audited（`:3214`），均为定义模块后代，私有字段可达。 |
| fixture 的既有构造依赖 | prepared::tests `prepared.rs:1876`、Fixture `:1944`、new `:1956`、db_path `:2009` 均 pub(crate)，模块 cfg(test)；v2::ClockPlan `v2/mod.rs:78` 为 pub struct、parse `:83` 为 pub(super)，init_v3 `v3/mod.rs:492` 为 pub(super)。均覆盖调用。 |

导入链为 `audit_owner.rs:5632 use super::*` → `audit_owner.rs:6 use super::*` → `s_session_v3/mod.rs:32 use super::*` → 根 `s_structure_session.rs:82` Path/PathBuf、`:86` canonical_json、`:93` Connection、`:94` json/Value 及根模块声明。两个测试面没有新增 use；std、rusqlite、serde_json 的显式路径及其迭代器、路径、连接和进程方法均为公开 API。

### 三档 cfg 与告警风险

父 `construction_tests` 在 `audit_owner.rs:5630` 限 test+s_incremental；`:5635–5640` 对 history_clamp_tests、admission_tests、legacy_admission_tests 三个 include 同加 unix。三份 include 展开在同一个模块，文件分拆不形成私有 sibling 边界。

| features | 非 test | test |
|---|---|---|
| s_session | 父模块排除，无测试路径或导入进入生产。 | 缺 s_incremental，父模块及两个测试面排除。 |
| s_session,s_incremental | 缺 test，父模块排除。 | Unix 下三个 include 同时存在；state 计数/trace、Driver owner 查询及所有 fixture 均同门或更宽，无悬空引用。 |
| s_session,s_incremental,s_capture_native_testonly | 同上，测试代码排除。 | 同一可达性；legacy `:233/:234` 只给已构造的 child 追加 native 环境。child 在两档都用 args/env 的可变方法，不产生 unused_mut。 |

非 Unix 时，三个 unix include 同时排除；不据此声称整个既有 bin 已支持非 Unix。Cargo `required-features=["s_session"]` 保持，default/backtest_bin 不编入该 bin；native-testonly 在 Cargo.toml 中依赖 s_incremental，现行 CI 写 `s_session,s_capture_native_testonly` 与表中第三档等效。

PathBuf 修改后仍使用，没有新增 helper、import、变量、allow/expect lint 豁免或生产可见性变更。取参闭包返回借用，PathBuf 随即复制路径内容，没有从借用移出 String、临时借用逃逸或错误类型推断分支。上述是 `-D warnings` 风险的源码穷举，不能替代编译收据。

### 实际验证与保持项

- `rustfmt --edition 2021 --check rust/src/bin/s_session_v3/audit_owner/legacy_admission_tests.rs rust/src/bin/s_session_v3/audit_owner/admission_tests.rs` 通过，未使用 skip_children。
- `git diff --check` 通过。Python 对照 HEAD 确认 legacy 仅替换一条取参语句、全部断言及其他字节保持，admission_tests 全文件保持；本报告历史前缀保持，只追加 S1M。
- `python3 -B -m unittest s_session.tests.test_tb01c_r4_anchor_lock -v`，3/3 通过。
- `python3 -B .github/scripts/check-prod-no-test-hooks.py`，violations=0；`python3 -B .github/scripts/gen-prod-banned-literals.py --check .`，清单与源码一致，未改清单。
- 按现行 BSP 锁原 grep/WHITELIST 双向集合静态复现：hits=23、whitelist=23、stray=[]、stale=[]，未改锁。
- 非作者子代理独立只读复核唯一代码差异、两个测试面的私有项/cfg/导入与断言保持，未发现确定阻断；该结论仅为源码评审。

没有运行 cargo/rustc、Rust 测试、CI、生产构建或重型窗口，没有 git add/commit/push 或写 Git metadata。所有生产代码、workflow、资源帽、S1K 关闭分类与调用方迁移、S1L 旧入口拒门/冷 seed/拆帧保持。Rust 编译、三档 test/非 test 的 `-D warnings` 及 H1/native 运行结果待主控封包后精确新 head 的 CI；本轮 ready 只表示指定修复候选可交接，不表示 H1 或整票验收通过。

### S1M 实际改动文件

- `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md`
- `rust/src/bin/s_session_v3/audit_owner/legacy_admission_tests.rs`

## S1N

> 状态：ready，交主控封包后取精确新 head 的 CI。基线为 `sandcastle/issue-1481` / `f39e32cb29551ffd92a0325297f4f5c1bafe19e1`，开工工作区干净。根因为测试后验只读连接重建旁挂；生产关闭恢复未发现缺口。本轮仅修夹具，**Rust 未验证，待 CI**。

### 原始证据与根因

已读本报告 S1K 的 46L P1-2 失败分类、S1L、S1M，以及 S1D–S1M 提示和所引 S1/S1B/S1C 边界。工位报告的实际路径是本文件，根目录没有另一份 REPORT.md。以下日志行号指原件 `/Volumes/AgentStorage/h1-recover45c/ci-f39e32cb/125061567066.log`，Rust 简称相对 `rust/src/bin/`；源码行号为本轮候选，明确标注基线者除外。

原日志 `:154`、`:242` 确认为精确 head `f39e32cb`；`:423–424` 是 release、`s_session,s_incremental`、`h1_`、单测试线程的执行命令；`:494` 实选 77 项，`:710–713` 是 `admission_tests.rs:951` 的直接清理断言 panic，`:893` 汇总 76 passed / 1 failed。没有把这个旧 head 的结果记为修后 PASS。

问题是 **advance 已关闭后，夹具又开了一个最后关闭的只读 WAL 连接**。连接没有设置 `NO_CKPT_ON_CLOSE=true`；其只读合同本身就使最后关闭检查点无法成功，因此仍留下 `-wal/-shm`。完整生命周期如下。

| 阶段与文件:行 | 连接及关闭语义 |
|---|---|
| `audit_owner/legacy_admission_tests.rs:4–26`，父进程建库 | `H1AdmissionFixture` 在 `admission_tests.rs:4–8` 只存 anchor、路径和 clock，`:51–53` 的 conn() 每次返回临时连接；anchor 的库与本库不同。seed、冷 driver 及 ALTER/DELETE 的普通连接都已关闭；父进程不持本库 SQLite 连接进入 child。初始只读检查位于 DELETE 域，不是 WAL。 |
| `writer_cli.rs:928–931`，请求预检 | scope 先持锁；`preflight_input` 在 `:484` 开只读连接，函数返回时事务及连接已析构。此时仍是 MainOnly/DELETE，先于 WAL 转换，没有跨入 driver 的活连接。 |
| `driver.rs:489–520`、`:582`，H1 冷认证 | advance 的 retention 入口与 serve 共用冷 context 和 `AuditConnection::open_rebuilt_history_into`。`audit_owner.rs:576` 的 certify_cold 使用私有 FD/VFS；`admission_private/mod.rs:780–796` 关闭 SQLite、撤销注册后才交接，无普通主库连接存活到正式写者退出。 |
| `audit_owner.rs:761–776`，合法模式转换 | 首个连接设 true；`s_structure_session.rs:454–488` 在 EXCLUSIVE 下核原件、转 WAL、采交接凭据。`:477–478` 明写 heap 索引，不建 SHM。转换连接在 `audit_owner.rs:771` 有意带 true 关闭，保留空 WAL 的 inode/stamp；随后 `:776` 重开真正 owner。 |
| `audit_owner.rs:795–870`，迁移、seed/adopt | 首锁复核、migrate/COMMIT、Clamp、第二锁 seed、adopt 后，`:870` 调共同 `h1_normal_close`；根 `s_structure_session.rs:389–400` 核恢复 false。advance 没有绕过此步。 |
| `writer_cli.rs:972–975`、`driver.rs:316–320`，正式推进及输出 | 真实 handle 推进 g4；into_output 先显式 drop(conn)，才移交仅持预算、原件 FD 和锁的 CommandOutput。dispatch 的 emit 不持 SQLite。至此正式最后连接已正常关闭。 |
| `audit_owner/legacy_admission_tests.rs:175–176`，child 第一项介质检查 | 仍要求 cuts `[1,2,3,4]`，紧接着在任何后验连接前断言无 SHM/WAL。若生产退出留下旁挂，仍在此失败，后面的连接不能替生产清理。 |
| 同文件基线 `:177–184`，后验检查及 child 返回 | 原来重新以 `SQLITE_OPEN_READ_ONLY` 打开已是 WAL 的库，查询补列、generation、journal_mode，return 才关闭。该连接重新创建旁挂，且只读最后关闭无法清理。父进程在基线 `:245` 再查无旁挂，于是失败。 |

日志是父进程直接在共享 helper panic；若 child 的 dispatch、cuts 或第一项清理断言失败，父会在子进程状态断言处报带 `stdout=... stderr=...` 的失败包装。日志没有调用栈，不能单凭 helper 行号区分父建库期间的 `admission_tests.rs:512`、legacy `:15` 与父末尾检查。源码证明 child 的后验只读 WAL 连接会产生相同残留，下述探针复现该机制，生产入口穷举则确认 advance 共用成功恢复 false 的实现。三者支持本轮夹具修法；精确 Rust 调用点及修后结果仍待 CI，不把原日志冒充完整动态生命周期记录。

依赖源码证据也闭合此因果。`rust/Cargo.toml:45` 启用 bundled；`rust/Cargo.lock:838–839` 固定 libsqlite3-sys 0.38.2，`:1638–1639` 固定 rusqlite 0.40.2。读取本机对应依赖原件 `/Users/silencehan/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libsqlite3-sys-0.38.2/sqlite3/sqlite3.c`：`:470` 版本为 3.53.2；`:63781–63800` 的 PagerClose 在未禁关闭检查点时向 WalClose 提供检查点缓冲；`:71762` 对只读 WAL 的 checkpoint 返回 SQLITE_READONLY；`:69973–70009` 只有 checkpoint 成功才设 isDelete 并清理 WAL/index。故默认 false 也无法使这个只读连接完成清理。

修码前先用 Python SQLite 3.53.4 实跑两臂探针，随后主控独立复跑。执行器在仓外 `/private/tmp/issue1481-s1n-sqlite-close-probe.py`，命令 `python3 -B /private/tmp/issue1481-s1n-sqlite-close-probe.py`；每次新建临时目录，无手工删除旁挂。主控复跑目录 `/private/tmp/issue1481-s1n-sqlite-close-3t0t4wd1`，结果如下。

| 后验连接，仅做相同查询 | 打开前 WAL / SHM | 查询时 WAL / SHM | close 后 WAL / SHM |
|---|---|---|---|
| `mode=ro`，原夹具 | 不存在 / 不存在 | 0 / 32768 字节 | 0 / 32768 字节，复现残留 |
| `mode=rw`，修后夹具 | 不存在 / 不存在 | 0 / 32768 字节 | 不存在 / 不存在，SQLite 自动清理 |

Python 探针验证 SQLite 生命周期机制；依赖原件验证 bundled 的相同拒绝及清理条件。两者均不代替 Rust 锁在 CI 上的真实执行。

### 修法与同类夹具核对

`audit_owner/legacy_admission_tests.rs:177–180` 将迁移后的后验连接改为 `SQLITE_OPEN_READ_WRITE`，不带 CREATE，仅执行原查询；`:187–188` 显式 drop 后再调用原清理 helper。保留 `:176` 的生产关闭检查、`:249` 的父进程最终检查，以及原补列、generation、mode、cuts、旧审计零调用和子进程成功断言。共享 helper `admission_tests.rs:947–955` 逐字保持，未放宽“正常最后连接退出须清理 -shm”。普通读写连接的正常关闭与生产 owner 成功恢复 false 后的 SQLite 生命周期一致；第一项断言保证它不能代替生产连接补做清理。

同类夹具扫描：legacy 的另一只读检查 `:20–27` 只核 DELETE 域；`admission_tests.rs` 无 READ_ONLY，其暂时失败/重启锁 `:967/:1007` 和正常退出锁 `:1061/:1086` 均用普通连接并显式关闭，清理检查分别在 `:973/:1018/:1094`。hot journal 的 `:1257` 是故意制造热日志、进程中止的拒绝负控。history_clamp_tests 与 audit_owner 其余测试没有同款“正常关闭后只读重开 WAL，再由父检查清理”的组合。未发现第二个夹具缺口。

### 入口 × 路径的 NO_CKPT 核对

下表 true/false 均指实际 SQLite 配置；完整性拒绝特指 S1K 的冷交接失败分类。恢复 false 失败仍返回错误及关闭失败证据，绝不当成成功。

| 入口 | 成功路径 | 冷开门/交接自身失败 | 冷交接完整性拒绝 | 已交接后的业务或输出失败 |
|---|---|---|---|---|
| serve，`writer_cli.rs:769` → `driver.rs:260/:582` | 共同 owner `audit_owner.rs:870` 核 false；服务返回析构 driver，普通最后关闭 | 开门前无正式连接；开门后按 `s_structure_session.rs:421`、owner `:394/:903` 核 false | `audit_owner.rs:873–903` 合并 Clamp/原件/版本失配或失证，保留 true、源字节保全 | 已是 false；startup/resume/identity/transport 失败沿原事务及析构语义，无重设 true |
| advance，`writer_cli.rs:933` → `driver.rs:489/:582` | 同上核 false；`driver.rs:316–320` 输出前 drop(conn) | 同一分类收口，预检失败则尚未开正式连接 | 同一分类，不因一次性命令而放行或重试 | handle/summary/emit 失败时已是 false；成功后的夹具后验连接本轮修齐 |
| recover，`writer_cli.rs:994` → `driver.rs:837/:881` | 同一冷 owner 核 false；`driver.rs:892–893` Recover 后显式 drop | 冷构造与 seed/adopt 同一收口 | 同一分类，保留 true | Recover 自身事务失败时也已是 false，`?` 析构；不改 epoch/资金恢复语义 |
| serve 内 accept/Begin/Prepare/commit/pending resume，`driver.rs:647/:651` 起 | 复用已核 false 的 driver.conn，不另开正式连接 | 若失败在 startup 的冷交接前段，仍走上面共同分类 | 同上 | 复用 false，走既有事务回滚/retire，不新增关闭配置切换 |
| 独立 accept、无选择器旧 v1 accept/advance/recover、v2 clock recover | v3 无独立 accept 分派，`mod.rs:286–289` 拒未接线命令；旧入口在 `admission_private/mod.rs:448–450` 对 v3 只读拒绝 | 未开正式连接，无配置恢复义务；v1/v2 域旧行为保持 | 不进入 H1 交接 | 不进入该路径 |

所有生产设置点及旁路连接也已穷举，未发现 advance 漏掉 S1K 恢复。

| 连接类别 | 配置与成功/失败关闭 |
|---|---|
| MainOnly/非 WAL 一次性转换连接，`audit_owner.rs:761–776` | 成功有意保留 true 至 drop，保空 WAL 的身份交接；EXCLUSIVE 索引在 heap，无 SHM。`s_structure_session.rs:491` 对自身失败核 false、完整性失配保留 true。真正 owner 随后恢复 false，不能把转换连接的 true 当遗漏。 |
| 冷认证私有 FD/VFS，`admission_private/fd_wal.rs:1149` | 有意 true，保护只读原件；`admission_private/mod.rs:780–796` 在结果交接前 close/撤销注册，私有 VFS 不提供 xShmMap，不是晚关的普通读连接。失败沿同一收尾，不参与正式关闭恢复。 |
| ledger，`audit_ledger.rs:487–495` | 独立 `.audit-ledger.sqlite`，DELETE/FULL，默认 false；成功/失败均沿原关闭，不占主库最后连接。scope 的 `admission.rs:59–73` 和 `admission_provider.rs:99–109` 只持 FD、预算、锁，无 SQLite。 |
| `admission_anchor/native_n1.rs:1667` | true 属文件头明写的 #1527 受限候选，未接生产 main，不是本票 H1 正式入口；保留既有候选测试。 |
| `admission_tests.rs:394/:1325` | true 是外写完整性负控为保全字节而故意设置，原断言及关闭方式保持。 |
| 本轮后验检查连接，legacy `:180` | 默认 false，READ_WRITE 不 CREATE；查询后显式 drop，再要求无 SHM/WAL。原只读连接未设 true，问题在只读 WAL checkpoint 拒绝。 |

### cfg、实际检查与保持项

| features | 非 test 源码穷举 | test 源码穷举 |
|---|---|---|
| `s_session` | 父 construction_tests 被排除，无新增生产引用 | 缺 s_incremental，整个新测试排除 |
| `s_session,s_incremental` | 缺 test，夹具代码排除，生产关闭分类逐字保持 | `audit_owner.rs:5630` 的 test+incremental 与 `:5639` 的 unix include 覆盖修改；OpenFlags 公共常量、drop 和共享 helper 均可达 |
| `s_session,s_incremental,s_capture_native_testonly` | 同上，无新增生产测试钩子或捷径 | 与上档相同；原 child native 环境门不变，未添加环境读取或线程配置 |

非 Unix 仍排除该 include；default/backtest_bin 的 required-features 行为保持。没有新增导入、helper、未用变量或 lint 豁免；conn、mode 继续原用途，drop 消费连接后无再借用。上述六格是 `-D warnings` 风险的源码穷举，实际编译及告警检查待 CI。

实际检查：

- `rustfmt --edition 2021 --check rust/src/bin/s_session_v3/audit_owner/legacy_admission_tests.rs`、`git diff --check` 通过。
- `python3 -B -m unittest s_session.tests.test_tb01c_r4_anchor_lock -v`，3/3 通过，锁文件未改。
- `python3 -B .github/scripts/check-prod-no-test-hooks.py`，violations=0；`python3 -B .github/scripts/gen-prod-banned-literals.py --check .`，源码与禁串清单一致，无须重生成。
- 按现行 BSP 原 grep/WHITELIST 双向集合静态复现：`python3 -B /private/tmp/issue1481-s1h-bsp-scan.py`，hits=23、whitelist=23、stray=[]、stale=[]，未改锁判据。
- 上述 SQLite 两臂探针通过；Python 字节对拍核定唯一 Rust 差异为一处连接 flags、两行注释、显式 drop 与新增同一 helper 调用，原全部断言保持；除该夹具和本报告外无改动，报告历史前缀保持。
- 非作者独立代理只读核对根因、全部入口/连接设置点、同类夹具、最终差异及六格 cfg，未发现确定阻断。独立性来自非作者进程；结论限源码与 SQLite 机制证据，不冒充 Rust 实跑。

生产准入 SHM 判据、转换及完整性拒绝的源字节保护、S1K 自身失败恢复/监督器调用方迁移、S1L 旧入口拒门与冷 seed/拆帧、MainOnly/非 WAL 合法域、台账四文件精确登记/实际字节/64 GiB 帽、未知旁挂拒绝、Ordinary、资金、E6 与 FU16 全部保持。没有手工删 SHM/WAL，没有改 workflow、禁串表、资源帽、断言助手或已有测试名；原 H1 CI `ci.yml:473` 的逐名项继续执行该正控，无新增测试名需接线。

本机未运行 cargo/rustc、Rust 测试、CI、生产构建或重型窗口，未操作 git add/commit/push 或 Git metadata。主控须在新精确 head 取三档 test/非 test `-D warnings` 以及原 H1/native CI 收据；本轮 ready 仅为修复候选可交接，不宣称 H1 77 项已转绿或整票验收。

### S1N 实际改动文件

- `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md`
- `rust/src/bin/s_session_v3/audit_owner/legacy_admission_tests.rs`

## S1O

> 状态：ready，仅文档候选。基线 `sandcastle/issue-1481` / `da6d27043ee38e15bb13a805966269b22d38ba49`，开工工作区干净。沿 #1481 本阶段明确授权补 46Q P2-A，不改生产代码、Python、workflow 或测试，不扩大代码层 PASS 的含义。

已读取 `/Volumes/AgentStorage/h1-delta46q/REPORT.md` 全文、上文 S1K 失败分类表、ADR 0042、README 恢复段与当前代码。本节追加历史报告，不追改 S1K 的恢复主张。根目录没有另一份 REPORT.md，本文件是实际工位报告。

### 新增位置与恢复结论

- `docs/runbook_production_session.md:98-183` 新增「H1 写入完整性 fail-stop」；`:102` 触发类别，`:133` 停机保全，`:177` 人工恢复门。
- `s_session/README.md:123` 在既有恢复示例之后加一行指针；原 `:111-120` 的合同与命令未改。
- **保全后停在 #1481 人工恢复门。** ADR 0042 `docs/adr/0042-s-writer-incremental-production-path.md:38-42` 只批准监督器 recover → 增量持久态重建；README `:111-120` 只给既有停止、recover、start-s 合同，具体示例限冻结 op95/epoch 2/`recover-001`。两者均未批准完整性拒绝后以普通 SQLite 连接正常关闭清旁挂，故没有编造或列出该恢复命令。运维负责人携原件/日志/元数据与停机证据交 #1481 主控，由主控提交用户/恢复合同裁定人决定介质恢复、可信原件和恢复事件；批准前保持停写。runbook 明写「禁止手工删除 -shm/-wal」，并说明若以后批准普通连接清理，会把 WAL 检查点进主库，之后仍须 recover/start-s 完整 H1 冷认证。

### 逐项类别出处

下表 Rust 路径相对 `rust/src/bin/`；runbook 表格使用完整仓库相对路径。

| 类别 | 分类器与原件判据 |
|---|---|
| `SourceIdentityChanged` | `s_structure_session.rs:409`；`s_session_v3/admission_provider.rs:75-86` |
| `SourcePathLost` | `s_structure_session.rs:410`；同 provider `:77` |
| `SourceFdLost` | `s_structure_session.rs:411`；同 provider `:76` |
| `SourceMetadataUnavailable` | `s_structure_session.rs:412`；同 provider `:50`、`:64` |
| `SourceWalChanged` | `s_structure_session.rs:413`；同 provider `:141-149`、`:400-402` |
| `SourceTypeOrHardlinkUnsupported` | `s_structure_session.rs:414`；同 provider `:50-52` |
| `SourceChangedDuringPin` | `s_structure_session.rs:415`；同 provider `:64-66` |
| `ColdConversionMediumUnsupported` | `s_structure_session.rs:416`；同 provider `:388-390` |

Clamp 的 `HistoricalAuditStale：` 前缀由 `s_session_v3/audit_owner/history_clamp.rs:14-15` 生成；实际完整性置位分支在 `:45-65` 的主库/WAL 打开或 stat 失证及「路径与持有文件身份/stat不一致」，`:144-149` 的「H1组件要求既有WAL域，不自动改变journal_mode」，`:203-221` 的 WAL 头读取失证及「文件观测期间发生变化」，`:229-254` 的「锁内观测期间文件变化」，`:258-263` 的「私有认证交接期间发生外部SQLite提交」，`:266-270` 的「版本、文件、控制原件、写计数或连接域改变」。底层 I/O 错误没有固定子类，须保留动态原文。

`s_session_v3/audit_owner.rs:879-896` 在迁移后失败收口复核 `data_version`，不等报同名外部 SQLite 提交，读失败报「失败后源连续性失证：{error}」，均置完整性标记。runbook 按实际置位判据逐项写，没有把所有 `HistoricalAuditStale` 一概归完整性，资源错误同前缀见 `history_clamp.rs:422-425`。

自身失败与完整性失败的区别依据 `s_structure_session.rs:402-430`、`audit_owner.rs:873-903`：未发现完整性问题且连续性复核通过才恢复正常关闭，原错误仍报；关闭恢复本身失败则附加关闭错误。`SourcePinFailed` 明确走自身失败；当前 `SourceWalMetadataFailed` 未在八类中。拒启条件逐字取 `admission_provider.rs:231-232` 的 `shm || (journal && wal_index.is_some())`，没有误写成单独 WAL 存在即拒。

### 命令与保全清单出处

- runbook `:138-143` 的 `service status → service stop → service status` 逐字复用 README `:74-75` 与 `:107-108`；README `:111` 明确 stop 按 Q→S、只操作登记且匹配的进程、超时不自动升级信号。另须人工确认所有外部进程与连接已停，不能把 status 当作外部连接不存在的证明。本轮未对任何服务执行这些运维命令。
- runbook `:154-173` 是本阶段获准新增的文件保全示例，只有参数检查、建全新证据目录与 `cp -RPp` 复制，没有恢复操作。子 shell `set -eu` 保证复制失败不继续。主库旁挂 `-wal/-shm/-journal` 与四个 audit-ledger 名称对齐 `admission_provider.rs:117-123`；其余控制/写锁/owner 原件和整个原控制目录一并复制。原件不移动、不删除，符号链接与悬空链接也保全；设备号/inode、大小、mtime 单独登记，复制后复核原件与副本。
- 写者 stdout/stderr 同写 `s.log` 的出处是 `s_session/s_service_control.py:550-553`；`s.process.json/q.process.json` 的出处是同文件 `:329-330`。外部 stderr、配置、冻结资源若在目录外，逐份复制并记路径映射。未提供普通 SQLite 连接、手工检查点或改日志模式命令。

### 代码注释与后续记录

`rust/src/bin/s_session_v3/audit_owner.rs:783-788` 的现注释，特别是 `:785-787`，写了“普通连接正常关闭完成介质恢复；核无 SHM 后 service recover → start-s”，并援引 ADR 0042/README。普通连接清旁挂这一步在所引两份已批准合同里没有依据，超出本轮能确认的授权；本 runbook 因而停人工门。S1K `:1228` 的同类文字也是历史主张，不能补足批准依据。本阶段按要求只登记分歧，不改代码注释或历史报告，交 #1481 主控随恢复合同裁定收口。

46Q 的以下 P3 均仅登记为 **#1481 后续**，不在 S1O 实施、不新建票。独立只读代理已用 `gh issue view 1481` 确认本票仍 OPEN，职责含 U-A 夹持、冷认证和对拍锁，适合承接三项。

| 46Q 项 | 在案事实与建议归属 |
|---|---|
| P3-B | `admission_provider.rs:153` 的 `SourceWalMetadataFailed` 丢失 errno，但未列入完整性类；与 `SourceMetadataUnavailable` 的保守分类口径不齐。挂 #1481 原件失证分类后续，不改现状。 |
| P3-C | `audit_owner/history_physical.rs:23-40` 的 CommitMonitor 复制 Clamp 时新建独立 Cell，提交观察置位不回传主 Clamp。当前冷交接第二锁只读，不经过 CommitMonitor，之后已恢复正常关闭，本轮关闭策略影响面为零。挂 #1481 夹持状态一致性观察。 |
| P3-D | `s_session_v2/mod.rs:4798-4805` 的旧测试直接用 `WriterScope::admit`/None 模拟 recover，未代表现役 `LegacyWrite` 入口。挂 #1481 入口回归锁整理，不改测试。 |

### 自检与边界

- 新增 Markdown 相对链接、H1 标题锚、源码行号和错误原文逐项核对；八类与分类器集合一致，六个 Clamp 固定文本与源文件一致。README status/stop 命令逐字比对通过；两块新增 shell 示例经 `bash -n` 通过，仅解析未执行。
- 非作者独立代理只读核对代码判据、合同授权与文档差异，确认人工门及出处准确；采纳其符号链接保全意见，复制使用 `-RPp`，存在性同时检查 `-e/-L`。
- `git diff --check` 通过。`.github/prod-banned-literals.txt` 只用于 Rust 测试字面抽取及生产二进制检查，不覆盖 Markdown，依据 `.github/scripts/gen-prod-banned-literals.py:4-9`、`:22` 和 `.github/scripts/check-prod-artifact-hooks.sh:1-5`；仍额外运行 `python3 -B .github/scripts/gen-prod-banned-literals.py --check .`，结果“与源码一致”，未改禁串表。
- 差异仅下列三份 Markdown，本报告原 2201 行前缀逐字保留。没有运行 Rust/Python 测试、构建、CI、生产恢复或重型窗口，没有 git add/commit/push，也未写 Git metadata。本轮 ready 只交文档增量审，不宣称人工恢复方案已经批准或 #1481 整票验收完成。

### S1O 实际改动文件

- `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md`
- `docs/runbook_production_session.md`
- `s_session/README.md`
