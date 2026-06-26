//! Θ v0 共享数据类型（整数 tick 域 + 结构对象 + 信号/声部/订单）。
//!
//! ## bit-exact 第一原理：价格在整数 tick 域（reference-theta-v0.md:15）
//!
//! 所有价格量化为 `Tick`（`i64`）。浮点价格只在引擎入口 `quantize` 一次，内部所有
//! 比较/取 max/min/重叠判定都在整数域——整数运算无浮点约简顺序歧义，是 bit-exact
//! 的结构基础。MACD（`[L3]` 辅助）的浮点运算隔离在 classifier 内，按固定顺序约简。
//!
//! ## 平局裁决（reference-theta-v0.md:16）
//!
//! 每个结构对象携带 `source_index`（原始 K 序号）。所有平局按 `(timestamp,
//! source_index)` 升序裁决——见各对象的 `tie_key()`。

/// 整数 tick 价格（reference-theta-v0.md:15，全局量化）。
pub type Tick = i64;

/// 时间戳（毫秒或纳秒，单调递增；只用于排序，不参与价格运算）。
pub type Timestamp = i64;

/// 把浮点价格按 `tick_size` 量化为整数 tick（引擎边界唯一浮点→整数转换点）。
///
/// bit-exact：用 `round`（四舍五入到最近 tick），不是 floor/ceil——避免方向偏置。
/// 边界条件：`tick_size <= 0` 在 config 校验层拒绝（此处假设已校验 > 0）。
pub fn quantize(price: f64, tick_size: f64) -> Tick {
    (price / tick_size).round() as Tick
}

/// 几何方向（向上 / 向下）。结构识别层的方向，非操作极性。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
}

impl Direction {
    pub fn flip(self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }
}

/// 原始 OHLC bar（量化后，整数 tick 域）。
///
/// `untradable` 标记不可交易 bar（reference-theta-v0.md:53：缺 OHLC / `high<max(open,
/// close,low)` / `low>min(open,close,high)` / volume=0 / halt / limit flag）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bar {
    pub source_index: usize,
    pub timestamp: Timestamp,
    pub open: Tick,
    pub high: Tick,
    pub low: Tick,
    pub close: Tick,
    pub volume: i64,
    pub untradable: bool,
}

impl Bar {
    /// 平局裁决键（reference-theta-v0.md:16）：`(timestamp, source_index)` 升序。
    pub fn tie_key(&self) -> (Timestamp, usize) {
        (self.timestamp, self.source_index)
    }
}

/// 分型类型（reference-theta-v0.md:20）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractalKind {
    /// 顶分型：中 K 高低都**严格**高于左右（等价不成立）。
    Top,
    /// 底分型：中 K 高低都**严格**低于左右。
    Bottom,
}

/// 已确认分型（包含处理 + 第三根 K 收盘确认后）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fractal {
    pub kind: FractalKind,
    /// 分型中 K 对应的原始 K 序号。
    pub source_index: usize,
    pub timestamp: Timestamp,
    /// 分型极值价（顶=high，底=low），整数 tick。
    pub price: Tick,
}

/// 笔（新笔，reference-theta-v0.md:21）。顶/底分型不共用 K，间隔 ≥ config 规定根数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stroke {
    pub direction: Direction,
    pub start_index: usize,
    pub end_index: usize,
    pub start_price: Tick,
    pub end_price: Tick,
}

/// 线段（67 课特征序列法，reference-theta-v0.md:22）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub direction: Direction,
    pub start_index: usize,
    pub end_index: usize,
    pub start_price: Tick,
    pub end_price: Tick,
}

/// 中枢（reference-theta-v0.md:23,33）。前三连续完成次级别走势 A,B,C。
///
/// `zd=max(low_A,low_B,low_C)`，`zg=min(high_A,high_B,high_C)`；闭区间 `[zd,zg]`，
/// `zd<=zg` 成立即中枢成立。`gg=max(highs)`/`dd=min(lows)` 外包络。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Center {
    /// 核心区间下沿 ZD（闭区间）。
    pub zd: Tick,
    /// 核心区间上沿 ZG（闭区间）。
    pub zg: Tick,
    /// 外包络下沿 DD。
    pub dd: Tick,
    /// 外包络上沿 GG。
    pub gg: Tick,
    pub start_index: usize,
    pub end_index: usize,
}

/// 走势类型 / Move（次级别走势的递归构造单元，reference-theta-v0.md:29）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    /// 趋势（≥2 同向中枢）。
    Trend,
    /// 盘整（1 中枢）。
    Consolidation,
}

/// 未完成尾部（reference-theta-v0.md:25）。显式保存，不输出为 confirmed。
///
/// 含方向/起点/当前极值/确认条件——bit-exact 要求尾部状态完整可序列化（区别于
/// confirmed 结构）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingTail {
    PendingFractal {
        kind: FractalKind,
        source_index: usize,
        extreme: Tick,
    },
    PendingStroke {
        direction: Direction,
        start_index: usize,
        current_extreme: Tick,
    },
    PendingSegment {
        direction: Direction,
        start_index: usize,
        current_extreme: Tick,
    },
    AliveCenter {
        zd: Tick,
        zg: Tick,
        start_index: usize,
    },
    PendingMove {
        direction: Direction,
        start_index: usize,
        current_extreme: Tick,
    },
}

/// 买卖点类型 bit-vector（契约锚 `Origin.BspClassification.no_exclusive_trichotomy` 非互斥裁定）。
///
/// ★关键（`Origin.BspClassification.no_exclusive_trichotomy`）：买卖点**不是互斥三分**——2B/3B
/// 可在同一点重合（maimai.md:170）。故用 bit-vector（标签集），不是 sum type。`b1/b2/b3` 各为
/// 独立 bool，一个点可同时是 `[2B,3B]`。这是 Origin BspClassification 核心诚实裁定的 Rust 镜像。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BspBits {
    pub buy1: bool,
    pub buy2: bool,
    pub buy3: bool,
    pub sell1: bool,
    pub sell2: bool,
    pub sell3: bool,
}

/// 买卖向 `Side`（port `Origin.BspClassification.Side`：long/short）。
///
/// 缠论买卖点的方向语境（§10.1 买卖对偶）：`Long` = 买点侧（底背驰/向上离开中枢之上）；
/// `Short` = 卖点侧（顶背驰/向下离开中枢之下）。卖点判据要求 `Side::Short`（见 closed_loop/sell.rs）。
/// ★这是判据**方向特化参数**，非持仓方向（`Pos`）——背驰力度判据本身方向无关（顶/底背驰同构），
/// `Side` 只在 side/trend 语境区分买卖（Lean `type1_buy_sell_share_divergence`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Long,
    Short,
}

/// 持仓方向（契约锚 `Origin.FullDefinitionStrategy` 动作类前件 `Pos`：long/short/flat）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pos {
    Long,
    Short,
    Flat,
}

/// 信号方向（契约锚 `Origin.BspClassification` 信号前件 `Sig`：buySide/sellSide/none）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sig {
    BuySide,
    SellSide,
    None,
}

/// 严格动作（契约锚 `Origin.FullDefinitionStrategy.ActionClass` 投影，7 构造子，含 wait/hold 区分）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrictAction {
    Buy,
    Sell,
    Add,
    Reduce,
    /// 持仓不动（仅 long/short + none 合法）。
    Hold,
    Close,
    /// 空仓观望（仅 flat 合法）。
    Wait,
}

/// 订单（strategy 子模块输出，Θ_exec 执行单元）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    pub action: StrictAction,
    /// 数量（整数 lot；`qty<=0` 不交易，reference-theta-v0.md:47）。
    pub qty: i64,
    /// 触发该订单的 bar（执行延迟后的成交 bar，reference-theta-v0.md:50）。
    pub exec_index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantize_rounds_to_nearest_tick() {
        // 1e-8 tick：1.000_000_004 → 100_000_000 tick（四舍五入）。
        assert_eq!(quantize(1.000_000_004, 1e-8), 100_000_000);
        assert_eq!(quantize(1.000_000_006, 1e-8), 100_000_001);
    }

    #[test]
    fn direction_flip_involutive() {
        assert_eq!(Direction::Up.flip(), Direction::Down);
        assert_eq!(Direction::Up.flip().flip(), Direction::Up);
    }

    #[test]
    fn bsp_bits_non_exclusive_2b3b_coincide() {
        // BSP.lean 核心：2B/3B 可重合（非互斥三分）。bit-vector 必须能表达 [2B,3B]。
        let bits = BspBits {
            buy2: true,
            buy3: true,
            ..Default::default()
        };
        assert!(bits.buy2 && bits.buy3);
    }
}
