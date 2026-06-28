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

use bsp::BspPoint;
use center::UnitRange;
use level::{classify_move, outcome_to_kind, MoveOutcome};
use recursive_tower::{
    compose_level, descend_leveled, index_of_in, map_src_to_close_idx, project_to_units, LeveledMove,
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
    let min_parts = config.level.min_parts_per_level as usize;
    let l_max = config.level.l_max as usize;

    // L0 输入单元 = parser 线段账本（reference:29 L0=1分钟线段账本）。
    let mut units: Vec<UnitRange> = l0.segments.iter().map(segment_to_unit).collect();

    // 空 L0：无可构造级别（自然终止于 L0 之前）。
    if units.is_empty() {
        return Classification::default();
    }

    // ★递归塔对象（#53 升级，still-MISSING-塔解除）：L0 走势单元 = 携坐标的 `RMove::Segment`
    // （`LeveledMove`，递归底 level 0）。旧塔把每级走势单元折叠为无 subs 的 `UnitRange`，
    // `extract_second_signals`（消费 `RMove::Compose` 的 descend 取回次级别走势）永产不出 B2/S2。
    // 新塔每级走势单元携次级别走势 subs（`RMove::Compose`）+ source_index 坐标 ⟹ B2/S2 真可产。
    let mut moves_tower: Vec<LeveledMove> = units.iter().map(LeveledMove::from_unit).collect();

    // 第一类背驰 MACD：closes/close_src 在全递归层共享（L0 唯一可达 close 序列；上级走势的
    // 次级别 close 区间由 source_index 坐标定位，见 macd 接入点）。
    let closes: Vec<f64> = l0.merged_bars.iter().map(|b| b.close as f64).collect();
    let close_src: Vec<usize> = l0.merged_bars.iter().map(|b| b.source_index).collect();
    let hist = divergence::compute_macd(&closes, &config.macd).hist;

    let mut levels: Vec<LevelState> = Vec::new();

    // 递归级别构造：每级由下级走势单元构造（L0 直接是线段单元，从 L0 开始裁决）。
    for level_idx in 0..=l_max {
        // 自然终止（reference:30）：某层无 ≥min_parts 完成部件 ⟹ 无法产生完整走势，停止。
        if units.len() < min_parts {
            break;
        }

        // L0（level_idx==0）用完整判据（方向交替，线段有方向）；上级用几何路径（外缘，单元无方向）。
        let is_l0 = level_idx == 0;
        let (centers, outcome) = classify_level(&units, is_l0);

        // 走势裁决 → MoveKind（HigherCenterCandidate 退化态映 None，不入 moves）。
        let moves: Vec<MoveKind> = outcome_to_kind(outcome).into_iter().collect();

        // L(k+1) 走势塔 = 本级窗口化 compose（每中枢的构成三段次级别走势 → 一个上级 `RMove::Compose`，
        // 契约锚 `Origin.RecursiveLevelSystem.composeStep` 窗口封装）。`upper_moves` 是携坐标的上级走势
        // 序列（descend 取回构成它的次级别走势 ⟹ B2/S2 可产），与 `centers` 一一对应。
        let (centers_w, upper_moves) = compose_level(&units, &moves_tower, is_l0, level_idx as u32 + 1);
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
        moves_tower = upper_moves;

        // 本级无中枢 ⟹ 无上级输入单元，停止递归（自然终止）。
        if units.is_empty() {
            break;
        }
    }

    Classification { levels }
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
                |m| sublevel_diverges(m, &subs, hist, close_src),
                // 坐标：从侧车按结构身份查回次级别走势的原始 K 序（end_index）。
                |m| index_of_in(&subs, m),
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
        let layer = ParseLayer { segments, merged_bars: bars_from_closes(&closes), ..Default::default() };
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
        let layer = ParseLayer { segments, merged_bars: bars_from_closes(&closes), ..Default::default() };
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
            segments: vec![seg(Direction::Up, 0, 4, 0, 10), seg(Direction::Down, 4, 8, 10, 5)],
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
            segments: vec![
                seg(Direction::Up, 0, 4, 0, 10),
                seg(Direction::Down, 4, 8, 12, 3),
                seg(Direction::Up, 8, 12, 5, 15),
            ],
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
            segments: vec![
                seg(Direction::Up, 0, 4, 10, 20),
                seg(Direction::Up, 4, 8, 18, 25),
                seg(Direction::Up, 8, 12, 22, 30),
            ],
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
        let layer = ParseLayer { segments, ..Default::default() };
        let out = classify(&layer, &cfg);
        // 级别数不超过 l_max+1（reference:30 上界，default l_max=6）。
        assert!(out.levels.len() <= cfg.level.l_max as usize + 1);
    }

    #[test]
    fn no_center_terminates_recursion() {
        // 三段无公共重叠 ⟹ L0 无中枢 ⟹ moves 为空（HigherCenterCandidate→None）+ 递归终止。
        let cfg = ThetaConfig::default();
        let layer = ParseLayer {
            segments: vec![
                seg(Direction::Up, 0, 4, 0, 4),
                seg(Direction::Down, 4, 8, 14, 10),
                seg(Direction::Up, 8, 12, 20, 24),
            ],
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
            segments: vec![
                seg(Direction::Up, 0, 4, 100, 200),
                seg(Direction::Down, 4, 8, 200, 100),
                seg(Direction::Up, 8, 12, 100, 200),
                seg(Direction::Up, 12, 16, 150, 250),   // 离开中枢上方
                seg(Direction::Down, 16, 20, 250, 210), // 回试低点 >= zg → 3 买
            ],
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
            segments: vec![
                seg(Direction::Up, 0, 4, 100, 200),
                seg(Direction::Down, 4, 8, 200, 100),
                seg(Direction::Up, 8, 12, 100, 200),
            ],
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        assert_eq!(out.levels[0].centers.len(), 1);
        assert!(out.levels[0].bsp.is_empty(), "无离开/回试 ⟹ 无买卖点（诚实空）");
    }
}
