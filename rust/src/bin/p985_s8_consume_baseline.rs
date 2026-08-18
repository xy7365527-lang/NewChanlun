//! #986（N7）E2E-S8 基线：consume_at 消费缝合线验收（map #529）。#1088（SPEC #1085 S4）缩为
//! 读数 bin——装配/投影/幂等/prior 累积全部迁入 `E2eOAssembly`，本 bin 只喂真实数据 + 印三窗
//! 读数（读数口径不变）。
//!
//! ## Seam（一行）
//!
//! ```text
//! btc_1m_full.json(+max_bars) → classify_incremental（逐 bar 因果通道）
//!   → E2eOAssembly::advance（模块两入口之推进）→ E2eOAssembly::consume（模块两入口之消费）
//!   → 验收读数（Closed 链数/受管 BSP 数/BspLink 数/幂等/只收 Closed）
//! ```
//!
//! ## 认识论等级
//!
//! L1（管线正确性读数）：本 bin 只验证「因果通道 → 装配模块 → 消费」的接线与幂等语义成立，
//! **不声明 alpha**、不评估 consume_at 在市场上的有效性（L2/L3 归后续票）。
//!
//! ## #1088 变化（相对 #986 首版）
//!
//! - `advance_every` 参数退役：推进按 bar（`E2eOAssembly::advance` 内部 B 增量化去节拍，
//!   `closed_at` 落真 bar 位）；
//! - 链簿/桥接簿/event_to_bsp 投影/consume prior 全在模块内，本 bin 不自装配；
//! - 幂等/只收 Closed 的机器锁在模块单测（`e2eo::tests`），本 bin 只印读数 + 二次 consume
//!   零增量验收门。
//!
//! 用法：`cargo run --release --features backtest_bin --bin p985_s8_consume_baseline -- \
//!   <btc_1m_full.json> [max_bars]`

use std::collections::BTreeSet;
use std::path::Path;

use newchan_rust::theta_v0::classifier::cand_event::CandidateKey;
use newchan_rust::theta_v0::classifier::chain_cert::{ChainNodeStatus, ChainStatus};
use newchan_rust::theta_v0::classifier::consume_at::{ManagedBspPolicy, PolicyRule};
use newchan_rust::theta_v0::classifier::e2eo::E2eOAssembly;
use newchan_rust::theta_v0::classifier::{self, TowerCache};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{quantize, Bar};
use serde::Deserialize;

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

/// 从日期串提取前 14 位数字作为时间戳。解析失败是输入损坏，不是可默认化的情况。
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
            volume: volume as f64,
            untradable: values.4 || volume <= 0.0,
        });
    }
    Ok(bars)
}

/// 单条 rule 的合法 policy（policy_id=1 / rule_id=1 / slot=0）。rule 语义的完整判定归后续票，
/// 本票只建形状（#983 冻结签名，见 consume_at.rs）。
fn single_rule_policy() -> ManagedBspPolicy {
    ManagedBspPolicy {
        rules: vec![PolicyRule {
            rule_id: 1,
            policy_id: 1,
            slot: 0,
        }],
    }
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p985_s8_consume_baseline <btc_1m_full.json> [max_bars]")?;
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

    // 逐 bar 因果通道 → 装配模块推进（closed_at 真 bar 位；模块内 B 增量化去节拍）。
    let mut parser = ParseLayerIncr::new(&config);
    let mut cache = TowerCache::new();
    let mut assembly = E2eOAssembly::new();
    for (i, bar) in bars.iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let out = classifier::classify_incremental(&l0, &config, &mut cache, &[]);
        assembly.advance(&out.classification, &out.candidate_streams, i);
    }

    // ── 只读读数区（全部借用在此作用域内结束，之后才允许 consume 的 &mut 借用）──
    let (summary, closed_count, non_closed, bridge_readings, overlap_readings) = {
        let summary = assembly.chain_book().summarize();
        let heads = assembly.chain_book().heads();
        let closed: Vec<_> = heads
            .iter()
            .filter(|certificate| certificate.status == ChainStatus::Closed)
            .copied()
            .collect();
        let non_closed = heads
            .iter()
            .filter(|certificate| certificate.status != ChainStatus::Closed)
            .count();

        // 桥接边读数 + event_to_bsp 投影（模块内维护）。
        let bridge_edges = assembly.bridge_book().edges();
        let bridge_heads = assembly.bridge_book().heads();
        let distinct_bridge_keys: BTreeSet<CandidateKey> =
            bridge_edges.iter().map(|edge| edge.key.event).collect();
        let event_to_bsp = assembly.event_to_bsp();

        // 诊断（只读、不判）：event_to_bsp 的 event 键落在 Closed 链存活/任意节点上的重叠计数。
        let mut bridge_events_in_closed_alive = 0usize;
        let mut bridge_events_in_closed_any = 0usize;
        for event_key in event_to_bsp.keys() {
            let mut alive = false;
            let mut any = false;
            for certificate in &closed {
                for node in &certificate.nodes {
                    if node.key == *event_key {
                        any = true;
                        if node.status == ChainNodeStatus::Alive {
                            alive = true;
                        }
                    }
                }
            }
            if alive {
                bridge_events_in_closed_alive += 1;
            }
            if any {
                bridge_events_in_closed_any += 1;
            }
        }

        let bridge_readings = (
            bridge_edges.len(),
            bridge_heads.len(),
            distinct_bridge_keys.len(),
            event_to_bsp.len(),
        );
        let overlap_readings = (bridge_events_in_closed_any, bridge_events_in_closed_alive);
        (
            summary,
            closed.len(),
            non_closed,
            bridge_readings,
            overlap_readings,
        )
    };

    println!(
        "P985_S8_INPUT bars={} max_bars={max_bars} advance=per_bar",
        bars.len(),
    );
    println!(
        "P985_S8_CHAIN chains={} closed={} open={} invalidated={} revisions={} \
         closed_with_zero_segments={}",
        summary.chains,
        summary.closed,
        summary.open,
        summary.invalidated,
        summary.revisions,
        summary.closed_with_zero_segments,
    );
    println!(
        "P985_S8_BRIDGE revisions={} heads={} distinct_events={} event_to_bsp={}",
        bridge_readings.0, bridge_readings.1, bridge_readings.2, bridge_readings.3,
    );
    println!(
        "P985_S8_OVERLAP bridge_events={} in_closed_any_node={} in_closed_alive_node={}",
        bridge_readings.3, overlap_readings.0, overlap_readings.1,
    );

    // 消费：模块两入口之消费（内部纯 consume_at + 跨链 prior 累积）。
    let policy = single_rule_policy();
    let (creations, links) = assembly
        .consume(&policy)
        .map_err(|error| format!("consume 失败: {error:?}"))?;
    let managed_bsp = assembly.prior_bsp().len();
    let bsp_links = assembly.prior_links().len();

    // 幂等：prior 已累积，二次 consume 必须零增量。
    let (second_creations, second_links) = assembly
        .consume(&policy)
        .map_err(|error| format!("二次 consume 失败: {error:?}"))?;

    println!(
        "P985_S8_CONSUME closed_chains={closed_count} creations_total={} managed_bsp={managed_bsp} \
         links_total={} bsp_links={bsp_links} non_closed_not_consumed={non_closed}",
        creations.len(),
        links.len(),
    );
    println!(
        "P985_S8_IDEMPOTENCE second_creations={} second_links={}",
        second_creations.len(),
        second_links.len(),
    );

    // 验收门（读数之上的机器锁）：二次 consume 零增量。
    if !second_creations.is_empty() || !second_links.is_empty() {
        return Err(format!(
            "验收门未过：二次 consume 非零增量 second_creations={} second_links={}",
            second_creations.len(),
            second_links.len()
        ));
    }
    Ok(())
}
