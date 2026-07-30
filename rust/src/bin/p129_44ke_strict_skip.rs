//! #751（票面②，#685 备料）：44 课式严格 skip 诊断标注 census（读数层，不进门）。
//!
//! 纪律：只写不判、只读生产既有簿（`chain_cert::SkippedLevel` + `Classification.levels[].cp_ownership`
//! + `ThirdClassInCp`），零改动任何生产判据；不改 `ChainEdge` 结构、不进 `Closed` 判定、不改
//! N7 消费门（ADR-0009 严档谓词仍只是 `Closed ∧ 零跳`，本 bin 的读数不叠加为门条件）。
//!
//! ## 票面口径
//!
//! 对每条 `ChainEdgeKind::Skip` 边的每个 [`SkippedLevel`]（被跳过的中间级别），检查该级别在
//! **父区间邻域**内是否存在「**完成的次级别中枢 + 第三类买卖点**」（44 课「小转大」替代必要条件，
//! `chain_cert::mod` 头部文档已明示该证据路径由 #636 裁定③留 fog——本 bin 是首次接通它的诊断读数，
//! 不改 `chain_cert` 本身）。命中 ⟹ 标「44 课式严格 skip」（跳级有替代证据，非几何噪声）；未命中
//! ⟹ 标「噪声 skip」（跳级无此替代证据，`#741` 三分支①/②的候选，本 bin 不进一步区分）。
//!
//! ## 数据来源与口径落地（票面①的核查结论——可读，见下）
//!
//! - **完成的次级别中枢**：`Classification.levels[level].cp_ownership`（`CpScanOwnership`，与
//!   `centers` 1:1）过滤 `lifecycle == CpLifecycleStatus::Closed`——`recursive_tower.rs` 生产字段，
//!   本 bin 零新增判据，只读既有终态。
//! - **第三类买卖点**：同一 `CpScanOwnership.third_class_in_c: Option<ThirdClassInCp>`——终态
//!   `c_p` 内部第三类离开/回试结构，`Closed` 时才可能 `Some`（`recursive_tower.rs` 构造点原样），
//!   携 `side: Side`（`Long`=三买，`Short`=三卖，与 `BspPoint.bits.buy3/sell3` 同源）。
//! - **父区间**：`ChainEdge::parent` 经候选事件流（`latest_by_key`）取其
//!   `CandidateEvent::interval`（C 段闭区间，`source_index` 坐标）——与 `chain_cert::build_edge`
//!   计算 `SkippedLevel::inside_parent` 时用的同一个父区间字段，未引入第二个"父区间"定义。
//! - **邻域定义**（本 bin 的操作化选择，非新裁；两档并报，不隐藏敏感性）：
//!   1. **containment**（首选尝试）：镜像 `chain_cert` 计算 `inside_parent` 的同一条包含谓词
//!      [`interval_is_sub`]（`cand_event.rs` 单一来源，闭区间含端点、相切算包含），应用到
//!      `CpScanOwnership.b_center` 的 `(start_index, end_index)` 上——即"完成的次级别中枢的核心
//!      窗口整体落在父 C 段区间内"。**首跑发现并订正**：真实数据面上此档恒 0（诊断见
//!      `P129_DIAG2`：`side_match>0` 但 `strict` 恒 false）——次级别中枢的核心窗口跨度（`b_center`
//!      从识别到当前/终结的整段 source_index span）系统性大于父级单条 C 段的跨度，不是"级别越小
//!      跨度越窄"的直觉（中枢可持续延伸，父 C 段只是父级单条走势腿），全包含在几何上近乎不可能
//!      满足——这条镜像谓词对"候选 C 段 vs 候选 C 段"（同类对象）成立，搬到"中枢核心窗口 vs
//!      候选 C 段"（异类对象）不传递。
//!   2. **overlap**（订正后的主读数）：两区间不相离（`!intervals_are_disjoint`，`cand_event.rs`
//!      单一来源，同一相离/相切/退化判据族的另一支，非本 bin 新造）——次级别中枢的核心窗口与父
//!      C 段有交集即算落入邻域。两档均落盘（`P129_STRICT_*_CONTAIN`/`P129_STRICT_*_OVERLAP`），
//!      报告以 overlap 为主读数、containment 恒零之事实照实登记。
//! - **方向匹配**（票面字面）：顶背驰/卖点侧链（`edge.parent.side == Side::Short`）→ 三类卖
//!   （`tc.side == Side::Short`）；底背驰/买点侧链（`Side::Long`）→ 三类买（`Side::Long`）。
//!   直接比较 `tc.side == edge.parent.side`（两者同一 `Side` 编码约定，无需换算）。
//! - **"缺"不预先剔除**：`SkippedLevel.alive_at_level == 0`（该级别无存活*候选*）不代表该级别
//!   无*中枢*——候选（`cand_event`）与中枢识别（`center`/`recursive_tower`）是两条独立产线，
//!   一个跳级即使候选缺失，`cp_ownership` 仍可能有完成的中枢+三类点（本 bin 存在的意义正在于此：
//!   candidate 视角的"缺"未必是 44 课视角的"无替代证据"）。故本 bin 对每个 `SkippedLevel` 一律
//!   评估，不因 `alive_at_level == 0` 跳过；同时把 `chain_cert` 已有的缺/断-在外/断-在内三分
//!   （`inside_parent`/`alive_at_level`）与本 bin 的严格/噪声二分做交叉输出，供报告核对两条产线
//!   是否系统性重合或分离。
//!
//! ## 用法
//!
//! `cargo run --release --bin p129_44ke_strict_skip -- <btc_1m_full.json>`

use newchan_rust::theta_v0::classifier::cand_event::{
    interval_is_sub, intervals_are_disjoint, CandidateEvent, CandidateKey,
};
use newchan_rust::theta_v0::classifier::chain_cert::{
    ChainCertificateBook, ChainEdgeKind, ChainStatus, TowerChainCertificate,
};
use newchan_rust::theta_v0::classifier::recursive_tower::CpLifecycleStatus;
use newchan_rust::theta_v0::classifier::{self, Classification, TowerCache};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{quantize, Bar, Side};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// 三个结构窗口（票面点名，与 p127 同窗口，读数可交叉核对）。
const WINDOWS: [usize; 3] = [20_000, 100_000, 300_000];

/// 链簿推进节拍（与 p127 同款纪律——首跑发现的更正见 p127 头部文档，本 bin 直接沿用）。
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
        .ok_or("用法: p129_44ke_strict_skip <btc_1m_full.json>")?;
    let config = ThetaConfig::default();
    let max_window = *WINDOWS.iter().max().expect("WINDOWS 非空");
    let bars = load(Path::new(&path), config.tick.tick_size, max_window)?;
    println!(
        "P129_INPUT loaded_bars={} windows={:?}",
        bars.len(),
        WINDOWS
    );

    for &w in &WINDOWS {
        if w > bars.len() {
            println!("P129_WINDOW_SKIP window={w} reason=insufficient_data available={}", bars.len());
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
    let mut final_classification: Option<Classification> = None;
    for (i, bar) in bars[..w].iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let (classification, _, s) = classifier::classify_with_tower_events_incremental(&l0, config, &mut cache);
        streams = s;
        if (i + 1) % CHAIN_ADVANCE_EVERY == 0 || i + 1 == w {
            book.advance(&streams, i);
        }
        final_classification = Some(classification);
    }
    let classification = final_classification.expect("非空窗口至少一根 bar");
    let heads = book.heads();

    let mut latest_by_key = BTreeMap::<CandidateKey, &CandidateEvent>::new();
    for stream in streams.iter() {
        for event in stream.iter() {
            latest_by_key.insert(event.key, event);
        }
    }

    println!("P129_WINDOW window={w} bars={w} chains={}", heads.len());
    if std::env::var_os("P129_DIAG").is_some() {
        for (level, ls) in classification.levels.iter().enumerate() {
            let closed = ls
                .cp_ownership
                .iter()
                .filter(|cp| cp.lifecycle == CpLifecycleStatus::Closed)
                .count();
            let closed_third = ls
                .cp_ownership
                .iter()
                .filter(|cp| {
                    cp.lifecycle == CpLifecycleStatus::Closed && cp.third_class_in_c.is_some()
                })
                .count();
            println!(
                "P129_DIAG window={w} level={level} centers={} cp_ownership={} closed={closed} \
                 closed_with_third={closed_third}",
                ls.centers.len(),
                ls.cp_ownership.len(),
            );
        }
    }
    ke44_strict_skip(w, &heads, &classification, &latest_by_key);
    Ok(())
}

/// 每个 (parent_level, skip_level) 桶的严格/噪声计数。
#[derive(Default, Clone, Copy)]
struct StrictBucket {
    strict: usize,
    noise: usize,
}

impl StrictBucket {
    fn total(&self) -> usize {
        self.strict + self.noise
    }
    fn record(&mut self, strict: bool) {
        if strict {
            self.strict += 1;
        } else {
            self.noise += 1;
        }
    }
}

/// 邻域判据的两档口径（票面口径注释头部已述：containment 首跑发现恒零，overlap 是订正后主读数）。
#[derive(Clone, Copy)]
struct NeighborhoodTiers {
    /// 完成中枢核心窗口整体落在父 C 段区间内（`interval_is_sub`，与 `inside_parent` 同一谓词）。
    contain: bool,
    /// 完成中枢核心窗口与父 C 段区间有交集（`!intervals_are_disjoint`）。
    overlap: bool,
}

fn evidence_tiers(
    classification: &Classification,
    level: u32,
    side: Side,
    parent_interval: (usize, usize),
) -> NeighborhoodTiers {
    let Some(level_state) = classification.levels.get(level as usize) else {
        return NeighborhoodTiers { contain: false, overlap: false };
    };
    let mut tiers = NeighborhoodTiers { contain: false, overlap: false };
    for cp in level_state.cp_ownership.iter() {
        if cp.lifecycle != CpLifecycleStatus::Closed {
            continue;
        }
        if !cp.third_class_in_c.map_or(false, |tc| tc.side == side) {
            continue;
        }
        let b_interval = (cp.b_center.start_index, cp.b_center.end_index);
        if interval_is_sub(b_interval, parent_interval) {
            tiers.contain = true;
        }
        if !intervals_are_disjoint(b_interval, parent_interval) {
            tiers.overlap = true;
        }
    }
    tiers
}

fn ke44_strict_skip(
    w: usize,
    heads: &[&TowerChainCertificate],
    classification: &Classification,
    latest_by_key: &BTreeMap<CandidateKey, &CandidateEvent>,
) {
    let mut by_pair_contain = BTreeMap::<(u32, u32), StrictBucket>::new();
    let mut by_pair_overlap = BTreeMap::<(u32, u32), StrictBucket>::new();
    let mut by_side_overlap = BTreeMap::<Side, StrictBucket>::new();
    let mut total_contain = StrictBucket::default();
    let mut total_overlap = StrictBucket::default();
    // 与 ADR-0009 严档/悬置档关系：只在「悬置档」（Closed ∧ 含跳）链内的跳级实例上单独计数。
    let mut suspended_overlap = StrictBucket::default();
    // 与 #736 缺/断-在外/断-在内三分的交叉（`alive_at_level`/`inside_parent`，chain_cert 既有分档）；overlap 口径。
    let mut cross_missing = StrictBucket::default();
    let mut cross_broken_outside = StrictBucket::default();
    let mut cross_broken_inside = StrictBucket::default();

    for cert in heads {
        let is_suspended =
            cert.status == ChainStatus::Closed && cert.edges.iter().any(|e| e.kind == ChainEdgeKind::Skip);
        for edge in &cert.edges {
            if edge.kind != ChainEdgeKind::Skip {
                continue;
            }
            let Some(&parent_event) = latest_by_key.get(&edge.parent) else {
                continue;
            };
            let side = edge.parent.side;
            for gap in &edge.skipped_levels {
                let tiers = evidence_tiers(classification, gap.level, side, parent_event.interval);

                if std::env::var_os("P129_DIAG2").is_some() {
                    if let Some(level_state) = classification.levels.get(gap.level as usize) {
                        let any_closed_third = level_state
                            .cp_ownership
                            .iter()
                            .filter(|cp| cp.lifecycle == CpLifecycleStatus::Closed && cp.third_class_in_c.is_some())
                            .count();
                        let side_match = level_state
                            .cp_ownership
                            .iter()
                            .filter(|cp| {
                                cp.lifecycle == CpLifecycleStatus::Closed
                                    && cp.third_class_in_c.map_or(false, |tc| tc.side == side)
                            })
                            .count();
                        if any_closed_third > 0 {
                            println!(
                                "P129_DIAG2 window={w} parent_level={} skip_level={} side={side:?} \
                                 parent_interval={:?} any_closed_third_at_level={any_closed_third} \
                                 side_match={side_match} contain={} overlap={}",
                                edge.parent.level, gap.level, parent_event.interval,
                                tiers.contain, tiers.overlap,
                            );
                        }
                    }
                }

                by_pair_contain.entry((edge.parent.level, gap.level)).or_default().record(tiers.contain);
                by_pair_overlap.entry((edge.parent.level, gap.level)).or_default().record(tiers.overlap);
                by_side_overlap.entry(side).or_default().record(tiers.overlap);
                total_contain.record(tiers.contain);
                total_overlap.record(tiers.overlap);
                if is_suspended {
                    suspended_overlap.record(tiers.overlap);
                }
                if gap.alive_at_level == 0 {
                    cross_missing.record(tiers.overlap);
                } else if gap.inside_parent == 0 {
                    cross_broken_outside.record(tiers.overlap);
                } else {
                    cross_broken_inside.record(tiers.overlap);
                }
            }
        }
    }

    for ((parent_level, skip_level), bucket) in &by_pair_overlap {
        let contain = by_pair_contain.get(&(*parent_level, *skip_level)).copied().unwrap_or_default();
        println!(
            "P129_STRICT_BY_PAIR window={w} L{parent_level}->L{skip_level} \
             overlap_strict={} overlap_noise={} contain_strict={} contain_noise={} total={} \
             overlap_strict_ratio={:.4} contain_strict_ratio={:.4}",
            bucket.strict,
            bucket.noise,
            contain.strict,
            contain.noise,
            bucket.total(),
            ratio(bucket.strict, bucket.total()),
            ratio(contain.strict, contain.total()),
        );
    }
    if by_pair_overlap.is_empty() {
        println!("P129_STRICT_BY_PAIR window={w} none=true");
    }
    for (side, bucket) in &by_side_overlap {
        println!(
            "P129_STRICT_BY_SIDE window={w} side={side:?} overlap_strict={} overlap_noise={} \
             total={} overlap_strict_ratio={:.4}",
            bucket.strict,
            bucket.noise,
            bucket.total(),
            ratio(bucket.strict, bucket.total()),
        );
    }
    println!(
        "P129_STRICT_TOTAL window={w} overlap_strict={} overlap_noise={} contain_strict={} \
         contain_noise={} total={} overlap_strict_ratio={:.4} contain_strict_ratio={:.4}",
        total_overlap.strict,
        total_overlap.noise,
        total_contain.strict,
        total_contain.noise,
        total_overlap.total(),
        ratio(total_overlap.strict, total_overlap.total()),
        ratio(total_contain.strict, total_contain.total()),
    );
    println!(
        "P129_STRICT_SUSPENDED window={w} overlap_strict={} overlap_noise={} total={} \
         overlap_strict_ratio_in_suspended={:.4}",
        suspended_overlap.strict,
        suspended_overlap.noise,
        suspended_overlap.total(),
        ratio(suspended_overlap.strict, suspended_overlap.total()),
    );
    println!(
        "P129_STRICT_CROSS736 window={w} missing_strict={} missing_noise={} \
         broken_outside_strict={} broken_outside_noise={} broken_inside_strict={} \
         broken_inside_noise={}",
        cross_missing.strict,
        cross_missing.noise,
        cross_broken_outside.strict,
        cross_broken_outside.noise,
        cross_broken_inside.strict,
        cross_broken_inside.noise,
    );
}
