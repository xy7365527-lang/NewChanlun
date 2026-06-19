//! **T 算子操作层引擎**（两层自我复制：操作逻辑 + 仓位结构，都是 T 的自我复制）。
//!
//! ## 核心命题：T 自我复制（aₙ=f(aₙ₋₁) 同时投影到操作层与仓位层）
//! T 算子在信号层是 `aₙ=f(aₙ₋₁)`（第65课）——所有级别共用同一 `f`。本引擎把这个不变性
//! 延伸到两个层：
//! 1. **操作逻辑自我复制**：每个级别独立运转完全相同的 sink/recover（无 root 方向）。
//! 2. **仓位结构自我复制**：1/3 几何塔——每个级别的仓位 = 上级别仓位的 1/3（卖点触发减仓
//!    下放）。`units[k] = units[k+1] / 3`（递归，σ-不变 Casimir）。
//!
//! ## 1/3 几何塔（仓位递归 = 自我复制）
//! - **卖点@k → sink**：从 level k 取 **1/3** units 下放到 k−1 做空（穿手性缝，flip 方向）。
//! - **买点@k → recover**：把 k−1 的短差**全平**，units 还给 k（缠论"次级别买点确认→回补"）。
//! - 卖点逐步下放（每次 1/3，几何递减塔），买点遇次级别底一次性收回。
//!
//! > 比例 1/3 当前为 L2 取值（fugue_v3 `MOBILE_FRAC=1/λ`，026:80「用其中的 1/3」⟹ λ=3）。
//! > 比例的**必然性推导**（为何是 1/3 而非其他）后续单独做——本引擎先用 1/3 跑通回测。
//!
//! ## 涌现接管（替代 σ-ascend——这是本次唯一的新机制）
//! 不需要 σ-ascend（fugue_v3 的方向检查式核心仓升级）。改为**涌现驱动**：
//! - 新级别涌现（T 迭代多一层，`emergent_ceiling` 增长）→ 新最高级别**接管总仓位**：
//!   核心 units 从原最高层 relabel 上移到新最高层（不交易，会计 relabel），原最高层变成
//!   新最高层的次级别，继续按 1/3 递归下放。
//! - 入场（现金→核心仓）：首次涌现时在最高涌现层用 `free/c` 全压建核心，方向由本 bar 最高
//!   信号层的 BSP 决定（buy→Long / sell→Short）。这是现金↔仓位的边界。
//!
//! ## 级别之间 = 递归嵌套（275号谱系：局部依赖原则）
//! - 本级别的短差是上级别仓位的 1/3（递归嵌套，附庸的附庸不是我的附庸）。
//! - 上级别不指挥本级别——每个级别只看自己 BSP，调用同一套 sink/recover。
//! - 全局仓位 = Σ_k layers[k] 自然涌现（非预设 core，无 root_direction）。
//!
//! ## 复用 fugue_v3 会计原语 vs 不复用（no-patch-mentality：声明=能力）
//! **复用**：`Layer` / `add_at` / `reduce_at`（NAV 中性双重会计）/ `sink_chunk`（1/3 配额 σ⁻¹∘τ
//! 下放，含 σ-quota/cross-level 守卫，与「卖点取 1/3 下放」逐字一致）/ `nav` / `exposure` /
//! `liquidate_*`（1x 逐仓边界 A）/ `record_trade`（trade11 契约）/ `prove_nav_neutral` /
//! `prove_recursive_consistency` / `prove_conservation` / `prove_cross_level_closure` /
//! `count_chiral_violations`。
//! **不复用**：`recover_chunk`（fugue_v3 的 1/3 recover——本引擎 recover **全平**次级别，
//! 用户「平空 units 还给 k」字面）/ `core_emergent_ladder`（σ-ascend，被涌现接管替代）/
//! `prove_epsilon_symmetry`（依赖 root_direction）/ `prove_no_double_act`（同级别同 bar 可被
//! sink+recover 两次触及，互斥假设不成立）。
//!
//! ## 守卫语义诚实标注（formalization-validity-domain）
//! - `prove_nav_neutral`：**L0 强守恒**（add/reduce 同价 c 中性）。每 bar panic。
//! - `prove_conservation`：**会计自洽**（n_base≡Σ|units|）。sink/recover 守恒（不改 n_base），
//!   仅入场/强平/eod 改 n_base——这不是 fugue_v3 单核心仓的 Σ|units|=const 强守恒。
//! - `prove_recursive_consistency`：仓位层 ≥ FIRST_BSP_LADDER（T 持仓 ≥ BASE_LADDER ≥ 2）。
//! - `count_chiral_violations`：相邻级别同向观测（非 panic；几何塔手性交替，同向是接力中段）。
//!
//! ## 认识论等级
//! - 两层自我复制结构 / 涌现接管 / NAV 中性：**L0**；BSP fire 时机：**L2**；回测 alpha：**L3**。

use crate::trading::types::{Polarity, LADDER_MOVE, MAX_LADDER};

use crate::fugue_v3::accounting::{
    accrue_held_bars, active_voice_count, add_at, exposure, flip, nav, reduce_at,
};
use crate::fugue_v3::cycle::{liquidate_long, liquidate_short, sink_chunk};
use crate::fugue_v3::layer::{FugueResult, Layer};
use crate::fugue_v3::prove::{
    count_chiral_violations, prove_conservation, prove_cross_level_closure, prove_nav_neutral,
    prove_recursive_consistency,
};
use crate::fugue_v3::{EQUITY_SAMPLE_BARS, INITIAL_CAPITAL, SUB_LIQ_FACTOR};

/// T level 0 在 ladder 空间的基准（= 走势级，move(L1)）。T level k ↦ ladder k + BASE_LADDER。
/// 也是**最低操作级别**：ladder == BASE_LADDER 的层不再 sink（伪代码 `if k>0`，无 T 次级别）。
pub const BASE_LADDER: usize = LADDER_MOVE;

/// 本 bar 信号视图（从 T 全塔 BSP diff + 涌现 ceiling 构造，stream 层填充）。
///
/// 索引空间 = **ladder**（= T level + BASE_LADDER）。新设计不区分 type1/2/3（都是"该级别
/// 买/卖点"）；`ceiling` 用于涌现接管（新级别涌现 = ceiling 增长）。
#[derive(Debug, Clone)]
pub struct TSignalView {
    /// 本 bar 该 ladder 是否新增**任意**买点（type1/2/3 不分）。
    pub buy: [bool; MAX_LADDER],
    /// 本 bar 该 ladder 是否新增**任意**卖点。
    pub sell: [bool; MAX_LADDER],
    /// T 涌现上界 r*（`emergent_ceiling`，完整走势级别数）；增长 = 新级别涌现。
    pub ceiling: usize,
}

impl TSignalView {
    /// 空信号（无 BSP）。`ceiling` 由 stream 缓存沿用（涌现态持续）。
    pub fn empty() -> Self {
        TSignalView { buy: [false; MAX_LADDER], sell: [false; MAX_LADDER], ceiling: 0 }
    }
}

impl Default for TSignalView {
    fn default() -> Self {
        Self::empty()
    }
}

/// T 操作层引擎（1/3 几何塔 + 涌现接管，无 root/core_polarity/σ-ascend）。
pub struct TPositionEngine {
    res: FugueResult,
    /// 每级别独立净仓位（ladder 索引；全局仓位 = Σ layers，涌现非预设）。
    layers: Vec<Layer>,
    /// 统一现金池。
    free: f64,
    /// 当前总持仓基数 Σ|units|（会计自洽锚；入场/强平/eod 改它，sink/recover 守恒）。
    n_base: f64,
    /// 当前最高涌现层 ladder（核心仓所在；0 = 未入场/无涌现层）。涌现接管单调上移。
    emergent_top: usize,
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
            n_base: 0.0,
            emergent_top: 0,
            last_close: f64::NAN,
            max_concurrent_seen: 0,
        }
    }

    /// 全局是否空仓（n_base≈0 ⟹ 资金全在 free）。
    fn global_flat(&self) -> bool {
        self.n_base <= 1e-9
    }

    pub fn result(&self) -> &FugueResult {
        &self.res
    }

    pub fn n_trades(&self) -> usize {
        self.res.trades.len()
    }

    /// 状态快照: (nav, long_units, short_units, n_active_voices)。
    pub fn snapshot(&self) -> (f64, f64, f64, usize) {
        let c = if self.last_close.is_finite() { self.last_close } else { 0.0 };
        let navv = nav(&self.layers, self.free, c);
        let (lu, su) = exposure(&self.layers);
        (navv, lu, su, active_voice_count(&self.layers))
    }

    /// **涌现接管**（替代 σ-ascend）：ceiling 增长 ⟹ 核心 units relabel 上移到新最高层。
    ///
    /// 新最高层 ladder = `(ceiling−1) + BASE_LADDER`。核心仓换级别标签（不交易，units/nav
    /// 不变），原最高层清空变成新最高层的次级别。无核心时仅更新 emergent_top（待入场）。
    fn takeover_on_emergence(&mut self, ceiling: usize) {
        if ceiling == 0 {
            return;
        }
        let new_top = (ceiling - 1 + BASE_LADDER).min(MAX_LADDER - 1);
        if new_top <= self.emergent_top {
            return; // 无新级别涌现（ceiling 持平或回缩——保守不降级核心）
        }
        let old = self.emergent_top;
        // 核心存在 ⟹ relabel 上移（会计 relabel，不产 trade）。
        if old >= BASE_LADDER && self.layers[old].units > 1e-12 && self.layers[new_top].units <= 1e-12 {
            let core = self.layers[old];
            self.layers[new_top] = Layer { ladder: new_top, ..core };
            self.layers[old] = Layer::idle(old);
        }
        self.emergent_top = new_top;
    }

    /// **入场**（现金→核心仓）：在最高涌现层用 free 全压建核心，方向由本 bar 最高信号决定。
    fn try_enter(&mut self, view: &TSignalView, bar: i64, c: f64) {
        if self.emergent_top < BASE_LADDER || self.free <= 0.0 || c <= 0.0 {
            return;
        }
        // 最高有信号层的 BSP 定核心方向（buy→Long / sell→Short）。
        for k in (0..MAX_LADDER).rev() {
            if view.buy[k] || view.sell[k] {
                let dir = if view.buy[k] { Polarity::Long } else { Polarity::Short };
                let m = self.free / c;
                if m > 1e-12 && m.is_finite() {
                    add_at(&mut self.layers, self.emergent_top, m, dir, &mut self.free, c, bar, &mut self.res);
                    self.n_base += m;
                    self.res.n_entries_by_ladder[self.emergent_top] += 1;
                }
                return;
            }
        }
    }

    /// **recover（买点@k）**：把次级别 k−1 的短差**全平**，units 还给本级别 k（flip 翻回）。
    ///
    /// 用户「买点@k → level_states[k−1] 平空，units 还给 level_states[k]」字面（全平，非 1/3）。
    /// 守恒：m 从 k−1 移到 k（reduce + add 同量），Σ|units| 不变（n_base 不动）。
    fn recover_full(&mut self, k: usize, bar: i64, c: f64) {
        let sub = k - 1;
        let u_sub = self.layers[sub].units;
        if u_sub <= 1e-12 {
            return;
        }
        let d_sub = self.layers[sub].direction;
        let want = flip(d_sub); // 还给 k 的方向（翻回核心侧）
        // 父层相容前提（add_at 层内单一方向不变量）：k 已占用须同向 want，否则操作不适用。
        if self.layers[k].units > 1e-12 && self.layers[k].direction != want {
            return;
        }
        prove_cross_level_closure(k, sub);
        reduce_at(&mut self.layers, sub, u_sub, &mut self.free, c, bar, &mut self.res, "recover");
        add_at(&mut self.layers, k, u_sub, want, &mut self.free, c, bar, &mut self.res);
        self.res.n_cycle_closes_by_ladder[k] += 1;
        self.res.cross_level_closures += 1;
    }

    /// 整仓清到现金（eod）。
    fn clear_to_cash(&mut self, bar: i64, c: f64, reason: &'static str) {
        for k in 0..self.layers.len() {
            if self.layers[k].units > 1e-12 {
                let u = self.layers[k].units;
                reduce_at(&mut self.layers, k, u, &mut self.free, c, bar, &mut self.res, reason);
            }
        }
        self.n_base = 0.0;
        self.emergent_top = 0;
    }

    /// 单 bar 操作步进：A 强平 → 涌现接管 → 入场 / 逐级别 sink·recover（1/3 几何塔）。
    pub fn step(&mut self, view: &TSignalView, bar: i64, price: f64) {
        let c = price;
        self.last_close = c;
        let nav_pre = nav(&self.layers, self.free, c);

        // ── A. 边界算子：1x 逐仓强平（多空对称；无 core 豁免）──
        for k in 0..self.layers.len() {
            let l = self.layers[k];
            if l.units <= 1e-12 {
                continue;
            }
            let liquidate = match l.direction {
                Polarity::Short => c >= SUB_LIQ_FACTOR * l.basis,
                Polarity::Long => c > 0.0 && c <= l.basis / SUB_LIQ_FACTOR,
            };
            if liquidate {
                let m = match l.direction {
                    Polarity::Short => liquidate_short(&mut self.layers, k, &mut self.free, c, bar, &mut self.res),
                    Polarity::Long => liquidate_long(&mut self.layers, k, &mut self.free, c, bar, &mut self.res),
                };
                self.n_base -= m;
                // emergent_top 是涌现**结构层**（由 ceiling 决定），不因强平归零——核心是否存在由
                // global_flat()/n_base 跟踪。强平后 n_base→0 ⟹ global_flat ⟹ 下个 BSP 在 emergent_top 重新入场。
            }
        }

        // ── 涌现接管（替代 σ-ascend）：新级别涌现 → 核心 relabel 上移 ──
        self.takeover_on_emergence(view.ceiling);

        // ── 入场（全局空仓）或 逐级别 sink·recover（1/3 几何塔，每级别自我复制）──
        if self.global_flat() {
            self.try_enter(view, bar, c);
        } else {
            // ladder 降序：高级别先下放，同 bar 确定性顺序。每级别同一套 sink/recover。
            for k in (0..MAX_LADDER).rev() {
                if k <= BASE_LADDER {
                    continue; // 塔底（T level 0）无次级别可 sink/recover
                }
                // 卖点@k → sink 1/3 下放 k−1（fugue_v3 sink_chunk：取 u_k/3，flip 翻向，含守卫）。
                if view.sell[k] && self.layers[k].units > 1e-12 {
                    sink_chunk(&mut self.layers, k, &mut self.free, c, bar, &mut self.res);
                }
                // 买点@k → recover 全平 k−1 还给 k。
                if view.buy[k] {
                    self.recover_full(k, bar, c);
                }
            }
        }

        // ── 必然性运行时证明（每 bar；violation = panic = 验收）──
        prove_recursive_consistency(&self.layers, bar);
        let chiral = count_chiral_violations(&self.layers) as u64;
        self.res.max_chiral_same_dir = self.res.max_chiral_same_dir.max(chiral);
        let nav_post = nav(&self.layers, self.free, c);
        prove_nav_neutral(nav_pre, nav_post, bar);
        prove_conservation(&self.layers, self.n_base, bar);

        // ── 观测 ──
        let (long_u, short_u) = exposure(&self.layers);
        if long_u > 0.0 {
            self.res.phys_long_bars += 1;
        }
        if short_u > 0.0 {
            self.res.phys_short_bars += 1;
        }
        accrue_held_bars(&mut self.res, &self.layers);
        self.max_concurrent_seen = self.max_concurrent_seen.max(active_voice_count(&self.layers));

        if bar % EQUITY_SAMPLE_BARS == 0 {
            self.res.equity.push((bar, nav(&self.layers, self.free, c)));
            // 真实净敞口采样（long_u/short_u 已在上方 exposure() 算出）——magnitude 真值源。
            self.res.exposure_series.push((bar, long_u, short_u));
        }
    }

    /// 收尾（末 bar equity 补采样 + 清仓到现金）。`last_bar=None`（零 bar）⇒ 仅设 final_nav=free。
    pub fn finish(&mut self, last_bar: Option<i64>) {
        if let Some(lb) = last_bar {
            let c_last = self.last_close;
            if lb % EQUITY_SAMPLE_BARS != 0 {
                self.res.equity.push((lb, nav(&self.layers, self.free, c_last)));
                let (lu, su) = exposure(&self.layers);
                self.res.exposure_series.push((lb, lu, su));
            }
            if !self.global_flat() {
                self.clear_to_cash(lb, c_last, "eod");
            }
        }
        self.res.final_nav = self.free;
        self.res.max_concurrent_voices = self.max_concurrent_seen as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ceiling=c 的买点视图（建仓/sink 用）。
    fn buy_view(k: usize, ceiling: usize) -> TSignalView {
        let mut v = TSignalView::empty();
        v.buy[k] = true;
        v.ceiling = ceiling;
        v
    }

    fn sell_view(k: usize, ceiling: usize) -> TSignalView {
        let mut v = TSignalView::empty();
        v.sell[k] = true;
        v.ceiling = ceiling;
        v
    }

    #[test]
    fn 涌现入场核心多头nav中性() {
        let mut eng = TPositionEngine::new();
        // ceiling=2 ⟹ 最高涌现层 ladder4。买点 → 入场 Long@ladder4 全压。
        eng.step(&buy_view(4, 2), 0, 100.0);
        let (navv, lu, su, _) = eng.snapshot();
        assert!((lu - 1000.0).abs() < 1e-6, "满仓 1000 units，得 {lu}");
        assert_eq!(su, 0.0);
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "入场 NAV 中性");
        assert_eq!(eng.emergent_top, 4, "核心在最高涌现层 ladder4");
        assert!((eng.layers[4].units - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn 卖点sink三分之一下放次级别做空() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(4, 2), 0, 100.0); // 核心 ladder4 Long 1000
        let base = eng.n_base;
        // 卖点@ladder4 → sink 1/3 下放 ladder3 做空。
        eng.step(&sell_view(4, 2), 10, 110.0);
        let (_, lu, su, _) = eng.snapshot();
        assert!((lu - base * 2.0 / 3.0).abs() < 1e-6, "核心剩 2/3 Long，得 {lu}");
        assert!((su - base / 3.0).abs() < 1e-6, "次级别 1/3 Short 短差，得 {su}");
        assert_eq!(eng.layers[3].direction, Polarity::Short);
        // Σ|units| 守恒（sink 不改 n_base）。
        assert!((eng.n_base - base).abs() < 1e-9, "sink 守恒");
        assert!((lu + su - base).abs() < 1e-6);
    }

    #[test]
    fn 买点recover全平次级别还给核心() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(4, 2), 0, 100.0); // 核心 ladder4 Long 1000
        let base = eng.n_base;
        eng.step(&sell_view(4, 2), 10, 110.0); // sink → ladder3 Short 333, core 667
        // 买点@ladder4 → recover 全平 ladder3 还给核心。
        eng.step(&buy_view(4, 2), 20, 105.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(su, 0.0, "次级别空头全平");
        assert!((lu - base).abs() < 1e-6, "核心恢复全仓 1000，得 {lu}");
        // 空头 110→105 降价回补盈利。
        let mob: f64 = eng.result().mobile_realized_pnl_by_ladder.iter().sum();
        assert!(mob > 0.0, "次级别空头降价回补盈利，得 {mob}");
    }

    #[test]
    fn 几何塔多次sink递减() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(4, 2), 0, 100.0); // core 1000
        eng.step(&sell_view(4, 2), 10, 110.0); // sink: core 667, sub 333
        eng.step(&sell_view(4, 2), 20, 112.0); // sink: core 667*2/3=444, sub +667/3=222 → 555
        let (_, lu, su, _) = eng.snapshot();
        assert!((lu - 1000.0 * 4.0 / 9.0).abs() < 1e-3, "core 4/9，得 {lu}");
        assert!((su - 1000.0 * 5.0 / 9.0).abs() < 1e-3, "sub 5/9，得 {su}");
        assert!((eng.n_base - 1000.0).abs() < 1e-6, "几何塔守恒");
    }

    #[test]
    fn 涌现接管核心上移不交易() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(4, 2), 0, 100.0); // 核心 ladder4
        let n_trades_before = eng.n_trades();
        // ceiling 增到 3 ⟹ 新最高层 ladder5，核心 relabel 上移。
        eng.step(&TSignalView { ceiling: 3, ..TSignalView::empty() }, 5, 100.0);
        assert_eq!(eng.emergent_top, 5, "涌现接管：核心上移 ladder5");
        assert!((eng.layers[5].units - 1000.0).abs() < 1e-6, "核心 units 不变");
        assert_eq!(eng.layers[4].units, 0.0, "原最高层清空");
        assert_eq!(eng.n_trades(), n_trades_before, "relabel 不产 trade");
    }

    #[test]
    fn 向下涌现入场做空() {
        let mut eng = TPositionEngine::new();
        // 卖点 → 入场 Short 核心。
        eng.step(&sell_view(4, 2), 0, 100.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0);
        assert!((su - 1000.0).abs() < 1e-6, "入场满仓 Short");
        assert_eq!(eng.layers[4].direction, Polarity::Short);
        // 买点@ladder4 → sink? 不，核心 Short 时卖点 sink/买点 recover 仍按 flip 手性。
        // 这里测核心 Short 的 sink：卖点@ladder4 取 1/3 下放 ladder3 flip(Short)=Long。
        eng.step(&sell_view(4, 2), 10, 90.0);
        let (_, lu2, su2, _) = eng.snapshot();
        assert!((su2 - 1000.0 * 2.0 / 3.0).abs() < 1e-6, "核心剩 2/3 Short");
        assert!((lu2 - 1000.0 / 3.0).abs() < 1e-6, "次级别 1/3 Long（flip 手性）");
    }

    #[test]
    fn 空输入零操作守恒() {
        let mut eng = TPositionEngine::new();
        for i in 0..5 {
            eng.step(&TSignalView::empty(), i, 100.0);
        }
        eng.finish(Some(4));
        assert!(eng.result().trades.is_empty(), "无信号无 trade");
        assert!((eng.result().final_nav - INITIAL_CAPITAL).abs() < 1e-6);
    }

    #[test]
    fn 核心多头清仓盈利() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(4, 2), 0, 100.0); // core Long 1000@100
        eng.finish(Some(0)); // eod 清仓...但 bar0%sample==0 已采样，finish 用 last_close=100
        // eod 清在 c=100 ⟹ final_nav=100000（无盈利，因价格未变）。验证清仓机制。
        assert!((eng.result().final_nav - INITIAL_CAPITAL).abs() < 1e-3);
        assert!(!eng.result().trades.is_empty(), "eod 清仓产 trade");
    }

    #[test]
    fn 空头强平边界() {
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(4, 2), 0, 100.0); // 核心 Short 1000@100
        eng.step(&TSignalView { ceiling: 2, ..TSignalView::empty() }, 10, 200.0); // 2×basis 强平
        let (_, _, su, _) = eng.snapshot();
        assert_eq!(su, 0.0, "空头被强平");
        assert_eq!(eng.res.n_liquidations_by_ladder[4], 1);
        assert!(eng.global_flat(), "核心强平后 global_flat（n_base→0，可重新入场）");
        assert_eq!(eng.emergent_top, 4, "涌现结构层不因强平归零（由 ceiling 决定）");
    }

    #[test]
    fn 仓位层合法() {
        use crate::trading::types::FIRST_BSP_LADDER;
        assert!(BASE_LADDER >= FIRST_BSP_LADDER);
    }
}
