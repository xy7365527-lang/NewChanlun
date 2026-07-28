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
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    let raw: RawBars = serde_json::from_str(&text)
        .map_err(|error| format!("解析 {} 失败: {error}", path.display()))?;
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
        let volume = raw.volumes.get(i).and_then(|value| *value).unwrap_or(0.0);
        let prior = bars.last().map_or(0, |bar: &Bar| bar.close);
        let (open, high, low, close, invalid) =
            match (raw.opens[i], raw.highs[i], raw.lows[i], raw.closes[i]) {
                (Some(open), Some(high), Some(low), Some(close)) => {
                    let invalid = high < open.max(close).max(low)
                        || low > open.min(close).min(high)
                        || open <= 0.0
                        || high <= 0.0
                        || low <= 0.0
                        || close <= 0.0;
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
        bars.push(Bar {
            source_index: i,
            timestamp: timestamp(&raw.dates[i]),
            open,
            high,
            low,
            close,
            volume: volume as i64,
            untradable: invalid || volume <= 0.0,
        });
    }
    Ok(bars)
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

    // 两个公开真实入口逐 bar 对拍：借用式 ParseLayerIncr+TowerCache 与 owned streaming。
    let mut parser = ParseLayerIncr::new(&config);
    let mut cache = TowerCache::new();
    let mut owned = OwnedIncrementalClassifier::new(config.clone());
    let mut terminal_streams = std::rc::Rc::new(Vec::new());
    for (i, bar) in bars.iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let direct = classifier::classify_with_tower_events_incremental(&l0, &config, &mut cache);
        let streamed = owned.append_bar_events(bar);
        if direct != streamed {
            return Err(format!("逐 bar 三元通道不等: bar={i}"));
        }
        terminal_streams = streamed.2;
    }

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
    Ok(())
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
