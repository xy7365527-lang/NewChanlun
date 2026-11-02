# ADR 0031：本金批次、阶段准入谓词、零分母与公司行动的会计口径

**日期**：2026-10-13
**裁定人**：代理依用户 2026-10-13 授权裁定（分析：claude-fable-5-1；根逐条复核采纳并补充），走 [[grilling] 本金与真实回收的边界会计 policy #1348](https://github.com/xy7365527-lang/NewChanlun/issues/1348)（RD-08 / FU-01，SPEC #1340）
**状态**：已采纳。本 ADR 只定口径、数据形态与受影响面；实施归 #1406 / TB-06 叶票。
**教义正本**：[`.chanlun/definitions/sanjieduan.md`](../../.chanlun/definitions/sanjieduan.md)。成本主干不变：[ADR 0024](0024-total-ledger-convention.md) 裁定四。

## 裁定

### 一、本金分批冻结，利润资本化不计本金（第一问 A）

`P = Σ_b P_b`，每批记 `(batch_id, source_ref, amount: i64, currency, money_unit, quantum, known_at, authorization_ref)`。开局批 P0 为开局锁定的自有现金/保证金；空头重为投入保证金 M（[ADR 0013](0013-partition-machine-bidirectional-form.md) 裁定三）。income-book 的 A（资本化）与 W（提取）不改 P；退本金走既有 `RecoverCapital`（free→withdrawn）并记 `P_recovered`。新增批次只能由显式授权事件 `principal_contribution` 形成。本金已收回 ⟺ `P_recovered ≥ P`（多头）／`累计费后已实现利润 ≥ Σ M_b`（空头）。P 与 #1394 E 侧 `baseline.amount`（共同责任界，FIN `finance.rs:234-239`）是不同量，禁止互代。

> **★ 2026-10-21 订正指针（[#1590](https://github.com/xy7365527-lang/NewChanlun/issues/1590)，[ADR 0043](0043-chong-reversal-account-continuity.md) 裁定一／二，用户批复）**：「多头／空头」读作在世声部为多／为空；P、K、`phase_events` 跨方向连续，不重置、不分账；换向不开新批，`Σ M_b ≔ Σ P_b`。原文不删。

### 二、金额账属重、归属属 Voice；零数量期间单位量 NotComputable（第二问 A）

K、P、R、费用跨 Voice 连续；迟到费用 f 进该重 K，归属指针留在原 command/family/Voice（SPEC C05.1、C07.10）。`du = 0` 且带已声明费用分量的原件合法（新 kind `fee_only`），只改 K；`du = 0` 且无费用分量仍拒。单位量输出只有两种形态：q = 0 ⟹ `{status:"NotComputable", reason:"quantity_zero", K, q:"0"}`；q > 0 ⟹ 整数对 `{K, q}`，不做除法。新 Voice 开局不改写旧尾账。整重终结归 FU-07。

### 三、阶段事件史保留，扩量资格按真实状态重核（第三问 A）

`phase_events` append-only，AsKnown 不回填；更正类按 C08.6 以 RecomputedWithRevision 另列。唯一准入谓词 `growth_eligible ⟺ K ≤ 0 ∧ P_recovered ≥ P`（多头）／`费后已实现利润 ≥ Σ M_b ∧ 负债仍列`（空头）。

> **★ 2026-10-21 订正指针（[#1590](https://github.com/xy7365527-lang/NewChanlun/issues/1590)，[ADR 0043](0043-chong-reversal-account-continuity.md) 裁定二／三）**：两侧原式保留，按在世声部方向取侧；重处于 0 时只在开新声部的求值点按拟开方向取侧。空头蕴含订正为 `latch.rank ≥ CapitalRecovered`，开多求值点补发既有 `enterEarning`（见 `sanjieduan.md` §2）。原文不删。

`TStage` latch（`rust/src/theta_v0/strategy/ledger.rs:94`、`:121`；`formal/Origin/TotalWealth.lean:140`、`:194`、`:208`）只记阶段事件史，不改、不读作准入（根确认）。本 ADR 不改 Lean，本轮不另开 Lean 事件集；「TStage 直接回退」未被采纳，将来若要改须另开改裁票并重述 `:194`/`:208` 的前件。K 取有符号 i64，不 clamp；跨零不削减任何目标。

### 四、零分母 Typed NotComputable，绝对差额保留（第四问 A）

[ADR 0022](0022-acceptance-criterion-parameters.md) 的速率是闸门前指标；`C_before ≤ 0` 的事件在其定义域之外，输出 `{status:"NotComputable", reason:"nonpositive_base", delta: K_before − K_after}`，统计样本登记 `out_of_domain:post_gate`。不换分母、不改已裁统计法；闸门后判据仍按 ADR 0022 延后。报告层零成本标记 ＝ `K ≤ 0`。

### 五、公司行动（第五问 A，`[新缠论:选择]`，原文留白）

现金分红政策 `dividend_treatment={recover_reduce_k,capitalize_no_reduce_k}`（#1599 用户 2026-10-26 批准），默认 recover_reduce_k：净额 `dividend_cash_net: i64` 计已实现回收、减 K；备选 capitalize_no_reduce_k：净额计资本化 A、不减 K，代价是延后本金回收门触达。gross/withheld/net 不双计，不把 A 或 K 偷换成 G；事件入口与回放消费同一具名版本，改值新版本、不失败换值。代扣税 `withholding_tax: i64` 作费用分量，gross/withheld/net 三值与币种留原件；除权参考价调整是估值，不改 K；拆股以整数比 `(num, den)` 重基，`q' = q·num/den` 须整除，否则原件须自带零股处置（现金代付即回收）或 `Unsupported`，不四舍五入；拆股不造回收、不改 K；fixed_q 按原件重基（是否触发 [ADR 0016](0016-chong-quantity-layer-capacity-frequency-shift.md) 换档另裁）。空头重支付的分红是费用分量。公司行动一律走 P4 Recorded+Pending 原件。本条收口 ADR 0024 裁定四的遗留子项。

## 数值口径

全部金额为规范十进制整数文本解析的 `i64`（main `rust/src/session_protocol.rs:39` `parse_canonical_i64`），加法用 `checked_add`（FIN `finance.rs:143-145`）；不出现浮点，不出现除法结果字段；比率只在报告层以 typed 结果出现。单位成本只输出整数对 `{K, q}`（根确认）。

## 受影响代码清单

见教义正本 [`sanjieduan.md`](../../.chanlun/definitions/sanjieduan.md) §5（基线 main `465174240b`，FIN 线 `0d8a6017` 单独标注）；工作草稿副本 [`.chanlun/review-results/issue1323-grilling-20261013/1348-AFFECTED-CODE.md`](../../.chanlun/review-results/issue1323-grilling-20261013/1348-AFFECTED-CODE.md)。

## 下游票

[#1406](https://github.com/xy7365527-lang/NewChanlun/issues/1406)（G2b 四项转为可做；T-17 成功路径；新增 T-18/T-19）、[#1421](https://github.com/xy7365527-lang/NewChanlun/issues/1421)（family→Voice 映射）、[#1378](https://github.com/xy7365527-lang/NewChanlun/issues/1378)/[#1397](https://github.com/xy7365527-lang/NewChanlun/issues/1397)（TB-06 账例与统计样本口径）、[#1350](https://github.com/xy7365527-lang/NewChanlun/issues/1350)（迟到费用的分配指针，见 [ADR 0032](0032-fill-allocation-policy.md) 裁定五）。

## 溯源标签

裁定一至三的教义部分为 `[旧缠论]`（见教义正本 §1–§4 的原文锚）；数据形态、零分母处置与公司行动为 `[新缠论:选择]`。

## #1599 批复与受影响清单

[用户批复](https://github.com/xy7365527-lang/NewChanlun/issues/1599#issuecomment-6403278294)；[受影响清单](../../.chanlun/review-results/issue1599-policy-20261026/affected-code.md)含固定基线、Rust/Python/Lean 接缝及下游未承接项。本次为政策文书，不声称实现或关票；新增自由度为 `[新缠论:选择]`，既裁不变量不变。
