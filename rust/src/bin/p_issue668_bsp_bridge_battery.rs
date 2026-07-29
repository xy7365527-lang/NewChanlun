//! #668（N4）桥接对象验收对拍（#666 裁定⑦）：BTC 三窗跑生产 `BspBridgeBook`，与本 bin
//! 独立写就的 `source_index` 等值拼参照集逐条 cmp——两条代码路径共享同一 v2 键公式（真值表已在
//! `p_issue668_bsp_key_truth` 证过公式本身唯一），本对拍验的是**接线**：`bsp_bridge.rs` 的产出
//! 是否与「拿 `Classification` + `CandidateStreams` 直接手工拼」逐条一致，捕获实现层
//! （字段取错/off-by-one/漏判类）的错误，而非重新论证键公式。
//!
//! 用法：`cargo run --release --bin p_issue668_bsp_bridge_battery -- <btc_1m_full.json> [max_bars]`

use std::collections::BTreeSet;
use std::path::Path;

use newchan_rust::theta_v0::classifier::bsp::OwnerRef;
use newchan_rust::theta_v0::classifier::bsp_bridge::{BspBridgeBook, BspPointClass};
use newchan_rust::theta_v0::classifier::cand_event::{CandidateEvent, CandidateKey, CandidateKind, CandidateStreams};
use newchan_rust::theta_v0::classifier::{self, Classification};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{quantize, Bar, Side};
use serde::Deserialize;
use std::collections::BTreeMap;

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
    let digits: String = date.chars().take_while(|c| *c != '+').filter(char::is_ascii_digit).take(14).collect();
    digits.parse().unwrap_or_else(|_| panic!("日期 {date:?} 解析失败"))
}

fn load(path: &Path, tick_size: f64, limit: usize) -> Vec<Bar> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("读取 {} 失败: {e}", path.display()))
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    let raw: RawBars = serde_json::from_str(&text).unwrap_or_else(|e| panic!("解析失败: {e}"));
    let n = raw.closes.len().min(raw.opens.len()).min(raw.highs.len()).min(raw.lows.len()).min(raw.dates.len()).min(limit);
    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let prior = bars.last().map_or(0, |bar: &Bar| bar.close);
        let volume = raw.volumes.get(i).and_then(|value| *value).unwrap_or(0.0);
        let values = match (raw.opens[i], raw.highs[i], raw.lows[i], raw.closes[i]) {
            (Some(open), Some(high), Some(low), Some(close)) => {
                let invalid = high < open.max(close).max(low)
                    || low > open.min(close).min(high)
                    || [open, high, low, close].iter().any(|price| *price <= 0.0);
                (quantize(open, tick_size), quantize(high, tick_size), quantize(low, tick_size), quantize(close, tick_size), invalid)
            }
            _ => (prior, prior, prior, prior, true),
        };
        bars.push(Bar {
            source_index: i,
            timestamp: timestamp(&raw.dates[i]),
            open: values.0,
            high: values.1,
            low: values.2,
            close: values.3,
            volume: volume as i64,
            untradable: values.4 || volume <= 0.0,
        });
    }
    bars
}

/// 独立参照集：不调用 `bsp_bridge` 任何函数，直接拿 `Classification`/`CandidateStreams` 手工拼
/// 「(level, source_index, class) → 命中的 N1 事件键」——与 `resolve_bridge` 平行但分开写。
fn reference_join(classification: &Classification, streams: &CandidateStreams) -> BTreeSet<(u32, usize, &'static str, CandidateKey)> {
    let mut latest: BTreeMap<CandidateKey, CandidateEvent> = BTreeMap::new();
    for batch in streams.iter() {
        for event in batch.iter() {
            latest.insert(event.key, event.clone());
        }
    }
    let mut trend_by_end: BTreeMap<(u32, Side, (usize, i64, i64), usize), CandidateKey> = BTreeMap::new();
    for event in latest.values() {
        if event.kind != CandidateKind::Trend {
            continue;
        }
        let p = (event.key.parent.center_start, event.key.parent.zd, event.key.parent.zg);
        trend_by_end.insert((event.event_level, event.key.side, p, event.interval.1), event.key);
    }

    let mut out = BTreeSet::new();
    for (level_idx, level) in classification.levels.iter().enumerate() {
        for point in level.bsp.iter() {
            let center_fp = match point.center {
                Some(OwnerRef::Center(c)) => Some((c.start_index, c.zd, c.zg)),
                _ => None,
            };
            // 一类
            for (set, side, name) in [(point.bits.buy1, Side::Long, "Buy1"), (point.bits.sell1, Side::Short, "Sell1")] {
                if !set {
                    continue;
                }
                let Some(parent) = center_fp else { continue };
                if let Some(&ek) = trend_by_end.get(&(level_idx as u32, side, parent, point.source_index)) {
                    out.insert((level_idx as u32, point.source_index, name, ek));
                }
            }
            // 三类
            for (set, side, name) in [(point.bits.buy3, Side::Long, "Buy3"), (point.bits.sell3, Side::Short, "Sell3")] {
                if !set {
                    continue;
                }
                let (Some(parent), Some(entry)) = (center_fp, point.bits.third_class_entry) else { continue };
                if let Some(&ek) = trend_by_end.get(&(level_idx as u32, side, parent, entry.leave_interval.1)) {
                    out.insert((level_idx as u32, point.source_index, name, ek));
                }
            }
            // 二类：反查同级一类锚
            for (set, side, name, want_buy1) in [
                (point.bits.buy2, Side::Long, "Buy2", true),
                (point.bits.sell2, Side::Short, "Sell2", false),
            ] {
                if !set {
                    continue;
                }
                let anchor_idx = match point.center {
                    Some(OwnerRef::Type1Anchor(idx)) => idx,
                    _ => continue,
                };
                let anchor_parent = level.bsp.iter().find_map(|p| {
                    if p.source_index != anchor_idx {
                        return None;
                    }
                    let hit = if want_buy1 { p.bits.buy1 } else { p.bits.sell1 };
                    if !hit {
                        return None;
                    }
                    match p.center {
                        Some(OwnerRef::Center(c)) => Some((c.start_index, c.zd, c.zg)),
                        _ => None,
                    }
                });
                let Some(parent) = anchor_parent else { continue };
                if let Some(&ek) = trend_by_end.get(&(level_idx as u32, side, parent, anchor_idx)) {
                    out.insert((level_idx as u32, point.source_index, name, ek));
                }
            }
        }
    }
    out
}

fn class_name(class: BspPointClass) -> &'static str {
    match class {
        BspPointClass::Buy1 => "Buy1",
        BspPointClass::Buy2 => "Buy2",
        BspPointClass::Buy3 => "Buy3",
        BspPointClass::Sell1 => "Sell1",
        BspPointClass::Sell2 => "Sell2",
        BspPointClass::Sell3 => "Sell3",
    }
}

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("用法: p_issue668_bsp_bridge_battery <btc_1m_full.json> [max_bars]");
        return std::process::ExitCode::FAILURE;
    };
    let max_bars = args.next().map(|v| v.parse::<usize>().expect("max_bars 非法")).unwrap_or(100_000);
    let config = ThetaConfig::default();
    let bars = load(Path::new(&path), config.tick.tick_size, max_bars);
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
    let as_of = bars.last().map_or(0, |b| b.source_index);

    let mut trend = 0usize;
    let mut pan = 0usize;
    for batch in streams.iter() {
        for event in batch.iter() {
            match event.kind {
                CandidateKind::Trend => trend += 1,
                CandidateKind::Pan => pan += 1,
            }
        }
    }
    let mut b1 = 0usize;
    let mut b2 = 0usize;
    let mut b3 = 0usize;
    let mut b1_by_level: BTreeMap<usize, usize> = BTreeMap::new();
    for (level_idx, level) in classification.levels.iter().enumerate() {
        for p in level.bsp.iter() {
            if p.bits.buy1 || p.bits.sell1 {
                b1 += 1;
                *b1_by_level.entry(level_idx).or_default() += 1;
            }
            if p.bits.buy2 || p.bits.sell2 {
                b2 += 1;
            }
            if p.bits.buy3 || p.bits.sell3 {
                b3 += 1;
            }
        }
    }

    let reference = reference_join(&classification, &streams);
    let mut hit_by_class: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut hit_by_level_b1: BTreeMap<usize, usize> = BTreeMap::new();
    for (level, _, name, _) in &reference {
        *hit_by_class.entry(*name).or_default() += 1;
        if matches!(*name, "Buy1" | "Sell1") {
            *hit_by_level_b1.entry(*level as usize).or_default() += 1;
        }
    }
    println!(
        "ISSUE668_BRIDGE_COVERAGE bars={} trend_events={trend} pan_events={pan} \
         b1_points={b1} b2_points={b2} b3_points={b3} hit_by_class={hit_by_class:?} \
         b1_by_level={b1_by_level:?} b1_hit_by_level={hit_by_level_b1:?}",
        bars.len(),
    );

    let mut bridge = BspBridgeBook::default();
    bridge.advance(&classification, &streams, as_of);
    let produced: BTreeSet<(u32, usize, &'static str, CandidateKey)> = bridge
        .heads()
        .into_iter()
        .map(|edge| (edge.bsp_level, edge.bsp_source_index, class_name(edge.key.bsp.class), edge.key.event))
        .collect();

    let missing_in_bridge: Vec<_> = reference.difference(&produced).collect();
    let extra_in_bridge: Vec<_> = produced.difference(&reference).collect();
    let cmp = missing_in_bridge.len() + extra_in_bridge.len();

    println!(
        "ISSUE668_BRIDGE_BATTERY bars={} reference_edges={} produced_edges={} missing_in_bridge={} extra_in_bridge={} cmp={}",
        bars.len(),
        reference.len(),
        produced.len(),
        missing_in_bridge.len(),
        extra_in_bridge.len(),
        cmp,
    );
    for item in missing_in_bridge.iter().take(5) {
        println!("ISSUE668_BRIDGE_MISSING {item:?}");
    }
    for item in extra_in_bridge.iter().take(5) {
        println!("ISSUE668_BRIDGE_EXTRA {item:?}");
    }
    if cmp == 0 {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}
