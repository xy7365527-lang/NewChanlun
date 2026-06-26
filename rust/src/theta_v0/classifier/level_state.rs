//! R6 位置态 + BSP bit-vector + 每级 LevelState（reference-theta-v0.md:32-36）。
//!
//! ## 契约重锚（legacy Strict/LevelState → `Origin.CenterStates` + `Origin.RecursiveLevelSystem`）
//!
//! - R6 态 ↔ `Origin.CenterStates.CenterPosition` 三态 × 第三类事件精化：
//!   `{bot, inside, aboveNo3B, aboveB3, belowNo3S, belowS3}`（互斥 sum）。
//! - R6 判定 ↔ `rlevelOf`：无中枢 → bot；有中枢按 `Origin.CenterStates.classifyPosition` 三态分流，
//!   above 按 b3 裂 aboveB3/aboveNo3B，below 按 s3 裂 belowS3/belowNo3S，within → inside。
//! - BSP bit-vector ↔ `Origin.BspClassification.BspClass`（{0,1}⁶ 非互斥，2/3 类可共存）= `types::BspBits`。
//! - LevelState 三元组 (D_ℓ, R_ℓ, E_ℓ)：decomp/position/signal（递归级别态，锚 `Origin.RecursiveLevelSystem`）。
//!
//! ## 两分类代数性质相反（核心诚实点）
//!
//! - **R 位置态**：partition（互斥穷尽 Σ𝟙=1）——sum type `RLevel`。
//! - **E 信号**：subset（2/3 类可共存）——bit-vector `BspBits`。把 E 做成互斥 sum 是错误。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 全部 L0（给定 Θ 后）：`rlevelOf` 是给定 `RContext`（含 Θ_signal 的 b3/s3 事件）后的
//! 全函数。R6 态**不**由缠论结构公理单独导出——依赖 Θ_parse(最后中枢) + Θ_signal(3B/3S
//! 判据)。本工位实装「给定 RContext 后的全函数」（L0），不声称 6 态是无参数完全分类。

use super::super::types::{BspBits, Center};
use super::super::types::Tick;
use super::center::{classify_position, RelativePosition};

/// R6 位置态（契约锚 `Origin.CenterStates.CenterPosition` 精化，互斥 sum type）。
///
/// 相对最后中枢 Z_ℓ 的位置状态机精化（`Origin.CenterStates.classifyPosition` 三态 × 第三类事件
/// + 无中枢）：within → Inside（不拆）；above 按 3B 裂 2 态；below 按 3S 裂 2 态；+ Bot。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RLevel {
    /// ⊥：无最后中枢（ExistsZ = false）。
    Bot,
    /// I：当下在 Z_ℓ 核心区间 [zd,zg] 内（within）。
    Inside,
    /// U⁰：Z_ℓ 上方（p>zg）且无第三类买点。
    AboveNo3B,
    /// U¹：Z_ℓ 上方且有第三类买点（3B 成立）。
    AboveB3,
    /// D⁰：Z_ℓ 下方（p<zd）且无第三类卖点。
    BelowNo3S,
    /// D¹：Z_ℓ 下方且有第三类卖点（3S 成立）。
    BelowS3,
}

/// R6 态运行上下文（reference-theta-v0.md:32；`LevelState.RContext`，Θ-参数化运行输入）。
///
/// - `last_center`：最后中枢 Z_ℓ（`None` = 无中枢 → Bot）。哪个是「最后」由 Θ_parse 选取。
/// - `price`：当下价格点 p（相对 Z_ℓ 核心区间定位）。
/// - `b3`：第三类买点事件是否成立（Θ_signal 判据：离开中枢上破后回试不入）。
/// - `s3`：第三类卖点事件是否成立（Θ_signal 判据）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RContext {
    pub last_center: Option<Center>,
    pub price: Tick,
    pub b3: bool,
    pub s3: bool,
}

/// R6 态判定（契约锚 `Origin.CenterStates.classifyPosition` 精化，给定 Θ 后全函数）。
///
/// 逐分支对齐 `Origin.CenterStates.classifyPosition` 三态 × 第三类事件精化：
/// - 无中枢 → Bot；
/// - 有中枢 c 按 `classify_position(c, p)`（= `Origin.classifyPosition`）三态分流：
///   within → Inside；above → b3 ? AboveB3 : AboveNo3B；below → s3 ? BelowS3 : BelowNo3S。
pub fn rlevel_of(ctx: &RContext) -> RLevel {
    match ctx.last_center {
        None => RLevel::Bot,
        Some(c) => match classify_position(&c, ctx.price) {
            RelativePosition::Within => RLevel::Inside,
            RelativePosition::Above => {
                if ctx.b3 {
                    RLevel::AboveB3
                } else {
                    RLevel::AboveNo3B
                }
            }
            RelativePosition::Below => {
                if ctx.s3 {
                    RLevel::BelowS3
                } else {
                    RLevel::BelowNo3S
                }
            }
        },
    }
}

/// 单级别完整状态（reference-theta-v0.md:32；`Strict.LevelState.LevelState`）。
///
/// 三分量代数性质不同：`position`（R_ℓ）是 partition（`RLevel` 互斥）；`signal`（E_ℓ）
/// 是 subset（`BspBits` 非互斥，2/3 类可共存）。`decomp`（D_ℓ）由 classifier 顶层填充
/// （moves/centers），本结构承载位置态 + 信号 bit-vector。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointState {
    /// 该候选点在 L0 原始 K 序的位置（平局裁决 + 回溯定位）。
    pub source_index: usize,
    /// R_ℓ 位置态（相对最后中枢，互斥穷尽）。
    pub position: RLevel,
    /// E_ℓ 买卖点信号 bit-vector（非互斥 subset）。
    pub signal: BspBits,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn center(dd: Tick, zd: Tick, zg: Tick, gg: Tick) -> Center {
        Center { zd, zg, dd, gg, start_index: 0, end_index: 0 }
    }

    #[test]
    fn rlevel_bot_when_no_center_bit_exact() {
        // 无中枢 → Bot（rlevelOf_eq_bot）。
        let ctx = RContext { last_center: None, price: 100, b3: true, s3: true };
        assert_eq!(rlevel_of(&ctx), RLevel::Bot);
    }

    #[test]
    fn rlevel_inside_when_within_bit_exact() {
        // 核心 [0,10]，p=5 within → Inside（不按第三类拆分）。
        let c = center(-5, 0, 10, 15);
        let ctx = RContext { last_center: Some(c), price: 5, b3: true, s3: true };
        assert_eq!(rlevel_of(&ctx), RLevel::Inside);
    }

    #[test]
    fn rlevel_above_splits_by_b3_bit_exact() {
        // 核心 [0,10]，p=20 above；b3 裂 AboveB3 / AboveNo3B。
        let c = center(-5, 0, 10, 15);
        let with_b3 = RContext { last_center: Some(c), price: 20, b3: true, s3: false };
        assert_eq!(rlevel_of(&with_b3), RLevel::AboveB3);
        let no_b3 = RContext { last_center: Some(c), price: 20, b3: false, s3: false };
        assert_eq!(rlevel_of(&no_b3), RLevel::AboveNo3B);
    }

    #[test]
    fn rlevel_below_splits_by_s3_bit_exact() {
        // 核心 [0,10]，p=-5 below；s3 裂 BelowS3 / BelowNo3S。
        let c = center(-10, 0, 10, 15);
        let with_s3 = RContext { last_center: Some(c), price: -5, b3: false, s3: true };
        assert_eq!(rlevel_of(&with_s3), RLevel::BelowS3);
        let no_s3 = RContext { last_center: Some(c), price: -5, b3: false, s3: false };
        assert_eq!(rlevel_of(&no_s3), RLevel::BelowNo3S);
    }

    #[test]
    fn rlevel_within_ignores_b3_s3() {
        // within 不按第三类拆分——b3/s3 任意值都 Inside（逐态精化，非全三态乘 Bool）。
        let c = center(-5, 0, 10, 15);
        for b3 in [true, false] {
            for s3 in [true, false] {
                let ctx = RContext { last_center: Some(c), price: 5, b3, s3 };
                assert_eq!(rlevel_of(&ctx), RLevel::Inside);
            }
        }
    }

    #[test]
    fn rlevel_boundary_zg_is_inside() {
        // p=zg 归 within → Inside（闭核心区间，破 zg 才 above）。
        let c = center(-5, 0, 10, 15);
        let ctx = RContext { last_center: Some(c), price: 10, b3: true, s3: true };
        assert_eq!(rlevel_of(&ctx), RLevel::Inside);
    }

    /// property：R6 态对任意 RContext 穷尽落 6 态之一（rlevel_constructor_exhaustive）。
    #[test]
    fn property_rlevel_total() {
        let c = center(-5, 0, 10, 15);
        let centers = [None, Some(c)];
        for lc in centers {
            for p in [-20, -5, 0, 5, 10, 20] {
                for b3 in [true, false] {
                    for s3 in [true, false] {
                        let ctx = RContext { last_center: lc, price: p, b3, s3 };
                        let r = rlevel_of(&ctx);
                        assert!(matches!(
                            r,
                            RLevel::Bot
                                | RLevel::Inside
                                | RLevel::AboveNo3B
                                | RLevel::AboveB3
                                | RLevel::BelowNo3S
                                | RLevel::BelowS3
                        ));
                    }
                }
            }
        }
    }
}
