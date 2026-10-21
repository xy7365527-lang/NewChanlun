# 重回 0 换向后的账：单账延续、按声部取侧、声部开局资金 policy

**日期**：2026-10-21　**裁定人**：用户（P 类草稿批复，[#1590 评论 6309984967](https://github.com/xy7365527-lang/NewChanlun/issues/1590#issuecomment-6309984967)，原话「同意」；两字段默认值出自同日 `/grill-with-docs` 记录 [6309459846](https://github.com/xy7365527-lang/NewChanlun/issues/1590#issuecomment-6309459846)）　**驱动票**：[#1590](https://github.com/xy7365527-lang/NewChanlun/issues/1590)　**被批草稿**：[评论 6309950579](https://github.com/xy7365527-lang/NewChanlun/issues/1590#issuecomment-6309950579)（独审 attempt 4 PASS）

[ADR 0040](0040-funding-signature-and-chong-derivation.md) 裁定一把方向从「重」移到「在世声部」：重 = ⟨S, ℓ, Q⟩，同一 (S, ℓ) 只有一个重，换向须先回到 0，再由本级结构证书开新声部。三阶段与本金的既有正本（[ADR 0013](0013-partition-machine-bidirectional-form.md) 裁定三／四、[ADR 0031](0031-principal-batches-phase-predicate-corporate-actions.md) 裁定一／三、`.chanlun/definitions/sanjieduan.md` §2／§3、`CONTEXT.md`）却按「多头重／空头重」分写。ADR 0040「未裁项」留下的问题是：一重回 0 换向时，三阶段状态、闸门进度、本金与成本账延续还是重置。本 ADR 回答四件事：换向后的账怎么记、谓词取哪一侧、已退本金是否回到在险池、新声部开局量不足 Q 怎么办。

## 裁定

### 一、单账延续 `[旧缠论]`＋`[新缠论:推论]`

一重回 0 后由本级结构证书开新声部，是 [ADR 0010](0010-chong-persistent-operating-unit.md) 意义下的一次 Voice 换代（「声部死而重不死」）。本金批次 P、成本 K、已实现 R、费用、`phase_events`、增长权益 G、withdrawn 全部属重，跨方向连续，**不重置、不分账**。K 是单一有符号 i64，空头声部的费后已实现利润按 ADR 0013 统一公式 `basis ← basis − d·π/units` 记进同一个 K。

依据：`040-第40课.md:20`【正文】「每一重都对应着一定的资金与筹码」；ADR 0013「一重 ＝ 一份专属筹码 ＝ 一个成本 ＝ 一个三阶段状态」；ADR 0031 裁定二「K、P、R、费用跨 Voice 连续……新 Voice 开局不改写旧尾账」；`sanjieduan.md` §4 金额账属重；[#842](https://github.com/xy7365527-lang/NewChanlun/issues/842) R-6「三阶段枢轴是本金回收、与方向无关」。

被排除：**换向即重置**（K 与阶段进度归零）——违 ADR 0031 裁定二「跨 Voice 连续」与 `sanjieduan.md` §3「Σ P_b 只增不减」；Lean 只在 `clearCampaign` 重置，重置即整重终结，归 FU-07（[#1560](https://github.com/xy7365527-lang/NewChanlun/issues/1560)）。**按方向分账**——两个成本即两份筹码即两个重（ADR 0013、[ADR 0014](0014-intra-chong-unidirectional-inter-chong-no-arbitration.md) 裁定一），违 ADR 0040「同一 (S, ℓ) 只有一个重」。

### 二、谓词按在世声部方向取侧；`Σ M_b ≔ Σ P_b` `[新缠论:推论]`

`growth_eligible` 与 `cost_remaining` 保留 ADR 0031 裁定三两侧原式，**求值时按在世声部方向取侧**。重处于 0 时只在**开新声部的求值点**按拟开方向取侧——持仓为 0 时不存在扩量求值点；这是求值时机，不是备用路径。

换向不开新批：开局批与方向无关（ADR 0040 裁定四第 4 步），新增批次只能来自显式 `principal_contribution`（ADR 0031 裁定一）⟹ 空头侧谓词里的 `Σ M_b` 读作 `Σ P_b`。

一致性代价照实：`profit_only`（裁定四）下空头声部的保证金来自 free，不是新批。多头声部已退回全部本金（多头侧资格为真）而累计费后已实现利润仍小于 `Σ P_b` 时，换空后空头侧资格为假，该声部期间不走 `q_rebase` 和溢出扩量，直到累计利润追上 `Σ P_b`。这是 ADR 0013「闸门在两侧的表现不同、判据一致」「两侧的通道数不同……会计结构自带的偏斜」已裁的偏斜；资格只管扩量、不管开局，不构成新的准入门。

被排除：**合成单谓词**（「`P_recovered ≥ P` 或 累计利润 ≥ Σ M_b」两侧通用）——「一条不过就看另一条」是 [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 的失败兜底形态，且两侧差异是载体事实，不是可选值。

### 三、latch 不回退；空头蕴含订正；开多求值点补发 `enterEarning` `[新缠论:推论]`

`TStage` latch 不回退（`earning_no_regress`，ADR 0031 裁定三「TStage 直接回退未被采纳」）。空头资格只由空头侧谓词决定，空头声部在世期间不走 ADR 0013「原重内纯差价白赚数量」通道。回 0 时 `units_g` 已随平仓变现，G 只剩 `cash_g`（[ADR 0034](0034-quantity-tier-policy-growth-equity-shift.md) 裁定五），G 不是本金，不受裁定四字段影响。

**空头蕴含订正**：latch 已到 `EarningShares` 的重换空后，`sanjieduan.md` §2 原写的「空头 `growth_eligible ⇒ latch = CapitalRecovered`」在单账下必然出现反例，订正为

> 空头 `growth_eligible ⇒ latch.rank ≥ CapitalRecovered`；空头事件不把 latch 推进到 `EarningShares`。

**多头蕴含原文不变，由补发规则维持**：K 跨方向连续会让多头蕴含出现缺口——多头声部退完本金（latch = CapitalRecovered）但 K 仍 > 0 时换空，空头利润把 K 压到 ≤ 0，回 0 后再开多，开多求值点多头谓词立即为真，latch 却仍是 CapitalRecovered。补法：开多声部的求值点若多头谓词为真且 `latch.rank < EarningShares`，在该求值点追加**既有**的 `enterEarning` 阶段事件（`formal/Origin/TotalWealth.lean:121`），known_at ＝ 求值点，append-only，AsKnown 不回填。该事件须走 legal 层：Lean 合法进入的前件是 `LegalEnterEarning s ⟺ openLegacyLegs = 0`（`TotalWealth.lean:256`，`LegalTransition` 定义 `:268`），回 0 开多时重内无未闭合腿，前件成立；实现（[#1378](https://github.com/xy7365527-lang/NewChanlun/issues/1378)）须按 `LegalTransition` 校验后发事件，不得走无守卫的 raw `twStep`。本条不新增事件。

### 四、声部开局资金 policy 记录 `#1590/voice-open-funding/v1` `[新缠论:选择]`

已退本金在开新声部时是否重新暴露、可用资金不足 Q 时怎么开，缠师原文无直接定论，属风险偏好与资金参数；按 [ADR 0041](0041-degrees-of-freedom-configurable-not-excluded.md) 落为可调字段，合法候选保留为可选值。

**记录**：新的具名 policy 记录，政策 id 族 `#1590/voice-open-funding/v1`，形状

> ⟨policy_id, version, supersedes, profile_id, reversal_principal_exposure, voice_open_shortfall, authorizer, authorized_at, valid_through⟩

v1 为 `P0`、`P1-hl-testnet`、`P0-acceptance-5m` 三个 profile（ADR 0040）各签一条：`reversal_principal_exposure: profit_only`、`voice_open_shortfall: open_available`。

签署记录（`valid_through` 取同 profile funding-record 的有效期，ADR 0040「签署记录」，不另立数值）：

```voice-open-funding-record
profile_id: P0
policy_id: #1590/voice-open-funding/v1
version: v1
supersedes: none
reversal_principal_exposure: profit_only
voice_open_shortfall: open_available
authorizer: 用户（#1590 2026-10-21 批复，评论 6309984967）
authorized_at: 2026-10-21
valid_through: 2027-10-20

profile_id: P1-hl-testnet
policy_id: #1590/voice-open-funding/v1
version: v1
supersedes: none
reversal_principal_exposure: profit_only
voice_open_shortfall: open_available
authorizer: 用户（#1590 2026-10-21 批复，评论 6309984967）
authorized_at: 2026-10-21
valid_through: 2027-10-20

profile_id: P0-acceptance-5m
policy_id: #1590/voice-open-funding/v1
version: v1
supersedes: none
scope: evidence_only
reversal_principal_exposure: profit_only
voice_open_shortfall: open_available
authorizer: 用户（#1590 2026-10-21 批复，评论 6309984967）
authorized_at: 2026-10-21
valid_through: 2027-10-20
```

**不放进 funding-record**：funding-record 形状由 [ADR 0029](0029-common-cap-contention-policy.md) 定为八字段 ⟨policy_id, version, cap, currency, valid_through, authorizer, authorized_at, profile_id⟩、ADR 0040 沿用，塞政策字段会改另一份正本的形状；ADR 0041 裁定三允许落在 policy 记录。

#### 字段 `reversal_principal_exposure`：开新声部时已退本金是否回到在险池

| 值 | 内容 | 出处 | 代价与适用条件 |
|---|---|---|---|
| `profit_only`（默认） | 已退本金留在 withdrawn，只在整重终结（`clearCampaign`）时回流；新声部只用 free（未退本金余额与利润）开局 | `031-第31课.md:28`【正文】「这样，原来投入的资金就全部收回来了」；`031:30`【正文】「操作资金不会增加」；`035-第35课.md:32`【正文】「只有当成本为0时，才算真正脱离风险」；在用先例 `rust/src/theta_v0/strategy/ledger.rs:146`（withdrawn 在 clear 时归还 free）、`:989`（「已退本金是历史事实，不被后续亏损否定」）、`TotalWealth.lean:125`／`:178`；用户 2026-10-21 默认 | 回本后的新声部只用利润做，开局量常常小于 Q，增长慢于固定 Q（见下一字段）。不新增任何事件，Rust 与 Lean 都不改，与 ADR 0031 裁定三「本轮不另开 Lean 事件集」一致 |
| `chong_full`（可选） | 开新声部时 withdrawn 全额以一条显式 typed 回流事件回到 free（withdrawn→free、`P_recovered` 同额减少、`phase_events` 追加、AsKnown 不回填），新声部按 Q 开局 | ADR 0010／0040 固定 Q 是动作基；#842 R-6「永远在场」；ADR 0013「各重保证金逐仓分开」损失有界 | **已收回的本金再次暴露**，重回「成本为0以前」；多头侧资格因 `P_recovered < P` 失效直到再次退本金；损失有界于该重自己那份钱。须新增 Rust 与 Lean 回流事件并与 `clearCampaign` 区分，由实施票 [#1651](https://github.com/xy7365527-lang/NewChanlun/issues/1651) 交付；**交付前签成此值的记录无效**（见下「缺值与非法值」） |

#### 字段 `voice_open_shortfall`：可用资金不足 Q 时怎么开局

| 值 | 内容 | 出处 | 代价与适用条件 |
|---|---|---|---|
| `open_available`（默认） | 开局量 ＝ min(Q, 可用资金)，不凑足 Q；本声部的满仓就是这个开局量，声部存续期间不追加 | `026-第26课.md:34`【正文】「最开始多少就是多少……但绝对不加仓，一开始就买够」；`031:26`【正文】「投入以后就不能再增加」；[ADR 0017](0017-chong-quantity-numerator-shortdiff-amplitude.md) 裁定一「先一次性买够，再做短差」；ADR 0031 裁定一新增本金只能经 `principal_contribution`；`CONTEXT.md`「反向根」准入只认反向证书、无额外门；用户 2026-10-21 默认 | 声部规模随利润与亏损浮动，回报读数须按实际开局量归一，不能按 Q 直接比 |
| `topup_from_withdrawn`（可选） | 只在 `reversal_principal_exposure = profit_only` 时有意义：从 withdrawn 回流 min(Q − 可用, withdrawn) 补到 Q，回流按 `chong_full` 同款 typed 事件记账 | ADR 0010／0034 固定 Q 是动作基 | 部分本金再暴露，量为补足差额；需要与 `chong_full` 相同的回流事件，同样在 [#1651](https://github.com/xy7365527-lang/NewChanlun/issues/1651) 交付前无效。`chong_full` 下 withdrawn 已全额回流，此值与默认等价 |

两个字段的值都不违原文、无实测否证，不是同一判断的宽严两档；运行前单选、一次运行只读一个值，没有「主值失败换另一值」的路径（ADR 0041 裁定二）。**保留两个字段是设计取舍**：`topup_from_withdrawn` 是 `chong_full` 的部分版本，(`chong_full`, `topup_from_withdrawn`) 与 (`chong_full`, `open_available`) 等价，存在退化组合；合成一个三值字段也合法，本 ADR 按用户批复的两问分写两字段，退化组合照上表等价处理，签署时不必拒收。

被排除的真错（ADR 0041 裁定四）：

- **开不足就等补款或不开**（等 `principal_contribution` 签下来再开、或可用 < Q 就跳过本次证书）：在反向证书之外加了一道准入门，违 `CONTEXT.md`「反向根」「无额外门」与 ADR 0040 裁定一「每开一个声部，方向由本级结构证书决定」。
- **自动把 Q 调小到可用资金**：违 ADR 0034 裁定五「Q 只能由 `q_rebase` 政策版本改写」。
- **开局后再加仓补到 Q**：违 `026:34`「绝对不加仓，一开始就买够」与 ADR 0017 裁定一。
- **临时加杠杆补足**：杠杆是 ADR 0040 `derivation-input` 已签字段，运行中按资金状况换值属 ADR 0041 判别句所禁的运行中换值。

#### 读取点与计算

- **唯一读取点**：「重处于 0、按本级结构证书开新声部」，由 [#1421](https://github.com/xy7365527-lang/NewChanlun/issues/1421)（TB-03-E Voice 换代）实现。**换向与同向重开同一规则**，承重理由是 ADR 0040 裁定一：方向不是重的属性，回 0 后重上没有「上一声部方向」可比，读取点上换向与同向重开不可区分。默认取值下这也与现行先例一致——withdrawn 只在 `clearCampaign` 回流（`TotalWealth.lean:125`、`ledger.rs:146`），同向重开本来就拿不到 withdrawn。可选值下同向重开同样回流，这是上一句的直接后果，签可选值时须知。
- **开局量** ＝ min(Q, 可用资金)；可用资金 ＝ free（`profit_only` 下不含 withdrawn）。可用资金超过 Q 时按 Q 开局，余额留在重内按 ADR 0034 裁定五／六与 ADR 0040 裁定四利润溢出段走 `q_rebase` 与溢出。
- **数量为零**：可用资金按场所下单粒度取整后数量为 0 时不发开局指令，输出 typed `NotComputable{reason: quantity_zero}`（ADR 0031 裁定二同款形态）。这是物理上下不了单，与被排除的「可用 < Q 就不开」不同——后者是数量大于 0 时由政策拦下。该重是否终结归 FU-07（#1560）。
- **满仓与量档**：本声部的满仓 ＝ 开局量，ADR 0014 裁定一的 `[0, 满仓]` 约束按此读；ADR 0017 裁定二「`w(0) ≡ 1.0` 即满仓」在开局量 < Q 时读作 `w(0)×Q` 经 `[0, 满仓]` 截断后等于满仓。量档按 ADR 0034 裁定四 `tier_quantity = ⌊Q·num/den⌋` 原式不缩放，执行时受 `[0, 满仓]` 截断，回补与卖出同量。被排除：逐声部按开局量重算档量——违 ADR 0034 裁定一「tier_quantity 整数版本内冻结」、裁定三版本冻结与「回本前同量回补按原卖出冻结的版本与数量」及裁定四取整式（裁定五「Q 固定、动作基」为辅证）。
- **缺值与非法值**：记录缺失、字段未签、或值不在可选集时，输出 `AwaitingPolicy{voice_open_funding}` 并停在该开局点，不产生订单；代码里不得有常数默认或备用值（ADR 0041 裁定二）。`chong_full`、`topup_from_withdrawn` 在 #1651 交付前由 #1421 的记录读取／签署校验拒收：这样的记录不生效，读取点看到的就是「没有有效记录」，同样输出 `AwaitingPolicy{voice_open_funding}`，**不回退到默认值**。#1651 交付后由它撤销该拒收。
- **改值**：签新版本，`supersedes` 指向旧版本，只作用于生效后开出的声部。
- **两个字段都不是准入门**：开不开局只由本级结构证书决定（ADR 0040 裁定一；根声部换向即 `CONTEXT.md`「反向根」），字段只决定开局用哪部分钱、开多少。

### 五、Lean 与事件

默认取值下不新增任何事件：withdrawn 照旧只在 `clearCampaign` 回流，ADR 0031 裁定三「本轮不另开 Lean 事件集」维持；裁定三补发的 `enterEarning` 是既有事件。选 `chong_full` 或 `topup_from_withdrawn` 的前提是 [#1651](https://github.com/xy7365527-lang/NewChanlun/issues/1651) 交付：新增 typed 回流事件（Rust `rust/src/theta_v0/strategy/ledger.rs` 与 Lean `formal/Origin/TotalWealth.lean`），与 `clearCampaign` 区分，并声明 Lean 覆盖面；空头侧按 `sanjieduan.md` §2 由 Rust 同输入同输出锁（[#804](https://github.com/xy7365527-lang/NewChanlun/issues/804)）兜住。

## 被比较的方案

| 方案 | 处置 |
|---|---|
| 换向即重置 K 与阶段进度 | 排除（裁定一） |
| 按方向分账 | 排除（裁定一） |
| 两侧谓词合成单谓词 | 排除，#799 兜底形态（裁定二） |
| latch 换空时回退 | 排除，`earning_no_regress`（裁定三） |
| `reversal_principal_exposure: chong_full` | 保留为可选值，代价见裁定四 |
| `voice_open_shortfall: topup_from_withdrawn` | 保留为可选值，代价见裁定四 |
| 等补款／不开、自动调小 Q、开局后加仓、临时加杠杆 | 排除，真错（裁定四） |
| 字段塞进 funding-record | 排除，改 ADR 0029 形状（裁定四） |
| 逐声部按开局量重算档量 | 排除，违 ADR 0034 冻结（裁定四） |

## 受影响票与代码

**新增 blocking 边**：#1590 blocks [#1378](https://github.com/xy7365527-lang/NewChanlun/issues/1378)、[#1406](https://github.com/xy7365527-lang/NewChanlun/issues/1406)、[#1415](https://github.com/xy7365527-lang/NewChanlun/issues/1415)、#1421。

- **#1378**（TB-06-B）：裁定一、二、三的直接实现者。验收须含负控：多头退本金 → K > 0 换空 → 空头利润使 K ≤ 0 → 回 0 开多，断言开多求值点经 `LegalTransition` 追加 `enterEarning` 且多头蕴含成立。实现时另核：`LegalTransition` 要求 `openShareLeg` 前 `stage.rank < EarningShares`（`TotalWealth.lean:270`），latch 到 `EarningShares` 后的任何声部（含空头）都不能再开 legacy ShareConserving 腿，空头声部是否依赖它由 #1378／#1406 核对。
- **#1406**（TB-03-C）：`sanjieduan.md` §2 OQ-2a 蕴含及证明与对拍义务归它；空头蕴含按裁定三订正。默认取值下 ledger 不新增回流事件。
- **#1415**（TB-03-D）：满仓 ＝ 本声部开局量、档量不缩放、截断后同量回补写进验收；开局量由记录与账本算出，不是新数量默认。
- **#1421**（TB-03-E）：`#1590/voice-open-funding` 唯一读取点与签署校验。验收须含：缺记录、缺字段、非法值、签成未实装值各一例负控，断言输出 `AwaitingPolicy{voice_open_funding}` 且不产生订单；`profit_only` 下 withdrawn > 0 时开局量 ＝ min(Q, free) 的正例；数量为零输出 `NotComputable{reason: quantity_zero}`；把读取改成常数的变异体须让测试变红。
- **#1651**（新开实施票，挂 [SPEC #1340](https://github.com/xy7365527-lang/NewChanlun/issues/1340)）：可选值所需的 withdrawn→free typed 回流事件（Rust＋Lean），交付后撤销 #1421 对两个可选值的拒收。

受影响但不增边：#1560（终结语义不变；数量为零后是否终结归它）、#1601（funding-record 形状不变）、#1444（换向是重内 Voice 换代，不开新重）、#1446／#1398（经 #1378 传递）、#1445（只经 `q_rebase` 前件间接相关）、#1610（无交集）。

**受影响代码清单**（基线 main `02b5b571eb`）：

| 侧 | 文件:行 | 处置 |
|---|---|---|
| Rust | `rust/src/economic_session/book.rs:442` | `capital_phase` 按在世或拟开方向取侧；开多求值点经 legal 层补发 `enterEarning`（#1378） |
| Rust | `rust/src/economic_session/voice.rs:169` | 开局量 > 0 不变；开局量改读 `#1590/voice-open-funding` 记录按 min(Q, 可用资金) 计算，缺记录 `AwaitingPolicy{voice_open_funding}`（#1421） |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:97`／`:146` | 默认取值下不改；可选值的回流事件归 #1651 |
| Lean | `formal/Origin/TotalWealth.lean:125`／`:178` | 默认取值下不改；可选值的回流事件归 #1651 |
| Lean | `formal/Origin/TotalWealth.lean:256`／`:268` | `LegalEnterEarning`／`LegalTransition` 不改，补发 `enterEarning` 须满足 |

**正本订正指针**（原文不删）：`sanjieduan.md` §2／§3、`CONTEXT.md`「重内单向」「cost_remaining」「本金批次」、ADR 0013 裁定四闸门两侧段与空头扩量段、ADR 0017 裁定二、ADR 0031 裁定一／三、ADR 0040 未裁项。

## 溯源标签

裁定一 `[旧缠论]`（`031:28`、`040:20`）＋`[新缠论:推论]`（R-6）；裁定二、三 `[新缠论:推论]`；裁定四两个字段 `[新缠论:选择]`（默认值由用户 2026-10-21 给出并批复）；裁定四「不凑足 Q、不追加」与读取点计算、裁定五 `[新缠论:推论]`（`026:34`、`031:26`、ADR 0017、ADR 0031、ADR 0034）。
