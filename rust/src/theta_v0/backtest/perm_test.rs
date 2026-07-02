//! 路径 A 分层内 δ 置换检验（acc-alpha 预注册 §1.4，perm_p 生产者）。
//!
//! decontam.rs 三态判据消费逐桶 perm_p 但不生产它（Welford 聚合量算不出置换——需逐笔明细）。
//! 本模块消费 MuEstimator 留存的逐笔 (ℓ,bsp_class,δ,X_γ)，按 §1.4 路径 A 产出逐桶 perm_p：
//! 每个 (ℓ,bsp_class) 层内随机置换 δ 标签重算 μ̂_perm，perm_p = #{μ̂_perm ≥ μ̂_obs}/N（单边）。
//! 层内置换保留 (ℓ,bsp_class) 边际，只打乱 δ↔X_γ 关联。
//!
//! 认识论 L0（纯代数：置换是逐笔明细的确定性重排统计）；施加于真实数据产出的逐笔明细时才是 L2。
//! 冻结常量（N_PERM=200、种子 20260701）见 acc-alpha-estimand-prereg-20260701.md §4。

use std::collections::{BTreeMap, HashMap};

/// 预注册 §4 冻结：置换次数。
pub const N_PERM: usize = 200;
/// 预注册 §4 冻结：Fisher-Yates 起始种子。
pub const PERM_SEED: u64 = 20260701;

/// 逐桶键 (ℓ, bsp_class, δ)。
pub type BucketKey = (u32, u8, i8);

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

/// 层内两桶均值单遍融合：δ=+1 与 δ=−1 各自独立累加（勿用 total−plus 派生——
/// 独立累加保 mp/mm 与旧 bucket_mean 逐加序 bit-exact）。
fn bucket_means(ds: &[i8], xs: &[f64]) -> (Option<f64>, Option<f64>) {
    let (mut sum_p, mut n_p) = (0.0_f64, 0_usize);
    let (mut sum_m, mut n_m) = (0.0_f64, 0_usize);
    for (&d, &x) in ds.iter().zip(xs) {
        if d == 1 {
            sum_p += x;
            n_p += 1;
        } else if d == -1 {
            sum_m += x;
            n_m += 1;
        }
    }
    (
        (n_p > 0).then(|| sum_p / n_p as f64),
        (n_m > 0).then(|| sum_m / n_m as f64),
    )
}

/// 每层记录：obs 均值 + ge 计数内联，消 perm 内 HashMap 查找（引用提外，c）。
struct Stratum {
    key: (u32, u8),
    ds: Vec<i8>,
    xs: Vec<f64>,
    obs_plus: Option<f64>,
    obs_minus: Option<f64>,
    ge_plus: usize,
    ge_minus: usize,
}

/// 路径 A 分层内 δ 置换，产出逐桶 perm_p（预注册 §1.4，单边右尾）。
/// trades：逐笔 (ℓ, bsp_class, δ, X_γ)。空 ⟹ 空 map。
pub fn stratified_delta_perm_p(
    trades: &[(u32, u8, i8, f64)],
    n_perm: usize,
    seed: u64,
) -> HashMap<BucketKey, f64> {
    let mut grouped: BTreeMap<(u32, u8), (Vec<i8>, Vec<f64>)> = BTreeMap::new(); // 确定序⟹跨进程 perm_p 可复现(prereg §4)
    for &(lv, bc, d, x) in trades {
        let e = grouped.entry((lv, bc)).or_default();
        e.0.push(d);
        e.1.push(x);
    }
    // BTreeMap 序 → Vec：锁定 stratum 迭代序（= RNG 消耗序，不可变），并内联 obs/ge。
    let mut strata: Vec<Stratum> = grouped
        .into_iter()
        .map(|(key, (ds, xs))| {
            let (obs_plus, obs_minus) = bucket_means(&ds, &xs);
            Stratum { key, ds, xs, obs_plus, obs_minus, ge_plus: 0, ge_minus: 0 }
        })
        .collect();

    let mut rng = SplitMix64::new(seed);
    let mut perm_ds: Vec<i8> = Vec::new(); // 缓冲提外，逐 stratum copy_from_slice 复用容量（a）
    for _ in 0..n_perm {
        for s in &mut strata {
            perm_ds.resize(s.ds.len(), 0);
            perm_ds.copy_from_slice(&s.ds);
            fisher_yates(&mut perm_ds, &mut rng);
            let (mp, mm) = bucket_means(&perm_ds, &s.xs);
            if let (Some(mp), Some(o)) = (mp, s.obs_plus) {
                if mp >= o {
                    s.ge_plus += 1;
                }
            }
            if let (Some(mm), Some(o)) = (mm, s.obs_minus) {
                if mm >= o {
                    s.ge_minus += 1;
                }
            }
        }
    }

    let inv = n_perm as f64;
    let mut out: HashMap<BucketKey, f64> = HashMap::new();
    for s in &strata {
        let (lv, bc) = s.key;
        if s.obs_plus.is_some() {
            out.insert((lv, bc, 1), s.ge_plus as f64 / inv);
        }
        if s.obs_minus.is_some() {
            out.insert((lv, bc, -1), s.ge_minus as f64 / inv);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_stratum_reproducible() {
        let t: Vec<(u32,u8,i8,f64)> = (0..90).map(|i| ((i%3) as u32,((i/3)%3+1) as u8, if i%2==0 {1} else {-1}, (i as f64)*0.31-14.0)).collect();
        assert_eq!(stratified_delta_perm_p(&t,N_PERM,PERM_SEED), stratified_delta_perm_p(&t,N_PERM,PERM_SEED), "多层同种子须 bit-exact");
    }

    /// H0（δ 与 X_γ 独立）：δ 标签与幅度无系统关联 ⟹ perm_p 不显著（> 0.05）。
    #[test]
    fn h0_independent_delta_not_significant() {
        let trades: Vec<(u32, u8, i8, f64)> = (0..40)
            .map(|i| (0u32, 1u8, if i % 2 == 0 { 1 } else { -1 }, (i % 5) as f64 - 2.0))
            .collect();
        let p = stratified_delta_perm_p(&trades, N_PERM, PERM_SEED);
        assert!(p[&(0, 1, 1)] > 0.05, "买桶 H0 不应显著: {}", p[&(0, 1, 1)]);
        assert!(p[&(0, 1, -1)] > 0.05, "卖桶 H0 不应显著: {}", p[&(0, 1, -1)]);
    }

    /// 强关联（δ=+1 全高、δ=−1 全低）：买桶 μ̂ 是层内极右 ⟹ perm_p 极小（检出）。
    #[test]
    fn strong_delta_signal_is_significant() {
        let mut trades: Vec<(u32, u8, i8, f64)> = Vec::new();
        for _ in 0..30 {
            trades.push((2, 1, 1, 10.0));
            trades.push((2, 1, -1, -10.0));
        }
        let p = stratified_delta_perm_p(&trades, N_PERM, PERM_SEED);
        assert!(p[&(2, 1, 1)] < 0.05, "强信号买桶应显著: {}", p[&(2, 1, 1)]);
    }

    /// 同种子两跑 bit-exact（预注册可复现硬约束）。
    #[test]
    fn same_seed_reproducible() {
        let trades: Vec<(u32, u8, i8, f64)> = (0..50)
            .map(|i| (1u32, 2u8, if i % 3 == 0 { 1 } else { -1 }, (i as f64) * 0.37 - 9.0))
            .collect();
        let a = stratified_delta_perm_p(&trades, N_PERM, PERM_SEED);
        let b = stratified_delta_perm_p(&trades, N_PERM, PERM_SEED);
        assert_eq!(a, b);
    }

    /// 置换保留 δ 边际（多重集与计数不变，§1.4）。
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
}
