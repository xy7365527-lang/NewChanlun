//! μ(z,a) 类别条件边际收益估计器（alpha2.pdf §12-§14，task #39 mu-estimator）。
//!
//! ## 命题（alpha2 §12 / §16）
//!
//! 把买卖点操作状态拆成**全互斥类别** z，在每类上估计条件边际收益样本均值：
//!
//! ```text
//!   z = (ℓ, δ, I_γ, 父声部方向, 短差/顺势, 仓位态)         (§16 line 3262)
//!   τ_γ = inf{u > t : 出现该声部出场证书或风险退出}          (§12 line 2140)
//!   X_γ = δ·(P_τγ − P_t) − C_{t:τγ}                          (§12 line 2147)
//!   μ(z) = E[X_γ | Z = z] ≈ (1/|S_z|) Σ_{γ∈S_z} X_γ          (§12 line 2173, 样本均值)
//! ```
//!
//! 严格 alpha 条件 `μ(z) > 0`（§12 line 2180）；`μ(z) ≤ 0` ⟹ 该类在此退出规则/成本模型/
//! 样本下无正期望（§12 line 2186）。本模块**只估计 μ**——不做 χ_θ 阈值过滤（那是下游
//! chi-theta-filter 工位 acc-chi-theta-filter），不做 argmax_a 动作选择。诚实声明：本模块
//! 是 alpha2「估计 μ」这一步（§17 line 3519 流程的 `估计 μ` 环节），不是选择器。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! | 组件 | 等级 | 理由 |
//! |------|------|------|
//! | 估计器逻辑（[`MuEstimator`] 分桶/求均值/[`MuClass`] 编码） | **L1** | 给定观测序列求条件均值是确定性变换，验证算法正确，零信息增量 |
//! | 真实数据驱动的 μ 值 | **L2** | 喂真实历史交易的 X_γ 才能否证「某类 z 有正边际收益」（正信息增量） |
//!
//! **合成数据 μ 值是 L1**——自造 X_γ 求均值只验证分桶/平均无 bug，不验证任何 z 类在市场有
//! 正期望（合成数据独立性验证是同义反复，231号）。本模块单测全部喂合成观测 ⟹ L1。
//!
//! ## 因果性硬约束（alpha2 §5 / formalization-validity-domain / project_zero_lookahead_backtest）
//!
//! `X_γ` 的 `P_τγ` 必须是持仓**实际兑现**到未来退出时刻 τ_γ 的价格——这是 F_τγ-可测的真实
//! 退出，**不是**端点后视 `ε_e = sign(P_ρe − P_λe)`（用段终点反推方向 = 未来函数泄漏）。
//! 本模块不计算 τ_γ（退出时刻由上游交易轨迹给定，[`MuObservation::x_gamma`] 由调用方按
//! 真实 entry/exit 价格用 [`marginal_return`] 算出后传入）。μ 估计器只对**已实现**的 X_γ 分桶——
//! 这是 μ 与构造性恒真 G_e 的本质区别：G_e 用端点拼接恒真（L0 同义反复），μ 用实际兑现可否证。

use std::collections::HashMap;

use super::metrics::trade_abs_pnl;
use crate::theta_v0::types::BspBits;

/// 仓位态（z 的分量，§16 line 3262「仓位态」）。
///
/// 区分声部在持仓树中的角色：根声部（无父，主趋势腿）vs 子声部（有父，对冲/短差腿）。
/// 这是 z 全互斥分类的一维——不同仓位态不混（alpha2 §18「多空双开状态不会混在一起」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PositionState {
    /// 根声部（§3 ⊥，host 父容器=边界胚元 ∂，无父声部）。主趋势持仓腿。
    Root,
    /// 子声部（§3 有父声部 p(v)）。对冲/短差腿，依附父持仓期内。
    Child,
}

/// 全互斥分类值 z（alpha2 §12 `γ=(c,ℓ,δ,I_γ,t)` + §16 line 3262 扩展态）。
///
/// 六维全互斥（§18：级别/三类/短差顺势/父声部方向/仓位态/多空 不混）：
/// - `level` ℓ：买卖点所在级别（Voice.carrier.level）。
/// - `delta` δ ∈ {+1,−1}：持仓方向（Voice.dir；买点 +1 / 卖点 −1）。
/// - `i_class` I_γ⊆{1,2,3}：买卖点类别集合 **bit-vector 不压扁**（[`BspBits::class_index`]
///   的 6-bit，B1/B2/B3/S1/S2/S3 各独立 ⟹ 2B/3B 重合保留，§P4 §5 非互斥三分）。
/// - `parent_dir` σ_p：父声部方向（根声部 = 0/Ambient，去根化非「未持仓」；runner.rs:218）。
/// - `short_swing` 短差/顺势：子声部 σ_u=−σ_p ⟹ 短差（true）；同向 ⟹ 顺势（false）。
///   `Voice::child_dir(parent_dir) = −parent_dir`（pi_bsp_timing.rs:122 §6/§16）。
/// - `position` 仓位态：[`PositionState`]。
///
/// 派生 `Eq + Hash` ⟹ 可作 HashMap key（分桶载体）。**全互斥**：每个 z 是 {0,1}^6 × 级别 ×
/// 方向 × 父向 × 短差 × 仓位态 的唯一组合，无重叠（§13 精细分类优势定理的可计算落点）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MuClass {
    pub level: u32,
    pub delta: i8,
    pub i_class: u8,
    pub parent_dir: i8,
    pub short_swing: bool,
    pub position: PositionState,
}

impl MuClass {
    /// 从证书原始分量构造 z（§12 `γ=(c,ℓ,δ,I_γ,t)` + §16 扩展态）。
    ///
    /// `i_class` 取 [`BspBits::class_index`]——**不压扁** I_γ（2B/3B 重合保留为不同 z）。
    /// `short_swing` 由 `delta` 与 `parent_dir` 关系判定：子声部且 δ=−σ_p ⟹ 短差（§6/§16）；
    /// 根声部（`parent_dir=0`）恒顺势（无父可对冲，short_swing=false）。
    pub fn from_certificate(
        level: u32,
        delta: i8,
        bits: BspBits,
        parent_dir: i8,
        position: PositionState,
    ) -> Self {
        // 短差判定：有父（parent_dir≠0）且方向与父反向（δ=−σ_p）⟹ 短差对冲腿（§6/§16）。
        // 根声部（parent_dir=0）无父，short_swing 恒 false（顺势主腿）。
        let short_swing = parent_dir != 0 && delta == -parent_dir;
        MuClass {
            level,
            delta,
            i_class: bits.class_index(),
            parent_dir,
            short_swing,
            position,
        }
    }
}

/// 单笔交易观测：分类值 z + 已实现交易收益 X_γ（§12 line 2147）。
///
/// `x_gamma` 是**已兑现**的 `δ(P_τγ−P_t)−C`——由调用方用真实 entry/exit 价格经
/// [`marginal_return`] 算出（F_τγ-可测，非端点后视）。μ 估计器只消费已实现值，不重算退出时刻。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MuObservation {
    pub class: MuClass,
    pub x_gamma: f64,
}

/// 交易收益 X_γ = δ(P_τγ−P_t) − C_{t:τγ}（§12 line 2147）。
///
/// **复用** [`trade_abs_pnl`]（metrics.rs 单一来源的方向感知 PnL：多头 `q(P_τ(1−f)−P_t(1+f))`，
/// 空头 `q(P_t(1−f)−P_τ(1+f))`，canonical 镜像 σ→−σ）——X_γ 的 `δ·(·)−C` 正是该公式：方向由
/// `delta` 选公式（δ=+1 long / δ=−1 short），成本 C 由 `fee_rate` 双边扣（建+平仓各 ·(1±f)）。
///
/// 单位诚实：返回**绝对**收益（与 metrics 一致），非归一化——调用方若需归一化自行 ÷nav_base。
///
/// # Panics（debug）
/// `delta ∉ {+1,−1}` ⟹ debug 断言失败（z 的 δ 只能是买/卖方向，fail-fast 非静默）。
pub fn marginal_return(
    entry_px: f64,
    exit_px: f64,
    qty: f64,
    fee_rate: f64,
    delta: i8,
) -> f64 {
    debug_assert!(delta == 1 || delta == -1, "δ 必须 ∈ {{+1,−1}}，收到 {delta}");
    trade_abs_pnl(entry_px, exit_px, qty, fee_rate, delta == 1)
}

/// μ(z) 条件边际收益估计器（§12 line 2173 样本均值）。
///
/// 按 [`MuClass`] z 分桶累加 X_γ 与计数，`mu(z) = sum_z / count_z`（条件期望的样本估计）。
/// **不做** χ_θ 过滤 / argmax 选择（下游工位）——只产 μ 表供选择器消费。
#[derive(Debug, Clone, Default)]
pub struct MuEstimator {
    /// z → (ΣX_γ, 计数)。样本均值 = ΣX_γ / 计数（[`MuEstimator::mu`]）。
    buckets: HashMap<MuClass, (f64, u64)>,
}

impl MuEstimator {
    pub fn new() -> Self {
        MuEstimator::default()
    }

    /// 累加一笔观测到对应 z 桶（在线累加，O(1) 摊销）。
    pub fn observe(&mut self, obs: MuObservation) {
        let entry = self.buckets.entry(obs.class).or_insert((0.0, 0));
        entry.0 += obs.x_gamma;
        entry.1 += 1;
    }

    /// 批量累加（迭代器 fold，等价逐笔 [`MuEstimator::observe`]）。
    pub fn observe_all(&mut self, obs: impl IntoIterator<Item = MuObservation>) {
        for o in obs {
            self.observe(o);
        }
    }

    /// μ(z) = E[X_γ|Z=z] 样本估计（§12）。
    ///
    /// 返回 `Some(sum/count)`（该 z 有观测），`None`（该 z 无样本——空类无估计，**不**冒充 μ=0；
    /// 空类与 μ=0 是不同认识状态：前者无数据，后者有数据且均值为 0）。
    pub fn mu(&self, class: &MuClass) -> Option<f64> {
        self.buckets.get(class).map(|(sum, n)| sum / *n as f64)
    }

    /// 该 z 类的样本量 |S_z|（统计功效判定用——小样本 μ 估计不可靠）。
    pub fn count(&self, class: &MuClass) -> u64 {
        self.buckets.get(class).map_or(0, |(_, n)| *n)
    }

    /// 已观测的全部 z 类及其 μ 估计（按需消费；顺序不定，HashMap 无序）。
    pub fn iter_mu(&self) -> impl Iterator<Item = (MuClass, f64)> + '_ {
        self.buckets
            .iter()
            .map(|(z, (sum, n))| (*z, sum / *n as f64))
    }

    /// 已观测 z 类的数量（分桶覆盖了多少互斥类别）。
    pub fn n_classes(&self) -> usize {
        self.buckets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buy_bits() -> BspBits {
        BspBits { buy1: true, ..Default::default() }
    }

    /// X_γ = δ(P_τ−P_t)−C 与 metrics 方向感知 PnL bit-exact 一致（复用单一来源，无公式漂移）。
    #[test]
    fn marginal_return_matches_directional_pnl() {
        // 多头 δ=+1：低买(100)高卖(110)，零成本 ⟹ +10。
        assert_eq!(marginal_return(100.0, 110.0, 1.0, 0.0, 1), 10.0);
        // 空头 δ=−1：高卖(120)低买(100)，零成本 ⟹ +20（空头盈利）。
        assert_eq!(marginal_return(120.0, 100.0, 1.0, 0.0, -1), 20.0);
        // 空头在上涨(100→110)亏损 ⟹ −10。
        assert_eq!(marginal_return(100.0, 110.0, 1.0, 0.0, -1), -10.0);
    }

    /// 成本 C_{t:τ} 双边扣（建+平仓各 ·(1±fee)）⟹ X_γ 比零成本低。
    #[test]
    fn marginal_return_deducts_cost() {
        let gross = marginal_return(100.0, 110.0, 1.0, 0.0, 1);
        let net = marginal_return(100.0, 110.0, 1.0, 0.01, 1);
        assert!(net < gross, "含成本 X_γ({net}) 必 < 零成本({gross})");
    }

    /// μ(z) = ΣX_γ/|S_z| 样本均值（§12 line 2173）。
    #[test]
    fn mu_is_sample_mean_per_class() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let mut est = MuEstimator::new();
        est.observe_all([
            MuObservation { class: z, x_gamma: 10.0 },
            MuObservation { class: z, x_gamma: 20.0 },
            MuObservation { class: z, x_gamma: 30.0 },
        ]);
        assert_eq!(est.mu(&z), Some(20.0)); // (10+20+30)/3
        assert_eq!(est.count(&z), 3);
    }

    /// 不同 z 不混（§18 全互斥）——同级别同方向但 I_γ 不同 ⟹ 不同桶，μ 独立。
    #[test]
    fn distinct_classes_do_not_mix() {
        let z_b1 = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let z_b2 = MuClass::from_certificate(
            3,
            1,
            BspBits { buy2: true, ..Default::default() },
            0,
            PositionState::Root,
        );
        assert_ne!(z_b1, z_b2, "B1 与 B2 是不同 I_γ ⟹ 不同 z");
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z_b1, x_gamma: 10.0 });
        est.observe(MuObservation { class: z_b2, x_gamma: -10.0 });
        assert_eq!(est.mu(&z_b1), Some(10.0));
        assert_eq!(est.mu(&z_b2), Some(-10.0)); // §12 line 2186：μ≤0 该类无正期望
        assert_eq!(est.n_classes(), 2);
    }

    /// I_γ 不压扁：2B+3B 重合（buy2∧buy3）是与单一 buy2 不同的 z（§P4 §5 非互斥三分）。
    #[test]
    fn i_gamma_not_collapsed_coincident_bsp() {
        let only_b2 = MuClass::from_certificate(
            1,
            1,
            BspBits { buy2: true, ..Default::default() },
            0,
            PositionState::Root,
        );
        let b2_and_b3 = MuClass::from_certificate(
            1,
            1,
            BspBits { buy2: true, buy3: true, ..Default::default() },
            0,
            PositionState::Root,
        );
        assert_ne!(only_b2.i_class, b2_and_b3.i_class, "重合买卖点保留为不同 I_γ");
    }

    /// 短差判定：子声部 δ=−σ_p ⟹ short_swing=true（§6/§16 σ_u=−σ_p 对冲腿）。
    #[test]
    fn short_swing_when_child_opposes_parent() {
        // 父向上 σ_p=+1，子声部 δ=−1（反向）⟹ 短差。
        let short = MuClass::from_certificate(2, -1, buy_bits(), 1, PositionState::Child);
        assert!(short.short_swing);
        // 子声部同向 δ=+1=σ_p ⟹ 顺势（非短差）。
        let trend = MuClass::from_certificate(2, 1, buy_bits(), 1, PositionState::Child);
        assert!(!trend.short_swing);
        // 根声部 parent_dir=0 ⟹ 恒顺势（无父可对冲）。
        let root = MuClass::from_certificate(2, 1, buy_bits(), 0, PositionState::Root);
        assert!(!root.short_swing);
    }

    /// 空类返回 None（无样本 ≠ μ=0；不冒充估计）。
    #[test]
    fn empty_class_returns_none_not_zero() {
        let z = MuClass::from_certificate(5, 1, buy_bits(), 0, PositionState::Root);
        let est = MuEstimator::new();
        assert_eq!(est.mu(&z), None);
        assert_eq!(est.count(&z), 0);
    }
}
