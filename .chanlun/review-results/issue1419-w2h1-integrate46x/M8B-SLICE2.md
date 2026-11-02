# #1419 M8B：片 2 受管同事务读回

名分：绑定 #1419 的阶段工作草稿。开工基线 `sandcastle/issue-1419@30770b7cd11e91716a939f7d5a6e4803b1d1b5bb`。下文既有核的行号以该基线为准；新实施的行号以封包工作区为准。本片不启用淘汰或回收，第 4 片继续受八项证明义务约束。当前累计 capture、成功表及其他全史状态的持有事实不因增加回读接口而消失。

已全文读取本目录 [M7D-DESIGN.md](M7D-DESIGN.md)、[M8A-SLICE1.md](M8A-SLICE1.md)、M2-NOTES 的 M8A 节，以及 `/Volumes/AgentStorage/m8a-review48z/REPORT.md` 与 `/Volumes/AgentStorage/m7d-design-review48x/REPORT.md`。写作按 `.agents/skills/writing-for-agents/SKILL.md` 组织证据与未完成门，按 `/Users/silencehan/.agents/skills/unslop/SKILL.md` 校订措辞。两份技能只管文档写法，不替代本票合同或验收。

## 1. 首片说法订正与本片边界

本节明确取代 M8A-SLICE1 §5 及 M2-NOTES「M8A」中“四个 pair 是独立旧路径对拍”的说法。原记录保留，四锁继续保留，准确名称为“包装透明锁”。`direct()` 与 `read_checked()` 都执行 `verify_owner_refview → OwnerCapture.read → 错误 revoke`，所锁的是包装不改变正文、错误与 typed origin、F/reads/peak、父账事件。它们未构成对真实生产读者的独立差分，不能作为本片第二项必修的证据。

四锁分别为 `m8a_step_reader_body_context_and_permit_events_match_capture`、`m8a_step_reader_full_reference_mode_and_json_flavor_keep_first_errors`、`m8a_step_reader_missing_corrupt_and_partial_success_revoke_without_retry`、`m8a_step_reader_budget_and_origin_failures_preserve_typed_source_and_release`。它们的完整前缀仍是 `s_content::step_reader::tests::`。

本片只在测试接入同事务回读与真实生产读者差分。生产调用点迁移仍留第 3 片；新后端的 SQL/F/许可事件若与既有调用路径不同，须如实登记，不能据语义输出相同便宣称生产观测逐位不变。原 M3 四锁、admission 六代、certified/qualified、effects、少 1B/首错来源、ordinary full_some 及 M7D §5 的全部验收保持。

## 2. 异父工作域与撤销语义

本轮只读回查了现核、撤销点与原合同。结论分入口阶段，不能用一个“异父必撤销”概括。

| 位置 | 逐字原文或代码 | 可据此作出的结论 |
|---|---|---|
| `rust/src/s_content.rs:3943-3945` | “仅供实际 owner 的 append-only/effects 续接：持同一个原件 owner”，“不接受外来 source、成功集合或资格 bool。调用方负责真实事务/effects/COMMIT。” | owner 身份与事务夹持由封闭调用域保证，核没有接受任意外来成功表的入口。 |
| `rust/src/s_content.rs:3955-3956` | “零新增的 warm hit 同样必须核当前工作域健康，不凭旧容量绕过停止门。”；`let _health = work.reserve_native(0)?;` | 每次核当前 WorkScope 的健康。此处没有逐节点扫描旧正文许可的父身份。 |
| `rust/src/s_content/closed.rs:776-795` | `let _health = work.reserve_native(0)?;`；`capture.original(d)`；`let actual = frame(tag, body);`；`Ok((tag, VerifierBody::Borrowed(body)))` | 原 OwnerCapture 读核借真实正文核摘要，没有 capture 父账与 work 父账相等检查。健康的 foreign 父账在旧核中可能成功，旧核并没有“异父必拒”的事实。 |
| `rust/src/s_content.rs:3998-3999` | `if result.is_err() { self.owner_codec.borrow_mut().revoke(); }` | 已进入 verify_owner_refview 后，实际发生的任何 Err 都撤销 owner 成功能力，包括当前 work 健康/资源失败。新接口继续保留这个行为。 |
| `rust/src/s_content.rs:3525-3530` | “零增长也核本次活动域与原许可同父，含容量为零的正文。”；`body.try_reserve_native(work, 0)?;`；`if !matches!(&result, Ok(true)) { self.owner_codec.get_mut().revoke(); }` | append 是原有真正校同父的入口；移交异父正文失败会 revoke，不能把这种移交语义套到只读描述符绑定。普通 append 的对应撤销点在 `:3501-3502`。 |
| `rust/src/s_content.rs:2821-2826` | `self.clear(); self.revoked = true;`；`!self.revoked && self.format == CONTENT_FORMAT && self.predicate_revision == 1` | revoke 清 Public/Template 两套成功树并不可由下一次新 StepReader 自动恢复。 |
| `rust/src/s_content/capture_allocation/scratch.rs:55-61` | “只核父账身份；有效操作域仍由本次reserve_native检查。”；`Arc::ptr_eq(&self.parent, parent)` | 封闭描述符可按一个既有父身份见证作 O(1) 绑定检查，身份相同与当前域健康是两件事。 |

因此，新封闭存储描述符在进入 owner 核之前发现调用者拿错工作域，只拒绝该次绑定，不附加 revoke。它替换的是 M8A 构造器新增的全表父身份扫描，不能把 M8A 测试接缝的额外撤销带进生产。进入 owner 核后发生读、parse、核验或资源错误，仍沿原核撤销并传播第一个 typed origin；append 失败的原撤销也保持。这个入口区分不宣称旧核已具有新的 descriptor 身份门，也不把旧核可能接受健康 foreign 的行为误记为“仅拒本次”。

现有 `codec_original_empty_binding_requires_same_parent_and_live_healthy_operation` 的基线 `s_content.rs:9327-9344` 明确断言异父 append 为 Ledger 且 owner 已撤销；`b2_owner_codec_failed_partial_closure_never_becomes_reusable` 的 `:8730-8763` 锁住核验失败后不恢复能力。两种合同均保留。

原合同查询实际执行 `env -u GH_TOKEN -u GITHUB_TOKEN gh issue view <号> --json body,comments`，1481、1488、1644 均退出 0；随后用同一 `gh issue view` 的 `--jq` 读回下列原文，均退出 0。未向 GitHub 写入内容，未把票面缓存或外卷原件作为新文件入仓。

| 原件 | 本轮回查的逐字原文 | 对本片的约束 |
|---|---|---|
| [#1481](https://github.com/xy7365527-lang/NewChanlun/issues/1481) 当前正文第 15 行 | “夹持失效即 stale 停写，不降级、不回落。” | foreign/stale 及生涯失效不能换源重试。 |
| 同正文第 24 行 | “不常驻全部原件；写者保留结构有具名冻结帽，超帽即启动失败、不降级。” | 加 reader 并未完成原持有合同，不能提前签有界。 |
| 同正文第 26 行 | “代码中不存在「Δ 失败或资源不足 → 旧审计重判」分支” | 新读失败直接返回，不设第二判据或旧源补读。 |
| [#1644 批准评论 6308923666](https://github.com/xy7365527-lang/NewChanlun/issues/1644#issuecomment-6308923666) | “每次重启从原件重核全部历史” | 冷折叠仍须真读全部历史，不能信跨生涯旧能力。仓内对应正本为 `.chanlun/review-results/issue1644-p6-audit-ledger-20261021/AMENDMENT.md:9/:45/:60-61`。 |
| [#1488 追认评论 6313949653](https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6313949653) | “许可改为 RAII，持有原父许可直到真实析构。” | 租约必须真正持 backing，不能只 drop 许可或先退款。评论引用的 `control/1488-budget-full-chain-counterexample-fix-v2.md:9` 是原 P1c 来源；本轮在本工位未找到该原路径，不将其写成已读文件。 |

这些原件没有把“调用者在描述符绑定前拿错工作域”规定为 owner 永久撤销的事件；本片按既有核的实际撤销位置区分处理，不另造合同。

## 3. 四项必修的实施封包

本片实现位于 `rust/src/s_content/step_reader/managed_sql.rs`。它仅由库测试或 `s_capture_native_testonly` 接缝可达；Rust 编译及下列锁的执行全部待 CI。这里的字节与事件是源码及锁的检查目标，不是已经实跑的 Rust 收据。

| 必修 | 本片实施与锁 |
|---|---|
| 同事务受管 adapter | `ManagedSqlStep` 独占借 `&mut Transaction`，持原 `WorkScope`、零字节同父身份许可及上层原采样回调。独占借用封闭提交/换事务/事务控制 SQL；固定三项元数据夹持和 total_changes 另挡测试内部注入的隔离、main/TEMP schema 与行变更。`CheckedStatement::row.blob` 只借 SQLite 行，在 Rust 正文 Vec 复制前 `work.reserve_native(bytes.len())`，按原 origin 持到析构；CheckedFrame 按值接该 Vec，不复制第二份证明正文。首错保存原 CodecError，后续请求返回同一错误且节点 SELECT 不增长，无 stage 或第二来源。首次可达、同摘要异实际正文、真实 SQL 正文移交 append 的逐字异原件拒绝、缺失/篡改、foreign/WAL、原 Job 绝对期限、SQLite progress 取消和 40/39 B 行复制门均有锁。 |
| 真生产差分 | `step_reader/production_diff.rs` 调真实 `capture.reader()` 和 `read_scoped_checked`；`bin/s_session_v2/content_store/m8b_production_diff.rs` 调真实 `TxStore::new(...).with_work(...).checked_read_scoped`，以及 effects 使用的 `read_scoped → attach_native_reservation → append_original_codec_for_owner → verify_owner_refview` 序列。无 `StageOwner` 或 `new_with_stage`。比较实际 tag/body、错误 code/detail/typed 类别与每臂 trace 桥、F、节点 reads、父账事件与保留量；差异如下表。 |
| 租约持有/释放 | `SqlReadLease` 独占借 reader 的步生命周期，不提供 Clone/into_body。实际持 `ScopedBody`；drop 显式 `drop(bytes)`，随后置析构见证，再 `drop(permit)`。父账合同模型逐笔 reserve/release 并记录 live/remaining；40 B 根租约释放原 origin=7，当前父 origin=8 时 live 减 40、remaining 不变，release 端断言正文析构见证已发生。模型核对应下述 native.rs 原实现，不冒称真实生产父账实跑或 RSS 收据。 |
| 去除 O(N) 构造扫描 | `StepReader::new` 删除 `native.nodes.values().any(...)`，仅核活动域和既有索引身份；索引身份不再冒充全体正文同父证明。SQL descriptor 持唯一事务借用及一张原父身份封印，只做固定数量 metadata 查询，不扫描节点、原件或历史表。首片 foreign 锁改为 `m8a_step_reader_foreign_descriptor_rejects_request_without_revoking_owner`，先取得非零成功树，再核错域绑定拒绝不清旧成功，原父可继续验证。四个包装透明锁的名称保留。 |

本片 descriptor 的能力范围限同一步事务和父账，不签发、载入或继承既有 owner 的跨步成功。外连接/写者生涯检查由调用方的原 sample 门承担；测试用真实 WAL 外连接提交及独立 observer 的 data_version 变化证明该回调能够在新增节点读前拒绝，不能把它写成已经接通原 Access 的不可伪造生涯、FD/WAL 或生产资格。那部分接线留第 3 片。独占事务借用不开放连接或原始事务 getter；测试模块直接访问私有字段的 DDL/PRAGMA 注入属于故障注入。

| 对照 | 锁定的相同内容 | 明确不同的物理观测 |
|---|---|---|
| CaptureReader / read_scoped_checked 与新 SQL 节点读 | 正文、缺失/损坏 code/detail、无资源 origin、F；每臂各一次节点读取 | capture 夹具不新签正文许可；SQL 有身份 0 B reserve/release，以及实际行长度的正文 reserve/release。完整序列有定值断言。 |
| 真实 TxStore scoped 与新 SQL 节点读 | 正文、缺失/损坏/少 1B 的错误和 typed 类别、逐臂资源桥、F、一次节点 SELECT、父峰 | 新臂完整父账事件 = 身份 0 B reserve + 旧臂完整事件 + 身份 0 B release；新增 sample/metadata 查询单列。 |
| effects 冷非空 STRING 三节点 append/owner 核与新根租约 | 正文；kind/extent、损坏、Budget 的 code/detail/typed 类别与各自资源桥；同一冷 Verifier 的 Layout 事件序列 | 成功旧 F=10、节点 SQL=3、capture reads=4；新 F=5、节点 SQL=5、capture reads=0。旧臂保留三正文/索引/owner AVL，新臂临时正文与 scratch 归零。新 Native 完整事件逐位定值；旧完整事件按 charge 分解，其净 Native 与实际 retained 对账、Layout 净额为零，未称旧 Native 已逐位重冻。 |

F/reads 数字是新锁中的预期，须由 CI 执行确认。`reads`/`SQL_READS` 与现 `content_store::SQL_READS` 一样只计节点 SELECT，不能代表所有 SQLite statement。新 descriptor 的初验及每次域检查另有三条 PRAGMA 查询，以 `metadata_reads` 独立计数；按成功调用图，STRING 五次节点读另有 21 次 metadata 查询，BOOL 三次节点读另有 15 次，这两数是源码推导、尚非执行读数；原 TxStore::new 的格式/协议 metadata 查询也不在旧节点 SELECT 计数内。sample 在原绝对门上执行，不建立新时钟；原 Job 手动 Clock 锁在同一 first_read_at+1 ms 到期，SQLite progress 锁保留真实 SOURCE/interrupted 错误，回调注入锁另保留首错传播。

`rust/src/s_content/job.rs:380-390` 的 CheckedFrame 把 Vec 按值持有，ScopedFrame 字段顺序是 frame 再 permit；`:523-543` 的 checked 按值接收 Vec 并核 F，不另复制正文。真实 SQL 行复制是单独 allocation，与 capture 里的旧正文不能并作一个 backing。租约 adapter 如改为额外保存证明副本，必须再计该副本真实容量。

旧 origin 的生产依据为 `rust/src/bin/s_session_v3/audit_owner/native.rs:1643-1659`：`origin == d.serial` 才 `d.remaining.checked_add(bytes)`，否则保留 `d.remaining`；total live 仍 `checked_sub(bytes)`。`NativeReservation::drop` 在 `scratch.rs:105-113` 把保存的 `self.origin` 传给原父 release，释放失败使父失效。不能把旧载荷释放额当成当前请求的新额度。

## 4. 八项证明进度

这里记录源码已经证明的持有事实和仍缺的证据。八项目前均未闭合，不将接口、局部许可或尚未执行的锁写成完整有界证明。

| 项 | 本片取得的可核查证据 | 仍未证明与下片所需证据 |
|---|---|---|
| ① owner codec HashSet/AVL 与 AuditMemo | `s_content.rs:2530-2532` 的根 HashSet；`:2739-2744` 五棵 owner AVL。`:2765-2790` 把本步新成功键逐项 insert 到旧树；`:2628-2641` 重复键不增长，新键按实际 `Layout<OwnerProofNode<K,V>>` 累加 retained_bytes。正常 `finish_owner_step :3596-3618` 清请求 memo/dirty proof，却保留完整原件、根及 owner 树。故成功键表在持续出现新单位/比较对的输入上仍随全史累积，AVL 解决每次插入复杂度，未消除总驻留增长。 | 没有历史无关容量界。须迁移非活跃成功描述符到既有可重建台账，并证明完整上下文、模式、版本和撤销保持；给出各树节点数×真实 Layout 与各 HashSet capacity/control 的按代收据。不能笼统把“owner 元数据”豁免出读取工作集。 |
| ① 本步 AuditMemo 的独立部分 | `s_content.rs:2170-2183` 保存 seen/pushed/todo/read_digests 及 bytes/elem/field/key_cmp；`:5550-5562/:5587-5588` 随本次实际读与待核集合增长；`:6090-6092` 比较双向存结果。`:3981/:3986/:3995` owner verify 每次新建 memo、成功合并后析构；普通 capture `:3916-3922` 在成功时留入 slot，finish 才清。 | 本步完整闭包所需 memo 不能由局部 O(h) DFS 栈替代。需真实 Δ 闭包/memo容量、并存时刻及失败析构收据；也需证明冷折叠不会将全史闭包塞入同一步 memo。 |
| ② Counts 子集闭包与 ledger | M8A §2 已列 NativeCountsSubsetReader、三闭包与 ledger 的真实借用；本片不迁移这些生产持有者。 | 仍未证明。须迁移原件长度/角色费来源、整组三闭包接纳/rollback、子集 capture与ledger容量及租约重叠峰，不能靠失效 slice 算账。 |
| ③ 字段/标量/身份缓存 | M8A §2 的 NativeFieldMemo、NativeRecordFields、NativeWitnessIdentityMemo 仍按原方式借 capture；缓存包含真实字符串/PlainJson及字段引用。 | 仍未证明。须逐缓存给容量、生命周期与清理时点，并证明 descriptor 不间接 pin 全 capture。 |
| ④ 三组与发展批成员 | M8A §3 已指出成员、坐标和输出材料是实际工作窗，局部导航路径数不约束这些集合。 | 仍未证明。须按三组、发展批成员、源坐标与输出的实际容量取共存峰，并保留整组判据/角色收费。 |
| ⑤ 多路 edits 共存 | 原 PendingSet 同时持三路 arrays 与可选 requests 的 edits；本片不改该表示。 | 仍未证明。须枚举 changed object、各 edits、输出/旧索引的同时存活区间，并按 capacity 逐项计入 Δ，不以“单路最大”代全部。 |
| ⑥ SQL 行复制与页缓存归属 | 基线 `content_store.rs:137-147` 在 statement.next/row.blob 后进入 native_body_reserved；`:175-184` 在 Rust Vec 分配复制前 reserve body.len，`:207-216` 把该 Vec 按值交 CheckedFrame/ScopedFrame。故基线可证 Rust 行正文副本的复制前许可，不能证 SQLite 取页或转换前已预留。新 adapter 的读前收据另见 §3 封包。 | 页缓存、statement、SQLite BLOB 转换及连接 allocator 的字节归属仍未证明。它们属于同连接 SQLite 内部资源，不能由 Rust body.len 许可覆盖，也不能以 cache_size 建议值作硬上界。须提供页缓存/statement/临时转换实测及其同父归属或完整连接域预算方案，连同 SQL、正文、证明共存峰。 |
| ⑦ 生产栈真实容量峰 | 本片只有测试接缝；M8A 的 k/h 是逐调用族局部形状。 | 仍未证明。须在生产真实嵌套中观测同时存活的 SQL/codec/memo/比较帧/缓存/资格/旧新索引，以真实 capacity/Layout 和父总账峰对账。不能将各函数单独最大值取 max 代总共存。 |
| ⑧ 冷折叠不重攒旧正文 | 本片未淘汰/回收，也未替换冷折叠；finish 仍保全史原件。#1644 冷启动从原件全史重核合同不变。 | 仍未证明。须从空 g1 折叠按步释放非前沿 backing，并量逐代驻留与共存峰、同父/旧 origin账；与在线同代内容/资格/台账逐位对拍，丢台账/recover 原锁全保留。 |

⑥ 的页缓存结论另由既有生产代码 `audit_owner.rs:4516` 的同 active transaction `cache_flush` 佐证：连接 pager 可持脏页，须在 effects 仍活跃时先 flush 再 finish。M2-NOTES 的 M6D 已撤销“CACHE_SPILL 计数正增长证明压力 spill”的旧说法，本片沿用订正；该计数不能证明页缓存容量或具体页归属。这里未重跑旧原型或 Rust 压力锁。

## 5. CI、检查与后续门

现有 `.github/workflows/ci.yml:455-460` 的 native 组调用 `test-feature-delta.sh s_capture_native_testonly`。脚本以 `s_session` 为 base，分别取 lib/bin 的 `--list` 差集后逐名 `--exact` 执行，差集为空失败。`rust/Cargo.toml:84-87` 中 `s_capture_native_testonly = ["s_incremental"]`，因此仅在 test+s_incremental 构建的新锁自然进入该组；不改 CI。脚本真实编译、枚举和执行均待 CI，不由静态阅读签发成功。

新锁共 16 条，以下按实际模块列出。7 条 M8A 锁继续保留，其中 foreign 锁按 §3 改名；原 M3、admission、certified/qualified、effects、full_some 等锁的源码未改。

完整前缀 `s_content::step_reader::managed_sql::tests::`：

- `m8b_sql_first_reachable_missing_corrupt_and_complete_reference`
- `m8b_sql_foreign_stale_and_same_transaction_mutation_are_not_retried`
- `m8b_sql_same_transaction_pragma_main_ddl_and_temp_shadow_change_seal`
- `m8b_sql_closed_transaction_cannot_bind_managed_domain`
- `m8b_sql_same_digest_different_actual_original_fails_without_cached_authority`
- `m8b_sql_actual_original_transfer_keeps_append_duplicate_comparison_and_revoke`
- `m8b_sql_original_job_absolute_deadline_is_not_restarted`
- `m8b_sql_sqlite_progress_cancellation_preserves_source_and_stops_io`
- `m8b_sql_deadline_cancel_and_one_byte_short_keep_first_typed_origin`
- `m8b_sql_row_copy_exact_bytes_and_one_less_preserve_origin_without_allocation`
- `m8b_sql_real_foreign_wal_commit_hits_original_stale_gate_without_retry`
- `m8b_sql_lease_destroys_body_before_original_origin_release_without_refund`

完整前缀 `s_content::step_reader::production_diff::`：

- `production_capture_reader_scoped_body_errors_and_physical_allocation_difference`

完整前缀 `content_store::m8b_production_diff::`：

- `m8b_real_txstore_scoped_matches_sql_body_errors_origin_f_and_parent_events`
- `m8b_effects_real_sql_append_owner_verify_and_managed_root_observation_difference`
- `m8b_effects_nonempty_body_reference_errors_budget_origin_and_role_receipts`

运行步骤是在 `rust/` 下执行 `../.github/scripts/test-feature-delta.sh s_capture_native_testonly`；现有 native 作业会同时枚举并逐名执行 lib/bin 新增集合。可用 `cargo test --locked --features s_session,s_capture_native_testonly --lib -- --exact <上列完整库测试名>` 和对应 `--bin s_structure_session` 命令靶向定位。这些命令本机未执行，需 CI 运行，不能只编译不执行。

最终本机实际检查：

- `rustfmt --edition 2021 --config skip_children=true --check` 覆盖 `s_content.rs`、`step_reader.rs`、`managed_sql.rs`、其 `tests.rs`、库 `production_diff.rs`、`bin/s_session_v2/content_store.rs` 和其 `m8b_production_diff.rs`，退出 0。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py` 同步清单，输出 `count=1774`，退出 0；随后同命令加 `--check` 退出 0，清单与源码一致。仅生成清单，未改生成器。
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py` 退出 0，3 项通过。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/check-prod-no-test-hooks.py` 退出 0，`violations=0`。
- `git diff --check` 退出 0；5 份新文件另经 Python 逐行核尾部空白，通过。
- Python 核 M2-NOTES 的 HEAD 前缀 256960 字节逐字保留，SHA-256 `b6b3603ca114b2c5b3c018b1cb3e27b24b8ee228680fe5246e0b978a8752b86b`。M8A、M7D、ci.yml、Cargo.toml、特性差集脚本、禁串生成器及 closed.rs 与 HEAD 逐字相同。去掉本片库 test-only 桥/模块声明、bin test-only 模块挂载后，两份既有生产源码与 HEAD 逐字相同，暂存区为空。

上述源码逐字对拍证明的是普通生产构建零行为变化。测试特性下明确增加 SQL adapter、metadata 查询和许可事件；没有把测试特性的观测差异当成生产不变证明，也没有接入任何生产读取调用点或更改 finish/回收。没有改动 J/R、角色收费、错误文本、effects、认证/资格、deadline、W2 规则串和历史规模。

本机未执行 cargo/rustc、Rust 编译或测试，也未执行全量重放、推送、git add 或 commit。**Rust 未验证，待 CI。** 16 个锁的逻辑、字节与事件断言尚未形成执行收据，原 C/D、E/F 和完整验收未转绿。

本片交付不销除 C/D g23、E/F g26 原失败，不完成 #1419 整票验收，不开启第 4 片。后续回收前八项须逐项有字节与生命周期证据，尤其全史成功表和冷折叠不得继续用局部栈形状代替。

非作者独立进程已读回最终封包并给出 PASS，范围限本片测试后端、真实生产核差分和静态审查，无剩余 BLOCK。报告 `/private/tmp/issue1419-m8b-independent-review.md`，SHA-256 `b2b56e2ac069c8b657b0c09e0dec24f7f44f8347679fec8643726e1ae61596e0`，附七份最终 Rust 的逐文件 SHA-256，主控读回确认一致。审中 effects 差分覆盖不足及共享事务夹持缺口均已修复；独审实际重跑格式、空白、禁串检查通过。此 PASS 不替代 CI 编译、warnings、16 锁执行、原完整验收或八项有界证明。本段仅追加评审收据，评审后的 Rust 源码未变。
