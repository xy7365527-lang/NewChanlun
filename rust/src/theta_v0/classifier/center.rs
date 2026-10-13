//! 中枢边界构造（完整判据）+ 中枢关系四态（新生上/下 + 扩展 + 延伸，#898 写全扩展支）+ 点位三态。
//!
//! ## 契约重锚（legacy Formal/RecursiveConstruction → Origin canonical，task #127 A′ Phase2）
//!
//! - **完整中枢判据** ↔ `Origin.CenterComplete.CenterConfirmedComplete`（CenterComplete.lean）：
//!   三段次级别走势构成真中枢 ⟺ 两条**全部**成立——
//!   1. **方向交替** `DirAlternates`：`s1.dir≠s2.dir ∧ s2.dir≠s3.dir`（§6.1 典型形态
//!      下-上-下/上-下-上）。
//!   2. **全三段核心非空** `computeZD s1 s2 s3 < computeZG s1 s2 s3`（**严格**，#321 裁定
//!      2026-07-26；口径 B，§6.3/§6.4：核心 = 前三段重叠部分 `[max(三段低), min(三段高)]`，
//!      **严格**非空 ⟹ 单点核心 `computeZD==computeZG` 不成立）。★与 Lean `Origin.CenterComplete`
//!      / `Origin.CenterConstruction.centerHolds` 的 `≤`（弱，单点核心成立）在**端点分支**上
//!      不一致——Lean 侧未随 #321 跟进，此处口径分歧登记见 `center_from_segments` doc。
//! - 核心/外缘构造 ↔ `Origin.CenterConstruction`：`computeZG s1 s2 s3 = min(三段 hi)`、
//!   `computeZD s1 s2 s3 = max(三段 lo)`（**口径 B 全三段核心**，637号）；`computeGG/computeDD` 三段
//!   聚合外缘 `gg=max(三段hi)`、`dd=min(三段lo)`。
//! - 中枢关系/发展态 ↔ `Origin.CenterStates.classifyDevelopment`（CenterStates.lean：新生
//!   (上/下)=外缘分离 `IsUpTrend next.dd>prev.gg / IsDownTrend next.gg<prev.dd`；扩展=核心
//!   分离∧外缘重叠；延伸=核心重叠）——#898 起四态写全，扩展支不再 else 兜底。
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
//! [方向交替 ∧ 全三段核心非空 `ZD_B<ZG_B`]（**严格**，#321 裁定 2026-07-26 从严后的口径）
//! **接受/拒绝集严格相等**（恒等式：`max(d₁,d₂,d₃)≤min(g₁,g₂,g₃)` ⟺ 前两段核心非空 ∧ 第三段
//! 落入核心，其中第 9 个交叉不等式 `d₃≤g₃` 由段不变量 `segLow≤segHigh` 永真补齐——**该恒等式
//! 证明本身是在旧弱口径 `ZD_B≤ZG_B` 下完成的**；#321 从严后的严格版重述与证明见下方"严格版等价式
//! 重述"节，需额外前提「每段 `lo<hi`」）。故 B 口径下 `ThirdSpansCore` 被
//! 核心非空蕴含——保留它作独立判据支 = 声明膨胀（no-patch 禁止）。`third_spans_core` 降为派生
//! 谓词（供测试见证 + 上级几何路径复用），**不**再是 `center_from_segments` 的独立合取支。核心
//! 数值 B≠A（B 收窄到三段交：`ZD_B≥ZD_A`、`ZG_B≤ZG_A`），下游消费 ZD/ZG 的位置/三买卖/边界数值
//! 随 B 收窄。
//!
//! ## 严格版等价式重述（#328 交付，2026-07-27）
//!
//! 上段"接受/拒绝集严格相等"原证明基于**弱**口径 `ZD_B≤ZG_B`，靠段不变量 `d₃≤g₃`（对任意段恒真）
//! 补齐第 9 个交叉不等式。#321 从严为 `ZD_B<ZG_B` 后，该步不再由 `UnitRange` 的 `lo<=hi` 不变量
//! 永真补齐。严格版重述如下（记 `dᵢ=segᵢ.lo`、`gᵢ=segᵢ.hi`）：
//!
//! > **前提（非退化段）**：`d₁<g₁ ∧ d₂<g₂ ∧ d₃<g₃`。
//! > **等价式**：`ZD_B < ZG_B` ⟺ `max(d₁,d₂) < min(g₁,g₂)`（前两段核心**严格**非空）
//! > `∧ d₃ < min(g₁,g₂) ∧ max(d₁,d₂) < g₃`（第三段**严格**贯穿前两段核心）。
//!
//! **证明**。`ZD_B<ZG_B` 即 `max(d₁,d₂,d₃) < min(g₁,g₂,g₃)`，由 max/min 定义等价于 9 条交叉不等式
//! `∀i,j∈{1,2,3}: dᵢ<gⱼ`。按 (i,j) 分三组：`i,j∈{1,2}` 的 4 条 ⟺ 前两段核心严格非空；
//! `(i=3, j∈{1,2})` 的 2 条 ⟺ `d₃<min(g₁,g₂)`，`(i∈{1,2}, j=3)` 的 2 条 ⟺ `max(d₁,d₂)<g₃`，
//! 合起来 ⟺ 第三段严格贯穿；余下第 9 条 `d₃<g₃` 恰是前提第三项。⟸ 三组并起来给出全部 9 条，
//! 取 max/min 得 `ZD_B<ZG_B`；⟹ 由 `max(d₁,d₂)≤ZD_B`、`d₃≤ZD_B`、`ZG_B≤min(g₁,g₂)`、`ZG_B≤g₃`
//! 逐条放缩取出。∎
//!
//! **前提不可省（反例）**：`seg₁=seg₂=[0,10]`、`seg₃=[5,5]`（退化段，`lo<=hi` 满足但 `lo<hi` 不满足）
//! ——右边全真（前两段核心 `[0,10]` 严格非空；`5<10 ∧ 0<5`），但 `ZD_B=5=ZG_B`，左边假。故弱口径下
//! 靠不变量白拿的第 9 条，严格版下必须由「每段 `lo<hi`」显式供给。**该前提 `UnitRange` 不保证**
//! （只有 `lo<=hi`），因此严格版等价式是**条件**等价，非无条件恒等式；`center_from_segments` 的
//! 拒绝集不受影响（退化段场景左边为假 ⟹ 拒绝，方向安全）。
//!
//! **实装口径分叉（保留，非缺陷）**：`third_spans_core` 仍是**闭区间**（弱）贯穿见证
//! `c.lo<=zg₂ ∧ zd₂<=c.hi`，与严格版右边的严格贯穿在**相切**处分叉（`seg₁=seg₂=[0,10]`、
//! `seg₃=[10,20]`：闭区间贯穿真、严格贯穿假、`ZD_B=ZG_B=10`）。该谓词零生产调用点（仅测试见证），
//! 按反膨胀不改实装——见其函数 doc。
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

/// 中枢关系四态（契约锚 `Origin.CenterStates.classifyDevelopment`：新生(上/下)/扩展/延伸）。
///
/// 中心定理二（`020-第20课.md:58`【正文】）管**核心已分离**的同级别新生中枢对——
/// 上涨延续／下跌延续／级别扩张三态；核心重叠对属中心定理一（延伸——同一中枢继续震荡，
/// `030-第30课.md:92`【答疑】「在一个中枢还延伸的时候，只有一个中枢」），**不**进入中心定理二的
/// 三歧（`Formal.CenterTrichotomy.SameLevelNewCenterPair` 前件的补集；`Origin.CenterStates.
/// classifyDevelopment` 的 `extension` 支）。
///
/// ★#898：扩展支写全前，`LevelExpansion` 是 `¬up ∧ ¬down` 的 else 兜底，缺核心分离前件，
/// 把延伸（核心重叠，约 27% 相邻中枢对，探针 #894 P-2）也吞进扩展——比原文宽。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CenterRelation {
    /// 上涨延续：`next.dd > prev.gg`（外缘完全分离向上）。
    UpContinuation,
    /// 下跌延续：`next.gg < prev.dd`（外缘完全分离向下）。
    DownContinuation,
    /// 级别扩张：核心分离 ∧ 外缘重叠（`(next.zg<prev.zd ∧ next.gg>=prev.dd) ∨
    /// (next.zd>prev.zg ∧ next.dd<=prev.gg)`，`020:58` 原公式）——形成高级别走势中枢。
    LevelExpansion,
    /// 中枢延伸：核心重叠（中心定理一）——两中枢实为同一中枢的延伸，不属中心定理二三态。
    CoreOverlap,
}

impl CenterRelation {
    /// 是否趋势延续关系（Up/DownContinuation——M-2 外缘分离判「是不是趋势」，#815 转正）。
    /// 扩展（升一级）与延伸（中心定理一）均非趋势关系，趋势 run 在二者处一律断链。
    pub fn is_continuation(self) -> bool {
        matches!(
            self,
            CenterRelation::UpContinuation | CenterRelation::DownContinuation
        )
    }

    /// 趋势延续关系携带的方向；扩展/延伸（非趋势关系）→ None。
    pub fn trend_direction(self) -> Option<Direction> {
        match self {
            CenterRelation::UpContinuation => Some(Direction::Up),
            CenterRelation::DownContinuation => Some(Direction::Down),
            CenterRelation::LevelExpansion | CenterRelation::CoreOverlap => None,
        }
    }
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

/// Zn 判定（★#1419 D-A，全仓唯一一份）：`u` 与 seed 第一单元**方向相等** ⟺ `u` 是 Zn。
///
/// 原文 `020-第20课.md:52`【正文】：「与中枢方向一致的次级别走势类型称为Z走势段……GG=max(gn)……
/// DD=min(dn)，n遍历中枢中所有Zn」——中枢方向由形成它的首段定（回升形成的中枢由向上的次级别走势
/// 确定），故判据取「与 seed 首单元同向」。L0 端点相连交替时即位次奇偶（第 1、3、5…段）；上级输入
/// 方向不保证交替，两种判据在那里分歧，按 D-A 取方向（Lean `CenterConstruct.lean` 同拍带方向）。
/// seed 外缘（`compute_gg/compute_dd`）与延伸外缘（`recursive_tower::detect_centers_windowed_resume`）
/// 都只经本谓词判 Zn。
pub(super) fn is_zn(u: &UnitRange, seed_first: &UnitRange) -> bool {
    u.direction == seed_first.direction
}

/// 中枢成员里 **Zn** 的四个极值（★#1419 S1；`020-第20课.md:52`【正文】「GG=max(gn),G=min(gn),
/// D=max(dn),DD=min(dn)，n遍历中枢中所有Zn」）。
///
/// 全仓唯一一份 Zn 聚合：seed 外缘 [`compute_gg`]/[`compute_dd`]、共用扫描的延伸外缘
/// （`recursive_tower::scan_center_facts`）、S 事实层的六变量（[`CenterRoles`]）都只经 [`zn_extrema`] 取值。
/// ≥9 成员窗口的子窗外缘仍取全部单元，是 `zhongshu.md` §9.5 登记的过渡态（退场条件：#1429 落上级 Zn）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ZnExtrema {
    /// `DD = min(dn)`：外缘下沿。
    pub dd: Tick,
    /// `GG = max(gn)`：外缘上沿。
    pub gg: Tick,
    /// `D = max(dn)`：Zn 低点的最高者。不替代核心 ZD。
    pub d: Tick,
    /// `G = min(gn)`：Zn 高点的最低者。不替代核心 ZG；允许 `D > G`。
    pub g: Tick,
}

/// 聚合 `members` 中 Zn 的 [`ZnExtrema`]。`members[0]` 是 seed 第一单元（恒为 Zn），其余成员经 [`is_zn`]
/// 判。`members` 为空时 panic（中枢至少有 seed 三单元）。
pub fn zn_extrema(members: &[UnitRange]) -> ZnExtrema {
    let first = members.first().expect("中枢成员非空（至少 seed 三单元）");
    members.iter().filter(|u| is_zn(u, first)).fold(
        ZnExtrema {
            dd: first.lo,
            gg: first.hi,
            d: first.lo,
            g: first.hi,
        },
        |acc, u| ZnExtrema {
            dd: acc.dd.min(u.lo),
            gg: acc.gg.max(u.hi),
            d: acc.d.max(u.lo),
            g: acc.g.min(u.hi),
        },
    )
}

/// 外缘上沿 `computeGG s1 s2 s3`（契约锚 `Origin.CenterConstruction.computeGG`）：seed 三段中 **Zn** 的
/// hi 取 max（★#1419 S2：`a` 恒为 Zn，`b`/`c` 经 [`is_zn`] 判；取值经 [`zn_extrema`]）。L0 端点相连交替时
/// Zn = {a, c}，中间段两端点分别是 a、c 的端点 ⟹ 与三段 max 恒等（性质测试 `center_scan_1419`）。
/// `pub(super)`：#95 CarriedOnly 投影（level_view V3）需按同一公式算外缘，禁止公式重抄漂移。
pub(super) fn compute_gg(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Tick {
    zn_extrema(&[*a, *b, *c]).gg
}

/// 外缘下沿 `computeDD s1 s2 s3`（契约锚 `Origin.CenterConstruction.computeDD`）：seed 三段中 **Zn** 的
/// lo 取 min（★#1419 S2，同 `compute_gg`）。`pub(super)`：同 `compute_gg`（#95）。
pub(super) fn compute_dd(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Tick {
    zn_extrema(&[*a, *b, *c]).dd
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
///
/// ★**严格版对照（#328，2026-07-27）**：上面两处 `ZD_B≤ZG_B` 是 637号原等价式的**弱**口径版本，
/// 与 `center_from_segments` 的 #321 从严口径 `ZD_B<ZG_B` **不同一条命题**。严格版是条件等价——
/// 在前提「每段 `lo<hi`」下 `ZD_B<ZG_B` ⟺ 前两段核心**严格**非空 ∧ 第三段**严格**贯穿
/// （`c.lo<zg2 ∧ zd2<c.hi`），证明与前提必要性反例见模块 doc"严格版等价式重述"节。
///
/// 本函数**实装不动**（`c.lo <= zg2 && zd2 <= c.hi`，闭区间）：它是 A 口径 legacy 的**弱**贯穿
/// 见证，零生产调用点。与严格贯穿的分叉只在相切处（`a=b=[0,10]`、`c=[10,20]`：本函数真、严格贯穿
/// 假、`ZD_B=ZG_B` ⟹ `center_from_segments` 拒绝）——见 `third_spans_core_weak_vs_strict_diverge`
/// 测试见证。故本函数**不可**当作 `center_from_segments` 接受性的充分条件使用。
pub fn third_spans_core(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> bool {
    // 前两段核心（A 口径，仅本派生谓词用——查第三段相对前两段核心的位置）。
    let zg2 = a.hi.min(b.hi);
    let zd2 = a.lo.max(b.lo);
    c.lo <= zg2 && zd2 <= c.hi
}

/// seed 核心序三叶（★#1419 S1，A-CC-016-01/02；oracle `core_order`）：`ZD<ZG` / `ZD=ZG` / `ZD>ZG` 恰落其一。
/// strict 位 ⟺ `ZdLtZg`（#321：单点核心不成立）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoreOrder {
    /// `ZD < ZG`：核心严格非空。
    ZdLtZg,
    /// `ZD = ZG`：相切，单点核心不成立（#321 裁定）。
    ZdEqZg,
    /// `ZD > ZG`：三段无共同重叠。
    ZdGtZg,
}

impl CoreOrder {
    /// 由核心两端定三叶。
    pub fn of(zd: Tick, zg: Tick) -> Self {
        match zd.cmp(&zg) {
            std::cmp::Ordering::Less => CoreOrder::ZdLtZg,
            std::cmp::Ordering::Equal => CoreOrder::ZdEqZg,
            std::cmp::Ordering::Greater => CoreOrder::ZdGtZg,
        }
    }

    /// oracle / payload 字面量。
    pub fn as_str(self) -> &'static str {
        match self {
            CoreOrder::ZdLtZg => "ZD<ZG",
            CoreOrder::ZdEqZg => "ZD=ZG",
            CoreOrder::ZdGtZg => "ZD>ZG",
        }
    }
}

/// seed 判据口径。两者是同一份 [`SeedFacts`] 的两种投影：L0 完整判据查方向交替与 strict，上级几何判据
/// 只查 strict（上级单元方向取自 Move 趋势裁决，不承担 §6.1 的交替维度）。Z-3 判二者在 L0 交替输入上
/// 恒等，锁见 `center_segments_window_identity_on_l0_domain`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeedRule {
    /// L0 完整判据（[`center_from_segments`]）：方向交替 ∧ `ZD<ZG`。
    Complete,
    /// 上级几何判据（[`center_from_window`]）：`ZD<ZG`。
    Geometric,
}

/// seed 不成立的排除原因（★#1419 S1；`TANGENT_ZD_EQ_ZG`/`INVERTED_ZD_GT_ZG` 为 S0 oracle 提议名，按根裁定 r2
/// R1 采纳、并入 D-J schema 独审定名）。先查方向交替，再查核心序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeedExclusion {
    /// 完整判据下三单元方向不交替（L0 端点相连的段恒交替，正常输入不出现；S 写者据此 fail-closed）。
    NotAlternating,
    /// `ZD = ZG`。
    TangentZdEqZg,
    /// `ZD > ZG`。
    InvertedZdGtZg,
}

impl SeedExclusion {
    /// payload 字面量。
    pub fn as_str(self) -> &'static str {
        match self {
            SeedExclusion::NotAlternating => "NOT_ALTERNATING",
            SeedExclusion::TangentZdEqZg => "TANGENT_ZD_EQ_ZG",
            SeedExclusion::InvertedZdGtZg => "INVERTED_ZD_GT_ZG",
        }
    }
}

/// seed 五条件里由输入方给出的三位（★#1419 S1）：三单元是否都已完成、是否首尾相连、是否同一级别。
///
/// 这三位是输入事实，不是几何判定：生产塔的 L0 单元是同一层已确认线段，首尾相连、同级、已完成，
/// 由构造保证（[`SeedInput::TOWER`]）；S 写者从已发布的 `CC-013.segment` 对象逐位核实，核不上即
/// fail-closed，不进入扫描。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SeedInput {
    /// 三单元都已完成。
    pub completed: bool,
    /// 三单元首尾相连（前一单元终点 = 后一单元起点）。
    pub consecutive: bool,
    /// 三单元同一级别。
    pub same_level: bool,
}

impl SeedInput {
    /// 生产塔 L0 单元的输入三位（由构造保证全真）。
    pub const TOWER: SeedInput = SeedInput {
        completed: true,
        consecutive: true,
        same_level: true,
    };
}

/// seed 五条件向量 Bool⁵（★#1419 S1，A-CC-016-01/02；顺序 = oracle `seed_vector_order`）。
///
/// 各位独立取值，不 one-hot。三单元未全完成时 strict 位不评估，记 `None`（根裁定 r2 R5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SeedConditionVector {
    pub completed: bool,
    pub consecutive: bool,
    pub same_level: bool,
    pub alternating: bool,
    /// `ZD<ZG`；三单元未全完成时为 `None`（未评估）。
    pub strict: Option<bool>,
}

impl SeedConditionVector {
    /// 五位全真（L0 正式中枢的充要条件，「11111 才成中枢」）。
    pub fn all_true(&self) -> bool {
        self.completed
            && self.consecutive
            && self.same_level
            && self.alternating
            && self.strict == Some(true)
    }
}

/// seed 判定的全部几何事实（★#1419 S1，PLAN S1 / PREP C4）。全仓唯一一份 seed 判定：
/// [`center_from_segments`] 与 [`center_from_window`] 都是 [`SeedFacts::center`] 的投影，S 事实层发布的
/// seed 尝试（CC-016）读同一结构。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SeedFacts {
    /// 方向交替 `DirAlternates`。
    pub alternating: bool,
    /// `ZD = max(三低)`。
    pub zd: Tick,
    /// `ZG = min(三高)`。
    pub zg: Tick,
    /// 核心序三叶。
    pub core_order: CoreOrder,
    /// seed 三单元里 Zn 的四个极值。
    pub zn: ZnExtrema,
    /// 首单元起点。
    pub start_index: usize,
    /// 末单元终点。
    pub end_index: usize,
}

/// 计算三单元 seed 的 [`SeedFacts`]（不判成立与否，成立由 [`SeedFacts::center`] 按口径投影）。
pub fn seed_facts(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> SeedFacts {
    let zd = compute_zd(a, b, c);
    let zg = compute_zg(a, b, c);
    SeedFacts {
        alternating: dir_alternates(a, b, c),
        zd,
        zg,
        core_order: CoreOrder::of(zd, zg),
        zn: zn_extrema(&[*a, *b, *c]),
        start_index: a.start_index,
        end_index: c.end_index,
    }
}

impl SeedFacts {
    /// strict 位：`ZD < ZG`。
    pub fn strict(&self) -> bool {
        self.core_order == CoreOrder::ZdLtZg
    }

    /// 按口径给出排除原因；`None` ⟺ seed 成立。
    pub fn exclusion(&self, rule: SeedRule) -> Option<SeedExclusion> {
        if rule == SeedRule::Complete && !self.alternating {
            return Some(SeedExclusion::NotAlternating);
        }
        match self.core_order {
            CoreOrder::ZdLtZg => None,
            CoreOrder::ZdEqZg => Some(SeedExclusion::TangentZdEqZg),
            CoreOrder::ZdGtZg => Some(SeedExclusion::InvertedZdGtZg),
        }
    }

    /// 按口径投影为中枢：成立 ⟹ 核心 `[ZD,ZG]`、外缘取 seed 三单元的 Zn 极值；不成立 ⟹ `None`。
    pub fn center(&self, rule: SeedRule) -> Option<Center> {
        if self.exclusion(rule).is_some() {
            return None;
        }
        Some(Center {
            zd: self.zd,
            zg: self.zg,
            dd: self.zn.dd,
            gg: self.zn.gg,
            start_index: self.start_index,
            end_index: self.end_index,
        })
    }

    /// 组 Bool⁵：输入三位取自 `input`，交替与 strict 取自本事实；三单元未全完成时 strict 记 `None`。
    pub fn condition_vector(&self, input: SeedInput) -> SeedConditionVector {
        SeedConditionVector {
            completed: input.completed,
            consecutive: input.consecutive,
            same_level: input.same_level,
            alternating: self.alternating,
            strict: input.completed.then(|| self.strict()),
        }
    }
}

/// 中枢角色变量（★#1419 S1，A-CC-017-01/02；oracle `roles`）：CORE 的 ZD/ZG 冻结，OUTER 的 DD/GG 与
/// INNER_EXTREMA 的 D/G 随 Zn 成员更新。六个变量各自成立，数值相等也不合并。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoleVar {
    Zd,
    Zg,
    Dd,
    Gg,
    D,
    G,
}

impl RoleVar {
    /// 发布顺序：CORE(ZD,ZG) → OUTER(DD,GG) → INNER_EXTREMA(D,G)。
    pub const ALL: [RoleVar; 6] = [
        RoleVar::Zd,
        RoleVar::Zg,
        RoleVar::Dd,
        RoleVar::Gg,
        RoleVar::D,
        RoleVar::G,
    ];

    /// payload 字面量。
    pub fn as_str(self) -> &'static str {
        match self {
            RoleVar::Zd => "ZD",
            RoleVar::Zg => "ZG",
            RoleVar::Dd => "DD",
            RoleVar::Gg => "GG",
            RoleVar::D => "D",
            RoleVar::G => "G",
        }
    }

    /// 所属角色：`CORE` / `OUTER` / `INNER_EXTREMA`。
    pub fn role(self) -> &'static str {
        match self {
            RoleVar::Zd | RoleVar::Zg => "CORE",
            RoleVar::Dd | RoleVar::Gg => "OUTER",
            RoleVar::D | RoleVar::G => "INNER_EXTREMA",
        }
    }

    /// 是否冻结（只有核心冻结，#819）。
    pub fn frozen(self) -> bool {
        matches!(self, RoleVar::Zd | RoleVar::Zg)
    }
}

/// 中枢六变量（★#1419 S1，A-CC-017-01）：核心取 seed 三单元，四个动态极值取全部成员中的 Zn。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CenterRoles {
    pub zd: Tick,
    pub zg: Tick,
    pub zn: ZnExtrema,
}

impl CenterRoles {
    /// `members` = seed 三单元 + 延伸成员（`len >= 3`，否则 panic）。
    pub fn of_members(members: &[UnitRange]) -> Self {
        assert!(members.len() >= 3, "中枢成员至少 seed 三单元");
        CenterRoles {
            zd: compute_zd(&members[0], &members[1], &members[2]),
            zg: compute_zg(&members[0], &members[1], &members[2]),
            zn: zn_extrema(members),
        }
    }

    /// 取某一变量的值。
    pub fn value(&self, v: RoleVar) -> Tick {
        match v {
            RoleVar::Zd => self.zd,
            RoleVar::Zg => self.zg,
            RoleVar::Dd => self.zn.dd,
            RoleVar::Gg => self.zn.gg,
            RoleVar::D => self.zn.d,
            RoleVar::G => self.zn.g,
        }
    }

    /// 取到该极值的成员（`members` 内相对下标，升序）。ZD/ZG 只在 seed 三单元内找，DD/GG/D/G 只在 Zn
    /// 内找；两个及以上即同价见证。
    pub fn contributors(&self, members: &[UnitRange], v: RoleVar) -> Vec<usize> {
        let target = self.value(v);
        let first = &members[0];
        members
            .iter()
            .enumerate()
            .filter(|(k, u)| match v {
                RoleVar::Zd => *k < 3 && u.lo == target,
                RoleVar::Zg => *k < 3 && u.hi == target,
                RoleVar::Dd | RoleVar::D => is_zn(u, first) && u.lo == target,
                RoleVar::Gg | RoleVar::G => is_zn(u, first) && u.hi == target,
            })
            .map(|(k, _)| k)
            .collect()
    }
}

/// 突破段的离开方向（契约锚 Lean `breakDirectionUp`，`formal/Origin/CenterConstruct.lean`）：整体在核心上方
/// ⟹ 向上；整体在核心下方 ⟹ 向下；仍与核心相交时（非突破段，扫描不会传入）按高点是否越过 ZG 定。
pub fn breaks_upward(breaker: &UnitRange, zd: Tick, zg: Tick) -> bool {
    if breaker.lo > zg {
        true
    } else if breaker.hi < zd {
        false
    } else {
        breaker.hi > zg
    }
}

/// 从连续三段次级别走势单元构造**完整中枢**（契约锚 `Origin.CenterComplete.CenterConfirmedComplete`
/// + `Origin.CenterConstruction.centerFromThree`）。
///
/// 真中枢 ⟺ 两条**全部**成立（口径 B，637号；对齐 `CenterConfirmedComplete` 二支合取）：
/// 1. **方向交替** `dir_alternates`（§6.1，`DirAlternates`）。
/// 2. **全三段核心非空** `compute_zd(a,b,c) < compute_zg(a,b,c)`（**严格**，#321 裁定 2026-07-26；
///    §6.3/§6.4 口径 B：核心 = 前三段重叠部分 `[max(三段低),min(三段高)]` 严格非空 ⟺ 三段有共同
///    重叠区间且非单点）。
///
/// ★第三段贯穿已吸收进支2（637号 codex L0 等价）：B 口径核心非空恒等价于 A 口径
/// [前两段核心非空 ∧ 第三段贯穿]，故不再单列第三段判据支（声明膨胀禁止）。
///
/// 任一支不成立 ⟹ 返回 `None`（非中枢，不静默造退化中枢）。成立 ⟹ 核心取**全三段**
/// `[compute_zd(a,b,c), compute_zg(a,b,c)]`，外缘取 seed 三段中 **Zn** 的 dd/gg（★#1419 S2）。
///
/// 边界条件（结论翻转）：方向交替用严格异向；全三段核心非空用**严格**开口径（`ZD_B<ZG_B` 才成立，
/// `ZD_B==ZG_B` 单点核心**不**成立）——#321 裁定（2026-07-26 用户裁决）：原弱口径（闭区间含单点）
/// 为实现者自选、无教义依据，原文（17 课定义、20 课公式）未涉及端点口径，22 课 Q&A
/// （`docs/chanlun/text/blog/022-第22课.md:514`）单点中枢之问，缠师答的是级别谬误（只说明该例子
/// 只构成 1 分钟中枢的延续，未答单点边界本身是否成立），故统一向 #290 裁定 B
/// （Python 严格口径）看齐，三实现（Lean/Python/Rust）向严格统一。
///
/// ⚠边界声明作废：中枢**成立**落点 ZD==ZG —— Lean `centerHolds` 为 `ZD ≤ ZG`（弱，单点成立）/
/// Rust 本函数严格（不成立，#321 裁定）；**该落点的 Lean↔Rust 对齐声明在本边界上作废，此边界不作
/// 机械锁用**（不得据本实装断言 Lean 侧行为，亦不得据 Lean `centerHolds` 反推本实装期望值）；
/// 其余落点对齐声明不受影响。Lean 侧跟进留对方线。
///
/// 缺方向维度（上级无方向单元）则方向交替不可判——见 `center_from_window`。
pub fn center_from_segments(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Option<Center> {
    // ★#1419 S1：[`seed_facts`] 的 `Complete` 投影——支1 方向交替（§6.1，DirAlternates），支2 全三段核心
    // 严格非空（口径 B，§6.3/§6.4，已含第三段贯穿，637号；#321 单点核心不成立）。核心 = 全三段 [zd,zg]；
    // 外缘 = seed 三段中 Zn 的 dd/gg（#1419 S2）。
    seed_facts(a, b, c).center(SeedRule::Complete)
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
/// 三段有共同重叠，第三段贯穿已吸收进核心非空）；外缘 = seed 三段中 Zn 的 dd/gg（★#1419 S2：上级
/// 单元方向取自 Move 趋势裁决，不保证交替，Zn 按 [`is_zn`] 判方向）。核心非空须**严格** `ZD_B<ZG_B`
/// ——核心空或单点（`ZD_B>=ZG_B`）⟹ `None`（#321 裁定，2026-07-26 用户裁决：单点核心不成立，与
/// #290 裁定 B / Python 严格口径三方对齐；原弱口径为实现者自选、原文未涉及端点口径）。
///
/// ⚠边界声明作废：中枢**成立**落点 ZD==ZG —— Lean `centerHolds` 为 `ZD ≤ ZG`（弱，单点成立）/
/// Rust 本函数严格（不成立，#321 裁定）；**该落点的 Lean↔Rust 对齐声明在本边界上作废，此边界不作
/// 机械锁用**（不得据本实装断言 Lean 侧行为，亦不得据 Lean `centerHolds` 反推本实装期望值）；
/// 其余落点对齐声明不受影响。Lean 侧跟进留对方线。
pub fn center_from_window(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> Option<Center> {
    // ★#1419 S1：[`seed_facts`] 的 `Geometric` 投影——全三段核心严格非空（口径 B，已含第三段贯穿，637号；
    // #321 单点核心不成立），不查方向交替。
    seed_facts(a, b, c).center(SeedRule::Geometric)
}

/// 判定两中枢关系（契约锚 `Origin.CenterStates.classifyDevelopment`：新生/扩展/延伸三态
/// + `IsUpTrend`/`IsDownTrend` 外缘趋势判据；扩展支写全 = #815 M-2 裁定 + #898 落地）。
///
/// 中心定理二（`020-第20课.md:58`【正文】，#815 M-2 已核逐字）：
/// - `next.dd > prev.gg → 上涨延续`（`Origin.CenterStates.IsUpTrend`，外缘完全分离向上）；
/// - `next.gg < prev.dd → 下跌延续`（`IsDownTrend`，外缘完全分离向下）；
/// - **核心分离 ∧ 外缘重叠 → 级别扩张**：写全为原公式 `(next.zg<prev.zd ∧ next.gg>=prev.dd)
///   ∨ (next.zd>prev.zg ∧ next.dd<=prev.gg)`，不再由 `¬up ∧ ¬down` 兜底（在外缘重叠前件下，
///   核心分离 ⟺ 该合取式——Lean `CenterTrichotomy.trichotomy_predicates_total` 已机器证明）；
/// - 核心重叠 → `CoreOverlap`（中枢延伸，中心定理一，`classifyDevelopment` 的 `extension`
///   支）——探针 #894 P-2 实测该情形占相邻中枢对约 27%，旧 else 兜底把它静默吞进扩展。
pub fn classify_relation(prev: &Center, next: &Center) -> CenterRelation {
    if next.dd > prev.gg {
        CenterRelation::UpContinuation
    } else if next.gg < prev.dd {
        CenterRelation::DownContinuation
    } else if (next.zg < prev.zd && next.gg >= prev.dd) || (next.zd > prev.zg && next.dd <= prev.gg)
    {
        CenterRelation::LevelExpansion
    } else {
        CenterRelation::CoreOverlap
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
        UnitRange {
            start_index: si,
            end_index: ei,
            direction: dir,
            lo,
            hi,
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    //  G4 完整判据 center_from_segments（契约锚 Origin.CenterComplete.CenterConfirmedComplete）
    // ──────────────────────────────────────────────────────────────────────

    /// ★完整判据正例（契约锚 `Origin.CenterComplete.trueCenter_centerConfirmed`，口径 B）：上-下-上，
    /// 全三段核心非空 ⟹ 真中枢。核心取**全三段** [compute_zd, compute_zg]。
    fn up() -> Direction {
        Direction::Up
    }
    fn down() -> Direction {
        Direction::Down
    }

    #[test]
    fn complete_center_confirmed_bit_exact() {
        // 上-下-上：a=[10,20]up, b=[12,20]down, c=[12,22]up（对齐 Origin trueCenterSeg*）。
        // 核心 = 全三段（口径 B）：zd=max(10,12,12)=12, zg=min(20,20,22)=20。第三段 [12,22] 贯穿前两段
        // 核心 [12,20] ⟹ 全三段核心 = 前两段核心（此 case A/B 重合）。外缘 = 三段：dd=10, gg=22。
        let a = unit(0, 4, up(), 10, 20);
        let b = unit(4, 8, down(), 12, 20);
        let c = unit(8, 12, up(), 12, 22);
        let center = center_from_segments(&a, &b, &c).expect("完整判据两支全成立 ⟹ 真中枢");
        assert_eq!(
            (center.zd, center.zg, center.dd, center.gg),
            (12, 20, 10, 22)
        );
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
        assert_eq!(
            (center.zd, center.zg, center.dd, center.gg),
            (14, 18, 10, 20)
        );
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
        assert_eq!(
            center_from_segments(&a, &b, &c),
            None,
            "无方向交替 ⟹ 完整判据拒绝"
        );
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
        assert!(
            !third_spans_core(&a, &b, &c),
            "第三段不贯穿前两段核心（派生谓词）"
        );
        assert_eq!(
            center_from_segments(&a, &b, &c),
            None,
            "全三段核心空（第三段不贯穿）⟹ 完整判据拒绝"
        );
    }

    /// ★#328 严格版等价式见证：弱（闭区间）贯穿谓词 ≠ 严格贯穿，且「每段 `lo<hi`」前提不可省。
    ///
    /// 见模块 doc"严格版等价式重述"节：`ZD_B<ZG_B` ⟺ 前两段核心严格非空 ∧ 第三段严格贯穿，
    /// **前提**每段 `lo<hi`（`UnitRange` 只保证 `lo<=hi`，不足）。本测试钉住两处分叉。
    #[test]
    fn third_spans_core_weak_vs_strict_diverge() {
        // (1) 相切分叉：a=b=[0,10]、c=[10,20]——闭区间谓词真，但 ZD_B==ZG_B ⟹ #321 严格口径拒绝。
        let a = unit(0, 4, up(), 0, 10);
        let b = unit(4, 8, down(), 0, 10);
        let c = unit(8, 12, up(), 10, 20);
        assert!(third_spans_core(&a, &b, &c), "闭区间（弱）贯穿：相切算贯穿");
        assert_eq!(
            compute_zd(&a, &b, &c),
            compute_zg(&a, &b, &c),
            "ZD_B==ZG_B=10（单点核心）"
        );
        assert_eq!(
            center_from_segments(&a, &b, &c),
            None,
            "严格口径拒绝单点核心 ⟹ 弱谓词非充分条件"
        );

        // (2) 前提反例：c=[5,5] 退化段（满足 lo<=hi、不满足 lo<hi）——严格贯穿右边全真但左边假。
        let c_deg = unit(8, 12, up(), 5, 5);
        let (zd2, zg2) = (a.lo.max(b.lo), a.hi.min(b.hi));
        assert!(zd2 < zg2, "前两段核心严格非空");
        assert!(c_deg.lo < zg2 && zd2 < c_deg.hi, "第三段严格贯穿前两段核心");
        assert_eq!(
            compute_zd(&a, &b, &c_deg),
            compute_zg(&a, &b, &c_deg),
            "但退化段令 ZD_B==ZG_B=5"
        );
        assert_eq!(
            center_from_segments(&a, &b, &c_deg),
            None,
            "⟹ 等价式必须显式带「每段 lo<hi」前提"
        );
    }

    /// ★完整判据拒绝核心空（前两段无重叠）：zd>zg ⟹ 拒绝（即使方向交替）。
    #[test]
    fn complete_rejects_empty_core() {
        // 上-下-上但前两段无重叠：a=[0,4]up, b=[10,14]down, c=[5,9]up。
        // 核心 zd=max(0,10)=10 > zg=min(4,14)=4 ⟹ 核心空。
        let a = unit(0, 4, up(), 0, 4);
        let b = unit(4, 8, down(), 10, 14);
        let c = unit(8, 12, up(), 5, 9);
        assert_eq!(
            center_from_segments(&a, &b, &c),
            None,
            "前两段核心空 ⟹ 拒绝"
        );
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
        assert!(
            center_from_window(&a, &b, &c).is_some(),
            "几何路径接受同向（不查方向）"
        );
        // 完整判据拒绝（无方向交替）。
        assert_eq!(
            center_from_segments(&a, &b, &c),
            None,
            "完整判据拒绝同向（查方向交替）"
        );
    }

    /// #1294 对拍锁 3：#804/#812 Z-3 恒等性锁——`center_from_segments ≡ center_from_window`
    /// 在 L0 生产输入域（方向交替，`dir_alternates` 真）上**逐位恒等**。
    ///
    /// #812 Z-3 已裁 E1 伪分歧：`dir_alternates` 在 L0 生产输入上恒真（L0 段有内在方向），
    /// 故两函数可达输入域恒等、输出恒等——AGENTS.md 在案实例撤销、例外配额回「用 0 剩 3」。
    /// 本锁是该裁定的随票锁（#1278 批三 ⑧列收口）：同输入同输出对拍，防两函数在 L0 域上
    /// 再分叉（任何一处加宽/收窄判据即红）。
    ///
    /// 历史分歧案例（Z-3 来源，非举例）：同向三段（无方向交替）时 `center_from_segments`
    /// 拒绝、`center_from_window` 接受——两函数**并非无条件**恒等；本锁先钉住这条分歧臂
    /// （证明锁非 vacuous），再枚举 L0 交替方向域逐位对拍。
    #[test]
    fn center_segments_window_identity_on_l0_domain() {
        // ── (a) 历史分歧案例（Z-3 来源）：同向三段 → 两函数结论分叉。──
        // 同向（全 up）且核心非空：segments 拒绝（无方向交替）、window 接受（不查方向）。
        let a = unit(0, 4, up(), 10, 20);
        let b = unit(4, 8, up(), 18, 25);
        let c = unit(8, 12, up(), 18, 22);
        assert!(!dir_alternates(&a, &b, &c), "同向三段 = L0 域外");
        assert_eq!(
            center_from_segments(&a, &b, &c),
            None,
            "域外（同向）：完整判据拒绝"
        );
        assert!(
            center_from_window(&a, &b, &c).is_some(),
            "域外（同向）：几何路径接受 ⟹ 两函数非无条件恒等（锁非 vacuous）"
        );

        // ── (b) L0 生产输入域（方向交替）逐位对拍：两函数输出必须逐位相同。──
        // 方向交替只有两种三元组形态（二元方向下相邻异向）：上-下-上 / 下-上-下。
        const DIRS: [[Direction; 3]; 2] = [
            [Direction::Up, Direction::Down, Direction::Up],
            [Direction::Down, Direction::Up, Direction::Down],
        ];
        // 区间端点网格：覆盖核心非空 / 核心空 / 单点核心（zd==zg）三类落点。
        const LOS: [Tick; 4] = [0, 5, 10, 20];
        const HIS: [Tick; 4] = [5, 10, 20, 25];
        for dirs in DIRS {
            for &a_lo in &LOS {
                for &a_hi in &HIS {
                    if a_lo > a_hi {
                        continue;
                    }
                    for &b_lo in &LOS {
                        for &b_hi in &HIS {
                            if b_lo > b_hi {
                                continue;
                            }
                            for &c_lo in &LOS {
                                for &c_hi in &HIS {
                                    if c_lo > c_hi {
                                        continue;
                                    }
                                    let a = unit(0, 4, dirs[0], a_lo, a_hi);
                                    let b = unit(4, 8, dirs[1], b_lo, b_hi);
                                    let c = unit(8, 12, dirs[2], c_lo, c_hi);
                                    assert!(
                                        dir_alternates(&a, &b, &c),
                                        "交替方向三元组必在 L0 域内"
                                    );
                                    assert_eq!(
                                        center_from_segments(&a, &b, &c),
                                        center_from_window(&a, &b, &c),
                                        "L0 域 dir={dirs:?} a=[{a_lo},{a_hi}] b=[{b_lo},{b_hi}] c=[{c_lo},{c_hi}] 两函数必须逐位恒等"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// ★#321 裁定新增（先红后绿）：完整判据 `center_from_segments` 单点核心 `ZD==ZG` **不成立**。
    ///
    /// 旧口径（本文件迁移前）：闭区间核心非空含单点 `zd==zg`，判**成立**（返回 `Some`）。
    /// #321 裁定后（2026-07-26 用户裁决，与 #290 裁定 B / Python 严格口径三方对齐）：支2
    /// 全三段核心非空须**严格** `zd<zg`，`zd==zg` 单点核心判**不成立**（返回 `None`）。理由：
    /// 原文（17 课定义、20 课公式）未涉及端点口径，22 课 Q&A（`022-第22课.md:514`）单点中枢
    /// 之问，缠师答的是级别谬误（未答单点之问，只说明该例子只构成 1 分钟中枢的延续）；原弱口径
    /// 为实现者自选、无教义依据，统一向 #290 裁定 B 严格口径看齐。
    /// 用例直接复用 `geometric_window_rejects_boundary_zd_eq_zg` 的同一组单元（up-down-up
    /// 恰好方向交替，支1 天然成立）——让支2（核心非空判据）成为本用例唯一判据来源。
    #[test]
    fn complete_center_rejects_boundary_zd_eq_zg() {
        // a=[0,5]up, b=[5,10]down, c=[3,5]up：方向交替（支1 成立）；
        // zd=max(0,5,3)=5, zg=min(5,10,5)=5 ⟹ 单点核心 [5,5]（支2 zd==zg）。
        let a = unit(0, 4, up(), 0, 5);
        let b = unit(4, 8, down(), 5, 10);
        let c = unit(8, 12, up(), 3, 5);
        assert!(
            dir_alternates(&a, &b, &c),
            "支1（方向交替）预先成立，排除干扰"
        );
        assert_eq!(compute_zd(&a, &b, &c), 5);
        assert_eq!(compute_zg(&a, &b, &c), 5);
        assert!(
            center_from_segments(&a, &b, &c).is_none(),
            "#321 裁定：单点核心 zd==zg ⟹ 完整判据不成立（从严，旧口径判成立已作废）"
        );
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
        assert_eq!(
            center_from_window(&a, &b, &c),
            None,
            "全三段核心空（第三段不贯穿）⟹ 非中枢"
        );
    }

    /// ★本用例固化的是旧弱口径（zd==zg 单点核心闭区间合法 ⟹ 成立）；#321 裁定后期望翻转：
    /// 单点核心不成立，返回 `None`（2026-07-26 用户裁决，与 #290 裁定 B / Python 严格口径三方
    /// 对齐；原弱口径为实现者自选、原文未涉及端点口径，见 `022-第22课.md:514`）。原断言
    /// （`center_from_window(...).expect(...)` 判 `Some((5,5))`）已作废，不得据旧读数反推。
    #[test]
    fn geometric_window_rejects_boundary_zd_eq_zg() {
        // a=[0,5], b=[5,10], c=[3,5]：zd=max(0,5,3)=5, zg=min(5,10,5)=5 ⟹ 单点核心 [5,5]。
        // #321 从严：zd==zg 不再判核心非空 ⟹ center_from_window 返回 None。
        let a = unit(0, 4, up(), 0, 5);
        let b = unit(4, 8, down(), 5, 10);
        let c = unit(8, 12, up(), 3, 5);
        assert_eq!(compute_zd(&a, &b, &c), 5);
        assert_eq!(compute_zg(&a, &b, &c), 5);
        assert!(
            center_from_window(&a, &b, &c).is_none(),
            "#321 裁定：单点核心 zd==zg ⟹ 几何路径不成立（从严，旧口径判成立已作废）"
        );
    }

    fn center(dd: Tick, zd: Tick, zg: Tick, gg: Tick) -> Center {
        Center {
            zd,
            zg,
            dd,
            gg,
            start_index: 0,
            end_index: 0,
        }
    }

    #[test]
    fn relation_up_continuation_bit_exact() {
        // prev 外缘[0,8]，next 外缘[9,15]：next.dd=9 > prev.gg=8 ⟹ up（chain2_up 同构）。
        let prev = center(0, 2, 5, 8);
        let next = center(9, 10, 13, 15);
        assert_eq!(
            classify_relation(&prev, &next),
            CenterRelation::UpContinuation
        );
    }

    #[test]
    fn relation_down_continuation_bit_exact() {
        // prev 外缘[9,15]，next 外缘[0,8]：next.gg=8 < prev.dd=9 ⟹ down。
        let prev = center(9, 10, 13, 15);
        let next = center(0, 2, 5, 8);
        assert_eq!(
            classify_relation(&prev, &next),
            CenterRelation::DownContinuation
        );
    }

    #[test]
    fn relation_level_expansion_bit_exact() {
        // codex 见证 prev=(0,2,5,8), next=(4,6,9,11)：核心分离 + 外缘重叠 ⟹ expansion。
        let prev = center(0, 2, 5, 8);
        let next = center(4, 6, 9, 11);
        assert_eq!(
            classify_relation(&prev, &next),
            CenterRelation::LevelExpansion
        );
    }

    #[test]
    fn relation_level_expansion_down_bit_exact() {
        // 扩展支下行侧：prev=(4,6,9,11), next=(0,2,5,8)——next.zg=5 < prev.zd=6（核心分离向下）
        // 且 next.gg=8 >= prev.dd=4（外缘重叠）⟹ 中心定理二原公式第二析取支。
        let prev = center(4, 6, 9, 11);
        let next = center(0, 2, 5, 8);
        assert_eq!(
            classify_relation(&prev, &next),
            CenterRelation::LevelExpansion
        );
    }

    #[test]
    fn relation_core_overlap_is_extension_not_expansion() {
        // ★#898：核心重叠（中枢延伸，中心定理一）不再是扩展——prev=(0,2,5,8), next=(1,3,7,9)：
        // 核心 [2,5] 与 [3,7] 重叠（next.zd=3 <= prev.zg=5 且 next.zg=7 >= prev.zd=2），
        // 外缘 [0,8] 与 [1,9] 亦重叠 ⟹ CoreOverlap（约 27% 相邻中枢对，探针 #894 P-2）。
        let prev = center(0, 2, 5, 8);
        let next = center(1, 3, 7, 9);
        assert_eq!(classify_relation(&prev, &next), CenterRelation::CoreOverlap);
    }

    #[test]
    fn relation_core_touching_boundary_is_core_overlap() {
        // 核心端点相贴（next.zd == prev.zg，闭核心区间共享单点）不构成分离
        // （Lean `SameLevelNewCenterPair` 严格不等式）⟹ CoreOverlap，非扩展。
        let prev = center(0, 2, 5, 8);
        let next = center(1, 5, 8, 10);
        assert_eq!(classify_relation(&prev, &next), CenterRelation::CoreOverlap);
    }

    #[test]
    fn relation_outer_separation_beats_core_overlap() {
        // 外缘分离优先于核心判据：next 整体在 prev 之上（dd>gg）⟹ 上涨延续，
        // 即使核心自然也分离——延续支与扩展支互斥（Lean up_expansion_disjoint）。
        let prev = center(0, 2, 5, 8);
        let next = center(9, 10, 13, 15);
        assert!(classify_relation(&prev, &next).is_continuation());
        assert_eq!(
            classify_relation(&prev, &next).trend_direction(),
            Some(Direction::Up)
        );
        assert!(!CenterRelation::LevelExpansion.is_continuation());
        assert!(!CenterRelation::CoreOverlap.is_continuation());
        assert_eq!(CenterRelation::LevelExpansion.trend_direction(), None);
        assert_eq!(CenterRelation::CoreOverlap.trend_direction(), None);
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

    // ──────────────────────────────────────────────────────────────────────
    //  ★#1419 S1：seed 唯一判定 `seed_facts`（Bool⁵、核心序三叶、贡献成员、排除原因）
    // ──────────────────────────────────────────────────────────────────────

    /// 端点序列 → 相连单元（`u_k = P_k → P_{k+1}`，S0 oracle 编码）。
    fn connected(p: &[Tick]) -> Vec<UnitRange> {
        p.windows(2)
            .enumerate()
            .map(|(k, w)| {
                let dir = if w[1] > w[0] { up() } else { down() };
                unit(k, k + 1, dir, w[0].min(w[1]), w[0].max(w[1]))
            })
            .collect()
    }

    /// 一条 oracle seed 例的期待值：(Bool⁵, ZD, ZD 贡献, ZG, ZG 贡献, 三叶, 排除原因)。
    type SeedRow = (
        [bool; 5],
        Tick,
        &'static [usize],
        Tick,
        &'static [usize],
        &'static str,
        Option<&'static str>,
    );

    fn assert_seed_row(name: &str, endpoints: &[Tick], want: SeedRow) {
        let u = connected(endpoints);
        let f = seed_facts(&u[0], &u[1], &u[2]);
        let v = f.condition_vector(SeedInput::TOWER);
        assert_eq!(
            [
                v.completed,
                v.consecutive,
                v.same_level,
                v.alternating,
                v.strict == Some(true)
            ],
            want.0,
            "{name}: Bool⁵"
        );
        let roles = CenterRoles::of_members(&u[..3]);
        assert_eq!(
            (f.zd, roles.contributors(&u[..3], RoleVar::Zd).as_slice()),
            (want.1, want.2),
            "{name}: ZD"
        );
        assert_eq!(
            (f.zg, roles.contributors(&u[..3], RoleVar::Zg).as_slice()),
            (want.3, want.4),
            "{name}: ZG"
        );
        assert_eq!(f.core_order.as_str(), want.5, "{name}: 三叶");
        assert_eq!(
            f.exclusion(SeedRule::Complete).map(SeedExclusion::as_str),
            want.6,
            "{name}: 排除原因"
        );
        assert_eq!(
            center_from_segments(&u[0], &u[1], &u[2]).is_some(),
            want.6.is_none(),
            "{name}: 成立与否"
        );
        assert_eq!(v.all_true(), want.6.is_none(), "{name}: 11111 ⟺ 成立");
    }

    /// S0 oracle `seed_examples` E1a/E1b/E1b′/E1c（期待值取自 `worked-examples.json`，不从被测函数推出）。
    #[test]
    fn s1_seed_facts_match_oracle_e1_examples() {
        let t = [true; 5];
        let f4 = [true, true, true, true, false];
        assert_seed_row(
            "E1a",
            &[10, 40, 20, 35],
            (t, 20, &[1, 2], 35, &[2], "ZD<ZG", None),
        );
        assert_seed_row(
            "E1b",
            &[10, 40, 5, 10],
            (f4, 10, &[0], 10, &[2], "ZD=ZG", Some("TANGENT_ZD_EQ_ZG")),
        );
        assert_seed_row(
            "E1b_prime",
            &[10, 40, 5, 11],
            (t, 10, &[0], 11, &[2], "ZD<ZG", None),
        );
        assert_seed_row(
            "E1c",
            &[10, 40, 5, 9],
            (f4, 10, &[0], 9, &[2], "ZD>ZG", Some("INVERTED_ZD_GT_ZG")),
        );
    }

    /// E1e：第三单元未完成 ⟹ strict 位不评估（`None`，根裁定 r2 R5），五位不全真。
    #[test]
    fn s1_incomplete_input_leaves_strict_unevaluated() {
        let u = connected(&[10, 40, 20, 35]);
        let v = seed_facts(&u[0], &u[1], &u[2]).condition_vector(SeedInput {
            completed: false,
            ..SeedInput::TOWER
        });
        assert_eq!(v.strict, None);
        assert!(!v.completed && !v.all_true());
    }

    /// 定义式直算（不调被测函数）：(ZD, ZG, 交替, Zn 的 DD/GG/D/G)。
    fn brute(a: &UnitRange, b: &UnitRange, c: &UnitRange) -> (Tick, Tick, bool, [Tick; 4]) {
        let zd = *[a.lo, b.lo, c.lo].iter().max().unwrap();
        let zg = *[a.hi, b.hi, c.hi].iter().min().unwrap();
        let alt = a.direction != b.direction && b.direction != c.direction;
        let zn: Vec<&UnitRange> = [a, b, c]
            .into_iter()
            .filter(|u| u.direction == a.direction)
            .collect();
        let dd = zn.iter().map(|u| u.lo).min().unwrap();
        let gg = zn.iter().map(|u| u.hi).max().unwrap();
        let d = zn.iter().map(|u| u.lo).max().unwrap();
        let g = zn.iter().map(|u| u.hi).min().unwrap();
        (zd, zg, alt, [dd, gg, d, g])
    }

    fn assert_projection(a: &UnitRange, b: &UnitRange, c: &UnitRange) {
        let (zd, zg, alt, [dd, gg, d, g]) = brute(a, b, c);
        let f = seed_facts(a, b, c);
        let ctx = format!("{a:?} {b:?} {c:?}");
        assert_eq!((f.zd, f.zg, f.alternating), (zd, zg, alt), "{ctx}");
        assert_eq!((f.zn.dd, f.zn.gg, f.zn.d, f.zn.g), (dd, gg, d, g), "{ctx}");
        let leaf = if zd < zg {
            CoreOrder::ZdLtZg
        } else if zd == zg {
            CoreOrder::ZdEqZg
        } else {
            CoreOrder::ZdGtZg
        };
        assert_eq!(f.core_order, leaf, "{ctx}");
        let want = |ok: bool| {
            ok.then_some(Center {
                zd,
                zg,
                dd,
                gg,
                start_index: a.start_index,
                end_index: c.end_index,
            })
        };
        assert_eq!(center_from_segments(a, b, c), want(alt && zd < zg), "{ctx}");
        assert_eq!(center_from_window(a, b, c), want(zd < zg), "{ctx}");
        assert_eq!(
            f.condition_vector(SeedInput::TOWER).all_true(),
            center_from_segments(a, b, c).is_some(),
            "{ctx}: 11111 ⟺ 完整判据成立"
        );
    }

    /// #804 L1：两个 build 都等于 `seed_facts` 的投影，且与定义式直算逐位相同。
    /// (a) 端点 0..=6 的全部相连四点序列（含方向不交替的）；(b) 任意区间网格 × 全部 8 种方向组合。
    #[test]
    fn s1_builds_are_projections_of_seed_facts_small_domain() {
        let mut n = 0usize;
        for p0 in 0..=6 {
            for p1 in 0..=6 {
                for p2 in 0..=6 {
                    for p3 in 0..=6 {
                        if p0 == p1 || p1 == p2 || p2 == p3 {
                            continue;
                        }
                        let u = connected(&[p0, p1, p2, p3]);
                        assert_projection(&u[0], &u[1], &u[2]);
                        n += 1;
                    }
                }
            }
        }
        const BOUNDS: [Tick; 4] = [0, 5, 10, 20];
        let ranges: Vec<(Tick, Tick)> = BOUNDS
            .iter()
            .flat_map(|&lo| {
                BOUNDS
                    .iter()
                    .filter(move |&&hi| lo <= hi)
                    .map(move |&hi| (lo, hi))
            })
            .collect();
        for mask in 0..8u8 {
            let dir = |bit: u8| if mask & (1 << bit) != 0 { up() } else { down() };
            for &(al, ah) in &ranges {
                for &(bl, bh) in &ranges {
                    for &(cl, ch) in &ranges {
                        let a = unit(0, 4, dir(0), al, ah);
                        let b = unit(4, 8, dir(1), bl, bh);
                        let c = unit(8, 12, dir(2), cl, ch);
                        assert_projection(&a, &b, &c);
                        n += 1;
                    }
                }
            }
        }
        assert!(n > 9_000, "穷举域非空（{n}）");
    }
}
