//! #690（口径 v2，#687 三裁裁定）D3 递降钟——塔内候选事件父子级 `first_provable_at` 时序普查。
//!
//! 只写不判：本 bin 只做只读普查 + 分桶计数，不做任何门（不拦截、不改生产路径、不判定裁决）。
//!
//! **口径 v2（#687 裁定，替换 #671 v1）**：父子对 = [`cand_sub::candidate_is_sub`]
//! （`child.event_level < parent.event_level` ∧ `C⊆C` 区间包含——生产既有的跨级候选父子关系
//! 判据，非本 bin 新造）**∧ 方向一致**（`child.key.side == parent.key.side`）。方向错配的对
//! 继续计数（`side_mismatch_pairs`，供对照），但不再进入违规分母/分子（v1 曾把方向错配对也计入
//! 分母，#687 认定这会稀释/污染违规率的可解释性）。
//!
//! 违规定义（不变）：一对合格父子（v2 口径）若两者 `first_provable_at` 均已钉死（`Some`）且
//! `parent.first_provable_at > child.first_provable_at`，即父级首证钟**晚于**子级——是为
//! 「递降钟」违规（因果直觉：父级应先或同时可证，不应晚于子级）。
//!
//! **多重归属不强行唯一**（#687 裁定②）：同方向下一个 child 可被多个 parent 包含，不去重、不
//! 挑「唯一父」。新增归属分布：每个 child 被多少个同方向 parent 包含（`candidate_is_sub` 成立
//! 计数），按 n=0/1/2/3+ 分桶报数。
//!
//! **违规个案 dump**（#687 裁定③）：每条违规打出 child/parent 键标识（级别、side、区间坐标）
//! 与两端 `first_provable_at` 值，供编排者钉因。
//!
//! 分桶：窗口 × 级别对（`child_level → parent_level`）× 方向（`CandidateKey.side`）。
//! 用法：`cargo run --release --bin p126_d3_descending_clock -- <btc_1m_full.json> [w1,w2,...]`
//! （窗口默认 `20000,100000,300000`，#641 电池窗口口径同款）。

use newchan_rust::theta_v0::classifier::cand_event::{
    CandidateEvent, CandidateKey, CandidateStreams,
};
use newchan_rust::theta_v0::classifier::cand_sub::candidate_is_sub;
use newchan_rust::theta_v0::classifier::{self};
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

/// 从日期串提取前 14 位数字作为时间戳。解析失败是输入损坏，不是可默认化的情况——
/// 一律返回 `Err` 并带上原始 date 串，绝不用哨兵值（如 0）冒充有效时间戳。
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
    let raw = load_raw(path)?;
    raw_to_bars(raw, tick_size, limit)
}

fn load_raw(path: &Path) -> Result<RawBars, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    serde_json::from_str(&text).map_err(|error| format!("解析 {} 失败: {error}", path.display()))
}

fn raw_to_bars(raw: RawBars, tick_size: f64, limit: usize) -> Result<Vec<Bar>, String> {
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
    let volume = raw.volumes.get(i).copied().flatten().unwrap_or(0.0);
    let missing = raw.opens[i].is_none()
        || raw.highs[i].is_none()
        || raw.lows[i].is_none()
        || raw.closes[i].is_none();
    let values = if missing {
        (prior, prior, prior, prior, true)
    } else {
        let open = raw.opens[i].unwrap();
        let high = raw.highs[i].unwrap();
        let low = raw.lows[i].unwrap();
        let close = raw.closes[i].unwrap();
        (
            quantize(open, tick_size),
            quantize(high, tick_size),
            quantize(low, tick_size),
            quantize(close, tick_size),
            false,
        )
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

/// 每 key 最新 revision，按 `event_level` 分组（`BTreeMap` ⟹ 级别升序确定）。
///
/// 与 `cand_sub::latest_by_level`（私有）同一口径的独立复算——本 bin 只读探针，不依赖该私有
/// 函数的可见性，也不改 `cand_sub` 模块本身（#671 票禁扩大改动面到 `cand_sub.rs`）。
fn latest_by_level(streams: &CandidateStreams) -> BTreeMap<u32, Vec<&CandidateEvent>> {
    let mut latest = BTreeMap::<CandidateKey, &CandidateEvent>::new();
    for stream in streams.iter() {
        for event in stream.iter() {
            latest.insert(event.key, event);
        }
    }
    let mut by_level = BTreeMap::<u32, Vec<&CandidateEvent>>::new();
    for event in latest.into_values() {
        by_level.entry(event.event_level).or_default().push(event);
    }
    by_level
}

fn side_name(side: Side) -> &'static str {
    match side {
        Side::Long => "Long",
        Side::Short => "Short",
    }
}

#[derive(Default, Clone, Copy)]
struct Bucket {
    /// 满足 `candidate_is_sub` ∧ 方向一致（v2 父子对）且两端 `first_provable_at` 均 `Some`
    /// （可判）的对数——违规率分母。
    judged_pairs: usize,
    /// `judged_pairs` 中 `parent.first_provable_at > child.first_provable_at` 的对数。
    violations: usize,
    /// 满足 v2 父子对但至少一端 `first_provable_at` 为 `None`（尚未可证，不可判）的对数。
    unjudged_pairs: usize,
}

/// 单 child 被多少个同方向 parent 包含（`candidate_is_sub` 成立）的分布——#687 裁定②
/// 「多重归属不强行唯一」的计数载体。n=0/1/2/3+ 四桶。
#[derive(Default, Clone, Copy)]
struct AttributionCounts {
    n0: usize,
    n1: usize,
    n2: usize,
    n3_plus: usize,
}

impl AttributionCounts {
    fn record(&mut self, parent_count: usize) {
        match parent_count {
            0 => self.n0 += 1,
            1 => self.n1 += 1,
            2 => self.n2 += 1,
            _ => self.n3_plus += 1,
        }
    }

    fn total(&self) -> usize {
        self.n0 + self.n1 + self.n2 + self.n3_plus
    }
}

/// 一条违规个案的键标识 + 两端首证钟——#687 裁定③ dump，供编排者钉因。
fn print_violation(
    window_bars: usize,
    child_level: u32,
    parent_level: u32,
    side: &'static str,
    child: &CandidateEvent,
    parent: &CandidateEvent,
    child_at: usize,
    parent_at: usize,
) {
    println!(
        "ISSUE690_D3_VIOLATION window_bars={window_bars} child_level={child_level} \
         parent_level={parent_level} side={side} \
         child_kind={:?} child_interval=({},{}) child_c_start={} child_seg_a=({},{}) \
         child_first_provable_at={child_at} \
         parent_kind={:?} parent_interval=({},{}) parent_c_start={} parent_seg_a=({},{}) \
         parent_first_provable_at={parent_at}",
        child.key.kind,
        child.interval.0,
        child.interval.1,
        child.key.c_start,
        child.key.seg_a.0,
        child.key.seg_a.1,
        parent.key.kind,
        parent.interval.0,
        parent.interval.1,
        parent.key.c_start,
        parent.key.seg_a.0,
        parent.key.seg_a.1,
    );
}

/// 单窗口普查：只读——不改事件流、不产生任何观察或修订，返回值只被打印。
///
/// 口径 v2（#687 裁定①）：父子对 = `candidate_is_sub` ∧ 方向一致。方向错配对仍计入
/// `side_mismatch_pairs`（供对照）但 `continue` 跳过，不进入 `buckets`/归属分布。
fn scan_window(path: &Path, config: &ThetaConfig, max_bars: usize) -> Result<(), String> {
    let bars = load(path, config.tick.tick_size, max_bars)?;
    if bars.is_empty() {
        return Err("输入窗口为空".to_string());
    }

    let mut parser = ParseLayerIncr::new(config);
    let mut l0 = parser.append(bars[0]);
    for bar in bars.iter().copied().skip(1) {
        l0 = parser.append(bar);
    }
    let streams = classifier::classify_with_tower_events(&l0, config).2;
    let by_level = latest_by_level(&streams);

    let mut buckets = BTreeMap::<(u32, u32, &'static str), Bucket>::new();
    let mut side_mismatch_pairs = 0usize;
    let mut attribution = AttributionCounts::default();
    let mut violations: Vec<(
        u32,
        u32,
        &'static str,
        &CandidateEvent,
        &CandidateEvent,
        usize,
        usize,
    )> = Vec::new();

    for (&child_level, children) in &by_level {
        let parent_level = child_level + 1;
        let parents = by_level.get(&parent_level);
        for child in children {
            let mut same_side_parent_count = 0usize;
            if let Some(parents) = parents {
                for parent in parents {
                    if !candidate_is_sub(child, parent) {
                        continue;
                    }
                    if child.key.side != parent.key.side {
                        side_mismatch_pairs += 1;
                        continue;
                    }
                    same_side_parent_count += 1;
                    let side = side_name(child.key.side);
                    let bucket = buckets
                        .entry((child_level, parent_level, side))
                        .or_default();
                    match (child.first_provable_at, parent.first_provable_at) {
                        (Some(c_at), Some(p_at)) => {
                            bucket.judged_pairs += 1;
                            if p_at > c_at {
                                bucket.violations += 1;
                                violations.push((
                                    child_level,
                                    parent_level,
                                    side,
                                    child,
                                    parent,
                                    c_at,
                                    p_at,
                                ));
                            }
                        }
                        _ => bucket.unjudged_pairs += 1,
                    }
                }
            }
            attribution.record(same_side_parent_count);
        }
    }

    for (child_level, parent_level, side, child, parent, c_at, p_at) in &violations {
        print_violation(
            bars.len(),
            *child_level,
            *parent_level,
            side,
            child,
            parent,
            *c_at,
            *p_at,
        );
    }

    let (mut total_judged, mut total_violations, mut total_unjudged) = (0usize, 0usize, 0usize);
    for ((child_level, parent_level, side), bucket) in &buckets {
        total_judged += bucket.judged_pairs;
        total_violations += bucket.violations;
        total_unjudged += bucket.unjudged_pairs;
        let rate = if bucket.judged_pairs == 0 {
            0.0
        } else {
            bucket.violations as f64 / bucket.judged_pairs as f64
        };
        println!(
            "ISSUE690_D3_BUCKET window_bars={} child_level={child_level} parent_level={parent_level} \
             side={side} judged_pairs={} violations={} violation_rate={rate:.6} unjudged_pairs={}",
            bars.len(),
            bucket.judged_pairs,
            bucket.violations,
            bucket.unjudged_pairs,
        );
    }

    let total_rate = if total_judged == 0 {
        0.0
    } else {
        total_violations as f64 / total_judged as f64
    };
    println!(
        "ISSUE690_D3_WINDOW window_bars={} levels={} judged_pairs={total_judged} \
         violations={total_violations} violation_rate={total_rate:.6} unjudged_pairs={total_unjudged} \
         side_mismatch_pairs={side_mismatch_pairs}",
        bars.len(),
        by_level.len(),
    );
    println!(
        "ISSUE690_D3_ATTRIBUTION window_bars={} children_total={} n0={} n1={} n2={} n3_plus={}",
        bars.len(),
        attribution.total(),
        attribution.n0,
        attribution.n1,
        attribution.n2,
        attribution.n3_plus,
    );
    Ok(())
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p126_d3_descending_clock <btc_1m_full.json> [w1,w2,...]")?;
    let windows: Vec<usize> = match args.next() {
        Some(raw) => raw
            .split(',')
            .map(|value| value.parse::<usize>())
            .collect::<Result<_, _>>()
            .map_err(|error| format!("窗口列表非法: {error}"))?,
        None => vec![20_000, 100_000, 300_000],
    };
    let config = ThetaConfig::default();
    for window in windows {
        scan_window(Path::new(&path), &config, window)?;
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
