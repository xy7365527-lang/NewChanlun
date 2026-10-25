# ADR 0040：资金签署记录与重的确定性推导

> 正本：本文件是资金签署记录（总投入、共同 cap、逐级容量、起始操作级别、频率门、杠杆、量档）及其推出的重清单的正本。
> 最后裁定：#1610（2026-10-26 分维订正）；#1557 / #1558（2026-10-20 资金签署原件保留）
> 受影响代码清单：见「受影响票与代码」节

**日期**：2026-10-20
**裁定人**：用户（2026-10-20 `/grill-with-docs` Q1–Q5 共识，#1557 评论 6283050080、6284629893、6284716884）；代理起草
**状态**：已采纳。签署单与推导全文 `/Volumes/AgentStorage/decisions/1557/draft.md`；默认值的考古出处 `/Volumes/AgentStorage/decisions/1557/archaeology.md`。

## 背景

ADR 0029、0031、0034 定了 cap、本金批次、量档与容量记录的形状，数值全部留给用户签。第一版签署单按方向签投入额（多头重、空头重各一份钱，同级两重再签怎样分容量），用户否掉了这个前提（#1557 评论 6282686046 引用户原话「重不是按照一开始做多投多少的，这个东西也应该是自适应」）。重新对词后的共识：重没有固定方向；重按级别开；唯一人签的输入是每个 profile 的总投入；其余全部推导，凡正本留有自由度的量都落为可改签的版本化记录。

## 裁定

### 一、重没有固定方向；方向属于在世的声部

重 = ⟨S, ℓ, Q⟩（ADR 0010:23），方向不是要件。一重某一时刻的持仓方向就是它当前在世声部的方向；每开一个声部，方向由本级结构证书决定（Q1）。ADR 0014 裁定一的约束保留：一个声部存活期间各结构级别只在 `[0, 满仓]` 内同向加减、不穿零；**换向须先回到 0**。ADR 0014:21「该重的净持仓符号在其生命周期内由该重自己的方向决定」中的「重的生命周期」订正为「一个声部的生命周期」。

同一 (S, ℓ) 只有一个重（Q2「重按级别开、不按方向开」）。同时持多与持空（ADR 0014:19「两份筹码，那就是两个重」）只出现在不同操作级别的两个重之间——跨级多空并存是多重赋格的设计（ADR 0011:39、ADR 0013:17 与裁定六 X5、ADR 0014 裁定三、#787、#837、#842 R-4）。

### 二、小资金只有一重是原文设计

钱 ≤ C(ℓ_start) 时只有一个重，这是原文给小资金的最优做法：`026-第26课.md:36`【正文】「对于资金量小的投资者，完全可以全仓进出，游走在不同的凭证之间。这样的效率当然是最高的，不过这不适用于大资金」；`035-第35课.md:30`【正文】「从纯理论的角度，操作级别越低，相应的效率越高」。大资金的部分筹码进出是装不下时的补救：`044-第44课.md:50`【正文】「这样的操作，对一定量的资金是唯一可行的，因为这种资金，不可能在任何一定级别的卖点都全仓卖掉」；`049-第49课.md:76`【正文】「对于大资金，以上的方法需要有特殊的处理，资金越大，利润率显然越低，因为很多级别的操作不可能全仓参与」。

- **开在 L0 的单重**：L0 是塔底（线段层，`.chanlun/definitions/qujiantao.md:516`；`:441` 标塔底位置为 `[新缠论:选择]`），其下没有次级别，所以不做同向短差。形态 = 全仓进出 L0 买卖点 + 卖点后清零换向，即 `026:36` 的「全仓进出」。
- **开在 L1 及以上的重**：满仓基线（ADR 0017:54）+ 同向短差 + 卖点后清零换向。同向短差是操作级别以下各结构级别的卖点减 `w(d)×Q`、买点回补，持仓在 `[0, 满仓]` 内、并进本重成本（ADR 0013:34；`026-第26课.md:34`【正文】「卖点的时候变少，买点的时候又回复原来的数量」），用本重自己的钱，合法；ADR 0013 X5 只禁「中枢完成后的向上移动时的差价」（`049-第49课.md:52`【正文】注前一句）。w(1) 只用于操作级别 L1 的重。

同向短差出现在三种情形：签 `ℓ_start = L1`；资金长满 L0 溢出开出 L1 重；阶段二塔向下延伸（ADR 0035，成本门归 #1559）。反向短差与跨级多空需要另一个级别的重，随资金溢出长出（ADR 0016 裁定五「N 重是长出来的」）。

### 三、唯一人签输入与默认值

每个 profile 只签总投入 `Q_total`：`P0` 100,000；`P1-hl-testnet` 500；`P0-acceptance-5m` 5,000,000（验收专用回放，只出证据、不进生产、不下单）。币种 USDC 名义整数。其余默认值经用户同意（Q4、Q5），出处如下：

| 字段 | 默认 | 出处 |
|---|---|---|
| `l_start` | L0，可签 L1 | ADR 0016:92/:98；`035:30`；`026:36`；Q5 |
| symbol | BTC | #1283 评论 5465238861；首轮品种池 {OKLO, BTC} |
| C(ℓ) | L0 1,110,000；L1 3,200,000；L2 9,630,000；L3 Unknown | #917 评论 5199839236；#915 报告 `:268-272`；ADR 0017:180/:192 |
| `margin_leverage` | 1 | #842 R-10 挂账；γ = 1.0 先例 |
| 频率门 | 买点 ≥ 1 且卖点 ≥ 1（`F_min: 1`） | ADR 0016:106；#915 评论 5209250256；BTC L0–L3 ROUNDTRIP n 均 > 0（#915 报告 `:153`） |
| w(1) | 347/1000，Measured，域 BTC 1m/δ=0.5/中位/L0→L1，只用于 L1 重 | ADR 0017:215/:219；ADR 0034:14 |
| cap | ⌈Q_total × 101/100⌉ | γ = 1；1% 费用尾余量为 `[新缠论:选择]` |
| 有效期 | 签署日 + 365 天 | `[新缠论:选择]` |

**通则（Q5）**：凡正本留有自由度的量，一律落为版本化签署/配置记录，带默认值与出处；实现只从当前生效版本读，不写成代码常数（与 ADR 0034:39 被否「常数表」「运行时现算（伪造默认）」同向）。

### 四、确定性推导

整数运算，P0 保证金向上取整，其余向下取整。

1. `ℓ := l_start`，`R := Q_total`。成本门 NotEvaluable 不挡 L0（ADR 0035:15）。
2. 逐级：买点或卖点 < `F_min` ⟹ NotViable，不开重，看 ℓ+1；`F_min` 未签、无频率读数或 C(ℓ) = Unknown ⟹ `AwaitingPolicy{frequency | capacity, level:ℓ}`，停止；其余 Placeable。
3. Placeable：`q := min(R, C(ℓ))`，开重 ⟨S, ℓ, q⟩，`R := R − q`；`R = 0` 停止，否则 ℓ+1。无方向参数，同一 (S, ℓ) 至多一个重。
4. 每重开局批 `amount = ⌈Q_amount / margin_leverage⌉`（原公式中的 L 是无量纲杠杆，不是价格；#1610、ADR 0049）：声部为多时是锁定的自有现金/保证金，声部为空时就是投入保证金 M。`authorization_ref` 指该 profile funding-record 的版本。
5. 量档记录 `q_ref` 指 funding-record 授权身份并绑定 version；逐重 Q_amount 由同签版及 (S,ℓ) 派生，C8 载体为 manifest.chongs[i].funding.Q，不能读总 cap 替代。金额域激活复核 Q_amount≤C_amount(ℓ)；动作量 Q_units 与冻结档量另依 ADR 0049 定标，不把金额直接当 BTC quantum。

运行中利润长大按 ADR 0016:155 与 ADR 0034:29 溢出到上一级开新重，不重签；目标级 Unknown ⟹ 不触发，G 留重内。改总投入、`l_start`、容量、频率门或杠杆都签新版本，以新版本重推，只作用于生效后：新推出而现役没有的重照开；Q 变大不自动改现役 Q（输出 `AwaitingPolicy{q_rebase, (S,ℓ)}`）；变小或消失不减仓（ADR 0029:25-27）。旧版本不删，`supersedes` 指向旧版本（ADR 0034:20）。

### 五、两条验收路径

小资金单重路径（`P0`、`P1-hl-testnet`）：任一时刻只有 ⟨BTC, L0⟩ 一个重，只在 L0 买卖点全仓进出，符号改变都经过 0。多重赋格路径（`P0-acceptance-5m`）：开出 L0/L1/L2 三重，各重内符号改变都经过 0，至少一个 bar 上两个不同级别的重持相反符号，L1 重短差档量须按 ADR 0049 的数量定标记录冻结；旧 1,110,400 为金额乘系数结果，不是 BTC quantum。两条都由下游实施票验收（#1444/#1445/#1446/#1406）。

## #1610 订正：历史原件不等于数量定标

下列资金签署和历史推导原样保留。历史 `tier_quantity:1110400` 仅是 3,200,000 USDC×347/1000，量纲为金额，撤回其作为资产数量的解释；不能机械填入新数量档。新记录按 [ADR 0049](0049-funding-quantity-and-policy-reference-dimensions.md) 的 quantity_basis、冻结锚及报价尺度得到 Q_units 后再算 tier_quantity。本文不签实际报价、max_quote_age 或 quantity quantum。

新 cap 的生效检查与责任变更共用原子顺序；新 cap 小于责任拒绝整次激活，原有效 funding/cap/policy 保持，不部分切换；相等可激活，低 1 拒绝。旧版过期不因此恢复新授信。没有新增自动排队协议。

## 签署记录

```cap-record
profile_id: P0
policy_id: #1349/atomic-slot-wait/v1
version: v1
supersedes: none
cap: 101000
currency: USDC
authorizer: 用户（#1557/#1558 2026-10-20 批复）
authorized_at: 2026-10-20
valid_through: 2027-10-20

profile_id: P1-hl-testnet
policy_id: #1349/atomic-slot-wait/v1
version: v1
supersedes: none
cap: 505
currency: USDC
authorizer: 用户（#1557/#1558 2026-10-20 批复）
authorized_at: 2026-10-20
valid_through: 2027-10-20

profile_id: P0-acceptance-5m
policy_id: #1349/atomic-slot-wait/v1
version: v1
supersedes: none
scope: evidence_only
cap: 5050000
currency: USDC
authorizer: 用户（#1557/#1558 2026-10-20 批复）
authorized_at: 2026-10-20
valid_through: 2027-10-20
```

funding-record 沿用 ADR 0029:31 八字段形状（#1355 Q1），`cap` 字段承载该 profile 的总投入 `Q_total`。

```funding-record
profile_id: P0
policy_id: #1557/funding-authorization/v1
version: v1
supersedes: none
cap: 100000
currency: USDC
authorizer: 用户（#1557/#1558 2026-10-20 批复）
authorized_at: 2026-10-20
valid_through: 2027-10-20

profile_id: P1-hl-testnet
policy_id: #1557/funding-authorization/v1
version: v1
supersedes: none
cap: 500
currency: USDC
authorizer: 用户（#1557/#1558 2026-10-20 批复）
authorized_at: 2026-10-20
valid_through: 2027-10-20

profile_id: P0-acceptance-5m
policy_id: #1557/funding-authorization/v1
version: v1
supersedes: none
scope: evidence_only
cap: 5000000
currency: USDC
authorizer: 用户（#1557/#1558 2026-10-20 批复）
authorized_at: 2026-10-20
valid_through: 2027-10-20
```

量档记录（ADR 0034:14 形状），每个推出的重一条。容量域：`{1m, δ=0.5, x=10%, Y=0.9, σ=GK 中位日, 单腿}`，来源 `UserSigned(引 #917 评论 5199839236 / #915 报告 :272)`。

```policy-record
profile_id: P0
policy_id: #1355/quantity-tier-policy/v1
version: v1
supersedes: none
symbol: BTC
operation_level: L0
q_ref: #1557/funding-authorization/v1@P0
tiers: none（塔底之下无次级别）
capacities:
- level: L0  C: 1110000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L1  C: 3200000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L2  C: 9630000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L3  C: Unknown  source: none  valid_through: 2027-10-20
frequency_evidence_ref: #915 报告 :153
authorizer: 用户（#1557/#1558 2026-10-20 批复）
authorized_at: 2026-10-20
valid_through: 2027-10-20

profile_id: P1-hl-testnet
policy_id: #1355/quantity-tier-policy/v1
version: v1
supersedes: none
symbol: BTC
operation_level: L0
q_ref: #1557/funding-authorization/v1@P1-hl-testnet
tiers: none（塔底之下无次级别）
capacities:
- level: L0  C: 1110000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L1  C: 3200000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L2  C: 9630000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L3  C: Unknown  source: none  valid_through: 2027-10-20
frequency_evidence_ref: #915 报告 :153
authorizer: 用户（#1557/#1558 2026-10-20 批复）
authorized_at: 2026-10-20
valid_through: 2027-10-20

profile_id: P0-acceptance-5m
policy_id: #1355/quantity-tier-policy/v1
version: v1
supersedes: none
scope: evidence_only
symbol: BTC
operation_level: L0
q_ref: #1557/funding-authorization/v1@P0-acceptance-5m
tiers: none（塔底之下无次级别）
capacities:
- level: L0  C: 1110000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L1  C: 3200000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L2  C: 9630000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L3  C: Unknown  source: none  valid_through: 2027-10-20
frequency_evidence_ref: #915 报告 :153
authorizer: 用户（#1557/#1558 2026-10-20 批复）
authorized_at: 2026-10-20
valid_through: 2027-10-20

profile_id: P0-acceptance-5m
policy_id: #1355/quantity-tier-policy/v1
version: v1
supersedes: none
scope: evidence_only
symbol: BTC
operation_level: L1
q_ref: #1557/funding-authorization/v1@P0-acceptance-5m
tiers:
- d: 1  w: 347/1000  source: Measured{#915 BTC, 1m, δ=0.5, 中位, L0→L1}
capacities:
- level: L0  C: 1110000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L1  C: 3200000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L2  C: 9630000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L3  C: Unknown  source: none  valid_through: 2027-10-20
frequency_evidence_ref: #915 报告 :153
authorizer: 用户（#1557/#1558 2026-10-20 批复）
authorized_at: 2026-10-20
valid_through: 2027-10-20

profile_id: P0-acceptance-5m
policy_id: #1355/quantity-tier-policy/v1
version: v1
supersedes: none
scope: evidence_only
symbol: BTC
operation_level: L2
q_ref: #1557/funding-authorization/v1@P0-acceptance-5m
tiers:
- d: 1  w: Unmeasured  source: none（来源域 L0→L1 与 L1→L2 不一致）
- d: 2  w: Unmeasured  source: none
capacities:
- level: L0  C: 1110000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L1  C: 3200000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L2  C: 9630000  source: UserSigned(#917)  valid_through: 2027-10-20
- level: L3  C: Unknown  source: none  valid_through: 2027-10-20
frequency_evidence_ref: #915 报告 :153
authorizer: 用户（#1557/#1558 2026-10-20 批复）
authorized_at: 2026-10-20
valid_through: 2027-10-20
```

```derivation-input
profile_id: P0
policy_id: #1558/chong-derivation-input/v1
version: v1
supersedes: none
symbol: BTC
l_start: L0
margin_leverage: 1
frequency_gate: {F_min: 1, unit: per_side_count_in_evidence_window, one_sided_rule: NotViable, evidence_ref: #915 报告 :153}
authorizer: 用户（#1557/#1558 2026-10-20 批复）
authorized_at: 2026-10-20
valid_through: 2027-10-20

profile_id: P1-hl-testnet
policy_id: #1558/chong-derivation-input/v1
version: v1
supersedes: none
symbol: BTC
l_start: L0
margin_leverage: 1
frequency_gate: {F_min: 1, unit: per_side_count_in_evidence_window, one_sided_rule: NotViable, evidence_ref: #915 报告 :153}
authorizer: 用户（#1557/#1558 2026-10-20 批复）
authorized_at: 2026-10-20
valid_through: 2027-10-20

profile_id: P0-acceptance-5m
policy_id: #1558/chong-derivation-input/v1
version: v1
supersedes: none
scope: evidence_only
symbol: BTC
l_start: L0
margin_leverage: 1
frequency_gate: {F_min: 1, unit: per_side_count_in_evidence_window, one_sided_rule: NotViable, evidence_ref: #915 报告 :153}
authorizer: 用户（#1557/#1558 2026-10-20 批复）
authorized_at: 2026-10-20
valid_through: 2027-10-20
```

推导结果每重一行 `| (S,ℓ) | Q | C(ℓ) |`，`p0_batch` 为开局批金额。

```derivation-result
profile_id: P0
versions: funding v1, cap v1, policy v1, derivation-input v1
| (BTC,L0) | 100000 | 1110000 |
p0_batch: (BTC,L0) 100000
remaining: 0
stop: 完成
authorization_ref: v1

profile_id: P1-hl-testnet
versions: funding v1, cap v1, policy v1, derivation-input v1
| (BTC,L0) | 500 | 1110000 |
p0_batch: (BTC,L0) 500
remaining: 0
stop: 完成
authorization_ref: v1

profile_id: P0-acceptance-5m
versions: funding v1, cap v1, policy v1, derivation-input v1
| (BTC,L0) | 1110000 | 1110000 |
| (BTC,L1) | 3200000 | 3200000 |
| (BTC,L2) | 690000 | 9630000 |
p0_batch: (BTC,L0) 1110000; (BTC,L1) 3200000; (BTC,L2) 690000
tier_quantity: (BTC,L1) d=1 1110400
remaining: 0
stop: 完成
authorization_ref: v1
```

P1-hl-testnet profile 须增 `margin_leverage` 字段，按 ADR 0028:28 升 v2（内容为 v1 加 `margin_leverage: 1`），由 #1412/#1367 落地；本 ADR 不改 ADR 0028。

## 被否方案

按方向签投入额、开多头重与空头重（用户 Q1/Q2 否，ADR 0010:23 三要件无方向）；同一 (S, ℓ) 开一多一空两个重并签容量占用口径（Q2；Netting 场所上只是双份保证金的净额）；签方向份额切一份钱（ADR 0013:17）；小资金强行切多重（Q3；ADR 0016:92/:98）；起始级别写死成 L0 或 L1（Q5）；C(ℓ) Unknown 时跳级（ADR 0034:23）；未签域或证据门未满足就启用探针容量；测试网余额当 cap 或投入额（ADR 0029:33）；成本门 NotEvaluable 时挡 L0（ADR 0035:15）。

## 合法候选：探针容量来源（#1598）

`ProbeAuto` 按 ADR 0034 裁定四保留，默认仍 UserSigned。已签域并不替代窗口和不确定性带；当前证据前置未满足，不自动生效。每次新读数新版本，在途不追改。

## 未裁项

ADR 0013 裁定三/四与 ADR 0031:12 按「多头重 / 空头重」写三阶段、扩量闸门与投入保证金 M。本 ADR 把它们读作「当前在世声部为多 / 为空的重」。一重回到 0 换向时，三阶段状态与闸门进度延续还是重置，正本未裁，须另开票；不影响本 ADR 的数值。

> **★ 2026-10-21 已裁（[#1590](https://github.com/xy7365527-lang/NewChanlun/issues/1590)，[ADR 0043](0043-chong-reversal-account-continuity.md)，用户批复）**：单账延续、不重置不分账；谓词按在世声部方向取侧，`Σ M_b ≔ Σ P_b`；开新声部用哪部分钱由 policy 记录 `#1590/voice-open-funding/v1` 决定（默认 `profit_only`／`open_available`）。原文不删。

## 受影响票与代码

#1444 TB-06-C：推导与测试锁去掉方向参数、加 `l_start`，三个 profile 的签署结果作黄金值；#1445/#1446：换档激活复核与溢出阈值读 policy-record 当前版本；#1552：`q_ref` 指 funding-record；#1406：开局批按裁定四第 4 步；#1408：cap 读 cap-record、处理 `PolicyExpired`；#1412/#1367：P1 profile 升 v2；代码中的 `INITIAL_CAPITAL`（`rust/src/trading/types.rs:38-39`）、γ = 1.0（`rust/src/theta_v0/config.rs:177/:243`）只是默认值出处，实现须改读记录。

## 溯源标签

重无固定方向、按级别开：`[新缠论:推论]`（ADR 0010 三要件 + ADR 0014 裁定一/三 + 用户 Q1/Q2）。小资金单重：`[旧缠论]`（`026:36`、`035:30`）。塔底 L0：`[新缠论:选择]`（qujiantao.md:441）。频率门「≥ 1」、cap 1% 余量、有效期、USDC 1:1 记美元读数：`[新缠论:选择]`。数值：用户签署。

## #1598 批复与受影响清单

用户 2026-10-26 批复 [resolution](https://github.com/xy7365527-lang/NewChanlun/issues/1598#issuecomment-6403277308)。本次字段修订为 `[新缠论:选择]`，ROUNDTRIP 沿用已裁推论。受影响文件、固定基线与候选承接见 [清单](../../.chanlun/review-results/issue1598-policy-20261026/affected-code.md)；政策批准不代表下游已接单或实现。

## #1610 受影响清单

[用户批复](https://github.com/xy7365527-lang/NewChanlun/issues/1610#issuecomment-6403279340)；[ADR 0049](0049-funding-quantity-and-policy-reference-dimensions.md) 给出完整方法与待实施负控；[历史基线清单](../../.chanlun/review-results/issue1610-policy-20261026/affected-code.md)。本次不修改任何旧资金签署块、测试黄金或已验 /2 合同。
