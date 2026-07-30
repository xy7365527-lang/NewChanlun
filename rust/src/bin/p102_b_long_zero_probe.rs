//! task #102 归因探针：B 口径 Long 侧多级（top≥2, exec=1）nest 证书 0 张。
//!
//! 用法：
//! `cargo run --release --bin p102_b_long_zero_probe -- <btc_1m_full.json>`
//!
//! 只读复核，不改任何生产源码。从原始 bar 重建终态候选事件集（与 p92 终态同管线），
//! 然后独立回答三件事：
//! 1. A 独有的 5 张 exec=1 多级 Long 链，在 B 口径下到底是哪扇门失败（⊆ / Cand / 基例确认）；
//! 2. 失败是 DFS 顺序假象还是候选集事实（枚举父级全池，逐个判定 is_sub）；
//! 3. 门对 Long 是否有结构性不利（两侧对称性：候选池、区间宽度、可嵌率对照）。

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CoordinateWindow,
    LevelViewMaterial, LevelViewQuery, NestCandidateEvent, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::nest::{
    assemble_typed_certificates, is_sub, NestInterval, NestIntervalCaliber, TypedNestCertificate,
};
use newchan_rust::theta_v0::classifier::recursive_tower::LeveledMove;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::{ParseLayer, ParseLayerIncr};
use newchan_rust::theta_v0::types::{quantize, Bar, BspBits, Side, Timestamp};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

fn main() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or("用法: p102_b_long_zero_probe <btc_1m_full.json>")?;
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P102_MAX_BARS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |v| v.min(loaded.bars.len()));
    println!(
        "P102_INPUT bars={} replay_bars={} last_date={}",
        loaded.bars.len(),
        max_bars,
        loaded.last_date
    );

    let terminal = run_terminal_pass(&loaded.bars[..max_bars], &config)?;
    let (hist, close_src) = terminal.cache.causal_series();
    let dif = terminal.cache.macd_dif();
    let (events, proj_errors) =
        collect_terminal_events(&terminal.tower, max_bars - 1, hist, dif, close_src)?;
    println!(
        "P102_EVENTS levels={} total={} projection_errors={}",
        events.len(),
        events.iter().map(Vec::len).sum::<usize>(),
        proj_errors
    );

    // ── ① 候选池对称性：层级×方向 计数 / 确认数 / 区间宽度 p50 ──
    for level in 1..events.len() {
        for side in [Side::Long, Side::Short] {
            let pool: Vec<&NestCandidateEvent> =
                events[level].iter().filter(|e| e.side == side).collect();
            if pool.is_empty() {
                println!("P102_POOL level={level} side={side:?} candidates=0");
                continue;
            }
            let confirmed = pool.iter().filter(|e| e.divergence_confirmed).count();
            let mut wb: Vec<usize> = pool
                .iter()
                .map(|e| e.interval_b.1 - e.interval_b.0)
                .collect();
            wb.sort_unstable();
            println!(
                "P102_POOL level={} side={:?} candidates={} confirmed={} b_width_p50={}",
                level,
                side,
                pool.len(),
                confirmed,
                wb[wb.len() / 2]
            );
        }
    }

    // ── ② 复现双口径证书集（与 p92 observe_snapshot 同 dedup 键）──
    let mut seen_a = BTreeSet::new();
    let mut seen_b = BTreeSet::new();
    let mut certs_a: Vec<(usize, usize, TypedNestCertificate)> = Vec::new();
    let mut certs_b: Vec<(usize, usize, TypedNestCertificate)> = Vec::new();
    for exec in 1..events.len() {
        for top in exec..events.len() {
            for cert in
                assemble_typed_certificates(&events, exec, top, NestIntervalCaliber::A, |e| {
                    terminal_bits_new(&terminal.classification, e)
                })
            {
                if seen_a.insert(certificate_key(exec, top, &cert)) {
                    certs_a.push((exec, top, cert));
                }
            }
            for cert in
                assemble_typed_certificates(&events, exec, top, NestIntervalCaliber::B, |e| {
                    terminal_bits_new(&terminal.classification, e)
                })
            {
                if seen_b.insert(certificate_key(exec, top, &cert)) {
                    certs_b.push((exec, top, cert));
                }
            }
        }
    }
    let count = |set: &Vec<(usize, usize, TypedNestCertificate)>, side: Side, multi: bool| {
        set.iter()
            .filter(|(exec, top, c)| {
                c.certificate().side() == side && (!multi || (*exec == 1 && *top > 1))
            })
            .count()
    };
    println!(
        "P102_CERT caliber=A certs={} long={} long_exec1_multi={}",
        certs_a.len(),
        count(&certs_a, Side::Long, false),
        count(&certs_a, Side::Long, true)
    );
    println!(
        "P102_CERT caliber=B certs={} long={} long_exec1_multi={}",
        certs_b.len(),
        count(&certs_b, Side::Long, false),
        count(&certs_b, Side::Long, true)
    );
    for (caliber, set) in [("A", &certs_a), ("B", &certs_b)] {
        for (exec, top, cert) in set {
            let ids = cert
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
            println!(
                "P102_CERTLINE caliber={} exec={} top={} side={:?} ids={}",
                caliber,
                exec,
                top,
                cert.certificate().side(),
                ids
            );
        }
    }

    // ── ③ A 独有 exec=1 多级 Long 链：逐边枚举父级全池 ──
    let b_keys: BTreeSet<String> = certs_b
        .iter()
        .map(|(exec, top, c)| format!("{exec}:{top}:{}", ids_key(c)))
        .collect();
    let mut edge_rows = 0usize;
    for (exec, top, cert) in &certs_a {
        if *exec != 1 || *top <= 1 || cert.certificate().side() != Side::Long {
            continue;
        }
        if b_keys.contains(&format!("{exec}:{top}:{}", ids_key(cert))) {
            continue; // 非 A 独有
        }
        let ids = cert.identities();
        for k in 0..ids.len() - 1 {
            let parent_id = ids[k];
            let child_id = ids[k + 1];
            let child_level = child_id.level as usize;
            let parent_level = parent_id.level as usize;
            let Some(child) = events[child_level].iter().find(|e| {
                e.turn_source == child_id.turn_source && e.interval_b == child_id.interval_b
            }) else {
                println!("P102_EDGE_WARN child not found: {child_id:?}");
                continue;
            };
            let child_a = typed_iv(child, NestIntervalCaliber::A);
            let child_b = typed_iv(child, NestIntervalCaliber::B);
            let pool: Vec<&NestCandidateEvent> = events[parent_level]
                .iter()
                .filter(|e| e.side == Side::Long)
                .collect();
            let mut sub_a = 0usize;
            let mut sub_b = 0usize;
            println!(
                "P102_EDGE chain_base={} edge=L{}->L{} child_b=({}, {}) child_a=({}, {}) pool_long_L{}={}",
                ids.last().map(|i| i.turn_source).unwrap_or(0),
                parent_level,
                child_level,
                child_b.start_time,
                child_b.end_time,
                child_a.start_time,
                child_a.end_time,
                parent_level,
                pool.len()
            );
            for cand in pool {
                let pa = typed_iv(cand, NestIntervalCaliber::A);
                let pb = typed_iv(cand, NestIntervalCaliber::B);
                let ok_a = is_sub(&child_a, &pa);
                let ok_b = is_sub(&child_b, &pb);
                sub_a += usize::from(ok_a);
                sub_b += usize::from(ok_b);
                if ok_a || cand.turn_source == parent_id.turn_source {
                    let left_gap = child_b.start_time as i64 - pb.start_time as i64;
                    let right_gap = pb.end_time as i64 - child_b.end_time as i64;
                    let shape = if child_b.start_time > pb.end_time {
                        "child_right_of_parent"
                    } else if child_b.end_time < pb.start_time {
                        "child_left_of_parent"
                    } else {
                        "overlap_not_contained"
                    };
                    println!(
                        "P102_CAND parent={}:{}-{} chosen={} sub_A={} sub_B={} parent_b=({}, {}) parent_a=({}, {}) left_gap={} right_gap={} shape={}",
                        cand.turn_source,
                        cand.interval_b.0,
                        cand.interval_b.1,
                        cand.turn_source == parent_id.turn_source,
                        ok_a,
                        ok_b,
                        pb.start_time,
                        pb.end_time,
                        pa.start_time,
                        pa.end_time,
                        left_gap,
                        right_gap,
                        shape
                    );
                }
            }
            println!(
                "P102_EDGE_SUM edge=L{}->L{} parents_sub_A={} parents_sub_B={}",
                parent_level, child_level, sub_a, sub_b
            );
            edge_rows += 1;
        }
    }
    println!("P102_EDGES analyzed={edge_rows}");

    // ── ④ Short 对照：B 下唯一存活的 exec=1 多级链 ──
    for (exec, top, cert) in &certs_b {
        if *exec != 1 || *top <= 1 || cert.certificate().side() != Side::Short {
            continue;
        }
        let ids = cert.identities();
        for k in 0..ids.len() - 1 {
            let child_id = ids[k + 1];
            let child_level = child_id.level as usize;
            let parent_level = ids[k].level as usize;
            let Some(child) = events[child_level].iter().find(|e| {
                e.turn_source == child_id.turn_source && e.interval_b == child_id.interval_b
            }) else {
                continue;
            };
            let child_b = typed_iv(child, NestIntervalCaliber::B);
            let parents_b = events[parent_level]
                .iter()
                .filter(|e| {
                    e.side == Side::Short && is_sub(&child_b, &typed_iv(e, NestIntervalCaliber::B))
                })
                .count();
            println!(
                "P102_CONTROL_SHORT edge=L{}->L{} child_b=({}, {}) parents_sub_B={}",
                parent_level, child_level, child_b.start_time, child_b.end_time, parents_b
            );
        }
    }

    // ── ⑤ 池级可嵌率：每个合格基例在同侧上一级是否存在 A/B 父级 ──
    for level in 1..events.len() - 1 {
        for side in [Side::Long, Side::Short] {
            let mut bases = 0usize;
            let mut with_a = 0usize;
            let mut with_b = 0usize;
            for base in events[level].iter().filter(|e| e.side == side) {
                if !base.divergence_confirmed {
                    continue;
                }
                if terminal_bits_new(&terminal.classification, base).is_none() {
                    continue;
                }
                bases += 1;
                let ca = typed_iv(base, NestIntervalCaliber::A);
                let cb = typed_iv(base, NestIntervalCaliber::B);
                let ok_a = events[level + 1]
                    .iter()
                    .any(|e| e.side == side && is_sub(&ca, &typed_iv(e, NestIntervalCaliber::A)));
                let ok_b = events[level + 1]
                    .iter()
                    .any(|e| e.side == side && is_sub(&cb, &typed_iv(e, NestIntervalCaliber::B)));
                with_a += usize::from(ok_a);
                with_b += usize::from(ok_b);
            }
            println!(
                "P102_NESTABLE L{}->L{} side={:?} eligible_bases={} with_A_parent={} with_B_parent={}",
                level + 1,
                level,
                side,
                bases,
                with_a,
                with_b
            );
        }
    }
    Ok(())
}

fn typed_iv(event: &NestCandidateEvent, caliber: NestIntervalCaliber) -> NestInterval {
    let (start, end) = match caliber {
        NestIntervalCaliber::A => event.interval_a,
        NestIntervalCaliber::B => event.interval_b,
    };
    NestInterval {
        start_time: start as u64,
        end_time: end as u64,
        idx: event.turn_source as u64,
    }
}

fn ids_key(cert: &TypedNestCertificate) -> String {
    cert.identities()
        .iter()
        .map(|id| {
            format!(
                "{}:{}:{}-{}",
                id.level, id.turn_source, id.interval_b.0, id.interval_b.1
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn certificate_key(exec: usize, top: usize, certificate: &TypedNestCertificate) -> String {
    let cert = certificate.certificate();
    let rungs = cert
        .rungs()
        .iter()
        .map(|rung| (rung.interval(), rung.child_interval()))
        .collect::<Vec<_>>();
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

fn terminal_bits_new(
    classification: &classifier::Classification,
    event: &NestCandidateEvent,
) -> Option<BspBits> {
    // issue #747 C1 单源：改调 classifier::bsp::bind_turn（生产绑定规则原型 nest.rs:681 语义，
    // bin 侧诊断复制点收敛，见 issue747-impl 报告）。
    classifier::bsp::bind_turn(
        &classification.levels.get(event.level as usize)?.bsp,
        event.turn_source,
        event.side,
    )
    .map(|point| point.bits)
}

struct TerminalState {
    l0: ParseLayer,
    classification: classifier::Classification,
    tower: Vec<Rc<Vec<LeveledMove>>>,
    cache: classifier::TowerCache,
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
                "P102_PROGRESS bar={index}/{} elapsed={:.1}s",
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

/// 与 p92 `collect_snapshot_candidates` 同管线：终态 tower → 逐 run 投影 → C2 视图 → 候选事件。
/// 返回（按级事件集, 投影错误数）。judge_at 保持快照原值（本探针不消费首见钟）。
fn collect_terminal_events(
    tower: &[Rc<Vec<LeveledMove>>],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
) -> Result<(Vec<Vec<NestCandidateEvent>>, usize), String> {
    let mut errors = 0usize;
    let mut by_level = vec![Vec::new(); tower.len()];
    for level in 1..tower.len() {
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
        let windows = &tower[level];
        let mut run_start = None;
        for index in 0..=windows.len() {
            let valid = index < windows.len()
                && match project_extended_windows_carried_only(std::slice::from_ref(
                    &windows[index],
                )) {
                    Ok(_) => true,
                    Err(_) => {
                        errors += 1;
                        false
                    }
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
                            start: projection.seeds.first().expect("nonempty run").start_index,
                            end: projection.seeds.last().expect("nonempty run").end_index,
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
        events.sort_by_key(|event| {
            (
                event.level,
                matches!(event.side, Side::Short),
                event.kind,
                event.seg_a,
                event.interval_b,
                event.interval_a,
                event.turn_source,
            )
        });
        events.dedup_by(|left, right| {
            (
                left.level,
                left.side,
                left.kind,
                left.seg_a,
                left.interval_b,
                left.interval_a,
                left.turn_source,
            ) == (
                right.level,
                right.side,
                right.kind,
                right.seg_a,
                right.interval_b,
                right.interval_a,
                right.turn_source,
            )
        });
    }
    Ok((by_level, errors))
}

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
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
