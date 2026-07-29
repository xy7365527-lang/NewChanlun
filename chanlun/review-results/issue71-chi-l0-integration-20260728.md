# #71「L0 χ 接入」实装与验证报告

- 日期：2026-07-28
- 工位：`/private/tmp/kimi-nest-mainline`
- 基线 HEAD：`b615bd7143`（`merge: ticket-69 合流——#69 5a/5b 线形化入 kimi-nest-mainline`）
- 裁定口径：#65 方案③，`X_γ = (δ(P_exit-P_entry)-fee)/(qty·P_entry)`
- 验收状态：**Slice 1、Slice 2、三腿 bit-exact、四臂实跑与全量编译/测试均已执行；§5-C 双向交叉对账未全通过，故按设计稿纪律，本票当前不能声明验收完成。**

## 0. 环境与测试指纹

合并后重核：

- `git log --oneline -3` 首行 = `b615bd7143`，含 ticket-69 合流。
- `grep -c chi_dimension_three_return rust/src/theta_v0/backtest/l3_delta_r_alpha.rs` = `3`。
- 未执行任何 git 写操作；未还原共享工作面的既有改动。

| 闸门 | 基线 | 收尾 | 结论 |
|---|---:|---:|---|
| `cargo test` passed | 2015 | 2019 | +4，均为本票 GammaDump 常规测试 |
| `cargo test` failed | 1 | 1 | 失败集未扩大 |
| `cargo test` ignored | 135 | 136 | +1，本票真实数据验证 harness |
| `cargo build --all-targets` | — | GREEN，13.13s | 全 targets 可编译 |
| `git diff --check` | — | GREEN | 无 whitespace error |

基线与收尾唯一失败均为：

`theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`

- left = `16618955402698307653`
- right = `10432481772907336594`
- 该失败为 #115 在案；本票失败集没有新增 #446/#491 或其他红。

全仓 `cargo fmt --all -- --check` 仍被约 8.8 万行既有格式漂移打红，命中大量本票外文件；未运行会污染共享工作面的全仓自动格式化。新增独立文件 `gamma_dump.rs` 已单文件 `rustfmt`。

## 1. 切片 diff 摘要

### Slice 1：量纲③

1. `l3_delta_r_alpha.rs`
   - 新增 `chi_dimension_three_return`。
   - `build_mu_from_bars` 的 χ 生产喂入改为费扣绝对额除以 `entry_px`（该处 `qty=1`）。
   - 注释从“绝对额/单位边际”订正为 #65 持仓期相对收益。
2. `pi_bsp_timing.rs`
   - 两个 χ 生产喂入点均改为除以 `qty·entry_px`。
   - 诊断表头从 `μ(z)绝对` 订正为 `μ(z)相对`。
3. `ledger.rs`
   - 只订正 estimand 声明；`units` 仍不进 μ。
4. `marginal_return`、`mu_lcb`、`mu_shrink`、`pnl_raw_unlevered` 均未改。

`rg "\.observe\(MuObservation" rust/src` 的生产命中只有：

- `pi_bsp_timing.rs` 两处；
- `l3_delta_r_alpha.rs` 一处。

其余命中均位于测试/合成数据，输入值是测试所声明的 μ 样本，量纲切换不要求改写。

### Slice 1 冻结测试重锚逐处清单

| 测试 | 动作 | 结果 |
|---|---|---|
| `chi_feed_uses_entry_notional_relative_return` | 新增；显式锚定 `marginal_return/entry_px` | GREEN |
| `chi_feed_dimension_three_is_qty_immune` | 新增；显式锚定逐笔除以 `qty·entry_px` 后 qty 免疫 | GREEN（需 `--features backtest_bin`） |

没有既有 μ/LCB 数值冻结断言因本次切换而打红，故**既有测试重锚数 = 0**；没有静默改写旧期望值。新增两处测试注释均写明“量纲③重锚（#65 裁定 2026-07-21）”。

### Slice 2：GammaDump

- 新文件 `rust/src/theta_v0/backtest/gamma_dump.rs`。
- 独立 env：`OPSEM_GAMMA_DUMP_DIR`；未设/空串均关闭。
- 独立产物：`<dir>/gamma_candidates.jsonl`。
- `#[cfg(test)]` 注入为线程局部 override；测试目录使用 pid + nonce。
- 只读消费 `step_gamma`、生产 χ 后成员、`step_trace.opened`；不调用 `est.observe`。
- `chi_admit` 来自 `gamma_index ∈ step_gamma_trade`，不重跑 χ 门。
- `admission_value` 只读复用 `z_of_candidate` + `mu_lcb/mu_shrink`；缺席为 null。
- dump 字段不进入 `entry_z`、`MuClass`、μ 桶键、`J_Θ` 或 χ。
- 未改禁碰文件 `rust/src/theta_v0/mod.rs`；模块只接在 `backtest/mod.rs`。

钩子位于 ticket-69 拆分后的 `fill.rs::pi_theta_fill_loop_overlay`：`step_gamma`、χ 成员与 `step_trace` 首次同时在手之后、PanDiv 最终选址消费之前。#69 已把 PanDiv 候选准备前移到 `step_trace` 之前，因此旧规格的物理行序无法逐字保持；GammaDump 仍只含标准 Γ_t，不含 PanDiv 通道。

### Slice 2 TDD 证据

- RED 1：先挂模块/测试，`GammaDump` 与 override 未定义。
- RED 2：首次实现直接依赖私有 `StepTrace`，编译器拒绝跨模块访问。
- GREEN：API 改为只接收生产已经算出的 opened gamma index；`cargo test --lib gamma_dump_` = `4 passed`。

新增四项测试：

1. `gamma_dump_env_gated_bit_exact`
2. `gamma_dump_schema_on_synthetic`
3. `gamma_dump_chi_consistency`
4. `gamma_dump_thread_local_override_is_parallel_safe`

既有 `opsem_dump_env_gated_bit_exact` 与 `config_defaults_frozen` 亦单独复核 GREEN。

## 2. 真实数据与三腿验证

运行命令：

```text
ISSUE71_OUTPUT_DIR=/tmp/issue71-chi-gamma-validation \
M8_SYMBOL=BTC M8_WIN_FILTER=wf8 \
cargo test --release --lib \
theta_v0::backtest::wverify_run::issue71_chi_gamma_validation \
-- --ignored --nocapture
```

- 结果：`1 passed`，91.89s；未截窗。
- train：BTC `2023-02-17..2023-08-16`，260,560 bars。
- test：BTC `2023-08-17..2024-02-16`，264,960 bars。
- `ISSUE71_MAX_BARS=None`。
- 参数：θ=0，z_alpha=1.645；Arm1 teap=false；Arm2 teap=true。

### §5-B 三腿

| 腿 | 硬断言 | 结果 |
|---|---|---|
| 基线腿 | env 全不设 vs opsem-only / 双设 / repeat 的 `n_orders`、`trades`、`trade_pnls_with_forced`、`equity_curve` | 全部 bit-exact |
| 隔离腿 | opsem-only vs 双设的 `trades.jsonl` | SHA-256 同为 `8e0cb8663d5930f881c688c4f8efba3d3b4f834360f0606079404a9150abda47` |
| 隔离腿 | opsem-only vs 双设的 `tower_events.jsonl` | SHA-256 同为 `1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f` |
| 确定性腿 | 双设两次的 `gamma_candidates.jsonl` | SHA-256 同为 `b83c9f7fb459fc013f6fb4f0b7fb4e3204b3f6816e58d44bc6c934b71a920eec` |

四臂 Gamma 文件均为 266,478 行 = 264,960 个 bar record + 1,518 个 candidate record。

## 3. M1：chi=None 候选侧频数

总候选 1,518；`nest_confirmed=false` 30，过滤率 `30/1518 = 1.976285%`；opened 508。

| level | dir | bsp_class | n | opened | nest=false |
|---:|---|---:|---:|---:|---:|
| 0 | Long | -1 | 3 | 0 | 3 |
| 0 | Long | 1 | 5 | 0 | 0 |
| 0 | Long | 2 | 135 | 62 | 0 |
| 0 | Long | 3 | 429 | 165 | 0 |
| 0 | Short | -1 | 2 | 0 | 2 |
| 0 | Short | 1 | 6 | 0 | 0 |
| 0 | Short | 2 | 140 | 58 | 0 |
| 0 | Short | 3 | 353 | 138 | 0 |
| 1 | Long | 2 | 129 | 23 | 0 |
| 1 | Long | 3 | 23 | 9 | 0 |
| 1 | Short | -1 | 25 | 0 | 25 |
| 1 | Short | 1 | 27 | 5 | 0 |
| 1 | Short | 2 | 92 | 16 | 0 |
| 1 | Short | 3 | 1 | 0 | 0 |
| 2 | Long | 2 | 36 | 5 | 0 |
| 2 | Long | 3 | 17 | 2 | 0 |
| 2 | Short | 2 | 11 | 0 | 0 |
| 2 | Short | 3 | 1 | 0 | 0 |
| 3 | Long | 2 | 46 | 21 | 0 |
| 3 | Long | 3 | 6 | 1 | 0 |
| 3 | Short | 1 | 21 | 3 | 0 |
| 3 | Short | 2 | 10 | 0 | 0 |

nest_depth 分布：

| depth | n | opened | nest=false |
|---:|---:|---:|---:|
| 0 | 464 | 210 | 16 |
| 1 | 590 | 144 | 13 |
| 2 | 300 | 93 | 1 |
| 3 | 112 | 44 | 0 |
| 4 | 52 | 17 | 0 |

## 4. M2：chi=Some walk-forward 频数

Arm1（teap=false）：

- evaluated = 1,488；
- rejected = 1,482，真实 χ 拒绝率 `99.596774%`；
- admitted = 6；
- unevaluated = 30，占全候选 `1.976285%`；
- admission_value 非 null = 297；evaluated 中 null = 1,191（`80.040323%`，空桶或 n<2，未编造值）。

`admission_value - theta`：

| min | p10 | p25 | median | p75 | p90 | max |
|---:|---:|---:|---:|---:|---:|---:|
| -0.02159801 | -0.00597555 | -0.00565557 | -0.00390958 | -0.00239725 | -0.00202370 | 0.00310915 |

| 距离桶 | n |
|---|---:|
| `<= -0.01` | 15 |
| `(-0.01,-0.001]` | 276 |
| `(-0.001,0]` | 0 |
| `(0,0.001]` | 3 |
| `(0.001,0.01]` | 3 |
| `>0.01` | 0 |

按 `(level,dir,bsp_class,nest_depth)` 的非空桶，以 `depth:n/reject/open` 压缩登记：

```text
L0 Long C1 d0:3/3/0 d1:2/2/0
L0 Long C2 d0:1/1/0 d1:49/48/1 d2:53/50/2 d3:20/19/1 d4:12/12/0
L0 Long C3 d0:169/169/0 d1:142/142/0 d2:59/59/0 d3:39/39/0 d4:20/20/0
L0 Short C1 d0:4/4/0 d2:1/1/0 d3:1/1/0
L0 Short C2 d0:1/1/0 d1:53/53/0 d2:60/60/0 d3:17/17/0 d4:9/9/0
L0 Short C3 d0:155/154/1 d1:104/104/0 d2:64/64/0 d3:19/19/0 d4:11/11/0
L1 Long C2 d0:6/6/0 d1:92/92/0 d2:28/28/0 d3:3/3/0
L1 Long C3 d0:19/19/0 d1:4/4/0
L1 Short C1 d0:18/18/0 d1:9/9/0
L1 Short C2 d0:6/6/0 d1:58/58/0 d2:15/15/0 d3:13/13/0
L1 Short C3 d0:1/1/0
L2 Long C2 d1:25/25/0 d2:11/11/0
L2 Long C3 d0:16/16/0 d1:1/1/0
L2 Short C2 d0:3/3/0 d2:8/8/0
L2 Short C3 d0:1/1/0
L3 Long C2 d0:27/27/0 d1:19/19/0
L3 Long C3 d0:6/6/0
L3 Short C1 d0:11/11/0 d1:10/10/0
L3 Short C2 d0:1/1/0 d1:9/9/0
```

## 5. M3：交叉表与 γ=0 bar

Arm1 八格表：

| chi_evaluated | chi_admit | opened | n |
|---|---|---|---:|
| false | false | false | 0 |
| false | false | true | 0 |
| false | true | false | 30 |
| false | true | true | 0 |
| true | false | false | 1,482 |
| true | false | true | 0 |
| true | true | false | 1 |
| true | true | true | 5 |

| 臂 | bar 总数 | raw γ=0 bar | trade γ=0 bar | χ 将非空 Γ 致空 |
|---|---:|---:|---:|---:|
| Arm0 无χ | 264,960 | 263,792（99.559179%） | 263,792 | 0 |
| Arm1 χ teap=false | 264,960 | 263,792 | 264,924（99.986413%） | 1,132 / 1,168（96.917808%） |

M3 只能把 `chi_admit=true ∧ opened=false` 识别为“下游未开”，不能继续把 ConflictOK 与 RiskOK 分开；本票没有伪造该分解。

## 6. M8 四臂对照与参数行为

当前 `m8.rs` 的 `run_m8_e2e_all_systems_oos` 经 #69 合流已不是旧简报所述四臂；四臂惯例实存于 `run_q4_fullpi_policy`。本票新增的 ignored harness 复用该四臂调用口径，不改既有 M8/Q4 报告。

| 臂 | n_orders | `trade_pnls_with_forced.len()` | Σpnl |
|---|---:|---:|---:|
| Arm0 无χ | 833 | 479 | +1,173,840.77604132 |
| Arm1 χ teap=false | 303 | 299 | -11,679,257.06010596 |
| Arm2 χ teap=true | 301 | 165 | +2,623,082.67309706 |
| Arm3 χ 隔离 | 12 | 9 | -52,177.40911600 |

| 对照 | Δorders | Δtrades | ΔΣpnl |
|---|---:|---:|---:|
| Arm1 - Arm0 | -530 | -180 | -12,853,097.83614728 |
| Arm2 - Arm1 | -2 | -134 | +14,302,339.73320302 |
| Arm1 - Arm3 | +291 | +290 | -11,627,079.65098996 |

以上仅是固定窗口的臂间频数/账面差，不解释为策略质量、概率优势或样本外外推。

参数行为观察：

- z_alpha=1.645、θ=0 下，非空 LCB 多数位于 θ 下方；297 个可数值求值候选中仅 6 个最终由 χ 放行。
- teap=false 对空桶/n<2 候选不放行；teap=true 会改变这类候选的准入路径，但并不简单等价于更多最终订单：Arm2 相对 Arm1 orders 为 -2，typed/forced trade 计数也受后续生命周期与结算口径影响。
- 该差异只供编排者裁定；生产 frozen 默认 `chi_theta=None` 未改。

## 7. §5-C 交叉对账：**FAIL**

以 `(level, source_index, dir)` 对 `opened=true` Gamma 候选与同 run `trades.jsonl.certificate` 做多重集双向 join：

| 臂 | opened | typed trade rows | opened→trade 缺失 | trade→opened 缺失 |
|---|---:|---:|---:|---:|
| Arm0 无χ | 508 | 499 | 9 | 0 |
| Arm1 χ teap=false | 5 | 5 | 0 | 0 |
| Arm2 χ teap=true | 154 | 151 | 3 | 0 |
| Arm3 χ 隔离 | 2 | 2 | 0 | 0 |

Arm0 缺失 9 条：

```text
bar2978   L1 Long  src2922  SameFollow  Ambient   Plus
bar3047   L1 Long  src3017  SameReverse Ambient   Plus
bar3047   L1 Short src3017  SameReverse Ambient   Minus
bar78090  L0 Long  src78063 SameFollow  ShortDiff Plus
bar99685  L1 Long  src99659 SameFollow  ShortDiff Plus
bar149455 L3 Short src149353 SameFollow ShortDiff Minus
bar150372 L3 Short src150282 SameFollow ShortDiff Minus
bar152690 L3 Short src152640 SameFollow ShortDiff Minus
bar209394 L0 Long  src209363 SameReverse ShortDiff Plus
```

Arm2 缺失 3 条：

```text
bar149455 L3 Short src149353 SameFollow ShortDiff Minus
bar150372 L3 Short src150282 SameFollow ShortDiff Minus
bar245306 L1 Long  src245248 SameReverse ShortDiff Plus
```

定位证据：

- Gamma `opened` 直接取生产 `step_trace.opened`，没有自行推演。
- `fill.rs` 随后逐条消费同一 `step_trace.opened`。
- typed 开仓表按 `ElementId` 执行 `open_trades.insert(leg.id, LedgerOpen { ... })`；插入前没有结算/拒绝已有同键。
- 同文件现有注释已经明确 `interpret` 无历史 tombstone，carrier 可再 open，且 `open_trades.insert` 会二次覆盖；`generation` 只进入 `PositionNodeId`，没有进入 `open_trades` 的 HashMap key。
- 因而再次打开同一 carrier 时，较早的 `LedgerOpen` 可被覆盖，最终只剩后一个生命周期落 `TypedTrade`。这与“Gamma 漏标 opened”不相符，也解释了反向 `trade→opened` 始终为 0。

该定位是代码路径与频数的一致解释，未另加临时生产探针证明每个缺失键的覆盖时刻。按设计稿“任一方向失配 = 实装作废”的硬闸，本报告将 §5-C 记为 **FAIL**，不以三腿 GREEN 或测试 GREEN 覆盖它，也不擅自扩大 #71 去修改 typed-ledger 生命周期。

## 8. 照实边界

- 未测 BTC wf8 之外的 symbol/window；本次全窗完成，未截断。
- 未离线重放其他 θ/z_alpha；dump 不含逐 bar μ 历史，不能伪造反事实门结果。
- 未把过滤率或 PnL 差解释为概率、策略有效性或未来表现。
- 未区分 M3 下游拒绝中的 ConflictOK 与 RiskOK。
- 未修复 §5-C 暴露的 carrier 重开覆盖问题；它需要独立裁定 typed ledger 的“一次 opened 对应一条生命周期记录”口径及 key/schema 迁移范围。
- 输出根 `/tmp/issue71-chi-gamma-validation` 是临时证据目录，不属于提交文件。

## 9. 提交文件清单（供线主按票切分）

Slice 1：

- `rust/src/theta_v0/backtest/l3_delta_r_alpha.rs`
- `rust/src/bin/pi_bsp_timing.rs`
- `rust/src/theta_v0/backtest/mu_estimator.rs`
- `rust/src/theta_v0/backtest/ledger.rs`

Slice 2：

- `rust/src/theta_v0/backtest/gamma_dump.rs`（新增）
- `rust/src/theta_v0/backtest/mod.rs`
- `rust/src/theta_v0/backtest/fill.rs`（含 Gamma 两上下文的唯一调用点适配）

Slice 3 验证 harness：

- `rust/src/theta_v0/backtest/wverify_run/issue71_chi_gamma.rs`（新增）
- `rust/src/theta_v0/backtest/wverify_run.rs`

报告：

- `chanlun/review-results/issue71-chi-l0-integration-20260728.md`

禁止文件及共享并行线文件不在本票提交清单。

## 10. Standards FAIL 修复轮（2026-07-28）

本轮只修 Standards 评审点名项；未执行 git 写操作，未还原共享 worktree 的既有改动。

### 10.1 修复内容

1. **GammaDump 证据通道 fail-loud**
   - `OPSEM_GAMMA_DUMP_DIR` 未设或空串仍合法关闭并返回 `None`。
   - env 非 Unicode、目录创建失败、`gamma_candidates.jsonl` 创建失败均立即 panic，错误包含目标路径（env 非 Unicode包含变量名）。
   - `fill.rs` 不再以 `let _ =` 丢弃 `write_step` 结果；写失败 panic 并带完整文件路径。
   - `write_step` 每步显式 flush，避免错误只在 `BufWriter` drop 时被静默丢失。
   - 对照发现 `opsem_dump.rs::at_dir` 的 `.ok()?` 以及 tower-event 写入的 `let _ =` 仍属同类既有债；按本轮边界登记但不修。
2. **800 行闸恢复**
   - `wverify_run/m8.rs` 的 #71 ignored harness 整体移到新文件 `wverify_run/issue71_chi_gamma.rs`。
   - `wverify_run.rs` 登记新模块并把原测试入口改调新模块；`ISSUE71_OUTPUT_DIR`、`M8_SYMBOL`、`M8_WIN_FILTER` 与 `#[ignore]` 保持不变。
   - `m8.rs` 恢复 **710 行**，且相对 HEAD **零 diff**。
3. **runner 测试迁移**
   - 四个 `gamma_dump_*` 测试整体迁入 `gamma_dump.rs` 的 `#[cfg(test)] mod tests`。
   - 合成 `buy1_at3_confirmed_at7`/bar fixture 在 GammaDump 测试模块本地承载，生产 `pi_theta_fill_loop` 仍直接从 `fill` 模块调用。
   - `runner.rs` 恢复 **7673 行**，且相对 HEAD **零 diff**。
4. **量纲③ helper 单一归属**
   - `chi_dimension_three_return` 唯一定义移到 `mu_estimator.rs`、紧邻 `marginal_return`；`l3_delta_r_alpha.rs` 与 `pi_bsp_timing.rs` 改为 import。
   - 两处既有 `chi_feed` 测试留在原调用点。
   - 可见性说明：`pi_bsp_timing` 是独立 binary crate，不能导入 library 的 `pub(crate)` 符号；为保持单一实现并使 bin 验收可编译，helper 使用 `#[doc(hidden)] pub`，不进入生成文档 API 面。

### 10.2 文件搬家清单

| 原位置 | 新位置 | 结果 |
|---|---|---|
| `wverify_run/m8.rs::run_issue71_chi_gamma_validation` | `wverify_run/issue71_chi_gamma.rs` | m8 恢复 710 行/零 diff |
| `runner.rs` 四个 `gamma_dump_*` 测试 | `gamma_dump.rs::tests` | runner 恢复 7673 行/零 diff |
| `l3_delta_r_alpha.rs` 与 `pi_bsp_timing.rs` 两份同款 helper | `mu_estimator.rs::chi_dimension_three_return` | 单一定义，两调用点 import |

本轮提交清单已同步更新至 §9：新增 `mu_estimator.rs` 与
`wverify_run/issue71_chi_gamma.rs`，移除零 diff 的 `runner.rs`、`wverify_run/m8.rs`。

### 10.3 指纹复核

| 命令 | 结果 |
|---|---|
| `cargo build --all-targets` | GREEN，10.89s |
| `cargo test --lib gamma_dump_` | GREEN，4 passed / 0 failed |
| `cargo test --lib chi_feed` | GREEN，1 passed / 0 failed |
| `cargo test --features backtest_bin --bin pi_bsp_timing chi_feed` | GREEN，1 passed / 0 failed |
| `cargo test` | **2019 passed / 1 failed / 136 ignored** |

全量唯一失败仍为
`theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`：
left `16618955402698307653`，right `10432481772907336594`，#115 在案；前后指纹未漂移。

### 10.4 harness 复跑证据

命令：

```text
ISSUE71_OUTPUT_DIR=/tmp/issue71-chi-gamma-validation-fix \
M8_SYMBOL=BTC M8_WIN_FILTER=wf8 \
cargo test --release --lib \
theta_v0::backtest::wverify_run::issue71_chi_gamma_validation \
-- --ignored --nocapture
```

- 结果：**1 passed / 0 failed**，91.14s。
- train：BTC `2023-02-17..2023-08-16`，260,560 bars。
- test：BTC `2023-08-17..2024-02-16`，264,960 bars。
- `ISSUE71_MAX_BARS=None`，未截窗。
- 三腿硬断言全部通过：基线生产输出一致、OPSEM 隔离逐字节一致、Gamma 两跑逐字节一致。
- 原始摘要：`/tmp/issue71-chi-gamma-validation-fix/raw-summary.md`。

顺带登记不修项：harness 内原地截断数据集/改配置属于 test-only `#[ignore]` 诊断入口，隔离后接受；硬编码日期/参数同属该入口，接受；`write_step` 8 参数为一次成型 API 面，改动收益低于风险，本轮不改。

## 11. Standards 复审遗留修复轮

本轮只闭合第二轮点名的结构性遗留；保留 §10 的 fail-loud、文件行数、runner/m8
零 diff 与量纲 helper 单源结论。未执行 git 写操作。

### 11.1 函数拆分与签名

- `GammaDump::write_step` 从 79 行拆为：
  - `write_bar_record`：17 行；
  - `write_candidate_records`：11 行；
  - `write_candidate_record`：42 行；
  - `admission_fields`：25 行；
  - `write_step`：14 行。
- bar 输入收束为 `GammaBarContext`，χ/成员关系输入收束为 `GammaChiContext`；
  `write_step` 由 8 参数降为 2 个上下文参数，任一新增函数签名均不超过 5 参数。
- `fill.rs` 只同步唯一调用点的两个上下文构造；既有写失败 panic、完整路径和逐步 flush
  均未回退。
- `run_issue71_chi_gamma_validation` 从 205 行降为 20 行；拆出的
  `setup_validation` 45 行、`run_arm` 23 行、`three_leg_asserts` 37 行、
  `write_report` 23 行，最长新增 helper `run_chi_arms` 恰为 50 行。

### 11.2 常量与构造式改写

模块顶具名常量清单：

- 日期/默认入口：`DEFAULT_SYMBOL`、`DEFAULT_WINDOW_FILTER`、`DEFAULT_OUTPUT_ROOT`、
  `P3_TRAIN_START`、`P3_TRAIN_END`、`P3_TEST_START`、`P3_TEST_END`；
- 年化口径：`MINUTE_BARS_PER_YEAR = 365.25 × 24 × 60`；
- χ 口径：`CHI_THETA=0`、`CHI_Z_ALPHA=1.645`、
  `STRICT_TEAP=false`、`PERMISSIVE_TEAP=true`。

`limited_dataset` 消费切窗结果并返回新 `Dataset`，以 iterator `take` 构造新的
`bars/dates`，不再原地 `truncate`。`chi_config` 通过 `ThetaConfig`/`RiskConfig`
结构体更新语法一次性构造臂配置，不再逐字段修改 `let mut` 配置。

### 11.3 指纹验收

| 命令 | 结果 |
|---|---|
| `cargo build --all-targets` | GREEN，11.40s |
| `cargo test --lib gamma_dump_` | GREEN，4 passed / 0 failed |
| `cargo test --lib chi_feed` | GREEN，1 passed / 0 failed |
| `cargo test --features backtest_bin --bin pi_bsp_timing chi_feed` | GREEN，1 passed / 0 failed |
| `cargo test` | **2019 passed / 1 failed / 136 ignored** |

全量唯一失败仍为
`theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`：
left `16618955402698307653`，right `10432481772907336594`；失败集与指纹均未漂移。
`runner.rs` 仍为 7673 行且相对 HEAD 零 diff；`wverify_run/m8.rs` 仍为 710 行且
相对 HEAD 零 diff。

### 11.4 harness 与逐字节复证

```text
ISSUE71_OUTPUT_DIR=/tmp/issue71-chi-gamma-validation-fix2 \
M8_SYMBOL=BTC M8_WIN_FILTER=wf8 \
cargo test --release --lib \
theta_v0::backtest::wverify_run::issue71_chi_gamma_validation \
-- --ignored --nocapture
```

- 结果：**1 passed / 0 failed**，97.45s。
- train：BTC `2023-02-17..2023-08-16`，260,560 bars。
- test：BTC `2023-08-17..2024-02-16`，264,960 bars；未截窗。
- 三腿硬断言全部通过；新旧 `raw-summary.md` 逐字节一致。
- `/tmp/issue71-chi-gamma-validation-fix2` 与
  `/tmp/issue71-chi-gamma-validation-fix` 的五个臂 Gamma 文件逐一 `cmp` 相同：

| 臂 | 新旧一致的 SHA-256 |
|---|---|
| Arm0 both / repeat | `b83c9f7fb459fc013f6fb4f0b7fb4e3204b3f6816e58d44bc6c934b71a920eec` |
| Arm1 χ teap=false | `4c9c98138fb700772314eeee728fe613319c7f32123c8134973aa2049897c778` |
| Arm2 χ teap=true | `31700beb7aacefceb908ff80bc837ed5befc76d6d6c900e8a90a3be13bc5760c` |
| Arm3 χ isolation | `463a295a58c1a18293e9b5da81dcc200d6b3cf39b1109d517bf0b921a99ee11a` |

## 12. Standards 三轮遗留微修（编排侧补录，2026-07-28）

三轮复审遗留（仅 `issue71_chi_gamma.rs` 一个文件）：4 处 IO/断言消息缺路径 + 2 处魔法数字。
微修由 codex 执行（禁碰其他文件），本节由编排侧按修复方终报补录。

- **消息补齐**：全部 IO/解析/断言消息含路径、键或现场值；全文件同类扫尾完成。
- **常量化**：`NAV_FALLBACK_UNIT_PRICE` / `NAV_NOTIONAL_MULTIPLIER` / `MU_TIME_BLOCK_BASE` 具名。
  **订正**：三轮所报「fee 硬编码 0」经修复方核为 time-block 基址参数误认，已正名
  `MU_TIME_BLOCK_BASE`；`NAV_NOTIONAL_MULTIPLIER` = q4/p3fold 冻结口径「nav₀=窗首可交易价×1000」
  （`m8.rs:77` 同源注释）。
- **函数长度**：全文件 ≤50 行（扫尾声明）。
- **验收**：`cargo build --all-targets` 绿；GammaDump 4/4、两组 chi_feed 各 1/1；全量指纹
  2019/1/136 不漂移（唯一失败 #115）；release harness 1 passed（95.52s），
  fix2/fix3 输出 18 个文件逐字节一致（摘要 SHA `00cab58d…9a788`）。
- §9 提交文件清单不变（仅 `issue71_chi_gamma.rs` 内容修订，无新增文件）。
