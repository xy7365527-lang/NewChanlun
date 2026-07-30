//! **H⁰ 形态学轴桥接**（MorphologyBridge impl MorphologyAxis）：包装 BarSig + SignalState + MorphologyState。
//!
//! GUARD-ROLE: t-engine-accounting-basis——名分：现役（详见 `fugue_v3/mod.rs`
//! 头部 GUARD-ROLE 块，#762 C7-E3 核定）。
//!
//! 用户裁决（2026-06-17）：方向涌现从信号层 nf fire 驱动——nf_sell[k] fire（k 级别 type1 sell
//! confirmed = 顶背驰确认）⟹ k 级别上涨完成、下跌开始 ⟹ dir_state[k] = Down；nf_buy[k] 反之。
//! 信号层 `SignalState.dir_state`（外部 K 线层 flip_edge 驱动）保留用于其他模块兼容；
//! fugue_v3 的形态学方向态由 `MorphologyState` 自主维护（每 bar 在 signal.process 后 emerge 一次）。
//!
//! 根方向 = 最高涌现级别走势方向（`root_direction` 默认实现从 emergent_ceiling-1 向下扫）。
//!
//! ## 认识论等级
//! - MorphologyState 涌现规则（nf→翻转）：**L0**（缠论 type1 BSP confirmed = 走势完成的定义）。
//! - MorphologyBridge 桥接 view：**L1**（trait 接口转发，无新计算）。

use crate::spiral::signal::{GroupEventFrame, SignalState};
use crate::stroke::Direction;
use crate::trading::tape::BarSig;
use crate::trading::types::MAX_LADDER;

use super::axis::MorphologyAxis;
use super::{PENDING_LO, SUB_COST_MIN_OBS, SUB_COST_Q};

/// H⁰ 形态学持久态（每级别方向态 + 段锚，跨 bar 持有）。
///
/// 涌现规则（每 bar engine 调 `emerge(&frame, bar)`，operate 步进**前**）：
/// - `frame.nf_sell[k].is_some()` ⟹ k 级别上涨完成 ⟹ `dir_state[k] = Down`，`anchor_state[k] = bar`
/// - `frame.nf_buy[k].is_some()` ⟹ k 级别下跌完成 ⟹ `dir_state[k] = Up`，`anchor_state[k] = bar`
/// - 二者同 bar 共触 ⟹ 后到者覆盖（实际 nf 在向心 confirm 中互斥，此情形不出现）。
///
/// 缠论根据：type1 BSP confirmed = 该级别走势已完成（缠师 17/27 课原文），nf fire 是
/// 向心回溯贯通的 confirm 事件（T49），与走势完成时刻等同。
#[derive(Debug)]
pub struct MorphologyState {
    pub dir_state: [Option<Direction>; MAX_LADDER],
    pub anchor_state: [i64; MAX_LADDER],
}

impl Default for MorphologyState {
    fn default() -> Self {
        MorphologyState::new()
    }
}

impl MorphologyState {
    pub fn new() -> Self {
        MorphologyState {
            dir_state: [None; MAX_LADDER],
            anchor_state: [-1; MAX_LADDER],
        }
    }

    /// 单 bar 方向涌现：消费 frame.nf_sell/nf_buy 翻转级别走势方向。
    /// 调用时机：engine.step 中 signal.process 之**后**、operate.step 之**前**——保证 operate
    /// 通过 MorphologyAxis.direction()/root_direction() 读到本 bar 已更新的方向态。
    pub fn emerge(&mut self, frame: &GroupEventFrame, bar: i64) {
        for k in PENDING_LO..MAX_LADDER {
            // nf_sell[k] fire ⟹ k 级别上涨走势 confirmed 完成 ⟹ 方向翻为 Down（新一段下跌）。
            if frame.nf_sell[k].is_some() {
                self.dir_state[k] = Some(Direction::Down);
                self.anchor_state[k] = bar;
            }
            // nf_buy[k] fire ⟹ k 级别下跌走势 confirmed 完成 ⟹ 方向翻为 Up（新一段上涨）。
            if frame.nf_buy[k].is_some() {
                self.dir_state[k] = Some(Direction::Up);
                self.anchor_state[k] = bar;
            }
        }
    }
}

/// H⁰ 形态学轴桥接（轻量 view，持引用零拷贝）。
pub struct MorphologyBridge<'a> {
    sig: &'a BarSig,
    signal: &'a SignalState,
    frame: &'a GroupEventFrame,
    morph: &'a MorphologyState,
}

impl<'a> MorphologyBridge<'a> {
    pub fn new(
        sig: &'a BarSig,
        signal: &'a SignalState,
        frame: &'a GroupEventFrame,
        morph: &'a MorphologyState,
    ) -> Self {
        MorphologyBridge {
            sig,
            signal,
            frame,
            morph,
        }
    }
}

impl MorphologyAxis for MorphologyBridge<'_> {
    fn emergent_ceiling(&self) -> usize {
        self.frame.max_l
    }

    fn buy1(&self, ladder: usize) -> bool {
        self.sig.buy1.get(ladder)
    }

    fn sell1(&self, ladder: usize) -> bool {
        self.sig.sell1.get(ladder)
    }

    fn direction(&self, ladder: usize) -> Option<Direction> {
        // 方向态来自 MorphologyState（nf 驱动涌现，与缠论 type1 BSP confirm 同步），
        // 不读 SignalState.dir_state（后者由外部 K 线层 flip_edge 驱动，保留兼容其他模块）。
        self.morph.dir_state[ladder]
    }

    fn anchor(&self, ladder: usize) -> i64 {
        self.morph.anchor_state[ladder]
    }

    fn theta(&self, sub: usize) -> Option<f64> {
        // 成本门 N4：复用 unn DepthRef θ 机件（None=参照集 < min_obs，势不可测）。
        self.signal
            .depth
            .theta(sub, None, SUB_COST_Q, SUB_COST_MIN_OBS)
    }
}
