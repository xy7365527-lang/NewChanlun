//! 下钻次级别——port **`Origin/SubLevelDescent.lean`（#119）**。
//!
//! ## 工位定位（G2 下钻次级别缺口，task #126）
//!
//! 闭环引擎无 `descend` 算子、classifier 单级别 → **2 类买卖点结构上不可识别**（2 类=次级别 1 类
//! 构造，§10.2 买卖点定律一）。本文件 port Origin 已形式化的真下钻：
//! - [`descend`]：从次级别走势序列取回（对齐 RecursiveLevelSystem `lift` 逆，Lean `descend`）。
//! - 级别递归（[`RMove::level`] 严格递减，well-founded，Lean `descend_level_decreases`）。
//! - 2 类买卖点识别（次级别 1 类构造，Lean `secondType_via_subLevel_type1`）。
//!
//! ★力度分量用已实装的 MACD（rust 在此领先 Origin）：Lean SubLevelDescent 把背驰 `divPair` 设为
//! **外部参数**（still-MISSING-C：无 Origin 模块从 K 线算 MACD 面积）。rust 已实装 MACD
//! （divergence.rs `segments_diverge`），故本文件的次级别第一类**力度分量真算**（[`SubLevelType1`]
//! 携带真 MACD 背驰 bool，非外部占位）——这是 rust 相对 Origin 的提升。
//!
//! ## bit-exact 对齐 Lean（语义对应表）
//!
//! | Rust | Lean（Origin SubLevelDescent） | 语义 |
//! |------|------------------------------|------|
//! | [`RMove`] | `Origin.SubLevelDescent.RMove`（= `Formal.RecursiveConstruction.Move` μF 别名）| 递归走势（携 level/interval/subs）|
//! | [`descend`] | `descend`（lift 逆） | 取回次级别走势序列 subs |
//! | [`sub_broke_below`] | `SubBrokeBelow`（`m.lo < c.zd`）| 向下破中枢（买点侧几何）|
//! | [`sub_broke_above`] | `SubBrokeAbove`（`c.zg < m.hi`）| 向上破中枢（卖点侧几何）|
//! | [`sub_level_has_broken_center`] | `subLevelHasBrokenCenter` | 次级别存在破中枢走势 |
//! | [`sub_level_type1`] | `SubLevelType1`（破中枢∧背驰）| 次级别完整第一类（力度真算）|
//! | [`second_type_via_sublevel_type1`] | `secondType_via_subLevel_type1` | 第二类⟸次级别第一类 |
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! 全部 **L0/L1**（结构递归下钻 + 几何破位判定 + 级别递减；力度分量经 MACD = L1 管线正确性）。
//! `cargo test` 通过 = 真下钻取次级别走势序列 + 次级别重跑破中枢几何 + 级别严格递减终止 +
//! 第二类⟸次级别第一类构成在定义层成立，**不**是「次级别第一类识别在真实行情上有效」的实证
//! 断言（L2+，需真实 K 线 + MACD 力度计算的否证检验）。
//!
//! ## 诚实边界（no声明膨胀，与 Lean §still-MISSING 一致）
//!
//! - ✗ 完整 ParseStruct 重跑：本文件下钻到次级别**走势序列**层 + 破中枢几何，未重跑次级别
//!   mergeBars→fractals→strokes→segments→centers 全元素流水线（那需次级别 Bar，#84 L2 territory）。
//!   descend 取的是递归塔的次级别走势（已是 Move 层），非从次级别 Bar 重解析。
//! - ✗ 中枢分配 `center_of`：次级别每走势配「最后一个中枢」的自动提取未实装——`center_of` 设为
//!   闭包参数（次级别中枢由上游 detect_centers 提供）。

use super::super::types::{Center, Direction, Side, Tick};

/// 递归走势 `RMove`（port `Origin.SubLevelDescent.RMove` = `Formal.RecursiveConstruction.Move` μF 别名，携带 level/interval/subs）。
///
/// 缠论递归走势的两个构造子（对齐 Lean `Move.segment` / `Move.compose`）：
/// - `Segment`：线段（递归底 level 0，无次级别，Lean `Move.segment d lo hi`）。
/// - `Compose`：上级走势（由次级别走势序列 `subs` compose 而成，携 level/centers，
///   Lean `Move.compose subs centers level`）。`subs` 是 RecursiveLevelSystem `lift`（composeStep）
///   把次级别窗口封装为上级走势的载荷——[`descend`] 逆向取回它。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RMove {
    /// 线段（递归底，level 0）。
    Segment {
        direction: Direction,
        lo: Tick,
        hi: Tick,
    },
    /// 上级走势（compose 次级别走势序列）。
    Compose {
        subs: Vec<RMove>,
        centers: Vec<Center>,
        level: u32,
    },
}

impl RMove {
    /// 走势级别（Lean `Move.level`）：Segment=0（递归底）；Compose=携带的 level。
    pub fn level(&self) -> u32 {
        match self {
            RMove::Segment { .. } => 0,
            RMove::Compose { level, .. } => *level,
        }
    }

    /// 走势区间下沿 lo（Lean `Move.lo`/`Move.interval`）。
    /// Segment 直接取 lo；Compose 取 subs 区间下沿的最小值（外缘下沿，无 subs ⟹ 0 占位）。
    pub fn lo(&self) -> Tick {
        match self {
            RMove::Segment { lo, .. } => *lo,
            RMove::Compose { subs, .. } => subs.iter().map(|m| m.lo()).min().unwrap_or(0),
        }
    }

    /// 走势区间上沿 hi（Lean `Move.hi`/`Move.interval`）。
    pub fn hi(&self) -> Tick {
        match self {
            RMove::Segment { hi, .. } => *hi,
            RMove::Compose { subs, .. } => subs.iter().map(|m| m.hi()).max().unwrap_or(0),
        }
    }
}

/// ★下钻算子 `descend`（port `SubLevelDescent.descend`，lift 逆，真下钻核心）。
///
/// 从一个走势取回它的**次级别走势序列**：
/// - `Compose { subs, .. }`：取回封装的 `subs`（次级别走势序列）——`lift`（composeStep）把次级别
///   窗口封装为本级别走势的**逆操作**（Lean `descend_compose` rfl）。取出的是真实次级别走势对象
///   （携各自 interval），非本级别端点复制。
/// - `Segment`：线段是递归底（level 0），**无次级别**——下钻得空序列（Lean `descend_segment`）。
// ponytail: 返回 &[RMove] 替 Vec<RMove>::clone——调用方只读遍历，零拷贝切片引用
pub fn descend(parent: &RMove) -> &[RMove] {
    match parent {
        RMove::Segment { .. } => &[],
        RMove::Compose { subs, .. } => subs.as_slice(),
    }
}

/// 次级别走势破中枢（向下，买点侧，几何层；Lean `SubBrokeBelow`：`m.lo < c.zd`）。
///
/// 一个次级别走势跌破中枢 ⟺ 其价格区间下沿严格低于中枢核心下沿（走势离开中枢进入「之下」）。
/// 这是第一类买点判据的几何必要条件「次级别向下跌破最后一个中枢」。
pub fn sub_broke_below(m: &RMove, c: &Center) -> bool {
    m.lo() < c.zd
}

/// 次级别走势破中枢（向上，卖点侧，几何层；Lean `SubBrokeAbove`：`c.zg < m.hi`）。
pub fn sub_broke_above(m: &RMove, c: &Center) -> bool {
    c.zg < m.hi()
}

/// 次级别重跑破中枢判定（按方向，几何层；Lean `subReclassifyBroke`）。
///
/// 对下钻取回的次级别走势 + 其中枢 + 买卖方向，重跑破中枢几何：买点侧判向下破，卖点侧判向上破。
/// 对**次级别对象**算（非本级别 e 重判），是真「次级别重跑」。
pub fn sub_reclassify_broke(side: Side, m: &RMove, c: &Center) -> bool {
    match side {
        Side::Long => sub_broke_below(m, c),
        Side::Short => sub_broke_above(m, c),
    }
}

/// 次级别走势序列存在破中枢者（几何层，真下钻；Lean `subLevelHasBrokenCenter`）。
///
/// `descend parent` 取真实次级别走势序列，对每个次级别走势重跑破中枢几何，判**存在**一个真破中枢
/// 的走势。`center_of` 给次级别每走势配其相关中枢（Lean 同名参数，自动提取未实装，见模块头）。
pub fn sub_level_has_broken_center(
    parent: &RMove,
    side: Side,
    center_of: impl Fn(&RMove) -> Center,
) -> bool {
    descend(parent)
        .iter()
        .any(|m| sub_reclassify_broke(side, m, &center_of(m)))
}

/// 次级别第一类（完整判据，破中枢 ∧ 背驰；Lean `SubLevelType1`）。
///
/// 一个下钻取回的次级别走势是第一类买卖点 ⟺ 破中枢（几何，真下钻可判）∧ 背驰（力度）。
///
/// ★rust 领先 Origin：`is_divergence` 由已实装 MACD（divergence.rs `segments_diverge`）**真算**
/// 后传入——Lean 把 `divPair` 设外部参数（still-MISSING-C 无 MACD 引擎），rust 已有 MACD 引擎，
/// 故力度分量真算非占位。本谓词消费 MACD 计算结果（破中枢几何 ∧ MACD 背驰）。
pub fn sub_level_type1(side: Side, m: &RMove, c: &Center, is_divergence: bool) -> bool {
    let broke = match side {
        Side::Long => sub_broke_below(m, c),
        Side::Short => sub_broke_above(m, c),
    };
    broke && is_divergence
}

/// ★★第二类买卖点 ⟸ 次级别第一类构成（买卖点定律一，真下钻；Lean `secondType_via_subLevel_type1`）。
///
/// 本级别第二类成立的构成性必要条件：若次级别（descend 取回）某走势是完整第一类（破中枢 ∧ 背驰），
/// 则次级别存在破中枢走势（[`sub_level_has_broken_center`] 真）——即本级别第二类的次级别第一类
/// 构成在几何层被真下钻见证（§10.2「第二类由次级别第一类构成」的真下钻形式）。
///
/// 返回 `true` ⟺ 存在一个下钻取回的次级别走势满足完整第一类（破中枢 ∧ 背驰）⟹ 第二类构成成立。
/// `divergence_of` 给每个次级别走势配 MACD 背驰判定（rust 真算，divergence.rs）。
pub fn second_type_via_sublevel_type1(
    parent: &RMove,
    side: Side,
    center_of: impl Fn(&RMove) -> Center,
    divergence_of: impl Fn(&RMove) -> bool,
) -> bool {
    descend(parent)
        .iter()
        .any(|m| sub_level_type1(side, m, &center_of(m), divergence_of(m)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub_center() -> Center {
        // 核心 [ZD,ZG]=[10,20]，外缘 [DD,GG]=[5,25]（port Lean `subCenter`）。
        Center { zd: 10, zg: 20, dd: 5, gg: 25, start_index: 0, end_index: 0 }
    }

    /// 向下跌破的次级别走势（区间 [-5,1]，下沿 -5 < zd=10；port Lean `subMoveBroke`）。
    fn sub_move_broke() -> RMove {
        RMove::Segment { direction: Direction::Down, lo: -5, hi: 1 }
    }

    /// 未破中枢的次级别走势（区间 [12,18]，下沿 12 >= zd=10；port Lean `subMoveInside`）。
    fn sub_move_inside() -> RMove {
        RMove::Segment { direction: Direction::Up, lo: 12, hi: 18 }
    }

    /// 本级别走势（compose 三个次级别走势，其一向下破中枢；port Lean `parentWit`）。
    fn parent_wit() -> RMove {
        RMove::Compose {
            subs: vec![sub_move_broke(), sub_move_inside(), sub_move_inside()],
            centers: vec![sub_center()],
            level: 1,
        }
    }

    // ── descend（port SubLevelDescent §1）────────────────────────────────────

    /// descend 取回真实次级别走势序列（Lean `descend_compose` / `witness_descend`）。
    #[test]
    fn descend_compose_returns_subs() {
        let parent = parent_wit();
        let subs = descend(&parent);
        assert_eq!(subs.len(), 3);
        assert_eq!(subs[0], sub_move_broke());
        assert_eq!(subs[1], sub_move_inside());
    }

    /// 线段下钻得空（递归底；Lean `descend_segment`）。
    #[test]
    fn descend_segment_empty() {
        let seg = RMove::Segment { direction: Direction::Up, lo: 0, hi: 10 };
        assert!(descend(&seg).is_empty());
    }

    /// ★级别严格递减（well-founded 基础；Lean `descend_level_decreases`）：
    /// compose level=lvl ⟹ 下钻次级别 level = lvl - 1（这里 segment subs level=0 = 1-1）。
    #[test]
    fn descend_level_decreases() {
        let parent = parent_wit(); // level 1
        assert_eq!(parent.level(), 1);
        for m in descend(&parent) {
            assert_eq!(m.level(), parent.level() - 1, "下钻级别严格递减");
        }
    }

    /// 多级嵌套级别递减（level 2 → subs level 1 → subs level 0）。
    #[test]
    fn descend_nested_level_decreases() {
        let l2 = RMove::Compose {
            subs: vec![parent_wit(), parent_wit()], // each level 1
            centers: vec![sub_center()],
            level: 2,
        };
        for m in descend(&l2) {
            assert_eq!(m.level(), 1, "L2 下钻得 L1");
            for sub in descend(&m) {
                assert_eq!(sub.level(), 0, "L1 下钻得 L0 线段");
            }
        }
    }

    // ── 破中枢几何（port SubLevelDescent §2）────────────────────────────────

    /// 取回的次级别走势真跌破中枢（Lean `witness_subMove_broke`）：lo=-5 < zd=10。
    #[test]
    fn sub_move_broke_below() {
        assert!(sub_broke_below(&sub_move_broke(), &sub_center()));
    }

    /// 未破中枢的次级别走势真未破（Lean `witness_subMove_inside`）：lo=12 >= zd=10。
    #[test]
    fn sub_move_inside_not_broke() {
        assert!(!sub_broke_below(&sub_move_inside(), &sub_center()));
    }

    /// 向上破中枢（卖点侧）：hi > zg。
    #[test]
    fn sub_move_broke_above_sell_side() {
        // 区间 [22,30]：hi=30 > zg=20 ⟹ 向上突破（卖点侧）。
        let up = RMove::Segment { direction: Direction::Up, lo: 22, hi: 30 };
        assert!(sub_broke_above(&up, &sub_center()));
        assert!(sub_reclassify_broke(Side::Short, &up, &sub_center()));
    }

    // ── 真下钻第一类几何（port SubLevelDescent §3）──────────────────────────

    /// 真下钻判到次级别存在破中枢走势（买点侧，几何层；Lean `witness_subLevel_hasBrokenCenter`）。
    #[test]
    fn sub_level_has_broken_center_witness() {
        assert!(sub_level_has_broken_center(&parent_wit(), Side::Long, |_| sub_center()));
    }

    /// 无破中枢走势 ⟹ false（全 inside 次级别走势）。
    #[test]
    fn sub_level_no_broken_center() {
        let all_inside = RMove::Compose {
            subs: vec![sub_move_inside(), sub_move_inside()],
            centers: vec![sub_center()],
            level: 1,
        };
        assert!(!sub_level_has_broken_center(&all_inside, Side::Long, |_| sub_center()));
    }

    // ── 第二类 ⟸ 次级别第一类（port SubLevelDescent §4，力度 MACD 真算）─────

    /// ★★第二类⟸次级别第一类（破中枢 ∧ MACD 背驰；Lean `witness_secondType_via_type1`）。
    /// 力度由 rust MACD 真算传入（背驰 true）+ 几何破中枢真下钻判定 ⟹ 第二类构成成立。
    #[test]
    fn second_type_via_sublevel_type1_witness() {
        // divergence_of：sub_move_broke 处 MACD 背驰为 true（rust 真算的占位见证），其余 false。
        let ok = second_type_via_sublevel_type1(
            &parent_wit(),
            Side::Long,
            |_| sub_center(),
            |m| *m == sub_move_broke(), // 破中枢的那段背驰（MACD forceC < forceA）
        );
        assert!(ok, "次级别第一类（破中枢∧背驰）⟹ 第二类构成");
    }

    /// 破中枢但**不背驰**（MACD 力度延续）⟹ 非第一类 ⟹ 第二类构成不成立（否定性边界）。
    /// 这是 formalization-validity-domain 的 L2 否证入口：几何破中枢非充分，需 MACD 背驰。
    #[test]
    fn broke_but_no_divergence_no_second_type() {
        let ok = second_type_via_sublevel_type1(
            &parent_wit(),
            Side::Long,
            |_| sub_center(),
            |_| false, // MACD 力度延续（破中枢但不背驰）
        );
        assert!(!ok, "破中枢但不背驰 ⟹ 非次级别第一类 ⟹ 第二类不成立");
    }

    /// sub_level_type1 完整判据：破中枢 ∧ 背驰（力度 MACD 真算）。
    #[test]
    fn sub_level_type1_needs_both() {
        let c = sub_center();
        // 破中枢 ∧ 背驰 ⟹ 第一类。
        assert!(sub_level_type1(Side::Long, &sub_move_broke(), &c, true));
        // 破中枢 ∧ 非背驰 ⟹ 非第一类（力度不足）。
        assert!(!sub_level_type1(Side::Long, &sub_move_broke(), &c, false));
        // 未破中枢 ∧ 背驰 ⟹ 非第一类（几何不足）。
        assert!(!sub_level_type1(Side::Long, &sub_move_inside(), &c, true));
    }
}
