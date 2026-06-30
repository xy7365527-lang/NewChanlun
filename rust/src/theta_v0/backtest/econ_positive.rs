//! 经济正条件逐信号分解（《经济正条件.pdf》第5节可捕获价差判据 L2 诊断）。
//!
//! 证明链 L0 见 `.chanlun/proofs/economic-positive-condition-chain.md`。本模块是其
//! **唯一可否证环节**（可捕获价差判据前件）的 L2 实装：逐信号确定性分解
//!
//!     captured = Ab − ηin − ηout − Ce/qe        (PDF p4-5 §5)
//!
//! 其中（向上笔 εb=+1，向下笔 εb=−1）：
//! - `Ab   = εb(Pρb − Pλb)`         结构端点价差（触发笔两端点 close，理想入出场）。
//! - `ηin  = max(0, εb(Pτin − Pλb))` 入场滞后损耗（确认 bar 成交价相对理想起点的不利滑移，≥0）。
//! - `ηout = max(0, εb(Pρb − Pτout))` 出场损耗（实际出场价相对理想终点的损耗，≥0）。
//! - `Ce/qe = Pτ·fee_rate·2`          单位双边成本（与 `mu_estimator::marginal_return` 同口径）。
//!
//! **可否证（L2）**：若 Σ(ηin+ηout+Ce) ≥ ΣAb（执行损耗吃光结构价差），则该信号集无可捕获 alpha，
//! 且分解定位「钱去哪了」（结构无价差 / 执行滞后 / 成本三者分离）。与 663 咬合：逐信号确定性分解，
//! 非统计功效检验。
//!
//! **端点价缺口的严格解（不改 TradeRecord/Order）**：理想端点价 Pλb/Pρb 不在 fill 配对链
//! （`TradeRecord` 只有 entry/exit bar），但**信号收集路径**（同 `build_walk_forward_mu`）在确认点
//! 持有 tower——按触发笔右端点 `end_index == source_index`（coverage.rs:381 判准）从 tower 定位
//! 触发 `LeveledMove`，取其 `start_index`(λb)/`end_index`(ρb) 端点 bar 的 close 作 Pλb/Pρb。
//! 故走 μ 路径无需透传 Order 端点价。

use super::data::Dataset;
use super::incremental::IncrementalClassifier;
use super::super::classifier::recursive_tower::LeveledMove;
use super::super::config::ThetaConfig;
use super::super::strategy::interp::assemble_gamma_with_tower;
use super::super::strategy::voice::VoiceSide;
use super::super::types::{Bar, BspBits};
use std::rc::Rc;

/// 单信号的可捕获价差分解（PDF §5 五项 + captured = Ab−ηin−ηout−Ce/qe）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalDecomp {
    /// 入场确认 bar（τin，F_i 可测）。
    pub entry_bar: usize,
    /// 出场 bar（τout，下一反向确认信号 / 末 bar censored）。
    pub exit_bar: usize,
    /// 方向 δ：+1 多 / −1 空。
    pub delta: i8,
    /// Ab = εb(Pρb − Pλb)：结构端点价差（≥0，端点方向同义反复）。
    pub a_b: f64,
    /// ηin = max(0, εb(Pτin − Pλb))：入场滞后损耗。
    pub eta_in: f64,
    /// ηout = max(0, εb(Pρb − Pτout))：出场损耗。
    pub eta_out: f64,
    /// Ce/qe：单位双边成本。
    pub ce_unit: f64,
    /// captured = Ab − ηin − ηout − Ce/qe（>0 ⟺ 路径级正收益，PDF §5）。
    pub captured: f64,
}

/// 聚合诊断「钱去哪了」（确定性分解，Σ 精确等于 Σ trade gross，非概率推断）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpreadAttribution {
    pub n_signals: usize,
    /// Σ Ab：结构给的理想总价差。
    pub sum_a_b: f64,
    /// Σ ηin：入场滞后吃掉。
    pub sum_eta_in: f64,
    /// Σ ηout：出场滞后吃掉。
    pub sum_eta_out: f64,
    /// Σ Ce/qe：成本吃掉。
    pub sum_ce: f64,
    /// Σ captured：剩下的可捕获 alpha。
    pub sum_captured: f64,
    /// 可捕获信号数（captured>0）——逐信号路径级正占比的分子。
    pub n_captured_positive: usize,
}

impl SpreadAttribution {
    /// L2 否证判据：执行损耗（含成本）是否吃光结构价差。
    ///
    /// `true` ⟺ Σ(ηin+ηout+Ce) ≥ ΣAb ⟺ Σcaptured ≤ 0 ⟹ 该信号集无可捕获 alpha（PDF §5 前件失败）。
    pub fn spread_eaten(&self) -> bool {
        self.sum_captured <= 0.0
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

/// 在 tower 第 `lvl` 级中按右端点 `end_index == source_index` 定位触发走势元素，返回其起点 bar
/// `start_index`(λb)。判准来自 coverage.rs:381（买卖点触发元素 `e.rho == source_index`，禁 lambda）。
///
/// `None` ⟹ 该 source_index 在 lvl 级无匹配 move（信号触发点未入该级塔，跳过该信号——诚实跳过，
/// 不用 entry_bar 兜底，否则 λb=τin ⟹ ηin≡0 退化为同义反复）。
fn locate_lambda_bar(tower: &[Rc<Vec<LeveledMove>>], lvl: usize, source_index: usize) -> Option<usize> {
    tower
        .get(lvl)?
        .iter()
        .find(|m| m.end_index == source_index)
        .map(|m| m.start_index)
}

/// 逐信号可捕获价差分解（L2）。复用 `build_walk_forward_mu` 的信号收集 + 退出配对模板，
/// 在确认点用 tower 定位触发笔端点，算 PDF §5 五项分解。
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

    // ── 信号收集（同 build_walk_forward_mu）：逐 bar 因果分类，收新确认买卖点 (entry_bar, dir, λb_bar)。 ──
    let mut classifier_incr = IncrementalClassifier::new(bars, config);
    let mut seen: std::collections::HashSet<(usize, usize, u8)> = std::collections::HashSet::new();
    // 每条信号：(entry_bar=确认 bar τin, dir, lambda_bar=触发笔起点 bar λb, rho_bar=触发笔终点 bar ρb)。
    // 触发笔终点 ρb 的 bar 序 = p.source_index（coverage.rs:381 判准）。
    let mut signals: Vec<(usize, VoiceSide, usize, usize)> = Vec::new();

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
                // 触发笔起点 bar（λb）：tower 第 lvl 级 end_index==source_index 的 move 的 start_index。
                let lambda_bar = match locate_lambda_bar(&tower_i, lvl, p.source_index) {
                    Some(lb) => lb,
                    None => continue, // 诚实跳过：无匹配 move ⟹ 算不出 Ab，不兜底
                };
                let rho_bar = p.source_index; // 触发笔终点 = source_index（coverage.rs:381）
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
                    signals.push((i, c.dir, lambda_bar, rho_bar));
                }
            }
        }
    }

    // ── 退出配对（事前固定规则：持有到下一反向新确认信号确认 bar，或末 bar censored；同 build_walk_forward_mu）。 ──
    let mut decomps: Vec<SignalDecomp> = Vec::new();
    let mut agg = SpreadAttribution::default();

    for (idx, &(entry_bar, dir, lambda_bar, rho_bar)) in signals.iter().enumerate() {
        let delta: i8 = match dir {
            VoiceSide::Long => 1,
            VoiceSide::Short => -1,
            VoiceSide::Flat => continue,
        };
        let opp = if delta == 1 { VoiceSide::Short } else { VoiceSide::Long };
        let exit_bar = signals[idx + 1..]
            .iter()
            .find(|(eb, d, _, _)| *eb > entry_bar && *d == opp)
            .map(|(eb, _, _, _)| *eb)
            .unwrap_or_else(|| {
                (0..n)
                    .rev()
                    .find(|&j| j > entry_bar && !bars[j].untradable && bars[j].close > 0)
                    .unwrap_or(entry_bar)
            });
        if exit_bar <= entry_bar {
            continue;
        }
        // 端点价（close 口径，与回测成交价同口径，避免端点价定义混入）：
        // Pλb=触发笔起点 close（理想入场），Pρb=触发笔终点 close（理想出场，rho_bar=source_index），
        // Pτin=确认 bar close（实际入场，含确认滞后），Pτout=配对退出 bar close（实际出场）。
        let p_lambda = px_at(bars, lambda_bar, tick);
        let p_rho = px_at(bars, rho_bar, tick);
        let p_tau_in = px_at(bars, entry_bar, tick);
        let p_tau_out = px_at(bars, exit_bar, tick);
        if [p_lambda, p_rho, p_tau_in, p_tau_out].iter().any(|&x| x <= 0.0) {
            continue;
        }
        let eps = delta as f64;
        let a_b = eps * (p_rho - p_lambda);
        let eta_in = (eps * (p_tau_in - p_lambda)).max(0.0);
        let eta_out = (eps * (p_rho - p_tau_out)).max(0.0);
        // 单位双边成本：与 marginal_return 口径一致（fee 在 entry/exit 各扣一次）。
        let ce_unit = (p_tau_in + p_tau_out) * fee_rate;
        let captured = a_b - eta_in - eta_out - ce_unit;

        decomps.push(SignalDecomp { entry_bar, exit_bar, delta, a_b, eta_in, eta_out, ce_unit, captured });
        agg.n_signals += 1;
        agg.sum_a_b += a_b;
        agg.sum_eta_in += eta_in;
        agg.sum_eta_out += eta_out;
        agg.sum_ce += ce_unit;
        agg.sum_captured += captured;
        if captured > 0.0 {
            agg.n_captured_positive += 1;
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

    /// PDF §4 反例的分解自检（合成，L1 验证分解算术正确）：
    /// 向上笔理想 Pλ=10, Pρ=12（Ab=2）；确认滞后 Pτin=11.8, Pτout=11.1。
    /// ηin=max(0,11.8−10)=1.8, ηout=max(0,12−11.1)=0.9, captured=2−1.8−0.9−Ce<0（执行吃光）。
    #[test]
    fn pdf_counterexample_decomp() {
        let eps = 1.0_f64; // 向上笔
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

    /// spread_eaten 判据自检：Σcaptured≤0 ⟺ 损耗吃光。
    #[test]
    fn spread_eaten_iff_captured_nonpositive() {
        let mut agg = SpreadAttribution::default();
        agg.sum_captured = -0.1;
        assert!(agg.spread_eaten());
        agg.sum_captured = 0.1;
        assert!(!agg.spread_eaten());
    }
}
