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
