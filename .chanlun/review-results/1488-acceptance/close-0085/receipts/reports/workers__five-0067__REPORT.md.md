# 收据 workers/five-0067/REPORT.md

- 原件：`workers/five-0067/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`80f9be99372688d151c8ac13fd84a02467449a4628d145ecef0bdbd856aef5d4`
- 字节数：11992；行数：93

## 关键行（行号: 原文）

```
1: # #1488 0067 REPORT：宽对拍 v3:: 另 5 件本票引入失败清零
3: 工位 five-0067-checkout，分支 codex/1488-five-0067-20261021，起点 665b49e689，交付 HEAD 3c99f03ca8。基线口径：相对 main（6badaa6049）。NC_TARGET=target-f67。
6: ## 1. 五件结论
10: | 1 | `v3::aux_index::tests::native_aux_failed_table_construction_releases_partial_owners` | (c) | 夹具把父账填到 `limit-baseline`，连 `codec_work` 开域所需的 `CodecWork::metadata_bytes()` 都无位，`BudgetExceeded` 在到达被测表构造前就发生；被测路径根本没跑 | 本票新增（546b08740b/ea41d4dfd1），首个可编译 5cb7253d46 即红 | 3c2615cdae：占位改为 `limit - basel
11: | 2 | `v3::aux_index::tests::r12_cold_object_aux_shared_scope_matches_complete_plain_outputs` | (a) | 探针定位拒绝点：`content_store.rs` `capture_originals_admitted` 每插入一个新原件就 `reserve_native(node × depth)` 并累积在 `tree_permit` 里不释放（总量 ≈ n·node·log n）；父 native_live=67098435 时一笔 12480 字节 Native 被拒（remaining=10
12: | 3 | `v3::control_audit::tests::r12_native_control_cold_matches_plain_published_facts_on_parent_scope` | **(b) 待裁** | 断言编码 main 的“许可保守留在父账到父作用域结束”旧模型（`NativeReservation::retain`）；本票 959d860065 起按 v4 L9 许可随对象析构返还，旧模型被删除。实测 before=361、消费后=4359、finish 后=361、离域=128（=父元数据）。见 §2 | c2c9c5640a 绿 → 5cb7253d
13: | 4 | `v3::object_index::tests::native_plan_owners_outlive_scope_and_match_independent_change_rows` | (c) | oracle 按 old 迭代序拼 withdrawals，与测试注释自述的“旧公开顺序 shape 在前”相反。与 plain 路径对拍：`plan_from_candidates`（无工作域）与 `changes_at`（持久索引读）两场景均为 `[shape-first window_removed, typed-z superseded_by_revision]` = na
14: | 5 | `v3::prepared::tests::r10_native_manifest_workspace_shared_scope_normal_commits_match_full_rows` | (c) | 夹具每代新建一个 `Budget` 父；warm `GenerationRuntime` 的 event_node FixedRadix memo（ea41d4dfd1 引入，绑首个父）在 `fixed_radix.rs:244 check_admission` 对 gen2 的新父报 `Ledger`（wrong-parent）。v4 L6“同一生产路径只有一个真实分配父和
16: ### 披露与观察
21: ## 2. 待裁 (b)：#3 control_audit r12_native_control_cold
47: 生产头（不变码）上拟改写 PASS：20261021-021119-f3fea5d692（before=361 after=4359 finished=361 out_of_scope=128）。
59: ## 3. 全锁 + 宽对拍（HEAD 3c99f03ca8）
69: ## 4. 提交清单（665b49e689..HEAD，只追加）
91: ## 证据引用迁移（0077 P30，2026-10-21 追加）
93: 本文原引用的 6 份 `/tmp/nc-remote/<run-id>.log` 已改为仓内收据 `.chanlun/review-results/1488-acceptance/receipts/<run-id>.md`（相对工位 checkout 根；关键行 + 整文件 sha256，原日志名与 run-id 一一对应）。索引见 `.chanlun/review-results/1488-acceptance/receipts/INDEX.md`。
```
