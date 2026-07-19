//! # `p100_cert_bsp_recon` —— nest 证书 × 现有买卖点全量对账（任务 #100，只读探针）
//!
//! ## 两侧口径
//! - **CERT 侧**：读 `P92_DUMP` 明细行（`p92_nest_replay_postruling` 产物；#105 修复后
//!   口径 A=41 / B=25）。证书时钟取 `judge_at` 各身份判定钟的 max（全体身份判定完成时点）。
//! - **BSP 侧**：生产 `IncrementalClassifier::classify_at(i)` + 与 runner
//!   `newly_confirmed_step` 逐字相同的 seen-set append-only diff（644 meta-rule：
//!   探针必走生产路径，无影子分叉；与 `pure_bsp_timing` 同一提取循环）。
//!
//! ## 对账主键
//! 时间（cert judge_max ↔ BSP 确认 bar）× 方向（cert side=Long↔buy bits / Short↔sell bits）。
//! 层级不做硬映射（nest exec/top 与 classifier lvl 语义不同构），只报告最近命中的 lvl 供归因。
//!
//! ## 只读纪律
//! 不动判据、不写主路径状态；输出 `P100_*` 报表行到 stdout。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin p100_cert_bsp_recon -- <P92_DUMP路径>
//! ```

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::backtest::incremental::IncrementalClassifier;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::types::BspBits;
use std::collections::HashSet;

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
    as_of: i64,
    judge_max: i64,
    bucket: String,
    kinds: String,
}

/// 解析 dump 中 `CERT key=value ...` 行（值内无空格；judge_at 为逗号钟表）。
fn parse_certs(path: &str) -> Result<Vec<Cert>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读 {path} 失败: {e}"))?;
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("CERT ") else {
            continue;
        };
        let mut caliber = String::new();
        let mut side = String::new();
        let mut bucket = String::new();
        let mut kinds = String::new();
        let mut as_of: i64 = -1;
        let mut judge_max: i64 = -1;
        for token in rest.split_whitespace() {
            let Some((key, value)) = token.split_once('=') else {
                continue;
            };
            match key {
                "caliber" => caliber = value.to_string(),
                "side" => side = value.to_string(),
                "bucket" => bucket = value.to_string(),
                "kinds" => kinds = value.to_string(),
                "as_of" => as_of = value.parse().unwrap_or(-1),
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
        if !caliber.is_empty() && judge_max >= 0 {
            out.push(Cert {
                caliber,
                side,
                as_of,
                judge_max,
                bucket,
                kinds,
            });
        }
    }
    Ok(out)
}

/// 在升序 bar 序列中找最近元素，返回带符号距离（bsp_bar - x）。
fn nearest_dt(sorted: &[i64], x: i64) -> Option<i64> {
    if sorted.is_empty() {
        return None;
    }
    let i = sorted.partition_point(|&v| v < x);
    let mut best: Option<i64> = None;
    if i < sorted.len() {
        best = Some(sorted[i] - x);
    }
    if i > 0 {
        let d = sorted[i - 1] - x;
        if best.map_or(true, |b| d.abs() < b.abs()) {
            best = Some(d);
        }
    }
    best
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("用法: p100_cert_bsp_recon <P92_DUMP路径>");
        return std::process::ExitCode::from(2);
    }
    let certs = match parse_certs(&args[1]) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let n_a = certs.iter().filter(|c| c.caliber == "A").count();
    let n_b = certs.iter().filter(|c| c.caliber == "B").count();

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
    println!(
        "P100_INPUT bars={n} certs_total={} A={n_a} B={n_b}",
        certs.len()
    );

    // ── BSP 侧：生产因果塔 + runner 同语义 seen-set diff。 ──
    let mut classifier = IncrementalClassifier::new(bars, &config);
    let mut seen: HashSet<(usize, usize, u8)> = HashSet::new();
    // (确认 bar, lvl, buy?, sell?)
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
            eprintln!("P100_PROGRESS bar={i}/{n} events={}", events.len());
        }
    }
    let long_bars: Vec<i64> = events.iter().filter(|e| e.2).map(|e| e.0).collect();
    let short_bars: Vec<i64> = events.iter().filter(|e| e.3).map(|e| e.0).collect();
    let all_bars: Vec<i64> = events.iter().map(|e| e.0).collect();
    let max_lvl = events.iter().map(|e| e.1).max().unwrap_or(0);
    println!(
        "P100_BSP events_total={} buys={} sells={} max_lvl={max_lvl}",
        events.len(),
        long_bars.len(),
        short_bars.len()
    );
    for lvl in 0..=max_lvl {
        let (b, s) = events
            .iter()
            .filter(|e| e.1 == lvl)
            .fold((0usize, 0usize), |(b, s), e| {
                (b + e.2 as usize, s + e.3 as usize)
            });
        println!("P100_BSP_LVL lvl={lvl} buys={b} sells={s}");
    }

    // ── 正向对账：每张证书 → 最近同向/任向 BSP。 ──
    const WINDOWS: [i64; 5] = [0, 5, 30, 240, 1440];
    for caliber in ["A", "B"] {
        let mut dts: Vec<i64> = Vec::new();
        let mut hits = [0usize; WINDOWS.len()];
        for c in certs.iter().filter(|c| c.caliber == caliber) {
            let same = if c.side.contains("Long") {
                &long_bars
            } else {
                &short_bars
            };
            let dt_same = nearest_dt(same, c.judge_max);
            let dt_any = nearest_dt(&all_bars, c.judge_max);
            // 最近同向命中的 lvl（同 bar 距离处任取一个，报告用）。
            let near_lvl = dt_same.and_then(|d| {
                let target = c.judge_max + d;
                events
                    .iter()
                    .find(|e| {
                        e.0 == target && (if c.side.contains("Long") { e.2 } else { e.3 })
                    })
                    .map(|e| e.1)
            });
            println!(
                "P100_CERT caliber={} side={} as_of={} judge_max={} bucket={} kinds={} dt_same={:?} dt_any={:?} near_lvl={:?}",
                c.caliber, c.side, c.as_of, c.judge_max, c.bucket, c.kinds, dt_same, dt_any, near_lvl
            );
            if let Some(d) = dt_same {
                dts.push(d.abs());
                for (k, w) in WINDOWS.iter().enumerate() {
                    if d.abs() <= *w {
                        hits[k] += 1;
                    }
                }
            }
        }
        dts.sort_unstable();
        let median = if dts.is_empty() { -1 } else { dts[dts.len() / 2] };
        println!(
            "P100_SUMMARY caliber={caliber} n={} w0={} w5={} w30={} w240={} w1440={} median_abs_dt={median}",
            dts.len(),
            hits[0],
            hits[1],
            hits[2],
            hits[3],
            hits[4]
        );
    }

    // ── 反向覆盖：BSP 事件中，±W 内存在同向证书的比例（证书极稀疏，预期覆盖极低）。 ──
    let mut cert_long: Vec<i64> = certs
        .iter()
        .filter(|c| c.side.contains("Long"))
        .map(|c| c.judge_max)
        .collect();
    let mut cert_short: Vec<i64> = certs
        .iter()
        .filter(|c| !c.side.contains("Long"))
        .map(|c| c.judge_max)
        .collect();
    cert_long.sort_unstable();
    cert_short.sort_unstable();
    for w in [240i64, 1440] {
        let covered = events
            .iter()
            .filter(|e| {
                let cs = if e.2 { &cert_long } else { &cert_short };
                nearest_dt(cs, e.0).map_or(false, |d| d.abs() <= w)
            })
            .count();
        println!(
            "P100_REVERSE w={w} bsp_covered={covered}/{} rate={:.6}",
            events.len(),
            covered as f64 / events.len().max(1) as f64
        );
    }
    println!("P100_DONE");
    std::process::ExitCode::SUCCESS
}
