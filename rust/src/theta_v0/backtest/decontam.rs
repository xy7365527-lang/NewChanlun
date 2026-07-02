//! beta 去污三态判据评估层（acc-alpha 预注册 §3，冻结判据的可执行落点）。
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
/// `n_eff = n / (1 + 2·Σρk)`，Σρk 用 Geyer 初始正序列估计（首个非正 ρk 处截断），
/// 保证 Σρk≥0 ⟹ n_eff≤n——同方向信号的时间聚集只会**降低**有效样本，不凭空增加。
/// ρk 为滞后 k 的样本自相关（分母用全序列离差平方和，标准 ESS 口径）；常数序列或
/// n<2 无自相关信息，退回 n_eff=n。序列须按**成交时间序**传入（wverify_run 逐 bar 因果收集）。
pub fn effective_n(x: &[f64]) -> f64 {
    let n = x.len();
    if n < 2 {
        return n as f64;
    }
    let mean = x.iter().sum::<f64>() / n as f64;
    let c0: f64 = x.iter().map(|v| (v - mean).powi(2)).sum();
    if c0 == 0.0 {
        return n as f64; // 常数序列无自相关
    }
    let mut sum_rho = 0.0_f64;
    for k in 1..n {
        let ck: f64 = (0..n - k).map(|i| (x[i] - mean) * (x[i + k] - mean)).sum();
        let rho = ck / c0;
        if rho <= 0.0 {
            break; // 初始正序列：首个非正自相关处截断，Σρk≥0
        }
        sum_rho += rho;
    }
    n as f64 / (1.0 + 2.0 * sum_rho)
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
        assert_eq!(global_verdict(&[Validated, Falsified, Inconclusive]), AcceptanceVerdict::Pass);
        assert_eq!(global_verdict(&[Falsified, Inconclusive]), AcceptanceVerdict::Inconclusive);
        assert_eq!(global_verdict(&[Falsified, Falsified]), AcceptanceVerdict::Falsified);
        // 空桶集 ⟹ Inconclusive（无证据不 161）。
        assert_eq!(global_verdict(&[]), AcceptanceVerdict::Inconclusive);
    }

    /// effective_n：短序列/常数退回 n；正相关聚集 ⟹ n_eff<n；反相关 ⟹ 首个非正处截断 ⟹ n_eff=n。
    #[test]
    fn effective_n_autocorr_correction() {
        assert_eq!(effective_n(&[]), 0.0);
        assert_eq!(effective_n(&[5.0]), 1.0);
        assert_eq!(effective_n(&[3.0, 3.0, 3.0, 3.0]), 4.0); // 常数：c0=0 退回 n
        let ramp: Vec<f64> = (0..40).map(|i| i as f64).collect();
        assert!(effective_n(&ramp) < 40.0); // 强正自相关 ⟹ Σρk>0 ⟹ n_eff 显著缩水
        let alt: Vec<f64> = (0..40).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }).collect();
        assert!((effective_n(&alt) - 40.0).abs() < 1e-9); // ρ1<0 立即截断 ⟹ Σρk=0
    }
}
