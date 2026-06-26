//! 递归级别构造 + 走势裁决（reference-theta-v0.md:29-30）。
//!
//! ## 契约重锚（legacy Formal/RecursiveConstruction + CenterTrichotomy →
//! `Origin.TrendCompleteClassification` + `Origin.RecursiveLevelSystem`）
//!
//! - 走势裁决 ↔ `Origin.TrendCompleteClassification.{TrendClass,chooseTrend}`：0 中枢 →
//!   `HigherCenterCandidate`（unfinished）；1 中枢 → `Consolidation`；≥2 中枢全链上涨延续 →
//!   `Trend(Up)`（trendUp）；全链下跌延续 → `Trend(Down)`（trendDown）；非全链一致 →
//!   `HigherCenterCandidate`（本级终结，交父级）。`chooseTrend complete hasTwoCenters up` 逐分支对齐。
//! - 全链同向 ↔ `allAdjacent`：所有相邻中枢的 `classify_relation`（`Origin.CenterStates` 外缘判据）
//!   都等于目标关系。
//! - `MoveOutcome → MoveKind` 桥接 ↔ `Origin.ChanlunElements.MoveKind`（consolidation/trendUp/trendDown）：
//!   trend up/down → Trend；consolidation → Consolidation；higherCenterCandidate → none（裁决退化）。
//!
//! ## 递归级别（reference-theta-v0.md:29-30）
//!
//! `L0=1分钟线段账本`；`L(k+1)` 只由 `Lk` 已完成走势/Move 构造；禁跳级混级。
//! 某层无 `>=config.level.min_parts_per_level` 完成部件则自然终止；上界 `config.level.l_max`。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 全部 L0：走势裁决是 `Origin.TrendCompleteClassification.chooseTrend` 的纯函数镜像，不依赖经验数据。级别终止判据是
//! reference-theta-v0.md:30 `[设计选择]`（min_parts_per_level/l_max 进 config）。

use super::super::types::{Direction, MoveKind};
use super::center::{classify_relation, CenterRelation};
use super::super::types::Center;

/// 走势裁决结果（契约锚 `Origin.TrendCompleteClassification.TrendClass`）。
///
/// 比 `types::MoveKind` 严格更细——含方向 + `HigherCenterCandidate`（裁决退化态：
/// 0 中枢 / 级别扩张 / 中枢非依次同向 = 本级终结，交父级重分类）。`MoveKind` 是
/// {趋势,盘整} 粗投影（见 `outcome_to_kind`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveOutcome {
    /// 趋势（≥2 同向中枢全链一致；携带方向）。
    Trend(Direction),
    /// 盘整（恰好 1 中枢）。
    Consolidation,
    /// 裁决退化（0 中枢 / 扩张 / 方向混合）——本级终结，交父级重分类。
    HigherCenterCandidate,
}

/// 全链同向判定（契约锚 `Origin.CenterStates` 外缘趋势判据的全链推广）。
///
/// 中枢序列中**每对相邻中枢**的 `classify_relation` 都等于 `rel`。趋势要求全链同向
/// （非仅首两个）——`[up,up,expansion]` 或 `[up,down]` 不是趋势（mixed → 交父级）。
fn all_adjacent(rel: CenterRelation, centers: &[Center]) -> bool {
    // 空链 / 单中枢：无相邻对，平凡 true（空 windows 的 all 平凡真，与全链同向定义一致）。
    centers
        .windows(2)
        .all(|w| classify_relation(&w[0], &w[1]) == rel)
}

/// 走势裁决（契约锚 `Origin.TrendCompleteClassification.chooseTrend`，全链一致）。
///
/// 逐分支对齐 `Origin.TrendCompleteClassification.chooseTrend complete hasTwoCenters up`：
/// - 0 中枢 → `HigherCenterCandidate`（**非盘整**——完成走势必含 ≥1 中枢，0 中枢归退化）。
/// - 1 中枢 → `Consolidation`（盘整定义）。
/// - ≥2 中枢全链上涨延续 → `Trend(Up)`；全链下跌延续 → `Trend(Down)`。
/// - ≥2 中枢非全链一致（含扩张/方向混合）→ `HigherCenterCandidate`（本级终结，交父级）。
pub fn classify_move(centers: &[Center]) -> MoveOutcome {
    match centers.len() {
        0 => MoveOutcome::HigherCenterCandidate,
        1 => MoveOutcome::Consolidation,
        _ => {
            if all_adjacent(CenterRelation::UpContinuation, centers) {
                MoveOutcome::Trend(Direction::Up)
            } else if all_adjacent(CenterRelation::DownContinuation, centers) {
                MoveOutcome::Trend(Direction::Down)
            } else {
                MoveOutcome::HigherCenterCandidate
            }
        }
    }
}

/// `MoveOutcome → Option<MoveKind>` 桥接（契约锚 `Origin.ChanlunElements.MoveKind`）。
///
/// trend up/down/consolidation → `Some(MoveKind)`；`HigherCenterCandidate` → `None`
/// （裁决退化不落 {趋势,盘整}——其分类归状态层，不是已完成走势结果）。
///
/// 注：`MoveKind` 只区分 {Trend, Consolidation}，不携带方向（zoushi 第31课盘整无方向，
/// 趋势方向由中枢序列派生而非自由携带）。方向信息保留在 `MoveOutcome`，需要时另取。
pub fn outcome_to_kind(outcome: MoveOutcome) -> Option<MoveKind> {
    match outcome {
        MoveOutcome::Trend(_) => Some(MoveKind::Trend),
        MoveOutcome::Consolidation => Some(MoveKind::Consolidation),
        MoveOutcome::HigherCenterCandidate => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn center(dd: i64, zd: i64, zg: i64, gg: i64) -> Center {
        Center { zd, zg, dd, gg, start_index: 0, end_index: 0 }
    }

    /// 真链中枢（对齐 Origin 外缘依次上移见证 c0/c1/c2：[0,3]/[4,7]/[8,11]）。
    fn c0() -> Center { center(0, 1, 2, 3) }
    fn c1() -> Center { center(4, 5, 6, 7) }
    fn c2() -> Center { center(8, 9, 10, 11) }

    #[test]
    fn zero_centers_higher_candidate_bit_exact() {
        // classifyMove [] = higherCenterCandidate（0 中枢非盘整，zero_centers_not_consolidation）。
        assert_eq!(classify_move(&[]), MoveOutcome::HigherCenterCandidate);
    }

    #[test]
    fn one_center_consolidation_bit_exact() {
        // classifyMove [c0] = consolidation（one_center_is_consolidation）。
        assert_eq!(classify_move(&[c0()]), MoveOutcome::Consolidation);
    }

    #[test]
    fn two_chain_up_trend_bit_exact() {
        // classifyMove [c0,c1] = trend up（chain2_up）。
        assert_eq!(classify_move(&[c0(), c1()]), MoveOutcome::Trend(Direction::Up));
    }

    #[test]
    fn three_chain_up_trend_bit_exact() {
        // classifyMove [c0,c1,c2] = trend up（chain3_up，全链 upContinuation）。
        assert_eq!(
            classify_move(&[c0(), c1(), c2()]),
            MoveOutcome::Trend(Direction::Up)
        );
    }

    #[test]
    fn two_chain_down_trend_bit_exact() {
        // classifyMove [c2,c0] = trend down（chain2_down，外缘依次下移）。
        assert_eq!(classify_move(&[c2(), c0()]), MoveOutcome::Trend(Direction::Down));
    }

    #[test]
    fn mixed_chain_higher_candidate_bit_exact() {
        // [c0,c1] 是 up，[c1,c0] 是 down ⟹ [c0,c1,c0] 方向混合 → higherCenterCandidate
        // （mixed_centers_is_higher_candidate：非全链一致交父级）。
        assert_eq!(
            classify_move(&[c0(), c1(), c0()]),
            MoveOutcome::HigherCenterCandidate
        );
    }

    #[test]
    fn expansion_in_chain_higher_candidate() {
        // 含级别扩张对 ⟹ 既非全链 up 也非全链 down → higherCenterCandidate。
        // prev=(0,2,5,8), exp=(4,6,9,11) 是 expansion。
        let exp = center(4, 6, 9, 11);
        let prev = center(0, 2, 5, 8);
        assert_eq!(
            classify_move(&[prev, exp]),
            MoveOutcome::HigherCenterCandidate
        );
    }

    #[test]
    fn outcome_to_kind_bridge_bit_exact() {
        // outcomeToKind：trend → Trend；consolidation → Consolidation；higher → None。
        assert_eq!(outcome_to_kind(MoveOutcome::Trend(Direction::Up)), Some(MoveKind::Trend));
        assert_eq!(outcome_to_kind(MoveOutcome::Trend(Direction::Down)), Some(MoveKind::Trend));
        assert_eq!(outcome_to_kind(MoveOutcome::Consolidation), Some(MoveKind::Consolidation));
        assert_eq!(outcome_to_kind(MoveOutcome::HigherCenterCandidate), None);
    }

    /// property：走势裁决对任意中枢序列穷尽落四态之一（outcome_total）。
    #[test]
    fn property_outcome_total() {
        let chains: Vec<Vec<Center>> = vec![
            vec![],
            vec![c0()],
            vec![c0(), c1()],
            vec![c0(), c1(), c2()],
            vec![c2(), c1(), c0()],
            vec![c0(), c1(), c0()],
        ];
        for ch in chains {
            let o = classify_move(&ch);
            assert!(matches!(
                o,
                MoveOutcome::Trend(Direction::Up)
                    | MoveOutcome::Trend(Direction::Down)
                    | MoveOutcome::Consolidation
                    | MoveOutcome::HigherCenterCandidate
            ));
        }
    }
}
