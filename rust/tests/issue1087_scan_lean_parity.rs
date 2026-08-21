//! #1087 真实 EQUS.MINI 窗口签收：Rust 生产 merged 是被检侧；Lean 从原始
//! rows/centers/blocks 重算 prelude，从逐段 sink 重算四输出，并重算 CandDelta 装配与 c_p
//! stable-revision 推进。三标的各 200k bars，L0/L1/L2 逐字段硬门；无 legacy oracle。
use newchan_rust::theta_v0::{
    classifier::{
        self, issue1087_parity,
        recursive_tower::{
            CandDeltaCpEdge, CandDeltaEvent, CompletedTrendDecomposition, CpLifecycleStatus,
            CpScanOwnership, CpStructureIdentity, ElementId, FullTrendCQualified,
            FullTrendQualificationEvidence, InternalSublevelCenters, NewExtremeInDirection,
            ParentCenterIdentity, ThirdClassInCp, TrendContext,
        },
        TowerCache,
    },
    config::ThetaConfig,
    parser,
    types::{Bar, Direction, Side},
};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::Command,
};

const BAR_COUNT: usize = 200_000;
const YEARS: [u32; 4] = [2023, 2024, 2025, 2026];
const NS_PER_MINUTE: i64 = 60_000_000_000;
const NANOS_PER_CENT: i64 = 10_000_000;
/// EQUS.MINI equities are admitted as integer cents; ThetaConfig must dequantize one Tick as $0.01.
const EQUS_MINI_TICK_SIZE: f64 = 0.01;

fn databento_nanodollars_to_tick(raw_price: i64) -> i64 {
    assert!(raw_price >= 0, "价格必须非负");
    raw_price
        .checked_add(NANOS_PER_CENT / 2)
        .expect("Databento nanodollar price rounding overflow")
        / NANOS_PER_CENT
}

fn tick_to_price(tick: i64, tick_size: f64) -> f64 {
    tick as f64 * tick_size
}

fn issue1087_config() -> ThetaConfig {
    let mut config = ThetaConfig::default();
    config.tick.tick_size = EQUS_MINI_TICK_SIZE;
    config.level.l_max = 8;
    config
}

#[derive(Clone, Copy)]
struct MinuteBar {
    minute: i64,
    first_event: i64,
    last_event: i64,
    open: i64,
    high: i64,
    low: i64,
    close: i64,
    volume: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WindowSpec {
    symbol: &'static str,
    start: usize,
    end: usize,
}

const WINDOWS: [WindowSpec; 3] = [
    WindowSpec {
        symbol: "AAPL",
        start: 0,
        end: BAR_COUNT,
    },
    WindowSpec {
        symbol: "MSFT",
        start: 0,
        end: BAR_COUNT,
    },
    WindowSpec {
        symbol: "NVDA",
        start: 0,
        end: BAR_COUNT,
    },
];

/// 完整验收入口没有 symbol 过滤器；它机械固定为 AAPL/MSFT/NVDA 三窗。
fn complete_acceptance_windows() -> [WindowSpec; 3] {
    WINDOWS
}

fn assert_complete_level_ladder(window: WindowSpec, stage: &str, raw_levels: &[u32]) {
    assert!(!raw_levels.is_empty(), "{window:?} {stage} 至少 L0");
    let mut levels = raw_levels.to_vec();
    levels.sort_unstable();
    let reached = *levels.last().expect("非空已断言");
    assert!(
        reached >= 2,
        "{window:?} {stage} 必须至少达到 L2，实际 L{reached}"
    );
    assert!(reached <= 8, "{window:?} {stage} l_max=8 不得越界");
    assert_eq!(
        levels,
        (0_u32..=reached).collect::<Vec<_>>(),
        "{window:?} {stage} 必须恰有连续且唯一的 L0..L{reached} 快照"
    );
}

fn windows_are_pairwise_disjoint(windows: &[WindowSpec]) -> bool {
    windows.iter().enumerate().all(|(i, left)| {
        left.start < left.end
            && windows.iter().skip(i + 1).all(|right| {
                left.symbol != right.symbol || left.end <= right.start || right.end <= left.start
            })
    })
}

#[test]
fn complete_acceptance_plan_is_exactly_aapl_msft_nvda() {
    assert_eq!(complete_acceptance_windows(), WINDOWS);
}

#[test]
#[should_panic(expected = "至少达到 L2")]
fn acceptance_stage_without_l2_is_rejected() {
    assert_complete_level_ladder(
        WindowSpec {
            symbol: "NEGATIVE",
            start: 0,
            end: BAR_COUNT,
        },
        "negative",
        &[0, 1],
    );
}

#[test]
fn three_acceptance_windows_are_three_symbols_and_pairwise_disjoint() {
    assert!(windows_are_pairwise_disjoint(&WINDOWS));
    assert!(WINDOWS.iter().enumerate().all(|(index, left)| WINDOWS
        .iter()
        .skip(index + 1)
        .all(|right| left.symbol != right.symbol)));
    assert!(WINDOWS
        .iter()
        .all(|window| window.start == 0 && window.end - window.start == BAR_COUNT));
}

fn parse_trade(line: &str, path: &Path, line_no: usize) -> (i64, i64, f64) {
    let fields: Vec<_> = line.split(',').collect();
    assert!(fields.len() >= 10, "{}:{} 缺字段", path.display(), line_no);
    let integer = |index: usize, name: &str| {
        fields[index]
            .parse::<i64>()
            .unwrap_or_else(|_| panic!("{}:{} {name} 非 i64", path.display(), line_no))
    };
    let size = fields[9]
        .parse::<f64>()
        .unwrap_or_else(|_| panic!("{}:{} size 非 f64", path.display(), line_no));
    (integer(1, "ts_event"), integer(8, "price"), size)
}

#[test]
fn databento_nanodollar_price_uses_cents_tick_roundtrip() {
    let raw_price = 123_450_000_000_i64; // $123.45 in Databento nanodollars.
    let tick = databento_nanodollars_to_tick(raw_price);
    assert_eq!(tick, 12_345);
    assert_eq!(tick_to_price(tick, EQUS_MINI_TICK_SIZE), 123.45);
    assert_eq!(
        (tick_to_price(tick, EQUS_MINI_TICK_SIZE) * 1_000_000_000.0).round() as i64,
        raw_price
    );
    assert_eq!(issue1087_config().tick.tick_size, EQUS_MINI_TICK_SIZE);
}

fn load_bars(root: &Path, symbol: &str, required: usize) -> Vec<Bar> {
    let mut completed = Vec::with_capacity(required);
    for year in YEARS {
        let path = root.join(format!("{symbol}_{year}_trades.json"));
        let mut lines = BufReader::new(
            File::open(&path)
                .unwrap_or_else(|error| panic!("无法读取 {}: {error}", path.display())),
        )
        .lines();
        assert_eq!(lines.next().unwrap().unwrap(), "ts_recv,ts_event,rtype,publisher_id,instrument_id,action,side,depth,price,size,flags,ts_in_delta,sequence");
        // Databento 文件按接收序落行，ts_event 可小幅倒序。按 event-time 分钟聚合，禁止把行序
        // 当作行情时序；OHLC 的 open/close 也由分钟内首末 ts_event 决定。
        let mut minutes: BTreeMap<i64, MinuteBar> = BTreeMap::new();
        for (offset, line) in lines.enumerate() {
            let (timestamp, raw_price, size) = parse_trade(&line.unwrap(), &path, offset + 2);
            let minute = timestamp.div_euclid(NS_PER_MINUTE);
            let price = databento_nanodollars_to_tick(raw_price);
            minutes
                .entry(minute)
                .and_modify(|bar| {
                    bar.high = bar.high.max(price);
                    bar.low = bar.low.min(price);
                    bar.volume += size;
                    if timestamp < bar.first_event {
                        bar.first_event = timestamp;
                        bar.open = price;
                    }
                    if timestamp >= bar.last_event {
                        bar.last_event = timestamp;
                        bar.close = price;
                    }
                })
                .or_insert(MinuteBar {
                    minute,
                    first_event: timestamp,
                    last_event: timestamp,
                    open: price,
                    high: price,
                    low: price,
                    close: price,
                    volume: size,
                });
        }
        completed.extend(minutes.into_values());
        if completed.len() >= required {
            break;
        }
    }
    assert!(
        completed.len() >= required,
        "{symbol} 可用完整 1m bars 不足"
    );
    completed.truncate(required);
    assert!(completed
        .windows(2)
        .all(|pair| pair[0].minute < pair[1].minute));
    completed
        .into_iter()
        .enumerate()
        .map(|(source_index, bar)| Bar {
            source_index,
            timestamp: bar.minute * NS_PER_MINUTE,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
            untradable: bar.volume <= 0.0,
        })
        .collect()
}

fn lean_dir(direction: issue1087_parity::WireDirection) -> &'static str {
    match direction {
        issue1087_parity::WireDirection::Up => "Direction.up",
        issue1087_parity::WireDirection::Down => "Direction.down",
    }
}

fn rust_dir(direction: issue1087_parity::WireDirection) -> &'static str {
    match direction {
        issue1087_parity::WireDirection::Up => "RustDirectionTag.up",
        issue1087_parity::WireDirection::Down => "RustDirectionTag.down",
    }
}

fn option<T>(value: &Option<T>, render: impl FnOnce(&T) -> String) -> String {
    value
        .as_ref()
        .map(|value| format!("some ({})", render(value)))
        .unwrap_or_else(|| "none".to_string())
}

fn list<T>(values: &[T], render: impl Fn(&T) -> String) -> String {
    format!(
        "[{}]",
        values.iter().map(render).collect::<Vec<_>>().join(",")
    )
}

fn interval(value: &issue1087_parity::WireInterval, rust: bool) -> String {
    if rust {
        format!("{{ left := {}, right := {} }}", value.left, value.right)
    } else {
        format!("({}, {})", value.left, value.right)
    }
}

fn side(value: issue1087_parity::WireSide, rust: bool) -> &'static str {
    match (value, rust) {
        (issue1087_parity::WireSide::Long, true) => "RustSideTag.long",
        (issue1087_parity::WireSide::Short, true) => "RustSideTag.short",
        (issue1087_parity::WireSide::Long, false) => "Side.long",
        (issue1087_parity::WireSide::Short, false) => "Side.short",
    }
}

fn kind(value: issue1087_parity::WireCandidateKind, rust: bool) -> &'static str {
    match (value, rust) {
        (issue1087_parity::WireCandidateKind::Trend, true) => "RustCandidateKindTag.trend",
        (issue1087_parity::WireCandidateKind::Pan, true) => "RustCandidateKindTag.pan",
        (issue1087_parity::WireCandidateKind::Trend, false) => "CandidateKind.trend",
        (issue1087_parity::WireCandidateKind::Pan, false) => "CandidateKind.pan",
    }
}

fn state(value: issue1087_parity::WireObservedState, rust: bool) -> &'static str {
    match (value, rust) {
        (issue1087_parity::WireObservedState::Provisional, true) => {
            "RustObservedStateTag.provisional"
        }
        (issue1087_parity::WireObservedState::Unresolved, true) => {
            "RustObservedStateTag.unresolved"
        }
        (issue1087_parity::WireObservedState::Confirmed, true) => "RustObservedStateTag.confirmed",
        (issue1087_parity::WireObservedState::Provisional, false) => "ObservedState.provisional",
        (issue1087_parity::WireObservedState::Unresolved, false) => "ObservedState.unresolved",
        (issue1087_parity::WireObservedState::Confirmed, false) => "ObservedState.confirmed",
    }
}

fn center(value: &issue1087_parity::WireCenter) -> String {
    format!(
        "{{ zd := {}, zg := {}, dd := {}, gg := {}, startIndex := {}, endIndex := {} }}",
        value.zd, value.zg, value.dd, value.gg, value.start_index, value.end_index
    )
}

fn third_class(value: &issue1087_parity::WireThirdClassEntry, rust: bool) -> String {
    format!(
        "{{ centerSi := {}, centerZd := {}, centerZg := {}, leaveInterval := {}, retestInterval := {} }}",
        value.center_si,
        value.center_zd,
        value.center_zg,
        interval(&value.leave_interval, rust),
        interval(&value.retest_interval, rust)
    )
}

fn bits(value: &issue1087_parity::WireBspBits, rust: bool) -> String {
    format!(
        "{{ buy1 := {}, buy2 := {}, buy3 := {}, sell1 := {}, sell2 := {}, sell3 := {}, thirdClassEntry := {} }}",
        value.buy1,
        value.buy2,
        value.buy3,
        value.sell1,
        value.sell2,
        value.sell3,
        option(&value.third_class_entry, |entry| third_class(entry, rust))
    )
}

fn owner(value: &issue1087_parity::WireOwner, rust: bool) -> String {
    match (value, rust) {
        (issue1087_parity::WireOwner::Center(value), true) => {
            format!("RustOwnerExtraction.center ({})", center(value))
        }
        (issue1087_parity::WireOwner::Type1Anchor(value), true) => {
            format!("RustOwnerExtraction.type1Anchor {value}")
        }
        (issue1087_parity::WireOwner::Center(value), false) => {
            format!("OwnerRef.center ({})", center(value))
        }
        (issue1087_parity::WireOwner::Type1Anchor(value), false) => {
            format!("OwnerRef.type1Anchor {value}")
        }
    }
}

fn force_features(value: &issue1087_parity::WireForceFeatures) -> String {
    format!(
        "{{ macdAreaBits := {}, difPeakBits := {}, priceAmplitude := {}, priceSpeedBits := {} }}",
        value.macd_area_bits, value.dif_peak_bits, value.price_amplitude, value.price_speed_bits
    )
}

fn force(value: &issue1087_parity::WireForceProxies) -> String {
    format!(
        "{{ segA := {}, segC := {} }}",
        force_features(&value.seg_a),
        force_features(&value.seg_c)
    )
}

fn point(value: &issue1087_parity::WireBspPoint, rust: bool) -> String {
    format!(
        "{{ sourceIndex := {}, bits := {}, pivotLow := {}, pivotHigh := {}, center := {}, structBreakDir := {}, force := {}, retraceBreaksType1 := {} }}",
        value.source_index,
        bits(&value.bits, rust),
        value.pivot_low,
        value.pivot_high,
        option(&value.center, |value| owner(value, rust)),
        option(&value.struct_break_dir, |value| side(*value, rust).to_string()),
        option(&value.force, force),
        option(&value.retrace_breaks_type1, |value| value.to_string())
    )
}

fn pan(value: &issue1087_parity::WirePanDivCert, rust: bool) -> String {
    format!(
        "{{ sourceIndex := {}, side := {}, center := {}, segA := {}, segC := {} }}",
        value.source_index,
        side(value.side, rust),
        center(&value.center),
        interval(&value.seg_a, rust),
        interval(&value.seg_c, rust)
    )
}

fn grade(value: &issue1087_parity::WireT3InCGrade, rust: bool) -> String {
    let prefix = if rust {
        "RustT3InCGradeExtraction"
    } else {
        "T3InCGrade"
    };
    match value {
        issue1087_parity::WireT3InCGrade::Present {
            leave_interval,
            retest_interval,
        } => format!(
            "{prefix}.present ({}) ({})",
            interval(leave_interval, rust),
            interval(retest_interval, rust)
        ),
        issue1087_parity::WireT3InCGrade::Missing(reason) => {
            let reason = match (reason, rust) {
                (issue1087_parity::WireT3InCGradeReason::MissingLeave, true) => {
                    "RustT3InCGradeReasonTag.missingLeave"
                }
                (issue1087_parity::WireT3InCGradeReason::MissingRetest, true) => {
                    "RustT3InCGradeReasonTag.missingRetest"
                }
                (issue1087_parity::WireT3InCGradeReason::SameDirection, true) => {
                    "RustT3InCGradeReasonTag.sameDirection"
                }
                (issue1087_parity::WireT3InCGradeReason::LeaveNotOutside, true) => {
                    "RustT3InCGradeReasonTag.leaveNotOutside"
                }
                (issue1087_parity::WireT3InCGradeReason::RetestReentered, true) => {
                    "RustT3InCGradeReasonTag.retestReentered"
                }
                (issue1087_parity::WireT3InCGradeReason::MissingLeave, false) => {
                    "T3InCGradeReason.missingLeave"
                }
                (issue1087_parity::WireT3InCGradeReason::MissingRetest, false) => {
                    "T3InCGradeReason.missingRetest"
                }
                (issue1087_parity::WireT3InCGradeReason::SameDirection, false) => {
                    "T3InCGradeReason.sameDirection"
                }
                (issue1087_parity::WireT3InCGradeReason::LeaveNotOutside, false) => {
                    "T3InCGradeReason.leaveNotOutside"
                }
                (issue1087_parity::WireT3InCGradeReason::RetestReentered, false) => {
                    "T3InCGradeReason.retestReentered"
                }
            };
            format!("{prefix}.missing {reason}")
        }
    }
}

fn grade_record(value: &issue1087_parity::WireFirstClassGradeRecord, rust: bool) -> String {
    format!(
        "{{ level := {}, sourceIndex := {}, side := {}, centerStartIndex := {}, centerEndIndex := {}, centerZd := {}, centerZg := {}, grade := {} }}",
        value.level,
        value.source_index,
        side(value.side, rust),
        value.center_start_index,
        value.center_end_index,
        value.center_zd,
        value.center_zg,
        grade(&value.grade, rust)
    )
}

fn parent(value: &issue1087_parity::WireParentFingerprint) -> String {
    format!(
        "{{ centerStart := {}, zd := {}, zg := {} }}",
        value.center_start, value.zd, value.zg
    )
}

fn key(value: &issue1087_parity::WireCandidateKey, rust: bool) -> String {
    format!(
        "{{ ruleVersion := {}, level := {}, kind := {}, side := {}, previousCenterStart := {}, parent := {}, segA := {}, cStart := {} }}",
        value.rule_version,
        value.level,
        kind(value.kind, rust),
        side(value.side, rust),
        option(&value.previous_center_start, |value| value.to_string()),
        parent(&value.parent),
        interval(&value.seg_a, rust),
        value.c_start
    )
}

fn predicates(value: &issue1087_parity::WireStructuralPredicates) -> String {
    format!(
        "{{ direction := {}, comparable := {}, extreme := {} }}",
        value.direction, value.comparable, value.extreme
    )
}

fn observation(value: &issue1087_parity::WireCandidateObservation, rust: bool) -> String {
    format!(
        "{{ key := {}, kind := {}, centerIds := {}, candidateGroupId := {}, pairId := {}, structuralPredicates := {}, extremeProof := {}, thirdClassProof := {}, interval := {}, state := {}, firstProvableAt := {}, confirmedAt := {} }}",
        key(&value.key, rust),
        kind(value.kind, rust),
        option(&value.center_ids, |value| interval(value, rust)),
        value.candidate_group_id,
        value.pair_id,
        predicates(&value.structural_predicates),
        interval(&value.extreme_proof, rust),
        option(&value.third_class_proof, |value| value.to_string()),
        interval(&value.interval, rust),
        state(value.state, rust),
        option(&value.first_provable_at, |value| value.to_string()),
        option(&value.confirmed_at, |value| value.to_string())
    )
}

fn candidate_leg(value: &issue1087_parity::WireCandidateLeg) -> String {
    format!(
        "{{ key := {}, kind := {}, centerIds := {}, structuralPredicates := {}, extremeProof := {}, thirdClassProof := {}, interval := {}, state := {}, firstProvableAt := {}, confirmedAt := {} }}",
        key(&value.key, true),
        kind(value.kind, true),
        option(&value.center_ids, |value| interval(value, true)),
        predicates(&value.structural_predicates),
        interval(&value.extreme_proof, true),
        option(&value.third_class_proof, |value| value.to_string()),
        interval(&value.interval, true),
        state(value.state, true),
        option(&value.first_provable_at, |value| value.to_string()),
        option(&value.confirmed_at, |value| value.to_string())
    )
}

fn output(value: &issue1087_parity::WireMergedScanOutput, rust: bool) -> String {
    format!(
        "{{ points := {}, panDivs := {}, grades := {}, observations := {} }}",
        list(&value.points, |value| point(value, rust)),
        list(&value.pan_divs, |value| pan(value, rust)),
        list(&value.grades, |value| grade_record(value, rust)),
        list(&value.observations, |value| observation(value, rust))
    )
}

fn emission(value: &issue1087_parity::WireScanSinkEmission) -> String {
    format!(
        "{{ bspPoints := {}, panDivCerts := {}, firstClassGrades := {}, candidateLegs := {} }}",
        list(&value.points, |value| point(value, true)),
        list(&value.pan_divs, |value| pan(value, true)),
        list(&value.grades, |value| grade_record(value, true)),
        list(&value.candidate_legs, candidate_leg)
    )
}

fn segment_extraction(value: &issue1087_parity::WireSegmentRow) -> String {
    format!(
        "{{ direction := {}, startIndex := {}, endIndex := {}, startPrice := {}, endPrice := {} }}",
        rust_dir(value.direction),
        value.start_index,
        value.end_index,
        value.start_price,
        value.end_price
    )
}

fn move_block(value: &issue1087_parity::WireMoveBlock) -> String {
    let kind = match value.kind {
        issue1087_parity::WireMoveKind::Trend => "RustMoveKindTag.trend",
        issue1087_parity::WireMoveKind::Consolidation => "RustMoveKindTag.consolidation",
    };
    format!(
        "{{ startCenter := {}, endCenter := {}, kind := {}, direction := {}, levelLift := {} }}",
        value.start_center,
        value.end_center,
        kind,
        option(&value.direction, |direction| rust_dir(*direction)
            .to_string()),
        value.level_lift
    )
}

fn a_segment(value: &issue1087_parity::WireASegmentEnvelope) -> String {
    format!(
        "{{ span := {}, low := {}, high := {} }}",
        interval(&value.span, true),
        value.low,
        value.high
    )
}

fn prelude_input(value: &issue1087_parity::WirePreludeInput) -> String {
    format!(
        "{{ rows := {}, centers := {}, blocks := {} }}",
        list(&value.rows, segment_extraction),
        list(&value.centers, center),
        list(&value.blocks, move_block)
    )
}

fn prelude_output(value: &issue1087_parity::WirePreludeOutput) -> String {
    format!(
        "{{ segmentsSorted := {}, centersSorted := {}, anchors := {}, trendGate := {}, firstMatchIdx := {}, aSegments := {} }}",
        value.segments_sorted,
        value.centers_sorted,
        list(&value.anchors, |entry| option(entry, |direction| rust_dir(*direction).to_string())),
        list(&value.trend_gate, |entry| option(entry, |direction| rust_dir(*direction).to_string())),
        list(&value.first_match_idx, |entry| option(entry, |index| index.to_string())),
        list(&value.a_segments, |entry| option(entry, a_segment))
    )
}

fn cp_scan_base(value: &issue1087_parity::WireCpScanBase) -> String {
    format!(
        "{{ bCenterIndex := {}, bCenterId := {}, bCenter := {}, departureMoveId := {}, departureInterval := {} }}",
        value.b_center_index,
        element_id(&value.b_center_id),
        center(&value.b_center),
        option(&value.departure_move_id, element_id),
        option(&value.departure_interval, |value| interval(value, true))
    )
}

fn unit_move_fact(value: &issue1087_parity::WireUnitMoveFact) -> String {
    format!(
        "{{ id := {}, startIndex := {}, endIndex := {}, low := {}, high := {}, center := {} }}",
        element_id(&value.id),
        value.start_index,
        value.end_index,
        value.low,
        value.high,
        option(&value.center, center)
    )
}

fn event_raw_context(value: &issue1087_parity::WireEventRawContext) -> String {
    format!(
        "{{ centers := {}, rows := {}, anchors := {}, cpScan := {}, unitMoves := {} }}",
        list(&value.centers, center),
        list(&value.rows, segment_extraction),
        list(&value.anchors, |entry| option(entry, |direction| rust_dir(
            *direction
        )
        .to_string())),
        list(&value.cp_scan, cp_scan_base),
        list(&value.unit_moves, unit_move_fact)
    )
}

fn actual_direction(value: Direction) -> &'static str {
    match value {
        Direction::Up => "RustDirectionTag.up",
        Direction::Down => "RustDirectionTag.down",
    }
}

fn actual_side(value: Side) -> &'static str {
    match value {
        Side::Long => "RustSideTag.long",
        Side::Short => "RustSideTag.short",
    }
}

fn element_id(value: &ElementId) -> String {
    format!(
        "{{ level := {}, ordinal := {} }}",
        value.level, value.ordinal
    )
}

fn element_ids(values: &[ElementId]) -> String {
    list(values, element_id)
}

fn parent_center_identity(value: &ParentCenterIdentity) -> String {
    format!(
        "{{ centerIndex := {}, centerId := {}, sourceInterval := {{ left := {}, right := {} }}, zd := {}, zg := {} }}",
        value.center_index,
        element_id(&value.center_id),
        value.source_interval.0,
        value.source_interval.1,
        value.zd,
        value.zg
    )
}

fn cp_structure_identity(value: &CpStructureIdentity) -> String {
    format!(
        "{{ level := {}, bCenterId := {}, departureMoveId := {}, terminalMoveId := {}, sourceStart := {}, sourceEnd := {} }}",
        value.level,
        element_id(&value.b_center_id),
        element_id(&value.departure_move_id),
        option(&value.terminal_move_id, element_id),
        value.source_start,
        option(&value.source_end, |value| value.to_string())
    )
}

fn third_class_in_cp(value: &ThirdClassInCp) -> String {
    format!(
        "{{ bCenterId := {}, cpDepartureMoveId := {}, departureMoveId := {}, retestMoveId := {}, departureInterval := {{ left := {}, right := {} }}, retestInterval := {{ left := {}, right := {} }}, pointSourceIndex := {}, side := {} }}",
        element_id(&value.b_center_id),
        element_id(&value.cp_departure_move_id),
        element_id(&value.departure_move_id),
        element_id(&value.retest_move_id),
        value.departure_interval.0,
        value.departure_interval.1,
        value.retest_interval.0,
        value.retest_interval.1,
        value.point_source_index,
        actual_side(value.side)
    )
}

fn trend_context(value: &TrendContext) -> String {
    format!(
        "{{ predecessorCenterId := {}, bCenterId := {}, direction := {} }}",
        element_id(&value.predecessor_center_id),
        element_id(&value.b_center_id),
        actual_direction(value.direction)
    )
}

fn new_extreme(value: &NewExtremeInDirection) -> String {
    format!(
        "{{ bCenterId := {}, direction := {}, referencePrice := {}, extremePrice := {}, extremeMoveId := {}, confirmSrc := {} }}",
        element_id(&value.b_center_id),
        actual_direction(value.direction),
        value.reference_price,
        value.extreme_price,
        element_id(&value.extreme_move_id),
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
        "{{ direction := {}, centerIds := {}, closingSuccessorMoveId := {}, confirmSrc := {} }}",
        actual_direction(value.direction),
        element_ids(&value.center_ids),
        element_id(&value.closing_successor_move_id),
        value.confirm_src
    )
}

fn full_trend_evidence(value: &FullTrendQualificationEvidence) -> String {
    format!(
        "{{ trendContext := {}, newExtremeInDirection := {}, internalSublevelCenters := {}, completedTrendDecomposition := {}, decompositionReviewMoveId := {}, decompositionReviewSrc := {} }}",
        option(&value.trend_context, trend_context),
        option(&value.new_extreme_in_direction, new_extreme),
        option(&value.internal_sublevel_centers, internal_centers),
        option(&value.completed_trend_decomposition, completed_trend),
        option(&value.decomposition_review_move_id, element_id),
        option(&value.decomposition_review_src, |value| value.to_string())
    )
}

fn full_trend_qualified(value: &FullTrendCQualified) -> String {
    format!(
        "{{ trendContext := {}, thirdClassInsideC := {}, newExtremeInDirection := {}, internalSublevelCenters := {}, completedTrendDecomposition := {}, confirmSrc := {} }}",
        trend_context(&value.trend_context),
        third_class_in_cp(&value.third_class_inside_c),
        new_extreme(&value.new_extreme_in_direction),
        internal_centers(&value.internal_sublevel_centers),
        completed_trend(&value.completed_trend_decomposition),
        value.confirm_src
    )
}

fn cp_edge(value: &CandDeltaCpEdge) -> String {
    format!(
        "{{ bCenterId := {}, cpDepartureMoveId := {}, cpSourceStart := {} }}",
        element_id(&value.b_center_id),
        element_id(&value.cp_departure_move_id),
        value.cp_source_start
    )
}

fn event_extraction(value: &CandDeltaEvent) -> String {
    format!(
        "{{ level := {}, side := {}, divergenceConfirmSrc := {}, confirmSrc := {}, intervalLeft := {}, intervalRight := {}, aIntervalLeft := {}, aIntervalRight := {}, cEpisodeStart := {}, cEpisodeLeft := {}, cEpisodeRight := {}, cIntervalFull := {}, bParent := {}, cStructure := {}, thirdClassInC := {}, cpCertificateConfirmSrc := {}, fullTrendCQualified := {}, fullTrendEvidence := {}, cpOwnership := {}, enterSrc := {}, candDelta := {}, panDivDiag := {} }}",
        value.level,
        actual_side(value.side),
        value.divergence_confirm_src,
        value.confirm_src,
        value.interval.0,
        value.interval.1,
        value.a_interval.0,
        value.a_interval.1,
        value.c_episode_start,
        value.c_episode_interval.0,
        value.c_episode_interval.1,
        option(&value.c_interval_full, |value| format!("{{ left := {}, right := {} }}", value.0, value.1)),
        option(&value.b_parent, parent_center_identity),
        option(&value.c_structure, cp_structure_identity),
        option(&value.third_class_in_c, third_class_in_cp),
        option(&value.cp_certificate_confirm_src, |value| value.to_string()),
        option(&value.full_trend_c_qualified, full_trend_qualified),
        option(&value.full_trend_evidence, full_trend_evidence),
        option(&value.cp_ownership, cp_edge),
        value.enter_src,
        value.cand_delta,
        value.pan_div_diag
    )
}

fn actual_cp_lifecycle(value: CpLifecycleStatus) -> &'static str {
    match value {
        CpLifecycleStatus::Pending => "RustCpLifecycleTag.pending",
        CpLifecycleStatus::Closed => "RustCpLifecycleTag.closed",
    }
}

fn actual_center(value: &newchan_rust::theta_v0::types::Center) -> String {
    format!(
        "{{ zd := {}, zg := {}, dd := {}, gg := {}, startIndex := {}, endIndex := {} }}",
        value.zd, value.zg, value.dd, value.gg, value.start_index, value.end_index
    )
}

fn actual_segment(value: &newchan_rust::theta_v0::types::Segment) -> String {
    format!(
        "{{ direction := {}, startIndex := {}, endIndex := {}, startPrice := {}, endPrice := {} }}",
        actual_direction(value.direction),
        value.start_index,
        value.end_index,
        value.start_price,
        value.end_price
    )
}

fn cp_object(value: &CpScanOwnership) -> String {
    format!(
        "{{ bCenterIndex := {}, bCenterId := {}, bCenter := {}, departureMoveId := {}, departureInterval := {}, lifecycle := {}, cpCertificateConfirmSrc := {}, cStructure := {}, thirdClassInC := {}, fullTrendEvidence := {}, fullTrendCQualified := {} }}",
        value.b_center_index,
        element_id(&value.b_center_id),
        actual_center(&value.b_center),
        option(&value.departure_move_id, element_id),
        option(&value.departure_interval, |value| format!("{{ left := {}, right := {} }}", value.0, value.1)),
        actual_cp_lifecycle(value.lifecycle),
        option(&value.cp_certificate_confirm_src, |value| value.to_string()),
        option(&value.c_structure, cp_structure_identity),
        option(&value.third_class_in_c, third_class_in_cp),
        option(&value.full_trend_evidence, full_trend_evidence),
        option(&value.full_trend_c_qualified, full_trend_qualified)
    )
}

fn cp_closure(value: &issue1087_parity::WireCpClosureEvidence, context_name: &str) -> String {
    format!(
        "{{ context := {context_name}, visibleUnitMoveCount := {}, cpDepartureMoveId := {}, cpStart := {}, leave := {}, retest := {}, leaveAnchor := {}, leaveMoveId := {}, retestMoveId := {} }}",
        value.visible_unit_move_count,
        element_id(&value.cp_departure_move_id),
        value.cp_start,
        actual_segment(&value.leave),
        actual_segment(&value.retest),
        option(&value.leave_anchor, |value| actual_direction(*value).to_string()),
        element_id(&value.leave_move_id),
        element_id(&value.retest_move_id),
    )
}

fn write_fixture(path: &Path, records: &[issue1087_parity::ScanParityRecord]) {
    let mut out = File::create(path).unwrap();
    writeln!(out, "import Origin.ScanAssemblyBridge\nset_option maxRecDepth 1000000\nset_option maxHeartbeats 0\nnamespace Issue1087Generated\nopen NewChanlun.Origin.ScanAssemblyMirror\nopen NewChanlun.Origin.ScanAssemblyBridge").unwrap();
    for (record_index, record) in records.iter().enumerate() {
        writeln!(
            out,
            "def preludeInput{record_index} : RustPreludeInputExtraction := {}",
            prelude_input(&record.prelude_input)
        )
        .unwrap();
        writeln!(
            out,
            "def rustPrelude{record_index} : RustPreludeOutputExtraction := {}",
            prelude_output(&record.prelude_output)
        )
        .unwrap();
        writeln!(
            out,
            "def candContext{record_index} : RustEventRawContextExtraction := {}",
            event_raw_context(&record.event_context)
        )
        .unwrap();
        writeln!(
            out,
            "def leanPrelude{record_index} : PreludeOutput := recomputeRustPrelude preludeInput{record_index}"
        )
        .unwrap();
        writeln!(out, "example : PreludeParity rustPrelude{record_index} leanPrelude{record_index} := by native_decide").unwrap();
        writeln!(out, "abbrev rows{record_index} : List SegmentRow := preludeInput{record_index}.rows.map RustSegmentExtraction.toLean").unwrap();

        writeln!(
            out,
            "def rustMerged{record_index} : RustMergedScanOutputExtraction := {}",
            output(&record.rust_output, true)
        )
        .unwrap();
        writeln!(
            out,
            "def rustEmissions{record_index} : List RustScanSinkEmissionExtraction := ["
        )
        .unwrap();
        for value in &record.emissions {
            writeln!(out, "{},", emission(value)).unwrap();
        }
        writeln!(out, "]").unwrap();
        writeln!(
            out,
            "def leanRecomputed{record_index} : MergedScanOutput := recomputeRustScanSinkEmissions {} rustEmissions{record_index}",
            record.level
        )
        .unwrap();
        // 四件分别立门；任何一个字段、长度或顺序 mismatch 都使 Lean 编译 FAIL。
        writeln!(out, "example : PointsParity rustMerged{record_index}.points leanRecomputed{record_index}.points = true := by native_decide").unwrap();
        writeln!(out, "example : PanDivsParity rustMerged{record_index}.panDivs leanRecomputed{record_index}.panDivs = true := by native_decide").unwrap();
        writeln!(out, "example : GradesParity rustMerged{record_index}.grades leanRecomputed{record_index}.grades = true := by native_decide").unwrap();
        writeln!(out, "example : ObservationsParity rustMerged{record_index}.observations leanRecomputed{record_index}.observations = true := by native_decide").unwrap();
        writeln!(out, "example : MergedOutputParity rustMerged{record_index} leanRecomputed{record_index} := by native_decide").unwrap();

        let episode_sample: Vec<_> = record.episode_cases.iter().collect();
        if !episode_sample.is_empty() {
            write!(out, "example : [").unwrap();
            for case in &episode_sample {
                write!(out, "episodeStart rows{record_index} {{ zd := {}, zg := {}, dd := {}, gg := {}, startIndex := {}, endIndex := {} }} {} {},",
                    case.center_zd, case.center_zg, case.center_dd, case.center_gg,
                    case.center_start_index, case.center_end_index, lean_dir(case.departure_dir), case.until_start).unwrap();
            }
            writeln!(out, "] = [").unwrap();
            for case in &episode_sample {
                match case.rust_lambda_c {
                    Some(value) => write!(out, "some {value},").unwrap(),
                    None => write!(out, "none,").unwrap(),
                }
            }
            writeln!(
                out,
                "] := by native_decide -- level={} all deterministic λ_C cases",
                record.level
            )
            .unwrap();
        }

        for (case_index, case) in record.cand_delta_cases.iter().enumerate() {
            let event_kind = match case.kind {
                issue1087_parity::WireCandidateKind::Trend => "RustCandDeltaKindTag.trend",
                issue1087_parity::WireCandidateKind::Pan => "RustCandDeltaKindTag.pan",
            };
            writeln!(out, "def candInput{record_index}_{case_index} : RustAssemblyInputExtraction := {{ context := candContext{record_index}, centerIndex := {}, segmentIndex := {}, level := {}, side := {}, divergenceConfirmSrc := {}, aIntervalLeft := {}, aIntervalRight := {}, predicate := {{ kind := {}, structuralCandidate := {}, direction := {}, comparable := {}, extreme := {}, buy1 := {}, sell1 := {}, panDiverges := {} }}, panDivDiag := {} }}",
                case.center_index, case.segment_index,
                record.level, side(case.side, true), case.divergence_confirm_src,
                case.a_interval.left, case.a_interval.right, event_kind, case.structural_candidate,
                case.direction, case.comparable, case.extreme, case.buy1, case.sell1,
                case.pan_diverges,
                case.pan_div_diag).unwrap();
        }
        write!(
            out,
            "def candInputs{record_index} : List RustAssemblyInputExtraction := ["
        )
        .unwrap();
        for case_index in 0..record.cand_delta_cases.len() {
            write!(out, "candInput{record_index}_{case_index},").unwrap();
        }
        writeln!(out, "]").unwrap();
        writeln!(
            out,
            "def candEvents{record_index} : List RustEventExtraction := ["
        )
        .unwrap();
        for event in &record.cand_delta_events {
            writeln!(out, "{},", event_extraction(event)).unwrap();
        }
        writeln!(out, "]").unwrap();
        writeln!(
            out,
            "example : EventBijectionCoreCheck candInputs{record_index} candEvents{record_index} := by native_decide"
        )
        .unwrap();

        for (transition_index, transition) in record.cp_transitions.iter().enumerate() {
            writeln!(
                out,
                "def cpBefore{record_index}_{transition_index} : RustCpObjectExtraction := {}",
                cp_object(&transition.before)
            )
            .unwrap();
            writeln!(
                out,
                "def cpAfter{record_index}_{transition_index} : RustCpObjectExtraction := {}",
                cp_object(&transition.after)
            )
            .unwrap();
            writeln!(
                out,
                "def cpRaw{record_index}_{transition_index} : RustCpClosureEvidenceExtraction := {}",
                cp_closure(&transition.raw, &format!("candContext{record_index}"))
            )
            .unwrap();
            writeln!(out, "example : RecomputeCpClosureCheck cpBefore{record_index}_{transition_index} cpAfter{record_index}_{transition_index} cpRaw{record_index}_{transition_index} := by native_decide").unwrap();
        }
    }
    write!(
        out,
        "def allCandInputs : List RustAssemblyInputExtraction := "
    )
    .unwrap();
    for record_index in 0..records.len() {
        write!(out, "candInputs{record_index} ++ ").unwrap();
    }
    writeln!(out, "[]").unwrap();
    write!(out, "def allCandEvents : List RustEventExtraction := ").unwrap();
    for record_index in 0..records.len() {
        write!(out, "candEvents{record_index} ++ ").unwrap();
    }
    writeln!(out, "[]").unwrap();
    writeln!(
        out,
        "example : EventBijectionCheck allCandInputs allCandEvents := by native_decide"
    )
    .unwrap();
    writeln!(out, "end Issue1087Generated").unwrap();
}

fn equs_mini_root() -> PathBuf {
    if let Some(root) = std::env::var_os("EQUS_MINI_DIR") {
        return PathBuf::from(root);
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("git")
        .current_dir(manifest)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .expect("无法定位 git common dir；可显式设置 EQUS_MINI_DIR");
    assert!(
        output.status.success(),
        "无法定位 git common dir；可显式设置 EQUS_MINI_DIR"
    );
    let common = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
    common
        .parent()
        .expect("git common dir 没有仓库父目录")
        .join("analysis/data_cache/equs_mini")
}

fn cand_case_is_accepted(case: &issue1087_parity::WireCandDeltaCase) -> bool {
    match case.kind {
        issue1087_parity::WireCandidateKind::Trend => {
            case.structural_candidate && case.direction && case.comparable && case.extreme
        }
        issue1087_parity::WireCandidateKind::Pan => case.structural_candidate && case.pan_diverges,
    }
}

fn stage_event_domain_counts(records: &[issue1087_parity::ScanParityRecord]) -> (usize, usize) {
    let accepted_raw = records
        .iter()
        .flat_map(|record| &record.cand_delta_cases)
        .filter(|case| cand_case_is_accepted(case))
        .count();
    let production_events = records
        .iter()
        .map(|record| record.cand_delta_events.len())
        .sum();
    (accepted_raw, production_events)
}

fn verify_records_in_lean(
    formal: &Path,
    window: WindowSpec,
    stage: &str,
    records: &[issue1087_parity::ScanParityRecord],
) {
    assert!(!records.is_empty(), "{window:?} {stage} 不得无快照");
    let mut levels: Vec<_> = records.iter().map(|record| record.level).collect();
    levels.sort_unstable();
    levels.dedup();
    assert_eq!(
        levels.len(),
        records.len(),
        "{window:?} {stage} 每级必须恰一份快照"
    );
    assert_complete_level_ladder(window, stage, &levels);
    let (accepted_raw, production_events) = stage_event_domain_counts(records);
    assert!(
        accepted_raw > 0,
        "{window:?} {stage} 必须至少有一个 Lean 可接受 raw，实际 0"
    );
    assert!(
        production_events > 0,
        "{window:?} {stage} 必须至少有一个 production event，实际 0"
    );
    for record in records {
        assert!(
            !record.prelude_input.rows.is_empty(),
            "{window:?} {stage} L{} rows 空",
            record.level
        );
        if record.level <= 2 {
            assert!(
                !record.prelude_input.centers.is_empty(),
                "{window:?} {stage} L{} centers 空",
                record.level
            );
            assert!(
                !record.episode_cases.is_empty(),
                "{window:?} {stage} L{} λ_C 空",
                record.level
            );
        }
    }
    let fixture = std::env::temp_dir().join(format!(
        "issue1087-{}-{}-{}-{}-{}.lean",
        window.symbol,
        window.start,
        window.end,
        stage,
        std::process::id()
    ));
    write_fixture(&fixture, records);
    let lean = Command::new("lake")
        .current_dir(formal)
        .args(["env", "lean"])
        .arg(&fixture)
        .output()
        .expect("无法执行 lake env lean");
    assert!(
        lean.status.success(),
        "{window:?} {stage} Lean prelude/四输出/CandDelta/c_p mismatch：
{}
{}
fixture={}",
        String::from_utf8_lossy(&lean.stdout),
        String::from_utf8_lossy(&lean.stderr),
        fixture.display()
    );
    fs::remove_file(&fixture).unwrap();
}

#[test]
fn three_real_windows_recompute_in_lean_and_match_every_field() {
    let root = equs_mini_root();
    assert!(
        root.is_dir(),
        "真实 EQUS.MINI 目录不存在：{}；可设置 EQUS_MINI_DIR",
        root.display()
    );
    assert!(windows_are_pairwise_disjoint(&WINDOWS));
    let formal = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("formal");
    let build = Command::new("lake")
        .current_dir(&formal)
        .args([
            "build",
            "Origin.ScanAssemblyMirror",
            "Origin.ScanAssemblyBridge",
        ])
        .output()
        .expect("无法执行 lake build");
    assert!(
        build.status.success(),
        "Lean 构建失败：\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let config = issue1087_config();
    let mut executed_windows = Vec::with_capacity(WINDOWS.len());
    for window in complete_acceptance_windows() {
        let all_bars = load_bars(&root, window.symbol, window.end);
        let bars = &all_bars[window.start..window.end];
        assert_eq!(bars.len(), BAR_COUNT);
        assert_eq!(bars[0].source_index, window.start);
        assert_eq!(bars[BAR_COUNT - 1].source_index, window.end - 1);
        assert!(bars
            .windows(2)
            .all(|pair| pair[0].timestamp < pair[1].timestamp));

        // checkpoint → 末根 → 回缩 → 末根恢复。四次都复用同一 TowerCache，真实穿过
        // cache/frontier resume 与 shrink invalidation；每次 Rust 快照都由 fresh raw sink 喂 Lean。
        let stages = [
            ("checkpoint", BAR_COUNT / 2),
            ("terminal", BAR_COUNT),
            ("shrink", BAR_COUNT / 2),
            ("terminal_resume", BAR_COUNT),
        ];
        let mut cache = TowerCache::new();
        let mut saw_cand = false;
        let mut saw_rejected = false;
        let mut saw_cp_close = false;
        for (stage, prefix) in stages {
            issue1087_parity::begin();
            let layer = parser::parse_layer(&bars[..prefix], &config);
            let _ = classifier::classify_incremental(&layer, &config, &mut cache, &[]);
            let records = issue1087_parity::finish();
            verify_records_in_lean(&formal, window, stage, &records);
            let (accepted_raw, production_events) = stage_event_domain_counts(&records);

            saw_cand |= records
                .iter()
                .any(|record| !record.cand_delta_cases.is_empty());
            saw_rejected |= records
                .iter()
                .flat_map(|record| &record.cand_delta_cases)
                .any(|case| !cand_case_is_accepted(case));
            saw_cp_close |= records
                .iter()
                .flat_map(|record| &record.cp_transitions)
                .any(|transition| {
                    transition.before.lifecycle == CpLifecycleStatus::Pending
                        && transition.after.lifecycle == CpLifecycleStatus::Closed
                });
            eprintln!(
                "#1087 {} stage={} prefix={} levels={} cand_cases={} accepted_raw={} production_events={} rejected={} cp_closed={} PASS",
                window.symbol,
                stage,
                prefix,
                records.len(),
                records
                    .iter()
                    .map(|record| record.cand_delta_cases.len())
                    .sum::<usize>(),
                accepted_raw,
                production_events,
                records
                    .iter()
                    .flat_map(|record| &record.cand_delta_cases)
                    .filter(|case| !cand_case_is_accepted(case))
                    .count(),
                records
                    .iter()
                    .flat_map(|record| &record.cp_transitions)
                    .filter(|transition| {
                        transition.before.lifecycle == CpLifecycleStatus::Pending
                            && transition.after.lifecycle == CpLifecycleStatus::Closed
                    })
                    .count(),
            );
        }
        assert!(saw_cand, "{window:?} 必须有 event 前 raw CandDelta case");
        assert!(saw_rejected, "{window:?} 必须覆盖至少一个 Lean 判拒样本");
        assert!(
            saw_cp_close,
            "{window:?} 必须覆盖 raw closure 判定的 Pending→Closed"
        );
        executed_windows.push(window);
    }
    assert_eq!(
        executed_windows.as_slice(),
        WINDOWS.as_slice(),
        "完整验收必须恰好执行 AAPL/MSFT/NVDA 三窗"
    );
}
