//! beta 去污三态判据评估层（acc-alpha 预注册 §3，冻结判据的可执行落点）。
//!
//! **档处置（连带枚举·断粮定档，`chanlun/escalate/chi-line-falsification-ruling-20260728.md` §1②
//! 连带段，2026-07-28）**：本模块三态判据消费 `perm_test.rs` 生产的逐桶 `perm_p`——该生产者已随
//! 档2-修砍单撤销，本模块推断链随之断粮，一并定档。诊断件保留（禁删）；历史裁决照旧有效。登记详见
//! `chanlun/review-results/prob-inference-disposition-registry-20260728.md`。
//!
//! ## 存在论位置
//!
//! 本模块是 acc-alpha acceptance 工位「beta 去污后 LCB>0 占比」判据的**纯逻辑内核**。
//! 消费逐桶标量 `(μ̂, LCB, UCB, perm_p, n_eff, CV)`，产出三态 `AlphaState`，再聚合为全局
//! `AcceptanceVerdict`。判据常量在 `.chanlun/review-results/acc-alpha-estimand-prereg-20260701.md`
//! §4 冻结（`z_α=1.645`、`perm_α=0.05`、功效门 `n_eff≥(1.645·CV)²`）。
//!
//! ## 认识论等级（231号，强制标注）
//!
//! 本模块 **L0**（纯代数：三态判据是冻结常量与逐桶标量的确定性函数，零 L2 信息增量）。
//! 判据施加于**真实数据产出的** `(μ̂, LCB, UCB, perm_p, n_eff)` 时结论才是 L2/L3——
//! 那些标量由（当前阻塞的）Π_full 回测跑批 + 置换生产。本模块只把判据形式化，不产出数据。
//!
//! ## 为何**现在**写（预注册纪律，非过度构建）
//!
//! 预注册 §0 要求判据在「看任何本轮 Π_full L2 回测结果之前」冻结（防数据挖掘）。回测跑完
//! 后再写判据 = 看了数据再定判据 = 预注册禁止的事后改判据。故三态判据必须先于结果实装。
//!
//! ## 阻塞边界（诚实声明）
//!
//! 本模块**不含** `perm_p` 生产者。置换（路径 A 分层内 δ-shuffle / 路径 B 跨标的独立因子）
//! 需**逐笔** `(class, X_γ)` 原始记录，而 [`super::mu_estimator::MuEstimator`] 只存 Welford
//! 聚合量（n/mean/m2），拿不到逐笔数据 ⟹ `perm_p` 真阻塞于回测跑批（依赖 acc-classification
//! + acc-P2 落地）。本层把 `perm_p` 作为**输入标量**接收——待回测产出逐笔记录后由跑批层填。
//!
//! ponytail: perm_p 生产者留空，回测入口产出逐笔 (class,X_γ) 后在跑批层实装置换（Fisher-Yates
//! 200 次固定种子，perm_p = 置换 μ̂ ≥ 观测 μ̂ 的占比）；本层的三态逻辑届时零改动直接消费。

/// 逐桶 `z=(ℓ,δ,bsp_class)` 的 alpha 三态（预注册 §3.1，**非二态**——underpowered 不判纯 beta）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaState {
    /// 超 beta 可交易 alpha：`μ̂>0 ∧ perm_p<perm_α（去污显著）∧ LCB>0 ∧ powered`。
    Validated,
    /// 纯 beta / 无 alpha：`powered ∧ LCB≤0 ∧ UCB≤0`——有功效地判定 μ≤0，照实 161。
    Falsified,
    /// 未认证：`¬powered` **或** `LCB≤0<UCB`——判据无检出力，`LCB≤0 ⊬ μ≤0`（667/231）。
    /// 不判纯 beta、不落 161（把 underpowered 的 LCB≤0 当纯 beta = 231 否定膨胀）。
    Inconclusive,
}

/// 全局 acceptance 裁决（预注册 §3.2，逐桶三态聚合）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptanceVerdict {
    /// 存在超 beta 可交易 alpha：VALIDATED 占比 > 0。
    Pass,
    /// 纯 beta，照实 161：**所有**桶 ∈ FALSIFIED（powered ∧ UCB≤0）。
    Falsified,
    /// 存在 INCONCLUSIVE 桶且无 VALIDATED：不 PASS 不 161，走 §3.3 提功效重估。
    Inconclusive,
}

/// 事件聚集/自相关校正后的有效样本数（预注册 §1.3，665 号 `neff/nraw` 口径）。
///
/// `n_eff = n / τ`，`τ = 1 + 2·Σ_{k≥1}ρk`（积分自相关时间）。Σρk 用**标准 Geyer 配对初始
/// 正序列（IPS）**估计：配对自相关 `Γ_m = ρ_{2m}+ρ_{2m+1}`（ρ_0=1），在**首个 `Γ_m≤0` 处
/// 截断**。配对（而非单 lag `ρk≤0` 截断）防止中段单个噪声负 ρk 提前掐断仍显著为正的尾部——
/// 单 lag 法在该情形**低估 Σρk ⟹ 高估 n_eff**，对 VALIDATED 判定不保守；配对法捕获更多正尾，
/// 是不利于 VALIDATED 方向的修正（codex-r2-audit Q1）。ρk 分母用全序列离差平方和（标准 ESS
/// 口径）。反相关（`τ<1`）clamp 到 `τ=1 ⟹ n_eff≤n`：同方向信号的时间聚集只**降低**有效样本，
/// 不凭空增加（预注册 n_eff≤n 口径）。lag 上限 = `⌊n/2⌋`（bandwidth 保护：每个 ρk 至少
/// `⌊n/2⌋` 个样本对，挡远 lag 少样本对的噪声）。常数序列/`n<2` 无自相关信息 ⟹ `n_eff=n`；
/// 含非有限值 ⟹ `NaN` 上浮（下游 `powered()` 判 false ⟹ Inconclusive，不静默膨胀 n_eff）。
/// 序列须按**成交时间序**传入（wverify_run 逐 bar 因果收集）。
pub fn effective_n(x: &[f64]) -> f64 {
    let n = x.len();
    if n < 2 {
        return n as f64;
    }
    if x.iter().any(|v| !v.is_finite()) {
        return f64::NAN; // 入口校验：非有限值无法估计自相关，上浮而非静默退回 n
    }
    let mean = x.iter().sum::<f64>() / n as f64;
    let c0: f64 = x.iter().map(|v| (v - mean).powi(2)).sum();
    if c0 == 0.0 {
        return n as f64; // 常数序列无自相关
    }
    let max_lag = n / 2; // bandwidth 上限：每个 ρk 至少 ⌊n/2⌋ 个样本对
    let rho = |k: usize| -> f64 {
        (0..n - k)
            .map(|i| (x[i] - mean) * (x[i + k] - mean))
            .sum::<f64>()
            / c0
    };
    let sum_rho = geyer_paired_sum(rho, max_lag);
    let tau = (1.0 + 2.0 * sum_rho).max(1.0); // clamp：反相关不增有效样本（n_eff≤n）
    n as f64 / tau
}

/// Geyer 配对初始正序列的 `Σ_{k≥1}ρk`：`Γ_m = ρ_{2m}+ρ_{2m+1}`（`rho(0)=1`），首个 `Γ_m≤0`
/// 或无法凑成完整配对（`2m+1>max_lag`）时截断。只累加 **k≥1** 部分（ρ_0=1 已归入 τ 的常数项）。
fn geyer_paired_sum(rho: impl Fn(usize) -> f64, max_lag: usize) -> f64 {
    let mut sum_rho = 0.0_f64;
    let mut m = 0usize;
    loop {
        let (lo, hi) = (2 * m, 2 * m + 1);
        if hi > max_lag {
            break; // 只累加完整配对，末尾单 lag 不入（避免少样本对噪声）
        }
        let (r_lo, r_hi) = (rho(lo), rho(hi)); // rho(0)=1
        if r_lo + r_hi <= 0.0 {
            break; // 初始正序列：首个非正配对处截断
        }
        if lo >= 1 {
            sum_rho += r_lo; // m=0 时 lo=0 是 ρ_0=1，不入 Σ_{k≥1}
        }
        sum_rho += r_hi; // hi=2m+1≥1 恒成立
        m += 1;
    }
    sum_rho
}

/// 功效门槛（预注册 §4 / 667）：`n_eff ≥ (z_α · CV)²`。`n_eff` 为 `effective_n` 的自相关校正量。
///
/// `CV`（变异系数 = std/|mean|）刻画桶内相对噪声；`n_eff` 是桶有效样本数。样本不足以把
/// `z_α` 倍相对噪声压到均值量级下 ⟹ ¬powered ⟹ `LCB≤0` 是无检出力的必然结果，不判纯 beta。
pub fn powered(n_eff: f64, cv: f64, z_alpha: f64) -> bool {
    let threshold = (z_alpha * cv).powi(2);
    n_eff >= threshold
}

/// 逐桶三态判定（预注册 §3.1）。`perm_p` 为去污置换 p 值（路径 A/B 产出）；`z_alpha=1.645`、
/// `perm_alpha=0.05` 为 §4 冻结常量。判定顺序严格照冻结表——先功效门，再三态。
#[allow(clippy::too_many_arguments)]
pub fn classify_bucket(
    mu_hat: f64,
    lcb: f64,
    ucb: f64,
    perm_p: f64,
    n_eff: f64,
    cv: f64,
    z_alpha: f64,
    perm_alpha: f64,
) -> AlphaState {
    let powered = powered(n_eff, cv, z_alpha);
    if powered && mu_hat > 0.0 && perm_p < perm_alpha && lcb > 0.0 {
        AlphaState::Validated
    } else if powered && lcb <= 0.0 && ucb <= 0.0 {
        AlphaState::Falsified
    } else {
        // ¬powered，或 powered 但 LCB≤0<UCB（跨零，判据无检出力）。
        AlphaState::Inconclusive
    }
}

/// 全局裁决（预注册 §3.2）：任一 VALIDATED ⟹ Pass；否则任一 INCONCLUSIVE ⟹ Inconclusive；
/// 否则（全 FALSIFIED）⟹ Falsified。空桶集 ⟹ Inconclusive（无证据，不 161）。
pub fn global_verdict(states: &[AlphaState]) -> AcceptanceVerdict {
    if states.iter().any(|s| *s == AlphaState::Validated) {
        AcceptanceVerdict::Pass
    } else if states.is_empty() || states.iter().any(|s| *s == AlphaState::Inconclusive) {
        AcceptanceVerdict::Inconclusive
    } else {
        AcceptanceVerdict::Falsified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const Z: f64 = 1.645;
    const PA: f64 = 0.05;

    /// 功效门：n_eff 恰达 (z·CV)² 过门，差一失守（CV=10 ⟹ 门槛≈270.6）。
    #[test]
    fn powered_gate_at_threshold() {
        let cv = 10.0;
        let thr = (Z * cv).powi(2); // ≈270.6
        assert!(powered(thr.ceil(), cv, Z));
        assert!(!powered(thr.floor() - 1.0, cv, Z));
    }

    /// VALIDATED：powered ∧ μ̂>0 ∧ 去污显著 ∧ LCB>0。
    #[test]
    fn validated_requires_all_four() {
        let s = classify_bucket(0.5, 0.1, 0.9, 0.01, 500.0, 10.0, Z, PA);
        assert_eq!(s, AlphaState::Validated);
        // LCB≤0 ⟹ 掉出 VALIDATED（此处跨零 ⟹ INCONCLUSIVE）。
        let s2 = classify_bucket(0.5, -0.1, 0.9, 0.01, 500.0, 10.0, Z, PA);
        assert_eq!(s2, AlphaState::Inconclusive);
        // 去污不显著（perm_p≥α）⟹ 非 VALIDATED（LCB>0 但 UCB>0 ⟹ 跨零 INCONCLUSIVE）。
        let s3 = classify_bucket(0.5, 0.1, 0.9, 0.20, 500.0, 10.0, Z, PA);
        assert_eq!(s3, AlphaState::Inconclusive);
    }

    /// FALSIFIED 必须 powered——underpowered 的 LCB≤0∧UCB≤0 判 INCONCLUSIVE（667 核心）。
    #[test]
    fn falsified_requires_powered_not_just_lcb_le_zero() {
        // powered ∧ LCB≤0 ∧ UCB≤0 ⟹ FALSIFIED（有功效判 μ≤0）。
        let s = classify_bucket(-0.5, -0.9, -0.1, 0.5, 500.0, 10.0, Z, PA);
        assert_eq!(s, AlphaState::Falsified);
        // 同符号但 ¬powered（n_eff 不足）⟹ INCONCLUSIVE，**不判纯 beta**（不落 161）。
        let s2 = classify_bucket(-0.5, -0.9, -0.1, 0.5, 3.0, 10.0, Z, PA);
        assert_eq!(s2, AlphaState::Inconclusive);
    }

    /// powered 但 UCB>0（跨零）⟹ INCONCLUSIVE，非 FALSIFIED（LCB≤0<UCB）。
    #[test]
    fn straddle_zero_is_inconclusive_not_falsified() {
        let s = classify_bucket(-0.1, -0.5, 0.3, 0.5, 500.0, 10.0, Z, PA);
        assert_eq!(s, AlphaState::Inconclusive);
    }

    /// 全局裁决：VALIDATED 优先；无 VALIDATED 但有 INCONCLUSIVE ⟹ Inconclusive；全 FALSIFIED ⟹ Falsified。
    #[test]
    fn global_verdict_priority() {
        use AlphaState::*;
        assert_eq!(
            global_verdict(&[Validated, Falsified, Inconclusive]),
            AcceptanceVerdict::Pass
        );
        assert_eq!(
            global_verdict(&[Falsified, Inconclusive]),
            AcceptanceVerdict::Inconclusive
        );
        assert_eq!(
            global_verdict(&[Falsified, Falsified]),
            AcceptanceVerdict::Falsified
        );
        // 空桶集 ⟹ Inconclusive（无证据不 161）。
        assert_eq!(global_verdict(&[]), AcceptanceVerdict::Inconclusive);
    }

    /// effective_n：短序列/常数退回 n；正相关聚集 ⟹ n_eff<n；反相关 ⟹ clamp τ=1 ⟹ n_eff=n；
    /// 非有限值 ⟹ NaN 上浮。
    #[test]
    fn effective_n_autocorr_correction() {
        assert_eq!(effective_n(&[]), 0.0);
        assert_eq!(effective_n(&[5.0]), 1.0);
        assert_eq!(effective_n(&[3.0, 3.0, 3.0, 3.0]), 4.0); // 常数：c0=0 退回 n
        let ramp: Vec<f64> = (0..40).map(|i| i as f64).collect();
        assert!(effective_n(&ramp) < 40.0); // 强正自相关 ⟹ Σρk>0 ⟹ n_eff 显著缩水
        let alt: Vec<f64> = (0..40)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        assert!((effective_n(&alt) - 40.0).abs() < 1e-9); // 反相关 τ<1 ⟹ clamp τ=1 ⟹ n_eff=n
        assert!(effective_n(&[1.0, f64::NAN, 2.0, 3.0]).is_nan()); // 非有限值上浮 NaN
    }

    /// 保真度方向（codex-r2-audit Q1）：中段单个噪声负 ρk 场景，配对 IPS 的 Σρk ≥ 单 lag IPS——
    /// 配对不被中段噪声提前截断 ⟹ 捕获更多正尾 ⟹ 更大 Σρk ⟹ 更小 n_eff ⟹ 对 VALIDATED 更保守。
    #[test]
    fn geyer_paired_ge_single_lag_on_noise_dip() {
        // ρ_1..ρ_6（ρ_0=1 隐含）。ρ_2 为中段噪声负值，但 ρ_2+ρ_3>0（配对存活）；ρ_4+ρ_5<0（截断）。
        let rho_vals = [0.5, -0.05, 0.30, 0.10, -0.40, -0.30_f64];
        let rho = |k: usize| if k == 0 { 1.0 } else { rho_vals[k - 1] };
        let max_lag = rho_vals.len();
        // 单 lag 法：k=1 加 0.5，k=2 遇 -0.05≤0 立即截断 ⟹ Σρk=0.5。
        let single = {
            let mut s = 0.0_f64;
            for k in 1..=max_lag {
                let r = rho(k);
                if r <= 0.0 {
                    break;
                }
                s += r;
            }
            s
        };
        // 配对法：Γ_0=1+0.5>0 加 ρ_1；Γ_1=ρ_2+ρ_3=0.25>0 加 ρ_2+ρ_3；Γ_2=ρ_4+ρ_5=-0.30≤0 截断。
        let paired = geyer_paired_sum(rho, max_lag);
        assert!(
            (single - 0.5).abs() < 1e-12,
            "单 lag 截断于噪声 dip: {single}"
        );
        assert!(
            (paired - 0.75).abs() < 1e-12,
            "配对捕获正尾 ρ_1+ρ_2+ρ_3: {paired}"
        );
        assert!(
            paired >= single,
            "配对 Σρk ≥ 单 lag（保守方向）: {paired} vs {single}"
        );
    }
}
