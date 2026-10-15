# 三阶段与本金（Sanjieduan / Principal）

> 正本：本文件是 三阶段与本金 的教义正本。
> 最后裁定：[#1348](https://github.com/xy7365527-lang/NewChanlun/issues/1348)（2026-10-13）；[#1406](https://github.com/xy7365527-lang/NewChanlun/issues/1406) OQ-2a 补裁（2026-10-14）
> 受影响代码清单：本文件 §5（基线 main `465174240b`；FIN 线 `0d8a6017` 单独标注）；工作草稿副本 [`.chanlun/review-results/issue1323-grilling-20261013/1348-AFFECTED-CODE.md`](../review-results/issue1323-grilling-20261013/1348-AFFECTED-CODE.md)，票关后随归档出仓

**状态**: 已裁定（#1348 第一至第三问的教义部分；数据形态、零分母与公司行动见 [ADR 0031](../../docs/adr/0031-principal-batches-phase-predicate-corporate-actions.md)）
**最后更新**: 2026-10-15
**原文依据**: 第 31、35、40、49 课（博文，`docs/chanlun/text/blog/`）

**与既有生成态文件的关系**：[`fengkong.md`](fengkong.md)「概念3：成本归零策略」与 [`chanlun-trading-system.md`](chanlun-trading-system.md)「概念2：资金管理」描述的是同一组三阶段，二者仍为生成态操作说明。自 #1348 起，阶段如何判定、本金按什么口径计，以本文件为准；两处与本文件不一致时以本文件为准。

## §1 原文锚

全部按四步核过（行号、逐字、正文界、行首与行内注），路径一律 `docs/chanlun/text/blog/`。

- `031-第31课.md:24`【正文，界 :32】「一个最简单又最有效的管理，就是当成本为0以前，要把成本变为0；当成本变成0以后，就要挣股票，直到股票见到历史性大顶」
- `031-第31课.md:30`【正文，界 :32】「直到牛市结束前，才把所有股票全部清仓。而资金，就可以不断增加参与的股票种类，把这程序不断下去，这样，操作资金不会增加」
- `031-第31课.md:169`【界后，作者答疑回复：`:165` 回复块署名「缠中说禅：2007-02-15 16:16:43」，`:167` 为读者问，`:169` 在「==」后】「成本为0前，只补进相同的数量，仓位不增加。成本为0后，抛出后，跌回来，就把抛出的钱，全补进去，这样买回来的数量一定多了，股票才会越来越多」
- `035-第35课.md:32`【正文，界 :38】「只有当成本为0时，才算真正脱离风险」
- `040-第40课.md:20`【正文，界 :24】「每一重都对应着一定的资金与筹码」
- `049-第49课.md:64`【正文，界 :78，行首无注】「在中枢上方全部抛出筹码，在下方如数接回」

## §2 阶段是成本状态的函数 `[旧缠论]`

三阶段（降成本 → 退本金 → 增股数）由该重当前的真实成本 K 与已退本金决定（§1 `031:24`、`035:32`）。费用、亏损等真实经济事件可以使 K 回到正数，此时该重重新处于「成本为0以前」，扩量资格随之失效。

**与已交付单向 latch 的分工**（#1348 第三问，`[新缠论:选择]`）：`TStage`（Rust `rust/src/theta_v0/strategy/ledger.rs:94`；Lean `formal/Origin/TotalWealth.lean:140`、`:194`、`:208`）是 campaign 内的阶段**事件史**投影，只记「达标事件曾经发生」，AsKnown 不回填；**扩量资格**由唯一谓词 `growth_eligible` 按 ADR 0031 裁定三的分方向当前账本事实计算（多头为 K、P、P_recovered；空头为费后已实现利润、保证金批次与负债），latch 不得作为它的输入。两者是「历史事实」与「当前准入」两个对象，不是同一判断的两档（#799 限定词①）。更正类事件（bust）引起的阶段重算按 SPEC C08.6 以 RecomputedWithRevision 独立命名。

**分方向的单向蕴含**（#1406 OQ-2a 补裁）：多头 `growth_eligible ⇒ latch = EarningShares`；空头 `growth_eligible ⇒ latch = CapitalRecovered`，空头 latch 不进入赚股第三相。依据为 ADR 0031 裁定一的空头回本判据、裁定三的分方向资格谓词与 SPEC C05.5。两条均非双向等价：回本后的亏损往返即使没有费用，也可令当前资格失效而阶段事件史保留；latch 仍不得作为资格输入。

**证明与对拍义务归 #1406**：`TotalWealth.lean` 既有阶段单调性定理不等于上述资格蕴含已获证明。交付时须声明候选 Lean 模型的实际覆盖面；若只覆盖多头，空头条由 Rust 同输入同输出锁（#804）承担，不得声称空头资格关系已形式证明。

## §3 本金按来源分批，利润不是本金 `[旧缠论]`

本金 P ＝ 该重已授权来源的真实自有投入之和，分批冻结：开局批 P0（多头重为开局锁定的自有现金/保证金；空头重为投入保证金 M，ADR 0013 裁定三）。回本后利润再投入是**同一重的股数增长**（§1 `031:169`），进 income-book 的资本化 A 栏，不进 P；提取 W 也不动 P。新增外来投入只能以显式授权批次进入（§1 `031:30`「操作资金不会增加」；ADR 0016 裁定五「扩张走换档，不挪钱」）。回本目标 Σ P_b 只增不减。

回本后滚出的筹码与资本化现金记增长权益 G（ADR 0034），不是本金；换档转移形成的目标重本金批次来源为 `transfer(id)`，金额 = 转移现金 + 转移筹码的账面成本。（#1355／#1357）

## §4 数量归零不是账的终点 `[旧缠论]`

§1 `049:64` 全部抛出再如数接回，中间数量为零而操作没有中断。金额账（K、P、R、费用）属重，跨 Voice 连续；归属指针属 Voice/family/command（ADR 0010「声部死而重不死」）。数量为零期间单位量输出 Typed NotComputable。整重经营终结（§1 `031:30`「才把所有股票全部清仓」；Lean `clearCampaign`）是另一事件，归 FU-07，本文件不裁。

## §5 受影响代码清单

基线：main `465174240b989a1d99f99a047fbb46b3ee215f49`；FIN = #1394 财务线 `0d8a6017231348e2c4a005f68d4e2e7a457a59fe`（未合入 main）。改码归 #1406、#1421 与 TB-06 叶票；本节只列影响面。

| 侧 | 文件:行 | 基线 | 处置 |
|---|---|---|---|
| Rust | `rust/src/economic_session/book.rs:151` | main | 分量白名单增 principal_contribution / fee_only / dividend_cash / withholding_tax / split |
| Rust | `rust/src/economic_session/book.rs:225` | main | `du==0` 二分：无费用分量拒；有已声明费用分量则接纳、只改 K |
| Rust | `rust/src/economic_session/book.rs:347` | main | `principal_baseline` → Known 批次清单；`capital_phase` → phase_events + growth_eligible；`unit_cost_policy` → 整数对/NotComputable |
| Rust | `rust/src/economic_session/book.rs:356-357` | main | 非负存量域不变，q=0 合法 |
| Rust | `rust/src/economic_session/book.rs:370` | main | Π/A/W/R 算子不变；A 不进 P |
| Rust | `rust/src/economic_session/book.rs:398-413` | main | K 有符号累加（处置现金、回收、分红进入后可为负），不 clamp |
| Rust | `rust/src/economic_session/store.rs:636` | main | 审计重放覆盖 fee_only / principal_contribution / 公司行动事件 |
| Rust | `rust/src/economic_session/voice.rs:149` | main | 新 Voice 开局依 Applied 原件；不接旧尾账 |
| Rust | `rust/src/economic_session/voice.rs:165` | main | 开局量 > 0 不变 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:94` | main | `TStage` 只作阶段事件史投影；分工注释须带 #1348 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:121` | main | `advance_to` 不改 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:175` | main | `TwState`：notional_in 不是本金批次；stage 不作准入输入 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:716/742` | main | `LedgerEvent` / `ledger_step_components` 不变 |
| Rust | `rust/src/theta_v0/backtest/fill.rs:200-255` | main | 旧净额臂（全平重置 entry_cost，`:224`）与本裁定不同口径，不作本金定义；列入 C11 收编/退役清单，本轮不代裁终态 |
| Rust | `rust/src/session_protocol.rs:39` | main | 引用：`parse_canonical_i64` 规范十进制整数文本，不改 |
| Rust | `rust/src/economic_session/finance.rs:143-145` | FIN | 引用：`checked_add` 整数约定，不改 |
| Rust | `rust/src/economic_session/finance.rs:208-210/227` | FIN | 引用：money_unit / quantity_unit / quantum 政策字段与 `quantum == "1"` 校验，不改 |
| Rust | `rust/src/economic_session/finance.rs:234-239` | FIN | 引用：`baseline.amount` 是 E 共同责任界，与本金 P 禁止互代，不改 |
| Lean | `formal/Origin/TotalWealth.lean:97` | main | `TWState.notional_in` 登记为非本金批次名分；不改定义 |
| Lean | `formal/Origin/TotalWealth.lean:140` | main | `advanceTo` 不改 |
| Lean | `formal/Origin/TotalWealth.lean:194` | main | `stage_rank_monotone` 不改；#1406 OQ-2a 义务：多头 `growth_eligible ⇒ latch = EarningShares`，空头 `growth_eligible ⇒ latch = CapitalRecovered` 且无第三相；非双向等价，latch 不作资格输入；Lean 覆盖面与 Rust 同输入同输出锁见 §2 |
| Lean | `formal/Origin/TotalWealth.lean:208` | main | EarningShares 不可回退（`earning_no_regress`）不改 |
| Lean | `formal/Origin/LedgerReconciliation.lean:467` | main | `hAlloc` 前件不改；不从中推本金 |
| Python | `s_session/economic_query.py:72` | main | 只透传 principal / capital_phase / unit_cost 的 typed 结构 |
| Python | `s_session/economic_http.py:36/62` | main | GET/POST 入口只透传，不新判 |
| JS | `s_session/browser/economic.html:67` | main | 「本金与阶段」卡片显示 NotComputable 原因与 K/q 整数对，不计算比率 |
| JS | `s_session/browser/tb03a-client.js:124` | main | 图像校验键集若扩 principal/phase_events 须同步 |

## 更新日志

- 2026-10-15：同步 #1406 OQ-2a（2026-10-14 补裁），资格与 latch 关系分方向写为单向蕴含，订正 §5 并明确 Lean/Rust 验证覆盖义务；无代码行为变更。
- 2026-10-13：v1.0 新建（#1348 裁定落正本层）。
