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
//! - **RMove 递归塔生产路径已接入**（still-MISSING-塔已解除，#53 升级）：`mod.rs::classify` 已把递归塔
//!   走势单元从 `UnitRange` 升级为携 subs 的 `RMove::Compose`（`recursive_tower.rs::LeveledMove`+source_index
//!   坐标侧车），`extract_second_signals`/`extract_second_for_level` 零改动真接入生产路径——端到端产 B2
//!   （`mod.rs::tests::end_to_end_second_buy_via_l1_l2_geometric`）。诚实边界（still-MISSING-窗口，codex 裁决）：
//!   B2/S2 只在 L1→L2 几何路径产，L0→L1 三段交替窗口结构上界（可背驰同向段仅位置2，非接入缺陷），
//!   见 `mod.rs::l0_level_emits_no_second_class_window_bound`。
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
/// `src_to_idx` 是 merged_bars 下标 → source_index 的**升序**映射（`closes[k]` 对应 `src_to_idx[k]`；
/// merged_bars source_index 单调递增，见函数头）。找首个 `>= start` 的下标 lo 与末个 `<= end` 的
/// 下标 hi。区间空（无 bar 落入）⟹ None。
///
/// ★性能（O(logS) 二分，原 O(S) 线性 `position`/`rposition`）：`src_to_idx` 升序 ⟹ 谓词 `s >= start`
/// 与 `s <= end` 都是单调前缀/后缀，用 `partition_point` 二分定位（替代每次调用 O(S) 全扫）。type1
/// 路径每段调本函数 2 次（curr + prev），原 O(S) 线性扫使 type1 退化为 O(S²)——这是 L2 profile 坐实
/// 的真 O(n²) 热点之一（与 type3 前驱重复 O(C·S) 并列）；二分后降为 O(S·logS)。
fn map_src_range_to_close_idx(
    src_to_idx: &[usize],
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    if start > end {
        return None;
    }
    // lo = 首个 source_index >= start 的 closes 下标（升序 ⟹ `s < start` 是前缀，二分其长度）。
    let lo = src_to_idx.partition_point(|&s| s < start);
    if lo >= src_to_idx.len() {
        return None; // 无 source_index >= start（全部 < start）。
    }
    // hi = 末个 source_index <= end 的 closes 下标（升序 ⟹ `s <= end` 是前缀，末元素 = 前缀长 - 1）。
    let cnt_le_end = src_to_idx.partition_point(|&s| s <= end);
    if cnt_le_end == 0 {
        return None; // 无 source_index <= end（全部 > end）。
    }
    let hi = cnt_le_end - 1;
    if lo > hi {
        return None;
    }
    Some((lo, hi))
}

/// 某线段归属的「最近已确认中枢」（第18课定理三「该中枢」+ 第49课「当下之前最后一个中枢」）。
///
/// ★中枢归属语义（codex 异质裁决 2026-06-27，原文坐实）：第一/三类买卖点针对的中枢是该端点
/// **刚离开并首次回试的那个中枢**——即该线段在原始 K 序上**之前最后一个已确认中枢**（不是所有
/// 价格阈值更低/更高的前驱中枢）。原文依据：
/// - 第18课定理三（018:64）：「中枢破坏 = 离开**该**中枢后回抽不重新回到**该**中枢内」——单数「该中枢」。
/// - 第20课定理（020:60）：「离开缠中说禅走势中枢...回试...必须是**第一次**」——单数中枢 + 第一次。
/// - 第49课/第29课：「当下之前最后一个中枢」组织买卖点（`bsp.rs:110` BspPoint.center 注「最后中枢」）。
/// - maimai.md:103：第一类「向下跌破**最后一个**中枢」。
/// - Lean `BspClassification.BspEndpoint`（lean:75-84）只带**单一** center，`IsType3Buy/Sell` 相对该
///   **一个** center 判定（lean:111-121）。
///
/// `centers` 按 `end_index` 升序（`detect_centers_with` 非重叠扫描保证，mod.rs:136）。返回 `end_index
/// <= seg_start` 的**最后一个**中枢（该线段离开/回试时「当下之前最后一个中枢」）。无满足者 ⟹ None
/// （线段在所有中枢之前 ⟹ 无可离开/回试的中枢 ⟹ 非第一/三类）。
fn nearest_confirmed_center(centers: &[Center], seg_start: usize) -> Option<&Center> {
    // 升序 centers 上，end_index <= seg_start 的是一个前缀；取该前缀末元素 = 最近中枢。
    let hi = centers.partition_point(|c| c.end_index <= seg_start);
    if hi == 0 {
        None
    } else {
        Some(&centers[hi - 1])
    }
}

/// 第一类买卖点判定（契约锚 `Origin.BspClassification.IsType1 = brokeCenter ∧ IsDivergence`）。
///
/// reference:34 + maimai.md:103「某级别趋势中，次级别向下跌破**最后一个**中枢后形成的**背驰点**」。
/// 对单个线段 `seg`（已归属其**最近中枢** `c`，见 [`nearest_confirmed_center`]）+ 前一同向段 `prev`
/// 判破中枢 ∧ 背驰：
/// - **1买**：向下线段端点 `< c.zd`（破最近中枢下沿，`brokeCenter` 几何 L0）∧ 相对前向下段 MACD
///   面积严格变小（背驰，`segments_diverge` 真算 L1）⟹ 1 买。
/// - **1卖**：镜像——向上线段端点 `> c.zg`，相对前向上段面积严格变小 ⟹ 1 卖。
///
/// ★中枢归属（codex 裁决）：`c` 是 `seg` 的**最近中枢**（"最后一个中枢"），非所有 zd > 端点的前驱
/// 中枢——每个破中枢段端点只相对**一个**中枢判一次（消解旧 `for c in centers` 对前驱重复产出）。
///
/// ★分量 L 级（formalization-validity-domain）：破中枢 `< zd`/`> zg` 整数几何 **L0**；背驰 MACD
/// 真算 **L1**。两者合取 = `IsType1`（对齐 Lean，bit-exact）。「前一同向段」是序列序最近同向前驱
/// （reference:34「末段相对前同向段」的确定配对）。`prev` 无（首个同向段）⟹ 无背驰对照 ⟹ None。
///
/// `hist` 是 MACD hist 序列；`src_to_idx` 是 closes 下标→source_index 映射。段无法映射到 closes
/// 区间（越界）⟹ None（无面积 ⟹ 非背驰）。
fn judge_first(
    c: &Center,
    seg: &Segment,
    prev: &Segment,
    hist: &[f64],
    src_to_idx: &[usize],
) -> Option<BspPoint> {
    let end = seg_end(seg);
    // 破中枢几何（L0）：买侧向下破（端点 < zd）；卖侧向上破（端点 > zg）。
    let (broke, is_sell) = match end.dir {
        Direction::Down if end.price < c.zd => (true, false), // 1 买：向下破最近中枢下沿
        Direction::Up if c.zg < end.price => (true, true),    // 1 卖：向上破最近中枢上沿
        _ => (false, false),
    };
    if !broke {
        return None;
    }
    // 段区间（source_index）→ closes 下标区间（MACD 面积坐标系）。
    let (Some(curr_seg), Some(prev_seg)) = (
        map_src_range_to_close_idx(src_to_idx, seg.start_index, seg.end_index),
        map_src_range_to_close_idx(src_to_idx, prev.start_index, prev.end_index),
    ) else {
        // 段无法映射到 closes 区间（越界/空）⟹ 无 MACD 面积 ⟹ 非背驰。
        return None;
    };
    // 背驰（L1 真算）：破中枢段（curr）面积严格小于前同向段（prev）面积。
    if !segments_diverge(hist, prev_seg, curr_seg) {
        return None; // 力度未衰减 ⟹ 非背驰 ⟹ 非第一类。
    }
    // 第一类端点：below_last_center（买）/对偶（卖），未离开中枢（破中枢 ≠ 离开后回抽）。
    let situ = EndpointSituation {
        after_first_buy: false,
        is_pullback_end: false,
        left_center: false,      // 第一类是破中枢背驰，非第三类的离开后回抽
        retrace_not_reenter: false,
        below_last_center: true, // 破中枢背驰端点（买=中枢下方/卖镜像）
        is_sell_side: is_sell,
    };
    let bits = endpoint_to_bsp(&situ);
    // 第一类止损 = pivot（破中枢段端点极值）：买点 pivot_low、卖点 pivot_high。
    Some(make_first_point(end.source_index, bits, end.price))
}

/// 第三类买卖点判定（契约锚 `Origin.BspClassification.IsType3Buy/IsType3Sell` 点位判据）。
///
/// 对相邻线段对 (leave, retest)（leave 已归属其**最近中枢** `c`，见 [`nearest_confirmed_center`]）
/// 按 reference:36 判第三类：
/// - **3买**：向上 leave 离开 `c` 上方（端点 `> c.zg`）+ 向下 retest 回试低点 `> c.zg`（不触及闭区间
///   中枢）⟹ 该回试低点 = 3 买。
/// - **3卖**：向下 leave 离开 `c` 下方（端点 `< c.zd`）+ 向上 retest 回抽高点 `< c.zd`（不触及闭区间
///   中枢）⟹ 该回抽高点 = 3 卖。
///
/// ★中枢归属（codex 裁决，第18课定理三「该中枢」）：`c` 是 leave 段刚离开的**最近中枢**——同一
/// retest 端点只相对**该一个**中枢判一次，不对所有 `zg < retest` 的前驱中枢重复产出（旧
/// `for c in centers` 的重复 bug 根源）。
///
/// bit-exact 边界：严格口径（`> zg` / `< zd`，等号排除）——retest==zg=单点重叠仍触及闭区间中枢
/// `[ZD,ZG]`（中心定理一：重叠=中枢延伸，非终结）⟹ 非第三类。逐字段对齐 Lean `IsType3Buy`
/// （`zg < retracePrice` 严格，BspClassification.lean:113）。codex 裁决 2026-06-27（中枢终结语义）。
fn judge_third(c: &Center, leave_seg: &Segment, retest_seg: &Segment) -> Option<BspPoint> {
    let leave = seg_end(leave_seg);
    let retest = seg_end(retest_seg);
    match (leave.dir, retest.dir) {
        // 3 买：向上离开（leave 端点 > c.zg）+ 向下回试（retest 低点 > c.zg，不触及闭区间中枢）。
        (Direction::Up, Direction::Down) if leave.price > c.zg && retest.price > c.zg => {
            let situ = EndpointSituation {
                after_first_buy: false,
                is_pullback_end: false,
                left_center: true,         // 离开最近中枢（leave.price > c.zg）
                retrace_not_reenter: true, // 回试不触及闭区间中枢（retest > c.zg，严格）
                below_last_center: false,
                is_sell_side: false,
            };
            let bits = endpoint_to_bsp(&situ);
            Some(make_third_point(retest.source_index, bits, retest.price, c))
        }
        // 3 卖：向下离开（leave 端点 < c.zd）+ 向上回抽（retest 高点 < c.zd，不触及闭区间中枢）。
        (Direction::Down, Direction::Up) if leave.price < c.zd && retest.price < c.zd => {
            let situ = EndpointSituation {
                after_first_buy: false,
                is_pullback_end: false,
                left_center: true,
                retrace_not_reenter: true,
                below_last_center: false,
                is_sell_side: true,
            };
            let bits = endpoint_to_bsp(&situ);
            Some(make_third_point(retest.source_index, bits, retest.price, c))
        }
        // 同向相邻（无回试）/破中枢方向不符/触及中枢：非第三类结构。
        _ => None,
    }
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

    // ★线段按 start_index **稳定**升序排一次（生产路径 parser 线段账本本已 start_index 严格单调
    // 递增——流式 push 时 seg_start 单调推进，segment.rs:344-380——故排序对生产路径是恒等）。稳定
    // 排序保证 start_index 相同时保留原序。中枢归属用 start_index，单趟扫描需线段时间序。
    let mut sorted: Vec<Segment> = segments.to_vec();
    sorted.sort_by_key(|s| s.start_index);

    // ★中枢归属用 centers 按 end_index 升序（`detect_centers_with` 非重叠扫描已保证，mod.rs:136；
    // 公开函数不假设入参有序 ⟹ 本入口稳定排序，`nearest_confirmed_center` 二分前缀依赖升序）。
    let mut centers_sorted: Vec<Center> = centers.to_vec();
    centers_sorted.sort_by_key(|c| c.end_index);

    // ★单趟扫描（消解旧 `for c in centers` 对前驱中枢重复产出，codex 裁决 2026-06-27）：每个线段端点
    // 只相对其**最近已确认中枢**（"当下之前最后一个中枢"，第18课定理三「该中枢」+ 第49课）判第一/三
    // 类**一次**，不对所有阈值更低/更高的前驱中枢重复认领。
    //
    // 复杂度 O(S·logC + S)：每段 `nearest_confirmed_center` 二分 O(logC) + 前同向段增量 O(1)。替代旧
    // O(C·S)（每 center 全扫后缀）——同时解 O(n²) 全窗瓶颈（旧 first 内层 `rev().find` 的 O(S²) 已在
    // 增量 last 维护中消除，此处保留）。
    let mut points = Vec::new();
    // 各方向序列序最近已遍历前驱段的下标（增量维护，type1 背驰对照「前一同向段」）。
    let mut last_up_idx: Option<usize> = None;
    let mut last_down_idx: Option<usize> = None;
    for (i, seg) in sorted.iter().enumerate() {
        // 该段归属的最近已确认中枢（"当下之前最后一个中枢"）。无 ⟹ 该段在所有中枢之前 ⟹ 非第一/三类。
        let center_for_seg = nearest_confirmed_center(&centers_sorted, seg.start_index);

        if let Some(c) = center_for_seg {
            // 第一类：破最近中枢 ∧ 背驰（相对前一同向段）。
            let prev_same_dir = match seg.direction {
                Direction::Up => last_up_idx,
                Direction::Down => last_down_idx,
            };
            if let Some(prev_idx) = prev_same_dir {
                if let Some(p) = judge_first(c, seg, &sorted[prev_idx], &hist, close_src) {
                    points.push(p);
                }
            }

            // 第三类：当前段作 retest，前一段作 leave。中枢归属 = **leave 段离开的最近中枢**（第18课
            // 「该中枢」），故用 leave 段 start_index 定位中枢，retest 相对**同一**中枢判一次。
            if i > 0 {
                let leave_seg = &sorted[i - 1];
                if let Some(c_leave) = nearest_confirmed_center(&centers_sorted, leave_seg.start_index)
                {
                    if let Some(p) = judge_third(c_leave, leave_seg, seg) {
                        points.push(p);
                    }
                }
            }
        }

        // 段处理完后更新「前一同向段」last（当前段不作为自己的前驱）。
        match seg.direction {
            Direction::Up => last_up_idx = Some(i),
            Direction::Down => last_down_idx = Some(i),
        }
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

    // ★探索性测试（揭示 bug，testing-override 生成态例外）：坐实「同一 retest 端点对所有
    //   zg < retest 的前驱中枢重复产出」的重复机制。修复后此测试断言**唯一归属最近中枢**。
    #[test]
    fn third_buy_belongs_only_to_nearest_center_not_all_predecessors() {
        // 三个前驱中枢，zg 递增：C0[10,20] end=2、C1[30,40] end=14、C2[50,60] end=26。
        // 离开+回试发生在 C2 之后：向上离开（端点 90 > 60），向下回试低点 70 > 60（不入 C2）。
        // bug 语义：retest 端点 70 同时满足 70 > C0.zg=20、70 > C1.zg=40、70 > C2.zg=60 ⟹ 旧码产 3 个 bsp。
        // 正确语义（第18课定理三「该中枢」+ 第49课「当下之前最后一个中枢」）：retest 只归属刚离开的
        //   最近中枢 C2（end=26，离 leave 段最近）⟹ 唯一 1 个 bsp，center=C2。
        let c0 = center(10, 20, 2);
        let c1 = center(30, 40, 14);
        let c2 = center(50, 60, 26);
        let segs = vec![
            seg(Direction::Up, 27, 30, 60, 90),    // 离开 C2 上方（端点 90 > 60）
            seg(Direction::Down, 30, 34, 90, 70),  // 回试低点 70 > 60（不入 C2）→ 3 买（仅归属 C2）
        ];
        let points = extract_third_only(&[c0, c1, c2], &segs);
        let buy3: Vec<_> = points.iter().filter(|p| p.bits.buy3).collect();
        assert_eq!(
            buy3.len(),
            1,
            "同一 retest 端点只归属刚离开的最近中枢 C2，不对 C0/C1 前驱重复产出（第18课定理三「该中枢」）"
        );
        assert_eq!(buy3[0].source_index, 34, "唯一 bsp 在回试端点");
        assert_eq!(
            buy3[0].center.map(|c| (c.zd, c.zg)),
            Some((50, 60)),
            "归属中枢 = 刚离开的最近中枢 C2[50,60]（第49课「当下之前最后一个中枢」）"
        );
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

    // ── 中枢归属语义（codex 裁决：唯一归属最近中枢）+ 复杂度标度证据 ─────────────

    /// 确定性合成数据：n_seg 条升序线段（对齐生产路径 parser 线段账本 start_index 严格单调）+
    /// n_center 个中枢 + closes/close_src。无 RNG（确定性，可复现），价格用确定性正弦式震荡造背驰差。
    fn synth_scale_input(n_seg: usize, n_center: usize) -> (Vec<Center>, Vec<Segment>, Vec<f64>, Vec<usize>) {
        // 段：方向交替，start_index 严格升序（0,2,4,...），价格在中枢上下震荡（造破中枢 + 背驰对照）。
        let mut segments = Vec::with_capacity(n_seg);
        for k in 0..n_seg {
            let dir = if k % 2 == 0 { Direction::Down } else { Direction::Up };
            let si = k * 2;
            let ei = k * 2 + 1;
            // 端点在 [40,260] 区间确定性摆动（破中枢核心 [100,200]：部分段端点 <100 或 >200）。
            let phase = (k as i64 * 37) % 220;
            let ep: Tick = 40 + phase; // 40..260
            let sp: Tick = 150;
            segments.push(Segment { direction: dir, start_index: si, end_index: ei, start_price: sp, end_price: ep });
        }
        // 中枢：核心 [100,200]，end_index 散布（造不同二分定位起点）。
        let mut centers = Vec::with_capacity(n_center);
        for j in 0..n_center {
            let ei = (j * n_seg / n_center.max(1)) * 2; // 散布在段序列各处
            centers.push(Center { zd: 100, zg: 200, dd: 50, gg: 250, start_index: 0, end_index: ei });
        }
        // closes/close_src：覆盖全段区间，确定性震荡（MACD hist 有非平凡面积 → 背驰判定真触发）。
        let n_close = n_seg * 2 + 2;
        let mut closes = Vec::with_capacity(n_close);
        let mut close_src = Vec::with_capacity(n_close);
        for t in 0..n_close {
            let osc = (((t as i64 * 53) % 80) - 40) as f64; // -40..40 确定性震荡
            closes.push(150.0 + osc);
            close_src.push(t);
        }
        (centers, segments, closes, close_src)
    }

    #[test]
    fn each_retest_endpoint_belongs_to_unique_nearest_center() {
        // ★conformance 测试（codex 明确要求，锁正确语义，替代锚定 bug 产出的旧 bit-exact oracle）：
        // 多中枢散布输入下，每个 retest 端点（source_index）只产**一个** bsp——唯一归属其最近中枢，
        // 不对所有前驱中枢重复（第18课定理三「该中枢」+ 第49课「当下之前最后一个中枢」）。
        //
        // 旧 `for c in centers` 对同一 source_index 重复产出 O(C) 次（98.9% 重复）；修复后单趟扫描
        // 每端点唯一归属 ⟹ 同一 source_index 至多一个第一类 + 至多一个第三类（互斥语义不同 bit）。
        for &(n_seg, n_center) in &[(200usize, 8usize), (1000, 16), (3000, 24)] {
            let (centers, segments, closes, close_src) = synth_scale_input(n_seg, n_center);
            let cfg = MacdConfig::default();
            let points = extract_signals(&centers, &segments, &closes, &close_src, &cfg);

            // 每个 source_index 的第三类买卖点至多一个（唯一归属最近中枢，无前驱重复）。
            use std::collections::HashMap;
            let mut third_per_src: HashMap<usize, usize> = HashMap::new();
            let mut first_per_src: HashMap<usize, usize> = HashMap::new();
            for p in &points {
                if p.bits.buy3 || p.bits.sell3 {
                    *third_per_src.entry(p.source_index).or_insert(0) += 1;
                }
                if p.bits.buy1 || p.bits.sell1 {
                    *first_per_src.entry(p.source_index).or_insert(0) += 1;
                }
            }
            for (&src, &cnt) in &third_per_src {
                assert_eq!(
                    cnt, 1,
                    "标度 ({n_seg} seg, {n_center} center)：source_index={src} 的第三类买卖点必须唯一\
                     （归属最近中枢，不对前驱重复，第18课定理三「该中枢」）"
                );
            }
            for (&src, &cnt) in &first_per_src {
                assert_eq!(
                    cnt, 1,
                    "标度 ({n_seg} seg, {n_center} center)：source_index={src} 的第一类买卖点必须唯一\
                     （归属最后中枢，不对前驱重复，maimai.md:103「最后一个中枢」）"
                );
            }
            // 含 3 类 bit 的 bsp 的 center 必 Some 且 = 某真实输入中枢（归属正确，非零占位）。
            for p in &points {
                if p.bits.buy3 || p.bits.sell3 {
                    let c = p.center.expect("含 3 类 bit ⟹ center 必 Some");
                    assert!(
                        centers.iter().any(|ic| ic.zd == c.zd && ic.zg == c.zg && ic.end_index == c.end_index),
                        "归属中枢必是真实输入中枢之一（最近中枢，非臆造）"
                    );
                }
            }
        }
    }

    #[test]
    fn extract_signals_handles_unsorted_input_via_stable_sort() {
        // 严格性（零假设）：extract_signals 入口稳定排序 ⟹ 即使输入乱序（非生产路径，但公开函数
        // 不能假设升序），结果 = 按 start_index 升序处理的确定性输出。验证排序后切片二分定位正确：
        // 乱序输入与其升序版本产相同 bsp（排序消除输入顺序依赖）。
        let c = center(100, 200, 2);
        let asc = vec![
            seg(Direction::Down, 3, 5, 150, 90),
            seg(Direction::Down, 6, 8, 120, 80),
        ];
        let desc = vec![
            seg(Direction::Down, 6, 8, 120, 80),
            seg(Direction::Down, 3, 5, 150, 90),
        ];
        let prices: Vec<Tick> = vec![100, 100, 100, 100, 60, 140, 100, 95, 105];
        let (closes, src) = closes_seq(&prices);
        let cfg = MacdConfig::default();
        let from_asc = extract_signals(&[c], &asc, &closes, &src, &cfg);
        let from_desc = extract_signals(&[c], &desc, &closes, &src, &cfg);
        assert_eq!(from_asc, from_desc, "稳定排序消除输入顺序依赖 ⟹ 乱序与升序输入产相同 bsp");
    }

    /// 多中枢散布的合成数据（隔离单趟扫描的 O(S·logC) 标度——大量中枢 + 大量线段）。
    ///
    /// ★为什么需要：旧 `for c in centers` 主循环对每 center 全扫线段后缀 = O(C·S)，且第一/三类对
    /// 前驱中枢**重复产出** O(C) 份/端点（98.9% 重复 + O(n²) 全窗瓶颈双重根源）。本档造 n_center 个
    /// 中枢散布在段序列各处 + 段端点剧烈摆动（破中枢 + 离开/回试），复现旧码 C·S 主循环成本。修复后
    /// 单趟扫描每段 O(logC) 二分定位归属中枢 ⟹ 总 O(S·logC)，无 C·S 全扫 + 无前驱重复。
    fn synth_many_center_input(n_seg: usize, n_center: usize) -> (Vec<Center>, Vec<Segment>, Vec<f64>, Vec<usize>) {
        // 方向交替段，start_index 升序，端点剧烈摆动（破中枢 + 离开/回试，最大化产出路径触发）。
        let mut segments = Vec::with_capacity(n_seg);
        for k in 0..n_seg {
            let dir = if k % 2 == 0 { Direction::Down } else { Direction::Up };
            let si = k * 2;
            let ei = k * 2 + 1;
            let phase = (k as i64 * 37) % 220;
            let ep: Tick = 40 + phase; // 40..260（破中枢核心 [100,200] + 离开/回试）
            segments.push(Segment { direction: dir, start_index: si, end_index: ei, start_price: 150, end_price: ep });
        }
        // n_center 个中枢，end_index 散布在段序列各处（每段二分定位不同归属中枢，复现 C 维成本）。
        let mut centers = Vec::with_capacity(n_center);
        for j in 0..n_center {
            let ei = (j * n_seg / n_center.max(1)) * 2;
            centers.push(Center { zd: 100, zg: 200, dd: 50, gg: 250, start_index: 0, end_index: ei });
        }
        let n_close = n_seg * 2 + 2;
        let mut closes = Vec::with_capacity(n_close);
        let mut close_src = Vec::with_capacity(n_close);
        for t in 0..n_close {
            let osc = (((t as i64 * 53) % 80) - 40) as f64;
            closes.push(150.0 + osc);
            close_src.push(t);
        }
        (centers, segments, closes, close_src)
    }

    /// 标度计时（`--ignored` 显式触发，不拖累常规测试）：修复后单趟扫描 `extract_signals` 在
    /// 多中枢散布输入上耗时。验证 O(S·logC) 标度（线性 × log）——段数 ×N、中枢数 ×N 时耗时近线性增长，
    /// 非旧 O(C·S)/O(n²) 的二次/平方爆炸。L2 profile 坐实旧热点 = `extract_signals` O(n²)（前驱重复 +
    /// 每 center 全扫），本档复现规模并展示修复后近线性耗时（无 oracle 对照——旧 oracle 锚定 bug 产出
    /// 已删，no-patch）。
    #[test]
    #[ignore = "标度计时，--ignored 显式触发"]
    fn scale_timing_single_pass() {
        use std::time::Instant;
        let cfg = MacdConfig::default();
        // 段数与中枢数同步放大（旧 O(C·S) 会二次爆炸；修复后 O(S·logC) 近线性）。
        for &(n_seg, n_center) in &[(4000usize, 32usize), (16000, 64), (64000, 128)] {
            let (centers, segments, closes, close_src) = synth_many_center_input(n_seg, n_center);
            let t0 = Instant::now();
            let points = extract_signals(&centers, &segments, &closes, &close_src, &cfg);
            let dt = t0.elapsed();
            println!(
                "[单趟扫描 S={n_seg} C={n_center}] 耗时={:?} bsp={}（O(S·logC) 近线性，无前驱重复）",
                dt, points.len()
            );
        }
    }
}
