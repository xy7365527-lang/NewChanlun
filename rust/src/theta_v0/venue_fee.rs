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
//!   （报告 §3.1 item 4 的保守初档）。[`LiquidityRole::Maker`] 档存在于 datum 但生产不取；
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
                let bps = match role {
                    LiquidityRole::Maker => *maker_bps,
                    LiquidityRole::Taker => *taker_bps,
                };
                notional * bps / 10_000.0
            }
            FeeUnit::PerShare(f) => {
                // 佣金：per-share 起，每单最低佣金托底，成交额比例封顶（IBKR 三段口径）。
                let commission = (f.commission_per_share_usd * qty)
                    .max(f.min_commission_usd)
                    .min(notional * f.max_commission_frac_of_notional);
                let passthru = commission
                    * (f.passthru_exchange_frac_of_commission + f.passthru_finra_frac_of_commission);
                let per_share_both_sides =
                    (f.clearing_per_share_usd + f.cat_per_share_usd) * qty;
                let sell_reg = match side {
                    FillSide::Sell => {
                        notional * f.sell_sec_fee_frac
                            + (f.sell_taf_per_share_usd * qty).min(f.sell_taf_cap_usd)
                    }
                    FillSide::Buy => 0.0,
                };
                commission + passthru + per_share_both_sides + sell_reg
            }
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
                let bps = match role {
                    LiquidityRole::Maker => *maker_bps,
                    LiquidityRole::Taker => *taker_bps,
                };
                bps / 10_000.0
            }
            FeeUnit::PerShare(_) => self.fee_usd(qty, px, side, role) / (qty * px),
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

    /// 本笔成交的单边费率。未标定档忽略 `qty/px/side/role`（常率，与改动前逐位相同）。
    pub fn rate_for(&self, qty: f64, px: f64, side: FillSide, role: LiquidityRole) -> f64 {
        match self.schedule {
            None => self.fallback_rate,
            Some(s) => s.effective_rate(qty, px, side, role) + self.uncalibrated_addon,
        }
    }

    /// 成交量/价合法才询价，否则返回未标定常率占位——该 fill 是 noop/拒单，费率不进算术。
    /// （`fill.rs` 与 `overlay_state.rs` 的成交点共用此一处判定，不各写一份。）
    pub fn rate_or_fallback(&self, qty: f64, px: f64, side: FillSide, role: LiquidityRole) -> f64 {
        if qty > 0.0 && px > 0.0 {
            self.rate_for(qty, px, side, role)
        } else {
            self.fallback_rate
        }
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
/// 默认 cdylib（Python 扩展）构建不含，零依赖膨胀。
#[cfg(any(test, feature = "backtest_bin"))]
mod datum_io {
    use super::{FeeUnit, PerShareFees, VenueFeeBook, VenueFeeSchedule};
    use serde::Deserialize;
    use sha2::{Digest, Sha256};
    use std::path::{Path, PathBuf};

    /// datum 落盘目录（与 `backtest::data::data_dir` 同一约定：`analysis/data_cache`）。
    pub fn datum_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("rust/ 的父目录 = 项目根")
            .join("analysis/data_cache")
    }

    /// datum schema 版本（#62 §4 列式约定）。**只接受 1**——未知版本 fail-loud，
    /// 禁按旧字段表静默解析新 schema。
    pub const DATUM_SCHEMA_VERSION: u32 = 1;

    #[derive(Deserialize)]
    struct RawBook {
        schema_version: u32,
        venue: String,
        source_url: String,
        retrieved_at_utc: String,
        entries: Vec<RawEntry>,
    }

    #[derive(Deserialize)]
    struct RawEntry {
        symbol: String,
        tier: String,
        unit: String,
        // per-notional 档
        maker_bps: Option<f64>,
        taker_bps: Option<f64>,
        // per-share 档
        commission_per_share_usd: Option<f64>,
        min_commission_usd: Option<f64>,
        max_commission_frac_of_notional: Option<f64>,
        clearing_per_share_usd: Option<f64>,
        cat_per_share_usd: Option<f64>,
        sell_sec_fee_frac: Option<f64>,
        sell_taf_per_share_usd: Option<f64>,
        sell_taf_cap_usd: Option<f64>,
        passthru_exchange_frac_of_commission: Option<f64>,
        passthru_finra_frac_of_commission: Option<f64>,
    }

    fn need(v: Option<f64>, field: &str, ctx: &str) -> Result<f64, String> {
        let x = v.ok_or_else(|| format!("{ctx}: 缺字段 {field}"))?;
        if !x.is_finite() || x < 0.0 {
            return Err(format!("{ctx}: 字段 {field} 非有限非负（{x}）"));
        }
        Ok(x)
    }

    /// sha256 小写 hex。
    pub(super) fn sha256_hex(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }

    /// 读 datum 文件 + **sha256 版本校验**（sidecar `<path>.sha256`，`shasum -a 256` 格式）。
    ///
    /// fail-loud：文件缺失 / sidecar 缺失 / 哈希不符 / JSON 非法 / 字段缺失或非有限非负 /
    /// 未知 `unit` / 条目为空 / (symbol,tier) 重复 ⟹ `Err`。**禁静默退化**（未校验的费率表
    /// 进生产 = 口径标签谎报 L2）。
    pub fn load_datum(json_path: &Path) -> Result<VenueFeeBook, String> {
        let bytes = std::fs::read(json_path)
            .map_err(|e| format!("venue fee datum 读失败 {}: {e}", json_path.display()))?;
        let actual = sha256_hex(&bytes);

        let sidecar = PathBuf::from(format!("{}.sha256", json_path.display()));
        let sidecar_txt = std::fs::read_to_string(&sidecar)
            .map_err(|e| format!("venue fee datum sha256 sidecar 读失败 {}: {e}", sidecar.display()))?;
        let expected = sidecar_txt
            .split_whitespace()
            .next()
            .ok_or_else(|| format!("sha256 sidecar 空 {}", sidecar.display()))?
            .to_ascii_lowercase();
        if expected != actual {
            return Err(format!(
                "venue fee datum 哈希不符 {}: sidecar={expected} 实算={actual}（datum 漂移或被改写）",
                json_path.display()
            ));
        }

        let raw: RawBook = serde_json::from_slice(&bytes)
            .map_err(|e| format!("venue fee datum JSON 非法 {}: {e}", json_path.display()))?;
        build_book(raw, actual)
    }

    fn build_book(raw: RawBook, datum_sha256: String) -> Result<VenueFeeBook, String> {
        if raw.schema_version != DATUM_SCHEMA_VERSION {
            return Err(format!(
                "venue fee datum {}: schema_version={} 不受支持（本实装只认 {}）",
                raw.venue, raw.schema_version, DATUM_SCHEMA_VERSION
            ));
        }
        if raw.entries.is_empty() {
            return Err(format!("venue fee datum {}: entries 空", raw.venue));
        }
        let mut entries: Vec<VenueFeeSchedule> = Vec::with_capacity(raw.entries.len());
        for e in &raw.entries {
            let ctx = format!("venue fee datum {} 条目 {}/{}", raw.venue, e.symbol, e.tier);
            if entries.iter().any(|p| p.symbol == e.symbol && p.tier == e.tier) {
                return Err(format!("{ctx}: (symbol, tier) 重复"));
            }
            let unit = match e.unit.as_str() {
                "notional" => FeeUnit::Notional {
                    maker_bps: need(e.maker_bps, "maker_bps", &ctx)?,
                    taker_bps: need(e.taker_bps, "taker_bps", &ctx)?,
                },
                "per_share" => {
                    let max_frac = need(
                        e.max_commission_frac_of_notional,
                        "max_commission_frac_of_notional",
                        &ctx,
                    )?;
                    let taf_cap = need(e.sell_taf_cap_usd, "sell_taf_cap_usd", &ctx)?;
                    if max_frac <= 0.0 {
                        return Err(format!("{ctx}: max_commission_frac_of_notional 必须 >0"));
                    }
                    if taf_cap <= 0.0 {
                        return Err(format!("{ctx}: sell_taf_cap_usd 必须 >0"));
                    }
                    FeeUnit::PerShare(PerShareFees {
                        commission_per_share_usd: need(
                            e.commission_per_share_usd,
                            "commission_per_share_usd",
                            &ctx,
                        )?,
                        min_commission_usd: need(e.min_commission_usd, "min_commission_usd", &ctx)?,
                        max_commission_frac_of_notional: max_frac,
                        clearing_per_share_usd: need(
                            e.clearing_per_share_usd,
                            "clearing_per_share_usd",
                            &ctx,
                        )?,
                        cat_per_share_usd: need(e.cat_per_share_usd, "cat_per_share_usd", &ctx)?,
                        sell_sec_fee_frac: need(e.sell_sec_fee_frac, "sell_sec_fee_frac", &ctx)?,
                        sell_taf_per_share_usd: need(
                            e.sell_taf_per_share_usd,
                            "sell_taf_per_share_usd",
                            &ctx,
                        )?,
                        sell_taf_cap_usd: taf_cap,
                        passthru_exchange_frac_of_commission: need(
                            e.passthru_exchange_frac_of_commission,
                            "passthru_exchange_frac_of_commission",
                            &ctx,
                        )?,
                        passthru_finra_frac_of_commission: need(
                            e.passthru_finra_frac_of_commission,
                            "passthru_finra_frac_of_commission",
                            &ctx,
                        )?,
                    })
                }
                other => return Err(format!("{ctx}: 未知计费单位 unit={other}")),
            };
            entries.push(VenueFeeSchedule {
                venue: raw.venue.clone(),
                symbol: e.symbol.clone(),
                tier: e.tier.clone(),
                unit,
                datum_sha256: datum_sha256.clone(),
            });
        }
        Ok(VenueFeeBook {
            venue: raw.venue,
            source_url: raw.source_url,
            retrieved_at_utc: raw.retrieved_at_utc,
            datum_sha256,
            entries,
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// NIST 向量：sha256("abc") / sha256("")——证明哈希实现口径 = 标准 SHA-256
        /// （⟹ sidecar 可用 `shasum -a 256 -c` 独立复核，非自造校验和）。
        #[test]
        fn sha256_matches_nist_vectors() {
            assert_eq!(
                sha256_hex(b"abc"),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            );
            assert_eq!(
                sha256_hex(b""),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            );
        }
    }
}

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

    /// **None ⟹ 逐位等于 fallback 常率**（任意 qty/px/side/role 全枚举，`assert_eq!` 位相等）。
    #[test]
    fn quoter_none_is_bit_exact_fallback() {
        for fallback in [3e-4, 0.0, 1e-3, 7.5e-4, 1.234_567_891_011e-4] {
            let q = FeeQuoter::uncalibrated(fallback);
            assert!(!q.is_calibrated());
            assert_eq!(q.datum_sha256(), None);
            for qty in [1e-8, 1.0, 12_345.678, 1e9] {
                for px in [1e-6, 20.0, 50_000.0, 1e7] {
                    for side in [FillSide::Buy, FillSide::Sell] {
                        for role in [LiquidityRole::Maker, LiquidityRole::Taker] {
                            assert_eq!(
                                q.rate_for(qty, px, side, role),
                                fallback,
                                "None 档必须逐位返回 fallback（qty={qty}, px={px}）"
                            );
                        }
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
            q.rate_for(1.0, 50_000.0, FillSide::Buy, LiquidityRole::Taker),
            1e-3 + 2e-4,
            "VIP0 taker 10bp + 未标定滑点 2bp"
        );

        let i = ibkr();
        let oklo = i.resolve("OKLO", "PRO_TIERED_LE_300K_SHARES").unwrap();
        let q2 = FeeQuoter::new(3e-4, 2e-4, Some(&oklo));
        approx(
            q2.rate_for(100.0, 20.0, FillSide::Sell, LiquidityRole::Taker),
            2.156295e-4 + 2e-4,
            "quoter 转发 per-share 档 + 未标定滑点",
        );
        // addon=0 ⟹ 纯 datum 费率（滑点科目的可分离性物证）。
        let q3 = FeeQuoter::new(3e-4, 0.0, Some(&oklo));
        approx(
            q3.rate_for(100.0, 20.0, FillSide::Sell, LiquidityRole::Taker),
            2.156295e-4,
            "addon=0 ⟹ 纯 venue 档",
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
