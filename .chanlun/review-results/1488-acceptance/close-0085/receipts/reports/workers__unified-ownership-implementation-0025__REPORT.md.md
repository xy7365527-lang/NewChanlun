# 收据 workers/unified-ownership-implementation-0025/REPORT.md

- 原件：`workers/unified-ownership-implementation-0025/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`0a507231629298a3ef664a71d3512cca8b6ea1911bca59eaf1cebfd49b6a4934`
- 字节数：10960；行数：140

## 关键行（行号: 原文）

```
1: # #1488 round 0025 报告——评审 0024 发现修复
4: - 起点 `5759bbec6a`（31 PASS / 4 已知 FAIL）→ 终点 **`b6969af4cc`**（tree `d107c9d6f1394c46db13777c04293676d0900c90`）
5: - 全锁：`/tmp/nc-remote/20261020-005315-b6969af4cc.log`，结果为 **42 PASS / 4 FAIL**。失败的四条都是已知项（accept_tail、incremental、normal_append、write_budget），panic 文案和基线一致。
6: - 宽范围对拍覆盖 v3::audit_owner、request_audit、cross_request_source_cache、fixed_radix、driver 的全部测试，含非锁原测试：
11:   | 终点 b6969af4cc | 201P/20F | 20261020-005513 |
14: - CI：uo-ci-fixed.sh 的 run 为 **39290457765**（结论见文末）。
16: ## 逐条发现
18: ### P1-1 去掉 max 补差，old_pool 改为逐项映射 —— 已修
46: ### P2-1 adopt_native 与 Budget::new(retained≠0) 加门 —— 已修
53: ### P2-2 journal 许可先于分配取得 —— 已修
59: ### P2-3 rebirth 峰值按实计 —— 已修
64: ### P2-4 r14 独立逐许可普查，负控走真实封存 —— 已修
88: ### P3-1 legacy cache 不在已析构的冷根上留许可 —— 已修
94: ### P3-3 96B 常量改由 Layout 推导 —— 已修
99: ### P3-4 BootstrapGrant 仅在冻结入口可构造 —— 已修
104: ## 锁表（本轮新增，全部登记于 .github/1488-uo-locks.txt，终点全部 PASS）
107: | P1-1 | request_audit::handoff_domain_tests::p1_1_same_parent_pool_counts_l0_once_and_items_keep_contract |
108: | P1-1 | request_audit::handoff_domain_tests::p1_1_unmappable_old_pool_is_rejected_without_mutation |
109: | P1-1 | driver::tests::p1_1_driver_warm_old_pool_partition_counts_l0_once |
110: | P2-1 | audit_owner::native::u2_exit_adopt_tests::u2_fresh_parent_codec_vec_grow_never_adopts_allocated_plain |
111: | P2-2 | audit_owner::effects::construction_coverage_tests::p2_2_journal_permit_taken_before_recording_allocation |
112: | P2-2 | audit_owner::effects::construction_coverage_tests::p2_2_private_journal_parent_born_before_allocation_and_material_adds_no_fee |
113: | P2-3 | audit_owner::native::u2_exit_adopt_tests::p2_3_rebirth_vec_peak_counts_old_and_new_blocks |
114: | P2-3 | audit_owner::native::u2_exit_adopt_tests::p2_3_rebirth_codec_vec_peak_counts_old_and_new_blocks |
115: | P2-4 | audit_owner::p2_4_seal_census_flags_duplicate_permit_on_real_seal_path（另有 r14 追加 C1–C5） |
120: ## 未决项
126:    - 如果主控裁定它不属于合同双角色，需要另开票处理，本轮未改业务语义。
131: ## CI 结论（run 39290457765，sha b6969af4cc，分支 codex/1488-uo-ci-20261019）
133: | job | 结论 | 说明 |
135: | `uo (s_session)` | success | 只有 fixed_radix 锁适用（1 PASS），其余 45 条为特征门控的 NOT_LISTED |
136: | `uo (s_session,s_capture_native_testonly)` | failure | **42 PASS / 4 FAIL(101)** |
139: - 本轮新增锁全部 PASS；结果与远端全锁一致。
```
