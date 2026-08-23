//! GUARD-ROLE: organic-fugue-v2-trading-layer——名分：现役（详见 `trading/mod.rs` 头部 GUARD-ROLE 块，#763 C7-E4 核定；零删除/零移入 legacy/）。
//!
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
use crate::theta_v0::classifier::{TurnClassRow, XzdSecondCandidate};

/// 单 bar 信号。布尔行压缩为位掩码；事件行 Option<Box<…>> 表达稀疏性
/// （绝大多数 bar 无事件，None ⇔ Python 共享单例 NO_LADDER_EVENTS）。
#[derive(Debug, Default, Clone)]
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
    /// turn_class 投影行（#1195；稀疏注解行 `(bar, ladder, class, evidence)`，与
    /// `trend_flips` 行同构）。每级标签一行——`class` = [`crate::theta_v0::classifier::TurnClassKind`] 四类，
    /// `evidence` 仅 `XiaozhuandaCandidate` 携带（`third_src` + `second_class`）。
    /// None = 分类投影层未产出（零行为变化；L-重情况二程序臂的 capability guard）。
    ///
    /// ★#1198 传输位（同 `BspEvent.seg_idx` 先例）：消费接线已并入磁带——#1202 风控臂
    /// （行式标注三件）与 #1208 ②件 053:28 二卖臂（候选事件标签门）读本行。
    pub turn_class_rows: Option<Vec<TurnClassRow>>,
    /// xzd_second 候选事件行（#1208 ②件；稀疏行，bar 升序，`source_index` = 事件 bar）。
    /// zero six-bit、不进 `BspBits`、不作终端背书（044:30 类型封锁保持）。FSM 消费前
    /// 须过 `turn_class_rows` 标签门（同 ladder `XiaozhuandaCandidate` 且 `turn_extreme`
    /// 一致；#1195 三层分工「判据产点宽、链定类窄」）。
    /// None = 事件层未产出（零行为变化；053:28 二卖臂的 capability guard）。
    pub xzd_second_candidates: Option<Vec<XzdSecondCandidate>>,
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

    /// turn_class 投影行的能力守卫（#1195 第二块；L-重情况二程序臂的 capability
    /// guard 用，见 [`SignalTape::turn_class_rows`] 传输位注）。
    #[allow(dead_code)]
    pub fn has_turn_class_rows(&self) -> bool {
        self.turn_class_rows.is_some()
    }

    /// xzd_second 候选事件行的能力守卫（#1208 ②件；053:28 二卖臂的 capability
    /// guard——None ⟹ 事件层未产出 ⟹ 零行为变化）。
    #[allow(dead_code)]
    pub fn has_xzd_second_candidates(&self) -> bool {
        self.xzd_second_candidates.is_some()
    }
}
