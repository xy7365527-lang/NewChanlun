//! task #86：D1 滑动 seed 与 D2 失败 episode 回退的只读反事实测量。
//!
//! 不修改生产对象或语义；输入与生产塔均只读。用法：
//! `p86_anchor_hypothesis <btc_1m_full.json>`

use newchan_rust::theta_v0::classifier::center::{
    center_from_segments, center_from_window, dir_alternates, UnitRange,
};
use newchan_rust::theta_v0::classifier::classify_with_tower;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::descend::RMove;
use newchan_rust::theta_v0::classifier::divergence::{
    compute_macd, departure_move_c_start, locate_departure_move_a, segment_macd_area,
    segments_diverge, self_anchors,
};
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows, C2LevelViewConfig,
    C2VersionTuple, CompletionStatus, CoordinateWindow, ExactThreeProjection, ExactThreeSeed,
    LevelViewMaterial, LevelViewQuery, LowerLeg, PendingReason, ProjectionError,
    ProjectionMaterial, ProviderVersion, SeedCoreProvenance,
};
use newchan_rust::theta_v0::classifier::recursive_tower::{
    compose_level, map_src_to_close_idx, project_to_units, ElementId, LeveledMove,
};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{quantize, Bar, Center, Direction, Segment, Timestamp};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const EXPECTED_D1: [usize; 5] = [165, 43, 7, 0, 0];
const EXPECTED_MISSING: [usize; 5] = [0, 17, 2, 1, 1];
const EXPECTED_OTHER_VALID: [usize; 5] = [29, 5, 2, 0, 0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectMode {
    Production,
    Sliding,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SeedKey {
    start: usize,
    end: usize,
    zd: i64,
    zg: i64,
}

impl From<&ExactThreeSeed> for SeedKey {
    fn from(value: &ExactThreeSeed) -> Self {
        Self {
            start: value.start_index,
            end: value.end_index,
            zd: value.center.zd,
            zg: value.center.zg,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct LevelStats {
    windows: usize,
    invalid_seed: usize,
    seeds: usize,
    centers: usize,
    trend_blocks: usize,
    pairs: usize,
    missing_pair: usize,
    recovered_windows: usize,
    bit_same_checks: usize,
    seed_keys: Vec<SeedKey>,
}

#[derive(Debug, Clone)]
struct AffectedWindow {
    level: usize,
    id: ElementId,
    window_start: usize,
    window_end: usize,
    sub_count: usize,
    valid_starts: Vec<usize>,
    chosen_start: usize,
    chosen_source_start: usize,
    center: Center,
}

#[derive(Debug, Clone)]
struct MissingDetail {
    level: usize,
    run: usize,
    block_start: usize,
    block_end: usize,
    direction: Direction,
    move_start: usize,
    move_end: usize,
    prev_center: Center,
    last_center: Center,
    a_window_segments: usize,
    a_window_same_dir: usize,
    boundary: Option<usize>,
    after_boundary: usize,
    counterfactual_a: Option<(usize, usize)>,
    c: Option<(usize, usize)>,
}

#[derive(Debug, Clone)]
struct CounterfactualPair {
    level: usize,
    run: usize,
    block_start: usize,
    block_end: usize,
    direction: Direction,
    move_start: usize,
    move_end: usize,
    prev_center_end: usize,
    last_center_end: usize,
    a_window_segments: usize,
    boundary: usize,
    seg_a: (usize, usize),
    seg_c: (usize, usize),
    area_a: f64,
    area_c: f64,
    diverges: bool,
}

#[derive(Debug, Clone, Default)]
struct Replay {
    stats: BTreeMap<usize, LevelStats>,
    raw: BTreeMap<usize, Vec<LeveledMove>>,
}

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
    first_date: String,
    last_date: String,
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p86_anchor_hypothesis <btc_1m_full.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }

    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.len() != 4_613_599 {
        return Err(format!(
            "守恒失败：原始 bars={}，预期 4613599",
            loaded.bars.len()
        ));
    }
    let as_of = loaded.bars.len() - 1;
    if as_of != 4_613_598 {
        return Err(format!("守恒失败：as_of={as_of}，预期 4613598"));
    }
    let layer = parse_layer(&loaded.bars, &config);
    let (classification, tower) = classify_with_tower(&layer, &config);
    if tower.len() != 6 {
        return Err(format!("守恒失败：tower_levels={}，预期 6", tower.len()));
    }
    let closes: Vec<f64> = layer.merged_bars.iter().map(|b| b.close as f64).collect();
    let close_src: Vec<usize> = layer.merged_bars.iter().map(|b| b.source_index).collect();
    let hist = compute_macd(&closes, &config.macd).hist;

    let mut baseline = BTreeMap::new();
    let mut missing = Vec::new();
    for level in 1..tower.len() {
        let raw = vec![tower[level].as_ref().clone()];
        let lower = tower[level - 1].as_ref();
        let (stats, _) = measure_level(
            level,
            &raw,
            lower,
            ProjectMode::Production,
            as_of,
            &hist,
            &close_src,
            Some(&mut missing),
        )?;
        baseline.insert(level, stats);
    }
    let affected = collect_affected(&tower)?;
    validate_baseline(&baseline, &affected, &missing)?;

    let main_variant = recursive_replay(
        1,
        tower[1].as_ref().clone(),
        tower[0].as_ref().clone(),
        ProjectMode::Sliding,
        tower.len() - 1,
        as_of,
        &hist,
        &close_src,
    )?;

    let mut local_replays = BTreeMap::new();
    for start_level in 1..=3 {
        let prod = recursive_replay(
            start_level,
            tower[start_level].as_ref().clone(),
            tower[start_level - 1].as_ref().clone(),
            ProjectMode::Production,
            tower.len() - 1,
            as_of,
            &hist,
            &close_src,
        )?;
        let slide = recursive_replay(
            start_level,
            tower[start_level].as_ref().clone(),
            tower[start_level - 1].as_ref().clone(),
            ProjectMode::Sliding,
            tower.len() - 1,
            as_of,
            &hist,
            &close_src,
        )?;
        local_replays.insert(start_level, (prod, slide));
    }

    let counterfactuals = build_counterfactuals(&missing, &hist, &close_src)?;
    if counterfactuals.len() != 7 {
        return Err(format!(
            "D2 守恒失败：反事实候选={}，预期 7",
            counterfactuals.len()
        ));
    }

    println!(
        "P86_CONSERVATION status=PASS bars={} merged={} as_of={} first_date={} last_date={} tower_levels={} classification_levels={} d1_invalid_seed=215 d1_other_windows=36 d1_other_starts=46 missing_pair=21 fallback_candidates=7",
        loaded.bars.len(),
        layer.merged_bars.len(),
        as_of,
        loaded.first_date,
        loaded.last_date,
        tower.len(),
        classification.levels.len(),
    );
    println!(
        "P86_HYPOTHESES D1=sliding_min_legal_triplet D2=ignore_last_reentry_use_full_A_window_first_to_last_same_direction_anchor status=COUNTERFACTUAL_ONLY"
    );

    let mut total_new = 0;
    let mut per_level_new = Vec::new();
    for level in 1..tower.len() {
        let prod = baseline.get(&level).expect("baseline level");
        let var = main_variant.stats.get(&level).cloned().unwrap_or_default();
        let (seed_same, seed_changed, seed_new) = multiset_diff(&prod.seed_keys, &var.seed_keys);
        total_new += seed_new;
        per_level_new.push(format!("L{level}:{seed_new}"));
        let prod_inputs = tower[level]
            .iter()
            .map(window_signature)
            .collect::<Vec<_>>();
        let var_inputs = main_variant
            .raw
            .get(&level)
            .map(|v| v.iter().map(window_signature).collect::<Vec<_>>())
            .unwrap_or_default();
        let (input_same, input_changed, input_new) = multiset_diff(&prod_inputs, &var_inputs);
        println!(
            "P86_LEVEL L{level} baseline_windows={} variant_windows={} baseline_invalid={} variant_invalid={} baseline_seeds={} variant_seeds={} baseline_centers={} variant_centers={} baseline_trend_blocks={} variant_trend_blocks={} baseline_pairs={} variant_pairs={} baseline_missing={} variant_missing={} seed_same={} seed_changed={} seed_new={} input_same={} input_changed={} input_new={} recovered_windows={} drift_scope={}",
            prod.windows,
            var.windows,
            prod.invalid_seed,
            var.invalid_seed,
            prod.seeds,
            var.seeds,
            prod.centers,
            var.centers,
            prod.trend_blocks,
            var.trend_blocks,
            prod.pairs,
            var.pairs,
            prod.missing_pair,
            var.missing_pair,
            seed_same,
            seed_changed,
            seed_new,
            input_same,
            input_changed,
            input_new,
            var.recovered_windows,
            if level == 1 { "direct" } else { "recursive" },
        );
    }
    println!(
        "P86_NEW_SEEDS total={} distribution={}",
        total_new,
        per_level_new.join(",")
    );
    println!(
        "P86_DIRECT_RECOVERED windows=36 legal_starts=46 distribution=L1:29,L2:5,L3:2,L4:0,L5:0"
    );

    for (index, detail) in affected.iter().enumerate() {
        let (prod, slide) = local_replays
            .get(&detail.level)
            .expect("affected levels are L1-L3");
        let cascade = cascade_levels(detail, prod, slide, tower.len() - 1);
        let starts = detail
            .valid_starts
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "P86_A index={} level=L{} id={}:{} window={}:{} sub_moves={} legal_offsets={} chosen_offset={} chosen_source_start={} new_seed={}:{} new_zd={} new_zg={} upper_cascade={} cascade_levels={}",
            index + 1,
            detail.level,
            detail.id.level,
            detail.id.ordinal,
            detail.window_start,
            detail.window_end,
            detail.sub_count,
            starts,
            detail.chosen_start,
            detail.chosen_source_start,
            detail.center.start_index,
            detail.center.end_index,
            detail.center.zd,
            detail.center.zg,
            !cascade.is_empty(),
            if cascade.is_empty() {
                "-".to_string()
            } else {
                cascade
                    .iter()
                    .map(|v| format!("L{v}"))
                    .collect::<Vec<_>>()
                    .join(",")
            },
        );
    }

    for (index, pair) in counterfactuals.iter().enumerate() {
        println!(
            "P86_B index={} level=L{} run={} block={}:{} direction={:?} move={}:{} prev_center_end={} last_center_end={} a_window_segments={} reentry_boundary={} pair_a={}:{} pair_c={}:{} area_a={:.12} area_c={:.12} would_diverge={} violation=episode_no_bridge+completed_departure status=COUNTERFACTUAL_ONLY",
            index + 1,
            pair.level,
            pair.run,
            pair.block_start,
            pair.block_end,
            pair.direction,
            pair.move_start,
            pair.move_end,
            pair.prev_center_end,
            pair.last_center_end,
            pair.a_window_segments,
            pair.boundary,
            pair.seg_a.0,
            pair.seg_a.1,
            pair.seg_c.0,
            pair.seg_c.1,
            pair.area_a,
            pair.area_c,
            pair.diverges,
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn recursive_replay(
    start_level: usize,
    initial: Vec<LeveledMove>,
    initial_lower: Vec<LeveledMove>,
    mode: ProjectMode,
    max_level: usize,
    as_of: usize,
    hist: &[f64],
    close_src: &[usize],
) -> Result<Replay, String> {
    let mut replay = Replay::default();
    let mut raw_runs = vec![initial];
    let mut lower = initial_lower;
    for level in start_level..=max_level {
        if raw_runs.iter().all(Vec::is_empty) {
            break;
        }
        let raw_flat = flatten_runs(&raw_runs);
        let (stats, valid_runs) =
            measure_level(level, &raw_runs, &lower, mode, as_of, hist, close_src, None)?;
        replay.raw.insert(level, raw_flat.clone());
        replay.stats.insert(level, stats);
        let next = compose_next(level + 1, &valid_runs, mode)?;
        lower = raw_flat;
        raw_runs = next;
    }
    Ok(replay)
}

fn compose_next(
    next_level: usize,
    runs: &[Vec<LeveledMove>],
    mode: ProjectMode,
) -> Result<Vec<Vec<LeveledMove>>, String> {
    let mut out = Vec::new();
    let mut ordinal = 0_u64;
    for run in runs {
        if run.is_empty() {
            continue;
        }
        let projection = project(run, mode)
            .map_err(|error| format!("L{next_level} 递归投影失败: {error:?}"))?
            .0;
        let centers: Vec<_> = projection.seeds.iter().map(|s| s.center).collect();
        let blocks = decompose::decompose(&centers);
        let units = project_to_units(run, &blocks);
        let (_, mut upper, _) = compose_level(&units, run, false, next_level as u32);
        for value in &mut upper {
            value.id = ElementId {
                level: next_level as u32,
                ordinal,
            };
            ordinal += 1;
        }
        if !upper.is_empty() {
            out.push(upper);
        }
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn measure_level(
    level: usize,
    raw_runs: &[Vec<LeveledMove>],
    lower: &[LeveledMove],
    mode: ProjectMode,
    as_of: usize,
    hist: &[f64],
    close_src: &[usize],
    mut missing_out: Option<&mut Vec<MissingDetail>>,
) -> Result<(LevelStats, Vec<Vec<LeveledMove>>), String> {
    let lower_legs =
        lower_legs_from(lower).map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
    let mut stats = LevelStats::default();
    let mut valid_runs = Vec::new();
    let mut global_run = 0;
    for raw in raw_runs {
        stats.windows += raw.len();
        let mut start = None;
        for index in 0..=raw.len() {
            let valid = if index < raw.len() {
                match project(std::slice::from_ref(&raw[index]), mode) {
                    Ok((projection, offsets)) => {
                        stats.bit_same_checks += usize::from(offsets[0] == 0);
                        stats.recovered_windows += usize::from(offsets[0] > 0);
                        stats.seed_keys.push(SeedKey::from(&projection.seeds[0]));
                        true
                    }
                    Err(ProjectionError::InvalidSeed { .. }) => {
                        stats.invalid_seed += 1;
                        false
                    }
                    Err(other) => {
                        return Err(format!("L{level} 意外投影错误: {other:?}"));
                    }
                }
            } else {
                false
            };
            match (start, valid) {
                (None, true) => start = Some(index),
                (Some(s), false) => {
                    let run = raw[s..index].to_vec();
                    measure_run(
                        level,
                        global_run,
                        &run,
                        &lower_legs,
                        mode,
                        as_of,
                        hist,
                        close_src,
                        &mut stats,
                        missing_out.as_deref_mut(),
                    )?;
                    valid_runs.push(run);
                    global_run += 1;
                    start = None;
                }
                _ => {}
            }
        }
    }
    stats.seeds = stats.seed_keys.len();
    stats.centers = stats.seeds;
    stats.seed_keys.sort();
    Ok((stats, valid_runs))
}

#[allow(clippy::too_many_arguments)]
fn measure_run(
    level: usize,
    run_index: usize,
    windows: &[LeveledMove],
    lower_legs: &[LowerLeg],
    mode: ProjectMode,
    as_of: usize,
    hist: &[f64],
    close_src: &[usize],
    stats: &mut LevelStats,
    mut missing_out: Option<&mut Vec<MissingDetail>>,
) -> Result<(), String> {
    let projection = project(windows, mode)
        .map_err(|error| format!("L{level} run={run_index} 投影失败: {error:?}"))?
        .0;
    let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
    let blocks = decompose::decompose(&centers);
    let start = projection.seeds.first().expect("nonempty").start_index;
    let end = projection.seeds.last().expect("nonempty").end_index;
    let query = LevelViewQuery {
        level: level as u32,
        coordinate_window: CoordinateWindow { start, end },
        as_of,
        version: C2VersionTuple::auto_pairing(),
    };
    let view = assemble_level_view(
        C2LevelViewConfig { enabled: true },
        query,
        LevelViewMaterial {
            projection: ProjectionMaterial::ExactThree(&projection),
            move_blocks: &blocks,
            lower_legs,
            hist,
            close_src,
        },
    )
    .map_err(|error| format!("L{level} run={run_index} assemble 失败: {error:?}"))?;
    stats.pairs += view.pairs.len();
    let segments: Vec<_> = lower_legs.iter().map(leg_as_segment).collect();
    let anchors = self_anchors(&segments);
    for (block, assembled) in blocks.iter().zip(&view.moves) {
        let Some(direction) = block.dir else {
            continue;
        };
        stats.trend_blocks += 1;
        let is_missing = matches!(
            assembled.completion,
            CompletionStatus::Pending {
                reason: PendingReason::MissingDivergencePair,
                ..
            }
        );
        if is_missing {
            stats.missing_pair += 1;
            if let Some(out) = missing_out.as_deref_mut() {
                out.push(diagnose_missing(
                    level,
                    run_index,
                    block.start_center,
                    block.end_center,
                    direction,
                    assembled.start_index,
                    assembled.end_index,
                    &projection,
                    &segments,
                    &anchors,
                    as_of,
                )?);
            }
        }
    }
    Ok(())
}

fn project(
    windows: &[LeveledMove],
    mode: ProjectMode,
) -> Result<(ExactThreeProjection, Vec<usize>), ProjectionError> {
    if mode == ProjectMode::Production {
        let value = project_extended_windows(windows)?;
        return Ok((value, vec![0; windows.len()]));
    }
    let mut seeds = Vec::with_capacity(windows.len());
    let mut offsets = Vec::with_capacity(windows.len());
    for (index, window) in windows.iter().enumerate() {
        let count = window.sub_moves.len();
        if count < 3 {
            return Err(ProjectionError::TooShort {
                index,
                sub_count: count,
            });
        }
        let mut chosen = None;
        for start in 0..=count - 3 {
            if let Some(center) = center_at(window.id.level as usize, &window.sub_moves, start) {
                chosen = Some((start, center));
                break;
            }
        }
        let (start, center) = chosen.ok_or(ProjectionError::InvalidSeed { index })?;
        let a = as_unit(&window.sub_moves[start]).expect("center_at checked");
        let c = as_unit(&window.sub_moves[start + 2]).expect("center_at checked");
        let seed = ExactThreeSeed {
            source_id: window.id,
            source_sub_count: count,
            start_index: a.start_index,
            end_index: c.end_index,
            center,
            // 审计工具按 offset 自核构造；start==0 时与生产 seed 比对——若生产侧因
            // 继承核改写（InheritedRecut）而不等，按原逻辑记 InvalidSeed 显式暴露。
            core_provenance: SeedCoreProvenance::SelfConsistent,
        };
        if start == 0 {
            let production = project_extended_windows(std::slice::from_ref(window))?;
            if production.seeds[0] != seed {
                return Err(ProjectionError::InvalidSeed { index });
            }
        }
        seeds.push(seed);
        offsets.push(start);
    }
    Ok((
        ExactThreeProjection {
            version: ProviderVersion::EXTENDED_TO_EXACT_THREE_V1,
            seeds,
        },
        offsets,
    ))
}

fn center_at(level: usize, values: &[LeveledMove], start: usize) -> Option<Center> {
    let a = as_unit(&values[start])?;
    let b = as_unit(&values[start + 1])?;
    let c = as_unit(&values[start + 2])?;
    if level == 1 {
        center_from_segments(&a, &b, &c)
    } else {
        center_from_window(&a, &b, &c)
    }
}

fn collect_affected(
    tower: &[std::rc::Rc<Vec<LeveledMove>>],
) -> Result<Vec<AffectedWindow>, String> {
    let mut out = Vec::new();
    for level in 1..tower.len() {
        for window in tower[level].iter() {
            match project_extended_windows(std::slice::from_ref(window)) {
                Ok(_) => continue,
                Err(ProjectionError::InvalidSeed { .. }) => {}
                Err(other) => return Err(format!("L{level} 意外基线投影错误: {other:?}")),
            }
            if window.sub_moves.len() < 3 {
                return Err(format!("L{level} InvalidSeed 却少于三段"));
            }
            let first = [
                as_unit(&window.sub_moves[0]),
                as_unit(&window.sub_moves[1]),
                as_unit(&window.sub_moves[2]),
            ];
            let [Some(a), Some(b), Some(c)] = first else {
                return Err(format!("L{level} D1 非 A3：as_unit=None"));
            };
            if level == 1 && !dir_alternates(&a, &b, &c) {
                return Err("D1 非 A3：方向不交替".to_string());
            }
            if a.lo.max(b.lo).max(c.lo) <= a.hi.min(b.hi).min(c.hi) {
                return Err("D1 非 A3：首三段核心非空".to_string());
            }
            let valid_starts = if window.sub_moves.len() > 3 {
                (1..=window.sub_moves.len() - 3)
                    .filter(|start| center_at(level, &window.sub_moves, *start).is_some())
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            if let Some(&chosen_start) = valid_starts.first() {
                let center = center_at(level, &window.sub_moves, chosen_start)
                    .expect("valid start has center");
                out.push(AffectedWindow {
                    level,
                    id: window.id,
                    window_start: window.start_index,
                    window_end: window.end_index,
                    sub_count: window.sub_moves.len(),
                    valid_starts,
                    chosen_start,
                    chosen_source_start: window.sub_moves[chosen_start].start_index,
                    center,
                });
            }
        }
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn diagnose_missing(
    level: usize,
    run: usize,
    block_start: usize,
    block_end: usize,
    direction: Direction,
    move_start: usize,
    move_end: usize,
    projection: &ExactThreeProjection,
    segments: &[Segment],
    anchors: &[Option<Direction>],
    as_of: usize,
) -> Result<MissingDetail, String> {
    if block_end <= block_start || block_end >= projection.seeds.len() {
        return Err("MissingDivergencePair 命中 B1 guard".to_string());
    }
    let prev = projection.seeds[block_end - 1].center;
    let last = projection.seeds[block_end].center;
    let lo = segments.partition_point(|s| s.start_index < prev.end_index);
    let hi = segments.partition_point(|s| s.start_index < last.end_index);
    let same = anchors[lo..hi]
        .iter()
        .filter(|a| **a == Some(direction))
        .count();
    let reenters = |s: &Segment| {
        s.direction != direction
            && match direction {
                Direction::Down => s.end_price >= prev.zd,
                Direction::Up => s.end_price <= prev.zg,
            }
    };
    let boundary = segments[lo..hi]
        .iter()
        .rev()
        .find(|s| reenters(s))
        .map(|s| s.end_index);
    let boundary_value = boundary.unwrap_or(0);
    let after_boundary = segments[lo..hi]
        .iter()
        .zip(&anchors[lo..hi])
        .filter(|(s, a)| **a == Some(direction) && s.start_index >= boundary_value)
        .count();
    if locate_departure_move_a(segments, anchors, &prev, &last, direction).is_some() {
        return Err("MissingDivergencePair 非 B2".to_string());
    }
    let mut same_segments = segments[lo..hi]
        .iter()
        .zip(&anchors[lo..hi])
        .filter(|(_, a)| **a == Some(direction))
        .map(|(s, _)| s);
    let first = same_segments.next();
    let last_same = same_segments.last().or(first);
    let counterfactual_a = first
        .zip(last_same)
        .map(|(a, z)| (a.start_index, z.end_index));
    let c_terminal = segments.iter().rev().find(|s| {
        s.direction == direction && s.start_index >= last.end_index && s.end_index <= as_of
    });
    let c = c_terminal.and_then(|terminal| {
        departure_move_c_start(segments, anchors, &last, direction, terminal.start_index)
            .map(|start| (start, terminal.end_index))
    });
    Ok(MissingDetail {
        level,
        run,
        block_start,
        block_end,
        direction,
        move_start,
        move_end,
        prev_center: prev,
        last_center: last,
        a_window_segments: hi - lo,
        a_window_same_dir: same,
        boundary,
        after_boundary,
        counterfactual_a,
        c,
    })
}

fn build_counterfactuals(
    missing: &[MissingDetail],
    hist: &[f64],
    close_src: &[usize],
) -> Result<Vec<CounterfactualPair>, String> {
    let mut out = Vec::new();
    for detail in missing.iter().filter(|d| d.a_window_same_dir > 0) {
        if detail.boundary.is_none() || detail.after_boundary != 0 {
            return Err("7 条回退候选不满足边界后归零".to_string());
        }
        let seg_a = detail.counterfactual_a.ok_or("反事实 A 不可定位")?;
        let seg_c = detail.c.ok_or("反事实 C 不可定位")?;
        let a_idx =
            map_src_to_close_idx(close_src, seg_a.0, seg_a.1).ok_or("反事实 A 无 MACD 坐标")?;
        let c_idx =
            map_src_to_close_idx(close_src, seg_c.0, seg_c.1).ok_or("反事实 C 无 MACD 坐标")?;
        let area_a = segment_macd_area(hist, a_idx.0, a_idx.1);
        let area_c = segment_macd_area(hist, c_idx.0, c_idx.1);
        let diverges = segments_diverge(hist, a_idx, c_idx);
        if diverges != (area_c < area_a) {
            return Err("segments_diverge 与面积严格比较不一致".to_string());
        }
        out.push(CounterfactualPair {
            level: detail.level,
            run: detail.run,
            block_start: detail.block_start,
            block_end: detail.block_end,
            direction: detail.direction,
            move_start: detail.move_start,
            move_end: detail.move_end,
            prev_center_end: detail.prev_center.end_index,
            last_center_end: detail.last_center.end_index,
            a_window_segments: detail.a_window_segments,
            boundary: detail.boundary.expect("candidate has reentry boundary"),
            seg_a,
            seg_c,
            area_a,
            area_c,
            diverges,
        });
    }
    Ok(out)
}

fn validate_baseline(
    baseline: &BTreeMap<usize, LevelStats>,
    affected: &[AffectedWindow],
    missing: &[MissingDetail],
) -> Result<(), String> {
    let d1 = (1..=5)
        .map(|l| baseline.get(&l).map_or(0, |s| s.invalid_seed))
        .collect::<Vec<_>>();
    let missing_levels = (1..=5)
        .map(|l| baseline.get(&l).map_or(0, |s| s.missing_pair))
        .collect::<Vec<_>>();
    let other = (1..=5)
        .map(|l| affected.iter().filter(|a| a.level == l).count())
        .collect::<Vec<_>>();
    let other_starts = affected.iter().map(|a| a.valid_starts.len()).sum::<usize>();
    let with_anchor = missing.iter().filter(|d| d.a_window_same_dir > 0).count();
    let without_anchor = missing.iter().filter(|d| d.a_window_same_dir == 0).count();
    if d1 != EXPECTED_D1 || d1.iter().sum::<usize>() != 215 {
        return Err(format!("D1 守恒失败: {d1:?}"));
    }
    if missing_levels != EXPECTED_MISSING || missing.len() != 21 {
        return Err(format!(
            "D2 守恒失败: {missing_levels:?}, total={}",
            missing.len()
        ));
    }
    if other != EXPECTED_OTHER_VALID || affected.len() != 36 || other_starts != 46 {
        return Err(format!(
            "D1 备选守恒失败: levels={other:?} windows={} starts={other_starts}",
            affected.len()
        ));
    }
    if with_anchor != 7 || without_anchor != 14 {
        return Err(format!(
            "D2 7/14 守恒失败: with={with_anchor} without={without_anchor}"
        ));
    }
    if missing.iter().any(|d| d.after_boundary != 0) {
        return Err("D2 守恒失败：存在界后锚非零".to_string());
    }
    Ok(())
}

fn cascade_levels(
    detail: &AffectedWindow,
    production: &Replay,
    sliding: &Replay,
    max_level: usize,
) -> Vec<usize> {
    let mut out = Vec::new();
    for level in detail.level + 1..=max_level {
        let baseline = production
            .raw
            .get(&level)
            .map(|v| v.iter().map(window_signature).collect::<BTreeSet<_>>())
            .unwrap_or_default();
        let changed_descendant = sliding.raw.get(&level).is_some_and(|values| {
            values.iter().any(|value| {
                contains_id(value, detail.id) && !baseline.contains(&window_signature(value))
            })
        });
        if changed_descendant {
            out.push(level);
        }
    }
    out
}

fn contains_id(value: &LeveledMove, id: ElementId) -> bool {
    value.id == id || value.sub_moves.iter().any(|sub| contains_id(sub, id))
}

fn multiset_diff<T: Ord + Clone>(baseline: &[T], variant: &[T]) -> (usize, usize, usize) {
    let mut a = baseline.to_vec();
    let mut b = variant.to_vec();
    a.sort();
    b.sort();
    let mut i = 0;
    let mut j = 0;
    let mut same = 0;
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                same += 1;
                i += 1;
                j += 1;
            }
        }
    }
    (same, baseline.len() - same, variant.len() - same)
}

fn window_signature(value: &LeveledMove) -> String {
    let subs = value
        .sub_moves
        .iter()
        .map(|s| {
            format!(
                "{}:{}:{}:{}:{}",
                s.start_index,
                s.end_index,
                s.rmove.lo(),
                s.rmove.hi(),
                direction_code(first_leaf_direction(s))
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    format!(
        "{}:{}:{}:{}:{}:[{}]",
        value.start_index,
        value.end_index,
        value.rmove.lo(),
        value.rmove.hi(),
        direction_code(first_leaf_direction(value)),
        subs
    )
}

fn direction_code(value: Option<Direction>) -> u8 {
    match value {
        None => 0,
        Some(Direction::Up) => 1,
        Some(Direction::Down) => 2,
    }
}

fn flatten_runs(runs: &[Vec<LeveledMove>]) -> Vec<LeveledMove> {
    runs.iter().flat_map(|run| run.iter().cloned()).collect()
}

fn as_unit(value: &LeveledMove) -> Option<UnitRange> {
    Some(UnitRange {
        start_index: value.start_index,
        end_index: value.end_index,
        direction: first_leaf_direction(value)?,
        lo: value.rmove.lo(),
        hi: value.rmove.hi(),
    })
}

fn first_leaf_direction(value: &LeveledMove) -> Option<Direction> {
    let mut node = &value.rmove;
    loop {
        match node {
            RMove::Segment { direction, .. } => return Some(*direction),
            RMove::Compose { subs, .. } => node = subs.first()?,
        }
    }
}

fn leg_as_segment(value: &LowerLeg) -> Segment {
    let (start_price, end_price) = match value.direction {
        Direction::Up => (value.lo, value.hi),
        Direction::Down => (value.hi, value.lo),
    };
    Segment {
        direction: value.direction,
        start_index: value.start_index,
        end_index: value.end_index,
        start_price,
        end_price,
    }
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
    let first_date = raw.dates.first().cloned().unwrap_or_default();
    let last_date = raw.dates.last().cloned().unwrap_or_default();
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
        first_date,
        last_date,
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
