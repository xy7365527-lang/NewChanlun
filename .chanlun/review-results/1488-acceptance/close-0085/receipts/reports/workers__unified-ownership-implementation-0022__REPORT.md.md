# 收据 workers/unified-ownership-implementation-0022/REPORT.md

- 原件：`workers/unified-ownership-implementation-0022/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`ab4c6cce7b8a580f524eb48e508d765a811677df063819e864663a819bfc43ba`
- 字节数：14948；行数：149

## 关键行（行号: 原文）

```
1: # #1488 续轮 0022 报告：staged / U2 / MoveIn / r14 / effects key
4: 最终 HEAD **ec9456a4409d23cf3b3f4bd46d87525521c20b4a**，tree **e60b9a236f90571d24d582bc9bcaa9a888bb5545**。
7: ## 结论
10: - 步骤 2 U2：五处追认点退出生产路径，新增 10 条正/负锁，全部 PASS。
15: - 最终快轨与 CI 结果一致：**30 PASS / 4 FAIL**。剩下的 4 条是 accept_tail、incremental（Recover，票面禁止碰）以及 normal_append、write_budget（帽/粒度，归用户裁）。
17: ## 1. 提交表
34: ## 2. 各触发点根因与修法
44: ## 3. 许可来源 → owner → drop
57: ## 4. 实际写集（相对 8554ad05e7）
73: ## 5. 原断言核对
75: - `git diff 8554ad05e7 HEAD` 删除的断言只有三处：
81: ## 6. 锁结果
83: 快通道日志：`/tmp/nc-remote/20261019-222341-ec9456a440.log`（副本在 workers/uo-remote-runs/）。CI run 39268783566 的 native 组与之逐条一致。
87: | audit_owner_complete_accept_tail_begin_recover_retry_and_revision_qualification | FAIL driver.rs:990「Carry租约已撤销」 |
88: | audit_owner_incremental_normal_writes_full_oracle_and_suffix | FAIL driver.rs:3989「Carry租约已撤销」 |
89: | audit_owner_normal_append_keeps_candidate_below_old_multiplier_gate | FAIL driver.rs:1147「stage缺材料cut、重入或顺序倒退」 |
90: | audit_owner_write_budget_releases_candidate_after_successful_normal_append | FAIL driver.rs:1203「publication暂存资源已失效」 |
91: | audit_owner_staged_normal_append_consumes_actual_nodes_and_suffix | **PASS**（步骤 1 转绿） |
92: | r14_source_keeper_seal_and_next_request_share_explicit_parent | **PASS**（步骤 4 转绿） |
93: | audit_owner_complete_normal_second_connection_and_new_life_are_cold | PASS |
94: | cross_request_ready_first_write_and_two_requests_preserve_parent | PASS |
95: | audit_owner_q_itemized_normal_cold_growth_and_typed_limit | PASS |
96: | audit_owner_complete_optional_budget_releases_without_changing_reply | PASS |
97: | r13_cold_candidate_and_q_share_parent_and_preserve_complete_output | PASS |
98: | r13_optional_sql_rows_share_parent_without_changing_cells | PASS |
99: | frozen_request_rejects_foreign_parent_and_expired_domain | PASS |
100: | used_foreign_cold_parent_cannot_be_replaced_by_handoff | PASS |
101: | v4 的 10 条（observer×2、move_in×2、handoff、700+20、bootstrap、warm_prepare、cold_candidate[改名]、raw_pending） | 全部 PASS |
102: | U2 新增 10 条（material×2、q raw×2、effects rows×2、effects row×2、history 前缀×2） | 全部 PASS |
106: ### 最终 CI（run 39268783566，sha ec9456a440）
111: | s_session,s_capture_native_testonly | failure | 通过，0 error | 30 PASS / 4 FAIL(101)，失败集合与快通道 222341 一致 |
113: 前一轮 CI run 39267097043（2b9b316454）的 s_session 组编译失败，即 T5，已由 ec9456a440 修复。
115: ## 7. 未闭合项（逐条）
```
