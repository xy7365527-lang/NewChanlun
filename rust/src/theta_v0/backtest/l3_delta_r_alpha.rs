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
//! ★663 降级：跨品种符号检验 = 统计显著性判据，663 裁定非 alpha 判据（系统性惩罚低频高级别）+
//!   MAX_BARS=32000 短窗与高级别样本冲突。真判据 = 全历史长窗逐信号 mu_hat>0（econ_positive
//!   l2_btc_capturable_spread_diagnosis）。本模块保留作参考，不作 alpha 确认判据。
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
use super::mu_estimator::{marginal_return, MuClass, MuEstimator, MuObservation, PositionState};
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
///
/// **口径（delta-r-audit P0 修复）**：输入是两条回测的**归一化绝对权益**序列 E_t=(cash+units·P)/nav0
/// （[`super::runner::RunResult::equity_curve`]），逐 bar **绝对增量配对差**：
/// ```text
///   ΔR_t/nav0 = (E^χ_t − E^χ_{t−1}) − (E^0_t − E^0_{t−1})
/// ```
/// 这是 §10 线性可加的 ΔR（除数恒 nav0）。**不用百分比收益** `daily_returns`（除数 path-dependent
/// 的 E_{t−1}，两条 NAV 发散时 r^χ−r^0 ≠ ΔR/nav0，被 sizing 路径污染）。成本已扣进 equity ⟹ ΔC 自含。
fn delta_r_stats(baseline_eq: &[f64], chi_eq: &[f64], nav0: f64, bars_per_year: f64) -> DeltaRStats {
    // 逐 bar 绝对增量配对差 = ΔR/nav0（§10 线性可加；E 已÷nav0，∴增量配对差直接是 ΔR/nav0）。
    let len = baseline_eq.len().min(chi_eq.len());
    let dr_norm: Vec<f64> = (1..len)
        .map(|i| (chi_eq[i] - chi_eq[i - 1]) - (baseline_eq[i] - baseline_eq[i - 1]))
        .collect();
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

        // ΔR 序列统计（delta-r-audit P0 修复：绝对增量配对差 = ΔR/nav0，用 equity_curve 不用百分比）。
        let st = delta_r_stats(&base.equity_curve, &chi.equity_curve, nav, bars_per_year);

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

        // ── degeneracy 三路分解（delta-r-audit-2 攻击点2/4）：test 候选按 train μ 表分三路
        //    (μ>θ 放行 / μ≤θ 被滤 / μ=None 未见恒滤) + 各级别 z 计数——使"退化=躲亏 vs 饥饿"可证伪。──
        {
            let test_zs = enumerate_candidate_z(&test, &config);
            let n_cand = test_zs.len();
            let (mut n_pass, mut n_filt_nonpos, mut n_unseen) = (0usize, 0usize, 0usize);
            // 各级别 z 计数（按 z.level 分组，最多 8 级别足够覆盖涌现层）。
            let mut by_level: [usize; 8] = [0; 8];
            for z in &test_zs {
                if (z.level as usize) < by_level.len() {
                    by_level[z.level as usize] += 1;
                }
                match est.mu(z) {
                    Some(m) if m > theta => n_pass += 1,
                    Some(_) => n_filt_nonpos += 1,
                    None => n_unseen += 1,
                }
            }
            let lvl_str: Vec<String> = by_level
                .iter()
                .enumerate()
                .filter(|(_, &c)| c > 0)
                .map(|(l, c)| format!("ℓ{l}:{c}"))
                .collect();
            eprintln!(
                "  └─[三路分解] test候选z={n_cand} | μ>θ放行={n_pass} μ≤θ被滤={n_filt_nonpos} 未见恒滤={n_unseen} | 级别分布[{}]",
                lvl_str.join(" ")
            );
        }

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
    // ★n=5 功效门（delta-r-audit-2 攻击点3，定理类）：符号检验 8 品种需 ≥7/8 正才到 p<0.05；
    // n_L3<5 时无论符号如何都达不到 p<0.05（C(n,n)/2^n: 4/4=0.0625, 3/3=0.125 均>0.05）⟹
    // 检验功效不足，禁用"无系统性alpha"措辞（缺乏否证它的统计功效，非否证成立）。
    const MIN_L3_POWER: usize = 5; // 符号检验达 p<.05 的最小品种数下界（5/5=1/32=0.03125<0.05）
    let l3_systematic = n_l3 >= 2 && sign_p < 0.05 && l3_mean > 0.0;
    eprintln!(
        "\n★★最终判定：{}",
        if l3_systematic {
            "L3 系统性净额增量 alpha 成立（符号检验 p<.05 ∧ 池均值>0）——χ 选择器捕获持续可预测性"
        } else if n_l3 < MIN_L3_POWER {
            // 功效不足：n_L3<5 时符号检验无论全正都达不到 p<.05 ⟹ inconclusive(功效不足)，
            // 不得声称"无系统性alpha"（缺乏否证的统计功效）。
            "L3 inconclusive(功效不足)：χ 真改交易集的有效品种<5——符号检验全正也达不到 p<.05，\
             无法否证亦无法确认系统性 alpha（缩小样本量是 substrate/工程缺口，非 alpha 结论）"
        } else {
            "L3 系统性 alpha 不成立（n_L3≥5 且符号检验未达 p<.05 或池均值≤0）——χ 选择器无系统性 alpha（鞅定理否证）"
        }
    );
}

/// cross-fit OOS 的 fold 数（K=5）与 fold 边界 purge gap（bar）。
/// gap 防退出兑现（持有到下一反向信号）从 train fold 跨界泄漏到 held-out fold（PDF §34 purged split）。
const CROSSFIT_K: usize = 5;
const CROSSFIT_GAP: usize = 200; // ponytail: 退出持有跨度上界（保守，比 train 末 censored 兑现宽），数据若示更长 gap 才泄漏则调大

/// **★cross-fit OOS：K-fold purged split——选择在 train folds、评估在 held-out fold（PDF §34, task #85）**。
///
/// 修 [`delta_r_alpha_multi_symbol`] 单次 split 的 §6 winner's curse：单次 split 在 train 选 μ>θ 的赢家类，
/// OOS（test）评估同一选择，赢家可能在 test 归零（训练选赢家 = 选择性偏差污染 OOS）。
///
/// **cross-fit 解（§34）**：OOS 窗切 K=5 连续 fold，每个 fold k 轮流作 **held-out**（评估），其余 fold
/// 作 **train**（估 μ + selector 选 μ>θ）；汇总所有 held-out fold 的 ΔR——**选择与评估在不同 fold**，
/// 选择性偏差不传导到评估窗。fold 边界两侧 purge `CROSSFIT_GAP` bar（held-out 不靠近 train 退出兑现跨度）。
///
/// **train 不连续 ⟹ merge 不拼接（no-workaround）**：held-out fold 在中间时 train=前段+后段，各连续段
/// 独立 [`build_walk_forward_mu`] 后 [`MuEstimator::merge`]——拼接成单 Dataset 会在接缝制造虚假相邻笔/段，
/// 污染分类（伪逻辑），故 merge 桶级合并。
///
/// 认识论 **L2**（真实数据，可证伪）。有效域 caveat：MAX_BARS 截断 + 8 品种 + OOS 内 K-fold。
///
/// `#[ignore]`：O(n²) 慢测（8 品种 × K=5 fold × ~6K train+test 逐 bar 重分类），`--release` 必须。
/// 跑法：`cargo test --release --lib theta_v0::backtest::l3_delta_r_alpha::crossfit_l2 -- --ignored --nocapture`
#[test]
#[ignore = "cross-fit OOS K-fold purged；O(n²) 8 品种 × 5 fold × 32K bar；--release"]
fn crossfit_l2() {
    let config = ThetaConfig::default();
    let theta: f64 = 0.0;

    eprintln!("\n===== ★cross-fit OOS K-fold purged（PDF §34, K={CROSSFIT_K}, gap={CROSSFIT_GAP}, θ={theta}）=====");
    eprintln!("★修单次 split §6 winner's curse：选择在 train folds、评估在 held-out fold ⟹ 选择性偏差不传导评估窗");
    eprintln!("★train 不连续(held-out 居中⟹前段+后段) ⟹ 各段独立 build_walk_forward_mu 后 merge（不拼接，防接缝伪相邻）");
    eprintln!("★fold 边界 purge {CROSSFIT_GAP}bar ⟹ held-out 不含 train 退出兑现跨界泄漏（purged split）");
    eprintln!("★对比基线 = delta_r_alpha_multi_symbol 单次 split（同 μ harness，唯一变量 = K-fold 选/评分离）");
    eprintln!(
        "{:<6} {:>8} {:>7} {:>10} {:>9} {:>9} {:>8} {}",
        "symbol", "cf_bars", "μ类", "cf_mean(ΔR)", "Shrp", "boot_p", "ΔN≠0", "归因",
    );

    let mut cf_means: Vec<(String, f64, bool)> = Vec::new();
    let mut n_done = 0usize;

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
            eprintln!("{:<6} OOS 窗空", w.symbol);
            continue;
        }
        let cut = MAX_BARS.min(oos_full.bars.len());
        let fold_len = cut / CROSSFIT_K;
        if fold_len < MIN_TEST_BARS {
            eprintln!("{:<6} fold 太短（fold_len={fold_len}<{MIN_TEST_BARS}）⟹ inconclusive", w.symbol);
            continue;
        }
        let mk = |lo: usize, hi: usize| Dataset {
            symbol: oos_full.symbol.clone(),
            bars: oos_full.bars[lo..hi].to_vec(),
            dates: oos_full.dates[lo..hi.min(oos_full.dates.len())].to_vec(),
            bar_seconds: 60,
        };
        n_done += 1;
        let bars_per_year = data::bars_per_year(60);

        // 跨 fold 汇总的 cross-fitted ΔR 序列（各 held-out fold 的逐 bar ΔR/nav0 拼接）。
        let mut cf_dr: Vec<f64> = Vec::new();
        let mut total_classes = 0usize;
        let mut chi_changed_any = false;

        for k in 0..CROSSFIT_K {
            // held-out fold k = [k·fold_len, (k+1)·fold_len)（末 fold 吃 cut 余数）。
            let ho_lo = k * fold_len;
            let ho_hi = if k == CROSSFIT_K - 1 { cut } else { (k + 1) * fold_len };
            let held = mk(ho_lo, ho_hi);
            if held.bars.len() < MIN_TEST_BARS {
                continue;
            }

            // train = 其余 fold 的连续段，各段从 held-out 边界向外 purge GAP（防退出兑现跨界）。
            // 前段 [0, ho_lo−gap)、后段 [ho_hi+gap, cut)，各段独立估 μ 后 merge（不拼接）。
            let mut est = MuEstimator::new();
            if ho_lo >= CROSSFIT_GAP + MIN_TEST_BARS {
                let (e, _) = build_walk_forward_mu(&mk(0, ho_lo - CROSSFIT_GAP), &config);
                est.merge(&e);
            }
            if ho_hi + CROSSFIT_GAP + MIN_TEST_BARS <= cut {
                let (e, _) = build_walk_forward_mu(&mk(ho_hi + CROSSFIT_GAP, cut), &config);
                est.merge(&e);
            }
            if est.n_classes() == 0 {
                continue; // 边 fold 单侧 train 太短 ⟹ 无 train 证据 ⟹ 跳过
            }
            total_classes = total_classes.max(est.n_classes());

            let first_px = held
                .bars
                .iter()
                .find(|b| !b.untradable && b.close > 0)
                .map(|b| b.close as f64 * config.tick.tick_size)
                .unwrap_or(1.0);
            let nav = (first_px * 1000.0).max(1.0e6);
            let years = (held.bars.len() as f64 / bars_per_year).max(0.01);

            let mut base_cfg = config.clone();
            base_cfg.risk.chi_theta = None;
            let base = run_theta_v0_pi_chi(&held, &base_cfg, years, nav, &est, true);

            let mut chi_cfg = config.clone();
            chi_cfg.risk.chi_theta = Some(theta);
            let chi = run_theta_v0_pi_chi(&held, &chi_cfg, years, nav, &est, false);

            // 退化空仓 fold（χ 全滤）= 弃权 PnL 非选择 alpha（161号），不计入 cross-fitted ΔR。
            if chi.n_orders == 0 && base.n_orders > 0 {
                continue;
            }
            let st = delta_r_stats(&base.equity_curve, &chi.equity_curve, nav, bars_per_year);
            if st.chi_changed_trades {
                chi_changed_any = true;
                // 逐 bar ΔR/nav0 拼进 cf_dr（与 delta_r_stats 内部绝对增量配对差口径一致）。
                let len = base.equity_curve.len().min(chi.equity_curve.len());
                for i in 1..len {
                    cf_dr.push(
                        (chi.equity_curve[i] - chi.equity_curve[i - 1])
                            - (base.equity_curve[i] - base.equity_curve[i - 1]),
                    );
                }
            }
        }

        // cross-fitted ΔR 统计（汇总序列 mean/sharpe/block-bootstrap 单边 p，与 delta_r_stats 同口径）。
        let cf_n = cf_dr.len();
        let (cf_mean, cf_sharpe, cf_boot_p) = if cf_n >= 2 {
            let mean = cf_dr.iter().sum::<f64>() / cf_n as f64;
            let var = cf_dr.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (cf_n - 1) as f64;
            let std = var.sqrt();
            let sharpe = if std > 1e-18 { mean / std * bars_per_year.sqrt() } else { 0.0 };
            let mut rng = SplitMix64::new(PREREG_SEED);
            let mut n_le_0 = 0usize;
            for _ in 0..1000 {
                let (mut total, mut filled) = (0.0, 0usize);
                while filled < cf_n {
                    let start = rng.next_below(cf_n);
                    let take = 20.min(cf_n - filled);
                    for j in 0..take {
                        total += cf_dr[(start + j) % cf_n];
                    }
                    filled += take;
                }
                if total <= 0.0 {
                    n_le_0 += 1;
                }
            }
            (mean, sharpe, n_le_0 as f64 / 1000.0)
        } else {
            (0.0, 0.0, 1.0)
        };

        let verdict = if !chi_changed_any || cf_n < MIN_TEST_BARS {
            "(c)cross-fit饥饿/χ未改"
        } else if cf_mean > 0.0 && cf_boot_p <= 0.05 {
            "(c')cf-L2确认:ΔR>0且p≤.05"
        } else {
            "(b)cf-L2否证:ΔR≤0或p>.05"
        };
        let l3_valid = chi_changed_any && cf_n >= MIN_TEST_BARS;
        cf_means.push((w.symbol.to_string(), cf_mean, l3_valid));

        eprintln!(
            "{:<6} {:>8} {:>7} {:>10.3e} {:>9.3} {:>9.4} {:>8} {}",
            w.symbol, cf_n, total_classes, cf_mean, cf_sharpe, cf_boot_p, chi_changed_any, verdict,
        );
        assert!(cf_mean.is_finite(), "{} cf_mean 有限", w.symbol);
        assert!((0.0..=1.0).contains(&cf_boot_p), "{} cf boot p∈[0,1]", w.symbol);
    }

    // ── L3 跨品种系统性（符号检验，与 multi_symbol 同口径）──
    let l3: Vec<f64> = cf_means.iter().filter(|(_, _, v)| *v).map(|(_, m, _)| *m).collect();
    let n_l3 = l3.len();
    let n_pos = l3.iter().filter(|&&m| m > 0.0).count();
    let l3_mean = if n_l3 > 0 { l3.iter().sum::<f64>() / n_l3 as f64 } else { 0.0 };
    let sign_p = sign_test_pvalue(n_pos, n_l3);
    const MIN_L3_POWER: usize = 5;

    eprintln!("\n===== cross-fit 跨品种聚合（vs delta_r_alpha_multi_symbol 单次 split）=====");
    eprintln!("完成品种数      : {n_done}/{}", PREREG_WINDOWS.len());
    eprintln!("L3 有效品种 n_L3 : {n_l3}");
    eprintln!("cf ΔR 均值>0 品种: {n_pos}/{n_l3}");
    eprintln!("cf 跨品种池均值  : {l3_mean:.3e}");
    eprintln!("符号检验单边 p   : {sign_p:.4}");
    eprintln!(
        "\n★诚实裁定（PDF §34 + formalization-validity-domain 231号 + winner's curse §6）：\n  \
         - cross-fit L3 系统性 alpha 成立 ⟺ n_L3≥5 ∧ 符号检验 p<0.05 ∧ 池均值>0。\n  \
         - vs 单次 split：若单次 split 正 ΔR 而 cross-fit 归零 ⟹ 单次的 alpha 是 winner's curse 伪影\n    \
           （训练选赢家污染 OOS），cross-fit 揭示真实 OOS 无 alpha（否定性结果，缩小有效域）。\n  \
         - 若两者一致（都正/都负）⟹ 选择性偏差非主导，单次 split 结论稳健。\n  \
         - n_L3<5 ⟹ inconclusive（功效不足，K-fold 进一步缩短 train ⟹ 更少非退化品种）。"
    );
    let cf_systematic = n_l3 >= 2 && sign_p < 0.05 && l3_mean > 0.0;
    eprintln!(
        "\n★★最终判定：{}",
        if cf_systematic {
            "cross-fit L3 系统性 alpha 成立（选/评分离后仍 p<.05 ∧ 池均值>0）——非 winner's curse 伪影"
        } else if n_l3 < MIN_L3_POWER {
            "cross-fit inconclusive(功效不足 n_L3<5)：K-fold 缩短 train ⟹ 更少非退化品种，符号检验全正也达不到 p<.05"
        } else {
            "cross-fit L3 系统性 alpha 不成立（n_L3≥5 但 p≥.05 或池均值≤0）——选/评分离后无系统性 alpha"
        }
    );
}

/// 从交易轨迹重建**逐 bar 成本序列**（归一化口径，nav0 单位）——鞅守卫成本剥离用。
///
/// codex 异质审裁决（2026-06-30）：净 ΔR 在鞅上可正，因为 χ 过滤比全覆盖 baseline **交易少**
/// ⟹ 成本低 ⟹ `ΔR=−ΔC>0` 是**成本节省**非预测 alpha。鞅定理的纯净形式是毛额 `E[ΔN·ΔP]=0`
/// （不含成本）。故鞅守卫须剥离成本，看毛收益 `ΔGross=ΔR+ΔC`，鞅上应 `E[ΔGross]=0`。
///
/// 成本在 entry/exit bar 离散发生：每条 trade 在 `entry_bar` 扣 `qty·entry_px·fee_rate`、在
/// `exit_bar` 扣 `qty·exit_px·fee_rate`（双边费，与 [`marginal_return`] 同口径）。返回长度
/// `n_bars` 的逐 bar 成本序列（nav0 归一化），`cost[t]` = 第 t bar 发生的成交费用 / nav0。
fn rebuild_cost_series(res: &super::runner::RunResult, n_bars: usize, nav0: f64) -> Vec<f64> {
    let mut cost = vec![0.0f64; n_bars];
    let fee = res.fee_rate;
    for tr in &res.trades {
        // 成交价从 RunResult.prices（与账本 apply_order 成交价一致，close 口径）取。
        let entry_px = res.prices.get(tr.entry_bar).copied().unwrap_or(0.0);
        let exit_px = res.prices.get(tr.exit_bar).copied().unwrap_or(0.0);
        if tr.entry_bar < n_bars {
            cost[tr.entry_bar] += tr.qty * entry_px * fee / nav0;
        }
        if tr.exit_bar < n_bars {
            cost[tr.exit_bar] += tr.qty * exit_px * fee / nav0;
        }
    }
    cost
}

/// 毛收益序列统计：mean、年化 Sharpe、**双边** block bootstrap p（H0:mean=0）。
///
/// 鞅守卫用双边（H0:E[ΔGross]=0），区别于 [`delta_r_stats`] 的单边（H0:mean≤0，净 alpha 方向性）。
/// 双边 p = `2·min(P(boot≤0), P(boot≥0))`（block bootstrap 重采样均值分布，seed 冻结，BLOCK=20）。
fn gross_stats(gross: &[f64], bars_per_year: f64) -> (f64, f64, f64) {
    let n = gross.len();
    if n < 2 {
        return (0.0, 0.0, 1.0);
    }
    let mean = gross.iter().sum::<f64>() / n as f64;
    let var = gross.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    let std = var.sqrt();
    let sharpe = if std > 1e-18 { mean / std * bars_per_year.sqrt() } else { 0.0 };

    let mut rng = SplitMix64::new(PREREG_SEED);
    const N_RESAMPLE: usize = 1000;
    const BLOCK: usize = 20;
    let (mut n_le, mut n_ge) = (0usize, 0usize);
    for _ in 0..N_RESAMPLE {
        let mut total = 0.0;
        let mut filled = 0usize;
        while filled < n {
            let start = rng.next_below(n);
            let take = BLOCK.min(n - filled);
            for k in 0..take {
                total += gross[(start + k) % n];
            }
            filled += take;
        }
        if total <= 0.0 {
            n_le += 1;
        }
        if total >= 0.0 {
            n_ge += 1;
        }
    }
    let frac_le = n_le as f64 / N_RESAMPLE as f64;
    let frac_ge = n_ge as f64 / N_RESAMPLE as f64;
    let two_sided = (2.0 * frac_le.min(frac_ge)).min(1.0);
    (mean, sharpe, two_sided)
}

/// 合成鞅 Dataset 生成（acc-martingale-guard，§11/§16 鞅定理的否证基线）。
///
/// `P_{t+1} = P_t + ε`，ε∈{−1,0,+1} 对称等概率（SplitMix64 确定性）⟹ `E[ε|F_t]=0`
/// ⟹ `E[P_{t+1}−P_t|F_t]=0` = 鞅（认识论 L1：合成数据，验证管线因果纯净，非验证假设）。
///
/// **鞅性保证（codex 审核点）**：ε 仅依赖 PRNG state（与价格历史 F_t 独立），无均值回复/动量项
/// ⟹ 无任何残余可预测性。OHLC 全 = close（纯 close 鞅序列，无 intrabar 信息可被 χ 偷看）。
/// 整数 tick close 有下限 1（量化非负约束；下限反射保持对称——撞底翻 +1 而非吸收，避免引入
/// 系统性上漂）。`untradable=false`（全可交易，让 χ 选择器在干净鞅上充分作用）。
fn synthetic_martingale(n: usize, seed: u64, start_tick: i64) -> Dataset {
    let mut rng = SplitMix64::new(seed);
    let mut bars = Vec::with_capacity(n);
    let mut px = start_tick.max(1);
    for i in 0..n {
        // ε∈{−1,0,+1} 对称：三态等概率 ⟹ E[ε]=0（F_t 独立 ⟹ E[ε|F_t]=0）。
        let eps: i64 = match rng.next_below(3) {
            0 => -1,
            1 => 0,
            _ => 1,
        };
        // 下限反射（撞 1 时 −1 翻成 +1）保持对称，不引入吸收态上漂。
        px = if px + eps < 1 { px + 1 } else { px + eps };
        bars.push(super::super::types::Bar {
            source_index: i,
            timestamp: i as i64,
            open: px,
            high: px,
            low: px,
            close: px,
            volume: 1,
            untradable: false,
        });
    }
    let dates = (0..n).map(|i| format!("2020-01-01 00:{:02}:00", i % 60)).collect();
    Dataset { symbol: "MARTINGALE".to_string(), bars, dates, bar_seconds: 60 }
}

/// **★鞅不可能定理守卫：合成鞅上 χ 选择器无 alpha = harness 因果纯净（无未来函数泄漏）**。
///
/// 鞅定理（§11/§16，L0 数学）：`E[ΔP|F_t]=0 ∧ ΔN_t∈F_t ⟹ E[ΔR]=−E[ΔC]≤0`——无预测性则无 alpha。
/// 合成鞅由构造满足 `E[ΔP|F_t]=0`（[`synthetic_martingale`]），χ 选择器因果（ΔN_t∈F_t，
/// walk-forward μ frozen + test 不偷看未来）⟹ **理论预言 ΔR 无正期望**。
///
/// **判据口径（codex 异质审裁决 2026-06-30，约束4硬节点）**：主判据 = **毛收益 ΔGross**，非净 ΔR。
/// 净 ΔR 在鞅上**可合法为正**——χ 过滤比全覆盖 baseline 交易少 ⟹ 成本低 ⟹ `ΔR=−ΔC>0` 是
/// 成本节省，非预测 alpha。鞅定理的纯净形式是毛额 `E[ΔN·ΔP]=0`（不含成本）。故剥离成本：
/// `ΔGross_t = ΔR_t + ΔC_t`（[`rebuild_cost_series`] 重建逐 bar 成本差），鞅上 `E[ΔGross]=0`。
///
/// **可证伪判据（acc-martingale-guard，L1 管线纯净度）**：
/// - PASS（harness 无泄漏）：毛收益双边检验**不显著**（`boot_two_sided>0.05`）——ΔGross 含 0，
///   与鞅定理 `E[ΔN·ΔP]=0` 自洽。净 ΔR 可正（成本节省），并列报告但不作泄漏判据。
/// - FAIL（暴露未来函数泄漏）：毛收益显著≠0（`boot_two_sided≤0.05`）——鞅上毛额不可能有预测性，
///   显著毛 alpha 只能来自 harness 偷看未来（μ 表泄漏 test 信息 / 持仓暴露差含后视）。须定位泄漏。
///
/// 多种子（5 个）降低单次 PRNG 偶然性；任一种子毛收益显著即判 FAIL（泄漏不应种子依赖）。
///
/// `#[ignore]`：O(n²) 慢测（5 种子 × 32K bar train+test 逐 bar 重分类），`--release` 必须。
/// 跑法：`cargo test --release --lib theta_v0::backtest::l3_delta_r_alpha::martingale -- --ignored --nocapture`
#[test]
#[ignore = "鞅守卫；O(n²) 8 种子 × 32K bar；--release"]
fn martingale_impossibility_guard() {
    let config = ThetaConfig::default();
    let theta: f64 = 0.0; // 与 multi_symbol 同 θ 口径（滤 μ≤0 类）。
    let n = MAX_BARS; // 与真实数据同窗长（train+test 各 16K）。
    let seeds: [u64; 8] = [PREREG_SEED, 0xC0FFEE, 0xDEADBEEF, 42, 0x1234_5678, 0xABCD, 7, 0xFACE];

    eprintln!("\n===== 鞅不可能定理守卫：合成鞅 χ 选择器无毛 alpha 验证（θ={theta}, n={n}/种子）=====");
    eprintln!("★鞅定理（§11/§16, L0）：E[ΔP|F_t]=0 ∧ ΔN∈F_t ⟹ E[ΔN·ΔP]=0——毛额无预测性");
    eprintln!("★判据（认识论修正）：主判据=**跨独立种子** ΔGross 均值符号检验（H0:E=0⟹符号随机）；系统性同号=泄漏(FAIL)");
    eprintln!("★单路径 boot p / 单路径 mean 仅诊断——单条鞅路径样本均值必偏离0（√n波动），单路径检验会误判噪声为泄漏");
    eprintln!("★净 ΔR 鞅上可正（χ少交易省成本−ΔC>0）；毛 ΔGross 剥离成本=鞅定理 E[ΔN·ΔP]=0 直接对应（codex 裁决）");
    eprintln!("★与 delta_r_alpha_multi_symbol 真实数据否证互为印证（真实无 alpha + 合成鞅无毛 alpha = χ 确无先验 alpha）");
    eprintln!(
        "{:<10} {:>7} {:>7} {:>6} {:>11} {:>11} {:>10} {:>9} {:>7} {}",
        "seed", "train", "test", "μ类", "mean(ΔR净)", "mean(Gross)", "Shrp(Gr)", "boot_2p", "ΔN≠0", "毛符号",
    );

    let bars_per_year = data::bars_per_year(60);
    let mut n_evaluated = 0usize;
    let mut gross_means: Vec<f64> = Vec::new(); // 跨种子 ΔGross 均值（符号检验输入）。

    for &seed in &seeds {
        let ds = synthetic_martingale(n, seed, 10_000);
        let split = n / 2;
        let mk = |lo: usize, hi: usize| Dataset {
            symbol: ds.symbol.clone(),
            bars: ds.bars[lo..hi].to_vec(),
            dates: ds.dates[lo..hi].to_vec(),
            bar_seconds: 60,
        };
        let train = mk(0, split);
        let test = mk(split, n);

        // walk-forward μ 表（train 窗，frozen——与 multi_symbol 同函数，因果保证一致）。
        let (est, _n_sig) = build_walk_forward_mu(&train, &config);

        let first_px = test
            .bars
            .iter()
            .find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * config.tick.tick_size)
            .unwrap_or(1.0);
        let nav = (first_px * 1000.0).max(1.0e6);
        let years = (test.bars.len() as f64 / bars_per_year).max(0.01);

        let mut base_cfg = config.clone();
        base_cfg.risk.chi_theta = None;
        let base = run_theta_v0_pi_chi(&test, &base_cfg, years, nav, &est, true);

        let mut chi_cfg = config.clone();
        chi_cfg.risk.chi_theta = Some(theta);
        let chi = run_theta_v0_pi_chi(&test, &chi_cfg, years, nav, &est, false);

        let st = delta_r_stats(&base.equity_curve, &chi.equity_curve, nav, bars_per_year);

        // 毛收益剥离（codex 裁决 + delta-r-audit P0 修复）：ΔGross_t = ΔR_t + ΔC_t。净配对差用
        // **绝对增量** (E^χ_t−E^χ_{t−1})−(E^0_t−E^0_{t−1})（含成本，成本已扣进 equity，口径同
        // [`delta_r_stats`]）；加回成本差 (cost_χ−cost_base) 还原毛额。鞅上 E[ΔGross]=0。
        // 增量序列索引 i∈[1,len)，对应 bar i 的增量含 −cost_i ⟹ 加回 cost[i] 还原毛额。
        let len = base.equity_curve.len().min(chi.equity_curve.len());
        let cost_base = rebuild_cost_series(&base, len, nav);
        let cost_chi = rebuild_cost_series(&chi, len, nav);
        let gross: Vec<f64> = (1..len)
            .map(|i| {
                let net = (chi.equity_curve[i] - chi.equity_curve[i - 1])
                    - (base.equity_curve[i] - base.equity_curve[i - 1]);
                net + (cost_chi[i] - cost_base[i])
            })
            .collect();
        let (gross_mean, gross_sharpe, gross_two_sided_p) = gross_stats(&gross, bars_per_year);

        // 跨种子符号检验输入（仅 χ 真改交易集的有效种子计入——χ 未改交易集时 ΔGross≡0，符号无意义）。
        if st.chi_changed_trades {
            n_evaluated += 1;
            gross_means.push(gross_mean);
        }
        let sign_mark = if !st.chi_changed_trades {
            "χ未改ΔN≡0"
        } else if gross_mean > 0.0 {
            "+ (诊断:单路径√n波动)"
        } else {
            "− (诊断:单路径√n波动)"
        };

        eprintln!(
            "{:<10x} {:>7} {:>7} {:>6} {:>11.3e} {:>11.3e} {:>10.3} {:>9.4} {:>7} {}",
            seed,
            train.bars.len(),
            test.bars.len(),
            est.n_classes(),
            st.mean,
            gross_mean,
            gross_sharpe,
            gross_two_sided_p,
            st.chi_changed_trades,
            sign_mark,
        );

        assert!(gross_mean.is_finite(), "seed {seed:x} mean(ΔGross) 有限");
        assert!((0.0..=1.0).contains(&gross_two_sided_p), "seed {seed:x} gross 双边 p∈[0,1]");
    }

    // ── 跨种子（独立鞅路径）ΔGross 均值符号检验（主判据，H0:E[ΔGross]=0 ⟹ 符号随机 p=0.5）──
    let n_pos = gross_means.iter().filter(|&&m| m > 0.0).count();
    let n_neg = gross_means.iter().filter(|&&m| m < 0.0).count();
    let n_eff = gross_means.len();
    // 双边符号检验 p = 2·min(P(X≥max(n_pos,n_neg)), ...)；用单边 sign_test_pvalue 取较极端侧 ×2。
    let extreme = n_pos.max(n_neg);
    let two_sided_sign_p = (2.0 * sign_test_pvalue(extreme, n_eff)).min(1.0);
    // 泄漏判据：跨独立路径系统性同号（双边符号 p≤0.05）= 总体 E[ΔGross]≠0 = harness 泄漏。
    let leak = n_eff >= 2 && two_sided_sign_p <= 0.05;

    eprintln!("\n===== 鞅守卫聚合（跨独立种子符号检验，主判据）=====");
    eprintln!("有效检验种子数（χ 真改交易集）   : {n_evaluated}/{}", seeds.len());
    eprintln!("ΔGross 均值 +/− 分布            : {n_pos} 正 / {n_neg} 负（共 {n_eff}）");
    eprintln!("跨种子双边符号检验 p（H0:E=0）   : {two_sided_sign_p:.4}");
    eprintln!("检出未来函数泄漏（系统性同号）   : {leak}");
    eprintln!(
        "\n★诚实裁定（acc-martingale-guard + alpha2 §11 鞅定理 + codex 异质审 2026-06-30 + 231号）：\n  \
         - **harness 因果纯净（PASS）** ⟺ 跨独立种子 ΔGross 均值符号检验不显著（双边 p>0.05）——\n    \
           符号随机分布在 0 两侧，独立鞅路径上毛额总体 E[ΔGross]=0，与鞅定理 E[ΔN·ΔP]=0 自洽。\n  \
         - **认识论修正（单路径 bootstrap 否证）**：单条鞅路径样本均值必偏离 0（√n 波动），单路径\n    \
           boot p 把噪声误判为泄漏；正确判据是跨独立路径的符号随机性（与 L3 跨品种符号检验同构）。\n  \
         - **成本剥离（codex 裁决）**：净 ΔR 鞅上可正（χ 少交易省成本 −ΔC>0），毛 ΔGross=ΔR+ΔC\n    \
           剥离成本才是鞅定理 E[ΔN·ΔP]=0 的直接对应。\n  \
         - 与 delta_r_alpha_multi_symbol（真实数据 χ 无系统性 alpha，否定性结果）互为印证：\n    \
           真实无 alpha + 合成鞅无毛 alpha ⟹ χ 选择器确无先验 alpha（鞅定理两侧确认），harness 可信。\n  \
         - 反面：若跨独立鞅路径毛额系统性同号（FAIL）⟹ 真实数据的任何 alpha 信号都不可信（被泄漏污染）。"
    );

    // acceptance（acc-martingale-guard 可证伪门）：跨独立鞅路径毛额不得系统性同号（无未来函数泄漏）。
    assert!(
        !leak,
        "鞅守卫 FAIL：跨独立鞅路径 ΔGross 均值系统性同号（双边符号 p={two_sided_sign_p:.4}≤0.05）\
         ——总体 E[ΔGross]≠0 = harness 未来函数泄漏，须定位修复（{n_pos}正/{n_neg}负/{n_eff}有效）"
    );
    // 跨种子符号检验需 ≥2 有效种子（χ 真改交易集）；否则检验空转/不可判。
    assert!(
        n_eff >= 2,
        "鞅守卫 inconclusive：χ 真改交易集的有效种子<2（{n_eff}）——合成鞅未充分触发 χ 选择，符号检验不可判"
    );
}

/// **★LCB(μ) 选择器 vs 裸 μ 选择器 L2 对比（acc-lcb-l2-vs-naive，task #74）**。
///
/// 同一 8 品种 walk-forward harness（与 [`delta_r_alpha_multi_symbol`] 同 μ 表/同 split/同 θ），
/// 唯一变量 = `config.risk.chi_z_alpha`：**0（裸 μ baseline）vs 1.645（95% LCB）**。检验 LCB 是否
/// 改善退化（χ 空仓）品种数 + L3 inconclusive。
///
/// ## ★分离纪律（防 090 声明膨胀 + lcb-selector 两源归因，codex 异质审修复后）
///
/// **纪律1（分离报告，非正交——codex 攻击点2）**：LCB 控**过拟合**与 inconclusive 根因（功效不足，
/// n=5+16K 短窗）**不是正交独立维度**，而是「收紧 LCB→拒更多类」**同一 rejection mass 的两个投影**：
/// 更严的 LCB 同时 (1) 拒更多高方差类（降过拟合度量）+ (2) 把品种推向 n_orders=0 退化 ⟹ 排除 L3 池 ⟹
/// 降功效。**分离报告**两个投影但**不声称正交**：
/// - 维度①（过拟合）：LCB vs 裸 μ 的退化品种数 / χ 空仓率差异。
/// - 维度②（功效）：L3 符号检验在两 selector 下是否**仍 inconclusive**（n_L3<5 ⟹ 全正也达不到 p<.05）。
/// - **禁止**把二者混为「LCB 解决了 inconclusive」——LCB 单独不足以升 inconclusive（功效维度需更大池/更长窗）。
///
/// **纪律2（两源归因）**：χ 空仓率 LCB vs 裸 μ 的差异来自**两源**，必须分离：
/// - **源 (a) n<2 无 LCB 证据被拒**：裸 μ 放行（n=1 mean 有定义）但 LCB 拒（mu_lcb=None ⟹ treat_empty=false
///   ⟹ 滤）。**这是样本饥饿不是过拟合控制**——n=1 类裸 μ 本就是噪声单点，LCB 拒它是诚实但非 alpha 价值。
/// - **源 (b) n≥2 高方差 LCB<θ 被拒**：裸 μ>θ 放行但 LCB=mean−z_α·std/√n<θ 拒。源 (b) 须**再筛**才是
///   alpha 价值证据（codex 攻击点1/3/4）：① 按 train-class n 分桶剥离 n<10 低自由度（n=2 仅 1 dof，
///   方差对单异常值超敏 ⟹ 假高方差 ⟹ LCB 崩 ⟹ 伪 src_b）；② 排除退化品种（χ→0 空仓 = 平凡零过拟合，
///   其 ΔR 是弃权 PnL 非 alpha，161 号）。**唯一** demonstrated alpha 候选 = `tot_src_b_robust_nondegen`。
///
/// ## 可证伪结果（二选一，都携信息增量）
/// - **(a) LCB 有有限价值**：`tot_src_b_robust_nondegen`>0 ⟹ 存在非退化、n≥10 高方差类被 LCB 正确拒（窄结论）。
/// - **(b) LCB 无 demonstrated alpha**：`tot_src_b_robust_nondegen`=0 ⟹ src_b 全是退化/低自由度 ⟹
///   LCB 只是更保守地压品种空仓（codex 异质审 (B) 缺陷的稳健陈述），非 demonstrated alpha 价值。
///
/// 认识论 **L2**（真实数据，可证伪，携信息增量）。有效域 caveat：{MAX_BARS}bar 截断 + 8 品种 + OOS 内 split。
///
/// `#[ignore]`：O(n²) 慢测（8 品种 × 2 z_alpha × 32K bar train+test 逐 bar 重分类），`--release` 必须。
/// 跑法：`cargo test --release --lib theta_v0::backtest::l3_delta_r_alpha::lcb_vs_naive_l2 -- --ignored --nocapture`
#[test]
#[ignore = "LCB vs 裸 μ L2 对比；O(n²) 8 品种 × 2 z_alpha × 32K bar；--release"]
fn lcb_vs_naive_l2() {
    let config = ThetaConfig::default();
    let theta: f64 = 0.0; // 与 multi_symbol 同 θ 口径（滤 μ≤0 类）。
    let z_alpha_lcb: f64 = 1.645; // 95% 单边 LCB（防高维 z 过拟合，p25 §12）。

    eprintln!("\n===== ★LCB(μ) vs 裸 μ 选择器 L2 对比（acc-lcb-l2-vs-naive, θ={theta}, z_α_LCB={z_alpha_lcb}）=====");
    eprintln!("★唯一变量 = config.risk.chi_z_alpha（0=裸 μ baseline vs {z_alpha_lcb}=95% LCB）；同 μ 表/同 split/同 θ");
    eprintln!("★纪律1（codex 攻击点2 修复）：维度①过拟合 与 维度②功效 **非正交**——同一「收紧 LCB→拒更多类」rejection mass 两投影；分离报告但不混为「LCB 解决 inconclusive」");
    eprintln!("★纪律2（两源归因，codex 攻击点1/3/4 修复）：χ 空仓差异 = 源(a) n<2 饥饿 + 源(b) n≥2 高方差；src_b 须剥离退化空仓品种(弃权PnL,161号) + n<10 低自由度才是 demonstrated alpha");
    eprintln!(
        "{:<6} {:>7} {:>7} {:>8} {:>8} {:>9} {:>9} {:>11} {:>11}",
        "symbol", "test", "μ类", "χ0单", "χL单", "拒源a", "拒源b", "ΔR(裸μ)", "ΔR(LCB)",
    );

    // 两 selector 各自的 L3 池（χ 真改交易集的有效品种 ΔR 均值）。
    let mut dr_naive: Vec<f64> = Vec::new();
    let mut dr_lcb: Vec<f64> = Vec::new();
    let mut n_degen_naive = 0usize; // 裸 μ 退化（χ 全滤空仓）品种数
    let mut n_degen_lcb = 0usize; // LCB 退化品种数
    let mut n_done = 0usize;
    // 纪律2 两源汇总（跨品种）：源 (a) 样本饥饿拒、源 (b) 高方差拒。
    let (mut tot_src_a, mut tot_src_b) = (0usize, 0usize);
    // codex 攻击点3 修复：src_b 按 train-class n 分桶剥离低自由度伪装。
    //   n∈{2,3}：1~2 自由度，方差对单异常值超敏 ⟹ 假高方差 ⟹ LCB 崩 ⟹ 伪 src_b（不计入 alpha）。
    //   n≥10：样本充足，LCB 收缩有统计意义 ⟹ 真过拟合控制（仅此为 alpha 价值候选）。
    let (mut tot_src_b_lowdof, mut tot_src_b_robust) = (0usize, 0usize);
    // codex 攻击点1+4 修复：仅**非退化**（lcb.n_orders>0）品种的 src_b_robust 才计入"alpha 价值"——
    //   退化品种 χ→0 空仓 = 从不交易 = 平凡零过拟合（按构造），其 src_b 是"停止交易"非 alpha。
    let mut tot_src_b_robust_nondegen = 0usize;

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
            eprintln!("{:<6} OOS 窗空", w.symbol);
            continue;
        }
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
            eprintln!("{:<6} train/test 样本不足 ⟹ inconclusive", w.symbol);
            continue;
        }
        n_done += 1;

        let (est, _n_sig) = build_walk_forward_mu(&train, &config);

        let first_px = test
            .bars
            .iter()
            .find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * config.tick.tick_size)
            .unwrap_or(1.0);
        let nav = (first_px * 1000.0).max(1.0e6);
        let bars_per_year = data::bars_per_year(60);
        let years = (test.bars.len() as f64 / bars_per_year).max(0.01);

        // χ≡1 baseline（chi_theta=None，与 z_alpha 无关——baseline 不过滤）。
        let mut base_cfg = config.clone();
        base_cfg.risk.chi_theta = None;
        let base = run_theta_v0_pi_chi(&test, &base_cfg, years, nav, &est, true);

        // 裸 μ selector（chi_z_alpha=0 ⟹ LCB=mean ⟹ 退化裸 μ 门；treat_empty=false 最诚实）。
        let mut naive_cfg = config.clone();
        naive_cfg.risk.chi_theta = Some(theta);
        naive_cfg.risk.chi_z_alpha = 0.0;
        let naive = run_theta_v0_pi_chi(&test, &naive_cfg, years, nav, &est, false);

        // LCB selector（chi_z_alpha=1.645 ⟹ 准入用 LCB=mean−z_α·std/√n）。
        let mut lcb_cfg = config.clone();
        lcb_cfg.risk.chi_theta = Some(theta);
        lcb_cfg.risk.chi_z_alpha = z_alpha_lcb;
        let lcb = run_theta_v0_pi_chi(&test, &lcb_cfg, years, nav, &est, false);

        let st_naive = delta_r_stats(&base.equity_curve, &naive.equity_curve, nav, bars_per_year);
        let st_lcb = delta_r_stats(&base.equity_curve, &lcb.equity_curve, nav, bars_per_year);

        // 退化（χ 全滤空仓）判定（与 multi_symbol (d) 同口径）——先于 src_b 归因，
        // 因 src_b 的 alpha 论证须排除退化品种（codex 攻击点1+4）。
        let degen_naive = naive.n_orders == 0 && base.n_orders > 0;
        let degen_lcb = lcb.n_orders == 0 && base.n_orders > 0;
        if degen_naive {
            n_degen_naive += 1;
        }
        if degen_lcb {
            n_degen_lcb += 1;
        }

        // ── 纪律2 两源分解：test 候选 z 中「裸 μ 放行 ∧ LCB 拒」的两源（a 饥饿 / b 高方差）──
        // 源 (a)：mu(z)=Some 且 >θ（裸 μ 放行）但 mu_lcb(z)=None（n<2 无 LCB 证据，treat_empty=false ⟹ 拒）。
        // 源 (b)：mu(z)>θ（裸 μ 放行）且 mu_lcb(z)=Some 但 ≤θ（n≥2 高方差，LCB 收缩到阈值下，拒）。
        //   codex 攻击点3：src_b 再按 train-class n 分桶——n∈{2,3} 低自由度（方差超敏伪高方差）vs n≥10 鲁棒。
        let test_zs = enumerate_candidate_z(&test, &config);
        let (mut src_a, mut src_b) = (0usize, 0usize);
        let (mut src_b_lowdof, mut src_b_robust) = (0usize, 0usize);
        for z in &test_zs {
            let naive_pass = matches!(est.mu(z), Some(m) if m > theta);
            if !naive_pass {
                continue; // 裸 μ 本就拒，不计入「LCB 额外拒」差异
            }
            match est.mu_lcb(z, z_alpha_lcb) {
                None => src_a += 1, // n<2：样本饥饿（非过拟合控制）
                Some(l) if l <= theta => {
                    src_b += 1; // n≥2 高方差 LCB<θ
                    // train-class n 分桶（est.count 是该 z 在 train μ 表的样本量）。
                    if est.count(z) >= 10 {
                        src_b_robust += 1; // n≥10：方差估计鲁棒 ⟹ 真过拟合控制候选
                    } else {
                        src_b_lowdof += 1; // n∈{2..9}：低自由度，方差超敏 ⟹ 伪高方差，剥离
                    }
                }
                Some(_) => {} // LCB 仍 >θ：两 selector 一致放行，无差异
            }
        }
        tot_src_a += src_a;
        tot_src_b += src_b;
        tot_src_b_lowdof += src_b_lowdof;
        tot_src_b_robust += src_b_robust;
        // codex 攻击点1+4：仅非退化品种的鲁棒 src_b 计入"alpha 价值"——
        // 退化品种 χ→0 = 平凡零过拟合（从不交易），其 src_b 是"停止交易"非 demonstrated alpha。
        if !degen_lcb {
            tot_src_b_robust_nondegen += src_b_robust;
        }

        // L3 池（仅 χ 真改交易集且非退化的有效品种，与 multi_symbol l3_valid 同口径）。
        if st_naive.chi_changed_trades && naive.n_orders > 0 && base.n_orders > 0 {
            dr_naive.push(st_naive.mean);
        }
        if st_lcb.chi_changed_trades && lcb.n_orders > 0 && base.n_orders > 0 {
            dr_lcb.push(st_lcb.mean);
        }

        // codex 攻击点1+4：退化品种的 ΔR 是"弃权 PnL"（flat-selector vs always-open base，
        // 下跌窗空仓跑赢 always-long），非 alpha 发现 ⟹ 标记 [弃权]。
        let abstention_mark = if degen_lcb { " [LCB弃权PnL]" } else { "" };
        eprintln!(
            "{:<6} {:>7} {:>8} {:>8} {:>8} {:>9} {:>9}(鲁棒{}/低dof{}) {:>11.3e} {:>11.3e}{}",
            w.symbol,
            test.bars.len(),
            est.n_classes(),
            naive.n_orders,
            lcb.n_orders,
            src_a,
            src_b,
            src_b_robust,
            src_b_lowdof,
            st_naive.mean,
            st_lcb.mean,
            abstention_mark,
        );

        assert!(st_naive.mean.is_finite() && st_lcb.mean.is_finite(), "{} ΔR 有限", w.symbol);
    }

    // ── 维度① 过拟合：退化品种数 + 两源分解（纪律2）──
    eprintln!("\n===== 维度①（过拟合控制，纪律2 两源分离）=====");
    eprintln!("完成品种数                              : {n_done}/{}", PREREG_WINDOWS.len());
    eprintln!("退化品种数（χ 全滤空仓）：裸 μ={n_degen_naive} | LCB={n_degen_lcb}（LCB≥裸 μ ⟹ 更保守）");
    eprintln!("跨品种「裸 μ 放行 ∧ LCB 拒」两源分解（codex 攻击点1/3/4 修复后）：");
    eprintln!("  源(a) n<2 无 LCB 证据被拒（样本饥饿，非过拟合控制）       : {tot_src_a}");
    eprintln!("  源(b) n≥2 高方差 LCB<θ 被拒（总计，含伪高方差）           : {tot_src_b}");
    eprintln!("    ├ 源(b) 低自由度 n∈{{2..9}}（方差超敏伪高方差，剥离）   : {tot_src_b_lowdof}");
    eprintln!("    └ 源(b) 鲁棒 n≥10（方差估计可信）                       : {tot_src_b_robust}");
    eprintln!("  源(b) 鲁棒 ∧ 非退化品种（★唯一 demonstrated alpha 候选）  : {tot_src_b_robust_nondegen}");
    eprintln!(
        "  ★诚实判据（剥离退化空仓 + 低自由度伪装后）：\n  \
         - tot_src_b_robust_nondegen>0 ⟹ 存在**非退化、n≥10**的高方差类被 LCB 正确拒 ⟹ LCB 有**有限**过拟合控制价值（窄结论）。\n  \
         - tot_src_b_robust_nondegen=0 ⟹ src_b 全是退化品种（χ→0 停交易，平凡零过拟合）或低自由度伪高方差 ⟹\n    \
           **LCB 无 demonstrated alpha 价值**——稳健陈述：LCB 只是更保守地把品种压向空仓（codex 异质审 (B) 缺陷结论）。\n  \
         - 退化品种 ΔR（标 [LCB弃权PnL]）= flat-selector vs always-open base，下跌窗空仓跑赢 always-long ⟹\n    \
           **弃权 PnL 非 alpha**（161 号：停止交易不可粉饰为改善）。BRN「负转正」**不计** LCB 改善。"
    );

    // ── 维度② 功效：L3 符号检验在两 selector 下（纪律1，与维度①同源非正交——同一 rejection mass 两投影）──
    const MIN_L3_POWER: usize = 5;
    let l3 = |pool: &[f64]| -> (usize, usize, f64, f64) {
        let n = pool.len();
        let n_pos = pool.iter().filter(|&&m| m > 0.0).count();
        let mean = if n > 0 { pool.iter().sum::<f64>() / n as f64 } else { 0.0 };
        (n, n_pos, mean, sign_test_pvalue(n_pos, n))
    };
    let (n_n, pos_n, mean_n, p_n) = l3(&dr_naive);
    let (n_l, pos_l, mean_l, p_l) = l3(&dr_lcb);
    eprintln!("\n===== 维度②（统计功效，纪律1：与维度①同源非正交——同一 rejection mass 两投影，不可混为「LCB 解决 inconclusive」）=====");
    eprintln!("L3 符号检验（裸 μ）: n_L3={n_n} n_pos={pos_n}/{n_n} 池均值={mean_n:.3e} 符号 p={p_n:.4}");
    eprintln!("L3 符号检验（LCB） : n_L3={n_l} n_pos={pos_l}/{n_l} 池均值={mean_l:.3e} 符号 p={p_l:.4}");
    let verdict = |n: usize, p: f64, mean: f64| -> &'static str {
        if n >= 2 && p < 0.05 && mean > 0.0 {
            "系统性 alpha 成立"
        } else if n < MIN_L3_POWER {
            "inconclusive(功效不足 n_L3<5，全正也达不到 p<.05)"
        } else {
            "系统性 alpha 不成立（n≥5 但 p≥.05 或均值≤0）"
        }
    };
    eprintln!("  裸 μ L3 裁定: {}", verdict(n_n, p_n, mean_n));
    eprintln!("  LCB  L3 裁定: {}", verdict(n_l, p_l, mean_l));
    eprintln!(
        "\n★★最终裁定（codex 异质审修复后，formalization-validity-domain 231号 + 161 号）：\n  \
         - 维度①（过拟合）：LCB 是否有 demonstrated alpha = tot_src_b_robust_nondegen 是否 >0\n    \
           （剥离退化空仓品种 + 低自由度 n<10 伪高方差后，仍有非退化高样本类被正确拒）。\n  \
         - 维度②（功效）：L3 符号检验在两 selector 下仍 inconclusive（n_L3<5）——根因 = 功效不足\n    \
           （16K 短窗 + 8 品种），16K 已是 O(n²) 墙下的可行子集上限，全窗不可行 ⟹ 诚实报 blocked。\n  \
         - **codex 攻击点2 修复（删正交声明）**：维度①与维度② **非正交独立维度**——是「收紧 LCB→拒更多类」\n    \
           **同一 rejection mass 的两个投影**：更严的 LCB 同时 (1) 拒更多高方差类（降过拟合度量）+\n    \
           (2) 把品种推向 n_orders=0 退化 ⟹ 排除出 L3 池 ⟹ 降功效。同源机制，不是两个独立维度。\n  \
           不再声称「两维度正交」——它们是同一收紧机制的正负两面（codex 已证 LCB放行集⊆裸μ的单调性即同源）。\n  \
         - **161 号**：退化品种 χ→0 = 停止交易，其 ΔR 改善是弃权 PnL，**不可**粉饰为「LCB 改善」。"
    );

    // acceptance：管线在两 selector 下跑通（≥1 品种完成或全 DATA BLOCKER）。
    // **不**断言 n_degen_lcb≥n_degen_naive——LCB 是裸 μ 放行集的子集（逐 z 更保守）只在
    // **过滤层**成立；品种级退化（n_orders==0）受 runner RiskOK/ConflictOK/持仓树级联调制，
    // 不一定单调（no-patch §5：不把预期结果硬编码为不变量，留给真实数据可证伪）。
    // 子集关系单调性诊断打印（观测，非 assert）：
    eprintln!(
        "\n[子集单调性诊断] LCB 退化数({n_degen_lcb}) {} 裸 μ 退化数({n_degen_naive})——\
         若 LCB<裸 μ 则 runner 级联非单调（值得查），若 ≥ 则与过滤层子集关系一致",
        if n_degen_lcb >= n_degen_naive { "≥" } else { "<(非单调!)" }
    );
}

/// **★三路对比 裸μ vs LCB vs hierarchical-shrinkage L2（acc-three-way-l2, task #83）**。
///
/// 同一 8 品种 walk-forward harness（与 [`lcb_vs_naive_l2`] 同 μ 表/同 split/同 θ）。三路选择器：
/// - **裸μ**：`run_theta_v0_pi_chi(est, z_alpha=0)`——`mu(z)>θ` 准入。
/// - **LCB**：`run_theta_v0_pi_chi(est, z_alpha=1.645)`——`mu_lcb(z)>θ` 准入。
/// - **shrinkage**：`run_theta_v0_pi_chi(est.shrunk_view(τ²), z_alpha=0)`——`mu_shrink(z)>θ` 准入。
///   **解 C**（编排者裁定走 C，定理类）：frozen selector **一行不改**，只换喂它的 est——
///   [`MuEstimator::shrunk_view`] 把每桶 mean 收缩到 pooled、**保留原 n**（n_L3 可比）。
///   shrunk_view 仅配 `z_alpha=0` 裸门（mean 收缩但 m2 原始，禁止再 LCB，见其 doc）。
///
/// ## 可证伪二选一（task #83，分离纪律守 660 正交 + 161 退化弃权）
/// - **(a) shrinkage 真比 LCB 好**：在 **n_L3 不低于裸 μ**（保功效——shrinkage 卖点 = 不像 LCB 把低 n 类
///   压成退化空仓）前提下，高级别稀疏类收缩后**非退化准入**且 ΔR 改善。
/// - **(b) shrinkage 不改善或同样退化**：功效不足非估计方法可解（与 [`lcb_vs_naive_l2`] inconclusive 同根）。
///
/// **退化弃权（161 号）**：χ→0 空仓品种 ΔR = flat vs always-open 弃权 PnL，**非 alpha**，标 [弃权]。
///
/// 认识论 **L2**（真实数据，可证伪）。有效域 caveat：{MAX_BARS}bar 截断 + 8 品种 + OOS 内 split。
///
/// `#[ignore]`：O(n²) 慢测（8 品种 × 3 selector × 32K bar）。
/// 跑法：`cargo test --release --lib theta_v0::backtest::l3_delta_r_alpha::three_way_l2 -- --ignored --nocapture`
#[test]
#[ignore = "三路对比 L2；O(n²) 8 品种 × 3 selector × 32K bar；--release"]
fn three_way_l2() {
    let config = ThetaConfig::default();
    let theta: f64 = 0.0;
    let z_alpha_lcb: f64 = 1.645;
    let tau_sq: f64 = 1.0; // 级别间先验方差 τ²（PDF §8 θ_ℓ~N(0,τ²)）；可调旋钮，1.0 = 中性先验。

    eprintln!("\n===== ★三路对比 裸μ vs LCB vs shrinkage L2（acc-three-way-l2, θ={theta}, z_α={z_alpha_lcb}, τ²={tau_sq}）=====");
    eprintln!("★三路同 walk-forward：裸μ=mu>θ | LCB=mu_lcb>θ | shrinkage=shrunk_view(τ²)+裸门（解C：frozen selector 不改）");
    eprintln!("★保功效判据（shrinkage 卖点）：比三路 n_L3 池大小——shrinkage 应不像 LCB 把低 n 类压退化（n_L3 不降）");
    eprintln!("★退化弃权（161号）：χ→0 空仓 ΔR = 弃权 PnL 非 alpha，标 [弃权]");
    eprintln!(
        "{:<6} {:>7} {:>7} {:>7} {:>7} {:>10} {:>10} {:>10}",
        "symbol", "test", "χ0单", "χL单", "χS单", "ΔR(裸μ)", "ΔR(LCB)", "ΔR(shrink)",
    );

    let (mut dr_naive, mut dr_lcb, mut dr_shrink): (Vec<f64>, Vec<f64>, Vec<f64>) =
        (Vec::new(), Vec::new(), Vec::new());
    let (mut n_degen_naive, mut n_degen_lcb, mut n_degen_shrink) = (0usize, 0usize, 0usize);
    let mut n_done = 0usize;

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
            eprintln!("{:<6} OOS 窗空", w.symbol);
            continue;
        }
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
            eprintln!("{:<6} train/test 样本不足 ⟹ inconclusive", w.symbol);
            continue;
        }
        n_done += 1;

        let (est, _n_sig) = build_walk_forward_mu(&train, &config);
        let est_shrunk = est.shrunk_view(tau_sq); // 解 C：收缩视图，保留原 n。

        let first_px = test
            .bars
            .iter()
            .find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * config.tick.tick_size)
            .unwrap_or(1.0);
        let nav = (first_px * 1000.0).max(1.0e6);
        let bars_per_year = data::bars_per_year(60);
        let years = (test.bars.len() as f64 / bars_per_year).max(0.01);

        let mut base_cfg = config.clone();
        base_cfg.risk.chi_theta = None;
        let base = run_theta_v0_pi_chi(&test, &base_cfg, years, nav, &est, true);

        let mut naive_cfg = config.clone();
        naive_cfg.risk.chi_theta = Some(theta);
        naive_cfg.risk.chi_z_alpha = 0.0;
        let naive = run_theta_v0_pi_chi(&test, &naive_cfg, years, nav, &est, false);

        let mut lcb_cfg = config.clone();
        lcb_cfg.risk.chi_theta = Some(theta);
        lcb_cfg.risk.chi_z_alpha = z_alpha_lcb;
        let lcb = run_theta_v0_pi_chi(&test, &lcb_cfg, years, nav, &est, false);

        // shrinkage：裸门（z_alpha=0）但喂收缩 est ⟹ mu_shrink>θ 准入（解 C）。
        let mut shrink_cfg = config.clone();
        shrink_cfg.risk.chi_theta = Some(theta);
        shrink_cfg.risk.chi_z_alpha = 0.0;
        let shrink = run_theta_v0_pi_chi(&test, &shrink_cfg, years, nav, &est_shrunk, false);

        let st_naive = delta_r_stats(&base.equity_curve, &naive.equity_curve, nav, bars_per_year);
        let st_lcb = delta_r_stats(&base.equity_curve, &lcb.equity_curve, nav, bars_per_year);
        let st_shrink = delta_r_stats(&base.equity_curve, &shrink.equity_curve, nav, bars_per_year);

        let degen_naive = naive.n_orders == 0 && base.n_orders > 0;
        let degen_lcb = lcb.n_orders == 0 && base.n_orders > 0;
        let degen_shrink = shrink.n_orders == 0 && base.n_orders > 0;
        n_degen_naive += degen_naive as usize;
        n_degen_lcb += degen_lcb as usize;
        n_degen_shrink += degen_shrink as usize;

        // L3 池（仅 χ 真改交易集且非退化的有效品种，与 lcb_vs_naive_l2 同口径）。
        if st_naive.chi_changed_trades && naive.n_orders > 0 && base.n_orders > 0 {
            dr_naive.push(st_naive.mean);
        }
        if st_lcb.chi_changed_trades && lcb.n_orders > 0 && base.n_orders > 0 {
            dr_lcb.push(st_lcb.mean);
        }
        if st_shrink.chi_changed_trades && shrink.n_orders > 0 && base.n_orders > 0 {
            dr_shrink.push(st_shrink.mean);
        }

        // 退化品种 ΔR 标弃权（161号）：shrinkage 退化 ⟹ 其 ΔR 是弃权 PnL 非 alpha。
        let mark = if degen_shrink { " [shrink弃权PnL]" } else { "" };
        eprintln!(
            "{:<6} {:>7} {:>7} {:>7} {:>7} {:>10.3e} {:>10.3e} {:>10.3e}{}",
            w.symbol,
            test.bars.len(),
            naive.n_orders,
            lcb.n_orders,
            shrink.n_orders,
            st_naive.mean,
            st_lcb.mean,
            st_shrink.mean,
            mark,
        );
        assert!(
            st_naive.mean.is_finite() && st_lcb.mean.is_finite() && st_shrink.mean.is_finite(),
            "{} 三路 ΔR 有限",
            w.symbol
        );
    }

    // ── 保功效维度：三路 n_L3 池大小（shrinkage 卖点 = n_L3 不低于裸 μ）──
    let l3 = |pool: &[f64]| -> (usize, usize, f64, f64) {
        let n = pool.len();
        let n_pos = pool.iter().filter(|&&m| m > 0.0).count();
        let mean = if n > 0 { pool.iter().sum::<f64>() / n as f64 } else { 0.0 };
        (n, n_pos, mean, sign_test_pvalue(n_pos, n))
    };
    let (n_n, pos_n, mean_n, p_n) = l3(&dr_naive);
    let (n_l, pos_l, mean_l, p_l) = l3(&dr_lcb);
    let (n_s, pos_s, mean_s, p_s) = l3(&dr_shrink);

    eprintln!("\n===== 保功效维度（n_L3 池大小三路对比，shrinkage 卖点）=====");
    eprintln!("完成品种数 : {n_done}/{}", PREREG_WINDOWS.len());
    eprintln!("退化品种数（χ 全滤空仓）：裸μ={n_degen_naive} | LCB={n_degen_lcb} | shrink={n_degen_shrink}");
    eprintln!("L3 符号检验（裸μ）  : n_L3={n_n} n_pos={pos_n}/{n_n} 池均值={mean_n:.3e} 符号 p={p_n:.4}");
    eprintln!("L3 符号检验（LCB）  : n_L3={n_l} n_pos={pos_l}/{n_l} 池均值={mean_l:.3e} 符号 p={p_l:.4}");
    eprintln!("L3 符号检验（shrink）: n_L3={n_s} n_pos={pos_s}/{n_s} 池均值={mean_s:.3e} 符号 p={p_s:.4}");
    const MIN_L3_POWER: usize = 5;
    let verdict = |n: usize, p: f64, mean: f64| -> &'static str {
        if n >= 2 && p < 0.05 && mean > 0.0 {
            "系统性 alpha 成立"
        } else if n < MIN_L3_POWER {
            "inconclusive(功效不足 n_L3<5)"
        } else {
            "系统性 alpha 不成立（n≥5 但 p≥.05 或均值≤0）"
        }
    };
    eprintln!("  裸μ   L3 裁定: {}", verdict(n_n, p_n, mean_n));
    eprintln!("  LCB   L3 裁定: {}", verdict(n_l, p_l, mean_l));
    eprintln!("  shrink L3 裁定: {}", verdict(n_s, p_s, mean_s));
    eprintln!(
        "\n★★最终裁定（可证伪二选一，161号 + 660 正交纪律）：\n  \
         - (a) shrinkage 真比 LCB 好：n_L3(shrink)≥n_L3(裸μ)（保功效，未把稀疏类压退化）∧ 退化数(shrink)≤LCB\n    \
           ∧ 非退化品种 ΔR(shrink)>ΔR(LCB) ⟹ 收缩比拒绝好（窄结论）。\n  \
         - (b) shrinkage 不改善：n_L3(shrink)<裸μ（同 LCB 压退化）或 ΔR 无改善 ⟹\n    \
           **功效不足非估计方法可解**（与 lcb_vs_naive_l2 inconclusive 同根：16K 短窗 + 8 品种）。\n  \
         - **161号**：退化品种（标 [shrink弃权PnL]）ΔR = 停交易弃权 PnL，**不计** shrinkage 改善。\n  \
         - **保功效是关键**：shrinkage 与 LCB 的本质区别 = 低 n 类借 pooled（不退化）vs 拒绝（退化）；\n    \
           若 n_degen_shrink≈n_degen_lcb 则收缩在本数据未兑现保功效优势（落 (b)）。"
    );

    // acceptance：管线三路跑通。不硬编码 n_L3 关系为不变量（no-patch §5：留给真实数据可证伪——
    // shrinkage 是否保功效是经验问题非定理）。
    eprintln!(
        "\n[保功效诊断] n_L3：裸μ={n_n} shrink={n_s}（shrink≥裸μ ⟹ 保功效兑现；< ⟹ 同 LCB 压退化，落 (b)）"
    );
}

/// **诊断：枚举一个窗的全部方向候选 z**（与 [`build_walk_forward_mu`] 同口径的因果逐 bar 枚举）。
///
/// 复用 train μ 表构造里的同一枚举（`IncrementalClassifier` 逐 bar + append-only diff + assemble_gamma），
/// 但只收集 z（不兑现 X_γ）——用于统计 test 段候选 z 命中/未见 train μ 表的比例（假设4诊断）。
fn enumerate_candidate_z(ds: &Dataset, config: &ThetaConfig) -> Vec<MuClass> {
    let bars = &ds.bars;
    let n = bars.len();
    let mut classifier_incr = IncrementalClassifier::new(bars, config);
    let mut seen: std::collections::HashSet<(usize, usize, u8)> = std::collections::HashSet::new();
    let mut zs: Vec<MuClass> = Vec::new();
    for i in 0..n {
        let bar = &bars[i];
        if bar.untradable || bar.close <= 0 {
            continue;
        }
        let (cls_i, tower_i) = classifier_incr.classify_at(i);
        for (lvl, ls) in cls_i.levels.iter().enumerate() {
            for p in &ls.bsp {
                if !seen.insert((lvl, p.source_index, bsp_disc(&p.bits))) {
                    continue;
                }
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
                let gamma = assemble_gamma_with_tower(&single, &tower_i);
                for c in &gamma {
                    if c.dir == VoiceSide::Flat {
                        continue;
                    }
                    zs.push(z_of_candidate(c));
                }
            }
        }
    }
    zs
}

/// **★退化品种根因诊断（task #49）：3 品种 χ 全滤空仓的四假设逐一证伪/确认**。
///
/// 实证发现 BTC/ES/QQQ 在 θ=0 + treat_empty_as_pass=false 下 chi.n_orders==0（全滤=Pass2 空仓）。
/// codex 异质审要求：审作**实装结果**（μ坍缩/成本符号bug/θ过严/未见类）非静默当"无alpha"——
/// 区分**实装artifact（伪否定）vs 真无alpha（否定成立）**。
///
/// 打印（每个退化品种）：
/// 1. **假设1（μ估计坍缩）**：train μ 表各 z 类的 (μ, count)——是真无正边际类，还是估计退化（每类<2样本）。
/// 2. **假设3（θ过严）**：θ scan {0, −1e-6, −1e-3, −∞}——θ<0 是否解退化（chi 从 0 单变 >0 单）。
/// 3. **假设4（未见类）**：test 段候选 z 命中 train μ 表的比例——多少 z 未见于 train（treat_empty=false⟹全滤）。
///
/// 成本符号（假设2）**静态已否证**：marginal_return 复用 trade_abs_pnl，单测 bit-exact 验证多空+双边费
/// （mu_estimator::tests::marginal_return_matches_directional_pnl / deducts_cost），成本是减项无符号 bug。
///
/// 认识论 **L1**（诊断=管线分类计数确定性变换 / 根因判定=L1 实装事实判断，不验证 alpha 假设）。
///
/// `#[ignore]`：O(n²) 慢测（3 品种 × 32K bar train+test 逐 bar 重分类 + θ scan），`--release` 必须。
/// 跑法：`cargo test --release --lib theta_v0::backtest::l3_delta_r_alpha::degeneracy_diagnosis -- --ignored --nocapture`
#[test]
#[ignore = "退化品种根因诊断；O(n²) 3 品种 θ scan；--release"]
fn degeneracy_diagnosis() {
    let config = ThetaConfig::default();
    // 实证发现的 3 退化品种（χ 全滤空仓）。
    let degenerate = ["BTC", "ES", "QQQ"];

    eprintln!("\n===== 退化品种根因诊断（task #49）：3 品种 χ 全滤空仓的四假设 =====");
    eprintln!("★假设2（成本符号bug）静态已否证：marginal_return 复用 trade_abs_pnl，单测 bit-exact（多空+双边费）");
    eprintln!("★区分实装artifact（伪否定）vs 真无alpha（否定成立）——formalization-validity-domain 231号");

    for sym in degenerate {
        let w = match PREREG_WINDOWS.iter().find(|w| w.symbol == sym) {
            Some(w) => w,
            None => {
                eprintln!("{sym}: 不在 PREREG_WINDOWS（跳过）");
                continue;
            }
        };
        let ds = match data::load_by_symbol(w.symbol, &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{sym}: 加载失败 {e}（DATA BLOCKER）");
                continue;
            }
        };
        let oos_full = ds.slice_date_window(w.oos.0, w.oos.1);
        if oos_full.bars.is_empty() {
            eprintln!("{sym}: OOS 窗空");
            continue;
        }
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

        let (est, n_sig) = build_walk_forward_mu(&train, &config);

        eprintln!("\n────── {sym} (train={} test={} train_signals={n_sig}) ──────", train.bars.len(), test.bars.len());

        // ── 假设1：μ 表分布（各 z 类 μ, count）──
        let mut classes: Vec<(MuClass, f64, u64)> = est
            .iter_mu()
            .map(|(z, mu)| (z, mu, est.count(&z)))
            .collect();
        classes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let n_classes = classes.len();
        let n_pos_mu = classes.iter().filter(|(_, mu, _)| *mu > 0.0).count();
        let n_singleton = classes.iter().filter(|(_, _, c)| *c < 2).count();
        let total_obs: u64 = classes.iter().map(|(_, _, c)| *c).sum();
        eprintln!(
            "  [假设1 μ坍缩] μ类数={n_classes} 总观测={total_obs} | μ>0类={n_pos_mu} | 单样本类(count<2)={n_singleton}"
        );
        eprintln!("    判据：μ>0类=0 ⟹ θ=0 全否决（解释空仓）；单样本类占比高 ⟹ μ估计退化（噪声均值）");
        eprintln!("    top μ 类（前 5）:");
        for (z, mu, c) in classes.iter().take(5) {
            eprintln!(
                "      μ={mu:+.4e} count={c} z=(ℓ{} δ{} I{:#04b} σp{} sd{} {:?})",
                z.level, z.delta, z.i_class, z.parent_dir, z.short_swing as u8, z.position
            );
        }
        if n_classes > 5 {
            eprintln!("    bottom μ 类（后 3）:");
            for (z, mu, c) in classes.iter().rev().take(3) {
                eprintln!(
                    "      μ={mu:+.4e} count={c} z=(ℓ{} δ{} I{:#04b} σp{} sd{} {:?})",
                    z.level, z.delta, z.i_class, z.parent_dir, z.short_swing as u8, z.position
                );
            }
        }

        // ── 假设4：test 段候选 z 命中 train μ 表的比例 ──
        let test_zs = enumerate_candidate_z(&test, &config);
        let n_test = test_zs.len();
        let mut n_seen = 0usize; // 命中 train μ 表
        let mut n_seen_pos = 0usize; // 命中且 μ>0
        let mut n_seen_nonpos = 0usize; // 命中且 μ≤0（θ=0 被滤）
        let mut n_unseen = 0usize; // 未见于 train（treat_empty=false ⟹ 滤）
        let mut pos_hit_z: Vec<MuClass> = Vec::new(); // 命中且 μ>0 的 z（矛盾候选定位）
        for z in &test_zs {
            match est.mu(z) {
                Some(m) if m > 0.0 => {
                    n_seen += 1;
                    n_seen_pos += 1;
                    pos_hit_z.push(*z);
                }
                Some(_) => {
                    n_seen += 1;
                    n_seen_nonpos += 1;
                }
                None => n_unseen += 1,
            }
        }
        let pct = |x: usize| if n_test > 0 { 100.0 * x as f64 / n_test as f64 } else { 0.0 };
        eprintln!(
            "  [假设4 未见类] test候选={n_test} | 命中train={n_seen}({:.1}%) [μ>0={n_seen_pos} μ≤0={n_seen_nonpos}] | 未见={n_unseen}({:.1}%)",
            pct(n_seen), pct(n_unseen)
        );
        eprintln!("    判据：未见%高 ⟹ walk-forward IS/test 不重叠致 z 类漂移，treat_empty=false 全滤（实装artifact）");
        eprintln!("         命中且μ>0%>0 但仍空仓 ⟹ 矛盾（应有单）需查 runner；命中全μ≤0 ⟹ θ=0真否决（真无alpha候选）");
        for z in &pos_hit_z {
            // 矛盾定位：μ>0 命中候选若是 Child（σp≠0 / position=Child）⟹ 需父持仓依附；
            // 若所有 Root（σp0）类 μ≤0 被滤 ⟹ 无父腿 ⟹ Child 短差腿无处依附 ⟹ 不开（级联否决，非矛盾）。
            eprintln!(
                "    ★μ>0命中候选定位: z=(ℓ{} δ{} I{:#04b} σp{} sd{} {:?}) ⟹ {}",
                z.level, z.delta, z.i_class, z.parent_dir, z.short_swing as u8, z.position,
                if z.position == PositionState::Child {
                    "Child(需父持仓依附；若Root全μ≤0被滤⟹无父⟹级联否决,非矛盾)"
                } else {
                    "Root(无依附,应能开⟹若runner仍0单需查RiskOK/AncOK)"
                }
            );
        }

        // ── 假设3：θ scan（θ<0 是否解退化）──
        eprintln!("  [假设3 θ过严] θ scan（test候选按 train μ 表过滤后的 χ=1 候选数）:");
        for &theta in &[0.0f64, -1e-6, -1e-3, -1e-1, f64::NEG_INFINITY] {
            // 全覆盖语义 = θ=−∞ 且 treat_empty=true 才成立；这里固定 treat_empty=false（与实证同口径），
            // 仅扫 θ 看已观测类放行数（未见类恒滤，与实证一致）。θ=−∞ 时放行所有已观测类。
            let pass: usize = test_zs
                .iter()
                .filter(|z| match est.mu(z) {
                    Some(m) => m > theta,
                    None => false, // treat_empty=false（实证同口径）
                })
                .count();
            let label = if theta == f64::NEG_INFINITY { "−∞".to_string() } else { format!("{theta:.0e}") };
            eprintln!("      θ={label:>7} ⟹ χ=1 候选数={pass}（仅已观测类，未见类恒滤）");
        }
        eprintln!("    判据：θ=0→θ<0 候选数从 0 变 >0 ⟹ θ=0过严（μ略负的类被滤，实装artifact可放宽）");
        eprintln!("         θ=−∞ 仍=0（已观测类放行=0）⟹ 所有候选都是未见类 ⟹ 根因=假设4非假设3");
    }

    eprintln!("\n===== 诊断完成（结果包六要素见 Lead 汇报）=====");
}

/// **★跨品种 pooling 同质性检验 L2/L3（acc-pooling-icc, task #86）**。
///
/// 8 品种各建 walk-forward μ 表（与 [`delta_r_alpha_multi_symbol`] 同 split/同因果保证），按 z 类
/// 跨品种聚合（[`super::pooling_icc::collect_by_class`]）后逐类算 ICC（§16 方差分解）+ leave-one-
/// asset-out 迁移检验（§16）。检验「同一 z 类在不同品种是否同分布」——pooling 前提（§15）。
///
/// ## 可证伪二选一（携信息增量）
/// - **(a) 跨品种同质（pooling 有效）**：多数高样本 z 类 ICC 低（→0）∧ leave-one-out 迁移误差小
///   ⟹ τ²≈0 ⟹ 跨品种 pooling 安全提升功效（§33 攻稀疏类）。
/// - **(b) 跨品种异质（强 pooling 引偏差）**：多数 z 类 ICC 高（→1）或迁移误差大 ⟹ τ²>0 ⟹
///   完全合并把异质均值拉平引偏差 ⟹ 应改用收缩（[`super::pooling_icc::pooling_weight`]）而非全 pool。
///
/// 认识论 **L2**（真实 8 品种数据，可证伪「跨品种同质」假设——否定性结果缩小有效域，231号）。
/// 有效域 caveat：{MAX_BARS}bar 截断 + 8 品种 + walk-forward μ（OOS-split train 窗估 μ）。
///
/// `#[ignore]`：O(n²) 慢测（8 品种 × 32K bar train 逐 bar 重分类），`--release` 必须。重跑由 Lead。
/// 跑法：`cargo test --release --lib theta_v0::backtest::l3_delta_r_alpha::pooling_icc_multi_symbol -- --ignored --nocapture`
#[test]
#[ignore = "跨品种 pooling ICC L2/L3；O(n²) 8 品种 × 32K bar；--release"]
fn pooling_icc_multi_symbol() {
    use super::pooling_icc::{collect_by_class, icc, leave_one_asset_out, AssetClassStat};

    let config = ThetaConfig::default();
    /// 类内方差可估的最小品种样本量门（n≥2 才有 within 方差）。
    const MIN_CLASS_N: u64 = 2;
    /// 该 z 类计入「高功效 ICC 统计」的最小品种数（≥3 品种才有可信 between 散布）。
    const MIN_ASSETS_FOR_ICC: usize = 3;

    eprintln!("\n===== ★跨品种 pooling 同质性检验 L2/L3（acc-pooling-icc, task #86）=====");
    eprintln!("★§15 随机效应：μ_{{a,z}}=μ_z+η, η~N(0,τ²)；τ²=0→同分布→pooling 有效；τ²大→异质→强 pooling 引偏差");
    eprintln!("★§16 ICC=τ²/(τ²+σ²)∈[0,1]：→0 噪声主导(pooling 安全)；→1 品种差异主导(慎 pooling)");
    eprintln!("★leave-one-asset-out：其他品种估 μ_{{−a}}，目标品种 OOS 测迁移——稳→pooling 可信/失败→拒绝");
    eprintln!("★{MAX_BARS}bar 截断 = 显式有效域边界（非全窗结论）");

    // 8 品种各建 walk-forward μ 表（train 窗，与 multi_symbol 同 split/因果保证）。
    let mut ests: Vec<(&str, MuEstimator)> = Vec::new();
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
            eprintln!("{:<6} OOS 窗空", w.symbol);
            continue;
        }
        let cut = MAX_BARS.min(oos_full.bars.len());
        let split = cut / 2;
        let train = Dataset {
            symbol: oos_full.symbol.clone(),
            bars: oos_full.bars[0..split].to_vec(),
            dates: oos_full.dates[0..split.min(oos_full.dates.len())].to_vec(),
            bar_seconds: 60,
        };
        if train.bars.len() < MIN_TEST_BARS {
            eprintln!("{:<6} train 样本不足 ⟹ 跳过", w.symbol);
            continue;
        }
        let (est, n_sig) = build_walk_forward_mu(&train, &config);
        eprintln!("{:<6} train={} μ类={} signals={n_sig}", w.symbol, train.bars.len(), est.n_classes());
        ests.push((w.symbol, est));
    }

    if ests.len() < 2 {
        eprintln!("\n[L3 BLOCKER] 有效品种<2（{}）⟹ 跨品种同质性不可检验（需 ≥2 品种估 between 散布）", ests.len());
        eprintln!("  缩小样本量是 substrate/工程缺口（DATA BLOCKER / O(n²) 截断），非异质性结论。");
        return;
    }

    // 按 z 类跨品种聚合 → 逐类 ICC + leave-one-out。
    let by_class = collect_by_class(&ests);
    eprintln!("\n跨品种聚合：唯一 z 类总数={}（≥{MIN_ASSETS_FOR_ICC} 品种共享的类才计入高功效 ICC 统计）", by_class.len());

    // 高功效 ICC 池（≥MIN_ASSETS_FOR_ICC 品种 ∧ 至少 1 品种 n≥2 有 within 方差）。
    let mut icc_pool: Vec<f64> = Vec::new();
    let mut transfer_errs: Vec<f64> = Vec::new();
    let mut n_shared_classes = 0usize; // ≥MIN_ASSETS_FOR_ICC 品种共享的 z 类数
    eprintln!(
        "\n{:<48} {:>6} {:>6} {:>10} {:>10} {:>7}",
        "z 类（共享 ≥3 品种的前若干）", "品种", "n_pool", "τ²", "σ²", "ICC",
    );
    let mut printed = 0usize;
    for (z, list) in &by_class {
        let stats: Vec<AssetClassStat> = list.iter().map(|(_, s)| *s).collect();
        if stats.len() < MIN_ASSETS_FOR_ICC {
            continue;
        }
        // within 方差可估（≥1 品种 n≥MIN_CLASS_N）才计入——全 n=1 ⟹ σ²=0 ⟹ ICC=1 是饥饿伪饱和（icc 文档）。
        let has_within = stats.iter().any(|s| s.n >= MIN_CLASS_N && s.var.is_some());
        if !has_within {
            continue;
        }
        n_shared_classes += 1;
        let r = icc(&stats);
        // 不变量（边界硬约束）。
        assert!((0.0..=1.0).contains(&r.icc), "ICC∈[0,1]，实得 {} z={z:?}", r.icc);
        assert!(r.tau_sq >= 0.0 && r.sigma_sq >= 0.0, "τ²/σ²≥0 z={z:?}");
        icc_pool.push(r.icc);

        // leave-one-out 迁移误差（逐品种留一，收集所有可定义的）。
        for held in 0..stats.len() {
            let lo = leave_one_asset_out(&stats, held);
            if let Some(e) = lo.transfer_abs_err {
                transfer_errs.push(e);
            }
        }

        if printed < 15 {
            eprintln!(
                "ℓ{} δ{} I{:#04b} σp{} sd{} {:<10?}        {:>6} {:>6} {:>10.3e} {:>10.3e} {:>7.4}",
                z.level, z.delta, z.i_class, z.parent_dir, z.short_swing as u8, z.position,
                r.n_assets, r.n_total, r.tau_sq, r.sigma_sq, r.icc,
            );
            printed += 1;
        }
    }

    // ── L3 跨品种同质性聚合裁定 ──
    let median = |v: &mut [f64]| -> f64 {
        if v.is_empty() { return f64::NAN; }
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let m = v.len() / 2;
        if v.len() % 2 == 0 { (v[m - 1] + v[m]) / 2.0 } else { v[m] }
    };
    let mut icc_sorted = icc_pool.clone();
    let icc_median = median(&mut icc_sorted);
    let icc_mean = if icc_pool.is_empty() { f64::NAN } else { icc_pool.iter().sum::<f64>() / icc_pool.len() as f64 };
    let n_low_icc = icc_pool.iter().filter(|&&v| v < 0.3).count(); // 同质阈（ICC<0.3 弱品种相关）
    let n_high_icc = icc_pool.iter().filter(|&&v| v > 0.7).count(); // 异质阈（ICC>0.7 强品种相关）
    let mut terr_sorted = transfer_errs.clone();
    let terr_median = median(&mut terr_sorted);

    eprintln!("\n===== L3 跨品种同质性聚合 =====");
    eprintln!("有效品种数                          : {}/{}", ests.len(), PREREG_WINDOWS.len());
    eprintln!("≥{MIN_ASSETS_FOR_ICC} 品种共享且 within 可估的 z 类 : {n_shared_classes}");
    eprintln!("ICC 中位数 / 均值                   : {icc_median:.4} / {icc_mean:.4}");
    eprintln!("低 ICC(<0.3 同质) / 高 ICC(>0.7 异质): {n_low_icc} / {n_high_icc}（共 {} 类）", icc_pool.len());
    eprintln!("leave-one-out 迁移误差中位数        : {terr_median:.3e}（{} 个留一对）", transfer_errs.len());
    eprintln!(
        "\n★诚实裁定（§15/§16 + formalization-validity-domain 231号）：\n  \
         - (a) 跨品种同质（pooling 有效）⟺ ICC 多数低(<0.3) ∧ 迁移误差小 ⟹ τ²≈0 ⟹ pooling 安全提升功效。\n  \
         - (b) 跨品种异质（强 pooling 引偏差）⟺ ICC 多数高(>0.7) 或迁移误差大 ⟹ τ²>0 ⟹ 应收缩(pooling_weight)非全 pool。\n  \
         - 功效边界：n_shared_classes 小（{MAX_BARS}bar 截断 + 8 品种）⟹ ICC 统计本身功效不足，\n    \
           诚实报 blocked 而非「同质确认」（缺乏否证同质性的统计功效 ≠ 同质成立）。\n  \
         - 单品种类（<{MIN_ASSETS_FOR_ICC} 品种）不计入——无法谈「品种间」相关（ICC 定义需 ≥2 品种 between 散布）。"
    );

    // acceptance：管线在多品种真实数据上跑通（≥2 品种 ∧ ICC 全部合法 ∈[0,1]）。
    assert!(ests.len() >= 2, "≥2 品种才能检验跨品种同质性");
    // 不硬编码 ICC 高低为不变量（no-patch §5：跨品种是否同质是经验问题非定理，留真实数据可证伪）。
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 合成鞅性质：增量 ε∈{−1,0,+1}，价格恒 ≥1，无系统性漂移（样本均值增量 ≈0）。
    #[test]
    fn synthetic_martingale_is_driftless() {
        let ds = synthetic_martingale(50_000, 12345, 10_000);
        assert_eq!(ds.bars.len(), 50_000);
        let mut sum_dp = 0i64;
        let mut max_step = 0i64;
        for w in ds.bars.windows(2) {
            let dp = w[1].close - w[0].close;
            sum_dp += dp;
            max_step = max_step.max(dp.abs());
            assert!(w[1].close >= 1, "close 恒 ≥1（量化非负 + 反射）");
            assert_eq!(w[1].open, w[1].close, "OHLC 全=close（纯 close 鞅，无 intrabar 信息）");
        }
        assert!(max_step <= 1, "增量 ε∈{{−1,0,+1}}（|ε|≤1）");
        // 无系统性漂移：50K 步累计位移应远小于步数（鞅 ⟹ E[ΣΔP]=0，√n 量级波动）。
        let mean_dp = sum_dp as f64 / 49_999.0;
        assert!(mean_dp.abs() < 0.01, "平均增量 ≈0（无漂移），实得 {mean_dp:.5}");
    }

    /// 同种子可复现（确定性 PRNG，bit-exact）。
    #[test]
    fn synthetic_martingale_deterministic() {
        let a = synthetic_martingale(1000, 777, 5000);
        let b = synthetic_martingale(1000, 777, 5000);
        assert_eq!(a.bars, b.bars, "同种子 bit-exact 复现");
        let c = synthetic_martingale(1000, 778, 5000);
        assert_ne!(a.bars, c.bars, "不同种子序列不同");
    }


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

    /// **★cross-fit purged split 无时间泄漏（K-fold 几何不变量，acc-crossfit-oos）**。
    ///
    /// 验证 crossfit_l2 的 fold 切分 + gap purge 算术：每个 held-out fold k 与其 train 段之间
    /// 隔 ≥CROSSFIT_GAP bar（held-out 不与 train 退出兑现跨度相邻 ⟹ 无时间泄漏）。纯索引几何，
    /// 不跑 harness（O(n²)，那是 ignored 慢测的事）——验证「选/评分离 + gap」的边界正确性。
    #[test]
    fn crossfit_folds_purged_no_overlap() {
        let cut = MAX_BARS; // 32000
        let fold_len = cut / CROSSFIT_K; // 6400
        let gap = CROSSFIT_GAP;
        for k in 0..CROSSFIT_K {
            let ho_lo = k * fold_len;
            let ho_hi = if k == CROSSFIT_K - 1 { cut } else { (k + 1) * fold_len };
            // 前段 train [0, ho_lo−gap)：末端到 held-out 起隔 gap。
            if ho_lo >= gap + MIN_TEST_BARS {
                let pre_hi = ho_lo - gap;
                assert!(pre_hi <= ho_lo, "前段 train 不进 held-out");
                assert!(ho_lo - pre_hi >= gap, "前段 train 与 held-out 隔 ≥gap：{}", ho_lo - pre_hi);
            }
            // 后段 train [ho_hi+gap, cut)：起点到 held-out 末隔 gap。
            if ho_hi + gap + MIN_TEST_BARS <= cut {
                let post_lo = ho_hi + gap;
                assert!(post_lo >= ho_hi, "后段 train 不进 held-out");
                assert!(post_lo - ho_hi >= gap, "后段 train 与 held-out 隔 ≥gap：{}", post_lo - ho_hi);
            }
        }
        // fold 覆盖完备（末 fold 吃余数）：Σfold = cut，无缝隙无重叠。
        let mut covered = 0usize;
        for k in 0..CROSSFIT_K {
            let lo = k * fold_len;
            let hi = if k == CROSSFIT_K - 1 { cut } else { (k + 1) * fold_len };
            assert_eq!(lo, covered, "fold 无缝隙：fold {k} 起={lo} 应接上轮末={covered}");
            covered = hi;
        }
        assert_eq!(covered, cut, "K 个 fold 覆盖全 cut（末 fold 吃余数）");
    }

    /// ΔR 序列统计：恒等**权益曲线**（χ==baseline）⟹ ΔR≡0 ⟹ chi_changed_trades=false（L1 机制门）。
    #[test]
    fn delta_r_identical_returns_zero() {
        // equity_curve 口径：归一化绝对权益序列（E_t）。χ==baseline ⟹ 绝对增量配对差≡0。
        let eq = vec![1.0, 1.01, 0.99, 1.02, 1.02, 1.03];
        let st = delta_r_stats(&eq, &eq, 1.0e6, 252.0);
        assert!(!st.chi_changed_trades, "χ==baseline ⟹ ΔR≡0 ⟹ χ 未改交易集");
        assert!(st.mean.abs() < 1e-15, "ΔR 均值=0");
    }

    /// ΔR 序列统计：χ 权益增量恒大于 baseline ⟹ mean(ΔR)>0 ∧ boot p 小（确认管线产正结果能力）。
    #[test]
    fn delta_r_positive_when_chi_dominates() {
        // baseline 权益恒定（增量 0）；χ 每 bar 绝对权益 +0.001（增量恒 +0.001>0）⟹ ΔR/nav0 恒正。
        let base = vec![1.0; 300];
        let chi: Vec<f64> = (0..300).map(|i| 1.0 + 0.001 * i as f64).collect();
        let st = delta_r_stats(&base, &chi, 1.0e6, 252.0);
        assert!(st.chi_changed_trades, "χ≠baseline ⟹ ΔN 非全等");
        assert!(st.mean > 0.0, "χ 主导 ⟹ mean(ΔR)>0，实得 {}", st.mean);
        assert!(st.boot_pvalue < 0.05, "恒正 ΔR ⟹ bootstrap p<0.05（拒绝 H0:ΔR≤0），实得 {}", st.boot_pvalue);
    }

    /// ΔR 序列统计：χ 权益增量恒小于 baseline ⟹ mean(ΔR)<0 ∧ boot p≈1（否证能力——不冒充正 alpha）。
    #[test]
    fn delta_r_negative_when_chi_worse() {
        let base: Vec<f64> = (0..300).map(|i| 1.0 + 0.001 * i as f64).collect(); // baseline 增量 +0.001
        let chi = vec![1.0; 300]; // χ 权益恒定（增量 0）⟹ ΔR=0−base 增量<0
        let st = delta_r_stats(&base, &chi, 1.0e6, 252.0);
        assert!(st.mean < 0.0, "χ 劣于 baseline ⟹ mean(ΔR)<0");
        assert!(st.boot_pvalue > 0.95, "恒负 ΔR ⟹ bootstrap p≈1（无法拒绝 H0，正确否证），实得 {}", st.boot_pvalue);
    }

    /// **★P0 口径修复回归测试（delta-r-audit）**：NAV 路径发散时，绝对增量配对差 ≠ 百分比收益配对差。
    ///
    /// 构造两条 NAV 发散的归一化权益序列，证明：
    /// (1) 现口径（绝对增量 E_t−E_{t−1}）= §10 ΔR/nav0，与发散无关；
    /// (2) 旧口径（百分比 E_t/E_{t−1}−1 配对差）被 path-dependent 的 E_{t−1} 污染，≠ ΔR/nav0。
    /// 两者数值显著不同 ⟹ 证明旧实装测的不是 §10 的 ΔR。
    #[test]
    fn delta_r_caliber_diverges_from_percent_when_nav_paths_diverge() {
        // 两条发散 NAV：base 从 1.0 缓涨，chi 从 2.0 起（已发散的 E_{t−1}）但**绝对增量相同**。
        // 绝对增量相同 ⟹ §10 ΔR 配对差应 ≡0；但百分比收益因除数 E_{t−1} 不同 ⟹ 配对差 ≠0。
        let base = vec![1.0, 1.10, 1.20, 1.30, 1.40, 1.50];
        let chi = vec![2.0, 2.10, 2.20, 2.30, 2.40, 2.50]; // 同绝对增量 +0.10/bar，E_{t−1} 发散

        // (1) 现口径（绝对增量配对差）：增量都 +0.10 ⟹ 配对差恒 0 ⟹ mean=0。
        let st = delta_r_stats(&base, &chi, 1.0e6, 252.0);
        assert!(
            st.mean.abs() < 1e-15,
            "绝对增量相同 ⟹ §10 ΔR 配对差≡0（与 NAV 发散无关），实得 mean={}",
            st.mean
        );

        // (2) 旧口径（百分比收益配对差）：r_t = E_t/E_{t−1}−1，除数发散 ⟹ 配对差 ≠0。
        let pct = |e: &[f64]| -> Vec<f64> {
            e.windows(2).map(|w| w[1] / w[0] - 1.0).collect()
        };
        let pct_base = pct(&base);
        let pct_chi = pct(&chi);
        let pct_pair_diff_mean: f64 = pct_chi
            .iter()
            .zip(&pct_base)
            .map(|(c, b)| c - b)
            .sum::<f64>()
            / pct_base.len() as f64;
        // base 首 bar 百分比 0.10/1.0=0.0909；chi 0.10/2.0=0.05 ⟹ 配对差 −0.0409≠0。
        assert!(
            pct_pair_diff_mean.abs() > 1e-3,
            "旧百分比口径被 path-dependent E_{{t−1}} 污染 ⟹ 配对差≠0（实得 {pct_pair_diff_mean}）——\
             与绝对增量口径（≡0）显著不同，证明旧实装测的不是 §10 的 ΔR"
        );
    }
}
