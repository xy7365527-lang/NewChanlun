//! # `p108_interval_probe` —— 区间套收敛探针（任务 #108，只读探针）
//!
//! ## 检验对象（条款 9 前置经验事实）
//! 区间套（缠论第 27 课式收敛）在 classifier 递归塔内的三级经验检验：
//! 1. **同点降级存在性**：lvl k 买卖点是否同 bar 同侧存在 lvl k-1 点（「高级别点
//!    同时是低级别点」——区间套的点身份前提；p107 P107_L0NN 的逐级精化）。
//! 2. **嵌套收缩**：同 (source_index, side) 多级组内，相邻两级的
//!    a) 最后中枢区间 `[center.start_index, center.end_index]`（`P108_NEST_C`）、
//!    b) 离开窗口 `[min(center.end,src), max(center.end,src)]`（`P108_NEST_W`）
//!    是否低级 ⊆ 高级（收缩方向），并归类破例（equal/partial/disjoint/reversed）。
//! 3. **lvl0 极值 bar 收敛**：每级离开窗口内的极值 bar（买=argmin low，卖=argmax high）
//!    到 source_index 的距离 |d| 是否随级别下降而收敛（`P108_CONV`/`P108_MONO`），
//!    lvl0 是否 |d|=0（点即极值）。
//!
//! ## 坐标基（代码事实）
//! `BspPoint.source_index` 与 `Center.start_index/end_index` 均为 L0 原始 K 序坐标
//! （bsp.rs:115 / center.rs:48 UnitRange 文档），同基可直接比较。
//!
//! ## 只读纪律
//! 不动判据、不写主路径状态；输出 `P108_*` 报表行到 stdout。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin p108_interval_probe
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

/// 区间对关系归类（lo=低级区间, hi=高级区间；闭区间）。
fn classify_pair(lo: (usize, usize), hi: (usize, usize)) -> &'static str {
    let (a1, b1) = lo;
    let (a2, b2) = hi;
    if a1 == a2 && b1 == b2 {
        "equal"
    } else if a2 <= a1 && b1 <= b2 {
        "contain" // 低级 ⊂ 高级：收缩方向正确
    } else if a1 <= a2 && b2 <= b1 {
        "reversed" // 高级 ⊂ 低级：方向反了
    } else if b1 < a2 || b2 < a1 {
        "disjoint"
    } else {
        "partial"
    }
}

#[derive(Default, Clone)]
struct NestStat {
    contain: usize,
    equal: usize,
    partial: usize,
    disjoint: usize,
    reversed: usize,
}

impl NestStat {
    fn add(&mut self, kind: &str) {
        match kind {
            "contain" => self.contain += 1,
            "equal" => self.equal += 1,
            "partial" => self.partial += 1,
            "disjoint" => self.disjoint += 1,
            "reversed" => self.reversed += 1,
            _ => unreachable!(),
        }
    }
    fn total(&self) -> usize {
        self.contain + self.equal + self.partial + self.disjoint + self.reversed
    }
}

fn main() -> std::process::ExitCode {
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
    println!("P108_INPUT bars={n}");

    // ── 事件采集：p107 同规则逐 bar 扫描 + (lvl, src, disc) 首见去重。 ──
    let mut classifier = IncrementalClassifier::new(bars, &config);
    let mut seen: HashSet<(usize, usize, u8)> = HashSet::new();
    // (src, side=buy?) -> lvl -> 首见 center 区间
    let mut groups: BTreeMap<(usize, bool), BTreeMap<usize, Option<(usize, usize)>>> =
        BTreeMap::new();
    let mut no_center3 = 0usize; // 3 类 bit 却无 center（判据蕴含应为 0）
    let mut events_total = 0usize;
    for i in 0..n {
        let (classification, _tower) = classifier.classify_at(i);
        for (lvl, ls) in classification.levels.iter().enumerate() {
            for p in ls.bsp.iter() {
                if seen.insert((lvl, p.source_index, bsp_bits_disc(&p.bits))) {
                    let b = &p.bits;
                    let buy = b.buy1 || b.buy2 || b.buy3;
                    let sell = b.sell1 || b.sell2 || b.sell3;
                    if !(buy || sell) {
                        continue;
                    }
                    let center = p.center.map(|c| (c.start_index, c.end_index));
                    if (b.buy3 || b.sell3) && center.is_none() {
                        no_center3 += 1;
                    }
                    events_total += 1;
                    if buy {
                        groups
                            .entry((p.source_index, true))
                            .or_default()
                            .entry(lvl)
                            .or_insert(center);
                    }
                    if sell {
                        groups
                            .entry((p.source_index, false))
                            .or_default()
                            .entry(lvl)
                            .or_insert(center);
                    }
                }
            }
        }
        if i % 500_000 == 0 && i > 0 {
            eprintln!("P108_PROGRESS bar={i}/{n} groups={}", groups.len());
        }
    }

    let max_lvl = groups
        .values()
        .flat_map(|m| m.keys().copied())
        .max()
        .unwrap_or(0);
    let mut evt_lvl = vec![0usize; max_lvl + 1];
    let mut multi_groups = 0usize;
    for m in groups.values() {
        for &l in m.keys() {
            evt_lvl[l] += 1;
        }
        if m.len() >= 2 {
            multi_groups += 1;
        }
    }
    for (l, cnt) in evt_lvl.iter().enumerate() {
        println!("P108_EVT lvl={l} points={cnt}");
    }
    println!(
        "P108_EVT groups_total={} multi_lvl_groups={multi_groups} events_raw={events_total} no_center3={no_center3}",
        groups.len()
    );

    // ── 检验 1：同点降级存在性（lvl k 点是否有 lvl k-1 同点同侧点）。 ──
    let mut have_lower = vec![0usize; max_lvl + 1];
    let mut miss_lower = vec![0usize; max_lvl + 1];
    for m in groups.values() {
        for &l in m.keys() {
            if l >= 1 {
                if m.contains_key(&(l - 1)) {
                    have_lower[l] += 1;
                } else {
                    miss_lower[l] += 1;
                }
            }
        }
    }
    for l in 1..=max_lvl {
        let tot = have_lower[l] + miss_lower[l];
        if tot == 0 {
            continue;
        }
        println!(
            "P108_LOWER lvl={l} have={} miss={} miss_pct={:.2}",
            have_lower[l],
            miss_lower[l],
            miss_lower[l] as f64 * 100.0 / tot as f64
        );
    }

    // ── 检验 2：嵌套收缩（相邻在场级对；center 区间 + 离开窗口）。 ──
    // 检验 3：极值 bar 收敛（逐级窗口极值距离 + 组内单调性）。
    let mut cstat: Vec<NestStat> = vec![NestStat::default(); max_lvl + 1]; // 按高级 lvl 归账
    let mut wstat: Vec<NestStat> = vec![NestStat::default(); max_lvl + 1];
    let mut pair_nocenter = vec![0usize; max_lvl + 1];
    // conv 桶：exact0 / ≤10 / ≤60 / ≤240 / ≤1440 / far
    let mut conv = vec![[0usize; 6]; max_lvl + 1];
    let mut conv_pts = vec![0usize; max_lvl + 1];
    let mut mono_multi = 0usize;
    let mut mono_ok = 0usize;
    for ((src, buy), m) in &groups {
        let src = *src;
        // 逐级窗口极值距离。
        let mut dists: Vec<(usize, u64)> = Vec::new(); // (lvl, |d|)
        for (&l, c) in m.iter() {
            let Some((_cs, ce)) = c else { continue };
            let (w0, w1) = (src.min(*ce), src.max(*ce));
            let w1 = w1.min(n - 1);
            let mut ext = w0;
            for j in w0..=w1 {
                if *buy {
                    if bars[j].low < bars[ext].low {
                        ext = j;
                    }
                } else if bars[j].high > bars[ext].high {
                    ext = j;
                }
            }
            let d = (ext as i64 - src as i64).unsigned_abs();
            conv_pts[l] += 1;
            let bucket = if d == 0 {
                0
            } else if d <= 10 {
                1
            } else if d <= 60 {
                2
            } else if d <= 240 {
                3
            } else if d <= 1440 {
                4
            } else {
                5
            };
            conv[l][bucket] += 1;
            dists.push((l, d));
        }
        if dists.len() >= 2 {
            mono_multi += 1;
            // 按 lvl 升序：|d| 应随 lvl 上升而非降（低级更贴近极值）。
            let ok = dists.windows(2).all(|w| w[0].1 <= w[1].1);
            if ok {
                mono_ok += 1;
            }
        }
        // 相邻在场级对（组内实际出现的级别序列）。
        let lvls: Vec<usize> = m.keys().copied().collect();
        for w in lvls.windows(2) {
            let (lo, hi) = (w[0], w[1]);
            let (clo, chi) = (m[&lo], m[&hi]);
            let (Some(clo), Some(chi)) = (clo, chi) else {
                pair_nocenter[hi] += 1;
                continue;
            };
            cstat[hi].add(classify_pair(clo, chi));
            let wlo = (src.min(clo.1), src.max(clo.1));
            let whi = (src.min(chi.1), src.max(chi.1));
            wstat[hi].add(classify_pair(wlo, whi));
        }
    }
    for l in 1..=max_lvl {
        let cs = &cstat[l];
        if cs.total() + pair_nocenter[l] == 0 {
            continue;
        }
        println!(
            "P108_NEST_C hi_lvl={l} contain={} equal={} partial={} disjoint={} reversed={} nocenter={} contain_pct={:.2}",
            cs.contain, cs.equal, cs.partial, cs.disjoint, cs.reversed, pair_nocenter[l],
            if cs.total() > 0 { (cs.contain + cs.equal) as f64 * 100.0 / cs.total() as f64 } else { 0.0 }
        );
        let ws = &wstat[l];
        println!(
            "P108_NEST_W hi_lvl={l} contain={} equal={} partial={} disjoint={} reversed={} contain_pct={:.2}",
            ws.contain, ws.equal, ws.partial, ws.disjoint, ws.reversed,
            if ws.total() > 0 { (ws.contain + ws.equal) as f64 * 100.0 / ws.total() as f64 } else { 0.0 }
        );
    }
    for l in 0..=max_lvl {
        if conv_pts[l] == 0 {
            continue;
        }
        let c = &conv[l];
        println!(
            "P108_CONV lvl={l} pts={} exact0={} le10={} le60={} le240={} le1440={} far={} exact0_pct={:.2}",
            conv_pts[l], c[0], c[1], c[2], c[3], c[4], c[5],
            c[0] as f64 * 100.0 / conv_pts[l] as f64
        );
    }
    println!(
        "P108_MONO multi_groups={mono_multi} mono_ok={mono_ok} mono_pct={:.2}",
        if mono_multi > 0 {
            mono_ok as f64 * 100.0 / mono_multi as f64
        } else {
            0.0
        }
    );
    std::process::ExitCode::SUCCESS
}
