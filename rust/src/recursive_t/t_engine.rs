//! **T 算子操作层引擎**（同级别/次级别 BSP 分层语义 + ε 对称 sink/recover 几何塔；递归多重赋格）。
//!
//! ## 核心修复（本次重写）：区分同级别 BSP 与次级别 BSP 的操作含义
//! 旧版 bug（诊断报告 `analysis/t_short_leg_diagnosis_report.md` §4）：任意 ladder k 的 BSP 都
//! 触发该级别的 sink/recover——把「级别 k 自己的卖点」当作「对 k 做 1/3 短差」的触发，使同级别
//! 卖点被误读为减仓信号，引擎退化为无条件双向翻转机（涨段照样建空 ⟹ 趋势标的空头腿巨亏）。
//!
//! 缠论原意：BSP 的**级别 j** 相对**持仓级别 K** 决定操作语义（卖点 = 多头出场时机，**不是**
//! 统一的开空/减仓信号）。设核心持仓级别 = 当前最高持仓层 K（塔从核心向下铺，核心恒为最高层）：
//!
//! ### 同级别 BSP（j == K，含更高级别 j > K）= 主仓位进出场（B1）
//! - 入场时（global_flat）：同级别买点 → 在 K 建核心做多；卖点 → 建核心做空（方向字面）。
//! - 持仓后，**反向**同级别/更高级别 BSP（持多遇卖点 / 持空遇买点）→ **清仓出场到现金**
//!   （级联平全塔，回 global_flat 等重入）。**不翻空**——这是诊断报告根因的概念修复：
//!   卖点让多头走人（回现金），而非建立等量空头被趋势碾压。
//! - 同向同级别 BSP（持多遇买点 / 持空遇卖点）→ no-op（已在该方向，不重复建仓）。
//!
//! ### 次级别 BSP（j == K−1，及塔中每层 m 的 m−1）= 短差 sink/recover（B2）
//! 对塔中每个持仓层 m，看其**次级别 m−1** 的 BSP（ε 对称，方向由 m 当前持仓 d_m 决定）：
//! - **反向次级别 BSP**（d_m=Long + 卖点@m−1 / d_m=Short + 买点@m−1）→ **sink**：减 m 的 1/3
//!   下放 m−1 做短差（穿手性缝，方向 flip(d_m)）。前提：m 持仓（已检查）。
//!   - 持多 + 次级别卖点 → 减多头 1/3 下放**做空**（用户「持多卖点减仓做空」）；
//!   - 持空 + 次级别买点 → 减空头 1/3 下放**做多**（ε 对称：用户「持空买点减仓做多」）。
//! - **同向次级别 BSP**（d_m=Long + 买点@m−1 / d_m=Short + 卖点@m−1）→ **recover**：把 m−1
//!   短差**全平**归还 m。前提：m−1 持 flip(d_m)（确是短差腿，非核心）。
//!   - 持多 + 次级别买点 → 平空头短差归还（用户「买点平空归还上级」）；
//!   - 持空 + 次级别卖点 → 平多头短差归还（ε 对称）。
//!
//! ## 分层职责消歧（275号局部依赖：每层只管自己的直接次级别）
//! 每个 ladder 的 BSP **只被一个层处理**，无重复无歧义：
//! - BSP @ j ≥ K（核心层及更高）→ 核心进出场逻辑（B1）。
//! - BSP @ j < K → 由上级 layer[j+1] 作为「次级别」处理 sink/recover（B2 遍历持仓层各看 m−1）。
//! 短差腿的「出场回补」**等于**上级的 recover（「m−1 持空头同级别买点出场」≡「m 的次级别买点平
//! 短差归还」是同一操作），故短差腿不单独处理同级别出场——避免与上级 recover 重复触发。
//!
//! ## 第一次入场（现金 → 核心仓的唯一边界）
//! 全局空仓（n_base≈0）收到本 bar 最高 ladder 的 BSP：用**全部资金**在该级别建核心，方向 =
//! 该 BSP 字面（buy→Long / sell→Short）。塔由后续次级别反向 BSP 的 sink 逐步向下铺。
//! 清仓 / 强平清空后（n_base→0）自动允许用剩余资金重新入场。
//!
//! ## 级别升级（T 新增一层）= 更高级别 BSP 走 B1
//! T 新增 level K+1 涌现时，K+1 的 BSP 是「比核心更大级别的进出场信号」：反向（持多 + K+1 卖点
//! = 更大级别顶背驰）→ 清仓（比同级别更强的出场理由，缠论大级别优先）；同向 → no-op。
//! 核心不 relabel 上移——它只是被更大级别的反向 BSP 清掉后重新入场。
//!
//! ## 会计模型（formalization-validity-domain 诚实标注）
//! 复用 fugue_v3 会计原语：
//! - **sink**（`cycle::sink_chunk`）：reduce_at(m) + add_at(m−1)，m=1/3·u_m 在 m↔m−1 转移，
//!   Σ|units| 不变（n_base 不变，T48 Casimir），含 σ-quota / cross-level 守卫。
//! - **recover**（`recover_full`，本引擎全平非 1/3）：reduce_at(m−1 全部) + add_at(m)，Σ不变。
//! - **清仓 / 强平**（`clear_to_cash` / `liquidate_*`）：reduce 到现金，**改 n_base**（边界）。
//! - **NAV 中性**：每个 reduce/add 同价 c 价值中性（`prove_nav_neutral`，L0）。
//! - **n_base（守恒锚）**：仅入场/清仓/强平/eod 改；sink/recover 转移守恒不改。
//! - **杠杆 / MtM**：1x 逐仓强平边界约束单层风险，爆仓（NAV<0）由 L3 回测裁决。
//!
//! ## 守卫语义（formalization-validity-domain）
//! - `prove_nav_neutral`：L0 强守恒；`prove_conservation`：Σ|units|=n_base（T48）。
//! - `prove_recursive_consistency`：仓位层 ≥ FIRST_BSP_LADDER（sink 下放至 FIRST_BSP_LADDER=2）。
//! - `count_chiral_violations`：相邻级别同向观测（非 panic；几何塔手性交替）。
//!
//! ## 认识论等级
//! - 同级别/次级别分层语义 / sink/recover ε 对称 / Σ 守恒 / NAV 中性：**L0**；
//! - BSP fire 时机 / 清仓 vs 翻空的方向选择：**L2**；回测 alpha：**L3**。

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

/// T 操作层引擎（同级别/次级别分层 BSP + ε 对称 sink/recover 几何塔；无 root/方向态推断）。
pub struct TPositionEngine {
    res: FugueResult,
    /// 每级别独立净仓位（ladder 索引；全局仓位 = Σ layers，多重赋格涌现）。
    layers: Vec<Layer>,
    /// 统一现金池（NAV 中性双重会计的现金腿）。
    free: f64,
    /// 当前总持仓基数 Σ|units|（会计自洽锚；仅入场/清仓/强平/eod 改，sink/recover 守恒不改）。
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

    /// 全局是否空仓（n_base≈0 ⟹ 资金全在 free；首次/清仓/强平清空后为真 ⟹ 下个 BSP 重新入场）。
    fn global_flat(&self) -> bool {
        self.n_base <= 1e-9
    }

    /// 当前**核心持仓级别**（= 最高占用 ladder；塔从核心向下铺 ⟹ 核心恒为最高占用层）。
    /// 调用前提：非 global_flat（else 分支保证至少一层 units>0）。返回 `None` 仅当全空（不可达）。
    fn core_ladder(&self) -> Option<usize> {
        (0..MAX_LADDER).rev().find(|&k| self.layers[k].units > 1e-12)
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

    /// **入场**（现金→核心仓）：全局空仓时在本 bar 最高 ladder 的 BSP 级别用全部资金建核心，
    /// 方向由该 BSP 字面决定（buy→Long / sell→Short）。塔由后续次级别反向 BSP 的 sink 逐步铺。
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

    /// **核心是否出场**：核心级别 K 及更高级别 j≥K 出现**反向** BSP（持多遇卖点 / 持空遇买点）。
    /// = 缠论「同级别/大级别卖点 = 多头出场时机」（持空镜像：买点 = 空头出场时机）。
    fn core_should_exit(&self, view: &TSignalView, top: usize) -> bool {
        let d_top = self.layers[top].direction;
        (top..MAX_LADDER).any(|j| match d_top {
            Polarity::Long => view.sell[j],  // 持多遇（同级别/更高级别）卖点 → 反向出场
            Polarity::Short => view.buy[j],  // 持空遇买点 → 反向出场（ε 对称）
        })
    }

    /// **recover（同向次级别 BSP）**：把次级别 m−1 的短差**全平**，units 归还 level m（ε 对称）。
    ///
    /// 归还方向 = flip(d_sub) = d_m（核心侧）。Σ 守恒（reduce + add 同量，n_base 不变）。
    /// 前提（add_at 层内单一方向不变量）：m−1 须持 flip(d_m)（确是短差腿）；m 若占用须同向 d_m。
    fn recover_full(&mut self, m: usize, bar: i64, c: f64) {
        let sub = m - 1;
        let u_sub = self.layers[sub].units;
        if u_sub <= 1e-12 {
            return;
        }
        let d_sub = self.layers[sub].direction;
        let want = flip(d_sub); // 归还 level m 的方向（翻回核心侧 d_m）
        if self.layers[m].units > 1e-12 && self.layers[m].direction != want {
            return;
        }
        prove_cross_level_closure(m, sub);
        reduce_at(&mut self.layers, sub, u_sub, &mut self.free, c, bar, &mut self.res, "recover");
        add_at(&mut self.layers, m, u_sub, want, &mut self.free, c, bar, &mut self.res);
        self.res.n_cycle_closes_by_ladder[m] += 1;
        self.res.cross_level_closures += 1;
    }

    /// 整仓清到现金（核心出场级联平全塔 / eod 收尾）。改 n_base（边界算子）。
    fn clear_to_cash(&mut self, bar: i64, c: f64, reason: &'static str) {
        for k in 0..self.layers.len() {
            if self.layers[k].units > 1e-12 {
                let u = self.layers[k].units;
                reduce_at(&mut self.layers, k, u, &mut self.free, c, bar, &mut self.res, reason);
            }
        }
        self.n_base = 0.0;
    }

    /// 单 bar 操作步进：A 边界强平 → B 分层 BSP（入场 / 核心出场 B1 / 次级别短差 B2）→ C 守卫 → D 观测。
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

        // ── B. 分层 BSP 驱动（同级别进出场 + 次级别 ε 对称短差；无 BSP 不动）──
        if self.global_flat() {
            // 入场（含清仓/强平清空后重新入场）：第一个 BSP 全部资金在最高 ladder 建核心。
            self.try_enter(view, bar, c);
        } else if let Some(top) = self.core_ladder() {
            // ── B1. 核心进出场：j≥top 的反向 BSP（持多遇卖点 / 持空遇买点）→ 清仓出场（不翻空）──
            if self.core_should_exit(view, top) {
                self.res.n_core_clears_by_ladder[top] += 1;
                self.clear_to_cash(bar, c, "core_exit");
            } else {
                // ── B2. 次级别短差：每个持仓层 m（ladder 降序）看次级别 m−1 BSP，ε 对称 sink/recover ──
                for m in (BASE_LADDER..MAX_LADDER).rev() {
                    // 无持仓 ⟹ 无方向 d_m ⟹ 跳过（BSP@m−1 仅当 m 持仓才作用，前提检查）。
                    if self.layers[m].units <= 1e-12 {
                        continue;
                    }
                    let d_m = self.layers[m].direction;
                    let sub = m - 1;
                    let sub_bsp = if view.buy[sub] {
                        Some(Polarity::Long)
                    } else if view.sell[sub] {
                        Some(Polarity::Short)
                    } else {
                        None
                    };
                    if let Some(bd) = sub_bsp {
                        if bd == flip(d_m) {
                            // 反向次级别 BSP → sink（减本级 1/3 下放次级别做短差，方向 flip(d_m)，ε 对称）。
                            sink_chunk(&mut self.layers, m, &mut self.free, c, bar, &mut self.res);
                        } else {
                            // 同向次级别 BSP → recover（次级别 m−1 短差全平归还本级，ε 对称）。
                            self.recover_full(m, bar, c);
                        }
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
    fn 入场卖点全压最高信号层做空() {
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0);
        assert!((su - 1000.0).abs() < 1e-6, "满仓 1000 Short，得 {su}");
        assert_eq!(eng.layers[5].direction, Polarity::Short);
    }

    #[test]
    fn 同级别卖点清仓出场不翻空() {
        // 核心 Long@5 + 同级别卖点@5（反向）→ 清仓到现金（不翻空——诊断报告根因修复）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // 核心 Long@5 = 1000
        eng.step(&sell_view(5), 10, 110.0);
        let (navv, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "同级别卖点清多头");
        assert_eq!(su, 0.0, "不翻空——回现金，无空头");
        assert!(eng.global_flat(), "清仓后 global_flat");
        // 多头 100→110 升价卖出盈利 ⟹ NAV > 初始。
        assert!(navv > INITIAL_CAPITAL, "升价清仓盈利，得 {navv}");
        assert_eq!(eng.res.n_core_clears_by_ladder[5], 1);
    }

    #[test]
    fn 同级别买点持多no_op() {
        // 核心 Long@5 + 同级别买点@5（同向）→ no-op（已在该方向，不重复建仓）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let n_before = eng.n_trades();
        eng.step(&buy_view(5), 10, 110.0);
        let (_, lu, su, _) = eng.snapshot();
        assert!((lu - 1000.0).abs() < 1e-6, "仍满仓 Long 1000，得 {lu}");
        assert_eq!(su, 0.0);
        assert_eq!(eng.n_trades(), n_before, "同向同级别 BSP 无 trade");
    }

    #[test]
    fn 更高级别卖点清仓出场() {
        // 核心 Long@5 + 更高级别卖点@7（大级别顶背驰）→ 清仓（缠论大级别优先，比同级别更强出场）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        eng.step(&sell_view(7), 10, 110.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "更高级别卖点清多头");
        assert_eq!(su, 0.0, "不翻空");
        assert!(eng.global_flat(), "清仓后 global_flat");
        assert_eq!(eng.res.n_core_clears_by_ladder[5], 1);
    }

    #[test]
    fn 次级别卖点sink下放做空() {
        // 核心 Long@5 + 次级别卖点@4（反向）→ sink 1/3 下放 ladder4 做空（前提：5 持多头）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // 核心 Long@5 = 1000
        let base = eng.n_base;
        eng.step(&sell_view(4), 10, 110.0);
        let (_, lu, su, _) = eng.snapshot();
        assert!((lu - base * 2.0 / 3.0).abs() < 1e-6, "核心剩 2/3 Long，得 {lu}");
        assert!((su - base / 3.0).abs() < 1e-6, "次级别 1/3 Short，得 {su}");
        assert_eq!(eng.layers[4].direction, Polarity::Short);
        assert!((eng.n_base - base).abs() < 1e-9, "sink Σ 守恒（n_base 不变）");
        // 核心仍在 ladder5（最高持仓层）。
        assert_eq!(eng.core_ladder(), Some(5));
    }

    #[test]
    fn 次级别买点recover回补归还() {
        // 核心 Long@5 → 次级别卖点@4 sink 做空 → 次级别买点@4（同向）→ recover 全平归还核心。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // 核心 Long@5 1000
        let base = eng.n_base;
        eng.step(&sell_view(4), 10, 110.0); // sink → ladder4 Short 333, 核心 667
        eng.step(&buy_view(4), 20, 105.0); // 次级别买点 → recover
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(su, 0.0, "次级别空头全平");
        assert!((lu - base).abs() < 1e-6, "核心恢复全仓 1000，得 {lu}");
        // 空头 110→105 降价回补盈利。
        let mob: f64 = eng.result().mobile_realized_pnl_by_ladder.iter().sum();
        assert!(mob > 0.0, "次级别空头降价回补盈利，得 {mob}");
    }

    #[test]
    fn ε对称_持空次级别买点sink下放做多() {
        // 核心 Short@5 + 次级别买点@4（反向）→ sink 1/3 下放 ladder4 做多（ε 对称，前提：5 持空）。
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0); // 核心 Short@5 1000
        let base = eng.n_base;
        eng.step(&buy_view(4), 10, 90.0);
        let (_, lu, su, _) = eng.snapshot();
        assert!((su - base * 2.0 / 3.0).abs() < 1e-6, "核心剩 2/3 Short，得 {su}");
        assert!((lu - base / 3.0).abs() < 1e-6, "次级别 1/3 Long（ε 对称做多），得 {lu}");
        assert_eq!(eng.layers[4].direction, Polarity::Long);
        assert!((eng.n_base - base).abs() < 1e-9, "sink Σ 守恒");
    }

    #[test]
    fn ε对称_持空次级别卖点recover归还() {
        // 核心 Short@5 → 次级别买点@4 sink 做多 → 次级别卖点@4（同向）→ recover 全平归还（ε 对称）。
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0); // 核心 Short@5 1000
        let base = eng.n_base;
        eng.step(&buy_view(4), 10, 90.0); // sink → ladder4 Long 333, 核心 Short 667
        eng.step(&sell_view(4), 20, 95.0); // 次级别卖点 → recover
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "次级别多头全平");
        assert!((su - base).abs() < 1e-6, "核心恢复全仓 Short 1000，得 {su}");
        assert_eq!(eng.layers[5].direction, Polarity::Short);
    }

    #[test]
    fn ε对称_持空同级别买点清仓出场() {
        // 核心 Short@5 + 同级别买点@5（反向）→ 清仓出场（ε 对称：买点 = 空头出场时机）。
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0); // 核心 Short@5 1000
        eng.step(&buy_view(5), 10, 90.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0);
        assert_eq!(su, 0.0, "同级别买点清空头出场");
        assert!(eng.global_flat(), "清仓后 global_flat");
    }

    #[test]
    fn 几何塔多次sink递减() {
        // 核心 Long@5 + 两次次级别卖点@4 → 两次 sink，核心几何递减 4/9，次级别累积 5/9。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // 核心 1000
        eng.step(&sell_view(4), 10, 110.0); // sink: 核心 667, ladder4 333
        eng.step(&sell_view(4), 20, 112.0); // sink: 核心 667*2/3=444, ladder4 +667/3=222 → 555
        let (_, lu, su, _) = eng.snapshot();
        assert!((lu - 1000.0 * 4.0 / 9.0).abs() < 1e-3, "核心 4/9，得 {lu}");
        assert!((su - 1000.0 * 5.0 / 9.0).abs() < 1e-3, "次级别 5/9，得 {su}");
        assert!((eng.n_base - 1000.0).abs() < 1e-6, "几何塔 Σ 守恒");
    }

    #[test]
    fn 多重赋格三级别同时持仓() {
        // 核心 Long@5 → 次级别卖点@4 sink（4 Short）→ 4 的次级别买点@3 sink（3 Long）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // 核心 Long@5
        eng.step(&sell_view(4), 10, 110.0); // sink → ladder4 Short
        // 持空头@4 + 次级别买点@3（反向 flip(Short)=Long）→ sink 下放 ladder3 做多。
        eng.step(&buy_view(3), 20, 105.0);
        // 三级别同时持仓：ladder5 Long, ladder4 Short, ladder3 Long（手性交替）。
        assert!(eng.layers[5].units > 1e-6 && eng.layers[5].direction == Polarity::Long);
        assert!(eng.layers[4].units > 1e-6 && eng.layers[4].direction == Polarity::Short);
        assert!(eng.layers[3].units > 1e-6 && eng.layers[3].direction == Polarity::Long);
        assert!((eng.n_base - 1000.0).abs() < 1e-6, "多重赋格 Σ 守恒");
        assert_eq!(eng.core_ladder(), Some(5), "核心仍在最高层 5");
    }

    #[test]
    fn 同级别卖点不被误读为次级别sink() {
        // 回归测试（核心 bug 修复）：旧版 sell@5（同级别）触发 sink 减 1/3。
        // 新版 sell@5（同级别反向）触发清仓——验证不再产生 1/3 短差空头。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        eng.step(&sell_view(5), 10, 110.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(su, 0.0, "同级别卖点不产生短差空头（修复：非 sink）");
        assert_eq!(lu, 0.0, "全清而非保留 2/3");
        assert!(eng.global_flat());
    }

    #[test]
    fn 核心出场级联平全塔() {
        // 核心 Long@5 + 次级别卖点@4 建短差空头 → 同级别卖点@5 清仓应级联平掉 ladder4 短差。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // 核心 Long@5
        eng.step(&sell_view(4), 10, 110.0); // sink → ladder4 Short
        assert!(eng.layers[4].units > 1e-6, "短差空头已建");
        eng.step(&sell_view(5), 20, 115.0); // 同级别卖点 → 清仓级联
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0);
        assert_eq!(su, 0.0, "级联平全塔（含 ladder4 短差）");
        assert!(eng.global_flat());
    }

    #[test]
    fn 次级别操作需上级持仓前提() {
        // 空仓时次级别 BSP 不入场（入场只认最高 ladder）；无核心 ⟹ 次级别短差无对象。
        let mut eng = TPositionEngine::new();
        // 仅次级别卖点@4，空仓 → try_enter 选最高有信号 ladder（4）建核心 Short@4，非短差。
        eng.step(&sell_view(4), 0, 100.0);
        assert_eq!(eng.core_ladder(), Some(4), "空仓入场以最高信号 ladder 为核心");
        assert_eq!(eng.layers[4].direction, Polarity::Short);
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
