//! 第二类走势递归组装层——port **`Origin/RMoveCompose.lean`（task #51）**。
//!
//! ## 工位定位（#5 单声部根因「第二类结构不可产」的真完整态核心缺口）
//!
//! L2 signal.rs 已产 B1/S1/B3/S3，但 B2/S2（第二类）诚实标「递归组装层缺口」——第二类是中枢
//! 内部回拉结构，需走势递归组装（RMove::Compose + descend）才能产生。本文件 port Origin 已
//! 形式化的**第二类走势结构层**（结构层 L0），建立在 [`super::descend`] 已有的 `RMove`/`descend`/
//! `sub_level_type1` 之上，补**缺失的第二类走势结构层**：
//! - [`compose_move`]：RMove::Compose 组装构造（descend 的逆，组装-取回对偶）。
//! - [`no_new_low`]/[`no_new_high`]/[`retrace_no_break`]：回拉不创新低/新高（第15课「未创新低」）。
//! - [`SecondTypeStructure`] + [`find_second_type_structure`]：第二类走势结构（第一类离开 + 回拉
//!   不创新低/新高 + i1<i2 时间序 + descend 取回的递归组装来源）。
//! - [`second_type_imp_broken_center`]：买卖点定律一连接（第二类结构 ⟹ 次级别破中枢）。
//! - [`second_point_price`]：第二类买卖点价位（回拉走势结束点，§10.1）。
//!
//! 本文件**不**做 B2/S2 信号提取层（从本级别走势自动提取 BspEndpoint + 注入 signal.rs）——
//! 留下一轮（依赖①迁主塔修正后的中枢区间口径②本文件产的递归结构，no-patch 不在未定口径上
//! 构建提取层）。本文件只产**第二类走势结构**，不碰 signal.rs。
//!
//! ## canonical 依据（一级权威博文 + reference §5/§10）
//!
//! - 第14课（博文，买点定律一原始出处）：「大级别的第二类买点由次一级别相应走势的第一类买点
//!   构成。例如，周线上的第二类买点由日线上相应走势的第一类买点构成。」
//! - 第15课（博文）：「第二类买点都是第一次上0轴后回抽确认形成的」+「比较像背驰但**未创新低**」
//!   ——第二类回抽**不创新低**（买点）/不创新高（卖点）。
//! - §10.1（reference）：第二类买点 = 第一类买点后，**次级别上涨结束、再次下跌的那个次级别走势
//!   的结束点**。⟹ 第二类点 = 回拉次级别走势的结束点。
//! - 走势分解定理二（zoushi 第17课，#89）：parent 由次级别走势 compose，subs 含「第一类离开 +
//!   回拉走势」。
//!
//! ## bit-exact 对齐 Lean（语义对应表，Origin RMoveCompose）
//!
//! | Rust | Lean（Origin.RMoveCompose） | 语义 |
//! |------|---------------------------|------|
//! | [`compose_move`] | `composeMove` | RMove::Compose 组装（descend 逆）|
//! | [`no_new_low`] | `NoNewLow`（`m1.lo ≤ m2.lo`）| 回拉不创新低（买点）|
//! | [`no_new_high`] | `NoNewHigh`（`m2.hi ≤ m1.hi`）| 回拉不创新高（卖点）|
//! | [`retrace_no_break`] | `RetraceNoBreak` | 回拉位置约束（按方向）|
//! | [`SecondTypeStructure`] | `SecondTypeStructure` | 第二类走势结构 |
//! | [`find_second_type_structure`] | `SecondTypeStructure`（存在性的构造性判定）| 在 subs 内识别第二类结构 |
//! | [`second_type_imp_broken_center`] | `secondTypeStructure_imp_subBrokenCenter` | 买卖点定律一连接 |
//! | [`second_point_price`] | `secondPointPrice` | 第二类点价位（回拉结束点）|
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! 全部 **L0/L1**（结构定义 + descend 递归取回 + 回拉极值几何比较 + 级别递减；力度分量经 MACD
//! = L1 管线正确性）。`cargo test` 通过 = 第二类走势结构（第一类离开 + 回拉不创新低/新高 +
//! 递归组装来源 + 买卖点定律一连接）在定义层成立，**不**是「第二类识别在真实行情上有效」的
//! 实证断言（L2+，需真实 K 线 + MACD 力度计算的否证检验）。
//!
//! ## 诚实边界（no声明膨胀，与 Lean §still-MISSING 一致）
//!
//! - ✗ B2/S2 信号提取层：本文件产**走势结构** [`SecondTypeStructure`]，未产从本级别走势自动
//!   提取 `BspEndpoint` + 注入 `signal.rs` 多声部的适配器（依赖迁主塔中枢区间口径，留下一轮）。
//! - ✗ 中枢分配 `center_of`：次级别每走势配其相关中枢的自动提取未实装——`center_of` 设为闭包
//!   参数（次级别中枢由上游 detect_centers 提供，同 descend.rs 边界）。
//! - ✗ 「第一次回拉」强约束：本文件回拉只约束「m1 之后 i1<i2 + 不创新低」，未强制是紧邻第一个
//!   回拉（§10.1「相应走势」的强 firstRetrace 留提取层，同 Lean §still-MISSING）。

use super::super::types::{Center, Direction, Side, Tick};
use super::descend::{descend, sub_level_type1, RMove};

/// ★RMove::Compose 组装构造（port `RMoveCompose.composeMove`，组装算子，descend 的逆）。
///
/// 把次级别走势序列 `subs` + 派生中枢 `centers` 封装成级别 `level` 的本级别走势。这是
/// composeStep 逐窗封装用的单走势组装子——第二类的本级别走势由它组装（subs 含第一类离开 +
/// 回拉走势）。组装-取回对偶：`descend(compose_move(subs, ..)) == subs`（见测试）。
pub fn compose_move(subs: Vec<RMove>, centers: Vec<Center>, level: u32) -> RMove {
    RMove::Compose {
        subs,
        centers,
        level,
    }
}

/// ★回拉不创新低（买点侧，几何层；Lean `NoNewLow`：`m1.lo ≤ m2.lo`）。
///
/// 回拉走势 `m2` 的低点不低于第一类走势 `m1` 的低点——第二类买点核心位置约束（第15课「未创
/// 新低」）：第一类探底后，回拉再次下跌但**没创新低**（含相等，临界不创新低）。
pub fn no_new_low(m1: &RMove, m2: &RMove) -> bool {
    m1.lo() <= m2.lo()
}

/// ★回拉不创新高（卖点侧，几何层；Lean `NoNewHigh`：`m2.hi ≤ m1.hi`）。
///
/// 回拉走势 `m2` 的高点不高于第一类走势 `m1` 的高点——第二类卖点对偶位置约束：第一类探顶后，
/// 回抽再次上涨但**没创新高**（含相等，临界不创新高）。
pub fn no_new_high(m1: &RMove, m2: &RMove) -> bool {
    m2.hi() <= m1.hi()
}

/// ★回拉位置约束（按方向，几何层；Lean `RetraceNoBreak`）。
///
/// 第二类回拉走势 `m2` 相对第一类走势 `m1` 不创极值：买点侧判不创新低，卖点侧判不创新高。
pub fn retrace_no_break(side: Side, m1: &RMove, m2: &RMove) -> bool {
    match side {
        Side::Long => no_new_low(m1, m2),
        Side::Short => no_new_high(m1, m2),
    }
}

/// ★第二类买卖点价位 = 回拉走势结束点（port `RMoveCompose.secondPointPrice`，§10.1）。
///
/// 给定回拉走势 `m2`，第二类买卖点价位 = 回拉走势的极值结束点（买点 = 回拉低点 m2.lo，
/// 卖点 = 回拉高点 m2.hi）——§10.1「次级别再次下跌走势的结束点」。
pub fn second_point_price(side: Side, m2: &RMove) -> Tick {
    match side {
        Side::Long => m2.lo(),
        Side::Short => m2.hi(),
    }
}

/// ★第二类走势结构（port `RMoveCompose.SecondTypeStructure`，识别出的结构证据）。
///
/// 本级别走势 `parent`（RMove::Compose 组装）的第二类买卖点结构在 `descend parent` 内识别为：
/// - `i1` / `i2`：第一类离开走势 / 回拉走势在 `descend parent` 中的索引，`i1 < i2`（§10.1
///   「第一类后再次下跌」时间序）。
/// - 第一类离开走势 `descend parent`[i1] 是完整次级别第一类（破中枢 ∧ 背驰，[`sub_level_type1`]）。
/// - 回拉走势 `descend parent`[i2] 不创新低/新高（[`retrace_no_break`]，相对第一类）。
///
/// 第二类买卖点价位 = [`second_point_price`]`(side, descend parent[i2])`（回拉走势结束点）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecondTypeStructure {
    /// 第一类离开走势在 `descend parent` 中的索引。
    pub i1: usize,
    /// 回拉走势在 `descend parent` 中的索引（`i1 < i2`）。
    pub i2: usize,
    /// 第二类买卖点价位（回拉走势结束点）。
    pub second_point: Tick,
}

/// ★第二类走势结构识别（port `RMoveCompose.SecondTypeStructure` 存在性的构造性判定，L0/L1）。
///
/// 在 `descend parent`（RMove::Compose 取回的真实次级别走势序列）内部识别第二类走势结构：
/// 存在 `i1 < i2`，使次级别走势 `[i1]` 是完整第一类（破中枢 ∧ 背驰）∧ 次级别走势 `[i2]`
/// 回拉不创新低/新高。返回**第一个**满足的 `(i1, i2)` 结构证据（None 表无第二类结构）。
///
/// 这是「中枢内部回拉结构」的真递归组装判定——第一类离开是次级别破中枢趋势走势，回拉是次级别
/// 反向走势，二者都在 parent 由 RMove::Compose 封装的 subs（= descend parent）中。
///
/// - `center_of`：次级别每走势配其相关中枢（自动提取未实装，闭包参数，同 descend.rs 边界）。
/// - `divergence_of`：每个次级别走势配 MACD 背驰判定（rust 真算，divergence.rs；力度分量领先
///   Origin——Lean 把 divPair 设外部参数 still-MISSING-C）。
pub fn find_second_type_structure(
    parent: &RMove,
    side: Side,
    center_of: impl Fn(&RMove) -> Center,
    divergence_of: impl Fn(&RMove) -> bool,
) -> Option<SecondTypeStructure> {
    let subs = descend(parent);
    // 第一类离开走势 i1：破中枢 ∧ 背驰（次级别完整第一类）。
    for i1 in 0..subs.len() {
        let m1 = &subs[i1];
        if !sub_level_type1(side, m1, &center_of(m1), divergence_of(m1)) {
            continue;
        }
        // 回拉走势 i2 > i1：不创新低/新高（相对第一类 m1）。
        for i2 in (i1 + 1)..subs.len() {
            let m2 = &subs[i2];
            if retrace_no_break(side, m1, m2) {
                return Some(SecondTypeStructure {
                    i1,
                    i2,
                    second_point: second_point_price(side, m2),
                });
            }
        }
    }
    None
}

/// ★第二类走势结构 ⟹ 次级别有破中枢走势（买卖点定律一连接；Lean
/// `secondTypeStructure_imp_subBrokenCenter`，L0/L1）。
///
/// 若 [`find_second_type_structure`] 识别出第二类结构（含第一类离开 m1），则次级别存在破中枢
/// 走势——买卖点定律一「第二类由次级别第一类构成」的几何必要条件在第二类走势结构上**成立**。
/// 返回 `true` ⟺ 存在第二类走势结构（含第一类离开 ⟹ 次级别破中枢前件成立）。
///
/// 这把本文件的「第二类走势结构」与 descend.rs 的「买卖点定律一几何层」真正连接：第二类走势
/// 结构 ⟹ 买卖点定律一前件（次级别有破中枢走势）。
pub fn second_type_imp_broken_center(
    parent: &RMove,
    side: Side,
    center_of: impl Fn(&RMove) -> Center,
    divergence_of: impl Fn(&RMove) -> bool,
) -> bool {
    find_second_type_structure(parent, side, center_of, divergence_of).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 第二类结构的次级别中枢（核心 [ZD, ZG] = [0, 4]，对齐 Lean `c1Wit`）。
    fn c1_wit() -> Center {
        Center {
            zd: 0,
            zg: 4,
            dd: -2,
            gg: 6,
            start_index: 0,
            end_index: 0,
        }
    }

    /// 第一类离开走势（向下跌破中枢，区间 [-10, -2]，lo=-10 < zd=0 ⟹ 破中枢；对齐 Lean `m1Wit`）。
    fn m1_wit() -> RMove {
        RMove::Segment {
            direction: Direction::Down,
            lo: -10,
            hi: -2,
        }
    }

    /// 回拉走势（再次下跌但不创新低，区间 [-8, 3]，lo=-8 ≥ 第一类 lo=-10；对齐 Lean `m2Wit`）。
    fn m2_wit() -> RMove {
        RMove::Segment {
            direction: Direction::Up,
            lo: -8,
            hi: 3,
        }
    }

    /// 收尾走势（第三段，凑满 ≥3 段；对齐 Lean `m3Wit`）。
    fn m3_wit() -> RMove {
        RMove::Segment {
            direction: Direction::Up,
            lo: 1,
            hi: 5,
        }
    }

    /// 本级别走势：RMove::Compose 组装三个次级别走势（第一类离开 + 回拉 + 收尾），级别 1。
    fn parent_wit2() -> RMove {
        compose_move(vec![m1_wit(), m2_wit(), m3_wit()], vec![c1_wit()], 1)
    }

    /// ★组装-取回对偶（Lean `descend_composeMove` rfl）：descend 取回组装进去的 subs。
    #[test]
    fn compose_descend_roundtrip() {
        let parent = parent_wit2();
        let subs = descend(&parent);
        assert_eq!(subs.to_vec(), vec![m1_wit(), m2_wit(), m3_wit()]);
    }

    /// ★回拉不创新低（Lean `witness_retrace_noNewLow`）：m2.lo=-8 ≥ m1.lo=-10。
    #[test]
    fn retrace_no_new_low() {
        assert!(no_new_low(&m1_wit(), &m2_wit()));
    }

    /// ★回拉创新低则非第二类位置（非平凡，Lean `witness_newLow_breaks_type2`）：
    /// 回拉低点 -12 < 第一类低点 -10 ⟹ ¬no_new_low（「不创新低」是真约束，破之即非第二类）。
    #[test]
    fn new_low_breaks_type2() {
        let m2_break = RMove::Segment {
            direction: Direction::Up,
            lo: -12,
            hi: 3,
        };
        assert!(!no_new_low(&m1_wit(), &m2_break));
    }

    /// ★回拉不创新高（卖点侧对偶）：m2.hi 不高于 m1.hi。
    #[test]
    fn retrace_no_new_high_sell_side() {
        // 卖点：第一类向上探顶 [2, 10]，回抽 [1, 8]，hi=8 ≤ 第一类 hi=10 ⟹ 不创新高。
        let m1_sell = RMove::Segment {
            direction: Direction::Up,
            lo: 2,
            hi: 10,
        };
        let m2_sell = RMove::Segment {
            direction: Direction::Down,
            lo: 1,
            hi: 8,
        };
        assert!(no_new_high(&m1_sell, &m2_sell));
        assert!(retrace_no_break(Side::Short, &m1_sell, &m2_sell));
    }

    /// ★第二类走势结构识别真跑通（Lean `witness_secondTypeStructure`）：
    /// 第一类离开（破中枢 ∧ 背驰）+ 回拉不创新低 + i1=0 < i2=1，识别出第二类结构。
    /// 背驰由外部 divergence_of 提供（第一类离开 m1 背驰，回拉 m2/收尾 m3 不背驰）。
    #[test]
    fn find_second_type_structure_witness() {
        let parent = parent_wit2();
        let result = find_second_type_structure(
            &parent,
            Side::Long,
            |_m| c1_wit(),
            // 仅第一类离开走势（区间含 lo=-10）背驰，回拉/收尾不背驰。
            |m| m.lo() == -10,
        );
        assert_eq!(
            result,
            Some(SecondTypeStructure {
                i1: 0,
                i2: 1,
                second_point: -8, // 回拉走势 m2 结束点 = m2.lo
            })
        );
    }

    /// ★破中枢但不背驰则无第二类结构（非平凡，对齐 Lean 力度必要）：
    /// 第一类离开破中枢但 divergence_of 全 false ⟹ 无完整第一类 ⟹ 无第二类结构。
    #[test]
    fn broke_but_no_divergence_no_second_type() {
        let parent = parent_wit2();
        let result = find_second_type_structure(
            &parent,
            Side::Long,
            |_m| c1_wit(),
            |_m| false, // 力度全 false：无背驰
        );
        assert_eq!(result, None);
    }

    /// ★回拉创新低则无第二类结构（非平凡）：第一类离开后唯一后继回拉创新低（lo=-12 < -10）
    /// ⟹ retrace_no_break 假 ⟹ 无第二类结构。
    #[test]
    fn retrace_new_low_no_second_type() {
        // parent: 第一类离开 [-10,-2] + 回拉创新低 [-12, 3]（破前低）。
        let m2_break = RMove::Segment {
            direction: Direction::Up,
            lo: -12,
            hi: 3,
        };
        let parent = compose_move(vec![m1_wit(), m2_break], vec![c1_wit()], 1);
        let result = find_second_type_structure(&parent, Side::Long, |_m| c1_wit(), |m| m.lo() == -10);
        assert_eq!(result, None);
    }

    /// ★第二类走势结构 ⟹ 次级别破中枢（买卖点定律一连接真跑通，Lean
    /// `witness_secondType_imp_brokenCenter`）。
    #[test]
    fn second_type_imp_broken_center_witness() {
        let parent = parent_wit2();
        assert!(second_type_imp_broken_center(
            &parent,
            Side::Long,
            |_m| c1_wit(),
            |m| m.lo() == -10,
        ));
    }

    /// ★第二类点价位 = 回拉走势结束点（Lean `witness_secondPoint_price`）：买点 = 回拉低点 -8。
    #[test]
    fn second_point_is_retrace_end() {
        assert_eq!(second_point_price(Side::Long, &m2_wit()), -8);
    }

    /// ★线段无第二类结构（递归底，Lean `segment_no_secondType`）：线段 descend 得空 ⟹ 无结构。
    #[test]
    fn segment_no_second_type() {
        let seg = RMove::Segment {
            direction: Direction::Down,
            lo: -10,
            hi: -2,
        };
        let result = find_second_type_structure(&seg, Side::Long, |_m| c1_wit(), |_m| true);
        assert_eq!(result, None);
    }
}
