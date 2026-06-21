//! **递归 T 流式回测器**：真实 OHLC 流 → iterate 真树 → extract_chain → RecDriver.on_view。
//!
//! 与 flat `stream::TFugueStreamCore`（驱动 TPositionEngine）并存——本模块驱动递归 `RecDriver`：
//! 复用同一 orchestrator（产笔/段）+ 段门控重跑 + online MACD 前缀和（口径逐字一致，segments
//! bit-exact），但重跑产物经 `extract_chain` 投影为操作链视图，喂递归 driver（每级别一 T 实例树）。
//!
//! ## 因果性（无 look-ahead）
//! BSP/走势完成直到**段确认那根 bar** 才被算出/投放，`on_view` 在确认时点 close 执行 reconcile
//! ⟹ 因果（除不可消除的确认滞后本身）。与 flat 流式同口径。
//!
//! ## 认识论等级
//! 流式接入 = **L1**（管线正确性，与 flat 共享 orch/MACD/段门控）；回测 alpha + 节点身份跨重跑
//! 稳定性（§8.6 前缀冻结/N4）= **L2/L3 待验证**——这正是本回测器要回答的。

use crate::macd::OnlineMacdState;
use crate::orchestrator::RecursiveOrchestrator;
use crate::segment::SegKind;
use crate::trading::types::INITIAL_CAPITAL;

use super::iterate;
use super::rec_driver::{extract_chain, RecDriver};
use super::stream::build_a0_fast;
use super::types::PerfectionMode;

/// 诊断：BSPKind → 索引（0=t1buy 1=t1sell 2=t2buy 3=t2sell 4=t3buy 5=t3sell）。
fn bsp_kind_idx(k: crate::recursive_t::types::BSPKind) -> u8 {
    use crate::recursive_t::types::BSPKind::*;
    match k {
        Type1Buy => 0,
        Type1Sell => 1,
        Type2Buy => 2,
        Type2Sell => 3,
        Type3Buy => 4,
        Type3Sell => 5,
    }
}

/// 递归 T 流式核心（orchestrator + 重跑 + extract_chain + RecDriver，全 Rust 内聚）。
pub struct RecStream {
    orch: RecursiveOrchestrator,
    mode: PerfectionMode,
    driver: RecDriver,
    macd: OnlineMacdState,
    prefix_pos: Vec<f64>,
    prefix_neg: Vec<f64>,
    cur_bar: i64,
    last_close: f64,
    finished: bool,
    last_stroke_n: usize,
    last_cs_segs: usize,
    /// 重跑次数（性能 + 诊断）。
    pub n_reruns: u64,
    /// 诊断：chain[0]（最高级别走势）方向 bar 加权分布——定位 82% 持空根因（extract_chain vs 操作）。
    last_c0: i8, // 1=up, -1=down, 0=none
    pub c0_up_bars: u64,
    pub c0_down_bars: u64,
    /// 诊断：核心(root)方向 + 净敞口符号 bar 加权（区分核心方向错 vs sink/exposure 主导）。
    pub core_long_bars: u64,
    pub core_short_bars: u64,
    pub net_long_bars: u64,
    pub net_short_bars: u64,
    /// 诊断（编排者回补质询）：全程 type1_buy 买点 (bar, price, level) dedup；
    /// 核心短头 episodes (entry_bar, entry_px, exit_bar, exit_px)——查做空后下跌段有无买点、是否被消费。
    pub t1buy: Vec<(i64, f64, usize)>,
    pub t1sell: Vec<(i64, f64, usize)>,
    pub short_episodes: Vec<(i64, f64, i64, f64)>,
    cur_short_entry: Option<(i64, f64)>,
    prev_core_dir: i8,
    /// 诊断：全 6 类 BSP 产出计数（dedup by bar+level+kind）+ 按级别分布——查引擎消费/未消费哪些。
    /// 索引: 0=t1buy 1=t1sell 2=t2buy 3=t2sell 4=t3buy 5=t3sell。
    pub bsp_counts: [u64; 6],
    pub bsp_by_level: [[u64; 6]; 10], // [level][kind]
    bsp_seen: std::collections::HashSet<(i64, usize, u8)>,
}

impl RecStream {
    pub fn new(mode: PerfectionMode) -> Self {
        RecStream {
            // orch 配置与 flat stream / backtest_run 逐字一致（segments bit-exact）。
            orch: RecursiveOrchestrator::new(6, "wide", 5, false, 3, false, false, false),
            mode,
            driver: RecDriver::new(INITIAL_CAPITAL),
            macd: OnlineMacdState::new(12, 26, 9),
            prefix_pos: vec![0.0],
            prefix_neg: vec![0.0],
            cur_bar: 0,
            last_close: f64::NAN,
            finished: false,
            last_stroke_n: 0,
            last_cs_segs: 0,
            n_reruns: 0,
            last_c0: 0,
            c0_up_bars: 0,
            c0_down_bars: 0,
            core_long_bars: 0,
            core_short_bars: 0,
            net_long_bars: 0,
            net_short_bars: 0,
            t1buy: Vec::new(),
            t1sell: Vec::new(),
            short_episodes: Vec::new(),
            cur_short_entry: None,
            prev_core_dir: 0,
            bsp_counts: [0; 6],
            bsp_by_level: [[0; 6]; 10],
            bsp_seen: std::collections::HashSet::new(),
        }
    }

    /// 诊断脚手架（编排者 panic 根因质询）：§8.1 free 不足兜底（同金额回补可用部分）跑完全程统计，
    /// 非 §8.1 修复。用于查多重赋格下 free 不足的特征（盈利 vs 亏损短差），区分多级别耦合 vs C3。
    pub fn set_diag_no_panic(&mut self) {
        self.driver.root_mut().diag_no_panic = true;
    }

    /// 逐 bar 推送。段门控触发重跑 → extract_chain → driver.on_view（确认时点 close）。
    pub fn push_bar(&mut self, o: f64, h: f64, l: f64, c: f64) {
        self.orch.process_bar(o, h, l, c);
        // 增量 MACD + pos/neg 前缀和（区间面积 O(1) 查表基础，与 flat 同源）。
        let (_, _, hist) = self.macd.update(c);
        let lp = *self.prefix_pos.last().unwrap();
        let ln = *self.prefix_neg.last().unwrap();
        self.prefix_pos.push(lp + if hist > 0.0 { hist } else { 0.0 });
        self.prefix_neg.push(ln + if hist < 0.0 { -hist } else { 0.0 });
        let bar = self.cur_bar;
        self.last_close = c;

        // 段门控：笔增长 → 数 confirmed&&settled 段 → count 变则重跑。
        let sc = self.orch.strokes().len();
        if sc > self.last_stroke_n {
            self.last_stroke_n = sc;
            let n_cs = self
                .orch
                .segments()
                .iter()
                .filter(|s| s.confirmed && s.kind == SegKind::Settled)
                .count();
            if n_cs != self.last_cs_segs {
                self.last_cs_segs = n_cs;
                // 块内借 orch（segs+m2r）→ build_a0 → iterate → extract_chain（owned，块后释放借用）。
                // 诊断：同时捕获本树全 6 类 BSP（回补质询：查产出/消费）。
                let (view, new_bsps) = {
                    let segs = self.orch.segments();
                    let m2r = self.orch.merged_to_raw();
                    let a0 = build_a0_fast(segs, m2r, &self.prefix_pos, &self.prefix_neg);
                    let tree = iterate(a0, self.mode);
                    let bs: Vec<(i64, f64, usize, u8)> = tree
                        .all_bsps()
                        .into_iter()
                        .map(|b| (b.bar, b.price, b.level, bsp_kind_idx(b.kind)))
                        .collect();
                    (extract_chain(&tree), bs)
                };
                for (bb, bp, bl, bk) in new_bsps {
                    if self.bsp_seen.insert((bb, bl, bk)) {
                        self.bsp_counts[bk as usize] += 1;
                        if bl < 10 {
                            self.bsp_by_level[bl][bk as usize] += 1;
                        }
                        if bk == 0 {
                            self.t1buy.push((bb, bp, bl)); // type1_buy → episode 用
                        } else if bk == 1 {
                            self.t1sell.push((bb, bp, bl)); // type1_sell → 做空卖点诊断
                        }
                    }
                }
                self.n_reruns += 1;
                // 诊断：记录本次 chain[0]（最高级别走势）方向。
                self.last_c0 = match view.nodes.first().map(|n| n.node.direction) {
                    Some(crate::recursive_t::types::Direction::Up) => 1,
                    Some(crate::recursive_t::types::Direction::Down) => -1,
                    None => 0,
                };
                self.driver.on_view(&view, c, bar);
            }
        }
        // bar 加权 chain[0] 方向（持续到下次重跑）。
        match self.last_c0 {
            1 => self.c0_up_bars += 1,
            -1 => self.c0_down_bars += 1,
            _ => {}
        }
        // bar 加权核心(root)方向 + 净敞口符号 + 核心短头 episode 追踪。
        let root = self.driver.root();
        let cur_dir: i8 = match root.root_slot() {
            Some(rs) => match root.instance(rs).direction {
                crate::trading::types::Polarity::Long => {
                    self.core_long_bars += 1;
                    1
                }
                crate::trading::types::Polarity::Short => {
                    self.core_short_bars += 1;
                    -1
                }
            },
            None => 0,
        };
        let (lu, su) = root.exposure();
        if lu - su > 1e-9 {
            self.net_long_bars += 1;
        } else if su - lu > 1e-9 {
            self.net_short_bars += 1;
        }
        // 核心短头 episode：进空记 entry，离空记 exit（查做空后下跌段买点产出/消费）。
        if cur_dir == -1 && self.prev_core_dir != -1 {
            self.cur_short_entry = Some((bar, c));
        } else if cur_dir != -1 && self.prev_core_dir == -1 {
            if let Some((eb, ep)) = self.cur_short_entry.take() {
                self.short_episodes.push((eb, ep, bar, c));
            }
        }
        self.prev_core_dir = cur_dir;
        self.driver.root_mut().update_lows(c); // 诊断：逐 bar 更新活跃短差腿持仓极值（查开得晚/平得晚）
        self.cur_bar += 1;
    }

    /// 收尾（全平到现金 + 归还 withdrawn）。返回 final_nav。幂等。
    pub fn finish(&mut self) -> f64 {
        if self.finished {
            return self.driver.root().free();
        }
        self.finished = true;
        let c = if self.last_close.is_finite() { self.last_close } else { 0.0 };
        self.driver.finish(c)
    }

    pub fn final_nav(&self) -> f64 {
        self.driver.root().free()
    }
    pub fn driver(&self) -> &RecDriver {
        &self.driver
    }
    /// 快照: (cur_bar, total_wealth, n_active_instances)。
    pub fn snapshot(&self) -> (i64, f64, usize) {
        (self.cur_bar, self.driver.root().total_wealth(self.last_close), self.driver.root().n_active())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 递归流式合成序列零panic() {
        // 锯齿上行→顶→下行：流式喂应零 panic（TW 守恒守卫每操作）+ 产出有限 final_nav。
        let mut s = RecStream::new(PerfectionMode::Structural);
        let mut price = 100.0;
        let mut up = true;
        for i in 0..400 {
            if i % 20 == 0 {
                up = !up;
            }
            price += if up { 1.0 } else { -0.8 };
            let c = price;
            s.push_bar(c - 0.1, c + 0.5, c - 0.5, c);
        }
        let fin = s.finish();
        assert!(fin.is_finite() && fin > 0.0, "final_nav 有限正，得 {fin}");
    }

    #[test]
    fn 递归流式零bar不panic() {
        let mut s = RecStream::new(PerfectionMode::And);
        assert!((s.finish() - INITIAL_CAPITAL).abs() < 1e-6);
    }

    /// 递归引擎 **8 标的 × 3 模式**回测（L3 验证）：vs flat 引擎（t_backtest_8x3）vs BH。
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::rec_btc -- --ignored --nocapture`
    /// `BT_SYMBOLS=CL,BTC` 过滤标的（默认全 8）。
    #[test]
    #[ignore = "递归引擎全量回测，需 analysis/data_cache/*.json"]
    fn rec_btc() {
        use crate::recursive_t::backtest_run::{load_clean_ohlc, SYMBOLS};
        use std::path::PathBuf;

        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("analysis/data_cache");
        let only: Vec<String> = std::env::var("BT_SYMBOLS")
            .ok()
            .map(|s| s.split(',').map(|x| x.trim().to_uppercase()).filter(|x| !x.is_empty()).collect())
            .unwrap_or_default();

        eprintln!("\n========== 递归 T 引擎 8 标的 × 3 模式回测（区间套递归归还修复后）==========");
        // (sym, bh, strat[3], short_pnl[3], net_short%[3], sink[3], freeshort[3])
        type Row = (String, f64, [f64; 3], [f64; 3], [f64; 3], [u64; 3], [u64; 3]);
        let mut rows: Vec<Row> = Vec::new();

        for (sym, file) in SYMBOLS {
            if !only.is_empty() && !only.contains(&sym.to_uppercase()) {
                continue;
            }
            let path = data_dir.join(file);
            if !path.exists() {
                eprintln!("[{sym}] 数据缺失 {path:?}，跳过");
                continue;
            }
            let (o, h, l, c) = load_clean_ohlc(&path);
            let n = c.len();
            let bh = if n > 0 && c[0] > 0.0 { (c[n - 1] / c[0] - 1.0) * 100.0 } else { 0.0 };
            let (mut strat, mut spnl, mut nshort) = ([0.0; 3], [0.0; 3], [0.0; 3]);
            let (mut sinks, mut fshort) = ([0u64; 3], [0u64; 3]);
            for (mi, mode) in [PerfectionMode::Structural, PerfectionMode::And, PerfectionMode::Or]
                .iter()
                .enumerate()
            {
                let t0 = std::time::Instant::now();
                let mut s = RecStream::new(*mode);
                s.set_diag_no_panic(); // §8.1 free 不足兜底（同金额回补）跑完全程——C3 亏损短差处理
                for i in 0..n {
                    s.push_bar(o[i], h[i], l[i], c[i]);
                }
                let fin = s.finish();
                strat[mi] = (fin / INITIAL_CAPITAL - 1.0) * 100.0;
                let r = s.driver().root();
                spnl[mi] = r.short_leg_pnl;
                nshort[mi] = 100.0 * s.net_short_bars as f64 / n.max(1) as f64;
                sinks[mi] = r.n_sinks;
                fshort[mi] = r.n_freeshort;
                eprintln!(
                    "[{sym:<5}/{:>10?}] strat={:+.1}% bh={:+.1}% sink={} recover={} short_pnl={:+.0} \
                     net_short={:.1}% §8.1={} ({:.1}s)",
                    mode,
                    strat[mi],
                    bh,
                    r.n_sinks,
                    r.n_recovers,
                    r.short_leg_pnl,
                    nshort[mi],
                    r.n_freeshort,
                    t0.elapsed().as_secs_f64()
                );
                // ── per-level 短差 P&L（编排者 G1：低级别是否摩擦地板下噪声亏损）──
                if sym == "BTC" {
                    let log = &s.driver().root().sink_recover_log;
                    let (mut nl, mut pl, mut al) = ([0u64; 8], [0.0f64; 8], [0.0f64; 8]);
                    for &(lvl, c1, c2, _low, realized) in log.iter() {
                        if lvl < 8 {
                            nl[lvl] += 1;
                            pl[lvl] += realized;
                            if c1 > 1e-9 {
                                al[lvl] += ((c1 - c2) / c1).abs() * 100.0; // 相对振幅%（对比摩擦地板~1-2bps）
                            }
                        }
                    }
                    eprintln!("  [{sym}/{mode:?}] per-level短差 lvl(n/Σpnl/avg每笔/avg相对振幅%):");
                    for lvl in 0..6 {
                        if nl[lvl] > 0 {
                            eprintln!(
                                "    L{lvl}: n={} Σpnl={:+.0} avg={:+.3} amp={:.4}%",
                                nl[lvl],
                                pl[lvl],
                                pl[lvl] / nl[lvl] as f64,
                                al[lvl] / nl[lvl] as f64
                            );
                        }
                    }
                }
                assert!(fin.is_finite(), "[{sym}] final_nav 有限（NaN/Inf=会计 bug）");
            }
            rows.push((sym.to_string(), bh, strat, spnl, nshort, sinks, fshort));
        }

        // ── 汇总矩阵 ──
        println!("\n===== 递归 T 8×3 strat_pct 矩阵（区间套递归归还修复后）=====");
        println!("{:<6} {:>12} {:>12} {:>12} {:>11}", "标的", "Structural", "AND", "OR", "BH");
        for (sym, bh, strat, _, _, _, _) in &rows {
            let mark = |x: f64| if x > *bh { "*" } else { " " }; // * = 超 BH
            println!(
                "{:<6} {:>+10.1}%{} {:>+10.1}%{} {:>+10.1}%{} {:>+10.1}%",
                sym,
                strat[0],
                mark(strat[0]),
                strat[1],
                mark(strat[1]),
                strat[2],
                mark(strat[2]),
                bh
            );
        }
        println!("--- 做空腿 short_leg_pnl（S/A/O）+ net_short%（S）+ §8.1(S）---");
        for (sym, _, _, spnl, nshort, sinks, fshort) in &rows {
            println!(
                "{:<6} short_pnl S={:+.0} A={:+.0} O={:+.0} | net_short(S)={:.0}% sink(S)={} §8.1(S)={}",
                sym, spnl[0], spnl[1], spnl[2], nshort[0], sinks[0], fshort[0]
            );
        }
        println!("======================================================\n");
        assert!(!rows.is_empty(), "至少跑出一个标的");
    }
}
