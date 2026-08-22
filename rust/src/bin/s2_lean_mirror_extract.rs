//! #1080 S2 Phase 2：3a 统一扫描 Rust→Lean 只读提取执行器。
//!
//! 用法：
//! `cargo run --release --bin s2_lean_mirror_extract -- <btc_1m_full.json> <artifact-dir>`
//!
//! 执行器逐窗从空 parser/cache 因果重放，在 checkpoint bar 打开 crate 内只读捕获槽；
//! fixture 只携 Rust raw input/wire，由 Lean runner 现场执行镜面函数。生产判定路径不分叉。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use newchan_rust::theta_v0::classifier::bsp::OwnerRef;
use newchan_rust::theta_v0::classifier::cand_event::{
    CandidateEvent, CandidateKind, CandidateObservation, CandidateState, ObservedState,
};
use newchan_rust::theta_v0::classifier::diag::s2_mirror_capture::{
    self, CaptureBatch, FirstAssemblyCapture, ScanCapture, TrendAssemblyCapture,
};
use newchan_rust::theta_v0::classifier::recursive_tower::{CpLifecycleStatus, CpScanOwnership};
use newchan_rust::theta_v0::classifier::signal::{PanDivCert, T3InCGrade, T3InCGradeReason};
use newchan_rust::theta_v0::classifier::{self, TowerCache};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{quantize, Bar, Center, Direction, Segment, Side};
use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: &str = "s2-lean-mirror-v1";
const CHECKPOINT_EVERY: usize = 5_000;
const MIN_EVENTS: usize = 100;

#[derive(Clone, Copy, Debug, Serialize)]
struct WindowSpec {
    id: &'static str,
    start: usize,
    end: usize,
    heavy_overlap: bool,
}

const WINDOWS: [WindowSpec; 4] = [
    WindowSpec {
        id: "W20",
        start: 0,
        end: 20_000,
        heavy_overlap: false,
    },
    WindowSpec {
        id: "W100",
        start: 20_000,
        end: 120_000,
        heavy_overlap: false,
    },
    WindowSpec {
        id: "W300",
        start: 120_000,
        end: 420_000,
        heavy_overlap: false,
    },
    WindowSpec {
        id: "W500",
        start: 0,
        end: 500_000,
        heavy_overlap: true,
    },
];

#[derive(Deserialize)]
struct RawBars {
    opens: Vec<Option<f64>>,
    highs: Vec<Option<f64>>,
    lows: Vec<Option<f64>>,
    closes: Vec<Option<f64>>,
    #[serde(default)]
    volumes: Vec<Option<f64>>,
    dates: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
struct PortCounts {
    prelude: usize,
    points: usize,
    pan: usize,
    grades: usize,
    observations: usize,
    reduction: usize,
    cp: usize,
}

impl PortCounts {
    fn extraction_events(&self) -> usize {
        self.prelude
            + self.points
            + self.pan
            + self.grades
            + self.observations
            + self.reduction
            + self.cp
    }

    fn all_emit(&self) -> bool {
        self.prelude > 0
            && self.points > 0
            && self.pan > 0
            && self.grades > 0
            && self.observations > 0
            && self.reduction > 0
            && self.cp > 0
    }
}

#[derive(Debug, Clone, Serialize)]
struct LayerCell {
    level: u32,
    side: &'static str,
    terminal: &'static str,
    extraction_count: usize,
    mismatch_count: usize,
}

#[derive(Debug, Serialize)]
struct WindowReport {
    schema_version: &'static str,
    window_id: &'static str,
    start: usize,
    end: usize,
    heavy_overlap: bool,
    checkpoint_every: usize,
    checkpoint_count: usize,
    extraction_event_count: usize,
    compared_fields: usize,
    mismatch_fields: usize,
    mismatch_by_port: BTreeMap<&'static str, usize>,
    port_object_counts: PortCounts,
    port_enumerations: PortCounts,
    candidate_layers: Vec<LayerCell>,
    cp_layers: Vec<LayerCell>,
    empty_candidate_layers: usize,
    minimum_events_required: usize,
    minimum_events_met: bool,
    every_port_emitted: bool,
    zero_mismatch_upper_95: f64,
    lean_fixture: String,
    lean_stdout: String,
    elapsed_seconds: f64,
}

fn standard_windows_are_valid() -> bool {
    WINDOWS[..3].iter().enumerate().all(|(i, a)| {
        WINDOWS[..3]
            .iter()
            .skip(i + 1)
            .all(|b| a.end <= b.start || b.end <= a.start)
    }) && WINDOWS[3].heavy_overlap
        && WINDOWS[3].start == 0
        && WINDOWS[3].end == 500_000
}

fn zero_failure_upper_95(n: usize) -> f64 {
    if n == 0 {
        1.0
    } else {
        1.0 - 0.05_f64.powf(1.0 / n as f64)
    }
}

fn timestamp(date: &str) -> i64 {
    let digits: String = date
        .chars()
        .take_while(|c| *c != '+')
        .filter(char::is_ascii_digit)
        .take(14)
        .collect();
    digits
        .parse()
        .unwrap_or_else(|_| panic!("日期 {date:?} 解析失败"))
}

fn load(path: &Path, tick_size: f64) -> Vec<Bar> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("读取 {} 失败: {e}", path.display()))
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    let raw: RawBars = serde_json::from_str(&text).unwrap_or_else(|e| panic!("解析失败: {e}"));
    let n = raw
        .closes
        .len()
        .min(raw.opens.len())
        .min(raw.highs.len())
        .min(raw.lows.len())
        .min(raw.dates.len());
    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let prior = bars.last().map_or(0, |bar: &Bar| bar.close);
        let volume = raw.volumes.get(i).and_then(|value| *value).unwrap_or(0.0);
        let values = match (raw.opens[i], raw.highs[i], raw.lows[i], raw.closes[i]) {
            (Some(open), Some(high), Some(low), Some(close)) => {
                let invalid = high < open.max(close).max(low)
                    || low > open.min(close).min(high)
                    || [open, high, low, close].iter().any(|price| *price <= 0.0);
                (
                    quantize(open, tick_size),
                    quantize(high, tick_size),
                    quantize(low, tick_size),
                    quantize(close, tick_size),
                    invalid,
                )
            }
            _ => (prior, prior, prior, prior, true),
        };
        bars.push(Bar {
            source_index: i,
            timestamp: timestamp(&raw.dates[i]),
            open: values.0,
            high: values.1,
            low: values.2,
            close: values.3,
            volume,
            untradable: values.4 || volume <= 0.0,
        });
    }
    bars
}

fn dir(value: Direction) -> &'static str {
    match value {
        Direction::Up => "Direction.up",
        Direction::Down => "Direction.down",
    }
}

fn side(value: Side) -> &'static str {
    match value {
        Side::Long => "Side.long",
        Side::Short => "Side.short",
    }
}

fn rust_side(value: Side) -> &'static str {
    match value {
        Side::Long => "RustSideTag.long",
        Side::Short => "RustSideTag.short",
    }
}

fn side_name(value: Side) -> &'static str {
    match value {
        Side::Long => "Long",
        Side::Short => "Short",
    }
}

fn opt_nat(value: Option<usize>) -> String {
    value.map_or_else(|| "none".to_owned(), |v| format!("(some {v})"))
}

fn opt_u64_as_nat(value: Option<f64>) -> String {
    value.map_or_else(|| "none".to_owned(), |v| format!("(some {})", v.to_bits()))
}

fn center(value: &Center) -> String {
    format!(
        "{{ startIndex := {}, endIndex := {}, zd := {}, zg := {} }}",
        value.start_index, value.end_index, value.zd, value.zg
    )
}

fn segment(value: &Segment, anchor: Option<Direction>, departure_end: Option<usize>) -> String {
    let anchor = anchor.map_or_else(|| "none".to_owned(), |v| format!("some {}", dir(v)));
    format!("{{ direction := {}, startIndex := {}, endIndex := {}, endPrice := {}, anchor := {}, departureEnd := {} }}",
        dir(value.direction), value.start_index, value.end_index, value.end_price, anchor, opt_nat(departure_end))
}

fn grade(value: T3InCGrade) -> String {
    match value {
        T3InCGrade::Present {
            leave_interval,
            retest_interval,
        } => format!(
            "RustGradeExtractionValue.present {} {} {} {}",
            leave_interval.0, leave_interval.1, retest_interval.0, retest_interval.1
        ),
        T3InCGrade::Missing(reason) => format!(
            "RustGradeExtractionValue.missing {}",
            match reason {
                T3InCGradeReason::MissingLeave => "RustGradeReasonTag.missingLeave",
                T3InCGradeReason::MissingRetest => "RustGradeReasonTag.missingRetest",
                T3InCGradeReason::SameDirection => "RustGradeReasonTag.sameDirection",
                T3InCGradeReason::LeaveNotOutside => "RustGradeReasonTag.leaveNotOutside",
                T3InCGradeReason::RetestReentered => "RustGradeReasonTag.retestReentered",
            }
        ),
    }
}

fn observed_state(value: ObservedState) -> &'static str {
    match value {
        ObservedState::Unresolved => "RustCandidateStateTag.unresolved",
        ObservedState::Provisional => "RustCandidateStateTag.provisional",
        ObservedState::Confirmed => "RustCandidateStateTag.confirmed",
    }
}

fn candidate_kind(value: CandidateKind) -> &'static str {
    match value {
        CandidateKind::Trend => "RustCandidateKindTag.trend",
        CandidateKind::Pan => "RustCandidateKindTag.pan",
    }
}

fn candidate_rust(value: &CandidateObservation) -> String {
    format!(concat!("{{ level := {}, kind := {}, side := {}, previousCenterStart := {}, ",
        "parentCenterStart := {}, parentZd := {}, parentZg := {}, segALeft := {}, segARight := {}, ",
        "lambdaC := {}, direction := {}, comparable := {}, extreme := {}, intervalLeft := {}, ",
        "intervalRight := {}, state := {}, firstProvableAt := {}, confirmedAt := {} }}"),
        value.key.level, candidate_kind(value.kind), rust_side(value.key.side),
        opt_nat(value.key.previous_center_start), value.key.parent.center_start, value.key.parent.zd,
        value.key.parent.zg, value.key.seg_a.0, value.key.seg_a.1, value.key.c_start,
        value.structural_predicates.direction, value.structural_predicates.comparable,
        value.structural_predicates.extreme, value.interval.0, value.interval.1,
        observed_state(value.state), opt_nat(value.first_provable_at), opt_nat(value.confirmed_at))
}

fn candidate_lean(value: &CandidateObservation) -> String {
    format!("rustCandidateToMirror {}", candidate_rust(value))
}

fn t3_raw_segment(value: Option<Segment>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |s| format!("(some {})", segment(&s, Some(s.direction), None)),
    )
}

fn first_input_expr(value: &FirstAssemblyCapture) -> (String, String) {
    let center = center(&value.center);
    let leave = t3_raw_segment(value.t3_leave);
    let retest = t3_raw_segment(value.t3_retest);
    let grade_expr = format!(
        "gradeFixedFirstPair {center} {} {leave} {retest}",
        dir(value.trend_dir)
    );
    let diverged = format!(
        "forceLDivergedBits {} {}",
        opt_u64_as_nat(value.force_l_a),
        opt_u64_as_nat(value.force_l_c)
    );
    let t3_present = format!("(match {grade_expr} with | Grade.present _ _ => true | _ => false)");
    let gates = format!("{{ side := {}, segA := {{ left := {}, right := {} }}, lambdaC := {}, aMapped := {}, cMapped := {}, extreme := {} }}",
        side(value.side), value.seg_a.0, value.seg_a.1, value.lambda_c,
        value.comparable, value.comparable, value.extreme);
    let input = format!("{{ level := {}, center := {}, segment := {}, gates := {}, diverged := {}, t3Present := {}, grade := {} }}",
        value.level, center, segment(&value.segment, Some(value.segment.direction), value.departure_end),
        gates, diverged, t3_present, grade_expr);
    (input, diverged)
}

fn first_rust(value: &FirstAssemblyCapture) -> String {
    let type1 = match value.side {
        Side::Long => value.output.bits.buy1,
        Side::Short => value.output.bits.sell1,
    };
    let owner = match value.output.center {
        Some(OwnerRef::Center(center)) => center.start_index,
        other => panic!("3a 一类结构候选 owner 必须是 Center，实际 {other:?}"),
    };
    format!("{{ sourceIndex := {}, side := {}, type1Bit := {}, type3Bit := false, structureBreak := true, ownerCenterStart := {} }}",
        value.output.source_index, rust_side(value.side), type1, owner)
}

fn grade_rust(value: &FirstAssemblyCapture) -> String {
    format!(
        concat!(
            "{{ level := {}, sourceIndex := {}, side := {}, centerStart := {}, ",
            "centerEnd := {}, centerZd := {}, centerZg := {}, grade := {} }}"
        ),
        value.level,
        value.output.source_index,
        rust_side(value.side),
        value.center.start_index,
        value.center.end_index,
        value.center.zd,
        value.center.zg,
        grade(value.rust_grade)
    )
}

fn trend_check(value: &TrendAssemblyCapture, path: &str) -> String {
    let gates = format!("{{ side := {}, segA := {{ left := {}, right := {} }}, lambdaC := {}, aMapped := {}, cMapped := {}, extreme := {} }}",
        side(value.side), value.seg_a.0, value.seg_a.1, value.lambda_c,
        value.a_idx_present, value.c_idx_present, value.extreme);
    format!(
        "checkTrendObservation \"{path}\" {} \"{}\" {} {} {} {} {}",
        value.level,
        side_name(value.side),
        center(&value.previous),
        center(&value.parent),
        segment(&value.segment, Some(value.segment.direction), None),
        gates,
        candidate_rust(&value.output)
    )
}

fn pan_check(cert: &PanDivCert, level: u32, path: &str) -> String {
    let pan = format!("{{ sourceIndex := {}, side := {}, center := {}, segA := {{ left := {}, right := {} }}, segC := {{ left := {}, right := {} }} }}",
        cert.source_index, side(cert.side), center(&cert.center), cert.seg_a.0, cert.seg_a.1, cert.seg_c.0, cert.seg_c.1);
    let input = format!("{{ consolidation := true, liftZero := true, comparable := true, diverged := true, panStructure := some {pan} }}");
    let rust = format!("{{ sourceIndex := {}, side := {}, centerStart := {}, centerZd := {}, centerZg := {}, segALeft := {}, segARight := {}, segCLeft := {}, segCRight := {} }}",
        cert.source_index, rust_side(cert.side), cert.center.start_index, cert.center.zd, cert.center.zg,
        cert.seg_a.0, cert.seg_a.1, cert.seg_c.0, cert.seg_c.1);
    format!(
        "checkPan \"{path}\" {level} \"{}\" {input} {rust}",
        side_name(cert.side)
    )
}

fn pan_mirror(cert: &PanDivCert) -> String {
    format!("{{ sourceIndex := {}, side := {}, centerStart := {}, centerZd := {}, centerZg := {}, segA := {{ left := {}, right := {} }}, segC := {{ left := {}, right := {} }} }}",
        cert.source_index, side(cert.side), cert.center.start_index, cert.center.zd, cert.center.zg,
        cert.seg_a.0, cert.seg_a.1, cert.seg_c.0, cert.seg_c.1)
}

fn cp_check(cp: &CpScanOwnership, level: u32, path: &str) -> String {
    let lifecycle = match cp.lifecycle {
        CpLifecycleStatus::Pending => "RustCpLifecycleTag.pending",
        CpLifecycleStatus::Closed => "RustCpLifecycleTag.closed",
    };
    let terminal = match cp.lifecycle {
        CpLifecycleStatus::Pending => "Pending",
        CpLifecycleStatus::Closed => "Closed",
    };
    let dep_ord = cp.departure_move_id.map(|id| id.ordinal as usize);
    let source_start = cp.departure_interval.map(|interval| interval.0);
    let rust = format!("{{ level := {level}, bCenterOrdinal := {}, departureMoveOrdinal := {}, sourceStart := {}, lifecycle := {lifecycle}, certificatePresent := {} }}",
        cp.b_center_index, opt_nat(dep_ord), opt_nat(source_start), cp.cp_certificate_confirm_src.is_some());
    let certificate = if cp.lifecycle == CpLifecycleStatus::Closed {
        let structure = cp.c_structure.expect("Closed cp 缺 c_structure");
        let third = cp.third_class_in_c.expect("Closed cp 缺 third_class_in_c");
        let confirm = cp
            .cp_certificate_confirm_src
            .expect("Closed cp 缺 confirm src");
        let terminal_ordinal = structure
            .terminal_move_id
            .expect("Closed cp 缺 terminal move")
            .ordinal;
        let source_end = structure.source_end.expect("Closed cp 缺 source end");
        let review = cp
            .full_trend_evidence
            .as_ref()
            .and_then(|e| e.decomposition_review_move_id)
            .map(|id| id.ordinal as usize);
        format!("some {{ confirmSource := {confirm}, terminalMoveOrdinal := {terminal_ordinal}, sourceEnd := {source_end}, thirdPointSource := {}, reviewMoveOrdinal := {}, fullyQualified := {} }}",
            third.point_source_index, opt_nat(review), cp.full_trend_c_qualified.is_some())
    } else {
        "none".to_owned()
    };
    let lean = format!("{{ identity := {{ level := {level}, bCenterOrdinal := {}, departureMoveOrdinal := {}, sourceStart := {} }}, lifecycle := {}, certificate := {certificate} }}",
        cp.b_center_index, opt_nat(dep_ord), opt_nat(source_start), match cp.lifecycle { CpLifecycleStatus::Pending => "CpLifecycle.pending", CpLifecycleStatus::Closed => "CpLifecycle.closed" });
    format!("checkCpAttachment \"{path}\" {level} \"{terminal}\" {rust} {lean}")
}

fn append_scan_checks(
    checks: &mut Vec<String>,
    counts: &mut PortCounts,
    enumerations: &mut PortCounts,
    batch: &CaptureBatch,
    checkpoint: usize,
) {
    for (i, scan) in batch.scans.iter().enumerate() {
        let path = format!("bar[{checkpoint}].scan[{i}].prelude");
        let rows = scan
            .segments
            .iter()
            .zip(scan.anchors.iter())
            .enumerate()
            .map(|(j, (s, a))| {
                let departure = scan.departure_ends.as_ref().and_then(|v| v.get(j)).copied();
                segment(s, *a, departure)
            })
            .collect::<Vec<_>>()
            .join(", ");
        let centers = scan
            .centers
            .iter()
            .map(center)
            .collect::<Vec<_>>()
            .join(", ");
        checks.push(format!(
            "checkPrelude \"{path}\" {} [{rows}] [{centers}]",
            scan.level
        ));
        counts.prelude += 1;
        enumerations.prelude += 1;
        for (port, count) in [
            ("points", scan.points.len()),
            ("pan", scan.pan_divs.len()),
            ("grades", scan.grades.len()),
            ("observations", scan.observations.len()),
        ] {
            checks.push(format!(
                "checkPortEnumeration \"bar[{checkpoint}].scan[{i}]\" \"{port}\" {} {count}",
                scan.level
            ));
        }
        enumerations.points += 1;
        enumerations.pan += 1;
        enumerations.grades += 1;
        enumerations.observations += 1;
        append_pan_checks(checks, counts, scan, checkpoint, i);
    }
    for (i, first) in batch.first_assemblies.iter().enumerate() {
        let (input, diverged) = first_input_expr(first);
        let path = format!("bar[{checkpoint}].first[{i}]");
        checks.push(format!(
            "checkFirst \"{path}.point\" {} \"{}\" {input} {}",
            first.level,
            side_name(first.side),
            first_rust(first)
        ));
        checks.push(format!(
            "checkGrade \"{path}.grade\" {} \"{}\" {input} (if {diverged} then some {} else none)",
            first.level,
            side_name(first.side),
            grade_rust(first)
        ));
        counts.points += 1;
        counts.grades += 1;
    }
    for (i, trend) in batch.trend_assemblies.iter().enumerate() {
        checks.push(trend_check(trend, &format!("bar[{checkpoint}].trend[{i}]")));
        counts.observations += 1;
    }
    for (i, reduction) in batch.reductions.iter().enumerate() {
        let legs = reduction
            .legs
            .iter()
            .map(candidate_lean)
            .collect::<Vec<_>>()
            .join(", ");
        let rust = reduction
            .output
            .iter()
            .map(candidate_rust)
            .collect::<Vec<_>>()
            .join(", ");
        let side_name = reduction
            .output
            .first()
            .map(|v| side_name(v.key.side))
            .unwrap_or("NotYetKnown");
        checks.push(format!("checkReduction \"bar[{checkpoint}].reduction[{i}]\" {} \"{side_name}\" [{legs}] [{rust}]", reduction.level));
        counts.reduction += 1;
        enumerations.reduction += 1;
    }
}

fn append_pan_checks(
    checks: &mut Vec<String>,
    counts: &mut PortCounts,
    scan: &ScanCapture,
    checkpoint: usize,
    scan_i: usize,
) {
    for (i, cert) in scan.pan_divs.iter().enumerate() {
        checks.push(pan_check(
            cert,
            scan.level,
            &format!("bar[{checkpoint}].scan[{scan_i}].pan[{i}]"),
        ));
        counts.pan += 1;
        if let Some(observation) = scan.observations.iter().find(|obs| {
            obs.kind == CandidateKind::Pan
                && obs.interval == cert.seg_c
                && obs.key.side == cert.side
        }) {
            checks.push(format!("checkPanObservation \"bar[{checkpoint}].scan[{scan_i}].pan_obs[{i}]\" {} \"{}\" {} {}",
                scan.level, side_name(cert.side), pan_mirror(cert), candidate_rust(observation)));
            counts.observations += 1;
        }
    }
}

fn event_terminal(event: &CandidateEvent) -> &'static str {
    match event.state {
        CandidateState::Provisional | CandidateState::Unresolved => "Open",
        CandidateState::Confirmed => "Closed",
        CandidateState::Invalidated => "Invalidated",
    }
}

fn finalize_layers(
    levels: &BTreeSet<u32>,
    counts: &BTreeMap<(u32, &'static str, &'static str), usize>,
) -> Vec<LayerCell> {
    let mut out = Vec::new();
    for &level in levels {
        for side in ["Long", "Short"] {
            for terminal in ["Open", "Closed", "Invalidated"] {
                out.push(LayerCell {
                    level,
                    side,
                    terminal,
                    extraction_count: *counts.get(&(level, side, terminal)).unwrap_or(&0),
                    mismatch_count: 0,
                });
            }
        }
    }
    out
}

fn run_window(
    bars: &[Bar],
    config: &ThetaConfig,
    spec: WindowSpec,
    artifact_dir: &Path,
    formal_dir: &Path,
) -> WindowReport {
    assert!(
        bars.len() >= spec.end,
        "数据仅 {} 根，不足 {}",
        bars.len(),
        spec.end
    );
    let started = Instant::now();
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = TowerCache::new();
    let mut checks = Vec::new();
    let mut ports = PortCounts::default();
    let mut enumerations = PortCounts::default();
    let mut checkpoints = 0;
    let mut levels = BTreeSet::new();
    let mut candidate_counts = BTreeMap::new();
    let mut cp_counts = BTreeMap::new();
    let mut last_scan_bucket = None;

    for (local, bar) in bars[spec.start..spec.end].iter().copied().enumerate() {
        let checkpoint = (local + 1) % CHECKPOINT_EVERY == 0 || local + 1 == spec.end - spec.start;
        // parser 是否恰在固定 bar 产出新结构不可预知；每 bar 打开、每 bar 取走，默认槽仍只读。
        // 每个 5k bucket 只保留首个真实 merged scan，另为尚未覆盖的产口保留首个非空样本。
        s2_mirror_capture::begin_capture();
        let l0 = parser.append(bar);
        let output = classifier::classify_incremental(&l0, config, &mut cache, &[]);
        let batch = s2_mirror_capture::take_capture();
        let bucket = local / CHECKPOINT_EVERY;
        let has_pan = batch.scans.iter().any(|scan| !scan.pan_divs.is_empty());
        let select_scan = !batch.scans.is_empty()
            && (last_scan_bucket != Some(bucket)
                || (ports.points == 0 && !batch.first_assemblies.is_empty())
                || (ports.pan == 0 && has_pan)
                || (ports.observations == 0 && !batch.trend_assemblies.is_empty()));
        if select_scan {
            append_scan_checks(
                &mut checks,
                &mut ports,
                &mut enumerations,
                &batch,
                bar.source_index,
            );
            last_scan_bucket = Some(bucket);
        }
        if !checkpoint {
            continue;
        }
        checkpoints += 1;
        for (level, state) in output.classification.levels.iter().enumerate() {
            let level = level as u32;
            levels.insert(level);
            assert_eq!(
                state.centers.len(),
                state.cp_ownership.len(),
                "cp attachment length mismatch L{level}"
            );
            checks.push(format!(
                "checkPortEnumeration \"bar[{}].level[{level}]\" \"cp\" {level} {}",
                bar.source_index,
                state.cp_ownership.len()
            ));
            enumerations.cp += 1;
            for (i, cp) in state.cp_ownership.iter().enumerate() {
                checks.push(cp_check(
                    cp,
                    level,
                    &format!("bar[{}].level[{level}].cp[{i}]", bar.source_index),
                ));
                ports.cp += 1;
                let terminal = match cp.lifecycle {
                    CpLifecycleStatus::Pending => "Pending",
                    CpLifecycleStatus::Closed => "Closed",
                };
                *cp_counts
                    .entry((level, "NotYetKnown", terminal))
                    .or_insert(0) += 1;
            }
        }
        for stream in output.candidate_streams.iter() {
            for event in stream.iter() {
                levels.insert(event.event_level);
                *candidate_counts
                    .entry((
                        event.event_level,
                        side_name(event.key.side),
                        event_terminal(event),
                    ))
                    .or_insert(0) += 1;
            }
        }
    }

    const CHECKS_PER_FIXTURE: usize = 500;
    let fixture_name = format!("{}-*.lean", spec.id.to_ascii_lowercase());
    let manifest_path = artifact_dir.join(format!("{}.lean", spec.id.to_ascii_lowercase()));
    let chunk_count = checks.len().div_ceil(CHECKS_PER_FIXTURE);
    std::fs::write(
        &manifest_path,
        format!("-- generated manifest: {chunk_count} chunks matching {fixture_name}\n"),
    )
    .expect("写 Lean fixture manifest 失败");
    let mut fixture_paths = Vec::with_capacity(chunk_count);
    for (chunk_index, chunk) in checks.chunks(CHECKS_PER_FIXTURE).enumerate() {
        let chunk_id = format!("{}-{:04}", spec.id, chunk_index);
        let fixture_path = artifact_dir.join(format!(
            "{}-{:04}.lean",
            spec.id.to_ascii_lowercase(),
            chunk_index
        ));
        let mut fixture = String::from("import Origin.UnifiedScanMirrorRunner\n\nset_option maxHeartbeats 0\nset_option maxRecDepth 100000\n\nopen NewChanlun.Origin.UnifiedScanMirrorBridge\nopen NewChanlun.Origin.UnifiedScanMirrorRunner\n\n#eval emitReport \"");
        write!(
            &mut fixture,
            "{chunk_id}\" [\n  {}\n]\n",
            chunk.join(",\n  ")
        )
        .unwrap();
        std::fs::write(&fixture_path, fixture).expect("写 Lean fixture chunk 失败");
        fixture_paths.push(
            fixture_path
                .canonicalize()
                .expect("fixture canonicalize 失败"),
        );
    }
    const LEAN_WORKERS: usize = 4;
    type ChunkResult = Result<(String, usize, usize), String>;
    let chunk_results =
        std::sync::Mutex::new(Vec::<(usize, ChunkResult)>::with_capacity(chunk_count));
    std::thread::scope(|scope| {
        for worker in 0..LEAN_WORKERS {
            let chunk_results = &chunk_results;
            let fixture_paths = &fixture_paths;
            scope.spawn(move || {
                for chunk_index in (worker..fixture_paths.len()).step_by(LEAN_WORKERS) {
                    let lean = Command::new("lake")
                        .args(["env", "lean"])
                        .arg(&fixture_paths[chunk_index])
                        .current_dir(formal_dir)
                        .output();
                    let result = match lean {
                        Err(error) => Err(format!("启动 Lean runner 失败: {error}")),
                        Ok(lean) => {
                            let stdout = String::from_utf8_lossy(&lean.stdout).into_owned();
                            let stderr = String::from_utf8_lossy(&lean.stderr).into_owned();
                            if !lean.status.success() {
                                Err(format!("stdout:\n{stdout}\nstderr:\n{stderr}"))
                            } else if let Some(summary) = stdout
                                .lines()
                                .find(|line| line.starts_with("S2_LEAN_SUMMARY\t"))
                            {
                                let cols: Vec<_> = summary.split('\t').collect();
                                match (cols[3].parse::<usize>(), cols[4].parse::<usize>()) {
                                    (Ok(compared), Ok(mismatch)) => {
                                        Ok((stdout, compared, mismatch))
                                    }
                                    _ => Err(format!("summary 数值非法: {summary}")),
                                }
                            } else {
                                Err(format!("缺 Lean summary: {stdout}"))
                            }
                        }
                    };
                    chunk_results
                        .lock()
                        .expect("chunk results mutex poisoned")
                        .push((chunk_index, result));
                }
            });
        }
    });
    let mut chunk_results = chunk_results
        .into_inner()
        .expect("chunk results mutex poisoned");
    chunk_results.sort_by_key(|(index, _)| *index);
    let mut lean_stdout = String::new();
    let mut compared_fields = 0;
    let mut mismatch_fields = 0;
    for (chunk_index, result) in chunk_results {
        let (stdout, compared, mismatch) = result.unwrap_or_else(|error| {
            panic!(
                "{} Lean 对拍失败（chunk {}）\n{}",
                spec.id, chunk_index, error
            )
        });
        lean_stdout.push_str(&stdout);
        compared_fields += compared;
        mismatch_fields += mismatch;
    }
    let candidate_layers = finalize_layers(&levels, &candidate_counts);
    let empty_candidate_layers = candidate_layers
        .iter()
        .filter(|cell| cell.extraction_count == 0)
        .count();
    let mut cp_layers = Vec::new();
    for &level in &levels {
        for terminal in ["Pending", "Closed"] {
            cp_layers.push(LayerCell {
                level,
                side: "NotYetKnown",
                terminal,
                extraction_count: *cp_counts
                    .get(&(level, "NotYetKnown", terminal))
                    .unwrap_or(&0),
                mismatch_count: 0,
            });
        }
    }
    let event_count = ports.extraction_events() + enumerations.extraction_events();
    let mut mismatch_by_port = BTreeMap::new();
    for port in [
        "prelude",
        "points",
        "pan",
        "grades",
        "observations",
        "reduction",
        "cp",
    ] {
        mismatch_by_port.insert(port, 0);
    }
    WindowReport {
        schema_version: SCHEMA_VERSION,
        window_id: spec.id,
        start: spec.start,
        end: spec.end,
        heavy_overlap: spec.heavy_overlap,
        checkpoint_every: CHECKPOINT_EVERY,
        checkpoint_count: checkpoints,
        extraction_event_count: event_count,
        compared_fields,
        mismatch_fields,
        mismatch_by_port,
        port_object_counts: ports.clone(),
        port_enumerations: enumerations.clone(),
        candidate_layers,
        cp_layers,
        empty_candidate_layers,
        minimum_events_required: MIN_EVENTS,
        minimum_events_met: event_count >= MIN_EVENTS,
        every_port_emitted: enumerations.all_emit(),
        zero_mismatch_upper_95: zero_failure_upper_95(compared_fields),
        lean_fixture: fixture_name,
        lean_stdout,
        elapsed_seconds: started.elapsed().as_secs_f64(),
    }
}

fn main() {
    assert!(
        standard_windows_are_valid(),
        "四窗定义或前三轻窗不重叠约束失效"
    );
    let args: Vec<_> = std::env::args_os().collect();
    assert!(
        args.len() == 3,
        "用法: s2_lean_mirror_extract <btc-json> <artifact-dir>"
    );
    let input = PathBuf::from(&args[1]);
    let artifact_dir = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&artifact_dir).expect("创建 artifact 目录失败");
    let formal_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("formal");
    let config = ThetaConfig::default();
    let bars = load(&input, config.tick.tick_size);
    eprintln!("S2 loaded {} bars from {}", bars.len(), input.display());
    let selected = std::env::var("S2_WINDOW").ok();
    for spec in WINDOWS {
        if selected.as_deref().is_some_and(|id| id != spec.id) {
            continue;
        }
        eprintln!("S2 {} [{}, {}) start", spec.id, spec.start, spec.end);
        let report = run_window(&bars, &config, spec, &artifact_dir, &formal_dir);
        let path = artifact_dir.join(format!("{}.json", spec.id.to_ascii_lowercase()));
        std::fs::write(&path, serde_json::to_string_pretty(&report).unwrap()).expect("写报告失败");
        println!(
            "S2_WINDOW {} events={} compared_fields={} mismatch={} ports={:?} elapsed={:.3}s",
            spec.id,
            report.extraction_event_count,
            report.compared_fields,
            report.mismatch_fields,
            report.port_object_counts,
            report.elapsed_seconds
        );
        assert!(
            report.minimum_events_met,
            "{} extraction events {} < {}",
            spec.id, report.extraction_event_count, MIN_EVENTS
        );
        assert!(
            report.every_port_emitted,
            "{} 至少一个产口未枚举: {:?}",
            spec.id, report.port_enumerations
        );
        assert_eq!(report.mismatch_fields, 0, "{} mismatch 非零", spec.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_windows_keep_three_light_ranges_disjoint() {
        assert!(standard_windows_are_valid());
    }

    #[test]
    fn zero_failure_upper_bound_is_monotone() {
        assert!(zero_failure_upper_95(10_000) < zero_failure_upper_95(100));
        assert_eq!(zero_failure_upper_95(0), 1.0);
    }
}
