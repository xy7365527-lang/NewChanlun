//! #1087 对拍 wire 格式：Wire* 类型族 + 生产 → wire 渲染层（单一渲染口径）。
//!
//! 自 `scan.rs` 分离（#1177，零行为）：本模块只装 wire schema（Lean 逐字段复核读入的数据
//! 形状）与「生产类型 → wire」的纯渲染——`From` 转换 + [`wire_emission`]/[`wire_output`]。
//! 渲染器单源、不复制（ADR 0026 N-3）。探针状态机（begin/finish/record）留在
//! [`super::issue1087_probe`]，装配逻辑留在 [`super`]。

use super::super::bsp::OwnerRef;
use super::super::cand_event::{
    CandidateKey, CandidateKind, ObservedState, ParentFingerprint, StructuralPredicates,
};
use super::super::divergence::{ForceFeatures, ForceProxies};
use super::super::recursive_tower::{CandDeltaEvent, CpScanOwnership, ElementId};
use super::super::signal::T3InCScan;
use super::*;
use crate::theta_v0::types::{BspBits, Side, ThirdClassEntryIdentity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireDirection {
    Up,
    Down,
}
impl From<Direction> for WireDirection {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Up => Self::Up,
            Direction::Down => Self::Down,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireSide {
    Long,
    Short,
}
impl From<Side> for WireSide {
    fn from(value: Side) -> Self {
        match value {
            Side::Long => Self::Long,
            Side::Short => Self::Short,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireCandidateKind {
    Trend,
    Pan,
}
impl From<CandidateKind> for WireCandidateKind {
    fn from(value: CandidateKind) -> Self {
        match value {
            CandidateKind::Trend => Self::Trend,
            CandidateKind::Pan => Self::Pan,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireObservedState {
    Provisional,
    Unresolved,
    Confirmed,
}
impl From<ObservedState> for WireObservedState {
    fn from(value: ObservedState) -> Self {
        match value {
            ObservedState::Provisional => Self::Provisional,
            ObservedState::Unresolved => Self::Unresolved,
            ObservedState::Confirmed => Self::Confirmed,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireCenter {
    pub zd: Tick,
    pub zg: Tick,
    pub dd: Tick,
    pub gg: Tick,
    pub start_index: usize,
    pub end_index: usize,
}
impl From<Center> for WireCenter {
    fn from(value: Center) -> Self {
        let Center {
            zd,
            zg,
            dd,
            gg,
            start_index,
            end_index,
        } = value;
        Self {
            zd,
            zg,
            dd,
            gg,
            start_index,
            end_index,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireInterval {
    pub left: usize,
    pub right: usize,
}
impl From<(usize, usize)> for WireInterval {
    fn from((left, right): (usize, usize)) -> Self {
        Self { left, right }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireThirdClassEntry {
    pub center_si: usize,
    pub center_zd: Tick,
    pub center_zg: Tick,
    pub leave_interval: WireInterval,
    pub retest_interval: WireInterval,
}
impl From<ThirdClassEntryIdentity> for WireThirdClassEntry {
    fn from(value: ThirdClassEntryIdentity) -> Self {
        let ThirdClassEntryIdentity {
            center_si,
            center_zd,
            center_zg,
            leave_interval,
            retest_interval,
        } = value;
        Self {
            center_si,
            center_zd,
            center_zg,
            leave_interval: leave_interval.into(),
            retest_interval: retest_interval.into(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireBspBits {
    pub buy1: bool,
    pub buy2: bool,
    pub buy3: bool,
    pub sell1: bool,
    pub sell2: bool,
    pub sell3: bool,
    pub third_class_entry: Option<WireThirdClassEntry>,
}
impl From<BspBits> for WireBspBits {
    fn from(value: BspBits) -> Self {
        let BspBits {
            buy1,
            buy2,
            buy3,
            sell1,
            sell2,
            sell3,
            third_class_entry,
        } = value;
        Self {
            buy1,
            buy2,
            buy3,
            sell1,
            sell2,
            sell3,
            third_class_entry: third_class_entry.map(Into::into),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireOwner {
    Center(WireCenter),
    Type1Anchor(usize),
}
impl From<OwnerRef> for WireOwner {
    fn from(value: OwnerRef) -> Self {
        match value {
            OwnerRef::Center(center) => Self::Center(center.into()),
            OwnerRef::Type1Anchor(source_index) => Self::Type1Anchor(source_index),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireForceFeatures {
    pub macd_area_bits: u64,
    pub dif_peak_bits: u64,
    pub price_amplitude: i64,
    pub price_speed_bits: u64,
}
impl From<ForceFeatures> for WireForceFeatures {
    fn from(value: ForceFeatures) -> Self {
        let ForceFeatures {
            macd_area,
            dif_peak,
            price_amplitude,
            price_speed,
        } = value;
        Self {
            macd_area_bits: macd_area.to_bits(),
            dif_peak_bits: dif_peak.to_bits(),
            price_amplitude,
            price_speed_bits: price_speed.to_bits(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireForceProxies {
    pub seg_a: WireForceFeatures,
    pub seg_c: WireForceFeatures,
}
impl From<ForceProxies> for WireForceProxies {
    fn from(value: ForceProxies) -> Self {
        let ForceProxies { seg_a, seg_c } = value;
        Self {
            seg_a: seg_a.into(),
            seg_c: seg_c.into(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireBspPoint {
    pub source_index: usize,
    pub bits: WireBspBits,
    pub pivot_low: Tick,
    pub pivot_high: Tick,
    pub center: Option<WireOwner>,
    pub struct_break_dir: Option<WireSide>,
    pub force: Option<WireForceProxies>,
    pub retrace_breaks_type1: Option<bool>,
}
impl From<BspPoint> for WireBspPoint {
    fn from(value: BspPoint) -> Self {
        let BspPoint {
            source_index,
            bits,
            pivot_low,
            pivot_high,
            center,
            struct_break_dir,
            force,
            retrace_breaks_type1,
        } = value;
        Self {
            source_index,
            bits: bits.into(),
            pivot_low,
            pivot_high,
            center: center.map(Into::into),
            struct_break_dir: struct_break_dir.map(Into::into),
            force: force.map(Into::into),
            retrace_breaks_type1,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WirePanDivCert {
    pub source_index: usize,
    pub side: WireSide,
    pub center: WireCenter,
    pub seg_a: WireInterval,
    pub seg_c: WireInterval,
}
impl From<PanDivCert> for WirePanDivCert {
    fn from(value: PanDivCert) -> Self {
        let PanDivCert {
            source_index,
            side,
            center,
            seg_a,
            seg_c,
        } = value;
        Self {
            source_index,
            side: side.into(),
            center: center.into(),
            seg_a: seg_a.into(),
            seg_c: seg_c.into(),
        }
    }
}
/// #1249 统一后 wire 分级：T3-in-c 全窗后扫结果（`trend_third_class_in_c` 折叠）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireT3InCScan {
    Present {
        leave_interval: WireInterval,
        retest_interval: WireInterval,
    },
    Missing,
}
impl From<T3InCScan> for WireT3InCScan {
    fn from(value: T3InCScan) -> Self {
        match value {
            T3InCScan::Present {
                leave_interval,
                retest_interval,
            } => Self::Present {
                leave_interval: leave_interval.into(),
                retest_interval: retest_interval.into(),
            },
            T3InCScan::Missing => Self::Missing,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireFirstClassGradeRecord {
    pub level: u32,
    pub source_index: usize,
    pub side: WireSide,
    pub center_start_index: usize,
    pub center_end_index: usize,
    pub center_zd: Tick,
    pub center_zg: Tick,
    pub grade: WireT3InCScan,
}
impl From<FirstClassGradeRecord> for WireFirstClassGradeRecord {
    fn from(value: FirstClassGradeRecord) -> Self {
        let FirstClassGradeRecord {
            level,
            source_index,
            side,
            center_start_index,
            center_end_index,
            center_zd,
            center_zg,
            grade,
        } = value;
        Self {
            level,
            source_index,
            side: side.into(),
            center_start_index,
            center_end_index,
            center_zd,
            center_zg,
            grade: grade.into(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireParentFingerprint {
    pub center_start: usize,
    pub zd: i64,
    pub zg: i64,
}
impl From<ParentFingerprint> for WireParentFingerprint {
    fn from(value: ParentFingerprint) -> Self {
        let ParentFingerprint {
            center_start,
            zd,
            zg,
        } = value;
        Self {
            center_start,
            zd,
            zg,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireCandidateKey {
    pub rule_version: u32,
    pub level: u32,
    pub kind: WireCandidateKind,
    pub side: WireSide,
    pub previous_center_start: Option<usize>,
    pub parent: WireParentFingerprint,
    pub seg_a: WireInterval,
    pub c_start: usize,
}
impl From<CandidateKey> for WireCandidateKey {
    fn from(value: CandidateKey) -> Self {
        let CandidateKey {
            rule_version,
            level,
            kind,
            side,
            previous_center_start,
            parent,
            seg_a,
            c_start,
        } = value;
        Self {
            rule_version,
            level,
            kind: kind.into(),
            side: side.into(),
            previous_center_start,
            parent: parent.into(),
            seg_a: seg_a.into(),
            c_start,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireStructuralPredicates {
    pub direction: bool,
    pub comparable: bool,
    pub extreme: bool,
}
impl From<StructuralPredicates> for WireStructuralPredicates {
    fn from(value: StructuralPredicates) -> Self {
        let StructuralPredicates {
            direction,
            comparable,
            extreme,
        } = value;
        Self {
            direction,
            comparable,
            extreme,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireCandidateObservation {
    pub key: WireCandidateKey,
    pub kind: WireCandidateKind,
    pub center_ids: Option<WireInterval>,
    pub candidate_group_id: u64,
    pub pair_id: u64,
    pub structural_predicates: WireStructuralPredicates,
    pub extreme_proof: WireInterval,
    pub third_class_proof: Option<usize>,
    pub interval: WireInterval,
    pub state: WireObservedState,
    pub first_provable_at: Option<usize>,
    pub confirmed_at: Option<usize>,
}
impl From<&CandidateObservation> for WireCandidateObservation {
    fn from(value: &CandidateObservation) -> Self {
        let CandidateObservation {
            key,
            kind,
            center_ids,
            candidate_group_id,
            pair_id,
            structural_predicates,
            extreme_proof,
            third_class_proof,
            interval,
            state,
            first_provable_at,
            confirmed_at,
        } = value;
        Self {
            key: (*key).into(),
            kind: (*kind).into(),
            center_ids: center_ids.map(Into::into),
            candidate_group_id: *candidate_group_id,
            pair_id: *pair_id,
            structural_predicates: (*structural_predicates).into(),
            extreme_proof: (*extreme_proof).into(),
            third_class_proof: *third_class_proof,
            interval: (*interval).into(),
            state: (*state).into(),
            first_provable_at: *first_provable_at,
            confirmed_at: *confirmed_at,
        }
    }
}
/// 原始 Trend sink 腿。candidate_group_id/pair_id 是 post-scan 派生量，禁止出现在此 wire。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireCandidateLeg {
    pub key: WireCandidateKey,
    pub kind: WireCandidateKind,
    pub center_ids: Option<WireInterval>,
    pub structural_predicates: WireStructuralPredicates,
    pub extreme_proof: WireInterval,
    pub third_class_proof: Option<usize>,
    pub interval: WireInterval,
    pub state: WireObservedState,
    pub first_provable_at: Option<usize>,
    pub confirmed_at: Option<usize>,
}
impl From<&CandidateObservation> for WireCandidateLeg {
    fn from(value: &CandidateObservation) -> Self {
        Self {
            key: value.key.into(),
            kind: value.kind.into(),
            center_ids: value.center_ids.map(Into::into),
            structural_predicates: value.structural_predicates.into(),
            extreme_proof: value.extreme_proof.into(),
            third_class_proof: value.third_class_proof,
            interval: value.interval.into(),
            state: value.state.into(),
            first_provable_at: value.first_provable_at,
            confirmed_at: value.confirmed_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireMergedScanOutput {
    pub points: Vec<WireBspPoint>,
    pub pan_divs: Vec<WirePanDivCert>,
    pub grades: Vec<WireFirstClassGradeRecord>,
    pub observations: Vec<WireCandidateObservation>,
}

/// 生产扫描某一段发出的原始 sink。列表尚未排序，候选腿尚未归约，也未加入 Pan 投影。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireScanSinkEmission {
    pub points: Vec<WireBspPoint>,
    pub pan_divs: Vec<WirePanDivCert>,
    pub grades: Vec<WireFirstClassGradeRecord>,
    pub candidate_legs: Vec<WireCandidateLeg>,
}

pub(super) fn wire_emission(
    points: &[BspPoint],
    pan_divs: &[PanDivCert],
    grades: &[FirstClassGradeRecord],
    candidate_legs: &[CandidateObservation],
) -> WireScanSinkEmission {
    WireScanSinkEmission {
        points: points.iter().copied().map(Into::into).collect(),
        pan_divs: pan_divs.iter().copied().map(Into::into).collect(),
        grades: grades.iter().copied().map(Into::into).collect(),
        candidate_legs: candidate_legs.iter().map(Into::into).collect(),
    }
}

pub(super) fn wire_output(
    points: &[BspPoint],
    pan_divs: &[PanDivCert],
    grades: &[FirstClassGradeRecord],
    observations: &[CandidateObservation],
) -> WireMergedScanOutput {
    WireMergedScanOutput {
        points: points.iter().copied().map(Into::into).collect(),
        pan_divs: pan_divs.iter().copied().map(Into::into).collect(),
        grades: grades.iter().copied().map(Into::into).collect(),
        observations: observations.iter().map(Into::into).collect(),
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireSegmentRow {
    pub direction: WireDirection,
    pub start_index: usize,
    pub end_index: usize,
    pub start_price: Tick,
    pub end_price: Tick,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeCase {
    pub center_start_index: usize,
    pub center_end_index: usize,
    pub center_zd: Tick,
    pub center_zg: Tick,
    pub center_dd: Tick,
    pub center_gg: Tick,
    pub departure_dir: WireDirection,
    pub until_start: usize,
    pub rust_lambda_c: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireMoveKind {
    Trend,
    Consolidation,
}
impl From<MoveKind> for WireMoveKind {
    fn from(value: MoveKind) -> Self {
        match value {
            MoveKind::Trend => Self::Trend,
            MoveKind::Consolidation => Self::Consolidation,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireMoveBlock {
    pub start_center: usize,
    pub end_center: usize,
    pub kind: WireMoveKind,
    pub direction: Option<WireDirection>,
    pub level_lift: u8,
}
impl From<MoveBlock> for WireMoveBlock {
    fn from(value: MoveBlock) -> Self {
        Self {
            start_center: value.start_center,
            end_center: value.end_center,
            kind: value.kind.into(),
            direction: value.dir.map(Into::into),
            level_lift: value.level_lift,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireASegmentEnvelope {
    pub span: WireInterval,
    pub low: Tick,
    pub high: Tick,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WirePreludeInput {
    pub rows: Vec<WireSegmentRow>,
    pub centers: Vec<WireCenter>,
    pub blocks: Vec<WireMoveBlock>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WirePreludeOutput {
    pub segments_sorted: bool,
    pub centers_sorted: bool,
    pub anchors: Vec<Option<WireDirection>>,
    pub trend_gate: Vec<Option<WireDirection>>,
    pub first_match_idx: Vec<Option<usize>>,
    pub a_segments: Vec<Option<WireASegmentEnvelope>>,
}

/// `cp_event_objects` 所需的 c_p 扫描基础事实；故意不携带成品事件投影。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireCpScanBase {
    pub b_center_index: usize,
    pub b_center_id: ElementId,
    pub b_center: WireCenter,
    pub departure_move_id: Option<ElementId>,
    pub departure_interval: Option<WireInterval>,
}

/// `LeveledMove` 的低层几何/分解事实；`center` 仅为 Compose 末位中枢。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireUnitMoveFact {
    pub id: ElementId,
    pub start_index: usize,
    pub end_index: usize,
    pub low: Tick,
    pub high: Tick,
    pub center: Option<WireCenter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireEventRawContext {
    pub centers: Vec<WireCenter>,
    pub rows: Vec<WireSegmentRow>,
    pub anchors: Vec<Option<WireDirection>>,
    pub cp_scan: Vec<WireCpScanBase>,
    pub unit_moves: Vec<WireUnitMoveFact>,
}

/// event 之前的 raw 判据输入 + Rust 生产事件（被检侧）。accepted 不在 wire 中。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireCandDeltaCase {
    pub kind: WireCandidateKind,
    pub side: WireSide,
    pub divergence_confirm_src: usize,
    pub a_interval: WireInterval,
    pub center_index: usize,
    pub segment_index: usize,
    pub structural_candidate: bool,
    pub direction: bool,
    pub comparable: bool,
    pub extreme: bool,
    pub buy1: bool,
    pub sell1: bool,
    pub pan_diverges: bool,
    pub pan_div_diag: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireCpClosureEvidence {
    pub visible_unit_move_count: usize,
    pub cp_departure_move_id: super::super::recursive_tower::ElementId,
    pub cp_start: usize,
    pub leave: Segment,
    pub retest: Segment,
    pub leave_anchor: Option<Direction>,
    pub leave_move_id: super::super::recursive_tower::ElementId,
    pub retest_move_id: super::super::recursive_tower::ElementId,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireCpTransition {
    pub before: CpScanOwnership,
    pub after: CpScanOwnership,
    pub raw: WireCpClosureEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanParityRecord {
    pub level: u32,
    /// Rust 生产合并扫描的最终四件输出，只作为被检侧。
    pub rust_output: WireMergedScanOutput,
    /// Rust 生产路径逐段 sink；Lean 从这里独立重算最终输出。
    pub emissions: Vec<WireScanSinkEmission>,
    pub prelude_input: WirePreludeInput,
    pub prelude_output: WirePreludeOutput,
    /// CandDelta cases 共享的 raw 低层上下文。
    pub event_context: WireEventRawContext,
    pub rows: Vec<WireSegmentRow>,
    pub episode_cases: Vec<EpisodeCase>,
    pub cand_delta_cases: Vec<WireCandDeltaCase>,
    /// Rust 生产 event 的完整列表；Lean 列表级门与 raw cases 做 fail-closed 双射。
    pub cand_delta_events: Vec<CandDeltaEvent>,
    /// 本 level 的 stable-revision 生命周期转移；raw 复用同 record 上下文。
    pub cp_transitions: Vec<WireCpTransition>,
}
