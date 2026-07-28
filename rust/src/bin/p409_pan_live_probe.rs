//! task #409 探针：pan 活窗真实频次（**只读探针，不接线、不改生产路径**）。
//!
//! 目的（票 #409「要测出的数字」）：在真实行情上量出
//!   1. 产窗次数（`provide_pan_live_windows` 产出的 `PanLiveWindow`，按级别 ℓ 分解）；
//!   2. 反超触发次数（`ForceCheck::Verified(false)` ∧ 曾可证 ⟹ `Invalidated(ForceOvertake)`）；
//!   3. `ForceCheck` 三值分布（Verified(true)/Verified(false)/Unavailable(两码)）；
//!   4. `first_provable_at` 写入时点分布（相对 observed_at 的 prefix 数）。
//!
//! 通道（#301 探针通道先例 + #248 `tangency_probe` 自含 bin 形态）：独立 bin，
//! 不侵入 `rust/src/theta_v0/**` 任何生产模块；`leg_as_segment`（level_view.rs:436 私有）
//! 按票面授权**复制一份**（`runner.rs` 诊断臂同款做法），不为探针提 pub。
//!
//! ── 上游输入的取法（与生产逐字同调用链）──
//! `evaluate_run`（p123_fast_replay.rs:1096-1145）/ `collect_snapshot_candidates`
//! （同文件 :1146+）的 run 分区 + 投影 + decompose 逐字复制：
//!   tower[ℓ] → 单窗 `project_extended_windows_carried_only` 判 run 边界
//!   → run 切片投影 → `centers` = seeds.center → `decompose` → `center_block_kind`
//!   → `segments` = `lower_legs_from(tower[ℓ-1])` 逐腿 `leg_as_segment`
//!   → `anchors_self` = 逐段 `Some(direction)`（**与 `provide_nest_candidate_events_ext`
//!      level_view.rs:735 同口径，不是 `self_anchors`**）
//!   → `provide_pan_live_windows(ℓ, centers, kinds, segments, anchors_self, as_of)`。
//!
//! ── 稀疏化（成本纪律，见报告 §「跑了多少」）──
//! 活窗的**结构分量**（level/side/seg_a/c_start/b_center_start）只依赖 tower[ℓ]/tower[ℓ-1]
//! 内容；`as_of` 只经 (a) `segment.end_index <= as_of` 过滤、(b) 活窗右端 `seg_c_live.1 = as_of`
//! 进入。腿端点恒 ≤ 其写入 bar ⟹ tower 内容不变期间 (a) 恒全通过（p123 模块头 (iii)⊂(ii)
//! 同一论据）。故：`forest_epoch` 变才重算结构，其余 bar 只把右端推到 as_of。
//! `P409_VERIFY=1` 打开**逐 bar 全量重算并逐值对拍**（把稀疏假设变成实测），
//! 计数 `verify_mismatches` 必须为 0。
//!
//! ── 力度三值分布的取法 ──
//! `LifecycleObservation::force` 是私有 fn，探针**照其分支逐字复算**（同一生产原语
//! `map_src_to_close_idx` + `segments_diverge_or`，禁第二查法），并与 book 的 revision
//! 流交叉校验（`force_xcheck_mismatches` 必须为 0）：
//!   - 复算 Unavailable ⟺ 该 prefix 该身份出 `ForceUnavailable`（首见/非幂等抑制时）；
//!   - 复算 Verified(false) ∧ 已有 first_provable ⟺ 该 prefix 出 `Invalidated{ForceOvertake}`；
//!   - 复算 Verified(true) ∧ 无 first_provable ⟺ 该 prefix 出 `FirstProvable`。
//!
//! ── 口径登记（090：声明 = 能力）──
//! - `structure_completed` 恒 `false`：pan 的结构完成信号按契约「判据属调用方」
//!   （nest_lifecycle.rs:325-327），生产侧无既存判据可读，探针不自造。⟹ 本探针
//!   **测不出 Confirmed**，四组数字均不依赖 Confirmed（第 6 步先于第 7 步）。
//! - 事件通道（trend / pan 完成事件）**不投喂**：本票只测 pan 活窗通道。
//! - 只统计不裁决；不做任何概率/统计推断，不回测策略（v3 硬禁令）。
//!
//! env：`P409_MAX_BARS`（截断重放）、`P409_VERIFY=1`（稀疏对拍）、`P409_DUMP=<path>`
//! （逐 entry JSONL）、`P409_PROGRESS`（进度行间隔 bar 数，默认 50000）。
//! 用法：`cargo run --release --bin p409_pan_live_probe -- <bars.json>`

use newchan_rust::theta_v0::classifier::{self, decompose};
use newchan_rust::theta_v0::classifier::divergence::segments_diverge_or;
use newchan_rust::theta_v0::classifier::level_view::{
    lower_legs_from, project_extended_windows_carried_only, LowerLeg,
};
use newchan_rust::theta_v0::classifier::nest_lifecycle::{
    provide_pan_live_windows, ForceMaterial, InvalidatedReason, LifecycleObservation,
    LifecycleRevisionKind, NestEventState, NestLifecycleBook, PanLiveWindow, UnavailReason,
};
use newchan_rust::theta_v0::classifier::recursive_tower::{map_src_to_close_idx, LeveledMove};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{quantize, Bar, Direction, Segment, Side, Timestamp};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

// ═══════════════════════════════════════════════════════════════════════════
// level_view.rs:436 私有 fn 的探针复制（票面授权；不为探针提 pub）
// ═══════════════════════════════════════════════════════════════════════════

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

fn side_tag(side: Side) -> u8 {
    match side {
        Side::Long => 0,
        Side::Short => 1,
    }
}

/// 活窗的**结构分量**（右端 = as_of，不进本键）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct WindowStem {
    level: u32,
    side_tag: u8,
    seg_a: (usize, usize),
    c_start: usize,
    b_center_start: usize,
}

impl WindowStem {
    fn of(window: &PanLiveWindow) -> Self {
        Self {
            level: window.level,
            side_tag: side_tag(window.side),
            seg_a: window.seg_a,
            c_start: window.seg_c_live.0,
            b_center_start: window.b_center_start,
        }
    }

    fn side(&self) -> Side {
        if self.side_tag == 0 {
            Side::Long
        } else {
            Side::Short
        }
    }

    fn window_at(&self, as_of: usize) -> PanLiveWindow {
        PanLiveWindow {
            level: self.level,
            side: self.side(),
            seg_a: self.seg_a,
            seg_c_live: (self.c_start, as_of.max(self.c_start)),
            b_center_start: self.b_center_start,
        }
    }
}

/// 逐级逐 run 全量重算活窗（生产调用链逐字复制）。
fn recompute_windows(tower: &[Rc<Vec<LeveledMove>>], as_of: usize) -> Vec<PanLiveWindow> {
    let mut out = Vec::new();
    for level in 1..tower.len() {
        let Ok(lower) = lower_legs_from(&tower[level - 1]) else {
            continue;
        };
        let segments: Vec<Segment> = lower.iter().map(leg_as_segment).collect();
        let anchors_self: Vec<Option<Direction>> =
            segments.iter().map(|segment| Some(segment.direction)).collect();
        let windows = &tower[level];
        let mut run_start = None;
        for index in 0..=windows.len() {
            let valid = index < windows.len()
                && project_extended_windows_carried_only(std::slice::from_ref(&windows[index]))
                    .is_ok();
            match (run_start, valid) {
                (None, true) => run_start = Some(index),
                (Some(start), false) => {
                    if let Ok(projection) =
                        project_extended_windows_carried_only(&windows[start..index])
                    {
                        let centers: Vec<_> =
                            projection.seeds.iter().map(|seed| seed.center).collect();
                        let blocks = decompose::decompose(&centers);
                        let kinds = decompose::center_block_kind(centers.len(), &blocks);
                        out.extend(provide_pan_live_windows(
                            level as u32,
                            &centers,
                            &kinds,
                            &segments,
                            &anchors_self,
                            as_of,
                        ));
                    }
                    run_start = None;
                }
                _ => {}
            }
        }
    }
    out
}

/// `LifecycleObservation::force`（nest_lifecycle.rs:407-428）pan 分支逐字复算。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ForceTri {
    True,
    False,
    MissingForceSeries,
    CoordinateMapFailed,
}

fn force_of(window: &PanLiveWindow, hist: &[f64], dif: &[f64], close_src: &[usize]) -> ForceTri {
    let (Some(a), Some(c)) = (
        map_src_to_close_idx(close_src, window.seg_a.0, window.seg_a.1),
        map_src_to_close_idx(close_src, window.seg_c_live.0, window.seg_c_live.1),
    ) else {
        return ForceTri::CoordinateMapFailed;
    };
    if segments_diverge_or(hist, dif, window.side, a, c) {
        ForceTri::True
    } else {
        ForceTri::False
    }
}

#[derive(Debug, Default, Clone)]
struct LevelStats {
    /// 逐 prefix 的活窗观察数（同一身份每延展一次计一次）。
    emissions: usize,
    /// 去重后的活窗身份数（= 「产窗次数」的身份口径）。
    identities: usize,
    force_true: usize,
    force_false: usize,
    missing_series: usize,
    coord_failed: usize,
    /// 反超（Verified(false) ∧ 已有 first_provable）落地次数。
    force_overtake: usize,
    /// 从未构成（结构完成时从未写 first_provable）落地次数——本探针硬编码
    /// structure_completed=false ⟹ 恒 0（票 #425 新增原因码，接线票 #426 后才可非零）。
    never_constituted: usize,
    identity_vanished: usize,
    confirmed: usize,
    provisional_alive: usize,
    first_provable_entries: usize,
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p409_pan_live_probe <bars.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P409_MAX_BARS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |value| value.min(loaded.bars.len()));
    let verify = std::env::var("P409_VERIFY").ok().as_deref() == Some("1");
    let progress_every: usize = std::env::var("P409_PROGRESS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(50_000);

    println!(
        "P409_INPUT file={path} bars={} replay_bars={max_bars} first_date={} last_date={} verify={verify}",
        loaded.bars.len(),
        loaded.first_date,
        loaded.last_date
    );
    println!(
        "P409_MODE channel=pan_live_only structure_completed=false feed=every_prefix sparse=forest_epoch_gated"
    );

    let bars = &loaded.bars[..max_bars];
    let mut parser = ParseLayerIncr::new(&config);
    let mut cache = classifier::TowerCache::new();
    let mut book = NestLifecycleBook::new();

    let mut last_epoch: Option<u64> = None;
    let mut stems: Vec<WindowStem> = Vec::new();
    let mut stats: BTreeMap<u32, LevelStats> = BTreeMap::new();
    // 身份首见 prefix（去重计「产窗次数」）。
    let mut seen_stems: BTreeMap<WindowStem, usize> = BTreeMap::new();
    // 已进终态的身份（advance 第 3 步吸收 ⟹ 力度不再求值；探针同口径跳过）。
    let mut terminal_stems: std::collections::BTreeSet<WindowStem> =
        std::collections::BTreeSet::new();
    let mut emissions_total = 0usize;
    let mut verify_mismatches = 0usize;
    let mut force_xcheck_mismatches = 0usize;
    let mut recomputes = 0usize;
    let mut max_live_stems = 0usize;
    let started = Instant::now();

    for (index, bar) in bars.iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let (_classification, tower) =
            classifier::classify_with_tower_incremental(&l0, &config, &mut cache);
        let epoch = cache.forest_epoch();
        if last_epoch != Some(epoch) {
            last_epoch = Some(epoch);
            recomputes += 1;
            let mut fresh: Vec<WindowStem> = recompute_windows(&tower, index)
                .iter()
                .map(WindowStem::of)
                .collect();
            fresh.sort();
            fresh.dedup();
            stems = fresh;
        } else if verify {
            let mut fresh: Vec<WindowStem> = recompute_windows(&tower, index)
                .iter()
                .map(WindowStem::of)
                .collect();
            fresh.sort();
            fresh.dedup();
            if fresh != stems {
                verify_mismatches += 1;
                if verify_mismatches <= 10 {
                    eprintln!(
                        "P409_VERIFY_MISMATCH bar={index} cached={} fresh={}",
                        stems.len(),
                        fresh.len()
                    );
                }
                stems = fresh;
            }
        }
        max_live_stems = max_live_stems.max(stems.len());

        let (hist, close_src) = cache.causal_series();
        let dif = cache.macd_dif();
        let material = ForceMaterial {
            hist: Some(hist),
            dif: Some(dif),
            close_src,
        };

        let windows: Vec<PanLiveWindow> =
            stems.iter().map(|stem| stem.window_at(index)).collect();
        // 力度三值分布（逐观察计数；终态吸收的身份不再计——与 book 的求值点对齐）。
        let mut expected: BTreeMap<(u32, (usize, usize), usize), ForceTri> = BTreeMap::new();
        for (stem, window) in stems.iter().zip(windows.iter()) {
            let key = (stem.level, stem.seg_a, stem.c_start);
            // 终态吸收的身份 book 侧不再求值（advance 第 3 步 continue）⟹ 三值分布同口径跳过。
            if terminal_stems.contains(stem) {
                continue;
            }
            let tri = force_of(window, hist, dif, close_src);
            expected.insert(key, tri);
            let entry = stats.entry(stem.level).or_default();
            entry.emissions += 1;
            emissions_total += 1;
            match tri {
                ForceTri::True => entry.force_true += 1,
                ForceTri::False => entry.force_false += 1,
                ForceTri::MissingForceSeries => entry.missing_series += 1,
                ForceTri::CoordinateMapFailed => entry.coord_failed += 1,
            }
            seen_stems.entry(*stem).or_insert(index);
        }

        let observations: Vec<LifecycleObservation> = windows
            .iter()
            .map(|window| LifecycleObservation::pan_live(*window, false))
            .collect();
        let delta = book.advance(&observations, index, &material);

        // 交叉校验：book 的 revision 流与探针复算的三值一致 + 终态身份登记。
        for revision in &delta {
            let key = (revision.key.level, revision.key.seg_a, revision.key.seg_c_full.0);
            if matches!(
                revision.kind,
                LifecycleRevisionKind::Invalidated { .. } | LifecycleRevisionKind::Confirmed
            ) {
                terminal_stems.insert(WindowStem {
                    level: revision.key.level,
                    side_tag: side_tag(revision.key.side),
                    seg_a: revision.key.seg_a,
                    c_start: revision.key.seg_c_full.0,
                    b_center_start: revision.key.b_center_start,
                });
            }
            match revision.kind {
                LifecycleRevisionKind::ForceUnavailable { reason } => {
                    let want = match reason {
                        UnavailReason::MissingForceSeries => ForceTri::MissingForceSeries,
                        UnavailReason::CoordinateMapFailed => ForceTri::CoordinateMapFailed,
                    };
                    if expected.get(&key) != Some(&want) {
                        force_xcheck_mismatches += 1;
                    }
                }
                LifecycleRevisionKind::FirstProvable => {
                    if expected.get(&key) != Some(&ForceTri::True) {
                        force_xcheck_mismatches += 1;
                    }
                }
                LifecycleRevisionKind::Invalidated {
                    reason: InvalidatedReason::ForceOvertake,
                } => {
                    if expected.get(&key) != Some(&ForceTri::False) {
                        force_xcheck_mismatches += 1;
                    }
                }
                _ => {}
            }
        }

        if progress_every > 0 && index > 0 && index % progress_every == 0 {
            eprintln!(
                "P409_PROGRESS bar={index}/{max_bars} elapsed={:.1}s stems={} book={} emissions={emissions_total} recomputes={recomputes}",
                started.elapsed().as_secs_f64(),
                stems.len(),
                book.len(),
            );
            // 中途快照：全量汇总每 progress_every bar 落一次 ⟹ 超预算被中止时
            // 仍有「跑到第 N bar 的完整四组数字」，不静默截断（票面纪律）。
            emit_report(
                "PARTIAL",
                index + 1,
                &book,
                &stats,
                &seen_stems,
                emissions_total,
                recomputes,
                max_live_stems,
                verify_mismatches,
                force_xcheck_mismatches,
                started.elapsed().as_secs_f64(),
                None,
            );
        }
    }

    let dump = std::env::var("P409_DUMP")
        .ok()
        .and_then(|p| std::fs::File::create(p).ok())
        .map(std::io::BufWriter::new);
    emit_report(
        "FINAL",
        max_bars,
        &book,
        &stats,
        &seen_stems,
        emissions_total,
        recomputes,
        max_live_stems,
        verify_mismatches,
        force_xcheck_mismatches,
        started.elapsed().as_secs_f64(),
        dump,
    );
    book.assert_invariants();
    println!("P409_INVARIANTS ok");
    Ok(())
}

/// 汇总输出（PARTIAL/FINAL 同格式；PARTIAL 每 progress_every bar 落一次）。
#[allow(clippy::too_many_arguments)]
fn emit_report(
    tag: &str,
    bars_done: usize,
    book: &NestLifecycleBook,
    base: &BTreeMap<u32, LevelStats>,
    seen_stems: &BTreeMap<WindowStem, usize>,
    emissions_total: usize,
    recomputes: usize,
    max_live_stems: usize,
    verify_mismatches: usize,
    force_xcheck_mismatches: usize,
    elapsed: f64,
    mut dump: Option<std::io::BufWriter<std::fs::File>>,
) {
    let mut stats = base.clone();
    let mut first_provable_lag: BTreeMap<usize, usize> = BTreeMap::new();
    for (key, entry) in book.entries() {
        let level_stats = stats.entry(key.level).or_default();
        match entry.state {
            NestEventState::Provisional => level_stats.provisional_alive += 1,
            NestEventState::Confirmed => level_stats.confirmed += 1,
            NestEventState::Invalidated => match entry.invalidated_reason {
                Some(InvalidatedReason::ForceOvertake) => level_stats.force_overtake += 1,
                Some(InvalidatedReason::NeverConstituted) => level_stats.never_constituted += 1,
                Some(InvalidatedReason::IdentityVanished) => level_stats.identity_vanished += 1,
                None => {}
            },
        }
        if let Some(first) = entry.first_provable_at {
            level_stats.first_provable_entries += 1;
            *first_provable_lag
                .entry(first.saturating_sub(entry.observed_at))
                .or_default() += 1;
        }
        if let Some(sink) = dump.as_mut() {
            let _ = writeln!(
                sink,
                "{{\"level\":{},\"side\":\"{:?}\",\"seg_a\":[{},{}],\"c_start\":{},\"b_center_start\":{},\"state\":\"{:?}\",\"reason\":\"{:?}\",\"observed_at\":{},\"first_provable_at\":{:?},\"invalidated_at\":{:?},\"last_as_of\":{},\"revisions\":{}}}",
                key.level,
                key.side,
                key.seg_a.0,
                key.seg_a.1,
                key.seg_c_full.0,
                key.b_center_start,
                entry.state,
                entry.invalidated_reason,
                entry.observed_at,
                entry.first_provable_at,
                entry.invalidated_at,
                entry.last_as_of,
                entry.revisions.len()
            );
        }
    }
    for (stem, _) in seen_stems {
        stats.entry(stem.level).or_default().identities += 1;
    }
    println!(
        "P409_TOTALS[{tag}] bars_done={bars_done} emissions={emissions_total} identities={} book_entries={} recomputes={recomputes} max_live_stems={max_live_stems} verify_mismatches={verify_mismatches} force_xcheck_mismatches={force_xcheck_mismatches} retrograde={} elapsed_s={elapsed:.1}",
        seen_stems.len(),
        book.len(),
        book.retrograde_rejections().len()
    );
    for (level, s) in &stats {
        println!(
            "P409_LEVEL[{tag}] level={level} identities={} emissions={} force_true={} force_false={} unavail_missing={} unavail_coordmap={} invalid_force_overtake={} invalid_never_constituted={} invalid_identity_vanished={} confirmed={} provisional_alive={} entries_with_first_provable={}",
            s.identities,
            s.emissions,
            s.force_true,
            s.force_false,
            s.missing_series,
            s.coord_failed,
            s.force_overtake,
            s.never_constituted,
            s.identity_vanished,
            s.confirmed,
            s.provisional_alive,
            s.first_provable_entries
        );
    }
    let mut lag_line = String::new();
    for (lag, count) in &first_provable_lag {
        lag_line.push_str(&format!("{lag}:{count} "));
    }
    println!("P409_FIRST_PROVABLE_LAG_HIST[{tag}] {}", lag_line.trim_end());
    use std::io::Write as _;
    let _ = std::io::stdout().flush();
}

// ═══════════════════════════════════════════════════════════════════════════
// 数据加载（逐字抄 p123_fast_replay.rs:1550-1619 / tangency_probe.rs 同款）
// ═══════════════════════════════════════════════════════════════════════════

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
