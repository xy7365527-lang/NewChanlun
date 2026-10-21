# #1488 宽对拍基线即失败名单（16 条）

来源：#1488 宽对拍（`cargo test --features s_session,s_capture_native_testonly --bin s_structure_session` 过滤 v3::{audit_owner,request_audit,cross_request_source_cache,fixed_radix,driver}，WSL nc-remote）。
- 冻结源 `fff166540e355fe09c03ee441894b9e82265a59c`：235 过 / 16 败 / 5 ignored，日志副本 `1488-wide-fff166540e.log`（原 /tmp/nc-remote/20261020-112302-fff166540e.log）。
- 预算段冻结源 `62bad42a1b`：209/20（多出的 4 条已在 #1488 修复：accept_tail、incremental、normal_append、write_budget），日志副本 `1488-wide-62bad42a1b.log`。
- 非作者终审 0045（/Volumes/AgentStorage/issue1488-ticket-manager-20261018/workers/review-0045/RESULT.json evidence.wide_failures）逐条标为“基线既有”：同名同 panic 在 #1488 改动前即失败；更早基线 5759bbec6a（190P/20F）同。
- 合并 main 后（57f9680c4e / c2bdf9b662）同 16 条仍失败（merge-0047 报告），#1488 不修这些。

- `v3::audit_owner::construction_tests::owner_set_real_publish_commit_failure_revokes_everything` — src/bin/s_session_v3/audit_owner.rs:5320:14: | called `Result::unwrap()` on an `Err` value: "prototype_audit_allocation_incomplete：Native文本复制需要当前WorkScope"
- `v3::audit_owner::construction_tests::owner_set_real_publish_stage_effects_commit_promotes` — src/bin/s_session_v3/audit_owner.rs:5320:14: | called `Result::unwrap()` on an `Err` value: "prototype_audit_allocation_incomplete：Native文本复制需要当前WorkScope"
- `v3::audit_owner::construction_tests::owner_set_real_publish_unscoped_same_value_update_rejects` — src/bin/s_session_v3/audit_owner.rs:5320:14: | called `Result::unwrap()` on an `Err` value: "prototype_audit_allocation_incomplete：Native文本复制需要当前WorkScope"
- `v3::audit_owner::effects::construction_coverage_tests::owner_set_exact_work_cap_lives_through_effects_then_releases_only_its_charge` — src/bin/s_session_v3/audit_owner/effects.rs:3024:9: | 原codec retain仍在同一个父账中
- `v3::audit_owner::effects::construction_coverage_tests::owner_set_failed_source_drops_new_cap_without_erasing_existing_codec_retain` — src/bin/s_session_v3/audit_owner/effects.rs:3053:9: | 原codec retain仍由原父账持有
- `v3::audit_owner::effects::e_owner_move_tests::e_apply_native_insert_is_ordered_and_releases_only_manual_outer_charge` — src/bin/s_session_v3/audit_owner/effects.rs:3161:9: | assertion `left == right` failed
- `v3::audit_owner::effects::e_owner_move_tests::e_apply_native_replace_and_delete_keep_remaining_rows_owned` — src/bin/s_session_v3/audit_owner/effects.rs:3227:13: | assertion `left == right` failed
- `v3::audit_owner::effects::e_owner_move_tests::e_stage_into_row_keeps_charge_after_header_drop` — src/bin/s_session_v3/audit_owner/effects.rs:3094:9: | assertion `left == right` failed
- `v3::audit_owner::q::actual_nonempty_native_q_accept_preserves_full_raw_without_manual_double_charge` — src/bin/s_session_v3/audit_owner/q.rs:1033:5: | assertion `left == right` failed
- `v3::cross_request_source_cache::tests::r11_source_binding_new_and_seal_use_shared_scope` — src/bin/s_session_v3/cross_request_source_cache.rs:1074:17: | 新trial完整原件绑定须消费父域许可
- `v3::cross_request_source_cache::tests::r13_source_trial_cold_hit_miss_record_and_seal_use_shared_scope` — src/bin/s_session_v3/cross_request_source_cache.rs:1383:13: | assertion `left == right` failed: 保守费用保留在父账
- `v3::cross_request_source_cache::tests::r14_source_pool_normal_growth_and_sparse_cap_match_none` — src/bin/s_session_v3/cross_request_source_cache.rs:1205:18: | called `Result::unwrap()` on an `Err` value: Resource(Ledger)
- `v3::driver::tests::source_keeper_driver_append_repost_restart_preserves_results_and_request_reuse` — src/bin/s_session_v3/driver.rs:3088:9: | 追加请求须复用已核旧cut
- `v3::request_audit::r_owner_move_tests::r_optional_bad_sql_precedes_scope_and_rejected_read_refunds` — src/bin/s_session_v3/request_audit.rs:2555:39: | called `Result::unwrap()` on an `Err` value: BudgetExceeded
- `v3::request_audit::r_owner_move_tests::r_sql_row_pop_and_blob_move_keep_exact_parent_charge` — src/bin/s_session_v3/request_audit.rs:2530:9: | assertion `left == right` failed
- `v3::request_audit::tests::r11_request_native_shared_scope_cold_warm_append_full_output_equal` — src/bin/s_session_v3/request_audit.rs:1326:21: | legacy retain is still explicitly present

## 主控（owner_1588）给 tests 分片的指示（2026-10-20 追加，依据 #1488 负责人回复）

- 这 16 条所在文件（audit_owner.rs、audit_owner/{effects,q,native}.rs、cross_request_source_cache.rs、request_audit.rs、driver.rs、construction_state.rs）已被 #1488 大改（预算 v4、Recover、journal 合并，约 260 提交，将 squash 合入 main）；多条失败本身是 #1488 预算模型下的断言，在 main 上的形态不同。
- **不要在 origin/main 基础上修这 16 条。** 做法：先在 origin/main 上用 nc-remote 复现确认它们在 main 是否同样失败（记录结果）；然后从 #1488 合并面起独立分支 `eng-debt-1588/tests-1488`（基 origin/codex/1488-budget-full-chain-fix-v2-20261018 当前头；工位 /Volumes/AgentStorage/eng-debt-20261020/tests-1488），在其上逐条做根因与修复；PR 目标仍为 main 但在 PR 正文写明「依赖 #1488 先合入，合入后 rebase」，或若根因判断为需裁定则转票。
- 必须避开 #1488 当前在改的文件：content_producer.rs、content_bi.rs、content_manifest.rs、aux_index 相关、generation.rs、state.rs、prepared.rs、driver.rs 测试模块、.github/1488-uo-locks.txt。若某条修复必须动这些文件，不改，写入报告由主控协调。
- B（financial_session WSL）与 C（m3）照原任务在 eng-debt-1588/tests（基 origin/main）做，与 A 分开 PR。
