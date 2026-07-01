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
                // ponytail（fullhist-oom-fix-20260630）: single 的空 moves/centers 不是冗余——它**屏蔽其他层
                // bsp**，只让这一个 bsp 产单候选。elements 从 tower 提取，classification 只供 bsp 做 Γ 组装；
                // dir 来自该 bsp 的 coverage role 判定，非 tower 裸结构可读。此重建仅每新信号触发（600K bar=1682
                // 次，万级噪声非 O(bar) 主导），勿当 O(N×L) 去重写 coverage 核心——会破 bit-exact。O(n²) 真因在
                // classify_at 每 bar O(tree) 续算（#104/#105/#106 generation 快路）。
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
        // ★fullhist-oom-fix-20260630 订正：旧注释「500K OOM 被杀=内存上限」是**误诊**（错误归因，090号）。
        // /usr/bin/time -l 实测峰值 RSS：100K=1990MB、200K=2010MB、400K=2034MB、600K=2083MB——
        // 内存随 bar **平坦**（100K→600K 仅 +5%），无 OOM。塔状态（TowerCache.upper_moves）累积是塔元素数
        // （走势/中枢，亚线性于 bar），不是「每 bar 一份 Rc 副本」。旧「500K OOM」实为 **O(n²) 时间墙超时被
        // kill** 被误报为 OOM。真瓶颈 = classify_with_tower_incremental 的 O(tree)/bar 续算（profile_classify_at
        // 已坐实 t_exp≈2.0；归 Task #104/#105 classifier 核心优化）。截断窗在此**为时间非内存**：实测
        // 100K=8.7s、200K=31.8s、400K=124.5s、600K=283s（O(n²)），全量 461万≈数小时可跑通但慢。
        // env ECON_L2_MAX_BARS 覆盖供 Lead 调窗（>4.6M=不截断跑全量）。
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
        crate::theta_v0::classifier::stage_profile::dump();

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

    /// per-class (level,δ) 真实成交统计：(n, Σactual_pnl, n_act+, Σab_rev)。OOS 测试复用，避免重复分桶。
    fn class_actual_pnl(decomps: &[SignalDecomp], level: u32, delta: i8) -> (usize, f64, usize, f64) {
        decomps.iter().filter(|d| d.level == level && d.delta == delta).fold(
            (0usize, 0.0f64, 0usize, 0.0f64),
            |(n, pnl, npos, ab), d| {
                (n + 1, pnl + d.actual_pnl, npos + (d.actual_pnl > 0.0) as usize, ab + d.a_b)
            },
        )
    }

    /// 窗口净涨跌方向：首尾 close 总收益**百分数**（>0 涨段 / <0 跌段）。tick 抵消，直接用 close 整数比。
    fn window_net_return(ds: &Dataset) -> f64 {
        let (first, last) = (ds.bars.first(), ds.bars.last());
        match (first, last) {
            (Some(f), Some(l)) if f.close > 0 => 100.0 * (l.close as f64 - f.close as f64) / f.close as f64,
            _ => 0.0,
        }
    }

    /// **663 推论 OOS 验证（防 winner's curse）**：对 in-sample 最强正类 level0卖（level=0,δ=−1，
    /// in-sample actual_pnl=+3.47e4）做 train/holdout 时间切分 + 方向不对称分解。
    ///
    /// `#[ignore]`：同 l2_btc 需 BTC 全量 + O(n²) 重分类，`--release`。
    ///
    /// **L2 纪律**：holdout level0卖 actual_pnl≤0 ⟹ +3.47e4 是窗口/挑赢家产物（否证 alpha，照实报）；
    /// >0 ⟹ OOS 初步稳健（仍单标的单切分，标有效域）。否定性结果照实报（161/formalization-validity-domain）。
    #[test]
    #[ignore]
    fn acc_level0sell_oos() {
        use super::super::data;
        use std::fmt::Write as _;

        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => panic!("BTC 加载失败：{e}（DATA BLOCKER，不伪造合成）"),
        };
        let n_full = ds_full.bars.len();

        // 截断窗（同 l2_btc_capturable_spread_diagnosis：最后 MAX_BARS，OOM 边界=显式有效域）。
        const MAX_BARS: usize = 300_000;
        let max_bars = std::env::var("ECON_L2_MAX_BARS").ok()
            .and_then(|s| s.parse().ok()).unwrap_or(MAX_BARS);
        let ds = if n_full > max_bars {
            ds_full.slice_bar_range(n_full - max_bars, n_full)
        } else {
            ds_full
        };
        let n = ds.bars.len();
        let win_start = ds.dates.first().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        let win_end = ds.dates.last().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();

        // ── 任务1：train(前 frac)/holdout(后 1−frac) 时间切分（半开区间无重叠，时间序不打乱）。 ──
        let train_frac = std::env::var("ECON_L2_TRAIN_FRAC").ok()
            .and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.6);
        let split = (n as f64 * train_frac) as usize;
        let ds_train = ds.slice_bar_range(0, split);
        let ds_hold = ds.slice_bar_range(split, n);
        let train_split_day = ds.dates[split].get(..10).unwrap_or("").to_string();

        let (decomps_train, _) = decompose_capturable_spread(&ds_train, &config);
        let (decomps_hold, _) = decompose_capturable_spread(&ds_hold, &config);
        let (tr_n, tr_pnl, tr_pos, tr_ab) = class_actual_pnl(&decomps_train, 0, -1);
        let (hd_n, hd_pnl, hd_pos, hd_ab) = class_actual_pnl(&decomps_hold, 0, -1);
        // 对照：买（δ=+1，in-sample 亏损主源）OOS 是否仍亏。
        let (hd_buy_n, hd_buy_pnl, hd_buy_pos, _) = class_actual_pnl(&decomps_hold, 0, 1);

        let train_ret = window_net_return(&ds_train);
        let hold_ret = window_net_return(&ds_hold);
        let full_ret = window_net_return(&ds);

        // ── 任务2：2 个不重叠子窗（前半/后半，各 n/2），分别算 level0卖 vs level0买。 ──
        let half = n / 2;
        let ds_w1 = ds.slice_bar_range(0, half);
        let ds_w2 = ds.slice_bar_range(half, n);
        let (dw1, _) = decompose_capturable_spread(&ds_w1, &config);
        let (dw2, _) = decompose_capturable_spread(&ds_w2, &config);
        let (w1_s_n, w1_sell, _, _) = class_actual_pnl(&dw1, 0, -1);
        let (w1_b_n, w1_buy, _, _) = class_actual_pnl(&dw1, 0, 1);
        let (w2_s_n, w2_sell, _, _) = class_actual_pnl(&dw2, 0, -1);
        let (w2_b_n, w2_buy, _, _) = class_actual_pnl(&dw2, 0, 1);
        let w1_ret = window_net_return(&ds_w1);
        let w2_ret = window_net_return(&ds_w2);

        // ── 判定 ──
        let oos_robust = hd_pnl > 0.0; // holdout 卖正 ⟹ 初步稳健
        // 方向不对称结构性 ⟺ 两个不重叠子窗 level0卖都优于买（不论子窗涨跌）。
        let sell_beats_buy_w1 = w1_sell > w1_buy;
        let sell_beats_buy_w2 = w2_sell > w2_buy;
        let asym_structural = sell_beats_buy_w1 && sell_beats_buy_w2;

        let mut rpt = String::new();
        let _ = writeln!(rpt, "# 663 推论 OOS 验证：level0卖 train/holdout 切分 + 方向不对称（防 winner's curse）");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "**认识论等级**：L2（真实数据单标的单切分，可产否定性结果；非 L3 跨标的）。");
        let _ = writeln!(rpt, "**对象**：in-sample 最强正类 level0卖（level=0, δ=−1，in-sample actual_pnl=+3.47e4/349/36%）。");
        let _ = writeln!(rpt, "**复算**：`cargo test -p <crate> --release acc_level0sell_oos -- --ignored --nocapture`（确定性）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 数据 / 窗口");
        let _ = writeln!(rpt, "- BTC 全量 {n_full} bar；**截断窗 [{win_start}→{win_end}]，bars={n}**（最后 {max_bars} bar；OOM 边界=显式有效域，非全历史，同 l2_btc 纪律）。");
        let _ = writeln!(rpt, "- train_frac={train_frac}，切分 bar 索引={split}（切分日 {train_split_day}，半开区间 train=[0,{split}) / holdout=[{split},{n}) 无重叠）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 窗净涨跌方向（首尾 close 总收益）");
        let _ = writeln!(rpt, "| 窗 | 净收益 | 方向 |");
        let _ = writeln!(rpt, "|---|---|---|");
        let _ = writeln!(rpt, "| 截断全窗 | {full_ret:+.2}% | {} |", if full_ret > 0.0 { "涨" } else { "跌" });
        let _ = writeln!(rpt, "| train | {train_ret:+.2}% | {} |", if train_ret > 0.0 { "涨" } else { "跌" });
        let _ = writeln!(rpt, "| holdout | {hold_ret:+.2}% | {} |", if hold_ret > 0.0 { "涨" } else { "跌" });
        let _ = writeln!(rpt, "| 子窗1（前半） | {w1_ret:+.2}% | {} |", if w1_ret > 0.0 { "涨" } else { "跌" });
        let _ = writeln!(rpt, "| 子窗2（后半） | {w2_ret:+.2}% | {} |", if w2_ret > 0.0 { "涨" } else { "跌" });
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 任务1：level0卖 train vs holdout（关键——正负决定 winner's curse 判定）");
        let _ = writeln!(rpt, "| 窗 | n | Σactual_pnl | n_act+/n（胜率） | Σab_rev |");
        let _ = writeln!(rpt, "|---|---|---|---|---|");
        let _ = writeln!(rpt, "| train | {tr_n} | {tr_pnl:.4e} | {tr_pos}/{tr_n} ({:.0}%) | {tr_ab:.4e} |", winrate(tr_pos, tr_n));
        let _ = writeln!(rpt, "| **holdout** | {hd_n} | **{hd_pnl:.4e}** | {hd_pos}/{hd_n} ({:.0}%) | {hd_ab:.4e} |", winrate(hd_pos, hd_n));
        let _ = writeln!(rpt, "| holdout 对照·买(δ+1) | {hd_buy_n} | {hd_buy_pnl:.4e} | {hd_buy_pos}/{hd_buy_n} ({:.0}%) | — |", winrate(hd_buy_pos, hd_buy_n));
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "**判定**：holdout level0卖 actual_pnl = {hd_pnl:.4e} {} 0", if hd_pnl > 0.0 { ">" } else { "≤" });
        if oos_robust {
            let _ = writeln!(rpt, "→ **OOS 初步稳健**：holdout 卖仍正，+3.47e4 不是纯挑赢家产物。**但仅 BTC 单标的单切分 L2，非 L3**——holdout 窗净涨跌={hold_ret:+.2}%，若 holdout 仍跌段则方向效应未排除（见任务2）。");
        } else {
            let _ = writeln!(rpt, "→ **否证 alpha（winner's curse 坐实）**：holdout 卖 actual_pnl≤0 ⟹ in-sample +3.47e4 是窗口/挑赢家产物。这是有价值的否定性结果（缩小有效域边界，161/formalization-validity-domain）——照实报，不粉饰。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 任务2：方向不对称（2 个不重叠子窗，结构性 vs 窗口效应）");
        let _ = writeln!(rpt, "| 子窗 | 净涨跌 | 卖(δ−1) actual_pnl | n卖 | 买(δ+1) actual_pnl | n买 | 卖>买? |");
        let _ = writeln!(rpt, "|---|---|---|---|---|---|---|");
        let _ = writeln!(rpt, "| 1（前半） | {w1_ret:+.2}% | {w1_sell:.4e} | {w1_s_n} | {w1_buy:.4e} | {w1_b_n} | {} |", if sell_beats_buy_w1 { "是" } else { "否" });
        let _ = writeln!(rpt, "| 2（后半） | {w2_ret:+.2}% | {w2_sell:.4e} | {w2_s_n} | {w2_buy:.4e} | {w2_b_n} | {} |", if sell_beats_buy_w2 { "是" } else { "否" });
        let _ = writeln!(rpt);
        if asym_structural {
            let _ = writeln!(rpt, "**判定**：两个不重叠子窗 level0卖均优于买。");
            if w1_ret * w2_ret < 0.0 {
                let _ = writeln!(rpt, "→ **结构性（跨涨跌都卖优）**：子窗1/子窗2 净涨跌符号相反（一涨一跌）卖仍都优 ⟹ 卖优势非单纯下跌段做空产物，是结构性方向不对称。L2 单标的。");
            } else {
                let _ = writeln!(rpt, "→ **方向同号，未充分隔离**：两子窗净涨跌同号（{w1_ret:+.2}%/{w2_ret:+.2}%），卖优势可能仍含方向效应——结构性结论需跨涨跌子窗或 L3 跨标的。");
            }
        } else {
            let _ = writeln!(rpt, "**判定**：卖优势在子窗间不一致（子窗1卖>买={sell_beats_buy_w1}，子窗2={sell_beats_buy_w2}）。");
            let _ = writeln!(rpt, "→ **窗口效应**：卖优势依赖特定子窗（很可能是下跌子窗做空），非结构性。否定性结果照实报。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 结果包六要素");
        let _ = writeln!(rpt, "1. **结论**：holdout level0卖 actual_pnl={hd_pnl:.4e}（{}），方向不对称={}。", if oos_robust { "OOS 初步稳健" } else { "winner's curse 否证" }, if asym_structural { "结构性候选" } else { "窗口效应" });
        let _ = writeln!(rpt, "2. **定义依据**：level0卖=663 in-sample 最强正类（level=0/δ=−1/顶背驰卖空反转腿）；actual_pnl=δ(Pτout−Pτin)−Ce 真实成交口径（664-Q3）。");
        let _ = writeln!(rpt, "3. **边界条件**：holdout 窗净涨跌={hold_ret:+.2}%；若 holdout 为下跌段（做空天然赚），OOS 正不足以证 alpha（需跨涨跌子窗，见任务2）。train_frac={train_frac} 改变切分点结论可能翻转（单切分脆弱）。");
        let _ = writeln!(rpt, "4. **下游推论**：{}", if oos_robust { "level0卖可作信号层 entry 候选，但须 L3 跨标的 + 涨段验证后才升基座（防方向效应）。" } else { "level0卖不可单独作 entry——in-sample 正是窗口产物，下游策略勿基于此类升基座。" });
        let _ = writeln!(rpt, "5. **谱系引用**：663 econpositive 推论；664 对象错配修复（δ=反转腿方向）；formalization-validity-domain（L2 有效域 < 定义域）；161（务实=把缺口留后面）。");
        let _ = writeln!(rpt, "6. **影响声明**：新增 Dataset::slice_bar_range（半开区间切片，复用 source_index 重置契约）+ acc_level0sell_oos 测试；不改 decompose_capturable_spread/TradeRecord/Order。");

        eprint!("{rpt}");
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().expect("rust/ 父目录 = 项目根")
            .join(".chanlun/review-results/econpositive-oos-level0sell-20260630.md");
        std::fs::write(&out, &rpt).unwrap_or_else(|e| panic!("写报告 {out:?} 失败：{e}"));
        eprintln!("\n报告已落盘：{out:?}");

        // 真封：train+holdout 信号数之和与全窗同口径无丢失（半开切分无重叠无遗漏 ⟹ 级别涌现局部性除外，
        // 切窗会改变级别涌现 ⟹ 不强求 n 守恒；仅断言切片本身非空、actual_pnl 与逐信号和一致）。
        let recomputed_hold: f64 = decomps_hold.iter().filter(|d| d.level == 0 && d.delta == -1)
            .map(|d| d.actual_pnl).sum();
        assert!((hd_pnl - recomputed_hold).abs() < 1e-6, "holdout level0卖 Σactual_pnl 聚合不一致");
        assert!(ds_train.bars.len() + ds_hold.bars.len() == n, "train+holdout bar 数 ≠ 全窗（半开区间应无重叠无遗漏）");
    }

    fn winrate(pos: usize, n: usize) -> f64 {
        if n > 0 { 100.0 * pos as f64 / n as f64 } else { 0.0 }
    }

    /// **PDF §11 正确 OOS 验收（除 codex Q2 BIAS-FATAL 选择偏差）**：train-only 挑类 → 锁 holdout
    /// 评估 + 多窗滚动 walk-forward + neff + block bootstrap + 剔最大赢家。
    ///
    /// 与 `acc_level0sell_oos`（被否证：从含 holdout 全样本挑 level0卖）的关键差异：
    /// **选类只用 train 段**（codex/PDF§6：用挑赢家同一数据验证不能反驳挑赢家）。train 赢家可能 ≠ level0卖。
    ///
    /// `#[ignore]`：同上需 BTC 全量 + O(n²) 重分类 ×（1 主切分 + K 滚动窗），`--release`。
    ///
    /// **L2 纪律（161/formalization-validity-domain）**：任一结果照实报——
    /// train 赢家=level0卖 且 holdout 正 ⟹ 缠论买卖点首个干净 OOS alpha 证据；
    /// train 赢家≠level0卖 或 holdout 翻负 ⟹ +7.87e3 是选择偏差产物（诚实否证，缩有效域）。
    #[test]
    #[ignore]
    fn acc_walkforward_trainonly() {
        use super::super::data;
        use std::fmt::Write as _;

        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => panic!("BTC 加载失败：{e}（DATA BLOCKER，不伪造合成）"),
        };
        let n_full = ds_full.bars.len();
        const MAX_BARS: usize = 300_000;
        let max_bars = std::env::var("ECON_L2_MAX_BARS").ok()
            .and_then(|s| s.parse().ok()).unwrap_or(MAX_BARS);
        let ds = if n_full > max_bars { ds_full.slice_bar_range(n_full - max_bars, n_full) } else { ds_full };
        let n = ds.bars.len();
        let win_start = ds.dates.first().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        let win_end = ds.dates.last().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();

        // ══ 任务1+2：主切分 train-only 挑类 → 锁 holdout 评估（除 Q2 选择偏差核心）══
        let train_frac = std::env::var("ECON_L2_TRAIN_FRAC").ok()
            .and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.6);
        let split = (n as f64 * train_frac) as usize;
        let ds_train = ds.slice_bar_range(0, split);
        let ds_hold = ds.slice_bar_range(split, n);
        let split_day = ds.dates[split].get(..10).unwrap_or("").to_string();

        let (decomps_train, _) = decompose_capturable_spread(&ds_train, &config);
        let (decomps_hold, _) = decompose_capturable_spread(&ds_hold, &config);

        // train-only 挑赢家（不看 holdout）。记录是否 level0卖。
        let winner = train_winner_class(&decomps_train);
        let is_level0_sell = winner == Some((0, -1));
        let train_l0sell = class_actual_pnl(&decomps_train, 0, -1);

        // 锁 holdout 评估 train 选出的类。
        let (hd_n, hd_pnl, hd_pos, hd_ab, hd_pnls) = match winner {
            Some((lv, dl)) => {
                let (n_, pnl, npos, ab) = class_actual_pnl(&decomps_hold, lv, dl);
                let pnls: Vec<f64> = decomps_hold.iter()
                    .filter(|d| d.level == lv && d.delta == dl).map(|d| d.actual_pnl).collect();
                (n_, pnl, npos, ab, pnls)
            }
            None => (0, 0.0, 0, 0.0, Vec::new()),
        };
        let oos_mu_positive = hd_pnl > 0.0 && winner.is_some(); // μ_OOS>0（弱：未扣不确定性）

        // 任务4：neff vs nraw（holdout 赢家类逐笔 PnL 自相关）。
        let nraw = hd_pnls.len();
        let (neff, sum_rho) = neff_autocorr(&hd_pnls, 20);
        let (mu_oos, se_oos, lcb_oos) = mean_se_lcb(&hd_pnls, neff);

        // 任务5/Q4：剔最大赢家 + block bootstrap p。
        let (q4_full, q4_d1, q4_d3, q4_d5) = drop_top_winners(&hd_pnls);
        let q4_p = block_bootstrap_pvalue(&hd_pnls, 20, 2000);

        // ══ 任务3：多窗滚动 walk-forward（除 Q5 单切分脆弱）══
        let k_windows: usize = std::env::var("ECON_WF_WINDOWS").ok()
            .and_then(|s| s.parse().ok()).unwrap_or(4);
        let wf_train_frac = 0.6;
        let win_len = n / k_windows;
        let mut wf_rows: Vec<(Option<(u32, i8)>, bool, usize, f64, f64, bool)> = Vec::new();
        let mut lcb_pos_count = 0usize;
        let mut l0sell_winner_count = 0usize;
        for w in 0..k_windows {
            let w_start = w * win_len;
            let w_end = if w + 1 == k_windows { n } else { (w + 1) * win_len };
            let w_mid = w_start + ((w_end - w_start) as f64 * wf_train_frac) as usize;
            if w_mid <= w_start || w_end <= w_mid { continue; }
            let dtr = decompose_capturable_spread(&ds.slice_bar_range(w_start, w_mid), &config).0;
            let dos = decompose_capturable_spread(&ds.slice_bar_range(w_mid, w_end), &config).0;
            let wwin = train_winner_class(&dtr);
            let w_is_l0 = wwin == Some((0, -1));
            if w_is_l0 { l0sell_winner_count += 1; }
            let (on, opnl, lcb) = match wwin {
                Some((lv, dl)) => {
                    let pnls: Vec<f64> = dos.iter().filter(|d| d.level == lv && d.delta == dl)
                        .map(|d| d.actual_pnl).collect();
                    let (nf, _) = neff_autocorr(&pnls, 20);
                    let (_, _, lcb) = mean_se_lcb(&pnls, nf);
                    (pnls.len(), pnls.iter().sum::<f64>(), lcb)
                }
                None => (0, 0.0, 0.0),
            };
            if lcb > 0.0 { lcb_pos_count += 1; }
            wf_rows.push((wwin, w_is_l0, on, opnl, lcb, opnl > 0.0));
        }
        let n_wf = wf_rows.len();
        let lcb_pos_ratio = if n_wf > 0 { 100.0 * lcb_pos_count as f64 / n_wf as f64 } else { 0.0 };

        // PDF §7.3 强判据 χ=1[LCB>θ]（θ=0）：μ>0 不够，须扣除不确定性后仍正。
        // §11 全验收 = LCB>0 ∧ Q4 剔最大5仍正 ∧ bootstrap p<0.05 ∧ 多窗 LCB>0 占比高。
        let oos_robust = lcb_oos > 0.0 && q4_d5 > 0.0 && q4_p < 0.05 && lcb_pos_ratio >= 100.0;

        // ══ 报告 ══
        let cls_str = |c: Option<(u32, i8)>| c.map(|(l, d)| format!("(level={l},δ={d:+})")).unwrap_or_else(|| "None(train无正类)".into());
        let mut rpt = String::new();
        let _ = writeln!(rpt, "# PDF §11 walk-forward 验收：train-only 挑类（除 codex Q2 BIAS-FATAL 选择偏差）");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "**认识论等级**：L2（真实数据单标的，train-only 选类已除选择偏差；仍单标的，非 L3 跨品种）。");
        let _ = writeln!(rpt, "**与被否证 acc_level0sell_oos 的差异**：选类只用 train 段（PDF§6/codex：用挑赢家同一数据验证不能反驳挑赢家）。");
        let _ = writeln!(rpt, "**复算**：`cargo test -p <crate> --release acc_walkforward_trainonly -- --ignored --nocapture`（确定性，含固定种子 bootstrap）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 数据 / 窗口");
        let _ = writeln!(rpt, "- BTC 全量 {n_full} bar；截断窗 [{win_start}→{win_end}]，bars={n}（最后 {max_bars}，OOM 边界=显式有效域）。");
        let _ = writeln!(rpt, "- 主切分 train_frac={train_frac}，split={split}（{split_day}，半开 train=[0,{split}) / holdout=[{split},{n}) 无重叠）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 任务1+2：train-only 挑类 → 锁 holdout 评估（核心：除 Q2）");
        let _ = writeln!(rpt, "- **train 期最强正类 = {}**", cls_str(winner));
        let _ = writeln!(rpt, "- **train 赢家是否 level0卖？{}**（level0卖 train 期 Σpnl={:.4e}/n={}）", if is_level0_sell { "是" } else { "否（⟹ level0卖不是 train-only 赢家！）" }, train_l0sell.1, train_l0sell.0);
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "| holdout 评估 train 选定类 {} | 值 |", cls_str(winner));
        let _ = writeln!(rpt, "|---|---|");
        let _ = writeln!(rpt, "| holdout n (=nraw) | {hd_n} |");
        let _ = writeln!(rpt, "| **holdout Σactual_pnl** | **{hd_pnl:.4e}** |");
        let _ = writeln!(rpt, "| holdout 胜率 | {hd_pos}/{hd_n} ({:.0}%) |", winrate(hd_pos, hd_n));
        let _ = writeln!(rpt, "| holdout Σab_rev | {hd_ab:.4e} |");
        let _ = writeln!(rpt, "| μ_OOS | {mu_oos:.4e} |");
        let _ = writeln!(rpt, "| se（用 neff） | {se_oos:.4e} |");
        let _ = writeln!(rpt, "| **LCB（μ−1.645·se，单侧5%）** | **{lcb_oos:.4e}** |");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "**判定**：holdout μ_OOS {} 0，**LCB {} 0**（PDF§7.3 强判据 χ=1[LCB>0]={}）", if mu_oos > 0.0 { ">" } else { "≤" }, if lcb_oos > 0.0 { ">" } else { "≤" }, lcb_oos > 0.0);
        if !is_level0_sell {
            let _ = writeln!(rpt, "→ **train 赢家变了（level0卖被否证为选择偏差产物）**：train-only 选出的是 {}，不是 level0卖。+7.87e3 是「从含 holdout 全样本挑 level0卖」的选择偏差产物（codex Q2/PDF§6 坐实）。", cls_str(winner));
        } else if !oos_mu_positive {
            let _ = writeln!(rpt, "→ **holdout 翻负（否证）**：level0卖虽是 train-only 赢家，但 holdout Σpnl≤0 ⟹ 无 OOS alpha。+7.87e3 是窗口/选择偏差产物（诚实否证，缩有效域，161）。");
        } else if lcb_oos > 0.0 {
            let _ = writeln!(rpt, "→ **Q2 消除 + μ_OOS 正 + LCB>0**：level0卖是 train-only 赢家、holdout μ 正且扣除不确定性（neff 修正）后仍正 ⟹ 初步通过 §7.3 强判据。但 §11 全验收还需 Q4/bootstrap/多窗全过（见下，综合判定在结果包）。仍 BTC 单标的 L2。");
        } else {
            let _ = writeln!(rpt, "→ **Q2 消除，但 μ_OOS 正而 LCB≤0（未通过 §7.3 强判据）**：level0卖确是 train-only 赢家（选择偏差消除，这一点为正），但 holdout μ_OOS={mu_oos:.2e} 扣除不确定性后 LCB={lcb_oos:.2e}≤0 ⟹ **+7.87e3 不达可交易阈值**。neff/nraw 见下（事件聚集致有效样本远小于 nraw）。这不是「干净 OOS 正」——是「选择偏差消除后，弱正信号被不确定性吞没」。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 任务4：neff vs nraw（PDF§6 条件二，自相关修正）");
        let _ = writeln!(rpt, "| 量 | 值 |");
        let _ = writeln!(rpt, "|---|---|");
        let _ = writeln!(rpt, "| nraw（holdout 赢家类原始信号数） | {nraw} |");
        let _ = writeln!(rpt, "| Σρk（k=1..20，仅正自相关） | {sum_rho:.4} |");
        let _ = writeln!(rpt, "| **neff = nraw/(1+2Σρk)** | **{neff:.1}** |");
        let _ = writeln!(rpt, "| neff/nraw | {:.2} |", if nraw > 0 { neff / nraw as f64 } else { 0.0 });
        let _ = writeln!(rpt, "（neff≪nraw ⟹ 事件高度聚集，有效检验力远低于原始 n；se 已用 neff）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 任务5/Q4：赢家集中度（剔最大赢家 + block bootstrap）");
        let _ = writeln!(rpt, "| 剔除 | 剩余 Σpnl | 仍正? |");
        let _ = writeln!(rpt, "|---|---|---|");
        let _ = writeln!(rpt, "| 不剔（全量） | {q4_full:.4e} | {} |", if q4_full > 0.0 { "是" } else { "否" });
        let _ = writeln!(rpt, "| 剔最大 1 | {q4_d1:.4e} | {} |", if q4_d1 > 0.0 { "是" } else { "否" });
        let _ = writeln!(rpt, "| 剔最大 3 | {q4_d3:.4e} | {} |", if q4_d3 > 0.0 { "是" } else { "否" });
        let _ = writeln!(rpt, "| 剔最大 5 | {q4_d5:.4e} | {} |", if q4_d5 > 0.0 { "是" } else { "否" });
        let _ = writeln!(rpt, "- **block bootstrap p（H0:μ≤0，block_len=20，B=2000，固定种子）= {q4_p:.4}**（p<0.05 ⟹ 正均值稳健远离 0）。");
        if q4_d5 <= 0.0 && q4_full > 0.0 {
            let _ = writeln!(rpt, "→ **Q4 坐实少数大赢家驱动**：剔最大 5 后转负 ⟹ holdout 正总和由极少数大赢家撑起，非稳健 alpha。");
        } else if q4_d5 > 0.0 {
            let _ = writeln!(rpt, "→ **Q4 排除少数大赢家**：剔最大 5 仍正 ⟹ 正收益非单一赢家驱动。");
        } else {
            let _ = writeln!(rpt, "→ holdout 全量已非正，Q4 剔赢家不适用（无正可剔）。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 任务3：多窗滚动 walk-forward（除 Q5 单切分脆弱，K={k_windows}）");
        let _ = writeln!(rpt, "每窗：窗内 train(前{:.0}%) 挑类 → OOS(后{:.0}%) 评估该类（每 train 只用窗内过去）。", wf_train_frac * 100.0, (1.0 - wf_train_frac) * 100.0);
        let _ = writeln!(rpt, "| 窗 | train赢家 | =level0卖? | OOS n | OOS Σpnl | OOS LCB | LCB>0? |");
        let _ = writeln!(rpt, "|---|---|---|---|---|---|---|");
        for (i, (w, isl0, on, opnl, lcb, _)) in wf_rows.iter().enumerate() {
            let _ = writeln!(rpt, "| {} | {} | {} | {on} | {opnl:.4e} | {lcb:.4e} | {} |",
                i + 1, cls_str(*w), if *isl0 { "是" } else { "否" }, if *lcb > 0.0 { "是" } else { "否" });
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "- **各窗 OOS LCB>0 占比 = {lcb_pos_count}/{n_wf} ({lcb_pos_ratio:.0}%)**");
        let _ = writeln!(rpt, "- level0卖为 train 赢家的窗数 = {l0sell_winner_count}/{n_wf}");
        if lcb_pos_ratio >= 100.0 && n_wf > 0 {
            let _ = writeln!(rpt, "→ 全窗 LCB>0 ⟹ Q5 单切分脆弱被排除，OOS 跨窗稳健正（仍单标的）。");
        } else if lcb_pos_count > 0 {
            let _ = writeln!(rpt, "→ 部分窗 LCB>0（{lcb_pos_ratio:.0}%）⟹ 非全窗稳健，单切分脆弱未完全排除（Q5 部分成立）。");
        } else {
            let _ = writeln!(rpt, "→ 无窗 LCB>0 ⟹ 扣除不确定性后无窗稳健正，OOS 不稳健（Q5 坐实）。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 结果包六要素（完整版）");
        let _ = writeln!(rpt, "1. **结论**：train-only 赢家={}（{}level0卖，Q2 选择偏差{}），holdout μ_OOS={mu_oos:.4e}/**LCB={lcb_oos:.4e}**；多窗 LCB>0 占比={lcb_pos_ratio:.0}%（{lcb_pos_count}/{n_wf}）；neff/nraw={:.2}（{neff:.0}/{nraw}）；剔最大5 {}；bootstrap p={q4_p:.3}。**综合判定：{}**",
            cls_str(winner), if is_level0_sell { "=" } else { "≠" },
            if is_level0_sell { "消除" } else { "坐实——非 train 赢家" },
            if nraw > 0 { neff / nraw as f64 } else { 0.0 },
            if q4_d5 > 0.0 { "仍正" } else { "转负" },
            if oos_robust { "通过 §11 全验收（缠论买卖点首个干净稳健 OOS alpha 证据，单标的 L2）" }
            else if is_level0_sell && oos_mu_positive { "Q2 选择偏差消除（level0卖确为 train-only 赢家），但 §11 稳健性验收未过——LCB≤0/少数大赢家驱动/多窗不稳之一以上成立 ⟹ +7.87e3 不达可交易 alpha 阈值（诚实否证，161/formalization-validity-domain）" }
            else { "level0卖 +7.87e3 是选择偏差/窗口产物（否证）" });
        let _ = writeln!(rpt, "2. **定义依据**：actual_pnl=δ(Pτout−Pτin)−Ce（664-Q3 真实成交口径）；train-only 挑类=PDF§11「train 决定规则」；neff=nraw/(1+2Σρk)=PDF§6 条件二；LCB=μ−1.645se=PDF§7.3。");
        let _ = writeln!(rpt, "3. **边界条件**：单标的 BTC L2——结论翻转条件：(a) L3 跨标的若 level0卖不普遍赢则 BTC 是品种特例；(b) train_frac/窗数 K 改变 train 赢家身份则选类不稳；(c) holdout 仍为下跌段则卖优势含方向效应。");
        let _ = writeln!(rpt, "4. **下游推论**：{}", if oos_robust { "level0卖可作信号层 entry 候选（§11 全验收过），但升基座仍需 L3 跨标的。" } else { "level0卖不可单独作 entry——§11 稳健性验收未过（LCB≤0/赢家集中/多窗不稳），下游策略勿基于 +7.87e3 升基座。Q2 消除只证「不是挑赢家产物」，不证「是可交易 alpha」——二者独立。" });
        let _ = writeln!(rpt, "5. **谱系引用**：663 econpositive；664-Q3 真实成交口径；codex Q2 BIAS-FATAL（codex-oos-level0sell-audit-20260630.md）；PDF§6/§11（overfit-consult-20260630.txt）；161（务实=留缺口）；formalization-validity-domain（L2 有效域<定义域）。");
        let _ = writeln!(rpt, "6. **影响声明**：新增 acc_walkforward_trainonly 测试 + train_winner_class/neff_autocorr/mean_se_lcb/drop_top_winners/block_bootstrap_pvalue helper（均纯函数，L1 自检 walkforward_helpers_l1）；复用 slice_bar_range/decompose_capturable_spread/class_actual_pnl；不改生产代码/TradeRecord/Order。被否证的 acc_level0sell_oos 保留（谱系：选择偏差的发生史）。");

        eprint!("{rpt}");
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().expect("rust/ 父目录 = 项目根")
            .join(".chanlun/review-results/econpositive-walkforward-20260630.md");
        std::fs::write(&out, &rpt).unwrap_or_else(|e| panic!("写报告 {out:?} 失败：{e}"));
        eprintln!("\n报告已落盘：{out:?}");

        // 真封：切片无重叠无遗漏 + holdout 聚合一致 + neff≤nraw。
        assert!(ds_train.bars.len() + ds_hold.bars.len() == n, "train+holdout bar 数 ≠ 全窗（半开应无重叠无遗漏）");
        let recomputed: f64 = hd_pnls.iter().sum();
        assert!((hd_pnl - recomputed).abs() < 1e-6, "holdout 赢家类 Σpnl 聚合不一致");
        assert!(neff <= nraw as f64 + 1e-9, "neff 应 ≤ nraw");
    }

    // ── PDF §11 walk-forward 验收 helper（纯函数，L1 自检见 mod 末 #[test]）──

    /// train 段 per-class 选最强正类（PDF §11「train 决定规则」）：扫所有出现的 (level,δ)，
    /// 取 Σactual_pnl 最大者；若全非正返 None（train 期无可交易候选）。
    /// **关键反偏差**：选类只用 train 段 decomps，holdout 信息不进入选择环节（消 codex Q2 BIAS-FATAL）。
    fn train_winner_class(decomps_train: &[SignalDecomp]) -> Option<(u32, i8)> {
        use std::collections::BTreeMap;
        let mut sums: BTreeMap<(u32, i8), f64> = BTreeMap::new();
        for d in decomps_train {
            *sums.entry((d.level, d.delta)).or_default() += d.actual_pnl;
        }
        sums.into_iter()
            .filter(|(_, pnl)| *pnl > 0.0)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(cls, _)| cls)
    }

    /// 有效样本数 neff = nraw/(1+2Σρk)（PDF §6 条件二）。ρk=逐笔 PnL 序列样本自相关，
    /// k=1..=lag_max，仅累加 ρk>0（正自相关才膨胀方差；负自相关不缩 neff 以保守）。
    /// 退化：n<2 或方差≈0 ⟹ neff=nraw（无相关信息）。返回 (neff, sum_rho_pos)。
    fn neff_autocorr(pnls: &[f64], lag_max: usize) -> (f64, f64) {
        let n = pnls.len();
        if n < 2 { return (n as f64, 0.0); }
        let mean = pnls.iter().sum::<f64>() / n as f64;
        let var = pnls.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        if var <= 1e-12 { return (n as f64, 0.0); }
        let mut sum_rho = 0.0;
        for k in 1..=lag_max.min(n - 1) {
            let cov: f64 = (0..n - k).map(|i| (pnls[i] - mean) * (pnls[i + k] - mean)).sum::<f64>() / n as f64;
            let rho = cov / var;
            if rho > 0.0 { sum_rho += rho; }
        }
        let neff = n as f64 / (1.0 + 2.0 * sum_rho);
        (neff.max(1.0).min(n as f64), sum_rho)
    }

    /// 单侧 5% LCB = μ − 1.645·se，se=σ/√neff（PDF §7.3，neff 而非 nraw）。返回 (mu, se, lcb)。
    fn mean_se_lcb(pnls: &[f64], neff: f64) -> (f64, f64, f64) {
        let n = pnls.len();
        if n == 0 { return (0.0, 0.0, 0.0); }
        let mu = pnls.iter().sum::<f64>() / n as f64;
        let var = pnls.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / n as f64;
        let se = if neff > 1.0 { (var / neff).sqrt() } else { f64::INFINITY };
        (mu, se, mu - 1.645 * se)
    }

    /// Q4 剔最大赢家：降序剔除 top-k 后剩余 Σ（>0 ⟹ 非少数大赢家驱动）。返回 (sum_full, sum_drop1, sum_drop3, sum_drop5)。
    fn drop_top_winners(pnls: &[f64]) -> (f64, f64, f64, f64) {
        let mut v = pnls.to_vec();
        v.sort_by(|a, b| b.partial_cmp(a).unwrap()); // 降序
        let full: f64 = v.iter().sum();
        let drop = |k: usize| -> f64 { v.iter().skip(k.min(v.len())).sum() };
        (full, drop(1), drop(3), drop(5))
    }

    /// Block bootstrap 右尾 p-value（H0:μ≤0，移动块重采样保留逐笔 PnL 自相关）：
    /// 对原始序列做循环移动块重采样得 B 个 boot_mean，p = (#{boot_mean ≤ 0}+1)/(B+1)
    /// = 重采样分布落在 0 及以下的占比 ⟹ 正均值越稳健远离 0 则 p 越小（PDF §11 block bootstrap）。
    /// block_len 保块内时间相关（缠论同趋势段多信号相关，PDF §6）。确定性：固定 LCG（bit-exact 可复算）。
    fn block_bootstrap_pvalue(pnls: &[f64], block_len: usize, b_iters: usize) -> f64 {
        let n = pnls.len();
        if n < 2 { return 1.0; }
        let blk = block_len.max(1).min(n);
        let mut seed: u64 = 0x9E3779B97F4A7C15; // 固定种子（确定性，无外部 rng）
        let next = |seed: &mut u64| -> usize {
            *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((*seed >> 33) as usize) % n
        };
        let mut le_zero = 0usize;
        for _ in 0..b_iters {
            let (mut acc, mut cnt) = (0.0f64, 0usize);
            while cnt < n {
                let start = next(&mut seed);
                for j in 0..blk {
                    if cnt >= n { break; }
                    acc += pnls[(start + j) % n]; // 循环移动块（原始序列，未居中）
                    cnt += 1;
                }
            }
            if acc / n as f64 <= 0.0 { le_zero += 1; }
        }
        (le_zero as f64 + 1.0) / (b_iters as f64 + 1.0)
    }

    /// neff/LCB/bootstrap helper L1 自检（合成数据，验证算术，零信息增量但保非平凡逻辑不破）。
    #[test]
    fn walkforward_helpers_l1() {
        // train_winner：(0,-1) Σ=+5 最强，(1,1) Σ=−2 被滤。
        let synth = |level: u32, delta: i8, pnl: f64| SignalDecomp {
            entry_bar: 0, exit_bar: 1, level, delta, a_b: 0.0, x_in: 0.0, y_out: 0.0,
            eta_in: 0.0, eta_out: 0.0, actual_spread: 0.0, ce_unit: 0.0, captured: 0.0, actual_pnl: pnl,
        };
        let ds = vec![synth(0, -1, 3.0), synth(0, -1, 2.0), synth(1, 1, -2.0)];
        assert_eq!(train_winner_class(&ds), Some((0, -1)), "train 应选 Σpnl 最大正类 (0,-1)");
        let all_neg = vec![synth(0, -1, -1.0)];
        assert_eq!(train_winner_class(&all_neg), None, "全非正应返 None");

        // neff 边界：始终 ∈[1, nraw]。块状正相关 ⟹ neff<nraw（事件聚集缩有效样本，PDF§6）。
        let pos_corr = vec![1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0]; // 块状正相关 lag1>0
        let (neff_pc, sr_pc) = neff_autocorr(&pos_corr, 4);
        assert!(neff_pc < 8.0 && neff_pc >= 1.0 && sr_pc > 0.0,
            "正自相关应缩 neff∈[1,nraw)，实得 neff={neff_pc} sr={sr_pc}");
        // 常量序列方差≈0 ⟹ 退化 neff=nraw（无相关信息可缩）。
        let (neff_const, _) = neff_autocorr(&[2.0; 6], 4);
        assert!((neff_const - 6.0).abs() < 1e-9, "零方差退化 neff=nraw，实得 {neff_const}");

        // LCB：μ>0 但 se 大 ⟹ LCB 可能<0（压制噪声赢家）。
        let (mu, se, lcb) = mean_se_lcb(&[2.0, -1.0, 3.0, -2.0], 4.0);
        assert!((mu - 0.5).abs() < 1e-9 && se > 0.0 && lcb < mu, "LCB=μ−1.645se<μ，实得 mu={mu} lcb={lcb}");

        // drop_top：[10,1,1,1] 剔最大后 Σ=3>0（非单一赢家）；[10,-1,-1,-1] 剔后 Σ=−3<0（单一赢家驱动）。
        let (f1, d1, _, _) = drop_top_winners(&[10.0, 1.0, 1.0, 1.0]);
        assert!((f1 - 13.0).abs() < 1e-9 && (d1 - 3.0).abs() < 1e-9, "剔最大后剩余");
        let (_, d1b, _, _) = drop_top_winners(&[10.0, -1.0, -1.0, -1.0]);
        assert!(d1b < 0.0, "单一大赢家驱动：剔后转负");

        // bootstrap：强正信号 p 小；纯噪声 p 接近 0.5（确定性，固定种子）。
        let strong_pos = vec![5.0; 20];
        let p_pos = block_bootstrap_pvalue(&strong_pos, 3, 500);
        assert!(p_pos <= 0.05, "强正信号 block bootstrap p 应小，实得 {p_pos}");
    }
}
