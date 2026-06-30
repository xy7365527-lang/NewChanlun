//! 经济正条件逐信号分解（《经济正条件.pdf》第5节可捕获价差判据 L2 诊断）。
//!
//! 证明链 L0 见 `.chanlun/proofs/economic-positive-condition-chain.md`。本模块是其
//! **唯一可否证环节**（可捕获价差判据前件）的 L2 实装：逐信号确定性分解
//!
//!     captured = Ab_rev − ηin − ηout − Ce/qe        (PDF p4-5 §5，对象=反转交易腿)
//!
//! **664 号对象错配修复（codex 异质审计 diagnose 坐实，2026-06-30）**：缠论买卖点是
//! **反转交易**（底背驰买点 δ=+1 出现在下跌段末端，顶背驰卖点 δ=−1 出现在上涨段末端），
//! 交易方向 δ 与信号前触发/背驰段方向 ε **内在相反**。旧实装测「信号前触发段端点价差」
//! `Ab=εb(Pρb−Pλb)`（locate_lambda_bar 定位触发段起点）= **测错对象**：触发段几何方向反平行
//! 于 δ ⟹ δ·(Pρ−Pλ)<0 系统性产生 ΣAb<0（BTC L2 ΣAb=−1.8e4）。正确对象 = **post-signal
//! 反转交易腿**（信号确认后真正持有的那一段）。**禁止补丁**：改 eps 回 sign(Pρ−Pλ) 只恢复
//! Ab=|Pρ−Pλ|≥0 同义反复（命题S L0 恒真，零信息）= 声明膨胀（090）+ 掩盖错配（no-patch）。
//!
//! 反转交易腿分解（δ=+1 多 / δ=−1 空）：
//! - `Ab_rev = δ(P[ρ_rev] − P[λ_rev])`      反转交易腿理想价差（λ_rev=入场信号挂靠 pivot，
//!                                          ρ_rev=配对出场信号挂靠 pivot，端点 close）。
//! - `ηin    = max(0, δ(Pτin − P[λ_rev]))`  入场滞后损耗（确认 bar 成交价相对入场 pivot 的不利滑移，≥0）。
//! - `ηout   = max(0, δ(P[ρ_rev] − Pτout))` 出场损耗（实际出场价相对出场 pivot 的损耗，≥0）。
//! - `Ce/qe  = Pτ·fee_rate·2`               单位双边成本（与 `mu_estimator::marginal_return` 同口径）。
//!
//! **可否证（L2）**：若 Σ(ηin+ηout+Ce) ≥ ΣAb_rev（执行损耗吃光反转腿价差），则该信号集无可捕获 alpha，
//! 且分解定位「钱去哪了」（反转腿无价差 / 执行滞后 / 成本三者分离）。与 663 咬合：逐信号确定性分解，
//! 非统计功效检验。注意：与旧触发段实装不同，Ab_rev 在反转腿上可正可负——是测对了对象之后的真实价差，
//! 不是同义反复（命题S L0 恒真仅在顺笔/触发段成立）。
//!
//! **端点价缺口的严格解（不改 TradeRecord/Order）**：理想 pivot 端点价 P[λ_rev]/P[ρ_rev] 不在 fill
//! 配对链（`TradeRecord` 只有 entry/exit bar），但**信号收集路径**（同 `build_walk_forward_mu`）在确认点
//! 持有 BspPoint——其 `source_index`（bsp.rs:103，L0 原始 K 序的 pivot 端点位置）即信号挂靠的 pivot 端点
//! bar，取其 close 作 P[λ_rev]（入场信号）/P[ρ_rev]（配对出场信号）。故走 μ 路径无需透传 Order 端点价。

use super::data::Dataset;
use super::incremental::IncrementalClassifier;
use super::super::config::ThetaConfig;
use super::super::strategy::interp::assemble_gamma_with_tower;
use super::super::strategy::voice::VoiceSide;
use super::super::types::{Bar, BspBits};

/// 单信号的可捕获价差分解（PDF §5 五项 + captured = Ab−ηin−ηout−Ce/qe）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalDecomp {
    /// 入场确认 bar（τin，F_i 可测）。
    pub entry_bar: usize,
    /// 出场 bar（τout，下一反向确认信号 / 末 bar censored）。
    pub exit_bar: usize,
    /// 触发买卖点所在 tower 级别（per-class 分桶键 (level,δ) 的 level 分量，MuClass.level 同口径）。
    pub level: u32,
    /// 方向 δ：+1 多 / −1 空（交易方向，反转交易腿；664 号 δ≠ε 笔方向）。
    pub delta: i8,
    /// Ab_rev = δ(P[ρ_rev] − P[λ_rev])：反转交易腿理想价差（λ_rev=入场信号 pivot，ρ_rev=出场信号 pivot；
    /// 664 号修复测错对象——非触发段价差，可正可负）。
    pub a_b: f64,
    /// x = δ(Pτin − P[λ_rev])：**signed** 入场滑移（664-Q3 codex）。x<0=有利（确认价比 pivot 更优），x>0=不利。
    /// ηin=max(0,x) 丢弃了 x<0 的有利滑移 ⟹ captured 系统性低估真实 PnL（min(x,0)≤0）。
    pub x_in: f64,
    /// y = δ(P[ρ_rev] − Pτout)：**signed** 出场滑移（664-Q3 codex）。y<0=有利，y>0=不利。
    pub y_out: f64,
    /// ηin = max(0, x_in)：入场滞后损耗（adverse-only，≥0）。
    pub eta_in: f64,
    /// ηout = max(0, y_out)：出场损耗（adverse-only，≥0）。
    pub eta_out: f64,
    /// actual_spread = δ(Pτout − Pτin) = Ab_rev − x_in − y_out：**真实成交价差**（无成本，664-Q3）。
    /// 两个确认 bar（实际成交点）close 的有向差——含有利+不利滑移全部，非 adverse-only。
    pub actual_spread: f64,
    /// Ce/qe：单位双边成本。
    pub ce_unit: f64,
    /// captured = Ab_rev − ηin − ηout − Ce/qe（adverse-only 保守压力测试，PDF §5；丢有利滑移）。
    pub captured: f64,
    /// actual_pnl = actual_spread − Ce/qe = δ(Pτout − Pτin) − Ce：**真实成交 PnL 代理**（664-Q3，含全部滑移）。
    pub actual_pnl: f64,
}

/// 聚合诊断「钱去哪了」（确定性分解，Σ 精确等于 Σ trade gross，非概率推断）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpreadAttribution {
    pub n_signals: usize,
    /// Σ Ab_rev：反转交易腿给的理想总价差（664 号：post-signal 腿，非触发段）。
    pub sum_a_b: f64,
    /// Σ x_in：**signed** 入场滑移总和（664-Q3）。含有利（<0）+不利（>0），非 adverse-only。
    pub sum_x_in: f64,
    /// Σ y_out：**signed** 出场滑移总和（664-Q3）。
    pub sum_y_out: f64,
    /// Σ ηin = Σ max(0, x_in)：入场滞后吃掉（adverse-only，≥0）。
    pub sum_eta_in: f64,
    /// Σ ηout = Σ max(0, y_out)：出场滞后吃掉（adverse-only，≥0）。
    pub sum_eta_out: f64,
    /// Σ actual_spread = Σ δ(Pτout − Pτin)：**真实成交价差**总和（无成本，664-Q3，含全部滑移）。
    pub sum_actual_spread: f64,
    /// Σ Ce/qe：成本吃掉。
    pub sum_ce: f64,
    /// Σ captured：adverse-only 保守压力测试剩余 alpha（丢有利滑移，系统性偏负）。
    pub sum_captured: f64,
    /// 可捕获信号数（captured>0）——逐信号路径级正占比的分子。
    pub n_captured_positive: usize,
    /// actual_pnl>0 信号数（真实成交口径）——区别于 n_captured_positive（adverse-only）。
    pub n_actual_positive: usize,
    /// 三审计统计①：rho_rev_bar>lambda_rev_bar 计数（出场 pivot 端点在入场 pivot 端点之后；codex Q1 不变量）。
    pub n_rho_after_lambda: usize,
    /// 三审计统计②：same_bar_opposite 计数（同 bar 出现反向信号被 eb>entry_bar 排除；codex Q2 边界）。
    pub n_same_bar_opposite: usize,
    /// 三审计统计③：n_unpaired 计数（入场信号无配对出场反转信号，右删失诚实跳过；codex Q4 边界）。
    pub n_unpaired: usize,
}

impl SpreadAttribution {
    /// L2 否证判据（**adverse-only 保守口径**）：执行损耗（含成本）是否吃光结构价差。
    ///
    /// `true` ⟺ Σcaptured ≤ 0。注意：captured 用 max(0,·) 丢弃有利滑移 ⟹ 系统性低估真实 PnL，
    /// 这是**压力测试**口径，不等于真实成交。真实成交口径见 `actual_pnl_proxy`/`actual_pnl_eaten`。
    pub fn spread_eaten(&self) -> bool {
        self.sum_captured <= 0.0
    }

    /// 真实成交 PnL 代理（664-Q3 codex）：Σactual_spread − ΣCe = Σδ(Pτout−Pτin) − ΣCe。
    /// 含全部滑移（有利+不利），是确认 bar 实际成交价差减成本——比 adverse-only captured 更接近真实成交。
    pub fn actual_pnl_proxy(&self) -> f64 {
        self.sum_actual_spread - self.sum_ce
    }

    /// 真实成交口径的「执行吃光」判据：actual_pnl_proxy ≤ 0。
    ///
    /// 与 `spread_eaten`（adverse-only）的分歧即 codex Q3 关注点：若 `spread_eaten`=true 但本判据=false，
    /// 则「执行滞后吃光」是 adverse-only 伪结论（有利滑移被丢弃所致），真实成交其实可捕获。
    pub fn actual_pnl_eaten(&self) -> bool {
        self.actual_pnl_proxy() <= 0.0
    }
}

/// 买卖点身份判别 u8（seen-set diff 键，与 l3_delta_r_alpha::bsp_disc 同口径）。
fn bsp_disc(b: &BspBits) -> u8 {
    (b.buy1 as u8)
        | (b.buy2 as u8) << 1
        | (b.buy3 as u8) << 2
        | (b.sell1 as u8) << 3
        | (b.sell2 as u8) << 4
        | (b.sell3 as u8) << 5
}

/// 逐信号可捕获价差分解（L2）。复用 `build_walk_forward_mu` 的信号收集 + 退出配对模板，
/// 取入场信号与配对出场信号的 pivot 端点（source_index）算 PDF §5 反转交易腿分解（664 号）。
///
/// 返回 `(Vec<SignalDecomp>, SpreadAttribution)`：逐信号分解 + 聚合归因。
///
/// **认识论 L2**：真实数据逐信号分解，可产否定性结果（spread_eaten=true ⟹ 信号集无 alpha）。
pub fn decompose_capturable_spread(data: &Dataset, config: &ThetaConfig) -> (Vec<SignalDecomp>, SpreadAttribution) {
    let bars = &data.bars;
    let n = bars.len();
    let tick = config.tick.tick_size;
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;

    // ── 信号收集（同 build_walk_forward_mu）：逐 bar 因果分类，收新确认买卖点。 ──
    // 664 号：每条信号挂靠的 pivot 端点 = p.source_index（bsp.rs:103，L0 原始 K 序）。
    // 反转交易腿 λ_rev = 入场信号 pivot 端点；ρ_rev 在退出配对时取配对出场信号的 pivot 端点。
    let mut classifier_incr = IncrementalClassifier::new(bars, config);
    let mut seen: std::collections::HashSet<(usize, usize, u8)> = std::collections::HashSet::new();
    // 每条信号：(entry_bar=确认 bar τin, dir=交易方向 δ, pivot_bar=信号挂靠 pivot 端点 source_index, lvl=级别)。
    let mut signals: Vec<(usize, VoiceSide, usize, u32)> = Vec::new();

    for i in 0..n {
        let bar = &bars[i];
        if bar.untradable || bar.close <= 0 {
            continue;
        }
        let (cls_i, tower_i) = classifier_incr.classify_at(i);
        for (lvl, ls) in cls_i.levels.iter().enumerate() {
            for p in &ls.bsp {
                if !seen.insert((lvl, p.source_index, bsp_disc(&p.bits))) {
                    continue; // 已确认过
                }
                let pivot_bar = p.source_index; // 信号挂靠 pivot 端点（bsp.rs:103）= λ_rev / ρ_rev 取价处
                // dir 经 assemble_gamma 拿（构造仅含该点的单级别分类，同 build_walk_forward_mu）。
                let single = super::super::classifier::Classification {
                    levels: cls_i
                        .levels
                        .iter()
                        .enumerate()
                        .map(|(l2, _)| super::super::classifier::LevelState {
                            moves: Vec::new(),
                            centers: Vec::new(),
                            bsp: if l2 == lvl { vec![p.clone()] } else { Vec::new() },
                        })
                        .collect(),
                };
                for c in &assemble_gamma_with_tower(&single, &tower_i) {
                    if c.dir == VoiceSide::Flat {
                        continue;
                    }
                    signals.push((i, c.dir, pivot_bar, lvl as u32));
                }
            }
        }
    }

    // ── 退出配对（664 号反转交易腿）：持有到下一反向新确认信号，ρ_rev = 该配对出场信号的 pivot 端点。 ──
    // 与旧实装的关键差异：ρ_rev 是 **post-signal 且策略 owned**（配对出场信号挂靠 pivot），
    // 不是触发段起点（错对象）。无配对出场信号 ⟹ 诚实跳过（末 bar 不是 pivot 端点，无 ρ_rev，不兜底）。
    let mut decomps: Vec<SignalDecomp> = Vec::new();
    let mut agg = SpreadAttribution::default();

    for (idx, &(entry_bar, dir, lambda_rev_bar, level)) in signals.iter().enumerate() {
        let delta: i8 = match dir {
            VoiceSide::Long => 1,
            VoiceSide::Short => -1,
            VoiceSide::Flat => continue,
        };
        let opp = if delta == 1 { VoiceSide::Short } else { VoiceSide::Long };
        // 三审计统计②（codex Q2）：同 bar 出现反向信号（被后续 eb>entry_bar 配对条件排除的边界）。
        if signals[idx + 1..].iter().any(|(eb, d, _, _)| *eb == entry_bar && *d == opp) {
            agg.n_same_bar_opposite += 1;
        }
        // 配对出场信号（首个后续反向新确认信号，π^bsp owned）：取其 entry_bar(τout) + pivot_bar(ρ_rev)。
        let (exit_bar, rho_rev_bar) = match signals[idx + 1..]
            .iter()
            .find(|(eb, d, _, _)| *eb > entry_bar && *d == opp)
            .map(|&(eb, _, pivot, _)| (eb, pivot))
        {
            Some(pair) => pair,
            None => {
                agg.n_unpaired += 1; // 三审计统计③（codex Q4）：右删失，无配对出场反转信号，诚实跳过不兜底
                continue;
            }
        };
        if exit_bar <= entry_bar {
            continue;
        }
        // 端点价（close 口径，与回测成交价同口径）：
        // P[λ_rev]=入场信号 pivot 端点 close（理想入场），P[ρ_rev]=配对出场信号 pivot 端点 close（理想出场），
        // Pτin=入场确认 bar close（实际入场，含确认滞后），Pτout=出场确认 bar close（实际出场）。
        let p_lambda = px_at(bars, lambda_rev_bar, tick);
        let p_rho = px_at(bars, rho_rev_bar, tick);
        let p_tau_in = px_at(bars, entry_bar, tick);
        let p_tau_out = px_at(bars, exit_bar, tick);
        if [p_lambda, p_rho, p_tau_in, p_tau_out].iter().any(|&x| x <= 0.0) {
            continue;
        }
        let eps = delta as f64; // 交易方向 δ（反转交易腿；664 号 δ≠ε 笔方向）
        let a_b = eps * (p_rho - p_lambda); // Ab_rev=δ(P[ρ_rev]−P[λ_rev])，可正可负（测对了对象）
        let x_in = eps * (p_tau_in - p_lambda); // signed 入场滑移（664-Q3）：<0 有利 / >0 不利
        let y_out = eps * (p_rho - p_tau_out); // signed 出场滑移（664-Q3）
        let eta_in = x_in.max(0.0); // adverse-only（丢 x<0 有利滑移）
        let eta_out = y_out.max(0.0);
        let actual_spread = eps * (p_tau_out - p_tau_in); // = a_b − x_in − y_out（真实成交价差，含全部滑移）
        // 单位双边成本：与 marginal_return 口径一致（fee 在 entry/exit 各扣一次）。
        let ce_unit = (p_tau_in + p_tau_out) * fee_rate;
        let captured = a_b - eta_in - eta_out - ce_unit; // adverse-only 保守压力测试
        let actual_pnl = actual_spread - ce_unit; // 真实成交 PnL 代理（664-Q3）

        decomps.push(SignalDecomp {
            entry_bar, exit_bar, level, delta, a_b, x_in, y_out,
            eta_in, eta_out, actual_spread, ce_unit, captured, actual_pnl,
        });
        agg.n_signals += 1;
        agg.sum_a_b += a_b;
        agg.sum_x_in += x_in;
        agg.sum_y_out += y_out;
        agg.sum_eta_in += eta_in;
        agg.sum_eta_out += eta_out;
        agg.sum_actual_spread += actual_spread;
        agg.sum_ce += ce_unit;
        agg.sum_captured += captured;
        if captured > 0.0 {
            agg.n_captured_positive += 1;
        }
        if actual_pnl > 0.0 {
            agg.n_actual_positive += 1;
        }
        if rho_rev_bar > lambda_rev_bar {
            agg.n_rho_after_lambda += 1; // 三审计统计①（codex Q1 不变量）
        }
    }

    (decomps, agg)
}

/// bar close → 价格（close×tick），越界/非正返 0。
fn px_at(bars: &[Bar], i: usize, tick: f64) -> f64 {
    bars.get(i).map(|b| b.close as f64 * tick).filter(|&p| p > 0.0).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PDF §4 反例的分解算术自检（合成，L1 验证分解算术正确）：
    /// 多头反转交易腿 P[λ_rev]=10, P[ρ_rev]=12（Ab_rev=δ·2=2，δ=+1）；确认滞后 Pτin=11.8, Pτout=11.1。
    /// ηin=max(0,11.8−10)=1.8, ηout=max(0,12−11.1)=0.9, captured=2−1.8−0.9−Ce<0（执行吃光）。
    /// 664 号：λ_rev/ρ_rev 是入场/出场信号挂靠 pivot 端点价，δ=交易方向（非触发段笔方向 ε）。
    #[test]
    fn pdf_counterexample_decomp() {
        let eps = 1.0_f64; // 多头反转腿 δ=+1
        let (p_lambda, p_rho, p_tau_in, p_tau_out) = (10.0, 12.0, 11.8, 11.1);
        let a_b = eps * (p_rho - p_lambda);
        let eta_in = (eps * (p_tau_in - p_lambda)).max(0.0);
        let eta_out = (eps * (p_rho - p_tau_out)).max(0.0);
        let captured = a_b - eta_in - eta_out; // 无成本版
        assert!((a_b - 2.0).abs() < 1e-9);
        assert!((eta_in - 1.8).abs() < 1e-9);
        assert!((eta_out - 0.9).abs() < 1e-9);
        // 2 − 1.8 − 0.9 = −0.7 < 0：执行损耗吃光结构价差（PDF §4 因果亏 −0.7 的分解归因）。
        assert!(captured < 0.0, "captured={captured} 应 <0（PDF §4 反例：执行吃光价差）");
        assert!((captured - (-0.7)).abs() < 1e-9);
    }

    /// 664 号反转交易腿方向语义自检（合成，L1）：底背驰买点 δ=+1，入场 pivot 低、出场 pivot 高
    /// ⟹ Ab_rev=δ(P[ρ_rev]−P[λ_rev])>0；顶背驰卖点 δ=−1，入场 pivot 高、出场 pivot 低 ⟹ Ab_rev>0。
    /// 两侧反转腿价差都为正——证明 δ 作用在 post-signal 腿（入场→出场 pivot）上，不与触发段方向 ε 耦合。
    #[test]
    fn reversal_leg_direction_semantics() {
        // 底买反转腿：λ_rev=入场低点 pivot 100，ρ_rev=出场高点 pivot 120，δ=+1。
        let ab_buy = 1.0_f64 * (120.0 - 100.0);
        assert!(ab_buy > 0.0, "底买反转腿 Ab_rev 应 >0（入场低点→出场高点），实得 {ab_buy}");
        // 顶卖反转腿：λ_rev=入场高点 pivot 120，ρ_rev=出场低点 pivot 100，δ=−1。
        let ab_sell = (-1.0_f64) * (100.0 - 120.0);
        assert!(ab_sell > 0.0, "顶卖反转腿 Ab_rev 应 >0（入场高点→出场低点，δ=−1 翻正），实得 {ab_sell}");
        // 对照（664 号错配示意）：δ 作用在触发段（底买出现在下跌段末端，触发段 Pρ<Pλ）⟹ δ·(Pρ−Pλ)<0
        // ——这正是旧实装系统性 ΣAb<0 的来源（测错对象，非判据被否证）。
        let ab_old_trigger = 1.0_f64 * (100.0 - 120.0);
        assert!(ab_old_trigger < 0.0, "旧触发段对象 δ·(Pρ−Pλ) 系统性负（对象错配示意）");
    }

    /// spread_eaten 判据自检：Σcaptured≤0 ⟺ 损耗吃光。
    #[test]
    fn spread_eaten_iff_captured_nonpositive() {
        let mut agg = SpreadAttribution::default();
        agg.sum_captured = -0.1;
        assert!(agg.spread_eaten());
        agg.sum_captured = 0.1;
        assert!(!agg.spread_eaten());
    }

    /// 664-Q3 signed 分解恒等式自检（合成，L1）：actual_spread = Ab_rev − x_in − y_out 恒成立，
    /// 覆盖有利滑移（x<0/y<0）——证明 actual_spread 含全部滑移，而 captured(adverse-only)=Ab−max(0,x)−max(0,y) 丢有利滑移。
    #[test]
    fn signed_decomp_actual_spread_identity() {
        // 多头：λ_rev=100, ρ_rev=120 ⟹ Ab_rev=20。Pτin=98（确认价比入场 pivot 更优，x<0 有利），
        // Pτout=125（出场比 pivot 更高，y<0 有利）。δ=+1。
        for &(p_lambda, p_rho, p_tau_in, p_tau_out, delta) in &[
            (100.0, 120.0, 98.0, 125.0, 1.0_f64),  // 双侧有利滑移
            (100.0, 120.0, 105.0, 110.0, 1.0),     // 双侧不利滑移
            (120.0, 100.0, 122.0, 95.0, -1.0),     // 空头：入场更高(有利)、出场更低(有利)
        ] {
            let a_b = delta * (p_rho - p_lambda);
            let x_in = delta * (p_tau_in - p_lambda);
            let y_out = delta * (p_rho - p_tau_out);
            let actual_spread = delta * (p_tau_out - p_tau_in);
            // 恒等式：actual_spread = Ab_rev − x_in − y_out（codex Q3）。
            assert!((actual_spread - (a_b - x_in - y_out)).abs() < 1e-9,
                "signed 分解恒等式破：actual_spread={actual_spread} ≠ Ab−x−y={}", a_b - x_in - y_out);
            // adverse-only captured 与 actual_spread 的差 = min(x,0)+min(y,0) ≤ 0（系统性低估）。
            let captured_no_cost = a_b - x_in.max(0.0) - y_out.max(0.0);
            let diff = captured_no_cost - actual_spread;
            assert!(diff <= 1e-9,
                "adverse-only captured 应 ≤ actual_spread（丢有利滑移），diff={diff}");
            assert!((diff - (x_in.min(0.0) + y_out.min(0.0))).abs() < 1e-9,
                "captured−actual 应 = min(x,0)+min(y,0)");
        }
    }

    /// actual_pnl_proxy / actual_pnl_eaten 判据自检：与 spread_eaten 可分歧（codex Q3 核心）。
    #[test]
    fn actual_pnl_proxy_diverges_from_adverse_only() {
        let mut agg = SpreadAttribution::default();
        // 构造：actual_spread 正、但 adverse-only captured 负（有利滑移被丢导致伪「吃光」）。
        agg.sum_actual_spread = 100.0;
        agg.sum_ce = 30.0;
        agg.sum_captured = -50.0; // adverse-only 判定「吃光」
        assert!(agg.spread_eaten(), "adverse-only 判吃光");
        assert!((agg.actual_pnl_proxy() - 70.0).abs() < 1e-9);
        assert!(!agg.actual_pnl_eaten(), "真实成交口径未吃光 ⟹ adverse-only 是伪结论");
    }

    /// L2 重测「钱去哪了」：BTC 全历史逐信号反转交易腿 Ab_rev 可捕获价差分解（真实数据，可产否定性结果；664 号）。
    ///
    /// `#[ignore]`：需 BTC 全量数据（314M）+ O(n²) 逐 bar 重分类，`--release` 必须。重跑由 Lead。
    /// 报告落盘 `.chanlun/review-results/econ-abrev-l2-btc-20260630.md`（确定性，可复算；不覆盖旧触发段报告）。
    ///
    /// **L2 纪律**：spread_eaten=true ⟹ 该信号集执行损耗吃光反转交易腿价差（否证可交易，有效域收窄）；
    /// spread_eaten=false ⟹ 该级别有可捕获 alpha——两者照实报，不粉饰（161/formalization-validity-domain）。
    #[test]
    #[ignore]
    fn l2_btc_capturable_spread_diagnosis() {
        use super::super::data;
        use std::collections::BTreeMap;
        use std::fmt::Write as _;

        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => panic!("BTC 加载失败：{e}（DATA BLOCKER，不伪造合成）"),
        };
        let n_full = ds_full.bars.len();

        // 截断窗（显式有效域边界，非全窗结论；l3_delta_r_alpha::MAX_BARS 同纪律）：
        // BTC 全量 461 万 bar 逐 bar 增量重分类 cache（每 bar 一份 Rc<塔>）跨 bar 累积 ⟹ 内存不可行
        // （实测 500K bar OOM 被杀，300K=67s 可行、是内存上限）。取**最后** MAX_BARS（近期行情，
        // 与近期可交易性相关）作截断窗。env ECON_L2_MAX_BARS 覆盖供 Lead 调窗（50K/150K/300K 实测一致）。
        const MAX_BARS: usize = 300_000;
        let max_bars = std::env::var("ECON_L2_MAX_BARS").ok()
            .and_then(|s| s.parse().ok()).unwrap_or(MAX_BARS);
        let ds = if n_full > max_bars {
            let start = ds_full.dates[n_full - max_bars].get(..10).unwrap_or("").to_string();
            let end = ds_full.dates[n_full - 1].get(..10).unwrap_or("").to_string();
            eprintln!("截断窗 [{start}→{end}]（最后 {max_bars} bar / 全量 {n_full}）= 显式有效域边界");
            ds_full.slice_date_window(&start, &end)
        } else {
            ds_full
        };
        let untradable = ds.untradable_ratio();
        let n_bars = ds.bars.len();
        let window_start = ds.dates.first().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        let window_end = ds.dates.last().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();

        let (decomps, agg) = decompose_capturable_spread(&ds, &config);

        // per-class (level, δ) 分桶：adverse-only Σcaptured + 真实成交 Σactual_pnl 双口径（664-Q3）。
        // key=(level, δ)，value=(n, Σab, Σηin, Σηout, Σce, Σcaptured, n_cap_pos, Σactual_spread, Σactual_pnl, n_act_pos)。
        type Bucket = (usize, f64, f64, f64, f64, f64, usize, f64, f64, usize);
        let mut buckets: BTreeMap<(u32, i8), Bucket> = BTreeMap::new();
        for d in &decomps {
            let e = buckets.entry((d.level, d.delta)).or_default();
            e.0 += 1;
            e.1 += d.a_b;
            e.2 += d.eta_in;
            e.3 += d.eta_out;
            e.4 += d.ce_unit;
            e.5 += d.captured;
            if d.captured > 0.0 {
                e.6 += 1;
            }
            e.7 += d.actual_spread;
            e.8 += d.actual_pnl;
            if d.actual_pnl > 0.0 {
                e.9 += 1;
            }
        }

        // 失血三源占比（分母 = Σ(ηin+ηout+Ce) 总损耗；ΣAb 为正分母比对结构价差）。
        let total_drain = agg.sum_eta_in + agg.sum_eta_out + agg.sum_ce;
        let pct = |x: f64| if total_drain > 0.0 { 100.0 * x / total_drain } else { 0.0 };

        let actual_pnl_proxy = agg.actual_pnl_proxy();

        let mut rpt = String::new();
        let _ = writeln!(rpt, "# 经济正条件⑤ L2 重测：BTC signed 滑移分解（adverse-only vs 真实成交，664-Q3 codex 口径修正）");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "**认识论等级**：L2（真实数据单标的逐信号确定性分解，可产否定性结果）。");
        let _ = writeln!(rpt, "**664-Q3 修正**：旧 captured=Ab_rev−max(0,x)−max(0,y)−Ce 用 max(0,·) **丢有利滑移、全计不利滑移** ⟹ 系统性低估真实 PnL（captured−actual_pnl=min(x,0)+min(y,0)≤0）= **adverse-only 保守压力测试，非真实成交**。本报告补 signed Σx/Σy 与真实成交价差 actual_spread=δ(Pτout−Pτin)=Ab_rev−x−y。");
        let _ = writeln!(rpt, "**两口径**：x=δ(Pτin−Pλ_rev)（signed 入场滑移）、y=δ(Pρ_rev−Pτout)（signed 出场滑移）。");
        let _ = writeln!(rpt, "- **adverse-only captured** = Ab_rev−max(0,x)−max(0,y)−Ce（压力测试，保守，保留兼容旧口径）。");
        let _ = writeln!(rpt, "- **真实成交 actual_pnl** = actual_spread−Ce = δ(Pτout−Pτin)−Ce（含全部滑移，更接近真实成交）。");
        let _ = writeln!(rpt, "**复算**：`cargo test -p <crate> --release l2_btc_capturable_spread_diagnosis -- --ignored --nocapture`（确定性）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 数据");
        let _ = writeln!(rpt, "- 品种：BTC（btc_1m_full.json，全量 {n_full} bar，2017-08→2026-05）");
        let _ = writeln!(rpt, "- **截断窗 [{window_start}→{window_end}]，bars={n_bars}**（最后 {max_bars} bar；全量 461万 OOM 不可行 ⟹ 截断窗=显式有效域边界，非全窗结论，l3 同纪律）");
        let _ = writeln!(rpt, "- untradable_ratio={:.4}", untradable);
        let _ = writeln!(rpt, "- 收集信号数 n_signals={}（无配对出场反转信号的入场信号被诚实跳过，不入此集——无 ρ_rev 不兜底）", agg.n_signals);
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 三审计统计（codex #97 要求）");
        let _ = writeln!(rpt, "| 统计 | 值 | 说明 |");
        let _ = writeln!(rpt, "|---|---|---|");
        let _ = writeln!(rpt, "| n_rho_after_lambda | {}/{} | rho_rev_bar>lambda_rev_bar（codex Q1 不变量：出场 pivot 端点在入场 pivot 之后） |", agg.n_rho_after_lambda, agg.n_signals);
        let _ = writeln!(rpt, "| n_same_bar_opposite | {} | 同 bar 出现反向信号（被 eb>entry_bar 排除的边界，codex Q2） |", agg.n_same_bar_opposite);
        let _ = writeln!(rpt, "| n_unpaired | {} | 无配对出场反转信号（右删失诚实跳过，codex Q4） |", agg.n_unpaired);
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 全局归因（双口径）");
        let _ = writeln!(rpt, "| 量 | 值 |");
        let _ = writeln!(rpt, "|---|---|");
        let _ = writeln!(rpt, "| ΣAb_rev（反转交易腿理想总价差） | {:.6e} |", agg.sum_a_b);
        let _ = writeln!(rpt, "| Σx（signed 入场滑移） | {:.6e} |", agg.sum_x_in);
        let _ = writeln!(rpt, "| Σy（signed 出场滑移） | {:.6e} |", agg.sum_y_out);
        let _ = writeln!(rpt, "| Σmax(0,x)=Σηin（adverse-only 入场） | {:.6e} ({:.1}%) |", agg.sum_eta_in, pct(agg.sum_eta_in));
        let _ = writeln!(rpt, "| Σmax(0,y)=Σηout（adverse-only 出场） | {:.6e} ({:.1}%) |", agg.sum_eta_out, pct(agg.sum_eta_out));
        let _ = writeln!(rpt, "| ΣCe（成本） | {:.6e} ({:.1}%) |", agg.sum_ce, pct(agg.sum_ce));
        let _ = writeln!(rpt, "| Σactual_spread（真实成交价差 δ(Pτout−Pτin)） | {:.6e} |", agg.sum_actual_spread);
        let _ = writeln!(rpt, "| **Σcaptured（adverse-only 压力测试剩余）** | {:.6e} |", agg.sum_captured);
        let _ = writeln!(rpt, "| **actual_pnl_proxy=Σactual_spread−ΣCe（真实成交 PnL 代理）** | {:.6e} |", actual_pnl_proxy);
        let _ = writeln!(rpt, "| n_captured_positive/n（adverse-only 正占比） | {}/{} ({:.1}%) |",
            agg.n_captured_positive, agg.n_signals,
            if agg.n_signals > 0 { 100.0 * agg.n_captured_positive as f64 / agg.n_signals as f64 } else { 0.0 });
        let _ = writeln!(rpt, "| n_actual_positive/n（真实成交正占比） | {}/{} ({:.1}%) |",
            agg.n_actual_positive, agg.n_signals,
            if agg.n_signals > 0 { 100.0 * agg.n_actual_positive as f64 / agg.n_signals as f64 } else { 0.0 });
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## L2 判定（关键：两口径分歧 = codex Q3 核心）");
        let _ = writeln!(rpt, "- **adverse-only spread_eaten = {}**（Σcaptured {} 0）", agg.spread_eaten(), if agg.spread_eaten() { "≤" } else { ">" });
        let _ = writeln!(rpt, "- **真实成交 actual_pnl_eaten = {}**（actual_pnl_proxy {} 0）", agg.actual_pnl_eaten(), if agg.actual_pnl_eaten() { "≤" } else { ">" });
        if agg.spread_eaten() && !agg.actual_pnl_eaten() {
            let _ = writeln!(rpt, "- **翻案（codex Q3 坐实）**：adverse-only 判「执行吃光」但真实成交 actual_pnl_proxy>0 ⟹ 之前「执行滞后吃光」(Σcaptured=−2.33e5) 是 **adverse-only 伪结论**——有利滑移被 max(0,·) 丢弃所致。真实成交口径下反转腿可捕获。");
        } else if agg.actual_pnl_eaten() {
            let _ = writeln!(rpt, "- **「执行吃光」成立（真实成交口径）**：actual_pnl_proxy≤0 ⟹ 即便不丢有利滑移，真实成交价差减成本仍非正 ⟹ 该信号集真实亏（否定性结果，缩小有效域边界，161/formalization-validity-domain）。");
        } else {
            let _ = writeln!(rpt, "- **两口径一致正**：adverse-only 与真实成交均 >0 ⟹ 反转腿价差未被吃光（保守口径都过 ⟹ 真实更宽松）。仅 BTC 单标的 L2，非跨品种功效。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 失血三源对比（ΣAb_rev 为反转腿结构上限）");
        let _ = writeln!(rpt, "- 反转腿价差：ΣAb_rev={:.6e}（664 号测对了对象——可正可负，非触发段同义反复）", agg.sum_a_b);
        let _ = writeln!(rpt, "- 执行吃光：Ση={:.6e}（入场+出场滞后）", agg.sum_eta_in + agg.sum_eta_out);
        let _ = writeln!(rpt, "- 成本：ΣCe={:.6e}", agg.sum_ce);
        if agg.sum_a_b < 0.0 {
            let _ = writeln!(rpt);
            let _ = writeln!(rpt, "**ΣAb_rev<0（反转交易腿）：** 664 号修复后仍为负 ⟹ 不是对象错配（已测对反转腿），是反转交易腿");
            let _ = writeln!(rpt, "本身的真实价差为负——配对出场信号 pivot 相对入场信号 pivot 在交易方向 δ 上整体不利。这是测对");
            let _ = writeln!(rpt, "对象之后的真实否定性结果（231：缩小有效域边界），不靠改符号修复（改 eps=恢复同义反复，090+no-patch）。");
            let _ = writeln!(rpt, "即使零滞后零成本（Ση=ΣCe=0），captured=ΣAb_rev<0 仍无 alpha——但这次测的是反转交易腿，不是触发段。");
        } else {
            let _ = writeln!(rpt);
            let _ = writeln!(rpt, "**ΣAb_rev>0（反转交易腿）：** 664 号修复后反转腿理想价差为正 ⟹ 结构给了可捕获价差（与旧触发段 ΣAb=−1.8e4");
            let _ = writeln!(rpt, "形成对照——后者是测错对象的伪否证）。剩余 alpha = ΣAb_rev − Ση − ΣCe（执行/成本是否吃光见上表 Σcaptured）。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## per-class (level, δ) 分桶（双口径）");
        let _ = writeln!(rpt, "| level | δ | n | ΣAb_rev | Ση(adv) | ΣCe | Σcaptured(adv) | actual_pnl | n_cap+/n | n_act+/n |");
        let _ = writeln!(rpt, "|---|---|---|---|---|---|---|---|---|---|");
        for ((lvl, dlt), (n, sab, sin, sout, sce, scap, ncap, _sact, sactpnl, nact)) in &buckets {
            let _ = writeln!(rpt, "| {} | {:+} | {} | {:.4e} | {:.4e} | {:.4e} | {:.4e} | {:.4e} | {}/{} ({:.0}%) | {}/{} ({:.0}%) |",
                lvl, dlt, n, sab, sin + sout, sce, scap, sactpnl,
                ncap, n, if *n > 0 { 100.0 * *ncap as f64 / *n as f64 } else { 0.0 },
                nact, n, if *n > 0 { 100.0 * *nact as f64 / *n as f64 } else { 0.0 });
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 663 原生口径（真实成交 actual_pnl>0 状态类）");
        let _ = writeln!(rpt, "actual_pnl>0 的 (level,δ) 类 = 该类真实成交逐信号路径级净正（出现即做，真实成交口径，非 adverse-only）：");
        let mut any_pos_class = false;
        for ((lvl, dlt), (n, _, _, _, _, _, _, _, sactpnl, nact)) in &buckets {
            if *sactpnl > 0.0 {
                any_pos_class = true;
                let _ = writeln!(rpt, "- (level={}, δ={:+}): actual_pnl={:.4e}, n_act+/n={}/{}", lvl, dlt, sactpnl, nact, n);
            }
        }
        if !any_pos_class {
            let _ = writeln!(rpt, "- （无 actual_pnl>0 的类——全级别全方向真实成交亏，否定性结果）");
        }

        eprint!("{rpt}");

        // 落盘 signed 报告（664-Q3，不覆盖 econ-abrev-l2-btc-20260630.md 旧 adverse-only-only 报告）。
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().expect("rust/ 父目录 = 项目根")
            .join(".chanlun/review-results/econ-abrev-signed-l2-20260630.md");
        std::fs::write(&out, &rpt).unwrap_or_else(|e| panic!("写报告 {out:?} 失败：{e}"));
        eprintln!("\n报告已落盘：{out:?}");

        // 真封①：adverse-only captured 分解恒等式逐信号成立。
        for d in &decomps {
            let recomputed = d.a_b - d.eta_in - d.eta_out - d.ce_unit;
            assert!((d.captured - recomputed).abs() < 1e-6,
                "captured 分解恒等式破：level={} δ={} captured={} ≠ {}", d.level, d.delta, d.captured, recomputed);
            // 真封②（664-Q3 核心）：actual_spread = Ab_rev − x − y 逐信号恒成立（signed 分解）。
            assert!((d.actual_spread - (d.a_b - d.x_in - d.y_out)).abs() < 1e-6,
                "actual_spread 恒等式破：level={} δ={} actual_spread={} ≠ Ab−x−y={}",
                d.level, d.delta, d.actual_spread, d.a_b - d.x_in - d.y_out);
            // 真封③：eta = max(0, signed) 一致。
            assert!((d.eta_in - d.x_in.max(0.0)).abs() < 1e-9 && (d.eta_out - d.y_out.max(0.0)).abs() < 1e-9,
                "η 应 = max(0, signed)：level={} δ={}", d.level, d.delta);
            // 真封④：captured ≤ actual_pnl（adverse-only 系统性 ≤ 真实成交；丢有利滑移）。
            assert!(d.captured <= d.actual_pnl + 1e-6,
                "adverse-only captured 应 ≤ actual_pnl：level={} δ={} captured={} > actual_pnl={}",
                d.level, d.delta, d.captured, d.actual_pnl);
        }
        // 聚合 Σ 无丢失。
        let sum_check: f64 = decomps.iter().map(|d| d.captured).sum();
        assert!((agg.sum_captured - sum_check).abs() < 1e-3,
            "Σcaptured 聚合 {} ≠ 逐信号和 {}", agg.sum_captured, sum_check);
        let sum_act: f64 = decomps.iter().map(|d| d.actual_spread).sum();
        assert!((agg.sum_actual_spread - sum_act).abs() < 1e-3,
            "Σactual_spread 聚合 {} ≠ 逐信号和 {}", agg.sum_actual_spread, sum_act);
    }
}
