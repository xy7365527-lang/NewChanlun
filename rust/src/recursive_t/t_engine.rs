//! **T 算子操作层引擎**（纯 BSP 驱动 + ε 对称 sink/recover 几何塔；递归嵌套多重赋格）。
//!
//! ## 核心命题：T 自我复制（aₙ=f(aₙ₋₁) 同时投影到操作层与仓位层）
//! T 算子在信号层是 `aₙ=f(aₙ₋₁)`（第65课）——所有级别共用同一 `f`。本引擎把这个不变性
//! 延伸到两个层，且**只由买卖点（BSP）驱动**，不读任何方向态（无 root/core_polarity/σ-ascend）：
//! 1. **操作逻辑自我复制**：每个级别独立运转完全相同的 sink/recover（多重赋格）。
//! 2. **仓位结构自我复制**：1/3 几何塔——sink 每次取本级 `MOBILE_FRAC`(=1/3) 下放次级别。
//!
//! ## ε 对称 sink/recover（本次修正核心：方向由「当前持仓」决定，不写死买卖角色）
//! 每个级别 k 的操作由 **level k 当前持仓方向 d_k** 与 **BSP 方向** 的关系决定：
//! - **反向 BSP（BSP 方向 = flip(d_k)）→ sink**：从 level k 减 1/3 units 下放次级别 k−1，
//!   方向 flip(d_k)（穿手性缝）。
//!   - 持多头 + 卖点 → 减多头 1/3 下放 k−1 **做空**（用户「持多头卖点减仓做空」）；
//!   - 持空头 + 买点 → 减空头 1/3 下放 k−1 **做多**（ε 对称：用户「持空头买点减仓做多」）。
//! - **同向 BSP（BSP 方向 = d_k）→ recover**：把次级别 k−1 短差**全平**，units 归还 level k。
//!   - 持多头 + 买点 → k−1 空头全平归还（用户「买点平空归还上级」）；
//!   - 持空头 + 卖点 → k−1 多头全平归还（ε 对称：用户「次级别卖点平多归还」）。
//!
//! ε∈{±1} 对称：做多/做空两场景买卖点角色互换，由 d_k 自动选取。没有 BSP 不动（仅边界强平）。
//!
//! ## 递归嵌套多重赋格（每级别独立运转，多级别同时持仓，275号局部依赖）
//! `step` 逐级别（ladder 降序）扫描：每个有持仓的级别 k 独立按自己的 d_k 运转 ε 对称 sink/recover。
//! 一个卖点@k 只下放一层（k→k−1）；次级别 k−1 收到自己的 BSP 时再独立 sink/recover（k−1→k−2）。
//! 多个级别同时持有（核心 + 各级短差 = 多重赋格），全局仓位 = Σ_k layers[k] 涌现，非预设 core。
//!
//! ## 第一次入场（现金 → 仓位的唯一边界）
//! 全局空仓（`n_base≈0`）时收到本 bar 最高 ladder 的 BSP：用**全部资金**在该级别建仓（核心），
//! 方向 = 该 BSP（buy→Long / sell→Short，字面非推断）。塔由后续反向 BSP 的 sink 逐步向下铺。
//! 强平清空（`n_base→0`）后自动允许用剩余资金重新入场（重入层 = 当时最高有信号 ladder）。
//!
//! ## 级别升级（T 新增一层）= 不需要额外操作（删 σ-ascend / 涌现接管 / 核心 relabel）
//! T 新增一层（level k+1 涌现）时：level k 的走势类型完成（= level k+1 的一笔）= 走势终完美 =
//! **level k 的 BSP 已 fire**，已在 level k 触发过 sink/recover 处理了 level k 仓位。故 level k+1
//! 涌现不需特殊处理——它只是 ladder 数组多占用一格 LevelState。新层若有 BSP 但**当前空仓**
//! （前层已处理过），则跳过（无持仓 ⟹ 无 d_k ⟹ 无 sink/recover），等它通过 sink 被上级填充或
//! flat 后重新入场。**无核心 relabel 上移**（旧 emergent_top/takeover 全删）。
//!
//! ## 会计模型（formalization-validity-domain 诚实标注）
//! 复用 fugue_v3 会计原语。sink/recover 是**相邻级别转移**（Σ|units| 强守恒，T48 Casimir）：
//! - **sink**（`cycle::sink_chunk`）：reduce_at(k) + add_at(k−1)，m=1/3·u_k 在 k↔k−1 转移，
//!   Σ|units| 不变（n_base 不变），含 σ-quota/cross-level 守卫。
//! - **recover**（`recover_full`，本引擎全平非 1/3）：reduce_at(k−1 全部) + add_at(k)，转移 u_{k−1}，
//!   Σ|units| 不变。
//! - **NAV 中性**：每个 reduce/add 同价 c 价值中性（`prove_nav_neutral`，L0）。
//! - **n_base（守恒锚）**：仅入场/强平/eod 改（与 Σ|units| 一致）；sink/recover 不改（转移守恒）。
//! - **杠杆 / MtM 负债**：1x 逐仓强平边界约束单层风险，爆仓（NAV<0）由 L3 回测裁决。
//!
//! ## 守卫语义（formalization-validity-domain）
//! - `prove_nav_neutral`：L0 强守恒（同价 c 中性）；`prove_conservation`：Σ|units|=n_base（T48）。
//! - `prove_recursive_consistency`：仓位层 ≥ FIRST_BSP_LADDER（sink 下放至 FIRST_BSP_LADDER=2）。
//! - `count_chiral_violations`：相邻级别同向观测（非 panic；几何塔手性交替，同向是接力中段）。
//!
//! ## 认识论等级
//! - sink/recover ε 对称结构 / Σ 守恒 / NAV 中性：**L0**；BSP fire 时机：**L2**；回测 alpha：**L3**。

use crate::trading::types::{Polarity, FIRST_BSP_LADDER, LADDER_MOVE, MAX_LADDER};

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
pub const BASE_LADDER: usize = LADDER_MOVE;

/// 本 bar 信号视图（从 T 全塔 BSP diff 构造，stream 层填充）。
///
/// 索引空间 = **ladder**（= T level + BASE_LADDER）。纯 BSP 驱动：不区分 type1/2/3（都是
/// 「该级别买/卖点」），无 ceiling/方向态——级别涌现信息已隐含在「哪个 ladder 有 BSP」中。
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

/// T 操作层引擎（纯 BSP 驱动 + ε 对称 sink/recover 几何塔；无 root/core_polarity/σ-ascend/涌现接管）。
pub struct TPositionEngine {
    res: FugueResult,
    /// 每级别独立净仓位（ladder 索引；全局仓位 = Σ layers，多重赋格涌现）。
    layers: Vec<Layer>,
    /// 统一现金池（NAV 中性双重会计的现金腿）。
    free: f64,
    /// 当前总持仓基数 Σ|units|（会计自洽锚；仅入场/强平/eod 改，sink/recover 守恒不改）。
    n_base: f64,
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
            last_close: f64::NAN,
            max_concurrent_seen: 0,
        }
    }

    /// 全局是否空仓（n_base≈0 ⟹ 资金全在 free；首次/强平清空后为真 ⟹ 下个 BSP 重新入场）。
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

    /// **入场**（现金→核心仓）：全局空仓时在本 bar 最高 ladder 的 BSP 级别用全部资金建仓，
    /// 方向由该 BSP 字面决定（buy→Long / sell→Short）。塔由后续反向 BSP 的 sink 逐步铺。
    fn try_enter(&mut self, view: &TSignalView, bar: i64, c: f64) {
        if self.free <= 0.0 || c <= 0.0 {
            return;
        }
        for k in (0..MAX_LADDER).rev() {
            let target = if view.buy[k] {
                Some(Polarity::Long)
            } else if view.sell[k] {
                Some(Polarity::Short)
            } else {
                None
            };
            if let Some(t) = target {
                let m = self.free / c;
                if m > 1e-12 && m.is_finite() {
                    add_at(&mut self.layers, k, m, t, &mut self.free, c, bar, &mut self.res);
                    self.n_base += m;
                    self.res.n_entries_by_ladder[k] += 1;
                }
                return;
            }
        }
    }

    /// **recover（同向 BSP）**：把次级别 k−1 的短差**全平**，units 归还 level k（ε 对称）。
    ///
    /// 归还方向 = flip(d_sub) = d_k（核心侧）。Σ 守恒（reduce + add 同量，n_base 不变）。
    /// 父层相容前提（add_at 层内单一方向不变量）：k 已占用须同向 d_k，否则操作不适用，跳过。
    fn recover_full(&mut self, k: usize, bar: i64, c: f64) {
        let sub = k - 1;
        let u_sub = self.layers[sub].units;
        if u_sub <= 1e-12 {
            return;
        }
        let d_sub = self.layers[sub].direction;
        let want = flip(d_sub); // 归还 level k 的方向（翻回核心侧 d_k）
        if self.layers[k].units > 1e-12 && self.layers[k].direction != want {
            return;
        }
        prove_cross_level_closure(k, sub);
        reduce_at(&mut self.layers, sub, u_sub, &mut self.free, c, bar, &mut self.res, "recover");
        add_at(&mut self.layers, k, u_sub, want, &mut self.free, c, bar, &mut self.res);
        self.res.n_cycle_closes_by_ladder[k] += 1;
        self.res.cross_level_closures += 1;
    }

    /// 整仓清到现金（eod / 收尾）。
    fn clear_to_cash(&mut self, bar: i64, c: f64, reason: &'static str) {
        for k in 0..self.layers.len() {
            if self.layers[k].units > 1e-12 {
                let u = self.layers[k].units;
                reduce_at(&mut self.layers, k, u, &mut self.free, c, bar, &mut self.res, reason);
            }
        }
        self.n_base = 0.0;
    }

    /// 单 bar 操作步进：A 边界强平 → B 纯 BSP 驱动（入场 / 逐级别 ε 对称 sink·recover）→ C 守卫 → D 观测。
    pub fn step(&mut self, view: &TSignalView, bar: i64, price: f64) {
        let c = price;
        self.last_close = c;
        let nav_pre = nav(&self.layers, self.free, c);

        // ── A. 边界算子：1x 逐仓强平（多空对称；市场被动触发）──
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
            }
        }

        // ── B. 纯 BSP 驱动（ε 对称 sink/recover；方向由当前持仓 d_k 决定；无 BSP 不动）──
        if self.global_flat() {
            // 入场（含强平清空后重新入场）：第一个 BSP 全部资金在该级别建核心。
            self.try_enter(view, bar, c);
        } else {
            // 已持仓：每个有持仓的级别 k 独立运转 ε 对称 sink/recover（多重赋格，ladder 降序）。
            for k in (0..MAX_LADDER).rev() {
                // sink 下放 k→k−1，要求 k−1 ≥ FIRST_BSP_LADDER ⟹ k ≥ BASE_LADDER（=PENDING_LO）。
                if k < BASE_LADDER {
                    continue;
                }
                // 无持仓 ⟹ 无方向 d_k ⟹ 跳过（级别升级：前层 BSP 已 fire 处理过本层）。
                if self.layers[k].units <= 1e-12 {
                    continue;
                }
                let d_k = self.layers[k].direction;
                let bsp_dir = if view.buy[k] {
                    Some(Polarity::Long)
                } else if view.sell[k] {
                    Some(Polarity::Short)
                } else {
                    None
                };
                if let Some(bd) = bsp_dir {
                    if bd == flip(d_k) {
                        // 反向 BSP → sink（减本级 1/3 下放次级别，方向 flip(d_k)，ε 对称）。
                        sink_chunk(&mut self.layers, k, &mut self.free, c, bar, &mut self.res);
                    } else {
                        // 同向 BSP → recover（次级别 k−1 全平归还本级，ε 对称）。
                        self.recover_full(k, bar, c);
                    }
                }
            }
        }

        // ── C. 必然性运行时证明（每 bar；violation = panic = 验收）──
        prove_recursive_consistency(&self.layers, bar);
        let chiral = count_chiral_violations(&self.layers) as u64;
        self.res.max_chiral_same_dir = self.res.max_chiral_same_dir.max(chiral);
        let nav_post = nav(&self.layers, self.free, c);
        prove_nav_neutral(nav_pre, nav_post, bar);
        prove_conservation(&self.layers, self.n_base, bar);

        // ── D. 观测 ──
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

    #[test]
    fn 入场买点全压最高信号层做多() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let (navv, lu, su, _) = eng.snapshot();
        assert!((lu - 1000.0).abs() < 1e-6, "满仓 1000 units，得 {lu}");
        assert_eq!(su, 0.0);
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "入场 NAV 中性");
        assert_eq!(eng.layers[5].direction, Polarity::Long);
        assert!((eng.n_base - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn 持多卖点sink三分之一下放次级别做空() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // 核心 Long@5 = 1000
        let base = eng.n_base;
        // 持多头 + 卖点@5（反向）→ sink 1/3 下放 ladder4 做空。
        eng.step(&sell_view(5), 10, 110.0);
        let (_, lu, su, _) = eng.snapshot();
        assert!((lu - base * 2.0 / 3.0).abs() < 1e-6, "核心剩 2/3 Long，得 {lu}");
        assert!((su - base / 3.0).abs() < 1e-6, "次级别 1/3 Short，得 {su}");
        assert_eq!(eng.layers[4].direction, Polarity::Short);
        assert!((eng.n_base - base).abs() < 1e-9, "sink Σ 守恒（n_base 不变）");
    }

    #[test]
    fn 持多买点recover全平次级别归还() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // 核心 Long@5 1000
        let base = eng.n_base;
        eng.step(&sell_view(5), 10, 110.0); // sink → ladder4 Short 333, 核心 667
        // 持多头 + 买点@5（同向）→ recover 全平 ladder4 归还核心。
        eng.step(&buy_view(5), 20, 105.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(su, 0.0, "次级别空头全平");
        assert!((lu - base).abs() < 1e-6, "核心恢复全仓 1000，得 {lu}");
        // 空头 110→105 降价回补盈利。
        let mob: f64 = eng.result().mobile_realized_pnl_by_ladder.iter().sum();
        assert!(mob > 0.0, "次级别空头降价回补盈利，得 {mob}");
    }

    #[test]
    fn ε对称_持空买点sink下放做多() {
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0); // 核心 Short@5 1000
        let base = eng.n_base;
        // 持空头 + 买点@5（反向）→ sink 1/3 下放 ladder4 做多（ε 对称！）。
        eng.step(&buy_view(5), 10, 90.0);
        let (_, lu, su, _) = eng.snapshot();
        assert!((su - base * 2.0 / 3.0).abs() < 1e-6, "核心剩 2/3 Short，得 {su}");
        assert!((lu - base / 3.0).abs() < 1e-6, "次级别 1/3 Long（ε 对称做多），得 {lu}");
        assert_eq!(eng.layers[4].direction, Polarity::Long);
        assert!((eng.n_base - base).abs() < 1e-9, "sink Σ 守恒");
    }

    #[test]
    fn ε对称_持空卖点recover归还() {
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0); // 核心 Short@5 1000
        let base = eng.n_base;
        eng.step(&buy_view(5), 10, 90.0); // sink → ladder4 Long 333, 核心 Short 667
        // 持空头 + 卖点@5（同向）→ recover 全平 ladder4 归还核心（ε 对称！）。
        eng.step(&sell_view(5), 20, 95.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "次级别多头全平");
        assert!((su - base).abs() < 1e-6, "核心恢复全仓 Short 1000，得 {su}");
        assert_eq!(eng.layers[5].direction, Polarity::Short);
    }

    #[test]
    fn 几何塔多次sink递减() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // 核心 1000
        eng.step(&sell_view(5), 10, 110.0); // sink: 核心 667, ladder4 333
        eng.step(&sell_view(5), 20, 112.0); // sink: 核心 667*2/3=444, ladder4 +667/3=222 → 555
        let (_, lu, su, _) = eng.snapshot();
        assert!((lu - 1000.0 * 4.0 / 9.0).abs() < 1e-3, "核心 4/9，得 {lu}");
        assert!((su - 1000.0 * 5.0 / 9.0).abs() < 1e-3, "次级别 5/9，得 {su}");
        assert!((eng.n_base - 1000.0).abs() < 1e-6, "几何塔 Σ 守恒");
    }

    #[test]
    fn 多重赋格三级别同时持仓() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // 核心 Long@5
        eng.step(&sell_view(5), 10, 110.0); // sink → ladder4 Short
        // 持空头@4 + 买点@4（反向）→ sink 下放 ladder3 做多（次级别的次级别）。
        eng.step(&buy_view(4), 20, 105.0);
        // 三级别同时持仓：ladder5 Long, ladder4 Short, ladder3 Long。
        assert!(eng.layers[5].units > 1e-6 && eng.layers[5].direction == Polarity::Long);
        assert!(eng.layers[4].units > 1e-6 && eng.layers[4].direction == Polarity::Short);
        assert!(eng.layers[3].units > 1e-6 && eng.layers[3].direction == Polarity::Long);
        assert!((eng.n_base - 1000.0).abs() < 1e-6, "多重赋格 Σ 守恒");
    }

    #[test]
    fn 级别升级高层空仓bsp跳过() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // 核心 Long@5
        let n_before = eng.n_trades();
        // 更高层 ladder7 涌现 BSP，但 ladder7 空仓（前层已处理）→ 跳过，无操作。
        eng.step(&sell_view(7), 10, 100.0);
        assert_eq!(eng.layers[7].units, 0.0, "高层空仓 BSP 不建仓（级别升级不 relabel）");
        assert_eq!(eng.n_trades(), n_before, "高层空仓 BSP 跳过，无 trade");
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
    fn 强平后重新入场() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // Long@5 1000@100
        eng.step(&TSignalView::empty(), 10, 50.0); // 腰斩强平多头
        assert!(eng.global_flat(), "多头强平后 global_flat");
        assert_eq!(eng.res.n_liquidations_by_ladder[5], 1);
        // 强平后买点重新入场。
        eng.step(&buy_view(5), 20, 50.0);
        assert_eq!(eng.layers[5].direction, Polarity::Long, "重新入场做多");
        assert!(eng.layers[5].units > 1e-6);
    }

    #[test]
    fn 空头价格翻倍爆仓nav归零() {
        // 1x 逐仓做空，价格涨到 2×basis ⟹ NAV→0（做空本质风险，非 bug）。
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0); // Short@5 1000@100
        eng.step(&TSignalView::empty(), 10, 200.0); // c≥2×basis 强平
        assert_eq!(eng.res.n_liquidations_by_ladder[5], 1);
        let (navv, _, _, _) = eng.snapshot();
        assert!(navv.abs() < 1.0, "空头翻倍爆仓 NAV→0，得 {navv}");
    }

    #[test]
    fn 仓位层合法() {
        assert!(BASE_LADDER > FIRST_BSP_LADDER, "核心层 sink 下放至 FIRST_BSP_LADDER 仍合法");
    }
}
