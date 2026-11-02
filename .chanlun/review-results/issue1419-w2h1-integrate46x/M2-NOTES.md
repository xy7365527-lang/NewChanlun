# #1419 M2：W2 × H1 类型接口闭合

本文件是绑定 #1419 的阶段工作草稿。开工 HEAD 为 `b70b5268a0`，上一提交 `89fea32012` 已合并 W2 `ab2bfac7` 与 H1 `2dff0d59`，HEAD 已并入 H1 文档尾修 `42a3f959`。开工工作区干净。已完整阅读指定 M1 笔记的三组语义缺口及逐文件取舍表；本阶段沿用已有票与用户授权，不开新票。

本阶段状态为 **ready**。已完成可静态确认的 typed 接口修复；本轮按集成 owner 46X 的裁定由正本生成器重写禁串清单，最终 1756 项，`--check` 实际退出 0。Rust 未验证，待 CI；本机未运行 cargo/rustc。

## owner 裁定与交付范围

#1419 集成 owner 46X 裁定：M2 原提示“删除只允许是该串已不在任何测试专属源码中出现”写错了，与 `.github/prod-banned-literals.txt` 头部“自动生成，勿手改”的现行合同冲突。正本是 `.github/scripts/gen-prod-banned-literals.py`（#1640 §2.3），CI 用 `--check` 强制精确一致。生成器按完整字面量抽取，并排除含花括号的格式串，这是已批准的设计，本阶段不改。

生成器从 M1 并集 1759 项删除 6 项、增加 3 项，最终 1756 项。被删的两个短串按 owner 裁定分别沿用以下现状：

- `rust/src/bin/s_session_v3/audit_owner.rs:5608` 的 `②索引槽宽连带降幅` 是更长测试字面 `②索引槽宽连带降幅（每槽窄16B）` 的前缀；长串仍在清单内，产物中该短串来自该长串，覆盖不丢。
- `rust/src/bin/s_session_v3/object_records.rs:2929` 的 `生成态装配不得读取载荷子图（记录自身字段根除外）` 在 W2 `ab2bfac7` 已是格式串的静态前缀；W2 原清单本就不含此短串，W2 链已按此状态通过 `--check`。本轮沿用生成器结论，与 W2 侧现状一致。

前轮按错误删除限制恢复两项后的 1758 项及 `--check` 退出 1 保留为历史记录，不是本轮交付状态。本轮重写及检查实绩见后文禁串记录；W2 冷往返证明对象表保留，交棒下一阶段 M3。

本阶段未修改完整 W2 写者串、冻结库规则、资源帽、判据或准入分支，也未增删测试与锁。保留生产写者规则：

```text
s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1+raw-initialization-w/2+bounded-layout/2
```

未增加 `test_writer_rule` 注入；未增加严格失败后切换宽松判据的路径。冷来源与在线来源继续实现同一个 `SourceInput` 并调用同一来源核；错误类型调整不改变 #799/#804 的判定路径。

## 下一阶段 M3 的 W2 冷往返证明

下表是 M1 缺口 2、3 向下一阶段 M3 的具体交棒。本阶段没有补对象恢复设计或冷往返测试。缺少专项证明不等于已经证明实现产错；H1 通用来源导出也不能代替 W2 新对象的逐对象证明。所有建议测试均须使用上述完整生产 W2 串，不使用测试写者注入或改冻结原件规则。

| 对象 | 当前文件:行 | 建议测试名与所需对拍 |
|---|---|---|
| C/D 段记录、segment version 与 second cursor | `rust/src/bin/s_session_v3/generation.rs:92` 的 `StructurePriors`，`:873` 至 `:879` 的 prior 与 cursor 恢复；`rust/src/bin/s_session_v3/content_structure.rs:545` 的 `build_cd` | `w2_h1_cold_cd_second_cursor_continuation_matches_online`：同一真实原件读集在冷点认证、恢复并续写，与连续在线路径比较段根、版本、second cursor、请求与完整历史 canonical；包含末段修订。 |
| E/F center seed、center、roles、assignment、position、pair 及各自版本根 | `rust/src/bin/s_session_v3/content_structure.rs:1274` 的 `EfPrior`，`:1740`/`:1751` 的 `build_ef`/`build_ef_units`，`:1814` 的版本根核验，`:1917` 的 seed 发出，`:2624` 的 pair 遍历，`:2761` 的 pair 对象，`:2840` 的 `center_pair_development` | `w2_h1_cold_ef_seed_pair_qualification_and_canonical_matches_online`：分别在 seed 成立、center 延伸/移动、pair 建立/修订处取冷点，核认证资格与 exact prior 根、角色、分配、位置、开发请求、修订连续性及四根/历史 canonical。 |
| raw-initialization-w/2 方向未定 prior 与待定请求 | `rust/src/bin/s_session_v3/generation.rs:98` 的 `w_undetermined`，`:1078` 至 `:1079` 的 prior 消费；`rust/src/bin/s_session_v3/generation.rs:2797` 的 `read_w_prior`，`rust/src/bin/s_session_v2/content_producer.rs:646` 的 `PriorUndetermined` 与 `:1505` 的 `build_initialization` | `w2_h1_cold_w_undetermined_prior_revision_matches_online`：方向未定期冷恢复后续写至方向成立，核 `pending_since_cut`、raw 初始化对象、W 待定组、来源绑定、请求撤回及版本根逐位相同。 |
| bounded-layout/2 的 shape window、分型历史与 CC-007.development_batch | `rust/src/bin/s_session_v3/generation.rs:392`/`:394` 的 `development_batches`/`development_ranges`，`:908` 的三组窗口分类，`:931` 的 `build_parser_shape`，`:2672` 的开发批构造；`rust/src/bin/s_session_v2/content_producer.rs:749` 的 `fractal_histories`，`:1897` 的 `seed_cold_delta`，`:2737`/`:2746`/`:2848` 的 shape 对象、窗口和身份 | `w2_h1_cold_bounded2_shape_development_window_matches_online`：在分型出生、后续发展、修订移窗/失效处取冷点，比较窗口身份、三组来源、历史序列、开发批 entries 窗口序、载荷根与完整输出。不能只比较 raw/bi 缓存。 |

共同恢复入口为 `rust/src/bin/s_session_v3/generation.rs:484` 的 `seed_cold`，当前显式恢复 raw/effective 序列及 producer/ContentBi 原账缓存；对应 `rust/src/bin/s_session_v2/content_producer.rs:1897` 与 `rust/src/bin/s_session_v2/content_bi.rs:1216`。下一阶段须证明这些接线对上表对象足够；如需从已发布原件恢复额外状态，另行实施，不能以当前格式检查替代语义验收。



# #1419 M2 typed 接口闭合组

已完整阅读 `/Volumes/AgentStorage/w2-h1-integrate46x/M1-MERGE-NOTES.md`，包括三组 H1 × W2 缺口及各文件取舍表。只做类型传播与调用适配；未新增/删除函数、判据、资源帽、锁或测试；未改生产规则、冻结夹具或 test_writer_rule；未运行 cargo/rustc、未 git add/commit、未写 Git metadata。

## 本组改动文件（相对工位）

- rust/src/bin/s_session_v2/content_manifest.rs
- rust/src/bin/s_session_v3/audit_ledger.rs
- rust/src/bin/s_session_v3/audit_owner/cold_source.rs
- rust/src/bin/s_session_v3/audit_owner/effects.rs
- rust/src/bin/s_session_v3/construction_set.rs
- rust/src/bin/s_session_v3/construction_state.rs
- rust/src/bin/s_session_v3/source_facts.rs
- rust/src/bin/s_session_v3/source_receipt.rs
- rust/src/bin/s_session_v3/state.rs
- rust/src/bin/s_session_v3/audit_owner.rs

## 类型传播与终界

统一使用 `newchan_rust::s_content::ContextError`（现有别名 PublishError）。没有实现 From<ContextError> for String。

1. `audit_owner/cold_source.rs:21/77/107/113/116/123`：ColdSource::new、CodecRead::parse_rendered_plain、SourceInput 的 sequence_delta/request_delta/array_member/request_member 都返回 PublishError，与 HashReader、SourceInput trait、LazySet typed 原核一致。`state.rs:1371` 唯一 ColdSource 构造位与 `audit_source_step` 动态调用链原已 typed，直接传播。
2. `construction_set.rs:223/238/343/387/739`：CommittedSet::rebind_cold_owner/cold、OwnerSourceCheck::counts_for_cut、PendingSet::bind_source_context、LazySet::cold 取消内部 typed→String 包装；检查、分支与状态变更原样保留。LazySet::cold 唯一 ColdSource 调用不再丢失原件/资源失败来源。counts_for_cut 被 state.rs:1169/3928 及本文件现有负控测试消费，均 typed；bind_source_context 唯一 effects.rs:1306 typed 调用。
3. `audit_owner/effects.rs:1269`：QualifiedSourceCut::consume 返回 PublishError；Declaration::read 不再 into_message；NativeMetadata 与 journal 错误采用现有 publish_err。唯一 `source_receipt.rs:206` 消费者现已 typed。
4. `source_receipt.rs:43-599`：audit_raw_receipt/audit_source_partitions_receipt/audit_receipt/export_raw_receipt/receipt_from_native/receipt_from_cold_step、RawReceipt::export_business/export_history_business/business_rows/with_store/check_request/check_manifest/check_raw_partition/bind_four_roots、PreparedReceipt::consume 全部 typed；with_store 的回调也 typed。所有原检查和资源操作保留。
5. `source_facts.rs:40-350`：push_cover/push_removed/coordinates_digest/observation_sources/object_record/export/export_history/export_parts 全部 typed。共享严格来源核、在线/冷业务导出、cover/removed/observation 来源导出不经文本化。
6. `content_manifest.rs:1486/1511/1564`：ManifestSourceReceipt::take_verified_units/consume、assemble_with_source 属 prepared/commit 内部 typed 链，改为 PublishError；with_store 回调不再 e.to_string，workspace/无workspace CodecError→PublishError 使用 From，保留 BridgeProof。NativeMetadata/NativeRoots/batch_id 的同型转换亦保留 typed。没有因为旧签名是 String 就误当外边界。
7. `audit_ledger.rs:135/142`：QueuedFacts::from_receipt/from_receipt_view 两个生产消费者在 prepared.rs:1668/state.rs:1429，故改 typed，business_rows 不文本化。已有台账 String API 的 stage_partial 内 `source.check_raw_partition` 与 `source.business_rows` 三个调用显式 ContextError::into_message；这是既有旁挂台账外部 API 终界，未重写台账全套错误接口。
8. `construction_state.rs:295/347`：既有 ColdRawState::into_owner / CommittedRawState::cold_seed 的 String 冷状态 API 在调用新 typed CommittedSet 方法时显式 into_message；入口其余已有 typed 核的转换保持原样。

## 全 rust 调用静态穷举

实际对每个改签名函数名执行 `git grep -n -w <name> -- rust/`（无 cfg 过滤）；原始结果 `/tmp/issue1419-m2-typed-callers.txt`，前后签名清单 `/tmp/issue1419-m2-typed-signatures.json`。本组未改参数个数；with_store 回调错误类型与返回类型一同改 typed。构造/trait 同名命中另外按具体接收者筛核。

- ColdSource::new 唯一 state.rs 冷步；parse_rendered_plain 的 trait/StoreView/HashReader/NativeSourceAudit/ColdSource 实现全仓逐项比对；SourceInput 在线 SourceMaterial 与冷 ColdSource 四方法一致。
- receipt 的两 audit 入口：prepared.rs 动态选择器 + 本文件测试 + prepared.rs 的真实 prepare 测试；动态选择器两臂签名完全一致。
- audit_receipt 仅两入口包装；export_raw_receipt 被 audit_receipt/native/cold 三处调用；receipt_from_native 仅 prepared；receipt_from_cold_step 仅 state。
- export_business 在 prepared 两分支；export_history_business 仅 state；business_rows 在 QueuedFacts 与 stage_partial 两路径；check_raw_partition 在 stage_partial 前后两次；check_manifest 在 ManifestSourceReceipt::take_verified_units、assemble_with_source、本文件；check_request 在 ManifestSourceReceipt::consume 与本文件。
- with_store 仅 assemble_with_source；bind_four_roots 在本文件测试与 prepared 真实测试；PreparedReceipt::consume 在同两组测试。测试内 unwrap/err 操作仍适配 typed；两个既有 VerifyDeadline contains 断言仅增加 to_string 显示适配，不改期望。
- push_cover 在 source_audit 与 source_contributions；push_removed 在 source_contributions；observation_sources 在两共享来源核；coordinates_digest 在 source_facts 内部与原有资源测试；export/export_history 在 receipt；object_record/export_parts 在 source_facts 内部。所有消费者本来返回 PublishError，故直接 ?。
- ManifestSourceReceipt::{take_verified_units,consume}/assemble_with_source 的所有调用分别 prepared.rs:712、commit.rs:271、prepared.rs:1718，均 typed。QueuedFacts 两入口在 prepared/state typed 链。
- LazySet::cold 仅 ColdSource；CommittedSet::cold/rebind_cold_owner 仅 construction_state 的既有 String API；counts_for_cut 两 state 生产位 + 本文件负控；bind_source_context 仅 QualifiedSourceCut::consume。
- source_receipt 模块整体由 s_incremental 开关；另核 `#[cfg(test)]`、`s_incremental`、`s_capture_native_testonly` 下测试路径，未删改 cfg 或缩窄可达域。全仓 grep 不跳过禁用分支。

## 实际验证

- 八文件 `rustfmt --edition 2021 --config skip_children=true --check`：退出 0。
- 八文件 `git diff --check -- <八文件>`：退出 0。
- Python 比对 HEAD/工作树函数名清单：八文件函数名集合相等；改签名函数逐名生成 git grep 结果并核查。
- 两处原有测试的 contains 显示适配之外，没有新增/删除测试，也未修改测试期望。
- Rust 未验证，待 CI。未运行 cargo/rustc，不把格式检查称作编译或执行验证。

## 另组只读复核

只读检查 signature_audit 改的 `history.rs`、`s_fact_cache.rs`。history 的 EffectiveBlock/EffectiveSequenceFacts 原核和所有分支保持，typed 资源/PrefixArray::branch 错误继续传播；生产 apply/array_root 均在 PublishError 函数内。s_fact_cache::DeltaLane::cold_seed 唯一 generation::seed_cold 调用位在 PublishError 闭包中，原有真实 String 冷 API 最终 into_message。

独立复核发现 `history.rs:10046` 的 `effective_sequence_tests::budget_case` 原 `Ok::<_, String>(facts)` 与新的 facts.apply? 返回不一致，已反馈作者改为 PublishError；主控末次已确认修复并通过格式检查。复核者未自行编辑另组文件。

## 最后追加：冷目录参数及完整 typed 冷帧

signature_audit 的参数穷举又发现 `state.rs` 的冷认证接线沿 H1 旧参数，而 W2 已扩接口。本组经主控授权新增第九文件：

- `verify_unrun_catalog_admitted` 第三个参数实际是 live_axes；冷 captured 路径现在逐字沿已有正常冷路径，`match indexed.cuts.last() { Some(cut) => catalog_evidence::rule_axes(&cut.metadata.rule_revision)?, None => &[] }`，将 live_axes 放第三参，原 None/ work 是第四/五参。没有硬编码13轴，更没有新判据。
- `catalog_evidence::verify_current_for_audit` 第四参补 `&last.metadata.rule_revision`，与同cut生成态根一致。
- `HistoricalCertificate::cold_captured` 内 `NativeSourceAudit::new_with_records_measured`、field、catalog 均 typed。为避免逐句中途文本化，将该内部完整帧直接返回 PublishError；precheck/capture/frame reuse/HashReader/native scope 原 codec 错误统一 publish_err，保留来源。唯一两调用 `cold` / `cold_for_request_inner` 在既有 String 冷认证出口显式 into_message。
- `history::fold_certified_history` 与其 read_indexed_history_inner 本来就是既有 String 全历史API；当前调用不构成类型不一致，未私自改另一组历史入口。
- 已全 rust git grep cold_captured、verify_unrun_catalog_admitted、verify_current_for_audit；这三处参数与其余所有调用逐项核过。

末次九文件 rustfmt --check、git diff --check 均退出0。九文件函数顺序和 #[test] 计数与 HEAD 不变。Rust 未验证，待 CI。

## 末轮追加：冷 policy 迁移列 typed 漏口

第十文件 `audit_owner.rs`：`CompleteCandidate::prepare_cold_policy_migration` 改返回 PublishError，`TextValue::from_str_admitted` typed 结果直接传播；bytes 原 CodecError 的 `map_err(invalid)` 改 `map_err(publish_err)`，保留同一 S3DriverInvalid 文本前缀和 P57 来源。两处 push_cell 与 refresh_cold_sql_charge 为既有 String API，后者显式 From<String> 回到 typed 返回。全 rust git grep 确认唯一消费者在 certify_cold 的既有 String codec_work 闭包内，在该出口显式 PublishError::into_message。原 prepolicy 分派、列名、LEGACY_POLICY_RECORDS_UNKNOWN、保留费与预算门不变。没有增删测试。

该文件 rustfmt --edition 2021 --config skip_children=true --check、git diff --check 均退出0；函数顺序和 #[test] 数量与 HEAD 不变。已通知 signature_audit 独立只读复核。Rust 未验证，待 CI。


## M2 全仓静态签名与调用者穷举（signature_audit）

比较共同基线 `9e6eb6f46c`、W2 `ab2bfac7`、H1 `2dff0d59`、封包 `b70b5268a0`。完整读取 M1 280 行，含三组取舍表及三份“H1 × W2 语义缺口”。本组仅改三文件：

- `rust/src/bin/s_session_v3/history.rs`：H1 新增 `EffectiveBlock::{allocate,leaf,branch}` 与 `EffectiveSequenceFacts::{array_root,apply,emit_edit,emit_block}` 改为 W2 `PublishError`；Native 预算/分配经 `publish_err` 保留 `CodecError` 的来源。原 `branch` 在 String 函数内 `?` 调 typed `PrefixArray::branch` 是确定的接口断裂；`apply/array_root` 的生产消费者全部在同一个 typed `audit_one_published_cut`。现有 `effective_sequence_tests::budget_case` 的 `Ok::<_,String>(facts)` 改为 `Ok::<_,PublishError>(facts)`，断言与测试数不变。
- `rust/src/bin/s_session_v2/s_fact_cache.rs`：`DeltaLane::cold_seed` 改 `ContextError`；原 String 直接 `?` 调 typed `RawPrefixIndex::advance_measured` 是确定断裂。新冷阶段 Native reserve/push、budget precheck 的 `CodecError` 用 `ContextError::from`，不再 `to_string` 擦除来源。唯一生产调用是 `generation.rs::GenerationRuntime::seed_cold` 的 typed 闭包；其已有冷 String API 最后显式 `into_message`。
- `rust/src/bin/s_session_v3/source_audit_development_tests.rs`：现有 `audit_generated_in_store` 调用补 `#[cfg(feature="s_incremental")] None`，对应 H1 exports 新参数。原调用默认构建 12 参数正确，incremental 构建要求 13 参数却只有 12；未增加/删除测试或改断言。

另发现并交 typed_closure 修复：`state.rs::cold_captured` 的 `verify_unrun_catalog_admitted` 漏 live_axes、`verify_current_for_audit` 漏 rule，及 `NativeSourceAudit::new_with_records_measured`/`field`/catalog typed 错误直接进入 String 函数。修后以实际末 cut 的 rule 及 `rule_axes` 原函数供值，不改任何判据。

机械范围与局限：两侧相对共同基线改动 155 个 Rust 文件；按“文件+函数名”比较签名多重集合，保守筛出 597 组、500 个不同名字（含同名新增实现造成的数量变化）；去掉仅数量变化后是 579 组签名集合变化、487 个名字。另记录两侧新名字 1597 组，用全仓 AST 调用清单补查。对保守 500 个名字逐名实际运行 `git grep -n -w -F NAME -- rust/`，不按 cfg 过滤；再穷举本轮冷函数与 H1 新接线名，完整命中数见下表。名字命中包括定义、注释、同名异物，不能把命中数说成真实调用数。

用临时 Python/tree-sitter 脚本遍历所有 670 个已跟踪 Rust 文件，记录函数定义、参数、调用、可选参数属性，并分诊同名调用参数及 String 函数内直接 `?` 消费 typed 核。Tree-sitter 不是 Rust 编译器：`object_index.rs` 的表达式属性、`prepared.rs` 的带 cfg 字段初始化各有文法 ERROR；这些位置以原文及 rustfmt 核对，未据此宣称语法/类型整体通过。全仓 String 原始 `?` 候选先筛 953 条；本次合并相关项逐一核查：独立冻结 oracle 的自有 String 方法、既有 String 外部 API、内嵌明确 typed 闭包、通用库同名函数均不能据名字误报。`audit_owner.rs::prepare_cold_policy_migration` 的 `TextValue::from_str_admitted` 实际返回 PublishError（另有返回 CodecError 的 `_typed`），最终直接回查定义后确认已有冷 String 出口缺显式 into_message，已交 typed_closure 补齐。预算测试的显式 String 闭包另行改为 typed，防止只检查生产返回类型而漏测试。

参数/cfg 逐组核对：

| 入口 | 全部调用与取舍 |
|---|---|
| `read_indexed_history_inner` | history 6 调用，均 15 参数，同时保留 sqlite_base 与 certify |
| `read_verified_public_history` | history 两测试 + state 生产，均 12 参数 |
| `audit_one_published_cut` | history 冷读、测试、append 三调用，均 25 参数 |
| `audit_one_cut_sources` | state 正常与 HistoricalSourceFold 调用；默认 10 / incremental 14 参数，certified 与 cold_counts 均保留 |
| `verify_unrun_catalog_admitted` / `verify_current_for_audit` | 各全部调用核查，修 cold_captured 漏项后分别 5 / 4 参数 |
| `publish_borrowed_staged` | prepared 测试增量调用 11 参数，source_manifest 同次传递 |
| `publish_borrowed_with_workspace_admitted` | commit wrapper + prepared 生产两调用，incremental 9 参数 |
| `publish_prepared_inner` | commit 三调用，默认 7 / incremental 9 / test+incremental 11；所有可选参数属性与 cfg 分支一致 |
| `generate_node_products` / `finish_node_products` | prepared 各两调用，前者保留 token 和双 lifetime；后者保留 source_receipt；默认/增量/测试可选参数逐项核对 |
| `audit_generated_in_store` | 全仓当前核与 frozen_oracle 同名分开；当前核默认 12 / incremental 13 参数；development_tests 漏 exports 已补，冻结 oracle 独立 11 参数不改 |
| `audit_object_record_with_slots` | ordinary_source、source_audit 两处、source_contributions 共 4 调用，均 14 参数，rule/current_generation/development 不丢 |
| `plan_native_union` | prepared 2 + object_index 测试 6，均 6 参数，末参 rule |
| `audit_native_bound` / `audit_source_step` | effects 1 调用 / source_contributions+state 冷链各 1 调用，均 6 参数；export_sources 与 SourceRaw 不互代 |
| `insert_group` | ObjectSlots trait + Full/ordinary ObjectWrites/Native ObjectKeys 实现均 id,index,group，调用均 4 显式参数，typed 返回一致 |
| `CodecRead::parse_rendered_plain` | trait、StoreView、HashReader、ArrayAuditSource、ColdSource 5 定义/实现全为同一 ContextError；所有消费者沿 typed 核或真实 String 出口显式转换 |
| `SourceInput` 四方法 | SourceMaterial、ColdSource 同 trait typed 合同，grep 全 rust 覆盖未过滤 cfg |
| `CutSourceVerifier::verify` | HistoricalSourceFold 与 ColdHistoricalSourceFold 两实现均 PublishError |
| `open_for_serve_diagnostic_fixture_testonly` | writer_cli 与 driver 测试两调用，均 7 参数；正式服务入口仍自有完整 retention/production 参数 |
| `seed_cold_delta` / `rebind_cold_delta` / `cold_seed` | producer/ContentBi 两类 seed 参数一致；generation 的种子与重绑消费相应类型；DeltaLane cold_seed 唯一调用已 typed |

`cfg(test)`、`s_incremental`、`s_capture_native_testonly`、`s_allocation_json_testonly`、`unix` 没有在 grep 阶段被过滤。Cargo 的 `s_capture_native_testonly=["s_incremental"]`，`mod.rs` 分支选择对应 capture 模块；逐参对照不把无法到达的 cfg 组合误当编译错误。可选参数的精确 test/incremental 四组合辅助脚本有意保守，必须人工排除父级 cfg 不可达、闭包名、标准库同名方法。原始 AST/grep 仅分诊，Rust 未验证，待 CI。

已只读复核 typed_closure 的 cold_source/source_receipt/source_facts/audit_ledger/effects/construction_set/construction_state/content_manifest 的 diff：内部 typed 贯通，`ManifestSourceReceipt`/`QueuedFacts` 不在中途擦除来源；ledger 持有的已有 String 外部 API 处显式 into_message。未发现另一个可证接口问题。

本组三文件 `rustfmt --edition 2021 --config skip_children=true --check` 与 `git diff --check -- <三文件>` 均退出 0。未运行 cargo/rustc、未写 Git metadata、未增删判据/帽/锁/测试。完整临时证据为 `/tmp/issue1419-m2-signatures.json`、`/tmp/issue1419-m2-all-signature-grep.json`、`/tmp/issue1419-m2-ast-report.json`、`/tmp/issue1419-m2-cfg-report.json`；下表把逐名 grep 覆盖直接纳入交付笔记，交棒不依赖这些临时文件。

最终只读复核 `audit_owner.rs`：`prepare_cold_policy_migration` 已改 PublishError，TextValue 原核直接传播，Native bytes 用 PublishError::from 保留资源来源；唯一调用在 certify_cold 既有 String 外部闭包显式 into_message。未发现新的可证接口断裂。

### 逐名 git grep 覆盖附录

下列命中数是本轮枚举时源码快照的计数；因协作适配仍可能移动行号或增加同名类型转换引用，它不是末态实时计数或 Rust 调用数。

| 函数/方法名 | 全 rust 命中数 |
|---|---:|
| `accept_prepared_frame` | 27 |
| `admit_append` | 2 |
| `advance` | 550 |
| `advance_admitted` | 12 |
| `advance_bound_checked` | 3 |
| `advance_bound_inner` | 2 |
| `advance_inner` | 6 |
| `advance_measured` | 14 |
| `advance_raw` | 2 |
| `advance_raw_bound_inner` | 3 |
| `allocate` | 26 |
| `append` | 722 |
| `append_owner_cut` | 14 |
| `apply` | 244 |
| `apply_order` | 54 |
| `array` | 358 |
| `array_elements` | 4 |
| `array_field_ref` | 5 |
| `array_len` | 10 |
| `array_member` | 7 |
| `array_reference_len` | 4 |
| `array_root` | 15 |
| `at` | 564 |
| `at_cut` | 20 |
| `at_cut_metered` | 7 |
| `at_cut_projected` | 5 |
| `audit` | 914 |
| `audit_admission` | 29 |
| `audit_core_fresh_inner` | 4 |
| `audit_core_fresh_with_policy` | 4 |
| `audit_generated` | 20 |
| `audit_generated_for_audit` | 9 |
| `audit_generated_frozen_for_test` | 5 |
| `audit_generated_in_store` | 11 |
| `audit_generated_metered` | 6 |
| `audit_generated_until` | 1 |
| `audit_native_bound` | 4 |
| `audit_native_source` | 6 |
| `audit_native_source_cut` | 2 |
| `audit_object_record` | 5 |
| `audit_object_record_with_slots` | 6 |
| `audit_one_cut_sources` | 3 |
| `audit_one_published_cut` | 8 |
| `audit_persisted_observation` | 3 |
| `audit_persisted_relation` | 3 |
| `audit_persisted_witness` | 3 |
| `audit_shape_record` | 6 |
| `audit_source_step` | 4 |
| `audit_witness_record` | 4 |
| `audit_witness_record_with_slots` | 4 |
| `axes_entries` | 6 |
| `before` | 2093 |
| `before_fixed_write` | 8 |
| `before_fixed_write_inner` | 2 |
| `bind` | 141 |
| `bind_json_text` | 4 |
| `bind_originals` | 5 |
| `bind_shape_leaf` | 4 |
| `bind_shape_refs` | 1 |
| `bind_shape_refs_view` | 3 |
| `branch` | 224 |
| `branch_node` | 7 |
| `bridges` | 30 |
| `build` | 323 |
| `build_group_object` | 3 |
| `build_incremental` | 5 |
| `build_knowledge_state` | 3 |
| `build_template` | 5 |
| `build_typed` | 10 |
| `build_with_units` | 4 |
| `built` | 312 |
| `built_ref` | 31 |
| `bytes` | 2537 |
| `candidate` | 855 |
| `candidate_fields` | 4 |
| `candidate_fixed_fields` | 2 |
| `candidate_verified_legacy_fields` | 4 |
| `canonical_admitted` | 4 |
| `canonical_array_admitted` | 2 |
| `canonical_matches` | 10 |
| `catalog_axes_admitted` | 5 |
| `catalog_axes_oracle` | 5 |
| `catalog_projection_entry` | 3 |
| `center` | 1898 |
| `change_array` | 9 |
| `changes_admitted` | 5 |
| `changes_at_admitted` | 7 |
| `check` | 286 |
| `check_appended` | 2 |
| `check_appended_work` | 3 |
| `check_batch` | 3 |
| `check_commit` | 7 |
| `check_deadline` | 137 |
| `check_identity` | 13 |
| `check_native_scope` | 3 |
| `check_owner_raw` | 2 |
| `check_public_ranks` | 4 |
| `check_resolution_header` | 3 |
| `check_scope` | 5 |
| `check_store` | 3 |
| `checked_input` | 8 |
| `clone_in_admitted` | 3 |
| `close_against_persisted` | 4 |
| `close_set` | 2 |
| `close_stages` | 3 |
| `codec_error` | 58 |
| `cold_seed` | 5 |
| `collect_gen` | 7 |
| `compare_cut_array` | 3 |
| `comparisons` | 136 |
| `concat_core` | 7 |
| `coords` | 337 |
| `copy_coords` | 13 |
| `copy_source_coords` | 3 |
| `counts_for` | 4 |
| `coverage_at` | 8 |
| `current_publication_frontier` | 2 |
| `current_publication_frontier_admitted` | 5 |
| `cv` | 126 |
| `decimal` | 62 |
| `decode_manifest` | 7 |
| `decode_persisted_entry` | 3 |
| `decoded_scalar` | 5 |
| `dedupe_observations` | 2 |
| `delta` | 3147 |
| `delta_bound_input` | 9 |
| `depend_coord` | 4 |
| `deref` | 49 |
| `development_batches` | 13 |
| `diff_rank` | 4 |
| `difference` | 62 |
| `difference_coords` | 4 |
| `digest` | 3257 |
| `dirty_from` | 74 |
| `drop` | 1552 |
| `edge` | 334 |
| `effective_life` | 7 |
| `elements` | 480 |
| `elements_ref` | 6 |
| `emit_block` | 4 |
| `emit_edit` | 4 |
| `emit_state` | 4 |
| `empty` | 566 |
| `empty_node` | 5 |
| `entry` | 1919 |
| `equal` | 105 |
| `equal_is_evidence` | 34 |
| `err` | 2895 |
| `event_bucket` | 5 |
| `event_node` | 8 |
| `exact` | 1696 |
| `extend_gen` | 19 |
| `extend_set` | 4 |
| `fact_generation` | 35 |
| `field` | 1206 |
| `field_meta` | 9 |
| `field_ref` | 46 |
| `field_text` | 18 |
| `field_text_admitted` | 6 |
| `fields` | 1137 |
| `finish` | 494 |
| `finish_axes` | 20 |
| `finish_fixed_write_in_publication` | 2 |
| `finish_fixed_write_inner` | 3 |
| `finish_node_products` | 3 |
| `fixture` | 2083 |
| `fmt` | 409 |
| `fmt_text` | 6 |
| `forget` | 11 |
| `from` | 1713 |
| `from_bytes` | 14 |
| `from_ids_admitted` | 3 |
| `from_sql_admitted` | 55 |
| `from_str_admitted` | 13 |
| `from_view` | 41 |
| `full` | 670 |
| `full_input` | 7 |
| `gap_at` | 3 |
| `generate` | 44 |
| `generate_bound` | 12 |
| `generate_impl` | 7 |
| `generate_native_lease` | 2 |
| `generate_node_products` | 4 |
| `generate_with_allocation` | 3 |
| `generated_axes` | 2 |
| `generated_axes_ref` | 4 |
| `generation_admitted` | 3 |
| `get` | 3869 |
| `get_admitted` | 97 |
| `group_changed` | 3 |
| `group_payload` | 7 |
| `group_payload_value` | 2 |
| `grow` | 39 |
| `hash` | 598 |
| `hash_array_metered_with_work` | 7 |
| `hash_json_in_store` | 41 |
| `hash_json_with_owner` | 2 |
| `hash_view` | 7 |
| `heap_bytes` | 261 |
| `history_decode_in_store` | 4 |
| `hold_parent_charge` | 2 |
| `hold_resource_budget` | 2 |
| `immutable` | 90 |
| `insert` | 1153 |
| `insert_admitted` | 111 |
| `insert_bound` | 8 |
| `insert_group` | 7 |
| `insert_id` | 7 |
| `insert_rank` | 5 |
| `insert_request` | 10 |
| `insert_slot` | 5 |
| `integer` | 316 |
| `integer_meta` | 4 |
| `integer_ref` | 2 |
| `into_arc_admitted` | 5 |
| `into_parts` | 30 |
| `invalidate` | 102 |
| `invalidate_witnesses` | 3 |
| `json_from_view_admitted` | 16 |
| `json_text` | 77 |
| `key` | 6276 |
| `knowledge_at` | 6 |
| `knowledge_record` | 3 |
| `known_value` | 3 |
| `leaf` | 359 |
| `legacy_json_owner_admitted` | 6 |
| `len` | 8575 |
| `link` | 110 |
| `link_ref` | 10 |
| `load_in_store` | 5 |
| `local_fields` | 9 |
| `mask` | 63 |
| `match_field_view` | 5 |
| `matches` | 1102 |
| `member` | 93 |
| `member_change` | 2 |
| `meta` | 627 |
| `meta_admitted` | 7 |
| `meta_i64_in_store` | 4 |
| `meta_optional_admitted` | 3 |
| `meta_text_in_store` | 6 |
| `mutate` | 65 |
| `native` | 1602 |
| `native_cut_binding` | 3 |
| `native_cut_projection` | 5 |
| `native_permit_inventory` | 9 |
| `native_rank` | 9 |
| `native_ref` | 7 |
| `native_sweep_admitted` | 4 |
| `native_text` | 90 |
| `new` | 11151 |
| `new_admitted` | 38 |
| `new_node` | 3 |
| `new_with_records` | 4 |
| `new_with_records_admitted` | 2 |
| `new_with_records_measured` | 6 |
| `node` | 952 |
| `note_qualified_source_cut` | 3 |
| `note_source_cut` | 16 |
| `nulls` | 12 |
| `number` | 240 |
| `object` | 1380 |
| `object_births` | 2 |
| `object_phase` | 2 |
| `observation` | 318 |
| `observation_births` | 7 |
| `observation_id` | 58 |
| `observation_identity_value` | 6 |
| `observation_phase` | 2 |
| `observation_wire` | 3 |
| `open_for_serve_diagnostic_fixture_testonly` | 3 |
| `open_for_serve_with_identity` | 5 |
| `optional_sql_admitted` | 7 |
| `ordered` | 138 |
| `original_sources` | 26 |
| `owned` | 714 |
| `owned_admitted` | 5 |
| `parse` | 976 |
| `parse_rendered_plain` | 16 |
| `parsed_json` | 16 |
| `partition_phase` | 2 |
| `path_of` | 2 |
| `payload` | 1680 |
| `pending_phase` | 2 |
| `plan` | 507 |
| `plan_for_layout` | 4 |
| `plan_from_candidates` | 7 |
| `precheck` | 102 |
| `prepare` | 476 |
| `prepare_node` | 37 |
| `prior_records` | 8 |
| `producer_error` | 20 |
| `production_venue_from_records` | 10 |
| `production_venue_where` | 4 |
| `project_bar` | 2 |
| `projection_field` | 6 |
| `public_rank` | 7 |
| `public_sets` | 14 |
| `publication_degrade_verdict` | 6 |
| `publication_material` | 4 |
| `publication_put` | 2 |
| `publish` | 385 |
| `publish_body` | 5 |
| `publish_borrowed_staged` | 2 |
| `publish_borrowed_with_workspace_admitted` | 3 |
| `publish_impl` | 5 |
| `publish_impl_admitted` | 3 |
| `publish_impl_admitted_bound` | 3 |
| `publish_impl_bound` | 2 |
| `publish_in_owner` | 3 |
| `publish_prepared_inner` | 4 |
| `push` | 3930 |
| `push_change` | 4 |
| `push_gen` | 34 |
| `push_native` | 9 |
| `push_native_array` | 3 |
| `push_raw_withdrawal` | 3 |
| `push_replace` | 5 |
| `push_withdrawal` | 4 |
| `put` | 495 |
| `qualify` | 5 |
| `rank` | 211 |
| `raw` | 4169 |
| `raw_advance` | 2 |
| `raw_bar` | 43 |
| `raw_relation` | 2 |
| `raw_root` | 8 |
| `read` | 895 |
| `read_exact` | 4 |
| `read_fields` | 14 |
| `read_header` | 4 |
| `read_indexed_history_inner` | 8 |
| `read_local` | 3 |
| `read_metadata` | 2 |
| `read_publication_fields` | 6 |
| `read_seq_node` | 29 |
| `read_subset` | 40 |
| `read_text` | 29 |
| `read_verified_public_history` | 4 |
| `reader` | 1299 |
| `rebind_cold_delta` | 4 |
| `reclaim` | 11 |
| `record` | 1917 |
| `record_error` | 10 |
| `ref_value` | 10 |
| `reference` | 1092 |
| `reference_json` | 2 |
| `reference_text` | 4 |
| `references` | 55 |
| `relation` | 154 |
| `remember_wire` | 5 |
| `remove` | 378 |
| `remove_object` | 6 |
| `remove_rank` | 5 |
| `remove_request` | 4 |
| `remove_shape` | 3 |
| `remove_step` | 2 |
| `remove_witness` | 6 |
| `render` | 63 |
| `render_json` | 22 |
| `render_text` | 2 |
| `replace_batch_field` | 3 |
| `replace_reserved` | 8 |
| `replacement_relation` | 3 |
| `request_delta` | 10 |
| `request_member` | 9 |
| `request_phase` | 2 |
| `request_reasons` | 20 |
| `request_record` | 2 |
| `request_record_with_slots` | 5 |
| `require_ohlc` | 3 |
| `reserve` | 217 |
| `reserve_admitted` | 15 |
| `reserve_group` | 6 |
| `resolve` | 167 |
| `resolve_current_births` | 10 |
| `resolve_current_births_inner` | 4 |
| `resolve_current_births_staged` | 2 |
| `resolve_current_births_with_workspace` | 1 |
| `resolve_current_births_with_workspace_admitted` | 4 |
| `resolve_in_publication` | 1 |
| `resolve_in_publication_admitted` | 3 |
| `resolve_in_publication_inner` | 3 |
| `resolve_in_publication_staged` | 2 |
| `resolve_indexed_templates` | 7 |
| `resolve_indexed_templates_pending` | 4 |
| `resolve_indexed_templates_pending_core` | 4 |
| `resolve_indexed_templates_pending_view` | 13 |
| `resolve_native_dual_templates_pending_view` | 8 |
| `resolve_native_indexed_templates_pending_view` | 9 |
| `resolve_objects` | 4 |
| `resolve_observations` | 3 |
| `resolve_record_batch` | 2 |
| `restore` | 276 |
| `restore_for_owner` | 6 |
| `resume_owner` | 2 |
| `retain` | 149 |
| `revision_invalidates_affected_coords_and_capacity_eviction_is_output_neutral` | 2 |
| `role_coordinates` | 5 |
| `root` | 2704 |
| `root_at` | 7 |
| `root_header_bytes` | 4 |
| `root_ref_bytes` | 24 |
| `run` | 1598 |
| `same_coord_with_changed_content_is_recomputed_and_old_version_kept` | 2 |
| `same_digest` | 8 |
| `same_json` | 8 |
| `same_ledger` | 66 |
| `scalar` | 123 |
| `scalar_ref` | 2 |
| `scalar_value` | 23 |
| `schedule` | 97 |
| `seed_cold_delta` | 4 |
| `seek_admitted` | 8 |
| `sequence_delta` | 6 |
| `set_seed` | 15 |
| `shape_observation` | 4 |
| `shape_phase` | 2 |
| `shape_witnesses` | 17 |
| `share` | 64 |
| `singleton` | 142 |
| `slot` | 1328 |
| `small_field` | 3 |
| `sorted_numeric_entries` | 8 |
| `source_allocation` | 24 |
| `source_check` | 19 |
| `source_coords` | 374 |
| `source_references` | 7 |
| `splice_reserved` | 3 |
| `split` | 286 |
| `split_core` | 7 |
| `sql_text_in_store` | 12 |
| `stable_sort` | 7 |
| `stable_sort_admitted` | 4 |
| `stable_sort_typed_admitted` | 2 |
| `stage_current` | 10 |
| `stage_in_material` | 10 |
| `start` | 1553 |
| `state_members` | 6 |
| `step_record` | 5 |
| `step_sequence_record` | 2 |
| `step_sequence_record_view` | 3 |
| `stream_hash` | 6 |
| `strict_json_admitted` | 4 |
| `strict_plain_json_admitted` | 6 |
| `strings` | 28 |
| `supersedes_relation` | 9 |
| `sync` | 285 |
| `sync_splice` | 9 |
| `tb1405_pending_fixture` | 6 |
| `templates` | 13 |
| `text` | 1774 |
| `text_meta` | 23 |
| `text_ref` | 28 |
| `text_value` | 21 |
| `trace_event` | 59 |
| `try_clone_admitted` | 104 |
| `typed_kind` | 4 |
| `union_coords` | 10 |
| `units_are_computed_once_per_exact_content_key_and_reused_afterwards` | 2 |
| `unlink` | 23 |
| `unsettled_seed_group` | 3 |
| `update_from_owner` | 4 |
| `validate_effects` | 4 |
| `validate_unrun_row` | 3 |
| `validate_withdrawal_target` | 3 |
| `value` | 4318 |
| `vec_push` | 8 |
| `venue_header` | 3 |
| `verdict` | 176 |
| `verify` | 353 |
| `verify_current` | 10 |
| `verify_current_begin` | 2 |
| `verify_current_begin_admitted` | 6 |
| `verify_current_begin_view` | 2 |
| `verify_current_for_audit` | 4 |
| `verify_current_frontiers` | 5 |
| `verify_current_in_store` | 5 |
| `verify_cut_for_audit` | 2 |
| `verify_cut_in_store` | 3 |
| `verify_generated_members_in_view` | 4 |
| `verify_generated_members_swept_with_records` | 3 |
| `verify_generated_members_with_changes` | 3 |
| `verify_new_closure` | 6 |
| `verify_public` | 22 |
| `verify_refview` | 49 |
| `verify_required_meta_admitted` | 5 |
| `verify_source` | 3 |
| `verify_unrun_catalog_admitted` | 4 |
| `verify_witness_id` | 3 |
| `window` | 632 |
| `winner_intervals` | 3 |
| `winner_intervals_admitted` | 5 |
| `wire` | 654 |
| `wire_at` | 13 |
| `wire_root` | 19 |
| `with_bound_raw_advance_in_publication` | 3 |
| `with_construction_material` | 2 |
| `with_construction_material_in_publication` | 2 |
| `with_material_access` | 20 |
| `with_material_in_publication` | 3 |
| `witness` | 463 |
| `witness_identity_native` | 8 |
| `witness_phase` | 2 |
| `witness_wire` | 3 |
| `write_fields` | 4 |
| `write_json` | 31 |
| `write_json_view` | 51 |
| `writer_for_generation` | 2 |


# #1419 M2 禁串清单静态核对

## 结论与交付

前轮禁串子任务只改 `.github/prod-banned-literals.txt`；本轮收口仅重写该清单并更新本笔记，未改生成器、checker、Rust 源码、测试、规则或 Git metadata。初始 M1 并集 1759 项；本轮按原生成器重写后最终 **1756 项 = 1759 - 6 + 3**，`--check` 实际退出 0。清单沿用生成器的 Python sorted 顺序、无重复，头部不变。

本子任务状态 **ready**。集成 owner 46X 已裁定原提示的源码字节串删除门错误，清单应服从“自动生成，勿手改”的现行合同；正本生成器为 #1640 §2.3 的 `.github/scripts/gen-prod-banned-literals.py`，CI 用 `--check` 强制一致。完整字面量抽取及排除含花括号格式串均为已批准设计，本阶段不改。前轮恢复两项形成 1758 项并检查失败的过程只作历史记录，不再构成阻断，也不要求下一阶段先做生成器新设计。

## 生成器输入与 cfg 语义

完整读取 `.github/scripts/gen-prod-banned-literals.py`、其依赖 `.github/scripts/check-prod-no-test-hooks.py`、产物消费者 `check-prod-artifact-hooks.sh` 及现有 Python 单测。生成器从 lib.rs 与 s_structure_session.rs 沿 mod 图遍历；管辖面取 checker ROOTS，生产排除面覆盖整个 crate 图。两生产集合为 `{s_session}` 与 `{s_session,s_incremental}`，均 test=false（生成器 :32-35）；测试集合另含 s_capture_native_testonly、s_allocation_json_testonly 并枚举 test=true（:36-45）。未知 cfg 保守真假两可；生产任一配置可达的字面及其子串都排除。生成器按完整字面量抽取；含控制符、花括号、非 distinctive 或与生产字面重合/被包含的串不入清单（:291-307）。

消费者按字节 `grep -aF` 匹配；清单成员仍以生成器的完整字面量及排除规则为准。owner 46X 裁定：短/长串保留关系与 W2 格式串现状分别按下表处理，不再以原提示的字节串删除门覆盖生成器合同。

## 对照基线

- W2：`ab2bfac7` 的原清单和源码。
- H1 文档尾修：`42a3f959` 的原清单和源码。
- 开工源码 HEAD：`b70b5268a0`。
- 开工清单逐字备份 `/tmp/issue1419-m2-banned-before.txt`，两侧清单只读快照 `/tmp/issue1419-m2-banned-w2.txt`、`/tmp/issue1419-m2-banned-h1.txt`。

## 原生成器删除 6 项：逐项来源与 owner 裁定

| 原串 | 原侧证据 | 当前源码与删除判定 |
|---|---|---|
| `16M 下冷根不应失败` | 仅 H1 清单；42a3f959:driver.rs:4558 | 全 rust 源码不再出现原串。当前 driver.rs:4884 为 `31M 下冷根不应失败`，位于 :946 的 cfg(test) tests 模块；W2 新夹具预算文案，不在本轮修改预算/测试。当前 31M 串已在生成清单。允许删除。 |
| `ingest-3 须与全程 16M 对照组同路` | 仅 H1 清单；42a3f959:driver.rs:4576 | 全 rust 源码不再出现原串。driver.rs:4902 同一 cfg(test) 模块变为 `ingest-3 须与全程 31M 对照组同路`；替代串已在清单。允许删除。 |
| `②索引槽宽连带降幅` | 仅 H1 清单；42a3f959:audit_owner.rs:5580 | **仍出现**于 audit_owner.rs:5608 完整字面 `②索引槽宽连带降幅（每槽窄16B）`，属 :2164 cfg(test) tests 模块。长串源自 ab2bfac7:audit_owner.rs:4472，长串当前也在清单。owner 46X 裁定：短串是长串前缀，产物中该短串来自该长串，覆盖不丢；本轮按生成器删除短串。 |
| `未签生产 profile 不得打开 Driver` | 仅 W2 清单；ab2bfac7:driver.rs:883 | 全 rust 源码不再出现原串。当前 driver.rs:1164 为 `未签 synthetic profile 不得打开 Driver`，仍在 :946 cfg(test) tests 模块，实际构造 SYNTHETIC_UNSIGNED_V1；新文案已在清单。允许删除。 |
| `生成态装配不得读取载荷子图（记录自身字段根除外）` | 仅 H1 清单；42a3f959:object_records.rs:2889 | **仍出现**于 object_records.rs:2929 的完整格式串 `生成态装配不得读取载荷子图（记录自身字段根除外）：{leaked:?}`，属 :1688 cfg(test) tests 模块。完整格式串本已在 ab2bfac7:object_records.rs:2929。生成器 :304 排除带花括号字面。owner 46X 裁定：W2 原清单本就不含该短串，W2 链已按此状态通过 `--check`；本轮按生成器删除短串，沿用 W2 侧现状，不改已批准的格式串排除设计。 |
| `目录必须含 13 轴之外的行` | 仅 H1 清单；42a3f959:state.rs:2618 | 全 rust 源码不再出现原串。当前 state.rs:2663 为 `目录必须含规则轴表之外的行`，位于 :2627 cfg(test) tests 模块，实际规则轴来自 catalog_evidence::rule_axes；新文案已在清单。允许删除。 |

表内省略前缀文件均位于 `rust/src/bin/s_session_v3/`。已用 git grep 两侧源和当前全部 rust/ 源码搜索；另用生成器 lex 对全 rust/**/*.rs 核对：6 个旧串均已无**完全相等的字面量**，其中上述 2 个仍有源码子串命中；owner 46X 已撤回原提示的字节串删除限制，本轮按完整字面量生成器合同收口。

## 新增 3 项：逐项来源与生产不可达证明

| 新串 | 当前文件:行与 cfg | 两侧来源/为何新增 |
|---|---|---|
| `SchemaUnsupported：s-clock-system/1 只用于 s-session/3` | rust/src/bin/s_session_v2/mod.rs:2801，所属 init_v2 在 :2785 受 `#[cfg(any(test, feature = "s_v2_seed_testonly"))]` 门控。两个生产特征集都没有 seed feature 且 test=false，故不可达；test=true 可达。 | H1 42a3f959:同文件:2761 原有拒绝文案；H1 的 init_v2 尚无 W2 测试种子门，因此 H1 生产字面排除；W2 侧没有此新文案。集成门与文案组合后成为测试专属，新增合法。 |
| `只用于 s-session/3` | rust/src/bin/s_session_v2/mod.rs:4308，属 :3539 cfg(test) tests 模块中的 v2_init_rejects_system_clock_without_writing；另在 :2801 上行门控完整文案中出现。 | H1 42a3f959:同文件:4217 原有断言，过去与 H1 生产完整字面子串重合，被生产排除；集成后完整拒绝文案也受 seed/test 门，两生产配置均不可达，因此新增合法。 |
| `W2_PREVIOUS_SIDE` | rust/src/bin/s_session_v3/financial_session_tests.rs:2461，模块由 financial_session.rs:1889 cfg(test) + :1890 path 引入；更外层 financial_session 在 mod.rs:71 受 cfg(unix)。两个生产集 test=false 均不满足必需 test 条件；test=true 的 unix 分支可达。 | W2 ab2bfac7:financial_session_tests.rs:2461 已有同名环境变量，H1 源码无此项；两侧清单均未收录。不能仅凭旧清单未收录推断生产可达；当前原生成器及源码门明确要求补入。当前生成器产生本项，没有修改环境读取/门。 |

所有新增完整字面均用生成器 lex 在全 rust/**/*.rs 枚举：各 1 个完全相等字面出现；源码门如上，未发现其他生产同名字面。`W2_PREVIOUS_SIDE` 为全大写仓内专用环境名，满足 distinctive；其余两项 UTF-8 字节长度与中文条件满足。

## 前轮实际检查（历史）

1. `python3 .github/scripts/gen-prod-banned-literals.py`：成功，`gen-prod-banned-literals count=1756`。
2. 此时 `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check`：成功，清单与源码一致（1756 项时）。
3. `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/check-prod-no-test-hooks.py`：成功，`violations=0`。
4. `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s .github/scripts -p 'test_check_prod_no_test_hooks*.py'`：20 项通过。
5. 前轮按原提示删除限制恢复上述 2 项后为 1758 项；当轮 `--check` 实际退出码 **1**，输出 `::error::.github/prod-banned-literals.txt 与源码不一致，须重跑 gen-prod-banned-literals.py`。当轮集合 count=1758、unique=1758、排序通过，相对初始删除 4 项、新增 3 项。此状态已由本轮 owner 裁定及生成器重写替代。
6. `git diff --check -- .github/prod-banned-literals.txt`：通过。
7. 未运行 cargo/rustc，未运行会触发 cargo 的 test-feature-delta.sh，未运行产物检查（二进制不可用）。Rust 未验证，待 CI。

## 本轮 owner 裁定后实际检查

1. `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py`：实际退出 0，输出 `gen-prod-banned-literals count=1756`，重写最终清单。
2. `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check`：实际退出 0，最终清单与源码一致。
3. 本轮仅改清单与本笔记，未改 Rust 文件、生成器、checker 或测试；未运行 cargo/rustc，未 git add/commit。Rust 未验证，待 CI。

## 下一阶段 M3 交棒

W2 冷往返证明按本文前部对象表交棒 M3，表内对象、建议测试及所需对拍保持不变。本轮已按现行生成器合同收口，不把格式串静态前缀或保留输入的新设计作为下一阶段前置条件。

## 主控末次验收

- 全部 13 个 Rust 改动文件执行 `rustfmt --edition 2021 --config skip_children=true --check`，退出 0。仅证明格式/解析可接受，不证明类型检查或运行行为通过。
- 对全部 13 文件用 Python 比较 HEAD 与最终文本的函数名顺序和 `#[test]` 计数，逐文件相同。原有断言仅做 typed 错误显示与闭包错误类型适配；development 测试仅补 feature 对应参数。
- `git diff --check` 退出 0，`git diff --cached --name-only` 为空；未暂存、未提交、未写 Git metadata。
- 生产规则入口、冻结测试原件、结构/生成判据、ContextError 定义及两个禁串脚本均无改动；未新增测试写者注入或失败后换宽判据。
- 独立只读复核发现的 `history.rs:10046` 测试闭包现已为 `Ok::<_, PublishError>(facts)`，该遗漏已修。末轮冷迁移的 Native bytes 错误采用既有 `publish_err`，同时保留原 `S3DriverInvalid` 前缀和 typed 来源。
- 本轮按 owner 46X 裁定实际运行正本生成器重写，最终清单 1756 项，随后 `--check` 实际退出 0。前轮恢复两项后的 1758 项及检查退出 1 仅为历史记录；逐项删除/新增证据和裁定见上文。
- 现有 Python 源码钩子检查 `violations=0`，现有 checker/生成器单测 20 项通过；未新增测试。
- 静态枚举不能代替编译，未运行 cargo/rustc，也未运行会调用 cargo 的 feature-delta 或产物构建脚本。Rust 未验证，待 CI。
- 本阶段已完成允许范围内的接口修复及禁串清单收口，交付 `ready`。W2 冷往返证明保留为下一阶段 M3 交棒，按本文对象表实施；Rust 未验证，待 CI。

交付文件共 15 份：

- `.github/prod-banned-literals.txt`
- `rust/src/bin/s_session_v2/content_manifest.rs`
- `rust/src/bin/s_session_v2/s_fact_cache.rs`
- `rust/src/bin/s_session_v3/audit_ledger.rs`
- `rust/src/bin/s_session_v3/audit_owner.rs`
- `rust/src/bin/s_session_v3/audit_owner/cold_source.rs`
- `rust/src/bin/s_session_v3/audit_owner/effects.rs`
- `rust/src/bin/s_session_v3/construction_set.rs`
- `rust/src/bin/s_session_v3/construction_state.rs`
- `rust/src/bin/s_session_v3/history.rs`
- `rust/src/bin/s_session_v3/source_audit_development_tests.rs`
- `rust/src/bin/s_session_v3/source_facts.rs`
- `rust/src/bin/s_session_v3/source_receipt.rs`
- `rust/src/bin/s_session_v3/state.rs`
- `.chanlun/review-results/issue1419-w2h1-integrate46x/M2-NOTES.md`

## M2C

本节绑定 #1419，按 PR #1855 首轮 CI 修复。开工 HEAD `d4002d145667c4ddbd98fa451245a3a58f8551ca`，分支 `sandcastle/issue-1419`，工作区干净。先读 `/Volumes/AgentStorage/w2-h1-integrate46x/ci-d4002d14/rc.txt`、全部失败 job 的错误诊断及 `prodsess-fail.txt`，并核 M1/M2 取舍记录。本轮仅修改工作区，不暂存、不提交、不运行 cargo/rustc。

### 重复定义与 stats 接口

| 对象 | W2 `ab2bfac7` / H1 `42a3f959` 逐行比对 | 本轮取舍 |
|---|---|---|
| `s_content.rs` 的 `verified_public_subroots_tests` | W2 原 `:9443`、H1 原 `:9378` 的完整模块均为 135 行，与集成原 `:9540`、`:9677` 两份逐字相同；均含 2 个测试、6 个 assert。 | 保留 `rust/src/s_content.rs:9540` 首份，删除完全相同的第二份；夹具和断言没有损失。 |
| `ordinary_material_readonly_owner_and_some_gates_match_uncached_source` | 两侧各 95 行，差异只有 payload 的 `repeat(20)` / `repeat(200)`，以及末两处 `pool.stats()` / `pool.stats`。其余夹具、Meter、fail_at、结果/事件轨迹及 SQL 读数断言逐字相同。 | `ordinary_materials_tests.rs:245` 合成一个测试，`:284` 对 `[20, 200]` 分别建立完整独立夹具、执行全部原有 5 个断言；访问统一为现有 `stats()` 方法。没有只选一侧 payload。 |
| `ordinary_material_pool_actual_heap_released_after_readonly_scope` | 增量预检发现原 `:343`、`:462` 也重复，CI JSON 同样报 E0428/E0609/E0615。两侧差异仅 `stats().slab_capacity` / `slab.capacity()` 与 stats 方法/字段写法。 | 保留当前 `ordinary_materials_tests.rs:345` 的一份，3 个断言全保留。`ordinary_materials.rs:48` 的 `stats()` 转发 `raw.stats()`；`s_content/raw_materials.rs:127` 将真实 `self.slab.capacity()` 填入 `stats.slab_capacity`，两侧检查等义。 |

Python 实际逐字比较了完整模块；独立实装组另将合并测试按 20/200 展开、归一 stats 访问后，逐字对回两侧原测试，并对 heap 测试归一访问后逐字对回两侧。三个符号在全 `rust/` 的 `git grep` 中各只剩一个定义，无残留 `pool.stats.` / `pool.slab`。没有修改签名。

### LegacyWrite 与写入口同门

根因是 H1 新拒绝入口仍在生产构建，W2 已把它保护的旧写入口门控为 `any(test, feature = "s_v2_seed_testonly")`，导致 `-D warnings` 下方法/变体不可达。按同一个门覆盖：

- `rust/src/bin/s_session_v3/admission.rs:313` 的 `WriterScope::admit_legacy_write`；
- `admission_private/mod.rs:379` 的 `RejectionPrefix::LegacyWrite`、`:457` 的增量拒绝及 `:511` 的 match arm；增量拒绝原有 `s_incremental` 前提保留；
- `admission_private/mixed.rs:20` 的 match arm，从原合并模式拆成两个同义 arm，只给 LegacyWrite 加门。

没有删除拒绝逻辑，没有新增 `allow(dead_code)`。方法直接调用者只有 `s_session_v2/mod.rs:3442` 的 `dispatch_with_policy` 和 `:3505` 的 `legacy_write_lock`；前者原本就与拒绝路径同门。

后续增量编译另有已记录的 E0425：`audit_owner/legacy_admission_tests.rs:121` 调用的 `legacy_write_lock` 原来仅 seed feature 可见。现将 `s_session_v2/mod.rs:3458`、`:3472`、`:3491` 的 Guard、impl、函数三个门统一为 `any(all(test, feature = "s_incremental", unix), feature = "s_v2_seed_testonly")`。这精确覆盖种子入口和 H1 测试两个真实调用域，不在无增量的普通 test 构建引入无调用项。拒绝路径本身仍按要求使用较完整的 `any(test, seed)` 门，覆盖所有旧写入口。

H1 测试的 include 门仍为 `test+s_incremental+unix`，所有既有 assert 逐字不变。`legacy_admission_tests.rs:122` 增加 `Result::inspect`，让测试与根种子分派一样消费成功 guard 的 `require_admitted/enter` API，避免只编入返回类型而留下无调用方法。`inspect` 不改变 Result：Err 原样进入原来的 VersionMismatch 和提示断言；任何 Ok 要么在 guard 检查处失败，要么仍在原 `.err().expect(...)` 处失败，不能把错误放行变成通过。根 `s_structure_session.rs:6214` 的 seed 分派不变。

全 `rust/` 穷举 `LegacyWrite`、`admit_legacy_write`、`legacy_write_lock`、Guard 及方法调用；Python 穷举 test/seed/incremental/unix 的 16 组真值，所有活调用均有定义，所有 guard 构建都有拒绝路径。独立只读复核发现并促成本轮采用精确测试门，末次复核没有剩余具体问题。上述仅是静态核对，H1 测试实际编译和执行待 CI。

### 第 4 项 E0308 的诊断订正：保留刻意编译负例

`rust/src/bin/s_session_v2/content_store.rs:3285` 不是漏改的正常测试调用，而是 `#[cfg(all(test, issue1394_capture_callback_refusal))]` 下的刻意编译负例 `capture_callback_must_not_compile`。因此本轮没有把它改成可编译引用，也没有改变函数签名或取消类型。

实际执行 `git log -S CaptureCancellation` 追到 `23c3188ecef3dc9e9d5cb0deeb299cb9fb1098b9`，提交 `test(#1394): close capture cancellation callback capability`，2026-10-02。它在同一提交中把任意 `FnMut` 改为 `&CaptureCancellation`，并新增这个旧闭包负例。W2/H1/当前三份的取消类型、完整捕获函数、完整负例均逐字相同，Python 比对通过。所要求的“第 n+1 次检查把 body 置零”正是这个负例试图注入、应被类型边界拒绝的能力；不能把拒绝这项能力误修成允许它。

直接证据是 `.github/scripts/check-compile-negatives-1394.sh:2-6` 与 `:20-46`：负例必须编译失败，全部错误必须是负例块内的 E0308；若改为可编译引用，该脚本会明确失败并报“负例编译通过，类型边界不再拒绝旧写法”。`job-125097986384.txt` 原文同样明报“出现 E0308 以外的错误，不能证明是负例本身被拒”，混入的是本轮已修的 E0428/E0615。CI 启用该 cfg 的唯一入口就是这个负例脚本，普通构建不启用它。修无关错误后，应由 CI 重新证明仅剩预期 E0308。

`git grep` 全 `rust/` 共 17 个捕获调用：content_store 内 11 个正常测试、1 个刻意闭包负例，外部 catalog_evidence/object_index/projection/state 共 5 个调用。16 个正常调用全部传 `&CaptureCancellation`；`never/at_check` 与其调用处同在 test 门内，`state.rs:555` 的非 test 调用与 `until` 都在 `s_allocation_json_testonly` 门内。`s_incremental` 不启用该负例 cfg。未运行会启动 cargo 的负例脚本。

### 行为失败：补齐夹具，不放宽错误断言

CI 原文只显示 `.contains("InvalidDomain")` 断言失败，未输出 `out.stderr`，因此不能声称已读取实际 stderr。静态路径明确指向缺锚：原测试 `s_init_v3(None)` 后没有调用 `anchor`，manifest 还指向不存在的 `space/limits.json`；资源文件则由 `launcher_manifest` 正常生成。`writer_cli.rs:269` 先执行 `admission_anchor::from_args_for_db`，其 `load_for_db` 在 `admission_anchor.rs:1324` 读取缺失的 `<db>.control-owner.json`，返回 `AnchorMissing/E1`，Display 文本为 `R4准入拒绝：AnchorMissing；阶段=E1…`。这发生在 `writer_cli.rs:845` / `Driver::check_structure_source` 的域核对之前，是静态归因，待 CI 取证确认。

两侧核对也解释了为何 H1 通过而集成新增尾部用例失败：W2 的原名是 `production_session_refuses_s_db_without_source_declaration`，夹具缺锚；H1 同名用例已改用完整 `structure_source_fixture("missing")` 和 `assert_launcher_source_failure`。M1 为保留两侧把 W2 体改名后追加到文件尾部，缺锚随之保留；`42a3f959` 中不存在集成尾部的新名字。H1 的已通过用例不能证明这份缺锚夹具正确。

`rust/tests/production_session_entry.rs:1083` 现在调用既有 `anchor(&s_db)` 并把返回路径传给 manifest，沿完整 W2 v3 初始化路径，不改生产规则、域检查顺序或准入语义。原 `InvalidDomain` 和 raw 行数前后相等断言保留；增加 stderr 失败诊断、无 socket 断言，改用已有的有界 `run_refusal`，清除该测试不需要的凭据/诊断环境键。任何门失效均不会无限挂起。补齐锚后应到达现有缺结构源的 InvalidDomain 路径；本机未运行 Rust，是否通过待 CI，未用另一个拒绝原因替换原断言。

### 后续 feature 组合的静态预检

实际 workflow 是 `.github/workflows/ci.yml`。rust-check 在 `s_session` 已停止，后两项为 `s_session,s_incremental` 和 `s_session,s_capture_native_testonly`，后者通过 Cargo feature 依赖包含增量。另核 allocation JSON testonly 和 seed 构建相关引用，没有把首轮 s_session 通过当作所有组合通过。

JSON 诊断 `job-125097986410.txt` 还给出了以下被普通错误文本遮住的确定接口缺口，已一并修复：

- `history.rs:3858` 的 `HashReader::sqlite_base` 漏初始化只在 incremental 下存在的 `request_verifier`，报 E0063。现与其他 ordinary 构造器一致，按 `#[cfg(feature = "s_incremental")]` 设 `None`。没有修改签名或引入宽松验证；无 captured owner 的 ordinary 构造器不携带请求捕获器。全 `rust/` 穷举 `HashReader::sqlite_base`，history/state/source_audit_development_tests/sqlite_base_tests 调用均无需改参。
- `prepared.rs:2254`、`:3042` 的 H1 增量测试仍传三参 `Fixture::build(5, 5, None)`，报 E0061；W2 已在第三参加入 `recover_events: &[(&str, i64)]`。两处现在传 `&[]`，保持原夹具不创建恢复事件的意图，原 `None` 规则参数保留，不使用测试写者规则放行。`git grep` 全 `rust/` 两处具体 `Fixture::build` 均已适配，定义与 `Self::build/with_clock_plan/build_at` 路径另逐一核对，签名不改。
- 增量 duplicate heap 测试、旧 `pool.slab/stats` 访问和旧写锁 E0425，已在前两节逐项结清。

本轮未改变任何函数签名。生产写者规则仍是完整 W2 串 `s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1+raw-initialization-w/2+bounded-layout/2`。没有改冻结库、资源帽、生成/结构判据、准入语义，没有新增 `test_writer_rule` 或严格失败后的兜底路径。M3 冷往返证明义务仍按本文已有对象表交棒。

### 禁串重建与实际检查

先运行生成器 `--check` 确认受 cfg 变更影响，再运行 `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py` 重建。清单由 1756 项变为 1757 项，仅新增一条：

```text
VersionMismatch：s-session/3 写入口必须显式使用 --protocol-revision s-session/3
```

来源是现在与 LegacyWrite 一起门控的只读拒绝文本；无删除项。随后 `--check` 实际退出 0、报告与源码一致，没有修改生成器或手改清单。

实际完成：

- 10 个 Rust 改动文件 `rustfmt --edition 2021 --check --config skip_children=true` 退出 0；额外未修改的 content_store 同样格式检查通过。初次 production 测试格式差异已按 rustfmt 修正，末次全部通过。
- `git diff --check` 退出 0；改动 Rust 文件冲突标记 0；`git diff --cached --name-only` 为空。未新增工作产物文件，未改其他代理的改动。
- Python 静态检查：重复项唯一、两侧模块/捕获类型/函数/负例逐字保留、16 组 cfg 真值、H1 原 assert 逐字保留、增量字段/参数完整及 production 负控域/raw 断言保留，全部通过。逐行比对和独立只读复核见各节。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/check-prod-no-test-hooks.py` 退出 0，`violations=0`。
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s .github/scripts -p 'test_check_prod_no_test_hooks*.py'`：20 项通过。
- 正本禁串生成器重建及 `--check` 均退出 0，最终 1757 项。
- Rust 未验证，待 CI。未运行 cargo/rustc、Rust 测试、编译负例门或产物构建；静态检查不证明实际编译/行为通过。本轮可交付 ready，供 CI 继续揭示后续错误。


## M3

本节绑定 #1419，开工 HEAD `045a478d2da16be6126f6ddf8bf807b7d3e9b1b5`，分支 `sandcastle/issue-1419`，开工工作区干净。已完整阅读本文件 853 行，包括 M3 对象表与 M2C。沿用户指定工位实施，不切基线、不暂存、不提交；本机不运行 cargo/rustc。新增测试属于现役准入入口的测试锁，本节仍是绑定 #1419 的工作草稿。

### 冷恢复静态分析

结论是现有冷恢复链具备这些对象的语义状态来源，本轮未发现确证遗漏，未修改生产恢复实现。缓存为空会多做计算，不能据此认定语义缺失。以下均为静态结论，实际类型检查与冷往返结果待 CI。

共同入口 `audit_owner.rs:666` 至 `:689` 用认证 candidate 的 raw 与末代 generated_state 冷种；`:844` 至 `:875` 在 owner 交接后带走同一 audited 索引。`generation.rs:484` 的 `seed_cold` 恢复 raw/effective sequence、DeltaLane、producer source 与 ContentBi 坐标缓存。其余对象的 prior 每代从已认证原件选取，不以进程 memo 为权威。

| 对象 | 已认证原件与续写消费链 | 冷后状态是否足够 |
|---|---|---|
| C/D 段、段版本与 second cursor | `prepared.rs:1005` 从 `audited.objects` 经 `object_index.rs:2550` 的 `structure_prior_records` 取活动 `CC-013.segment` 和 `CC-056.segment_version`；`content_structure.rs:404` 从版本原件 `scan_cursors` 恢复 SecondFoldStore，`generation.rs:873` 交同一 parser。`content_structure.rs:361` 核段 revision/result/version。 | 足够。cursor 的正本是段版本原件，fresh Runtime 不需接续旧进程 second store。 |
| E/F seed、center、roles、assignment、position、pair 与双版本 | 同一 prior 选择器取 E/F 全族及 center/pair version；`content_structure.rs:1785` 核 prior 并建 `(kind,entity)` 索引；`:1390` 的 revision/same_fields/keep_known/revise 辅助核延续已知时点与版本。 | 足够。已认证 payload 提供上一版语义，不由空 runtime 缓存另作判定。 |
| W 未定 prior | `object_index.rs:2604` 选活动 `CC-004.undetermined_raw`，`generation.rs:1078` 经 `read_w_prior`、`content_bi.rs:986` 核身份并取 `pending_since_cut`；`content_producer.rs:1602` 用同一 fact_key/input_refs 判定沿用 since 或本 cut。 | 足够。等待起点与来源版本均由活动原件恢复。 |
| bounded2 分型历史 | `content_producer.rs:2317` 缺历史 memo 时从 empty 重算，`:2390` 用同一核生成全部 bar/relations/source_ref 单元及前缀根；缓存键为 references/end，缓存不含 cut/generation/出生。`:2511` 的 bounded 分型身份绑定 shape 与基础来源。 | 足够。memo 仅省重算；重算仍绑定当前八字段 source_ref。 |
| bounded2 开发批与窗口序 | `generation.rs:908` 按 wins 窗口序分类，`:977` 同序收 developments；`:2689` 缺 prior range 时将当前全区标 dirty，`:2738` 按同一 developments 顺序重建当前全部 batch，BTreeMap 保持 bar 坐标序。 | 足够。entries 认当前窗口序；已消失 batch 由历史 object index 的差集撤回，不依赖 memo 留存。 |
| 对象出生、修订、撤回历史 | `content_producer.rs:1176` 投当代 gen/cut；`object_index.rs:1640` 的 `plan_from_candidates` 按同 ID 匹配旧 Entry、保留 birth_wire/life，仅向 changes.births 追加新对象。 | 足够。生命周期从已认证旧索引续写，不取重算 memo 的时点。 |

上表行号锚定本轮开工源码。ADR 0054 与 ADR 0037 P2 的当前口径也要求 entries 按当前窗口序，unit 不含代、cut 或生命周期。本轮没有增加第二份恢复判据、失败后全审计兜底或测试写者规则，没有修改 spec、资源帽、隔离或准入语义。


### 夹具、冷点与非空证明

新增文件由 `audit_owner.rs` 的 `construction_tests` 在 `test+s_incremental+unix` 下 include，与既有 H1 admission 锁同域。共享夹具自行建立临时目录、时钟、生产 `init_v3` 与正式 `Driver::open_with_request_and_mode_retention`；不借用内部带规则包装的旧 Fixture，不调用 `with_test_writer_rule`。初始化后逐字断言库规则为完整生产 W2 串。

| 测试名 | 输入与最终代 | 冷点 k | 对象存在与变化断言 |
|---|---|---|---|
| `w2_h1_cold_cd_second_cursor_continuation_matches_online` | `s_session/tests/fixtures/tb02d/raw-ledger.json` 的独立手算 `second_open_up` 40 根；raw37 极值 +1 的 revision 2、恢复 revision 3、末尾追加，N=43。 | 27、33、40、41、42 | 真正发出 SECOND feature_sequence、segment_version；第40代按最大 `construction.start_stroke` 选末活动段，第41代必须是该同 entity 的 revision 递增，随后取冷点。每代核生产 `scan_budget.policy`、`round_strokes=0`、`scan_cursors=[]`。非空 cursor 的冷往返锁为条件义务，见下节。 |
| `w2_h1_cold_ef_seed_pair_qualification_and_canonical_matches_online` | 逐字复用 `financial_session_tests.rs` 的 `q3_w_bars` 104 根 PIVOTS 输入配方；2120 顶小幅 +1 修订、复原、尾追加，N=107。 | seed 成立的56；首次实际 center members 或 resume_anchor 改变的代动态取点；pair 建立104、pair 修订105、恢复106。 | 第56代有 center 及 seed→center；首次实际成员/锚变化单独冷开，不把 settled 状态变化充作延伸。第104代至少2个 center，roles/assignment/position/center version/pair version 非空且 pair 真成立。第105代同 pair entity 的 revision 递增、previous 非空、inputs 实际改变。中心 data 不含价，不强行要求小幅改价一定改变 a/b_revision。 |
| `w2_h1_cold_w_undetermined_prior_revision_matches_online` | 复用 `pending_source_ordered_same_ohlc`，等待期 a 同价 revision 2 后 d 使方向成立，N=5；另复用 `pending_source_earlier_revision` 四行后追加 d，N=5。 | 第一场1、2、3、4；第二场1、3、4。 | 等待期真实未定原件与待定组请求非空；第一场第4代 a 新身份/since=cut4，b/c 仍沿用 cut2/cut3。方向成立后全部 W 未定原件撤回且 pending 组请求消失。第二场覆盖成立→修订再未定→再成立。 |
| `w2_h1_cold_bounded2_shape_development_window_matches_online` | 复用 `tb02b_oracle_rows` 的14根独立 oracle；沿 `bounded2_revision_restore_preserves_archived_cuts` 对第11根 +1 修订再复原；另将 b1 改为 H20/L1 包含左右后复原，N=18。 | 3、4、5、14、15、17。 | 第3代真实出生 `[0,10,20]` 分型；第4代右组包含使身份改变并有历史撤回；第5代起有真实开发批及增长；第15代源修订撤回旧 batch、产生新身份。第17代活窗口实际消失/移窗且历史分型撤回，第18代恢复原几何窗。逐代核三组来源、entries 严格窗口序、单元 source_ref、完整发展数组根与正计数。 |

共享 helper 先以同一个不中断 Driver 跑全部 N 代，每代保存完整 canonical 到夹具临时目录；每个冷点通过独立只读 SQLite 连接 `VACUUM main INTO` 保存已提交前缀。全部参照写完并 drop 在线 Driver 后才冷开各副本。冷副本不继承 owner、Runtime、生成器或台账，必须实际逐 cut 获得 H1 资格；从 k 冷状态一直比到 N，每次续写前后 `state::full_audit_calls_for_test()` 严格不增加，冷认证自身也有同样计数断言。

对拍保留全部业务表每格实际类型与完整值，NULL/整数/文本/Blob 原值、REAL 的位模式都进入 canonical；同时保留 owner 的完整 `h1_canonical`（所有 cut 的四根、snapshot、delta、请求与开发批以及完整对象/辅助事实历史），渲染全部对象出生 public wire 并附当前撤回状态。没有用哈希、计数或 raw/bi 缓存替代载荷。

唯一不作全表相等的是 `s_content_nodes` 的不可达编码 memo。现有 `prepared.rs:3762` 的 `runtime_root_tables(...,false)` 和 `:3809` 的可达原件对拍已采用该口径；生产 eager store 会留不可达编码缓存，其有无不是业务结果。本轮另循相同全部持久根及 tag/子边配方，逐代对拍可达节点的完整 digest/tag/body 与各自 SQLite typeof。其余业务表无过滤。未借此排除任何可达发布原件、历史版本或对象载荷。

### 条件义务：非空 second cursor 的 H1 冷往返锁

本轮交付状态为 **ready**。#1419 集成 owner 46X 裁定：非空 second cursor 在当前生产预算下不可达的判断正确，但不构成本集成的阻断。生产可达域内 cursor 只有空值，已落地的 C/D 锁覆盖空 checkpoint 连续性与段版本 revision 递增；四个具名锁供 CI 验证，不将其称作非空 second cursor 的冷恢复证明。

静态证据链：`driver.rs:503` 至 `:507` 的正式入口固定使用 `ThetaConfig::for_s_structure_writer`；`theta_v0/config.rs:804` 与 `run_records.rs:544` 走 active records，`config.rs:113` 至 `:120` 读取已签预算。`rust/src/theta_v0/policy_records/second-seq-scan-budget.json` 的 `second_seq_window.value=0`，该记录明确写“改值即新版本”；`parser/second_fold.rs:731` 至 `:739` 的 `checkpoint_cursors` 在预算0时恒为空。已有 `research_override_second_seq_window` 同时切换 ResearchOverride，不能当生产证据。

条件义务：任何把 `second_seq_window` 改为非零的新版本预算记录，必须同时提交非空 cursor 的 H1 冷往返锁。在该条件触发之前，此锁不构成本集成的未完成项。预算记录变更属于政策/资源记录变更，不在本票授权内；本轮没有修改预算，也没有为测试注入研究预算。其余对象静态恢复分析未发现遗漏，实际 Rust 编译、对象变化断言与逐代冷对拍仍待 CI。

### 独立复核与检查范围

实装与只读复核分离。复核修正了首段修订冒充末段、仅 settled 变化冒充延伸、小幅改价强行要求中心 revision 变化、冷副本路径/inode 导致额外台账身份不可比等测试问题；没有因此放宽生产判据或资源帽。最终 C/D 按实际末段 entity 核修订，E/F 按实际成员/锚变化选冷点，pair 核同实体 revision/previous/inputs。

另用 Python 的 SQLite 进行了 WAL 靶向检查：保持源写连接在线，通过独立 mode=ro 连接参数化执行 `VACUUM main INTO ?`，复制已提交行成功，源写连接 data_version 不变。这只验证复制工具语义，不是 Rust Driver 实证。Python 静态整数几何核对也确认 b1 包络修订使原 `[0,10,20]` 等窗口变为 `[0,60,70]` 等新窗口；这不是 Rust 测试执行。


本轮实际检查：

- 3 个 Rust 改动文件执行 `rustfmt --edition 2021 --config skip_children=true --check`，全部退出0。只证明格式/解析接受，不等于 Rust 类型检查或测试通过。
- Python 静态检查通过：四个 `#[test]` 定义各唯一、四名各一次进入 H1 owned 逐名清单、所有既有清单名字保留；`h1_` 过滤包含 `w2_h1_`，三层 test/feature/unix 门一致；新夹具无测试规则包装和旧 Fixture 调用。
- 正本禁串生成器实际重建后仍为1757项，文件无差异；随后 `--check` 退出0。未手改清单或生成器。当前生成器按现有 `mod` 图遍历，不额外扩展其对 `include!` 文件的收录口径。
- `check-prod-no-test-hooks.py` 退出0、`violations=0`；现有相关 Python 单测20项通过。
- SQLite WAL 在线源的只读 `VACUUM INTO` 靶向检查通过，原 Blob 值/typeof 一致且源 data_version 不变。
- `git diff --check` 退出0；`git diff --cached --name-only` 为空。未暂存、提交或回退任何改动，最终生产逻辑仅新增测试 include 门，无恢复实现变更。
- Rust 未验证，待 CI。未运行 cargo/rustc、Rust 测试、编译负例或会间接调用 cargo 的检查。阶段为 ready，实际 Rust 编译与冷对拍待 CI，不能把上述静态检查称作四条冷往返锁通过。

## M2D

本节绑定 #1419，按用户授权处理 PR #1855 的四项定点失败。开工 HEAD 为 `640bf49e93722048de160bc507bb82eb3e8bc04d`，分支 `sandcastle/issue-1419`，开工工作区干净。先完整阅读本文件 M1/M2/M2C/M3 交接，再读 `/Volumes/AgentStorage/w2-h1-integrate46x/ci-045a478d/job-*.txt` 与 `ci-pr1793/job-*.txt` 中的失败原文。下列运行读数均来自已有 CI；本机只作源码、历史和 Python 静态推导，不运行 cargo/rustc，也不运行间接调用它们的门脚本。不暂存、不提交、不回退任何改动。

### 1．完整 Some 审计冻结：逐对象归因后重冻

原文 `ci-045a478d/job-125104867274.txt:2523`：

```text
Some full: counts=Counts { objects: 91, groups: 13, windows: 11, effective: 14, insufficient: 0, outside_domain: 0 }, reads=232828, F=254341, read_hash=54375e9815ef8078d68d213e0a077f0b814ad7ce7a22432854441f1747eedfac, budget_hash=6dea88d382928c12c511d19a2acd7ce03c5e0b3ff4273408b1b66df1c92290a4
```

随后 `:2525` 至 `:2529` 报对象元组 `(91,13,11,14,0,0)` 不等于旧冻 `(81,13,11,14,0,0)`。对照 `ci-pr1793/job-123737814511.txt:2929` 至 `:2934` 同样为 91≠81；该侧 reads=202561、F=224074，read_hash=`6d5e5711000a85cdce417685edc9ca08caa0d48c8ff77064739cfe9cfb6e3801`，budget_hash=`488ea53514684092948a4a9cb06f8d81c86688fd61cce80faef66010eaf97008`。因此对象冻结过期继承自 W2，不是 H1 集成新增的十个对象。

实际执行 `git log ac24e9e3eb..HEAD -- rust/src/bin/s_session_v3/generation.rs rust/src/bin/s_session_v2/content_producer.rs rust/src/bin/s_session_v3/source_audit.rs rust/src/bin/s_session_v3/content_structure.rs` 并阅读相关提交：`0bace7c395`（#1780 / ADR 0037）新增 `CC-007.development_batch`；`6028c3e5ee` 将正式 W2 写者切入 bounded-layout/1；`4004c7cded` 升 bounded-layout/2，只为单元增加 `source_ref`，不另增对象种类或条数。现行 ADR 0037 的每后续 bar 一条批记录口径与 ADR 0054 的 entries 窗口序保持完整。

从 `s_structure_session.rs::tb02b_oracle_rows` 的真实 14 行整数 OHLC 静态复算包含组和严格双轴分型，得到 13 组、11 窗；五个分型窗口及右组末成员为 `[0,10,20]→30`、`[40,50,60]→60`、`[50,60,70]→70`、`[60,70,80]→80`、`[100,110,120]→120`。首个底分型右组包含 raw20/raw30，其后 bar 恰为 `40、50、60、70、80、90、100、110、120、130`。其余分型的后续 bar 都在此集合内，加入对应批记录的 entries，不另造同坐标对象。因此十个新增对象逐个对应这十个坐标的 `[CC-007.development_batch, coord]`。

唯一性证据：`generation.rs:2730` 的 BTreeSet 收集坐标，`:2763` 的 BTreeMap 每坐标只输出一条；`content_producer.rs:1799` 把坐标写入 fact_key；`source_audit.rs:2281` 在计数前拒绝重复 object_id，`:3140` 从对象数组长度计数。夹具 `source_audit.rs::fixture` 创建 fresh GenerationRuntime、空 StructurePriors 和单代 generation=1，无上代对象或撤回历史。故 +10 来自已批准批记录行为，没有发现重复计数或撤回遗漏，满足重冻前提。上述几何推导只作静态归因，不冒充 Rust 生成器实跑。

`source_audit.rs::full_some_audit_keeps_frozen_read_and_budget_trace` 保留全部旧注释，追加上述提交、坐标与唯一性归因。数值以旧值加两段已测净增量表达：

| 项 | ac24e9e3eb 旧冻 | ac24→PR1793 W2 链净变化 | PR1793→045a 集成净变化 | 新冻 |
|---|---:|---:|---:|---:|
| objects | 81 | +10 | 0 | 91 |
| reads | 151351+13016+120+37928=202415 | +146 | +30267 | 232828 |
| F | 170797+14276+120+38245=223438 | +636 | +30267 | 254341 |

两段 reads/F 净变化均**未逐节点分解**；不能将整段净差全归给十个新增对象。完整 read_hash/budget_hash 按上述 045a CI 原文重冻，不使用 PR1793 的摘要替代。

### 2．请求前缀稳定性：补齐六消息时钟

`ci-045a478d/job-125104867177.txt:2294` 至 `:2297` 报 `w_requests_published_prior_prefix_stability` 在 `prepared.rs:2155` 解包失败：`MissingDependency：clock事件 input-5`；PR1793 `job-123737814511.txt:2922` 至 `:2925` 是同一错误，属于 W2 继承夹具缺陷。

`prepared.rs::Fixture::new()` 仅建立默认四个 input 事件，而该测试依次驱动 1–6 六条消息，包括第 5 条历史插入和第 6 条修订。`projection.rs:1055` 改为既有 `Fixture::with_clock_events(4, 6)`：保留 retain=4，仅补全 input-1..6；全部消息、发布路径、请求排序/前缀/删除/插入/修订断言逐字保留。没有改生产时钟检查、断言或 Fixture 实现，也没有添加规则注入。

### 3．两种 payload：按 W2 固定槽保证保留暖读锁

`ci-045a478d/job-125104867274.txt:2514` 至 `:2519` 报 `ordinary_materials_tests.rs:332` 的“暖原件不重读 SQL”96≠95。PR1793 没有此项失败。CI 未打印 payload/fail_at；失败分支由下述静态路径推导，不能称 CI 已打印这些值。

M2C 循环按 `[20,200]` 顺序分别创建独立夹具；每组按 `[usize::MAX,1,4,9,25]` 顺序执行。W2 `bc896d5e0c` 将 RawMaterials 改为固定槽，`a8ba7383bc` 已将 W2 原测试 repeat(200) 改为 repeat(20) 适配；H1 `42a3f959` 的原池仍使用共享连续 body 区，1400B 可驻留。集成沿用 W2 固定槽，M2C 又保留了 H1 的 repeat(200)，因此无条件零 SQL 断言不再适用于那一个超槽块。

当前 `s_content/raw_materials.rs` 的 CAP=33554432、META_RESERVE=4096、ENTRY_BYTES=42、ENTRIES=65536、BUCKETS=131072，生产每槽 body 上限为 `(33554432-4096-65536*42-131072*4)/65536` 向下取整，即 **461B**。`"payload".repeat(20)` 为140B；repeat(200) 为1400B，未达4096B分片阈值。STRING 根 body 固定40B，真正拒存的是其唯一1400B `BYTES_CHUNK`。`RawMaterials::save` 按 body 长度拒存，后续 `read_material` 对该摘要再次调用 SQL reader；它仍正常通过同一 F 与业务审计，没有第二套判据。`sql_before` 位于 uncached expected 完成之后、actual 之前，采样没有跨循环错位。

因此第一组20可以暖驻留；第二组200的首个 fail_at=MAX 完整验证会多读该块一次，对应95→96。较早预算失败停在该块之前。独立 Python 编码、SHA256 和槽映射推导用于交叉核对节点/访问次序；这仍是静态证据。

仅修改测试夹具：测试内 WarmSource 对实际源读取按完整 digest 逐次检查。两组所有可驻留节点（包括200组）继续断言“暖原件不重读 SQL”，每次增量必须为0。例外只绑定 `frame(BYTES_CHUNK, payload1400)` 的完整摘要，并逐次核类型、1400B body 与**恰好1次 SQL**；拒存计数增量必须等于该块实际读取次数。外层总 SQL 也只扣除这个精确计数后与 sql_before 相等。保留两种 payload、全部五个 fail_at、完整 Some 结果/事件轨迹对拍、命中和容量检查。没有改缓存实现、槽宽或资源帽，也没有用宽松生产判据兜底。

### 4．native feature：两项同名锁保留增量等价版本

`ci-045a478d/job-125104867194.txt:945` 至 `:946`：

```text
s_capture_native_testonly removed 2 tests from bin
feature_delta target=bin feature=s_capture_native_testonly base=964 with_feature=1528 delta=566 removed=2
```

完整读取 `.github/scripts/test-feature-delta.sh`：它用 s_session 基底与 feature 构建的测试名差集，removed 非零立即失败；**没有允许名单或登记格式**。没有修改门脚本。`s_capture_native_testonly` 隐含 s_incremental。

静态沿模块、include 和测试 cfg 查得恰好两项，均在 `sqlite_base_tests.rs`：

- `sqlite_base_two_readers_overlay_clear_and_fresh_obey_domain`；
- `sqlite_base_real_audit_matches_frozen_baseline_and_first_errors`。

二者自 `ebaff48880` 起就带 `not(s_incremental)`，属于 W2 有意的非增量证明测试，不是本轮合并错门；文件存在于 W2 `ab2bfac7`，H1 `42a3f959` 无此文件。`history.rs:3884` 的 eligible_sqlite_base 和 `state.rs:842` 的 ordinary_materials_qualified 都明确排除 s_incremental，生产资格门须保留。

修法为去掉两项测试函数的外层非增量门，在函数内部保留原非增量全部断言，并提供增量语义下的对应断言：第一项同样跑双 reader、错误 overlay、clear、无关 overlay、fresh 与 NativeMeasured 隔离；增量读必须完整遍历，proof commits/hits 必须为0。第二项继续跑真实冻结库审计、持久表只读不变以及损坏/缺失首错对拍；增量构建要求 LAST_REPORT/LAST_PROOFS 始终为None，预先清空线程局部观测，避免继承前一测试残值。原25个 assert 全部保留，新增隔离断言。没有删测试、改生产排除门或处理 #1481 native 栈溢出。

同名测试保留满足 feature-delta 不得丢名的合同；该脚本仅执行新增名字，因此原有名字的增量版本实际运行仍须由 CI 的相应测试执行确认，本机不作通过声明。

### 独立复核、禁串与实际检查

实装与只读复核分离。对象组独审 SQLite 两锁，确认全部原25个 assert 保留，新增增量隔离断言符合现有资格门；feature组独审暖缓存计量及冻结摘要。暖缓存三源口逐一直接委托底层同名 read/checked_read/checked_read_scoped，只借用返回值检查 tag/body 长度与 SQL，不重建 CheckedFrame 或增加 F。复核发现的 fact_key 行号已订正为 content_producer.rs:1799。

实际完成以下检查：

- 四个 Rust 改动文件统一执行 `rustfmt --edition 2021 --config skip_children=true --check`，退出0。
- Python 从根入口遍历135个 mod/include 文件及其 cfg 链，以 Counter 保留同文件同名的多重计数：HEAD 基底964、native1532、消失2；工作树基底964、native1534、消失0。HEAD 比045 CI的1528多4，来自已封包M3四锁。此为静态枚举，不是 Rust `--list`。
- Python token 对比确认 projection 原116个 assert、SQLite原25个 assert 全部按原顺序保留；现分别116/35。另确认 source_audit、projection、ordinary_materials_tests 各自目标测试函数以外的 token 与 HEAD 完全相同、测试数量不变；CI完整摘要与加法一致，两payload、五门、三个实际源口及精确超槽检查保留。
- Python 独立静态模型：真实14行OHLC对应13组、11窗、五分型及十批坐标；暖缓存两payload各八个编码节点经SHA256映射到不同set，无碰撞挤出。完整预算路径11次源读取，200组仅最后大块多1次SQL；fail_at=1/4/9/25分别在0/1/2/6次读取后停止，均未到该大块。模型不代替Rust执行。
- 正本生成器 `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py` 重建为1756项，随后同一生成器 `--check` 实际退出0、报告与源码一致；清单仅删除旧完整字面 `暖原件不重读 SQL`。源码保留了这一检查及相同文字前缀，但诊断已含payload/fail_at格式参数，按现行生成器的格式串排除规则不再单列该字面。没有手改清单、生成器或放宽门。
- `check-prod-no-test-hooks.py` 退出0、violations=0；现有相关Python单测20项通过。`git diff --check` 退出0、暂存区为空；原交接笔记73753字节前缀SHA256保持 `c17dba9c23cb44d4e6dd40d0734e12b3d86e5469425a89f77c874f849cb2a7c9`，仅追加本节。

本轮保留完整W2生产写者串，未新增test_writer_rule、修改资源帽/准入语义或任何生产判据；未修改生产函数，未增加宽严两档或失败兜底，未处理#1481自身native栈溢出。阶段交付ready；Rust未验证，待CI，实际行为与同名增量锁仍须执行确认。

## M3C

本节绑定 #1419，处理 PR #1855@dde4cd4b 的 admission 与 s-observer 同因失败。开工分支为 `sandcastle/issue-1419`，HEAD 为 `dde4cd4b65bf77f176134971e19279a00f7c5a72`，工作区干净。

失败原文摘自 `/tmp/goal1323-s-observer-125128761166.log:363-368`：

```text
FAIL: test_rust_test_prepare_exists_only_under_cfg_test (test_tb01c_r4_anchor_lock.AnchorLockTests.test_rust_test_prepare_exists_only_under_cfg_test)
Traceback (most recent call last):
  File "/home/runner/work/NewChanlun/NewChanlun/s_session/tests/test_tb01c_r4_anchor_lock.py", line 44, in test_rust_test_prepare_exists_only_under_cfg_test
    self.assertRegex(window, r"#\[cfg\((all\()?test", f"{path} 在非测试上下文调用 test_prepare")
```

第 368 行断言诊断末尾原文为 `/home/runner/work/NewChanlun/NewChanlun/rust/src/bin/s_session_v3/audit_owner/w2_h1_structure_tests.rs 在非测试上下文调用 test_prepare`。本机修改前运行同一目标单测，实际复现该失败。

仅在 `w2_h1_structure_tests.rs` 的 `W2H1Fixture::open` 前增加 `#[cfg(test)]`。该文件经 `audit_owner.rs` 的 `construction_tests` 模块 `all(test, feature = "s_incremental")` 门和 `unix` include 门编入，函数声明与真实构建条件一致，不改变构建可达性。机械锁要求同文件调用前文存在测试 cfg，外层 include 门不能满足其文本判据。未改锁脚本、正则、测试内容或生产规则，未添加 allow。

实际检查：

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py -k test_rust_test_prepare_exists_only_under_cfg_test`：修改前失败，修改后 1 项通过；已核该目标方法只读源码，不调用 cargo。
- Python 按原锁完整复现定义唯一性、定义前 cfg、调用正则、定义排除及调用前文 cfg 判据，遍历 `rust/src` 全部 646 个 `.rs`，检查 113 处调用，违规 0。`w2_h1_window_tests.rs` 无直接 `test_prepare(` 调用，其余文件没有未覆盖调用，无需追加修改。
- `rustfmt --edition 2021 --config skip_children=true --check rust/src/bin/s_session_v3/audit_owner/w2_h1_structure_tests.rs`：退出 0，仅验证格式。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check`：退出 0，报告 `.github/prod-banned-literals.txt 与源码一致`；清单未受影响，无需重建。
- `git diff --check`：退出 0。核对源码相对 HEAD 仅新增上述属性；本笔记仅追加 M3C，未暂存、提交或回退任何改动。

Rust 未验证，待 CI。本机未运行 cargo/rustc、Rust 编译或测试。
