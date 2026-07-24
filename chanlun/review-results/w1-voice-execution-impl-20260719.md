# W1 声部独立执行（churn 修复）实装报告

- 工位：W1 声部独立执行实装（wave kimi-nest-mainline-20260717，worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`）。
- 纪律：090（声明=能力，照实否定合格）/ v3 硬禁令（本实装全部为账本机制与结构恒等，无概率/统计推断作决策基础、无回测验证策略、无 EMH 假设）。授权文件只改 `rust/src/theta_v0/backtest/runner.rs` + `rust/src/theta_v0/strategy/overlay_state.rs`；`rust/Cargo.toml` 未动一行；无 git mutation；主仓 `/Users/silencehan/Projects/NewChanlun` 未写。
- 设计依据：`chanlun/review-results/netting-vs-voice-execution-audit-20260719.md` §7（修复设计）+ §4/§5（churn 机制论证）。
- 结论先行：任务 ①-⑦ 全部兑现，`cargo test --release --lib` **1752 passed / 0 failed / 128 ignored**（全绿零变红；新增 9 测试，其余 1743 为实装前既有绿）。**一处与设计文档的偏差如实登记**（§1.2：风控门输入取影子 p_t 而非 N_derived——任务约束③决策层逐字节不变优先）。

## 0. 任务验收对照

| # | 要求 | 兑现 | 锚 |
|---|---|---|---|
| ① | 新臂 `pi_theta_fill_loop_voice`：声部独立持仓驱动 fill、声部开合才产 fill、决策单源复用 step_trace 五类事件 | ✅ | `runner.rs:1136`（臂函数）；`overlay_state.rs:294-647`（VoiceExecBook 及关联类型）；事件消费 `runner.rs:2061-2112` |
| ② | sizing 事件化：开仓冻结 q_v，不再每 bar 按 equity_nav/px 重算在仓声部目标 | ✅ | 冻结点 `runner.rs:2093-2110`（q 取开仓 bar `SepLeg.q_units` 取整后落簿，存续期零重定）；簿内无 resize API（`overlay_state.rs:482-535` apply_open 注释） |
| ③ | typed_ledger/TW/sep_legs 决策层逐字节不变 | ✅（构造性 + 测试见证） | 影子方案 §1；测试 `voice_exec_event_driven_fills_decision_bitexact` / `voice_exec_frozen_sizing_under_nav_drift`（typed_ledger、tw_final 逐字段相等） |
| ④ | env gate（VOICE_EXEC=1）接入，默认关闭生产路径逐字节不变 | ✅ | `runner.rs:712-732`（gate + 线程局部注入）、`runner.rs:764-784`（gate 判定 + `voice_book.as_mut()` 接线）；测试 `voice_exec_env_gate_off_bitexact_on_voice_readings` |
| ⑤ | 单测：声部簿守恒 / 事件驱动 fill / env 未设 bit-exact | ✅ 9 个新测试 | §5 清单 |
| ⑥ | cargo test --release --lib 全绿零变红 | ✅ | §5.3 输出 |
| ⑦ | diff + 测试输出 + 报告落盘 | ✅ | 本文 §5/§8 |

## 1. 关键设计决定

### 1.1 净额影子账本驱动决策层（bit-exact 的构造性保证）

设计文档 §7.1 要求「信号、typed ledger、TW 账本序列与净额臂 bit-exact，只有执行层不同」。但决策层的每 bar 输入——`p_t`（`pi_theta_step_traced` 参数 + `k_theta_risk_gate` 风控门）、`equity_nav`（→ `base_units=equity_nav/px` sizing 基准）、TW 成本划转（`basis_now=|units|·|entry_cost|`）与 Realize 入账（平仓 fill 费后 PnL）——全部由**真实成交结果**反馈。声部执行改变成交（冻结持仓 ≠ 净额臂逐 bar 重定目标的持仓），若让这些输入读声部账户真值，决策轨迹在一般情形下必然偏离净额臂（Σσ_v q_v 冻结持仓 ≠ 净额臂 p_t；二者只在无对冲且零漂移时相等）。

因此实装采用**影子方案**：`pi_theta_fill_loop_overlay` 内既有的 `(cash, units, entry_cost, realized_cum)` 净额账本在声部臂下**原样全量运行**（净额订单照常挂、照常成交——代码逐行未动），作为决策层唯一真值源；声部簿 `VoiceExecBook` 是叠加其上的**真实执行投影**（独立 `pending_voice` 队列、独立现金、逐声部持仓）。构造性结论：决策层每一个读取点读的都是与净额臂完全相同的值 ⟹ typed_ledger/TW/sep_legs/gate/base_units 逐字节不变。这也是设计文档 §7.2「后」伪码 `pi_theta_step_traced(... 同参 ...)` 的唯一自洽读法。

### 1.2 偏差登记（090 照实）：风控门输入 = 影子 p_t，不是 N_derived

设计文档 §7.1 另有一行「净敞口 N=Σσ_v q_v 降级为派生只读量（仅供风控门 k_theta_risk_gate 与 cap 判据消费）」。此行与「决策轨迹 bit-exact」在一般情形下不可同时成立（§1.1）。按本任务约束③（决策层逐字节不变为硬边界），实装取：**风控门/cap 仍消费影子 p_t**（现状口径不变），N_derived 由 `VoiceExecBook::net_signed()`（`overlay_state.rs:427-429`）派生暴露、并实际用于声部账户自身的持仓成本计费（`runner.rs:1392-1411`）与权益 MtM（`runner.rs:2145-2151`），但**不反馈进决策层**。hedge-mode 下毛敞口 Q⁺+Q⁻ 的 margin 口径（设计文档 §8 已标注留编排者裁定）不受影响地保留为未决项——本实装不做该裁定。

### 1.3 sizing 冻结的落点（不改决策层）

`coverage.rs` 的 legs sizing（`units=base_units×w_depth×w_dir`）是两臂共享的决策层目标，一行未改。冻结发生在执行臂把目标落簿时（设计文档 §7.3「冻结发生在执行臂把 q_units 落簿时」）：开仓事件当步，从 `step_trace.sep_legs` 取该腿 `q_units`（决策层用开仓 bar 的 base_units 算好的单源值，与 typed ledger B1 快照同值），`round(q_units/lot)·lot` 取整为手数后冻结；存续期簿内无任何重定目标路径（VoiceExecBook 没有 resize 方法；结构事件 resize 默认空集 = 设计文档的「带宽默认 ∞」）。净额臂的 `base_units` 每 bar 重算原样保留（影子需要，bit-exact）。

## 2. 实装清单（行号 = 实装后文件）

### 2.1 `rust/src/theta_v0/strategy/overlay_state.rs`（+489 行，全部新增，无改动既有行）

- `VoiceOrderKind`/`VoiceOrder`（`overlay_state.rs:294-316`）：执行投影层订单（voice_id + kind 留痕——净额 `Order` 三字段无身份的对照修复）；**不改** `types.rs Order`（决策层类型）。
- `VoicePosition`（:318-334）：逐声部 (σ_v, q_v 冻结, entry_bar/entry_px 含费成本基, pnl_price 价格 PnL, fee_paid 实付费)——pnl_price（毛）与 fee/pnl_net（费后）字段分立，禁冒充（090，设计 §7.3 同款要求）。
- `ClosedVoiceExec`（:336-358）：已结算 campaign 归因行（forced 强平标记）。
- `VoiceExecBook`（:380-647）：hedge-mode position book（PDF §10.2 (Q⁺,Q⁻)，对冲两腿各自存续各自结算）。方法：
  - `mark_to_market`（:464-477）：逐 bar 价格 PnL 累计（与净额臂 `cum_price_pnl` 同一 Δpx 时点口径）；
  - `apply_open`（:482-535）：开仓 fill，现金约束与 `apply_fill` 段2 同口径（不足拒开）；同 carrier 已有持仓 debug_assert + release 防御性先平后开；
  - `apply_close`（:538-552）/`flatten`（:555-597）：平仓 fill = flatten 簿内该声部全部持仓，费后 PnL 与 `apply_fill` 段1 同口径；无持仓 no-op（restore 祖先腿/未成交开仓单的 close 事件，与 typed ledger「表中无登记 ⟹ 不入 ledger」同语义）；
  - `settle_forced_virtual`（:599-641）：窗口终点强平**虚拟兑现**（不改现金/不计 fill/不计费，与净额臂 forced_pnl 同口径；确定序输出）；
  - `debit_cash`（:643-645）：持仓成本/罚金扣款（cost_model=None ⟹ 永不调用）。
- 守恒不变量（构造性，runner R 分解断言锚）：`cash + N·px − nav0 == account_price_pnl − cum_fee − 持仓成本`——开/平每段现金流与费、MtM 逐段对齐（单测 `voice_exec_open_close_conservation` 数值见证）。

### 2.2 `rust/src/theta_v0/backtest/runner.rs`

- `pi_theta_fill_loop_voice`（:1136-1157，cfg(test) 薄封装）：`pi_theta_fill_loop_overlay(..., None, Some(voice))`。生产接入走 `run_theta_v0_pi_overlay` 的 env gate（臂逻辑在内联分支，无需公开第二入口）。
- `pi_theta_fill_loop_overlay` 签名（:1162-1174）：新增 `voice_exec: Option<&mut VoiceExecBook>`；`pi_theta_fill_loop` 传 `None`（:1124）——**None ⟹ 逐字节不变**。
- 声明块（:1233-1245）：`voice_enabled` / `pending_voice` 队列 / `n_voice_fills` / `voice_trade_pnls` / `voice_trades` / 声部持仓成本累计器。
- 声部插入点（全部 `voice_enabled` 门控，净额影子代码逐行未动）：
  - 循环顶 MtM（:1326-1330）；
  - ①-voice 声部 fill（:1350-1375）：逐声部成交，平仓产 trade_pnls/TradeRecord；
  - ①⁺-voice 持仓成本镜像（:1392-1411，对 N_derived 名义）；
  - ③⁻-voice 强平罚金镜像（:1481-1493，触发边沿与影子同一 `liq_active` 去抖，名义 = |N_derived|·px）；
  - ④-voice 声部挂单（:2061-2112）：**决策单源** = `step_trace` 五类事件（opened/closed/silent_drops/risk_exits/overlay_closes，与 typed ledger 消费同源，无第二裁决源）；**平单先挂、开单后挂**（同 bar close→reopen 同 carrier 时 fill 序 = 先平后开）；无事件 bar 零订单；
  - ⑥ 权益曲线（:2145-2151）：声部臂输出 `cash_v + N_derived·px`；
  - 窗口终点强平（:2169-2204）：声部臂逐声部虚拟兑现（forced 行 + TradeRecord）；`voice_end_net` 在清簿前捕获供 R 分解；
  - R 分解 + FillOutput（:2270-2312）：声部账户口径组装，**同一守恒断言**对声部账户成立；`trade_pnls_realized/trades/n_orders` 换声部读数，`typed_ledger/tw_final` 原样（bit-exact）。
- env gate（:718-733 线程局部 + :727 gate 函数）：`voice_exec_gate()` = `VOICE_EXEC=1`；测试经线程局部 `VOICE_EXEC_OVERRIDE` 注入（生产构建不含该分支——OPSEM_DUMP_DIR_OVERRIDE 同惯例，规避 2026-07-13 实录的并行测试 env 竞态）。
- `OverlayRunResult.voice_exec: Option<VoiceExecRunSummary>`（:688-722）+ 装配（:839-863）：VOICE_EXEC=1 时 Some（fill 上界见证/毛周转/费/守恒残差/价格 PnL 对账残差/终态簿），未设 None；`net_result`/`n_overlay_fill_events`/`tw_final` 字段注释已同步声明 VOICE_EXEC 例外口径（090 注释扫描）。

### 2.3 既有未提交改动的说明（非本工位产出）

worktree 的 `runner.rs` 在本工位开工前已含其他工位未提交改动（93+/22-：`n_overlay_fill_events` 三态一致、B1 sizing 快照/opsem L1-P1/L2-P4-P6 字段、`via_anc_ok_prune` 键名诚实化）。§8 diff 附录中 hunk `@@ -663,13` 与 `@@ -752,20` 为**混合 hunk**（含上述既有行），其余 18 个 hunk 全部为本工位产出；`overlay_state.rs` 全部 2 个 hunk 为本工位产出。

## 3. 机制论证的实装落实（审计 §4/§5 → 代码）

1. **churn 根源切除**：净额臂 churn = 每 bar `base_units=equity_nav/px` 重算 → 声部目标漂移 → 聚合 ΔN 过阈 → 单订单兜底成交收费。声部臂下 q_v 开仓冻结（§1.3），存续期无任何重定路径 ⟹ 漂移驱动 fill 在构造上不可能（簿内无该代码路径，非参数调优）。
2. **fill 事件率 = 声部事件率**：fill 只在五类生命周期事件的 exec_index 产生，构造性上界 `n_fills ≤ 2×n_voices`（开+平各一次；resize 空集）——`VoiceExecRunSummary.event_bound_holds` 逐跑见证 + 单测断言。
3. **对冲毛额化**：hedge-mode 簿保存 (Q⁺,Q⁻)，A 多 100/B 空 60 两腿各自成交各自结算（设计 §3 对照表第一行兑现）；净额消除的 (G−T) 部分在声部臂毛额付回——设计 §6.3 已诚实计入该反向修正，不改变佣金降 1-2 个数量级的量级结论（量级论证非预测，本文不重复）。
4. **费用口径不变**：成交费 = 成交名义 × fee_rate（3bps 未标定，config.rs 默认 1+2+0），逐 fill 独立累计（`cum_fee`），R 分解守恒断言同公式锚定——降的是周转不是费率假设（设计 §7.5 衔接）。

## 4. bit-exact 边界清单

**逐字节不变（构造性保证 + 测试见证）**：

- `typed_ledger` 全序列（测试逐字段 `assert_eq!`，含 B1 sizing 快照 `units` 字段——见证 sep_legs 在开仓腿的 sizing 一致）；
- `tw_final` 及 TW 事件序列（影子驱动；测试 `on.tw_final == off.tw_final` + loop 层相等断言）；
- `step_trace.sep_legs` 逐 bar 序列（结构论证：决策输入全部读影子 ⟹ coverage 输出同值；无直接外化可断口，见证 = typed_ledger 逐字段相等——照实标注此链是「构造论证 + 间接见证」，非逐 bar 直断）；
- 分类器/塔/χ：未碰；
- **`VOICE_EXEC` 未设时的全部生产产出**：voice 分支全部门控跳过，既有行仅 4 处等价改写（⑥ equity 分支取同值、forced 块 if/else、r_decomp 元组取同值、FillOutput 字段取同值），测试 `voice_exec_env_gate_off_bitexact_on_voice_readings` 第 (1) 段对 `run_theta_v0_pi` 逐字段断言。

**设计性改变（非回归，如实登记）**：`n_orders`（→ 声部 fill 事件数，上界 2×声部数）、`equity_curve`/`daily_returns`（声部账户费用路径）、`trade_pnls_realized`/`with_forced`（按声部一次性结算，条数 ≈ 声部数而非 fill 数）、`trades`（逐声部 TradeRecord）、`r_decomp`（声部账户分解；`tw_holding_cost_bridge` 行换声部账户自身持仓成本量化——cost_model=None 时与净额臂同为 0）。

## 5. 测试

### 5.1 新增单测（9 个，全绿）

`overlay_state.rs`（5）：
- `voice_exec_open_close_conservation`：开→MtM→平全链，现金/费/价格 PnL 逐段数值断言 + 守恒恒等 + PDF §11 Σpnl_price 对账；
- `voice_exec_hedged_voices_independent_books`：**声部独立持仓守恒**（Σ_v σ_v q_v 与声部簿一致；对冲两腿不湮灭、独立结算）；
- `voice_exec_forced_settle_virtual_no_cash`：虚拟兑现不改现金/不计 fill/不计费；
- `voice_exec_open_rejected_on_insufficient_cash`：现金不足拒开不伪造执行事实；
- `voice_exec_close_without_position_noop`：无持仓 close 事件 no-op。

`runner.rs`（4）：
- `voice_exec_event_driven_fills_decision_bitexact`：决策层 bit-exact（typed_ledger/tw_final 逐字段）+ **事件驱动 fill 只在声部开合产生**（20 bar 恰 2 fill = 开+平）+ 上界断言 + 按声部结算 1 行 + 守恒；
- `voice_exec_frozen_sizing_under_nav_drift`：**sizing 冻结直接见证**——锯齿价（base_units 每 bar 剧烈漂移）下 fill 仍只有开仓 1 次，存续期零重定；
- `voice_exec_no_signal_zero_fills`：无事件 ⟹ 零订单零费（权益恒 1）；
- `voice_exec_env_gate_off_bitexact_on_voice_readings`：**env 未设逐字节不变**（对 run_theta_v0_pi 逐字段）+ env 开时读数装配/TW bit-exact/上界/守恒/对账。

### 5.2 新增测试输出

```text
running 9 tests
test theta_v0::strategy::overlay_state::tests::voice_exec_open_rejected_on_insufficient_cash ... ok
test theta_v0::strategy::overlay_state::tests::voice_exec_close_without_position_noop ... ok
test theta_v0::strategy::overlay_state::tests::voice_exec_open_close_conservation ... ok
test theta_v0::strategy::overlay_state::tests::voice_exec_hedged_voices_independent_books ... ok
test theta_v0::strategy::overlay_state::tests::voice_exec_forced_settle_virtual_no_cash ... ok
test theta_v0::backtest::runner::tests::voice_exec_no_signal_zero_fills ... ok
test theta_v0::backtest::runner::tests::voice_exec_event_driven_fills_decision_bitexact ... ok
test theta_v0::backtest::runner::tests::voice_exec_env_gate_off_bitexact_on_voice_readings ... ok
test theta_v0::backtest::runner::tests::voice_exec_frozen_sizing_under_nav_drift ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 1871 filtered out; finished in 0.00s
```

### 5.3 全量回归（实装后最终状态，`cd rust && cargo test --release --lib`）

```text
test result: ok. 1752 passed; 0 failed; 128 ignored; 0 measured; 0 filtered out; finished in 1.07s
```

- 全绿零变红：0 failed。新增 9 个全部 passed；其余 1743 个为实装前既有绿（任务书基线 1737 对应更早树态；本 worktree 开工时已含其他工位新增测试，实装前既有通过数 = 1752 − 9 = 1743，实装后无一变红）。

## 6. 验收量读数（审计 §7.4 落地）

- `n_voice_fills ≤ 2×n_voices_total`（事件驱动上界，构造性）：`VoiceExecRunSummary.event_bound_holds` 逐跑装配 + 两测试断言 ✓。
- `gross_voice_turnover`（G）与净成交周转 T：事件驱动下 G == T == Σ(q开+q平)（簿内 `gross_turnover_lots`），对照臂读数 `hedge_netted_away`/`voice_shred` 需净额臂侧计数器——**照实缺席**（本工位授权文件不含净额臂计数器落点；审计 §4.2 的诚实缺席在声部臂侧已补 G/T/L 三量中的 G≡T≡L，剩余两对照量归后续工位）。
- 声部账户守恒残差 / 价格 PnL 对账残差：逐跑装配（`conservation_residual` / `price_pnl_reconcile_residual`），测试断言 ≤ 容差 ✓。

## 7. 诚实缺席 / 已知边界

- **PanDiv 振荡子腿不覆盖**：`center_oscillation.enabled=true` 时 ShortDiff 子腿开合走 `pan_div_state` 旁路（`runner.rs:1674-1757` 净额订单槽覆写），**不经** step_trace 五类事件 ⟹ 声部簿将缺失这些子腿持仓（决策层不受影响，仍 bit-exact；声部账户执行投影不完整）。默认配置 `enabled=false`（`runner.rs:4445` 测试断言）下无影响。VOICE_EXEC=1 与 PanDiv 同开的组合**未实装、不声明可用**。
- **结构事件 resize = 空集**（设计默认「带宽 ∞ 不重定」）：簿内无 resize API；若未来引入级别/角色变化触发的重定，上界变为 `2×n_voices + n_resize_events`（设计 §6.2），需同步加计数器。
- **margin/风控衔接**：hedge-mode 毛敞口 Q⁺+Q⁻ 的 margin 口径未裁定（设计 §8 留编排者）；本实装风控门读影子 p_t（§1.2），声部账户自身不独立的破产/强平状态机（强平触发继承影子边沿，罚金按声部名义计）。
- **sep_legs 逐 bar 直断缺席**：见 §4 第三条——构造论证 + typed_ledger 间接见证，无逐 bar 外化对拍（加外化口属另一授权）。
- **三窗实跑数字缺席**：本工位纪律为实装 + 单测 + 全量回归；p3fold/wf7/wf8 的 VOICE_EXEC=1 对照跑数（订单数/佣金量级实测 vs 设计 §6 估算区间）归跑批工位，本文不预测。
- 费率 3bps 未标定不变（设计 §7.5）；本实装降周转，不触碰费率假设。

## 8. diff 附录（本工位产出 hunk；两个混合 hunk 见 §2.3 说明）

```diff
diff --git a/rust/src/theta_v0/backtest/runner.rs b/rust/src/theta_v0/backtest/runner.rs
index e43f61d88b..3bbb7e330b 100644
--- a/rust/src/theta_v0/backtest/runner.rs
+++ b/rust/src/theta_v0/backtest/runner.rs@@ -652,8 +652,17 @@ fn run_theta_v0_pi_inner(
 pub struct OverlayRunResult {
     pub symbol: String,
     pub n_bars: usize,
-    /// 净额执行层订单数（overlay ΔN 非零步数 = 真实下单次数）。
-    pub n_overlay_orders: usize,
+    /// 净额执行层 fill 事件数（overlay ΔN 非零步数 = 真实下单次数）。
+    /// ★090 三态一致（L1-P3，D-Ovr-1）：同源 `net_result.n_orders`（= `fill.n_orders`，
+    /// 每 fill 事件 `executed_qty>0` 才计数，runner.rs:1191-1195），与 ΔN 守恒断言
+    /// （`pi_theta_fill_loop_overlay` 内逐 bar `order==ΔN` debug_assert）口径一致。
+    /// ★W1 例外：env `VOICE_EXEC=1` 时 = **声部 fill 事件数**（执行投影口径，见 `voice_exec`
+    /// 字段——两读数分列，禁冒充净额口径）。
+    pub n_overlay_fill_events: usize,
+    /// overlay 层声部总数（已离场 + 活动声部），诊断读数——**声部 ≠ 订单**
+    /// （一声部生命周期内可产多次 ΔN fill 事件：开仓/减仓/平仓/再开）。
+    /// 本字段承载原 `n_overlay_orders` 的赋值语义（该旧字段名声部数冒充订单数，090 违规已修）。
+    pub n_overlay_voices: usize,
     /// 账户级累计净额价格 PnL（`Σ_t N_t·ΔP_t`，PDF §11 对账账户侧）。
     pub account_price_pnl: f64,
     /// Σ_v pnl_v（分账本侧，PDF §11 应 ≈ `account_price_pnl`）。
@@ -663,13 +672,66 @@ pub struct OverlayRunResult {
     /// 终态 overlay 账本（活动 + 已离场声部归因，逐声部 entry_v/exit_v/parent(v)/role(v)/pnl_v）。
     pub overlay: super::super::strategy::overlay_state::OverlayState,
     /// 净额执行层 RunResult（同 `run_theta_v0_pi`，净额订单/权益——overlay 是其只读旁路，数字不变）。
+    /// ★W1 例外：env `VOICE_EXEC=1` 时本字段承载**声部执行投影**口径（见 `voice_exec` 字段
+    /// 注释——fill.n_orders=声部 fill 事件数、equity/trade_pnls/r_decomp=声部账户），
+    /// 决策层（typed_ledger/TW/sep_legs）仍与净额臂逐字节一致。
     pub net_result: RunResult,
     /// ★M8 treasury 层终态（TARGET_STRATEGY_MAXFULL.md M7:156-159）：三阶段资金账本 `TwState`
     /// 终态（stage/free/holding/withdrawn/notional_in/open_legacy_legs）。overlay 臂驱动的同一
     /// 主 loop 内建 TW 账本（`pi_theta_fill_loop_overlay` 的 `fill.tw_final`），此前被
     /// `net_result: RunResult` 装配丢弃（RunResult 无 tw_final 字段）——M8 端到端四层报告的
     /// treasury 层（第三层）需读它算 `Reach(Stage)`/`Q_T`/`W_T`/`η_T`。`None` 仅当 bars 为空。
+    /// ★W1：TW 账本由净额影子账本驱动 ⟹ VOICE_EXEC=1 时本字段仍与净额臂逐字节一致。
     pub tw_final: Option<super::super::strategy::ledger::TwState>,
+    /// ★W1 声部独立执行读数（churn 修复臂，netting-vs-voice-execution-audit-20260719 §7）。
+    /// env `VOICE_EXEC=1` 时 `Some`：此时 `net_result`/`n_overlay_fill_events` 承载**声部执行投影**
+    /// 口径（fill.n_orders = 声部 fill 事件数，equity/trade_pnls/r_decomp = 声部账户）——与净额臂
+    /// 语义不同，两读数经本字段分列，禁互相冒充（090）。未设 env ⟹ `None`，全部净额读数逐字节不变。
+    pub voice_exec: Option<VoiceExecRunSummary>,
+}
+
+/// ★W1 声部独立执行跑批读数（VOICE_EXEC=1 时装配；验收量见审计 §7.4）。
+pub struct VoiceExecRunSummary {
+    /// 声部 fill 事件数（executed>0 才计；== 本臂 `net_result.n_orders`）。
+    pub n_voice_fills: usize,
+    /// 开过的声部 campaign 总数（含在飞）。
+    pub n_voices_total: usize,
+    /// 窗口终点仍在飞的声部数（censored，经虚拟兑现入归因表）。
+    pub n_voices_open_end: usize,
+    /// 声部毛周转（手数）G = Σ(q开+q平)——事件驱动下 == 实际成交周转 T（审计 §7.4 验收量）。
+    pub gross_turnover_lots: i64,
+    /// 声部账户累计成交费（Commission+Slippage+Tax）。
+    pub fee_paid: f64,
+    /// 声部账户 R 分解守恒残差（应 ≈0；loop 内 debug_assert 同锚）。
+    pub conservation_residual: f64,
+    /// 事件驱动上界不变量见证：`n_voice_fills ≤ 2×n_voices_total`（resize 事件默认空集，
+    /// 构造性可断——审计 §7.4 断言）。
+    pub event_bound_holds: bool,
+    /// 声部账户价格 PnL 对账残差 |account_price_pnl − Σ_v pnl_price|（PDF §11，< eps）。
+    pub price_pnl_reconcile_residual: f64,
+    /// 终态声部执行簿（逐声部归因表；含 forced 强平行）。
+    pub book: super::super::strategy::overlay_state::VoiceExecBook,
+}
+
+#[cfg(test)]
+thread_local! {
+    /// 测试注入点：Some(v) ⟹ **本线程**的 VOICE_EXEC gate 返回 v（优先于进程级 env）。
+    /// 进程级 env 会被并行测试同时读到（OPSEM_DUMP_DIR 2026-07-13 竞态实录同型风险）；
+    /// 线程局部对并行测试不可见。
+    static VOICE_EXEC_OVERRIDE: std::cell::Cell<Option<bool>> = std::cell::Cell::new(None);
+}
+
+/// ★W1 env gate：`VOICE_EXEC=1` ⟹ 声部独立执行臂（churn 修复）；未设/非"1" ⟹ 净额臂
+/// （bit-exact 回归锁）。测试经线程局部 [`VOICE_EXEC_OVERRIDE`] 注入（生产构建不含该分支，
+/// OPSEM_DUMP_DIR_OVERRIDE 同惯例）。
+fn voice_exec_gate() -> bool {
+    #[cfg(test)]
+    {
+        if let Some(v) = VOICE_EXEC_OVERRIDE.with(|c| c.get()) {
+            return v;
+        }
+    }
+    std::env::var("VOICE_EXEC").ok().as_deref() == Some("1")
 }
 
 /// ★M5 声部执行层 arm（多空对冲.pdf p16 关卡10）：与 [`run_theta_v0_pi`] **同一信号决策路径**
@@ -684,6 +746,12 @@ pub struct OverlayRunResult {
 ///
 /// **认识论 L1**：ΔN 守恒 + 对账是结构恒等（构造性 + 线性代数），**不声明 alpha**——首轮数字
 /// （亏损/空转）照实（执行层首次真实化，PDF §9 声部生成层 + 净额可见层验收，非经济有效层）。
+///
+/// ★W1（churn 修复臂，netting-vs-voice-execution-audit-20260719 §7）：env `VOICE_EXEC=1` ⟹
+/// 接入声部独立执行（`VoiceExecBook` 驱动真实 fill：声部独立持仓 + 事件驱动开合 + 开仓冻结
+/// sizing），决策层（typed_ledger/TW/sep_legs）由净额影子账本驱动 ⟹ 与净额臂逐字节一致；
+/// **env 未设 ⟹ 净额执行 + overlay 只读旁路，生产路径逐字节不变**（bit-exact 回归锁，
+/// `voice_exec_env_unset_bit_exact` 测试锚）。
 pub fn run_theta_v0_pi_overlay(
     dataset: &Dataset,
     config: &ThetaConfig,
@@ -692,6 +760,13 @@ pub fn run_theta_v0_pi_overlay(
 ) -> OverlayRunResult {
     let bars = &dataset.bars;
     let mut overlay = super::super::strategy::overlay_state::OverlayState::new();
+    // ★W1 env gate：VOICE_EXEC=1 ⟹ 声部独立执行臂；未设/非"1" ⟹ 净额臂（bit-exact）。
+    let voice_exec_on = voice_exec_gate();
+    let mut voice_book = if voice_exec_on {
+        Some(super::super::strategy::overlay_state::VoiceExecBook::new(initial_nav))
+    } else {
+        None
+    };
 
     let mut classifier_incr = super::incremental::IncrementalClassifier::new(bars, config);
     let fill = pi_theta_fill_loop_overlay(
@@ -752,20 +828,46 @@ pub fn run_theta_v0_pi_overlay(
     let account_price_pnl = overlay.account_price_pnl();
     let total_voice_pnl = overlay.total_voice_pnl();
     let reconcile_residual = (account_price_pnl - total_voice_pnl).abs();
-    // 净额执行订单数 = 活动 + 已离场声部（每声部至少一次 open/close 产 ΔN；诚实计数用 closed 表大小
-    // + 活动声部数——每条离场声部完整经历过 open+close 两次 ΔN 端点，活动声部至少一次 open）。
-    let n_overlay_orders = overlay.closed_voices().len() + overlay.active_voices().count();
+    // ★090 三态一致修正（L1-P3，D-Ovr-1）：fill 事件数同源 fill.n_orders（真 ΔN 非零步数，
+    // 与 ΔN 守恒断言对齐）；原赋值（closed+active 声部数）保留为 n_overlay_voices 诊断读数——
+    // 声部 ≠ 订单（一声部至少 open+close 两次 ΔN 端点，还可减仓/再开）。
+    let n_overlay_fill_events = fill.n_orders;
+    let n_overlay_voices = overlay.closed_voices().len() + overlay.active_voices().count();
+
+    // ★W1：VOICE_EXEC=1 时装配声部执行读数（验收量 §7.4：fill 上界/毛周转/守恒残差/价格 PnL
+    //    对账）；未设 ⟹ None（净额读数逐字节不变）。
+    let voice_exec = voice_book.map(|book| {
+        let conservation_residual = net_result
+            .r_decomp
+            .map(|r| r.conservation_residual)
+            .unwrap_or(0.0);
+        let price_pnl_reconcile_residual =
+            (book.account_price_pnl() - book.total_voice_price_pnl()).abs();
+        VoiceExecRunSummary {
+            n_voice_fills: book.n_fills(),
+            n_voices_total: book.total_voices(),
+            n_voices_open_end: book.n_open_end(),
+            gross_turnover_lots: book.gross_turnover_lots(),
+            fee_paid: book.cum_fee(),
+            conservation_residual,
+            event_bound_holds: book.n_fills() <= 2 * book.total_voices(),
+            price_pnl_reconcile_residual,
+            book,
+        }
+    });
 
     OverlayRunResult {
         symbol: dataset.symbol.clone(),
         n_bars: bars.len(),
-        n_overlay_orders,
+        n_overlay_fill_events,
+        n_overlay_voices,
         account_price_pnl,
         total_voice_pnl,
         reconcile_residual,
         overlay,
         net_result,
         tw_final,
+        voice_exec,
     }
 }
 
@@ -1019,7 +1121,30 @@ where
 {
     // ★M5 wrapper：overlay=None ⟹ 现有净额路径逐字节不变（bit-exact）。overlay 簿接线走
     // [`pi_theta_fill_loop_overlay`]（run_theta_v0_pi_overlay arm 传 Some）。
-    pi_theta_fill_loop_overlay(classify_at, bars, initial_nav, config, chi, None)
+    pi_theta_fill_loop_overlay(classify_at, bars, initial_nav, config, chi, None, None)
+}
+
+/// ★W1 声部独立执行臂（churn 修复，netting-vs-voice-execution-audit-20260719 §7）：与
+/// [`pi_theta_fill_loop`] **同一决策路径**——净额影子账本（cash/units/entry_cost 全部原有
+/// 逻辑原样跑）驱动决策层 ⟹ typed_ledger/TW/sep_legs 与净额臂逐字节一致；执行投影换为
+/// [`VoiceExecBook`](super::super::strategy::overlay_state::VoiceExecBook)——
+/// ① 声部独立持仓（hedge-mode (Q⁺,Q⁻)，PDF §10.2）；② 事件驱动 fill（消费同一
+/// `pi_theta_step_traced` 的 step_trace 五类生命周期事件，禁第二裁决源；无事件 bar 零订单）；
+/// ③ sizing 冻结（q_v 取开仓 bar 决策层 `SepLeg.q_units` 取整落簿，存续期不随 NAV/价重定）。
+/// 生产接入 = `run_theta_v0_pi_overlay` 的 env `VOICE_EXEC=1` gate（默认关闭 = 净额臂 bit-exact）。
+#[cfg(test)]
+fn pi_theta_fill_loop_voice<F>(
+    classify_at: F,
+    bars: &[Bar],
+    initial_nav: f64,
+    config: &ThetaConfig,
+    chi: Option<ChiFilterCtx>,
+    voice: &mut super::super::strategy::overlay_state::VoiceExecBook,
+) -> FillOutput
+where
+    F: FnMut(usize) -> (classifier::Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>, u64, u64),
+{
+    pi_theta_fill_loop_overlay(classify_at, bars, initial_nav, config, chi, None, Some(voice))
 }
 
 /// ★M5 声部执行层 fill loop（多空对冲.pdf p16 关卡10）：与 [`pi_theta_fill_loop`] **同一决策路径**
@@ -1028,6 +1153,12 @@ where
 ///
 /// `overlay=None` ⟹ 净额路径 bit-exact（所有现有臂）；`Some(&mut ov)` ⟹ 逐 bar 决策点把 sep_legs
 /// 步进 overlay（**只读旁路**，不改净额 fill 的 cash/units/trade_pnls ⟹ 现有数字不动）。
+///
+/// ★W1 `voice_exec=Some`（声部独立执行臂，审计 §7）：净额影子账本（cash/units/entry_cost）
+/// **原样跑**——决策层（typed_ledger/TW/sep_legs/gate/base_units）全部读影子真值 ⟹ 与净额臂
+/// 逐字节一致；真实执行投影走声部簿（pending_voice 队列 + 事件驱动 fill + 开仓冻结 sizing），
+/// 输出层（equity/trade_pnls/trades/n_orders/r_decomp）换为声部账户口径（设计性改变，非回归）。
+/// `voice_exec=None` ⟹ 下列声部分支全部跳过，净额路径逐字节不变（bit-exact 回归锁）。
 fn pi_theta_fill_loop_overlay<F>(
     mut classify_at: F,
     bars: &[Bar],
@@ -1035,6 +1166,7 @@ fn pi_theta_fill_loop_overlay<F>(
     config: &ThetaConfig,
     chi: Option<ChiFilterCtx>,
     mut overlay: Option<&mut super::super::strategy::overlay_state::OverlayState>,
+    mut voice_exec: Option<&mut super::super::strategy::overlay_state::VoiceExecBook>,
 ) -> FillOutput
 where
     // ★工位 4g/on2w2：闭包返回四元组——第三个 u64 = 塔代次（candidate 段判据）；第四个 u64 =
@@ -1044,6 +1176,7 @@ where
     use super::super::strategy::coverage::{self, PiThetaWeights};
     use super::super::strategy::exec::fill_bar_index;
     use super::super::strategy::interp::{self, ActiveLeg};
+    use super::super::strategy::overlay_state::{VoiceOrder, VoiceOrderKind};
 
     let n = bars.len();
     let nav0 = if initial_nav > 0.0 { initial_nav } else { 1.0 };
@@ -1092,6 +1225,19 @@ where
     let mut trades: Vec<metrics::TradeRecord> = Vec::new();
     let mut pos_entry_bar: Option<usize> = None;
     let mut n_orders_executed: usize = 0;
+    // ── ★W1 声部独立执行层（churn 修复臂，netting-vs-voice-execution-audit-20260719 §7）：
+    //    Some ⟹ 决策层由净额影子账本（上方 cash/units/entry_cost 全部原有逻辑原样跑）驱动——
+    //    typed_ledger/TW/sep_legs 与净额臂逐字节一致；真实执行投影走 VoiceExecBook（声部独立
+    //    持仓 + 事件驱动 fill + 开仓冻结 sizing），输出层（equity/trade_pnls/trades/n_orders/
+    //    r_decomp）换声部账户口径。None ⟹ 本块全部变量闲置、声部分支全跳过（bit-exact 回归锁）。──
+    let voice_enabled = voice_exec.is_some();
+    let mut pending_voice: Vec<Vec<VoiceOrder>> = if voice_enabled { vec![Vec::new(); n] } else { Vec::new() };
+    let mut n_voice_fills: usize = 0;
+    let mut voice_trade_pnls: Vec<f64> = Vec::new(); // 声部平仓 fill 费后已实现（不含强平）
+    let mut voice_trades: Vec<metrics::TradeRecord> = Vec::new();
+    let mut cum_funding_v: f64 = 0.0; // 声部账户持仓成本（cost_model=None ⟹ 恒 0）
+    let mut cum_borrow_v: f64 = 0.0;
+    let mut cum_liq_v: f64 = 0.0;
     // ── G4 typed ledger（#134）：腿级在飞表（voice_id → 入场登记）+ 已结算 typed 交易。 ──
     let mut open_trades: std::collections::HashMap<
         classifier::recursive_tower::ElementId,
@@ -1177,6 +1323,11 @@ where
                 cum_price_pnl += units * (px - pp);
             }
         }
+        // ★W1：声部簿价格 PnL 累计（同一 Δpx 口径——簿内持仓是前 bar 收盘持仓，在本 bar
+        //    声部 fill 之前累计，与上方净额影子累计同一时点）。
+        if voice_enabled {
+            voice_exec.as_deref_mut().expect("voice_enabled ⟹ Some").mark_to_market(px);
+        }
 
         // ── ① 延迟成交：本 bar 到达 exec_index 的挂单 fill（apply_order，先平后开）。 ──
         if !pending[i].is_empty() && !bar.untradable && px > 0.0 {
@@ -1196,6 +1347,32 @@ where
                 }
             }
         }
+        // ── ★W1 ①-voice 声部 fill（执行投影）：本 bar 到达的声部订单逐声部成交（hedge-mode
+        //    簿，每声部独立 units_v/entry_cost_v）；平仓结算产 trade_pnls/TradeRecord 行。──
+        if voice_enabled && !pending_voice[i].is_empty() && !bar.untradable && px > 0.0 {
+            let vos = std::mem::take(&mut pending_voice[i]);
+            let vb = voice_exec.as_deref_mut().expect("voice_enabled ⟹ Some");
+            for vo in &vos {
+                let out = match vo.kind {
+                    VoiceOrderKind::Open => vb.apply_open(vo.voice, vo.side, vo.qty, px, i, fee_rate),
+                    VoiceOrderKind::Close => vb.apply_close(vo.voice, px, i, fee_rate),
+                };
+                if out.executed_qty > 0.0 {
+                    n_voice_fills += 1;
+                }
+                if let Some(cinfo) = out.closed {
+                    voice_trade_pnls.push(cinfo.pnl);
+                    voice_trades.push(metrics::TradeRecord {
+                        entry_bar: cinfo.entry_bar,
+                        exit_bar: i,
+                        hold_bars: i.saturating_sub(cinfo.entry_bar).max(1),
+                        qty: cinfo.qty as f64,
+                        long: cinfo.long,
+                        forced_close: false,
+                    });
+                }
+            }
+        }
 
         // ── M6 ①⁺ Funding + Borrow 持仓期成本计提（本 bar 成交后持仓的持有成本；px>0 才计——
         //    untradable bar（px=0）无有效 mark ⟹ 跳过计提，与 cum_price_pnl 的 px>0 累计口径
@@ -1212,6 +1389,23 @@ where
                 cum_borrow += borrow;
             }
         }
+        // ★W1 ①⁺-voice：声部账户持仓成本镜像（对 N_derived=Σσ_v q_v 名义计费，现金真扣 ⟹
+        //    守恒断言覆盖；cost_model=None ⟹ 恒 0 不调用）。触发时点与净额影子同（本 bar 成交后）。
+        if voice_enabled {
+            if let Some(cost) = config.cost_model.as_ref() {
+                let vb = voice_exec.as_deref_mut().expect("voice_enabled ⟹ Some");
+                let nv = vb.net_signed() as f64;
+                if px > 0.0 && nv != 0.0 {
+                    let net_notional_usd = nv.abs() * px;
+                    let equity_pre = vb.cash() + nv * px;
+                    let funding = cost.funding_accrual(i, net_notional_usd);
+                    let borrow = cost.borrow_accrual(net_notional_usd, equity_pre);
+                    vb.debit_cash(funding + borrow);
+                    cum_funding_v += funding;
+                    cum_borrow_v += borrow;
+                }
+            }
+        }
 
         // ── ② p_t = 净 lot（成交后真实持仓）。 ──
         let p_t = units;
@@ -1284,6 +1478,19 @@ where
                         cum_liq_loss += penalty;
                     }
                 }
+                // ★W1 ③⁻-voice：声部账户强平罚金镜像（触发边沿与净额影子同一 liq_active 去抖；
+                //    罚金名义 = |N_derived|·px——声部账户自身真实持仓，非影子 p_t）。
+                if voice_enabled {
+                    if let Some(cost) = config.cost_model.as_ref() {
+                        let vb = voice_exec.as_deref_mut().expect("voice_enabled ⟹ Some");
+                        let nv = vb.net_signed() as f64;
+                        if in_liq && !liq_active && nv != 0.0 && px > 0.0 {
+                            let penalty = cost.liquidation_penalty(nv.abs() * px);
+                            vb.debit_cash(penalty);
+                            cum_liq_v += penalty;
+                        }
+                    }
+                }
                 liq_active = in_liq;
             }
             // ── A10 C5（裁定 (b) TW 桥 G1，零账本侵入）：`cum_holding_cost` i64 shadow =
@@ -1818,6 +2058,57 @@ where
                     }
                 }
             }
+            // ── ★W1 ④-voice 声部挂单（事件驱动，churn 修复）：决策单源 = 本步 step_trace 五类
+            //    生命周期事件（与 typed_ledger 消费同源，禁第二裁决源）；无事件 bar 零订单。
+            //    sizing 冻结：q_v 取开仓 bar 决策层 SepLeg.q_units（base_units×w_depth×w_dir 已在
+            //    coverage 算好，单源复用）取整为手数，落簿后存续期不再随 NAV/价每 bar 重定
+            //    （治 runner base_units=equity_nav/px 每 bar 重算导致的声部目标漂移，审计 §4/§5）。
+            //    平单先挂、开单后挂：同 bar close→reopen 同 carrier 时 fill 序 = 先平后开
+            //    （与 apply_fill 段序同义）；跨 bar 天然按挂单方面时序排列。──
+            if voice_enabled {
+                if let Some(ei) = exec_index {
+                    if ei < n {
+                        let lot = config.risk.default_lot.max(1) as i64;
+                        for leg in step_trace
+                            .closed
+                            .iter()
+                            .map(|(l, _)| l)
+                            .chain(step_trace.silent_drops.iter())
+                            .chain(step_trace.risk_exits.iter())
+                            .chain(step_trace.overlay_closes.iter())
+                        {
+                            pending_voice[ei].push(VoiceOrder {
+                                voice: leg.id,
+                                side: leg.dir,
+                                qty: 0, // Close = fill 时刻 flatten 簿内该声部全部持仓（执行真值）
+                                kind: VoiceOrderKind::Close,
+                                exec_index: ei,
+                            });
+                        }
+                        for (c, leg) in &step_trace.opened {
+                            // B1 同源快照（typed ledger units 字段同值）：opened 腿在 sep_legs 缺席
+                            // （coverage work.get filter_map 跳过）⟹ 0.0 ⟹ q<lot 不开（诚实退化，
+                            // 与 B1 release 防御同口径——不伪造 sizing）。
+                            let q_units = step_trace
+                                .sep_legs
+                                .iter()
+                                .find(|s| s.id == leg.id)
+                                .map(|s| s.q_units)
+                                .unwrap_or(0.0);
+                            let q = (q_units / lot as f64).round() as i64 * lot;
+                            if q >= lot {
+                                pending_voice[ei].push(VoiceOrder {
+                                    voice: leg.id,
+                                    side: c.dir,
+                                    qty: q, // ★冻结：开仓时刻快照，存续期不重定
+                                    kind: VoiceOrderKind::Open,
+                                    exec_index: ei,
+                                });
+                            }
+                        }
+                    }
+                }
+            }
             // ── ⑤ thread 活动集台账（喂下一 bar interpret 闭环）+ persistent registry 合并。 ──
             // ★persistent overlay（anc.pdf §9）：Pi+1 = merge(Pi, Ei+1, held legs)。
             // 用本 bar snapshot（elements）+ held legs（next_active）刷新 registry。
@@ -1849,7 +2140,15 @@ where
         }
 
         // ── ⑥ 权益曲线（mark-to-market，归一化 ÷nav0）。 ──
-        equity_curve.push((cash + units * px) / nav0);
+        // ★W1：声部臂输出声部账户权益（cash_v + N_derived·px；设计性改变——费用路径不同）；
+        //    净额影子权益不进输出（决策层真值源，非本臂账户）。
+        let eq_mtm = if voice_enabled {
+            let vb = voice_exec.as_deref().expect("voice_enabled ⟹ Some");
+            vb.cash() + vb.net_signed() as f64 * px
+        } else {
+            cash + units * px
+        };
+        equity_curve.push(eq_mtm / nav0);
         // M6：记录本 bar 有效价供下 bar 价格 PnL 差分（px>0 才更新——untradable/零价 bar 不刷，
         // 避免 Δpx 跨越无效价产生伪价格贡献）。
         if px > 0.0 {
@@ -1867,8 +2166,36 @@ where
     }
 
     // ── 窗口终点强平（含浮盈口径，编排者铁律「不把浮盈算上不合理」；与 plan_and_fill_mtm 同）──
-    let mut trade_pnls_with_forced = trade_pnls.clone();
-    if units != 0.0 {
+    let mut trade_pnls_with_forced = if voice_enabled { voice_trade_pnls.clone() } else { trade_pnls.clone() };
+    // ★W1：声部臂 final equity 的真值 = cash_v + N_derived·final_px——在下方虚拟兑现（清空
+    //    positions）之前捕获 N_derived（cash_v 不被虚拟兑现改动，r_decomp 装配处直读）。
+    let voice_end_net: i64 = if voice_enabled {
+        voice_exec.as_deref().expect("voice_enabled ⟹ Some").net_signed()
+    } else {
+        0
+    };
+    if voice_enabled {
+        // 声部臂：逐声部强平（虚拟兑现，与净额臂 forced_pnl 同口径——不改现金、不计 fill）；
+        // forced pnl 行 + TradeRecord（forced_close=true）按声部产（条数 = 在飞声部数）。
+        if let Some(last_bar) = bars.last() {
+            let last_px = last_bar.close as f64 * config.tick.tick_size;
+            if last_px > 0.0 {
+                let exit_bar = n.saturating_sub(1);
+                let vb = voice_exec.as_deref_mut().expect("voice_enabled ⟹ Some");
+                for cinfo in vb.settle_forced_virtual(last_px, exit_bar, fee_rate) {
+                    trade_pnls_with_forced.push(cinfo.pnl);
+                    voice_trades.push(metrics::TradeRecord {
+                        entry_bar: cinfo.entry_bar,
+                        exit_bar,
+                        hold_bars: exit_bar.saturating_sub(cinfo.entry_bar).max(1),
+                        qty: cinfo.qty as f64,
+                        long: cinfo.long,
+                        forced_close: true,
+                    });
+                }
+            }
+        }
+    } else if units != 0.0 {
         if let Some(last_bar) = bars.last() {
             let last_px = last_bar.close as f64 * config.tick.tick_size;
             if last_px > 0.0 {
@@ -1937,27 +2264,46 @@ where
     // A10 C5 TW 桥对账行：= ⌊cum_funding+cum_borrow+cum_liq_loss⌋（与循环内 shadow 同一量化
     // 口径——对累计值截断，三项恒 ≥0 ⟹ 截断=⌊⌋）；cost_model=None ⟹ 0（bit-exact）。
     let tw_holding_cost_bridge: i64 = (cum_funding + cum_borrow + cum_liq_loss) as i64;
+    // ★W1：声部臂 R 分解换声部账户口径（价格 PnL/费/持仓成本/ledger_delta 全部来自声部簿
+    //    与其镜像累计器；守恒断言同一公式同样成立——审计 §7.4 要求）。tw_holding_cost_bridge
+    //    行换声部账户自身持仓成本量化（声部账户的 TW 桥对账行；净额影子 TW 账本不受影响）。
+    let (r_price, r_fee, r_fund, r_borr, r_liq, r_ledger_delta, r_bridge) = if voice_enabled {
+        let vb = voice_exec.as_deref().expect("voice_enabled ⟹ Some");
+        let final_equity_v = vb.cash() + voice_end_net as f64 * final_px;
+        (
+            vb.account_price_pnl(),
+            vb.cum_fee(),
+            cum_funding_v,
+            cum_borrow_v,
+            cum_liq_v,
+            final_equity_v - nav0,
+            (cum_funding_v + cum_borrow_v + cum_liq_v) as i64,
+        )
+    } else {
+        (cum_price_pnl, cum_fee, cum_funding, cum_borrow, cum_liq_loss, ledger_delta, tw_holding_cost_bridge)
+    };
     let r_decomp = super::super::strategy::risk::RDecomposition::assemble(
-        cum_price_pnl, cum_fee, cum_funding, cum_borrow, cum_liq_loss, ledger_delta,
-        tw_holding_cost_bridge,
+        r_price, r_fee, r_fund, r_borr, r_liq, r_ledger_delta, r_bridge,
     );
     // 守恒断言（no-patch-mentality：残差超容差 = 真实资金泄漏 bug，不静默）。容差按名义规模缩放
     // （f64 累加 O(n) 舍入；nav0 量级 + 累计项量级）——绝对容差 max(1e-6, 1e-9·(|nav0|+|price_pnl|)）。
-    let cons_tol = 1e-6_f64.max(1e-9 * (nav0.abs() + cum_price_pnl.abs()));
+    let cons_tol = 1e-6_f64.max(1e-9 * (nav0.abs() + r_price.abs()));
     debug_assert!(
         r_decomp.conservation_residual.abs() <= cons_tol,
         "M6 R 分解守恒残差 {} 超容差 {}（net_r={} vs ledger_delta={}）——资金泄漏",
-        r_decomp.conservation_residual, cons_tol, r_decomp.net_r, ledger_delta
+        r_decomp.conservation_residual, cons_tol, r_decomp.net_r, r_ledger_delta
     );
 
     let daily_returns = bar_returns(&equity_curve);
     FillOutput {
         equity_curve,
         daily_returns,
-        trade_pnls_realized: trade_pnls,
+        // ★W1：声部臂输出声部账户口径（平仓 fill 费后已实现/逐声部 TradeRecord/声部 fill
+        //    事件数）——设计性改变（审计 §7.4 如实登记），非回归；净额影子对应量不进输出。
+        trade_pnls_realized: if voice_enabled { voice_trade_pnls } else { trade_pnls },
         trade_pnls_with_forced,
-        trades,
-        n_orders: n_orders_executed,
+        trades: if voice_enabled { voice_trades } else { trades },
+        n_orders: if voice_enabled { n_voice_fills } else { n_orders_executed },
         typed_ledger,
         tw_final: Some(tw),
         r_decomp: Some(r_decomp),
@@ -5039,6 +5413,159 @@ mod tests {
         assert!(ov.account_price_pnl.is_finite(), "账户净额价格 PnL 有限");
     }
 
+    // ──────────────────────────────────────────────────────────────────────
+    //  ★★W1：声部独立执行臂（churn 修复，netting-vs-voice-execution-audit-20260719 §7）
+    // ──────────────────────────────────────────────────────────────────────
+
+    /// ★W1 决策层 bit-exact + 事件驱动 fill（审计 §7.4 验收）：同一合成信号（buy@3 确认7 →
+    /// sell@12 确认14）下，声部臂 typed_ledger/tw_final 与净额臂逐字段一致（决策单源 = 净额
+    /// 影子账本，禁第二裁决源）；fill 只在声部开/合产生——20 bar 窗恰 2 个 fill 事件（开+平），
+    /// 事件驱动上界 n_fills ≤ 2×n_voices；平仓按声部一次性结算（1 行），非逐 fill 切碎。
+    #[test]
+    fn voice_exec_event_driven_fills_decision_bitexact() {
+        let config = ThetaConfig::default();
+        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
+        let baseline = pi_theta_fill_loop(buy_then_sell(1), &bars, 1.0e6, &config, None);
+        let mut book =
+            super::super::super::strategy::overlay_state::VoiceExecBook::new(1.0e6);
+        let fill = pi_theta_fill_loop_voice(buy_then_sell(1), &bars, 1.0e6, &config, None, &mut book);
+        // ③ 决策层 bit-exact：typed ledger / TW 终态逐字段一致。
+        assert_eq!(fill.typed_ledger, baseline.typed_ledger, "决策单源 ⟹ typed_ledger bit-exact");
+        assert_eq!(fill.tw_final, baseline.tw_final, "TW 由净额影子驱动 ⟹ tw_final bit-exact");
+        // ② 事件驱动 fill：开+平 = 恰 2 个 fill 事件（声部事件率 ≪ bar 率；无事件 bar 零订单）。
+        assert_eq!(fill.n_orders, 2, "恰 开+平 2 fill，实得 {}", fill.n_orders);
+        assert_eq!(book.n_fills(), 2);
+        assert_eq!(book.total_voices(), 1, "恰 1 个声部 campaign");
+        assert!(book.n_fills() <= 2 * book.total_voices(), "事件驱动上界（§7.4 断言）");
+        // ②' sizing 冻结：毛周转 = q开+q平（存续期零重定——churn 修复直接见证；净额臂同信号
+        //    下 trade_pnls 可有多段减仓，声部臂恒 = 1 round-trip 1 结算）。
+        let closed = &book.closed_voices()[0];
+        assert_eq!(book.gross_turnover_lots() % 2, 0, "G = q开+q平 = 2q（开平对称）");
+        assert!(!closed.forced);
+        assert!(closed.fee_paid > 0.0, "开+平费实付");
+        // 按声部结算：平仓费后已实现恰 1 行 + 1 条 TradeRecord（非逐 fill 切碎）。
+        assert_eq!(fill.trade_pnls_realized.len(), 1);
+        assert_eq!(fill.trades.len(), 1);
+        assert!(!fill.trades[0].forced_close);
+        assert_eq!(fill.trade_pnls_realized[0], closed.pnl_net, "输出 PnL = 簿内声部结算行（同源）");
+        // 声部账户守恒（R 分解，loop 内 debug_assert 同锚；release 复核）。
+        let r = fill.r_decomp.expect("声部臂产 R 分解");
+        let tol = 1e-6_f64.max(1e-9 * (1.0e6 + r.price_pnl_gross.abs()));
+        assert!(r.conservation_residual.abs() <= tol, "声部账户守恒残差 {} 超容差", r.conservation_residual);
+        // 价格 PnL 对账（PDF §11）：Σpnl_price == account_price_pnl。
+        assert!((book.total_voice_price_pnl() - book.account_price_pnl()).abs() < 1e-6);
+    }
+
+    /// ★W1 sizing 冻结（churn 机制修复直接见证）：锯齿价格（NAV/px 每 bar 剧烈漂移 ⟹ 净额臂
+    /// base_units=equity_nav/px 每 bar 重算）下，声部臂仍只有 1 个 fill 事件（开仓）——q_v 开仓
+    /// 时刻冻结，存续期零重定（无事件 bar 零订单）；窗口终点虚拟强平产 1 行 forced PnL。
+    #[test]
+    fn voice_exec_frozen_sizing_under_nav_drift() {
+        let config = ThetaConfig::default();
+        // 锯齿价（与 run_theta_v0_pi_overlay_reconciles_and_bit_exact_net 同款）：px 大幅摆动
+        // ⟹ base_units 每 bar 漂移（churn 机制根源，审计 §5.1），净额臂 target 随之漂移。
+        let bars: Vec<Bar> = (0..60)
+            .map(|i| {
+                let up = ((i / 4) % 2) == 0;
+                let base = 10_000_000_000i64;
+                let step = 250_000_000i64 * ((i % 4) as i64);
+                mk_bar(i, if up { base + step } else { base + 1_000_000_000 - step }, false)
+            })
+            .collect();
+        let baseline = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
+        let mut book =
+            super::super::super::strategy::overlay_state::VoiceExecBook::new(1.0e6);
+        let fill = pi_theta_fill_loop_voice(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None, &mut book);
+        // 决策层 bit-exact（锯齿 + 漂移下仍成立——影子账本承载全部决策真值）。
+        assert_eq!(fill.typed_ledger, baseline.typed_ledger, "typed_ledger bit-exact");
+        assert_eq!(fill.tw_final, baseline.tw_final, "tw_final bit-exact");
+        // sizing 冻结核心断言：价格/ NAV 全程漂移，fill 仍只有开仓 1 次（无 churn fill）。
+        assert_eq!(fill.n_orders, 1, "冻结 sizing ⟹ 仅开仓 1 fill（存续期零重定），实得 {}", fill.n_orders);
+        assert_eq!(book.total_voices(), 1);
+        // 窗口终点在飞 1 声部 ⟹ censored 强平（虚拟兑现）：1 行 forced PnL + 1 条 forced TradeRecord。
+        assert_eq!(fill.trade_pnls_realized.len(), 0, "无平仓 fill ⟹ 无已实现行");
+        assert_eq!(fill.trade_pnls_with_forced.len(), 1, "强平含浮盈 1 行");
+        assert_eq!(fill.trades.len(), 1);
+        assert!(fill.trades[0].forced_close);
+        assert!(book.closed_voices()[0].forced);
+        // 守恒（含强平在飞持仓 MtM 的 ledger_delta 口径）。
+        let r = fill.r_decomp.expect("R 分解");
+        let tol = 1e-6_f64.max(1e-9 * (1.0e6 + r.price_pnl_gross.abs()));
+        assert!(r.conservation_residual.abs() <= tol, "守恒残差 {} 超容差", r.conservation_residual);
+        assert!((book.total_voice_price_pnl() - book.account_price_pnl()).abs() < 1e-6, "Σpnl_price 对账");
+    }
+
+    /// ★W1 无信号零订单（事件驱动的空集情形）：全程无分类事件 ⟹ opened/closed 全空 ⟹
+    /// 声部簿零 fill、零持仓、零费用（不伪造执行事实）。
+    #[test]
+    fn voice_exec_no_signal_zero_fills() {
+        let config = ThetaConfig::default();
+        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
+        let mut book =
+            super::super::super::strategy::overlay_state::VoiceExecBook::new(1.0e6);
+        let fill = pi_theta_fill_loop_voice(
+            |i| (Classification::default(), Vec::new(), i as u64, i as u64),
+            &bars,
+            1.0e6,
+            &config,
+            None,
+            &mut book,
+        );
+        assert_eq!(fill.n_orders, 0);
+        assert_eq!(book.n_fills(), 0);
+        assert_eq!(book.gross_turnover_lots(), 0);
+        assert!(fill.trade_pnls_with_forced.is_empty());
+        assert!(fill.equity_curve.iter().all(|&e| e == 1.0), "无持仓零费用 ⟹ 权益恒 1");
+    }
+
+    /// ★W1 env gate（④⑤）：VOICE_EXEC 关闭 ⟹ 生产路径逐字节不变（bit-exact 回归锁）；
+    /// 开启 ⟹ 声部执行读数装配 + 决策层 TW 与净额 bit-exact + 验收量（上界/守恒/对账）成立。
+    /// 线程局部注入（并行安全；进程级 env 不动——OPSEM_DUMP_DIR_OVERRIDE 同惯例）。
+    #[test]
+    fn voice_exec_env_gate_off_bitexact_on_voice_readings() {
+        let config = ThetaConfig::default();
+        let bars: Vec<Bar> = (0..60)
+            .map(|i| {
+                let up = ((i / 4) % 2) == 0;
+                let base = 10_000_000_000i64;
+                let step = 250_000_000i64 * ((i % 4) as i64);
+                mk_bar(i, if up { base + step } else { base + 1_000_000_000 - step }, false)
+            })
+            .collect();
+        let ds = Dataset {
+            symbol: "ZZ60W1".to_string(),
+            bars,
+            dates: (0..60).map(|i| format!("2024-02-{:02} 00:00:00", (i % 28) + 1)).collect(),
+            bar_seconds: 60,
+        };
+        let baseline = run_theta_v0_pi(&ds, &config, 1.0, 1.0e6);
+        // (1) gate 关 ⟹ 逐字节不变（env 未设语义，线程局部注入 false 显式锚定）。
+        VOICE_EXEC_OVERRIDE.with(|c| c.set(Some(false)));
+        let off = run_theta_v0_pi_overlay(&ds, &config, 1.0, 1.0e6);
+        assert!(off.voice_exec.is_none(), "gate 关 ⟹ 无声部读数（None，不伪造）");
+        assert_eq!(off.net_result.n_orders, baseline.n_orders, "gate 关 ⟹ n_orders bit-exact");
+        assert_eq!(off.net_result.trades, baseline.trades, "trades bit-exact");
+        assert_eq!(
+            off.net_result.metrics.strat_return, baseline.metrics.strat_return,
+            "strat_return bit-exact"
+        );
+        assert_eq!(off.net_result.equity_curve, baseline.equity_curve, "equity bit-exact");
+        // (2) gate 开 ⟹ 声部执行读数；决策层 TW 与净额 bit-exact。
+        VOICE_EXEC_OVERRIDE.with(|c| c.set(Some(true)));
+        let on = run_theta_v0_pi_overlay(&ds, &config, 1.0, 1.0e6);
+        VOICE_EXEC_OVERRIDE.with(|c| c.set(None));
+        let vs = on.voice_exec.expect("VOICE_EXEC=1 ⟹ 声部执行读数 Some");
+        assert_eq!(on.tw_final, off.tw_final, "TW 由净额影子驱动 ⟹ gate 开/关 bit-exact");
+        assert_eq!(vs.n_voice_fills, on.net_result.n_orders, "n_orders = 声部 fill 事件数（同源）");
+        assert!(vs.event_bound_holds, "n_voice_fills({}) ≤ 2×n_voices({})", vs.n_voice_fills, vs.n_voices_total);
+        assert_eq!(vs.n_voice_fills, vs.book.n_fills(), "读数与簿同源");
+        let tol = 1e-6_f64.max(1e-9 * 1.0e6);
+        assert!(vs.conservation_residual.abs() <= tol, "声部账户守恒残差 {} 超容差", vs.conservation_residual);
+        assert!(vs.price_pnl_reconcile_residual < 1e-6, "价格 PnL 对账残差 {}", vs.price_pnl_reconcile_residual);
+        // 事件驱动上界（构造性）：fill 数 ≤ 2×声部数（resize 事件默认空集）。
+        assert!(vs.n_voice_fills <= 2 * vs.n_voices_total, "事件驱动上界");
+    }
+
     /// ★on2w3 merge skip 神谕深覆盖（debug 构建）：真实 CL 全引擎 run_theta_v0_pi。生产路径 forest_epoch
     /// 稳定（bump 率 2.17%）⟹ merge cand 段大量走 skip；`merge_in_place_split` 内嵌 debug_assert 逐 bar
     /// 对拍 skip vs 强制全量末态——任一 bar 发散立即 panic（O(n²) 修复的 bit-exact 铁律守卫）。
diff --git a/rust/src/theta_v0/strategy/overlay_state.rs b/rust/src/theta_v0/strategy/overlay_state.rs
index 39844aafb1..0e266c3639 100644
--- a/rust/src/theta_v0/strategy/overlay_state.rs
+++ b/rust/src/theta_v0/strategy/overlay_state.rs
@@ -271,6 +271,380 @@ impl OverlayState {
     }
 }
 
+// ─────────────────────────────────────────────────────────────────────────────
+// ★W1 声部独立执行簿 `VoiceExecBook`（churn 修复臂；netting-vs-voice-execution-audit-20260719 §7）
+//
+// 与上方 `OverlayState`（每 bar rebalance 到目标 P^sep 的**只读旁路**簿）范畴不同：本簿是
+// **事件驱动**的真实执行投影——
+//   ① 声部独立持仓（hedge-mode (Q⁺,Q⁻)，PDF §10.2：对冲两腿各自存续、各自结算，不互相湮灭）；
+//   ② fill 只在声部开/合事件产生（事件驱动；不再每 bar 净 ΔN 兜底——churn 机制根源的修复，
+//      审计 §4/§5：sizing 基准 base_units=equity_nav/px 每 bar 重算致每声部目标每 bar 漂移）；
+//   ③ sizing 冻结：q_v 在开仓 fill 落簿时定死（调度侧取开仓 bar 决策层 SepLeg.q_units 取整），
+//      存续期不随 NAV/价重定（审计 §6.1：base_units 从「每 bar 协变量」降级为「开仓时刻快照」）。
+//
+// 费口径：成交费 = 成交名义 × fee_rate（与 runner `apply_fill` :3778 同口径，逐段独立累计
+// `cum_fee`）；`pnl_price` 是逐 bar MtM **价格** PnL（σ_v·q_v·ΔP 累计），`pnl_net` 是平仓
+// 费后已实现 PnL（apply_fill 段1 同口径：proceeds − 含费成本基）——两字段分立，禁互相冒充
+// （090；审计 §7.3「禁复用 pnl_v 冒充费后」同款）。守恒不变量（runner 侧 R 分解断言锚）：
+// cash + N·px − nav0 == account_price_pnl − cum_fee − 持仓成本（构造性，逐段现金流对齐）。
+// ─────────────────────────────────────────────────────────────────────────────
+
+/// 声部执行订单类型（执行投影层订单；≠ `types.rs Order` 净额决策层订单——决策层类型不改）。
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum VoiceOrderKind {
+    /// 开仓（qty = 冻结 q_v，手数）。
+    Open,
+    /// 平仓（qty 字段不用；fill 时刻 flatten 簿内该声部全部持仓——执行真值口径：
+    /// 延迟成交期间簿内持仓才是真值，事件时刻簿内可能尚未成交开仓单）。
+    Close,
+}
+
+/// 声部执行订单（执行投影层；voice 身份在订单上留痕——净额 `Order` 三字段无身份的对照修复）。
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub struct VoiceOrder {
+    /// carrier v 身份（= `ActiveLeg::id` / `SepLeg::id`）。
+    pub voice: ElementId,
+    /// σ_v：声部方向（Open 的消费字段；Close 不用，占位记录事件腿方向）。
+    pub side: VoiceSide,
+    /// Open = 冻结 q_v（手数，开仓 bar 决策层 `SepLeg.q_units` 取整）；Close = 0（flatten 语义）。
+    pub qty: i64,
+    pub kind: VoiceOrderKind,
+    /// 延迟成交 bar（与净额 `Order.exec_index` 同语义，spec:50）。
+    pub exec_index: usize,
+}
+
+/// 声部在簿持仓（hedge-mode 单向腿 + 执行真值归因）。
+#[derive(Debug, Clone)]
+pub struct VoicePosition {
+    pub id: ElementId,
+    /// σ_v（本簿 hedge-mode 每声部单向，入场固定）。
+    pub side: VoiceSide,
+    /// q_v：**冻结**手数（开仓 fill 落簿时定死，存续期不变——churn 修复核心）。
+    pub q: i64,
+    /// 开仓 fill bar（执行真值，非决策 bar）。
+    pub entry_bar: usize,
+    /// 含费单位成本基 = px_fill×(1+δ·fee_rate)（`apply_fill` 段2 unit_cost 同口径）。
+    pub entry_px: f64,
+    /// 逐 bar 累计 MtM **价格** PnL（σ_v·q_v·ΔP；PDF §11 对账分量，毛口径不含费）。
+    pub pnl_price: f64,
+    /// 该声部实付成交费累计（开仓段；平仓段结算时并入 `ClosedVoiceExec.fee_paid`）。
+    pub fee_paid: f64,
+}
+
+/// 已结算声部执行归因行（一个 campaign 的完整生命周期：开→平/强平）。
+#[derive(Debug, Clone)]
+pub struct ClosedVoiceExec {
+    pub id: ElementId,
+    pub side: VoiceSide,
+    /// 开仓 fill bar。
+    pub entry_bar: usize,
+    /// 平仓 fill bar（forced=窗口终点虚拟兑现 bar）。
+    pub exit_bar: usize,
+    /// 含费开仓成本基。
+    pub entry_px: f64,
+    /// 平仓成交价（forced=末可交易 bar close）。
+    pub exit_px: f64,
+    /// 生命周期 MtM 价格 PnL（毛，σ_v·q_v·ΔP 累计）。
+    pub pnl_price: f64,
+    /// 实付成交费（开+平；forced 的假设性平仓费未实付、不入本字段——见 settle_forced_virtual）。
+    pub fee_paid: f64,
+    /// 费后已实现 PnL（apply_fill 段1 同口径；forced=虚拟兑现口径，含假设性平仓费净额）。
+    pub pnl_net: f64,
+    /// 窗口终点强平（虚拟兑现，含浮盈口径）标记。
+    pub forced: bool,
+}
+
+/// 声部平仓结算信息（runner 装配 trade_pnls / TradeRecord 的输入）。
+#[derive(Debug, Clone, Copy)]
+pub struct VoiceCloseInfo {
+    /// 开仓 fill bar。
+    pub entry_bar: usize,
+    /// 平掉的手数。
+    pub qty: i64,
+    /// 方向（true=多）。
+    pub long: bool,
+    /// 费后已实现 PnL。
+    pub pnl: f64,
+}
+
+/// 声部 fill 产出（executed_qty=0 ⟹ 未成交（拒开/无仓可平），不计 fill 事件）。
+#[derive(Debug, Clone, Copy)]
+pub struct VoiceFillOutcome {
+    pub executed_qty: f64,
+    /// 平仓分量（Some ⟹ 本 fill 结算了一个声部 campaign）。
+    pub closed: Option<VoiceCloseInfo>,
+}
+
+/// ★W1 声部独立执行簿（事件驱动 + sizing 冻结；hedge-mode position book，PDF §10.2）。
+#[derive(Debug, Clone)]
+pub struct VoiceExecBook {
+    /// 在簿声部持仓（键 = carrier id）。
+    positions: HashMap<ElementId, VoicePosition>,
+    /// 已结算声部归因表。
+    closed: Vec<ClosedVoiceExec>,
+    /// 声部账户现金（nav0 起；真实执行投影的现金真值）。
+    cash: f64,
+    /// 账户基线（runner nav0 同口径：initial_nav>0 ? initial_nav : 1.0）。
+    nav0: f64,
+    /// 上一有效价（MtM 累计用；px≤0 不刷新，与 runner prev_px 同口径）。
+    last_px: Option<f64>,
+    /// 账户级累计价格 PnL（Σ_t N_t·ΔP_t，N=Σσ_v q_v 派生净敞口）。
+    account_price_pnl: f64,
+    /// 累计成交费（Commission+Slippage+Tax，逐 fill 独立测得——R 分解守恒断言锚）。
+    cum_fee: f64,
+    /// fill 事件数（executed>0 才计；= runner 输出 n_orders 在声部臂下的值）。
+    n_fills: usize,
+    /// 开过的声部 campaign 总数（含在飞）。
+    total_voices: usize,
+    /// 声部毛周转（手数）G = Σ(q开+q平)（事件驱动下 = 实际成交周转 T，审计 §7.4 验收量）。
+    gross_turnover_lots: i64,
+}
+
+impl VoiceExecBook {
+    pub fn new(initial_nav: f64) -> Self {
+        let nav0 = if initial_nav > 0.0 { initial_nav } else { 1.0 };
+        VoiceExecBook {
+            positions: HashMap::new(),
+            closed: Vec::new(),
+            cash: nav0,
+            nav0,
+            last_px: None,
+            account_price_pnl: 0.0,
+            cum_fee: 0.0,
+            n_fills: 0,
+            total_voices: 0,
+            gross_turnover_lots: 0,
+        }
+    }
+
+    pub fn cash(&self) -> f64 {
+        self.cash
+    }
+    pub fn nav0(&self) -> f64 {
+        self.nav0
+    }
+    /// N_derived = Σ_v σ_v·q_v（派生只读净敞口；权益 MtM 与持仓成本计费用）。
+    pub fn net_signed(&self) -> i64 {
+        self.positions.values().map(|p| side_sign(p.side) * p.q).sum()
+    }
+    pub fn account_price_pnl(&self) -> f64 {
+        self.account_price_pnl
+    }
+    pub fn cum_fee(&self) -> f64 {
+        self.cum_fee
+    }
+    pub fn n_fills(&self) -> usize {
+        self.n_fills
+    }
+    pub fn total_voices(&self) -> usize {
+        self.total_voices
+    }
+    pub fn gross_turnover_lots(&self) -> i64 {
+        self.gross_turnover_lots
+    }
+    /// 窗口终点仍在飞的声部数（censored）。
+    pub fn n_open_end(&self) -> usize {
+        self.positions.len()
+    }
+    pub fn positions(&self) -> impl Iterator<Item = &VoicePosition> {
+        self.positions.values()
+    }
+    pub fn closed_voices(&self) -> &[ClosedVoiceExec] {
+        &self.closed
+    }
+    /// Σ_v pnl_price（活动 + 已结算；PDF §11 对账分账本侧，应 ≈ account_price_pnl）。
+    pub fn total_voice_price_pnl(&self) -> f64 {
+        let active: f64 = self.positions.values().map(|p| p.pnl_price).sum();
+        let closed: f64 = self.closed.iter().map(|c| c.pnl_price).sum();
+        active + closed
+    }
+
+    /// 逐 bar MtM 价格 PnL 累计（runner 每 bar **成交前**调用——簿内持仓是前 bar 收盘
+    /// 持仓，与净额臂 `cum_price_pnl += units·Δpx`（runner.rs 循环顶）同一时点口径）。
+    pub fn mark_to_market(&mut self, px: f64) {
+        if px > 0.0 {
+            if let Some(lp) = self.last_px {
+                let dp = px - lp;
+                if dp != 0.0 {
+                    let net = self.net_signed();
+                    self.account_price_pnl += net as f64 * dp;
+                    for p in self.positions.values_mut() {
+                        p.pnl_price += side_sign(p.side) as f64 * p.q as f64 * dp;
+                    }
+                }
+            }
+            self.last_px = Some(px);
+        }
+    }
+
+    /// 开仓 fill（q_lots 已在调度侧冻结；现金约束与 `apply_fill` 段2 同口径——开多需现金
+    /// 充足否则拒开（executed=0），开空收现金无预付）。
+    pub fn apply_open(
+        &mut self,
+        id: ElementId,
+        side: VoiceSide,
+        q_lots: i64,
+        px: f64,
+        bar: usize,
+        fee_rate: f64,
+    ) -> VoiceFillOutcome {
+        let noop = VoiceFillOutcome { executed_qty: 0.0, closed: None };
+        if q_lots <= 0 || px <= 0.0 {
+            return noop;
+        }
+        // 防御性：同 carrier 已有在簿持仓。正常路径不发生（close→reopen 由 Close 订单先平，
+        // runner 调度同 bar 平单先于开单）；release 兜底先按本价结算旧 campaign 再开新。
+        debug_assert!(
+            !self.positions.contains_key(&id),
+            "W1: 声部 {:?} 开仓时在簿已有持仓（close 事件缺失/排序违例）",
+            id
+        );
+        let replaced = if self.positions.contains_key(&id) {
+            self.flatten(id, px, bar, fee_rate, false).map(|(info, _fee)| info)
+        } else {
+            None
+        };
+        let delta = side_sign(side) as f64;
+        if delta == 0.0 {
+            return VoiceFillOutcome { executed_qty: 0.0, closed: replaced }; // Flat 不开仓
+        }
+        let qty = q_lots as f64;
+        let cost = qty * px * (1.0 + fee_rate);
+        if delta > 0.0 && self.cash < cost {
+            return VoiceFillOutcome { executed_qty: 0.0, closed: replaced }; // 现金不足拒开
+        }
+        // 开仓现金流（apply_fill 段2 同口径）：cash += −δ·qty·px·(1+δ·fee)。
+        let cash_flow = -delta * qty * px * (1.0 + delta * fee_rate);
+        let fee = qty * px * fee_rate;
+        self.cash += cash_flow;
+        self.cum_fee += fee;
+        self.positions.insert(id, VoicePosition {
+            id,
+            side,
+            q: q_lots, // ★冻结：存续期不再重定（churn 修复）
+            entry_bar: bar,
+            entry_px: px * (1.0 + delta * fee_rate),
+            pnl_price: 0.0,
+            fee_paid: fee,
+        });
+        self.n_fills += 1;
+        self.total_voices += 1;
+        self.gross_turnover_lots += q_lots;
+        VoiceFillOutcome { executed_qty: qty, closed: replaced }
+    }
+
+    /// 平仓 fill：flatten 簿内该声部全部持仓（无持仓 ⟹ no-op executed=0——restore 祖先腿/
+    /// 开仓单未成交等情形，与 typed ledger「表中无登记 ⟹ 不入 ledger」同语义）。
+    pub fn apply_close(
+        &mut self,
+        id: ElementId,
+        px: f64,
+        bar: usize,
+        fee_rate: f64,
+    ) -> VoiceFillOutcome {
+        match self.flatten(id, px, bar, fee_rate, false) {
+            Some((info, _fee)) => {
+                self.n_fills += 1;
+                VoiceFillOutcome { executed_qty: info.qty as f64, closed: Some(info) }
+            }
+            None => VoiceFillOutcome { executed_qty: 0.0, closed: None },
+        }
+    }
+
+    /// 内部平仓（真实现金流；apply_fill 段1 同口径）。返回 (结算信息, 平仓费)。
+    fn flatten(
+        &mut self,
+        id: ElementId,
+        px: f64,
+        bar: usize,
+        fee_rate: f64,
+        forced: bool,
+    ) -> Option<(VoiceCloseInfo, f64)> {
+        let pos = self.positions.remove(&id)?;
+        let sigma = side_sign(pos.side) as f64;
+        let qty = pos.q as f64;
+        // 平仓现金流：平多（σ+1）+qty·px·(1−fee)；平空（σ−1）−qty·px·(1+fee)。
+        let cash_flow = sigma * qty * px * (1.0 - sigma * fee_rate);
+        let fee = qty * px * fee_rate;
+        let px_exit_net = px * (1.0 - sigma * fee_rate);
+        // 费后已实现 PnL = σ·(px_exit_net − 含费成本基)·qty（apply_fill 段1 同口径）。
+        let pnl = sigma * (px_exit_net - pos.entry_px) * qty;
+        self.cash += cash_flow;
+        self.cum_fee += fee;
+        self.gross_turnover_lots += pos.q;
+        let fee_total = pos.fee_paid + fee;
+        self.closed.push(ClosedVoiceExec {
+            id,
+            side: pos.side,
+            entry_bar: pos.entry_bar,
+            exit_bar: bar,
+            entry_px: pos.entry_px,
+            exit_px: px,
+            pnl_price: pos.pnl_price,
+            fee_paid: fee_total,
+            pnl_net: pnl,
+            forced,
+        });
+        Some((
+            VoiceCloseInfo { entry_bar: pos.entry_bar, qty: pos.q, long: pos.side == VoiceSide::Long, pnl },
+            fee,
+        ))
+    }
+
+    /// 窗口终点强平（含浮盈口径，与净额臂 `forced_pnl` 同铁律）：**虚拟兑现**——不改 cash、
+    /// 不计 fill、不计费（净额臂 forced_pnl 同样不动 cash/units，final equity 按持仓 MtM 计）；
+    /// 逐声部产 forced pnl 行 + 归因行入 closed（forced=true；fee_paid 只记实付开仓费，
+    /// 假设性平仓费已净入 pnl_net 但未实付、不入 fee_paid——090 口径分立）。
+    /// 返回确定序（entry_bar, level, ordinal 排序，bit-exact 可复现）。
+    pub fn settle_forced_virtual(
+        &mut self,
+        last_px: f64,
+        exit_bar: usize,
+        fee_rate: f64,
+    ) -> Vec<VoiceCloseInfo> {
+        let mut rows: Vec<(usize, u32, u64, ElementId)> = self
+            .positions
+            .values()
+            .map(|p| (p.entry_bar, p.id.level, p.id.ordinal, p.id))
+            .collect();
+        rows.sort_by_key(|(eb, lvl, ord, _)| (*eb, *lvl, *ord));
+        let mut out = Vec::with_capacity(rows.len());
+        for (_, _, _, id) in rows {
+            if let Some(pos) = self.positions.remove(&id) {
+                let sigma = side_sign(pos.side) as f64;
+                let qty = pos.q as f64;
+                let px_exit_net = last_px * (1.0 - sigma * fee_rate);
+                let pnl = sigma * (px_exit_net - pos.entry_px) * qty;
+                self.closed.push(ClosedVoiceExec {
+                    id,
+                    side: pos.side,
+                    entry_bar: pos.entry_bar,
+                    exit_bar,
+                    entry_px: pos.entry_px,
+                    exit_px: last_px,
+                    pnl_price: pos.pnl_price,
+                    fee_paid: pos.fee_paid,
+                    pnl_net: pnl,
+                    forced: true,
+                });
+                out.push(VoiceCloseInfo {
+                    entry_bar: pos.entry_bar,
+                    qty: pos.q,
+                    long: pos.side == VoiceSide::Long,
+                    pnl,
+                });
+            }
+        }
+        out
+    }
+
+    /// 持仓成本/罚金扣款（funding/borrow/liq；runner 镜像净额臂口径对 N_derived 计费后经此
+    /// 扣声部账户现金——守恒断言要求 cash 真扣）。cost_model=None ⟹ 永不调用（bit-exact）。
+    pub fn debit_cash(&mut self, amount: f64) {
+        self.cash -= amount;
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
@@ -357,4 +731,119 @@ mod tests {
         assert_eq!(s.net_after, 0, "0.4 手 round→0 ⟹ 不进 P^sep");
         assert_eq!(ov.active_voices().count(), 0);
     }
+
+    // ──────────────────────────────────────────────────────────────────────
+    //  ★W1 VoiceExecBook（声部独立执行簿：事件驱动 + sizing 冻结）
+    // ──────────────────────────────────────────────────────────────────────
+
+    /// ★W1 守恒（验收）：开多 10@100 → MtM 110 → 平@110，fee=3bps。
+    /// 账本净变动（cash−nav0，N=0）== 价格 PnL − 成交费（构造性恒等，逐段现金流对齐）。
+    #[test]
+    fn voice_exec_open_close_conservation() {
+        let fee = 0.0003;
+        let mut vb = VoiceExecBook::new(1.0e6);
+        let v = eid(1, 0);
+        vb.mark_to_market(100.0); // bar0：首价，无 ΔP
+        let o = vb.apply_open(v, VoiceSide::Long, 10, 100.0, 0, fee);
+        assert_eq!(o.executed_qty, 10.0);
+        assert_eq!(vb.net_signed(), 10);
+        assert_eq!(vb.positions().next().unwrap().q, 10, "q_v 冻结为开仓手数");
+        vb.mark_to_market(110.0); // ΔP=+10：account += 10·10=100；pnl_price += 100
+        assert!((vb.account_price_pnl() - 100.0).abs() < 1e-9);
+        let c = vb.apply_close(v, 110.0, 1, fee);
+        assert_eq!(c.executed_qty, 10.0);
+        let info = c.closed.expect("平仓结算");
+        // 费后 PnL = (110·0.9997 − 100·1.0003)·10 = (109.967−100.03)·10 = 99.37。
+        assert!((info.pnl - 99.37).abs() < 1e-9, "费后 PnL={}", info.pnl);
+        assert_eq!(vb.net_signed(), 0);
+        assert_eq!(vb.n_fills(), 2, "开+平 = 2 个 fill 事件（事件驱动上界 2×1 声部）");
+        assert_eq!(vb.gross_turnover_lots(), 20, "G = q开+q平 = 20 手");
+        // 守恒：cash−nav0 == account_price_pnl − cum_fee（N=0 终点）。
+        let fee_total = 10.0 * 100.0 * fee + 10.0 * 110.0 * fee; // 0.30+0.33=0.63
+        assert!((vb.cum_fee() - fee_total).abs() < 1e-9);
+        let ledger_delta = vb.cash() - vb.nav0();
+        let net_r = vb.account_price_pnl() - vb.cum_fee();
+        assert!((ledger_delta - net_r).abs() < 1e-9, "守恒：{} vs {}", ledger_delta, net_r);
+        // Σpnl_price 对账（PDF §11）：唯一声部 pnl_price=100 == account。
+        assert!((vb.total_voice_price_pnl() - vb.account_price_pnl()).abs() < 1e-9);
+        assert_eq!(vb.closed_voices().len(), 1);
+        assert!(!vb.closed_voices()[0].forced);
+    }
+
+    /// ★W1 声部独立持仓守恒（hedge-mode，PDF §10.2）：父多 10 + 子空 6 同时存续（不互相
+    /// 湮灭），N_derived=Σσ_v q_v=4 与声部簿逐声部之和一致；各自独立结算。
+    #[test]
+    fn voice_exec_hedged_voices_independent_books() {
+        let fee = 0.0003;
+        let mut vb = VoiceExecBook::new(1.0e6);
+        let parent = eid(2, 0);
+        let child = eid(1, 0);
+        vb.mark_to_market(100.0);
+        vb.apply_open(parent, VoiceSide::Long, 10, 100.0, 0, fee);
+        vb.apply_open(child, VoiceSide::Short, 6, 100.0, 0, fee);
+        // 守恒断言：Σ_v σ_v·q_v（簿派生）== 逐声部手数有符号和。
+        let sum: i64 = vb.positions().map(|p| match p.side {
+            VoiceSide::Long => p.q,
+            VoiceSide::Short => -p.q,
+            VoiceSide::Flat => 0,
+        }).sum();
+        assert_eq!(vb.net_signed(), 4, "对冲两腿毛额存续，净敞口 10−6=4（不湮灭）");
+        assert_eq!(vb.net_signed(), sum, "Σ_v σ_v q_v 与声部簿一致");
+        vb.mark_to_market(110.0); // account += 4·10=40；父+100，子−60
+        assert!((vb.account_price_pnl() - 40.0).abs() < 1e-9);
+        // 各自独立结算：只平空腿，多腿存续。
+        vb.apply_close(child, 110.0, 1, fee);
+        assert_eq!(vb.net_signed(), 10, "空腿已平，多腿独立存续");
+        assert_eq!(vb.closed_voices().len(), 1);
+        assert_eq!(vb.n_open_end(), 1);
+        assert!((vb.total_voice_price_pnl() - vb.account_price_pnl()).abs() < 1e-9, "Σpnl_price 对账");
+    }
+
+    /// ★W1 窗口终点强平（虚拟兑现）：不改 cash、不计 fill、不计费；pnl 含假设性平仓费净额；
+    /// 归因行入 closed（forced=true）。
+    #[test]
+    fn voice_exec_forced_settle_virtual_no_cash() {
+        let fee = 0.0003;
+        let mut vb = VoiceExecBook::new(1.0e6);
+        let v = eid(1, 0);
+        vb.mark_to_market(100.0);
+        vb.apply_open(v, VoiceSide::Long, 10, 100.0, 0, fee);
+        vb.mark_to_market(110.0);
+        let cash_before = vb.cash();
+        let fee_before = vb.cum_fee();
+        let fills_before = vb.n_fills();
+        let rows = vb.settle_forced_virtual(110.0, 19, fee);
+        assert_eq!(rows.len(), 1);
+        // 虚拟兑现 pnl = (110·0.9997 − 100·1.0003)·10 = 99.37（与真平同价同额）。
+        assert!((rows[0].pnl - 99.37).abs() < 1e-9);
+        assert_eq!(vb.cash(), cash_before, "虚拟兑现不改现金（与净额臂 forced_pnl 同口径）");
+        assert_eq!(vb.cum_fee(), fee_before, "假设性平仓费未实付");
+        assert_eq!(vb.n_fills(), fills_before, "强平虚拟兑现不计 fill 事件");
+        assert_eq!(vb.n_open_end(), 0);
+        assert!(vb.closed_voices()[0].forced);
+        assert!((vb.closed_voices()[0].fee_paid - 10.0 * 100.0 * fee).abs() < 1e-9,
+            "fee_paid 只记实付开仓费");
+    }
+
+    /// ★W1 现金约束（apply_fill 段2 同口径）：开多现金不足 ⟹ 拒开（executed=0，不计 fill、
+    /// 不入簿、不计费——不伪造执行事实）。
+    #[test]
+    fn voice_exec_open_rejected_on_insufficient_cash() {
+        let mut vb = VoiceExecBook::new(100.0);
+        let o = vb.apply_open(eid(1, 0), VoiceSide::Long, 10, 100.0, 0, 0.0003);
+        assert_eq!(o.executed_qty, 0.0, "10 手×100×1.0003=1000.3 > 现金 100 ⟹ 拒开");
+        assert_eq!(vb.n_fills(), 0);
+        assert_eq!(vb.net_signed(), 0);
+        assert_eq!(vb.cum_fee(), 0.0);
+    }
+
+    /// ★W1 平仓无持仓 no-op（restore 祖先腿/未成交开仓单的 close 事件）：executed=0，不计 fill。
+    #[test]
+    fn voice_exec_close_without_position_noop() {
+        let mut vb = VoiceExecBook::new(1.0e6);
+        let o = vb.apply_close(eid(9, 9), 100.0, 3, 0.0003);
+        assert_eq!(o.executed_qty, 0.0);
+        assert!(o.closed.is_none());
+        assert_eq!(vb.n_fills(), 0);
+    }
 }
```
