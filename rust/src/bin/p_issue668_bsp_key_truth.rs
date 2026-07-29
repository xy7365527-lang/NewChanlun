//! #668（N4）BSP 结构身份键唯一性真值表实查（#666 裁定②，动手前置检查）。
//!
//! 键候选 = 被破中枢指纹（`ParentFingerprint`: center_start+zd+zg）+ 方向（`Side`）+
//! 点类（六 bit 之一，`BspPointClass`）。本 bin **只读**——跑生产 `classify_with_tower_events`
//! 一次（fresh-full，终态窗口投影，与 issue550_event_battery 的 `ISSUE551_FORK` 同一入口），
//! 对每级 `LevelState.bsp` 的每个置位 bit 求键，按键分组统计 `source_index` 去重后的基数。
//! 基数 > 1 即键不唯一——不改任何判据，不加料，照实印数。
//!
//! 用法：`cargo run --release --bin p_issue668_bsp_key_truth -- <btc_1m_full.json> [max_bars]`

use newchan_rust::theta_v0::classifier::bsp::OwnerRef;
use newchan_rust::theta_v0::classifier::cand_event::ParentFingerprint;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    parent: (usize, i64, i64),
    side: Side,
    class: PointClass,
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
    let (classification, _tower, _streams) = classifier::classify_with_tower_events(&l0, &config);

    let mut groups: BTreeMap<Key, Vec<usize>> = BTreeMap::new();
    let mut total_bit_instances = 0usize;
    let mut anchor_unresolved = 0usize;
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
                match resolve_fingerprint(level, class, idx_in_level) {
                    Some(fp) => {
                        let key = Key {
                            parent: (fp.center_start, fp.zd, fp.zg),
                            side: class.side(),
                            class,
                        };
                        groups
                            .entry(key)
                            .or_default()
                            .push((level_idx << 32) | point.source_index);
                    }
                    None => anchor_unresolved += 1,
                }
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
        "ISSUE668_TRUTH bars={} levels={} bsp_points_total={} bit_instances={} \
         distinct_keys={} anchor_unresolved={} ambiguous_keys={}",
        bars.len(),
        classification.levels.len(),
        per_level_bsp_points,
        total_bit_instances,
        groups.len(),
        anchor_unresolved,
        ambiguous,
    );
    println!("ISSUE668_TRUTH_BY_CLASS {by_class_ambiguous:?}");
    for (key, sources) in &ambiguous_examples {
        println!(
            "ISSUE668_TRUTH_EXAMPLE class={:?} side={:?} parent={:?} distinct_source_index(level<<32|idx)={:?}",
            key.class, key.side, key.parent, sources
        );
    }
    std::process::ExitCode::SUCCESS
}
