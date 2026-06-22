# recursive_t 8 标的 × 3 模式 L3 验证报告

**日期**：2026-06-22
**入口**：`rust/src/recursive_t/rec_stream.rs::rec_btc()`（`#[ignore]`，复用 `backtest_run::SYMBOLS` + `RecStream`）
**跑法**：`BT_SYMBOLS=… cargo test --release recursive_t::rec_stream::tests::rec_btc -- --exact --ignored --nocapture`
**认识论标注**：prove 守卫零 panic = **L3**（8 标的 ~1500 万 bar 交叉验证）；每标的收益 = L2；跨标的 regime 规律 = L3 观测。

---

## 1. 结论

8 标的 × 3 模式（24 组）全量跑通（两批 EXIT 0，796s）。**核心成果：4 个 panic 守卫（`prove_sink_descends`/`prove_sigma_quota`/`prove_relabel_invariant`/`prove_bsp_triggers_operation`）在全 8 标的零 panic、`no_trigger=0`——prove 守卫移植验收从 L2（BTC 单标的）提升到 L3。** `prove_guards_migration.md` 第 88 行边界条件第 3 条（「若未来数据使某 panic 守卫在其他标的 fire → 降级为观测」）的否证测试**通过**——未 fire，L0 不变量有效域覆盖全 8 标的。

## 2. 引擎收益 % vs Buy-Hold（`*`=超 BH）

| 标的 | Structural | AND | OR | BH | 备注 |
|------|-----------:|----:|---:|---:|------|
| CL | +22.2% | +19.0% | **+35.6%** `*` | +28.2% | OR 超 BH（震荡 regime alpha） |
| BRN | −9.8% | +3.9% | −26.7% | +87.4% | 全 <BH |
| DX | −1.0% | −1.0% | −1.1% | +4.1% | 全 <BH（平 regime） |
| GC | −28.3% | −29.1% | −17.5% | +257.3% | 全 <BH（最弱） |
| ES | −8.3% | +7.1% | +5.3% | +594.3% | 强牛踏空 |
| QQQ | −0.7% | −1.4% | +2.0% | +174.6% | 全 <BH |
| OKLO | +173.8% | +186.8% | −6.6% | +307.1% | S/A 正但 <BH；OR 转负 |
| BTC | +37.3% | −3.0% | +25.0% | +1380.4% | 强牛踏空（确认滞后税 + 僵尸核心死锁，见 546 号） |

**超 BH 仅 1 组（CL/OR）**。7/8 标的全模式 <BH = 已知基线性质（确认滞后税 + 强牛踏空，非 bug）。注：超长上行数据上「无 alpha」结论信息增量低（[[project_backtest_benchmark_falsifiability]]）。

## 3. per-level 短差 P&L 规律（L0..L5）

- **L1 几乎恒为正且常是最大赚层**（8 标的全部 L1≥0 除 BRN/Or）。
- **L0 符号随 regime 翻转**（强牛 BTC/And、GC 为负；震荡 CL 为正）。
- **L4/L5 全标的全为 0**（深层级别在 1min 数据 BSP 罕见）。
- 异常：OKLO/Structural L2=−18042 / L3=+33823（次新股极端走势使深层罕见承载大额短差）。
- BTC/And vs Structural 断崖：+37.3%→−3.0%，flip 2→5、ascend 6→11、强平 2→10，L0 +8588→−22679（AND 触发更多翻转→做空腿失血放大，[[project_mismatch_spectator_not_mediator]] r=+0.735）。

## 4. 做空腿 / dir_mismatch / 强平（观测）

- **做空腿**：CL 全正；BTC/GC/ES/QQQ/OKLO/DX 普遍负——印证 [[project_t_short_leg_regime_function]]（空头腿=亏损来源，有效域⊂非上行 regime）。
- **dir_mismatch**（观测非 panic）：强牛高（BTC 24~40%、GC 73~82%），平/震荡低（QQQ 1.3%、CL 19%）。`prove_epsilon_symmetry` 已降级为 `check_direction` 观测。
- **强平**：集中强牛（BTC/And=10）+ 金属（GC 7~13）；CL/DX/QQQ 全 0 强平。
- **sink≈2×recover 失衡普遍**（观测守卫 `prove_sink_recover_balance`/`prove_per_level_pnl`，BTC 已知违反故非 panic）。

## 5. 结果包六要素

1. **结论**：8 标的 × 3 模式现状读数 + prove 守卫 8 标的零 panic（L2→L3 成立）。
2. **定义依据**：`rec_stream.rs::rec_btc` 入口 + `backtest_run.rs:122` SYMBOLS 表（8 数据文件确认存在）+ `prove_guards.rs` 4 panic 守卫每 bar 热路径断言。
3. **边界条件**：若未来某标的某 bar 触发 panic 守卫 → 有效域 ⊊ 定义域的重大发现，须按 `count_chiral_violations` 先例从 panic 降级为观测（本次 8 标的未发生）。
4. **下游推论**：必然性累积在 recursive_t 经 8 标的 L3 加固；后续引擎改动违反这些 L0 不变量会在热路径 panic 暴露。7/8 标的强牛踏空给 546 号僵尸核心死锁的普适性提供 L3 间接证据。
5. **谱系引用**：[[project_t_short_leg_regime_function]]、[[project_mismatch_spectator_not_mediator]]、[[project_t_engine_btc_baseline]]、`docs/prove_guards_migration.md`；546 号（僵尸核心死锁）。
6. **影响声明**：纯观测验证，未改任何引擎代码（既有 `#[ignore]` 测试，接入点只读，bit-exact 保持）。
