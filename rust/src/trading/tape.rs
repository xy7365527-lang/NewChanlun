//! SignalTape — 信号磁带（`fugue_version_i.BarSignalI` 的 Rust 形态）。
//!
//! 一次构造（PyO3 边界 marshal 一次），多变体共享只读引用——compute-once 原则。
//!
//! ## D3 行的表示（v2 §7 的稀疏化）
//! 设计 §7 把 dir_row/run_anchor 列为每 bar 磁带行。本实现取**稀疏翻转行**：
//! `dir_flips = [(bar, ladder, direction)]`（方向行只在翻转 bar 变化），runner
//! 维护滚动状态数组——逐 bar 视图与密集行逐位等价，内存从 O(N×11) 降到
//! O(翻转数)（BRN 2.4M 密集形态 ≈450MB，翻转数 ~10⁴）。
//! `run_anchor[k]` 由翻转行直接导出（= 该层最近翻转 bar）——anchor 语义 =
//! 方向 run 的**信号观测起点**（确认滞后与全系统事件时间口径一致）。
//! run_high（FatigueGate 路径(1) 依赖）保留密集可选列（仅 rev_gate 变体消费，
//! 当前信号层不产出 → None）。
//!
//! NaN 纪律（T7 陷阱）：close 含 NaN 在构造期拒绝；事件 cs 存在时 zd/zg 必须
//! 同时存在（CenterBook 算术前提）。

use super::types::*;
use crate::stroke::Direction;

/// 单 bar 信号。布尔行压缩为位掩码；事件行 Option<Box<…>> 表达稀疏性
/// （绝大多数 bar 无事件，None ⇔ Python 共享单例 NO_LADDER_EVENTS）。
#[derive(Debug, Default)]
pub struct BarSig {
    pub close: f64,
    pub buy1: LadderMask,
    pub sell1: LadderMask,
    pub sell_any: LadderMask,
    pub buy_any: LadderMask,
    pub max_ladder: u8,
    pub type2_buy: bool,
    pub bsp_events: Option<Box<[Vec<BspEvent>; MAX_LADDER]>>,
    pub div_events: Option<Box<[Vec<DivEvent>; MAX_LADDER]>>,
    pub up_move_settled: LadderMask,
}

/// 完整磁带。
#[derive(Debug, Default)]
pub struct SignalTape {
    pub bars: Vec<BarSig>,
    /// D3 方向行（稀疏翻转，bar 升序）。None = 信号层未产出（capability guard）。
    pub dir_flips: Option<Vec<(i64, u8, Direction)>>,
    /// D3 run 高点行（密集 n×MAX_LADDER 展平）。None = 未产出。
    pub run_high: Option<Vec<f64>>,
    /// 趋势态行（稀疏翻转，bar 升序；(bar, ladder, is_trend)）。该层尾 move
    /// kind 的翻转流——is_trend ⟺ kind==Trend（≥2 同向中枢，17课趋势定义）。
    /// None = 信号层未产出（rev_cycle=Cycle38 的 capability guard 依赖）。
    pub trend_flips: Option<Vec<(i64, u8, bool)>>,
}

impl SignalTape {
    /// 能力守卫（run_organic 开头逐字对应）：事件磁带非全空。
    pub fn has_bsp_events(&self) -> bool {
        self.bars.iter().any(|b| b.bsp_events.is_some())
    }

    pub fn has_div_events(&self) -> bool {
        self.bars.iter().any(|b| b.div_events.is_some())
    }

    pub fn has_dir_rows(&self) -> bool {
        self.dir_flips.is_some()
    }

    pub fn has_run_high(&self) -> bool {
        self.run_high.is_some()
    }

    pub fn has_trend_rows(&self) -> bool {
        self.trend_flips.is_some()
    }
}
