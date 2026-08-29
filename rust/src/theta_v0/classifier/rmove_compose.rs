//! 第二类走势递归组装层——port **`Origin/RMoveCompose.lean`（task #51）**。
//!
//! ## 工位定位（#5 单声部根因「第二类结构不可产」的真完整态核心缺口）
//!
//! L2 signal.rs 已产 B1/S1/B3/S3，但 B2/S2（第二类）诚实标「递归组装层缺口」——第二类是中枢
//! 内部回拉结构，需走势递归组装（RMove::Compose + descend）才能产生。本文件 port Origin 已
//! 形式化的**第二类走势结构层**（结构层 L0），建立在 [`super::descend`] 已有的 `RMove`/`descend`/
//! `sub_level_type1` 之上，补**缺失的第二类走势结构层**：
//! - [`compose_move`]：RMove::Compose 组装构造（descend 的逆，组装-取回对偶）。
//! - [`no_new_low`]/[`no_new_high`]/[`retrace_no_break`]：回拉极值几何谓词（第15课「未创新低」）——
//!   **#816 B-2② 后不再是准入判据**，只供 [`SecondTypeStructure::retrace_breaks_extreme`] 重合标注取值。
//! - [`m2_broke_center`]：二类回拉 m2 再破中枢（方向敏感，买侧判跌破下沿 / 卖侧判升破上沿）——
//!   #1291 G5 F-2，供 signal.rs `below_last_center`（= Lean `brokeCenter`）作标注面，不作准入分档。
//! - [`SecondTypeStructure`] + [`find_second_type_structure`]：第二类走势结构（第一类离开 + 回拉
//!   段 + i1<i2 时间序 + descend 取回的递归组装来源；回拉**不问**新不新低——#816 B-2②）。
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
//!   ——第二类回抽**不创新低**（买点）/不创新高（卖点）是**常见形态**；经 #816 B-2②
//!   （`101-第101课.md:32`【正文】「第二类买点跌破第一类买点，也就是第二类买点比第一类买点低，
//!   这是完全可以的」），不创新低/新高**不再是准入必要条件**——跌破者标重合身份（二类 × 盘整
//!   背驰，语义归 [#817](https://github.com/xy7365527-lang/NewChanlun/issues/817)），不作闸。
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
//! | [`no_new_low`] | `NoNewLow`（`m1.lo ≤ m2.lo`）| 回拉不创新低（买点；#816 B-2② 后仅作标注取值）|
//! | [`no_new_high`] | `NoNewHigh`（`m2.hi ≤ m1.hi`）| 回拉不创新高（卖点；#816 B-2② 后仅作标注取值）|
//! | [`retrace_no_break`] | `RetraceNoBreak` | 回拉极值谓词（按方向；不作准入闸，#816 B-2②）|
//! | `retrace_breaks_extreme` | `RetraceBreaksExtreme` | 重合标注（回拉破一类极值，不作闸，#816 B-2②）|
//! | [`SecondTypeStructure`] | `SecondTypeStructure` | 第二类走势结构 |
//! | [`find_second_type_structure`] | `SecondTypeStructure`（存在性的构造性判定）| 在 subs 内识别第二类结构 |
//! | [`second_type_imp_broken_center`] | `secondTypeStructure_imp_subBrokenCenter` | 买卖点定律一连接 |
//! | [`second_point_price`] | `secondPointPrice` | 第二类点价位（回拉结束点）|
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! 全部 **L0/L1**（结构定义 + descend 递归取回 + 回拉极值几何比较 + 级别递减；力度分量经 MACD
//! = L1 管线正确性）。`cargo test` 通过 = 第二类走势结构（第一类离开 + 回拉段 + 递归组装来源 +
//! 买卖点定律一连接，回拉**不问**新不新低——#816 B-2②）在定义层成立，**不**是「第二类识别在
//! 真实行情上有效」的实证断言（L2+，需真实 K 线 + MACD 力度计算的否证检验）。
//!
//! ## 诚实边界（no声明膨胀，与 Lean §still-MISSING 一致）
//!
//! - ✗ B2/S2 信号提取层：本文件产**走势结构** [`SecondTypeStructure`]，未产从本级别走势自动
//!   提取 `BspEndpoint` + 注入 `signal.rs` 多声部的适配器（依赖迁主塔中枢区间口径，留下一轮）。
//! - ✗ 中枢分配 `center_of`：次级别每走势配其相关中枢的自动提取未实装——`center_of` 设为闭包
//!   参数（次级别中枢由上游 detect_centers 提供，同 descend.rs 边界）。
//! - ✗ 「第一次回拉」强约束：本文件回拉取 m1 之后**首个后继段**（i2 = i1+1），未强制反向回拉
//!   方向；破一类极值仅作 `retrace_breaks_extreme` 重合标注（#816 B-2②，不作闸）。§10.1
//!   「相应走势」的强 firstRetrace 留提取层（同 Lean §still-MISSING）。

#[cfg(test)]
use super::super::types::Direction; // #913：仅测试消费
use super::super::types::{Center, Side, Tick};
use super::descend::{descend, sub_level_type1, RMove};

/// ★RMove::Compose 组装构造（port `RMoveCompose.composeMove`，组装算子，descend 的逆）。
///
/// 把次级别走势序列 `subs` + 派生中枢 `centers` 封装成级别 `level` 的本级别走势。这是
/// composeStep 逐窗封装用的单走势组装子——第二类的本级别走势由它组装（subs 含第一类离开 +
/// 回拉走势）。组装-取回对偶：`descend(compose_move(subs, ..)) == subs`（见测试）。
pub fn compose_move(subs: Vec<RMove>, centers: Vec<Center>, level: u32) -> RMove {
    RMove::Compose {
        subs: std::rc::Rc::new(subs),
        centers,
        level,
    }
}

/// ★回拉不创新低（买点侧，几何层；Lean `NoNewLow`：`m1.lo ≤ m2.lo`）。
///
/// 回拉走势 `m2` 的低点不低于第一类走势 `m1` 的低点——第15课「未创新低」的几何读数（含相等，
/// 临界不创新低）。★#816 B-2②：本谓词**不再是第二类准入判据**（判据不得以「回拉不创新低/
/// 新高」为必要条件，`101:32`【正文】跌破一买「这是完全可以的」）——只作
/// [`SecondTypeStructure::retrace_breaks_extreme`] 重合标注的取值来源（`!no_new_low` =
/// 跌破一类，标注「一般都构成盘整背驰」的重合身份，语义归 #817，不作闸）。
pub fn no_new_low(m1: &RMove, m2: &RMove) -> bool {
    m1.lo() <= m2.lo()
}

/// ★回拉不创新高（卖点侧，几何层；Lean `NoNewHigh`：`m2.hi ≤ m1.hi`）。
///
/// 回抽走势 `m2` 的高点不高于第一类走势 `m1` 的高点——第15课「未创新高」的几何读数（含相等，
/// 临界不创新高）。★#816 B-2②：本谓词**不再是第二类准入判据**（同 [`no_new_low`] 侧），只作
/// [`SecondTypeStructure::retrace_breaks_extreme`] 重合标注的取值来源（`!no_new_high` =
/// 升破一类，标注重合身份，语义归 #817，不作闸）。
pub fn no_new_high(m1: &RMove, m2: &RMove) -> bool {
    m2.hi() <= m1.hi()
}

/// ★回拉极值谓词（按方向，几何层；Lean `RetraceNoBreak`）。
///
/// 第二类回拉走势 `m2` 相对第一类走势 `m1` 不创极值：买点侧判不创新低，卖点侧判不创新高。
/// ★#816 B-2②：本谓词**不作准入闸**（旧 `:160` 硬闸已拆，实施 #884）——`!retrace_no_break`
/// 即「回拉破一类极值」，由 [`find_second_type_structure`] 取为
/// [`SecondTypeStructure::retrace_breaks_extreme`] 重合标注（跌破一买/升破一卖仍是合法二类点，
/// `101:32`【正文】；标注语义归 #817）。
pub fn retrace_no_break(side: Side, m1: &RMove, m2: &RMove) -> bool {
    match side {
        Side::Long => no_new_low(m1, m2),
        Side::Short => no_new_high(m1, m2),
    }
}

/// ★二类回拉 m2 再破中枢（方向敏感，几何层；#1291 G5 F-2——补 faithful 契约 `¬brokeCenter`
/// 在 m2 上的方向敏感真检查，替换 `signal.rs` 旧常量断言 `below_last_center: false`）。
///
/// 第二类回拉走势 `m2` 相对第一类离开所破的中枢 `c`（B 口径核心区间 [zd,zg]，`c1`）是否
/// **再破**该中枢——方向敏感（对齐 `Origin.BspClassification.IsType2 = afterTypeOne ∧
/// ¬brokeCenter` 的可观测 `brokeCenter` 读数）：
/// - 买侧（Long）：回抽低点 `m2.lo` 是否跌破中枢带下沿 `c.zd`（`m2.lo < c.zd`）；
/// - 卖侧（Short）：回抽高点 `m2.hi` 是否升破中枢带上沿 `c.zg`（`c.zg < m2.hi`）。
///
/// ★方向敏感的必要性（#1291 病灶 ③）：Lean 唯一 L0 定义 `brokeCenterOf m c = m.endPrice <
/// c.zd ∨ c.zg < m.endPrice`（BspConstruction.lean:273）是**方向无关**的「末端价在带外」判据
/// ——字面套用到二类回拉 m2（买侧回拉末端在中枢下方）会把「弱回抽 / 强回抽」都判 `true`
/// （`m2.lo < c.zd` 与 `c.zg < m2.lo` 至少一个成立），与旧常量 `false` 相抵。方向敏感版只判
/// 「本侧再破」（买侧只判跌破下沿，卖侧只判升破上沿），与 `sub_reclassify_broke` 同几何基准
/// （descend.rs，次级别重跑破中枢判定按方向）。
///
/// ★#1291（G5 F-2）口径：本谓词结果进 [`super::bsp::EndpointSituation::below_last_center`]
/// （= Lean `brokeCenter`）作**标注面**，**不作准入分档**（B-2② 口径一致性——`is_second`
/// 判据不含 `below_last_center`；回拉再破中枢仍产二类点，破中枢读数由本字段承载（未持久化
/// 到 `BspPoint`，无下游标注消费者），语义归 #817 同族）。邻接缺口随票记档（t3_in_c_present
/// 无 Lean 对应物、find_map 回填 vs 不回填口径）。
pub fn m2_broke_center(side: Side, m2: &RMove, c: &Center) -> bool {
    match side {
        Side::Long => m2.lo() < c.zd,
        Side::Short => c.zg < m2.hi(),
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
///   「第一类后再次下跌」时间序；回拉段 = 第一类离开之后的**首个后继段**，i2 = i1 + 1）。
/// - 第一类离开走势 `descend parent`[i1] 是完整次级别第一类（破中枢 ∧ 背驰，[`sub_level_type1`]）。
/// - 回拉走势 `descend parent`[i2] **不问**新不新低/新高（#816 B-2②：判据不得以「回拉不创新低/
///   新高」为必要条件）——是否破一类极值由 `retrace_breaks_extreme` 重合标注记录，**不作闸**。
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
    /// ★#816 B-2② 重合身份标注（`101-第101课.md:32`【正文】「第二类买点跌破第一类买点……这是
    /// 完全可以的，这里一般都构成盘整背驰」）：回拉走势是否**跌破**（买）/**升破**（卖）第一类
    /// 离开走势的极值（`!retrace_no_break(side, m1, m2)`）。
    ///
    /// 标注**不作准入分档**——跌破者照样是合法二类点；其「一般都构成盘整背驰」的重合身份
    /// （二类 × 盘整背驰）由本标注供下游消费，语义归
    /// [区间套的教义正本 #817](https://github.com/xy7365527-lang/NewChanlun/issues/817)。
    pub retrace_breaks_extreme: bool,
}

/// ★第二类走势结构识别（port `RMoveCompose.SecondTypeStructure` 存在性的构造性判定，L0/L1）。
///
/// 在 `descend parent`（RMove::Compose 取回的真实次级别走势序列）内部识别第二类走势结构：
/// 存在 `i1 < i2`，使次级别走势 `[i1]` 是完整第一类（破中枢 ∧ 背驰），回拉段 = 其后**首个后继
/// 走势** `[i1+1]`。返回**第一个**满足的 `(i1, i2=i1+1)` 结构证据（None 表无第二类结构）。
///
/// ★#816 B-2②（实施 #884）：旧「回拉不创新低/新高」硬闸（`retrace_no_break`）已拆——判据不得以
/// 「回拉不创新低/新高」为必要条件（`101:32`【正文】跌破一买「这是完全可以的」）；回拉破一类
/// 极值由 [`SecondTypeStructure::retrace_breaks_extreme`] 重合标注记录（「一般都构成盘整背驰」，
/// 语义归 #817），**不作准入分档**。
///
/// 这是「中枢内部回拉结构」的真递归组装判定——第一类离开是次级别破中枢趋势走势，回拉是次级别
/// 后继走势，二者都在 parent 由 RMove::Compose 封装的 subs（= descend parent）中。
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
        // 回拉段 = 第一类离开之后的首个后继走势（i2 = i1 + 1；B-2① 构成形：次级别一类点 +
        // 次级别回拉段；**不问**新不新低——#816 B-2②）。无后继 ⟹ 无回拉段 ⟹ 无第二类结构。
        if i1 + 1 >= subs.len() {
            return None;
        }
        let m2 = &subs[i1 + 1];
        return Some(SecondTypeStructure {
            i1,
            i2: i1 + 1,
            second_point: second_point_price(side, m2),
            retrace_breaks_extreme: !retrace_no_break(side, m1, m2),
        });
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

    /// ★回拉创新低 ⟹ ¬no_new_low（几何谓词本身；Lean `witness_newLow_breaks_type2`）。
    /// #816 B-2②：`¬no_new_low` 即 `retrace_breaks_extreme` 重合标注为真的情形——跌破一类
    /// **仍是合法二类点**（`101:32`【正文】「这是完全可以的」），标注不作准入分档。
    #[test]
    fn new_low_marks_retrace_breaks_annotation() {
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

    /// ★二类回拉 m2 再破中枢（买侧正例，方向敏感；#1291 G5 F-2）：
    /// 回抽低点 m2.lo=-8 < 中枢带下沿 c1.zd=0 ⟹ `m2_broke_center=true`。
    /// 旧常量 `below_last_center: false` 在此输入为常量断言（非检查）；方向敏感真检查返回 `true`
    /// ——与 Lean 方向无关 `brokeCenterOf`（「末端价在带外」）的 `-8 < 0` 读数一致。
    #[test]
    fn m2_broke_center_buy_side_breaks_below() {
        assert!(m2_broke_center(Side::Long, &m2_wit(), &c1_wit()));
    }

    /// ★二类回拉 m2 未再破中枢（买侧反例，方向敏感；#1291 G5 F-2）：
    /// 回抽低点 m2.lo=1 ≥ 中枢带下沿 c1.zd=0 ⟹ `m2_broke_center=false`（回抽未跌破中枢带下沿）。
    #[test]
    fn m2_broke_center_buy_side_stays_inside() {
        let m2_inside = RMove::Segment {
            direction: Direction::Up,
            lo: 1, // ≥ zd=0：回抽低点未跌破中枢带下沿
            hi: 5,
        };
        assert!(!m2_broke_center(Side::Long, &m2_inside, &c1_wit()));
    }

    /// ★二类回抽 m2 再破中枢（卖侧正例，方向敏感；#1291 G5 F-2 镜像）：
    /// 回抽高点 m2.hi=6 > 中枢带上沿 c1.zg=4 ⟹ `m2_broke_center=true`。
    #[test]
    fn m2_broke_center_sell_side_breaks_above() {
        let m2_sell = RMove::Segment {
            direction: Direction::Down,
            lo: 0,
            hi: 6, // > zg=4：回抽高点升破中枢带上沿
        };
        assert!(m2_broke_center(Side::Short, &m2_sell, &c1_wit()));
    }

    /// ★二类回抽 m2 未再破中枢（卖侧反例，方向敏感；#1291 G5 F-2 镜像）：
    /// 回抽高点 m2.hi=3 ≤ 中枢带上沿 c1.zg=4 ⟹ `m2_broke_center=false`（回抽未升破中枢带上沿）。
    #[test]
    fn m2_broke_center_sell_side_stays_inside() {
        let m2_sell = RMove::Segment {
            direction: Direction::Down,
            lo: 1,
            hi: 3, // ≤ zg=4：回抽高点未升破中枢带上沿
        };
        assert!(!m2_broke_center(Side::Short, &m2_sell, &c1_wit()));
    }

    /// ★第二类走势结构识别真跑通（Lean `witness_secondTypeStructure`）：
    /// 第一类离开（破中枢 ∧ 背驰）+ 回拉段（i1=0 < i2=1）+ i2 不破一类极值（m2.lo=-8 ≥ m1.lo=-10）
    /// ⟹ 标注 `retrace_breaks_extreme=false`，识别出第二类结构。
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
                second_point: -8,              // 回拉走势 m2 结束点 = m2.lo
                retrace_breaks_extreme: false, // m2.lo=-8 ≥ m1.lo=-10 ⟹ 未破一类极值
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

    /// ★回拉创新低**仍产**第二类结构（#816 B-2② 拆闸后，实施 #884）：第一类离开后唯一后继
    /// 回拉创新低（lo=-12 < m1.lo=-10，跌破一类）⟹ 是合法二类点（`101:32`【正文】「这是完全
    /// 可以的」），标注重合身份 `retrace_breaks_extreme=true`（「一般都构成盘整背驰」，语义归
    /// #817），**不作准入分档**。旧硬闸下此输入整类判不存在（本票验收的 0→有 对照，见证级）。
    #[test]
    fn retrace_new_low_still_second_type_with_annotation() {
        // parent: 第一类离开 [-10,-2] + 回拉创新低 [-12, 3]（跌破一类极值）。
        let m2_break = RMove::Segment {
            direction: Direction::Up,
            lo: -12,
            hi: 3,
        };
        let parent = compose_move(vec![m1_wit(), m2_break], vec![c1_wit()], 1);
        let result =
            find_second_type_structure(&parent, Side::Long, |_m| c1_wit(), |m| m.lo() == -10);
        assert_eq!(
            result,
            Some(SecondTypeStructure {
                i1: 0,
                i2: 1,
                second_point: -12, // 回拉走势 m2 结束点 = m2.lo（跌破一类的二类点价位）
                retrace_breaks_extreme: true, // m2.lo=-12 < m1.lo=-10 ⟹ 破一类极值（重合标注）
            })
        );
    }

    /// ★回拉段 = 第一类离开之后**首个后继段**（不跳过破极值段）：i2 恒 = i1+1——旧闸会跳过
    /// 破极值的 i1+1 而取更后不破极值的段（判据形状变化，B-2① 构成形 + B-2② 拆闸的读数）。
    /// 本测试锁定新口径：破极值的首个后继段即回拉段，标注 `retrace_breaks_extreme=true`。
    #[test]
    fn pullback_is_first_successor_move() {
        // parent: 第一类离开 [-10,-2] + 回拉创新低 [-12, 3] + 收尾 [1, 5]（m3.lo=1 ≥ m1.lo=-10，
        // 旧闸下会跳过 m2_break 取 m3 为 i2=2）。新口径：回拉段 = m2_break（i2=1）。
        let m2_break = RMove::Segment {
            direction: Direction::Up,
            lo: -12,
            hi: 3,
        };
        let parent = compose_move(vec![m1_wit(), m2_break, m3_wit()], vec![c1_wit()], 1);
        let result =
            find_second_type_structure(&parent, Side::Long, |_m| c1_wit(), |m| m.lo() == -10);
        assert_eq!(
            result,
            Some(SecondTypeStructure {
                i1: 0,
                i2: 1, // ★首个后继段（旧闸下会是 2）
                second_point: -12,
                retrace_breaks_extreme: true,
            })
        );
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
