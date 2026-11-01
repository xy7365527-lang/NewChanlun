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
