# 影子评审 #370 — #360 venue 费率标定实装（两轴，独立上下文）

- 评审对象：commit `1b2642c038`（16 文件 +1629/−56）；工作面 = `/tmp/nc-review-360`（detached @ `247b7fc887`，含 #360 全部内容）。
- 评审者：独立 opus lineage（新上下文），**不采信实装自述**——下述每项均自行复核。
- 纪律：090（照实）；231（认识论等级）；禁 git mutation（本评审只读；临时 symlink 行情文件跑完对拍后已删除，`git status` 干净）。

## 裁决

**PASS**，问题分级：**MED×2 / LOW×6，无 HIGH**。Spec ①②③④⑤ 成立，⑥ 部分成立；Standards 四面三合格一越线。
None 支「逐值不破」经独立核实成立；Some 支数字与报告 §2 一手来源逐项吻合（我手算重演全部对上）。

## 复跑证据（我方独立执行）

| 项 | 命令 | 结果 |
|---|---|---|
| 全库基线 | `cargo test --release --lib` | **1920 passed / 1 failed / 134 ignored**，与自述一致 |
| 唯一失败 | — | `classifier::signal::extract_signals_bit_exact_digest_guard`（`signal.rs:3382`，0xe6a2… ≠ GOLDEN 0x90c7…）。**与 #360 无关**：该文件未在 diff 内，费率不进 classifier 输入域。附注：其"#115 线在案"的归属我未能核实（#115 = `#110-review` 影子评审票，标题无关），但不影响本票判定 |
| 定向 | `--lib venue_fee` | 22 passed / 1 ignored |
| 定向 | `--lib fee_quoter` | 2 passed |
| 真实窗对拍 | `--lib real_window_datum_reconciliation -- --ignored` | **通过，逐值复现自述**：BTC 2000bar/40单 Σfee 886.8103→3547.2414，账本差 = 重算差 = 2660.431032；OKLO 68.8738→63.4888，差 = −5.385046。（review worktree 无行情缓存——白名单只放 datum——需从主仓 symlink 两个 json 才能跑，跑完已删） |
| datum 哈希 | `shasum -a 256 -c venue_fee_*.sha256` | **OK ×2**（独立于 rust 实现） |
| 构建口径 | `cargo check --release --lib` | 0 error（cdylib 默认构建不含 sha2/serde_json datum_io 路径） |
| 构建口径 | `cargo check --release --features backtest_bin --all-targets` | **4 error**，全部来自 `tests/econ_oddeven_diagnosis.rs`（`Rc<Vec<_>>` / `LevelState` 缺 3 字段），与 #360 无关 → 见 LOW-H |

## Spec 轴逐项

### ① None 逐值不破的证据链 —— **成立**

三条独立支撑，我逐条核实：

1. **同表达式同约简序**：旧 inline 式 `(commission_bps + slippage_bps + tax_bps)/10_000.0`（`fill.rs:46-48` 等四处）→ `treasury::fee_rate` 逐字同序同括号（`treasury.rs:33`），无重排。
2. **None 支不做任何算术**：`FeeQuoter::rate_for` 的 `None => self.fallback_rate`（`venue_fee.rs:206`），非 `fee_usd/notional` 除法；全枚举 `assert_eq!` 两处（`venue_fee.rs:744`、`treasury.rs:130`）。
3. **fallback 占位位点确实不进算术**（这条自述最需核实，我逐点走过，**结论成立**）：
   - `order_fee_rate` 的 `None` 分支只在 `Hold/Wait/空仓平仓` 触发 → `apply_order` 随即 `noop`；
   - `leg_fee_rate` 的 `q_leg = 0` 情形 → `apply_fill_dual` 在 `close_qty <= 0.0` 早退（`dual_ledger.rs:185/206`）；
   - `forced_flatten_fee_rate` 两处调用均被 `if units != 0.0` 包裹（`fill.rs:1971` / `fill.rs:2506`）；
   - `overlay_state::open_fee_rate` 在 `q_lots <= 0 || px <= 0` 早退**之后**才求值（`overlay_state.rs:523-530`），`flatten` 用 `pos.q`（构造期即 >0）。

冻结 digest / 守恒测试全绿（唯一失败与费率无关，见上表）。

### ② Some 对账 vs 报告 §2 一手数字 —— **成立（我独立手算重演）**

| 项 | 报告 §2 | datum 字段 | 我方手算 | 实装断言 |
|---|---|---|---|---|
| Binance VIP0 | maker/taker 0.1000% | 10.0/10.0 bps | 10bp ✓ | `= 1e-3` 逐位 |
| Binance BNB25 | 0.075% | 7.5/7.5 | 7.5bp ✓ | `= 7.5e-4` |
| IBKR 100股@$20 买 | §2.3 逐项 | — | `max(0.35,0.35).min(20)=0.35` + `0.35×0.00074=0.000259` + `0.000203×100=0.0203` = **0.370559** ✓ | `0.370559` |
| 同单卖 | +SEC+TAF | — | `+2000×0.0000206=0.0412` `+min(0.0195,9.79)=0.0195` = **0.431259**，差 **0.0607** ✓ | `0.431259` / 差 `0.0607` |
| 10 股（最低佣金主导） | min $0.35 | — | **0.352289**（17.61445bp）✓ | 同 |
| 1 股（1% 上限压最低佣金） | 上限 1% | — | `min(0.35→0.2)` = **0.200351** ✓ | 同 |
| 10 万股卖（TAF 封顶） | cap $9.79 | — | `350+0.259+20.3+41.2+9.79` = **421.549** ✓ | 同 |

datum 全字段逐项 = 报告 §2.1 / §2.3 / §2.5（0.0035 / 0.35 / 1% / 0.00020 / 0.000003 / 0.0000206 / 0.000195 / 9.79 / 0.000175 / 0.000565），无编造、无越出来源的数字。

### ③ datum 契约防篡改链 —— **闭环**

- sidecar 哈希我用 `shasum -a 256 -c` 独立复核 OK（非自造校验和；`sha256_hex` 另有 NIST 向量锁 `venue_fee.rs:471`）；
- 哈希算的是**读入的同一份字节**，随后 `serde_json::from_slice(&bytes)` 用同一 buffer → 无 TOCTOU 窗口；
- 篡改 / 缺 sidecar / 未知 `schema_version` / 未入簿品种四路 Err 均有测试并通过；
- 白名单让 datum 真入仓（`git ls-files` 仅多出 4 个文件），任何 clone 可复核 —— 契约要求成立。
- 缺口见 LOW-G（另 4 条 Err 分支无测试）。

### ④ 标定档口径 = datum + `slippage_bps`，与 #303 spot 声明一致 —— **成立**

- BTC 档 = Binance **现货** VIP0；datum note 明写「venue 口径 = spot（#303 裁定…）与 perp 费率表不混用」，与 `risk.rs` 的 spot 裁定同口径；
- 滑点相加而非替换（`FeeQuoter::uncalibrated_addon`），与报告 §3.3「保留 `slippage_bps` 未标定」一致；`addon=0` 的可分离性物证在测（`venue_fee.rs:789`）；实测 OKLO 窗标定后总摩擦**下降**（63.49 < 68.87），代码注释已照实声明「不是普适命题」，未把"标定必然更贵"当结论——合格；
- `tax_bps ≠ 0` 与标定档并用 fail-loud（`treasury.rs:44`），防双计；
- 持有成本三项照实保持 `[L1机制/费率未标定]`，未跳级。

### ⑤ 品种绑定（跨品种借档 Err）—— **成立**

`data::load_by_symbol` 校验前置于读盘（`data.rs:296`），大小写不敏感与 `SYMBOLS` 查表同口径，有测试。
边界：校验只挂在 `load_by_symbol` 这条路径上——直接调 `load_symbol` 或自造 `Dataset` 不受校验。生产/研究 bin 均走 `load_by_symbol`，可接受。

### ⑥ `[L2费率标定]` 标注位置 —— **部分成立**

`wverify_run.rs:1136/1260` 两处已换成构造子；但第三处 `runner.rs:5399`（M8 treasury 多窗报告，同样带成本数值）仍硬编 `RATE_UNCALIBRATED_LABEL` → **LOW-C**。且 L2 分支当前无生产者 → **MED-B**。

## Standards 轴四面

| 面 | 裁决 | 依据 |
|---|---|---|
| `venue_fee.rs` 内聚 | **越线（LOW-D）** | 804 行 > `coding-style.md` 的 800 硬上限。职责单一、`datum_io` 已 cfg 隔离，切文件成本极低 |
| FeeQuoter 单源收口 | **票面三处合格；票面外仍有旁路** | 我把 `apply_order`/`apply_fill_dual`/`apply_open`/`apply_close`/`settle_forced_virtual` 的**全部**生产调用点列了一遍：fill.rs 四回路（`:75`/`:972`/`:2351`/`:2406`/`:2726`/`:2806`/`:2946`/`:2981`）+ overlay 三扣费点 + 两处强平，**无一处漏用 quoter**。旁路在票面外：`runner.rs`×4、`econ_positive.rs`×2、`l3_delta_r_alpha.rs`、`bin/`×2、`exec::apply_fees`——且这些位点连 fallback 都是 inline 复制公式（报告 §3.1 item 3 要求的「收敛到 `treasury::fee_rate` 单源」未做）。已登记，但可观测后果 = **MED-A** |
| `.gitignore` 白名单最小性 | **合格** | `git ls-files analysis/data_cache/` 恰 4 个 datum 文件；`git check-ignore -v` 实测行情缓存仍被 `:82` 忽略。`/*` + `!` 的改法是 git 规则所迫（目录级排除无法被 `!` 重纳），非取巧 |
| `ExecConfig` 去 `Copy` 爆炸半径 | **小** | `ThetaConfig` 本来就只 `Clone`（`config.rs:354`）；默认 lib 与 `backtest_bin --all-targets` 均无 `Copy` 相关 error（唯 4 个 error 属陈旧测试文件，LOW-H） |

## 问题清单

### MED-A｜标定档下随机对照与成本剥离仍按未标定常率，且无 fail-loud

- 位点：`runner.rs:242`（同 `:320`/`:546`/`:737`）产 `RunResult::fee_rate`；消费于 `metrics.rs:350 random_entry_controls` 与 `l3_delta_r_alpha.rs:861 rebuild_cost_series`。
- 翻转场景：`exec.fee_schedule = Some(OKLO)` 时，策略臂逐笔按 per-share datum 扣费（10 股单 17.6bp），随机对照臂与成本剥离仍按 3bp → `metrics.rs:316` 的注释断言「随机对照含**同等**成本」变为假；`ΔGross = ΔR + ΔC` 的 C 被系统性低估 → 鞅守卫读数偏移。
- 严格性论点：实装对**同类**「口径双源」问题（`tax_bps` 与 datum 重复计）选择了 `assert!` fail-loud（`treasury.rs:44`），此处却只在 `config.rs:275-279` 写文字登记、无任何 guard。同一风险两种处置 = 不一致（090）。
- 建议（不扩大范围）：`RunResult::fee_rate` 改 `Option<f64>`，或在 `fee_schedule.is_some()` 时于该四处 `assert!`/`panic!` 挡住——把"未收编"从文字登记升级为代码锁。

### MED-B｜L2 档在生产不可达，标签 L2 分支无生产者

- `ExecConfig::default()` 恒 `None`；全仓 `fee_schedule = Some(...)` 的赋值点**只在测试内**（我 grep 核过）；CLI `theta_backtest.rs` 只收 `<SYMBOL> [START END]`，无注入通道。
- 后果：datum + quoter + `rate_calibration_label` 三件在生产口径上等价 no-op；`wverify_run` 两处升级后的标签**永远输出 L1**。`config.rs` 文档称本字段是费率落差的「**可执行**出口」——端到端尚不可执行，措辞略强于事实。
- 已列入 resolution 偏离⑥（「票体未要求」），我确认票体确实未要求；故不判 HIGH。但这决定了 #360 的价值何时兑现，应作为后继票的第一顺位（连同 MED-A 一并解）。

### LOW-C｜第三处口径标签产物未随档升级

`runner.rs:5399`（M8 treasury 多窗报告，A10 注入时同样输出带成本数值）仍硬编 `RATE_UNCALIBRATED_LABEL`，与 `wverify_run` 两处改法不一致。标定档下该报告会自称 L1（方向上是保守的，不构成跳级），但标签与实际运行档脱钩。

### LOW-D｜`venue_fee.rs` 804 行越 800 行硬上限

`.claude/rules/common/coding-style.md`：「Files are focused (<800 lines)」。建议 `datum_io`（约 200 行）单独成 `venue_fee/datum_io.rs`。

### LOW-E｜per-share 档 + 开仓段现金拒单 ⟹ 低估成本，偏差无测试固化

`fill.rs:96-100` 已诚实登记该偏差（询价量 = 请求量上界，开仓段整段拒单时最低佣金按偏大量摊薄 ⟹ 低估）。但：无测试锁住偏差方向与量级，也无 `debug_assert` 在偏差发生时留痕 → 后续重构可能悄悄放大。建议加一个构造性测试（现金刚好只够平反向段）固化「低估」这一有向事实。

### LOW-F｜报告 §3.1 item 2 要求的 `*_provenance.md` 溯源件未落，且未列入偏离清单

溯源信息内嵌于 JSON 的 `note` + `source_url`（可用），但报告明文要求「照搬 #62 §4 的列式+哈希约定与 `*_provenance.md` 溯源件」。这是一处未声明的规格偏离——偏离本身可接受（信息未丢），**未登记**才是问题（照实纪律）。

### LOW-G｜datum 装载的 4 条 Err 分支无测试

已测：哈希不符 / 缺 sidecar / 未知 `schema_version` / 未入簿 (symbol,tier)。未测：未知 `unit`、`(symbol,tier)` 重复、`entries` 空、字段非有限或负（`need()`）。这四条都是防篡改链的组成部分，属 fail-loud 语义的一部分，宜同等固化。

### LOW-H｜自述「全测试目标三口径 0 error」对 `--all-targets` 不成立

`cargo check --release --features backtest_bin --all-targets` 有 4 个 error，全在 `tests/econ_oddeven_diagnosis.rs`（`Rc<Vec<_>>` 与 `LevelState` 三字段陈旧），**与 #360 无关**（该文件不在 diff 内，错误与费率无涉）。问题不在 #360 引入回归，而在 resolution 的口径措辞覆盖面大于实测范围（090）。

## 结果包六要素

1. **结论**：#360 实装 **PASS**。Spec ①②③④⑤ 成立、⑥ 部分成立；Standards 三合格一越线。MED×2 / LOW×6，无 HIGH，无「绕过矛盾」「补丁思维」类违规。None 支逐值不破经独立核实（不止于自述），Some 支全部数字我手算重演吻合。
2. **定义依据**：报告 `venue-fee-source-research-20260726.md` §2.1/§2.3/§2.5（一手费率数字）、§3.1（两种计费单位必须原生表达 + 消费点收敛 + 全 taker 初档）、§3.3（滑点保留未标定）；`risk.rs:709` 口径升级契约（不得跳级）；#303 spot 裁定；#62 §4 datum 列式+哈希约定。输入数据侧：datum 两份文件的字段值逐项落在报告 §2 的一手数字上，`sha256` sidecar 独立复核通过，故满足「L2 = venue 官方费率表版本化快照」的判据。
3. **边界条件**（结论翻转的条件）：
   - 若把 `ExecConfig::default()` 切成 `Some(datum)`，MED-A 立即从潜在升为**实际**（随机对照/鞅守卫成本口径不对称），本 PASS 翻为 HIGH；
   - 若发现 `apply_fill`/`apply_fill_dual`/`apply_open` 中存在**部分成交**（executed < requested 且不是我核过的两种早退/clamp 情形），则「询价量 = 成交量」前提破，per-share 档等效费率错，Spec ② 翻转；
   - 若 datum 的 `unit`/字段语义与 IBKR 实际计费顺序不同（例如 1% 上限应先于最低佣金而非后），则 `.max(min).min(cap)` 的约简序错，§2 对账翻转（我按 IBKR 页「min $0.35 / max 1% of trade value」双侧约束读，取 cap 压 min，与实装一致）;
   - 若 `analysis/data_cache/` 未来出现体量大的 `venue_fee_*.json`，白名单最小性结论翻转。
4. **下游推论**：
   - 任何**当前**在册的带成本数值结论**逐位不变**（default `None`），无需重跑——这是本次 PASS 的直接推论；
   - 反之，一旦启用标定档，所有 `RunResult::fee_rate` 的下游（significance 随机对照、`l3_delta_r_alpha` 鞅守卫、`econ_positive`、两个 bin）都进入「未登记口径」状态，其数值不可作 alpha 论据——这条比 #360 前更强，因为不一致性从"全局单一常率"变成"账本与对照两套口径"；
   - 口径标签的可信性现在依赖 `rate_calibration_label` 的覆盖完整性，`runner.rs:5399` 是已知缺口（LOW-C）。
5. **谱系引用**：涉 231 号（有效域 ≠ 定义域）——实装的 L2 声明严格限于「佣金/监管/清算」科目，滑点与持有成本三项照实留 L1，认识论等级标注在每组测试头（`fill.rs:298-300` 明写合成 bar ⟹ L1、datum 的 L2 由 `venue_fee` 对账测试承担），未出现有效域膨胀；涉 090 号（声明膨胀禁止）——`LiquidityRole::Maker` 档存在但生产恒 Taker，代码明写「声称 maker 档是声明膨胀」，合规；LOW-H 是 090 的轻微违反（自述口径宽于实测）。未发现与 no-workaround / no-patch-mentality 相关的谱系冲突：`Option<VenueFeeSchedule>` 不是兼容性垫片（旧三常数是**被显式定义**的未标定 fallback，带 L1 标签，不是"保留已知错误的旧代码作 fallback"）。
6. **影响声明**：本评审为只读产出，未改任何代码/配置。产出文件 = 本报告。评审期间在 `/tmp/nc-review-360/analysis/data_cache/` 临时 symlink 过 `btc_1m_full.json` / `oklo_1m_databento.json`（跑 `#[ignore]` 对拍所需），**已删除**，`git status` 干净；未进入 `/tmp/kimi-nest-mainline` 除本文件写入外的任何路径；未执行任何 git mutation。
