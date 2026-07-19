//! task #93 调研探针：对 #83/#84 已定格的 215 个 D1 InvalidSeed 窗口测两件事：
//!
//! 1. 携带核覆盖率：生产顺序上 `InvalidSeed`（level_view.rs:305）先于携带核检查（:311）
//!    抛出，塔 compose 携带核对这批窗口从未被读取过。本探针只读记录：`rmove` 是否为
//!    Compose 且 `centers` 非空、携带核 `[ZD,ZG]` 是否合法（zd<=zg）。
//! 2. 证书产量域交集：以最终 as_of 快照收集全部 NestCandidateEvent（与 p92 同一 run
//!    切分 + provider 路径），测 215 窗坐标区间与事件 seg_a/interval_a/interval_b/
//!    turn_source 的交集（同级/跨级分列），并记录相邻合法 run 拓扑（收复是否触发 run 合并）。
//!
//! 只读消费生产塔与 provider seam；不修改生产对象、不执行语义裁决、不提出收复方案。
//! 用法：`cargo run --release --bin p93_invalidseed_probe -- <btc_1m_full.json>`

use newchan_rust::theta_v0::classifier::classify_with_tower;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::descend::RMove;
use newchan_rust::theta_v0::classifier::divergence::compute_macd;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows,
    project_extended_windows_carried_only, provide_nest_candidate_events,
    C2LevelViewConfig, C2VersionTuple, CoordinateWindow, LevelViewMaterial, LevelViewQuery,
    NestCandidateEvent, ProjectionError, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::recursive_tower::{ElementId, LeveledMove};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{quantize, Bar, Tick, Timestamp};
use serde::Deserialize;
use std::path::Path;

/// #83/#84 守恒锚：D1 InvalidSeed 按级别 [L1..L5]。
const EXPECTED_D1: [usize; 5] = [165, 43, 7, 0, 0];

#[derive(Debug)]
struct ProbeRow {
    level: usize,
    id: ElementId,
    window_index: usize,
    sub_count: usize,
    span: (usize, usize),
    /// Compose 携带核（first center）：Some((zd, zg, centers_len))；Segment 或空 centers 为 None。
    carried: Option<(Tick, Tick)>,
    centers_len: usize,
    /// 紧邻左侧是否存在合法 run（run 以本窗口为右边界截断）。
    left_run: bool,
    /// 紧邻右侧是否存在合法 run（run 以本窗口为左边界起始）。
    right_run: bool,
    /// 相邻窗口本身也是 InvalidSeed（连片失败）。
    left_invalid: bool,
    right_invalid: bool,
    overlap_same_level: usize,
    overlap_cross_level: usize,
    /// #94（0010:29 延伸判据）：子段 envelope 与携带核 [zd,zg] 有重叠的计数。
    subs_touch: usize,
}

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
    first_date: String,
    last_date: String,
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p93_invalidseed_probe <btc_1m_full.json>")?;
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

    if tower.len() != EXPECTED_D1.len() + 1 {
        return Err(format!(
            "守恒失败：tower_levels={}，预期含 L0 的 {}",
            tower.len(),
            EXPECTED_D1.len() + 1
        ));
    }

    let mut rows: Vec<ProbeRow> = Vec::new();
    let mut events: Vec<NestCandidateEvent> = Vec::new();
    let mut per_level = [0usize; 5];

    for level in 1..tower.len() {
        let windows: &[LeveledMove] = &tower[level];
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;

        // pass 1：逐窗有效性 + InvalidSeed 明细。
        let mut valid = vec![false; windows.len()];
        for (index, window) in windows.iter().enumerate() {
            match project_extended_windows(std::slice::from_ref(window)) {
                Ok(_) => valid[index] = true,
                Err(ProjectionError::TooShort { .. }) => {}
                Err(ProjectionError::InvalidSeed { .. }) => {
                    per_level[level - 1] += 1;
                    let (carried, centers_len) = match &window.rmove {
                        RMove::Compose { centers, .. } => (
                            centers.first().map(|c| (c.zd, c.zg)),
                            centers.len(),
                        ),
                        RMove::Segment { .. } => (None, 0),
                    };
                    // #94（0010:29）：走势中枢的延伸等价于任意区间 [dn,gn] 与 [ZD,ZG] 有重叠。
                    // 逐子段 envelope 触核计数（闭区间相交：lo<=zg && hi>=zd）。
                    let subs_touch = carried
                        .map(|(zd, zg)| {
                            window
                                .sub_moves
                                .iter()
                                .filter(|sub| {
                                    let (lo, hi) = sub.envelope();
                                    lo <= zg && hi >= zd
                                })
                                .count()
                        })
                        .unwrap_or(0);
                    rows.push(ProbeRow {
                        level,
                        id: window.id,
                        window_index: index,
                        sub_count: window.sub_moves.len(),
                        span: (window.start_index, window.end_index),
                        carried,
                        centers_len,
                        left_run: false,
                        right_run: false,
                        left_invalid: false,
                        right_invalid: false,
                        overlap_same_level: 0,
                        overlap_cross_level: 0,
                        subs_touch,
                    });
                }
                Err(ProjectionError::InvalidLowerLeg { .. }) => {
                    return Err(format!("L{level} 单窗口意外返回 InvalidLowerLeg"));
                }
                Err(ProjectionError::MissingCarriedCenter { .. }) => {
                    return Err(format!(
                        "L{level} 单窗口意外返回 MissingCarriedCenter（#90：塔应逐窗携带核）"
                    ));
                }
            }
        }

        // run 切分（与 p84/p92 相同的连续合法窗分区）。
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

        // 相邻拓扑回填。
        for row in rows.iter_mut().filter(|row| row.level == level) {
            let i = row.window_index;
            row.left_run = runs.iter().any(|&(_, end)| end == i);
            row.right_run = runs.iter().any(|&(start, _)| start == i + 1);
            row.left_invalid = i > 0 && !valid[i - 1] && !is_too_short(&windows[i - 1]);
            row.right_invalid =
                i + 1 < windows.len() && !valid[i + 1] && !is_too_short(&windows[i + 1]);
        }

        // 最终 as_of 快照事件收集（p92 provider 路径）。
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
    }

    if per_level != EXPECTED_D1 || rows.len() != 215 {
        return Err(format!(
            "#84 守恒失败：actual={per_level:?} total={}; expected={EXPECTED_D1:?} total=215",
            rows.len()
        ));
    }

    // 交集测量。
    for row in &mut rows {
        for event in &events {
            if event_overlaps(event, row.span) {
                if event.level as usize == row.level {
                    row.overlap_same_level += 1;
                } else {
                    row.overlap_cross_level += 1;
                }
            }
        }
    }

    println!(
        "P93_CONSERVATION status=PASS bars={} as_of={} first_date={} last_date={} d1_invalid_seed=215 per_level={:?} events_total={}",
        loaded.bars.len(),
        as_of,
        loaded.first_date,
        loaded.last_date,
        per_level,
        events.len(),
    );

    for row in &rows {
        let (carried_str, carried_valid) = match row.carried {
            Some((zd, zg)) => (format!("{zd}:{zg}"), zd <= zg),
            None => ("-".to_string(), false),
        };
        println!(
            "P93_D1 level=L{} id={}:{} window_index={} sub_moves={} span={}:{} carried={} carried_exists={} carried_valid={} centers_len={} left_run={} right_run={} left_invalid={} right_invalid={} overlap_same_level={} overlap_cross_level={} subs_touch={}",
            row.level,
            row.id.level,
            row.id.ordinal,
            row.window_index,
            row.sub_count,
            row.span.0,
            row.span.1,
            carried_str,
            row.carried.is_some(),
            carried_valid,
            row.centers_len,
            row.left_run,
            row.right_run,
            row.left_invalid,
            row.right_invalid,
            row.overlap_same_level,
            row.overlap_cross_level,
            row.subs_touch,
        );
    }

    let carried_exists = rows.iter().filter(|r| r.carried.is_some()).count();
    let carried_valid = rows
        .iter()
        .filter(|r| r.carried.map(|(zd, zg)| zd <= zg).unwrap_or(false))
        .count();
    let overlap_same = rows.iter().filter(|r| r.overlap_same_level > 0).count();
    let overlap_cross = rows.iter().filter(|r| r.overlap_cross_level > 0).count();
    let overlap_any = rows
        .iter()
        .filter(|r| r.overlap_same_level + r.overlap_cross_level > 0)
        .count();
    let both_runs = rows.iter().filter(|r| r.left_run && r.right_run).count();
    let one_run = rows
        .iter()
        .filter(|r| r.left_run != r.right_run)
        .count();
    let no_run = rows.iter().filter(|r| !r.left_run && !r.right_run).count();
    let clustered = rows
        .iter()
        .filter(|r| r.left_invalid || r.right_invalid)
        .count();

    println!(
        "P93_SUMMARY carried_exists={carried_exists}/215 carried_valid={carried_valid}/215 carried_missing={} overlap_same_level={overlap_same}/215 overlap_cross_level={overlap_cross}/215 overlap_any={overlap_any}/215 adjacent_runs_both={both_runs} adjacent_runs_one={one_run} adjacent_runs_none={no_run} clustered_invalid={clustered}",
        215 - carried_exists,
    );
    Ok(())
}

fn is_too_short(window: &LeveledMove) -> bool {
    window.sub_moves.len() < 3
}

fn ranges_overlap(a: (usize, usize), b: (usize, usize)) -> bool {
    a.0.max(b.0) <= a.1.min(b.1)
}

fn event_overlaps(event: &NestCandidateEvent, span: (usize, usize)) -> bool {
    ranges_overlap(event.seg_a, span)
        || ranges_overlap(event.interval_a, span)
        || ranges_overlap(event.interval_b, span)
        || (event.turn_source >= span.0 && event.turn_source <= span.1)
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
