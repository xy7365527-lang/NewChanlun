# 有机赋格交易逻辑 Rust 化设计 — 类型系统作为递归正则化的编译期守卫

> 状态：设计稿（不含实现）。设计精确到 struct/enum/trait 字段级，可直接抄进 .rs 文件。
> 上游：`analysis/organic_fugue_design.md`（框架设计）、`analysis/organic_fugue.py`（Python 原型，本设计的逐位等价目标）。
> 认识论等级：类型翻译为 **L0**（Python 原型语义的结构保持映射）；性能预期引用既有 L2 实测
> （ladder2 成本墙 17s→402s、引擎 Rust 化 E7.5×/I14.6×）；bit-exact 等价为 **待验证契约**（§7）。
> 日期：2026-06-10。

---

## 0. 立场：为什么 Rust 化不只是性能

Python 原型用注释和运行时断言表达的约束，Rust 用类型系统表达——**编译器成为
no-patch-mentality 的机械执行者**。被 Rust 逼着澄清的，正是缠论操盘的严格性：

| 缠论/账本约束 | Python 表达（概率性遵守） | Rust 表达（编译期不可违反） |
|--------------|--------------------------|---------------------------|
| rev 腿无中枢锚（§4b：价格已离开中枢） | `anchor=None` 约定 + docstring | `LegAnchor::SegmentScale` 变体——"带中枢锚的 rev 腿"**不可表示** |
| 两阶段守恒律由腿 open 时刻冻结 | `was_earning: bool` 字段 | `ConservationLaw` 枚举存进腿记录，`close_diff` 对其 `match` 穷举——漏掉一条守恒律编译不过 |
| earning 后 cost_basis 锁 0 | `cost_basis = 0.0` 赋值 + 注释 | `LedgerPhase::EarningShares` 变体**没有 cost_basis 字段**——挣股数阶段改写成本是编译错误 |
| 信号 kind 不是 bool | `e[0] == "type1"` 字符串比对 | `BspKind::{Type1,Type2,Type3}` 枚举 `match` 穷举——新增 kind 时所有消费点编译报错 |
| 仓位只在 _LONG 态存在 | `pos: OrganicLedger \| None` + 每处判 None | `RunnerState::Long { ledger, voices, … }`——FLAT 态读仓位**不可表示** |
| profit 只对已闭合腿有定义 | `if self.is_open: return 0.0` | `OpenCycle` / `ClosedCycle` 两个类型，`profit()` 只在 `ClosedCycle` 上存在 |
| 槽空间 main/osc/rev 不可混淆 | `key = ladder + 100/200` 整数算术 | `SlotKey { ladder, leg: LegClass }`——offset 算术消失，`key % 100` 类 bug 不可表示 |
| futures 模式不在本实现 | `raise NotImplementedError` 运行时 | `MarketMode` 枚举只有 `Stock` 变体——futures 不是错误分支，是**不在类型里** |

**诚实声明（声明膨胀禁止，090号）**：用户提出的"中枢 lifetime 和短差腿 lifetime 绑定→
中枢死了短差腿的引用编译不过"在 Rust 借用检查器的字面意义上**不可实现**——中枢死亡是
运行时市场事件（数据驱动），不是作用域退出；跨 bar 持有 `&LiveCenter` 引用与 `&mut CenterBook`
逐 bar 更新冲突。本设计的严格形式是：

- **编译期守卫**覆盖*结构性*非法状态（上表全部）——非法状态不可表示（make illegal states unrepresentable）；
- **运行时不变量**（中枢死亡→强制回补、INV-1 真实股数域）保持运行时检查，但从 Python 的
  "约定 + 可选断言"升级为 Rust 的**穷举 match + 类型化锚点**：`LegAnchor::Center { seg_start, .. }`
  对照 `CenterBook::is_dead(ladder, seg_start)`，分支不可静默遗漏。

声明的能力 = 上述两层，不多不少。

---

## 1. 模块划分与依赖图

```
rust/src/trading/
├── mod.rs          模块出口 + 公共 re-export
├── types.rs        交易类型基座（Ladder/SlotKey/事件/锚点/守恒律）
├── config.rs       OrganicConfig + 变体表 + MarketMode/StopMode
├── tape.rs         SignalTape（BarSignalI 的 Rust 形态）+ 边界解析
├── center_book.rs  中枢生命周期账本（市场性质，跨 trade）
├── fatigue.rs      FatigueMonitor（41课守门员）
├── allocator.rs    SizeAllocator（40课结构规模）
├── ledger.rs       OrganicLedger + 两阶段守恒律 + ShortDiffCycle 移植
├── lou.rs          LevelOperatingUnit（38课程式 FSM，T1-T7）
└── runner.rs       OrganicRunner（主循环：ARM/区间套入场/master 循环/voice 调度）

依赖方向（单向，无环）：

  crate::buysellpoint{BspKind,Side}  crate::divergence{DivKind}     ← 引擎枚举，零重复定义
            │                              │
            └────────────┬─────────────────┘
                      types.rs
                     ╱    │    ╲
              config.rs  tape.rs  center_book.rs
                  │        │      ╱        │
                  │     fatigue.rs    allocator.rs
                  │        │               │
                  └──── ledger.rs ─────────┤
                           │               │
                         lou.rs ───────────┘
                           │
                        runner.rs
                           │
                    lib.rs（PyO3 绑定：OrganicTape / run_organic_rust / OrganicBacktester）
```

关键架构事实：交易层**直接复用引擎枚举**（`BspKind`/`Side`/`DivKind` 已在
`buysellpoint.rs`/`divergence.rs` 中定义）。Python 版每个事件做字符串比对
（`e[0] == "type1"`）；Rust 版事件在磁带边界一次解析为枚举（M2 阶段连解析都消失——
引擎内部产出即枚举），概念在引擎→交易全链上**同一**，无字符串往返。

---

## 2. 类型系统设计（字段级）

### 2.1 types.rs — 基座类型

```rust
use crate::buysellpoint::{BspKind, Side};   // Type1|Type2|Type3, Buy|Sell（引擎原生）
use crate::divergence::DivKind;             // Trend|Consolidation（引擎原生）
use crate::stroke::Direction;               // Up|Down（引擎原生）

/// 中枢承载层编号（0..MAX_LADDER）。newtype 防止与 bar 下标 / slot 偏移混用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ladder(pub u8);

pub const MAX_LADDER: usize = 11;            // = Python MAX_LEVELS(8) + 3
pub const LADDER_BAR: Ladder = Ladder(0);
pub const LADDER_BI: Ladder = Ladder(1);
pub const LADDER_SEG: Ladder = Ladder(2);    // FIRST_BSP_LADDER：首个中枢承载层
pub const LADDER_MOVE: Ladder = Ladder(3);
pub const FIRST_BSP_LADDER: Ladder = LADDER_SEG;

/// 11 层布尔行的位掩码（buy1/sell1/sell_any/buy_any/up_move_settled 各一行）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LadderMask(pub u16);
impl LadderMask {
    pub fn get(self, k: Ladder) -> bool { self.0 >> k.0 & 1 == 1 }
    pub fn set(&mut self, k: Ladder)    { self.0 |= 1 << k.0 }
}

/// 腿类别。Python 槽键算术（ladder / ladder+100 / ladder+200）的类型化替代。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LegClass { Main, Osc, Rev }

/// 账本槽键。`leg_kind(key)` 的逆向工程消失——类别是字段不是除法余数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotKey { pub ladder: Ladder, pub leg: LegClass }

/// BSP 事件（BarSignalI.bsp_events 单元素的类型化）。
/// Python: (kind, side, seg_idx, confirmed, center_seg_start, center_zd, center_zg, price)
#[derive(Debug, Clone, Copy)]
pub struct BspEvent {
    pub kind: BspKind,                 // match 穷举：漏 Type2 编译不过
    pub side: Side,
    pub seg_idx: i64,
    pub confirmed: bool,
    pub center: Option<CenterRef>,     // cs is None 的类型化（type1 可无锚）
    pub price: f64,
}

/// 事件携带的中枢锚点快照（事件时刻的 zd/zg，非账本实时值）。
#[derive(Debug, Clone, Copy)]
pub struct CenterRef { pub seg_start: usize, pub zd: f64, pub zg: f64 }

/// 背驰事件。Python: (kind, direction, side, seg_idx, force_a, force_c, price)。
/// **side 不存储**——它是 direction 的纯函数（up→Sell / down→Buy），
/// 存两个字段允许不一致状态，违反严格性。
#[derive(Debug, Clone, Copy)]
pub struct DivEvent {
    pub kind: DivKind,                 // Trend | Consolidation（盘整背驰可见，E10）
    pub direction: Direction,          // 背驰所在 move 方向
    pub seg_idx: i64,                  // 背驰段锚（seg_c_end）
    pub force_a: f64,
    pub force_c: f64,
    pub price: f64,
}
impl DivEvent {
    pub fn side(self) -> Side {
        match self.direction { Direction::Up => Side::Sell, Direction::Down => Side::Buy }
    }
}

/// 腿锚点。rev 腿"无中枢锚"从 None 约定升级为独立变体——
/// "带中枢锚的 rev 腿"在类型上不可表示（E5 kind 盲配对否证的编码）。
#[derive(Debug, Clone, Copy)]
pub enum LegAnchor {
    /// osc/main 腿：锚定存活中枢。zd_floor = 触线回补价（osc）或 zg 门（main）。
    Center { seg_start: usize, boundary: f64, kind: BspKind },
    /// rev 腿：段尺度，无中枢锚（§4b：价格已离开中枢，T7 冻结走市场级 center book）。
    SegmentScale,
}

/// 两阶段守恒律（31/43课）。腿 open 时刻按账本阶段冻结进腿记录，
/// close 时 match 穷举——这是 was_earning: bool 的类型化。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConservationLaw {
    /// 降成本阶段开的腿：股数守恒，价差 profit 降共享 cost_basis。
    ShareConserving,
    /// 挣股数阶段开的腿：金额守恒（卖 V 买 V），total_shares 净增，cost_basis 锁 0。
    AmountConserving,
}
```

### 2.2 config.rs — 配置与变体

```rust
/// 市场语境。futures（INV-3 真实空头）不是错误分支——不在枚举里。
/// 设计 F1 实装时新增变体，编译器强制所有 match 点逐一表态（这正是要的效果）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketMode { Stock }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopMode { None, A, B }     // A/B 当前同语义（2% 核心止损），保留区分位

/// REV 关腿消融轴 R（§5.2）。字符串 "conf"/"cand"/"nested" 的枚举化，
/// 非法值在 PyO3 边界解析时拒绝，运行时不再有 ValueError 分支。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevClose { Conf, Cand, Nested }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sizing { Equal, Structure }

/// 有机赋格配置。字段逐一对应 Python OrganicConfig（默认值 = P5 = O0）。
#[derive(Debug, Clone)]
pub struct OrganicConfig {
    // ── P5 继承轴 ──
    pub open_kinds: Vec<BspKind>,        // 默认 [Type1, Type2]
    pub hard_type3: bool,                // true
    pub pre_type3: bool,                 // true
    pub center_gate: bool,               // true
    pub theta_amp: f64,                  // 0.01
    pub same_center_close: bool,         // true
    pub osc_mode: bool,                  // true
    pub osc_buy_sub: bool,               // false
    // ── 有机扩展轴 ──
    pub rev_mode: bool,                  // false
    pub master_seg_end: bool,            // true
    pub rev_gate: bool,                  // false
    pub rev_close: RevClose,             // Conf
    pub sizing: Sizing,                  // Equal
    pub earning_reaction: bool,          // false
    pub market_mode: MarketMode,         // Stock
}
```

变体表 `ORGANIC_VARIANTS`（O0/O1/O1v/O2/O2c/O2n/O3/O4）作为 `fn variant(name: &str) ->
Option<OrganicConfig>` 提供，字段值逐一照抄 Python `ORGANIC_VARIANTS`。

### 2.3 tape.rs — 信号磁带

```rust
/// 单 bar 信号（BarSignalI 的 Rust 形态）。
/// 布尔行压缩为位掩码；事件行用 Option<Box<…>> 表达稀疏性
/// （绝大多数 bar 无事件，None ⇔ Python 共享单例 NO_LADDER_EVENTS）。
#[derive(Debug, Clone)]
pub struct BarSig {
    pub close: f64,
    pub buy1: LadderMask,
    pub sell1: LadderMask,
    pub sell_any: LadderMask,
    pub buy_any: LadderMask,
    pub max_ladder: Ladder,
    pub type2_buy: bool,
    pub bsp_events: Option<Box<[Vec<BspEvent>; MAX_LADDER]>>,
    pub div_events: Option<Box<[Vec<DivEvent>; MAX_LADDER]>>,
    pub up_move_settled: LadderMask,
}

/// 完整磁带。一次构造（PyO3 边界 marshal 一次），多变体共享只读引用——
/// compute-once 原则在 Rust 侧的延续。
pub struct SignalTape { pub bars: Vec<BarSig> }

impl SignalTape {
    /// 能力守卫（与 run_organic 开头逐字对应）：
    /// 事件磁带非全空；rev_mode 消费方另查 div_events 非全空。
    pub fn has_bsp_events(&self) -> bool { … }
    pub fn has_div_events(&self) -> bool { … }
}
```

边界解析（`from_py_rows`）：kind/side/direction 字符串 → 引擎枚举，沿用 lib.rs
`parse_direction` 的 fail-fast 风格（非法值即 `PyValueError`，不静默吞）。

### 2.4 center_book.rs — 中枢生命周期账本

```rust
/// 存活中枢快照（账本视角的"最后已知中枢"）。
#[derive(Debug, Clone, Copy)]
pub struct LiveCenter { pub seg_start: usize, pub zd: f64, pub zg: f64 }

/// 中枢生命周期账本。市场性质：跨 trade 持续，与 run_version_i 逐字一致。
pub struct CenterBook {
    last: [Option<LiveCenter>; MAX_LADDER],
    dead: [std::collections::HashSet<usize>; MAX_LADDER],
    frozen: [Option<usize>; MAX_LADDER],      // ladder → 冻结中枢 seg_start
    pub version: u64,                          // 生死/边界事件版本号（SizeAllocator 门控）
}

impl CenterBook {
    /// 消费一层的本 bar BSP 事件流，更新生命周期（逐字移植 run_organic 主循环
    /// 的中枢账本段，含 version 自增的 not-in 守卫语义）。
    pub fn ingest(&mut self, ladder: Ladder, evs: &[BspEvent], hard_type3: bool);

    /// 该层当前存活中枢（last 存在 ∧ 不在 dead）。
    /// 返回 Option ⇒ "没有结构域就没有操作权"是类型签名而非注释（E2 同构）。
    pub fn alive(&self, ladder: Ladder) -> Option<LiveCenter>;

    pub fn is_dead(&self, ladder: Ladder, seg_start: usize) -> bool;
    pub fn is_frozen(&self, ladder: Ladder) -> bool;
}
```

### 2.5 fatigue.rs — 41课守门员

```rust
/// 衰竭证据键。Python 元组 ("bsp", kind, seg_idx, confirmed) / ("div", kind, seg_idx)
/// 的类型化——两类证据不再靠首元素字符串区分。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvidenceKey {
    Bsp { kind: BspKind, seg_idx: i64, confirmed: bool },
    Div { kind: DivKind, seg_idx: i64 },
}

pub struct FatigueMonitor {
    evidence: [std::collections::HashSet<EvidenceKey>; MAX_LADDER],
    structure_seen: LadderMask,
}

impl FatigueMonitor {
    /// 逐字移植 Python observe（清空先于添加的事件序约定保留）。
    pub fn observe(&mut self, ladder: Ladder, bsp: &[BspEvent], div: &[DivEvent],
                   up_settled: bool);
    /// u(k)：k 之上最近曾涌现结构的承载层（≤ entry_ladder）。
    pub fn u_of(&self, k: Ladder, entry_ladder: Ladder) -> Option<Ladder>;
    pub fn gate_open(&self, k: Ladder, entry_ladder: Ladder) -> bool;
}
```

证据集只被查询"非空"与"清空"，HashSet 迭代序不参与任何算术 → 无 bit-exact 风险。

### 2.6 ledger.rs — 账本与两阶段守恒律（本设计的核心类型）

```rust
/// 开放短差循环。is_open: bool 消失——开放/闭合是两个类型。
#[derive(Debug, Clone, Copy)]
pub struct OpenCycle { pub shares: f64, pub sell_price: f64 }

/// 已闭合短差循环。profit() 只在此类型上存在——
/// Python "if self.is_open: return 0.0" 分支不可表示。
#[derive(Debug, Clone, Copy)]
pub struct ClosedCycle { pub shares: f64, pub sell_price: f64, pub buy_price: f64 }
impl ClosedCycle {
    pub fn profit(self) -> f64 { (self.sell_price - self.buy_price) * self.shares }
}

/// 账本阶段（31课两阶段的类型化）。
/// EarningShares 变体**没有 cost_basis 字段**——挣股数阶段"成本"概念不存在
/// （锁 0 不是值是性质），改写它是编译错误。单向转换：只有
/// CostReduction → EarningShares 的代码路径，逆向不可达（earning 单调，INV-2）。
#[derive(Debug, Clone, Copy)]
pub enum LedgerPhase {
    CostReduction { cost_basis: f64 },   // 不变量：> 0（≤0 即刻转移）
    EarningShares,
}
impl LedgerPhase {
    /// 报告/trace 用读数（earning ⇒ 0.0）。只读投影，无 setter。
    pub fn cost_basis(self) -> f64 {
        match self {
            LedgerPhase::CostReduction { cost_basis } => cost_basis,
            LedgerPhase::EarningShares => 0.0,
        }
    }
    pub fn is_earning(self) -> bool { matches!(self, LedgerPhase::EarningShares) }
}

/// 开放腿记录：循环 + open 时刻冻结的守恒律 + 锚点。
#[derive(Debug, Clone, Copy)]
pub struct OpenLeg {
    pub cycle: OpenCycle,
    pub law: ConservationLaw,    // open 时刻的账本阶段，close 时 match 穷举
    pub anchor: LegAnchor,
}

/// 腿 trace 记录（diag 模式；Python trace dict 的 12 字段逐一对应）。
#[derive(Debug, Clone)]
pub struct LegTrace {
    pub slot: SlotKey,
    pub sell_bar: i64, pub sell_price: f64,
    pub buy_bar: i64,  pub buy_price: f64,
    pub shares: f64, pub diff: f64, pub profit: f64,
    pub was_earning: bool,
    pub shares_delta: f64,
    pub cost_basis_before: f64, pub cost_basis_after: f64,
}

/// 共享仓位 + 三类腿并发短差账本。守恒律算术与 Python OrganicLedger 逐字一致。
pub struct OrganicLedger {
    pub entry_price: f64,
    pub total_shares: f64,
    pub phase: LedgerPhase,
    pub level_frac: f64,
    pub cumulative_recovered: f64,
    /// 开放腿，**插入序存储**（Vec 非 HashMap）——_close 清腿顺序 = Python dict
    /// 插入序，cost_basis 串行递推对腿序敏感，这是 bit-exact 的硬前提（§7）。
    /// 腿数 ≤ 3×声部数（个位数），线性查找即最优。
    legs: Vec<(SlotKey, OpenLeg)>,
    pub completed: Vec<(SlotKey, ClosedCycle)>,
    pub n_open_rejects_zero: u32,
    pub trace: Option<Vec<LegTrace>>,
}

impl OrganicLedger {
    pub fn new(entry_price: f64, total_shares: f64, level_frac: f64,
               with_trace: bool) -> Self;     // phase = CostReduction { cost_basis: entry_price }

    /// 开腿（高抛/REV 先卖）。返回是否实际开启（LOU 状态转换依赖）。
    /// law = 按当前 phase 冻结；earning 阶段即时扣减 total_shares（INV-1 由构造保证）。
    pub fn open_diff(&mut self, key: SlotKey, frac: f64, sell_price: f64,
                     bar: i64, anchor: LegAnchor) -> bool;

    /// 闭腿。核心 match（穷举守恒律，第三种守恒律不可静默引入）：
    ///
    /// ```text
    /// match leg.law {
    ///     ConservationLaw::AmountConserving => {
    ///         // INV-2 金额守恒：卖 V 买 V，total_shares 净增，phase 不变（已是 Earning）
    ///         self.total_shares += cycle.shares * cycle.sell_price / buy_price;
    ///     }
    ///     ConservationLaw::ShareConserving => {
    ///         // 股数守恒：profit 落袋 + 降共享 cost_basis；触发 ≤0 → 单向相变
    ///         let profit = closed.profit();
    ///         self.cumulative_recovered += profit;
    ///         if let LedgerPhase::CostReduction { cost_basis } = &mut self.phase {
    ///             if self.total_shares > 0.0 { *cost_basis -= profit / self.total_shares; }
    ///             if *cost_basis <= 0.0 { self.phase = LedgerPhase::EarningShares; }
    ///         }
    ///         // phase 已是 Earning 而腿是 ShareConserving：合法（earning 转换前开的旧腿），
    ///         // 此时 cost_basis 无处可降——与 Python "cost_basis=0 后 -= 再钳 0" 语义
    ///         // 等价处需逐字核对（§7 陷阱 T5）。
    ///     }
    /// }
    /// ```
    pub fn close_diff(&mut self, key: SlotKey, buy_price: f64, bar: i64);

    pub fn open_slot(&self, key: SlotKey) -> Option<&OpenLeg>;
    /// _close 强制清腿：按插入序闭全部开放腿。
    pub fn close_all(&mut self, price: f64, bar: i64);
}
```

> **设计注记（T5 预先标记）**：Python `close_diff` 的股数守恒分支在 `earning=True`
> 后仍执行 `cost_basis -= profit/total_shares` 然后 `if cost_basis <= 0: cost_basis = 0`
> ——由于 earning 时 cost_basis 已是 0，profit>0 时减出负数再钳回 0，profit<0 时
> **cost_basis 会短暂上浮为正但随即被 `<=0` 检查……不成立而保留正值**。逐字核对
> Python：`if self.cost_basis <= 0: self.cost_basis = 0.0; self.earning = True`——
> 亏损短差可使 earning 后的 cost_basis 回正、earning 标志已置 True 不回退。
> 这是 Python 原型中 phase 与 cost_basis 可分歧的**真实语义**（earning=True 而
> cost_basis>0）。Rust 的 LedgerPhase 单向相变**不可表示**此状态——这不是移植
> 偏差，是 Python 原型的概念模糊点被类型系统暴露。处置：实装时先在 Python 侧
> 用断言扫描全部既有回测 trace 确认该路径是否实际到达（cost>0 的 ShareConserving
> 旧腿在 earning 转换后亏损闭合）；若到达 ⇒ 这是**定义冲突**（守恒律相变是否可逆），
> 走矛盾上浮（no-workaround），不得为 bit-exact 在 Rust 里复刻模糊语义。
> 若不可达 ⇒ Rust 类型即严格形式，附不可达性证明（事件序：master earning 反作用
> 下旧 ShareConserving 腿在 MASTER_CLOSE 前已全闭）。
>
> **实证状态补注（2026-06-10，Phase 5 验收回测落盘后）**：OKLO 447K + QQQ 728K
> 全变体（O0-O4/O2c/O2n/O1v）earning 触发率 = 0（`organic_fugue_backtest.md`
> counters.n_earning_reached），T5 路径**当前空有效域**——既有 trace 中不存在
> "earning 转换时仍有未闭合 ShareConserving 腿"的实例。裁决悬置条件精确化：
> 仅当某数据集 earning 触发率 > 0 时 M0 断言扫描才有信息增量；在此之前
> LedgerPhase 单向相变与 Python 在全部已观测路径上逐位等价（分歧点不可达 ⇒
> bit-exact 对账门不受 T5 影响）。E9/complete_fugue 空有效域先例同构。

### 2.7 lou.rs — LevelOperatingUnit（38课程式 FSM）

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LouState { Ride, Rev }

/// 每 bar 传给 LOU 的本层语境（runner 预组装，LOU 不持磁带引用）。
pub struct LadderCtx<'a> {
    pub bar: i64,
    pub close: f64,
    pub evs: &'a [BspEvent],          // 本层 bsp_events
    pub devs: &'a [DivEvent],         // 本层 div_events
    pub sub_buy: bool,                // buy_any[k-1]（区间套/osc_buy_sub）
    pub sub_sell: bool,               // sell_any[k-1]（域腿定位）
    pub frac: f64,                    // 本腿规模快照
    pub entry_ladder: Ladder,
}

pub struct LevelOperatingUnit { pub ladder: Ladder, pub state: LouState }

impl LevelOperatingUnit {
    /// 38课段终结三触发（master 复用同一谓词——单一 FSM 多声部转位的代码落点）。
    pub fn seg_end_trigger(evs: &[BspEvent], devs: &[DivEvent]) -> bool {
        evs.iter().any(|e| e.confirmed && e.side == Side::Sell
                           && matches!(e.kind, BspKind::Type1 | BspKind::Type3))
        || devs.iter().any(|d| d.side() == Side::Sell)
    }

    /// T5 三岔（rev_close 消融轴）。match 穷举 RevClose 与 BspKind——
    /// 新增消融模式或新增 kind 时本函数编译报错，强制表态。
    pub fn rev_close_trigger(cfg: &OrganicConfig, evs: &[BspEvent],
                             devs: &[DivEvent], sub_buy: bool) -> bool;

    /// 每 bar 一步（仅 Long 态由 runner 调用）。转换表 T1-T7 为顺序 match 块，
    /// 优先级（T6/T7 逃逸 > T5 > T1 > T3/T4）由代码顺序表达，与 Python 逐字同序。
    pub fn step(&mut self, cfg: &OrganicConfig, ctx: &LadderCtx,
                ledger: &mut OrganicLedger, book: &CenterBook,
                fatigue: &FatigueMonitor, counters: &mut Counters);
}
```

`step` 内部结构（转换表 → match arm 的直译，伪代码）：

```text
let osc = SlotKey { ladder: k, leg: LegClass::Osc };
let rev = SlotKey { ladder: k, leg: LegClass::Rev };

// ── REV 态：T6/T7 逃逸 > T5 正常关 ──
if self.state == LouState::Rev {
    let pre  = cfg.pre_type3 && evs.any(|e| !e.confirmed && e.kind==Type3 && e.side==Buy);
    let hard = evs.any(|e| e.confirmed && e.kind==Type3 && e.side==Buy);
    if hard      { ledger.close_diff(rev, c, bar); counters.n_rev_close_t7 += 1; self.state = Ride; }
    else if pre  { ledger.close_diff(rev, c, bar); counters.n_rev_close_t6 += 1; self.state = Ride; }
    else if Self::rev_close_trigger(cfg, evs, devs, ctx.sub_buy)
                 { ledger.close_diff(rev, c, bar); counters.n_rev_close_t5 += 1; self.state = Ride; }
}
// ── RIDE 态：T1（REV 开腿，守卫 = 门开 ∧ 非冻结）/ T2（门关无动作）──
if self.state == LouState::Ride && cfg.rev_mode && Self::seg_end_trigger(evs, devs) {
    counters.n_rev_attempts += 1;
    if cfg.rev_gate && !fatigue.gate_open(k, ctx.entry_ladder)
         { counters.n_rev_gate_rejects += 1; }
    else if book.is_frozen(k)
         { counters.n_rev_frozen_rejects += 1; }
    else {
        if ledger.open_slot(osc).is_some() { ledger.close_diff(osc, c, bar); }  // 先 CLOSE_OSC
        if ledger.open_diff(rev, ctx.frac, c, bar, LegAnchor::SegmentScale)
             { counters.n_rev_open += 1; self.state = LouState::Rev; }
    }
}
// ── 域腿子循环 T3/T4（P5 逐字；Rev 态不开新 osc）──
if cfg.osc_mode && self.state == LouState::Ride {
    match ledger.open_slot(osc) {
        Some(leg) => match leg.anchor {
            LegAnchor::Center { seg_start, boundary, .. } => {
                if book.is_dead(k, seg_start)            { ledger.close_diff(osc, c, bar); }
                else if c <= boundary && (!cfg.osc_buy_sub || ctx.sub_buy)
                     { ledger.close_diff(osc, c, bar); counters.n_osc_zd_close += 1; }
            }
            LegAnchor::SegmentScale => unreachable!("osc 槽内不可能有段尺度锚——\
                open 路径只以 Center 锚开 osc 腿；此 arm 是类型完备性要求，到达即 bug"),
        },
        None => if let Some(lc) = book.alive(k) {
            if !book.is_frozen(k) && k >= Ladder(1) && c >= lc.zg && ctx.sub_sell {
                let anchor = LegAnchor::Center { seg_start: lc.seg_start,
                                                 boundary: lc.zd, kind: BspKind::Type1 /*"osc"标记位，见注*/ };
                if ledger.open_diff(osc, ctx.frac, c, bar, anchor)
                     { counters.n_osc_open += 1; }
            }
        },
    }
}
```

> 注：Python osc 锚第三元是字符串 `"osc"`（与 main 腿的 kind 字符串共用槽位）。
> Rust 形态：`LegAnchor::Center` 的 `kind` 字段改为
> `enum AnchorKind { Bsp(BspKind), Osc }`——字符串标记的类型化，main/osc 锚来源
> 在类型上可分辨（trace 归因需要）。上面伪代码从简，实装用 `AnchorKind`。

### 2.8 allocator.rs / counters

```rust
pub struct SizeAllocator {
    frac: [f64; MAX_LADDER],
    version: Option<u64>,
}
impl SizeAllocator {
    /// 中枢生死事件门控重算。**求和顺序 = ladder 升序**（Python dict 插入序
    /// = active_levels 升序），bit-exact 前提之一（§7 T1）。
    pub fn maybe_recompute(&mut self, version: u64, active: &[Ladder],
                           book: &CenterBook, c: f64);
    pub fn frac(&self, k: Ladder) -> f64;
}

/// 计数器：Python 字符串 dict 的结构体化（拼错键名 = 编译错误）。
#[derive(Debug, Default, Clone)]
pub struct Counters {
    pub n_close_normal: u32, pub n_close_pre_type3: u32, pub n_close_hard_type3: u32,
    pub n_open_gate_rejects: u32, pub n_osc_open: u32, pub n_osc_zd_close: u32,
    pub n_rev_attempts: u32, pub n_rev_gate_rejects: u32, pub n_rev_frozen_rejects: u32,
    pub n_rev_open: u32, pub n_rev_close_t5: u32, pub n_rev_close_t6: u32,
    pub n_rev_close_t7: u32, pub n_master_rev_open: u32, pub n_master_rev_close: u32,
    pub n_earning_reached: u32, pub n_exit_upgraded: u32, pub n_open_rejects_zero: u32,
    pub rev_attempts_by_ladder: [u32; MAX_LADDER],
    pub rev_opens_by_ladder: [u32; MAX_LADDER],
}
```

### 2.9 runner.rs — 主循环

```rust
/// 持仓上下文：仓位相关一切状态**只在 Long 变体中存在**。
/// Python 的 pos: Optional / voices: dict / master_state 三个平行变量
/// 及其同步置空约定，折叠为一个变体的字段——FLAT 态读仓位不可表示。
pub struct LongCtx {
    pub entry_bar: i64,
    pub entry_price: f64,
    pub entry_ladder: Ladder,
    pub active_levels: Vec<Ladder>,            // [floor, entry) 升序
    pub ledger: OrganicLedger,
    pub voices: Vec<LevelOperatingUnit>,       // 与 active_levels 同序
    pub master: LouState,                      // earning 反作用下的 master RIDE/REV
}

pub enum RunnerState {
    Flat,
    Armed { arm_bar: i64, arm_ladder: Ladder },
    Long(Box<LongCtx>),
}

/// 完成交易记录（CompletedTrade 逐字段对应；round 语义见 §7 T2）。
#[derive(Debug, Clone)]
pub struct TradeRec {
    pub entry_bar: i64, pub entry_price: f64,
    pub exit_bar: i64,  pub exit_price: f64,
    pub pnl_pct: f64,                  // py_round(·, 4)
    pub exit_reason: ExitReason,
    pub n_short_diffs: u32,
    pub cost_basis_at_exit: f64,       // py_round(·, 6)
}

/// 出场原因：Python f-string 的枚举化（marshal 回 Python 时再格式化为同字符串）。
#[derive(Debug, Clone, Copy)]
pub enum ExitReason {
    Type1Sell { ladder: Ladder },
    EarningUpgrade { ladder: Ladder },
    StopCore2pct,
    EodClose,
}

pub struct RunResult {
    pub trades: Vec<TradeRec>,
    pub counters: Counters,
    pub ladder_attribution: [u32; MAX_LADDER],
    pub ladder_held_bars: [u64; MAX_LADDER],
    pub leg_recovered: Vec<(SlotKey, f64)>,    // fsm_recovered（slot 排序后输出）
    pub leg_diffs: Vec<(SlotKey, u32)>,
    pub n_addon: u32, pub n_core_stops: u32,
    pub diag: Option<Vec<TradeDiag>>,          // diag 模式逐 trade 快照 + LegTrace
}

/// 主入口。控制流逐字移植 run_organic：
/// 中枢账本 ingest → fatigue observe → FLAT/ARMED/LONG 分派 →
/// (LONG) allocator → master 循环 → voice main 腿 → LOU.step → eod_close。
pub fn run_organic(tape: &SignalTape, floor_ladder: Ladder,
                   cfg: &OrganicConfig, stop_mode: StopMode,
                   diag: bool) -> RunResult;
```

master 循环（earning 反作用，§5.6）的 Rust 形态要点：master 的 REV 腿槽 =
`SlotKey { ladder: entry_ladder, leg: Rev }`——与 voice 无碰撞（voices 域为
`[floor, entry)`，不含 entry_ladder），这一不相交性在 Python 里是注释，在 Rust 里
由 `active_levels` 的构造函数保证并 `debug_assert`。

---

## 3. 与现有 Rust 引擎的集成

### 3.1 决策：独立交易层，不嵌入 process_bar

**裁决：交易逻辑是引擎事件流的独立消费层，不嵌入 `RecursiveOrchestrator::process_bar`。**

理由（按强度排序）：

1. **compute-once 原则**（既有架构裁决）：一次信号计算服务 8 个变体（O0-O4/O2c/O2n/O1v）
   ×消融矩阵。嵌入 process_bar ⇒ 每变体重跑引擎，8× 引擎成本；独立层 ⇒ 磁带构造一次，
   变体循环只跑交易逻辑（毫秒级）。
2. **回归守卫边界**：引擎的 bit-exact 守卫（447K oracle、seg_checkpoint_tests）与交易层的
   O0≡P5 守卫是两套独立契约。嵌入 ⇒ 任一侧改动都迫使全链重验；分层 ⇒ 磁带是清晰的
   验证切面。
3. **信号层多消费者**：E 版/I 版/P 系变体/区间套实验都消费同一引擎，交易逻辑只是
   消费者之一。

"零 Python 边界"通过**同进程同循环**达成而非嵌入：M2 阶段的 `OrganicBacktester`
（§3.3）在一个 Rust 循环里做 `process_bar → 事件提取 → 多变体 runner 步进`，
bar 级别零跨语言调用——独立层与全 Rust 闭环不矛盾。

### 3.2 M1 接口：磁带注入（Python 信号层 + Rust 交易层）

lib.rs 新增两个绑定：

```rust
/// 信号磁带容器。从 Python BarSignalI 列表一次性构造（O(N) marshal，每数据集一次），
/// 之后所有变体回测共享，不再跨边界。
#[pyclass(name = "OrganicTape")]
struct PyOrganicTape { inner: trading::SignalTape }

#[pymethods] impl PyOrganicTape {
    /// 从列式数组构造（避免 447K 个 Python 对象逐个 getattr）：
    /// closes: Vec<f64>；布尔行: Vec<u16> 位掩码（Python 侧打包）；
    /// 事件行: 扁平化 (bar_idx, ladder, kind, side, seg_idx, confirmed, cs, zd, zg, price)
    /// 元组列表 + div 同构 —— 稀疏事件不为空 bar 付费。
    #[staticmethod]
    fn from_columns(…) -> PyResult<Self>;
    fn n_bars(&self) -> usize;
}

/// 单变体回测。返回 (trades 元组列表, extra 嵌套元组)——与既有 lib.rs 元组风格一致。
#[pyfunction]
#[pyo3(signature = (tape, variant, floor_ladder = 2, stop_mode = "none", diag = false))]
fn run_organic_rust(tape: &PyOrganicTape, variant: &str, floor_ladder: u8,
                    stop_mode: &str, diag: bool) -> PyResult<(Vec<TradeTuple>, ExtraTuple)>;
```

Python 侧 `organic_fugue_backtest.py` 增加 `--engine rust` 开关：磁带照旧由
`compute_organic_signals` 产出 → 打包传给 `OrganicTape.from_columns` → 变体循环改调
`run_organic_rust` → 结果与 Python `run_organic` 逐位对账（§7）。

### 3.3 M2 接口：信号层下沉（全 Rust 闭环）

```rust
/// 全 Rust 端到端回测器：bars 进，多变体交易记录出。
#[pyclass(name = "OrganicBacktester")]
struct PyOrganicBacktester {
    orch: orchestrator::RecursiveOrchestrator,
    signal_state: trading::signal::OrganicSignalState,   // 见下
    runners: Vec<(String, trading::runner::StreamingRunner)>,
}
```

信号层下沉需要三块新 Rust 能力（按工作量排序）：

| 块 | 现状 | 下沉方案 | 消除的成本 |
|----|------|---------|-----------|
| **(a) ladder2 div_events** | Rust `IncrementalBiZhongshuBsp` 内部计算 divergences 但不暴露；Python 被迫每 stroke-增长 bar marshal `current_strokes()` O(S) + 全链重算 → **OKLO 信号层 17s→402s 的成本墙** | 增量引擎新增 `take_new_divergences()` delta 接口（内部已有数据，纯 surfacing）；等价性沿用既有差分守卫模式（Python 组合链 ≡ Rust 增量链已有布尔级先例） | 402s 墙整体拆除（marshal 与重算同时消失） |
| **(b) ladder3/递归层事件** | ladder3 divs 在 bsp_epoch 门控点 Python marshal 重算；递归层 BSP+divs 在 Python 用 `_level_bsps_with_divs` 组合 Rust 纯函数 | orchestrator 内 `inc_seg_div.current()` 已维护（透传 delta）；递归层在 `LevelEngine::process` 末尾追加 divs+bsps 计算（全部纯函数已在 Rust） | 每 epoch 的 segments/zhongshus/moves marshal |
| **(c) ladder0/1 PH 子级别** | `PHLevelState`（流式 merge-tree top2 + settle 检测）纯 Python | 移植到 `rust/src/ph.rs`（新增流式状态机，批量闭式 `persistence≡max-min` 已有先例）；这是唯一**真正的新算法移植**，需独立 bit-exact 差分 | 每 bar 2×2 次 Python 树更新 |

(a) 是性能主轴也是概念主轴——盘整背驰可见性（E10 最后一块）目前靠 Python 侧
O(S²) 摊还重算赎买，Rust 化后它回归"引擎中间产物 surfacing，增量成本≈0"的
设计原意（organic_fugue_design §5.1 成本声明勘误的反向修复）。

### 3.4 引擎侧 diff 面（M2）

| 文件 | 改动 | 性质 |
|------|------|------|
| `bi_zhongshu_bsp.rs` | 暴露 `take_new_divergences() -> Vec<Divergence>`（delta，内部 seen 下沉） | surfacing，不改计算 |
| `orchestrator.rs` | `LevelEngine` 增 divs/bsps 产出 + delta 缓存；新增 `take_div_events(ladder)` 族 accessor | 新增旁路，结构化列表零改动（既有 parity 不受扰） |
| `ph.rs` | 新增流式 `PhLevelState`（top2 merge tree + settle 检测） | 新算法移植，独立差分守卫 |
| `lib.rs` | `OrganicTape` / `run_organic_rust` / `OrganicBacktester` 绑定 | 纯增 |

M1 阶段**引擎零改动**（磁带来自 Python）。

---

## 4. 关键设计决策（问题 5 逐一回答）

### 4.1 嵌入 process_bar 还是独立层？

独立层（§3.1 已裁决）。补充存在论理由：缠论引擎计算的是**市场性质**（中枢/走势/
买卖点对所有观察者相同），交易层计算的是**操作者状态**（仓位/成本/声部）。两者
范畴不同——一个市场状态可被 N 个操作者配置同时消费。嵌入 = 把操作者状态塞进
市场性质的计算，范畴混淆。

### 4.2 仓位状态的所有权

**所有权链是一条单链，无共享**：

```
OrganicRunner（每标的一个）
  └── RunnerState::Long(Box<LongCtx>)
        ├── ledger: OrganicLedger          ← 资金状态唯一所有者
        │     ├── total_shares / phase / cumulative_recovered
        │     └── legs: Vec<(SlotKey, OpenLeg)>
        ├── voices: Vec<LevelOperatingUnit> ← 只持 (ladder, state)，零资金状态
        └── master: LouState
```

- **LOU 不拥有钱**。它是纯状态机（2 字段：ladder + RIDE/REV），每 bar 经
  `step(&mut self, …, ledger: &mut OrganicLedger, …)` 借入账本可变引用，调用结束
  归还。借用检查器保证同一 bar 内不存在两个组件同时改写账本（Python 里这只是
  "恰好没写出来"）。
- **CostBasis 不是独立对象**——它是 `LedgerPhase::CostReduction` 变体的字段。
  谁拥有 phase 谁拥有 cost_basis；earning 后它不存在（不是 0，是无此字段）。
- **CenterBook / FatigueMonitor 归 Runner 直接持有**（市场性质，跨 trade 存活），
  以 `&CenterBook`（只读）借给 LOU——LOU 改写中枢账本不可表示，市场性质对操作者
  只读这一条存在论约束成为类型签名。

### 4.3 多标的并行

**shared-nothing**：每标的一个 `OrganicRunner` 拥有全部状态，标的之间零共享可变
状态。`runners.par_iter_mut()`（rayon）即并行，`Send` 由编译器自动验证——若未来
有人引入跨标的共享（如全局风控池），编译器在并行点报错，强制显式化（`Arc<Mutex<…>>`
或消息通道），不可能静默引入数据竞争。这沿用 1.88M bar/边 10 边 fork 并行的既有
模式，但把"进程隔离保证无共享"升级为"类型系统保证无共享"（线程级，省 fork/序列化）。

资金语义注意：多标的共享资金池是**未定义需求**（当前所有回测每标的独立
INITIAL_CAPITAL）。本设计不预留共享池钩子——预留未验证需求的接口违反 no-patch
（声明膨胀）。需求出现时走类型系统强制的显式化路径（上段）。

### 4.4 trait 的克制使用

本设计**不引入 trait 抽象**（无 `trait Strategy`/`trait Ledger`）。理由：当前只有
一个交易框架（有机赋格）和一个账本语义；为单实现建 trait = 推测性抽象 =
声明膨胀。F1（futures/INV-3）实装时若出现第二账本语义，届时由编译器驱动提取
（`MarketMode` 新变体迫使所有 match 点表态，自然显形出接口边界）。

---

## 5. 与 Python 版的关系

| 维度 | Python `organic_fugue.py` | Rust `trading/` |
|------|--------------------------|-----------------|
| 角色 | 原型/语义基准（oracle） | 生产版 |
| 验证地位 | O0≡P5 守卫的持有者；Rust 版的逐位对账对象 | 对账通过后成为默认执行路径 |
| 长期保留 | **保留**——差分守卫的 oracle（与引擎侧 "Python oracle + Rust 实现" 既有模式一致，非死代码：每次 Rust 侧改动的回归基线） | — |
| 演进 | 冻结（语义变更先改 Python 过 O0 守卫，再同步 Rust 过对账） | 跟随 |

变更纪律：**两版语义分叉 = 定义冲突**，走矛盾上浮，不允许"Rust 先改了 Python 回头补"。

---

## 6. 迁移路线图

```
M0  类型骨架 + 账本 + LOU（无 runner）                      [引擎零改动]
    ├─ trading/{types,config,ledger,lou,fatigue,allocator,center_book}.rs
    ├─ TDD：LOU 转换表 T1-T7 逐条单测（合成事件序，镜像 test_organic_fugue.py）
    ├─ TDD：守恒律不变量测试（ShareConserving/AmountConserving 算术、相变单向性、
    │        INV-1 构造性保证、open 拒绝计数）
    └─ §2.6 T5 语义疑点的 Python 侧断言扫描（先于 Rust 实装裁决）

M1  runner + 磁带注入 + 全变体 bit-exact                    [引擎零改动]
    ├─ runner.rs + tape.rs + lib.rs 绑定（OrganicTape / run_organic_rust）
    ├─ 对账门 1：Rust O0 ≡ Python O0（OKLO 447K，trades+trace+counters 逐字段）
    │   （传递性：Python O0 ≡ P5 已有守卫 ⇒ Rust O0 ≡ P5）
    ├─ 对账门 2：O1/O1v/O2/O2c/O2n/O3/O4 全变体 ≡ Python（同磁带）
    └─ 交叉标的：QQQ + BRN（费率敏感表同跑）

M2  信号层下沉（全 Rust 闭环）                              [引擎增量改动，§3.4]
    ├─ (a) IncrementalBiZhongshuBsp div surfacing（402s 墙拆除）
    ├─ (b) ladder3/递归层事件 delta 接口
    ├─ (c) PH 子级别流式移植（独立差分守卫）
    ├─ 对账门 3：Rust 磁带 ≡ organic_signals.py 磁带（逐 bar 逐字段，三标的）
    └─ OrganicBacktester：bar 进多变体结果出，每 bar 零 Python 边界

M3  生产化
    ├─ 多标的 rayon 并行（shared-nothing，§4.3）
    ├─ Python 执行路径降级为 oracle（默认 --engine rust；organic_fugue.py 保留）
    └─ 增量持久化对接（既有 28 核并行回测基建的消费接口）
```

依赖关系：M0→M1 串行（M1 用 M0 类型）；M2 的 (a)(b)(c) 三块相互独立**可并行**
（三个独立工位）；M2 整体不阻塞 M1 产出可用性（M1 后即可用 Rust 跑全变体矩阵，
只是信号层还慢）。

---

## 7. bit-exact 验证策略

### 7.1 对账面（三道门，见 §6）

1. **trades**：全字段（entry/exit bar+price、pnl_pct、exit_reason 字符串、
   n_short_diffs、cost_basis_at_exit）。
2. **trace**（diag 模式）：LegTrace 12 字段逐条——这是账本算术的逐步对账，
   trades 相同而 trace 不同 = 误差抵消假象，必须 trace 级。
3. **counters + by_ladder 表 + leg_contribution**：路径计数相同 ⇒ 控制流逐分支同构。
4. **磁带门（M2）**：BarSig 全字段逐 bar（含事件元组逐元素）。

### 7.2 f64 陷阱清单（既有移植经验的预先固化）

| # | 陷阱 | 来源 | 设计应对 |
|---|------|------|---------|
| T1 | **求和/遍历顺序**：Python dict 插入序 ≠ Rust HashMap 迭代序；`sum(amps.values())`、`_close` 清腿序、报告聚合序都对浮点结果敏感 | SizeAllocator / OrganicLedger / 报告层 | 全部用 Vec/数组按 ladder 升序或插入序存储（§2.6/§2.8 已入类型设计）；HashMap 仅用于纯成员判定（dead/seen/evidence） |
| T2 | **Python round = 银行家舍入**（round-half-even on decimal） | pnl_pct round(·,4) / cost_basis round(·,6) | 复用 macd.rs 既有 py-round 实现（`area round(·,6)` 先例已 bit-exact 验证） |
| T3 | **max/min tie-break**：Rust `max_by` 返回最后极值，Python 返回首个 | 既有教训（引擎 Rust 重写陷阱清单） | 交易层无 max 扫描（事件序优先级是 if-chain）；若新增，手写 first-extreme |
| T4 | **除零/负价**：`(lc.zg - lc.zd) / c`、`buy_price <= 0` 守卫 | allocator / close_diff | 逐字保留 Python 守卫条件与短路顺序 |
| T5 | **earning 后 ShareConserving 旧腿亏损闭合**可使 Python cost_basis 回正而 earning 不回退——LedgerPhase 单向相变不可表示此状态 | §2.6 设计注记 | M0 先 Python 断言扫描裁决：不可达 ⇒ 类型即严格形式；可达 ⇒ 定义冲突上浮，**不复刻模糊语义** |
| T6 | **事件序内优先级**：同 bar hard>pre>t5、逃逸>关>开、master 先于 voice | run_organic 控制流 | 控制流逐字移植 + counters 对账（7.1-3）捕获任何分支序漂移 |
| T7 | **NaN**：磁带数据含 nan 时比较语义 Python/Rust 不同 | 既有教训（BRN 0.68% nan 必删） | 磁带构造期拒绝 nan（fail-fast），与既有清洗纪律一致 |

### 7.3 测试分层（testing-override 适用性声明）

LOU/账本依赖的定义（38课程式、守恒律、41课门）均为**已结算/已落盘裁决**（E1-E10），
非生成态 ⇒ 标准 TDD 纪律适用（80% 覆盖率目标有效）。唯一例外是 T5 疑点——若断言
扫描判定可达，则该分支属于定义冲突，其测试按生成态例外处理（暴露问题而非通过）。

---

## 8. 结果包（六要素）

1. **结论**：Rust 交易层 = `rust/src/trading/` 九模块；类型系统把 Python 原型的
   8 类约定/断言升级为编译期不可表示（§0 表）；独立消费层架构（非嵌入
   process_bar）；三阶段迁移（磁带注入 → 信号层下沉 → 生产化），每阶段带逐位
   对账门；设计精确到字段级可直接实装。
2. **定义依据**：类型语义逐一映射自 `organic_fugue.py`（O0≡P5 守卫持有者）与
   `organic_fugue_design.md` §5 模块定义；引擎枚举（BspKind/Side/DivKind/Direction）
   取自 `buysellpoint.rs`/`divergence.rs`/`stroke.rs` 现有定义，零新定义；守恒律
   算术取自 `cost_reduction_fsm.py` ShortDiffCycle / `OrganicLedger` 逐字。
3. **边界条件**：(a) 编译期守卫范围以 §0 诚实声明为界——运行时市场事件（中枢死亡）
   仍是运行时检查，若实装中发现有人依赖"编译期保证中枢死亡安全"即为声明膨胀，
   结论翻转；(b) T5 疑点若判定可达且上浮裁决"相变可逆"，LedgerPhase 单向设计
   作废，退回带不变量断言的可变 cost_basis；(c) M2 性能预期（402s 墙拆除）依赖
   "增量引擎内部 divergences 与 Python 组合链逐位等价"——若 div 级差分守卫失败
   （既有守卫只到布尔级），(a) 块改为引擎内全链重算的 Rust 版（墙降级为消 marshal
   不消重算）；(d) bit-exact 在 tie-break/求和序之外若出现新的浮点分歧源（如
   编译器 FMA 收缩），需 `-C llvm-args` 级别排查，对账门不放宽。
4. **下游推论**：M1 后变体矩阵回测从分钟级降到秒级 ⇒ §8 消融矩阵（O0-O4×标的×
   费率）可全量重跑而非抽样；M2 后盘整背驰可见性回归零增量成本 ⇒ div_events
   可默认开启进所有下游磁带；类型化事件流若被其他消费者（E 版/区间套）采纳，
   字符串事件元组可整体退役；T5 裁决无论结果如何都会精化两阶段守恒律的定义
   （相变可逆性此前未显式化）。
5. **谱系引用**：E1-E10（interval_nesting 结果包）；267/268a号（守恒律 FSM）；
   525号（笔中枢）；521号（纯拓扑无动量）；项目记忆 project_engine_rust_rewrite
   （tie-break/clone 陷阱）、project_organic_fugue_implementation（402s 墙）、
   project_segment_level_incremental（增量器复用边界）。不确定谱系：**"类型系统
   作为语法守卫"作为概念此前未单独立谱**——若实装中发生概念分离（编译期守卫 vs
   运行时不变量的边界争议，T5 即首个候选），需新谱系记录。
6. **影响声明**：本产出为新设计文档 `analysis/organic_fugue_rust_design.md`，
   未改动任何代码/定义/磁带。声明的未来 diff 面：M1 纯增（trading/ 九文件 +
   lib.rs 绑定 + 回测脚本 --engine 开关）；M2 触及 bi_zhongshu_bsp.rs/
   orchestrator.rs/ph.rs（§3.4 表，结构化列表零改动）；Python organic_fugue.py
   全程不改（oracle 冻结）。
