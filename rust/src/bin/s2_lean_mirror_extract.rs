//! #1080 S2 Phase 2：3a 统一扫描 Rust→Lean 只读提取执行器。
//!
//! 用法：
//! `cargo run --release --bin s2_lean_mirror_extract -- <btc_1m_full.json> <artifact-dir>`
//!
//! 执行器逐窗从空 parser/cache 因果重放，在 checkpoint bar 打开 crate 内只读捕获槽；
//! fixture 只携 Rust raw input/wire，由 Lean runner 现场执行镜面函数。生产判定路径不分叉。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use newchan_rust::theta_v0::classifier::bsp::BspPoint;
use newchan_rust::theta_v0::classifier::bsp::OwnerRef;
use newchan_rust::theta_v0::classifier::cand_event::{
    CandidateKey, CandidateKind, CandidateObservation, ObservedState, ParentFingerprint,
    StructuralPredicates,
};
use newchan_rust::theta_v0::classifier::decompose::{decompose, MoveBlock, MoveStatus};
use newchan_rust::theta_v0::classifier::descend::RMove;
use newchan_rust::theta_v0::classifier::diag::s2_mirror_capture::{
    self, CaptureBatch, CpLifecycleEvent, FirstAssemblyCapture, ForceSegmentRawCapture,
    FourRcMemoCapture, FrontierCapture, ScanCapture, SecondLegRawCapture, SecondProductionCapture,
    ThirdAssemblyCapture, TrendAssemblyCapture,
};
use newchan_rust::theta_v0::classifier::divergence::{
    DivergenceGauge, ForceFeatures, ForceProxies,
};
use newchan_rust::theta_v0::classifier::recursive_tower::{
    CompletedTrendDecomposition, CpLifecycleStatus, CpScanOwnership, CpStructureIdentity,
    ElementId, FullTrendCQualified, FullTrendQualificationEvidence, InternalSublevelCenters,
    LeveledMove, NewExtremeInDirection, ThirdClassInCp, TrendContext,
};
use newchan_rust::theta_v0::classifier::signal::{PanDivCert, T3InCScan};
use newchan_rust::theta_v0::classifier::{self, TowerCache};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{
    quantize, Bar, BspBits, Center, Direction, MoveKind, Segment, Side, Stroke,
    ThirdClassEntryIdentity,
};
use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: &str = "s2-lean-mirror-v3";
const CHECKPOINT_EVERY: usize = 5_000;
const MIN_EVENTS: usize = 100;

/// spec Prelude + O-01..O-04 + c_p + P-10 的递归叶字段全集。Option 明列 tag 与 payload，
/// Vec 明列 length/canonical_order/item，ElementId 明列 level/ordinal。分子只能来自实际
/// RuntimeCheck 或同 Runner 的独立 fixture，禁止常量回填。
const PRELUDE_FIELD_PATHS: &[&str] = &[
    "P01.segment_start_order",
    "P01.center_end_strict_order",
    "P02.anchor_structural_direction",
    "P02.parallel_array_length",
    "P03.nearest_confirmed_center",
    "P04.incoming_trend_gate",
    "P05.consolidation_lift_zero",
    "P06.first_match_identity",
    "P07.a_segment_cache",
    "P08.lambda_c_episode",
    "P09.structural_gate",
];

fn push_fields(paths: &mut Vec<String>, prefix: &str, fields: &[&str]) {
    paths.extend(fields.iter().map(|field| format!("{prefix}.{field}")));
}

fn push_option(paths: &mut Vec<String>, prefix: &str, payload_fields: &[&str]) {
    paths.push(format!("{prefix}.tag"));
    push_fields(paths, prefix, payload_fields);
}

fn push_list(paths: &mut Vec<String>, prefix: &str, item_fields: &[&str]) {
    paths.push(format!("{prefix}.length"));
    paths.push(format!("{prefix}.canonical_order"));
    push_fields(paths, &format!("{prefix}.item"), item_fields);
}

/// 完整 typed wire schema。这里保留 union 的所有 payload，即使某个具体生产子通道按构造
/// 恒为 `None`；它只用于报告 serializer/typed parity 的结构覆盖，不直接授予生产语义信用。
fn declared_field_paths() -> Vec<String> {
    const CENTER: &[&str] = &["start_index", "end_index", "zd", "zg", "dd", "gg"];
    const ELEMENT_ID: &[&str] = &["level", "ordinal"];
    const INTERVAL: &[&str] = &["left", "right"];
    const O01_ITEM: &[&str] = &[
        "source_index",
        "bits.buy1",
        "bits.buy2",
        "bits.buy3",
        "bits.sell1",
        "bits.sell2",
        "bits.sell3",
        "bits.third_class_entry.tag",
        "bits.third_class_entry.center_si",
        "bits.third_class_entry.center_zd",
        "bits.third_class_entry.center_zg",
        "bits.third_class_entry.leave.left",
        "bits.third_class_entry.leave.right",
        "bits.third_class_entry.retest.left",
        "bits.third_class_entry.retest.right",
        "pivot_low",
        "pivot_high",
        "owner.tag",
        "owner.variant",
        "owner.center.start_index",
        "owner.center.end_index",
        "owner.center.zd",
        "owner.center.zg",
        "owner.center.dd",
        "owner.center.gg",
        "owner.type1_anchor",
        "struct_break_dir.tag",
        "struct_break_dir.value",
        "force.tag",
        "force.seg_a.macd_area_bits",
        "force.seg_a.dif_peak_bits",
        "force.seg_a.price_amplitude",
        "force.seg_a.price_speed_bits",
        "force.seg_c.macd_area_bits",
        "force.seg_c.dif_peak_bits",
        "force.seg_c.price_amplitude",
        "force.seg_c.price_speed_bits",
        "retrace_breaks_type1.tag",
        "retrace_breaks_type1.value",
    ];
    const O02_ITEM: &[&str] = &[
        "source_index",
        "side",
        "center.start_index",
        "center.end_index",
        "center.zd",
        "center.zg",
        "center.dd",
        "center.gg",
        "seg_a.left",
        "seg_a.right",
        "seg_c.left",
        "seg_c.right",
    ];
    const O03_ITEM: &[&str] = &[
        "level",
        "source_index",
        "side",
        "center_start_index",
        "center_end_index",
        "center_zd",
        "center_zg",
        "grade.tag",
        "grade.present.leave.left",
        "grade.present.leave.right",
        "grade.present.retest.left",
        "grade.present.retest.right",
        "grade.missing.reason",
    ];
    const O04_ITEM: &[&str] = &[
        "key.rule_version",
        "key.level",
        "key.kind",
        "kind",
        "key.side",
        "key.previous_center_start.tag",
        "key.previous_center_start.value",
        "key.parent.start",
        "key.parent.zd",
        "key.parent.zg",
        "key.seg_a.left",
        "key.seg_a.right",
        "key.c_start",
        "center_ids.tag",
        "center_ids.left",
        "center_ids.right",
        "candidate_group_id",
        "pair_id",
        "structural_predicates.direction",
        "structural_predicates.comparable",
        "structural_predicates.extreme",
        "extreme_proof.left",
        "extreme_proof.right",
        "third_class_proof.tag",
        "third_class_proof.value",
        "interval.left",
        "interval.right",
        "state",
        "first_provable_at.tag",
        "first_provable_at.value",
        "confirmed_at.tag",
        "confirmed_at.value",
    ];
    let mut paths = PRELUDE_FIELD_PATHS
        .iter()
        .map(|path| (*path).to_owned())
        .collect();
    push_fields(&mut paths, "O01", O01_ITEM);
    push_fields(&mut paths, "O02", O02_ITEM);
    push_fields(&mut paths, "O03", O03_ITEM);
    push_fields(&mut paths, "O04", O04_ITEM);

    paths.push("cp.b_center_index".into());
    push_fields(&mut paths, "cp.b_center_id", ELEMENT_ID);
    push_fields(&mut paths, "cp.b_center", CENTER);
    push_option(&mut paths, "cp.departure_move_id", ELEMENT_ID);
    push_option(&mut paths, "cp.departure_interval", INTERVAL);
    paths.push("cp.lifecycle".into());
    push_option(&mut paths, "cp.cp_certificate_confirm_src", &["value"]);
    push_option(
        &mut paths,
        "cp.c_structure",
        &[
            "level",
            "b_center_id.level",
            "b_center_id.ordinal",
            "departure_move_id.level",
            "departure_move_id.ordinal",
            "terminal_move_id.tag",
            "terminal_move_id.level",
            "terminal_move_id.ordinal",
            "source_start",
            "source_end.tag",
            "source_end.value",
        ],
    );
    const THIRD_CP: &[&str] = &[
        "b_center_id.level",
        "b_center_id.ordinal",
        "cp_departure_move_id.level",
        "cp_departure_move_id.ordinal",
        "departure_move_id.level",
        "departure_move_id.ordinal",
        "retest_move_id.level",
        "retest_move_id.ordinal",
        "departure_interval.left",
        "departure_interval.right",
        "retest_interval.left",
        "retest_interval.right",
        "point_source_index",
        "side",
    ];
    push_option(&mut paths, "cp.third_class_in_c", THIRD_CP);
    const TREND: &[&str] = &[
        "predecessor_center_id.level",
        "predecessor_center_id.ordinal",
        "b_center_id.level",
        "b_center_id.ordinal",
        "direction",
    ];
    const EXTREME: &[&str] = &[
        "b_center_id.level",
        "b_center_id.ordinal",
        "direction",
        "reference_price",
        "extreme_price",
        "extreme_move_id.level",
        "extreme_move_id.ordinal",
        "confirm_src",
    ];
    let evidence = "cp.full_trend_evidence";
    paths.push(format!("{evidence}.tag"));
    push_option(&mut paths, &format!("{evidence}.trend_context"), TREND);
    push_option(&mut paths, &format!("{evidence}.new_extreme"), EXTREME);
    paths.push(format!("{evidence}.internal_centers.tag"));
    paths.push(format!("{evidence}.internal_centers.c_level"));
    push_list(
        &mut paths,
        &format!("{evidence}.internal_centers.center_ids"),
        ELEMENT_ID,
    );
    paths.push(format!("{evidence}.completed_decomposition.tag"));
    paths.push(format!("{evidence}.completed_decomposition.direction"));
    push_list(
        &mut paths,
        &format!("{evidence}.completed_decomposition.center_ids"),
        ELEMENT_ID,
    );
    push_fields(
        &mut paths,
        &format!("{evidence}.completed_decomposition.closing_successor_move_id"),
        ELEMENT_ID,
    );
    paths.push(format!("{evidence}.completed_decomposition.confirm_src"));
    push_option(
        &mut paths,
        &format!("{evidence}.decomposition_review_move_id"),
        ELEMENT_ID,
    );
    push_option(
        &mut paths,
        &format!("{evidence}.decomposition_review_src"),
        &["value"],
    );

    let qualified = "cp.full_trend_c_qualified";
    paths.push(format!("{qualified}.tag"));
    push_fields(&mut paths, &format!("{qualified}.trend_context"), TREND);
    push_fields(&mut paths, &format!("{qualified}.third_class"), THIRD_CP);
    push_fields(&mut paths, &format!("{qualified}.new_extreme"), EXTREME);
    paths.push(format!("{qualified}.internal_centers.c_level"));
    push_list(
        &mut paths,
        &format!("{qualified}.internal_centers.center_ids"),
        ELEMENT_ID,
    );
    paths.push(format!("{qualified}.completed_decomposition.direction"));
    push_list(
        &mut paths,
        &format!("{qualified}.completed_decomposition.center_ids"),
        ELEMENT_ID,
    );
    push_fields(
        &mut paths,
        &format!("{qualified}.completed_decomposition.closing_successor_move_id"),
        ELEMENT_ID,
    );
    paths.push(format!("{qualified}.completed_decomposition.confirm_src"));
    paths.push(format!("{qualified}.confirm_src"));

    push_fields(
        &mut paths,
        "P10",
        &[
            "cached_count_before",
            "prefix_count",
            "dirty_e",
            "freeze_boundary_src.tag",
            "freeze_boundary_src.value",
            "stable_seg",
        ],
    );
    push_list(&mut paths, "P10.segment_end_indices", &["value"]);
    push_list(&mut paths, "P10.centers", CENTER);
    for output in [
        "P10.confirmed_append",
        "P10.cache_after",
        "P10.tail",
        "P10.final_output",
    ] {
        push_list(&mut paths, &format!("{output}.points"), O01_ITEM);
        push_list(&mut paths, &format!("{output}.pan_divs"), O02_ITEM);
        push_list(&mut paths, &format!("{output}.grades"), O03_ITEM);
        push_list(&mut paths, &format!("{output}.candidate_legs"), O04_ITEM);
    }
    paths.push("P10.cache_after.cached_count".into());
    push_fields(
        &mut paths,
        "P10.memo.key",
        &["centers", "upper_moves", "structures"],
    );
    push_option(
        &mut paths,
        "P10.memo.cached_key_before",
        &["centers", "upper_moves", "structures"],
    );
    paths.push("P10.memo.hit".into());
    for list in ["prior_reused", "after_ptr_eq", "before_lengths", "lengths"] {
        push_list(&mut paths, &format!("P10.memo.{list}"), &["value"]);
    }
    push_list(&mut paths, "P10.memo.direct_3a_points", O01_ITEM);
    push_list(&mut paths, "P10.memo.second_07b_points", O01_ITEM);
    push_list(&mut paths, "P10.memo.pipeline_points", O01_ITEM);
    push_list(&mut paths, "P10.memo.pan_divs", O02_ITEM);
    push_list(&mut paths, "P10.memo.grades", O03_ITEM);
    push_list(&mut paths, "P10.memo.candidate_legs", O04_ITEM);
    paths.push("P10.memo.pipeline_07b_composition".into());
    let unique = paths.iter().collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), paths.len(), "coverage schema 路径重复");
    paths
}

/// 某个具体生产子通道按唯一构造器不可能承载的 union payload。
///
/// tag/variant 仍在分母：`None`/另一 union 臂本身就是需要对拍的生产值。这里只剔除永远
/// 不会进入该上下文 parity 的 payload，避免把 typed schema 可表达性错当作每个产口都可达。
fn production_context_impossible_path(path: &str) -> bool {
    const DIRECT_POINT_PREFIXES: &[&str] = &[
        "P10.confirmed_append.points.item",
        "P10.cache_after.points.item",
        "P10.tail.points.item",
        "P10.final_output.points.item",
        "P10.memo.direct_3a_points.item",
    ];
    const SECOND_POINT_PREFIX: &str = "P10.memo.second_07b_points.item";
    const SCAN_CANDIDATE_PREFIXES: &[&str] = &[
        "O04",
        "P10.confirmed_append.candidate_legs.item",
        "P10.cache_after.candidate_legs.item",
        "P10.tail.candidate_legs.item",
        "P10.final_output.candidate_legs.item",
        "P10.memo.candidate_legs.item",
    ];

    if DIRECT_POINT_PREFIXES.iter().any(|prefix| {
        path == format!("{prefix}.owner.type1_anchor")
            || path == format!("{prefix}.retrace_breaks_type1.value")
    }) {
        return true;
    }

    if let Some(field) = path.strip_prefix(&format!("{SECOND_POINT_PREFIX}.")) {
        return matches!(
            field,
            "bits.third_class_entry.center_si"
                | "bits.third_class_entry.center_zd"
                | "bits.third_class_entry.center_zg"
                | "bits.third_class_entry.leave.left"
                | "bits.third_class_entry.leave.right"
                | "bits.third_class_entry.retest.left"
                | "bits.third_class_entry.retest.right"
                | "owner.center.start_index"
                | "owner.center.end_index"
                | "owner.center.zd"
                | "owner.center.zg"
                | "owner.center.dd"
                | "owner.center.gg"
                | "struct_break_dir.value"
                | "force.seg_a.macd_area_bits"
                | "force.seg_a.dif_peak_bits"
                | "force.seg_a.price_amplitude"
                | "force.seg_a.price_speed_bits"
                | "force.seg_c.macd_area_bits"
                | "force.seg_c.dif_peak_bits"
                | "force.seg_c.price_amplitude"
                | "force.seg_c.price_speed_bits"
        );
    }

    SCAN_CANDIDATE_PREFIXES.iter().any(|prefix| {
        path == format!("{prefix}.third_class_proof.value")
            || path == format!("{prefix}.confirmed_at.value")
    })
}

/// spec 靶面中真实生产上下文可进入逐叶 parity 的字段全集。
fn required_field_paths() -> Vec<String> {
    declared_field_paths()
        .into_iter()
        .filter(|path| !production_context_impossible_path(path))
        .collect()
}

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
    frontier: usize,
    memo: usize,
    pipeline_07b: usize,
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
            + self.frontier
            + self.memo
            + self.pipeline_07b
    }

    fn all_emit(&self) -> bool {
        self.prelude > 0
            && self.points > 0
            && self.pan > 0
            && self.grades > 0
            && self.observations > 0
            && self.reduction > 0
            && self.cp > 0
            && self.frontier > 0
            && self.memo > 0
            && self.pipeline_07b > 0
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

#[derive(Debug, Clone, Serialize)]
struct RuntimeLayerCell {
    level: u32,
    side: String,
    terminal: String,
    port: String,
    compared_fields: usize,
    mismatch_fields: usize,
}

#[derive(Debug, Clone, Serialize)]
struct OptionPayloadCoverage {
    path: &'static str,
    some_count: usize,
    empty_count: usize,
    exercised: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CoverageReport {
    numerator: usize,
    denominator: usize,
    rate: f64,
    covered_paths: Vec<String>,
    uncovered_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct VariantCoverage {
    family: &'static str,
    required_variants: Vec<&'static str>,
    covered_variants: Vec<&'static str>,
    complete: bool,
}

#[derive(Debug, Clone, Serialize)]
struct FixtureEvidence {
    source: &'static str,
    records: usize,
    compared_fields: usize,
    mismatch_fields: usize,
    passed_checks: Vec<String>,
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
    mismatch_by_port: BTreeMap<String, usize>,
    compared_by_port: BTreeMap<String, usize>,
    failure_records_by_port: BTreeMap<String, usize>,
    port_object_counts: PortCounts,
    port_enumerations: PortCounts,
    candidate_layers: Vec<LayerCell>,
    cp_layers: Vec<LayerCell>,
    runtime_layers: Vec<RuntimeLayerCell>,
    empty_candidate_layers: usize,
    minimum_events_required: usize,
    minimum_events_met: bool,
    every_port_emitted: bool,
    zero_mismatch_upper_95: f64,
    match_rate: f64,
    field_coverage_numerator: usize,
    field_coverage_denominator: usize,
    field_coverage_rate: f64,
    field_coverage_paths: Vec<String>,
    field_uncovered_paths: Vec<String>,
    real_window_coverage: CoverageReport,
    production_semantic_fixture_coverage: CoverageReport,
    wire_schema_smoke_coverage: CoverageReport,
    fixture_coverage: CoverageReport,
    combined_coverage: CoverageReport,
    variant_coverage: Vec<VariantCoverage>,
    fixture_evidence: FixtureEvidence,
    option_payload_coverage: Vec<OptionPayloadCoverage>,
    max_level: Option<u32>,
    missing_levels: Vec<u32>,
    lean_fixture: String,
    lean_stdout_bytes: usize,
    lean_stdout_digest_fnv1a64: String,
    aggregate_complete: bool,
    aggregate_status: &'static str,
    elapsed_seconds: f64,
}

#[derive(Debug, Deserialize)]
struct SavedWindowReport {
    schema_version: String,
    window_id: String,
    start: usize,
    end: usize,
    heavy_overlap: bool,
    mismatch_fields: usize,
    minimum_events_met: bool,
    every_port_emitted: bool,
    combined_coverage: CoverageReport,
}

#[derive(Debug, Serialize)]
struct AggregateReport {
    schema_version: &'static str,
    aggregate_complete: bool,
    aggregate_status: &'static str,
    source_window_ids: Vec<&'static str>,
    zero_mismatch: bool,
    minimum_events_met: bool,
    every_port_emitted: bool,
    typed_schema_denominator: usize,
    semantic_coverage: CoverageReport,
}

fn fnv1a64_hex(bytes: &[u8]) -> String {
    let mut digest = 0xcbf29ce484222325_u64;
    for byte in bytes {
        digest ^= u64::from(*byte);
        digest = digest.wrapping_mul(0x100000001b3);
    }
    format!("{digest:016x}")
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

fn bump_option(
    stats: &mut BTreeMap<&'static str, (usize, usize)>,
    path: &'static str,
    present: bool,
) {
    let entry = stats.entry(path).or_default();
    if present {
        entry.0 += 1;
    } else {
        entry.1 += 1;
    }
}

fn observe_point_options(point: &BspPoint, stats: &mut BTreeMap<&'static str, (usize, usize)>) {
    bump_option(
        stats,
        "O01.bits.third_class_entry",
        point.bits.third_class_entry.is_some(),
    );
    bump_option(stats, "O01.owner", point.center.is_some());
    bump_option(stats, "O01.force", point.force.is_some());
    bump_option(
        stats,
        "O01.retrace_breaks_type1",
        point.retrace_breaks_type1.is_some(),
    );
}

fn observe_candidate_options(
    value: &CandidateObservation,
    stats: &mut BTreeMap<&'static str, (usize, usize)>,
) {
    bump_option(stats, "O04.center_ids", value.center_ids.is_some());
    bump_option(
        stats,
        "O04.third_class_proof",
        value.third_class_proof.is_some(),
    );
    bump_option(
        stats,
        "O04.first_provable_at",
        value.first_provable_at.is_some(),
    );
    bump_option(stats, "O04.confirmed_at", value.confirmed_at.is_some());
}

fn observe_cp_options(cp: &CpScanOwnership, stats: &mut BTreeMap<&'static str, (usize, usize)>) {
    bump_option(
        stats,
        "cp.departure_move_id",
        cp.departure_move_id.is_some(),
    );
    bump_option(
        stats,
        "cp.departure_interval",
        cp.departure_interval.is_some(),
    );
    bump_option(
        stats,
        "cp.cp_certificate_confirm_src",
        cp.cp_certificate_confirm_src.is_some(),
    );
    bump_option(stats, "cp.c_structure", cp.c_structure.is_some());
    bump_option(stats, "cp.third_class_in_c", cp.third_class_in_c.is_some());
    bump_option(
        stats,
        "cp.full_trend_evidence",
        cp.full_trend_evidence.is_some(),
    );
    bump_option(
        stats,
        "cp.full_trend_c_qualified",
        cp.full_trend_c_qualified.is_some(),
    );
}

fn observe_batch_options(batch: &CaptureBatch, stats: &mut BTreeMap<&'static str, (usize, usize)>) {
    for scan in &batch.scans {
        for point in &scan.points {
            observe_point_options(point, stats);
        }
        for candidate in &scan.observations {
            observe_candidate_options(candidate, stats);
        }
        for grade in &scan.grades {
            bump_option(
                stats,
                "O03.grade.present",
                matches!(grade.grade, T3InCScan::Present { .. }),
            );
            bump_option(
                stats,
                "O03.grade.missing",
                matches!(grade.grade, T3InCScan::Missing),
            );
        }
    }
    for memo in &batch.four_rc_memos {
        bump_option(
            stats,
            "P10.memo.cached_key_before",
            memo.cached_key_before.is_some(),
        );
        for point in memo.pipeline_points.iter() {
            observe_point_options(point, stats);
        }
        for candidate in memo.candidates.iter() {
            observe_candidate_options(candidate, stats);
        }
    }
}

fn coverage_report(covered: BTreeSet<String>, required: &[String]) -> CoverageReport {
    let required_set = required.iter().cloned().collect::<BTreeSet<_>>();
    let covered = covered
        .intersection(&required_set)
        .cloned()
        .collect::<BTreeSet<_>>();
    let uncovered = required
        .iter()
        .filter(|path| !covered.contains(*path))
        .cloned()
        .collect::<Vec<_>>();
    let numerator = covered.len();
    let denominator = required.len();
    CoverageReport {
        numerator,
        denominator,
        rate: if denominator == 0 {
            1.0
        } else {
            numerator as f64 / denominator as f64
        },
        covered_paths: covered.into_iter().collect(),
        uncovered_paths: uncovered,
    }
}

fn aggregate_coverage_reports<'a>(
    reports: impl IntoIterator<Item = &'a CoverageReport>,
    required: &[String],
) -> CoverageReport {
    let covered = reports
        .into_iter()
        .flat_map(|report| report.covered_paths.iter().cloned())
        .collect::<BTreeSet<_>>();
    coverage_report(covered, required)
}

fn fixture_variant_coverage(passed_checks: &BTreeSet<String>) -> Vec<VariantCoverage> {
    let specs = [
        (
            "O01.owner",
            vec![
                ("Center", "coverage.o01.first_some"),
                ("Type1Anchor", "coverage.o01.owner_type1_anchor"),
            ],
        ),
        (
            "O01.first",
            vec![
                ("Some", "coverage.o01.first_some"),
                ("None", "coverage.o01.first_none"),
            ],
        ),
        (
            "O01.third",
            vec![
                ("Success", "coverage.o01.third_success"),
                ("Failure", "coverage.o01.third_failure"),
            ],
        ),
        (
            "O03.grade",
            vec![
                ("Present", "coverage.o03.present"),
                ("MissingLeave", "coverage.o03.missing_leave"),
                ("MissingRetest", "coverage.o03.missing_retest"),
                ("SameDirection", "coverage.o03.same_direction"),
                ("LeaveNotOutside", "coverage.o03.leave_not_outside"),
                ("RetestReentered", "coverage.o03.retest_reentered"),
            ],
        ),
        (
            "O04.predicates.extreme",
            vec![
                ("true", "coverage.o04.extreme_true.third_some"),
                ("false", "coverage.o04.extreme_false.third_none"),
            ],
        ),
        (
            "O04.third_class_proof",
            vec![
                ("Some", "coverage.o04.extreme_true.third_some"),
                ("None", "coverage.o04.extreme_false.third_none"),
            ],
        ),
    ];
    specs
        .into_iter()
        .map(|(family, variants)| {
            let required_variants = variants.iter().map(|(variant, _)| *variant).collect();
            let covered_variants = variants
                .iter()
                .filter(|(_, check)| passed_checks.contains(*check))
                .map(|(variant, _)| *variant)
                .collect::<Vec<_>>();
            VariantCoverage {
                family,
                complete: covered_variants.len() == variants.len(),
                covered_variants,
                required_variants,
            }
        })
        .collect()
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

fn rust_dir(value: Direction) -> &'static str {
    match value {
        Direction::Up => "RustDirectionTag.up",
        Direction::Down => "RustDirectionTag.down",
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

fn opt_bool(value: Option<bool>) -> String {
    value.map_or_else(|| "none".to_owned(), |v| format!("(some {v})"))
}

fn interval(value: (usize, usize)) -> String {
    format!("{{ left := {}, right := {} }}", value.0, value.1)
}

fn opt_u64_as_nat(value: Option<f64>) -> String {
    value.map_or_else(|| "none".to_owned(), |v| format!("(some {})", v.to_bits()))
}

fn center(value: &Center) -> String {
    format!(
        "{{ startIndex := {}, endIndex := {}, zd := {}, zg := {}, dd := {}, gg := {} }}",
        value.start_index, value.end_index, value.zd, value.zg, value.dd, value.gg
    )
}

fn rust_center(value: &Center) -> String {
    format!(
        "{{ startIndex := {}, endIndex := {}, zd := {}, zg := {}, dd := {}, gg := {} }}",
        value.start_index, value.end_index, value.zd, value.zg, value.dd, value.gg
    )
}

fn element_id(value: ElementId) -> String {
    format!(
        "{{ level := {}, ordinal := {} }}",
        value.level, value.ordinal
    )
}

fn opt_element_id(value: Option<ElementId>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |id| format!("(some {})", element_id(id)),
    )
}

fn force_feature(value: &ForceFeatures) -> String {
    format!(
        "{{ macdAreaBits := {}, difPeakBits := {}, priceAmplitude := {}, priceSpeedBits := {} }}",
        value.macd_area.to_bits(),
        value.dif_peak.to_bits(),
        value.price_amplitude,
        value.price_speed.to_bits()
    )
}

fn force_proxies(value: &ForceProxies) -> String {
    format!(
        "{{ segA := {}, segC := {} }}",
        force_feature(&value.seg_a),
        force_feature(&value.seg_c)
    )
}

fn opt_force(value: Option<ForceProxies>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |force| format!("(some {})", force_proxies(&force)),
    )
}

fn bsp_bits(value: &BspPoint) -> String {
    let entry =
        value.bits.third_class_entry.map_or_else(
            || "none".to_owned(),
            |entry| {
                format!(
                    concat!("(some {{ centerSi := {}, centerZd := {}, centerZg := {}, ",
                "leaveLeft := {}, leaveRight := {}, retestLeft := {}, retestRight := {} }})"),
                    entry.center_si,
                    entry.center_zd,
                    entry.center_zg,
                    entry.leave_interval.0,
                    entry.leave_interval.1,
                    entry.retest_interval.0,
                    entry.retest_interval.1
                )
            },
        );
    format!(
        concat!(
            "{{ buy1 := {}, buy2 := {}, buy3 := {}, sell1 := {}, sell2 := {}, ",
            "sell3 := {}, thirdClassEntry := {} }}"
        ),
        value.bits.buy1,
        value.bits.buy2,
        value.bits.buy3,
        value.bits.sell1,
        value.bits.sell2,
        value.bits.sell3,
        entry
    )
}

fn bsp_owner(value: &BspPoint) -> String {
    match value.center {
        None => "none".to_owned(),
        Some(OwnerRef::Center(center)) => format!(
            "(some (RustOwnerRefExtraction.center {}))",
            rust_center(&center)
        ),
        Some(OwnerRef::Type1Anchor(source_index)) => {
            format!("(some (RustOwnerRefExtraction.type1Anchor {source_index}))")
        }
    }
}

fn opt_rust_side(value: Option<Side>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |side| format!("(some {})", rust_side(side)),
    )
}

fn bsp_rust(value: &BspPoint) -> String {
    format!(
        concat!(
            "{{ sourceIndex := {}, bits := {}, pivotLow := {}, pivotHigh := {}, ",
            "owner := {}, structBreakDir := {}, force := {}, retraceBreaksType1 := {} }}"
        ),
        value.source_index,
        bsp_bits(value),
        value.pivot_low,
        value.pivot_high,
        bsp_owner(value),
        opt_rust_side(value.struct_break_dir),
        opt_force(value.force),
        opt_bool(value.retrace_breaks_type1)
    )
}

fn segment(value: &Segment, anchor: Option<Direction>, departure_end: Option<usize>) -> String {
    let anchor = anchor.map_or_else(|| "none".to_owned(), |v| format!("some {}", dir(v)));
    format!("{{ direction := {}, startIndex := {}, endIndex := {}, endPrice := {}, anchor := {}, departureEnd := {} }}",
        dir(value.direction), value.start_index, value.end_index, value.end_price, anchor, opt_nat(departure_end))
}

fn grade(value: T3InCScan) -> String {
    match value {
        T3InCScan::Present {
            leave_interval,
            retest_interval,
        } => format!(
            "RustGradeExtractionValue.present {} {} {} {}",
            leave_interval.0, leave_interval.1, retest_interval.0, retest_interval.1
        ),
        // #1249 后扫统一：Missing 不再携五桶 reason；Lean `RustGradeExtractionValue` 的
        // `missing (reason)` 形状待 Lean 侧补后扫镜面（后续票）——此处用 `missingLeave` 占位
        // 保 wire 闭合，语义对拍在 Lean 镜面跟上前不成立。
        T3InCScan::Missing => {
            "RustGradeExtractionValue.missing RustGradeReasonTag.missingLeave".to_owned()
        }
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
    let center_ids = value
        .center_ids
        .map_or_else(|| "none".to_owned(), |(a, b)| format!("(some ({a}, {b}))"));
    format!(concat!("{{ ruleVersion := {}, level := {}, kind := {}, observationKind := {}, side := {}, previousCenterStart := {}, ",
        "parentCenterStart := {}, parentZd := {}, parentZg := {}, segALeft := {}, segARight := {}, ",
        "lambdaC := {}, centerIds := {}, candidateGroupId := {}, pairId := {}, ",
        "direction := {}, comparable := {}, extreme := {}, extremeProofLeft := {}, extremeProofRight := {}, ",
        "thirdClassProof := {}, intervalLeft := {}, ",
        "intervalRight := {}, state := {}, firstProvableAt := {}, confirmedAt := {} }}"),
        value.key.rule_version, value.key.level, candidate_kind(value.key.kind),
        candidate_kind(value.kind), rust_side(value.key.side),
        opt_nat(value.key.previous_center_start), value.key.parent.center_start, value.key.parent.zd,
        value.key.parent.zg, value.key.seg_a.0, value.key.seg_a.1, value.key.c_start,
        center_ids, value.candidate_group_id, value.pair_id,
        value.structural_predicates.direction, value.structural_predicates.comparable,
        value.structural_predicates.extreme, value.extreme_proof.0, value.extreme_proof.1,
        opt_nat(value.third_class_proof), value.interval.0, value.interval.1,
        observed_state(value.state), opt_nat(value.first_provable_at), opt_nat(value.confirmed_at))
}

fn candidate_lean(value: &CandidateObservation) -> String {
    format!("rustCandidateToMirror {}", candidate_rust(value))
}

fn grade_record_rust(
    value: &newchan_rust::theta_v0::classifier::signal::FirstClassGradeRecord,
) -> String {
    format!(
        concat!(
            "{{ level := {}, sourceIndex := {}, side := {}, centerStart := {}, ",
            "centerEnd := {}, centerZd := {}, centerZg := {}, grade := {} }}"
        ),
        value.level,
        value.source_index,
        rust_side(value.side),
        value.center_start_index,
        value.center_end_index,
        value.center_zd,
        value.center_zg,
        grade(value.grade)
    )
}

fn mirror_list<T>(values: &[T], encode: impl Fn(&T) -> String) -> String {
    format!(
        "[{}]",
        values.iter().map(encode).collect::<Vec<_>>().join(", ")
    )
}

fn scan_emission_expr(
    value: &newchan_rust::theta_v0::classifier::diag::s2_mirror_capture::ScanEmissionCapture,
) -> String {
    format!(
        concat!("{{ points := {}, panDivs := {}, grades := {}, candidateLegs := {} }}"),
        mirror_list(&value.points, bsp_rust),
        mirror_list(&value.pan_divs, pan_rust),
        mirror_list(&value.grades, grade_record_rust),
        mirror_list(&value.candidate_legs, candidate_rust)
    )
}

fn four_cache_expr(
    value: &newchan_rust::theta_v0::classifier::diag::s2_mirror_capture::FourCacheCapture,
) -> String {
    format!(concat!("{{ points := {}, panDivs := {}, grades := {}, candidateLegs := {}, cachedCount := {} }}"),
        mirror_list(&value.points, bsp_rust), mirror_list(&value.pan_divs, pan_rust),
        mirror_list(&value.grades, grade_record_rust),
        mirror_list(&value.candidate_legs, candidate_rust), value.cached_count)
}

fn merged_output_expr(
    value: &newchan_rust::theta_v0::classifier::diag::s2_mirror_capture::ScanEmissionCapture,
) -> String {
    format!(
        concat!("{{ points := {}, panDivs := {}, grades := {}, observations := {} }}"),
        mirror_list(&value.points, bsp_rust),
        mirror_list(&value.pan_divs, pan_rust),
        mirror_list(&value.grades, grade_record_rust),
        mirror_list(&value.observations, candidate_rust)
    )
}

fn merged_rust_lists(
    points: &[BspPoint],
    pan_divs: &[PanDivCert],
    grades: &[newchan_rust::theta_v0::classifier::signal::FirstClassGradeRecord],
    observations: &[CandidateObservation],
) -> String {
    format!(
        concat!("{{ points := {}, panDivs := {}, grades := {}, observations := {} }}"),
        mirror_list(points, bsp_rust),
        mirror_list(pan_divs, pan_rust),
        mirror_list(grades, grade_record_rust),
        mirror_list(observations, candidate_rust)
    )
}

fn second_leg_raw_rust(value: &SecondLegRawCapture) -> String {
    format!(
        concat!(
            "{{ direction := {}, lo := {}, hi := {}, diverged := {}, ",
            "sourceIndex := {} }}"
        ),
        dir(value.direction),
        value.lo,
        value.hi,
        value.diverged,
        value.source_index
    )
}

fn second_production_rust(value: &SecondProductionCapture) -> String {
    format!(
        concat!(
            "{{ center := {}, side := {}, level := {}, legs := {}, ",
            "output := {} }}"
        ),
        center(&value.center),
        rust_side(value.side),
        value.level,
        mirror_list(&value.legs, second_leg_raw_rust),
        mirror_list(&value.output, bsp_rust)
    )
}

/// 全部 actual 先构造成 Rust 生产类型，再复用窗口 serializer；Lean 只持独立 raw/expected。
fn coverage_fixture_wire() -> String {
    use newchan_rust::theta_v0::classifier::diag::s2_mirror_capture::FourCacheCapture;
    use newchan_rust::theta_v0::classifier::signal::FirstClassGradeRecord;

    let center0 = Center {
        start_index: 10,
        end_index: 20,
        zd: 100,
        zg: 120,
        dd: 90,
        gg: 130,
    };
    let mut first_bits = BspBits::default();
    first_bits.buy1 = true;
    let first_some = BspPoint {
        source_index: 29,
        bits: first_bits,
        pivot_low: 80,
        pivot_high: 0,
        center: Some(OwnerRef::Center(center0)),
        struct_break_dir: Some(Side::Long),
        force: None,
        retrace_breaks_type1: None,
    };
    let raw_a = ForceSegmentRawCapture {
        hist_bits: vec![
            (-1.0_f64).to_bits(),
            (-2.0_f64).to_bits(),
            1.0_f64.to_bits(),
        ],
        dif_bits: vec![
            (-1.0_f64).to_bits(),
            (-3.0_f64).to_bits(),
            (-2.0_f64).to_bits(),
        ],
        closes: vec![100, 90, 80],
        direction: Direction::Down,
    };
    let raw_c = ForceSegmentRawCapture {
        hist_bits: vec![(-0.5_f64).to_bits(), (-1.0_f64).to_bits()],
        dif_bits: vec![(-1.0_f64).to_bits(), (-2.0_f64).to_bits()],
        closes: vec![90, 80],
        direction: Direction::Down,
    };
    let first_force = BspPoint {
        force: Some(ForceProxies {
            seg_a: newchan_rust::theta_v0::classifier::divergence::force_features(
                &[-1.0, -2.0, 1.0],
                &[-1.0, -3.0, -2.0],
                &[100, 90, 80],
                0,
                2,
                Direction::Down,
            ),
            seg_c: newchan_rust::theta_v0::classifier::divergence::force_features(
                &[-0.5, -1.0],
                &[-1.0, -2.0],
                &[90, 80],
                0,
                1,
                Direction::Down,
            ),
        }),
        ..first_some
    };
    let second_production = s2_mirror_capture::run_second_production_diagnostic(
        Center {
            start_index: 0,
            end_index: 0,
            zd: 0,
            zg: 4,
            dd: -2,
            gg: 6,
        },
        Side::Long,
        1,
        vec![
            SecondLegRawCapture {
                direction: Direction::Down,
                lo: -10,
                hi: -2,
                diverged: true,
                source_index: 30,
            },
            SecondLegRawCapture {
                direction: Direction::Up,
                lo: -8,
                hi: 3,
                diverged: false,
                source_index: 42,
            },
            SecondLegRawCapture {
                direction: Direction::Up,
                lo: 1,
                hi: 5,
                diverged: false,
                source_index: 50,
            },
        ],
    );
    assert!(
        matches!(
            second_production
                .output
                .first()
                .and_then(|point| point.center),
            Some(OwnerRef::Type1Anchor(30))
        ),
        "production second fixture 必须真实产 Type1Anchor"
    );
    // 旧 wire 变体 smoke 保持自身固定 payload；生产语义覆盖只由上面的 secondProduction
    // raw adapter 承重，不能从本手写变体取得 top-level schema credit。
    let owner_type1 = BspPoint {
        source_index: 41,
        bits: BspBits::default(),
        pivot_low: 0,
        pivot_high: 0,
        center: Some(OwnerRef::Type1Anchor(37)),
        struct_break_dir: None,
        force: None,
        retrace_breaks_type1: Some(true),
    };
    let mut third_bits = BspBits::default();
    third_bits.buy3 = true;
    third_bits.third_class_entry = Some(ThirdClassEntryIdentity {
        center_si: 1,
        center_zd: 10,
        center_zg: 20,
        leave_interval: (6, 7),
        retest_interval: (8, 9),
    });
    let third_success = BspPoint {
        source_index: 9,
        bits: third_bits,
        pivot_low: 25,
        pivot_high: 0,
        center: Some(OwnerRef::Center(Center {
            start_index: 1,
            end_index: 5,
            zd: 10,
            zg: 20,
            dd: 0,
            gg: 0,
        })),
        struct_break_dir: None,
        force: None,
        retrace_breaks_type1: None,
    };
    let grade_record = |grade| FirstClassGradeRecord {
        level: 0,
        source_index: 29,
        side: Side::Long,
        center_start_index: 10,
        center_end_index: 20,
        center_zd: 100,
        center_zg: 120,
        grade,
    };
    // ★#1249（#1229 裁定 a）：固定首对五桶随 `T3InCScan`（Missing 不携 reason）退役——本 fixture
    // 分级字段由 `gradePresent + 五个 gradeMissing*` 收成 `gradePresent + gradeMissing` 两档。
    // ⚠ Lean `CoverageFixtureExtraction`（formal/Origin/UnifiedScanMirrorRunner.lean:1289）仍持
    // 五桶字段、`coverageFixtureChecks`（同文件 :1392-1400）仍发五条 `coverageGradeCheck`——
    // 本 fixture 生成的 Lean 结构字面量因此在 Lean 侧补后扫镜面前不闭合（`gradeMissing` 字段 Lean
    // 尚无、五桶字段缺省）。收口归后续 Lean 票：五桶字段换单一 `gradeMissing`、五条 check 换成
    // `coverageGradeCheck "o03.missing" (.missing .missingLeave) wire.gradeMissing`（占位 reason
    // 与 `grade()` 的 `RustGradeExtractionValue.missing` 同源，见其注释）。
    let grade_present = grade_record(T3InCScan::Present {
        leave_interval: (1, 2),
        retest_interval: (3, 4),
    });
    let grade_missing = grade_record(T3InCScan::Missing);
    let candidate_key = CandidateKey {
        rule_version: 1,
        level: 2,
        kind: CandidateKind::Trend,
        side: Side::Short,
        previous_center_start: Some(11),
        parent: ParentFingerprint {
            center_start: 21,
            zd: 100,
            zg: 120,
        },
        seg_a: (1, 9),
        c_start: 22,
    };
    let candidate_true = CandidateObservation {
        key: candidate_key,
        kind: CandidateKind::Trend,
        center_ids: Some((3, 4)),
        candidate_group_id: 101,
        pair_id: 202,
        structural_predicates: StructuralPredicates {
            direction: true,
            comparable: true,
            extreme: true,
        },
        extreme_proof: (5, 6),
        third_class_proof: Some(7),
        interval: (22, 30),
        state: ObservedState::Confirmed,
        first_provable_at: Some(30),
        confirmed_at: None,
    };
    let candidate_false = CandidateObservation {
        key: CandidateKey {
            previous_center_start: None,
            ..candidate_key
        },
        center_ids: None,
        structural_predicates: StructuralPredicates {
            extreme: false,
            ..candidate_true.structural_predicates
        },
        third_class_proof: None,
        first_provable_at: None,
        confirmed_at: Some(31),
        ..candidate_true.clone()
    };

    let id = |level, ordinal| ElementId { level, ordinal };
    let third_cp = ThirdClassInCp {
        b_center_id: id(2, 3),
        cp_departure_move_id: id(1, 5),
        departure_move_id: id(1, 6),
        retest_move_id: id(1, 7),
        departure_interval: (21, 25),
        retest_interval: (26, 30),
        point_source_index: 30,
        side: Side::Long,
    };
    let context = TrendContext {
        predecessor_center_id: id(2, 2),
        b_center_id: id(2, 3),
        direction: Direction::Up,
    };
    let extreme = NewExtremeInDirection {
        b_center_id: id(2, 3),
        direction: Direction::Up,
        reference_price: 130,
        extreme_price: 140,
        extreme_move_id: id(1, 6),
        confirm_src: 25,
    };
    let internal = InternalSublevelCenters {
        c_level: 1,
        center_ids: vec![id(1, 5), id(1, 6)],
    };
    let completed = CompletedTrendDecomposition {
        direction: Direction::Up,
        center_ids: internal.center_ids.clone(),
        closing_successor_move_id: id(1, 8),
        confirm_src: 35,
    };
    let evidence = FullTrendQualificationEvidence {
        trend_context: Some(context),
        new_extreme_in_direction: Some(extreme),
        internal_sublevel_centers: Some(internal.clone()),
        completed_trend_decomposition: Some(completed.clone()),
        decomposition_review_move_id: Some(id(1, 8)),
        decomposition_review_src: Some(35),
    };
    let cp = CpScanOwnership {
        b_center_index: 3,
        b_center_id: id(2, 3),
        b_center: center0,
        departure_move_id: Some(id(1, 5)),
        departure_interval: Some((21, 25)),
        lifecycle: CpLifecycleStatus::Closed,
        cp_certificate_confirm_src: Some(30),
        c_structure: Some(CpStructureIdentity {
            level: 1,
            b_center_id: id(2, 3),
            departure_move_id: id(1, 5),
            terminal_move_id: Some(id(1, 7)),
            source_start: 21,
            source_end: Some(30),
        }),
        third_class_in_c: Some(third_cp),
        full_trend_evidence: Some(evidence),
        full_trend_c_qualified: Some(FullTrendCQualified {
            trend_context: context,
            third_class_inside_c: third_cp,
            new_extreme_in_direction: extreme,
            internal_sublevel_centers: internal,
            completed_trend_decomposition: completed,
            confirm_src: 35,
        }),
    };

    let floats = |bits: &[u64]| bits.iter().copied().map(f64::from_bits).collect::<Vec<_>>();

    let first_hist_bits = [
        13_835_058_055_282_163_712,
        13_835_058_055_282_163_712,
        0,
        0,
        13_821_547_256_400_052_224,
        13_821_547_256_400_052_224,
        13_821_547_256_400_052_224,
        13_821_547_256_400_052_224,
    ];
    let first_hist = floats(&first_hist_bits);
    let first_dif = vec![0.0; 8];
    let first_closes = [100, 75, 75, 80, 100, 50, 50, 55];
    let first_close_src = [5, 6, 7, 8, 10, 11, 12, 13];
    let first_centers = [
        Center {
            start_index: 0,
            end_index: 4,
            zd: 90,
            zg: 110,
            dd: 90,
            gg: 110,
        },
        Center {
            start_index: 5,
            end_index: 9,
            zd: 60,
            zg: 80,
            dd: 60,
            gg: 80,
        },
        Center {
            start_index: 14,
            end_index: 14,
            zd: 30,
            zg: 50,
            dd: 30,
            gg: 50,
        },
        Center {
            start_index: 15,
            end_index: 20,
            zd: 10,
            zg: 25,
            dd: 10,
            gg: 25,
        },
    ];
    let first_segments = [
        Segment {
            direction: Direction::Down,
            start_index: 5,
            end_index: 6,
            start_price: 100,
            end_price: 75,
        },
        Segment {
            direction: Direction::Up,
            start_index: 7,
            end_index: 8,
            start_price: 75,
            end_price: 80,
        },
        Segment {
            direction: Direction::Down,
            start_index: 10,
            end_index: 11,
            start_price: 100,
            end_price: 50,
        },
        Segment {
            direction: Direction::Up,
            start_index: 12,
            end_index: 13,
            start_price: 50,
            end_price: 55,
        },
    ];
    let first_blocks = decompose(&first_centers);
    let (p10_first_series, p10_first_frontier) = s2_mirror_capture::capture_fixture_frontier(
        FourCacheCapture::default(),
        0,
        &first_centers,
        &first_segments,
        None,
        None,
        &first_blocks,
        4,
        14,
        &first_hist,
        &first_dif,
        &first_closes,
        &first_close_src,
        DivergenceGauge::MacdArea,
        &[],
    );
    let (p10_first_tail_series, p10_first_tail_frontier) =
        s2_mirror_capture::capture_fixture_frontier(
            FourCacheCapture::default(),
            0,
            &first_centers,
            &first_segments,
            None,
            None,
            &first_blocks,
            0,
            0,
            &first_hist,
            &first_dif,
            &first_closes,
            &first_close_src,
            DivergenceGauge::MacdArea,
            &[],
        );
    let pan_hist_bits = [
        13_835_058_055_282_163_712,
        13_835_058_055_282_163_712,
        13_835_058_055_282_163_712,
        13_821_547_256_400_052_224,
        13_821_547_256_400_052_224,
    ];
    let pan_hist = floats(&pan_hist_bits);
    let pan_dif = vec![0.0; 5];
    let pan_closes = [20, 15, 5, 12, 5];
    let pan_close_src = [1, 2, 3, 6, 7];
    let pan_centers = [Center {
        start_index: 4,
        end_index: 5,
        zd: 10,
        zg: 20,
        dd: 5,
        gg: 25,
    }];
    let pan_segments = [
        Segment {
            direction: Direction::Down,
            start_index: 1,
            end_index: 3,
            start_price: 20,
            end_price: 5,
        },
        Segment {
            direction: Direction::Down,
            start_index: 6,
            end_index: 7,
            start_price: 12,
            end_price: 5,
        },
    ];
    let pan_blocks = [MoveBlock {
        start_center: 0,
        end_center: 0,
        kind: MoveKind::Consolidation,
        dir: None,
        level_lift: 0,
        status: MoveStatus::Active,
    }];
    let (p10_pan_series, p10_pan_frontier) = s2_mirror_capture::capture_fixture_frontier(
        FourCacheCapture::default(),
        0,
        &pan_centers,
        &pan_segments,
        None,
        None,
        &pan_blocks,
        0,
        0,
        &pan_hist,
        &pan_dif,
        &pan_closes,
        &pan_close_src,
        DivergenceGauge::MacdArea,
        &[],
    );
    // 独立的合法 confirmed-pan raw：后接两个价格重叠、end 更晚的中枢，使全链仍为
    // 本级 Consolidation；prefix=3 将两段都冻结进 confirmed/cacheAfter，并由同一次
    // 生产调用归约出最终 pan observation。
    let pan_cache_centers = [
        pan_centers[0],
        Center {
            start_index: 8,
            end_index: 10,
            zd: 12,
            zg: 18,
            dd: 8,
            gg: 22,
        },
        Center {
            start_index: 11,
            end_index: 12,
            zd: 11,
            zg: 19,
            dd: 7,
            gg: 23,
        },
    ];
    let pan_cache_blocks = decompose(&pan_cache_centers);
    let (p10_cache_series, p10_cache_frontier) = s2_mirror_capture::capture_fixture_frontier(
        FourCacheCapture::default(),
        0,
        &pan_cache_centers,
        &pan_segments,
        None,
        None,
        &pan_cache_blocks,
        3,
        12,
        &pan_hist,
        &pan_dif,
        &pan_closes,
        &pan_close_src,
        DivergenceGauge::MacdArea,
        &[],
    );

    let third_centers = [Center {
        start_index: 1,
        end_index: 5,
        zd: 10,
        zg: 20,
        dd: 0,
        gg: 0,
    }];
    let third_segments = [
        Segment {
            direction: Direction::Up,
            start_index: 6,
            end_index: 7,
            start_price: 20,
            end_price: 30,
        },
        Segment {
            direction: Direction::Down,
            start_index: 8,
            end_index: 9,
            start_price: 30,
            end_price: 25,
        },
    ];
    let (p10_third_series, p10_third_frontier) = s2_mirror_capture::capture_fixture_frontier(
        FourCacheCapture::default(),
        0,
        &third_centers,
        &third_segments,
        None,
        None,
        &[],
        0,
        0,
        &[],
        &[],
        &[],
        &[],
        DivergenceGauge::ForceL,
        &[],
    );
    let raw_expr = |raw: &ForceSegmentRawCapture| {
        format!(
            "{{ histBits := {}, difBits := {}, closes := {}, direction := {} }}",
            u64_nat_list(&raw.hist_bits),
            u64_nat_list(&raw.dif_bits),
            i64_int_list(&raw.closes),
            dir(raw.direction)
        )
    };
    format!(
        concat!(
            "{{ firstSome := (some {}), firstNone := none, forceSegA := {}, forceSegC := {}, ",
            "firstForce := (some {}), ownerType1 := {}, secondProduction := {}, thirdSuccess := (some {}), thirdFailure := none, ",
            "gradePresent := {}, gradeMissing := {}, ",
            "candidateExtremeTrue := {}, candidateExtremeFalse := {}, cpFull := {}, ",
            "p10Cache := {}, p10First := {}, p10FirstTail := {}, p10Pan := {}, p10Third := {}, ",
            "pipelineDirect := [{}], pipelineSecond07b := [{}], pipelineOutput := [{}, {}] }}"
        ),
        bsp_rust(&first_some), raw_expr(&raw_a), raw_expr(&raw_c), bsp_rust(&first_force),
        bsp_rust(&owner_type1), second_production_rust(&second_production),
        bsp_rust(&third_success), grade_record_rust(&grade_present),
        grade_record_rust(&grade_missing), candidate_rust(&candidate_true),
        candidate_rust(&candidate_false), cp_full_rust(&cp),
        p10_coverage_case_expr(&p10_cache_series, &p10_cache_frontier),
        p10_coverage_case_expr(&p10_first_series, &p10_first_frontier),
        p10_coverage_case_expr(&p10_first_tail_series, &p10_first_tail_frontier),
        p10_coverage_case_expr(&p10_pan_series, &p10_pan_frontier),
        p10_coverage_case_expr(&p10_third_series, &p10_third_frontier),
        bsp_rust(&first_force), bsp_rust(&owner_type1),
        bsp_rust(&first_force), bsp_rust(&owner_type1)
    )
}

fn frontier_check(value: &FrontierCapture, path: &str, series_name: &str) -> String {
    format!(
        "checkMergedScan \"{path}\" {series_name} {} {} {} {} {} {} {}",
        raw_resume_expr(value),
        four_cache_expr(&value.cache_before),
        value.stable_seg,
        scan_emission_expr(&value.confirmed_append),
        four_cache_expr(&value.cache_after),
        scan_emission_expr(&value.tail),
        merged_output_expr(&value.final_output)
    )
}

fn raw_segment(value: &Segment) -> String {
    format!(
        "{{ direction := {}, startIndex := {}, endIndex := {}, startPrice := {}, endPrice := {} }}",
        dir(value.direction),
        value.start_index,
        value.end_index,
        value.start_price,
        value.end_price
    )
}

fn raw_stroke(value: &Stroke) -> String {
    format!(
        "{{ direction := {}, startIndex := {}, endIndex := {}, startPrice := {}, endPrice := {} }}",
        dir(value.direction),
        value.start_index,
        value.end_index,
        value.start_price,
        value.end_price
    )
}

fn raw_block(value: &MoveBlock) -> String {
    let kind = match value.kind {
        MoveKind::Trend => "MoveKind.trend",
        MoveKind::Consolidation => "MoveKind.consolidation",
    };
    let direction = value.dir.map_or_else(
        || "none".to_owned(),
        |direction| format!("(some {})", dir(direction)),
    );
    format!(
        "{{ startCenter := {}, endCenter := {}, kind := {kind}, direction := {direction}, levelLift := {} }}",
        value.start_center, value.end_center, value.level_lift
    )
}

fn opt_direction_list(value: Option<&[Option<Direction>]>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |values| {
            format!(
                "(some [{}])",
                values
                    .iter()
                    .map(|value| value.map_or_else(
                        || "none".to_owned(),
                        |direction| format!("(some {})", dir(direction))
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RawSequencePlan<T> {
    Prefix { len: usize },
    TailPatch { prefix_len: usize, tail: T },
    Fallback(Vec<T>),
}

impl<T: Clone> RawSequencePlan<T> {
    fn rebuild(&self, base: &[T]) -> Result<Vec<T>, String> {
        match self {
            Self::Prefix { len } => base
                .get(..*len)
                .map(<[T]>::to_vec)
                .ok_or_else(|| format!("raw prefix {len} 越过基底长度 {}", base.len())),
            Self::TailPatch { prefix_len, tail } => {
                let mut rebuilt = base.get(..*prefix_len).map(<[T]>::to_vec).ok_or_else(|| {
                    format!("raw tail patch {prefix_len} 越过基底长度 {}", base.len())
                })?;
                rebuilt.push(tail.clone());
                Ok(rebuilt)
            }
            Self::Fallback(values) => Ok(values.clone()),
        }
    }
}

fn classify_raw_sequence<T: Clone + Eq>(values: &[T], base: &[T]) -> RawSequencePlan<T> {
    if values.len() <= base.len() && values == &base[..values.len()] {
        RawSequencePlan::Prefix { len: values.len() }
    } else if !values.is_empty()
        && values.len() <= base.len()
        && values[..values.len() - 1] == base[..values.len() - 1]
    {
        RawSequencePlan::TailPatch {
            prefix_len: values.len() - 1,
            tail: values.last().expect("non-empty checked").clone(),
        }
    } else {
        RawSequencePlan::Fallback(values.to_vec())
    }
}

fn raw_sequence_expr<T>(
    plan: &RawSequencePlan<T>,
    base_name: &str,
    encode_list: impl Fn(&[T]) -> String,
    encode_item: impl Fn(&T) -> String,
) -> String {
    match plan {
        RawSequencePlan::Prefix { len } => format!("{base_name}.take {len}"),
        RawSequencePlan::TailPatch { prefix_len, tail } => {
            format!("({base_name}.take {prefix_len} ++ [{}])", encode_item(tail))
        }
        RawSequencePlan::Fallback(values) => encode_list(values),
    }
}

fn raw_gauge_expr(gauge: DivergenceGauge) -> &'static str {
    match gauge {
        DivergenceGauge::ForceL => "DivergenceGaugeMirror.forceL",
        DivergenceGauge::MacdArea => "DivergenceGaugeMirror.macdArea",
        DivergenceGauge::ThetaDom => "DivergenceGaugeMirror.thetaDom",
        DivergenceGauge::Conjunction => "DivergenceGaugeMirror.conjunction",
        DivergenceGauge::ThetaLex => "DivergenceGaugeMirror.thetaLex",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawSeriesDefinitionPlan {
    hist_bits: RawSequencePlan<u64>,
    dif_bits: RawSequencePlan<u64>,
    closes_ticks: RawSequencePlan<i64>,
    close_src: RawSequencePlan<usize>,
    gauge: DivergenceGauge,
    strokes: RawSequencePlan<Stroke>,
}

impl RawSeriesDefinitionPlan {
    fn from_capture(
        name: &str,
        value: &s2_mirror_capture::RawSeriesCapture,
        base: &s2_mirror_capture::RawSeriesCapture,
    ) -> Result<Self, String> {
        let plan = Self {
            hist_bits: classify_raw_sequence(&value.hist_bits, &base.hist_bits),
            dif_bits: classify_raw_sequence(&value.dif_bits, &base.dif_bits),
            closes_ticks: classify_raw_sequence(&value.closes_ticks, &base.closes_ticks),
            close_src: classify_raw_sequence(&value.close_src, &base.close_src),
            gauge: value.gauge,
            strokes: classify_raw_sequence(&value.strokes, &base.strokes),
        };
        macro_rules! verify {
            ($field:ident) => {
                if plan.$field.rebuild(&base.$field)? != value.$field {
                    return Err(format!(
                        "raw definition {name}.{} 压缩后重建不等于捕获原值",
                        stringify!($field)
                    ));
                }
            };
        }
        verify!(hist_bits);
        verify!(dif_bits);
        verify!(closes_ticks);
        verify!(close_src);
        verify!(strokes);
        Ok(plan)
    }

    fn lean_expr(&self) -> String {
        format!(
            concat!(
                "{{ histBits := {}, difBits := {}, closesTicks := {}, closeSrc := {}, ",
                "gauge := {}, strokes := {} }}"
            ),
            raw_sequence_expr(
                &self.hist_bits,
                "rawSeriesBaseHistBits",
                u64_nat_list,
                u64::to_string,
            ),
            raw_sequence_expr(
                &self.dif_bits,
                "rawSeriesBaseDifBits",
                u64_nat_list,
                u64::to_string,
            ),
            raw_sequence_expr(
                &self.closes_ticks,
                "rawSeriesBaseClosesTicks",
                i64_int_list,
                i64::to_string,
            ),
            raw_sequence_expr(
                &self.close_src,
                "rawSeriesBaseCloseSrc",
                nat_list,
                usize::to_string,
            ),
            raw_gauge_expr(self.gauge),
            raw_sequence_expr(
                &self.strokes,
                "rawSeriesBaseStrokes",
                |values| mirror_list(values, raw_stroke),
                raw_stroke,
            ),
        )
    }
}

#[derive(Debug)]
struct RawSeriesWindowPlan {
    base_definition_name: String,
    base: s2_mirror_capture::RawSeriesCapture,
    definitions: BTreeMap<String, RawSeriesDefinitionPlan>,
}

impl RawSeriesWindowPlan {
    fn new(
        captures: &BTreeMap<String, s2_mirror_capture::RawSeriesCapture>,
    ) -> Result<Self, String> {
        let (base_definition_name, base) = captures
            .iter()
            .max_by(|(left_name, left), (right_name, right)| {
                raw_series_extent(left)
                    .cmp(&raw_series_extent(right))
                    .then_with(|| left_name.cmp(right_name))
            })
            .ok_or_else(|| "窗口没有 raw series capture".to_owned())?;
        let definitions = captures
            .iter()
            .map(|(name, value)| {
                RawSeriesDefinitionPlan::from_capture(name, value, base)
                    .map(|plan| (name.clone(), plan))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        Ok(Self {
            base_definition_name: base_definition_name.clone(),
            base: base.clone(),
            definitions,
        })
    }

    fn base_definitions(&self) -> String {
        format!(
            concat!(
                "-- window raw base selected from {}\n",
                "def rawSeriesBaseHistBits : List Nat := {}\n",
                "def rawSeriesBaseDifBits : List Nat := {}\n",
                "def rawSeriesBaseClosesTicks : List Int := {}\n",
                "def rawSeriesBaseCloseSrc : List Nat := {}\n",
                "def rawSeriesBaseStrokes : List RawStrokeFrame := {}\n\n"
            ),
            self.base_definition_name,
            u64_nat_list(&self.base.hist_bits),
            u64_nat_list(&self.base.dif_bits),
            i64_int_list(&self.base.closes_ticks),
            nat_list(&self.base.close_src),
            mirror_list(&self.base.strokes, raw_stroke),
        )
    }

    fn base_module_source(&self) -> String {
        format!(
            concat!(
                "import Origin.UnifiedScanMirrorBridge\n\n",
                "set_option maxHeartbeats 0\n",
                // W500 的公共基底包含十万量级 List literal；100000 在 W100 已实跑触顶。
                "set_option maxRecDepth 1000000\n\n",
                "open NewChanlun.Origin.UnifiedScanMirrorBridge\n\n",
                "{}"
            ),
            self.base_definitions()
        )
    }
}

fn raw_series_extent(
    value: &s2_mirror_capture::RawSeriesCapture,
) -> (usize, usize, usize, usize, usize) {
    (
        value.hist_bits.len(),
        value.dif_bits.len(),
        value.closes_ticks.len(),
        value.close_src.len(),
        value.strokes.len(),
    )
}

fn raw_series_expr(value: &s2_mirror_capture::RawSeriesCapture) -> String {
    format!(
        "{{ histBits := {}, difBits := {}, closesTicks := {}, closeSrc := {}, gauge := {}, strokes := {} }}",
        u64_nat_list(&value.hist_bits),
        u64_nat_list(&value.dif_bits),
        i64_int_list(&value.closes_ticks),
        nat_list(&value.close_src),
        raw_gauge_expr(value.gauge),
        mirror_list(&value.strokes, raw_stroke)
    )
}

fn raw_resume_expr(value: &FrontierCapture) -> String {
    format!(
        concat!(
            "{{ level := {}, centers := {}, segments := {}, anchorDirs := {}, ",
            "departureEnds := {}, blocks := {}, prefixCount := {}, dirtyE := {} }}"
        ),
        value.level,
        mirror_list(&value.centers, center),
        mirror_list(&value.segments, raw_segment),
        opt_direction_list(value.anchor_dirs.as_deref()),
        value.departure_ends.as_ref().map_or_else(
            || "none".to_owned(),
            |ends| format!("(some {})", nat_list(ends))
        ),
        mirror_list(&value.blocks, raw_block),
        value.prefix_count,
        value.dirty_e
    )
}

fn p10_coverage_case_expr(
    series: &s2_mirror_capture::RawSeriesCapture,
    frontier: &FrontierCapture,
) -> String {
    format!(
        concat!(
            "{{ series := {}, raw := {}, stableSeg := {}, cacheBefore := {}, ",
            "confirmedAppend := {}, cacheAfter := {}, tail := {}, output := {} }}"
        ),
        raw_series_expr(series),
        raw_resume_expr(frontier),
        frontier.stable_seg,
        four_cache_expr(&frontier.cache_before),
        scan_emission_expr(&frontier.confirmed_append),
        four_cache_expr(&frontier.cache_after),
        scan_emission_expr(&frontier.tail),
        merged_output_expr(&frontier.final_output)
    )
}

fn memo_key_nat(value: (usize, usize, usize)) -> String {
    format!(
        "(({} * 340282366920938463463374607431768211456) + ({} * 18446744073709551616) + {})",
        value.0, value.1, value.2
    )
}

fn bool_list(values: &[bool]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(bool::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn nat_list(values: &[usize]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn u64_nat_list(values: &[u64]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn i64_int_list(values: &[i64]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn memo_check(value: &FourRcMemoCapture, expected: &MemoPortWitness, path: &str) -> String {
    let cached_key = value.cached_key_before.map_or_else(
        || "none".to_owned(),
        |key| format!("(some {})", memo_key_nat(key)),
    );
    format!(
        concat!(
            "checkFourRcMemo \"{}\" {} {{ key := {}, cachedKey := {}, hit := {}, ",
            "priorReused := {}, afterPtrEq := {}, beforeLengths := {}, afterLengths := {}, ",
            "expectedPoints := {}, actualPoints := {}, expectedPanDivs := {}, actualPanDivs := {}, ",
            "expectedGrades := {}, actualGrades := {}, expectedCandidateLegs := {}, actualCandidateLegs := {} }}"
        ),
        path,
        value.level,
        memo_key_nat(value.key),
        cached_key,
        value.hit,
        bool_list(&value.prior_reused),
        bool_list(&value.cache_after_matches_return),
        nat_list(&value.before_lengths),
        nat_list(&value.lengths),
        mirror_list(&expected.pipeline_points, bsp_rust),
        mirror_list(&value.pipeline_points, bsp_rust),
        mirror_list(&expected.pan_divs, pan_rust),
        mirror_list(&value.pan_divs, pan_rust),
        mirror_list(&expected.grades, grade_record_rust),
        mirror_list(&value.grades, grade_record_rust),
        mirror_list(&expected.candidate_legs, candidate_rust),
        mirror_list(&value.candidates, candidate_rust)
    )
}

fn t3_raw_segment(value: Option<Segment>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |s| format!("(some {})", segment(&s, Some(s.direction), None)),
    )
}

fn first_input_expr(value: &FirstAssemblyCapture) -> String {
    let center = center(&value.center);
    let leave = t3_raw_segment(value.t3_leave);
    let retest = t3_raw_segment(value.t3_retest);
    let grade_expr = format!(
        "gradeFixedFirstPair {center} {} {leave} {retest}",
        dir(value.trend_dir)
    );
    let force_l_diverged = format!(
        "forceLDivergedBits {} {}",
        opt_u64_as_nat(value.force_l_a),
        opt_u64_as_nat(value.force_l_c)
    );
    let diverged = format!(
        "({} && {} && {force_l_diverged})",
        value.extreme, value.comparable
    );
    let t3_present = format!("(match {grade_expr} with | Grade.present _ _ => true | _ => false)");
    let gates = format!("{{ side := {}, segA := {{ left := {}, right := {} }}, lambdaC := {}, aMapped := {}, cMapped := {}, extreme := {} }}",
        side(value.side), value.seg_a.0, value.seg_a.1, value.lambda_c,
        value.comparable, value.comparable, value.extreme);
    let force_a = force_segment_raw(value.force_seg_a_raw.as_ref());
    let force_c = force_segment_raw(value.force_seg_c_raw.as_ref());
    let input = format!("{{ level := {}, center := {}, segment := {}, gates := {}, diverged := {}, t3Present := {}, grade := {}, forceSegA := {}, forceSegC := {} }}",
        value.level, center, segment(&value.segment, Some(value.segment.direction), value.departure_end),
        gates, diverged, t3_present, grade_expr,
        force_a, force_c);
    input
}

fn first_rust(value: &FirstAssemblyCapture) -> String {
    value.output.as_ref().map_or_else(
        || "none".to_owned(),
        |point| format!("(some {})", bsp_rust(point)),
    )
}

fn grade_rust(value: &FirstAssemblyCapture) -> Option<String> {
    let record = value.grade_output.as_ref()?;
    Some(format!(
        concat!(
            "{{ level := {}, sourceIndex := {}, side := {}, centerStart := {}, ",
            "centerEnd := {}, centerZd := {}, centerZg := {}, grade := {} }}"
        ),
        record.level,
        record.source_index,
        rust_side(record.side),
        record.center_start_index,
        record.center_end_index,
        record.center_zd,
        record.center_zg,
        grade(record.grade)
    ))
}

fn force_segment_raw(raw: Option<&ForceSegmentRawCapture>) -> String {
    raw.map_or_else(
        || "none".to_owned(),
        |raw| {
            format!(
                "(some {{ histBits := {}, difBits := {}, closes := {}, direction := {} }})",
                u64_nat_list(&raw.hist_bits),
                u64_nat_list(&raw.dif_bits),
                i64_int_list(&raw.closes),
                dir(raw.direction)
            )
        },
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

fn third_check(value: &ThirdAssemblyCapture, path: &str) -> String {
    let input = format!(
        "{{ center := {}, leave := {}, retest := {} }}",
        center(&value.center),
        segment(&value.leave, value.leave_anchor, None),
        segment(&value.retest, Some(value.retest.direction), None)
    );
    let rust = value.output.as_ref().map_or_else(
        || "none".to_owned(),
        |point| format!("(some {})", bsp_rust(point)),
    );
    let side = value.output.as_ref().map_or("NotYetKnown", |point| {
        if point.bits.buy3 {
            "Long"
        } else if point.bits.sell3 {
            "Short"
        } else {
            "NotYetKnown"
        }
    });
    format!(
        "checkThird \"{path}\" {} \"{side}\" {input} {rust}",
        value.level
    )
}

fn pan_check(cert: &PanDivCert, level: u32, path: &str) -> String {
    let pan = format!("{{ sourceIndex := {}, side := {}, center := {}, segA := {{ left := {}, right := {} }}, segC := {{ left := {}, right := {} }} }}",
        cert.source_index, side(cert.side), center(&cert.center), cert.seg_a.0, cert.seg_a.1, cert.seg_c.0, cert.seg_c.1);
    let input = format!("{{ consolidation := true, liftZero := true, comparable := true, diverged := true, panStructure := some {pan} }}");
    let rust = format!("{{ sourceIndex := {}, side := {}, centerStart := {}, centerEnd := {}, centerZd := {}, centerZg := {}, centerDd := {}, centerGg := {}, segALeft := {}, segARight := {}, segCLeft := {}, segCRight := {} }}",
        cert.source_index, rust_side(cert.side), cert.center.start_index, cert.center.end_index,
        cert.center.zd, cert.center.zg, cert.center.dd, cert.center.gg,
        cert.seg_a.0, cert.seg_a.1, cert.seg_c.0, cert.seg_c.1);
    format!(
        "checkPan \"{path}\" {level} \"{}\" {input} {rust}",
        side_name(cert.side)
    )
}

fn pan_rust(cert: &PanDivCert) -> String {
    format!("{{ sourceIndex := {}, side := {}, centerStart := {}, centerEnd := {}, centerZd := {}, centerZg := {}, centerDd := {}, centerGg := {}, segALeft := {}, segARight := {}, segCLeft := {}, segCRight := {} }}",
        cert.source_index, rust_side(cert.side), cert.center.start_index, cert.center.end_index,
        cert.center.zd, cert.center.zg, cert.center.dd, cert.center.gg,
        cert.seg_a.0, cert.seg_a.1, cert.seg_c.0, cert.seg_c.1)
}

fn pan_mirror(cert: &PanDivCert) -> String {
    format!("{{ sourceIndex := {}, side := {}, centerStart := {}, centerEnd := {}, centerZd := {}, centerZg := {}, centerDd := {}, centerGg := {}, segA := {{ left := {}, right := {} }}, segC := {{ left := {}, right := {} }} }}",
        cert.source_index, side(cert.side), cert.center.start_index, cert.center.end_index,
        cert.center.zd, cert.center.zg, cert.center.dd, cert.center.gg,
        cert.seg_a.0, cert.seg_a.1, cert.seg_c.0, cert.seg_c.1)
}

fn element_ids(values: &[ElementId]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .copied()
            .map(element_id)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn cp_structure(value: &CpStructureIdentity) -> String {
    format!(
        concat!(
            "{{ level := {}, bCenterId := {}, departureMoveId := {}, terminalMoveId := {}, ",
            "sourceStart := {}, sourceEnd := {} }}"
        ),
        value.level,
        element_id(value.b_center_id),
        element_id(value.departure_move_id),
        opt_element_id(value.terminal_move_id),
        value.source_start,
        opt_nat(value.source_end)
    )
}

fn third_in_cp(value: &ThirdClassInCp) -> String {
    format!(
        concat!(
            "{{ bCenterId := {}, cpDepartureMoveId := {}, departureMoveId := {}, ",
            "retestMoveId := {}, departureInterval := {}, retestInterval := {}, ",
            "pointSourceIndex := {}, side := {} }}"
        ),
        element_id(value.b_center_id),
        element_id(value.cp_departure_move_id),
        element_id(value.departure_move_id),
        element_id(value.retest_move_id),
        interval(value.departure_interval),
        interval(value.retest_interval),
        value.point_source_index,
        side(value.side)
    )
}

fn third_in_cp_rust(value: &ThirdClassInCp) -> String {
    format!(
        concat!(
            "{{ bCenterId := {}, cpDepartureMoveId := {}, departureMoveId := {}, ",
            "retestMoveId := {}, departureInterval := {}, retestInterval := {}, ",
            "pointSourceIndex := {}, side := {} }}"
        ),
        element_id(value.b_center_id),
        element_id(value.cp_departure_move_id),
        element_id(value.departure_move_id),
        element_id(value.retest_move_id),
        interval(value.departure_interval),
        interval(value.retest_interval),
        value.point_source_index,
        rust_side(value.side)
    )
}

fn trend_context(value: &TrendContext) -> String {
    format!(
        "{{ predecessorCenterId := {}, bCenterId := {}, direction := {} }}",
        element_id(value.predecessor_center_id),
        element_id(value.b_center_id),
        rust_dir(value.direction)
    )
}

fn new_extreme(value: &NewExtremeInDirection) -> String {
    format!(
        concat!(
            "{{ bCenterId := {}, direction := {}, referencePrice := {}, extremePrice := {}, ",
            "extremeMoveId := {}, confirmSrc := {} }}"
        ),
        element_id(value.b_center_id),
        rust_dir(value.direction),
        value.reference_price,
        value.extreme_price,
        element_id(value.extreme_move_id),
        value.confirm_src
    )
}

fn internal_centers(value: &InternalSublevelCenters) -> String {
    format!(
        "{{ cLevel := {}, centerIds := {} }}",
        value.c_level,
        element_ids(&value.center_ids)
    )
}

fn completed_trend(value: &CompletedTrendDecomposition) -> String {
    format!(
        concat!(
            "{{ direction := {}, centerIds := {}, closingSuccessorMoveId := {}, ",
            "confirmSrc := {} }}"
        ),
        rust_dir(value.direction),
        element_ids(&value.center_ids),
        element_id(value.closing_successor_move_id),
        value.confirm_src
    )
}

fn opt_expr<T>(value: Option<&T>, encode: impl FnOnce(&T) -> String) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |value| format!("(some {})", encode(value)),
    )
}

fn full_trend_evidence(value: &FullTrendQualificationEvidence) -> String {
    format!(concat!("{{ trendContext := {}, newExtremeInDirection := {}, internalSublevelCenters := {}, ",
        "completedTrendDecomposition := {}, decompositionReviewMoveId := {}, decompositionReviewSrc := {} }}"),
        opt_expr(value.trend_context.as_ref(), trend_context),
        opt_expr(value.new_extreme_in_direction.as_ref(), new_extreme),
        opt_expr(value.internal_sublevel_centers.as_ref(), internal_centers),
        opt_expr(value.completed_trend_decomposition.as_ref(), completed_trend),
        opt_element_id(value.decomposition_review_move_id), opt_nat(value.decomposition_review_src))
}

fn full_trend_qualified(value: &FullTrendCQualified) -> String {
    format!(
        concat!(
            "{{ trendContext := {}, thirdClassInsideC := {}, newExtremeInDirection := {}, ",
            "internalSublevelCenters := {}, completedTrendDecomposition := {}, confirmSrc := {} }}"
        ),
        trend_context(&value.trend_context),
        third_in_cp_rust(&value.third_class_inside_c),
        new_extreme(&value.new_extreme_in_direction),
        internal_centers(&value.internal_sublevel_centers),
        completed_trend(&value.completed_trend_decomposition),
        value.confirm_src
    )
}

fn trend_context_mirror(value: &TrendContext) -> String {
    format!(
        "{{ predecessorCenterId := {}, bCenterId := {}, direction := {} }}",
        element_id(value.predecessor_center_id),
        element_id(value.b_center_id),
        dir(value.direction)
    )
}

fn new_extreme_mirror(value: &NewExtremeInDirection) -> String {
    format!(
        concat!(
            "{{ bCenterId := {}, direction := {}, referencePrice := {}, extremePrice := {}, ",
            "extremeMoveId := {}, confirmSrc := {} }}"
        ),
        element_id(value.b_center_id),
        dir(value.direction),
        value.reference_price,
        value.extreme_price,
        element_id(value.extreme_move_id),
        value.confirm_src
    )
}

fn completed_trend_mirror(value: &CompletedTrendDecomposition) -> String {
    format!(
        "{{ direction := {}, centerIds := {}, closingSuccessorMoveId := {}, confirmSrc := {} }}",
        dir(value.direction),
        element_ids(&value.center_ids),
        element_id(value.closing_successor_move_id),
        value.confirm_src
    )
}

fn full_trend_evidence_mirror(value: &FullTrendQualificationEvidence) -> String {
    format!(concat!("{{ trendContext := {}, newExtremeInDirection := {}, internalSublevelCenters := {}, ",
        "completedTrendDecomposition := {}, decompositionReviewMoveId := {}, decompositionReviewSrc := {} }}"),
        opt_expr(value.trend_context.as_ref(), trend_context_mirror),
        opt_expr(value.new_extreme_in_direction.as_ref(), new_extreme_mirror),
        opt_expr(value.internal_sublevel_centers.as_ref(), internal_centers),
        opt_expr(value.completed_trend_decomposition.as_ref(), completed_trend_mirror),
        opt_element_id(value.decomposition_review_move_id), opt_nat(value.decomposition_review_src))
}

fn full_trend_qualified_mirror(value: &FullTrendCQualified) -> String {
    format!(
        concat!(
            "{{ trendContext := {}, thirdClassInsideC := {}, newExtremeInDirection := {}, ",
            "internalSublevelCenters := {}, completedTrendDecomposition := {}, confirmSrc := {} }}"
        ),
        trend_context_mirror(&value.trend_context),
        third_in_cp(&value.third_class_inside_c),
        new_extreme_mirror(&value.new_extreme_in_direction),
        internal_centers(&value.internal_sublevel_centers),
        completed_trend_mirror(&value.completed_trend_decomposition),
        value.confirm_src
    )
}

fn cp_full_rust(cp: &CpScanOwnership) -> String {
    let lifecycle = match cp.lifecycle {
        CpLifecycleStatus::Pending => "RustCpLifecycleTag.pending",
        CpLifecycleStatus::Closed => "RustCpLifecycleTag.closed",
    };
    format!(concat!("{{ bCenterIndex := {}, bCenterId := {}, bCenter := {}, departureMoveId := {}, ",
        "departureInterval := {}, lifecycle := {}, cpCertificateConfirmSrc := {}, cStructure := {}, ",
        "thirdClassInC := {}, fullTrendEvidence := {}, fullTrendCQualified := {} }}"),
        cp.b_center_index, element_id(cp.b_center_id), rust_center(&cp.b_center),
        opt_element_id(cp.departure_move_id),
        opt_expr(cp.departure_interval.as_ref(), |value| interval(*value)), lifecycle,
        opt_nat(cp.cp_certificate_confirm_src), opt_expr(cp.c_structure.as_ref(), cp_structure),
        opt_expr(cp.third_class_in_c.as_ref(), third_in_cp_rust),
        opt_expr(cp.full_trend_evidence.as_ref(), full_trend_evidence),
        opt_expr(cp.full_trend_c_qualified.as_ref(), full_trend_qualified))
}

fn cp_full_mirror(cp: &CpScanOwnership) -> String {
    let lifecycle = match cp.lifecycle {
        CpLifecycleStatus::Pending => "CpLifecycle.pending",
        CpLifecycleStatus::Closed => "CpLifecycle.closed",
    };
    format!(concat!("{{ bCenterIndex := {}, bCenterId := {}, bCenter := {}, departureMoveId := {}, ",
        "departureInterval := {}, lifecycle := {}, cpCertificateConfirmSrc := {}, cStructure := {}, ",
        "thirdClassInC := {}, fullTrendEvidence := {}, fullTrendCQualified := {} }}"),
        cp.b_center_index, element_id(cp.b_center_id), center(&cp.b_center),
        opt_element_id(cp.departure_move_id),
        opt_expr(cp.departure_interval.as_ref(), |value| interval(*value)), lifecycle,
        opt_nat(cp.cp_certificate_confirm_src), opt_expr(cp.c_structure.as_ref(), cp_structure),
        opt_expr(cp.third_class_in_c.as_ref(), third_in_cp),
        opt_expr(cp.full_trend_evidence.as_ref(), full_trend_evidence_mirror),
        opt_expr(cp.full_trend_c_qualified.as_ref(), full_trend_qualified_mirror))
}

fn cp_move_raw(value: &LeveledMove) -> String {
    let move_center = match &value.rmove {
        RMove::Compose { centers, .. } => centers.last(),
        RMove::Segment { .. } => None,
    };
    format!(
        concat!(
            "{{ id := {}, startIndex := {}, endIndex := {}, lo := {}, hi := {}, ",
            "center := {} }}"
        ),
        element_id(value.id),
        value.start_index,
        value.end_index,
        value.rmove.lo(),
        value.rmove.hi(),
        opt_expr(move_center, center)
    )
}

fn cp_advance_raw(value: &s2_mirror_capture::CpAdvanceCapture) -> String {
    format!(
        concat!(
            "{{ leave := {}, retest := {}, leaveMoveId := {}, retestMoveId := {}, ",
            "centers := {}, visibleMoves := {} }}"
        ),
        segment(&value.leave, value.leave_anchor, None),
        segment(&value.retest, Some(value.retest.direction), None),
        element_id(value.leave_move_id),
        element_id(value.retest_move_id),
        mirror_list(&value.centers, center),
        mirror_list(&value.visible_moves, cp_move_raw)
    )
}

fn cp_check(cp: &CpScanOwnership, expected: &CpScanOwnership, level: u32, path: &str) -> String {
    let terminal = match cp.lifecycle {
        CpLifecycleStatus::Pending => "Pending",
        CpLifecycleStatus::Closed => "Closed",
    };
    format!(
        "checkCpAttachment \"{path}\" {level} \"{terminal}\" {} {}",
        cp_full_rust(cp),
        cp_full_mirror(expected)
    )
}

fn cp_key(cp: &CpScanOwnership) -> (u32, u64) {
    (cp.b_center_id.level, cp.b_center_id.ordinal)
}

/// Emit raw lifecycle transition checks and return the latest independently checked state by B_p id.
fn append_cp_transition_checks(
    checks: &mut Vec<String>,
    batch: &CaptureBatch,
    checkpoint: usize,
    known: &BTreeMap<(u32, u64), CpScanOwnership>,
) -> BTreeMap<(u32, u64), CpScanOwnership> {
    let mut latest = known.clone();
    for (event_i, event) in batch.cp_lifecycle_events.iter().enumerate() {
        match event {
            CpLifecycleEvent::Init(init) => {
                checks.push(format!(
                    "checkCpInit \"bar[{checkpoint}].cp_event[{event_i}].init\" {} {} {} {} {}",
                    element_id(init.b_center_id),
                    rust_center(&init.center),
                    opt_element_id(init.departure_move_id),
                    opt_expr(init.departure_interval.as_ref(), |value| interval(*value)),
                    cp_full_rust(&init.after)
                ));
                // Init 是重扫/重组后新对象的真实 writer；即使相同 B_p key 的旧对象曾
                // Closed，也必须先按事件序落 Pending。若身份稳定，紧随其后的 Reinherit
                // 会恢复旧证书；若依赖已脏而不继承，Pending 就是生产终态。
                latest.insert(cp_key(&init.after), init.after.clone());
            }
            CpLifecycleEvent::Dirty(dirty) => {
                assert_eq!(dirty.before.len(), dirty.after.len(), "cp dirty 必须同长");
                for (j, (before, after)) in dirty.before.iter().zip(&dirty.after).enumerate() {
                    checks.push(format!(
                        "checkCpDirty \"bar[{checkpoint}].cp_event[{event_i}].dirty[{j}]\" {} {} {}",
                        dirty.dirty_from,
                        cp_full_rust(before),
                        cp_full_rust(after)
                    ));
                    latest.insert(cp_key(after), after.clone());
                }
            }
            CpLifecycleEvent::Reinherit(reinherit) => {
                checks.push(format!(
                    "checkCpReinherit \"bar[{checkpoint}].cp_event[{event_i}].reinherit\" {} {} {} {}",
                    reinherit.dirty_from,
                    cp_full_rust(&reinherit.rebuilt_before),
                    opt_expr(reinherit.prior.as_ref(), cp_full_rust),
                    cp_full_rust(&reinherit.after)
                ));
                latest.insert(cp_key(&reinherit.after), reinherit.after.clone());
            }
            CpLifecycleEvent::Advance(advance) => {
                checks.push(format!(
                    "checkCpAdvance \"bar[{checkpoint}].cp_event[{event_i}].advance\" {} (some {}) {}",
                    cp_full_rust(&advance.before),
                    cp_advance_raw(advance),
                    cp_full_rust(&advance.after)
                ));
                latest.insert(cp_key(&advance.after), advance.after.clone());
            }
            CpLifecycleEvent::Review(review) => {
                checks.push(format!(
                    "checkCpReview \"bar[{checkpoint}].cp_event[{event_i}].review\" {} {} {} {}",
                    cp_full_rust(&review.before),
                    mirror_list(&review.centers, center),
                    mirror_list(&review.visible_moves, cp_move_raw),
                    cp_full_rust(&review.after)
                ));
                latest.insert(cp_key(&review.after), review.after.clone());
            }
        }
    }
    latest
}

fn cp_review_production_check() -> String {
    let review = s2_mirror_capture::run_cp_full_qualified_review_diagnostic();
    format!(
        "checkCpReview \"coverage.cp.review_production\" {} {} {} {}",
        cp_full_rust(&review.before),
        mirror_list(&review.centers, center),
        mirror_list(&review.visible_moves, cp_move_raw),
        cp_full_rust(&review.after)
    )
}

fn first_production_checks() -> Vec<String> {
    let captures = s2_mirror_capture::run_first_production_diagnostics();
    assert_eq!(captures.len(), 2, "O-01/O-03 production fixture 数量漂移");
    captures
        .iter()
        .zip(["present", "missing"])
        .flat_map(|(first, variant)| {
            let input = first_input_expr(first);
            let grade = grade_rust(first)
                .map(|grade| format!("(some {grade})"))
                .expect("production first fixture 必须产生 grade");
            [
                format!(
                    "checkFirst \"coverage.o01.first_production_{variant}\" {} \"{}\" {input} {}",
                    first.level,
                    side_name(first.side),
                    first_rust(first)
                ),
                format!(
                    "checkGrade \"coverage.o03.grade_production_{variant}\" {} \"{}\" {input} {grade}",
                    first.level,
                    side_name(first.side)
                ),
            ]
        })
        .collect()
}

fn append_scan_checks(
    checks: &mut Vec<String>,
    shared_series_defs: &mut BTreeMap<String, s2_mirror_capture::RawSeriesCapture>,
    counts: &mut PortCounts,
    enumerations: &mut PortCounts,
    batch: &CaptureBatch,
    checkpoint: usize,
) {
    let series_name = format!("rawSeries{checkpoint}");
    if !batch.frontiers.is_empty() {
        let series = batch
            .raw_series
            .as_ref()
            .unwrap_or_else(|| panic!("bar {checkpoint} frontier 缺 batch shared raw series"));
        if let Some(existing) = shared_series_defs.insert(series_name.clone(), series.clone()) {
            assert_eq!(existing, *series, "bar {checkpoint} raw series 定义不一致");
        }
    }
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
        let reduction_count = batch
            .reductions
            .iter()
            .filter(|reduction| reduction.level == scan.level)
            .count();
        checks.push(format!(
            "checkPortEnumeration \"bar[{checkpoint}].scan[{i}]\" \"reduction\" {} {reduction_count}",
            scan.level
        ));
        enumerations.reduction += 1;
        append_pan_checks(checks, counts, scan, checkpoint, i);
    }
    for (i, first) in batch.first_assemblies.iter().enumerate() {
        let input = first_input_expr(first);
        let path = format!("bar[{checkpoint}].first[{i}]");
        checks.push(format!(
            "checkFirst \"{path}.point\" {} \"{}\" {input} {}",
            first.level,
            side_name(first.side),
            first_rust(first)
        ));
        let grade =
            grade_rust(first).map_or_else(|| "none".to_owned(), |grade| format!("(some {grade})"));
        checks.push(format!(
            "checkGrade \"{path}.grade\" {} \"{}\" {input} {grade}",
            first.level,
            side_name(first.side)
        ));
        counts.points += usize::from(first.output.is_some());
        counts.grades += usize::from(first.grade_output.is_some());
    }
    for (i, third) in batch.third_assemblies.iter().enumerate() {
        checks.push(third_check(third, &format!("bar[{checkpoint}].third[{i}]")));
        counts.points += usize::from(third.output.is_some());
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
    }
    for (i, frontier) in batch.frontiers.iter().enumerate() {
        checks.push(frontier_check(
            frontier,
            &format!("bar[{checkpoint}].frontier[{i}]"),
            &series_name,
        ));
        counts.frontier += 1;
        enumerations.frontier += 1;
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

type MemoPointHistory = BTreeMap<(u32, (usize, usize, usize)), (Vec<BspPoint>, Vec<BspPoint>)>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoPortWitness {
    pipeline_points: Vec<BspPoint>,
    pan_divs: Vec<PanDivCert>,
    grades: Vec<newchan_rust::theta_v0::classifier::signal::FirstClassGradeRecord>,
    candidate_legs: Vec<CandidateObservation>,
}

type MemoPortHistory = BTreeMap<(u32, (usize, usize, usize)), MemoPortWitness>;

fn memo_port_witness(memo: &FourRcMemoCapture) -> MemoPortWitness {
    MemoPortWitness {
        pipeline_points: memo.pipeline_points.as_ref().clone(),
        pan_divs: memo.pan_divs.as_ref().clone(),
        grades: memo.grades.as_ref().clone(),
        candidate_legs: memo.candidates.as_ref().clone(),
    }
}

fn remember_memo_port_witness(
    history: &mut MemoPortHistory,
    memo: &FourRcMemoCapture,
    production_miss: bool,
) {
    let witness = memo_port_witness(memo);
    let key = (memo.level, memo.key);
    if production_miss {
        // 同 key 可在不同缓存驻留期合法演化；真实 miss 是新驻留期的唯一写点，
        // 因此覆盖为“最近一次 miss”，后续 hit 必须对拍这份见证。
        history.insert(key, witness);
    } else {
        history.entry(key).or_insert(witness);
    }
}

fn assert_hit_matches_latest_miss(memo: &FourRcMemoCapture, latest: &MemoPortWitness) {
    if memo.hit {
        assert_eq!(
            &memo_port_witness(memo),
            latest,
            "memo hit 四产口必须逐位等于同 level/key 最近一次 production miss"
        );
    }
}

fn remember_point_witness(
    history: &mut MemoPointHistory,
    level: u32,
    key: (usize, usize, usize),
    direct: &[BspPoint],
    second_07b: &[BspPoint],
    production_miss: bool,
) {
    let witness = (direct.to_vec(), second_07b.to_vec());
    if production_miss {
        history.insert((level, key), witness);
    } else {
        history.entry((level, key)).or_insert(witness);
    }
}

fn append_memo_checks(
    checks: &mut Vec<String>,
    counts: &mut PortCounts,
    enumerations: &mut PortCounts,
    batch: &CaptureBatch,
    raw_batch: Option<&CaptureBatch>,
    checkpoint: usize,
    point_history: &mut MemoPointHistory,
    port_history: &mut MemoPortHistory,
) {
    for (i, memo) in batch.four_rc_memos.iter().enumerate() {
        let base = format!("bar[{checkpoint}].memo[{i}]");
        // memo hit 自身不重跑 07a/07b；优先接入同一 checkpoint 的独立空缓存
        // diagnostic miss，再接生产 miss。两路若同键并存必须逐位相同。
        let raw_memo = raw_batch.and_then(|raw| {
            raw.four_rc_memos
                .iter()
                .find(|candidate| candidate.level == memo.level && candidate.key == memo.key)
        });
        if let Some(second) = &memo.second_07b_points {
            remember_point_witness(
                point_history,
                memo.level,
                memo.key,
                &memo.direct_3a_points,
                second,
                true,
            );
            remember_memo_port_witness(port_history, memo, true);
        }
        // fresh diagnostic miss 代表“当前重算”，不能覆盖生产 memo 命中实际复用的历史分区；
        // 仅在尚无生产 miss 见证时兜底（正常持续捕获路径不会走到）。
        if !point_history.contains_key(&(memo.level, memo.key)) {
            if let Some(source) = raw_memo {
                if let Some(second) = &source.second_07b_points {
                    remember_point_witness(
                        point_history,
                        source.level,
                        source.key,
                        &source.direct_3a_points,
                        second,
                        false,
                    );
                    remember_memo_port_witness(port_history, source, false);
                }
            }
        }
        let (direct_points, second_07b_points) = point_history
            .get(&(memo.level, memo.key))
            .unwrap_or_else(|| {
                panic!(
                    "bar {checkpoint} L{} memo key {:?} 缺独立 07a/07b miss 见证",
                    memo.level, memo.key
                )
            });
        let expected_ports = port_history
            .get(&(memo.level, memo.key))
            .unwrap_or_else(|| {
                panic!(
                    "bar {checkpoint} L{} memo key {:?} 缺四产口 miss 历史见证",
                    memo.level, memo.key
                )
            });
        assert_hit_matches_latest_miss(memo, expected_ports);
        checks.push(memo_check(memo, expected_ports, &base));
        checks.push(format!(
            "checkPipelinePointComposition \"{base}.pipeline_07b\" {} {} {} {}",
            memo.level,
            mirror_list(direct_points, bsp_rust),
            mirror_list(second_07b_points, bsp_rust),
            mirror_list(&memo.pipeline_points, bsp_rust),
        ));
        counts.memo += 1;
        counts.pipeline_07b += second_07b_points.len();
        enumerations.memo += 1;
        enumerations.pipeline_07b += 1;
        let scan = batch.scans.iter().find(|scan| scan.level == memo.level);
        if scan.is_none() {
            for port in ["prelude", "reduction", "frontier"] {
                checks.push(format!(
                    "checkPortEnumeration \"{base}.memo_hit_empty\" \"{port}\" {} 0",
                    memo.level
                ));
            }
            enumerations.prelude += 1;
            enumerations.reduction += 1;
            enumerations.frontier += 1;
        }
        for (port, count) in [
            ("points", direct_points.len()),
            ("points_07b", second_07b_points.len()),
            ("points_pipeline_with_07b", memo.pipeline_points.len()),
            ("pan", memo.pan_divs.len()),
            ("grades", memo.grades.len()),
            ("observations", memo.candidates.len()),
        ] {
            checks.push(format!(
                "checkPortEnumeration \"{base}\" \"{port}\" {} {count}",
                memo.level
            ));
        }
        counts.points += direct_points.len();
        counts.pan += memo.pan_divs.len();
        counts.grades += memo.grades.len();
        counts.observations += memo.candidates.len();
        enumerations.points += 1;
        enumerations.pan += 1;
        enumerations.grades += 1;
        enumerations.observations += 1;

        let raw_frontier = raw_batch
            .unwrap_or(batch)
            .frontiers
            .iter()
            .find(|frontier| frontier.level == memo.level);
        if raw_batch.is_some() && raw_frontier.is_none() {
            panic!(
                "bar {checkpoint} L{} checkpoint 缺独立 full raw P10 frontier",
                memo.level
            );
        }
        if let Some(raw_frontier) = raw_frontier {
            // P-10 检查只能使用同一次 raw miss 的四产口。checkpoint 的生产 memo
            // 可能复用历史 Rc，而 point_history 专用于证明该历史 key 的 07a+07b 组成；
            // 两类见证不得混合。
            let p10_memo = raw_memo.unwrap_or(memo);
            let series_name = format!("rawSeries{checkpoint}");
            checks.push(format!(
                "checkCheckpointOutput \"{base}.four_ports\" {series_name} {} {} {}",
                raw_resume_expr(raw_frontier),
                four_cache_expr(&s2_mirror_capture::FourCacheCapture::default()),
                merged_rust_lists(
                    &p10_memo.direct_3a_points,
                    &p10_memo.pan_divs,
                    &p10_memo.grades,
                    &p10_memo.candidates
                )
            ));
        }
    }
}

fn point_side(point: &BspPoint) -> &'static str {
    if point.bits.buy1 || point.bits.buy2 || point.bits.buy3 {
        "Long"
    } else if point.bits.sell1 || point.bits.sell2 || point.bits.sell3 {
        "Short"
    } else {
        point.struct_break_dir.map_or("NotYetKnown", side_name)
    }
}

fn finalize_layers(
    levels: &BTreeSet<u32>,
    counts: &BTreeMap<(u32, &'static str, &'static str), usize>,
) -> Vec<LayerCell> {
    let mut out = Vec::new();
    for &level in levels {
        for side in ["Long", "Short"] {
            for terminal in ["NoCpApplicable"] {
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

type LeanPortMetrics = (
    BTreeMap<String, usize>,
    BTreeMap<String, usize>,
    BTreeMap<String, usize>,
);

fn parse_lean_metrics(stdout: &str) -> (LeanPortMetrics, Vec<RuntimeLayerCell>) {
    let mut compared = BTreeMap::new();
    let mut failures = BTreeMap::new();
    let mut mismatches = BTreeMap::new();
    let mut layers: BTreeMap<(u32, String, String, String), (usize, usize)> = BTreeMap::new();
    for line in stdout.lines() {
        let cols: Vec<_> = line.split('\t').collect();
        match cols.first().copied() {
            Some("S2_LEAN_PORT_MISMATCH") if cols.len() == 6 => {
                let port = cols[2].to_owned();
                *compared.entry(port.clone()).or_insert(0) +=
                    cols[3].parse::<usize>().expect("port compared_fields 非法");
                *failures.entry(port.clone()).or_insert(0) +=
                    cols[4].parse::<usize>().expect("port failure_records 非法");
                *mismatches.entry(port).or_insert(0) +=
                    cols[5].parse::<usize>().expect("port mismatch_fields 非法");
            }
            Some("S2_LEAN_LAYER") if cols.len() == 8 => {
                let level = cols[2]
                    .strip_prefix('L')
                    .expect("layer level 缺 L")
                    .parse::<u32>()
                    .expect("layer level 非法");
                let key = (
                    level,
                    cols[3].to_owned(),
                    cols[4].to_owned(),
                    cols[5].to_owned(),
                );
                let entry = layers.entry(key).or_insert((0, 0));
                entry.0 += cols[6].parse::<usize>().expect("layer compared 非法");
                entry.1 += cols[7].parse::<usize>().expect("layer mismatch 非法");
            }
            _ => {}
        }
    }
    let layers = layers
        .into_iter()
        .map(
            |((level, side, terminal, port), (compared_fields, mismatch_fields))| {
                RuntimeLayerCell {
                    level,
                    side,
                    terminal,
                    port,
                    compared_fields,
                    mismatch_fields,
                }
            },
        )
        .collect();
    ((compared, failures, mismatches), layers)
}

fn parse_lean_port_records(stdout: &str) -> BTreeMap<String, usize> {
    let mut records = BTreeMap::new();
    for line in stdout.lines() {
        let cols: Vec<_> = line.split('\t').collect();
        if matches!(cols.first().copied(), Some("S2_LEAN_PORT")) && cols.len() == 4 {
            *records.entry(cols[2].to_owned()).or_insert(0) +=
                cols[3].parse::<usize>().expect("port record count 非法");
        }
    }
    records
}

fn parse_passed_fixture_checks(stdout: &str) -> BTreeSet<String> {
    stdout
        .lines()
        .filter_map(|line| {
            let cols = line.split('\t').collect::<Vec<_>>();
            (cols.len() == 7
                && cols[0] == "S2_LEAN_CHECK"
                && cols[3] == "coverage_fixture"
                && cols[4] == "1")
                .then(|| cols[2].to_owned())
        })
        .collect()
}

#[derive(Debug, Default, PartialEq, Eq)]
struct LeanCoveredPaths {
    real_window: BTreeSet<String>,
    production_fixture: BTreeSet<String>,
    wire_schema_smoke: BTreeSet<String>,
}

/// Runner 是覆盖归因的唯一真值源。每行携带产生逐叶比较的 check path；Rust 只分类、
/// 去重和汇总，绝不再按“某 list 非空”批量猜测整族字段已比较。
fn parse_lean_covered_paths(stdout: &str) -> LeanCoveredPaths {
    #[derive(Default)]
    struct CheckEvidence {
        count: usize,
        port: Option<String>,
        all_succeeded: bool,
    }

    fn production_expected_port(path: &str) -> Option<&'static str> {
        match path {
            "coverage.p10.cache_frontier_pan"
            | "coverage.p10.first_grade_candidate_confirmed"
            | "coverage.p10.first_grade_candidate_tail"
            | "coverage.p10.pan_tail"
            | "coverage.p10.third_tail" => Some("coverage_fixture"),
            "coverage.p10.second_production" => Some("second_production"),
            "coverage.cp.review_production" => Some("cp_review"),
            "coverage.o01.first_production_present" | "coverage.o01.first_production_missing" => {
                Some("points")
            }
            "coverage.o03.grade_production_present" | "coverage.o03.grade_production_missing" => {
                Some("grades")
            }
            _ => None,
        }
    }

    let mut checks = BTreeMap::<String, CheckEvidence>::new();
    for line in stdout.lines() {
        let cols = line.split('\t').collect::<Vec<_>>();
        if cols.len() != 7 || cols[0] != "S2_LEAN_CHECK" {
            continue;
        }
        let succeeded = cols[4] == "1" && cols[6] == "0";
        let evidence = checks.entry(cols[2].to_owned()).or_default();
        evidence.count += 1;
        if evidence.port.is_none() {
            evidence.port = Some(cols[3].to_owned());
            evidence.all_succeeded = succeeded;
        } else {
            evidence.all_succeeded &= succeeded;
        }
    }
    let mut covered = LeanCoveredPaths::default();
    for line in stdout.lines() {
        let cols = line.split('\t').collect::<Vec<_>>();
        if cols.len() != 3 || cols[0] != "S2_LEAN_COVERED" {
            continue;
        }
        let check_path = cols[1];
        let schema_path = cols[2].to_owned();
        if let Some(expected_port) = production_expected_port(check_path) {
            let valid = checks.get(check_path).is_some_and(|evidence| {
                evidence.count == 1
                    && evidence.port.as_deref() == Some(expected_port)
                    && evidence.all_succeeded
            });
            if valid {
                covered.production_fixture.insert(schema_path);
            }
            // allowlist 命中但证据无效时必须直接拒绝，不得降级成 wire-smoke credit。
            continue;
        }
        if !checks
            .get(check_path)
            .is_some_and(|evidence| evidence.count > 0 && evidence.all_succeeded)
        {
            continue;
        }
        if check_path.starts_with("coverage.") {
            covered.wire_schema_smoke.insert(schema_path);
        } else {
            covered.real_window.insert(schema_path);
        }
    }
    covered
}

#[derive(Debug)]
struct CompiledRawBase {
    module_name: String,
    source_path: PathBuf,
    olean_path: PathBuf,
    removed_stale_shards: usize,
}

fn raw_base_module_suffix(window_id: &str) -> Result<String, String> {
    let suffix = window_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    if suffix.is_empty() {
        Err("window id 必须含 ASCII 字母或数字".to_owned())
    } else {
        Ok(suffix)
    }
}

fn remove_stale_raw_shards(artifact_dir: &Path, window_id: &str) -> Result<usize, String> {
    let prefix = format!("S2MirrorRaw{}Shard", raw_base_module_suffix(window_id)?);
    let mut removed = 0;
    for entry in std::fs::read_dir(artifact_dir)
        .map_err(|error| format!("读取产物目录 {} 失败: {error}", artifact_dir.display()))?
    {
        let entry = entry.map_err(|error| format!("读取产物目录项失败: {error}"))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let normalized = name.strip_prefix('.').unwrap_or(name);
        if !normalized.starts_with(&prefix)
            || !(normalized.ends_with(".lean")
                || normalized.ends_with(".olean")
                || normalized.ends_with(".pending.olean"))
        {
            continue;
        }
        std::fs::remove_file(entry.path()).map_err(|error| {
            format!("清理旧 raw shard {} 失败: {error}", entry.path().display())
        })?;
        removed += 1;
    }
    Ok(removed)
}

fn remove_stale_fixture_chunks(artifact_dir: &Path, window_id: &str) -> Result<usize, String> {
    let prefix = format!("{}-", window_id.to_ascii_lowercase());
    let mut removed = 0;
    for entry in std::fs::read_dir(artifact_dir)
        .map_err(|error| format!("读取产物目录 {} 失败: {error}", artifact_dir.display()))?
    {
        let entry = entry.map_err(|error| format!("读取产物目录项失败: {error}"))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(index) = name
            .strip_prefix(&prefix)
            .and_then(|rest| rest.strip_suffix(".lean"))
        else {
            continue;
        };
        if index.len() != 4 || !index.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        std::fs::remove_file(entry.path()).map_err(|error| {
            format!(
                "清理旧 Lean fixture chunk {} 失败: {error}",
                entry.path().display()
            )
        })?;
        removed += 1;
    }
    Ok(removed)
}

fn compile_raw_base_module(
    formal_dir: &Path,
    artifact_dir: &Path,
    window_id: &str,
    plan: &RawSeriesWindowPlan,
) -> Result<CompiledRawBase, String> {
    let suffix = raw_base_module_suffix(window_id)?;
    let module_name = format!("S2MirrorRawBase{suffix}");
    let source_path = artifact_dir.join(format!("{module_name}.lean"));
    let olean_path = artifact_dir.join(format!("{module_name}.olean"));
    let staging_olean = artifact_dir.join(format!(".{module_name}.pending.olean"));
    let removed_stale_shards = remove_stale_raw_shards(artifact_dir, window_id)?;
    std::fs::write(&source_path, plan.base_module_source())
        .map_err(|error| format!("写 raw base {} 失败: {error}", source_path.display()))?;
    for stale in [&staging_olean, &olean_path] {
        if stale.exists() {
            std::fs::remove_file(stale)
                .map_err(|error| format!("清理旧 raw base {} 失败: {error}", stale.display()))?;
        }
    }
    let lean = Command::new("lake")
        .args(["env", "lean", "-R"])
        .arg(artifact_dir)
        .arg("-o")
        .arg(&staging_olean)
        .arg(&source_path)
        .env("LEAN_PATH", artifact_dir)
        .current_dir(formal_dir)
        .output()
        .map_err(|error| format!("启动 raw base Lean 编译失败: {error}"))?;
    if !lean.status.success() {
        let _ = std::fs::remove_file(&staging_olean);
        return Err(format!(
            "raw base {module_name} 编译失败\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&lean.stdout),
            String::from_utf8_lossy(&lean.stderr)
        ));
    }
    if !staging_olean.is_file() {
        return Err(format!(
            "raw base {module_name} 编译成功但未产 {}",
            staging_olean.display()
        ));
    }
    std::fs::rename(&staging_olean, &olean_path).map_err(|error| {
        format!(
            "发布 raw base {} -> {} 失败: {error}",
            staging_olean.display(),
            olean_path.display()
        )
    })?;
    let source_mtime = std::fs::metadata(&source_path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| format!("读取 raw base source mtime 失败: {error}"))?;
    let olean_mtime = std::fs::metadata(&olean_path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| format!("读取 raw base olean mtime 失败: {error}"))?;
    if olean_mtime < source_mtime {
        return Err(format!(
            "raw base olean 早于 source: {} < {}",
            olean_path.display(),
            source_path.display()
        ));
    }
    Ok(CompiledRawBase {
        module_name,
        source_path,
        olean_path,
        removed_stale_shards,
    })
}

fn referenced_raw_series_for_checks<'a>(
    plan: &'a RawSeriesWindowPlan,
    checks: &[String],
) -> Result<Vec<(String, &'a RawSeriesDefinitionPlan)>, String> {
    let mut names = BTreeSet::<String>::new();
    for check in checks {
        for identifier in check.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_')) {
            if !identifier.starts_with("rawSeries") {
                continue;
            }
            if !plan.definitions.contains_key(identifier) {
                return Err(format!(
                    "check 引用未知 raw definition {identifier}: {check}"
                ));
            }
            names.insert(identifier.to_owned());
        }
    }
    Ok(names
        .into_iter()
        .map(|name| {
            let definition = plan
                .definitions
                .get(&name)
                .expect("membership checked above");
            (name, definition)
        })
        .collect())
}

fn fixture_chunk_source(
    raw_plan: &RawSeriesWindowPlan,
    raw_base_module_name: &str,
    chunk_id: &str,
    checks: &[String],
    coverage_wire: Option<&str>,
) -> Result<String, String> {
    let raw_definitions = referenced_raw_series_for_checks(raw_plan, checks)?;
    let mut fixture = String::from("import Origin.UnifiedScanMirrorRunner\n");
    if !raw_definitions.is_empty() {
        writeln!(&mut fixture, "import {raw_base_module_name}").unwrap();
    }
    fixture.push_str(
        "\nset_option maxHeartbeats 0\nset_option maxRecDepth 1000000\n\nopen NewChanlun.Origin.UnifiedScanMirrorBridge\nopen NewChanlun.Origin.UnifiedScanMirrorRunner\n\n",
    );
    if !raw_definitions.is_empty() {
        for (name, definition) in raw_definitions {
            writeln!(
                &mut fixture,
                "def {name} : MergedScanRawSeries := {}\n",
                definition.lean_expr()
            )
            .unwrap();
        }
    }
    fixture.push_str("#eval emitReport \"");
    let (list_open, list_close) = coverage_wire.map_or_else(
        || ("[".to_owned(), "]"),
        |wire| (format!("(coverageFixtureChecks {wire} ++ ["), "])"),
    );
    write!(
        &mut fixture,
        "{chunk_id}\" {list_open}\n  {}\n{list_close}\n",
        checks.join(",\n  ")
    )
    .unwrap();
    Ok(fixture)
}

fn fixture_cache_key(context_hash: u64, fixture: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    context_hash.hash(&mut hasher);
    fixture.hash(&mut hasher);
    hasher.finish()
}

fn parse_successful_chunk(stdout: String) -> Result<(String, usize, usize), String> {
    let Some(summary) = stdout
        .lines()
        .find(|line| line.starts_with("S2_LEAN_SUMMARY\t"))
    else {
        return Err(format!("缺 Lean summary: {stdout}"));
    };
    let cols: Vec<_> = summary.split('\t').collect();
    match (cols[3].parse::<usize>(), cols[4].parse::<usize>()) {
        (Ok(compared), Ok(mismatch)) => Ok((stdout, compared, mismatch)),
        _ => Err(format!("summary 数值非法: {summary}")),
    }
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
    let mut checks = vec![cp_review_production_check()];
    checks.extend(first_production_checks());
    let mut shared_series_defs = BTreeMap::<String, s2_mirror_capture::RawSeriesCapture>::new();
    let mut ports = PortCounts::default();
    let mut enumerations = PortCounts::default();
    let mut checkpoints = 0;
    let mut levels = BTreeSet::new();
    let mut candidate_counts = BTreeMap::new();
    let mut cp_counts = BTreeMap::new();
    let mut captured_real_miss = false;
    let mut option_stats = BTreeMap::from([
        ("O01.bits.third_class_entry", (0, 0)),
        ("O01.owner", (0, 0)),
        ("O01.force", (0, 0)),
        ("O01.retrace_breaks_type1", (0, 0)),
        ("O03.grade.present", (0, 0)),
        ("O03.grade.missing", (0, 0)),
        ("O04.center_ids", (0, 0)),
        ("O04.third_class_proof", (0, 0)),
        ("O04.first_provable_at", (0, 0)),
        ("O04.confirmed_at", (0, 0)),
        ("cp.departure_move_id", (0, 0)),
        ("cp.departure_interval", (0, 0)),
        ("cp.cp_certificate_confirm_src", (0, 0)),
        ("cp.c_structure", (0, 0)),
        ("cp.third_class_in_c", (0, 0)),
        ("cp.full_trend_evidence", (0, 0)),
        ("cp.full_trend_c_qualified", (0, 0)),
    ]);
    let mut cp_history = BTreeMap::<(u32, u64), CpScanOwnership>::new();
    let mut memo_point_history = MemoPointHistory::new();
    let mut memo_port_history = MemoPortHistory::new();

    for (local, bar) in bars[spec.start..spec.end].iter().copied().enumerate() {
        let checkpoint = (local + 1) % CHECKPOINT_EVERY == 0 || local + 1 == spec.end - spec.start;
        // 只在 checkpoint/末根打开捕获槽。禁止为寻找 P-10 miss 从 bucket 首根起逐 bar
        // 打开，否则 memo hit 也会深拷贝四件 Vec，改变验收执行面的复杂度。
        if checkpoint {
            s2_mirror_capture::begin_capture();
        } else if !captured_real_miss {
            s2_mirror_capture::begin_miss_probe();
        } else {
            s2_mirror_capture::begin_cp_probe();
        }
        let l0 = parser.append(bar);
        let output = classifier::classify_incremental(&l0, config, &mut cache, &[]);
        let batch = s2_mirror_capture::take_capture();
        let raw_batch = if checkpoint {
            // Checkpoint memo hit 只证明四 Rc 身份；内容 expected 必须来自独立空缓存全 raw 扫描。
            // 诊断重放不接回生产 cache/output，捕获 seam 仍只读且默认关闭。
            s2_mirror_capture::begin_capture();
            let mut diagnostic_cache = TowerCache::new();
            let _diagnostic =
                classifier::classify_incremental(&l0, config, &mut diagnostic_cache, &[]);
            let raw = s2_mirror_capture::take_capture();
            raw
        } else {
            CaptureBatch::default()
        };
        cp_history =
            append_cp_transition_checks(&mut checks, &batch, bar.source_index, &cp_history);
        observe_batch_options(
            if checkpoint { &raw_batch } else { &batch },
            &mut option_stats,
        );
        if !checkpoint && (!batch.frontiers.is_empty() || !batch.four_rc_memos.is_empty()) {
            append_scan_checks(
                &mut checks,
                &mut shared_series_defs,
                &mut ports,
                &mut enumerations,
                &batch,
                bar.source_index,
            );
            append_memo_checks(
                &mut checks,
                &mut ports,
                &mut enumerations,
                &batch,
                None,
                bar.source_index,
                &mut memo_point_history,
                &mut memo_port_history,
            );
            captured_real_miss = true;
        }
        if !checkpoint {
            continue;
        }
        checkpoints += 1;
        append_scan_checks(
            &mut checks,
            &mut shared_series_defs,
            &mut ports,
            &mut enumerations,
            &raw_batch,
            bar.source_index,
        );
        assert_eq!(
            batch.four_rc_memos.len(),
            output.classification.levels.len(),
            "checkpoint 必须逐 level 捕获四 Rc memo"
        );
        append_memo_checks(
            &mut checks,
            &mut ports,
            &mut enumerations,
            &batch,
            Some(&raw_batch),
            bar.source_index,
            &mut memo_point_history,
            &mut memo_port_history,
        );
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
                observe_cp_options(cp, &mut option_stats);
                let expected = cp_history.get(&cp_key(cp)).unwrap_or_else(|| {
                    panic!(
                        "bar {} L{level} cp[{i}] 缺独立 lifecycle transition 见证",
                        bar.source_index
                    )
                });
                checks.push(cp_check(
                    cp,
                    expected,
                    level,
                    &format!("bar[{}].level[{level}].cp[{i}]", bar.source_index),
                ));
                ports.cp += 1;
                let (side, terminal) = match cp.lifecycle {
                    CpLifecycleStatus::Pending => ("NotYetKnown", "Pending"),
                    CpLifecycleStatus::Closed => (
                        side_name(
                            cp.third_class_in_c
                                .as_ref()
                                .expect("Closed cp 必须携 third_class_in_c")
                                .side,
                        ),
                        "Closed",
                    ),
                };
                *cp_counts.entry((level, side, terminal)).or_insert(0) += 1;
            }
        }
        for memo in &batch.four_rc_memos {
            levels.insert(memo.level);
            let direct_points = &memo_point_history
                .get(&(memo.level, memo.key))
                .unwrap_or_else(|| {
                    panic!(
                        "bar {} L{} memo key {:?} 缺 07a 历史",
                        bar.source_index, memo.level, memo.key
                    )
                })
                .0;
            for point in direct_points {
                let side = point_side(point);
                if side != "NotYetKnown" {
                    *candidate_counts
                        .entry((memo.level, side, "NoCpApplicable"))
                        .or_insert(0) += 1;
                }
            }
            for cert in memo.pan_divs.iter() {
                *candidate_counts
                    .entry((memo.level, side_name(cert.side), "NoCpApplicable"))
                    .or_insert(0) += 1;
            }
            for grade in memo.grades.iter() {
                *candidate_counts
                    .entry((memo.level, side_name(grade.side), "NoCpApplicable"))
                    .or_insert(0) += 1;
            }
            for candidate in memo.candidates.iter() {
                *candidate_counts
                    .entry((memo.level, side_name(candidate.key.side), "NoCpApplicable"))
                    .or_insert(0) += 1;
            }
        }
    }

    // A Lean process imports the window-sized raw base for every chunk.  Phase 3
    // full-certificate measurements found 1,000 checks non-linear in both memory
    // (about 10 GiB/process in the W100 dense region) and recursion time.  250
    // keeps the complete check stream while bounding each Lean process; chunking
    // changes scheduling only, not checks or mismatch accounting.
    const CHECKS_PER_FIXTURE: usize = 250;
    let fixture_name = format!("{}-*.lean", spec.id.to_ascii_lowercase());
    let manifest_path = artifact_dir.join(format!("{}.lean", spec.id.to_ascii_lowercase()));
    let chunk_count = checks.len().div_ceil(CHECKS_PER_FIXTURE);
    let removed_stale_fixture_chunks = remove_stale_fixture_chunks(artifact_dir, spec.id)
        .unwrap_or_else(|error| panic!("{} 旧 fixture chunk 清理失败\n{error}", spec.id));
    let coverage_wire = coverage_fixture_wire();
    let raw_plan = RawSeriesWindowPlan::new(&shared_series_defs)
        .unwrap_or_else(|error| panic!("{} raw series 压缩规划失败\n{error}", spec.id));
    let raw_base = compile_raw_base_module(formal_dir, artifact_dir, spec.id, &raw_plan)
        .unwrap_or_else(|error| panic!("{} raw base 编译失败\n{error}", spec.id));
    let mut cache_context_hasher = std::collections::hash_map::DefaultHasher::new();
    for source in [
        formal_dir.join("Origin/UnifiedScanMirrorBridge.lean"),
        formal_dir.join("Origin/UnifiedScanMirrorRunner.lean"),
        raw_base.source_path.clone(),
    ] {
        std::fs::read(&source)
            .unwrap_or_else(|error| panic!("读取缓存上下文 {} 失败: {error}", source.display()))
            .hash(&mut cache_context_hasher);
    }
    let cache_context_hash = cache_context_hasher.finish();
    let manifest = format!(
        concat!(
            "-- generated manifest: {} chunks matching {}\n",
            "-- raw definitions are exact-reference inline; one precompiled window base\n",
            "-- window raw base capture: {}\n",
            "-- window raw base module: {}\n",
            "-- window raw base source: {}\n",
            "-- window raw base olean: {}\n",
            "-- stale fixture chunks removed: {}\n",
            "-- stale raw shard artifacts removed: {}\n"
        ),
        chunk_count,
        fixture_name,
        raw_plan.base_definition_name,
        raw_base.module_name,
        raw_base.source_path.display(),
        raw_base.olean_path.display(),
        removed_stale_fixture_chunks,
        raw_base.removed_stale_shards,
    );
    std::fs::write(&manifest_path, manifest).expect("写 Lean fixture manifest 失败");
    let mut fixture_jobs = Vec::with_capacity(chunk_count);
    for (chunk_index, chunk) in checks.chunks(CHECKS_PER_FIXTURE).enumerate() {
        let chunk_id = format!("{}-{:04}", spec.id, chunk_index);
        let fixture_path = artifact_dir.join(format!(
            "{}-{:04}.lean",
            spec.id.to_ascii_lowercase(),
            chunk_index
        ));
        let fixture = fixture_chunk_source(
            &raw_plan,
            &raw_base.module_name,
            &chunk_id,
            chunk,
            (chunk_index == 0).then_some(coverage_wire.as_str()),
        )
        .unwrap_or_else(|error| panic!("{chunk_id} 内联 raw definitions 失败\n{error}"));
        let cache_key = fixture_cache_key(cache_context_hash, &fixture);
        std::fs::write(&fixture_path, fixture).expect("写 Lean fixture chunk 失败");
        let cache_path = artifact_dir.join(format!(
            "{}-{:04}.stdout.cache",
            spec.id.to_ascii_lowercase(),
            chunk_index
        ));
        fixture_jobs.push((
            fixture_path
                .canonicalize()
                .expect("fixture canonicalize 失败"),
            cache_path,
            cache_key,
        ));
    }
    // 28-core release host: 8 workers still left the W100 heavy-tail batches
    // CPU-underfilled.  Sixteen changes scheduling only; fixture bytes and all
    // mismatch accounting stay identical.  The env override is diagnostic and
    // bounded so a rerun can lower pressure without editing semantics.
    let lean_workers = std::env::var("S2_LEAN_WORKERS")
        .ok()
        .map(|raw| raw.parse::<usize>().expect("S2_LEAN_WORKERS 必须是正整数"))
        .unwrap_or(16);
    assert!(
        (1..=32).contains(&lean_workers),
        "S2_LEAN_WORKERS 必须在 1..=32"
    );
    type ChunkResult = Result<(String, usize, usize), String>;
    let chunk_results =
        std::sync::Mutex::new(Vec::<(usize, ChunkResult)>::with_capacity(chunk_count));
    let completed_chunks = std::sync::atomic::AtomicUsize::new(0);
    let next_chunk = std::sync::atomic::AtomicUsize::new(0);
    let stop_after_failure = std::sync::atomic::AtomicBool::new(false);
    std::thread::scope(|scope| {
        for _worker in 0..lean_workers {
            let chunk_results = &chunk_results;
            let completed_chunks = &completed_chunks;
            let next_chunk = &next_chunk;
            let stop_after_failure = &stop_after_failure;
            let fixture_jobs = &fixture_jobs;
            scope.spawn(move || loop {
                if stop_after_failure.load(std::sync::atomic::Ordering::Acquire) {
                    break;
                }
                let chunk_index = next_chunk.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if chunk_index >= fixture_jobs.len() {
                    break;
                }
                let (fixture_path, cache_path, cache_key) = &fixture_jobs[chunk_index];
                let cache_header = format!("S2_LEAN_CACHE\t{cache_key:016x}\n");
                let cached = std::fs::read_to_string(cache_path).ok().and_then(|wire| {
                    wire.strip_prefix(&cache_header)
                        .map(ToOwned::to_owned)
                        .and_then(|stdout| parse_successful_chunk(stdout).ok())
                        .filter(|(_, _, mismatch)| *mismatch == 0)
                });
                let result = if let Some(cached) = cached {
                    Ok(cached)
                } else {
                    match Command::new("lake")
                        .args(["env", "lean"])
                        .arg(fixture_path)
                        .env("LEAN_PATH", artifact_dir)
                        .current_dir(formal_dir)
                        .output()
                    {
                        Err(error) => Err(format!("启动 Lean runner 失败: {error}")),
                        Ok(lean) => {
                            let stdout = String::from_utf8_lossy(&lean.stdout).into_owned();
                            let stderr = String::from_utf8_lossy(&lean.stderr).into_owned();
                            if !lean.status.success() {
                                Err(format!("stdout:\n{stdout}\nstderr:\n{stderr}"))
                            } else {
                                let parsed = parse_successful_chunk(stdout);
                                if let Ok((stdout, _, 0)) = &parsed {
                                    let mut wire = cache_header.clone();
                                    wire.push_str(stdout);
                                    std::fs::write(cache_path, wire).unwrap_or_else(|error| {
                                        panic!(
                                            "写 Lean 成功块缓存 {} 失败: {error}",
                                            cache_path.display()
                                        )
                                    });
                                }
                                parsed
                            }
                        }
                    }
                };
                let failed = match &result {
                    Err(_) => true,
                    Ok((_, _, mismatch)) => *mismatch != 0,
                };
                chunk_results
                    .lock()
                    .expect("chunk results mutex poisoned")
                    .push((chunk_index, result));
                if failed {
                    stop_after_failure.store(true, std::sync::atomic::Ordering::Release);
                }
                let completed =
                    completed_chunks.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                if completed == fixture_jobs.len() || completed % 10 == 0 {
                    eprintln!(
                        "S2_LEAN_PROGRESS\t{}\t{}/{}",
                        spec.id,
                        completed,
                        fixture_jobs.len()
                    );
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
        if mismatch != 0 {
            let evidence = stdout
                .lines()
                .filter(|line| {
                    line.starts_with("S2_LEAN_SUMMARY\t")
                        || line.starts_with("S2_LEAN_MISMATCH\t")
                        || line.starts_with("S2_LEAN_FIELD_MISMATCH\t")
                        || line.starts_with("S2_LEAN_FIELD_DETAIL\t")
                })
                .collect::<Vec<_>>()
                .join("\n");
            panic!(
                "{} Lean 对拍 mismatch（chunk {}，fields={}）\n{}",
                spec.id, chunk_index, mismatch, evidence
            );
        }
        lean_stdout.push_str(&stdout);
        compared_fields += compared;
        mismatch_fields += mismatch;
    }
    let ((compared_by_port, failure_records_by_port, mismatch_by_port), runtime_layers) =
        parse_lean_metrics(&lean_stdout);
    let port_records = parse_lean_port_records(&lean_stdout);
    let passed_fixture_checks = parse_passed_fixture_checks(&lean_stdout);
    let lean_covered = parse_lean_covered_paths(&lean_stdout);
    let lean_stdout_bytes = lean_stdout.len();
    let lean_stdout_digest_fnv1a64 = fnv1a64_hex(lean_stdout.as_bytes());
    let mut candidate_layers = finalize_layers(&levels, &candidate_counts);
    for cell in &mut candidate_layers {
        cell.mismatch_count = runtime_layers
            .iter()
            .filter(|runtime| {
                runtime.level == cell.level
                    && runtime.side == cell.side
                    && runtime.terminal == cell.terminal
            })
            .map(|runtime| runtime.mismatch_fields)
            .sum();
    }
    let empty_candidate_layers = candidate_layers
        .iter()
        .filter(|cell| cell.extraction_count == 0)
        .count();
    let mut cp_layers = Vec::new();
    for &level in &levels {
        for side in ["NotYetKnown", "Long", "Short"] {
            for terminal in ["Pending", "Closed"] {
                cp_layers.push(LayerCell {
                    level,
                    side,
                    terminal,
                    extraction_count: *cp_counts.get(&(level, side, terminal)).unwrap_or(&0),
                    mismatch_count: 0,
                });
            }
        }
    }
    for cell in &mut cp_layers {
        cell.mismatch_count = runtime_layers
            .iter()
            .filter(|runtime| {
                runtime.level == cell.level
                    && runtime.side == cell.side
                    && runtime.terminal == cell.terminal
                    && runtime.port == "cp"
            })
            .map(|runtime| runtime.mismatch_fields)
            .sum();
    }
    let event_count = ports.extraction_events() + enumerations.extraction_events();
    let max_level = levels.iter().next_back().copied();
    let missing_levels = max_level.map_or_else(Vec::new, |max| {
        (0..=max).filter(|level| !levels.contains(level)).collect()
    });
    let match_rate = if compared_fields == 0 {
        0.0
    } else {
        (compared_fields - mismatch_fields) as f64 / compared_fields as f64
    };
    let option_payload_coverage = option_stats
        .iter()
        .map(
            |(&path, &(some_count, empty_count))| OptionPayloadCoverage {
                path,
                some_count,
                empty_count,
                exercised: some_count > 0,
            },
        )
        .collect();
    let required = required_field_paths();
    let declared = declared_field_paths();
    let real_paths = lean_covered.real_window;
    let fixture_records = port_records.get("coverage_fixture").copied().unwrap_or(0);
    let fixture_compared = compared_by_port
        .get("coverage_fixture")
        .copied()
        .unwrap_or(0);
    let fixture_mismatch = mismatch_by_port
        .get("coverage_fixture")
        .copied()
        .unwrap_or(0);
    let production_fixture_paths = lean_covered.production_fixture;
    let fixture_paths = lean_covered.wire_schema_smoke;
    let semantic_paths = real_paths
        .union(&production_fixture_paths)
        .cloned()
        .collect::<BTreeSet<_>>();
    let combined_paths = semantic_paths;
    let real_window_coverage = coverage_report(real_paths, &required);
    let production_semantic_fixture_coverage = coverage_report(production_fixture_paths, &required);
    let wire_schema_smoke_coverage = coverage_report(fixture_paths.clone(), &declared);
    let fixture_coverage = coverage_report(fixture_paths, &declared);
    let combined_coverage = coverage_report(combined_paths, &required);
    let variant_coverage = fixture_variant_coverage(&passed_fixture_checks);
    let field_coverage_numerator = combined_coverage.numerator;
    let field_coverage_denominator = combined_coverage.denominator;
    let field_coverage_rate = if field_coverage_denominator == 0 {
        1.0
    } else {
        field_coverage_numerator as f64 / field_coverage_denominator as f64
    };
    let field_coverage_paths = combined_coverage.covered_paths.clone();
    let field_uncovered_paths = combined_coverage.uncovered_paths.clone();
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
        compared_by_port,
        failure_records_by_port,
        port_object_counts: ports.clone(),
        port_enumerations: enumerations.clone(),
        candidate_layers,
        cp_layers,
        runtime_layers,
        empty_candidate_layers,
        minimum_events_required: MIN_EVENTS,
        minimum_events_met: event_count >= MIN_EVENTS,
        every_port_emitted: enumerations.all_emit(),
        zero_mismatch_upper_95: zero_failure_upper_95(compared_fields),
        match_rate,
        field_coverage_numerator,
        field_coverage_denominator,
        field_coverage_rate,
        field_coverage_paths,
        field_uncovered_paths,
        real_window_coverage,
        production_semantic_fixture_coverage,
        wire_schema_smoke_coverage,
        fixture_coverage,
        combined_coverage,
        variant_coverage,
        fixture_evidence: FixtureEvidence {
            source: "wire_schema_smoke",
            records: fixture_records,
            compared_fields: fixture_compared,
            mismatch_fields: fixture_mismatch,
            passed_checks: passed_fixture_checks.into_iter().collect(),
        },
        option_payload_coverage,
        max_level,
        missing_levels,
        lean_fixture: fixture_name,
        lean_stdout_bytes,
        lean_stdout_digest_fnv1a64,
        aggregate_complete: false,
        aggregate_status: "not_evaluated",
        elapsed_seconds: started.elapsed().as_secs_f64(),
    }
}

fn verify_saved_aggregate(artifact_dir: &Path) -> AggregateReport {
    let mut saved = Vec::with_capacity(WINDOWS.len());
    for spec in WINDOWS {
        let path = artifact_dir.join(format!("{}.json", spec.id.to_ascii_lowercase()));
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("读取单窗报告 {} 失败: {error}", path.display()));
        let report: SavedWindowReport = serde_json::from_slice(&bytes)
            .unwrap_or_else(|error| panic!("解析单窗报告 {} 失败: {error}", path.display()));
        assert_eq!(
            report.schema_version,
            SCHEMA_VERSION,
            "{} schema version 不匹配",
            path.display()
        );
        assert_eq!(
            report.window_id,
            spec.id,
            "{} window id 不匹配",
            path.display()
        );
        assert_eq!(
            (report.start, report.end, report.heavy_overlap),
            (spec.start, spec.end, spec.heavy_overlap),
            "{} 固定窗口边界不匹配",
            path.display()
        );
        assert_eq!(report.mismatch_fields, 0, "{} mismatch 非零", spec.id);
        assert!(report.minimum_events_met, "{} 提取记录未达门", spec.id);
        assert!(report.every_port_emitted, "{} 存在未枚举产口", spec.id);
        assert_eq!(
            report.combined_coverage.denominator,
            required_field_paths().len(),
            "{} semantic denominator 不匹配",
            spec.id
        );
        saved.push(report);
    }
    let semantic_required = required_field_paths();
    let semantic_coverage = aggregate_coverage_reports(
        saved.iter().map(|report| &report.combined_coverage),
        &semantic_required,
    );
    assert_eq!(
        semantic_coverage.numerator, semantic_coverage.denominator,
        "四份单窗 JSON aggregate semantic coverage 未满: {:?}",
        semantic_coverage.uncovered_paths
    );
    AggregateReport {
        schema_version: SCHEMA_VERSION,
        aggregate_complete: true,
        aggregate_status: "complete",
        source_window_ids: WINDOWS.iter().map(|window| window.id).collect(),
        zero_mismatch: true,
        minimum_events_met: true,
        every_port_emitted: true,
        typed_schema_denominator: declared_field_paths().len(),
        semantic_coverage,
    }
}

fn write_aggregate_report(artifact_dir: &Path, report: &AggregateReport) {
    let path = artifact_dir.join("aggregate.json");
    std::fs::write(&path, serde_json::to_string_pretty(report).unwrap())
        .unwrap_or_else(|error| panic!("写 aggregate 报告 {} 失败: {error}", path.display()));
}

fn main() {
    assert!(
        standard_windows_are_valid(),
        "四窗定义或前三轻窗不重叠约束失效"
    );
    let args: Vec<_> = std::env::args_os().collect();
    if std::env::var("S2_AGGREGATE_ONLY").as_deref() == Ok("1") {
        assert!(
            args.len() == 2,
            "用法: S2_AGGREGATE_ONLY=1 s2_lean_mirror_extract <artifact-dir>"
        );
        let artifact_dir = PathBuf::from(&args[1]);
        let aggregate = verify_saved_aggregate(&artifact_dir);
        write_aggregate_report(&artifact_dir, &aggregate);
        eprintln!(
            "S2_AGGREGATE_SEMANTIC_COVERAGE scope=saved-four-windows aggregate_complete=true windows={} {}/{} typed_schema_denominator={}",
            aggregate.source_window_ids.len(),
            aggregate.semantic_coverage.numerator,
            aggregate.semantic_coverage.denominator,
            aggregate.typed_schema_denominator
        );
        return;
    }
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
    if let Some(selected) = selected.as_deref() {
        assert!(
            WINDOWS.iter().any(|window| window.id == selected),
            "S2_WINDOW={selected} 不是固定四窗之一"
        );
    }
    let mut semantic_window_reports = Vec::new();
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
        eprintln!(
            "S2_WINDOW_SEMANTIC_COVERAGE {} {}/{}（逐窗诚实报告，不冒充四窗总签）",
            spec.id, report.combined_coverage.numerator, report.combined_coverage.denominator
        );
        semantic_window_reports.push(report.combined_coverage.clone());
    }

    let semantic_required = required_field_paths();
    let semantic_aggregate =
        aggregate_coverage_reports(semantic_window_reports.iter(), &semantic_required);
    let signing_scope = if selected.is_none() {
        "all-four-windows"
    } else {
        "partial-single-window-no-sign"
    };
    eprintln!(
        concat!(
            "S2_AGGREGATE_SEMANTIC_COVERAGE scope={} aggregate_complete={} windows={} {}/{} ",
            "typed_schema_denominator={}"
        ),
        signing_scope,
        selected.is_none(),
        semantic_window_reports.len(),
        semantic_aggregate.numerator,
        semantic_aggregate.denominator,
        declared_field_paths().len()
    );
    if selected.is_none() {
        assert_eq!(
            semantic_window_reports.len(),
            WINDOWS.len(),
            "总签 coverage 必须来自完整四窗"
        );
        assert_eq!(
            semantic_aggregate.numerator, semantic_aggregate.denominator,
            "四窗 aggregate semantic coverage 未满: {:?}",
            semantic_aggregate.uncovered_paths
        );
        let aggregate = AggregateReport {
            schema_version: SCHEMA_VERSION,
            aggregate_complete: true,
            aggregate_status: "complete",
            source_window_ids: WINDOWS.iter().map(|window| window.id).collect(),
            zero_mismatch: true,
            minimum_events_met: true,
            every_port_emitted: true,
            typed_schema_denominator: declared_field_paths().len(),
            semantic_coverage: semantic_aggregate,
        };
        write_aggregate_report(&artifact_dir, &aggregate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_stroke_fixture(end_index: usize) -> Stroke {
        Stroke {
            direction: Direction::Up,
            start_index: end_index.saturating_sub(1),
            end_index,
            start_price: end_index as i64,
            end_price: end_index as i64 + 1,
        }
    }

    fn raw_series_fixture(
        len: usize,
        gauge: DivergenceGauge,
    ) -> s2_mirror_capture::RawSeriesCapture {
        s2_mirror_capture::RawSeriesCapture {
            hist_bits: (0..len as u64).collect(),
            dif_bits: (10..10 + len as u64).collect(),
            closes_ticks: (20..20 + len as i64).collect(),
            close_src: (30..30 + len).collect(),
            gauge,
            strokes: (0..len).map(raw_stroke_fixture).collect(),
        }
    }

    #[test]
    fn approved_windows_keep_three_light_ranges_disjoint() {
        assert!(standard_windows_are_valid());
    }

    #[test]
    fn zero_failure_upper_bound_is_monotone() {
        assert!(zero_failure_upper_95(10_000) < zero_failure_upper_95(100));
        assert_eq!(zero_failure_upper_95(0), 1.0);
    }

    #[test]
    fn stdout_digest_is_deterministic_fnv1a64() {
        assert_eq!(fnv1a64_hex(b"hello"), "a430d84680aabd0b");
        assert_ne!(fnv1a64_hex(b"hello"), fnv1a64_hex(b"hello\n"));
    }

    #[test]
    fn saved_four_window_aggregate_requires_matching_v3_reports() {
        let dir = std::env::temp_dir().join(format!(
            "s2-saved-aggregate-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let required = required_field_paths();
        for spec in WINDOWS {
            let value = serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "window_id": spec.id,
                "start": spec.start,
                "end": spec.end,
                "heavy_overlap": spec.heavy_overlap,
                "mismatch_fields": 0,
                "minimum_events_met": true,
                "every_port_emitted": true,
                "combined_coverage": {
                    "numerator": required.len(),
                    "denominator": required.len(),
                    "rate": 1.0,
                    "covered_paths": required,
                    "uncovered_paths": []
                }
            });
            std::fs::write(
                dir.join(format!("{}.json", spec.id.to_ascii_lowercase())),
                serde_json::to_vec(&value).unwrap(),
            )
            .unwrap();
        }
        let aggregate = verify_saved_aggregate(&dir);
        assert!(aggregate.aggregate_complete);
        assert_eq!(
            aggregate.semantic_coverage.numerator,
            aggregate.semantic_coverage.denominator
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn lean_metrics_parser_aggregates_ports_and_layers_across_chunks() {
        let stdout = concat!(
            "S2_LEAN_PORT_MISMATCH\tW20\tpoints\t11\t0\t0\n",
            "S2_LEAN_PORT_MISMATCH\tW20\tpoints\t13\t1\t2\n",
            "S2_LEAN_PORT_MISMATCH\tW20\tcp\t17\t0\t0\n",
            "S2_LEAN_LAYER\tW20\tL2\tLong\tClosed\tcp\t7\t0\n",
            "S2_LEAN_LAYER\tW20\tL2\tLong\tClosed\tcp\t5\t1\n",
        );

        let ((compared, failures, mismatches), layers) = parse_lean_metrics(stdout);
        assert_eq!(compared.get("points"), Some(&24));
        assert_eq!(failures.get("points"), Some(&1));
        assert_eq!(mismatches.get("points"), Some(&2));
        assert_eq!(compared.get("cp"), Some(&17));
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].level, 2);
        assert_eq!(layers[0].side, "Long");
        assert_eq!(layers[0].terminal, "Closed");
        assert_eq!(layers[0].port, "cp");
        assert_eq!(layers[0].compared_fields, 12);
        assert_eq!(layers[0].mismatch_fields, 1);
    }

    fn all_fixture_checks() -> BTreeSet<String> {
        [
            "coverage.o01.first_some",
            "coverage.o01.force_raw",
            "coverage.o01.first_none",
            "coverage.o01.owner_type1_anchor",
            "coverage.o01.third_success",
            "coverage.o01.third_failure",
            "coverage.o03.present",
            "coverage.o03.missing_leave",
            "coverage.o03.missing_retest",
            "coverage.o03.same_direction",
            "coverage.o03.leave_not_outside",
            "coverage.o03.retest_reentered",
            "coverage.o04.extreme_true.third_some",
            "coverage.o04.extreme_false.third_none",
            "coverage.cp.full_qualified",
            "coverage.p10.cache_frontier_pan",
            "coverage.p10.first_grade_candidate_confirmed",
            "coverage.p10.first_grade_candidate_tail",
            "coverage.p10.pan_tail",
            "coverage.p10.third_tail",
            "coverage.p10.checkpoint_populated",
            "coverage.p10.pipeline_nonempty",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }

    #[test]
    fn fixture_evidence_parser_only_accepts_individual_passed_checks() {
        let stdout = concat!(
            "S2_LEAN_CHECK\tW20\tcoverage.o01.first_some\tcoverage_fixture\t1\t31\t0\n",
            "S2_LEAN_CHECK\tW20\tcoverage.o01.first_none\tcoverage_fixture\t0\t1\t1\n",
            "S2_LEAN_CHECK\tW20\tbar[9].point\tpoints\t1\t31\t0\n",
        );
        assert_eq!(
            parse_passed_fixture_checks(stdout),
            BTreeSet::from(["coverage.o01.first_some".to_owned()])
        );
    }

    #[test]
    fn lean_covered_requires_successful_matching_check_then_applies_evidence_class() {
        let stdout = concat!(
            "S2_LEAN_COVERED\tbar[9].point\tO01.source_index\n",
            "S2_LEAN_COVERED\tcoverage.p10.second_production\tO01.owner.type1_anchor\n",
            "S2_LEAN_COVERED\tcoverage.o01.owner_type1_anchor\tO01.owner.type1_anchor\n",
            "S2_LEAN_COVERED\tcoverage.p10.pipeline_nonempty\tP10.memo.pipeline_07b_composition\n",
            "S2_LEAN_COVERED\tmissing.check\tO01.force.tag\n",
            "S2_LEAN_COVERED\tfailed.check\tO01.struct_break_dir.value\n",
            "S2_LEAN_CHECK\tW20\tbar[9].point\tpoints\t1\t31\t0\n",
            "S2_LEAN_CHECK\tW20\tcoverage.p10.second_production\tsecond_production\t1\t31\t0\n",
            "S2_LEAN_CHECK\tW20\tcoverage.o01.owner_type1_anchor\tcoverage_fixture\t1\t17\t0\n",
            "S2_LEAN_CHECK\tW20\tcoverage.p10.pipeline_nonempty\tcoverage_fixture\t1\t48\t0\n",
            "S2_LEAN_CHECK\tW20\tfailed.check\tpoints\t0\t1\t1\n",
            "S2_LEAN_COVERED\tbad-extra-column\tO01.force.tag\textra\n",
        );
        let covered = parse_lean_covered_paths(stdout);
        assert_eq!(
            covered.real_window,
            BTreeSet::from(["O01.source_index".to_owned()])
        );
        assert_eq!(
            covered.production_fixture,
            BTreeSet::from(["O01.owner.type1_anchor".to_owned()])
        );
        assert_eq!(
            covered.wire_schema_smoke,
            BTreeSet::from([
                "O01.owner.type1_anchor".to_owned(),
                "P10.memo.pipeline_07b_composition".to_owned(),
            ])
        );
        assert!(!covered.real_window.contains("O01.struct_break_dir.value"));
        assert!(!covered.real_window.contains("O01.force.tag"));
    }

    #[test]
    fn production_covered_requires_unique_check_with_exact_allowlisted_port() {
        let stdout = concat!(
            // valid: exactly one CHECK, exact port, pass, mismatch=0
            "S2_LEAN_COVERED\tcoverage.p10.third_tail\tP10.tail.points.length\n",
            "S2_LEAN_CHECK\tW20\tcoverage.p10.third_tail\tcoverage_fixture\t1\t74\t0\n",
            "S2_LEAN_COVERED\tcoverage.cp.review_production\tcp.full_trend_c_qualified.confirm_src\n",
            "S2_LEAN_CHECK\tW20\tcoverage.cp.review_production\tcp_review\t1\t121\t0\n",
            "S2_LEAN_COVERED\tcoverage.o01.first_production_present\tO01.struct_break_dir.value\n",
            "S2_LEAN_CHECK\tW20\tcoverage.o01.first_production_present\tpoints\t1\t41\t0\n",
            "S2_LEAN_COVERED\tcoverage.o03.grade_production_missing\tO03.grade.missing.reason\n",
            "S2_LEAN_CHECK\tW20\tcoverage.o03.grade_production_missing\tgrades\t1\t12\t0\n",
            // wrong port: must reject, never downgrade to wire
            "S2_LEAN_COVERED\tcoverage.p10.second_production\tO01.owner.type1_anchor\n",
            "S2_LEAN_CHECK\tW20\tcoverage.p10.second_production\tcoverage_fixture\t1\t31\t0\n",
            // duplicate path: even two successful rows are ambiguous and must reject
            "S2_LEAN_COVERED\tcoverage.p10.cache_frontier_pan\tP10.cache_after.pan_divs.length\n",
            "S2_LEAN_CHECK\tW20\tcoverage.p10.cache_frontier_pan\tcoverage_fixture\t1\t81\t0\n",
            "S2_LEAN_CHECK\tW20\tcoverage.p10.cache_frontier_pan\tcoverage_fixture\t1\t81\t0\n",
            // failed and missing CHECK rows must reject
            "S2_LEAN_COVERED\tcoverage.p10.pan_tail\tP10.tail.pan_divs.length\n",
            "S2_LEAN_CHECK\tW20\tcoverage.p10.pan_tail\tcoverage_fixture\t0\t69\t1\n",
            "S2_LEAN_COVERED\tcoverage.p10.first_grade_candidate_tail\tP10.tail.grades.length\n",
        );
        let covered = parse_lean_covered_paths(stdout);
        assert_eq!(
            covered.production_fixture,
            BTreeSet::from([
                "O01.struct_break_dir.value".to_owned(),
                "O03.grade.missing.reason".to_owned(),
                "P10.tail.points.length".to_owned(),
                "cp.full_trend_c_qualified.confirm_src".to_owned(),
            ])
        );
        assert!(covered.wire_schema_smoke.is_empty());
        assert!(covered.real_window.is_empty());
    }

    #[test]
    fn legacy_wire_variant_report_remains_complete_but_is_not_schema_credit() {
        let all = all_fixture_checks();
        assert!(fixture_variant_coverage(&all)
            .iter()
            .all(|family| family.complete));

        let mut without_none = all;
        without_none.remove("coverage.o01.first_none");
        assert!(fixture_variant_coverage(&without_none)
            .iter()
            .find(|family| family.family == "O01.first")
            .is_some_and(|family| !family.complete));
    }

    #[test]
    fn semantic_denominator_excludes_context_impossible_union_payloads() {
        let typed = declared_field_paths().into_iter().collect::<BTreeSet<_>>();
        let semantic = required_field_paths().into_iter().collect::<BTreeSet<_>>();

        for path in [
            "P10.confirmed_append.points.item.owner.type1_anchor",
            "P10.memo.direct_3a_points.item.retrace_breaks_type1.value",
            "P10.memo.second_07b_points.item.bits.third_class_entry.center_si",
            "P10.memo.second_07b_points.item.owner.center.start_index",
            "P10.memo.second_07b_points.item.struct_break_dir.value",
            "P10.memo.second_07b_points.item.force.seg_a.macd_area_bits",
            "O04.third_class_proof.value",
            "P10.memo.candidate_legs.item.confirmed_at.value",
        ] {
            assert!(
                typed.contains(path),
                "typed wire schema 必须继续声明 {path}"
            );
            assert!(
                !semantic.contains(path),
                "生产上下文不可能 payload 不得进入 semantic 分母: {path}"
            );
        }

        for path in [
            "P10.confirmed_append.points.item.owner.tag",
            "P10.memo.second_07b_points.item.owner.type1_anchor",
            "P10.memo.pipeline_points.item.force.seg_a.macd_area_bits",
            "O04.third_class_proof.tag",
            "O04.center_ids.left",
            "cp.full_trend_c_qualified.confirm_src",
        ] {
            assert!(
                semantic.contains(path),
                "真实可达字段不得从分母删除: {path}"
            );
        }

        assert_eq!(typed.len(), 871);
        assert_eq!(semantic.len(), 827);
    }

    #[test]
    fn coverage_report_cannot_gain_credit_from_paths_outside_its_denominator() {
        let report = coverage_report(
            BTreeSet::from(["kept".to_owned(), "typed-only".to_owned()]),
            &["kept".to_owned()],
        );
        assert_eq!(report.numerator, 1);
        assert_eq!(report.denominator, 1);
        assert_eq!(report.covered_paths, vec!["kept"]);
        assert!(report.uncovered_paths.is_empty());
    }

    #[test]
    fn semantic_signing_gate_uses_union_of_all_four_windows() {
        let required = vec!["a".to_owned(), "b".to_owned()];
        let first = coverage_report(BTreeSet::from(["a".to_owned()]), &required);
        let second = coverage_report(BTreeSet::from(["b".to_owned()]), &required);
        let aggregate = aggregate_coverage_reports([&first, &second], &required);

        assert_eq!(first.numerator, 1);
        assert_eq!(second.numerator, 1);
        assert_eq!(aggregate.numerator, aggregate.denominator);
    }

    #[test]
    fn fresh_raw_witness_never_overwrites_same_key_production_history() {
        let point = |source_index| BspPoint {
            source_index,
            bits: BspBits::default(),
            pivot_low: 0,
            pivot_high: 0,
            center: None,
            struct_break_dir: None,
            force: None,
            retrace_breaks_type1: None,
        };
        let key = (3, 5, 8);
        let mut history = MemoPointHistory::new();
        remember_point_witness(
            &mut history,
            1,
            key,
            &[point(54_890)],
            &[point(54_891)],
            true,
        );
        remember_point_witness(
            &mut history,
            1,
            key,
            &[point(56_421)],
            &[point(56_422)],
            false,
        );
        let (direct, second) = history.get(&(1, key)).expect("同键历史见证");
        assert_eq!(direct[0].source_index, 54_890);
        assert_eq!(second[0].source_index, 54_891);
    }

    #[test]
    fn same_key_later_miss_refreshes_latest_and_hit_must_match_it() {
        let point = |source_index| BspPoint {
            source_index,
            bits: BspBits::default(),
            pivot_low: 0,
            pivot_high: 0,
            center: None,
            struct_break_dir: None,
            force: None,
            retrace_breaks_type1: None,
        };
        let memo = |source_index, hit: bool| FourRcMemoCapture {
            level: 1,
            key: (3, 5, 8),
            cached_key_before: hit.then_some((3, 5, 8)),
            hit,
            prior_reused: [hit; 4],
            cache_after_matches_return: [true; 4],
            before_lengths: [0; 4],
            lengths: [1, 0, 0, 0],
            pipeline_points: std::rc::Rc::new(vec![point(source_index)]),
            direct_3a_points: vec![point(source_index)],
            second_07b_points: Some(Vec::new()),
            pan_divs: std::rc::Rc::new(Vec::new()),
            grades: std::rc::Rc::new(Vec::new()),
            candidates: std::rc::Rc::new(Vec::new()),
        };

        let mut point_history = MemoPointHistory::new();
        let mut history = MemoPortHistory::new();
        let first_miss = memo(2011, false);
        remember_point_witness(
            &mut point_history,
            1,
            (3, 5, 8),
            &first_miss.direct_3a_points,
            first_miss.second_07b_points.as_deref().unwrap(),
            true,
        );
        remember_memo_port_witness(&mut history, &first_miss, true);

        let later_miss = memo(2178, false);
        remember_point_witness(
            &mut point_history,
            1,
            (3, 5, 8),
            &later_miss.direct_3a_points,
            later_miss.second_07b_points.as_deref().unwrap(),
            true,
        );
        remember_memo_port_witness(&mut history, &later_miss, true);

        let (direct, _) = point_history.get(&(1, (3, 5, 8))).unwrap();
        assert_eq!(
            direct[0].source_index, 2178,
            "最近 production miss 必须刷新见证"
        );
        let latest = history.get(&(1, (3, 5, 8))).unwrap();
        assert_eq!(latest.pipeline_points[0].source_index, 2178);

        assert_hit_matches_latest_miss(&memo(2178, true), latest);
        let stale_hit = memo(2011, true);
        assert!(
            std::panic::catch_unwind(|| assert_hit_matches_latest_miss(&stale_hit, latest))
                .is_err(),
            "后续 hit 若仍返回旧驻留期值必须 fail loud"
        );
    }

    fn cp_tracker_fixture(
        lifecycle: CpLifecycleStatus,
        center_end: usize,
        departure_ordinal: Option<u64>,
    ) -> CpScanOwnership {
        CpScanOwnership {
            b_center_index: 7,
            b_center_id: ElementId {
                level: 1,
                ordinal: 7,
            },
            b_center: Center {
                start_index: 10,
                end_index: center_end,
                zd: 100,
                zg: 120,
                dd: 90,
                gg: 130,
            },
            departure_move_id: departure_ordinal.map(|ordinal| ElementId { level: 1, ordinal }),
            departure_interval: departure_ordinal.map(|_| (center_end, center_end + 3)),
            lifecycle,
            cp_certificate_confirm_src: (lifecycle == CpLifecycleStatus::Closed)
                .then_some(center_end + 10),
            c_structure: None,
            third_class_in_c: None,
            full_trend_evidence: None,
            full_trend_c_qualified: None,
        }
    }

    fn cp_init(after: CpScanOwnership) -> s2_mirror_capture::CpInitCapture {
        s2_mirror_capture::CpInitCapture {
            b_center_id: after.b_center_id,
            center: after.b_center,
            departure_move_id: after.departure_move_id,
            departure_interval: after.departure_interval,
            after,
        }
    }

    #[test]
    fn cp_transition_writer_is_emitted_even_when_after_equals_latest_state() {
        let cp = cp_tracker_fixture(CpLifecycleStatus::Pending, 20, None);
        let init = s2_mirror_capture::CpInitCapture {
            b_center_id: cp.b_center_id,
            center: cp.b_center,
            departure_move_id: None,
            departure_interval: None,
            after: cp.clone(),
        };
        let batch = CaptureBatch {
            cp_inits: vec![init.clone()],
            cp_lifecycle_events: vec![CpLifecycleEvent::Init(init)],
            ..CaptureBatch::default()
        };
        let known = BTreeMap::from([(cp_key(&cp), cp)]);
        let mut checks = Vec::new();
        append_cp_transition_checks(&mut checks, &batch, 42, &known);
        assert_eq!(
            checks.len(),
            1,
            "状态值相同也必须验证 init writer 的 raw 字段"
        );
        assert!(checks[0].contains("cp_event[0].init"));
    }

    #[test]
    fn pending_init_after_closed_state_is_an_ordered_rebuild_writer() {
        let closed = cp_tracker_fixture(CpLifecycleStatus::Closed, 20, Some(70));
        let pending = cp_tracker_fixture(CpLifecycleStatus::Pending, 20, Some(70));
        let repeated_init = cp_init(pending.clone());
        let batch = CaptureBatch {
            cp_lifecycle_events: vec![CpLifecycleEvent::Init(repeated_init)],
            ..CaptureBatch::default()
        };
        let mut checks = Vec::new();
        let latest = append_cp_transition_checks(
            &mut checks,
            &batch,
            4_999,
            &BTreeMap::from([(cp_key(&closed), closed.clone())]),
        );
        assert_eq!(checks.len(), 1, "重复 Init 仍须保留构造检查");
        assert_eq!(latest.get(&cp_key(&pending)), Some(&pending));
    }

    #[test]
    fn stable_reinherit_after_pending_init_restores_closed_state_in_event_order() {
        let closed = cp_tracker_fixture(CpLifecycleStatus::Closed, 20, Some(70));
        let pending = cp_tracker_fixture(CpLifecycleStatus::Pending, 20, Some(70));
        let batch = CaptureBatch {
            cp_lifecycle_events: vec![
                CpLifecycleEvent::Init(cp_init(pending.clone())),
                CpLifecycleEvent::Reinherit(s2_mirror_capture::CpReinheritCapture {
                    dirty_from: 80,
                    rebuilt_before: pending,
                    prior: Some(closed.clone()),
                    prior_is_stable: true,
                    after: closed.clone(),
                }),
            ],
            ..CaptureBatch::default()
        };
        let latest = append_cp_transition_checks(&mut Vec::new(), &batch, 5_000, &BTreeMap::new());
        assert_eq!(latest.get(&cp_key(&closed)), Some(&closed));
    }

    #[test]
    fn dirty_reset_allows_following_init_to_refresh_cp_tracker() {
        let closed = cp_tracker_fixture(CpLifecycleStatus::Closed, 20, Some(70));
        let dirty_pending = cp_tracker_fixture(CpLifecycleStatus::Pending, 20, None);
        let refreshed = cp_tracker_fixture(CpLifecycleStatus::Pending, 24, Some(71));
        let batch = CaptureBatch {
            cp_lifecycle_events: vec![
                CpLifecycleEvent::Dirty(s2_mirror_capture::CpDirtyCapture {
                    dirty_from: 18,
                    before: vec![closed.clone()],
                    after: vec![dirty_pending],
                    scan_from: 18,
                    pending_fallbacks: 1,
                    certificate_clear_recomputes: 1,
                }),
                CpLifecycleEvent::Init(cp_init(refreshed.clone())),
            ],
            ..CaptureBatch::default()
        };
        let latest = append_cp_transition_checks(
            &mut Vec::new(),
            &batch,
            5_000,
            &BTreeMap::from([(cp_key(&closed), closed)]),
        );
        assert_eq!(latest.get(&cp_key(&refreshed)), Some(&refreshed));
    }

    #[test]
    fn pending_same_key_init_can_refresh_center_and_departure() {
        let pending = cp_tracker_fixture(CpLifecycleStatus::Pending, 20, None);
        let refreshed = cp_tracker_fixture(CpLifecycleStatus::Pending, 24, Some(71));
        let batch = CaptureBatch {
            cp_lifecycle_events: vec![CpLifecycleEvent::Init(cp_init(refreshed.clone()))],
            ..CaptureBatch::default()
        };
        let latest = append_cp_transition_checks(
            &mut Vec::new(),
            &batch,
            5_001,
            &BTreeMap::from([(cp_key(&pending), pending)]),
        );
        assert_eq!(latest.get(&cp_key(&refreshed)), Some(&refreshed));
    }

    #[test]
    fn required_schema_includes_all_four_memo_value_ports() {
        let required = required_field_paths().into_iter().collect::<BTreeSet<_>>();
        for path in [
            "P10.memo.pipeline_points.item.source_index",
            "P10.memo.pan_divs.item.center.end_index",
            "P10.memo.grades.item.grade.missing.reason",
            "P10.memo.candidate_legs.item.key.rule_version",
        ] {
            assert!(required.contains(path), "memo schema 缺 {path}");
        }
    }

    #[test]
    fn raw_sequence_exact_prefix_rebuilds_original_values() {
        let plan = classify_raw_sequence(&[10_u64, 20], &[10, 20, 30]);
        assert_eq!(plan, RawSequencePlan::Prefix { len: 2 });
        assert_eq!(plan.rebuild(&[10, 20, 30]).unwrap(), vec![10, 20]);
    }

    #[test]
    fn raw_sequence_single_tail_fork_uses_tail_patch() {
        let plan = classify_raw_sequence(&[10_u64, 99], &[10, 20, 30]);
        assert_eq!(
            plan,
            RawSequencePlan::TailPatch {
                prefix_len: 1,
                tail: 99
            }
        );
        assert_eq!(plan.rebuild(&[10, 20, 30]).unwrap(), vec![10, 99]);
    }

    #[test]
    fn raw_sequence_early_fork_falls_back_to_full_literal() {
        let plan = classify_raw_sequence(&[10_u64, 99, 30], &[10, 20, 30, 40]);
        assert_eq!(plan, RawSequencePlan::Fallback(vec![10, 99, 30]));
        assert_eq!(plan.rebuild(&[10, 20, 30, 40]).unwrap(), vec![10, 99, 30]);
    }

    #[test]
    fn raw_sequence_overlong_input_falls_back_and_invalid_take_fails_loud() {
        let plan = classify_raw_sequence(&[10_u64, 20, 30], &[10, 20]);
        assert_eq!(plan, RawSequencePlan::Fallback(vec![10, 20, 30]));
        assert_eq!(plan.rebuild(&[10, 20]).unwrap(), vec![10, 20, 30]);

        let invalid = RawSequencePlan::<u64>::Prefix { len: 3 };
        assert!(invalid.rebuild(&[10, 20]).unwrap_err().contains("越过"));
    }

    #[test]
    fn raw_window_plan_classifies_every_field_and_preserves_each_gauge() {
        let base = raw_series_fixture(3, DivergenceGauge::ForceL);
        let mut fork = raw_series_fixture(2, DivergenceGauge::ThetaLex);
        fork.dif_bits[1] = 99;
        fork.closes_ticks[0] = 88;
        fork.strokes[1] = raw_stroke_fixture(99);
        let captures = BTreeMap::from([
            ("rawSeriesBaseCandidate".to_owned(), base),
            ("rawSeriesFork".to_owned(), fork),
        ]);

        let window = RawSeriesWindowPlan::new(&captures).unwrap();
        assert_eq!(window.base_definition_name, "rawSeriesBaseCandidate");
        let plan = window.definitions.get("rawSeriesFork").unwrap();
        assert_eq!(plan.hist_bits, RawSequencePlan::Prefix { len: 2 });
        assert_eq!(
            plan.dif_bits,
            RawSequencePlan::TailPatch {
                prefix_len: 1,
                tail: 99
            }
        );
        assert!(matches!(plan.closes_ticks, RawSequencePlan::Fallback(_)));
        assert_eq!(plan.close_src, RawSequencePlan::Prefix { len: 2 });
        assert!(matches!(plan.strokes, RawSequencePlan::TailPatch { .. }));
        assert_eq!(plan.gauge, DivergenceGauge::ThetaLex);
        let lean = plan.lean_expr();
        assert!(lean.contains("rawSeriesBaseHistBits.take 2"));
        assert!(lean.contains("rawSeriesBaseDifBits.take 1 ++ [99]"));
        assert!(lean.contains("closesTicks := [88, 21]"));
        assert!(lean.contains("gauge := DivergenceGaugeMirror.thetaLex"));
    }

    #[test]
    fn fixture_chunk_inlines_only_referenced_raw_definitions_once() {
        let captures = BTreeMap::from([
            (
                "rawSeries10".to_owned(),
                raw_series_fixture(3, DivergenceGauge::ForceL),
            ),
            (
                "rawSeries20".to_owned(),
                raw_series_fixture(2, DivergenceGauge::MacdArea),
            ),
        ]);
        let window = RawSeriesWindowPlan::new(&captures).unwrap();
        let checks = vec![
            "checkMergedScan \"a\" rawSeries20 rawA stableA cacheA appendA tailA outputA"
                .to_owned(),
            "checkCheckpointOutput \"b\" rawSeries20 cacheB tailB outputB".to_owned(),
        ];
        let source =
            fixture_chunk_source(&window, "S2MirrorRawBaseTiny", "tiny-0000", &checks, None)
                .unwrap();
        assert_eq!(source.matches("def rawSeries20 :").count(), 1);
        assert!(!source.contains("def rawSeries10 :"));
        assert_eq!(source.matches("def rawSeriesBaseHistBits :").count(), 0);
        assert_eq!(
            window
                .base_module_source()
                .matches("def rawSeriesBaseHistBits :")
                .count(),
            1
        );
        assert!(window
            .base_module_source()
            .contains("set_option maxRecDepth 1000000"));
        assert!(source.contains("import S2MirrorRawBaseTiny\n"));
        assert!(checks.iter().all(|check| source.contains(check)));

        let empty =
            fixture_chunk_source(&window, "S2MirrorRawBaseTiny", "tiny-empty", &[], None).unwrap();
        assert!(!empty.contains("import S2MirrorRawBaseTiny"));
        assert!(!empty.contains("rawSeriesBaseHistBits"));
        assert!(!empty.contains("MergedScanRawSeries :="));

        let unknown = vec!["checkMergedScan \"bad\" rawSeries999 x x x x x x x".to_owned()];
        assert!(
            fixture_chunk_source(&window, "S2MirrorRawBaseTiny", "tiny-bad", &unknown, None,)
                .unwrap_err()
                .contains("rawSeries999")
        );
    }

    #[test]
    #[ignore = "显式 smoke：编译 prefix/tail-patch/fallback raw 内联"]
    fn inline_raw_fixture_is_lean_green() {
        let base = raw_series_fixture(3, DivergenceGauge::ForceL);
        let mut fork = raw_series_fixture(2, DivergenceGauge::ThetaLex);
        fork.dif_bits[1] = 99;
        fork.closes_ticks[0] = 88;
        fork.strokes[1] = raw_stroke_fixture(99);
        let window = RawSeriesWindowPlan::new(&BTreeMap::from([
            ("rawSeries10".to_owned(), base),
            ("rawSeries20".to_owned(), fork),
        ]))
        .unwrap();
        let formal_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("formal");
        let artifact_dir =
            std::env::temp_dir().join(format!("s2-inline-raw-smoke-{}", std::process::id()));
        std::fs::create_dir_all(&artifact_dir).expect("创建 inline raw smoke 目录");
        std::fs::write(
            artifact_dir.join("S2MirrorRawTinySmokeShard0000.lean"),
            "stale",
        )
        .expect("预置 stale raw shard source");
        std::fs::write(
            artifact_dir.join("S2MirrorRawTinySmokeShard0000.olean"),
            "stale",
        )
        .expect("预置 stale raw shard olean");
        let compiled = compile_raw_base_module(&formal_dir, &artifact_dir, "TinySmoke", &window)
            .expect("编译单一 raw base module");
        assert_eq!(compiled.module_name, "S2MirrorRawBaseTinySmoke");
        assert_eq!(compiled.removed_stale_shards, 2);
        let source = fixture_chunk_source(
            &window,
            &compiled.module_name,
            "inline-raw-smoke",
            &["(let _series := rawSeries20; checkPortEnumeration \"inline-raw-smoke\" \"points\" 0 0)".to_owned()],
            None,
        )
        .unwrap();
        let path = artifact_dir.join("inline-raw-smoke.lean");
        std::fs::write(&path, source).expect("写 inline raw smoke fixture");
        let output = Command::new("lake")
            .args(["env", "lean"])
            .arg(&path)
            .env("LEAN_PATH", &artifact_dir)
            .current_dir(formal_dir)
            .output()
            .expect("启动 inline raw smoke Lean");
        let _ = std::fs::remove_dir_all(&artifact_dir);
        assert!(
            output.status.success(),
            "inline raw smoke 失败\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("S2_LEAN_SUMMARY\tinline-raw-smoke")
        );
    }

    #[test]
    #[ignore = "显式 smoke：调用本机 Lean toolchain"]
    fn extractor_owned_coverage_fixture_is_lean_green() {
        let formal_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("formal");
        let path = std::env::temp_dir().join("s2-phase3-coverage-smoke.lean");
        let mut production_checks = vec![cp_review_production_check()];
        production_checks.extend(first_production_checks());
        let source = format!(
            concat!(
                "import Origin.UnifiedScanMirrorRunner\n\n",
                "set_option maxHeartbeats 0\nset_option maxRecDepth 100000\n\n",
                "open NewChanlun.Origin.UnifiedScanMirrorBridge\n",
                "open NewChanlun.Origin.UnifiedScanMirrorRunner\n\n",
                "#eval emitReport \"coverage-smoke\" (coverageFixtureChecks {} ++ [{}])\n"
            ),
            coverage_fixture_wire(),
            production_checks.join(",\n")
        );
        std::fs::write(&path, source).expect("写 coverage smoke fixture");
        let output = Command::new("lake")
            .args(["env", "lean"])
            .arg(&path)
            .current_dir(formal_dir)
            .output()
            .expect("启动 coverage smoke Lean");
        assert!(
            output.status.success(),
            "coverage smoke 失败，fixture={}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("S2_LEAN_SUMMARY\tcoverage-smoke\t"));
        assert!(
            stdout.contains("\t0\n"),
            "coverage smoke mismatch 非零: {stdout}"
        );
        let covered = parse_lean_covered_paths(&stdout);
        let report = coverage_report(covered.production_fixture, &required_field_paths());
        eprintln!(
            "S2_PRODUCTION_FIXTURE_COVERAGE\t{}\t{}\t{}",
            report.numerator,
            report.denominator,
            report.uncovered_paths.join(",")
        );
        assert!(
            report.numerator > 0,
            "Runner 必须逐叶上报 production fixture 覆盖"
        );
    }
}
