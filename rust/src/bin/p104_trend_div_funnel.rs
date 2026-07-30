//! task #104 调研探针：trend 侧背驰确认极低（旧测 1/807）的归因漏斗。
//!
//! 对最终 as_of 快照下的全部 NestCandidateEvent（与 p92/p93 同一 run 切分 + provider
//! 路径），按 kind 分组复算 confirm 链路的两个可失败点：
//!   ① `map_src_to_close_idx(seg_a)` / `map_src_to_close_idx(seg_c=interval_b)` 是否 None
//!      （level_view.rs:574 对 None 静默置 divergence_confirmed=false）；
//!   ② 两段皆可映射时，MACD 面积比 curr/prev（严格 curr<prev 才确认，divergence.rs:243）。
//!
//! 输出：per-level/per-kind 漏斗计数 + unconfirmed 面积比直方图 + A/C 段长（close idx 域）
//! 分位数。只读消费 provider seam；不修改生产对象、不做语义裁决。
//! 用法：`cargo run --release --bin p104_trend_div_funnel -- <btc_1m_full.json>`

use newchan_rust::theta_v0::classifier::classify_with_tower;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::divergence::compute_macd;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CoordinateWindow,
    LevelViewMaterial, LevelViewQuery, LowerLeg, NestCandidateEvent, NestDivergenceKind,
    ProjectionError, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::recursive_tower::map_src_to_close_idx;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{quantize, Bar, Direction, Side, Timestamp};
use serde::Deserialize;
use std::path::Path;

fn segment_area(hist: &[f64], lo: usize, hi: usize) -> f64 {
    hist[lo..=hi.min(hist.len() - 1)]
        .iter()
        .map(|v| v.abs())
        .sum()
}

#[derive(Debug, Default, Clone)]
struct Funnel {
    total: usize,
    map_a_none: usize,
    map_c_none: usize,
    confirmed: usize,
    diverge_false: usize,
    /// (ratio, a_len, c_len, level)
    false_rows: Vec<(f64, usize, usize, u32)>,
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p104_trend_div_funnel <btc_1m_full.json>")?;
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

    let mut events: Vec<NestCandidateEvent> = Vec::new();
    let mut lower_by_level: std::collections::BTreeMap<u32, Vec<LowerLeg>> = Default::default();
    for level in 1..tower.len() {
        let windows = &tower[level];
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
        let mut valid = vec![false; windows.len()];
        for (index, window) in windows.iter().enumerate() {
            match project_extended_windows_carried_only(std::slice::from_ref(window)) {
                Ok(_) => valid[index] = true,
                Err(ProjectionError::InvalidLowerLeg { .. }) => {
                    return Err(format!("L{level} 单窗口意外返回 InvalidLowerLeg"));
                }
                Err(_) => {}
            }
        }
        let mut runs: Vec<(usize, usize)> = Vec::new();
        let mut run_start = None;
        for index in 0..=windows.len() {
            let v = index < windows.len() && valid[index];
            match (run_start, v) {
                (None, true) => run_start = Some(index),
                (Some(start), false) => {
                    runs.push((start, index));
                    run_start = None;
                }
                _ => {}
            }
        }
        for &(start, end) in &runs {
            let projection = project_extended_windows_carried_only(&windows[start..end])
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
                    hist: &hist,
                    dif: &dif,
                    close_src: &close_src,
                },
            )
            .map_err(|error| format!("L{level} C2 assemble 失败: {error:?}"))?;
            events.extend(provide_nest_candidate_events(
                level as u32,
                &projection,
                &blocks,
                &lower,
                &view,
                &hist,
                &dif,
                &close_src,
            ));
        }
        lower_by_level.insert(level as u32, lower);
    }

    let mut trend = Funnel::default();
    let mut pan = Funnel::default();
    for event in &events {
        let funnel = match event.kind {
            NestDivergenceKind::Trend => &mut trend,
            NestDivergenceKind::Consolidation => &mut pan,
        };
        funnel.total += 1;
        if event.divergence_confirmed {
            funnel.confirmed += 1;
            continue;
        }
        let map_a = map_src_to_close_idx(&close_src, event.seg_a.0, event.seg_a.1);
        let map_c = map_src_to_close_idx(&close_src, event.interval_b.0, event.interval_b.1);
        match (map_a, map_c) {
            (None, _) => funnel.map_a_none += 1,
            (_, None) => funnel.map_c_none += 1,
            (Some((a_lo, a_hi)), Some((c_lo, c_hi))) => {
                funnel.diverge_false += 1;
                let prev = segment_area(&hist, a_lo, a_hi);
                let curr = segment_area(&hist, c_lo, c_hi);
                let ratio = if prev > 0.0 {
                    curr / prev
                } else {
                    f64::INFINITY
                };
                funnel
                    .false_rows
                    .push((ratio, a_hi - a_lo + 1, c_hi - c_lo + 1, event.level));
            }
        }
    }

    // #104 反事实（只读）：Trend 侧 C 终段改取「离开后第一个同向下级段」（forward find，
    // 对照生产 level_view.rs:691 的 rev().find 全域末段），c_start 沿用生产 interval_b.0。
    // 量化若修复 C 定位后 confirm 率的变化；不改生产语义、不做裁决。
    let mut cf_total = 0usize;
    let mut cf_no_terminal = 0usize;
    let mut cf_map_none = 0usize;
    let mut cf_lt1 = 0usize;
    let mut cf_ratios: Vec<f64> = Vec::new();
    let mut cf_c_lens: Vec<usize> = Vec::new();
    for event in events
        .iter()
        .filter(|e| matches!(e.kind, NestDivergenceKind::Trend) && !e.divergence_confirmed)
    {
        cf_total += 1;
        let dir = match event.side {
            Side::Long => Direction::Down,
            Side::Short => Direction::Up,
        };
        let c_start = event.interval_b.0;
        let Some(term) = lower_by_level.get(&event.level).and_then(|legs| {
            legs.iter().find(|leg| {
                leg.direction == dir && leg.start_index >= c_start && leg.end_index <= as_of
            })
        }) else {
            cf_no_terminal += 1;
            continue;
        };
        let map_a = map_src_to_close_idx(&close_src, event.seg_a.0, event.seg_a.1);
        let map_c = map_src_to_close_idx(&close_src, c_start, term.end_index);
        let (Some((a_lo, a_hi)), Some((c_lo, c_hi))) = (map_a, map_c) else {
            cf_map_none += 1;
            continue;
        };
        let prev = segment_area(&hist, a_lo, a_hi);
        let curr = segment_area(&hist, c_lo, c_hi);
        let ratio = if prev > 0.0 {
            curr / prev
        } else {
            f64::INFINITY
        };
        if ratio < 1.0 {
            cf_lt1 += 1;
        }
        cf_ratios.push(ratio);
        cf_c_lens.push(c_hi - c_lo + 1);
    }

    println!(
        "P104_INPUT bars={} as_of={} events_total={}",
        loaded.bars.len(),
        as_of,
        events.len()
    );
    println!(
        "P104_CF kind=Trend total={cf_total} no_terminal={cf_no_terminal} map_none={cf_map_none} lt1={cf_lt1}"
    );
    if !cf_ratios.is_empty() {
        cf_ratios.sort_by(|a, b| a.partial_cmp(b).unwrap());
        cf_c_lens.sort_unstable();
        let n = cf_ratios.len();
        let pct = |p: f64| cf_ratios[((n - 1) as f64 * p) as usize];
        println!(
            "P104_CF_RATIO n={n} min={:.3} p25={:.3} p50={:.3} p75={:.3} max={:.3} c_len_p50={}",
            cf_ratios[0],
            pct(0.25),
            pct(0.50),
            pct(0.75),
            cf_ratios[n - 1],
            cf_c_lens[n / 2],
        );
    }
    for (name, funnel) in [("Trend", &trend), ("Pan", &pan)] {
        println!(
            "P104_FUNNEL kind={name} total={} confirmed={} map_a_none={} map_c_none={} diverge_false={}",
            funnel.total, funnel.confirmed, funnel.map_a_none, funnel.map_c_none, funnel.diverge_false
        );
        let mut ratios: Vec<f64> = funnel.false_rows.iter().map(|r| r.0).collect();
        ratios.sort_by(|a, b| a.partial_cmp(b).unwrap());
        if !ratios.is_empty() {
            let n = ratios.len();
            let pct = |p: f64| ratios[((n - 1) as f64 * p) as usize];
            println!(
                "P104_RATIO kind={name} n={n} min={:.3} p25={:.3} p50={:.3} p75={:.3} max={:.3} lt1={} lt1_2={} lt2={} ge2={}",
                ratios[0],
                pct(0.25),
                pct(0.50),
                pct(0.75),
                ratios[n - 1],
                ratios.iter().filter(|r| **r < 1.0).count(),
                ratios.iter().filter(|r| **r >= 1.0 && **r < 1.2).count(),
                ratios.iter().filter(|r| **r >= 1.2 && **r < 2.0).count(),
                ratios.iter().filter(|r| **r >= 2.0).count(),
            );
            let mut a_lens: Vec<usize> = funnel.false_rows.iter().map(|r| r.1).collect();
            let mut c_lens: Vec<usize> = funnel.false_rows.iter().map(|r| r.2).collect();
            a_lens.sort_unstable();
            c_lens.sort_unstable();
            println!(
                "P104_SPAN kind={name} a_len_p50={} c_len_p50={} a_len_max={} c_len_max={} c_gt_a={}",
                a_lens[n / 2],
                c_lens[n / 2],
                a_lens[n - 1],
                c_lens[n - 1],
                funnel
                    .false_rows
                    .iter()
                    .filter(|r| r.2 > r.1)
                    .count(),
            );
        }
        let mut by_level: std::collections::BTreeMap<u32, (usize, usize)> = Default::default();
        for event in events.iter().filter(|e| {
            matches!(
                (e.kind, name),
                (NestDivergenceKind::Trend, "Trend") | (NestDivergenceKind::Consolidation, "Pan")
            )
        }) {
            let entry = by_level.entry(event.level).or_default();
            entry.0 += 1;
            if event.divergence_confirmed {
                entry.1 += 1;
            }
        }
        for (level, (total, confirmed)) in &by_level {
            println!("P104_LEVEL kind={name} level=L{level} total={total} confirmed={confirmed}");
        }
    }
    Ok(())
}

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
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
    Ok(LoadedBars { bars })
}

#[derive(Debug, Deserialize)]
struct BarsJson {
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    volumes: Vec<f64>,
    closes: Vec<f64>,
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
