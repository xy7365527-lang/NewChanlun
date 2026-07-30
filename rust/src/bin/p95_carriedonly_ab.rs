//! task #95：CarriedOnly 版本化迁移（V2→V3）全量 A/B 对账探针。
//!
//! 验收门槛 = 裁定 `chanlun/escalate/d1-invalidseed-carriedonly-migration-ruling-20260716.md` Q2 条款：
//! - 条款 2：全量重放 A/B——未涉窗 bit-exact 零 diff；215 窗逐窗显式 provenance 清单。
//! - 条款 3：下游证书差异清单——run 合并导致的 `provide_nest_candidate_events` /
//!   CompletedMove 输出差异逐条列出（交裁决，不默认接受）。
//! - 条款 4：收复窗邻域采样的 prefix 重放逐时点对账（A/B 同时点双侧重放，
//!   B-only 回翻与收复域前差异记违规；全前缀 O(N^2) 在单 run 覆盖全级时不可行，
//!   且 decompose 前缀增长下的块级回翻为管线既有性质，须以 A 侧作对照）。
//!
//! A = 生产 V2（`project_extended_windows`，InvalidSeed fail-closed）；
//! B = V3（`project_extended_windows_carried_only`，携带核收复 + `CarriedOnly` 打标）。
//! 只读消费；不改塔、不生成信号、不执行裁决。
//! 用法：`cargo run --release --bin p95_carriedonly_ab -- <btc_1m_full.json>`

use newchan_rust::theta_v0::classifier::classify_with_tower;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::divergence::compute_macd;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, completed_move_starts, lower_legs_from, project_extended_windows,
    project_extended_windows_carried_only, provide_nest_candidate_events, C2LevelViewConfig,
    C2VersionTuple, CoordinateWindow, ExactThreeProjection, LevelAsOfView, LevelViewMaterial,
    LevelViewQuery, LowerLeg, NestCandidateEvent, ProjectionError, ProjectionMaterial,
    SeedCoreProvenance,
};
use newchan_rust::theta_v0::classifier::recursive_tower::LeveledMove;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{quantize, Bar, Timestamp};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Default)]
struct LevelTally {
    windows: usize,
    bit_exact: usize,
    recovered: usize,
    both_fail_too_short: usize,
    both_fail_invalid_units: usize,
    both_fail_other: usize,
    seed_diff: usize,
}

#[derive(Debug, Clone, Copy)]
struct RunSpan {
    start: usize,
    end: usize, // exclusive
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p95_carriedonly_ab <btc_1m_full.json>")?;
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
    let (_classification, tower) = classify_with_tower(&layer, &config);
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

    println!(
        "P95_INPUT bars={} merged={} as_of={} first_date={} last_date={} tower_levels={}",
        loaded.bars.len(),
        layer.merged_bars.len(),
        as_of,
        loaded.first_date,
        loaded.last_date,
        tower.len()
    );
    println!(
        "P95_VERSION a=extended-to-exact-three-v2-inherited-core b=extended-to-exact-three-v3-carried-only dir=central-ggdd-v1 pair=move-block-ac-v1"
    );

    let mut per_level_recovered = Vec::new();
    let mut total = LevelTally::default();
    let mut clause3_diff_runs = 0usize;
    let mut clause3_event_added = 0usize;
    let mut clause3_event_removed = 0usize;
    let mut clause3_completed_added = 0usize;
    let mut clause3_completed_removed = 0usize;
    let mut clause4_checkpoints = 0usize;
    let mut clause4_pre_violation = 0usize;
    let mut clause4_offdomain_diffs = 0usize;
    let mut clause4_indomain_diff_ckpts = 0usize;
    let mut clause4_bonly_flips = 0usize;
    let mut clause4_flip_obs = 0usize;

    for level in 1..tower.len() {
        let windows = &tower[level];
        let lower = &tower[level - 1];
        let lower_legs = lower_legs_from(lower)
            .map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
        let mut tally = LevelTally::default();
        tally.windows = windows.len();

        // ---- 条款 2：逐窗 A/B ----
        let mut valid_a = vec![false; windows.len()];
        let mut valid_b = vec![false; windows.len()];
        let mut recovered_idx: BTreeSet<usize> = BTreeSet::new();
        for (index, window) in windows.iter().enumerate() {
            let a = project_extended_windows(std::slice::from_ref(window));
            let b = project_extended_windows_carried_only(std::slice::from_ref(window));
            match (a, b) {
                (Ok(pa), Ok(pb)) => {
                    valid_a[index] = true;
                    valid_b[index] = true;
                    if pa.seeds == pb.seeds {
                        tally.bit_exact += 1;
                    } else {
                        tally.seed_diff += 1;
                        println!(
                            "P95_SEED_DIFF level={level} idx={index} a={:?} b={:?}",
                            pa.seeds[0], pb.seeds[0]
                        );
                    }
                }
                (Err(ProjectionError::InvalidSeed { .. }), Ok(pb)) => {
                    valid_b[index] = true;
                    recovered_idx.insert(index);
                    tally.recovered += 1;
                    let seed = &pb.seeds[0];
                    if seed.core_provenance != SeedCoreProvenance::CarriedOnly {
                        return Err(format!(
                            "L{level} idx={index} 收复窗 provenance 非 CarriedOnly: {:?}",
                            seed.core_provenance
                        ));
                    }
                    println!(
                        "P95_RECOVERED level={level} idx={index} id={:?} span={}:{} sub_count={} zd={:?} zg={:?} dd={:?} gg={:?} provenance=CarriedOnly",
                        seed.source_id,
                        seed.start_index,
                        seed.end_index,
                        seed.source_sub_count,
                        seed.center.zd,
                        seed.center.zg,
                        seed.center.dd,
                        seed.center.gg
                    );
                }
                (Err(ea), Err(eb)) => {
                    match (ea, eb) {
                        (
                            ProjectionError::TooShort { .. },
                            ProjectionError::TooShort { .. },
                        ) => tally.both_fail_too_short += 1,
                        (
                            ProjectionError::InvalidSeed { .. },
                            ProjectionError::InvalidSeed { .. },
                        ) => tally.both_fail_invalid_units += 1,
                        _ => {
                            tally.both_fail_other += 1;
                            println!(
                                "P95_BOTH_FAIL_MIXED level={level} idx={index} a={ea:?} b={eb:?}"
                            );
                        }
                    }
                }
                (Ok(_), Err(eb)) => {
                    return Err(format!(
                        "L{level} idx={index} 致命: A 合法而 B 失败 {eb:?}——V3 只可扩不可缩"
                    ))
                }
                (Err(ea), Ok(_)) => {
                    return Err(format!(
                        "L{level} idx={index} 致命: A 以非 InvalidSeed 失败（{ea:?}）而 B 合法——收复域越界"
                    ))
                }
            }
        }
        println!(
            "P95_LEVEL L{level} windows={} bit_exact={} recovered={} both_fail_too_short={} both_fail_invalid_units={} both_fail_other={} seed_diff={}",
            tally.windows,
            tally.bit_exact,
            tally.recovered,
            tally.both_fail_too_short,
            tally.both_fail_invalid_units,
            tally.both_fail_other,
            tally.seed_diff
        );
        per_level_recovered.push(tally.recovered);

        // ---- run 分区拓扑 ----
        let runs_a = extract_runs(&valid_a);
        let runs_b = extract_runs(&valid_b);
        let mut merged = 0usize;
        let mut extended = 0usize;
        let mut isolated = 0usize;
        let mut unchanged = 0usize;
        for rb in &runs_b {
            let contained: Vec<_> = runs_a
                .iter()
                .filter(|ra| ra.start >= rb.start && ra.end <= rb.end)
                .collect();
            let affected = recovered_idx.iter().any(|i| *i >= rb.start && *i < rb.end);
            if !affected {
                unchanged += 1;
                // 条款 2 下游覆盖：未涉 run 的 A/B 组装结果必须逐位一致。
                let (va, ea) = assemble_run(
                    level,
                    windows,
                    rb,
                    &lower_legs,
                    as_of,
                    &hist,
                    &dif,
                    &close_src,
                    false,
                )?;
                let (vb, eb) = assemble_run(
                    level,
                    windows,
                    rb,
                    &lower_legs,
                    as_of,
                    &hist,
                    &dif,
                    &close_src,
                    true,
                )?;
                if completed_move_starts(&va) != completed_move_starts(&vb)
                    || va.moves != vb.moves
                    || event_keys(&ea) != event_keys(&eb)
                {
                    return Err(format!(
                        "L{level} 未涉 run {}..{} A/B 视图不一致——违反零 diff 门槛",
                        rb.start, rb.end
                    ));
                }
                continue;
            }
            match contained.len() {
                0 => isolated += 1,
                1 => extended += 1,
                _ => merged += 1,
            }
            // ---- 条款 3：下游差异清单 ----
            let (vb, eb) = assemble_run(
                level,
                windows,
                rb,
                &lower_legs,
                as_of,
                &hist,
                &dif,
                &close_src,
                true,
            )?;
            let mut starts_a_union: BTreeSet<usize> = BTreeSet::new();
            let mut events_a_union: BTreeSet<String> = BTreeSet::new();
            for ra in &contained {
                let (va, ea) = assemble_run(
                    level,
                    windows,
                    ra,
                    &lower_legs,
                    as_of,
                    &hist,
                    &dif,
                    &close_src,
                    false,
                )?;
                starts_a_union.extend(completed_move_starts(&va));
                events_a_union.extend(event_keys(&ea));
            }
            let starts_b = completed_move_starts(&vb);
            let events_b = event_keys(&eb);
            let added_starts: Vec<_> = starts_b.difference(&starts_a_union).collect();
            let removed_starts: Vec<_> = starts_a_union.difference(&starts_b).collect();
            let added_events: Vec<_> = events_b.difference(&events_a_union).collect();
            let removed_events: Vec<_> = events_a_union.difference(&events_b).collect();
            clause3_diff_runs += 1;
            clause3_completed_added += added_starts.len();
            clause3_completed_removed += removed_starts.len();
            clause3_event_added += added_events.len();
            clause3_event_removed += removed_events.len();
            println!(
                "P95_RUN_DIFF level={level} b_run={}..{} a_runs_contained={} recovered_in_run={} completed_a={} completed_b={} starts_added={:?} starts_removed={:?} events_a={} events_b={}",
                rb.start,
                rb.end,
                contained.len(),
                recovered_idx
                    .iter()
                    .filter(|i| **i >= rb.start && **i < rb.end)
                    .count(),
                starts_a_union.len(),
                starts_b.len(),
                added_starts,
                removed_starts,
                events_a_union.len(),
                events_b.len()
            );
            for key in &added_events {
                println!(
                    "P95_EVENT_ADDED level={level} b_run={}..{} {key}",
                    rb.start, rb.end
                );
            }
            for key in &removed_events {
                println!(
                    "P95_EVENT_REMOVED level={level} b_run={}..{} {key}",
                    rb.start, rb.end
                );
            }

            // ---- 条款 4：收复窗邻域采样的 prefix 重放逐时点对账 ----
            // 口径说明：全前缀 O(N^2) 在单 run 覆盖全级（L1 b_run=0..9266）时不可行；
            // 且 decompose 在前缀增长下本就存在块级回翻（A 侧同样发生），单侧监测把
            // 管线既有性质误记为 B 侧违规。此处以每个收复窗 r 的邻域 [r-1, r+16] 采样
            // checkpoint，同一时点 A/B 双侧重放：
            //   (a) 前缀尚无收复窗时 A==B 必须逐位成立（同输入同输出）；
            //   (b) 收复域之前坐标（首个已含收复窗 start_index 之前、且 A-run 已覆盖）
            //       的完成集 A/B 不得出现差异；
            //   (c) 相邻采样点间 B 侧回翻若 A 侧同转移无对应回翻、坐标位于收复域之前
            //       且落在 A 已覆盖坐标域内（gone >= a_cover，否则 A 侧对照空洞），
            //       记违规（B-only 回翻）；其余回翻记观察量。
            let rec_in_run: Vec<usize> = recovered_idx
                .iter()
                .filter(|i| **i >= rb.start && **i < rb.end)
                .copied()
                .collect();
            let mut sample: BTreeSet<usize> = BTreeSet::new();
            for r in &rec_in_run {
                let lo = r.saturating_sub(1).max(rb.start);
                let hi = (r + 16).min(rb.end - 1);
                sample.extend(lo..=hi);
            }
            let mut prev: Option<(usize, BTreeSet<usize>, Option<BTreeSet<usize>>)> = None;
            for &k in &sample {
                let prefix = RunSpan {
                    start: rb.start,
                    end: k + 1,
                };
                let prefix_as_of = windows[k].end_index;
                let (vbk, _) = assemble_run(
                    level,
                    windows,
                    &prefix,
                    &lower_legs,
                    prefix_as_of,
                    &hist,
                    &dif,
                    &close_src,
                    true,
                )?;
                let b_starts = completed_move_starts(&vbk);
                clause4_checkpoints += 1;
                // 域界：前缀内首个收复窗（若有）的 start_index。
                let boundary = rec_in_run
                    .iter()
                    .find(|r| **r <= k)
                    .map(|r| windows[*r].start_index);
                // A 侧同点重放（checkpoint 落在某 A-run 内部时）。
                let a_view = if let Some(ra) = runs_a
                    .iter()
                    .find(|ra| k >= ra.start && k < ra.end && ra.start >= rb.start)
                {
                    let a_prefix = RunSpan {
                        start: ra.start,
                        end: k + 1,
                    };
                    let (vak, _) = assemble_run(
                        level,
                        windows,
                        &a_prefix,
                        &lower_legs,
                        prefix_as_of,
                        &hist,
                        &dif,
                        &close_src,
                        false,
                    )?;
                    Some((windows[ra.start].start_index, completed_move_starts(&vak)))
                } else {
                    None
                };
                if let Some((a_cover, a_starts)) = &a_view {
                    match boundary {
                        None => {
                            // (a) 首个收复窗之前：同起点同窗，必须零 diff。
                            if *a_starts != b_starts {
                                clause4_pre_violation += 1;
                                println!(
                                    "P95_PREFIX_VIOLATION level={level} b_run={}..{} checkpoint={k} a={:?} b={:?}",
                                    rb.start, rb.end, a_starts, b_starts
                                );
                            }
                        }
                        Some(bd) => {
                            // (b) 收复域之前、A 已覆盖坐标域：完成集不得有差异。
                            let a_before: BTreeSet<_> = a_starts
                                .iter()
                                .filter(|s| **s < bd && **s >= *a_cover)
                                .copied()
                                .collect();
                            let b_before: BTreeSet<_> = b_starts
                                .iter()
                                .filter(|s| **s < bd && **s >= *a_cover)
                                .copied()
                                .collect();
                            if a_before != b_before {
                                clause4_offdomain_diffs += 1;
                                println!(
                                    "P95_PREFIX_OFFDOMAIN level={level} b_run={}..{} checkpoint={k} boundary={bd} a_cover={a_cover} a_before={:?} b_before={:?}",
                                    rb.start, rb.end, a_before, b_before
                                );
                            }
                            // 域内差异：逐时点对账观察量。
                            let added = b_starts.difference(a_starts).count();
                            let removed = a_starts.difference(&b_starts).count();
                            if added + removed > 0 {
                                clause4_indomain_diff_ckpts += 1;
                                println!(
                                    "P95_CKPT level={level} b_run={}..{} checkpoint={k} as_of={prefix_as_of} boundary={bd} a={} b={} added={added} removed={removed}",
                                    rb.start,
                                    rb.end,
                                    a_starts.len(),
                                    b_starts.len()
                                );
                            }
                        }
                    }
                }
                // (c) 相邻采样点间回翻对照。
                if let Some((pk, pb, pa)) = &prev {
                    if *pk + 1 == k {
                        let b_lost: BTreeSet<usize> = pb.difference(&b_starts).copied().collect();
                        let a_lost: Option<BTreeSet<usize>> = match (pa, &a_view) {
                            (Some(prev_a), Some((_, cur_a))) => {
                                Some(prev_a.difference(cur_a).copied().collect())
                            }
                            _ => None,
                        };
                        for gone in &b_lost {
                            let shared = a_lost.as_ref().is_some_and(|al| al.contains(gone));
                            let off_domain = boundary.map(|bd| *gone < bd).unwrap_or(true);
                            // A 侧对照仅在 A 实际覆盖该坐标时才有效力：A-run 起点之前的
                            // 坐标 A 视图本就不含，"A 未回翻"是空洞对照，不构成控制组。
                            let covered = a_view.as_ref().is_some_and(|(c, _)| gone >= c);
                            if !shared && off_domain && covered && a_lost.is_some() {
                                clause4_bonly_flips += 1;
                                println!(
                                    "P95_RETRO_BONLY level={level} b_run={}..{} checkpoint={k} lost_completed_start={gone}",
                                    rb.start, rb.end
                                );
                            } else {
                                clause4_flip_obs += 1;
                            }
                        }
                    }
                }
                prev = Some((k, b_starts, a_view.map(|(_, s)| s)));
            }
        }
        println!(
            "P95_RUNS L{level} runs_a={} runs_b={} unchanged={} merged={} extended={} isolated={}",
            runs_a.len(),
            runs_b.len(),
            unchanged,
            merged,
            extended,
            isolated
        );

        total.windows += tally.windows;
        total.bit_exact += tally.bit_exact;
        total.recovered += tally.recovered;
        total.both_fail_too_short += tally.both_fail_too_short;
        total.both_fail_invalid_units += tally.both_fail_invalid_units;
        total.both_fail_other += tally.both_fail_other;
        total.seed_diff += tally.seed_diff;
    }

    println!(
        "P95_CLAUSE2 windows={} bit_exact={} recovered={} per_level_recovered={:?} seed_diff={} both_fail_too_short={} both_fail_invalid_units={} both_fail_other={}",
        total.windows,
        total.bit_exact,
        total.recovered,
        per_level_recovered,
        total.seed_diff,
        total.both_fail_too_short,
        total.both_fail_invalid_units,
        total.both_fail_other
    );
    println!(
        "P95_CLAUSE3 diff_runs={clause3_diff_runs} completed_added={clause3_completed_added} completed_removed={clause3_completed_removed} events_added={clause3_event_added} events_removed={clause3_event_removed}"
    );
    println!(
        "P95_CLAUSE4 checkpoints={clause4_checkpoints} pre_violation={clause4_pre_violation} offdomain_diffs={clause4_offdomain_diffs} indomain_diff_ckpts={clause4_indomain_diff_ckpts} bonly_flips={clause4_bonly_flips} flip_obs={clause4_flip_obs}"
    );
    let status = if total.seed_diff == 0
        && total.both_fail_other == 0
        && clause4_pre_violation == 0
        && clause4_offdomain_diffs == 0
        && clause4_bonly_flips == 0
    {
        "PASS"
    } else {
        "FAIL"
    };
    println!("P95_STATUS status={status}");
    Ok(())
}

fn extract_runs(valid: &[bool]) -> Vec<RunSpan> {
    let mut runs = Vec::new();
    let mut start: Option<usize> = None;
    for index in 0..=valid.len() {
        let v = index < valid.len() && valid[index];
        match (start, v) {
            (None, true) => start = Some(index),
            (Some(s), false) => {
                runs.push(RunSpan {
                    start: s,
                    end: index,
                });
                start = None;
            }
            _ => {}
        }
    }
    runs
}

#[allow(clippy::too_many_arguments)]
fn assemble_run(
    level: usize,
    windows: &[LeveledMove],
    run: &RunSpan,
    lower_legs: &[LowerLeg],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    carried_only: bool,
) -> Result<(LevelAsOfView, Vec<NestCandidateEvent>), String> {
    let slice = &windows[run.start..run.end];
    let projection: ExactThreeProjection = if carried_only {
        project_extended_windows_carried_only(slice)
    } else {
        project_extended_windows(slice)
    }
    .map_err(|error| {
        format!(
            "L{level} run={}..{} carried_only={carried_only} 投影失败: {error:?}",
            run.start, run.end
        )
    })?;
    let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
    let blocks = decompose::decompose(&centers);
    let start = projection.seeds.first().expect("nonempty").start_index;
    let end = projection.seeds.last().expect("nonempty").end_index;
    let version = if carried_only {
        C2VersionTuple::carried_only()
    } else {
        C2VersionTuple::auto_pairing()
    };
    let query = LevelViewQuery {
        level: level as u32,
        coordinate_window: CoordinateWindow { start, end },
        as_of,
        version,
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
    .map_err(|error| {
        format!(
            "L{level} run={}..{} carried_only={carried_only} C2 assemble 失败: {error:?}",
            run.start, run.end
        )
    })?;
    let events = provide_nest_candidate_events(
        level as u32,
        &projection,
        &blocks,
        lower_legs,
        &view,
        hist,
        dif,
        close_src,
    );
    Ok((view, events))
}

/// 事件对账键：排除 `provider_window`（run 分区变化下必然不同，不构成语义 diff——
/// 语义域 = 事件几何 + 判定坐标 + 方向/种类 + 背驰确认位）。
fn event_keys(events: &[NestCandidateEvent]) -> BTreeSet<String> {
    events
        .iter()
        .map(|e| {
            format!(
                "side={:?} kind={:?} seg_a={:?} interval_b={:?} interval_a={:?} confirmed={} turn={} judge={}",
                e.side,
                e.kind,
                e.seg_a,
                e.interval_b,
                e.interval_a,
                e.divergence_confirmed,
                e.turn_source,
                e.judge_at
            )
        })
        .collect()
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
