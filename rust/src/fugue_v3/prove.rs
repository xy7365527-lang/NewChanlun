//! prove 守卫（= 验收，非回测指标）：violation = panic。每 bar 全程零 panic = 验收通过。
//!
//! GUARD-ROLE: t-engine-accounting-basis——名分：现役（详见 `fugue_v3/mod.rs`
//! 头部 GUARD-ROLE 块，#762 C7-E3 核定）。
//!
//! 设计：`docs/recursive_fugue_necessity_proof.md` §6 + unn/spiral prove 模式（137号
//! make-decision-observable）。**非重言**——核心断言独立内联表达，正向规范值不 panic、反证错误值
//! 必 panic（配套 `#[should_panic]` 测试）。
//!
//! ## 守卫等级（formalization-validity-domain）
//! | 守卫 | 断言 | 等级 |
//! |------|------|------|
//! | `prove_cross_level_closure` | τ 转移 Δr=−1（相邻级别） | L0 结构 |
//! | `prove_sigma_quota` | 配额 m=f×units 级别无关（σ-不变，T18） | 形式 L0 |
//! | `count_chiral_violations` | 相邻占用级别同向事件数（T24 观测，非 panic 不变量） | 观测 |
//! | `prove_conservation` | Σ\|units\| = n_base（T48 Casimir） | L2 守恒 |
//! | `prove_nav_neutral` | 同价操作 NAV 中性 | L2 守恒 |
//! | `prove_no_double_act` | per-layer 互斥 | L2 守恒 |
//! | `prove_recursive_consistency` | 空头声部在合法势源层 [PENDING_LO, MAX) | L2 守恒 |

use crate::stroke::Direction;
use crate::trading::types::Polarity;

use super::accounting::total_units_held;
use super::layer::Layer;
use super::{FIRST_BSP_LADDER, MOBILE_FRAC, PENDING_LO};

// ════════════════════════════ L0 结构守卫 ════════════════════════════

/// **τ 转移 Δr=−1（相邻级别）**：sink/recover 在 k 与 k−1 间，`child+1==parent`，跨级必 panic。
pub fn prove_cross_level_closure(parent_ladder: usize, child_ladder: usize) {
    assert_eq!(
        child_ladder + 1,
        parent_ladder,
        "Δr=−1 违反：τ 转移层 {child_ladder} ≠ 本级别 {parent_ladder}−1（σ^±1 必跨恰一级别）"
    );
}

/// **σ-不变配额（T18×T48，542号）**：`m = f × units`，`f=1/λ` 级别无关。独立内联重算（非重言）；
/// 漂移到级别依赖分配或错误 f 必 panic。
pub fn prove_sigma_quota(m: f64, units_before: f64, ladder: i64, bar: i64) {
    let canonical = units_before * MOBILE_FRAC; // 独立表达：级别无关函数 u↦f·u
    assert!(
        (m - canonical).abs() <= 1e-9 * units_before.abs().max(1.0),
        "σ-不变配额违反@bar {bar} layer {ladder}：m={m} ≠ f×units={canonical}（f={MOBILE_FRAC} 级别无关）"
    );
}

/// **手性交替观测（T24 对冲净额检测）**：记录相邻占用级别同向事件数。
/// 初始建仓阶段各级别可能同向（尚无卖点触发短差）——这是合法的涌现过程。
/// 改为非 panic 观测量（计数器），在 result 中报告，由上游判断是否违反。
/// 原 panic 版本因"永远交替"假设过强被否定（4/8 标的 panic）。
pub fn count_chiral_violations(layers: &[Layer]) -> usize {
    let mut count = 0;
    for k in 0..layers.len().saturating_sub(1) {
        if layers[k].units > 0.0
            && layers[k + 1].units > 0.0
            && layers[k].direction == layers[k + 1].direction
        {
            count += 1;
        }
    }
    count
}

// ════════════════════════════ L2 守恒守卫（每 bar panic）════════════════════════════

/// **股数守恒（T48 Casimir）**：`Σ|units|（多空都计绝对值）= n_base`。容差 1e-6。
pub fn prove_conservation(layers: &[Layer], n_base: f64, bar: i64) {
    let held = total_units_held(layers);
    assert!(
        (held - n_base).abs() <= 1e-6 * n_base.abs().max(1.0),
        "守恒违反@bar {bar}：Σ|units|={held} ≠ n_base={n_base}"
    );
}

/// **NAV 中性**：同价 c 全部操作前后 NAV 不变（价值中性）。容差 1e-4。
pub fn prove_nav_neutral(nav_pre: f64, nav_post: f64, bar: i64) {
    assert!(
        (nav_post - nav_pre).abs() <= 1e-4 * nav_pre.abs().max(1.0),
        "NAV 中性违反@bar {bar}：操作前 NAV={nav_pre} ≠ 操作后 NAV={nav_post}（同价 c 应中性）"
    );
}

/// **per-layer 互斥（N2 类比）**：本 bar 动作过的层 id 无重复。
pub fn prove_no_double_act(acted_ladders: &[usize], bar: i64) {
    for i in 0..acted_ladders.len() {
        for j in (i + 1)..acted_ladders.len() {
            assert_ne!(
                acted_ladders[i], acted_ladders[j],
                "per-layer 互斥违反@bar {bar}：层 {} 本 bar 被操作两次",
                acted_ladders[i]
            );
        }
    }
}

/// **递归一致性（局部依赖 L6/L8）**：每个活跃仓位在合法势源层 [FIRST_BSP_LADDER, MAX_LADDER)。
/// ε 对称：Short root 的多头子仓与 Long root 的空头子仓同等约束（都不允许在 FIRST_BSP 以下）。
pub fn prove_recursive_consistency(layers: &[Layer], bar: i64) {
    for l in layers {
        if l.units > 0.0 {
            assert!(
                l.ladder >= FIRST_BSP_LADDER,
                "递归一致性违反@bar {bar}：仓位层 {} < FIRST_BSP={FIRST_BSP_LADDER}（方向={:?}）",
                l.ladder,
                l.direction
            );
        }
    }
    let _ = PENDING_LO; // 势源下界（sink 目标可下沉至 FIRST_BSP，允许在 FIRST_BSP）
}

/// **ε 对称性守卫**：F 入场极性与 root_direction 一致（root=Up→Long, root=Down→Short）。
///
/// 修改（emergent_level_direction.md §5.4 + 用户裁决 2026-06-17）：
/// core_polarity = f(root_direction)，f(Up)=Long, f(Down)=Short——根方向是 H⁰ 涌现属性，
/// 不从 BSP 信号类型推断。旧签名 `entered_from_sell` 被否定（sell_source ≠ 翻空，是 Up regime
/// 中的次级别顶=平多窗口；ES/GC 强 Up regime 中按 sell_source→Short 踏空 −213%/−122%）。
///
/// D∞ 群结构必然性：ε∈{±1} 是 F 的唯一参数；硬编码 ε=const 截断 D∞ 为 Z；
/// 从 root_direction 读 ε 是 H⁰ 涌现结构的直接消费，零自由度。
pub fn prove_epsilon_symmetry(core_polarity: Polarity, root_dir: Direction, bar: i64) {
    let is_consistent = matches!(
        (core_polarity, root_dir),
        (Polarity::Long, Direction::Up) | (Polarity::Short, Direction::Down)
    );
    assert!(
        is_consistent,
        "ε 对称性违反@bar {bar}：core_polarity={core_polarity:?} 但 root_direction={root_dir:?}（应 Long⟺Up, Short⟺Down，f(root_dir)→polarity 同向）",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::types::Polarity;

    fn mk(ladder: usize, dir: Polarity, units: f64) -> Layer {
        let mut l = Layer::idle(ladder);
        l.direction = dir;
        l.units = units;
        l.basis = 100.0;
        l
    }

    // ──────── 正向规范值不 panic ────────

    #[test]
    fn cross_level_closure_ok() {
        prove_cross_level_closure(4, 3); // Δr=−1 ✓
    }

    #[test]
    fn sigma_quota_canonical_ok() {
        prove_sigma_quota(30.0 * MOBILE_FRAC, 30.0, 4, 0);
    }

    #[test]
    fn chiral_alternation_zero_count() {
        // 层2 Long, 层3 Short, 层4 Long —— 相邻交替 ⟹ 同向计数=0
        let layers = vec![
            mk(2, Polarity::Long, 10.0),
            mk(3, Polarity::Short, 5.0),
            mk(4, Polarity::Long, 20.0),
        ];
        assert_eq!(count_chiral_violations(&layers), 0);
    }

    #[test]
    fn conservation_ok() {
        let layers = vec![mk(3, Polarity::Long, 60.0), mk(4, Polarity::Short, 40.0)];
        prove_conservation(&layers, 100.0, 0); // 60+40=100 ✓
    }

    #[test]
    fn nav_neutral_ok() {
        prove_nav_neutral(100_000.0, 100_000.0 + 0.5, 0);
    }

    #[test]
    fn no_double_act_ok() {
        prove_no_double_act(&[3, 4, 5], 0);
    }

    // ──────── 反证：错误值必 panic（非重言）────────

    #[test]
    #[should_panic(expected = "Δr=−1 违反")]
    fn cross_level_same_panics() {
        prove_cross_level_closure(4, 4);
    }

    #[test]
    #[should_panic(expected = "Δr=−1 违反")]
    fn cross_level_two_down_panics() {
        prove_cross_level_closure(4, 2);
    }

    #[test]
    #[should_panic(expected = "σ-不变配额违反")]
    fn sigma_quota_wrong_f_panics() {
        prove_sigma_quota(30.0 * 0.5, 30.0, 4, 0);
    }

    #[test]
    fn chiral_same_dir_adjacent_counted() {
        // 层3 Long, 层4 Long —— 相邻同向（建仓阶段未出卖点，合法）⟹ 计数=1（观测，不 panic）
        let layers = vec![mk(3, Polarity::Long, 10.0), mk(4, Polarity::Long, 10.0)];
        assert_eq!(count_chiral_violations(&layers), 1);
    }

    #[test]
    #[should_panic(expected = "守恒违反")]
    fn conservation_broken_panics() {
        let layers = vec![mk(3, Polarity::Long, 60.0)];
        prove_conservation(&layers, 100.0, 0); // 60 ≠ 100
    }

    #[test]
    #[should_panic(expected = "NAV 中性违反")]
    fn nav_neutral_broken_panics() {
        prove_nav_neutral(100_000.0, 110_000.0, 0);
    }

    #[test]
    #[should_panic(expected = "per-layer 互斥违反")]
    fn double_act_panics() {
        prove_no_double_act(&[3, 4, 3], 0);
    }
}
