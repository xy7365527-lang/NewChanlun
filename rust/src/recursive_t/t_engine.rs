//! **T 操作层引擎**（双向耦合多重赋格版：单核心 + 区间套 sink/recover 短差执行层）。
//!
//! ## 本次重写（修正上一版 c06db11942 的耦合方向错误）
//! 上一版用「子级开独立 short 腿、父级 units 不动」实现耦合——这是**错的**（用户裁决 2026-06-19）：
//! 那让子级是独立仓位、父级不减仓，丢失了 sink 的本质。本版改为**正确的双向耦合**（= 已验证的
//! ε 对称 sink/recover 几何塔，记忆 `project_t_operation_self_replication` 里 OKLO +382 的赢家
//! 语义），照搬 `fugue_v3::cycle`/`accounting` 的验证会计，但用更简的编排（无 σ-ascend/root，单核心
//! = 最高活跃级别，parent = 最近活跃祖先）。
//!
//! ## 双向耦合（用户 5 点）
//! 1. **父级减仓下放**：父级持多时子级卖点 ⇒ 父级**真减仓 1/3**（`reduce_at`），把这 1/3「下放」次级
//!    别做空（`add_at` flip 方向）。父级仍持多（剩 2/3），不翻空。
//! 2. **子级不独立翻转**：子级是父级核心仓的**短差执行层**（H¹ 机动仓），不是独立仓位——子级永远
//!    不独立 enter/flip。
//! 3. **双向**：信号自下而上涌现（BSP 在各 ladder 出现），操作约束自上而下（区间套：子级语义由
//!    父级状态决定）。
//! 4. **向上查父级**：子级收 BSP ⇒ 查最近活跃祖先 P。P 持多 ⇒ 子级卖点=sink 做空 / 买点=recover 回补；
//!    P 持空 ⇒ 子级买点=sink 做多 / 卖点=recover 回补（ε 镜像）。
//! 5. **只有最高涌现级别独立翻转**：无活跃祖先的级别（= 最高活跃，core）才 enter/flip；其余全是子级。
//!
//! ## 三个 τ 原子（不硬编码四步循环，四步是涌现序列）
//! - **sink**（子级 j 收反父向 BSP，父级 P 活跃）：`reduce_at(P, m=u_P/3)` + `add_at(j, m, flip(d_P))`。
//!   父级减 1/3，次级别开反向短差 m。Δexposure：父 u_P→2u_P/3，子 0→u_P/3 反向 ⟹ 净敞口 = 2/3 父 − 1/3 父。
//! - **recover**（子级 j 收同父向 BSP，j 持反父向短差）：`reduce_at(j, m=u_j/3)` + `add_at(P, m, d_P)`。
//!   次级别平 1/3 短差，升回父级。完整 sink→recover 净 free += 2m(c_sink − c_recover)（穿 ε=−1 降成本 alpha）。
//! - **drain**（子级持**同父向**遗留仓，中间级别插入后出现）：反父向 BSP ⇒ `reduce_at(j, u_j/3)` 减暴露
//!   （父级主导不翻转），排空后回归纯短差。
//!
//! ## 核心仓（最高活跃级别，独立翻转）
//! 无活跃祖先的 BSP = 核心级信号：空仓 ⇒ enter（用全部 free）；同向且更高空 ladder ⇒ ascend（relabel
//! 上移，骑乘最高涌现级别，无新资金）；反向 ⇒ flip（clear 全塔到现金 + 同 ladder 反向 enter）。
//!
//! ## 仓位递归（几何塔，由 sink sizing 自然涌现）
//! sink 转移 = 父级 units/3 ⟹ 次级别 = 核心 1/3、次次级别 = 1/9 …… 相邻级别方向相反（手性交替，
//! sink 穿 ε=−1）。高级别大仓吃趋势、低级别小仓做短差。
//!
//! ## 会计（单一共享 free 池，复用 `fugue_v3::accounting`）
//! NAV = free + Σ_k sign(d_k)·u_k·c。每个 reduce_at/add_at 在同价 c 上 NAV 中性 ⟹ 总 NAV 逐 bar 守恒
//! （`prove_nav_neutral`）。Σ|units| 仅 enter/flip/clear/liq 改，sink/recover 只级间转移（守恒）。
//! 单一 free（非上一版的每级别独立池）——sink/recover 的资金在级别间流动，单池是诚实建模。
//!
//! ## 边界算子：1x 逐仓强平（每层独立，作用于任意持仓含短差腿）
//! 多头 c≤basis/2 平掉；空头 c≥2·basis 爆仓。强平 → 该层归零现金。
//!
//! ## 认识论等级
//! - sink/recover/flip 会计 NAV 中性 / 几何塔涌现 / 手性交替 / 区间套 top-down：**L0**；
//! - sizing 1/3（MOBILE_FRAC）/ core 升降编排 / BSP fire 时机：**L2**；回测 alpha：**L3**（可否证）。

use crate::trading::types::{Polarity, INITIAL_CAPITAL, LADDER_MOVE, MAX_LADDER};

use crate::fugue_v3::accounting::{
    active_voice_count, add_at, exposure, flip, mobile_quota, nav, reduce_at,
};
use crate::fugue_v3::layer::{FugueResult, Layer};
use crate::fugue_v3::prove::prove_nav_neutral;
use crate::fugue_v3::{EQUITY_SAMPLE_BARS, SUB_LIQ_FACTOR};

/// T level 0 在 ladder 空间的基准（= 走势级，move(L1)）。T level k ↦ ladder k + BASE_LADDER。
pub const BASE_LADDER: usize = LADDER_MOVE;

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

/// T 操作层引擎（双向耦合：单核心 + 区间套 sink/recover 短差执行层）。
///
/// `layers[k]` 是 ladder k 的净仓位（复用 `fugue_v3::Layer`：单一 direction，相邻级别方向相反）。
/// `free` 是**单一共享现金池**（初始 = `INITIAL_CAPITAL`）——sink/recover 在级别间转移资金，单池诚实。
pub struct TPositionEngine {
    res: FugueResult,
    /// 每 ladder 一个净仓位（索引 = ladder）。idle 时 units=0。
    layers: Vec<Layer>,
    /// 单一共享现金池（NAV = free + Σ sign(d)·u·c）。
    free: f64,
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
        let layers = (0..MAX_LADDER).map(Layer::idle).collect();
        TPositionEngine {
            res: FugueResult::default(),
            layers,
            free: INITIAL_CAPITAL,
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

    /// 总 NAV（free + Σ sign(d)·u·c）。
    fn total_nav(&self, c: f64) -> f64 {
        nav(&self.layers, self.free, c)
    }

    /// 最高活跃级别（= 核心仓所在 ladder）。无活跃仓 → None。
    fn highest_active(&self) -> Option<usize> {
        (0..MAX_LADDER).rev().find(|&k| self.layers[k].is_active())
    }

    /// 级别 j 的最近活跃祖先（严格更高的第一个活跃 ladder）。无 → None（j 是核心级）。
    fn nearest_active_parent(&self, j: usize) -> Option<usize> {
        (j + 1..MAX_LADDER).find(|&k| self.layers[k].is_active())
    }

    /// 状态快照: (nav, long_units, short_units, n_active_voices)。
    pub fn snapshot(&self) -> (f64, f64, f64, usize) {
        let c = if self.last_close.is_finite() { self.last_close } else { 0.0 };
        let (lu, su) = exposure(&self.layers);
        (self.total_nav(c), lu, su, active_voice_count(&self.layers))
    }

    // ──────────────── τ 原子（复用 accounting 会计）────────────────

    /// **enter**：核心级空仓首次建仓，用**全部 free** 建 `dir` 方向核心仓。
    fn enter(&mut self, j: usize, dir: Polarity, bar: i64, c: f64) {
        if c <= 0.0 {
            return;
        }
        let m = self.free / c;
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        add_at(&mut self.layers, j, m, dir, &mut self.free, c, bar, &mut self.res);
        self.res.n_entries_by_ladder[j] += 1;
    }

    /// **clear_all**：全塔平仓到现金（flip 前 / eod）。
    fn clear_all(&mut self, bar: i64, c: f64, reason: &'static str) {
        for k in 0..MAX_LADDER {
            let u = self.layers[k].units;
            if u > 1e-12 {
                reduce_at(&mut self.layers, k, u, &mut self.free, c, bar, &mut self.res, reason);
            }
        }
    }

    /// **ascend**：核心仓 relabel 上移（from→to，to 须 idle）——骑乘最高涌现级别，无新资金（NAV 不变）。
    /// 不变量：`to` 必须 idle（区间套核心上移不可覆盖活跃层，否则 silent 覆盖 = NAV 破裂）——
    /// release 也 fail-loud（守恒关键）。调用点 `route_bsp` 已保证 `!is_active(to)`。
    fn ascend(&mut self, from: usize, to: usize) {
        assert!(
            !self.layers[to].is_active(),
            "ascend 目标 ladder {to} 非 idle（units={}）：核心上移不可覆盖活跃层",
            self.layers[to].units
        );
        let mut moved = self.layers[from];
        moved.ladder = to;
        self.layers[to] = moved;
        self.layers[from] = Layer::idle(from);
    }

    /// **sink @ (parent→sub)**（σ⁻¹∘τ）：父级减仓 m=u_P/3，次级别开 flip(d_P) 短差 m。父级真减仓（剩 2/3）。
    fn sink(&mut self, parent: usize, sub: usize, bar: i64, c: f64) {
        let u_p = self.layers[parent].units;
        let pdir = self.layers[parent].direction;
        let m = mobile_quota(u_p);
        if !(m > 1e-12 && m.is_finite()) || m > u_p + 1e-9 {
            return;
        }
        let mob = flip(pdir);
        // 子层方向相容（add_at 层内单一 direction 不变量）：sub 空 或 已是 mob 方向才可加。
        if self.layers[sub].is_active() && self.layers[sub].direction != mob {
            return;
        }
        reduce_at(&mut self.layers, parent, m, &mut self.free, c, bar, &mut self.res, "reduce");
        add_at(&mut self.layers, sub, m, mob, &mut self.free, c, bar, &mut self.res);
        self.res.n_cycle_opens_by_ladder[parent] += 1;
        self.res.cross_level_closures += 1;
    }

    /// **recover @ (sub→parent)**（σ∘τ，ε 对称）：次级别平短差 m=u_sub/3，升回父级 d_P 方向。
    fn recover(&mut self, parent: usize, sub: usize, bar: i64, c: f64) {
        let pdir = self.layers[parent].direction;
        let mob = flip(pdir);
        // ε 对称：sub 必须持反父向短差（mob）才能升回。
        if !self.layers[sub].is_active() || self.layers[sub].direction != mob {
            return;
        }
        let u_s = self.layers[sub].units;
        let m = mobile_quota(u_s);
        if !(m > 1e-12 && m.is_finite()) || m > u_s + 1e-9 {
            return;
        }
        reduce_at(&mut self.layers, sub, m, &mut self.free, c, bar, &mut self.res, "recover");
        add_at(&mut self.layers, parent, m, pdir, &mut self.free, c, bar, &mut self.res);
        self.res.n_cycle_closes_by_ladder[parent] += 1;
    }

    /// **drain @ j**：子级持同父向遗留仓 → 反父向 BSP 减暴露 1/3（不翻转，父级主导）。
    fn drain(&mut self, j: usize, bar: i64, c: f64) {
        let u = self.layers[j].units;
        let m = mobile_quota(u);
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        reduce_at(&mut self.layers, j, m, &mut self.free, c, bar, &mut self.res, "drain");
    }

    // ──────────────── BSP 路由（区间套 top-down）────────────────

    /// 单个 BSP（ladder j，is_buy）的路由：查父级 → 子级 sink/recover/drain；无父级 → 核心 enter/ascend/flip。
    fn route_bsp(&mut self, j: usize, is_buy: bool, bar: i64, c: f64) {
        match self.nearest_active_parent(j) {
            // ── 子级（有活跃祖先 P）：区间套约束，永不独立翻转 ──
            Some(p) => {
                let pdir = self.layers[p].direction;
                let mob = flip(pdir); // 短差方向 = 反父向
                let is_reduce = match pdir {
                    Polarity::Long => !is_buy,  // 父多：卖点=减仓信号
                    Polarity::Short => is_buy,  // 父空：买点=减仓信号
                };
                if is_reduce {
                    // 反父向 BSP：sink（空/已是短差）或 drain（遗留同父向仓）。
                    if !self.layers[j].is_active() || self.layers[j].direction == mob {
                        self.sink(p, j, bar, c);
                    } else {
                        self.drain(j, bar, c);
                    }
                } else {
                    // 同父向 BSP：recover（j 持短差则平 1/3 升回）；否则 no-op（不 pyramid）。
                    if self.layers[j].is_active() && self.layers[j].direction == mob {
                        self.recover(p, j, bar, c);
                    }
                }
            }
            // ── 核心级（无活跃祖先：j 是最高活跃或全空）：唯一独立翻转点 ──
            None => {
                let dir = if is_buy { Polarity::Long } else { Polarity::Short };
                match self.highest_active() {
                    None => self.enter(j, dir, bar, c), // 首次建仓
                    Some(cc) => {
                        let cdir = self.layers[cc].direction;
                        if dir == cdir {
                            // 同向：更高空 ladder ⇒ ascend 骑乘；同 ladder ⇒ no-op。
                            if j > cc && !self.layers[j].is_active() {
                                self.ascend(cc, j);
                            }
                        } else {
                            // 反向：核心翻转（走势终完美→新走势）——清全塔 + 同 ladder 反向 enter。
                            self.clear_all(bar, c, "flip");
                            self.enter(j, dir, bar, c);
                        }
                    }
                }
            }
        }
    }

    // ──────────────── 单 bar 步进 ────────────────

    /// 单 bar 操作步进：强平 → BSP top-down 路由（高 ladder 先更新 ⟹ 子级读父级本 bar 最新状态）。
    pub fn step(&mut self, view: &TSignalView, bar: i64, price: f64) {
        let c = price;
        self.last_close = c;
        let nav_pre = self.total_nav(c);

        // ── A. 边界算子：1x 逐仓强平（每层独立）──
        for k in 0..MAX_LADDER {
            let l = self.layers[k];
            if l.units > 1e-12 {
                let liq = match l.direction {
                    Polarity::Long => c > 0.0 && c <= l.basis / SUB_LIQ_FACTOR,
                    Polarity::Short => c >= SUB_LIQ_FACTOR * l.basis,
                };
                if liq {
                    reduce_at(&mut self.layers, k, l.units, &mut self.free, c, bar, &mut self.res, "liq");
                    self.res.n_liquidations_by_ladder[k] += 1;
                }
            }
        }

        // ── B. BSP 路由：top-down（高 ladder 先，区间套约束自上而下）──
        for j in (0..MAX_LADDER).rev() {
            let b = view.buy[j];
            let s = view.sell[j];
            if b && s {
                continue; // 同 bar 同 ladder 买卖冲突 → 跳过（歧义）
            }
            if b {
                self.route_bsp(j, true, bar, c);
            } else if s {
                self.route_bsp(j, false, bar, c);
            }
        }

        // ── 守卫：全局 NAV 中性（同价 c 全部操作前后中性）──
        prove_nav_neutral(nav_pre, self.total_nav(c), bar);

        // ── 观测 ──
        let (long_u, short_u) = exposure(&self.layers);
        if long_u > 0.0 {
            self.res.phys_long_bars += 1;
        }
        if short_u > 0.0 {
            self.res.phys_short_bars += 1;
        }
        for l in &self.layers {
            if l.units > 1e-12 && l.direction == Polarity::Short {
                self.res.short_held_bars_by_ladder[l.ladder.min(MAX_LADDER - 1)] += 1;
            }
        }
        // 相邻同向占用级别对数（T24 观测：几何塔应手性交替 ⟹ 同向对 = 异常提示，非 panic）。
        let mut chiral = 0u64;
        for k in 0..MAX_LADDER.saturating_sub(1) {
            let a = &self.layers[k];
            let b = &self.layers[k + 1];
            if a.units > 1e-12 && b.units > 1e-12 && a.direction == b.direction {
                chiral += 1;
            }
        }
        self.res.max_chiral_same_dir = self.res.max_chiral_same_dir.max(chiral);
        self.max_concurrent_seen = self.max_concurrent_seen.max(active_voice_count(&self.layers));

        if bar % EQUITY_SAMPLE_BARS == 0 {
            self.res.equity.push((bar, self.total_nav(c)));
            self.res.exposure_series.push((bar, long_u, short_u));
        }
    }

    /// 收尾（末 bar equity 补采样 + 全塔平仓到现金）。`last_bar=None`（零 bar）⇒ final_nav = free。
    pub fn finish(&mut self, last_bar: Option<i64>) {
        if let Some(lb) = last_bar {
            let c_last = self.last_close;
            if lb % EQUITY_SAMPLE_BARS != 0 {
                self.res.equity.push((lb, self.total_nav(c_last)));
                let (lu, su) = exposure(&self.layers);
                self.res.exposure_series.push((lb, lu, su));
            }
            self.clear_all(lb, c_last, "eod");
        }
        // 全平后 free = 净值（所有仓位转现金）。
        self.res.final_nav = self.free;
        self.res.max_concurrent_voices = self.max_concurrent_seen as u64;
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

    // ──────────────── 核心仓 enter / flip / ascend ────────────────

    #[test]
    fn 核心_首个买点全仓建多() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        assert_eq!(eng.layers[6].direction, Polarity::Long);
        let units = INITIAL_CAPITAL / 100.0;
        assert!((eng.layers[6].units - units).abs() < 1e-6, "全仓 {units} units（单一 free 池）");
        let (navv, _, _, _) = eng.snapshot();
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "建仓 NAV 中性");
    }

    #[test]
    fn 核心_最高级别反向翻转清全塔() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // 核心 Long@6
        eng.step(&sell_view(6), 10, 110.0); // 最高级别卖点 → 翻空
        assert_eq!(eng.layers[6].direction, Polarity::Short, "核心翻空");
        let (_, lu, su) = (0.0, exposure(&eng.layers).0, exposure(&eng.layers).1);
        assert_eq!(lu, 0.0, "多头清掉");
        assert!(su > 0.0, "建反向空");
    }

    #[test]
    fn 核心_同向更高级别ascend骑乘上移() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // 核心 Long@5
        let u5 = eng.layers[5].units;
        eng.step(&buy_view(7), 5, 100.0); // 更高级别同向买点 → ascend 上移到 7
        assert!(!eng.layers[5].is_active(), "原核心 5 已上移（idle）");
        assert_eq!(eng.layers[7].direction, Polarity::Long);
        assert!((eng.layers[7].units - u5).abs() < 1e-9, "ascend 仅 relabel，units 不变（无新资金）");
    }

    // ──────────────── 子级 sink（父级真减仓，核心新语义）────────────────

    #[test]
    fn sink_父持多子卖点父级真减仓三分之一() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // 核心 Long@6
        let u6 = eng.layers[6].units;
        eng.step(&sell_view(5), 10, 100.0); // 子级 5 卖点（父6持多）→ sink
        // 父级**真减仓** 1/3（关键：上一版错在父级不动）。
        assert!((eng.layers[6].units - u6 * 2.0 / 3.0).abs() < 1e-6, "父级减到 2/3，得 {}", eng.layers[6].units);
        assert_eq!(eng.layers[5].direction, Polarity::Short, "次级别开反向短差");
        assert!((eng.layers[5].units - u6 / 3.0).abs() < 1e-6, "短差 = 1/3 父级 units（下放）");
        // 净多头敞口 = 2/3 父；空 = 1/3 父。
        let (lu, su) = exposure(&eng.layers);
        assert!((lu - u6 * 2.0 / 3.0).abs() < 1e-6 && (su - u6 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn recover_父持多子买点平短差升回父级() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        eng.step(&sell_view(5), 10, 100.0); // sink：父6→2/3，子5 短差空 1/3
        let u6_after_sink = eng.layers[6].units;
        let u5_short = eng.layers[5].units;
        eng.step(&buy_view(5), 20, 90.0); // 子级买点 → recover（平 1/3 短差，升回父级）
        // 子级短差减 1/3。
        assert!((eng.layers[5].units - u5_short * 2.0 / 3.0).abs() < 1e-6, "短差平掉 1/3");
        // 父级升回 1/3 子级 units。
        assert!(eng.layers[6].units > u6_after_sink, "父级升回");
        // 短差空 100→90 降价回补盈利。
        let mob = eng.result().mobile_realized_pnl_by_ladder[5];
        assert!(mob > 0.0, "短差降价回补盈利，得 {mob}");
    }

    #[test]
    fn 子级永不独立翻转_父持多子卖点是sink非独立做空() {
        // 子级 5 在父级 6 持多时收卖点：必须是 sink（父级减仓），而非独立全仓做空。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        let u6 = eng.layers[6].units;
        eng.step(&sell_view(5), 10, 100.0);
        // 若是独立做空，子级 units 会是 free/c 量级（远大于 1/3 父级）；sink 则恰好 1/3 父级。
        assert!((eng.layers[5].units - u6 / 3.0).abs() < 1e-6, "子级短差恰 1/3 父级（非独立全仓）");
        assert!(eng.layers[6].units < u6, "父级被减仓（非独立做空时父级不动）");
    }

    // ──────────────── 四步循环 + 手性几何塔 ────────────────

    #[test]
    fn 四步循环_完整齿轮咬合() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // ① 高级别买点建核心多
        let u6 = eng.layers[6].units;
        eng.step(&sell_view(5), 10, 105.0); // ② 次级别卖点 → 父减仓 1/3 + 次级别做空
        assert!((eng.layers[6].units - u6 * 2.0 / 3.0).abs() < 1e-6);
        assert_eq!(eng.layers[5].direction, Polarity::Short);
        eng.step(&buy_view(5), 20, 100.0); // ③ 次级别买点 → recover 平空回补升回
        assert_eq!(eng.layers[6].direction, Polarity::Long, "核心仍持多");
        eng.step(&sell_view(6), 30, 120.0); // ④ 高级别卖点 → 核心翻空
        assert_eq!(eng.layers[6].direction, Polarity::Short, "核心方向变");
    }

    #[test]
    fn 手性几何塔_相邻级别方向相反() {
        // 核心 Long@6 → sink 到 5（Short）→ 5 作父 sink 到 4（Long）。塔：6 Long / 5 Short / 4 Long。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        eng.step(&sell_view(5), 10, 100.0); // 5 = flip(Long) = Short（父6多→卖点sink）
        let u5_at_sink = eng.layers[5].units; // 4 从 5 sink 前，5 的 units
        // 5 现持 Short；对 4 而言父级是 5（Short），4 买点 = 反父向(父空买点=减仓) → sink 做多。
        eng.step(&buy_view(4), 20, 100.0);
        assert_eq!(eng.layers[6].direction, Polarity::Long);
        assert_eq!(eng.layers[5].direction, Polarity::Short, "相邻反向");
        assert_eq!(eng.layers[4].direction, Polarity::Long, "手性交替");
        // 几何塔：4 从 5 sink ⟹ u4 = u5(sink前)/3，u5 被减到 2/3（父级真减仓）。
        assert!((eng.layers[4].units - u5_at_sink / 3.0).abs() < 1e-6, "次次级别 = sink时次级别 1/3");
        assert!((eng.layers[5].units - u5_at_sink * 2.0 / 3.0).abs() < 1e-6, "次级别被次次级别 sink 减到 2/3");
    }

    // ──────────────── 守恒 / 边界 ────────────────

    #[test]
    fn 总nav守恒_sink_recover同价不创造价值() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // 核心多
        eng.step(&sell_view(5), 5, 100.0); // sink（同价）
        eng.step(&buy_view(5), 10, 100.0); // recover（同价）
        eng.step(&sell_view(6), 15, 100.0); // 核心翻空（同价）
        let (navv, _, _, _) = eng.snapshot();
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "同价全操作总 NAV 守恒，得 {navv}");
    }

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
    fn 核心多头强平到现金() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // Long@6 @100
        eng.step(&TSignalView::empty(), 10, 50.0); // c≤basis/2 强平多头
        assert!(!eng.layers[6].is_active(), "多头强平归零");
        assert_eq!(eng.res.n_liquidations_by_ladder[6], 1);
    }

    #[test]
    fn 短差空头价格翻倍爆仓() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0); // 核心多
        eng.step(&sell_view(5), 10, 100.0); // 子5 短差空 @100
        assert_eq!(eng.layers[5].direction, Polarity::Short);
        eng.step(&TSignalView::empty(), 20, 200.0); // c≥2×basis 短差空爆仓
        assert_eq!(eng.res.n_liquidations_by_ladder[5], 1, "短差空腿强平");
        assert!(!eng.layers[5].is_active(), "短差归零");
    }

    #[test]
    fn 收尾全平到现金final_nav() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(6), 0, 100.0);
        eng.step(&sell_view(5), 10, 100.0);
        eng.finish(Some(10));
        // 全平后所有层 idle。
        assert!(eng.layers.iter().all(|l| !l.is_active()), "收尾全平");
        assert!((eng.result().final_nav - INITIAL_CAPITAL).abs() < 1e-4, "同价全平 final_nav 守恒");
    }
}
