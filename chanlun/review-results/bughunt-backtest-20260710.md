# 回测/资金核算深度 Bug 审计报告

审计对象：`gap3-rework-codex9-fix` @ `bf1854d71960`

范围：`rust/src/theta_v0/backtest/` 的 runner、treasury、Realize、费用/滑点、capture_oos、wverify_run、segment_gn 及直接依赖的执行/指标代码。

验证：

- 39 个相关定向测试全部通过。
- 对资金舍入、终点强平、分批减仓、止损跳空构造了最小数值反例。
- 审计范围内工作树无未提交修改。
- 共确认 11 项：P0 × 3、P1 × 7、P2 × 1。

严重级定义：

- P0：使回测核心结果或 OOS 有效性失真，应阻断结论使用。
- P1：可实质改变资金、PnL、状态或统计结论。
- P2：边界条件或局部窗口错误，影响有限但确定存在。

---

## 1. P0：旧 `run_theta_v0` 用全窗分类结果回放历史，存在结构确认前视

**file:line**

- [rust/src/theta_v0/backtest/runner.rs:248](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:248)
- [rust/src/theta_v0/backtest/runner.rs:330](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:330)

**问题**

`run_theta_v0` 先对完整 `bars` 调用 `parse_layer` 和 `classify`，再把所得 BSP 决策放回其历史 `source_index` 执行。买卖点可能在更晚 bar 才获得确认，因此该路径会在当时尚不可知的位置交易。

**证据**

- 248–257 行直接对完整数据解析、分类、识别。
- 同文件 330–334 行明确说明：全窗分类可能使用 `>i` 数据确认，执行层禁止使用；BSP 的 `source_index` 往往早于实际确认 bar。
- 因果修复只存在于 `run_theta_v0_pi` 的逐 bar `IncrementalClassifier` 路径，未覆盖旧入口。
- 现有 `run_theta_v0_pi_prefix_classify_is_causal_no_lookahead` 测试只验证 π 路径，不能保护旧路径。

**影响**

旧入口产生的订单数、交易 PnL、Sharpe、OOS 结论均可能包含确认前视。`is_l2 = n_orders > 0` 仍会把它标成 L2。

**最小修复建议**

让 `run_theta_v0` 同样按 bar 前缀部署“本 bar 新确认”的 BSP；若旧入口只保留作诊断，应禁止其产出 L2/L3 声明，并从结论型测试中移除。

---

## 2. P0：runner 在目标 bar 的 close 成交，违背冻结的“下一可交易 open”与止损成交规则

**file:line**

- [rust/src/theta_v0/backtest/runner.rs:1031](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1031)
- [rust/src/theta_v0/backtest/runner.rs:2516](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:2516)
- [rust/src/theta_v0/backtest/runner.rs:2831](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:2831)
- [rust/src/theta_v0/strategy/exec.rs:32](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/exec.rs:32)
- [rust/src/theta_v0/strategy/exit.rs:130](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/exit.rs:130)

**问题**

冻结执行协议要求普通订单在延迟后的下一可交易 bar 的 `open` 成交；runner 三条 fill 路径却统一使用该 bar 的 `close`。止损也只生成普通延迟 Close 订单，没有采用当前 bar 的跳空 open/stop 成交价。

**证据**

- `exec::fill_bar_index` 文档及实现规定“下一可交易 open”。
- runner 1033、2518、2834 行均取 `bar.close` 作为 `apply_order` 的成交价。
- `exit_decision_for` 检测止损后把 `signal_index=i` 的普通退出决策放入延迟队列；实际仍在后续 bar close 成交。
- `exec::stop_fill_price` 已实现正确规则，但 runner 没有用它。
- runner 也绕过 `exec::apply_fees` 的 tick 舍入成交价，改用连续比例现金扣费，无法保持执行层声明的 bit-exact 口径。

**最小反例**

止损价 90，当根跳空 open=85，后续延迟成交 close=95：

- 冻结协议：85 出场。
- `capture_oos`：90 出场。
- runner：可能在后续 close=95 出场。

三种资金结果完全不同。

**最小修复建议**

建立唯一 fill 入口：

1. 普通订单在 `fill_bar_index` 对应 bar 的 `open` 成交；
2. 止损在触发 bar 立即用 `stop_fill_price`；
3. 使用同一个 `apply_fees`/费用舍入口径；
4. close 仅用于成交后的 MtM。

---

## 3. P0：1 分钟 bar return 被当成日收益并按 `√252` 年化

**file:line**

- [rust/src/theta_v0/backtest/runner.rs:1646](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1646)
- [rust/src/theta_v0/backtest/runner.rs:2688](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:2688)
- [rust/src/theta_v0/backtest/runner.rs:3008](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:3008)
- [rust/src/theta_v0/backtest/metrics.rs:677](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/metrics.rs:677)

**问题**

`bar_returns` 对逐 bar 权益直接求变化率，调用方把结果命名为 `daily_returns`；`metrics` 随后固定使用 `√252` 年化，并把 CVaR 标为“日收益尾部”。

**证据**

- runner 3008–3016 行没有任何按日期聚合。
- 两条主 runner 都直接把其结果传给 `metrics::compute`。
- `sharpe_sortino` 固定 `ANN=252`。
- 对 BTC 1 分钟 24/7 序列，正确 bar 年化根号与当前实现的比例约为：

```text
sqrt((365.25 × 24 × 60) / 252) = 45.6853
```

因此当前 Sharpe/Sortino 的量纲不是日频，也不是正确的分钟频；CVaR 同样是分钟 CVaR，却以日口径报告。

**最小修复建议**

利用 `Dataset.dates` 按 UTC/交易日取得每日最后权益，再计算日收益；年化因子按品种实际交易日设定。接口上应禁止把任意 bar return 传入名为 `daily_returns` 的参数。

---

## 4. P1：终点“强制平仓”只写入交易 PnL，没有把平仓费写入权益曲线

**file:line**

- [rust/src/theta_v0/backtest/runner.rs:1548](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1548)
- [rust/src/theta_v0/backtest/runner.rs:1566](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1566)
- [rust/src/theta_v0/backtest/runner.rs:2649](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:2649)
- [rust/src/theta_v0/backtest/runner.rs:2654](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:2654)

**问题**

末 bar 权益先按持仓 MtM 写入 `equity_curve`；随后计算含退出费的 `forced_pnl`，但没有修改 `cash/units`，也没有覆盖末端权益。

**证据**

初始 NAV=1000，多头 1 单位，entry=100，末价=110，单边费率 0.0003：

```text
末端 MtM 账本增量       = 9.970
真实终点平仓后账本增量 = 9.937
被遗漏的退出费         = 0.033
```

因此：

- `metrics.strat_return`、CAGR、MaxDD 使用未扣终点退出费的权益；
- 胜率、盈亏比等使用已经扣除退出费的 `trade_pnls_with_forced`；
- 同一报告内部两套 PnL 口径不守恒。

**最小修复建议**

在终点执行真实 `apply_fill` 强平，更新 cash、units、费用累计及末端 equity；如必须同时保留“未平 MtM”和“假设强平”两套口径，应分别返回两条完整权益曲线，不能只修改交易列表。

---

## 5. P1：现金不足导致 fill 被拒绝后，订单仍被计为已执行并更新声部状态

**file:line**

- [rust/src/theta_v0/backtest/runner.rs:1044](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1044)
- [rust/src/theta_v0/backtest/runner.rs:2574](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:2574)
- [rust/src/theta_v0/backtest/runner.rs:2985](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:2985)

**问题**

`apply_fill` 在开多现金不足时直接返回，且返回值不携带“是否成交/成交数量”。调用方仍无条件：

- `n_orders_executed += 1`；
- 更新 `voice_qty`；
- 登记 `held`；
- π 路径继续推进目标活动集。

**证据**

现金=100、价格=100、费率=0.0003、qty=1：

```text
开仓所需现金 = 100.03 > 100
```

`apply_fill` 不改变 cash/units，但上层会把它计为已执行订单。`is_l2 = n_orders > 0` 因而可能在零真实成交时为真。

默认 `base_units=NAV/px`、`gamma=1`、lot=1 时，满资本目标恰可产生这种费用后不可承受订单。

**最小修复建议**

让 fill 返回 `FillOutcome { requested_qty, executed_qty, rejected_qty, realized, fee }`；只有 `executed_qty>0` 才更新订单计数、持仓/声部/typed ledger 状态。资本上限同时应使用费用后的可承受数量。

---

## 6. P1：加仓/部分减仓没有进入 `TradeRecord`，随机对照与重算 PnL 错位

**file:line**

- [rust/src/theta_v0/backtest/runner.rs:2714](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:2714)
- [rust/src/theta_v0/backtest/runner.rs:2734](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:2734)
- [rust/src/theta_v0/strategy/coverage.rs:2648](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/coverage.rs:2648)
- [rust/src/theta_v0/backtest/metrics.rs:468](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/metrics.rs:468)

**问题**

`track_position_transition` 只在仓位符号改变或归零时生成 `TradeRecord`。同向 Add/Reduce 被视为“entry_bar 不变”，没有记录各批次成交价格或部分兑现。

但 `apply_fill` 会为每次部分减仓立即写入真实 `trade_pnls`。因此 `trade_pnls` 与供随机对照使用的 `trades` 不再表示同一交易集合。

**最小反例**

零费简化：

1. 100 买 10；
2. 110 减 5；
3. 120 平剩余 5。

真实 PnL：

```text
5 × (110−100) + 5 × (120−100) = 150
```

`TradeRecord` 只会记录最后一笔 `qty=5, entry=100, exit=120`，重算为 100，遗漏 50。加仓时还会把后加仓数量错误地视作全部在首次 entry_bar 建仓。

含默认费用的实算遗漏为 `49.685`。

**最小修复建议**

随机对照和显著性输入应来自逐 fill 的 lot ledger，至少记录每个开仓批次、部分平仓数量和实际成本基；不能仅靠“当前净仓符号+首次 entry_bar”重建交易。

---

## 7. P1：TW `Realize` 存在两套不一致舍入口径，且生产路径把负数向零截断

**file:line**

- [rust/src/theta_v0/backtest/treasury.rs:62](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/treasury.rs:62)
- [rust/src/theta_v0/backtest/treasury.rs:75](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/treasury.rs:75)
- [rust/src/theta_v0/backtest/runner.rs:1008](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1008)
- [rust/src/theta_v0/backtest/runner.rs:1110](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1110)

**问题**

- `treasury::settle` 对每笔 PnL 使用 `.floor() as i64`。
- runner 对累计 PnL 使用 `realized_cum as i64`。
- Rust 的浮点转整数是向零截断，不是注释声称的 `floor`。

**证据**

```text
PnL = -0.7:
treasury floor = -1
runner cast     = 0
```

此外，两笔 `+0.6`：

```text
settle_all 逐笔 floor = 0 + 0 = 0
runner 累计 floor     = floor(1.2) = 1
```

即便把 cast 修成 `floor`，逐笔舍入与累计舍入仍不一致。TW 的 `free`、阶段门和 `eta_bucket` 会因调用入口不同而出现不同状态。

**最小修复建议**

冻结唯一量化规则，并抽成共享状态ful量化器。建议累计精确 PnL并保留小数残差，只对累计值执行明确的 `floor`；`treasury::settle_all` 与 runner 必须复用同一实现。

---

## 8. P1：`capture_oos` 的训练 λ 使用了越过 `train_end` 的段内未来路径

**file:line**

- [rust/src/theta_v0/backtest/capture_oos.rs:154](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/capture_oos.rs:154)
- [rust/src/theta_v0/backtest/capture_oos.rs:159](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/capture_oos.rs:159)
- [rust/src/theta_v0/backtest/capture_oos.rs:170](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/capture_oos.rs:170)

**问题**

训练段筛选只检查 `segment.start_index` 的日期落在 train 窗内；`seg_gap` 却读取直到 `segment.end_index` 的完整路径。若段从训练末尾延伸到 test，测试期 high/low 会进入训练 p95 λ。

**证据**

```rust
d = day(segment.start_index);
d >= train_start && d <= train_end
```

没有检查 `segment.end_index` 或确认时间；随后直接读取：

```rust
bars[start_index..=end_index]
```

文件头第 21 行也承认训练窗尾存在后视，但该 λ 随后直接用于主 OOS 停损，并非“主判据不受影响”。

**最小修复建议**

在每个 train 切片上独立因果解析并只纳入截至 `train_end` 已确认、路径也完全位于 train 的段。至少同时要求段终点及确认 bar `<=train_end`。

---

## 9. P1：`capture_oos` 的持仓和 λ 跨 walk-forward 窗口延续，窗口没有独立结算/reset

**file:line**

- [rust/src/theta_v0/backtest/capture_oos.rs:185](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/capture_oos.rs:185)
- [rust/src/theta_v0/backtest/capture_oos.rs:192](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/capture_oos.rs:192)
- [rust/src/theta_v0/backtest/capture_oos.rs:250](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/capture_oos.rs:250)
- [rust/src/theta_v0/backtest/capture_oos.rs:289](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/capture_oos.rs:289)

**问题**

所有 walk-forward 窗共用一个全历史 `ParseLayerIncr`、一个 `open` 持仓和一个主循环。持仓只在入场时冻结所属窗口及 λ，没有在 `test_end` 结算或切换。

**证据**

相邻预注册 test 窗按日期连续。若交易在 Wk 尾部入场、Wk+1 才退出：

- 整个 Wk+1 期间仍使用 Wk 的训练 λ；
- 出场价格来自 Wk+1；
- 聚合时仍按 entry 上保存的 `win=Wk` 计入 Wk；
- Wk+1 的独立测试结果没有反映该时段已占用的持仓状态。

这既不是独立窗口回测，也不是明确声明的连续组合回测。

**最小修复建议**

每个窗口单独执行：train 用于结构暖启动/定参，test 开始时重置资金与持仓，test_end 强平或显式 censored。若要连续组合回放，应改成单一连续策略并明确 λ 的切换及跨窗仓位归属，不能再报告“逐窗独立”。

---

## 10. P1：`capture_oos` 在不可交易 bar 成交，止损跳空固定按 stop，OOS PnL 系统性偏乐观

**file:line**

- [rust/src/theta_v0/backtest/capture_oos.rs:201](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/capture_oos.rs:201)
- [rust/src/theta_v0/backtest/capture_oos.rs:212](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/capture_oos.rs:212)
- [rust/src/theta_v0/backtest/capture_oos.rs:237](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/capture_oos.rs:237)
- [rust/src/theta_v0/backtest/capture_oos.rs:250](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/capture_oos.rs:250)

**问题**

- 止损只检查 high/low，成交价永远设为 `stop_px`，不处理跳空。
- 入场和普通出场直接使用当前 close。
- `bar.untradable` 只计数，不阻止成交。

这与仓库统一执行规则“不可交易不成交、跳空按 open”冲突。

**证据**

代码明确执行：

```rust
exit: stop_px
entry: bar.close
exit: bar.close
```

同时把 `bar.untradable` 加入计数，但仍写入交易。文件头的“诚实声明”不能消除资金核算错误；该结果仍被作为 OOS 捕获率和净 `g_n` 输出。

**最小修复建议**

复用 `exec::stop_fill_price`、`fill_bar_index` 和统一费用逻辑。不可交易 bar 应顺延至下一可交易成交点；若无法成交则 censored，而不是用不可交易 close 记账。

---

## 11. P2：`wverify_run` 的日期运算把“前一天/前推六个月”错误钳到 28 日

**file:line**

- [rust/src/theta_v0/backtest/wverify_run.rs:908](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/wverify_run.rs:908)
- [rust/src/theta_v0/backtest/wverify_run.rs:917](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/wverify_run.rs:917)
- [rust/src/theta_v0/backtest/wverify_run.rs:1519](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/wverify_run.rs:1519)

**问题**

`q4_prev_day` 遇每月 1 日统一返回上月 28 日；`q4_shift_back_6m` 对任何日号大于 28 都无条件钳到 28，即使目标月有 29/30/31 日。

**证据**

当前测试甚至冻结了错误结果：

```text
q4_prev_day("2023-01-01") == "2022-12-28"
```

正确结果应为 `2022-12-31`。这会无故漏掉 2–3 天训练数据；六个月起点也可能被向前扩展 1–3 天。

**最小修复建议**

使用真实历法日期运算；若不引入日期库，至少实现闰年和每月天数表，并增加月末、闰年、跨年测试。

---

## 未发现高置信问题的部分

- `segment_gn.rs` 的 `(H−L)/(H+L) > fee_rate` 与双腿费用公式代数一致。
- `MuSwing::net_realized` 的多空归一后符号与双边费用扣除本身正确。
- `wverify_run` 的 Welford 在线均值/方差及 dump bit round-trip 未发现确定性偏差。
- 当前数据全部声明为 UTC 或按 UTC 解释；没有足够证据把非零时区未换算列为当前数据集上的实发 bug。


