//! #668（N4）桥接对象验收对拍——修复轮 1（#670 影子评审 HIGH-3 回炉，
//! `chanlun/review-results/issue668-n4-fix-round1-20260729.md`）。
//!
//! ## 门重定（HIGH-3 处置）
//!
//! 旧版本的参照集与生产模块共享同一 v2 键公式**且逐字段同构复写**——两侧共享全部设计判断，
//! 包括错的那些，验不出判据层错误（HIGH-2 就在它眼前而它 cmp=0）。裁定第三轮 supersede ⑤
//! 明文给的替代方案 = 「跨 as_of 平价锁 + 一条会因 HIGH-2 式漏配变红的负控」：
//! - **跨 as_of 平价锁**：`bsp_bridge::tests::bridge_book_incremental_equals_full_replay_across_as_of`
//!   （对齐 N1/N3 先例，单测覆盖，非本 bin 职责）。
//! - **负控**：`bsp_bridge::tests::trend_event_growth_does_not_orphan_earlier_first_class_point`
//!   （在旧右端等值判据下必然失败，锁住 HIGH-2 那条根因不再复发）。
//!
//! 本 bin 保留的职责收窄为**真独立参照集**——不再对拍 `bridge.heads()`（只含链头，同 episode
//! 多物理点会被折叠），改对拍 `bridge.edges()`（append-only 全量修订历史；单次 fresh-full
//! 窗口内，一个 episode 覆盖的全部物理点都会在同一次 `advance` 里顺序追加成 revision 链，
//! 见 `bsp_bridge.rs` `resolve_first_class_episode_edges` 文档）——参照集独立实现「episode
//! 区间覆盖」这同一条已由 dispatch 第三轮 supersede 裁定settled 的判据（判据本身不再是本 bin
//! 的论证对象，`bsp_bridge.rs` 模块头 + 真值表已界定），但**代码路径不共享**：本 bin 用线性扫描
//! + 独立数据结构，零调用 `bsp_bridge` 内部索引/折叠函数——仍能捕获实现层错误（字段取错/
//! off-by-one/漏判类/边界开闭错），只是不再重新论证「episode 覆盖是不是对的判据」（那件事已经
//! 由三轮 supersede + 单元测试负控关闭）。
//!
//! ## 现役拼缝跨对象族不可直接对拍（HIGH-3 ①，如实登记不可执行）
//!
//! 裁定②点名的现役拼缝 `nest::terminal_bits_at_event` 需要 `NestCandidateEvent`（`nest` 模块
//! 自有事件体系，非本票 `cand_event::CandidateEvent`）+ `OwnerAnchorCtx`（owner 判同 oracle）+
//! `event_bsp_book_level` 级别移位；把桥接对象接进这条拼缝需要新构造一整套 nest 侧事件与锚
//! 供给，这本身是一次新的消费接线（违反裁定④「p92/π runner 本票零消费接线」+ 「不重算既有
//! 判据」方法学）。按 dispatch 「跨对象族不可直接对拍则上报改门，不得自替代」的处置指引，
//! 此路在本修复轮判**不可执行**，改用上述两件替代验收物。
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

/// 独立参照集（真独立：不调用 `bsp_bridge` 任何函数/索引结构，线性扫描 + 自有数据形状）：
/// 一类 = episode 区间覆盖（`c_start <= source_index <= interval.1`，同 level/side/中枢指纹）；
/// 二类 = 其一类锚坐标同样按 episode 区间覆盖反查；三类 = `leave_interval.1` 精确等值（v2 三类
/// 锚本轮未改，评审 #670 已验不空洞）。
fn reference_join(classification: &Classification, streams: &CandidateStreams) -> BTreeSet<(u32, usize, &'static str, CandidateKey)> {
    let mut latest: BTreeMap<CandidateKey, CandidateEvent> = BTreeMap::new();
    for batch in streams.iter() {
        for event in batch.iter() {
            latest.insert(event.key, event.clone());
        }
    }
    // 独立数据形状：Vec 线性表，不建 BTreeMap 索引（刻意与生产 `trend_episodes`/`find_episode`
    // 的实现路径分道——只共享「episode 区间覆盖」这条已被 dispatch 裁定settled 的判据本身）。
    let trend_events: Vec<(u32, Side, (usize, i64, i64), usize, usize, CandidateKey)> = latest
        .values()
        .filter(|event| event.kind == CandidateKind::Trend)
        .map(|event| {
            let p = (event.key.parent.center_start, event.key.parent.zd, event.key.parent.zg);
            (event.event_level, event.key.side, p, event.key.c_start, event.interval.1, event.key)
        })
        .collect();
    let trend_by_exact_end: BTreeMap<(u32, Side, (usize, i64, i64), usize), CandidateKey> = trend_events
        .iter()
        .map(|&(level, side, p, _, end, key)| ((level, side, p, end), key))
        .collect();

    let find_covering = |level: u32, side: Side, parent: (usize, i64, i64), source_index: usize| -> Option<CandidateKey> {
        let mut found: Option<CandidateKey> = None;
        for &(el, es, ep, c_start, interval_end, key) in &trend_events {
            if el == level && es == side && ep == parent && c_start <= source_index && source_index <= interval_end {
                found = Some(key);
                break; // 独立扫描不断言唯一性（生产侧的 debug_assert 已在单测覆盖），取首个。
            }
        }
        found
    };

    let mut out = BTreeSet::new();
    for (level_idx, level) in classification.levels.iter().enumerate() {
        for point in level.bsp.iter() {
            let center_fp = match point.center {
                Some(OwnerRef::Center(c)) => Some((c.start_index, c.zd, c.zg)),
                _ => None,
            };
            // 一类：episode 区间覆盖。
            for (set, side, name) in [(point.bits.buy1, Side::Long, "Buy1"), (point.bits.sell1, Side::Short, "Sell1")] {
                if !set {
                    continue;
                }
                let Some(parent) = center_fp else { continue };
                if let Some(ek) = find_covering(level_idx as u32, side, parent, point.source_index) {
                    out.insert((level_idx as u32, point.source_index, name, ek));
                }
            }
            // 三类：leave_interval.1 精确等值（未改判据）。
            for (set, side, name) in [(point.bits.buy3, Side::Long, "Buy3"), (point.bits.sell3, Side::Short, "Sell3")] {
                if !set {
                    continue;
                }
                let (Some(parent), Some(entry)) = (center_fp, point.bits.third_class_entry) else { continue };
                if let Some(&ek) = trend_by_exact_end.get(&(level_idx as u32, side, parent, entry.leave_interval.1)) {
                    out.insert((level_idx as u32, point.source_index, name, ek));
                }
            }
            // 二类：反查同级一类锚，同样按 episode 区间覆盖（HIGH-2 修复覆盖二类）。
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
                if let Some(ek) = find_covering(level_idx as u32, side, parent, anchor_idx) {
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
    // 对拍全量修订历史（非 heads()）：同 episode 多物理点在单次 fresh-full advance 里会顺序
    // 追加成 revision 链，每个被覆盖的物理点都应在 edges() 里留痕（HIGH-3 处置：真独立参照集
    // 对全量历史，不是只对链头）。
    let produced: BTreeSet<(u32, usize, &'static str, CandidateKey)> = bridge
        .edges()
        .iter()
        .map(|edge| (edge.bsp_level, edge.bsp_source_index, class_name(edge.key.bsp.class), edge.key.event))
        .collect();

    let missing_in_bridge: Vec<_> = reference.difference(&produced).collect();
    let extra_in_bridge: Vec<_> = produced.difference(&reference).collect();
    let cmp = missing_in_bridge.len() + extra_in_bridge.len();

    println!(
        "ISSUE668_BRIDGE_BATTERY bars={} reference_edges={} produced_edges={} heads={} missing_in_bridge={} extra_in_bridge={} cmp={}",
        bars.len(),
        reference.len(),
        produced.len(),
        bridge.heads().len(),
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
