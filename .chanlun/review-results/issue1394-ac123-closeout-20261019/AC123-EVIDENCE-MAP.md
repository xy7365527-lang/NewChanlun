# #1394 AC1–AC3 证据映射（拆票后本票留存部分，2026-10-19）

名分：#1394 工作草稿，接续负责人 /root/owner_1394b 编制；不是关票声明。AC4/AC5 已迁 #1552。

## 被映射的 source

- 分支 `codex/1394-integration-20261013`，HEAD `b46ba29699`（即「b46」）。相对 origin/main `9c85815880`：领先 271 提交、落后 28 提交；`git merge-tree --write-tree origin/main b46ba29699` 无冲突（tree `94875206ca`）。
- CI run 38516525815（2026-10-14，headSha b46ba29699）：test/s-observer/macos-session/fixture-drift 成功；rust-check 失败仅为 `theta_v0::backtest` 三条（l3_delta_r_alpha m3_partition_synthetic_holds、tick calibration_sweep_finds_non_collapsed_segments、tick seam_btc_tick_bars_yield_non_trivial_structure），main 同时期同样失败，已由 #1520 追踪，与财务无关。
- `economic_session::finance::tests` 14 项在 rust-check(Linux) 与 macos-session 各跑一遍：每个作业 14 ok，另 `durable_child`（SIGKILL 测试的子进程入口）按设计 ignored。

## 逐条映射

| AC | 原文要点 | 测试（rust/src/economic_session/finance_tests.rs 行） | 运行证据（.chanlun/review-results/issue1394-financial-owner-20260930/EVIDENCE.md） |
|---|---|---|---|
| AC1 先 E 增责任才允许 B Committed；失败/重复/旧版本保原权利和状态 | 正向与负向 | 新增 commit_before_e_reserve_is_rejected_and_keeps_prepared、commit_while_slot_awaits_policy_is_rejected_without_winner（分支 `codex/1394-ac123-closeout-20261019` @ `b333d252c5`）；:302 all_fit_actual_e_grants_commit_and_immutable_history；:418 stale_grant_first_commit_blocked_by_new_uncovered_pending_and_receipt_not_rewritten；:450 identity_cas_stale_epoch_and_missing_evidence_preserve_locks；:702 version_cas_and_incomplete_slot_rejections_leave_old_responsibility；:726 closed_generation_rejects_commit_and_recovery_knowledge_is_never_backdated；:505 abort_and_closed_applicant_keep_reserved_liability；:583 transaction_before_after_sigkill_recovery_two_fresh_runs_each | 「新 /2 SIGKILL 矩阵」6 操作×before/after×两鲜进程 24/24；「正式CLI/Unix」Prepareα/β→E Reserve→B Commitα/β、同 ID 重发，责任 90、locked 30/40；「最终COMMIT真实磁盘满」原像与公责不变 |
| AC2 实际运行最小 E 财务步骤，不伪造 grant、不内部赋值取得 Committed | 实际 E 路径 | :302；:384 reserve_persists_resolved_pending_and_keeps_new_uncovered_token_across_restart；:521 altered_original_receipts_or_message_binding_fail_full_replay | 「正式CLI/Unix」三个 serve 进程经单 LF+EOF Unix 请求；EVIDENCE 明记「实际金融步骤没有伪造 grant或手工置 Committed」；Book/Voice/20 责任 setup 为声明的 TestOnly 前置 |
| AC3 共同满足的具名受测域正常提交；不足且无资金政策给精确等待；FIFO 不代资金排序 | 正向+等待 | 新增 commit_while_slot_awaits_policy_is_rejected_without_winner；:302（all_fit → Reserved/Committed）；:342 insufficient_cap_waits_for_exact_policy_without_selecting_winner（AwaitingAllocationPolicy，waiting_for=`#1349/atomic-slot-wait/v1`，finance.rs:920–923「不得让其他 slot 插队」）；:358 covered_pending_without_applied_does_not_block_healthy_beta_reserve；:485 unreadable_reserved_alpha_does_not_block_healthy_beta_but_unknown_e_does | 条件 FIFO/稳定点归已裁 #1416（EVIDENCE.md:67），本片只证明「调度 FIFO 不代资金排序」与精确等待 |

## 边界（照实）

- 资金数值未签：#1349/#1355/#1357 未签前只能 AwaitingPolicy/NotEvaluable，本映射不以默认值代填。
- 不覆盖：统一 source（#1488）上的复证、正式会话入口浏览器核对（AC4）、独立 oracle 新进程重放与持久恢复对拍（AC5）——全部归 #1552。
- 本映射未在本机运行 cargo；测试结果全部来自上列 CI run。

## 独审回流（REVIEW-AC123-01，issuecomment-6262742173，PASS）

- M1 采纳：AC1–AC3 证据对应 **b46（`b46ba29699`）**，不是 b46+C1；C1 `99e819192b` 只改 S 侧 financial_reader.rs，不碰财务判定。
- M2 照记：b46 单测跑在 /1 版本域，/2 端到端证据产生于更早提交；判定代码两版共用。/2 复证归 #1552。
- LOW-1 采纳：AC2 不再引 :836（只测期限计算）。
- LOW-2 采纳：补两条负向测试（见上表），CI 验证 run 39221888630。
- LOW-3 照记：单测直调 persist，绕过 write 自组证明；新测试同时断言 write 路径被拒，write 自组证明的正向路径仍由正式 CLI/Unix 运行证据覆盖。
- 「另一 slot 插队」负向需双 slot 夹具，未补；finance.rs:920–923 的拒绝分支仍只有代码阅读证据。

## CI 验证（新负向测试）

run 39221888630（workflow_dispatch，HEAD b333d252c5）：macos-session 作业 economic_session 库测试 77 passed / 0 failed，其中 finance::tests 16 ok（原 14 + 新 2）；test、s-observer、fixture-drift 成功；rust-check(Linux) 本记录时仍在跑。

REVIEW-PRODUCT-02.md 为读侧适配器 29cf83564d 独审原报告；其测试未运行一节由接续负责人补跑：77 项普通/-O OK，run_checks.py 268/268 普通/-O（见 #1394 issuecomment-6262851762）。
