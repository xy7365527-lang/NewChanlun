//! Θ_level + Θ_signal 子模块（reference-theta-v0.md:27-37）。
//!
//! ## 契约重锚（legacy Strict/* → Origin canonical，task #127 A′ Phase2）
//!
//! 递归级别构造 + R6 态 + 买卖点 bit-vector + 背驰度量 + 区间套。给定 Θ_level/Θ_signal ⟹ R6 态 +
//! BSP 证书唯一。契约锚点从 legacy `Strict/{LevelState,Center,BSP,Trend,Nest,Recursive}.lean`
//! **重锚到 Origin canonical**：`Origin.RecursiveLevelSystem` / `Origin.CenterStates` /
//! `Origin.BspClassification` / `Origin.TrendCompleteClassification` / `Origin.SubLevelDescent`。
//!
//! ## 子模块拓扑（各子模块契约锚 Origin def）
//!
//! - [`center`]：完整中枢判据（方向交替+第三段贯穿）+ 关系/位置三态。对齐
//!   `Origin.CenterComplete.CenterConfirmedComplete` / `Origin.CenterConstruction.centersOf` /
//!   `Origin.CenterStates.{classifyDevelopment,classifyPosition}`。
//! - [`level`]：递归级别走势裁决（`classifyMove` 全链同向）。对齐
//!   `Origin.TrendCompleteClassification.{TrendClass,chooseTrend}` + `Origin.RecursiveLevelSystem`。
//! - [`level_state`]：R6 位置态 + LevelState 三元组。对齐 `Origin.CenterStates.CenterPosition` +
//!   `Origin.RecursiveLevelSystem`。
//! - [`bsp`]：买卖点 bit-vector 判据（三类结构谓词，非互斥）。对齐
//!   `Origin.BspClassification.{BspEndpoint,IsType1,IsType2,IsType3Buy,IsType3Sell}`。
//! - [`divergence`]：背驰 MACD 度量（浮点域隔离 + 同向段面积严格变小）。对齐
//!   `Origin.Divergence.{Force,IsDivergence}` + `Origin.ForceInterface.ForceMeasure`（reference:37）。
//! - [`nest`]：区间套有限递归证书 χ（`Sel_Θ` 选择器 + 终端确认）。对齐
//!   `Origin.SubLevelDescent.{descend,subLevelHasBrokenCenter}`。
//!
//! ## 递归级别（reference-theta-v0.md:29-30；契约锚 `Origin.RecursiveLevelSystem`）
//!
//! `L0=1分钟线段账本`（parser segments）；`L(k+1)` 只由 `Lk` 已完成走势/Move 构造（对齐
//! `Origin.RecursiveLevelSystem.lift` + `chanRecursiveLevelSystem` 的 `composeStep`：Lk 走势单元 →
//! 连续三段窗口中枢 → 中枢序列裁决走势 → L(k+1) 输入单元）。禁跳级混级。
//! 某层无 ≥`config.level.min_parts_per_level` 完成部件则自然终止；上界 `config.level.l_max`。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 实装本身 = L1（bit-exact 一致性：Rust 与 Lean spec 输出对齐 = 验证管线正确，不验证
//! Θ 在市场上有效）。各子模块的判定函数是 Lean 纯函数的镜像（L0 给定 Θ 后）；golden/
//! property 测试是 L1（管线正确性，零信息增量）。L2/L3 有效域检验是 Phase 3-4，本模块不声称。
//!
//! ## 铁律（编排者硬指令）
//!
//! 只实装已冻结 Θ v0；遇 spec 漏洞/与 Lean 冲突 → change request，不静默改语义。
//! 不可变：构造新对象，不原地修改。不可交易/退化情况显式处理不静默吞。

use super::config::ThetaConfig;
use super::parser::ParseLayer;
use super::types::{Center, Direction, MoveKind, Segment};
use divergence::MacdState;
use std::rc::Rc;

pub mod center;
pub mod ref_v1;
pub mod level;
pub mod level_state;
pub mod bsp;
pub mod divergence;
pub mod force_conformance;
pub mod descend;
pub mod rmove_compose;
pub mod recursive_tower;
pub mod nest;
pub mod signal;
pub mod six_state;
pub mod voice_eat;
pub mod cand_predicate;

use bsp::BspPoint;
use center::UnitRange;
use level::{classify_move, outcome_to_kind, MoveOutcome};
use recursive_tower::{
    compose_level, descend_leveled, index_of_in, map_src_to_close_idx, project_to_units,
    ElementId, LeveledMove,
};
use super::types::Side;

/// 单级别分类状态（R6 态 + 走势类型 + 中枢 + 买卖点）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LevelState {
    /// 该级别识别出的走势类型序列（Trend/Consolidation；HigherCenterCandidate 不入 moves）。
    pub moves: Vec<MoveKind>,
    pub centers: Vec<Center>,
    /// 各买卖点条目（非互斥 bit-vector + 结构止损价 single source，BSP.lean + reference:46）。
    ///
    /// 路 B（Lead 接口契约裁定）：每个 `BspPoint` 携带 pivot_low/pivot_high/center——
    /// strategy 直接构造 `StopInput`，**不**从 bars 重算结构 pivot。classifier 是结构
    /// 止损价的唯一来源（识别买卖点时已定位 pivot，避免两处结构逻辑漂移）。
    pub bsp: Vec<BspPoint>,
}

/// 多级别递归分类输出（L0..Lmax；某层自然终止则该层及以上为空）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Classification {
    /// 索引 = 级别 ℓ（0=L0=1分钟线段账本）。
    pub levels: Vec<LevelState>,
}

/// 把 L0 线段规约为携带方向的走势单元（契约锚 `Origin.ChanlunElements.Segment` + `CenterConstruction.segHigh/segLow`）。
///
/// L0 单元 = parser 线段（**含方向**，完整判据 `DirAlternates` 的输入）；`[lo,hi]` 对齐
/// `Origin.CenterConstruction.segHigh/segLow`（向上段 hi=端价/向下段 lo=端价已规约为区间）。
fn segment_to_unit(seg: &Segment) -> UnitRange {
    let (lo, hi) = if seg.start_price <= seg.end_price {
        (seg.start_price, seg.end_price)
    } else {
        (seg.end_price, seg.start_price)
    };
    UnitRange {
        start_index: seg.start_index,
        end_index: seg.end_index,
        direction: seg.direction,
        lo,
        hi,
    }
}

/// 从 L0 线段单元序列识别中枢序列（**完整判据**，契约锚 `Origin.CenterComplete.CenterConfirmedComplete`）。
///
/// L0 线段有内在方向 ⟹ 用 `center::center_from_segments`（完整判据：方向交替 ∧ 前两段核心非空 ∧
/// 第三段贯穿）。从左到右扫描：连续三段构成真中枢则前进 3 段（已确认中枢不回写，reference:16）；
/// 任一支不成立（无方向交替/核心空/第三段不贯穿）则前进一段继续找（对齐 `Origin.centersOf` 滑窗：
/// 成立支消费 3、不成立支消费 1）。
///
/// ★诚实范围：v0 用**非重叠三段窗口**识别中枢（连续三段成真枢则前进 3 段）。延伸中枢
/// （同一中枢吸收后续段）的完整 start/finish 区间识别留待后续（Origin `centersOf` 当前亦三段窗口）。
fn detect_centers_complete(units: &[UnitRange]) -> Vec<Center> {
    detect_centers_with(units, center::center_from_segments)
}

/// 从上级走势单元序列识别中枢序列（**几何路径**，契约锚 `Origin.centerHolds` + 三段共同重叠）。
///
/// 上级单元是中枢外缘区间（**无内在缠论方向**，方向由 Move 趋势裁决携带）⟹ 用
/// `center::center_from_window`（几何三支：前两段核心非空 + 第三段贯穿，无方向交替）。上级发展
/// 裁决用 `Origin.CenterStates.classifyDevelopment`（外缘判据，无方向交替要求）——见 `center.rs`
/// 诚实有效域声明。
fn detect_centers_geometric(units: &[UnitRange]) -> Vec<Center> {
    detect_centers_with(units, center::center_from_window)
}

/// 三段窗口扫描骨架（成立支消费 3 段、不成立支消费 1 段，对齐 `Origin.centersOf` 滑窗终止性）。
///
/// `build` 是中枢构造函数（L0=完整判据 `center_from_segments`；上级=几何 `center_from_window`）。
fn detect_centers_with(
    units: &[UnitRange],
    build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>,
) -> Vec<Center> {
    let mut centers = Vec::new();
    let mut i = 0usize;
    while i + 2 < units.len() {
        match build(&units[i], &units[i + 1], &units[i + 2]) {
            Some(c) => {
                centers.push(c);
                // 成立支：前进 3 段（已确认中枢不回写，reference:16）。
                i += 3;
            }
            None => {
                // 不成立支：前进 1 段继续找（对齐 Origin centersOf 滑窗）。
                i += 1;
            }
        }
    }
    centers
}

/// 把一级走势单元序列规约为该级走势裁决 + 中枢（reference:29 `classifyMove`）。
///
/// `is_l0`：L0 用完整判据（方向交替），上级用几何路径（外缘）。返回 `(中枢序列, 走势裁决)`。
fn classify_level(units: &[UnitRange], is_l0: bool) -> (Vec<Center>, MoveOutcome) {
    let centers = if is_l0 {
        detect_centers_complete(units)
    } else {
        detect_centers_geometric(units)
    };
    let outcome = classify_move(&centers);
    (centers, outcome)
}

/// Θ_level + Θ_signal 分类内部实现（`classify` / `classify_with_tower` 共享单一来源）。
///
/// `tower_snapshots[i]` = 处理第 i 级时 `compose_level` 前的 `moves_tower` 快照，下标与
/// `Classification.levels` 同构（`tower_snapshots.len() == levels.len()`）：
/// - 索引 0（L0 级）：全 `RMove::Segment`（递归底，`sub_moves` 空）。
/// - 索引 ≥1（L(k) 级）：前一级产出的 `RMove::Compose` 序列（携次级别 subs，depth≥1 真嵌套）。
fn classify_impl(l0: &ParseLayer, config: &ThetaConfig) -> (Classification, Vec<Rc<Vec<LeveledMove>>>) {
    let min_parts = config.level.min_parts_per_level as usize;
    let l_max = config.level.l_max as usize;

    // L0 输入单元 = parser 线段账本（reference:29 L0=1分钟线段账本）。
    let mut units: Vec<UnitRange> = l0.segments.iter().map(segment_to_unit).collect();

    // 空 L0：无可构造级别（自然终止于 L0 之前）。
    if units.is_empty() {
        return (Classification::default(), Vec::new());
    }

    // ★递归塔对象（#53 升级，still-MISSING-塔解除）：L0 走势单元 = 携坐标的 `RMove::Segment`
    // （`LeveledMove`，递归底 level 0）。旧塔把每级走势单元折叠为无 subs 的 `UnitRange`，
    // `extract_second_signals`（消费 `RMove::Compose` 的 descend 取回次级别走势）永产不出 B2/S2。
    // 新塔每级走势单元携次级别走势 subs（`RMove::Compose`）+ source_index 坐标 ⟹ B2/S2 真可产。
    // ★O(n) 重构：moves_tower/snapshots 用 Rc（与增量版同返回类型 `Vec<Rc<Vec<LeveledMove>>>`，
    // bit-exact 测试逐字段比较 *rc）。全量版非 per-bar 热点（O(n) 单趟），Rc 仅为类型对齐。
    let mut moves_tower: Rc<Vec<LeveledMove>> = Rc::new(
        units
            .iter()
            .enumerate()
            .map(|(i, u)| LeveledMove::from_unit(u, ElementId { level: 0, ordinal: i as u64 }))
            .collect(),
    );

    // 第一类背驰 MACD：closes/close_src 在全递归层共享（L0 唯一可达 close 序列；上级走势的
    // 次级别 close 区间由 source_index 坐标定位，见 macd 接入点）。
    let closes: Vec<f64> = l0.merged_bars.iter().map(|b| b.close as f64).collect();
    let close_src: Vec<usize> = l0.merged_bars.iter().map(|b| b.source_index).collect();
    let hist = divergence::compute_macd(&closes, &config.macd).hist;

    let mut levels: Vec<LevelState> = Vec::new();
    let mut tower_snapshots: Vec<Rc<Vec<LeveledMove>>> = Vec::new();

    // 递归级别构造：每级由下级走势单元构造（L0 直接是线段单元，从 L0 开始裁决）。
    for level_idx in 0..=l_max {
        // 自然终止（reference:30）：某层无 ≥min_parts 完成部件 ⟹ 无法产生完整走势，停止。
        if units.len() < min_parts {
            break;
        }

        // 本级输入塔快照（compose_level 前，与 levels[level_idx] 对应——同步 index 不变量）。
        tower_snapshots.push(Rc::clone(&moves_tower));

        // L0（level_idx==0）用完整判据（方向交替，线段有方向）；上级用几何路径（外缘，单元无方向）。
        let is_l0 = level_idx == 0;
        let (centers, outcome) = classify_level(&units, is_l0);

        // 走势裁决 → MoveKind（HigherCenterCandidate 退化态映 None，不入 moves）。
        let moves: Vec<MoveKind> = outcome_to_kind(outcome).into_iter().collect();

        // L(k+1) 走势塔 = 本级窗口化 compose（每中枢的构成三段次级别走势 → 一个上级 `RMove::Compose`，
        // 契约锚 `Origin.RecursiveLevelSystem.composeStep` 窗口封装）。`upper_moves` 是携坐标的上级走势
        // 序列（descend 取回构成它的次级别走势 ⟹ B2/S2 可产），与 `centers` 一一对应。
        let (centers_w, upper_moves) = compose_level(&units, &moves_tower[..], is_l0, level_idx as u32 + 1);
        debug_assert_eq!(centers, centers_w, "compose_level 与 classify_level 中枢序列一致");

        // BSP 信号提取（reference:34-36）。三层覆盖：
        // - **L0 线段层**（`extract_signals`）：第一类（破中枢几何 L0 ∧ MACD 背驰 L1 真算）+ 第三类
        //   （confirmed 结构几何）。L0 走势单元 = 线段（有方向），第一/三类在线段端点上 bit-exact 判定。
        // - **递归组装层**（`extract_second_signals`，#53 接入）：第二类（B2/S2）由次级别第一类构成
        //   （买卖点定律一 §10.2）。对本级**每个上级走势** `RMove::Compose`，从 descend 取回的次级别
        //   走势序列内识别第二类走势结构（第一类离开 + 回拉不创新低/新高），产 B2/S2。背驰力度由
        //   `divergence_of` 闭包用 `divergence.rs` MACD 真算（次级别走势 close 区间 → 面积比较）。
        let mut bsp: Vec<BspPoint> = if is_l0 {
            signal::extract_signals(&centers, &l0.segments, &closes, &close_src, &config.macd)
        } else {
            Vec::new()
        };
        // 递归组装层 B2/S2（#53 接入）：对每个上级走势的次级别走势序列识别第二类结构。
        bsp.extend(extract_second_for_level(&upper_moves, &hist, &close_src));
        bsp.sort_by_key(|p| p.source_index);

        levels.push(LevelState {
            moves,
            centers: centers.clone(),
            bsp,
        });

        // L(k+1) 输入单元 = 上级走势塔的 `UnitRange` 投影（外缘区间 + 坐标 + 外缘趋势方向）。
        // 上级走势携 subs（`RMove::Compose`），投影只为下一级几何中枢检测提供 [lo,hi] 区间——
        // 真递归 subs 在 `moves_tower` 里保留（不丢弃，旧塔丢弃 subs 是 B2 不可产的根因）。
        units = project_to_units(&upper_moves);
        moves_tower = Rc::new(upper_moves);

        // 本级无中枢 ⟹ 无上级输入单元，停止递归（自然终止）。
        if units.is_empty() {
            break;
        }
    }

    (Classification { levels }, tower_snapshots)
}

/// Θ_level + Θ_signal 顶层入口（reference-theta-v0.md:27-37）。
///
/// 递归构造 L0..Lmax：L0=parser 线段账本；每级由下级已完成走势单元构造中枢 + 裁决走势，
/// 走势成为上级输入单元。自然终止：某级单元数 < `min_parts_per_level`（无法产生完整走势），
/// 或达 `l_max` 上界。
///
/// ★边界条件：
/// - L0 线段数 < `min_parts_per_level` ⟹ `levels` 仅含 L0（或为空，见下）—— 自然终止。
/// - 任一级中枢序列裁决为 `HigherCenterCandidate`（退化）⟹ 该级 moves 不收录该裁决
///   （outcome_to_kind → None），但中枢/bsp 仍保留（结构事实）。
/// - 空 ParseLayer（无线段）⟹ `Classification::default()`（空 levels，无可构造级别）。
pub fn classify(l0: &ParseLayer, config: &ThetaConfig) -> Classification {
    classify_impl(l0, config).0
}

/// 分类 + 逐级塔导出入口（(i) 段导出桥，MEMORY coverage-engine-needs-tower-export-bridge）。
///
/// 返回 `(Classification, Vec<Vec<LeveledMove>>)`：
/// - `Classification`：与 `classify` bit-identical（共用 `classify_impl` 单一来源，原行为不变）。
/// - `Vec<Vec<LeveledMove>>`：逐级走势塔快照（`tower[i]` 对应第 i 级处理的输入塔）：
///   - `tower[0]`：L0 线段层（全 `RMove::Segment`，递归底，`sub_moves` 空）。
///   - `tower[k]`（k≥1）：第 k 级输入塔，含 `RMove::Compose` 携次级别 subs（depth≥1 真嵌套），
///     下游 `descend_leveled` 可遍历次级别走势。
///
/// ★不碰附着映射：本函数只导出塔，不消费 coverage/interp 的附着规则（(ii) 段职责）。
/// ★L0/L1 认识论等级：纯结构导出操作，不依赖经验数据（formalization-validity-domain 231号）。
pub fn classify_with_tower(
    l0: &ParseLayer,
    config: &ThetaConfig,
) -> (Classification, Vec<Rc<Vec<LeveledMove>>>) {
    classify_impl(l0, config)
}

// ════════════════════════════════════════════════════════════════════════════
//  增量塔 API（task #93：per-bar substrate 塔构造 O(n²) → O(n)）
// ════════════════════════════════════════════════════════════════════════════
//
// ## 超线性根因（前序 aed4d5f5 实证：classify c_exp≈2.31 主导）
//
// `classify_impl` 的塔循环 `for level_idx in 0..=l_max` 每级调 `compose_level`（→
// `detect_centers_windowed`）+ `classify_level`（→ `detect_centers_with`）——两次全量滑窗扫描。
// per-bar substrate（每 bar 追加段）下，前级 confirmed 前缀稳定却重复全量扫描 ⟹ 超线性。
//
// ## 增量策略（严格有效域声明，formalization-validity-domain 231号）
//
// **有效域**：`tower_snapshots`（Vec<Vec<LeveledMove>>）+ `LevelState.centers` 的**中枢扫描构造**
// 可增量——`detect_centers_windowed` 是确定性左折叠（见 recursive_tower.rs 增量证明），已产出
// 中枢是不可变前缀，尾部追加续扫产出 bit-exact 尾部。
//
// **不在有效域**（诚实声明，no-声明膨胀）：
// - `LevelState.moves`（走势裁决）：`classify_move(&centers)` 在完整 centers 序列上裁决趋势，
//   尾部追加中枢可改变整体裁决 ⟹ 须每 bar 从累积 centers 全量裁决（非增量）。但裁决是 O(centers)
//   单趟，不是超线性源。
// - `LevelState.bsp`：依赖 MACD hist（每 bar 变）+ 完整 centers ⟹ 每 bar 全量重算。
//
// 故增量塔的 LevelState.centers/moves/bsp 由**累积的完整 centers 序列**经与全量同口径的裁决/
// BSP 提取产出（bit-exact），仅**中枢扫描构造**走增量 resume。塔构造的超线性（双次全量扫描）
// 被消除 ⟹ exp≈1。
//
// **641 谱系标注（有效域声明）**：incr_total exp 0.91 @16K ≈ 1.0 → 塔构造 O(n) **已达成**
// （L2 真实数据验证，非 L0/L1）。有效域 = 中枢扫描构造（compose_level_resume）+ parse_layer
// 增量合并（process_inclusion）。**不在有效域**：全引擎 per-bar 端到端 exp 须大规模验证（moves/
// bsp 裁决每 bar 全量但 O(centers) 单趟，非超线性源；实测端到端 exp 见 benchmark）。
// **注意（ab5f5a29d）**：O(n) 达成 ≠ 身份稳定→Stale 降根。增量塔 bit-exact 跨 bar 复用，但
// held_leg_tree_index 值字段比较（level/ρ/eps/λ）非对象身份——bit-exact 不变 ⟹ Stale 不降。
// "增量塔→身份连续→Stale 降根→ΔSharpe 可非零"路径被 L2 证伪（见 runner.rs 注释 + memory
// newchanlun-deltasharpe-zero-stale-rooting-perbar-reclass）。
//
// ## 跨级增量传播（严格不变量）
//
// 每级 units = `project_to_units(&upper_moves)`。上级 upper_moves 尾部追加时下级 units 尾部变，
// 下级从自己的断点续扫。**前缀不变量链**：L0 段前缀不变 ⟹ L0 upper_moves 前缀不变（resume 已证）
// ⟹ L1 units 前缀不变（project 逐元素，前缀同序同值）⟹ L1 扫描断点有效 ⟹ ... 逐级传播。
//
// bit-exact 充要：`cache.last_units_len` 与再次进入时的前缀严格增长（只追加，不修改前缀）。

use recursive_tower::{compose_level_resume, WindowScanCursor};

/// 单级增量缓存：已确认前缀 + 续扫断点。
///
/// 不变量（跨 bar 保持）：
/// - `scan_cursor.consumed`：本级窗口扫描退出断点（上次扫到此处，`units[..consumed]` 路径确定）。
/// - `upper_moves`：本级已 compose 的上级走势序列（前缀不可变；尾部续扫追加）。
/// - `last_input_len`：上次扫描时本级 `subs_moves`（= units）长度——再次进入时前缀须不大于此。
/// - `cached_outcome`：上次 `classify_move` 结果（增量续算用，O(1) 续判新尾对）。
#[derive(Debug, Clone, Default)]
struct LevelCache {
    scan_cursor: WindowScanCursor,
    /// 已 compose 的上级走势序列（前缀不可变；尾部续扫追加）。每元素 `RMove::Compose` 携真 subs。
    /// ★O(n) 重构：`Rc` 共享塔——`moves_tower`/`tower_snapshots` 经 `Rc::clone`（O(1) 引用计数）取得，
    /// 消除 per-bar 全塔深拷贝（O(n²) 热点①）。`extend` 经 `Rc::make_mut`：caller 逐 bar drop 返回的
    /// snapshot ⟹ 下 bar extend 时 strong_count==1 ⟹ 原地追加 O(tail)，不触发写时复制。
    upper_moves: Rc<Vec<LeveledMove>>,
    /// 已识别中枢序列（与 `upper_moves` 一一对应，每窗口一中枢；前缀不可变，尾部续扫追加）。
    centers: Vec<Center>,
    last_input_len: usize,
    /// 上次扫描的本级输入单元快照（前缀变异检测）。`detect_centers_windowed_resume` 充要条件 #2
    /// 要求 `units[..consumed]` 跨 bar 不可变；但 parser frontier 末段会**原地改写**（缠论古怪线段
    /// 重划，67/78课「顶高于底」+ 特征序列再分辨——定义层正确行为，非 parser bug）。仅长度回缩
    /// 守卫漏掉「长度不变/增长但末段值改写」的 frontier 变异。本快照逐值比对扫描区，变异 ⟹ 该级
    /// 缓存全量重置（anc.pdf §16：confirmed prefix immutable 可跳，frontier mutable 必须每 bar 重算）。
    cached_units: Vec<UnitRange>,
    /// 增量 classify_move 缓存（None=未初始化；Some=对应 `centers` 当前序列的裁决）。
    cached_outcome: Option<level::MoveOutcome>,
    /// BSP memo 缓存：上次提取的 BSP 序列（与 `cached_bsp_key` 配对）。
    cached_bsp: Vec<BspPoint>,
    /// BSP memo guard key = (centers.len, upper_moves.len, segments.len[L0 only])。三者不变 ⟹
    /// BSP 纯函数同输入同输出（confirmed 元素区间在稳定前缀，尾 bar 不影响）⟹ 复用缓存 bit-exact。
    cached_bsp_key: Option<(usize, usize, usize)>,
    /// ★O(n²) 真修（#106）：本级 `upper_moves` 投影缓存（= `project_to_units(&upper_moves)`，下一级输入）。
    /// `upper_moves` 前缀不变仅尾部 append（§16）⟹ 投影前缀不变（`fold_direction(prev)` 只依赖元素+前驱，
    /// 前缀稳定 ⟹ 前缀投影稳定）。每 bar 只对新 append 的 tail 投影（`projected_units.len()..`），避免
    /// per-bar 全量 `project_to_units` 递归 `rmove.lo()/hi()` 整棵子树 O(nodes)/bar=O(n²)。
    /// cascade_reset（前缀重排）⟹ 与 upper_moves 同步清空（line 850 旁）重投影。
    projected_units: Vec<UnitRange>,
}

/// 增量塔缓存（跨 bar 跨级复用）：每级 `LevelCache` + L0 段账本快照长度 + MACD 增量状态。
///
/// **使用契约**（bit-exact 充要，违反则增量破裂）：
/// 1. 每 bar 喂 `classify_with_tower_incremental(l0_i, config, &mut cache)`，`l0_i.segments` 是
///    `parse_layer(&bars[..=i])`——段账本**只允许尾部增长**（前缀段不可变，parser 前缀稳定语义）。
/// 2. 若某 bar 段账本前缀**回缩或改写**（非单调追加），须 `cache.clear()` 重置（退化为全量）。
/// 3. `config.level`（l_max/min_parts）不可变——变则 `cache.clear()`。
/// 4. `merged_bars.close` 前缀须单调追加（前缀不变）——inclusion 合并可能改写尾部，
///    故 `macd_closes_prefix` 校验 `closes[..last_merged_len]` 逐值一致；不一致则全量重建。
#[derive(Debug, Clone, Default)]
pub struct TowerCache {
    /// 逐级缓存（下标 = 级别 idx，与 `tower_snapshots` 同构）。
    levels: Vec<LevelCache>,
    /// 上次处理的 L0 段数（前缀不变量校验用）。
    last_l0_segments_len: usize,
    /// MACD 增量递推状态（None=未初始化；Some=已处理至 `macd_state_len` 末）。
    macd_state: Option<MacdState>,
    /// 已增量产出的 hist 前缀（不可变；尾部 append 续产）。bit-exact 等价于
    /// `compute_macd(closes[..macd_state_len]).hist`。
    macd_hist: Vec<f64>,
    /// MACD state 实际消费的 close 数（state 表示 `closes[..macd_state_len]` 的累积）。
    /// #106：替代旧 `macd_closes_prefix: Vec<f64>`（每 bar O(n) 全量比较 + to_vec 克隆 = O(n²)）。
    /// 增量边界 = `macd_state_len`；前缀稳定性靠 parser `confirmed_len` 证书（codex：绑 state_len
    /// 而非 cached_len-1，修血缘断裂漏洞）。config 变更经 `clear()` 失效（ponytail: config 在 cache
    /// 生命周期固定，clear 已覆盖，无需独立 macd_epoch）。
    macd_state_len: usize,
    /// #106：L0 走势塔增量缓存（前缀稳定，仅 segments 末段可古怪线段重划改写）。用 parser
    /// `segments_confirmed_len` 证书复用前缀，只 map 新尾段——替代每 bar 全量 `from_unit` 重建
    /// （O(segs)×n = O(n²)，profile 坐实 400K=5.8s）。bit-exact：from_unit 只依赖单 seg（无相邻
    /// 依赖，codex 确认），前缀稳定 ⟹ moves_tower_l0 前缀稳定；ordinal=全局索引（reuse+i）跨 bar 稳定。
    moves_tower_l0: Rc<Vec<LeveledMove>>,
    /// #106：L0 输入单元（segment_to_unit 投影）增量缓存——同 moves_tower_l0 证书复用，消除每 bar
    /// 全量 `l0.segments.map(segment_to_unit).collect()`（O(segs)×n，profile 坐实 400K=0.6s）。
    /// segment_to_unit 只依赖单 seg ⟹ 前缀稳定 bit-exact。
    l0_units_cache: Vec<UnitRange>,
    /// merged_bars.close 增量缓存（前缀稳定，仅尾 bar 可能 inclusion 改写）。每 bar mem::take 出借
    /// 给 BSP/MACD，用毕放回——避免每 bar 全量 `.map().collect()` 重建（O(n)/bar → O(n²) 根因）。
    closes: Vec<f64>,
    /// merged_bars.source_index 增量缓存（与 `closes` 同步，坐标系映射用）。
    close_src: Vec<usize>,
    /// ★工位 4g：塔变更代次（generation）——下游 [`super::strategy::interp::TreeCache`] 用其 O(1) 判断
    /// 是否复用缓存树，**跳过每 bar O(tree) 的 `TreeKey::of(tower)` 全量重算**（exp≈2.0 真因）。
    ///
    /// **单调递增**，仅当 `extract_elements(tower)` **可观察输出可能变化**时 +1。维护点（codex 异质审
    /// soundness 全覆盖）：(a) 任一级 `cascade_reset`（frontier 改写/回缩传播至最高级重扫）；(b) 任一级
    /// `upper_moves` extend 非空 tail（含新级涌现首产 + 最高级 append）；(c) `clear()`（全量重扫）。
    ///
    /// soundness 充要（codex Q3）：generation 绑定"可观察树变更"非指针/len。低级 extend 不影响最高级
    /// 输出时**过度** +1（保守——多算一次 TreeKey，绝不假命中），sound 安全。`extract_elements` 只读最高
    /// 非空级，但 `sub_moves: Vec<LeveledMove>` 值拷贝（compose 时 `subs.to_vec()`）⟹ 低级静默变异必经
    /// cascade 重建父级才更新副本（codex Q1 确认无静默路径）。
    generation: u64,
}

impl TowerCache {
    /// 构造空缓存（首次调用全量扫，之后增量复用）。
    pub fn new() -> Self {
        Self::default()
    }

    /// ★工位 4g：当前塔变更代次（下游 `TreeCache` O(1) 命中判据）。同代次 ⟹ `extract_elements`
    /// 输出逐字节不变（soundness 见 `generation` 字段文档）。
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// 重置缓存（退化为下次全量重扫）。
    ///
    /// 调用时机：段账本前缀非单调追加（回缩/改写）、或 config 变更、或 merged_bars
    /// 前缀改写（inclusion 合并回退改写尾部）。
    pub fn clear(&mut self) {
        self.levels.clear();
        self.last_l0_segments_len = 0;
        self.macd_state = None;
        self.macd_hist.clear();
        self.macd_state_len = 0;
        Rc::make_mut(&mut self.moves_tower_l0).clear();
        self.l0_units_cache.clear();
        self.closes.clear();
        self.close_src.clear();
        // ★工位 4g：clear=全量重扫 ⟹ extract 输出会变 ⟹ +generation（不 reset 为 0——下游缓存旧
        // generation 可能恰为 0 ⟹ 假命中复用陈旧树）。单调递增保 sound。
        self.generation += 1;
    }
}

/// 增量走势裁决（bit-exact 等价于 `classify_move(centers)`，O(1) 续判）。
///
/// 利用 centers 前缀不可变 + `cached_outcome` 续算：
/// - **HigherCenterCandidate（0 centers）→ 1 center**：Consolidation（首中枢=盘整）。
/// - **Consolidation（1 center）→ 2 centers**：判首对关系，全 Up ⟹ Trend(Up)，全 Down ⟹
///   Trend(Down)，否则 HigherCenterCandidate。
/// - **Trend(rel) → +1 center**：判新尾对（倒数第二，末）关系==rel ⟹ 保持 Trend(rel)；
///   否则 HigherCenterCandidate（全链同向破裂）。
/// - **HigherCenterCandidate（≥2 centers，mixed）→ +1 center**：保持 HigherCenterCandidate
///   （全链同向一旦破裂不可恢复——追加 center 不能修复已有的 mixed 对）。
///
/// bit-exact：与 `classify_move` 在相同 centers 序列上产出相同 `MoveOutcome`（全链同向判据一致）。
/// `cached` 为 None 时全量计算首初始化（与 `classify_move` 一致），之后续判 O(1)。
fn classify_move_incremental(
    centers: &[Center],
    cached: &mut Option<level::MoveOutcome>,
) -> level::MoveOutcome {
    use level::MoveOutcome;
    use center::CenterRelation;

    // 全量首初始化或 centers 回缩（centers.len() < 上次）⟹ 全量重算。
    let need_full = cached.is_none()
        || match cached {
            Some(MoveOutcome::HigherCenterCandidate) => centers.len() <= 1,
            Some(MoveOutcome::Consolidation) => centers.len() <= 1,
            Some(MoveOutcome::Trend(_)) => centers.len() <= 1,
            None => true,
        };
    if need_full {
        let outcome = classify_move(centers);
        *cached = Some(outcome);
        return outcome;
    }

    let prev = cached.unwrap();
    // 从 prev 状态 + prev 时的 centers 长度续算到当前 centers。
    // prev 对应的 centers 长度推断：
    //   HigherCenterCandidate(0) → len 0; Consolidation → len 1; Trend → len ≥2
    //   HigherCenterCandidate(≥2 mixed) → len ≥2
    // 但 HigherCenterCandidate 可能来自 0 或 ≥2 mixed。用 centers.len() 推断 prev_len。
    // 简化：从 prev_len = centers.len() - (新增数) 续算。但增量只 +1~few centers。
    // 更简单：直接全量重算 if prev 与当前 centers 长度推断不匹配。
    //
    // ★严格增量：从 prev_outcome + prev_centers_len 续算。prev_centers_len = centers 增长前的长度。
    // 但 LevelCache 不存 prev_centers_len。故用 cached_outcome 的语义反推：
    //   Consolidation ⟹ prev_len == 1; Trend ⟹ prev_len ≥ 2; HCC ⟹ prev_len == 0 或 ≥2.
    // HCC 歧义：0 vs ≥2 mixed。0→1 是 Consolidation；≥2 mixed→+1 仍是 HCC。
    // 用 centers.len() 判：如果 centers.len() == 1 且 prev 是 HCC ⟹ prev_len=0 ⟹ Consolidation.
    // 如果 centers.len() >= 2 且 prev 是 HCC ⟹ prev_len≥2 mixed ⟹ 保持 HCC.
    let outcome = match (prev, centers.len()) {
        (MoveOutcome::HigherCenterCandidate, 1) => MoveOutcome::Consolidation,
        (MoveOutcome::HigherCenterCandidate, _) => MoveOutcome::HigherCenterCandidate, // ≥2 mixed 保持
        (MoveOutcome::Consolidation, 2) => {
            // 首对关系：全 Up ⟹ Trend(Up)，全 Down ⟹ Trend(Down)，否则 HCC。
            let rel = center::classify_relation(&centers[0], &centers[1]);
            match rel {
                CenterRelation::UpContinuation => MoveOutcome::Trend(Direction::Up),
                CenterRelation::DownContinuation => MoveOutcome::Trend(Direction::Down),
                _ => MoveOutcome::HigherCenterCandidate,
            }
        }
        (MoveOutcome::Consolidation, _) => {
            // len > 2 从 Consolidation 续算不应发生（Consolidation 仅 len==1）。
            // 防御性：全量重算。
            let o = classify_move(centers);
            *cached = Some(o);
            return o;
        }
        (MoveOutcome::Trend(dir), n) if n >= 2 => {
            // Trend(rel) + 新尾 center：判新尾对（n-2, n-1）关系是否保持 rel。
            let rel = center::classify_relation(&centers[n - 2], &centers[n - 1]);
            let expected = match dir {
                Direction::Up => CenterRelation::UpContinuation,
                Direction::Down => CenterRelation::DownContinuation,
            };
            if rel == expected {
                MoveOutcome::Trend(dir)
            } else {
                MoveOutcome::HigherCenterCandidate
            }
        }
        _ => {
            // 防御性：全量重算。
            let o = classify_move(centers);
            *cached = Some(o);
            return o;
        }
    };
    *cached = Some(outcome);
    outcome
}

/// 增量 MACD hist 计算（231号纯性能，bit-exact 铁律）。
///
/// 返回完整 hist 序列（等长 `closes`），与 `divergence::compute_macd(closes, cfg).hist`
/// 逐元素 bit-identical（ac75d4b3 已证 `MacdState::append` bit-exact）。
///
/// ## per-bar substrate 语义（inclusion 合并的尾部不稳定性）
///
/// `parse_layer(&bars[..i])` 的 `merged_bars` = `merged_prefix`（confirmed 稳定前缀）+
/// `acc`（当前未定稿合并段，可能被后续 bar 吸收改写）。故跨 bar：
/// - `merged_bars[..len-1]` 稳定（confirmed 前缀不动）
/// - `merged_bars[len-1]`（= acc）可能改写
///
/// 增量 state 表示 `closes[..stable_prefix]`（stable_prefix = `closes.len() - 1`，排除
/// 不稳定的尾 bar）。尾 bar 的 hist 由 state + 尾 close O(1) 派生。每 bar 追加：
/// - 长度相同（inclusion 吸收）：stable_prefix 不变 ⟹ state 有效，尾 bar hist 重算。
/// - 长度 +1（非包含定稿）：旧尾 bar 现稳定 ⟹ state append 旧尾 close，新尾 bar hist 派生。
///
/// ## 退化路径（cache 空 / 前缀改写 / 长度回缩）
///
/// 逐 bar `append` 重建 state + hist 数组（与全量同 EMA 约简顺序，bit-exact）。O(n) 单次，
/// 非 fallback——是 inclusion 回退的合法 bit-exact 退化（同塔 cache clear 逻辑）。
///
/// ## 认识论（formalization-validity-domain 231号）
///
/// L1：bit-exact 等价于全量 `compute_macd`（管线正确性，零 alpha 信息增量）。
/// 增量只把同一浮点约简从「全量重算」改成「逐 bar 延伸」，数值不变。
/// closes/close_src 增量缓存更新（231号纯性能，bit-exact 铁律）。
///
/// 维护 `cache.closes == merged_bars.iter().map(|b| b.close as f64).collect()` 与
/// `cache.close_src == merged_bars.iter().map(|b| b.source_index).collect()`，逐元素 bit-identical。
///
/// ## per-bar substrate 语义（inclusion 尾部不稳定性，同 `compute_macd_hist_incremental`）
///
/// `merged_bars` = confirmed 稳定前缀 + 当前未定稿合并段 `acc`（尾元素，可能被后续 bar 吸收改写）。
/// 故 `merged_bars[..len-1]` 跨 bar 稳定（confirmed 前缀不动），仅 `merged_bars[len-1]`（acc）可能改写。
/// ⟹ 缓存只需：截掉尾元素（重算的边界），从稳定前缀末续推 close/source_index。
///
/// - 长度 +k（新 bar 定稿）：旧尾现稳定 ⟹ 续 push 新元素。
/// - 长度不变（inclusion 吸收）：尾元素可能改写 ⟹ 截尾重 push。
/// - 长度回缩 / 前缀改写：全量重建（bit-exact 退化，同 cache clear 逻辑）。
fn update_closes_cache(
    merged_bars: &[super::types::Bar],
    confirmed_len: usize,
    cache: &mut TowerCache,
) {
    let n = merged_bars.len();
    let cached = cache.closes.len();

    // ★#106 O(1) 证书路径（替代旧每 bar O(n) 前缀全量比较 = O(n²) 主导根因，profile 坐实
    // 200K=15.3s/67%）：parser `merged_confirmed_len` 保证 `merged_bars[..confirmed_len]` 跨 bar
    // **物理不变**（相 B append_folded 仅 Rc::make_mut pop/push 末根，前缀字节不动；codex 锚定）。
    // ⟹ `cache.closes[..confirmed_len]` 与 `merged_bars[..confirmed_len]` 逐值一致（close + source_index
    // 双稳定，codex 3a/3b 满足——同一 Bar 物理不变两字段都不变），无需 O(n) 比较。
    //
    // bit-exact 前提（codex 血缘）：cache 由 classifier 每 bar 同源维护（生产路径每 bar 调 classify_at），
    // confirmed_len 单调 ⟹ cache[..confirmed_len] 是上轮前缀。前提失守（cached < confirmed_len，
    // 跨 bar 漏调 / 血缘断裂）⟹ reuse = min(cached, confirmed_len)，余下全量重扫（保守 bit-exact）。
    // confirmed_len=0（相 A / 相 A→B fold_all 整段重写 / 全量 parse_layer 无血缘）⟹ reuse=0 = 全量重建。
    let reuse = confirmed_len.min(cached).min(cache.close_src.len());
    cache.closes.truncate(reuse);
    cache.close_src.truncate(reuse);
    cache.closes.reserve(n.saturating_sub(reuse));
    cache.close_src.reserve(n.saturating_sub(reuse));
    for b in &merged_bars[reuse..] {
        cache.closes.push(b.close as f64);
        cache.close_src.push(b.source_index);
    }
}

fn compute_macd_hist_incremental(
    closes: &[f64],
    confirmed_len: usize,
    cfg: &super::config::MacdConfig,
    cache: &mut TowerCache,
) {
    // 空 closes：无 MACD 可算，清空 cache MACD 域（防御性，classify 入口已防空 layer）。
    if closes.is_empty() {
        cache.macd_state = None;
        cache.macd_hist.clear();
        cache.macd_state_len = 0;
        return;
    }

    // 单 bar：state = init(closes[0])，hist = [0.0]（首 bar DIF=DEA=hist=0）。state 覆盖 closes[..1]。
    if closes.len() == 1 {
        let state = MacdState::init(closes[0], cfg);
        cache.macd_hist = vec![state.current_point().hist];
        cache.macd_state = Some(state);
        cache.macd_state_len = 1;
        return;
    }

    // 稳定前缀长度（排除不稳定的尾 bar）。state 续推目标 = closes[..stable_prefix]。
    let stable_prefix = closes.len() - 1;

    // ★#106 O(1) 证书增量（替代旧每 bar O(n) 前缀比较 + `closes.to_vec()` 全量克隆 = O(n²)，
    // profile 坐实 200K=4.7s/21%）：复用边界 = `min(macd_state_len, confirmed_len)`。
    // - `macd_state_len`：state 已消费的 close 数（state 表示 closes[..macd_state_len] 累积）。
    // - `confirmed_len`：parser 证书——closes[..confirmed_len] 跨 bar 物理不变（前缀未被 inclusion
    //   改写）。取 min ⟹ state 覆盖的部分**全在稳定前缀内** ⟹ 续推不被改写污染（codex 血缘修复：
    //   绑 state_len 非 cached_len-1）。
    // resume_from > stable_prefix 不可能（resume_from <= macd_state_len <= 上轮 stable < 本轮 stable）；
    // 但 confirmed_len 收缩（相 A→B fold_all=0）⟹ resume_from=0 ⟹ 从头全量重推（bit-exact 退化）。
    let resume_from = cache.macd_state_len.min(confirmed_len).min(stable_prefix);

    let mut state = if resume_from > 0 && cache.macd_state.is_some() {
        // 增量：从 resume_from 的 state 续推。需要 state 恰好表示 closes[..resume_from]——
        // 若 macd_state_len > resume_from（confirmed_len 收缩截断），state 比 resume_from 多消费了
        // 已失效的 close ⟹ 不能直接用，须从头重推。故仅 macd_state_len == resume_from 时复用。
        if cache.macd_state_len == resume_from {
            cache.macd_hist.truncate(resume_from);
            cache.macd_state.clone().expect("is_some 已判")
        } else {
            cache.macd_hist.clear();
            rebuild_macd_state_to(closes, resume_from, cfg, &mut cache.macd_hist)
        }
    } else {
        // 全量重建（resume_from=0 或 state 空）。
        cache.macd_hist.clear();
        rebuild_macd_state_to(closes, 0, cfg, &mut cache.macd_hist)
    };

    // 续推 closes[hist.len()..stable_prefix]（新稳定 bar）+ 尾 bar（不稳定）hist。
    for &c in &closes[cache.macd_hist.len()..stable_prefix] {
        state = divergence::compute_macd_append(&state, c);
        cache.macd_hist.push(state.current_point().hist);
    }
    let tail_state = divergence::compute_macd_append(&state, closes[stable_prefix]);
    cache.macd_hist.push(tail_state.current_point().hist);
    cache.macd_state = Some(state);
    cache.macd_state_len = stable_prefix;
}

/// MACD state 重建到 `closes[..target]`（target=0 ⟹ init(closes[0])，state_len=1）。
/// `hist` 被 push 至 len==max(target,1)（首 bar hist=0 + 续 bar）。返回 closes[..hist.len()] 的 state。
/// bit-exact：与全量 `compute_macd` 同 EMA 约简（逐 bar append）。
fn rebuild_macd_state_to(
    closes: &[f64],
    target: usize,
    cfg: &super::config::MacdConfig,
    hist: &mut Vec<f64>,
) -> MacdState {
    let mut state = MacdState::init(closes[0], cfg);
    hist.push(state.current_point().hist);
    let end = target.max(1);
    for &c in &closes[1..end] {
        state = divergence::compute_macd_append(&state, c);
        hist.push(state.current_point().hist);
    }
    state
}

// ════════════════════════════════════════════════════════════════════════════
//  阶段计时插桩（profile-only，#106 真热点定位）
// ════════════════════════════════════════════════════════════════════════════
//
// thread_local 累加器，env `THETA_PROFILE_STAGES=1` 时启用。只测时间不改逻辑（bit-exact 安全）。
pub mod stage_profile {
    use std::cell::RefCell;
    use std::time::{Duration, Instant};

    thread_local! {
        static ACC: RefCell<Vec<(&'static str, Duration)>> = const { RefCell::new(Vec::new()) };
        static ENABLED: bool = std::env::var("THETA_PROFILE_STAGES").is_ok();
    }

    pub fn enabled() -> bool {
        ENABLED.with(|e| *e)
    }

    pub struct Guard {
        label: &'static str,
        start: Instant,
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            let d = self.start.elapsed();
            let label = self.label;
            ACC.with(|a| {
                let mut a = a.borrow_mut();
                if let Some(slot) = a.iter_mut().find(|(l, _)| *l == label) {
                    slot.1 += d;
                } else {
                    a.push((label, d));
                }
            });
        }
    }

    pub fn stage(label: &'static str) -> Option<Guard> {
        if enabled() {
            Some(Guard { label, start: Instant::now() })
        } else {
            None
        }
    }

    /// 计时一个表达式（闭包包裹；env 未启用时零开销直通）。
    pub fn time<T>(label: &'static str, f: impl FnOnce() -> T) -> T {
        if !enabled() {
            return f();
        }
        let start = Instant::now();
        let r = f();
        let d = start.elapsed();
        ACC.with(|a| {
            let mut a = a.borrow_mut();
            if let Some(slot) = a.iter_mut().find(|(l, _)| *l == label) {
                slot.1 += d;
            } else {
                a.push((label, d));
            }
        });
        r
    }

    pub fn dump() {
        if !enabled() {
            return;
        }
        ACC.with(|a| {
            let a = a.borrow();
            eprintln!("=== THETA STAGE PROFILE ===");
            let mut rows: Vec<_> = a.iter().collect();
            rows.sort_by_key(|(_, d)| std::cmp::Reverse(*d));
            for (label, d) in rows {
                eprintln!("  {:<36} {:>10.3} ms", label, d.as_secs_f64() * 1000.0);
            }
            eprintln!("===========================");
        });
    }
}

/// ★增量塔入口：返回 `(Classification, tower_snapshots)` bit-exact 等价于
/// `classify_with_tower(l0, config)`，但塔构造的中枢扫描走增量 resume（前级 confirmed 前缀缓存，
/// 仅尾部续扫），解 per-bar substrate 的塔构造 O(n²) 根因。
///
/// ## bit-exact 保证（#93 铁律）
///
/// 输出 `(Classification, Vec<Vec<LeveledMove>>)` 与 `classify_with_tower(l0, config)` 逐字段
/// bit-identical：
/// - `Classification.levels[k].centers`：增量累积的中枢序列 == 全量 `detect_centers`（resume bit-exact，
///   见 recursive_tower.rs 证明）。
/// - `Classification.levels[k].moves`：从累积 centers 经 `classify_move` 裁决 == 全量裁决（同 centers 输入）。
/// - `Classification.levels[k].bsp`：从累积 centers + segments/hist 经同口径提取 == 全量提取。
/// - `tower_snapshots[k]`：本级 compose 前的 `moves_tower`，前缀来自缓存 + 尾部续扫 == 全量 compose。
///
/// ## 硬契约（#106 证书路径，codex 双轮审计锚定）
///
/// `cache: &mut TowerCache` 必须与 `l0` **同源逐 bar 推进**——即同一 `ParseLayerIncr` 血缘、同一
/// `ThetaConfig`、每 bar 调用一次（无跳 bar、无跨数据流复用、config 不变）。`merged_confirmed_len`/
/// `segments_confirmed_len` 证书只保证「本 parser 自己的前缀稳定」，不验证 cache 内旧前缀与本轮 l0 同源。
/// 违反（同 cache 跑两条流不 clear / 中途换 config）⟹ MACD/L0 tower/closes/frontier 复用陈旧前缀
/// （bit-exact 破裂）。生产路径满足：`IncrementalClassifier::new` 每实例新建 TowerCache + 同源逐 bar。
/// 换流/换 config 的调用方须先 `cache.clear()`。
/// ponytail: 不加运行时 lineage epoch——生产 IncrementalClassifier 结构保证同源，epoch 是为不存在的
/// 滥用场景加防御（YAGNI）；契约由本文档 + clear() 入口声明。
/// ## 增量有效性（exp≈1 前提）
///
/// 段账本单调追加（前缀稳定）时，每级扫描从 `consumed` 续扫 O(tail) 而非 O(units) ⟹ 塔构造总扫描
/// O(Σ tail) = O(n)（amortized）。段账本前缀回缩时自动退化为全量（`clear` + 重扫），仍 bit-exact。
///
/// ## 边界
///
/// - 空 ParseLayer（无线段）⟹ `Classification::default()` + 空 tower，cache 清空。
/// - 段账本前缩（`segments.len() < last_l0_segments_len`）⟹ cache 清空 + 全量重扫（bit-exact 退化）。
/// - merged_bars 前缀改写（inclusion 合并回退）⟹ MACD cache 局部重建（bit-exact 退化）。
pub fn classify_with_tower_incremental(
    l0: &ParseLayer,
    config: &ThetaConfig,
    cache: &mut TowerCache,
) -> (Classification, Vec<Rc<Vec<LeveledMove>>>) {
    let min_parts = config.level.min_parts_per_level as usize;
    let l_max = config.level.l_max as usize;

    // L0 输入单元 = parser 线段账本。#106 证书增量（同 moves_tower_l0，消除每 bar 全量 collect）。
    stage_profile::time("00_l0_units_build", || {
        let reuse = l0.segments_confirmed_len.min(cache.l0_units_cache.len());
        cache.l0_units_cache.truncate(reuse);
        cache.l0_units_cache.extend(l0.segments[reuse..].iter().map(segment_to_unit));
    });
    let l0_units: Vec<UnitRange> = cache.l0_units_cache.clone();

    // DIAG(frontier-bit-exact): 对拍复用版 l0_units_cache vs 全量 segment_to_unit（隔离 L0 units 前缀复用是否陈旧）。
    if std::env::var("DIAG_L0UNITS").is_ok() {
        let full: Vec<UnitRange> = l0.segments.iter().map(segment_to_unit).collect();
        if l0_units != full {
            let m = l0_units.len().min(full.len());
            let first = (0..m).find(|&i| l0_units[i] != full[i]);
            eprintln!(
                "[DIAG-L0UNITS] segs={} confirmed_len={} cache.len(reuse前)={} ★l0_units陈旧 first_diff={:?} \
                 lens=({},{})",
                l0.segments.len(), l0.segments_confirmed_len, cache.l0_units_cache.len(),
                first, l0_units.len(), full.len()
            );
        }
    }

    // 空 L0：无可构造级别 + 清空缓存（下次从头扫）。
    if l0_units.is_empty() {
        cache.clear();
        return (Classification::default(), Vec::new());
    }

    // 前缀不变量校验：段账本回缩 ⟹ 清空重扫（bit-exact 退化，非增量）。
    if l0.segments.len() < cache.last_l0_segments_len {
        cache.clear();
    }
    cache.last_l0_segments_len = l0.segments.len();

    // L0 走势塔 = 携坐标的 RMove::Segment（递归底）。
    // ★#106 证书增量（替代每 bar 全量 from_unit 重建 = O(segs)×n，profile 坐实 400K=5.8s）：
    // parser `segments_confirmed_len` 保证 segments[..confirmed_len] 跨 bar bit-stable（仅末段可古怪
    // 线段重划，codex 确认）⟹ moves_tower_l0[..reuse] 复用（from_unit 只依赖单 seg，无相邻依赖）。
    // ordinal=reuse+i 全局索引（前缀 reuse<=confirmed_len 时 ordinal 不变 = 全量 enumerate，bit-exact）。
    // make_mut：caller 逐 bar drop 上轮 tower_snapshots[0] ⟹ strong_count==1 ⟹ 原地 O(tail)；
    // >1（理论 caller 跨 bar 持有）⟹ 写时复制（仍 bit-exact）。clear() 已同步清空（退化全量）。
    stage_profile::time("01_l0_tower_rebuild", || {
        let reuse = l0.segments_confirmed_len.min(cache.moves_tower_l0.len());
        let m = Rc::make_mut(&mut cache.moves_tower_l0);
        m.truncate(reuse);
        for (off, seg) in l0.segments[reuse..].iter().enumerate() {
            let i = reuse + off;
            let u = segment_to_unit(seg);
            m.push(LeveledMove::from_unit(&u, ElementId { level: 0, ordinal: i as u64 }));
        }
    });
    let moves_tower_l0: Rc<Vec<LeveledMove>> = Rc::clone(&cache.moves_tower_l0);

    // MACD hist（背驰真算，增量递推——231号纯性能，解 aed4d5f5 实证的 c_exp≈2.31 主导根因）。
    //
    // 增量策略（bit-exact 铁律，ac75d4b3 已证 append bit-exact）：
    // - merged_bars.close 前缀与 `macd_closes_prefix` 逐值一致 + 长度 >= 前缀 ⟹ 从
    //   `macd_state` 增量 append 新 close，`current_point().hist` 追加到 `macd_hist`。
    // - merged_bars 回缩 / 前缀改写 / cache 空 ⟹ `macd_state_from_closes` 全量重建
    //   state，并逐 bar append 重建 hist 数组（bit-exact 退化，同塔 cache clear 逻辑）。
    //
    // ★不调 `compute_macd`（全量）——增量路径用 `MacdState::append` O(1)/bar；退化路径
    //   用 `macd_state_from_closes` + 逐 bar `current_point`（与全量同 EMA 约简，bit-exact）。
    // closes/close_src 增量缓存（前缀稳定，仅尾 bar 可能 inclusion 改写——同 MACD stable_prefix 语义）。
    // 不每 bar 全量重建（O(n)/bar → O(n²) 根因之一，工位 E profile 坐实 t=0.085s@16K）。
    // mem::take 取出缓存 Vec（避免 &cache.closes 与下游 &mut cache 别名），用毕放回（缓冲复用，零额外分配）。
    stage_profile::time("00_update_closes_cache", || {
        update_closes_cache(&l0.merged_bars, l0.merged_confirmed_len, cache)
    });
    let closes: Vec<f64> = std::mem::take(&mut cache.closes);
    let close_src: Vec<usize> = std::mem::take(&mut cache.close_src);
    // 增量 MACD：更新 cache.macd_hist（不返回克隆，直接借用缓存避免 O(n) 拷贝）。
    stage_profile::time("02_macd_incremental", || {
        compute_macd_hist_incremental(&closes, l0.merged_confirmed_len, &config.macd, cache)
    });
    let hist: &[f64] = &cache.macd_hist;

    let mut levels: Vec<LevelState> = Vec::new();
    // ★O(n) 重构：snapshots 存 Rc——L≥1 级 push Rc::clone(&lc.upper_moves)（O(1)）；L0 级 push
    // moves_tower_l0（Rc）。下游（runner/interp/l3）只读借 &[Rc<Vec<LeveledMove>>]。
    let mut tower_snapshots: Vec<Rc<Vec<LeveledMove>>> = Vec::new();

    // 逐级增量构造。`units`/`moves_tower` 是本级输入（前缀来自缓存，尾部新增）。
    let mut units: Vec<UnitRange> = l0_units;
    let mut moves_tower: Rc<Vec<LeveledMove>> = moves_tower_l0;

    // ★cascade reset（codex 异质审查裁决：option 2）：任一级检出 frontier 变异 ⟹ 该级**及所有更高级**
    // 无条件 reset。根因：本级守卫只比对 `project_to_units` 投影（有损——丢弃 `sub_moves`/`rmove.subs`）。
    // 当 L0 末段内点改写使上级投影 bit-identical 但底层 sub_moves 变时，上级 frontier_mutated=false 会
    // 漏 reset ⟹ `upper_moves` 深嵌套 sub_moves 陈旧 + BSP memo 复用陈旧（codex 反例
    // `cascade_reset_on_frontier_interior_rewrite` 坐实）。cascade 消除整类"投影是否捕获深字段变化"
    // 的易错判断（no-patch）：下级变异无条件向上传播，上级不依赖投影完备性。代价：变异 bar（16K 中
    // ~120 次稀疏）该级+所有上级全量重扫，amortized 仍 O(n)。
    let mut cascade_reset = false;
    // ★工位 4g：本 bar 是否有任一级 extend 非空 tail（含新级涌现首产 + 最高级 append）——驱动
    // generation +1（与 cascade_reset 一起完整覆盖 extract 可观察树变更，codex Q3）。
    let mut did_extend = false;

    for level_idx in 0..=l_max {
        // 自然终止：单元数 < min_parts ⟹ 停止（与 classify_impl 同口径）。
        if units.len() < min_parts {
            break;
        }

        let is_l0 = level_idx == 0;

        // 缓存槽按需扩展（首次到达该级 ⟹ 新建空 LevelCache，start_i=0 全量扫）。
        if cache.levels.len() <= level_idx {
            cache.levels.push(LevelCache::default());
        }
        let lc = &mut cache.levels[level_idx];

        // 前缀不变量校验（anc.pdf §16：confirmed prefix immutable / frontier mutable）。
        // 两类违反 resume 充要条件 #2（`units[..consumed]` 不可变）⟹ 该级缓存全量重置（退化为
        // 全量扫，bit-exact）：
        //   (1) 长度回缩：units.len() < last_input_len（段账本前缩）。
        //   (2) frontier 末段原地改写：长度不变/增长但**扫描区**（`units[..consumed+2]`，已 build 读过
        //       的单元）内某单元值变了——parser 古怪线段重划改写末段（定义层正确，非 bug）。仅长度
        //       守卫漏此例（bar 1464 seg[9] end 1384→1170，段数不变）。扫描区外（未读尾部）的变化
        //       无害（resume 从 consumed 续扫会读到新值），不触发重置。
        let scanned = (lc.scan_cursor.consumed + 2).min(units.len()).min(lc.cached_units.len());
        // #106 证书跳前缀（仅 L0 安全，codex 裁决）：L0 units = segments 投影，segments[..confirmed_len]
        // 跨 bar bit-stable ⟹ units[..stable] 必等，只比 [stable..scanned]。L1+ units 来自上级投影
        // （无 segments 证书）⟹ stable=0 全量比较（保守，不假定相等）。证书不证明 [stable..scanned]
        // 没变（bar-1464 末段改写在 scanned frontier 内）⟹ 该区间仍逐值比，触发 cascade。
        let stable = if is_l0 {
            l0.segments_confirmed_len.min(scanned)
        } else {
            0
        };
        let frontier_mutated = stage_profile::time("03_frontier_compare", || {
            units[stable..scanned] != lc.cached_units[stable..scanned]
        });
        // 本级触发 reset ⟹ 置 cascade，所有更高级无条件跟随（投影有损，上级不能仅靠本级投影判断）。
        if units.len() < lc.last_input_len || frontier_mutated {
            cascade_reset = true;
        }
        if cascade_reset {
            lc.scan_cursor = WindowScanCursor::default();
            // make_mut：若上 bar snapshot 仍持引用则写时复制再 clear（退化 bit-exact）；否则原地清。
            Rc::make_mut(&mut lc.upper_moves).clear();
            lc.centers.clear();
            lc.cached_outcome = None;
            lc.cached_bsp.clear();
            lc.cached_bsp_key = None;
            lc.projected_units.clear(); // #106：upper_moves 前缀重排 ⟹ 投影缓存失效，重投影。
        }
        lc.last_input_len = units.len();
        // 快照本级输入（下 bar 比对 frontier 变异）。clone 是 O(units) memcpy（UnitRange: Copy）；
        // amortized 仍 O(n)——本就每 bar 全量重建 units（line 857 project_to_units）。
        stage_profile::time("04_cached_units_copy", || {
            lc.cached_units.clear();
            lc.cached_units.extend_from_slice(&units);
        });

        // ★frontier 修复（task #47/#21，区间套.pdf 六~十节裁决②）：resume 起点用 `resume_from`
        // （上次扫描最后一个成立窗口的起点）**而非** `consumed`——`consumed` 越过了最后一个成立
        // 窗口，把它当 sealed prefix，但该窗口第三段可能是 frontier（新 bar 后其后续段落使全量非
        // 重叠扫描产出不同中枢）。回退：从 `resume_from` 重扫 ⟹ 最后一个（frontier）中枢每 bar 重算，
        // 真正 sealed（后面又出现成立窗口）后自然稳定。对应地 **pop 最后一个 center/upper_move**
        // （它会被重扫重新产出），`prefix_count` 用 pop 后的长度（ordinal 接续，全量/增量同 ID）。
        //
        // 保守正确性（PDF §九增量等价定理）：`resume_from <= consumed` 恒成立（窗口起点 ≤ 退出点），
        // 从更早处重扫产出的 tail ⊇ 从 consumed 重扫的 tail（多含重算的末窗口）。cascade_reset 后
        // `scan_cursor = default`（resume_from=0=consumed）⟹ 无回退，从 0 全扫（bit-exact 退化）。
        // 无成立窗口时 `resume_from == 上次 start_i`（无中枢可 pop），guard `resume_from < consumed`
        // 为假 ⟹ 不 pop、从 resume_from(=上次start_i) 续扫（续进语义，仅不成立支推进过的区间）。
        let resume_start = lc.scan_cursor.resume_from;
        let had_emitted_window = lc.scan_cursor.resume_from < lc.scan_cursor.consumed;
        if had_emitted_window {
            // pop 最后一个中枢（frontier 中枢，重扫会重新产出）——前缀不变量不破（pop 的是尾部）。
            debug_assert!(
                !lc.centers.is_empty() && !lc.upper_moves.is_empty(),
                "had_emitted_window ⟹ 至少一个已产出中枢可回退"
            );
            lc.centers.pop();
            Rc::make_mut(&mut lc.upper_moves).pop();
        }
        // ★codex Q4：prefix_count = lc.upper_moves.len()（pop 后的已产出前缀数），tail ordinal 接续
        // 前缀 ⟹ 全量/增量产同 ElementId（跨 bar 稳定身份）。
        let (tail_centers, tail_upper, new_cursor) =
            stage_profile::time("05_compose_resume", || {
                compose_level_resume(
                    &units,
                    &moves_tower[..],
                    is_l0,
                    level_idx as u32 + 1,
                    resume_start,
                    lc.upper_moves.len(),
                )
            });

        // 追加到已缓存前缀（前缀不可变，仅尾部追加）⟹ 累积 centers/upper == 全量扫描结果。
        did_extend |= !tail_upper.is_empty();
        stage_profile::time("06_extend_centers_upper", || {
            lc.centers.extend(tail_centers);
            // make_mut：strong_count==1 ⟹ 原地 extend O(tail)；>1 ⟹ 写时复制（bit-exact）。
            Rc::make_mut(&mut lc.upper_moves).extend(tail_upper);
        });
        lc.scan_cursor = new_cursor;
        debug_assert!(
            lc.centers.len() == lc.upper_moves.len(),
            "增量塔：centers 与 upper_moves 一一对应（每窗口一中枢）"
        );

        // 本级输入塔快照（compose 前）。
        // ★O(1) 优化：moves_tower 是 Rc——move 入 snapshots（所有权转移，零拷贝）。下一级用
        // Rc::clone(&lc.upper_moves) 重置 moves_tower（line 912），故此处 move 后 moves_tower 失效合法。
        // bit-exact：snapshots 内容 == 全量版（Rc 指向的 Vec 值不变，仅所有权/引用计数变）。
        tower_snapshots.push(std::mem::take(&mut moves_tower));

        // 走势裁决（增量续算：从 cached_outcome + 新尾对 O(1) 续判，与全量 classify_move bit-exact）。
        let outcome = classify_move_incremental(&lc.centers, &mut lc.cached_outcome);
        let moves: Vec<MoveKind> = outcome_to_kind(outcome).into_iter().collect();

        // BSP 提取（同 classify_impl：L0 线段层 + 递归组装层）。
        // ★增量接入：传预计算 hist（从 cache 增量产出），避免 extract_signals 内部全量 compute_macd。
        //
        // ★memo 缓存（工位 E，O(n²) 主导根因——profile 坐实 bsp t=0.279s@16K，53% of tower）：
        // BSP 是 (centers, segments, upper_moves, hist, close_src) 的纯函数。其中 centers/segments/
        // upper_moves 跨 bar 单调追加（前缀不可变）；hist/close_src 仅尾 bar（不稳定，inclusion 可改写）
        // 变化。但 **confirmed 线段/中枢/上级走势的 source_index 区间全部落在稳定前缀**——尾 bar 在
        // 所有 confirmed 元素区间之后，故不影响其 BSP 判定。⟹ 当 (centers.len, segments.len,
        // upper_moves.len) 三者与上次一致时，BSP 逐字段 bit-identical（纯函数同输入同输出 + 尾 bar 不
        // 触及 confirmed 区间）。实测 16K bar 中 L0 segments 仅变 120 次（maxseg=134），其余 ~99.2%
        // bar 全量重算是冗余——memo 把 16K 次重算降为 O(段变化次数) 次，每次 O(S)，BSP 总成本坍缩近常数。
        //
        // bit-exact 铁律：guard 命中 ⟹ 复用上次输出（纯函数同输入）；guard miss ⟹ 全量重算并刷新
        // 缓存。与无 memo 版逐字段相等（同 extract_signals_with_hist/extract_second_for_level 代码路径）。
        let seg_len = if is_l0 { l0.segments.len() } else { 0 };
        let bsp_key = (lc.centers.len(), lc.upper_moves.len(), seg_len);
        let bsp: Vec<BspPoint> = if lc.cached_bsp_key == Some(bsp_key) {
            lc.cached_bsp.clone()
        } else {
            let mut b: Vec<BspPoint> = if is_l0 {
                stage_profile::time("07a_extract_signals_l0", || {
                    signal::extract_signals_with_hist(&lc.centers, &l0.segments, hist, &close_src)
                })
            } else {
                Vec::new()
            };
            let second = stage_profile::time("07b_extract_second", || {
                extract_second_for_level(&lc.upper_moves[..], hist, &close_src)
            });
            b.extend(second);
            b.sort_by_key(|p| p.source_index);
            lc.cached_bsp = b.clone();
            lc.cached_bsp_key = Some(bsp_key);
            b
        };

        let level_centers = stage_profile::time("08_levels_centers_clone", || lc.centers.clone());
        levels.push(LevelState {
            moves,
            centers: level_centers,
            bsp,
        });

        // 下一级输入 = 上级走势塔投影（前缀来自缓存 upper_moves 前缀，尾部来自续扫）。
        // ★O(n²) 真修（#106）：增量投影——只对 upper_moves 新 tail 投影（`rmove.lo()/hi()` 递归整棵
        // 子树 O(nodes) 仅算新元素），前缀复用 lc.projected_units。clone 给 units 是 O(level) memcpy
        // （UnitRange: Copy，无递归）。cascade_reset 已清空 projected_units（line 855 旁）⟹ 退化全量。
        stage_profile::time("09_project_to_units_resume", || {
            recursive_tower::project_to_units_resume(&lc.upper_moves[..], &mut lc.projected_units);
        });
        units = stage_profile::time("10_projected_units_clone", || lc.projected_units.clone());

        if units.is_empty() {
            break;
        }
        // ★O(n) 重构：moves_tower = Rc::clone(&lc.upper_moves) —— O(1) 引用计数，消除 per-bar 全塔
        // 深拷贝（旧 `lc.upper_moves.clone()` 是 O(n²) 热点①根因）。下一级 line 832 借 &moves_tower[..]
        // 只读，line 854 move 入 snapshots。lc.upper_moves 跨 bar 持久于 cache；extend（line 843）经
        // make_mut，caller 逐 bar drop snapshot ⟹ strong_count==1 ⟹ 原地 O(tail)。
        // bit-exact：Rc 指向同一 Vec，逐字段与旧 clone 等价。
        moves_tower = Rc::clone(&lc.upper_moves);
    }

    // closes/close_src 缓冲放回 cache（mem::take 取出的所有权归还，下 bar 复用，零额外分配）。
    cache.closes = closes;
    cache.close_src = close_src;

    // ★工位 4g：本 bar 若有 cascade 重扫或任一级 extend 非空 tail ⟹ extract_elements 可观察树变更 ⟹
    // +generation（下游 TreeCache 据此 O(1) 跳过 TreeKey::of）。无变化 bar（~99.8%）generation 不变 ⟹
    // 命中。soundness：保守过度计数（低级 extend 不动最高级输出时多算一次 TreeKey）安全，绝不假命中。
    //
    // ★L0-root 边界（codex Q3 漏洞 + gen_fastpath_bit_exact_debug bar 345 坐实）：当最高非空级是 **L0**
    // （tower 无 compose 级，extract 直接读 `moves_tower_l0`），L0 走势塔每 bar 从 `l0.segments` 重建
    // （非缓存 Rc），其增长/同 len 古怪线段重划**不经** cascade/did_extend（那两者只覆盖各级 upper_moves）。
    // 故 L0-root 阶段**强制每 bar +generation**（走 TreeKey fallback）——此阶段 tree 极小（早期 bar），
    // TreeKey O(small) 不影响大 n 标度。一旦 L1+ 出现（extract 读 L1），L0 任何变化必经 cascade 传播至
    // L1（codex Q1：L0 frontier 改写→L1 units 投影变→L1 frontier_mutated→cascade），被完整捕获。
    let highest_nonempty = tower_snapshots.iter().rposition(|s| !s.is_empty());
    let l0_is_root = highest_nonempty == Some(0);
    if cascade_reset || did_extend || l0_is_root {
        cache.generation += 1;
    }

    (Classification { levels }, tower_snapshots)
}

/// 递归组装层第二类提取（对一级的每个上级走势 `RMove::Compose` 产 B2/S2）。
///
/// ★#53 接入点（still-MISSING-塔解除）：对本级每个上级走势 `LeveledMove`（`RMove::Compose`），
/// 双侧（Long/Short）调 `signal::extract_second_signals`——从 `descend parent` 取回的次级别走势
/// 序列内识别第二类走势结构（第一类离开 + 回拉不创新低/新高，§10.2 买卖点定律一）。产出的 B2/S2
/// 端点零改动接入生产路径。
///
/// 三个闭包参数的真实接入（非占位）：
/// - `c1`（次级别中枢）：`RMove::Compose.centers` 的首个中枢（窗口三段区间重叠真派生，B 口径核心
///   区间）——`find_second_type_structure` 用它判次级别第一类离开是否破中枢。
/// - `divergence_of`（MACD 背驰）：次级别走势的 source_index 区间 → `hist` 面积，相对**前一同向次
///   级别走势**面积严格变小（reference:34 背驰，`divergence.rs` 真算 L1）。
/// - `index_of`（坐标）：从坐标侧车 `subs`（携 source_index 的 `LeveledMove`）按结构身份查回原始
///   K 序（`index_of_in`）——B2/S2 的 `source_index` 真坐标（still-MISSING-坐标解除）。
///
/// ★诚实 still-MISSING（背驰力度引擎配对，no-声明膨胀）：`divergence_of` 对次级别走势的「前一同向
/// 走势」配对用**序列序最近同向前驱**（与 signal.rs `extract_first_for_center` 同口径）——次级别
/// 走势的 close 区间由 source_index 坐标定位到 `hist`。L1 管线正确性（MACD 面积比较确定），**不**是
/// 「背驰预测在真实行情有效」（L2/L3，不在本层）。
fn extract_second_for_level(
    upper_moves: &[LeveledMove],
    hist: &[f64],
    close_src: &[usize],
) -> Vec<BspPoint> {
    let mut points = Vec::new();
    for parent in upper_moves {
        // 次级别中枢（RMove::Compose.centers 首个，窗口真派生 B 口径核心区间）。
        let c1 = match &parent.rmove {
            descend::RMove::Compose { centers, .. } => match centers.first() {
                Some(c) => *c,
                None => continue, // 无中枢载荷 ⟹ 跳过（compose_level 必带中枢，防御性）。
            },
            // L0 线段（递归底）不会出现在 upper_moves（compose_level 只产 Compose），防御性跳过。
            descend::RMove::Segment { .. } => continue,
        };
        // 坐标侧车：构成 parent 的次级别 LeveledMove 序列（与 descend parent 同序同长）。
        let subs = descend_leveled(parent);
        // 双侧识别第二类结构（B2=Long / S2=Short），各产至多一个端点。
        for side in [Side::Long, Side::Short] {
            let pts = signal::extract_second_signals(
                &parent.rmove,
                side,
                &c1,
                // 背驰：次级别走势 source_index 区间 → hist 面积，相对前一同向次级别走势严格变小。
                |m| sublevel_diverges(m, &subs[..], hist, close_src),
                // 坐标：从侧车按结构身份查回次级别走势的原始 K 序（end_index）。
                |m| index_of_in(&subs[..], m),
            );
            points.extend(pts);
        }
    }
    points
}

/// 次级别走势的 MACD 背驰判定（reference:34，`divergence.rs` 真算 L1）。
///
/// 给定次级别走势 `m`（descend 取回的 `RMove`）+ 坐标侧车 `subs`：定位 `m` 在 `subs` 中的位置，
/// 取其 source_index 区间 → `hist` 面积，相对**序列序最近同向次级别前驱走势**面积严格变小 ⟹ 背驰。
/// 同向 = 走势的 direction 相同（`RMove::Segment.direction`；`Compose` 走势取外缘趋势方向占位）。
///
/// ★诚实 still-MISSING（背驰力度引擎）：无前同向走势（`m` 是序列首个该向走势）⟹ 无背驰对照
/// ⟹ false（与 signal.rs `extract_first_for_center` 同口径——第一类是趋势末段必有前同向段）。
/// 无法定位 source_index 区间到 `hist`（坐标越界）⟹ false（不冒充背驰）。
fn sublevel_diverges(
    m: &descend::RMove,
    subs: &[LeveledMove],
    hist: &[f64],
    close_src: &[usize],
) -> bool {
    // 定位 m 在 subs 中的位置（结构身份匹配）。
    let Some(idx) = subs.iter().position(|x| &x.rmove == m) else {
        return false; // m 不在 subs（防御性）⟹ 无坐标 ⟹ 非背驰。
    };
    let curr = &subs[idx];
    let curr_dir = rmove_direction(&curr.rmove);
    // 序列序最近同向前驱走势（reference:34「末段相对前同向段」的确定配对）。
    let Some(prev) = subs[..idx].iter().rev().find(|x| rmove_direction(&x.rmove) == curr_dir) else {
        return false; // 无前同向走势 ⟹ 无背驰对照 ⟹ 非第一类（趋势末段必有前同向段）。
    };
    // 两走势 source_index 区间 → hist 面积比较（curr < prev ⟹ 背驰，divergence.rs 真算）。
    let (Some(curr_seg), Some(prev_seg)) = (
        map_src_to_close_idx(close_src, curr.start_index, curr.end_index),
        map_src_to_close_idx(close_src, prev.start_index, prev.end_index),
    ) else {
        return false; // 区间越界/空 ⟹ 无面积 ⟹ 不冒充背驰。
    };
    divergence::segments_diverge(hist, prev_seg, curr_seg)
}

/// 走势方向（`RMove::Segment` 直接取 direction；`Compose` 取外缘趋势方向占位——首子升=Up）。
///
/// ★诚实有效域：`Compose` 走势的方向是**外缘占位**（subs 区间聚合趋势），用于背驰「同向段」配对的
/// 序列序判定。不冒充 §6.1 意义的线段方向交替（中枢检测用几何路径，不读方向，见 center.rs）。
fn rmove_direction(m: &descend::RMove) -> Direction {
    match m {
        descend::RMove::Segment { direction, .. } => *direction,
        // Compose 走势：外缘下沿 vs 上沿——hi 偏离 lo 多者为趋势向（占位，背驰同向配对用）。
        // subs 首尾区间趋势：末子 hi >= 首子 hi ⟹ Up（外缘上移），否则 Down。
        descend::RMove::Compose { subs, .. } => {
            match (subs.first(), subs.last()) {
                (Some(f), Some(l)) if l.hi() >= f.hi() => Direction::Up,
                (Some(_), Some(_)) => Direction::Down,
                _ => Direction::Up, // 空 subs ⟹ 缺省 Up（防御性）。
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::parser::ParseLayer;
    use super::super::types::Direction;

    fn seg(dir: Direction, si: usize, ei: usize, sp: i64, ep: i64) -> Segment {
        Segment { direction: dir, start_index: si, end_index: ei, start_price: sp, end_price: ep }
    }

    /// 构造 merged_bars：source_index 连续 0..n，close = vals（MACD 背驰真算用）。
    fn bars_from_closes(vals: &[i64]) -> Vec<super::super::types::Bar> {
        vals.iter()
            .enumerate()
            .map(|(i, &v)| super::super::types::Bar {
                source_index: i,
                timestamp: i as i64,
                open: v,
                high: v,
                low: v,
                close: v,
                volume: 1,
                untradable: false,
            })
            .collect()
    }

    /// ★端到端 B2 真产出（#53 验证门，L1 管线正确性）：升级后的递归塔（`RMove::Compose` 携 subs）
    /// 让 `extract_second_signals` **真接入生产路径**——classify 在真实结构输入上产出 B2 买点。
    ///
    /// 旧塔（`UnitRange` 无 subs）在**任何**输入上产 0 个 B2（descend 得空，结构上不可产）；新塔
    /// 在此输入上产 1 个 B2，坐实升级解除了 still-MISSING-塔。
    ///
    /// 路径（codex 异质裁决确认）：B2 在 **L1→L2 几何路径**产出——3 个同向 L1 走势经几何窗口
    /// （不强制方向交替）compose 成 L2 走势，其 descend 取回的 3 个 L1 走势内识别第二类结构：
    /// L1[1]（i1=1）破 L2 中枢 + MACD 背驰（相对前同向 L1[0]）= 第一类离开；L1[2]（i2=2）回拉不
    /// 创新低 = 第二类回拉走势。B2 端点 = L1[2] 的回拉结束点（坐标由 source_index 侧车真映射）。
    ///
    /// ★诚实边界（still-MISSING-窗口，codex 裁决坐实）：B2/S2 **不在 L0→L1 三段交替窗口产**——
    /// 三段方向交替窗口里可背驰的同向段只在位置 2（末段，无后继回拉），位置 0 无前同向对照，
    /// 位置 1 是唯一异向（无前同向）。这是固定三段封装的结构上界，非接入缺陷（接入逻辑双侧完整）。
    #[test]
    fn end_to_end_second_buy_via_l1_l2_geometric() {
        let cfg = ThetaConfig::default();
        // 9 段 L0：三组 up-down-up（每组 → 一个 L1 走势）。三个 L1 走势外缘重叠成 L2 中枢，
        // 但 L1[1] 向下深破核心下沿（第一类离开候选，Side::Long），L1[2] 回拉不创新低。
        // L1 走势外缘 = 组内三段 [dd,gg]：A=[110,150], B=[80,145], C=[115,148]。
        // L2 核心 = max(110,80,115)=115 .. min(150,145,148)=145 → [115,145] 非空（盘整 L2 中枢）。
        let segments = vec![
            // 组A（L1[0]）：up-down-up，外缘 [110,150]
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            // 组B（L1[1]）：up-down-up，外缘 [80,145]，lo=80 深破 L2 核心下沿 115
            seg(Direction::Up,  12, 16, 130, 145),
            seg(Direction::Down,16, 20, 145, 80),
            seg(Direction::Up,  20, 24, 80, 144),
            // 组C（L1[2]）：up-down-up，外缘 [115,148]，回拉不创新低（lo=115 >= L1[1].lo=80）
            seg(Direction::Up,  24, 28, 120, 148),
            seg(Direction::Down,28, 32, 148, 115),
            seg(Direction::Up,  32, 36, 115, 147),
        ];
        // closes 让 L1[1] 区间（source_index [12,24]）MACD 面积 < L1[0] 区间（[0,12]）= 背驰（真算）。
        // 前段大幅波动（面积大），后段小幅（面积小）。
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); } // L1[0] 大幅
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 5 } else { -5 }); }   // L1[1] 小幅（背驰）
        for i in 0..16 { closes.push(100 + if i % 2 == 0 { 3 } else { -3 }); }   // L1[2] 更小
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
        let out = classify(&layer, &cfg);

        // L1 级别（索引 1）含 L2 中枢 + B2（递归组装层产出）。
        assert!(out.levels.len() >= 2, "三组 L0 → L1 走势塔 → L2 中枢，至少 2 级");
        let l1 = &out.levels[1];
        assert_eq!(l1.centers.len(), 1, "3 个 L1 走势 → 1 个 L2 中枢（几何路径）");
        let second_buys: Vec<_> = l1.bsp.iter().filter(|p| p.bits.buy2).collect();
        assert_eq!(second_buys.len(), 1, "★升级后塔真产 B2（旧 UnitRange 塔产 0）");
        let b2 = second_buys[0];
        // B2 端点坐标由 source_index 侧车真映射（回拉走势 L1[2] 的 end_index=36）。
        assert_eq!(b2.source_index, 36, "B2 source_index = 回拉走势 L1[2] 的原始 K 序（坐标侧车真映射）");
        // 第二类止损 = 回拉低点（second_point = 回拉走势 m2.lo）；center=None（1/2 类用 pivot 非 center）。
        assert!(b2.pivot_low != 0, "B2 携结构止损价 pivot_low（回拉低点 single source）");
        assert!(b2.center.is_none(), "1/2 类止损用 pivot 非 center ⟹ center=None");
        // 互斥语义：B2 端点不置 1/3 类 bit。
        assert!(!b2.bits.buy1 && !b2.bits.buy3, "第二类端点不置 1/3 类 bit");
    }

    /// ★still-MISSING-窗口边界（codex 异质裁决坐实，编码为可执行断言，formalization-validity-domain）：
    /// L0→L1 的三段方向交替窗口**结构上不产 B2/S2**——三段交替里可背驰的同向段只在位置 2（末段，
    /// 无后继回拉），位置 0 无前同向对照，位置 1 是唯一异向（无前同向）。故单个 L1 走势的 3 段 L0
    /// subs 内识别不出「第一类离开（破中枢∧背驰）+ 后继回拉」。
    ///
    /// 此断言锁定边界：L0 级别（索引 0）的 bsp **不含 B2/S2**（B2/S2 由 L1→L2 几何路径产，见
    /// `end_to_end_second_buy_via_l1_l2_geometric`）。这是固定三段封装的结构上界，非补丁——放松
    /// 背驰约束或窗口大小来强产 L0 层 B2 = 声明膨胀（no-patch 禁止）。
    #[test]
    fn l0_level_emits_no_second_class_window_bound() {
        let cfg = ThetaConfig::default();
        // 简单 up-down-up 三段（一个 L0 中枢，一个 L1 走势）——L1 走势 3 段 subs 内无法产 B2/S2。
        let segments = vec![
            seg(Direction::Up,   0,  4, 100, 200),
            seg(Direction::Down, 4,  8, 200, 100),
            seg(Direction::Up,   8, 12, 100, 200),
        ];
        let closes: Vec<i64> = (0..16).map(|i| 100 + (i % 4) * 10).collect();
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
        let out = classify(&layer, &cfg);
        // L0 级别 bsp 不含第二类（三段交替窗口结构上界）。
        for p in &out.levels[0].bsp {
            assert!(!p.bits.buy2, "L0→L1 三段交替窗口不产 B2（still-MISSING-窗口，codex 裁决）");
            assert!(!p.bits.sell2, "L0→L1 三段交替窗口不产 S2（still-MISSING-窗口，codex 裁决）");
        }
    }

    #[test]
    fn classify_empty_layer_yields_empty() {
        let cfg = ThetaConfig::default();
        let out = classify(&ParseLayer::default(), &cfg);
        assert_eq!(out, Classification::default());
    }

    #[test]
    fn fewer_than_min_parts_natural_termination() {
        // L0 线段数 < min_parts_per_level(3) ⟹ 无 L0 走势，levels 空（自然终止）。
        let cfg = ThetaConfig::default();
        let layer = ParseLayer {
            segments: Rc::new(vec![seg(Direction::Up, 0, 4, 0, 10), seg(Direction::Down, 4, 8, 10, 5)]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        assert!(out.levels.is_empty(), "2 段 < min_parts 3 ⟹ 自然终止");
    }

    #[test]
    fn three_overlapping_segments_form_center() {
        // 完整判据（Origin.CenterComplete，口径 B 637号）：方向交替 上-下-上 + 全三段核心非空。
        // 段区间 [0,10]up,[3,12]down,[5,15]up：核心取**全三段** zd=max(0,3,5)=5, zg=min(10,12,15)=10。
        // 第三段 [5,15] 收窄核心下沿（A 口径 zd=3 → B zd=5）⟹ 真中枢成立。
        let cfg = ThetaConfig::default();
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 0, 10),
                seg(Direction::Down, 4, 8, 12, 3),
                seg(Direction::Up, 8, 12, 5, 15),
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        assert!(!out.levels.is_empty());
        let l0 = &out.levels[0];
        assert_eq!(l0.centers.len(), 1, "三段方向交替+贯穿 ⟹ 一个真中枢");
        // 核心取全三段（口径 B，637号 computeZD/computeZG s1 s2 s3）——第三段收窄核心下沿至 5。
        assert_eq!((l0.centers[0].zd, l0.centers[0].zg), (5, 10));
        // 一个中枢 ⟹ classifyMove = consolidation ⟹ moves=[Consolidation]。
        assert_eq!(l0.moves, vec![MoveKind::Consolidation]);
    }

    #[test]
    fn same_direction_three_segments_rejected_by_complete() {
        // G4 完整判据反退化：三段同向（全 up）+ 前两段核心非空，但无方向交替 ⟹ L0 不识别中枢
        // （旧几何窗口会误判，完整判据正确拒绝，对齐 Origin.CenterComplete.sameDir_not_centerConfirmed）。
        let cfg = ThetaConfig::default();
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 10, 20),
                seg(Direction::Up, 4, 8, 18, 25),
                seg(Direction::Up, 8, 12, 22, 30),
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        // 无方向交替 ⟹ L0 无中枢（完整判据拒绝单边三段）。
        assert_eq!(out.levels.len(), 1);
        assert!(out.levels[0].centers.is_empty(), "同向三段无方向交替 ⟹ 完整判据拒绝（非中枢）");
    }

    #[test]
    fn lmax_bound_respected() {
        // 构造大量重叠段，验证递归不超过 l_max+1 级（每级至少消耗中枢，最终自然终止）。
        let cfg = ThetaConfig::default();
        let mut segments = Vec::new();
        // 27 段全重叠区间 [0,100]（每三段成一中枢，逐级递归）。
        for i in 0..27 {
            let dir = if i % 2 == 0 { Direction::Up } else { Direction::Down };
            segments.push(seg(dir, i * 4, i * 4 + 4, 0, 100));
        }
        let layer = ParseLayer { segments: Rc::new(segments), ..Default::default() };
        let out = classify(&layer, &cfg);
        // 级别数不超过 l_max+1（reference:30 上界，default l_max=6）。
        assert!(out.levels.len() <= cfg.level.l_max as usize + 1);
    }

    #[test]
    fn no_center_terminates_recursion() {
        // 三段无公共重叠 ⟹ L0 无中枢 ⟹ moves 为空（HigherCenterCandidate→None）+ 递归终止。
        let cfg = ThetaConfig::default();
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 0, 4),
                seg(Direction::Down, 4, 8, 14, 10),
                seg(Direction::Up, 8, 12, 20, 24),
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        // L0 无中枢 ⟹ 该级 moves 空（裁决退化），递归在该级后终止（无上级单元）。
        assert_eq!(out.levels.len(), 1);
        assert!(out.levels[0].centers.is_empty());
        assert!(out.levels[0].moves.is_empty());
    }

    /// ★端到端 fixture（Lead 验证门）：≥3 重叠线段 → 非空中枢 + 至少一个买卖点。
    /// 解阻塞点 A：classify 在真实结构输入上返回非空 Classification（n_orders>0 的前提）。
    #[test]
    fn end_to_end_third_buy_signal() {
        let cfg = ThetaConfig::default();
        // 段0-2：三段在 [100,200] 重叠 ⟹ 中枢 zd=100,zg=200,end_index=12。
        // 段3：向上离开（端点 250 > zg=200）。段4：向下回试低点 210 >= zg=200 ⟹ 3 买。
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 100, 200),
                seg(Direction::Down, 4, 8, 200, 100),
                seg(Direction::Up, 8, 12, 100, 200),
                seg(Direction::Up, 12, 16, 150, 250),   // 离开中枢上方
                seg(Direction::Down, 16, 20, 250, 210), // 回试低点 >= zg → 3 买
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);

        // 1) 非空 Classification + L0 有中枢。
        assert!(!out.levels.is_empty(), "解阻塞 A：classify 返回非空");
        let l0 = &out.levels[0];
        assert_eq!(l0.centers.len(), 1, "三段重叠 ⟹ 一个中枢");
        assert_eq!((l0.centers[0].zd, l0.centers[0].zg), (100, 200));

        // 2) 至少一个买卖点（第三类买点），且携带结构止损价 single source。
        assert!(!l0.bsp.is_empty(), "解阻塞 A：L0 至少一个买卖点");
        let third_buys: Vec<_> = l0.bsp.iter().filter(|p| p.bits.buy3).collect();
        assert_eq!(third_buys.len(), 1, "一个第三类买点");
        let p = third_buys[0];
        assert_eq!(p.source_index, 20, "买卖点定位回试端点");
        assert_eq!(p.pivot_low, 210, "结构止损价 pivot_low single source");
        assert_eq!(p.center.map(|c| c.zg), Some(200), "3 买止损 = ZG single source");
    }

    /// 端到端边界：有中枢但无离开/回试 ⟹ 中枢非空、bsp 空（无买卖点是诚实产出，非错误）。
    #[test]
    fn end_to_end_center_without_signal() {
        let cfg = ThetaConfig::default();
        // 三段重叠成中枢，但无后续离开线段 ⟹ 无第三类买卖点。
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 100, 200),
                seg(Direction::Down, 4, 8, 200, 100),
                seg(Direction::Up, 8, 12, 100, 200),
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        assert_eq!(out.levels[0].centers.len(), 1);
        assert!(out.levels[0].bsp.is_empty(), "无离开/回试 ⟹ 无买卖点（诚实空）");
    }

    /// ★classify_with_tower (i) 段导出桥——tower 非空 + depth≥1 真嵌套存在。
    ///
    /// 9 段 L0 → 3 个 L1 走势 → L2 中枢（几何路径）：tower[1] 含 sub_moves 非空的
    /// LeveledMove（RMove::Compose，depth=1 真嵌套）。坐实：导出桥正确产出真嵌套塔。
    #[test]
    fn classify_with_tower_depth_ge1_true_nesting() {
        let cfg = ThetaConfig::default();
        // 9 段：三组 up-down-up（每组 → 一个 L1 走势），三个 L1 走势外缘重叠成 L2 中枢。
        let segments = vec![
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            seg(Direction::Up,  12, 16, 130, 145),
            seg(Direction::Down,16, 20, 145,  80),
            seg(Direction::Up,  20, 24,  80, 144),
            seg(Direction::Up,  24, 28, 120, 148),
            seg(Direction::Down,28, 32, 148, 115),
            seg(Direction::Up,  32, 36, 115, 147),
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
        for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
        for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
        let (_, tower) = classify_with_tower(&layer, &cfg);
        assert!(!tower.is_empty(), "tower 非空（至少 L0 级被处理）");
        assert!(tower.len() >= 2, "9 段 L0 → 3 个 L1 走势 → L2 中枢 ⟹ tower 至少 2 层");
        // depth≥1 真嵌套：tower[1] 含 sub_moves 非空的 LeveledMove（RMove::Compose，L1 输入塔）。
        let has_true_nesting = tower[1].iter().any(|m| !m.sub_moves.is_empty());
        assert!(has_true_nesting, "tower[1] 含真嵌套 LeveledMove（sub_moves 非空，depth≥1）");
    }

    /// ★classify_with_tower Classification 与 classify 同输入 bit-identical（导出不改原分类）。
    #[test]
    fn classify_with_tower_classification_equals_classify() {
        let cfg = ThetaConfig::default();
        let segments = vec![
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            seg(Direction::Up,  12, 16, 130, 145),
            seg(Direction::Down,16, 20, 145,  80),
            seg(Direction::Up,  20, 24,  80, 144),
            seg(Direction::Up,  24, 28, 120, 148),
            seg(Direction::Down,28, 32, 148, 115),
            seg(Direction::Up,  32, 36, 115, 147),
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
        for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
        for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
        let expected = classify(&layer, &cfg);
        let (actual, _) = classify_with_tower(&layer, &cfg);
        assert_eq!(actual, expected, "classify_with_tower Classification 与 classify bit-identical（原分类不变）");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  增量塔 API bit-exact（task #93：incremental == 全量，逐 bar 断言）
    // ──────────────────────────────────────────────────────────────────────

    /// 构造逐段追加的合成段序列（方向交替 + 价格震荡，产足够中枢触发多级塔）。
    fn synthetic_segments(count: usize) -> Vec<Segment> {
        (0..count)
            .map(|i| {
                let dir = if i % 2 == 0 { Direction::Up } else { Direction::Down };
                let base = 100i64 + (i as i64) * 3;
                let swing = if i % 2 == 0 { 50 } else { -50 };
                let sp = base;
                let ep = base + swing;
                seg(dir, i * 4, i * 4 + 4, sp, ep)
            })
            .collect()
    }

    /// ★增量塔单次 bit-exact：对完整段序列，`classify_with_tower_incremental(.., fresh cache)`
    /// 输出 == `classify_with_tower`（Classification + tower 逐字段相等）。
    ///
    /// fresh cache（空）从 consumed=0 续扫 == 全量扫描。验证增量入口的基础正确性。
    #[test]
    fn incremental_tower_fresh_cache_equals_full() {
        let cfg = ThetaConfig::default();
        for n in [3usize, 6, 9, 12, 18] {
            let segments = synthetic_segments(n);
            let closes: Vec<i64> = (0..(n * 4 + 8) as i64).map(|i| 100 + (i % 5) * 5).collect();
            let layer =
                ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };

            let (full_cls, full_tower) = classify_with_tower(&layer, &cfg);
            let mut cache = TowerCache::new();
            let (inc_cls, inc_tower) = classify_with_tower_incremental(&layer, &cfg, &mut cache);

            assert_eq!(inc_cls, full_cls, "n={n}: 增量 Classification == 全量");
            assert_eq!(inc_tower.len(), full_tower.len(), "n={n}: 增量 tower 层数 == 全量");
            for (lvl, (il, fl)) in inc_tower.iter().zip(full_tower.iter()).enumerate() {
                assert_eq!(il, fl, "n={n} level {lvl}: 增量 tower 级 LeveledMove 序列 == 全量");
            }
        }
    }

    /// ★增量塔逐段追加 bit-exact（#93 核心铁律）：模拟 per-bar substrate 逐段追加，
    /// 每步断言 `classify_with_tower_incremental(layer[..=i], cache)` ==
    /// `classify_with_tower(layer[..=i])`（Classification + tower 逐字段相等）。
    ///
    /// 这是增量塔的真实使用场景——段账本单调增长，cache 跨步复用前级 confirmed 前缀。
    /// 任何 resume bit-exact 破裂、跨级传播错误、裁决漂移都会在此捕获。
    #[test]
    fn incremental_tower_per_segment_append_matches_full() {
        let cfg = ThetaConfig::default();
        let all_segments = synthetic_segments(21);
        let closes: Vec<i64> = (0..100).map(|i| 100 + (i % 7) * 4).collect();

        let mut cache = TowerCache::new();
        for n in 1..=all_segments.len() {
            let segments = all_segments[..n].to_vec();
            let layer = ParseLayer {
                segments: Rc::new(segments),
                merged_bars: Rc::new(bars_from_closes(&closes)),
                ..Default::default()
            };

            // 全量基准。
            let (full_cls, full_tower) = classify_with_tower(&layer, &cfg);
            // 增量（cache 跨步复用）。
            let (inc_cls, inc_tower) = classify_with_tower_incremental(&layer, &cfg, &mut cache);

            assert_eq!(inc_cls, full_cls, "n={n}: 增量 Classification != 全量（bit-exact 破裂）");
            assert_eq!(
                inc_tower.len(),
                full_tower.len(),
                "n={n}: 增量 tower 层数 != 全量"
            );
            for (lvl, (il, fl)) in inc_tower.iter().zip(full_tower.iter()).enumerate() {
                assert_eq!(
                    il, fl,
                    "n={n} level {lvl}: 增量 tower 级 LeveledMove != 全量（真 subs 嵌套破裂）"
                );
            }
        }
    }



    /// ★段账本回缩退化 bit-exact：模拟 parser 回撤最后一段（非单调追加），
    /// `cache` 自动检测回缩 ⟹ 清空 + 全量重扫 ⟹ 仍 bit-exact（退化不破坏正确性）。
    #[test]
    fn incremental_tower_shrink_falls_back_to_full() {
        let cfg = ThetaConfig::default();
        let all_segments = synthetic_segments(12);
        let closes: Vec<i64> = (0..80).map(|i| 100 + (i % 6) * 4).collect();

        let mut cache = TowerCache::new();
        // 先追加到 12 段。
        let layer_full =
            ParseLayer { segments: Rc::new(all_segments.clone()), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
        let _ = classify_with_tower_incremental(&layer_full, &cfg, &mut cache);
        // 回缩到 8 段（parser 回撤）。
        let layer_shrink = ParseLayer {
            segments: Rc::new(all_segments[..8].to_vec()),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let (full_cls, full_tower) = classify_with_tower(&layer_shrink, &cfg);
        let (inc_cls, inc_tower) = classify_with_tower_incremental(&layer_shrink, &cfg, &mut cache);
        assert_eq!(inc_cls, full_cls, "回缩退化：增量 Classification == 全量");
        assert_eq!(inc_tower, full_tower, "回缩退化：增量 tower == 全量");
    }

    /// ★B2 真产出 bit-exact：增量塔在产 B2 的真实结构（9 段三组 up-down-up）下，
    /// `classify_with_tower_incremental` 产出的 B2 与全量 `classify_with_tower` bit-identical——
    /// 验证增量 compose 的真 Fugue 547（subs 真 Compose，B2 真可产，禁级别差伪造）。
    #[test]
    fn incremental_tower_preserves_b2_second_buy() {
        let cfg = ThetaConfig::default();
        let segments = vec![
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            seg(Direction::Up,  12, 16, 130, 145),
            seg(Direction::Down,16, 20, 145,  80),
            seg(Direction::Up,  20, 24,  80, 144),
            seg(Direction::Up,  24, 28, 120, 148),
            seg(Direction::Down,28, 32, 148, 115),
            seg(Direction::Up,  32, 36, 115, 147),
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
        for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
        for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };

        let (full_cls, _) = classify_with_tower(&layer, &cfg);
        let mut cache = TowerCache::new();
        let (inc_cls, _) = classify_with_tower_incremental(&layer, &cfg, &mut cache);

        // 全量产 1 个 B2（见 end_to_end_second_buy_via_l1_l2_geometric），增量须 bit-identical。
        let full_b2: Vec<_> = full_cls.levels[1].bsp.iter().filter(|p| p.bits.buy2).collect();
        let inc_b2: Vec<_> = inc_cls.levels[1].bsp.iter().filter(|p| p.bits.buy2).collect();
        assert_eq!(inc_b2.len(), full_b2.len(), "增量塔 B2 数量 == 全量（真 Fugue 547 保留）");
        assert_eq!(inc_b2.len(), 1, "增量塔仍真产 B2（subs 真 Compose，非级别差伪造）");
        assert_eq!(inc_b2[0].source_index, full_b2[0].source_index, "B2 source_index bit-exact");
        assert_eq!(inc_cls, full_cls, "增量塔完整 Classification == 全量（含 B2 BSP）");
    }

    /// ★codex 反例（cascade reset 完备性，L1 构造）：L0 frontier 末段**内点改写**——
    /// 上级投影 `UnitRange(lo,hi)` bit-identical 但底层 `sub_moves` 变。
    ///
    /// 场景：9 段三组 up-down-up，末段 seg[8] `Up[32,36]` end_price 从 147 改写为 140。
    /// 140 是组 C 外缘内点（组 C max-hi=148(seg[6]) / min-lo=115(seg[7]) 不变）⟹ L0 该窗口
    /// 中枢 gg/dd 不变 ⟹ L1 输入投影 `project_to_units` bit-identical。但 seg[8] 的 lo/hi 从
    /// [115,147] 变 [115,140] ⟹ L0 upper_moves[2].sub_moves[2] 深嵌套坐标变。
    ///
    /// 旧守卫（仅比对本级 `project_to_units` 投影）：L0 reset 正确，但 L1 frontier_mutated=false
    /// 漏 reset ⟹ `cache.levels[1].upper_moves` 深嵌套 sub_moves 陈旧（仍 [115,147]）+ BSP memo
    /// （key 仅三长度）复用陈旧 BSP ⟹ 与全量发散。
    /// cascade reset 修复：L0 变异 → 强制 reset L1+（无条件跟随下级），深嵌套 sub_moves 重建为 [115,140]。
    ///
    /// **L1**（合成构造，验证管线完备性，非真实数据假设——formalization-validity-domain 231号）。
    #[test]
    fn cascade_reset_on_frontier_interior_rewrite() {
        let cfg = ThetaConfig::default();
        let base = vec![
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            seg(Direction::Up,  12, 16, 130, 145),
            seg(Direction::Down,16, 20, 145,  80),
            seg(Direction::Up,  20, 24,  80, 144),
            seg(Direction::Up,  24, 28, 120, 148),
            seg(Direction::Down,28, 32, 148, 115),
            seg(Direction::Up,  32, 36, 115, 147), // v1 末段
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
        for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
        for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
        let merged = Rc::new(bars_from_closes(&closes));

        // v1: 末段 end_price=147。
        let layer_v1 = ParseLayer { segments: Rc::new(base.clone()), merged_bars: merged.clone(), ..Default::default() };
        // v2: 仅末段 end_price 改写 147→140（组 C 外缘内点，L1 投影不变，L0 sub_moves 变）。
        let mut v2_segs = base.clone();
        v2_segs[8].end_price = 140;
        let layer_v2 = ParseLayer { segments: Rc::new(v2_segs), merged_bars: merged.clone(), ..Default::default() };

        // 前提自检（codex 反例成立的必要条件）：L1 投影输入 v1==v2 bit-identical（守卫看不到变异），
        // 但 L0 末段 sub_moves 已变（147→140）。若此前提不成立，本测试不构成反例。
        let mk_l1_units = |segs: &Rc<Vec<Segment>>| {
            let l0_units: Vec<UnitRange> = segs.iter().map(segment_to_unit).collect();
            let moves_l0: Vec<LeveledMove> = l0_units.iter().enumerate()
                .map(|(i,u)| LeveledMove::from_unit(u, recursive_tower::ElementId{level:0,ordinal:i as u64})).collect();
            let (_c, upper) = recursive_tower::compose_level(&l0_units, &moves_l0, true, 1);
            recursive_tower::project_to_units(&upper)
        };
        assert_eq!(mk_l1_units(&layer_v1.segments), mk_l1_units(&layer_v2.segments),
            "前提：L1 投影输入 v1==v2（守卫的本级投影比对看不到此变异）");

        // 共享 cache：先喂 v1（缓存 L0/L1），再喂 v2（frontier 内点改写）——模拟 per-bar 末段重划。
        let mut cache = TowerCache::new();
        let _ = classify_with_tower_incremental(&layer_v1, &cfg, &mut cache);
        let (inc_v2, inc_tower_v2) = classify_with_tower_incremental(&layer_v2, &cfg, &mut cache);
        let (full_v2, full_tower_v2) = classify_with_tower(&layer_v2, &cfg);

        // ★核心断言（latent 陈旧检测，非仅返回值）：cache 内 L1 深嵌套 sub_moves 末段坐标必须 == v2
        // 的 [115,140]。返回的 Classification/tower 不消费 cache 内 L1 upper_moves 的深 subs（tower 用
        // 新鲜 moves_tower 快照），故陈旧在返回值里 latent——但它喂 BSP（extract_second_for_level）+
        // 下一 bar 的 L2 投影。直接断言 cache 深 subs，捕获 latent 陈旧（640：不靠返回值碰巧相等）。
        let l0_seg8_full = classify_with_tower(&layer_v2, &cfg).1[0].last().unwrap().rmove.clone();
        assert_eq!(l0_seg8_full, descend::RMove::Segment { direction: Direction::Up, lo: 115, hi: 140 },
            "前提：v2 全量 L0 末段 == [115,140]");
        // cache.L1.upper_moves[0].sub_moves[2](groupC).sub_moves[2](seg[8]) 应 == [115,140]。
        let l1_deep = &cache.levels[1].upper_moves[0].sub_moves[2].sub_moves[2].rmove;
        assert_eq!(*l1_deep, descend::RMove::Segment { direction: Direction::Up, lo: 115, hi: 140 },
            "cascade: cache L1 深嵌套 seg[8] == v2 [115,140]（陈旧则 [115,147]——L1 漏 cascade reset）");

        // 返回值也须 bit-exact（cascade 后 L1 重建，tower/Classification 全对齐）。
        assert_eq!(inc_v2, full_v2, "cascade: v2 增量 Classification == 全量");
        assert_eq!(inc_tower_v2.len(), full_tower_v2.len(), "cascade: tower 层数 == 全量");
        for (lvl, (il, fl)) in inc_tower_v2.iter().zip(full_tower_v2.iter()).enumerate() {
            assert_eq!(il, fl, "cascade: level {lvl} LeveledMove == 全量");
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    //  标度验证（task #93：per-bar 累积成本，增量 vs 全量）
    // ──────────────────────────────────────────────────────────────────────

    /// ★合成标度：per-bar 段追加累积成本，增量 exp 显著 < 全量 exp。
    ///
    /// 全量 `classify_with_tower` 每步从 0 重扫塔 ⟹ 累积 O(Σ i) ≈ O(N²)，exp≈2。
    /// 增量 `classify_with_tower_incremental` 每步续扫 tail ⟹ 累积 O(Σ tail) ≈ O(N)，exp≈1。
    /// 合成段序列单调追加（增量有效域）；此测试 always-run（无需真实数据）。
    #[test]
    fn incremental_tower_scaling_dominates_full_synthetic() {
        let cfg = ThetaConfig::default();
        let sizes = [100usize, 200, 400];
        let mut full_times = Vec::new();
        let mut inc_times = Vec::new();

        for &n in &sizes {
            let all_segments = synthetic_segments(n);
            let closes: Vec<i64> = (0..(n * 4 + 8) as i64).map(|i| 100 + (i % 7) * 4).collect();

            // 全量 per-bar 累积。
            let t0 = std::time::Instant::now();
            for k in 1..=n {
                let layer = ParseLayer {
                    segments: Rc::new(all_segments[..k].to_vec()),
                    merged_bars: Rc::new(bars_from_closes(&closes)),
                    ..Default::default()
                };
                let _ = classify_with_tower(&layer, &cfg);
            }
            full_times.push(t0.elapsed().as_secs_f64());

            // 增量 per-bar 累积（cache 跨步复用）。
            let t0 = std::time::Instant::now();
            let mut cache = TowerCache::new();
            for k in 1..=n {
                let layer = ParseLayer {
                    segments: Rc::new(all_segments[..k].to_vec()),
                    merged_bars: Rc::new(bars_from_closes(&closes)),
                    ..Default::default()
                };
                let _ = classify_with_tower_incremental(&layer, &cfg, &mut cache);
            }
            inc_times.push(t0.elapsed().as_secs_f64());
        }

        // exp 估计（log-log 斜率，sizes 翻倍）。
        let full_exp = (full_times[2] / full_times[0]).ln() / (sizes[2] as f64 / sizes[0] as f64).ln();
        let inc_exp = (inc_times[2] / inc_times[0]).ln() / (sizes[2] as f64 / sizes[0] as f64).ln();

        eprintln!(
            "\n===== 增量塔标度（合成 per-bar 累积）=====\n  \
             sizes={sizes:?}\n  full_times={full_times:?} (exp≈{full_exp:.2})\n  \
             inc_times={inc_times:?} (exp≈{inc_exp:.2})\n  \
             增量/全量比 @n={}: {:.2}x（越小增量越优）",
            sizes[2],
            inc_times[2] / full_times[2].max(1e-12)
        );

        // 增量须显著快于全量（MACD 增量 + 塔构造增量 + classify_move 增量 综合加速）。
        // ★判据：最大规模下增量/全量时间比 < 0.5（即增量至少 2x 加速）。
        // exp 差距在小规模 debug 噪声大（两者均 O(n²) 受限于 LevelState/tower_snapshots clone
        // 的 API 所需 O(k)/iter），但增量消除 MACD 全量重算 + 塔构造全量扫描 ⟹ 常数因子显著优。
        // 实测增量/全量比 @n=400 ≈ 0.15-0.25（4-7x 加速），断言 < 0.5 为稳健下界。
        let ratio_at_max = inc_times[2] / full_times[2].max(1e-12);
        assert!(
            ratio_at_max < 0.5,
            "增量/全量比 @n={} = {ratio_at_max:.3} 须 < 0.5（增量至少 2x 加速；MACD+塔+classify_move 增量）\n\
             full_exp≈{full_exp:.2}, inc_exp≈{inc_exp:.2}",
            sizes[2]
        );
    }
}

#[cfg(test)]
mod incremental_profile {
    //! 增量塔真实数据标度 profile（#[ignore]，需真实数据 + release）。
    use super::*;
    use super::super::backtest::data;
    use super::super::parser;

    /// ★真实数据 per-bar 标度：CL 真实段账本逐段追加，增量 vs 全量累积成本 + exp。
    ///
    /// 真实段账本单调追加（parser 前缀稳定语义）⟹ 增量有效域命中。验证真实数据下增量 exp≈1
    /// 而全量 exp≈2（塔构造超线性消除）。L2 经验标度（formalization-validity-domain 231号）。
    #[test]
    #[ignore = "真实数据标度 profile：需 CL 数据；--release（per-bar 双跑对照）"]
    fn profile_incremental_tower_real_scaling() {
        let cfg = ThetaConfig::default();
        let ds = match data::load_by_symbol("CL", &cfg) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("DATA BLOCKER: {e}");
                panic!("需真实数据");
            }
        };
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        // ponytail: 大规模标度验证 acceptance[4]——全引擎 per-bar exp≈1.0 @16K。
        // a7dfec46: n<5000 不可靠；此处用 [5K, 10K, 16K] 真实大规模。
        let sizes = [5_000usize, 10_000, 16_000];
        let mut full_times = Vec::new();
        let mut inc_times = Vec::new();
        let mut used_sizes = Vec::new();

        for &n in &sizes {
            if n > oos.bars.len() {
                eprintln!("DATA LIMIT: n={n} > oos.bars.len()={}，跳过", oos.bars.len());
                break;
            }
            let bars = &oos.bars[..n];

            // 全量 per-bar 累积。
            let t0 = std::time::Instant::now();
            for i in 50..n {
                let l0 = parser::parse_layer(&bars[..i], &cfg);
                let _ = classify_with_tower(&l0, &cfg);
            }
            full_times.push(t0.elapsed().as_secs_f64());

            // 增量 per-bar 累积。
            let t0 = std::time::Instant::now();
            let mut cache = TowerCache::new();
            for i in 50..n {
                let l0 = parser::parse_layer(&bars[..i], &cfg);
                let _ = classify_with_tower_incremental(&l0, &cfg, &mut cache);
            }
            inc_times.push(t0.elapsed().as_secs_f64());
            used_sizes.push(n);
            eprintln!("n={n} done: full={:.2}s inc={:.2}s", *full_times.last().unwrap(), *inc_times.last().unwrap());
        }

        // 逐相邻对算 exp（log-log 斜率），大规模验证 acceptance[4]。
        eprintln!("\n===== 增量塔真实标度（CL per-bar 累积，大规模）=====");
        for w in used_sizes.windows(2) {
            let (n0, n1) = (w[0], w[1]);
            let i0 = used_sizes.iter().position(|&s| s == n0).unwrap();
            let i1 = i0 + 1;
            let full_exp = (full_times[i1] / full_times[i0].max(1e-12)).ln()
                / (n1 as f64 / n0 as f64).ln();
            let inc_exp = (inc_times[i1] / inc_times[i0].max(1e-12)).ln()
                / (n1 as f64 / n0 as f64).ln();
            eprintln!(
                "  [{n0}→{n1}] full exp≈{full_exp:.2}  inc exp≈{inc_exp:.2}  \
                 增量/全量比 @{n1}: {:.2}x",
                inc_times[i1] / full_times[i1].max(1e-12)
            );
        }
    }
}
