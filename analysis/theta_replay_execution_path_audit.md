# theta_replay 执行路径审计：回放是否触发完整生产链（#1318）

> 票号：[#1318](https://github.com/xy7365527-lang/NewChanlun/issues/1318)
> 交付物：逐环节审计表 + 回放路径运行时断言（补锁）+ 处置说明。
> 结论性质：代码路径事实盘点（推论层；是否「补上生产链」的裁定不在本票，本票按 issue 第 3 条
> 的「或如实报告」分支交底，见「缺失环节处置」节）。

---

## 1. 结论

**theta_replay 只驱动 theta_v0 的「生产 π 决策核」（`ThetaPiStream::push_bar`，stream.rs），
它没有、也不可能触发 issue 所列的「41课门 / fatigue门 / RiskGate / TwLedger」——这些环节
分属两个不同的外围，二者都不在 `ThetaPiStream` 的执行路径上：**

1. **41课门 / fatigue门 属于另一台引擎**（organic 赋格 v2 交易层 `run_organic`，
   `rust/src/trading/runner.rs`），与 theta_v0 零生产引用（`trading/mod.rs` 明写
   「theta_v0（π，唯一现役引擎）对本目录生产引用 = 0」）。theta_replay 走的是 theta_v0，
   **结构性不可达**，不是「回放绕过了」。
2. **RiskGate（`k_theta_risk_gate` 实判）/ TwLedger（`TwLedgerThread`）/ nest 门 / χ 门 /
   entry_stop_recheck / typed ledger / 中枢震荡** 是**批量回测 fill loop**
   （`pi_theta_fill_loop_overlay`，fill.rs）的外围机械。`ThetaPiStream` 是**实盘 π 回路**
   （PyO3 `ffi::PyThetaStream` 与 nautilus `ThetaPiStrategy` 的内核，stream.rs 模块头自陈），
   它的设计边界就是「决策核 + per-leg 账本」两样 D1 出口，不含上述批量外围；stream.rs 模块头
   与 #1306 对拍 harness（theta_pi_diff.rs）均把这套差异登记为「共核不共外围」的预期差。

**因此**：M1 实验（#1317）若以 theta_replay 读数归因，其归因面 = **theta_v0 生产 π 决策核**
（三类买卖点判据 + 多级别联立 + PersistentRegistry + per-leg 账本），**不覆盖** RiskGate/
TwLedger（批量外围）与 41课门/fatigue门（另一引擎）。这是本票要如实交底的事实。

本票落地：对回放路径**实际在册**的门控/账本写入补了运行时断言（第 6 节），使「回放绕过了
哪个环节」从注释声明变成运行时可判；不在册的环节按第 5 节列原因，不硬凑、不改实盘 π 回路。

---

## 2. theta_replay 实际执行路径

```
theta_replay.rs:main
  → load_by_symbol (theta_replay.rs:79)            # 数据 → Dataset
  → run_replay_double (theta_replay.rs:108)        # replay_dump.rs:272
      → replay() × 2 (replay_dump.rs:143)          # 同输入双跑逐位自检
          → ThetaPiStream::new (replay_dump.rs:145)
          → 逐 bar: push_bar(bar, p_t, nav) (replay_dump.rs:150, stream.rs:117)
              ├─ classifier.append_bar (stream.rs:120)          # 增量分类塔（含三类买卖点）
              ├─ newly_confirmed_step (stream.rs:133)           # 确认-bar 部署 diff
              ├─ KThetaRiskGate::open() (stream.rs:137)         # 风控门【开闸，不实判】
              ├─ pi_theta_step_traced_with_risk_seeds (stream.rs:165)
              │    ├─ 无 TW 源 (None, stream.rs:178)            # P2/P3/P4 阶段门跳过
              │    └─ 无 risk-close seeds (&[], stream.rs:175)  # 无强平/止损种子
              ├─ overlay.step (stream.rs:183)                   # 逐声部账本写入
              └─ level_ledger.step + observe_lee_net (stream.rs:184,186)  # 级别账本写入
          → stream.finish() (replay_dump.rs:188)                # 窗口终点强平
```

---

## 3. 逐环节审计表

行号以本票提交时的 main 基线为准。表分两块：**theta_v0 π 链**（theta_replay 所在引擎）与
**organic 赋格交易层**（issue 引用的「中枢账本 ingest → fatigue observe → 门控分派 →
allocator → 账本」推进序所属的引擎）。

### 3.1 theta_v0 π 链（theta_replay 实际驱动）

| 生产链环节 | 回放中有/无 | 证据（代码行号） |
|---|---|---|
| 增量分类塔驱动（append_bar） | **有** | stream.rs:120 |
| 三类买卖点判据（BspBits buy/sell 1/2/3，newly_confirmed_step） | **有** | stream.rs:133、:298-330；types.rs:201-207 |
| 多级别联立（pi_theta_step_traced_with_risk_seeds 跨级） | **有** | stream.rs:165 |
| PositionRegistry（PersistentRegistry 跨 bar 持久注册表） | **有** | stream.rs:176（参构造）；模块头 stream.rs:52-55 |
| RiskGate（k_theta_risk_gate 实判） | **无** | 回放侧 stream.rs:137 `KThetaRiskGate::open()`；批量侧实判在 fill.rs:5187 |
| TwLedger（TwLedgerThread/TwState，P2/P3/P4 阶段门） | **无** | 回放侧 stream.rs:178 `None` TW 源；批量侧 fill.rs:5004 |
| nest 门（N^δ/Xzd 证书门） | **无** | 批量侧 fill.rs:5339；回放侧无此段 |
| χ_t 阈值门 | **无** | 批量侧 fill.rs:5303；回放侧 stream.rs:161 `step_gamma_trade = step_gamma.clone()`（χ≡1） |
| entry_stop_recheck 入场复检门 | **无** | 批量侧 fill.rs:5424；回放侧无此段 |
| 中枢账本/中枢震荡（cl_machines + CenterOscillationBook） | **无** | 批量侧 fill.rs:5548；回放侧无此段 |
| 逐声部账本写入（OverlayState::step） | **有** | stream.rs:183 |
| 级别账本写入（LevelLedgerMirror::step + LEE-Net 见证） | **有** | stream.rs:184、:186 |
| typed ledger（TypedTrade） | **无** | 批量侧 fill.rs:5860 等；回放侧无此段 |

### 3.2 organic 赋格交易层（run_organic，issue 所引推进序的引擎）

| 生产链环节 | 回放中有/无 | 证据（代码行号） |
|---|---|---|
| 中枢账本 ingest（CenterBook::ingest） | **无** | trading/runner.rs:806 |
| fatigue observe（FatigueGate） | **无** | trading/runner.rs:827-840 |
| 41课门（TrendExhaustion） | **无** | trading/runner.rs:761-790 |
| 门控分派（FLAT/ARMED/LONG 状态机） | **无** | trading/runner.rs:847 |
| allocator（SizeAllocator） | **无** | trading/runner.rs:722（构造）、:146（open_position） |
| 账本（OrganicLedger） | **无** | trading/runner.rs:158 |

`run_organic` 主循环推进序见 trading/runner.rs:4-7（「中枢账本 ingest → fatigue observe →
FLAT/ARMED/LONG 分派 → (LONG) allocator → master 循环 → voice main 腿 → VoiceUnit.step →
eod_close」）。**这是与 theta_v0 完全独立的消费层**：theta_v0 对本目录生产引用 = 0
（trading/mod.rs 头部 GUARD-ROLE 块），theta_replay 不可能触达。

---

## 4. 「回放是否绕过生产链」的判定

- **对 theta_v0 实盘 π 回路**：回放**没有绕过**——`ThetaPiStream` 就是实盘 π 回路本身
  （PyO3 `ffi::PyThetaStream` / nautilus `ThetaPiStrategy` 的内核），回放走的是同一份决策核，
  实盘也不含 RiskGate/TwLedger/nest/χ/typed（它们在批量回测 fill loop 里）。回放忠实复刻实盘
  π 路径，差异面已在 #1306 对拍 harness 登记（theta_pi_diff.rs「共核不共外围」）。
- **对批量回测 fill loop**：回放**确实不含** RiskGate/TwLedger/nest/χ/entry_stop/typed/
  中枢震荡——它们是批量侧外围。若某实验需要「全外围生产链」，应当跑批量侧
  `run_theta_v0_pi_overlay`（对拍 harness 的 `run_batch` 已封装），而非把外围塞进 `ThetaPiStream`
  （那会改实盘 π 回路、破坏 stream.rs 冻结的 bit-exact 契约，属范围外）。
- **对 41课门/fatigue门**：不同引擎，回放结构性不可达（见 3.2）。

---

## 5. 缺失环节处置（issue 第 3 条）

| 缺失环节 | 处置 | 原因 |
|---|---|---|
| 41课门 / fatigue门 | **如实报告：不可用** | 属 organic 赋格交易层（run_organic），非 theta_v0；theta_v0 生产引用 = 0。补入需跨引擎重架构，不属本票。 |
| RiskGate（k_theta_risk_gate 实判） | **如实报告：不可用** | 实判需 typed ledger（open_trades 止损冻结）+ ChongBook（保证金基数）+ margin 注入；`ThetaPiStream` 是无状态决策核（不维护影子仓/不接成交回执，stream.rs「状态裁定」），无这些输入。补入 = 改实盘 π 回路，范围外。 |
| TwLedger（P2/P3/P4 阶段门） | **如实报告：不可用** | TwLedgerThread 需真实 fill 的 realized PnL 入账；`ThetaPiStream` 无 fill 模拟（theta_replay.rs 模块头自陈「不接 fill 模拟」）。无 fill ⟹ 无 realized ⟹ TW 阶段门无数据源。 |
| nest 门 / χ 门 / entry_stop_recheck / 中枢震荡 / typed ledger | **如实报告：不可用** | 同上：均为批量 fill loop 外围，依赖 fill/typed/margin 等流式侧不携带的状态。 |

**不采取的方案**：把上述外围塞进 `ThetaPiStream`。理由——stream.rs 模块头已把「不携带」这些
机械写死为设计边界（bit-exact 契约），且该模块是实盘 PyO3 内核；改它 = 改实盘行为，超出
「回放路径审计」票面，且违反 #799/#804 同判断单一实现纪律（会在流式/批量两条路径上再造
第二份门控实现）。

---

## 6. 补锁（issue 第 4 条：运行时断言，非注释）

回放路径**在册**的四个门控/账本写入环节，在 `replay_dump.rs::replay()` 内加了 `assert!`
（非 `debug_assert!`，`--release` 跑批同样执行）：

| 断言 | 锁住什么 | 位置 |
|---|---|---|
| `order.qty == \|round(p_star − p_t)\|`（逐**决策** bar） | Schedule_Θ 单出口契约：分派/allocator 产出与决策一致（不可交易/close≤0 bar 决策层早退、`last_order` 是上一决策 bar 的残留 ⟹ 跳过本锁） | replay_dump.rs:157-163 |
| `stream.bar_count() == dataset.bars.len()` | 决策核（分类塔→买卖点→π step）逐 bar 驱动，不漏 bar | replay_dump.rs:179-185 |
| `lee_net_witness().max_abs_residual == 0` | 级别账本写入与 overlay 净敞口的 LEE-Net 恒等（整数，精确） | replay_dump.rs:215-219 |
| `reconcile_residual_sorted < 1e-6 × scale` | 逐声部账本写入与账户侧价格 PnL 对账守恒（相对容差，量级随名义放大） | replay_dump.rs:220-231 |

测试锁：`replay_dump::tests::replay_double_exercises_runtime_locks_on_synthetic_data`
（replay_dump.rs:372-389）在合成锯齿数据上直接跑 `run_replay_double`（即 theta_replay 的同一份
回放核心），四道锁任一破会在此先 panic；同输入双跑逐位一致照旧锁定。另
`replay_dump::tests::replay_handles_untradable_bars_without_breaking_locks`（replay_dump.rs:399-423）
把不可交易 bar 掺进合成数据，锁混合可交易性数据的回放不 panic、bar 数/可交易数不漏（决策层早退
路径的驱动完整性）。

---

## 7. 验证

- `cargo check --features backtest_bin --bin theta_replay` 通过（162 条既有 warning 基线，无新增）。
- `cargo test --features backtest_bin replay_dump::` ：新增测试 2 个通过。
- `cargo test --features backtest_bin theta_pi_diff::` ：3 通过 / 1 ignored（既有 `#[ignore]`）。
- `cargo test --features backtest_bin --test theta_pi_stream` ：3 通过。
