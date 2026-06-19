//! **T 操作层引擎**（多尺度独立滤波器版：每涌现级别一个独立 Z₂ 状态机）。
//!
//! ## 本次重写（架构转变：单一 core 链顶 → 每级别独立滤波器）
//! 上一版（4 状态翻转版）用 `core_ladder()`（最高占用 ladder = 链顶）把**全部**级别的反向
//! BSP 汇聚到唯一一个 core 上做整仓 M=N 翻转（`flip_core` 平掉全塔 + 建等量反向核心），并通过
//! 跨级别 `sink/recover` 做短差。后果（见记忆 `project_t_flip_vs_clear_verdict` /
//! `project_t_short_close_level_mismatch`）：「一个 core 在 ladder 3 翻来翻去」+ 机械 1:1
//! 多空对冲 ⟹ mdd 巨大、做空腿是亏损唯一来源。
//!
//! 本版彻底改为 **N 个独立的 Z₂ 滤波器**：每个 ladder 一个 [`LevelEngine`]，各自持有独立资金池
//! /状态/持仓，**只响应自己 ladder 的 BSP** 做翻转。没有链顶集中、没有跨级别 sink/recover 耦合
//! （275号局部依赖的极致：每级别只管自己的 BSP，连相邻级别都不碰）。
//!
//! ## 每级别 = 一个 Z₂ 滤波器（3 态）
//! `Empty`（未入场）/ `Long`（满多）/ `Short`（满空）。每个 `LevelEngine[k]` 独立运转：
//! ```text
//! Empty   --[buy@k]-->  Long      （首个 BSP 决定初始方向，用本级别分配资金建仓）
//! Empty   --[sell@k]--> Short
//! Long    --[sell@k]--> Short      （翻转：平多 + 用当前 NAV 全建反向空，永远在市场）
//! Short   --[buy@k]-->  Long       （ε 镜像翻转）
//! Long    --[buy@k]-->  Long       （同向 no-op）
//! Short   --[sell@k]--> Short       （同向 no-op）
//! ```
//! 多尺度滤波器：ladder 低（笔/段/走势级）→ BSP 密集 → 高频翻转 = 短差；ladder 高（中枢套
//! 中枢）→ BSP 稀疏 → 低频翻转 = 趋势腿。**短差不靠显式 sink**——由低 ladder 的高频独立翻转
//! 自然涌现。总 PnL = Σ_k LevelEngine[k] 的已实现盈亏（`mobile_realized_pnl_by_ladder[k]`）。
//!
//! ## 资金分配（平均分配，第一版；隔离为 `level_capital()` 便于迭代）
//! 预分配 [`N_LEVEL_SLOTS`] 个槽位（ladder ∈ `[BASE_LADDER, MAX_LADDER)`），每槽
//! `INITIAL_CAPITAL / N_LEVEL_SLOTS`。未涌现的 level 资金闲置在 `free` 中（计入 NAV，守恒诚实）。
//! 总 NAV 初始 = `INITIAL_CAPITAL`。**有效域 L0**（资金分配是设计选择，非定理；加权 vs 平均的
//! alpha 差异由 L3 回测甄别）。
//!
//! ## 单级别翻转会计（NAV 中性，永远在市场）
//! 翻转 @ 价 c = `close`（平旧腿，record trade，free ← 当前 NAV）+ `open`（用全部 free 建反向）。
//! 同价 c 下 NAV 中性（手算：close(Long) free+=u·c 抵消市值 −u·c；open(Short) free+=m·c 抵消
//! 负债 −m·c）。units 量变（翻转用**当前 NAV**≠初始资金重建，盈亏累积改变 units）⟹ 全局
//! Σ|units| **不守恒**（独立翻转池非 M=N），故不再 `prove_conservation`；保留更本质的 NAV 中性。
//!
//! ## 边界算子：1x 逐仓强平（每级别独立）
//! 多头 c≤basis/2 平掉剩半（保护性平仓，NAV=cap/2）；空头 c≥2·basis 爆仓归零（NAV=0，该
//! level 资金死光）。强平后 → Empty，下个 BSP 用剩余 free 重新入场（Empty 的唯一非首入来源）。
//! 各级别强平互不影响（独立池）。
//!
//! ## 认识论等级
//! - Z₂ 状态机结构 / 翻转 NAV 中性 / 多尺度独立性：**L0**；
//! - BSP fire 时机 / 平均分配 vs 加权：**L2**；回测 alpha：**L3**（独立滤波器叠加，可否证）。

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

/// **腿的出生途径**（诊断归因，与盈亏极性正交）。本版无跨级别 sink ⟹ 只有 `Entry`（首次入场
/// /强平重入）与 `Flip`（翻转建仓）两种途径。dump 经 `trade_origins` 平行数组导出。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegOrigin {
    Entry,
    Flip,
}

impl LegOrigin {
    fn as_str(self) -> &'static str {
        match self {
            LegOrigin::Entry => "entry",
            LegOrigin::Flip => "flip",
        }
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

    /// 单级别步进：强平 → BSP 翻转/入场。返回本次新增 trade 数（诊断）。
    fn step(&mut self, buy: bool, sell: bool, bar: i64, c: f64, res: &mut FugueResult) {
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

        // ── B. BSP 翻转 / 入场（Z₂ 状态机）──
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

        // ── BSP 分派：每个 ladder 的滤波器独立步进 ──
        for k in 0..MAX_LADDER {
            // 空槽位（ladder < BASE_LADDER，零资金）跳过——它们不会有 BSP 也无暴露。
            if self.levels[k].free <= 0.0 && self.levels[k].state == Z2::Empty {
                continue;
            }
            self.levels[k].step(view.buy[k], view.sell[k], bar, c, &mut self.res);
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

    // ──────────────── 多尺度独立性（核心新语义）────────────────

    #[test]
    fn 多级别独立_不同ladder互不影响() {
        // ladder 5 做多、ladder 4 做空、ladder 6 做多——三个独立滤波器同时持仓。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        eng.step(&sell_view(4), 1, 100.0);
        eng.step(&buy_view(6), 2, 100.0);
        assert_eq!(eng.levels[5].state, Z2::Long);
        assert_eq!(eng.levels[4].state, Z2::Short);
        assert_eq!(eng.levels[6].state, Z2::Long);
        // ladder 5 翻转不影响 ladder 4/6。
        eng.step(&sell_view(5), 10, 100.0);
        assert_eq!(eng.levels[5].state, Z2::Short, "ladder5 翻空");
        assert_eq!(eng.levels[4].state, Z2::Short, "ladder4 不受影响");
        assert_eq!(eng.levels[6].state, Z2::Long, "ladder6 不受影响");
        let (lu, su) = eng.exposure();
        assert!(lu > 0.0 && su > 0.0, "多空并存（多尺度叠加）");
    }

    #[test]
    fn 多级别独立_次级别bsp不触发父级别() {
        // ladder 5 做多，ladder 4 收到卖点——只翻转 ladder 4（入场空），ladder 5 不减仓（无跨级别 sink）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let u5_before = eng.levels[5].units;
        eng.step(&sell_view(4), 10, 110.0);
        assert!((eng.levels[5].units - u5_before).abs() < 1e-12, "ladder5 持仓不动（无 sink 耦合）");
        assert_eq!(eng.levels[4].state, Z2::Short, "ladder4 独立入场空");
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
    fn 总nav守恒_翻转不创造价值() {
        // 多级别翻转后，同价 c 总 NAV 应等于价格驱动的真值（NAV 中性由 step 内 prove 守）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        eng.step(&buy_view(4), 0, 100.0);
        eng.step(&sell_view(5), 10, 100.0); // 同价翻转，NAV 不变
        let (navv, _, _, _) = eng.snapshot();
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "同价翻转总 NAV 守恒，得 {navv}");
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
