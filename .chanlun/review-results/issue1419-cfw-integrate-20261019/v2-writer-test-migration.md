# #1419 裁定②：99 条旧写者测试的逐断言迁移账

> 本账绑定 #1419；原件基线以每行 commit:path:line 为准，当前代码核对基线为 04bbfc799。
> 逐断言原文及当前同文位置见同目录 `v2-writer-test-migration-assertions.json`；索引 ordinal 与下表一致。

## 已核结果

99 条原测试均找回原件：95 条当前仍存在，4 条整函数已删除。提取原函数体内 581 个明文 assert 宏，当前体内 549 个；不包含 helper 内部断言，宏数量不表示行为覆盖率。95 条当前函数体与 b78 已执行快照逐字相同，复用 CI run 40881001192：93 条有 PASS、1 条 ignored、1 条未找到执行行。4 条已删除测试没有当前执行证据。

保留旧测试只能证明它实际调用的入口，不等于已迁至正式 S3。此前按主题重复填入的 S3 候选已撤去；没有逐断言同判据证据的项不签等价。生产 v2 写入口继续拒绝，新写只走 S3；保真锁使用测试种子不构成开放生产写口。

## 有断言差额的七项

- `tb02c_persistent_first_kind_and_normal_recovery`：原 Python tb02c_verify 子进程成功断言移除；余 5 条同文保留。冻结旧串合同通过不代签同输入 S3 原公共验证器。
- `tb02b_persisted_identity_lifecycle_revision_and_old_cuts`：原 Python tb02b_verify 子进程成功断言移除；余 5 条同文保留；公共验证器同输入承接仍需证据。
- `tb02a_three_groups_with_equal_roots_publish_shape_with_root_set`：原 CC-054 域等于 established_direction_with_extreme_root_sets 断言移除；余 13 条保留。W 域须按签署 W 合同核，不把移除旧域断言计为等价。
- `tb1405_composed_s2_recover_rejects_old_rules_before_any_persistent_change`：原 9 规则 × 3 pending 状态（无 raw / 已接纳 / 伪 Begin）的 VersionMismatch、全表不变及当前规则成功恢复矩阵已删。现有 S3 规则门仅部分承接；空 raw 负控不代签该矩阵。
- `tb1405_composed_s2_recover_rejects_old_rule_before_journal_mode_change`：原 DELETE journal 模式、旧规则 VersionMismatch、journal 仍 delete/meta 不变锁已删。当前 S3 返回 VersionUnsupported，错误名与原锁不同，禁止当同文迁移。
- `tb02u_old_cut_explicit_replaces_revision_and_recovery`：原 20 条 assert 随测试删去。新 `projection::tests::w_published_lifecycle_first_known_revision_replaces_and_recovery` 经正式接纳/发布/recover 核首获知、修订身份、逐原件 replaces 及完整 AsKnown；CI 40911916241 的 shard 2 在04bb实际运行，到逐原件替换断言报“定向后组须逐原件替代未定对象：0”（日志 L1806–1807）；此前首获知/修订身份/恢复前三代AsKnown已通过，替换及后续完整历史检查未通过。
- `tb02u_pending_append_keeps_first_known_and_policy_gate`：原 4 条 assert 随测试删去。首获知/零组由上述新生命周期锁承接；旧规则拒写由 `structure_writer_does_not_rewrite_historical_rule_domains` 与 `historical_bi_rule_is_readable_but_writer_and_recover_leave_it_unchanged` 部分承接，非空旧历史及完整旧规则矩阵仍欠。

另有一项新增而非缺失：`real_generations_preserve_first_records_and_emit_only_new_auxiliary_members` 保留原全部断言，增加 4 条布局数组等价断言。JSON 逐条保留新增位置。

## 逐条原件、当前断言与执行证据

原件范围是完整函数；断言原文和逐断言行号在 JSON。保留数按同文 assert occurrence 计；原文完全相同的重复 assert 不据文本集合重复算作不同迁移结论。PASS 均来自 run 40881001192，其原日志归档 `/Volumes/AgentStorage/review1419-A7-29bi/all-ci.log`，下表 L 为该归档行号。

| # | 原测试及原件 | 当前实际函数/断言 | 执行结果 |
|---|---|---|---|
| 1 | `tb02c_persistent_first_kind_and_normal_recovery`<br>`028796ba08:rust/src/bin/s_structure_session.rs:6224–6283` | rust/src/bin/s_structure_session.rs:6437–6483；原 6 / 现 5 宏；有差额，见上节 | PASS；rust-test-s-session (shard 1)；L15232 |
| 2 | `tb02w_killed_case_two_scan_budget_resume_and_recover`<br>`028796ba08:rust/src/bin/s_structure_session.rs:6463–6535` | rust/src/bin/s_structure_session.rs:6663–6735；原 9 / 现 9 宏；原断言同文保留 | PASS；rust-test-s-session (shard 0)；L48329 |
| 3 | `tb02b_persisted_identity_lifecycle_revision_and_old_cuts`<br>`028796ba08:rust/src/bin/s_structure_session.rs:6563–6632` | rust/src/bin/s_structure_session.rs:6763–6823；原 6 / 现 5 宏；有差额，见上节 | PASS；rust-test-s-session (shard 2)；L28869 |
| 4 | `tb02b_same_input_extension_emits_all_changes`<br>`028796ba08:rust/src/bin/s_structure_session.rs:6635–6662` | rust/src/bin/s_structure_session.rs:6826–6853；原 4 / 现 4 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41586 |
| 5 | `tb02b_historical_identity_insertion_opens_new_generation_and_keeps_old_cut`<br>`028796ba08:rust/src/bin/s_structure_session.rs:6665–6724` | rust/src/bin/s_structure_session.rs:6856–6915；原 15 / 现 15 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28682 |
| 6 | `tb02b_same_generation_tail_append_keeps_first_confirmation`<br>`028796ba08:rust/src/bin/s_structure_session.rs:6727–6744` | rust/src/bin/s_structure_session.rs:6918–6935；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41541 |
| 7 | `tb02b_successor_extension_preserves_confirmation_and_relation`<br>`028796ba08:rust/src/bin/s_structure_session.rs:6747–6789` | rust/src/bin/s_structure_session.rs:6938–6980；原 8 / 现 8 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41667 |
| 8 | `tb02b_revision_of_inserted_identity_opens_another_generation`<br>`028796ba08:rust/src/bin/s_structure_session.rs:6792–6820` | rust/src/bin/s_structure_session.rs:6983–7011；原 5 / 现 5 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28834 |
| 9 | `tb02b_batched_history_revision_and_tail_keep_distinct_fact_generation`<br>`028796ba08:rust/src/bin/s_structure_session.rs:6823–6883` | rust/src/bin/s_structure_session.rs:7014–7074；原 9 / 现 9 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28937 |
| 10 | `tb02a_sealed_revision_history_and_independent_fact_index`<br>`028796ba08:rust/src/bin/s_structure_session.rs:6910–6965` | rust/src/bin/s_structure_session.rs:7101–7156；原 9 / 现 9 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41518 |
| 11 | `tb02a_three_groups_with_equal_roots_publish_shape_with_root_set`<br>`028796ba08:rust/src/bin/s_structure_session.rs:6968–7073` | rust/src/bin/s_structure_session.rs:7159–7263；原 14 / 现 13 宏；有差额，见上节 | PASS；rust-test-s-session (shard 3)；L41519 |
| 12 | `tb02a_window_external_direction_revision_disappearance_and_return`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7076–7195` | rust/src/bin/s_structure_session.rs:7266–7385；原 14 / 现 14 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28652 |
| 13 | `tb02a_restored_numeric_shape_has_new_boundary_identity_without_reviving_history`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7198–7262` | rust/src/bin/s_structure_session.rs:7388–7452；原 10 / 现 10 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41510 |
| 14 | `tb02a_raw_sidecar_tampering_is_not_tick_fallback`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7265–7275` | rust/src/bin/s_structure_session.rs:7455–7465；原 1 / 现 1 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41509 |
| 15 | `r9_prefix_observation_retains_sealed_first_version`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7278–7330` | rust/src/bin/s_structure_session.rs:7468–7520；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12785 |
| 16 | `r9_all_required_delta_collections_and_scope_gate_writes`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7333–7381` | rust/src/bin/s_structure_session.rs:7523–7571；原 5 / 现 5 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12803 |
| 17 | `r10_as_known_zero_is_stable_through_accept_publish_and_revision`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7384–7413` | rust/src/bin/s_structure_session.rs:7574–7603；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12783 |
| 18 | `r10_pending_profile_corruption_fails_before_any_write`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7416–7454` | rust/src/bin/s_structure_session.rs:7606–7644；原 5 / 现 5 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28626 |
| 19 | `full_signed_catalog_preserves_both_declared_branch_shapes`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7457–7491` | rust/src/bin/s_structure_session.rs:7647–7681；原 9 / 现 9 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41408 |
| 20 | `complete_batch_is_visible_before_reachable_root_and_never_overwritten`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7494–7565` | rust/src/bin/s_structure_session.rs:7684–7755；原 7 / 现 7 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28355 |
| 21 | `accept_rejects_stale_epoch_without_writing_receipts_or_metadata`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7593–7614` | rust/src/bin/s_structure_session.rs:7783–7804；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41291 |
| 22 | `replay_does_not_rebind_profile_of_an_existing_cut`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7617–7639` | rust/src/bin/s_structure_session.rs:7807–7829；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41454 |
| 23 | `equal_prices_publish_domain_observation_and_corrupt_json_fails_read`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7642–7682` | rust/src/bin/s_structure_session.rs:7832–7872；原 5 / 现 5 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28365 |
| 24 | `source_coord_invalid_input_rolls_back_whole_accept_batch`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7748–7782` | rust/src/bin/s_structure_session.rs:7938–7972；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41456 |
| 25 | `source_coord_collision_rejects_same_batch_and_other_business_identities`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7785–7832` | rust/src/bin/s_structure_session.rs:7975–8022；原 6 / 现 6 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12792 |
| 26 | `source_coord_revision_and_replay_keep_one_owner`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7835–7877` | rust/src/bin/s_structure_session.rs:8025–8067；原 7 / 现 7 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41457 |
| 27 | `source_coord_exact_large_and_sparse_positions_remain_valid`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7880–7906` | rust/src/bin/s_structure_session.rs:8070–8096；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28629 |
| 28 | `source_coord_pending_corruption_blocks_all_root_operations`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7909–7973` | rust/src/bin/s_structure_session.rs:8099–8163；原 4 / 现 4 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12795 |
| 29 | `source_coord_stored_corruption_rejects_before_new_begin`<br>`028796ba08:rust/src/bin/s_structure_session.rs:7976–8016` | rust/src/bin/s_structure_session.rs:8166–8206；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28630 |
| 30 | `correction_revision_chain_withdraws_top_then_rising_then_new_top`<br>`028796ba08:rust/src/bin/s_structure_session.rs:8021–8118` | rust/src/bin/s_structure_session.rs:8211–8308；原 33 / 现 33 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12774 |
| 31 | `native_cold_raw_sql_json_hash_receipt_revision_matches_plain`<br>`028796ba08:rust/src/bin/s_structure_session.rs:8202–8221` | rust/src/bin/s_structure_session.rs:8392–8411；原 0 / 现 0 宏；原断言同文保留 | PASS；rust-test-s-session-extras (native)；L16207 |
| 32 | `native_cold_raw_ohlc_and_revision_matches_plain`<br>`028796ba08:rust/src/bin/s_structure_session.rs:8312–8319` | rust/src/bin/s_structure_session.rs:8502–8509；原 0 / 现 0 宏；原断言同文保留 | PASS；rust-test-s-session-extras (native)；L16206 |
| 33 | `revision_replay_conflict_and_old_version_no_revive`<br>`028796ba08:rust/src/bin/s_structure_session.rs:8322–8355` | rust/src/bin/s_structure_session.rs:8512–8545；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41455 |
| 34 | `out_of_order_late_revision_rejected`<br>`028796ba08:rust/src/bin/s_structure_session.rs:8358–8379` | rust/src/bin/s_structure_session.rs:8548–8569；原 2 / 现 2 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12781 |
| 35 | `writer_epoch_invalid_persistence_rejects_all_writes`<br>`028796ba08:rust/src/bin/s_structure_session.rs:8382–8414` | rust/src/bin/s_structure_session.rs:8572–8604；原 4 / 现 4 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12800 |
| 36 | `writer_epoch_invalid_configuration_does_not_change_state`<br>`028796ba08:rust/src/bin/s_structure_session.rs:8417–8439` | rust/src/bin/s_structure_session.rs:8607–8629；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41590 |
| 37 | `writer_epoch_zero_and_i64_max_are_exact_valid_epochs`<br>`028796ba08:rust/src/bin/s_structure_session.rs:8442–8456` | rust/src/bin/s_structure_session.rs:8632–8646；原 2 / 现 2 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28841 |
| 38 | `writer_epoch_transition_rejects_old_writer_and_recovers_pending_begin`<br>`028796ba08:rust/src/bin/s_structure_session.rs:8459–8495` | rust/src/bin/s_structure_session.rs:8649–8685；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28840 |
| 39 | `watch_returns_ordered_delta_and_gap_detection`<br>`028796ba08:rust/src/bin/s_structure_session.rs:8498–8543` | rust/src/bin/s_structure_session.rs:8688–8733；原 7 / 现 7 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12798 |
| 40 | `financial_basis_pending_distinguishes_commit_history_and_unpublished_revision`<br>`028796ba08:rust/src/bin/s_structure_session.rs:8993–9038` | rust/src/bin/s_structure_session.rs:9183–9228；原 8 / 现 8 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28420 |
| 41 | `financial_basis_pending_detects_active_begin_without_new_raw_input`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9041–9056` | rust/src/bin/s_structure_session.rs:9231–9246；原 2 / 现 2 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12777 |
| 42 | `tb1405_composed_s2_recover_rejects_old_rules_before_any_persistent_change`<br>`7f4f1eeb4b:rust/src/bin/s_structure_session.rs:7832–7914` | 整函数已删除；见上节原义务欠账 | 无当前执行证据 |
| 43 | `tb1405_composed_s2_recover_rejects_old_rule_before_journal_mode_change`<br>`7f4f1eeb4b:rust/src/bin/s_structure_session.rs:7917–7939` | 整函数已删除；见上节原义务欠账 | 无当前执行证据 |
| 44 | `recover_clears_pending_begin_and_keeps_reachable_root`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9059–9099` | rust/src/bin/s_structure_session.rs:9249–9289；原 6 / 现 6 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28627 |
| 45 | `as_known_cut_filters_all_fields_and_no_premature_top`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9101–9185` | rust/src/bin/s_structure_session.rs:9291–9375；原 22 / 现 22 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12773 |
| 46 | `recover_requires_strictly_increasing_epoch`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9188–9230` | rust/src/bin/s_structure_session.rs:9378–9420；原 6 / 现 6 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28628 |
| 47 | `advance_without_new_input_is_idempotent`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9233–9263` | rust/src/bin/s_structure_session.rs:9423–9453；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12771 |
| 48 | `as_of_unpublished_cut_is_rejected`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9266–9285` | rust/src/bin/s_structure_session.rs:9456–9475；原 1 / 现 1 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28352 |
| 49 | `current_snapshot_excludes_unpublished_accepted_revision`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9288–9317` | rust/src/bin/s_structure_session.rs:9478–9507；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12776 |
| 50 | `corrupt_reachable_root_blocks_writes`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9320–9351` | rust/src/bin/s_structure_session.rs:9510–9541；原 2 / 现 2 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41405 |
| 51 | `corrupt_root_variants_block_all_writes`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9354–9435` | rust/src/bin/s_structure_session.rs:9544–9625；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28363 |
| 52 | `interior_delta_missing_blocks_writes_and_reads`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9438–9489` | rust/src/bin/s_structure_session.rs:9628–9679；原 4 / 现 4 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28429 |
| 53 | `bad_last_advance_frontier_blocks_reads_and_writes`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9492–9528` | rust/src/bin/s_structure_session.rs:9682–9718；原 4 / 现 4 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41292 |
| 54 | `future_withdrawn_object_is_rejected_as_corrupt`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9531–9567` | rust/src/bin/s_structure_session.rs:9721–9757；原 2 / 现 2 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12779 |
| 55 | `future_index_rows_block_current_history_and_writes`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9570–9654` | rust/src/bin/s_structure_session.rs:9760–9844；原 4 / 现 4 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28428 |
| 56 | `zero_generation_rejects_published_index_but_allows_pending_input`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9657–9683` | rust/src/bin/s_structure_session.rs:9847–9873；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41592 |
| 57 | `empty_advance_is_valid_frontier_minus_one`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9686–9707` | rust/src/bin/s_structure_session.rs:9876–9897；原 6 / 现 6 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12775 |
| 58 | `wrong_root_tuple_blocks_writes_and_reads`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9710–9783` | rust/src/bin/s_structure_session.rs:9900–9973；原 4 / 现 4 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41591 |
| 59 | `inner_delta_payload_mismatch_blocks_writes_and_reads`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9786–9860` | rust/src/bin/s_structure_session.rs:9976–10050；原 4 / 现 4 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12793 |
| 60 | `large_predecessor_supersedes_and_evidence_wire_are_strings`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9863–9942` | rust/src/bin/s_structure_session.rs:10053–10132；原 4 / 现 4 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12782 |
| 61 | `bigint_source_coords_and_revisions_roundtrip_wire`<br>`028796ba08:rust/src/bin/s_structure_session.rs:9971–10040` | rust/src/bin/s_structure_session.rs:10161–10230；原 12 / 现 12 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41293 |
| 62 | `changes_only_delta_omits_unchanged_objects_and_keeps_first_metadata`<br>`028796ba08:rust/src/bin/s_structure_session.rs:10159–10228` | rust/src/bin/s_structure_session.rs:10349–10418；原 8 / 现 8 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41577 |
| 63 | `changes_only_replacement_keeps_withdrawn_history_and_first_published_families`<br>`028796ba08:rust/src/bin/s_structure_session.rs:10232–10350` | rust/src/bin/s_structure_session.rs:10422–10540；原 15 / 现 15 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41404 |
| 64 | `changes_only_base_snapshot_plus_delta_replays_next_snapshot`<br>`028796ba08:rust/src/bin/s_structure_session.rs:10354–10424` | rust/src/bin/s_structure_session.rs:10544–10614；原 5 / 现 5 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41366 |
| 65 | `changes_only_audit_rejects_missing_birth_and_fake_old_member`<br>`028796ba08:rust/src/bin/s_structure_session.rs:10428–10478` | rust/src/bin/s_structure_session.rs:10618–10668；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28403 |
| 66 | `changes_only_rejects_witness_index_and_delta_drift_from_sealed_source`<br>`028796ba08:rust/src/bin/s_structure_session.rs:10482–10531` | rust/src/bin/s_structure_session.rs:10672–10721；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41403 |
| 67 | `legacy_full_domain_rejects_dropped_redundant_upsert`<br>`028796ba08:rust/src/bin/s_structure_session.rs:10535–10574` | rust/src/bin/s_structure_session.rs:10725–10764；原 2 / 现 2 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41452 |
| 68 | `storage_format_registry_corruption_is_rejected_before_any_write`<br>`028796ba08:rust/src/bin/s_structure_session.rs:10578–10701` | rust/src/bin/s_structure_session.rs:10768–10891；原 14 / 现 14 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41508 |
| 69 | `evidence_graph_is_lossless_public_identical_and_shared`<br>`028796ba08:rust/src/bin/s_structure_session.rs:10706–10784` | rust/src/bin/s_structure_session.rs:10896–10974；原 7 / 现 7 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28601 |
| 70 | `evidence_graph_damage_blocks_every_write_before_it_happens`<br>`028796ba08:rust/src/bin/s_structure_session.rs:10788–10844` | rust/src/bin/s_structure_session.rs:10978–11034；原 4 / 现 4 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28388 |
| 71 | `evidence_graph_publish_rejects_same_id_different_content`<br>`028796ba08:rust/src/bin/s_structure_session.rs:10848–10876` | rust/src/bin/s_structure_session.rs:11038–11066；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41406 |
| 72 | `evidence_graph_downgrade_is_strict`<br>`028796ba08:rust/src/bin/s_structure_session.rs:10880–10988` | rust/src/bin/s_structure_session.rs:11070–11178；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L29776 |
| 73 | `batch_bytes_and_hash_do_not_depend_on_delta_domain`<br>`028796ba08:rust/src/bin/s_structure_session.rs:11015–11068` | rust/src/bin/s_structure_session.rs:11205–11258；原 4 / 现 4 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28419 |
| 74 | `issue1394_same_price_revision_preserves_exact_current_source`<br>`028796ba08:rust/src/bin/s_structure_session.rs:11165–11213` | rust/src/bin/s_structure_session.rs:11355–11403；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28624 |
| 75 | `issue1394_incremental_matches_oracle_on_revision_return`<br>`028796ba08:rust/src/bin/s_structure_session.rs:11216–11276` | rust/src/bin/s_structure_session.rs:11406–11466；原 2 / 现 2 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28776 |
| 76 | `graph3_source_byte_budget_holds_inside_and_after_captures`<br>`028796ba08:rust/src/bin/s_structure_session.rs:11521–11567` | rust/src/bin/s_structure_session.rs:11711–11757；原 5 / 现 5 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41285 |
| 77 | `graph2_domain_stays_read_only_and_mixed_roots_are_rejected`<br>`028796ba08:rust/src/bin/s_structure_session.rs:11693–11751` | rust/src/bin/s_structure_session.rs:11883–11941；原 6 / 现 6 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28343 |
| 78 | `graph3_delta_and_fact_damage_blocks_every_write_before_it_happens`<br>`028796ba08:rust/src/bin/s_structure_session.rs:11756–11843` | rust/src/bin/s_structure_session.rs:11946–12033；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41275 |
| 79 | `tb02u_old_cut_explicit_replaces_revision_and_recovery`<br>`028796ba08:rust/src/bin/s_session_v2/tb02u_tests.rs:14–137` | 整函数已删除；见上节原义务欠账 | 无当前执行证据 |
| 80 | `tb02u_pending_append_keeps_first_known_and_policy_gate`<br>`028796ba08:rust/src/bin/s_session_v2/tb02u_tests.rs:140–171` | 整函数已删除；见上节原义务欠账 | 无当前执行证据 |
| 81 | `v2_observe_batch_rejects_s3_marker_and_keeps_shared_consumption`<br>`028796ba08:rust/src/bin/s_session_v2/mod.rs:2978–2991` | rust/src/bin/s_session_v2/mod.rs:2999–3012；原 2 / 现 2 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12807 |
| 82 | `issue1394_prefix_cost_measurement`<br>`028796ba08:rust/src/bin/s_session_v2/mod.rs:3026–3349` | rust/src/bin/s_session_v2/mod.rs:3047–3370；原 2 / 现 2 宏；原断言同文保留 | ignored；L28844 |
| 83 | `v2_warm_cache_cross_connection_damage_stops_all_writes`<br>`028796ba08:rust/src/bin/s_session_v2/mod.rs:3830–3904` | rust/src/bin/s_session_v2/mod.rs:3851–3925；原 4 / 现 4 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28963 |
| 84 | `v2_pending_recovery_preserves_identity_and_uses_next_attempt`<br>`028796ba08:rust/src/bin/s_session_v2/mod.rs:4248–4278` | rust/src/bin/s_session_v2/mod.rs:4269–4299；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28856 |
| 85 | `v2_exhausted_attempt_plan_has_zero_new_begin_or_write`<br>`028796ba08:rust/src/bin/s_session_v2/mod.rs:4280–4289` | rust/src/bin/s_session_v2/mod.rs:4301–4310；原 2 / 现 2 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L41594 |
| 86 | `v2_fresh_audit_rejects_birth_drift_in_every_cumulative_family`<br>`028796ba08:rust/src/bin/s_session_v2/mod.rs:4291–4313` | rust/src/bin/s_session_v2/mod.rs:4312–4334；原 5 / 现 5 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28854 |
| 87 | `v2_same_generation_pending_damage_is_not_hidden_by_previous_healthy_audit`<br>`028796ba08:rust/src/bin/s_session_v2/mod.rs:4315–4338` | rust/src/bin/s_session_v2/mod.rs:4336–4359；原 3 / 现 3 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12811 |
| 88 | `v2_all_control_rows_and_pending_message_coverage_gate_every_write`<br>`028796ba08:rust/src/bin/s_session_v2/mod.rs:4351–4372` | rust/src/bin/s_session_v2/mod.rs:4372–4393；原 1 / 现 1 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12804 |
| 89 | `v2_epoch_history_and_recover_phase_are_one_complete_chain`<br>`028796ba08:rust/src/bin/s_session_v2/mod.rs:4374–4401` | rust/src/bin/s_session_v2/mod.rs:4395–4422；原 1 / 现 1 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L28845 |
| 90 | `v2_corrupt_control_or_pending_rows_block_all_writes`<br>`028796ba08:rust/src/bin/s_session_v2/mod.rs:4403–4452` | rust/src/bin/s_session_v2/mod.rs:4424–4473；原 7 / 现 7 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L12805 |
| 91 | `reset_route_refuses_new_domain_and_keeps_legacy_paths`<br>`028796ba08:rust/src/bin/s_session_v3/mod.rs:1365–1411` | rust/src/bin/s_session_v3/mod.rs:1476–1522；原 9 / 现 9 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L42419 |
| 92 | `reset_refuses_tampered_or_partial_new_domain_markers`<br>`028796ba08:rust/src/bin/s_session_v3/mod.rs:1414–1479` | rust/src/bin/s_session_v3/mod.rs:1525–1590；原 8 / 现 8 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L29665 |
| 93 | `reset_refuses_undecidable_files_and_keeps_bytes`<br>`028796ba08:rust/src/bin/s_session_v3/mod.rs:1482–1515` | rust/src/bin/s_session_v3/mod.rs:1593–1626；原 7 / 现 7 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L42418 |
| 94 | `reset_does_not_delete_a_database_owned_by_another_writer`<br>`028796ba08:rust/src/bin/s_session_v3/mod.rs:1560–1568` | rust/src/bin/s_session_v3/mod.rs:1666–1674；原 2 / 现 2 宏；原断言同文保留 | PASS；rust-test-s-session (shard 1)；L13354 |
| 95 | `old_write_and_read_entries_refuse_the_new_domain`<br>`028796ba08:rust/src/bin/s_session_v3/mod.rs:1668–1689` | rust/src/bin/s_session_v3/mod.rs:1774–1795；原 7 / 现 7 宏；原断言同文保留 | PASS；rust-test-s-session (shard 3)；L42417 |
| 96 | `oracle_historical_revision_parity`<br>`028796ba08:rust/src/bin/s_session_v3/generation.rs:5031–5094` | rust/src/bin/s_session_v3/generation.rs:5391–5454；原 4 / 现 4 宏；原断言同文保留 | PASS；rust-test-s-session (shard 2)；L29212 |
| 97 | `real_generations_preserve_first_records_and_emit_only_new_auxiliary_members`<br>`028796ba08:rust/src/bin/s_session_v3/aux_index.rs:3577–4027` | rust/src/bin/s_session_v3/aux_index.rs:3584–4079；原 33 / 现 37 宏；原断言全保留，另新增 | PASS；rust-test-s-session (shard 2)；L28965 |
| 98 | `serve_grammar_has_no_consistency_choice`<br>`028796ba08:rust/src/bin/s_session_v3/financial_session_tests.rs:1011–1052` | rust/src/bin/s_session_v3/financial_session_tests.rs:1011–1052；原 7 / 现 7 宏；原断言同文保留 | 无当前执行证据 |
| 99 | `native_raw_ledger_real_multicut_matches_full_source_audit`<br>`028796ba08:rust/src/bin/s_session_v3/source_audit.rs:3357–3440` | rust/src/bin/s_session_v3/source_audit.rs:3388–3472；原 9 / 现 9 宏；原断言同文保留 | PASS；rust-test-s-session-extras (native)；L17480 |

## Python / launcher 的独立承接

#1768 owner 报告 `/Volumes/AgentStorage/review1768-repair29bl/REPORT.md`：42 历史方法 PASS；它们验证冻结旧会话读取。3 个活 reader 的 g4→g5/g7 动态提交仍报 E6 UnsupportedWalShmJournalCombination，不能改静态 fixture 或删侧件替代；Python memo 表示域由该 owner 处理。本账不代签这些独立义务，也不再建议重开 v2 seed 生产写口。

## 使用本账验收

先用下表定位原函数，再逐项读 JSON 的 assertions（原 line / assertion / current_line；null 表示无当前同文断言）及 added_assertions。当前函数全文与调用入口共同决定覆盖范围。正式 S3 的新锁须有自己的执行证据；旧保真锁 PASS、按主题相近或同一个 helper 都不能自动提供该证据。
