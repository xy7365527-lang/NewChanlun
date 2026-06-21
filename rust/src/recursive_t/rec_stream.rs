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
use super::rec_driver::{extract_view, RecDriver};
use super::rec_engine::{LevelView, MAX_LEVEL};
use super::stream::build_a0_fast;
use super::types::{BSPKind, PerfectionMode};

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
    /// 重跑触发 key = (settled 段数, _)。第二位为区间套提前确认预留（当前回退=usize::MAX 固定）。
    last_trigger: (usize, usize),
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
            last_trigger: (0, usize::MAX),
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

    /// 逐 bar 推送。段门控触发重跑 → extract_view → driver.on_view（确认时点 close）。
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

        // 段门控（区间套提前确认，编排者 2026-06-21）：笔增长 → 触发 key=(settled 段数, 最后 candidate
        // 端点 ep1_i)。candidate 出现/c 段顶部移动 → key 变 → 重跑（**顶部附近**），不等 settled（回调底）。
        // 每 bar 一个信号视图（无重跑=empty；重跑时填 nodes/emergent/fresh BSP）= flat：每 bar step。
        let mut view = LevelView::empty();
        let sc = self.orch.strokes().len();
        if sc > self.last_stroke_n {
            self.last_stroke_n = sc;
            let trig = {
                let segs = self.orch.segments();
                // 区间套提前确认试验回退（无效+每笔重跑慢）：触发回 settled count only。
                let settled_n = segs.iter().filter(|s| s.confirmed && s.kind == SegKind::Settled).count();
                (settled_n, usize::MAX)
            };
            if trig != self.last_trigger {
                self.last_trigger = trig;
                // 块内借 orch（segs+m2r）→ build_a0 → iterate → extract_chain（owned，块后释放借用）。
                // 诊断：同时捕获本树全 6 类 BSP（回补质询：查产出/消费）。
                let (v, new_bsps) = {
                    let segs = self.orch.segments();
                    let m2r = self.orch.merged_to_raw();
                    let a0 = build_a0_fast(segs, m2r, &self.prefix_pos, &self.prefix_neg, false);
                    let tree = iterate(a0, self.mode);
                    let bs: Vec<(i64, f64, usize, BSPKind)> = tree
                        .all_bsps()
                        .into_iter()
                        .map(|b| (b.bar, b.price, b.level, b.kind))
                        .collect();
                    (extract_view(&tree), bs)
                };
                view = v;
                // **只消费本次重跑新 fire 的 BSP**（fresh）→ LevelView buy/sell per level（= flat
                // TSignalView diff）。根因修复：all_bsps 全历史累积，存在性检查会机械触发；fresh 是本 bar diff。
                for (bb, bp, bl, kind) in new_bsps {
                    let bk = bsp_kind_idx(kind);
                    if self.bsp_seen.insert((bb, bl, bk)) {
                        self.bsp_counts[bk as usize] += 1;
                        if bl < 10 {
                            self.bsp_by_level[bl][bk as usize] += 1;
                        }
                        if bk == 0 {
                            // 记**流式确认时点 bar**（= cur_bar），同 sink/recover 坐标。
                            self.t1buy.push((bar, bp, bl));
                        } else if bk == 1 {
                            self.t1sell.push((bar, bp, bl));
                        }
                        // fresh BSP → buy/sell（buy: bk 偶 0/2/4；sell: bk 奇 1/3/5，不分 type1/2/3）。
                        if bl < MAX_LEVEL {
                            if bk % 2 == 0 {
                                view.buy[bl] = true;
                            } else {
                                view.sell[bl] = true;
                            }
                        }
                    }
                }
                self.n_reruns += 1;
                // 诊断：本次最高级别走势方向（emergent_top 极性）。
                self.last_c0 = match view.emergent_top {
                    Some((_, crate::trading::types::Polarity::Long)) => 1,
                    Some((_, crate::trading::types::Polarity::Short)) => -1,
                    None => 0,
                };
            }
        }
        // 每 bar 调 on_bar（= flat step 每 bar）：强平每 bar 检查 + emergence/route_bsp 仅在 view 有
        // BSP/emergent（重跑）时触发——empty view（无重跑）仅强平 + 守恒守卫，对照 flat step(empty)。
        self.driver.on_view(&view, c, bar);
        // bar 加权 chain[0] 方向（持续到下次重跑）。
        match self.last_c0 {
            1 => self.c0_up_bars += 1,
            -1 => self.c0_down_bars += 1,
            _ => {}
        }
        // bar 加权核心(root)方向 + 净敞口符号 + 核心短头 episode 追踪。
        let root = self.driver.root();
        let cur_dir: i8 = match root.highest_active() {
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
                for i in 0..n {
                    s.push_bar(o[i], h[i], l[i], c[i]);
                }
                let fin = s.finish();
                strat[mi] = (fin / INITIAL_CAPITAL - 1.0) * 100.0;
                let r = s.driver().root();
                spnl[mi] = r.short_leg_pnl;
                nshort[mi] = 100.0 * s.net_short_bars as f64 / n.max(1) as f64;
                sinks[mi] = r.n_sinks;
                fshort[mi] = r.n_flips;
                eprintln!(
                    "[{sym:<5}/{:>10?}] strat={:+.1}% bh={:+.1}% enter={} sink={} recover={} drain={} \
                     flip={} ascend={} short_pnl={:+.0} net_short={:.1}% liq={} ({:.1}s)",
                    mode,
                    strat[mi],
                    bh,
                    r.n_enters,
                    r.n_sinks,
                    r.n_recovers,
                    r.n_drains,
                    r.n_flips,
                    r.n_ascends,
                    r.short_leg_pnl,
                    nshort[mi],
                    r.n_liquidations,
                    t0.elapsed().as_secs_f64()
                );
                // BTC per-level BSP fire 分布（buy/sell per level，诊断 flat 递归化后操作分布）。
                if sym == "BTC" {
                    let buys: Vec<u64> = (0..6).map(|k| s.bsp_by_level[k][0]).collect();
                    let sells: Vec<u64> = (0..6).map(|k| s.bsp_by_level[k][1]).collect();
                    eprintln!(
                        "  [{sym}/{mode:?}] reruns={} t1buy_by_level={:?} t1sell_by_level={:?}",
                        s.n_reruns, buys, sells
                    );
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
        println!("--- 做空腿 short_leg_pnl（S/A/O）+ net_short%（S）+ flip(S）---");
        for (sym, _, _, spnl, nshort, sinks, fshort) in &rows {
            println!(
                "{:<6} short_pnl S={:+.0} A={:+.0} O={:+.0} | net_short(S)={:.0}% sink(S)={} flip(S)={}",
                sym, spnl[0], spnl[1], spnl[2], nshort[0], sinks[0], fshort[0]
            );
        }
        println!("======================================================\n");
        assert!(!rows.is_empty(), "至少跑出一个标的");
    }
}
