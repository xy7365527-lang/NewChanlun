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
| launcher/5、Python 服务控制、ADR 修订 | 留 S2 | 未改，Ordinary `/1` 与旧二进制 launcher/4 合同保留 |
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
