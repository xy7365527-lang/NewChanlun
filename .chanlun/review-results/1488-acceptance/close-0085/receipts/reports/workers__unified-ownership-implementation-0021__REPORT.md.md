# 收据 workers/unified-ownership-implementation-0021/REPORT.md

- 原件：`workers/unified-ownership-implementation-0021/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`650e19d6b940c7f5a1c87e6d8f5fbe951be4528be9c3677257dd63987fd8159f`
- 字节数：11355；行数：156

## 关键行（行号: 原文）

```
1: # #1488 续轮 0021 报告：U1（工作域内裸分配）
4: - 终点：**8554ad05e78ca0a82d2314da51f09e8b3b4c10d2**（tree 1e34a33978c5764da1c6e08594f394c4cc9b514b）
7: ## 结论
9: U1 没有全部闭合。锁清单 24 条中 18 条 PASS，6 条 FAIL。
25: ## 1. 提交表
41: ## 2. 各触发点根因与修法
50: ## 3. 许可来源 → owner → drop
62: ## 4. 实际写集
71: 三处写集外改动都只涉及许可的持有与接线或账务口径，没有改业务读数、codec、首错/COMMIT 顺序、Recover、资源帽或 Cargo/features。是否认可，交负责人裁定。
73: ## 5. 原断言核对
79: ## 6. 锁结果（快通道，8554ad05e7，日志 /tmp/nc-remote/20261019-201523-8554ad05e7.log，副本 workers/uo-remote-runs/）
83: | audit_owner_complete_accept_tail_begin_recover_retry_and_revision_qualification | FAIL driver.rs:990「Carry租约已撤销」 |
84: | audit_owner_complete_normal_second_connection_and_new_life_are_cold | **PASS**（本轮转绿） |
85: | audit_owner_incremental_normal_writes_full_oracle_and_suffix | FAIL driver.rs:3989「Carry租约已撤销」 |
86: | audit_owner_normal_append_keeps_candidate_below_old_multiplier_gate | FAIL driver.rs:1147「stage缺材料cut、重入或顺序倒退」 |
87: | audit_owner_write_budget_releases_candidate_after_successful_normal_append | FAIL driver.rs:1203「publication暂存资源已失效」 |
88: | audit_owner_staged_normal_append_consumes_actual_nodes_and_suffix | FAIL driver.rs:3752 runtime_root_tables 不等 |
89: | audit_owner_q_itemized_normal_cold_growth_and_typed_limit | PASS |
90: | audit_owner_complete_optional_budget_releases_without_changing_reply | PASS |
91: | r13_cold_candidate_and_q_share_parent_and_preserve_complete_output | PASS |
92: | r13_optional_sql_rows_share_parent_without_changing_cells | PASS |
93: | r14_source_keeper_seal_and_next_request_share_explicit_parent | FAIL audit_owner.rs:3213 |
94: | frozen_request_rejects_foreign_parent_and_expired_domain | PASS |
95: | used_foreign_cold_parent_cannot_be_replaced_by_handoff | PASS |
96: | cross_request_ready_first_write_and_two_requests_preserve_parent | **PASS**（本轮转绿） |
97: | v4 新增 10 条（observer×2、move_in×2、handoff、700+20、bootstrap、warm_prepare、cold_candidate、raw_pending） | 全部 PASS |
101: ## 7. 未闭合项与反例（逐条）
109: - RUNTIME-INDEPENDENT-REVIEW.md【01】已标为「需根裁定」。修复方向是 control/1488-controlled-recover-after-budget-pass-template.md 描述的「受控同 owner Recover 迁移」，那是后续票，而 Recover 在本票禁区内。
141: - 是否修订断言的语义（例如改以 drop 之后的 live 为基准），需根裁定；本票不得改原断言。
147: ## 8. 最终 CI
154: | s_session,s_capture_native_testonly | failure | 通过，无 error | 18 PASS / 6 FAIL(101)，失败集合与快通道 201523 完全一致：r14、accept_tail、incremental、normal_append、write_budget、staged |
```
