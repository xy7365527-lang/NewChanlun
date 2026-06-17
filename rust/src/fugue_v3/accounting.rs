//! 会计层（两投影之会计投影，reinterp §1/§5：操作层 ⊥ 会计层）。
//!
//! ## NAV（物理单真值，sign(ε) 模型）
//! `NAV = free + Σ_k sign(d_k)·u_k·c`（Long:+u·c 市值 / Short:−u·c 负债 MtM）。每个 reduce/add
//! 现金流在同价 c 下 NAV 中性（手算：reduce_at(Long) free+=m·c 抵消市值 −m·c；空头层镜像）。
//!
//! ## 会计双重性（多方/空方，同一 τ 按 direction 镜像）
//! - reduce_at(Long)=卖出 free+=m·c；reduce_at(Short)=平空 cover free−=m·c。
//! - add_at(Long)=买入 free−=m·c；add_at(Short)=开空 free+=m·c。
//! 四者皆 NAV 中性。**完整 sink→recover 循环净 free += 2m(c−c')** = 穿 ε=−1 的 2× 降成本 alpha。
//!
//! ## 守恒（Σ|units| = n_base，T48 σ-不变 Casimir）
//! sink/recover 在相邻级别间转移 m（一减一加），Σ|units| 不变。仅 F/C/A 改 n_base。
//!
//! ## 认识论等级
//! NAV 中性 / Σ|units| 守恒：**L0**（守恒律）；basis 加权 / trade 记录：**L0**（纯会计）。

use crate::trading::positional::LayerTrade;
use crate::trading::types::{Polarity, MAX_LADDER};

use super::layer::{FugueResult, Layer};
use super::MOBILE_FRAC;

/// 方向 ε 翻转（τ 的操作投影）。
pub fn flip(d: Polarity) -> Polarity {
    match d {
        Polarity::Long => Polarity::Short,
        Polarity::Short => Polarity::Long,
    }
}

// ════════════════════════════ NAV（物理单真值，sign(ε)·u·c）════════════════════════════

/// 物理 NAV：自由现金 + Σ 各级别 sign(direction)·units·c（Long 市值 / Short 负债 MtM）。
pub fn nav(layers: &[Layer], free: f64, c: f64) -> f64 {
    let mut v = free;
    for l in layers {
        if l.units > 0.0 {
            v += match l.direction {
                Polarity::Long => l.units * c,
                Polarity::Short => -l.units * c,
            };
        }
    }
    v
}

/// 守恒律左项：Σ|units|（多空都计绝对值，T48 Casimir）。
pub fn total_units_held(layers: &[Layer]) -> f64 {
    layers.iter().map(|l| l.units).sum()
}

/// 物理暴露：(多头 units, 空头 units)。
pub fn exposure(layers: &[Layer]) -> (f64, f64) {
    let mut long_u = 0.0;
    let mut short_u = 0.0;
    for l in layers {
        if l.units > 0.0 {
            match l.direction {
                Polarity::Long => long_u += l.units,
                Polarity::Short => short_u += l.units,
            }
        }
    }
    (long_u, short_u)
}

/// 活跃空头声部数（多声部 L6 观测：当前做空的层数）。
pub fn active_voice_count(layers: &[Layer]) -> usize {
    layers
        .iter()
        .filter(|l| l.units > 0.0 && l.direction == Polarity::Short)
        .count()
}

// ════════════════════════════ σ-不变配额（T18×T48，542号）════════════════════════════

/// 单次 τ 转移配额 `m = f × units`，`f = 1/λ` 级别无关（σ-不变常数）。
pub fn mobile_quota(units: f64) -> f64 {
    units * MOBILE_FRAC
}

// ════════════════════════════ reduce_at / add_at（τ 的两腿会计）════════════════════════════

/// 在级别 k **减仓** m（关闭一个 chunk）：按 direction 双重会计 + 记 closed trade + 累计 realized pnl。
///
/// reduce_at(Long)=卖出 free+=m·c；reduce_at(Short)=平空 cover free−=m·c。NAV 中性。
#[allow(clippy::too_many_arguments)]
pub fn reduce_at(
    layers: &mut [Layer],
    k: usize,
    m: f64,
    free: &mut f64,
    c: f64,
    bar: i64,
    res: &mut FugueResult,
    reason: &'static str,
) {
    let (d, b, entry_bar) = {
        let l = &layers[k];
        (l.direction, l.basis, l.entry_bar)
    };
    let pnl = match d {
        Polarity::Long => m * (c - b),
        Polarity::Short => m * (b - c),
    };
    match d {
        Polarity::Long => *free += m * c,
        Polarity::Short => *free -= m * c,
    }
    record_trade(res, k, entry_bar.max(0), b, bar, c, m, reason, d);
    res.mobile_realized_pnl_by_ladder[k] += pnl;
    let l = &mut layers[k];
    l.units -= m;
    if l.units <= 1e-12 {
        l.units = 0.0;
        l.basis = f64::NAN;
        l.entry_bar = -1;
    }
}

/// 在级别 k **加仓** m（开一个 chunk，方向 dir）：按 dir 双重会计 + basis 加权（手性交替守卫）。
///
/// add_at(Long)=买入 free−=m·c；add_at(Short)=开空 free+=m·c。NAV 中性。
#[allow(clippy::too_many_arguments)]
pub fn add_at(
    layers: &mut [Layer],
    k: usize,
    m: f64,
    dir: Polarity,
    free: &mut f64,
    c: f64,
    bar: i64,
    _res: &mut FugueResult,
) {
    match dir {
        Polarity::Long => *free -= m * c,
        Polarity::Short => *free += m * c,
    }
    let l = &mut layers[k];
    if l.units > 1e-12 {
        // 层内方向一致性：加到占用级别必须同向（一个 Layer 单一 direction，不可多空并存）。
        assert_eq!(
            l.direction, dir,
            "add_at 手性违反@bar {bar} layer {k}：向 {:?} 级别加 {:?} chunk（对冲净额）",
            l.direction, dir
        );
        l.basis = (l.basis * l.units + c * m) / (l.units + m);
    } else {
        l.direction = dir;
        l.basis = c;
        l.entry_bar = bar;
    }
    l.units += m;
}

// ════════════════════════════ trade 行记录 ════════════════════════════

/// 记录一笔 trade 行（trade11 契约，复用 `LayerTrade`）。
#[allow(clippy::too_many_arguments)]
pub fn record_trade(
    res: &mut FugueResult,
    ladder: usize,
    entry_bar: i64,
    entry_price: f64,
    exit_bar: i64,
    exit_price: f64,
    shares: f64,
    reason: &'static str,
    polarity: Polarity,
) {
    res.trades.push(LayerTrade {
        ladder: ladder as u8,
        entry_bar,
        entry_price,
        exit_bar,
        exit_price,
        shares,
        weight_at_entry: 1.0,
        deferred_bars: 0,
        partial: false,
        exit_reason: reason,
        polarity,
    });
}

/// 各层短差做空持有 bar 累计（观测，每 bar 调用）。
pub fn accrue_held_bars(res: &mut FugueResult, layers: &[Layer]) {
    for l in layers {
        if l.units > 0.0 && l.direction == Polarity::Short {
            res.short_held_bars_by_ladder[l.ladder.min(MAX_LADDER - 1)] += 1;
        }
    }
}
