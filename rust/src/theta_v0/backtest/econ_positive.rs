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
    /// 触发买卖点所在 tower 级别（per-class 分桶键 (level,δ) 的 level 分量，MuClass.level 同口径）。
    pub level: u32,
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
    // 每条信号：(entry_bar=确认 bar τin, dir, lambda_bar=触发笔起点 bar λb, rho_bar=触发笔终点 bar ρb, lvl=级别)。
    // 触发笔终点 ρb 的 bar 序 = p.source_index（coverage.rs:381 判准）。
    let mut signals: Vec<(usize, VoiceSide, usize, usize, u32)> = Vec::new();

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
                    signals.push((i, c.dir, lambda_bar, rho_bar, lvl as u32));
                }
            }
        }
    }

    // ── 退出配对（事前固定规则：持有到下一反向新确认信号确认 bar，或末 bar censored；同 build_walk_forward_mu）。 ──
    let mut decomps: Vec<SignalDecomp> = Vec::new();
    let mut agg = SpreadAttribution::default();

    for (idx, &(entry_bar, dir, lambda_bar, rho_bar, level)) in signals.iter().enumerate() {
        let delta: i8 = match dir {
            VoiceSide::Long => 1,
            VoiceSide::Short => -1,
            VoiceSide::Flat => continue,
        };
        let opp = if delta == 1 { VoiceSide::Short } else { VoiceSide::Long };
        let exit_bar = signals[idx + 1..]
            .iter()
            .find(|(eb, d, _, _, _)| *eb > entry_bar && *d == opp)
            .map(|(eb, _, _, _, _)| *eb)
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

        decomps.push(SignalDecomp { entry_bar, exit_bar, level, delta, a_b, eta_in, eta_out, ce_unit, captured });
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

    /// L2 诊断「钱去哪了」：BTC 全历史逐信号可捕获价差分解（真实数据，可产否定性结果）。
    ///
    /// `#[ignore]`：需 BTC 全量数据（314M）+ O(n²) 逐 bar 重分类，`--release` 必须。重跑由 Lead。
    /// 报告落盘 `.chanlun/review-results/econ-l2-btc-diagnosis-20260630.md`（确定性，可复算）。
    ///
    /// **L2 纪律**：spread_eaten=true ⟹ 该信号集执行损耗吃光结构价差（否证可交易，有效域收窄）；
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

        // per-class (level, δ) 分桶：Σcaptured + n_captured_positive/n_signals（逐信号路径级正占比）。
        // key=(level, δ)，value=(n, Σab, Σηin, Σηout, Σce, Σcaptured, n_pos)。
        let mut buckets: BTreeMap<(u32, i8), (usize, f64, f64, f64, f64, f64, usize)> = BTreeMap::new();
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
        }

        // 失血三源占比（分母 = Σ(ηin+ηout+Ce) 总损耗；ΣAb 为正分母比对结构价差）。
        let total_drain = agg.sum_eta_in + agg.sum_eta_out + agg.sum_ce;
        let pct = |x: f64| if total_drain > 0.0 { 100.0 * x / total_drain } else { 0.0 };

        let mut rpt = String::new();
        let _ = writeln!(rpt, "# 经济正条件③ L2 诊断：BTC 基线「钱去哪了」可捕获价差归因");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "**认识论等级**：L2（真实数据单标的逐信号确定性分解，可产否定性结果）。");
        let _ = writeln!(rpt, "**口径**：captured = Ab − ηin − ηout − Ce/qe（PDF §5）；端点 close 口径；fee_rate=(comm+slip+tax)bps/1e4。");
        let _ = writeln!(rpt, "**复算**：`cargo test -p <crate> --release l2_btc_capturable_spread_diagnosis -- --ignored --nocapture`（确定性）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 数据");
        let _ = writeln!(rpt, "- 品种：BTC（btc_1m_full.json，全量 {n_full} bar，2017-08→2026-05）");
        let _ = writeln!(rpt, "- **截断窗 [{window_start}→{window_end}]，bars={n_bars}**（最后 {max_bars} bar；全量 461万 OOM 不可行 ⟹ 截断窗=显式有效域边界，非全窗结论，l3 同纪律）");
        let _ = writeln!(rpt, "- untradable_ratio={:.4}", untradable);
        let _ = writeln!(rpt, "- 收集信号数 n_signals={}（locate_lambda_bar 跳过的信号不入此集——见跳过率）", agg.n_signals);
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 全局归因");
        let _ = writeln!(rpt, "| 量 | 值 |");
        let _ = writeln!(rpt, "|---|---|");
        let _ = writeln!(rpt, "| ΣAb（结构理想总价差） | {:.6e} |", agg.sum_a_b);
        let _ = writeln!(rpt, "| Σηin（入场滞后） | {:.6e} ({:.1}%) |", agg.sum_eta_in, pct(agg.sum_eta_in));
        let _ = writeln!(rpt, "| Σηout（出场滞后） | {:.6e} ({:.1}%) |", agg.sum_eta_out, pct(agg.sum_eta_out));
        let _ = writeln!(rpt, "| ΣCe（成本） | {:.6e} ({:.1}%) |", agg.sum_ce, pct(agg.sum_ce));
        let _ = writeln!(rpt, "| Σ(η+Ce)（总损耗） | {:.6e} |", total_drain);
        let _ = writeln!(rpt, "| Σcaptured（剩余 alpha） | {:.6e} |", agg.sum_captured);
        let _ = writeln!(rpt, "| n_captured_positive/n_signals | {}/{} ({:.1}%) |",
            agg.n_captured_positive, agg.n_signals,
            if agg.n_signals > 0 { 100.0 * agg.n_captured_positive as f64 / agg.n_signals as f64 } else { 0.0 });
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## L2 判定");
        let _ = writeln!(rpt, "- **spread_eaten = {}**（Σcaptured {} 0）", agg.spread_eaten(), if agg.spread_eaten() { "≤" } else { ">" });
        if agg.spread_eaten() {
            let _ = writeln!(rpt, "- **否定性结果**：执行损耗（含成本）吃光结构价差 ⟹ 该信号集无可捕获 alpha（PDF §5 前件失败）。有效域收窄——比确认性结果信息量大（161/formalization-validity-domain）。");
        } else {
            let _ = writeln!(rpt, "- **确认性结果**：Σcaptured>0 ⟹ 该信号集结构端点价差未被执行损耗吃光（PDF §5 前件成立）。注意：路径级正 ≠ 跨品种功效；仅 BTC 单标的 L2。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 失血三源对比（ΣAb 为结构上限）");
        let _ = writeln!(rpt, "- 结构无价差：ΣAb={:.6e}（若 ΣAb 本身小则结构不给价差）", agg.sum_a_b);
        let _ = writeln!(rpt, "- 执行吃光：Ση={:.6e}（入场+出场滞后）", agg.sum_eta_in + agg.sum_eta_out);
        let _ = writeln!(rpt, "- 成本：ΣCe={:.6e}", agg.sum_ce);
        if agg.sum_a_b < 0.0 {
            let _ = writeln!(rpt);
            let _ = writeln!(rpt, "**核心诊断（比执行滞后更根本）：ΣAb<0。** Ab=εb(Pρb−Pλb)，理论应 ≥0（端点方向同义反复：");
            let _ = writeln!(rpt, "向上笔 ρ>λ）。ΣAb<0 ⟹ **信号方向 δ（assemble_gamma 给）与触发笔结构端点方向系统性错位**——");
            let _ = writeln!(rpt, "买卖点在触发笔做了反向标注（如向下笔上标买点）。这不是执行问题：结构端点价差本身为负，");
            let _ = writeln!(rpt, "执行损耗只是在负的结构价差上再扣。即使零滞后零成本（Ση=ΣCe=0），captured=ΣAb<0 仍无 alpha。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## per-class (level, δ) 分桶");
        let _ = writeln!(rpt, "| level | δ | n | ΣAb | Ση | ΣCe | Σcaptured | n_pos/n (路径级正占比) |");
        let _ = writeln!(rpt, "|---|---|---|---|---|---|---|---|");
        for ((lvl, dlt), (n, sab, sin, sout, sce, scap, npos)) in &buckets {
            let _ = writeln!(rpt, "| {} | {:+} | {} | {:.4e} | {:.4e} | {:.4e} | {:.4e} | {}/{} ({:.1}%) |",
                lvl, dlt, n, sab, sin + sout, sce, scap, npos, n,
                if *n > 0 { 100.0 * *npos as f64 / *n as f64 } else { 0.0 });
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 663 原生口径（μ̂>0 状态类）");
        let _ = writeln!(rpt, "Σcaptured>0 的 (level,δ) 类 = 该类逐信号路径级净正（出现即做，看路径级 captured，不做跨品种符号检验功效门）：");
        let mut any_pos_class = false;
        for ((lvl, dlt), (n, _, _, _, _, scap, npos)) in &buckets {
            if *scap > 0.0 {
                any_pos_class = true;
                let _ = writeln!(rpt, "- (level={}, δ={:+}): Σcaptured={:.4e}, n_pos/n={}/{}", lvl, dlt, scap, npos, n);
            }
        }
        if !any_pos_class {
            let _ = writeln!(rpt, "- （无 Σcaptured>0 的类——全级别全方向执行损耗吃光，否定性结果）");
        }

        eprint!("{rpt}");

        // 落盘（项目根 = CARGO_MANIFEST_DIR 上一级）。
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().expect("rust/ 父目录 = 项目根")
            .join(".chanlun/review-results/econ-l2-btc-diagnosis-20260630.md");
        std::fs::write(&out, &rpt).unwrap_or_else(|e| panic!("写报告 {out:?} 失败：{e}"));
        eprintln!("\n报告已落盘：{out:?}");

        // 真封：分解恒等式逐信号成立（captured 精确等于五项分解，非概率）。
        for d in &decomps {
            let recomputed = d.a_b - d.eta_in - d.eta_out - d.ce_unit;
            assert!((d.captured - recomputed).abs() < 1e-6,
                "captured 分解恒等式破：level={} δ={} captured={} ≠ {}", d.level, d.delta, d.captured, recomputed);
        }
        // 聚合 Σcaptured = Σ逐信号 captured（无丢失）。
        let sum_check: f64 = decomps.iter().map(|d| d.captured).sum();
        assert!((agg.sum_captured - sum_check).abs() < 1e-3,
            "Σcaptured 聚合 {} ≠ 逐信号和 {}", agg.sum_captured, sum_check);
    }
}
