//! **跨品种 pooling + ICC 同质性检验 + leave-one-asset-out 迁移检验**
//! （第5份PDF《全互斥定义策略2》§15-§16，task #86 sg-pooling-icc）。
//!
//! **档处置（决策统计族·χ线撤销，`chanlun/escalate/chi-line-falsification-ruling-20260728.md` §1①，
//! 2026-07-28）**：本模块攻的 pooling 目标（延长 μ 估计样本量）随 μ/χ 门族一并撤销——不再作为准入
//! 机制的功效补强手段。诊断件保留（禁删）；历史裁决照旧有效。登记详见
//! `chanlun/review-results/prob-inference-disposition-registry-20260728.md`。
//!
//! ## 动机（§33 功效不足约束）
//!
//! 高级别稀疏类要功效充足，必须**延长样本 / 跨品种 pooling / 强先验**之一。本模块攻 pooling：
//! 把同一 z 类在多个品种 a 的观测合并（`n_pool = Σ_a n_{a,z}` ⟹ 标准误 √n 收缩）。但 pooling
//! **只在品种间同分布时有效**——异质则强 pooling 引偏差（§15）。故 pooling 前必须**检验同质性**。
//!
//! ## §15 随机效应模型（品种异质性的形式化）
//!
//! 同一 z 类，品种 a 的真均值 `μ_{a,z} = μ_z + η_{a,z}`，`η_{a,z} ~ N(0, τ_z²)`：
//! - `τ_z² = 0` ⟹ 所有品种同真均值 ⟹ **同分布** ⟹ pooling 有效（合并降方差，无偏差）。
//! - `τ_z² > 0` ⟹ 品种间真均值有散布 ⟹ **异质** ⟹ 强 pooling（完全合并）把异质均值拉平 ⟹ 引偏差。
//!
//! ## §16 ICC + leave-one-asset-out（同质性的可计算检验）
//!
//! **ICC（类内相关系数）** `ICC_z = τ_z² / (τ_z² + σ_z²)`，分解单笔观测 `X_{a,z,i} = μ_z + u_{a,z} + ε`：
//! - `τ_z²`（between-asset 方差）：品种均值 μ_{a,z} 围绕总均值 μ_z 的散布（异质性来源）。
//! - `σ_z²`（within-asset 方差）：单笔 X 围绕本品种均值 μ_{a,z} 的散布（噪声）。
//! - `ICC_z ∈ [0,1]`：→0 噪声主导（品种间无差异，pooling 安全）；→1 品种差异主导（异质，慎 pooling）。
//!
//! **leave-one-asset-out**：用其他品种估 μ_z^{(−a)}，目标品种 a 上 OOS 测迁移——稳 ⟹ pooling 可信；
//! 失败 ⟹ 拒绝跨该品种 pooling（§16 异质性的直接经验检验，非仅方差分量）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! | 组件 | 等级 | 理由 |
//! |------|------|------|
//! | [`icc`] / [`pooling_weight`] / [`pooled_mean`] 数值变换 | **L1** | 给定 per-asset 统计量求方差分解是确定性算法，验证管线正确，零信息增量 |
//! | 真实 8 品种 μ 表喂入产生的 ICC 值 / leave-one-out 迁移结果 | **L2** | 真实数据才能否证「某 z 类跨品种同质」（正信息增量，可拒绝 pooling） |
//!
//! 本模块的纯统计 fn 单测全部喂**合成** per-asset 统计量 ⟹ **L1**（验证方差分解算法无 bug，
//! 不验证任何 z 类在真实市场跨品种同质——合成数据的同质性是我造的，验证是同义反复，231号）。
//!
//! 真实数据驱动的 L2/L3 检验（[`icc_multi_symbol`]）`#[ignore]`，由 Lead 重跑。

use super::mu_estimator::{MuClass, MuEstimator};

/// 单品种单 z 类的统计量 `(n_{a,z}, mean_{a,z}, var_{a,z})`——ICC 方差分解的输入原子。
///
/// `var` 是**类内**样本方差（`m2/(n−1)`，无偏）；`n<2` ⟹ `var=None`（单样本类内方差未定义）。
/// 由 [`MuEstimator::iter_class_stats`] 提取（不碰 frozen selector / runner，只读）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssetClassStat {
    /// 品种内该 z 类样本量 n_{a,z}（≥1，否则不会出现在估计器桶里）。
    pub n: u64,
    /// 品种内该 z 类样本均值 μ_{a,z}（= μ_z + η_{a,z}，§15）。
    pub mean: f64,
    /// 品种内该 z 类无偏样本方差 σ²_{a,z}（`n<2` ⟹ None）。
    pub var: Option<f64>,
}

/// 单 z 类的 ICC 方差分解结果（§16）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IccResult {
    /// 进入分解的品种数（n_{a,z}≥1 的品种）。<2 ⟹ ICC 不可定义（无 between 散布可估）。
    pub n_assets: usize,
    /// 总样本量 Σ_a n_{a,z}（pooling 后的 n_pool）。
    pub n_total: u64,
    /// between-asset 方差 τ²_z 的矩估计（ANOVA 法，可能被截断到 0，见 [`icc`] 文档）。
    pub tau_sq: f64,
    /// within-asset 方差 σ²_z 的合并估计（按 (n_a−1) 加权各品种类内方差）。
    pub sigma_sq: f64,
    /// ICC_z = τ²/(τ²+σ²) ∈ [0,1]。τ²+σ²=0（全品种零方差且同均值）⟹ 0（无散布即无类内相关）。
    pub icc: f64,
}

/// **ICC 方差分解（§16，one-way random-effects ANOVA 矩估计）**。
///
/// 输入：同一 z 类在各品种的 [`AssetClassStat`] 列表。输出 [`IccResult`]。
///
/// ## 估计公式（标准随机效应 ANOVA 矩估计，Searle et al. 1992）
///
/// 设品种 a 有 `n_a` 笔、均值 `ȳ_a`、类内方差 `s²_a`。总笔数 `N=Σn_a`，品种数 `k`，
/// 加权总均值 `ȳ = (Σ n_a·ȳ_a)/N`。
/// - **组内均方** `MSW = (Σ_a (n_a−1)·s²_a) / (N−k)` = σ²_z 的无偏估计（合并类内方差）。
/// - **组间均方** `MSB = (Σ_a n_a·(ȳ_a−ȳ)²) / (k−1)` = σ² + n₀·τ² 的估计，其中
///   `n₀ = (N − Σn_a²/N)/(k−1)`（不等样本量的有效组大小）。
/// - `τ²_z = max(0, (MSB − MSW)/n₀)`（**截断到 0**——矩估计可负，负 τ² 无意义 ⟹ 视为同质 τ²=0，
///   §15「τ²=0 同分布」的边界，诚实标注非 bug）。
///
/// `ICC = τ²/(τ²+σ²)`。分母 0（全品种零类内方差且品种均值相同 ⟹ MSB=MSW=0）⟹ ICC=0
/// （无任何散布 ⟹ 无类内相关，安全 pooling）。
///
/// ## 边界（诚实声明）
/// - `n_assets < 2`：无 between 散布可估 ⟹ 返回 `tau_sq=0, sigma_sq=MSW(或0), icc=0`
///   （单品种无法谈"品种间"相关，ICC 定义上为 0；非"同质确认"而是"不可检验"，调用方须区分）。
/// - 仅 n=1 的品种（var=None）：贡献 between（其 ȳ_a 入 MSB）但不贡献 within（n_a−1=0 ⟹ 不入 MSW）。
///   全品种 n=1 ⟹ MSW 分母 N−k=0 ⟹ σ²=0（无类内信息）⟹ ICC=1（全散布归 between）。这是
///   样本饥饿的诚实信号（单笔无法分离 τ²/σ² ⟹ ICC 饱和），非异质性证据。
pub fn icc(stats: &[AssetClassStat]) -> IccResult {
    let k = stats.len();
    let n_total: u64 = stats.iter().map(|s| s.n).sum();
    if k < 2 {
        // 单品种：无品种间散布可估。σ² 取该品种类内方差（若有），τ²=0，ICC=0。
        let sigma_sq = stats.first().and_then(|s| s.var).unwrap_or(0.0);
        return IccResult {
            n_assets: k,
            n_total,
            tau_sq: 0.0,
            sigma_sq,
            icc: 0.0,
        };
    }
    let n = n_total as f64;
    let kf = k as f64;

    // 加权总均值 ȳ = Σ n_a·ȳ_a / N。
    let grand_mean = stats.iter().map(|s| s.n as f64 * s.mean).sum::<f64>() / n;

    // MSW = Σ(n_a−1)·s²_a / (N−k)；仅 n_a≥2 的品种有类内方差贡献。
    let ss_within: f64 = stats
        .iter()
        .filter_map(|s| s.var.map(|v| (s.n as f64 - 1.0) * v))
        .sum();
    let df_within = n - kf; // N−k
    let msw = if df_within > 0.0 {
        ss_within / df_within
    } else {
        0.0
    };

    // MSB = Σ n_a·(ȳ_a−ȳ)² / (k−1)。
    let ss_between: f64 = stats
        .iter()
        .map(|s| s.n as f64 * (s.mean - grand_mean).powi(2))
        .sum();
    let msb = ss_between / (kf - 1.0);

    // 有效组大小 n₀ = (N − Σn_a²/N)/(k−1)（不等样本量校正）。
    let sum_n_sq: f64 = stats.iter().map(|s| (s.n as f64).powi(2)).sum();
    let n0 = (n - sum_n_sq / n) / (kf - 1.0);

    // τ² = max(0, (MSB−MSW)/n₀)（矩估计截断；n₀=0 不可能因 k≥2 且 n_a≥1）。
    let tau_sq = if n0 > 0.0 {
        ((msb - msw) / n0).max(0.0)
    } else {
        0.0
    };
    let sigma_sq = msw;
    let denom = tau_sq + sigma_sq;
    let icc = if denom > 0.0 { tau_sq / denom } else { 0.0 };

    IccResult {
        n_assets: k,
        n_total,
        tau_sq,
        sigma_sq,
        icc,
    }
}

/// **收缩权重 w_{a,z}（§32：pooling 权重由样本数 + 异质性定）**。
///
/// `w = n_a·τ² / (n_a·τ² + σ²)` = James-Stein/经验贝叶斯收缩量：品种 a 的 pooled μ 估计 =
/// `w·ȳ_a + (1−w)·μ_pool`。直觉：
/// - `τ²→0`（同质）⟹ w→0 ⟹ 完全信 pooled（pooling 有效，§15）。
/// - `τ²` 大（异质）⟹ w→1 ⟹ 信本品种自身（拒绝 pooling，避免异质偏差）。
/// - `n_a` 大 ⟹ w→1 ⟹ 自身样本足，少借 pooled。
///
/// `σ²=0 ∧ τ²=0` ⟹ 分母 0 ⟹ 返回 0（无任何方差 ⟹ pooled 与自身相同，借 pooled 无害，取 0 保守）。
pub fn pooling_weight(n_a: u64, tau_sq: f64, sigma_sq: f64) -> f64 {
    let na = n_a as f64;
    let denom = na * tau_sq + sigma_sq;
    if denom > 0.0 {
        (na * tau_sq) / denom
    } else {
        0.0
    }
}

/// **跨品种 pooled 均值 μ_pool = Σ_a n_a·ȳ_a / Σ_a n_a**（按样本量加权，等价合并全部 X_γ 求均值）。
///
/// 与逐笔合并 bit-exact（`Σ n_a·ȳ_a = ΣΣX_γ`）。空输入或 Σn_a=0 ⟹ `None`（无样本无估计，
/// 不冒充 0，与 [`MuEstimator::mu`] 同诚实语义）。
pub fn pooled_mean(stats: &[AssetClassStat]) -> Option<f64> {
    let n_total: u64 = stats.iter().map(|s| s.n).sum();
    if n_total == 0 {
        return None;
    }
    let sum: f64 = stats.iter().map(|s| s.n as f64 * s.mean).sum();
    Some(sum / n_total as f64)
}

/// leave-one-asset-out 迁移检验单 z 类结果（§16）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeaveOneOutResult {
    /// 被留出（目标）品种在估计器列表中的下标。
    pub held_out: usize,
    /// 其他品种 pooled μ_z^{(−a)}（留一估计；其他品种无样本 ⟹ None）。
    pub mu_minus_a: Option<f64>,
    /// 目标品种自身均值 μ_{a,z}（OOS 真值；目标品种无样本 ⟹ None）。
    pub mu_held: Option<f64>,
    /// 迁移误差 |μ_z^{(−a)} − μ_{a,z}|（两者皆 Some 才有定义）。
    pub transfer_abs_err: Option<f64>,
}

/// **leave-one-asset-out：留出品种 `held_out`，用其余品种估 μ_z^{(−a)}，与目标品种自身均值比**。
///
/// §16：迁移误差小 ⟹ 跨品种 μ 稳定 ⟹ pooling 可信；误差大 ⟹ 目标品种偏离群体 ⟹ 异质 ⟹
/// 拒绝跨该品种 pooling。**不泄漏**：μ_z^{(−a)} **不含** held_out 品种的任何样本（留一是
/// 干净的——其他品种估，目标品种只做 OOS 真值对照，被估侧从不看目标侧）。
///
/// `held_out` 越界 ⟹ debug 断言失败（调用方传错下标 fail-fast）。
pub fn leave_one_asset_out(stats: &[AssetClassStat], held_out: usize) -> LeaveOneOutResult {
    debug_assert!(
        held_out < stats.len(),
        "held_out={held_out} 越界（{} 品种）",
        stats.len()
    );
    // 其余品种（排除 held_out）pooled 均值——干净，不含目标品种样本（无泄漏）。
    let others: Vec<AssetClassStat> = stats
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != held_out)
        .map(|(_, s)| *s)
        .collect();
    let mu_minus_a = pooled_mean(&others);
    let mu_held = stats.get(held_out).map(|s| s.mean);
    let transfer_abs_err = match (mu_minus_a, mu_held) {
        (Some(m), Some(h)) => Some((m - h).abs()),
        _ => None,
    };
    LeaveOneOutResult {
        held_out,
        mu_minus_a,
        mu_held,
        transfer_abs_err,
    }
}

/// 把多品种 [`MuEstimator`] 重组为 **z → 各品种 [`AssetClassStat`]** 映射（ICC/leave-one-out 输入）。
///
/// `ests` 是 `(symbol, MuEstimator)` 列表（每品种一个 walk-forward μ 表）。输出按 z 聚合：每个
/// z 类收集所有**有该类观测**的品种统计量（无该类的品种不入列表 ⟹ ICC 的 n_assets 只数有样本品种）。
///
/// 顺序：z 的遍历顺序不定（HashMap），但每个 z 的 AssetClassStat 列表按 `ests` 输入顺序——
/// leave_one_asset_out 的 `held_out` 下标据此对齐 `ests` 中的品种顺序（过滤掉无样本品种后的相对序）。
pub fn collect_by_class<'a>(
    ests: &'a [(&'a str, MuEstimator)],
) -> std::collections::HashMap<MuClass, Vec<(&'a str, AssetClassStat)>> {
    let mut by_class: std::collections::HashMap<MuClass, Vec<(&str, AssetClassStat)>> =
        std::collections::HashMap::new();
    for (sym, est) in ests {
        for (z, n, mean, var) in est.iter_class_stats() {
            by_class
                .entry(z)
                .or_default()
                .push((sym, AssetClassStat { n, mean, var }));
        }
    }
    by_class
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 同质（τ²=0）：所有品种同真均值 + 同类内方差 ⟹ ICC→0（噪声主导，pooling 安全）。
    /// 构造：3 品种均 mean=10，类内 var=4（n=50 ⟹ MSB 仅 √n 噪声，τ² 矩估计应 ≈0 或截断 0）。
    #[test]
    fn icc_zero_when_homogeneous() {
        let stats = vec![
            AssetClassStat {
                n: 50,
                mean: 10.0,
                var: Some(4.0),
            },
            AssetClassStat {
                n: 50,
                mean: 10.0,
                var: Some(4.0),
            },
            AssetClassStat {
                n: 50,
                mean: 10.0,
                var: Some(4.0),
            },
        ];
        let r = icc(&stats);
        // 品种均值完全相同 ⟹ MSB=0 ⟹ τ²=max(0, (0−MSW)/n0)=0（截断）⟹ ICC=0。
        assert_eq!(r.tau_sq, 0.0, "品种均值同 ⟹ τ²=0（同质）");
        assert_eq!(r.icc, 0.0, "τ²=0 ⟹ ICC=0（pooling 安全）");
        assert!((r.sigma_sq - 4.0).abs() < 1e-9, "σ²=合并类内方差=4");
        assert_eq!(r.n_total, 150);
    }

    /// 异质（τ²>0）：品种均值大幅散布、类内方差小 ⟹ ICC→1（品种差异主导，慎 pooling）。
    /// 3 品种 mean={0,50,100}（大 between 散布）、var=1（小 within）⟹ ICC 接近 1。
    #[test]
    fn icc_high_when_heterogeneous() {
        let stats = vec![
            AssetClassStat {
                n: 100,
                mean: 0.0,
                var: Some(1.0),
            },
            AssetClassStat {
                n: 100,
                mean: 50.0,
                var: Some(1.0),
            },
            AssetClassStat {
                n: 100,
                mean: 100.0,
                var: Some(1.0),
            },
        ];
        let r = icc(&stats);
        assert!(
            r.tau_sq > 0.0,
            "品种均值大散布 ⟹ τ²>0（异质），实得 {}",
            r.tau_sq
        );
        assert!(
            r.icc > 0.9,
            "small within + large between ⟹ ICC→1，实得 {}",
            r.icc
        );
        assert!((r.sigma_sq - 1.0).abs() < 1e-9, "σ²=1（小类内方差）");
    }

    /// ICC 恒 ∈ [0,1]（任意输入，τ²/σ²≥0 ⟹ 比值有界）——边界硬约束。
    #[test]
    fn icc_in_unit_interval() {
        let cases = vec![
            vec![
                AssetClassStat {
                    n: 3,
                    mean: -100.0,
                    var: Some(0.5),
                },
                AssetClassStat {
                    n: 7,
                    mean: 200.0,
                    var: Some(50.0),
                },
                AssetClassStat {
                    n: 1,
                    mean: 0.0,
                    var: None,
                },
            ],
            vec![
                AssetClassStat {
                    n: 2,
                    mean: 1.0,
                    var: Some(0.0),
                },
                AssetClassStat {
                    n: 2,
                    mean: 1.0,
                    var: Some(0.0),
                },
            ],
        ];
        for stats in &cases {
            let r = icc(stats);
            assert!(
                (0.0..=1.0).contains(&r.icc),
                "ICC∈[0,1]，实得 {} for {stats:?}",
                r.icc
            );
            assert!(r.tau_sq >= 0.0, "τ²≥0（截断），实得 {}", r.tau_sq);
            assert!(r.sigma_sq >= 0.0, "σ²≥0，实得 {}", r.sigma_sq);
        }
    }

    /// 单品种（n_assets<2）：无品种间散布可估 ⟹ ICC=0、τ²=0、σ²=该品种类内方差。
    #[test]
    fn icc_single_asset_undefined_between() {
        let r = icc(&[AssetClassStat {
            n: 10,
            mean: 5.0,
            var: Some(2.0),
        }]);
        assert_eq!(r.n_assets, 1);
        assert_eq!(r.tau_sq, 0.0, "单品种无 between 散布");
        assert_eq!(r.icc, 0.0);
        assert_eq!(r.sigma_sq, 2.0, "σ²=该品种类内方差");
    }

    /// 收缩权重：τ²=0（同质）⟹ w=0（完全信 pooled）；τ² 大（异质）⟹ w→1（信自身）。
    #[test]
    fn pooling_weight_reflects_heterogeneity() {
        // 同质 τ²=0 ⟹ w=0（完全 pooling）。
        assert_eq!(
            pooling_weight(10, 0.0, 5.0),
            0.0,
            "τ²=0 ⟹ w=0 完全信 pooled"
        );
        // 大 τ² ⟹ w→1（信自身，拒 pooling）。
        let w_het = pooling_weight(10, 1000.0, 1.0);
        assert!(w_het > 0.99, "τ² 远大于 σ² ⟹ w→1，实得 {w_het}");
        // n_a 大 ⟹ w 上升（自身样本足）。
        let w_small_n = pooling_weight(2, 1.0, 1.0);
        let w_large_n = pooling_weight(100, 1.0, 1.0);
        assert!(
            w_large_n > w_small_n,
            "n_a↑ ⟹ w↑：{w_large_n} > {w_small_n}"
        );
        // 全零方差 ⟹ w=0（pooled 与自身同，借无害取保守）。
        assert_eq!(pooling_weight(5, 0.0, 0.0), 0.0);
        // w∈[0,1] 边界。
        assert!((0.0..=1.0).contains(&w_het) && (0.0..=1.0).contains(&w_small_n));
    }

    /// pooled 均值按样本量加权（= 合并全部 X_γ 求均值，bit-exact）。
    #[test]
    fn pooled_mean_sample_weighted() {
        let stats = vec![
            AssetClassStat {
                n: 1,
                mean: 10.0,
                var: None,
            },
            AssetClassStat {
                n: 3,
                mean: 30.0,
                var: Some(1.0),
            },
        ];
        // (1·10 + 3·30)/4 = 100/4 = 25。
        assert_eq!(pooled_mean(&stats), Some(25.0));
        assert_eq!(pooled_mean(&[]), None, "空输入 ⟹ None（无样本无估计）");
    }

    /// **leave-one-asset-out 不泄漏（核心约束）**：μ_z^{(−a)} 只用其他品种，不含目标品种样本。
    /// 构造：目标品种 mean=1000（极端偏离），其他品种 mean=10 ⟹ μ_{−a} 必 ≈10（不被 1000 污染）。
    #[test]
    fn leave_one_out_no_leakage() {
        let stats = vec![
            AssetClassStat {
                n: 100,
                mean: 1000.0,
                var: Some(1.0),
            }, // 目标（held_out=0）
            AssetClassStat {
                n: 50,
                mean: 10.0,
                var: Some(1.0),
            },
            AssetClassStat {
                n: 50,
                mean: 10.0,
                var: Some(1.0),
            },
        ];
        let r = leave_one_asset_out(&stats, 0);
        // μ_{−0} = (50·10 + 50·10)/100 = 10——完全不含目标品种的 1000（无泄漏铁证）。
        assert_eq!(
            r.mu_minus_a,
            Some(10.0),
            "μ_{{−a}} 只含其他品种，不被目标 1000 污染"
        );
        assert_eq!(r.mu_held, Some(1000.0), "目标自身均值 = OOS 真值");
        // 迁移误差 |10 − 1000| = 990（目标极端偏离群体 ⟹ pooling 该品种会大错）。
        assert_eq!(r.transfer_abs_err, Some(990.0));
    }

    /// leave-one-out 同质群体：所有品种同均值 ⟹ 迁移误差 ≈0（pooling 可信）。
    #[test]
    fn leave_one_out_homogeneous_low_error() {
        let stats = vec![
            AssetClassStat {
                n: 30,
                mean: 7.0,
                var: Some(2.0),
            },
            AssetClassStat {
                n: 30,
                mean: 7.0,
                var: Some(2.0),
            },
            AssetClassStat {
                n: 30,
                mean: 7.0,
                var: Some(2.0),
            },
        ];
        for held in 0..3 {
            let r = leave_one_asset_out(&stats, held);
            assert_eq!(
                r.transfer_abs_err,
                Some(0.0),
                "同质群体 ⟹ 迁移误差 0（held={held}）"
            );
        }
    }

    /// leave-one-out 目标品种是唯一有样本的 ⟹ μ_{−a}=None（其他品种全空，无可借估计）。
    #[test]
    fn leave_one_out_no_others() {
        let stats = vec![
            AssetClassStat {
                n: 10,
                mean: 5.0,
                var: Some(1.0),
            },
            AssetClassStat {
                n: 0,
                mean: 0.0,
                var: None,
            }, // 空品种（n=0）
        ];
        let r = leave_one_asset_out(&stats, 0);
        assert_eq!(r.mu_minus_a, None, "其他品种全无样本 ⟹ μ_{{−a}} None");
        assert_eq!(r.transfer_abs_err, None, "无 μ_{{−a}} ⟹ 迁移误差未定义");
    }

    /// collect_by_class：跨品种重组，同 z 聚合各品种统计；不同品种同 z 进同一列表。
    #[test]
    fn collect_groups_same_class_across_assets() {
        use crate::theta_v0::backtest::mu_estimator::{MuObservation, PositionState};
        use crate::theta_v0::types::BspBits;
        let z = MuClass::from_certificate(
            3,
            1,
            BspBits {
                buy1: true,
                ..Default::default()
            },
            0,
            PositionState::Root,
        );
        let mut est_a = MuEstimator::new();
        est_a.observe_all([
            MuObservation {
                class: z,
                x_gamma: 8.0,
            },
            MuObservation {
                class: z,
                x_gamma: 12.0,
            },
        ]); // mean=10, n=2
        let mut est_b = MuEstimator::new();
        est_b.observe(MuObservation {
            class: z,
            x_gamma: 20.0,
        }); // mean=20, n=1
        let ests = vec![("A", est_a), ("B", est_b)];
        let by = collect_by_class(&ests);
        let list = by.get(&z).expect("z 类存在");
        assert_eq!(list.len(), 2, "两品种同 z ⟹ 列表 2 项");
        // 验证能直接喂 icc（端到端：collect → icc）。
        let stats: Vec<AssetClassStat> = list.iter().map(|(_, s)| *s).collect();
        let r = icc(&stats);
        assert_eq!(r.n_assets, 2);
        assert_eq!(r.n_total, 3, "2+1");
        assert!((0.0..=1.0).contains(&r.icc));
    }
}
