# 收据 workers/unified-ownership-implementation-0017/REPORT.md

- 原件：`workers/unified-ownership-implementation-0017/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`f155a4f34fb8086debadd4b4636ab5c4cc63781d642ab3d9d17e3f639dde43ec`
- 字节数：13847；行数：134

## 关键行（行号: 原文）

```
1: # #1488 统一所有权 v4 实施报告（opus-0017）
4: - 终点 commit：`644779c8cd57c2ec1c7b7b89eb85a84919a02a21`
6: - 最后 CI run：**39230999839**（round9，日志 `ci/round9.log`，原件 `workers/uo-ci-runs/39230999839/`）
9: **结论**：v4 账本层（A/B/D/E）的新锁全部通过，R13 也已转绿。原六条 driver、cross_request 和 r14 仍然 FAIL。它们的失败点都在写集外（见 §7），本轮按合同停在该点交回。
11: ## 0. 提交序列（c4e1a38811 之后）
16: | 938e3da66a | ci：cache key 去逗号（issue-1488-uo-iter.yml） | 在 .github 允许范围内 |
18: | 115e87e43c | ci：锁清单追加原 128/1024/R13/handoff 精确测试名 | 在允许范围内 |
27: ## 1. 许可：来源 → 所有者 → 移交 → 释放
39: ## 2. E/O/M/A 划分（T = E + O + A）
50: ## 3. freeze / enable / R13 / CompleteCandidate 调用点及首次分配
69: ## 4. 实际写集
71: v4 本轮（19d4654882..HEAD）：
81: 从 CI 基线（c4e1a38811..HEAD）算，写集外文件只有 a36c252589 和 25333c0f2a 两个编译修复：cache.rs（机械类型适配，不是按合同条件改的）、content_publication.rs、evidence_store.rs、native_shared.rs、source_contributions.rs、source_pending.rs、s_structure_session.rs。.github 下只改了 `issue-1488-uo-iter.yml`（cache key）和 `1488-uo-locks.txt`。
83: ## 5. 原断言逐字核对
85: - `git diff c4e1a38811 HEAD -- rust | grep '^-.*assert'` 结果为**空**：没有删改任何 assert 或 debug_assert。新增 assert 64 行，全部在新测试和新守卫里。
90: ## 6. 锁清单与 run 39230999839 结果（feature 集 `s_session,s_capture_native_testonly`）
94: | driver::audit_owner_complete_accept_tail_begin_recover_retry_and_revision_qualification（原六之一） | FAIL：driver.rs:906 codec 工作域分配失败 |
95: | driver::audit_owner_complete_normal_second_connection_and_new_life_are_cold（原六之一） | FAIL：driver.rs:1224，同上 |
96: | driver::audit_owner_incremental_normal_writes_full_oracle_and_suffix（原六之一） | FAIL：driver.rs:3809，同上 |
97: | driver::audit_owner_normal_append_keeps_candidate_below_old_multiplier_gate（原六之一） | FAIL：driver.rs:1130，同上 |
98: | driver::audit_owner_write_budget_releases_candidate_after_successful_normal_append（原六之一） | FAIL：driver.rs:1191，同上 |
99: | driver::audit_owner_staged_normal_append_consumes_actual_nodes_and_suffix（原六之一） | FAIL：driver.rs:3728，同上 |
100: | audit_owner_q_itemized_normal_cold_growth_and_typed_limit（128） | PASS |
101: | driver::audit_owner_complete_optional_budget_releases_without_changing_reply（1024） | PASS |
102: | r13_cold_candidate_and_q_share_parent_and_preserve_complete_output（R13） | **PASS**（round8 为 FAIL，644779c8cd 修复） |
103: | r13_optional_sql_rows_share_parent_without_changing_cells | PASS |
104: | r14_source_keeper_seal_and_next_request_share_explicit_parent | FAIL：audit_owner.rs:3213 "封存新增许可必须留在冷核同父账" |
105: | handoff_domain_tests::frozen_request_rejects_foreign_parent_and_expired_domain | PASS |
106: | handoff_domain_tests::used_foreign_cold_parent_cannot_be_replaced_by_handoff | PASS |
107: | driver::cross_request_ready_first_write_and_two_requests_preserve_parent | FAIL：driver.rs:1109 codec 工作域分配失败 |
108: | v4：observer_sponsor_counts_in_allowance_and_old_drop_never_refills（B：948、sponsor 128、旧费 drop 不回填） | PASS |
109: | v4：observer_rejects_when_sponsor_exceeds_allowance_without_mutation（B：new200 拒） | PASS |
110: | v4：move_in_reserves_splits_and_returns_rest_without_partial_mutation（A/E） | PASS |
111: | v4：move_in_rejects_legacy_moved_in_without_mutation（E：拒绝时不改 peak） | PASS |
112: | v4：handoff_ends_move_in_and_old_rest_never_refills_new_allowance（E） | PASS |
113: | lifecycle_repair_v2_tests::frozen_increment_accepts_700_old_and_20_new_with_200_allowance（E：old700+new20） | PASS |
114: | v4：bootstrap_grant_is_consumed_exactly_once（A） | PASS |
115: | v4：warm_prepare_binds_real_parent_and_voids_grant（A/B） | PASS |
116: | v4：owner_cold_candidate_born_from_grant_with_move_in（A） | PASS |
117: | v4：owner_raw_pending_state_holds_its_fee_until_drop（D） | PASS |
123: ## 7. 未闭合项
129:   - 这几个文件都不在 6 文件写集内，按合同停在此处，交根裁定是否放开。
132: - **U4：r14 FAIL（写集外）。** audit_owner.rs:3213 "封存新增许可必须留在冷核同父账"，与基线相同。推测是 `drop(audited)` 降 L 的量超过封存增加的量，修复需要动 cross_request_source_cache.rs。r14 是我加进锁清单的，不属于原六条。
```
