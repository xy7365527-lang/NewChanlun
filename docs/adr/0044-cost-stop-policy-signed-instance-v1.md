# ADR 0044：成本停止 policy 签署记录 v1（P0 / P1-hl-testnet）

> 正本：本文件是 `#1354/cost-stop-policy/v1` 在 `P0` 与 `P1-hl-testnet` 两个 profile 上的签署值（θ 与四项成本输入的状态、来源与有效域）的正本。
> 最后裁定：#1559（2026-10-21，用户批复）
> 受影响代码清单：见「下游与受影响票」节（本 ADR 只登记数值，读取实现归 #1441 TB-02-S）

**日期**：2026-10-21
**裁定人**：用户（2026-10-21 批复 [#1559 评论 6310844461](https://github.com/xy7365527-lang/NewChanlun/issues/1559#issuecomment-6310844461)，原话「不重要你自己定吧」，委托主控按推荐默认值逐项确认）；代理起草，非作者独审
**状态**：已采纳（待 PR 合入）。签署单、考古与独审：`/Volumes/AgentStorage/decisions/1559/{draft.md,archaeology.md,review.attempt3.json}`；落地独审 `.chanlun/review-results/pbatch-land-20261021/`。

## 背景

[ADR 0035](0035-cost-stop-policy-instance-and-base-extension-evidence.md) 定了成本停止 policy 的记录形状、读法、缺项状态与有效域（`:12`/`:15`/`:24`），θ 与四项输入的值留给用户签。签署前系统恒为 `AwaitingPolicy{cost_input}`、成本门 `NotEvaluable`、停止判据只剩塔底 L0（ADR 0035:15）。本 ADR 是第一份签署记录。

教义依据：`035-第35课.md:30`【正文】「那些太小的级别，不足以让交易成本、交易误差等相对买卖点间波幅足够小，这样的操作，从长期的角度看，是没有意义的」——原文只给方向「足够小」，不给数；`.chanlun/definitions/qujiantao.md:467`「参数（费率、滑点、延迟、该级平均波幅）按场所与执行系统给，不入正本」，`:519`「终止 ＝ 成本门 ∧ 塔底；塔底 ＝ 现有 L0（线段层）」。

## 裁定

### 一、签署值与出处

| 项 | 签署值 | 出处与推理链 |
|---|---|---|
| θ | 两版均 `2/1`（下界比值至少是往返总成本的 2 倍才允许往该级延伸） | 1 是存在论下界、余量归部署层（`rust/src/trading/axiom_voice.rs:69-70`；ADR 0035:12 拒收 θ < 1；ADR 0035:27 否「θ 固定为 1」）。本仓在册的部署余量全部是 2：`theta_cost_k = 2.0`（`rust/src/trading/config.rs:668`，断言 `:1677-1680`「门槛常数沿用在册预注册值」）、`sub_cost_k = 2.0`（`:692`）。只继承倍数，不继承旧门的中位数读法（ADR 0035:27 已否）与 10 bps 常数分母。继承倍数是 `[新缠论:选择]`；同一个 2 在新读法下更严（95% 下界对四项实测成本） |
| P0 费率 | `Known`；BINANCE_SPOT、VIP0、taker、10 bps/side（定点 `1/1000`） | `analysis/data_cache/venue_fee_binance_spot_20260726.json:3-13`（含 `.sha256`）；同域费率簿仅此一份；VIP0 在两档中取较高费率、方向保守（`[新缠论:选择]`）；role 取 taker：`rust/src/theta_v0/venue_fee.rs:41`/`:112` 生产恒 Taker |
| P1 费率 | `Known`；Hyperliquid 主 DEX 永续、Tier 0 Base rate、taker、0.045%（定点 `45/100000`） | 官方页 `https://hyperliquid.gitbook.io/hyperliquid-docs/trading/fees`（抓取 2026-10-20T02:42:29Z，快照 sha256 `75e55504b1b887a89e1668a9544d09b013242747bcc29f8648a5410142a71258`）Perps Tier 0 表；新账户 14 日量为 0 ⟹ Tier 0；无 HYPE 质押与推荐码 ⟹ Base rate；官方页明写加密永续不适用 growth mode。官方页不单列 testnet 费率，按同表是 `[新缠论:选择]`；TB-08 回报 fee 字段与本值不一致即出新版本 |
| 滑点 | 两版 `Missing` | 仓内无逐笔滑点落盘（`.chanlun/review-results/issue907-cost-gate-level-20260805.md:282`）；`rust/src/theta_v0/venue_fee.rs:36` 与 `rust/src/theta_v0/config.rs` 的滑点常数明文未标定，不作输入（ADR 0035:27 否「代理下界充 Known」「缺项代 0」） |
| 延迟误差 | P0 `ProxyLowerBound`（只能判失败）；P1 `Missing` | P0：#907 报告 `:104` DELAYERR「是真实交易误差的下界」、`:161-169` 逐级表，测量域（Binance 现货 BTC 1m）与 P0 一致；ADR 0035:18 允许作 probe_run_ref。P1 跨域不得借（ADR 0035:24） |
| 持有成本 | P0 `Missing`（起始级别 L0）；P1 `Missing`（起始级别 L0），指定来源 = Hyperliquid 官方 funding 规则 + BTC 逐小时 funding 历史序列 | P1：官方 funding 页 `https://hyperliquid.gitbook.io/hyperliquid-docs/trading/funding`（抓取同上，快照 sha256 `126d1a0489517bad351787542d555a0de25339c69fd544a0d8d283ae172d234c`）每小时结算；L0 往返中位持有 553 根 1m（#907 报告 `:239`）> 1 h；funding 按整条序列计（[#62 评论 5029790605](https://github.com/xy7365527-lang/NewChanlun/issues/62#issuecomment-5029790605)）。序列按 #62 datum 格式落盘（带 sha256）后出 v2 升为 `Known`；落盘前照实为 `Missing`。P0：现货无 funding 周期，借币利率无在册来源（#62：需认证端点）；起始级别取最保守的 L0 是 `[新缠论:选择]` |
| 标的 / 粒度 | 两版 BTC / `1m` | 已签正本 [ADR 0040](0040-funding-signature-and-chong-derivation.md):39（symbol BTC）；Binance 簿 `analysis/data_cache/venue_fee_provenance.md:48` 为 BTC 现货 1m；P1 标的 = BTC（[#1283 评论 5465238861](https://github.com/xy7365527-lang/NewChanlun/issues/1283#issuecomment-5465238861)） |
| 有效期 | 签署日 + 365 天 | 已签正本 ADR 0040:45（有效期签署日 + 365 天）同批对齐 |

后果照实：两版滑点均为 `Missing`，成本门 `NotEvaluable`，阶段二不开，L0 照常（ADR 0035:15）。θ 要等 TB-08 把 P1 的滑点、延迟、持有成本升为 `Known` 后才第一次参与判定。

### 二、记录与读法

1. 两版互不引用（ADR 0035:24 跨域 Unsupported）；P2 不在本记录（#1561 已批不做 P2 真钱首发，见 [ADR 0028](0028-first-capability-verification-profile.md)「P2 处置」）。
2. #907 材料只作 `probe_run_ref`（ADR 0035:18 = `InsufficientEvidence`）；θ 不得由 2.59× 反推（`CONTEXT.md` 成本停止 policy 词条 _Avoid_）。
3. 本记录不改 ADR 0035 任何规则：Missing 不代 0、Proxy 不充 Known、θ < 1 拒收照旧；不新增第二套判据或兜底。
4. TB-08 升级任一输入来源即出新版本（`supersedes` 指向旧版），不改 θ 与波幅证据（ADR 0035:24）。
5. 波幅证据（`amplitude`）由探针按 ADR 0035:12 产出，不由签署填写；本记录不含波幅数值。
6. TB-02-S [#1441](https://github.com/xy7365527-lang/NewChanlun/issues/1441) 读取当前生效版本，输出 Enabled / Disabled / InsufficientEvidence / NotEvaluable 之一；机读文件路径与格式归 #1441 实施，本 ADR 是唯一数值来源。

## 签署记录

### P0（确定性回放）

```cost-stop-record
policy_id: #1354/cost-stop-policy/v1
profile_id: P0
version: v1
supersedes: none
symbol: BTC
granularity: 1m
tower_base_ref: L0（qujiantao.md:519 塔底；ADR 0035:15）
fee: Known  ref: analysis/data_cache/venue_fee_binance_spot_20260726.json  source_url: https://www.binance.com/en/fee/trading  retrieved_at: 2026-07-26  tier: VIP0  role: taker  rate: 1/1000
slippage: Missing
latency_error: ProxyLowerBound  probe_run_ref: .chanlun/review-results/issue907-cost-gate-level-20260805.md:161-169（DELAYERR，只能判失败）
holding: Missing  start_level: L0
theta: 2/1
evidence_gate: ADR 0022 裁定六/七
gate_status: AwaitingPolicy{cost_input} / NotEvaluable
authorizer: 用户（#1559 2026-10-21 批复）
authorized_at: 2026-10-21
valid_through: 2027-10-21
```

### P1-hl-testnet（能力验证）

```cost-stop-record
policy_id: #1354/cost-stop-policy/v1
profile_id: P1-hl-testnet
version: v1
supersedes: none
symbol: BTC
granularity: 1m
tower_base_ref: L0（qujiantao.md:519 塔底；ADR 0035:15）
fee: Known  source_url: https://hyperliquid.gitbook.io/hyperliquid-docs/trading/fees  retrieved_at: 2026-10-20  tier: Tier 0 Base rate  role: taker  rate: 45/100000
slippage: Missing
latency_error: Missing
holding: Missing  start_level: L0  designated_source: https://hyperliquid.gitbook.io/hyperliquid-docs/trading/funding（BTC 逐小时 funding 序列，落盘后升 Known）
theta: 2/1
evidence_gate: ADR 0022 裁定六/七
gate_status: AwaitingPolicy{cost_input} / NotEvaluable
authorizer: 用户（#1559 2026-10-21 批复）
authorized_at: 2026-10-21
valid_through: 2027-10-21
```

## 被否方案

- **θ 取 1**：ADR 0035:27 已否「θ 固定为 1」；1 是下界不是默认值（#1354 resolution 6153713798）。
- **连同旧引擎的中位数读法与 10 bps 常数分母一起继承 k = 2**：ADR 0035:27 已否中位数读法；新分母由四项输入构成（ADR 0035:12）。
- **拿 #907 的 2.59× 反推 θ**：ADR 0035:18 判 #907 为 `InsufficientEvidence`。
- **P1 费率取推论层的「growth mode 0.45–0.9 bp」**：[#853 评论 5224634834](https://github.com/xy7365527-lang/NewChanlun/issues/853#issuecomment-5224634834) 自陈无落盘；官方页明写加密永续不适用 growth mode。
- **滑点用未标定常数凑数**：`venue_fee.rs:36`、`venue_fee_provenance.md:46` 明文未标定。
- **P0 借用 P1 的 Hyperliquid 费率或 funding（或反之）**：ADR 0035:24 跨域 Unsupported。

## 下游与受影响票

- [#1360](https://github.com/xy7365527-lang/NewChanlun/issues/1360) TB-02、[#1366](https://github.com/xy7365527-lang/NewChanlun/issues/1366) TB-07、[#1367](https://github.com/xy7365527-lang/NewChanlun/issues/1367) TB-08：#1559 关票后 blocking 边解除；#1366 回放按「请求时刻生效版本」引本 ADR；#1367 升级 P1 滑点/延迟/funding 来源即出新版本。
- [#1441](https://github.com/xy7365527-lang/NewChanlun/issues/1441) TB-02-S：读本 ADR 的版本输出四态之一（不加边，票面验收允许「未定参数准确缺项」输出）。
- [#1602](https://github.com/xy7365527-lang/NewChanlun/issues/1602)：成本参数版本化记录以本 ADR 为数值来源。

## 溯源标签

θ 倍数、VIP0 档、testnet 按主网同表、P0 holding 起始级别 L0 为 `[新缠论:选择]`；成本门方向 `[旧缠论]`（`035:30`）；其余为已裁沿用（ADR 0035、ADR 0040）。
