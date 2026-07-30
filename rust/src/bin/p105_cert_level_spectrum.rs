//! # `p105_cert_level_spectrum` —— nest 证书级别谱统计（任务 #105 事实底座，只读探针）
//!
//! ## 框架背景（新裁定）
//! 买卖点必须由背驰出现的级别向下递归到最小级别做区间套确认；nest 证书 = 递归区间套
//! 背驰的成立记录，是买卖点『级别形成』的构成条件——下游按级别用证书，不按权重用。
//! 本探针回答：66 张证书（#105 口径 A=41/B=25）的级别身份（exec/top/链深/方向×级别），
//! 并对照全量 BSP（生产 `IncrementalClassifier` 因果塔）的级别分布，给出证书标出的
//! 『递归确认到证书级的高级别买卖点』在级别谱上的落点与各级别占比。
//!
//! ## 两侧口径
//! - **CERT 侧**：读 `P92_DUMP` 末尾 `CERT ` 明细行（`p92_nest_replay_postruling` 产物，
//!   rust/src/bin/p92_nest_replay_postruling.rs:824）。ids 身份向量（高→低，含基例），
//!   链深 = ids 段数 = top − exec + 1。
//! - **BSP 侧**：生产 `IncrementalClassifier::classify_at(i)` + 与 runner
//!   `newly_confirmed_step` 逐字相同的 seen-set append-only diff（644 meta-rule：
//!   探针必走生产路径，无影子分叉；与 p100_cert_bsp_recon 同一提取循环）。
//!
//! ## 层级索引对齐（结构身份连接的依据）
//! - nest 事件 `level` 直接索引 `classification.levels[event.level]`
//!   （生产装配门 `terminal_bits_new`，p92_nest_replay_postruling.rs:931-944）；
//! - 增量分类器 `levels` 按 `level_idx in 0..=l_max` 逐级 push（classifier/mod.rs:1553,1595,2015），
//!   p92 provider `by_level[level]` 用 `tower[level]` 构造（p92_nest_replay_postruling.rs:618-619）。
//!   ⟹ **nest 级 k == classifier lvl k == 塔级 k**，不做任何换算。
//! - 基例身份 (exec, turn_source) 必有同级同向 BSP（生产门 `confirm_side`，
//!   types.rs:216-221）；rung 身份无此门（nest.rs:550-551 D1 裁定：rung 不要求
//!   divergence_confirmed），其 BSP 命中率为实证问题，由本探针测。
//!
//! ## 只读纪律
//! 不动判据、不写主路径状态；输出 `P105_*` 报表行到 stdout。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin p105_cert_level_spectrum -- <P92_DUMP路径>
//! ```

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::backtest::incremental::IncrementalClassifier;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::types::BspBits;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// 与 runner.rs `bsp_bits_disc` 同口径——6 类 bit 打包（身份判别用）。
fn bsp_bits_disc(b: &BspBits) -> u8 {
    (b.buy1 as u8)
        | (b.buy2 as u8) << 1
        | (b.buy3 as u8) << 2
        | (b.sell1 as u8) << 3
        | (b.sell2 as u8) << 4
        | (b.sell3 as u8) << 5
}

fn bits_buy(b: &BspBits) -> bool {
    b.buy1 || b.buy2 || b.buy3
}

fn bits_sell(b: &BspBits) -> bool {
    b.sell1 || b.sell2 || b.sell3
}

/// 链上单级身份（dump ids 段 `level:turn_source:start-end`）。
/// start/end 为 interval_b 区间（本任务只消费 level/turn_source 身份键，区间保留备查）。
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct ChainId {
    level: usize,
    turn_source: usize,
    start: usize,
    end: usize,
}

#[derive(Debug)]
struct Cert {
    caliber: String,
    exec: usize,
    top: usize,
    side: String, // "Long" / "Short"
    bucket: String,
    kinds: String,
    chain: Vec<ChainId>, // 高→低，含基例（末元 = 基例）
}

/// 解析 dump 中 `CERT key=value ...` 行（p92_nest_replay_postruling.rs:824 发射格式）。
fn parse_certs(path: &str) -> Result<Vec<Cert>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读 {path} 失败: {e}"))?;
    let mut out = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        let Some(rest) = line.strip_prefix("CERT ") else {
            continue;
        };
        let mut caliber = String::new();
        let mut side = String::new();
        let mut bucket = String::new();
        let mut kinds = String::new();
        let mut exec: usize = 0;
        let mut top: usize = 0;
        let mut chain: Vec<ChainId> = Vec::new();
        for token in rest.split_whitespace() {
            let Some((key, value)) = token.split_once('=') else {
                continue;
            };
            match key {
                "caliber" => caliber = value.to_string(),
                "side" => side = value.to_string(),
                "bucket" => bucket = value.to_string(),
                "kinds" => kinds = value.to_string(),
                "exec" => exec = value.parse().unwrap_or(0),
                "top" => top = value.parse().unwrap_or(0),
                "ids" => {
                    for seg in value.split('|') {
                        let parts: Vec<&str> = seg.split(':').collect();
                        if parts.len() != 3 {
                            return Err(format!("dump:{} ids 段畸形: {seg}", lineno + 1));
                        }
                        let level = parts[0]
                            .parse()
                            .map_err(|_| format!("dump:{} level 畸形: {seg}", lineno + 1))?;
                        let turn_source = parts[1]
                            .parse()
                            .map_err(|_| format!("dump:{} turn_source 畸形: {seg}", lineno + 1))?;
                        let Some((s, e)) = parts[2].split_once('-') else {
                            return Err(format!("dump:{} 区间畸形: {seg}", lineno + 1));
                        };
                        chain.push(ChainId {
                            level,
                            turn_source,
                            start: s.parse().unwrap_or(0),
                            end: e.parse().unwrap_or(0),
                        });
                    }
                }
                _ => {}
            }
        }
        if caliber.is_empty() || chain.is_empty() {
            return Err(format!("dump:{} CERT 行缺字段", lineno + 1));
        }
        // 结构自校验：首元 level 必须等于 top，末元 level 必须等于 exec（ids 高→低含基例，
        // nest.rs:407-413 注释 + p92 :805-810 发射序）。
        if chain[0].level != top || chain[chain.len() - 1].level != exec {
            return Err(format!(
                "dump:{} 链级与 exec/top 不一致: exec={exec} top={top} chain[0].level={} chain.last={}",
                lineno + 1,
                chain[0].level,
                chain[chain.len() - 1].level
            ));
        }
        out.push(Cert {
            caliber,
            exec,
            top,
            side,
            bucket,
            kinds,
            chain,
        });
    }
    Ok(out)
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("用法: p105_cert_level_spectrum <P92_DUMP路径>");
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
        "P105_INPUT bars={n} certs_total={} A={n_a} B={n_b}",
        certs.len()
    );

    // ── 一、证书级别身份（纯 dump 侧，无需塔）────────────────────────────
    for caliber in ["A", "B"] {
        let mine: Vec<&Cert> = certs.iter().filter(|c| c.caliber == caliber).collect();
        let mut exec_dist: BTreeMap<usize, usize> = BTreeMap::new();
        let mut top_dist: BTreeMap<usize, usize> = BTreeMap::new();
        let mut exectop: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        let mut depth_dist: BTreeMap<usize, usize> = BTreeMap::new();
        let mut side_top: BTreeMap<(String, usize), usize> = BTreeMap::new();
        let mut side_exec: BTreeMap<(String, usize), usize> = BTreeMap::new();
        let mut side_n: BTreeMap<String, usize> = BTreeMap::new();
        for c in &mine {
            *exec_dist.entry(c.exec).or_default() += 1;
            *top_dist.entry(c.top).or_default() += 1;
            *exectop.entry((c.exec, c.top)).or_default() += 1;
            *depth_dist.entry(c.chain.len()).or_default() += 1;
            *side_top.entry((c.side.clone(), c.top)).or_default() += 1;
            *side_exec.entry((c.side.clone(), c.exec)).or_default() += 1;
            *side_n.entry(c.side.clone()).or_default() += 1;
        }
        for (side, k) in &side_n {
            println!("P105_CERT_SIDE caliber={caliber} side={side} n={k}");
        }
        for (e, k) in &exec_dist {
            println!("P105_CERT_EXEC caliber={caliber} exec={e} n={k}");
        }
        for (t, k) in &top_dist {
            println!("P105_CERT_TOP caliber={caliber} top={t} n={k}");
        }
        for ((e, t), k) in &exectop {
            println!("P105_CERT_EXECTOP caliber={caliber} exec={e} top={t} n={k}");
        }
        for (d, k) in &depth_dist {
            println!("P105_CERT_DEPTH caliber={caliber} depth={d} n={k}");
        }
        for ((side, t), k) in &side_top {
            println!("P105_CERT_SIDE_TOP caliber={caliber} side={side} top={t} n={k}");
        }
        for ((side, e), k) in &side_exec {
            println!("P105_CERT_SIDE_EXEC caliber={caliber} side={side} exec={e} n={k}");
        }
    }

    // ── 二、BSP 侧：生产因果塔 + runner 同语义 seen-set diff（与 p100 同循环）──
    let mut classifier = IncrementalClassifier::new(bars, &config);
    let mut seen: HashSet<(usize, usize, u8)> = HashSet::new();
    // (lvl, source_index) → (首次确认 bar, bits 并集)
    let mut by_id: HashMap<(usize, usize), (i64, BspBits)> = HashMap::new();
    // 每级 (buy 事件数, sell 事件数)：一个事件可同时带买卖 bit，按 bit 各计一次（与 p100 同口径）。
    let mut lvl_bs: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    let mut events_total = 0usize;
    for i in 0..n {
        let (classification, _tower) = classifier.classify_at(i);
        for (lvl, ls) in classification.levels.iter().enumerate() {
            for p in ls.bsp.iter() {
                if seen.insert((lvl, p.source_index, bsp_bits_disc(&p.bits))) {
                    let b = &p.bits;
                    let buy = bits_buy(b);
                    let sell = bits_sell(b);
                    if buy || sell {
                        events_total += 1;
                        let e = lvl_bs.entry(lvl).or_default();
                        e.0 += buy as usize;
                        e.1 += sell as usize;
                        by_id
                            .entry((lvl, p.source_index))
                            .and_modify(|(_bar, acc)| {
                                acc.buy1 |= b.buy1;
                                acc.buy2 |= b.buy2;
                                acc.buy3 |= b.buy3;
                                acc.sell1 |= b.sell1;
                                acc.sell2 |= b.sell2;
                                acc.sell3 |= b.sell3;
                            })
                            .or_insert((i as i64, *b));
                    }
                }
            }
        }
        if i % 500_000 == 0 && i > 0 {
            eprintln!("P105_PROGRESS bar={i}/{n} events={events_total}");
        }
    }
    let max_lvl = *lvl_bs.keys().max().unwrap_or(&0);
    let tot_b: usize = lvl_bs.values().map(|v| v.0).sum();
    let tot_s: usize = lvl_bs.values().map(|v| v.1).sum();
    println!("P105_BSP events_total={events_total} buys={tot_b} sells={tot_s} max_lvl={max_lvl}");
    for lvl in 0..=max_lvl {
        let (b, s) = lvl_bs.get(&lvl).copied().unwrap_or((0, 0));
        println!("P105_BSP_LVL lvl={lvl} buys={b} sells={s} total={}", b + s);
    }
    // 每级去重 (lvl, source) 身份数（事件数按 bits 展开会多于身份数）。
    let mut lvl_ids: BTreeMap<usize, usize> = BTreeMap::new();
    for (lvl, _) in by_id.keys() {
        *lvl_ids.entry(*lvl).or_default() += 1;
    }
    for lvl in 0..=max_lvl {
        println!(
            "P105_BSP_IDS lvl={lvl} distinct_sources={}",
            lvl_ids.get(&lvl).copied().unwrap_or(0)
        );
    }

    // ── 三、身份连接：证书链身份 → 同级 BSP（terminal_bits_new 同构查法）──
    // 命中定义：by_id 存在 (level, turn_source)；side_hit：其 bits 确认证书方向。
    let side_confirms = |bits: &BspBits, side: &str| {
        if side == "Long" {
            bits_buy(bits)
        } else {
            bits_sell(bits)
        }
    };
    for caliber in ["A", "B"] {
        let mine: Vec<&Cert> = certs.iter().filter(|c| c.caliber == caliber).collect();
        let mut base_hit = 0usize;
        let mut top_hit = 0usize;
        let mut top_side_hit = 0usize;
        let mut rung_hit = 0usize;
        let mut rung_total = 0usize;
        // 被证书标记的去重 BSP 身份：(lvl, source)。top 集 / 全链集，各按方向拆。
        let mut mark_top: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        let mut mark_top_side: BTreeMap<(usize, String), BTreeSet<usize>> = BTreeMap::new();
        let mut mark_chain: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        let mut mark_chain_side: BTreeMap<(usize, String), BTreeSet<usize>> = BTreeMap::new();
        for c in &mine {
            let base = c.chain[c.chain.len() - 1];
            let top_id = c.chain[0];
            let mut per_chain_marks: Vec<(usize, usize)> = Vec::new();
            for (pos, id) in c.chain.iter().enumerate() {
                rung_total += 1;
                if let Some((_, bits)) = by_id.get(&(id.level, id.turn_source)) {
                    rung_hit += 1;
                    per_chain_marks.push((id.level, id.turn_source));
                    if pos == 0 {
                        top_hit += 1;
                        if side_confirms(bits, &c.side) {
                            top_side_hit += 1;
                        }
                    }
                    if pos == c.chain.len() - 1 {
                        base_hit += 1;
                    }
                }
            }
            // 标记集只收『同级确有 BSP 身份』的落点（买卖点真实落在级别谱上的位置）。
            for (lvl, src) in per_chain_marks {
                mark_chain.entry(lvl).or_default().insert(src);
                mark_chain_side
                    .entry((lvl, c.side.clone()))
                    .or_default()
                    .insert(src);
            }
            if by_id.contains_key(&(top_id.level, top_id.turn_source)) {
                mark_top
                    .entry(top_id.level)
                    .or_default()
                    .insert(top_id.turn_source);
                mark_top_side
                    .entry((top_id.level, c.side.clone()))
                    .or_default()
                    .insert(top_id.turn_source);
            }
            let base_ok = by_id.contains_key(&(base.level, base.turn_source));
            let top_ok = by_id.contains_key(&(top_id.level, top_id.turn_source));
            println!(
                "P105_CERT_DETAIL caliber={} side={} exec={} top={} depth={} bucket={} kinds={} base_id={}:{} base_bsp={} top_id={}:{} top_bsp={} chain={}",
                c.caliber,
                c.side,
                c.exec,
                c.top,
                c.chain.len(),
                c.bucket,
                c.kinds,
                base.level,
                base.turn_source,
                base_ok,
                top_id.level,
                top_id.turn_source,
                top_ok,
                c.chain
                    .iter()
                    .map(|id| format!("{}:{}", id.level, id.turn_source))
                    .collect::<Vec<_>>()
                    .join("|")
            );
        }
        println!(
            "P105_JOIN caliber={caliber} n={} base_hit={base_hit} top_hit={top_hit} top_side_hit={top_side_hit} rung_hit={rung_hit} rung_total={rung_total}",
            mine.len()
        );
        for lvl in 0..=max_lvl {
            let mt = mark_top.get(&lvl).map_or(0, |s| s.len());
            let mc = mark_chain.get(&lvl).map_or(0, |s| s.len());
            let (b, s) = lvl_bs.get(&lvl).copied().unwrap_or((0, 0));
            let total = b + s;
            println!(
                "P105_MARK_LVL caliber={caliber} lvl={lvl} bsp_total={total} top_marked={mt} chain_marked={mc} top_rate={:.6} chain_rate={:.6}",
                mt as f64 / total.max(1) as f64,
                mc as f64 / total.max(1) as f64
            );
            for side in ["Long", "Short"] {
                let key = (lvl, side.to_string());
                let mts = mark_top_side.get(&key).map_or(0, |s| s.len());
                let mcs = mark_chain_side.get(&key).map_or(0, |s| s.len());
                let denom = if side == "Long" { b } else { s };
                println!(
                    "P105_MARK_LVL_SIDE caliber={caliber} lvl={lvl} side={side} bsp_side={denom} top_marked={mts} chain_marked={mcs} top_rate={:.6} chain_rate={:.6}",
                    mts as f64 / denom.max(1) as f64,
                    mcs as f64 / denom.max(1) as f64
                );
            }
        }
    }
    println!("P105_DONE");
    std::process::ExitCode::SUCCESS
}
