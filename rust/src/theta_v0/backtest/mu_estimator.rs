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

/// 单 z 桶的 Welford 在线均值/方差累加器（Welford 1962，数值稳定，无 ΣX² 灾难性抵消）。
///
/// 状态 `(n, mean, m2)`：`mean = ΣX_γ/n`，`m2 = Σ(X_γ−mean)²`。样本方差 = `m2/(n−1)`
/// （无偏，贝塞尔校正；n<2 时未定义 ⟹ 见 [`Welford::std_sample`]）。LCB 需方差 ⟹ 必须
/// 存二阶量；选 Welford 而非 (ΣX,ΣX²) 因后者大样本下 ΣX² 与 (ΣX)²/n 相减灾难性抵消。
#[derive(Debug, Clone, Copy, Default)]
struct Welford {
    n: u64,
    mean: f64,
    m2: f64,
}

impl Welford {
    /// 累加一个观测（Welford 递推，O(1)）。
    fn push(&mut self, x: f64) {
        self.n += 1;
        let delta = x - self.mean;
        self.mean += delta / self.n as f64;
        let delta2 = x - self.mean;
        self.m2 += delta * delta2;
    }

    /// 样本标准差 √(m2/(n−1))（无偏方差开方）。`n<2` ⟹ `None`（方差未定义）。
    fn std_sample(&self) -> Option<f64> {
        if self.n < 2 {
            None
        } else {
            Some((self.m2 / (self.n - 1) as f64).sqrt())
        }
    }
}

/// μ(z) 条件边际收益估计器（§12 line 2173 样本均值）。
///
/// 按 [`MuClass`] z 分桶 Welford 累加 X_γ，`mu(z) = mean_z`（条件期望的样本估计），
/// 并存方差供 [`MuEstimator::mu_lcb`] 算置信下界（§12 实操选择器 χ_t 用 LCB(μ) 非裸 μ，
/// 防高维 z 过拟合估计噪声）。**不做** χ_θ 过滤 / argmax 选择（下游工位）。
#[derive(Debug, Clone, Default)]
pub struct MuEstimator {
    /// z → Welford(n, mean, m2)。样本均值 = mean（[`MuEstimator::mu`]）。
    buckets: HashMap<MuClass, Welford>,
}

impl MuEstimator {
    pub fn new() -> Self {
        MuEstimator::default()
    }

    /// 累加一笔观测到对应 z 桶（Welford 在线递推，O(1) 摊销）。
    pub fn observe(&mut self, obs: MuObservation) {
        self.buckets.entry(obs.class).or_default().push(obs.x_gamma);
    }

    /// 批量累加（迭代器 fold，等价逐笔 [`MuEstimator::observe`]）。
    pub fn observe_all(&mut self, obs: impl IntoIterator<Item = MuObservation>) {
        for o in obs {
            self.observe(o);
        }
    }

    /// μ(z) = E[X_γ|Z=z] 样本估计（§12）。
    ///
    /// 返回 `Some(mean)`（该 z 有观测），`None`（该 z 无样本——空类无估计，**不**冒充 μ=0；
    /// 空类与 μ=0 是不同认识状态：前者无数据，后者有数据且均值为 0）。
    pub fn mu(&self, class: &MuClass) -> Option<f64> {
        self.buckets.get(class).map(|w| w.mean)
    }

    /// LCB(μ(z)) = mean − z_α·(std/√n) 单边置信下界（§12 实操选择器 χ_t 的准入量）。
    ///
    /// `z_alpha` 是单边正态分位（如 95%→1.645，99%→2.326）——由调用方按置信水平传入
    /// （估计器不绑定分布表，下游 selector 决定置信水平）。语义（诚实标注）：
    /// - `n≥2`：`Some(mean − z_alpha·std/√n)`，标准误 std/√n 随 √n 收敛 ⟹ LCB→mean。
    /// - `n=1`：标准差未定义 ⟹ `None`（**不**冒充 LCB=mean——单样本无方差信息，给不出
    ///   收缩后的保守下界；下游 selector 自行决定单样本是否准入，估计器不替它造数）。
    /// - 无样本：`None`（与 [`MuEstimator::mu`] 一致——空类无估计）。
    ///
    /// LCB≤mean 恒成立（`z_alpha≥0` 且 std/√n≥0）⟹ 置信下界不超过点估计（不乐观）。
    pub fn mu_lcb(&self, class: &MuClass, z_alpha: f64) -> Option<f64> {
        let w = self.buckets.get(class)?;
        let std = w.std_sample()?; // n<2 ⟹ None
        Some(w.mean - z_alpha * std / (w.n as f64).sqrt())
    }

    /// 该 z 类的样本量 |S_z|（统计功效判定用——小样本 μ 估计不可靠）。
    pub fn count(&self, class: &MuClass) -> u64 {
        self.buckets.get(class).map_or(0, |w| w.n)
    }

    /// pooled 均值 μ_pooled = (Σ_z n_z·mean_z)/(Σ_z n_z)，聚合范围 = 同 `(level, delta)`
    /// 的所有 z 类（PDF §8 收缩目标 = 同级别同多空 pooled mean）。
    ///
    /// 加权合并各桶 `(n, mean)`——`Σ n·mean = ΣΣX_γ` ⟹ 等于把同 (level,delta) 全部 X_γ
    /// 拉平后求总均值（与逐笔合并 bit-exact）。`None` ⟹ 该 (level,delta) 无任何样本。
    fn pooled_mean(&self, level: u32, delta: i8) -> Option<f64> {
        let mut sum = 0.0_f64;
        let mut n_total = 0_u64;
        for (z, w) in &self.buckets {
            if z.level == level && z.delta == delta {
                sum += w.mean * w.n as f64;
                n_total += w.n;
            }
        }
        if n_total == 0 {
            None
        } else {
            Some(sum / n_total as f64)
        }
    }

    /// 层级收缩 μ_shrink(z) = w_z·mean_z + (1−w_z)·μ_pooled（PDF §8，样本稀疏类抗过拟合）。
    ///
    /// `w_z = n_z/(n_z + σ²_z/τ²)`，`σ²_z` = 本类样本方差（Welford `m2/(n−1)`），`tau_sq` τ² =
    /// 级别间先验方差（PDF θ_ℓ~N(0,τ_ℓ²)，由调用方传入）。`μ_pooled` = 同 (level,delta) 的
    /// [`pooled_mean`]（收缩目标）。
    ///
    /// 语义（诚实标注，与 [`MuEstimator::mu_lcb`] 的 None 语义**区别**）：
    /// - `n_z≥2`：σ²_z 有定义 ⟹ `Some(w_z·mean_z + (1−w_z)·μ_pooled)`。n_z 大 ⟹ w_z→1
    ///   ⟹ 信本类均值；n_z 小 ⟹ w_z→0 ⟹ 收缩到 pooled。
    /// - `n_z<2`：样本方差未定义 ⟹ `w_z=0` ⟹ **完全收缩到 μ_pooled**（样本太少就别信它，
    ///   借 pooled 估计——这是收缩的意义，与 mu_lcb「拒绝返 None」相反：mu_shrink 保功效）。
    /// - 该 (level,delta) 无任何样本（连 pooled 都没有）⟹ `None`（无可借的估计）。
    ///
    /// `tau_sq≤0` ⟹ debug 断言失败（先验方差须正——τ²=0 退化为分母 ∞ ⟹ w_z=0 全收缩，
    /// τ²<0 无意义，fail-fast 非静默）。
    pub fn mu_shrink(&self, class: &MuClass, tau_sq: f64) -> Option<f64> {
        debug_assert!(tau_sq > 0.0, "τ²（tau_sq）须 > 0，收到 {tau_sq}");
        let pooled = self.pooled_mean(class.level, class.delta)?;
        let w = self.buckets.get(class)?;
        // n<2 ⟹ 样本方差未定义 ⟹ w_z=0 ⟹ 完全收缩到 pooled（保功效，非拒绝）。
        let w_z = match w.std_sample() {
            None => 0.0,
            Some(std) => {
                let var_z = std * std; // σ²_z
                w.n as f64 / (w.n as f64 + var_z / tau_sq)
            }
        };
        Some(w_z * w.mean + (1.0 - w_z) * pooled)
    }

    /// 收缩视图：返回新 [`MuEstimator`]，每桶 `mean ← mu_shrink(z,τ²)`、**保留原 n/m2**（acc-three-way-l2）。
    ///
    /// 用途：让 frozen selector（[`super::runner::run_theta_v0_pi_chi`]，准入 `mu(z)>θ`）对**收缩后**
    /// 的 μ 兑现 ΔR——selector 一行不改，只换喂给它的 est。三路对比中 shrinkage 路 = `shrunk_view(τ²)`
    /// 配 `z_alpha=0`（裸 μ 门），与裸 μ/LCB 同 walk-forward harness。保留原 n ⟹ n_L3 池大小三路可比
    /// （shrinkage 卖点 = 保功效，可被 [`MuEstimator::count`] / n_L3 验证）。
    ///
    /// **m2 保留是已知陷阱**：mean 被收缩但 m2（⟹ std）仍是原始样本 ⟹ 对 shrunk_view 再求
    /// [`MuEstimator::mu_lcb`] 会得到 `shrunk_mean − z_α·orig_std/√n`，**LCB 语义失真**（下界基于的
    /// mean 不再是该桶样本均值）。故 **shrunk_view 仅配 `z_alpha=0` 裸门用**，禁止再 LCB。
    ///
    /// n<2 桶：[`MuEstimator::mu_shrink`] 完全收缩到 pooled（w_z=0），新桶 mean=pooled、n/m2 不变。
    /// `mu_shrink` 仅当 pooled None 才返 None，而该桶 ∈ 自己的 (level,delta) pooled ⟹ pooled 必 Some
    /// ⟹ unwrap_or 的 fallback 不可达（保险保留原 mean，不 panic）。
    pub fn shrunk_view(&self, tau_sq: f64) -> MuEstimator {
        debug_assert!(tau_sq > 0.0, "τ²（tau_sq）须 > 0，收到 {tau_sq}");
        let mut buckets = HashMap::with_capacity(self.buckets.len());
        for (z, w) in &self.buckets {
            let shrunk_mean = self.mu_shrink(z, tau_sq).unwrap_or(w.mean);
            buckets.insert(*z, Welford { n: w.n, mean: shrunk_mean, m2: w.m2 });
        }
        MuEstimator { buckets }
    }

    /// 已观测的全部 z 类及其 μ 估计（按需消费；顺序不定，HashMap 无序）。
    pub fn iter_mu(&self) -> impl Iterator<Item = (MuClass, f64)> + '_ {
        self.buckets.iter().map(|(z, w)| (*z, w.mean))
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
        assert_eq!(est.mu_lcb(&z, 1.645), None);
        assert_eq!(est.count(&z), 0);
    }

    /// LCB ≤ mean 恒成立（置信下界不超过点估计——z_α≥0 且 std/√n≥0，不乐观）。
    #[test]
    fn lcb_never_exceeds_mean() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let mut est = MuEstimator::new();
        est.observe_all([
            MuObservation { class: z, x_gamma: 5.0 },
            MuObservation { class: z, x_gamma: 15.0 },
            MuObservation { class: z, x_gamma: 10.0 },
        ]);
        let mean = est.mu(&z).unwrap();
        let lcb = est.mu_lcb(&z, 1.645).unwrap();
        assert!(lcb <= mean, "LCB({lcb}) 必 ≤ mean({mean})");
    }

    /// LCB 单调性：固定均值/标准差，n 增大 ⟹ 标准误 std/√n 收缩 ⟹ LCB 单调逼近 mean。
    /// 用同一对称样本 {mean−s, mean+s} 重复 k 份 ⟹ mean、样本 std 不变，仅 n 变。
    #[test]
    fn lcb_converges_to_mean_as_n_grows() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let (m, s) = (20.0_f64, 4.0_f64); // 每对 {16,24}：均值 20，n 趋大时样本 std→s
        let lcb_at = |reps: usize| -> f64 {
            let mut est = MuEstimator::new();
            for _ in 0..reps {
                est.observe(MuObservation { class: z, x_gamma: m - s });
                est.observe(MuObservation { class: z, x_gamma: m + s });
            }
            est.mu_lcb(&z, 1.645).unwrap()
        };
        let (lcb_small, lcb_large) = (lcb_at(2), lcb_at(50)); // n=4 vs n=100
        // 均值恒为 20，样本 std 在两规模下都 ≈4（对称样本）⟹ 仅 √n 不同。
        assert!(lcb_large > lcb_small, "n↑ ⟹ LCB 上移逼近 mean：{lcb_large} > {lcb_small}");
        assert!(lcb_large < m, "LCB 仍 < mean（n 有限，标准误 > 0）");
        assert!((m - lcb_large) < (m - lcb_small) * 0.3, "√n 收敛：大样本 gap 显著缩小");
    }

    /// n=1 边界：单样本方差未定义 ⟹ mu_lcb 返回 None（不冒充 LCB=mean，诚实语义）。
    /// mu(z) 仍返回该单点（点估计有定义，与 LCB 语义分离）。
    #[test]
    fn lcb_undefined_for_single_sample() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z, x_gamma: 42.0 });
        assert_eq!(est.mu(&z), Some(42.0), "单样本点估计有定义");
        assert_eq!(est.mu_lcb(&z, 1.645), None, "单样本方差未定义 ⟹ LCB None");
        assert_eq!(est.count(&z), 1);
    }

    /// 收缩单调性：固定 mean_z/pooled/σ²_z，n_z↑ ⟹ w_z↑ ⟹ mu_shrink 单调逼近 mean_z。
    /// 同一对称样本 {mean−s,mean+s} 重复 k 份 ⟹ mean_z、σ²_z 不变，仅 n 变。另设一个高样本
    /// 同 (level,delta) 邻类拉低 pooled，使 mean_z≠pooled ⟹ 收缩方向可观测。
    #[test]
    fn shrink_converges_to_mean_as_n_grows() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        // 邻类：同 level=3 同 delta=+1，I_γ 不同 ⟹ 拉低 pooled（大量低收益样本）。
        let neighbor = MuClass::from_certificate(
            3,
            1,
            BspBits { buy2: true, ..Default::default() },
            0,
            PositionState::Root,
        );
        let (m, s) = (50.0_f64, 4.0_f64); // z 类均值 50，σ²_z 恒定
        let shrink_at = |reps: usize| -> f64 {
            let mut est = MuEstimator::new();
            // 邻类灌 1000 笔 x=0 ⟹ pooled 被强拉向 0（远离 z 的 50）。
            for _ in 0..1000 {
                est.observe(MuObservation { class: neighbor, x_gamma: 0.0 });
            }
            for _ in 0..reps {
                est.observe(MuObservation { class: z, x_gamma: m - s });
                est.observe(MuObservation { class: z, x_gamma: m + s });
            }
            est.mu_shrink(&z, 1.0).unwrap()
        };
        let (small, large) = (shrink_at(1), shrink_at(50)); // n_z=2 vs n_z=100
        assert!(large > small, "n_z↑ ⟹ w_z↑ ⟹ 收缩值上移逼近 mean：{large} > {small}");
        assert!(large < m, "n_z 有限 ⟹ w_z<1 ⟹ 仍 < mean_z");
        assert!(small > 0.0, "即使 n_z 小，w_z>0 ⟹ 未完全坍到 pooled(≈0)");
    }

    /// 收缩方向：n_z=1 高偏离类 ⟹ w_z=0 ⟹ mu_shrink 完全坍到 pooled（vs 裸 mu 不拉）。
    /// 这是 n<2 保功效语义（借 pooled），与 mu_lcb 的 None 拒绝相反。
    #[test]
    fn shrink_single_sample_collapses_to_pooled() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let neighbor = MuClass::from_certificate(
            3,
            1,
            BspBits { buy2: true, ..Default::default() },
            0,
            PositionState::Root,
        );
        let mut est = MuEstimator::new();
        // 邻类大量 x=10 ⟹ 主导 pooled。
        for _ in 0..100 {
            est.observe(MuObservation { class: neighbor, x_gamma: 10.0 });
        }
        // z 单样本极端偏离值 1000。
        est.observe(MuObservation { class: z, x_gamma: 1000.0 });
        assert_eq!(est.mu(&z), Some(1000.0), "裸 μ 不拉，仍是单样本值");
        // pooled = (100·10 + 1·1000)/101 ≈ 19.8；n_z=1 ⟹ w_z=0 ⟹ 完全坍到 pooled。
        let pooled = (100.0 * 10.0 + 1000.0) / 101.0;
        let shrunk = est.mu_shrink(&z, 1.0).unwrap();
        assert!((shrunk - pooled).abs() < 1e-9, "n=1 ⟹ 完全收缩到 pooled：{shrunk} ≈ {pooled}");
        assert_ne!(est.mu_lcb(&z, 1.645), Some(shrunk), "mu_lcb n=1 返 None，与 mu_shrink 语义分离");
    }

    /// pooled 聚合正确性：同 (level,delta) 按样本加权聚合，跨 level / 跨 delta 不混。
    #[test]
    fn pooled_mean_aggregates_same_level_delta_only() {
        let z_a = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        // 同 level=3 同 delta=+1，I_γ 不同 ⟹ 进同一 pooled。
        let z_b = MuClass::from_certificate(
            3,
            1,
            BspBits { buy2: true, ..Default::default() },
            0,
            PositionState::Root,
        );
        // 跨 level（5≠3）⟹ 不进 pooled(3,+1)。
        let z_other_level = MuClass::from_certificate(5, 1, buy_bits(), 0, PositionState::Root);
        // 跨 delta（−1≠+1）⟹ 不进 pooled(3,+1)。
        let z_other_delta = MuClass::from_certificate(3, -1, buy_bits(), 0, PositionState::Root);
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z_a, x_gamma: 10.0 }); // n=1
        est.observe(MuObservation { class: z_b, x_gamma: 30.0 }); // n=1
        est.observe(MuObservation { class: z_b, x_gamma: 50.0 }); // n=2 ⟹ z_b 均值 40
        est.observe(MuObservation { class: z_other_level, x_gamma: 1000.0 });
        est.observe(MuObservation { class: z_other_delta, x_gamma: -1000.0 });
        // pooled(3,+1) = (10 + 30 + 50)/3 = 30（z_a 1 笔 + z_b 2 笔；其他 level/delta 不混）。
        assert_eq!(est.pooled_mean(3, 1), Some(30.0));
        // pooled(5,+1) 只含 z_other_level。
        assert_eq!(est.pooled_mean(5, 1), Some(1000.0));
        // pooled(3,−1) 只含 z_other_delta。
        assert_eq!(est.pooled_mean(3, -1), Some(-1000.0));
        // 无样本的 (level,delta) ⟹ None。
        assert_eq!(est.pooled_mean(7, 1), None);
    }

    /// 无样本 z 且其 (level,delta) 也无样本 ⟹ mu_shrink None（无可借估计）。
    #[test]
    fn shrink_none_when_no_pooled_sample() {
        let z = MuClass::from_certificate(9, 1, buy_bits(), 0, PositionState::Root);
        let est = MuEstimator::new();
        assert_eq!(est.mu_shrink(&z, 1.0), None);
    }

    /// shrunk_view：各桶 mean==mu_shrink、n/count 保留（n_L3 池大小不变，保功效卖点可验证）。
    #[test]
    fn shrunk_view_replaces_mean_keeps_n() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let neighbor =
            MuClass::from_certificate(3, 1, BspBits { buy2: true, ..Default::default() }, 0, PositionState::Root);
        let mut est = MuEstimator::new();
        // 邻类大量低值拉低 pooled，z 高值 ⟹ 收缩可观测。
        for _ in 0..100 {
            est.observe(MuObservation { class: neighbor, x_gamma: 0.0 });
        }
        est.observe_all([
            MuObservation { class: z, x_gamma: 40.0 },
            MuObservation { class: z, x_gamma: 60.0 },
        ]); // z 均值 50，n=2
        let tau_sq = 1.0;
        let view = est.shrunk_view(tau_sq);
        // 各桶 mean 恰为 mu_shrink，n 保留（count 不变 ⟹ n_L3 可比）。
        for (zc, _) in est.iter_mu() {
            assert_eq!(view.mu(&zc), est.mu_shrink(&zc, tau_sq), "桶 mean 应==mu_shrink");
            assert_eq!(view.count(&zc), est.count(&zc), "n 保留 ⟹ n_L3 池可比");
        }
        // z 收缩后 < 原 mean（被 pooled 向 0 拉）但 > 0（n=2 ⟹ w_z>0 未全坍）。
        let shrunk_z = view.mu(&z).unwrap();
        assert!(shrunk_z < 50.0 && shrunk_z > 0.0, "收缩方向正确：0 < {shrunk_z} < 50");
        assert_eq!(view.n_classes(), est.n_classes(), "桶集合不变");
    }
}
