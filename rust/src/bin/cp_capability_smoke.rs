//! #43 前置能力冒烟：只验证 `CandDeltaEvent` 的完整 `c_p` 证书承载能力。
//! 不执行区间套装配、不横比基线、不读取/裁定三个预注册样本。

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser;
use newchan_rust::theta_v0::types::{quantize, Bar, Timestamp};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

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
    let digits: String = date
        .chars()
        .take_while(|c| *c != '+')
        .filter(|c| c.is_ascii_digit())
        .take(14)
        .collect();
    digits.parse().unwrap_or(0)
}

fn load_bars(path: &PathBuf, tick_size: f64, limit: usize) -> Result<Vec<Bar>, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
    let raw: BarsJson = serde_json::from_str(&text)
        .map_err(|e| format!("{}: JSON 解析失败: {e}", path.display()))?;
    let n = raw
        .closes
        .len()
        .min(limit)
        .min(raw.opens.len())
        .min(raw.highs.len())
        .min(raw.lows.len())
        .min(raw.volumes.len())
        .min(raw.dates.len());
    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let (o, h, l, c, v) = (
            raw.opens[i],
            raw.highs[i],
            raw.lows[i],
            raw.closes[i],
            raw.volumes[i],
        );
        bars.push(Bar {
            source_index: i,
            timestamp: date_to_timestamp(&raw.dates[i]),
            open: quantize(o, tick_size),
            high: quantize(h, tick_size),
            low: quantize(l, tick_size),
            close: quantize(c, tick_size),
            volume: v as i64,
            untradable: h < o.max(c).max(l)
                || l > o.min(c).min(h)
                || o <= 0.0
                || h <= 0.0
                || l <= 0.0
                || c <= 0.0
                || v <= 0.0,
        });
    }
    Ok(bars)
}

fn run() -> Result<(), String> {
    let config = ThetaConfig::default();
    let max_bars = std::env::var("CP_SMOKE_MAX_BARS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000usize);
    let verbose_events = std::env::var("CP_SMOKE_VERBOSE").map_or(true, |value| value != "0");
    let verbose_p50_certificates = std::env::var("CP_SMOKE_P50_CERTS").is_ok();
    let p52_audit = std::env::var("CP_SMOKE_P52_AUDIT").is_ok();
    let batch_mode = std::env::var("CP_SMOKE_MODE").is_ok_and(|value| value == "batch");
    let root =
        std::env::var("CP_SMOKE_DATA_ROOT").unwrap_or_else(|_| "/tmp/codex-work-p7".to_string());
    let path = PathBuf::from(root).join("analysis/data_cache/btc_1m_full.json");
    let bars = load_bars(&path, config.tick.tick_size, max_bars)?;
    if bars.is_empty() {
        return Err("冒烟输入为空".to_string());
    }

    if p52_audit {
        classifier::cp_replay_diagnostics::enable();
    }
    let (l0, classification, tower, events) = if batch_mode {
        let l0 = parser::parse_layer(&bars, &config);
        let (classification, tower) = classifier::classify_with_tower(&l0, &config);
        let events = classifier::cand_delta_tower(&l0, &classification, &tower, &config);
        (l0, classification, tower, events)
    } else {
        let mut parser_incr = parser::ParseLayerIncr::new(&config);
        let mut tower_cache = classifier::TowerCache::new();
        let mut final_state = None;
        for bar in &bars {
            let l0 = parser_incr.append(*bar);
            let (classification, tower) =
                classifier::classify_with_tower_incremental(&l0, &config, &mut tower_cache);
            final_state = Some((l0, classification, tower));
        }
        let (l0, classification, tower) = final_state.expect("bars 非空");
        let events = classifier::cand_delta_tower_cached(
            &l0,
            &classification,
            &tower,
            &config,
            &tower_cache,
        );
        (l0, classification, tower, events)
    };
    let frontier_counters = classifier::cp_replay_diagnostics::snapshot();

    let mut total = 0usize;
    let mut stable_edges = 0usize;
    let mut snapshot_keys = HashSet::new();
    let mut terminal_keys = HashSet::new();
    let mut p46_snapshot_keys = HashSet::new();
    let mut p46_terminal_keys = HashSet::new();
    let mut p46_full_keys = HashSet::new();
    let mut p46_row20_keys = HashSet::new();
    let mut p46_row22_keys = HashSet::new();
    let mut p46_decomposition_keys = HashSet::new();
    let mut p46_all_keys = HashSet::new();
    let mut p46_all_by_level: [HashSet<_>; 5] = std::array::from_fn(|_| HashSet::new());
    let mut p46_snapshot_by_level: [HashSet<_>; 5] = std::array::from_fn(|_| HashSet::new());
    let mut p46_terminal_by_level: [HashSet<_>; 5] = std::array::from_fn(|_| HashSet::new());
    let mut p46_full_by_level: [HashSet<_>; 5] = std::array::from_fn(|_| HashSet::new());
    let mut p46_row20_by_level: [HashSet<_>; 5] = std::array::from_fn(|_| HashSet::new());
    let mut p46_row22_by_level: [HashSet<_>; 5] = std::array::from_fn(|_| HashSet::new());
    let mut p46_decomposition_by_level: [HashSet<_>; 5] = std::array::from_fn(|_| HashSet::new());
    let mut all_by_level: Vec<HashSet<_>> = (0..classification.levels.len())
        .map(|_| HashSet::new())
        .collect();
    let mut third_by_level: Vec<HashSet<_>> = (0..classification.levels.len())
        .map(|_| HashSet::new())
        .collect();
    let mut full_by_level: Vec<HashSet<_>> = (0..classification.levels.len())
        .map(|_| HashSet::new())
        .collect();
    let (mut eq, mut lt, mut gt) = (0usize, 0usize, 0usize);
    let (mut missing_start, mut missing_end) = (0usize, 0usize);
    for event in events.iter().flatten().filter(|e| e.cand_delta) {
        total += 1;
        let objects = &classification.levels[event.level as usize].cp_ownership;
        let snapshot = classifier::recursive_tower::cp_certificate_at_divergence(event);
        let terminal = classifier::recursive_tower::cp_terminal_certificate(event, objects);
        let edge_key = event
            .cp_ownership
            .map(|edge| (event.level, edge.b_center_id, edge.cp_departure_move_id));
        if let Some(key) = edge_key {
            stable_edges += 1;
            all_by_level[event.level as usize].insert(key);
            if snapshot.is_some() {
                snapshot_keys.insert(key);
            }
            if terminal.is_some() {
                terminal_keys.insert(key);
                third_by_level[event.level as usize].insert(key);
            }
            if terminal
                .as_ref()
                .is_some_and(|certificate| certificate.full_trend_c_qualified.is_some())
            {
                full_by_level[event.level as usize].insert(key);
            }
            if (1..=4).contains(&event.level) {
                p46_all_keys.insert(key);
                p46_all_by_level[event.level as usize].insert(key);
                if snapshot.is_some() {
                    p46_snapshot_keys.insert(key);
                    p46_snapshot_by_level[event.level as usize].insert(key);
                }
                if terminal.is_some() {
                    p46_terminal_keys.insert(key);
                    p46_terminal_by_level[event.level as usize].insert(key);
                }
                if terminal
                    .as_ref()
                    .is_some_and(|certificate| certificate.full_trend_c_qualified.is_some())
                {
                    p46_full_keys.insert(key);
                    p46_full_by_level[event.level as usize].insert(key);
                }
                if let Some(evidence) = terminal
                    .as_ref()
                    .and_then(|certificate| certificate.full_trend_evidence.as_ref())
                {
                    if evidence.new_extreme_in_direction.is_some() {
                        p46_row20_keys.insert(key);
                        p46_row20_by_level[event.level as usize].insert(key);
                    }
                    if evidence.internal_sublevel_centers.is_some() {
                        p46_row22_keys.insert(key);
                        p46_row22_by_level[event.level as usize].insert(key);
                    }
                    if evidence.completed_trend_decomposition.is_some() {
                        p46_decomposition_keys.insert(key);
                        p46_decomposition_by_level[event.level as usize].insert(key);
                    }
                }
            }
        }
        let start = event.c_structure.map(|c| c.source_start);
        let end = event.c_structure.and_then(|c| c.source_end);
        let relation = match start {
            Some(s) if s == event.c_episode_start => {
                eq += 1;
                "=="
            }
            Some(s) if s < event.c_episode_start => {
                lt += 1;
                "<"
            }
            Some(_) => {
                gt += 1;
                ">"
            }
            None => {
                missing_start += 1;
                "None"
            }
        };
        if end.is_none() {
            missing_end += 1;
        }
        if verbose_events {
            println!(
                "CP_EVENT level={} divergence_confirm_src={} B={:?} c_start_full={:?} c_episode_start={} snapshot_cp_confirm={:?} snapshot_end={:?} terminal_cp_confirm={:?} terminal_end={:?} relation={} stable_edge={}",
                event.level,
                event.divergence_confirm_src,
                event.b_parent.map(|b| (b.center_index, b.center_id, b.source_interval)),
                start,
                event.c_episode_start,
                event.cp_certificate_confirm_src,
                end,
                terminal.as_ref().map(|cert| cert.cp_certificate_confirm_src),
                terminal.as_ref().map(|cert| cert.c_interval_full.1),
                relation,
                event.cp_ownership.is_some(),
            );
        }
        if matches!(
            (event.level, event.divergence_confirm_src),
            (0, 42_704) | (1, 3_306_324)
        ) {
            println!(
                "P46_CASE level={} divergence_confirm_src={} snapshot_cp_confirm={:?} snapshot_end={:?} terminal_cp_confirm={:?} terminal_end={:?} stable_edge={}",
                event.level,
                event.divergence_confirm_src,
                event.cp_certificate_confirm_src,
                end,
                terminal.as_ref().map(|cert| cert.cp_certificate_confirm_src),
                terminal.as_ref().map(|cert| cert.c_interval_full.1),
                event.cp_ownership.is_some(),
            );
        }
        if verbose_p50_certificates && (1..=4).contains(&event.level) {
            let evidence = terminal
                .as_ref()
                .and_then(|certificate| certificate.full_trend_evidence.as_ref());
            let center_ids = evidence
                .and_then(|parts| parts.internal_sublevel_centers.as_ref())
                .map(|certificate| {
                    certificate
                        .center_ids
                        .iter()
                        .map(|id| format!("L{}#{}", id.level, id.ordinal))
                        .collect::<Vec<_>>()
                        .join("->")
                })
                .unwrap_or_else(|| "None".to_string());
            println!(
                "P50_CERT level={} divergence_confirm_src={} third_confirm={:?} row20_confirm={:?} row22_ids={} decomposition_confirm={:?} full_confirm={:?}",
                event.level,
                event.divergence_confirm_src,
                terminal.as_ref().map(|certificate| certificate.cp_certificate_confirm_src),
                evidence
                    .and_then(|parts| parts.new_extreme_in_direction.as_ref())
                    .map(|certificate| certificate.confirm_src),
                center_ids,
                evidence
                    .and_then(|parts| parts.completed_trend_decomposition.as_ref())
                    .map(|certificate| certificate.confirm_src),
                terminal
                    .as_ref()
                    .and_then(|certificate| certificate.full_trend_c_qualified.as_ref())
                    .map(|certificate| certificate.confirm_src),
            );
        }
    }
    println!(
        "CP_DISTRIBUTION bars={} parent_events={} stable_edges={} snapshot_objects={} terminal_objects={} terminal_delta={} relation_eq_lt_gt={}/{}/{} missing_start={} missing_end={}",
        bars.len(),
        total,
        stable_edges,
        snapshot_keys.len(),
        terminal_keys.len(),
        terminal_keys.len().saturating_sub(snapshot_keys.len()),
        eq,
        lt,
        gt,
        missing_start,
        missing_end
    );
    println!(
        "P50_BUCKETS event_time={} third_closed={} full_qualified={} classification_review={}",
        p46_all_keys.len(),
        p46_terminal_keys.len(),
        p46_full_keys.len(),
        p46_all_keys.len().saturating_sub(p46_terminal_keys.len())
    );
    println!(
        "P50_COMPONENTS third_closed={} row20_new_extreme={} row22_center_id_chain={} completed_trend_decomposition={}",
        p46_terminal_keys.len(),
        p46_row20_keys.len(),
        p46_row22_keys.len(),
        p46_decomposition_keys.len(),
    );
    for level in 1..=4 {
        println!(
            "P50_LEVEL level={} event_time={} third_closed={} full_qualified={} classification_review={}",
            level,
            p46_all_by_level[level].len(),
            p46_terminal_by_level[level].len(),
            p46_full_by_level[level].len(),
            p46_all_by_level[level]
                .len()
                .saturating_sub(p46_terminal_by_level[level].len())
        );
        println!(
            "P50_LEVEL_COMPONENTS level={} row20_new_extreme={} row22_center_id_chain={} completed_trend_decomposition={}",
            level,
            p46_row20_by_level[level].len(),
            p46_row22_by_level[level].len(),
            p46_decomposition_by_level[level].len(),
        );
    }
    for level in 0..all_by_level.len() {
        println!(
            "P50_ALL_LEVEL level={} event_time={} third_closed={} full_qualified={} classification_review={}",
            level,
            all_by_level[level].len(),
            third_by_level[level].len(),
            full_by_level[level].len(),
            all_by_level[level].len().saturating_sub(third_by_level[level].len())
        );
    }
    if p52_audit {
        let recall = classifier::cp_recall_upper_bound_audit(&l0, &classification, &tower);
        let legacy_true_keys: HashSet<_> = events
            .iter()
            .flatten()
            .filter(|event| event.cand_delta && (1..=4).contains(&event.level))
            .filter_map(|event| {
                event
                    .cp_ownership
                    .map(|edge| (event.level, edge.b_center_id, edge.cp_departure_move_id))
            })
            .collect();
        let legacy_record_keys: HashSet<_> = events
            .iter()
            .flatten()
            .filter(|event| (1..=4).contains(&event.level))
            .filter_map(|event| {
                event
                    .cp_ownership
                    .map(|edge| (event.level, edge.b_center_id, edge.cp_departure_move_id))
                    .or_else(|| {
                        event.c_structure.map(|structure| {
                            (
                                event.level,
                                structure.b_center_id,
                                structure.departure_move_id,
                            )
                        })
                    })
            })
            .collect();
        let relaxed_entries = classifier::cand_delta_entry_tower(&classification, &events);
        let event_keys: HashSet<_> = relaxed_entries
            .iter()
            .flatten()
            .filter(|entry| (1..=4).contains(&entry.level))
            .map(|entry| {
                (
                    entry.level,
                    entry.cp_ownership.b_center_id,
                    entry.cp_ownership.cp_departure_move_id,
                )
            })
            .collect();
        let entry_origins: HashMap<_, _> = relaxed_entries
            .iter()
            .flatten()
            .filter(|entry| (1..=4).contains(&entry.level))
            .map(|entry| {
                (
                    (
                        entry.level,
                        entry.cp_ownership.b_center_id,
                        entry.cp_ownership.cp_departure_move_id,
                    ),
                    entry.origin,
                )
            })
            .collect();
        let success_keys: HashSet<_> = recall
            .iter()
            .filter(|case| {
                (1..=4).contains(&case.level)
                    && case.atom == classifier::recursive_tower::CpRecallAtom::Success
            })
            .filter_map(|case| {
                case.cp_departure_move_id
                    .map(|departure| (case.level, case.b_center_id, departure))
            })
            .collect();
        let stable_objects = recall
            .iter()
            .filter(|case| (1..=4).contains(&case.level) && case.cp_departure_move_id.is_some())
            .count();
        let aligned_success = success_keys.intersection(&event_keys).count();
        let missing_keys: Vec<_> = success_keys.difference(&event_keys).copied().collect();
        let extra_keys: Vec<_> = event_keys.difference(&success_keys).copied().collect();
        if !missing_keys.is_empty() || !extra_keys.is_empty() {
            for (level, b, departure) in &missing_keys {
                println!(
                    "P53_EXCEPTION kind=MISSING_RELAXED_ENTRY level={} B=L{}#{} cp_departure=L{}#{}",
                    level, b.level, b.ordinal, departure.level, departure.ordinal
                );
            }
            for (level, b, departure) in &extra_keys {
                println!(
                    "P53_EXCEPTION kind=ENTRY_WITHOUT_GEOMETRY level={} B=L{}#{} cp_departure=L{}#{}",
                    level, b.level, b.ordinal, departure.level, departure.ordinal
                );
            }
            return Err(format!(
                "P53 入口集合与 judge_third_cert 上界不一致: missing={} extra={}",
                missing_keys.len(),
                extra_keys.len()
            ));
        }
        let existing_true = entry_origins
            .values()
            .filter(|origin| {
                **origin == classifier::recursive_tower::CandDeltaEntryOrigin::ExistingCandDeltaTrue
            })
            .count();
        let existing_false = entry_origins
            .values()
            .filter(|origin| {
                **origin
                    == classifier::recursive_tower::CandDeltaEntryOrigin::ExistingCandDeltaFalse
            })
            .count();
        let stable_geometry = entry_origins
            .values()
            .filter(|origin| {
                **origin == classifier::recursive_tower::CandDeltaEntryOrigin::StableCpGeometry
            })
            .count();
        let legacy_review_keys: HashSet<_> = legacy_true_keys
            .difference(&success_keys)
            .copied()
            .collect();
        println!(
            "P52_RECALL_SUMMARY levels=1-4 stable_objects={} upper_success={} event_objects={} aligned_success={} missed_candidates={} missed_cand_delta_false={} missed_before_event={} event_only_failures={}",
            stable_objects,
            success_keys.len(),
            event_keys.len(),
            aligned_success,
            missing_keys.len(),
            0,
            0,
            event_keys.difference(&success_keys).count(),
        );
        println!(
            "P53_ENTRY_SUMMARY old_e={} new_e={} existing_cand_delta_true={} existing_cand_delta_false={} stable_cp_geometry={} legacy_records={} legacy_review={}",
            legacy_true_keys.len(),
            event_keys.len(),
            existing_true,
            existing_false,
            stable_geometry,
            legacy_record_keys.len(),
            legacy_review_keys.len(),
        );

        for level in 1..=4_u32 {
            let level_cases: Vec<_> = recall
                .iter()
                .filter(|case| case.level == level && case.cp_departure_move_id.is_some())
                .collect();
            let level_events = event_keys.iter().filter(|key| key.0 == level).count();
            let level_success = success_keys.iter().filter(|key| key.0 == level).count();
            let level_aligned = success_keys
                .iter()
                .filter(|key| key.0 == level && event_keys.contains(key))
                .count();
            let mut atoms: BTreeMap<&str, usize> = BTreeMap::new();
            for case in &level_cases {
                *atoms.entry(case.atom.as_str()).or_default() += 1;
            }
            let atom_text = atoms
                .iter()
                .map(|(atom, count)| format!("{atom}:{count}"))
                .collect::<Vec<_>>()
                .join(",");
            println!(
                "P52_RECALL_LEVEL level={} stable_objects={} upper_success={} event_objects={} aligned_success={} missed={} missed_cand_delta_false={} missed_before_event={} event_failures={} atoms={}",
                level,
                level_cases.len(),
                level_success,
                level_events,
                level_aligned,
                level_success.saturating_sub(level_aligned),
                0,
                0,
                level_events.saturating_sub(level_aligned),
                atom_text,
            );
        }

        for case in recall.iter().filter(|case| (1..=4).contains(&case.level)) {
            let Some(cp_departure) = case.cp_departure_move_id else {
                continue;
            };
            let key = (case.level, case.b_center_id, cp_departure);
            let (alignment, entry_cause) = if let Some(origin) = entry_origins.get(&key) {
                ("EVENT", origin.as_str())
            } else {
                continue;
            };
            println!(
                "P52_CASE alignment={} entry_cause={} level={} B=L{}#{} B_source=[{},{}] B_core=[{},{}] cp_departure=L{}#{} cp_start={:?} leave_id={:?} leave_source={:?} retest_id={:?} retest_source={:?} atom={}",
                alignment,
                entry_cause,
                case.level,
                case.b_center_id.level,
                case.b_center_id.ordinal,
                case.b_source_interval.0,
                case.b_source_interval.1,
                case.b_core.0,
                case.b_core.1,
                cp_departure.level,
                cp_departure.ordinal,
                case.cp_source_start,
                case.departure_move_id.map(|id| (id.level, id.ordinal)),
                case.departure_interval,
                case.retest_move_id.map(|id| (id.level, id.ordinal)),
                case.retest_interval,
                case.atom.as_str(),
            );
        }

        for case in recall.iter().filter(|case| (1..=4).contains(&case.level)) {
            let Some(cp_departure) = case.cp_departure_move_id else {
                continue;
            };
            let key = (case.level, case.b_center_id, cp_departure);
            if !legacy_review_keys.contains(&key) {
                continue;
            }
            println!(
                "P53_LEGACY_REVIEW_CASE level={} B=L{}#{} cp_departure=L{}#{} atom={} status=ConsolidationReviewPending",
                case.level,
                case.b_center_id.level,
                case.b_center_id.ordinal,
                cp_departure.level,
                cp_departure.ordinal,
                case.atom.as_str(),
            );
        }

        let mut frontier_total = classifier::cp_replay_diagnostics::CpFrontierCounters::default();
        for (level, counter) in frontier_counters.iter().enumerate() {
            println!(
                "P53_FRONTIER level={} pending_fallbacks={} tail_reinherits={} certificate_clear_recomputes={}",
                level,
                counter.pending_fallbacks,
                counter.tail_reinherits,
                counter.certificate_clear_recomputes,
            );
            frontier_total.pending_fallbacks += counter.pending_fallbacks;
            frontier_total.tail_reinherits += counter.tail_reinherits;
            frontier_total.certificate_clear_recomputes += counter.certificate_clear_recomputes;
        }
        println!(
            "P53_FRONTIER_TOTAL pending_fallbacks={} tail_reinherits={} certificate_clear_recomputes={}",
            frontier_total.pending_fallbacks,
            frontier_total.tail_reinherits,
            frontier_total.certificate_clear_recomputes,
        );
        classifier::cp_replay_diagnostics::disable();
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("cp_capability_smoke 失败: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
