//! 赋格引擎 v3 **三轴组装器**（`FugueEngineCore`）：每 bar 协调 H⁰/groupoid/H¹ 三轴。
//!
//! 用户裁决（2026-06-17）：三轴**分开实装**，轴间通过 trait 接口耦合。本文件是组装层——
//! 持有信号层（H⁰+groupoid 计算内聚）+ `OperateEngine`（H¹ = D∞ word 处理器），每 bar：
//!
//! ```text
//! 1. signal.process(sig, flip)       → frame  （H⁰ 形态学 + groupoid 向心 confirm，§6B.4 内聚）
//! 2. MorphologyBridge / ObserveBridge          （两轴 trait view，零拷贝）
//! 3. operate.step(&h0, &obs)         → outcome （H¹ 通过 trait 读两轴，决定 h/τ word）
//! 4. outcome.consumed → signal.clear_located_* （operate 消费的定位链 → observe 重置）
//! ```
//!
//! ## 公开 API 不变 ⇒ ffi.rs 零改动
//! step/finish/n_trades/result/snapshot 与原契约一致，`PyFugueV3Stream` 透明复用。

use crate::spiral::signal::SignalState;
use crate::spiral::SpiralResult;
use crate::stroke::Direction;
use crate::trading::tape::BarSig;
use crate::trading::types::MAX_LADDER;

use super::axis::OperateAxis;
use super::layer::FugueResult;
use super::morphology::MorphologyBridge;
use super::observe::ObserveBridge;
use super::operate::OperateEngine;
use super::FIRST_BSP_LADDER;

/// 流式/批量共享的赋格引擎 v3 三轴组装器。公开 API 与原契约一致（ffi.rs 零改动）。
pub struct FugueEngineCore {
    /// 信号层（H⁰ 形态学 + groupoid 向心 confirm 计算内聚；§6B.4 内在耦合不强拆）。
    signal: SignalState,
    /// 信号层计数器累加器（finish 拷入 operate.res）。
    sig_res: SpiralResult,
    /// H¹ 操作引擎（D∞ word 处理器：层结构 + h/τ 原子）。
    operate: OperateEngine,
    cur_bar: i64,
    finished: bool,
}

impl FugueEngineCore {
    /// 初始化（`floor_ladder` 仅作结构递归基断言 = FIRST_BSP_LADDER，零操作参数引擎）。
    pub fn new(floor_ladder: usize) -> Result<Self, String> {
        if floor_ladder != FIRST_BSP_LADDER {
            return Err(format!(
                "赋格引擎 v3 是零操作参数引擎：floor_ladder 仅作结构递归基 = FIRST_BSP_LADDER={FIRST_BSP_LADDER}；得 {floor_ladder}"
            ));
        }
        Ok(FugueEngineCore {
            signal: SignalState::new(),
            sig_res: SpiralResult::default(),
            operate: OperateEngine::new(),
            cur_bar: 0,
            finished: false,
        })
    }

    /// 单 bar 推进。三轴数据流：信号层→桥接→operate→协调清链。
    pub fn step(&mut self, sig: &BarSig, flip_edge: &[Option<Direction>; MAX_LADDER]) {
        let bar = self.cur_bar;

        // ── 1. 信号层 → 群事件帧（H⁰ 形态学 + groupoid 向心 confirm，§6B.4 内聚）──
        let frame = self.signal.process(sig, flip_edge, bar, &mut self.sig_res);

        // ── 2/3. 三轴桥接 + H¹ 步进（operate 通过 trait 读两轴）──
        let outcome = {
            let morphology = MorphologyBridge::new(sig, &self.signal, &frame);
            let observe = ObserveBridge::new(&self.signal, &frame);
            self.operate.step(&morphology, &observe, bar, sig.close)
        };

        // ── 4. engine 协调：operate 消费的定位链 → observe 重置（ε 对称双向）──
        // 消费卖链：Long root C 清仓 / Short root F 入场
        if outcome.cleared_sell_source.is_some() || outcome.entered_sell_source.is_some() {
            self.signal.clear_located_sell();
        }
        // 消费买链：Long root F 入场 / Short root C 清仓
        if outcome.entered_buy_source.is_some() || outcome.cleared_buy_source.is_some() {
            self.signal.clear_located_buy();
        }

        self.cur_bar += 1;
    }

    /// 收尾（末 bar equity 补采样 + 清仓 + 拷信号层计数器）。幂等。
    pub fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        let last_bar = if self.cur_bar > 0 { Some(self.cur_bar - 1) } else { None };
        self.operate.finish(last_bar);
        let res = self.operate.result_mut();
        for k in 0..MAX_LADDER {
            res.n_arms_by_ladder[k] = self.sig_res.n_arms_by_ladder[k];
            res.n_fire_sell_by_ladder[k] = self.sig_res.n_fire_sell_by_ladder[k];
            res.n_fire_buy_by_ladder[k] = self.sig_res.n_fire_buy_by_ladder[k];
            res.n_breaks_by_ladder[k] = self.sig_res.n_breaks_by_ladder[k];
        }
    }

    pub fn n_trades(&self) -> usize {
        self.operate.result().trades.len()
    }

    pub fn result(&self) -> &FugueResult {
        self.operate.result()
    }

    /// 状态快照: (cur_bar, nav, long_units, short_units, n_active_voices)。
    pub fn snapshot(&self) -> (i64, f64, f64, f64, usize) {
        let (navv, lu, su, voices) = self.operate.snapshot();
        (self.cur_bar, navv, lu, su, voices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::INITIAL_CAPITAL;
    use crate::trading::types::{BspClass, BspEvent, LadderMask};

    const NO_FLIP: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];

    fn mk_bar(close: f64, buy1: u16, sell1: u16, max_ladder: u8, events: Vec<(usize, BspClass, bool, f64)>) -> BarSig {
        let mut sig = BarSig {
            close,
            buy1: LadderMask(buy1),
            sell1: LadderMask(sell1),
            max_ladder,
            ..Default::default()
        };
        if !events.is_empty() {
            let mut arr: [Vec<BspEvent>; MAX_LADDER] = Default::default();
            for (lad, class, confirmed, price) in events {
                arr[lad].push(BspEvent { class, seg_idx: lad as i64, confirmed, cs: None, zd: None, zg: None, price });
            }
            sig.bsp_events = Some(Box::new(arr));
        }
        sig
    }

    #[test]
    fn empty_bars_noop_conservation_holds() {
        let mut core = FugueEngineCore::new(FIRST_BSP_LADDER).expect("floor=FIRST_BSP");
        for _ in 0..5 {
            core.step(&mk_bar(100.0, 0, 0, 0, vec![]), &NO_FLIP);
        }
        core.finish();
        assert!(core.result().trades.is_empty(), "无事件不应产生 trade");
        assert!(
            (core.result().final_nav - INITIAL_CAPITAL).abs() < 1e-6,
            "无操作 final_nav 应 = 初始资本，得 {}",
            core.result().final_nav
        );
    }

    #[test]
    fn f_entry_builds_full_long_and_eod_conserves() {
        // bar0：segment(2) type1 Buy confirmed + move(3) type1 Buy candidate。
        // bar1：向心 confirm@3 + buy1@3 ⇒ F 建仓 @ move(L1)，全仓 Long（无硬编码拆分）。
        let mut core = FugueEngineCore::new(FIRST_BSP_LADDER).expect("floor=FIRST_BSP");
        core.step(
            &mk_bar(100.0, 0, 0, 3, vec![(2, BspClass::Buy1, true, 100.0), (3, BspClass::Buy1, false, 100.0)]),
            &NO_FLIP,
        );
        core.step(&mk_bar(100.0, 1 << 3, 0, 3, vec![]), &NO_FLIP);
        let (_, nav_after, long_u, short_u, _) = core.snapshot();
        // 满仓 total = 100000/100 = 1000 units，全 Long ⇒ long_u=1000、short_u=0。
        assert!((long_u - 1000.0).abs() < 1e-6, "满仓 1000 units 全 Long，得 {long_u}");
        assert_eq!(short_u, 0.0, "建仓后无空头（1/3 拆分是后续 τ 涌现，非建仓硬编码）");
        assert!((nav_after - INITIAL_CAPITAL).abs() < 1e-4, "F 建仓 NAV 中性");
        // flat 至 eod，同价进出 ⇒ final_nav = 初始资本。
        core.step(&mk_bar(100.0, 0, 0, 3, vec![]), &NO_FLIP);
        core.finish();
        assert!(
            (core.result().final_nav - INITIAL_CAPITAL).abs() < 1e-4,
            "同价进出 final_nav 应 = 初始资本，得 {}",
            core.result().final_nav
        );
    }
}
