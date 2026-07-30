//! # `p106_cert_forward_pnl` —— 证书绑定组 vs 未绑定组前向收益分组实验（task #106，只读探针）
//!
//! ## 性质声明（090 纪律：声明与能力一致）
//! - 这是**策略层级别标定实验**：只回答「入场 BSP 落在 nest 证书绑定窗口内的交易，
//!   其前向收益分布（mfe/fin_ok/fin_ret）与其余交易是否可区分」。
//! - **不碰判据**：不修改/不暗示 divergence/bsp/signal/level_view/nest 任何判据改动。
//! - **不做信号升格结论**：分组差异是描述性统计，不升格为入场过滤/加权规则；
//!   绑定组样本量小（证书 66 张），所有数字须带小样本谨慎读。
//! - 新框架语境：nest 证书 = 买卖点级别形成的构成条件；本实验测量的是
//!   「证书已绑定的那批 BSP 入场」在现有交易流中的前向表现，供下游按级别用证书的
//!   标定参考，不是证书有效性的独立验证（证书与 BSP 的空间一致性已由 #100 对账固定）。
//!
//! ## 绑定规则（逐字镜像 p101 探针，cert-bsp-binding-ruling DRAFT 条款 2/3/4）
//! - 钟：cert 取 `judge_at` 逗号钟表的 max（judge_max）；BSP 取确认 bar。禁 created_at。
//! - 匹配：最近**同侧** BSP（Long↔buy bits / Short↔sell bits）；|dt| 相等时 forward 胜。
//! - 强度：|dt| ≤ 240 → strong；240 < |dt| ≤ 1440 → weak；>1440 → unbound。
//! - 分组主口径 V1：交易 `(entry_bar, dir)` 命中某张证书绑定的 `(bsp_bar, side)` ⟹ 绑定组。
//!   敏感性口径 V2：存在同侧证书 |entry_bar − judge_max| ≤ 1440（窗口成员，非最近规则）。
//!
//! ## BSP 侧口径（与 p100/p101 同源）
//! 生产因果塔：`ParseLayerIncr::append` + `classify_with_tower_incremental`（TowerCache 跨 bar
//! 复用）+ runner 同语义 seen-set diff（见 strict_nest_check.rs:53-86 同款内联先例；
//! backtest::incremental::IncrementalClassifier 门控于 backtest_bin，本探针不依赖该 feature，
//! 两次调用本身逐字相同）。bar 加载镜像 `backtest::data::load_symbol`（NaN→null、缺 OHLC 前收
//! 填充、untradable 标注不删除）。
//! **验证门**：P106_BSP 必须逐字复现 p100/p101 的 events_total=28417 buys=14762 sells=13655
//! 及 L0-L4 分层；不符即暴露口径漂移，停线 escalate。
//!
//! ## 只读纪律
//! 不动判据、不写主路径状态、不写主仓；输出 `P106_*` 报表行到 stdout。
//!
//! ## 用法
//! ```text
//! cargo run --release --bin p106_cert_forward_pnl -- \
//!   <btc_1m_full.json> <P92_DUMP路径> <trades.jsonl路径>
//! ```

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser;
use newchan_rust::theta_v0::types::{quantize, Bar, BspBits, Timestamp};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

// ═══════════════════════ bar 加载（镜像 backtest/data.rs load_symbol:207-286） ═══════════════════════

#[derive(Debug, Deserialize)]
struct RawData {
    opens: Vec<Option<f64>>,
    highs: Vec<Option<f64>>,
    lows: Vec<Option<f64>>,
    closes: Vec<Option<f64>>,
    #[serde(default)]
    volumes: Vec<Option<f64>>,
    dates: Vec<String>,
}

/// 与 data.rs `date_to_timestamp`（:182-189）同口径：数字抽取，≤14 位 YYYYMMDDHHMMSS。
fn date_to_timestamp(date: &str) -> Timestamp {
    let digits: String = date
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(14)
        .collect();
    digits.parse::<i64>().unwrap_or(0)
}

fn load_bars(path: &Path, config: &ThetaConfig) -> Result<(Vec<Bar>, Vec<String>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读取 {path:?} 失败: {e}"))?;
    // Python json(allow_nan) 写出 NaN/Infinity（serde_json 硬拒）→ null（与 data.rs:209-212 同）。
    let text = text
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    let raw: RawData =
        serde_json::from_str(&text).map_err(|e| format!("解析 {path:?} 失败: {e}"))?;
    drop(text);
    let n = raw.closes.len();
    if n == 0 {
        return Err(format!("{path:?} 无 closes 数据"));
    }
    for (name, len) in [
        ("opens", raw.opens.len()),
        ("highs", raw.highs.len()),
        ("lows", raw.lows.len()),
        ("dates", raw.dates.len()),
    ] {
        if len != n {
            return Err(format!("列长度不一致：{name}={len} vs closes={n}"));
        }
    }
    let tick_size = config.tick.tick_size;
    let mut bars: Vec<Bar> = Vec::with_capacity(n);
    for i in 0..n {
        let o = raw.opens[i];
        let h = raw.highs[i];
        let l = raw.lows[i];
        let c = raw.closes[i];
        let v = raw.volumes.get(i).and_then(|x| *x).unwrap_or(0.0);
        let (oq, hq, lq, cq, untradable) =
            if let (Some(o), Some(h), Some(l), Some(c)) = (o, h, l, c) {
                let bad_range = h < o.max(c).max(l) || l > o.min(c).min(h);
                let bad_price = o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0;
                let untradable = bad_range || bad_price || v <= 0.0;
                (
                    quantize(o, tick_size),
                    quantize(h, tick_size),
                    quantize(l, tick_size),
                    quantize(c, tick_size),
                    untradable,
                )
            } else {
                let prev = bars.last().map(|b: &Bar| b.close).unwrap_or(0);
                (prev, prev, prev, prev, true)
            };
        bars.push(Bar {
            source_index: i,
            timestamp: date_to_timestamp(&raw.dates[i]),
            open: oq,
            high: hq,
            low: lq,
            close: cq,
            volume: v as i64,
            untradable,
        });
    }
    Ok((bars, raw.dates))
}

// ═══════════════════════ 增量分类器（strict_nest_check.rs:53-86 同款内联；两次调用与 backtest::incremental 逐字相同） ═══════════════════════

struct IncrementalClassifier<'a> {
    bars: &'a [Bar],
    config: &'a ThetaConfig,
    parser_incr: parser::ParseLayerIncr<'a>,
    tower_cache: classifier::TowerCache,
}

impl<'a> IncrementalClassifier<'a> {
    fn new(bars: &'a [Bar], config: &'a ThetaConfig) -> Self {
        Self {
            bars,
            config,
            parser_incr: parser::ParseLayerIncr::new(config),
            tower_cache: classifier::TowerCache::new(),
        }
    }

    fn classify_at(&mut self, i: usize) -> classifier::Classification {
        let l0_i = self.parser_incr.append(self.bars[i]);
        let (classification, _tower) =
            classifier::classify_with_tower_incremental(&l0_i, self.config, &mut self.tower_cache);
        classification
    }
}

/// 与 runner.rs `bsp_bits_disc` / p101:33-40 同口径——6 类 bit 打包（身份判别用）。
fn bsp_bits_disc(b: &BspBits) -> u8 {
    (b.buy1 as u8)
        | (b.buy2 as u8) << 1
        | (b.buy3 as u8) << 2
        | (b.sell1 as u8) << 3
        | (b.sell2 as u8) << 4
        | (b.sell3 as u8) << 5
}

// ═══════════════════════ 证书解析（p101:52-98 镜像 + exec/top 分层字段） ═══════════════════════

#[derive(Debug)]
struct Cert {
    caliber: String,
    side: String,
    exec: usize,
    top: usize,
    judge_max: i64,
    bucket: String,
    kinds: String,
    ids: String,
}

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
        let mut exec = 0usize;
        let mut top = 0usize;
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
                "exec" => exec = value.parse().unwrap_or(0),
                "top" => top = value.parse().unwrap_or(0),
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
                exec,
                top,
                judge_max,
                bucket,
                kinds,
                ids,
            });
        }
    }
    Ok(out)
}

/// 最近元素带符号距离（bsp_bar - x）；|dt| 相等时 forward 胜（与 p101:102-121 逐字同规则）。
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

/// 强度分档（与 p101:123-129 逐字同规则）。
fn strength(dt: i64) -> &'static str {
    match dt.abs() {
        0..=240 => "strong",
        241..=1440 => "weak",
        _ => "unbound",
    }
}

// ═══════════════════════ trades.jsonl 解析 ═══════════════════════

#[derive(Debug, Deserialize)]
struct Trade {
    seg_idx: usize,
    entry_bar: usize,
    entry: i64,
    dir: i8,
    exit_bar: usize,
    exit: i64,
    mfe: Option<f64>,
    fin_ok: bool,
    dir_match: bool,
    entry_day: String,
}

fn load_trades(path: &str) -> Result<Vec<Trade>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读 {path} 失败: {e}"))?;
    let mut out = Vec::new();
    for (ln, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let t: Trade =
            serde_json::from_str(line).map_err(|e| format!("行 {}: JSON 解析失败: {e}", ln + 1))?;
        if t.dir != 1 && t.dir != -1 {
            return Err(format!("行 {}: dir={} 非 ±1", ln + 1, t.dir));
        }
        out.push(t);
    }
    Ok(out)
}

// ═══════════════════════ 分组统计 ═══════════════════════

#[derive(Default)]
struct StatAcc {
    n: usize,
    fin_ok: usize,
    dir_match: usize,
    mfe_null: usize,
    mfe: Vec<f64>,
    fin_ret: Vec<f64>,
}

impl StatAcc {
    fn push(&mut self, t: &Trade) {
        self.n += 1;
        self.fin_ok += t.fin_ok as usize;
        self.dir_match += t.dir_match as usize;
        match t.mfe {
            Some(v) => self.mfe.push(v),
            None => self.mfe_null += 1,
        }
        // 终值收益（派生字段）：dir × (exit − entry)/entry；entry/exit 为 tick 整数价。
        if t.entry > 0 {
            self.fin_ret
                .push(t.dir as f64 * (t.exit - t.entry) as f64 / t.entry as f64);
        }
    }
}

/// 分位数：排序后 nearest-rank（round(q×(n−1))），方法随输出声明。
fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let idx = (q * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx]
}

fn emit_group(cut: &str, name: &str, acc: &StatAcc) {
    let mut mfe = acc.mfe.clone();
    mfe.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut fr = acc.fin_ret.clone();
    fr.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = |v: &[f64]| {
        if v.is_empty() {
            f64::NAN
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    println!(
        "P106_GROUP cut={cut} group={name} n={} mfe_n={} mfe_null={} mfe_mean={:.6e} mfe_p25={:.6e} mfe_p50={:.6e} mfe_p75={:.6e} mfe_p90={:.6e} finret_mean={:.6e} finret_p50={:.6e} fin_ok_rate={:.6} dir_match_rate={:.6}",
        acc.n,
        mfe.len(),
        acc.mfe_null,
        mean(&mfe),
        quantile(&mfe, 0.25),
        quantile(&mfe, 0.50),
        quantile(&mfe, 0.75),
        quantile(&mfe, 0.90),
        mean(&fr),
        quantile(&fr, 0.50),
        acc.fin_ok as f64 / acc.n.max(1) as f64,
        acc.dir_match as f64 / acc.n.max(1) as f64,
    );
}

// ═══════════════════════ 绑定键属性（多证书同键合并：max exec / min|dt| / caliber 集合） ═══════════════════════

#[derive(Debug, Clone)]
struct KeyAttr {
    max_exec: usize,
    min_abs_dt: i64,
    dt_of_min_abs: i64,
    calibers: HashSet<char>,
    n_certs: usize,
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!(
            "用法: p106_cert_forward_pnl <btc_1m_full.json> <P92_DUMP路径> <trades.jsonl路径>"
        );
        return std::process::ExitCode::from(2);
    }
    let config = ThetaConfig::default();
    let (bars, dates) = match load_bars(Path::new(&args[1]), &config) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let certs = match parse_certs(&args[2]) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let trades = match load_trades(&args[3]) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let n = bars.len();
    let n_a = certs.iter().filter(|c| c.caliber == "A").count();
    println!(
        "P106_INPUT bars={n} certs_total={} A={n_a} B={} trades={} first_date={} last_date={}",
        certs.len(),
        certs.len() - n_a,
        trades.len(),
        dates.first().cloned().unwrap_or_default(),
        dates.last().cloned().unwrap_or_default()
    );

    // ── BSP 侧：生产因果塔 + runner 同语义 seen-set diff（与 p101:157-178 同一提取循环）。 ──
    let mut classifier = IncrementalClassifier::new(&bars, &config);
    let mut seen: HashSet<(usize, usize, u8)> = HashSet::new();
    let mut events: Vec<(i64, usize, bool, bool)> = Vec::new();
    for i in 0..n {
        let classification = classifier.classify_at(i);
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
            eprintln!("P106_PROGRESS bar={i}/{n} events={}", events.len());
        }
    }
    let long_bars: Vec<i64> = events.iter().filter(|e| e.2).map(|e| e.0).collect();
    let short_bars: Vec<i64> = events.iter().filter(|e| e.3).map(|e| e.0).collect();
    println!(
        "P106_BSP events_total={} buys={} sells={}",
        events.len(),
        long_bars.len(),
        short_bars.len()
    );
    for lvl in 0..=4usize {
        let b = events.iter().filter(|e| e.1 == lvl && e.2).count();
        let s = events.iter().filter(|e| e.1 == lvl && e.3).count();
        println!("P106_BSP_LVL lvl={lvl} buys={b} sells={s}");
    }
    // 全量 BSP (bar,side) 集（交易入场是否为 BSP 的 sanity 用）。
    let bsp_set: HashSet<(i64, bool)> = events
        .iter()
        .flat_map(|e| {
            let mut v = Vec::with_capacity(2);
            if e.2 {
                v.push((e.0, true));
            }
            if e.3 {
                v.push((e.0, false));
            }
            v
        })
        .collect();

    // ── 逐证书绑定物化（p101:188-222 镜像；unbound 不进绑定表，修正 p101 缺口 a 的语义）。 ──
    let mut unbound = 0usize;
    let mut ties = 0usize;
    let mut bound_map: BTreeMap<(i64, bool), KeyAttr> = BTreeMap::new();
    let mut counts: BTreeMap<(String, String), (usize, usize)> = BTreeMap::new();
    let mut judge_long: Vec<i64> = Vec::new();
    let mut judge_short: Vec<i64> = Vec::new();
    for (ci, c) in certs.iter().enumerate() {
        let is_long = c.side.contains("Long");
        if is_long {
            judge_long.push(c.judge_max);
        } else {
            judge_short.push(c.judge_max);
        }
        let same = if is_long { &long_bars } else { &short_bars };
        let Some((dt, tie)) = nearest_dt_tie(same, c.judge_max) else {
            println!(
                "P106_BIND idx={ci} caliber={} side={} judge_max={} bsp_bar=NONE",
                c.caliber, c.side, c.judge_max
            );
            unbound += 1;
            continue;
        };
        let s = strength(dt);
        if s == "unbound" {
            println!(
                "P106_BIND idx={ci} caliber={} side={} judge_max={} bsp_bar=NONE dt={dt}（>1440，按条款 3 escalate 可见，不入绑定表）",
                c.caliber, c.side, c.judge_max
            );
            unbound += 1;
            continue;
        }
        if tie {
            ties += 1;
        }
        let bsp_bar = c.judge_max + dt;
        let attr = bound_map
            .entry((bsp_bar, is_long))
            .or_insert_with(|| KeyAttr {
                max_exec: c.exec,
                min_abs_dt: dt.abs(),
                dt_of_min_abs: dt,
                calibers: HashSet::new(),
                n_certs: 0,
            });
        attr.max_exec = attr.max_exec.max(c.exec);
        if dt.abs() < attr.min_abs_dt {
            attr.min_abs_dt = dt.abs();
            attr.dt_of_min_abs = dt;
        }
        attr.calibers
            .insert(c.caliber.chars().next().unwrap_or('?'));
        attr.n_certs += 1;
        let e = counts
            .entry((c.caliber.clone(), s.to_string()))
            .or_insert((0, 0));
        e.0 += 1;
        e.1 += tie as usize;
        println!(
            "P106_BIND idx={ci} caliber={} side={} exec={} top={} judge_max={} bsp_bar={bsp_bar} dt={dt} strength={s} tie={} bucket={} kinds={} ids={}",
            c.caliber, c.side, c.exec, c.top, c.judge_max, tie as u8, c.bucket, c.kinds, c.ids
        );
    }
    for ((caliber, s), (cnt, tie_cnt)) in &counts {
        println!("P106_BIND_COUNT caliber={caliber} strength={s} n={cnt} ties={tie_cnt}");
    }
    for ((bar, is_long), attr) in bound_map.iter().filter(|(_, v)| v.n_certs > 1) {
        println!(
            "P106_MULTI bsp_bar={bar} side={} n_certs={} max_exec={} calibers={:?}",
            if *is_long { "Long" } else { "Short" },
            attr.n_certs,
            attr.max_exec,
            attr.calibers
        );
    }
    let multi = bound_map.values().filter(|v| v.n_certs > 1).count();
    println!(
        "P106_BIND_SUMMARY certs={} bound={} unbound={unbound} ties={ties} distinct_bsp={} multi_bsp={multi}",
        certs.len(),
        certs.len() - unbound,
        bound_map.len()
    );
    judge_long.sort_unstable();
    judge_short.sort_unstable();

    // ── 交易侧：单调性 / BSP 成员 sanity / 样本内外切分点。 ──
    let monotonic = trades.windows(2).all(|w| w[0].entry_bar <= w[1].entry_bar);
    let mfe_null_total = trades.iter().filter(|t| t.mfe.is_none()).count();
    let bsp_member = trades
        .iter()
        .filter(|t| bsp_set.contains(&(t.entry_bar as i64, t.dir == 1)))
        .count();
    println!(
        "P106_TRADES n={} monotonic={} mfe_null={} bsp_member={} bsp_member_rate={:.6}",
        trades.len(),
        monotonic as u8,
        mfe_null_total,
        bsp_member,
        bsp_member as f64 / trades.len().max(1) as f64
    );
    // 样本内/外：按 seg_idx 时间序 70/30 机械切分（无预注册切分，声明为机械分段）。
    let is_last = (trades.len() as f64 * 0.7) as usize; // seg_idx ≤ is_last ⟹ 样本内
    let is_day = trades
        .iter()
        .find(|t| t.seg_idx == is_last)
        .map(|t| t.entry_day.clone())
        .unwrap_or_default();
    let oos_day = trades
        .iter()
        .find(|t| t.seg_idx == is_last + 1)
        .map(|t| t.entry_day.clone())
        .unwrap_or_default();
    println!(
        "P106_SPLIT is_seg_idx<={is_last} is_last_day={is_day} oos_first_day={oos_day} is_n={} oos_n={}",
        trades.iter().filter(|t| t.seg_idx <= is_last).count(),
        trades.iter().filter(|t| t.seg_idx > is_last).count()
    );

    // ── 分组与统计。 ──
    let mut full_all = StatAcc::default();
    let mut v1_in = StatAcc::default();
    let mut v1_out = StatAcc::default();
    let mut v2_in = StatAcc::default();
    let mut v2_out = StatAcc::default();
    let mut exec1 = StatAcc::default();
    let mut exec2p = StatAcc::default();
    let mut strong = StatAcc::default();
    let mut weak = StatAcc::default();
    let mut causal = StatAcc::default();
    let mut expost = StatAcc::default();
    let mut cal_a = StatAcc::default();
    let mut cal_b = StatAcc::default();
    let mut cal_ab = StatAcc::default();
    let mut is_in = StatAcc::default();
    let mut is_out = StatAcc::default();
    let mut oos_in = StatAcc::default();
    let mut oos_out = StatAcc::default();
    let mut is_all = StatAcc::default();
    let mut oos_all = StatAcc::default();
    let mut v1_dir_long = StatAcc::default();
    let mut v1_dir_short = StatAcc::default();

    for t in &trades {
        let is_long = t.dir == 1;
        let key = (t.entry_bar as i64, is_long);
        let attr = bound_map.get(&key);
        // V2 敏感性口径：最近同侧 judge_max 距离 ≤1440。
        let judges = if is_long { &judge_long } else { &judge_short };
        let v2_hit =
            nearest_dt_tie(judges, t.entry_bar as i64).is_some_and(|(dt, _)| dt.abs() <= 1440);

        full_all.push(t);
        if t.seg_idx <= is_last {
            is_all.push(t);
        } else {
            oos_all.push(t);
        }
        if v2_hit {
            v2_in.push(t);
        } else {
            v2_out.push(t);
        }
        if let Some(a) = attr {
            v1_in.push(t);
            if is_long {
                v1_dir_long.push(t);
            } else {
                v1_dir_short.push(t);
            }
            if a.max_exec >= 2 {
                exec2p.push(t);
            } else {
                exec1.push(t);
            }
            if a.min_abs_dt <= 240 {
                strong.push(t);
            } else {
                weak.push(t);
            }
            if a.dt_of_min_abs >= 0 {
                causal.push(t);
            } else {
                expost.push(t);
            }
            match (a.calibers.contains(&'A'), a.calibers.contains(&'B')) {
                (true, true) => cal_ab.push(t),
                (true, false) => cal_a.push(t),
                (false, true) => cal_b.push(t),
                _ => {}
            }
            if t.seg_idx <= is_last {
                is_in.push(t);
            } else {
                oos_in.push(t);
            }
        } else {
            v1_out.push(t);
            if t.seg_idx <= is_last {
                is_out.push(t);
            } else {
                oos_out.push(t);
            }
        }
    }

    emit_group("full", "all", &full_all);
    emit_group("v1", "bound", &v1_in);
    emit_group("v1", "unbound", &v1_out);
    emit_group("v1_bound_dir", "long", &v1_dir_long);
    emit_group("v1_bound_dir", "short", &v1_dir_short);
    emit_group("v1_bound_exec", "exec1", &exec1);
    emit_group("v1_bound_exec", "exec2plus", &exec2p);
    emit_group("v1_bound_strength", "strong", &strong);
    emit_group("v1_bound_strength", "weak", &weak);
    emit_group("v1_bound_causal", "dt_ge0", &causal);
    emit_group("v1_bound_causal", "dt_lt0", &expost);
    emit_group("v1_bound_caliber", "A_only", &cal_a);
    emit_group("v1_bound_caliber", "B_only", &cal_b);
    emit_group("v1_bound_caliber", "AB", &cal_ab);
    emit_group("v2", "in_window", &v2_in);
    emit_group("v2", "out_window", &v2_out);
    emit_group("is", "all", &is_all);
    emit_group("is", "bound", &is_in);
    emit_group("is", "unbound", &is_out);
    emit_group("oos", "all", &oos_all);
    emit_group("oos", "bound", &oos_in);
    emit_group("oos", "unbound", &oos_out);

    // 绑定键覆盖到的交易逐键明细（小样本可审计性）。
    let mut per_key: BTreeMap<(i64, bool), usize> = BTreeMap::new();
    for t in &trades {
        let key = (t.entry_bar as i64, t.dir == 1);
        if bound_map.contains_key(&key) {
            *per_key.entry(key).or_default() += 1;
        }
    }
    let mut attr_by_key: HashMap<(i64, bool), &KeyAttr> = HashMap::new();
    for (k, v) in &bound_map {
        attr_by_key.insert(*k, v);
    }
    for ((bar, is_long), cnt) in &per_key {
        let a = attr_by_key[&(*bar, *is_long)];
        println!(
            "P106_KEY bsp_bar={bar} side={} trades={cnt} n_certs={} max_exec={} min_abs_dt={} dt_of_min_abs={} calibers={:?}",
            if *is_long { "Long" } else { "Short" },
            a.n_certs,
            a.max_exec,
            a.min_abs_dt,
            a.dt_of_min_abs,
            a.calibers
        );
    }
    println!(
        "P106_KEY_SUMMARY keys_with_trades={} keys_total={} trades_tagged={}",
        per_key.len(),
        bound_map.len(),
        per_key.values().sum::<usize>()
    );
    println!("P106_DONE");
    std::process::ExitCode::SUCCESS
}
