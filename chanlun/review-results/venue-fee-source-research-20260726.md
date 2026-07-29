# wayfinder #285 — venue 真实费率标定数据源：现佣金模型盘点 + 各品种 venue 费率权威来源与标定口径

- 工位：worktree `/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`），AFK 只读调研，唯一产出 = 本文件。
- 调研时间：2026-07-26 04:23–04:40 UTC（仓外来源访问日期均为此日，另注者除外）。
- 姊妹票：#62 报告 `chanlun/review-results/venue-fee-data-sources-20260720.md`（Binance USDⓈ-M **永续** funding/fee/borrow/liq 数据源可得性，2026-07-21 实测）——本票覆盖**全链路现状盘点 + 各品种（含美股 OKLO）venue 费率表 + 标定口径**，不重复 #62 的 funding 抓取方案。
- 纪律：090（查不到的明说，不编数字）；v3（本票不涉及策略有效性断言）。

## 结论先行

1. **现状**：全系统佣金只有**一种形态**——per-notional 单边合并费率 `fee_rate = (commission_bps + slippage_bps + tax_bps)/10⁴`，rust 默认 3bp/side（`config.rs:233-235`），Python 侧三个互不一致的常数（10bp/5bp/20bp）+ 一批完全无费的回测。**无 maker/taker 之分、无阶梯、无最低佣金、无 per-share 口径、无监管费项**。
2. **来源**：BTC（Binance spot）与 OKLO（美股，执行假设 = IBKR——仓内实盘集成就接的 IBKR）两条线的官方费率表**本次已核到一手数字**（Binance spot 费率页、IBKR 佣金/融资利率页、SEC/FINRA 费率）；Binance 永续 maker/taker 静态表、CME/ICE 期货费率**未核到**（列入 §4 待核）。
3. **推荐口径**：`ExecConfig` 扩一个 `Option<FeeSchedule>` 按品种 venue 费率档（datum 文件 + sha256 版本哈希），旧三常数保留作未标定 fallback——与 `FundingScheduleBook`/`[L2费率标定: datum <hash>]` 既有升口径契约（`risk.rs:709`）同构，None ⟹ 现状 bit-exact。

---

## 1. 现状盘点（file:line 证据）

### 1.1 rust 侧佣金实装：单源三常数，per-notional 单边

**配置点（唯一）**：`rust/src/theta_v0/config.rs:218-237`

```rust
pub struct ExecConfig {
    pub entry_delay_bars: u32,   // default 1
    pub commission_bps: f64,     // default 1.0  [设计选择;L3经验待标定]
    pub slippage_bps: f64,       // default 2.0  L3
    pub tax_bps: f64,            // default 0.0  L3
}
```

- 默认合计 **3bp/side**（1+2+0），注释自标 `[设计选择;L3经验待标定]`（:221）。
- **无 CLI / 配置文件 override**：`rust/src/bin/theta_backtest.rs:34-46` 只收 `<SYMBOL> [START END]`；全仓 grep 无任何外部注入 `commission_bps` 的通道。

**费用公式与消费点（全部同一式，f64 域）**：

| 位点 | 行号 | 形态 |
|---|---|---|
| `backtest/fill.rs` | :46-48 | `fee_rate = (commission+slippage+tax)/10_000`；`apply_fill` 平仓费 :197、开仓费 :240 `fee = 成交名义 × fee_rate`；含费成本基 :243 |
| `strategy/exec.rs` | :73-84 | `apply_fees`：tick 域变体——成交价 ±`base·bps/10⁴` round 到 tick（买上调/卖下调）；注释明确 slippage 是唯一买卖不对称项 |
| `backtest/runner.rs` | :230-231, :308-309, :534-535, :699-700 | 四处同一 fee_rate 式注入各执行臂；`fill.rs` 2026-07-21 自 runner 纯移动（fill.rs:1-4），runner:66 `pub(crate) use` 门面 |
| `backtest/treasury.rs` | :25-27 | `pub fn fee_rate(exec) = 同式`；默认恰 3e-4 有**冻结测试**（:93-97 `default_fee_rate_is_3e4`） |
| `backtest/dual_ledger.rs` | :94-95, :130-133 | 双账本 fee 独立逐笔测得，进 R 分解守恒断言 |
| `strategy/overlay_state.rs` | :512-520, :566-573 | 声部账本同口径 `fee = qty·px·fee_rate`，`cum_fee` 守恒锚（:393-394） |
| `backtest/l3_delta_r_alpha.rs` / `econ_positive.rs` / `bin/pure_bsp_timing.rs` / `bin/pi_bsp_timing.rs` | :247 / :234,:4441 / :84 / :520 | 同式散布（改口径需全改或收敛到单源） |
| `strategy/mod.rs` | :581 | sizing 侧 `cost_per_unit` **v0 占位 0.0**（"成交费用由 exec::apply_fees 施加；每单位成本模型待 L3 标定"） |

**形态小结**：多空对称、按成交名义额（qty×px）的单边比率费；commission/slippage/tax 三项**合并为一个标量**进 fill（不可分别归因到 venue 科目）；`exec.rs` 的 tick 域变体把滑点体现为成交价偏移、与 `fill.rs` 的现金扣费是**两套并存表达**（同一参数源）。

### 1.2 持有成本（funding/borrow/liq）：机制真实装、费率保底未标定

- `strategy/risk.rs:818-846` `CostModel { funding_rate_per_period, funding_period_bars, borrow_rate_per_bar, liq_penalty_rate }`，fail-loud 构造。
- 保底值：`backtest/wverify_run.rs:1067-1075` `CostModel::new(0.0001, 480, 0.000001, 0.005)`——**自述"Binance 永续近似"**：funding 1bp/8h(=480 根 1m bar)、borrow 0.01bp/bar、强平罚金 0.5%。
- 强制标签：`risk.rs:750-754` `RATE_UNCALIBRATED_LABEL = "[L1机制/费率未标定]"`——带成本 R 数值禁作 alpha 论据/策略择优输入；datum 注入后升 `[L2费率标定: datum 版本哈希]`（:709，不得跳级）。
- datum 承接位已实装：`risk.rs:697-748` `FundingScheduleBook`（分段带符号 funding 快照簿，`as_of` 零前视）；`risk.rs:878-896` `funding_accrual_signed` 预冻结接口。**费率标定是已登记的 L2 缺口，本票即是其前置调研。**

### 1.3 nautilus 集成路径

- `rust/src/theta_v0/nautilus/backtest_engine.rs:113-131`：`SimulatedVenueConfig` venue=`BINANCE`、Cash 账户、L1_MBP、起始 1M USDT；instrument `BTCUSDT.BINANCE`（:38-44）。
- **未显式配置任何 fee model**——费用行为取决于 nautilus-backtest 0.60.0（crates.io 依赖，`rust/Cargo.toml:24-33`）的默认 FeeModel。该默认行为**本次未核**（§4）；worktree 内 `nautilus_trader/` 目录无内容可查。

### 1.4 Python 侧口径（互不统一，照实陈列）

| 位点 | 常数 | 口径 |
|---|---|---|
| `analysis/full_system_backtest.py:59` | `FEE_RATE = 0.001` | **10bp/side** per-notional（:467,612,675-679,729-732 买卖双边含费价）；:818 报告自述 "0.1%手续费"；同文件 :909 另一节又"无滑点/手续费，等权分配" |
| `scripts/three_stage_backtest.py:82` | `FRICTION = 0.0005` | **5bp**/次，注释"佣金+滑点，日线 QQQ"（配合 LEVERAGE=2.0 融资假设，:79-82） |
| `scripts/position_manager.py:194` | `friction_cost = 0.002` | **20bp**/次，注释"佣金+印花税+滑点"（A 股口吻：印花税） |
| `analysis/fugue_btc_backtest_1min.py:235` | — | 明确"**不含**手续费/滑点/资金费率" |
| `analysis/fugue_complete_backtest_1min.py:1072` | — | "无滑点/手续费建模"（fugue 系列普遍无费） |
| `analysis/cl_1s_maker_model_backtest.py:29-32,65,73` | `F_TAKER = 1.45e-4` | CL 期货专场：taker 1.45bp/侧（1 tick 点差+佣金"在册物理下限"）；IBKR maker +0.04bp/侧；CME maker rebate −0.2~−0.5bp/侧（量级假设，未引来源） |

### 1.5 数据品种清单与 venue 假设

**`analysis/data_cache/` 现存 2 个文件**（本 worktree 实测）：

| 品种 | 文件 | 来源（脚本证据） | venue 假设 | 窗口 |
|---|---|---|---|---|
| BTCUSDT | `btc_1m_full.json` | `scripts/download_btc_binance.py:38-40`：`data.binance.vision/data/spot/monthly/klines/BTCUSDT/1m` | **Binance 现货** | 2017-08-17 → 2026-05-31，4,613,599 根 1m bar |
| OKLO | `oklo_1m_databento.json` | Databento；路由规则 `src/newchan/data_databento.py:99-103`：非期货 → **`XNAS.ITCH` raw_symbol**（Nasdaq TotalView，UTP 含 NYSE 上市股） | 美股（OKLO 为 NYSE 上市；数据走 Nasdaq UTP 通道） | 2024-05-10 起 |

**`rust/src/theta_v0/backtest/data.rs:55-62` 声明 8 品种**：BTC / ES / CL / GC / BRN / DX / QQQ / OKLO（全部 1min，"databento / Binance 归档"，:10）。其中 ES/CL/GC/BRN/DX/QQQ 六个文件**现存不在仓**（本 worktree data_cache 只有上表 2 个）；其来源在案：CME `GLBX.MDP3`（`_FUTURES_MAP`，`data_databento.py:32-46`）、BRN=`IFEU.IMPACT`、DX=`IFUS.IMPACT`（`scripts/fetch_1m_databento_10y.py` 头注释）、QQQ=`XNAS.ITCH`。

**执行侧 venue 假设**：

- 美股实盘数据源/执行 = **IBKR**：`src/newchan/cli.py:18-19` `--source {ibkr,av}` 默认 `ibkr`；`src/newchan/b_chart.py` IBKR 连接 UI（:653-731）。⟹ OKLO 标定口径以 IBKR 费率表为一手来源（§2.2）。
- BTC 回测执行假设 = Binance（nautilus venue 名即 BINANCE，§1.3）；**但价格数据是现货**而 CostModel 保底自述"永续近似"——见下条。

### 1.6 口径裂缝（盘点中发现的，照实登记）

1. **spot 价格数据 × perp 成本模型**：`btc_1m_full.json` 是 Binance **spot** klines；`wverify_run.rs:1067` 的 funding 保底却按"Binance 永续近似"。现货无 funding——标定时须先裁定 venue 假设（用 spot 费率+无 funding，还是承认策略假设的是 perp 而用 #62 的 perp funding datum + perp 费率）。这不是 bug 申报，是标定前的口径前置裁定项。
2. **rust 3bp vs Python 10/5/20bp vs fugue 0**：同一体系四套摩擦口径并存；老 Python 脚本多为研究态，rust 是唯一被冻结测试锁住的口径（`treasury.rs:93-97`）。
3. **滑点与佣金合并且不可归因**：`fee_rate` 单一标量进 R 分解的 `commission_slippage` 项（`risk.rs:930`）；venue 标定只能标"佣金"，滑点本质是价差/冲击 datum，宜分科。

## 2. 权威来源（URL + 访问日期 + 关键数字）

### 2.1 BTC → Binance 现货费率（已核，一手）

- URL：<https://www.binance.com/en/fee/trading>（Spot & Margin 表），访问 2026-07-26。
- **Regular User（VIP0，30 日成交额 < $1M）：maker 0.1000% / taker 0.1000%**（= **10bp/side**）；BNB 抵扣 25% 后 **0.075%/0.075%（7.5bp/side）**。
- 阶梯：VIP1（≥$1M + ≥25 BNB）0.0900%/0.1000%；VIP3（≥$20M）0.0400%/0.0600%；VIP9（≥$4B）0.0110%/0.0230%。USDC 交易对 taker 另档 0.0950%。
- 粒度可得性：maker/taker 分立 ✅、VIP 阶梯 ✅、无最低佣金概念、无监管费。**对现 3bp 默认值：taker 10bp 是其 3.3 倍。**
- 借贷利率（现货杠杆场景）：`GET /sapi/v1/margin/interestRateHistory` 需认证、30 天/窗（#62 §1.5 已实测证）。

### 2.2 BTC → Binance USDⓈ-M 永续（部分在案，本次未取新数）

- funding 数据：#62（2026-07-21 实测）已证**全史免费可得**（API `/fapi/v1/fundingRate` 自 2019-09-13 = 上市全史；vision 归档 2020-01→2026-06，7119 条），8h 周期、cap/floor ±0.003。
- maker/taker：仅认证端点 `/fapi/v1/commissionRate`（当前账户快照，无历史）；**静态 VIP0 表本次未取得**——`binance.com/en/fee/futureFee` 页面 JS 渲染，FetchURL 只取到监管声明 boilerplate（090：照实标注未核，不引二手数字）。
- 强平清算费：无公开程序化源，费率表人工核（#62 §1.6）。

### 2.3 OKLO → IBKR 美股（已核，一手；执行假设对齐仓内 IBKR 集成）

佣金页：<https://www.interactivebrokers.com/en/pricing/commissions-stocks.php>，访问 2026-07-26。

| 计划 | 费率 | 最低/单 | 上限 | 第三方费 |
|---|---|---|---|---|
| IBKR Pro **Tiered** | 月量 ≤30 万股 **$0.0035/股**；300k–3M $0.0020；3M–20M $0.0015；20M–100M $0.0010；>100M $0.0005 | $0.35 | 1% 成交额 | 监管+交易所+清算+pass-through 全另计 |
| IBKR Pro **Fixed** | **$0.005/股** | $1.00 | 1% 成交额 | 仅监管费另计 |
| IBKR **Lite** | **$0**（限美国居民、限交易所上市股） | — | — | 仅监管费另计 |

监管与清算费（同页公布）：

- **SEC Section 31**：`0.0000206 × 卖出总额`（**$20.60/百万，仅卖出**）；
- **FINRA TAF**：`$0.000195/股`（仅卖出，**上限 $9.79/笔**）；
- FINRA CAT：$0.000003/股；NSCC/DTC 清算：$0.00020/股（Tiered 另计）；
- Pass-through：NYSE = 佣金×0.000175；FINRA = 佣金×0.000565。

融资利率页：<https://www.interactivebrokers.com/en/trading/margin-rates.php>，访问 2026-07-26。

- USD **IBKR Pro**：BM+1.5%（余额 ≤$100k）→ BM+1%（≤$1M）→ BM+0.75%（≤$50M）→ BM+0.5%；**IBKR Lite**：BM+2.5% 全档。页面当时公布绝对值 5.130%（Pro Tier I，隐含 BM≈3.63%）。
- 逐日计提、次月第三个工作日入账；地板 0.75%；>$250M 未预安排融资 +1% 附加。
- 换算参照（声明：换算是我算的，非来源原文）：OKLO 价格量级 $20–40/股（仓内数据 2024-05 起 $18 附近）——Tiered $0.0035/股 @$20 ≈ **1.75bp/side**，+清算 0.1bp，卖出再 +SEC 0.206bp +TAF ≈0.1bp；**合计约 2–2.5bp/side（Pro Tiered 首档，小额单受 $0.35 最低佣金主导）**。现 3bp 默认对此 venue 量级巧合接近，但结构（per-share + 最低佣金 + 卖出监管费）与 per-notional 单标量完全不同。

### 2.4 OKLO 备择 → Alpaca（部分核到）

- 官方费率表 PDF：<https://files.alpaca.markets/disclosures/library/BrokFeeSched.pdf>（revised 2024-12-19），访问 2026-07-26。
- 核到：**自营零售账户"in general, we do not charge a commission"**（美股零佣金）；监管费（SEC/TAF/CAT 等）按交易另收 pass-through。表格具体数字在 PDF 抽取中丢失——监管费数字以 §2.5 官方来源为准，Alpaca 自身加价项未逐项核到（§4）。
- 定位：若未来执行假设切折扣 API 券商，Alpaca 是候选；但仓内实盘集成就绪的是 IBKR，故 Alpaca 仅备择。

### 2.5 监管费官方来源（交叉核实）

- **SEC Section 31 = $20.60/百万，2026-04-04 起生效**（FY2026 费率咨询；此前 2026-04-03 前为 $0.00/百万）：SEC 公告转载 <https://mondovisione.com/media-and-resources/news/sec-section-31-transaction-fee-rate-advisory-for-fiscal-year-2026-2026228/>（2026-02-27）；Federal Register 令 <https://www.federalregister.gov/documents/2026/03/04/2026-04233/order-making-fiscal-year-2026-annual-adjustments-to-transaction-fee-rates>；FINRA 信息通告 <https://www.finra.org/rules-guidance/notices/information-notice-20260317>。与 IBKR 页 0.0000206 一致。
- **FINRA TAF = $0.000195/股、上限 $9.79/笔，2026-01-01 起**（自 $0.000166/$8.30 调整）：FINRA 规则书 <https://www.finra.org/rules-guidance/rulebooks/corporate-organization/section-1-member-regulatory-fees>（ finra.org 直访本环境 403，数字取自该页搜索快照 + Schwab/E*TRADE 等多券商一致披露 + IBKR 页一致）。

### 2.6 声明品种（ES/CL/GC/BRN/DX/QQQ）——未核

仓内 `data.rs:55-62` 声明但数据文件现存不在仓；本次未取 CME/ICE/Nasdaq 官方费率页。仓内既有的唯一量级参照是 `cl_1s_maker_model_backtest.py:29-32`（CL taker 1.45bp/侧"在册物理下限"、IBKR maker +0.04bp、CME rebate −0.2~−0.5bp——脚本自述，未引一手来源）。列入 §4 待核。

## 3. 推荐标定口径

### 3.1 承接模块与配置形态

1. **rust `ExecConfig` 扩字段，不删旧字段**（config.rs:218-237）：

   ```rust
   pub struct ExecConfig {
       pub entry_delay_bars: u32,
       pub commission_bps: f64,   // 保留：未标定 fallback（L1 标签不动）
       pub slippage_bps: f64,     // 保留：滑点另立 datum 前继续承担
       pub tax_bps: f64,
       pub fee_schedule: Option<VenueFeeSchedule>,  // 新增：None ⟹ 现状 bit-exact
   }
   ```

   `VenueFeeSchedule` 按品种分档：`{ venue, symbol, maker_bps, taker_bps, per_share_usd: Option<f64>, min_commission_usd: Option<f64>, sell_reg_bps: f64, vip_tier: Option<String>, datum_sha256 }`。**per-share 与 per-notional 两种单位都要能表达**——OKLO/IBKR 是 per-share + 最低佣金，压成纯 per-notional 会丢掉 $0.35/单最低档在小单上的主导效应；BTC/Binance 是纯 per-notional maker/taker。

2. **datum 落盘**：`analysis/data_cache/venue_fee_<venue>_<yyyymmdd>.json`，带 `source_url`/`retrieved_at_utc`/`datum_sha256`——照搬 #62 §4 的列式+哈希约定与 `*_provenance.md` 溯源件；报告标签按 `risk.rs:709` 契约升 `[L2费率标定: datum <hash 前 12 位>]`，**不得跳级**。

3. **消费点收敛**：当前 fee_rate 公式散布 10+ 位点（§1.1 表）——实装时以 `treasury.rs:25-27` 的 `fee_rate()` 为单源门面，各消费点改调 `fee_rate_for(symbol, side, liquidity)`；`fill.rs`/`runner.rs` 四处/`exec.rs`/各 bin 逐步收编。冻结测试 `treasury.rs:93-97`（默认 3e-4）锁的是 fallback 路径，不受新档影响。

4. **maker/taker 区分需要执行层表态**：现 fill 全部按 bar close 成交，无挂单概念——初档标定建议**全按 taker**（保守，市价成交假设）；maker 档只在 cl_1s_maker 类限价执行模型实装的臂启用。

### 3.2 与现有佣金模型的迁移关系

| 旧件 | 处置 |
|---|---|
| `commission_bps/slippage_bps/tax_bps` 三常数 | **保留**为 `fee_schedule=None` 时的 fallback，L1 未标定标签照旧；冻结测试不动 |
| `CostModel` 保底（wverify_run.rs:1067-1075） | funding 由 #62 datum 注入 `FundingScheduleBook`；borrow 对 Binance 永续照实 N/A（#62 §1.5 裁定）；liq 人工核表后改常量+哈希 |
| Python 各常数（10/5/20bp） | 不迁移、不作权威——rust datum 为唯一费率源；老脚本报告里的费率声明保留历史语境 |
| nautilus 臂 | 先核默认 FeeModel 行为（§4），再决定是否显式配 `MakerTakerFeeModel` 与 datum 对齐 |

### 3.3 建议初值（来源可追溯；均为 taker 保守档）

| 品种 | venue | maker/taker (bp/side) | 附加项 | 来源 |
|---|---|---|---|---|
| BTCUSDT | Binance spot VIP0 | 10 / 10（BNB 档 7.5/7.5 作第二档） | 无 funding（spot）；杠杆另核借贷利率 | §2.1 |
| BTCUSDT perp | Binance UM VIP0 | 待核（§4）；funding 用 #62 datum | liq 罚金人工核表 | #62 + §2.2 |
| OKLO | IBKR Pro Tiered 首档 | $0.0035/股（≈1.75bp @$20） | 卖出 +SEC 0.206bp +TAF $0.000195/股(cap $9.79) +CAT；min $0.35/单；融资 BM+1.5% 起 | §2.3 |
| OKLO（备择） | IBKR Pro Fixed / Lite | $0.005/股 min $1 / $0（限美国居民） | 监管费同 | §2.3 |

滑点：保留 `slippage_bps` 未标定（或另立 spread datum——Binance spot BTCUSDT 1m 档价差与 OKLO 美股价差量级差异大，不宜共用一个常数；本票不展开）。

## 4. 未决与不确定项（照实）

1. **Binance USDⓈ-M 永续 maker/taker 静态费率表未核到**（官网页 JS 渲染，本环境只取到 boilerplate）；可得路径在案 = 认证端点快照（#62 §1.4）。未引任何二手数字。
2. **nautilus-backtest 0.60.0 默认 FeeModel 行为未核**——§1.3 路径当前费用行为不明（可能零费，可能默认 maker/taker 档），实装前必须查 crate 源码钉死。
3. **Alpaca 费率表具体数字未逐项核到**（PDF 表格抽取丢失）；仅核到"零售零佣金 + 监管费 pass-through"定性。
4. **FINRA 官网直访 403**：TAF 数字取自 finra.org 规则书搜索快照 + 多券商一致披露 + IBKR 页，三方一致但缺一次直连核验。
5. **CME/ICE 期货（ES/CL/GC/BRN/DX）与 QQQ ETF 费率全部未核**——仓内声明品种，数据文件现存不在仓；实装排期时须补 CME Globex/ICE 费率页 + IBKR 期货佣金页（仓内 cl_1s_maker_model_backtest.py:29-32 的量级假设可作交叉校验起点，但本身未引来源）。
6. **OKLO per-share → bp 换算依赖价格档**：§2.3 的 1.75bp 是按 $20 参考价算的说明性换算，不是费率本身——配置形态必须原生支持 per-share（§3.1）。
7. **spot vs perp venue 前置裁定未决**（§1.6 裂缝 1）：CostModel 的 funding 项是否对本数据窗生效，取决于裁定 BTC 执行假设是现货还是永续——这超出本调研票职权，登记待裁定。
8. **数据窗未覆盖分红/拆股/借券费**：OKLO 做空腿的 borrow fee（美股借券费率，IBKR 按票逐日报价）无任何公开历史源，本次未核；多头口径不受影响。
