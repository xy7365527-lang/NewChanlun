# #649 FeeQuoter 移植留痕报告（#646 裁定 a / ⚠MED-6 + ⚠LOW-8）

- 日期：2026-08-18
- 分支：`sandcastle/issue-649`（off main）
- 票据：#649（#646 裁定 a：移植 #419 逐科目 fee quote 进 main fill.rs）
- 参照面：kimi 封存 tip `797c9ad35c`（只读，经 git 读取）
- 红线：fee 是成本记账，不改开仓/平仓决策语义。

---

## §1 接线位点清单（main fill.rs 与 overlay_state.rs）

### 1.1 `strategy/overlay_state.rs`（#419 核心，⚠MED-6 恢复）

`apply_open`/`apply_close`/`settle_forced_virtual`（及内部 `flatten`）签名
`fee_rate: f64 → fees: &FeeQuoter`，费率先在成交点按 (σ_v, q_v, px, 方向) 逐声部解析：

| 符号 | 行 | 说明 |
|---|---|---|
| `open_fee_rate` | :62 | 开仓侧：开多=买入、开空=卖出（Flat 不开仓 ⟹ fallback 占位） |
| `close_fee_rate` | :72 | 平仓侧：平多=卖出、平空=买回（与开仓方向相反） |
| `quote` | :81 | 统一走 `FeeQuoter::production_rate_or_fallback`（无角色参数，角色单一来源） |
| `apply_open` | :553 | 签名 `fees: &FeeQuoter`，`fee_rate = open_fee_rate(...)` |
| `apply_close` | :628 | 签名 `fees: &FeeQuoter`，透传 `flatten` |
| `flatten` | :651 | 签名 `fees: &FeeQuoter`，`fee_rate = close_fee_rate(...)` |
| `settle_forced_virtual` | :702 | 签名 `fees: &FeeQuoter`，**逐声部**解析（per-share 档 q_v 不同 ⟹ 不可共用标量） |

### 1.2 `backtest/fill.rs`（消费面，⚠MED-6 三处调用点类型不匹配的修复）

| 行 | 位点 |
|---|---|
| :4719 | `let fees = super::treasury::fee_quoter(&config.exec);`（FeeQuoter 单源构造） |
| :4990 | `vb.apply_open(vo.voice, vo.side, vo.qty, px, i, &fees)` |
| :4991 | `vb.apply_close(vo.voice, px, i, &fees)` |
| :6309 | `vb.settle_forced_virtual(last_px, exit_bar, &fees)` |

净额账户路径（`apply_order` / 窗口终点净额强平）**仍用标量 `fee_rate`**——其 fee_quoter
接线属另票（treasury.rs `scalar_cost_rate` 文档登记的「接 (qty, px, side) 缝改用 fee_quoter
逐笔解析（另票）」同款缝），本票不越权扩面（红线：净额臂是默认生产路径，改动其费口径需
单独对拍声明）。

### 1.3 ⚠LOW-8 三处 fail-loud 恢复（+ `RunResult.fee_rate` 类型承载）

`RunResult.fee_rate` 恢复为 `Option<f64>`（runner.rs:143），取值经
`treasury::scalar_cost_rate_opt`（runner.rs:394 / :605）按档位形态三分叉；三处 l3 消费面 +
一处 m8 消费面恢复 `.expect(SCALAR_COST_RATE_UNDEFINED)`：

| 文件 | 行 | 恢复点 |
|---|---|---|
| `l3_fullwindow.rs` | :158 | `res.fee_rate.expect(SCALAR_COST_RATE_UNDEFINED)` |
| `l3_pi_falsify.rs` | :180 | 同上 |
| `l3_delta_r_alpha.rs` | :982 | `rebuild_cost_series` 内 `expect`（鞅守卫成本剥离） |
| `wverify_run.rs` | :1801 | m8 层4 `significance` 输入 |

---

## §2 对拍表（资金路径硬门：旧 f64 费率 vs FeeQuoter 逐科目）

口径：旧 = `fee_rate = (commission+slippage+tax)bps/1e4`，`ExecConfig::default()` 下恒
`3e-4`（1+2+0 bp/side，单边常率，与 qty/px/side 无关）。新 = `FeeQuoter`：
- 未标定档（`fee_schedule=None`）⟹ `production_rate_or_fallback` 恒返回 `3e-4`，
  **逐位同值**（`assert_eq!` 位相等，venue_fee.rs `quoter_none_is_bit_exact_fallback` 全枚举
  锁定）⟹ 本档新旧**零差异**；
- 标定档（`fee_schedule=Some`）⟹ datum 费率 + 未标定滑点 addon（`slippage_bps/1e4`）。

| 成交例 | 旧 f64 费率 | FeeQuoter 费率 | 旧总费 | 新总费 | 差异解释 |
|---|---:|---:|---:|---:|---|
| BTC 1.5@$50000（notional 档，VIP0 taker） | 3.0000e-4 | 1.2000e-3 | 22.500000 | 90.000000 | datum 10bp/side 取代 3bp 常数，+2bp 滑点 |
| OKLO 100@$20 买（per-share） | 3.0000e-4 | 3.8528e-4 | 0.600000 | 0.770559 | 佣金 0.35 + pass-thru + 清算/CAT 0.0203，摊回名义 + 2bp 滑点 |
| OKLO 100@$20 卖（per-share，含监管费） | 3.0000e-4 | 4.1563e-4 | 0.600000 | 0.831259 | 买侧基础 + 卖出 SEC 0.0412 + TAF 0.0195 |
| OKLO 10@$20 买（最低佣金主导） | 3.0000e-4 | 1.9614e-3 | 0.060000 | 0.392289 | 最低佣金 $0.35 托底在小单摊薄成 ~17.6bp（旧常率结构性无法表达） |
| OKLO 1@$20 买（1% 上限压最低佣金） | 3.0000e-4 | 1.0218e-2 | 0.006000 | 0.204351 | 1% 名义额上限（$0.20）压过最低佣金 $0.35 |

**逐条解释口径**：
1. 未标定档零差异是**结构性保证**（`FeeQuoter::None` 分支不读档、不读 (qty,px,side)），
   不是碰巧相等——venue_fee.rs `quoter_none_is_bit_exact_fallback` 在 qty∈{1e-8…1e9}×
   px∈{1e-6…1e7}×side∈{Buy,Sell} 上 `assert_eq!`。
2. notional 档（BTC）差异 = datum 档 bps 与未标定 3bp 常数不同（10bp/7.5bp vs 3bp）——
   这是 #360 标定的**目的**，不是回归。
3. per-share 档（OKLO）差异 = 逐科目（佣金/pass-thru/清算+CAT/卖出 SEC/TAF）摊回名义的
   等效费率 + 滑点 addon。最低佣金/1% 上限在小单上的主导效应（报告 §3.1 明文）只有逐笔
   解析才保留，旧常率结构性无法表达。
4. 生产成交路径当前**无 fee_schedule 注入通道**（`data.rs` 只校验 datum 与 dataset 的
   symbol 匹配，fill loop 此前不消费 `fee_schedule`）⟹ 默认跑批恒在未标定档 ⟹ 本迁移对
   现有生产读数**逐位不变**；标定档差异仅在显式注入 `exec.fee_schedule` 后生效。

**零静默默认值（费率未定义一律 fail-loud）**：
- 未入簿品种/档位 ⟹ `VenueFeeBook::resolve` 返回 `Err`（venue_fee.rs，`unlisted_symbol_fails_loud` 锁定）；
- 单标量成本口径无良定义（per-share / per-notional 非对称）⟹ `scalar_cost_rate_opt` 返
  `None`，消费面 `.expect(SCALAR_COST_RATE_UNDEFINED)` panic（§1.3 四处在案）；
- 标定档 + 非零 `tax_bps` ⟹ `fee_quoter` 内 `assert!`（禁与 datum 税费科目双计）；
- 非法量/价（qty≤0 或 px≤0）⟹ 该 fill 是 noop/拒单，返回 fallback 占位**不进任何算术**
  （overlay_state.rs 的 Flat 分支与 `production_rate_or_fallback` 的非法量价分支同此口径，
  非「给真实成交静默喂默认费率」）。

---

## §3 fail-loud 恢复点汇总（⚠LOW-8）

见 §1.3。恢复语义 = kimi `797c9ad35c` 的 `res.fee_rate.expect(SCALAR_COST_RATE_UNDEFINED)`
原口径：类型承载 `Option<f64>`，位点从构造期移到消费期（#388 T2），防线等价不减弱。
三处 l3 消费面在现有调用面上恒在未标定档（`ThetaConfig::default()`，无 datum 注入通道），
`expect` 是「未来接 datum 注入则 fail-loud」的防线；m8 `wverify_run` 消费面同款。

---

## §4 验证

| 命令 | 结果 |
|---|---|
| `cargo test --lib` | **2756 passed / 0 failed / 154 ignored**（零新增红；基线 2444/0/138 系票面 2026-07-29 快照，现 main 用例数已增长） |
| `cargo check --all-targets` | **0 错**（exit 0） |
| `cargo check --all-targets --features backtest_bin` | **0 错**（exit 0，CI 第二闸） |
| `cargo fmt --all -- --check` | **0 差** |
| `theta_v0::strategy::overlay_state` 10 测 | 全绿（含 4 个 VoiceExecBook 守恒/强平/拒开/平仓 noop + 新增 `voice_exec_calibrated_per_share_charges_datum_fee` 标定档账本层对拍） |
| `theta_v0::venue_fee` 28 测 | 全绿（FeeQuoter 新接口单测已在 main，本票零改动） |
| `theta_v0::backtest::treasury` 17 测 | 全绿（scalar_cost_rate_opt 三分叉 + fee_quoter） |
| `theta_v0::backtest::l3_*` 11 测 | 全绿（18 条 `#[ignore]` 重型 O(n²)/需数据用例维持原状） |

环境注：沙盒内 `cargo test` 需 `PYO3_PYTHON=/home/agent/.local/bin/python3.11` +
`LD_LIBRARY_PATH`/`LIBRARY_PATH` 指向 uv cpython 的 lib 目录（系统 python3.11 无
`libpython3.11.so`，链接 `-lpython3.11` 失败；此为本沙盒环境问题，非本票改动）。

---

## §5 红线合规声明

- fee 是成本记账：本票只改**费率解析入口**（f64 常数 → FeeQuoter 逐笔解析），**未改**
  `apply_fill`/`apply_open`/`flatten` 的现金流与含费成本基公式（仍为 `px·(1±δ·fee_rate)` 同形）；
- 未改开仓/平仓决策语义：现金约束（开多需 `cash ≥ cost`）、拒开/平仓 noop 判据、order 生成
  路径均未动。未标定档下 `FeeQuoter` 逐位等于旧 f64 ⟹ 现有生产路径（默认无 fee_schedule
  注入）**逐字节不变**；
- 交易行为面变更：无。唯一行为变化面 = 显式注入 `exec.fee_schedule=Some` 后的费口径
  （datum 费率 + 滑点），这是 #360/#419 标定的既定语义，非本票新引入，且已在对拍表逐条
  登记差异口径。
