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
