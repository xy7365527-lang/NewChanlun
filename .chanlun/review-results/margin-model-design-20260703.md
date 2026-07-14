# D2 真保证金模型设计 v2（阶段1 纯设计，只读）

- **工位**：ws-margin（topo_address: swarm/ws-margin，parent_callback: main）
- **任务**：TaskCreate #103（v1 设计）+ #111（v2 修订）
- **日期**：2026-07-03（v2）
- **上游裁定**：`codex-p2-design-ruling-20260702.md` §D2（fail）→ `codex-margin-ruling-20260703.md`（conditional-pass，3致命+2重要缺口）
- **状态**：纯设计（只读），**实装等 team-lead 排工**。本文件不改任何生产代码。
- **认识论等级**：本设计文档 = 设计层（L0 定义 + L1 规则一致性方案）。文档本身不产出 L2/L3 经验结论。

## v2 修订摘要（对照 codex-margin conditional-pass）

| # | 缺口 | 修订位置 |
|---|------|---------|
| ① | 历史快照时间对齐缺失（用最新快照跑历史=时间错配） | §1.1 + 新增 §2.7 MarginScheduleBook |
| ② | 单位错误：net_notional 已是美元，公式二次乘 mark_price | §2.2（去掉 ·mark_price） |
| ③ | liquidation_flag 签名缺 equity 参数（判据 E≤MM 不可算） | §2.4（补 equity 参数） |
| ④ | M2/M3 枚举值算对但无下游消费者（订单流不变=半成品） | 新增 §2.8 KThetaRiskGate 扩展（纳入实装范围） |
| ⑤ | risk_mode 输入无校验（NaN/负 buffer/未排序 bracket 静默退化 Normal） | 新增 §2.9 fail-loud 构造期校验 |
| ⑥ | 影响声明只提 alpha 冻结，遗漏 RunResult 全字段 + L3 管线 | §4（补全下游产物清单） |
| ⑦ | 三选择项落裁定 | §7（buffer 归 Θ_risk+敏感性网格 / 默认 one-way / CME-simple 标签） |

---

## 0. 问题定位：D2 为什么 fail，翻转条件是什么

codex 裁定的**否定核心**（非误判）：D2 原案「`maint_margin = 名义仓位 × config 保证金率`」是**引入一个从未经真实数据校准的新 Θ_risk 参数**——把交易所真实的**分级维持保证金表**坍缩成一个拍脑袋的标量 `config.margin_rate`。这违反 formalization-validity-domain（231号）：把定义域（sizing 已有的名义仓位公式）套用到有效域外（真实保证金约束需要真实交易所规则，非缠论/sizing 参数可导）。

codex 给出的**翻转条件**（裁定 §3 边界条件 D2）：

> 「若团队有明确计划接入真实 venue/broker 的保证金数据源（而非 config 常数反推），D2 判定翻转为可行；纯 config 常数反推的方案维持 fail。」

**本设计的存在论移动**：`maint_margin` 不是 Θ_risk 参数，而是**交易所公开规则表的版本化快照**——与「价格」同等的**基准数据（datum）**，不是可调旋钮。

| 维度 | codex 否定的 D2 原案 | 本设计 |
|------|---------------------|--------|
| MM 来源 | `config.margin_rate`（单个虚构标量） | 交易所公布的分级维保表快照（Binance leverageBracket / CME margins） |
| 认识论 | 未校准 Θ_risk 参数（声明膨胀） | L1 数据摄入（可对照公布规则验证，非自由参数） |
| 可否证 | 否（无对照对象） | 是（golden vector 对照交易所公布公式） |
| 分级 | 无（单一率坍缩全表） | 保留分级 bracket 结构（notional↑ ⟹ MMR↑） |

这与 no-patch-mentality（090号）一致：不是在虚构率上打补丁，而是删除虚构率、用真实规则表替换。

---

## 1. 数据源调研

### 1.1 BTC 永续（Binance USDⓈ-M，代表 crypto perp）

**规则形态**：分级（tiered/bracket）维持保证金系统。每档含：名义 bracket（USDT 区间）、该档最大杠杆、维持保证金率 MMR、维持速算额（maintenance amount，简化分档计算的固定扣减项）。

**维持保证金公式**（交易所公布，逐仓/one-way）：
```
MM_t = Σ_bracket-crossed  → 实用简化式：MM_t = notional · MMR(tier) − maint_amount(tier)
```
其中 tier 由当前名义 `notional = mark_price · |q|` 落入哪个 bracket 决定。

**BTCUSDT 典型档位结构**（方向性描述，**具体数字随交易所更新，实装时必须快照**）：最小仓位档最大 125x 杠杆、MMR 起于约 0.4%，随名义增大逐档抬升（0.5% → 1% → 2.5% → 5% …），最大杠杆逐档下降。

**权威来源**：
- 交易规则页：`binance.com/en/futures/trading-rules/perpetual/leverage-margin`
- 交易参数页：`binance.com/en/futures/trading-parameters/perpetual/leverage-margin`
- **精确当前值**：`GET /fapi/v1/leverageBracket`（返回 BTCUSDT 每档 bracket 上下界、MMR、maint_amount）——**实装取数的权威端点**。

**★关键约束（诚实）**：Binance 公告明确「These values change over time」——在极端行情下会调整最大杠杆/bracket/MMR。故规则表是**带快照日期的版本化常量表**，不是永久常量。实装时从 leverageBracket 端点拉一次，落为 `MarginSchedule { venue, instrument, snapshot_date, source_url, brackets: [...] }`，快照日期入版本。

### 1.2 CME 期货（BTC/MBT，代表受监管期货）

**规则形态**：CME Clearing 用 **SPAN** 组合风险场景法——覆盖至少 99% 的预期价格变动，输入含历史/前瞻波动率、流动性、相关性。故**美元数额随 BTC 价格与波动率频繁变化**，不是固定常数。

**可硬编码的公布形态**（百分比基准，历史口径，subject to change）：
- 标准 BTC（5 BTC/合约）：维持保证金历史约名义的 **37%**；初始保证金 = 维持的 100%（hedger）/ 110%（speculator）。维持/日内保证金历史 > $100,000/手。
- Micro BTC（MBT，0.1 BTC/合约）：约 $2,000/手。
- **零售加成**：CFTC/CME/NFA 归类零售为 "Heightened Risk Profile"，交易所保证金 +10%。
- 券商（FCM）可在交易所最低要求上追加。
- 抵消：对冲头寸有 margin credit（如 BTC↔MBT 1:50 抵消）。

**权威来源**：
- 标准 BTC：`cmegroup.com/markets/cryptocurrencies/bitcoin/bitcoin.margins.html`
- Micro BTC：`cmegroup.com/markets/cryptocurrencies/bitcoin/micro-bitcoin.margins.html`

**★简化取舍（v0）**：SPAN 全场景引擎超出 v0 范围（L2 延后，见 §2.4）。v0 用**百分比×名义**的公布口径（维持率 % × 合约名义 × 手数，零售 +10%），带 snapshot_date。这是公布规则的忠实转录，不是自造场景引擎——诚实标注为「CME 简化口径（非 SPAN 全场景）」。

### 1.3 缺口诚实清单

- **portfolio margin / SPAN 全场景**：v0 不实装组合风险场景引擎（多空对冲.pdf §10.3：CME SPAN 按组合情景评估潜在损失）。v0 仅支持 one-way/net 与 hedge 两种账户模型的**逐档/逐腿**保证金。SPAN 组合抵消保证金标为 L2 延后。
- **ADL（自动减仓）/ 资金费驱动强平**：Binance 除 E<MM 外还有 ADL、资金费扣减触发。v0 `liq_flag` 仅建模 `E_t ≤ MM_t` 的价格触发，ADL/funding 标为缺口。
- **借券费/替代股息（股票短卖）**：多空对冲.pdf §3/§10.3 指出股票短卖涉及借券、利息、替代股息成本。v0 标的为 BTC perp/futures，无股票短卖，此缺口对当前标的不 binding，但接入股票时必须补。

---

## 2. 模型设计

### 2.1 数据模型：AccountState 扩展（设计，不实装）

现状 `strategy/mod.rs:119` `AccountState { nav: f64, voice_qty: Vec<u32> }`。保证金模型**不改 AccountState 结构**，而是新增一个**只读派生函数**，从 `(持仓, mark 价格, 保证金表)` 计算 `RiskModeInput`——理由：`RiskModeInput`（risk.rs:315）已是良定义的输入结构，`risk_mode()`（risk.rs:333）机制已完整正确。D2 的缺口**纯粹是输入接线**（runner.rs:489-496、exit.rs:108-117 硬编码 0/false），不是机制缺陷。ponytail：不新建类型，填已有的 `RiskModeInput` 五字段。

新增（设计签名）：
```
// 从当前持仓 + mark 价 + 保证金表，派生 RiskModeInput（真实账户层输入）。
fn margin_inputs(
    positions: &[VoiceNotional],   // 已有类型（risk.rs:437），含 side + notional_mag
    equity: f64,                   // mark-to-market 账户权益 E_t（runner fill 侧，与 sizing 的 nav 同源）
    schedule: &MarginSchedule,     // 交易所规则表快照（新增数据结构）
    caps: &RiskCushions,           // B1/B2 缓冲（Θ_risk，见 §2.3）
    account_model: AccountModel,   // OneWayNet | HedgeMode（多空对冲.pdf §10）
) -> RiskModeInput
```

新增数据结构（**数据表，非 Θ 参数**）：
```
struct MarginSchedule {
    venue: Venue,            // Binance | Cme
    instrument: String,      // "BTCUSDT" | "BTC" | "MBT"
    snapshot_date: Date,     // 快照日期（版本化，诚实：规则会变）
    source_url: String,      // 权威来源（可回溯）
    brackets: Vec<MarginTier>,       // Binance 分级
    // 或 pct_maint: f64, contract_mult: f64（CME 百分比口径）
}
struct MarginTier { notional_floor: f64, notional_cap: f64, mmr: f64, maint_amount: f64, max_leverage: f64 }
```

### 2.2 maint_margin 计算链（从持仓 + 规则表推导，非拍脑袋）

**Binance 分级 / one-way·net 账户模型**（多空对冲.pdf §10.1：`N = Σ σ_v q_v`，只看净敞口）：
```
1. net_notional N_t = net_notional(voices)   （直接复用 risk.rs:478；结果已是美元，勿再乘价）
2. tier = brackets 中满足 notional_floor ≤ N_t < notional_cap 的档   （partition_point 二分）
3. MM_t = N_t · tier.mmr − tier.maint_amount
```

**★v2 单位修正（codex 致命缺口②）**：`VoiceNotional.notional_mag`（risk.rs:437 文档「单位美元」）已完成 `M_v·P_v·q_v` 折算，`net_notional()`（risk.rs:478）对这些美元值求和取绝对值，**结果已是美元**。v1 公式 `|Σ signed_notional| · mark_price` 对已是美元的量二次乘价 → 量纲错（美元→美元·价格）。修正：直接用 `net_notional(voices)`，不再乘 `mark_price`。§1.1 的「`notional = mark_price·|q|`」是**构造 `notional_mag` 时**（价×手数×乘数）的一次折算，发生在填 `VoiceNotional` 之前；MM 计算链消费的是已折算的美元值，不重复折算。

**Binance 分级 / hedge 账户模型**（多空对冲.pdf §10.2：position book 存 (Q⁺,Q⁻)，腿的保证金可分开）：
```
MM_t = Σ_v [ |n_v| · mmr(tier_v) − maint_amount(tier_v) ]   逐腿分档求和
```
（腿身份/保证金/执行/止损可分开，但价格 PnL 仍线性抵消——多空对冲.pdf §10.2 boxed。故 hedge 模式毛敞口双腿都吃保证金，与 gross_notional risk.rs:470 对齐。）

**CME 百分比口径 / one-way**：
```
MM_t = pct_maint · (contract_mult · mark_price · |contracts|) · retail_multiplier(1.10)
```

**账户模型选择即绑定 §绝对资本方案C 的 B_2**：多空对冲.pdf §10.3 表——one-way 只看净敞口 / hedge 腿可分但价格 PnL 线性抵消 / portfolio 按组合风险。v0 支持前两者，后者 L2 延后。**同单位数双开在 one-way 净额化为 0 → MM 低；hedge 毛额双腿 → MM 高**（对齐 risk.rs `net_le_gross` + `hedged_gross_high_net_zero` 已证性质）。

### 2.3 buffer1/buffer2 的诚实定位（Θ_risk，非交易所数据）

**关键区分**（避免重蹈 D2 覆辙）：`maint_margin`/`liq_flag` 来自**真实交易所规则**（数据），但 `buffer1`/`buffer2`（去杠杆/只平仓缓冲）是**策略自己在维持线之上的减仓垫**——**是 Θ_risk 设计参数**（与已有 ρ/γ/β/κ 同类），交易所不公布「你该在维持线上方多少开始去杠杆」。

- **绝对资本.pdf §方案A/§5 协变形式**：B1/B2 必须写成**协变资本单位**下的无量纲比率 `B̄_i = B_i / U_ℓ`，保尺度不变（对齐 risk.rs:311 注释「`E_t < MM_t + B1` 等价归一化 `Ē_t < MM̄_t + B̄_1`」，两侧同除 U_ℓ 对消）。故 B1/B2 以「MM 的倍数」或「U_ℓ 的比率」声明，不用裸美元绝对值（裸绝对值 = 绝对资本.pdf §1 不相容定理的特权尺度，破坏自相似）。
- **诚实声明**：B1/B2 是 Θ_risk 参数，**不声明经验校准**（L2 still-MISSING，同 risk.rs:521 杠杆上限的诚实标注）。它们不由「config 常数反推 MM」——它们本就是策略层的减仓阈值，Θ_risk 的合法成员。这与 codex 否定的「反推虚构 MM 率」是**不同的对象**：codex 否定的是把交易所的 MM 造假；B1/B2 是策略的自主减仓垫，从来就是 Θ。

### 2.4 liq_flag 的产者（补 codex 指出的「不存在的模块」）

codex 裁定 §D2：「`liq_flag` 声称由模拟撮合产，但回测侧不存在任何模拟撮合生产 liq_flag 的模块或契约——D2 把问题转移给一个不存在的模块」。

**本设计补上命名产者**（**v2 修正 codex 致命缺口③：补 equity 参数**）：
```
// 模拟保证金引擎：对持仓 + mark 价 + 权益应用交易所强平规则，产 liq_flag。
fn liquidation_flag(positions: &[VoiceNotional], equity: f64, mark: f64, schedule: &MarginSchedule) -> bool
```
**v0 强平谓词**（单 venue 逐仓 BTC perp）：`liq_flag ⟺ equity ≤ MM(positions, schedule)`。v1 签名 `(positions, mark, schedule)` **缺 equity**，无法算所声称的判据 `E≤MM` ——自相矛盾。修正：补 `equity` 参数。

**★更干净的方案（采纳）**：`liquidation_flag` **不单独暴露**，而由 `margin_inputs()`（§2.1）统一算 `MM_t` 与 `liq_flag` 并填进 `RiskModeInput`——单一数据源，避免签名分叉不自洽。`margin_inputs` 已持有 `equity/positions/schedule` 全部入参，内部 `liq_flag = equity ≤ MM_t`。独立 `liquidation_flag` 仅作可选内部 helper（若拆分则带全三参）。

此时 M1 的两析取项 `liq_flag ∨ E_t < MM_t` 在 v0 退化为等价（`≤` vs `<` 边界外一致）。`liq_flag` 独立字段保留是为**未来扩展**（ADL/funding/跨 venue 触发，届时 liq_flag ≠ E≤MM）——v0 诚实标注「liq_flag = 价格触发子集，ADL/funding 缺口」。产者是 `SimMarginEngine`，不再是「不存在的模块」。

### 2.5 与 GAP3 注资流 / TW 三阶段账本的合并边界

**唯一读依赖（275号局部依赖 + 674号裁决C 双账本分离）**：保证金模型**只读** `equity E_t`（runner fill 侧 mark-to-market，与 sizing 的 `nav` 同源，strategy/mod.rs:120）与 `positions`（voice_qty），**产** `RiskModeInput`。它**不修改** `LedgerComp`（R=Π−A−W）、`TwState`（free/holding/withdrawn）、`AssemblyState` 任何字段。

- **E_t 的语义**：保证金要的是**盯市账户权益** = free_cash + holding 的 mark 市值 − 借款。这**既不是** R（会计收益，income view）**也不精确等于** TW（TW 在成本口径下 notional-守恒）。E_t 来自 runner fill 侧的盯市 NAV（sizing 已消费的同一 `nav`）。
- **三阶段取本金的影响是单向读**：取本金 W 减少账户可用权益 → E_t 下降 → MM headroom 收窄。但保证金模型**不需要知道 stage 机制**——它只读「当前 E_t（取本金后）」与「当前 positions」，按 §2.2 算 MM。stage 推进由 TW 机管，保证金模型是其**下游读者**（局部依赖：保证金依赖 E_t 值，不依赖 stage 转移逻辑）。
- **不违反 674号裁决C**：双账本仍显式分离，保证金模型不引入任何双向同构转换，只读标量 E_t。

### 2.6 RiskMode 五态触发条件（逐态公式，机制已在 risk.rs:333 实装）

机制**已完整正确**（risk.rs `risk_mode`，Lean `risk_mode_complete_unique` Σ𝟙=1）。本设计只是给出**真实输入接线后**每态的触发公式（MM_t 现为真实分级值，B1/B2 为 Θ_risk 协变缓冲）：

| 态 | 名称 | 触发公式 | 触发后行为 |
|----|------|---------|-----------|
| M0 | Insolvent | `E_t ≤ 0` | GlobalRiskClose → 全局平根仓（级联全平） |
| M1 | Liquidation | `¬M0 ∧ (liq_flag ∨ E_t < MM_t)` | GlobalRiskClose → 全局平仓 |
| M2 | Deleverage | `¬M0∧¬M1 ∧ E_t < MM_t + B1` | 限增仓（G(q')≤G(q_t)，strict §12），不强平 → **须经 §2.8 gate 通道** |
| M3 | CloseOnly | `¬M0∧¬M1∧¬M2 ∧ E_t < MM_t + B2` | 只许平仓，不许开新仓 → **须经 §2.8 gate 通道** |
| M4 | Normal | 以上皆否 | 正常交易 |

前提 `0 < B1 < B2`（strict §11，risk.rs:332 边界条件）。GlobalRiskClose 仅 {M0,M1} 触发（risk.rs:370，strict §16 P1），M2/M3 限增仓不强平——**接入真实 MM 后这三态首次可达**（现状 MM=0 使 M1/M2/M3 恒不可达，只 M0 由 E≤0 可达）。

**★v2 关键（codex 重要缺口④）**：M2/M3 在 `risk_mode()` 层可达 ≠ 改变订单流。现有 `KThetaRiskGate`（coverage.rs:1919）只有 `force_flat/stop_long/stop_short` 三字段，`k_theta_risk_gate`（runner.rs:489）只把 `global_risk_close(mode)`（=M0/M1）接进 `force_flat`——**M2/M3 无任何下游消费者**，接入后只是「算对枚举值但订单流不变」的空转。故 M2/M3 接线扩展**纳入 §2.8 实装范围**（否则半成品，conditional-pass 不能升 pass）。

### 2.7 历史快照时间对齐（codex 致命缺口①）

**问题**：交易所规则「These values change over time」。用**最新**快照跑历史区间 = 时间错配（历史 bar 用了当时不存在的规则）。

**修正——分段快照簿 `MarginScheduleBook`**：
```
struct MarginScheduleBook { snapshots: Vec<MarginSchedule> }  // 各 MarginSchedule 带 effective_from/effective_to
impl MarginScheduleBook {
    // 回测 bar 的时间戳 → 该时刻生效的快照（as-of 查找，禁未来快照泄漏）
    fn as_of(&self, bar_ts: Timestamp) -> &MarginSchedule   // effective_from ≤ bar_ts < effective_to
}
```
`margin_inputs()` 按当前 bar 时间戳取 `book.as_of(bar_ts)`，不用全局最新表。**zero-lookahead**：`as_of` 硬禁 `effective_from > bar_ts` 的快照进入（与回测零前视一致）。

**诚实的有效域降级（若拿不到历史分段）**：Binance leverageBracket 端点只返回**当前**表，历史 bracket 需交易所公告考古（可能不全）。若只能取单快照，则**限定有效域**：诚实声明「本快照仅对 `snapshot_date` 之后的 OOS 段 L1 有效；之前的回测段 = as-of forward 近似，不声称历史 L2 精度」——把有效域限死在快照日之后，不假装历史精确（formalization-validity-domain 231号：有效域 ≠ 定义域）。二者取一由数据可得性决定，实装时明确标注选了哪个。

### 2.8 M2/M3 订单流接线（KThetaRiskGate 扩展，纳入实装范围）

**现状**（coverage.rs:1919-1943）：`KThetaRiskGate.caps(cap)` 把 `force_flat→(0,0)`、`stop_long→hi=0`、`stop_short→lo=0`。M2/M3 无对应字段。

**扩展（最小改动）**：`KThetaRiskGate` 加**一个** `no_increase_cap: Option<f64>` 字段——M2/M3 时 = 当前净持仓幅度 `|net_t|`（美元或 lot，与 cap 同单位）：
```
pub struct KThetaRiskGate { force_flat: bool, stop_long: bool, stop_short: bool,
                            no_increase_cap: Option<f64> }   // v2 新增：M2/M3 净幅上限
fn caps(&self, cap) -> (lo, hi):
    if force_flat { return (0,0) }                    // M0/M1 优先（不变）
    let cap = match no_increase_cap { Some(c) => cap.min(c), None => cap };  // M2/M3：净幅不得超当前
    (stop_short?0:cap, stop_long?0:cap)
```
**为什么一个字段够**（ponytail）：strict §12 M2 去杠杆 = `G(q')≤G(q_t)`，M3 只平仓 = 净幅不增；在 **one-way/net**（v0 默认账户模型，§7 裁定 b）两者都坍缩为「净持仓幅度 `|net|` 不得超过当前」——因为 one-way 下任一订单非增即减净幅，"不许开新仓"="净幅不增"。M2 与 M3 的差异（去杠杆须主动减 vs 只平仓可持有）在 `≤` 形式约束下同为「上限=当前幅度」，v0 不细分（M2 的严格「主动减」延后，标 gap）。

`k_theta_risk_gate`（runner.rs:489）接线：`mode==Deleverage|CloseOnly ⟹ gate.no_increase_cap = Some(current_net_magnitude)`。**这一改才让 M2/M3 真改订单流**（feasible_candidates 的 hi/lo 被压到当前幅度 → 不再产增仓候选）。

// ponytail: M2/M3 在 one-way 坍缩为同一 no_increase_cap；hedge 模式需分别约束毛/净，届时再拆字段。

### 2.9 输入校验（fail-loud，codex 重要缺口⑤）

**问题**：`risk_mode()`（risk.rs:333）对 NaN/负 buffer/未排序 bracket 无防线。NaN 参与所有比较判假 → 落 `else` → **静默退化 Normal**（数据污染冒充「一切正常」，风控最危险的静默失败）。

**修正——构造期校验，fail-loud 不静默**：`MarginSchedule`/`RiskCushions` 的构造函数（或 `margin_inputs` 入口）显式 reject：
```
// MarginSchedule::new / RiskCushions::new 返回 Result，非法输入即 Err（禁静默）
- equity/mark/mmr/maint_amount/buffer 任一 NaN 或 ∞  → Err
- buffer1 < 0 或 buffer2 < 0 或 ¬(0 < B1 < B2)        → Err（strict §11 前提）
- brackets 未按 notional_floor 升序 / 有重叠/空洞     → Err（二分查找前提）
- mmr ∉ (0,1] 或 maint_amount < 0                    → Err
```
回测入口构造一次即校验；`risk_mode()` 内层保持纯函数（不重复校验，输入已在边界净化）。**原则**：污染在**系统边界**拦截并报错（coding-style「validate at system boundaries / fail fast」），不让它流进风控判定后静默变 Normal。

---

## 3. 校准与验证方案（L1 规则一致性）

**L1（规则一致性，非 L2 盈利）测试设计**——验证「转录的规则表 = 交易所公布规则」，可否证管线转录错误，**不**验证任何盈利假设：

### 3.1 MM golden-vector 对照测试
从交易所公布文档取 3-5 个 `(instrument, notional, 期望 MM)` 已知点，硬编码期望值，断言模型重算一致：
```
// Binance BTCUSDT：取 leverageBracket 快照的某档，独立按公布公式 MM = N·mmr − maint_amount 重算
assert_eq!(margin_schedule.maint_margin(notional=100_000), 100_000·mmr − maint_amount);  // 对照公布档
// CME BTC：pct 口径 + 零售加成
assert_eq!(cme_schedule.maint_margin(1 contract), 0.37·(5·mark)·1.10);
```
断言点须来自**交易所文档的公布数字**（golden），不是模型自产（否则同义反复，L0 零增量）。

### 3.2 强平价对照测试
断言 `liquidation_flag` 恰在交易所**文档记载的强平价**处翻转：
```
// 逐仓 BTC perp：强平价 = 使 E_t = MM_t 的 mark。断言 mark 略高于强平价 → false，略低 → true。
assert!(!liquidation_flag(pos, mark_above_liq, sched));
assert!( liquidation_flag(pos, mark_below_liq, sched));
```

### 3.3 账户模型区分测试（多空对冲.pdf §10 核心）
同单位数双开，断言 one-way 与 hedge 的 MM 不同：
```
// long k + short k：one-way net=0 → MM 低（近 0）；hedge gross=2k → MM 高（双腿分档求和）
assert!(mm_oneway(double_open) < mm_hedge(double_open));
```
对齐 risk.rs 已证 `hedged_gross_high_net_zero`（net=0, gross=2k）。

### 3.4 五态可达性测试（接入后首次可达）
构造 `E_t` 落入每个 `[MM, MM+B1), [MM+B1, MM+B2), …` 区间，断言 risk_mode 返回对应态——验证 M1/M2/M3 在真实 MM 下**可达**（现状 MM=0 时这些区间为空）。

### 3.5 buffer 敏感性网格（codex 三选择项(a) 强制项）

buffer1/2 归 Θ_risk（§7 裁定 a），但**必须暴露其影响**——否则「归 Θ_risk」只是把自由参数诚实藏起来。设计一组敏感性测试：在 `(B1, B2)` 网格上（如 B1∈{0.1,0.2,0.3}·MM、B2∈{0.3,0.5,0.8}·MM，满足 0<B1<B2）跑同一回测段，记录每格的 **M2/M3 触发次数、forced_close 次数、n_orders、equity_curve 末值** 的变化范围。目的不是选「最优 buffer」（那是 L2 校准，不做），而是**报告 buffer 扰动 → 订单流/触发时机的敏感区间**，让 buffer 的自由度可观测（认识论：这是 L1 管线敏感性，不是 L2 参数校准）。

**认识论标注**：§3.1-3.4 全是 **L1**（规则转录正确性 + 管线正确性，golden 来自交易所文档）；§3.5 是 L1 敏感性诊断（暴露 Θ 自由度，非 L2 校准）。**均不含 L2**——不验证「这套保证金规则在真实回测里盈利」，那需真实数据回测（§4）。

---

## 4. 对回测的影响评估（口径变化，强制标注）

**接入前**：MM=0/buffer=0/liq_flag=false（runner.rs:489-496、exit.rs:108-117）→ M1/M2/M3 **恒不可达**，只 M0（E≤0）可达 → Deleverage/CloseOnly/Liquidation 从不触发。

**接入后**：
1. **Liquidation(M1) 可触发** → GlobalRiskClose → 强制全局平仓（级联，risk.rs:370）。
2. **Deleverage(M2)/CloseOnly(M3) 可触发** → 限增仓/只平仓（strict §12 可行集 G(q')≤G(q_t)）。
3. ⟹ **改变订单流 π** → 改变实际成交/平仓序列 → **改变 μ̂ 估计管线的输入分布**。

**codex 定性确认**：这与 p1 报告「补输入流，不改判定逻辑」的定性**不符**——实际影响范围**覆盖到 alpha 测量结果**（codex 裁定 §D2）。

**formalization-validity-domain（231号）强制口径声明**：
- 接入真实 MM 后的 π 与接入前的 π 是**不同的订单流**。任何用 MM=0 退化保证金算出的 μ̂/alpha 结果，是在**另一个 π** 上测的。
- ⟹ **接入使此前所有 MM=0 口径下的 alpha 冻结失效**，必须在真实保证金口径下**重跑并重新冻结**，且结果标注「保证金口径：真实分级 vs 退化 MM=0」。
- 不接受「MM=0 的旧结果继续用」——那是口径混用（务实思维，161号禁止）。

**v2 补全：受影响的下游产物清单（codex 缺口⑥——不止 alpha 冻结一项）**。π 变化后必然联动失效、须同步重跑重冻结并标口径的具体产物：

| 产物 | 位置/字段 | 为何失效 |
|------|----------|---------|
| `RunResult.trade_pnls_with_forced` | 回测输出 | forced_close（M1 强平）新增/改变 → PnL 序列变 |
| `RunResult.daily_returns` / `equity_curve` | 回测输出 | 订单流变 → 权益轨迹变 |
| `trades.forced_close` | 逐笔标记 | M1 触发的强平笔首次出现 |
| `RunResult.n_orders` / `is_l2` | 回测输出 | M2/M3 限增仓 → 订单数变；is_l2 口径随之 |
| μ̂ 估计管线输入 | econ_positive.rs / classifier | 交易分布变 → μ̂ 输入分布变 |
| L3 report 管线 | `l3_fullwindow.rs` 等 | 全窗 alpha/perm_p 在新 π 上须重算 |

**残留路径标注**：`closed_loop_final` 若仍固定走 Normal 分支（未接 margin_inputs），须在实装中**显式标注为「尚未接线的残留路径」**——避免与已接线路径（runner/exit）口径混用（一半真实 MM、一半 MM=0 的混合口径是无效的）。

---

## 5. 原文锚点（PDF 页码引用）

### 5.1 绝对资本.pdf（`docs/formal-chain/绝对资本.pdf`，标题「推导完全分类」，12 页）
- **p6 §方案C「把资本约束也分类进去」**：优先级绑定 regime `B_0 无资本约束绑定 / B_1 总杠杆绑定 / **B_2 保证金绑定** / B_3 三阶段提现约束绑定 / B_4 短差同股数不可行 / B_5 借券/执行约束绑定`，优先级互斥化 `B̂_i = B_i ∧ ⋀_{j<i} ¬B_j`，`Σ𝟙[B̂_i]=1`。**这是「保证金约束」作为一个优先级互斥绑定态的权威锚点**（与 RiskMode 五态同构的分类哲学）。
- **p2-4 §方案A「把资本改成协变量」**：引入级别资本单位 `U_ℓ(x)>0`，`U_{ℓ+k}(S_k x)=a_k U_ℓ(x)`，全部资本变量无量纲化 `Ī=I/U_ℓ, W̄, R̄, Ā`。**B1/B2 协变缓冲的形式依据（§2.3）**。
- **p8 §5「三阶段资本约束的正确自相似写法」**：`W_t<I_0, R_t≥R_⋆+ℒ^wc` 改归一化 `W̄_t<Ī_0, R̄_t≥R̄_⋆+ℒ̄^wc`。**E_t/MM_t 比较归一化（risk.rs:311）的原文依据**。
- **p1 §1 不相容定理**：`非平凡尺度等变 + 固定绝对资本上限 + 全定义域策略等变` 三者不能同时成立。**为什么 MM/buffer 不能用裸美元绝对数（须协变/无量纲）的根因**。

### 5.2 多空对冲.pdf（`docs/formal-chain/多空对冲.pdf`，标题「推导完全分类」，16 页）
- **p12 §10.1-10.3「支持多空双开的保证金系统」**：账户模型三分类表——`one-way/net (N=Q⁺−Q⁻，只看净敞口) / hedge mode ((Q⁺,Q⁻)，腿可分开但价格 PnL 仍线性抵消) / portfolio margin (组合风险情景，保证金按风险/抵消计算)`。**§2.2 账户模型分叉 + §1.3 SPAN 缺口的权威锚点**。CME SPAN = 组合风险场景法。
- **p8 §6.3「毛保证金归一化的 overlay return」**：`M_t^overlay` 可取子腿毛名义资金/初始保证金/维持保证金/压力损失预算/portfolio margin 增量保证金。**MM 的多种口径来源**。
- **p3 §3「§9 双开抵消定理」**：`父仓保持 + 同单位数反向双开 ⟹ 对父子腿在组合净值级别不产生正价格收益`；短卖涉及借券、保证金、利息、替代股息成本，保证金账户有维持要求。**hedge 模式毛敞口双腿吃保证金（§2.2）+ 借券费缺口（§1.3）的依据**。

---

## 6. 结果包（六要素）

1. **结论**：D2 真保证金模型设计——`maint_margin`/`liq_flag` 从**交易所公布规则表的版本化快照**（数据 datum，非 Θ 参数）派生，`buffer1/2` 诚实归为 Θ_risk 协变缓冲；接入 `RiskModeInput` 五字段（机制 risk.rs:333 已完整），补 `SimMarginEngine::liquidation_flag` 命名产者；账户模型分 one-way/hedge（SPAN L2 延后）。此设计满足 codex 翻转条件（真实 venue 数据源，非 config 反推）。

2. **定义依据**：绝对资本.pdf p6 §方案C（`B_2 保证金绑定` 优先级互斥态）+ §方案A/§5（协变资本单位，B1/B2 无量纲化）；多空对冲.pdf p12 §10（账户模型三分类 → MM 按 net/gross/组合风险聚合）；risk.rs:315 `RiskModeInput` 契约 + risk.rs:333 `risk_mode` 五态穷尽互斥（Lean `risk_mode_complete_unique`）。真实分级表满足「有效域=定义域」——因为 MM 是可对照公布规则验证的数据，不是声明膨胀的参数。

3. **边界条件（结论翻转条件）**：
   - 若 codex/编排者裁定「buffer1/2 也必须来自交易所公布数据而非 Θ_risk」→ 本设计的 §2.3 定位翻转（但交易所不公布策略减仓垫，此裁定与现实不符）。
   - 若 v0 标的从 BTC perp/futures 扩到**股票短卖** → §1.3 借券费/替代股息缺口从「不 binding」翻转为「必须补」，MM 模型不完整。
   - 若要求 **portfolio margin/SPAN 全场景** → §2.4 逐档/逐腿 MM 不足，需组合风险场景引擎（L2 延后项被激活）。
   - 若 `liq_flag` 被要求建模 **ADL/funding 触发** → v0 `liq_flag ⟺ E≤MM` 退化不足。

4. **下游推论**：接入后 M1/M2/M3 首次可达 → π（订单流）变化 → μ̂ 输入分布变化 → **所有 MM=0 口径下的 alpha 冻结失效，须真实保证金口径重跑重冻结并标注口径**（§4，231号强制）。GlobalRiskClose 在真实 MM 下可被 Liquidation 触发（此前只 Insolvent）。

5. **谱系引用**：formalization-validity-domain（231号）——MM 数据 vs Θ 参数的有效域区分是本设计的核心（避免重蹈 D2 覆辙）；no-patch-mentality（090号）——删除虚构 config 率、用真实规则表替换，非在虚构率上打补丁；674号裁决C——双账本分离，保证金模型只读 E_t 不引入双向同构；275号局部依赖——保证金模型只依赖 E_t 值不依赖 stage 转移逻辑。codex-p2-design-ruling-20260702.md §D2 是本设计的直接上游否定。

6. **影响声明**：本文件**纯设计，未改任何生产代码**。实装（team-lead 排工）将影响：`strategy/risk.rs`（新增 `margin_inputs`/`MarginSchedule`/`MarginScheduleBook`/`SimMarginEngine` + 构造期 fail-loud 校验 §2.9）、`strategy/coverage.rs:1919`（`KThetaRiskGate` 加 `no_increase_cap` 字段 + `caps()` 扩展 §2.8）、`backtest/runner.rs:489-496`（替换 0/false 占位 + M2/M3 接 no_increase_cap）、`strategy/exit.rs:108-117`（替换占位）、`strategy/mod.rs`（AccountState 派生链，结构不变）。**改动 μ̂ 输入分布 → 影响 §4 全部下游产物（RunResult 全字段 + L3 管线）**（须重冻结并标口径）。

---

## 7. 三选择项裁定（codex-margin-ruling-20260703.md 已裁，v2 落定）

| 项 | 裁定 | 理由（codex） | 本设计落地 |
|----|------|--------------|-----------|
| (a) buffer1/2 归属 | **归 Θ_risk，不另立 EmpiricalDomain，但强制敏感性网格** | 归类诚实（交易所不公布策略减仓垫）；但 buffer 直接决定 M1/M2/M3 边界，须暴露影响，否则只是把自由参数藏起来 | §2.3 归 Θ_risk 协变参数 + §3.5 敏感性网格测试（强制） |
| (b) 账户模型默认 | **one-way/net** | 现有 `AccountState`（mod.rs:119）+ Nautilus adapter 均净仓骨架，hedge 需额外双腿簿记；one-way 最小改动 | §2.2 默认 one-way；§2.8 gate 的 no_increase_cap 在 one-way 下单字段够；hedge 显式 opt-in |
| (c) CME 简化口径 | **条件接受，强制「CME-simple」口径标签** | 百分比×名义可作 v0，但产出须显式标注非 SPAN/portfolio/FCM 实盘保证金，避免声明膨胀（090号） | §1.2 标注「CME 简化口径（非 SPAN 全场景）」；实装时任何报告/日志带 `CME-simple` 标签 |

三项均已 codex 裁定，无待编排者上浮项——实装按此执行。
