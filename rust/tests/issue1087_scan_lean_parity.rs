//! #1087 真实 EQUS.MINI 窗口签收：Rust merged 对 Rust legacy full oracle，Lean 验完整 wire
//! 逐字段 bridge parity 与 λ_C（显式 ignored 重型测试；Lean 不独立重算四输出）。
use newchan_rust::theta_v0::{
    classifier::{self, issue1087_parity},
    config::ThetaConfig,
    parser,
    types::Bar,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::Command,
};

const BAR_COUNT: usize = 200_000;
const TOTAL_BAR_COUNT: usize = 3 * BAR_COUNT;
const SYMBOL: &str = "COST";
const YEARS: [u32; 4] = [2023, 2024, 2025, 2026];
const NS_PER_MINUTE: i64 = 60_000_000_000;
const NANOS_PER_CENT: i64 = 10_000_000;

#[derive(Clone, Copy)]
struct MinuteBar {
    minute: i64,
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
        symbol: SYMBOL,
        start: 0,
        end: BAR_COUNT,
    },
    WindowSpec {
        symbol: SYMBOL,
        start: BAR_COUNT,
        end: 2 * BAR_COUNT,
    },
    WindowSpec {
        symbol: SYMBOL,
        start: 2 * BAR_COUNT,
        end: 3 * BAR_COUNT,
    },
];

fn windows_are_pairwise_disjoint(windows: &[WindowSpec]) -> bool {
    windows.iter().enumerate().all(|(i, left)| {
        left.start < left.end
            && windows.iter().skip(i + 1).all(|right| {
                left.symbol != right.symbol || left.end <= right.start || right.end <= left.start
            })
    })
}

#[test]
fn three_acceptance_windows_are_same_symbol_contiguous_and_disjoint() {
    assert!(windows_are_pairwise_disjoint(&WINDOWS));
    assert!(WINDOWS
        .windows(2)
        .all(|pair| { pair[0].symbol == pair[1].symbol && pair[0].end == pair[1].start }));
    assert_eq!(WINDOWS[0].start, 0);
    assert_eq!(WINDOWS[2].end, TOTAL_BAR_COUNT);
}

/// 确定性锯齿，仅用于执行 wire 序列化/Lean bridge smoke；不冒充真实行情或独立语义 oracle。
fn synthetic_bridge_bars() -> Vec<Bar> {
    let mut bars = Vec::with_capacity(480);
    let mut close = 100_000i64;
    for source_index in 0..480usize {
        let swing = source_index / 7;
        let direction = if swing % 2 == 0 { 1 } else { -1 };
        close += direction * (300 + (swing % 7) as i64 * 10);
        bars.push(Bar {
            source_index,
            timestamp: source_index as i64 * NS_PER_MINUTE,
            open: close,
            high: close + 50,
            low: close - 50,
            close,
            volume: 1_000.0,
            untradable: false,
        });
    }
    bars
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

fn load_bars(root: &Path, symbol: &str, required: usize) -> Vec<Bar> {
    let mut completed = Vec::with_capacity(required);
    let mut active: Option<MinuteBar> = None;
    'years: for year in YEARS {
        let path = root.join(format!("{symbol}_{year}_trades.json"));
        let mut lines = BufReader::new(
            File::open(&path).unwrap_or_else(|e| panic!("无法读取 {}: {e}", path.display())),
        )
        .lines();
        assert_eq!(lines.next().unwrap().unwrap(), "ts_recv,ts_event,rtype,publisher_id,instrument_id,action,side,depth,price,size,flags,ts_in_delta,sequence");
        for (offset, line) in lines.enumerate() {
            let (timestamp, raw_price, size) = parse_trade(&line.unwrap(), &path, offset + 2);
            let minute = timestamp.div_euclid(NS_PER_MINUTE);
            assert!(raw_price >= 0, "价格必须非负");
            let price = (raw_price + NANOS_PER_CENT / 2) / NANOS_PER_CENT;
            match active.as_mut() {
                Some(bar) if bar.minute == minute => {
                    bar.high = bar.high.max(price);
                    bar.low = bar.low.min(price);
                    bar.close = price;
                    bar.volume += size;
                }
                Some(bar) => {
                    assert!(minute > bar.minute, "{} 时间戳非单调", path.display());
                    completed.push(*bar);
                    if completed.len() == required {
                        break 'years;
                    }
                    active = Some(MinuteBar {
                        minute,
                        open: price,
                        high: price,
                        low: price,
                        close: price,
                        volume: size,
                    });
                }
                None => {
                    active = Some(MinuteBar {
                        minute,
                        open: price,
                        high: price,
                        low: price,
                        close: price,
                        volume: size,
                    })
                }
            }
        }
    }
    assert_eq!(completed.len(), required, "{symbol} 可用完整 1m bars 不足");
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

fn output(value: &issue1087_parity::WireMergedScanOutput, rust: bool) -> String {
    format!(
        "{{ points := {}, panDivs := {}, grades := {}, observations := {} }}",
        list(&value.points, |value| point(value, rust)),
        list(&value.pan_divs, |value| pan(value, rust)),
        list(&value.grades, |value| grade_record(value, rust)),
        list(&value.observations, |value| observation(value, rust))
    )
}

fn write_fixture(path: &Path, records: &[issue1087_parity::ScanParityRecord]) {
    let mut out = File::create(path).unwrap();
    writeln!(out, "import Origin.ScanAssemblyBridge\nset_option maxRecDepth 1000000\nset_option maxHeartbeats 0\nnamespace Issue1087Generated\nopen NewChanlun.Origin.ScanAssemblyMirror\nopen NewChanlun.Origin.ScanAssemblyBridge").unwrap();
    for (record_index, record) in records.iter().enumerate() {
        writeln!(
            out,
            "def rustMerged{record_index} : RustMergedScanOutputExtraction := {}",
            output(&record.merged, true)
        )
        .unwrap();
        writeln!(
            out,
            "def oracleMerged{record_index} : MergedScanOutput := {}",
            output(&record.oracle, false)
        )
        .unwrap();
        writeln!(out, "example : MergedOutputParity rustMerged{record_index} oracleMerged{record_index} := by native_decide").unwrap();
        writeln!(out, "abbrev rows{record_index} : List SegmentRow := [").unwrap();
        for row in &record.rows {
            writeln!(out, "{{ direction := {}, startIndex := {}, endIndex := {}, startPrice := {}, endPrice := {} }},",
                lean_dir(row.direction), row.start_index, row.end_index, row.start_price, row.end_price).unwrap();
        }
        writeln!(out, "]").unwrap();
        for (chunk_index, chunk) in record.episode_cases.chunks(128).enumerate() {
            write!(out, "example : [").unwrap();
            for case in chunk {
                write!(out, "episodeStart rows{record_index} {{ zd := {}, zg := {}, dd := {}, gg := {}, startIndex := {}, endIndex := {} }} {} {},",
                    case.center_zd, case.center_zg, case.center_dd, case.center_gg,
                    case.center_start_index, case.center_end_index, lean_dir(case.departure_dir), case.until_start).unwrap();
            }
            writeln!(out, "] = [").unwrap();
            for case in chunk {
                match case.rust_lambda_c {
                    Some(v) => write!(out, "some {v},").unwrap(),
                    None => write!(out, "none,").unwrap(),
                }
            }
            writeln!(
                out,
                "] := by native_decide -- level={} chunk={chunk_index}",
                record.level
            )
            .unwrap();
        }
    }
    writeln!(out, "end Issue1087Generated").unwrap();
}

/// synthetic 分类未必产信号；追加一条明确标注为 serializer-only 的非空 Pan 列表，锁住非空 wire。
fn append_nonempty_wire_serializer_case(path: &Path) {
    let pan = issue1087_parity::WirePanDivCert {
        source_index: 13,
        side: issue1087_parity::WireSide::Long,
        center: issue1087_parity::WireCenter {
            zd: 100,
            zg: 110,
            dd: 90,
            gg: 120,
            start_index: 3,
            end_index: 9,
        },
        seg_a: issue1087_parity::WireInterval { left: 1, right: 2 },
        seg_c: issue1087_parity::WireInterval {
            left: 10,
            right: 13,
        },
    };
    let value = issue1087_parity::WireMergedScanOutput {
        points: Vec::new(),
        pan_divs: vec![pan],
        grades: Vec::new(),
        observations: Vec::new(),
    };
    let mut out = OpenOptions::new().append(true).open(path).unwrap();
    writeln!(out, "namespace Issue1087SyntheticSerializerOnly\nopen NewChanlun.Origin.ScanAssemblyMirror\nopen NewChanlun.Origin.ScanAssemblyBridge").unwrap();
    writeln!(
        out,
        "def rustNonempty : RustMergedScanOutputExtraction := {}",
        output(&value, true)
    )
    .unwrap();
    writeln!(
        out,
        "def leanNonempty : MergedScanOutput := {}",
        output(&value, false)
    )
    .unwrap();
    writeln!(out, "example : MergedOutputParity rustNonempty leanNonempty := by native_decide\nend Issue1087SyntheticSerializerOnly").unwrap();
}

#[test]
fn synthetic_wire_execution_smoke() {
    let config = ThetaConfig::default();
    let bars = synthetic_bridge_bars();
    issue1087_parity::begin();
    let layer = parser::parse_layer(&bars, &config);
    let _ = classifier::classify(&layer, &config, &[]);
    let records = issue1087_parity::finish();
    assert!(!records.is_empty(), "synthetic 分类必须经过统一扫描探针");
    assert!(
        records.iter().all(|record| record.total_mismatches() == 0),
        "synthetic Rust merged 必须逐字段等于 Rust legacy full oracle"
    );
    assert!(
        records.iter().any(|record| !record.rows.is_empty()),
        "synthetic 分类至少应产一条 SegmentRow serializer payload"
    );

    let formal = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("formal");
    let fixture = std::env::temp_dir().join(format!(
        "issue1087-synthetic-wire-{}.lean",
        std::process::id()
    ));
    write_fixture(&fixture, &records);
    append_nonempty_wire_serializer_case(&fixture);
    let lean = Command::new("lake")
        .current_dir(&formal)
        .args(["env", "lean"])
        .arg(&fixture)
        .output()
        .expect("无法执行 lake env lean");
    fs::remove_file(&fixture).expect("synthetic Lean fixture 删除失败");
    assert!(
        lean.status.success(),
        "synthetic Lean wire bridge parity/λ_C 执行失败：\n{}",
        String::from_utf8_lossy(&lean.stderr)
    );
}

#[test]
#[ignore = "#1087 重型签收：需 EQUS_MINI_DIR 和 lake；3×200k 真实 1m bars"]
fn three_real_windows_match_rust_oracle_and_pass_lean_wire_bridge() {
    let root = PathBuf::from(std::env::var("EQUS_MINI_DIR").expect("须设置 EQUS_MINI_DIR"));
    assert!(root.is_dir());
    assert!(windows_are_pairwise_disjoint(&WINDOWS));
    let all_bars = load_bars(&root, SYMBOL, TOTAL_BAR_COUNT);
    assert_eq!(all_bars.len(), TOTAL_BAR_COUNT);
    assert!(all_bars
        .windows(2)
        .all(|pair| pair[0].timestamp < pair[1].timestamp));
    for pair in WINDOWS.windows(2) {
        assert!(
            all_bars[pair[0].end - 1].timestamp < all_bars[pair[1].start].timestamp,
            "真实 timestamp 窗口必须严格分离"
        );
    }
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
        "Lean 构建失败：\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let config = ThetaConfig::default();
    for window in WINDOWS {
        let bars = &all_bars[window.start..window.end];
        assert_eq!(bars.len(), BAR_COUNT);
        assert_eq!(bars[0].source_index, window.start);
        assert_eq!(bars[BAR_COUNT - 1].source_index, window.end - 1);
        assert!(bars.windows(2).all(|p| p[0].timestamp < p[1].timestamp));
        issue1087_parity::begin();
        let layer = parser::parse_layer(bars, &config);
        let _ = classifier::classify(&layer, &config, &[]);
        let records = issue1087_parity::finish();
        for level in 0..=2 {
            let matches: Vec<_> = records
                .iter()
                .filter(|record| record.level == level)
                .collect();
            assert_eq!(matches.len(), 1, "{window:?} L{level} 必须恰有一份末端快照");
            let record = matches[0];
            assert_eq!(
                record.total_mismatches(),
                0,
                "{window:?} L{level}: {record:#?}"
            );
        }
        let fixture = std::env::temp_dir().join(format!(
            "issue1087-{}-{}-{}-{}.lean",
            window.symbol,
            window.start,
            window.end,
            std::process::id()
        ));
        write_fixture(&fixture, &records);
        let output = Command::new("lake")
            .current_dir(&formal)
            .args(["env", "lean"])
            .arg(&fixture)
            .output()
            .expect("无法执行 lake env lean");
        assert!(
            output.status.success(),
            "{window:?} Lean wire bridge parity/λ_C 验证失败：\n{}\n{}",
            String::from_utf8_lossy(&output.stderr),
            fixture.display()
        );
        fs::remove_file(&fixture).unwrap();
        eprintln!("#1087 {} bars=[{},{}) levels={} Rust merged↔legacy oracle 四输出=0 mismatch；Lean wire bridge parity + λ_C={} cases verified",
            window.symbol, window.start, window.end, records.len(), records.iter().map(|r| r.episode_cases.len()).sum::<usize>());
    }
}
