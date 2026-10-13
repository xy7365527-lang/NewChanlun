# #1352 受影响代码清单（完整退出授权与责任退休 policy）

> 票：[#1352](https://github.com/xy7365527-lang/NewChanlun/issues/1352)；裁定正本：[ADR 0030](../../../docs/adr/0030-exit-authorization-and-liability-retirement.md)。
> 基线：**main `465174240b989a1d99f99a047fbb46b3ee215f49`**（标 main）；#1394 财务线 **FIN `0d8a6017231348e2c4a005f68d4e2e7a457a59fe`**（`codex/1394-transport-revision-20261012`，未合入 main，标 FIN）。行号于各自基线逐行核过。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归下表「归属」列的实施票，本清单本身不改任何代码。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/economic_session/book.rs:334` | main `465174240b` | 现状保留：缺开局/family 不符记 Unmatched，不挂新 Voice；加带票号注释；重开协议归 #1397 | #1397 |
| Rust | `rust/src/economic_session/book.rs:342` | main `465174240b` | 现状：Unmatched 原因 MissingDependency；补 Retired/Reopened 状态的可查呈现 | #1380/#1406 |
| Rust | `rust/src/economic_session/finance.rs:300` | FIN `0d8a6017` | 现状：计划含 `fee_tail`；退休条件要求费用尾由实收事实结清 | #1380 |
| Rust | `rust/src/economic_session/finance.rs:420-425` | FIN `0d8a6017` | 现状：锁额含费用尾；退休释放整笔锁额，差额进尾账 | #1380/#1408 |
| Rust | `rust/src/economic_session/finance.rs:1015-1024` | FIN `0d8a6017` | 退休释放事件须减少 responsibilities 并触发等待 slot 重算（ADR 0029 裁定二） | #1408 |
| Rust | `rust/src/economic_session/` | main `465174240b` | 缺口：退出授权记录（exit_source/scope/action/authorization）、逐重展开、动作表、退休状态、恢复续做 | #1380 |
| Rust | `rust/src/theta_v0/backtest/admission.rs:1294` | main `465174240b` | 不改：旧 GlobalRiskClose→force_flat 是旧组织的全局动作，不自动成为新架构授权 | — |
| Rust | `rust/src/theta_v0/backtest/fill.rs:5960-5962` | main `465174240b` | 不改：旧 RiskExit 标记 | — |
| Rust | `rust/src/theta_v0/backtest/fill.rs:4699` | main `465174240b` | 不改：#647 入场复检门反证开关，与本票无关 | — |
| Python | `trading_system/execution/risk_controls.py:12/20/37` | main `465174240b` | 可收编不继承：旧 kill switch 拒新单+撤全部+FLATTEN；新架构下触发经 command、执行归 X、范围写入授权记录 | #1380/#1412 |
| Python | `trading_system/execution/risk_controls.py:112` | main `465174240b` | 可收编不继承：手动 reset 解除；新架构冻结态的解除须记授权 | #1380 |
| Doc | `.chanlun/definitions/fengkong.md:31-34` | main `465174240b` | 名分标注段下补一行带票号指针（本次提交已落）；三行头不动 | 本票落文档 |
| Doc | `.chanlun/definitions/chanlun-trading-system.md:21` | main `465174240b` | 不改：名分口径与本票一致 | — |

基线迁移说明：工位草稿以 main `f95a7f3e` 为基线；`git diff f95a7f3e 465174240b` 在本清单涉及的全部路径上为空，main 行号因此不漂。
