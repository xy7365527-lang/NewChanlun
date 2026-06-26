//! 中枢边界构造（完整判据）+ 中枢关系三态 + 点位三态。
//!
//! ## 契约重锚（legacy Formal/RecursiveConstruction → Origin canonical，task #127 A′ Phase2）
//!
//! - **完整中枢判据** ↔ `Origin.CenterComplete.CenterConfirmedComplete`（CenterComplete.lean:108-111）：
//!   三段次级别走势构成真中枢 ⟺ 三条**全部**成立——
//!   1. **方向交替** `DirAlternates`（:61-62）：`s1.dir≠s2.dir ∧ s2.dir≠s3.dir`（§6.1 典型形态
//!      下-上-下/上-下-上）。
//!   2. **核心非空** `computeZD s1 s2 ≤ computeZG s1 s2`（§6.4，**前两段**定核心，CenterConstruction.lean）。
//!   3. **第三段贯穿** `ThirdSpansCore`（:88-89）：`segLow s3 ≤ ZG ∧ ZD ≤ segHigh s3`（§6.3 三段共同重叠）。
//! - 核心/外缘构造 ↔ `Origin.CenterConstruction`：`computeZG s1 s2 = tmin(segHigh s1)(segHigh s2)`、
//!   `computeZD s1 s2 = tmax(segLow s1)(segLow s2)`（**前两段**核心）；`computeGG/computeDD` 三段聚合外缘
//!   `gg=tmax(三段hi)`、`dd=tmin(三段lo)`（CenterConstruction.lean computeGG/computeDD）。`centerFromThree`
//!   （:centerFromThree）由确认的三段构造 `CenterFull`（核心 + 外缘 DD/GG）。
//! - 中枢关系/发展三态 ↔ `Origin.CenterStates.classifyDevelopment`（CenterStates.lean）+
//!   外缘判据 `IsUpTrend next.dd>prev.gg / IsDownTrend next.gg<prev.dd`。
//! - 点位三态 ↔ `Origin.CenterStates.classifyPosition`（CenterStates.lean）：
//!   `p<zd → below`，`zg<p → above`，否则 `within`（闭核心区间 [zd,zg]）。
//!
//! ## G4 完整判据重锚（关键语义升级，非改名）
//!
//! 旧 rust `center_from_window` 是 Origin `centerHolds`（**几何必要条件**：仅查 `zd<=zg`）的对应物，
//! 它**忽略方向交替 + 第三段贯穿**两个维度（CenterComplete.lean §4 反退化见证 sameDirSeg*/noSpanSeg*
//! 证两者结论分叉）。本文件重锚到 `CenterConfirmedComplete` 完整判据——`center_from_segments`
//! 携带方向，逐字对齐 Origin 三支合取（方向交替 ∧ 核心非空[前两段] ∧ 第三段贯穿）。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 全部 L0：纯整数/方向比较，不依赖经验数据。完整判据三支是 §6.1/§6.3/§6.4 `[缠论可导]`，不进 config。

use super::super::types::{Center, Direction, Tick};

/// 走势单元的方向 + 价格区间投影（`lo<=hi` 不变量 + 方向，对齐 `Origin.Segment`）。
///
/// 对齐 `Origin.ChanlunElements.Segment`（含 direction）：中枢由次级别走势单元构造，**方向**是
/// 完整判据 `DirAlternates` 的必要输入（旧 `UnitRange` 丢弃方向，无法判方向交替——G4 缺口根源）。
/// `segHigh/segLow` 对齐 `Origin.CenterConstruction.segHigh/segLow`（向上段 hi=端价/向下段 lo=端价
/// 已规约为 `[lo,hi]`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitRange {
    /// 该走势单元在 L0 原始 K 序的起点（中枢 start_index 取首单元起点）。
    pub start_index: usize,
    /// 该走势单元在 L0 原始 K 序的终点（中枢 end_index 取末单元终点）。
    pub end_index: usize,
    /// 走势单元方向（完整判据 `DirAlternates` 的输入；对齐 `Origin.Segment.direction`）。
    pub direction: Direction,
    pub lo: Tick,
    pub hi: Tick,
}

/// 中枢关系三态（契约锚 `Origin.CenterStates.CenterDevelopment` 外缘趋势判据）。
///
/// 两个**核心已分离的同级别新生中枢**的关系（外缘 dd/gg 判据，第18/20课中心定理二）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterRelation {
    /// 上涨延续：`next.dd > prev.gg`（外缘完全分离向上）。
    UpContinuation,
    /// 下跌延续：`next.gg < prev.dd`（外缘完全分离向下）。
    DownContinuation,
    /// 级别扩张：外缘重叠（既非向上分离也非向下分离）。
    LevelExpansion,
}

/// 点相对中枢核心区间 `[zd,zg]` 的位置三态（契约锚 `Origin.CenterStates.CenterPosition`）。
///
/// ★边界口径（`Origin.CenterStates.classifyPosition`）：闭核心区间 `[zd,zg]` 归 within；破 zg
/// （`p>zg`）才 above，跌破 zd（`p<zd`）才 below。第49课「小于 ZD / 大于 ZG」严格不等式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativePosition {
    /// 之下：`p < zd`。
    Below,
    /// 之中：`zd <= p <= zg`（闭核心区间）。
    Within,
    /// 之上：`p > zg`。
    Above,
}

/// 核心上沿 `computeZG s1 s2`（契约锚 `Origin.CenterConstruction.computeZG`）：**前两段** hi 取 min。
fn compute_zg(a: &UnitRange, b: &UnitRange) -> Tick {
    a.hi.min(b.hi)
}

/// 核心下沿 `computeZD s1 s2`（契约锚 `Origin.CenterConstruction.computeZD`）：**前两段** lo 取 max。
fn compute_zd(a: &UnitRange, b: &UnitRange) -> Tick {
    a.lo.max(b.lo)
}

/// 外缘上沿 `computeGG s1 s2 s3`（契约锚 `Origin.CenterConstruction.computeGG`）：三段 hi 取 max。
fn compute_gg(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Tick {
    a.hi.max(b.hi.max(c.hi))
}

/// 外缘下沿 `computeDD s1 s2 s3`（契约锚 `Origin.CenterConstruction.computeDD`）：三段 lo 取 min。
fn compute_dd(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Tick {
    a.lo.min(b.lo.min(c.lo))
}

/// 三段方向交替判定（契约锚 `Origin.CenterComplete.DirAlternates`，CenterComplete.lean:61-62）。
///
/// `s1.dir≠s2.dir ∧ s2.dir≠s3.dir`（§6.1 典型形态下-上-下/上-下-上）。二元方向下交替 ⟺ 相邻异向。
/// 这正是旧 `center_from_window` 完全忽略的维度（仅看价位不看方向）。
pub fn dir_alternates(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> bool {
    a.direction != b.direction && b.direction != c.direction
}

/// 第三段贯穿核心判定（契约锚 `Origin.CenterComplete.ThirdSpansCore`，CenterComplete.lean:88-89）。
///
/// `segLow s3 ≤ computeZG s1 s2 ∧ computeZD s1 s2 ≤ segHigh s3`（§6.3 三段共同重叠）。前两段定核心
/// `[ZD,ZG]`，要求第三段 `[lo,hi]` 也与核心重叠（否则三段无共同重叠部分，非中枢）。闭区间相切算贯穿。
pub fn third_spans_core(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> bool {
    let zg = compute_zg(a, b);
    let zd = compute_zd(a, b);
    c.lo <= zg && zd <= c.hi
}

/// 从连续三段次级别走势单元构造**完整中枢**（契约锚 `Origin.CenterComplete.CenterConfirmedComplete`
/// + `Origin.CenterConstruction.centerFromThree`，CenterComplete.lean:108-111/centerFromThree）。
///
/// 真中枢 ⟺ 三条**全部**成立（逐字对齐 `CenterConfirmedComplete` 三支合取）：
/// 1. **方向交替** `dir_alternates`（§6.1，`DirAlternates`）。
/// 2. **核心非空** `compute_zd(a,b) ≤ compute_zg(a,b)`（§6.4，**前两段**定核心）。
/// 3. **第三段贯穿** `third_spans_core`（§6.3，`ThirdSpansCore`）。
///
/// 任一支不成立 ⟹ 返回 `None`（非中枢，不静默造退化中枢）。成立 ⟹ 核心取**前两段**
/// `[compute_zd(a,b), compute_zg(a,b)]`（对齐 Origin centerFromThree.core），外缘取**三段** dd/gg。
///
/// 边界条件（结论翻转，CenterComplete.lean §7）：方向交替用严格异向；第三段贯穿用闭区间重叠
/// （segLow s3 = ZG 算贯穿）；缺方向维度（上级无方向单元）则方向交替不可判——见 `center_from_window`。
pub fn center_from_segments(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Option<Center> {
    // 支1：方向交替（§6.1，DirAlternates）。
    if !dir_alternates(a, b, c) {
        return None;
    }
    // 支2：核心非空（§6.4，前两段定核心）。
    let zd = compute_zd(a, b);
    let zg = compute_zg(a, b);
    if zd > zg {
        return None;
    }
    // 支3：第三段贯穿核心（§6.3，ThirdSpansCore）。
    if !third_spans_core(a, b, c) {
        return None;
    }
    // 三支全成立 ⟹ 真中枢。核心 = 前两段 [zd,zg]；外缘 = 三段 dd/gg（centerFromThree）。
    Some(Center {
        zd,
        zg,
        dd: compute_dd(a, b, c),
        gg: compute_gg(a, b, c),
        start_index: a.start_index,
        end_index: c.end_index,
    })
}

/// 从连续三段构造中枢（**几何路径**，契约锚 `Origin.CenterConstruction.centerHolds` + 三段共同重叠）。
///
/// ★诚实有效域（formalization-validity-domain，非补丁）：此函数用于**上级递归层**（L≥1），其输入
/// 单元是中枢外缘区间（`UnitRange` 由中枢 dd/gg 合成，**无内在缠论方向**——上级走势方向由 Move
/// 的趋势裁决携带，不在单元区间里）。Origin 的**完整判据** `CenterConfirmedComplete`（方向交替）
/// 定义在 **L0 Segment**（有内在方向）上；上级中枢的发展裁决用 `Origin.CenterStates.classifyDevelopment`
/// （**外缘判据**，无方向交替要求，CenterStates.lean classifyDevelopment）。故此路径只保留**几何**
/// 三支（核心非空[前两段] + 第三段贯穿 + 三段共同重叠），**不**查方向交替——这是 Origin 上级
/// 发展态判据的有效域，非省略完整判据：上级单元根本无 §6.1 意义的方向交替维度。
///
/// 核心 = 前两段 `[compute_zd, compute_zg]`（对齐 centerFromThree.core）；要求第三段贯穿核心
/// （`third_spans_core`，§6.3 三段共同重叠）；外缘 = 三段 dd/gg。任一几何支不成立 ⟹ `None`。
pub fn center_from_window(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Option<Center> {
    // 核心非空（前两段，对齐 centerHolds）。
    let zd = compute_zd(a, b);
    let zg = compute_zg(a, b);
    if zd > zg {
        return None;
    }
    // 第三段贯穿核心（§6.3 三段共同重叠——上级亦要求三段重叠才是中枢）。
    if !third_spans_core(a, b, c) {
        return None;
    }
    Some(Center {
        zd,
        zg,
        dd: compute_dd(a, b, c),
        gg: compute_gg(a, b, c),
        start_index: a.start_index,
        end_index: c.end_index,
    })
}

/// 判定两中枢关系（契约锚 `Origin.CenterStates`：`IsUpTrend`/`IsDownTrend`/发展态外缘判据）。
///
/// `next.dd>prev.gg → up`（`Origin.CenterStates.IsUpTrend`），`next.gg<prev.dd → down`
/// （`IsDownTrend`），否则 `expansion`。逐字对齐 Origin 外缘趋势判据（CenterStates.lean）。
pub fn classify_relation(prev: &Center, next: &Center) -> CenterRelation {
    if next.dd > prev.gg {
        CenterRelation::UpContinuation
    } else if next.gg < prev.dd {
        CenterRelation::DownContinuation
    } else {
        CenterRelation::LevelExpansion
    }
}

/// 判定点位三态（契约锚 `Origin.CenterStates.classifyPosition`，闭核心区间）。
///
/// `p<zd → below`，`zg<p → above`，否则 `within`。逐字对齐 `Origin.CenterStates.classifyPosition`
/// （CenterStates.lean：`if p < c.zd then below else if c.zg < p then above else within`）。
pub fn classify_position(c: &Center, p: Tick) -> RelativePosition {
    if p < c.zd {
        RelativePosition::Below
    } else if c.zg < p {
        RelativePosition::Above
    } else {
        RelativePosition::Within
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 方向交替单元（默认 up-down-up 用于完整判据正例；几何路径忽略方向）。
    fn unit(si: usize, ei: usize, dir: Direction, lo: Tick, hi: Tick) -> UnitRange {
        UnitRange { start_index: si, end_index: ei, direction: dir, lo, hi }
    }

    // ──────────────────────────────────────────────────────────────────────
    //  G4 完整判据 center_from_segments（契约锚 Origin.CenterComplete.CenterConfirmedComplete）
    // ──────────────────────────────────────────────────────────────────────

    /// ★完整判据正例（契约锚 `Origin.CenterComplete.trueCenter_centerConfirmed`）：上-下-上，
    /// 前两段核心非空，第三段贯穿核心 ⟹ 真中枢。核心取**前两段** [compute_zd, compute_zg]。
    fn up() -> Direction { Direction::Up }
    fn down() -> Direction { Direction::Down }

    #[test]
    fn complete_center_confirmed_bit_exact() {
        // 上-下-上：a=[10,20]up, b=[12,20]down, c=[12,22]up（对齐 Origin trueCenterSeg*）。
        // 核心 = 前两段：zd=max(10,12)=12, zg=min(20,20)=20。第三段 [12,22] 贯穿 [12,20]（12<=20 ∧ 12<=22）。
        // 外缘 = 三段：dd=min(10,12,12)=10, gg=max(20,20,22)=22。
        let a = unit(0, 4, up(), 10, 20);
        let b = unit(4, 8, down(), 12, 20);
        let c = unit(8, 12, up(), 12, 22);
        let center = center_from_segments(&a, &b, &c).expect("完整判据三支全成立 ⟹ 真中枢");
        assert_eq!((center.zd, center.zg, center.dd, center.gg), (12, 20, 10, 22));
        assert_eq!((center.start_index, center.end_index), (0, 12));
    }

    /// ★完整判据拒绝同向三段（契约锚 `Origin.CenterComplete.sameDir_not_centerConfirmed`）：
    /// 全 up（无方向交替）⟹ 完整判据拒绝（即使前两段核心非空 centerHolds 成立）。
    #[test]
    fn complete_rejects_same_direction() {
        // 全 up（对齐 Origin sameDirSeg*）：a=[10,20], b=[18,25], c=[22,30]。
        // 前两段核心 zd=max(10,18)=18 <= zg=min(20,25)=20（centerHolds 成立），但全 up 无方向交替。
        let a = unit(0, 4, up(), 10, 20);
        let b = unit(4, 8, up(), 18, 25);
        let c = unit(8, 12, up(), 22, 30);
        assert_eq!(center_from_segments(&a, &b, &c), None, "无方向交替 ⟹ 完整判据拒绝");
        // 但方向交替谓词单独可见（dir_alternates=false）。
        assert!(!dir_alternates(&a, &b, &c));
    }

    /// ★完整判据拒绝第三段不贯穿（契约锚 `Origin.CenterComplete.noSpan_not_centerConfirmed`）：
    /// 方向交替成立但第三段脱离核心 ⟹ 完整判据拒绝（centerHolds 仅查前两段会漏判）。
    #[test]
    fn complete_rejects_third_not_spanning() {
        // 上-下-上（方向交替）但第三段离开核心（对齐 Origin noSpanSeg*）：
        // a=[10,20]up, b=[12,20]down, c=[40,50]up。核心 [12,20]，第三段 [40,50] segLow=40 > zg=20 ⟹ 不贯穿。
        let a = unit(0, 4, up(), 10, 20);
        let b = unit(4, 8, down(), 12, 20);
        let c = unit(8, 12, up(), 40, 50);
        assert!(dir_alternates(&a, &b, &c), "方向交替成立");
        assert_eq!(compute_zd(&a, &b), 12);
        assert_eq!(compute_zg(&a, &b), 20);
        assert!(!third_spans_core(&a, &b, &c), "第三段不贯穿核心");
        assert_eq!(center_from_segments(&a, &b, &c), None, "第三段不贯穿 ⟹ 完整判据拒绝");
    }

    /// ★完整判据拒绝核心空（前两段无重叠）：zd>zg ⟹ 拒绝（即使方向交替）。
    #[test]
    fn complete_rejects_empty_core() {
        // 上-下-上但前两段无重叠：a=[0,4]up, b=[10,14]down, c=[5,9]up。
        // 核心 zd=max(0,10)=10 > zg=min(4,14)=4 ⟹ 核心空。
        let a = unit(0, 4, up(), 0, 4);
        let b = unit(4, 8, down(), 10, 14);
        let c = unit(8, 12, up(), 5, 9);
        assert_eq!(center_from_segments(&a, &b, &c), None, "前两段核心空 ⟹ 拒绝");
    }

    /// 完整判据严格强于几何路径（对齐 `Origin.CenterComplete.complete_strictly_refines_centerHolds`）：
    /// 同向三段 center_from_window（几何）接受，center_from_segments（完整）拒绝——两者结论分叉。
    #[test]
    fn complete_strictly_refines_geometric() {
        // 同向三段 + 第三段贯穿：a=[10,20]up, b=[18,25]up, c=[18,22]up（核心 [18,20]，第三段 [18,22] 贯穿）。
        let a = unit(0, 4, up(), 10, 20);
        let b = unit(4, 8, up(), 18, 25);
        let c = unit(8, 12, up(), 18, 22);
        // 几何路径（忽略方向）接受。
        assert!(center_from_window(&a, &b, &c).is_some(), "几何路径接受同向（不查方向）");
        // 完整判据拒绝（无方向交替）。
        assert_eq!(center_from_segments(&a, &b, &c), None, "完整判据拒绝同向（查方向交替）");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  几何路径 center_from_window（上级递归层，契约锚 Origin.centerHolds + 三段共同重叠）
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn geometric_window_two_segment_core_bit_exact() {
        // 前两段定核心：a=[0,10], b=[3,12], c=[5,15]。zd=max(0,3)=3, zg=min(10,12)=10。
        // 第三段 [5,15] 贯穿 [3,10]（5<=10 ∧ 3<=15）。dd=min(0,3,5)=0, gg=max(10,12,15)=15。
        let a = unit(0, 4, up(), 0, 10);
        let b = unit(4, 8, down(), 3, 12);
        let c = unit(8, 12, up(), 5, 15);
        let center = center_from_window(&a, &b, &c).expect("前两段核心非空 + 第三段贯穿 ⟹ 中枢");
        assert_eq!((center.zd, center.zg, center.dd, center.gg), (3, 10, 0, 15));
        assert_eq!((center.start_index, center.end_index), (0, 12));
    }

    #[test]
    fn geometric_window_no_overlap_yields_none() {
        // 前两段无公共重叠（[0,4],[10,14]）：zd=10 > zg=4 ⟹ 非中枢。
        let a = unit(0, 4, up(), 0, 4);
        let b = unit(4, 8, down(), 10, 14);
        let c = unit(8, 12, up(), 20, 24);
        assert_eq!(center_from_window(&a, &b, &c), None);
    }

    #[test]
    fn geometric_window_third_not_spanning_yields_none() {
        // 前两段核心 [5,5]（zd=zg），第三段 [10,12] 不贯穿（segLow=10 > zg=5）⟹ 非中枢。
        let a = unit(0, 4, up(), 0, 5);
        let b = unit(4, 8, down(), 5, 10);
        let c = unit(8, 12, up(), 10, 12);
        assert_eq!(center_from_window(&a, &b, &c), None, "第三段不贯穿 ⟹ 非中枢");
    }

    #[test]
    fn geometric_window_boundary_zd_eq_zg_is_center() {
        // 退化但合法：zd==zg（前两段 [0,5],[5,10] ⟹ zd=max(0,5)=5, zg=min(5,10)=5），第三段 [3,5] 贯穿。
        let a = unit(0, 4, up(), 0, 5);
        let b = unit(4, 8, down(), 5, 10);
        let c = unit(8, 12, up(), 3, 5);
        let center = center_from_window(&a, &b, &c).expect("zd==zg + 第三段贯穿 ⟹ 闭区间中枢成立");
        assert_eq!((center.zd, center.zg), (5, 5));
    }

    fn center(dd: Tick, zd: Tick, zg: Tick, gg: Tick) -> Center {
        Center { zd, zg, dd, gg, start_index: 0, end_index: 0 }
    }

    #[test]
    fn relation_up_continuation_bit_exact() {
        // prev 外缘[0,8]，next 外缘[9,15]：next.dd=9 > prev.gg=8 ⟹ up（chain2_up 同构）。
        let prev = center(0, 2, 5, 8);
        let next = center(9, 10, 13, 15);
        assert_eq!(classify_relation(&prev, &next), CenterRelation::UpContinuation);
    }

    #[test]
    fn relation_down_continuation_bit_exact() {
        // prev 外缘[9,15]，next 外缘[0,8]：next.gg=8 < prev.dd=9 ⟹ down。
        let prev = center(9, 10, 13, 15);
        let next = center(0, 2, 5, 8);
        assert_eq!(classify_relation(&prev, &next), CenterRelation::DownContinuation);
    }

    #[test]
    fn relation_level_expansion_bit_exact() {
        // codex 见证 prev=(0,2,5,8), next=(4,6,9,11)：核心分离 + 外缘重叠 ⟹ expansion。
        let prev = center(0, 2, 5, 8);
        let next = center(4, 6, 9, 11);
        assert_eq!(classify_relation(&prev, &next), CenterRelation::LevelExpansion);
    }

    #[test]
    fn position_three_states_bit_exact_origin() {
        // 核心 [0,2]：p=-1 below；p=1 within；p=5 above（Origin.CenterStates.classifyPosition 见证）。
        let c = center(-1, 0, 2, 3);
        assert_eq!(classify_position(&c, -1), RelativePosition::Below);
        assert_eq!(classify_position(&c, 1), RelativePosition::Within);
        assert_eq!(classify_position(&c, 5), RelativePosition::Above);
    }

    #[test]
    fn position_boundary_closed_core_within() {
        // 边界点 p=zd、p=zg 归 within（闭核心区间，Origin.CenterStates 严格不等式判据）。
        let c = center(-1, 0, 2, 3);
        assert_eq!(classify_position(&c, 0), RelativePosition::Within); // p=zd
        assert_eq!(classify_position(&c, 2), RelativePosition::Within); // p=zg
    }

    /// property：点位三态对任意点穷尽（below/within/above 恰一，Origin.CenterStates position_total/disjoint）。
    #[test]
    fn property_position_total_exclusive() {
        let c = center(-5, 0, 10, 15);
        for p in -20..=30 {
            let r = classify_position(&c, p);
            let below = p < c.zd;
            let above = c.zg < p;
            let within = c.zd <= p && p <= c.zg;
            // 恰好一态成立（穷尽 + 互斥）。
            let count = [below, within, above].iter().filter(|&&b| b).count();
            assert_eq!(count, 1, "p={p} 必恰好落一态");
            match r {
                RelativePosition::Below => assert!(below),
                RelativePosition::Within => assert!(within),
                RelativePosition::Above => assert!(above),
            }
        }
    }
}
