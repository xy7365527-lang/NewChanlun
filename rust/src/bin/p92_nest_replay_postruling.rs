//! task #92：裁定后 NestCertificate 因果 prefix 重放与 A/B 口径对账。
//!
//! ★口径版本：p117 T1 终端背书裁定（bsp-terminal-endorsement-ruling-20260718，2026-07-18
//! 生效）——终端查法自 `levels[ℓ]` 位格等式移位到 `levels[ℓ-1].bsp` + C-b 窗口
//!（`[interval_b.0, turn_source]` 最早 confirm_side 点；lib 单一来源
//! `nest::terminal_bits_at_event`/`terminal_bits_in_book`，含旧事件路径）。本文件此前的
//! 落盘读数引用时须注记「级别移位前口径」（T4 裁决5）。
//!
//! 用法：
//! `cargo run --release --bin p92_nest_replay_postruling -- <btc_1m_full.json>`

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events,
    C2LevelViewConfig, C2VersionTuple, CoordinateWindow, LevelViewMaterial, LevelViewQuery,
    NestCandidateEvent, NestDivergenceKind, ProjectionError, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::nest::{
    assemble_certificates_snapshot, assemble_typed_certificates, event_bsp_book_level,
    terminal_bits_at_event, terminal_bits_in_book, NestIntervalCaliber, TerminalMatch,
    TypedNestCertificate,
};
use newchan_rust::theta_v0::classifier::turn_class::{
    classify_certificate_turn, is_defer_orphan_event, NestTurnClass,
};
use newchan_rust::theta_v0::classifier::recursive_tower::LeveledMove;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::{ParseLayer, ParseLayerIncr};
use newchan_rust::theta_v0::types::{quantize, Bar, BspBits, Side, Timestamp};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// #98/#99 调研侧信道：`P92_DUMP=<path>` 时逐条落盘明细行。
/// 只写不判——不参与任何账本、真值或裁定；未设 env 时零行为差异。
static DUMP: OnceLock<Option<Mutex<BufWriter<File>>>> = OnceLock::new();

fn dump_line(args: std::fmt::Arguments<'_>) {
    let sink = DUMP.get_or_init(|| {
        std::env::var("P92_DUMP")
            .ok()
            .and_then(|path| File::create(path).ok())
            .map(|file| Mutex::new(BufWriter::new(file)))
    });
    if let Some(sink) = sink {
        if let Ok(mut writer) = sink.lock() {
            let _ = writer.write_fmt(args).and_then(|()| writer.write_all(b"\n"));
        }
    }
}

fn dump_flush() {
    if let Some(Some(sink)) = DUMP.get() {
        if let Ok(mut writer) = sink.lock() {
            let _ = writer.flush();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EventKey {
    level: u32,
    short: bool,
    kind: NestDivergenceKind,
    seg_a: (usize, usize),
    interval_b: (usize, usize),
    interval_a: (usize, usize),
    turn_source: usize,
}

impl From<&NestCandidateEvent> for EventKey {
    fn from(event: &NestCandidateEvent) -> Self {
        Self {
            level: event.level,
            short: event.side == Side::Short,
            kind: event.kind,
            seg_a: event.seg_a,
            interval_b: event.interval_b,
            interval_a: event.interval_a,
            turn_source: event.turn_source,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct ProviderAudit {
    snapshots: usize,
    views: usize,
    projection_too_short: usize,
    projection_invalid_seed: usize,
    projection_missing_carried_center: usize,
    projection_other: usize,
    snapshot_future_violations: usize,
}

impl ProviderAudit {
    fn observe_projection(&mut self, error: ProjectionError) {
        match error {
            ProjectionError::TooShort { .. } => self.projection_too_short += 1,
            ProjectionError::InvalidSeed { .. } => self.projection_invalid_seed += 1,
            ProjectionError::MissingCarriedCenter { .. } => {
                self.projection_missing_carried_center += 1
            }
            ProjectionError::InvalidLowerLeg { .. } => self.projection_other += 1,
        }
    }

    fn provider_complete(&self) -> bool {
        self.projection_too_short == 0
            && self.projection_invalid_seed == 0
            && self.projection_missing_carried_center == 0
            && self.projection_other == 0
    }
}

/// #97 身份键：与 [`NestEventIdentity`] 字段一一对应，用于对账链覆盖。
type IdentityKey = (u32, usize, (usize, usize));

#[derive(Debug, Default)]
struct YieldBook {
    candidates: BTreeMap<EventKey, usize>,
    divergences: BTreeMap<EventKey, usize>,
    terminal_confirmed: BTreeSet<EventKey>,
    cert_a: BTreeSet<String>,
    cert_b: BTreeSet<String>,
    cert_a_kind: BTreeMap<&'static str, usize>,
    cert_b_kind: BTreeMap<&'static str, usize>,
    d3_edges: usize,
    d3_violations: usize,
    /// #97: B 口径链实际吸收过的事件身份（任意 rung 位置）。
    covered_b: BTreeSet<IdentityKey>,
    /// #97: 走盘整块回退进料口的候选。
    intake_fallbacks: BTreeSet<EventKey>,
}

struct TerminalState {
    l0: ParseLayer,
    classification: classifier::Classification,
    tower: Vec<Rc<Vec<LeveledMove>>>,
    cache: classifier::TowerCache,
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p92_nest_replay_postruling <btc_1m_full.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P92_MAX_BARS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |value| value.min(loaded.bars.len()));

    println!(
        "P92_INPUT bars={} replay_bars={} first_date={} last_date={}",
        loaded.bars.len(),
        max_bars,
        loaded.first_date,
        loaded.last_date
    );
    println!(
        "P92_RULE cand=dir_and_comparable_and_extreme weak=divergence_stage interval_production=B typed=trend_or_consolidation d3=sidecar terminal=BspBits_confirm_side clock=first_prefix created_at=forbidden"
    );

    let mut audit = ProviderAudit::default();
    let terminal = run_terminal_pass(&loaded.bars[..max_bars], &config)?;
    let (hist, close_src) = terminal.cache.causal_series();
    let dif = terminal.cache.macd_dif();
    let terminal_events =
        collect_snapshot_candidates(&terminal.tower, max_bars - 1, hist, dif, close_src, &mut audit)?;
    let mut targets = BTreeMap::new();
    for event in terminal_events.iter().flatten() {
        targets.insert(EventKey::from(event), *event);
    }
    let (mut book, prefix_views, unresolved_targets) =
        run_targeted_prefix_pass(&loaded.bars[..max_bars], &config, &targets)?;
    audit.views += prefix_views;
    audit.snapshots += book.candidates.len();

    let l0 = terminal.l0;
    let classification = terminal.classification;
    let tower = terminal.tower;
    let cache = terminal.cache;
    let mut final_events = terminal_events;
    for events in &mut final_events {
        for event in events {
            let key = EventKey::from(&*event);
            event.judge_at = book
                .divergences
                .get(&key)
                .or_else(|| book.candidates.get(&key))
                .copied()
                .unwrap_or(max_bars - 1);
        }
    }
    observe_snapshot(
        &classification,
        final_events,
        max_bars - 1,
        &mut book,
        &mut audit,
    );
    let (full_classification, full_tower) = classifier::classify_with_tower(&l0, &config);
    let classification_diff = usize::from(full_classification != classification);
    let tower_diff = usize::from(full_tower != tower);
    let mut old_semantic_diff = tower_diff;
    let mut lifecycle_diff = 0usize;
    let levels = full_classification
        .levels
        .len()
        .max(classification.levels.len());
    for level in 0..levels {
        match (
            full_classification.levels.get(level),
            classification.levels.get(level),
        ) {
            (Some(full), Some(incremental)) => {
                old_semantic_diff += usize::from(full.moves != incremental.moves)
                    + usize::from(full.centers != incremental.centers)
                    + usize::from(full.bsp != incremental.bsp)
                    + usize::from(full.pan_div != incremental.pan_div);
                lifecycle_diff += usize::from(full.cp_ownership != incremental.cp_ownership);
            }
            _ => old_semantic_diff += 1,
        }
    }
    if classification_diff != 0 {
        let levels = full_classification
            .levels
            .len()
            .max(classification.levels.len());
        for level in 0..levels {
            let full = full_classification.levels.get(level);
            let incremental = classification.levels.get(level);
            let summary = match (full, incremental) {
                (Some(full), Some(incremental)) => format!(
                    "moves={} centers={} ownership={} bsp={} pan={}",
                    usize::from(full.moves != incremental.moves),
                    usize::from(full.centers != incremental.centers),
                    usize::from(full.cp_ownership != incremental.cp_ownership),
                    usize::from(full.bsp != incremental.bsp),
                    usize::from(full.pan_div != incremental.pan_div),
                ),
                _ => "missing_level=1".to_string(),
            };
            println!("P92_BIT_DIFF_LEVEL L{level} {summary}");
        }
    }
    let old_events =
        classifier::cand_delta_tower_cached(&l0, &classification, &tower, &config, &cache);
    let old_candidates = old_events
        .iter()
        .flatten()
        .filter(|event| event.cand_delta)
        .count();
    let old_terminal = old_events
        .iter()
        .enumerate()
        .map(|(level, events)| {
            events
                .iter()
                .filter(|event| {
                    event.cand_delta
                        && terminal_bits_old(
                            &classification,
                            level,
                            event.c_episode_start,
                            event.confirm_src,
                            event.side,
                            event.b_parent.map(|p| p.source_interval.0),
                        )
                        .is_some()
                })
                .count()
        })
        .sum::<usize>();
    let mut old_certificates = 0usize;
    for exec in 0..old_events.len() {
        for top in exec..old_events.len() {
            old_certificates += assemble_certificates_snapshot(&old_events, exec, top, |event| {
                terminal_bits_old(
                    &classification,
                    exec,
                    event.c_episode_start,
                    event.confirm_src,
                    event.side,
                    event.b_parent.map(|p| p.source_interval.0),
                )
            })
            .len();
        }
    }

    let trend_candidates = book
        .candidates
        .keys()
        .filter(|key| key.kind == NestDivergenceKind::Trend)
        .count();
    let pan_candidates = book.candidates.len() - trend_candidates;
    let trend_divergences = book
        .divergences
        .keys()
        .filter(|key| key.kind == NestDivergenceKind::Trend)
        .count();
    let pan_divergences = book.divergences.len() - trend_divergences;
    let d3_rate = if book.d3_edges == 0 {
        0.0
    } else {
        book.d3_violations as f64 / book.d3_edges as f64
    };

    println!(
        "P92_YIELD candidates={} trend_candidates={} pan_candidates={} divergence_confirmed={} trend_divergence={} pan_divergence={} terminal_confirmed={}",
        book.candidates.len(),
        trend_candidates,
        pan_candidates,
        book.divergences.len(),
        trend_divergences,
        pan_divergences,
        book.terminal_confirmed.len()
    );
    println!(
        "P92_CERT caliber_A={} A_trend={} A_pan={} A_mixed={} caliber_B={} B_trend={} B_pan={} B_mixed={}",
        book.cert_a.len(),
        book.cert_a_kind.get("trend").copied().unwrap_or(0),
        book.cert_a_kind.get("pan").copied().unwrap_or(0),
        book.cert_a_kind.get("mixed").copied().unwrap_or(0),
        book.cert_b.len(),
        book.cert_b_kind.get("trend").copied().unwrap_or(0),
        book.cert_b_kind.get("pan").copied().unwrap_or(0),
        book.cert_b_kind.get("mixed").copied().unwrap_or(0),
    );
    println!(
        "P92_D3 edges={} violations={} rate={:.9}",
        book.d3_edges, book.d3_violations, d3_rate
    );
    println!(
        "P92_BASELINE old_candidates={} old_terminal_confirmed={} old_certificates={} new_candidates={} new_terminal_confirmed={} new_B_certificates={}",
        old_candidates,
        old_terminal,
        old_certificates,
        book.candidates.len(),
        book.terminal_confirmed.len(),
        book.cert_b.len()
    );
    println!(
        "P92_PROVIDER snapshots={} views={} too_short={} invalid_seed={} missing_carried_center={} other={} unresolved_targets={} complete={}",
        audit.snapshots,
        audit.views,
        audit.projection_too_short,
        audit.projection_invalid_seed,
        audit.projection_missing_carried_center,
        audit.projection_other,
        unresolved_targets,
        audit.provider_complete() && unresolved_targets == 0
    );
    println!(
        "P92_SNAPSHOT future_violations={} created_at_reads=0 no_forward={}",
        audit.snapshot_future_violations,
        audit.snapshot_future_violations == 0
    );
    println!(
        "P92_BIT_EXACT old_path_diff={} tower_diff={} moves_centers_bsp_pan_diff={} lifecycle_cp_ownership_diff={} classification_total_diff={}",
        old_semantic_diff,
        tower_diff,
        old_semantic_diff.saturating_sub(tower_diff),
        lifecycle_diff,
        classification_diff
    );
    println!(
        "P92_R7 provider_complete={} definition_faithful=true snapshot_no_forward={} B_zero={}",
        audit.provider_complete() && unresolved_targets == 0,
        audit.snapshot_future_violations == 0,
        book.cert_b.is_empty()
    );
    // #97: 遗漏对账 —— 钟位可证的候选却从未被任何 B 链吸收，逐条枚举。
    let missed: Vec<&EventKey> = book
        .terminal_confirmed
        .iter()
        .filter(|key| !book.covered_b.contains(&(key.level, key.turn_source, key.interval_b)))
        .collect();
    println!(
        "P92_MISSED terminal_confirmed={} covered_b={} intake_fallback_events={} missed={}",
        book.terminal_confirmed.len(),
        book.covered_b.len(),
        book.intake_fallbacks.len(),
        missed.len()
    );
    for key in &missed {
        println!(
            "P92_MISSED_EVENT level={} kind={:?} short={} seg_a={:?} interval_b={:?} interval_a={:?} turn_source={} intake_fallback={}",
            key.level,
            key.kind,
            key.short,
            key.seg_a,
            key.interval_b,
            key.interval_a,
            key.turn_source,
            book.intake_fallbacks.contains(*key)
        );
    }
    dump_flush();
    Ok(())
}

fn signal_signature(
    classification: &classifier::Classification,
) -> Vec<(usize, usize, usize, usize)> {
    classification
        .levels
        .iter()
        .map(|level| {
            (
                level.bsp.len(),
                level
                    .bsp
                    .last()
                    .map_or(usize::MAX, |point| point.source_index),
                level.pan_div.len(),
                level
                    .pan_div
                    .last()
                    .map_or(usize::MAX, |cert| cert.source_index),
            )
        })
        .collect()
}

fn run_terminal_pass(bars: &[Bar], config: &ThetaConfig) -> Result<TerminalState, String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = classifier::TowerCache::new();
    let mut terminal = None;
    let started = Instant::now();
    for (index, bar) in bars.iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let (classification, tower) =
            classifier::classify_with_tower_incremental(&l0, config, &mut cache);
        if index > 0 && index % 500_000 == 0 {
            eprintln!(
                "P92_TERMINAL_PROGRESS bar={index}/{} elapsed={:.1}s",
                bars.len(),
                started.elapsed().as_secs_f64()
            );
        }
        if index + 1 == bars.len() {
            terminal = Some((l0, classification, tower));
        }
    }
    let (l0, classification, tower) = terminal.ok_or("空 replay")?;
    Ok(TerminalState {
        l0,
        classification,
        tower,
        cache,
    })
}

fn run_targeted_prefix_pass(
    bars: &[Bar],
    config: &ThetaConfig,
    targets: &BTreeMap<EventKey, NestCandidateEvent>,
) -> Result<(YieldBook, usize, usize), String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = classifier::TowerCache::new();
    let mut book = YieldBook::default();
    let mut pending: BTreeSet<EventKey> = targets.keys().cloned().collect();
    let mut views = 0usize;
    let mut last_trigger = None;
    let started = Instant::now();
    // #103 侧信道：P92_CKPT=<K> 时每 K bars 做一次全量快照装配并 dump（只写不判）。
    let ckpt_every: usize = std::env::var("P92_CKPT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    for (index, bar) in bars.iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let (classification, tower) =
            classifier::classify_with_tower_incremental(&l0, config, &mut cache);
        let trigger = (cache.forest_epoch(), signal_signature(&classification));
        if last_trigger.as_ref() != Some(&trigger) && !pending.is_empty() {
            let (hist, close_src) = cache.causal_series();
            let dif = cache.macd_dif();
            let (events, used_views) =
                collect_target_candidates(&tower, index, hist, dif, close_src, targets, &pending)?;
            views += used_views;
            for event in events {
                let key = EventKey::from(&event);
                if !pending.contains(&key) {
                    continue;
                }
                book.candidates.entry(key.clone()).or_insert(index);
                let target = targets.get(&key).expect("pending target");
                if event.divergence_confirmed {
                    book.divergences.entry(key.clone()).or_insert(index);
                }
                if event.divergence_confirmed == target.divergence_confirmed {
                    pending.remove(&key);
                }
            }
            last_trigger = Some(trigger);
        }
        if ckpt_every > 0 && index > 0 && index % ckpt_every == 0 {
            let (hist, close_src) = cache.causal_series();
            let dif = cache.macd_dif();
            let mut ckpt_audit = ProviderAudit::default();
            match collect_snapshot_candidates(&tower, index, hist, dif, close_src, &mut ckpt_audit) {
                Ok(by_level) => checkpoint_certificates(&by_level, &classification, index),
                Err(error) => {
                    dump_line(format_args!("CKPT_ERR as_of={index} err={error}"));
                }
            }
        }
        if index > 0 && index % 500_000 == 0 {
            eprintln!(
                "P92_PREFIX_PROGRESS bar={index}/{} elapsed={:.1}s pending={}/{} views={views}",
                bars.len(),
                started.elapsed().as_secs_f64(),
                pending.len(),
                targets.len()
            );
        }
    }
    Ok((book, views, pending.len()))
}

fn collect_target_candidates(
    tower: &[Rc<Vec<LeveledMove>>],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    targets: &BTreeMap<EventKey, NestCandidateEvent>,
    pending: &BTreeSet<EventKey>,
) -> Result<(Vec<NestCandidateEvent>, usize), String> {
    let mut runs_by_level: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for key in pending {
        let Some(event) = targets.get(key) else { continue };
        // snapshot 无前视的结构下，turn_source 尚未到达时该对象不可能成为 Cand。
        // 提前投影这些终态目标只增加扫描量，不可能改变首次可证钟。
        if event.turn_source <= as_of {
            runs_by_level
                .entry(event.level as usize)
                .or_default()
                .insert(event.provider_window.0);
        }
    }
    let mut out = Vec::new();
    let mut views = 0usize;
    for (level, run_sources) in runs_by_level {
        if level == 0 || level >= tower.len() {
            continue;
        }
        let windows = &tower[level];
        // 同一触发快照内，每级 lower legs 与合法 run 分区都只构造一次。
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} targeted lower legs 失败: {error:?}"))?;
        let mut run_ranges = BTreeMap::new();
        let mut run_start = None;
        let mut run_source = None;
        for index in 0..=windows.len() {
            let seed_start = (index < windows.len())
                .then(|| project_extended_windows_carried_only(std::slice::from_ref(&windows[index])).ok())
                .flatten()
                .and_then(|projection| projection.seeds.first().map(|seed| seed.start_index));
            match (run_start, seed_start) {
                (None, Some(source)) => {
                    run_start = Some(index);
                    run_source = Some(source);
                }
                (Some(start), None) => {
                    run_ranges.insert(run_source.expect("合法 run 有 source"), (start, index));
                    run_start = None;
                    run_source = None;
                }
                _ => {}
            }
        }
        for run_source_start in run_sources {
            let Some(&(start, end)) = run_ranges.get(&run_source_start) else { continue };
            let projection = project_extended_windows_carried_only(&windows[start..end])
                .map_err(|error| format!("L{level} targeted projection 失败: {error:?}"))?;
            let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
            let blocks = decompose::decompose(&centers);
            let query = LevelViewQuery {
                level: level as u32,
                coordinate_window: CoordinateWindow {
                    start: projection.seeds.first().expect("nonempty").start_index,
                    end: projection.seeds.last().expect("nonempty").end_index,
                },
                as_of,
                version: C2VersionTuple::auto_pairing(),
            };
            let view = assemble_level_view(
                C2LevelViewConfig { enabled: true },
                query,
                LevelViewMaterial {
                    projection: ProjectionMaterial::ExactThree(&projection),
                    move_blocks: &blocks,
                    lower_legs: &lower,
                    hist,
                    dif,
                    close_src,
                },
            )
            .map_err(|error| format!("L{level} targeted C2 assemble 失败: {error:?}"))?;
            views += 1;
            out.extend(
                provide_nest_candidate_events(
                    level as u32,
                    &projection,
                    &blocks,
                    &lower,
                    &view,
                    hist,
                    dif,
                    close_src,
                )
                .into_iter()
                .filter(|event| pending.contains(&EventKey::from(event))),
            );
        }
    }
    Ok((out, views))
}

fn collect_snapshot_candidates(
    tower: &[Rc<Vec<LeveledMove>>],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    audit: &mut ProviderAudit,
) -> Result<Vec<Vec<NestCandidateEvent>>, String> {
    audit.snapshots += 1;
    let mut by_level = vec![Vec::new(); tower.len()];
    for level in 1..tower.len() {
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
        let windows = &tower[level];
        let mut run_start = None;
        for index in 0..=windows.len() {
            let valid = if index < windows.len() {
                match project_extended_windows_carried_only(std::slice::from_ref(&windows[index])) {
                    Ok(_) => true,
                    Err(error) => {
                        audit.observe_projection(error);
                        false
                    }
                }
            } else {
                false
            };
            match (run_start, valid) {
                (None, true) => run_start = Some(index),
                (Some(start), false) => {
                    let projection = project_extended_windows_carried_only(&windows[start..index])
                        .map_err(|error| format!("L{level} run projection 失败: {error:?}"))?;
                    let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
                    let blocks = decompose::decompose(&centers);
                    let start_source = projection.seeds.first().expect("nonempty run").start_index;
                    let end_source = projection.seeds.last().expect("nonempty run").end_index;
                    let query = LevelViewQuery {
                        level: level as u32,
                        coordinate_window: CoordinateWindow {
                            start: start_source,
                            end: end_source,
                        },
                        as_of,
                        version: C2VersionTuple::auto_pairing(),
                    };
                    let view = assemble_level_view(
                        C2LevelViewConfig { enabled: true },
                        query,
                        LevelViewMaterial {
                            projection: ProjectionMaterial::ExactThree(&projection),
                            move_blocks: &blocks,
                            lower_legs: &lower,
                            hist,
                            dif,
                            close_src,
                        },
                    )
                    .map_err(|error| format!("L{level} C2 assemble 失败: {error:?}"))?;
                    audit.views += 1;
                    by_level[level].extend(provide_nest_candidate_events(
                        level as u32,
                        &projection,
                        &blocks,
                        &lower,
                        &view,
                        hist,
                        dif,
                        close_src,
                    ));
                    run_start = None;
                }
                _ => {}
            }
        }
    }
    for events in &mut by_level {
        events.sort_by_key(|event| EventKey::from(&*event));
        events.dedup_by(|left, right| EventKey::from(&*left) == EventKey::from(&*right));
    }
    Ok(by_level)
}

fn observe_snapshot(
    classification: &classifier::Classification,
    mut current: Vec<Vec<NestCandidateEvent>>,
    as_of: usize,
    book: &mut YieldBook,
    audit: &mut ProviderAudit,
) {
    for events in &mut current {
        for event in events {
            if event.interval_a.1 > as_of || event.interval_b.1 > as_of || event.turn_source > as_of
            {
                audit.snapshot_future_violations += 1;
            }
            let key = EventKey::from(&*event);
            if event.intake_fallback && book.intake_fallbacks.insert(key.clone()) {
                dump_line(format_args!(
                    "FALLBACK as_of={} level={} kind={:?} side={:?} turn_source={} seg_a={:?} interval_a={:?} interval_b={:?} divergence_confirmed={} judge_at={}",
                    as_of,
                    event.level,
                    event.kind,
                    event.side,
                    event.turn_source,
                    event.seg_a,
                    event.interval_a,
                    event.interval_b,
                    event.divergence_confirmed,
                    event.judge_at,
                ));
            }
            let candidate_at = *book.candidates.entry(key.clone()).or_insert(as_of);
            if event.interval_a.1 > candidate_at
                || event.interval_b.1 > candidate_at
                || event.turn_source > candidate_at
            {
                audit.snapshot_future_violations += 1;
            }
            if event.divergence_confirmed {
                let first = *book.divergences.entry(key.clone()).or_insert(as_of);
                event.judge_at = first;
                if terminal_bits_new(classification, event).is_some() {
                    book.terminal_confirmed.insert(key);
                }
            } else {
                event.judge_at = *book.candidates.get(&key).expect("inserted");
            }
        }
    }
    for exec in 1..current.len() {
        for top in exec..current.len() {
            observe_certificates(
                &current,
                classification,
                exec,
                top,
                as_of,
                NestIntervalCaliber::A,
                &mut book.cert_a,
                &mut book.cert_a_kind,
                &mut book.d3_edges,
                &mut book.d3_violations,
                false,
                None,
            );
            observe_certificates(
                &current,
                classification,
                exec,
                top,
                as_of,
                NestIntervalCaliber::B,
                &mut book.cert_b,
                &mut book.cert_b_kind,
                &mut book.d3_edges,
                &mut book.d3_violations,
                true,
                Some(&mut book.covered_b),
            );
        }
    }
    // p118 关④ TURN_CLASS 侧信道：039:34 defer 孤儿发射（只写不判）——c 破极值未确认
    // Trend 事件未被本快照任何 B 链消费（covered_b 在上方装配环后对本快照完备）。
    // 事件层在册的未确认 Trend 事件即「c 破极值」：extreme 预滤击杀的未破极值形态属
    // 037:20 否则条款域（盘整/二类点通道），明示排除在本分支之外（施工图 §1.4/§2.5）。
    for events in &current {
        for event in events {
            let covered = book
                .covered_b
                .contains(&(event.level, event.turn_source, event.interval_b));
            if is_defer_orphan_event(event, covered) {
                dump_line(format_args!(
                    "TURN_CLASS ids={}:{}:{}-{} class=DeferOrphan confirmed_vec=0",
                    event.level,
                    event.turn_source,
                    event.interval_b.0,
                    event.interval_b.1,
                ));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn observe_certificates(
    events: &[Vec<NestCandidateEvent>],
    classification: &classifier::Classification,
    exec: usize,
    top: usize,
    as_of: usize,
    caliber: NestIntervalCaliber,
    seen: &mut BTreeSet<String>,
    kinds: &mut BTreeMap<&'static str, usize>,
    d3_edges: &mut usize,
    d3_violations: &mut usize,
    count_d3: bool,
    mut covered: Option<&mut BTreeSet<IdentityKey>>,
) {
    let certificates = assemble_typed_certificates(events, exec, top, caliber, |event| {
        terminal_bits_new(classification, event)
    });
    for certificate in certificates {
        if let Some(covered) = covered.as_deref_mut() {
            for identity in certificate.identities() {
                covered.insert((identity.level, identity.turn_source, identity.interval_b));
            }
        }
        let key = certificate_key(exec, top, &certificate);
        if seen.insert(key) {
            let bucket = certificate_kind(&certificate);
            *kinds.entry(bucket).or_default() += 1;
            if count_d3 {
                let (edges, violations) = certificate.d3_descent_stats();
                *d3_edges += edges;
                *d3_violations += violations;
            }
            // #98/#99 侧信道：身份向量（高→低，含基例）为 A/B 跨口径对账主键；
            // judge_at 向量供 D3 逐边离线复算。只写不判。
            let ids = certificate
                .identities()
                .iter()
                .map(|id| format!("{}:{}:{}-{}", id.level, id.turn_source, id.interval_b.0, id.interval_b.1))
                .collect::<Vec<_>>()
                .join("|");
            let kinds_str = certificate
                .kinds()
                .iter()
                .map(|kind| format!("{kind:?}"))
                .collect::<Vec<_>>()
                .join(",");
            let clocks = certificate
                .judge_at()
                .iter()
                .map(|clock| clock.to_string())
                .collect::<Vec<_>>()
                .join(",");
            dump_line(format_args!(
                "CERT caliber={:?} exec={} top={} as_of={} side={:?} bucket={} kinds={} judge_at={} ids={}",
                certificate.caliber(),
                exec,
                top,
                as_of,
                certificate.certificate().side(),
                bucket,
                kinds_str,
                clocks,
                ids,
            ));
            // p118 关④ TURN_CLASS 侧信道（只写不判；行契约见 turn_class.rs 模块头）：
            // 与 CERT 行同主键（ids）配对——CERT 行格式零改，本行为独立新增行。
            // class = NestTurnClass 四类之一（DeferOrphan 不走此处，见 observe_snapshot 尾部）。
            let turn_class = classify_certificate_turn(&certificate, classification);
            let confirmed_vec = certificate
                .confirmed()
                .iter()
                .map(|flag| if *flag { '1' } else { '0' })
                .collect::<String>();
            let (class_name, evidence_str) = turn_class_dump(&turn_class);
            dump_line(format_args!(
                "TURN_CLASS caliber={:?} exec={} top={} as_of={} ids={} class={} confirmed_vec={}{}",
                certificate.caliber(),
                exec,
                top,
                as_of,
                ids,
                class_name,
                confirmed_vec,
                evidence_str,
            ));
        }
    }
}

/// #103 侧信道：检查点全量证书导出（只写不判，不触碰 book/seen 等主路径状态）。
/// 与 observe_certificates 的差异：seen 为检查点局部——每个检查点导出该时刻完整在场集合，
/// 供离线做存在性 diff（主键 ids 身份向量）；judge_at 取快照原值，不参与对账。
fn checkpoint_certificates(
    events: &[Vec<NestCandidateEvent>],
    classification: &classifier::Classification,
    as_of: usize,
) {
    for caliber in [NestIntervalCaliber::A, NestIntervalCaliber::B] {
        let mut seen = BTreeSet::new();
        let mut total = 0usize;
        for exec in 1..events.len() {
            for top in exec..events.len() {
                let certificates =
                    assemble_typed_certificates(events, exec, top, caliber, |event| {
                        terminal_bits_new(classification, event)
                    });
                for certificate in certificates {
                    let key = certificate_key(exec, top, &certificate);
                    if !seen.insert(key) {
                        continue;
                    }
                    total += 1;
                    let ids = certificate
                        .identities()
                        .iter()
                        .map(|id| {
                            format!(
                                "{}:{}:{}-{}",
                                id.level, id.turn_source, id.interval_b.0, id.interval_b.1
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("|");
                    dump_line(format_args!(
                        "CKPT caliber={:?} as_of={} exec={} top={} side={:?} bucket={} ids={}",
                        certificate.caliber(),
                        as_of,
                        exec,
                        top,
                        certificate.certificate().side(),
                        certificate_kind(&certificate),
                        ids,
                    ));
                }
            }
        }
        dump_line(format_args!(
            "CKPT_STATS caliber={caliber:?} as_of={as_of} certs={total}"
        ));
    }
}

/// p118 关④：`NestTurnClass` → TURN_CLASS 行的 (class 名, evidence 片段) 投影。
/// 行契约见 `turn_class.rs` 模块头；Candidate 的 evidence 只含结构坐标（044:30 无确认语义）。
fn turn_class_dump(class: &NestTurnClass) -> (&'static str, String) {
    match class {
        NestTurnClass::NestedConfirmed => ("NestedConfirmed", String::new()),
        NestTurnClass::ExecEvidenceOnly => ("ExecEvidenceOnly", String::new()),
        NestTurnClass::XiaozhuandaCandidate { evidence } => (
            "XiaozhuandaCandidate",
            format!(
                " evidence=c_prime=({},{},{},{});third={};second={}",
                evidence.c_prime.zg,
                evidence.c_prime.zd,
                evidence.c_prime.start_index,
                evidence.c_prime.end_index,
                evidence.third_src,
                evidence
                    .second_class
                    .map_or_else(|| "-".to_string(), |src| src.to_string()),
            ),
        ),
        // DeferOrphan 由 observe_snapshot 尾部单独发射（事件层条目，非证书行）。
        NestTurnClass::DeferOrphan { .. } => ("DeferOrphan", String::new()),
    }
}

fn certificate_kind(certificate: &TypedNestCertificate) -> &'static str {
    let trend = certificate
        .kinds()
        .iter()
        .any(|kind| *kind == NestDivergenceKind::Trend);
    let pan = certificate
        .kinds()
        .iter()
        .any(|kind| *kind == NestDivergenceKind::Consolidation);
    match (trend, pan) {
        (true, false) => "trend",
        (false, true) => "pan",
        _ => "mixed",
    }
}

fn certificate_key(exec: usize, top: usize, certificate: &TypedNestCertificate) -> String {
    let cert = certificate.certificate();
    let rungs = cert
        .rungs()
        .iter()
        .map(|rung| (rung.interval(), rung.child_interval()))
        .collect::<Vec<_>>();
    // #97: 身份标签参与去重键，同结构不同 provider 身份的链不互相吞并。
    let ids = certificate
        .identities()
        .iter()
        .map(|id| (id.level, id.turn_source, id.interval_b))
        .collect::<Vec<_>>();
    format!(
        "{exec}:{top}:{:?}:{:?}:{:?}:{:?}:{:?}:{ids:?}",
        certificate.caliber(),
        cert.side(),
        cert.base_interval(),
        rungs,
        certificate.kinds()
    )
}

/// ★p117 T1 终端背书生产口径常数（bsp-terminal-endorsement-ruling-20260718 裁决2）：
/// C-b = `[c_start, t*]` 窗口最早 confirm_side 点。C-a（`TerminalMatch::Exact`）保留为
/// 敏感性对照口径——验收报告落 C-a/C-b 对照读数时切换本常数一处（预测差 = 5 延迟案）。

/// #218 面 B 研究 bin 锚供给说明（诚实，090）：owner 判同已换两族锚（一/三类核心区间
/// 带判同经账本 `centers` 全功能；二类一类点身份锚判同需 T1 oracle + 事件锚账本）。
/// 本 bin 是归档研究/审计工具，未接事件锚账本——二类判同锚不可解 = 诚实判负（与
/// 生产 gate 全接线读数有别，面 B 注册项；一/三类判同不受影响）。
fn bin_anchor_ctx() -> newchan_rust::theta_v0::classifier::nest::OwnerAnchorCtx<'static> {
    fn never(_: usize) -> Option<(newchan_rust::theta_v0::types::Tick, usize)> {
        None
    }
    newchan_rust::theta_v0::classifier::nest::OwnerAnchorCtx { anchor_at: &never, event_anchor: (None, None) }
}

const TERMINAL_MATCH: TerminalMatch = TerminalMatch::CWindow;

/// 终端背书查法（生产）：委托 lib 单一来源 `nest::terminal_bits_at_event`——账本级别移位
/// `levels[ℓ-1].bsp` + 口径常数 [`TERMINAL_MATCH`]。旧 `levels[ℓ]` 位格等式查法已删除
///（单一来源纪律，不得保留为 fallback；裁定不回滚条款）。
fn terminal_bits_new(
    classification: &classifier::Classification,
    event: &NestCandidateEvent,
) -> Option<BspBits> {
    // 关③ P3：lib 返回形状扩为 `TerminalEndorsement`（bits + owner start_index）——
    // 生产装配消费 bits 层；owner 两维构成归收紧后重放审计读数，非本 bin 职责。
    terminal_bits_at_event(classification, event, TERMINAL_MATCH, &bin_anchor_ctx()).map(|t| t.bits)
}

/// 旧事件路径（`CandDeltaEvent`，P1 基线对账/P92_BASELINE 类审计）的终端查法：同一级别
/// 移位 + C-b 口径平移——账本 = `levels[ℓ-1].bsp`；窗口 = `[c_episode_start, confirm_src]`
///（`c_episode_start` 即 `departure_move_c_start` 当前 episode 首腿，与新事件路径
/// `interval_b.0` 同义）。委托 lib 单一来源 `nest::terminal_bits_in_book`，同口径平移并
/// 标注（S1a 图 H2）；位格等式不得恢复。
fn terminal_bits_old(
    classification: &classifier::Classification,
    level: usize,
    c_start: usize,
    source: usize,
    side: Side,
    b_center_start: Option<usize>,
) -> Option<BspBits> {
    let book_level = classification.levels.get(event_bsp_book_level(level as u32)?)?;
    let book = &book_level.bsp;
    // 关③ P3 平移：旧事件 = Cand^δ 趋势族线（pan_div_diag 为 cand_delta=false 纯诊断，
    // 结构性不入终端查询）⟹ kind=Trend；B 身份 = 事件自带 `b_parent.source_interval.0`
    //（ParentCenterIdentity 已携 B start_index 快照，单一来源，无第二查法）。
    // #218 面 B：一/三类判同的 B 带由同层 `centers` 查出（b_center_start 只当查找键）。
    terminal_bits_in_book(
        book, &book_level.centers, c_start, source, side, NestDivergenceKind::Trend,
        b_center_start, TERMINAL_MATCH, &bin_anchor_ctx(),
    )
    .map(|t| t.bits)
}

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
    first_date: String,
    last_date: String,
}

fn load_bars(path: &Path, tick_size: f64) -> Result<LoadedBars, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?;
    let raw: BarsJson = serde_json::from_str(&text)
        .map_err(|error| format!("{} JSON 解析失败: {error}", path.display()))?;
    let n = raw.closes.len();
    for (name, len) in [
        ("opens", raw.opens.len()),
        ("highs", raw.highs.len()),
        ("lows", raw.lows.len()),
        ("volumes", raw.volumes.len()),
        ("dates", raw.dates.len()),
    ] {
        if len != n {
            return Err(format!("列长度不一致: {name}={len}, closes={n}"));
        }
    }
    let bars = (0..n)
        .map(|index| {
            let open = raw.opens[index];
            let high = raw.highs[index];
            let low = raw.lows[index];
            let close = raw.closes[index];
            let volume = raw.volumes[index];
            Bar {
                source_index: index,
                timestamp: date_to_timestamp(&raw.dates[index]),
                open: quantize(open, tick_size),
                high: quantize(high, tick_size),
                low: quantize(low, tick_size),
                close: quantize(close, tick_size),
                volume: volume as i64,
                untradable: high < open.max(close).max(low)
                    || low > open.min(close).min(high)
                    || open <= 0.0
                    || high <= 0.0
                    || low <= 0.0
                    || close <= 0.0
                    || volume <= 0.0,
            }
        })
        .collect();
    Ok(LoadedBars {
        bars,
        first_date: raw.dates.first().cloned().unwrap_or_default(),
        last_date: raw.dates.last().cloned().unwrap_or_default(),
    })
}

#[derive(Debug, Deserialize)]
struct BarsJson {
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    volumes: Vec<f64>,
    dates: Vec<String>,
}

fn date_to_timestamp(date: &str) -> Timestamp {
    let mut digits = String::with_capacity(14);
    for ch in date.chars().take_while(|value| *value != '+') {
        if ch.is_ascii_digit() {
            digits.push(ch);
            if digits.len() == 14 {
                break;
            }
        }
    }
    digits.parse().unwrap_or(0)
}
