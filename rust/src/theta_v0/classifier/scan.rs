//! 3a 生产单扫描（SPEC #1077 D1）：一趟段扫描合并 BSP 三投影与候选观察两路。
//!
//! 生产门查走逐点口径（[`decompose::center_own_dir_at`]/[`decompose::center_block_kind_at`]/
//! [`decompose::center_block_lift_at`]）；批式数组法（`center_trend_gate`）只活在全量神谕
//! （[`signal::extract_signals_with_hist_anchored`]），不产生交易决策（#1057 Q2）。P1 镜像
//! （`recursive_tower::level_cand_delta`）已随 3b 退役（ADR 0026 裁定三：镜像签收在先），其
//! CandDeltaEvent 谓词闭包职责由本模块 [`project_cand_delta_events`] 纯投影接任（#1059/#1060）。
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
//! ★D2/D3 字段族边界（#1057 补接 + #1059 路线 A，#1060）：CandDeltaEvent 的快照字段
//! （c_interval_full/b_parent/c_structure/third_class_in_c/cp_certificate_confirm_src/
//! full_trend_c_qualified/full_trend_evidence）与 pan_div_diag 在 3a 期间由 P1 谓词闭包层
//! （`level_cand_delta`/`cp_event_objects`）独立计算；3b 纯投影（统一记录 + `LevelState.cp_ownership`，
//! #1059 路线 A）落地——[`project_cand_delta_events`] 以生产逐段素材 + cp_ownership 为输入合成，
//! 生产热路径不物化 CandDeltaEvent。本扫描冻结的素材装全生产两域所需字段：P1 谓词层字段
//! （cand_delta=bits.buy1/sell1、confirm_src=source_index、side=struct_break_dir 均可从
//! BspPoint 派生）与区间族（interval/a_interval/c_episode_start/enter_src 均落候选腿
//! key/interval 与 PanDivCert 内），冻结不裁。

use super::super::types::{Center, Direction, MoveKind, Segment, Side, Stroke, Tick};
use super::cand_event::{self, CandidateKind, CandidateObservation};
use super::decompose::{
    center_block_kind, center_block_kind_at, center_block_lift_at, center_own_dir_at, decompose,
    MoveBlock,
};
use super::divergence::{
    departure_move_c_start, locate_departure_move_a, move_range_envelope, DivergenceGauge,
};
use super::recursive_tower::{
    full_trend_c_qualification, full_trend_qualification_evidence, CandDeltaCpEdge, CandDeltaEvent,
    CpScanOwnership, CpStructureIdentity, FullTrendCQualified, FullTrendQualificationEvidence,
    LeveledMove, ParentCenterIdentity, ThirdClassInCp,
};
use super::signal::{self, BspPoint, FirstClassGradeRecord, PanDivCert};
use std::collections::HashMap;

/// Wire* 类型族 + 生产→wire 渲染层（#1177 自 scan.rs 分离，零行为）：渲染器单源、不复制
/// （ADR 0026 N-3）；探针状态机留在 [`issue1087_probe`]，装配逻辑留在本文件。
#[cfg(feature = "issue1087_parity")]
pub(crate) mod wire;

/// #1087 重型签收只读观测面（探针状态机）：begin/finish/record 捕获 Rust 生产四件输出与
/// 逐段未后处理 sink。Wire* 类型族 + 渲染层已迁至 [`wire`]（#1177，零行为）。Lean 从 sink
/// 独立重算排序、归约、Pan 投影和 FNV 身份；不调用 Rust legacy oracle。
/// feature 关闭时不编译，显式 begin 才提取。
#[cfg(feature = "issue1087_parity")]
pub(crate) mod issue1087_probe {
    use super::super::descend::RMove;
    use super::super::recursive_tower::CpScanOwnership;
    use super::wire::{
        wire_output, EpisodeCase, ScanParityRecord, WireASegmentEnvelope, WireCandDeltaCase,
        WireCandidateKind, WireCpClosureEvidence, WireCpScanBase, WireCpTransition, WireDirection,
        WireEventRawContext, WirePreludeInput, WirePreludeOutput, WireScanSinkEmission,
        WireSegmentRow, WireSide, WireUnitMoveFact,
    };
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    };

    static ACTIVE: AtomicBool = AtomicBool::new(false);
    static RECORDS: Mutex<Vec<ScanParityRecord>> = Mutex::new(Vec::new());
    static CP_TRANSITIONS: Mutex<Vec<WireCpTransition>> = Mutex::new(Vec::new());
    pub fn begin() {
        RECORDS.lock().expect("#1087 probe mutex poisoned").clear();
        CP_TRANSITIONS
            .lock()
            .expect("#1087 cp probe mutex poisoned")
            .clear();
        ACTIVE.store(true, Ordering::SeqCst);
    }
    pub fn finish() -> Vec<ScanParityRecord> {
        ACTIVE.store(false, Ordering::SeqCst);
        let mut records = std::mem::take(&mut *RECORDS.lock().expect("#1087 probe mutex poisoned"));
        let transitions = std::mem::take(
            &mut *CP_TRANSITIONS
                .lock()
                .expect("#1087 cp probe mutex poisoned"),
        );
        for transition in transitions {
            let level = transition.raw.cp_departure_move_id.level;
            records
                .iter_mut()
                .find(|record| record.level == level)
                .unwrap_or_else(|| panic!("#1087 cp transition 缺 L{level} raw context"))
                .cp_transitions
                .push(transition);
        }
        records
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_cp_transition(
        before: &CpScanOwnership,
        after: &CpScanOwnership,
        cp_departure_move_id: super::super::recursive_tower::ElementId,
        cp_start: usize,
        leave: &Segment,
        retest: &Segment,
        leave_anchor: Option<Direction>,
        leave_move_id: super::super::recursive_tower::ElementId,
        retest_move_id: super::super::recursive_tower::ElementId,
        visible_unit_move_count: usize,
    ) {
        if !ACTIVE.load(Ordering::SeqCst) {
            return;
        }
        CP_TRANSITIONS
            .lock()
            .expect("#1087 cp probe mutex poisoned")
            .push(WireCpTransition {
                before: before.clone(),
                after: after.clone(),
                raw: WireCpClosureEvidence {
                    visible_unit_move_count,
                    cp_departure_move_id,
                    cp_start,
                    leave: leave.clone(),
                    retest: retest.clone(),
                    leave_anchor,
                    leave_move_id,
                    retest_move_id,
                },
            });
    }

    pub(super) fn active() -> bool {
        ACTIVE.load(Ordering::SeqCst)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn record(
        level: u32,
        centers: &[Center],
        segments: &[Segment],
        anchor_dirs: Option<&[Option<Direction>]>,
        departure_ends: Option<&[usize]>,
        cp_scan: &[CpScanOwnership],
        unit_moves: &[super::super::recursive_tower::LeveledMove],
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
        emissions: Vec<WireScanSinkEmission>,
    ) {
        let anchors_self: Vec<Option<Direction>> = segments
            .iter()
            .map(|segment| Some(segment.direction))
            .collect();
        let anchors = anchor_dirs.unwrap_or(&anchors_self);
        let any_trend = blocks.iter().any(|block| block.kind == MoveKind::Trend);
        // CandDelta sidecar consumes every consolidation ownership at this level; `level_lift == 0`
        // is only the merged four-output projection gate and must not narrow the event raw-case domain.
        let any_event_consol = blocks
            .iter()
            .any(|block| block.kind == MoveKind::Consolidation);
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
        let rows: Vec<_> = segments
            .iter()
            .map(|segment| WireSegmentRow {
                direction: segment.direction.into(),
                start_index: segment.start_index,
                end_index: segment.end_index,
                start_price: segment.start_price,
                end_price: segment.end_price,
            })
            .collect();
        let wire_centers: Vec<_> = centers.iter().copied().map(Into::into).collect();
        let wire_blocks: Vec<_> = blocks.iter().copied().map(Into::into).collect();
        let trend_gate: Vec<Option<WireDirection>> = (0..centers.len())
            .map(|index| center_own_dir_at(blocks, index).map(Into::into))
            .collect();
        let first_match_idx = centers
            .iter()
            .map(|center| {
                centers.iter().position(|candidate| {
                    candidate.end_index == center.end_index
                        && candidate.zd == center.zd
                        && candidate.zg == center.zg
                })
            })
            .collect();
        let a_segments = centers
            .iter()
            .enumerate()
            .map(|(index, center)| {
                let direction = center_own_dir_at(blocks, index)?;
                let previous = centers.get(index.checked_sub(1)?)?;
                locate_departure_move_a(segments, &anchors_self, previous, center, direction)
                    .and_then(|span| {
                        move_range_envelope(segments, span).map(|(low, high)| {
                            WireASegmentEnvelope {
                                span: span.into(),
                                low,
                                high,
                            }
                        })
                    })
            })
            .collect();
        let prelude_input = WirePreludeInput {
            rows: rows.clone(),
            centers: wire_centers,
            blocks: wire_blocks,
        };
        let prelude_output = WirePreludeOutput {
            segments_sorted: segments
                .windows(2)
                .all(|pair| pair[0].start_index <= pair[1].start_index),
            centers_sorted: centers
                .windows(2)
                .all(|pair| pair[0].end_index < pair[1].end_index),
            anchors: anchors.iter().map(|value| value.map(Into::into)).collect(),
            trend_gate,
            first_match_idx,
            a_segments,
        };
        let event_context = WireEventRawContext {
            centers: prelude_input.centers.clone(),
            rows: rows.clone(),
            anchors: anchors.iter().map(|value| value.map(Into::into)).collect(),
            cp_scan: cp_scan
                .iter()
                .map(|scan| WireCpScanBase {
                    b_center_index: scan.b_center_index,
                    b_center_id: scan.b_center_id,
                    b_center: scan.b_center.into(),
                    departure_move_id: scan.departure_move_id,
                    departure_interval: scan.departure_interval.map(Into::into),
                })
                .collect(),
            unit_moves: unit_moves
                .iter()
                .map(|movement| {
                    let (low, high) = movement.envelope();
                    let center = match &movement.rmove {
                        RMove::Compose { centers, .. } => centers.last().copied().map(Into::into),
                        RMove::Segment { .. } => None,
                    };
                    WireUnitMoveFact {
                        id: movement.id,
                        start_index: movement.start_index,
                        end_index: movement.end_index,
                        low,
                        high,
                        center,
                    }
                })
                .collect(),
        };

        // event 之前的 raw Trend 腿是验收输入；Rust CandDeltaEvent 只作为被检侧。
        let cand_events = project_cand_delta_events(
            level,
            centers,
            cp_scan,
            segments,
            Some(unit_moves),
            anchor_dirs,
            departure_ends,
            hist,
            dif,
            closes_tick,
            close_src,
            gauge,
            strokes,
        );
        let mut cand_delta_cases: Vec<WireCandDeltaCase> = emissions
            .iter()
            .flat_map(|emission| emission.candidate_legs.iter())
            .filter(|leg| leg.kind == WireCandidateKind::Trend)
            .filter_map(|leg| {
                let segment_index = segments
                    .iter()
                    .position(|segment| segment.end_index == leg.interval.right)?;
                let segment = segments.get(segment_index)?;
                let center_index = centers.iter().position(|center| {
                    center.start_index == leg.key.parent.center_start
                        && center.zd == leg.key.parent.zd
                        && center.zg == leg.key.parent.zg
                })?;
                let center = centers.get(center_index)?;
                let side = match leg.key.side {
                    WireSide::Long => Side::Long,
                    WireSide::Short => Side::Short,
                };
                let divergence_confirm_src = departure_ends
                    .and_then(|values| values.get(segment_index).copied())
                    .unwrap_or(segment.end_index);
                // buy/sell 来自 event 之前的 BSP 判据输出，不从最终 cand_delta 反推。
                let point = points.iter().find(|point| {
                    point.source_index == divergence_confirm_src
                        && point.struct_break_dir == Some(side)
                });
                let buy1 = point.is_some_and(|point| point.bits.buy1);
                let sell1 = point.is_some_and(|point| point.bits.sell1);
                // level_cand_delta 的事件侧车按完整 consolidation ownership 诊断，
                // 不套 merged 四输出的 lift==0 投影门。
                let kind_consol = any_event_consol
                    && center_block_kind_at(blocks, center_index) == Some(MoveKind::Consolidation);
                let pan_div_diag = kind_consol
                    && signal::judge_pan_div_observation(
                        center,
                        segment,
                        segments,
                        &anchors_self,
                        hist,
                        dif,
                        close_src,
                    )
                    .is_some();
                Some(WireCandDeltaCase {
                    kind: WireCandidateKind::Trend,
                    side: leg.key.side,
                    divergence_confirm_src,
                    a_interval: leg.key.seg_a,
                    center_index,
                    segment_index,
                    structural_candidate: true,
                    direction: leg.structural_predicates.direction,
                    comparable: leg.structural_predicates.comparable,
                    extreme: leg.structural_predicates.extreme,
                    buy1,
                    sell1,
                    pan_diverges: false,
                    pan_div_diag,
                })
            })
            .collect();
        // Pan 诊断事件也从 event 前的 observation 判据输入枚举；不得只遍历已产 event。
        for (segment_index, segment) in segments.iter().enumerate() {
            let Some(center_index) =
                signal::nearest_confirmed_center_idx(centers, segment.start_index)
            else {
                continue;
            };
            // 与 level_cand_delta 的 Pan event 前门逐字同口径；lift==0 只属于 merged 四输出。
            let kind_consol = any_event_consol
                && center_block_kind_at(blocks, center_index) == Some(MoveKind::Consolidation);
            if !kind_consol {
                continue;
            }
            let center = &centers[center_index];
            let Some(cert) = signal::judge_pan_div_observation(
                center,
                segment,
                segments,
                &anchors_self,
                hist,
                dif,
                close_src,
            ) else {
                continue;
            };
            cand_delta_cases.push(WireCandDeltaCase {
                kind: WireCandidateKind::Pan,
                side: cert.side.into(),
                divergence_confirm_src: cert.source_index,
                a_interval: cert.seg_a.into(),
                center_index,
                segment_index,
                structural_candidate: true,
                direction: true,
                comparable: true,
                extreme: true,
                buy1: false,
                sell1: false,
                pan_diverges: true,
                pan_div_diag: true,
            });
        }

        RECORDS
            .lock()
            .expect("#1087 probe mutex poisoned")
            .push(ScanParityRecord {
                level,
                rust_output: wire_output(points, pan_divs, grades, observations),
                emissions,
                prelude_input,
                prelude_output,
                event_context,
                rows,
                episode_cases,
                cand_delta_cases,
                cand_delta_events: cand_events,
                cp_transitions: Vec::new(),
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

/// 为 #1087 提取一次 fresh 生产扫描的逐段原始 sink。这里只调用合并扫描的单段核，
/// 不调用 `extract_signals_with_hist_anchored` 或 `observations_for_level` legacy oracle。
#[cfg(feature = "issue1087_parity")]
#[allow(clippy::too_many_arguments)]
fn issue1087_scan_sink_emissions(
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
) -> Vec<wire::WireScanSinkEmission> {
    let anchors_self: Vec<Option<Direction>> = segments
        .iter()
        .map(|segment| Some(segment.direction))
        .collect();
    let anchors = anchor_dirs.unwrap_or(&anchors_self);
    let any_trend = blocks.iter().any(|block| block.kind == MoveKind::Trend);
    let any_consol = blocks
        .iter()
        .any(|block| block.kind == MoveKind::Consolidation && block.level_lift == 0);
    let mut a_seg_cache: ASegCache = HashMap::new();
    let mut emissions = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        let Some(center_index) = signal::nearest_confirmed_center_idx(centers, segment.start_index)
        else {
            continue;
        };
        let gate_dir = if any_trend {
            center_own_dir_at(blocks, center_index).map(|direction| (center_index, direction))
        } else {
            None
        };
        let kind_consol = any_consol
            && center_block_kind_at(blocks, center_index) == Some(MoveKind::Consolidation)
            && center_block_lift_at(blocks, center_index) == Some(0);
        let mut points = Vec::new();
        let mut pan_divs = Vec::new();
        let mut grades = Vec::new();
        let mut candidate_legs = Vec::new();
        merged_judge_segment(
            index,
            segment,
            center_index,
            gate_dir,
            kind_consol,
            segments,
            anchors,
            &anchors_self,
            centers,
            &mut a_seg_cache,
            hist,
            dif,
            closes_tick,
            close_src,
            gauge,
            strokes,
            level,
            departure_ends,
            &mut points,
            &mut pan_divs,
            &mut grades,
            &mut candidate_legs,
        );
        emissions.push(wire::wire_emission(
            &points,
            &pan_divs,
            &grades,
            &candidate_legs,
        ));
    }
    emissions
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
    #[cfg(feature = "issue1087_parity")] cp_scan: &[super::recursive_tower::CpScanOwnership],
    #[cfg(feature = "issue1087_parity")] unit_moves: &[super::recursive_tower::LeveledMove],
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

    // #1080 S2 Phase 3：只在显式捕获会话内复制进入回缩守卫前的四缓存。关闭时为 None，
    // 不分配、不克隆，也不改变下面任何缓存推进/排序语义。
    let capture_cache_before = super::diag::s2_mirror_capture::data_capture_enabled().then(|| {
        super::diag::s2_mirror_capture::FourCacheCapture {
            points: cached_pts.clone(),
            pan_divs: cached_pans.clone(),
            grades: cached_grades.clone(),
            candidate_legs: cached_legs.clone(),
            cached_count: *cached_count,
        }
    });

    // 单调守卫（confirmed 前缀单调非降 ⟹ 正常永不触发；cascade 前缀回缩由 caller 别处 clear +
    // 本守卫兜底）：缓存越过本 bar 冻结边界 ⟹ 保守清空重判（四件产出锁步清空）。
    if *cached_count > stable_seg {
        cached_pts.clear();
        cached_pans.clear();
        cached_grades.clear();
        cached_legs.clear();
        *cached_count = 0;
    }
    let confirmed_base_lengths = (
        cached_pts.len(),
        cached_pans.len(),
        cached_grades.len(),
        cached_legs.len(),
    );

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

    let capture_confirmed_append = capture_cache_before.as_ref().map(|_| {
        super::diag::s2_mirror_capture::ScanEmissionCapture {
            points: cached_pts[confirmed_base_lengths.0..].to_vec(),
            pan_divs: cached_pans[confirmed_base_lengths.1..].to_vec(),
            grades: cached_grades[confirmed_base_lengths.2..].to_vec(),
            candidate_legs: cached_legs[confirmed_base_lengths.3..].to_vec(),
            observations: Vec::new(),
        }
    });
    let capture_cache_after =
        capture_cache_before
            .as_ref()
            .map(|_| super::diag::s2_mirror_capture::FourCacheCapture {
                points: cached_pts.clone(),
                pan_divs: cached_pans.clone(),
                grades: cached_grades.clone(),
                candidate_legs: cached_legs.clone(),
                cached_count: *cached_count,
            });

    // 结果 = confirmed 前缀产出（缓存 clone）+ frontier tail 产出（每 bar 重判，tail 小）。push 序拼接。
    let mut points = cached_pts.clone();
    let mut pan_divs = cached_pans.clone();
    let mut grades = cached_grades.clone();
    let mut legs = cached_legs.clone();
    let tail_base_lengths = (points.len(), pan_divs.len(), grades.len(), legs.len());
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
    let capture_tail = capture_cache_before.as_ref().map(|_| {
        super::diag::s2_mirror_capture::ScanEmissionCapture {
            points: points[tail_base_lengths.0..].to_vec(),
            pan_divs: pan_divs[tail_base_lengths.1..].to_vec(),
            grades: grades[tail_base_lengths.2..].to_vec(),
            candidate_legs: legs[tail_base_lengths.3..].to_vec(),
            observations: Vec::new(),
        }
    });

    points.sort_by_key(|p: &BspPoint| p.source_index);
    pan_divs.sort_by_key(|p: &PanDivCert| p.source_index);
    grades.sort_by_key(|g: &FirstClassGradeRecord| g.source_index);

    // 候选域归约层（merged/首证钟/state）留在扫描之后：prefix+tail 腿全量重跑归约（O(observations)
    // 轻量），再并入 Pan 域投影（从 BSP pan_div 证书投影，不重判结构或力度），统一 (interval,key) 排序。
    let captured_legs =
        super::diag::s2_mirror_capture::data_capture_enabled().then(|| legs.clone());
    let mut observations = cand_event::reduce_structural_legs(legs);
    if let Some(captured_legs) = captured_legs {
        super::diag::s2_mirror_capture::record_reduction(level, captured_legs, &observations);
    }
    observations.extend(cand_event::pan_observations_for_level(level, &pan_divs));
    observations.sort_by_key(|observation| (observation.interval, observation.key));

    #[cfg(feature = "issue1087_parity")]
    if issue1087_probe::active() {
        let emissions = issue1087_scan_sink_emissions(
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
        );
        issue1087_probe::record(
            level,
            centers,
            segments,
            anchor_dirs,
            departure_ends,
            cp_scan,
            unit_moves,
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
            emissions,
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
    let output = MergedScanOutput {
        points,
        pan_divs,
        grades,
        observations,
    };
    if let (Some(cache_before), Some(confirmed_append), Some(cache_after), Some(tail)) = (
        capture_cache_before,
        capture_confirmed_append,
        capture_cache_after,
        capture_tail,
    ) {
        super::diag::s2_mirror_capture::record_frontier(
            super::diag::s2_mirror_capture::FrontierCapture {
                level,
                prefix_count,
                dirty_e,
                freeze_boundary_src: e_src,
                stable_seg,
                segment_count: segments.len(),
                centers: centers.to_vec(),
                segments: segments.to_vec(),
                anchors: anchors.to_vec(),
                anchor_dirs: anchor_dirs.map(<[Option<Direction>]>::to_vec),
                departure_ends: departure_ends.map(<[usize]>::to_vec),
                blocks: blocks.to_vec(),
                cache_before,
                confirmed_append,
                cache_after,
                tail,
                final_output: super::diag::s2_mirror_capture::ScanEmissionCapture {
                    points: output.points.clone(),
                    pan_divs: output.pan_divs.clone(),
                    grades: output.grades.clone(),
                    candidate_legs: Vec::new(),
                    observations: output.observations.clone(),
                },
            },
            hist,
            dif,
            closes_tick,
            close_src,
            gauge,
            strokes,
        );
    }
    // #1080 S2：默认关闭的只读验收捕获。只复制本次 3a 原始输入/四产口；位于 07b 二类
    // 并入前，不参与判定、排序、缓存或返回值。
    super::diag::s2_mirror_capture::record_scan(
        level,
        centers,
        segments,
        anchors,
        departure_ends,
        blocks,
        &output.points,
        &output.pan_divs,
        &output.grades,
        &output.observations,
    );
    output
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
                let cert = signal::judge_third_cert(c_leave, leave_seg, anchors[i - 1], seg);
                super::diag::s2_mirror_capture::record_third_assembly(
                    level,
                    c_leave,
                    leave_seg,
                    anchors[i - 1],
                    seg,
                    cert.as_ref().map(|cert| &cert.point),
                );
                if let Some(cert) = cert {
                    points.push(cert.point);
                }
            }
        }
    }
}

// ═══════════════════════ 3b CandDeltaEvent 纯投影（#1059 路线 A / #1060） ═══════════════════════
//
// P1 对照装配线（`recursive_tower::level_cand_delta` 独立扫描 + `cp_event_objects` 快照合成）
// 随 3b 退役（ADR 0026 裁定三：镜像签收在先、退役在后，无空窗期）。CandDeltaEvent 改为库内
// 纯函数投影：统一提取记录（生产合并扫描逐段素材 = 第一类 BspPoint + 候选结构腿）与生产侧
// `LevelState.cp_ownership`（pipeline.rs:63，Pending→Closed 已在生产推进）合成（#1059/#1060）。
// 审计 bin 自重放时调用本投影；生产热路径不物化 CandDeltaEvent（post-3b 零成本）。
//
// 判据单一：#1057 Q2 生产门查逐点口径（center_own_dir_at/center_block_kind_at/lift_at）——
// 谓词层字段（cand_delta=bits.buy1∨sell1、confirm_src=source_index、side=struct_break_dir）与
// 区间族（interval/a_interval/c_episode_start/enter_src 均落候选腿 key/interval 与 PanDivCert）
// 全部从生产合并扫描逐段素材读回，不再重判第一类（P1 的 judge_first_cached 第三趟扫描随退役
// 删除）。盘整背驰诊断事件（pan_div_diag，cand_delta=false，非门）与 c_p 快照合成
// （cp_event_projection：逐段可见窗口径）随 P1 谓词闭包职责移交本投影（#1060 裁定三：逐 bar
// 因果诊断保留、转镜像锚定——Lean `recomputeCpEventProjection` 独立重算本合成面）。

/// 单段投影素材：该段生产第一类点（无则 None）+ 该段候选结构腿（无则 None）。
struct SegmentScanMaterial {
    c_idx: usize,
    first_class_point: Option<BspPoint>,
    trend_leg: Option<CandidateObservation>,
}

/// 级别 ℓ 的 Cand^δ 事件纯投影（P1 谓词闭包职责的 3b 接任者；替代已退役的
/// `recursive_tower::level_cand_delta`）。
///
/// 入参口径与生产合并扫描（[`merged_scan_resume`]）逐项同源：centers/segments/anchor_dirs/
/// departure_ends/hist/dif/closes_tick/close_src/gauge/strokes 与管线层一致；`cp_scan` =
/// `LevelState.cp_ownership`；`unit_moves` = 本级输入塔快照。事件枚举 = 生产逐段素材读回
/// （第一类点 + 候选腿）+ 盘整诊断段（批量 ownership 口径，与旧 P1 前门逐字同口径）+
/// c_p 快照合成；排序键 = (interval, a_interval, enter_src, side, cand_delta, pan_div_diag)
/// （旧口径不变）。生产路径 segments/centers 已有序（parser 账本 + 非重叠中枢扫描）；乱序入参
/// 仍走旧 prelude 排序守卫（诊断面防御，与生产判定无关）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn project_cand_delta_events(
    level: u32,
    centers: &[Center],
    cp_scan: &[CpScanOwnership],
    segments: &[Segment],
    unit_moves: Option<&[LeveledMove]>,
    anchor_dirs: Option<&[Option<Direction>]>,
    departure_ends: Option<&[usize]>,
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
    gauge: DivergenceGauge,
    strokes: &[Stroke],
) -> Vec<CandDeltaEvent> {
    // ── prelude 排序守卫（与旧 level_cand_delta / signal.rs full 路径逐行同构） ──
    let sorted_owned: Vec<Segment>;
    let anchors_perm: Vec<Option<Direction>>;
    let dep_ends_perm: Vec<usize>;
    let (sorted, anchors_in, dep_ends_in): (
        &[Segment],
        Option<&[Option<Direction>]>,
        Option<&[usize]>,
    ) = if segments
        .windows(2)
        .all(|w| w[0].start_index <= w[1].start_index)
    {
        (segments, anchor_dirs, departure_ends)
    } else {
        let mut idx: Vec<usize> = (0..segments.len()).collect();
        idx.sort_by_key(|&i| segments[i].start_index);
        sorted_owned = idx.iter().map(|&i| segments[i].clone()).collect();
        match (anchor_dirs, departure_ends) {
            (Some(a), Some(d)) => {
                anchors_perm = idx.iter().map(|&i| a[i]).collect();
                dep_ends_perm = idx.iter().map(|&i| d[i]).collect();
                (
                    &sorted_owned[..],
                    Some(&anchors_perm[..]),
                    Some(&dep_ends_perm[..]),
                )
            }
            (Some(a), None) => {
                anchors_perm = idx.iter().map(|&i| a[i]).collect();
                (&sorted_owned[..], Some(&anchors_perm[..]), None)
            }
            (None, Some(d)) => {
                dep_ends_perm = idx.iter().map(|&i| d[i]).collect();
                (&sorted_owned[..], None, Some(&dep_ends_perm[..]))
            }
            (None, None) => (&sorted_owned[..], None, None),
        }
    };
    let anchors_self: Vec<Option<Direction>> = sorted.iter().map(|s| Some(s.direction)).collect();
    let anchors: &[Option<Direction>] = anchors_in.unwrap_or(&anchors_self);
    debug_assert_eq!(
        anchors.len(),
        sorted.len(),
        "anchor_dirs 与 segments 必等长"
    );

    let centers_owned: Vec<Center>;
    let centers_sorted: &[Center] = if centers.windows(2).all(|w| w[0].end_index <= w[1].end_index)
    {
        centers
    } else {
        centers_owned = {
            let mut v = centers.to_vec();
            v.sort_by_key(|c| c.end_index);
            v
        };
        &centers_owned
    };

    // blocks = 全量分解（decompose ≡ 管线层增量续折输出，decompose.rs 定义性等价 ⟹ bit-exact）。
    let blocks = decompose(centers_sorted);
    let center_kind = center_block_kind(centers_sorted.len(), &blocks);
    let any_trend = blocks.iter().any(|b| b.kind == MoveKind::Trend);
    let any_consol = center_kind
        .iter()
        .any(|k| *k == Some(MoveKind::Consolidation));
    // #898：生产盘背只认本级别盘整块（lift==0）；投影的盘整**诊断**事件沿用旧 P1 前门口径
    // （批量 ownership、不套 lift 过滤——诊断面与生产四输出面分离，镜像已按同口径独立重算）。
    let any_consol_lift0 = blocks
        .iter()
        .any(|b| b.kind == MoveKind::Consolidation && b.level_lift == 0);

    // ── 生产合并扫描逐段素材（同一 single-judgment 核 merged_judge_segment，判据零分叉） ──
    let mut a_seg_cache: ASegCache = HashMap::new();
    let mut materials: Vec<Option<SegmentScanMaterial>> = Vec::with_capacity(sorted.len());
    for (i, seg) in sorted.iter().enumerate() {
        let Some(c_idx) = signal::nearest_confirmed_center_idx(centers_sorted, seg.start_index)
        else {
            materials.push(None);
            continue;
        };
        let gate_dir = if any_trend {
            center_own_dir_at(&blocks, c_idx).map(|d| (c_idx, d))
        } else {
            None
        };
        let kind_consol = any_consol_lift0
            && center_block_kind_at(&blocks, c_idx) == Some(MoveKind::Consolidation)
            && center_block_lift_at(&blocks, c_idx) == Some(0);
        let mut points = Vec::new();
        let mut pan_divs = Vec::new();
        let mut grades = Vec::new();
        let mut legs = Vec::new();
        merged_judge_segment(
            i,
            seg,
            c_idx,
            gate_dir,
            kind_consol,
            sorted,
            anchors,
            &anchors_self,
            centers_sorted,
            &mut a_seg_cache,
            hist,
            dif,
            closes_tick,
            close_src,
            gauge,
            strokes,
            level,
            dep_ends_in,
            &mut points,
            &mut pan_divs,
            &mut grades,
            &mut legs,
        );
        // 第三类点 struct_break_dir=None（make_third_point 语义）；第一类点恒 Some ⟹ 此过滤即
        // 第一类素材。kind_consol 只影响 pan_divs sink，不影响 points/legs（merged_judge_segment）。
        let first_class_point = points.into_iter().find(|p| p.struct_break_dir.is_some());
        let trend_leg = legs.into_iter().find(|l| l.kind == CandidateKind::Trend);
        materials.push(Some(SegmentScanMaterial {
            c_idx,
            first_class_point,
            trend_leg,
        }));
    }

    // ── 事件枚举（单趟，逐段至多一件；push 序与旧 level_cand_delta 一致） ──
    let mut events: Vec<CandDeltaEvent> = Vec::new();
    for (i, (seg, material)) in sorted.iter().zip(materials.iter()).enumerate() {
        let Some(material) = material else {
            continue;
        };
        let c_idx = material.c_idx;
        // 第一类趋势事件：谓词层字段 + 区间族全部从生产逐段素材读回（不重判）。
        if let (Some(point), Some(leg)) = (&material.first_class_point, &material.trend_leg) {
            let lambda_c = leg.key.c_start;
            let a_interval = leg.key.seg_a;
            let side = point
                .struct_break_dir
                .expect("第一类点必携 struct_break_dir（make_first_point 无条件置）");
            let confirm_src = point.source_index;
            let cand_delta = point.bits.buy1 || point.bits.sell1;
            let interval_end = seg.end_index;
            let (
                b_parent,
                c_structure,
                third_class_in_c,
                cp_ownership,
                c_interval_full,
                full_trend_evidence,
                full_trend_c_qualified,
            ) = cp_event_projection(
                level,
                centers_sorted,
                cp_scan,
                sorted,
                anchors,
                unit_moves.unwrap_or(&[]),
                c_idx,
                i,
                interval_end,
                cand_delta,
            );
            let cp_certificate_confirm_src = c_interval_full
                .and_then(|_| third_class_in_c.map(|third| third.point_source_index));
            events.push(CandDeltaEvent {
                level,
                side,
                divergence_confirm_src: confirm_src,
                confirm_src,
                interval: (lambda_c, interval_end),
                a_interval,
                c_episode_start: lambda_c,
                c_episode_interval: (lambda_c, interval_end),
                c_interval_full,
                b_parent,
                c_structure,
                third_class_in_c,
                cp_certificate_confirm_src,
                full_trend_c_qualified,
                full_trend_evidence,
                cp_ownership,
                enter_src: lambda_c,
                cand_delta,
                pan_div_diag: false, // 趋势事件的中枢落 Trend 块 ⟹ 盘整诊断恒 false（旧口径同）
            });
        }
        // 盘整背驰诊断事件（非门）：与旧 level_cand_delta 的 Pan 事件前门逐字同口径
        // （批量 ownership、无 lift 过滤；judge_pan_div_observation 含 #483 不破核心支）。
        if any_consol && center_kind[c_idx] == Some(MoveKind::Consolidation) {
            let c = &centers_sorted[c_idx];
            if let Some(cert) = signal::judge_pan_div_observation(
                c,
                seg,
                sorted,
                &anchors_self,
                hist,
                dif,
                close_src,
            ) {
                events.push(CandDeltaEvent {
                    level,
                    side: cert.side,
                    divergence_confirm_src: cert.source_index,
                    confirm_src: cert.source_index,
                    interval: cert.seg_c,
                    a_interval: cert.seg_a,
                    c_episode_start: cert.seg_c.0,
                    c_episode_interval: cert.seg_c,
                    c_interval_full: None,
                    b_parent: None,
                    c_structure: None,
                    third_class_in_c: None,
                    cp_certificate_confirm_src: None,
                    full_trend_c_qualified: None,
                    full_trend_evidence: None,
                    cp_ownership: None,
                    enter_src: cert.seg_c.0,
                    cand_delta: false,
                    pan_div_diag: true,
                });
            }
        }
    }
    // 确认时点只作诊断，不能直接或间接参与候选排序。
    // 完全相同的结构键保留上游 Segment 的稳定结构顺序。
    events.sort_by_key(|e| {
        (
            e.interval,
            e.a_interval,
            e.enter_src,
            match e.side {
                Side::Long => 0_u8,
                Side::Short => 1_u8,
            },
            e.cand_delta,
            e.pan_div_diag,
        )
    });
    events
}

/// c_p 快照合成（自退役的 `recursive_tower::cp_event_objects` 移交投影；算法逐字保留）。
///
/// 逐段可见窗口径：第三类只在事件段可见窗内首次可证时附着；完整右端只能取第三类 retest
/// 首次可证点。Lean 镜像 `recomputeCpEventProjection`（formal/Origin/ScanAssemblyMirror.lean）
/// 独立重算本合成面——逐 bar 因果诊断转镜像锚定（#1060 裁定三）。
///
/// 第三类 find_map 名分 = **037:18 存在性**（#1278-tail5）：事件段可见窗内全窗后扫「c 至少
/// 包含一个第三类买卖点」（#1081 3b 接任 + #1229 全窗后扫先例），与 **020:62 点位**
/// （advance_cp_lifecycles 固定首对域）判的是两样东西。
#[allow(clippy::too_many_arguments)]
pub(crate) fn cp_event_projection(
    level: u32,
    centers: &[Center],
    cp_scan: &[CpScanOwnership],
    segments: &[Segment],
    anchors: &[Option<Direction>],
    unit_moves: &[LeveledMove],
    c_idx: usize,
    event_seg_idx: usize,
    event_end: usize,
    is_complete_divergence: bool,
) -> (
    Option<ParentCenterIdentity>,
    Option<CpStructureIdentity>,
    Option<ThirdClassInCp>,
    Option<CandDeltaCpEdge>,
    Option<(usize, usize)>,
    Option<FullTrendQualificationEvidence>,
    Option<FullTrendCQualified>,
) {
    let Some(c) = centers.get(c_idx) else {
        return (None, None, None, None, None, None, None);
    };
    let Some(scan) = cp_scan
        .iter()
        .find(|o| o.b_center_index == c_idx && o.b_center == *c)
    else {
        return (None, None, None, None, None, None, None);
    };
    let b = ParentCenterIdentity {
        center_index: scan.b_center_index,
        center_id: scan.b_center_id,
        source_interval: (scan.b_center.start_index, scan.b_center.end_index),
        zd: scan.b_center.zd,
        zg: scan.b_center.zg,
    };
    let (Some(departure_move_id), Some((c_start_full, _))) =
        (scan.departure_move_id, scan.departure_interval)
    else {
        return (Some(b), None, None, None, None, None, None);
    };

    // 第三类判据只调用 signal.rs 的单一真值函数；这里仅增加 B/c 所有权与区间边界。
    let third = (1..=event_seg_idx).find_map(|i| {
        let leave = &segments[i - 1];
        let retest = &segments[i];
        if leave.start_index < c_start_full || retest.end_index > event_end {
            return None;
        }
        if signal::nearest_confirmed_center_idx(centers, leave.start_index) != Some(c_idx) {
            return None;
        }
        let cert = signal::judge_third_cert(c, leave, anchors[i - 1], retest)?;
        Some((i, cert))
    });
    let third_obj = third.and_then(|(i, cert)| {
        Some(ThirdClassInCp {
            b_center_id: scan.b_center_id,
            cp_departure_move_id: departure_move_id,
            departure_move_id: unit_moves.get(i - 1)?.id,
            retest_move_id: unit_moves.get(i)?.id,
            departure_interval: cert.departure_interval,
            retest_interval: cert.retest_interval,
            point_source_index: cert.point.source_index,
            side: if cert.point.bits.buy3 {
                Side::Long
            } else {
                Side::Short
            },
        })
    });
    // 确认时快照的完整右端只能取第三类 retest 首次可证点，禁止取当前/后续 Cand 事件 seg.end。
    let third_inside_component_span = third_obj.is_some_and(|third| {
        third.departure_move_id.level == departure_move_id.level
            && third.retest_move_id.level == departure_move_id.level
            && departure_move_id.ordinal <= third.departure_move_id.ordinal
            && third.departure_move_id.ordinal <= third.retest_move_id.ordinal
            && unit_moves
                .get(event_seg_idx)
                .is_some_and(|end_move| third.retest_move_id.ordinal <= end_move.id.ordinal)
    });
    let c_end_full = (is_complete_divergence && third_inside_component_span).then(|| {
        third_obj
            .expect("third_inside_component_span 蕴含 third_obj Some")
            .retest_interval
            .1
    });
    let terminal_move_id = (is_complete_divergence && third_inside_component_span).then(|| {
        third_obj
            .expect("third_inside_component_span 蕴含 third_obj Some")
            .retest_move_id
    });
    let c_structure = Some(CpStructureIdentity {
        level,
        b_center_id: scan.b_center_id,
        departure_move_id,
        terminal_move_id,
        source_start: c_start_full,
        source_end: c_end_full,
    });
    let c_interval_full = c_end_full.map(|end| (c_start_full, end));
    let edge = is_complete_divergence.then_some(CandDeltaCpEdge {
        b_center_id: scan.b_center_id,
        cp_departure_move_id: departure_move_id,
        cp_source_start: c_start_full,
    });
    // 事件证书只能消费事件时已经存在的走势；尤其不得提前看见 terminal 的未来后继。
    let visible_moves = unit_moves.get(..=event_seg_idx);
    let full_trend_evidence = c_structure.zip(third_obj).zip(visible_moves).and_then(
        |((structure, third), visible_moves)| {
            full_trend_qualification_evidence(
                centers,
                c_idx,
                scan.b_center_id,
                structure,
                third,
                visible_moves,
            )
        },
    );
    let full_trend_c_qualified = c_structure.zip(third_obj).zip(visible_moves).and_then(
        |((structure, third), visible_moves)| {
            full_trend_c_qualification(
                centers,
                c_idx,
                scan.b_center_id,
                structure,
                third,
                visible_moves,
            )
        },
    );
    (
        Some(b),
        c_structure,
        third_obj,
        edge,
        c_interval_full,
        full_trend_evidence,
        full_trend_c_qualified,
    )
}

#[cfg(test)]
mod projection_tests {
    use super::super::super::config::MacdConfig;
    use super::super::super::types::{Center, Direction, Segment, Side, Tick};
    use super::super::divergence::{compute_macd, DivergenceGauge};
    use super::super::signal::extract_signals_with_hist;
    use super::project_cand_delta_events;

    fn dc(zd: Tick, zg: Tick, dd: Tick, gg: Tick, ei: usize) -> Center {
        Center {
            zd,
            zg,
            dd,
            gg,
            start_index: 0,
            end_index: ei,
        }
    }
    fn seg(direction: Direction, s: usize, e: usize, sp: Tick, ep: Tick) -> Segment {
        Segment {
            direction,
            start_index: s,
            end_index: e,
            start_price: sp,
            end_price: ep,
        }
    }

    /// 投影对拍（fixture 移植自 signal.rs::first_buy_extracted_with_trend_divergence，合成数据；
    /// 原 recursive_tower.rs p1_tests 随 P1 装配线退役迁入）：ℓ0 投影输出与 extract_signals 的
    /// buy1/sell1 背驰确认支逐 bit 一致 + 事件字段见证（P1 硬门退役后投影仍须复现同一事件面）。
    #[test]
    fn projection_bit_exact_with_extract_signals_buy1() {
        let c0 = dc(300, 400, 290, 410, 2);
        let c1 = dc(100, 200, 90, 210, 8);
        let segs = vec![
            seg(Direction::Down, 3, 5, 350, 250),
            seg(Direction::Up, 5, 7, 250, 280),
            seg(Direction::Down, 9, 11, 150, 80), // leave：三卖离开，破 zd=100
            seg(Direction::Up, 11, 13, 80, 90),   // retest：三卖回试，不重回（< zd）
            seg(Direction::Down, 13, 15, 90, 70), // bottom：一买破新低（后扫命中 leave+retest）
        ];
        let prices: Vec<Tick> = vec![
            300, 300, 300, 300, 100, 250, 250, 250, 250, 248, 246, 244, 242, 240, 238, 236,
        ];
        let closes: Vec<f64> = prices.iter().map(|&p| p as f64).collect();
        let src: Vec<usize> = (0..prices.len()).collect();
        let series = compute_macd(&closes, &MacdConfig::default());
        let centers = [c0, c1];
        let (points, _pan) = extract_signals_with_hist(
            &centers,
            &segs,
            &series.hist,
            &series.dif,
            &prices,
            &src,
            DivergenceGauge::MacdArea,
            &[],
            &mut Vec::new(),
        );
        let events = project_cand_delta_events(
            0,
            &centers,
            &[],
            &segs,
            None,
            None,
            None,
            &series.hist,
            &series.dif,
            &prices,
            &src,
            DivergenceGauge::MacdArea,
            &[],
        );
        // 逐 bit：buy1/sell1 背驰确认支 ⟺ cand_delta=true 事件（(src, side) 多重集相等）。
        let mut lhs: Vec<(usize, i8)> = points
            .iter()
            .flat_map(|p| {
                let mut v = Vec::new();
                if p.bits.buy1 {
                    v.push((p.source_index, 1i8));
                }
                if p.bits.sell1 {
                    v.push((p.source_index, -1i8));
                }
                v
            })
            .collect();
        let mut rhs: Vec<(usize, i8)> = events
            .iter()
            .filter(|e| e.cand_delta)
            .map(|e| (e.confirm_src, if e.side == Side::Long { 1i8 } else { -1i8 }))
            .collect();
        lhs.sort_unstable();
        rhs.sort_unstable();
        assert!(!lhs.is_empty(), "fixture 必产 buy1（非空对拍）");
        assert_eq!(
            lhs, rhs,
            "P1 铁律：谓词 cand_delta 与 buy1/sell1 背驰确认支逐 bit 一致"
        );
        assert_eq!(
            events.len(),
            2,
            "两个破中枢结构候选（leave + bottom，#1249 后扫）"
        );
        let e = events
            .iter()
            .find(|e| e.cand_delta)
            .expect("C<A 背驰确认 ⟹ 存在 Cand^δ=true 事件（bottom 段）");
        assert_eq!(e.side, Side::Long);
        assert_eq!(e.confirm_src, 15, "确认时点=完成时（bottom 段端点，裁决③）");
        assert_eq!(
            e.interval,
            (9, 15),
            "I(C) = [λ_C, seg.end]（Q5 区间口径，λ_C=9）"
        );
        assert_eq!(e.a_interval, (3, 5), "I(A) = 前中枢离开 episode");
        assert_eq!(e.enter_src, 9, "兼容别名 = c_episode_start");
        assert!(!e.pan_div_diag, "趋势路径无盘整背驰诊断（盘背当前实装态不入链；0708 文书裁决 2 已被 0716 裁决⑤ supersede，#726）");
    }

    /// cert F-02 回归：非趋势门段（最近中枢按 ownership 落 Consolidation 块）的盘整背驰诊断
    /// 独立可达——fixture 移植自 signal.rs::pan_div_cert_emitted_in_consolidation_block_zero_
    /// first_class_bits。修复前 pan_div_diag 挂在趋势门之后，与盘整 ownership 在同一中枢上
    /// 互斥 ⟹ 恒 false 死分支；修复后产恰一条 cand_delta=false 的**纯诊断**事件（装配器基例
    /// 过滤与链攀升双重跳过 ⟹ 结构性不入链＝实装态边界，0708 文书裁决 2 已被 0716 裁决⑤ supersede，#726）。
    #[test]
    fn pan_div_diag_reachable_in_consolidation_without_trend_gate() {
        let c0 = dc(100, 200, 90, 210, 2);
        let c1 = dc(300, 400, 290, 410, 5); // c0→c1 上涨（趋势块）
        let c2 = dc(350, 450, 250, 460, 8); // c1→c2 扩张 ⟹ c2 按 ownership 落盘整块
        let segs = vec![
            seg(Direction::Down, 9, 11, 460, 330), // A：第一次离开（330 < zd=350 破核心）
            seg(Direction::Up, 11, 13, 330, 380),  // 回中枢段（380 ≥ 350 回核心内侧）
            seg(Direction::Down, 13, 15, 380, 300), // C：第二次离开破核心（C<A 背驰）
        ];
        let prices: Vec<Tick> = vec![
            100, 100, 100, 100, 60, 140, 100, 95, 105, 105, 60, 90, 95, 93, 91, 89,
        ];
        let closes: Vec<f64> = prices.iter().map(|&p| p as f64).collect();
        let src: Vec<usize> = (0..prices.len()).collect();
        let series = compute_macd(&closes, &MacdConfig::default());
        let centers = [c0, c1, c2];
        let events = project_cand_delta_events(
            0,
            &centers,
            &[],
            &segs,
            None,
            None,
            None,
            &series.hist,
            &series.dif,
            &prices,
            &src,
            DivergenceGauge::MacdArea,
            &[],
        );
        // 恰一条纯诊断事件；零 cand_delta=true（诊断不入谓词）。
        assert_eq!(
            events.len(),
            1,
            "盘整块内恰一张 PanDivCert ⟹ 恰一条诊断事件"
        );
        let e = &events[0];
        assert!(
            e.pan_div_diag,
            "cert F-02：盘整背驰诊断可达（修复前死分支恒 false）"
        );
        assert!(
            !e.cand_delta,
            "盘整背驰不入谓词（实装态；0708 文书裁决 2 已被 0716 裁决⑤ supersede，#726）⟹ cand_delta=false（装配器双重跳过 ⟹ 不入链）"
        );
        assert_eq!(e.side, Side::Long, "向下破 ⟹ Long 候选（仅诊断标注）");
        assert_eq!(e.confirm_src, 15, "因果触发点 = 破中枢段端点");
        assert_eq!(e.interval, (13, 15), "I(C) = 当前离开走势区间");
        assert_eq!(e.a_interval, (9, 11), "I(A) = 前一次同向离开末段");
        assert_eq!(e.enter_src, 13, "enter_src = λ_C = I(C) 起点");
    }

    /// #483：24 课 C 不破核心 + 同色柱面积 C<A 也必须抵达纯诊断通道；仍然
    /// cand_delta=false，因而不入链、不置买卖点 bit。
    #[test]
    fn pan_div_diag_reaches_unbroken_core_area_branch_without_entering_chain() {
        let c0 = dc(100, 200, 90, 210, 2);
        let c1 = dc(300, 400, 290, 410, 5);
        let c2 = dc(350, 450, 250, 460, 8);
        let segs = vec![
            seg(Direction::Down, 9, 11, 460, 330),
            seg(Direction::Up, 11, 13, 330, 380),
            seg(Direction::Down, 13, 15, 380, 360), // C：核心内
        ];
        let mut hist = vec![0.0; 16];
        hist[9..=11].copy_from_slice(&[-4.0, -3.0, -2.0]);
        hist[13..=15].copy_from_slice(&[-1.0, -1.0, -1.0]);
        let prices = vec![100; hist.len()];
        let src: Vec<usize> = (0..hist.len()).collect();

        let events = project_cand_delta_events(
            0,
            &[c0, c1, c2],
            &[],
            &segs,
            None,
            None,
            None,
            &hist,
            &[],
            &prices,
            &src,
            DivergenceGauge::MacdArea,
            &[],
        );

        assert_eq!(events.len(), 1, "不破核心面积背驰应产恰一条诊断事件");
        let event = &events[0];
        assert!(event.pan_div_diag, "新分支必须进入 pan_div_diag");
        assert!(!event.cand_delta, "纯诊断事件不得进入 Cand^δ 链");
        assert_eq!(event.interval, (13, 15));
        assert_eq!(event.a_interval, (9, 11));
    }
}
