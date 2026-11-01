# PR1693 第二轮 main 集成

集成候选为 `2fee8af14fdcdb317024b0a201bb36ea85ee3063`。父一旧候选为 `9f13b8efac9231eadf5ff755ca3943b21d8200a2`，吸收 main `40f11b540f02666a854bcf48e5ee259ff88af8bb`。本轮只负责集成和靶向验证，非作者独审与精确候选 CI 归主控后续消费。

## 冲突与实际解法

本地工位 `/Volumes/AgentStorage/issue1693-integration-20261026/worktree` 原 clean、HEAD `3e587ab2d05478cbe721d030565eeeb89617a76f`。查询进程和 lsof 未见该工位消费者，未见其他1693写者。先 fast-forward 到远端已审候选，再普通 merge origin/main。没有 rebase、force 或改 PR base。

手工冲突只有两文件。

- `rust/src/theta_v0/policy_records.rs` 保留 trend-scope、tower-base 两份记录常量，同时保留 main 的 arm-sub-expiry id 和 include。没有二选一丢失记录。
- `rust/src/trading/positional.rs` 保留 main 新入口的 `&ReplayRecords`、结果 `records` 标签及测试夹具入口。将记录继续传入原 trend-scope helper，由其读取 `q_total` 与 `op_ladder`；Fusion 使用经原 profile 校验的趋势值，并写回 `trend_scope_policy`。没有重引生产资金或操作层默认常量。

两父写集交集以 merge-base `9b66f8a7acc85402f15fc9918628a86439d7ddb3` 为基准，方法为两侧 `git diff --name-only` 后 `comm -12`。交集为 ADR0035、lib.rs、policy_records.rs、positional.rs、positional_fusion.rs。自动合并三处也已检查。

- ADR0035 相对新 main 只保留 #1626 的 tower_base_ref 段，main 新 Q6 方法未覆盖。
- lib.rs 同时保留 main 的 funding_profile/records/q_total 和 #1626 的 trend_scope_policy 输出。
- positional_fusion.rs 保留 main 的 `op_ladder/q_total` 形参、行为与 #1626 的趋势记录输出测试。

旧候选至最终候选的 formal/、recursive_tower.rs、config.rs、trend-scope.json、tower-base.json 零差异。既有 Lean 与塔底语义审查可以复用。

## 新增交叉回归

`positional.rs:1950` 的 `fusion_preserves_funding_and_trend_scope_records_together` 在三个 Fusion profile 经公开 `run_positional` 入口验证 500 测试资金、完整三记录标签和 trend-scope/1 同时保留。
`positional.rs:1973` 的 `fusion_rejects_record_op_ladder_without_fixture_fallback` 验证实际签署记录给出的 op_ladder=2 仍按既有 Fusion 边界拒绝，不偷换旧夹具操作层。

## 验证进展

行为 merge commit `2554f669bd9923fbb4bc0fee53d86defaef04dd6` 已在 nc-remote 验证。日志 `/tmp/nc-remote/20261026-103749-2554f669bd.log` 绑定完整 SHA，positional 20 passed、funding_record 6 passed、policy_records 12 passed，均0 failed，EXIT=0。两新增测试实际命中通过。该命令最初 rustfmt --check 报一处测试换行，已单独提交 `2fee8af14f` 修正，纯格式无行为差异；不把首轮 fmt 写成通过。

最终候选 `2fee8af14fdcdb317024b0a201bb36ea85ee3063` 的远端日志为 `1693-main-integration26b-validation/20261026-104044-2fee8af14f.log`，首行绑定完整 SHA。命令用 `set -e`，两冲突 Rust 文件的 rustfmt --check 通过后继续执行；Fusion 78 passed，BSP 白名单真实测试1 passed，cargo check --lib 成功，EXIT=0。首轮38项行为测试使用格式前候选；最终候选只改一处断行，完整 diff 已核，因此复用这组行为证据，不重复跑它们。

两份远端日志已逐字保存到本报告同目录的 `1693-main-integration26b-validation/`，不只留 `/tmp`。生产 Rust 净差异的 `git diff --check 40f11b540f HEAD -- rust/src` 通过。整个 merge 相对父一的 diff-check 会指出 main 已归档的 `atomic-init-tests-nocapture-73bbf098d9.log:6` 尾空格，历史原件保留，不把这条写成无告警或修改原件凑绿。

## 已发布与下一步

普通 push `HEAD:refs/heads/sandcastle/issue-1626` 已成功。GitHub 现场回读 PR1693 OPEN、base=main、head=`2fee8af14fdcdb317024b0a201bb36ea85ee3063`，未改 Draft、base 或任何审批状态。回读发生在 `2026-10-26T02:43:55Z` 前，当时新 head check 列表为空，mergeable/mergeStateStatus 均 UNKNOWN，属于刚推送后的状态，不能称 CI 通过。

主控下一步将该候选交给 Opus 非作者尾审，重点核两处冲突、`positional.rs` 的资金记录与趋势记录组合及两新增回归。最终候选后续 CI 须另消费。已有 main 授权不替代本新增差异独审与新 CI。

工位 branch 为 `codex/1693-main-integration-20261026`，本地 HEAD 与远端 PR head 相同，工作树 clean。保留工位供独审只读与必要返修。不合 main、不关闭 #1626，不改共享 ledger/units。

本轮采用 Prove It Works，核实际 merge 冲突、自动合并交集、真实 nc-remote 行为日志与远端 head，而不是依据旧 CI 绿灯判定新集成完成。另按 resolving-merge-conflicts 保留两父意图，没有发明新的资金或趋势政策。
