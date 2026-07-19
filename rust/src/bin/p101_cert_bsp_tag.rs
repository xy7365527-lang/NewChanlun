//! # `p101_cert_bsp_tag` —— 证书 → 就近 BSP 打标物化（任务 #101，只读探针）
//!
//! ## 依据
//! #100 对账结论：两口径证书 1440 bar 内 100% 有同侧 BSP，过半同 bar；
//! 反向覆盖 0.17%~0.97% → 证书不可作独立入场源，只能给 BSP 打标（加权/过滤）。
//!
//! ## 绑定规则（cert-bsp-binding-ruling DRAFT 条款的可执行镜像）
//! - 钟：cert 取 `judge_at` 逗号钟表的 max（judge_max）；BSP 取确认 bar。禁 created_at。
//! - 匹配：最近**同侧** BSP（Long↔buy bits / Short↔sell bits）；|dt| 相等时 forward
//!   （bsp_bar ≥ judge_max）胜出——与 p100 `nearest_dt` 逐字同规则。
//! - 强度：|dt| ≤ 240 → strong；240 < |dt| ≤ 1440 → weak；>1440 → unbound（当前应为 0）。
//!
//! ## 校验
//! 1. 完备：每张证书必有绑定（unbound=0）。
//! 2. 唯一/确定：tie（前后等距）逐张标记 tie=1 并按 forward 规则消歧；重跑同输入结果逐字节稳定。
//! 3. 重叠：同一 BSP 收到多张证书打标时报 P101_MULTI（允许，但要可见）。
//!
//! ## 只读纪律
//! 不动判据、不写主路径状态；输出 `P101_*` 报表行到 stdout。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin p101_cert_bsp_tag -- <P92_DUMP路径>
//! ```

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::backtest::incremental::IncrementalClassifier;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::types::BspBits;
use std::collections::{BTreeMap, HashSet};

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
    judge_max: i64,
    bucket: String,
    kinds: String,
    ids: String,
}

/// 解析 dump 中 `CERT key=value ...` 行（与 p100 同规则，另留 ids 作证书主键）。
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
        let mut ids = String::new();
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
                "ids" => ids = value.to_string(),
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
                judge_max,
                bucket,
                kinds,
                ids,
            });
        }
    }
    Ok(out)
}

/// 最近元素带符号距离（bsp_bar - x）；|dt| 相等时 forward 胜（与 p100 逐字同规则）。
/// 返回 (dt, tie)：tie=前后等距（消歧后仍确定）。
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

fn strength(dt: i64) -> &'static str {
    match dt.abs() {
        0..=240 => "strong",
        241..=1440 => "weak",
        _ => "unbound",
    }
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("用法: p101_cert_bsp_tag <P92_DUMP路径>");
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
    println!("P101_INPUT bars={n} certs_total={}", certs.len());

    // ── BSP 侧：生产因果塔 + runner 同语义 seen-set diff（与 p100 同一提取循环）。 ──
    let mut classifier = IncrementalClassifier::new(bars, &config);
    let mut seen: HashSet<(usize, usize, u8)> = HashSet::new();
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
            eprintln!("P101_PROGRESS bar={i}/{n} events={}", events.len());
        }
    }
    let long_bars: Vec<i64> = events.iter().filter(|e| e.2).map(|e| e.0).collect();
    let short_bars: Vec<i64> = events.iter().filter(|e| e.3).map(|e| e.0).collect();
    println!(
        "P101_BSP events_total={} buys={} sells={}",
        events.len(),
        long_bars.len(),
        short_bars.len()
    );

    // ── 逐证书绑定物化。 ──
    let mut unbound = 0usize;
    let mut ties = 0usize;
    let mut per_bsp: BTreeMap<(i64, bool), Vec<usize>> = BTreeMap::new();
    let mut counts: BTreeMap<(String, String), (usize, usize)> = BTreeMap::new(); // (caliber,strength)->(n,ties)
    for (ci, c) in certs.iter().enumerate() {
        let is_long = c.side.contains("Long");
        let same = if is_long { &long_bars } else { &short_bars };
        let Some((dt, tie)) = nearest_dt_tie(same, c.judge_max) else {
            println!(
                "P101_TAG idx={ci} caliber={} side={} judge_max={} bucket={} kinds={} ids={} bsp_bar=NONE",
                c.caliber, c.side, c.judge_max, c.bucket, c.kinds, c.ids
            );
            unbound += 1;
            continue;
        };
        let s = strength(dt);
        if s == "unbound" {
            unbound += 1;
        }
        if tie {
            ties += 1;
        }
        let bsp_bar = c.judge_max + dt;
        per_bsp.entry((bsp_bar, is_long)).or_default().push(ci);
        let e = counts
            .entry((c.caliber.clone(), s.to_string()))
            .or_insert((0, 0));
        e.0 += 1;
        e.1 += tie as usize;
        println!(
            "P101_TAG idx={ci} caliber={} side={} judge_max={} bsp_bar={bsp_bar} dt={dt} strength={s} tie={} bucket={} kinds={} ids={}",
            c.caliber, c.side, c.judge_max, tie as u8, c.bucket, c.kinds, c.ids
        );
    }
    for ((caliber, s), (cnt, tie_cnt)) in &counts {
        println!("P101_COUNT caliber={caliber} strength={s} n={cnt} ties={tie_cnt}");
    }
    for ((bar, is_long), list) in per_bsp.iter().filter(|(_, v)| v.len() > 1) {
        println!(
            "P101_MULTI bsp_bar={bar} side={} certs={:?}",
            if *is_long { "Long" } else { "Short" },
            list
        );
    }
    let multi = per_bsp.values().filter(|v| v.len() > 1).count();
    println!(
        "P101_SUMMARY certs={} bound={} unbound={unbound} ties={ties} distinct_bsp={} multi_bsp={multi}",
        certs.len(),
        certs.len() - unbound,
        per_bsp.len()
    );
    println!("P101_DONE");
    std::process::ExitCode::SUCCESS
}
