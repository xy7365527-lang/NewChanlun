//! **T 操作层引擎**（多重赋格版：每涌现级别一个 Z₂ 状态机 + **跨级别耦合**齿轮咬合）。
//!
//! ## 本次重写（架构转变：N 个独立滤波器 → N 个耦合滤波器 / 多重赋格）
//! 上一版（多尺度独立滤波器，commit 50f2235013）每个 ladder 一个 [`LevelEngine`]，**只响应
//! 自己 ladder 的 BSP** 做整仓翻转，连相邻级别都不碰（275号局部依赖的极致）。问题：缠论的操作
//! 是**跨级别耦合**的（齿轮咬合）——高级别持多时，低级别的"卖点翻空"不该是独立做空，而该是
//! 高级别核心仓的**短差减仓**（H¹ 机动仓）。独立版丢失了这层耦合 ⟹ 低级别在高级别趋势腿里
//! 机械翻空 = 逆势对冲（见记忆 `project_t_short_leg_regime_function`：做空腿是亏损唯一来源）。
//!
//! 本版给每个 [`LevelEngine`] 加 **parent_state 耦合**：`LevelEngine[j]` 收到 BSP 时先看
//! `LevelEngine[j+1]`（上级别）的状态，据此决定**翻转**还是**短差**。
//!
//! ## 耦合规则（齿轮咬合）
//! `LevelEngine[j]` 的父级 = `LevelEngine[j+1]`（高 ladder = 高级别 = 核心仓侧）：
//! - **父级持多（Long）**：j 卖点 → 不翻空，开**短差空**（1/3 父级 units，机动仓）；j 买点 → 平短差（回补归还）。
//! - **父级持空（Short）**：j 买点 → 不翻多，开**短差多**（1/3 父级 units）；j 卖点 → 平短差。
//! - **父级空仓（Empty）/ 顶层（无 j+1）**：j 正常独立翻转（无上级约束，用自己资金池）。
//!
//! 这正是**四步循环**（每对相邻级别）：① 高级别买点→建核心多 → ② 次级别卖点→减仓 1/3 做空（短差）
//! → ③ 次级别买点→平空回补 → ④ 高级别卖点→翻空（核心方向变）。每级别同时是其次级别的"核心"
//! 和其父级的"机动仓"——递归嵌套 ⟹ 多重赋格。
//!
//! ## 仓位递归（几何塔，由短差 sizing 自然涌现）
//! 短差 units = **父级 units / 3**。⟹ 次级别机动仓 = 核心的 1/3，次次级别 = 1/9 …… 高级别大仓位
//! 吃趋势、低级别小仓位做短差，**指数衰减的暴露几何塔自然涌现**（无需显式塔配额）。资金仍每槽
//! 等额预分配（`level_capital()`，独立翻转模式用自己池），短差 sizing 锚在父级 units 上。
//!
//! ## 处理顺序：top-down（齿轮咬合方向）
//! 独立版顺序无关（零耦合）。耦合后必须 **高 ladder 先更新**：`TPositionEngine::step` 从
//! `MAX_LADDER-1` 降序处理，子级读到父级**本 bar 最新**状态 ⟹"高级别翻空时低级别跟着翻方向"。
//!
//! ## 会计（NAV 中性 ⟹ 守恒与 sizing 解耦）
//! 每个 open/open_sized/close 在成交价 c 上 NAV 中性（做空 `free += m·c` 抵消负债 `−m·c`；
//! 做多 `free −= m·c`）。⟹ **无论短差 sizing 取多少，总 NAV 逐 bar 守恒**（`prove_nav_neutral`
//! 守），不需要跨级别现金搬运维持守恒。短差"借父级 1/3"= 借**size 参照**（units），损益落在 j 级
//! 自己的 `free`（其闲置预分配资金 + 全局 NAV 隐式背书超额亏损 = "借"的物质实现）。
//! 做空短差自融资（全额 1/3 父级）；做多短差需现金 ⟹ 受 j 级 `free` 上限约束（长/短资金非对称
//! 是 L0 会计事实，见 `project_bidirectional_accounting`，非 bug）。units 量变（翻转用当前 NAV
//! 重建）⟹ 全局 Σ|units| 不守恒，保留更本质的 NAV 中性。
//!
//! ## 边界算子：1x 逐仓强平（每级别独立，作用于任意持仓含短差腿）
//! 多头 c≤basis/2 平掉剩半；空头 c≥2·basis 爆仓归零。强平后 → Empty，下个 BSP 重新进入耦合/独立逻辑。
//!
//! ## 认识论等级
//! - Z₂ 状态机 / 翻转·短差 NAV 中性 / 几何塔由 sizing 涌现 / top-down 咬合：**L0**；
//! - BSP fire 时机 / sizing 取 1/3 / 平均分配：**L2**；回测 alpha：**L3**（耦合叠加，可否证）。

use crate::trading::types::{Polarity, INITIAL_CAPITAL, LADDER_MOVE, MAX_LADDER};

use crate::fugue_v3::accounting::record_trade;
use crate::fugue_v3::layer::FugueResult;
use crate::fugue_v3::prove::prove_nav_neutral;
use crate::fugue_v3::{EQUITY_SAMPLE_BARS, SUB_LIQ_FACTOR};

/// T level 0 在 ladder 空间的基准（= 走势级，move(L1)）。T level k ↦ ladder k + BASE_LADDER。
pub const BASE_LADDER: usize = LADDER_MOVE;

/// 可能的级别槽位数（ladder ∈ `[BASE_LADDER, MAX_LADDER)`，stream 层 `ladder>=MAX_LADDER` 丢弃）。
/// = 8（ladder 3..10）。每个槽位预分配等额资金。
pub const N_LEVEL_SLOTS: usize = MAX_LADDER - BASE_LADDER;

/// **单级别资金额度**（平均分配，第一版）。隔离为函数 ⟹ 后续迭代为加权（高 level 大仓）只改此处。
/// `ladder` 参数为加权预留（当前平均分配不读）。**有效域 L0**（设计选择，非定理）。
fn level_capital(_ladder: usize) -> f64 {
    INITIAL_CAPITAL / N_LEVEL_SLOTS as f64
}

// ════════════════════════════ 单级别 Z₂ 状态 ════════════════════════════

/// 单级别 k 的离散状态（3 态 Z₂ 滤波器）。units/basis/free 是会计真值。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Z2 {
    /// 未入场（首个 BSP 前 / 空头爆仓后）。
    Empty,
    /// 满仓做多。
    Long,
    /// 满仓做空。
    Short,
}

impl Z2 {
    /// 该状态的持仓方向（Empty → None）。
    fn direction(self) -> Option<Polarity> {
        match self {
            Z2::Empty => None,
            Z2::Long => Some(Polarity::Long),
            Z2::Short => Some(Polarity::Short),
        }
    }

    /// 方向 → 占用态。
    fn of(d: Polarity) -> Self {
        match d {
            Polarity::Long => Z2::Long,
            Polarity::Short => Z2::Short,
        }
    }
}

/// **腿的出生途径**（诊断归因，与盈亏极性正交）。
/// - `Entry`：独立模式首次入场（或强平重入）。
/// - `Flip`：独立模式翻转建仓（平旧腿同价建反向）。
/// - `Sink`：**短差机动仓**——父级持仓时反父向 BSP 开的 1/3 父级 units 对冲腿（耦合途径）。
///
/// dump 经 `trade_origins` 平行数组导出（layer.rs 契约："entry"/"flip"/"sink"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegOrigin {
    Entry,
    Flip,
    Sink,
}

impl LegOrigin {
    fn as_str(self) -> &'static str {
        match self {
            LegOrigin::Entry => "entry",
            LegOrigin::Flip => "flip",
            LegOrigin::Sink => "sink",
        }
    }
}

/// 极性翻转（短差方向 = 父级方向的反向；ε 镜像）。本地 helper（改动收敛在 t_engine）。
#[inline]
fn opp(p: Polarity) -> Polarity {
    match p {
        Polarity::Long => Polarity::Short,
        Polarity::Short => Polarity::Long,
    }
}

// ════════════════════════════ 单级别独立滤波器 ════════════════════════════

/// **单个级别的独立 Z₂ 状态机**（自带资金池/会计，与其他级别零耦合）。
///
/// NAV_k = `free` + sign(direction)·`units`·c（Empty 时 = free）。每个操作（enter/flip/close）
/// 在同价 c 下 NAV 中性。`capital` 是初始分配资金（PnL 基准），`funded` 标记是否已注资。
#[derive(Debug, Clone, Copy)]
struct LevelEngine {
    /// 真实 ladder（= T level + BASE_LADDER；trade 行 ladder 字段 = 此值，PnL 按级别分离）。
    ladder: usize,
    state: Z2,
    /// 当前持仓 |units| ≥ 0（direction 携符号）。
    units: f64,
    /// 加权入场价（短差 P&L 锚 + 1x 强平基准）。Empty 时 NaN。
    basis: f64,
    /// 当前 chunk 入场 bar（trade 行锚）。Empty 时 −1。
    entry_bar: i64,
    /// 该级别现金腿（NAV 中性双重会计的现金侧）。
    free: f64,
    /// 当前占用腿的出生途径（诊断归因）。
    origin: LegOrigin,
}

impl LevelEngine {
    /// 注资构造：分配 `level_capital(ladder)` 现金，初始 Empty。
    fn funded(ladder: usize) -> Self {
        LevelEngine {
            ladder,
            state: Z2::Empty,
            units: 0.0,
            basis: f64::NAN,
            entry_bar: -1,
            free: level_capital(ladder),
            origin: LegOrigin::Entry,
        }
    }

    /// 空槽位（ladder < BASE_LADDER，永不涌现 BSP，零资金，不计入 NAV 暴露）。
    fn empty_slot(ladder: usize) -> Self {
        LevelEngine {
            ladder,
            state: Z2::Empty,
            units: 0.0,
            basis: f64::NAN,
            entry_bar: -1,
            free: 0.0,
            origin: LegOrigin::Entry,
        }
    }

    /// 该级别 NAV（free + sign·units·c）。
    fn nav(&self, c: f64) -> f64 {
        self.free
            + match self.state {
                Z2::Empty => 0.0,
                Z2::Long => self.units * c,
                Z2::Short => -self.units * c,
            }
    }

    /// **平仓**（关闭当前腿）：按 direction 双重会计 + record trade + 累计该级别已实现 PnL。
    /// 平仓后 free = 该级别全 NAV（Empty 态），state → Empty。无占用时 no-op。
    fn close(&mut self, bar: i64, c: f64, reason: &'static str, res: &mut FugueResult) {
        let d = match self.state.direction() {
            Some(d) => d,
            None => return,
        };
        if self.units <= 1e-12 {
            self.state = Z2::Empty;
            return;
        }
        let pnl = match d {
            Polarity::Long => self.units * (c - self.basis),
            Polarity::Short => self.units * (self.basis - c),
        };
        match d {
            Polarity::Long => self.free += self.units * c,
            Polarity::Short => self.free -= self.units * c,
        }
        record_trade(
            res,
            self.ladder,
            self.entry_bar.max(0),
            self.basis,
            bar,
            c,
            self.units,
            reason,
            d,
        );
        res.trade_origins.push(self.origin.as_str());
        res.mobile_realized_pnl_by_ladder[self.ladder] += pnl;
        self.units = 0.0;
        self.basis = f64::NAN;
        self.entry_bar = -1;
        self.state = Z2::Empty;
    }

    /// **开仓**（用全部 free 建 `dir` 方向仓）：NAV 中性双重会计。前提 state==Empty。
    /// free≤0（空头爆仓后）⟹ 无可建仓，留 Empty（该级别死亡）。
    fn open(&mut self, dir: Polarity, bar: i64, c: f64, origin: LegOrigin) {
        debug_assert_eq!(self.state, Z2::Empty, "open 前提：Empty");
        if self.free <= 1e-12 || c <= 0.0 {
            return;
        }
        let m = self.free / c;
        if m <= 1e-12 || !m.is_finite() {
            return;
        }
        match dir {
            Polarity::Long => self.free -= m * c,
            Polarity::Short => self.free += m * c,
        }
        self.units = m;
        self.basis = c;
        self.entry_bar = bar;
        self.state = Z2::of(dir);
        self.origin = origin;
    }

    /// **开指定 units 仓**（短差机动仓用：`target_units` 锚在父级 units 的 1/3，非 `free/c`）。
    /// NAV 中性双重会计。前提 state==Empty。长/短资金非对称（L0 会计事实，非 bug）：
    /// - 做空自融资（收到 `m·c` 现金）⟹ 全额建 `target_units`；
    /// - 做多需现金 `m·c` ⟹ 受本级别 `free` 上限约束（`m = min(target, free/c)`，超额则部分建仓）。
    ///
    /// 守恒与 sizing 解耦：无论 `target_units` 取多少，open 在价 c 上 NAV 中性，总 NAV 守恒。
    fn open_sized(&mut self, dir: Polarity, target_units: f64, bar: i64, c: f64, origin: LegOrigin) {
        debug_assert_eq!(self.state, Z2::Empty, "open_sized 前提：Empty");
        if target_units <= 1e-12 || c <= 0.0 {
            return;
        }
        let m = match dir {
            // 做空自融资 ⟹ 全额；做多受现金约束 ⟹ cap 至 free/c。
            Polarity::Short => target_units,
            Polarity::Long => target_units.min(self.free / c),
        };
        if m <= 1e-12 || !m.is_finite() {
            return;
        }
        match dir {
            Polarity::Long => self.free -= m * c,
            Polarity::Short => self.free += m * c,
        }
        self.units = m;
        self.basis = c;
        self.entry_bar = bar;
        self.state = Z2::of(dir);
        self.origin = origin;
    }

    /// 单级别步进：强平 → 父级状态分派（独立翻转 / 短差耦合）。
    ///
    /// `parent`：上级别（ladder j+1）的 `(方向, units)`——活跃时 `Some`，空仓/顶层 `None`。
    /// `None` → 独立 Z₂ 翻转（自己资金池）；`Some` → 短差耦合（机动仓锚父级 1/3）。
    fn step(
        &mut self,
        buy: bool,
        sell: bool,
        bar: i64,
        c: f64,
        parent: Option<(Polarity, f64)>,
        res: &mut FugueResult,
    ) {
        // ── A. 边界算子：1x 逐仓强平（独立）──
        if let Some(d) = self.state.direction() {
            if self.units > 1e-12 {
                let liq = match d {
                    Polarity::Long => c > 0.0 && c <= self.basis / SUB_LIQ_FACTOR,
                    Polarity::Short => c >= SUB_LIQ_FACTOR * self.basis,
                };
                if liq {
                    self.close(bar, c, "liq", res);
                    res.n_liquidations_by_ladder[self.ladder] += 1;
                }
            }
        }

        // ── B. 父级状态分派：独立翻转 / 短差耦合 ──
        match parent {
            None => self.step_independent(buy, sell, bar, c, res),
            Some((pdir, punits)) => self.step_coupled(buy, sell, bar, c, pdir, punits, res),
        }
    }

    /// **独立翻转**（无活跃父级：顶层 / 父级空仓）——3 态 Z₂，用自己资金池，永远在市场。
    fn step_independent(&mut self, buy: bool, sell: bool, bar: i64, c: f64, res: &mut FugueResult) {
        match self.state {
            Z2::Empty => {
                // 首个 BSP（或强平后重入）决定方向，用剩余 free 建仓。
                if buy {
                    self.open(Polarity::Long, bar, c, LegOrigin::Entry);
                    if self.state == Z2::Long {
                        res.n_entries_by_ladder[self.ladder] += 1;
                    }
                } else if sell {
                    self.open(Polarity::Short, bar, c, LegOrigin::Entry);
                    if self.state == Z2::Short {
                        res.n_entries_by_ladder[self.ladder] += 1;
                    }
                }
            }
            Z2::Long => {
                if sell {
                    // 翻转：平多 + 用当前 NAV 全建空（永远在市场）。
                    self.close(bar, c, "flip", res);
                    self.open(Polarity::Short, bar, c, LegOrigin::Flip);
                    res.n_core_clears_by_ladder[self.ladder] += 1; // 复用为翻转计数
                }
                // buy = 同向 no-op
            }
            Z2::Short => {
                if buy {
                    // ε 镜像翻转：平空 + 用当前 NAV 全建多。
                    self.close(bar, c, "flip", res);
                    self.open(Polarity::Long, bar, c, LegOrigin::Flip);
                    res.n_core_clears_by_ladder[self.ladder] += 1;
                }
                // sell = 同向 no-op
            }
        }
    }

    /// **短差耦合**（活跃父级 `pdir`/`punits`）——本级别 = 父级核心仓的 H¹ 机动仓。
    ///
    /// 短差方向 = 反父向（`opp(pdir)`，ε 镜像）。reduce-BSP（反父向：父多→卖点 / 父空→买点）
    /// 开短差（1/3 父级 units）；restore-BSP（同父向）平短差（回补归还）。父级主导 ⟹ 本级别
    /// **不独立翻转**（不会建到父级量级的反向核心），只在 ±1/3 父级 units 间做机动短差。
    fn step_coupled(
        &mut self,
        buy: bool,
        sell: bool,
        bar: i64,
        c: f64,
        pdir: Polarity,
        punits: f64,
        res: &mut FugueResult,
    ) {
        // reduce = 反父向 BSP（减仓信号）；restore = 同父向 BSP（回补信号）。
        let (is_reduce, is_restore) = match pdir {
            Polarity::Long => (sell, buy),  // 父多：卖点减仓、买点回补
            Polarity::Short => (buy, sell), // 父空：买点减仓、卖点回补
        };
        let mobile_dir = opp(pdir); // 短差方向（反父向）

        match self.state.direction() {
            None => {
                // 空仓 + reduce-BSP → 开短差机动仓（反父向，1/3 父级 units）。
                if is_reduce {
                    self.open_sized(mobile_dir, punits / 3.0, bar, c, LegOrigin::Sink);
                    if self.state != Z2::Empty {
                        res.n_cycle_opens_by_ladder[self.ladder] += 1;
                    }
                }
                // restore-BSP 空仓 → no-op（不与父级同向 pyramid）。
            }
            Some(d) if d == mobile_dir => {
                // 持反父向仓（短差机动仓）→ restore-BSP 平仓（回补归还父级）。
                if is_restore {
                    self.close(bar, c, "recover", res);
                    res.n_cycle_closes_by_ladder[self.ladder] += 1;
                }
                // reduce-BSP 已 engaged → no-op。
            }
            Some(_) => {
                // 持同父向仓（子先于父建仓的遗留独立 core）→ reduce-BSP 平仓减暴露
                // （父级主导，不翻空），排空后回归纯短差循环。
                if is_reduce {
                    self.close(bar, c, "drain", res);
                }
                // restore-BSP 同父向 → no-op（不 pyramid）。
            }
        }
    }
}

// ════════════════════════════ 本 bar 信号视图 ════════════════════════════

/// 本 bar 信号视图（从 T 全塔 BSP diff 构造，stream 层填充）。
///
/// 索引空间 = **ladder**（= T level + BASE_LADDER）。纯 BSP 驱动：不区分 type1/2/3（都是
/// 「该级别买/卖点」），级别涌现信息隐含在「哪个 ladder 有 BSP」中。
#[derive(Debug, Clone)]
pub struct TSignalView {
    /// 本 bar 该 ladder 是否新增**任意**买点（type1/2/3 不分）。
    pub buy: [bool; MAX_LADDER],
    /// 本 bar 该 ladder 是否新增**任意**卖点。
    pub sell: [bool; MAX_LADDER],
}

impl TSignalView {
    /// 空信号（无 BSP）。
    pub fn empty() -> Self {
        TSignalView { buy: [false; MAX_LADDER], sell: [false; MAX_LADDER] }
    }
}

impl Default for TSignalView {
    fn default() -> Self {
        Self::empty()
    }
}

// ════════════════════════════ T 操作层引擎 ════════════════════════════

/// T 操作层引擎（多尺度独立滤波器版：每涌现级别一个独立 Z₂ 状态机，零跨级别耦合）。
pub struct TPositionEngine {
    res: FugueResult,
    /// 每 ladder 一个独立滤波器（索引 = ladder）。`ladder >= BASE_LADDER` 已注资，其余空槽。
    levels: Vec<LevelEngine>,
    last_close: f64,
    max_concurrent_seen: usize,
}

impl Default for TPositionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TPositionEngine {
    pub fn new() -> Self {
        let levels = (0..MAX_LADDER)
            .map(|k| {
                if k >= BASE_LADDER {
                    LevelEngine::funded(k)
                } else {
                    LevelEngine::empty_slot(k)
                }
            })
            .collect();
        TPositionEngine {
            res: FugueResult::default(),
            levels,
            last_close: f64::NAN,
            max_concurrent_seen: 0,
        }
    }

    pub fn result(&self) -> &FugueResult {
        &self.res
    }

    pub fn n_trades(&self) -> usize {
        self.res.trades.len()
    }

    /// 总 NAV（Σ 各级别 NAV）。
    fn total_nav(&self, c: f64) -> f64 {
        self.levels.iter().map(|le| le.nav(c)).sum()
    }

    /// 物理暴露（Σ 多头 units, Σ 空头 units）。
    fn exposure(&self) -> (f64, f64) {
        let mut lu = 0.0;
        let mut su = 0.0;
        for le in &self.levels {
            if le.units > 1e-12 {
                match le.state {
                    Z2::Long => lu += le.units,
                    Z2::Short => su += le.units,
                    Z2::Empty => {}
                }
            }
        }
        (lu, su)
    }

    /// 活跃空头声部数（当前做空的级别数）。
    fn active_voices(&self) -> usize {
        self.levels.iter().filter(|le| le.units > 1e-12 && le.state == Z2::Short).count()
    }

    /// 状态快照: (nav, long_units, short_units, n_active_voices)。
    pub fn snapshot(&self) -> (f64, f64, f64, usize) {
        let c = if self.last_close.is_finite() { self.last_close } else { 0.0 };
        let (lu, su) = self.exposure();
        (self.total_nav(c), lu, su, self.active_voices())
    }

    // ──────────────── 单 bar 步进 ────────────────

    /// 单 bar 操作步进：每个 ladder 的滤波器独立处理自己 ladder 的 BSP（强平 → 翻转/入场）。
    /// 各级别零耦合 ⟹ 顺序无关（同 bar 各 ladder 互不影响）。
    pub fn step(&mut self, view: &TSignalView, bar: i64, price: f64) {
        let c = price;
        self.last_close = c;
        let nav_pre = self.total_nav(c);

        // ── BSP 分派：top-down（高 ladder 先更新 ⟹ 子级读父级本 bar 最新状态，齿轮咬合）──
        for k in (0..MAX_LADDER).rev() {
            // 空槽位（ladder < BASE_LADDER，零资金）跳过——它们不会有 BSP 也无暴露。
            if self.levels[k].free <= 0.0 && self.levels[k].state == Z2::Empty {
                continue;
            }
            // 父级 = 上级别（ladder k+1）。活跃 → Some((方向, units))；空仓/顶层 → None（独立翻转）。
            let parent = if k + 1 < MAX_LADDER {
                let p = self.levels[k + 1];
                match p.state.direction() {
                    Some(d) if p.units > 1e-12 => Some((d, p.units)),
                    _ => None,
                }
            } else {
                None
            };
            self.levels[k].step(view.buy[k], view.sell[k], bar, c, parent, &mut self.res);
        }

        // ── 守卫：全局 NAV 中性（同价 c 全部操作前后中性）──
        let nav_post = self.total_nav(c);
        prove_nav_neutral(nav_pre, nav_post, bar);

        // ── 观测 ──
        let (long_u, short_u) = self.exposure();
        if long_u > 0.0 {
            self.res.phys_long_bars += 1;
        }
        if short_u > 0.0 {
            self.res.phys_short_bars += 1;
        }
        for le in &self.levels {
            if le.units > 1e-12 && le.state == Z2::Short {
                self.res.short_held_bars_by_ladder[le.ladder.min(MAX_LADDER - 1)] += 1;
            }
        }
        // 相邻同向占用级别对数（T24 观测，非 panic 不变量）。
        let mut chiral = 0u64;
        for k in 0..MAX_LADDER.saturating_sub(1) {
            let a = &self.levels[k];
            let b = &self.levels[k + 1];
            if a.units > 1e-12 && b.units > 1e-12 && a.state == b.state {
                chiral += 1;
            }
        }
        self.res.max_chiral_same_dir = self.res.max_chiral_same_dir.max(chiral);
        self.max_concurrent_seen = self.max_concurrent_seen.max(self.active_voices());

        if bar % EQUITY_SAMPLE_BARS == 0 {
            self.res.equity.push((bar, self.total_nav(c)));
            self.res.exposure_series.push((bar, long_u, short_u));
        }
    }

    /// 收尾（末 bar equity 补采样 + 全级别平仓到现金）。`last_bar=None`（零 bar）⇒ final_nav = Σ free。
    pub fn finish(&mut self, last_bar: Option<i64>) {
        if let Some(lb) = last_bar {
            let c_last = self.last_close;
            if lb % EQUITY_SAMPLE_BARS != 0 {
                self.res.equity.push((lb, self.total_nav(c_last)));
                let (lu, su) = self.exposure();
                self.res.exposure_series.push((lb, lu, su));
            }
            // 各级别独立平仓（eod 收尾）。
            for k in 0..self.levels.len() {
                if self.levels[k].units > 1e-12 {
                    self.levels[k].close(lb, c_last, "eod", &mut self.res);
                }
            }
        }
        // final_nav = Σ 各级别现金（全平后 free 即净值）。
        self.res.final_nav = self.levels.iter().map(|le| le.free).sum();
        self.res.max_concurrent_voices = self.max_concurrent_seen as u64;
        // 严格对齐验收：每条 trade 必有且仅有一个出生途径标签。
        assert_eq!(
            self.res.trade_origins.len(),
            self.res.trades.len(),
            "trade_origins 与 trades 长度失配（漏标/多标出生途径）"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buy_view(k: usize) -> TSignalView {
        let mut v = TSignalView::empty();
        v.buy[k] = true;
        v
    }

    fn sell_view(k: usize) -> TSignalView {
        let mut v = TSignalView::empty();
        v.sell[k] = true;
        v
    }

    /// 单级别资金 = INITIAL/8。
    fn cap() -> f64 {
        INITIAL_CAPITAL / N_LEVEL_SLOTS as f64
    }

    // ──────────────── 入场 ────────────────

    #[test]
    fn 入场买点用本级别资金做多() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let units = cap() / 100.0;
        assert!((eng.levels[5].units - units).abs() < 1e-9, "本级别满仓 {units} units");
        assert_eq!(eng.levels[5].state, Z2::Long);
        // 总 NAV 守恒（入场 NAV 中性）= INITIAL_CAPITAL。
        let (navv, lu, su, _) = eng.snapshot();
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "总 NAV 守恒，得 {navv}");
        assert!((lu - units).abs() < 1e-9);
        assert_eq!(su, 0.0);
    }

    #[test]
    fn 入场卖点用本级别资金做空() {
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0);
        assert_eq!(eng.levels[5].state, Z2::Short);
        let (navv, lu, su, voices) = eng.snapshot();
        assert_eq!(lu, 0.0);
        assert!((su - cap() / 100.0).abs() < 1e-9);
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "做空入场 NAV 中性");
        assert_eq!(voices, 1, "1 个做空声部");
    }

    // ──────────────── 翻转 ────────────────

    #[test]
    fn 翻转_同级别卖点long翻short永远在市场() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // Long@5
        eng.step(&sell_view(5), 10, 110.0); // 翻 Short@5
        assert_eq!(eng.levels[5].state, Z2::Short, "翻转到 Short");
        let (lu, su) = eng.exposure();
        assert_eq!(lu, 0.0, "多头清掉");
        assert!(su > 0.0, "建了反向空头");
        // 多头 100→110 升值已实现 ⟹ 该级别 NAV = cap*1.1。
        let nav5 = eng.levels[5].nav(110.0);
        assert!((nav5 - cap() * 1.1).abs() < 1e-4, "翻转后本级别 NAV=cap×1.1，得 {nav5}");
    }

    #[test]
    fn 翻转_往返多空多() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        eng.step(&sell_view(5), 10, 110.0); // → Short
        assert_eq!(eng.levels[5].state, Z2::Short);
        eng.step(&buy_view(5), 20, 105.0); // → Long
        assert_eq!(eng.levels[5].state, Z2::Long);
        let (_, su) = eng.exposure();
        assert_eq!(su, 0.0, "往返回多头");
    }

    #[test]
    fn 翻转_short同级别买点翻long() {
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0);
        eng.step(&buy_view(5), 10, 90.0); // 空头降价盈利后翻多
        assert_eq!(eng.levels[5].state, Z2::Long);
        let mob = eng.result().mobile_realized_pnl_by_ladder[5];
        assert!(mob > 0.0, "空头 100→90 降价翻多盈利，得 {mob}");
    }

    // ──────────────── 同向 no-op ────────────────

    #[test]
    fn 同级别买点持多no_op() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let n_before = eng.n_trades();
        let u_before = eng.levels[5].units;
        eng.step(&buy_view(5), 10, 110.0);
        assert_eq!(eng.n_trades(), n_before, "同向同级别 BSP 无 trade");
        assert!((eng.levels[5].units - u_before).abs() < 1e-12, "持仓不变");
        assert_eq!(eng.levels[5].state, Z2::Long);
    }

    // ──────────────── 跨级别耦合（多重赋格核心新语义）────────────────

    #[test]
    fn 耦合_父持多次级别卖点做短差不减父仓() {
        // ladder6 核心多（父级空→独立），ladder5（子=ladder6）收卖点 → 开短差空 1/3 父级 units，
        // 父级核心仓**不动**（短差是独立机动腿，净多头敞口 = 父 − 1/3父 = 2/3父 = 减仓 1/3）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // ladder6 核心 Long
        let u6 = eng.levels[6].units;
        eng.step(&sell_view(5), 10, 100.0); // ladder5 短差空（父6持多）
        assert!((eng.levels[6].units - u6).abs() < 1e-12, "父级核心仓不动");
        assert_eq!(eng.levels[5].state, Z2::Short, "子级开短差空");
        assert!((eng.levels[5].units - u6 / 3.0).abs() < 1e-9, "短差 = 1/3 父级 units");
        let (lu, su) = eng.exposure();
        assert!((lu - u6).abs() < 1e-9 && (su - u6 / 3.0).abs() < 1e-9, "净敞口 2/3 父（减仓 1/3）");
    }

    #[test]
    fn 耦合_父持多次级别买点平短差回补() {
        // 接上：ladder5 短差空后收买点 → 平短差（回补归还），降价盈利；父级核心仓不受短差循环影响。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        eng.step(&sell_view(5), 10, 100.0); // 短差空 @100
        eng.step(&buy_view(5), 20, 90.0); // 平短差 @90（父6仍持多）
        assert_eq!(eng.levels[5].state, Z2::Empty, "短差回补归还 → 空仓");
        let mob = eng.result().mobile_realized_pnl_by_ladder[5];
        assert!(mob > 0.0, "短差空 100→90 降价回补盈利，得 {mob}");
        assert_eq!(eng.levels[6].state, Z2::Long, "父级核心仓不受短差循环影响");
    }

    #[test]
    fn 耦合_父空仓子独立翻转用自己资金池() {
        // ladder5 父级（6）空 → 独立翻转（全仓，非 1/3 短差量级）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // 独立 Long（父6空）
        let full = eng.levels[5].units;
        eng.step(&sell_view(5), 10, 110.0); // 独立翻空（全仓）
        assert_eq!(eng.levels[5].state, Z2::Short, "父空 → 独立翻空");
        assert!(eng.levels[5].units > full * 0.9, "独立翻转是全仓，非 1/3 短差");
    }

    #[test]
    fn 耦合_父持多子同向遗留core被减仓不翻空() {
        // 子先于父建仓（独立 Long），父后涌现持多 → 子收卖点 = 减仓排空（drain），**不翻空**
        //（父级主导，子不建反向核心）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // ladder5 独立 Long（父6空）
        eng.step(&buy_view(6), 1, 100.0); // ladder6 涌现 Long → 成为 ladder5 的父
        eng.step(&sell_view(5), 10, 100.0); // ladder5 同父向遗留 core + 卖点 → drain
        assert_eq!(eng.levels[5].state, Z2::Empty, "遗留 core 被减仓排空，未翻空");
        assert_eq!(eng.levels[6].state, Z2::Long, "父级不受影响");
    }

    #[test]
    fn 耦合_四步循环完整齿轮咬合() {
        // ① 高级别买点建核心多 → ② 次级别卖点减仓1/3做空 → ③ 次级别买点平空回补 → ④ 高级别卖点翻空。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // ① ladder6 核心 Long
        assert_eq!(eng.levels[6].state, Z2::Long);
        eng.step(&sell_view(5), 10, 105.0); // ② ladder5 短差空（1/3）
        assert_eq!(eng.levels[5].state, Z2::Short);
        eng.step(&buy_view(5), 20, 100.0); // ③ ladder5 平空回补
        assert_eq!(eng.levels[5].state, Z2::Empty);
        eng.step(&sell_view(6), 30, 120.0); // ④ ladder6 核心翻空（父级空→独立）
        assert_eq!(eng.levels[6].state, Z2::Short, "核心仓翻空，方向变");
    }

    // ──────────────── 边界 / 守恒 ────────────────

    #[test]
    fn 空输入零操作守恒() {
        let mut eng = TPositionEngine::new();
        for i in 0..5 {
            eng.step(&TSignalView::empty(), i, 100.0);
        }
        eng.finish(Some(4));
        assert!(eng.result().trades.is_empty(), "无信号无 trade");
        assert!((eng.result().final_nav - INITIAL_CAPITAL).abs() < 1e-6, "总资金守恒");
    }

    #[test]
    fn 强平后重新入场() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // Long@5 @100
        eng.step(&TSignalView::empty(), 10, 50.0); // c≤basis/2 强平多头
        assert_eq!(eng.levels[5].state, Z2::Empty, "多头强平");
        assert_eq!(eng.res.n_liquidations_by_ladder[5], 1);
        // 强平剩半（NAV=cap/2），下个买点重入。
        eng.step(&buy_view(5), 20, 50.0);
        assert_eq!(eng.levels[5].state, Z2::Long, "强平后重新入场");
        assert!(eng.levels[5].units > 1e-9);
    }

    #[test]
    fn 空头价格翻倍爆仓nav归零() {
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0); // Short@5 @100
        eng.step(&TSignalView::empty(), 10, 200.0); // c≥2×basis 爆仓
        assert_eq!(eng.res.n_liquidations_by_ladder[5], 1);
        assert_eq!(eng.levels[5].state, Z2::Empty);
        let nav5 = eng.levels[5].nav(200.0);
        assert!(nav5.abs() < 1e-6, "空头翻倍爆仓本级别 NAV→0，得 {nav5}");
        // 爆仓后 free≈0 ⟹ 无法重入（该级别死亡）。
        eng.step(&sell_view(5), 20, 200.0);
        assert_eq!(eng.levels[5].state, Z2::Empty, "资金死光无法重入");
    }

    #[test]
    fn 总nav守恒_短差耦合同价不创造价值() {
        // 父级核心 + 子级短差开/平 + 核心翻转，全部同价 c=100 ⟹ 总 NAV 恒 = INITIAL
        //（NAV 中性逐 bar 由 step 内 prove 守；守恒与短差 sizing 解耦）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // 核心多
        eng.step(&sell_view(5), 5, 100.0); // 短差空（同价）
        eng.step(&buy_view(5), 10, 100.0); // 平短差（同价）
        eng.step(&sell_view(6), 15, 100.0); // 核心翻空（同价）
        let (navv, _, _, _) = eng.snapshot();
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "同价耦合操作总 NAV 守恒，得 {navv}");
    }

    #[test]
    fn 资金分配_八槽位平均() {
        let eng = TPositionEngine::new();
        let total: f64 = eng.levels.iter().map(|le| le.free).sum();
        assert!((total - INITIAL_CAPITAL).abs() < 1e-6, "总分配 = INITIAL_CAPITAL");
        // ladder < BASE_LADDER 零资金。
        for k in 0..BASE_LADDER {
            assert_eq!(eng.levels[k].free, 0.0, "ladder {k} 空槽零资金");
        }
        // ladder >= BASE_LADDER 等额。
        for k in BASE_LADDER..MAX_LADDER {
            assert!((eng.levels[k].free - cap()).abs() < 1e-9, "ladder {k} 等额 cap");
        }
    }
}
