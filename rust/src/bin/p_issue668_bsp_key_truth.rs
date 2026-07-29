//! #668（N4）BSP 结构身份键唯一性真值表实查——v2 键公式（#666 supersede 回炉裁定）。
//!
//! v1 键（被破中枢指纹 + 方向 + 点类）已被真值表证伪（三窗 ambiguous=10/74/260，报告
//! `issue668-n4-impl-ticket668-20260729.md`）。v2 = v1 + **锚段坐标**：
//! - 一类：背驰确认段坐标 = `(key.seg_a, interval)`（`interval=(c_start, source_index)`），
//!   经生产候选事件流（[`cand_event::CandidateStreams`]，`classify_with_tower_events` 第三个
//!   返回值，v1 探针原弃用）按 `(level, side, parent_fingerprint, interval.1==point.source_index)`
//!   反查同一次塔扫描产出的 Trend 候选（`CandidateKey.seg_a`/`c_start` 与本点共用同一
//!   `first_structural_gates` 结构门，见 `cand_event::trend_observation`）。
//! - 三类：离开段+回试段对 = 既有 `bits.third_class_entry`（`ThirdClassEntryIdentity`，#542
//!   随证书直传，零新增）的 `(leave_interval, retest_interval)`，无需查表。
//! - 二类：一类锚身份（既有 `OwnerRef::Type1Anchor` 坐标）+ 回抽段坐标——**限**：回抽走势
//!   （`RMove` 次级别走势）在 `extract_second_signals` 入参层已坐标剥离（signal.rs 注释
//!   「坐标 still-MISSING」），生产路径只把回拉走势*终点*（= 本点 `source_index` 自身）经
//!   `index_of` 传出，起点未接线到任何输出结构；反查需重跑 `find_second_type_structure`
//!   （新判定路径，违反「零改动/不重算」方法学），故本轮**只用回拉终点**（即
//!   `(source_index, source_index)`，对键无增量区分力，见报告 §方法 限制登记）近似占位，
//!   不冒充真实回抽段坐标。
//!
//! 本 bin 仍**只读**：跑生产 `classify_with_tower_events` 一次（fresh-full），零改动任何
//! `judge_*`/`extract_*` 判据函数，只新增读取既有输出字段（`third_class_entry`/候选事件流）
//! 与一次「按 key 取最新 revision」的折叠（与 `cand_sub::latest_by_level` 同方法学，非新判据）。
//!
//! 用法：`cargo run --release --bin p_issue668_bsp_key_truth -- <btc_1m_full.json> [max_bars]`

use newchan_rust::theta_v0::classifier::bsp::OwnerRef;
use newchan_rust::theta_v0::classifier::cand_event::{
    CandidateEvent, CandidateKind, CandidateStreams, ParentFingerprint,
};
use newchan_rust::theta_v0::classifier::{self, LevelState};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{quantize, Bar, Side};
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

fn timestamp(date: &str) -> Result<i64, String> {
    let digits: String = date
        .chars()
        .take_while(|c| *c != '+')
        .filter(char::is_ascii_digit)
        .take(14)
        .collect();
    digits
        .parse()
        .map_err(|error| format!("日期 {date:?} 解析时间戳失败（提取数字串 {digits:?}）: {error}"))
}

fn load(path: &Path, tick_size: f64, limit: usize) -> Result<Vec<Bar>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    let raw: RawBars =
        serde_json::from_str(&text).map_err(|error| format!("解析 {} 失败: {error}", path.display()))?;
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
        bars.push(Bar {
            source_index: i,
            timestamp: timestamp(&raw.dates[i])?,
            open: values.0,
            high: values.1,
            low: values.2,
            close: values.3,
            volume: volume as i64,
            untradable: values.4 || volume <= 0.0,
        });
    }
    Ok(bars)
}

/// 点类：六 bit 之一，逐 bit 独立入键（一个 `BspPoint` 可能同时贡献多把键，如 2B/3B 共存）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PointClass {
    Buy1,
    Buy2,
    Buy3,
    Sell1,
    Sell2,
    Sell3,
}

impl PointClass {
    fn side(self) -> Side {
        match self {
            PointClass::Buy1 | PointClass::Buy2 | PointClass::Buy3 => Side::Long,
            PointClass::Sell1 | PointClass::Sell2 | PointClass::Sell3 => Side::Short,
        }
    }
}

/// v2 键：v1 三分量 + 锚段坐标（可变长——一/三类两段，二类占位一段，见模块头限制登记）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    parent: (usize, i64, i64),
    side: Side,
    class: PointClass,
    anchor: Vec<(usize, usize)>,
}

/// 按 key 取最新 revision（与 `cand_sub::latest_by_level` 同方法学——纯折叠，非新判据），
/// 按 `(level, side, parent, interval.1)` 建索引供一类反查（同一次塔扫描的 Trend 候选）。
fn index_trend_candidates(
    streams: &CandidateStreams,
) -> BTreeMap<(u32, Side, (usize, i64, i64), usize), CandidateEvent> {
    let mut latest: BTreeMap<_, CandidateEvent> = BTreeMap::new();
    for batch in streams.iter() {
        for event in batch.iter() {
            latest.insert(event.key, event.clone());
        }
    }
    let mut index = BTreeMap::new();
    for event in latest.into_values() {
        if event.kind != CandidateKind::Trend {
            continue;
        }
        let parent = (event.key.parent.center_start, event.key.parent.zd, event.key.parent.zg);
        index.insert((event.event_level, event.key.side, parent, event.interval.1), event);
    }
    index
}

fn resolve_fingerprint(level: &LevelState, class: PointClass, idx_in_level: usize) -> Option<ParentFingerprint> {
    let point = &level.bsp[idx_in_level];
    match class {
        PointClass::Buy1 | PointClass::Sell1 | PointClass::Buy3 | PointClass::Sell3 => {
            match point.center {
                Some(OwnerRef::Center(c)) => Some(ParentFingerprint {
                    center_start: c.start_index,
                    zd: c.zd,
                    zg: c.zg,
                }),
                _ => None,
            }
        }
        PointClass::Buy2 | PointClass::Sell2 => {
            let anchor = match point.center {
                Some(OwnerRef::Type1Anchor(idx)) => idx,
                _ => return None,
            };
            let want_bit = matches!(class, PointClass::Buy2);
            level.bsp.iter().find_map(|p| {
                if p.source_index != anchor {
                    return None;
                }
                let hit = if want_bit { p.bits.buy1 } else { p.bits.sell1 };
                if !hit {
                    return None;
                }
                match p.center {
                    Some(OwnerRef::Center(c)) => Some(ParentFingerprint {
                        center_start: c.start_index,
                        zd: c.zd,
                        zg: c.zg,
                    }),
                    _ => None,
                }
            })
        }
    }
}

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("用法: p_issue668_bsp_key_truth <btc_1m_full.json> [max_bars]");
            return std::process::ExitCode::FAILURE;
        }
    };
    let max_bars = args
        .next()
        .map(|v| v.parse::<usize>().expect("max_bars 非法"))
        .unwrap_or(100_000);
    let config = ThetaConfig::default();
    let bars = load(Path::new(&path), config.tick.tick_size, max_bars).expect("加载失败");
    if bars.is_empty() {
        eprintln!("空窗口");
        return std::process::ExitCode::FAILURE;
    }

    let mut parser = ParseLayerIncr::new(&config);
    let mut l0 = parser.append(bars[0]);
    for bar in bars.iter().copied().skip(1) {
        l0 = parser.append(bar);
    }
    let (classification, _tower, streams) = classifier::classify_with_tower_events(&l0, &config);
    let trend_index = index_trend_candidates(&streams);

    let mut groups: BTreeMap<Key, Vec<usize>> = BTreeMap::new();
    let mut total_bit_instances = 0usize;
    let mut anchor_unresolved = 0usize;
    let mut anchor_seg_unresolved = 0usize;
    let mut anchor_seg_unresolved_by_class: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut per_level_bsp_points = 0usize;

    for (level_idx, level) in classification.levels.iter().enumerate() {
        per_level_bsp_points += level.bsp.len();
        for (idx_in_level, point) in level.bsp.iter().enumerate() {
            let bits = [
                (point.bits.buy1, PointClass::Buy1),
                (point.bits.buy2, PointClass::Buy2),
                (point.bits.buy3, PointClass::Buy3),
                (point.bits.sell1, PointClass::Sell1),
                (point.bits.sell2, PointClass::Sell2),
                (point.bits.sell3, PointClass::Sell3),
            ];
            for (set, class) in bits {
                if !set {
                    continue;
                }
                total_bit_instances += 1;
                let Some(fp) = resolve_fingerprint(level, class, idx_in_level) else {
                    anchor_unresolved += 1;
                    continue;
                };
                let parent = (fp.center_start, fp.zd, fp.zg);
                let side = class.side();
                // 锚段坐标（v2 新增分量，见模块头方法学；一/三类查真实段坐标，
                // 二类仅回拉终点占位——查不到锚段计入 anchor_seg_unresolved，不进分组。
                let anchor = match class {
                    PointClass::Buy1 | PointClass::Sell1 => trend_index
                        .get(&(level_idx as u32, side, parent, point.source_index))
                        .map(|event| vec![event.key.seg_a, event.interval]),
                    PointClass::Buy3 | PointClass::Sell3 => {
                        point.bits.third_class_entry.map(|entry| {
                            vec![entry.leave_interval, entry.retest_interval]
                        })
                    }
                    PointClass::Buy2 | PointClass::Sell2 => {
                        let anchor_idx = match point.center {
                            Some(OwnerRef::Type1Anchor(idx)) => idx,
                            _ => unreachable!("resolve_fingerprint 已保证 Type1Anchor 存在"),
                        };
                        Some(vec![
                            (anchor_idx, anchor_idx),
                            (point.source_index, point.source_index),
                        ])
                    }
                };
                let Some(anchor) = anchor else {
                    anchor_seg_unresolved += 1;
                    let name = match class {
                        PointClass::Buy1 => "Buy1",
                        PointClass::Buy2 => "Buy2",
                        PointClass::Buy3 => "Buy3",
                        PointClass::Sell1 => "Sell1",
                        PointClass::Sell2 => "Sell2",
                        PointClass::Sell3 => "Sell3",
                    };
                    *anchor_seg_unresolved_by_class.entry(name).or_default() += 1;
                    continue;
                };
                let key = Key { parent, side, class, anchor };
                groups
                    .entry(key)
                    .or_default()
                    .push((level_idx << 32) | point.source_index);
            }
        }
    }

    let mut ambiguous = 0usize;
    let mut ambiguous_examples = Vec::new();
    let mut by_class_ambiguous: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (key, sources) in &groups {
        let mut distinct: Vec<usize> = sources.clone();
        distinct.sort_unstable();
        distinct.dedup();
        if distinct.len() > 1 {
            ambiguous += 1;
            let name = match key.class {
                PointClass::Buy1 => "Buy1",
                PointClass::Buy2 => "Buy2",
                PointClass::Buy3 => "Buy3",
                PointClass::Sell1 => "Sell1",
                PointClass::Sell2 => "Sell2",
                PointClass::Sell3 => "Sell3",
            };
            *by_class_ambiguous.entry(name).or_default() += 1;
            let is_first_class = matches!(key.class, PointClass::Buy1 | PointClass::Sell1);
            if ambiguous_examples.len() < 8 || is_first_class {
                ambiguous_examples.push((key.clone(), distinct.clone()));
            }
        }
    }

    println!(
        "ISSUE668_TRUTH_V2 bars={} levels={} bsp_points_total={} bit_instances={} \
         distinct_keys={} anchor_unresolved={} anchor_seg_unresolved={} ambiguous_keys={}",
        bars.len(),
        classification.levels.len(),
        per_level_bsp_points,
        total_bit_instances,
        groups.len(),
        anchor_unresolved,
        anchor_seg_unresolved,
        ambiguous,
    );
    println!("ISSUE668_TRUTH_V2_ANCHOR_SEG_UNRESOLVED_BY_CLASS {anchor_seg_unresolved_by_class:?}");
    println!("ISSUE668_TRUTH_BY_CLASS {by_class_ambiguous:?}");
    for (key, sources) in &ambiguous_examples {
        println!(
            "ISSUE668_TRUTH_EXAMPLE class={:?} side={:?} parent={:?} anchor={:?} \
             distinct_source_index(level<<32|idx)={:?}",
            key.class, key.side, key.parent, key.anchor, sources
        );
    }
    std::process::ExitCode::SUCCESS
}
