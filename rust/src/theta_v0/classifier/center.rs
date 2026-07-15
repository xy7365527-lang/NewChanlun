//! 中枢边界构造（完整判据）+ 中枢关系三态 + 点位三态。
//!
//! ## 契约重锚（legacy Formal/RecursiveConstruction → Origin canonical，task #127 A′ Phase2）
//!
//! - **完整中枢判据** ↔ `Origin.CenterComplete.CenterConfirmedComplete`（CenterComplete.lean）：
//!   三段次级别走势构成真中枢 ⟺ 两条**全部**成立——
//!   1. **方向交替** `DirAlternates`：`s1.dir≠s2.dir ∧ s2.dir≠s3.dir`（§6.1 典型形态
//!      下-上-下/上-下-上）。
//!   2. **全三段核心非空** `computeZD s1 s2 s3 ≤ computeZG s1 s2 s3`（口径 B，§6.3/§6.4：核心 =
//!      前三段重叠部分 `[max(三段低), min(三段高)]`，非空即三段有共同重叠区间）。
//! - 核心/外缘构造 ↔ `Origin.CenterConstruction`：`computeZG s1 s2 s3 = min(三段 hi)`、
//!   `computeZD s1 s2 s3 = max(三段 lo)`（**口径 B 全三段核心**，637号）；`computeGG/computeDD` 三段
//!   聚合外缘 `gg=max(三段hi)`、`dd=min(三段lo)`。
//! - 中枢关系/发展三态 ↔ `Origin.CenterStates.classifyDevelopment`（CenterStates.lean）+
//!   外缘判据 `IsUpTrend next.dd>prev.gg / IsDownTrend next.gg<prev.dd`。
//! - 点位三态 ↔ `Origin.CenterStates.classifyPosition`（CenterStates.lean）：
//!   `p<zd → below`，`zg<p → above`，否则 `within`（闭核心区间 [zd,zg]）。
//!
//! ## 中枢核心区间口径 A→B 迁移（637号谱系裁决，一级权威第17课答疑）
//!
//! 一级权威（缠师第17课答疑严格公式）：三个连续次级别走势 A、B、C 高低点 a1/a2, b1/b2, c1/c2，
//! 中枢区间 = `(max(a2,b2,c2), min(a1,b1,c1))`——**全三段重叠**（口径 B）。误口径 A（前两段
//! `min(g₁,g₂)/max(d₁,d₂)`）只在第三段贯穿核心时与 B 重合。本文件主塔已迁 B。
//!
//! ★第三段贯穿吸收进核心非空（codex 异质核验 L0 等价，637号）：旧 A 口径完整判据三支
//! [方向交替 ∧ 前两段核心非空 ∧ 第三段贯穿 `ThirdSpansCore`] 与 B 口径两支
//! [方向交替 ∧ 全三段核心非空 `ZD_B≤ZG_B`] **接受/拒绝集严格相等**（恒等式：`max(d₁,d₂,d₃)≤
//! min(g₁,g₂,g₃)` ⟺ 前两段核心非空 ∧ 第三段落入核心，其中第 9 个交叉不等式 `d₃≤g₃` 由段不变量
//! `segLow≤segHigh` 永真补齐）。故 B 口径下 `ThirdSpansCore` 被核心非空蕴含——保留它作独立判据支 =
//! 声明膨胀（no-patch 禁止）。`third_spans_core` 降为派生谓词（供测试见证 + 上级几何路径复用），
//! **不**再是 `center_from_segments` 的独立合取支。核心数值 B≠A（B 收窄到三段交：`ZD_B≥ZD_A`、
//! `ZG_B≤ZG_A`），下游消费 ZD/ZG 的位置/三买卖/边界数值随 B 收窄。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 全部 L0：纯整数/方向比较，不依赖经验数据。完整判据两支是 §6.1/§6.3/§6.4 `[缠论可导]`，不进
//! config。A→B 迁移是 L0 接受谓词等价重构（接受集不变）+ 核心数值收窄（非语义变更）。

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

/// 核心上沿 `computeZG s1 s2 s3`（契约锚 `Origin.CenterConstruction.computeZG`）：**全三段** hi 取 min。
///
/// ★口径 B（637号谱系，一级权威第17课答疑严格公式）：核心区间 = 前三个连续次级别走势的**重叠
/// 部分** = `(max(三段低点), min(三段高点))`。误口径 A（前两段 `min(g₁,g₂)`）仅在第三段贯穿核心
/// 时与 B 重合，第三段更窄时 A 高估核心上沿（见谱系 中枢核心区间口径分离）。canonical = B 全三段。
fn compute_zg(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Tick {
    a.hi.min(b.hi).min(c.hi)
}

/// 核心下沿 `computeZD s1 s2 s3`（契约锚 `Origin.CenterConstruction.computeZD`）：**全三段** lo 取 max。
///
/// ★口径 B（637号谱系）：`max(三段低点)`（第17课 max(a2,b2,c2)）。误口径 A（前两段 `max(d₁,d₂)`）
/// 仅在第三段贯穿核心时与 B 重合。canonical = B 全三段。
fn compute_zd(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Tick {
    a.lo.max(b.lo).max(c.lo)
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

/// [口径 A legacy 派生谓词] 第三段贯穿核心判定（**派生谓词**，口径 B 下被核心非空蕴含，637号）。
///
/// 原 A 口径定义"第三段 `[lo,hi]` 落入**前两段**核心 `[max(d₁,d₂), min(g₁,g₂)]`"（§6.3 三段共同
/// 重叠的独立判据支）。口径 B 迁移后核心已是**全三段** `[ZD_B,ZG_B]`，第三段贯穿语义被
/// `compute_zd/compute_zg` 全三段吸收：`ZD_B≤ZG_B`（核心非空）⟺ A 前两段核心非空 ∧ 第三段贯穿
/// （codex L0 等价核验，637号）。故本谓词**不再**是 `center_from_segments` 的独立判据支——降为
/// 派生见证：用前两段核心 `[max(d₁,d₂),min(g₁,g₂)]` 查第三段是否落入，供测试对照 A/B 等价
/// （`third_spans_core(a,b,c) ⟺ ZD_B≤ZG_B` 当前两段核心非空时）。闭区间相切算贯穿。
pub fn third_spans_core(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> bool {
    // 前两段核心（A 口径，仅本派生谓词用——查第三段相对前两段核心的位置）。
    let zg2 = a.hi.min(b.hi);
    let zd2 = a.lo.max(b.lo);
    c.lo <= zg2 && zd2 <= c.hi
}

/// 从连续三段次级别走势单元构造**完整中枢**（契约锚 `Origin.CenterComplete.CenterConfirmedComplete`
/// + `Origin.CenterConstruction.centerFromThree`）。
///
/// 真中枢 ⟺ 两条**全部**成立（口径 B，637号；对齐 `CenterConfirmedComplete` 二支合取）：
/// 1. **方向交替** `dir_alternates`（§6.1，`DirAlternates`）。
/// 2. **全三段核心非空** `compute_zd(a,b,c) ≤ compute_zg(a,b,c)`（§6.3/§6.4 口径 B：核心 = 前三段
///    重叠部分 `[max(三段低),min(三段高)]` 非空 ⟺ 三段有共同重叠区间）。
///
/// ★第三段贯穿已吸收进支2（637号 codex L0 等价）：B 口径核心非空恒等价于 A 口径
/// [前两段核心非空 ∧ 第三段贯穿]，故不再单列第三段判据支（声明膨胀禁止）。
///
/// 任一支不成立 ⟹ 返回 `None`（非中枢，不静默造退化中枢）。成立 ⟹ 核心取**全三段**
/// `[compute_zd(a,b,c), compute_zg(a,b,c)]`，外缘取**三段** dd/gg。
///
/// 边界条件（结论翻转）：方向交替用严格异向；全三段核心非空用闭区间（`ZD_B=ZG_B` 单点核心合法）；
/// 缺方向维度（上级无方向单元）则方向交替不可判——见 `center_from_window`。
pub fn center_from_segments(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Option<Center> {
    // 支1：方向交替（§6.1，DirAlternates）。
    if !dir_alternates(a, b, c) {
        return None;
    }
    // 支2：全三段核心非空（口径 B，§6.3/§6.4——已含第三段贯穿，637号）。
    let zd = compute_zd(a, b, c);
    let zg = compute_zg(a, b, c);
    if zd > zg {
        return None;
    }
    // 两支全成立 ⟹ 真中枢。核心 = 全三段 [zd,zg]（口径 B）；外缘 = 三段 dd/gg。
    Some(Center {
        zd,
        zg,
        dd: compute_dd(a, b, c),
        gg: compute_gg(a, b, c),
        start_index: a.start_index,
        end_index: c.end_index,
    })
}

/// 从连续三段构造中枢（**几何路径**，契约锚 `Origin.CenterConstruction.centerHolds` 全三段口径）。
///
/// ★诚实有效域（formalization-validity-domain，非补丁）：此函数用于**上级递归层**（L≥1），其输入
/// 单元是中枢外缘区间（`UnitRange` 由中枢 dd/gg 合成，**无内在缠论方向**——上级走势方向由 Move
/// 的趋势裁决携带，不在单元区间里）。Origin 的**完整判据** `CenterConfirmedComplete`（方向交替）
/// 定义在 **L0 Segment**（有内在方向）上；上级中枢的发展裁决用 `Origin.CenterStates.classifyDevelopment`
/// （**外缘判据**，无方向交替要求，CenterStates.lean classifyDevelopment）。故此路径只保留**几何**
/// 判据（全三段核心非空，口径 B），**不**查方向交替——这是 Origin 上级发展态判据的有效域，
/// 非省略完整判据：上级单元根本无 §6.1 意义的方向交替维度。
///
/// 核心 = 全三段 `[compute_zd(a,b,c), compute_zg(a,b,c)]`（口径 B，637号——全三段重叠部分非空 ⟺
/// 三段有共同重叠，第三段贯穿已吸收进核心非空）；外缘 = 三段 dd/gg。核心空（`ZD_B>ZG_B`）⟹ `None`。
pub fn center_from_window(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Option<Center> {
    // 全三段核心非空（口径 B——已含第三段贯穿，637号 codex L0 等价）。
    let zd = compute_zd(a, b, c);
    let zg = compute_zg(a, b, c);
    if zd > zg {
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

    /// ★完整判据正例（契约锚 `Origin.CenterComplete.trueCenter_centerConfirmed`，口径 B）：上-下-上，
    /// 全三段核心非空 ⟹ 真中枢。核心取**全三段** [compute_zd, compute_zg]。
    fn up() -> Direction { Direction::Up }
    fn down() -> Direction { Direction::Down }

    #[test]
    fn complete_center_confirmed_bit_exact() {
        // 上-下-上：a=[10,20]up, b=[12,20]down, c=[12,22]up（对齐 Origin trueCenterSeg*）。
        // 核心 = 全三段（口径 B）：zd=max(10,12,12)=12, zg=min(20,20,22)=20。第三段 [12,22] 贯穿前两段
        // 核心 [12,20] ⟹ 全三段核心 = 前两段核心（此 case A/B 重合）。外缘 = 三段：dd=10, gg=22。
        let a = unit(0, 4, up(), 10, 20);
        let b = unit(4, 8, down(), 12, 20);
        let c = unit(8, 12, up(), 12, 22);
        let center = center_from_segments(&a, &b, &c).expect("完整判据两支全成立 ⟹ 真中枢");
        assert_eq!((center.zd, center.zg, center.dd, center.gg), (12, 20, 10, 22));
        assert_eq!((center.start_index, center.end_index), (0, 12));
    }

    /// ★口径 B 收窄正例（637号，A≠B 见证）：第三段更窄 ⟹ 全三段核心严格窄于前两段核心。
    /// 上-下-上：a=[10,20]up, b=[12,20]down, c=[14,18]up。前两段核心 [12,20]，但第三段 [14,18] 收窄。
    /// B 口径核心 = 全三段：zd=max(10,12,14)=14, zg=min(20,20,18)=18 ⟹ [14,18] 严格窄于 A 的 [12,20]。
    #[test]
    fn complete_b_caliber_narrows_core() {
        let a = unit(0, 4, up(), 10, 20);
        let b = unit(4, 8, down(), 12, 20);
        let c = unit(8, 12, up(), 14, 18);
        let center = center_from_segments(&a, &b, &c).expect("全三段核心非空 ⟹ 真中枢");
        // B 收窄：核心 [14,18]（非 A 口径的 [12,20]）。外缘 = 三段 dd=min(10,12,14)=10, gg=max(20,20,18)=20。
        assert_eq!((center.zd, center.zg, center.dd, center.gg), (14, 18, 10, 20));
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

    /// ★完整判据拒绝第三段不贯穿（契约锚 `Origin.CenterComplete.noSpan_not_centerConfirmed`，口径 B）：
    /// 方向交替成立但第三段脱离核心 ⟹ 全三段核心空 ⟹ 完整判据拒绝。
    /// ★口径 B 等价见证（637号）：第三段不贯穿 ⟺ 全三段核心空（第三段贯穿吸收进核心非空）。
    #[test]
    fn complete_rejects_third_not_spanning() {
        // 上-下-上（方向交替）但第三段离开核心（对齐 Origin noSpanSeg*）：
        // a=[10,20]up, b=[12,20]down, c=[40,50]up。前两段核心 [12,20]，第三段 [40,50] segLow=40 > 20 不贯穿。
        // B 口径全三段核心：zd=max(10,12,40)=40 > zg=min(20,20,50)=20 ⟹ 核心空。
        let a = unit(0, 4, up(), 10, 20);
        let b = unit(4, 8, down(), 12, 20);
        let c = unit(8, 12, up(), 40, 50);
        assert!(dir_alternates(&a, &b, &c), "方向交替成立");
        // 全三段核心空（口径 B）= 第三段不贯穿前两段核心（派生谓词 A 口径，两者 637号等价）。
        assert_eq!(compute_zd(&a, &b, &c), 40, "全三段 zd = max(10,12,40)");
        assert_eq!(compute_zg(&a, &b, &c), 20, "全三段 zg = min(20,20,50)");
        assert!(!third_spans_core(&a, &b, &c), "第三段不贯穿前两段核心（派生谓词）");
        assert_eq!(center_from_segments(&a, &b, &c), None, "全三段核心空（第三段不贯穿）⟹ 完整判据拒绝");
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
        // 同向三段（口径 B 全三段核心非空）：a=[10,20]up, b=[18,25]up, c=[18,22]up
        // （核心 zd=max(10,18,18)=18, zg=min(20,25,22)=20 ⟹ [18,20] 非空）。
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
    fn geometric_window_three_segment_core_bit_exact() {
        // 全三段定核心（口径 B）：a=[0,10], b=[3,12], c=[5,15]。
        // zd=max(0,3,5)=5, zg=min(10,12,15)=10。第三段 [5,15] 收窄核心下沿（A 口径 zd=3 → B zd=5）。
        // dd=min(0,3,5)=0, gg=max(10,12,15)=15。
        let a = unit(0, 4, up(), 0, 10);
        let b = unit(4, 8, down(), 3, 12);
        let c = unit(8, 12, up(), 5, 15);
        let center = center_from_window(&a, &b, &c).expect("全三段核心非空 ⟹ 中枢");
        assert_eq!((center.zd, center.zg, center.dd, center.gg), (5, 10, 0, 15));
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
        // 全三段核心空（口径 B）：a=[0,5], b=[5,10], c=[10,12]。
        // zd=max(0,5,10)=10 > zg=min(5,10,12)=5 ⟹ 核心空（= 第三段不贯穿前两段核心 [5,5]）⟹ 非中枢。
        let a = unit(0, 4, up(), 0, 5);
        let b = unit(4, 8, down(), 5, 10);
        let c = unit(8, 12, up(), 10, 12);
        assert_eq!(center_from_window(&a, &b, &c), None, "全三段核心空（第三段不贯穿）⟹ 非中枢");
    }

    #[test]
    fn geometric_window_boundary_zd_eq_zg_is_center() {
        // 退化但合法（口径 B 单点核心）：a=[0,5], b=[5,10], c=[3,5]。
        // zd=max(0,5,3)=5, zg=min(5,10,5)=5 ⟹ 全三段核心 [5,5]（单点，闭区间合法）。
        let a = unit(0, 4, up(), 0, 5);
        let b = unit(4, 8, down(), 5, 10);
        let c = unit(8, 12, up(), 3, 5);
        let center = center_from_window(&a, &b, &c).expect("全三段单点核心 zd==zg ⟹ 闭区间中枢成立");
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
