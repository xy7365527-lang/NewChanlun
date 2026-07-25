//! # 区间套必要条件——递归塔原生实现（条款 9，任务 #106）
//!
//! ## 公理与出处
//! cert-bsp-binding-ruling-DRAFT-20260717.md 条款 9：区间套必要条件应在**递归塔内部
//! 原生实现**（次级别构件、塔内时钟）；nest 管线保持独立对照实现身份，其产物禁止
//! 回灌判据 crate（tests/nest_isolation_guard.rs 公理级守卫）。本模块只消费塔构件
//! （`LevelState.centers` / `LevelState.bsp`），不触任何 nest 产物。
//!
//! ## 判据形状（p108-interval-probe-20260717.md 裁定建议 1–3）
//! P108 实证：同 bar 同侧硬判据 70–92% 缺失（误杀），离开窗口嵌套 100% 成立。故：
//! - **窗口式**：lvl k（k≥1）点的必要条件 = 存在 lvl k-1 **同侧**点，其
//!   `source_index` 落在本级**离开窗口** `[min(center.end, src), max(center.end, src)]`
//!   内（闭区间；center = 本级中枢全量序列中该点前最近一个中枢）。
//! - **中枢来源**：`LevelState.centers`（每级全量序列），不用 `BspPoint.center` 字段
//!   （1/2 类可为 `None`，nocenter 空洞会假性不可判）。
//! - **不要求** 点 bar = 窗口极值 bar / 同 bar 同点（P108 检验 1/3：exact0 仅 3.86%，
//!   收敛只按单调性成立）。
//!
//! ## 语义边界（诚实声明）
//! - 这是**必要条件的检查器**，不是过滤器：verdict 不回写 `BspBits`、不改任何判据
//!   bit（#102 归因未定前，级别归属结论不得用于方向不对称策略——开放条款）。
//!   过滤/硬门须另经主人裁定。
//! - lvl0 无更低级别，vacuous（不产 verdict）。
//! - `window=None`（该点前本级无任何中枢）= 不可判（undecidable），单列不计入缺失。
//!
//! ## 认识论等级
//! 纯结构检查（塔内对象间关系），不依赖经验数据（formalization-validity-domain）。

use super::super::types::Side;
use super::Classification;

/// 单点必要条件 verdict（lvl≥1 的每个 (买卖点, 侧) 一行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntervalNecessity {
    /// 级别 k（≥1；lvl0 vacuous 不产行）。
    pub level: u32,
    /// 该点在 L0 原始 K 序的位置（`BspPoint.source_index`，与 `Center.*_index` 同基）。
    pub source_index: usize,
    /// 检查的侧（买 bit ⟹ `Long`，卖 bit ⟹ `Short`；双侧 bit 共存则产两行）。
    pub side: Side,
    /// 离开窗口 `[min(center.end, src), max(center.end, src)]`；`None` = 该点前本级
    /// 无中枢（不可判）。
    pub window: Option<(usize, usize)>,
    /// 命中的 lvl k-1 同侧见证点 `source_index`（窗口内最近一个；`None` = 无见证）。
    pub witness: Option<usize>,
    /// 必要条件满足：`window` 可判 ∧ 存在同侧见证。
    pub satisfied: bool,
}

/// 每级汇总（报表/回归断言用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IntervalNecessitySummary {
    pub level: u32,
    /// verdict 行数（= (点,侧) 对数，非点数）。
    pub total: usize,
    pub satisfied: usize,
    /// 可判但无见证（真缺失）。
    pub missing: usize,
    /// `window=None`（该点前本级无中枢）。
    pub undecidable: usize,
}

/// 区间套必要条件——逐级检查（塔内原生，条款 9）。
///
/// 返回 `out[k]` = 第 k 级 verdict 序列（`out[0]` 恒空，lvl0 vacuous）。
/// 输入全量/增量分类输出均适用（增量塔 bit-exact 于全量）。
pub fn interval_necessity_tower(classification: &Classification) -> Vec<Vec<IntervalNecessity>> {
    let levels = &classification.levels;
    let mut out: Vec<Vec<IntervalNecessity>> = Vec::with_capacity(levels.len());
    for (lvl, ls) in levels.iter().enumerate() {
        if lvl == 0 {
            out.push(Vec::new());
            continue;
        }
        let lower = &levels[lvl - 1];
        // lvl k-1 同侧点 source_index 升序索引（classify_impl 已按 source_index 排序，
        // 此处不信任隐式契约，自行收集后排序——幂等）。
        let mut lower_buy: Vec<usize> = Vec::new();
        let mut lower_sell: Vec<usize> = Vec::new();
        for q in lower.bsp.iter() {
            if q.bits.conf_plus() {
                lower_buy.push(q.source_index);
            }
            if q.bits.conf_minus() {
                lower_sell.push(q.source_index);
            }
        }
        lower_buy.sort_unstable();
        lower_sell.sort_unstable();

        let mut rows: Vec<IntervalNecessity> = Vec::new();
        for p in ls.bsp.iter() {
            let src = p.source_index;
            // 本级中枢全量序列中该点前最近一个中枢（start_index <= src 的最后一个）。
            // centers 按构造序 start_index 单调 ⟹ partition_point 可用。
            let centers = &ls.centers;
            let idx = centers.partition_point(|c| c.start_index <= src);
            let window = idx.checked_sub(1).map(|i| {
                let ce = centers[i].end_index;
                (ce.min(src), ce.max(src))
            });
            for (has, side, pool) in [
                (p.bits.conf_plus(), Side::Long, &lower_buy),
                (p.bits.conf_minus(), Side::Short, &lower_sell),
            ] {
                if !has {
                    continue;
                }
                let witness = window.and_then(|(lo, hi)| {
                    // 窗口内最近见证：<= hi 的最大同侧 src，若 >= lo 则命中。
                    let j = pool.partition_point(|&s| s <= hi);
                    j.checked_sub(1).map(|jj| pool[jj]).filter(|&s| s >= lo)
                });
                rows.push(IntervalNecessity {
                    level: lvl as u32,
                    source_index: src,
                    side,
                    window,
                    witness,
                    satisfied: witness.is_some(),
                });
            }
        }
        out.push(rows);
    }
    out
}

/// 逐级汇总（`out[0]` 对应 lvl0，恒零行）。
pub fn summarize(verdicts: &[Vec<IntervalNecessity>]) -> Vec<IntervalNecessitySummary> {
    verdicts
        .iter()
        .enumerate()
        .map(|(lvl, rows)| {
            let mut s = IntervalNecessitySummary { level: lvl as u32, ..Default::default() };
            for r in rows {
                s.total += 1;
                if r.satisfied {
                    s.satisfied += 1;
                } else if r.window.is_none() {
                    s.undecidable += 1;
                } else {
                    s.missing += 1;
                }
            }
            s
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::super::types::{BspBits, Center, Side};
    use super::super::bsp::BspPoint;
    use super::super::{Classification, LevelState};
    use super::*;
    use std::rc::Rc;

    fn pt(src: usize, buy: bool) -> BspPoint {
        BspPoint {
            source_index: src,
            level_origin: 0, // 三方合并 schema 适配（#110 级别身份）
            bits: BspBits {
                buy1: buy,
                sell1: !buy,
                ..Default::default()
            },
            pivot_low: 0,
            pivot_high: 0,
            center: None,
            struct_break_dir: None,
            force: None,
        }
    }

    fn center(start: usize, end: usize) -> Center {
        Center { zd: 0, zg: 1, dd: 0, gg: 1, start_index: start, end_index: end }
    }

    fn level(centers: Vec<Center>, bsp: Vec<BspPoint>) -> LevelState {
        LevelState {
            centers: Rc::new(centers),
            bsp: Rc::new(bsp),
            ..Default::default()
        }
    }

    #[test]
    fn windowed_witness_satisfied_and_missing() {
        // lvl0：买点 src=10、卖点 src=40；lvl1：中枢 [2,8]，买点 src=12（窗口 [8,12] 含 10 ⟹ 满足）、
        // 买点 src=30（窗口 [8,30] 含 10 ⟹ 满足）、卖点 src=9（窗口 [8,9] 无卖见证 ⟹ 缺失）。
        let c = Classification {
            levels: vec![
                level(vec![], vec![pt(10, true), pt(40, false)]),
                level(vec![center(2, 8)], vec![pt(12, true), pt(30, true), pt(9, false)]),
            ],
        };
        let v = interval_necessity_tower(&c);
        assert!(v[0].is_empty(), "lvl0 vacuous");
        assert_eq!(v[1].len(), 3);
        assert_eq!(
            (v[1][0].satisfied, v[1][0].witness, v[1][0].window),
            (true, Some(10), Some((8, 12)))
        );
        assert_eq!((v[1][1].satisfied, v[1][1].witness), (true, Some(10)));
        let miss = &v[1][2];
        assert_eq!((miss.side, miss.satisfied, miss.witness), (Side::Short, false, None));
        let s = summarize(&v);
        assert_eq!((s[1].total, s[1].satisfied, s[1].missing, s[1].undecidable), (3, 2, 1, 0));
    }

    #[test]
    fn side_isolation_no_cross_witness() {
        // 窗口内只有异侧点 ⟹ 不得见证（同侧纪律）。
        let c = Classification {
            levels: vec![
                level(vec![], vec![pt(10, false)]), // lvl0 只有卖点
                level(vec![center(2, 8)], vec![pt(12, true)]), // lvl1 买点，窗口 [8,12]
            ],
        };
        let v = interval_necessity_tower(&c);
        assert_eq!((v[1][0].satisfied, v[1][0].witness), (false, None));
    }

    #[test]
    fn no_center_is_undecidable() {
        // 该点前本级无中枢 ⟹ window=None ⟹ 不可判（不计缺失）。
        let c = Classification {
            levels: vec![
                level(vec![], vec![pt(10, true)]),
                level(vec![center(20, 30)], vec![pt(12, true)]), // 中枢在点后 ⟹ 前无中枢
            ],
        };
        let v = interval_necessity_tower(&c);
        let r = &v[1][0];
        assert_eq!((r.window, r.satisfied), (None, false));
        let s = summarize(&v);
        assert_eq!((s[1].missing, s[1].undecidable), (0, 1));
    }

    #[test]
    fn dual_bit_point_yields_two_rows() {
        // 双侧 bit 共存 ⟹ 两行 verdict（各自独立判定）。
        let mut p = pt(12, true);
        p.bits.sell3 = true;
        let c = Classification {
            levels: vec![
                level(vec![], vec![pt(10, true)]),
                level(vec![center(2, 8)], vec![p]),
            ],
        };
        let v = interval_necessity_tower(&c);
        assert_eq!(v[1].len(), 2);
        assert_eq!((v[1][0].side, v[1][0].satisfied), (Side::Long, true));
        assert_eq!((v[1][1].side, v[1][1].satisfied), (Side::Short, false));
    }
}
