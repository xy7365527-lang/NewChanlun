//! #938 探针二：链尾 `CenterId` 变化的**谱系构成**——「迁移」还是「重建」。
//!
//! ## 前置（探针一 `p938_bound_center_openness.rs`，commit 86e33e6f3b）
//!
//! BTC 前 100 万 bar 上链尾 `CenterId` 逐帧迁移三分：身份不变 4,675,066 / 核心漂移 258 /
//! 起点移动 2,423。#938 在裁「限价单身份键用裸 `CenterId.start_index` 还是用
//! `CenterOscillationBook::resolve()` 的归一锚」，两键的差别只在**起点移动**那一族上显形。
//!
//! ## 本探针测什么
//!
//! 把每一次链尾 `CenterId` 变化，**按生产判据**逐条判成下面互斥的一类：
//!
//! | 类 | 判据（生产同源） |
//! |---|---|
//! | `advanced` | `CenterEventMachine::consume_chain` 返回 `Advanced`——链尾追加新中枢，旧尾**仍在链上**、没死。不是重基。 |
//! | `rebased_survived` | 返回 `Rebased`，但旧尾身份仍出现在新链某处 ⟹ `on_chain_rebase_lineage` 的「跟随迁移」路径（`center_oscillation_trade.rs:815-817`），无需证书。 |
//! | `rebased_migrate` | `Rebased` + 旧尾不在新链 + 谱系簿给 `Continued(new)` 且 `new` 在新链上 ⟹ 生产会**迁移身份锚**（`:831-835` → `:857-865`）。 |
//! | `rebased_no_cert` / `_ambiguous` / `_bar_mismatch` / `_target_absent` | `Rebased` + fail-closed 四桶（`:824-854`）⟹ 生产**保持 RebaseVanished 核销**，即判「重建」。 |
//!
//! 判据源逐条已打开确认，见报告
//! `.chanlun/review-results/issue938-seed-moved-composition-20260807.md`。
//!
//! ## bar 坐标双读（生产口径 vs 证书口径）
//!
//! 生产在 `fill.rs:1018` 用**循环 bar `i`** 建 `lineage_book::view_for(bar, lvl)`；
//! 而证书产出点 `classifier/mod.rs:884` 的 `txn_bar` = `l0.merged_bars.last().source_index`。
//! 两者在「本 bar 被 inclusion 吸收进上一根 merged bar」时**不相等** ⟹ 生产查簿会吃
//! `BarMismatch`。本探针**两种 bar 都查一遍**并分列，把这条差异测出来而不是假设它不存在。
//!
//! ## 只读声明
//!
//! 本 bin 不改任何既有 `.rs`，不写盘，不参与任何判定；只经既有 `pub` API 读数。
//!
//! 用法：
//! ```text
//! cargo run --release --features backtest_bin --bin p938_seed_moved_composition -- \
//!     ../analysis/data_cache/btc_1m_full.json 1000000
//! ```

use newchan_rust::theta_v0::backtest::incremental::IncrementalClassifier;
use newchan_rust::theta_v0::classifier::center_lifecycle::{
    CenterEventMachine, CenterId, ChainConsumed,
};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::lineage_book;
use newchan_rust::theta_v0::types::{quantize, Bar, Timestamp};
use serde::Deserialize;
use std::collections::BTreeSet;
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

/// 一次链尾身份变化的归类（互斥穷尽）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// 链只是往前长了一节，旧尾没死。
    Advanced,
    /// 重基，但旧尾身份仍在新链上（跟随迁移，无需证书）。
    RebasedSurvived,
    /// 重基 + 1→1 构造证书 ⟹ 迁移身份锚。
    RebasedMigrate,
    /// 重基 + 证书说延续到 X，但 X 不在新链上。
    RebasedTargetAbsent,
    /// 重基 + 本 bar 本级没有该旧身份的可过继边。
    RebasedNoCert,
    /// 重基 + 同一旧身份多个候选后继。
    RebasedAmbiguous,
    /// 重基 + 簿记 bar 与请求 bar 不符。
    RebasedBarMismatch,
    /// 重基，但上一 bar 该级链为空 / 本 bar 链为空（无旧尾可判）。
    RebasedNoOldTail,
}

impl Verdict {
    fn label(self) -> &'static str {
        match self {
            Verdict::Advanced => "advanced",
            Verdict::RebasedSurvived => "rebased_survived",
            Verdict::RebasedMigrate => "rebased_migrate",
            Verdict::RebasedTargetAbsent => "rebased_target_absent",
            Verdict::RebasedNoCert => "rebased_no_cert",
            Verdict::RebasedAmbiguous => "rebased_ambiguous",
            Verdict::RebasedBarMismatch => "rebased_bar_mismatch",
            Verdict::RebasedNoOldTail => "rebased_no_old_tail",
        }
    }
    /// 生产会不会保住身份（= 锚键不撤单）。`None` = 不适用（advanced：旧尾还活着）。
    fn keeps_identity(self) -> Option<bool> {
        match self {
            Verdict::Advanced => None,
            Verdict::RebasedSurvived | Verdict::RebasedMigrate => Some(true),
            _ => Some(false),
        }
    }
    const ALL: [Verdict; 8] = [
        Verdict::Advanced,
        Verdict::RebasedSurvived,
        Verdict::RebasedMigrate,
        Verdict::RebasedTargetAbsent,
        Verdict::RebasedNoCert,
        Verdict::RebasedAmbiguous,
        Verdict::RebasedBarMismatch,
        Verdict::RebasedNoOldTail,
    ];
    fn index(self) -> usize {
        Verdict::ALL.iter().position(|v| *v == self).unwrap()
    }
}

/// 逐级累计读数。
#[derive(Debug, Default, Clone)]
struct LevelStat {
    tail_same: u64,
    /// `start_index` 变（探针一的 `seed_moved`）× 八类归类（生产 bar 口径）。
    seed_moved: [u64; 8],
    /// `start_index` 同、`(zd,zg)` 变（探针一的 `core_drift`）× 八类。
    core_drift: [u64; 8],
    /// 同上，但查簿用 `txn_bar`（证书口径）——只为量化 bar 坐标差异。
    seed_moved_txnbar: [u64; 8],
    core_drift_txnbar: [u64; 8],
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Err(format!(
            "用法: {} <btc_1m_full.json> [MAX_BARS]",
            args.first().map(String::as_str).unwrap_or("p938b")
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
        "P938B 载入 {} bar（用前 {n}），{first_date} .. {last_date}；lineage 建簿={} 读法={}",
        bars.len(),
        lineage_book::consumer_enabled(),
        if lineage_book::wide_reading() {
            "wide"
        } else {
            "strict"
        }
    );

    let bars = &bars[..n];
    let mut incr = IncrementalClassifier::new(bars, &config);
    let mut machines: Vec<CenterEventMachine> = Vec::new();
    let mut stats: Vec<LevelStat> = Vec::new();
    let mut prev_tail: Vec<Option<CenterId>> = Vec::new();
    // bar 坐标差异计数：txn_bar != i 的 bar 数。
    let mut bar_coord_mismatch = 0u64;
    let mut samples: Vec<String> = Vec::new();

    for i in 0..n {
        let (l0, classification, _tower) = incr.classify_at_with_l0(i);
        let txn_bar = l0.merged_bars.last().map_or(0, |b| b.source_index);
        if txn_bar != i {
            bar_coord_mismatch += 1;
        }
        let n_levels = classification.levels.len();
        while machines.len() < n_levels {
            machines.push(CenterEventMachine::new(machines.len() as u32));
            stats.push(LevelStat::default());
            prev_tail.push(None);
        }
        for lvl in 0..n_levels {
            // ★与 fill.rs:959 + :965 逐字同源：喂同一张 `levels[lvl].centers`，同一台事件机。
            let chain = &classification.levels[lvl].centers;
            let consumed = machines[lvl].consume_chain(chain);

            let tail = chain.last().map(CenterId::of);
            if let (Some(prev), Some(cur)) = (prev_tail[lvl], tail) {
                if prev == cur {
                    stats[lvl].tail_same += 1;
                } else {
                    let chain_ids: BTreeSet<CenterId> = chain.iter().map(CenterId::of).collect();
                    let v = judge(&consumed, prev, &chain_ids, i, lvl);
                    let v_txn = judge(&consumed, prev, &chain_ids, txn_bar, lvl);
                    let seed = prev.start_index != cur.start_index;
                    if seed {
                        stats[lvl].seed_moved[v.index()] += 1;
                        stats[lvl].seed_moved_txnbar[v_txn.index()] += 1;
                    } else {
                        stats[lvl].core_drift[v.index()] += 1;
                        stats[lvl].core_drift_txnbar[v_txn.index()] += 1;
                    }
                    if samples.len() < 24 && !matches!(v, Verdict::Advanced) {
                        samples.push(format!(
                            "bar={i} txn_bar={txn_bar} L{lvl} kind={} prod={} txn={} \
                             old=({},{},{}) new=({},{},{})",
                            if seed { "seed_moved" } else { "core_drift" },
                            v.label(),
                            v_txn.label(),
                            prev.start_index,
                            prev.zd,
                            prev.zg,
                            cur.start_index,
                            cur.zd,
                            cur.zg,
                        ));
                    }
                }
            }
            prev_tail[lvl] = tail;
        }
        if i % 100_000 == 0 && i > 0 {
            eprintln!("  .. bar {i}/{n}");
        }
    }

    report(
        &stats,
        n,
        &first_date,
        &last_date,
        bar_coord_mismatch,
        &samples,
    );
    Ok(())
}

/// 按生产判据（`center_oscillation_trade.rs:813-855` 逐条同源）判一次链尾变化。
fn judge(
    consumed: &ChainConsumed,
    old_tail: CenterId,
    chain_ids: &BTreeSet<CenterId>,
    bar: usize,
    lvl: usize,
) -> Verdict {
    match consumed {
        // 链追加：旧尾没被改写，`consume_chain_impl:566-593` 只产 Born/Superseded。
        ChainConsumed::Advanced { .. } | ChainConsumed::Adopted { .. } => Verdict::Advanced,
        ChainConsumed::Rebased { .. } => {
            // ① 旧身份仍在新链上 ⟹ 跟随迁移（`center_oscillation_trade.rs:815-817`）。
            if chain_ids.contains(&old_tail) {
                return Verdict::RebasedSurvived;
            }
            // ② 查谱系簿（`:824`）。
            match lineage_book::lookup(bar, lvl, old_tail) {
                lineage_book::Verdict::Continued(w) => {
                    if chain_ids.contains(&w.new) {
                        Verdict::RebasedMigrate
                    } else {
                        Verdict::RebasedTargetAbsent
                    }
                }
                lineage_book::Verdict::NoCert => Verdict::RebasedNoCert,
                lineage_book::Verdict::Ambiguous => Verdict::RebasedAmbiguous,
                lineage_book::Verdict::BarMismatch => Verdict::RebasedBarMismatch,
            }
        }
    }
}

fn row(counts: &[u64; 8]) -> String {
    Verdict::ALL
        .iter()
        .map(|v| counts[v.index()].to_string())
        .collect::<Vec<_>>()
        .join(" | ")
}

fn total(counts: &[u64; 8]) -> u64 {
    counts.iter().sum()
}

fn report(
    stats: &[LevelStat],
    n: usize,
    first: &str,
    last: &str,
    bar_coord_mismatch: u64,
    samples: &[String],
) {
    let header = Verdict::ALL
        .iter()
        .map(|v| v.label())
        .collect::<Vec<_>>()
        .join(" | ");
    println!("# P938B 链尾身份变化的谱系构成（BTC 1m，前 {n} bar，{first} .. {last}）");
    println!();
    println!(
        "bar 坐标差异（`txn_bar != i` 的 bar 数）：**{bar_coord_mismatch}** / {n} = {:.2}%",
        100.0 * bar_coord_mismatch as f64 / n as f64
    );
    println!();

    for (name, pick) in [
        ("seed_moved（start_index 变）· 生产 bar 口径", 0usize),
        ("core_drift（start_index 同、核心变）· 生产 bar 口径", 1),
        ("seed_moved · 证书 txn_bar 口径（反事实对照）", 2),
        ("core_drift · 证书 txn_bar 口径（反事实对照）", 3),
    ] {
        println!("## {name}");
        println!();
        println!("| level | {header} | 合计 |");
        println!("|---|{}|---|", "---|".repeat(Verdict::ALL.len()));
        let mut agg = [0u64; 8];
        for (lvl, s) in stats.iter().enumerate() {
            let c = match pick {
                0 => &s.seed_moved,
                1 => &s.core_drift,
                2 => &s.seed_moved_txnbar,
                _ => &s.core_drift_txnbar,
            };
            if total(c) == 0 {
                continue;
            }
            for k in 0..8 {
                agg[k] += c[k];
            }
            println!("| L{lvl} | {} | {} |", row(c), total(c));
        }
        println!("| **合计** | {} | {} |", row(&agg), total(&agg));
        println!();
        // 保身份 vs 判重建（advanced 单列，不混进来）。
        let keep: u64 = Verdict::ALL
            .iter()
            .filter(|v| v.keeps_identity() == Some(true))
            .map(|v| agg[v.index()])
            .sum();
        let rebuild: u64 = Verdict::ALL
            .iter()
            .filter(|v| v.keeps_identity() == Some(false))
            .map(|v| agg[v.index()])
            .sum();
        let adv = agg[Verdict::Advanced.index()];
        println!(
            "**归结**：链推进 `advanced`={adv}；重基里保身份={keep}，判重建={rebuild}，\
             保身份占重基 {:.2}%",
            100.0 * keep as f64 / (keep + rebuild).max(1) as f64
        );
        println!();
    }

    println!("## 逐级 tail_same（分母参考）");
    println!();
    println!("| level | tail_same |");
    println!("|---|---|");
    for (lvl, s) in stats.iter().enumerate() {
        if s.tail_same == 0 {
            continue;
        }
        println!("| L{lvl} | {} |", s.tail_same);
    }
    println!();
    println!("## 前 {} 条非 advanced 样本", samples.len());
    println!();
    println!("```text");
    for s in samples {
        println!("{s}");
    }
    println!("```");
}

/// 与 `p938_bound_center_openness.rs:227-274` 同款载入（逐字抄，不自创量化口径）。
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
