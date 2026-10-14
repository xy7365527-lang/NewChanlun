# #1358 受影响代码清单

> 票：[#1358](https://github.com/xy7365527-lang/NewChanlun/issues/1358)；正本：`docs/adr/0036-*.md`。
> 基线：**main `3da0b2c872e6e7d69ffcfe29f989a08491d92dd1`**；FIN 线 `codex/1394-transport-revision-20261012` @ `0d8a6017`（标 FIN）。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归「归属」列的实施票，本清单本身不改任何代码。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/economic_session/book.rs:347` | main | capital_phase 投影增 GateReachedEvent / GateReachedCandidate / SimulatedGateReached 与 growth_enabled（事实与许可分列） | #1406/#1410 |
| Rust | `rust/src/economic_session/store.rs:636` | main | PostGatePolicy 版本表、FirstEnablementRecord、时序校验（触达<版本<启用） | #1410 |
| Rust | `rust/src/economic_session/service.rs` | main | 闸门后评价查询：AwaitingPolicy{post_gate_params} 直到版本落盘；启用 op 拒绝无版本 | #1410 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:94/121` | main | TStage latch 不作触达判定输入（ADR 0031 裁定三） | — |
| Rust | `rust/src/economic_session/finance.rs:204-213` | FIN | 无改动；growth_enabled 不进 E 政策字段 | — |
| Lean | `formal/Origin/TotalWealth.lean:140/194/208` | main | 不改；EarningShares latch 与 GateReachedEvent 的对拍义务沿 ADR 0031 既登记项 | — |
| Python | `s_session/economic_query.py:72` | main | 透传触达事件、PostGatePolicy 版本（Unset 槽位）与 AwaitingPolicy | #1410 |
| Python | `s_session/browser/economic.html:67` | main | 显示『已触达/待参数/已启用』三态，不显示任何默认参数 | #1410 |
| 文档 | `docs/adr/0036（编号待根分配）` | main | 与 #1356 合写 |  |
| 文档 | `CONTEXT.md` | main | 词条：触闸事件 / growth_enabled |  |
