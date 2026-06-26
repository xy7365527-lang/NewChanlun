//! 中枢边界构造 + 中枢关系三态 + 点位三态。
//!
//! ## bit-exact 对齐
//!
//! - 中枢窗口构造 ↔ `Formal/RecursiveConstruction.lean` `CenterDerivedAt`：连续三段窗口
//!   `zd=max(三段lo)`、`zg=min(三段hi)`、`dd=min(三段lo)`、`gg=max(三段hi)`。
//! - 中枢关系三态 ↔ `Formal/CenterTrichotomy.lean` `classify`：
//!   `next.dd>prev.gg → up`，`next.gg<prev.dd → down`，否则 `expansion`（外缘判据）。
//! - 点位三态 ↔ `Phase2/Claim9_CenterPosition.lean` `classify`：
//!   `p<zd → below`，`zg<p → above`，否则 `within`（闭核心区间 [zd,zg]）。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 全部 L0：纯整数比较，不依赖经验数据。中枢成立条件 `zd<=zg`（reference-theta-v0.md:23
//! 闭区间设计选择）；点位/关系判据是缠论第17/18/49课 `[缠论可导]`，不进 config。

use super::super::types::{Center, Tick};

/// 走势单元的价格区间投影 `[lo, hi]`（`lo<=hi` 不变量）。
///
/// 对齐 `RecursiveConstruction.Move.interval`：中枢由次级别走势单元的区间窗口生成。
/// 本结构是中枢构造的输入——next 工位提供的次级别走势已规约为 `[lo,hi]` 区间序列。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitRange {
    /// 该走势单元在 L0 原始 K 序的起点（中枢 start_index 取首单元起点）。
    pub start_index: usize,
    /// 该走势单元在 L0 原始 K 序的终点（中枢 end_index 取末单元终点）。
    pub end_index: usize,
    pub lo: Tick,
    pub hi: Tick,
}

/// 中枢关系三态（reference-theta-v0.md:33；`CenterTrichotomy.CenterRelation`）。
///
/// 两个**核心已分离的同级别新生中枢**的关系（外缘 dd/gg 判据，第18/20课中心定理二）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterRelation {
    /// 上涨延续：`next.dd > prev.gg`（外缘完全分离向上）。
    UpContinuation,
    /// 下跌延续：`next.gg < prev.dd`（外缘完全分离向下）。
    DownContinuation,
    /// 级别扩张：外缘重叠（既非向上分离也非向下分离）。
    LevelExpansion,
}

/// 点相对中枢核心区间 `[zd,zg]` 的位置三态（reference-theta-v0.md:36；Claim9）。
///
/// ★边界口径（Claim9）：闭核心区间 `[zd,zg]` 归 within；破 zg（`p>zg`）才 above，
/// 跌破 zd（`p<zd`）才 below。第49课「小于 ZD / 大于 ZG」严格不等式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativePosition {
    /// 之下：`p < zd`。
    Below,
    /// 之中：`zd <= p <= zg`（闭核心区间）。
    Within,
    /// 之上：`p > zg`。
    Above,
}

/// 从连续三段次级别走势单元窗口构造中枢（reference-theta-v0.md:23；`CenterDerivedAt`）。
///
/// `zd=max(三段lo)`、`zg=min(三段hi)`、`dd=min(三段lo)`、`gg=max(三段hi)`。
///
/// 边界条件（reference-theta-v0.md:23）：`zd<=zg` 成立才是中枢——三段区间有公共重叠。
/// `zd>zg`（无公共重叠）⟹ 返回 `None`（非中枢，自然终止于此，不静默造一个退化中枢）。
pub fn center_from_window(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Option<Center> {
    // bit-exact：嵌套 max/min 的约简顺序与 Lean `max x (max y z)` 一致。
    let zd = a.lo.max(b.lo.max(c.lo));
    let zg = a.hi.min(b.hi.min(c.hi));
    if zd > zg {
        // 无公共重叠 ⟹ 非中枢（reference-theta-v0.md:23 `ZD<=ZG` 成立条件不满足）。
        return None;
    }
    let dd = a.lo.min(b.lo.min(c.lo));
    let gg = a.hi.max(b.hi.max(c.hi));
    Some(Center {
        zd,
        zg,
        dd,
        gg,
        start_index: a.start_index,
        end_index: c.end_index,
    })
}

/// 判定两中枢关系（reference-theta-v0.md:33；`CenterTrichotomy.classify`，外缘判据）。
///
/// `next.dd>prev.gg → up`，`next.gg<prev.dd → down`，否则 `expansion`。逐字对齐 Lean。
pub fn classify_relation(prev: &Center, next: &Center) -> CenterRelation {
    if next.dd > prev.gg {
        CenterRelation::UpContinuation
    } else if next.gg < prev.dd {
        CenterRelation::DownContinuation
    } else {
        CenterRelation::LevelExpansion
    }
}

/// 判定点位三态（reference-theta-v0.md:36；`CenterPosition.classify`，闭核心区间）。
///
/// `p<zd → below`，`zg<p → above`，否则 `within`。逐字对齐 Lean。
pub fn classify_position(c: &Center, p: Tick) -> RelativePosition {
    if p < c.zd {
        RelativePosition::Below
    } else if c.zg < p {
        RelativePosition::Above
    } else {
        RelativePosition::Within
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(si: usize, ei: usize, lo: Tick, hi: Tick) -> UnitRange {
        UnitRange { start_index: si, end_index: ei, lo, hi }
    }

    #[test]
    fn center_window_bit_exact_recursive_lean() {
        // 三段区间 [0,10],[3,12],[5,15]：zd=max(0,3,5)=5, zg=min(10,12,15)=10,
        // dd=min=0, gg=max=15（CenterDerivedAt 核心+外缘 sound）。
        let a = unit(0, 4, 0, 10);
        let b = unit(4, 8, 3, 12);
        let c = unit(8, 12, 5, 15);
        let center = center_from_window(&a, &b, &c).expect("zd<=zg ⟹ 中枢成立");
        assert_eq!((center.zd, center.zg, center.dd, center.gg), (5, 10, 0, 15));
        assert_eq!((center.start_index, center.end_index), (0, 12));
    }

    #[test]
    fn center_window_no_overlap_yields_none() {
        // 三段无公共重叠（[0,4],[10,14],[20,24]）：zd=20 > zg=4 ⟹ 非中枢。
        let a = unit(0, 4, 0, 4);
        let b = unit(4, 8, 10, 14);
        let c = unit(8, 12, 20, 24);
        assert_eq!(center_from_window(&a, &b, &c), None);
    }

    #[test]
    fn center_window_boundary_zd_eq_zg_is_center() {
        // 退化但合法：zd==zg（[0,5],[5,10],[3,5]）：zd=max(0,5,3)=5, zg=min(5,10,5)=5。
        let a = unit(0, 4, 0, 5);
        let b = unit(4, 8, 5, 10);
        let c = unit(8, 12, 3, 5);
        let center = center_from_window(&a, &b, &c).expect("zd==zg ⟹ 闭区间中枢成立");
        assert_eq!((center.zd, center.zg), (5, 5));
    }

    fn center(dd: Tick, zd: Tick, zg: Tick, gg: Tick) -> Center {
        Center { zd, zg, dd, gg, start_index: 0, end_index: 0 }
    }

    #[test]
    fn relation_up_continuation_bit_exact() {
        // prev 外缘[0,8]，next 外缘[9,15]：next.dd=9 > prev.gg=8 ⟹ up（chain2_up 同构）。
        let prev = center(0, 2, 5, 8);
        let next = center(9, 10, 13, 15);
        assert_eq!(classify_relation(&prev, &next), CenterRelation::UpContinuation);
    }

    #[test]
    fn relation_down_continuation_bit_exact() {
        // prev 外缘[9,15]，next 外缘[0,8]：next.gg=8 < prev.dd=9 ⟹ down。
        let prev = center(9, 10, 13, 15);
        let next = center(0, 2, 5, 8);
        assert_eq!(classify_relation(&prev, &next), CenterRelation::DownContinuation);
    }

    #[test]
    fn relation_level_expansion_bit_exact() {
        // codex 见证 prev=(0,2,5,8), next=(4,6,9,11)：核心分离 + 外缘重叠 ⟹ expansion。
        let prev = center(0, 2, 5, 8);
        let next = center(4, 6, 9, 11);
        assert_eq!(classify_relation(&prev, &next), CenterRelation::LevelExpansion);
    }

    #[test]
    fn position_three_states_bit_exact_claim9() {
        // 核心 [0,2]：p=-1 below；p=1 within；p=5 above（Claim9 realized 见证）。
        let c = center(-1, 0, 2, 3);
        assert_eq!(classify_position(&c, -1), RelativePosition::Below);
        assert_eq!(classify_position(&c, 1), RelativePosition::Within);
        assert_eq!(classify_position(&c, 5), RelativePosition::Above);
    }

    #[test]
    fn position_boundary_closed_core_within() {
        // 边界点 p=zd、p=zg 归 within（闭核心区间，Claim9 严格不等式判据）。
        let c = center(-1, 0, 2, 3);
        assert_eq!(classify_position(&c, 0), RelativePosition::Within); // p=zd
        assert_eq!(classify_position(&c, 2), RelativePosition::Within); // p=zg
    }

    /// property：点位三态对任意点穷尽（below/within/above 恰一，Claim9 position_exactly_one）。
    #[test]
    fn property_position_total_exclusive() {
        let c = center(-5, 0, 10, 15);
        for p in -20..=30 {
            let r = classify_position(&c, p);
            let below = p < c.zd;
            let above = c.zg < p;
            let within = c.zd <= p && p <= c.zg;
            // 恰好一态成立（穷尽 + 互斥）。
            let count = [below, within, above].iter().filter(|&&b| b).count();
            assert_eq!(count, 1, "p={p} 必恰好落一态");
            match r {
                RelativePosition::Below => assert!(below),
                RelativePosition::Within => assert!(within),
                RelativePosition::Above => assert!(above),
            }
        }
    }
}
