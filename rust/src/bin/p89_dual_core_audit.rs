//! task #89：静默双核审计（#148 升级重切 C1 继承核 vs 生产投影 offset-0 核）。
//!
//! 背景（#88 对账结论）：215 个 InvalidSeed 窗全部是重切子窗（零反例）。本审计问其补集：
//! **offset-0 投影成功**的重切子窗里，有多少个投影核 [ZD,ZG] ≠ 塔继承核（父 seed 三段交）——
//! 即 econ 层（level_view seed）与塔层（compose 继承 center）**静默**看见两套核心。
//!
//! 只读探针：与生产完全同参重算每级 detect（`detect_centers_windowed_resume` 起点 0），
//! 用 `WinMeta.emitted > 1` 标记重切子窗；塔侧核直接读 `RMove::Compose.centers[0]`
//! （compose 携带的检测核），并与重算 detect 的产出逐窗核对（守恒）。
//!
//! 下游分叉测量：对每级 valid 连续窗 run，分别用投影 seed 核序列与塔携带核序列跑
//! `decompose`（econ 层 trend 块 vs 塔同源块），逐块比对——量化双核是否已实际改变走势块结构。
//!
//! 用法：`p89_dual_core_audit <btc_1m_full.json>`

use newchan_rust::theta_v0::classifier::center::{
    center_from_segments, center_from_window, UnitRange,
};
use newchan_rust::theta_v0::classifier::classify_with_tower;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::descend::RMove;
use newchan_rust::theta_v0::classifier::level_view::{project_extended_windows, ProjectionError};
use newchan_rust::theta_v0::classifier::recursive_tower::{
    detect_centers_windowed_resume, project_to_units, LeveledMove,
};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{quantize, Bar, Center, Direction, Segment, Timestamp};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

/// #86/#88 基线：生产 Production 逐窗投影 InvalidSeed 分布（level 1..=5）。
const EXPECTED_D1: [usize; 5] = [165, 43, 7, 0, 0];

/// L0 线段 → 走势单元（mod.rs `segment_to_unit` 逐字段复刻，私有故本地镜像）。
fn segment_to_unit(seg: &Segment) -> UnitRange {
    let (lo, hi) = if seg.start_price <= seg.end_price {
        (seg.start_price, seg.end_price)
    } else {
        (seg.end_price, seg.start_price)
    };
    UnitRange {
        start_index: seg.start_index,
        end_index: seg.end_index,
        direction: seg.direction,
        lo,
        hi,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjOutcome {
    /// 投影 InvalidSeed（#88：应全为重切子窗）。
    Invalid,
    /// offset-0 成功且投影核 == 塔核。
    OkEqual,
    /// offset-0 成功但投影核 ≠ 塔核（静默双核）。
    OkMismatch,
}

#[derive(Debug, Clone)]
struct MismatchDetail {
    level: usize,
    ordinal: usize,
    recut: bool,
    subs: usize,
    src_start: usize,
    src_end: usize,
    tower_zd: i64,
    tower_zg: i64,
    seed_zd: i64,
    seed_zg: i64,
}

#[derive(Debug, Default)]
struct LevelAudit {
    windows: usize,
    units: usize,
    recut_children: usize,
    recut_parents: usize,
    invalid_recut: usize,
    invalid_normal: usize,
    ok_equal_recut: usize,
    ok_equal_normal: usize,
    mismatch_recut: usize,
    mismatch_normal: usize,
    // 下游分叉（econ seed 核 vs 塔携带核，同 run decompose 逐块比对）。
    runs: usize,
    runs_forked: usize,
    blocks_seed: usize,
    blocks_tower: usize,
    blocks_diff: usize,
    trend_dir_diff: usize,
    // 双核窗中落在 econ 层 Trend 块（dir=Some）内的个数（下游消费面）。
    mismatch_in_trend: usize,
}

fn carried_center(mv: &LeveledMove) -> Result<Center, String> {
    match &mv.rmove {
        RMove::Compose { centers, .. } => centers
            .first()
            .copied()
            .ok_or_else(|| "Compose 缺携带 center".to_string()),
        RMove::Segment { .. } => Err("上级走势不应是 Segment".to_string()),
    }
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p89_dual_core_audit <btc_1m_full.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }

    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.len() != 4_613_599 {
        return Err(format!(
            "守恒失败：原始 bars={}，预期 4613599",
            loaded.bars.len()
        ));
    }
    let layer = parse_layer(&loaded.bars, &config);
    let (classification, tower) = classify_with_tower(&layer, &config);
    if tower.len() != 6 {
        return Err(format!("守恒失败：tower_levels={}，预期 6", tower.len()));
    }
    if classification.levels.len() != 6 {
        return Err(format!(
            "守恒失败：classification_levels={}，预期 6",
            classification.levels.len()
        ));
    }

    let mut audits: Vec<LevelAudit> = Vec::new();
    let mut mismatches: Vec<MismatchDetail> = Vec::new();

    // 检测发生在 level_idx k（消费 units_k），产出上级 L = k+1 的窗（tower[L]）。
    for k in 0..5usize {
        let upper_level = k + 1;
        let units: Vec<UnitRange> = if k == 0 {
            layer.segments.iter().map(segment_to_unit).collect()
        } else {
            project_to_units(tower[k].as_ref(), &classification.levels[k - 1].moves)
        };
        let build = if k == 0 {
            center_from_segments
        } else {
            center_from_window
        };
        let (windowed, metas, _) = detect_centers_windowed_resume(&units, build, 0);
        let upper: &Vec<LeveledMove> = tower[upper_level].as_ref();
        if windowed.len() != upper.len() || metas.len() != upper.len() {
            return Err(format!(
                "L{upper_level} 守恒失败：重算 detect 窗数={} metas={} != 塔窗数={}",
                windowed.len(),
                metas.len(),
                upper.len()
            ));
        }

        let mut audit = LevelAudit {
            windows: upper.len(),
            units: units.len(),
            ..Default::default()
        };
        let mut recut_parent_starts: BTreeSet<usize> = BTreeSet::new();
        let mut outcomes: Vec<ProjOutcome> = Vec::with_capacity(upper.len());
        let mut mismatch_ordinals_this_level: Vec<usize> = Vec::new();

        for (i, mv) in upper.iter().enumerate() {
            let tower_core = carried_center(mv)?;
            if windowed[i].0 != tower_core {
                return Err(format!(
                    "L{upper_level} ordinal={i} 守恒失败：重算 detect 核 != compose 携带核"
                ));
            }
            let recut = metas[i].emitted > 1;
            if recut {
                audit.recut_children += 1;
                recut_parent_starts.insert(metas[i].win_start);
            }
            let outcome = match project_extended_windows(std::slice::from_ref(mv)) {
                Err(ProjectionError::InvalidSeed { .. }) => {
                    if recut {
                        audit.invalid_recut += 1;
                    } else {
                        audit.invalid_normal += 1;
                    }
                    ProjOutcome::Invalid
                }
                Err(other) => {
                    return Err(format!(
                        "L{upper_level} ordinal={i} 意外投影错误: {other:?}"
                    ));
                }
                Ok(projection) => {
                    let seed = &projection.seeds[0];
                    let equal = seed.center.zd == tower_core.zd && seed.center.zg == tower_core.zg;
                    if equal {
                        if recut {
                            audit.ok_equal_recut += 1;
                        } else {
                            audit.ok_equal_normal += 1;
                        }
                        ProjOutcome::OkEqual
                    } else {
                        if recut {
                            audit.mismatch_recut += 1;
                        } else {
                            audit.mismatch_normal += 1;
                        }
                        mismatch_ordinals_this_level.push(i);
                        mismatches.push(MismatchDetail {
                            level: upper_level,
                            ordinal: i,
                            recut,
                            subs: mv.sub_moves.len(),
                            src_start: mv.start_index,
                            src_end: mv.end_index,
                            tower_zd: tower_core.zd,
                            tower_zg: tower_core.zg,
                            seed_zd: seed.center.zd,
                            seed_zg: seed.center.zg,
                        });
                        ProjOutcome::OkMismatch
                    }
                }
            };
            outcomes.push(outcome);
        }
        audit.recut_parents = recut_parent_starts.len();

        // 守恒：InvalidSeed 分布 == #86 基线。
        let invalid_total = audit.invalid_recut + audit.invalid_normal;
        if invalid_total != EXPECTED_D1[k] {
            return Err(format!(
                "L{upper_level} 守恒失败：invalid_seed={invalid_total}，预期 {}",
                EXPECTED_D1[k]
            ));
        }

        // 下游分叉：valid 连续 run（与 p86 measure_level 同切分），econ seed 核 vs 塔核 decompose。
        let mut start: Option<usize> = None;
        let mismatch_set: BTreeSet<usize> = mismatch_ordinals_this_level.iter().copied().collect();
        for index in 0..=upper.len() {
            let valid = index < upper.len() && outcomes[index] != ProjOutcome::Invalid;
            match (start, valid) {
                (None, true) => start = Some(index),
                (Some(s), false) => {
                    audit.runs += 1;
                    let run = &upper[s..index];
                    let projection = project_extended_windows(run)
                        .map_err(|e| format!("L{upper_level} run[{s},{index}) 投影失败: {e:?}"))?;
                    let seed_centers: Vec<Center> =
                        projection.seeds.iter().map(|seed| seed.center).collect();
                    let tower_centers: Vec<Center> =
                        run.iter().map(carried_center).collect::<Result<_, _>>()?;
                    let blocks_seed = decompose::decompose(&seed_centers);
                    let blocks_tower = decompose::decompose(&tower_centers);
                    audit.blocks_seed += blocks_seed.len();
                    audit.blocks_tower += blocks_tower.len();
                    let mut forked = blocks_seed.len() != blocks_tower.len();
                    let common = blocks_seed.len().min(blocks_tower.len());
                    for bi in 0..common {
                        if blocks_seed[bi] != blocks_tower[bi] {
                            audit.blocks_diff += 1;
                            forked = true;
                            if blocks_seed[bi].dir != blocks_tower[bi].dir {
                                audit.trend_dir_diff += 1;
                            }
                        }
                    }
                    audit.blocks_diff += blocks_seed.len().abs_diff(blocks_tower.len());
                    if forked {
                        audit.runs_forked += 1;
                    }
                    // 双核窗是否落在 econ 层 Trend 块内（run 内相对下标 = seed 序）。
                    for block in blocks_seed.iter().filter(|b| b.dir.is_some()) {
                        for rel in block.start_center..=block.end_center {
                            if mismatch_set.contains(&(s + rel)) {
                                audit.mismatch_in_trend += 1;
                            }
                        }
                    }
                    start = None;
                }
                _ => {}
            }
        }

        audits.push(audit);
    }

    // ── 输出 ──
    let total_windows: usize = audits.iter().map(|a| a.windows).sum();
    let total_invalid: usize = audits
        .iter()
        .map(|a| a.invalid_recut + a.invalid_normal)
        .sum();
    let total_recut: usize = audits.iter().map(|a| a.recut_children).sum();
    let total_mismatch: usize = audits
        .iter()
        .map(|a| a.mismatch_recut + a.mismatch_normal)
        .sum();
    let total_mismatch_normal: usize = audits.iter().map(|a| a.mismatch_normal).sum();
    let total_invalid_normal: usize = audits.iter().map(|a| a.invalid_normal).sum();

    println!(
        "P89_CONSERVATION status=PASS bars={} merged={} tower_levels=6 windows_total={} invalid=[{}] invalid_total={} carried_center_match=OK",
        loaded.bars.len(),
        layer.merged_bars.len(),
        total_windows,
        audits
            .iter()
            .map(|a| (a.invalid_recut + a.invalid_normal).to_string())
            .collect::<Vec<_>>()
            .join(","),
        total_invalid,
    );

    for (k, a) in audits.iter().enumerate() {
        println!(
            "P89_LEVEL level={} units={} windows={} recut_parents={} recut_children={} invalid_recut={} invalid_normal={} ok_equal_recut={} ok_equal_normal={} mismatch_recut={} mismatch_normal={}",
            k + 1,
            a.units,
            a.windows,
            a.recut_parents,
            a.recut_children,
            a.invalid_recut,
            a.invalid_normal,
            a.ok_equal_recut,
            a.ok_equal_normal,
            a.mismatch_recut,
            a.mismatch_normal,
        );
    }

    for a in audits.iter().enumerate() {
        let (k, a) = a;
        println!(
            "P89_FORK level={} runs={} runs_forked={} blocks_seed={} blocks_tower={} blocks_diff={} trend_dir_diff={} mismatch_in_trend={}",
            k + 1,
            a.runs,
            a.runs_forked,
            a.blocks_seed,
            a.blocks_tower,
            a.blocks_diff,
            a.trend_dir_diff,
            a.mismatch_in_trend,
        );
    }

    const DETAIL_CAP: usize = 80;
    for d in mismatches.iter().take(DETAIL_CAP) {
        println!(
            "P89_MISMATCH level={} ordinal={} recut={} subs={} src=[{},{}] tower=[{},{}] seed=[{},{}] dzd={} dzg={}",
            d.level,
            d.ordinal,
            d.recut,
            d.subs,
            d.src_start,
            d.src_end,
            d.tower_zd,
            d.tower_zg,
            d.seed_zd,
            d.seed_zg,
            d.seed_zd - d.tower_zd,
            d.seed_zg - d.tower_zg,
        );
    }
    if mismatches.len() > DETAIL_CAP {
        println!(
            "P89_MISMATCH_TRUNCATED shown={DETAIL_CAP} total={}",
            mismatches.len()
        );
    }

    println!(
        "P89_SUMMARY dual_core_total={} recut_children_total={} invalid_total={} mismatch_normal_total={} invalid_normal_total={} verdict_mismatch_all_recut={} verdict_invalid_all_recut={}",
        total_mismatch,
        total_recut,
        total_invalid,
        total_mismatch_normal,
        total_invalid_normal,
        total_mismatch_normal == 0,
        total_invalid_normal == 0,
    );

    Ok(())
}

// ── 数据加载（p86 逐字段复刻，探针自包含）──

struct LoadedBars {
    bars: Vec<Bar>,
    #[allow(dead_code)]
    first_date: String,
    #[allow(dead_code)]
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

// 编译期占位：Direction 仅在 fork 方向比对处经字段间接使用；显式引用防未来裁剪误报。
#[allow(dead_code)]
fn _dir_witness(d: Direction) -> Direction {
    d
}
