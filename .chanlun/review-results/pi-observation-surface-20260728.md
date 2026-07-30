# π 观测面口径盘点（rust/src/theta_v0/backtest/）—— issue #567 数据供给

日期：2026-07-28。范围：runner.rs / fill.rs / admission.rs / opsem_dump.rs / bin/theta_overlay.rs。

## 1. 入口 × 读数矩阵

`FillOutput`（fill.rs:4349，`pub(super)`，π fill loop 唯一全量产出，14 项）：
equity_curve / daily_returns / trade_pnls_realized / trade_pnls_with_forced / trades / n_orders /
typed_ledger / voice_verdicts / tw_final / r_decomp / account_view / center_oscillation_actions /
campaign_book / campaign_witness。

| 读数 | run_theta_v0_pi / pi_chi / pi_chi_shrink（→RunResult, runner.rs:111/374-392） | run_theta_v0_pi_overlay（→OverlayRunResult, runner.rs:399/599-613） |
|---|---|---|
| equity_curve | ✅ `equity_curve` | ✅ `net_result.equity_curve`（W1 时声部口径） |
| daily_returns | ✅ | ✅ `net_result.daily_returns`（随 equity 切换） |
| trade_pnls_realized | ✅ `trade_pnls` | ✅ `net_result.trade_pnls`（W1 时声部） |
| trade_pnls_with_forced | ✅ | ✅ `net_result.*`（W1 时声部+逐声部虚拟强平行） |
| trades（TradeRecord 序） | ✅ | ✅ `net_result.trades`（W1 时逐声部） |
| n_orders | ✅ | ✅ `net_result.n_orders` + `n_overlay_fill_events`（同源 fill.n_orders, runner.rs:574） |
| metrics / theta_return_mtm | ✅ 自产 | ✅ `net_result.*`（由切换后 equity/pnls 算出） |
| r_decomp | ✅ Option | ✅ `net_result.r_decomp`（W1 时声部账户口径） |
| tw_final | ❌ 装配丢弃（RunResult 无字段） | ✅ `tw_final`（runner.rs:432；W1 仍净额） |
| campaign_book / campaign_witness | ❌（非 overlay 路径恒空且丢弃） | ✅ 单独转发（runner.rs:441/444） |
| typed_ledger（typed 顺序） | ❌ 丢弃 | ❌ 丢弃 |
| voice_verdicts | ❌ | ❌ |
| account_view（账户顺序） | ❌ | ❌ |
| center_oscillation_actions | ❌ | ❌（恒空轨，测试可见） |
| overlay 逐声部账 / reconcile 三读数 | — | ✅ `overlay` / `account_price_pnl` / `total_voice_pnl` / `reconcile_residual` / `n_overlay_voices` |
| voice_exec 九字段+book | — | ✅ `voice_exec: Option<VoiceExecRunSummary>`（VOICE_EXEC=1 才 Some，runner.rs:448-468） |

其他暴露面：
- `typed_ledger_from_bars`（runner.rs:645，`pub(super)`）→ 仅 typed_ledger（G4 `build_mu_from_bars` 训练管线）。
- opsem dump（env `OPSEM_DUMP_DIR`，opsem_dump.rs:246/550）→ typed trade 逐笔 JSONL（trades.jsonl，#542 已加身份 schema）。
- CLI：`theta_backtest` 只打 RunResult 摘要；`theta_overlay` 打 net_result 摘要 + overlay 归因 + tw_final（bin/theta_overlay.rs:64-133）。
- 调用面：l3_fullwindow / l3_pi_falsify / l3_pi_probe / l3_delta_r_alpha / wverify_run 全走上述入口，无第三条读数通道。

## 2. W1 例外全枚举（VOICE_EXEC=1，仅 `run_theta_v0_pi_overlay` 生效）

Gate：admission.rs:32 `voice_exec_gate()`（env `VOICE_EXEC=="1"`；测试线程局部 `VOICE_EXEC_OVERRIDE`，admission.rs:26）。
机制：净额影子账本（cash/units/trade_pnls/n_orders_executed）**原样照跑**驱动决策层；VoiceExecBook 消费同一 step_trace 五类生命周期事件（fill.rs:3952-4002，禁第二裁决源），sizing 开仓冻结。

切「声部执行投影口径」的字段（7 处切换点）：
1. `equity_curve` — fill.rs:4036-4042：`vb.cash()+net_signed·px` 替换 `cash+units·px`。
2. `daily_returns` — fill.rs:4209：`bar_returns(&equity_curve)` 派生，随之切换。
3. `trade_pnls_realized` — fill.rs:4333：`voice_trade_pnls`（fill.rs:3131 逐声部平仓产行）。
4. `trade_pnls_with_forced` — fill.rs:4070 + 4078-4098：声部口径 + 逐声部虚拟强平行。
5. `trades` — fill.rs:4335：`voice_trades`（fill.rs:3132-3139）。
6. `n_orders` — fill.rs:4336：`n_voice_fills`（fill.rs:3128）；连带 `n_overlay_fill_events`（runner.rs:574）、`is_l2` 语义变。
7. `r_decomp` — fill.rs:4182-4196：七分量（price/fee/fund/borr/liq/ledger_delta/bridge）全换声部簿。
连带切换：`net_result.metrics`/`theta_return_mtm`（runner.rs:527-543 由上述读数算出）。

**不切换**（净额影子/决策层，与净额臂逐字节一致，锚 runner.rs:4097-4098/4144-4145/4222）：
typed_ledger、tw_final（runner.rs:431 明文）、voice_verdicts、account_view、overlay(OverlayState)、
campaign_book/campaign_witness（drive_campaign_wiring 读 account_view，fill.rs:1226-1258）、
center_oscillation_actions、closed_loop_final、prices/fee_rate。

## 3. 不可见读数

- `tw_final`：`run_theta_v0_pi` 系**不可见**（装配丢弃）；仅 `OverlayRunResult.tw_final`（消费：bin/theta_overlay.rs:83、wverify_run.rs:1302）。
- `campaign_book`/`campaign_witness`：仅 `OverlayRunResult`（wverify_run.rs:1340/1430）。
- `typed_ledger`（typed 顺序）：公开入口**全不可见**；可见面 = `typed_ledger_from_bars`（pub(super)）、opsem trades.jsonl、in-crate 测试。顺序口径 = 事件流 push 序 + 窗口终点 censored 按 (entry_bar, level, ordinal) 排序（fill.rs:4127-4131），`assert_eq!` bit-exact 先例 runner.rs:4097/4144。
- `voice_verdicts` / `account_view`（account 顺序）/ `center_oscillation_actions`：**任何 runner 结果均不可见**，仅 in-crate 测试（runner.rs:2312/3209+、account_view 测试群、fill.rs:2855+）。

## 4. 锁挂载点建议

- **净额影子账 e2e 锁**：挂 `run_theta_v0_pi_overlay`（VOICE_EXEC 未设）——锁 `net_result` 全字段 + `tw_final` + `n_overlay_fill_events` + `reconcile_residual`。备选/互补：`run_theta_v0_pi`（RunResult）。现有锚：runner.rs:4115 `run_theta_v0_pi_overlay_reconciles_and_bit_exact_net`、4263 `voice_exec_env_gate_off_bitexact_on_voice_readings`。
- **声部执行账 e2e 锁**：挂 `run_theta_v0_pi_overlay` + VOICE_EXEC=1（测试经 `VOICE_EXEC_OVERRIDE` 线程局部注入，并行安全）——锁 `voice_exec` 九字段（n_voice_fills/n_voices_total/n_voices_open_end/gross_turnover_lots/fee_paid/conservation_residual/event_bound_holds/price_pnl_reconcile_residual + book）与声部口径 `net_result`（equity_curve/双口径 trade_pnls/trades/n_orders/r_decomp），并锁 tw_final on/off 相等（先例 runner.rs:4222）。e2e 先例：wverify_run.rs:1577/1754。
- **typed/account 顺序锁**：公开入口不可见 → 挂 opsem dump 的 trades.jsonl 产物级锁（#542 schema 已含身份），或 in-crate `typed_ledger_from_bars`/`FillOutput` 断言（runner.rs:4097 先例）。决策层不变量锁：runner.rs:4163 `voice_exec_event_driven_fills_decision_bitexact`（typed_ledger/tw_final bit-exact）已是现成模板。
