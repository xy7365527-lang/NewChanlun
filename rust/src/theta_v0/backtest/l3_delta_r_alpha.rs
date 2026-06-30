//! **Phase-3 L2/L3 ΔR 净额增量 alpha 否证**（acc-delta-r-alpha，task #42）。
//!
//! 检验 χ_t(γ)=1[μ(z)>θ] 选择器（[`super::selector`]）是否产生**正净额增量 alpha**——
//! 对齐买卖点alpha2.pdf Doc2 §10-§14 + Doc3 §11-§16：
//!
//! ```text
//!   ΔN_t = N^bsp_t − N^0_t           （χ 过滤净头寸 − χ≡1 全覆盖净头寸）
//!   ΔR_{t+1} = ΔN_t·(P_{t+1}−P_t) − ΔC_t   （净额增量收益，§10）
//!   alpha 命题：E[ΔR]>0 或 Sharpe(ΔR)>0
//! ```
//!
//! ## 鞅不可能定理（§11/§16，本检验的零假设理论锚）
//!
//! 若 `E[ΔP|F_t]=0 ∧ ΔN_t∈F_t`（因果），则 `E[ΔR]=−E[ΔC]≤0`——**无预测性则无 alpha**。
//! χ 过滤本身不创造预测性；唯一可能的 alpha 来源 = walk-forward μ(z) 表是否捕获了某 z 类
//! **持续的**正期望（train 期正且 test 期延续）。若 μ 表本质是噪声（train 期偶然正、test 期
//! 不延续），则 ΔR 符号跨品种随机、均值≈0（鞅定理预言的否证）。
//!
//! ## ΔR 序列的因果干净实装（codex 方法论审核 Q2，2026-06-30）
//!
//! **主口径是 ΔR 序列自身，不是两条 equity 的 Sharpe 差**（codex：`Sharpe(A)−Sharpe(B) ≠
//! Sharpe(A−B)`，Sharpe 非线性；过滤可能只降波动/降成本改善 ΔSharpe 而 ΔR 无正期望）。
//!
//! 关键等价（消除 runner 改动，ponytail）：归一化权益 `E_t=(cash_t+units_t·P_t)/nav0`，逐 bar
//! 收益 `r_t = E_t−E_{t−1} = (units_{t−1}·(P_t−P_{t−1}) − cost_t)/nav0`。两条独立回测（χ≡1
//! baseline vs χ 过滤）的**逐 bar 收益配对差**：
//! ```text
//!   r^χ_t − r^0_t = ((N^bsp_{t−1}−N^0_{t−1})·ΔP_t − (C^bsp_t−C^0_t)) / nav0
//!                 = ΔR_t / nav0          （§10 ΔR 序列，归一化口径）
//! ```
//! 成本差 ΔC 已自动含入（成本扣进 equity）。故 **ΔR 序列 = nav0·(daily_returns_χ −
//! daily_returns_baseline)** 逐 bar 配对差——直接用 `RunResult.daily_returns`，无需导出净头寸序列。
//!
//! ## walk-forward μ 表（codex Q1：train 内未来打标签合法，test 决策不偷看 test 未来）
//!
//! OOS 窗内 split：前半 = train（估 μ），后半 = test（χ 过滤 + baseline 对比）。μ 估计在 train
//! 窗逐 bar 因果分类（[`super::incremental::IncrementalClassifier`]），对**新确认**买卖点取 z
//! （[`super::selector::z_of_candidate`]），用确认 bar 后**下一个反向新确认信号**（或 train 末
//! **censored 截断**——codex Q1 边界泄漏 guard）兑现 X_γ（[`super::mu_estimator::marginal_return`]）。
//! θ 常数（不从样本 μ 分布选，无 in-sample 泄漏）。test 窗的 χ 决策只读 frozen μ 表。
//!
//! ## 判定（codex Q4：三层，否定"1/8 ΔSharpe≠0"弱判据）
//!
//! - **L1 机制**：ΔN 非全等（χ 真改变交易集）+ μ walk-forward 无泄漏（结构保证）。
//! - **L2 单品种 alpha**：`mean(ΔR)>0 ∧ Sharpe(ΔR)>0` 且 ΔR block bootstrap 单边 p≤0.05。
//! - **L3 系统性 alpha**：8 品种 ΔR 均值跨品种**符号检验**（n_pos/8，二项单边 p）。codex：
//!   8 品种符号检验需 ≥7/8 正才到单边 p<0.05；否则只是"局部探索性命中"非系统性。
//!
//! **诚实强制（formalization-validity-domain + alpha2 §15）**：否定性结果（μ≤0/ΔR≤0/不超随机）
//! 比确认更有价值（鞅定理预言无预测性则无 alpha）。如实报 ΔR≤0，不调参凑正。覆盖≠盈利
//! （645/653/654）：χ 改变交易集（L1）≠ χ 提升 alpha（L2/L3 本检验否证）。
//!
//! 跑法：`cargo test --release --lib theta_v0::backtest::l3_delta_r_alpha -- --ignored --nocapture`

use super::data::{self, Dataset};
use super::incremental::IncrementalClassifier;
use super::mu_estimator::{marginal_return, MuClass, MuEstimator, MuObservation};
use super::prereg_windows::PREREG_WINDOWS;
use super::runner::run_theta_v0_pi_chi;
use super::selector::z_of_candidate;
use super::super::config::ThetaConfig;
use super::super::strategy::interp::assemble_gamma_with_tower;
use super::super::strategy::voice::VoiceSide;
use super::super::types::BspBits;

/// 预注册随机种子（§4，与 metrics 同值，bit-exact 可复现）。
const PREREG_SEED: u64 = 20260625;

/// SplitMix64 确定性 PRNG（本地副本，无依赖纯算术；与 metrics 私有版同口径，技术性工具非领域概念）。
struct SplitMix64 {
    state: u64,
}
impl SplitMix64 {
    fn new(seed: u64) -> Self {
        SplitMix64 { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
}

/// 可行子集窗 bar 数（与 [`super::l3_pi_falsify`] 同口径，O(n²) 全窗不可行）。
/// 32K 一窗 ⟹ train+test 各 16K（OOS 内 split）。
const MAX_BARS: usize = 32_000;
/// 统计功效门槛（协议 §5.5：real 样本 <30 inconclusive）。ΔR 序列以 bar 计，门槛指 test 有效 bar。
const MIN_TEST_BARS: usize = 252; // ≥1 交易年（日聚合等价）的 bar 量级下限（保守）

/// 买卖点身份判别 u8（与 runner `bsp_bits_disc` 同口径，seen-set diff 键）。
fn bsp_disc(b: &BspBits) -> u8 {
    (b.buy1 as u8)
        | (b.buy2 as u8) << 1
        | (b.buy3 as u8) << 2
        | (b.sell1 as u8) << 3
        | (b.sell2 as u8) << 4
        | (b.sell3 as u8) << 5
}

/// 在 train 窗上构造 **walk-forward μ(z) 表**（codex Q1：train 内未来兑现合法，无 test 泄漏）。
///
/// 逐 bar 因果分类（`IncrementalClassifier`，只用 ≤i 数据）→ 对本 bar **新确认**买卖点（append-only
/// diff vs seen）取 z（[`z_of_candidate`]）→ 记 (确认 bar i, z, dir, level)。退出规则**事前固定**：
/// 该候选持有到**下一个反向方向**新确认买卖点的确认 bar（或 train 末 **censored 截断**）兑现 X_γ。
///
/// **因果/泄漏 guard（codex Q1）**：
/// - z 在确认时点（确认 bar i，F_i 可测）取——非回填 vertex retro-entry。
/// - X_γ 用确认 bar 的 close 作 entry、退出 bar 的 close 作 exit（F-可测真实兑现，非端点后视）。
/// - 退出落在 train 窗内（≤ train 末）——跨边界样本截断到 train 末（censored，不偷看 test）。
/// - μ 表 frozen 后喂 test χ 过滤；test 决策不更新 μ（无 test 内未来）。
///
/// 返回 `(MuEstimator, n_signals)`：μ 表 + train 窗新确认买卖点总数（诊断信号密度）。
fn build_walk_forward_mu(train: &Dataset, config: &ThetaConfig) -> (MuEstimator, usize) {
    let bars = &train.bars;
    let n = bars.len();
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;
    let tick = config.tick.tick_size;

    // 逐 bar 因果分类，收集新确认买卖点的 (确认 bar, z, dir, entry_px)。
    let mut classifier_incr = IncrementalClassifier::new(bars, config);
    let mut seen: std::collections::HashSet<(usize, usize, u8)> = std::collections::HashSet::new();
    // 每条新确认信号：(确认 bar i, z, dir)。entry_px 用确认 bar i 的 close（F_i 可测）。
    let mut signals: Vec<(usize, MuClass, VoiceSide)> = Vec::new();

    for i in 0..n {
        let bar = &bars[i];
        if bar.untradable || bar.close <= 0 {
            continue;
        }
        let (cls_i, tower_i) = classifier_incr.classify_at(i);
        // 本 bar 新确认买卖点（append-only diff）。
        for (lvl, ls) in cls_i.levels.iter().enumerate() {
            for p in &ls.bsp {
                if !seen.insert((lvl, p.source_index, bsp_disc(&p.bits))) {
                    continue; // 已确认过，跳过
                }
                // 新确认：取 z（确认时点 F_i 可测）。需经 assemble_gamma_with_tower 拿
                // Candidate（带 dir/role）——构造仅含该点的单级别分类喂 assemble。
                let single = super::super::classifier::Classification {
                    levels: cls_i
                        .levels
                        .iter()
                        .enumerate()
                        .map(|(l2, _)| super::super::classifier::LevelState {
                            moves: Vec::new(),
                            centers: Vec::new(),
                            bsp: if l2 == lvl {
                                vec![p.clone()]
                            } else {
                                Vec::new()
                            },
                        })
                        .collect(),
                };
                let gamma = assemble_gamma_with_tower(&single, &tower_i);
                for c in &gamma {
                    if c.dir == VoiceSide::Flat {
                        continue; // Flat 不可交易候选，不入 μ
                    }
                    let z = z_of_candidate(c);
                    signals.push((i, z, c.dir));
                }
            }
        }
    }

    // 退出兑现（事前固定规则：持有到下一个反向新确认信号确认 bar，或 train 末 censored）。
    // signals 已按确认 bar i 升序（逐 bar 收集），同 bar 多信号保留。
    let mut est = MuEstimator::new();
    let n_signals = signals.len();
    for (idx, &(entry_bar, z, dir)) in signals.iter().enumerate() {
        let entry_px = bars[entry_bar].close as f64 * tick;
        if entry_px <= 0.0 {
            continue;
        }
        // 找下一个反向方向信号的确认 bar（exit）；无则 censored 到 train 末可交易 bar。
        let opp = match dir {
            VoiceSide::Long => VoiceSide::Short,
            VoiceSide::Short => VoiceSide::Long,
            VoiceSide::Flat => continue,
        };
        let exit_bar = signals[idx + 1..]
            .iter()
            .find(|(eb, _, d)| *eb > entry_bar && *d == opp)
            .map(|(eb, _, _)| *eb)
            // censored：无反向信号 ⟹ 截断到 train 末最后可交易 bar（不偷看 test）。
            .unwrap_or_else(|| {
                (0..n)
                    .rev()
                    .find(|&j| j > entry_bar && !bars[j].untradable && bars[j].close > 0)
                    .unwrap_or(entry_bar)
            });
        if exit_bar <= entry_bar {
            continue; // 无合法退出（entry 已是末 bar）⟹ 不兑现
        }
        let exit_px = bars[exit_bar].close as f64 * tick;
        if exit_px <= 0.0 {
            continue;
        }
        let delta: i8 = if dir == VoiceSide::Long { 1 } else { -1 };
        // X_γ = δ(P_exit−P_entry) − C（qty=1 名义单位，μ 是单位边际收益的类条件均值）。
        let x_gamma = marginal_return(entry_px, exit_px, 1.0, fee_rate, delta);
        est.observe(MuObservation { class: z, x_gamma });
    }

    (est, n_signals)
}

/// ΔR 序列统计（§10 净额增量收益的均值/Sharpe/bootstrap p）。
struct DeltaRStats {
    /// ΔR 序列长度（test 有效 bar 数）。
    n: usize,
    /// mean(ΔR)（归一化口径，nav0 单位）。
    mean: f64,
    /// Sharpe(ΔR)（年化，§3.1 √bars_per_year）。
    sharpe: f64,
    /// ΔR block bootstrap 单边 p 值（H0：mean(ΔR)≤0）。
    boot_pvalue: f64,
    /// χ 过滤是否真改变了交易集（ΔN 非全等 ⟺ ΔR 非全 0）——L1 机制门。
    chi_changed_trades: bool,
}

/// 计算 ΔR 序列统计（block bootstrap 单边 p，seed 冻结，复用 metrics 口径）。
fn delta_r_stats(baseline_rets: &[f64], chi_rets: &[f64], nav0: f64, bars_per_year: f64) -> DeltaRStats {
    // 逐 bar 配对差 = ΔR/nav0（codex Q2：ΔR 序列 = nav0·(r^χ−r^0)）。
    let len = baseline_rets.len().min(chi_rets.len());
    let dr_norm: Vec<f64> = (0..len).map(|i| chi_rets[i] - baseline_rets[i]).collect();
    let n = dr_norm.len();
    if n < 2 {
        return DeltaRStats { n, mean: 0.0, sharpe: 0.0, boot_pvalue: 1.0, chi_changed_trades: false };
    }
    let chi_changed = dr_norm.iter().any(|&d| d.abs() > 1e-15);
    let mean = dr_norm.iter().sum::<f64>() / n as f64;
    let var = dr_norm.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    let std = var.sqrt();
    let sharpe = if std > 1e-18 { mean / std * bars_per_year.sqrt() } else { 0.0 };

    // ΔR 序列 block bootstrap（H0：mean ≤ 0）——seed 冻结，块长保留 bar 级自相关。
    let mut rng = SplitMix64::new(PREREG_SEED);
    const N_RESAMPLE: usize = 1000;
    const BLOCK: usize = 20; // 协议 §3.4 块长 20（bar 序列自相关）
    let mut n_le_0 = 0usize;
    for _ in 0..N_RESAMPLE {
        let mut total = 0.0;
        let mut filled = 0usize;
        while filled < n {
            let start = rng.next_below(n);
            let take = BLOCK.min(n - filled);
            for k in 0..take {
                total += dr_norm[(start + k) % n];
            }
            filled += take;
        }
        if total <= 0.0 {
            n_le_0 += 1;
        }
    }
    let boot_pvalue = n_le_0 as f64 / N_RESAMPLE as f64;
    let _ = nav0; // mean/sharpe 在归一化口径上，nav0 仅文档语义

    DeltaRStats {
        n,
        mean,
        sharpe,
        boot_pvalue,
        chi_changed_trades: chi_changed,
    }
}

/// 二项符号检验单边 p 值（H0：ΔR 均值符号随机，p=0.5）。
///
/// `n_pos` 个品种 ΔR>0，共 `n` 个 ⟹ p = P(X≥n_pos | Bin(n,0.5))（上单边）。
/// codex Q4：8 品种需 ≥7/8 正才到单边 p<0.05。
fn sign_test_pvalue(n_pos: usize, n: usize) -> f64 {
    if n == 0 {
        return 1.0;
    }
    // P(X≥n_pos) = Σ_{k=n_pos}^{n} C(n,k) / 2^n。
    let mut comb = 1.0f64; // C(n, n) = 1，从 k=n 往下递推
    let mut sum = 0.0f64;
    for k in (0..=n).rev() {
        // 此处 comb = C(n,k)（从 k=n 的 C(n,n)=1 往下递推）。
        if k >= n_pos {
            sum += comb;
        }
        // C(n,k-1) = C(n,k) * k / (n-k+1)
        if k > 0 {
            comb = comb * k as f64 / (n - k + 1) as f64;
        }
    }
    sum / 2.0f64.powi(n as i32)
}

/// **★Phase-3 ΔR 净额增量 alpha 否证：8 品种 OOS-split walk-forward + ΔR 序列 + 三层判定**。
///
/// `#[ignore]`，需 8 品种数据；O(n²) 慢测 `--release` 必须（train+test 各 16K 逐 bar 重分类）。
#[test]
#[ignore = "Phase-3 ΔR alpha 否证；O(n²) OOS-split walk-forward；需 8 品种数据；--release"]
fn delta_r_alpha_multi_symbol() {
    let config = ThetaConfig::default();
    // θ 常数（成本/风险门槛 Θ_risk 参数，非缠论可导；不从样本 μ 分布选 = 无 in-sample 泄漏）。
    // θ=0：只滤负边际期望类（§13 θ≥0 下界，θ=0 滤 μ≤0 已观测类）。
    let theta: f64 = 0.0;

    eprintln!("\n===== Phase-3 ΔR 净额增量 alpha 否证：8 品种 OOS-split walk-forward（θ={theta}）=====");
    eprintln!("★主口径 = ΔR 序列自身（codex：Sharpe(A)−Sharpe(B)≠Sharpe(A−B)）；ΔR=nav0·(r^χ−r^0) 逐 bar 配对差");
    eprintln!("★walk-forward μ：OOS 前半 train（估 μ，确认时点 z + 下反向信号/censored 兑现）→ 后半 test（χ 过滤）");
    eprintln!("★鞅定理（§11/§16）：无预测性则无 alpha——μ 表噪声 ⟹ ΔR 符号跨品种随机、均值≈0");
    eprintln!("★截断窗 {MAX_BARS}bar（O(n²) 全窗不可行）= 显式有效域边界（非全窗结论）");
    eprintln!(
        "{:<6} {:>8} {:>6} {:>7} {:>10} {:>9} {:>9} {:>8} {:>8} {}",
        "symbol", "train", "test", "μ类", "mean(ΔR)", "Shrp(ΔR)", "boot_p", "ΔN≠0", "χ单/0单", "归因",
    );

    let mut dr_means: Vec<(String, f64, bool)> = Vec::new(); // (symbol, mean(ΔR), is_l2_valid)
    let mut n_done = 0usize;
    let mut n_l2_confirm = 0usize; // L2 单品种 alpha 确认
    let mut n_l2_falsify = 0usize; // L2 否证（mean≤0 或 p>0.05）
    let mut n_inconclusive = 0usize; // 样本饥饿/χ 未改变交易集/工程断流

    for w in PREREG_WINDOWS {
        let ds = match data::load_by_symbol(w.symbol, &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{:<6} 加载失败：{e}（DATA BLOCKER，不伪造合成）", w.symbol);
                continue;
            }
        };
        let oos_full = ds.slice_date_window(w.oos.0, w.oos.1);
        if oos_full.bars.is_empty() {
            eprintln!("{:<6} OOS 窗空（{}→{}）", w.symbol, w.oos.0, w.oos.1);
            continue;
        }
        // 截断到 MAX_BARS，OOS 内 split：前半 train（μ 估计）/ 后半 test（χ vs baseline）。
        let cut = MAX_BARS.min(oos_full.bars.len());
        let split = cut / 2;
        let mk = |lo: usize, hi: usize| Dataset {
            symbol: oos_full.symbol.clone(),
            bars: oos_full.bars[lo..hi].to_vec(),
            dates: oos_full.dates[lo..hi.min(oos_full.dates.len())].to_vec(),
            bar_seconds: 60,
        };
        let train = mk(0, split);
        let test = mk(split, cut);
        if train.bars.len() < MIN_TEST_BARS || test.bars.len() < MIN_TEST_BARS {
            eprintln!("{:<6} train/test 样本不足（train={} test={}）⟹ inconclusive", w.symbol, train.bars.len(), test.bars.len());
            n_inconclusive += 1;
            n_done += 1;
            continue;
        }
        n_done += 1;

        // walk-forward μ 表（train 窗，frozen）。
        let (est, n_sig) = build_walk_forward_mu(&train, &config);

        // test 窗 NAV（首可交易价×1000，下限 1e6——与 l3_pi_falsify 同口径）。
        let first_px = test
            .bars
            .iter()
            .find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * config.tick.tick_size)
            .unwrap_or(1.0);
        let nav = (first_px * 1000.0).max(1.0e6);
        let bars_per_year = data::bars_per_year(60);
        let years = (test.bars.len() as f64 / bars_per_year).max(0.01);

        // χ≡1 baseline（chi_theta=None）。
        let mut base_cfg = config.clone();
        base_cfg.risk.chi_theta = None;
        let base = run_theta_v0_pi_chi(&test, &base_cfg, years, nav, &est, true);

        // χ 过滤（chi_theta=Some(θ)，treat_empty_as_pass=false=最诚实：无 μ 证据不交易，codex Q3）。
        let mut chi_cfg = config.clone();
        chi_cfg.risk.chi_theta = Some(theta);
        let chi = run_theta_v0_pi_chi(&test, &chi_cfg, years, nav, &est, false);

        // ΔR 序列统计（codex Q2：ΔR = nav0·(r^χ−r^0) 逐 bar 配对差，主口径）。
        let st = delta_r_stats(&base.daily_returns, &chi.daily_returns, nav, bars_per_year);

        // 分层诊断（强制；(a)/(c)/(d) 严禁谎报否证 (b) 或确认 (c')）。
        // ★(d) χ 退化空仓 guard（实证发现 + codex Q3）：chi.n_orders==0 ∧ base.n_orders>0 ⟹
        // χ 门太严把**所有**交易滤光 ⟹ ΔR = 0 − baseline = −baseline_returns。若 baseline 亏损，
        // 则 mean(ΔR)>0 / Sharpe(ΔR) 巨大**纯属"空仓躲过亏损"**，**不是 χ 选择 alpha**（χ 没在选，
        // 是全拒）。这是工程退化，严禁谎报 (c') L2 确认（违反分层诊断铁律 + 覆盖≠盈利 645）。
        let chi_degenerate_flat = chi.n_orders == 0 && base.n_orders > 0;
        let verdict = if base.n_orders == 0 && chi.n_orders == 0 {
            n_inconclusive += 1;
            "(a)工程断流(双0单)"
        } else if chi_degenerate_flat {
            n_inconclusive += 1;
            "(d)χ退化空仓(全滤=躲亏非选)"
        } else if !st.chi_changed_trades {
            // χ 未改变交易集（μ 表全 pass）⟹ ΔR≡0，非否证，是 χ 无作用。
            n_inconclusive += 1;
            "(c)χ未改交易集ΔN≡0"
        } else if st.n < MIN_TEST_BARS {
            n_inconclusive += 1;
            "(c)ΔR样本饥饿"
        } else if st.mean > 0.0 && st.boot_pvalue <= 0.05 {
            n_l2_confirm += 1;
            "(c')L2确认:ΔR>0且p≤.05"
        } else {
            n_l2_falsify += 1;
            "(b)L2否证:ΔR≤0或p>.05"
        };

        // 跨品种符号检验输入（仅 χ **真选子集**的有效品种计入 L3——排除退化空仓 (d)，
        // 它的"正 ΔR"是躲亏伪影非选择 alpha，计入会污染系统性判定）。
        let l3_valid = st.chi_changed_trades && chi.n_orders > 0 && base.n_orders > 0;
        dr_means.push((w.symbol.to_string(), st.mean, l3_valid));

        eprintln!(
            "{:<6} {:>8} {:>6} {:>7} {:>10.3e} {:>9.3} {:>9.4} {:>8} {:>4}/{:<4} {}",
            w.symbol,
            train.bars.len(),
            test.bars.len(),
            est.n_classes(),
            st.mean,
            st.sharpe,
            st.boot_pvalue,
            st.chi_changed_trades,
            chi.n_orders,
            base.n_orders,
            verdict,
        );
        let _ = n_sig;

        // 不变量：管线不崩 + 检验值合法。
        assert!(st.mean.is_finite(), "{} mean(ΔR) 有限", w.symbol);
        assert!((0.0..=1.0).contains(&st.boot_pvalue), "{} boot p∈[0,1]", w.symbol);
    }

    // ── L3 跨品种系统性 alpha（符号检验，codex Q4）──
    let l3_pool: Vec<f64> = dr_means.iter().filter(|(_, _, v)| *v).map(|(_, m, _)| *m).collect();
    let n_l3 = l3_pool.len();
    let n_pos = l3_pool.iter().filter(|&&m| m > 0.0).count();
    let l3_mean = if n_l3 > 0 { l3_pool.iter().sum::<f64>() / n_l3 as f64 } else { 0.0 };
    let sign_p = sign_test_pvalue(n_pos, n_l3);

    eprintln!("\n===== Phase-3 ΔR 跨品种聚合（8 品种 OOS-split walk-forward）=====");
    eprintln!("完成品种数                          : {n_done}/{}", PREREG_WINDOWS.len());
    eprintln!("(a)/(c) inconclusive(断流/χ未改/饥饿): {n_inconclusive}");
    eprintln!("(b) L2 否证(mean(ΔR)≤0 或 p>.05)     : {n_l2_falsify}");
    eprintln!("(c') L2 确认(mean(ΔR)>0 且 p≤.05)     : {n_l2_confirm}");
    eprintln!("\n--- L3 系统性 alpha（χ 真改交易集的有效品种）---");
    eprintln!("有效品种数 n_L3                      : {n_l3}");
    eprintln!("ΔR 均值>0 的品种数 n_pos             : {n_pos}/{n_l3}");
    eprintln!("跨品种 ΔR 池均值                     : {l3_mean:.3e}");
    eprintln!("符号检验单边 p（H0:符号随机 p=.5）    : {sign_p:.4}");
    eprintln!(
        "\n★诚实裁定（codex Q4 三层 + formalization-validity-domain 231号 + alpha2 §11 鞅定理）：\n  \
         - **L3 系统性 alpha 成立** ⟺ n_pos/n_L3 符号检验 p<0.05（8 品种需 ≥7/8 正）∧ 池均值>0。\n  \
         - 否则：若多数 (a)/(c) ⟹ substrate/工程缺口（O(n²) 截断致结构饥饿/χ 未改交易集），inconclusive。\n  \
         - 若多数 (b) 且 χ 真改交易集 ⟹ **χ 选择器不产生系统性净额增量 alpha**（鞅定理预言的真否证：\n    \
           walk-forward μ 表未捕获持续可预测性，ΔR 符号跨品种随机）——**否定性结果，缩小有效域**。\n  \
         - 单品种 ΔSharpe≠0 不算 alpha（codex：8 品种里 ≥1 偶然为正概率 99.6%）——必须看跨品种系统性。\n  \
         - 有效域 caveat：保守欠对冲版 pi（ρ漂移剪枝）+ {MAX_BARS}bar 截断 + OOS 内 split（非独立 train 历史）。"
    );

    // acceptance：管线在多标的真实数据上跑通（≥1 品种完成或全 DATA BLOCKER 诚实计数）。
    assert_eq!(
        n_l2_confirm + n_l2_falsify + n_inconclusive,
        n_done,
        "分层完备：每完成品种恰归一类",
    );
    // L3 系统性 alpha 判定（诚实记录，非强制 PASS——否定性结果是合法产出，231号）。
    let l3_systematic = n_l3 >= 2 && sign_p < 0.05 && l3_mean > 0.0;
    eprintln!(
        "\n★★最终判定：{}",
        if l3_systematic {
            "L3 系统性净额增量 alpha 成立（符号检验 p<.05 ∧ 池均值>0）——χ 选择器捕获持续可预测性"
        } else if n_l3 >= 2 {
            "L3 系统性 alpha 不成立（符号检验未达 p<.05 或池均值≤0）——χ 选择器无系统性 alpha（鞅定理否证）"
        } else {
            "L3 不可判（χ 真改交易集的有效品种<2）——substrate/工程缺口主导，inconclusive"
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 符号检验 p 值已知值（二项 Bin(n,0.5) 上单边）。
    #[test]
    fn sign_test_known_values() {
        // 8 全正：P(X≥8|Bin(8,.5)) = 1/256 ≈ 0.0039。
        assert!((sign_test_pvalue(8, 8) - 1.0 / 256.0).abs() < 1e-9, "8/8 正 p=1/256");
        // 7/8 正：P(X≥7) = (C(8,7)+C(8,8))/256 = 9/256 ≈ 0.0352 < 0.05。
        assert!((sign_test_pvalue(7, 8) - 9.0 / 256.0).abs() < 1e-9, "7/8 正 p=9/256<0.05");
        // 6/8 正：P(X≥6) = (28+8+1)/256 = 37/256 ≈ 0.1445 > 0.05（codex：6/8 不够）。
        assert!(sign_test_pvalue(6, 8) > 0.05, "6/8 正 p>0.05（不达系统性）");
        // 4/8（半数）：p > 0.5（不显著）。
        assert!(sign_test_pvalue(4, 8) > 0.5, "4/8 正 p>0.5");
        // 边界：n=0 ⟹ p=1。
        assert_eq!(sign_test_pvalue(0, 0), 1.0, "无样本 p=1");
    }

    /// ΔR 序列统计：恒等收益（χ==baseline）⟹ ΔR≡0 ⟹ chi_changed_trades=false（L1 机制门）。
    #[test]
    fn delta_r_identical_returns_zero() {
        let rets = vec![0.01, -0.02, 0.03, 0.0, 0.01];
        let st = delta_r_stats(&rets, &rets, 1.0e6, 252.0);
        assert!(!st.chi_changed_trades, "χ==baseline ⟹ ΔR≡0 ⟹ χ 未改交易集");
        assert!(st.mean.abs() < 1e-15, "ΔR 均值=0");
    }

    /// ΔR 序列统计：χ 收益恒高于 baseline ⟹ mean(ΔR)>0 ∧ boot p 小（确认管线产正结果能力）。
    #[test]
    fn delta_r_positive_when_chi_dominates() {
        let base = vec![0.0; 300];
        let chi: Vec<f64> = vec![0.001; 300]; // χ 每 bar +0.1% 超额
        let st = delta_r_stats(&base, &chi, 1.0e6, 252.0);
        assert!(st.chi_changed_trades, "χ≠baseline ⟹ ΔN 非全等");
        assert!(st.mean > 0.0, "χ 主导 ⟹ mean(ΔR)>0，实得 {}", st.mean);
        assert!(st.boot_pvalue < 0.05, "恒正 ΔR ⟹ bootstrap p<0.05（拒绝 H0:ΔR≤0），实得 {}", st.boot_pvalue);
    }

    /// ΔR 序列统计：χ 恒低于 baseline ⟹ mean(ΔR)<0 ∧ boot p≈1（否证能力——不冒充正 alpha）。
    #[test]
    fn delta_r_negative_when_chi_worse() {
        let base = vec![0.001; 300];
        let chi = vec![0.0; 300]; // χ 每 bar 少赚 0.1%
        let st = delta_r_stats(&base, &chi, 1.0e6, 252.0);
        assert!(st.mean < 0.0, "χ 劣于 baseline ⟹ mean(ΔR)<0");
        assert!(st.boot_pvalue > 0.95, "恒负 ΔR ⟹ bootstrap p≈1（无法拒绝 H0，正确否证），实得 {}", st.boot_pvalue);
    }
}
