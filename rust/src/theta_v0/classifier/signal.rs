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
//!   IsDivergence(divPair)`。**第一类 = 趋势背驰点**（A/B/C 框架，第24课:22-24 + maimai.md:103-112）。
//!   三分量分层：
//!   · **局部趋势门**（`decompose`+`center_trend_gate`，task #143）：第一类**只由趋势背驰产生**——段所在走势类型块 ≥2 同向中枢
//!     ⟹ Trend(方向)，才产第一类；1 中枢（盘整）/mixed（扩张）**不产**（盘整背驰不产第一类，
//!     beichi #4 + maimai.md:56 已结算）。**L0**（中枢同向外缘关系，纯整数几何）。这是「假背驰=
//!     假买卖点」头号缺口的修复——退化实现跳过 τ 门控，单中枢/任意段都产第一类=噪声。
//!   · **破中枢几何分量**（`brokeCenter`）：C 段端点破**最后一个中枢**核心 `[zd,zg]` 之外（买侧
//!     向下破 `< zd`，卖侧向上破 `> zg`）——纯整数比较，**L0**（608号位置三态，对齐 descend.rs
//!     `sub_broke_below`/`sub_broke_above`）。
//!   · **趋势背驰力度分量**（`IsDivergence`，A/B/C）：C 段（破最后中枢段）相对 **A 段（倒数第二
//!     中枢的离开段，跨相邻中枢配对）** MACD 面积**严格变小**（第24课:24「C 段面积 < A 段面积」）。
//!     ★A/C 跨相邻中枢配对（`locate_trend_seg_a`）替换退化的「任意前同向段」——A 不是序列序任意前
//!     同向段，而是趋势中相邻前一中枢的离开段（第24课:22「同向趋势之间一定有中枢连接」=B 段）。
//!     由已实装 MACD（`divergence::AbcDivergence::diverges`）**真算**（★rust 领先 Origin：Lean
//!     `divPair` 是外部参数 still-MISSING-C 无 Origin MACD 引擎，且 Lean 标 still-MISSING-C 不实装
//!     A/B/C 段结构；rust 实装 A/B/C 框架 + MACD，故背驰分量**真算非占位**，见 descend.rs 模块头）。
//!     认识论：A/C 段配对 + MACD 面积比较确定 = **L1**（管线正确性，对齐 Lean IsType1 结构合取）；
//!     「MACD 趋势背驰预测在真实行情有效」才是 **L2/L3**（否证检验，**不在本工位**，alpha 影响须
//!     统一 L3 验证）。趋势/盘整门控判据（中枢同向关系）= **L0**（纯整数几何，零经验依赖）。
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
use super::super::types::{Center, Direction, MoveKind, Segment, Side, Tick};
// Side 已在上行 import（judge_first_cached 用它构造 BspPoint.struct_break_dir，P2-R2）。
use super::bsp::{endpoint_to_bsp, EndpointSituation};
use super::decompose::{center_block_kind, center_trend_gate, decompose};
use super::divergence::{
    compute_macd, departure_move_c_start, force_features, locate_departure_move_a, AbcDivergence,
    ForceProxies,
};
use super::super::types::BspBits;
use super::descend::RMove;
use super::rmove_compose::find_second_type_structure;

/// 买卖点条目（带结构止损价，single source，见 `bsp::BspPoint`）。
pub use super::bsp::BspPoint;

// ★P2-R2（codex-review-20260701-2251 护栏7）：`StructBreakFeature` sidecar 已删除。
// 它从未接通生产（mod.rs 全走 `extract_signals(...).0` / `extract_signals_with_hist(...).0` 丢弃
// sidecar，零生产消费者）。macd_c_lt_a（C<A 背驰）信息**完全可从产出 BspPoint 派生**：
//   `struct_break_dir=Some ∧ (bits.buy1 ∨ bits.sell1)` ⟺ macd_c_lt_a=true（破中枢 ∧ 背驰确认）；
//   `struct_break_dir=Some ∧ 六 bit 全零`                ⟺ macd_c_lt_a=false（破中枢 ∧ 未背驰）。
// 删除死 sidecar（no-patch 死代码清除）零信息损失——R2 的 struct_break_dir 直接承载破中枢方向，
// 候选恢复方向进样本不再需要「零 bit 候选 + feature 供 χ 学 MACD」这条从未接通的迂回路径。

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

/// ★性能工位（#93）：[`nearest_confirmed_center`] 的索引版本——直接返回**最近中枢在 `centers` 中的下标**
/// 而非 `&Center`。
///
/// ★为什么需要（消解 `.position()` O(S·C) 热点）：旧主循环第一类路径在拿到最近中枢 `c` 后，用
/// `centers.iter().position(|x| x.end_index==c.end_index && x.zd==c.zd && x.zg==c.zg)` 线性扫 O(C)
/// **反查** `c` 的下标（为取 `prev_center = centers[pos-1]`）。`nearest_confirmed_center` 内部
/// `partition_point` 已算出 `hi`（前缀长）⟹ 最近中枢下标 = `hi-1`——可直接返回，无需反查。
///
/// 旧路径的 O(S·C)：每段一次 O(C) 反查 ⟹ S 段 = O(S·C)。profile 坐实（`scale_timing_position_hotspot`
/// exp≈1.98）。本索引版直接由 `partition_point` 的 `hi` 推出下标 O(1)，主循环改为本函数 ⟹ O(S·logC)
/// （partition_point 二分 O(logC)）。
///
/// ★bit-exact 不变式（**核心**）：本函数返回 `hi-1`（前缀末下标 = 最近中枢真位置）。旧 `.position()`
/// 三元组首匹配在**重复三元组**下取**更小**下标（首匹配 ≠ 前缀末）⟹ `prev_center` 不同 ⟹ 第一类输出
/// 可能不同。为严格 bit-exact，[`extract_signals`] 在主循环前预建 `first_match_idx:
/// HashMap<(end_index, zd, zg), usize>`（正向扫 `centers_sorted`，仅在 key 不存在时插入 ⟹ 保留首匹配
/// 下标，与 `.position()` 语义逐位一致），主循环用 `first_match_idx.get(&c 三元组)` 取 `pos`（O(1)）。
/// `c`（last_center）仍取 `centers_sorted[c_idx]`（最近中枢语义正确）。`bit_exact_battery_digest`
/// 测试 D（重复三元组）锁定此路径。
fn nearest_confirmed_center_idx(centers: &[Center], seg_start: usize) -> Option<usize> {
    let hi = centers.partition_point(|c| c.end_index <= seg_start);
    if hi == 0 {
        None
    } else {
        Some(hi - 1)
    }
}

/// 第一类买卖点判定（契约锚 `Origin.BspClassification.IsType1 = brokeCenter ∧ IsDivergence`；
/// ★A/B/C 趋势背驰框架，第24课:22-24 + reference:34 + maimai.md:103-112）。
///
/// **第一类买卖点 = 趋势背驰点**（maimai.md:103「某级别**下跌趋势**中…向下跌破**最后一个**中枢后
/// 形成的**背驰点**」；前提 maimai.md:105「≥2个依次同向的同级别中枢」）。本函数实装 A/B/C 三段
/// 框架的趋势背驰判定（**非**退化的「任意前同向段」面积比较）：
///
/// - **C 段**（后一离开段）= 破**最后一个中枢** `last_center` 的离开段 `seg`（破中枢几何 L0）。
/// - **A 段**（前一离开段）= **倒数第二个中枢** `prev_center` 的同向离开段（[`locate_trend_seg_a`]
///   跨相邻中枢配对，第24课:24「A 之前已有一个中枢，B 是这个大趋势的另一个中枢」）。
/// - **B 段** = `prev_center` 与 `last_center` 之间的中间中枢（由趋势 ≥2 中枢门控隐含保证）。
/// - **趋势背驰** = C 段面积 < A 段面积（力度衰减，`segments_diverge` 力度原语 L1）。
///
/// - **1买**：下跌趋势（τ=Trend(Down)）中向下线段端点 `< last_center.zd`（破最后中枢下沿）∧
///   C段面积 < A段面积（底背驰）⟹ 1 买。
/// - **1卖**：上涨趋势（τ=Trend(Up)）镜像——向上端点 `> last_center.zg`，顶背驰 ⟹ 1 卖。
///
/// ★τ 门控（消解假背驰=假买卖点头号缺口）：调用方（[`extract_signals`]）已用局部趋势门（decompose，#143）
/// 确认 τ=Trend 才判第一类；`trend_dir`（趋势方向）= τ 的方向，破中枢方向必须与趋势方向一致
/// （下跌趋势=向下破=底背驰；上涨趋势=向上破=顶背驰）——盘整（Consolidation）/退化（mixed/扩张）
/// **不产第一类**（盘整背驰不产第一类，beichi #4 + maimai.md:56 已结算）。
///
/// ★A/C 跨相邻中枢配对（替换退化的「序列序任意前同向段」）：A 段不是任意前同向段，而是趋势中
/// **相邻前一中枢**（`prev_center`）的离开段。退化实现把任意相邻同向段面积变小都判背驰=产假买卖点；
/// 本实装要求 A/C 分属相邻两中枢（第24课:22「同向趋势之间一定有一个…中枢连接」）。
///
/// ★分量 L 级（formalization-validity-domain）：破中枢 `< zd`/`> zg` + 趋势门控（中枢同向关系）
/// 整数几何 **L0**；C<A 面积比较 MACD **L1**。三者合取 = 趋势背驰 = `IsType1`。`prev_center` 无
/// 同向离开段（A 段无法定位）⟹ None（无 A/C 配对 ⟹ 无趋势背驰）。
///
/// ★#93 性能工位（热点②修复）：A 段 `(a_start, a_end)` 不再在本函数内调 `locate_trend_seg_a`——改由
/// 调用方（[`extract_signals`]）按 `last_center_idx` 缓存预算后传入（`a_seg` 入参）。
/// `locate_trend_seg_a` 与 C 段 `seg` 无关（仅依赖 `(segments, prev_center, last_center, trend_dir)`）
/// ⟹ 趋势 τ 下多段共享同一 `(prev_center, last_center)` 对 ⟹ A 段每对至多算一次，消解旧「每段重算」
/// 的 O(S²)（profile `scale_timing_breaking_path` exp≈1.99 坐实）。
///
/// ★bit-exact 不变式：传 `a_seg = locate_trend_seg_a(segments, prev_center, last_center, trend_dir)`
/// 的结果时，本函数输出与内联调 `locate_trend_seg_a` 的旧实现**逐字段相等**——broke 判定、A/C 段区间、
/// MACD 面积、`AbcDivergence` 结构、`diverges` 判定、`EndpointSituation` 与 `BspPoint` 构造全部共享
/// 同一代码路径（仅 A 段来源从「内联调」变「外传入」，值同）。
///
/// ★`prev_center`/`segments` 不入参：A 段已预算入参（含 prev_center 的语义 + segments 已扫），本函数
/// 不再需要它们（仅 `last_center` 提供 zd/zg 几何判据）。prev_center/segments 的语义在调用方缓存键
/// （last_center_idx ⟹ prev_center=centers[idx-1]）与 `locate_trend_seg_a` 预算中保留。
///
/// `hist` 是 MACD hist 序列；`src_to_idx` 是 closes 下标→source_index 映射。段无法映射到 closes
/// 区间（越界）⟹ None（无面积 ⟹ 非背驰）。
///
/// `a_seg`：`None` = `locate_departure_move_a` 返回 None（无 A 候选）；`Some((λ_A,ρ_A))` = A 区间。
/// `c_move_start`：I(C) 起点 λ_C（Q5 + codex ac4 #2）= 离开 `last_center` 的**当前 episode** 首同向
/// 段起点（[`departure_move_c_start`] 单一来源——episode 边界 = seg 之前最后一个回中枢段，「失败
/// 离开→回中枢→重新离开」不桥接）。`None` = 无同向离开段——broke 成立时 seg 自身在窗口且在最后
/// 回中枢段之后 ⟹ 必 `Some`。
///
/// ★Q5 区间语义（task #145）：I(C) = [λ_C, seg.end_index]——离开最后中枢的**整个次级别走势区间**
/// （多段 departure 含中间反向段 bar），MACD 面积/力度 proxy 全用 I(C)。**破中枢几何仍用 seg 端点**
/// （因果触发）：破中枢判据 min P(C) < ZD（Up 镜像 max P(C) > ZG）与「某同向段端点越界」等价——
/// 段是单向对象、终点即极值，多段 move 的 min/max 首次越界 ⟺ 某同向段端点越界，触发时刻即该段，
/// 故 seg 端点判破 = I(C) 极值判破的因果触发点（等价性，裁决注记）。
fn judge_first_cached(
    last_center: &Center,
    trend_dir: Direction,
    seg: &Segment,
    anchor_dir: Option<Direction>,
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    src_to_idx: &[usize],
    a_seg: Option<(usize, usize)>,
    c_move_start: Option<usize>,
) -> Option<BspPoint> {
    let end = seg_end(seg);
    // C 破最后中枢几何（L0）+ 方向必须 = 趋势方向（下跌趋势=向下破=底背驰；上涨=向上破=顶背驰）。
    // 因果触发点 = 破中枢段端点（Q5 等价性注记见函数头）。
    // Q7-#1 裁定C：破中枢段方向锚用 anchor_dir（provenance 资格）——fallback 单元（None）不触发。
    // L0 恒 anchor_dir==Some(end.dir)（线段内在方向）⟹ 与旧 (end.dir, trend_dir) 匹配逐位一致。
    let (broke, is_sell) = match (anchor_dir, trend_dir) {
        // 1 买：下跌趋势中向下破最后中枢下沿（底背驰候选）。
        (Some(Direction::Down), Direction::Down) if end.price < last_center.zd => (true, false),
        // 1 卖：上涨趋势中向上破最后中枢上沿（顶背驰候选）。
        (Some(Direction::Up), Direction::Up) if last_center.zg < end.price => (true, true),
        _ => (false, false),
    };
    if !broke {
        return None; // 未破最后中枢 ⟹ 非第一类结构候选（几何分量不足）。
    }
    // A 区间由调用方预算传入（缓存复用，消解热点②）。无 A 候选 ⟹ A/C 无法配对 ⟹ 无趋势背驰对照。
    let Some((a_start, a_end)) = a_seg else {
        return None; // 无 prev_center 同向离开走势 ⟹ A/C 无法配对 ⟹ 无 struct_break 候选。
    };
    // λ_C（Q5）：broke 成立 ⟹ seg 自身满足「同向 ∧ start ≥ c.end_index」过滤 ⟹ 首匹配必存在。
    let Some(lambda_c) = c_move_start else {
        debug_assert!(false, "broke 成立时 λ_C 必 Some（seg 自身在过滤集内）");
        return None;
    };
    // I(A)/I(C)（source_index 区间）→ closes 下标区间（MACD 面积坐标系，Q5 全区间口径）。
    let (Some(c_idx), Some(a_idx)) = (
        map_src_range_to_close_idx(src_to_idx, lambda_c, seg.end_index),
        map_src_range_to_close_idx(src_to_idx, a_start, a_end),
    ) else {
        // 区间无法映射到 closes（越界/空）⟹ 无 MACD 面积 ⟹ 无法算 C<A ⟹ 无 struct_break 候选。
        return None;
    };
    // ★A/B/C 背驰段对（结构化，对齐 Lean `Origin.Divergence.DivergencePair { forceA, forceC, isTrend }`）：
    // I(A) + I(C)（source_index 区间，Q5 走势区间口径）+ is_trend=true（第一类只由趋势背驰产）。
    let abc = AbcDivergence { seg_a: (a_start, a_end), seg_c: (lambda_c, seg.end_index), is_trend: true };
    // ★P2-R2（codex-decide-20260701-2121 → p2-plan §2）：C<A 从 gate 降为「buy1 判据」——**不 return
    // None**，破中枢结构候选（趋势 ∧ 破最后中枢 ∧ A/C 可配对）全部进样本（消选择偏差，下游 χ 可否证
    // MACD）。趋势背驰（L1 真算）：C 段面积严格小于 A 段面积（第24课:24）。
    // ★macd_c_lt_a 不再作独立 StructBreakFeature sidecar（P2-R2 删除死代码，codex 护栏7）——它**完全
    // 可从产出 BspPoint 派生**：`struct_break_dir=Some ∧ bits.buy1/sell1=true` ⟺ macd_c_lt_a=true
    // （背驰确认）；`struct_break_dir=Some ∧ 六 bit 全零` ⟺ macd_c_lt_a=false（未背驰）。sidecar 从
    // 未接通生产（mod.rs 全走 `.0`），删除比接通更干净且零信息损失。
    let macd_c_lt_a = abc.diverges(hist, a_idx, c_idx);
    // buy1/sell1 保严格「趋势背驰」语义：仅背驰确认（C<A）才置第一类 bit。未背驰的破中枢候选
    // 进样本但**零 buy1/sell1**（Flat 候选，assemble_gamma 归 𝒦 不冒充第一类，codex 语义纪律）。
    let situ = EndpointSituation {
        after_first_buy: false,
        is_pullback_end: false,
        left_center: false,      // 第一类是破中枢趋势背驰，非第三类的离开后回抽
        retrace_not_reenter: false,
        below_last_center: macd_c_lt_a, // 仅背驰确认才置第一类端点语义（未背驰=零 bit struct_break）
        is_sell_side: is_sell,
    };
    let bits = endpoint_to_bsp(&situ);
    // ★P2-R2（p2-plan §2 + codex 护栏1/2）：破中枢方向源——买侧向下破=Long，卖侧向上破=Short。
    // **无条件置**（只要几何破了最后中枢 ∧ A/C 可配对就到这里，不管 macd_c_lt_a 背驰与否）。
    // buy1/sell1 仍严格按 macd_c_lt_a（上方 situ.below_last_center）——class_index 语义冻结。
    // 零 bit（C≥A）候选靠 struct_break_dir 在 candidate_dir 恢复方向进样本（消选择偏差）。
    let struct_break_dir = Some(if is_sell { Side::Short } else { Side::Long });
    // ★力度多 proxy（beta-route #115，force-proxy-survey-20260702.md）：A/C 段 close 下标区间已算出
    // （a_idx/c_idx），复用 force_features 算 5 proxy（MACD 面积/DIF 峰/振幅/速度/TV）。**不进 buy1 判据**
    // （class_index 冻结，671 力度=feature 非 veto）——纯 feature，收进 BspPoint.force 单一来源，供
    // selector z_of_candidate_with_force 读进 force_state 第 8 维。有 dif/closes_tick 输入时 Some
    // （生产热路径已接线，Batch 2）；空输入（旧测试/合成入口）⟹ None（诚实不造死字段）。A/C 同趋势方向。
    let force = if dif.is_empty() || closes_tick.is_empty() {
        None
    } else {
        Some(ForceProxies {
            seg_a: force_features(hist, dif, closes_tick, a_idx.0, a_idx.1, trend_dir),
            seg_c: force_features(hist, dif, closes_tick, c_idx.0, c_idx.1, trend_dir),
        })
    };
    // 结构候选端点：背驰确认 ⟹ buy1/sell1 止损源 pivot（破中枢段端点极值）；未背驰 ⟹ 零 bit，
    // pivot 仍按 bit 方向填（零 bit ⟹ 两侧 0）。force 旁挂进点（单一来源，不进任何 bit 判据）。
    Some(make_first_point(end.source_index, bits, end.price, struct_break_dir, force))
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
/// ★Q7-#1 裁定C：`leave_anchor` = leave 段的方向锚资格——三类的离开/突破段须有锚（fallback ⟹
/// None ⟹ 非三类结构）。retest 段是几何回试角色（回试不触中枢的区间判据），不设锚门。
fn judge_third(
    c: &Center,
    leave_seg: &Segment,
    leave_anchor: Option<Direction>,
    retest_seg: &Segment,
) -> Option<BspPoint> {
    let leave = seg_end(leave_seg);
    let retest = seg_end(retest_seg);
    match (leave_anchor, retest.dir) {
        // 3 买：向上离开（leave 端点 > c.zg）+ 向下回试（retest 低点 > c.zg，不触及闭区间中枢）。
        (Some(Direction::Up), Direction::Down) if leave.price > c.zg && retest.price > c.zg => {
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
        (Some(Direction::Down), Direction::Up) if leave.price < c.zd && retest.price < c.zd => {
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
///
/// ★P2-R2（p2-plan §2）：`struct_break_dir=Some(破中枢方向)`——由 [`judge_first_cached`] 传入
/// （买侧向下破=Long，卖侧向上破=Short），**与 bits 是否置 buy1/sell1 无关**。零 bit 破中枢候选
/// （C≥A 未背驰）靠此字段在 `candidate_dir` 恢复方向进样本（消选择偏差）。pivot 仍按 bit 方向填
/// （零 bit ⟹ 两侧 0，止损源留待背驰确认——未背驰候选不置 buy1，无 1 类 pivot 止损语义）。
fn make_first_point(
    source_index: usize,
    bits: BspBits,
    pivot_price: Tick,
    struct_break_dir: Option<Side>,
    force: Option<ForceProxies>,
) -> BspPoint {
    BspPoint {
        source_index,
        bits,
        // 1 买止损源 pivot_low（破中枢低点）；1 卖止损源 pivot_high（破中枢高点）。
        pivot_low: if bits.buy1 { pivot_price } else { 0 },
        pivot_high: if bits.sell1 { pivot_price } else { 0 },
        center: None,
        struct_break_dir,
        // ★β^div 力度（beta-route #115）：一类趋势背驰候选的 A/C 段 5 proxy（有 dif/closes_tick 输入时
        // Some，否则 None）——单一来源就在此字段，selector z_of_candidate_with_force 读它进 force_state。
        force,
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
        struct_break_dir: None, // 第三类=离开后回抽，非破中枢结构候选（P2-R2 语义：None）
        force: None,            // 第三类无 A/C 趋势段对 ⟹ 无力度 proxy（诚实 None，231号）
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
        struct_break_dir: None, // 第二类=中枢内部回拉，非破中枢结构候选（P2-R2 语义：None）
        force: None,            // 第二类无 A/C 趋势段对 ⟹ 无力度 proxy（诚实 None，231号）
    }
}

/// 盘整背驰证书（Q4 裁决，task #145：「盘整背驰不能消失——它必须被某级别买卖点或小转大/区间套
/// 证书承接」。`PanDiv^δ_ℓ ⟹ ∃e<ℓ, Conf^δ_e` 或 `PanDiv^δ_ℓ ⟹ XZD^δ_{ℓ↓e}`）。
///
/// **不是买卖点**：本证书**不置任何 six-bit、不产 BspPoint**——盘整背驰不冒充同级 B1/S1
/// （「标准第一类买卖点应锚定趋势背驰」，Do not call every consolidation divergence same-level
/// B1/S1. But do not drop it.）。承接路由在 econ 统计层（econ_positive::collect_signals 消费
/// `LevelState.pan_div`，走现有 Nest/XZD 二通道门；两门皆闭 ⟹ 诚实丢弃）。
///
/// 结构语义（第24课:34-36 + beichi.md:113 盘整背驰 = **同一中枢**两次同向离开，
/// `AbcDivergence.is_trend=false`）：
/// - `seg_c`：当前离开走势区间 I(C)（Q5 同款区间语义，source_index 闭区间），末段破中枢核心。
/// - `seg_a`：**前一次同向离开 episode 区间** I(A)（Q5 区间口径，A/C 对称）——锚段端点破核心，
///   与 C 之间存在回中枢段（否则是同一次离开）。
/// - Weak = MACD 面积 C < A（与 buy1 同一冻结力度原语 `segments_diverge`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanDivCert {
    /// 破中枢段端点 source_index（因果触发点，承接路由的定位键）。
    pub source_index: usize,
    /// 承接方向候选：向下破 = Long 候选 / 向上破 = Short。
    pub side: Side,
    /// 盘整背驰所在的中枢（A/C 两次离开的同一中枢 = B）。
    pub center: Center,
    /// I(A)（前一次同向离开 episode 区间，Q5 区间口径）source_index 闭区间。
    pub seg_a: (usize, usize),
    /// I(C)（当前离开走势区间）source_index 闭区间。
    pub seg_c: (usize, usize),
}

/// 盘整背驰判定（Q4，[`PanDivCert`] 的唯一构造点）。
///
/// 前提（调用方保证）：`c` 是 `seg` 的最近已确认中枢，且 c 按 ownership 落在 **Consolidation 块**
/// （[`center_block_kind`]，与趋势门同一 decompose 单一来源）。判据链：
/// 1. **破中枢核心**（因果触发 = seg 端点，等价性同 judge_first_cached）：Down ⟹ 端点 < c.zd
///    （Long 候选）/ Up ⟹ 端点 > c.zg（Short）。
/// 2. **当前离开区间 I(C)**（Q5 区间语义）：λ_C = 最后一个回中枢段 r（反向段、端点回到核心内侧：
///    Down 破侧 end ≥ zd / Up 破侧 end ≤ zg，r 在 seg 之前）之后的首个同向段起点；I(C)=[λ_C, seg.end]。
/// 3. **A = 同一中枢的前一次同向离开末段**：direction==dir ∧ end_index ≤ λ_C ∧ 端点破核心的末段；
///    且 A 与 C 之间存在回中枢段（r 的存在性 + 显式区间检查——否则是同一次离开）。
/// 4. **Weak**：MACD 面积 C < A（`AbcDivergence::diverges`，同 buy1 冻结原语）。
///
/// 无回中枢段（只有一次离开）/ 无 A / 未背驰 / 区间无法映射 closes ⟹ None（诚实不产证书）。
/// 复杂度：窗口 = start_index ∈ [c.end_index, seg.start_index] 的段（二分定界 + 窗口内线性扫）。
fn judge_pan_div(
    c: &Center,
    seg: &Segment,
    segments: &[Segment],
    anchors_self: &[Option<Direction>],
    hist: &[f64],
    src_to_idx: &[usize],
) -> Option<PanDivCert> {
    let end = seg_end(seg);
    // 1. 破中枢核心（因果触发点 = 破段端点）。
    let side = match end.dir {
        Direction::Down if end.price < c.zd => Side::Long,
        Direction::Up if end.price > c.zg => Side::Short,
        _ => return None, // 未破核心 ⟹ 非离开确认 ⟹ 无盘整背驰候选。
    };
    let dir = end.dir;
    // 窗口：与 seg 同归属本中枢的段（start_index ∈ [c.end_index, seg.start_index]，升序切片）。
    let lo = segments.partition_point(|s| s.start_index < c.end_index);
    let hi = segments.partition_point(|s| s.start_index <= seg.start_index);
    let win = &segments[lo..hi];
    // 回中枢段判据：反向段端点回到核心内侧（Down 破侧 ≥ zd / Up 破侧 ≤ zg——两次离开之间价格
    // 须回到中枢，否则是同一次离开的内部反弹）。
    let reenters = |s: &Segment| {
        s.direction != dir
            && match dir {
                Direction::Down => s.end_price >= c.zd,
                Direction::Up => s.end_price <= c.zg,
            }
    };
    // 2. 存在回中枢段（当前离开 episode 与前次离开的分界）。无 ⟹ 仅一次离开 ⟹ None。episode
    //    边界本身由共享 helper（departure_move_c_start → episode_start_in）内部定位同一段。
    win.iter().rev().filter(|s| s.end_index <= seg.start_index).find(|s| reenters(s))?;
    // λ_C = r 之后首个同向段起点（[`departure_move_c_start`] 单一来源；r = 窗口内最后回中枢段 ⟹
    // helper 的 episode 边界与 r 相同；seg 自身满足过滤 ⟹ 必 Some）。
    let lambda_c = departure_move_c_start(segments, anchors_self, c, dir, seg.start_index)?;
    // 3. A = 同一中枢的**前一次同向离开 episode 区间**（Q5 区间口径，codex ac4 复审：A 侧与 C 侧
    //    对称 episode 化，不再只取末段）。锚 = λ_C 之前端点破核心的末个同向段（存在性 = 前次离开
    //    确认）；I(A) = [λ_A, ρ_A]——λ_A 经共享 helper（锚所在 episode 起点），ρ_A = episode 内
    //    （首个后续回中枢段之前）末个同向段终点。
    let a_anchor = win
        .iter()
        .rev()
        .filter(|s| s.direction == dir && s.end_index <= lambda_c)
        .find(|s| match dir {
            Direction::Down => s.end_price < c.zd,
            Direction::Up => s.end_price > c.zg,
        })?;
    // A 与 C 之间存在回中枢段（显式区间检查：锚后 ≤ 回段 ≤ λ_C——否则 A 与 C 是同一次离开）。
    if !win
        .iter()
        .any(|s| reenters(s) && s.start_index >= a_anchor.end_index && s.end_index <= lambda_c)
    {
        return None;
    }
    let lambda_a = departure_move_c_start(segments, anchors_self, c, dir, a_anchor.start_index)?;
    // episode 终界 = 锚后首个回中枢段起点（分隔段，上一检查保证存在；fallback λ_C 防御性等价）。
    let episode_end = win
        .iter()
        .find(|s| reenters(s) && s.start_index >= a_anchor.end_index)
        .map_or(lambda_c, |r| r.start_index);
    // ρ_A = episode 内末个同向段终点（多段前次离开含中间未回核心的反向段 bar，与趋势侧 A 同口径）。
    let rho_a = win
        .iter()
        .rev()
        .filter(|s| s.direction == dir && s.start_index >= lambda_a && s.end_index <= episode_end)
        .map(|s| s.end_index)
        .next()?;
    // 4. Weak：MACD 面积 C < A（同 buy1 冻结原语；is_trend=false = 盘整背驰语义）。
    let (c_span, a_span) = ((lambda_c, seg.end_index), (lambda_a, rho_a));
    let (Some(c_idx), Some(a_idx)) = (
        map_src_range_to_close_idx(src_to_idx, c_span.0, c_span.1),
        map_src_range_to_close_idx(src_to_idx, a_span.0, a_span.1),
    ) else {
        return None; // 区间无法映射 closes ⟹ 无面积 ⟹ 不冒充背驰。
    };
    let abc = AbcDivergence { seg_a: a_span, seg_c: c_span, is_trend: false };
    if !abc.diverges(hist, a_idx, c_idx) {
        return None; // C ≥ A ⟹ 力度未衰减 ⟹ 非盘整背驰。
    }
    Some(PanDivCert { source_index: end.source_index, side, center: *c, seg_a: a_span, seg_c: c_span })
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
/// 对每个中枢，取其 `end_index` 之后的线段子序列，提取**第一类**（趋势背驰，A/B/C 框架）+
/// **第三类**（离开后回试不破）买卖点。买卖点按 source_index 升序返回（reference:16 平局裁决——
/// 已确认结构不回写，时间序天然升序）。
///
/// ★第一类 = 趋势背驰（A/B/C 框架，第24课:22-24 + maimai.md:103-112，消解 grammar-audit 头号缺口）：
/// 本函数在 L0 层从 `centers` 派生走势类型分解（[`decompose`]，Q1/Q8），**只在段所在块为 Trend 时产第一类**——
/// A 段 = 倒数第二中枢离开段，C 段 = 破最后中枢段，C段面积<A段面积（跨相邻中枢配对，非退化的「任意
/// 前同向段」）。盘整（Consolidation）/退化（mixed/扩张）τ **不产第一类**（盘整背驰不产第一类，
/// beichi #4 + maimai.md:56 已结算）。这是「假背驰=假买卖点」头号缺口的修复点。
///
/// ★第二类（B2/S2）**不在本函数产出**——本函数入参是 L0 线段（递归底，无次级别走势对象 ⟹ 结构
/// 上不可产第二类，签名层边界，见模块头）。B2/S2 由平行的递归组装层入口 [`extract_second_signals`]
/// 产（消费 RMove 递归塔的 `SecondTypeStructure`）——按输入对象分工，非本函数缺口，零冒充。
///
/// `closes`：close 序列（`ParseLayer.merged_bars` 的 close，MACD 算趋势背驰用）。
/// `close_src`：closes 各元素的 source_index（merged_bars 锚点，段区间坐标系转换用）。
/// `macd_cfg`：MACD 参数（fast/slow/signal，从 ThetaConfig.macd 读）。
///
/// ★本函数覆盖：B1/S1（趋势背驰 A/B/C 框架）+ B3/S3（confirmed 结构几何）。
/// B2/S2 见 [`extract_second_signals`]（递归组装层）。
pub fn extract_signals(
    centers: &[Center],
    segments: &[Segment],
    closes: &[f64],
    close_src: &[usize],
    macd_cfg: &MacdConfig,
) -> Vec<BspPoint> {
    // MACD hist（趋势背驰真算，浮点域隔离在 divergence.rs）。空 closes ⟹ 空 hist ⟹ 第一类结构候选
    // 不产（段无法映射 closes 区间 ⟹ 无 C<A 判据），第三类仍正常产（纯整数几何）。
    let hist = compute_macd(closes, macd_cfg).hist;
    // 简易入口传空 dif/closes_tick ⟹ 各点 force=None（此入口不算力度；GOLDEN 电池走此路，force 恒 None
    // ⟹ 结构 bit-exact，仅 Debug 多 `, force: None` 常量，见 digest guard 诚实重算说明）。
    // BspPoint 投影（.0）：PanDiv 证书唯一真值源在 extract_signals_with_hist（生产 classify 直调它
    // 消费 .1；本简易/测试入口只投影结构 bit 点，非第二套真值）。
    extract_signals_with_hist(centers, segments, &hist, &[], &[], close_src).0
}

/// 力度离线入口（force-proxy-survey-20260702.md）：与 [`extract_signals`] 同产 BspPoint，但传真
/// dif/closes_tick ⟹ 一类趋势背驰候选的 `point.force` 算得 `Some`（A/C 段可配对，5 proxy）；第二/
/// 三类无 A/C 对 ⟹ `point.force=None`。供 W-VERIFY(#13) 多 proxy 交叉验证读 `p.force`。
///
/// **认识论 L1**（formalization-validity-domain 231号）：段坐标→proxy 是确定性算术（管线正确性）。
/// **不改 buy1 判据**（class_index 冻结，671 力度=feature 非 veto）：返回的 BspPoint 结构字段（六
/// bit/pivot/center/struct_break_dir）与 [`extract_signals`] 逐字段相同（`PartialEq` 排除 force），
/// force 是旁挂量，不进任何 bit 判定。**与生产热路径同接线**（beta-route #115 后，增量路径亦传真 dif）。
pub fn extract_signals_force(
    centers: &[Center],
    segments: &[Segment],
    closes: &[f64],
    close_src: &[usize],
    macd_cfg: &MacdConfig,
) -> Vec<BspPoint> {
    let series = compute_macd(closes, macd_cfg);
    // closes 是 merged_bars.close(Tick) 的 as f64（mod.rs:217），整值往返 as i64 精确（振幅=tick 差）。
    let closes_tick: Vec<Tick> = closes.iter().map(|&c| c as Tick).collect();
    // BspPoint 投影（.0，同 extract_signals 注）：PanDiv 真值源在 with_hist，生产路径消费 .1。
    extract_signals_with_hist(centers, segments, &series.hist, &series.dif, &closes_tick, close_src).0
}

/// 增量 MACD 接入点（231号纯性能，bit-exact 铁律）：与 [`extract_signals`] 同逻辑，但接受
/// 预计算 `hist`（由调用方增量产出，避免全量 `compute_macd` 重算）。这是**第一/三类买卖点提取的
/// core**（[`extract_signals`] 委托本函数，只差 hist 来源：全量 `compute_macd` vs 增量缓存）。
///
/// `hist` 须与 `compute_macd(closes, macd_cfg).hist` 逐元素 bit-identical（ac75d4b3 已证
/// `MacdState::append` bit-exact）。调用方负责 hist 的增量产出与缓存（见
/// `classify_with_tower_incremental` 的 `compute_macd_hist_incremental`）。
///
/// ★P2-R2（codex-decide-20260701-2121 → p2-plan §2）：破中枢结构候选（趋势 ∧ 破最后中枢 ∧ A/C 可
/// 配对）全部进样本（消选择偏差），背驰确认（C<A）置 buy1/sell1，未背驰置零 bit + `struct_break_dir`
/// （下游 candidate_dir 恢复方向）。**不再产 StructBreakFeature sidecar**（护栏7 死代码删除，macd_c_lt_a
/// 可从 BspPoint 派生，见模块头）。
pub fn extract_signals_with_hist(
    centers: &[Center],
    segments: &[Segment],
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
) -> (Vec<BspPoint>, Vec<PanDivCert>) {
    // L0 入口：线段有内在缠论方向 ⟹ 锚方向 ≡ 结构方向（域定理）。Q7-#1 裁定C 的锚门只约束
    // 级别-N fallback 单元（经 [`extract_signals_with_hist_anchored`] 传 provenance 派生锚）。
    extract_signals_with_hist_anchored(centers, segments, None, hist, dif, closes_tick, close_src)
}

/// ★Q7-#1 裁定C（codex-q7-fallback-20260703，收窄 #121 裁定A）：`anchor_dirs[i]` = `segments[i]`
/// 的**方向锚资格**——级别-N 传 `center_own_dir_at` ownership 方向（provenance 派生）；`None` 元素
/// = endpoint fallback 单元，保留为序列/区间/面积成员，但不得作 A 段候选、C 段/破中枢段、离开段
/// 方向锚。整参 `None` = L0（段方向即锚方向）。盘整背驰路径（[`judge_pan_div`]）不受锚门约束
/// （盘整是独立范畴，腿方向即结构方向——恒用 `anchors_self`）。
pub fn extract_signals_with_hist_anchored(
    centers: &[Center],
    segments: &[Segment],
    anchor_dirs: Option<&[Option<Direction>]>,
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
) -> (Vec<BspPoint>, Vec<PanDivCert>) {
    // ★线段按 start_index **稳定**升序排一次（生产路径 parser 线段账本本已 start_index 严格单调
    // 递增——流式 push 时 seg_start 单调推进，segment.rs:344-380——故排序对生产路径是恒等）。稳定
    // 排序保证 start_index 相同时保留原序。中枢归属用 start_index，单趟扫描需线段时间序。
    // ★O(1) 优化：检测已有序则直接借用引用（避免 O(k) clone+sort）。生产路径 + 测试合成数据均有序。
    let sorted_owned: Vec<Segment>;
    let anchors_perm: Vec<Option<Direction>>;
    let (sorted, anchors_in): (&[Segment], Option<&[Option<Direction>]>) =
        if segments.windows(2).all(|w| w[0].start_index <= w[1].start_index) {
            (segments, anchor_dirs)
        } else {
            // 稳定排序经下标置换——anchor_dirs 与 segments 平行数组，须同一置换（Q7-#1 裁定C）。
            let mut idx: Vec<usize> = (0..segments.len()).collect();
            idx.sort_by_key(|&i| segments[i].start_index);
            sorted_owned = idx.iter().map(|&i| segments[i].clone()).collect();
            match anchor_dirs {
                Some(a) => {
                    anchors_perm = idx.iter().map(|&i| a[i]).collect();
                    (&sorted_owned[..], Some(&anchors_perm[..]))
                }
                None => (&sorted_owned[..], None),
            }
        };
    // 锚资格平行数组（与 `sorted` 等长）：anchors_self = 结构方向（L0 语义 + 盘整背驰路径专用）。
    let anchors_self: Vec<Option<Direction>> = sorted.iter().map(|s| Some(s.direction)).collect();
    let anchors: &[Option<Direction>] = anchors_in.unwrap_or(&anchors_self);
    debug_assert_eq!(anchors.len(), sorted.len(), "anchor_dirs 与 segments 必等长");

    // ★中枢归属用 centers 按 end_index 升序（`detect_centers_with` 非重叠扫描已保证，mod.rs:136；
    // 公开函数不假设入参有序 ⟹ 本入口稳定排序，`nearest_confirmed_center` 二分前缀依赖升序）。
    let centers_owned: Vec<Center>;
    let centers_sorted: &[Center] = if centers.windows(2).all(|w| w[0].end_index <= w[1].end_index) {
        centers
    } else {
        centers_owned = {
            let mut v = centers.to_vec();
            v.sort_by_key(|c| c.end_index);
            v
        };
        &centers_owned
    };

    // ★局部趋势门（Q1/Q8 裁决，task #143）：τ 从「全历史累积链 AllTrend」（吸收锁死谓词，
    // PDF §2 + 外审漏斗坐实）换为「段当时所在走势类型块」。分解块前缀稳定（decompose.rs 模块头）
    // ⟹ 批式逐段查表 = 因果判定：段的最近已确认中枢 c 在其当时的当前块中恒为尾中枢；
    // gate[pos]=Some(d) ⟺ 该块为 Trend(d) 且 pos>块首（prev=pos-1 同块前驱，A 段所在，PDF §7）。
    // 盘整/扩张 run 内不产第一类（盘整背驰承接路由归 #145；「刚完成趋势块」时限窗口归 #144）。
    let blocks = decompose(centers_sorted);
    let center_gate = center_trend_gate(centers_sorted.len(), &blocks);
    let any_trend = center_gate.iter().any(|g| g.is_some());
    // ★Q4（task #145）：每中枢 ownership 块类别（与趋势门同一 decompose 单一来源，不 fork 第二套
    // 分解）——段的最近中枢落在 Consolidation 块 ⟹ 走盘整背驰证书路径（不产第一类 bit）。
    let center_kind = center_block_kind(centers_sorted.len(), &blocks);
    let any_consol = center_kind.iter().any(|k| *k == Some(MoveKind::Consolidation));

    // ★单趟扫描（消解旧 `for c in centers` 对前驱中枢重复产出，codex 裁决 2026-06-27）：每个线段端点
    // 只相对其**最近已确认中枢**（"当下之前最后一个中枢"，第18课定理三「该中枢」+ 第49课）判第一/三
    // 类**一次**，不对所有阈值更低/更高的前驱中枢重复认领。
    //
    // ★复杂度 O(S·logC + S·logS)（#93 性能工位修复后，profile 坐实近线性）：
    // - 每段 `nearest_confirmed_center_idx` 二分 O(logC) 定位归属中枢下标 + `first_match_idx` O(1)
    //   查表取首匹配 `pos`（替代旧 `.position()` O(C) 线性反查——热点①，exp≈1.98 已 profile 坐实，
    //   见 `scale_timing_position_hotspot`）。
    // - 第一类 A 段定位（`locate_trend_seg_a`）O(S) 线性扫，但**按 last_center_idx 缓存**——趋势 τ 下
    //   多段共享同一 `(prev_center, last_center)` 对 ⟹ A 段每对至多算一次，替代旧每段重算（热点②，
    //   `scale_timing_breaking_path` exp≈1.99 已 profile 坐实）。
    let mut points = Vec::new();

    // ★首匹配下标表（热点① bit-exact 修复，#93）：旧 `.position(|x| 三元组==c 三元组)` 在重复三元组下
    // 返回**首匹配**（最小下标），非 `nearest_confirmed_center_idx` 给出的前缀末下标。为严格 bit-exact
    // 保留旧语义，正向扫 `centers_sorted`、仅在 key 不存在时插入 ⟹ 保留首匹配下标。主循环 O(1) 查表取
    // `pos`（替代旧 O(C) 线性反查），`c`（last_center）仍取 `centers_sorted[c_idx]`（最近中枢语义正确）。
    // 建表 O(C)。
    // ★B1 性能（algo-opt-plan-20260702 泳道B）：唯一消费点在 :632 `if let Some(dir) = trend_dir` 分支内
    // ⟹ 非趋势 τ（trend_dir=None）永不查表。建表移入 trend_dir.is_some() 守卫——非趋势时略过 O(C) 建表 +
    // 分配（空表从不被 get 查询 ⟹ 逐位恒等 bit-exact；仅趋势 τ 才产第一类，非趋势无第一类路径）。
    let mut first_match_idx: std::collections::HashMap<(usize, Tick, Tick), usize> =
        std::collections::HashMap::new();
    if any_trend {
        first_match_idx.reserve(centers_sorted.len());
        for (idx, c) in centers_sorted.iter().enumerate() {
            // 仅在 key 不存在时插入 ⟹ 首匹配（最小 idx）胜出，与旧 `.position()` 逐位一致。
            first_match_idx.entry((c.end_index, c.zd, c.zg)).or_insert(idx);
        }
    }

    // ★A 段缓存（热点②修复，#93）：key = last_center_idx（趋势 τ 下 prev_center = centers_sorted[pos-1]
    // 由 c 三元组经 first_match_idx 唯一决定 ⟹ 同 c_idx 下 (pos-1, c_idx) 配对唯一），val =
    // `locate_trend_seg_a` 结果（与 C 段无关，仅依赖 `(segments, prev_center, last_center, trend_dir)`）。
    // 趋势 τ 必 ≥2 同向中枢 ⟹ 所有触发第一类的段的 last_center_idx ∈ [1, C-1]，缓存规模至多 C 条。
    // 每段查缓存 O(1)（HashMap），首次 miss 才调 O(S) 的 `locate_trend_seg_a`——总成本从 O(S²)（每段
    // 重算）降为 O(C·S + S)（每中枢对一次 + 每段查缓存），C ≪ S 时近线性。
    let mut a_seg_cache: std::collections::HashMap<usize, Option<(usize, usize)>> =
        std::collections::HashMap::new();
    // ★Q5 λ_C（codex ac4 #2 修复）：episode 语义下 λ_C 依赖 seg 前的回中枢段集合 ⟹ 不再可按
    // c_idx 缓存（旧 c_start_cache 无 reentry 检测，「失败离开→回中枢→重新离开」时 λ_C 过早污染
    // I(C) 面积）。逐段调 departure_move_c_start：O(窗口)（partition_point 定界 + 窗口线性扫），
    // 窗口 = c 之后至 seg 的段（生产量级中枢均段数 ~3，与 judge_pan_div 同成本类）。
    // ★Q4：盘整背驰证书收集（不产 BspPoint，不置 six-bit——承接路由在 econ 层）。
    let mut pan_divs: Vec<PanDivCert> = Vec::new();

    for (i, seg) in sorted.iter().enumerate() {
        // 该段归属的最近已确认中枢下标（"当下之前最后一个中枢"）。无 ⟹ 该段在所有中枢之前 ⟹ 非第一/三类。
        let Some(c_idx) = nearest_confirmed_center_idx(&centers_sorted, seg.start_index) else {
            continue;
        };
        let c = &centers_sorted[c_idx];

        // 第一类（趋势背驰，A/B/C 框架）：仅段所在趋势块 + 破最后中枢段触发（Q8：「最后一个
        // 中枢」= 当前走势类型的最后中枢 = c）；prev_center = pos-1（同块前驱，A 段所在）。
        // center_gate[pos]=Some ⟹ pos > 块首 ≥ 0 ⟹ 前驱存在且同块。
        if any_trend {
            // 热点①修复：`pos` 由 `first_match_idx` O(1) 查表给出（首匹配下标，bit-exact 等价旧
            // `.position()`），替代旧 `centers.iter().position(...)` O(C) 线性反查。`c`（last_center）
            // 仍取 `centers_sorted[c_idx]`（最近中枢语义正确）。
            if let Some(&pos) = first_match_idx.get(&(c.end_index, c.zd, c.zg)) {
                if let Some(dir) = center_gate[pos] {
                    let prev_center = &centers_sorted[pos - 1];
                    // 热点②修复：A 区间按 last_center_idx=c_idx 缓存（与 C 段 `seg` 无关，见 a_seg_cache
                    // 注释）。首次 miss 才调 `locate_departure_move_a` O(S)，后续 hit O(1) 复用——消解
                    // 每段重算的 O(S²)。
                    let a_seg_entry = *a_seg_cache
                        .entry(c_idx)
                        .or_insert_with(|| locate_departure_move_a(&sorted, anchors, prev_center, c, dir));
                    // Q5 λ_C（codex ac4 #2）：当前 episode 首同向段起点（共享 helper，逐段计算）。
                    let c_start_entry =
                        departure_move_c_start(&sorted, anchors, c, dir, seg.start_index);
                    // P2-R2：judge_first_cached 回 Option<BspPoint>——破中枢结构候选（背驰=buy1/未背驰
                    // =零 bit + struct_break_dir）进 points。macd_c_lt_a 不再单产 sidecar（护栏7 删除）。
                    if let Some(pf) = judge_first_cached(
                        c, dir, seg, anchors[i], hist, dif, closes_tick, close_src, a_seg_entry,
                        c_start_entry,
                    ) {
                        points.push(pf);
                    }
                }
            }
        }

        // ★Q4 盘整背驰（task #145）：段的最近中枢按 ownership 落在 Consolidation 块 ⟹ 走盘整
        // 背驰证书路径（同一中枢两次同向离开 + C<A）。不产 BspPoint、不置 six-bit——证书由
        // econ 层经 Nest/XZD 二通道承接（PanDiv^δ_ℓ ⟹ ∃e<ℓ Conf^δ_e ∨ XZD^δ_{ℓ↓e}）。
        if any_consol && center_kind[c_idx] == Some(MoveKind::Consolidation) {
            if let Some(cert) = judge_pan_div(c, seg, &sorted, &anchors_self, hist, close_src) {
                pan_divs.push(cert);
            }
        }

        // 第三类：当前段作 retest，前一段作 leave。中枢归属 = **leave 段离开的最近中枢**（第18课
        // 「该中枢」），故用 leave 段 start_index 定位中枢，retest 相对**同一**中枢判一次。
        if i > 0 {
            let leave_seg = &sorted[i - 1];
            if let Some(c_leave_idx) =
                nearest_confirmed_center_idx(&centers_sorted, leave_seg.start_index)
            {
                let c_leave = &centers_sorted[c_leave_idx];
                // Q7-#1 裁定C：leave 段（三类离开/突破段）须有方向锚资格；retest 段是几何回试角色。
                if let Some(p) = judge_third(c_leave, leave_seg, anchors[i - 1], seg) {
                    // 第三类无 A/C 趋势段对 ⟹ p.force=None（make_third_point 已置，诚实不造死字段）。
                    points.push(p);
                }
            }
        }
    }
    // 按 source_index 升序（reference:16 平局裁决键的时间序分量）。force 已收进各 BspPoint.force。
    points.sort_by_key(|p: &BspPoint| p.source_index);
    pan_divs.sort_by_key(|p: &PanDivCert| p.source_index);
    (points, pan_divs)
}

/// 一类判据链漏斗计数（#141 外审问题包诊断探针，仅 test 构建）。
///
/// 与 [`extract_signals_with_hist`] **同一批判据函数**逐环计数（675号 meta-rule：探针走生产路径，
/// 不另起坐标系）——nearest_confirmed_center_idx / trend_class / first_match_idx / broke 几何 /
/// locate_trend_seg_a / map_src_range_to_close_idx / AbcDivergence::diverges 全部原函数调用。
/// 尾部 parity 断言：漏斗幸存数须与生产提取输出逐一相等（分叉即 panic，不产生伪计数）。
#[cfg(test)]
pub(crate) struct Type1Funnel {
    /// 分解块统计（task #143：AllTrend τ 已删，改报趋势/盘整块数 + 最长趋势 run 跨中枢数）。
    pub n_trend_blocks: usize,
    pub n_consol_blocks: usize,
    pub longest_trend_run: usize,
    pub n_centers: usize,
    pub n_segments: usize,
    /// 环1：有「最近已确认中枢」的段数（候选评估入口）。
    pub s_with_center: usize,
    /// 环2：last_center 有前驱中枢（pos≥1，即 ≥2 中枢可配 A/B/C）。
    pub s_pos_ge1: usize,
    /// 环3：局部趋势门开（center_gate[pos]=Some，段所在块为 Trend 且 pos>块首）⊆ 环2。
    pub s_gate_open: usize,
    /// 环4：C 段破最后中枢几何成立（方向=趋势方向 ∧ 端点越 zd/zg）。
    pub s_broke: usize,
    /// 环5：A 段可配对（locate_trend_seg_a=Some）。
    pub s_a_paired: usize,
    /// 环6：A/C 段均可映射到 closes 坐标（=生产 struct_break 候选数）。
    pub s_mapped: usize,
    /// 环7：MACD 背驰 C<A 成立（=生产 buy1/sell1 数）。
    pub s_diverge: usize,
}

#[cfg(test)]
pub(crate) fn type1_funnel_dx(
    centers: &[Center],
    segments: &[Segment],
    anchor_dirs: Option<&[Option<Direction>]>,
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
) -> Type1Funnel {
    let blocks = decompose(centers);
    let center_gate = center_trend_gate(centers.len(), &blocks);
    let mut f = Type1Funnel {
        n_trend_blocks: blocks.iter().filter(|b| b.kind == MoveKind::Trend).count(),
        n_consol_blocks: blocks.iter().filter(|b| b.kind == MoveKind::Consolidation).count(),
        longest_trend_run: blocks
            .iter()
            .filter(|b| b.kind == MoveKind::Trend)
            .map(|b| b.end_center - b.start_center + 1)
            .max()
            .unwrap_or(0),
        n_centers: centers.len(),
        n_segments: segments.len(),
        s_with_center: 0,
        s_gate_open: 0,
        s_pos_ge1: 0,
        s_broke: 0,
        s_a_paired: 0,
        s_mapped: 0,
        s_diverge: 0,
    };
    let mut first_match_idx: std::collections::HashMap<(usize, Tick, Tick), usize> =
        std::collections::HashMap::new();
    for (idx, c) in centers.iter().enumerate() {
        first_match_idx.entry((c.end_index, c.zd, c.zg)).or_insert(idx);
    }
    let mut a_seg_cache: std::collections::HashMap<usize, Option<(usize, usize)>> =
        std::collections::HashMap::new();
    // Q7-#1 裁定C + 675号（探针走生产路径）：锚与生产同源——L0 传 None（自锚），级别-N 由
    // census 传 `center_own_dir_at` 派生锚。漏斗环判据与生产 extract 逐锚一致，parity 断言收口。
    let anchors_owned = super::divergence::self_anchors(segments);
    let anchors: &[Option<Direction>] = anchor_dirs.unwrap_or(&anchors_owned);
    for (si, seg) in segments.iter().enumerate() {
        let Some(c_idx) = nearest_confirmed_center_idx(centers, seg.start_index) else {
            continue;
        };
        f.s_with_center += 1;
        let c = &centers[c_idx];
        let Some(&pos) = first_match_idx.get(&(c.end_index, c.zd, c.zg)) else { continue };
        if pos < 1 {
            continue;
        }
        f.s_pos_ge1 += 1;
        let Some(dir) = center_gate[pos] else { continue };
        f.s_gate_open += 1;
        // broke 几何（judge_first_cached 同判据同顺序）。
        let end = seg_end(seg);
        // Q7-#1 裁定C：broke 锚门与 judge_first_cached 同判据（fallback ⟹ 不触发）。
        let broke = match (anchors[si], dir) {
            (Some(Direction::Down), Direction::Down) => end.price < c.zd,
            (Some(Direction::Up), Direction::Up) => c.zg < end.price,
            _ => false,
        };
        if !broke {
            continue;
        }
        f.s_broke += 1;
        let prev_center = &centers[pos - 1];
        let a = *a_seg_cache
            .entry(c_idx)
            .or_insert_with(|| locate_departure_move_a(segments, anchors, prev_center, c, dir));
        let Some((a_start, a_end)) = a else { continue };
        f.s_a_paired += 1;
        // Q5 λ_C（生产同款共享 helper，codex ac4 #2）：I(C)=[λ_C, seg.end]（broke ⟹ 必 Some）。
        let Some(lambda_c) = departure_move_c_start(segments, anchors, c, dir, seg.start_index) else {
            continue;
        };
        let (Some(c_i), Some(a_i)) = (
            map_src_range_to_close_idx(close_src, lambda_c, seg.end_index),
            map_src_range_to_close_idx(close_src, a_start, a_end),
        ) else {
            continue;
        };
        f.s_mapped += 1;
        let abc = AbcDivergence {
            seg_a: (a_start, a_end),
            seg_c: (lambda_c, seg.end_index),
            is_trend: true,
        };
        if abc.diverges(hist, a_i, c_i) {
            f.s_diverge += 1;
        }
    }
    // parity 守卫（675号：探针不分叉）——漏斗幸存数须与生产提取逐一相等。
    let (prod, _pan) =
        extract_signals_with_hist_anchored(centers, segments, anchor_dirs, hist, dif, closes_tick, close_src);
    let prod_t1 = prod.iter().filter(|p| p.bits.buy1 || p.bits.sell1).count();
    let prod_sb = prod.iter().filter(|p| p.struct_break_dir.is_some()).count();
    assert_eq!(prod_t1, f.s_diverge, "漏斗环7（背驰）须=生产 buy1/sell1 数");
    assert_eq!(prod_sb, f.s_mapped, "漏斗环6（mapped 候选）须=生产 struct_break 候选数");
    f
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

    // ── 第一类（A/B/C 趋势背驰框架：≥2 同向中枢 + A/C 跨相邻中枢 + τ 门控）──────
    //
    // ★语义升级（第24课:22-24 + maimai.md:103-112，消解 grammar-audit 头号缺口）：第一类 = 趋势
    //   背驰点，**前提 ≥2 依次同向中枢**（趋势 τ）。A=前中枢离开段、C=后中枢破中枢段，C<A 力度。
    //   单中枢（盘整）**不产第一类**（盘整背驰，beichi #4）——旧测试用单中枢产第一类是退化语义，
    //   已被 A/B/C 框架正确否决。下列测试构造下跌趋势（两依次向下中枢）的标准 A/B/C 结构。

    /// 下跌趋势中枢辅助：dd/gg 携全（趋势门控用 dd/gg 外缘判 c1.gg < c0.dd）。
    fn dc(zd: Tick, zg: Tick, dd: Tick, gg: Tick, ei: usize) -> Center {
        Center { zd, zg, dd, gg, start_index: 0, end_index: ei }
    }

    #[test]
    fn first_buy_extracted_with_trend_divergence() {
        // ★趋势背驰 1 买（A/B/C 框架）：两依次向下中枢（下跌趋势 τ=Trend(Down)）。
        // - C0[300,400] dd=290 gg=410 end=2（前中枢，A 段所在）。
        // - C1[100,200] dd=90  gg=210 end=8（后中枢，c1.gg=210 < c0.dd=290 ⟹ 下跌延续 ⟹ 趋势）。
        // - A 段 [3,5]：C0 之后向下离开段（端点破 C0 下沿 < 300），MACD 面积大（强势）。
        // - C 段 [9,11]：C1 之后向下破中枢段（端点 80 < C1.zd=100），MACD 面积小（背驰）⟹ 1 买。
        let c0 = dc(300, 400, 290, 410, 2);
        let c1 = dc(100, 200, 90, 210, 8);
        let segs = vec![
            seg(Direction::Down, 3, 5, 350, 250),   // A 段：C0 离开段（破 C0 下沿）
            seg(Direction::Up, 5, 7, 250, 280),     // B 段连接（中间反向，构成 C1）
            seg(Direction::Down, 9, 11, 150, 80),   // C 段：破 C1 下沿（< 100）∧ 背驰 ⟹ 1 买
        ];
        // closes（merged_bars 序列，与 segment 抽象端点价解耦——结构判定在 tick 域，MACD 在浮点域，
        // closes 是 merged_bars，segment 端点是抽象极值，两者不必逐 bar 一致）：A 段 bar [3,5] 急跌
        // （hist 面积大=强力度），C 段 bar [9,11] 缓动（hist 面积小=力度衰减=背驰）。手算 A area[3,5]=
        // 23.26 > C area[9,11]=9.09 ⟹ C<A 趋势背驰成立。
        let prices: Vec<Tick> = vec![
            300, 300, 300,        // 0..2 C0 区（预热）
            300, 100, 250,        // 3..5 A 段：急跌（hist 绝对值大=强力度）
            250, 250, 250,        // 6..8 B 段：盘整让 EMA 收敛（hist 回拉 0 轴）
            248, 246, 244,        // 9..11 C 段：缓动（hist 小=力度衰减=趋势背驰）
        ];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&[c0, c1], &segs, &closes, &src, &MacdConfig::default());
        let buy1: Vec<_> = points.iter().filter(|p| p.bits.buy1).collect();
        assert_eq!(buy1.len(), 1, "下跌趋势 C 段破最后中枢 ∧ C<A 趋势背驰 ⟹ 一个 1 买");
        assert_eq!(buy1[0].source_index, 11, "1 买端点 = C 段（破最后中枢段）终止 source_index");
        assert_eq!(buy1[0].pivot_low, 80, "1 买止损源 = pivot_low（C 段破中枢端点）");
        assert!(buy1[0].center.is_none(), "1 类止损用 pivot 非 center ⟹ center=None");
    }

    #[test]
    fn force_proxies_juxtaposed_on_first_class_candidate() {
        // ★Step1/2 力度并置（force-proxy-survey-20260702.md）：extract_signals_force 与 extract_signals
        // 同产 BspPoint，且一类趋势背驰候选并置 A/C 段 5 proxy。复用上测 fixture（C<A 背驰成立）。
        let c0 = dc(300, 400, 290, 410, 2);
        let c1 = dc(100, 200, 90, 210, 8);
        let segs = vec![
            seg(Direction::Down, 3, 5, 350, 250),
            seg(Direction::Up, 5, 7, 250, 280),
            seg(Direction::Down, 9, 11, 150, 80),
        ];
        let prices: Vec<Tick> = vec![
            300, 300, 300, 300, 100, 250, 250, 250, 250, 248, 246, 244,
        ];
        let (closes, src) = closes_seq(&prices);
        let pts = extract_signals_force(&[c0, c1], &segs, &closes, &src, &MacdConfig::default());

        // BspPoint 结构字段与 extract_signals 逐字段一致（PartialEq 排除 force ⟹ Some/None 不改相等）。
        let plain = extract_signals(&[c0, c1], &segs, &closes, &src, &MacdConfig::default());
        assert_eq!(pts, plain, "extract_signals_force 的 BspPoint 结构字段与 extract_signals 相同（force 旁挂）");

        // 一类候选（buy1）的 point.force = Some(ForceProxies)，且 C<A（背驰=力度衰减，第24课:24）。
        let b1 = pts.iter().find(|p| p.bits.buy1).expect("fixture 产一个 1 买");
        let force = b1.force.expect("一类趋势背驰候选 point.force = Some（A/C 段可配对）");
        assert_eq!(b1.source_index, 11);
        assert!(force.seg_c.macd_area < force.seg_a.macd_area, "C 段面积 < A 段面积（趋势背驰）");
        assert!(force.seg_a.dif_peak > 0.0 || force.seg_c.dif_peak > 0.0, "DIF 峰 proxy 已填充");
        assert!(force.seg_a.price_amplitude > 0, "A 段价格振幅 proxy 已填充（tick 域）");

        // 填充率报告（force 填充率 = Some / 一类候选数；三类候选 force=None 是诚实缺省非漏填）。
        let first_class = pts.iter().filter(|p| p.bits.buy1 || p.bits.sell1).count();
        let filled = pts.iter().filter(|p| (p.bits.buy1 || p.bits.sell1) && p.force.is_some()).count();
        eprintln!(
            "[force 填充率] 候选总数={} 一类={} force填充={} 填充率={:.0}%",
            pts.len(), first_class, filled,
            if first_class > 0 { 100.0 * filled as f64 / first_class as f64 } else { 0.0 }
        );
        assert_eq!(filled, first_class, "所有一类趋势背驰候选（A/C 可配对）point.force 均 Some");
    }

    #[test]
    fn first_buy_rejected_in_consolidation_tau_gate() {
        // ★τ 门控核心否决（消解假背驰=假买卖点）：**单中枢（盘整 τ=Consolidation）不产第一类**。
        // 同样的破中枢段 + 力度衰减，但只有 1 个中枢 ⟹ 盘整背驰（非趋势背驰）⟹ 第一类不产
        // （beichi #4 + maimai.md:56「盘整背驰不产第一类」已结算）。这正是退化实现的假买卖点来源。
        let c = dc(100, 200, 90, 210, 2);
        let segs = vec![
            seg(Direction::Down, 3, 5, 150, 90),  // 破中枢段（前）
            seg(Direction::Down, 6, 8, 120, 80),  // 破中枢段（后，面积更小）
        ];
        let prices: Vec<Tick> = vec![100, 100, 100, 100, 60, 140, 100, 95, 105];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&[c], &segs, &closes, &src, &MacdConfig::default());
        assert!(
            points.iter().all(|p| !p.bits.buy1),
            "单中枢=盘整 τ ⟹ 第一类不产（盘整背驰非趋势背驰，beichi #4）——τ 门控否决假买卖点"
        );
    }

    #[test]
    fn first_buy_rejected_when_segment_in_consolidation_block() {
        // ★局部趋势门（Q1/Q8，task #144）：段所在块为 **Consolidation**（链尾扩张关系）⟹ 第一类
        // 不产。链 = [Trend(Up) 0..1, Consolidation 1..2]——早期趋势块不为后续盘整块内的段开门
        // （门作用域=段当时所在块，非任意历史块；盘整背驰承接路由归 #145/Q4）。
        let c0 = dc(100, 200, 90, 210, 2);
        let c1 = dc(300, 400, 290, 410, 5);   // c0→c1 上涨
        let c2 = dc(350, 450, 250, 460, 8);   // c1→c2 扩张 ⟹ c2 落在盘整块（gate[2]=None）
        let segs = vec![
            seg(Direction::Down, 9, 11, 150, 80),  // 破 c2，但段所在块=Consolidation ⟹ 门关
        ];
        let prices: Vec<Tick> = vec![100, 100, 100, 100, 60, 140, 100, 95, 105, 150, 145, 80];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&[c0, c1, c2], &segs, &closes, &src, &MacdConfig::default());
        assert!(
            points.iter().all(|p| !p.bits.buy1),
            "段的最近中枢落在盘整块 ⟹ 局部趋势门关 ⟹ 第一类不产"
        );
    }

    #[test]
    fn first_buy_fires_after_early_overlap_no_global_lock_in() {
        // ★§10 不锁死性质（PDF §10，task #144 机器验证）：早期 overlap 关系 + 后续局部同向块
        // ⟹ 门开、第一类照产。旧 AllTrend 谓词下本链 ∃r_j=Overlap ⟹ 永久 Degenerate ⟹ 0
        // （吸收锁死，外审坐实 BTC 全历史第 1-17 天锁死 8.8 年）；分解门下早期历史不传导。
        let c_pre = dc(280, 420, 270, 430, 1); // 与 c0 overlap（非 GG/DD 完全分离）
        let c0 = dc(300, 400, 290, 410, 2);
        let c1 = dc(100, 200, 90, 210, 8);     // c0→c1 Down（GG=210 < DD(c0)=290）
        let centers = [c_pre, c0, c1];
        // §10 前件断言：链首关系确为 overlap（产盘整块），尾部为局部同向趋势块。
        let blocks = decompose(&centers);
        assert_eq!(
            blocks.iter().map(|b| b.kind).collect::<Vec<_>>(),
            vec![MoveKind::Consolidation, MoveKind::Trend],
            "前件：早期 overlap ⟹ 首块盘整；后续同向 ⟹ 尾块 Trend"
        );
        // fixture 同 force 测试的 buy1 见证：A 段深 V 大力度，C 段缓降破 zd(c1)=100 ⟹ 背驰。
        let segs = vec![
            seg(Direction::Down, 3, 5, 350, 250),
            seg(Direction::Up, 5, 7, 250, 280),
            seg(Direction::Down, 9, 11, 150, 80),
        ];
        let prices: Vec<Tick> =
            vec![300, 300, 300, 300, 100, 250, 250, 250, 250, 248, 246, 244];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&centers, &segs, &closes, &src, &MacdConfig::default());
        assert!(
            points.iter().any(|p| p.bits.buy1 && p.source_index == 11),
            "§10：早期 overlap 不锁死——尾部局部趋势块内破中枢背驰段照产第一类"
        );
    }

    #[test]
    fn first_buy_rejected_without_divergence() {
        // ★P2 语义纪律（codex-decide-20260701-2121）：MACD C<A 从 gate 降为 feature。趋势中破最后
        //   中枢但 **C 段力度未衰减**（C 段面积 ≥ A 段 ⟹ macd_c_lt_a==false）——不再被 MACD 预删，
        //   而是作为 **struct_break 候选进样本**（消选择偏差，下游 χ 可检验），但**不置 buy1**（buy1
        //   保严格「趋势背驰」语义，未背驰的破中枢不冒充第一类）。feature `macd_c_lt_a` 记 sidecar，
        //   **不进 BspPoint**（bit-exact 隔离，codex 风险2）。
        let c0 = dc(300, 400, 290, 410, 2);
        let c1 = dc(100, 200, 90, 210, 8);
        let segs = vec![
            seg(Direction::Down, 3, 5, 350, 250),   // A 段
            seg(Direction::Up, 5, 7, 250, 280),
            seg(Direction::Down, 9, 11, 150, 80),   // C 段：破中枢但力度延续
        ];
        // A 段小幅、C 段大幅 ⟹ C 面积 > A ⟹ 力度延续（非背驰）。
        let prices: Vec<Tick> = vec![
            300, 300, 300,
            300, 295, 305,        // 3..5 A 段：小幅（hist 小）
            250, 280, 260,
            150, 60, 240,         // 9..11 C 段：大幅（hist 大 ⟹ 力度延续 ⟹ 非背驰）
        ];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&[c0, c1], &segs, &closes, &src, &MacdConfig::default());
        // buy1 保严格语义：未背驰 ⟹ 不置 buy1（不冒充第一类）。
        assert!(points.iter().all(|p| !p.bits.buy1), "C 段力度延续 ⟹ 非趋势背驰 ⟹ buy1 不置位（保严格语义）");
        // 破中枢结构候选**仍产出**（进样本，消选择偏差）——source_index=11（C 段破中枢端点）。
        let sb_pt: Vec<_> = points.iter().filter(|p| p.source_index == 11).collect();
        assert_eq!(sb_pt.len(), 1,
            "struct_break 候选进 BspPoint 样本（零 buy1=Flat 候选，assemble_gamma 归 𝒦 不冒充第一类）"
        );
        // ★P2-R2（p2-plan §2）：该零 bit 候选带 struct_break_dir=Some(Long)——下游 candidate_dir 用它
        // 恢复 Long 方向进 μ 样本（消选择偏差）。买侧向下破 ⟹ Long。bit-vector 仍全零（class_index=0）。
        assert_eq!(sb_pt[0].struct_break_dir, Some(Side::Long),
            "零 bit 破中枢候选带 struct_break_dir=Some(Long)（下游 candidate_dir 恢复方向进样本，消选择偏差）");
        assert_eq!(sb_pt[0].bits.class_index(), 0,
            "六 bit 全零（未背驰不置 buy1）⟹ class_index=0——struct_break_dir 不进桶键（bit-exact 冻结）");
    }

    #[test]
    fn diverging_break_sets_buy1_and_struct_break_dir() {
        // ★P2-R2 对偶（背驰确认路径）：C 段力度衰减（C<A）⟹ 严格第一类（置 buy1）∧ struct_break_dir
        //   =Some(Long)。macd_c_lt_a=true 从产出派生（bits.buy1=true ⟺ C<A，护栏7 删 sidecar 后的等价）。
        let c0 = dc(300, 400, 290, 410, 2);
        let c1 = dc(100, 200, 90, 210, 8);
        let segs = vec![
            seg(Direction::Down, 3, 5, 350, 250),
            seg(Direction::Up, 5, 7, 250, 280),
            seg(Direction::Down, 9, 11, 150, 80),   // C 段：破 C1 下沿 ∧ 背驰 ⟹ 1 买
        ];
        let prices: Vec<Tick> = vec![
            300, 300, 300,
            300, 100, 250,        // A 段急跌（强力度）
            250, 250, 250,
            248, 246, 244,        // C 段缓动（弱力度=背驰）
        ];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&[c0, c1], &segs, &closes, &src, &MacdConfig::default());
        // 背驰 ⟹ 置 buy1（严格第一类）。macd_c_lt_a=true 从 bits.buy1=true 派生（护栏7 删 sidecar 等价）。
        let buy1: Vec<_> = points.iter().filter(|p| p.bits.buy1).collect();
        assert_eq!(buy1.len(), 1, "C<A 趋势背驰 ⟹ 一个严格第一类 1 买（buy1 置位 ⟺ macd_c_lt_a=true）");
        assert_eq!(buy1[0].source_index, 11);
        // ★P2-R2：背驰确认的破中枢点也带 struct_break_dir=Some(Long)——**无条件置**（不管背驰）。
        // 此点有 buy1 六 bit ⟹ candidate_dir 走 root_sel=Long，struct_break_dir 与之一致（不改结果，护栏1）。
        assert_eq!(buy1[0].struct_break_dir, Some(Side::Long),
            "破中枢方向无条件置（背驰确认路径同样 Some(Long)）——与 buy1 的 root_sel 方向一致");
    }

    /// ★P2-R2 验收（判据「|结构候选域|>|MACD-veto候选域|」的可机器验证形式）：同一 C≥A 破中枢输入下，
    /// 结构版 `extract_signals`（无 veto，破中枢候选全进样本）产出的候选**严格多于** MACD-veto oracle
    /// `extract_signals_orig`（`judge_first_orig` 的 `!diverges ⟹ return None`，pre-P2-R2 面积门语义）。
    /// 差集恰是被 MACD C≥A 预删、经 `struct_break_dir` 恢复的零 bit 破中枢候选（下游 χ 可否证 MACD）。
    #[test]
    fn struct_candidate_domain_strictly_supersets_macd_veto_domain() {
        let cfg = MacdConfig::default();
        // first_buy_rejected_without_divergence 的输入：趋势 ∧ 破最后中枢 ∧ C≥A（力度延续，非背驰）。
        let c0 = dc(300, 400, 290, 410, 2);
        let c1 = dc(100, 200, 90, 210, 8);
        let segs = vec![
            seg(Direction::Down, 3, 5, 350, 250),
            seg(Direction::Up, 5, 7, 250, 280),
            seg(Direction::Down, 9, 11, 150, 80),
        ];
        let prices: Vec<Tick> = vec![
            300, 300, 300,
            300, 295, 305,
            250, 280, 260,
            150, 60, 240,
        ];
        let (closes, src) = closes_seq(&prices);
        let structural = extract_signals(&[c0, c1], &segs, &closes, &src, &cfg);
        let veto = extract_signals_orig(&[c0, c1], &segs, &closes, &src, &cfg);
        // C≥A 破中枢候选：veto oracle 预删，结构版恢复进样本 ⟹ 结构候选域严格 ⊋ veto 候选域。
        assert!(
            structural.len() > veto.len(),
            "结构候选域 |{}| 须严格 > MACD-veto 候选域 |{}|（C≥A 破中枢候选被 veto 预删，结构版恢复进样本）",
            structural.len(),
            veto.len()
        );
        // 差集载体：带 struct_break_dir=Some ∧ 零 bit（class_index=0，未冒充第一类）的破中枢候选。
        let recovered: Vec<_> = structural
            .iter()
            .filter(|p| p.struct_break_dir.is_some() && p.bits.class_index() == 0)
            .collect();
        assert!(
            !recovered.is_empty(),
            "被恢复的候选带 struct_break_dir=Some ∧ 零 bit（未冒充第一类，class_index=0）"
        );
    }

    #[test]
    fn first_buy_rejected_without_broke_center() {
        // 趋势中 C 段未破最后中枢下沿（端点 120 ∈ [100,200]）⟹ 非第一类（几何不足）。
        let c0 = dc(300, 400, 290, 410, 2);
        let c1 = dc(100, 200, 90, 210, 8);
        let segs = vec![
            seg(Direction::Down, 3, 5, 350, 250),
            seg(Direction::Up, 5, 7, 250, 280),
            seg(Direction::Down, 9, 11, 150, 120), // 端点 120 >= zd=100，未破最后中枢
        ];
        let prices: Vec<Tick> = vec![300, 300, 300, 300, 200, 400, 250, 280, 260, 150, 145, 120];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&[c0, c1], &segs, &closes, &src, &MacdConfig::default());
        assert!(points.iter().all(|p| !p.bits.buy1), "C 段未破最后中枢 ⟹ 非第一类（几何分量不足）");
    }

    #[test]
    fn first_buy_rejected_without_prev_center_leave_segment() {
        // 趋势中无 A 段（前中枢 C0 与后中枢 C1 之间无向下离开段）⟹ A/C 无法配对 ⟹ 非第一类。
        let c0 = dc(300, 400, 290, 410, 2);
        let c1 = dc(100, 200, 90, 210, 8);
        let segs = vec![
            // C0(end=2) 与 C1(end=8) 之间只有向上段，无向下离开段 ⟹ A 段（向下）无候选。
            seg(Direction::Up, 3, 5, 250, 350),
            seg(Direction::Down, 9, 11, 150, 80),  // C 段：破最后中枢但无 A 段对照
        ];
        let prices: Vec<Tick> = vec![300, 300, 300, 250, 300, 350, 250, 280, 260, 150, 145, 80];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&[c0, c1], &segs, &closes, &src, &MacdConfig::default());
        assert!(points.iter().all(|p| !p.bits.buy1), "无前中枢离开段（A 段无法定位）⟹ 非第一类");
    }

    #[test]
    fn first_sell_mirror_with_trend_divergence() {
        // ★趋势背驰 1 卖镜像：两依次向上中枢（上涨趋势 τ=Trend(Up)）。
        // - C0[100,200] dd=90  gg=210 end=2（前中枢，A 段所在）。
        // - C1[300,400] dd=290 gg=410 end=8（后中枢，c1.dd=290 > c0.gg=210 ⟹ 上涨延续 ⟹ 趋势）。
        // - A 段 [3,5]：C0 之后向上离开段（端点 > C0.zg=200），面积大。
        // - C 段 [9,11]：C1 之后向上破中枢段（端点 420 > C1.zg=400），面积小（顶背驰）⟹ 1 卖。
        let c0 = dc(100, 200, 90, 210, 2);
        let c1 = dc(300, 400, 290, 410, 8);
        let segs = vec![
            seg(Direction::Up, 3, 5, 150, 250),     // A 段：C0 离开段（破 C0 上沿）
            seg(Direction::Down, 5, 7, 250, 280),   // B 段连接
            seg(Direction::Up, 9, 11, 350, 420),    // C 段：破 C1 上沿（> 400）∧ 背驰 ⟹ 1 卖
        ];
        // closes 镜像（A 段急涨强力度、C 段缓动弱力度=顶背驰）：A area[3,5]=23.26 > C area[9,11]=9.09。
        let prices: Vec<Tick> = vec![
            200, 200, 200,
            200, 400, 250,        // 3..5 A 段：急涨（hist 大=强力度）
            250, 250, 250,        // 6..8 B 段：盘整收敛 EMA
            252, 254, 256,        // 9..11 C 段：缓动（hist 小=顶背驰）
        ];
        let (closes, src) = closes_seq(&prices);
        let points = extract_signals(&[c0, c1], &segs, &closes, &src, &MacdConfig::default());
        let sell1: Vec<_> = points.iter().filter(|p| p.bits.sell1).collect();
        assert_eq!(sell1.len(), 1, "上涨趋势 C 段破最后中枢上沿 ∧ 顶背驰 ⟹ 一个 1 卖");
        assert_eq!(sell1[0].pivot_high, 420, "1 卖止损源 = pivot_high（C 段破中枢端点）");
        assert!(sell1[0].center.is_none(), "1 类止损用 pivot 非 center");
    }

    // ── Q5（task #145）：A/C 从单段升级为次级别走势区间——多段 departure 语义真变 ────

    /// Q5 A 侧语义变更见证：A 由 2 个同向段构成（中间夹反向段）。旧单段口径（A=末个匹配段
    /// [7,9]）不判背驰（C 面积 ≥ 旧 A 面积）；新区间口径（I(A)=[3,9] 覆盖全离开走势）判背驰
    /// ⟹ buy1 置位。前置条件在测试内用同一 `segment_macd_area` 自证（fixture 不靠魔数）。
    #[test]
    fn q5_multi_segment_departure_a_interval_flips_divergence() {
        use super::super::divergence::segment_macd_area;
        let c0 = dc(300, 400, 290, 410, 2);
        let c1 = dc(100, 200, 90, 210, 8);
        let segs = vec![
            seg(Direction::Down, 3, 5, 350, 250), // A 腿1（深）
            seg(Direction::Up, 5, 7, 250, 280),   // A 内部反向段
            seg(Direction::Down, 7, 9, 280, 240), // A 腿2（浅，旧口径的"末段 A"）
            seg(Direction::Down, 13, 15, 150, 80), // C：破 c1.zd=100
        ];
        let prices: Vec<Tick> = vec![
            300, 300, 300, 300, 200, 260, 262, 264, 263, 262, 262, 262, 262, 240, 215, 190,
        ];
        let (closes, src) = closes_seq(&prices);
        let hist = compute_macd(&closes, &MacdConfig::default()).hist;
        // 前置自证：area(旧A=[7,9]) < area(C=[13,15]) < area(新A=[3,9])——旧口径不背驰、新口径背驰。
        let (a_old, c_area, a_new) = (
            segment_macd_area(&hist, 7, 9),
            segment_macd_area(&hist, 13, 15),
            segment_macd_area(&hist, 3, 9),
        );
        assert!(a_old < c_area, "前置：旧单段 A 面积({a_old:.3}) < C 面积({c_area:.3})（旧口径不判背驰）");
        assert!(c_area < a_new, "前置：C 面积({c_area:.3}) < 新区间 A 面积({a_new:.3})（新口径判背驰）");
        // 生产输出：新区间口径 ⟹ 背驰成立 ⟹ buy1（旧口径下此点是零 bit struct_break 候选）。
        let points = extract_signals(&[c0, c1], &segs, &closes, &src, &MacdConfig::default());
        let buy1: Vec<_> = points.iter().filter(|p| p.bits.buy1).collect();
        assert_eq!(buy1.len(), 1, "Q5：I(A) 覆盖全离开走势区间 ⟹ 背驰成立 ⟹ buy1（语义真变，非兼容重构）");
        assert_eq!(buy1[0].source_index, 15);
    }

    /// Q5 C 侧语义变更见证（反向）：C 由 2 个同向段构成。旧单段口径（C=破中枢段 [18,20]）判
    /// 背驰；新区间口径（I(C)=[λ_C=13, 20] 覆盖全离开走势）不判 ⟹ 零 bit struct_break 候选。
    #[test]
    fn q5_multi_segment_departure_c_interval_flips_divergence() {
        use super::super::divergence::segment_macd_area;
        let c0 = dc(300, 400, 290, 410, 2);
        let c1 = dc(100, 200, 90, 210, 8);
        let segs = vec![
            seg(Direction::Down, 3, 5, 350, 250),  // A（单段）
            seg(Direction::Up, 5, 7, 250, 280),
            seg(Direction::Down, 13, 15, 180, 90), // C 腿1：破 zd=100（候选点，episode 起点 λ_C=13）
            seg(Direction::Up, 16, 17, 90, 96),    // C 内部反向段（end 96 < zd=100 未回核心 ⟹ 同一 episode）
            seg(Direction::Down, 18, 20, 96, 80),  // C 腿2：再破（因果触发段，I(C)=[13,20]）
        ];
        let prices: Vec<Tick> = vec![
            300, 300, 300, 300, 262, 250, 265, 275, 280, 282, 283, 284, 284, 250, 200, 150,
            230, 280, 279, 278, 277,
        ];
        let (closes, src) = closes_seq(&prices);
        let hist = compute_macd(&closes, &MacdConfig::default()).hist;
        // 前置自证：area(旧C=[18,20]) < area(A=[3,5]) < area(新C 腿1=[13,15] ≤ [13,20])——
        // 旧口径在 20 判背驰、新区间口径两个候选点（15/20）都不判。
        let (c_old, a_area, c_leg1) = (
            segment_macd_area(&hist, 18, 20),
            segment_macd_area(&hist, 3, 5),
            segment_macd_area(&hist, 13, 15),
        );
        assert!(c_old < a_area, "前置：旧单段 C 面积({c_old:.3}) < A 面积({a_area:.3})（旧口径判背驰）");
        assert!(a_area < c_leg1, "前置：A 面积({a_area:.3}) < 新区间 C 面积下界({c_leg1:.3})（新口径不判背驰）");
        let points = extract_signals(&[c0, c1], &segs, &closes, &src, &MacdConfig::default());
        assert!(points.iter().all(|p| !p.bits.buy1),
            "Q5：I(C) 覆盖全离开走势区间 ⟹ 力度未衰减 ⟹ buy1 不置位（语义真变）");
        // 破中枢结构候选仍进样本（P2-R2 消选择偏差）：零 bit + struct_break_dir=Some(Long)。
        let sb: Vec<_> = points
            .iter()
            .filter(|p| p.source_index == 20 && p.struct_break_dir == Some(Side::Long))
            .collect();
        assert_eq!(sb.len(), 1, "未背驰破中枢候选仍产零 bit struct_break 点");
        assert_eq!(sb[0].bits.class_index(), 0);
    }

    /// codex ac4 #2 修复见证（C 侧 reentry）：失败离开 [9,11]（未破 zd=100）→ 回中枢段 [11,13]
    /// （端点 150 回到核心内侧 ≥ zd）→ 重新离开 [13,15] 破中枢。λ_C = 当前 episode 起点 13（非旧
    /// 口径 9）——I(C)=[13,15] 面积 < A < 桥接区间 [9,15] 面积 ⟹ episode 口径判背驰 buy1；桥接
    /// 口径不判（前置在测试内自证）。
    #[test]
    fn q5_c_side_reentry_episode_bounds_lambda_c() {
        use super::super::divergence::{departure_move_c_start, segment_macd_area};
        let c0 = dc(300, 400, 290, 410, 2);
        let c1 = dc(100, 200, 90, 210, 8);
        let segs = vec![
            seg(Direction::Down, 3, 5, 350, 250),  // A
            seg(Direction::Up, 5, 7, 250, 280),
            seg(Direction::Down, 9, 11, 190, 120), // 失败离开（end 120 > zd=100 未破）
            seg(Direction::Up, 11, 13, 120, 150),  // 回中枢段（end 150 ≥ zd=100）
            seg(Direction::Down, 13, 15, 150, 80), // 重新离开：破 zd（因果触发段）
        ];
        let prices: Vec<Tick> = vec![
            300, 300, 300, 300, 220, 200, 230, 260, 265, 250, 235, 240, 265, 266, 265, 264,
        ];
        let (closes, src) = closes_seq(&prices);
        let hist = compute_macd(&closes, &MacdConfig::default()).hist;
        // λ_C 直接锁定（共享 helper 单一来源）。
        assert_eq!(departure_move_c_start(&segs, &super::super::divergence::self_anchors(&segs), &c1, Direction::Down, 13), Some(13),
            "回中枢段之后 ⟹ λ_C = 当前 episode 首同向段起点");
        // 前置自证：episode I(C) < A < 桥接 [9,15]——修复前后布尔翻转的见证条件。
        let (c_ep, a_area, c_bridged) = (
            segment_macd_area(&hist, 13, 15),
            segment_macd_area(&hist, 3, 5),
            segment_macd_area(&hist, 9, 15),
        );
        assert!(c_ep < a_area, "前置：episode C 面积({c_ep:.3}) < A 面积({a_area:.3})");
        assert!(a_area < c_bridged, "前置：A 面积({a_area:.3}) < 桥接 C 面积({c_bridged:.3})（旧桥接口径不判背驰）");
        let points = extract_signals(&[c0, c1], &segs, &closes, &src, &MacdConfig::default());
        let buy1: Vec<_> = points.iter().filter(|p| p.bits.buy1).collect();
        assert_eq!(buy1.len(), 1, "λ_C=episode 起点 ⟹ I(C) 面积衰减可见 ⟹ buy1（reentry 修复语义见证）");
        assert_eq!(buy1[0].source_index, 15);
    }

    /// codex ac4 #3：C 侧单段兼容独立锁定——中枢后恰一个同向离开段（无回中枢段）⟹ λ_C =
    /// seg.start_index ⟹ I(C) = 旧单段区间，行为与旧口径 bit 相同（不依赖「全局第一个匹配段」
    /// 之外的隐式前提——本 fixture 中该段即首匹配段，且 helper 输出被显式锁定）。
    #[test]
    fn q5_c_side_single_segment_bit_compatible() {
        use super::super::divergence::{departure_move_c_start, segment_macd_area};
        let c0 = dc(300, 400, 290, 410, 2);
        let c1 = dc(100, 200, 90, 210, 8);
        let segs = vec![
            seg(Direction::Down, 3, 5, 350, 250), // A（单段）
            seg(Direction::Up, 5, 7, 250, 280),
            seg(Direction::Down, 9, 11, 280, 80), // C：唯一离开段，破 zd=100
        ];
        let prices: Vec<Tick> = vec![300, 300, 300, 300, 180, 170, 250, 300, 300, 298, 296, 294];
        let (closes, src) = closes_seq(&prices);
        let hist = compute_macd(&closes, &MacdConfig::default()).hist;
        assert_eq!(departure_move_c_start(&segs, &super::super::divergence::self_anchors(&segs), &c1, Direction::Down, 9), Some(9),
            "单段离开 ⟹ λ_C = seg.start_index（与旧单段口径 bit 相同）");
        let (c_area, a_area) = (segment_macd_area(&hist, 9, 11), segment_macd_area(&hist, 3, 5));
        assert!(c_area < a_area, "前置：C 面积({c_area:.3}) < A 面积({a_area:.3})（背驰成立）");
        let points = extract_signals(&[c0, c1], &segs, &closes, &src, &MacdConfig::default());
        let buy1: Vec<_> = points.iter().filter(|p| p.bits.buy1).collect();
        assert_eq!(buy1.len(), 1, "单段 C：与旧口径同判 buy1");
        assert_eq!(buy1[0].source_index, 11);
    }

    // ── Q4（task #145）：盘整背驰证书（PanDivCert）——盘整块内破中枢+背驰，零一类 bit ────

    /// Q4 正例：段的最近中枢落在 Consolidation 块（沿用 first_buy_rejected_when_segment_in_
    /// consolidation_block 的中枢链）+ 同一中枢两次同向离开（A 破核心 → 回中枢段 → C 破核心）
    /// + C<A 背驰 ⟹ 产 PanDivCert；**零 BspPoint 一类 bit**（盘整背驰不冒充 B1/S1）。
    #[test]
    fn pan_div_cert_emitted_in_consolidation_block_zero_first_class_bits() {
        use super::super::divergence::segment_macd_area;
        let c0 = dc(100, 200, 90, 210, 2);
        let c1 = dc(300, 400, 290, 410, 5);  // c0→c1 上涨（趋势块）
        let c2 = dc(350, 450, 250, 460, 8);  // c1→c2 扩张 ⟹ c2 按 ownership 落盘整块
        let segs = vec![
            seg(Direction::Down, 9, 11, 460, 330),  // A：第一次离开（端点 330 < c2.zd=350 破核心）
            seg(Direction::Up, 11, 13, 330, 380),   // 回中枢段（端点 380 ≥ 350 回到核心内侧）
            seg(Direction::Down, 13, 15, 380, 300), // C：第二次离开（端点 300 < 350 破核心）
        ];
        let prices: Vec<Tick> = vec![
            100, 100, 100, 100, 60, 140, 100, 95, 105, 105, 60, 90, 95, 93, 91, 89,
        ];
        let (closes, src) = closes_seq(&prices);
        let hist = compute_macd(&closes, &MacdConfig::default()).hist;
        // 前置自证：C 面积 < A 面积（盘整背驰 Weak 成立）。
        let (a_area, c_area) = (segment_macd_area(&hist, 9, 11), segment_macd_area(&hist, 13, 15));
        assert!(c_area < a_area, "前置：C 面积({c_area:.3}) < A 面积({a_area:.3})");
        let (points, pan) =
            extract_signals_with_hist(&[c0, c1, c2], &segs, &hist, &[], &[], &src);
        // 零一类 bit（盘整块内不产第一类——门关；盘整背驰不冒充 B1/S1）。
        assert!(points.iter().all(|p| !p.bits.buy1 && !p.bits.sell1),
            "盘整块内零 buy1/sell1（盘整背驰不冒充同级第一类）");
        // 恰一张证书，字段逐一锁定。
        assert_eq!(pan.len(), 1, "同一中枢两次同向离开 + C<A ⟹ 恰一张 PanDivCert");
        let cert = &pan[0];
        assert_eq!(cert.source_index, 15, "因果触发点 = 破中枢段端点");
        assert_eq!(cert.side, Side::Long, "向下破 ⟹ Long 候选");
        assert_eq!((cert.center.zd, cert.center.zg), (350, 450), "证书携同一中枢（两次离开的 B）");
        assert_eq!(cert.seg_a, (9, 11), "A = 前一次同向离开末段");
        assert_eq!(cert.seg_c, (13, 15), "I(C) = 当前离开走势区间（Q5 同款区间语义）");
    }

    /// Q4×Q5 对称（codex ac4-r2）：前一次离开为**多段 episode**（破核心腿 + 未回核心的中间反向
    /// 段 + 再破更深腿）⟹ I(A) = 整个 episode 区间 [λ_A, ρ_A]，非仅末段锚。
    #[test]
    fn pan_div_a_side_multi_segment_episode_interval() {
        use super::super::divergence::segment_macd_area;
        let c0 = dc(100, 200, 90, 210, 2);
        let c1 = dc(300, 400, 290, 410, 5);
        let c2 = dc(350, 450, 250, 460, 8); // 扩张 ⟹ ownership 盘整块
        let segs = vec![
            seg(Direction::Down, 9, 11, 460, 330),  // A 腿1：破核心（330 < zd=350）
            seg(Direction::Up, 11, 12, 330, 340),   // A 内部反向段（340 < 350 未回核心 ⟹ 同 episode）
            seg(Direction::Down, 12, 13, 340, 300), // A 腿2：再破（episode 锚）
            seg(Direction::Up, 13, 15, 300, 380),   // 回中枢段（380 ≥ 350，A/C 分隔）
            seg(Direction::Down, 15, 17, 380, 295), // C：第二次离开破核心
        ];
        let prices: Vec<Tick> = vec![
            100, 100, 100, 100, 60, 140, 100, 95, 105, 40, 45, 35, 30, 25, 60, 90, 88, 86,
        ];
        let (closes, src) = closes_seq(&prices);
        let hist = compute_macd(&closes, &MacdConfig::default()).hist;
        let (a_area, c_area) = (segment_macd_area(&hist, 9, 13), segment_macd_area(&hist, 15, 17));
        assert!(c_area < a_area, "前置：C 面积({c_area:.3}) < A episode 面积({a_area:.3})");
        let (_points, pan) = extract_signals_with_hist(&[c0, c1, c2], &segs, &hist, &[], &[], &src);
        assert_eq!(pan.len(), 1, "多段前次离开 + 回中枢 + 再破 + C<A ⟹ 恰一张证书");
        assert_eq!(pan[0].seg_a, (9, 13), "I(A) = 前次离开整个 episode 区间（λ_A=腿1起点, ρ_A=腿2终点）");
        assert_eq!(pan[0].seg_c, (15, 17), "I(C) = 当前离开区间");
    }

    /// Q4 负例：无回中枢段（两 Down 段之间的反向段未回到核心内侧）⟹ 同一次离开 ⟹ 无 A/C
    /// 两次离开结构 ⟹ 不产证书（诚实不产，非放宽判据）。
    #[test]
    fn pan_div_rejected_without_reentry_between_departures() {
        let c0 = dc(100, 200, 90, 210, 2);
        let c1 = dc(300, 400, 290, 410, 5);
        let c2 = dc(350, 450, 250, 460, 8);
        let segs = vec![
            seg(Direction::Down, 9, 11, 460, 330),
            seg(Direction::Up, 11, 13, 330, 340), // 反弹端点 340 < c2.zd=350：未回中枢
            seg(Direction::Down, 13, 15, 340, 300),
        ];
        let prices: Vec<Tick> = vec![
            100, 100, 100, 100, 60, 140, 100, 95, 105, 105, 60, 90, 95, 93, 91, 89,
        ];
        let (closes, src) = closes_seq(&prices);
        let hist = compute_macd(&closes, &MacdConfig::default()).hist;
        let (_points, pan) =
            extract_signals_with_hist(&[c0, c1, c2], &segs, &hist, &[], &[], &src);
        assert!(pan.is_empty(), "无回中枢段 ⟹ 同一次离开 ⟹ 无盘整背驰证书");
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
#[test]
fn diag_first_buy() {
    use super::super::divergence::{compute_macd, segment_macd_area, locate_departure_move_a};
    use super::super::decompose::decompose;
    let c0 = Center { zd:300, zg:400, dd:290, gg:410, start_index:0, end_index:2 };
    let c1 = Center { zd:100, zg:200, dd:90, gg:210, start_index:0, end_index:8 };
    println!("blocks = {:?}", decompose(&[c0, c1]));
    let prices: Vec<Tick> = vec![300,300,300,300,200,400,250,280,260,150,145,155];
    let closes: Vec<f64> = prices.iter().map(|&v| v as f64).collect();
    let src: Vec<usize> = (0..prices.len()).collect();
    let hist = compute_macd(&closes, &MacdConfig::default()).hist;
    let segs = vec![
        Segment{direction:Direction::Down,start_index:3,end_index:5,start_price:350,end_price:250},
        Segment{direction:Direction::Up,start_index:5,end_index:7,start_price:250,end_price:280},
        Segment{direction:Direction::Down,start_index:9,end_index:11,start_price:150,end_price:80},
    ];
    let a = locate_departure_move_a(&segs, &super::super::divergence::self_anchors(&segs), &c0, &c1, Direction::Down);
    println!("A seg = {:?}", a);
    println!("A area [3,5] = {}", segment_macd_area(&hist, 3, 5));
    println!("C area [9,11] = {}", segment_macd_area(&hist, 9, 11));
    println!("hist = {:?}", hist.iter().map(|h| (h*100.0).round()/100.0).collect::<Vec<_>>());
}

    // ── ★性能工位（#93）：O(n²) 双热点 profile + bit-exact 前后对拍 ────────────────────
    //
    // 守卫 signal.rs 两独立 O(n²) 热点（memory signal-on2-two-independent-hotspots）：
    // ① type3 前驱重复（已由单趟扫描 + nearest_confirmed_center 二分在前序修复解决）；
    // ② `.position()` O(S·C) 反查（热点①，map 线性扫）+ `locate_trend_seg_a` 每段重算 O(S²)（热点②）。
    // 本节实测标度（exp 修复前≈2，修复后≈1）+ bit-exact 前后对拍（FNV 摘要逐元素相等）。

    /// FNV-1a over Debug 串（bit-exact 前后对拍的确定性摘要，覆盖 BspPoint 全字段）。
    fn fnv1a(s: &str) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in s.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    }

    /// 严格下跌趋势的 n 个中枢（全链 DownContinuation：next.gg < prev.dd ⟹ trend_class=Trend(Down)
    /// ⟹ trend_dir=Some ⟹ 主循环 `.position()` 触发）。
    fn down_trend_centers(n: usize) -> Vec<Center> {
        (0..n)
            .map(|j| {
                let dd = ((n - j) as Tick) * 100;
                Center { zd: dd + 10, zg: dd + 50, dd, gg: dd + 60, start_index: 0, end_index: 2 * j }
            })
            .collect()
    }

    /// n_seg 条向上线段，start_index 全在所有中枢 end_index 之后（每段最近中枢=最后一个 ⟹ 旧
    /// `.position()` 全扫 C 元素 ⟹ O(S·C)）。向上段在下跌趋势中 broke=false ⟹ judge_first 早退
    /// （不触 locate_trend_seg_a），隔离 `.position()` 为唯一非平凡成本。
    fn up_segs_after(n_seg: usize, n_center: usize) -> Vec<Segment> {
        let base = 2 * n_center;
        (0..n_seg)
            .map(|k| Segment {
                direction: Direction::Up,
                start_index: base + 2 * k,
                end_index: base + 2 * k + 1,
                start_price: 500,
                end_price: 600,
            })
            .collect()
    }

    /// 确定性 LCG 中枢+线段+closes（趋势/破中枢/type3 概率覆盖，bit-exact 电池用）。
    fn lcg_signal_input(
        n_seg: usize,
        n_center: usize,
        seed: u64,
    ) -> (Vec<Center>, Vec<Segment>, Vec<f64>, Vec<usize>) {
        let mut st = seed ^ 0x9E37_79B9_7F4A_7C15;
        let mut nxt = || {
            st = st
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (st >> 33) as i64
        };
        let mut centers = Vec::with_capacity(n_center);
        let mut base: i64 = 200;
        for j in 0..n_center {
            base += (nxt() % 80) - 40;
            let zd = base;
            let zg = base + 30 + (nxt() % 40);
            centers.push(Center { zd, zg, dd: zd - 20, gg: zg + 20, start_index: 0, end_index: 2 * j });
        }
        let mut segments = Vec::with_capacity(n_seg);
        for k in 0..n_seg {
            let dir = if k % 2 == 0 { Direction::Down } else { Direction::Up };
            let ep = 100 + (nxt() % 200);
            segments.push(Segment {
                direction: dir,
                start_index: 2 * k,
                end_index: 2 * k + 1,
                start_price: 200,
                end_price: ep,
            });
        }
        let n_close = n_seg * 2 + 2;
        let mut closes = Vec::with_capacity(n_close);
        let mut cs = Vec::with_capacity(n_close);
        let mut p = 200.0_f64;
        for t in 0..n_close {
            p += ((nxt() % 100) - 50) as f64 * 0.1;
            closes.push(p);
            cs.push(t);
        }
        (centers, segments, closes, cs)
    }

    /// ★仅测试用：`extract_signals` 的**朴素无缓存参考实现**（#93 bit-exact 对拍的 oracle）。
    /// 结构承自 git HEAD b8667b968f（旧 `.position()` O(C) 反查 + 每段重算 A 定位），A/C 判据与
    /// 生产同步升级为 Q5 区间语义（`locate_departure_move_a` + 每段朴素重扫 λ_C，无缓存）——oracle
    /// 的职责是守卫 first_match_idx/a_seg_cache 两缓存优化的透明性（同 #143 换局部
    /// 趋势门先例：oracle 门与生产同步，非语义快照）。
    fn judge_first_orig(
        prev_center: &Center,
        last_center: &Center,
        trend_dir: Direction,
        seg: &Segment,
        segments: &[Segment],
        hist: &[f64],
        src_to_idx: &[usize],
    ) -> Option<BspPoint> {
        let end = seg_end(seg);
        let (broke, is_sell) = match (end.dir, trend_dir) {
            (Direction::Down, Direction::Down) if end.price < last_center.zd => (true, false),
            (Direction::Up, Direction::Up) if last_center.zg < end.price => (true, true),
            _ => (false, false),
        };
        if !broke {
            return None;
        }
        let Some((a_start, a_end)) =
            locate_departure_move_a(segments, &super::super::divergence::self_anchors(segments), prev_center, last_center, trend_dir)
        else {
            return None;
        };
        // Q5 λ_C（oracle 与生产同一共享 helper——codex ac4 #4：无平行实现 ⟹ 无坐标 fork）。
        let Some(lambda_c) =
            departure_move_c_start(segments, &super::super::divergence::self_anchors(segments), last_center, trend_dir, seg.start_index)
        else {
            return None;
        };
        let (Some(c_idx), Some(a_idx)) = (
            map_src_range_to_close_idx(src_to_idx, lambda_c, seg.end_index),
            map_src_range_to_close_idx(src_to_idx, a_start, a_end),
        ) else {
            return None;
        };
        let abc = AbcDivergence { seg_a: (a_start, a_end), seg_c: (lambda_c, seg.end_index), is_trend: true };
        if !abc.diverges(hist, a_idx, c_idx) {
            return None;
        }
        let situ = EndpointSituation {
            after_first_buy: false,
            is_pullback_end: false,
            left_center: false,
            retrace_not_reenter: false,
            below_last_center: true,
            is_sell_side: is_sell,
        };
        let bits = endpoint_to_bsp(&situ);
        // oracle 是 veto 语义（只产背驰确认第一类）——这些点必破中枢，struct_break_dir 同新版逻辑。
        let struct_break_dir = Some(if is_sell { Side::Short } else { Side::Long });
        // oracle 不算力度（前 force 时代实现）⟹ force=None，与优化版空-dif 电池路径逐字段相等。
        Some(make_first_point(end.source_index, bits, end.price, struct_break_dir, None))
    }

    /// ★仅测试用：优化前 `extract_signals`（oracle，逐字从 git HEAD 复制）。
    fn extract_signals_orig(
        centers: &[Center],
        segments: &[Segment],
        closes: &[f64],
        close_src: &[usize],
        macd_cfg: &MacdConfig,
    ) -> Vec<BspPoint> {
        let hist = compute_macd(closes, macd_cfg).hist;
        let mut sorted: Vec<Segment> = segments.to_vec();
        sorted.sort_by_key(|s| s.start_index);
        let mut centers_sorted: Vec<Center> = centers.to_vec();
        centers_sorted.sort_by_key(|c| c.end_index);
        // task #143：oracle 门与生产同步换局部趋势门（本 oracle 守卫热点①②的 O(n) 优化等价性，
        // 非门语义快照——保留 naive .position() 反查与每段 locate_trend_seg_a 重算）。
        let blocks = decompose(&centers_sorted);
        let center_gate = center_trend_gate(centers_sorted.len(), &blocks);
        let any_trend = center_gate.iter().any(|g| g.is_some());
        let mut points = Vec::new();
        for (i, seg) in sorted.iter().enumerate() {
            let center_for_seg = nearest_confirmed_center(&centers_sorted, seg.start_index);
            if let Some(c) = center_for_seg {
                if any_trend {
                    let last_pos = centers_sorted
                        .iter()
                        .position(|x| x.end_index == c.end_index && x.zd == c.zd && x.zg == c.zg);
                    if let Some(pos) = last_pos {
                        if let Some(dir) = center_gate[pos] {
                            let prev_center = &centers_sorted[pos - 1];
                            if let Some(p) = judge_first_orig(
                                prev_center, c, dir, seg, &sorted, &hist, close_src,
                            ) {
                                points.push(p);
                            }
                        }
                    }
                }
                if i > 0 {
                    let leave_seg = &sorted[i - 1];
                    if let Some(c_leave) = nearest_confirmed_center(&centers_sorted, leave_seg.start_index)
                    {
                        if let Some(p) = judge_third(c_leave, leave_seg, Some(leave_seg.direction), seg) {
                            points.push(p);
                        }
                    }
                }
            }
        }
        points.sort_by_key(|p| p.source_index);
        points
    }

    /// bit-exact 前后对拍电池（#93，no-patch）：固定多类输入 ⟹ extract_signals 全字段 FNV 摘要。
    fn bit_exact_battery_digest() -> u64 {
        let cfg = MacdConfig::default();
        let mut acc = String::new();
        // (a) 趋势 `.position()` 路径：下跌趋势 + 向上非破段（隔离 `.position()`）。
        for &(c, s) in &[(8usize, 16usize), (20, 40), (12, 60)] {
            let pts = extract_signals(&down_trend_centers(c), &up_segs_after(s, c), &[], &[], &cfg);
            acc.push_str(&format!("A{c}_{s}:{pts:?}\n"));
        }
        // (b) 多中枢散布合成电池（type3 路径，无趋势）。
        for &(s, c) in &[(50usize, 4usize), (200, 8), (500, 16)] {
            let (ce, se, cl, cs) = synth_many_center_input(s, c);
            let pts = extract_signals(&ce, &se, &cl, &cs, &cfg);
            acc.push_str(&format!("B{s}_{c}:{pts:?}\n"));
        }
        // (c) LCG 随机中枢+线段+closes（趋势/破中枢/type3 概率覆盖）。
        for &seed in &[1u64, 42, 0xDEAD_BEEF] {
            for &(s, c) in &[(60usize, 6usize), (180, 12)] {
                let (ce, se, cl, cs) = lcg_signal_input(s, c, seed);
                let pts = extract_signals(&ce, &se, &cl, &cs, &cfg);
                acc.push_str(&format!("C{seed}_{s}_{c}:{pts:?}\n"));
            }
        }
        // (d) 重复 (end_index,zd,zg) 三元组中枢（stress `.position()` 首匹配语义）。
        let dup = vec![
            center(100, 200, 4), center(100, 200, 4), center(50, 80, 4),
            center(100, 200, 10), center(30, 60, 16),
        ];
        let dsegs = vec![
            seg(Direction::Up, 5, 8, 60, 250), seg(Direction::Down, 8, 12, 250, 210),
            seg(Direction::Down, 17, 20, 60, 20), seg(Direction::Up, 20, 24, 20, 55),
        ];
        let (dcl, dcs) = closes_seq(&[
            100, 100, 100, 100, 100, 100, 100, 100, 90, 80, 70, 60, 50, 40, 30, 20,
            210, 205, 200, 195, 55, 50, 45, 40,
        ]);
        let pts = extract_signals(&dup, &dsegs, &dcl, &dcs, &cfg);
        acc.push_str(&format!("D:{pts:?}\n"));
        fnv1a(&acc)
    }

    #[test]
    fn extract_signals_bit_exact_digest_guard() {
        // bit-exact 前后对拍（#93）：优化（`.position()` → first_match_idx HashMap + locate_trend_seg_a
        // 缓存）后摘要必须 = 优化前 oracle（git HEAD b8667b968f `extract_signals`）在同电池上的输出。
        //
        // ★GOLDEN 校准（no-patch）：本 GOLDEN = oracle `extract_signals_orig`（从 git HEAD 逐字复制的
        // 优化前实现）在同电池上的 FNV 摘要。`extract_signals_bit_exact_vs_orig_per_case` 逐 case 坐实
        // 优化版与 oracle **逐字段**相等（含新增 struct_break_dir 字段）⟹ 此 GOLDEN 同等于优化版输出。
        //
        // ★P2-R2 诚实更新（codex-review-20260701-2251 护栏4 方案A）：`BspPoint` 新增 `struct_break_dir:
        // Option<Side>` 字段（p2-plan §2 破中枢方向源，消选择偏差）进入 `#[derive(Debug)]` ⟹ FNV 对
        // `{pts:?}` 的摘要必然翻转（新字段改变 Debug 字符串）。这是**诚实的、可证的**代价——**不**通过
        // 自定义 Debug 隐藏 struct_break_dir 假装 digest 不变（那是声明膨胀，护栏4 禁止）。六 bit
        // （buy1/2/3+sell1/2/3）逐字段不变由 `extract_signals_bit_exact_vs_orig_per_case`（逐 case
        // assert_eq 全字段比较）锁定——GOLDEN 翻转**仅**因新增字段，非六 bit 语义变化。
        //
        // ★beta-route #115 诚实更新（force_state 生产热路由）：`BspPoint` 再新增 `force: Option<ForceProxies>`
        // 字段（β^div 力度 proxy）进入 `#[derive(Debug)]` ⟹ 每点 Debug 尾多 `, force: None`（本电池经
        // `extract_signals` 传**空 dif/closes_tick** ⟹ force 恒 None，非力度值差）⟹ FNV 摘要再翻转。同
        // struct_break_dir 先例：**诚实、可证、均匀**（全 None 常量），**不**自定义 Debug 隐藏 force。六 bit
        // + pivot + center + struct_break_dir 逐字段不变仍由 per-case（PartialEq 排除 force，本电池 force
        // 恒 None 无差可漏）锁定——翻转**仅**因新增 `, force: None` 常量串。
        // 历史值：`0x56ed_dd65_1c59_5733`（force 引入前，struct_break_dir 后）；
        //         `0x37d2_45a7_cdc5_505a`（P2-R2 前，struct_break_dir 引入前）；
        //         `0x06b3_7c2f_3a5e_9d41`（更早，与 git HEAD oracle 不一致的历史电池状态）。
        const GOLDEN: u64 = 0x90c7_9ee6_17e1_1392;
        let digest = bit_exact_battery_digest();
        assert_eq!(
            digest, GOLDEN,
            "bit-exact 摘要={digest:#018x}（优化版须 = git HEAD oracle 摘要；`diag_compare_vs_orig` 锁定逐字段相等）"
        );
    }

    /// ★bit-exact 逐 case 对拍（#93，no-patch 锁）：优化版 `extract_signals` 须与 git HEAD oracle
    /// （`extract_signals_orig`，逐字复制的优化前实现）在**每个 case** 上逐字段相等。这比 FNV 摘要
    /// 更强——摘要碰撞理论上可能，逐 case `assert_eq` 零碰撞风险。覆盖趋势 `.position()` 路径、多中枢
    /// 散布 type3 路径、LCG 随机趋势/破中枢/type3、重复三元组 stress（14 case）。
    #[test]
    fn extract_signals_bit_exact_vs_orig_per_case() {
        let cfg = MacdConfig::default();
        let mut cases: Vec<(String, Vec<Center>, Vec<Segment>, Vec<f64>, Vec<usize>)> = Vec::new();
        for &(c, s) in &[(8usize, 16usize), (20, 40), (12, 60)] {
            cases.push((
                format!("A{c}_{s}"),
                down_trend_centers(c), up_segs_after(s, c), vec![], vec![],
            ));
        }
        for &(s, c) in &[(50usize, 4usize), (200, 8), (500, 16)] {
            let (ce, se, cl, cs) = synth_many_center_input(s, c);
            cases.push((format!("B{s}_{c}"), ce, se, cl, cs));
        }
        for &seed in &[1u64, 42, 0xDEAD_BEEF] {
            for &(s, c) in &[(60usize, 6usize), (180, 12)] {
                let (ce, se, cl, cs) = lcg_signal_input(s, c, seed);
                cases.push((format!("C{seed}_{s}_{c}"), ce, se, cl, cs));
            }
        }
        let dup = vec![
            center(100, 200, 4), center(100, 200, 4), center(50, 80, 4),
            center(100, 200, 10), center(30, 60, 16),
        ];
        let dsegs = vec![
            seg(Direction::Up, 5, 8, 60, 250), seg(Direction::Down, 8, 12, 250, 210),
            seg(Direction::Down, 17, 20, 60, 20), seg(Direction::Up, 20, 24, 20, 55),
        ];
        let (dcl, dcs) = closes_seq(&[
            100, 100, 100, 100, 100, 100, 100, 100, 90, 80, 70, 60, 50, 40, 30, 20,
            210, 205, 200, 195, 55, 50, 45, 40,
        ]);
        cases.push(("D".to_string(), dup, dsegs, dcl, dcs));
        for (name, ce, se, cl, cs) in &cases {
            let opt = extract_signals(ce, se, cl, cs, &cfg);
            let ori = extract_signals_orig(ce, se, cl, cs, &cfg);
            assert_eq!(opt, ori, "case {name}：优化版须与 git HEAD oracle 逐字段相等（bit-exact）");
        }
    }

    #[test]
    #[ignore = "标度计时（map 线性扫 `.position()` 热点），--ignored 显式触发"]
    fn scale_timing_position_hotspot() {
        use std::time::Instant;
        let cfg = MacdConfig::default();
        let mut prev: Option<(usize, f64)> = None;
        for &n in &[1000usize, 2000, 4000, 8000] {
            let centers = down_trend_centers(n);
            let segs = up_segs_after(n, n);
            let mut best = f64::INFINITY;
            let mut len = 0;
            for _ in 0..3 {
                let t0 = Instant::now();
                let pts = extract_signals(&centers, &segs, &[], &[], &cfg);
                let dt = t0.elapsed().as_secs_f64();
                best = best.min(dt);
                len = pts.len();
            }
            match prev {
                Some((pn, pt)) => {
                    let exp = (best / pt).ln() / (n as f64 / pn as f64).ln();
                    println!("[.position() S=C={n}] t={best:.6}s bsp={len} exp_vs_prev={exp:.3}");
                }
                None => println!("[.position() S=C={n}] t={best:.6}s bsp={len}（基准点）"),
            }
            prev = Some((n, best));
        }
    }

    #[test]
    #[ignore = "标度计时（破中枢路径，含 locate_trend_seg_a 残留），--ignored 显式触发"]
    fn scale_timing_breaking_path() {
        use std::time::Instant;
        let cfg = MacdConfig::default();
        let mut prev: Option<(usize, f64)> = None;
        for &n in &[1000usize, 2000, 4000, 8000] {
            let centers = down_trend_centers(n);
            let base = 2 * n;
            let segs: Vec<Segment> = (0..n)
                .map(|k| Segment {
                    direction: Direction::Down,
                    start_index: base + 2 * k,
                    end_index: base + 2 * k + 1,
                    start_price: 50,
                    end_price: 5,
                })
                .collect();
            let nc = base + 2 * n + 2;
            let closes: Vec<f64> = (0..nc).map(|t| 200.0 + ((t % 7) as f64) - 3.0).collect();
            let csrc: Vec<usize> = (0..nc).collect();
            let mut best = f64::INFINITY;
            let mut len = 0;
            for _ in 0..3 {
                let t0 = Instant::now();
                let pts = extract_signals(&centers, &segs, &closes, &csrc, &cfg);
                best = best.min(t0.elapsed().as_secs_f64());
                len = pts.len();
            }
            match prev {
                Some((pn, pt)) => {
                    let exp = (best / pt).ln() / (n as f64 / pn as f64).ln();
                    println!("[破中枢 S=C={n}] t={best:.6}s bsp={len} exp_vs_prev={exp:.3}");
                }
                None => println!("[破中枢 S=C={n}] t={best:.6}s bsp={len}（基准点）"),
            }
            prev = Some((n, best));
        }
    }

}