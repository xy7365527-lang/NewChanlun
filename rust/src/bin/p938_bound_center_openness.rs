//! #938 探针：中枢震荡短差**绑定中枢**是「开放（frontier）」还是「已封口（sealed）」。
//!
//! ## 本探针测什么（口径声明，先说清楚免得读者外推）
//!
//! 生产接线链（逐跳已逐条打开确认，见报告
//! `.chanlun/review-results/issue938-center-open-or-sealed-20260807.md`）：
//!
//! ```text
//! fill.rs:5099  step_center_oscillation_historical(i, &classification_i, ..)   ← 生产唯一调用点
//!   → fill.rs:918 → step_center_oscillation_impl
//!   → fill.rs:959  chain = &classification_i.levels[lvl].centers
//!   → fill.rs:965  cl_machines[lvl].consume_chain(chain)
//!   → fill.rs:1227 alive_with_index = cl_machines[lvl].alive_center()
//!   → fill.rs:1269 center = alive_with_index.map(|(center, _)| center)
//!   → fill.rs:1283 osc_books[lvl].on_trigger_side_bound(side, trigger, center)
//! ```
//!
//! 待验命题（静态穷举已证，本探针只做实测复核）：
//!
//! > **P**：`alive_center()` 若为 `Some`，其链下标恒 == `chain.len() - 1`（即**链尾**）。
//!
//! 链尾就是 frontier 协议每 bar `pop` 后重扫的那个**开放**中枢
//! （`recursive_tower.rs:709-711`「最后一个中枢在未出现 non-extension 单元前是**开放**的」；
//! `classifier/mod.rs:1376-1415` 逐 bar pop 末窗整窗；`mod.rs:1542-1554`
//! `confirmed_watermark = len - last_window_emitted` = 封口/开放的机检分界）。
//! ⟹ P 成立 ⟹ 绑定中枢**恒开放、恒不在 sealed 前缀内**。
//!
//! **本探针在全部 bar × 全部级别上检 P**，不只在触发 bar 上检——这是**更强**的断言：
//! 触发 bar 是全体 bar 的子集，全集上零违反 ⟹ 触发子集上零违反。故本探针**不重放买卖点触发**
//! （不调 `push_point`、不构造 `CenterOscillationTrigger`），省掉的那部分不影响结论方向。
//! **诚实边界**：因此本探针**不报**「触发次数」这个绝对量，只报占比性质的 P；要触发绝对量须另跑
//! 完整 fill 回路。
//!
//! ## 第二读数：绑定中枢的核心**实际漂移率**
//!
//! P 只说「绑定的是开放中枢」。ADR 0019 缺口一问的是「开放 ⟹ 核心会不会真的改写」。故同时逐 bar
//! 比对本级链尾 `CenterId` 与**上一 bar** 的链尾 `CenterId`，三分：
//!
//! | 分类 | 含义 |
//! |---|---|
//! | `same` | 三元组逐值相同——本 bar 未改写 |
//! | `core_drift` | `start_index` **相同**、`(zd, zg)` **变了**——★中枢还活着、核心被重写（= 挂单价会漂） |
//! | `seed_moved` | `start_index` 变了——换了个中枢（推进/重建），不是「同一张单漂价」 |
//!
//! ## 只读声明
//!
//! 本 bin 不改任何既有 `.rs`，不写盘，不参与任何判定；只经既有 `pub` API 读数。
//!
//! 用法：
//! ```text
//! cargo run --release --bin p938_bound_center_openness -- <btc_1m_full.json> [MAX_BARS]
//! ```

use newchan_rust::theta_v0::backtest::incremental::IncrementalClassifier;
use newchan_rust::theta_v0::classifier::center_lifecycle::{CenterEventMachine, CenterId};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::types::{quantize, Bar, Timestamp};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct BarsJson {
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    volumes: Vec<f64>,
    dates: Vec<String>,
}

/// 逐级累计读数。
#[derive(Debug, Default, Clone)]
struct LevelStat {
    /// 该级链非空、`alive_center()` 为 `Some` 的 bar 数（= P 的分母）。
    alive_bars: u64,
    /// 其中 `alive` 下标 == `chain.len()-1`（链尾 = 开放 frontier）的 bar 数（= P 的分子）。
    alive_is_tail: u64,
    /// ★P 的反例：`alive` 下标 != 链尾。逐条打印前若干个。
    alive_not_tail: u64,
    /// 链尾 `CenterId` 与上一 bar 相同。
    tail_same: u64,
    /// ★核心漂移：`start_index` 同、`(zd,zg)` 变。
    tail_core_drift: u64,
    /// 换中枢：`start_index` 变。
    tail_seed_moved: u64,
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Err(format!(
            "用法: {} <btc_1m_full.json> [MAX_BARS]",
            args.first().map(String::as_str).unwrap_or("p938")
        ));
    }
    let max_bars: usize = args
        .get(2)
        .and_then(|v| v.parse().ok())
        .unwrap_or(usize::MAX);

    let config = ThetaConfig::default();
    let (bars, first_date, last_date) = load_bars(Path::new(&args[1]), config.tick.tick_size)?;
    let n = bars.len().min(max_bars);
    if n == 0 {
        return Err("空数据".to_string());
    }
    eprintln!(
        "P938 载入 {} bar（用前 {n}），{first_date} .. {last_date}",
        bars.len()
    );

    let bars = &bars[..n];
    let mut incr = IncrementalClassifier::new(bars, &config);
    // 与 fill.rs:953-957 同款：级数按 classification 逐 bar 对齐补齐。
    let mut machines: Vec<CenterEventMachine> = Vec::new();
    let mut stats: Vec<LevelStat> = Vec::new();
    let mut prev_tail: Vec<Option<CenterId>> = Vec::new();
    let mut violations_shown = 0u32;

    for i in 0..n {
        let (classification, _tower) = incr.classify_at(i);
        let n_levels = classification.levels.len();
        while machines.len() < n_levels {
            machines.push(CenterEventMachine::new(machines.len() as u32));
            stats.push(LevelStat::default());
            prev_tail.push(None);
        }
        for lvl in 0..n_levels {
            // ★与 fill.rs:959 + :965 逐字同源：喂的是同一张 `levels[lvl].centers`。
            let chain = &classification.levels[lvl].centers;
            let _ = machines[lvl].consume_chain(chain);

            // ── 读数一：P（alive 是不是链尾 = 开放 frontier）。 ──
            if let Some((_center, idx)) = machines[lvl].alive_center() {
                stats[lvl].alive_bars += 1;
                if !chain.is_empty() && idx == chain.len() - 1 {
                    stats[lvl].alive_is_tail += 1;
                } else {
                    stats[lvl].alive_not_tail += 1;
                    if violations_shown < 20 {
                        violations_shown += 1;
                        eprintln!(
                            "P938_VIOLATION bar={i} level={lvl} alive_idx={idx} chain_len={}",
                            chain.len()
                        );
                    }
                }
            }

            // ── 读数二：链尾 CenterId 的逐 bar 迁移三分。 ──
            let tail = chain.last().map(CenterId::of);
            if let (Some(prev), Some(cur)) = (prev_tail[lvl], tail) {
                if prev == cur {
                    stats[lvl].tail_same += 1;
                } else if prev.start_index == cur.start_index {
                    stats[lvl].tail_core_drift += 1;
                } else {
                    stats[lvl].tail_seed_moved += 1;
                }
            }
            prev_tail[lvl] = tail;
        }
        if i % 100_000 == 0 && i > 0 {
            eprintln!("  .. bar {i}/{n}");
        }
    }

    // ── 汇报 ──
    println!("# P938 绑定中枢开放性实测（BTC 1m，前 {n} bar，{first_date} .. {last_date}）");
    println!();
    println!("## 读数一：P =「alive_center() 恒为链尾（= 开放 frontier）」");
    println!();
    println!("| level | alive_bars | is_tail | not_tail | is_tail 占比 |");
    println!("|---|---|---|---|---|");
    let mut tot_alive = 0u64;
    let mut tot_tail = 0u64;
    let mut tot_not = 0u64;
    for (lvl, s) in stats.iter().enumerate() {
        if s.alive_bars == 0 {
            continue;
        }
        tot_alive += s.alive_bars;
        tot_tail += s.alive_is_tail;
        tot_not += s.alive_not_tail;
        println!(
            "| L{lvl} | {} | {} | {} | {:.6}% |",
            s.alive_bars,
            s.alive_is_tail,
            s.alive_not_tail,
            100.0 * s.alive_is_tail as f64 / s.alive_bars as f64
        );
    }
    println!(
        "| **合计** | {tot_alive} | {tot_tail} | {tot_not} | {:.6}% |",
        100.0 * tot_tail as f64 / tot_alive.max(1) as f64
    );
    println!();
    println!("## 读数二：链尾 CenterId 逐 bar 迁移（core_drift = 同 start_index 但核心被改写）");
    println!();
    println!("| level | same | core_drift | seed_moved | core_drift 占变化 |");
    println!("|---|---|---|---|---|");
    let (mut ts, mut tc, mut tm) = (0u64, 0u64, 0u64);
    for (lvl, s) in stats.iter().enumerate() {
        if s.tail_same + s.tail_core_drift + s.tail_seed_moved == 0 {
            continue;
        }
        ts += s.tail_same;
        tc += s.tail_core_drift;
        tm += s.tail_seed_moved;
        let changed = s.tail_core_drift + s.tail_seed_moved;
        println!(
            "| L{lvl} | {} | {} | {} | {:.2}% |",
            s.tail_same,
            s.tail_core_drift,
            s.tail_seed_moved,
            100.0 * s.tail_core_drift as f64 / changed.max(1) as f64
        );
    }
    println!(
        "| **合计** | {ts} | {tc} | {tm} | {:.2}% |",
        100.0 * tc as f64 / (tc + tm).max(1) as f64
    );
    Ok(())
}

/// 与 `p122_replay_profile.rs:806-853` 同款载入（逐字抄，不自创量化口径）。
fn load_bars(path: &Path, tick_size: f64) -> Result<(Vec<Bar>, String, String), String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?;
    let raw: BarsJson = serde_json::from_str(&text)
        .map_err(|error| format!("{} JSON 解析失败: {error}", path.display()))?;
    let n = raw.closes.len();
    for (name, len) in [
        ("opens", raw.opens.len()),
        ("highs", raw.highs.len()),
        ("lows", raw.lows.len()),
        ("volumes", raw.volumes.len()),
        ("dates", raw.dates.len()),
    ] {
        if len != n {
            return Err(format!("列长度不一致: {name}={len}, closes={n}"));
        }
    }
    let bars = (0..n)
        .map(|index| {
            let open = raw.opens[index];
            let high = raw.highs[index];
            let low = raw.lows[index];
            let close = raw.closes[index];
            let volume = raw.volumes[index];
            Bar {
                source_index: index,
                timestamp: date_to_timestamp(&raw.dates[index]),
                open: quantize(open, tick_size),
                high: quantize(high, tick_size),
                low: quantize(low, tick_size),
                close: quantize(close, tick_size),
                volume: volume as i64,
                untradable: high < open.max(close).max(low)
                    || low > open.min(close).min(high)
                    || open <= 0.0
                    || high <= 0.0
                    || low <= 0.0
                    || close <= 0.0
                    || volume <= 0.0,
            }
        })
        .collect();
    Ok((
        bars,
        raw.dates.first().cloned().unwrap_or_default(),
        raw.dates.last().cloned().unwrap_or_default(),
    ))
}

fn date_to_timestamp(date: &str) -> Timestamp {
    let mut digits = String::with_capacity(14);
    for ch in date.chars().take_while(|value| *value != '+') {
        if ch.is_ascii_digit() {
            digits.push(ch);
            if digits.len() == 14 {
                break;
            }
        }
    }
    digits.parse().unwrap_or(0)
}
