//! 3a 生产单扫描（SPEC #1077 D1）：一趟段扫描合并 BSP 三投影与候选观察两路。
//!
//! 生产门查走逐点口径（[`decompose::center_own_dir_at`]/[`decompose::center_block_kind_at`]/
//! [`decompose::center_block_lift_at`]）；批式数组法（`center_trend_gate`）只活在全量神谕
//! （[`signal::extract_signals_with_hist_anchored`]）与 P1 镜像（`recursive_tower::level_cand_delta`），
//! 不产生交易决策（#1057 Q2）。
//!
//! 共享 per-segment 中间记录（seg_idx / 最近 confirmed 中枢 idx / A 段+b 包络 / λ_C /
//! first_structural_gates / 方向锚 / departure 终点）在一次判定内喂 BSP 域 sink
//! （BspPoint/PanDivCert/FirstClassGradeRecord）与候选域 sink（CandidateObservation 腿）。
//! 候选域归约层（merged/首证钟/state）留在扫描之后（[`cand_event::reduce_structural_legs`]），
//! 不进 per-segment 记录、不进 frontier（#1057 Q1/Q3）。
//!
//! frontier 冻结到 per-segment 素材层：BSP 域冻结一/三类点、盘背证书、分级记录（同旧
//! `extract_first_third_resume` 的 `cached_first_third*`）；候选域冻结结构宽候选**腿**
//! （`cached_candidate_legs`，未归约）。扫描末尾对 prefix+tail 腿全量重跑归约
//! （O(observations) 轻量），`first_provable_at` 一次写入不后移不变式天然保持。
//!
//! ★D2 字段族边界（#1057 补接 + #1059 路线 A）：CandDeltaEvent 的快照字段
//! （c_interval_full/b_parent/c_structure/third_class_in_c/cp_certificate_confirm_src/
//! full_trend_c_qualified/full_trend_evidence）与 pan_div_diag 是 P1 谓词闭包层（
//! `recursive_tower::level_cand_delta`/`cp_event_objects`）从 `cp_ownership` 侧车合成的字段——
//! 3a 期间 P1 手工镜像照跑（#1057 Q5），这些字段由 P1 独立计算、不经本生产扫描；3b 纯投影
//! （统一记录 + `LevelState.cp_ownership`，#1059 路线 A）落地时再以 cp_ownership 为输入合成，
//! 生产热路径不物化 CandDeltaEvent。本扫描冻结的素材装全生产两域所需字段：P1 谓词层字段
//! （cand_delta=bits.buy1/sell1、confirm_src=source_index、side=struct_break_dir 均可从
//! BspPoint 派生）与区间族（interval/a_interval/c_episode_start/enter_src 均落候选腿
//! key/interval 与 PanDivCert 内），冻结不裁。

use super::super::types::{Center, Direction, MoveKind, Segment, Stroke, Tick};
use super::cand_event::{self, CandidateObservation};
use super::decompose::{center_block_kind_at, center_block_lift_at, center_own_dir_at, MoveBlock};
use super::divergence::{
    departure_move_c_start, locate_departure_move_a, move_range_envelope, DivergenceGauge,
};
use super::signal::{self, BspPoint, FirstClassGradeRecord, PanDivCert};
use std::collections::HashMap;

/// #1087 重型签收只读观测面：比较 Rust merged 与 Rust legacy full oracle；Lean 只验 wire bridge。
/// feature 关闭时不编译，显式 begin 才重跑神谕。
#[cfg(feature = "issue1087_parity")]
pub(crate) mod issue1087_probe {
    use super::super::bsp::OwnerRef;
    use super::super::cand_event::{
        CandidateKey, CandidateKind, ObservedState, ParentFingerprint, StructuralPredicates,
    };
    use super::super::divergence::{ForceFeatures, ForceProxies};
    use super::super::signal::{T3InCGrade, T3InCGradeReason};
    use super::*;
    use crate::theta_v0::types::{BspBits, Side, ThirdClassEntryIdentity};
    use std::fmt::Debug;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    };

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
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum WireT3InCGradeReason {
        MissingLeave,
        MissingRetest,
        SameDirection,
        LeaveNotOutside,
        RetestReentered,
    }
    impl From<T3InCGradeReason> for WireT3InCGradeReason {
        fn from(value: T3InCGradeReason) -> Self {
            match value {
                T3InCGradeReason::MissingLeave => Self::MissingLeave,
                T3InCGradeReason::MissingRetest => Self::MissingRetest,
                T3InCGradeReason::SameDirection => Self::SameDirection,
                T3InCGradeReason::LeaveNotOutside => Self::LeaveNotOutside,
                T3InCGradeReason::RetestReentered => Self::RetestReentered,
            }
        }
    }
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum WireT3InCGrade {
        Present {
            leave_interval: WireInterval,
            retest_interval: WireInterval,
        },
        Missing(WireT3InCGradeReason),
    }
    impl From<T3InCGrade> for WireT3InCGrade {
        fn from(value: T3InCGrade) -> Self {
            match value {
                T3InCGrade::Present {
                    leave_interval,
                    retest_interval,
                } => Self::Present {
                    leave_interval: leave_interval.into(),
                    retest_interval: retest_interval.into(),
                },
                T3InCGrade::Missing(reason) => Self::Missing(reason.into()),
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
        pub grade: WireT3InCGrade,
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
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct WireMergedScanOutput {
        pub points: Vec<WireBspPoint>,
        pub pan_divs: Vec<WirePanDivCert>,
        pub grades: Vec<WireFirstClassGradeRecord>,
        pub observations: Vec<WireCandidateObservation>,
    }
    fn wire_output(
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
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ScanParityRecord {
        pub level: u32,
        pub merged: WireMergedScanOutput,
        pub oracle: WireMergedScanOutput,
        pub point_mismatches: usize,
        pub pan_div_mismatches: usize,
        pub grade_mismatches: usize,
        pub observation_mismatches: usize,
        pub first_mismatch: Option<String>,
        pub rows: Vec<WireSegmentRow>,
        pub episode_cases: Vec<EpisodeCase>,
    }
    impl ScanParityRecord {
        pub fn total_mismatches(&self) -> usize {
            self.point_mismatches
                + self.pan_div_mismatches
                + self.grade_mismatches
                + self.observation_mismatches
        }
    }
    static ACTIVE: AtomicBool = AtomicBool::new(false);
    static RECORDS: Mutex<Vec<ScanParityRecord>> = Mutex::new(Vec::new());
    pub fn begin() {
        RECORDS.lock().expect("#1087 probe mutex poisoned").clear();
        ACTIVE.store(true, Ordering::SeqCst);
    }
    pub fn finish() -> Vec<ScanParityRecord> {
        ACTIVE.store(false, Ordering::SeqCst);
        std::mem::take(&mut *RECORDS.lock().expect("#1087 probe mutex poisoned"))
    }
    pub(super) fn active() -> bool {
        ACTIVE.load(Ordering::SeqCst)
    }

    fn snapshot_mismatches<T: Debug + Eq>(
        left: &[T],
        right: &[T],
        label: &str,
    ) -> (usize, Option<String>) {
        let mut count = left.len().abs_diff(right.len());
        let mut first = (left.len() != right.len())
            .then(|| format!("{label}.len: merged={} oracle={}", left.len(), right.len()));
        for (index, (merged, oracle)) in left.iter().zip(right).enumerate() {
            if merged != oracle {
                count += 1;
                first.get_or_insert_with(|| {
                    format!("{label}[{index}] wire mismatch: merged={merged:?}; oracle={oracle:?}")
                });
            }
        }
        (count, first)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn record(
        level: u32,
        centers: &[Center],
        segments: &[Segment],
        anchor_dirs: Option<&[Option<Direction>]>,
        departure_ends: Option<&[usize]>,
        blocks: &[MoveBlock],
        hist: &[f64],
        dif: &[f64],
        closes_tick: &[Tick],
        close_src: &[usize],
        gauge: DivergenceGauge,
        strokes: &[Stroke],
        points: &[BspPoint],
        pan_divs: &[PanDivCert],
        grades: &[FirstClassGradeRecord],
        observations: &[CandidateObservation],
    ) {
        let anchors_self: Vec<Option<Direction>> = segments
            .iter()
            .map(|segment| Some(segment.direction))
            .collect();
        let anchors = anchor_dirs.unwrap_or(&anchors_self);
        let mut full_grades = Vec::new();
        let (full_points, full_pan_divs) = signal::extract_signals_with_hist_anchored(
            centers,
            segments,
            anchor_dirs,
            departure_ends,
            hist,
            dif,
            closes_tick,
            close_src,
            gauge,
            strokes,
            &mut full_grades,
        );
        for grade in &mut full_grades {
            grade.level = level;
        }
        let full_observations = cand_event::observations_for_level(
            level,
            centers,
            blocks,
            segments,
            anchors,
            close_src,
            &full_pan_divs,
        );

        let merged = wire_output(points, pan_divs, grades, observations);
        let oracle = wire_output(
            &full_points,
            &full_pan_divs,
            &full_grades,
            &full_observations,
        );
        let (point_mismatches, point_first) =
            snapshot_mismatches(&merged.points, &oracle.points, "points");
        let (pan_div_mismatches, pan_first) =
            snapshot_mismatches(&merged.pan_divs, &oracle.pan_divs, "pan_divs");
        let (grade_mismatches, grade_first) =
            snapshot_mismatches(&merged.grades, &oracle.grades, "grades");
        let (observation_mismatches, observation_first) =
            snapshot_mismatches(&merged.observations, &oracle.observations, "observations");

        let any_trend = blocks.iter().any(|block| block.kind == MoveKind::Trend);
        let episode_cases = segments
            .iter()
            .filter_map(|segment| {
                let center_index =
                    signal::nearest_confirmed_center_idx(centers, segment.start_index)?;
                let direction = any_trend
                    .then(|| center_own_dir_at(blocks, center_index))
                    .flatten()?;
                let center = &centers[center_index];
                Some(EpisodeCase {
                    center_start_index: center.start_index,
                    center_end_index: center.end_index,
                    center_zd: center.zd,
                    center_zg: center.zg,
                    center_dd: center.dd,
                    center_gg: center.gg,
                    departure_dir: direction.into(),
                    until_start: segment.start_index,
                    rust_lambda_c: departure_move_c_start(
                        segments,
                        &anchors_self,
                        center,
                        direction,
                        segment.start_index,
                    ),
                })
            })
            .collect();
        let rows = segments
            .iter()
            .map(|segment| WireSegmentRow {
                direction: segment.direction.into(),
                start_index: segment.start_index,
                end_index: segment.end_index,
                start_price: segment.start_price,
                end_price: segment.end_price,
            })
            .collect();
        RECORDS
            .lock()
            .expect("#1087 probe mutex poisoned")
            .push(ScanParityRecord {
                level,
                merged,
                oracle,
                point_mismatches,
                pan_div_mismatches,
                grade_mismatches,
                observation_mismatches,
                first_mismatch: point_first
                    .or(pan_first)
                    .or(grade_first)
                    .or(observation_first),
                rows,
                episode_cases,
            });
    }
}

/// A 段区间 + b 包络缓存（键 = 最近 confirmed 中枢下标 c_idx；值 = I(A) span 与其包络）。
/// 热点②：趋势 τ 下多段共享同一 (prev_center, last_center) 对 ⟹ 每 c_idx 至多算一次。
type ASegCache = HashMap<usize, Option<((usize, usize), (Tick, Tick))>>;

/// 合并扫描四件产出（SPEC #1077 T1：BSP 三投影 + 候选观察逐字段零改动）。
pub(crate) struct MergedScanOutput {
    pub points: Vec<BspPoint>,
    pub pan_divs: Vec<PanDivCert>,
    pub grades: Vec<FirstClassGradeRecord>,
    pub observations: Vec<CandidateObservation>,
}

/// ★3a 生产合并扫描的 frontier-resume 入口（替代旧 `extract_first_third_resume` +
/// `cand_event::observations_for_level` 两次调用；#1057 Q3「管线层 per-level 两次调用收为一次」）。
///
/// 一趟段扫描内共享 per-segment 素材，同时产 BSP 三投影与候选观察腿；候选域归约在扫描末尾对
/// prefix+tail 全量重跑。生产门查逐点（`center_own_dir_at`/`center_block_kind_at`/
/// `center_block_lift_at`），全量神谕与 P1 镜像保持批式数组。
///
/// `cached_pts/cached_pans/cached_grades/cached_legs`：与旧 `extract_first_third_resume` 同款
/// frontier 缓存（confirmed 前缀的产出/素材），`cached_count` = 已覆盖 confirmed 段前缀数。
#[allow(clippy::too_many_arguments)]
pub(crate) fn merged_scan_resume(
    cached_pts: &mut Vec<BspPoint>,
    cached_pans: &mut Vec<PanDivCert>,
    cached_grades: &mut Vec<FirstClassGradeRecord>,
    cached_legs: &mut Vec<CandidateObservation>,
    cached_count: &mut usize,
    level: u32,
    centers: &[Center],
    segments: &[Segment],
    anchor_dirs: Option<&[Option<Direction>]>,
    departure_ends: Option<&[usize]>,
    blocks: &[MoveBlock],
    prefix_count: usize,
    dirty_e: usize,
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
    gauge: DivergenceGauge,
    strokes: &[Stroke],
) -> MergedScanOutput {
    // 生产路径 segments/centers 已 start_index/end_index 升序（parser 账本 + 非重叠中枢扫描）；
    // anchors 平行。resume 是热路径专用入口，**假设有序**（debug_assert 守护，release 剥离）——
    // 全量 fallback（乱序入参）走 [`signal::extract_signals_with_hist_anchored`]，本入口不重排。
    debug_assert!(
        segments
            .windows(2)
            .all(|w| w[0].start_index <= w[1].start_index),
        "resume 入口假设 segments 有序（生产路径恒真）"
    );
    debug_assert!(
        centers.windows(2).all(|w| w[0].end_index < w[1].end_index),
        "resume 入口假设 centers end_index 严格递增（非重叠扫描恒真 ⟹ 三元组唯一 ⟹ pos==c_idx）"
    );
    let anchors_self: Vec<Option<Direction>> = segments.iter().map(|s| Some(s.direction)).collect();
    let anchors: &[Option<Direction>] = anchor_dirs.unwrap_or(&anchors_self);
    debug_assert_eq!(
        anchors.len(),
        segments.len(),
        "anchor_dirs 与 segments 必等长"
    );
    debug_assert!(
        departure_ends.is_none_or(|d| d.len() == segments.len()),
        "departure_ends 与 segments 必等长（#1028 平行数组）"
    );
    // 生产契约：anchor_dirs 恒为结构方向锚（L0=None ⟹ 回退 self；L≥1=单元结构方向）。本入口
    // 候选域与 BSP 第一类同用 `anchors_self`（与全量候选神谕的 `anchor_dirs` 在生产路径逐位一致）。
    debug_assert!(
        anchors
            .iter()
            .zip(anchors_self.iter())
            .all(|(a, s)| *a == *s),
        "生产路径 anchor_dirs 须 ≡ 结构方向锚（resume 热路径契约）"
    );

    let any_trend = blocks.iter().any(|b| b.kind == MoveKind::Trend);
    // #898：本级盘背只认本级别盘整块（lift==0）——扩展折出的高一级盘整块不算。
    let any_consol = blocks
        .iter()
        .any(|b| b.kind == MoveKind::Consolidation && b.level_lift == 0);

    // 冻结边界 e_src → 可封段数 stable_seg（segments 按 start 升序 ∧ 非重叠 ⟹ end 亦升序 ⟹
    // partition_point 二分）。公式由 `freeze_boundary_src` 单一持有。
    let e_src = signal::freeze_boundary_src(centers, prefix_count, dirty_e);
    let stable_seg = segments.partition_point(|s| s.end_index < e_src);

    // 单调守卫（confirmed 前缀单调非降 ⟹ 正常永不触发；cascade 前缀回缩由 caller 别处 clear +
    // 本守卫兜底）：缓存越过本 bar 冻结边界 ⟹ 保守清空重判（四件产出锁步清空）。
    if *cached_count > stable_seg {
        cached_pts.clear();
        cached_pans.clear();
        cached_grades.clear();
        cached_legs.clear();
        *cached_count = 0;
    }

    // 逐段判定核（advance 与 tail 共享）：pointwise 解析门/类别后共享 per-segment 素材喂两域 sink。
    let mut a_seg_cache: ASegCache = HashMap::new();
    let scan_range = |from: usize,
                      to: usize,
                      pts: &mut Vec<BspPoint>,
                      pans: &mut Vec<PanDivCert>,
                      grades: &mut Vec<FirstClassGradeRecord>,
                      legs: &mut Vec<CandidateObservation>,
                      a_cache: &mut ASegCache| {
        for i in from..to {
            let seg = &segments[i];
            let Some(c_idx) = signal::nearest_confirmed_center_idx(centers, seg.start_index) else {
                continue;
            };
            // pos==c_idx（end_index 严格递增 ⟹ 三元组唯一）；门 = pointwise ownership 方向。
            let gate_dir = if any_trend {
                center_own_dir_at(blocks, c_idx).map(|d| (c_idx, d))
            } else {
                None
            };
            // #898：本级盘背只认本级别盘整块（lift==0）。
            let kind_consol = any_consol
                && center_block_kind_at(blocks, c_idx) == Some(MoveKind::Consolidation)
                && center_block_lift_at(blocks, c_idx) == Some(0);
            merged_judge_segment(
                i,
                seg,
                c_idx,
                gate_dir,
                kind_consol,
                segments,
                anchors,
                &anchors_self,
                centers,
                a_cache,
                hist,
                dif,
                closes_tick,
                close_src,
                gauge,
                strokes,
                level,
                departure_ends,
                pts,
                pans,
                grades,
                legs,
            );
        }
    };

    // 推进：新晋 confirmed 的段 [cached_count..stable_seg) 一次性判入缓存（一生一算，push 序）。
    if *cached_count < stable_seg {
        scan_range(
            *cached_count,
            stable_seg,
            cached_pts,
            cached_pans,
            cached_grades,
            cached_legs,
            &mut a_seg_cache,
        );
        *cached_count = stable_seg;
    }

    // 结果 = confirmed 前缀产出（缓存 clone）+ frontier tail 产出（每 bar 重判，tail 小）。push 序拼接。
    let mut points = cached_pts.clone();
    let mut pan_divs = cached_pans.clone();
    let mut grades = cached_grades.clone();
    let mut legs = cached_legs.clone();
    // tail 的 a_seg_cache 独立（advance 已消耗，tail 段最近中枢多在 frontier）——新建，与 full
    // 路径每调用一份 a_seg_cache 同语义（key=c_idx，命中即复用；跨 advance/tail 不复用不影响 bit）。
    let mut tail_a_cache: ASegCache = HashMap::new();
    scan_range(
        stable_seg,
        segments.len(),
        &mut points,
        &mut pan_divs,
        &mut grades,
        &mut legs,
        &mut tail_a_cache,
    );

    points.sort_by_key(|p: &BspPoint| p.source_index);
    pan_divs.sort_by_key(|p: &PanDivCert| p.source_index);
    grades.sort_by_key(|g: &FirstClassGradeRecord| g.source_index);

    // 候选域归约层（merged/首证钟/state）留在扫描之后：prefix+tail 腿全量重跑归约（O(observations)
    // 轻量），再并入 Pan 域投影（从 BSP pan_div 证书投影，不重判结构或力度），统一 (interval,key) 排序。
    let mut observations = cand_event::reduce_structural_legs(legs);
    observations.extend(cand_event::pan_observations_for_level(level, &pan_divs));
    observations.sort_by_key(|observation| (observation.interval, observation.key));

    #[cfg(feature = "issue1087_parity")]
    if issue1087_probe::active() {
        issue1087_probe::record(
            level,
            centers,
            segments,
            anchor_dirs,
            departure_ends,
            blocks,
            hist,
            dif,
            closes_tick,
            close_src,
            gauge,
            strokes,
            &points,
            &pan_divs,
            &grades,
            &observations,
        );
    }

    debug_assert!(
        {
            let mut full_grades = Vec::new();
            let (full_pts, full_pans) = signal::extract_signals_with_hist_anchored(
                centers,
                segments,
                anchor_dirs,
                departure_ends,
                hist,
                dif,
                closes_tick,
                close_src,
                gauge,
                strokes,
                &mut full_grades,
            );
            // #885：全量对拍入口 level=None ⟹ 记录 level 为占位 0；对拍内容 = 判定本体
            // （坐标/方向/中枢身份/grade），level 由本 resume 入口真实级别统一盖章后比对。
            for g in &mut full_grades {
                g.level = level;
            }
            // 候选域全量神谕（生产外、神谕+测试专用）：数组门查。
            let full_obs = cand_event::observations_for_level(
                level,
                centers,
                blocks,
                segments,
                anchors,
                close_src,
                &full_pans,
            );
            points == full_pts
                && pan_divs == full_pans
                && grades == full_grades
                && observations == full_obs
        },
        "3a 合并扫描破裂：合并扫描四件输出 != (全量 BSP 神谕, 全量候选神谕)（frontier 冻结/push 序/逐点门/归约不变式被违反）"
    );
    MergedScanOutput {
        points,
        pan_divs,
        grades,
        observations,
    }
}

/// ★单段判定核（共享 per-segment 素材）：一次判定喂 BSP 域与候选域两个 sink。
///
/// 与 [`signal::judge_segment`]（full/resume 共享的第一/盘整/三类判定）逐字段同构，唯一差异：
/// 结构门 [`signal::first_structural_gates`] 在此只判一次——BSP 第一类经
/// [`signal::judge_first_from_gates`] 消费、候选结构腿经 [`cand_event::make_trend_observation`]
/// 消费（#1057 Q1「共享 first_structural_gates」）。
#[allow(clippy::too_many_arguments)]
fn merged_judge_segment(
    i: usize,
    seg: &Segment,
    c_idx: usize,
    gate_dir: Option<(usize, Direction)>,
    kind_consol: bool,
    sorted: &[Segment],
    anchors: &[Option<Direction>],
    anchors_self: &[Option<Direction>],
    centers_sorted: &[Center],
    a_seg_cache: &mut ASegCache,
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
    gauge: DivergenceGauge,
    strokes: &[Stroke],
    level: u32,
    departure_ends: Option<&[usize]>,
    points: &mut Vec<BspPoint>,
    pan_divs: &mut Vec<PanDivCert>,
    grade_sink: &mut Vec<FirstClassGradeRecord>,
    legs: &mut Vec<CandidateObservation>,
) {
    let c = &centers_sorted[c_idx];
    // ★#1028 裁定 A：本段的 departure 单元终点（调用方预算，索引随 sorted 平行）。
    let departure_end = departure_ends.map(|d| d[i]);

    // 第一类（趋势背驰，A/B/C 框架）+ 候选结构腿：仅段所在趋势块触发。
    if let Some((pos, dir)) = gate_dir {
        let prev_center = &centers_sorted[pos - 1];
        // 热点②修复：A 区间按 last_center_idx=c_idx 缓存（逐点口径 pos==c_idx ⟹ prev_center=
        // centers[c_idx-1]）。首次 miss 才调 `locate_departure_move_a` O(窗口)，后续 hit O(1)。
        // ★p117 037:20（裁定 T3）：b 包络（`move_range_envelope`，单一来源）随 I(A) 同槽缓存。
        let a_seg_entry = *a_seg_cache.entry(c_idx).or_insert_with(|| {
            locate_departure_move_a(sorted, anchors_self, prev_center, c, dir)
                .and_then(|span| move_range_envelope(sorted, span).map(|env| (span, env)))
        });
        // Q5 λ_C（codex ac4 #2）：当前 episode 首同向段起点（共享 helper，逐段计算）。
        let c_start_entry = departure_move_c_start(sorted, anchors_self, c, dir, seg.start_index);
        // ★共享结构门（BSP 第一类与候选结构腿同读一次；#1057 Q1）。
        let gates = signal::first_structural_gates(
            c,
            dir,
            seg,
            anchors_self[i],
            close_src,
            a_seg_entry,
            c_start_entry,
        );
        if let Some(gates) = gates {
            // BSP 域 sink：第一类（extreme ⟹ 力度确认 ⟹ 产点；未破极值/不可映射 ⟹ 不产点）。
            if let Some(pf) = signal::judge_first_from_gates(
                gates,
                c,
                dir,
                seg,
                departure_end,
                hist,
                dif,
                closes_tick,
                gauge,
                strokes,
                sorted,
                Some(level),
                grade_sink,
            ) {
                points.push(pf);
            }
            // 候选域 sink：结构宽候选腿（未归约；归约留到扫描末尾）。
            legs.push(cand_event::make_trend_observation(
                level,
                prev_center,
                c,
                seg,
                &gates,
            ));
        }
    }

    // ★Q4 盘整背驰（task #145）：段的最近中枢按 ownership 落在本级别 Consolidation 块 ⟹ 走
    // 盘整背驰证书路径（不产 BspPoint、不置 six-bit）。
    if kind_consol {
        if let Some(cert) =
            signal::judge_pan_div(c, seg, sorted, anchors_self, hist, dif, close_src)
        {
            pan_divs.push(cert);
        }
    }

    // 第三类：当前段作 retest，前一段作 leave。
    if i > 0 {
        let leave_seg = &sorted[i - 1];
        if let Some(c_leave_idx) =
            signal::nearest_confirmed_center_idx(centers_sorted, leave_seg.start_index)
        {
            let c_leave = &centers_sorted[c_leave_idx];
            let first_retrace_pair = i == 1 || sorted[i - 2].start_index < c_leave.end_index;
            if first_retrace_pair {
                if let Some(p) = signal::judge_third_cert(c_leave, leave_seg, anchors[i - 1], seg)
                    .map(|cert| cert.point)
                {
                    points.push(p);
                }
            }
        }
    }
}
