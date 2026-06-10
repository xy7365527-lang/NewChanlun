//! SignalTape — 信号磁带（`fugue_version_i.BarSignalI` 的 Rust 形态）。
//!
//! 一次构造（PyO3 边界 marshal 一次），多变体共享只读引用——compute-once 原则。
//! v2 新增行（D3：dir_row/run_anchor/run_high）默认 None：当前磁带
//! （organic_signals.py）不产出 ⇒ 依赖它们的配置轴在 runner 入口被
//! capability guard 拒绝（fail-fast，声明=能力）。
//!
//! NaN 纪律（T7 陷阱）：close 含 NaN 在构造期拒绝；事件 cs 存在时 zd/zg 必须
//! 同时存在（CenterBook 算术前提，center_book.rs 注释的边界执行点）。

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
    // ── v2 D3 行（当前磁带恒 None）──
    pub dir_row: Option<Box<[Option<Direction>; MAX_LADDER]>>,
    pub run_anchor: Option<Box<[i64; MAX_LADDER]>>,
    pub run_high: Option<Box<[f64; MAX_LADDER]>>,
}

/// 完整磁带。
#[derive(Debug, Default)]
pub struct SignalTape {
    pub bars: Vec<BarSig>,
}

impl SignalTape {
    /// 能力守卫（run_organic 开头逐字对应）：事件磁带非全空。
    pub fn has_bsp_events(&self) -> bool {
        self.bars.iter().any(|b| b.bsp_events.is_some())
    }

    pub fn has_div_events(&self) -> bool {
        self.bars.iter().any(|b| b.div_events.is_some())
    }

    pub fn has_dir_row(&self) -> bool {
        self.bars.iter().any(|b| b.dir_row.is_some())
    }

    pub fn has_run_anchor(&self) -> bool {
        self.bars.iter().any(|b| b.run_anchor.is_some())
    }

    pub fn has_run_high(&self) -> bool {
        self.bars.iter().any(|b| b.run_high.is_some())
    }
}
