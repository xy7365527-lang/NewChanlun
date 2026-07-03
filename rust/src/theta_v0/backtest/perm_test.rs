//! 路径 A 分层内 δ 置换检验（**残差口径**，alpha分离.pdf §4.2 + acc-alpha 预注册 §1.4，perm_p 生产者）。
//!
//! decontam.rs 三态判据消费逐桶 perm_p 但不生产它（Welford 聚合量算不出置换——需逐笔明细）。
//! 本模块消费 [`ResidualTrade`]（逐笔 δ-free 残差基 r_i=H_i−B̂_i + 成本 C_i + 分层维 h桶/time block），
//! 按 alpha分离.pdf §4.2（p5-6）产出逐桶 perm_p。
//!
//! ## 残差置换（alpha分离.pdf §4.2 / §1，task #82 gap#1+#2）
//!
//! 原 retest 在 δ-baked 的 X_γ 上置换 δ（把固定 X_γ 在买/卖桶间重排）——但 §4.2 的置换统计是
//! **残差** `T^res_π = Σ δ_{π(i)}(H_i − B̂_i) − Σ C_i`：δ-free 基 `r_i=H_i−B̂_i` 固定，置换 δ 后
//! 重算 `Y=δ·r − C`。故本模块对每次置换用 [`ResidualTrade::resid_base`] 重新赋 δ 算 Y，而非重排 X_γ。
//!
//! ## 分层键 (ℓ, h桶, time block, σ^H)（alpha分离.pdf §4.2，p5，task #82 gap#2）
//!
//! `s(i) = (ℓ_i, h bucket, time block, σ_higher)`——补齐 h（持有期桶）+ time block 两维（retest 此前
//! 只有 (ℓ,bsp_class,σ^H)，缺 h/time block ⟹ 同层 B̂_i 不同质 ⟹ beta 从分层漏进 perm_p，§1.2）。
//! **bsp_class 不入分层键**：§4.2 明确分层键只含 (ℓ,h,time,σ_higher)——bsp_class 是被检验的 Z 维
//! （买卖点结构类别），不是要控制的混杂变量。层内置换 δ 时跨 bsp_class 打乱方向标签，检验的是
//! 「给定 (ℓ,h,time,σ^H)，δ 是否携带残差解释力」（§4.2 H0）。输出桶键仍保留 bsp_class 供逐桶三态判据。
//!
//! 加维代价（663 稀释方向）：同一 (ℓ,σ^H) 层被 h/time 进一步细分 ⟹ 每层样本量 n 下降 ⟹ 更容易落入
//! decontam::powered() 的 UNDERPOWERED 门（如实反映——细分层本就更少样本，含糊掉分层维才是虚假高功效）。
//!
//! 认识论 L0（纯代数：置换是逐笔明细的确定性重排统计）；施加于真实数据产出的逐笔明细时才是 L2。
//! 冻结常量（N_PERM=200、种子 20260701）见 acc-alpha-estimand-prereg-20260701.md §4。

use std::collections::{BTreeMap, HashMap};

use super::mu_estimator::{MuClass, ResidualTrade, UClass};

/// 预注册 §4 冻结：置换次数。
pub const N_PERM: usize = 200;
/// 预注册 §4 冻结：Fisher-Yates 起始种子。
pub const PERM_SEED: u64 = 20260701;

/// 逐桶键 (ℓ, bsp_class, δ, σ^H)。σ^H=parent_dir（父声部方向；根声部=0）。
pub type BucketKey = (u32, u8, i8, i8);

/// SplitMix64 确定性 PRNG（手写，不引入 rand 依赖——固定种子可复现是预注册硬约束）。
struct SplitMix64 {
    state: u64,
}
impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

fn fisher_yates<T>(v: &mut [T], rng: &mut SplitMix64) {
    for i in (1..v.len()).rev() {
        let j = rng.below(i + 1);
        v.swap(i, j);
    }
}

/// 通用分层内 δ 置换引擎（残差口径，alpha分离.pdf §4.2，单边右尾）。
///
/// 分层键恒 = (ℓ, h_bucket, time_block, σ^H)（§4.2，跨口径同序 ⟹ 跨进程复现根据）；层内置换 δ；
/// 输出桶由 `base_of`（δ-free 输出基，BTreeMap 确定序）+ `out_key`（从代表类+赋予δ 重构完整输出键，
/// short_swing 等 δ-派生量在此重算，prereg-fullz-policy A3.2）决定。4 元组 / full-z / UClass 三口径
/// 共用本引擎——RNG 只在分层遍历消耗（与输出桶聚合无关）⟹ 三口径切换不影响 4 元组 bit-exact。
fn stratified_delta_perm_p_by<B, K>(
    trades: &[ResidualTrade],
    n_perm: usize,
    seed: u64,
    base_of: impl Fn(&MuClass) -> B,
    out_key: impl Fn(&MuClass, i8) -> K,
) -> HashMap<K, f64>
where
    B: Ord + Copy,
    K: Eq + std::hash::Hash,
{
    let n = trades.len();
    let resid: Vec<f64> = trades.iter().map(|t| t.resid_base).collect();
    let cost: Vec<f64> = trades.iter().map(|t| t.cost).collect();
    let delta0: Vec<i8> = trades.iter().map(|t| t.class.delta).collect();

    // 分层键 (ℓ, h桶, time block, σ^H)——恒定，跨口径同序（RNG 消耗序不可变 ⟹ 跨进程可复现）。
    let mut strata_map: BTreeMap<(u32, u8, u32, i8), Vec<usize>> = BTreeMap::new();
    for (i, t) in trades.iter().enumerate() {
        strata_map
            .entry((t.class.level, t.h_bucket, t.time_block, t.class.parent_dir))
            .or_default()
            .push(i);
    }
    let strata: Vec<Vec<usize>> = strata_map.into_values().collect();

    // 输出桶按 δ-free base 聚合（BTreeMap 确定序）；存代表类（首成员）供 out_key 重构 + obs 均值。
    struct OutBucket<K> {
        plus_key: K,
        minus_key: K,
        members: Vec<usize>,
        obs_plus: Option<f64>,
        obs_minus: Option<f64>,
        ge_plus: usize,
        ge_minus: usize,
    }
    let mut base_map: BTreeMap<B, (MuClass, Vec<usize>)> = BTreeMap::new();
    for (i, t) in trades.iter().enumerate() {
        base_map
            .entry(base_of(&t.class))
            .or_insert_with(|| (t.class, Vec::new()))
            .1
            .push(i);
    }
    let mut buckets: Vec<OutBucket<K>> = base_map
        .into_iter()
        .map(|(_b, (rep, members))| {
            let (mut sp, mut np, mut sm, mut nm) = (0.0_f64, 0usize, 0.0_f64, 0usize);
            for &idx in &members {
                match delta0[idx] {
                    1 => { sp += resid[idx] - cost[idx]; np += 1; }
                    -1 => { sm += -resid[idx] - cost[idx]; nm += 1; }
                    _ => {}
                }
            }
            OutBucket {
                plus_key: out_key(&rep, 1),
                minus_key: out_key(&rep, -1),
                members,
                obs_plus: (np > 0).then(|| sp / np as f64),
                obs_minus: (nm > 0).then(|| sm / nm as f64),
                ge_plus: 0,
                ge_minus: 0,
            }
        })
        .collect();

    // 置换：层内打乱 δ（perm_delta），再逐输出桶按 perm δ 重算残差均值比观测。
    let mut rng = SplitMix64::new(seed);
    let mut perm_delta = vec![0i8; n];
    let mut buf: Vec<i8> = Vec::new();
    for _ in 0..n_perm {
        for members in &strata {
            buf.clear();
            for &idx in members {
                buf.push(delta0[idx]);
            }
            fisher_yates(&mut buf, &mut rng);
            for (k, &idx) in members.iter().enumerate() {
                perm_delta[idx] = buf[k];
            }
        }
        for b in &mut buckets {
            let (mut sp, mut np, mut sm, mut nm) = (0.0_f64, 0usize, 0.0_f64, 0usize);
            for &idx in &b.members {
                match perm_delta[idx] {
                    1 => { sp += resid[idx] - cost[idx]; np += 1; }
                    -1 => { sm += -resid[idx] - cost[idx]; nm += 1; }
                    _ => {}
                }
            }
            if let (Some(o), true) = (b.obs_plus, np > 0) {
                if sp / np as f64 >= o {
                    b.ge_plus += 1;
                }
            }
            if let (Some(o), true) = (b.obs_minus, nm > 0) {
                if sm / nm as f64 >= o {
                    b.ge_minus += 1;
                }
            }
        }
    }

    let inv = n_perm as f64;
    let mut out: HashMap<K, f64> = HashMap::new();
    for b in buckets {
        if b.obs_plus.is_some() {
            out.insert(b.plus_key, b.ge_plus as f64 / inv);
        }
        if b.obs_minus.is_some() {
            out.insert(b.minus_key, b.ge_minus as f64 / inv);
        }
    }
    out
}

/// 4 元组桶键 (ℓ, bsp, δ, σ^H) 逐桶 perm_p（s3/wverify 现口径，bit-exact 不变）。
pub fn stratified_delta_perm_p(
    trades: &[ResidualTrade],
    n_perm: usize,
    seed: u64,
) -> HashMap<BucketKey, f64> {
    stratified_delta_perm_p_by(
        trades,
        n_perm,
        seed,
        |c| (c.level, c.bsp_class(), c.parent_dir),
        |c, d| (c.level, c.bsp_class(), d, c.parent_dir),
    )
}

/// full-z 桶键 = 完整 [`MuClass`] 8 维逐桶 perm_p（prereg-fullz-policy A3.2 + beta-bucket-design v2 §5.3）。
/// δ-free base = (level, i_class, parent_dir, position, horizontal, force_state)；short_swing 由 perm-δ 重构。
///
/// **force_state δ-free**：力度支配态由 A/C 段绝对量算，置换 δ 时恒定（§3.1 mirror-invariant）⟹ 合法进 base。
/// ponytail: force_state 生产未接线前恒 `None`（同一 None 常量分量 ⟹ 分桶不变，向后兼容）；接线后
/// 进 base 前须逐层过 §3.2 δ-共线检查（谱系 iclass-delta-collinearity：δ-纯桶降级仅 μ 分层不置换）。
/// fill-rate 断言（防生产全 None 静默）属 L2 回测报告消费点，随路由接线落地——非本 lib 单元层。
pub fn stratified_delta_perm_p_fullz(
    trades: &[ResidualTrade],
    n_perm: usize,
    seed: u64,
) -> HashMap<MuClass, f64> {
    stratified_delta_perm_p_by(
        trades,
        n_perm,
        seed,
        |c| (c.level, c.i_class, c.parent_dir, c.position, c.horizontal, c.force_state),
        |c, d| MuClass { delta: d, short_swing: c.parent_dir != 0 && d == -c.parent_dir, ..*c },
    )
}

/// UClass 降维桶键逐桶 perm_p（prereg-fullz-policy A3.4，桶碎裂/winner's curse 防护）。
/// δ-free base = (level_bucket, position, parent_dir, divergence)；role 由 perm-δ 经 project_to_u 重构。
pub fn stratified_delta_perm_p_uclass(
    trades: &[ResidualTrade],
    n_perm: usize,
    seed: u64,
) -> HashMap<UClass, f64> {
    stratified_delta_perm_p_by(
        trades,
        n_perm,
        seed,
        |c| (UClass::level_bucket(c.level), c.position, c.parent_dir, c.i_class & 0b001001 != 0),
        |c, d| {
            UClass::project_to_u(&MuClass {
                delta: d,
                short_swing: c.parent_dir != 0 && d == -c.parent_dir,
                ..*c
            })
        },
    )
}

/// 删尾稳健：删除前 `k` 个最大值后的均值（alpha检验.pdf §6，p4：尾部依赖诊断）。
///
/// 返回 `(删尾后均值, 是否符号翻转)`——「剔除前 3 个最大赢家后转负」= 收益依赖数尾部事件（§6）。
/// 主桶 CV 极高（尾部主导疑似）尤需此诊断。样本 ≤k ⟹ 删尾后空 ⟹ 返回 `(0, false)`（无法诊断）。
pub fn drop_top_k_mean(values: &[f64], k: usize) -> (f64, bool) {
    if values.len() <= k {
        return (0.0, false);
    }
    let full_mean = values.iter().sum::<f64>() / values.len() as f64;
    let mut v = values.to_vec();
    // 降序 ⟹ 前 k 大在头部 ⟹ 取 [k..] 即删尾。
    v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let trimmed = &v[k..];
    let trimmed_mean = trimmed.iter().sum::<f64>() / trimmed.len() as f64;
    let flipped = full_mean != 0.0
        && trimmed_mean != 0.0
        && full_mean.signum() != trimmed_mean.signum();
    (trimmed_mean, flipped)
}

/// 方向不对称回归 `X_i = α + βD_i + ε`（D=1 卖 / 0 买；β=μ_sell−μ_buy），alpha检验.pdf §7-§8/§13 Step1 H2。
///
/// 单 dummy OLS ⟹ `β̂ = mean(sell) − mean(buy)`。检验 `H0: β≤0 vs H1: β>0`；事件聚集不能用 iid SE，
/// 用 **block bootstrap** 单边 p（§8：block bootstrap 或 cluster-robust SE）。返回 `(β̂, p)`；任一侧空 ⟹ `(0, 1)`。
/// 输入用残差 Y（alpha分离.pdf §1「所有后续 alpha 检验只看 Y」）——买/卖各自的残差 PnL 序列。
pub fn direction_asymmetry_beta_pvalue(
    buy: &[f64],
    sell: &[f64],
    n_resample: usize,
    block: usize,
    seed: u64,
) -> (f64, f64) {
    if buy.is_empty() || sell.is_empty() {
        return (0.0, 1.0);
    }
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let beta = mean(sell) - mean(buy);
    // block bootstrap 均值（块长保留事件聚集自相关；start 环绕取样）。
    fn boot_mean(v: &[f64], block: usize, rng: &mut SplitMix64) -> f64 {
        let n = v.len();
        let (mut total, mut filled) = (0.0_f64, 0usize);
        while filled < n {
            let start = rng.below(n);
            let take = block.min(n - filled);
            for k in 0..take {
                total += v[(start + k) % n];
            }
            filled += take;
        }
        total / n as f64
    }
    let mut rng = SplitMix64::new(seed);
    let mut n_le_0 = 0usize;
    for _ in 0..n_resample {
        // H0:β≤0 单边 p = #{bootstrap β* ≤ 0}/N。
        let b = boot_mean(sell, block, &mut rng) - boot_mean(buy, block, &mut rng);
        if b <= 0.0 {
            n_le_0 += 1;
        }
    }
    (beta, n_le_0 as f64 / n_resample as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::backtest::mu_estimator::{MuClass, PositionState};
    use crate::theta_v0::types::BspBits;

    /// 构造残差记录：给定 (level, bsp主类, δ, σ^H, resid_base, h_bucket, time_block)，cost=0（置换检验主看基）。
    fn rt(level: u32, bsp: u8, delta: i8, sigma_h: i8, resid_base: f64, h_bucket: u8, time_block: u32) -> ResidualTrade {
        let bits = match (bsp, delta > 0) {
            (1, true) => BspBits { buy1: true, ..Default::default() },
            (1, false) => BspBits { sell1: true, ..Default::default() },
            (2, true) => BspBits { buy2: true, ..Default::default() },
            (2, false) => BspBits { sell2: true, ..Default::default() },
            (_, true) => BspBits { buy3: true, ..Default::default() },
            (_, false) => BspBits { sell3: true, ..Default::default() },
        };
        let class = MuClass::from_certificate(level, delta, bits, sigma_h, PositionState::Root);
        ResidualTrade { class, resid_base, cost: 0.0, h_bucket, time_block }
    }

    #[test]
    fn multi_stratum_reproducible() {
        let t: Vec<ResidualTrade> = (0..90)
            .map(|i| {
                rt(
                    (i % 3) as u32,
                    ((i / 3) % 3 + 1) as u8,
                    if i % 2 == 0 { 1 } else { -1 },
                    if i % 4 < 2 { 1 } else { -1 },
                    (i as f64) * 0.31 - 14.0,
                    (i % 4) as u8,
                    (i % 2) as u32,
                )
            })
            .collect();
        assert_eq!(
            stratified_delta_perm_p(&t, N_PERM, PERM_SEED),
            stratified_delta_perm_p(&t, N_PERM, PERM_SEED),
            "多层同种子须 bit-exact"
        );
    }

    /// H0（δ 与残差独立）：δ 标签与残差基无系统关联 ⟹ perm_p 不显著（> 0.05）。σ^H 恒 0、单 h桶/time block。
    #[test]
    fn h0_independent_delta_not_significant() {
        let trades: Vec<ResidualTrade> = (0..40)
            .map(|i| rt(0, 1, if i % 2 == 0 { 1 } else { -1 }, 0, (i % 5) as f64 - 2.0, 0, 0))
            .collect();
        let p = stratified_delta_perm_p(&trades, N_PERM, PERM_SEED);
        assert!(p[&(0, 1, 1, 0)] > 0.05, "买桶 H0 不应显著: {}", p[&(0, 1, 1, 0)]);
        assert!(p[&(0, 1, -1, 0)] > 0.05, "卖桶 H0 不应显著: {}", p[&(0, 1, -1, 0)]);
    }

    /// 强关联（δ=+1 全高残差基、δ=−1 全低）：买桶残差均值层内极右 ⟹ perm_p 极小（检出）。
    #[test]
    fn strong_delta_signal_is_significant() {
        let mut trades: Vec<ResidualTrade> = Vec::new();
        for _ in 0..30 {
            trades.push(rt(2, 1, 1, 0, 10.0, 0, 0));
            trades.push(rt(2, 1, -1, 0, -10.0, 0, 0));
        }
        let p = stratified_delta_perm_p(&trades, N_PERM, PERM_SEED);
        assert!(p[&(2, 1, 1, 0)] < 0.05, "强信号买桶应显著: {}", p[&(2, 1, 1, 0)]);
    }

    /// h桶分层生效：跨 h桶的一致信号被分层置换检出（§4.2 gap#2）。h桶0 买 r=+10、h桶1 买 r=+20
    /// （同向），输出买桶聚合 obs mean=15；层内置换各桶均值→0 ⟹ obs 极右 ⟹ 显著。
    #[test]
    fn h_bucket_stratifies_independently() {
        let mut trades: Vec<ResidualTrade> = Vec::new();
        for _ in 0..30 {
            trades.push(rt(2, 1, 1, 0, 10.0, 0, 0)); // h桶0：买 r=+10
            trades.push(rt(2, 1, -1, 0, -10.0, 0, 0)); // h桶0：卖 r=−10
            trades.push(rt(2, 1, 1, 0, 20.0, 1, 0)); // h桶1：买 r=+20（同向，不同量级）
            trades.push(rt(2, 1, -1, 0, -20.0, 1, 0)); // h桶1：卖 r=−20
        }
        let p = stratified_delta_perm_p(&trades, N_PERM, PERM_SEED);
        assert!(p[&(2, 1, 1, 0)] < 0.05, "跨 h桶一致买信号经分层置换应检出: {}", p[&(2, 1, 1, 0)]);
    }

    /// σ^H 分层生效：同 (ℓ,h桶) 但 σ^H 不同 ⟹ 不同层，各层独立检出（G-A1 保留）。
    /// 两 σ^H 层各买 r=+10/卖 r=−10，层内置换不跨 σ^H ⟹ 各层买桶独立显著。
    #[test]
    fn parent_dir_stratifies_independently() {
        let mut trades: Vec<ResidualTrade> = Vec::new();
        for _ in 0..30 {
            trades.push(rt(2, 1, 1, 1, 10.0, 0, 0)); // σ^H=+1 层：买 r=+10
            trades.push(rt(2, 1, -1, 1, -10.0, 0, 0));
            trades.push(rt(2, 1, 1, -1, 10.0, 0, 0)); // σ^H=−1 层：买 r=+10
            trades.push(rt(2, 1, -1, -1, -10.0, 0, 0));
        }
        let p = stratified_delta_perm_p(&trades, N_PERM, PERM_SEED);
        assert!(p[&(2, 1, 1, 1)] < 0.05, "σ^H=+1 层买桶应独立检出: {}", p[&(2, 1, 1, 1)]);
        assert!(p[&(2, 1, 1, -1)] < 0.05, "σ^H=−1 层买桶应独立检出: {}", p[&(2, 1, 1, -1)]);
        assert_eq!(p.len(), 4, "两个 σ^H 层各产 2 个 δ 桶");
    }

    /// 残差口径生效：置换用 δ-free 基 resid_base（非 δ-baked X_γ）——当 r 与真实 δ **无关**（买卖同 r）
    /// 时，置换 δ 不改变桶均值 ⟹ perm_p≈1（残差检验正确判无结构）。这与旧 X_γ 置换的关键区别：
    /// X_γ=δ·H 把 δ 烘进值，即使 r 无结构，重排 X_γ 也会让买桶均值随机偏离 obs ⟹ 虚假显著。
    #[test]
    fn residual_permutation_finds_no_structure_when_r_constant() {
        // resid_base 全相同（=5），Y=δ·5 完全由 δ 决定。买桶 obs mean(r−c)=5，任何 δ 置换后买桶仍全是
        // r=5 的成员 ⟹ mean 恒 5=obs ⟹ perm_p=1（δ 标签对残差无解释力，§4.2 H0 不拒绝）。
        let mut trades: Vec<ResidualTrade> = Vec::new();
        for _ in 0..30 {
            trades.push(rt(1, 1, 1, 0, 5.0, 0, 0));
            trades.push(rt(1, 1, -1, 0, 5.0, 0, 0)); // 同 resid_base，δ 号相反
        }
        let p = stratified_delta_perm_p(&trades, N_PERM, PERM_SEED);
        assert!(p[&(1, 1, 1, 0)] > 0.5, "r 无结构 ⟹ 残差置换判无显著（区别于 X_γ 置换的虚假显著）: {}", p[&(1, 1, 1, 0)]);
    }

    /// full-z / UClass 扩维 L1 自检（prereg-fullz-policy A3.3）：(1) 同种子复现；(2) full-z 是 4 元组
    /// 的细化——每个 full-z 桶键投影到 (ℓ,bsp,δ,σ^H) 必落在 4 元组输出里（细分不产新粗桶）。
    #[test]
    fn fullz_uclass_reproducible_and_refines() {
        let t: Vec<ResidualTrade> = (0..120)
            .map(|i| {
                rt(
                    (i % 3) as u32,
                    ((i / 3) % 3 + 1) as u8,
                    if i % 2 == 0 { 1 } else { -1 },
                    if i % 4 < 2 { 1 } else { -1 }, // σ^H ∈ {+1,−1} ⟹ short_swing 随 δ 可变
                    (i as f64) * 0.29 - 17.0,
                    (i % 4) as u8,
                    (i % 2) as u32,
                )
            })
            .collect();
        // (1) 同种子复现。
        assert_eq!(
            stratified_delta_perm_p_fullz(&t, N_PERM, PERM_SEED),
            stratified_delta_perm_p_fullz(&t, N_PERM, PERM_SEED),
            "full-z 同种子须 bit-exact"
        );
        assert_eq!(
            stratified_delta_perm_p_uclass(&t, N_PERM, PERM_SEED),
            stratified_delta_perm_p_uclass(&t, N_PERM, PERM_SEED),
            "UClass 同种子须 bit-exact"
        );
        // (2) full-z 细化：每个 full-z 键投影为 4 元组必存在于 4 元组输出（细分不越出粗桶集）。
        let coarse = stratified_delta_perm_p(&t, N_PERM, PERM_SEED);
        let fullz = stratified_delta_perm_p_fullz(&t, N_PERM, PERM_SEED);
        assert!(fullz.len() >= coarse.len(), "full-z 桶数 ≥ 4 元组桶数（细化）");
        for z in fullz.keys() {
            let proj = (z.level, z.bsp_class(), z.delta, z.parent_dir);
            assert!(coarse.contains_key(&proj), "full-z 桶 {z:?} 投影 {proj:?} 须落在 4 元组输出");
        }
    }

    /// 同种子两跑 bit-exact（预注册可复现硬约束）。
    #[test]
    fn same_seed_reproducible() {
        let trades: Vec<ResidualTrade> = (0..50)
            .map(|i| {
                rt(
                    1,
                    2,
                    if i % 3 == 0 { 1 } else { -1 },
                    if i % 5 == 0 { 1 } else { 0 },
                    (i as f64) * 0.37 - 9.0,
                    (i % 3) as u8,
                    0,
                )
            })
            .collect();
        let a = stratified_delta_perm_p(&trades, N_PERM, PERM_SEED);
        let b = stratified_delta_perm_p(&trades, N_PERM, PERM_SEED);
        assert_eq!(a, b);
    }

    /// 置换保留 δ 边际（多重集与计数不变，§4.2）。
    #[test]
    fn permutation_preserves_delta_marginal() {
        let mut rng = SplitMix64::new(PERM_SEED);
        let orig: Vec<i8> = vec![1, 1, -1, 1, -1, -1, 1, -1];
        let mut v = orig.clone();
        fisher_yates(&mut v, &mut rng);
        let mut a = orig.clone();
        let mut b = v.clone();
        a.sort();
        b.sort();
        assert_eq!(a, b, "置换后 δ 多重集不变");
        assert_eq!(v.iter().filter(|&&d| d == 1).count(), 4, "买标签计数不变");
    }

    /// 删尾稳健（§6）：正均值靠少数尾部赢家 ⟹ 删前 3 后转负（符号翻转）。
    #[test]
    fn drop_top_k_flips_sign_when_tail_dominated() {
        // 20 笔 −1 + 3 笔巨赢 +100 ⟹ 全量均值 = (−20 + 300)/23 ≈ +12.2>0；删前 3 赢家 ⟹ 全 −1 ⟹ −1<0。
        let mut v = vec![-1.0_f64; 20];
        v.extend([100.0, 100.0, 100.0]);
        let (trimmed, flipped) = drop_top_k_mean(&v, 3);
        assert!(trimmed < 0.0, "删前3赢家后转负: {trimmed}");
        assert!(flipped, "符号翻转（尾部依赖）");
        // 均匀正样本：删尾不翻转。
        let (_, f2) = drop_top_k_mean(&[5.0, 6.0, 5.5, 6.5, 5.0, 6.0], 3);
        assert!(!f2, "均匀正样本删尾不翻转");
        // 样本 ≤k ⟹ 无法诊断。
        assert_eq!(drop_top_k_mean(&[1.0, 2.0], 3), (0.0, false));
    }

    /// H2 方向不对称（§7-§8）：sell 残差系统性高于 buy ⟹ β>0 且单边 p 小（拒绝 H0:β≤0）。
    #[test]
    fn direction_asymmetry_detects_sell_over_buy() {
        let buy = vec![-5.0_f64; 60];
        let sell = vec![5.0_f64; 60];
        let (beta, p) = direction_asymmetry_beta_pvalue(&buy, &sell, 1000, 20, PERM_SEED);
        assert!((beta - 10.0).abs() < 1e-9, "β=μ_sell−μ_buy=5−(−5)=10: {beta}");
        assert!(p < 0.05, "sell≫buy ⟹ 拒绝 H0:β≤0: {p}");
        // 对称（buy=sell）⟹ β≈0 ⟹ p 不显著。
        let (b2, p2) = direction_asymmetry_beta_pvalue(&vec![1.0; 40], &vec![1.0; 40], 1000, 20, PERM_SEED);
        assert!((b2).abs() < 1e-9 && p2 > 0.05, "对称 ⟹ β=0 p 不显著: β={b2} p={p2}");
        // 空侧 ⟹ (0,1)。
        assert_eq!(direction_asymmetry_beta_pvalue(&[], &sell, 1000, 20, PERM_SEED), (0.0, 1.0));
    }

    // ── 跨进程复现（预注册 §4 硬约束的真语义，D1）──────────────────────────────
    // same_seed_reproducible 仅证同进程连续调用一致；「跨进程可复现」需独立进程重跑。
    // 机制：fork 自身测试二进制，只跑 worker（唯一子串 filter），种子经 env 传入，
    // 逐字节比较序列化输出。HashMap 迭代序逐进程随机 → 序列化必须排序键，否则同结果也字节不同。
    const XPROC_BEGIN: &str = "@@PERM_XPROC_BEGIN@@\n";
    const XPROC_END: &str = "@@PERM_XPROC_END@@\n";

    /// worker 入口：仅在 PERM_XPROC_SEED 置位时运行；普通 cargo test 下 no-op。
    #[test]
    fn perm_xproc_child() {
        let seed = match std::env::var("PERM_XPROC_SEED") {
            Ok(s) => s.parse::<u64>().expect("PERM_XPROC_SEED 须为 u64"),
            Err(_) => return,
        };
        // 粗分层（level∈{0,1} × h桶∈{0,1}，σ^H=0/time_block=0）⟹ 每层 ~22 混合-δ 成员 ⟹ 置换随种子变化
        // （细分层会让每层单 δ ⟹ 置换恒等 ⟹ perm_p 全 1.0 无种子敏感度，红 demo 失去鉴别力）。
        let trades: Vec<ResidualTrade> = (0..88)
            .map(|i| {
                rt(
                    ((i / 44) % 2) as u32,        // level：前 44 level0 / 后 44 level1
                    1,
                    if i % 2 == 0 { 1 } else { -1 }, // δ 每步交替（与 level/h 解耦 ⟹ 层内混合-δ）
                    0,
                    (i as f64) * 0.31 - 14.0,
                    ((i / 22) % 2) as u8,          // h桶：每 22 一段
                    0,
                )
            })
            .collect();
        let out = stratified_delta_perm_p(&trades, N_PERM, seed);
        let mut keys: Vec<BucketKey> = out.keys().copied().collect();
        keys.sort_unstable(); // 跨进程逐字节可比的前提
        let mut body = String::new();
        for k in keys {
            body.push_str(&format!("{},{},{},{}={:.17}\n", k.0, k.1, k.2, k.3, out[&k]));
        }
        print!("{}{}{}", XPROC_BEGIN, body, XPROC_END);
    }

    fn run_child(seed: u64) -> String {
        let exe = std::env::current_exe().expect("current_exe");
        let out = std::process::Command::new(exe)
            .args(["--nocapture", "perm_xproc_child"])
            .env("PERM_XPROC_SEED", seed.to_string())
            .output()
            .expect("spawn 子测试进程");
        assert!(out.status.success(), "子进程失败: {}", String::from_utf8_lossy(&out.stderr));
        let s = String::from_utf8(out.stdout).expect("子进程 stdout 非 utf8");
        let b = s.find(XPROC_BEGIN).expect("子进程输出缺 BEGIN marker") + XPROC_BEGIN.len();
        let e = s.find(XPROC_END).expect("子进程输出缺 END marker");
        s[b..e].to_string()
    }

    /// 跨进程：两独立进程同种子须逐字节一致；扰动种子须改变输出（内建 red demo 证鉴别力）。
    #[test]
    fn perm_xproc_reproducible() {
        let a1 = run_child(PERM_SEED);
        let a2 = run_child(PERM_SEED);
        assert!(!a1.is_empty(), "worker 未产出（marker/env 未生效）");
        assert_eq!(a1, a2, "跨进程同种子须逐字节一致");
        let b = run_child(PERM_SEED ^ 0x9E37_79B9_7F4A_7C15);
        assert_ne!(a1, b, "扰动种子应改变输出——否则本复现测试无鉴别力");
    }
}
