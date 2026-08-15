//! # `p977_force_def_validation` —— T2（map #974）：#873 力度定义三个未验证欠账的实测，只读探针
//!
//! ## 被测对象（`.chanlun/definitions/beichi.md`「★力度的精确定义」，#873 裁定）
//!
//! > `L(段) = v(段末) − v(段初)`，速度在结构最小单元上量：这一单元涨了多少 ÷ 用了几根 K 线。
//!
//! 三部分：
//! - **A. 推理链第 3 步近似误差**：唯一近似步 = 同色柱端点 `H≈0 ⟹ DIF≈DEA`。实测每个同色 run
//!   端点的 `|ΔDIF − ΔDEA| / |ΔDEA|`（推理链把 ΔDEA 换成 ΔDIF 用的就是这个近似）。
//! - **B. 判定一致性**：同趋势相邻连接段对 (b,c)，新定义 `L(c)<L(b)` vs MACD 面积 `area(c)<area(b)`。
//!   MACD 面积双臂：`abs`（生产 `segment_macd_area` = |hist| 之和）/ `same_color`（#873 口径：
//!   按 d0 同色带号 hist 之和）。两臂各自独立出数，不互相兜底（#799）。
//! - **C. μ(D) 分布与绕父两险**：完整趋势 `μ(D)=v(D末)−v(D初)` 的实测分布；绕父除法的两个危险
//!   （分母≈0 / 分母<0）在真实数据里的频率。
//!
//! ## 口径选择（票面硬纪律：每个选择明写）
//!
//! 1. **速度的原子 = tower[j−1] 走势单元**（`LeveledMove`，`v = sign(dir)·(hi−lo)/(end−start+1)`，
//!    Tick/bar）。定义原文说「笔」；塔内最小结构单元是 L0 线段（递归底），j≥1 的段其最小单元是
//!    次级别走势——这是**塔内忠实读法**，与原文「笔」的差（线段 vs 笔）明写不隐瞒，G1 裁载体时收。
//! 2. **连接段 = 相邻中枢闭包间隙**（p856 同款 `closed` 口径：`lv[t].end_index..=lv[t+1].start_index`
//!    的 min/max 闭区间），保证非空。
//! 3. **趋势链 = p856 同款**：tower[j] 相邻中枢 ZD/ZG 不重叠同向连续段。
//! 4. **MACD** = 生产 `compute_macd`（hist = DIF − DEA，**无 ×2**；beichi.md 推理链按中式
//!    `hist=2(DIF−DEA)` 得「柱面积=8×ΔDEA」——系数差异只影响绝对值，A 部分测的是**相对误差**，
//!    不受影响；该系数差本身作为观察上报 G1/G2）。
//!
//! **只读**：不改任何生产判据，只调生产入口 `parse_layer` + `classify_with_tower` + `compute_macd`。

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::classifier::descend::RMove;
use newchan_rust::theta_v0::classifier::divergence::{compute_macd, segment_macd_area};
use newchan_rust::theta_v0::classifier::recursive_tower::LeveledMove;
use newchan_rust::theta_v0::classifier::{classify_with_tower, Classification};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{Center, Direction};
use std::collections::BTreeMap;
use std::rc::Rc;

fn center_of(m: &LeveledMove) -> Option<Center> {
    match &m.rmove {
        RMove::Compose { centers, .. } => centers.first().copied(),
        RMove::Segment { .. } => None,
    }
}

/// 相邻两中枢的趋势方向（p856 同款：核心口径 ZD/ZG）。
fn link_dir(a: &Center, b: &Center) -> Option<Direction> {
    if b.zd > a.zg {
        Some(Direction::Up)
    } else if b.zg < a.zd {
        Some(Direction::Down)
    } else {
        None
    }
}

fn dir_sign(d: Direction) -> f64 {
    match d {
        Direction::Up => 1.0,
        Direction::Down => -1.0,
    }
}

/// 单元速度：sign(dir)·(hi−lo)/(bar 数)。Compose 的方向由「hi 与 lo 谁先出现」不可得，
/// 改由次级别走势序列首元素方向递归取（Segment 直接有）。
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

/// 闭包间隙内的次级别单元区间（首/末），无单元 ⟹ None。
fn span_units(units: &[LeveledMove], lo: usize, hi: usize) -> Option<(usize, usize)> {
    let ul = units.partition_point(|u| u.start_index < lo);
    let uh = units.partition_point(|u| u.end_index <= hi);
    if uh > ul {
        Some((ul, uh - 1))
    } else {
        None
    }
}

/// L(段) = v(末单元) − v(首单元)。
fn l_of(units: &[LeveledMove], lo: usize, hi: usize) -> Option<f64> {
    let (first, last) = span_units(units, lo, hi)?;
    Some(v_of(&units[last]) - v_of(&units[first]))
}

/// 同色面积：段内 sign(hist)==sign(dir) 的 hist 之和（#873 口径）。
fn same_color_area(hist: &[f64], lo: usize, hi: usize, dir: Direction) -> f64 {
    let s = dir_sign(dir);
    let mut acc = 0.0;
    for &h in &hist[lo..=hi.min(hist.len() - 1)] {
        if h * s > 0.0 {
            acc += h;
        }
    }
    acc.abs()
}

fn pct(v: &[f64], p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let i = ((s.len() as f64 - 1.0) * p).round() as usize;
    s[i.min(s.len() - 1)]
}

fn main() -> std::process::ExitCode {
    let symbol = std::env::args().nth(1).unwrap_or_else(|| "BTC".to_string());
    let config = ThetaConfig::default();
    let dataset = match load_by_symbol(&symbol, &config) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("数据加载失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let bars = &dataset.bars;
    let n_bars = bars.len();
    let closes: Vec<f64> = bars.iter().map(|b| b.close as f64).collect();
    let l0 = parse_layer(bars, &config);
    let (classification, tower): (Classification, Vec<Rc<Vec<LeveledMove>>>) =
        classify_with_tower(&l0, &config);
    let series = compute_macd(&closes, &config.macd);
    let (dif, dea, hist) = (&series.dif, &series.dea, &series.hist);
    println!(
        "P977_INPUT symbol={symbol} bars={n_bars} closes={closes_len} tower_levels={tl}",
        closes_len = closes.len(),
        tl = tower.len()
    );

    // ── A. 第 3 步近似误差：同色 run 端点 DIF≈DEA ─────────────────────────
    let mut run_start = 0usize;
    let mut rel_errs: Vec<f64> = Vec::new();
    let mut endpoint_h_rel_peak: Vec<f64> = Vec::new();
    let mut runs = 0u64;
    for i in 1..=hist.len() {
        let boundary =
            i == hist.len() || (hist[i].signum() != hist[run_start].signum() && hist[i] != 0.0);
        if !boundary {
            continue;
        }
        let (s, e) = (run_start, i - 1);
        run_start = i;
        if e <= s || hist[s] == 0.0 {
            continue;
        }
        runs += 1;
        let d_dea = dea[e] - dea[s];
        let d_dif = dif[e] - dif[s];
        if d_dea.abs() > 1e-12 {
            rel_errs.push(((d_dif - d_dea) / d_dea).abs());
        }
        let peak = hist[s..=e].iter().fold(0f64, |a, x| a.max(x.abs()));
        if peak > 0.0 {
            endpoint_h_rel_peak.push((hist[s].abs().max(hist[e].abs())) / peak);
        }
    }
    println!(
        "P977_A runs={runs} relerr_n={n} relerr_median={:.6} relerr_p90={:.6} relerr_max={:.6} endpointH_over_peak_median={:.6}",
        pct(&rel_errs, 0.5),
        pct(&rel_errs, 0.9),
        rel_errs.iter().cloned().fold(0f64, f64::max),
        pct(&endpoint_h_rel_peak, 0.5),
        n = rel_errs.len(),
    );

    // ── B+C. 塔行走：趋势链 → 连接段对 ────────────────────────────────────
    struct Agree {
        n: u64,
        both_div: u64,
        both_not: u64,
        new_only: u64,
        macd_only: u64,
    }
    let mut agree_abs = BTreeMap::<usize, Agree>::new();
    let mut agree_sc = BTreeMap::<usize, Agree>::new();
    let mut disagree_samples: Vec<String> = Vec::new();
    let mut mu_d: Vec<f64> = Vec::new();
    let mut mu_d_by_level: BTreeMap<usize, Vec<f64>> = BTreeMap::new();
    let mut n_trends = 0u64;

    for j in 1..tower.len() {
        let units: &[LeveledMove] = &tower[j - 1];
        let lv = &tower[j];
        let cs: Vec<Center> = lv.iter().filter_map(center_of).collect();
        if cs.len() != lv.len() || cs.len() < 2 {
            continue;
        }
        let mut i = 0usize;
        while i + 1 < cs.len() {
            let Some(d0) = link_dir(&cs[i], &cs[i + 1]) else {
                i += 1;
                continue;
            };
            let mut end = i + 1;
            while end + 1 < cs.len() {
                match link_dir(&cs[end], &cs[end + 1]) {
                    Some(d) if d == d0 => end += 1,
                    _ => break,
                }
            }
            n_trends += 1;

            // C. μ(D)：趋势整跨（首中枢起点 .. 末中枢终点）
            let d_lo = lv[i].start_index;
            let d_hi = lv[end].end_index;
            if let Some((f, l)) = span_units(units, d_lo, d_hi) {
                let mu = v_of(&units[l]) - v_of(&units[f]);
                mu_d.push(mu);
                mu_d_by_level.entry(j - 1).or_default().push(mu);
            }

            // B. 相邻连接段对 (b=span(t), c=span(t+1))
            let span = |t: usize| -> Option<(usize, usize)> {
                let a_end = lv[t].end_index;
                let b_start = lv[t + 1].start_index;
                Some((a_end.min(b_start), a_end.max(b_start)))
            };
            for t in i..end {
                if t + 1 >= end {
                    continue; // 连接段 t+1 需要中枢 t+2（p856 同款 t+1<end 守卫）
                }
                let (Some((blo, bhi)), Some((clo, chi))) = (span(t), span(t + 1))
                else {
                    continue;
                };
                let (Some(lb), Some(lc)) = (l_of(units, blo, bhi), l_of(units, clo, chi))
                else {
                    continue;
                };
                let new_judge = lc < lb;
                let ab = segment_macd_area(hist, blo, bhi);
                let ac = segment_macd_area(hist, clo, chi);
                let macd_judge_abs = ac < ab;
                let sc_b = same_color_area(hist, blo, bhi, d0);
                let sc_c = same_color_area(hist, clo, chi, d0);
                let macd_judge_sc = sc_c < sc_b;
                for (_arm, judge, bucket) in [
                    (true, macd_judge_abs, &mut agree_abs),
                    (false, macd_judge_sc, &mut agree_sc),
                ] {
                    let a = bucket.entry(j - 1).or_insert(Agree {
                        n: 0,
                        both_div: 0,
                        both_not: 0,
                        new_only: 0,
                        macd_only: 0,
                    });
                    a.n += 1;
                    match (new_judge, judge) {
                        (true, true) => a.both_div += 1,
                        (false, false) => a.both_not += 1,
                        (true, false) => a.macd_only += 1,
                        (false, true) => a.new_only += 1,
                    }
                }
                if new_judge != macd_judge_abs && disagree_samples.len() < 12 {
                    disagree_samples.push(format!(
                        "P977_B_DISAGREE level={} dir={:?} b=({blo},{bhi}) L(b)={lb:.4} area(b)={ab:.2} c=({clo},{chi}) L(c)={lc:.4} area(c)={ac:.2} new=new_div macd=not_div"
                    , j - 1, d0));
                }
            }
            i = end;
        }
    }

    println!("P977_TRENDS n={n_trends}");
    for (lvl, a) in &agree_abs {
        let sc = &agree_sc[lvl];
        println!(
            "P977_B level={lvl} n={n} agree_abs={:.4} (both_div={bd} both_not={bn} new_only={no} macd_only={mo}) agree_samecolor={:.4} (both_div={sbd} both_not={sbn} new_only={sno} macd_only={smo})",
            (a.both_div + a.both_not) as f64 / a.n as f64,
            (sc.both_div + sc.both_not) as f64 / sc.n as f64,
            n = a.n,
            bd = a.both_div,
            bn = a.both_not,
            no = a.new_only,
            mo = a.macd_only,
            sbd = sc.both_div,
            sbn = sc.both_not,
            sno = sc.new_only,
            smo = sc.macd_only,
        );
    }
    for s in &disagree_samples {
        println!("{s}");
    }

    // ── C. μ(D) 汇总 ───────────────────────────────────────────────────────
    let med_v = {
        let mut vs: Vec<f64> = Vec::new();
        for j in 1..tower.len() {
            for u in tower[j - 1].iter() {
                vs.push(v_of(u).abs());
            }
        }
        pct(&vs, 0.5)
    };
    let neg = mu_d.iter().filter(|&&x| x < 0.0).count();
    let tiny = mu_d.iter().filter(|&&x| x.abs() < 0.05 * med_v).count();
    println!(
        "P977_C n={n} mu_median={:.6} mu_p10={:.6} mu_p90={:.6} neg_frac={:.4} tiny_frac={:.4} (tiny=|mu|<0.05*median|v|, median|v|={med_v:.6})",
        pct(&mu_d, 0.5),
        pct(&mu_d, 0.1),
        pct(&mu_d, 0.9),
        neg as f64 / mu_d.len().max(1) as f64,
        tiny as f64 / mu_d.len().max(1) as f64,
        n = mu_d.len(),
    );
    for (lvl, v) in &mu_d_by_level {
        println!(
            "P977_C_LEVEL level={lvl} n={n} neg={neg} median={:.6}",
            pct(v, 0.5),
            n = v.len(),
            neg = v.iter().filter(|&&x| x < 0.0).count(),
        );
    }
    let _ = &classification.levels.len();
    std::process::ExitCode::SUCCESS
}
