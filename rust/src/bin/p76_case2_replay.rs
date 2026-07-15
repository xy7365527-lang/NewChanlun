//! task #76：#56 例2在 D1 投影 + D2 自动配对后的只读 C2 重放探针。
//!
//! 用法：
//! `cargo run --release --bin p76_case2_replay -- <btc_1m_full.json> <as_of>`

use newchan_rust::theta_v0::classifier::classify_with_tower;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::divergence::compute_macd;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows, C2LevelViewConfig,
    C2VersionTuple, CompletionStatus, CoordinateWindow, LevelViewMaterial, LevelViewQuery,
    ProjectionMaterial,
};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{quantize, Bar, Timestamp};
use serde::Deserialize;
use std::path::Path;

const REGION_START: usize = 2_884_261;
const REGION_END: usize = 2_943_378;
const DEFAULT_CONTEXT_START: usize = 2_800_000;
const TARGET_LEVEL: usize = 3;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p76_case2_replay <btc_1m_full.json> <as_of>")?;
    let as_of = args
        .next()
        .ok_or("缺 as_of")?
        .parse::<usize>()
        .map_err(|error| format!("as_of 非整数: {error}"))?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }

    let config = ThetaConfig::default();
    let context_start = std::env::var("P76_CONTEXT_START")
        .ok()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("P76_CONTEXT_START 非整数: {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CONTEXT_START);
    let bars = load_bars(Path::new(&path), config.tick.tick_size)?;
    if as_of >= bars.len() {
        return Err(format!("as_of={as_of} 越界，总 bars={}", bars.len()));
    }
    let layer = parse_layer(&bars[..=as_of], &config);
    let (classification, tower) = classify_with_tower(&layer, &config);
    let all_windows = tower
        .get(TARGET_LEVEL)
        .ok_or_else(|| format!("缺 tower[{TARGET_LEVEL}]，实际 levels={}", tower.len()))?;
    let windows: Vec<_> = all_windows
        .iter()
        .filter(|value| value.end_index >= context_start && value.start_index <= as_of)
        .cloned()
        .collect();
    let lower = tower
        .get(TARGET_LEVEL - 1)
        .ok_or_else(|| format!("缺 tower[{}]", TARGET_LEVEL - 1))?;

    println!(
        "P76_INPUT as_of={as_of} context_start={context_start} all_windows_L3={} selected_windows_L3={}",
        all_windows.len(),
        windows.len()
    );
    for value in &windows {
        println!(
            "P76_WINDOW L{}#{} sub_count={} [{}..{}]",
            value.id.level,
            value.id.ordinal,
            value.sub_moves.len(),
            value.start_index,
            value.end_index
        );
    }

    let projection = project_extended_windows(&windows)
        .map_err(|error| format!("D1 projection 失败: {error:?}"))?;
    let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
    let blocks = decompose::decompose(&centers);
    let legs = lower_legs_from(lower).map_err(|error| format!("lower legs 失败: {error:?}"))?;
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
    let query = LevelViewQuery {
        level: TARGET_LEVEL as u32,
        coordinate_window: CoordinateWindow {
            start: REGION_START,
            end: REGION_END,
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
            lower_legs: &legs,
            hist: &hist,
            close_src: &close_src,
        },
    )
    .map_err(|error| format!("C2 assemble 失败: {error:?}"))?;

    println!(
        "P76 as_of={as_of} raw_bars={} merged={} levels={} windows_L3={} lower_L2={} seeds={} blocks={} pairs={}",
        as_of + 1,
        layer.merged_bars.len(),
        classification.levels.len(),
        windows.len(),
        lower.len(),
        projection.seeds.len(),
        blocks.len(),
        view.pairs.len()
    );
    for seed in projection
        .seeds
        .iter()
        .filter(|seed| overlaps(seed.start_index, seed.end_index))
    {
        println!(
            "P76_SEED L{}#{} source_sub_count={} [{}..{}] core=[{}..{}] envelope=[{}..{}]",
            seed.source_id.level,
            seed.source_id.ordinal,
            seed.source_sub_count,
            seed.start_index,
            seed.end_index,
            seed.center.zd,
            seed.center.zg,
            seed.center.dd,
            seed.center.gg
        );
    }
    for (index, value) in view
        .moves
        .iter()
        .enumerate()
        .filter(|(_, value)| overlaps(value.start_index, value.end_index))
    {
        let status = match value.completion {
            CompletionStatus::Completed { evidence, .. } => format!("Completed({evidence:?})"),
            CompletionStatus::Pending { reason, .. } => format!("Pending({reason:?})"),
        };
        println!(
            "P76_MOVE g{index} {:?}/{:?} {status} [{}..{}] centers={:?}",
            value.kind, value.direction, value.start_index, value.end_index, value.center_indices
        );
    }
    for pair in &view.pairs {
        if overlaps(pair.seg_a.0, pair.seg_c.1) {
            println!(
                "P76_AC {:?} move_start={} A=[{}..{}] C=[{}..{}]",
                pair.id, pair.move_start, pair.seg_a.0, pair.seg_a.1, pair.seg_c.0, pair.seg_c.1
            );
        }
    }
    Ok(())
}

fn overlaps(start: usize, end: usize) -> bool {
    start <= REGION_END && end >= REGION_START
}

fn load_bars(path: &Path, tick_size: f64) -> Result<Vec<Bar>, String> {
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
    Ok((0..n)
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
        .collect())
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
