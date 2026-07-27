# treasury 重验 T2：标定臂（臂D 真实费率）+ OKLO 真实窗

- **日期**：2026-07-27　**票据**：issue #388（母 SPEC #385，map #59 Destination 末项；上游 #387 = T1）
- **工位**：worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`。
  **跑批基线 HEAD = `0dc5bc785d`**（本票全部跑批、代码改造、评审固定点均以它为准）。
  **截至本报告落盘时**（2026-07-27，工作区基线 HEAD = `0dc5bc785d`）交付为**尚未提交的工作区改动**，
  零 git mutation；主仓 `/Users/silencehan/Projects/NewChanlun` 全程只读。
  **订正（#422 / #418 LOW-3）**：本报告其后已随 commit `d7d3451c64` 入仓，上句为**落盘当时**的历史陈述，
  不描述当前工作区状态。
  **并行工位登记（照实）**：本票跑批期间，同一 worktree 上有另一工位提交了 `9e3cd25fd6`
  （`docs(chanlun): 影子评审报告入库——#387 T1（#401）`，**纯文档、`rust/src` 零改动**）⟹
  跑批基线不受影响；同时 `chanlun/review-results/treasury-reverify-t1-armR-20260727.md` 的
  未提交改动（15 处 `#408 订正` 标记）**属该工位，不是本票产物**——本票未碰 T1 报告与其 golden。
- **性质**：treasury 验收**第一次建立在真实费率上**的新基线。臂D = 臂R + `ExecConfig.fee_schedule=Some(datum)`
  （BTC = Binance spot VIP0，datum 取自仓内文件，路径即契约）。
- **口径标签**：臂D 产物 `[L2费率标定: datum cdd23adef9b3]`（成交费率科目）／`[L1机制/费率未标定]`
  （cost 三常费率保底）。**认识论 L2**（真实 BTC OOS 三窗假设检验，可否证）。OKLO 节为 **L1**
  （费用算术 + 触达计数）。数值**不作 alpha 论据、不作策略择优输入**（v3 硬禁令 / A10 附则B）。
- **上游锚点**：臂R 产物与 digest golden 由 #387（commit `0dc5bc785d`）冻结；本票复跑臂R 三窗，
  `scripts/check_armR_trades_digest.py` **exit=0**（三窗 digest 全中，见 §6）。

---

## 0. 结论摘要

1. **臂D 三窗跑通**，产物带 `[L2费率标定: datum cdd23adef9b3]` 标签（§2）。这是 treasury 验收
   首次在 venue 标定费率下产出的读数。
2. **臂D 与臂R 的交易择时逐笔相同，只有仓位规模不同**（§4.2 实证：三窗 149/196/168 笔的
   `(entry_bar, exit_bar)` **全等**，费前 `Σpnl_raw` **逐位相等**；`units` 有约 2/3 的笔不同）。
   ⟹ 费率不进信号/门控，只进 sizing 与账本 —— 臂间差异可干净分解为
   **费用科目差 + 沿同一价格路径的规模差**，不掺择时差。
3. **报告层 honest 降级已落地**（§3）：标定档下 `LCB_OOS(R)`、三态判据、`significance` 随机对照系
   一律标 `不可用(标定档有效域收窄 #374)`，判据结算段的层4 改述为「本臂无结论」。
   **订正（#422 / #418 MED-1）**：该降级是**按档位类型一刀切的保守收窄**，**不是**「本臂无良定义」——
   无良定义的是**按股档（per-share）**；本臂是**按金额档（per-notional，恒 12bp）**，标量其实可定义（§3 订正块）。
   **未绕过、未删除、未削弱 #374 的 fail-loud 防线**——改的是承载方式（构造期 panic → 类型 `Option`
   + 消费点 `expect` / 报告层标注），消息串单一来源 `treasury::SCALAR_COST_RATE_UNDEFINED`。
4. **OKLO 真实窗 per-share 读数**（§5）：50 股/腿组**最低佣金 $0.35 托底 40/40 全触达**，有效费率
   1.3757e-4；100/500 股组不触达，有效费率 7.6544e-5。**同一档位下有效费率随单量变化 1.8 倍**
   ——这是 #374「单标量费率在 per-share 档无良定义」的经验证据面，不再只是定义层论证。
5. **不变量：臂D 三窗跑批内断言全绿**（R 分解守恒 + `W_T ≤ notional_in`），其余机器件
   （OQ-9 / dual_ledger R=Π−A−W / stage 单向 / floor 保守方向）随全量单测在 **default 档**通过
   （§7.4 措辞已按此收窄，不冒充「臂D 三窗上逐窗断言」）；`cargo test --release --lib` = **1937 passed / 1 failed**
   （唯一失败 = `extract_signals_bit_exact_digest_guard`，#115 线在案），相对票面基线 1928/1
   的 +9 恰为本票新增测试数（§8）。

---

## 1. 跑批命令 / env / 落盘路径

工作目录一律 `/tmp/kimi-nest-mainline/rust`。

| 用途 | 命令 | 落盘 |
|---|---|---|
| **臂D**（标定，逐窗 tag ∈ {p3fold,wf7,wf8}） | `M8_WIN_FILTER=<tag> VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 M8_FEE_DATUM=venue_fee_binance_spot_20260726.json:BTC:VIP0 OPSEM_DUMP_DIR=/tmp/m8_win_armD/<tag> cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture` | `/tmp/388_armD_<tag>.out`、`/tmp/m8_win_armD/<tag>/{trades,tower_events}.jsonl`、报告副本 `/tmp/388_armD_report_<tag>.md` |
| **臂R**（回归门复跑，同 T1 命令） | `M8_WIN_FILTER=<tag> VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 OPSEM_DUMP_DIR=/tmp/m8_win_gate/<tag> cargo test …（同上）` | `/tmp/388_armR_<tag>.out`、`/tmp/m8_win_gate/<tag>/*.jsonl`、`/tmp/388_armR_report_<tag>.md` |
| **回归门校验** | `python3 scripts/check_armR_trades_digest.py` | stdout（exit=0） |
| **费率科目分解** | `python3 scripts/fee_account_decomposition.py --out /tmp/388_fee_decomp.md` | `/tmp/388_fee_decomp.md` |
| **OKLO 真实窗** | `cargo test --release --lib oklo_real_window_per_share_readings -- --ignored --nocapture` | `/tmp/388_oklo_per_share_readings.md` |
| **#360 对拍复跑** | `cargo test --release --lib real_window_datum_reconciliation -- --ignored --nocapture` | stdout（BTC/OKLO 两窗账本差 = 重算差，逐位） |
| **基线全量** | `cargo test --release --lib` | `/tmp/388_full_lib_test.log` |
| 四层报告（每次跑批覆写） | 测试体内部 | `/tmp/m8_e2e_all_systems_oos.md` |

**分解表的稳定落盘位置 = 本报告 §4.1**（仓内文件，T3 直接引用）；`/tmp/388_fee_decomp.md` 只是
脚本副本（易失），生成器 `scripts/fee_account_decomposition.py` 已入仓可复跑。

**T1 产物保全**：臂R 复跑前把 #387 的 `/tmp/m8_win_gate` 整目录备份到 `/tmp/m8_win_gate_t1_backup`，
复跑后六个 dump（三窗 × {trades, tower_events}）与备份 **`cmp` 逐字节相同**。

**datum 来源（路径即契约）**：`analysis/data_cache/venue_fee_binance_spot_20260726.json`
（BTC/VIP0，maker=taker=10bp）+ `.sha256` sidecar，装载经 `venue_fee::load_datum` 哈希校验。
`M8_FEE_DATUM` 未设 ⟹ 注入 no-op（臂R 逐位不变，§6 实证）。

---

## 2. 臂D 三窗读数（四层报告表，含臂R 对照）

| 窗 | 臂 | n_orders | ΣN_tΔP_t | Comm+Slip | Funding | net_r(execR) | MaxDD | 声部数(A/S/F) | 终Stage | Q_T | W_T | η_T/η_* | cum_holding_cost | η_corrected | R(含浮盈) | LCB_OOS(R) | 三态 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| p3fold | **D** | 222 | -768177 | **791981** | 70729 | **-1630887** | 0.1591 | 93/109/108 | I(降成本) | 16543670 | 0 | 12038659/16543670 | 70728 | 11967931 | -1564400 | **不可用** | **不可用** |
| p3fold | R | 222 | -971478 | 219149 | 77924 | -1268551 | 0.1475 | 93/109/108 | I(降成本) | 16543670 | 0 | 14564077/16543670 | 77923 | 14486154 | -1191916 | -3550861 | 无(R≤0) |
| wf7 | **D** | 315 | -4317711 | **1793600** | 252494 | **-6363805** | 0.2915 | 109/119/141 | I(降成本) | 23471260 | 0 | 17237542/23471260 | 252493 | 16985049 | -6113276 | **不可用** | **不可用** |
| wf7 | R | 304 | -4650486 | 496332 | 280463 | -5427281 | 0.2601 | 109/119/141 | I(降成本) | 23471260 | 0 | 20774370/23471260 | 280463 | 20493907 | -5147413 | -10865720 | 无(R≤0) |
| wf8 | **D** | 260 | +7284932 | **2286097** | 494344 | **+4504490** | 0.0947 | 116/101/132 | I(降成本) | 28721770 | 0 | 33802912/28721770 | 494344 | 33308568 | +4980320 | **不可用** | **不可用** |
| wf8 | R | 260 | +7450186 | 594120 | 509091 | +6346975 | 0.0803 | 116/101/132 | I(降成本) | 28721770 | 0 | 36982765/28721770 | 509090 | 36473675 | +6851062 | -2105181 | INCONCLUSIVE |

「不可用」全称 = `不可用(标定档有效域收窄 #374)`（产物原文）。Borrow / LiqLoss 两列两臂三窗恒 0，略。

**口径标签证据**（三窗臂D 报告首行原文，逐窗相同）：

```
**口径标签：[L2费率标定: datum cdd23adef9b3]**（成交费率科目）／**[L1机制/费率未标定]**（cost 三常费率保底未标定）
```

**臂D 相对臂R 的读数方向**（照实登记，不作策略判断）：Comm+Slip 三窗均**约 3.6～3.9 倍**
（219149→791981 / 496332→1793600 / 594120→2286097），execR 三窗均更低；MaxDD 三窗均略升；
终Stage / Q_T / W_T / 声部数三窗**完全不变**。

---

## 3. metrics / l3 收窄声明的遵守（无静默越域）

标定档下**不可用**（产物内逐窗标注，并在报告头以引用块整段声明）：

| 读数 | 处置 | 依据 |
|---|---|---|
| `LCB_OOS(R)`（block bootstrap） | 表格单元标 `不可用(标定档有效域收窄 #374)` | 成本口径要一个单标量费率，而实现**按 `fee_schedule.is_some()` 一刀切**撤下（**订正（#422）**，见表下） |
| 三态判据 | 同上；**不用 R 的正负降格顶替** | 判据是 `LCB>0`，缺 LCB 即无判据 |
| `metrics::significance` 随机对照系（shift / indep p 值、`theta_beats_random`） | 标定档下**根本不算**（不进 `significance`） | #374：反事实臂上任何取自实际路径的标量都是错的（同受一刀切覆盖，**订正（#422）**） |
| `l3_delta_r_alpha::rebuild_cost_series` 鞅守卫成本剥离 | 不在 m8 路径内；消费点 `expect(SCALAR_COST_RATE_UNDEFINED)` fail-loud | #374 原文档已登记解锁路径 |
| 判据结算段「层4」 | 标定档改述为**「本臂无结论」**，明写「既不宣称 confirmed alpha，也不判 INCONCLUSIVE」 | 措辞§5.6 的 INCONCLUSIVE 是「有 LCB 且 ≤0」的态 |

> **订正（#422 / #418 MED-1）——撤下读数的理由，不是「本臂无良定义」**：
> 单标量费率**无良定义**的是 **per-share（按股）档**（IBKR 那类：最低佣金托底 / 1% 上限 / TAF 封顶
> 使有效费率随单量变动，§5 实测 1.8 倍）。**本臂用的是 per-notional（按金额）档**
> （Binance 现货 `unit=notional`，maker=taker=10bp，`FeeUnit::Notional` 分支不依赖 (qty, px)，
> 且本引擎流动性角色恒 `Taker`）⟹ 逐笔单边有效费率**恒 = 12bp**（10bp datum + 2bp 未标定滑点 addon），
> **单标量在本臂其实有良定义**。
> ⟹ 上表层4 读数的缺席是**实现按档位类型（`fee_schedule.is_some()`）一刀切的保守收窄**，
> **不是**「本臂无良定义」。方向是保守的（撤下读数不会造出假 alpha，不违反 v3），但**理由与本臂档位不符**，
> 090 要求明写。
> **本次只订正措辞，不恢复任何读数**——「不可用」诸格维持原状；按金额档层4 的恢复属 **#423**
> （按档位形态分叉 scalar 成本口径），在其落地前本节状态不变。

**仍然有效**（与单标量费率无关；逐笔实付经 `treasury::fee_quoter` 解析）：n_orders / ΣN_tΔP_t /
Comm+Slip / Funding / Borrow / LiqLoss / execR / MaxDD / 声部数 / 终Stage / Q_T / W_T / η 列 /
R(含浮盈) / `NEST_GATE_STATS`。

**`pi_bsp_timing` 未触达**（票体要求先核）：臂D 的跑批缝是 `wverify_run::m8_e2e_all_systems_oos`
→ `runner::run_theta_v0_pi_overlay`，**不经** `src/bin/pi_bsp_timing.rs`（独立 bin，自带手抄费率，
#377 LOW 在案）。本票不改它（票体明文不要求）。

**防线未削弱的实证**（含一条须披露的结构变化）：`treasury::scalar_cost_rate` 的标定档 panic 契约测试
（`scalar_cost_rate_calibrated_panics`，expect「无良定义」）**原样通过**；新增
`scalar_cost_rate_opt_calibrated_is_none` 覆盖类型承载版。5 个消费点（runner ×3、l3_pi_falsify、
l3_fullwindow、l3_delta_r_alpha）全部改为 `expect(SCALAR_COST_RATE_UNDEFINED)` —— 同一条消息串，
fail-loud 语义不变，只是位点从**构造期**移到**消费期**。

**须披露的结构变化**（评审 Standards/Spec 两轴同时指出）：改造后 `treasury::scalar_cost_rate`
**已无生产调用点**（只剩自身契约测试与文档链接）——#374 防线现在**全部由 5 个消费点的 `expect`
承载**，`scalar_cost_rate` 退为「需要标量且不接受缺席」的门面 + 契约见证。保留它而不删的理由：
它是 #374 那条裁定的可执行表述（删掉则该裁定只剩注释），且是新增 `scalar_cost_rate_opt` 的语义锚。
**这不是「防线在原处」——是防线换了承载位点**，报告不含糊其辞。

---

## 4. 费率科目分解与臂间差异归因

### 4.1 科目分解表（离线独立重算）

`python3 scripts/fee_account_decomposition.py`（臂D 费率从仓内 datum 读；臂R 三常数为脚本内常量，见下方费率来源条）：

| 窗 | 臂 | 成交腿数 | Σ名义额 | commission | 监管 | 清算 | slippage | Σ费用 |
|---|---|---|---|---|---|---|---|---|
| p3fold | D(标定) | 298 | 615050128.62 | 615050.13 | 0.00 | 0.00 | 123010.03 | 738060.15 |
| p3fold | R(未标定) | 298 | 680415371.65 | 68041.54 | 0.00 | 0.00 | 136083.07 | 204124.61 |
| wf7 | D(标定) | 392 | 1399495095.94 | 1399495.10 | 0.00 | 0.00 | 279899.02 | 1679394.12 |
| wf7 | R(未标定) | 392 | 1575995194.19 | 157599.52 | 0.00 | 0.00 | 315199.04 | 472798.56 |
| wf8 | D(标定) | 336 | 1883048838.10 | 1883048.84 | 0.00 | 0.00 | 376609.77 | 2259658.61 |
| wf8 | R(未标定) | 336 | 1958219343.84 | 195821.93 | 0.00 | 0.00 | 391643.87 | 587465.80 |

- **口径（诚实边界）**：这是基于 `trades.jsonl`（费前口径，`opsem_dump.rs:233-234`）的**独立重算**，
  覆盖**已平仓声部的双腿**，**不是账本实扣分项**——生产侧 `RDecomposition` 只有合并科目
  `commission_slippage`，无逐科目拆分。账本 `Comm+Slip` 列口径更宽（含减仓腿、强平腿、窗末未平腿），
  故本表 Σ 与 §2 的 Comm+Slip 列**不应相等**（例：p3fold 臂D 738060 vs 账本 791981）。
- **监管/清算恒 0 不是漏算**：Binance 现货 datum 无此两科目，三常数档同样无（IBKR per-share 档才有，
  见 §5）。
- **费率来源**：臂D commission = 10.0bp **从 datum 文件读**（`venue_fee_binance_spot_20260726.json`
  / BTC / VIP0 taker，不手抄）；臂R commission = 1.0bp、tax = 0.0bp、slippage = 2.0bp 在脚本内是
  **常量（手抄自 `config.rs:301-303`）**——三常数档按定义**没有 datum**（未标定就是它的性质），
  无文件可读。**代价照实**：Rust 侧改默认值而未同步脚本 ⟹ 本表静默漂移；防线只有「表头每次运行
  打印这三个数与来源行号，复核时人工对一眼」。这与 #377 LOW 登记的 `pi_bsp_timing.rs:621` 手抄
  是同一模式，登记在案。
- **本脚本的适用范围**：只实现 **per-notional** 形态的分解；per-share 形态（IBKR）的逐笔分解在
  §5 的 Rust 缝内做，两者不通用。

### 4.2 臂间差异归因：科目差 + 规模差，**无择时差**

逐笔比对 `/tmp/m8_win_armD/<tag>/trades.jsonl` ↔ `/tmp/m8_win_gate/<tag>/trades.jsonl`：

| 窗 | 笔数(D/R) | `(entry_bar, exit_bar)` 相同笔数 | `units` 相同笔数 | Σ`pnl_raw_unlevered`(D) | Σ(R) |
|---|---|---|---|---|---|
| p3fold | 149/149 | **149（全同）** | 45 | -11847.80 | **-11847.80** |
| wf7 | 196/196 | **196（全同）** | 50 | -17511.14 | **-17511.14** |
| wf8 | 168/168 | **168（全同）** | 42 | +16218.53 | **+16218.53** |

⟹ **费率不进信号与门控**（择时逐笔相同、`NEST_GATE_STATS` 三窗两臂逐字段相同，§7.3），
**只进 sizing 与账本**（约 2/3 的笔仓位规模改变，Σ名义额 p3fold 680M→615M）。故臂D 相对臂R 的
execR 差 = **① 费率科目差**（同名义额下 commission 1bp→10bp）**＋ ② 沿同一价格路径的规模差**
（名义额下降 ⟹ 费用与 PnL 同比缩放）。费前 Σpnl_raw 逐位相等是「② 只改规模不改路径」的证据。

**`n_orders` 的窗间不一致（照实登记）**：wf7 臂D `n_orders=315` vs 臂R `304`（p3fold 与 wf8 两臂
相同，均 222 / 260）。`n_orders` = `fill.n_orders` = **ΔN 非零步数**，不是声部平仓笔数（后者三窗两臂
全同，见上表）。规模改变会改变目标仓位轨迹的**步数切分**（同一次进出可能拆成不同的加/减仓步数），
故 n_orders 可以在择时完全相同的情况下变动。**这是描述，不是机制证明**——本票未做逐步 ΔN 对拍，
不宣称已归因到具体步。

**未做的事（诚实缺席）**：本票**未**把 ①/② 两项拆成两个数（那需要第三个臂：标定费率但冻结臂R 的
sizing 轨迹，属新配置面，超出票面范围）。故 §4.1 的表回答「费用落在哪个科目」，§4.2 回答
「差异的两个来源分别是什么性质」，**不宣称**「execR 差的 X% 归费用、Y% 归规模」。

---

## 5. OKLO 真实窗（IBKR Pro Tiered per-share 档）

缝 = `fill::tests::oklo_real_window_per_share_readings`（新增 `#[ignore]` 缝，仿
`real_window_datum_reconciliation` 同款构造：`data::load_by_symbol("OKLO")` → 末 2000 根可交易 bar →
确定性订单流「每 100 bar 买入、其后第 50 bar 平仓」→ datum 注入 → 独立重算逐笔对拍）。

| 每单股数 | 成交腿数 | Σ佣金 | Σpass-thru | Σ清算+CAT | Σ卖出SEC | Σ卖出TAF | Σ总费用 | 最低佣金触达 | 1%上限触达 | TAF上限触达 | 有效费率 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 50 | 40 | 14.0000 | 0.010360 | 0.4060 | 1.1803 | 0.1950 | 15.7916 | **40/40** | 0/40 | 0/40 | **1.375701e-4** |
| 100 | 40 | 14.0000 | 0.010360 | 0.8120 | 2.3605 | 0.3900 | 17.5729 | 0/40 | 0/40 | 0/40 | 7.654387e-5 |
| 500 | 40 | 70.0000 | 0.051800 | 4.0600 | 11.8027 | 1.9500 | 87.8645 | 0/40 | 0/40 | 0/40 | 7.654387e-5 |

- datum：`venue_fee_ibkr_pro_20260726.json` / OKLO / `PRO_TIERED_LE_300K_SHARES`，sha256 前 12 位
  = `cc0d3fa3695d`（sidecar 已校验）。有效费率 = Σ总费用 / Σ名义额，**不含**未标定滑点 addon（2bp/腿）。
- **最低佣金触达**：50 股/腿组 `0.0035×50 = $0.175 < $0.35` ⟹ **40/40 全触达**；100 股恰 `$0.35`
  （边界，不算触达）；500 股 `$1.75` 不触达。**1% 名义额上限**与 **TAF $9.79 上限**三组均未触达。
- **监管费科目在本窗全部生效且非零**：卖出 SEC Section 31（1.1803 / 2.3605 / 11.8027）与 FINRA TAF
  （0.1950 / 0.3900 / 1.9500）逐组落盘 —— per-share 档的监管科目在 treasury 层第一次有真实读数。
- **对 #374 的证据价值**：同一档位、同一价格序列，有效费率因单量从 `7.65e-5` 变到 `1.38e-4`
  （**1.8 倍**）。这直接证伪「**在 per-share（按股）档**存在一个使随机对照/成本剥离成立的常数费率」这一前提。
  **订正（#422 / #418 LOW-1）**：原句写作「证伪『存在一个……常数费率』」，未带档位限定词，越出了本组证据的
  有效域——OKLO 证据只覆盖 per-share 形态；per-notional（按金额）形态存在反例，就是本报告自己的臂D
  （Binance 现货 `unit=notional`，maker=taker=10bp，撮合角色恒 Taker ⟹ 单边有效费率恒 12bp）。
  故本条**不构成**对「所有档位下常数费率不成立」的证伪（231号有效域纪律）。
  **不是单调关系**（照实）：100 股与 500 股有效费率**完全相同**（7.654387e-5）——per-share 佣金与
  按股监管费都与股数同比，只有**托底/封顶两个拐点**破坏线性。所以准确表述是「有效费率不是单量的
  常数」，不是「随单量单调下降」。
- 双路径独立：分项由报告 §2.3/§2.5 一手数字**手写公式**算，与 datum 对象 `fee_usd` 逐笔对拍
  （`1e-9` 相对容差）；另有「分项之和 = datum 总额」与「账本实付差 = 独立重算差」两条守恒断言。
- 既有 `real_window_datum_reconciliation` 复跑通过（BTC 账本差 = 重算差 = 2660.431032；
  OKLO = -5.385046，逐位）。

---

## 6. 回归门（臂R 逐位不变）

```
$ python3 scripts/check_armR_trades_digest.py
p3fold: n_trades=149 digest=0x6bf47daf0aa737cd bytes=206057 ✓
wf7: n_trades=196 digest=0x282c28ca8ca65e16 bytes=270684 ✓
wf8: n_trades=168 digest=0xcafa7c4846c762cd bytes=228108 ✓
臂R trades 逐位无漂移。
exit=0
```

- 三窗 digest 与 #387 golden **全中**；臂R `[m8]` 行读数与 T1 报告 §2.2 **逐位相同**
  （-1268551 / -5427281 / +6346975；LCB -3550861 / -10865720 / -2105181）。
- 六个 dump 与 `/tmp/m8_win_gate_t1_backup` **`cmp` 逐字节相同**。
- ⟹ 本票的代码改造（`Option<f64>` 承载 + 报告层降级 + `M8_FEE_DATUM` 钩子）在 **未标定档
  bit-exact 中性**，实证而非声明。

---

## 7. 不变量逐窗读数（臂D）

### 7.1 跑批内断言（`wverify_run.rs` m8 测试体，臂D 三窗全部通过）

| 不变量 | 断言 | 臂D 结果 |
|---|---|---|
| R 分解守恒（无资金泄漏） | `\|conservation_residual\| ≤ tol` | 三窗**全绿** |
| 退本金不超投入 | `W_T ≤ notional_in` | 三窗**全绿**（W_T 恒 0） |

### 7.2 treasury / TW 逐窗读数（臂D）

| 窗 | 终Stage | Q_T(notional_in) | W_T | η_T / η_* | cum_holding_cost | η_corrected（判读口径） |
|---|---|---|---|---|---|---|
| p3fold | I(降成本) | 16543670 | 0 | 12038659 / 16543670 | 70728 | 11967931 |
| wf7 | I(降成本) | 23471260 | 0 | 17237542 / 23471260 | 252493 | 16985049 |
| wf8 | I(降成本) | 28721770 | 0 | 33802912 / 28721770 | 494344 | 33308568 |

**stage 单向性**：三窗终 Stage 恒 `I(降成本)`、`W_T` 恒 0 ⟹ 无 Stage 回退可能（与臂R / v4 同向）。
**Q_T 三窗与臂R 逐位相同**（16543670 / 23471260 / 28721770）——`notional_in` 由 nav0 定，不受费率影响。

### 7.3 NEST_GATE_STATS 逐窗拒绝率（臂D，与臂R **逐字段相同**）

| 窗 | 臂D | 臂R（#387） | 判定 |
|---|---|---|---|
| p3fold | 1159/1633 = 71.0%（admitted=474 nest_pass=24 xzd_pass=450；rej: flat_dir=0 no_level=0 cert_none=115 nest_n_delta_false=117 xzd_gate_fail=927） | 同左 | **零差异** |
| wf7 | 1116/1724 = 64.7%（admitted=608 nest_pass=32 xzd_pass=576；rej: cert_none=56 nest_n_delta_false=100 xzd_gate_fail=960） | 同左 | **零差异** |
| wf8 | 1074/1518 = 70.7%（admitted=444 nest_pass=33 xzd_pass=411；rej: cert_none=87 nest_n_delta_false=34 xzd_gate_fail=953） | 同左 | **零差异** |

⟹ 门控行为对费率标定**完全不敏感**（费率不进入 admission 判定），与 §4.2 的择时不变互为佐证。

### 7.4 不变量机器件（全量 `cargo test --release --lib` 内，本票复跑全绿）

TW 守恒 / stage rank 单向 / OQ-9 gate / dual_ledger R=Π-A-W / treasury floor 保守方向 / R 分解守恒
——机器件清单同 T1 报告 §6.3（本票未改动其中任何一条，全部随基线复跑通过）。

**有效域（评审 Spec 轴指出，照实收窄）**：这些机器件跑在**默认配置**上，**不是**在臂D 三窗跑批内
逐窗断言的。臂D 三窗上真正逐窗断言的只有 §7.1 的两条（R 分解守恒、`W_T ≤ notional_in`）。
「不变量清单逐窗断言全绿」这句在本票的准确读法 = **§7.1 两条逐窗全绿 + §7.4 清单在 default 档全绿**，
承袭 T1 §6.3 的同一口径。要把 OQ-9/dual_ledger 也做成臂D 逐窗断言，需在 m8 测试体内新增断言点
（票面未要求，登记为后续可做项）。

---

## 8. 基线复跑

```
test result: FAILED. 1937 passed; 1 failed; 135 ignored; 0 measured; 0 filtered out; finished in 1.09s
failures:
    theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard
```

（日志 `/tmp/388_full_lib_test.log`）

- 票面基线 = 1928 passed / 1 failed / 134 ignored。本票**新增 9 个非 ignore 测试**
  （treasury ×2、wverify_run ×7）+ **1 个 `#[ignore]` 测试**（OKLO 真实窗）
  ⟹ 1928+9 = **1937 passed**、134+1 = **135 ignored**，逐数对齐。
- 唯一失败仍是 `extract_signals_bit_exact_digest_guard`（#115 线在案）——**未劣化**。

---

## 9. 代码改动清单（落盘当时未提交；**订正（#422 / #418 LOW-3）**：其后已随 commit `d7d3451c64` 入仓）

| 文件 | 改动 | 性质 |
|---|---|---|
| `rust/src/theta_v0/backtest/treasury.rs` | 新增 `scalar_cost_rate_opt` + `SCALAR_COST_RATE_UNDEFINED` 常量；`scalar_cost_rate` 改为 opt+`expect`（契约不变）；新增 2 测试 | 有效域承载方式 |
| `rust/src/theta_v0/backtest/runner.rs` | `RunResult::fee_rate: f64 → Option<f64>`（4 处构造改 `scalar_cost_rate_opt`，3 处消费改 `expect`）+ 文档 | 类型守门 |
| `rust/src/theta_v0/backtest/l3_pi_falsify.rs`、`l3_fullwindow.rs`、`l3_delta_r_alpha.rs` | 各 1 处消费点改 `expect(SCALAR_COST_RATE_UNDEFINED)` | 同上（fail-loud 位点移到消费期） |
| `rust/src/theta_v0/backtest/wverify_run.rs` | 新增 `parse_fee_datum_spec` / `apply_m8_fee_datum_from_env` / `layer4_cells` / `CALIBRATED_UNAVAILABLE`；m8 接线（cfg 注入 ×2、层4 降级、报告头声明块、层4 结算措辞分叉）+ 7 测试 | T2 主体 |
| `rust/src/theta_v0/backtest/fill.rs` | 新增 `#[ignore]` 测试 `oklo_real_window_per_share_readings` | OKLO 读数 |
| `scripts/fee_account_decomposition.py` | 新增（离线科目分解，读仓内 datum 不手抄费率） | 分解表机器件 |
| `chanlun/review-results/treasury-reverify-t2-armD-20260727.md` | 本报告 | 交付件 |

---

## 10. 有效域与边界（`formalization-validity-domain` 231号）

- **认识论 L2**：臂D 三窗为真实 BTC OOS 假设检验，可否证。OKLO 节为 **L1**（费用算术 + 触达计数，
  零市场信息增量）。
- **有效域**：BTC / 三窗（p3fold 2023-01-01..06-30、wf7 2023-02-17..08-16、wf8 2023-08-17..2024-02-16）/
  臂D（`fee_schedule=Some(Binance spot VIP0)`、`enforce_level_cap=false`、κ=0 冻结、
  `THETA_NEST_CERT_GATE=1`、`VOICE_EXEC=1`）。**不外推**其它品种/窗口/档位/开关组合。
  OKLO 节有效域 = OKLO 末 2000 bar × 三种单量 × 该确定性订单流，**不外推**到策略臂
  （OKLO 未接 treasury 跑批，见 §11.2）。
- **臂D 读数的层4 缺席是有效域收窄，不是「跑失败」**：LCB/三态无判据（§3）。任何后续引用**不得**
  把「不可用」读作「未过」或「INCONCLUSIVE」。
  **订正（#422 / #418 MED-1）**：该收窄的准确理由 = **实现按档位类型一刀切的保守收窄**，
  **不是**「本臂无良定义」——无良定义只成立于**按股档（per-share）**；本臂是**按金额档（per-notional，
  恒 12bp）**，单标量其实可定义。恢复本臂层4 读数属 **#423**，本次不恢复。
- **§4.2 的分解限度**：只区分「费用科目差 vs 规模差」的**性质**，不给两者的**份额**（§4.2 末尾）。
- 全部数值**不作 alpha 论据、不作策略择优输入**（v3 硬禁令）。

---

## 11. 与票体/母 SPEC 的冲突与偏离登记

1. **票体前提「m8 缝无注入钩子，需新写」成立**，但票体建议的 env 名与格式（`M8_FEE_DATUM=<file>:<symbol>:<tier>`）
   **按建议原样采用**，无偏离。
2. **OKLO「在 treasury 层有真实读数」的实现口径**：母 SPEC #385 User Story 10 写「OKLO 真实窗
   （IBKR 标定）纳入标定臂……per-share 档在 treasury 层有真实读数」，而 #388 票体明确降为
   「经 `real_window_datum_reconciliation` **同款构造单跑**」。**本票按 #388 票体执行**（票体优先，
   派工明文）——OKLO 走 `simulate_fills` 费用面，**未**经 `run_theta_v0_pi_overlay` 的 treasury 臂。
   **技术原因（照实）**：m8 跑批数据集硬编码 BTC（`wverify_run.rs` 内 `load_by_symbol("BTC")`），
   把 OKLO 接进 treasury 臂需要新的窗口预注册与数据面配置，属独立工作面。
   ⟹ **本票交付的 OKLO 读数是费用层的，不是 treasury 三阶段层的**，报告不冒充后者。
3. **`RunResult::fee_rate` 改 `Option<f64>` 是本票新增的语义改动面**，票体未逐字要求，但它是
   「标定档下 m8 报告层 honest 降级」的**必要前置**（原构造期 panic 使标定臂根本跑不起来），
   且派工明文「让类型系统替你守门」。改动最小化：不删不弱化 #374 防线，5 个消费点全部保留
   fail-loud（同一消息串）。
4. **`/implement` skill 的「Commit your work to the current branch」未执行**——派工硬约束
   「禁一切 git mutation，提交由编排者负责」优先。
5. **费率科目分解的「监管/清算」两列在本票恒 0**：两个在册档位（Binance 现货、三常数）均无此科目。
   非零读数只在 OKLO per-share 档出现（§5），已逐项落盘。

---

## 12. 交付件自审（`/code-review` 两轴，并行子代理）

固定点 = `0dc5bc785d`（**自审执行当时**其后无 commit，被审面 = 当时工作区的未提交改动 + 两个未跟踪新文件）。

**订正（#422 / #418 LOW-3）**：本报告落盘后的实际提交链为
`0dc5bc785d → 9e3cd25fd6 → 78151e988e → d7d3451c64`（本报告随末者入仓）。中间两个 commit 只动
`chanlun/`、`rust/src` 零改动，故被审面与本节结论不受影响；上句「其后无 commit」仅对**自审执行的时点**成立。

### 12.1 Standards 轴

**已修（本轮）**：

1. `scripts/fee_account_decomposition.py` docstring 声明了不存在的 `--ratio-against`（声明膨胀，
   `no-patch-mentality.md`）→ 删除该承诺。
2. 本次改动使两处既有文档失真 → 同步订正：`metrics.rs:320`（「构造期 fail-loud 挡住」→ 改述为
   类型承载 + 消费期处置）、`config.rs:287`（`scalar_cost_rate` → `scalar_cost_rate_opt`）。
3. `fill.rs` 新增测试 `std::fs::write(...).ok()` 吞 IO 错 → 改 `unwrap_or_else(panic)`；落盘路径
   提为具名常量并注明与 m8 同惯例。
4. `wverify_run.rs` 报告正文把 `CALIBRATED_UNAVAILABLE` 字面量重抄 → 改为常量插值（单一来源）。
5. OKLO 测试注释称有效费率「随单量单调下降」，实测 100/500 股相同 → 改述为「不是单量的常数，
   只有托底/封顶两个拐点破线性」。

**未修（登记，附理由）**：

- **`fill.rs` 4040 行 / 新增测试 158 行**越 `coding-style.md` 的 800 行与 50 行线。不在本票拆分：
  拆 `fill.rs` 是独立票（母 SPEC #385 Out of Scope 明列「runner.rs/fill.rs 拆分」），本票在此拆分
  会把验收改动面扩大到与验收无关的搬迁 diff。**登记为既有欠账 + 本票新增贡献**，不粉饰。
- **`res.fee_rate.expect(SCALAR_COST_RATE_UNDEFINED)` 在 5 处逐字重复**（Duplicated Code /
  Shotgun Surgery 判断题）。可抽 `RunResult::fee_rate_or_die()`。不做的理由：这 5 处正是 #374
  要求「消费面各自表态」的位点，收进一个方法会让「谁在消费标量费率」重新变得不可见；且票体
  明文「不过度工程」。字面重复 = 有意的显式性，登记而非隐藏。
- **`treasury::scalar_cost_rate` 沦为 Middle Man / 无生产调用点** → 已在 §3 明文披露，保留理由同处。
- **py 脚本臂R 三常数手抄** → 无 datum 可读（三常数档按定义没有），已在 §4.1 与脚本 docstring
  双处登记代价与防线。

报告文档本身经两轴复核：结果包六要素齐、L0/L1/L2 等级逐节标注、有效域声明与 231 号引用到位。

### 12.2 Spec 轴

**独立复核为真**（评审子代理自行重跑/重算，非采信本报告）：臂D 三窗 `[L2费率标定: datum cdd23adef9b3]`
标签；标定档下 `significance` 根本不调用、LCB/三态两格全标不可用；回归门 `check_armR_trades_digest.py`
**exit=0**；基线 1937/1 与 1928+9 逐数对齐；§4.2 的「择时全同、费前 Σpnl_raw 逐位相等」由 trades.jsonl
重算复现；§2 表格读数与 `/tmp/388_armD_report_*.md` 逐位相符。

**已修（本轮）**：

1. wf7 `n_orders` 315 vs 304 未解释 → §4.2 补登记（ΔN 步数 ≠ 声部笔数；描述不冒充机制证明）。
2. 「不变量三窗全绿」强于证据 → §0/§7.4 收窄为「§7.1 两条逐窗 + §7.4 清单在 default 档」。
3. `scalar_cost_rate` 无生产调用者未披露 → §3 增「须披露的结构变化」。
4. 臂D 产物抬头「端到端负/INCONCLUSIVE 照实」与新层4 措辞相左 → 降级声明块内增「对上文抬头的更正」，
   明写该句对本臂不适用、不得把「不可用」读作「负」或「INCONCLUSIVE」（产物已重跑生效）。
5. 分解表只落 `/tmp` → §1 明确稳定落盘位置 = 本报告 §4.1（仓内），脚本入仓可复跑。
6. 脚本硬编码费率与「不手抄」措辞矛盾 → §4.1 措辞订正 + 适用范围声明。

**未修（登记）**：

- **OKLO 未进 treasury 层**（母 SPEC #385 US10 的原意）。本票按 #388 票体的降级口径
  「同款构造单跑」执行，已在 §11.2 登记技术原因（m8 数据集硬编码 BTC）。**建议 T3 或新票承接**
  「OKLO 接 treasury 臂」，本票不冒充已完成。
- **`RunResult::fee_rate → Option` 属票体未逐字要求的改动面**（§11.3 已自认），保留：它是标定臂
  能跑起来的必要前置，且派工明文授权「让类型系统替你守门」。

### 12.3 一句话小结

Standards 轴：5 项已修，4 项登记不修（其中最重的是 `fill.rs` 800 行上限，属 #385 明列的 Out of Scope）。
Spec 轴：6 项已修，2 项登记（其中最重的是 OKLO 未进 treasury 层，按票体降级口径执行并明文登记）。
