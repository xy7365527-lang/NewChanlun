//! #711 跳边/断边影响面存量 census + 断级后续命中率（#685 备料③）。
//!
//! 纪律：只写不判、只读生产判据（`chain_cert`/`cand_event`/`classify_with_tower_events_incremental`
//! 均为既有生产/诊断路径，本 bin 零改动它们）。产出格式仿 `issue550_event_battery` /
//! `ISSUE641_CHAIN*` 先例：每行一个 `P127_*` 标签，纯分桶计数，不重写任何判据。
//!
//! 用法：`cargo run --release --bin p127_skip_impact -- <btc_1m_full.json>`
//!
//! ## 口径（票面要求的两题）
//!
//! **题一（存量 census）**：对 BTC 20k/100k/300k 三个结构窗口，各跑一次因果逐 bar 重放
//! （`ParseLayerIncr` + `TowerCache::classify_with_tower_events_incremental`，与 `issue550_event_battery`
//! 同一因果通道），链簿按 [`CHAIN_ADVANCE_EVERY`] 节拍**周期推进**（末根必推，与 `ISSUE641_CHAIN`
//! 系列同口径——链的覆盖边计算是 O(n²)，不在逐 bar 热路径上）。**首跑发现并订正**：若只在窗口
//! 末根推进一次，断边（见下）在数学上不可能被观测到（[`CHAIN_ADVANCE_EVERY`] 文档有完整推导），
//! 故本 bin 必须周期推进；每次推进后 `book.heads()` 读的是**全窗口历史上出现过的每条链的最新
//! revision**（含早已终态、当前不再是极大路径的旧链），是「存量」的正确读法。产出：
//! - 含 ≥1 跳边（`ChainEdgeKind::Skip`）的链占比、跳边位置分布（`L{parent}->L{child}`）；
//! - 断边（`crossed_nodes` 非空——中间级候选曾存活、后被证伪、边跨过它）的链占比与位置分布
//!   （按被跨过节点自身的 level 计）；跳边与断边**分开计数**——跳边只问「两存活端点级别差
//!   ≥2」，断边问「路径自身节点被证伪并被跨过」，二者可同时发生于同一条边（断边隔的级别数
//!   ≥1 时也记一次跳边），也可各自独立发生（跳边可以是从未有候选的纯级别缺口）；
//! - 三档口径的现存量：(a) 松＝当前 `Closed` 链数（现状可消费数）；(b) 紧＝`Closed` 且零跳边
//!   （N7 严档：全级逐级下降，无一级被跳过）；(c) 分层＝`Closed` 且零断边（允许「缺」——从未
//!   有候选的级别缺口，但不允许「断」——候选存在过又被证伪跨过）。
//!
//! **题二（断级后续命中率）**：对含断边的链（链头 `Confirmed`）与不含断边的对照链（同样链头
//! `Confirmed`），各自看链头候选 `confirmed_at`（因果通道写入的真实 bar 位——单遍逐 bar
//! 推进保证该钟不是终态窗口投影的「窗口末尾」伪影，见 `cand_event.rs::make_revision`）之后
//! N bar 的收盘价方向，与候选 `Side`（`Long`=买点侧/底背驰，期望之后价涨；`Short`=卖点侧/
//! 顶背驰，期望之后价跌）比对。N 与阈值见 [`HIT_N_PRIMARY`]/[`HIT_N_SECONDARY`]/
//! [`HIT_THRESHOLD_BPS`] 头部常量文档。价格前瞻读**结构窗口之外**的原始 bar（数据源 460 万根，
//! 只在装载时多留够 margin，不影响任何结构判据——结构窗口严格只喂前 W 根给
//! `ParseLayerIncr`）。

use newchan_rust::theta_v0::classifier::cand_event::{CandidateEvent, CandidateKey, CandidateState};
use newchan_rust::theta_v0::classifier::chain_cert::{
    ChainCertificateBook, ChainEdgeKind, ChainNodeStatus, ChainStatus, TowerChainCertificate,
};
use newchan_rust::theta_v0::classifier::{self, TowerCache};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{quantize, Bar, Side};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// 三个结构窗口（票面点名）。
const WINDOWS: [usize; 3] = [20_000, 100_000, 300_000];

/// 命中率主读数 N（审定理由，头部文档已述）：1 分钟 bar，500 根 ≈ 8.3 小时——足以让背驰段
/// 确认后的价格走向脱离即时噪声，同时仍在多数确认点之后的可用数据范围内（窗口末尾少量确认点
/// 会落 `out_of_window`，照实计数不静默丢，见 [`HitBucket::out_of_window`]）。
const HIT_N_PRIMARY: usize = 500;
/// 次读数（稳健性交叉核对，同一批确认点、更长地平线）：1000 根 ≈ 16.7 小时。
const HIT_N_SECONDARY: usize = 1000;
/// 涨/跌判定阈值（审定理由）：20 bp（0.2%）—— 1 分钟级别的逐 tick 噪声通常在个位 bp，
/// 20bp 是「方向有效移动」与「原地打转」的粗分界，非精调超参数；本 bin 只用它做一次分类，
/// 不做任何阈值敏感性择优（择优＝判，不属「只写不判」纪律）。
const HIT_THRESHOLD_BPS: f64 = 20.0;
/// 价格前瞻装载 margin：覆盖 [`HIT_N_SECONDARY`] 的最大前瞻 + 安全余量。
const LOOKAHEAD_MARGIN: usize = 2_000;

/// 链簿推进节拍（与 `issue550_event_battery` 的 `chain_every` 同款纪律，末根必推）。
///
/// **关键更正（首跑发现）**：若只在窗口末根推进一次，`ChainCertificateBook::advance` 的路径
/// 只能来自**当次**存活候选构成的覆盖图（`chain_paths`）——见 `chain_cert::mod` 文档「被评估的
/// 路径 = 本次扫描的极大路径 ∪ 簿内非终态链的路径」，单次调用时后者恒空，故断边
/// （`crossed_nodes` 非空——路径节点曾存活、后证伪、被边跨过）在数学上**不可能**被观测到：
/// 一个节点要先在某次推进时作为路径的一部分被记入簿（成为「resident」），此后才谈得上「它证伪
/// 后被跨过」。首跑用单次终态推进，全三窗 `broken_chains` 恒 0——这是推进节拍设计缺陷的产物，
/// 不是数据真的没有断边，故改为窗口内周期推进（末根仍必推，语义与 `ISSUE641_CHAIN` 系列一致）。
const CHAIN_ADVANCE_EVERY: usize = 1_000;

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
        bars.push(raw_bar(&raw, i, prior, tick_size)?);
    }
    Ok(bars)
}

fn raw_bar(raw: &RawBars, i: usize, prior: i64, tick_size: f64) -> Result<Bar, String> {
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
    Ok(Bar {
        source_index: i,
        timestamp: timestamp(&raw.dates[i])?,
        open: values.0,
        high: values.1,
        low: values.2,
        close: values.3,
        volume: volume as i64,
        untradable: values.4 || volume <= 0.0,
    })
}

fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 {
        0.0
    } else {
        a as f64 / b as f64
    }
}

fn main() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or("用法: p127_skip_impact <btc_1m_full.json>")?;
    let config = ThetaConfig::default();
    let max_window = *WINDOWS.iter().max().expect("WINDOWS 非空");
    let load_limit = max_window + HIT_N_SECONDARY + LOOKAHEAD_MARGIN;
    let bars = load(Path::new(&path), config.tick.tick_size, load_limit)?;
    println!(
        "P127_INPUT loaded_bars={} load_limit={} windows={:?}",
        bars.len(),
        load_limit,
        WINDOWS
    );

    for &w in &WINDOWS {
        if w > bars.len() {
            println!("P127_WINDOW_SKIP window={w} reason=insufficient_data available={}", bars.len());
            continue;
        }
        run_window(&bars, w, &config)?;
    }
    Ok(())
}

fn run_window(bars: &[Bar], w: usize, config: &ThetaConfig) -> Result<(), String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = TowerCache::new();
    let mut streams = std::rc::Rc::new(Vec::new());
    let mut book = ChainCertificateBook::default();
    let mut advances = 0usize;
    for (i, bar) in bars[..w].iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let (_, _, s) = classifier::classify_with_tower_events_incremental(&l0, config, &mut cache);
        streams = s;
        if (i + 1) % CHAIN_ADVANCE_EVERY == 0 || i + 1 == w {
            book.advance(&streams, i);
            advances += 1;
        }
    }
    println!("P127_CHAIN_ADVANCES window={w} every={CHAIN_ADVANCE_EVERY} advances={advances}");
    if std::env::var_os("P127_DIAG").is_some() {
        let summary = book.summarize();
        println!(
            "P127_DIAG window={w} nodes_alive={} nodes_falsified={} nodes_absent={} \
             crossed_nodes={} revisions={} idempotent_replay={}",
            summary.nodes_alive,
            summary.nodes_falsified,
            summary.nodes_absent,
            summary.crossed_nodes,
            summary.revisions,
            book.advance(&streams, w - 1).len(),
        );
    }
    let heads = book.heads();

    let mut latest_by_key = BTreeMap::<CandidateKey, &CandidateEvent>::new();
    for stream in streams.iter() {
        for event in stream.iter() {
            latest_by_key.insert(event.key, event);
        }
    }

    println!("P127_WINDOW window={w} bars={w}");
    census(w, &heads);
    hit_rate(w, bars, &heads, &latest_by_key);
    Ok(())
}

fn census(w: usize, heads: &[&TowerChainCertificate]) {
    let total_chains = heads.len();
    let mut skip_chains = 0usize;
    let mut broken_chains = 0usize;
    let mut closed_chains = 0usize;
    let mut strict_closed = 0usize;
    let mut layered_closed = 0usize;
    let mut total_edges = 0usize;
    let mut total_skip_edges = 0usize;
    let mut total_broken_edges = 0usize;
    let mut skip_pos = BTreeMap::<(u32, u32), usize>::new();
    let mut break_pos = BTreeMap::<u32, usize>::new();

    for cert in heads {
        let mut has_skip = false;
        let mut has_broken = false;
        for edge in &cert.edges {
            total_edges += 1;
            if edge.kind == ChainEdgeKind::Skip {
                total_skip_edges += 1;
                has_skip = true;
                *skip_pos.entry((edge.parent.level, edge.child.level)).or_default() += 1;
            }
            if !edge.crossed_nodes.is_empty() {
                has_broken = true;
                total_broken_edges += 1;
                for crossed in &edge.crossed_nodes {
                    *break_pos.entry(crossed.level).or_default() += 1;
                }
            }
        }
        if has_skip {
            skip_chains += 1;
        }
        if has_broken {
            broken_chains += 1;
        }
        if cert.status == ChainStatus::Closed {
            closed_chains += 1;
            if !has_skip {
                strict_closed += 1;
            }
            if !has_broken {
                layered_closed += 1;
            }
        }
    }

    println!(
        "P127_CENSUS window={w} chains={total_chains} closed={closed_chains} \
         skip_chains={skip_chains} skip_chain_ratio={:.4} \
         broken_chains={broken_chains} broken_chain_ratio={:.4}",
        ratio(skip_chains, total_chains),
        ratio(broken_chains, total_chains),
    );
    println!(
        "P127_TIER window={w} loose_a_closed={closed_chains} \
         strict_b_closed_zero_skip={strict_closed} \
         layered_c_closed_zero_break={layered_closed}"
    );
    println!(
        "P127_EDGES window={w} total_edges={total_edges} skip_edges={total_skip_edges} \
         broken_edges={total_broken_edges} skip_edge_ratio={:.4} broken_edge_ratio={:.4}",
        ratio(total_skip_edges, total_edges),
        ratio(total_broken_edges, total_edges),
    );
    for ((parent_level, child_level), count) in &skip_pos {
        println!("P127_SKIP_POS window={w} L{parent_level}->L{child_level} count={count}");
    }
    if skip_pos.is_empty() {
        println!("P127_SKIP_POS window={w} none=true");
    }
    for (level, count) in &break_pos {
        println!("P127_BREAK_POS window={w} crossed_level={level} count={count}");
    }
    if break_pos.is_empty() {
        println!("P127_BREAK_POS window={w} none=true");
    }
}

#[derive(Default)]
struct HitBucket {
    hits: usize,
    misses: usize,
    flat: usize,
    out_of_window: usize,
    no_confirmed_at: usize,
}

impl HitBucket {
    fn evaluated(&self) -> usize {
        self.hits + self.misses
    }

    fn total(&self) -> usize {
        self.hits + self.misses + self.flat + self.out_of_window + self.no_confirmed_at
    }
}

fn hit_rate(
    w: usize,
    bars: &[Bar],
    heads: &[&TowerChainCertificate],
    latest_by_key: &BTreeMap<CandidateKey, &CandidateEvent>,
) {
    for &n in &[HIT_N_PRIMARY, HIT_N_SECONDARY] {
        let mut broken_bucket = HitBucket::default();
        let mut control_bucket = HitBucket::default();
        for cert in heads {
            let root_node = &cert.nodes[0];
            if root_node.status != ChainNodeStatus::Alive {
                continue;
            }
            if root_node.state != Some(CandidateState::Confirmed) {
                continue;
            }
            let has_broken = cert.edges.iter().any(|edge| !edge.crossed_nodes.is_empty());
            let bucket = if has_broken {
                &mut broken_bucket
            } else {
                &mut control_bucket
            };
            let Some(event) = latest_by_key.get(&root_node.key) else {
                bucket.no_confirmed_at += 1;
                continue;
            };
            let Some(confirmed_at) = event.confirmed_at else {
                bucket.no_confirmed_at += 1;
                continue;
            };
            let target_idx = confirmed_at + n;
            if confirmed_at >= bars.len() || target_idx >= bars.len() {
                bucket.out_of_window += 1;
                continue;
            }
            let base_close = bars[confirmed_at].close as f64;
            let target_close = bars[target_idx].close as f64;
            let return_bps = (target_close / base_close - 1.0) * 10_000.0;
            let expect_up = root_node.key.side == Side::Long;
            if return_bps.abs() < HIT_THRESHOLD_BPS {
                bucket.flat += 1;
            } else if (expect_up && return_bps > 0.0) || (!expect_up && return_bps < 0.0) {
                bucket.hits += 1;
            } else {
                bucket.misses += 1;
            }
        }
        print_bucket(w, n, "broken", &broken_bucket);
        print_bucket(w, n, "control_no_break", &control_bucket);
    }
}

fn print_bucket(w: usize, n: usize, label: &str, bucket: &HitBucket) {
    println!(
        "P127_HITRATE window={w} n_bar={n} group={label} total={} evaluated={} hits={} \
         misses={} flat={} out_of_window={} no_confirmed_at={} hit_rate={:.4} \
         hit_rate_incl_flat={:.4}",
        bucket.total(),
        bucket.evaluated(),
        bucket.hits,
        bucket.misses,
        bucket.flat,
        bucket.out_of_window,
        bucket.no_confirmed_at,
        ratio(bucket.hits, bucket.evaluated()),
        ratio(bucket.hits, bucket.total()),
    );
}
