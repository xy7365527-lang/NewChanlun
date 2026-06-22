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
    /// 诊断（编排者排查 2026-06-21）：核心(root highest_active)每 level 停留 bar + emergent_top 每 level
    /// bar——查核心是否到最高涌现级别（net_short 62.7% 疑核心没到最高级别被低级别 BSP flip）。
    pub core_level_bars: [u64; 9],
    pub emergent_level_bars: [u64; 9],
    last_emergent_level: i32, // -1=none
    /// 诊断（编排者 2026-06-21）：最后一次重跑 tree 各级别走势类型分布——查 L4 type1_buy=0
    /// 是否走势结构。[level][0=UpTrend,1=DownTrend,2=Consol-Up,3=Consol-Down]。
    pub tree_trend_stats: [[u64; 4]; 9],
    /// 诊断（编排者 2026-06-21，no_leave 根因）：跨重跑累加，逐级别 ConsolDown 切分性质——
    /// 验证 no_leave 盘整是「末组生长中走势」还是「中间走势被切掉离开段（segment bug）」。
    /// [level]: 0=ConsolDown总数 1=no_leave数 2=其中末组(gi+1==n_groups) 3=其中中间(gi+1<n,=segment bug)
    ///          4=no_leave且completed(被反向终结=apply_t标记) 5=no_leave且completed且无bsp。
    pub consoldn_split: [[u64; 6]; 9],
    /// 每级别**首个** no_leave ConsolDown 的完整结构 dump（units 长度/中枢范围/进入离开段/completed/gi）。
    pub first_no_leave_dump: [Option<String>; 9],
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
            core_level_bars: [0; 9],
            emergent_level_bars: [0; 9],
            last_emergent_level: -1,
            tree_trend_stats: [[0; 4]; 9],
            consoldn_split: [[0; 6]; 9],
            first_no_leave_dump: Default::default(),
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
                let (v, new_bsps, trend_stats, cd_split, cd_dumps) = {
                    let segs = self.orch.segments();
                    let m2r = self.orch.merged_to_raw();
                    let a0 = build_a0_fast(segs, m2r, &self.prefix_pos, &self.prefix_neg, false);
                    let tree = iterate(a0, self.mode);
                    use crate::recursive_t::types::{Direction, TrendKind};
                    // 诊断（编排者 2026-06-21）：各级别走势类型分布（覆盖=最后一次重跑 tree）。
                    let mut ts = [[0u64; 4]; 9];
                    // no_leave 根因切分（复刻 judge_consolidation_divergence 判据）+ 首例结构 dump。
                    let mut cds = [[0u64; 6]; 9];
                    let mut dumps: [Option<String>; 9] = Default::default();
                    for lvl_out in &tree.levels {
                        let lv = lvl_out.level;
                        if lv >= 9 {
                            continue;
                        }
                        let n_groups = lvl_out.trends.len();
                        for (gi, tr) in lvl_out.trends.iter().enumerate() {
                            match (tr.kind, tr.direction) {
                                (TrendKind::UpTrend, _) => ts[lv][0] += 1,
                                (TrendKind::DownTrend, _) => ts[lv][1] += 1,
                                (TrendKind::Consolidation, Direction::Up) => ts[lv][2] += 1,
                                (TrendKind::Consolidation, Direction::Down) => ts[lv][3] += 1,
                            }
                            if tr.kind != TrendKind::Consolidation || tr.direction != Direction::Down {
                                continue;
                            }
                            cds[lv][0] += 1; // ConsolDown 总数
                            if tr.zhongshus.len() != 1 {
                                continue;
                            }
                            let center = match tr.zhongshus.first() {
                                Some(c) if !c.units.is_empty() => c,
                                _ => continue,
                            };
                            let enter_end = *center.units.last().unwrap();
                            if enter_end + 1 < tr.units.len() {
                                continue; // 有离开段，非 no_leave
                            }
                            cds[lv][1] += 1; // no_leave
                            let is_last = gi + 1 == n_groups;
                            if is_last {
                                cds[lv][2] += 1; // 末组（生长中走势）
                            } else {
                                cds[lv][3] += 1; // 中间走势无离开段（= segment 切分 bug 证据）
                            }
                            if tr.completed {
                                cds[lv][4] += 1; // no_leave 却被标 completed
                                if tr.bsp.is_none() {
                                    cds[lv][5] += 1; // 被反向终结（apply_t line 96）标 completed 无 bsp
                                }
                            }
                            if dumps[lv].is_none() {
                                dumps[lv] = Some(format!(
                                    "units.len={} 中枢units={:?} 进入段[0..={}] 离开段[{}..{}](空={}) completed={} has_bsp={} gi={}/{} 末组={}",
                                    tr.units.len(), center.units, enter_end,
                                    enter_end + 1, tr.units.len(), enter_end + 1 >= tr.units.len(),
                                    tr.completed, tr.bsp.is_some(), gi, n_groups, is_last,
                                ));
                            }
                        }
                    }
                    let bs: Vec<(i64, f64, usize, BSPKind)> = tree
                        .all_bsps()
                        .into_iter()
                        .map(|b| (b.bar, b.price, b.level, b.kind))
                        .collect();
                    (extract_view(&tree), bs, ts, cds, dumps)
                };
                view = v;
                // 累加全程（每次重跑 tree 的走势类型 + no_leave 切分），首例结构只记一次。
                for lv in 0..9 {
                    for i in 0..4 {
                        self.tree_trend_stats[lv][i] += trend_stats[lv][i];
                    }
                    for i in 0..6 {
                        self.consoldn_split[lv][i] += cd_split[lv][i];
                    }
                    if self.first_no_leave_dump[lv].is_none() {
                        if let Some(d) = &cd_dumps[lv] {
                            self.first_no_leave_dump[lv] = Some(d.clone());
                        }
                    }
                }
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
                self.last_emergent_level = match view.emergent_top {
                    Some((lvl, _)) => lvl as i32,
                    None => -1,
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
        if self.last_emergent_level >= 0 && (self.last_emergent_level as usize) < 9 {
            self.emergent_level_bars[self.last_emergent_level as usize] += 1;
        }
        // bar 加权核心(root)方向 + 净敞口符号 + 核心短头 episode 追踪。
        let root = self.driver.root();
        let cur_dir: i8 = match root.highest_active() {
            Some(rs) => {
                if rs < 9 {
                    self.core_level_bars[rs] += 1; // 核心每 level 停留 bar（编排者排查）
                }
                match root.instance(rs).direction {
                    crate::trading::types::Polarity::Long => {
                        self.core_long_bars += 1;
                        1
                    }
                    crate::trading::types::Polarity::Short => {
                        self.core_short_bars += 1;
                        -1
                    }
                }
            }
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

    /// **盘整 no_leave 根因诊断**（编排者 2026-06-21）：流式 BTC Structural，逐级别切分 ConsolDown
    /// 的 no_leave 性质——验证「11784 个 L4 ConsolDown 全 no_leave」是 r* 边界生长中走势（末组、
    /// 不标 completed）还是 segment 切掉离开段（中间走势）/被反向终结误标 completed。
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::rec_btc_consol_no_leave -- --ignored --nocapture`
    #[test]
    #[ignore = "盘整 no_leave 诊断，需 analysis/data_cache/btc_1m_full.json"]
    fn rec_btc_consol_no_leave() {
        use crate::recursive_t::backtest_run::{load_clean_ohlc, SYMBOLS};
        use std::path::PathBuf;

        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("analysis/data_cache");
        for (sym, file) in SYMBOLS {
            if sym != "BTC" {
                continue;
            }
            let path = data_dir.join(file);
            if !path.exists() {
                eprintln!("[BTC] 数据缺失 {path:?}，跳过");
                return;
            }
            let (o, h, l, c) = load_clean_ohlc(&path);
            let n = c.len();
            let mut s = RecStream::new(PerfectionMode::Structural);
            for i in 0..n {
                s.push_bar(o[i], h[i], l[i], c[i]);
            }
            s.finish();
            eprintln!(
                "\n===== [BTC] 流式 ConsolDown no_leave 切分（Structural, bars={n} reruns={}）=====",
                s.n_reruns
            );
            eprintln!(
                "{:>5} {:>10} {:>10} {:>9} {:>10} {:>9} {:>12}",
                "level", "ConsolDn", "no_leave", "末组", "中间BUG", "compl", "compl无bsp"
            );
            for lv in 0..6 {
                let r = s.consoldn_split[lv];
                eprintln!(
                    "{:>5} {:>10} {:>10} {:>9} {:>10} {:>9} {:>12}",
                    lv, r[0], r[1], r[2], r[3], r[4], r[5]
                );
            }
            for lv in 0..6 {
                if let Some(d) = &s.first_no_leave_dump[lv] {
                    eprintln!("  L{lv} 首个 no_leave ConsolDown: {d}");
                }
            }
            eprintln!(
                "判读：'中间BUG'>0 ⟺ segment 切掉中间盘整离开段；'compl无bsp'>0 ⟺ no_leave 被反向终结误标 completed。"
            );
        }
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
                     flip={} ascend={} short_pnl={:+.0} net_short={:.1}% liq={} | 三阶段 cap_rec={} earn={} \
                     earn_u={:.1} gain={:.1}% ({:.1}s)",
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
                    r.n_capital_recovered,
                    r.n_earning_deploys,
                    r.earning_units_added,
                    r.max_core_gain_x1000 as f64 / 10.0,
                    t0.elapsed().as_secs_f64()
                );
                // per-level 短差 P&L 验收（编排者方向 3：哪个级别短差赚/亏）。
                let spnl_lvl: Vec<i64> = (0..6).map(|k| r.short_pnl_by_level[k] as i64).collect();
                eprintln!("  [{sym:<5}/{mode:?}] short_pnl_by_level={:?}", spnl_lvl);
                // ── prove 守卫族读数（L2：让数据告诉我们哪些不变量被违反，编排者 2026-06-21）──
                // no_trigger=0 是 panic 守卫保证的（>0 早已 panic）；其余三项为观测计数。
                let g = r.guards();
                eprintln!(
                    "  [{sym:<5}/{mode:?}] PROVE ops(bsp/emrg/eod)={:?} no_trigger={} | dir_mismatch={}/{} \
                     | sink≠recover campaigns={}/{} 净失衡={} | neg_pnl campaigns={} by_level={:?}",
                    &g.ops_by_trigger[..3],
                    g.n_ops_without_trigger,
                    g.n_dir_mismatch,
                    g.n_dir_checks,
                    g.n_sink_recover_imbalance,
                    g.n_campaigns_checked,
                    g.sink_recover_imbalance_total,
                    g.n_neg_pnl_campaigns,
                    &g.neg_pnl_by_level[..6.min(g.neg_pnl_by_level.len())],
                );
                if sym == "BTC" {
                    use crate::trading::types::Polarity as Pol;
                    // 编排者排查 2026-06-21：核心是否到最高涌现级别 + emergence_upgrade 调用/成功/跳过。
                    eprintln!(
                        "  [{sym}/{mode:?}] emergence: attempts={} upgrades={} skipped_dir={} | n_flips={} n_ascends={}",
                        r.n_emergence_attempts, r.n_emergence_upgrades, r.n_emergence_skipped_dir, r.n_flips, r.n_ascends
                    );
                    let core_lvl: Vec<u64> = (0..9).map(|k| s.core_level_bars[k]).collect();
                    let emg_lvl: Vec<u64> = (0..9).map(|k| s.emergent_level_bars[k]).collect();
                    eprintln!("  [{sym}/{mode:?}] core_level_bars(核心停留)={core_lvl:?}");
                    eprintln!("  [{sym}/{mode:?}] emergent_level_bars(涌现级别)={emg_lvl:?}");
                    let trend_s: Vec<[u64; 4]> = (0..6).map(|k| s.tree_trend_stats[k]).collect();
                    eprintln!("  [{sym}/{mode:?}] tree_trend_stats[Up,Down,ConsolUp,ConsolDown]/lvl={trend_s:?}");
                    let consol_diag: Vec<[u64; 4]> = crate::recursive_t::divergence::CONSOL_DOWN_DIAG
                        .with(|d| (0..6).map(|k| d.borrow()[k]).collect());
                    eprintln!("  [{sym}/{mode:?}] ConsolDown判定[no_new_low,no_force,produced,no_leave]/lvl={consol_diag:?}");
                    crate::recursive_t::divergence::CONSOL_DOWN_DIAG.with(|d| *d.borrow_mut() = [[0; 4]; 9]);
                    eprintln!(
                        "  [{sym}/{mode:?}] core_long_bars={} core_short_bars={} c0_up={} c0_down={}",
                        s.core_long_bars, s.core_short_bars, s.c0_up_bars, s.c0_down_bars
                    );
                    // 核心 flip 序列：查低级别 ping-pong。
                    let n_l2s = r.flip_log.iter().filter(|(_, f, t, _, _)| *f == Pol::Long && *t == Pol::Short).count();
                    let n_s2l = r.flip_log.iter().filter(|(_, f, t, _, _)| *f == Pol::Short && *t == Pol::Long).count();
                    let flip_by_lvl: Vec<usize> = r.flip_log.iter().map(|(_, _, _, j, _)| *j).collect();
                    eprintln!(
                        "  [{sym}/{mode:?}] flips={} (Long→Short={n_l2s} Short→Long={n_s2l}) 触发级别={flip_by_lvl:?}",
                        r.flip_log.len()
                    );
                    let head: Vec<(i64, i8, usize)> = r.flip_log.iter().take(40)
                        .map(|(b, _, t, j, _)| (*b, if *t == Pol::Long { 1i8 } else { -1 }, *j)).collect();
                    eprintln!("  [{sym}/{mode:?}] flip序列前40 (bar,到向[1=L/-1=S],触发级别)={head:?}");
                    // sink/recover 路由对称性（编排者排查：为什么 recover<sink）。
                    let sink_lvl: Vec<u64> = (0..6).map(|k| r.sink_by_level[k]).collect();
                    let recover_lvl: Vec<u64> = (0..6).map(|k| r.recover_by_level[k]).collect();
                    eprintln!("  [{sym}/{mode:?}] sink_by_level={sink_lvl:?} recover_by_level={recover_lvl:?}");
                    eprintln!(
                        "  [{sym}/{mode:?}] 买点路由分类: recover={} noop(无短差浪费)={} sink(父空)={} core(核心级)={}",
                        r.buy_recover, r.buy_noop, r.buy_sink, r.buy_core
                    );
                    let sells: Vec<u64> = (0..6).map(|k| s.bsp_by_level[k][1]).collect();
                    let t1buy_lvl: Vec<u64> = (0..6).map(|k| s.bsp_by_level[k][0]).collect();
                    eprintln!(
                        "  [{sym}/{mode:?}] reruns={} t1sell_by_level={:?} t1buy_by_level={:?}",
                        s.n_reruns, sells, t1buy_lvl
                    );
                    // 编排者 2026-06-21：type2/type3 买卖点（UpTrend 自身产买点 → 可 recover 短差空头，
                    // 不需 DownTrend）。验收最高级别走势类型上的 type2/3 buy 数量。
                    let t2buy: Vec<u64> = (0..6).map(|k| s.bsp_by_level[k][2]).collect();
                    let t3buy: Vec<u64> = (0..6).map(|k| s.bsp_by_level[k][4]).collect();
                    let t2sell: Vec<u64> = (0..6).map(|k| s.bsp_by_level[k][3]).collect();
                    let t3sell: Vec<u64> = (0..6).map(|k| s.bsp_by_level[k][5]).collect();
                    eprintln!(
                        "  [{sym}/{mode:?}] t2buy_by_level={t2buy:?} t3buy_by_level={t3buy:?} | t2sell={t2sell:?} t3sell={t3sell:?}"
                    );
                    // 强平三阶段快照（编排者：查降成本/退本金保护 + 全仓 vs 逐仓）。
                    eprintln!(
                        "  [{sym}/{mode:?}] 强平三阶段快照(stage[0CostRed/1CapRec/2Earn],cost_basis,withdrawn,notional_in,nav)={:?}",
                        r.liq_snapshot
                    );
                    // 强平 episode 表（编排者：查回补失败=空头没匹配买点被强平）。
                    eprintln!(
                        "  [{sym}/{mode:?}] 强平 episode（{}个）lv|开空bar|开空价|强平bar|强平价|空?|期间任意级别买点",
                        r.liq_log.len()
                    );
                    for &(lv, eb, ep, lb, lp, short) in r.liq_log.iter() {
                        // 期间 (开空bar, 强平bar] 任意级别 type1_buy（流式确认 bar 坐标）。
                        let buys: Vec<(i64, f64, usize)> =
                            s.t1buy.iter().filter(|(b, _, _)| *b > eb && *b <= lb).cloned().collect();
                        let buy_lvls: Vec<usize> = buys.iter().map(|(_, _, l)| *l).collect();
                        // 同级别买点（lv）是否 fire（高级别空头需同级别买点 recover）。
                        let same_lvl = buys.iter().filter(|(_, _, l)| *l == lv).count();
                        eprintln!(
                            "    L{lv}|{eb}|{ep:.0}|{lb}|{lp:.0}|{}|{}个(同级别{})levels={:?}",
                            if short { "空" } else { "多" },
                            buys.len(),
                            same_lvl,
                            buy_lvls
                        );
                    }
                    // 编排者诊断：L2/L3/L4 type1_buy 的 bar 分布（确认高级别空头持仓期间有无同级别买点）。
                    for lvl in [2usize, 3, 4] {
                        let buys: Vec<i64> =
                            s.t1buy.iter().filter(|(_, _, l)| *l == lvl).map(|(b, _, _)| *b).collect();
                        let first = buys.first().copied().unwrap_or(-1);
                        let last = buys.last().copied().unwrap_or(-1);
                        let head: Vec<i64> = buys.iter().take(8).copied().collect();
                        eprintln!(
                            "    L{lvl} type1_buy={}个 最早bar={} 最晚bar={} 前8={:?}",
                            buys.len(),
                            first,
                            last,
                            head
                        );
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

    /// **BTC 牛熊段收益归因**（编排者 2026-06-21）：zigzag(40%反转)分牛熊段，逐段算引擎 TW 收益（MtM）
    /// vs BH，段内短差 pnl + 强平落点（牛/熊段）。Structural 模式。诊断纯观测（不改引擎/不影响 bit-exact）。
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::btc_bull_bear_segments -- --exact --ignored --nocapture`
    #[test]
    #[ignore = "BTC 牛熊段归因诊断，需 btc_1m_full.json"]
    fn btc_bull_bear_segments() {
        use crate::recursive_t::backtest_run::load_clean_ohlc;
        use std::path::PathBuf;
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("analysis/data_cache/btc_1m_full.json");
        if !path.exists() {
            eprintln!("数据缺失 {path:?}");
            return;
        }
        let (o, h, l, c) = load_clean_ohlc(&path);
        let n = c.len();

        // ── zigzag 分段（反转阈值 env ZZ，默认 0.40，BTC 巨幅摆动）→ 交替 峰/谷 pivot ──
        let rev: f64 = std::env::var("ZZ").ok().and_then(|s| s.parse().ok()).unwrap_or(0.40);
        let mut pivots: Vec<(usize, f64, bool)> = Vec::new(); // (bar, price, is_peak)
        let (mut ext_bar, mut ext_price) = (0usize, c[0]);
        let mut up = true; // BTC 2015 低位起步 → 先找峰
        for i in 1..n {
            if up {
                if c[i] > ext_price {
                    ext_price = c[i];
                    ext_bar = i;
                } else if c[i] < ext_price * (1.0 - rev) {
                    pivots.push((ext_bar, ext_price, true));
                    up = false;
                    ext_price = c[i];
                    ext_bar = i;
                }
            } else if c[i] < ext_price {
                ext_price = c[i];
                ext_bar = i;
            } else if c[i] > ext_price * (1.0 + rev) {
                pivots.push((ext_bar, ext_price, false));
                up = true;
                ext_price = c[i];
                ext_bar = i;
            }
        }
        // 段边界 = [0, pivot_bars..., n-1]
        let mut bounds: Vec<usize> = vec![0];
        for &(b, _, _) in &pivots {
            bounds.push(b);
        }
        bounds.push(n - 1);
        bounds.dedup();

        // ── 跑 RecStream Structural，逐 bar 累计核心方向，边界记录快照（核心 vs 次级别分离）──
        use crate::trading::types::Polarity;
        struct Snap {
            bar: usize,
            tw: f64,
            spl: [f64; 6], // short_pnl_by_level 累计（次级别短差腿 per-level）
            cl: u64,       // 累计核心 Long bar（highest_active=Long）
            cs: u64,       // 累计核心 Short bar（主力做空 = 无 parent flip 到 Short）
            lv: f64,       // 边界处 多头市值 = long_units * c（核心+次级别多）
            free: f64,     // 自由现金
            wd: f64,       // withdrawn（退本金，锁定不在险）
        }
        let mut s = RecStream::new(PerfectionMode::Structural);
        let mut snaps: Vec<Snap> = Vec::new();
        let (mut cl, mut cs) = (0u64, 0u64);
        let mut bi = 0usize;
        for i in 0..n {
            s.push_bar(o[i], h[i], l[i], c[i]);
            // 逐 bar 核心方向（highest_active 的 direction）：区分核心 Long/Short（主力方向）。
            {
                let root = s.driver().root();
                if let Some(k) = root.highest_active() {
                    match root.instance(k).direction {
                        Polarity::Long => cl += 1,
                        Polarity::Short => cs += 1,
                    }
                }
            }
            while bi < bounds.len() && bounds[bi] == i {
                let root = s.driver().root();
                let mut spl = [0.0; 6];
                for (j, v) in spl.iter_mut().enumerate() {
                    *v = root.short_pnl_by_level[j];
                }
                let (lu, _) = root.exposure();
                snaps.push(Snap {
                    bar: i,
                    tw: root.total_wealth(c[i]),
                    spl,
                    cl,
                    cs,
                    lv: lu * c[i],
                    free: root.free(),
                    wd: root.withdrawn_total(),
                });
                bi += 1;
            }
        }
        let _ = s.finish();
        let liq = s.driver().root().liq_log.clone();
        let flips = s.driver().root().flip_log.clone(); // (bar, from, to, j_level, is_buy)

        // ── 表1：核心方向（主力）vs 次级别短差总和 ──
        eprintln!(
            "\n===== BTC 牛熊段 核心(主力)vs次级别(短差) 归因（Structural, zigzag {:.0}%）=====",
            rev * 100.0
        );
        eprintln!(
            "{:>4} {:>8} {:>8} {:>9} {:>9} {:>8} {:>8} {:>7} {:>11} {:>5}",
            "段", "起价", "止价", "引擎TW%", "BH%", "核心多%", "核心空%", "flip→S", "次级短差Σ", "强平"
        );
        for w in snaps.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let is_bull = c[b.bar] >= c[a.bar];
            let eng = (b.tw / a.tw - 1.0) * 100.0;
            let bh = (c[b.bar] / c[a.bar] - 1.0) * 100.0;
            let segbars = ((b.cl + b.cs) - (a.cl + a.cs)).max(1);
            let clp = 100.0 * (b.cl - a.cl) as f64 / segbars as f64;
            let csp = 100.0 * (b.cs - a.cs) as f64 / segbars as f64;
            let flip_s = flips
                .iter()
                .filter(|(fb, _, t, _, _)| {
                    *fb > a.bar as i64 && *fb <= b.bar as i64 && *t == Polarity::Short
                })
                .count();
            let seg_short: f64 = (0..6).map(|j| b.spl[j] - a.spl[j]).sum();
            let nliq =
                liq.iter().filter(|(_, _, _, lb, _, _)| *lb > a.bar as i64 && *lb <= b.bar as i64).count();
            eprintln!(
                "{:>4} {:>8.0} {:>8.0} {:>+8.1}% {:>+8.1}% {:>7.1}% {:>7.1}% {:>7} {:>+11.0} {:>5}",
                if is_bull { "牛" } else { "熊" },
                c[a.bar],
                c[b.bar],
                eng,
                bh,
                clp,
                csp,
                flip_s,
                seg_short,
                nliq
            );
        }

        // ── 表2：各段 次级别短差 per-level P&L（sink/recover 空头腿 L0..L5）──
        eprintln!("\n--- 各段 次级别短差 per-level（L0..L5，正=该级别短差段内赚）---");
        eprintln!(
            "{:>4} {:>8} {:>8} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}",
            "段", "起价", "止价", "L0", "L1", "L2", "L3", "L4", "L5"
        );
        for w in snaps.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let is_bull = c[b.bar] >= c[a.bar];
            let d: Vec<i64> = (0..6).map(|j| (b.spl[j] - a.spl[j]) as i64).collect();
            eprintln!(
                "{:>4} {:>8.0} {:>8.0} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}",
                if is_bull { "牛" } else { "熊" },
                c[a.bar],
                c[b.bar],
                d[0],
                d[1],
                d[2],
                d[3],
                d[4],
                d[5]
            );
        }

        // ── 表3：各段 资本结构（段首→段尾 暴露率，揭示熊段靠减仓避损还是主力做空）──
        eprintln!("\n--- 各段 资本结构（多头市值/TW · 现金/TW · withdrawn退本金/TW，段首→段尾）---");
        eprintln!(
            "{:>4} {:>8} {:>8} {:>17} {:>17} {:>17}",
            "段", "起价", "止价", "多头暴露%首→尾", "现金%首→尾", "退本金%首→尾"
        );
        for w in snaps.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let is_bull = c[b.bar] >= c[a.bar];
            let pct = |v: f64, tw: f64| if tw.abs() > 1.0 { 100.0 * v / tw } else { 0.0 };
            eprintln!(
                "{:>4} {:>8.0} {:>8.0}  {:>6.1}→{:>6.1}  {:>6.1}→{:>6.1}  {:>6.1}→{:>6.1}",
                if is_bull { "牛" } else { "熊" },
                c[a.bar],
                c[b.bar],
                pct(a.lv, a.tw),
                pct(b.lv, b.tw),
                pct(a.free, a.tw),
                pct(b.free, b.tw),
                pct(a.wd, a.tw),
                pct(b.wd, b.tw)
            );
        }

        let bh_total = (c[n - 1] / c[0] - 1.0) * 100.0;
        let eng_total = (snaps.last().unwrap().tw / snaps.first().unwrap().tw - 1.0) * 100.0;
        let flip_s_total = flips.iter().filter(|(_, _, t, _, _)| *t == Polarity::Short).count();
        eprintln!(
            "\n全程 BH={:+.1}% 引擎TW={:+.1}% | 核心flip总数={}(→Short={}) 触发级别={:?} | 强平={}",
            bh_total,
            eng_total,
            flips.len(),
            flip_s_total,
            flips.iter().filter(|(_, _, t, _, _)| *t == Polarity::Short).map(|(b, _, _, j, _)| (*b, *j)).collect::<Vec<_>>(),
            liq.len()
        );
        assert!(!snaps.is_empty());
    }

    /// **prove 守卫尺度不变性深度验证**（编排者 2026-06-22）：1s vs 1min 同标的同时段对照。
    ///
    /// ## 任务（观测分辨率维度，非操作床位）
    /// 缠论级别递归 ≅ 多尺度滤波器组（小波 MRA）。提高采样率（1min→1s）让滤波器组分辨更细尺度，
    /// 多出深层滤波器（lid4/lid5 涌现）。prove 守卫 = 滤波器组不变量的可执行形式，**应在新深层继续
    /// 成立**。本测试验证：
    /// 1. **滤波器层数对照**：1s vs 1min 最高涌现级别（highest_active 峰值）+ 各层走势组数（滤波器
    ///    通带，tree_trend_stats 汇总）多几层。
    /// 2. **4 panic 守卫尺度不变性**：sink_descends / sigma_quota / relabel_invariant /
    ///    bsp_triggers_operation 在 1s 更深塔**零 fire**（进程跑完零 panic = 尺度不变性在更细尺度成立；
    ///    这四个守卫已接入 rec_engine 操作热路径，违反即 panic 终止）。
    /// 3. **radial_scaling（f∝λ⁻ᵏ）**：1s 是否在**更宽 k 范围**（更多层）继续几何递减
    ///    （count_radial_scaling_violations on sink_by_level）。
    /// 4. **sigma_quota（m=u/3 增益尺度不变）**：在新深层成立（sigma_quota panic 守卫零 fire 覆盖）。
    ///
    /// ## 认识论等级
    /// prove 守卫尺度不变性的**深度验证**（尺度维度）vs 8 标的 L3 的**宽度验证**（标的维度）。
    /// 当前 **L2**（单标的 1s）。**不评估 1s 交易收益/alpha**——那是被否证的操作床位维度
    /// （[[project_cl_1s_a0_verdict]]：秒级=纯观测分辨率非操作床位，0.58bps<taker 1.45bps）。
    /// 若某 panic 守卫在 1s 深层 fire = 滤波器自相似的有效域边界（formalization-validity-domain，
    /// 否定性结果比确认性结果更有价值）——如实报告是哪个守卫/哪层/哪 bar。
    ///
    /// 跑法（先 CL 小数据控制内存，BT_1S_SYMBOLS=BTC 切 BTC）：
    /// `cargo test --release recursive_t::rec_stream::tests::scale_invariance_1s -- --exact --ignored --nocapture`
    #[test]
    #[ignore = "1s prove 守卫尺度不变性验证，需 cl_1s_databento_1mo.json / btc_1s_2week.json"]
    fn scale_invariance_1s() {
        use crate::recursive_t::backtest_run::{load_clean_ohlc, load_clean_ohlc_window};
        use std::path::PathBuf;
        // 数据目录：默认 `<repo>/analysis/data_cache`；worktree 隔离运行时大 JSON 被 gitignore
        // 不在 worktree 内，用 CHANLUN_DATA_DIR 指向主仓库 data_cache（绝对路径）。
        let data_dir = match std::env::var("CHANLUN_DATA_DIR") {
            Ok(d) => PathBuf::from(d),
            Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("analysis/data_cache"),
        };

        // 标的选择（默认 CL：29MB/577k bar 小数据；BT_1S_SYMBOLS=BTC 切 81MB/2 周）。
        let which = std::env::var("BT_1S_SYMBOLS").unwrap_or_else(|_| "CL".to_string()).to_uppercase();

        // (label, 1s 文件, 1min 对照文件 + 日期窗 [start,end] 闭区间（None=无 1min 对照）)
        let cases: Vec<(&str, PathBuf, Option<(PathBuf, &str, &str)>)> = match which.as_str() {
            "CL" => vec![
                // CL 1s 整月（2025-04-01..04-30）vs CL 1m 同窗（10y 文件切同区间）。
                ("CL 1s 1mo", data_dir.join("cl_1s_databento_1mo.json"),
                 Some((data_dir.join("cl_1m_databento_10y.json"), "2025-04-01", "2025-04-30"))),
            ],
            "BTC" => vec![
                // BTC 1s 2 周（2026-05-29..06-11）vs BTC 1m 同窗（btc_1m_full.json 切同区间）。
                ("BTC 1s 2w", data_dir.join("btc_1s_2week.json"),
                 Some((data_dir.join("btc_1m_full.json"), "2026-05-29", "2026-06-11"))),
            ],
            other => panic!("BT_1S_SYMBOLS={other} 未知（支持 CL / BTC）"),
        };

        for (label, path_1s, cmp_1min) in &cases {
            if !path_1s.exists() {
                eprintln!("[{label}] 1s 数据缺失 {path_1s:?}，跳过");
                continue;
            }
            // ── 跑 1s：完成 = 4 panic 守卫全程零 fire（尺度不变性核心交付）──
            let (o, h, l, c) = load_clean_ohlc(path_1s);
            let n = c.len();
            let r_1s = run_one_scale(&o, &h, &l, &c);
            eprintln!(
                "\n========== prove 守卫尺度不变性：{label}（Structural, bars={n}）==========",
            );
            report_scale(&format!("{label} [1s 采样]"), n, &r_1s);

            // ── 跑 1min 同时段对照（若有）──
            if let Some((p_1m, d0, d1)) = cmp_1min {
                if !p_1m.exists() {
                    eprintln!("[{label}] 1min 对照缺失 {p_1m:?}，仅 1s");
                } else {
                    let (o2, h2, l2, c2) = load_clean_ohlc_window(p_1m, d0, d1);
                    let n2 = c2.len();
                    let r_1m = run_one_scale(&o2, &h2, &l2, &c2);
                    report_scale(&format!("{label} [1min 采样 窗={d0}..{d1}]"), n2, &r_1m);
                    // ── 滤波器层数对照（尺度–频率关系：1s 更细尺度应多出深层滤波器）──
                    eprintln!(
                        "\n--- 滤波器组深度对照（{label}）：1s vs 1min ---\n\
                         采样比 1s/1min bar = {:.1}× | 最深活跃层 1s={} 1min={}（Δ={}）",
                        n as f64 / n2.max(1) as f64,
                        r_1s.max_active_level, r_1m.max_active_level,
                        r_1s.max_active_level as i64 - r_1m.max_active_level as i64,
                    );
                    eprintln!("{:>6} {:>14} {:>14} {:>10}", "level", "1s 通带(走势组)", "1min 通带", "1s 多出");
                    for lv in 0..MAX_LEVEL {
                        let p1s = r_1s.passbands[lv];
                        let p1m = r_1m.passbands[lv];
                        if p1s == 0 && p1m == 0 {
                            continue;
                        }
                        eprintln!("{:>6} {:>14} {:>14} {:>+10}", lv, p1s, p1m, p1s as i64 - p1m as i64);
                    }
                }
            }
        }
    }

    /// 单标的单尺度跑通的 prove 守卫读数（[`scale_invariance_1s`] 汇总单元）。
    struct ScaleResult {
        /// 全程 highest_active() 峰值（= 最深活跃滤波器层；滤波器组深度）。
        max_active_level: usize,
        /// 各级别走势组累计数（≈ 滤波器通带：tree_trend_stats[Up+Down+ConsolUp+ConsolDown] 汇总）。
        passbands: [u64; MAX_LEVEL],
        /// sink_by_level（径向标度律 radial_scaling 的 per_level 输入）。
        sink_by_level: [u64; MAX_LEVEL],
        /// recover_by_level（对照）。
        recover_by_level: [u64; MAX_LEVEL],
        /// t1buy/t1sell per-level（BSP 频率随级别分布，radial_scaling 第二输入）。
        bsp_by_level: [u64; MAX_LEVEL],
        /// radial_scaling 局部违反层数（per_level[k]>per_level[k-1]）on sink_by_level。
        radial_viol_sink: u64,
        /// radial_scaling 局部违反层数 on bsp_by_level。
        radial_viol_bsp: u64,
        n_reruns: u64,
        n_sinks: u64,
        n_recovers: u64,
        // ── 观测计数守卫（非 panic；L2 regime 读数）──
        n_dir_mismatch: u64,
        n_dir_checks: u64,
        n_sink_recover_imbalance: u64,
        n_campaigns_checked: u64,
        n_neg_pnl_campaigns: u64,
        ops_bsp: u64,
        ops_emergence: u64,
        ops_eod: u64,
        n_ops_without_trigger: u64,
    }

    /// 喂全序列 → 跑完（4 panic 守卫零 fire 是隐式前提：fire 则此函数 panic 终止）→ 抽读数。
    fn run_one_scale(o: &[f64], h: &[f64], l: &[f64], c: &[f64]) -> ScaleResult {
        let n = c.len();
        let mut s = RecStream::new(PerfectionMode::Structural);
        let mut max_active = 0usize;
        for i in 0..n {
            s.push_bar(o[i], h[i], l[i], c[i]);
            // 全程追踪最深活跃层（滤波器组深度）。
            if let Some(k) = s.driver().root().highest_active() {
                if k > max_active {
                    max_active = k;
                }
            }
        }
        s.finish();
        let r = s.driver().root();
        let g = r.guards();

        let mut passbands = [0u64; MAX_LEVEL];
        for (lv, pb) in passbands.iter_mut().enumerate() {
            // 走势组总数（Up+Down+ConsolUp+ConsolDown）= 该级别滤波器通带数。
            *pb = s.tree_trend_stats[lv].iter().sum();
        }
        let mut sink_by_level = [0u64; MAX_LEVEL];
        let mut recover_by_level = [0u64; MAX_LEVEL];
        let mut bsp_by_level = [0u64; MAX_LEVEL];
        for lv in 0..MAX_LEVEL {
            sink_by_level[lv] = r.sink_by_level[lv];
            recover_by_level[lv] = r.recover_by_level[lv];
            // t1buy + t1sell（buy=idx0, sell=idx1）= 该级别 type1 BSP 频率。
            bsp_by_level[lv] = s.bsp_by_level[lv][0] + s.bsp_by_level[lv][1];
        }
        ScaleResult {
            max_active_level: max_active,
            passbands,
            radial_viol_sink: crate::recursive_t::prove_guards::count_radial_scaling_violations(
                &sink_by_level,
            ),
            radial_viol_bsp: crate::recursive_t::prove_guards::count_radial_scaling_violations(
                &bsp_by_level,
            ),
            sink_by_level,
            recover_by_level,
            bsp_by_level,
            n_reruns: s.n_reruns,
            n_sinks: r.n_sinks,
            n_recovers: r.n_recovers,
            n_dir_mismatch: g.n_dir_mismatch,
            n_dir_checks: g.n_dir_checks,
            n_sink_recover_imbalance: g.n_sink_recover_imbalance,
            n_campaigns_checked: g.n_campaigns_checked,
            n_neg_pnl_campaigns: g.n_neg_pnl_campaigns,
            ops_bsp: g.ops_by_trigger[0],
            ops_emergence: g.ops_by_trigger[1],
            ops_eod: g.ops_by_trigger[2],
            n_ops_without_trigger: g.n_ops_without_trigger,
        }
    }

    /// 打印单尺度 prove 守卫读数（尺度不变性证据）。
    fn report_scale(tag: &str, n_bars: usize, r: &ScaleResult) {
        eprintln!(
            "\n[{tag}] bars={n_bars} reruns={} | 最深活跃滤波器层={} sink={} recover={}",
            r.n_reruns, r.max_active_level, r.n_sinks, r.n_recovers
        );
        // ① 4 panic 守卫：跑到这里 = sink_descends/sigma_quota/relabel_invariant 零 fire；
        //    bsp_triggers_operation 由 n_ops_without_trigger==0 确认（panic 守卫下恒 0）。
        eprintln!(
            "  [PANIC 守卫尺度不变性] 进程零 panic ⇒ sink_descends/sigma_quota/relabel_invariant 全程成立 \
             | bsp_triggers_operation: ops(bsp={} emrg={} eod={}) no_trigger={}（=0 即成立）",
            r.ops_bsp, r.ops_emergence, r.ops_eod, r.n_ops_without_trigger
        );
        // ② radial_scaling（f∝λ⁻ᵏ）：违反层数（0=全程频率随级别非增，几何递减成立）。
        eprintln!(
            "  [radial_scaling f∝λ⁻ᵏ] sink_by_level 违反层数={} | type1_bsp_by_level 违反层数={}",
            r.radial_viol_sink, r.radial_viol_bsp
        );
        let sbl: Vec<u64> = r.sink_by_level.to_vec();
        let rbl: Vec<u64> = r.recover_by_level.to_vec();
        let bbl: Vec<u64> = r.bsp_by_level.to_vec();
        eprintln!("     sink_by_level    ={sbl:?}");
        eprintln!("     recover_by_level ={rbl:?}");
        eprintln!("     type1bsp_by_level={bbl:?}");
        let pb: Vec<u64> = r.passbands.to_vec();
        eprintln!("     滤波器通带(走势组)/lvl={pb:?}");
        // ③ 观测计数守卫（L2 regime 读数，非 panic）。
        eprintln!(
            "  [观测计数 L2] dir_mismatch={}/{} sink≠recover campaigns={}/{} neg_pnl campaigns={}",
            r.n_dir_mismatch, r.n_dir_checks, r.n_sink_recover_imbalance,
            r.n_campaigns_checked, r.n_neg_pnl_campaigns
        );
    }

    /// **2019后不再入场根因追踪**（编排者 2026-06-22）：采样核心 units 衰减 + free/withdrawn/stage +
    /// enter/ascend/flip 累计 + buy 路由分类，定位「2018 做空赚完后引擎为何不再 enter 重新建仓」。
    /// 关键判据：enter 仅在 highest_active()==None（全塔空仓）触发；若核心 units 几何衰减但永不 ≤EPS
    /// → highest_active 恒 Some → enter 恒不触发 → free 闲置。
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::btc_no_reentry_trace -- --exact --ignored --nocapture`
    #[test]
    #[ignore = "2019后不入场追踪，需 btc_1m_full.json"]
    fn btc_no_reentry_trace() {
        use crate::recursive_t::backtest_run::load_clean_ohlc;
        use std::path::PathBuf;
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("analysis/data_cache/btc_1m_full.json");
        if !path.exists() {
            eprintln!("数据缺失 {path:?}");
            return;
        }
        let (o, h, l, c) = load_clean_ohlc(&path);
        let n = c.len();
        // 关键 bar：熊1底 248736 + 2019 各点 + 各周期边界。
        let key: std::collections::HashSet<usize> =
            [248736usize, 250000, 267706, 300000, 400000, 694639, 971781, 1345277, 2218375]
                .into_iter()
                .collect();

        let mut s = RecStream::new(PerfectionMode::Structural);
        eprintln!("\n===== BTC 2019后不再入场 状态追踪（Structural）=====");
        eprintln!(
            "{:>9} {:>8} {:>4} {:>12} {:>4} {:>11} {:>11} {:>5} {:>4} {:>4} {:>4} {:>7} {:>6} {:>6} {:>8}",
            "bar", "价", "核层", "核units", "活层", "free", "withdrawn", "stg", "ent", "asc",
            "flp", "buyCore", "sink", "recov", "BSP累"
        );
        let mut units_dump: Vec<(usize, Vec<f64>)> = Vec::new();
        for i in 0..n {
            s.push_bar(o[i], h[i], l[i], c[i]);
            if i % 200_000 == 0 || i == n - 1 || key.contains(&i) {
                let r = s.driver().root();
                let (hl, hu) = match r.highest_active() {
                    Some(k) => (k as i64, r.instance(k).units),
                    None => (-1, 0.0),
                };
                let stg = match r.stage().as_u8() {
                    0 => "Cost",
                    1 => "CapR",
                    _ => "Earn",
                };
                eprintln!(
                    "{:>9} {:>8.0} {:>4} {:>12.6} {:>4} {:>11.0} {:>11.0} {:>5} {:>4} {:>4} {:>4} {:>7} {:>6} {:>6} {:>8}",
                    i, c[i], hl, hu, r.n_active(), r.free(), r.withdrawn_total(), stg,
                    r.n_enters, r.n_ascends, r.n_flips, r.buy_core, r.n_sinks, r.n_recovers,
                    s.bsp_counts.iter().sum::<u64>()
                );
                if key.contains(&i) {
                    units_dump.push((i, (0..MAX_LEVEL).map(|k| r.instance(k).units).collect()));
                }
            }
        }
        let _ = s.finish();

        // ── 关键 bar 各 level units（看核心几何衰减是否到 0 / 是否恢复）──
        eprintln!("\n--- 关键 bar 各 level units（核心几何衰减，>EPS=1e-12 即'活着'阻止 enter）---");
        eprintln!(
            "{:>9} {:>8} {:>13} {:>13} {:>13} {:>13} {:>13} {:>13}",
            "bar", "价", "L0", "L1", "L2", "L3", "L4", "L5"
        );
        for (b, u) in &units_dump {
            eprintln!(
                "{:>9} {:>8.0} {:>13.9} {:>13.9} {:>13.9} {:>13.9} {:>13.9} {:>13.9}",
                b, c[*b], u[0], u[1], u[2], u[3], u[4], u[5]
            );
        }
        eprintln!(
            "\n判读：ent列在 248736 后是否不变=2018后零enter；核units衰减但>EPS=highest_active恒Some阻止enter；\
             buyCore增长但ent不变=核心级买点走ascend/flip非enter；free大但锁不进市场。"
        );
    }
}
