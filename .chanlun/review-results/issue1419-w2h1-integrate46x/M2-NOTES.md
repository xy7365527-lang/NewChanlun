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

M4 追加订正：上表及上一段保留为历史记录，其中“PR1793→045a 集成净变化 +30267”的归因已被逐读诊断推翻。正确归因是 **W2 链 `411db603→ab2bfac7` 内由 `4004c7cded`（bounded-layout/2 发展批来源绑定核）带入；集成段净增 0**。`ab2bfac7` 与集成候选 `de4fd36a` 的读序列逐行相同，运行对照为 [41401964388](https://github.com/xy7365527-lang/NewChanlun/actions/runs/41401964388) 与 [41401952562](https://github.com/xy7365527-lang/NewChanlun/actions/runs/41401952562)。订正依据为 `/Volumes/AgentStorage/cost-diag46z/REPORT.md:5-43` 与同目录 `final-buckets.txt`，不是新的本机 Rust 测量。

| 项 | PR1793 `411db603` | 后续 W2 链至 `ab2bfac7` | `ab2bfac7`→集成候选 | M4 修复前实测 |
|---|---:|---:|---:|---:|
| reads | 202561 | +30267 | 0 | 232828 |
| F | 224074 | +30267 | 0 | 254341 |

新增 30267 的逐读分桶为：重复 18858，其中 1a 已审单元重审 17640，拆为前缀 15876、条目 1764，1c 重读 `unit.bar` 1218；必需 11409，其中 1b 不同单元首次绑定核 4263、1d 批/条目结构遍历 7079、公共根闭包核验 67。原文“未逐节点分解”也由这份诊断补齐。M4 仅消除前缀末项自重复 4263 与 1c 的 1218；读数预期、待 CI 重冻范围与剩余重复的设计差异见文末 M4。

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

## M3D

本节绑定 #1419，修复 PR #1855@dde4cd4b 的 rust-check job 125128761231 所报两处 E0433。开工分支为 `sandcastle/issue-1419`，HEAD 为 `0a2b34040d08eb5d9e83b3fe2b7cad47403ebb8a`，工作区干净。

`w2_h1_structure_tests.rs` 原第 193、235 行的 `sc::TransportRef::from_json_view` 与 `sc::render_refview` 改用 `newchan_rust::s_content` 完整路径。已核 `s_content.rs:52` 的公开导出与 `:8158` 的公开函数；不新增共享模块别名，不改变测试逻辑。

用 `git grep -n -E '[[:alpha:]_][[:alnum:]_]*::'` 枚举 `w2_h1_structure_tests.rs` 与 `w2_h1_window_tests.rs` 的路径引用，逐项对照作用域。`audit_owner.rs:5663` 的 `construction_tests` 经 `use super::*` 导入 owner，owner 与 `s_session_v3/mod.rs` 也各有 `use super::*`。`v2`、`content_store`、`content_producer` 来自根 `s_structure_session.rs`；`driver`、`s_allocation`、`state` 在 `s_session_v3/mod.rs` 声明；`Ordering` 由 owner 导入，`Sql` 在使用处局部导入，其余为标准库、外部 crate、类型或显式 `crate`/`super` 路径。未发现其他缺失别名，window 文件无需修改。修改后两文件的 `git grep -n 'sc::'` 无命中，退出 1。

实际检查：

- `rustfmt --edition 2021 --config skip_children=true --check rust/src/bin/s_session_v3/audit_owner/w2_h1_structure_tests.rs`：退出 0。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check`：退出 0，报告 `.github/prod-banned-literals.txt 与源码一致`；禁串清单未受影响，无需重建。
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py -k test_rust_test_prepare_exists_only_under_cfg_test`：退出 0，1 项通过，cfg 判据无回退。已核目标方法仅扫描源码，不调用 cargo/rustc。
- `git diff --check`：退出 0。源码差异仅两处路径替换与必要换行，本笔记仅追加 M3D；未暂存、提交或回退任何改动。

Rust 未验证，待 CI。本机未运行 cargo/rustc、Rust 编译或测试。

## M4

本节绑定 #1419，执行用户授权的零新增分配修复及归因订正。开工分支 `sandcastle/issue-1419`，HEAD `de4fd36a540cd1f8174275dadfaf3c84e595a66a`，开工工作区干净。先完整阅读 `/Volumes/AgentStorage/cost-diag46z/REPORT.md`、`final-buckets.txt` 和本文件 M2D 第 1 节；报告行号均按该外部原件核对。本阶段只改 `source_audit.rs` 与本笔记，不新增 memo、表、跨审计资格，不改生产写者规则、判据或资源帽，不暂存、提交或回退改动。

### 1．零新增分配的两处修复

`audit_development_unit_coord` 返回 `(i64, RefValue)`，把本来已读的 `bar` 所有权连同 coord 移交调用者，没有 clone、重新构造引用或容器分配。`git grep -n audit_development_unit_coord` 穷举定义外三处调用：`source_audit.rs:2156` 的 unit wrapper 原样丢弃返回值，`:2213` 的当前条目解构并复用 `unit_bar` 比较，`:2240` 的 memo miss 解构 coord 并丢弃 bar。当前条目两条分支都免去原 `:2217` 的第二次字段读取，来源绑定、relations、source_ref 及 bar 相等判断保留。

无 memo 的前缀循环仅在 `index + 1 == units.len() && previous == &unit` 时免除重审，循环后 `units.last() != Some(&unit)` 的校验保留。这里的相等沿用 `RefValue::PartialEq`，比较 format/root/kind/extent，见 `audit_store.rs:1261`，不按指针地址或仅部分摘要放行。该末项指向本条目刚完整审核过的相同节点，因此没有新增资格或宽松判据。

非末项等于本条目 unit 的情况在现有审计判据下可能出现。局部核只要求 count 为正、前缀 Array/extent 一致、各 unit 合法及末项相等，没有前缀去重或坐标单调要求。例如 `[unit, unit]` 可满足现有判据。因此本轮没有把所有 `previous == &unit` 都跳过，非末项继续完整审核，没有新增拒绝条件。

memo 分支包括 `Some(memo)` 但表已禁用/空槽的情况都保留原控制流，不加入末项跳过；lookup、实际头部核验、`raw.get`、save 和 hits/misses/current_full/previous_full 计数保持。若把末项跳过也用于该分支，会改变 33W/P2 已冻结的工作量证据。两分支仍判相同单元合法性及末项相等，差别只在复用已审核节点与读数，不是 #799/#804 所禁的宽严两档或判据失败后的兜底。

### 2．归因订正与本轮冻结边界

原测试注释和 M2D 第 1 节表格均保留，分别追加订正。+30267 由 W2 链 `411db603→ab2bfac7` 内 `4004c7cded` 的 bounded-layout/2 发展批来源绑定核带入；集成段净增 0，`ab2bfac7` 与集成候选读序列逐行相同，运行对照 41401964388 vs 41401952562。证据为 `REPORT.md:5-23`、`:29-43` 与 `final-buckets.txt:1-17`。

| 分桶 | M4 修复前读数 | 本阶段处理 |
|---|---:|---|
| 重复 1a | 17640 = 前缀 15876 + 条目 1764 | 仅免前缀末项自重复 29×147 = 4263，其余 13377 留待裁定 |
| 重复 1c | 1218 | 复用已读 bar，全部免除 |
| 必需 1b | 4263 | 29 个不同单元各一次来源绑定核，保留 |
| 必需 1d | 7079 | 批/条目结构遍历，保留 |
| 必需闭包 | 67 | 公共根闭包核验，保留 |

修复前重复共 18858、必需共 11409，总计 30267。本轮预期免除 5481，剩余重复 13377；这些是报告分桶与修复范围的算术推导，不是本机 Rust 重测。

`full_some_audit_keeps_frozen_read_and_budget_trace` 仅把 reads/F 断言改成括号内完整前值减 `4263` 再减 `1218`，保留原累计来源表达式。reads 预期为 `232828 − 4263 − 1218 = 227347`，F 预期为 `254341 − 4263 − 1218 = 248860`。read_hash 仍为 `54375e9815ef8078d68d213e0a077f0b814ad7ce7a22432854441f1747eedfac`，budget_hash 仍为 `6dea88d382928c12c511d19a2acd7ce03c5e0b3ff4273408b1b66df1c92290a4`，测试注释明确“待 CI 实测重冻”。本阶段没有猜测或替换摘要，故尚不能称该 Rust 冻结测试通过；下一阶段由 CI 输出实测值统一重冻。

### 3．其他冻结与观测的逐项核对，均不改数值

以下按调用路径及现有断言静态区分数值冻结、动态对照和日志。没有执行 Rust，不能将“应保持”写成实跑通过。除上一节的完整 Some 外，扫描 `rust/src/bin/s_session_v3` 的 reads/F/hash、Native 与 proof/material 计数断言，未找到第二项确定需要重冻的硬编码完整审计 reads/F/hash；如 CI 报其他受影响冻结，下一阶段按实测统一处理，本阶段不预填。

| 文件与精确测试/载体 | 已冻结项及 M4 影响 |
|---|---|
| `source_audit_development_tests.rs:82` `development_previous_only_repeated_prefix_and_disabled_paths_match_frozen` | current_full=3；disabled previous_full=6、hits=0；enabled hits=6、previous_full=0。因整个 memo 分支保留，应保持；不改数字。 |
| 同文件 `:617` `development_measurement_totals_include_success_and_error_release` | current_full=4、previous_full=6/0、hits=0/6、capacity、分配与释放统计均应保持；bar 移动不新增分配。 |
| 同文件 `:331` `development_hit_rechecks_actual_header_and_f_and_releases_after_fault` | good hits=2，首 previous 头部故障 hits=0 应保持。F 仅要求为正，没有硬冻结总数；实际读取/F 由 CI 观察。 |
| 同文件 `:214` `development_sqlite_production_entry_hits_and_next_audit_has_fresh_raw_binding` | 结果、current_full 相等与 previous_full 减少为动态对照；F/material/proof 是日志，bar 复用会改变读/F 观测，不沿用旧读数。 |
| 同文件 `:417` `development_exclusion_budget_stop_at_ordinary_native_and_owned` | enabled/disabled 比较结果、SQL、F、预算事件和停止点，两臂均走无 memo 并获得同一修复；轨迹会变，无固定全读数可重冻，待 CI 对拍。 |
| 同文件 `:110` `development_first_errors_match_frozen_without_reordering_barriers` | 结果与首错冻结，不冻结读取序列；保留全部既有断言，待 CI。 |
| `sqlite_base_tests.rs:136` `sqlite_base_real_audit_matches_frozen_baseline_and_first_errors` | legacy S4 邻接锁，固定 Ok(1)、结果一致、proof hits>0、新 logical_reads<旧、slab_capacity 相等及持久表不变；没有固定完整 reads/F，不改。 |
| `.github/scripts/issue1419-g77-ab.py:75-84` `validate_work` | P2 两臂 current_full 相等、old.previous_full=new.previous_full+new.hits、两臂 misses=previous_full；memo 分支完整保留，不改合同。其 `validate_resources` 的 3 MiB proof 合计与 32 MiB 材料帽保留。 |
| `.github/scripts/test_issue1419_g77_ab.py:26-35` 合成夹具 | current29、previous120/12、hits0/108、misses120/12、proof 峰 2,953,712 B 是脚本测试数据；并非本轮 Rust 实测，不改。 |
| `financial_session_tests.rs:2428` `w2_pending_cost_probe` | F、logical_reads、SQL、work/events 为探针观测，会受 bar 复用影响，没有完整读取数硬冻结。 |

`sqlite_base_two_readers_overlay_clear_and_fresh_obey_domain`、`sqlite_base_actual_allocation_and_incremental_exclusion` 及 Native frame/field memo 的底层固定计数锁直接操作独立 reader/base/helper，不运行发展批，本轮不触及。`source_audit.rs` 的 local subset SQL/F 固定值同样只测字段读取助手，不进入发展批。Native 全审计与 deadline 路径确实少读，不能用这些底层锁推断其历史观测保持不变。

### 4．剩余 13377 次重复的设计差异，交主控裁定

本轮没有实施审计内单元表。若后续要减少跨条目、跨批记录的剩余 1a，表应在 `audit_generated_in_store` 已取得 `allocation` 后，与 `checked_refs` 同作用域创建，跨本次对象循环共享，并随成功返回、首错或截止退出一并析构。不能放进持久 NativeSourceAudit、OrdinaryProofs、state 或跨 cut 的缓存，也不能沿用上次审计的 raw 绑定资格。

候选键值为通过完整单元核后保存的完整摘要到 coord，键须维持现有 `DevelopmentTable::key` 的 format/kind/extent 防护。命中沿用原 `:2232-2236`、现 `:2232-2236` 的语义：对实际引用执行 `audit_verify_refview_headers`，再对本审计 raw 做 `raw.get(&coord)`，成功后才计命中；不得把缓存命中本身当来源存在或帧合格证明。当前条目仍完整审核，前缀 count、遍历及末项相等判据不变。命中头部读取本身仍有成本，13377 不是未来表可净省的读数承诺。

分配必须接现有合同，不能把 SQLite `DevelopmentTable` 的 std Vec 直接搬进所有路径。沿 `source_audit.rs:879` 的 `source_allocation` 与 `business_json.rs:327` 的 `bj::new_map(allocation.as_ref())` 创建受管表；Native 插入前须调用 `s.native_map_room`，其经 `audit_store.rs:227` 接 `BusinessMap::try_reserve_native` 的 WorkScope 许可。该实现计实际桶容量、控制区及扩容时新旧块共存费用，失败直接传播；Owned 原型则由现有 OwnedBudget 记账，不退回未计费 std。表的固定元数据、峰值与成功/失败释放也需计入现有 allocation/工作域证据，不能新增旁路额度、抬资源帽或吞掉分配错误。

这会触及 `NativeSourceAudit::audit` 的 measure budget/deadline 路径及带 deadline 的 `audit_generated_for_audit`。减少读取会改变 budget 事件/F/截止采样位置；增加表还可能提前触发既有内存预算拒绝，因此不是本轮“零新增分配”的等价扩展。后续实施须明确每次查表、插入、扩容的 deadline 检查边界，核失败停止顺序与释放；不能为复用绕开 HashReader 的门，也不能靠补虚假预算事件维持旧账本。保持资源帽的同时是否接受这些受管成本变化，交主控裁定后再实施并实测重冻。

`git show -s --format=fuller 88f3a35309` 的提交说明只有 `perf(#1419): reuse previous development units within one SQL audit`，没有更长的预算排除论证。限制的明确依据是 [#1419 issuecomment-6526264864](https://github.com/xy7365527-lang/NewChanlun/issues/1419#issuecomment-6526264864) 的 P2 口径：仅同次真实 SqliteBase 审计，current 完整核，原 3 MiB proof 合计与 32 MiB 材料帽，不扩 Owned/Closed、fresh 或预算域；[issuecomment-6527788241](https://github.com/xy7365527-lang/NewChanlun/issues/1419#issuecomment-6527788241) 登记 previous_full 120→12、hits 108、current 29 不变及 proof 合计峰值 2,953,712 B。配套设计原件 `/Volumes/AgentStorage/review1419-development34z/REPORT.md` 第 2 至 4 节要求 Some 保原 SQL/F/预算事件，不向预算路径补虚假事件，memo 与 closure 合计不超 3,145,728 B，不能挪用 32 MiB 材料池或跨 cut。

当前代码据此在 `audit_generated_in_store` 要求 ordinary/budget/deadline 均为空、Bounded2 且 `eligible_sqlite_base()`；`history.rs:3876` 还排除 Native/Owned 工作域、非 Ordinary policy、render 上限、overlay/fresh 和 s_incremental 等非真实 SQL 基源。现有会话的 Vec 租用的是 SqliteBase 共享 proof 许可，不持上述 Native 工作域/Owned allocation 的分配合同，不能直接解除 eligibility 门。M4 只按本阶段新授权免除可证明重复读取，没有放宽该门。

### 5．实际检查与交棒

- `rustfmt --edition 2021 --config skip_children=true --check rust/src/bin/s_session_v3/source_audit.rs` 退出 0，仅检查格式，不代表编译通过。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check` 退出 0，报告 `.github/prod-banned-literals.txt 与源码一致`。未增删测试字面量，禁串未受影响，无需重建；没有手改清单或生成器。
- 完整阅读 `test_tb01c_r4_anchor_lock.py`，复核 Rust `test_prepare` 唯一定义及其 cfg、调用点前文 cfg 判据、生产 Python 入口隔离与无锚 CLI 拒绝语义。实际执行 `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py`，3 项通过；该检查不调用 cargo/rustc。
- 独立非作者评审分别核需求轴与规范轴，均无发现；核对全部三处 helper 调用、无 memo 的末项限制与 memo 计数路径，确认返回既有 RefValue 为所有权移动，没有新分配语句。
- 独审用纯 Python 核对原笔记全部行及原测试历史注释按原顺序保留、两个摘要和生产写者规则串未改。`git diff --check` 退出 0，暂存区为空，HEAD 保持开工值；工作区全部改动仅为本笔记与 `source_audit.rs`。

本阶段 ready。Rust 未验证，待 CI。本机未运行 cargo/rustc、Rust 编译或测试，未暂存或提交。下一阶段按 CI 实测统一重冻完整 Some 的两个摘要及任何实际报出的其他受影响冻结；本阶段预期值不冒充实测。


## M5

# #1419 M5：并入 H1 bea93e19 的冲突处理

本笔记是绑定 #1419 的阶段工作草稿，按用户要求留在工位外供宿主搬运。

阶段状态为 ready，仅表示冲突文件已处理。非冲突源码发现三处确定的 typed 接口适配缺口，按本阶段边界列入下一阶段，见下文。Rust 未验证，待 CI。

## 开工状态与范围

- 分支为 `sandcastle/issue-1419`。
- HEAD 为 `b1f5a9d7241ddd7616ae86d6d9a1b0dbef313668`。
- MERGE_HEAD 为 `bea93e191971af3b7e4a607d3acb7669359fd642`。
- 开工 `git diff --name-only --diff-filter=U` 仅有 `.github/prod-banned-literals.txt`。
- 只修改上述冲突文件。非冲突文件的自动合并内容与暂存区均保留开工状态。
- 阅读工位 M2-NOTES 的接口与禁串裁定、M2C/M2D/M3/M4 记录，复核 H1 `42a3f959..bea93e19` 四个提交的差异及外盘 S1D-REPORT 末尾检查点。另派非作者工蜂只读复核全部笔记、增量、改签名函数与调用者。

## 唯一冲突文件：.github/prod-banned-literals.txt

HEAD 是 W2 × H1 经 M1–M4 整合后的 1756 项生成清单，包含 W2 结构对象的测试字面和此前夹具、格式串处理结果。H1 新增 S1S/S1T 合同夹具后清单为 1647 项；它新增 P6 对照臂、identity 和 schema 前缀检查字面，同时删除已被新断言替换的旧字面。

实际冲突位于 `startup不得沿用旧请求缓存` 后。HEAD 一侧有以下四项，H1 一侧为空：

```text
startup原件不能跨着恢复请求保留
subject 悬空须判出
subject 指向另一实际对象须判出
subject_id 非文本
```

按 M2-NOTES:7–16、:683–739 的 owner 裁定，清单是原生成器的精确输出，不采用手工并集。执行现有 `.github/scripts/gen-prod-banned-literals.py` 重建，未修改生成器、checker 或其规则。

- 三项 subject 字面全部保留，当前源分别在 `rust/src/bin/s_session_v3/generation.rs:7422`、`:7414`、`:7237`，保留 W2 的悬空、错对象、字段类型检查覆盖。
- 删除 `startup原件不能跨着恢复请求保留`。H1 已改为核对发布前 slot 的 generation、identity 重建后的当前 generation、完整审计次数和后续命中次数，旧字面在合并后全 rust 搜索无命中。新字面 `对照臂仍持有发布前原件` 与 `identity 必须重建当前发布前沿的原件` 均入清单。
- 同步接受已自动合并的 H1 新字面。最终相对 HEAD 新增 11 项、删除 3 项，为 `1756 + 11 - 3 = 1764` 项。其余 W2 字面逐项保留。
- 另外两项删除为 `内层已完整成功并retain后，epoch才拒绝` 和 `期限门之前业务核已完成retain`。两项均已从当前 Rust 源码消失，被 H1 的 schema-only 前缀断言及独立实际 startup 对照覆盖。没有删除测试或检查。
- 清单保留原两行头、按生成器排序，无重复，无冲突标记。

该文件不含函数，无改名或改签名。全仓 `git grep -n -F prod-banned-literals.txt -- .github/scripts` 核对生成器、检查脚本、产物消费者及单测；生成器仍为唯一产物源，消费者仍逐字节匹配，未缩减扫描规则。

## 两侧源码保留与调用者核对

所有自动合并 Rust 文件均未修改。对开工时已自动合并的文件逐一计算 SHA-256，与结束状态对拍相同；`git ls-files --stage` 全量字符串也与开工一致。

因此 HEAD 的 PublishError 接口、M2C/M2D/M3/M4 的锁、去重修复与完整 W2 写者规则保持；H1 S1R 的 Box 返回、`#[inline(never)]`、薄测试入口与独立 body、测试水位探针，以及 S1S/S1T 的夹具改动均保留。没有重新设计栈修复，没有使用 test_writer_rule，没有改资源帽或增加判据失败后的兜底路径。

完整生产规则仍为：

```text
s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1+raw-initialization-w/2+bounded-layout/2
```

S1R 的原始 `Box<Driver>`、inline 属性及 probe 改动已与 `git diff 29326e4d^ 29326e4d` 核对。非作者复核从该提交提取 18 个连续新增段，当前源码全部逐字保留，admission_tests.rs 整文件与 bea93e19 逐字相同。外盘检查点 22 记录 H1 原分支目标测试已 ok；该收据不能代替当前 W2 集成候选的 Rust 验证。检查点 23/24 的 S1S/S1T 断言修正随非冲突文件保留。

以下均已逐名实际执行全仓 `git grep`，未按 cfg 过滤，同名符号再按模块和局部作用域筛出真实调用。路径省略前缀 `rust/src/bin/s_session_v3/`。

| 新增或改签名函数 | 当前调用者核对 |
|---|---|
| `admission_tests.rs:512` 的 `h1_admission_driver`，返回 `Result<Box<Driver>, String>` | 同文件 :164/:224/:434/:465/:495/:575/:615/:717/:824/:951/:1233/:1251/:1369，另 `audit_owner/legacy_admission_tests.rs:8`。均为借用、方法调用或 drop；`h1_admission_driver_commit(&mut Driver)` 由 DerefMut 适配，没有按值 Driver 消费。 |
| `admission_tests.rs:567` 的 `h1_admission_driver_cold`，返回 `Box<Driver>` | 同文件 :638/:650/:737，均适配 Box。 |
| `admission_tests.rs:610` 的目标测试 body | 唯一调用 :604；原测试名与薄入口保留。 |
| `driver.rs:2308` 的局部 `open`，返回 `Box<Driver>` | 本测试中 :2337/:2339 及 epoch2 重开的 :2380/:2382，全部沿新 helper。全仓同名函数按局部作用域排除。 |
| `driver.rs:5432` 的局部 `open`，返回 `Result<(Box<Driver>, identity), String>` | :5463/:5469/:5526。局部 `ordinary` 定义 :5451，唯一调用 :5455。 |
| `audit_owner.rs:7594` 的 `p6_publish_full_lease_for_test` | 唯一真调用 `driver.rs:9398`。定义与调用保持 H1 对照臂语义，但有下文 typed 缺口。 |
| `audit_owner.rs:7663` 的既有 `p6_source_context_for_test` | `driver.rs:9396` 与 `prepared.rs:2291/:2566/:2832/:2848/:3050/:3137/:3173/:3209/:3259`，签名未改。 |
| 新 probe 的 `enter` / `sample_codec` / `record` | enter 在 `admission_tests.rs:603`；sample_codec 在 `history.rs:3932/:3983`；record 在 `audit_owner.rs:595/:753/:826/:945`、`admission_tests.rs:514/:605/:612`、`driver.rs:877/:900/:906`、`source_contributions.rs:2526`、`state.rs:1369`，另 probe 模块 enter 内部 :36。测试、增量、Unix 门控匹配。 |

另核 HEAD 的完整规则常量 `rust/src/bin/s_structure_session.rs:139`、生产规则文件、policy records 与 M4 `source_audit.rs` 相对 HEAD 无差异。ColdHistoricalSourceFold 与 audit_source_step 的 typed 返回保留，H1 probe 仅前插，不替换原核。

## 非冲突文件：下一阶段必须适配的 typed 缺口

以下全部位于未获准修改的 `rust/src/bin/s_session_v3/audit_owner.rs`，均在 H1 新增的 `p6_publish_full_lease_for_test` 中，定义为 `:7594`，外层返回 `String`，codec 闭包在 `:7641` 显式标成 `Ok::<_, String>`。

| 调用位置 | 被调函数与当前定义 | 确定缺口 |
|---|---|---|
| `rust/src/bin/s_session_v3/audit_owner.rs:7613` | 同文件 `before_fixed_write`，`:4279`，返回 `Result<(), PublishError>` | 新 helper 用 `?` 直接传播到 String 闭包，缺 typed 适配。 |
| `rust/src/bin/s_session_v3/audit_owner.rs:7636` | `rust/src/bin/s_session_v3/prepared.rs:524` 的 `Runtime::publish_in_owner`，返回 `Result<CommitReceipt, PublishError>` | codec 闭包显式 String，直接 `?` 不匹配。 |
| `rust/src/bin/s_session_v3/audit_owner.rs:7638` | 同文件 `finish_fixed_write_in_publication`，`:4473`，返回 `Result<(), PublishError>` | 同一个 String codec 闭包直接 `?` 不匹配。 |

全仓 grep 确认新 helper 的唯一调用是 `rust/src/bin/s_session_v3/driver.rs:9398`，在独立 full_lease 测试对照臂中。后续应保留 HEAD 的 PublishError 合同，让新 helper 内部及 codec 工作闭包使用 typed 错误，只在既有 String 外边界调用 `into_message`。不得把生产核退回 String，不得删前后写入检查或以另一条准入路径绕过。

这些是静态可证的错误类型不一致，不是本机编译诊断；本阶段 ready 不表示集成候选可编译。下一阶段需修复并由 CI 验证。

新增错误相等断言另行核过，未发现同类确定缺口。prepared.rs:3176/:3212 的结果来自 String 入口 `AuditConnection::publish`，不是 typed 的 `PublishPhase::publish`；前缀 oracle 的 schema、epoch、rule 验证入口也均为 String。

## 实际检查

- `python3 -B .github/scripts/gen-prod-banned-literals.py .`，退出 0，生成 1764 项。
- `python3 -B .github/scripts/gen-prod-banned-literals.py --check .`，退出 0，与源码一致。
- `python3 -B .github/scripts/check-prod-no-test-hooks.py`，退出 0，`violations=0`。
- `rustfmt --edition 2021 --config skip_children=true --check` 显式覆盖自动合并的七个 Rust 文件，退出 0：audit_owner.rs、audit_owner/admission_tests.rs、driver.rs、history.rs、prepared.rs、source_contributions.rs、state.rs。仅为格式检查。
- 重建后 `git diff --check`，退出 0。
- Python 检查清单 1764 项排序、唯一性、冲突标记清零，全部通过。
- Python 校验开工已暂存的非冲突文件 SHA-256 全部相同、完整 index stage 列表不变，全部通过。未暂存差异仅有原冲突文件。
- 未执行 cargo/rustc、Rust 编译或测试，未执行需 Rust 产物的检查。Rust 未验证，待 CI。
- 未执行 git add/commit。因用户禁止 git add，index 仍保留该文件的冲突三阶段，`git diff --name-only --diff-filter=U` 仍列它；工作文件内容已无冲突标记，留给宿主验收并暂存。


## M5B

本节绑定 #1419，按用户授权适配 M5 合并后 H1 新 helper 的 typed 接口。开工分支为 `sandcastle/issue-1419`，HEAD 为 `b3f2c358f9631991a31aced1059692308fddcc30`，工作区干净。先完整阅读 `/Volumes/AgentStorage/w2-h1-integrate46x/M5-MERGE-NOTES.md`，并复核本文 M2/M2C typed 合同。沿用本票，不另开票；本阶段未运行 cargo/rustc，未暂存、提交或回退改动。

### helper 接口适配

只改 `rust/src/bin/s_session_v3/audit_owner.rs:7594` 的 `p6_publish_full_lease_for_test`，外层仍返回 `Result<(commit::CommitReceipt, Ended), String>`。完整执行闭包显式返回 `PublishError`，codec 闭包的 `Ok::<_, String>` 改为 `Ok::<_, PublishError>`。`before_fixed_write`、`Runtime::publish_in_owner`、`finish_fixed_write_in_publication` 三个 typed 调用继续直接用 `?`，不改被调用函数签名，也不在这些调用之间擦除错误来源。

native 失败归一闭包改收 `native::Failure<PublishError>`。`codec_work` 仍返回既有 `Failure<String>`，经 `native.rs:28` 的现有转换上提；`run` 的 `WorkError<PublishError>` 经 `native.rs:1553` 的泛型转换处理，`Operation(PublishError)` 保持原 typed 错误，`Invalid(error)` 直接返回原错误。预算失败文案保持不变，只转入 typed 返回。既有 String helper 的结果由 `ContextError::From<String>` 上提，最终在原 helper 的 String 出口调用一次 `result.map_err(PublishError::into_message)`。

helper 唯一调用者仍是 `rust/src/bin/s_session_v3/driver.rs:9398` 的 full_lease 测试对照臂，无需改调用者。保留 raw/capture/source 为 None 的对照方式、写入前后检查、事务及旁挂台账提交顺序、错误后的 retired/候选清理/台账失效/runtime 丢弃/context 释放。没有增删测试、修改断言、生产规则、资源帽或准入判据。

### 笔记搬运与实际检查

- 将外盘 M5 笔记全文原样并入上面的 `M5` 小节，原件 9653 字节，SHA-256 为 `2fdbcdcb4259c057588bafe8c04269f2f8fcc7850e4b5b750c21a520ee9b4704`。本阶段开工笔记的 104452 字节前缀完整保留，其 SHA-256 为 `5ec02a818663980f56ae2872f1a02579a639d5b5fb44f4dc87d6ac53157ebd94`。
- `rustfmt --edition 2021 --config skip_children=true --check rust/src/bin/s_session_v3/audit_owner.rs` 实际退出 0，仅验证格式。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check .` 实际退出 0，报告 `.github/prod-banned-literals.txt 与源码一致`。helper 字面量及 cfg 均未改变，禁串清单不受影响，无需重建；未改生成器或手改清单。
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py` 实际退出 0，3 项通过。已读完整脚本，确认本命令不调用 cargo/rustc。
- Python 静态比对 HEAD 与工作树：helper 外源码逐字相同；helper 的字符串字面量、写入检查、提交和失败清理调用次数、full_lease 的 None 字段全部保持。完整文件的函数名顺序与 `#[test]` 数量不变；helper 内只有最终出口一次 `into_message`。
- `git diff --check` 实际退出 0。未暂存、提交或回退；最终工位改动为本笔记与 `audit_owner.rs`。

Rust 未验证，待 CI。上述格式、Python 锁与静态核查不证明 Rust 编译或测试通过；本阶段 ready 仅指已完成本轮接口适配及指定本机检查。


### M5B 独立只读静态核查

本核查绑定 #1419，基线为 `git diff b1f5a9d7 HEAD -- rust`，当前 HEAD 为 M5 合并结果 `b3f2c358f9631991a31aced1059692308fddcc30`。已完整阅读外盘 M5-MERGE-NOTES，并阅读 M2 类型传播与终界及 M2C feature 组合合同。仅写工位外 `/tmp` 报告，未编辑仓内文件、未运行 cargo/rustc、未暂存或提交。

对 7 个 Rust 差异文件按新增/删除行与函数词法范围相交枚举，共 35 个新增/改动函数、34 个不同名字；两个局部 `open` 单独记行。逐名实际执行 `git grep -n -w -F NAME -- rust`，不按 cfg 排除；原始结果为 `/tmp/issue1419-m5b-grep.txt`，范围及完整 HEAD 函数正文为 `/tmp/issue1419-m5b-functions.json`。一般同名函数另按模块、具体接收者及局部作用域排除，测试函数仅定义命中时由 Rust 测试入口消费。该枚举也覆盖仅改函数体、局部函数、闭包所在外层函数与 probe `Drop`，没有把“改签名函数”误作完整范围。

下表路径均省略 `rust/src/bin/s_session_v3/`。`T` = `test`，`I` = `s_incremental`，`U` = `unix`；表中 cfg 为相对 v3 模块的生效门。新增 probe 的调用统一落在 `T+I+U`；`source_contributions` 的 I 来自 `source_audit.rs:32` 的模块门，`advance_cold_sources` 的 I 来自 impl 门。`s_capture_native_testonly` 在 `rust/Cargo.toml:87` 明确依赖 I，已在同一份不删 cfg 的源码中核查，没有因当前本机 feature 状态跳过这些路径。

| 文件:HEAD 行 | 新增/改动函数 | 生效 cfg、返回值及真实调用核对 |
|---|---|---|
| `audit_owner.rs:23` | `drop` | T+I+U；返回 ()。Entry 的 Drop 实现，由 admission_tests:603 的 `_entry` 离开测试帧时调用；不是全仓其他类型的 drop。只报告水位并清空 ENTRY，无错误传播。 |
| `audit_owner.rs:31` | `enter` | T+I+U；返回 Entry。唯一外部调用 admission_tests:603；内部 record 位于 owner:36；无 Result/问号。 |
| `audit_owner.rs:39` | `sample_codec` | T+I+U；返回 ()。history:3932/:3983 两处调用，均同门；无 Result/问号。 |
| `audit_owner.rs:50` | `record` | T+I+U；返回 ()。owner:36/:595/:753/:826/:945，admission_tests:514/:605/:612，driver:877/:900/:906，source_contributions:2526，state:1369；全为水位旁观，不消费错误。 |
| `audit_owner.rs:581` | `open_rebuilt_history_into` | I+U；Result<Duration, String>。driver:582/:886 两处外部调用；只新增返回 () 的 probe，原有 String 边界和内部 typed→String 处理不变。 |
| `audit_owner.rs:807` | `finish_rebuilt_history` | I+U；Result<Duration, String>。唯一 owner:788 调用；只新增 probe，原有 String 核与闭包不变。 |
| `audit_owner.rs:7594` | `p6_publish_full_lease_for_test` | T+I；外层 Result<(CommitReceipt, Ended), String>，修复后操作闭包与 codec 闭包均 PublishError。唯一外部调用 driver:9398，state::traced 透传 Result，随后 unwrap/continue_with；详见后文。 |
| `audit_owner/admission_tests.rs:512` | `h1_admission_driver` | T+I+U；Result<Box<Driver>, String>。admission_tests:164/:224/:434/:465/:495/:575/:615/:717/:824/:951/:1233/:1251/:1369，legacy_admission_tests:8，共 14 调用；底层 open_with_request_and_mode_retention 为 String，map(Box::new) 不改错误。全部调用借用、方法调用或 drop，无按值 Driver 消费。 |
| `audit_owner/admission_tests.rs:567` | `h1_admission_driver_cold` | T+I+U；Box<Driver>。admission_tests:638/:650/:737 三调用；内部 :575 traced 取 h1_admission_driver 的同一 String Result 并 unwrap；借用处经 DerefMut 适配。 |
| `audit_owner/admission_tests.rs:601` | `h1_admission_driver_cold_recover_rebuilds_ledger_bitwise` | T+I+U；返回 ()。测试入口，grep 仅定义；调用 probe enter、独立 body、probe record，无 Result/问号。 |
| `audit_owner/admission_tests.rs:610` | `h1_admission_driver_cold_recover_rebuilds_ledger_bitwise_body` | T+I+U；返回 ()。唯一 admission_tests:604 调用；原测试主体保留，Box 被借用给 h1_admission_driver_commit(&mut Driver)，无 String/PublishError 传播。 |
| `driver.rs:867` | `recover_rebuilt_history` | I+U；Result<Value, String>。唯一 driver:859 调用；新增 probe 全返回 ()，其余 cold_context/open_rebuilt_history_into/recover 原调用和错误处理不变。 |
| `driver.rs:1756` | `native_prefix_preserves_business_epoch_rule_order_and_clears_context` | T+I；测试入口 ()，无显式调用者。新增 prefix_expected 的 schema/epoch/rule/meta 四入口均为 String，and_then 内 meta_get_opt? 与 String 相合；新增 startup 对照亦 String，.err/unwrap 相合。 |
| `driver.rs:1841` | `native_prefix_populated_capture_type_error_precedes_frame_and_epoch` | T+I；测试入口 ()。新增 schema/epoch 两段 and_then 均 String；实际 startup 对照错误也是 String；新增断言无 typed 值比较问题。 |
| `driver.rs:1903` | `native_prefix_context_rechecks_real_connection_current_material` | T+I；测试入口 ()。新增 startup(...).unwrap() 消费既有 String 边界；same/append/拒绝断言只核现有业务结果，没有 typed 闭包。 |
| `driver.rs:2002` | `native_prefix_outer_epoch_failure_clears_retained_slot_without_reviving_pool` | T+I；测试入口 ()。verify_writer_epoch、admit_prefix、Driver open 的错误均 String；audit_native_in_request 只 unwrap，不向 String 直接问号传播。 |
| `driver.rs:2116` | `native_prefix_final_deadline_preserves_business_and_clears_retained_context` | T+I；测试入口 ()。run/clock/test_with_clock_account 内层闭包分别返回元组、Instant、元组，无新增错误问号；新增独立 startup 对照为既有 String 外口。 |
| `driver.rs:2305` | `native_delta_explicit_policy_sql_receipt_and_failed_commit_retry` | T+I；测试入口 ()。四处改用下行局部 open 返回 Box；原失败注入闭包、native.handle String Result 未换错误类型，traced 原样透传。 |
| `driver.rs:2308` | `open` | T+I，属于上行测试局部作用域；Box<Driver>。driver:2337/:2339/:2380/:2382 四调用。Driver::open_with_request_and_mode_context 原返回 String，map(Box::new).unwrap() 类型一致。 |
| `driver.rs:5429` | `native_bootstrap_idle_identity_hits_then_drops_and_ready_is_new_cold_default_paths_unchanged` | T+I；测试入口 ()。覆盖下面两个局部 helper；ordinary 对照 traced 闭包仍 Ok::<_, String>，其中 open_with_request_and_mode_context 与 identity 均 String，map(Box::new) 仅改成功值。 |
| `driver.rs:5432` | `open` | T+I，属于上行 bootstrap 测试局部作用域；Result<(Box<Driver>, identity), String>。driver:5463/:5469/:5526 三调用；open_for_serve_with_identity 原 String，map 闭包仅 Box 包装。 |
| `driver.rs:5451` | `ordinary` | T+I，属于同一 bootstrap 局部作用域；Result<Box<Driver>, String>。唯一 driver:5455 调用，Fixture::open 原 String，map(Box::new) 不变错误。 |
| `driver.rs:5551` | `native_bootstrap_pending_drops_startup_slot_before_resume_then_identity_cold_matches_current_frontier` | T+I；测试入口 ()。只改计数/slot 与 identity 对拍，新增两次 identity_in_bootstrap 均 unwrap；没有新增 typed→String 传播。 |
| `driver.rs:5638` | `native_bootstrap_two_transactions_reject_current_damage_or_sql_error_without_old_proof` | T+I；测试入口 ()。只增加/调整审计次数和 context 计数断言；原 SQLite hook、identity 错误不变。 |
| `driver.rs:9248` | `p6_complete_ledger_cover_matches_across_delta_and_full_lease_owner_paths` | T+I；测试入口 ()。driver:9396 新请求上下文，:9398 调用 full_lease helper；traced/unwrap 不约束为 PublishError；原 full_lease 车道、每代一次完整分支和逐行台账对拍保留。 |
| `history.rs:3928` | `read_scoped` | HashReader 的 NodeSource 方法；Result<ScopedNodeRead, CodecError>，新增 probe 为 T+I+U。history:3926 的 read、cold_source:58 的 trait 转发、effects:1465 显式 map_err(publish_err)、泛型 NodeSource 消费者共同沿 CodecError 合同；完整 28 个同名命中按接收者排除其他实现，未改错误/trait 签名。 |
| `history.rs:3976` | `checked_read_scoped` | HashReader 的 NodeSource 方法；Result<Option<ScopedFrame>, CodecError>，新增 probe 为 T+I+U。history:3973 checked_read、cold_source:70 与 effects:1211/:1393 的 trait 转发，以及泛型 NodeSource 消费者沿 CodecError 合同；完整 55 个同名命中按接收者排除其他实现，未改错误/trait 签名。 |
| `prepared.rs:2825` | `p6_queued_units_real_owner_commits_only_verified_complete_keys` | T+I；测试入口 ()。prepared:2832/:2848 两个上下文改 p6_source_context_for_test；owner.publish 仍 String 并 unwrap，没有 typed 闭包或签名变动。 |
| `prepared.rs:3024` | `p6_complete_ledger_real_owner_cover_tracks_revisions_and_rebuild` | T+I；测试入口 ()。prepared:3050 上下文更换，owner.publish 仍 String/unwrap；台账及重建检查不改。 |
| `prepared.rs:3112` | `p6_complete_ledger_real_owner_records_pending_without_group` | T+I；测试入口 ()。prepared:3137 上下文更换，owner.publish 仍 String/unwrap；pending 与 cover 对拍不改。 |
| `prepared.rs:3160` | `p6_complete_ledger_rejects_partial_and_wrong_rule_without_commit` | T+I；测试入口 ()。prepared:3173 新上下文、:3174 owner.publish 返回 String，:3176 起新增错误逐字相等断言比较 String 与字符串字面；不是 PublishPhase::publish 的 typed 返回。 |
| `prepared.rs:3197` | `p6_queued_units_prepare_failure_discards_facts_and_stales` | T+I；测试入口 ()。prepared:3209 新上下文、:3210 owner.publish 返回 String，:3212 起逐字错误断言相合；没有 typed 值与字面直接比较。 |
| `prepared.rs:3242` | `p6_queued_units_ledger_commit_failure_keeps_content_and_never_retries` | T+I；测试入口 ()。prepared:3259 上下文更换，owner.publish 的错误沿 String，contains 原断言适配；提交与不重试语义保持。 |
| `source_contributions.rs:2508` | `audit_source_step` | I；Result<(Counts, NativeSourceState, SourceExports), PublishError>。source_contributions:2491 audit_native_bound 与 state:1392 advance_cold_sources 两真调用均 PublishError；source_audit:42 仅重导出。新增 T+U probe 叠加模块 I 后同定义门。 |
| `state.rs:1355` | `advance_cold_sources` | I；Result<(Counts, SourceExports), PublishError>。唯一 state:1469 ColdHistoricalSourceFold::verify 调用同为 PublishError；新增 T+U probe 叠加 impl I 后同定义门。 |

补充调用者核对：`p6_source_context_for_test` 自身不是本次 H1 差异中的改动函数，仍逐名实际 git grep，定义在 owner:7663，所有 10 个调用为 driver:9396 和 prepared:2291/:2566/:2832/:2848/:3050/:3137/:3173/:3209/:3259；均直接取 RequestAuditContext，无 Result。`h1_admission_driver_commit` 也另行逐名 git grep：参数仍 `&mut Driver`，新 Box 调用通过 DerefMut 借用，未新增按值 Driver 消费。全仓 probe 的 `record`/`enter` 同名命中分别有大量其他业务标识，表中只列实际 h1_stack_probe 接收者；Entry::drop 属语言析构入口，不把无关类型 drop 算作其调用者。

### 对本轮 full_lease 修复的独立复核

原来的三处确定缺口均在 `p6_publish_full_lease_for_test`：owner:7613 `before_fixed_write`、:7636 `Runtime::publish_in_owner`、:7638 `finish_fixed_write_in_publication` 都返回 PublishError，原操作帧/codec 闭包为 String。本轮只把这两个内部帧统一为 PublishError，未修改上述生产函数签名。

- `native::Failure<PublishError>` 的 `Invalid(error)` 原样返回 typed 错误，BudgetExceeded 仍生成原来的 `P6完整对照臂父预算不足` 文本并转入 PublishError。
- `Budget::codec_work` 在 native.rs:966 仍返回 `Failure<String>`，通过 native.rs:28 的专用 `From<Failure> for Failure<PublishError>` 接入 typed failure 回调；其预算与 Invalid 两分支均显式保留。`CodecWork::run` 在 scratch.rs:428 泛型返回 `WorkError<E>`，此处 E 由 `Ok::<_, PublishError>` 锁定。native.rs:1553 的 `From<WorkError<O>> for Failure<E>` 与 :23 的 PublishError→Failure<PublishError> 完整接住资源错误和操作错误；无错误文本化或吞错。
- 原 String 子入口通过既有 `ContextError::from(String)`（s_content.rs:237）进入同一 typed 帧；候选缺失显式 Err 同样使用 `.into()`。全部前/后写检查、construction seal/transition/run、publication commit、ledger commit 仍按原顺序执行；错误发生后 retired、clear_candidates、invalidate_audit_ledger、discard、context.take 清理仍全部保留。
- 外部签名仍为 String；唯一 `PublishError::into_message` 在最后 `result.map_err(...)`，位于错误清理之后。没有添加 `From<PublishError> for String`，没有中途降级或失败兜底。
- `PublishPhase` 的 audited 候选、publication 父域、`raw: None`、`capture: None`、`source: None` 及唯一调用者不变，独立 full_lease 对照臂语义保持；没有删除测试或写入检查。

结论：除已知 helper 的这三处外，在上述 35 个 H1 新增/改动函数及其全仓调用者中，未发现另一处静态可证的 String/PublishError 不一致。本报告为源码/类型合同核查，Rust 未验证，待 CI。

## M4B

本节绑定 #1419，按 48F 独审处理 M4 当前条目的 `unit_bar` 持有期。开工 HEAD 为 `37b58810d7a4261803b32b71a53b67c449e47529`，工作区干净。完整阅读 `/Volumes/AgentStorage/m4-review48f/REPORT.md` 与本笔记 M4 节；独审唯一阻塞为 B1，没有其他持有期修复项。

独审原文要点：“`unit_bar` 是带 Drop 的 `RefValue`，Rust 在作用域末尾析构，不按最后一次使用提前释放”；“Native 路径上会多持有一份 `NativeReservation` 跨过前缀审计，抬高峰值准入占用”。报告由此指出边界资源拒绝可能改变，要求“比较后立即释放”。这是源码所有权与析构语义分析，报告及本阶段均无资源峰值实测。

`source_audit.rs:2217` 将直接 `if unit_bar != bar` 改为先计算 `let same_bar = unit_bar == bar;`，下一句 `drop(unit_bar);`，再按 `if !same_bar` 返回原错误。`PartialEq` 借用两侧，仍逐项比较 format/root/kind/extent；比较完成后只保留布尔值，显式 drop 消费 `unit_bar` 并析构其载荷与预留。相等和不等两条路径都先释放，再进入原后续字段读取或返回错误，因此不跨越前缀循环，也不早于比较完成；释放点不晚于原重读 bar 的比较语句结束。`memo.save(&unit, coord)` 保持原位置，未增加读取、clone 或分配。

穷举当前文件 helper 定义外的三处调用：`:2156` wrapper 保持 `.map(|_| ())`，成功元组及其 bar 在该表达式结束前丢弃，不移交给 wrapper 调用者；`:2213` 当前条目按上述位置显式释放；`:2242` memo miss 保持 `let (coord, _) = ...?;`，`_` 不绑定，返回元组内的 bar 在该语句结束时析构，随后才 `memo.save(previous, coord)`，没有改成会延长持有期的 `_bar`。`RefValue::Plain` 的 `NativeValue<ContentRef>` 持有 `Option<NativeReservation>`，`native_owned.rs:50` 起的字段声明顺序使载荷先于许可析构；本轮只缩短持有期，不改释放机制。

实际检查：

- `rustfmt --edition 2021 --config skip_children=true --check rust/src/bin/s_session_v3/source_audit.rs` 退出 0，仅验证格式。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check` 退出 0，报告 `.github/prod-banned-literals.txt 与源码一致`，未修改或重建清单。
- 完整阅读 anchor_lock 脚本后执行 `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py`，退出 0，3 项通过；该检查不调用 cargo/rustc。
- Python 对拍 HEAD 与工作树，确认 `source_audit.rs` 仅有上述比较与显式释放替换；其他源码逐字保留，包括 memo 分支、wrapper、无 memo 前缀末项免审、循环后末项校验、冻结数值与摘要。追加本节前，原笔记与 HEAD 逐字相同。
- `git diff --check` 退出 0；未运行 cargo/rustc，未暂存、提交或回退改动。

本阶段 ready。Rust 未验证，待 CI。冻结数值与摘要沿用 M4 现状，由下一阶段依据 CI 实测更新。


## M6

本节绑定 #1419，开工分支 `sandcastle/issue-1419`，HEAD `653b051ee64935d66ba653bc19461c841dee4442`，工作区干净。已完整读取本文件原有 1332 行，包括 M3 四锁设计、M4/M4B、M5/M5B。按用户授权比较 W2 `ab2bfac7`、H1 `bea93e19` 与当前 HEAD，读取 `/Volumes/AgentStorage/w2-h1-integrate46x/ci-dde4cd4b/owned-48g.log` 的失败原文。原日志对应 `de4fd36a`，早于 M5，不能直接把旧失败算作当前 HEAD 的运行收据。

本阶段 **status=blocked，仅第 3 项需要新设计**。第 1/2 项共用生产物理写时序修复，第 4 项修测试准备额度，第 5 项按 S1R 方法拆帧；全部交付待 CI。第 3 项已定位，现役生成状态机缺少历史初始化版本，单独屏蔽非法 kind 会制造后续失败，故未提交半修。阻断不妨碍其余改动交付。未运行 cargo/rustc，未暂存、提交或回退任何改动。

### 第 1 项：逻辑 effects 已覆盖，pager 物理写尚未闭合

原文 `owned-48g.log:2081-2082/:2098-2099/:2117-2118`：三条 `w2_h1_cold_bounded2_shape_development_window_matches_online`、`w2_h1_cold_cd_second_cursor_continuation_matches_online`、`w2_h1_cold_ef_seed_pair_qualification_and_canonical_matches_online` 均在 `w2_h1_structure_tests.rs:292` 的在线 `online.handle(envelope).unwrap()` 报 `HistoricalAuditStale：effects闭合后出现未声明变化`。stdout 最后一条候选后缀日志分别为 g6、g6、g7；该日志先于最终 COMMIT，不能据此声称这些代已经提交，也不能把失败归为还未开始的冷恢复。

第一处出错的边界在 `audit_owner.rs:4508-4534`：原实现先 `Recording::finish` 结束逻辑效果，再 `Clamp::candidate` 拍物理快照，随后才 `validate_effects`。后者 `audit_owner.rs:4039-4059` 经 `history.rs:8958 append_owner_cut` 读取历史内容、构造并核验本代后缀；`history_clamp.rs:231-254` 的 `observe` 本身也会读 domain 与两个控制表。逻辑 SELECT 并不保证 SQLite pager 不写文件。

物理写的来源为 **同一个 owner 连接、同一个未提交 Publish/固定写事务此前写脏的页**。`s_session_v2/content_store.rs:995-1001` 的 `TxStore::put_before_write_inner` 已执行 `INSERT INTO s_content_nodes(digest,tag,body)`，紧接着 `:1005` 已交 `note_persisted_node`，逻辑新增行处于原 Node journal/effects 覆盖内；相关内容表及索引脏页不必在 execute 返回时落盘。后续扫描旧页，在页缓存压力下把尚未落盘的可驱逐脏页写成 WAL 的未提交 frame。它不是闭合后新发出的 INSERT，也不是另一个连接或 audit ledger sidecar 写入。

已对照 Cargo.lock 锁定的 `libsqlite3-sys 0.38.2/sqlite3/sqlite3.c`：`getPageNormal:65140/65160` 的读页 miss 可进入 `sqlite3PcacheFetchStress:57161`，该函数挑未引用脏页调用 `pagerStress:64214`；WAL 分支 `:64250-64254` 调 `pagerWalFrames(...,0,0)`。该路径不增加 SQL total_changes，却改变 WAL 长度/mtime/ctime。HEAD 的 `history_clamp.rs:353` 会因此正确地拒绝新旧候选观察值不同。原 CI 未记录具体被驱逐的页号/表页归属，不能把某一个 WAL frame 伪报为 CI 已确认的 s_content_nodes 页；源码定位到节点/索引写脏与后续 pager 写帧的边界，已足以修复遗漏的物理闭合动作。

W 未定锁两场最多五代，主要保持未定对象/请求，随后成立，未触发旧日志里的较大历史扫描工作集。CD/EF/bounded2 的窗口、分型发展批、段/中心对象由 `generation.rs:908-977/:1275`、`content_structure.rs` 构造，最终仍经同一 TxStore 持久化。失败前累计节点分别为 16002、16529、15589，工作集变大后读旧页更易触发 spill；EF 甚至尚未到最终 center/pair 冷点就失败。因此不能把“这些对象在最终夹具中存在”误当成“失败时它们已全部出生”，也不能给这三类对象另造 effects 例外。W 通过只说明该短夹具未触发同一时序缺陷，不构成另一套准入语义。H1 本身已经有 bounded1 开发批，不能把 CC-007.development_batch 整族写成 W2 才新增；W2 的 bounded2 来源绑定等编码差异与持续历史工作集暴露了同一边界遗漏。

**改法**在 `audit_owner.rs:4510-4517`：只在现有 Unix H1 history_strict 域，对同一 active transaction 执行 `cache_flush().map_err(publish_err)?`，然后才 `recording.finish`。这把可刷出的自身脏页写入提前到 effects 仍活跃且候选尚未拍照的时点。`Recording::finish`、total_changes 覆盖、controls.verify_effects、own_files、observe 的前后 stat、check_candidate 的逐位比较、COMMIT/checkpoint 收据及 data_version 均未改；flush 失败沿原错误清理/回滚/候选失效路径返回，不重新审计或切宽判据。

`rusqlite 0.40.2/src/lib.rs:1070` 已提供该 API，未加依赖。SQLite API 明确只刷未被引用的脏页，跳过 page1 与活跃游标页，不能称“flush 全部脏页”。正常发布帧在 `audit_owner.rs:1641` 返回，pending 于 `:1671` drop 后才进 finish；TxStore 的 execute 已完成并 reset，读取 helper 的 rows 在返回前 reset，不跨此边界保留游标。page1 恒在用，不会成为后续读可驱逐的页。未来若仍有残余物理变化，原比较继续拒绝。

COMMIT 监视器仍由 `commit_as` 在提交前 arm；提前 spill 的未提交 frame 被真正 COMMIT 返回的累计 WAL frames 及 `history_clamp.rs:367-422` 的长度/回填前沿核验覆盖，不需要伪造提交回调或更新 baseline。认证只读 FD/WAL VFS 已在 `admission.certify_cold` 结束时关闭；交接后的 writer 经根 `s_structure_session.rs:426-434` 的普通 PRIVATE_CACHE 连接打开。新 flush 不进入只读认证 VFS，也没有撤销后再借用 SQL 授权。

M5 覆盖：H1 S1R `29326e4d43` 只改栈形状及探针，S1S `ae46a1273b`、S1T `0940e55734` 改夹具/前缀对照；未加入 pager flush。`history_clamp.rs` 与 H1 bea93e19 整文件相同，原 finish→candidate→validate 顺序仍在，故 M5 未覆盖。H1 原分支的小工作集通过不证明这个物理闭合边界完整，W2 原分支没有 H1 Clamp，物理 spill 不会触发该新比较；修复同时保留 W2 真实持久化与 H1 严格观测。

### 物理写原型及压力锁

本机实际运行 Python 自带 SQLite 3.53.4 的 C API 小原型，不构建或执行 Rust。旧 history 512 页、cache_size=32、写入 25 个新节点页后，仅 SELECT 使 WAL 从 0 变 4152 字节；30 页时从 16512 变 24752 字节；两场 SQL total_changes、main stat 均不变。闭合前 `sqlite3_db_cacheflush` 后，1/5/10/15/20/25/30/40/50 页九场后续 SELECT 的 WAL stat 均不变。主控独立重跑退出0。原型证明机制，不能冒充当前 Rust 三锁已转绿，也不能从随机页原型推出旧 CI 的具体页号。

新增 `history_clamp_tests.rs:44 h1_effects_close_flushes_pager_before_observation_and_suffix_reads`：对 8/32/128 页三档各自建立独立真实 fixture/owner，开启该测试连接的 cache_spill，跑原 H1 Accept/Begin/Publish 两代，核完整冷输出、owner 未退休及 SQLITE_DBSTATUS_CACHE_SPILL 正增长。只改测试连接建议缓存，不改生产缓存配置或 Native 父帽。`.github/workflows/ci.yml:485` 加入 owned 的逐名成功收据，原 h1_ 执行过滤会实际运行它，原有名单全保留。

此新增锁是压力锁，未证明删除 flush 后一定变红；可能有些缓存配置在写阶段已 spill。已有 M3 三条真实失败锁才是原症状回归判据，CI 须同时跑 W 未定对照与全部外写/main/WAL/commit-hook 负控，不能用新增压力锁代替它们。

#799/#804 核对：只移动同一连接的物理写时点，同一严格判定与级别输入均不变，没有比较删除、失败兜底、第二套宽松证书或效果豁免。

### M6 第 2、4、5 项：冷恢复边界、真实预算来源锁与测试栈帧

本组完整阅读 M2-NOTES（含 M3 冷往返设计、M4/M4B 与 M5/S1R/S1S/S1T），对照 W2 `ab2bfac7`、H1 `bea93e19`、HEAD `653b051ee6`。只编辑 `rust/src/bin/s_session_v3/history.rs` 与 `rust/src/bin/s_session_v3/driver.rs`；第 2 项与 effects 修复组协调，不编辑其生产文件。未运行 cargo/rustc，未暂存、提交或回退改动。

#### 第 2 项：M5 没有覆盖锁内文件变化

原文 `owned-48g.log:2051-2052`：`h1_admission_driver_cold_recover_rebuilds_ledger_bitwise` 在旧 `admission_tests.rs:701` 报 `HistoricalAuditStale：锁内观测期间文件变化`。stdout 已完成 g1..g5 的真实提交；失败点不是此前冷重建台账或正式 recover，而是换 epoch 后重开 Driver、自动续接 pending 第六代：旧 `:701` 对应当前 `admission_tests.rs:718` 的 `driver.unwrap()`，其上一行 `state::traced(... h1_admission_driver(&f, "2"))` 覆盖冷认证及自动续写。

M5 覆盖证据须分开写：

- H1 S1R `29326e4d43a202a519a5dd7cbb1ce646b9ed17e1` 只将 `h1_admission_driver` 返回值改为 Box（当前 `admission_tests.rs:512-528`）、将 cold helper 返回值改为 Box（`:567-597`）、增加薄测试入口/独立 body（`:601-612`）及水位探针。`git diff 42a3f959 bea93e19 -- admission_tests.rs` 只有这三组栈形状变化，无观测/effects/SQLite 写入改动。
- H1 S1S `ae46a1273b6bb3babfcf958a3f2d970c83784280` 根本不改 admission_tests.rs；其 driver/prepared 夹具改动没有覆盖这里的 observe/effects 边界。S1T `0940e55734` 仅改 driver identity/schema 对照和报告，同样不覆盖。
- Python 逐字确认 HEAD admission_tests.rs 整文件与 H1 bea93e19 相同。因此只能说 M5 保留 S1R 栈修复，不能以 S1S 已合入声称本 stale 已修。

第一处显式失败为 `history_clamp.rs:231-254`：先取 main/WAL stat，随后读 domain/controls，末尾再取 stat并比较。与 effects 组协调的根因是同一 owner 写事务的脏页在后续读旧页时被 SQLite cache spill 写到 WAL；这不是另一个逻辑 SQL 写、不是台账 sidecar 混入监视范围。pending 第六代发布已经有脏内容页，控制读扫描足以把它们淘汰，因而锁内 stat 改变。生产修复与第 1 项共用：在记录中的自身效果仍未闭合时使同连接脏页落盘，然后才拍候选/观测快照。该判断以源码和父组 Python SQLite 探针为证，不把当前 Rust 路径称作已复现通过。最终生产行号及 probe 结果由 effects 组填入其小节；本组没有改比较、断言或控制门。

CI 需执行此原名锁，重点必须到达 g6 自动续写、资格事件 `[1,2,3,4,5,6]`、最终 g6 台账 bitwise 对拍与再冷开；无 full_audit 次数增加断言继续保留。

#### 第 4 项：8 MiB 是测试预演的人工额度，不是生产资源合同

原文 `owned-48g.log:1302-1304`：旧 `history.rs:7421` 的 `unwrap()` 收到 `ContextError { message: "PublicationInvalid：prototype_capture_owned_payload_limit: codec 工作域分配失败", proof: Some(BridgeProof { origin: FailureOrigin { capture: 113, born: 2 }, bridged: 3, failure: Budget }) }`。该组实际 `79 passed; 1 failed; 1 ignored`。

第一处错误在测试准备段：第一个 `resolve_native_dual_templates_pending_view` 原本应成功，再用 `scope.reserve_native(budget.remaining())` 压满余额，第二次同消费者才是本锁要证明的真实 Budget 失败。CI 停在第一次成功预演解包，尚未创建 ballast，故它没有测试到原来的 BridgeProof/最终 publication 判决断言。

源码预算分项不是只有新对象载荷：

- `history.rs:7401-7425` 中同一个 Native 父账先持 codec 元数据、aux index、history objects、manifest/roots 和 NativeCutProjection 数组；这些对象跨预演继续存活。
- `s_session_v2/content_publication.rs:468-565` 的真实双模板消费者依次核 scope、核两棵 Template 原件闭包、取字段、分配 values、构造模板 overlay，再替换 `index_frontier` 和 `catalog_evidence.batch_id`。`audit_store.rs:304-391` 在 work scope 下通过原始 capture 每次核验真实帧，复制 body 前 `reserve_native`；字段/数组/模板编码用同 scope 的 Layout 分配，`history.rs:4070` 起 NodeSink 将结果留在 overlay。
- `native.rs:1565-1609` 对 Native、Layout、Metadata 统一先扣 remaining 再核 total_limit；不能给各分项另开额度。CI 的 `failure=Budget` 与这条额度拒绝一致；现有日志没有打印内部具体第几次分配或各分项峰值，本组不编造精确分项字节或声称已知失败在 resolver 中哪一条 reserve。

反证默认“集成引入生产分配超原合同”：该锁由 W2 `4207f5f7fd` 首次加入，H1 bea93e19 中完全没有此测试。Python 提取完整函数，证明 W2 ab2bfac7 与 HEAD 的目标测试逐字相同；两侧对应 Native Budget 和 scratch 核未变（native.rs 差异只有只读 Permit::bytes）；native_cut_projection/dual resolver 也未增加分配语义。Fixture 建库、accept_begin 单行输入未被 H1 改变。故没有证据把这项改判为 H1 引入的生产额外分配；“W2 链其他测试通过”不能证明这条新锁真实跑过。

改法仅 `history.rs:7397-7401`：将本测试人工父账从 8 MiB 调至 64 MiB；依据同文件 `r9a_new_cut_manifest_consumes_scoped_sql_source_and_publication`（`:7027`）及 `aux_index.rs:2793` 的 `r12_cold_object_aux_shared_scope_matches_complete_plain_outputs`，两者均以 64 MiB 运行真实 Native 索引/投影/发布受管工作域。64 MiB 是同类测试的准备额度，没有改任一生产资源记录、配置或帽。成功预演是保证到达锁目标；额度大小本来不是该测试断言。第二次调用前仍精确占满全部 remaining，不靠一个固定 ballast 数字制造失败，因而增大准备额度并未放宽目标失败条件。

Python 验证当前函数去掉新增注释、将64恢复8后与 HEAD 完整逐字相同：两次真实调用、同一 scope、ballast、returned 克隆、only_resource、WorkError::Resource(Budget)、BridgeProof 消费及最终 publication_degrade_verdict 的全部原断言保留。S1R/S1S/S1T 不改该锁，所以 M5 没覆盖。是否 64 MiB 足以完成当前集成候选的预演须由 CI 实测；本机检查不代替它。

CI 需先完成真实索引/双投影与首个 resolver，再证明 ballast 后第二个 resolver 返回 Budget、trace 只有资源错误且最终 publication 判决接受该真实来源，不能只看函数返回或组计数。

#### 第 5 项：未被 S1R 覆盖的 Driver 返回大帧

原文 `owned-48g.log:2205-2207`：`audit_owner_incremental_normal_writes_full_oracle_and_suffix` 在线程 5700 栈溢出，随后进程 SIGABRT。日志没有完整回溯，本组不声称定位到运行中的某个最低层栈帧。

源码第一处可修的大帧是目标测试自身：旧 `driver.rs:8149` 的长函数同时分配 Native Driver、Ordinary Driver，并保留 epoch2 reopen 的按值 Driver 返回槽；带这些返回槽的测试帧再进入真实 handle/traced/codec 深调用。W2 ab2bfac7 原测试、H1 bea93e19 原测试及 HEAD 都保留三处按值开库。H1 S1R `29326e4d` 的已验证修法是 Box + `#[inline(never)]` helper + 薄测试入口，其适用位置是 admission cold 测试；该提交对 driver 的改动仅 recover 水位探针。另两处 driver 局部开库的 Box 化属于 S1S `ae46a127`（native_delta 与 native_bootstrap 测试）。这两提交均未改到本函数；S1T同样没有 Box 化本锁。因此 M5 不能消除本锁的大帧风险。

改 `driver.rs:8149-8178`：原测试名保留，测试入口只调用不可内联独立 body；body 内两个不可内联开库 helper 分别沿原正式 Native 和 Ordinary 入口生成 Box；主体 `:8272/:8273/:8461` 的 Native 首开、Ordinary 首开、Native epoch2 reopen 全部只保留 Box 指针。原资源值 `536_870_912`、模式、epoch、profile/clock/anchor、请求顺序、台账分区、source keeper 交接、对象/业务表/回执/事件和全部断言保持。没有增加线程或调栈大小，未改生产 Driver 构造或业务代码。

Python 将三个开库表达式还原并去掉新 helper 后，证明独立 body 与 HEAD 原函数逐字相同；故没有删比较/断言、绕过 source/epoch 门或删场景。借用调用依靠 Box 的 DerefMut，与 H1 S1R 同一方法。没有实际栈水位测量，CI需在默认测试线程栈下跑原名锁，完成 normal append、Accept R>F、Begin、recover、重试、revision、repost、historical insert 及释放后 reopen 全流程，随后 delta 组剩余三条过期 receipt 锁必须继续执行，确认不再 abort。

#### #799/#804 与实际检查

本组只改变测试持有位置和准备预算；生产仍跑完整 W2 规则串，未加 test_writer_rule，没有同一判断宽严两档，没有判据失败后切另一套判据，亦未增加级别差异。保留冷/在线、Native/Ordinary 的原对象与结果对拍；第2项生产效果边界修复由 effects 组按同判定同效果提交。

实际检查：

- `rustfmt --edition 2021 --config skip_children=true --check rust/src/bin/s_session_v3/history.rs rust/src/bin/s_session_v3/driver.rs`：退出0，仅格式检查。
- `git diff --check --` 上述两文件：退出0。
- Python 完整函数对拍：第4除注释/人工预算逐字相同；第5除三处开库槽/薄入口逐字相同；W2/H1/HEAD覆盖判定与 admission_tests 整文件一致性检查通过。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check`：退出0，报告与源码一致，未改清单或生成器。
- 完整读 anchor_lock 脚本，确认不运行 cargo/rustc；`PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py`：3项通过。
- Rust 未验证，待 CI。上述格式/Python检查不证明编译、运行、真实峰值或栈用量。


### M6 第 3 项：非 W 历史夹具误产 W 事实，设计阻断（未修改源码）

原文：`h1_certified_online_consumes_source_receipt_and_matches_cold` 在 `history_clamp_tests.rs:382`、`h1_qualified_cold_rejects_identity_rule_root_and_unit_mismatch` 在 `:942` 报 `SourceAuditInvalid：未声明typed事实种类`。两处均在初始 `Fixture::publish(...).unwrap()`，还未进入测试所要验证的 certified online / 错资格拒绝分支；规则分别为 V3 与 V3+bounded-layout/1，已有 with_test_writer_rule 保持原样。

第一处错误是 `generation.rs:1076–1085`：无规则门调用 `producer.build_initialization`；`content_producer.rs:57–58` 的新增种类为 `CC-004.initialization` 与 `CC-004.undetermined_raw`，`:1552–1562` 总是生成前者，`:1585–1624` 为等待原件生成后者。两测试的首代只有一根 raw，现役 parser 处于等待期，所以二者都存在。`generation.rs:1275` 后 `:1292` 与 `:1606–1617` 按 (kind,fact_key,id) 排序，initialization 在 undetermined_raw 前；首个被拒的 kind 因而为 **CC-004.initialization**，后者也不合法。这是源码顺序推导，原 CI 错误只显示通用文案，未输出 kind，本机未运行 Rust。

`source_audit.rs:2329–2337` 只在现有 `source_pending::is_w_rule(rule)` 成立时接受二者；该核在 `source_pending.rs:424–426` 直接使用 `crate::w_revision(rule).is_some()`，V3 两历史规则均为 false。因此白名单正确；没有合并丢项。比较 `git show ab2bfac7:...generation.rs` 与 HEAD，W 初始化段均无规则门；H1 `bea93e19` 的 `generation.rs:1027–1068` 没有初始化对象段，直接遍历全部 state.step_count；H1 source_audit:1922–1932 也只列旧通用/新笔种类。问题为 W2 新写生成核与 H1 历史规则测试组合的不兼容，不是 H1 白名单漏并。

**不能只补一行 is_w_rule 门**，否则仍会在后续原检查拒绝，原因已逐条核实：

1. `inclusion.rs:621` 的现役 IncrFactsState 没有初始化版本参数；`:878–905` 首根记 action=waiting、acc_after=None、groups/merged 为空，`:907` 建方向时从 raw0 重折。H1 同文件 `bea93e19:767–790` 首根立即记 seed 且创建一组。`generation.rs:1120` 现役等待期不发步骤；即使改为发步骤，`source_pending.rs:455–459` 仍严格要求首步为 seed 且 acc_after=incoming。建立后现役 seed 带初始化方向证据，非 W 核 `source_pending.rs:474–479` 又明确要求 direction/evidence 为 Null。故门控后会变成 CC004 覆盖或 seed 首错，无法满足两侧。
2. `content_producer.rs:3137–3179` 的 group_payload_value 无条件增加 initialization_policy_revision 和 initial_direction_evidence，传 None 也仍多这两字段，不能恢复 H1 旧载荷。该函数有 generation 与 producer 内部 build_group 两个调用路径，不能只改单一调用者。
3. `content_producer.rs:1673–1695` 的 build_knowledge_state 无条件增加 W 初始化字段且写 W_CONTRACT.knowledge_domain=`raw_adjacent_W_initialization`。`tb02b.rs:24–26/:35–48` 已登记 V3 绑定 CURRENT_CONTRACT 的旧知识域=`established_direction_with_extreme_root_sets`；`source_audit.rs:2435–2440` 仍按具体规则合同严格比较，正确地会拒绝。
4. `generation.rs:1078–1080` 的 read_w_prior 与 `content_bi.rs:984` 的 read_prior_undetermined 只读 W 未定原件；恢复非 W 构造时必须同门排除，不能让历史规则读/认证 W prior。C/D/E/F 已由 `structure_enabled` 门隔离，不是此次首错来源。

**现成入口穷举**：检索 IncrFactsState 全部构造/公开方法，现役只有 empty/default 与同一 append_with_changes，无历史初始化模式；Lane/DeltaLane 均使用该状态。`process_inclusion_with_facts_flat` 的 `inclusion.rs:575–582` 注释明确旧初始化已删除，当前调用同一 W 核。`generation.rs:7025` 的 A7 `layout1_historical_draft_nodes_remain_readable` 只从归档 nodes 读旧原件，注释明确“候选不再等于这些根”；没有历史续写生成入口。`git show bea93e19` 可以取回旧实现，但不是当前已存在的可接入核。

**所需最小设计面**：显式历史初始化版本作为规则值输入同一状态机；该值须贯穿 Lane、DeltaLane、cold_seed/冷恢复绑定，保证初始化版本不能跨缓存/资格复用。再按同一个合同门控 W 初始化对象/prior/请求、恢复历史 seed/waiting 与来源前缀、按版本恢复 group 与 knowledge 字段/知识域；需要同输入版本化全量/增量/冷恢复对拍。旧内核在 git 历史可取回，但恢复状态机版本及冷恢复身份是新设计，不能靠只包对象产出或拷贝一份测试专属宽松核完成。依用户“需要新设计则 blocked”规定，本项 **blocked，未改源码**，也不把首错诊断称作修复。

M5 覆盖核查：`git log 42a3f959..bea93e19` 只有 `29326e4d43`、`ae46a1273b`、`0940e55734`、`bea93e1919`；对 generation/source_audit/content_producer/inclusion 运行同范围路径 log 无提交。这四文件在 H1 S1R/S1S/S1T 中均未改，不能宣称 M5 已覆盖。HEAD 的 source_audit 除既有 M4/M4B 读数/持有期修复仍保留上述严格白名单。

#799/#804：当前白名单、prefix/seed、知识域核都应保留；不得按首次失败切 W 或扩白名单。将来采用按冻结规则值分派初始化语义本身是已登记历史合同的版本输入，不是严格失败后的宽松兜底；但须把全量/增量/冷链统一输入并以对拍证明，不能在 generation 临时复制第二套宽松判据。

CI 待验证：两原名历史锁保持原断言与原规则；设计完成后分别在 V3、V3+bounded-layout/1 验完整来源/在线/冷资格链；W2 四冷锁及 W 初始化/请求语义锁需保持完整生产 W2 串。现阶段两历史锁仍预期失败。本机未运行 cargo/rustc；本子项无源码改动。


### M6 独立复核、最终实际检查与交棒

非作者分别只读复核 effects/物理写时序与夹具/栈帧改动，无具体打回项。复核指出并已在本文保留两项限制：cache_flush 不保证刷出在用页；新压力锁未做 Rust 先红后绿证明。第二轮按 Cargo.lock 的 0.38.2/0.40.2 依赖及最终三档压力锁、owned 逐名清单复核，结论不变。第 4 项 64 MiB 只是现有同类测试准备额度先例，未声称实测峰值或已足够；第 5 项未声称已测栈水位。

主控实际检查：

- 四个 Rust 改动文件执行 `rustfmt --edition 2021 --config skip_children=true --check`，退出0。仅格式检查。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check` 退出0，报告 `.github/prod-banned-literals.txt 与源码一致`。未改/重建清单或生成器。
- 完整阅读 anchor_lock 脚本，执行 `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py`，3项通过，不调用 cargo/rustc。
- Python 核对所有旧具名测试保留；audit_owner/history_clamp_tests 的测试数 34→35，其余三文件数量不变。Clamp 整文件与开工 HEAD 逐字相同；新增压力锁在 owned 清单唯一。夹具组还原对拍确认预算之外原锁正文、三处开库之外原测试主体逐字保留。
- 本笔记开工1332行前缀的 SHA-256 为 `1d994cc335b18af960bfe8f0c04055b5aa86fea017d9a489ea6467b2051a9778`，追加 M6 前逐字核同 HEAD，原记录不追溯改写。
- `git diff --check` 退出0，暂存区为空，HEAD 未变。未运行 cargo/rustc、Rust 构建/测试或会间接调用它们的脚本。

生产写者仍保持完整 W2 串：

```text
s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1+raw-initialization-w/2+bounded-layout/2
```

未新增 test_writer_rule，现有两条历史规则锁中的原用法保持；未改准入语义、资源帽、冻结原件或教义判据。第 3 项在完成显式历史初始化版本及冷恢复绑定设计前仍会失败，不能把本次交付称作 owned 全绿。其余项先交 CI 验证：三条 M3 原失败锁及 W 未定对照、恢复 pending g6 台账逐位锁、真实预算来源锁、默认线程栈上的 delta 原名锁与 abort 后剩余锁，另跑 H1 物理篡改全部负控。Rust 未验证，待 CI。

本工位全部改动文件：

- `.github/workflows/ci.yml`
- `rust/src/bin/s_session_v3/audit_owner.rs`
- `rust/src/bin/s_session_v3/audit_owner/history_clamp_tests.rs`
- `rust/src/bin/s_session_v3/driver.rs`
- `rust/src/bin/s_session_v3/history.rs`
- `.chanlun/review-results/issue1419-w2h1-integrate46x/M2-NOTES.md`


### M6 封包口径

按 #1419 集成 owner 46X 裁定，第 1、2、4、5 项已完成并可交 CI，本封包以 **ready** 交付。第 3 项诊断清楚但需裁定，登记为**待裁定开放项**，不在本封包内解决；分析见本节「M6 第 3 项：非 W 历史夹具误产 W 事实，设计阻断（未修改源码）」。两条 H1 历史规则锁 `h1_certified_online_consumes_source_receipt_and_matches_cold`（V3）与 `h1_qualified_cold_rejects_identity_rule_root_and_unit_mismatch`（V3+bounded-layout/1）在当前 head 的写者下仍预期因 W 初始化事实被拒而失败，不能把本封包称为 owned 全绿；其余项待 CI 验证。Rust 未验证，待 CI。


### M6C

本轮按 #1419 主控裁定 A 处理 M6 第 3 项。开工 HEAD 为 `48ffac48c0b17317575fe0377f46f943716484cc`，工作区干净；先读上述诊断和封包口径，再由夹具实施、合同追溯、非作者只读评审分别核对。结论为 **ready**，只改两条测试及本节笔记，Rust 未验证，待 CI。上节是当时封包记录，本节记录后续裁定及实施，不追溯改写旧记录。

#### 原 H1 合同对照

实际分别执行 `git log -S h1_certified_online_consumes_source_receipt_and_matches_cold bea93e19 -- rust` 和 `git log -S h1_qualified_cold_rejects_identity_rule_root_and_unit_mismatch bea93e19 -- rust`，读来源提交正文与测试原文；另用 `env -u GH_TOKEN -u GITHUB_TOKEN gh` 读取 #1481、#1647、#1816 及 PR #1809 的正文、评论，并追查 history36h 前身提交。

- online 锁来源为 `38256274f3785c1117d89501c3e62276921c5c38`（#1809），提交名为 `feat: land verified H1 cold audit and P6 raw ledger components`。PR #1809 的组件范围原文为「H1 真实冷认证入口、控制载荷同父账、raw 后缀直接读取」；[#1481 评论 6533718168](https://github.com/xy7365527-lang/NewChanlun/issues/1481#issuecomment-6533718168) 原文为「覆盖U-A生涯夹持、历史cert语义认证和同核冷/在线对拍」。
- 继续沿 `origin/codex/issue1481-closure38h` 用 `git log -L` 追前身：`d10c7aac1ef3932df854a73484c6133afe1fcce5`「接入 H1 认证在线的同请求来源资格」首次加入测试时即有 V3 与 V3+bounded-layout/1 的动态建库、在线发布循环；`f02bdb4b2e8597f04341b778cd1fe464bd987009`「建立 H1 从原件冷认证的显式组件入口」将人工安装 certificate 换成真实 cold open；`ef71183a304f2658c3a6f84f3c4e56d91011c990` 增加真实 raw 后缀读量断言，`b39a2dcb246b2863a5902dd28739affc3d75265b` 只格式化。两规则循环并非为历史只读兼容另加，而是当时合法写者的在线/冷测试矩阵。
- qualified 锁来源为 `71df2ae749664714e6bff10f8f71d9e15cc4c815`（#1816），提交说明原文为 `Deliver H1 qualification consumption with persisted-rule checks and write-before-budget admission.`。#1816 正文明确「缺资格、部分账、规则/内容库/二进制/verifier 不符、cert 四根或 node 行被外写、台账 COMMIT 失败均拒绝并 stale」。原作者 `/Volumes/AgentStorage/issue1481-consume39p/REPORT.md:31` 将本锁列作「bounded-layout 规则库消费 V3 台账」，`:38` 说明 `QualifiedSource` 不带 `rule_revision`，规则从库 `meta.rule_revision` 读，调用方声明不是事实。因此必须保留的是资格与候选实际规则的绑定及各类拒绝，不能只留一个任意 `is_err`。

逐条对照：online 原锁核 g1 冷认证事件、Accept/Publish 两阶段 raw 后缀、每阶段完整历史、在线零完整来源重审、代号与认证事件、snapshot/delta、全部 runtime 根表、owner 不退休。这些断言全部保留。qualified 原锁的 content identity、binary identity、规则、末代四根、根单位五个场景及无 slot、父账归还、代号不变均保留；规则场景改为下述单字段候选篡改。

裁定范围内的合同结论：未发现这两条具名锁另承担永久的「退役 V3 / bounded-layout/1 冻结库只读兼容」义务，故不以恢复历史 writer 或新增历史锁作为本阶段前置。此结论只针对两锁，不撤销 #1481 的 bounded-layout/1 重型验收、#1647 的旧新二进制回退验收或读取核的历史兼容合同。[#1647 评论 6534406162](https://github.com/xy7365527-lang/NewChanlun/issues/1647#issuecomment-6534406162) 所述「旧 rule 下的 seed 组与 pending、新 rule 下的未定原件，均按各自已批准原件和判据认证，不借台账转换初始化政策」仍成立。

只读资产也已核查：`generation.rs:6961` 的 `issue1419_history()` 解包并校验 62 项 SHA；`rust/tests/fixtures/issue1419_history_5123/export.tar.gz` 内有 `historical-published-layout1.sqlite`，Python 只读核出 `s-session/3`、generation=4、规则为 V3，主库 SHA-256 为 `a8c49718a6ab2949c309dc7489de88dd6faab88126c7658735041269a4bc77b6`，与冻结清单一致。未找到 bounded-layout/1 的完整冻结库；归档 JSON 的规则只有 `r` 与 V3。现有 `generation.rs:7025 layout1_historical_draft_nodes_remain_readable` 保留逐节点/规范渲染，`q_runtime.rs:1143 historical_layout1_published_session_asknown_q` 保留同一冻结库四代 AsKnown 对拍。本轮未动这些覆盖，也不将其称为 H1 完整冷认证已执行。本轮不新增只读覆盖的依据是前述合同追溯，不是夹具是否可用。

#### 夹具与期望值变化

`audit_owner/history_clamp_tests.rs:414` 的 online 锁改用两个 `Fixture::with_clock_events(6, 6)`。原 `Fixture::with_rule` 固定保留六代、六个时钟事件，新入口保持同样参数，经 `prepared.rs:2024/2080` 的默认建库路径调用 `init_v3`；`mod.rs:605` 写入完整 `W_STRUCTURE_RULE_REVISION`。两库均直接读 `meta.rule_revision` 并断言该常量，不传测试写者规则，不手改 meta。原 ordinary 与 certified online 两侧、初次冷认证和后续全量冷对拍全部保留。

`audit_owner/history_clamp_tests.rs:945` 的 qualified 锁首库原本就是 `Fixture::new()`，本轮增加 W2 meta 断言；删除仅为规则负控而新写的 bounded-layout/1 第二库及其写者注入。两测试内均无 `Fixture::with_rule` 或 `with_test_writer_rule` 调用。共用 Fixture 内部的 `with_test_writer_rule(None, ...)` 只清空注入并走生产默认，不恢复旧写者，未修改该 helper。

变化只有合法写入规则矩阵收为生产完整 W2、补规则断言，以及规则负控的构造方式。原测试没有硬编码的对象数或具体根字面量，本轮未修改此类期望；W2 新事实及根由真实写者产出，仍逐次与 ordinary 和完整 cold oracle 比较 snapshot、delta、根表和全部 canonical。原六代输入、修订/撤回、保留代数、时钟、资源值与错误期望全部保留。

#### 四类负控的实际检查位置

| 负控 | 构造与断言 | 生产检查位置 |
|---|---|---|
| identity | 只将 `QualifiedSource.content_identity` 改为 other-content，另一次只改 binary_commit 为 other-binary；保持真实 W2 库与 runtime，原 cold open 拒绝闭包继续核具体错误、无 slot、父账回到基线 | `state.rs:1696-1700 QualifiedCertification::read_qualification`，分别由 content_identity / binary_commit 比较拒绝 |
| rule | 先用 W2 成功安装同一真实资格，再对候选安装规则复制完整 W2 串，末字节 `2` 改为 `3`，断言等长且不等；仅该参数变化，再调用现成 `AuditedState::install_cold_qualification` | `state.rs:1811-1823` 调 `QualifiedCertification::fold`，后者 `:1623` 调 `read_qualification`；`:1696` 的 q.rule_revision 比较触发 `:1700`，测试 `history_clamp_tests.rs:1037-1040` 精确断言 `S3StateInvalid：完整资格的规则/内容库/二进制身份与本写者不符` |
| root | 只将真实台账 cert.roots 改为 `[]`，原 cold open 断言「末代四根与完整资格四根不符」，随后恢复原 roots | `state.rs:1751-1752 QualifiedCertification::step`，在根单位点查后逐字比较实际四根与资格四根 |
| unit | roots 恢复后只删 scope 对应 `flavor=value, mode=public` 的一行 node，断言恰删一行；原 cold open 断言「根单位不在已认证单位集」 | `state.rs:1739-1740` 调 `runtime.contains_qualified_unit` 并拒绝 false；底层仍先核资格与已提交台账相符 |

规则负控用的不是第二个非法库或版本选择器入口，而是生产冷恢复也使用的资格安装核，直接隔离原 H1 规则比较。相同长度使 `Qualification::retained_bound` 和预留上界与 W2 正控相同，不依赖四根缓冲余量，不会因短规则导致资格读取先触发 cap。测试前后完整比较 `runtime.audit_qualification()` 的全部字段及候选 `h1_canonical()`、前沿 Qualification，保证 through、roots、fold_digest、verifier、真实规则、内容/二进制身份和所有 cuts 未变；父账 live 回到基线、无 slot，再用 W2 安装成功并完整冷对拍。精确错误断言排除了 identity、root、schema、版本选择器、准入或预算先拒而造成的伪覆盖。后一个 W2 正控是测试独立调用，不是生产失败后的兜底通道。

#### 独审与实际验证

非作者只读评审核对两锁起源、票面、当前源码及调用链；未发现打回项。额外指出本轮不得宣称全项目历史兼容义务撤销，已在本节明确保留。源文件除两条授权函数外与开工 HEAD 逐字相同；online 主循环逐 token 保留，qualified 的原 reject 闭包、两个 identity 调用及 root/unit 尾段逐字保留。两原名不变，当前 `ci.yml` owned 通过 `h1_` 过滤运行全部 H1 release 锁，两名字未列入其额外逐名清单，本轮不改 CI。

实际检查：

- `rustfmt --edition 2021 --config skip_children=true --check rust/src/bin/s_session_v3/audit_owner/history_clamp_tests.rs`：退出 0，仅格式检查。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check`：退出 0，清单与源码一致；生成器、白名单及清单均未改。
- 完整阅读 anchor_lock 脚本后执行 `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py`：3 项通过，不调用 cargo/rustc。
- `git diff --check` 通过；M2-NOTES 原前缀保留，暂存区为空，HEAD 未变。未 git add/commit，未回退其他代理改动。

没有生产代码、完整 W2 写者串、历史冻结原件、资源帽或 8→64 MiB 准备值变更，没有新增宽严两档或失败兜底。本节解除的是 M6 第 3 项的夹具阻断，不宣称 owned 全绿。CI 须实际运行两条原名锁，证明 W2 六代 online/ordinary/cold 全部对拍，以及规则精确错误和 identity/root/unit 各负控。Rust 未验证，待 CI。

### M6D

本节绑定 #1419 受管阶段 M6D。开工分支 `sandcastle/issue-1419`，HEAD `362eac3474a5e6cbe7f25a5e4ccc7b7d0f060557`，工作区干净。已读 M6 各项、M6 封包口径与 M6C；按主控采纳的独审 §2(a) 订正测试证明力并补预算准备值依据。本节追加记录，前文逐字保留。生产 `cache_flush` 时序、Driver Box 拆帧与 M6C 合同/身份负控修复保持原样。

#### 小缓存锁降为冒烟，订正 SPILL 读数的证明力

M6「物理写原型及压力锁」记载的 `SQLITE_DBSTATUS_CACHE_SPILL` 正增长，不能用来确认缓存压力造成了真实 spill。按主控摘录的独审源码核查，锁定依赖 `libsqlite3-sys 0.38.2/sqlite3/sqlite3.c:64248` 在 `pagerStress` 入口递增 `aStat[PAGER_STAT_SPILL]`；`sqlite3PagerFlush` 在 `:64297-64300` 对未引用脏页调用同一个 `pagerStress`。新增 `cache_flush` 自身就能增加该计数，计数无法区分显式 flush 与缓存压力 spill。因此撤销旧记录中将该读数视作压力 spill 确认的口径；也没有该新增锁删除 flush 后变红的证据。

测试改名为 `h1_small_cache_publish_two_generations_matches_cold_smoke`，删除其局部 `spills` 辅助、前值读取和增长断言。保留建议缓存 8/32/128 页、`cache_spill=true`、每档独立真实夹具/owner、g2/g3 发布、发布回执代号、完整历史与冷 oracle 对拍、owner 不退休等行为断言。注释明确它是小缓存冒烟锁，不作为 flush 回归证据。`.github/workflows/ci.yml:485` owned 逐名收据仅替换此名称，其余名单和执行参数不变。

本缺陷的回归证据归以下四条既有锁的真实红→绿：

- `w2_h1_cold_cd_second_cursor_continuation_matches_online`。
- `w2_h1_cold_ef_seed_pair_qualification_and_canonical_matches_online`。
- `w2_h1_cold_bounded2_shape_development_window_matches_online`。
- `h1_admission_driver_cold_recover_rebuilds_ledger_bitwise`。

四锁及其断言均未修改。本节没有新增 Rust 执行收据；当前候选的红→绿须由 CI 实跑提供，冒烟锁通过不能替代它们。

#### P57 的 8 MiB 成功反证与 V3→W 首代工作集差异

宿主已核实的反证为 P57 提交 `239f8afdd50737648510bca48f09677eb10441de`、workflow「P57 projection origin repair」[run 41041552050](https://github.com/xy7365527-lang/NewChanlun/actions/runs/41041552050)。2026-10-30T10:34:20Z 的日志为 `test v3::history::tests::publication_native_cut_scope_real_budget_preserves_origin ... ok`，执行命令如下，本机未重跑 Rust：

```sh
cargo test --locked --features s_session,s_incremental --bin s_structure_session publication_native_cut_scope_real_budget_preserves_origin
```

本轮用只读 `git show` 核对该提交，`history.rs:6755-6769` 确实是同名测试、相同 `Fixture::new()+accept_begin(1, ("a", 0, 6, 12, 3, 7), 1)` 首代，父账为 `8 * 1024 * 1024`。所以不能推断「该锁从未真实跑过」；M6 旧段关于仅凭其他 W2 测试通过不能证明此锁执行的逻辑限制仍成立，但 P57 这份具名成功收据已补足实际执行证据。

本轮核到的规则与首代差异如下，历史行号均属于 P57 提交，不与当前行号混用：

- P57 `rust/src/bin/s_session_v3/mod.rs:175-191` 的 `require_current_writer_rule` 在 `:188-189` 要求 `V3_RULE_REVISION`，`:618` 的 init 写入该串；`:1835` 的测试明确断言 V3 可写。当前 `mod.rs:175-188` 的同一写者门在 `:185-186` 要求 `W_STRUCTURE_RULE_REVISION`，`:605` 的 init 写入完整 W 串，`:1051` 的测试断言 W 可写。没有更改这些生产规则。
- P57 `rust/src/theta_v0/parser/inclusion.rs:1084-1119` 首根立即建立 merged/group 并记 `seed`；`generation.rs:965-1057` 直接发布 inclusion step/group。当前 `inclusion.rs:870-897` 首根处于 `waiting`，尚未建立方向；当前 `generation.rs:1075-1117` 调 `build_initialization` 产出 W 对象，`:1118-1125` 等待期不发布 inclusion step。两边首代的对象类别因此发生替换和增加，不能把相同 raw 输入当成相同持久化载荷。
- 当前 `rust/src/bin/s_session_v2/content_producer.rs:57-58` 声明 `CC-004.initialization` 与 `CC-004.undetermined_raw`，`:1552-1571` 构造初始化对象及其知识请求，`:1572-1582` 构造等待期 group pending 请求，`:1585-1630` 为每个 pending raw 构造未定对象及其请求。单根首代已包含这两类 W 事实及对应请求。它们不是本轮为预算测试添加的夹具对象。
- 目标测试在同一 scope 下加载 aux/object 索引，再构建 NativeCutProjection。当前 `history.rs:165-199` 读取对象双投影、snapshot/delta 成员并构造投影数组；`s_session_v2/content_publication.rs:477-488` 的消费者核验两棵模板原件闭包并读取 snapshot/delta。W 首代对象/请求带来的索引、模板及双投影工作集扩大，与 M6 日志中的 `failure=Budget` 一致。这是规则与源码解释，未测得 V3/W 的字节差、具体失败 reserve 或最低足够预算，不能据此报精确增量。

64 MiB 继续作为已接受的测试准备额；未恢复为 8 MiB，未改变生产资源合同。ballast 仍用全部 `budget.remaining()`，原两次真实消费者调用、Budget 分类、trace、BridgeProof 及最终 publication 判决断言均保留。

#### 首次成功预演后的既有预算遥测

在 `history.rs:7436-7446`，第一次 `resolve_native_dual_templates_pending_view(...).unwrap()` 成功返回后、`scope.reserve_native(budget.remaining())` 之前，只读现有读数并 `eprintln!`。稳定前缀为 `[budget-telemetry 1419-M6D]`，阶段字段为 `phase=pre-ballast`，所有字节字段单位为 bytes：

| 日志字段 | 现有读数与如实含义 |
|---|---|
| `parent_remaining_bytes` | `Budget::remaining()`，采样时该父账当前域的可用余额 |
| `parent_sampled_occupancy_bytes` | `Budget::live()`，采样时父账的账面占用，包含父账元数据及其管理的 Native/Layout/工作域元数据；不是峰值 |
| `parent_peak_bytes` | `Budget::peak()`，父账自出生至采样点的已有账面占用高水位，包含预演期间已释放的临时占用；不是 RSS/总堆峰值 |
| `scope_layout_sampled_occupancy_bytes` | `WorkScope::snapshot().live`，采样时此工作域的 Layout 计账占用 |
| `scope_layout_peak_bytes` | 同一次 snapshot 的 `peak`，此工作域至采样点的已有 Layout 高水位；不包含经 `reserve_native` 计入父账的 Native 预留或工作域元数据 |

读数依据为 `audit_owner/native.rs:917-933/:960-965` 的 remaining/request_peak/live/peak、`:1565-1609` 的父账扣款与高水位更新，以及 `s_content/capture_allocation/scratch.rs:281-330/:487-493` 的 Layout 计数与 snapshot。`reserve_native` 在同文件 `:205-218` 只向父账计 Native，故 scope 字段明确标 Layout。已有 `request_peak()` 是当前域的 allowance 减余额低水位，本测试 fresh 父账无 handoff，不额外打印重复口径；snapshot 的 `attempts` 是分配计数，不是字节用量，也不打印。

本轮没有使用 `64 MiB - remaining()` 冒称峰值，没有给生产类型加字段或 API，也没有为测量新增 reserve/释放。打印的是预算账本读数，不能推导操作系统内存峰值或各对象类别的独立峰值；本机没有 Rust 实测数值，具体值待 CI。

CI 收据须同时看原名预算锁 `...preserves_origin ... ok` 与 `grep -F '[budget-telemetry 1419-M6D]' <日志>`。现有 owned 的 `publication_` 步没有传 `--show-output`，默认会捕获成功测试输出；因此需在 CI 的靶向取证运行中显式带该参数，保留成功预演读数。本轮 ci.yml 按范围只改冒烟锁收据名称，不扩大其他配置改动。CI 待执行命令如下：

```sh
cargo test --locked --features s_session,s_incremental --bin s_structure_session publication_native_cut_scope_real_budget_preserves_origin -- --show-output
```

#### 实际检查

- `rustfmt --edition 2021 --config skip_children=true --check rust/src/bin/s_session_v3/audit_owner/history_clamp_tests.rs rust/src/bin/s_session_v3/history.rs`：退出 0，仅格式检查。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check`：退出 0，报告 `.github/prod-banned-literals.txt 与源码一致`；未改生成器、清单或白名单。
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py`：退出 0，3 项通过，不调用 cargo/rustc。
- Python `yaml.safe_load` 解析 `.github/workflows/ci.yml` 通过；逐字对拍确认此文件仅替换一个收据名。
- Python 源码对拍确认小缓存循环除两处计数语句外逐字相同，目标函数外的 `history_clamp_tests.rs` 与开工 HEAD 相同，因此 M6C 两锁保持不动；`history.rs` 去掉唯一遥测插入块后与开工 HEAD 逐字相同，64 MiB、ballast 及全部原断言保持。
- Python 核对 `audit_owner.rs`、`driver.rs`、M3 C/D、E/F、bounded2 与恢复锁所在文件和 HEAD 逐字相同；M2-NOTES 原前缀逐字保留。没有生产代码或回归锁修改。
- `git diff --check` 退出 0；工位改动仅本笔记、上述两份测试源码与 ci.yml，暂存区为空，HEAD 未变。未 git add/commit，未回退其他代理改动。

本阶段 **status=ready**，测试证明力与准备值依据已按独审返修。未运行 cargo/rustc，没有当前候选的 Rust 编译/执行收据或遥测实测值；Rust 未验证，待 CI。


### M6E

本节绑定 #1419 受管阶段 M6E。开工分支 `sandcastle/issue-1419`，HEAD `69e73e290903863a75e0741d82e19ded955fef80`，工作区干净。已读 M6D 末段；本阶段只让 owned 组成功测试的捕获输出进入 CI 日志。

`.github/workflows/ci.yml:467` 的 owned 生产增量构建步骤，仅在 `--` 之后、既有 `publication_` 过滤串之后追加 libtest 参数 `--show-output`。过滤串、`--jobs 1`、feature、包装脚本、其余步骤及名单逐字保留。

已完整阅读 `.github/scripts/run-cargo-with-telemetry.sh`。脚本以 `readonly -a command_argv=("$@")` 保存全部参数，再由 `("${command_argv[@]}")` 原样执行，libtest 参数不经改写。脚本不捕获或解析测试输出；判定取命令退出码、资源采样和边界快照状态，磁盘准入解析的是 `df` 输出。增加成功测试输出不会改变这些判定或解析，因此无需修改脚本。

CI 执行后，在 owned 步骤日志中核对原名 `publication_native_cut_scope_real_budget_preserves_origin ... ok`，并执行 `grep -F '[budget-telemetry 1419-M6D]' <日志>`，确认首次成功预演后的既有预算读数可见。此处仅记录待执行取证，未取得遥测实测值。

实际检查：Python `yaml.safe_load` 解析 ci.yml 通过；逐字对拍确认 ci.yml 相对开工 HEAD 仅追加这一处 ` --show-output`，本笔记原前缀逐字保留，包装脚本和所有 Rust 源码未改；`git diff --check` 通过。未运行 cargo/rustc，未 git add/commit。CI 待执行。

### M6F

本节绑定 #1419 受管阶段 M6F，开工 HEAD 为 `b3889d89ec4752697c907b14327769355a798d44`，分支 `sandcastle/issue-1419`。已读 M6C、M6D、M6E。**status=blocked**：源码确实从持久库读取规则，但在本阶段限定的「现役 W2 原件、仅改一个持久字段、不改生产代码」下，两个持久规则字段都先被其他检查拒绝，无法补出命中资格规则比较本身的真实 cold-open 负控；检索后也未找到可替代该绑定负控的既有具名锁。按用户明定的 fallback，只追加本节，所有 Rust 源码及现有内部精确负控、identity/root/unit 负控和正控保持原样。

#### 组件 cold-open 的持久规则调用链

本阶段要求的原拒绝闭包在 `rust/src/bin/s_session_v3/audit_owner/history_clamp_tests.rs:941-960`，调用 `AuditConnection::open_certified_history_component`；其余本节源码行号同属当前 HEAD。

1. `audit_owner.rs:488-500` 的组件入口把 `QualifiedSource` 交给 seeded 核；`:529-530` 开 Deferred 事务并调用 seed，`:1905-1940` 的 `seed_historical_certificate` 经 `HistoricalCertificate::cold_for_request` 传入 `QualifiedSeed`。
2. `state.rs:1877-1884` → `:1910-1934` → `:1960` 的 `cold_captured`。先在 `:2021-2030` 调 `history::fold_certified_history` 完整核验原件，随后才在 `:2076-2086` 用 `CheckedStatement` 读取**内容库** `meta.value WHERE key='rule_revision'`，把借用文本直接传给 `QualifiedCertification::fold(seed, row.text(0), &cuts)`。`QualifiedSource` 的定义（`:1496-1500`）不含规则字段，调用方不能声明替代规则。
3. `state.rs:1594-1623` 的 `fold` 按该持久文本长度持票并构造 `QualifiedIdentity.rule_revision`，调用 `read_qualification`；后者 `:1695` 经 `prepared.rs:398-405` 的 `Runtime::audit_qualification_bounded` 取得完整资格。
4. 资格的规则来自**旁挂台账**：`audit_ledger.rs:605-624` 同一 SELECT 联读末代 `cert` 和台账 `meta`，读取的是 `m.rule_revision`。`audit_ledger.rs:22-42` 的 schema 中，`cert` 没有独立规则列；`Qualification.rule_revision` 由台账 meta 字段产生。该读取先在 `:616-617` 比较持久规则与 Ledger 安装时保存的规则，再在 `:639-643` 核资格字符串总长度上界并复制。
5. 最终 `state.rs:1696-1700` 比较 `q.rule_revision` 与上述内容库规则身份，失败原文为 `S3StateInvalid：完整资格的规则/内容库/二进制身份与本写者不符`；其后的 cut 规则比较在 `:1724-1725`。绑定涉及内容库规则与台账持久规则两端，不是只传写者常量。

#### 单字段构造为何无法命中目标比较

| 仅篡改的持久字段（W2 末字节 `2`→`3`，等长） | 在目标比较之前的拒绝 | 不能算本阶段覆盖的原因 |
|---|---|---|
| 内容库 `meta.value WHERE key='rule_revision'` | `state.rs:2021` 的历史折叠进入 `history.rs:1250` 的 `audit_one_published_cut`；`:1563-1571` 将 manifest 的 `m.rule_revision` 与当前冻结 meta 逐字比较，返回 `manifest.rule_revision 与冻结域不符` | 原件 manifest 仍是 W2，仅改 meta 必然破坏冻结域一致性；该错误早于 `state.rs:2078` 的资格规则读取，更早于 `:1696` 的资格身份比较，不能证明后者的持久绑定。 |
| 台账 `meta.rule_revision WHERE singleton=1` | `audit_ledger.rs:616-617` 返回 `AuditLedgerStale：台账持久规则与安装规则不一致` | 内容库、Ledger 内存安装规则和原件仍为 W2；资格读取在返回 `q` 前已失败，`state.rs:1696` 尚未取得可比较的资格。这里虽也是持久规则比较，但比较的是台账值与安装内存值，无法证明内容库 meta→fold identity→q 规则的绑定。 |

两项阻断都不是规则长度/预算问题，改成等长单字节仍不能越过。同步改 manifest、安装规则或其他字段超出「单字段，其余保持一致」的限定；直接把参数交给内部核则正是 M6C 已有覆盖，不能重复称作真实入口绑定负控。空历史也不能隔离本题：`state.rs:1615-1618` 在 cuts 为空时直接取得 Empty 前沿，不调用 `read_qualification`。

已核此前的开门顺序：`audit_owner.rs:417-418` 经 `s_structure_session.rs:401-425` 的 `open_db` 打开库；`:636-683` 的 `open_historical_readonly` 对篡改后的未知规则返回只读连接，本身没有把它拒绝为 `VersionUnsupported`。本组件只做冷读，不走 `mod.rs:175-188` 的 `require_current_writer_rule`；schema 和版本选择器也不能消除后续 manifest 冻结域检查。本处实质阻断是上表两项，不能归因于写者门。`driver.rs:1625-1629` 既有注释也明确记录非空历史只改 meta 会造出不合法库，不能用 W parser 新造旧串库。

还区分了正式 rebuild 入口：`audit_owner.rs:581` 的 `open_rebuilt_history_into` 在 `:606-626` 清理旧台账缓存；`:636-648` 读取内容库 meta 规则并用同值创建 fresh Ledger，`:654-672` 完整重建后将同一规则传入 `install_cold_qualification`。此路径不会消费旧缓存中被篡改的规则，且不是原 reject 闭包调用的组件入口；其成功重建不能补本题的规则不匹配负控。

#### 既有具名锁检索及边界

实际用 `rg` 搜索整个 `rust/` 中的 `open_certified_history_component`、`install_cold_qualification` 和目标错误原文，并在 S3 测试中搜索包含 `cold`、`qualified`、`qualification`、`rule`、`persist` 的函数名及规则 UPDATE；对命中锁逐一阅读构造与断言：

- `h1_qualified_cold_rejects_identity_rule_root_and_unit_mismatch`，`audit_owner/history_clamp_tests.rs:930`：`:973-1065` 保留 M6C 的内部参数等长单字节负控，`:1022-1025` 精确核目标错误。它绕过持久读取，不能满足本阶段追加要求。
- `h1_qualified_online_consumes_real_ledger_and_cold_rebuild_matches`，同文件 `:829`：`:876-889` 真实 cold-open 三代并比较前沿与完整历史，内容库和台账规则始终同为 W2。这是经过绑定调用链的同值正控，无法区分该链是否错取常量，不能替代持久规则不匹配负控。
- `h1_qualified_stage_rejects_externally_rewritten_fold_digest_or_rule_revision`，同文件 `:1170`：`:1182-1197` 改台账 meta 后发起**在线 publish**，精确断言「上一代资格与台账当前已提交状态不符」；没有重新 cold-open。
- `p6_complete_ledger_rejects_partial_and_wrong_rule_without_commit`，`prepared.rs:3160`：安装其他规则的 Ledger 后调用在线 publish，断言 `AuditLedgerStale：台账请求、规则或代号不一致`，不是 cold-open 资格读取。
- `h1_admission_cold_negative_classes_match_full_oracle`，`audit_owner/admission_tests.rs:271`：六类实际损坏为 node、内容、index、clock、manifest，没有持久规则不匹配；`h1_cert_rejects_control_manifest_current_sql_and_missing_original`，`state.rs:4308`，同样没有规则扰动或 QualifiedSource。
- `cross_request_source_cache_damaged_current_material_rejects_and_clears`，`cross_request_source_cache.rs:1938`：`:1951` 改内容 meta 规则，只以 `audit_native_current`/source keeper 的 `is_err` 核整库审计拒绝，不走本题组件入口，也不精确命中资格比较。
- `historical_bi_rule_is_readable_but_writer_and_recover_leave_it_unchanged`，`driver.rs:1306`，及 `s3_writer_and_recover_reject_frozen_nonempty_historical_session_unchanged`，`:1631`：断言旧规则正式写者/恢复拒绝，覆盖写者规则门，不能替代资格规则绑定负控。

因此没有新增负控的命中位置或运行收据。本阶段可准确保留的结论是：当前源码的持久读取→资格比较调用链存在，M6C 锁证明内部比较，现有正控经过该调用链；**真实 cold-open 对持久规则绑定的差分负控仍缺**。解除该缺口至少需要主控扩大测试构造授权或重新裁定验收判据，允许为被前置检查遮蔽的链路设计独立可达的验证；本节没有据此引入生产改动、额外测试注入或多字段夹具方案。

#### 实际检查

主控已取得以下本机回执：`rustfmt --edition 2021 --config skip_children=true --check rust/src/bin/s_session_v3/audit_owner/history_clamp_tests.rs` 退出 0；`PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check` 退出 0、清单与源码一致；`PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py` 退出 0、3 项通过。检查不调用 cargo/rustc。最终差异与笔记原前缀保留由主控复核；本节不宣称 Rust 编译或执行通过。Rust 未验证，待 CI。未 git add/commit。

非作者只读终审通过，确认上述两处前置拒绝与既有锁的覆盖边界。主控执行 Python 逐字对拍，确认仅追加 M6F、笔记原前缀保留、全部源码未改、暂存区为空、HEAD 未变；最终 `git diff --check` 退出 0。


### M6G

本节绑定 #1419 受管阶段 M6G，接续 M6F；HEAD 仍为 `b3889d89ec4752697c907b14327769355a798d44`，分支 `sandcastle/issue-1419`。**status=blocked**。主控已授权把内容库规则的 meta 与 manifest 副本一致改为完整 W2 串末字节 `2`→`3`、台账保持 W2。本轮源码核查发现，这个完整串先被内容历史的规则注册门拒绝，尚到不了 manifest 完整性检查，更到不了 `state.rs:1696-1700`。不改白名单、不绕过检查、不扩大篡改字段；仅追加本节，全部 Rust 源码、既有 M6C 内部精确负控、identity/root/unit 负控与正控不变。M6F 小节逐字保留；其中「内容 meta 首先撞 manifest 冻结域」的先后顺序由本节订正。

#### 授权构造的持久位置与恢复顺序

以下是依源码定位的构造方案，**未实际创建、篡改或 cold-open Rust 夹具**，行数为配方推导，不能当作实际 SQL 回执。

| 内容 | 表／列／键与拟改行数 | 保持不动的字段 |
|---|---|---|
| 内容库规则 meta | 内容库 `meta.value WHERE key='rule_revision'`，1 行；原串来自 `s_structure_session.rs:139`，共 153 个 ASCII 字节 | meta 的其他键、generation |
| manifest 中规则副本 | manifest 并非 JSON 列；`s_content_publications.manifest_root` 指向 `s_content_nodes` 内容寻址闭包，`history.rs:2411` 经 `text_meta` 取规则字符串。按 `s_content.rs:1516-1527`、`:1076-1091` 的编码及 `CHUNK_BYTES=4096`，153 字节全串只占一个 `tag=0x10` 的 BYTES_CHUNK；拟仅改该行 `body` 的最后一字节，1 行 | 该行 `digest`、tag、字符串头、序列边、所有祖先节点及 `manifest_root` |
| 旁挂完整台账 | `h1-identity.sqlite` 的 `meta.rule_revision WHERE singleton=1`，0 行 | W2 规则、cert 四根、fold_digest、node 全部保留 |

Python 按 `s_content.rs:491-508` 的 F 配方静态计算：原 chunk 的 digest 为 `56dbb552a6fc0395a5f0e2316d3f162e14d0941891cbf0d89a16235f7a99a185`，末字节改为 `3` 后重算为 `cd7354fb013ed1c2a50ee5946b480a502a97bbca8b5df07623024934059797a9`。这是配方计算结果，未从运行中的夹具读到这些行；没有把重算值写入任何库。`content_store` 的节点表定义为 digest 主键（`s_session_v2/content_store.rs:77-80`），同一 W2 字符串跨 manifest／代数共享该 chunk，改它会一致影响所有引用该串的规则副本。本目标锁只发布第 1 代（`history_clamp_tests.rs:937-941`）；不另造其他代或其他规则。

若构造可达，顺序应为：先保存两行原始值及台账资格／候选快照；在内容库同一事务内更新 meta 与上述 chunk，核各更新 1 行并提交；经原 reject 闭包调用 `open_certified_history_component`；拒绝后核目标精确错误、无 slot、父账归基线及资格／候选不变；再在同一事务恢复两行原值并提交，逐字回查；最后同入口 cold-open，比较完整资格与 `h1_compare_complete_history` 的完整冷 oracle。当前构造在规则注册门必拒绝，因此未实施上述步骤，也未把这个方案写成断言目标错误的测试。没有持久篡改或恢复残留。

#### 实际先行边界：内容历史的完整规则注册门

当前 HEAD 的调用链如下；本节所有路径均相对 `rust/src/bin/`，另注明的 library 路径除外。

1. 原 reject 闭包在 `s_session_v3/audit_owner/history_clamp_tests.rs:942-958`，入口仍是 `audit_owner.rs:488-500` 的 `open_certified_history_component`。seeded 核在 `:529-530` 开 Deferred 事务并调用 seed。
2. `audit_owner.rs:1932-1940` → `state.rs:1877-1884` → `:1924-1933` 的 `cold_captured`；`:1977-1988` 从本事务捕获完整节点，`:2021-2030` 调 `history::fold_certified_history`。捕获不是绕过摘要的成功证明：`s_session_v2/content_store.rs:2495-2565` 只登记原件，library `s_content.rs:3817` 明写不预核 F。
3. `history.rs:551-577` 进入 `read_indexed_history_inner`，先在 `:1016` 读辅助索引，再在 `:1021` 调 `object_index::load_for_history`，尚未执行后面的 manifest Template 全核（`:1077-1092`）或逐代冻结域比较（`:1563-1571`）。
4. `object_index.rs:2124` 的第一条实质语句是 `Layout::of_library(tx).map_err(err)?`。`layout.rs:35-43` 从**持久内容库 meta**读取完整规则，经 `:24-25`、`:160-161` 进入 `registered` 的全串 match（`:120-155`）。W2 登记在 `:128`；末字节变成 `3` 的 `bounded-layout/3` 串未登记，`:155` 直接拒绝。
5. `object_index.rs:58-59` 增加 `ObjectIndexInvalid：`；`history.rs:1021` 与 `state.rs:2021-2030` 的 `?` 传播没有再包 `HistoryInvalid：` 或 `S3StateInvalid：`。library `s_content.rs:237-245` 的 `From<String>` 保留原 message；`state.rs:1933` 的 `into_message`、`audit_owner.rs:1924-1927` 的 Invalid 分支及 `:1017-1022` 的 `history_result` 同样保留原文。

源码可确定的错误原文是下面这一整串，**不是本轮 Rust 运行收据**：

```text
ObjectIndexInvalid：VersionUnsupported：未登记完整规则 s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1+raw-initialization-w/2+bounded-layout/3
```

既有规则锁已把完全相同的扰动列入非法值：`s_session_v3/mod.rs:1028` 的 `W_STRUCTURE_RULE_REVISION.replace("bounded-layout/2", "bounded-layout/3")`，并在 `:1038` 断言 `Layout::of_rule(&invalid).is_err()`。本轮只读该锁，未执行。`open_historical_readonly`（`s_structure_session.rs:636-683`）允许以只读连接打开陌生规则，不代表随后历史审计接受它；这里不是正式写者的 `require_current_writer_rule` 门。

这个注册门本身构成「真实读取持久内容库规则，并按全串决定能否解释历史」的证据；它没有读取或比较旁挂台账的资格规则，**不能替代内容库规则与完整资格绑定的差分负控**。因此 M6G 所要的 `S3StateInvalid：完整资格的规则/内容库/二进制身份与本写者不符` 仍未命中。

#### manifest 摘要的后续独立边界

即便另有裁定解除注册门，保持现有摘要与根不动而改 chunk 正文，仍不能把它当作合法 manifest。`history.rs:1077-1092` 完整 Template 核验在 decode_manifest 之前；当前 cold 路径有 work、使用 NativeMeasured capture（`:991-1002`、`:4343-4370`）。原件读经过 `HashReader::checked_read_scoped`（`:3976-4002`）→ library `s_content.rs:4827-4842` 的 `checked_native_current` → `:2509` 的 `CheckedFrame::checked`，实际摘要不符在 `s_content/job.rs:523-533` 返回 `node_digest`，细节原文为「节点 (tag, body) 与摘要不符：声称 {}，实算 {}；未核摘要不成帧」。这不是 `closed.rs:432` 或 allocation prototype 的 `s_content.rs:2065` 分支。本构造已被更早注册门拒绝，不能声称本轮实际命中此摘要错误。

此检查把规则字节绑定在持久 manifest 的内容寻址闭包中；配合冻结域比较可约束 meta 与 manifest 同值，但没有单独证明台账资格比较的差分拒绝。既有具名锁 `history.rs:6726-6766` 的 `manifest_template_coverage_failure_precedes_real_history_decode` 在修改 manifest 子节点后断言 `node_digest` 且 manifest 解码次数为 0，源码可佐证检查顺序，仍非本轮执行证据。重算叶摘要、改边、祖先、manifest_root 或批次身份均超出本阶段授权，未实施。

#### 各断言与交付边界

| 要求 | 本轮结果 |
|---|---|
| 同一原 reject 闭包／cold-open 入口，新负控精确命中 `state.rs:1696-1700` | blocked；注册门先拒绝，不添加明知不可达的断言 |
| 无 slot、父账回基线 | 原闭包 `history_clamp_tests.rs:956-957` 原样保留；本轮没有新增运行结果 |
| 资格／候选未变、M6C 内部精确比较 | 原内部锁 `:1022-1059` 原样保留；不能借它冒充本轮持久扰动的证明 |
| 两处恢复后同入口成功、完整冷 oracle 对拍 | 未执行新构造，未新增恢复成功声明；原有同规则 cold-open 与完整 oracle 正控保持原样 |
| 不改生产检查、白名单、规则、资源帽、CI，不用旧 writer 或规则注入 | 所有源码及这些文件均无改动；M6F 未改写，M6G 仅追加 |

#### 实际检查

本轮实际执行以下检查，全部退出 0：

- `rustfmt --edition 2021 --config skip_children=true --check rust/src/bin/s_session_v3/audit_owner/history_clamp_tests.rs`，无格式差异。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check`，报告 `.github/prod-banned-literals.txt 与源码一致`，未重建或手改清单。
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py`，3 项通过；执行前完整读过脚本，确认该检查不调用 cargo/rustc。
- `git diff --check`，无空白错误。

另以 Python 逐字对拍，确认开工时包含 M6F 的全部笔记前缀保留，仅追加 M6G；`git diff --name-only` 只有本笔记。HEAD、分支未变，暂存区为空。本轮未运行 cargo/rustc；Rust 未验证，待 CI。未 git add/commit。


### M6H

本节绑定 #1419 受管阶段 M6H，接续并逐字保留 M6F/M6G。开工 HEAD 为 `b3889d89ec4752697c907b14327769355a798d44`，分支 `sandcastle/issue-1419`。本阶段按主控裁定接受夹具迁移的源码合同，并仅重冻 full Some 测试的两个审计摘要。

#### 主控裁定与证据限制

主控接受 M6C 的分层证据：内部精确负控、真实 cold-open 同值正控 `h1_qualified_online_consumes_real_ledger_and_cold_rebuild_matches`，以及持久完整性边界。边界包括 `rust/src/bin/s_session_v3/state.rs:2076-2086` 直读内容库 meta 并传入 fold、`rust/src/bin/s_session_v3/audit_ledger.rs:605-624` 核台账持久规则与安装身份一致，以及 layout 全串注册门、内容寻址摘要与冻结域。这些证据足以接受本次夹具迁移的源码合同；不恢复旧 writer，不重算内容寻址树。

M6F/M6G 两次构造仅为源码推导，未跑 Rust，不能记成运行拒绝证据。前两节的 blocked 是各自限定构造的历史结论，本阶段的验收按上述主控裁定收口。全项目历史读取与回退义务保持，本次裁定不免除这些义务，也不增加真实持久规则差分负控的运行收据。

#### 重冻来源与原文

来源为 PR #1855 的 `48ffac48c0b17317575fe0377f46f943716484cc`，[CI run 41414959715 / job 125176459116](https://github.com/xy7365527-lang/NewChanlun/actions/runs/41414959715/job/125176459116)，`rust-test-s-session` shard 1 原始日志第 1900 行。本阶段逐字取任务提供的该行原文，不从摘要转抄；未另行下载日志。打印来自目标测试 `full_some_audit_keeps_frozen_read_and_budget_trace` 自身：

```text
Some full: counts=Counts { objects: 91, groups: 13, windows: 11, effective: 14, insufficient: 0, outside_domain: 0 }, reads=227347, F=248860, read_hash=13bd2c89eae9fa7590d79dcf172f3ee2419ced7b826191ecc73a888f6c3052c2, budget_hash=20e6e506fdbbe095e5838c68df99add568377e9881a26460ae97540da00da37d
```

该 CI 在当时 `source_audit.rs:5615` 的 read_hash 断言失败，left 为 `13bd2c89eae9fa7590d79dcf172f3ee2419ced7b826191ecc73a888f6c3052c2`，right 为旧期望 `54375e9815ef8078d68d213e0a077f0b814ad7ce7a22432854441f1747eedfac`。前面的 counts、reads、F 断言未失败，实测与 M4 预期一致。budget_hash 取同一打印行的完整值，不声称 CI 已运行到其断言。

唯一源码改动位于 `rust/src/bin/s_session_v3/source_audit.rs` 的该测试：read_hash、budget_hash 期望分别换成上述原文值，并更新注释注明 run、job、提交和 reads/F 与 M4 一致。counts 元组 `(81 + 10, 13, 11, 14, 0, 0)`、reads/F 公式及其值均不动。

#### 实测对当前 HEAD 的适用性

动手前执行 `git diff 48ffac48c0b17317575fe0377f46f943716484cc HEAD -- rust/src/bin/s_session_v3/source_audit.rs`，输出为空；另以 Python 对拍工作区整份文件与该 CI 提交，逐字相同，目标夹具、审计调用、轨迹收集和摘要计算均未变。

同时枚举两提交全部差异，只有 M2-NOTES、ci.yml、`audit_owner/history_clamp_tests.rs` 与 `history.rs` 四份文件。Rust 差异只有后两份：前者为独立 H1 测试变更；后者仅在 `pub(super) mod tests` 的另一预算锁内插入 11 行只读遥测。去掉该唯一插入块后，history.rs 与 CI 提交逐字相同，包括目标调用的 HashReader 及被测生产路径。ci.yml 差异为 owned 组输出参数与 H1 收据名，不改变 shard 1 的目标测试。因此该行实测仍适用于当前 HEAD，无影响目标路径的差异。

#### 实际检查

以下检查本轮实际执行，全部退出 0：

- `rustfmt --edition 2021 --config skip_children=true --check rust/src/bin/s_session_v3/source_audit.rs`，无格式差异。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check`，报告 `.github/prod-banned-literals.txt 与源码一致`。
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py`，3 项通过；该检查不调用 cargo/rustc。
- `git diff --check`，无空白错误。

Python 反向对拍确认 source_audit.rs 除两个期望与指定注释外和开工 HEAD 逐字相同；计算原公式仍得 reads=227347、F=248860。笔记开工前缀共 201108 字节，SHA-256 为 `64d7d79e43a8252e834f6dd2e945d16b94b35e49a5f6d2a89d3751197b3d54d1`，追加后逐字前缀对拍通过，M6F/M6G 均保留。工作区全部改动仅本笔记和 source_audit.rs；生产代码、其他测试和 ci.yml 未改，暂存区为空，HEAD 与分支未变。未 git add/commit，未回退任何改动。

本阶段 status=ready。本机未运行 cargo/rustc；Rust 未验证，待 CI。


## M7B

本节绑定 #1419，开工分支 `sandcastle/issue-1419`，HEAD `388c8a2e465dd06018e43bc88d115e1554c877e4`，工作区干净。已读本笔记 M2/M2C typed 合同、M5/M5B、M6 的预算与 Box 拆帧记录，以及 `.chanlun/review-results/issue1481-h1-managed43r/REPORT.md` 的 S1R/S1S。题面将拆帧记作 M6 第 4 项，正文实际在「第 5 项：未被 S1R 覆盖的 Driver 返回大帧」；第 4 项是 resolver 准备预算。本轮沿原授权修复，不开新票。状态 **ready，Rust 未验证，待 CI**。

### 原件与证据边界

实际通过 GitHub jobs logs API 读取 run `41414959715` / native job `125176458766`，原件临时存 `/private/tmp/issue1419-m7b-native-job.log`，未入仓。`gh run view --log` 因 run 仍在运行而拒绝；改读已经结束的 job 日志成功。原件物理行 `:1539–1540`、时刻 `2026-11-02T09:39:26.5208333Z` 明写目标线程 4500 overflow / fatal abort。`:862` 明写 bin 的 base=964、with_feature=1535、delta=571、removed=0。`:1544` 保留完整 `--exact` 启动名单，debug binary 为 `s_structure_session-ffdf887457b646bf`。features 入口为 `s_session,s_capture_native_testonly`；`rust/Cargo.toml:87` 表明后者依赖 `s_incremental`，没有遗漏增量条件。

从该行解析 571 个唯一且按 C 字典序排列的名字，目标是第 86 项，字典序后续 485 项。完整名单逐名换行并以末尾换行结束的 SHA-256 为 `6a421f34c891b04d74b766df5d7e8a8f0049c94f85f4ac1ba8c71971bb3784e1`。libtest 并发运行，后续项 `h1_admission_foreign_sqlite_changes_retire_writer` 已在日志 `:1502` 先完成，不能把后 485 项全称为未运行；准确说法是 abort 后没有剩余完整运行收据。H1 `bea93e19` 的同名 native 锁通过依据为用户实测与 H1 REPORT S1U（`:2845`，526 passed / 6 failed / 9 ignored，无 overflow）的完整运行记录，不以当前源码检查替代该收据。

源码定位可确认**第一处新增的大按值类型来源及其进入开库链的帧**，不能确认实际撞 guard 的指令、哪一次冷开或峰值字节。诊断代理的最小通过栈、逐阶段 H1_STACK_PROBE 读数不在本轮可得范围。以下尺寸全部是 64 位字段估算，未运行 size_of、cargo 或 rustc；不以估算值小于默认栈阈值签通过。

### H1 ↔ 集成差异表与字段来源

行号相对当前源码；Rust 路径以 `rust/src/` 为根。

| 类型／位置 | H1 bea93e19 → 集成 HEAD | 由来与栈影响 |
|---|---|---|
| `bin/s_session_v3/audit_owner/admission_tests.rs:512/:567/:601/:610` | 整文件逐字相同；S1R 的 `Result<Box<Driver>,String>`、cold Box、非内联 helper、薄入口/body 均完整保留 | 不能重复把测试局部装箱当作本轮新增修复。helper 内仍要承接生产 API 的按值 `Driver`。 |
| `bin/s_session_v3/source_contributions.rs:508–512` | 新增 `structure_dependents: Map<TextValue,Tree<Handle>>`、`structure_entities: Map<TextValue,Handle>`、`structure_entity_dependents: Map<TextValue,Tree<Handle>>`、`structure_versions: Map<TextValue,Handle>`、`structure_changes: Tree<Handle>` | `028796ba080463e73532e7d6ab94493808dee70d`，W2 重组提交；不是 H1 增补。 |
| 同文件 `:537–540` | 新增 `w_init: Option<Handle>`、`w_raws: Map<i64,Handle>`、`w_raw_handles: Map<Handle,i64>`、`w_requests: Map<Handle,i64>` | `12bea702b1178c28d7f284d7bc15deda68aee25b`，W 初始化/未定原件来源贡献接线。上述合计九字段。 |
| 同文件 `:133/:243` 的 `Map/Tree` | 新增字段不是裸堆指针；每个 Map/Tree 内联持 `NativeTree/NativeSet` 和 `bytes:usize` | `s_content/capture_allocation/native_tree.rs:6` 内联 `BTreeMap`、`high_water:usize`、`CodecVec<NativeReservation>`；`work.rs:163/:168` 的 CodecVec 内联 Plain/Owned 存储枚举及许可。九字段为七个 Map、一个 Tree、一个 Option<usize>，外壳增量约 1 KiB，**估算**；容器内元素另在堆上，不把历史条数乘进调用深度。 |
| `theta_v0/parser/inclusion.rs:627–637` | 新增 `initial_evidence`、`initial_node`、`pending_raw`、`established_at`、`replayed_from_zero`、`record_steps`、`full_steps` 七字段 | 同为 `028796ba08`；经 `s_session_v2/s_fact_cache.rs:234 DeltaLane.state` 内联传播。证据值含两份 Bar 与坐标集合外壳，其余是 Rc/Vec/标量；没有巨型数组。 |
| `bin/s_session_v2/tb02a_facts.rs:795` | `FactCache.initialization: Option<(usize,usize,String)>` | 同为 `028796ba08`，经 `DeltaLane.facts` 传播；与上一行合计为数百字节增量，**估算**。 |
| `bin/s_session_v3/generation.rs:371` / `prepared.rs:121` / `driver.rs:84` | GenerationRuntime、Runtime、Driver 的字段声明本身在两侧相同，嵌套字段类型变大 | Runtime 的 `generator:Option<GenerationRuntime>` → `delta_lane:Option<DeltaLane>` 含上述 parser/fact 增量。Driver.runtime 仍按值，属第二条较小增长支路，本轮未改其布局。 |
| `s_content.rs:186` 的 `ContextError`（PublishError） | H1 内部 String 变 typed，M2 又闭合 H1 冷链 | 源提交 `b21fe10984`，M2 传播记录见本笔记前文。字段仅 `message:String` 与 `proof:Option<BridgeProof>`；`scratch.rs:564–573` 的 proof 为两个 u64 origin、一个 u64 bridged、ScratchFailure 枚举。64 位约 56 B，**估算**，比 String 约增 32 B；大量 Result 槽会累积，但无大错误载荷证据，不据此改错误表示或擦除 proof。 |
| `bin/s_session_v3/history.rs:3311` 的 HashReader | 新增 SqliteBase 读取分支、typed 传播 | SqliteBase 持借用，不内嵌整库；不支持「整个数据库被装进错误/reader」假说。 |

`NativeSourceState` 原有 root/raw_binding/raw_prefix/rule、raw/ordinals/order、objects/order/sequence/ids、groups/indices/gaps、shapes/slots/missing、raw_objects/group_shapes/shape_witnesses、witnesses/order/sequence、slots/observations/sequence、计数、steps/coords/handles/subjects/knowledge、pending/members/overlaps/coverage/cover_dirty、requests/order/sequence/keys/bound/subjects/step_requests/missing_requests/work 均保留。大的是这些容器的**内联外壳总和**，不是堆内元素被逐个复制到每层栈。

### 第一处来源与调用帧

**选定修复对象是新增来源状态把 owner/Driver 按值返回槽放大，而非猜测 typed 错误或节点线性递归。** 最早可指出的持有链如下，首个空库 open 也会为这些 Option 的完整载荷预留类型尺寸，不要求它们当时为 Some：

1. `source_contributions.rs:496 NativeSourceState` 的九字段 → `construction_state.rs:246 CommittedRawState.source` → `audit_owner.rs:322 AuditConnection.construction` → 开工 `driver.rs:85 Driver.conn`。`PendingRawState.source:476` 在事务阶段也含同一状态。
2. 目标 `admission_tests.rs:604 → :610 body → :615 h1_admission_driver`，共享 helper `:512` 内调用 `Driver::open_with_request_and_mode_retention`。生产 `driver.rs:490` 外壳既承接 `Result<Self,String>`，又持局部 driver 后再返回；`open_with_request_and_mode_context:534` 原来同时保留 Runtime、`rebuilt:Option<AuditConnection>`、分支产出的 owner、组装出的 Self 以及返回/移动槽。S1R 只缩减 helper 外的测试槽，未缩减这些生产槽。debug 是否合并/复用具体槽由编译器决定，不把源码槽个数直接冒称精确帧尺寸。
3. 冷入口 `audit_owner.rs:581 open_rebuilt_history_into` 进入冷认证/交接；`state.rs:1910 cold_for_request_inner → :1960 cold_captured → history.rs:551 fold_certified_history → :942 read_indexed_history_inner → :1250 audit_one_published_cut`。M2 在这些内部帧保留 typed 来源并于原 String 出口转换，没有另加判据。
4. `state.rs:1290 ColdHistoricalSourceFold.cold_source` 仍按值持该状态；`:1355 advance_cold_sources` 以 `take()` 送入 `source_contributions.rs:2508 audit_source_step`，后者的 previous 入参、`:2545 state` match 与 `Result<(Counts,NativeSourceState,SourceExports),PublishError>` 返回槽再次按值含它。这是更深的固定帧压力，不因一次冷开完成而叠加六轮递归。
5. 监督 recover 原 `driver.rs:880` 的 Runtime 与 `:885 rebuilt` 同帧持有至 owner.recover；本轮一起去掉外层 owner 构造/移动槽。Runtime/context 仍覆盖真正 Recover 和 owner 的显式析构。

递归复核沿用并回查 S1R 的具体对象：`construction_set.rs:68/:108` 是显式 Vec 前沿；source_order 的 AVL 父引用是 Weak，其两侧差异是 typed 返回和测试适配；fixed_radix、native_tree 两侧无差异；sequence 的 concat/split 仍按规范分拆随树高递归，新增 CodecContext 委托没有逐代 owning 链；history prefix/effective 的循环/平衡子树未变成按原件条数递归。W2 新增来源字段是容器，新增结构/W 阶段按差分迭代；未找到本路径新增 O(n) 调用或析构链的源码证据。没有据未成立的假说修改 Drop。

### 修复与语义中立论证

只改两份 Rust 文件。

- `driver.rs:86` 将 **Driver.conn 字段**改为 `Box<AuditConnection>`；`AuditConnection`、NativeSourceState、Committed/PendingRawState、完整候选、错误类型本身都不改。`Driver::open_connection:631` 标 `#[inline(never)]`，承接原 startup_context 分支并仅返回 `Result<Box<AuditConnection>,String>`。普通开库在这个 helper 内 `.map(Box::new)`，上层开库包装链只搬连接指针。
- `driver.rs:853 open_rebuilt_connection` 为第二个非内联 helper，内部保留原 `rebuilt:Option<AuditConnection>` 和原 expect，成功后返回 Box。Driver 冷开与 `recover_rebuilt_history:906` 内的 `:924` 调用共用它。它仍调用同一个 `open_rebuilt_history_into`，按原顺序传同一个 db、limit、first_read_deadline、context、runtime、max_store_bytes、scope、输出槽。构造临时仍可能在 helper 内占栈，不能声称全部大帧已消失。
- 无增量构建只传 db；增量非 Unix 保留 context 与原 Unix 拒绝文本；增量 Unix 才传可变 Runtime/scope。调用处的 Runtime mut cfg 与参数门一致。原 `db` 在调用前已经取 `scope.db_path()`，抽取后再次用该路径是同值；没有新路径解析或准入重判。
- pager、startup、Driver 组装、保留预算设置、resume_pending、identity 等原顺序不动。recover 的 context/runtime 仍由原外层帧持有，owner.recover 的前后探针及 `drop(conn)` 原序保持；Box 先析构同一个 owner，再释放外壳分配。Driver 字段析构次序、`into_output` 先 drop(conn) 再交还 scope 的顺序保持。
- 全仓实际 `git grep` 核 Driver 定义、构造、字段访问、模式匹配及尺寸计费：唯一 Self 构造在 open；唯一拆构在 into_output；其余 conn 消费是方法调用或借用，无将 owner 按值移出的消费者。Box→AuditConnection→Connection 的 Deref/DerefMut 适配这些调用，未修改方法签名或错误类型。另逐项核 `open_rebuilt_history_into`、`recover_rebuilt_history`、两个新 helper 的调用者。
- **为何不直接装箱来源状态**：`construction_state.rs:355/:463/:519`、`audit_owner.rs:2840/:2852/:2858` 按 Committed/PendingRawState 的 size_of reserve/split/保留计费。修改其内联字段会改变资源读数。本轮保持这些类型逐字不变；独立全仓搜索未见按 Driver 或 AuditConnection 尺寸计费，故移动外层 owner 不改变原业务账。新增物理成本是每个 Driver 或 recover 的一个全局 allocator Box 分配及指针外壳；它不是 Native codec 许可，也不伪报为零分配。原 Native 内联费、容器费、保留费与许可释放点均保持。
- `audit_owner/w2_h1_structure_tests.rs:69–84` 的 W2H1Fixture::open 原来返回 Driver，selected 的 `:287 online`、`:323 traced`、`:324 cold` 又保留多个按值槽。按 S1R 增 `#[inline(never)]`、返回 `Box<Driver>`、成功结果 `.map(Box::new)`；两处调用经 Deref 借用同一 Driver，`:320/:357` 的显式 drop 顺序保持。四条既有 W2 锁共用此 helper，所有场景与断言不动。

完整 W2 规则串、所有错误文案、预算和资源帽、观察点、effects、资格/证书消费均未改；没有增加 #799/#804 所禁的另一套判据或失败兜底。未改 workflow/ci.yml、栈设置、测试名、ignore、原测试场景与断言。Driver 测试模块与 HEAD 逐字相同；W2 文件逆向还原上述三种栈形状改动后与 HEAD 全文逐字相同。

### 同路径后续 native 项预检

范围为 construction_tests 的主文件与六个 include，实际按值开库/Runtime 调用及共享 helper 全部检索，再与本次 571 名单对照。以下是源码预检，不等于栈帧实测或运行 PASS。`admission_tests.rs`、`history_clamp_tests.rs` 与 `audit_owner.rs` 本轮未改，表内其行号不漂；W2 行号为修后。同名测试实际经 `audit_owner.rs:5742/:5749` 的 construction_tests/include 编译，仓内没有实体 `construction_tests.rs`。

| 文件与位置 | 检出的大值槽／处理 |
|---|---|
| `admission_tests.rs:512/:567/:601/:610`；调用 `:164/:224/:434/:465/:495/:575/:615/:717/:824/:951/:1233/:1251/:1369`，`legacy_admission_tests.rs:8` | 原 S1R Box/薄入口完整；生产 owner Box 使其底层 Driver 返回槽同步变小。 |
| `admission_tests.rs:758/:844` | cold 原节点/记录门、真实 wire 短门的失败 Result<Driver>；两测试字典序在目标之前，不能冒作未运行尾段。生产修复同步缩减其 owner 返回槽。 |
| `admission_tests.rs:901/:995/:1082/:1162/:1192/:1305` | prepolicy、临时 seed 失败子进程、正常退出子进程、Ordinary 拒绝/成功、hot journal，均在目标后名单，仍有单个 Driver 或失败 Result；共用修后的生产构造。`:1016` 为该子进程的显式 drop，不是另一处构造。 |
| `admission_tests.rs:54 runtime` 与 `:68 open` | Runtime/组件 owner 按值 helper；所有调用分别为 `:281/:324/:357/:744/:797` 与 `:283/:326/:359/:746/:799`。foreign 已有 ok 原件，online 在后续名单；另几个在目标之前或为共享准备。直接组件单 owner/Runtime 构造与多层 Driver 返回槽不同，本轮只登记，不改其合同。 |
| `w2_h1_structure_tests.rs:70/:285/:287/:323/:324` | 多次 Driver/traced/遮蔽槽，同因修成 Box；CD `:379 → :436`、EF `:521 → :557`，及 `w2_h1_window_tests.rs:96 → :111/:161` 的两场 W 未定、`:207 → :232` 的 bounded2 锁，四原名测试均在目标后实际名单。 |
| `history_clamp_tests.rs:2 h1_owner` | 组件 owner 按值 helper；调用 `:16/:26/:51/:72/:81/:91/:101/:114/:132/:140/:145/:157/:218/:322/:365`。另 `:230` 直接普通开库；certified helper `:394–395`、qualified helper `:775–780` 及失败/直接 component 调用 `:561/:581/:946/:1264` 均按值返回 owner/Result。未改组件接口；它们没有经过 Driver 包装链，不能声称被本轮生产 Box 全部覆盖。 |
| `history_clamp_tests.rs:833/:883/:922/:939/:978/:1103/:1177/:1324` | qualified helper 的 traced、冷开、失败、重开调用，含 owner/Result 槽；后续名单包含各 qualified 锁。本轮未改，保留为 CI 重点观察项。 |
| `history_clamp_tests.rs:27/:60/:268/:287/:310/:425/:467/:503/:641/:769/:897/:913/:1241` | 直接 Runtime 构造；`:767 h1_qualified_runtime` 的调用为 `:831/:896/:936/:1101/:1175/:1322`。`:893` 的资格负控同时持 empty/partial/none 三份 Runtime；这属于已列第二条 parser/runtime 增长支路，不为未经实测的失败另改生成器布局或计费。 |
| `audit_owner.rs` construction_tests 内 `:5858/:6004/:6005/:6018/:6277/:6310/:6491/:6533/:6621/:6700/:6742/:6766/:6887/:6935/:7018/:7105/:7185/:7241/:7311/:7348/:7377/:7399/:7418/:7438/:7455/:7487` | 直接组件 owner/普通开库槽，包含 warm/cold、外来连接和重开；保留。warm/cold helper 可由后续 `h1_request_closure_real_owner_multicut_reuses_resolved_roots` 与 `p6_native_source_context_preserves_real_owner_multicut_contract` 进入，不能仅按 helper 名 b2 判定在目标之前。 |
| 同文件 `:6278/:6311–6312/:6492/:6534–6535/:6623/:6701/:6744/:6767/:6888/:6936/:7020/:7136/:7263` | 组件并存 Runtime；后三处分别是 set_publish_case、cross_request_publication_case、publication_scope_case 共享准备，后续 owner/publication 锁可达。与上一行单组件槽及第二增长支路一起登记，未泛化改测试。 |
| `b2_source_fault_tests.rs:495–496` | 直接 owner + Runtime；`legacy_admission_tests.rs` 仅用既有 Box Driver；两个 W2 include 除上述共享 open 无其他 Driver 构造。 |

本轮消除的是**Driver 包装链及 recover 外层重复承接变大 owner**，并修 W2 夹具的多次 Driver 返回同因；保留单组件槽、Runtime 支路和冷来源核自身的固定大局部。没有证据声称这些残余都会溢出，也没有证据声称它们已经栈安全。故不为“还有一个按值变量”就扩散修改所有组件或改变原收费类型。

### 实际检查、独立复核与待 CI

本轮实际执行并退出 0：

- `rustfmt --edition 2021 --config skip_children=true --check rust/src/bin/s_session_v3/driver.rs rust/src/bin/s_session_v3/audit_owner/w2_h1_structure_tests.rs`。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check`，产物与源码一致，未重写或手改禁串清单。
- 已完整读 anchor_lock 脚本后执行 `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py`，3 项通过；该脚本不调用 cargo/rustc。
- `git diff --check`；Python 核 Driver 原测试模块逐字不变、三个原拒绝/expect 文案各保留一次、W2 文件逆向还原逐字对拍通过。

非作者只读独审分别核两份 Rust diff、全仓使用者、三类 cfg 参数、Deref 适配、所有传参/错误文本、scope 与 drop 次序、sizeof 收费布局，未发现阻塞。独审没有 Rust 编译或运行收据。

追加前笔记为 205827 字节，SHA-256 `b14c2b1245920ed62eea89afb5a43b425c3937ecbd6c6d809ee815f68734f51f`；本轮只在末尾追加，不追溯改写 M2–M7 记录。未 git add/commit、回退改动或运行 cargo/rustc。

待 CI 必须以封包精确 head 验证：

1. 原 `h1_admission_driver_cold_recover_rebuilds_ledger_bitwise` 在 native **debug 默认测试线程栈**下完成全部原断言，包括冷3/冷5、真实 recover、epoch2 pending 第六代、台账 bitwise 与最终冷6；不得靠增大栈或资源帽通过。
2. 原 native delta 完整名单继续执行，不再 fatal abort；重点是上述 normal_exit/seed_failure/Ordinary、组件 qualified 与四条 W2 冷往返后续项。运行失败须逐项报告，不能把不再 abort 当作业务断言全过。
3. 三类 cfg 的 Rust 类型检查与既有 Native/Ordinary 对拍、资源账、effects/证书锁保持；本机格式/anchor 检查不能证明它们已通过。
4. 诊断读数回来后，对比两侧 `driver_open/cold_open/cold_source/source_step/recover_open/codec_read_peak/complete` 与最小通过栈；具体失败帧如仍在冷来源核或 Runtime 支路，应据该阶段读数继续修，不能把本轮结构估算当运行定位结论。

本阶段 status=ready；Rust 未验证，待 CI。


## M7C

本节绑定 #1419 受管阶段 M7C。开工 HEAD `2c6cb4e335bf6ce8888499b6de870b6d27f0c6a9`，分支 `sandcastle/issue-1419`，工作区干净。已读 M3 四锁、M4 去重/cost_diag/13377 的待裁定边界和 M6 pager flush。M7B 栈修复已在基线，本轮不改栈。**status=blocked，仅追加本节，不改 Rust。** 原件能支持一个与节点阈值吻合的具体扩容预算候选，但不足以唯一识别本次首次拒绝的内层分支或实际申请字节；也未证明集成回归。不能把候选推导签成已完成根因定位，更不能据此改帽、改夹具或实施新设计。

### 原件、最后可见代与读数口径

实际执行题面命令取得 run `41414959715` / owned job `125176459422` 日志，保存 `/tmp/issue1419-m7c-owned.log`，3117 物理行，SHA-256 `6c7f0f80da41dd472b4285d5d6c9344d298c3abd659250e96f83544968c96150`；临时原件未入仓。原件 `:1654` 是 release、`s_session,s_incremental`、`h1_` 过滤、单测试线程、`--show-output`；`:2668` 为 78 passed / 4 failed。两个 typed-kind 失败不在本题，M6C 已处理；本轮没有重跑或重签它们。bounded2 与 W 未定锁在 `:1780/:1783` 通过，C/D 与 E/F 在 `:1781/:1782` 失败；后两者 `:2603-2604/:2658-2659` 均为在线 `online.handle(envelope).unwrap()`，不是冷副本开库失败。

下表逐项摘自同一 stdout 分段。`nodes` 是候选累计原件计数，不是本代新增节点数；`native_live` 是 Native 分项当前存活，两个 peak 是各自高水位。B 表示字节。

| 场景/最后可见代 | 日志行 | nodes | native_live B | native_peak B | total_peak B |
|---|---|---:|---:|---:|---:|
| C/D g1 | 2558-2559 | 2946 | 4451279 | 7571502 | 8508486 |
| C/D g5 | 2566-2567 | 13668 | 15994017 | 26322524 | 28267325 |
| C/D g10 | 2576-2577 | 33538 | 47132113 | 71573281 | 75237691 |
| C/D g15 | 2586-2587 | 62148 | 89668175 | 133322155 | 139388259 |
| C/D g20 | 2596-2597 | 98306 | 108737203 | 175058686 | 185444200 |
| C/D g21 | 2598-2599 | 103815 | 110491786 | 179985720 | 187117044 |
| C/D g22 | 2600-2601 | 109677 | 112674716 | 185768224 | 195024666 |
| E/F g1 | 2607-2608 | 2946 | 4482564 | 8817285 | 10003835 |
| E/F g5 | 2615-2616 | 10792 | 13933213 | 22465056 | 24387142 |
| E/F g10 | 2625-2626 | 24999 | 28975375 | 46867550 | 50257220 |
| E/F g15 | 2635-2636 | 46090 | 54287601 | 86589233 | 91569115 |
| E/F g20 | 2645-2646 | 75863 | 98132772 | 149645586 | 157010360 |
| E/F g24 | 2653-2654 | 103810 | 112272258 | 181151806 | 190668500 |
| E/F g25 | 2655-2656 | 112896 | 117281429 | 190851405 | 200371967 |
| bounded2 首轮在线 g18 | 2286-2287 | 101481 | 109281600 | 185294089 | 194506608 |
| bounded2 冷点续写后的 g18 | 2386-2387 | 101481 | 171725731 | 246752052 | 258585775 |

必须保留两个限制：

1. `audit_owner.rs:4031` 在调用 append_cut 前打印 suffix，`:4061` 在 append_cut **返回后、判断结果前**打印 codec。故末条 g22/g25 既不是该代提交收据，也不是失败代收据。可能是该代 append_cut 返回资源错误，也可能该代完成后下一代在首条 suffix 之前失败。不能把失败代直接写成 g22/g25，或 g23/g26。
2. `audit_owner/native.rs:518/:983` 解释 Native 与 scratch 元组；`:900-903` 明写换冻结域不重置 Ledger.live/peak 及分项高水位，`:926` 的 request_peak 才按当请求 allowance 与 low_water 计算。`live_peak_meta=(0,25861024,96,192)` 的 0 是当时 scratch live，25861024 是历史 scratch peak，96/192 是工作域元数据当前值/高水位。各峰可能发生在不同时间，不能相加，不能当作本次失败申请值或从中反推 A/J。native_live 增长有当前存活证据；total_peak 单调本身不证明每代工作量线性增长，也不是 RSS。

### 预算闭环与最强候选

以下路径、公式以当前 HEAD 为准，Rust 路径前缀为 `rust/src/`。

| 层次 | 来源与公式 | 本锁已知值/未知值 |
|---|---|---|
| 第八门 R | `bin/s_session_v3/audit_owner/w2_h1_structure_tests.rs:86` 的正式 open 参数；`driver.rs:523-524` 保存 retention，legacy 第七门为 usize::MAX | R = 536870912 B，即 512 MiB；是既有夹具准备额，本轮不改 |
| 请求冻结 A | `driver.rs:692-708` 令 T=legacy+R；`request_audit.rs:498-508` 令 A=T−legacy−O−U=R−O−U | O 是旧池合同额，U 是未覆盖物理费用；本日志未输出失败请求 O、U 或 A。非负分区给出 A≤R |
| 具体父账 | `audit_owner.rs:2702` 签发 PublicationResources，`:7796-7800` 从该候选父账签 codec_work；`native.rs:650-663` 冻结总帽 L0+A、remaining=A；`:966-979` 工作域限额用当前 A | 同一候选的父 Arc/冻结域，不是独立额外预算。失败时 L0/remaining 未记录 |
| journal 分额 J | `audit_owner.rs:4372-4375` 的 P=min(A,parent.remaining())；`:4409` 传 P/8 给 `effects::Recording::start_on`；`effects.rs:574` 存 Journal.cap | J=floor(P/8)≤floor(R/8)=67108864 B。67,108,864 是理论上界，不是失败时 J 实测；O/U/已用余额会使实际值更低 |
| 索引扩容需求 G | `s_content.rs:3558-3564` 满表时 C'=max(2C,4)，G=(2C'+16)×(size_of<(NodeDigest,NativeNode)>+1) | C≥2 时 G=(4C+16)×W，W 是实际槽宽。整块新索引先申请，旧 index_permit 仍在，成功后才替换释放 |
| 两道不同的拒绝门 | `s_content.rs:3564` 先 work.reserve_native(G)；`:3565-3568` 再核 G>cap 或真实 HashMap.try_reserve 失败 | 前一道可报父 Budget；后一处为 ScratchFailure::Allocation。`native.rs:1551` 将 Budget/Allocation 都映为 BudgetExceeded，最终错误不保留这一区分 |

局部 cap 传递有两个入口，不能混写：`audit_owner/effects.rs:1462-1486` 的来源材料准备遍历本事务 j.nodes，回读原件后调用 append，并传 **j.cap=J**；`audit_owner.rs:3911-3913` 的尾部原件补入则传 **usize::MAX**，仍由同一父账收费。因此“累计表跨阈值必然撞 J”还要求跨档扩容实际发生在前一个入口；原件没有记录这一点。

仓内既有布局实测 `.chanlun/review-results/1488-acceptance/receipts/20261021-040601-cdd1f8690f.md:15` 的 `R14_P2B_DROP` 给 W=225 B、slot_before_p2a=241 B。它是历史布局读数，**不是本次 CI 的 size_of 实测**。本轮静态比较当前 NativeNode 与 H1 原件一致，详见下节。在该槽宽下，可复核算式为：

```
C = 57344:   G = (4×57344 +16)×225 =  51613200 B
C = 114688:  G = (4×114688+16)×225 = 103222800 B
J ≤ 67108864 B
103222800 > 67108864
```

常见当前 HashMap 的 7/8 有效容量档为 57344、114688、229376；这里按该容量模型推导，当前失败实例的 `.capacity()` 没有日志收据。C/D 最后 nodes=109677，距114688还5011；E/F=112896，还1792；bounded2 最大101481，还13207。前两场比 bounded2 更接近下一容量档。只要后续在 j.cap 入口满到114688，按上述槽宽就要申请103222800 B，即使总父账仍有空间也会被 J 拒绝。这解释了为什么较小 total_peak 的场景可能先失败、较大 total_peak 的 bounded2 仍能通过。前一档51613200 B可能获准，后一档103222800 B不可能过J，阈值是按累计节点容量跳变，而不是固定 total_peak。

这是一条有文件、帽来源和需求公式的**最强候选**，不是本次首次 Budget 的唯一定位：实际失败时的代号、入口、len/capacity、槽宽、G、J、剩余增量和失败分类全部缺测。不能把103222800 B写成本次实际需求，更不能写出一个未测的实际J。尤其父 reserve 在局部 cap 比较之前，故即便同一扩容最终必越J，也可能首先在父 remaining/总帽处被拒。

父账真实条件在 `audit_owner/native.rs:1575-1584`：申请r须同时满足 r≤remaining 与 live+r≤L0+A；失败预留不进入peak，故日志峰值小于帽不能反证父预算失败。工作域还会在本 scope 追踪失败并于退出时结账，不能只读总父峰忽略工作域限额。

### 为什么现有统一错误不能再细分

`audit_owner.rs:7798` 的 codec_work 签发失败可以直接返回None；`:7815-7864` 的闭包、关域或父健康失败，经 `publication_degrade_verdict`（`:7900`）仅在类型化记录全为 Budget/Allocation 且错误来源匹配时返回None。H1 的 `:1543-1547` 将其统一为“认证发布资源不足；禁止无候选重试或完整审计兜底”。stdout 没有 FailureTrace、失败申请或分支编号。因此该错误足以锁定纯资源失败类，不能区分 Budget 与 Allocation，也不能点名容器。

末条 codec 之后还存在 `:4075` 历史保留增量应用、`:4083` 资格票reserve、`:4100` 非节点SQL originals更新、`:4153` 总保留终检。后者需求为 `node_source_and_certificates + node_capacity + historical_and_raw_capacity + current_sql_capacity + q_capacity + inline + Budget元数据`，对照父总帽，而非日志total_peak；精确分项本日志也未给。**不能把所有返回None的门都当作本次同文的来源**：validate_effects 的None在 `:4561-4563` 经 missing() 变成effects未闭合，仍须检查外层类型化资源记录与桥，不能按错误文字猜测。

若主控接续定位，最小缺证集合是：原两锁首次资源拒绝时的请求消息/代、阶段与入口、失败种类、requested、scope limit/live、parent remaining/live/L0+A；若是索引则补nodes.len/capacity、W、G、J和父reserve是否已成功。应由独立诊断构建/既有预算普查取得，只跑原锁并保留原帽与原断言；诊断读数再与原生产路径对照。**本阶段没有插入任何新日志或改变观测语义，也未触发远端跑批。** 在这些读数回来前，第1项“失败那次首次尝试的具体预算及实际需求”仍未闭合。

### 累积存活的成因、H1 对照与合同边界

现有实现确有随历史增长的持有结构，但没有发现把整个前代运行态逐代复制、未释放的集成证据。

- `effects.rs:1462-1493` 对本事务真实 j.nodes 回读并追加已有 capture；`s_content.rs:3548-3554` 遇同digest先核正文相等直接返回，`:3586` 仅插入新digest。这是一份累计唯一原件表，不是重复read就保存一份副本。`:3591-3614 finish_owner_step` 清本步 proof/dirty链与 memo，保留节点正文/索引；未对历史节点做可达性回收。历史捕获及容量因此随累计发布原件增长，索引有阶梯扩容。
- `history.rs:8973-9004` 的 append_owner_cut 只接管旧状态再消费本代实际 object/aux 行；`:9070` 附近的SQL是 `WHERE p.generation=?1`；`:9123` 向cuts追加一代。原文注释明写“本轮只消费新变更”。没有每次从g1重新遍历所有cut的控制流。roots当前闭包、当前窗口发展记录本身的大小仍会增长，不能把“只核一代”误写成所有字节工作恒定。
- `state.rs:3844` 以self接管AuditedState；`:3876` 的 NativeRc::try_unwrap 移出共享历史，解包失败即拒。`audit_owner.rs:3826-3854 detach_slot` take capture并实际drop旧result/binding/source_pending，`:4197` 的continue_with替换complete；`source_contributions.rs:2544` 接管previous state，`:980-1031` 清理撤回对象的结构反向索引。W2新来源索引会保存活跃依赖，但没有发现额外持一套完整前代状态的证据。不能把无泄漏证据签成所有容器均已实测释放。

实际对 `git show bea93e19:rust/src/s_content.rs` 与当前源码作Python字节比较：NativeNode声明11行逐字相同（H1 `:2125`、当前`:2286`），满表扩容块17行逐字相同（H1增长式`:3399`、当前`:3560`）。`capture_allocation/work.rs`、`capture_allocation/native_owned.rs`、`s_content/job.rs` 三文件与该H1原件全文相同；NativeReservation字段未改，scratch差异为错误来源追踪。effects传j.cap的路径在H1已存在，typed传播未改变这个参数或增长式。故“typed错误让NativeNode槽宽增长、制造这档扩容失败”没有源码依据；历史捕获累积和这条局部cap耦合也不是本次集成新引入。

H1 admission同类6代锁 `audit_owner/admission_tests.rs:610` 的 driver_cold_recover_rebuilds_ledger_bitwise 共6代（g1-5、epoch2 pending g6，含多次真实冷开与recover）。当前owned原件`:1898-2002`有其逐代输出且`:1729`为ok：

| 代 | nodes | native_live B | total_peak B |
|---|---:|---:|---:|
| 1 | 2943 | 4433281 | 7922105 |
| 2 | 5043 | 7463124 | 12637523 |
| 3 | 7834 | 12598058 | 16863110 |
| 4 | 10878 | 14754843 | 24292875 |
| 5 | 13099 | 15193193 | 26653484 |
| 6 | 15809 | 22389762 | 35878494 |

这证明同一集成构建的既有H1锁也有累积存活，6代成功不覆盖22/25代的节点容量档；**它不是纯H1原件的性能实测**。另实际读取 `/Volumes/AgentStorage/h1-recover45c/ci-bea93e19/125154371398.log`：`:1565` 确认H1原分支的同名6代锁通过，但成功stdout未保存其逐代读数，不能补造。该原件`:3558-3565` 的另一失败锁 `p6_complete_ledger_cover_matches_across_delta_and_full_lease_owner_paths` 的增量臂有g1→g4：nodes1848→7823、native_live3456769→11937391、total_peak5035511→16064588。其失败不作为通过收据，只用于证明H1原实现已有同类小规模累积；它不提供本次容量档的H1成功反例。

**合同并不要求每代全史重读或常驻全部原件。** 本轮实际从GitHub取得 [#1481票面](https://github.com/xy7365527-lang/NewChanlun/issues/1481) 到 `/tmp/issue1419-m7c-h1-ticket.json`：目标句要求单条成本只随本代新增量；“Δ步实现”要求“新代只核本代新增节点与变化对象”“不常驻全部原件”；“判据单一”禁止资源不足回旧审计。已批准 `.chanlun/review-results/issue1644-p6-audit-ledger-20261021/AMENDMENT.md:9/:39/:45/:121` 分别规定重启从原件全史认证、台账在盘且每请求仅Δ行、冷从g1用同一个Δ步、“写前不再重读全部旧原件”。ADR0042“恢复与冷启动”同样要求持久态重建，未授权每条在线发布全史重审。

因此应区分：冷认证全史线性读取有合同依据；当前在线累计capture/cuts/index有既有实现证据；每代全史重审没有该合同依据。H1 REPORT原合同映射`:33`留资源/3等后续，阶段实现不能替代最终验收。**本轮结论为“既有实现累积成本及局部预算耦合的设计缺口候选，W2更多节点使其提前暴露；需裁定”**，不能写成“已批准合同本身必需的线性成本”，也不能据此宣判“集成回归已证实”。日志只给存活/高水位，没有逐代请求读取次数或工作分桶，故本轮不签每代工作量的精确增长阶。

### 与 M4 的13377次重复关系

M4的13377是特定full Some夹具、发展批前缀跨条目重审的**读取次数**，不是当前capture新增唯一digest数或Native存活字节。M4已查明+30267由W2链的bounded-layout/2来源绑定核带入，集成段净增0；本轮没有改写这一历史结论。

在线实际链是 `prepared.rs:1094-1114 → audit_owner/effects.rs:1523 → source_contributions.rs:2469/:2508`；object_phase只调度added/dirty（`:1189-1206`），`:1237-1251` 的audit_object_record_with_slots仍传development=None。因而同族重复审计仍可能发生，但不是每代完整来源审计。来源材料读取 `effects.rs:1378-1394` 委托ActualRead/TxStore，`:1401-1412` 的NodeSink拒写；capture追加只来自前述j.nodes。重复重读不会直接给累计capture索引新增一份节点，不能把13377称作索引扩容的直接来源。它会增加本代SQL/codec核验及临时工作，日志没有其分项字节，不能声称它的峰值贡献为零或等于13377。

`source_audit.rs:2882-2889` 的session门和 `history.rs:3876-3898` 的eligible_sqlite_base继续排除Native/Owned、工作域、预算/截止、overlay/fresh和s_incremental。M4引用的[#1419 P2批准](https://github.com/xy7365527-lang/NewChanlun/issues/1419#issuecomment-6526264864)限定同次真实SqliteBase审计及原proof/材料帽，不允许直接把该表挪进Native在线发布。因此补审计内受管单元表仍须主控裁定，且不能承诺它会修复累计capture索引扩容。

### 结论、修复门与交主控的选项

本阶段没有符合授权条件的语义中立最小修复：H1已存在的表/扩容式未变，未证前代状态泄漏或重复持有；新表、容器替换、原件淘汰均会触及分配/峰值/观测，不能直接实施。未改任何判据、帽/准备额、原锁代数/场景/断言、错误文案、effects、证书或观测；未启用无候选重试、完整审计兜底或第二档判据。M7B/M6改动全部保留。

交主控的独立选项与代价如下，均不是本阶段已实施方案：

1. **先补首次拒绝的精确收据。** 按上节最小缺证集合，在独立诊断构建中取原两锁的首次失败字段；保留既有锁、预算和断言，与原生产构建结果对照。代价是一次受控诊断与CI窗口；收益是确认局部J、父剩余额度或另一个容器哪一门先拒。现有日志不能替代此步。
2. **确认累计索引扩容为因后，裁定有界原件持有设计。** 候选方向是分段/可持续的受管索引，或让在线Δ消费持久原件并只持有有界必需前沿，与“不常驻全部原件”对齐。须证明同位置/同摘要继承、缺失/篡改首错、同父许可、冷/在线证书和全部effects合同保持；容器替换会改变容量与峰值观测，原件淘汰还可能影响当前引用存活和认证语义，均需单独设计评审，不能宣称只是删缓存。不得以放宽J或R作为本票修复。
3. **M4重复成本单独裁。** 可暂保现有memo排除，接受已知重复读；或授权一次Δ来源步内受管单元表，保current完整核、实际头部和本raw绑定、完整format/kind/extent键、前缀结构判据与统一释放。新增表改变受管容量、F/SQL/预算事件、deadline采样及可能的首个内存拒绝位置，必须实测重冻并独审；不能按本轮“观测语义不变”的门实施，也不能替代选项1/2。

### 实际检查

以下检查收据在本节末补记；本机不运行cargo/rustc，也不执行会间接启动它们的脚本。追加前原笔记225267字节，SHA-256 `dea0611ce98eed4b60f8108d2ead9879628af49a520ffebad4f31c670cf2aeee`，前文逐字保留。本轮只写M7C，不git add/commit，不回退任何他人改动。

实际执行并退出0：

- `rustfmt --edition 2021 --config skip_children=true --check`，覆盖 s_content.rs、audit_owner.rs、audit_owner/effects.rs、audit_owner/native.rs、history.rs、request_audit.rs、audit_owner/w2_h1_structure_tests.rs 七个相关Rust文件；本轮这些文件均无改动，检查仅证明格式可接受。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check`，输出清单与源码一致；未重建或手改清单。
- 已读完整anchor_lock脚本后执行 `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest s_session/tests/test_tb01c_r4_anchor_lock.py`，3项通过；其CLI负控仅用临时Python stub，未调用Rust工具链。
- Python核原笔记225267字节前缀与HEAD逐字相同；提取H1/当前NativeNode及扩容块逐字相同，三个底层文件全文相同；按日志物理行复核上表读数、节点距阈值与两档G算式。
- `git diff --check`，暂存区为空；改动清单仅本笔记。独立非作者只读复核未发现预算公式、原件读数、合同或M4归因的阻塞错误，保留本次实际首错缺测这一限制。

**Rust未验证，待CI。** 本机未运行cargo/rustc、Rust测试或构建；本阶段未修复原两锁，不能声称它们已通过。结论为blocked，附上述诊断缺证与设计选项交主控裁定。


## M7D

设计指针：[M7D-DESIGN.md](M7D-DESIGN.md)，绑定 #1419；本阶段 status=ready 仅指设计包交付，首次拒绝诊断仍待主控收据，不代表原两锁已通过。

实际检查收据：

- 已 live 执行 `env -u GH_TOKEN -u GITHUB_TOKEN gh issue view 1481 --comments` 及同命令的 `1644`，均退出0；核对 AMENDMENT、ADR0042 与 H1 当前源码。合同与源码两位非作者只读复核最终均 PASS。
- Python复算设计的8行估算与扩容公式通过，16个Rust引用文件存在；估算没有作为实测收据。
- `git diff --check` 与新文件 `git diff --no-index --check -- /dev/null .chanlun/review-results/issue1419-w2h1-integrate46x/M7D-DESIGN.md` 均退出0。
- 追加前笔记245211字节，SHA-256 `dfc512b40ecf029af946903ecf0dc53b58db69b909617f66dde262b8629bb3b2`；Python前缀逐字对拍通过。M7C原19942字节，SHA-256 `a96ae3b05bc704f361f4fda519657e881abbc5a7631bf0a18353c10dcf1da5c8`，逐字保留。
- 改动仅本笔记追加与设计新文件；HEAD/分支不变、暂存区为空。不运行cargo/rustc，不改Rust/ci.yml，不git add/commit。
