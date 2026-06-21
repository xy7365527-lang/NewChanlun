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
        }
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
                let view = {
                    let segs = self.orch.segments();
                    let m2r = self.orch.merged_to_raw();
                    let a0 = build_a0_fast(segs, m2r, &self.prefix_pos, &self.prefix_neg);
                    let tree = iterate(a0, self.mode);
                    extract_chain(&tree)
                };
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
        // bar 加权核心(root)方向 + 净敞口符号。
        let root = self.driver.root();
        if let Some(rs) = root.root_slot() {
            match root.instance(rs).direction {
                crate::trading::types::Polarity::Long => self.core_long_bars += 1,
                crate::trading::types::Polarity::Short => self.core_short_bars += 1,
            }
        }
        let (lu, su) = root.exposure();
        if lu - su > 1e-9 {
            self.net_long_bars += 1;
        } else if su - lu > 1e-9 {
            self.net_short_bars += 1;
        }
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

    /// 递归引擎 BTC 回测（L3 验证）：递归 vs flat 基线 vs BH。
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::rec_btc -- --ignored --nocapture`
    #[test]
    #[ignore = "递归引擎 BTC 回测，需 analysis/data_cache/btc_1m_full.json"]
    fn rec_btc() {
        use crate::recursive_t::backtest_run::{load_clean_ohlc, SYMBOLS};
        use std::path::PathBuf;

        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("analysis/data_cache");
        let (_sym, file) = SYMBOLS.iter().find(|(s, _)| *s == "BTC").unwrap();
        let path = data_dir.join(file);
        if !path.exists() {
            eprintln!("[BTC] 数据缺失 {path:?}，跳过");
            return;
        }
        let (o, h, l, c) = load_clean_ohlc(&path);
        let n = c.len();
        let t0 = std::time::Instant::now();
        let mut s = RecStream::new(PerfectionMode::Structural);
        for i in 0..n {
            s.push_bar(o[i], h[i], l[i], c[i]);
        }
        let fin = s.finish();
        let bh = if n > 0 && c[0] > 0.0 { (c[n - 1] / c[0] - 1.0) * 100.0 } else { 0.0 };
        let strat = (fin / INITIAL_CAPITAL - 1.0) * 100.0;
        let r = s.driver().root();
        eprintln!(
            "\n========== 递归 T 引擎 BTC 回测（structural）==========\n\
             bars={n} reruns={} ({:.1}s)\n\
             strat={strat:+.2}%  bh={bh:+.2}%  final_nav={fin:.2}\n\
             操作: enter={} sink={} recover={} spawn={} flip={} 退本金={}\n\
             short_leg_pnl={:+.0}  期末活跃实例={}\n\
             chain[0]方向分布: up={}({:.1}%) down={}({:.1}%)\n\
             核心(root)方向: long={}({:.1}%) short={}({:.1}%)\n\
             净敞口符号: long={}({:.1}%) short={}({:.1}%) [对比chain0定位根因]\n\
             ====================================================",
            s.n_reruns,
            t0.elapsed().as_secs_f64(),
            r.n_enters,
            r.n_sinks,
            r.n_recovers,
            r.n_spawns,
            r.n_flips,
            r.n_capital_recovered,
            r.short_leg_pnl,
            r.n_active(),
            s.c0_up_bars,
            100.0 * s.c0_up_bars as f64 / n.max(1) as f64,
            s.c0_down_bars,
            100.0 * s.c0_down_bars as f64 / n.max(1) as f64,
            s.core_long_bars,
            100.0 * s.core_long_bars as f64 / n.max(1) as f64,
            s.core_short_bars,
            100.0 * s.core_short_bars as f64 / n.max(1) as f64,
            s.net_long_bars,
            100.0 * s.net_long_bars as f64 / n.max(1) as f64,
            s.net_short_bars,
            100.0 * s.net_short_bars as f64 / n.max(1) as f64,
        );
        // 诊断测试：final_nav 可为负（核心 flip 翻空在 BTC 牛市被轧 = 真实灾难结果，非 bug；
        // TW 守恒守卫已全程零 panic 证会计正确）。只断言有限（NaN/Inf = 真 bug）。
        assert!(fin.is_finite(), "final_nav 有限（NaN/Inf=会计 bug）；负值=策略灾难是合法 L3 观测");
    }
}
