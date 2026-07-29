# 影子评审报告：#388 标定臂（臂D 真实费率）+ OKLO 真实窗

- **评审票**：#418　**被评票**：#388（CLOSED，resolution 在案）　**母 SPEC**：#385（map #59 Destination 末项）
- **被评对象**：commit `d7d3451c64`（11 文件，+1049/-50）+ 产物链
- **评审工位**：`/tmp/nc-review-388`（隔离 worktree，detached @ `d7d3451c64`）。全程只读，零 git mutation，未在工位内构建（无 `rust/target` 产生）。
- **评审者**：opus 新上下文，与被评交付零关系（禁自评）
- **日期**：2026-07-27

---

## 结论

**PASS with MED×1 + LOW×5。**

七条验收标准全部达成或按票体口径降级并照实登记；#385 的三条「红线」（不变量破 / 臂R 发散无法归因 / 标定臂标签缺失）**均不触发**。全部可机器复核的声明我都独立复跑或独立重算过，**无一条对不上**（详见「重点核查」六条）。

唯一的 MED 不是数值错，是**理由错**：臂D 的 datum 是 Binance **per-notional**（maker=taker=10bp）、撮合角色恒 Taker ⟹ 该臂的单边费率**恒等于 12bp 常数**，单标量口径在这一臂上**有**良定义；但交付产物与报告把撤下 LCB/三态的理由写成「在 **per-share** 档无良定义」。方向是保守的（不会产生假 alpha），所以不打回；但它会随 T3（#389 帽臂 = 臂D + 级别帽，同样是 per-notional）原样继承，**建议在 T3 复用同一段声明文本之前先订正**。

---

## 分级发现

### MED-1 — 标定档降级的理由与臂D 实际档位不符；臂D 层4 被以一个对本臂不成立的理由撤下

**位置**
- `rust/src/theta_v0/backtest/treasury.rs:76-79`（`scalar_cost_rate_opt`：`Some(_) => None`，按 `fee_schedule.is_some()` 一刀切）
- `rust/src/theta_v0/backtest/wverify_run.rs:1336-1352`（报告头降级声明块的措辞）
- 产物 `/tmp/388_armD_report_{p3fold,wf7,wf8}.md:9`
- 报告 `chanlun/review-results/treasury-reverify-t2-armD-20260727.md` §0.3 / §3 / §10

**主张**

降级声明的原文是：「单标量成本费率（`RunResult::fee_rate`）在 **per-share 档**无良定义 ⟹ 下列读数标不可用」。而本臂跑的是 Binance 现货 per-**notional** 档：

- `rust/src/theta_v0/venue_fee.rs:158-172`：`FeeUnit::Notional` 分支的 `effective_rate` **完全不依赖 qty/px**，只按 `LiquidityRole` 取 `maker_bps` / `taker_bps`，返回 `bps/1e4`；
- `rust/src/theta_v0/backtest/fill.rs:139,154,173,180`：本引擎**流动性角色恒 `Taker`**（模块注释明文，全部成交按 bar close 市价撮合）；
- `analysis/data_cache/venue_fee_binance_spot_20260726.json`：BTC/VIP0 `maker_bps = taker_bps = 10.0`；
- `treasury.rs:87-104` `fee_quoter`：标定档 = datum 费率 + `uncalibrated_addon`（滑点 2bp）。

⟹ 臂D 的逐笔单边费率**恒 = 12bp**，与 (qty, px, side) 无关。「随机对照含同等成本」在这一臂上有一个良定义的标量。所以：

1. **Standards（090 声明与实际一致）**：交付产物中给出的撤下理由，对该产物描述的那一臂**不成立**。报告 §1 虽然写了「maker=taker=10bp」，但全文从未把这一事实与「本臂标量其实有定义」连起来，也未登记「这是承袭 #374 `is_some()` 一刀切的保守收窄，而非本臂的必然」。
2. **Spec（#385 US3）**：「标定臂（BTC=Binance spot VIP0）的三窗验收读数」因此少了层4（LCB_OOS(R) / 三态），而这一层在本臂本可产出。

**边界（为什么不是 HIGH、不是打回）**：错误方向是**保守**的——撤下读数不会造出假 alpha，不违反 v3 硬禁令；`is_some()` 一刀切的判定**承袭自 #374**，不是 #388 新造的。#388 新增的部分是「第一次把这条理由写进交付产物」，而写下的理由与本臂档位不符。

**最小复现**
```
$ python3 -c "import json;b=json.load(open('analysis/data_cache/venue_fee_binance_spot_20260726.json'));print([ (e['symbol'],e['tier'],e['unit'],e['maker_bps'],e['taker_bps']) for e in b['entries']])"
[('BTC', 'VIP0', 'notional', 10.0, 10.0), ...]        # unit=notional，maker=taker
$ sed -n '158,172p' rust/src/theta_v0/venue_fee.rs     # Notional 分支：忽略 qty/px，返回 bps/1e4
$ sed -n '139,155p' rust/src/theta_v0/backtest/fill.rs # 角色恒 Taker
$ sed -n '9p' /tmp/388_armD_report_p3fold.md           # 产物声明：「在 per-share 档无良定义」
```

**建议处置（任选其一，T3 之前）**
- (a) 把 `scalar_cost_rate_opt` 的判定从「`fee_schedule.is_some()`」改为「datum 的等效费率是否与 (qty,px,side) 无关」——`FeeUnit::Notional` 且角色固定 ⟹ `Some(taker_bps/1e4 + addon)`，`FeeUnit::PerShare` ⟹ `None`。臂D/臂C 层4 随即恢复；或
- (b) 保持一刀切，但把产物声明与报告的理由改述为「本实现按档位类型一刀切收窄；本臂 datum 为 per-notional 常率，标量其实可定义，层4 的缺席是保守收窄而非无良定义」，并登记为待解锁项。

**不接受的处置**：维持现状把 (b) 的事实继续留在暗处——那正是 090 禁止的「声明与实际不一致」。

---

### LOW-1 — §0.4 / §5 的「证伪常数费率前提」表述越出证据有效域

**位置**：报告 §0 结论 4、§5 第 4 个 bullet（`chanlun/review-results/treasury-reverify-t2-armD-20260727.md:206`）

原文：「有效费率因单量从 7.65e-5 变到 1.38e-4（1.8 倍）。这直接证伪『存在一个使随机对照/成本剥离成立的常数费率』这一前提。」

OKLO 证据只覆盖 **per-share** 形态。per-notional 形态存在**反例**——就是本报告自己那一臂（恒 12bp，见 MED-1）。准确表述是「证伪了『**在 per-share 档**存在这样的常数』」。231号有效域纪律要求这句话自带档位限定词。与 MED-1 同源（把「标定档」整体等同于「per-share」）。

---

### LOW-2 — OKLO 产物的口径标签是**未填充的字面占位符**，认识论等级不可机器读

**位置**：`rust/src/theta_v0/backtest/fill.rs`（`oklo_real_window_per_share_readings` 的 md 头字符串）→ 产物 `/tmp/388_oklo_per_share_readings.md:3`

产物第 3 行原文：

```
口径标签：`[L2费率标定: datum <前12位>]`（成交费率科目）。**认识论 L1**（费用算术 + 触达计数；不作 alpha 论据）。
```

`<前12位>` 是字面占位符，不是 hash。真 hash（`cc0d3fa3695d`）在下方 bullet 里，由 `&sched.datum_sha256[..12]` 正确插值——同一个测试里下半段做对了，抬头忘了。母 SPEC #385 US4 的目的是「读数的认识论等级**机器可读**」；正则 `\[L2费率标定: datum ([0-9a-f]{12})\]` 在这份产物上匹配失败。三窗 m8 产物（`/tmp/388_armD_report_*.md:7`）无此问题，标签正确。

**最小复现**：`grep -n '口径标签' /tmp/388_oklo_per_share_readings.md`
**修法**：抬头改为 `format!` 插值 `&sched.datum_sha256[..12]`（与该测试自己的 footer 同源）。

---

### LOW-3 — 入仓报告的自述状态已过期（「未提交」「其后无 commit」）

**位置**：报告 §1 抬头、§9 标题、§12 抬头

报告在入仓 commit `d7d3451c64` 上仍写着「交付为**未提交改动**，零 git mutation」「代码改动清单（**未提交**）」「固定点 = `0dc5bc785d`（**其后无 commit**）」。实际链：`0dc5bc785d → 9e3cd25fd6 → 78151e988e → d7d3451c64`。

**已核实无实质影响**：中间两个 commit 只动 `chanlun/`，`rust/src` 零改动；我把评审工位（`d7d3451c64`）与跑批工位（`/tmp/kimi-nest-mainline`）的 11 个文件逐一 `diff`，**10 个 byte-identical**，唯一差异是 `wverify_run.rs`（159 行，全部是 T3 #389 帽臂 `M8_LEVEL_CAP` 的新增块，与 #388 无关）。故读数有效性不受影响，这条纯属文档时态。

---

### LOW-4 — `fee_datum_cross_symbol_borrow_is_blocked` 的「磁盘无关」注释不准确，且与既有测试语义重叠

**位置**：`rust/src/theta_v0/backtest/wverify_run.rs`（新增测试注释「**磁盘无关**（品种校验先于文件读取）」）

该测经 `parse_fee_datum_spec` → `load_datum` **读了仓内 datum 文件**，不是磁盘无关；准确说法是「不依赖**市场数据**文件」。品种校验先于读盘这一点本身已复核为真（`data.rs:299-310`，`if let Some(sched)` 分支在 `data_dir().join(file)` 之前），全量 `--lib` 1.07s 完成也侧证没有加载 BTC 全量 461 万 bar。

另：`data::tests::fee_schedule_symbol_must_match_dataset`（#360 已有）覆盖同一断言，新测的增量仅在「经 env spec 解析出的档」这条路径上。留着不算错，登记重叠。

---

### LOW-5 — 科目分解脚本的 `tax_bps` 不进任何科目列（当前 =0 无数值影响）

**位置**：`scripts/fee_account_decomposition.py`（`decompose()` 只累 commission / regulatory / clearing / slippage；`ARM_R_TAX_BPS` 仅在表头文字里出现）

生产臂R 的合成率是 `(commission + slippage + tax)/1e4`（`treasury::fee_rate`），脚本的 Σ 只有 commission+slippage。当前 `tax_bps = 0.0` ⟹ 数值恰好相等，表对得上（我独立复跑逐位复现了 §4.1 六行）。但若 `config.rs` 的 `tax_bps` 改非零，本表会**静默少算**且不落任何科目列——比报告 §4.1 已自登记的「三常数手抄漂移」多一层：手抄漂移至少表头会打出旧数，tax 则连列都没有。

**修法**：`Decomposition` 增一列（或把 tax 并入 `regulatory` 并注明），使 `total` 与 `treasury::fee_rate` 的合成口径同构。

---

## 两轴分述

### Standards 轴（090 / v3 硬禁令 / #374 防线纪律）

| 项 | 结论 |
|---|---|
| **#374 fail-loud 防线是否削弱** | **未削弱**。5 个 `RunResult::fee_rate` 消费点全部 `expect(treasury::SCALAR_COST_RATE_UNDEFINED)`，消息串**单一来源**（`treasury.rs:57`），`grep -rn '\.fee_rate' rust/src` 无第二查法、无 `unwrap_or`/`unwrap_or_default` 兜底。原契约测试 `scalar_cost_rate_calibrated_panics` 原样保留通过。位点从构造期迁到消费期已在报告 §3 明文披露。 |
| **是否出现「标定档静默产出 metrics 读数」的新路径** | **未出现**（穷查见下节重点核查①）。 |
| **声明膨胀（090）** | **一处**：MED-1（降级理由与本臂档位不符）+ LOW-1（证伪表述越域）。其余处置反而偏严：报告主动披露了「`scalar_cost_rate` 已无生产调用点」「不变量口径收窄」「n_orders 差异只是描述不是机制证明」「①/② 份额未拆」四条对自己不利的事实，未粉饰。 |
| **v3 硬禁令（不作 alpha 论据/策略择优）** | **遵守**。三窗产物均带 A10 附则B 强制标签；层4 在标定档改述为「本臂无结论」，明写「既不宣称 confirmed alpha，也不判 INCONCLUSIVE」，并在声明块里对抬头旧措辞作了更正（`/tmp/388_armD_report_p3fold.md:17`）。这一处理是本票最扎实的部分。 |
| **231号有效域纪律** | 认识论等级逐节标注（臂D=L2、OKLO=L1、新增单测 L0/L1）；有效域节明确不外推。扣分项 = LOW-1。 |
| **no-patch-mentality** | 未见 workaround：标定档不喂近似费率、不用 R 的正负顶替三态、落盘失败 `panic` 不 `.ok()` 吞错、非法 env 全部 fail-loud（格式/缺文件/未在册/跨品种四条各有测试）。 |
| **coding-style（800/50 行线）** | `fill.rs` 4040 行、新增测试 158 行超线，已在 §12.1 登记为不修（#385 Out of Scope 明列 `fill.rs` 拆分）。判断合理。 |

### Spec 轴（#388 七条 + 母 SPEC #385 对齐）

| # | 验收标准 | 结论 | 独立证据 |
|---|---|---|---|
| 1 | 臂D 三窗跑批完成，datum 来自仓内文件，产物带 `[L2费率标定: datum <前12位>]` | **PASS** | `shasum -a 256` 实算 = sidecar = `cdd23adef9b3…`；三窗产物第 7 行标签逐窗齐备且一致 |
| 2 | 费率科目分解表落盘，臂D-vs-臂R 差异解释到科目 | **PASS** | 独立复跑 `fee_account_decomposition.py` 六行**逐位复现** §4.1；单笔手算复核通过（见重点核查⑥）。份额未拆已诚实登记 |
| 3 | OKLO 真实窗单跑（per-share 读数落盘） | **PASS（按 #388 票体口径）** | 三组读数算术自洽（见重点核查⑤）；母 SPEC US10「进 treasury 层」未达，照实登记，承接票 **#419 已挂** |
| 4 | metrics 收窄声明遵守，无静默越域 | **PASS**（理由措辞见 MED-1） | 标定档下 `significance` 根本不进（`wverify_run.rs:1430` 的 `.map` 短路）；`pi_bsp_timing` 确不在 m8 路径（独立 bin） |
| 5 | 不变量清单逐窗断言全绿 | **部分达成，已诚实收窄** | 臂D 三窗**逐窗断言**只有 2 条（R 分解守恒、`W_T ≤ notional_in`）；NEST_GATE_STATS 逐窗**读数**两臂逐字段相同（我独立 grep 三窗六份 `.out` 核对，全等）；OQ-9 / dual_ledger / floor 只在 default 档全量单测里绿。§7.4 明文写清这一口径，且与 T1（#401 判 PASS）同口径 ⟹ 不另计扣分 |
| 6 | 命令/env/落盘路径记录 | **PASS** | §1 表齐全；我按其命令原样复跑回归门与分解脚本，均可复现 |
| 7 | 基线不劣化（1928/1 → 1937/1） | **PASS** | `/tmp/388_full_lib_test.log:2520` = `1937 passed; 1 failed; 135 ignored`；新增测试数 2(treasury)+7(wverify_run)=9 非 ignore + 1 ignore(OKLO) 与 +9/+1 逐数对齐；唯一失败 `extract_signals_bit_exact_digest_guard` = #115 线在案 |

**母 SPEC #385 对齐**：US3 达成（层4 缺失见 MED-1）、US4 达成（OKLO 产物标签见 LOW-2）、US7 达成、US9 达成、US10 **未达并已按票体降级 + #419 承接**、US6/US11 部分（同验收标准 5 的口径）。US1/US2/US5/US8/US12 属 T1/T3 面，不在本票。

---

## 重点核查（票体六条，逐条结论 + 证据）

### ① `fee_rate → Option<f64>`「防线未削弱」——5 个消费点 + 穷查新路径

**结论：防线未削弱；未发现「标定档静默产出 metrics 读数」的新路径。**

消费点穷举（`grep -rn '\.fee_rate' rust/src`，剔除 `exec.fee_rate`/`config.fee_rate`）共 **8 处**：

| 位置 | 处置 |
|---|---|
| `l3_delta_r_alpha.rs:868`（`rebuild_cost_series`） | `expect(SCALAR_COST_RATE_UNDEFINED)` |
| `l3_fullwindow.rs:137` | 同 |
| `l3_pi_falsify.rs:154` | 同 |
| `runner.rs:6963 / 7040 / 7192`（3 个测试） | 同（全限定路径，同一常量） |
| `wverify_run.rs:1430` | `.map(...)` ⟹ **报告层显式降级**（不产数，标不可用） |

消息串单一来源 `treasury.rs:57`，**无第二查法**；无 `unwrap_or*` 兜底；`RunResult` **无 serde derive**（`runner.rs:106` 只 `#[derive(Debug, Clone)]`）⟹ 不存在「`None` 被序列化成 `null` 后被下游当 0 读」的路径。

「标定档静默产出 metrics 读数」的新路径穷查（构造期 panic 移除后，标定档第一次能跑完整条 pipeline，所以必须查）：

- **`metrics::significance`**：标定档下 `wverify_run.rs:1430` 的 `.map` 使其**根本不被调用**（不是喂近似值）——这是唯一在 m8 路径上的调用点。其余调用点（`l3_*`、`runner` 测试、两个 `src/bin/`）均在 default 档运行。
- **`random_entry_controls`**：只被 `significance` 内部调用（`metrics.rs:356`），随之不可达。
- **`rebuild_cost_series`**：只被 `l3_delta_r_alpha.rs:1045-1046` 调用，不在 m8 路径；且已 `expect` fail-loud。
- **三阶段 TW / dual_ledger 会不会偷偷用未标定 3bp**：**不会**。`strategy/ledger.rs` 内 `grep fee|commission` **零命中**（TW 只消费已扣费的现金流）；`dual_ledger::apply_fill_dual` 的 `fee_rate` 参数全部来自 `fill.rs` 的 `leg_fee_rate/order_fee_rate` → `treasury::fee_quoter`（标定档逐笔解析）。`treasury::settle / net_realized / clears_cost`（收 `fee_rate: f64`）在 `rust/src` 内**无生产调用点**，只有自身单测。
- **`econ_positive` / `mu_estimator` / `bin::pi_bsp_timing` / `bin::pure_bsp_timing`**：各自带 PnL 引擎，不读 `RunResult::fee_rate`，且不在 m8 路径（`config.rs:290-294` 已登记为另票）。#377 LOW 的 `pi_bsp_timing.rs:621` 手抄费率**确未触达**。

**副产品（已在报告 §3 自曝，我复核为真）**：`treasury::scalar_cost_rate` 改造后**无生产调用点**——`grep` 只剩自身契约测试与文档链接。防线现在**全部由 5 个消费点的 `expect` 承载**。报告没含糊其辞，措辞是「这不是『防线在原处』——是防线换了承载位点」。这一条是本票 Standards 轴上的加分项。

### ② M8_FEE_DATUM 的 None 档逐位不变

**结论：实证成立。**

```
$ python3 scripts/check_armR_trades_digest.py
p3fold: n_trades=149 digest=0x6bf47daf0aa737cd bytes=206057 ✓
wf7: n_trades=196 digest=0x282c28ca8ca65e16 bytes=270684 ✓
wf8: n_trades=168 digest=0xcafa7c4846c762cd bytes=228108 ✓
臂R trades 逐位无漂移。
exit=0                                    ← 我独立复跑，非采信报告
```

另独立复核 T1 产物保全声明：`/tmp/m8_win_gate/{p3fold,wf7,wf8}/{trades,tower_events}.jsonl` 与 `/tmp/m8_win_gate_t1_backup/` 对应六份 `cmp` **全部 byte-identical**。

钩子代码路径（`wverify_run.rs`）：`apply_m8_fee_datum_from_env` = `if let Ok(spec) = std::env::var("M8_FEE_DATUM") { ... }` ⟹ 未设即完全 no-op，`cfg` 逐位 default。注入点两处（`plain_cfg` 供标签与 `load_by_symbol` 品种校验；循环内 `cfg` 供跑批），循环内 `cfg` 由 `ThetaConfig::default()` 新建后逐个 env-gate 施加，无双施加歧义。`M8_FEE_DATUM=""` 亦走 `Ok("")` → 格式断言 panic（fail-loud，不静默退回未标定档）。

报告层的 armR 渲染路径也确认逐字符不变：`"## 四层报告\n\n"` 从原 `format!` 尾部拆出为独立 `push_str`，未标定档拼接结果相同；`layer4_cells` 的 `Some` 分支三分支渲染与降级前逐字符相同（`layer4_cells_uncalibrated_render_is_unchanged` 三分支全覆盖），表格 format 的占位符与实参数量前后均为 21，未错位。

### ③ 标签一致性

**结论：全中。**

```
$ shasum -a 256 analysis/data_cache/venue_fee_binance_spot_20260726.json
cdd23adef9b34b7a…   ← 前 12 位 = cdd23adef9b3
$ cat analysis/data_cache/venue_fee_binance_spot_20260726.json.sha256
cdd23adef9b34b7a…   ← sidecar 一致
$ grep -n '口径标签' /tmp/388_armD_report_{p3fold,wf7,wf8}.md
…:7:**口径标签：[L2费率标定: datum cdd23adef9b3]**（成交费率科目）／**[L1机制/费率未标定]**（cost 三常费率保底未标定）…
```

三窗齐备且逐窗相同；臂R 对照产物为 `[L1机制/费率未标定]`（标签随档升降正确）。IBKR datum 同法核对：实算 = sidecar = `cc0d3fa3695d…`，与 OKLO 产物 footer 一致。**唯一缺口 = LOW-2**（OKLO 产物抬头的占位符未填充）。

datum 文件与 sidecar **均已入仓**（`git ls-files analysis/data_cache/` 命中，`git check-ignore` 未命中）⟹ 4 个新增非 ignore 测试在干净 checkout 上可跑，不会因 datum 缺失变红。

### ④ 「无择时差」抽查——**三窗全部独立重算**（非抽查）

**结论：逐位成立。**

直接读两臂 `trades.jsonl` 独立重算（不采信报告任何数字）：

| 窗 | 笔数 D/R | `(entry_bar, exit_bar)` 相同 | `units` 相同 | Σ`pnl_raw_unlevered` D | Σ R | 逐笔 pnl 全等 |
|---|---|---|---|---|---|---|
| p3fold | 149/149 | **149（全同）** | 45 | -11847.799999999988 | -11847.799999999988 | **True** |
| wf7 | 196/196 | **196（全同）** | 50 | -17511.140000000036 | -17511.140000000036 | **True** |
| wf8 | 168/168 | **168（全同）** | 42 | +16218.530000000072 | +16218.530000000072 | **True** |

比报告声明**更强**的一条：费前 `pnl_raw_unlevered` 不只 Σ 相等，而是**逐笔 f64 全等**。

配套佐证：`NEST_GATE_STATS` 三窗两臂**逐字段相同**（我从 6 份 `.out` grep 原始行对照，`total/admitted/rejected/nest_pass/xzd_pass/flat_dir/no_level/cert_none/nest_n_delta_false/xzd_gate_fail` 全等）。§2 表 18 列读数与 `/tmp/388_armD_report_*.md:31` / `/tmp/388_armR_report_*.md:21` 原始行**逐位相符**（含报告里省略的 Borrow/LiqLoss 两列，实为恒 0，省略声明属实）。

`n_orders` 窗间不一致（wf7 315 vs 304）已在 §4.2 登记为 ΔN 步数切分差，并自我限定「这是描述，不是机制证明」——措辞恰当，不算缺陷。

### ⑤ OKLO 降级登记

**结论：照实，无冒充；算术自洽；一处标签瑕疵（LOW-2）。**

- **降级表述**：报告 §11.2 与 §12.2 明写「本票交付的 OKLO 读数是**费用层**的，不是 treasury 三阶段层的，报告不冒充后者」，并给出技术原因（m8 数据集硬编码 `load_by_symbol("BTC")`）。#388 resolution 同口径。**承接票 #419 已挂**（「承接：OKLO 真实窗接入 treasury 三阶段层（US10 未达部分）」）⟹ 母 SPEC US10 的缺口闭环有主。措辞无冒充。
- **最低佣金 40/40 触达读数抽查**：`raw = 0.0035 × 50 = 0.175 < 0.35 ≤ cap` ⟹ 40 腿全触达；Σ佣金 `40 × 0.35 = 14.0000` ✓。100 股 `raw = 0.35` **恰等于**下限 ⟹ 判定式 `raw < 0.35` 为假、不计触达，Σ佣金仍为 14.0000（读数与「边界不算触达」的表述一致，未取巧）。500 股 `40 × 1.75 = 70.0000` ✓。
- **其余分项交叉验算**（对 `/tmp/388_oklo_per_share_readings.md` 三行逐项）：清算+CAT `(0.0002+0.000003) × 股数 × 40` = 0.406 / 0.812 / 4.06 ✓；TAF `0.000195 × 股数 × 20 卖出腿` = 0.195 / 0.390 / 1.950 ✓；SEC 与 pass-thru 三组严格按 1:2:10 与 1:1:5 缩放 ✓；四列相加 = Σ总费用（15.7916 / 17.5729 / 87.8645）✓；有效费率 = Σ费用/Σ名义，100 与 500 组**完全相同**（7.654387e-5）与「只有托底/封顶两个拐点破线性」的表述一致 ✓。
- **「独立重算」是否名副实**：**是**。测试内手写公式用一手常数，datum 路径经 `sched.fee_usd(...)`，两条路径逐笔 `1e-9` 相对容差对拍；另有「分项之和 = datum 总额」与「账本实付差 = 独立重算差」两条守恒断言。datum 若漂移，对拍即红（fail-loud），不是同义反复。
- **一处越域表述**：见 LOW-1。

### ⑥ 科目分解独立性

**结论：公式与 datum 数字一致；独立重算口径已在产物与脚本双处标注；一处口径缺列（LOW-5）。**

- **独立复跑**：`python3 scripts/fee_account_decomposition.py` 输出六行与报告 §4.1 **逐位相同**（exit=0）。
- **费率来源核对**：脚本 `binance_taker_bps()` 从仓内 datum 读 (BTC, VIP0) `taker_bps = 10.0` ✓；生产侧撮合角色恒 Taker ⟹ 取 `taker_bps` 是对的口径。臂R 三常数 1.0/2.0/0.0 与 `config.rs:301-303` `ExecConfig::default()` 逐字相符 ✓。滑点 addon 2bp 与 `treasury::fee_quoter` 的 `uncalibrated_addon = slippage_bps/1e4` 同源 ✓。
- **单笔手算复核**（p3fold 第 1 笔，两臂同笔）：`units = 599.8770778615658`，`entry_px = 16547.06`，`exit_px = 16562.45` ⟹ 双腿名义额 `19861636.108228292`。臂D：commission `@10bp = 19861.636108228293`、slippage `@2bp = 3972.3272216456585`、合计 `@12bp = 23833.96332987395`；臂R：commission `@1bp = 1986.1636108228292`、合计 `@3bp = 5958.4908324684875`。与脚本 `decompose()` 的公式（`total × bps/1e4`）逐位同构 ✓。
- **「独立重算」口径标注**：脚本 docstring、产物 `/tmp/388_fee_decomp.md` 末尾、报告 §4.1 三处均写明「非账本实扣分项，账本 `Comm+Slip` 口径更宽，两者 Σ 不应相等」。我另做了一致性校验：脚本 Σ / 账本 Comm+Slip = 0.9320（臂D p3fold）与 0.9314（臂R p3fold），两臂比值近乎相同 ⟹ 与「口径更宽但同比」的解释自洽，未见隐藏差错。
- **缺口**：`tax_bps` 不进任何科目列（LOW-5）；臂R 三常数手抄漂移风险已由交付方自登记，我确认防线（表头每次打印三数与来源行号）确实存在。

---

## 附：本次评审实际执行的独立验证命令

```
git show d7d3451c64 --stat / -- <每个文件>          # 全 diff 通读
shasum -a 256 analysis/data_cache/venue_fee_*.json  # ×2，对 sidecar
python3 scripts/check_armR_trades_digest.py          # exit=0
python3 scripts/fee_account_decomposition.py         # 六行逐位复现 §4.1
python3 <逐笔 trades.jsonl 重算脚本>                  # 三窗择时/units/费前 pnl（逐笔 f64 全等）
cmp /tmp/m8_win_gate/*/*.jsonl /tmp/m8_win_gate_t1_backup/*/*.jsonl   # 六份全同
grep NEST_GATE_STATS /tmp/388_arm{D,R}_*.out         # 三窗两臂逐字段对照
grep -rn '\.fee_rate|scalar_cost_rate|fee_quoter|significance\(' rust/src   # 消费点穷举
diff /tmp/kimi-nest-mainline/<11 文件> /tmp/nc-review-388/<同>          # 跑批工位 vs 交付 commit
git ls-files analysis/data_cache/ ; git check-ignore …                 # datum 入仓性
```

**未执行**：`cargo test --release --lib` 独立复跑。评审工位无 `rust/target`（全量构建会在只读评审工位产生 8.8G 构建产物，越出「零 git mutation / 只读验证」的派工边界）。替代证据 = `/tmp/388_full_lib_test.log` 逐行核对（`grep -c '^test .* ok$'` = 1937，与 `test result` 行自洽），且已确认交付 commit 的 `rust/src` 与产出该日志的工位 byte-identical（唯一差异文件为 T3 #389 事后新增）。这条限制照实登记，不冒充「已独立复跑」。
