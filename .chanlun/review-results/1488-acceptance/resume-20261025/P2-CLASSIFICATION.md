# #1488 P2 真实组合探针

基线候选：36628b7bc31a81656217f6f05254a43829c4ec0d。探针仅在远端临时工作树移除旧 `8组合实测无调用方` 的 dead_code/unused_imports/unused_variables allow；工作树本体未删除这些实现。

S=s_session；N=S+s_capture_native_testonly；J=S+s_allocation_json_testonly；JN=全部三者。P=--lib --bins；T=--tests。8档 cargo check --locked RC 均为0，完整输出见 logs/20261025-215454-36628b7bc3.log，脚本 probe-eight.sh。

结论：旧报告的 default/B/BN/BJ 组合与原探针不符；optional_sql 在 JT/JNT 使用，应按 JSON 测试门控；HashSet 在 NT/JNT 使用，应拆分导入；native_shared 为宏，告警对应部分实例，不能断言所有实例无调用。新出现 native 未用项按 P/T 实测分开门控，保留函数体和原断言。

下表位置属于去除 allow 后的临时探针，不作为最终源码行号。告警缺席只表示该编译组合未报该项，类型只在 feature 内存在时不推断其它组合调用。

| 探针位置 | 告警 | 出现组合 |
|---|---|---|
| src/bin/s_session_v2/cache.rs:23:34 | unused import: `HashMap` | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v2/content_manifest.rs:22:71 | unused imports: `NumberToken` and `Value as V` | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v2/content_publication.rs:37:34 | unused import: `BTreeSet` | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v2/evidence_store.rs:2124:13 | unused variable: `admission` | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v2/s_fact_cache.rs:387:9 | field `old_groups` is never read | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v3/audit_owner.rs:102:25 | unused import: `PublicationStageIdentity` | NP,JNP |
| src/bin/s_session_v3/audit_owner.rs:113:73 | unused import: `RawDomain` | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v3/audit_owner.rs:1306:19 | method `complete_retained_bytes` is never used | NP,JNP |
| src/bin/s_session_v3/audit_owner.rs:180:8 | method `remaining` is never used | NP,NT,JNP,JNT |
| src/bin/s_session_v3/audit_owner.rs:852:19 | method `put_source_keeper_for_test` is never used | NT,JNT |
| src/bin/s_session_v3/audit_owner/allocation_probe.rs:76:15 | function `released_bytes` is never used | NT,JNT |
| src/bin/s_session_v3/audit_owner/native.rs:1285:19 | method `budget` is never used | NT,JNT |
| src/bin/s_session_v3/audit_owner/native.rs:1313:19 | struct `BudgetObserver` is never constructed | NP,JNP |
| src/bin/s_session_v3/audit_owner/native.rs:1314:8 | struct `ObserverInner` is never constructed | NP,JNP |
| src/bin/s_session_v3/audit_owner/native.rs:1321:19 | associated items `sponsor_bytes`, `new`, `live`, `peak`, `total_live`, and `same_parent` are never used | NP,JNP |
| src/bin/s_session_v3/audit_owner/native.rs:1350:19 | method `total_live` is never used | NT,JNT |
| src/bin/s_session_v3/audit_owner/native.rs:1445:19 | method `is_empty` is never used | NP,JNP |
| src/bin/s_session_v3/audit_owner/native.rs:582:19 | method `bind_frozen` is never used | NP,JNP |
| src/bin/s_session_v3/audit_owner/native.rs:822:19 | method `bind_or_handoff` is never used | NT,JNT |
| src/bin/s_session_v3/audit_owner/native.rs:822:19 | methods `bind_or_handoff`, `bind_initial`, and `handoff` are never used | NP,JNP |
| src/bin/s_session_v3/audit_retention.rs:86:19 | methods `tree_len`, `tree_remove`, and `tree_insert` are never used | NP,NT,JNP,JNT |
| src/bin/s_session_v3/audit_store.rs:105:19 | method `permit_capacity` is never used | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v3/audit_store.rs:352:19 | struct `NativeFieldPath` is never constructed | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v3/audit_store.rs:490:8 | method `native_field` is never used | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v3/aux_index.rs:2481:15 | function `stage` is never used | NP,JNP |
| src/bin/s_session_v3/aux_index.rs:335:8 | method `try_clone` is never used | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v3/business_json.rs:109:12 | method `plain` is never used | NT,JNT |
| src/bin/s_session_v3/business_json.rs:1456:12 | associated function `optional_sql` is never used | SP,ST,NP,NT,JP,JNP |
| src/bin/s_session_v3/construction_set.rs:257:16 | field `previous` is never read | NP,NT,JNP,JNT |
| src/bin/s_session_v3/cross_request_source_cache.rs:632:19 | method `retention_cap` is never used | NP,JNP |
| src/bin/s_session_v3/generation.rs:70:5 | unused import: `newchan_rust::session_protocol::parse_canonical_i64` | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v3/native_shared.rs:114:27 | associated function `try_unwrap` is never used | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v3/native_shared.rs:137:27 | associated function `as_ptr` is never used | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v3/native_shared.rs:239:27 | method `as_ptr` is never used | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v3/native_shared.rs:97:27 | associated function `new` is never used | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v3/object_index.rs:118:19 | methods `text`, `integer`, and `slot` are never used | NP,NT,JNP,JNT |
| src/bin/s_session_v3/object_index.rs:2184:15 | function `stage` is never used | NP,JNP |
| src/bin/s_session_v3/object_index.rs:34:24 | unused import: `BTreeSet` | NT,JNT |
| src/bin/s_session_v3/object_index.rs:34:24 | unused imports: `BTreeSet` and `HashSet` | SP,ST,NP,JP,JT,JNP |
| src/bin/s_session_v3/prepared.rs:1306:4 | function `edge` is never used | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v3/prepared.rs:301:19 | methods `publish_impl` and `publish_impl_admitted` are never used | NP,JNP |
| src/bin/s_session_v3/request_audit.rs:2211:19 | methods `enable_cold_admission_for_testonly`, `prepare_parent`, `cold_parent_budget`, and `bound_parent_budget` are never used | NP,JNP |
| src/bin/s_session_v3/request_audit.rs:2272:19 | method `bound_parent_budget` is never used | NT,JNT |
| src/bin/s_session_v3/request_audit.rs:236:19 | multiple methods are never used | NP,JNP |
| src/bin/s_session_v3/request_audit.rs:2396:19 | associated function `read_optional_bounded_params` is never used | NP,NT,JNP,JNT |
| src/bin/s_session_v3/request_audit.rs:293:19 | struct `ParentPreparation` is never constructed | NP,JNP |
| src/bin/s_session_v3/request_audit.rs:299:19 | methods `allocation_parent` and `observer` are never used | NP,JNP |
| src/bin/s_session_v3/request_audit.rs:438:19 | method `freeze_manifest_workspace_budget` is never used | NP,JNP |
| src/bin/s_session_v3/source_audit.rs:1693:9 | unused variable: `s` | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_session_v3/source_audit.rs:39:48 | unused import: `SourceWork` | NP,JNP |
| src/bin/s_session_v3/source_contributions.rs:405:16 | fields `full_source_calls`, `full_pending_calls`, and `full_members_enumerations` are never read | NP,JNP |
| src/bin/s_structure_session.rs:863:4 | function `parse_batch_value` is never used | SP,ST,NP,NT,JP,JT,JNP,JNT |
| src/bin/s_structure_session.rs:874:27 | unused imports: `CodecRead` and `NativeAdmission` | SP,ST,NP,NT,JP,JT,JNP,JNT |
