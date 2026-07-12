//! #43 前置能力冒烟：只验证 `CandDeltaEvent` 的完整 `c_p` 证书承载能力。
//! 不执行区间套装配、不横比基线、不读取/裁定三个预注册样本。

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser;
use newchan_rust::theta_v0::types::{quantize, Bar, Timestamp};
use serde::Deserialize;
use std::collections::HashSet;
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
    let batch_mode = std::env::var("CP_SMOKE_MODE").is_ok_and(|value| value == "batch");
    let root =
        std::env::var("CP_SMOKE_DATA_ROOT").unwrap_or_else(|_| "/tmp/codex-work-p7".to_string());
    let path = PathBuf::from(root).join("analysis/data_cache/btc_1m_full.json");
    let bars = load_bars(&path, config.tick.tick_size, max_bars)?;
    if bars.is_empty() {
        return Err("冒烟输入为空".to_string());
    }

    let (classification, events) = if batch_mode {
        let l0 = parser::parse_layer(&bars, &config);
        let (classification, tower) = classifier::classify_with_tower(&l0, &config);
        let events = classifier::cand_delta_tower(&l0, &classification, &tower, &config);
        (classification, events)
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
        (classification, events)
    };

    let mut total = 0usize;
    let mut stable_edges = 0usize;
    let mut snapshot_keys = HashSet::new();
    let mut terminal_keys = HashSet::new();
    let mut p46_snapshot_keys = HashSet::new();
    let mut p46_terminal_keys = HashSet::new();
    let mut p46_all_keys = HashSet::new();
    let mut p46_all_by_level: [HashSet<_>; 5] = std::array::from_fn(|_| HashSet::new());
    let mut p46_snapshot_by_level: [HashSet<_>; 5] = std::array::from_fn(|_| HashSet::new());
    let mut p46_terminal_by_level: [HashSet<_>; 5] = std::array::from_fn(|_| HashSet::new());
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
            if snapshot.is_some() {
                snapshot_keys.insert(key);
            }
            if terminal.is_some() {
                terminal_keys.insert(key);
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
                terminal.map(|cert| cert.cp_certificate_confirm_src),
                terminal.map(|cert| cert.c_interval_full.1),
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
                terminal.map(|cert| cert.cp_certificate_confirm_src),
                terminal.map(|cert| cert.c_interval_full.1),
                event.cp_ownership.is_some(),
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
        "P46_L1_L4 objects={} snapshot_closed={} terminal_closed={} terminal_pending={}",
        p46_all_keys.len(),
        p46_snapshot_keys.len(),
        p46_terminal_keys.len(),
        p46_all_keys.len().saturating_sub(p46_terminal_keys.len())
    );
    for level in 1..=4 {
        println!(
            "P46_LEVEL level={} objects={} snapshot_closed={} terminal_closed={} terminal_pending={}",
            level,
            p46_all_by_level[level].len(),
            p46_snapshot_by_level[level].len(),
            p46_terminal_by_level[level].len(),
            p46_all_by_level[level]
                .len()
                .saturating_sub(p46_terminal_by_level[level].len())
        );
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
