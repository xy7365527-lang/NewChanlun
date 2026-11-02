# #1669 变异负控记录

两个变异体都只建在本地分支上，用 nc-remote 在远端跑 `cargo test --lib --features s_session -- tb03c_book_tests`，跑完即删分支，没有推送。

| 变异 | 改动 | 结果 |
|---|---|---|
| 1 | `rust/src/economic_session/tb03c_book.rs:608` `rank = if n.long { 2 } else { 1 };` → `rank = 2;`（空头 Eligible 也推进到 EarningShares） | `short_eligibility_implies_capital_recovered_and_never_earning_shares` FAILED：「空头事件把 latch 推进到 EarningShares seed=1」；其余 7 项 ok |
| 2 | `:568` `let status = n.status()?;` → 空头 latch 离开 CostReduction 即判 Eligible（资格读 latch） | 同一测试 FAILED：存储资格 `Eligible` ≠ 谓词重算 `NotEligible`；其余 7 项 ok |

两次运行里原有的 `eligibility_implies_latch_and_latch_never_regresses` 都保持绿色，说明拦下这两个变异的是新增的空头锁。

未变异基线：同一命令 8 passed / 0 failed（`cargo fmt --check` 通过）。
