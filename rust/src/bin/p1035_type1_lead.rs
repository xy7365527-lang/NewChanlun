//! #1035 探针：一类点提前量语义普查——点锚 vs 背驰极值 bar 的逐级 bar 数直方图。
//!
//! 只读探针（不改判据、不写主路径状态）。对每个一类点（buy1/sell1），按级测量：
//! 1. `anchor` = BspPoint.source_index（现点锚）；
//! 2. C 段（破中枢段）= `find_move_by_end_index(tower[lvl], anchor)` 的走势单元，
//!    窗口 = [C.start_index, C.end_index]；
//! 3. 背驰极值 bar = C 窗口内 buy1→argmin(low) / sell1→argmax(high)（取首次命中，ties 取最小下标）；
//! 4. lead_C = anchor − 极值 bar（≥0：点锚在极值之后；=0：点锚与极值同 bar）；
//! 5. 中枢窗口极值 bar（[center.start_index, center.end_index] 同侧极值）与 lead_center；
//! 6. 下钻首步方向（C 段 sub_moves 中 end_index==anchor 的段 rmove_dir）——检验
//!    「下钻对齐恒取反趋势段」的 H1 命题（顺趋势 = dir==−δ，反趋势 = dir==+δ）。
//!
//! 用法：
//! `cargo run --release --features backtest_bin --bin p1035_type1_lead -- BTC`
//! env：`P1035_DUMP`（逐点 JSONL 明细路径）、`P1035_START/P1035_END`（可选日期窗切片）。

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::{Mutex, OnceLock};

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::classifier::bsp::OwnerRef;
use newchan_rust::theta_v0::classifier::cand_predicate::rmove_dir;
use newchan_rust::theta_v0::classifier::classify_with_tower;
use newchan_rust::theta_v0::classifier::recursive_tower::{find_move_by_end_index, LeveledMove};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{Bar, Direction, Side};

static DUMP: OnceLock<Option<Mutex<BufWriter<File>>>> = OnceLock::new();

fn dump_line(s: &str) {
    let sink = DUMP.get_or_init(|| {
        std::env::var("P1035_DUMP")
            .ok()
            .and_then(|path| File::create(path).ok())
            .map(|file| Mutex::new(BufWriter::new(file)))
    });
    if let Some(sink) = sink {
        if let Ok(mut w) = sink.lock() {
            let _ = w.write_all(s.as_bytes());
            let _ = w.write_all(b"\n");
        }
    }
}

fn dump_flush() {
    if let Some(Some(sink)) = DUMP.get() {
        if let Ok(mut w) = sink.lock() {
            let _ = w.flush();
        }
    }
}

/// 窗口 [lo, hi] 内 buy1→low 最小 / sell1→high 最大的 bar（首次命中口径：ties 取最小下标，严格 `<`/`>` 比较）。
fn extreme_bar(bars: &[Bar], lo: usize, hi: usize, side: Side) -> Option<usize> {
    if lo > hi || bars.is_empty() {
        return None;
    }
    let lo = lo.min(bars.len() - 1);
    let hi = hi.min(bars.len() - 1);
    if lo > hi {
        return None;
    }
    let mut best = lo;
    for i in lo..=hi {
        let better = match side {
            Side::Long => bars[i].low < bars[best].low,
            Side::Short => bars[i].high > bars[best].high,
        };
        if better {
            best = i;
        }
    }
    Some(best)
}

fn side_name(s: Side) -> &'static str {
    match s {
        Side::Long => "buy1",
        Side::Short => "sell1",
    }
}

fn dir_name(d: Option<Direction>) -> &'static str {
    match d {
        Some(Direction::Up) => "Up",
        Some(Direction::Down) => "Down",
        None => "None",
    }
}

/// 顺趋势方向（δ 的反向：buy→Down，sell→Up）。
fn trend_dir(side: Side) -> Direction {
    match side {
        Side::Long => Direction::Down,
        Side::Short => Direction::Up,
    }
}

struct Hist {
    bins: BTreeMap<i64, usize>,
    n: usize,
    sum: i64,
}

impl Hist {
    fn new() -> Self {
        Hist {
            bins: BTreeMap::new(),
            n: 0,
            sum: 0,
        }
    }
    fn add(&mut self, lead: i64) {
        *self.bins.entry(lead).or_insert(0) += 1;
        self.n += 1;
        self.sum += lead;
    }
    fn median(&self) -> Option<i64> {
        if self.n == 0 {
            return None;
        }
        let mid = (self.n + 1) / 2;
        let mut acc = 0usize;
        for (&k, &v) in &self.bins {
            acc += v;
            if acc >= mid {
                return Some(k);
            }
        }
        None
    }
    fn pct(&self, p: f64) -> Option<i64> {
        if self.n == 0 {
            return None;
        }
        let target = (self.n as f64 * p).ceil() as usize;
        let mut acc = 0usize;
        for (&k, &v) in &self.bins {
            acc += v;
            if acc >= target {
                return Some(k);
            }
        }
        None
    }
}

fn print_hist(name: &str, h: &Hist) {
    if h.n == 0 {
        println!("{name}: n=0");
        return;
    }
    let min = h.bins.keys().next().copied().unwrap_or(0);
    let max = h.bins.keys().next_back().copied().unwrap_or(0);
    let median = h.median().unwrap_or(0);
    let p90 = h.pct(0.90).unwrap_or(0);
    let p99 = h.pct(0.99).unwrap_or(0);
    let mean = h.sum as f64 / h.n as f64;
    println!(
        "{name}: n={} min={} median={} mean={:.2} p90={} p99={} max={}",
        h.n, min, median, mean, p90, p99, max
    );
    // 紧凑直方图：0..=20 逐 bar，之上分桶（按数值序输出）。
    let mut order: Vec<(i64, usize)> = h.bins.iter().map(|(&k, &v)| (k, v)).collect();
    order.sort_by_key(|(k, _)| *k);
    let mut parts: Vec<String> = Vec::new();
    let mut cur_bucket: Option<(i64, i64, usize)> = None; // (lo, hi, count)
    for (k, v) in order {
        if k <= 20 {
            parts.push(format!("{k}:{v}"));
        } else {
            let lo = if k <= 50 {
                21
            } else if k <= 100 {
                51
            } else if k <= 200 {
                101
            } else {
                201
            };
            let hi = if k <= 50 {
                50
            } else if k <= 100 {
                100
            } else if k <= 200 {
                200
            } else {
                i64::MAX
            };
            match &mut cur_bucket {
                Some((blo, bhi, cnt)) if *blo == lo && *bhi == hi => *cnt += v,
                _ => {
                    if let Some((blo, bhi, cnt)) = cur_bucket {
                        parts.push(if bhi == i64::MAX {
                            format!("{blo}+:{cnt}")
                        } else {
                            format!("{blo}-{bhi}:{cnt}")
                        });
                    }
                    cur_bucket = Some((lo, hi, v));
                }
            }
        }
    }
    if let Some((blo, bhi, cnt)) = cur_bucket {
        parts.push(if bhi == i64::MAX {
            format!("{blo}+:{cnt}")
        } else {
            format!("{blo}-{bhi}:{cnt}")
        });
    }
    println!("  bins {}", parts.join(" "));
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: p1035_type1_lead <SYMBOL>  （SYMBOL=BTC 等，见 data::SYMBOLS）");
        return std::process::ExitCode::from(2);
    }
    let symbol = args[1].clone();
    let config = ThetaConfig::default();

    let full = match load_by_symbol(&symbol, &config) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("数据加载失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let dataset = match (
        std::env::var("P1035_START").ok(),
        std::env::var("P1035_END").ok(),
    ) {
        (Some(s), Some(e)) if !s.is_empty() && !e.is_empty() => full.slice_date_window(&s, &e),
        _ => full,
    };
    let bars = &dataset.bars;
    eprintln!(
        "[p1035] symbol={} bars={} window={}..{}",
        symbol,
        bars.len(),
        dataset
            .dates
            .first()
            .map(|d| d.get(..10).unwrap_or(""))
            .unwrap_or(""),
        dataset
            .dates
            .last()
            .map(|d| d.get(..10).unwrap_or(""))
            .unwrap_or(""),
    );

    let t0 = std::time::Instant::now();
    let l0 = parse_layer(bars, &config);
    eprintln!("[p1035] parse done in {:.1}s", t0.elapsed().as_secs_f64());
    let t1 = std::time::Instant::now();
    let (cls, tower) = classify_with_tower(&l0, &config);
    eprintln!(
        "[p1035] classify done in {:.1}s  levels={}",
        t1.elapsed().as_secs_f64(),
        cls.levels.len()
    );

    // 逐级逐侧直方图。
    let mut lead_c_by: BTreeMap<(usize, &'static str), Hist> = BTreeMap::new();
    let mut lead_center_by: BTreeMap<(usize, &'static str), Hist> = BTreeMap::new();
    let mut lead_sub_by: BTreeMap<(usize, &'static str), Hist> = BTreeMap::new();
    // 下钻首步方向普查。
    let mut descend_dir_by: BTreeMap<(usize, &'static str), BTreeMap<&'static str, usize>> =
        BTreeMap::new();

    let mut total = 0usize;
    let mut no_cmove = 0usize;

    for (lvl, ls) in cls.levels.iter().enumerate() {
        for p in ls.bsp.iter() {
            for side in [Side::Long, Side::Short] {
                let on = match side {
                    Side::Long => p.bits.buy1,
                    Side::Short => p.bits.sell1,
                };
                if !on {
                    continue;
                }
                total += 1;
                let anchor = p.source_index;
                let sname = side_name(side);

                // C 段 = 本级 end_index==anchor 的走势单元。
                let c_move: Option<&LeveledMove> = tower
                    .get(lvl)
                    .and_then(|mv| find_move_by_end_index(mv.as_slice(), anchor).map(|i| &mv[i]));
                let Some(cm) = c_move else {
                    no_cmove += 1;
                    dump_line(&format!(
                        "{{\"lvl\":{lvl},\"side\":\"{sname}\",\"anchor\":{anchor},\"c_move\":null}}"
                    ));
                    continue;
                };
                let (c_start, c_end) = (cm.start_index, cm.end_index);
                let cm_dir = rmove_dir(&cm.rmove);

                // C 窗口极值 bar。
                let ext_c = extreme_bar(bars, c_start, c_end, side);
                let lead_c = ext_c.map(|e| anchor as i64 - e as i64);

                // 中枢窗口极值 bar（OwnerRef::Center）。
                let mut lead_center = None;
                let mut ctr_start = None;
                let mut ctr_end = None;
                if let Some(OwnerRef::Center(c)) = &p.center {
                    ctr_start = Some(c.start_index);
                    ctr_end = Some(c.end_index);
                    if let Some(e) = extreme_bar(bars, c.start_index, c.end_index, side) {
                        lead_center = Some(anchor as i64 - e as i64);
                    }
                }

                // 下钻首步：C 段 sub_moves 中 end_index==anchor 的段（即 C 的末子段）。
                let mut lead_sub = None;
                let mut sub_dir = None;
                if let Some(si) = find_move_by_end_index(cm.sub_moves.as_slice(), anchor) {
                    let sub = &cm.sub_moves[si];
                    sub_dir = rmove_dir(&sub.rmove);
                    if let Some(e) = extreme_bar(bars, sub.start_index, sub.end_index, side) {
                        lead_sub = Some(anchor as i64 - e as i64);
                    }
                }

                if let Some(lc) = lead_c {
                    lead_c_by
                        .entry((lvl, sname))
                        .or_insert_with(Hist::new)
                        .add(lc);
                }
                if let Some(lc) = lead_center {
                    lead_center_by
                        .entry((lvl, sname))
                        .or_insert_with(Hist::new)
                        .add(lc);
                }
                if let Some(ls) = lead_sub {
                    lead_sub_by
                        .entry((lvl, sname))
                        .or_insert_with(Hist::new)
                        .add(ls);
                }
                // 方向分类：顺趋势 = dir == trend_dir（即 −δ）；反趋势 = 相反；None 单列。
                let dclass = match sub_dir {
                    Some(d) if d == trend_dir(side) => "trend",
                    Some(_) => "counter",
                    None => "none",
                };
                *descend_dir_by
                    .entry((lvl, sname))
                    .or_insert_with(BTreeMap::new)
                    .entry(dclass)
                    .or_insert(0) += 1;

                dump_line(&format!(
                    "{{\"lvl\":{lvl},\"side\":\"{sname}\",\"anchor\":{anchor},\"c_start\":{c_start},\"c_end\":{c_end},\"c_dir\":\"{}\",\"ext_c\":{},\"lead_c\":{},\"ctr_start\":{},\"ctr_end\":{},\"lead_center\":{},\"sub_dir\":\"{}\",\"lead_sub\":{}}}",
                    dir_name(cm_dir),
                    ext_c.map(|e| e.to_string()).unwrap_or_else(|| "null".into()),
                    lead_c.map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
                    ctr_start.map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
                    ctr_end.map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
                    lead_center.map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
                    dir_name(sub_dir),
                    lead_sub.map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
                ));
            }
        }
    }
    dump_flush();

    println!("== #1035 一类点提前量普查 ==");
    println!("total type1 points={total}  no_c_move={no_cmove}");
    println!();
    println!("## lead_C = anchor − C 窗口极值 bar（=0 ⟹ 点锚即背驰极值；>0 ⟹ 点锚晚于极值）");
    for ((lvl, sname), h) in &lead_c_by {
        print_hist(&format!("L{lvl} {sname}"), h);
    }
    println!();
    println!("## lead_center = anchor − 中枢窗口极值 bar");
    for ((lvl, sname), h) in &lead_center_by {
        print_hist(&format!("L{lvl} {sname}"), h);
    }
    println!();
    println!("## lead_sub = anchor − C 末子段极值 bar");
    for ((lvl, sname), h) in &lead_sub_by {
        print_hist(&format!("L{lvl} {sname}"), h);
    }
    println!();
    println!("## 下钻首步子段方向（trend=顺趋势 −δ；counter=反趋势 +δ；none=无方向）");
    for ((lvl, sname), m) in &descend_dir_by {
        println!(
            "L{lvl} {sname}: trend={} counter={} none={}",
            m.get("trend").copied().unwrap_or(0),
            m.get("counter").copied().unwrap_or(0),
            m.get("none").copied().unwrap_or(0),
        );
    }

    std::process::ExitCode::SUCCESS
}
