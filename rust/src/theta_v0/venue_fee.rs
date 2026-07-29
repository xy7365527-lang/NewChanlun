//! venue 真实费率标定 datum（#360 实装；规格 = `chanlun/review-results/venue-fee-source-research-20260726.md` §3）。
//!
//! ## 本模块解决的问题
//!
//! 改动前全系统佣金只有**一种形态**：`fee_rate = (commission+slippage+tax)bps/1e4` 的
//! per-notional 单边标量（`config::ExecConfig` 三常数，默认 3bp/side，标签
//! `[设计选择;L3经验待标定]`）。真实 venue 费率**不是**这个形态：
//!
//! - Binance 现货：per-notional，但 maker/taker 分立 + VIP 阶梯（VIP0 = 10bp/side，
//!   BNB 抵扣档 7.5bp/side，报告 §2.1 一手数字）；
//! - IBKR Pro 美股：**per-share**（$0.0035/股）+ **每单最低佣金**（$0.35）+ 名义额上限（1%）
//!   + 卖出监管费（SEC Section 31 按卖出额、FINRA TAF 按股带笔上限）+ 清算/CAT 按股
//!   + pass-through 按佣金比例（报告 §2.3/§2.5）。把它压成纯 per-notional 会丢掉最低佣金
//!   在小单上的主导效应（本模块测试 `ibkr_min_commission_dominates_small_order` 即其物证）。
//!
//! 故本模块提供**两种计费单位**（[`FeeUnit`]）的原生表达，datum 落盘 + sha256 版本哈希，
//! 由 [`FeeQuoter`] 在成交点解析为该笔成交的**等效单边费率**。
//!
//! ## 为什么解析成"等效费率"而不是"绝对费用"
//!
//! 账本侧（`backtest::fill`/`strategy::overlay_state`/`backtest::dual_ledger`）的全部现金流与
//! 含费成本基都写成 `px·(1 ± δ·fee_rate)` 的**费率**形态，守恒断言亦在该形态上成立。若改传
//! 绝对费用需在账本内并存两套算术（no-patch-mentality 禁止的"渐进式回避"）。等效费率
//! `fee_usd/(qty·px)` 是同一事实的无损改写：**per-notional 档按 bps/1e4 直接给**（与未标定
//! fallback 逐位同形，无除法误差），**per-share 档才做一次除法**。于是账本一行不改，
//! `fee_schedule = None` ⟹ 与改动前**逐位相同**。
//!
//! **溯源件**：`analysis/data_cache/venue_fee_provenance.md`（#374 LOW-F）——在册 datum 的
//! 来源 URL / 取数日期 / sha256 / 列式约定 / 有效域，与两份 JSON 及其 `.sha256` sidecar 同源。
//!
//! ## 有效域（231号 formalization-validity-domain）
//!
//! **L2（费率标定）**：datum 是 venue 官方公布费率表的版本化快照（URL + 访问日期 + sha256），
//! 不是模型估计。但——
//!
//! - 本模块只标定**佣金/监管/清算**科目。`slippage_bps` 仍是未标定常数（滑点本质是价差/冲击
//!   datum，报告 §1.6 裂缝 3 登记为另立 datum，不在本票）——故标定档的成交费率是
//!   **datum 费率 + slippage_bps/1e4**（[`FeeQuoter`] 的 `uncalibrated_addon`）：只替换被
//!   datum 覆盖的科目，不把滑点一起抹掉。`tax_bps` 在标定档下必须为 0（`treasury::fee_quoter`
//!   fail-loud）——两个在册 venue 的交易税费科目已由 datum 逐项承载，再叠一个笼统 tax 会重复计；
//! - maker/taker：现执行层全部按 bar close 成交、无挂单概念 ⟹ 生产路径**一律 Taker**
//!   （报告 §3.1 item 4 的保守初档，★#423 起有编译期常量载体
//!   [`PRODUCTION_LIQUIDITY_ROLE`]，其文档逐条登记了它锁住/锁不住什么）。
//!   [`LiquidityRole::Maker`] 档存在于 datum 但生产不取；
//! - 未入簿品种（ES/CL/GC/BRN/DX/QQQ、Binance 永续）= 报告 §4 的未核项，[`VenueFeeBook::resolve`]
//!   对其返回 `Err`（fail-loud，禁静默退化到某个"差不多"的档）。
//!
//! ## 口径标签契约（risk.rs:709/759，不得跳级）
//!
//! `fee_schedule = None` ⟹ 报告标 [`RATE_UNCALIBRATED_LABEL`](super::strategy::risk::RATE_UNCALIBRATED_LABEL)；
//! `Some(datum)` ⟹ 升 `[L2费率标定: datum <sha256 前 12 位>]`（构造子
//! [`rate_calibrated_label`](super::strategy::risk::rate_calibrated_label)）。

use super::strategy::exec::FillSide;

/// 流动性角色（venue 费率表的 maker/taker 维）。
///
/// 生产成交路径**恒为 [`Taker`](LiquidityRole::Taker)**：现执行层按 bar close 市价成交，
/// 无限价挂单语义 ⟹ 声称 maker 档是声明膨胀（090）。Maker 档只在真限价执行模型实装后启用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiquidityRole {
    Maker,
    Taker,
}

impl LiquidityRole {
    /// **角色 → per-notional 单边 bps 的唯一选择处**（★#423 收尾轮 E）。
    ///
    /// 抽出理由（纯重构，费率逐位不变）：同形 `match role { Maker => maker_bps, Taker => taker_bps }`
    /// 此前在 [`VenueFeeSchedule::fee_usd`] / [`VenueFeeSchedule::effective_rate`] /
    /// [`VenueFeeSchedule::constant_effective_rate`] 各写一份（Repeated Switches）——三份同形分支
    /// 的一致性靠人眼维持，新增角色枚举值时要改三处。收口后角色维的解读只有此一处。
    ///
    /// 只服务 [`FeeUnit::Notional`] 档：per-share 档没有"单边 bps"这个量（其费率是 (qty, px, side)
    /// 的非线性函数），故不在本方法的定义域内——那一档的角色相关项（仅卖出监管费）由
    /// [`FeeUnit::PerShare`] 分支按 `side` 处理，与角色无关。
    fn pick_bps(self, maker_bps: f64, taker_bps: f64) -> f64 {
        match self {
            LiquidityRole::Maker => maker_bps,
            LiquidityRole::Taker => taker_bps,
        }
    }
}

/// **生产成交路径的撮合角色**——编译期常量，单一来源（★#423）。
///
/// 上方 [`LiquidityRole`] 的"生产恒 Taker"此前只是**文档声明**，各成交点各写一个
/// `LiquidityRole::Taker` 字面量。本常量把该事实变成一个可被机器消费的取值：
/// [`VenueFeeSchedule::constant_effective_rate`] 的良定义判定与
/// [`FeeQuoter::production_quote_or_fallback`] 的成交报价都引用它选角色，
/// 于是"角色固定"不再是人工声明而是编译期事实。
///
/// **本常量锁住什么**（★#423 第二阶段 C 线收口后的实际强度）：
/// - `FeeQuoter` **没有任何接收 [`LiquidityRole`] 的 `pub` 方法**（角色版
///   `rate_with_role` 已收为私有）⟹ 生产成交点（`backtest::fill` 的 `order_fee_rate` /
///   `leg_fee_rate` / `forced_flatten_fee_rate`、`strategy::overlay_state::quote`）只能调
///   两个无角色参数门面：只需费率时调 [`FeeQuoter::production_rate_or_fallback`]，需要
///   treasury 科目时调 [`FeeQuoter::production_quote_or_fallback`]。在这些位点写角色
///   字面量**无参可传 ⟹ 编译错误**（不是测试失败，是编不过）；
/// - 那两个文件已不再 `use` [`LiquidityRole`]（生产区段），路径 `LiquidityRole::Taker`
///   本身即未解析符号；
/// - 单标量成本口径的分叉判定（[`VenueFeeSchedule::constant_effective_rate`]）取角色亦只经此处。
///
/// **锁不住什么**（照实登记，090）：
/// - 绕过 `FeeQuoter` 直接调 [`VenueFeeSchedule::effective_rate`] / [`VenueFeeSchedule::fee_usd`]
///   并传角色字面量——这两个方法是**费率表自身**的算术，按 datum 的 maker/taker 维参数化是其
///   定义的一部分（测试与逐笔对拍需要），故保留角色参数。这条绕行同时会绕掉 `FeeQuoter` 的
///   滑点 addon 与量/价合法性判定，但**没有编译期屏障**拦它；
/// - 改锁本身：把本常量改成 `Maker`、或给 quoter 加回一个接收角色的 `pub` 方法。前者由本模块
///   测试 `production_quote_takes_role_from_single_constant`（非对称档上可判别）与
///   `production_role_is_taker` 拦住；后者无自动屏障。
pub const PRODUCTION_LIQUIDITY_ROLE: LiquidityRole = LiquidityRole::Taker;

/// per-share 档（IBKR Pro 美股）的完整费项（报告 §2.3/§2.5 一手数字）。
///
/// 全部字段单位：`*_usd` = 美元，`*_frac` = 无量纲比例（非 bp）。全部非负有限，
/// 由 [`VenueFeeBook`] 构造时 fail-loud 校验。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerShareFees {
    /// 佣金：$/股（IBKR Pro Tiered 首档 0.0035）。
    pub commission_per_share_usd: f64,
    /// 每单最低佣金（$0.35）。
    pub min_commission_usd: f64,
    /// 佣金上限 = 成交额的该比例（IBKR 1% ⟹ 0.01）。**上限压最低佣金**（小额单 1% < $0.35 时取 1%）。
    pub max_commission_frac_of_notional: f64,
    /// NSCC/DTC 清算：$/股，双边（Tiered 另计）。
    pub clearing_per_share_usd: f64,
    /// FINRA CAT：$/股，双边（来源页按股列，未标仅卖出）。
    pub cat_per_share_usd: f64,
    /// SEC Section 31：卖出成交额 × 该比例（FY2026 = 0.0000206），**仅卖出**。
    pub sell_sec_fee_frac: f64,
    /// FINRA TAF：$/股，**仅卖出**。
    pub sell_taf_per_share_usd: f64,
    /// FINRA TAF 每笔上限（$9.79）。
    pub sell_taf_cap_usd: f64,
    /// 交易所 pass-through：佣金 × 该比例（NYSE 0.000175）。
    pub passthru_exchange_frac_of_commission: f64,
    /// FINRA pass-through：佣金 × 该比例（0.000565）。
    pub passthru_finra_frac_of_commission: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct PerShareFeeBreakdown {
    commission: f64,
    passthru: f64,
    clearing_cat: f64,
    sec: f64,
    taf: f64,
    min_commission_hit: bool,
    notional_cap_hit: bool,
    taf_cap_hit: bool,
}

impl PerShareFeeBreakdown {
    fn total(self) -> f64 {
        (self.commission + self.passthru + self.clearing_cat) + (self.sec + self.taf)
    }
}

impl PerShareFees {
    /// per-share 费则的单一算术真源；绝对费用与 treasury 科目报价共同消费本分解。
    fn breakdown(self, qty: f64, notional: f64, side: FillSide) -> PerShareFeeBreakdown {
        let raw = self.commission_per_share_usd * qty;
        let cap = notional * self.max_commission_frac_of_notional;
        let commission = raw.max(self.min_commission_usd).min(cap);
        let passthru = commission
            * (self.passthru_exchange_frac_of_commission + self.passthru_finra_frac_of_commission);
        let clearing_cat = (self.clearing_per_share_usd + self.cat_per_share_usd) * qty;
        let (sec, taf, taf_cap_hit) = if side == FillSide::Sell {
            let taf_raw = self.sell_taf_per_share_usd * qty;
            (
                notional * self.sell_sec_fee_frac,
                taf_raw.min(self.sell_taf_cap_usd),
                taf_raw > self.sell_taf_cap_usd,
            )
        } else {
            (0.0, 0.0, false)
        };
        PerShareFeeBreakdown {
            commission,
            passthru,
            clearing_cat,
            sec,
            taf,
            min_commission_hit: raw < self.min_commission_usd && self.min_commission_usd <= cap,
            notional_cap_hit: cap < raw.max(self.min_commission_usd),
            taf_cap_hit,
        }
    }
}

/// 计费单位——venue 费率表的两种原生形态（报告 §3.1：两种单位都要能表达）。
#[derive(Debug, Clone, PartialEq)]
pub enum FeeUnit {
    /// per-notional：成交名义额 × bp/side，maker/taker 分立（Binance 现货）。
    Notional { maker_bps: f64, taker_bps: f64 },
    /// per-share + 最低佣金 + 卖出监管费（IBKR Pro 美股）。
    PerShare(PerShareFees),
}

/// 单品种单档的 venue 费率表（datum 的一条条目 + 其来源哈希）。
///
/// `datum_sha256` 从 [`VenueFeeBook`] 继承——它标定的是**整个 datum 文件**的内容哈希
/// （sidecar `<file>.sha256` 可用 `shasum -a 256 -c` 独立复核），不是本条目的哈希：
/// 口径标签要回答的是"这份费率表是哪个版本"。
#[derive(Debug, Clone, PartialEq)]
pub struct VenueFeeSchedule {
    pub venue: String,
    pub symbol: String,
    pub tier: String,
    pub unit: FeeUnit,
    pub datum_sha256: String,
}

impl VenueFeeSchedule {
    /// 该笔成交的**绝对费用**（美元）。`qty` = 成交手/股数（>0），`px` = 成交价（>0）。
    ///
    /// per-share 档的 side 相关项（SEC/TAF）仅在 [`FillSide::Sell`] 计。
    pub fn fee_usd(&self, qty: f64, px: f64, side: FillSide, role: LiquidityRole) -> f64 {
        assert!(
            qty > 0.0 && px > 0.0 && qty.is_finite() && px.is_finite(),
            "venue 费率解析要求 qty>0 且 px>0（调用方已过滤非法成交）：qty={qty}, px={px}"
        );
        let notional = qty * px;
        match &self.unit {
            FeeUnit::Notional { maker_bps, taker_bps } => {
                notional * role.pick_bps(*maker_bps, *taker_bps) / 10_000.0
            }
            FeeUnit::PerShare(f) => f.breakdown(qty, notional, side).total(),
        }
    }

    /// 该笔成交的**等效单边费率**（分数，账本消费口径）。
    ///
    /// per-notional 档直接给 `bps/1e4`（不走 `fee_usd/notional` 的除法——保持与未标定 fallback
    /// 逐位同形）；per-share 档才把绝对费用摊回名义额。
    pub fn effective_rate(&self, qty: f64, px: f64, side: FillSide, role: LiquidityRole) -> f64 {
        match &self.unit {
            FeeUnit::Notional { maker_bps, taker_bps } => {
                assert!(
                    qty > 0.0 && px > 0.0 && qty.is_finite() && px.is_finite(),
                    "venue 费率解析要求 qty>0 且 px>0：qty={qty}, px={px}"
                );
                role.pick_bps(*maker_bps, *taker_bps) / 10_000.0
            }
            FeeUnit::PerShare(_) => self.fee_usd(qty, px, side, role) / (qty * px),
        }
    }

    /// **与 (qty, px, side) 无关的常数等效费率**（★#423）——本档存在这样一个常数则 `Some`，
    /// 否则 `None`。给"把成本当作一个常数费率"的下游做**结构性**良定义判定（不是人工开关）：
    /// 唯一消费者 = `backtest::treasury::scalar_cost_rate_opt`。
    ///
    /// ## 判定的两个合取条件（票 #423 裁定形态）
    ///
    /// 1. **计费单位 = per-notional**：[`FeeUnit::Notional`] 分支的 [`Self::effective_rate`]
    ///    直接给 `bps/1e4`，函数体不读 `qty`/`px`（venue_fee.rs:190-200）⟹ 规模维消失。
    ///    [`FeeUnit::PerShare`] 分支走 `fee_usd/(qty·px)`，含最低佣金托底 / 1% 名义额上限 /
    ///    仅卖出监管费 ⟹ 费率是 (qty, px, side) 的非线性函数，无此常数 ⟹ `None`。
    /// 2. **角色维消失**：`maker_bps` 与 `taker_bps` **逐位相等**（`to_bits` 比较），
    ///    且实际取值经 [`PRODUCTION_LIQUIDITY_ROLE`]（编译期常量）选 bps。
    ///    非对称档 ⟹ `None`（不选一侧充数——那是把角色维的不对称藏进一个常数，090 声明膨胀）。
    ///
    /// ## 这两个条件里角色常量的**实际判别力**（★#423 收尾轮 D，措辞按实收窄）
    ///
    /// 良定义在**数学上单由「单位形态 + 对称性」推出**：`symmetric` 守卫成立时
    /// `maker_bps` 与 `taker_bps` 逐位相等 ⟹ 本函数体内 `PRODUCTION_LIQUIDITY_ROLE` 的
    /// 两个分支**逐位同值** ⟹ 该 `match` 在当前形态下**无判别力**（把它换成任一角色字面量，
    /// 本函数所有取值不变）。原文把「角色取自编译期常量」与前两条并列为合取条件，读起来像它
    /// 在参与判别——那是声明膨胀（090）。
    ///
    /// 角色常量在此处的实际作用是**纵深防御**，不是判别：若将来 `symmetric` 守卫被放宽
    /// （例如接受"maker/taker 在容差内相等"），或 [`FeeUnit`] 扩充出别的按金额形态，角色来源
    /// 已收口在单一常量上 ⟹ 放宽当天不必再去找"该取哪一侧"的第二处判定。它同时使本判定与
    /// 生产报价 [`FeeQuoter::production_rate_or_fallback`] **同源**（不只是同值）。
    ///
    /// 合取条件本身**按票 #423 裁定形态（对称 ∧ 固定吃单）保留，未放宽**：本轮只订正措辞，
    /// 不因"当前无判别力"而删条件——放宽良定义的适用范围（如允许非对称档取生产侧充数）属
    /// 扩权，须编排者新裁定。
    ///
    /// `is_finite` 兜底：datum 装载路径已 fail-loud 拒非有限值（`datum_io::need`），但本类型
    /// 字段 `pub`、可手工构造 ⟹ NaN 的 `to_bits` 自等会让 NaN 冒充"对称"。照实设防，不假装
    /// 不可达。
    ///
    /// **本判定锁住什么**：档内容（单位形态 + 对称性）与分叉逻辑消费的角色来源。生产成交点
    /// 的角色来源由 [`FeeQuoter::production_rate_or_fallback`] 的签名（无角色参数）承保，
    /// 与本判定引用的是**同一个** [`PRODUCTION_LIQUIDITY_ROLE`]（★#423 第二阶段 C 线收口）。
    /// **锁不住什么**：绕过 [`FeeQuoter`] 直接调 [`Self::effective_rate`] / [`Self::fee_usd`]
    /// 传角色字面量的路径——逐条登记见 [`PRODUCTION_LIQUIDITY_ROLE`] 文档。
    pub fn constant_effective_rate(&self) -> Option<f64> {
        match &self.unit {
            FeeUnit::Notional { maker_bps, taker_bps } => {
                // ★#423 收尾轮 E：角色 → bps 的选择收口到 `LiquidityRole::pick_bps`（原此处与
                //   `fee_usd` / `effective_rate` 各写一份同形 match）。★收尾轮 D：本次取值在
                //   `symmetric` 守卫下两支逐位同值 ⟹ 无判别力，作用是纵深防御（见上方文档）。
                let production_bps = PRODUCTION_LIQUIDITY_ROLE.pick_bps(*maker_bps, *taker_bps);
                let symmetric = maker_bps.to_bits() == taker_bps.to_bits();
                if symmetric && production_bps.is_finite() {
                    Some(production_bps / 10_000.0)
                } else {
                    None
                }
            }
            FeeUnit::PerShare(_) => None,
        }
    }
}

/// 费率解析器——生产成交点的**唯一**费率入口（`fee_schedule` 的 None/Some 分叉在此收口）。
///
/// - `None` ⟹ 恒返回 `fallback_rate`（= `ExecConfig` 三常数合成率），与改动前**逐位相同**；
/// - `Some` ⟹ **datum 费率 + `uncalibrated_addon`**。后者是 datum **不覆盖**的摩擦科目，
///   当前恒 = `slippage_bps/1e4`：venue 费率表只标定佣金/监管/清算，滑点是价差/冲击性质，
///   报告 §3.3 明文「保留 `slippage_bps` 未标定（或另立 spread datum…本票不展开）」。
///   **不加这一项就是把 2bp 滑点悄悄抹掉**（标定后总摩擦反而低于未标定档 = 声明膨胀）。
#[derive(Debug, Clone, Copy)]
pub struct FeeQuoter<'a> {
    fallback_rate: f64,
    uncalibrated_addon: f64,
    schedule: Option<&'a VenueFeeSchedule>,
}

/// 单笔生产成交的 treasury 费率科目报价（★#419）。
///
/// `total = rate × notional` 是生产账本实际消费的总摩擦；per-share 标定档下，其余字段把
/// venue datum 与未标定滑点 addon 拆到原生科目。未标定档及 per-notional 档没有更细的
/// datum 科目定义，分别落 `unclassified_venue`（不得伪造佣金/监管费拆分）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FeeQuote {
    pub rate: f64,
    pub total: f64,
    pub notional: f64,
    pub commission: f64,
    pub passthru: f64,
    pub clearing_cat: f64,
    pub sec: f64,
    pub taf: f64,
    pub slippage: f64,
    pub unclassified_venue: f64,
    pub min_commission_hit: bool,
    pub notional_cap_hit: bool,
    pub taf_cap_hit: bool,
}

impl<'a> FeeQuoter<'a> {
    /// `fallback_rate` = 未标定三常数合成率（唯一来源 = `backtest::treasury::fee_rate`）；
    /// `uncalibrated_addon` = datum 不覆盖、须与 datum 费率**相加**的科目（滑点）。
    pub fn new(
        fallback_rate: f64,
        uncalibrated_addon: f64,
        schedule: Option<&'a VenueFeeSchedule>,
    ) -> Self {
        FeeQuoter { fallback_rate, uncalibrated_addon, schedule }
    }

    /// 未标定档构造（`schedule=None` ⟹ addon 不参与任何取值，置 0 无歧义）。
    pub fn uncalibrated(fallback_rate: f64) -> Self {
        FeeQuoter { fallback_rate, uncalibrated_addon: 0.0, schedule: None }
    }

    /// 本笔成交的单边费率（**私有**：角色维只在本类型内部存在）。未标定档忽略
    /// `qty/px/side/role`（常率，与改动前逐位相同）。
    ///
    /// 不对外暴露的理由（★#423 第二阶段 C 线）：`FeeQuoter` 是生产成交点的唯一报价入口，
    /// 而生产撮合角色只有一个合法取值 [`PRODUCTION_LIQUIDITY_ROLE`]。若本方法 `pub`，
    /// 调用方就能在成交点写一个角色字面量 ⟹ "角色单一来源"退回文档声明。故对外只留
    /// [`Self::production_rate_or_fallback`] / [`Self::production_quote_or_fallback`] 两个
    /// 无角色参数门面，角色在此处内部取常量。
    fn rate_with_role(&self, qty: f64, px: f64, side: FillSide, role: LiquidityRole) -> f64 {
        match self.schedule {
            None => self.fallback_rate,
            Some(s) => s.effective_rate(qty, px, side, role) + self.uncalibrated_addon,
        }
    }

    /// **生产成交点的无角色费率兼容门面**（★#423 第二阶段 C 线）——完整报价由
    /// [`Self::production_quote_or_fallback`] 生成，本方法只返回其中 `rate`。
    ///
    /// 成交量/价合法才询价，否则返回未标定常率占位——该 fill 是 noop/拒单，费率不进算术。
    /// （`fill.rs` 与 `overlay_state.rs` 的成交点共用此一处判定，不各写一份。）
    ///
    /// **本签名锁住什么**：`FeeQuoter` 不再有任何接收 [`LiquidityRole`] 的 `pub` 方法 ⟹
    /// 在任何调用点（生产或测试）向 quoter 传角色字面量是**编译错误**（无参可传）。
    /// **锁不住什么**：绕过本类型直接调 [`VenueFeeSchedule::effective_rate`] /
    /// [`VenueFeeSchedule::fee_usd`]——那两个是费率表**自身**的算术，按 datum 的
    /// maker/taker 维参数化是其定义的一部分（测试与逐笔对拍需要），故保留角色参数。
    /// 见 [`PRODUCTION_LIQUIDITY_ROLE`] 文档的逐条登记。
    pub fn production_rate_or_fallback(&self, qty: f64, px: f64, side: FillSide) -> f64 {
        self.production_quote_or_fallback(qty, px, side).rate
    }

    /// 生产账本逐笔报价 + 原生科目拆分（★#419）。
    ///
    /// 费率 `rate` 仍由既有 [`Self::rate_with_role`] 唯一计算，故旧成交现金流不换公式；
    /// 科目字段只提供同一报价的可审计分解。非法量/价沿用旧 API 的 fallback rate，但没有真实
    /// notional/费用（这些位点是 noop/拒单，报价不进账）。
    pub fn production_quote_or_fallback(&self, qty: f64, px: f64, side: FillSide) -> FeeQuote {
        if !(qty > 0.0 && px > 0.0 && qty.is_finite() && px.is_finite()) {
            return FeeQuote {
                rate: self.fallback_rate,
                ..Default::default()
            };
        }
        let notional = qty * px;
        let rate = self.rate_with_role(qty, px, side, PRODUCTION_LIQUIDITY_ROLE);
        let total = notional * rate;
        let mut quote = FeeQuote {
            rate,
            total,
            notional,
            ..Default::default()
        };
        match self.schedule.map(|s| &s.unit) {
            Some(FeeUnit::PerShare(f)) => {
                let fees = f.breakdown(qty, notional, side);
                quote.commission = fees.commission;
                quote.passthru = fees.passthru;
                quote.clearing_cat = fees.clearing_cat;
                quote.sec = fees.sec;
                quote.taf = fees.taf;
                quote.slippage = notional * self.uncalibrated_addon;
                quote.min_commission_hit = fees.min_commission_hit;
                quote.notional_cap_hit = fees.notional_cap_hit;
                quote.taf_cap_hit = fees.taf_cap_hit;
            }
            Some(FeeUnit::Notional { .. }) => {
                quote.slippage = notional * self.uncalibrated_addon;
                quote.unclassified_venue = total - quote.slippage;
            }
            None => {
                quote.unclassified_venue = total;
            }
        }
        quote
    }

    /// 未标定 fallback 常率（`ExecConfig` 三常数合成）。**非成交量/价的占位取值**用它——
    /// 那些位点的 fill 是 noop/拒单，费率不进任何算术（标定档下亦然）。
    pub fn fallback_rate(&self) -> f64 {
        self.fallback_rate
    }

    /// 是否已 venue 标定（决定口径标签 L1/L2，risk.rs:709 契约）。
    pub fn is_calibrated(&self) -> bool {
        self.schedule.is_some()
    }

    /// 标定档的 datum 内容哈希（未标定 ⟹ `None`）。
    pub fn datum_sha256(&self) -> Option<&str> {
        self.schedule.map(|s| s.datum_sha256.as_str())
    }
}

/// 一份 venue 费率 datum 文件（一个 venue、多条 (symbol, tier) 条目）。
#[derive(Debug, Clone, PartialEq)]
pub struct VenueFeeBook {
    pub venue: String,
    pub source_url: String,
    pub retrieved_at_utc: String,
    /// datum 文件内容的 sha256（小写 hex 64 位），与 sidecar `<file>.sha256` 逐字节核对通过。
    pub datum_sha256: String,
    entries: Vec<VenueFeeSchedule>,
}

impl VenueFeeBook {
    /// 解析 (symbol, tier) 档。未登记 ⟹ `Err`（fail-loud：未核品种不许静默借用别档）。
    pub fn resolve(&self, symbol: &str, tier: &str) -> Result<VenueFeeSchedule, String> {
        self.entries
            .iter()
            .find(|e| e.symbol == symbol && e.tier == tier)
            .cloned()
            .ok_or_else(|| {
                let have: Vec<String> = self
                    .entries
                    .iter()
                    .map(|e| format!("{}/{}", e.symbol, e.tier))
                    .collect();
                format!(
                    "venue fee datum {}: 无 ({symbol}, {tier}) 档（在册：{}）",
                    self.venue,
                    have.join(", ")
                )
            })
    }

    /// 在册档清单（`(symbol, tier)`），供诊断/报告列举。
    pub fn listing(&self) -> Vec<(String, String)> {
        self.entries
            .iter()
            .map(|e| (e.symbol.clone(), e.tier.clone()))
            .collect()
    }
}

#[cfg(any(test, feature = "backtest_bin"))]
pub use datum_io::{datum_dir, load_datum};

/// datum 文件 IO（serde_json + sha2 + fs）。门控同 [`backtest`](super::backtest)——
/// 默认 cdylib（Python 扩展）构建不含，零依赖膨胀。**实现在 `venue_fee/datum_io.rs`**（#374 LOW-D）。
#[cfg(any(test, feature = "backtest_bin"))]
mod datum_io;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const BINANCE_DATUM: &str = "venue_fee_binance_spot_20260726.json";
    const IBKR_DATUM: &str = "venue_fee_ibkr_pro_20260726.json";

    /// 相对误差判等（f64 求和顺序自由度内）。
    fn approx(a: f64, b: f64, ctx: &str) {
        let tol = 1e-12 * b.abs().max(1.0);
        assert!(
            (a - b).abs() <= tol,
            "{ctx}: 实得 {a:.12e} ≠ 预期 {b:.12e}（tol {tol:.1e}）"
        );
    }

    fn binance() -> VenueFeeBook {
        load_datum(&datum_dir().join(BINANCE_DATUM)).expect("Binance 现货 datum 可加载且哈希相符")
    }

    fn ibkr() -> VenueFeeBook {
        load_datum(&datum_dir().join(IBKR_DATUM)).expect("IBKR datum 可加载且哈希相符")
    }

    /// ★#484 / #481 HIGH-1：`total()` 必须逐位复现被替换前的 fee_usd 结合序：
    /// `(commission + passthru + clearing_cat) + (sec + taf)`。
    #[test]
    fn per_share_breakdown_total_is_bit_exact_with_legacy_formula() {
        let assert_legacy_bits = |commission, passthru, clearing_cat, sec, taf| {
            let breakdown = PerShareFeeBreakdown {
                commission,
                passthru,
                clearing_cat,
                sec,
                taf,
                ..Default::default()
            };
            let legacy = (commission + passthru + clearing_cat) + (sec + taf);
            assert_eq!(
                breakdown.total().to_bits(),
                legacy.to_bits(),
                "逐位漂移：c={commission:?} p={passthru:?} cc={clearing_cat:?} s={sec:?} t={taf:?}"
            );
        };

        for commission in [0.0, 0.35, 1.8558223564742051] {
            for passthru in [0.0, 0.000259, 0.0013733085437909118] {
                for clearing_cat in [0.0, 0.0203, 0.15169832475057743] {
                    for sec in [0.0, 0.0412, 1.9562940743891628] {
                        for taf in [0.0, 0.0195, 0.037145707047103835] {
                            assert_legacy_bits(commission, passthru, clearing_cat, sec, taf);
                        }
                    }
                }
            }
        }

        assert_legacy_bits(
            1.8558223564742051,
            0.0013733085437909118,
            0.15169832475057743,
            1.9562940743891628,
            0.037145707047103835,
        );
    }

    // ── datum 装载与 sha256 接线 ──────────────────────────────────────────────

    /// datum 两份齐备、sha256 与 sidecar 相符、来源字段照实落盘。
    #[test]
    fn datum_files_load_and_hash_matches_sidecar() {
        let b = binance();
        assert_eq!(b.venue, "BINANCE_SPOT");
        assert_eq!(b.source_url, "https://www.binance.com/en/fee/trading");
        assert_eq!(b.retrieved_at_utc, "2026-07-26");
        assert_eq!(b.datum_sha256.len(), 64, "sha256 = 64 位 hex");
        assert!(b.datum_sha256.chars().all(|c| c.is_ascii_hexdigit()));

        let i = ibkr();
        assert_eq!(i.venue, "IBKR_PRO_US_EQUITY");
        assert_eq!(
            i.source_url,
            "https://www.interactivebrokers.com/en/pricing/commissions-stocks.php"
        );
        assert_ne!(b.datum_sha256, i.datum_sha256, "两份 datum 哈希互异");
    }

    /// 哈希不符 ⟹ fail-loud（篡改 sidecar 即拒载，禁静默退化）。
    #[test]
    fn tampered_sha256_sidecar_is_rejected() {
        let dir = std::env::temp_dir().join("newchan_venue_fee_tamper_360");
        std::fs::create_dir_all(&dir).unwrap();
        let json = dir.join("v.json");
        std::fs::copy(datum_dir().join(BINANCE_DATUM), &json).unwrap();
        std::fs::write(
            PathBuf::from(format!("{}.sha256", json.display())),
            "0000000000000000000000000000000000000000000000000000000000000000  v.json\n",
        )
        .unwrap();
        let err = load_datum(&json).expect_err("哈希不符必须 Err");
        assert!(err.contains("哈希不符"), "错误须点名哈希不符：{err}");
    }

    /// 未知 `schema_version` ⟹ fail-loud（#62 §4 列式约定的版本门）。
    #[test]
    fn unknown_schema_version_is_rejected() {
        let dir = std::env::temp_dir().join("newchan_venue_fee_schema_360");
        std::fs::create_dir_all(&dir).unwrap();
        let json = dir.join("v.json");
        let src = std::fs::read_to_string(datum_dir().join(BINANCE_DATUM)).unwrap();
        let bumped = src.replace("\"schema_version\": 1", "\"schema_version\": 2");
        assert_ne!(bumped, src, "前置：替换生效");
        std::fs::write(&json, &bumped).unwrap();
        // sidecar 按篡改后的内容重算 ⟹ 哈希这关过得去，卡在版本门（隔离两道校验）。
        let hex = super::datum_io::sha256_hex(bumped.as_bytes());
        std::fs::write(PathBuf::from(format!("{}.sha256", json.display())), format!("{hex}  v.json\n"))
            .unwrap();
        let err = load_datum(&json).expect_err("未知 schema_version 必须 Err");
        assert!(err.contains("schema_version"), "错误须点名版本门：{err}");
    }

    /// sidecar 缺失 ⟹ fail-loud（无版本哈希的费率表不许进生产）。
    #[test]
    fn missing_sha256_sidecar_is_rejected() {
        let dir = std::env::temp_dir().join("newchan_venue_fee_nosidecar_360");
        std::fs::create_dir_all(&dir).unwrap();
        let json = dir.join("v.json");
        std::fs::copy(datum_dir().join(BINANCE_DATUM), &json).unwrap();
        let _ = std::fs::remove_file(PathBuf::from(format!("{}.sha256", json.display())));
        assert!(load_datum(&json).is_err(), "缺 sidecar 必须 Err");
    }

    /// 未入簿品种 ⟹ Err（报告 §4 未核项禁静默借档）。
    #[test]
    fn unlisted_symbol_fails_loud() {
        let b = binance();
        assert!(b.resolve("ES", "VIP0").is_err(), "未入簿品种必须 Err");
        assert!(b.resolve("BTC", "VIP3").is_err(), "未入簿档位必须 Err");
        assert!(ibkr().resolve("QQQ", "PRO_TIERED_LE_300K_SHARES").is_err());
        assert_eq!(b.listing().len(), 2, "Binance 簿在册 2 档");
    }

    // ── §2 一手数字对账（BTC / Binance 现货）──────────────────────────────────

    /// 报告 §2.1：VIP0 maker/taker 均 0.1000% = 10bp/side；BNB 抵扣档 0.075% = 7.5bp/side。
    #[test]
    fn binance_spot_vip0_matches_report_section_2_1() {
        let b = binance();
        let vip0 = b.resolve("BTC", "VIP0").expect("BTC/VIP0 在册");
        let bnb = b.resolve("BTC", "VIP0_BNB25").expect("BTC/VIP0_BNB25 在册");
        let (qty, px) = (1.5, 50_000.0);

        for role in [LiquidityRole::Maker, LiquidityRole::Taker] {
            assert_eq!(
                vip0.effective_rate(qty, px, FillSide::Buy, role),
                1e-3,
                "VIP0 {role:?} = 10bp/side（逐位，per-notional 档不过除法）"
            );
            assert_eq!(bnb.effective_rate(qty, px, FillSide::Sell, role), 7.5e-4);
        }
        // 绝对费用：1.5 BTC @ 50000 = 75000 名义 × 10bp = $75。
        approx(
            vip0.fee_usd(qty, px, FillSide::Buy, LiquidityRole::Taker),
            75.0,
            "VIP0 taker 绝对费用",
        );
        approx(
            bnb.fee_usd(qty, px, FillSide::Buy, LiquidityRole::Taker),
            56.25,
            "BNB 档绝对费用",
        );
        // 现 3bp 默认对 taker 10bp 的倍数（报告 §2.1 「3.3 倍」的可执行形态）。
        approx(
            vip0.effective_rate(qty, px, FillSide::Buy, LiquidityRole::Taker) / 3e-4,
            10.0 / 3.0,
            "VIP0 taker / 未标定默认 3bp",
        );
    }

    /// per-notional 档与 qty/px 无关（同档恒定率）——与 per-share 档的对照物证。
    #[test]
    fn binance_rate_is_size_invariant() {
        let vip0 = binance().resolve("BTC", "VIP0").unwrap();
        let r0 = vip0.effective_rate(1e-4, 3.0, FillSide::Buy, LiquidityRole::Taker);
        let r1 = vip0.effective_rate(1e6, 120_000.0, FillSide::Sell, LiquidityRole::Taker);
        assert_eq!(r0, r1, "per-notional 档：费率不随单量/价位变");
        assert_eq!(r0, 1e-3);
    }

    // ── §2.3 一手数字对账（OKLO / IBKR Pro Tiered）────────────────────────────

    /// 报告 §2.3：$0.0035/股 @$20 ≈ 1.75bp/side，+清算 0.1bp；卖出再 +SEC 0.206bp +TAF ≈0.1bp；
    /// 合计约 2–2.5bp/side。本测把逐项拆开对账（含报告未计入的 CAT/pass-through 微项）。
    #[test]
    fn ibkr_oklo_matches_report_section_2_3() {
        let s = ibkr()
            .resolve("OKLO", "PRO_TIERED_LE_300K_SHARES")
            .expect("OKLO 在册");
        let (qty, px) = (100.0, 20.0); // 名义 $2000；最低佣金恰好不 binding（0.0035×100 = 0.35）

        // 买入：佣金 0.35 + pass-through 0.35×0.00074 + 清算 0.02 + CAT 0.0003
        let buy = s.fee_usd(qty, px, FillSide::Buy, LiquidityRole::Taker);
        approx(buy, 0.370559, "OKLO 买入绝对费用");
        approx(
            s.effective_rate(qty, px, FillSide::Buy, LiquidityRole::Taker),
            1.852795e-4,
            "OKLO 买入等效费率",
        );
        // 卖出：+ SEC 2000×0.0000206 = 0.0412 + TAF min(100×0.000195, 9.79) = 0.0195
        let sell = s.fee_usd(qty, px, FillSide::Sell, LiquidityRole::Taker);
        approx(sell, 0.431259, "OKLO 卖出绝对费用");
        approx(
            s.effective_rate(qty, px, FillSide::Sell, LiquidityRole::Taker),
            2.156295e-4,
            "OKLO 卖出等效费率",
        );
        approx(sell - buy, 0.0607, "卖出监管费增量 = SEC + TAF");

        // 报告 §2.3 的分项换算（bp）：佣金 1.75、清算 0.1、SEC 0.206、TAF ≈0.0975。
        approx(0.0035 / px * 1e4, 1.75, "佣金分项 bp");
        approx(0.0002 / px * 1e4, 0.1, "清算分项 bp");
        approx(0.0000206 * 1e4, 0.206, "SEC 分项 bp");
        approx(0.000195 / px * 1e4, 0.0975, "TAF 分项 bp");
        // 合计落在报告声明的 2–2.5bp/side 带内（卖出侧）。
        let sell_bps = s.effective_rate(qty, px, FillSide::Sell, LiquidityRole::Taker) * 1e4;
        assert!(
            (2.0..=2.5).contains(&sell_bps),
            "卖出合计 {sell_bps:.4}bp 须落在报告 §2.3 的 2–2.5bp 带内"
        );
        // maker/taker 在 per-share venue 不分档（IBKR 佣金不按流动性角色分）。
        assert_eq!(
            s.fee_usd(qty, px, FillSide::Buy, LiquidityRole::Maker),
            buy,
            "IBKR per-share 档无 maker/taker 分歧"
        );
    }

    /// 最低佣金 $0.35 在小单上主导——这正是「压成纯 per-notional 会丢掉」的效应（报告 §3.1）。
    #[test]
    fn ibkr_min_commission_dominates_small_order() {
        let s = ibkr().resolve("OKLO", "PRO_TIERED_LE_300K_SHARES").unwrap();
        // 10 股 @$20：per-share 佣金 0.035 → 最低佣金 0.35 托底（1% 上限 = $2 不 binding）。
        approx(
            s.fee_usd(10.0, 20.0, FillSide::Buy, LiquidityRole::Taker),
            0.352289,
            "10 股买入绝对费用",
        );
        let bps = s.effective_rate(10.0, 20.0, FillSide::Buy, LiquidityRole::Taker) * 1e4;
        approx(bps, 17.61445, "10 股等效费率 bp（最低佣金主导）");
        assert!(
            bps > 10.0 * 1.75,
            "最低佣金档等效费率须显著高于 per-share 名义档（{bps:.3}bp）"
        );
    }

    /// 1% 成交额上限压最低佣金（微单：1%×$20 = $0.20 < $0.35）。
    #[test]
    fn ibkr_notional_cap_beats_min_commission() {
        let s = ibkr().resolve("OKLO", "PRO_TIERED_LE_300K_SHARES").unwrap();
        approx(
            s.fee_usd(1.0, 20.0, FillSide::Buy, LiquidityRole::Taker),
            0.200351,
            "1 股买入：佣金被 1% 上限压到 $0.20",
        );
    }

    /// FINRA TAF 每笔上限 $9.79 生效（大单卖出）。
    #[test]
    fn ibkr_taf_cap_binds_on_large_sell() {
        let s = ibkr().resolve("OKLO", "PRO_TIERED_LE_300K_SHARES").unwrap();
        // 10 万股 @$20：TAF 名义 19.5 → 封顶 9.79。
        approx(
            s.fee_usd(100_000.0, 20.0, FillSide::Sell, LiquidityRole::Taker),
            421.549,
            "10 万股卖出绝对费用（TAF 封顶）",
        );
        // 大单摊薄后回落到 per-share 主导区（2.1bp 级）。
        approx(
            s.effective_rate(100_000.0, 20.0, FillSide::Sell, LiquidityRole::Taker),
            2.107745e-4,
            "10 万股卖出等效费率",
        );
    }

    /// per-share 档的费率**随单量变化**（与 per-notional 档的对照）——单调不增。
    #[test]
    fn ibkr_rate_decreases_with_size() {
        let s = ibkr().resolve("OKLO", "PRO_TIERED_LE_300K_SHARES").unwrap();
        let rates: Vec<f64> = [10.0, 100.0, 1_000.0, 100_000.0]
            .iter()
            .map(|q| s.effective_rate(*q, 20.0, FillSide::Buy, LiquidityRole::Taker))
            .collect();
        for w in rates.windows(2) {
            assert!(w[0] >= w[1], "最低佣金摊薄 ⟹ 等效费率随单量单调不增：{rates:?}");
        }
    }

    // ── FeeQuoter：None 逐值不破 / Some 生效 ─────────────────────────────────

    /// **None ⟹ 逐位等于 fallback 常率**（任意 qty/px/side 全枚举，`assert_eq!` 位相等）。
    ///
    /// ★#423 第二阶段 C 线：原测还枚举 `role ∈ {Maker, Taker}` 断言"None 档忽略角色"。收口后
    /// quoter 的 `pub` 报价方法**不接收角色** ⟹ 该维度在 quoter 层不存在，"忽略角色"变成
    /// 无对象可断言的命题（不是覆盖被削弱，是被断言的对象被删除）。角色维本身仍由
    /// [`VenueFeeSchedule::effective_rate`] 承载，其两支取值由
    /// `production_quote_takes_role_from_single_constant` 在非对称档上分辨。
    #[test]
    fn quoter_none_is_bit_exact_fallback() {
        for fallback in [3e-4, 0.0, 1e-3, 7.5e-4, 1.234_567_891_011e-4] {
            let q = FeeQuoter::uncalibrated(fallback);
            assert!(!q.is_calibrated());
            assert_eq!(q.datum_sha256(), None);
            for qty in [1e-8, 1.0, 12_345.678, 1e9] {
                for px in [1e-6, 20.0, 50_000.0, 1e7] {
                    for side in [FillSide::Buy, FillSide::Sell] {
                        assert_eq!(
                            q.production_rate_or_fallback(qty, px, side),
                            fallback,
                            "None 档必须逐位返回 fallback（qty={qty}, px={px}）"
                        );
                    }
                }
            }
        }
    }

    /// Some ⟹ 走 datum；且 datum 哈希经 quoter 暴露（口径标签的输入）。
    #[test]
    fn quoter_some_uses_datum_and_exposes_hash() {
        let b = binance();
        let vip0 = b.resolve("BTC", "VIP0").unwrap();
        // addon = 2bp 滑点（datum 不覆盖，须相加——报告 §3.3）。
        let q = FeeQuoter::new(3e-4, 2e-4, Some(&vip0));
        assert!(q.is_calibrated());
        assert_eq!(q.datum_sha256(), Some(b.datum_sha256.as_str()));
        assert_eq!(
            q.production_rate_or_fallback(1.0, 50_000.0, FillSide::Buy),
            1e-3 + 2e-4,
            "VIP0 taker 10bp + 未标定滑点 2bp"
        );

        let i = ibkr();
        let oklo = i.resolve("OKLO", "PRO_TIERED_LE_300K_SHARES").unwrap();
        let q2 = FeeQuoter::new(3e-4, 2e-4, Some(&oklo));
        approx(
            q2.production_rate_or_fallback(100.0, 20.0, FillSide::Sell),
            2.156295e-4 + 2e-4,
            "quoter 转发 per-share 档 + 未标定滑点",
        );
        // addon=0 ⟹ 纯 datum 费率（滑点科目的可分离性物证）。
        let q3 = FeeQuoter::new(3e-4, 0.0, Some(&oklo));
        approx(
            q3.production_rate_or_fallback(100.0, 20.0, FillSide::Sell),
            2.156295e-4,
            "addon=0 ⟹ 纯 venue 档",
        );
    }

    /// ★#419 RED：生产报价须把 OKLO per-share venue 费拆成 treasury 可累计的原生科目，
    /// 同时保留未标定滑点 addon；触达布尔值与 IBKR 三段佣金/TAF 上限口径同源。
    /// **认识论 L1**（费用算术与公开报价 seam，不主张 alpha）。
    #[test]
    fn production_quote_exposes_oklo_treasury_components_and_triggers() {
        let oklo = ibkr()
            .resolve("OKLO", "PRO_TIERED_LE_300K_SHARES")
            .expect("OKLO 档在册");
        let q = FeeQuoter::new(3e-4, 2e-4, Some(&oklo));

        let small = q.production_quote_or_fallback(50.0, 20.0, FillSide::Sell);
        approx(small.commission, 0.35, "50 股最低佣金");
        approx(small.passthru, 0.35 * 0.00074, "pass-through");
        approx(small.clearing_cat, 50.0 * 0.000203, "清算+CAT");
        approx(small.sec, 1_000.0 * 0.0000206, "卖出 SEC");
        approx(small.taf, 50.0 * 0.000195, "卖出 TAF");
        approx(small.slippage, 1_000.0 * 2e-4, "未标定滑点 addon");
        assert!(small.min_commission_hit);
        assert!(!small.notional_cap_hit);
        assert!(!small.taf_cap_hit);
        approx(
            small.total,
            small.commission
                + small.passthru
                + small.clearing_cat
                + small.sec
                + small.taf
                + small.slippage,
            "逐科目和 = treasury 实付总费",
        );
        approx(
            small.rate,
            q.production_rate_or_fallback(50.0, 20.0, FillSide::Sell),
            "旧费率 API 与新 quote 同源",
        );

        let capped = q.production_quote_or_fallback(1.0, 20.0, FillSide::Buy);
        assert!(
            !capped.min_commission_hit,
            "1% 上限压过最低佣金时只记上限触达"
        );
        assert!(capped.notional_cap_hit);

        let taf_capped = q.production_quote_or_fallback(100_000.0, 20.0, FillSide::Sell);
        assert!(taf_capped.taf_cap_hit);
    }

    // ── ★#423：常数等效费率的结构判定 ──────────────────────────────────────────

    /// 生产撮合角色常量 = Taker（规格值，报告 §3.1 item 4 的保守初档）。
    ///
    /// **本测锁住**：常量取值本身（改成 Maker 即红）。**它不需要**再核对"生产成交点的字面量
    /// 与本常量同值"——★#423 第二阶段 C 线收口后那些位点已无字面量可写（`FeeQuoter` 的
    /// `pub` 报价方法不接收角色 ⟹ 传角色是编译错误），同源由**签名**承保，不由断言承保。
    /// **锁不住**：绕过 `FeeQuoter` 直传角色的路径（见 [`PRODUCTION_LIQUIDITY_ROLE`] 文档）。
    /// 与 `production_quote_takes_role_from_single_constant` 的分工：那一测在**非对称档**上
    /// 验"生产报价确实取 Taker 那一支"（行为可判别），本测只钉常量的规格取值。
    #[test]
    fn production_role_is_taker() {
        assert_eq!(PRODUCTION_LIQUIDITY_ROLE, LiquidityRole::Taker);
    }

    /// ★#423 第二阶段 C 线：**生产报价的撮合角色只有一个来源**——[`FeeQuoter`] 的生产报价
    /// 方法 [`FeeQuoter::production_rate_or_fallback`] **不接收角色参数**，角色在方法体内取
    /// [`PRODUCTION_LIQUIDITY_ROLE`]。
    ///
    /// **本测的判别力来源**（不是重言）：档取 **maker≠taker 的非对称合成档**（maker 5bp /
    /// taker 10bp）⟹ 两个角色的取值可区分。于是"生产报价 == taker 档取值"是一条能被否证的
    /// 断言：若 [`PRODUCTION_LIQUIDITY_ROLE`] 被改成 `Maker`，本测立即红。对称档上这条断言
    /// 无判别力（两支同值），故本测**必须**用非对称档。
    ///
    /// **同时是费率逐位不变的物证**：收口前三个成交点写 `LiquidityRole::Taker` 字面量并调
    /// 角色版报价，收口后调本方法。本测把"本方法 == 显式 Taker 字面量的档级取值 + addon"
    /// 钉成逐位相等（`assert_eq!`）⟹ 收口是纯重构，成交费率未改一位。
    ///
    /// **认识论 L0**（合成档 + 纯算术契约，零市场信息增量）。
    #[test]
    fn production_quote_takes_role_from_single_constant() {
        let asym = VenueFeeSchedule {
            venue: "SYNTH".into(),
            symbol: "X".into(),
            tier: "T".into(),
            unit: FeeUnit::Notional { maker_bps: 5.0, taker_bps: 10.0 },
            datum_sha256: "0".repeat(64),
        };
        const ADDON: f64 = 2e-4; // 未标定滑点（datum 不覆盖，须相加——报告 §3.3）
        const FALLBACK: f64 = 3e-4;
        let q = FeeQuoter::new(FALLBACK, ADDON, Some(&asym));
        for qty in [1e-8, 1.0, 12_345.678, 1e9] {
            for px in [1e-6, 20.0, 50_000.0, 1e7] {
                for side in [FillSide::Buy, FillSide::Sell] {
                    assert_eq!(
                        q.production_rate_or_fallback(qty, px, side),
                        asym.effective_rate(qty, px, side, LiquidityRole::Taker) + ADDON,
                        "生产报价须逐位等于 Taker 档取值 + addon（qty={qty}, px={px}）"
                    );
                    assert_ne!(
                        q.production_rate_or_fallback(qty, px, side),
                        asym.effective_rate(qty, px, side, LiquidityRole::Maker) + ADDON,
                        "非对称档下两角色取值必须可区分——否则本测无判别力"
                    );
                }
            }
        }
        // 非法量/价 ⟹ 未标定常率占位（该 fill 是 noop/拒单，费率不进算术）——与角色无关。
        assert_eq!(q.production_rate_or_fallback(0.0, 20.0, FillSide::Buy), FALLBACK);
        assert_eq!(q.production_rate_or_fallback(1.0, 0.0, FillSide::Sell), FALLBACK);
        // 未标定档 ⟹ 逐位 fallback（None 分支不读档也不读角色）。
        let none = FeeQuoter::uncalibrated(FALLBACK);
        assert_eq!(none.production_rate_or_fallback(1.0, 20.0, FillSide::Sell), FALLBACK);
    }

    /// 对称 per-notional 档 ⟹ `Some(bps/1e4)`，且与 [`VenueFeeSchedule::effective_rate`] 在
    /// 多组 (qty, px, side) 上**逐位一致**（"与规模/方向无关"的物证）。**认识论 L0**。
    #[test]
    fn constant_rate_some_iff_symmetric_notional() {
        let sym = VenueFeeSchedule {
            venue: "SYNTH".into(),
            symbol: "X".into(),
            tier: "T".into(),
            unit: FeeUnit::Notional { maker_bps: 10.0, taker_bps: 10.0 },
            datum_sha256: "0".repeat(64),
        };
        let c = sym.constant_effective_rate().expect("对称档有常数");
        assert_eq!(c, 1e-3);
        for qty in [1e-8, 1.0, 1e9] {
            for px in [1e-6, 20.0, 1e7] {
                for side in [FillSide::Buy, FillSide::Sell] {
                    assert_eq!(
                        sym.effective_rate(qty, px, side, PRODUCTION_LIQUIDITY_ROLE),
                        c,
                        "常数须与逐笔解析逐位相同（qty={qty}, px={px}）"
                    );
                }
            }
        }

        // ★#423 收尾轮 C：两个变体用**不可变**构造（原实现 `let mut x = sym.clone(); x.unit = …`
        //   违 coding-style「ALWAYS create new objects, NEVER mutate」；改为函数式更新，与同一
        //   diff 内 `..Default::default()` 的写法同型）。
        // 非对称 ⟹ None（角色维不消失）。
        let asym = VenueFeeSchedule {
            unit: FeeUnit::Notional { maker_bps: 5.0, taker_bps: 10.0 },
            ..sym.clone()
        };
        assert_eq!(asym.constant_effective_rate(), None);

        // NaN 的 `to_bits` 自等，不得让它冒充"对称"（datum 路径不可达，手工构造可达）。
        let nan = VenueFeeSchedule {
            unit: FeeUnit::Notional { maker_bps: f64::NAN, taker_bps: f64::NAN },
            ..sym.clone()
        };
        assert_eq!(nan.constant_effective_rate(), None);
    }

    /// per-share 档 ⟹ `None`；且其逐笔费率**确实随 qty 变**（"无常数"的物证，不是口头声明）。
    ///
    /// **认识论 L1**（★#423 收尾轮 B 订正，原标 L2「真实 IBKR datum」）。订正理由：datum 取自真实
    /// venue 官方费率表不足以判 L2——L2 要求「真实数据**假设检验**，可能产生否定性结果」。本测断言
    /// 的是费率表**算术形态**（per-share 分支给 `None`；两个不同 qty 上的等效费率不相等），这是
    /// 装载 + 算术管线的正确性，输入是契约值而非估计量 ⟹ 不可否证任何市场假设，信息增量为零。
    /// **本测能否证的假设：无**（同型先例 `wverify_run::fee_datum_spec_resolves_repo_datum` 标 L1）。
    #[test]
    fn constant_rate_none_for_per_share_with_witness() {
        let s = ibkr().resolve("OKLO", "PRO_TIERED_LE_300K_SHARES").unwrap();
        assert_eq!(s.constant_effective_rate(), None);
        let r_small = s.effective_rate(10.0, 20.0, FillSide::Buy, PRODUCTION_LIQUIDITY_ROLE);
        let r_big = s.effective_rate(100_000.0, 20.0, FillSide::Buy, PRODUCTION_LIQUIDITY_ROLE);
        assert_ne!(r_small, r_big, "per-share 档费率随规模变 ⟹ 无常数可取");
    }

    /// 真实 datum：Binance 现货两档（VIP0 / BNB 抵扣）maker=taker ⟹ 均给常数。
    ///
    /// **认识论 L1**（★#423 收尾轮 B 订正，原标 L2「datum = venue 官方费率表快照」）。订正理由：
    /// 「输入是真实数据」是 L2 的必要条件而非充分条件——L2 还要求该输入承载一个**可被否证的假设**。
    /// 本测断言两条在册档的对称性判定结果与 bps 换算（`Some(1e-3)` / `Some(7.5e-4)`），期望值直接
    /// 由费率表字面值算出 ⟹ 只验装载与除法，不验任何关于市场的命题。
    /// **本测能否证的假设：无**（同型先例 `wverify_run::fee_datum_spec_resolves_repo_datum` 标 L1）。
    #[test]
    fn constant_rate_binance_real_datum() {
        let b = binance();
        assert_eq!(
            b.resolve("BTC", "VIP0").unwrap().constant_effective_rate(),
            Some(1e-3)
        );
        assert_eq!(
            b.resolve("BTC", "VIP0_BNB25").unwrap().constant_effective_rate(),
            Some(7.5e-4)
        );
    }

    /// 非法成交量/价 ⟹ panic（fail-loud，调用方已过滤 ⟹ 到不了这里）。
    #[test]
    #[should_panic(expected = "venue 费率解析要求")]
    fn zero_qty_panics() {
        let vip0 = binance().resolve("BTC", "VIP0").unwrap();
        let _ = vip0.effective_rate(0.0, 20.0, FillSide::Buy, LiquidityRole::Taker);
    }
}
