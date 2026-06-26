//! Θ_level + Θ_signal 子模块（reference-theta-v0.md:27-37）。
//!
//! ## bit-exact 对齐 `Strict/LevelState.lean` / `Center.lean` / `BSP.lean` /
//! `Trend.lean` / `Nest.lean` / `Recursive.lean`
//!
//! 递归级别构造 + R6 态 + 买卖点 bit-vector + 背驰度量 + 区间套。
//! 给定 Θ_level/Θ_signal ⟹ R6 态 + BSP 证书 (D_ℓ,R_ℓ,E_ℓ) 唯一（LevelState 元定理）。
//!
//! ## 子模块拓扑
//!
//! - [`center`]：中枢边界构造（连续三段窗口）+ 中枢关系三态 + 点位三态。对齐
//!   `RecursiveConstruction.CenterDerivedAt` / `CenterTrichotomy` / `Claim9`。
//! - [`level`]：递归级别走势裁决（`classifyMove` 全链同向）。对齐 `RecursiveConstruction`。
//! - [`level_state`]：R6 位置态 + LevelState 三元组。对齐 `Strict/LevelState.lean`。
//! - [`bsp`]：买卖点 bit-vector 判据（三类结构谓词，非互斥）。对齐 `BSPLabels`/`Strict/BSP`。
//! - [`divergence`]：背驰 MACD 度量（浮点域隔离 + 同向段面积严格变小）。对齐 reference:37。
//! - [`nest`]：区间套有限递归证书 χ（`Sel_Θ` 选择器 + 终端确认）。对齐 `Strict/Nest.lean`。
//!
//! ## 递归级别（reference-theta-v0.md:29-30）
//!
//! `L0=1分钟线段账本`（parser segments）；`L(k+1)` 只由 `Lk` 已完成走势/Move 构造
//! （Lk 走势单元 → 连续三段窗口中枢 → 中枢序列裁决走势 → L(k+1) 输入单元）。禁跳级混级。
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
use super::types::{Center, MoveKind, Segment};

pub mod center;
pub mod level;
pub mod level_state;
pub mod bsp;
pub mod divergence;
pub mod nest;
pub mod signal;

use bsp::BspPoint;
use center::UnitRange;
use level::{classify_move, outcome_to_kind, MoveOutcome};

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

/// 走势单元的价格区间投影（线段 / 上级走势的 `[lo,hi]`，递归级别构造的输入单元）。
///
/// L0 单元 = parser 线段；L(k+1) 单元 = Lk 已完成走势（其区间由中枢外缘合成，对齐
/// `RecursiveConstruction.Move.interval`：compose 的 dd/gg 折叠）。
fn segment_to_unit(seg: &Segment) -> UnitRange {
    let (lo, hi) = if seg.start_price <= seg.end_price {
        (seg.start_price, seg.end_price)
    } else {
        (seg.end_price, seg.start_price)
    };
    UnitRange {
        start_index: seg.start_index,
        end_index: seg.end_index,
        lo,
        hi,
    }
}

/// 从一级走势单元序列识别中枢序列（reference:23,29；连续三段窗口扫描）。
///
/// 从左到右扫描：连续三段重叠产生中枢（`center_from_window`，`zd<=zg` 成立）。已确认中枢
/// 不回写重分解（reference:16）——窗口前进到中枢末段之后继续找下一中枢。无重叠的窗口
/// 跳过（非中枢，前进一段）。
///
/// ★诚实范围：v0 用**非重叠三段窗口**识别中枢（连续三段成枢则前进 3 段）。延伸中枢
/// （同一中枢吸收后续段）的完整 start/finish 区间识别留待后续——此处用 Recursive.lean
/// 给的三段窗口 sound contract（`CenterDerivedAt`），不超出已形式化的边界造延伸逻辑。
fn detect_centers(units: &[UnitRange]) -> Vec<Center> {
    let mut centers = Vec::new();
    let mut i = 0usize;
    while i + 2 < units.len() {
        match center::center_from_window(&units[i], &units[i + 1], &units[i + 2]) {
            Some(c) => {
                centers.push(c);
                // 非重叠窗口：前进到三段之后（已确认中枢不回写，reference:16）。
                i += 3;
            }
            None => {
                // 三段无公共重叠 ⟹ 非中枢，前进一段继续找。
                i += 1;
            }
        }
    }
    centers
}

/// 把一级走势单元序列规约为该级走势裁决 + 中枢（reference:29 `classifyMove`）。
///
/// 返回 `(中枢序列, 走势裁决)`：中枢由 `detect_centers` 识别，裁决由 `classify_move`。
/// 这是一级的"完整走势"——其裁决决定该单元序列是趋势/盘整/退化。
fn classify_level(units: &[UnitRange]) -> (Vec<Center>, MoveOutcome) {
    let centers = detect_centers(units);
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

    let mut levels: Vec<LevelState> = Vec::new();

    // 递归级别构造：每级由下级走势单元构造（L0 直接是线段单元，从 L0 开始裁决）。
    for level_idx in 0..=l_max {
        // 自然终止（reference:30）：某层无 ≥min_parts 完成部件 ⟹ 无法产生完整走势，停止。
        if units.len() < min_parts {
            break;
        }

        let (centers, outcome) = classify_level(&units);

        // 走势裁决 → MoveKind（HigherCenterCandidate 退化态映 None，不入 moves）。
        let moves: Vec<MoveKind> = outcome_to_kind(outcome).into_iter().collect();

        // BSP 信号提取（reference:34-36）。
        // ★诚实范围（formalization-validity-domain）：v0 在 **L0** 用线段（次级别走势=线段，
        // 有方向）提取第三类买卖点（confirmed 结构严格可判定，见 signal.rs）。上级级别的
        // 输入单元是中枢外缘区间（`UnitRange` 无方向），第三类「次级别回试」的方向判据无法
        // 在无方向的上级单元上 bit-exact 判定 ⟹ 上级 bsp 留空（非补丁：避免基于无方向单元
        // 的猜测信号）。上级信号需「上级走势携带方向」的递归扩展（units 加 direction），
        // 是后续增量——本工位先打通 L0 端到端（解阻塞点 A）。
        let bsp = if level_idx == 0 {
            signal::extract_signals(&centers, &l0.segments)
        } else {
            Vec::new()
        };

        levels.push(LevelState {
            moves,
            centers: centers.clone(),
            bsp,
        });

        // L(k+1) 输入单元 = 本级中枢外缘区间（compose 的 dd/gg 折叠，Move.interval）。
        // 中枢数 < min_parts ⟹ 上级无法产生完整走势，下轮循环自然终止。
        units = centers
            .iter()
            .map(|c| UnitRange {
                start_index: c.start_index,
                end_index: c.end_index,
                lo: c.dd,
                hi: c.gg,
            })
            .collect();

        // 本级无中枢 ⟹ 无上级输入单元，停止递归（自然终止）。
        if units.is_empty() {
            break;
        }
    }

    Classification { levels }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::parser::ParseLayer;
    use super::super::types::Direction;

    fn seg(dir: Direction, si: usize, ei: usize, sp: i64, ep: i64) -> Segment {
        Segment { direction: dir, start_index: si, end_index: ei, start_price: sp, end_price: ep }
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
        // 三段区间重叠 ⟹ L0 识别出一个中枢。段区间 [0,10],[3,12],[5,15]：
        // zd=max(0,3,5)=5, zg=min(10,12,15)=10 ⟹ 中枢成立。
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
        assert_eq!(l0.centers.len(), 1, "三段重叠 ⟹ 一个中枢");
        assert_eq!((l0.centers[0].zd, l0.centers[0].zg), (5, 10));
        // 一个中枢 ⟹ classifyMove = consolidation ⟹ moves=[Consolidation]。
        assert_eq!(l0.moves, vec![MoveKind::Consolidation]);
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
