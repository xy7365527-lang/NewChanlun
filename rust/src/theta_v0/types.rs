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
/// `zd<zg`（**严格**）成立即中枢成立——单点核心 `zd==zg` **不**成立（#321 裁定，2026-07-26
/// 用户裁决：与 #290 裁定 B / Python 严格口径三方对齐，Lean/Python/Rust 统一向严格看齐；原弱
/// 口径为实现者自选、无教义依据，原文 17 课定义/20 课公式未涉及端点口径，22 课 Q&A
/// `docs/chanlun/text/blog/022-第22课.md:514` 单点中枢之问，缠师答的是级别谬误——只说明该例子
/// 只构成 1 分钟中枢的延续，未答单点 `[ZD,ZG]` 边界本身是否成立）。`gg=max(highs)`/
/// `dd=min(lows)` 外包络。
///
/// ⚠边界声明作废：中枢**成立**落点 ZD==ZG —— Lean `centerHolds` 为 `ZD ≤ ZG`（弱，单点成立）/
/// Rust 本实装严格（不成立，#321 裁定）；**该落点的 Lean↔Rust 对齐声明在本边界上作废，此边界不作
/// 机械锁用**（不得据本实装断言 Lean 侧行为，亦不得据 Lean `centerHolds` 反推本实装期望值）；
/// 其余落点对齐声明不受影响。Lean 侧跟进留对方线。
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

/// #542：三类入口证书的生产者身份。
///
/// 这是只读归因载荷：由 `judge_third_cert` 一次签发，随既有 BSP 证书进入候选与成交快照；
/// 不参与六 bit 分类、排序、相等或任何交易判定。`level/source_index/side` 仍由候选本体承载。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThirdClassEntryIdentity {
    pub center: Center,
    pub leave_interval: (usize, usize),
    pub retest_interval: (usize, usize),
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
#[derive(Clone, Copy, Default)]
pub struct BspBits {
    pub buy1: bool,
    pub buy2: bool,
    pub buy3: bool,
    pub sell1: bool,
    pub sell2: bool,
    pub sell3: bool,
    /// #542：随证书直传的三类完整身份；不是第七个 bit，不进任何结构/交易语义。
    ///
    /// 置于既有证书载体而非成交侧旁路查询，保证实际选中的 candidate/order 才能把身份带进 fill。
    #[doc(hidden)]
    pub third_class_entry: Option<ThirdClassEntryIdentity>,
}

/// #542 schema-only 铁律：归因载荷不改变六 bit 的相等关系。
impl PartialEq for BspBits {
    fn eq(&self, other: &Self) -> bool {
        self.buy1 == other.buy1
            && self.buy2 == other.buy2
            && self.buy3 == other.buy3
            && self.sell1 == other.sell1
            && self.sell2 == other.sell2
            && self.sell3 == other.sell3
    }
}

impl Eq for BspBits {}

/// 保持历史 Debug 字节形状：新增归因载荷不进入任何既有 digest/报告旧字段。
impl std::fmt::Debug for BspBits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BspBits")
            .field("buy1", &self.buy1)
            .field("buy2", &self.buy2)
            .field("buy3", &self.buy3)
            .field("sell1", &self.sell1)
            .field("sell2", &self.sell2)
            .field("sell3", &self.sell3)
            .finish()
    }
}

impl BspBits {
    /// 买入方向确认 `Conf^+_e(x) = ⋁_{i=1}^{3} B_{i,e}(x) = B1 ∨ B2 ∨ B3`。
    ///
    /// ## 结果包（六要素）
    /// - **结论**：买入方向确认谓词，对买卖点向量 `b_ℓ` 的买侧三分量取析取 ⋁。
    /// - **定义依据**：spec `2026-06-28-recursive-complete-classification-bsp-pdf-extract.md`
    ///   P5 §6 方框 `Conf^+_e(x)=⋁_{i=1}^3 B_{i,e}(x)`（line 283）+ 七链 环1（line 1167）。
    ///   输入 `BspBits` 的 buy1/buy2/buy3 三 bool 直接对应 B_{1,e}/B_{2,e}/B_{3,e}。
    /// - **边界条件**：用 ⋁（析取，**非互斥**——P4 §5「不要求六个买卖点互斥，可重合」line 233）；
    ///   任一买点成立即确认。若改用 ∧（合取）或假设互斥三选一，则结论翻转——确认条件收紧、
    ///   64 类完全分类塌缩。三分量全 false ⟹ 买侧未确认。
    /// - **下游推论**：区间套证书 `N^δ_{ℓ↓e}` 基例（ℓ=e，δ=+1/Long）取本谓词（P5 §6 分段函数）；
    ///   与 [`BspBits::conf_minus`] 合取覆盖 [`crate::theta_v0::classifier::nest::confirm`] 的 Λ≠∅。
    /// - **谱系引用**：第三类边界谱系（MEMORY: theta-v0-type3-boundary，reference §36 含等号 vs Lean
    ///   严格<）在 B3 谓词层（`bsp::endpoint_to_bsp`）已与既有 rust `>=` 一致结算；本析取层只消费已置位
    ///   的 buy3 bool，**不重判边界**，故不引入新等号冲突。互斥责任分层谱系见 P4 §5 结果包（不互斥转移
    ///   到角色层/解释器层）。
    /// - **影响声明**：新增买侧确认析取；[`crate::theta_v0::classifier::nest::confirm`] 改为委托本方法
    ///   +`conf_minus`（单一来源，消除重复析取）。L0 操作语义，**不**声明择时 alpha（买卖点 v1 全窗 8/8
    ///   L3 已否证）。
    pub fn conf_plus(&self) -> bool {
        self.buy1 || self.buy2 || self.buy3
    }

    /// 卖出方向确认 `Conf^-_e(x) = ⋁_{i=1}^{3} S_{i,e}(x) = S1 ∨ S2 ∨ S3`。
    ///
    /// 镜像 [`BspBits::conf_plus`]（spec P5 §6 line 285；δ=-1/Short 方向）。六要素见 `conf_plus`，
    /// 此为其卖侧对偶（买卖对偶 §10.1）。
    pub fn conf_minus(&self) -> bool {
        self.sell1 || self.sell2 || self.sell3
    }

    /// 方向化确认 `Conf^δ_e(x)`（spec P5 §6 区间套证书基例 ℓ=e）：`δ=+1`(Long)→`Conf^+`，
    /// `δ=-1`(Short)→`Conf^-`。区间套证书 `N^δ_{ℓ↓e}` 按方向 δ 选取本谓词作基例。
    pub fn confirm_side(&self, side: Side) -> bool {
        match side {
            Side::Long => self.conf_plus(),
            Side::Short => self.conf_minus(),
        }
    }

    /// 64 类完全分类索引 `b_ℓ ∈ {0,1}^6 → 0..64`（spec P4 §5：`Σ_{u∈{0,1}^6} 1[b_ℓ=u]=1`）。
    ///
    /// bit 权重 `(B1,B2,B3,S1,S2,S3) = (1,2,4,8,16,32)`，是 `{0,1}^6 ↔ ℤ_64` 的**双射**——这是
    /// 「64 类完全分类、指示函数和恒为 1」(P4 §5 line 241) 的可计算落点。**非互斥**：6 位可任意组合
    /// （含多位同 1），覆盖全部 2^6=64 状态，故是完全分类而非互斥子集。与 [`BspBits::from_class_index`]
    /// 互逆（round-trip 见单测）。
    pub fn class_index(&self) -> u8 {
        (self.buy1 as u8)
            | (self.buy2 as u8) << 1
            | (self.buy3 as u8) << 2
            | (self.sell1 as u8) << 3
            | (self.sell2 as u8) << 4
            | (self.sell3 as u8) << 5
    }

    /// 64 类索引 `0..64 → b_ℓ ∈ {0,1}^6`（[`BspBits::class_index`] 的逆，双射另一半）。
    ///
    /// 定义域 `idx ∈ 0..64`（{0,1}^6 的 64 个元素）。`idx >= 64` 越界（高于 6 位无意义）⟹ debug 断言
    /// 失败（fail-fast 边界校验，非静默截断）。
    pub fn from_class_index(idx: u8) -> Self {
        debug_assert!(idx < 64, "BspBits 64 类索引必须 ∈ 0..64，收到 {idx}");
        BspBits {
            buy1: idx & 1 != 0,
            buy2: idx & 2 != 0,
            buy3: idx & 4 != 0,
            sell1: idx & 8 != 0,
            sell2: idx & 16 != 0,
            sell3: idx & 32 != 0,
            third_class_entry: None,
        }
    }
}

/// 买卖向 `Side`（port `Origin.BspClassification.Side`：long/short）。
///
/// 缠论买卖点的方向语境（§10.1 买卖对偶）：`Long` = 买点侧（底背驰/向上离开中枢之上）；
/// `Short` = 卖点侧（顶背驰/向下离开中枢之下）。卖点判据要求 `Side::Short`（Lean
/// `Origin/SellPointRecog.IsType1Sell`；rust 卖侧 port closed_loop/sell.rs 已于 #181 下线）。
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

    #[test]
    fn conf_plus_is_buy_side_disjunction() {
        // Conf^+ = B1 ∨ B2 ∨ B3（spec P5 §6 line 283）：任一买点成立即确认，卖点不参与。
        assert!(!BspBits::default().conf_plus(), "全零 ⟹ 买侧未确认");
        for f in [
            BspBits { buy1: true, ..Default::default() },
            BspBits { buy2: true, ..Default::default() },
            BspBits { buy3: true, ..Default::default() },
        ] {
            assert!(f.conf_plus(), "任一买点 ⟹ Conf^+");
        }
        // 仅卖点置位 ⟹ Conf^+ 假（方向隔离）。
        let only_sell = BspBits { sell1: true, sell2: true, sell3: true, ..Default::default() };
        assert!(!only_sell.conf_plus());
        assert!(only_sell.conf_minus());
    }

    #[test]
    fn conf_minus_is_sell_side_disjunction() {
        // Conf^- = S1 ∨ S2 ∨ S3（spec P5 §6 line 285），买卖对偶镜像。
        assert!(!BspBits::default().conf_minus());
        for f in [
            BspBits { sell1: true, ..Default::default() },
            BspBits { sell2: true, ..Default::default() },
            BspBits { sell3: true, ..Default::default() },
        ] {
            assert!(f.conf_minus());
        }
        let only_buy = BspBits { buy1: true, ..Default::default() };
        assert!(!only_buy.conf_minus());
        assert!(only_buy.conf_plus());
    }

    #[test]
    fn conf_disjunction_non_exclusive_coexist() {
        // ★不互斥（P4 §5 line 233）：买侧多位 + 卖侧多位可同时置位，两方向确认同真。
        let coexist = BspBits {
            buy2: true,
            buy3: true,
            sell1: true,
            ..Default::default()
        };
        assert!(coexist.conf_plus() && coexist.conf_minus());
    }

    #[test]
    fn confirm_side_selects_direction() {
        // Conf^δ_e：Long → Conf^+，Short → Conf^-（区间套证书方向化基例）。
        let buy = BspBits { buy1: true, ..Default::default() };
        let sell = BspBits { sell3: true, ..Default::default() };
        assert!(buy.confirm_side(Side::Long) && !buy.confirm_side(Side::Short));
        assert!(sell.confirm_side(Side::Short) && !sell.confirm_side(Side::Long));
    }

    #[test]
    fn class_index_from_index_round_trip_64_complete() {
        // ★64 类完全分类（P4 §5 line 241：Σ_{u∈{0,1}^6} 1[b=u]=1）：
        // class_index 是 {0,1}^6 ↔ 0..64 双射 ⟹ 枚举全 64 索引 round-trip 还原且索引各异
        // = 状态空间被 64 类无遗漏无重复覆盖（指示函数和恒为 1 的可计算见证）。
        use std::collections::BTreeSet;
        let mut seen = BTreeSet::new();
        for idx in 0u8..64 {
            let bits = BspBits::from_class_index(idx);
            assert_eq!(bits.class_index(), idx, "round-trip 还原 idx={idx}");
            assert!(seen.insert(idx), "索引 {idx} 唯一（无重复类）");
        }
        assert_eq!(seen.len(), 64, "恰好 64 类，完全分类");
    }

    #[test]
    fn class_index_bit_weights_exact() {
        // bit 权重 (B1,B2,B3,S1,S2,S3)=(1,2,4,8,16,32)。
        assert_eq!(BspBits::default().class_index(), 0);
        assert_eq!(BspBits { buy1: true, ..Default::default() }.class_index(), 1);
        assert_eq!(BspBits { sell3: true, ..Default::default() }.class_index(), 32);
        let all = BspBits {
            buy1: true,
            buy2: true,
            buy3: true,
            sell1: true,
            sell2: true,
            sell3: true,
            third_class_entry: None,
        };
        assert_eq!(all.class_index(), 63, "全 1 ⟹ 第 63 类（64 类的最后一类）");
    }
}
