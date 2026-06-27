//! 回测指标层——权益曲线 + 交易列表 → 协议 §3 全套指标。
//!
//! ## 认识论等级：**L1**（确定性变换，零信息增量）
//!
//! 给定权益曲线/交易序列，算 CAGR/Sharpe/... 是确定性数值变换——验证指标算法正确，
//! **不验证 Θ 有效**（formalization-validity-domain 231号）。指标的 **L2** 价值只在喂入
//! **真实历史数据驱动的引擎产出**的权益曲线时才产生（届时指标可否证 Θ）。
//!
//! ## 协议锚点（docs/backtest-protocol-v0.md §3，全部预注册，全部报告）
//!
//! 不允许只报有利指标。本模块计算 §3 列出的**全部**指标，调用方一次性输出。
//!
//! ## 实现诚实声明
//!
//! - **统计显著性**（§3.4 Sharpe 标准误 Lo 2002 调整 / block bootstrap p 值）和**随机
//!   入场对照**（§4，seed=20260625）是**独立的统计检验步骤**，依赖完整交易序列与
//!   per-trade 收益分布。本模块提供基础指标（CAGR/Sharpe/...）；bootstrap/随机对照在
//!   runner 层组织（需要引擎产出真实订单流后才有意义，属**等引擎阻塞点**）。
//! - 年化因子按"每年交易日"近似——日聚合 returns 的年化用 √252（标准约定）。各品种实际
//!   交易日数在 runner 层按数据日历精确化（§3.1"年化因子按各品种实际交易日"）。

/// 协议 §3 全套指标（单个检验窗）。每个字段对应 §3 一行。
#[derive(Debug, Clone, PartialEq)]
pub struct Metrics {
    // ── §3.1 收益与风险调整 ──
    /// 几何年化收益（扣成本后）。
    pub cagr: f64,
    /// 年化 Sharpe（rf=0，日 returns 年化）。
    pub sharpe: f64,
    /// 年化 Sortino（下行标准差）。
    pub sortino: f64,
    /// 最大回撤（峰谷，正数，0..1）。
    pub max_drawdown: f64,
    /// Calmar = CAGR / |MaxDD|。
    pub calmar: f64,
    /// CVaR(5%)：5% 尾部条件期望亏损（日 returns，负数表亏损）。
    pub cvar_5: f64,

    // ── §3.2 交易行为 ──
    /// 完整开平仓次数。
    pub n_trades: usize,
    /// 胜率 = 盈利交易 / 总交易。
    pub win_rate: f64,
    /// 盈亏比 = 平均盈利 / |平均亏损|。
    pub profit_factor: f64,
    /// top-5 交易盈亏和 / 总盈亏（§5.2 失败判据：>50% 收益过度集中）。
    pub top5_pnl_ratio: f64,

    // ── 基线对照（§4 Buy&Hold；随机/方向打乱在 runner 层）──
    /// 策略总收益率（期末权益/初始-1）。
    pub strat_return: f64,
    /// buy&hold 总收益率（末 close/首 close-1）。
    pub bh_return: f64,
}

/// 计算全套指标。
///
/// 输入：
/// - `equity_curve`：逐 bar 权益（归一化，初始=1.0），与回测 bar 序列同长。
/// - `trade_pnls`：每笔完整平仓交易的盈亏（归一化口径）。
/// - `bars_per_year`：年化因子分母（日 returns 数 / 年；§3.1 按各品种实际交易日）。
///   1min bar 聚合到日 returns 后，标准约定 √252——此处传日 returns 数对应的年化基数。
/// - `bh_return`：buy&hold 收益率（runner 层算，传入对照）。
///
/// **边界条件**：`equity_curve` 空或全平（无交易）⇒ 收益/风险指标为 0，盈亏比/胜率为 0
/// （NaN 防御，§5.5 由 runner 判 inconclusive）。
pub fn compute(
    equity_curve: &[f64],
    daily_returns: &[f64],
    trade_pnls: &[f64],
    years: f64,
    bh_return: f64,
) -> Metrics {
    let strat_return = if let (Some(&first), Some(&last)) =
        (equity_curve.first(), equity_curve.last())
    {
        if first > 0.0 {
            last / first - 1.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    // CAGR：几何年化。years<=0 或权益<=0 ⇒ 0。
    let cagr = if years > 0.0 && strat_return > -1.0 {
        (1.0 + strat_return).powf(1.0 / years) - 1.0
    } else {
        0.0
    };

    let (sharpe, sortino) = sharpe_sortino(daily_returns);
    let max_drawdown = max_drawdown(equity_curve);
    let calmar = if max_drawdown > 1e-12 {
        cagr / max_drawdown
    } else {
        0.0
    };
    let cvar_5 = cvar(daily_returns, 0.05);

    let (win_rate, profit_factor) = win_rate_profit_factor(trade_pnls);
    let top5_pnl_ratio = top_n_pnl_ratio(trade_pnls, 5);

    Metrics {
        cagr,
        sharpe,
        sortino,
        max_drawdown,
        calmar,
        cvar_5,
        n_trades: trade_pnls.len(),
        win_rate,
        profit_factor,
        top5_pnl_ratio,
        strat_return,
        bh_return,
    }
}

// ════════════════════════════════════════════════════════════════════════
//  统计显著性检验（§3.4 + §4，预注册，seed=20260625）
//
//  ## 认识论等级：**L2**（当且仅当输入是真实数据回测的交易序列）
//
//  §3.4 block bootstrap / §4 随机入场对照是**可否证**的假设检验——它们对真实数据
//  回测的 trade_pnls/daily_returns 给出 p 值与经验分布，用于判定 Θ 策略是否**显著
//  优于零收益 / 优于随机择时**（§5.1/§5.3 失败判据）。否定性结果（p>0.05 或 Sharpe
//  落在随机 95% 区间内）= Θ 被否证，缩小有效域，是正信息增量（231号）。
//
//  ★合成数据上跑这些检验仍是 **L1**（验证检验管线正确，零信息增量）——L2 只在喂
//  真实回测交易序列时产生。
//
//  ## seed 冻结（§4，可复现硬约束）
//
//  随机数 seed = **20260625**（codex 裁决日，backtest-protocol-v0.md:141 冻结）。
//  用 SplitMix64（无外部依赖、纯算术、确定性）——重跑 bit-exact 复现。
// ════════════════════════════════════════════════════════════════════════

/// 预注册随机种子（§4，backtest-protocol-v0.md:141 冻结，不可改）。
pub const PREREG_SEED: u64 = 20260625;

/// block bootstrap 块长（§3.4：20 个交易日，backtest-protocol-v0.md:127）。
pub const BLOCK_LEN: usize = 20;

/// bootstrap / 蒙特卡洛重采样次数（§3.4 + §4：n=1000）。
pub const N_RESAMPLE: usize = 1000;

/// SplitMix64 确定性 PRNG（无依赖，纯算术）。
///
/// 给定 seed，`next_u64` 序列完全确定（bit-exact 可复现，§4 seed 冻结要求）。
/// 这是技术性工具（伪随机数发生器），不涉及任何缠论领域概念。
#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        SplitMix64 { state: seed }
    }

    /// 下一个 u64（SplitMix64 标准约简，确定性）。
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// `[0, n)` 均匀整数（n>0；Lemire 无偏约简的简化——取模偏置在 n≪2^64 时可忽略，
    /// 但为确定性可复现，用直接取模，bit-exact）。
    fn next_below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0, "next_below 要求 n>0");
        (self.next_u64() % n as u64) as usize
    }
}

/// §3.4 + §4 统计显著性检验结果（预注册全套，全部报告，不挑有利项）。
#[derive(Debug, Clone, PartialEq)]
pub struct Significance {
    // ── §3.4 block bootstrap（H0：收益 ≤ 0）──
    /// bootstrap 重采样的交易序列**总收益**经验分布的均值。
    pub boot_mean_total_pnl: f64,
    /// **单边 p 值 H0：总收益 ≤ 0**（§5.1 失败判据：p>0.05 ⟹ 收益不显著）。
    /// = 重采样中总收益 ≤ 0 的比例（经验 p 值）。
    pub boot_pvalue_pnl_le_0: f64,
    /// bootstrap 总收益的 95% 置信区间下界（2.5 分位）。
    pub boot_ci95_lo: f64,
    /// bootstrap 总收益的 95% 置信区间上界（97.5 分位）。
    pub boot_ci95_hi: f64,

    // ── §3.4 Sharpe 标准误（Lo 2002 自相关调整）──
    /// 实测年化 Sharpe（日 returns）。
    pub sharpe: f64,
    /// Sharpe 标准误（Lo 2002 一阶自相关调整）。
    pub sharpe_se: f64,
    /// Sharpe 95% 置信区间下界（sharpe - 1.96·se）。
    pub sharpe_ci95_lo: f64,
    /// Sharpe 95% 置信区间上界（sharpe + 1.96·se）。
    pub sharpe_ci95_hi: f64,

    // ── §4 随机入场对照（择时信息含量）──
    /// 随机入场对照的总收益经验分布均值（保持交易笔数，仅打乱 pnl 配对顺序——
    /// 入场时点随机化的**离散代理**：在真实成交 pnl 池里重抽 n_trades 笔，检验 Θ 的
    /// 择时（哪些 bar 入场）是否优于随机抽取同样笔数）。
    pub rand_mean_total_pnl: f64,
    /// Θ 实测总收益在随机对照分布中的分位（0..1）。
    /// §5.3 失败判据：Θ 落在随机分布 95% 区间内（分位 < 0.95）⟹ 择时无信息。
    pub rand_percentile_of_theta: f64,
    /// Θ 是否**显著优于**随机择时（分位 ≥ 0.95，单边）。
    pub theta_beats_random: bool,
}

/// 计算 §3.4 block bootstrap + Sharpe 标准误 + §4 随机入场对照（预注册，seed 冻结）。
///
/// 输入：
/// - `trade_pnls`：每笔完整平仓交易的盈亏（真实数据回测产出 ⟹ L2；合成 ⟹ L1）。
/// - `daily_returns`：日 returns 序列（Sharpe 标准误 Lo 2002 调整用）。
///
/// **block bootstrap**（§3.4，backtest-protocol-v0.md:127）：以 [`BLOCK_LEN`]=20 为块长，
/// 从 `trade_pnls` 有放回抽连续块拼成等长重采样序列（保留交易序列的局部自相关结构），
/// 算其总收益，重复 [`N_RESAMPLE`]=1000 次 ⟹ 总收益经验分布 ⟹ 单边 p 值（H0：收益≤0）
/// + 95% 置信区间。
///
/// **Sharpe 标准误**（§3.4，Lo 2002）：日 returns 的 Sharpe 标准误，一阶自相关调整因子
/// √(1 - 2·ρ₁·(1-1/n)) 近似（Lo 2002 式 ρ₁=lag-1 自相关）。
///
/// **随机入场对照**（§4，backtest-protocol-v0.md:138）：保持交易笔数 n_trades 不变，从真实
/// 成交 pnl 池有放回重抽 n_trades 笔（入场时点随机化的离散代理：随机选哪些"成交结果"），
/// n=1000 次 ⟹ 总收益分布 ⟹ Θ 实测在其中的分位。Θ 分位 ≥0.95 ⟹ 择时携带信息（优于随机）。
///
/// **边界条件**：`trade_pnls` 为空或仅 1 笔 ⟹ bootstrap 退化（p=1.0，CI=[0,0]，
/// theta_beats_random=false）——无交易序列无法否证（§5.5 inconclusive，不冒充显著性）。
///
/// **谱系引用**：§3.4/§4 统计方法直接引用 backtest-protocol-v0.md（看结果前冻结，
/// codex 裁决 seed=20260625）。本函数不引入新判据（下游不可事后改判据，协议 §影响声明）。
pub fn significance(trade_pnls: &[f64], daily_returns: &[f64]) -> Significance {
    let (sharpe, _) = sharpe_sortino(daily_returns);
    let (sharpe_se, sharpe_ci95_lo, sharpe_ci95_hi) = sharpe_lo2002(daily_returns, sharpe);

    let n = trade_pnls.len();
    // 退化：无交易 / 仅 1 笔 ⟹ 无可重采样的序列结构 ⟹ 不冒充显著性（§5.5 inconclusive）。
    if n < 2 {
        return Significance {
            boot_mean_total_pnl: trade_pnls.iter().sum(),
            boot_pvalue_pnl_le_0: 1.0, // 无法拒绝 H0（无证据）
            boot_ci95_lo: 0.0,
            boot_ci95_hi: 0.0,
            sharpe,
            sharpe_se,
            sharpe_ci95_lo,
            sharpe_ci95_hi,
            rand_mean_total_pnl: trade_pnls.iter().sum(),
            rand_percentile_of_theta: 0.0,
            theta_beats_random: false,
        };
    }

    let theta_total: f64 = trade_pnls.iter().sum();

    // ── §3.4 block bootstrap（块长 20，n=1000，H0：总收益≤0）──
    let mut rng = SplitMix64::new(PREREG_SEED);
    let mut boot_totals: Vec<f64> = Vec::with_capacity(N_RESAMPLE);
    for _ in 0..N_RESAMPLE {
        let total = block_bootstrap_total(trade_pnls, BLOCK_LEN, &mut rng);
        boot_totals.push(total);
    }
    let boot_mean_total_pnl = boot_totals.iter().sum::<f64>() / N_RESAMPLE as f64;
    // 单边 p 值 H0：总收益 ≤ 0 = 重采样中总收益 ≤ 0 的比例。
    let n_le_0 = boot_totals.iter().filter(|&&t| t <= 0.0).count();
    let boot_pvalue_pnl_le_0 = n_le_0 as f64 / N_RESAMPLE as f64;
    let (boot_ci95_lo, boot_ci95_hi) = percentile_ci(&mut boot_totals, 0.025, 0.975);

    // ── §4 随机入场对照（保持笔数，pnl 池有放回重抽，n=1000）──
    // seed 续用同一流（确定性：bootstrap 后 rng 状态延续，仍 seed 派生，可复现）。
    let mut rand_totals: Vec<f64> = Vec::with_capacity(N_RESAMPLE);
    for _ in 0..N_RESAMPLE {
        let mut total = 0.0;
        for _ in 0..n {
            total += trade_pnls[rng.next_below(n)];
        }
        rand_totals.push(total);
    }
    let rand_mean_total_pnl = rand_totals.iter().sum::<f64>() / N_RESAMPLE as f64;
    // Θ 实测总收益在随机分布中的分位（< theta_total 的比例）。
    let n_below = rand_totals.iter().filter(|&&t| t < theta_total).count();
    let rand_percentile_of_theta = n_below as f64 / N_RESAMPLE as f64;
    // §5.3：Θ 分位 ≥ 0.95 ⟹ 显著优于随机（落在随机 95% 区间外的右侧）。
    let theta_beats_random = rand_percentile_of_theta >= 0.95;

    Significance {
        boot_mean_total_pnl,
        boot_pvalue_pnl_le_0,
        boot_ci95_lo,
        boot_ci95_hi,
        sharpe,
        sharpe_se,
        sharpe_ci95_lo,
        sharpe_ci95_hi,
        rand_mean_total_pnl,
        rand_percentile_of_theta,
        theta_beats_random,
    }
}

/// 一次 block bootstrap 重采样的总收益（块长 `block`，有放回抽连续块拼成 ≥len 序列后截断）。
///
/// 保留交易序列局部自相关结构（§3.4 block bootstrap 的目的——独立 bootstrap 破坏自相关）。
/// 抽块起点在 `[0, len)` 均匀（循环 wrap，使每笔被抽概率均等，避免末尾块偏置）。
fn block_bootstrap_total(pnls: &[f64], block: usize, rng: &mut SplitMix64) -> f64 {
    let len = pnls.len();
    let block = block.min(len).max(1);
    let mut total = 0.0;
    let mut filled = 0usize;
    while filled < len {
        let start = rng.next_below(len);
        let take = block.min(len - filled);
        for k in 0..take {
            total += pnls[(start + k) % len]; // 循环 wrap，等概率覆盖
        }
        filled += take;
    }
    total
}

/// 经验分位置信区间（就地排序 `samples`，取 `lo`/`hi` 分位）。
fn percentile_ci(samples: &mut [f64], lo: f64, hi: f64) -> (f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = samples.len();
    let idx = |q: f64| -> usize {
        (((n as f64 - 1.0) * q).round() as usize).min(n - 1)
    };
    (samples[idx(lo)], samples[idx(hi)])
}

/// Sharpe 标准误（Lo 2002 一阶自相关调整）+ 95% 置信区间。
///
/// Lo (2002)「The Statistics of Sharpe Ratios」：IID 下 SE(SR) ≈ √((1+SR²/2)/n)；
/// 一阶自相关 ρ₁ 调整因子 √(1 + ... )。本实装用 Lo 的自相关调整简化式：
/// SE(SR_annualized) ≈ SE_iid · √(adj)，adj = 1 - 2·ρ₁·(1 - 1/n)（一阶 AR(1) 近似）。
/// ρ₁ = lag-1 自相关。**诚实**：Lo 2002 完整式含全阶自相关；本实装取一阶（AR(1)）近似——
/// 标注 L1 口径（统计算法正确性），完整高阶 GMM 估计待 L3 精化（不冒充全阶）。
///
/// 边界：n<2 ⟹ SE=0，CI=[sharpe,sharpe]（无波动无标准误）。
fn sharpe_lo2002(daily_returns: &[f64], sharpe: f64) -> (f64, f64, f64) {
    let n = daily_returns.len();
    if n < 2 {
        return (0.0, sharpe, sharpe);
    }
    // IID 标准误（年化 Sharpe 的标准误 ≈ √((1 + SR²/2)/n)，SR 为年化值）。
    let se_iid = ((1.0 + sharpe * sharpe / 2.0) / n as f64).sqrt();
    // lag-1 自相关 ρ₁。
    let rho1 = lag1_autocorr(daily_returns);
    // AR(1) 调整因子（Lo 2002 一阶近似；ρ₁>0 正自相关 ⟹ SE 放大）。
    let adj = (1.0 + 2.0 * rho1 * (1.0 - 1.0 / n as f64)).max(0.0);
    let se = se_iid * adj.sqrt();
    (se, sharpe - 1.96 * se, sharpe + 1.96 * se)
}

/// lag-1 自相关系数 ρ₁（日 returns 序列）。
fn lag1_autocorr(x: &[f64]) -> f64 {
    let n = x.len();
    if n < 2 {
        return 0.0;
    }
    let mean = x.iter().sum::<f64>() / n as f64;
    let denom: f64 = x.iter().map(|v| (v - mean).powi(2)).sum();
    if denom <= 1e-18 {
        return 0.0;
    }
    let numer: f64 = (0..n - 1).map(|i| (x[i] - mean) * (x[i + 1] - mean)).sum();
    numer / denom
}

/// 年化 Sharpe + Sortino（rf=0）。日 returns 序列 → 均值/标准差 → ×√252 年化。
fn sharpe_sortino(daily_returns: &[f64]) -> (f64, f64) {
    let n = daily_returns.len();
    if n < 2 {
        return (0.0, 0.0);
    }
    let mean = daily_returns.iter().sum::<f64>() / n as f64;
    // 样本标准差（n-1）。
    let var = daily_returns
        .iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>()
        / (n - 1) as f64;
    let std = var.sqrt();
    // 下行标准差（仅负 returns，相对 0 目标）。
    let downside_var = daily_returns
        .iter()
        .filter(|&&r| r < 0.0)
        .map(|r| r.powi(2))
        .sum::<f64>()
        / (n - 1) as f64;
    let downside_std = downside_var.sqrt();

    const ANN: f64 = 252.0;
    let sharpe = if std > 1e-12 {
        mean / std * ANN.sqrt()
    } else {
        0.0
    };
    let sortino = if downside_std > 1e-12 {
        mean / downside_std * ANN.sqrt()
    } else {
        0.0
    };
    (sharpe, sortino)
}

/// 最大回撤（峰谷，正数，基于权益曲线）。
fn max_drawdown(equity: &[f64]) -> f64 {
    let mut peak = f64::MIN;
    let mut max_dd = 0.0;
    for &e in equity {
        if e > peak {
            peak = e;
        }
        if peak > 0.0 {
            let dd = (peak - e) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    max_dd
}

/// CVaR(α)：最差 α 分位尾部的条件期望（日 returns）。返回负数（亏损）。
fn cvar(daily_returns: &[f64], alpha: f64) -> f64 {
    let n = daily_returns.len();
    if n == 0 {
        return 0.0;
    }
    let mut sorted: Vec<f64> = daily_returns.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // 尾部样本数（至少 1）。
    let k = ((n as f64) * alpha).ceil().max(1.0) as usize;
    let tail = &sorted[..k.min(n)];
    tail.iter().sum::<f64>() / tail.len() as f64
}

/// 胜率 + 盈亏比。
fn win_rate_profit_factor(pnls: &[f64]) -> (f64, f64) {
    if pnls.is_empty() {
        return (0.0, 0.0);
    }
    let wins: Vec<f64> = pnls.iter().copied().filter(|&p| p > 0.0).collect();
    let losses: Vec<f64> = pnls.iter().copied().filter(|&p| p < 0.0).collect();
    let win_rate = wins.len() as f64 / pnls.len() as f64;
    let avg_win = if wins.is_empty() {
        0.0
    } else {
        wins.iter().sum::<f64>() / wins.len() as f64
    };
    let avg_loss = if losses.is_empty() {
        0.0
    } else {
        losses.iter().sum::<f64>() / losses.len() as f64
    };
    let profit_factor = if avg_loss.abs() > 1e-12 {
        avg_win / avg_loss.abs()
    } else {
        0.0
    };
    (win_rate, profit_factor)
}

/// top-N 交易盈亏和 / 总盈亏（§5.2 收益集中度）。总盈亏≤0 ⇒ 0（防御）。
fn top_n_pnl_ratio(pnls: &[f64], n: usize) -> f64 {
    let total: f64 = pnls.iter().sum();
    if total <= 1e-12 {
        return 0.0;
    }
    let mut sorted: Vec<f64> = pnls.to_vec();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)); // 降序
    let top_sum: f64 = sorted.iter().take(n).sum();
    top_sum / total
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空权益/无交易 → 全 0（不 NaN，§5.5 inconclusive 防御）。
    #[test]
    fn empty_inputs_yield_zero_metrics() {
        let m = compute(&[], &[], &[], 1.0, 0.0);
        assert_eq!(m.cagr, 0.0);
        assert_eq!(m.sharpe, 0.0);
        assert_eq!(m.n_trades, 0);
        assert_eq!(m.win_rate, 0.0);
        assert_eq!(m.profit_factor, 0.0);
        assert_eq!(m.top5_pnl_ratio, 0.0);
    }

    /// max_drawdown 峰谷正确（已知曲线）。
    #[test]
    fn max_drawdown_known_curve() {
        // 1 → 2 → 1（峰 2，谷 1，回撤 50%）→ 1.5。
        let dd = max_drawdown(&[1.0, 2.0, 1.0, 1.5]);
        assert!((dd - 0.5).abs() < 1e-9, "峰 2 谷 1 = 50% 回撤");
    }

    /// CVaR(5%) 取最差尾部均值（负数）。
    #[test]
    fn cvar_picks_worst_tail() {
        // 20 个 returns，最差 1 个（5%）= -0.10。
        let mut rets = vec![0.01; 19];
        rets.push(-0.10);
        let c = cvar(&rets, 0.05);
        assert!((c - (-0.10)).abs() < 1e-9, "5% 尾部 = 最差 1 个 = -0.10");
    }

    /// 胜率 + 盈亏比已知值。
    #[test]
    fn win_rate_and_profit_factor_known() {
        // 3 笔：+2, +4, -3 ⇒ 胜率 2/3；avg_win=3, avg_loss=3 ⇒ pf=1.0。
        let (wr, pf) = win_rate_profit_factor(&[2.0, 4.0, -3.0]);
        assert!((wr - 2.0 / 3.0).abs() < 1e-9);
        assert!((pf - 1.0).abs() < 1e-9, "avg_win 3 / avg_loss 3 = 1.0");
    }

    /// top-5 占比（§5.2）：少于 5 笔时取全部。
    #[test]
    fn top5_ratio_concentration() {
        // 总 = 10；top-1 = 8 ⇒ top5（不足5取全）= 10/10 = 1.0。
        let r = top_n_pnl_ratio(&[8.0, 1.0, 1.0], 5);
        assert!((r - 1.0).abs() < 1e-9);
        // 总 = 10；top-2 = 8+1=9 ⇒ 0.9。
        let r2 = top_n_pnl_ratio(&[8.0, 1.0, 0.5, 0.5], 2);
        assert!((r2 - 0.9).abs() < 1e-9);
    }

    /// Sharpe 符号：正均值 → 正 Sharpe；零波动 → 0（防 NaN）。
    #[test]
    fn sharpe_sign_and_zero_vol() {
        let (s, _) = sharpe_sortino(&[0.01, 0.02, 0.01, 0.015]);
        assert!(s > 0.0, "正均值 returns → 正 Sharpe");
        let (s0, so0) = sharpe_sortino(&[0.01, 0.01, 0.01]);
        assert_eq!(s0, 0.0, "零波动 → Sharpe 0（防 NaN）");
        assert_eq!(so0, 0.0, "无下行 → Sortino 0");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  统计显著性检验（§3.4 block bootstrap + §4 随机入场对照）
    //  认识论：合成 pnl 上跑 = L1（验证检验管线正确）。L2 只在真实回测交易序列时产生。
    // ──────────────────────────────────────────────────────────────────────

    /// SplitMix64 确定性可复现（同 seed ⟹ bit-exact 同序列，§4 seed 冻结要求）。
    #[test]
    fn splitmix64_deterministic_reproducible() {
        let mut a = SplitMix64::new(PREREG_SEED);
        let mut b = SplitMix64::new(PREREG_SEED);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64(), "同 seed ⟹ bit-exact 同序列");
        }
        // 不同 seed ⟹ 不同序列（PRNG 非常数）。
        let mut c = SplitMix64::new(PREREG_SEED + 1);
        let mut d = SplitMix64::new(PREREG_SEED);
        assert_ne!(c.next_u64(), d.next_u64(), "不同 seed ⟹ 不同序列");
    }

    /// next_below 落在 [0, n) 且均匀覆盖（确定性）。
    #[test]
    fn splitmix64_next_below_in_range() {
        let mut rng = SplitMix64::new(PREREG_SEED);
        let n = 7;
        let mut seen = [false; 7];
        for _ in 0..1000 {
            let v = rng.next_below(n);
            assert!(v < n, "next_below 落在 [0,{n})");
            seen[v] = true;
        }
        assert!(seen.iter().all(|&s| s), "1000 抽样覆盖全部 7 个值（均匀性弱见证）");
    }

    /// significance 可复现（同输入同 seed ⟹ bit-exact 同结果，§4 硬约束）。
    #[test]
    fn significance_reproducible() {
        let pnls: Vec<f64> = (0..50).map(|i| if i % 3 == 0 { -1.0 } else { 2.0 }).collect();
        let rets: Vec<f64> = (0..50).map(|i| 0.001 * (i % 5) as f64 - 0.001).collect();
        let s1 = significance(&pnls, &rets);
        let s2 = significance(&pnls, &rets);
        assert_eq!(s1, s2, "同输入同 seed ⟹ bit-exact 同结果（可复现）");
    }

    /// ★否证性见证①：**纯亏损序列** ⟹ bootstrap p 值 ≈ 1.0（无法拒绝 H0 收益≤0）。
    /// 这是统计检验**能产生否定性结果**的见证——亏损策略被正确判为不显著（§5.1 失败）。
    #[test]
    fn significance_losing_strategy_rejected() {
        // 全亏损交易（每笔 -1.0）。
        let pnls = vec![-1.0; 40];
        let rets = vec![-0.002; 40];
        let s = significance(&pnls, &rets);
        // H0：收益≤0 无法拒绝（p 值高）——所有重采样总收益都 <0 ⟹ p=1.0。
        assert!(
            s.boot_pvalue_pnl_le_0 > 0.95,
            "纯亏损 ⟹ bootstrap 总收益恒≤0 ⟹ p≈1.0（无法拒绝 H0），实得 {}",
            s.boot_pvalue_pnl_le_0
        );
        // 置信区间整体在负区（亏损分布）。
        assert!(s.boot_ci95_hi < 0.0, "纯亏损 ⟹ CI 上界<0");
    }

    /// ★否证性见证②：**强正收益序列** ⟹ bootstrap p 值 ≈ 0（拒绝 H0，收益显著>0）。
    /// 与亏损见证对偶——检验能正确**确认**显著盈利（不是只会说"不显著"）。
    #[test]
    fn significance_winning_strategy_confirmed() {
        // 全盈利交易（每笔 +2.0）。
        let pnls = vec![2.0; 40];
        let rets = vec![0.003; 40];
        let s = significance(&pnls, &rets);
        // 所有重采样总收益都 >0 ⟹ p=0（拒绝 H0，收益显著正）。
        assert!(
            s.boot_pvalue_pnl_le_0 < 0.05,
            "纯盈利 ⟹ bootstrap 总收益恒>0 ⟹ p≈0（拒绝 H0），实得 {}",
            s.boot_pvalue_pnl_le_0
        );
        assert!(s.boot_ci95_lo > 0.0, "纯盈利 ⟹ CI 下界>0");
    }

    /// 空 / 单笔交易 ⟹ 退化（p=1.0，不冒充显著性，§5.5 inconclusive）。
    #[test]
    fn significance_degenerate_no_trades() {
        let s0 = significance(&[], &[]);
        assert_eq!(s0.boot_pvalue_pnl_le_0, 1.0, "无交易 ⟹ p=1.0（无证据，不冒充）");
        assert!(!s0.theta_beats_random, "无交易 ⟹ 不优于随机");
        let s1 = significance(&[5.0], &[0.01]);
        assert_eq!(s1.boot_pvalue_pnl_le_0, 1.0, "仅 1 笔 ⟹ 无序列结构 ⟹ p=1.0");
    }

    /// 随机入场对照：恒定 pnl（每笔相同）⟹ 随机重抽总收益恒等 ⟹ Θ 不优于随机（分位 0，
    /// 因 Θ total == 所有随机 total，无一严格小于）。这见证「无择时差异 ⟹ 不优于随机」。
    #[test]
    fn random_control_constant_pnl_no_edge() {
        let pnls = vec![1.0; 30]; // 每笔相同 ⟹ 任意重抽 30 笔总收益恒 = 30.0
        let rets = vec![0.001; 30];
        let s = significance(&pnls, &rets);
        // Θ total=30，所有随机 total=30 ⟹ 无一 < 30 ⟹ 分位=0 ⟹ 不优于随机。
        assert_eq!(s.rand_percentile_of_theta, 0.0, "恒定 pnl ⟹ Θ 不严格优于随机重抽");
        assert!(!s.theta_beats_random, "无择时差异 ⟹ 不优于随机（§5.3）");
    }

    /// Sharpe Lo 2002 标准误：正自相关 ⟹ SE 放大（调整因子 >1）；CI 含 sharpe。
    #[test]
    fn sharpe_lo2002_autocorr_inflates_se() {
        // 强正自相关序列（趋势性）。
        let rets: Vec<f64> = (0..50).map(|i| 0.001 + 0.0001 * (i as f64)).collect();
        let (sharpe, _) = sharpe_sortino(&rets);
        let (se, lo, hi) = sharpe_lo2002(&rets, sharpe);
        assert!(se > 0.0, "有波动 ⟹ SE>0");
        assert!(lo < sharpe && sharpe < hi, "CI 含点估计");
        // 与 IID SE 对比：正自相关 ⟹ Lo SE ≥ IID SE。
        let se_iid = ((1.0 + sharpe * sharpe / 2.0) / rets.len() as f64).sqrt();
        let rho1 = lag1_autocorr(&rets);
        if rho1 > 0.0 {
            assert!(se >= se_iid * 0.999, "正自相关 ⟹ Lo SE ≥ IID SE");
        }
    }
}
