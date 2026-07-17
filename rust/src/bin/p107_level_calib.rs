//! # `p107_level_calib` —— nest exec/top ↔ classifier lvl 级别标定（任务 #107，只读探针）
//!
//! ## 结构侧前提（代码事实，先于本探针成立）
//! `classify_impl` 不变量：`tower_snapshots.len() == levels.len()` 且同 index 同构
//! （classifier/mod.rs:324）；p92 的 `collect_*_candidates` 以塔索引喂 nest，
//! 故 nest `event.level`（= 证书 exec/top 的索引基）与 `classification.levels` 的
//! lvl **同基同构**。本探针做经验交叉验证：证书 exec × 绑定 BSP 所在 lvl。
//!
//! ## 方法
//! 1. 全侧绑定（p101 逐字同规则，忽略 lvl）→ 复现 #101 的 bound=91 基线；
//! 2. 记录绑定 bar 上同侧 BSP 出现的 lvl 集合 → 交叉表 `P107_CROSS exec×lvl`；
//! 3. 同级绑定（限定 lvl == exec 的同侧 BSP）→ `P107_SAMELVL`：若同基假设成立，
//!    dt 分布应与全侧绑定同数量级；若系统性偏移/无解，则假设证伪。
//!
//! ## 只读纪律
//! 不动判据、不写主路径状态；输出 `P107_*` 报表行到 stdout。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin p107_level_calib -- <P92_DUMP路径>
//! ```

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::backtest::incremental::IncrementalClassifier;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::types::BspBits;
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// 与 runner.rs `bsp_bits_disc` 同口径——6 类 bit 打包（身份判别用）。
fn bsp_bits_disc(b: &BspBits) -> u8 {
    (b.buy1 as u8)
        | (b.buy2 as u8) << 1
        | (b.buy3 as u8) << 2
        | (b.sell1 as u8) << 3
        | (b.sell2 as u8) << 4
        | (b.sell3 as u8) << 5
}

#[derive(Debug)]
struct Cert {
    caliber: String,
    side: String,
    exec: usize,
    top: usize,
    judge_max: i64,
    ids: String,
}

/// 解析 dump 中 `CERT key=value ...` 行（p101 同规则 + exec/top）。
fn parse_certs(path: &str) -> Result<Vec<Cert>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读 {path} 失败: {e}"))?;
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("CERT ") else {
            continue;
        };
        let mut caliber = String::new();
        let mut side = String::new();
        let mut ids = String::new();
        let (mut exec, mut top) = (usize::MAX, usize::MAX);
        let mut judge_max: i64 = -1;
        for token in rest.split_whitespace() {
            let Some((key, value)) = token.split_once('=') else {
                continue;
            };
            match key {
                "caliber" => caliber = value.to_string(),
                "side" => side = value.to_string(),
                "ids" => ids = value.to_string(),
                "exec" => exec = value.parse().unwrap_or(usize::MAX),
                "top" => top = value.parse().unwrap_or(usize::MAX),
                "judge_at" => {
                    judge_max = value
                        .split(',')
                        .filter_map(|c| c.parse::<i64>().ok())
                        .max()
                        .unwrap_or(-1)
                }
                _ => {}
            }
        }
        if !caliber.is_empty() && judge_max >= 0 && exec != usize::MAX && top != usize::MAX {
            out.push(Cert {
                caliber,
                side,
                exec,
                top,
                judge_max,
                ids,
            });
        }
    }
    Ok(out)
}

/// 最近元素带符号距离（bsp_bar - x）；|dt| 相等时 forward 胜（与 p100/p101 逐字同规则）。
fn nearest_dt_tie(sorted: &[i64], x: i64) -> Option<(i64, bool)> {
    if sorted.is_empty() {
        return None;
    }
    let i = sorted.partition_point(|&v| v < x);
    let fwd = (i < sorted.len()).then(|| sorted[i] - x);
    let bwd = (i > 0).then(|| sorted[i - 1] - x);
    match (fwd, bwd) {
        (Some(f), Some(b)) => {
            if b.abs() < f.abs() {
                Some((b, false))
            } else {
                Some((f, f.abs() == b.abs()))
            }
        }
        (Some(f), None) => Some((f, false)),
        (None, Some(b)) => Some((b, false)),
        (None, None) => None,
    }
}

fn bucket(dt: i64) -> &'static str {
    match dt.abs() {
        0..=240 => "strong",
        241..=1440 => "weak",
        _ => "far",
    }
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("用法: p107_level_calib <P92_DUMP路径>");
        return std::process::ExitCode::from(2);
    }
    let certs = match parse_certs(&args[1]) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let config = ThetaConfig::default();
    let dataset = match load_by_symbol("BTC", &config) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("数据加载失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let bars = &dataset.bars;
    let n = bars.len();
    println!("P107_INPUT bars={n} certs_total={}", certs.len());

    // ── BSP 侧：与 p100/p101 同一提取循环，但保留 lvl。 ──
    let mut classifier = IncrementalClassifier::new(bars, &config);
    let mut seen: HashSet<(usize, usize, u8)> = HashSet::new();
    // (bar, lvl, buy, sell)
    let mut events: Vec<(i64, usize, bool, bool)> = Vec::new();
    for i in 0..n {
        let (classification, _tower) = classifier.classify_at(i);
        for (lvl, ls) in classification.levels.iter().enumerate() {
            for p in ls.bsp.iter() {
                if seen.insert((lvl, p.source_index, bsp_bits_disc(&p.bits))) {
                    let b = &p.bits;
                    let buy = b.buy1 || b.buy2 || b.buy3;
                    let sell = b.sell1 || b.sell2 || b.sell3;
                    if buy || sell {
                        events.push((i as i64, lvl, buy, sell));
                    }
                }
            }
        }
        if i % 500_000 == 0 && i > 0 {
            eprintln!("P107_PROGRESS bar={i}/{n} events={}", events.len());
        }
    }
    let max_lvl = events.iter().map(|e| e.1).max().unwrap_or(0);
    // 全侧序列 + 分级序列。
    let mut long_all: Vec<i64> = Vec::new();
    let mut short_all: Vec<i64> = Vec::new();
    let mut long_by_lvl: Vec<Vec<i64>> = vec![Vec::new(); max_lvl + 1];
    let mut short_by_lvl: Vec<Vec<i64>> = vec![Vec::new(); max_lvl + 1];
    // (bar, side) -> lvl 集合（绑定 bar 上同侧 BSP 的级别归属）。
    let mut lvls_at: BTreeMap<(i64, bool), BTreeSet<usize>> = BTreeMap::new();
    for &(bar, lvl, buy, sell) in &events {
        if buy {
            long_all.push(bar);
            long_by_lvl[lvl].push(bar);
            lvls_at.entry((bar, true)).or_default().insert(lvl);
        }
        if sell {
            short_all.push(bar);
            short_by_lvl[lvl].push(bar);
            lvls_at.entry((bar, false)).or_default().insert(lvl);
        }
    }
    for v in [&mut long_all, &mut short_all] {
        v.sort_unstable();
        v.dedup();
    }
    for lvl in 0..=max_lvl {
        for v in [&mut long_by_lvl[lvl], &mut short_by_lvl[lvl]] {
            v.sort_unstable();
            v.dedup();
        }
        println!(
            "P107_BSP_LVL lvl={lvl} buys={} sells={}",
            long_by_lvl[lvl].len(),
            short_by_lvl[lvl].len()
        );
    }
    println!(
        "P107_BSP events_total={} buy_bars={} sell_bars={}",
        events.len(),
        long_all.len(),
        short_all.len()
    );

    // ── 逐证书：全侧绑定复现 + lvl 归属 + 同级绑定。 ──
    let mut bound = 0usize;
    // (exec, lvl) -> n（绑定 bar 上出现同侧 BSP 的 lvl，多 lvl 各记一次）。
    let mut cross: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    // exec -> [同级绑定 strong, weak, far, none]
    let mut same_stats: BTreeMap<usize, [usize; 4]> = BTreeMap::new();
    for (ci, c) in certs.iter().enumerate() {
        let is_long = c.side.contains("Long");
        let all = if is_long { &long_all } else { &short_all };
        let by_lvl = if is_long { &long_by_lvl } else { &short_by_lvl };

        // 1) 全侧绑定（p101 基线复现）。
        let Some((dt, _tie)) = nearest_dt_tie(all, c.judge_max) else {
            println!(
                "P107_TAG idx={ci} caliber={} side={} exec={} top={} judge_max={} bsp_bar=NONE",
                c.caliber, c.side, c.exec, c.top, c.judge_max
            );
            continue;
        };
        if dt.abs() <= 1440 {
            bound += 1;
        }
        let bsp_bar = c.judge_max + dt;
        let lvls: Vec<usize> = lvls_at
            .get(&(bsp_bar, is_long))
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();
        for &lvl in &lvls {
            *cross.entry((c.exec, lvl)).or_insert(0) += 1;
        }

        // 2) 同级绑定（lvl == exec）。
        let same = by_lvl.get(c.exec).map(Vec::as_slice).unwrap_or(&[]);
        let same_dt = nearest_dt_tie(same, c.judge_max).map(|(d, _)| d);
        let slot = same_stats.entry(c.exec).or_insert([0; 4]);
        match same_dt {
            Some(d) if d.abs() <= 240 => slot[0] += 1,
            Some(d) if d.abs() <= 1440 => slot[1] += 1,
            Some(_) => slot[2] += 1,
            None => slot[3] += 1,
        }
        println!(
            "P107_TAG idx={ci} caliber={} side={} exec={} top={} judge_max={} bsp_bar={bsp_bar} dt={dt} b={} lvls={:?} same_dt={:?} ids={}",
            c.caliber,
            c.side,
            c.exec,
            c.top,
            c.judge_max,
            bucket(dt),
            lvls,
            same_dt,
            c.ids
        );
    }
    for ((exec, lvl), cnt) in &cross {
        println!("P107_CROSS exec={exec} lvl={lvl} n={cnt}");
    }
    for (exec, s) in &same_stats {
        println!(
            "P107_SAMELVL exec={exec} strong={} weak={} far={} none={}",
            s[0], s[1], s[2], s[3]
        );
    }
    println!("P107_SUMMARY certs={} bound1440={bound}", certs.len());
    println!("P107_DONE");
    std::process::ExitCode::SUCCESS
}
