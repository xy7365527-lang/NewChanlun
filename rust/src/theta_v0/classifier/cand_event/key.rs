//! #634 拆自单文件 `classifier/cand_event.rs` 的**类型与判据**域。
//!
//! 只定义候选「是什么」——身份键、状态、结构谓词、事件形状、载荷投影口径——与纯几何判据
//! （闭区间退化/包含/相切/相离）。不含事件簿修订协议（见 [`super::book`]），也不含单次塔扫描
//! 的观察产出（见 [`super::observe`]）。本文件零外部调用、零可变状态。

use std::rc::Rc;

use super::super::super::types::Side;

pub const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
pub const FNV_PRIME: u64 = 0x100000001b3;

/// 候选判定规则版本（E2E-O「上游身份或规则版本变 ⟹ 新 key，禁复活旧 key」的版本分量）。
///
/// 语义：本常量随**候选判定规则**（结构门口径、状态映射、身份字段构成）的任何实质变更递增。
/// 版本进 [`CandidateKey`] ⟹ 规则变更后旧 key 在新扫描中不再被观察到 ⟹ 经既有消失路径判
/// `Invalidated`（终态），新规则的候选以新 key 从 ∅ 开始——旧 key 不被复活、旧生命史不被改写。
///
/// 值 1 = #550 落地的结构宽候选口径 + #551 的四态全谱映射。
pub const CANDIDATE_RULE_VERSION: u32 = 1;

/// 背驰段候选类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CandidateKind {
    Trend,
    Pan,
}

/// Bₚ 的稳定结构指纹。中枢延伸的右端不参与身份。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParentFingerprint {
    pub center_start: usize,
    pub zd: i64,
    pub zg: i64,
}

/// 候选稳定身份。C 右端与 as_of 均不在键中。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CandidateKey {
    /// 判定规则版本（见 [`CANDIDATE_RULE_VERSION`]）：规则变更 ⟹ 全体 key 变 ⟹ 旧 key 走消失
    /// 路径判终态，新 key 从 ∅ 起——E2E-O「规则版本变 ⟹ 新 key 禁复活」的机器载体。
    pub rule_version: u32,
    pub level: u32,
    pub kind: CandidateKind,
    pub side: Side,
    /// Trend 域的前中枢；Pan 域没有前中枢，必须为 None。
    pub previous_center_start: Option<usize>,
    pub parent: ParentFingerprint,
    pub seg_a: (usize, usize),
    pub c_start: usize,
}

/// E2E-D5 候选状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CandidateState {
    Provisional,
    Unresolved,
    Confirmed,
    Invalidated,
}

impl CandidateState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Confirmed | Self::Invalidated)
    }
}

/// 结构谓词证据；字段值来自同一次塔扫描的 [`super::super::signal::FirstStructuralGates`]，
/// 不在本模块重判。
///
/// #551 起三项与结构门逐条同源，**不再恒真**：
/// - `direction`：破最后中枢 ∧ 破向 = 趋势向。恒 `true` 是**结构必然**而非占位——该门不成立时
///   结构门函数返回 `None`，候选身份根本不存在（无 `seg_a`/`c_start` 来源），故凡入簿事件此项必真。
/// - `comparable`：I(A)/I(C) 均可映射到 closes 下标（MACD 面积坐标系上 A/C 可比较）。
/// - `extreme`：037:20，c 端点严格破 b = I(A) 包络极值。
///
/// `comparable ∧ extreme` ⟺ 结构宽候选完全成立 ⟺ [`CandidateState::Provisional`]；任一不成立
/// ⟹ [`CandidateState::Unresolved`]（结构未决，非失效——同一 key 的 C 段后续可延伸至成立）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralPredicates {
    pub direction: bool,
    pub comparable: bool,
    pub extreme: bool,
}

impl StructuralPredicates {
    /// 三谓词全成立 ⟹ 结构宽候选成立（`Provisional`）；否则未决（`Unresolved`）。
    pub fn all_hold(self) -> bool {
        self.direction && self.comparable && self.extreme
    }

    /// 由谓词组合派生的活假设状态（Trend 域状态映射单一来源）。
    pub fn resolved_state(self) -> CandidateState {
        if self.all_hold() {
            CandidateState::Provisional
        } else {
            CandidateState::Unresolved
        }
    }
}

/// 一等候选事件的一次不可变 revision。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateEvent {
    pub key: CandidateKey,
    pub kind: CandidateKind,
    pub event_level: u32,
    /// Trend 域为（前中枢，父中枢）；Pan 域没有前中枢，必须为 None。
    pub center_ids: Option<(usize, usize)>,
    /// SPEC E2E-D2 字段；当前为 CandidateKey 的 FNV 种子哈希，与 key 双射，无独立信息。
    pub candidate_group_id: u64,
    /// SPEC E2E-D2 字段；当前为 CandidateKey 的另一种子哈希，与 key 双射，无独立信息。
    pub pair_id: u64,
    pub structural_predicates: StructuralPredicates,
    /// SPEC E2E-D3 形状位；当前恒为 `key.seg_a` 的副本，无独立业务信息。
    pub extreme_proof: (usize, usize),
    pub third_class_proof: Option<usize>,
    /// C 段闭区间，source_index 坐标域。
    pub interval: (usize, usize),
    pub state: CandidateState,
    /// 候选首次入簿时的 `as_of`（一次写入不后移）。
    pub observed_at: usize,
    /// 首证钟：结构宽候选**首次完全成立**时的结构位（一次写入不后移）。
    ///
    /// `None` = 尚未可证（该 key 迄今只观察到 [`CandidateState::Unresolved`]）。写入时机严格是
    /// 「首次全谓词成立的那次观察」的结构位——不提前（未决期不写）、不回填（写入后即钉死，
    /// 后续退回 `Unresolved` 也不抹去）。#535 红线②的机器载体。
    pub first_provable_at: Option<usize>,
    pub confirmed_at: Option<usize>,
    pub invalidated_at: Option<usize>,
    pub revision: u32,
    pub supersedes_revision: Option<u32>,
    pub revision_at: usize,
}

/// 每级一条 append-only 流；双层 Rc 让事件入口只克隆引用，不深拷贝全簿。
pub type CandidateStreams = Rc<Vec<Rc<Vec<CandidateEvent>>>>;

/// 区间退化：左端严格大于右端。
///
/// 退化 ≠ 空区间：C 段区间是**闭**区间，`(a, a)` 是合法单点区间（非退化）；`start > end` 是
/// 非法几何，没有对应的 source_index 跨度，端点/交集/包含在其上均无定义。
///
/// 本仓库三个闭区间判据（[`interval_is_sub`] / [`intervals_touch`] / [`intervals_are_disjoint`]）
/// 共用它作前置守卫，退化输入一律判 **false**——这三条不是「哪个成立」的三分，退化对在三者上
/// 同时为假。调用方若需区分「判过但全不成立」与「退化不可判」，须自行按本谓词单列计数。
pub fn interval_is_degenerate(interval: (usize, usize)) -> bool {
    interval.0 > interval.1
}

/// 闭区间 C⊆C；端点相等（相切）算包含。任一区间退化 ⟹ false。
pub fn interval_is_sub(child: (usize, usize), parent: (usize, usize)) -> bool {
    !interval_is_degenerate(child)
        && !interval_is_degenerate(parent)
        && parent.0 <= child.0
        && child.1 <= parent.1
}

/// 两闭区间**相切**：至少一端点相等（#246 相切口径）。任一区间退化 ⟹ false。
///
/// 与 [`interval_is_sub`] 正交：相切不蕴含包含（`(10,30)` 与 `(10,20)` 左端相切但互不包含），
/// 包含也不蕴含相切（严格内含两端均不等）。调用方要「包含且相切」须自取合取。
pub fn intervals_touch(a: (usize, usize), b: (usize, usize)) -> bool {
    !interval_is_degenerate(a) && !interval_is_degenerate(b) && (a.0 == b.0 || a.1 == b.1)
}

/// 两闭区间**相离**：交集为空。闭区间 ⟹ 端点相等即相交（`(10,20)` 与 `(20,30)` 不相离）。
/// 任一区间退化 ⟹ false。
pub fn intervals_are_disjoint(a: (usize, usize), b: (usize, usize)) -> bool {
    !interval_is_degenerate(a) && !interval_is_degenerate(b) && (a.1 < b.0 || b.1 < a.0)
}

/// [`CandidateEvent`] 的业务载荷投影 —— 候选「是什么」的**唯一比较口径**。
///
/// 进投影的是候选的业务身份与结构载荷：`kind`、`event_level`、`center_ids`、
/// `candidate_group_id`、`pair_id`、`structural_predicates`、`extreme_proof`、
/// `third_class_proof`、`interval`、`state`。
///
/// **不进投影**的是生命史记账字段：钟（`observed_at` / `first_provable_at` / `confirmed_at` /
/// `invalidated_at` / `revision_at`）与 revision 计数（`revision` / `supersedes_revision`）。
/// 它们记的是「这条载荷何时被观察到、是第几次修订」，属生命史而非载荷本身：同一份载荷在不同
/// as_of 下重跑必然带不同的钟与计数，把它们计入等价比较会让「载荷未变」永远判不成立。
/// 幂等跳过（[`super::book::CandidateEventBook::advance_observation`]）、fresh↔causal 双通道
/// 载荷比对（`issue550_event_battery` / `payload_eq`）共用本口径，不再各列一份字段表。
///
/// `key` 亦不入投影：它是流的索引而非载荷，全部比对点都在**同 key** 内进行；`event_level`
/// 虽由 `key.level` 派生，仍显式入投影，使投影自足可读、不依赖调用方保证同 key。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateProjection {
    pub kind: CandidateKind,
    pub event_level: u32,
    pub center_ids: Option<(usize, usize)>,
    pub candidate_group_id: u64,
    pub pair_id: u64,
    pub structural_predicates: StructuralPredicates,
    pub extreme_proof: (usize, usize),
    pub third_class_proof: Option<usize>,
    pub interval: (usize, usize),
    pub state: CandidateState,
}

impl CandidateEvent {
    /// 取本次 revision 的业务载荷投影，口径见 [`CandidateProjection`]。
    pub fn projection(&self) -> CandidateProjection {
        CandidateProjection {
            kind: self.kind,
            event_level: self.event_level,
            center_ids: self.center_ids,
            candidate_group_id: self.candidate_group_id,
            pair_id: self.pair_id,
            structural_predicates: self.structural_predicates,
            extreme_proof: self.extreme_proof,
            third_class_proof: self.third_class_proof,
            interval: self.interval,
            state: self.state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_interval_sub_truth_table_includes_touching_endpoints() {
        assert!(interval_is_sub((10, 20), (10, 20)));
        assert!(interval_is_sub((10, 15), (10, 20)));
        assert!(interval_is_sub((15, 20), (10, 20)));
        assert!(interval_is_sub((11, 19), (10, 20)));
        assert!(!interval_is_sub((9, 20), (10, 20)));
        assert!(!interval_is_sub((10, 21), (10, 20)));
        assert!(!interval_is_sub((20, 10), (10, 20)));
        assert!(!interval_is_sub((11, 19), (20, 10)));
    }

    #[test]
    fn degenerate_is_start_strictly_after_end_and_single_point_is_legal() {
        assert!(interval_is_degenerate((20, 10)));
        assert!(interval_is_degenerate((11, 10)));
        // 单点闭区间合法（非退化）——退化 ≠ 空。
        assert!(!interval_is_degenerate((10, 10)));
        assert!(!interval_is_degenerate((10, 20)));
    }

    #[test]
    fn touching_truth_table_covers_each_endpoint_and_degenerate_guard() {
        // 左端相切。
        assert!(intervals_touch((10, 15), (10, 20)));
        // 右端相切。
        assert!(intervals_touch((15, 20), (10, 20)));
        // 两端同时相切（同区间）。
        assert!(intervals_touch((10, 20), (10, 20)));
        // 相切但互不包含——相切与包含正交。
        assert!(intervals_touch((10, 30), (10, 20)));
        assert!(!interval_is_sub((10, 30), (10, 20)));
        // 严格内含 ⟹ 两端点均不等 ⟹ 不相切。
        assert!(!intervals_touch((11, 19), (10, 20)));
        // 相离且端点不等。
        assert!(!intervals_touch((30, 40), (10, 20)));
        // 交叉端点（a.1 == b.0）不算相切——本谓词判的是**同侧**端点相等。
        assert!(!intervals_touch((5, 10), (10, 20)));
        // 退化守卫：任一侧退化 ⟹ false，即便端点数值相等。
        assert!(!intervals_touch((20, 10), (20, 30)));
        assert!(!intervals_touch((10, 20), (20, 10)));
        assert!(!intervals_touch((20, 10), (20, 10)));
    }

    #[test]
    fn disjoint_truth_table_covers_closed_interval_touching_and_degenerate_guard() {
        // 完全相离（两侧各一）。
        assert!(intervals_are_disjoint((30, 40), (10, 20)));
        assert!(intervals_are_disjoint((1, 5), (10, 20)));
        // 闭区间 ⟹ 端点相接即相交，不相离。
        assert!(!intervals_are_disjoint((20, 30), (10, 20)));
        // 部分交叠 / 包含 / 同区间均不相离。
        assert!(!intervals_are_disjoint((5, 15), (10, 20)));
        assert!(!intervals_are_disjoint((11, 19), (10, 20)));
        assert!(!intervals_are_disjoint((10, 20), (10, 20)));
        // 单点区间：落在外面才相离。
        assert!(intervals_are_disjoint((30, 30), (10, 20)));
        assert!(!intervals_are_disjoint((15, 15), (10, 20)));
        // 退化守卫：任一侧退化 ⟹ false，即便数值上「看着无交」。
        assert!(!intervals_are_disjoint((60, 50), (10, 20)));
        assert!(!intervals_are_disjoint((10, 20), (60, 50)));
        assert!(!intervals_are_disjoint((60, 50), (40, 30)));
    }
}
