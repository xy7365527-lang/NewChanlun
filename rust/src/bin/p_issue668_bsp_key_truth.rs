//! #668（N4）BSP 结构身份真值表——修复轮 1（#670 影子评审 HIGH-1 回炉，第三轮 supersede 裁定①，
//! `chanlun/review-results/issue668-n4-fix-round1-20260729.md`）。
//!
//! ## 口径订正（撤销「键唯一性」旧断言）
//!
//! 旧版本本 bin 验的是「v2 键唯一性」（`ambiguous_keys=0`），而唯一性之所以恒成立，是因为验前
//! 判据（一类点 `source_index == 候选当前右端` 精确等值）把 62% 的一类点排除在检验域外——
//! `ambiguous_keys=0` 是排除规则的算术必然，不是键的区分力（评审 #670 HIGH-1 实证，300k 窗
//! 11/29 有键、5 组撞键、最坏一组 4 点）。
//!
//! 第三轮 supersede 裁定①**撤销「键唯一性」这个验收目标本身**：一类点身份 = episode，同 episode
//! 多物理点 = 同一候选身份的修订史，撞键**自动消解**、不需要任何区分量。本 bin 因此改验裁定①
//! 明文授权的真实不变量——**episode 归属唯一**：一个一类点只能落在一个 episode 的
//! `[c_start, interval.1]` 区间内（同 level/side/中枢指纹），不能同时归属两个 episode（若能，
//! 说明两个不同破中枢事件的区间重叠，是数据/候选扫描层面的问题，不是本对象能吞的歧义）。
//! 生产代码 `bsp_bridge::find_episode` 用 `debug_assert` 机器化同一不变量。
//!
//! 二类同样按其一类锚坐标验证 episode 归属唯一（HIGH-2 修复覆盖二类）；三类判据本轮未改
//! （leave_interval.1 精确等值，评审已验不空洞），仍报覆盖率但不纳入「归属唯一」检验（三类近零
//! 覆盖是候选域结构性错位，归 #688，不在本票范围）。
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

/// 点类：六 bit 之一，逐 bit 独立入检验（一个 `BspPoint` 可能同时贡献多把键，如 2B/3B 共存）。
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

    fn name(self) -> &'static str {
        match self {
            PointClass::Buy1 => "Buy1",
            PointClass::Buy2 => "Buy2",
            PointClass::Buy3 => "Buy3",
            PointClass::Sell1 => "Sell1",
            PointClass::Sell2 => "Sell2",
            PointClass::Sell3 => "Sell3",
        }
    }

    fn is_first_or_second(self) -> bool {
        !matches!(self, PointClass::Buy3 | PointClass::Sell3)
    }
}

/// 一次「破中枢」episode：候选事件的区间身份（`c_start` 已闭合；`interval_end` 会随 `as_of`
/// 生长，只用作区间上界，不入身份——与生产 `bsp_bridge::TrendEpisode` 同精神，独立实现）。
struct Episode {
    parent: ParentFingerprint,
    side: Side,
    c_start: usize,
    interval_end: usize,
}

fn episodes_by_level(streams: &CandidateStreams) -> BTreeMap<u32, Vec<Episode>> {
    let mut latest: BTreeMap<_, CandidateEvent> = BTreeMap::new();
    for batch in streams.iter() {
        for event in batch.iter() {
            latest.insert(event.key, event.clone());
        }
    }
    let mut by_level: BTreeMap<u32, Vec<Episode>> = BTreeMap::new();
    for event in latest.into_values() {
        if event.kind != CandidateKind::Trend {
            continue;
        }
        by_level.entry(event.event_level).or_default().push(Episode {
            parent: event.key.parent,
            side: event.key.side,
            c_start: event.key.c_start,
            interval_end: event.interval.1,
        });
    }
    by_level
}

fn owning_episodes<'a>(
    episodes: &'a [Episode],
    side: Side,
    parent: ParentFingerprint,
    source_index: usize,
) -> Vec<&'a Episode> {
    episodes
        .iter()
        .filter(|ep| ep.side == side && ep.parent == parent && ep.c_start <= source_index && source_index <= ep.interval_end)
        .collect()
}

fn resolve_fingerprint(level: &LevelState, class: PointClass, idx_in_level: usize) -> Option<ParentFingerprint> {
    let point = &level.bsp[idx_in_level];
    match class {
        PointClass::Buy1 | PointClass::Sell1 | PointClass::Buy3 | PointClass::Sell3 => match point.center {
            Some(OwnerRef::Center(c)) => Some(ParentFingerprint { center_start: c.start_index, zd: c.zd, zg: c.zg }),
            _ => None,
        },
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
                    Some(OwnerRef::Center(c)) => Some(ParentFingerprint { center_start: c.start_index, zd: c.zd, zg: c.zg }),
                    _ => None,
                }
            })
        }
    }
}

/// 反查坐标：一/二类用「本点自身 `source_index`」查其所属 episode（一类）或「一类锚坐标」
/// （二类）；三类未改判据，不走本函数（仍用 leave_interval.1 精确等值，另计）。
fn owner_query_source_index(level: &LevelState, class: PointClass, idx_in_level: usize) -> Option<usize> {
    let point = &level.bsp[idx_in_level];
    match class {
        PointClass::Buy1 | PointClass::Sell1 => Some(point.source_index),
        PointClass::Buy2 | PointClass::Sell2 => match point.center {
            Some(OwnerRef::Type1Anchor(idx)) => Some(idx),
            _ => None,
        },
        PointClass::Buy3 | PointClass::Sell3 => None,
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
    let episodes_by_level = episodes_by_level(&streams);

    let mut total_bit_instances = 0usize;
    let mut per_level_bsp_points = 0usize;
    let mut fingerprint_unresolved = 0usize;
    let mut owner_query_unresolved = 0usize;
    let mut episode_owned_zero = 0usize;
    let mut episode_owned_one = 0usize;
    let mut episode_owned_many = 0usize;
    let mut by_class_zero: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut by_class_many: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut many_examples: Vec<(String, usize, usize, usize)> = Vec::new(); // (class, level, source_index, owners)

    for (level_idx, level) in classification.levels.iter().enumerate() {
        per_level_bsp_points += level.bsp.len();
        let episodes = episodes_by_level.get(&(level_idx as u32));
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
                if !class.is_first_or_second() {
                    continue; // 三类未改判据，本轮不纳入「归属唯一」检验，见模块头。
                }
                let Some(parent) = resolve_fingerprint(level, class, idx_in_level) else {
                    fingerprint_unresolved += 1;
                    continue;
                };
                let Some(query_source_index) = owner_query_source_index(level, class, idx_in_level) else {
                    owner_query_unresolved += 1;
                    continue;
                };
                let owners = episodes
                    .map(|eps| owning_episodes(eps, class.side(), parent, query_source_index).len())
                    .unwrap_or(0);
                match owners {
                    0 => {
                        episode_owned_zero += 1;
                        *by_class_zero.entry(class.name()).or_default() += 1;
                    }
                    1 => episode_owned_one += 1,
                    n => {
                        episode_owned_many += 1;
                        *by_class_many.entry(class.name()).or_default() += 1;
                        if many_examples.len() < 8 {
                            many_examples.push((class.name().to_string(), level_idx, point.source_index, n));
                        }
                    }
                }
            }
        }
    }

    println!(
        "ISSUE668_TRUTH_V3 bars={} levels={} bsp_points_total={} bit_instances={} \
         fingerprint_unresolved={} owner_query_unresolved={} \
         episode_owned_zero={} episode_owned_one={} episode_owned_many={}",
        bars.len(),
        classification.levels.len(),
        per_level_bsp_points,
        total_bit_instances,
        fingerprint_unresolved,
        owner_query_unresolved,
        episode_owned_zero,
        episode_owned_one,
        episode_owned_many,
    );
    println!("ISSUE668_TRUTH_V3_ZERO_BY_CLASS {by_class_zero:?}");
    println!("ISSUE668_TRUTH_V3_MANY_BY_CLASS {by_class_many:?}");
    for (class, level, source_index, owners) in &many_examples {
        println!(
            "ISSUE668_TRUTH_V3_MANY_EXAMPLE class={class} level={level} source_index={source_index} owners={owners}"
        );
    }
    if episode_owned_many == 0 {
        std::process::ExitCode::SUCCESS
    } else {
        // dispatch 明文：「若仍有撞键，停手上报」——episode 归属不唯一时以非零退出码标出。
        std::process::ExitCode::FAILURE
    }
}
