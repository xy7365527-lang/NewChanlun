//! #550 真实数据事件通道电池。
//!
//! 用法：
//! `cargo run --release --bin issue550_event_battery -- <btc_1m_full.json> [max_bars]`

use newchan_rust::theta_v0::classifier::cand_event::{
    CandidateEvent, CandidateKey, CandidateKind, CandidateState,
};
use newchan_rust::theta_v0::classifier::streaming::OwnedIncrementalClassifier;
use newchan_rust::theta_v0::classifier::{self, TowerCache};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{quantize, Bar};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;

#[derive(Deserialize)]
struct RawBars {
    opens: Vec<Option<f64>>,
    highs: Vec<Option<f64>>,
    lows: Vec<Option<f64>>,
    closes: Vec<Option<f64>>,
    #[serde(default)]
    volumes: Vec<Option<f64>>,
    dates: Vec<String>,
}

fn timestamp(date: &str) -> i64 {
    date.chars()
        .take_while(|c| *c != '+')
        .filter(char::is_ascii_digit)
        .take(14)
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

fn load(path: &Path, tick_size: f64, limit: usize) -> Result<Vec<Bar>, String> {
    let raw = load_raw(path)?;
    Ok(raw_to_bars(raw, tick_size, limit))
}

fn load_raw(path: &Path) -> Result<RawBars, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    serde_json::from_str(&text).map_err(|error| format!("解析 {} 失败: {error}", path.display()))
}

fn raw_to_bars(raw: RawBars, tick_size: f64, limit: usize) -> Vec<Bar> {
    let n = raw
        .closes
        .len()
        .min(raw.opens.len())
        .min(raw.highs.len())
        .min(raw.lows.len())
        .min(raw.dates.len())
        .min(limit);
    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let prior = bars.last().map_or(0, |bar: &Bar| bar.close);
        bars.push(raw_bar(&raw, i, prior, tick_size));
    }
    bars
}

fn raw_bar(raw: &RawBars, i: usize, prior: i64, tick_size: f64) -> Bar {
    let volume = raw.volumes.get(i).and_then(|value| *value).unwrap_or(0.0);
    let values = match (raw.opens[i], raw.highs[i], raw.lows[i], raw.closes[i]) {
        (Some(open), Some(high), Some(low), Some(close)) => {
            let invalid = high < open.max(close).max(low)
                || low > open.min(close).min(high)
                || [open, high, low, close].iter().any(|price| *price <= 0.0);
            (
                quantize(open, tick_size),
                quantize(high, tick_size),
                quantize(low, tick_size),
                quantize(close, tick_size),
                invalid,
            )
        }
        _ => (prior, prior, prior, prior, true),
    };
    Bar {
        source_index: i,
        timestamp: timestamp(&raw.dates[i]),
        open: values.0,
        high: values.1,
        low: values.2,
        close: values.3,
        volume: volume as i64,
        untradable: values.4 || volume <= 0.0,
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: issue550_event_battery <btc_1m_full.json> [max_bars]")?;
    let max_bars = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| format!("max_bars 非法: {error}"))?
        .unwrap_or(100_000);
    let config = ThetaConfig::default();
    let bars = load(Path::new(&path), config.tick.tick_size, max_bars)?;
    if bars.is_empty() {
        return Err("输入窗口为空".to_string());
    }

    let terminal_streams = compare_streams(&bars, &config)?;
    print_summary(&bars, &terminal_streams);
    print_lifecycle_summary(&terminal_streams);
    print_projection_fork(&bars, &config, &terminal_streams)?;
    Ok(())
}

/// #551：因果簿生命史分解——四态构成、修订类别、首证钟落位。
///
/// 与 `print_summary` 的「每 key 最新 revision」口径互补：本节读**全部** revision，使
/// 「转移/修订/钟路径是否在真实数据上真被触发」机器可见（防真空绿）。
fn print_lifecycle_summary(streams: &classifier::cand_event::CandidateStreams) {
    let mut latest = BTreeMap::<CandidateKey, &CandidateEvent>::new();
    let mut history = BTreeMap::<CandidateKey, Vec<&CandidateEvent>>::new();
    for stream in streams.iter() {
        for event in stream.iter() {
            latest.insert(event.key, event);
            history.entry(event.key).or_default().push(event);
        }
    }

    let mut by_state = BTreeMap::<&'static str, usize>::new();
    let mut trend_by_state = BTreeMap::<&'static str, usize>::new();
    let mut with_clock = 0usize;
    for event in latest.values() {
        *by_state.entry(state_name(event.state)).or_default() += 1;
        if event.kind == CandidateKind::Trend {
            *trend_by_state.entry(state_name(event.state)).or_default() += 1;
        }
        if event.first_provable_at.is_some() {
            with_clock += 1;
        }
    }

    let (mut growth, mut payload, mut transition, mut invalidation) = (0usize, 0usize, 0usize, 0);
    for revisions in history.values() {
        for pair in revisions.windows(2) {
            let (prior, next) = (pair[0], pair[1]);
            if next.state == CandidateState::Invalidated && prior.state != next.state {
                invalidation += 1;
            } else if prior.state != next.state {
                transition += 1;
            } else if next.interval.1 > prior.interval.1 {
                growth += 1;
            } else {
                payload += 1;
            }
        }
    }
    if std::env::var_os("ISSUE551_DIAG").is_some() {
        for event in latest.values() {
            if event.state == CandidateState::Unresolved || event.kind == CandidateKind::Trend {
                println!(
                    "ISSUE551_DIAG trend kind={:?} state={:?} preds={:?} interval={:?} first_provable={:?} revision={} level={}",
                    event.kind, event.state, event.structural_predicates, event.interval,
                    event.first_provable_at, event.revision, event.key.level
                );
            }
        }
    }
    println!(
        "ISSUE551_LIFECYCLE states={by_state:?} trend_states={trend_by_state:?} \
         with_first_provable={with_clock} growth_revisions={growth} payload_revisions={payload} \
         transitions={transition} invalidations={invalidation}"
    );
}

fn state_name(state: CandidateState) -> &'static str {
    match state {
        CandidateState::Provisional => "Provisional",
        CandidateState::Unresolved => "Unresolved",
        CandidateState::Confirmed => "Confirmed",
        CandidateState::Invalidated => "Invalidated",
    }
}

/// #551 裁定(i) 的真实数据版：末态 fresh 全量流 vs 因果簿终态投影。
///
/// 只跑**一次** fresh 全量（O(n)，非逐前缀 O(n²)）。两侧按 key 对齐后分类报数。
///
/// 口径以裁定(i) 的**甲口径定稿**为准（编排者 2026-07-28 裁决，见
/// `chanlun/review-results/issue551-t2-impl-20260728.md` §五）：
/// - `fork_causal_only`：因果簿有而 fresh 无——终态语义的定义后果（中途消失 ⟹ Invalidated
///   在案），计量项；
/// - `revived`：因果簿已判终态却仍被 fresh 产出（复活型分叉）——**禁令已随裁决取消**，现为
///   终态域计量项，非零不是红；
/// - `fork_fresh_only`：fresh 有而因果簿无——**仍是禁止方向**，非零即为红。它不在终态域豁免内：
///   因果簿是正本，fresh 侧凭空多出从未入簿的身份 = 增量宿主漏记，是实装缺陷而非语义后果；
/// - `payload_differ_live`：非终态候选的载荷差异——甲口径的**无条件域**，非零即为红。
fn print_projection_fork(
    bars: &[Bar],
    config: &ThetaConfig,
    causal: &classifier::cand_event::CandidateStreams,
) -> Result<(), String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut l0 = parser.append(bars[0]);
    for bar in bars.iter().copied().skip(1) {
        l0 = parser.append(bar);
    }
    let fresh = classifier::classify_with_tower_events(&l0, config).2;

    let mut terminal = BTreeMap::<CandidateKey, &CandidateEvent>::new();
    for stream in causal.iter() {
        for event in stream.iter() {
            terminal.insert(event.key, event);
        }
    }
    let mut fresh_latest = BTreeMap::<CandidateKey, &CandidateEvent>::new();
    for stream in fresh.iter() {
        for event in stream.iter() {
            fresh_latest.insert(event.key, event);
        }
    }

    let mut payload_equal = 0usize;
    let mut payload_differ = 0usize;
    let mut payload_differ_live = 0usize;
    let mut revived = 0usize;
    let mut fork_fresh_only = 0usize;
    for (key, event) in &fresh_latest {
        match terminal.get(key) {
            None => fork_fresh_only += 1,
            Some(booked) if booked.state == CandidateState::Invalidated => revived += 1,
            Some(booked) => {
                if projection_of(booked) == projection_of(event) {
                    payload_equal += 1;
                } else {
                    payload_differ += 1;
                    if !booked.state.is_terminal() {
                        payload_differ_live += 1;
                    }
                }
            }
        }
    }
    let causal_only: Vec<_> = terminal
        .iter()
        .filter(|(key, _)| !fresh_latest.contains_key(key))
        .collect();
    let fork_causal_only = causal_only.len();
    let fork_causal_only_terminal = causal_only
        .iter()
        .filter(|(_, event)| event.state.is_terminal())
        .count();
    if std::env::var_os("ISSUE551_DIAG").is_some() {
        for (key, event) in causal_only.iter().take(4) {
            println!("ISSUE551_DIAG causal_only key={key:?} state={:?} interval={:?} observed_at={} revision={}",
                event.state, event.interval, event.observed_at, event.revision);
        }
        for (key, event) in &fresh_latest {
            if let Some(booked) = terminal.get(key) {
                if projection_of(booked) != projection_of(event) {
                    println!(
                        "ISSUE551_DIAG payload_differ key={key:?}\n  causal  state={:?} interval={:?} preds={:?} revision={}\n  fresh   state={:?} interval={:?} preds={:?}",
                        booked.state, booked.interval, booked.structural_predicates, booked.revision,
                        event.state, event.interval, event.structural_predicates
                    );
                }
            }
        }
    }
    println!(
        "ISSUE551_FORK fresh_identities={} causal_identities={} payload_equal={payload_equal} \
         payload_differ={payload_differ} payload_differ_live={payload_differ_live} \
         fork_causal_only={fork_causal_only} fork_causal_only_terminal={fork_causal_only_terminal} \
         fork_fresh_only={fork_fresh_only} revived={revived}",
        fresh_latest.len(),
        terminal.len(),
    );
    Ok(())
}

/// 业务载荷投影（钟与 revision 计数属生命史，不入等价比较）。
type Projection = (
    CandidateKind,
    u32,
    Option<(usize, usize)>,
    u64,
    u64,
    (bool, bool, bool),
    (usize, usize),
    Option<usize>,
    (usize, usize),
    CandidateState,
);

fn projection_of(event: &CandidateEvent) -> Projection {
    (
        event.kind,
        event.event_level,
        event.center_ids,
        event.candidate_group_id,
        event.pair_id,
        (
            event.structural_predicates.direction,
            event.structural_predicates.comparable,
            event.structural_predicates.extreme,
        ),
        event.extreme_proof,
        event.third_class_proof,
        event.interval,
        event.state,
    )
}

fn compare_streams(
    bars: &[Bar],
    config: &ThetaConfig,
) -> Result<classifier::cand_event::CandidateStreams, String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = TowerCache::new();
    let mut owned = OwnedIncrementalClassifier::new(config.clone());
    let mut terminal_streams = Rc::new(Vec::new());
    for (i, bar) in bars.iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let direct = classifier::classify_with_tower_events_incremental(&l0, config, &mut cache);
        let streamed = owned.append_bar_events(bar);
        if direct != streamed {
            return Err(format!("逐 bar 三元通道不等: bar={i}"));
        }
        terminal_streams = streamed.2;
    }
    Ok(terminal_streams)
}

fn print_summary(bars: &[Bar], terminal_streams: &classifier::cand_event::CandidateStreams) {
    let mut latest = BTreeMap::<CandidateKey, &CandidateEvent>::new();
    let mut revisions = 0usize;
    for stream in terminal_streams.iter() {
        for event in stream.iter() {
            revisions += 1;
            latest.insert(event.key, event);
        }
    }
    let identities = latest.len();
    let mut active_by_level = BTreeMap::<u32, usize>::new();
    let mut active = 0usize;
    let mut pan_identities = 0usize;
    let mut pan_active = 0usize;
    for event in latest.values() {
        if event.kind == CandidateKind::Pan {
            pan_identities += 1;
        }
        if event.state != CandidateState::Invalidated {
            active += 1;
            *active_by_level.entry(event.key.level).or_default() += 1;
            if event.kind == CandidateKind::Pan {
                pan_active += 1;
            }
        }
    }
    println!(
        "ISSUE550_BATTERY bars={} compared={} stream_levels={} identities={} revisions={} active={} levels={:?} pan_identities={} pan_active={}",
        bars.len(),
        bars.len(),
        terminal_streams.len(),
        identities,
        revisions,
        active,
        active_by_level,
        pan_identities,
        pan_active,
    );
    if std::env::var_os("ISSUE550_VERBOSE").is_some() {
        println!("ISSUE550_LATEST {latest:#?}");
    }
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ISSUE550_BATTERY_ERROR {error}");
            std::process::ExitCode::from(1)
        }
    }
}
