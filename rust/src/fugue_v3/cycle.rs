//! τ 操作（D∞ word 的核心字母）：sink（σ⁻¹∘τ 下沉）/ recover（σ∘τ 升回）/ liquidate（边界 A）。
//!
//! ## 不硬编码四步循环（用户裁决 2026-06-17）
//! 引擎**不预设**「平多→开空→平空→做多」四步序。它只有两个 τ 原子：
//! - `sink_chunk`（nf_sell[k] 触发）：从 k 取 f·u_k，**翻手性**下沉到 k−1（σ⁻¹∘τ，穿手性缝）。
//! - `recover_chunk`（nf_buy[k−1] 触发）：从 k−1 取 f·u_{k−1}，**翻回**升到 k（σ∘τ）。
//!   触发用**次级别 k−1** 买点（子仓物理在 k−1 层，级别锚定修复见 operate.rs D 步骤）。
//! 「四步循环」= sink 后接 recover 的**涌现序列**（被 nf_sell[k]/nf_buy[k−1] 分别触发），不是状态机。
//!
//! ## 守恒（sink/recover 不改 Σ|units|，仅级别间转移）
//! 每个 τ = reduce_at(源) + add_at(目标)，转移 m，Σ|units| 不变（n_base 不变）。NAV 中性（同价 c）。
//!
//! ## 认识论等级
//! τ 原子结构 / Δr=−1 / 手性翻转：**L0**（D∞ 群作用）；激活时机：**L2 regime**（nf 是否 fire）。

use crate::trading::types::Polarity;

use super::accounting::{add_at, flip, mobile_quota, reduce_at};
use super::layer::{FugueResult, Layer};
use super::prove::{prove_cross_level_closure, prove_sigma_quota};

/// **sink @ k**（nf_sell[k] fire ⇒ σ⁻¹∘τ）：从 k 取 m=f·u_k，翻手性下沉到 k−1（穿 ε 翻转缝）。
///
/// 前提（operate 已校验）：`layers[k].units > 0` ∧ 本 bar 未动作 ∧ 成本门通过 ∧ k≥PENDING_LO。
/// 返回是否成功下沉。守恒：m 从 k 移到 k−1（一减一加），Σ|units| 不变。
#[allow(clippy::too_many_arguments)]
pub fn sink_chunk(
    layers: &mut [Layer],
    k: usize,
    free: &mut f64,
    c: f64,
    bar: i64,
    res: &mut FugueResult,
) -> bool {
    let u_k = layers[k].units;
    let m = mobile_quota(u_k);
    if !(m > 0.0 && m.is_finite()) || m > u_k + 1e-9 {
        return false;
    }
    let d_k = layers[k].direction;
    let sub = k - 1; // Δr=−1（k≥PENDING_LO>FIRST_BSP ⇒ k−1≥FIRST_BSP，有次级别）
    // 子层方向相容前提（add_at 层内单一 direction 不变量）：sink 把 flip(d_k) 开到 sub。
    // 若 sub 已占用且方向 != flip(d_k)（= 与父同向，建仓阶段相邻同向 Long）⟹ 无法开短差 ⟹ 拒绝（操作不适用，N4 类比）。
    if layers[sub].units > 1e-12 && layers[sub].direction != flip(d_k) {
        return false;
    }
    prove_sigma_quota(m, u_k, k as i64, bar);
    prove_cross_level_closure(k, sub);

    // 操作投影：τ 翻 direction（d_k → flip）。会计投影：m 在 k↔k−1 转移。
    reduce_at(layers, k, m, free, c, bar, res, "reduce"); // 源层减仓（卖/cover）
    add_at(layers, sub, m, flip(d_k), free, c, bar, res); // 目标层开 flip(d_k) chunk（穿 ε=−1）

    res.n_cycle_opens_by_ladder[k] += 1;
    res.cross_level_closures += 1;
    true
}

/// **recover @ k**（ε 对称，σ∘τ）：从 k−1 取 m=f·u_{k−1}，翻回手性升到 k（成本下移）。
///
/// 前提（operate 已校验）：`layers[k−1].units > 0` ∧ 本 bar 未动作 ∧ k≥PENDING_LO。
/// `parent_dir`：根极性（Long root → parent=Long，期望子层=Short；Short root → parent=Short，期望子层=Long）。
/// 守恒：m 从 k−1 移到 k，Σ|units| 不变。完整 sink→recover 净 free += 2m(c_sink − c_recover)。
pub fn recover_chunk(
    layers: &mut [Layer],
    k: usize,
    parent_dir: Polarity,
    free: &mut f64,
    c: f64,
    bar: i64,
    res: &mut FugueResult,
) -> bool {
    let sub = k - 1;
    let u_sub = layers[sub].units;
    let m = mobile_quota(u_sub);
    if !(m > 0.0 && m.is_finite()) || m > u_sub + 1e-9 {
        return false;
    }
    let d_sub = layers[sub].direction;
    // ε 对称性：子层方向必须是 flip(parent_dir)（sink 穿过 ε 缝后子层方向为父方向的反向）。
    // Long root → 子层期望 Short；Short root → 子层期望 Long。
    let expected_child = flip(parent_dir);
    if d_sub != expected_child {
        return false;
    }
    // 父层方向相容前提（add_at 不变量）：升回 flip(d_sub)=parent_dir 到父层 k；
    // 若 k 已占用且方向 != parent_dir ⟹ 拒绝（操作不适用）。
    if layers[k].units > 1e-12 && layers[k].direction != flip(d_sub) {
        return false;
    }
    prove_sigma_quota(m, u_sub, sub as i64, bar);
    prove_cross_level_closure(k, sub);

    reduce_at(layers, sub, m, free, c, bar, res, "recover"); // 次级别平 chunk（cover/卖）
    add_at(layers, k, m, flip(d_sub), free, c, bar, res); // 升回本级别（翻回 parent_dir）

    res.n_cycle_closes_by_ladder[k] += 1;
    true
}

/// **liquidate_short @ k**（边界 A，1x 逐仓 c≥2×basis）：空头子仓强制 cover，units 永久缩水。
/// 会计终局（市场被动，非 τ 正常操作）——单边上扬穿 ε=−1 的踏空惩罚。返回缩水的 units 量。
pub fn liquidate_short(
    layers: &mut [Layer],
    k: usize,
    free: &mut f64,
    c: f64,
    bar: i64,
    res: &mut FugueResult,
) -> f64 {
    let m = layers[k].units;
    reduce_at(layers, k, m, free, c, bar, res, "liq_short");
    res.n_liquidations_by_ladder[k] += 1;
    m
}

/// **liquidate_long @ k**（边界 A，1x 逐仓 c≤basis/2）：多头子仓强制卖出，units 永久缩水。
/// Short root 对称：单边下跌穿 ε=+1 的踏空惩罚（与 liquidate_short 镜像）。
pub fn liquidate_long(
    layers: &mut [Layer],
    k: usize,
    free: &mut f64,
    c: f64,
    bar: i64,
    res: &mut FugueResult,
) -> f64 {
    let m = layers[k].units;
    reduce_at(layers, k, m, free, c, bar, res, "liq_long");
    res.n_liquidations_by_ladder[k] += 1;
    m
}
