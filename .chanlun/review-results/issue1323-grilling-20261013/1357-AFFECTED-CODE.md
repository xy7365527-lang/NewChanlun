# #1357 受影响代码清单

> 票：[#1357](https://github.com/xy7365527-lang/NewChanlun/issues/1357)；正本：`docs/adr/0034-*.md`。
> 基线：**main `3da0b2c872e6e7d69ffcfe29f989a08491d92dd1`**；FIN 线 `codex/1394-transport-revision-20261012` @ `0d8a6017`（标 FIN）。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归「归属」列的实施票，本清单本身不改任何代码。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/economic_session/book.rs:151` | main | 分量白名单增 growth_capitalization / q_rebase / transfer_out / transfer_in（与 ADR 0031 的 principal_contribution 同表） | #1406/#1446 |
| Rust | `rust/src/economic_session/book.rs:347/370` | main | 重视图增 growth_equity{units_g,cash_g,cost_g}、q_rebase 版本、transfer 状态；income-book A 栏与 cash_g 的对账字段 | #1446 |
| Rust | `rust/src/economic_session/book.rs:356-357` | main | 非负存量域：units_g ≥ 0、cash_g ≥ 0；q_actual = Q + units_g 的不变式检查 | #1446 |
| Rust | `rust/src/economic_session/store.rs:78/636` | main | transfer 记录表（七态）与重放覆盖；manifest 的 fixed_q 允许由 q_rebase 版本更新（带 supersedes） | #1446 |
| Rust | `rust/src/economic_session/voice.rs:149/165` | main | 新重开局原件 = transfer 激活事件；开局量 > 0 不变；源 Voice 不接目标尾账 | #1446/#1421 |
| Rust | `rust/src/economic_session/service.rs:105` | main | 路由校验 fixed_q 改为按版本校验（q_rebase 后） | #1446 |
| Rust | `rust/src/economic_session/finance.rs:917/1015-1024` | FIN | E 托管（Escrowed）作为责任类目登记为 transfer_escrow，不与 Reserve 混名；释放触发源加 transfer 激活/取消 | #1394 后续 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:94/121/175` | main | TStage latch 不读作换档准入（ADR 0031 裁定三）；TwState.notional_in 非 G | — |
| Rust | `rust/src/theta_v0/backtest/fill.rs:200-255` | main | 旧净额臂全平重置 entry_cost 与本票 G 口径不同；C11 收编/退役清单 | TB-10 |
| Lean | `formal/Origin/TotalWealth.lean:97/140/194/208` | main | 不改；登记 G 守恒（I1）为新的 Lean 跟进义务（可选） | Lean 跟进 |
| Python | `s_session/economic_query.py:72` | main | 透传 growth_equity / transfer 状态 | #1446 |
| Python | `s_session/browser/economic.html:67` | main | 显示 G 三分量与转移状态；不算比例 | #1446 |
| 文档 | `docs/adr/0034（编号待根分配）` | main | 与 #1355 合写 |  |
| 文档 | `CONTEXT.md` | main | 词条：增长权益 G / 换档转移 / q_rebase |  |
| 文档 | `.chanlun/definitions/sanjieduan.md` | main | §3 末补一句：G 不是本金；转移形成的 principal_contribution 来源为 transfer |  |
