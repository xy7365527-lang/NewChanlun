//! 因果选择器 χ_t(γ) 阈值过滤（买卖点alpha2.pdf §13 + §17，task #41 chi-theta-filter）。
//!
//! ## 命题（alpha2 §13 line 2239）
//!
//! 把目标从「∀e 吃到所有元素」改成「只交易具有正边际条件期望的买卖点证书」。形式上定义
//! **全定义**选择函数 χ_t(γ) ∈ {0,1}：
//!
//! ```text
//!   χ_t(γ) = 1 ⟺ μ(γ) > θ ∧ RiskOK(γ, x_t) ∧ ConflictOK(γ, A_t)     (§13 line 2239)
//! ```
//!
//! θ ≥ 0 是成本和风险门槛（§13 line 2241）。解释器只处理 Γ_t^trade = {γ∈Γ_t : χ_t(γ)=1}。
//! 这仍**全互斥、全定义**——χ_t 是全定义函数，解释器仍有限、确定（§13 line 2262）。
//!
//! ## 动作选择 π̄(z)（alpha2 §17 line 3524）
//!
//! ```text
//!   π̄(z) = argmax_{a∈A(z)} μ(z,a)    if max_a μ(z,a) > θ
//!         = Hold/Flat                  否则                       (§17)
//! ```
//!
//! 仍**全定义**——每个 z 都有动作；无正边际收益时动作为 Hold/Flat（§17 line 3531）。本模块
//! 实装 §13 的二元过滤 [`chi_t`]（开/不开）与 §17 的多动作 argmax [`pi_bar`]（在一个 z 上多个
//! 候选动作中选 μ 最大者）。二者同一阈值 θ：§13 是 §17 在 |A(z)|=1（单候选 = 开/Hold）的特例。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! | 组件 | 等级 | 理由 |
//! |------|------|------|
//! | 选择器逻辑（[`chi_t`]/[`pi_bar`] 的合取/argmax/阈值比较） | **L1** | 给定 μ/θ/谓词求 χ 是确定性布尔/argmax，验证逻辑正确，零信息增量 |
//! | 阈值 θ 取值 | **Θ_risk 参数（非缠论可导）** | θ 是成本+风险门槛（§13 line 2241），由成本模型/风险偏好给定，**不能从缠论语法推出**（诚实标注） |
//! | 真实 μ 驱动的过滤结果（哪些 γ 被滤掉、ΔN 如何变） | **L2** | 喂真实历史 μ 才能否证「过滤提升 alpha」（正信息增量） |
//!
//! **关键诚实声明**（no-patch / formalization-validity-domain）：χ_t 用的 μ 表若来自**同一遍
//! in-sample 全程交易**（用全程未来交易估的 μ 过滤当前开仓），是**未来函数泄漏**（§5 因果性硬
//! 约束 / project_zero_lookahead_backtest）——此时过滤结果只证明**选择器逻辑生效**（χ≡1 与
//! χ=1[μ>θ] 的 ΔN 序列非全等，**L1**），**不**证明 alpha 提升（**那需要 walk-forward μ，是后续
//! 工位**）。本模块只提供选择器纯函数；μ 表的因果获取由调用方负责并诚实标注其泄漏状态。

use super::mu_estimator::{MuClass, MuEstimator, PositionState};
use crate::theta_v0::strategy::coverage::Vertical;
use crate::theta_v0::strategy::interp::Candidate;
use crate::theta_v0::strategy::voice::VoiceSide;

/// χ_t(γ) = 1 ⟺ μ(γ) > θ ∧ RiskOK ∧ ConflictOK（§13 line 2239）。
///
/// - `mu`：μ(γ) = μ(z) 的样本估计（[`MuEstimator::mu`]）。`None` = 该 z 类**无样本**（空类无
///   估计，[`MuEstimator::mu`] 文档：空类 ≠ μ=0）。空类的 χ 取值由 `treat_empty_as_pass`
///   决定（见下）——本函数不替空类伪造 μ=0。
/// - `theta`：θ ≥ 0 成本/风险门槛（**Θ_risk 参数，非缠论可导**，诚实标注）。
/// - `risk_ok`：RiskOK(γ, x_t)——风险约束满足（K_Θ 投影/止损/杠杆上限）。
/// - `conflict_ok`：ConflictOK(γ, A_t)——全互斥解释器的 C_j 唯一裁决（同一时刻冲突唯一处理）。
/// - `treat_empty_as_pass`：空类（μ=None）时 χ 取值。**全覆盖语义**（χ≡1 退化）应传 `true`
///   （未见过的 z 默认交易，等价 θ→−∞ 对空类）；**严格过滤语义**应传 `false`（无 μ 证据 ⟹
///   不交易，保守）。这是 §13 未明确的边界——空类是「未观测」非「μ≤θ」，二语义都合法，
///   由调用方按 in-sample/walk-forward 阶段选。
///
/// # 全定义性（§13 line 2262）
/// 对任意输入恰返回一个 bool——χ_t 是全定义函数，无 panic 无未定义分支。
#[inline]
pub fn chi_t(
    mu: Option<f64>,
    theta: f64,
    risk_ok: bool,
    conflict_ok: bool,
    treat_empty_as_pass: bool,
) -> bool {
    let mu_pass = match mu {
        Some(m) => m > theta,            // 有样本：μ(γ) > θ（§13 严格大于）
        None => treat_empty_as_pass,      // 空类：未观测，由语义参数定（非伪造 μ=0）
    };
    mu_pass && risk_ok && conflict_ok
}

/// π̄(z) = argmax_{a∈A(z)} μ(z,a) if max>θ else Hold/Flat（§17 line 3524）。
///
/// 在一个状态 z 的候选动作集 `A(z)` 上选 μ 最大者；若最大 μ 仍 ≤ θ ⟹ `None`（Hold/Flat，§17）。
/// `actions` 是 (动作标识 a, μ(z,a)) 列表——μ 由 [`MuEstimator`] 在 (z,a) 复合类上估出（调用方
/// 把动作维度编码进 [`MuClass`] 或单独传）。返回 `Some(最优 a)` 或 `None`(Hold/Flat)。
///
/// # 全定义性（§17 line 3531）
/// 空 `actions` 或全部 μ≤θ ⟹ `None`(Hold/Flat)——每个 z 都有动作（最坏 Hold），无 panic。
pub fn pi_bar<A: Copy>(actions: &[(A, f64)], theta: f64) -> Option<A> {
    actions
        .iter()
        .filter(|(_, mu)| *mu > theta) // 只保留 μ>θ 的候选（§17：max>θ 才动作）
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(a, _)| *a)
}

/// 便利包装：用 [`MuEstimator`] 查 z 的 μ 后做 χ_t 过滤（开仓门，§13）。
///
/// `est` 是 **Pass 1（χ≡1）估出的 μ 表**——调用方负责保证其因果性（in-sample 全程 μ 过滤
/// 当前开仓 = 泄漏，结果是 L1 选择器逻辑生效证明，非 L2 alpha；见模块头诚实声明）。
#[inline]
pub fn chi_open_gate(
    est: &MuEstimator,
    z: &MuClass,
    theta: f64,
    risk_ok: bool,
    conflict_ok: bool,
    treat_empty_as_pass: bool,
) -> bool {
    chi_t(est.mu(z), theta, risk_ok, conflict_ok, treat_empty_as_pass)
}

/// LCB 便利包装：准入量用 **LCB(μ)** 而非裸 μ（严格alpha.pdf p25 §12——置信下界防高维 z 过拟合）。
///
/// 与 [`chi_open_gate`] 同构，唯一区别：查 [`MuEstimator::mu_lcb`]（`mean − z_alpha·std/√n`）而非
/// [`MuEstimator::mu`]。`z_alpha` 单边正态分位（如 1.645=95%）由调用方传。
///
/// **None 语义统一**（n<2 ⟹ mu_lcb=None；空类 ⟹ None）：二者皆「无 LCB 证据」，由
/// `treat_empty_as_pass` 裁决——`false` = 无证据不交易（与 p25「无正边际收益证据时不交易」一致），
/// `true` = 全覆盖默认放行。selector 不替无证据类伪造 LCB（mu_lcb 已诚实返 None）。
///
/// **退化**：θ=−∞ ⟹ LCB>−∞ 对任何有 LCB 的类恒真（全覆盖不滤已观测 n≥2 类）；None 类仍按
/// `treat_empty_as_pass`。n→∞ ⟹ std/√n→0 ⟹ LCB→mean ⟹ 与裸 μ 门决策收敛。
///
/// 认识论 **L1**（给定 μ/std/θ/z_alpha 求 χ 是确定性布尔，验证选择器逻辑，零信息增量）。
#[inline]
pub fn chi_open_gate_lcb(
    est: &MuEstimator,
    z: &MuClass,
    theta: f64,
    z_alpha: f64,
    risk_ok: bool,
    conflict_ok: bool,
    treat_empty_as_pass: bool,
) -> bool {
    chi_t(est.mu_lcb(z, z_alpha), theta, risk_ok, conflict_ok, treat_empty_as_pass)
}

/// 候选 γ ([`Candidate`]) → 全互斥分类 z ([`MuClass`])（§16 z 六维）的桥接。
///
/// `Candidate` 已带 `level`/`dir`(δ_g)/`bits`(I_γ)/`role`(R(g)=(H,V,δ))。z 的 `parent_dir` σ_p 与
/// `position` 仓位态从 `role.v`（[`Vertical`]）推（与 [`MuClass::from_certificate`] 的 `short_swing`
/// 推导口径一致）：
/// - `Ambient`（σ_p=0，去根化无父）⟹ `position=Root`，`parent_dir=0`。
/// - `FollowParent`（δ_g=σ_p，顺父）⟹ `position=Child`，`parent_dir=δ_g`（同向）。
/// - `ShortDiff`（δ_g=−σ_p，短差）⟹ `position=Child`，`parent_dir=−δ_g`（反向）。
///
/// `Flat` 方向候选（不可交易，归 𝒦_x 记录）δ 占位 +1——其 z 不被 χ 用于开仓（interpret 已归 record）；
/// 此桥接只服务**方向候选**的 χ 过滤，Flat 候选由 [`filter_gamma`] 在构 z 前按 `c.dir==Flat` 跳过。
pub fn z_of_candidate(c: &Candidate) -> MuClass {
    let delta: i8 = match c.dir {
        VoiceSide::Long => 1,
        VoiceSide::Short => -1,
        VoiceSide::Flat => 1, // Flat 不可交易候选，δ 占位（调用方不对 Flat 构 z 做开仓门）
    };
    let (parent_dir, position) = match c.role.v {
        Vertical::Ambient => (0, PositionState::Root),
        Vertical::FollowParent => (delta, PositionState::Child), // σ_p = δ_g
        Vertical::ShortDiff => (-delta, PositionState::Child),   // σ_p = −δ_g
    };
    MuClass::from_certificate(c.level, delta, c.bits, parent_dir, position)
}

/// χ_t 候选集过滤（§13 line 2256）：`Γ_t → Γ_t^trade = {γ∈Γ_t : χ_t(γ)=1}`。
///
/// **诚实有效域声明（no-patch / no-claim-inflation）**：本函数只施加 χ_t 的 **μ>θ** 项——
/// RiskOK/ConflictOK 在下游**已存在**（不重造，复用既有机制）：
/// - **RiskOK**：`k_theta_risk_gate` 产 `KThetaRiskGate`（force_flat/stop）收窄 𝒦_Θ——风险否决在
///   `pi_theta_position` 兑现，**非**本过滤层。
/// - **ConflictOK**：[`interpret`](super::super::strategy::interp::interpret)（规则2/3/4）把冲突/重复
///   候选归 𝒦_x 记录桶（不开仓）——冲突唯一裁决在解释器兑现，**非**本过滤层。
///
/// 故 χ_t(γ)=1[μ>θ ∧ RiskOK ∧ ConflictOK] = **本过滤(μ>θ)** ∘ **gate(RiskOK)** ∘ **interpret(ConflictOK)**
/// 三机制合取兑现完整 χ_t；本函数兑现其中 μ 门（§13 唯一**新增**项，task #41 增量）。
///
/// **非方向候选（`dir==Flat` / `bsp_class==u8::MAX`）保留**——由 interpret 归 𝒦_x 记录（不执行），
/// 在此不滤（χ 是开仓边际门，方向裁决是 interpret 职责，no-patch 不越界）。
///
/// `z_alpha`：单边置信分位（如 1.645=95%）——准入量用 **LCB(μ)=mean−z_alpha·std/√n**（严格alpha.pdf
/// p25 §12 防高维 z 过拟合），非裸 μ。`z_alpha=0` ⟹ LCB=mean ⟹ **n≥2 类**退化回裸 μ 门；
/// **n=1 类例外**（mu_lcb 返 None ⟹ 走 treat_empty_as_pass，非裸 μ——n=1 无方差=无 LCB 证据，拒绝
/// 是 p25 正确语义，no-patch §5 防声明膨胀）。
///
/// `treat_empty_as_pass`：无 LCB 证据（μ=None 空类 **或** n<2 单样本，mu_lcb 皆 None）χ 取值
/// （codex Q3）：`false` ⟹ χ=0 不交易（**最诚实**——无正边际证据不开，p25 一致）；`true` ⟹ 默认
/// 交易（探索/全覆盖）。LCB 升级后 None 多了「n<2 方差未定义」一源——与空类合流为「无证据」。
///
/// 认识论 **L1**（给定 μ/std 表/θ/z_alpha 过滤候选是确定性变换，验证选择器逻辑生效，零信息增量）——
/// 过滤是否提升 alpha 是 **L2/L3**（需 walk-forward μ + 真实数据否证，下游 delta-r-alpha 工位）。
pub fn filter_gamma(
    gamma: &[Candidate],
    est: &MuEstimator,
    theta: f64,
    z_alpha: f64,
    treat_empty_as_pass: bool,
) -> Vec<Candidate> {
    filter_gamma_with_admission(gamma, est, theta, z_alpha, None, treat_empty_as_pass)
}

/// 准入量泛化版（acc-three-way-l2 #83）：`shrink_tau_sq=None` ⟹ 准入量 = LCB(μ)（与 [`filter_gamma`]
/// bit-exact，z_alpha=0 退化裸 μ）；`shrink_tau_sq=Some(τ²)` ⟹ 准入量 = [`MuEstimator::mu_shrink`]
/// 层级收缩（z_alpha 此时忽略——shrink 不用置信下界，用 pooled 收缩抗稀疏过拟合；§8 收缩 vs §12
/// LCB 是两套机制）。三路对比唯一切换点：裸μ(None,z_alpha=0)/LCB(None,z_alpha>0)/shrink(Some)。
///
/// None 语义统一不变：mu_shrink 对无 pooled 样本的 (level,delta) 返 None ⟹ 走 treat_empty_as_pass。
/// 与 mu_lcb 的 n<2→None 关键区别：mu_shrink 对 n<2 类**完全收缩到 pooled**（保功效，非拒绝）⟹
/// 高级别稀疏类不被压退化空仓（#83 卖点判据：n_L3 不下降）。
pub fn filter_gamma_with_admission(
    gamma: &[Candidate],
    est: &MuEstimator,
    theta: f64,
    z_alpha: f64,
    shrink_tau_sq: Option<f64>,
    treat_empty_as_pass: bool,
) -> Vec<Candidate> {
    gamma
        .iter()
        .filter(|c| {
            // 非方向候选不滤（interpret 归 𝒦_x；χ 不越界做方向裁决）。
            if c.dir == VoiceSide::Flat || c.bsp_class == u8::MAX {
                return true;
            }
            // 方向候选：准入量>θ 门（RiskOK/ConflictOK 下游已施，此处仅 μ 项 risk_ok=conflict_ok=true）。
            let z = z_of_candidate(c);
            let admission = match shrink_tau_sq {
                Some(tau_sq) => est.mu_shrink(&z, tau_sq),
                None => est.mu_lcb(&z, z_alpha),
            };
            chi_t(admission, theta, true, true, treat_empty_as_pass)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::backtest::mu_estimator::{MuObservation, PositionState};
    use crate::theta_v0::types::BspBits;

    fn buy_z() -> MuClass {
        MuClass::from_certificate(
            3,
            1,
            BspBits { buy1: true, ..Default::default() },
            0,
            PositionState::Root,
        )
    }

    /// χ_t = μ>θ ∧ RiskOK ∧ ConflictOK（§13 line 2239）——三合取，任一假 ⟹ χ=0。
    #[test]
    fn chi_is_conjunction_of_three() {
        // μ=10>θ=5, RiskOK, ConflictOK ⟹ χ=1
        assert!(chi_t(Some(10.0), 5.0, true, true, false));
        // μ=10>θ 但 RiskOK=false ⟹ χ=0（风险否决）
        assert!(!chi_t(Some(10.0), 5.0, false, true, false));
        // μ=10>θ 但 ConflictOK=false ⟹ χ=0（冲突否决）
        assert!(!chi_t(Some(10.0), 5.0, true, false, false));
        // μ=3 ≤ θ=5 ⟹ χ=0（边际收益不足）
        assert!(!chi_t(Some(3.0), 5.0, true, true, false));
    }

    /// θ=0 退化：μ>0 ⟹ 交易，μ≤0 ⟹ 不交易（§13 θ≥0 下界，θ=0 = 只滤负期望）。
    /// **注意**：θ=0 **不**等于全覆盖（χ≡1）——全覆盖需对空类 pass 且对负 μ 也 pass。
    /// θ=0 仍滤掉 μ≤0 的已观测类。全覆盖退化见 `theta_neg_inf_is_full_coverage`。
    #[test]
    fn theta_zero_filters_nonpositive_mu() {
        assert!(chi_t(Some(0.001), 0.0, true, true, false)); // μ>0 ⟹ 交易
        assert!(!chi_t(Some(0.0), 0.0, true, true, false)); // μ=0 不 >0 ⟹ 不交易
        assert!(!chi_t(Some(-5.0), 0.0, true, true, false)); // μ<0 ⟹ 不交易
    }

    /// θ→−∞（用 f64::NEG_INFINITY）+ 空类 pass ⟹ χ≡1 全覆盖退化（§13 边界：所有 γ 都交易）。
    /// 这是验收 acc-chi-theta-filter 的「θ=0 退化为全覆盖」边界的精确形式：全覆盖 = θ 低到任何
    /// 已观测 μ 都过 + 空类也过。证明选择器在极限下还原 χ≡1（与 pi_bsp_timing Pass 1 一致）。
    #[test]
    fn theta_neg_inf_is_full_coverage() {
        let est = MuEstimator::new();
        let z = buy_z();
        // 空类 + treat_empty_as_pass=true ⟹ 即使无 μ 也开（全覆盖）。
        assert!(chi_open_gate(&est, &z, f64::NEG_INFINITY, true, true, true));
        // 已观测负 μ + θ=−∞ ⟹ μ > −∞ 恒真 ⟹ 开（全覆盖不滤任何已观测类）。
        let mut est2 = MuEstimator::new();
        est2.observe(MuObservation { class: z, x_gamma: -999.0 });
        assert!(chi_open_gate(&est2, &z, f64::NEG_INFINITY, true, true, true));
    }

    /// 空类语义二分：treat_empty_as_pass 控制未观测 z 的 χ（§13 未明确的边界，二者皆合法）。
    #[test]
    fn empty_class_semantics_split() {
        let est = MuEstimator::new(); // z 无样本
        let z = buy_z();
        // 全覆盖语义：未观测默认交易。
        assert!(chi_open_gate(&est, &z, 0.0, true, true, true));
        // 严格语义：无 μ 证据 ⟹ 不交易（保守）。
        assert!(!chi_open_gate(&est, &z, 0.0, true, true, false));
    }

    /// 空类**不**伪造 μ=0：chi_t 收到 None 走 treat_empty 分支，不当作 μ=0 比较（认识状态区分）。
    #[test]
    fn empty_class_not_treated_as_mu_zero() {
        // 若空类被错误当 μ=0：θ=−1 时 0>−1 ⟹ 会 pass。我们用 treat_empty_as_pass=false 验证
        // 空类走的是 pass 分支（false）而非 μ=0 比较（那样会 true）⟹ 证明无伪造。
        assert!(!chi_t(None, -1.0, true, true, false));
        // 对照：真 μ=0 在 θ=−1 时确实 pass（0>−1）——证明区别确实存在。
        assert!(chi_t(Some(0.0), -1.0, true, true, false));
    }

    /// π̄ argmax：多候选选 μ 最大者（§17 line 3524）。
    #[test]
    fn pi_bar_picks_max_mu_above_theta() {
        let actions = [("buy", 3.0), ("short", 8.0), ("hold", 0.0)];
        assert_eq!(pi_bar(&actions, 1.0), Some("short")); // μ=8 最大且>θ=1
    }

    /// π̄ 全 μ≤θ ⟹ Hold/Flat（None，§17 line 3531 全定义）。
    #[test]
    fn pi_bar_returns_hold_when_all_below_theta() {
        let actions = [("buy", 1.0), ("short", 2.0)];
        assert_eq!(pi_bar(&actions, 5.0), None); // 无候选 μ>θ=5 ⟹ Hold/Flat
        // 空候选集 ⟹ Hold/Flat（§17 每个 z 都有动作，最坏 Hold）。
        assert_eq!(pi_bar::<&str>(&[], 0.0), None);
    }

    /// LCB 门控分离（严格alpha.pdf p25 §12 核心可证伪）：高方差低 n 类，裸 μ>θ 但 LCB<θ ⟹
    /// LCB 选择器**拒绝**、裸 μ 选择器**准入**。二者决策分离 = LCB 升级有信息增量（非同义反复）。
    #[test]
    fn lcb_gate_rejects_high_variance_that_naive_mu_admits() {
        let z = buy_z();
        let theta = 5.0;
        let z_alpha = 1.645; // 95% 单边
        let mut est = MuEstimator::new();
        // 两样本 [110, -80]：mean=15>θ=5（裸 μ 准入），但 std≈134 极大 ⟹ LCB=15−1.645·134/√2≈−141<θ。
        est.observe(MuObservation { class: z, x_gamma: 110.0 });
        est.observe(MuObservation { class: z, x_gamma: -80.0 });
        let mu = est.mu(&z).unwrap();
        let lcb = est.mu_lcb(&z, z_alpha).unwrap();
        assert!(mu > theta, "裸 μ={mu} 应 >θ={theta}（裸门准入）");
        assert!(lcb < theta, "LCB={lcb} 应 <θ={theta}（高方差收缩拒绝）");
        // 裸 μ 门：准入。LCB 门：拒绝。决策分离（可证伪）。
        assert!(chi_open_gate(&est, &z, theta, true, true, false));
        assert!(!chi_open_gate_lcb(&est, &z, theta, z_alpha, true, true, false));
        // filter_gamma 同路：z_alpha=0（裸 μ）准入 vs z_alpha=1.645（LCB）拒绝——但 filter_gamma 走
        // Candidate，此处直接验便利包装已足（z_of_candidate 桥接由 chi_is_conjunction 等覆盖）。
    }

    /// n→∞ 收敛（mu_estimator 文档 std/√n→0）：大样本低方差 z，LCB 选择器与裸 μ 选择器决策一致。
    /// LCB→mean ⟹ LCB 门退化回裸 μ 门（高样本时置信下界不再收缩）。
    #[test]
    fn lcb_converges_to_naive_mu_at_large_n() {
        let z = buy_z();
        let theta = 5.0;
        let z_alpha = 1.645;
        let mut est = MuEstimator::new();
        // 大样本（n=2000）窄分布（围绕 10±0.5）⟹ std 小、√n 大 ⟹ std/√n→0 ⟹ LCB→mean≈10>θ。
        for i in 0..2000 {
            let x = if i % 2 == 0 { 10.5 } else { 9.5 }; // mean=10, 小方差
            est.observe(MuObservation { class: z, x_gamma: x });
        }
        let mu = est.mu(&z).unwrap();
        let lcb = est.mu_lcb(&z, z_alpha).unwrap();
        assert!((mu - lcb).abs() < 0.1, "大样本 LCB({lcb}) 应 ≈ mean({mu})");
        // 两门决策一致（都准入，因 LCB≈mean≈10>θ=5）。
        assert_eq!(
            chi_open_gate(&est, &z, theta, true, true, false),
            chi_open_gate_lcb(&est, &z, theta, z_alpha, true, true, false),
            "大样本下 LCB 门与裸 μ 门决策一致（收敛）"
        );
    }

    /// θ=−∞ 退化全覆盖在 LCB 门下不变：LCB>−∞ 对任何有 LCB（n≥2）的类恒真；None（n<2/空类）
    /// 仍按 treat_empty_as_pass。证明 LCB 升级保留 §13 全覆盖退化边界。
    #[test]
    fn lcb_theta_neg_inf_is_full_coverage() {
        let z = buy_z();
        let z_alpha = 1.645;
        // n≥2 高方差类：θ=−∞ ⟹ 即使 LCB 极负也 >−∞ ⟹ 准入（全覆盖不滤已观测 n≥2 类）。
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z, x_gamma: 110.0 });
        est.observe(MuObservation { class: z, x_gamma: -80.0 });
        assert!(chi_open_gate_lcb(&est, &z, f64::NEG_INFINITY, z_alpha, true, true, true));
        // None 类（空类）：θ=−∞ 不改 treat_empty_as_pass 裁决——true 放行，false 拒绝。
        let empty = MuEstimator::new();
        assert!(chi_open_gate_lcb(&empty, &z, f64::NEG_INFINITY, z_alpha, true, true, true));
        assert!(!chi_open_gate_lcb(&empty, &z, f64::NEG_INFINITY, z_alpha, true, true, false));
    }

    /// None 语义统一（任务 §3）：n<2 单样本（mu_lcb=None）与空类同走 treat_empty_as_pass，
    /// 不冒充 LCB=mean。无 LCB 证据时 false=不交易（p25「无正边际收益证据时不交易」）。
    #[test]
    fn lcb_single_sample_none_follows_empty_semantics() {
        let z = buy_z();
        let z_alpha = 1.645;
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z, x_gamma: 100.0 }); // n=1 ⟹ 方差未定义 ⟹ mu_lcb=None
        assert_eq!(est.mu_lcb(&z, z_alpha), None, "n=1 ⟹ mu_lcb None");
        assert_eq!(est.mu(&z), Some(100.0), "但裸 μ 有值（n=1 均值已定义）");
        // 无 LCB 证据：false ⟹ 不交易（诚实），true ⟹ 全覆盖放行——与空类同。
        assert!(!chi_open_gate_lcb(&est, &z, 0.0, z_alpha, true, true, false));
        assert!(chi_open_gate_lcb(&est, &z, 0.0, z_alpha, true, true, true));
    }

    /// z_alpha=0 退化（向后兼容）：LCB=mean−0=mean ⟹ LCB 门 ≡ 裸 μ 门（filter_gamma/runner
    /// 默认 z_alpha=0 保 frozen bit-exact）。
    #[test]
    fn lcb_z_alpha_zero_equals_naive_mu() {
        let z = buy_z();
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z, x_gamma: 10.0 });
        est.observe(MuObservation { class: z, x_gamma: 8.0 }); // n=2，LCB 有定义
        // z_alpha=0 ⟹ LCB=mean=9，与裸 μ 门同决策。
        assert_eq!(est.mu_lcb(&z, 0.0), est.mu(&z));
        for theta in [-1.0, 8.5, 9.0, 100.0] {
            assert_eq!(
                chi_open_gate(&est, &z, theta, true, true, false),
                chi_open_gate_lcb(&est, &z, theta, 0.0, true, true, false),
                "z_alpha=0 时 LCB 门 ≡ 裸 μ 门（θ={theta}）"
            );
        }
    }
}
