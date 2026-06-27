//! 信号端点提取（reference-theta-v0.md:34-36）——confirmed 走势结构 → 买卖点。
//!
//! ## 契约重锚（legacy Formal/BSPLabels + Claim9 → `Origin.BspClassification` + `Origin.CenterStates`）
//!
//! 把 confirmed 中枢 + 线段序列提取为买卖点端点（`BspPoint`）。每个端点的语义状态（对齐
//! `Origin.BspClassification.BspEndpoint` 字段）由走势结构相对中枢的位置（`Origin.CenterStates.
//! classifyPosition`）+ 背驰力度（`Origin.Divergence.IsDivergence`，由已实装 MACD 真算）bit-exact
//! 计算（非臆造）。
//!
//! ## 完整买卖点覆盖（formalization-validity-domain，★关键 L 级边界标注）
//!
//! 本工位提取**第一类 + 第三类**买卖点（confirmed 结构上**严格可判定**的两类）：
//!
//! - **第三类**（B3/S3）：reference:36「上离中枢后次级别回试低点 `>ZG`=3买；下离后回抽高点
//!   `<ZD`=3卖」是**点相对中枢核心区间的位置判据**（`Origin.BspClassification.IsType3Buy/IsType3Sell`
//!   + `Origin.CenterStates.classifyPosition`，已 bit-exact 形式化），只需 confirmed 中枢 + 后续
//!   线段端点价，**纯整数几何 L0**，零经验时序依赖。
//!
//! - **第一类**（B1/S1）：reference:34 + `Origin.BspClassification.IsType1 = brokeCenter ∧
//!   IsDivergence(divPair)`。两分量分层：
//!   · **破中枢几何分量**（`brokeCenter`）：离开中枢的段端点落中枢核心 `[zd,zg]` 之外（买侧
//!     向下破 `< zd`，卖侧向上破 `> zg`）——纯整数比较，**L0**（608号位置三态，对齐 descend.rs
//!     `sub_broke_below`/`sub_broke_above`）。
//!   · **背驰力度分量**（`IsDivergence`）：破中枢段相对**前一同向段** MACD 面积**严格变小**
//!     （reference:34「末段相对前同向段面积严格变小」）——由已实装 MACD（`divergence::segments_diverge`）
//!     **真算**（★rust 领先 Origin：Lean `divPair` 是外部参数 still-MISSING-C 无 Origin MACD 引擎；
//!     rust `divergence.rs` 已实装 MACD，故背驰分量**真算非占位**，见 descend.rs 模块头）。
//!     认识论：MACD 段面积比较确定 = **L1**（管线正确性，bit-exact 对齐 Lean IsType1 结构合取）；
//!     「MACD 背驰预测在真实行情有效」才是 **L2/L3**（否证检验，**不在本工位**）。
//!
//! ## 第二类（B2/S2）：递归组装层提取入口 `extract_second_signals`（#52 收尾，★no-patch）
//!
//! 第二类买卖点**不在 `extract_signals`（L0 segment 层）产出**——这是诚实的**签名层**边界，
//! 不是遗漏：`IsType2 = afterTypeOne ∧ ¬brokeCenter`，且**买卖点定律一**（§10.2，第14课）：
//! 「任何级别的第二类买卖点都由次级别相应走势的**第一类**构成」。`extract_signals` 入参 `segments`
//! 是 **L0 线段**——线段是递归底（`descend.rs::RMove::Segment` 下钻得空序列），**结构上无次级别
//! 走势对象**。「次级别第一类构成」需要 `descend.rs::RMove::Compose { subs, .. }` 的递归载荷
//! （次级别走势序列）作输入——`extract_signals(centers, segments, ...)` 的签名层**拿不到**它。
//! 在 `extract_signals` 内硬产第二类 = 用本级别 `afterTypeOne` 占位冒充次级别第一类构成 =
//! 声明膨胀（禁止）。`signal_extraction_emits_no_second_class` 锁定 L0 层不产第二类的边界。
//!
//! **#52 消除「递归组装层缺口」标记**：依赖已满足——`rmove_compose.rs::find_second_type_structure`
//! （port `Origin.RMoveCompose.SecondTypeStructure`，#51 已真封）消费 RMove 递归塔，从 `descend parent`
//! 取回的次级别走势序列内识别第二类走势结构（第一类离开 m1 + 回拉 m2 不创新低/新高 + i1<i2 时间序）。
//! 本模块补**平行的递归组装层提取入口** [`extract_second_signals`]：消费 RMove 递归塔（`parent: &RMove`）
//! + B 口径中枢区间，调 `find_second_type_structure` 产 B2/S2 `BspPoint`。这**不是**在 `extract_signals`
//! 内硬产（那会声明膨胀），而是补上**递归组装层本身**——`extract_signals`（L0）与 `extract_second_signals`
//! （递归组装层）是**两个平行的提取入口**，按输入对象（L0 segment vs RMove 递归塔）分工，零冒充。
//!
//! ⟹ `extract_signals` 多声部产 B1/S1/B3/S3（消解 #5「单声部 L0 第三类」根因）；
//!    `extract_second_signals` 递归组装层产 B2/S2（消解 #5「第二类结构不可产」收尾）。
//! 下游 E5 平仓闭环（trades=0）是 L3 层（不在本工位，见 closed_loop）。
//!
//! ## ★诚实 still-MISSING（B2/S2 递归组装层提取的开口，no-声明膨胀）
//!
//! - **RMove 递归塔生产路径未接入**（still-MISSING-塔）：`mod.rs::classify` 当前用 `UnitRange`+`Center`
//!   的递归级别系统，**未构造 RMove::Compose 塔**。故 `extract_second_signals` 的提取逻辑已封（消费
//!   `SecondTypeStructure`），但生产路径上**无 RMove 塔可喂**——接入需上游 `mod.rs` 把 `UnitRange`
//!   递归塔转译为 `RMove::Compose`（携 source_index 坐标），属递归级别系统的塔构造工位，不在本签名层。
//! - **次级别坐标 `index_of`**（still-MISSING-坐标）：`RMove`（port Lean `Move` μF）是纯结构区间，
//!   **无 source_index**。B2/S2 `BspPoint.source_index`（平局裁决+回溯，reference:16）由 `index_of`
//!   闭包提供（次级别走势→原始 K 序，上游塔构造时填）——与 `center_of`/`divergence_of` 同精神（次级别
//!   坐标/中枢/力度由上游提供，descend.rs 边界）。本模块不冒充自动坐标分配。
//! - **背驰力度引擎**（still-MISSING-C，承接 SubLevelDescent）：第一类离开 m1 的背驰由 `divergence_of`
//!   闭包提供。rust `divergence.rs` 已实装 MACD（领先 Origin），但「次级别走势→MACD 背驰」的自动配对
//!   需 RMove 塔携带各次级别走势的 close 区间——同 still-MISSING-塔，由上游塔构造时接入真 MACD。

use super::super::config::MacdConfig;
use super::super::types::{Center, Direction, Segment, Side, Tick};
use super::bsp::{endpoint_to_bsp, EndpointSituation};
use super::divergence::{compute_macd, segments_diverge};
use super::super::types::BspBits;
use super::descend::RMove;
use super::rmove_compose::find_second_type_structure;

/// 买卖点条目（带结构止损价，single source，见 `bsp::BspPoint`）。
pub use super::bsp::BspPoint;

/// 线段端点投影（信号候选点：每条线段的终止端点 = 一个潜在买卖点）。
struct SegEnd {
    source_index: usize,
    /// 端点价（线段终止价 = 该端点的极值）。
    price: Tick,
    /// 该线段方向（向上线段端点 = 高点候选，向下 = 低点候选）。
    dir: Direction,
}

fn seg_end(s: &Segment) -> SegEnd {
    SegEnd {
        source_index: s.end_index,
        price: s.end_price,
        dir: s.direction,
    }
}

/// 把段的 `source_index`（原始 K 序）区间映射到 `closes` 序列的下标区间（MACD 段面积坐标系）。
///
/// ★坐标系一致性（formalization-validity-domain，无漂移）：段的 `start_index/end_index` 是
/// `source_index`（原始 K 序，stroke.rs:94 链）；`closes` 是 `ParseLayer.merged_bars` 的 close
/// 序列（classify 唯一可达的 close 来源）。merged_bars 各元素携带 `source_index`（合并保留起始
/// bar 原始号，单调递增）。本函数把段的 source_index 端点映射到 merged_bars **下标**，使 MACD 段
/// 面积区间与 `closes` 同坐标系——**不**跨「原始 bars / merged_bars」两序列混用（那会引入面积错位）。
///
/// `src_to_idx` 是 merged_bars 下标 → source_index 的升序映射（`closes[k]` 对应 `src_to_idx[k]`）。
/// 找首个 `>= start` 的下标 lo 与末个 `<= end` 的下标 hi。区间空（无 bar 落入）⟹ None。
fn map_src_range_to_close_idx(
    src_to_idx: &[usize],
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    if start > end {
        return None;
    }
    // lo = 首个 source_index >= start 的 closes 下标。
    let lo = src_to_idx.iter().position(|&s| s >= start)?;
    // hi = 末个 source_index <= end 的 closes 下标。
    let hi = src_to_idx.iter().rposition(|&s| s <= end)?;
    if lo > hi {
        return None;
    }
    Some((lo, hi))
}

/// 第一类买卖点提取（契约锚 `Origin.BspClassification.IsType1 = brokeCenter ∧ IsDivergence`）。
///
/// reference:34「某级别趋势中，次级别向下跌破最后一个中枢后形成的**背驰点**」。对每个 confirmed
/// 中枢 `c`，扫描其后线段序列，找**破中枢的段** + **背驰**（破中枢段 vs 前一同向段 MACD 面积严格
/// 变小）：
/// - **1买**：一条**向下线段**跌破中枢下方（端点 `< zd`，`brokeCenter` 几何 L0），且该段相对
///   **前一向下段** MACD 面积**严格变小**（背驰，`segments_diverge` 真算）⟹ 该破中枢段端点 = 1 买。
/// - **1卖**：镜像——**向上线段**突破中枢上方（端点 `> zg`），相对**前一向上段**面积严格变小 ⟹ 1 卖。
///
/// ★分量 L 级（formalization-validity-domain）：破中枢 `< zd`/`> zg` 是整数几何 **L0**；背驰由
/// MACD `segments_diverge` 真算 = **L1**（管线正确性）。两者合取 = `IsType1`（对齐 Lean，bit-exact）。
/// 「前一同向段」是序列里同 direction 的最近前驱段（reference:34「末段相对**前**同向段」的确定配对——
/// 非任意配对，是序列序最近同向前驱，确定可定位 ⟹ 无歧义）。
///
/// `hist` 是 MACD hist 序列（`closes` 上算）；`src_to_idx` 是 closes 下标→source_index 映射（段
/// 区间坐标系转换）。破中枢段或前同向段无法映射到 closes 区间（越界）⟹ 跳过（无面积 ⟹ 非背驰）。
fn extract_first_for_center(
    c: &Center,
    segs_after: &[Segment],
    hist: &[f64],
    src_to_idx: &[usize],
) -> Vec<BspPoint> {
    let mut points = Vec::new();
    for (i, seg) in segs_after.iter().enumerate() {
        let end = seg_end(seg);
        // 破中枢几何（L0）：买侧向下破（端点 < zd）；卖侧向上破（端点 > zg）。
        let (broke, is_sell) = match end.dir {
            Direction::Down if end.price < c.zd => (true, false), // 1 买：向下破中枢下沿
            Direction::Up if c.zg < end.price => (true, true),    // 1 卖：向上破中枢上沿
            _ => (false, false),
        };
        if !broke {
            continue;
        }
        // 前一同向段（序列序最近同向前驱）——reference:34「末段相对前同向段」的确定配对。
        let prev_same_dir = segs_after[..i].iter().rev().find(|s| s.direction == end.dir);
        let Some(prev) = prev_same_dir else {
            // 无前同向段 ⟹ 无背驰对照标的 ⟹ 非第一类（第一类是趋势末段，必有前同向段）。
            continue;
        };
        // 段区间（source_index）→ closes 下标区间（MACD 面积坐标系）。
        let (Some(curr_seg), Some(prev_seg)) = (
            map_src_range_to_close_idx(src_to_idx, seg.start_index, seg.end_index),
            map_src_range_to_close_idx(src_to_idx, prev.start_index, prev.end_index),
        ) else {
            // 段无法映射到 closes 区间（越界/空）⟹ 无 MACD 面积 ⟹ 跳过（不冒充背驰）。
            continue;
        };
        // 背驰（L1 真算）：破中枢段（curr）面积严格小于前同向段（prev）面积。
        if !segments_diverge(hist, prev_seg, curr_seg) {
            continue; // 力度未衰减 ⟹ 非背驰 ⟹ 非第一类。
        }
        // 第一类端点：below_last_center（买）/对偶（卖），未离开中枢（破中枢 ≠ 离开后回抽）。
        let situ = EndpointSituation {
            after_first_buy: false,
            is_pullback_end: false,
            left_center: false,         // 第一类是破中枢背驰，非第三类的离开后回抽
            retrace_not_reenter: false,
            below_last_center: true,    // 破中枢背驰端点（买=中枢下方/卖镜像）
            is_sell_side: is_sell,
        };
        let bits = endpoint_to_bsp(&situ);
        // 第一类止损 = pivot（破中枢段端点极值）：买点 pivot_low、卖点 pivot_high。
        points.push(make_first_point(end.source_index, bits, end.price));
    }
    points
}

/// 第三类买卖点提取（契约锚 `Origin.BspClassification.IsType3Buy/IsType3Sell` 点位判据）。
///
/// 对每个 confirmed 中枢 `c`，扫描其后的线段端点序列，按 reference:36 判第三类：
/// - **3买**：一条**向上线段**离开中枢上方（端点 `> zg`），紧随的**向下线段**回试低点
///   `> zg`（不重新触及中枢闭区间）⟹ 该回试低点端点 = 3 买。
/// - **3卖**：一条**向下线段**离开中枢下方（端点 `< zd`），紧随的**向上线段**回抽高点
///   `< zd`（不重新触及中枢闭区间）⟹ 该回抽高点端点 = 3 卖。
///
/// `segs_after` 是中枢 `end_index` 之后的线段序列（按时间序）。逐相邻对 (leave, retest)
/// 判定。bit-exact：边界用 **严格口径**（`> zg` / `< zd`，等号排除）——第三类买点是中枢
/// **终结点**，retest==zg=单点重叠仍触及闭区间中枢 `[ZD,ZG]`（中心定理一：与 `[ZD,ZG]`
/// 重叠=中枢延伸，非终结）⟹ 非第三类。逐字段对齐 Lean `IsType3Buy`（`zg < retracePrice`
/// 严格，BspClassification.lean:113）与 legacy `buysellpoint.rs:414`（`low > zg` 严格）。
/// codex 裁决 2026-06-27（中枢终结语义）。
fn extract_third_for_center(c: &Center, segs_after: &[Segment]) -> Vec<BspPoint> {
    let mut points = Vec::new();
    // 逐相邻线段对：前者离开中枢，后者回试。
    for pair in segs_after.windows(2) {
        let leave = seg_end(&pair[0]);
        let retest = seg_end(&pair[1]);
        match (leave.dir, retest.dir) {
            // 3 买：向上离开（leave 端点 > zg）+ 向下回试（retest 低点 > zg，不触及闭区间中枢）。
            // 严格 `>`：retest==zg=单点重叠仍触及中枢 [ZD,ZG]=非终结=非第三类（codex 裁决 2026-06-27）。
            (Direction::Up, Direction::Down) => {
                if leave.price > c.zg && retest.price > c.zg {
                    let situ = EndpointSituation {
                        after_first_buy: false,
                        is_pullback_end: false,
                        left_center: true,        // 离开中枢（leave.price > zg）
                        retrace_not_reenter: true, // 回试不触及闭区间中枢（retest > zg，严格）
                        below_last_center: false,
                        is_sell_side: false,
                    };
                    let bits = endpoint_to_bsp(&situ);
                    points.push(make_third_point(retest.source_index, bits, retest.price, c));
                }
            }
            // 3 卖：向下离开（leave 端点 < zd）+ 向上回抽（retest 高点 < zd，不触及闭区间中枢）。
            // 严格 `<`：retest==zd=单点重叠仍触及中枢 [ZD,ZG]=非终结=非第三类（codex 裁决 2026-06-27）。
            (Direction::Down, Direction::Up) => {
                if leave.price < c.zd && retest.price < c.zd {
                    let situ = EndpointSituation {
                        after_first_buy: false,
                        is_pullback_end: false,
                        left_center: true,
                        retrace_not_reenter: true,
                        below_last_center: false,
                        is_sell_side: true,
                    };
                    let bits = endpoint_to_bsp(&situ);
                    points.push(make_third_point(retest.source_index, bits, retest.price, c));
                }
            }
            // 同向相邻（无回试）/其他：非第三类结构，跳过。
            _ => {}
        }
    }
    points
}

/// 构造第一类 BspPoint（结构止损价 = pivot 极值，reference:46；center 留 None——1 类止损用 pivot）。
///
/// 1/2 类止损取 `pivot_low`(买)/`pivot_high`(卖)（破中枢段端点极值），非 center.zg/zd（那是 3 类）。
/// 故第一类 center=None（1 类不用 center 止损，与 3 类区分）。pivot 按 bit 方向填，另一侧 0。
fn make_first_point(source_index: usize, bits: BspBits, pivot_price: Tick) -> BspPoint {
    BspPoint {
        source_index,
        bits,
        // 1 买止损源 pivot_low（破中枢低点）；1 卖止损源 pivot_high（破中枢高点）。
        pivot_low: if bits.buy1 { pivot_price } else { 0 },
        pivot_high: if bits.sell1 { pivot_price } else { 0 },
        center: None,
    }
}

/// 构造第三类 BspPoint（结构止损价 single source）。
///
/// 3 类止损取 `center.zg`(买)/`zd`(卖)，故 center 必 `Some`（不变量：含 3 类 bit ⟹ center
/// 有值）。pivot_low/pivot_high 取回试端点价（买点回试低点 = pivot_low，卖点 = pivot_high）。
fn make_third_point(source_index: usize, bits: BspBits, retest_price: Tick, c: &Center) -> BspPoint {
    BspPoint {
        source_index,
        bits,
        // 买点回试低点 → pivot_low；卖点回抽高点 → pivot_high。按 bit 方向填，另一侧 0。
        pivot_low: if bits.buy3 { retest_price } else { 0 },
        pivot_high: if bits.sell3 { retest_price } else { 0 },
        center: Some(*c),
    }
}

/// 构造第二类 BspPoint（结构止损价 = 回拉走势结束点，reference:46 1/2 类止损 = pivot）。
///
/// 第二类止损（reference:46，与第一类同列）：1/2 买止损 = `pivot_low`，1/2 卖止损 = `pivot_high`
/// ——非 `center.zg/zd`（那是第三类）。故第二类 center=None（与第三类区分，对齐 `make_first_point`
/// 1/2 类止损不用 center 的不变量）。`second_point` = `SecondTypeStructure.second_point`（回拉走势
/// 结束点：买侧 = 回拉低点 m2.lo / 卖侧 = 回拉高点 m2.hi，§10.1），按 bit 方向填 pivot，另一侧 0。
fn make_second_point(source_index: usize, bits: BspBits, second_point: Tick) -> BspPoint {
    BspPoint {
        source_index,
        bits,
        // 2 买止损源 pivot_low（回拉低点）；2 卖止损源 pivot_high（回拉高点）。
        pivot_low: if bits.buy2 { second_point } else { 0 },
        pivot_high: if bits.sell2 { second_point } else { 0 },
        center: None,
    }
}

/// 第二类买卖点提取（递归组装层入口，契约锚 `Origin.RMoveCompose.SecondTypeStructure` +
/// `Origin.BspClassification.IsType2`；买卖点定律一 §10.2 + 第14/15课）。
///
/// ★与 `extract_signals`（L0 segment 层）平行的**递归组装层提取入口**：`extract_signals` 入参是
/// L0 线段（递归底，无次级别走势对象 ⟹ 不可产第二类）；本函数入参是 **RMove 递归塔** `parent`
/// （`RMove::Compose`，携 `descend parent` = 次级别走势序列），故**能**产第二类——第二类 =
/// 次级别第一类构成（§10.2「任何级别的第二类由次级别相应走势的第一类构成」，第14课买点定律一）。
///
/// 语义（消费 `find_second_type_structure` 识别的第二类走势结构）：
/// - **B2**（side=Long）：第一类离开走势后，次级别**回拉再次下跌不创新低**（第15课「未创新低」）的
///   回拉走势结束点（§10.1「第一类买点后次级别上涨结束、再次下跌的那个次级别走势的结束点」）。
/// - **S2**（side=Short）：镜像——第一类离开后回抽**不创新高**的回抽走势结束点。
///
/// `find_second_type_structure(parent, side, center_of, divergence_of)` 在 `descend parent` 内识别
/// `SecondTypeStructure { i1, i2, second_point }`（i1=第一类离开，i2=回拉，i1<i2 时间序；second_point=
/// 回拉走势结束点）。识别出 ⟹ 产一个 B2/S2 端点（`is_second = after_first_buy ∧ is_pullback_end`，
/// 对齐 `IsType2 = afterTypeOne ∧ ¬brokeCenter`：回拉走势未再破中枢 = `is_pullback_end`）；否则空。
///
/// ★坐标 still-MISSING（见模块头）：`RMove`（port Lean `Move` μF）无 source_index——`BspPoint.source_index`
/// 由 `index_of` 闭包提供（次级别走势 → 原始 K 序，上游塔构造时填，与 `center_of`/`divergence_of` 同
/// 精神）。本函数不冒充自动坐标分配（在 RMove 上硬造 source_index = 声明膨胀）。
///
/// `c1`：第二类的次级别第一类离开走势所破的中枢（B 口径核心区间 [zd,zg]，迁主塔 637 定稿；
/// `center_of` 给次级别每走势配中枢，本入口对识别出的第一类离开走势用 `c1` 统一，同 descend.rs 边界）。
/// `divergence_of`：每个次级别走势的 MACD 背驰判定（rust 真算，divergence.rs；力度领先 Origin）。
/// `index_of`：每个次级别走势 → 原始 K 序号（坐标 still-MISSING，上游塔提供）。
pub fn extract_second_signals(
    parent: &RMove,
    side: Side,
    c1: &Center,
    divergence_of: impl Fn(&RMove) -> bool,
    index_of: impl Fn(&RMove) -> usize,
) -> Vec<BspPoint> {
    // 在 RMove 递归塔的次级别走势序列内识别第二类走势结构（第一类离开 + 回拉不创新低/新高 + i1<i2）。
    // center_of 对识别用的第一类离开走势统一配 c1（次级别中枢，B 口径核心区间，上游塔提供）。
    let Some(structure) = find_second_type_structure(parent, side, |_m| *c1, &divergence_of) else {
        // 无第二类走势结构（无第一类离开 / 无回拉 / 回拉创新低或新高）⟹ 无 B2/S2（诚实空）。
        return Vec::new();
    };
    let subs = super::descend::descend(parent);
    // 回拉走势 m2（结构识别出的 i2 位置）——第二类买卖点 = 其结束点 = structure.second_point。
    let m2 = &subs[structure.i2];
    let is_sell = matches!(side, Side::Short);
    // 第二类端点语义：after_first_buy（第一类离开在前，IsType2 的 afterTypeOne）∧ is_pullback_end
    //（回拉走势结束点，回拉未再破中枢 = IsType2 的 ¬brokeCenter）。
    let situ = EndpointSituation {
        after_first_buy: true,      // 第一类离开走势在前（structure.i1 < i2，§10.1「第一类后」）
        is_pullback_end: true,      // 回拉走势结束点（回拉不创新低/新高，第15课）
        left_center: false,         // 第二类是中枢内部回拉，非第三类的离开后回抽
        retrace_not_reenter: false,
        below_last_center: false,   // 第二类非第一类的破中枢背驰端点
        is_sell_side: is_sell,
    };
    let bits = endpoint_to_bsp(&situ);
    // source_index 由 index_of 取回拉走势 m2 的原始 K 序（坐标 still-MISSING，上游塔提供）。
    vec![make_second_point(index_of(m2), bits, structure.second_point)]
}

/// 从 confirmed 中枢序列 + 线段序列 + close 序列提取该级别全部买卖点（reference:34-36）。
///
/// 对每个中枢，取其 `end_index` 之后的线段子序列，提取**第一类**（破中枢 ∧ MACD 背驰真算）+
/// **第三类**（离开后回试不破）买卖点。买卖点按 source_index 升序返回（reference:16 平局裁决——
/// 已确认结构不回写，时间序天然升序）。
///
/// ★第二类（B2/S2）**不在本函数产出**——本函数入参是 L0 线段（递归底，无次级别走势对象 ⟹ 结构
/// 上不可产第二类，签名层边界，见模块头）。B2/S2 由平行的递归组装层入口 [`extract_second_signals`]
/// 产（消费 RMove 递归塔的 `SecondTypeStructure`）——按输入对象分工，非本函数缺口，零冒充。
///
/// `closes`：close 序列（`ParseLayer.merged_bars` 的 close，MACD 算第一类背驰用）。
/// `close_src`：closes 各元素的 source_index（merged_bars 锚点，段区间坐标系转换用）。
/// `macd_cfg`：MACD 参数（fast/slow/signal，从 ThetaConfig.macd 读）。
///
/// ★本函数覆盖：B1/S1（破中枢 ∧ 背驰真算）+ B3/S3（confirmed 结构几何）。
/// B2/S2 见 [`extract_second_signals`]（递归组装层）。
pub fn extract_signals(
    centers: &[Center],
    segments: &[Segment],
    closes: &[f64],
    close_src: &[usize],
    macd_cfg: &MacdConfig,
) -> Vec<BspPoint> {
    // MACD hist（第一类背驰真算，浮点域隔离在 divergence.rs）。空 closes ⟹ 空 hist ⟹ 第一类不产
    // （段无法映射 closes 区间），第三类仍正常产（纯整数几何，不依赖 MACD）。
    let hist = compute_macd(closes, macd_cfg).hist;

    let mut points = Vec::new();
    for c in centers {
        // 中枢之后的线段（start_index >= 中枢 end_index 的线段——离开+回试/破中枢发生在中枢后）。
        let segs_after: Vec<Segment> = segments
            .iter()
            .filter(|s| s.start_index >= c.end_index)
            .copied()
            .collect();
        // 第一类（破中枢 ∧ MACD 背驰真算）。
        points.extend(extract_first_for_center(c, &segs_after, &hist, close_src));
        // 第三类（离开后回试不破，纯整数几何）。
        points.extend(extract_third_for_center(c, &segs_after));
    }
    // 按 source_index 升序（reference:16 平局裁决键的时间序分量）。
    points.sort_by_key(|p| p.source_index);
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    fn center(zd: Tick, zg: Tick, end_index: usize) -> Center {
        Center { zd, zg, dd: zd - 5, gg: zg + 5, start_index: 0, end_index }
    }

    fn seg(dir: Direction, si: usize, ei: usize, sp: Tick, ep: Tick) -> Segment {
        Segment { direction: dir, start_index: si, end_index: ei, start_price: sp, end_price: ep }
    }

    /// closes + close_src 辅助：连续 source_index 0..n（无合并跳跃的简单坐标系）。
    fn closes_seq(vals: &[Tick]) -> (Vec<f64>, Vec<usize>) {
        let c: Vec<f64> = vals.iter().map(|&v| v as f64).collect();
        let src: Vec<usize> = (0..vals.len()).collect();
        (c, src)
    }

    /// 第三类专用入口（无 MACD 依赖——传空 closes，第一类自然不产）。
    fn extract_third_only(centers: &[Center], segs: &[Segment]) -> Vec<BspPoint> {
        extract_signals(centers, segs, &[], &[], &MacdConfig::default())
    }

    // ── 第三类（保留原判据，MACD 无关）────────────────────────────────────────

    #[test]
    fn third_buy_extracted_bit_exact() {
        // 中枢核心 [100,200] end_index=12。后续：向上线段离开（端点 250 > zg=200），
        // 向下回试低点 210 > zg=200（不入中枢）⟹ 3 买。
        let c = center(100, 200, 12);
        let segs = vec![
            seg(Direction::Up, 12, 16, 150, 250),   // 离开：端点 250 > 200
            seg(Direction::Down, 16, 20, 250, 210), // 回试：低点 210 > 200 → 3 买
        ];
        let points = extract_third_only(&[c], &segs);
        assert_eq!(points.len(), 1);
        assert!(points[0].bits.buy3);
        assert_eq!(points[0].source_index, 20); // 回试端点
        assert_eq!(points[0].pivot_low, 210);   // 回试低点 = pivot_low
        assert_eq!(points[0].center.map(|c| c.zg), Some(200)); // 3 买止损 = zg
    }

    #[test]
    fn third_buy_rejected_when_retest_reenters() {
        // 回试低点 190 < zg=200（重新跌破进入中枢）⟹ 不是 3 买。
        let c = center(100, 200, 12);
        let segs = vec![
            seg(Direction::Up, 12, 16, 150, 250),
            seg(Direction::Down, 16, 20, 250, 190), // 回试 190 < 200 → 入中枢，非 3 买
        ];
        let points = extract_third_only(&[c], &segs);
        assert!(points.is_empty());
    }

    #[test]
    fn third_buy_boundary_retest_eq_zg_not_third() {
        // 回试低点 == zg=200（边界临界点）：严格口径 `> zg` ⟹ 非第三类买点。
        // retest==zg=单点重叠仍触及闭区间中枢 [ZD,ZG]=中枢延伸（中心定理一）=非终结=非第三类。
        // 逐字段对齐 Lean IsType3Buy（zg < retracePrice，严格）。codex 裁决 2026-06-27。
        let c = center(100, 200, 12);
        let segs = vec![
            seg(Direction::Up, 12, 16, 150, 250),
            seg(Direction::Down, 16, 20, 250, 200), // 回试 == zg → 触及中枢 → 非 3 买（严格排除等号）
        ];
        let points = extract_third_only(&[c], &segs);
        assert!(points.is_empty(), "retest==zg 触及闭区间中枢=非终结=非第三类（严格口径，对齐 Lean）");
    }

    #[test]
    fn third_sell_extracted_bit_exact() {
        // 镜像：向下离开（端点 50 < zd=100），向上回抽高点 90 < zd=100 ⟹ 3 卖。
        let c = center(100, 200, 12);
        let segs = vec![
            seg(Direction::Down, 12, 16, 150, 50),  // 离开：端点 50 < 100
            seg(Direction::Up, 16, 20, 50, 90),     // 回抽：高点 90 < 100 → 3 卖
        ];
        let points = extract_third_only(&[c], &segs);
        assert_eq!(points.len(), 1);
        assert!(points[0].bits.sell3);
        assert_eq!(points[0].pivot_high, 90);   // 回抽高点 = pivot_high
        assert_eq!(points[0].center.map(|c| c.zd), Some(100)); // 3 卖止损 = zd
    }

    #[test]
    fn no_leave_no_third_signal() {
        // 线段未离开中枢（端点 180 < zg=200）⟹ 无第三类（且方向不破中枢，无第一类）。
        let c = center(100, 200, 12);
        let segs = vec![
            seg(Direction::Up, 12, 16, 150, 180),   // 未离开（180 < 200）
            seg(Direction::Down, 16, 20, 180, 160), // 端点 160 ∈ [100,200]，未破中枢下沿
        ];
        let points = extract_third_only(&[c], &segs);
        assert!(points.is_empty());
    }

    #[test]
    fn third_buy_has_center_invariant() {
        // 不变量：含 3 类 bit ⟹ center 必 Some（strategy 3 类止损 unwrap 安全）。
        let c = center(100, 200, 12);
        let segs = vec![
            seg(Direction::Up, 12, 16, 150, 250),
            seg(Direction::Down, 16, 20, 250, 210),
        ];
        let points = extract_third_only(&[c], &segs);
        for p in &points {
            if p.bits.buy3 || p.bits.sell3 {
                assert!(p.center.is_some(), "含 3 类 bit ⟹ center 必 Some");
            }
        }
    }

    #[test]
    fn empty_centers_no_signal() {
        let points = extract_third_only(&[], &[seg(Direction::Up, 0, 4, 0, 10)]);
        assert!(points.is_empty());
    }

    // ── 第一类（破中枢几何 L0 ∧ MACD 背驰真算 L1）────────────────────────────

    #[test]
    fn first_buy_extracted_with_divergence() {
        // 中枢核心 [100,200] end_index=2。后续两条向下段：
        // - 前向下段 [3,5]：端点 90（已破中枢下沿 < zd=100），MACD 面积大（强势）。
        // - 后向下段 [6,8]：端点 80（破中枢更深 < zd=100），MACD 面积小（背驰，力度衰减）⟹ 1 买。
        let c = center(100, 200, 2);
        let segs = vec![
            seg(Direction::Down, 3, 5, 150, 90),  // 前向下段（破中枢，作背驰对照）
            seg(Direction::Down, 6, 8, 120, 80),  // 后向下段（破中枢 ∧ 面积更小 → 背驰）
        ];
        // closes 构造：前段 bar [3,5] 波动大（|hist| 大），后段 bar [6,8] 波动小（|hist| 小）。
        // 用价格序列让 MACD hist 在前段绝对值大于后段（背驰）。
        let prices: Vec<Tick> = vec![
            100, 100, 100,        // 0..2（中枢区，预热）
            100, 60, 140,         // 3..5 前段：大幅震荡 ⟹ hist 绝对值大
            100, 95, 105,         // 6..8 后段：小幅震荡 ⟹ hist 绝对值小（力度衰减）
        ];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&[c], &segs, &closes, &src, &MacdConfig::default());
        // 至少后段（6,8）应识别为 1 买（破中枢 ∧ 面积严格小于前段）。
        let buy1: Vec<_> = points.iter().filter(|p| p.bits.buy1).collect();
        assert_eq!(buy1.len(), 1, "后向下段破中枢 ∧ 背驰 ⟹ 一个 1 买");
        assert_eq!(buy1[0].source_index, 8, "1 买端点 = 后破中枢段终止 source_index");
        assert_eq!(buy1[0].pivot_low, 80, "1 买止损源 = pivot_low（破中枢段端点）");
        assert!(buy1[0].center.is_none(), "1 类止损用 pivot 非 center ⟹ center=None");
    }

    #[test]
    fn first_buy_rejected_without_divergence() {
        // 破中枢但**力度未衰减**（后段面积 >= 前段）⟹ 非背驰 ⟹ 非第一类。
        let c = center(100, 200, 2);
        let segs = vec![
            seg(Direction::Down, 3, 5, 150, 90),
            seg(Direction::Down, 6, 8, 120, 80),
        ];
        // closes：前段小波动、后段大波动 ⟹ 后段面积 > 前段 ⟹ 力度延续，非背驰。
        let prices: Vec<Tick> = vec![
            100, 100, 100,
            100, 98, 102,         // 3..5 前段：小幅（hist 小）
            100, 50, 150,         // 6..8 后段：大幅（hist 大 ⟹ 力度延续 ⟹ 非背驰）
        ];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&[c], &segs, &closes, &src, &MacdConfig::default());
        assert!(points.iter().all(|p| !p.bits.buy1), "破中枢但力度延续 ⟹ 非第一类");
    }

    #[test]
    fn first_buy_rejected_without_broke_center() {
        // 向下段未破中枢下沿（端点 120 ∈ [100,200]）⟹ 非第一类（几何不足，无论背驰与否）。
        let c = center(100, 200, 2);
        let segs = vec![
            seg(Direction::Down, 3, 5, 150, 130),
            seg(Direction::Down, 6, 8, 140, 120), // 端点 120 >= zd=100，未破中枢
        ];
        let prices: Vec<Tick> = vec![100, 100, 100, 100, 60, 140, 100, 98, 102];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&[c], &segs, &closes, &src, &MacdConfig::default());
        assert!(points.iter().all(|p| !p.bits.buy1), "未破中枢 ⟹ 非第一类（几何分量不足）");
    }

    #[test]
    fn first_buy_rejected_without_prior_same_dir_segment() {
        // 破中枢段是序列**首个**向下段（无前同向段对照）⟹ 无背驰对照标的 ⟹ 非第一类。
        let c = center(100, 200, 2);
        let segs = vec![
            seg(Direction::Up, 3, 5, 100, 150),   // 向上段（非买点方向）
            seg(Direction::Down, 6, 8, 150, 80),  // 首个向下段（破中枢，但无前向下段对照）
        ];
        let prices: Vec<Tick> = vec![100, 100, 100, 100, 130, 170, 100, 95, 60];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&[c], &segs, &closes, &src, &MacdConfig::default());
        assert!(points.iter().all(|p| !p.bits.buy1), "无前同向段 ⟹ 无背驰对照 ⟹ 非第一类");
    }

    #[test]
    fn first_sell_mirror_with_divergence() {
        // 卖镜像：两条向上段突破中枢上沿（> zg=200），后段 MACD 面积严格小于前段 ⟹ 1 卖。
        let c = center(100, 200, 2);
        let segs = vec![
            seg(Direction::Up, 3, 5, 150, 210),  // 前向上段（破中枢上沿）
            seg(Direction::Up, 6, 8, 180, 220),  // 后向上段（破中枢 ∧ 背驰）
        ];
        let prices: Vec<Tick> = vec![
            200, 200, 200,
            200, 260, 140,        // 3..5 前段：大幅
            200, 205, 195,        // 6..8 后段：小幅（背驰）
        ];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&[c], &segs, &closes, &src, &MacdConfig::default());
        let sell1: Vec<_> = points.iter().filter(|p| p.bits.sell1).collect();
        assert_eq!(sell1.len(), 1, "后向上段破中枢上沿 ∧ 背驰 ⟹ 一个 1 卖");
        assert_eq!(sell1[0].pivot_high, 220, "1 卖止损源 = pivot_high");
        assert!(sell1[0].center.is_none(), "1 类止损用 pivot 非 center");
    }

    // ── 第二类 L0 签名层边界（extract_signals 不产；B2 由递归组装层入口产）──────

    #[test]
    fn signal_extraction_emits_no_second_class() {
        // ★L0 签名层边界（编码为可执行断言，formalization-validity-domain）：extract_signals 对任意
        // L0 segment 输入**永不产第二类**——第二类=次级别第一类构成（买卖点定律一），需 RMove 递归
        // 结构（descend.rs），L0 segment 是递归底（descend 得空）⟹ 结构上不可在本签名层产出。
        // 此断言锁定边界：B2/S2 由平行的递归组装层入口 `extract_second_signals` 产（消费 RMove 塔），
        // 非 extract_signals 缺口——按输入对象分工（L0 segment vs RMove 递归塔），见模块头。
        let c = center(100, 200, 2);
        let segs = vec![
            seg(Direction::Down, 3, 5, 150, 90),
            seg(Direction::Down, 6, 8, 120, 80),
            seg(Direction::Up, 9, 11, 80, 250),
            seg(Direction::Down, 12, 14, 250, 210),
        ];
        let prices: Vec<Tick> = vec![
            100, 100, 100, 100, 60, 140, 100, 95, 105,
            100, 175, 250, 250, 230, 210,
        ];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&[c], &segs, &closes, &src, &MacdConfig::default());
        for p in &points {
            assert!(!p.bits.buy2, "extract_signals（L0 层）不产第二类买点——B2 由 extract_second_signals 递归组装层产");
            assert!(!p.bits.sell2, "extract_signals（L0 层）不产第二类卖点——S2 由 extract_second_signals 递归组装层产");
        }
    }

    // ── 第二类递归组装层提取（extract_second_signals 消费 RMove 塔产 B2/S2）────────

    use super::super::descend::RMove;
    use super::super::rmove_compose::compose_move;

    /// 第二类结构的次级别中枢（B 口径核心区间 [zd,zg]=[0,4]，对齐 rmove_compose 见证 c1_wit）。
    fn second_center() -> Center {
        Center { zd: 0, zg: 4, dd: -2, gg: 6, start_index: 0, end_index: 0 }
    }

    /// 第一类离开走势（向下破中枢，区间 [-10,-2]，lo=-10 < zd=0 ⟹ 破中枢；对齐 rmove_compose m1_wit）。
    fn m1_break() -> RMove {
        RMove::Segment { direction: Direction::Down, lo: -10, hi: -2 }
    }

    /// 回拉走势（再次下跌不创新低，区间 [-8,3]，lo=-8 ≥ m1.lo=-10 ⟹ 不创新低；对齐 m2_wit）。
    fn m2_pullback() -> RMove {
        RMove::Segment { direction: Direction::Up, lo: -8, hi: 3 }
    }

    /// 收尾走势（凑满 ≥3 段；对齐 m3_wit）。
    fn m3_tail() -> RMove {
        RMove::Segment { direction: Direction::Up, lo: 1, hi: 5 }
    }

    /// 本级别走势塔：RMove::Compose 组装第一类离开 + 回拉 + 收尾。
    fn parent_tower() -> RMove {
        compose_move(vec![m1_break(), m2_pullback(), m3_tail()], vec![second_center()], 1)
    }

    #[test]
    fn second_buy_extracted_from_rmove_tower() {
        // ★递归组装层 B2 提取：RMove 塔含第一类离开（破中枢 ∧ 背驰）+ 回拉不创新低 ⟹ 一个 B2。
        // divergence_of：仅第一类离开 m1（lo=-10）背驰，回拉/收尾不背驰（rust MACD 真算的占位见证）。
        // index_of：回拉走势 m2 → 原始 K 序 42（坐标 still-MISSING，上游塔提供）。
        let parent = parent_tower();
        let points = extract_second_signals(
            &parent,
            Side::Long,
            &second_center(),
            |m| m.lo() == -10,        // 仅第一类离开背驰
            |m| if m.lo() == -8 { 42 } else { 0 }, // 回拉走势 m2 的原始 K 序
        );
        assert_eq!(points.len(), 1, "RMove 塔（第一类离开 ∧ 回拉不创新低）⟹ 一个 B2");
        assert!(points[0].bits.buy2, "递归组装层产第二类买点（buy2 置位）");
        assert!(!points[0].bits.buy1 && !points[0].bits.buy3, "第二类端点不置 1/3 类（互斥语义）");
        assert_eq!(points[0].source_index, 42, "B2 source_index = 回拉走势 m2 的原始 K 序（index_of）");
        assert_eq!(points[0].pivot_low, -8, "B2 止损源 = 回拉低点（second_point = m2.lo）");
        assert!(points[0].center.is_none(), "1/2 类止损用 pivot 非 center ⟹ center=None");
    }

    #[test]
    fn second_buy_rejected_when_retrace_makes_new_low() {
        // 回拉创新低（lo=-12 < m1.lo=-10）⟹ 破前低 ⟹ 非第二类（第15课「未创新低」是真约束）。
        let m2_break = RMove::Segment { direction: Direction::Up, lo: -12, hi: 3 };
        let parent = compose_move(vec![m1_break(), m2_break], vec![second_center()], 1);
        let points = extract_second_signals(
            &parent,
            Side::Long,
            &second_center(),
            |m| m.lo() == -10,
            |_m| 0,
        );
        assert!(points.is_empty(), "回拉创新低 ⟹ 非第二类（无 B2）");
    }

    #[test]
    fn second_buy_rejected_without_divergence() {
        // 第一类离开破中枢但**不背驰**（divergence_of 全 false）⟹ 无完整第一类 ⟹ 无第二类结构。
        let parent = parent_tower();
        let points = extract_second_signals(
            &parent,
            Side::Long,
            &second_center(),
            |_m| false, // 力度全 false：无背驰
            |_m| 0,
        );
        assert!(points.is_empty(), "破中枢但不背驰 ⟹ 非次级别第一类 ⟹ 无第二类");
    }

    #[test]
    fn second_sell_mirror_from_rmove_tower() {
        // S2 镜像：第一类向上离开探顶（破中枢上沿）+ 回抽不创新高 ⟹ 一个 S2（止损 = 回抽高点）。
        // 第一类离开 [2,10]（hi=10 > zg=4 ⟹ 向上破中枢），回抽 [1,8]（hi=8 ≤ m1.hi=10 ⟹ 不创新高）。
        let m1_sell = RMove::Segment { direction: Direction::Up, lo: 2, hi: 10 };
        let m2_sell = RMove::Segment { direction: Direction::Down, lo: 1, hi: 8 };
        let m3_sell = RMove::Segment { direction: Direction::Down, lo: 0, hi: 6 };
        let parent = compose_move(vec![m1_sell, m2_sell, m3_sell], vec![second_center()], 1);
        let points = extract_second_signals(
            &parent,
            Side::Short,
            &second_center(),
            |m| m.hi() == 10,         // 仅第一类离开背驰
            |m| if m.hi() == 8 { 17 } else { 0 }, // 回抽走势的原始 K 序
        );
        assert_eq!(points.len(), 1, "S2：第一类离开 ∧ 回抽不创新高 ⟹ 一个 S2");
        assert!(points[0].bits.sell2, "递归组装层产第二类卖点（sell2 置位）");
        assert!(!points[0].bits.buy2, "卖侧 ⟹ buy2 不置位");
        assert_eq!(points[0].source_index, 17, "S2 source_index = 回抽走势的原始 K 序");
        assert_eq!(points[0].pivot_high, 8, "S2 止损源 = 回抽高点（second_point = m2.hi）");
        assert!(points[0].center.is_none(), "1/2 类止损用 pivot 非 center");
    }

    #[test]
    fn second_signals_empty_for_l0_segment() {
        // 递归底：RMove::Segment（L0 线段）descend 得空 ⟹ 无次级别走势 ⟹ 无第二类（对齐
        // rmove_compose segment_no_second_type）。递归组装层入口在递归底诚实空，非崩溃。
        let seg = RMove::Segment { direction: Direction::Down, lo: -10, hi: -2 };
        let points = extract_second_signals(&seg, Side::Long, &second_center(), |_m| true, |_m| 0);
        assert!(points.is_empty(), "L0 线段（递归底）⟹ 无第二类结构 ⟹ 无 B2");
    }

    // ── 坐标系映射（source_index → closes 下标）─────────────────────────────

    #[test]
    fn src_range_maps_with_merged_bar_gaps() {
        // merged_bars 有合并跳跃：source_index = [0,2,5,9]（下标 0..3）。
        // 段 source_index [2,9] 应映射到 closes 下标 [1,3]（src=2 在下标1，src=9 在下标3）。
        let src_to_idx = vec![0usize, 2, 5, 9];
        assert_eq!(map_src_range_to_close_idx(&src_to_idx, 2, 9), Some((1, 3)));
        // 段 [3,6]：首个 >=3 是 src=5（下标2），末个 <=6 是 src=5（下标2）⟹ (2,2)。
        assert_eq!(map_src_range_to_close_idx(&src_to_idx, 3, 6), Some((2, 2)));
        // 段 [10,20]：无 source_index 落入 ⟹ None。
        assert_eq!(map_src_range_to_close_idx(&src_to_idx, 10, 20), None);
    }
}
