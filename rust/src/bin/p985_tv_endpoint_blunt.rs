//! # `p985_tv_endpoint_blunt` —— G4 裁定二数据义务（#985）：末端钝化场景下 L(Δv) vs TV 两候选判定差集，只读探针
//!
//! **被测命题**：beichi.md v1.6「TV 候选挂起」节——「末端钝化（冲到头已在减速）只有净增量读得出，
//! 峰值/平均/变差读法都读不出」是实证理由非定理；本探针给两端数据：
//! 1. 末端钝化场景采样：趋势末连接段 c，其末单元速度 < 首单元速度（段内减速）的样例集；
//! 2. 两候选判定差集：L(c)<L(b)（净增量判背驰）vs TV(c)<TV(b)（总变差判背驰）在全部可测对与钝化子集上的
//!    一致率、差集计数与差集样例坐标（复跑可查）。
//!
//! **口径**：完全沿用 p977（tower[j−1] 单元速度、相邻中枢闭包间隙连接段、p856 趋势链）——TV 此处取
//! 段内逐单元速度跳变绝对值之和（笔粒度总变差的塔内忠实读法）。只读，不改生产判据。

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::classifier::descend::RMove;
use newchan_rust::theta_v0::classifier::recursive_tower::LeveledMove;
use newchan_rust::theta_v0::classifier::{classify_with_tower, Classification};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{Center, Direction};
use std::rc::Rc;

fn center_of(m: &LeveledMove) -> Option<Center> {
    match &m.rmove {
        RMove::Compose { centers, .. } => centers.first().copied(),
        RMove::Segment { .. } => None,
    }
}

fn link_dir(a: &Center, b: &Center) -> Option<Direction> {
    if b.zd > a.zg { Some(Direction::Up) } else if b.zg < a.zd { Some(Direction::Down) } else { None }
}

fn dir_sign(d: Direction) -> f64 {
    match d { Direction::Up => 1.0, Direction::Down => -1.0 }
}

fn unit_direction(m: &RMove) -> Direction {
    match m {
        RMove::Segment { direction, .. } => *direction,
        RMove::Compose { subs, .. } => subs.first().map(unit_direction).unwrap_or(Direction::Up),
    }
}

fn v_of(m: &LeveledMove) -> f64 {
    let bars = (m.end_index - m.start_index + 1) as f64;
    let range = (m.rmove.hi() - m.rmove.lo()) as f64;
    dir_sign(unit_direction(&m.rmove)) * range / bars
}

fn span_units(units: &[LeveledMove], lo: usize, hi: usize) -> Option<(usize, usize)> {
    let ul = units.partition_point(|u| u.start_index < lo);
    let uh = units.partition_point(|u| u.end_index <= hi);
    if uh > ul { Some((ul, uh - 1)) } else { None }
}

/// L(段) 与 TV(段) 与 末段是否钝化（末单元速度绝对值 < 首单元速度绝对值，段内减速）。
fn l_tv_blunt(units: &[LeveledMove], lo: usize, hi: usize, dir: Direction) -> Option<(f64, f64, bool)> {
    let (f, l) = span_units(units, lo, hi)?;
    let mut tv = 0.0;
    for w in f..l {
        tv += (v_of(&units[w + 1]) - v_of(&units[w])).abs();
    }
    let vf = v_of(&units[f]);
    let vl = v_of(&units[l]);
    // 钝化：沿趋势方向速度衰减（带号速度同向比较）
    let blunt = match dir {
        Direction::Up => vl < vf,
        Direction::Down => vl > vf,
    };
    Some((vl - vf, tv, blunt))
}

fn main() -> std::process::ExitCode {
    let symbol = std::env::args().nth(1).unwrap_or_else(|| "OKLO".to_string());
    let config = ThetaConfig::default();
    let dataset = match load_by_symbol(&symbol, &config) {
        Ok(ds) => ds,
        Err(e) => { eprintln!("数据加载失败: {e}"); return std::process::ExitCode::FAILURE; }
    };
    let bars = &dataset.bars;
    let l0 = parse_layer(bars, &config);
    let (classification, tower): (Classification, Vec<Rc<Vec<LeveledMove>>>) =
        classify_with_tower(&l0, &config);
    println!("P985_INPUT symbol={symbol} bars={}", bars.len());

    let mut n_pairs = 0u64;
    let mut n_blunt_c = 0u64;
    let mut agree_all = 0u64;
    let mut agree_blunt = 0u64;
    let mut l_only_blunt = 0u64;   // 钝化子集里 L 判背驰、TV 不判
    let mut tv_only_blunt = 0u64;
    let mut samples: Vec<String> = Vec::new();

    for j in 1..tower.len() {
        let units: &[LeveledMove] = &tower[j - 1];
        let lv = &tower[j];
        let cs: Vec<Center> = lv.iter().filter_map(center_of).collect();
        if cs.len() != lv.len() || cs.len() < 2 { continue; }
        let mut i = 0usize;
        while i + 1 < cs.len() {
            let Some(d0) = link_dir(&cs[i], &cs[i + 1]) else { i += 1; continue; };
            let mut end = i + 1;
            while end + 1 < cs.len() {
                match link_dir(&cs[end], &cs[end + 1]) { Some(d) if d == d0 => end += 1, _ => break }
            }
            let span = |t: usize| -> Option<(usize, usize)> {
                let a_end = lv[t].end_index;
                let b_start = lv[t + 1].start_index;
                Some((a_end.min(b_start), a_end.max(b_start)))
            };
            for t in i..end {
                if t + 1 >= end { continue; }
                let (Some((blo, bhi)), Some((clo, chi))) = (span(t), span(t + 1)) else { continue; };
                let (Some((lb, tvb, _)), Some((lc, tvc, blunt_c))) =
                    (l_tv_blunt(units, blo, bhi, d0), l_tv_blunt(units, clo, chi, d0)) else { continue; };
                n_pairs += 1;
                let l_div = lc < lb;
                let tv_div = tvc < tvb;
                if l_div == tv_div { agree_all += 1; }
                if blunt_c {
                    n_blunt_c += 1;
                    if l_div == tv_div { agree_blunt += 1; }
                    if l_div && !tv_div {
                        l_only_blunt += 1;
                        if samples.len() < 10 {
                            samples.push(format!(
                                "P985_LONLY level={} dir={:?} b=({blo},{bhi}) L(b)={lb:.3} TV(b)={tvb:.3} c=({clo},{chi}) L(c)={lc:.3} TV(c)={tvc:.3} — L 判背驰、TV 不判（钝化只被 L 读出）",
                                j - 1, d0));
                        }
                    } else if tv_div && !l_div {
                        tv_only_blunt += 1;
                    }
                }
            }
            i = end;
        }
    }
    println!(
        "P985_SUMMARY pairs={n_pairs} blunt_c={n_blunt_c} agree_all={:.4} agree_blunt={:.4} l_only_in_blunt={l_only_blunt} tv_only_in_blunt={tv_only_blunt}",
        agree_all as f64 / n_pairs.max(1) as f64,
        agree_blunt as f64 / n_blunt_c.max(1) as f64,
    );
    for s in &samples { println!("{s}"); }
    let _ = classification.levels.len();
    std::process::ExitCode::SUCCESS
}
