//! task #83：D1-D7 闭环后，BTC 1m 全量严格 C2 产量与跨级嵌套结构重测。
//!
//! 只读消费生产 `assemble_level_view` seam；不改塔、不生成信号、不执行裁决。
//! 用法：`cargo run --release --bin p83_yield_remeasure -- <btc_1m_full.json>`

use newchan_rust::theta_v0::classifier::classify_with_tower;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::divergence::compute_macd;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows, C2LevelViewConfig,
    C2VersionTuple, CompletionStatus, CoordinateWindow, LevelAsOfView, LevelViewMaterial,
    LevelViewQuery, PendingReason, ProjectionError, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::level_view_store::{
    CompletedFreezeAdapter, JsonlCompletedEventStore,
};
use newchan_rust::theta_v0::classifier::nest::{is_sub, NestInterval};
use newchan_rust::theta_v0::classifier::recursive_tower::LeveledMove;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{quantize, Bar, Direction, MoveKind, Timestamp};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
struct PendingCounts {
    awaiting_subsequent_move: usize,
    missing_divergence_pair: usize,
    missing_macd_coordinates: usize,
    terminal_leg_not_divergent: usize,
}

impl PendingCounts {
    fn total(&self) -> usize {
        self.awaiting_subsequent_move
            + self.missing_divergence_pair
            + self.missing_macd_coordinates
            + self.terminal_leg_not_divergent
    }

    fn observe(&mut self, reason: PendingReason) {
        match reason {
            PendingReason::AwaitingSubsequentMove => self.awaiting_subsequent_move += 1,
            PendingReason::MissingDivergencePair => self.missing_divergence_pair += 1,
            PendingReason::MissingMacdCoordinates => self.missing_macd_coordinates += 1,
            PendingReason::TerminalLegNotDivergent => self.terminal_leg_not_divergent += 1,
        }
    }
}

#[derive(Debug, Default)]
struct DirectionCounts {
    trend_up: usize,
    trend_down: usize,
    consolidation_none: usize,
}

#[derive(Debug, Default)]
struct ProjectionFailures {
    too_short: usize,
    invalid_seed: usize,
}

impl ProjectionFailures {
    fn total(&self) -> usize {
        self.too_short + self.invalid_seed
    }

    fn observe(&mut self, error: ProjectionError) -> Result<(), String> {
        match error {
            ProjectionError::TooShort { .. } => self.too_short += 1,
            ProjectionError::InvalidSeed { .. } => self.invalid_seed += 1,
            ProjectionError::InvalidLowerLeg { .. } => {
                return Err("单窗口 D1 投影意外返回 InvalidLowerLeg".to_string())
            }
            ProjectionError::MissingCarriedCenter { .. } => {
                return Err(
                    "单窗口 D1 投影意外返回 MissingCarriedCenter（#90：塔应逐窗携带核）"
                        .to_string(),
                )
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct StrictPairInterval {
    start: usize,
    end: usize,
}

impl StrictPairInterval {
    fn as_nest(self, idx: usize) -> NestInterval {
        NestInterval {
            start_time: self.start as u64,
            end_time: self.end as u64,
            idx: idx as u64,
        }
    }
}

#[derive(Debug, Default)]
struct LevelStats {
    tower_windows: usize,
    projection_runs: usize,
    projected_seeds: usize,
    projection_failures: ProjectionFailures,
    unassigned_projected: usize,
    moves: usize,
    completed_moves: usize,
    completed_freeze_events: usize,
    directions: DirectionCounts,
    pending: PendingCounts,
    divergence_pairs: usize,
    strict_pairs: Vec<StrictPairInterval>,
}

impl LevelStats {
    fn unassigned(&self) -> usize {
        self.projection_failures.total() + self.unassigned_projected
    }
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
        .ok_or("用法: p83_yield_remeasure <btc_1m_full.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }

    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let as_of = loaded.bars.len() - 1;
    let layer = parse_layer(&loaded.bars, &config);
    let (classification, tower) = classify_with_tower(&layer, &config);
    let closes: Vec<_> = layer
        .merged_bars
        .iter()
        .map(|bar| bar.close as f64)
        .collect();
    let close_src: Vec<_> = layer
        .merged_bars
        .iter()
        .map(|bar| bar.source_index)
        .collect();
    let hist = compute_macd(&closes, &config.macd).hist;
    let dif = compute_macd(&closes, &config.macd).dif;
    let version = C2VersionTuple::auto_pairing();
    version
        .validate()
        .map_err(|error| format!("完整 version tuple 非法: {error:?}"))?;

    println!(
        "P83_INPUT bars={} merged={} as_of={} first_date={} last_date={} tower_levels={} classification_levels={}",
        loaded.bars.len(),
        layer.merged_bars.len(),
        as_of,
        loaded.first_date,
        loaded.last_date,
        tower.len(),
        classification.levels.len()
    );
    println!(
        "P83_VERSION direction_provider=central-ggdd-v1 divergence_pair_provider=move-block-ac-v1 projection_provider=extended-to-exact-three-v1"
    );
    println!(
        "P83_DEFINITION strict_pair=two_distinct_adjacent_completed_moves interval=[leave.start..retest.end] direction_distribution=completed_moves_only unassigned=d1_projection_failure_or_projected_seed_without_move"
    );
    println!(
        "P83_NEST_DEFINITION edge=is_sub(lower_pair_interval,higher_pair_interval) complete=one_pair_at_every_level_L1_through_Ln"
    );

    let mut levels = Vec::new();
    for level in 1..tower.len() {
        let stats = measure_level(
            level,
            &tower[level],
            &tower[level - 1],
            as_of,
            &hist,
            &dif,
            &close_src,
        )?;
        println!(
            "P83_LEVEL L{level} tower_windows={} projection_runs={} projected_seeds={} d1_too_short={} d1_invalid_seed={} moves={} completed_moves={} completed_freeze_events={} strict_c2_pairs={} trend_up={} trend_down={} consolidation_none={} pending_total={} pending_awaiting_subsequent={} pending_missing_divergence_pair={} pending_missing_macd_coordinates={} pending_terminal_not_divergent={} unassigned={} unassigned_projected={} divergence_pairs={}",
            stats.tower_windows,
            stats.projection_runs,
            stats.projected_seeds,
            stats.projection_failures.too_short,
            stats.projection_failures.invalid_seed,
            stats.moves,
            stats.completed_moves,
            stats.completed_freeze_events,
            stats.strict_pairs.len(),
            stats.directions.trend_up,
            stats.directions.trend_down,
            stats.directions.consolidation_none,
            stats.pending.total(),
            stats.pending.awaiting_subsequent_move,
            stats.pending.missing_divergence_pair,
            stats.pending.missing_macd_coordinates,
            stats.pending.terminal_leg_not_divergent,
            stats.unassigned(),
            stats.unassigned_projected,
            stats.divergence_pairs,
        );
        levels.push(stats);
    }

    let completed_moves: usize = levels.iter().map(|value| value.completed_moves).sum();
    let freeze_events: usize = levels
        .iter()
        .map(|value| value.completed_freeze_events)
        .sum();
    let strict_pairs: usize = levels.iter().map(|value| value.strict_pairs.len()).sum();
    let pending: usize = levels.iter().map(|value| value.pending.total()).sum();
    let unassigned: usize = levels.iter().map(LevelStats::unassigned).sum();
    let moves: usize = levels.iter().map(|value| value.moves).sum();
    let trend_up: usize = levels.iter().map(|value| value.directions.trend_up).sum();
    let trend_down: usize = levels.iter().map(|value| value.directions.trend_down).sum();
    let consolidation_none: usize = levels
        .iter()
        .map(|value| value.directions.consolidation_none)
        .sum();
    let pending_awaiting: usize = levels
        .iter()
        .map(|value| value.pending.awaiting_subsequent_move)
        .sum();
    let pending_missing_pair: usize = levels
        .iter()
        .map(|value| value.pending.missing_divergence_pair)
        .sum();
    let pending_missing_macd: usize = levels
        .iter()
        .map(|value| value.pending.missing_macd_coordinates)
        .sum();
    let pending_not_divergent: usize = levels
        .iter()
        .map(|value| value.pending.terminal_leg_not_divergent)
        .sum();
    let divergence_pairs: usize = levels.iter().map(|value| value.divergence_pairs).sum();
    let (complete_chains, max_depth, max_depth_chains) = chain_stats(&levels);
    println!(
        "P83_TOTAL levels={} moves={} completed_moves={} completed_freeze_events={} strict_c2_pairs={} trend_up={} trend_down={} consolidation_none={} pending={} pending_awaiting_subsequent={} pending_missing_divergence_pair={} pending_missing_macd_coordinates={} pending_terminal_not_divergent={} unassigned={} divergence_pairs={}",
        levels.len(),
        moves,
        completed_moves,
        freeze_events,
        strict_pairs,
        trend_up,
        trend_down,
        consolidation_none,
        pending,
        pending_awaiting,
        pending_missing_pair,
        pending_missing_macd,
        pending_not_divergent,
        unassigned,
        divergence_pairs
    );
    println!(
        "P83_NEST complete_chains={} full_depth={} max_consecutive_depth={} max_depth_chain_count={}",
        complete_chains,
        levels.len(),
        max_depth,
        max_depth_chains
    );
    Ok(())
}

fn measure_level(
    level: usize,
    windows: &[LeveledMove],
    lower: &[LeveledMove],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
) -> Result<LevelStats, String> {
    let lower_legs =
        lower_legs_from(lower).map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
    let mut stats = LevelStats {
        tower_windows: windows.len(),
        ..LevelStats::default()
    };

    // 生产 D1 对非法 seed fail closed。全量测量把非法窗口记为 Unassigned，并仅在其两侧的
    // 最大连续合法 run 内调用相同投影与组装 seam；绝不跨非法窗口拼接 CompletedMove。
    let mut run_start: Option<usize> = None;
    for index in 0..=windows.len() {
        let valid = if index < windows.len() {
            match project_extended_windows(std::slice::from_ref(&windows[index])) {
                Ok(_) => true,
                Err(error) => {
                    stats.projection_failures.observe(error)?;
                    false
                }
            }
        } else {
            false
        };
        match (run_start, valid) {
            (None, true) => run_start = Some(index),
            (Some(start), false) => {
                measure_run(
                    level,
                    stats.projection_runs,
                    &windows[start..index],
                    &lower_legs,
                    as_of,
                    hist,
                    dif,
                    close_src,
                    &mut stats,
                )?;
                stats.projection_runs += 1;
                run_start = None;
            }
            _ => {}
        }
    }
    Ok(stats)
}

#[allow(clippy::too_many_arguments)]
fn measure_run(
    level: usize,
    run_index: usize,
    windows: &[LeveledMove],
    lower_legs: &[newchan_rust::theta_v0::classifier::level_view::LowerLeg],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    stats: &mut LevelStats,
) -> Result<(), String> {
    let projection = project_extended_windows(windows)
        .map_err(|error| format!("L{level} run={run_index} D1 投影失败: {error:?}"))?;
    let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
    let blocks = decompose::decompose(&centers);
    let start = projection
        .seeds
        .first()
        .ok_or_else(|| format!("L{level} run={run_index} 空投影"))?
        .start_index;
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
            dif,
            close_src,
        },
    )
    .map_err(|error| format!("L{level} run={run_index} C2 assemble 失败: {error:?}"))?;

    stats.projected_seeds += projection.seeds.len();
    stats.moves += view.moves.len();
    stats.divergence_pairs += view.pairs.len();
    stats.unassigned_projected += projection
        .seeds
        .iter()
        .enumerate()
        .filter(|(seed_index, _)| {
            !view
                .moves
                .iter()
                .any(|value| value.center_indices.contains(seed_index))
        })
        .count();
    observe_moves(&view, stats)?;
    stats.completed_freeze_events += freeze_event_count(level, run_index, query, &view)?;
    Ok(())
}

fn observe_moves(view: &LevelAsOfView, stats: &mut LevelStats) -> Result<(), String> {
    for value in &view.moves {
        match value.completion {
            CompletionStatus::Completed { .. } => {
                stats.completed_moves += 1;
                match (value.kind, value.direction) {
                    (MoveKind::Trend, Some(Direction::Up)) => stats.directions.trend_up += 1,
                    (MoveKind::Trend, Some(Direction::Down)) => stats.directions.trend_down += 1,
                    (MoveKind::Consolidation, None) => stats.directions.consolidation_none += 1,
                    other => return Err(format!("D3 三值方向不变量破坏: {other:?}")),
                }
            }
            CompletionStatus::Pending { reason, .. } => stats.pending.observe(reason),
        }
    }
    for pair in view.moves.windows(2) {
        if matches!(pair[0].completion, CompletionStatus::Completed { .. })
            && matches!(pair[1].completion, CompletionStatus::Completed { .. })
        {
            stats.strict_pairs.push(StrictPairInterval {
                start: pair[0].start_index,
                end: pair[1].end_index,
            });
        }
    }
    Ok(())
}

fn freeze_event_count(
    level: usize,
    run_index: usize,
    query: LevelViewQuery,
    view: &LevelAsOfView,
) -> Result<usize, String> {
    let path = temp_event_path(level, run_index);
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("清理临时 event store 失败: {error}")),
    }
    let store = JsonlCompletedEventStore::new(&path);
    let mut adapter = CompletedFreezeAdapter::new(store, query)
        .map_err(|error| format!("L{level} run={run_index} D5 reopen 失败: {error:?}"))?;
    let appended = adapter
        .observe(view)
        .map_err(|error| format!("L{level} run={run_index} D5 observe 失败: {error:?}"))?;
    let event_count = adapter.reducer().events().len();
    if appended != event_count {
        return Err(format!(
            "L{level} run={run_index} D5 事件数不守恒: appended={appended}, events={event_count}"
        ));
    }
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        // 该 run 没有 CompletedMove 时 adapter 不会创建空文件，零事件是合法测量结果。
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("删除临时 event store 失败: {error}")),
    }
    Ok(event_count)
}

fn temp_event_path(level: usize, run_index: usize) -> PathBuf {
    std::env::temp_dir().join(format!(
        "p83-yield-remeasure-{}-L{level}-run{run_index}.jsonl",
        std::process::id()
    ))
}

/// 返回：(贯穿 L1..Ln 的完整链数，任意连续级别最大深度，该最大深度链数)。
fn chain_stats(levels: &[LevelStats]) -> (u128, usize, u128) {
    if levels.is_empty() {
        return (0, 0, 0);
    }
    let mut previous: Vec<(usize, u128)> = levels[0]
        .strict_pairs
        .iter()
        .map(|_| (1usize, 1u128))
        .collect();
    let mut global_depth = usize::from(!previous.is_empty());
    let mut global_count = previous.len() as u128;

    for level_index in 1..levels.len() {
        let lower_pairs = &levels[level_index - 1].strict_pairs;
        let mut current = Vec::with_capacity(levels[level_index].strict_pairs.len());
        for (parent_index, parent) in levels[level_index].strict_pairs.iter().enumerate() {
            let parent_iv = parent.as_nest(parent_index);
            let best_child_depth = lower_pairs
                .iter()
                .enumerate()
                .filter(|(child_index, child)| is_sub(&child.as_nest(*child_index), &parent_iv))
                .map(|(child_index, _)| previous[child_index].0)
                .max()
                .unwrap_or(0);
            let (depth, count) = if best_child_depth == 0 {
                (1, 1)
            } else {
                let count = lower_pairs
                    .iter()
                    .enumerate()
                    .filter(|(child_index, child)| {
                        previous[*child_index].0 == best_child_depth
                            && is_sub(&child.as_nest(*child_index), &parent_iv)
                    })
                    .map(|(child_index, _)| previous[child_index].1)
                    .sum();
                (best_child_depth + 1, count)
            };
            current.push((depth, count));
        }
        let level_depth = current.iter().map(|value| value.0).max().unwrap_or(0);
        let level_count = current
            .iter()
            .filter(|value| value.0 == level_depth)
            .map(|value| value.1)
            .sum();
        if level_depth > global_depth {
            global_depth = level_depth;
            global_count = level_count;
        } else if level_depth == global_depth && level_depth > 0 {
            global_count += level_count;
        }
        previous = current;
    }
    let complete = previous
        .iter()
        .filter(|value| value.0 == levels.len())
        .map(|value| value.1)
        .sum();
    (complete, global_depth, global_count)
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
