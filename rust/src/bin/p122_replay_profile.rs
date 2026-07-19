//! task #122（重放优化深研·热点剖析工位）：p92/p116 prefix pass 操作级成本探针。
//!
//! 语义 = p92_nest_replay_postruling 的 run_terminal_pass + run_targeted_prefix_pass 原样
//! （同一 trigger 判据 (forest_epoch, signal_signature)、同一 pending 生命周期、同一
//! collect_target_candidates 装配链：lower_legs → run 分区扫 → run 投影 → decompose →
//! assemble_level_view → provide_nest_candidate_events）。只叠加 Instant 分段计时，
//! 不改任何判定/账本/钟位；不落盘、不进真值。探针定位：只测量，只读。
//!
//! 用法：
//! `cargo run --release --bin p122_replay_profile -- <btc_1m_full.json>`
//! env：`P116_MAX_BARS`（优先，沿用 p116 协议）→ `P122_MAX_BARS`（回退）→ 默认 500_000。
//!
//! ── Phase 0 插桩（p123 §6.1，2026-07-18，只加计数/计时，不动任何判定与账本）──
//! 四组输出，计数口径逐条声明：
//! (a) P122_TRIGGER：trigger=(forest_epoch, signal_signature) 元组的逐 bar 变化分解。
//!     changes=相邻 bar 元组差次数（bar0 首次观察不计入）；epoch_only/sig_only/both=变化中
//!     仅 forest_epoch 变/仅 signal_signature 变/两者同变；fired*=其中 pending 非空真正
//!     执行评估的子集（首次观察单列 fired_initial）；suppressed_empty_pending=变化时
//!     pending 已空（现行实现不更新 last_trigger，评估被抑制）。恒等式：
//!     changes == (fired - fired_initial) + suppressed_empty_pending。
//! (b) P122_REEVAL：每 (level, run_source_start) run 在 prefix pass 内被评估（view）的
//!     次数直方图，桶=1 / 2-5 / 6-20 / 21-100 / >100，分 level + TOTAL。
//! (c) P122_FIXED：固定税=lower_legs+run_partition（每 trigger×级重建，跨 trigger 零缓存）；
//!     view 税=projection+decompose+assemble+provide。给出累计、per-trigger 均值与两者比例。
//! (d) P122_SHAPES(+_WINDOWS)：terminal 时 levels/L0 段数/各级窗数；pending 峰值（口径：
//!     pending 单调递减，峰值=初始 targets 数@其首个观察 bar）；arrived_peak=单 trigger
//!     的 arrived（turn_source<=as_of 的 pending 目标数）峰值。另每 100k bar 在 stderr
//!     输出 P122_SHAPE_PROGRESS 采样（l0 段数与各级窗数随 bar 增长曲线）。
//! (e) P122_ROUNDS(+_LEVEL)：有效评估口径——某 (level, run_source) run 在某 bar 的评估
//!     产出 ≥1 个推进账本的事件（candidates 首插 / divergences 首插 / pending 出清，
//!     重发的同 key 事件不计）。productive_evals=有效 (run,bar) 评估数，是 per-run dirty
//!     稀疏化（p123 候选 C）必须保留的评估下界：1 − productive_evals/views = view 数
//!     节省上界。轮次粒度：productive/unproductive/no_view rounds 与 unproductive_views
//!     （无账本推进轮次的 view 总数，轮次级浪费）。
//! 计时口径：墙钟 Instant 累加；后台有其他重放/探针争核时 elapsed 仅作相对对照。

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CoordinateWindow,
    LevelViewMaterial, LevelViewQuery, NestCandidateEvent, NestDivergenceKind, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::recursive_tower::LeveledMove;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{quantize, Bar, Side, Timestamp};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

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

/// 分段计时桶（全部累加，main 末尾统一换算打印）。
#[derive(Debug, Default)]
struct StageClock {
    parse: Duration,
    tower: Duration,
    signal_sig: Duration,
    group: Duration,
    lower_legs: Duration,
    run_partition: Duration,
    projection: Duration,
    decompose: Duration,
    assemble: Duration,
    provide: Duration,
    event_apply: Duration,
    snapshot_terminal: Duration,
}

/// 结构性计数（成本模型的形状参数）。
#[derive(Debug, Default)]
struct Counters {
    triggers: usize,
    views: usize,
    /// 每次触发时 arrived（turn_source<=as_of）目标数总和 —— 分组扫描规模。
    arrived_sum: usize,
    /// run 分区扫描消费的单窗投影尝试总数（O(W_level) 固定税）。
    partition_window_probes: usize,
    /// 完整 run 投影消费的窗口总数（Σ run_len over views）。
    projection_windows: usize,
    /// lower_legs 构造的元素总数（Σ legs over 触发×级）。
    lower_leg_elems: usize,
    per_level_views: BTreeMap<usize, usize>,
    per_level_ns: BTreeMap<usize, Duration>,
    per_level_run_windows: BTreeMap<usize, usize>,
    // ── Phase 0 (p123 §6.1) 插桩计数（口径见模块头注释）──
    trig_changes: usize,
    trig_epoch_only: usize,
    trig_sig_only: usize,
    trig_both: usize,
    trig_fired_epoch_only: usize,
    trig_fired_sig_only: usize,
    trig_fired_both: usize,
    trig_fired_initial: usize,
    trig_suppressed_empty: usize,
    pending_peak: usize,
    pending_peak_bar: usize,
    arrived_peak: usize,
    run_evals: BTreeMap<(usize, usize), usize>,
    // (e) 有效评估与轮次粒度（口径见模块头 (e) 段）
    productive_evals: usize,
    productive_evals_by_level: BTreeMap<usize, usize>,
    rounds_productive: usize,
    rounds_unproductive: usize,
    rounds_no_views: usize,
    unproductive_views: usize,
}

struct TerminalState {
    classification: classifier::Classification,
    tower: Vec<Rc<Vec<LeveledMove>>>,
    cache: classifier::TowerCache,
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p122_replay_profile <btc_1m_full.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P116_MAX_BARS")
        .ok()
        .or_else(|| std::env::var("P122_MAX_BARS").ok())
        .and_then(|value| value.parse::<usize>().ok())
        .map_or(500_000usize.min(loaded.bars.len()), |value| {
            value.min(loaded.bars.len())
        });
    println!(
        "P122_INPUT bars={} replay_bars={} first_date={} last_date={}",
        loaded.bars.len(),
        max_bars,
        loaded.first_date,
        loaded.last_date
    );

    let mut clock = StageClock::default();
    let mut counters = Counters::default();
    let total_started = Instant::now();

    // ── 段 1：terminal pass（与 p92 逐行同构，仅计时）──
    let terminal = run_terminal_pass(&loaded.bars[..max_bars], &config, &mut clock)?;
    let (hist, close_src) = terminal.cache.causal_series();
    let dif = terminal.cache.macd_dif();
    // ── 段 2：终态快照 → targets（同 p92 collect_snapshot_candidates，仅计时）──
    let snap_started = Instant::now();
    let terminal_events = collect_snapshot_candidates(
        &terminal.tower,
        max_bars - 1,
        hist,
        dif,
        close_src,
    )?;
    clock.snapshot_terminal = snap_started.elapsed();
    let mut targets = BTreeMap::new();
    for event in terminal_events.iter().flatten() {
        targets.insert(EventKey::from(event), *event);
    }
    println!("P122_TARGETS count={}", targets.len());
    // ── 段 3：targeted prefix pass（同 p92，分段计时）──
    let prefix_started = Instant::now();
    let unresolved = run_targeted_prefix_pass(
        &loaded.bars[..max_bars],
        &config,
        &targets,
        &mut clock,
        &mut counters,
    )?;
    let prefix_elapsed = prefix_started.elapsed();
    let total_elapsed = total_started.elapsed();

    // ── 报告 ──
    let measured = clock.parse
        + clock.tower
        + clock.signal_sig
        + clock.group
        + clock.lower_legs
        + clock.run_partition
        + clock.projection
        + clock.decompose
        + clock.assemble
        + clock.provide
        + clock.event_apply;
    println!(
        "P122_PROFILE total_s={:.3} terminal_tower_s={:.3} snapshot_s={:.3} prefix_s={:.3} targets={} unresolved={}",
        total_elapsed.as_secs_f64(),
        clock.tower.as_secs_f64() + clock.parse.as_secs_f64(),
        clock.snapshot_terminal.as_secs_f64(),
        prefix_elapsed.as_secs_f64(),
        targets.len(),
        unresolved
    );
    println!(
        "P122_COUNTERS triggers={} views={} arrived_sum={} partition_probes={} projection_windows={} lower_leg_elems={}",
        counters.triggers,
        counters.views,
        counters.arrived_sum,
        counters.partition_window_probes,
        counters.projection_windows,
        counters.lower_leg_elems
    );
    let stages = [
        ("parse", clock.parse),
        ("tower_incremental", clock.tower),
        ("signal_signature", clock.signal_sig),
        ("group_pending", clock.group),
        ("lower_legs", clock.lower_legs),
        ("run_partition", clock.run_partition),
        ("projection", clock.projection),
        ("decompose", clock.decompose),
        ("assemble_view", clock.assemble),
        ("provide_events", clock.provide),
        ("event_apply", clock.event_apply),
    ];
    for (name, dur) in stages {
        let pct = if measured.as_secs_f64() > 0.0 {
            100.0 * dur.as_secs_f64() / measured.as_secs_f64()
        } else {
            0.0
        };
        let secs = dur.as_secs_f64();
        println!("P122_STAGE name={name} secs={secs:.3} pct={pct:.2}");
    }
    for (level, views) in &counters.per_level_views {
        let ns = counters.per_level_ns.get(level).copied().unwrap_or_default();
        let rw = counters.per_level_run_windows.get(level).copied().unwrap_or(0);
        let per_view_ms = if *views > 0 {
            ns.as_secs_f64() * 1000.0 / *views as f64
        } else {
            0.0
        };
        let per_view_windows = if *views > 0 { rw / views } else { 0 };
        println!(
            "P122_LEVEL level={level} views={views} secs={:.3} ms_per_view={per_view_ms:.3} run_windows_per_view={per_view_windows}",
            ns.as_secs_f64()
        );
    }
    // ── Phase 0 (p123 §6.1) 四组实测输出（口径见模块头注释）──
    // (a) trigger 精确次数与分解
    println!(
        "P122_TRIGGER changes={} epoch_only={} sig_only={} both={} fired={} fired_initial={} fired_epoch_only={} fired_sig_only={} fired_both={} suppressed_empty_pending={}",
        counters.trig_changes,
        counters.trig_epoch_only,
        counters.trig_sig_only,
        counters.trig_both,
        counters.triggers,
        counters.trig_fired_initial,
        counters.trig_fired_epoch_only,
        counters.trig_fired_sig_only,
        counters.trig_fired_both,
        counters.trig_suppressed_empty
    );
    // (b) 每 (level, run_source_start) run 评估次数直方图（桶：1 / 2-5 / 6-20 / 21-100 / >100）
    let mut reeval: BTreeMap<usize, (usize, usize, usize, [usize; 5])> = BTreeMap::new();
    let mut total_runs = 0usize;
    let mut total_sum = 0usize;
    let mut total_max = 0usize;
    let mut total_hist = [0usize; 5];
    for ((level, _run_source), count) in &counters.run_evals {
        let bucket = match *count {
            1 => 0,
            2..=5 => 1,
            6..=20 => 2,
            21..=100 => 3,
            _ => 4,
        };
        let entry = reeval.entry(*level).or_default();
        entry.0 += 1;
        entry.1 += *count;
        entry.2 = entry.2.max(*count);
        entry.3[bucket] += 1;
        total_runs += 1;
        total_sum += *count;
        total_max = total_max.max(*count);
        total_hist[bucket] += 1;
    }
    for (level, (runs, sum, max, hist)) in &reeval {
        println!(
            "P122_REEVAL level={level} runs={runs} b1={} b2_5={} b6_20={} b21_100={} b100p={} max={max} mean={:.2}",
            hist[0], hist[1], hist[2], hist[3], hist[4],
            *sum as f64 / *runs as f64
        );
    }
    if total_runs > 0 {
        println!(
            "P122_REEVAL level=TOTAL runs={total_runs} b1={} b2_5={} b6_20={} b21_100={} b100p={} max={total_max} mean={:.2}",
            total_hist[0], total_hist[1], total_hist[2], total_hist[3], total_hist[4],
            total_sum as f64 / total_runs as f64
        );
    }
    // (e) 有效评估与轮次粒度（稀疏化节省界）
    println!(
        "P122_ROUNDS productive_rounds={} unproductive_rounds={} no_view_rounds={} unproductive_views={} productive_evals={}",
        counters.rounds_productive,
        counters.rounds_unproductive,
        counters.rounds_no_views,
        counters.unproductive_views,
        counters.productive_evals
    );
    for (level, prod) in &counters.productive_evals_by_level {
        let views = counters.per_level_views.get(level).copied().unwrap_or(0);
        println!("P122_ROUNDS_LEVEL level={level} productive_evals={prod} views={views}");
    }
    // (c) 固定税（lower_legs+run_partition）vs view 税（projection+decompose+assemble+provide）
    let fixed = clock.lower_legs + clock.run_partition;
    let view = clock.projection + clock.decompose + clock.assemble + clock.provide;
    let per_trigger_ms = if counters.triggers > 0 {
        fixed.as_secs_f64() * 1000.0 / counters.triggers as f64
    } else {
        0.0
    };
    let per_view_ms = if counters.views > 0 {
        view.as_secs_f64() * 1000.0 / counters.views as f64
    } else {
        0.0
    };
    let fixed_over_view = if view.as_secs_f64() > 0.0 {
        100.0 * fixed.as_secs_f64() / view.as_secs_f64()
    } else {
        0.0
    };
    let fixed_over_measured = if measured.as_secs_f64() > 0.0 {
        100.0 * fixed.as_secs_f64() / measured.as_secs_f64()
    } else {
        0.0
    };
    println!(
        "P122_FIXED fixed_s={:.3} lower_legs_s={:.3} run_partition_s={:.3} group_pending_s={:.3} per_trigger_ms={per_trigger_ms:.3} view_s={:.3} per_view_ms={per_view_ms:.3} fixed_over_view_pct={fixed_over_view:.2} fixed_over_measured_pct={fixed_over_measured:.2}",
        fixed.as_secs_f64(),
        clock.lower_legs.as_secs_f64(),
        clock.run_partition.as_secs_f64(),
        clock.group.as_secs_f64(),
        view.as_secs_f64()
    );
    // (d) terminal 形状 + pending/arrived 峰值
    let tower = &terminal.tower;
    println!(
        "P122_SHAPES levels={} l0_segments={} pending_peak={} pending_peak_bar={} arrived_peak={}",
        tower.len(),
        tower.first().map_or(0, |wins| wins.len()),
        counters.pending_peak,
        counters.pending_peak_bar,
        counters.arrived_peak
    );
    for (level, wins) in tower.iter().enumerate() {
        println!("P122_SHAPES_WINDOWS level={level} windows={}", wins.len());
    }
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

fn run_terminal_pass(
    bars: &[Bar],
    config: &ThetaConfig,
    clock: &mut StageClock,
) -> Result<TerminalState, String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = classifier::TowerCache::new();
    let mut terminal = None;
    for (index, bar) in bars.iter().copied().enumerate() {
        let t0 = Instant::now();
        let l0 = parser.append(bar);
        let t1 = Instant::now();
        let (classification, tower) =
            classifier::classify_with_tower_incremental(&l0, config, &mut cache);
        clock.parse += t1 - t0;
        clock.tower += t1.elapsed();
        if index > 0 && index % 100_000 == 0 {
            eprintln!(
                "P122_TERMINAL_PROGRESS bar={index}/{} tower_s={:.1}",
                bars.len(),
                clock.tower.as_secs_f64()
            );
        }
        if index + 1 == bars.len() {
            terminal = Some((classification, tower));
        }
    }
    let (classification, tower) = terminal.ok_or("空 replay")?;
    Ok(TerminalState {
        classification,
        tower,
        cache,
    })
}

fn run_targeted_prefix_pass(
    bars: &[Bar],
    config: &ThetaConfig,
    targets: &BTreeMap<EventKey, NestCandidateEvent>,
    clock: &mut StageClock,
    counters: &mut Counters,
) -> Result<usize, String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = classifier::TowerCache::new();
    let mut pending: BTreeSet<EventKey> = targets.keys().cloned().collect();
    let mut last_trigger = None;
    // Phase 0：逐 bar 的上一 trigger 快照（变化分解口径=相邻 bar 元组差；与 last_trigger
    // 不同——后者在 pending 空后冻结）。pending 非空期间二者逐位一致（last_trigger 每次
    // 变化都更新），故 fired 分解用 prev_trigger 是精确的。
    let mut prev_trigger: Option<(u64, Vec<(usize, usize, usize, usize)>)> = None;
    let started = Instant::now();
    // 账本存在性同 p92（candidates/divergences 首插钟），但探针只关心 pending 生命周期。
    let mut candidates: BTreeMap<EventKey, usize> = BTreeMap::new();
    let mut divergences: BTreeMap<EventKey, usize> = BTreeMap::new();
    for (index, bar) in bars.iter().copied().enumerate() {
        let t0 = Instant::now();
        let l0 = parser.append(bar);
        let t1 = Instant::now();
        let (classification, tower) =
            classifier::classify_with_tower_incremental(&l0, config, &mut cache);
        clock.parse += t1 - t0;
        clock.tower += t1.elapsed();
        let t2 = Instant::now();
        let trigger = (cache.forest_epoch(), signal_signature(&classification));
        clock.signal_sig += t2.elapsed();
        // ── Phase 0：trigger 逐 bar 变化分解（bar0 首次观察不进 trig_changes）──
        let (epoch_changed, sig_changed) = prev_trigger
            .as_ref()
            .map_or((true, true), |prev| {
                (prev.0 != trigger.0, prev.1 != trigger.1)
            });
        if prev_trigger.is_some() && prev_trigger.as_ref() != Some(&trigger) {
            counters.trig_changes += 1;
            match (epoch_changed, sig_changed) {
                (true, false) => counters.trig_epoch_only += 1,
                (false, true) => counters.trig_sig_only += 1,
                _ => counters.trig_both += 1,
            }
            if pending.is_empty() {
                counters.trig_suppressed_empty += 1;
            }
        }
        if last_trigger.as_ref() != Some(&trigger) && !pending.is_empty() {
            counters.triggers += 1;
            if prev_trigger.is_none() {
                counters.trig_fired_initial += 1;
            } else {
                match (epoch_changed, sig_changed) {
                    (true, false) => counters.trig_fired_epoch_only += 1,
                    (false, true) => counters.trig_fired_sig_only += 1,
                    _ => counters.trig_fired_both += 1,
                }
            }
            let (hist, close_src) = cache.causal_series();
            let dif = cache.macd_dif();
            let views_before = counters.views;
            let events = collect_target_candidates(
                &tower, index, hist, dif, close_src, targets, &pending, clock, counters,
            )?;
            let views_this_round = counters.views - views_before;
            let t3 = Instant::now();
            // Phase 0 (e)：账本推进归属到 (level, run_source)——只加 contains_key 观察，
            // entry/or_insert/remove 的判定语义与顺序同 p92 逐行一致。
            let mut round_productive_runs: BTreeSet<(usize, usize)> = BTreeSet::new();
            for event in events {
                let key = EventKey::from(&event);
                if !pending.contains(&key) {
                    continue;
                }
                let mut advanced = !candidates.contains_key(&key);
                candidates.entry(key.clone()).or_insert(index);
                let target = targets.get(&key).expect("pending target");
                if event.divergence_confirmed {
                    advanced = advanced || !divergences.contains_key(&key);
                    divergences.entry(key.clone()).or_insert(index);
                }
                if event.divergence_confirmed == target.divergence_confirmed {
                    pending.remove(&key);
                    advanced = true;
                }
                if advanced {
                    round_productive_runs.insert((event.level as usize, event.provider_window.0));
                }
            }
            counters.productive_evals += round_productive_runs.len();
            for (level, _run_source) in &round_productive_runs {
                *counters
                    .productive_evals_by_level
                    .entry(*level)
                    .or_default() += 1;
            }
            if views_this_round == 0 {
                counters.rounds_no_views += 1;
            } else if round_productive_runs.is_empty() {
                counters.rounds_unproductive += 1;
                counters.unproductive_views += views_this_round;
            } else {
                counters.rounds_productive += 1;
            }
            clock.event_apply += t3.elapsed();
            last_trigger = Some(trigger.clone());
        }
        prev_trigger = Some(trigger);
        if pending.len() > counters.pending_peak {
            counters.pending_peak = pending.len();
            counters.pending_peak_bar = index;
        }
        if index > 0 && index % 100_000 == 0 {
            eprintln!(
                "P122_PREFIX_PROGRESS bar={index}/{} elapsed={:.1}s pending={}/{} views={} triggers={}",
                bars.len(),
                started.elapsed().as_secs_f64(),
                pending.len(),
                targets.len(),
                counters.views,
                counters.triggers
            );
            let shapes: Vec<String> = tower
                .iter()
                .enumerate()
                .map(|(lv, wins)| format!("w{lv}={}", wins.len()))
                .collect();
            eprintln!(
                "P122_SHAPE_PROGRESS bar={index} levels={} {} pending={}",
                tower.len(),
                shapes.join(" "),
                pending.len()
            );
        }
    }
    Ok(pending.len())
}

#[allow(clippy::too_many_arguments)]
fn collect_target_candidates(
    tower: &[Rc<Vec<LeveledMove>>],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    targets: &BTreeMap<EventKey, NestCandidateEvent>,
    pending: &BTreeSet<EventKey>,
    clock: &mut StageClock,
    counters: &mut Counters,
) -> Result<Vec<NestCandidateEvent>, String> {
    let t0 = Instant::now();
    let mut runs_by_level: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let mut arrived = 0usize;
    for key in pending {
        let Some(event) = targets.get(key) else { continue };
        if event.turn_source <= as_of {
            arrived += 1;
            runs_by_level
                .entry(event.level as usize)
                .or_default()
                .insert(event.provider_window.0);
        }
    }
    counters.arrived_sum += arrived;
    if arrived > counters.arrived_peak {
        counters.arrived_peak = arrived;
    }
    clock.group += t0.elapsed();
    let mut out = Vec::new();
    for (level, run_sources) in runs_by_level {
        if level == 0 || level >= tower.len() {
            continue;
        }
        let windows = &tower[level];
        // ── 桶：lower legs（同 p92，每次触发×级一次）──
        let t1 = Instant::now();
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} targeted lower legs 失败: {error:?}"))?;
        counters.lower_leg_elems += lower.len();
        clock.lower_legs += t1.elapsed();
        // ── 桶：run 分区扫描（同 p92，每窗口一次单窗投影 try）──
        let t2 = Instant::now();
        let mut run_ranges = BTreeMap::new();
        let mut run_start = None;
        let mut run_source = None;
        for index in 0..=windows.len() {
            let seed_start = (index < windows.len())
                .then(|| {
                    project_extended_windows_carried_only(std::slice::from_ref(&windows[index]))
                        .ok()
                })
                .flatten()
                .and_then(|projection| projection.seeds.first().map(|seed| seed.start_index));
            if index < windows.len() {
                counters.partition_window_probes += 1;
            }
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
        clock.run_partition += t2.elapsed();
        for run_source_start in run_sources {
            let Some(&(start, end)) = run_ranges.get(&run_source_start) else {
                continue;
            };
            let view_started = Instant::now();
            // ── 桶：run 完整投影 ──
            let t3 = Instant::now();
            let projection = project_extended_windows_carried_only(&windows[start..end])
                .map_err(|error| format!("L{level} targeted projection 失败: {error:?}"))?;
            counters.projection_windows += end - start;
            *counters.per_level_run_windows.entry(level).or_default() += end - start;
            clock.projection += t3.elapsed();
            // ── 桶：decompose ──
            let t4 = Instant::now();
            let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
            let blocks = decompose::decompose(&centers);
            clock.decompose += t4.elapsed();
            let query = LevelViewQuery {
                level: level as u32,
                coordinate_window: CoordinateWindow {
                    start: projection.seeds.first().expect("nonempty").start_index,
                    end: projection.seeds.last().expect("nonempty").end_index,
                },
                as_of,
                version: C2VersionTuple::auto_pairing(),
            };
            // ── 桶：assemble_level_view（内含 provide_divergence_pairs + 逐 trend 块
            //        trend_confirm_time 全合取扫描）──
            let t5 = Instant::now();
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
            clock.assemble += t5.elapsed();
            // ── 桶：provide_nest_candidate_events（trend 分支第二遍 trend_confirm_time +
            //        pan 分支 O(legs) 全扫）──
            let t6 = Instant::now();
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
            clock.provide += t6.elapsed();
            counters.views += 1;
            *counters
                .run_evals
                .entry((level, run_source_start))
                .or_default() += 1;
            *counters.per_level_views.entry(level).or_default() += 1;
            *counters.per_level_ns.entry(level).or_default() += view_started.elapsed();
        }
    }
    Ok(out)
}

/// 终态快照（同 p92 collect_snapshot_candidates 语义：逐级 run 分区 + 全 run 装配）。
/// 探针复用同一分段计时口径但不进 StageClock（只记总数到 counters）。
fn collect_snapshot_candidates(
    tower: &[Rc<Vec<LeveledMove>>],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
) -> Result<Vec<Vec<NestCandidateEvent>>, String> {
    let mut by_level = vec![Vec::new(); tower.len()];
    for level in 1..tower.len() {
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
        let windows = &tower[level];
        let mut run_start = None;
        for index in 0..=windows.len() {
            let valid = if index < windows.len() {
                project_extended_windows_carried_only(std::slice::from_ref(&windows[index])).is_ok()
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
                    .map_err(|error| format!("L{level} C2 assemble 失败: {error:?}"))?;
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
