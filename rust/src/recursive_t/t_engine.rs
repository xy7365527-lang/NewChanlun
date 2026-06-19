//! **T 算子操作层引擎**（纯 BSP 驱动，零方向推断；操作逻辑 + 仓位结构两层自我复制）。
//!
//! ## 核心命题：T 自我复制（aₙ=f(aₙ₋₁) 同时投影到操作层与仓位层）
//! T 算子在信号层是 `aₙ=f(aₙ₋₁)`（第65课）——所有级别共用同一 `f`。本引擎把这个不变性
//! 延伸到两个层，且**只由买卖点（BSP）驱动**，不读任何方向态（无 root/core_polarity/σ-ascend）：
//! 1. **操作逻辑自我复制**：每个级别独立运转完全相同的算子 `apply_bsp`（平反向 → 建本向 →
//!    递归触发次级别反向）。买点/卖点完全对称。
//! 2. **仓位结构自我复制**：1/3 几何塔——次级别的目标规模 = 本级别的 `MOBILE_FRAC`(=1/3)。
//!
//! ## 纯 BSP 驱动 = 零方向推断（本次重写的核心约束）
//! 每个级别 k 收到 BSP 时，方向**直接来自 BSP 类型**，不从任何走势态推断：
//! - `Buy(k)`  → level k 持空头则平空，然后 level k 建多头（`target = Long`）；
//! - `Sell(k)` → level k 持多头则平多，然后 level k 建空头（`target = Short`）；
//! - **同时递归触发次级别反向**：`apply_bsp(k−1, flip(target), budget/3)`——
//!   `Sell(k)→Buy(k−1)→Sell(k−2)→…`、`Buy(k)→Sell(k−1)→Buy(k−2)→…`，方向逐级交替，
//!   规模逐级 ×1/3，一路传播到塔底 `BASE_LADDER`（次级别的次级别…局部依赖 275号）。
//!
//! 没有 BSP 就不动（仅边界强平算子按市场被动触发）。方向不被"推断"——它是 BSP 的字面。
//!
//! ## 第一次入场（现金 → 仓位的唯一边界）
//! 全局空仓（`n_base≈0`）时收到本 bar 最高 ladder 的 BSP：用**全部资金**在该 BSP 的级别
//! 建仓（`base_unit = free/c`），方向 = 该 BSP（buy→Long / sell→Short），随即由 `apply_bsp`
//! 递归把几何塔向下铺到塔底。强平清空（`n_base→0`）后自动允许用剩余资金重新入场。
//!
//! ## 级别升级（涌现）= 不需要任何特殊处理
//! level k+1 涌现（T 迭代多一层）时：level k 的走势类型完成（= level k+1 的一笔）= 走势终
//! 完美 = 该级别 BSP 已 fire（在更高 ladder 出现 buy/sell），`apply_bsp` 自动在新层运转同一
//! 套逻辑。**删除了旧设计的「涌现接管/σ-ascend/emergent_top/核心 relabel 上移」**——级别只
//! 是 ladder 数组多占用一格，无 relabel、无方向锚、无最高层概念。
//!
//! ## 会计模型（formalization-validity-domain 诚实标注）
//! 复用 fugue_v3 会计原语 `add_at`/`reduce_at`（NAV 中性双重会计：开空收现金/平空付现金）。
//! - **NAV 中性**：每次 add/reduce 在同价 c 下价值中性（`prove_nav_neutral`，L0 守恒）。
//! - **会计自洽**：`n_base` 每次操作同步增减，恒等于 Σ|units|（`prove_conservation`）。
//!   注意：本引擎用 add/reduce（与 free 交换），故 Σ|units| **不**是 `sink_chunk` 那种相邻
//!   级别转移的强守恒——`apply_bsp` 的平反向/建本向都改 Σ|units|，由 `n_base` 跟踪保持自洽。
//!   这是用户「平反向建本向」字面（同股数翻转，谱系 M=N）的必然推论，不是 sink 下放。
//! - **翻转保持规模（M=N，谱系 26:34）+ 次级别 1/3**：BSP@k 命中已有仓位层 ⟹ 同股数翻转
//!   （平掉多少建多少，规模守恒，忠实用户「平空→建多」字面）；命中空层（入场/涌现/强平重建）
//!   ⟹ 用 `base_unit` 建仓。递归把次级别**空层**按本级实际规模 ×1/3 建仓（忠实「次级别=上级
//!   1/3」），已有仓位的次级别翻转保持自身规模。`base_unit` 锚定入场 `free/c`。**会计选择**：
//!   「翻转 M=N」与「严格 1/3 几何塔」在多 BSP 动态序列下不能同时严格成立（见结果包边界）。
//! - **杠杆 / MtM 负债**：budget 不受 `free≥0` 约束（add_at(Long) 可使 free<0）——由 1x 逐仓
//!   强平边界约束单层风险，是否爆仓（NAV<0）由 L3 回测裁决，非设计先验解决。
//!
//! ## 守卫语义（formalization-validity-domain）
//! - `prove_nav_neutral`：**L0 强守恒**（add/reduce 同价 c 中性）。每 bar panic。
//! - `prove_conservation`：**会计自洽**（n_base ≡ Σ|units|）。每 bar panic。
//! - `prove_recursive_consistency`：仓位层 ≥ FIRST_BSP_LADDER（本引擎实际 ≥ BASE_LADDER=3）。
//! - `count_chiral_violations`：相邻级别同向观测（非 panic；几何塔手性交替，同向是接力中段）。
//!
//! ## 认识论等级
//! - 两层自我复制结构 / 递归翻转传播 / NAV 中性：**L0**；BSP fire 时机：**L2**；回测 alpha：**L3**。

use crate::trading::types::{Polarity, LADDER_MOVE, MAX_LADDER};

use crate::fugue_v3::accounting::{
    accrue_held_bars, active_voice_count, add_at, exposure, flip, nav, reduce_at,
};
use crate::fugue_v3::cycle::{liquidate_long, liquidate_short};
use crate::fugue_v3::layer::{FugueResult, Layer};
use crate::fugue_v3::prove::{
    count_chiral_violations, prove_conservation, prove_nav_neutral, prove_recursive_consistency,
};
use crate::fugue_v3::{EQUITY_SAMPLE_BARS, INITIAL_CAPITAL, MOBILE_FRAC, SUB_LIQ_FACTOR};

/// T level 0 在 ladder 空间的基准（= 走势级，move(L1)）。T level k ↦ ladder k + BASE_LADDER。
/// 也是**最低操作级别 / 塔底**：`apply_bsp` 在 ladder < BASE_LADDER 终止（无 T 次级别可下放）。
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

/// T 操作层引擎（纯 BSP 驱动 1/3 几何塔，无 root/core_polarity/σ-ascend/涌现接管/方向推断）。
pub struct TPositionEngine {
    res: FugueResult,
    /// 每级别独立净仓位（ladder 索引；全局仓位 = Σ layers，无预设 core）。
    layers: Vec<Layer>,
    /// 统一现金池（NAV 中性双重会计的现金腿；可为负 = 杠杆/MtM 负债）。
    free: f64,
    /// 当前总持仓基数 Σ|units|（会计自洽锚；每次 add/reduce/强平同步增减）。
    n_base: f64,
    /// 入场满仓规模基准 = 入场时 free/c（几何塔顶；每个 BSP 的 budget 锚）。重新入场时刷新。
    base_unit: f64,
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
            base_unit: 0.0,
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

    /// **核心算子 `apply_bsp`（纯 BSP 驱动，递归自我复制）**：把 level k 设为 `target` 方向，
    /// 并递归触发次级别 k−1 的反向操作（规模 ×1/3）。
    ///
    /// 规模语义（M=N 翻转，谱系 26:34 同股数翻转定理）：
    /// - **翻转**（level k 持反向）：平掉旧仓 + 同股数建本向（`actual = 旧 units`，规模守恒，
    ///   忠实用户「平空/平多 → 然后建本向」字面——翻转不放大不缩小）；
    /// - **同向**（level k 已 target）：保持（`actual = 当前 units`）；
    /// - **空层**（入场塔顶 / 涌现新层 / 强平后重建）：用 `fallback` 建仓（`actual = fallback`）。
    ///
    /// 递归（次级别 = 本级 ×1/3）：`apply_bsp(k−1, flip(target), actual·MOBILE_FRAC)`——次级别
    /// 若为空层则按本级实际规模 ×1/3 建仓（几何塔），若已有仓位则按 M=N 翻转保持自身规模。
    ///
    /// 守恒：每步 add/reduce 同步更新 `n_base`（≡ Σ|units|），NAV 中性（同价 c）。
    fn apply_bsp(&mut self, k: usize, target: Polarity, fallback: f64, bar: i64, c: f64) {
        if k < BASE_LADDER || c <= 0.0 {
            return;
        }

        let cur = self.layers[k];
        let actual: f64 = if cur.units > 1e-12 && cur.direction != target {
            // 翻转（平反向 + 同股数建本向，M=N 规模守恒）
            reduce_at(&mut self.layers, k, cur.units, &mut self.free, c, bar, &mut self.res, "flip_close");
            self.n_base -= cur.units;
            self.res.n_cycle_closes_by_ladder[k] += 1;
            add_at(&mut self.layers, k, cur.units, target, &mut self.free, c, bar, &mut self.res);
            self.n_base += cur.units;
            cur.units
        } else if cur.units > 1e-12 {
            // 已同向，保持
            cur.units
        } else if fallback > 1e-12 && fallback.is_finite() {
            // 空层：用 fallback 建仓（入场塔顶 / 涌现新层 / 重建）
            add_at(&mut self.layers, k, fallback, target, &mut self.free, c, bar, &mut self.res);
            self.n_base += fallback;
            fallback
        } else {
            0.0
        };

        // ── 递归触发次级别反向（规模 = 本级实际 ×1/3，方向 flip）──
        if k > BASE_LADDER && actual > 1e-12 {
            self.res.n_cycle_opens_by_ladder[k] += 1;
            self.res.cross_level_closures += 1;
            self.apply_bsp(k - 1, flip(target), actual * MOBILE_FRAC, bar, c);
        }
    }

    /// **入场**（现金→仓位）：全局空仓时在本 bar 最高 ladder 的 BSP 级别用全部资金建仓，
    /// 方向由该 BSP 字面决定（buy→Long / sell→Short），随即 `apply_bsp` 递归铺几何塔。
    fn try_enter(&mut self, view: &TSignalView, bar: i64, c: f64) {
        if self.free <= 0.0 || c <= 0.0 {
            return;
        }
        // 最高有信号 ladder 入场（同 bar 多 BSP 取塔顶；apply_bsp 递归覆盖低层）。
        for k in (0..MAX_LADDER).rev() {
            let target = if view.buy[k] {
                Some(Polarity::Long)
            } else if view.sell[k] {
                Some(Polarity::Short)
            } else {
                None
            };
            if let Some(t) = target {
                self.base_unit = self.free / c;
                self.res.n_entries_by_ladder[k] += 1;
                self.apply_bsp(k, t, self.base_unit, bar, c);
                return;
            }
        }
    }

    /// 整仓清到现金（eod / 强平后收尾）。
    fn clear_to_cash(&mut self, bar: i64, c: f64, reason: &'static str) {
        for k in 0..self.layers.len() {
            if self.layers[k].units > 1e-12 {
                let u = self.layers[k].units;
                reduce_at(&mut self.layers, k, u, &mut self.free, c, bar, &mut self.res, reason);
            }
        }
        self.n_base = 0.0;
    }

    /// 单 bar 操作步进：A 边界强平 → B 纯 BSP 驱动（入场 / 逐级别 apply_bsp）→ C 守卫 → D 观测。
    pub fn step(&mut self, view: &TSignalView, bar: i64, price: f64) {
        let c = price;
        self.last_close = c;
        let nav_pre = nav(&self.layers, self.free, c);

        // ── A. 边界算子：1x 逐仓强平（多空对称；无核心豁免，市场被动触发）──
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

        // ── B. 纯 BSP 驱动（无方向推断；无 BSP 不动）──
        if self.global_flat() {
            // 入场（含强平清空后重新入场）：第一个 BSP 全部资金在该级别建仓。
            self.try_enter(view, bar, c);
        } else {
            // 已持仓：每个有 BSP 的级别独立运转 apply_bsp（ladder 降序，同 bar 确定性顺序）。
            for k in (0..MAX_LADDER).rev() {
                if view.buy[k] {
                    self.apply_bsp(k, Polarity::Long, self.base_unit, bar, c);
                }
                if view.sell[k] {
                    self.apply_bsp(k, Polarity::Short, self.base_unit, bar, c);
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

    /// 在 ladder k 放一个买点视图。
    fn buy_view(k: usize) -> TSignalView {
        let mut v = TSignalView::empty();
        v.buy[k] = true;
        v
    }

    /// 在 ladder k 放一个卖点视图。
    fn sell_view(k: usize) -> TSignalView {
        let mut v = TSignalView::empty();
        v.sell[k] = true;
        v
    }

    #[test]
    fn 入场买点全压最高信号层做多() {
        let mut eng = TPositionEngine::new();
        // 买点@ladder3（塔底）→ 全压做多，无次级别可下放（单层塔）。
        eng.step(&buy_view(3), 0, 100.0);
        let (navv, lu, su, _) = eng.snapshot();
        assert!((lu - 1000.0).abs() < 1e-6, "满仓 1000 units，得 {lu}");
        assert_eq!(su, 0.0);
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "入场 NAV 中性");
        assert!((eng.layers[3].units - 1000.0).abs() < 1e-6);
        assert_eq!(eng.layers[3].direction, Polarity::Long);
    }

    #[test]
    fn 入场递归铺几何塔1_3递减反向() {
        let mut eng = TPositionEngine::new();
        // 买点@ladder5 → 入场 Long@5=1000，递归 Sell@4=333 Short，Buy@3=111 Long。
        eng.step(&buy_view(5), 0, 100.0);
        assert!((eng.layers[5].units - 1000.0).abs() < 1e-6, "塔顶 Long 1000");
        assert_eq!(eng.layers[5].direction, Polarity::Long);
        assert!((eng.layers[4].units - 1000.0 / 3.0).abs() < 1e-3, "次级别 1/3 Short");
        assert_eq!(eng.layers[4].direction, Polarity::Short);
        assert!((eng.layers[3].units - 1000.0 / 9.0).abs() < 1e-3, "次次级别 1/9 Long");
        assert_eq!(eng.layers[3].direction, Polarity::Long);
        assert_eq!(eng.layers[2].units, 0.0, "塔底 ladder2 < BASE_LADDER 不占用");
        // NAV 中性（全部同价 100）。
        let (navv, _, _, _) = eng.snapshot();
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "递归铺塔 NAV 中性");
    }

    #[test]
    fn 反向bsp翻转本级别同时翻转次级别() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // Long@5, Short@4, Long@3
        // 卖点@ladder5 → level5 平多建空（翻转），递归 Buy@4（翻多），Sell@3（翻空）。
        eng.step(&sell_view(5), 10, 100.0);
        assert_eq!(eng.layers[5].direction, Polarity::Short, "塔顶翻空");
        assert!((eng.layers[5].units - 1000.0).abs() < 1e-6, "翻转后规模回 base_unit");
        assert_eq!(eng.layers[4].direction, Polarity::Long, "次级别翻多");
        assert_eq!(eng.layers[3].direction, Polarity::Short, "次次级别翻空");
        let (navv, _, _, _) = eng.snapshot();
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "翻转 NAV 中性（同价）");
    }

    #[test]
    fn 翻空降价回补盈利() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(3), 0, 100.0); // Long@3 1000（单层塔）
        // 卖点@3 涨价翻空：100→110 平多盈利 +10000，开空@110。
        eng.step(&sell_view(3), 10, 110.0);
        assert_eq!(eng.layers[3].direction, Polarity::Short);
        // 平多盈利已实现。
        let realized: f64 = eng.result().mobile_realized_pnl_by_ladder.iter().sum();
        assert!(realized > 0.0, "110>100 平多盈利，得 {realized}");
        let (navv, _, _, _) = eng.snapshot();
        assert!((navv - 110_000.0).abs() < 1.0, "100→110 多头浮盈兑现，NAV≈110000，得 {navv}");
    }

    #[test]
    fn 入场做空对称() {
        let mut eng = TPositionEngine::new();
        // 卖点@ladder4 → 入场 Short@4=1000，递归 Buy@3=333 Long。
        eng.step(&sell_view(4), 0, 100.0);
        let (_, lu, su, _) = eng.snapshot();
        assert!((su - 1000.0).abs() < 1e-6, "塔顶 Short 1000");
        assert!((lu - 1000.0 / 3.0).abs() < 1e-3, "次级别 1/3 Long（flip 手性）");
        assert_eq!(eng.layers[4].direction, Polarity::Short);
        assert_eq!(eng.layers[3].direction, Polarity::Long);
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
        eng.step(&buy_view(3), 0, 100.0); // Long@3 1000@100，free→0，NAV=100000
        // 价格腰斩 → c≤basis/2 强平多头，回笼半仓现金（free=50000，NAV=50000）。
        eng.step(&TSignalView::empty(), 10, 50.0);
        assert!(eng.global_flat(), "多头强平后 global_flat（n_base→0）");
        assert_eq!(eng.res.n_liquidations_by_ladder[3], 1);
        // 强平后仍有 free → 买点用剩余资金重新入场做多。
        let nav_before = eng.snapshot().0;
        eng.step(&buy_view(3), 20, 50.0);
        assert_eq!(eng.layers[3].direction, Polarity::Long, "重新入场做多");
        assert!(eng.layers[3].units > 1e-6, "重新入场建仓");
        let (navv, _, _, _) = eng.snapshot();
        assert!((navv - nav_before).abs() < 1.0, "重新入场 NAV 中性（同价 50）");
    }

    #[test]
    fn 空头价格翻倍爆仓nav归零() {
        // 诚实记录边界：1x 逐仓做空，价格涨到 2×basis ⟹ 保证金亏光，NAV→0，无资金重新入场。
        // 这是做空的本质风险（最大亏损 100%），非 bug——是 1x 逐仓 + 单边上扬的正确会计语义。
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(3), 0, 100.0); // Short@3 1000@100，free=200000，NAV=100000
        eng.step(&TSignalView::empty(), 10, 200.0); // c≥2×basis 强平空头
        assert_eq!(eng.res.n_liquidations_by_ladder[3], 1);
        let (navv, _, _, _) = eng.snapshot();
        assert!(navv.abs() < 1.0, "空头翻倍爆仓 NAV→0，得 {navv}");
        assert!((eng.free).abs() < 1.0, "free 归零，无资金重新入场");
        // 后续买点无法入场（free≤0 守卫拒绝）。
        eng.step(&buy_view(3), 20, 200.0);
        assert_eq!(eng.layers[3].units, 0.0, "free=0 无法重新入场");
    }

    #[test]
    fn 核心多头清仓盈利() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(3), 0, 100.0); // Long 1000@100
        eng.finish(Some(0)); // eod 清在 last_close=100，无价格变动 → final_nav≈INITIAL。
        assert!((eng.result().final_nav - INITIAL_CAPITAL).abs() < 1e-3);
        assert!(!eng.result().trades.is_empty(), "eod 清仓产 trade");
    }

    #[test]
    fn 几何塔多级别守恒nav中性() {
        let mut eng = TPositionEngine::new();
        // 高层入场建 6 层塔，逐 bar 不同价检验每 bar NAV 中性（守卫每 bar panic 检查）。
        eng.step(&buy_view(8), 0, 100.0);
        eng.step(&sell_view(6), 1, 101.0);
        eng.step(&buy_view(5), 2, 99.0);
        eng.step(&sell_view(8), 3, 102.0);
        eng.step(&buy_view(7), 4, 98.0);
        eng.finish(Some(4));
        // 零 panic 到达此处 ⟹ 全程 NAV 中性 + 会计自洽守恒成立。
        assert!(eng.result().final_nav.is_finite(), "final_nav 有限");
    }

    #[test]
    fn 仓位层合法() {
        use crate::trading::types::FIRST_BSP_LADDER;
        assert!(BASE_LADDER >= FIRST_BSP_LADDER);
    }
}
