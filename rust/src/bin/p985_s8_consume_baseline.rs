//! #986（N7）E2E-S8 基线：consume_at 消费缝合线验收（map #529）。
//!
//! ## Seam（一行）
//!
//! ```text
//! btc_1m_full.json(+max_bars) → classify（逐 bar 因果通道）→ ChainCertificateBook(Closed 链)
//!   + BspBridgeBook(event_to_bsp 映射) → consume_at → 验收读数（Closed 链数/受管 BSP 数/
//!   BspLink 数/幂等/只收 Closed）
//! ```
//!
//! ## 认识论等级
//!
//! L1（管线正确性读数）：本 bin 只验证「现役对象 → consume_at」的接线与幂等语义成立，
//! **不声明 alpha**、不评估 consume_at 在市场上的有效性（L2/L3 归后续票）。
//!
//! ## 冻结语义来源
//!
//! 语义已冻结（roadmap 八道缝合线「E2E-S1–S8 = 实证门…S8 未全过只能称塔内近似」，
//! `git show 640609071d:chanlun/plans/mainline-merged-roadmap-20260717.md`）。
//! 本票消费的是**塔内 Closed 链证书**（`TowerChainCertificate`），样本以实跑 Closed 链数为准，
//! 不沿用 charting 侧「66 张 nest 证书」代表数。
//!
//! ## 管线 spec（只经现役库公共 API，不重写判据）
//!
//! 1. 加载 `analysis/data_cache/btc_1m_full.json`，`max_bars` 窗口参数。
//! 2. 逐 bar 因果分类：`classifier::classify_with_tower_events_incremental`（同一
//!    `ParseLayerIncr` 血缘 + 同一 `TowerCache`），末 bar 输出 bit-exact 等价于
//!    `classify_with_tower_events`（#93 铁律）。因果通道是 `closed_at` 真 bar 位的前提
//!    （终态窗口投影会把全部钟钉在窗口末尾，见 p127 头注）。
//! 3. 链证书：`ChainCertificateBook::default()`，按 `advance_every` 节拍逐 `as_of`
//!    `advance(&streams, as_of)`（末根必推，与 issue550/p127 同口径），收集
//!    `status == Closed` 的 `TowerChainCertificate`。
//! 4. 桥接边：`BspBridgeBook::default()`，同节拍逐 `as_of` `advance(...)`，从 `edges()` 的
//!    `BridgeKey{event, bsp}` 投影 `event_to_bsp: HashMap<CandidateKey, BspStructuralKey>`。
//! 5. consume_at：对每条 Closed 链（按 `closed_at` 升序），`consume_at(as_of=closed_at,
//!    &prior_bsp, &prior_links, &cert, &policy, &event_to_bsp)`，`policy` = 单条 rule 的合法
//!    policy。跨链 prior 累积 ⟹ 受管 BSP 数 = 唯一 BspStructuralKey 数、BspLink 数 = 唯一
//!    LinkKey 数。
//! 6. 幂等：每条 Closed 链连续 consume 两次，第二次（prior 含第一次产出）必须零增量。
//!    只收 Closed：每条非 Closed 链头调 consume_at 必须 `Err(NoConsumption)`。
//!
//! 用法：`cargo run --release --features backtest_bin --bin p985_s8_consume_baseline -- \
//!   <btc_1m_full.json> [max_bars] [advance_every]`

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use newchan_rust::theta_v0::classifier::bsp_bridge::{BspBridgeBook, BspStructuralKey};
use newchan_rust::theta_v0::classifier::cand_event::CandidateKey;
use newchan_rust::theta_v0::classifier::chain_cert::{
    ChainCertificateBook, ChainStatus, TowerChainCertificate,
};
use newchan_rust::theta_v0::classifier::consume_at::{
    consume_at, BspLink, ConsumeError, LinkKey, ManagedBsp, ManagedBspPolicy, PolicyRule,
};
use newchan_rust::theta_v0::classifier::{self, TowerCache};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{quantize, Bar};
use serde::Deserialize;

/// 链簿/桥接簿推进节拍（末根必推；默认 5000 = issue550/#641 同款口径，复现 18/89/293 三窗
/// 链总数）。覆盖边计算 O(n²)，逐 bar 推进在 10 万级窗口不可行；节拍是显式声明口径，
/// 随读数一并印出，不是静默采样。
const DEFAULT_ADVANCE_EVERY: usize = 5_000;

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
            volume: volume as i64,
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

/// 从桥接边全量修订史投影「候选事件 → BSP」映射（`BridgeKey{event, bsp}` 投影，bsp_bridge.rs:163）。
/// 迭代 `edges()`（append-only 全量修订史），后修订覆盖先修订（last-wins）；同一 event 映射多个
/// BspStructuralKey 时（多级别/多点类/多锚），最后入簿的键胜出——本投影是有损函数投影，只服务
/// consume_at 的 `event_to_bsp` 形参（#982 A：调用方预解析、只读不查簿）。
fn project_event_to_bsp(
    edges: &[newchan_rust::theta_v0::classifier::bsp_bridge::BspBridgeEdge],
) -> HashMap<CandidateKey, BspStructuralKey> {
    let mut map = HashMap::new();
    for edge in edges {
        map.insert(edge.key.event, edge.key.bsp.clone());
    }
    map
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p985_s8_consume_baseline <btc_1m_full.json> [max_bars] [advance_every]")?;
    let max_bars = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| format!("max_bars 非法: {error}"))?
        .unwrap_or(100_000);
    let advance_every = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| format!("advance_every 非法: {error}"))?
        .unwrap_or(DEFAULT_ADVANCE_EVERY)
        .max(1);

    let config = ThetaConfig::default();
    let bars = load(Path::new(&path), config.tick.tick_size, max_bars)?;
    if bars.is_empty() {
        return Err("输入窗口为空".to_string());
    }

    let mut parser = ParseLayerIncr::new(&config);
    let mut cache = TowerCache::new();
    let mut chain_book = ChainCertificateBook::default();
    let mut bridge_book = BspBridgeBook::default();
    let mut advances = 0usize;
    for (i, bar) in bars.iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let (classification, _tower, streams) =
            classifier::classify_with_tower_events_incremental(&l0, &config, &mut cache);
        if (i + 1) % advance_every == 0 || i + 1 == bars.len() {
            chain_book.advance(&streams, i);
            bridge_book.advance(&classification, &streams, i);
            advances += 1;
        }
    }

    // 链簿读数（heads() = 每 key 最新 revision，含早已终态、当前不再是极大路径的旧链——存量）。
    let summary = chain_book.summarize();
    let heads = chain_book.heads();
    let mut closed: Vec<&TowerChainCertificate> = heads
        .iter()
        .filter(|certificate| certificate.status == ChainStatus::Closed)
        .copied()
        .collect();
    // consume 按 closed_at 升序：prior 的 created_at/written_at 恒 ≤ 当前 as_of，不触发
    // InconsistentState；同 as_of 内按 key 升序去随机化。
    closed.sort_by_key(|certificate| (certificate.closed_at, certificate.key.clone()));

    // 桥接边读数 + event_to_bsp 投影。
    let bridge_edges = bridge_book.edges();
    let bridge_heads = bridge_book.heads();
    let distinct_bridge_keys: BTreeSet<CandidateKey> =
        bridge_edges.iter().map(|edge| edge.key.event).collect();
    let event_to_bsp = project_event_to_bsp(bridge_edges);

    println!(
        "P985_S8_INPUT bars={} max_bars={} advance_every={advance_every} advances={advances}",
        bars.len(),
        max_bars,
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
        bridge_edges.len(),
        bridge_heads.len(),
        distinct_bridge_keys.len(),
        event_to_bsp.len(),
    );

    let policy = single_rule_policy();
    let mut prior_bsp: HashMap<BspStructuralKey, ManagedBsp> = HashMap::new();
    let mut prior_links: HashMap<LinkKey, BspLink> = HashMap::new();
    let mut creations_total = 0usize;
    let mut links_total = 0usize;
    let mut closed_consumed = 0usize;
    let mut closed_without_closed_at = 0usize;
    let mut closed_errors: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut idempotency_violations = 0usize;

    for certificate in &closed {
        let Some(as_of) = certificate.closed_at else {
            closed_without_closed_at += 1;
            continue;
        };
        match consume_at(
            as_of,
            &prior_bsp,
            &prior_links,
            certificate,
            &policy,
            &event_to_bsp,
        ) {
            Ok((creations, links)) => {
                creations_total += creations.len();
                links_total += links.len();
                closed_consumed += 1;

                // 幂等：同一 (as_of, prior, cert) 连续第二次，prior 含第一次产出 ⟹ 零增量。
                let mut prior_bsp_replay = prior_bsp.clone();
                let mut prior_links_replay = prior_links.clone();
                for creation in &creations {
                    prior_bsp_replay.insert(creation.bsp.key.clone(), creation.bsp.clone());
                }
                for link in &links {
                    prior_links_replay.insert(link.key.clone(), link.clone());
                }
                match consume_at(
                    as_of,
                    &prior_bsp_replay,
                    &prior_links_replay,
                    certificate,
                    &policy,
                    &event_to_bsp,
                ) {
                    Ok((replay_creations, replay_links)) => {
                        if !replay_creations.is_empty() || !replay_links.is_empty() {
                            idempotency_violations += 1;
                        }
                    }
                    Err(error) => {
                        *closed_errors.entry(error_name(&error)).or_default() += 1;
                        idempotency_violations += 1;
                    }
                }

                // 并入跨链 prior（受管 BSP 跨链只创建一次、组内多链接）。
                for creation in creations {
                    prior_bsp.insert(creation.bsp.key.clone(), creation.bsp);
                }
                for link in links {
                    prior_links.insert(link.key.clone(), link);
                }
            }
            Err(error) => {
                *closed_errors.entry(error_name(&error)).or_default() += 1;
            }
        }
    }

    // 诊断（只读、不判）：event_to_bsp 的 event 键落在 Closed 链存活/任意节点上的重叠计数——
    // 解释 consume 零产出是「缝合线空域」还是「节点被跨过/缺席」。
    let mut bridge_events_in_closed_alive = 0usize;
    let mut bridge_events_in_closed_any = 0usize;
    for event_key in event_to_bsp.keys() {
        let mut alive = false;
        let mut any = false;
        for certificate in &closed {
            for node in &certificate.nodes {
                if node.key == *event_key {
                    any = true;
                    if node.status
                        == newchan_rust::theta_v0::classifier::chain_cert::ChainNodeStatus::Alive
                    {
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
    println!(
        "P985_S8_OVERLAP bridge_events={} in_closed_any_node={} in_closed_alive_node={}",
        event_to_bsp.len(),
        bridge_events_in_closed_any,
        bridge_events_in_closed_alive,
    );

    // 只收 Closed：每条非 Closed 链头（Open/Invalidated）调 consume_at 必须 Err(NoConsumption)。
    // NoConsumption 是 consume_at 的第一道校验，与 as_of/prior 无关，用空 prior 隔离测。
    let mut non_closed_checked = 0usize;
    let mut non_closed_rejected = 0usize;
    let mut non_closed_unexpected = 0usize;
    for certificate in heads.iter() {
        if certificate.status == ChainStatus::Closed {
            continue;
        }
        non_closed_checked += 1;
        match consume_at(
            certificate.revision_at,
            &HashMap::new(),
            &HashMap::new(),
            certificate,
            &policy,
            &event_to_bsp,
        ) {
            Err(ConsumeError::NoConsumption) => non_closed_rejected += 1,
            _ => non_closed_unexpected += 1,
        }
    }

    println!(
        "P985_S8_CONSUME closed_chains={} consumed={} closed_without_closed_at={} \
         creations_total={} managed_bsp={} links_total={} bsp_links={} closed_errors={:?}",
        closed.len(),
        closed_consumed,
        closed_without_closed_at,
        creations_total,
        prior_bsp.len(),
        links_total,
        prior_links.len(),
        closed_errors,
    );
    println!(
        "P985_S8_IDEMPOTENCE checked={} violations={}",
        closed_consumed, idempotency_violations,
    );
    println!(
        "P985_S8_NON_CLOSED_REJECTED checked={} rejected_no_consumption={} unexpected={}",
        non_closed_checked, non_closed_rejected, non_closed_unexpected,
    );

    // 验收门（读数之上的机器锁）：四项红条件任一命中 ⟹ exit FAILURE。
    let violations = idempotency_violations
        + non_closed_unexpected
        + closed_without_closed_at
        + closed_errors.values().sum::<usize>();
    if violations > 0 {
        return Err(format!(
            "验收门未过：idempotency_violations={idempotency_violations} \
             non_closed_unexpected={non_closed_unexpected} \
             closed_without_closed_at={closed_without_closed_at} \
             closed_errors={closed_errors:?}"
        ));
    }
    Ok(())
}

fn error_name(error: &ConsumeError) -> &'static str {
    match error {
        ConsumeError::NoConsumption => "NoConsumption",
        ConsumeError::InvalidPolicy => "InvalidPolicy",
        ConsumeError::InconsistentState => "InconsistentState",
        ConsumeError::LateAuthorization => "LateAuthorization",
    }
}
