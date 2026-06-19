//! **T 算子操作层引擎**（完整 6 步周期离散状态机；递归多重赋格）。
//!
//! ## 本次重写（编排者裁决 2026-06-19）：离散 5 状态机 + 固定 6 步周期
//! 旧版是连续 units 几何塔（每层可无限次 sink，4/9→5/9→…，ε 对称 + cross-level 守恒）。
//! 改为**离散 5 状态机 + 固定 6 步周期**：每个级别 k 用同一套状态机（第65课 `aₙ=f(aₙ₋₁)`
//! 的操作层投影，T 自我复制），`Full ⇌ Reduced` 往返，每次减/补**固定 1/3**（Reduced 态
//! 不二次减——离散 5 态对连续几何塔的关键区分）。
//!
//! ## 完整 6 步周期（做多场景；做空 ε 镜像）
//! ```text
//! 1. k 级别买入（同级别买点 → 建核心做多，FullLong@k）
//! 2. k 级别减仓（次级别卖点 → 减 1/3，ReducedLong@k + spawn Short@(k−1)）
//! 3. 次级别做空（减出的 1/3 在 k−1 做空，FullShort@(k−1)）
//! 4. 次级别平空（次级别买点 → 平 Short@(k−1)）
//! 5. k 级别回补（归还 1/3 到 k，FullLong@k）
//! 6. k 级别清仓（同级别卖点 = 走势终完美 → Empty@k）
//! ```
//! 步骤 2–5 可循环多次（多个次级别回调 = FullLong⇌ReducedLong 往返），直到步骤 6 清仓。
//!
//! ## 状态机转移表（做多；做空完全 ε 镜像）
//! ```text
//! Empty         --[同级别Buy]-->  FullLong                       （入场）
//! FullLong      --[次级别Sell]--> ReducedLong + spawn Short@(k−1) （减仓做空）
//! ReducedLong   --[次级别Buy]-->  FullLong（回补，平子 Short@k−1）（回补恢复满仓）
//! FullLong      --[同级别Sell]--> Empty（清仓）                    （清仓）
//! ReducedLong   --[同级别Sell]--> Empty（清仓，平子 Short@k−1）    （清仓级联）
//! ```
//! 关键约束（用户裁决）：
//! - 次级别 Sell **只在 FullLong** 触发（持满才能减仓做空；Reduced 已减，不二次减）。
//! - 次级别 Buy **只在 ReducedLong** 触发（有子空头才回补；Full 无子腿，忽略）。
//! - 同级别 Sell 在 FullLong 或 ReducedLong **都触发清仓**（清仓优先于次级别短差）。
//! - Empty 状态下次级别 BSP **全部忽略**。
//!
//! ## 分层职责消歧（275号局部依赖：链顶判定）
//! 信号 `buy@(k−1)` 既是 level k 的「次级别买点」又是 level k−1 的「同级别买点」——同一物理
//! 操作（平 k−1 的 Short）。消歧：**level k−1 的同级别进出场只在 k−1 是「链顶」时处理**
//! （= `layer[k]` 未占用，k−1 不是更高层的子腿）；当 `layer[k]` 占用时，k−1 是 k 的子腿，
//! 其同级别出场由父级 k 的 recover 统一处理（步骤 4=步骤 5 是同一操作的子/父两面），杜绝
//! 重复触发。core（最高占用层）恒为链顶。
//!
//! ## 第一次入场（现金 → 核心仓的唯一边界）
//! 全局空仓（n_base≈0）收到本 bar 最高 ladder 的 BSP：用**全部资金**建核心，方向 = 该 BSP
//! 字面（buy→FullLong / sell→FullShort）。塔由后续次级别反向 BSP 的 sink 逐层向下铺。
//! 清仓 / 强平清空后（n_base→0）自动允许重新入场。
//!
//! ## 递归多重赋格（子腿也是状态机）
//! spawn 出的子腿 `FullShort@(k−1)` 本身也是一个 level 状态机，遵守同一转移表：它收到自己的
//! 反向次级别 BSP@(k−2) 时 sink 出 `Long@(k−2)`（ε 镜像）。递归深度由信号驱动自然涌现，不
//! 硬编码层数。父级 recover/清仓时**级联折叠**整条子链（守恒归还 / 平到现金）。
//!
//! ## 会计模型（formalization-validity-domain 诚实标注）
//! 复用 fugue_v3 会计原语（操作层 ⊥ 会计层；`LevelState` = 操作投影，`Layer` = 会计投影，
//! units 单一真相源在 Layer）：
//! - **sink**（`cycle::sink_chunk`）：reduce_at(k, 1/3·u_k) + add_at(k−1, flip(d))，Σ|units| 不变。
//! - **recover**（级联折叠 `recover`）：reduce 整条子链 + add 归还 k，Σ|units| 不变（n_base 不变）。
//! - **清仓 / 强平**（`clear_to_cash` / `liquidate_*`）：reduce 到现金，**改 n_base**（边界）。
//! - **NAV 中性**：每个 reduce/add 同价 c 价值中性（`prove_nav_neutral`，L0）。
//! - **n_base（守恒锚）**：仅入场/清仓/强平/eod 改；sink/recover 转移守恒不改。
//!
//! ## 认识论等级
//! - 状态机转移结构 / sink-recover ε 对称 / Σ 守恒 / NAV 中性：**L0**；
//! - BSP fire 时机 / 清仓 vs 减仓的判定：**L2**；回测 alpha：**L3**。

use crate::trading::types::{Polarity, FIRST_BSP_LADDER, LADDER_MOVE, MAX_LADDER};

use crate::fugue_v3::accounting::{
    accrue_held_bars, active_voice_count, add_at, exposure, flip, nav, reduce_at,
};
use crate::fugue_v3::cycle::{liquidate_long, liquidate_short, sink_chunk};
use crate::fugue_v3::layer::{FugueResult, Layer};
use crate::fugue_v3::prove::{
    count_chiral_violations, prove_conservation, prove_nav_neutral, prove_recursive_consistency,
};
use crate::fugue_v3::{EQUITY_SAMPLE_BARS, INITIAL_CAPITAL, PENDING_LO, SUB_LIQ_FACTOR};

/// T level 0 在 ladder 空间的基准（= 走势级，move(L1)）。T level k ↦ ladder k + BASE_LADDER。
pub const BASE_LADDER: usize = LADDER_MOVE;

// ════════════════════════════ 离散级别状态（操作投影）════════════════════════════

/// 单个级别 k 的离散状态机状态（5 态）。units 真值在 `Layer`（单一真相源），本枚举只持
/// **离散语义标签**——避免双真相源（`FullLong` ⟺ layer 满仓，`ReducedLong` ⟺ layer 减为 2/3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LevelState {
    /// 空仓。
    Empty,
    /// 全仓做多（满仓）。
    FullLong,
    /// 减仓后的多头（2/3，已 spawn 子 Short@(k−1)）。
    ReducedLong,
    /// 全仓做空（满仓）。
    FullShort,
    /// 减仓后的空头（2/3，已 spawn 子 Long@(k−1)）。
    ReducedShort,
}

impl LevelState {
    /// 该状态的核心持仓方向（Empty → None）。
    fn direction(self) -> Option<Polarity> {
        match self {
            LevelState::Empty => None,
            LevelState::FullLong | LevelState::ReducedLong => Some(Polarity::Long),
            LevelState::FullShort | LevelState::ReducedShort => Some(Polarity::Short),
        }
    }

    /// 方向 → 满仓态。
    fn full(d: Polarity) -> Self {
        match d {
            Polarity::Long => LevelState::FullLong,
            Polarity::Short => LevelState::FullShort,
        }
    }

    /// 方向 → 减仓态。
    fn reduced(d: Polarity) -> Self {
        match d {
            Polarity::Long => LevelState::ReducedLong,
            Polarity::Short => LevelState::ReducedShort,
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

/// 该 ladder 是否有指定**极性**的 BSP（Long ⟺ 买点，Short ⟺ 卖点）。越界返回 false。
fn has_bsp(view: &TSignalView, ladder: usize, p: Polarity) -> bool {
    if ladder >= MAX_LADDER {
        return false;
    }
    match p {
        Polarity::Long => view.buy[ladder],
        Polarity::Short => view.sell[ladder],
    }
}

// ════════════════════════════ T 操作层引擎 ════════════════════════════

/// T 操作层引擎（离散 5 状态机 + 固定 6 步周期；递归多重赋格）。
pub struct TPositionEngine {
    res: FugueResult,
    /// 每级别会计真值（ladder 索引；units/basis/direction）。
    layers: Vec<Layer>,
    /// 每级别离散状态（操作投影，与 layers 一一对应）。
    states: [LevelState; MAX_LADDER],
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
            states: [LevelState::Empty; MAX_LADDER],
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

    /// 当前**核心持仓级别**（= 最高占用 ladder；塔从核心向下铺 ⟹ 核心恒为最高占用层 = 链顶）。
    fn core_ladder(&self) -> Option<usize> {
        (0..MAX_LADDER).rev().find(|&k| self.states[k] != LevelState::Empty)
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

    // ──────────────── 入场（步骤 1：现金 → 核心仓）────────────────

    /// **入场**：全局空仓时在本 bar 最高 ladder 的 BSP 级别用全部资金建核心，方向由该 BSP
    /// 字面决定（buy→FullLong / sell→FullShort）。塔由后续次级别反向 BSP 的 sink 逐层铺。
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
                    self.states[k] = LevelState::full(t);
                }
                return;
            }
        }
    }

    // ──────────────── 状态机驱动（步骤 2–6）────────────────

    /// 分层 BSP 驱动状态机（已非 global_flat）。
    ///
    /// (1) **core 清仓**（步骤 6）：core 及更高级别反向 BSP（持多遇卖点 / 持空遇买点）→ 清整链
    ///     到现金。缠论「同级别 / 大级别卖点 = 多头出场时机」（持空 ε 镜像）。清仓**不翻空**。
    /// (2) **次级别 sink/recover**（步骤 2–5）：遍历占用层（高到低），match 状态 × 次级别 BSP。
    fn drive(&mut self, view: &TSignalView, bar: i64, c: f64) {
        // ── (1) core 清仓（同级别 / 更高级别反向 BSP 优先于次级别短差）──
        if let Some(core) = self.core_ladder() {
            let d = self.states[core].direction().expect("core 占用必有方向");
            let exit = (core..MAX_LADDER).any(|j| has_bsp(view, j, flip(d)));
            if exit {
                self.res.n_core_clears_by_ladder[core] += 1;
                self.clear_to_cash(bar, c, "core_exit");
                return; // 清仓后回 global_flat，下个 BSP 重入
            }
        }

        // ── (2) 次级别 sink/recover（per-bar 互斥：每层每 bar 最多一次转移）──
        // 源层 k ≥ PENDING_LO（子腿 k−1 ≥ FIRST_BSP_LADDER，prove_recursive_consistency 下界）。
        let mut acted = [false; MAX_LADDER];
        for k in (PENDING_LO..MAX_LADDER).rev() {
            let st = self.states[k];
            let sub = k - 1;
            if st == LevelState::Empty || acted[k] || acted[sub] {
                continue;
            }
            let d = st.direction().expect("非 Empty 必有方向");
            match st {
                // FullLong/FullShort + 反向次级别 BSP（持多遇次级别卖点 / 持空遇次级别买点）→ sink。
                LevelState::FullLong | LevelState::FullShort if has_bsp(view, sub, flip(d)) => {
                    if self.sink(k, bar, c) {
                        acted[k] = true;
                        acted[sub] = true;
                    }
                }
                // ReducedLong/ReducedShort + 同向次级别 BSP（持多遇次级别买点 / 持空遇次级别卖点）→ recover。
                LevelState::ReducedLong | LevelState::ReducedShort if has_bsp(view, sub, d) => {
                    if self.recover(k, bar, c) {
                        acted[k] = true;
                        // 折叠可涉及整条子链，全标 acted（杜绝同 bar 重复操作）。
                        for slot in acted.iter_mut().take(k).skip(FIRST_BSP_LADDER) {
                            *slot = true;
                        }
                    }
                }
                // 其余组合（Full+同向次级别 / Reduced+反向次级别 / Empty）→ 忽略。
                _ => {}
            }
        }
    }

    /// **sink @ k**（步骤 2：Full + 反向次级别 BSP）：减 k 的 1/3 下放 k−1 做短差（穿手性缝，
    /// 方向 flip(d)），k → Reduced。复用 `sink_chunk`（含 σ-quota / cross-level 守卫）。返回成功否。
    fn sink(&mut self, k: usize, bar: i64, c: f64) -> bool {
        let d = self.states[k].direction().expect("sink 前提：k 占用");
        if sink_chunk(&mut self.layers, k, &mut self.free, c, bar, &mut self.res) {
            self.states[k] = LevelState::reduced(d);
            self.states[k - 1] = LevelState::full(flip(d));
            true
        } else {
            false
        }
    }

    /// **recover @ k**（步骤 5：Reduced + 同向次级别 BSP）：级联折叠 k 以下整条子链，units 归还
    /// k，k → Full。Σ 守恒（reduce 子链 + add 归还同量，n_base 不变）。返回成功否。
    ///
    /// 折叠覆盖 [FIRST_BSP_LADDER, k) 全部占用层——递归子腿（k−1 自身 Reduced 有 k−2 子腿）时把
    /// 整条下放的 1/3 一次卷回 k。NAV 中性（reduce/add 同价 c）。
    fn recover(&mut self, k: usize, bar: i64, c: f64) -> bool {
        let d = self.states[k].direction().expect("recover 前提：k 占用");
        let mut total = 0.0;
        for j in (FIRST_BSP_LADDER..k).rev() {
            let u = self.layers[j].units;
            if u > 1e-12 {
                reduce_at(&mut self.layers, j, u, &mut self.free, c, bar, &mut self.res, "recover");
                self.states[j] = LevelState::Empty;
                total += u;
            }
        }
        if total <= 1e-12 {
            return false; // 子腿已被强平清空，无可回补（k 保持 Reduced 等同级别清仓）
        }
        add_at(&mut self.layers, k, total, d, &mut self.free, c, bar, &mut self.res);
        self.states[k] = LevelState::full(d);
        self.res.n_cycle_closes_by_ladder[k] += 1;
        self.res.cross_level_closures += 1;
        true
    }

    /// 整链清到现金（步骤 6：core 出场级联平全塔 / eod 收尾）。改 n_base（边界算子）。
    fn clear_to_cash(&mut self, bar: i64, c: f64, reason: &'static str) {
        for k in 0..self.layers.len() {
            if self.layers[k].units > 1e-12 {
                let u = self.layers[k].units;
                reduce_at(&mut self.layers, k, u, &mut self.free, c, bar, &mut self.res, reason);
            }
            self.states[k] = LevelState::Empty;
        }
        self.n_base = 0.0;
    }

    /// **边界算子**：1x 逐仓强平（多空对称；市场被动触发）。强平层 → Empty，n_base 缩水。
    fn liquidate_boundary(&mut self, bar: i64, c: f64) {
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
                    Polarity::Short => {
                        liquidate_short(&mut self.layers, k, &mut self.free, c, bar, &mut self.res)
                    }
                    Polarity::Long => {
                        liquidate_long(&mut self.layers, k, &mut self.free, c, bar, &mut self.res)
                    }
                };
                self.n_base -= m;
                self.states[k] = LevelState::Empty;
            }
        }
    }

    // ──────────────── 单 bar 步进 ────────────────

    /// 单 bar 操作步进：A 边界强平 → B BSP 状态机（入场 / core 清仓 / 次级别 sink/recover）
    /// → C 守卫 → D 观测。
    pub fn step(&mut self, view: &TSignalView, bar: i64, price: f64) {
        let c = price;
        self.last_close = c;
        let nav_pre = nav(&self.layers, self.free, c);

        // ── A. 边界算子：1x 逐仓强平 ──
        self.liquidate_boundary(bar, c);

        // ── B. BSP 状态机（无 BSP 不动）──
        if self.global_flat() {
            self.try_enter(view, bar, c);
        } else {
            self.drive(view, bar, c);
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

    // ──────────────── 入场（步骤 1）────────────────

    #[test]
    fn 入场买点全压最高信号层做多() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let (navv, lu, su, _) = eng.snapshot();
        assert!((lu - 1000.0).abs() < 1e-6, "满仓 1000 units，得 {lu}");
        assert_eq!(su, 0.0);
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "入场 NAV 中性");
        assert_eq!(eng.states[5], LevelState::FullLong);
        assert!((eng.n_base - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn 入场卖点全压最高信号层做空() {
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0);
        assert!((su - 1000.0).abs() < 1e-6, "满仓 1000 Short，得 {su}");
        assert_eq!(eng.states[5], LevelState::FullShort);
    }

    // ──────────────── 步骤 2–3：减仓做空 ────────────────

    #[test]
    fn 步骤2_3_次级别卖点sink减仓做空() {
        // FullLong@5 + 次级别卖点@4（反向）→ ReducedLong@5 + spawn FullShort@4（减 1/3 做空）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&sell_view(4), 10, 110.0);
        let (_, lu, su, _) = eng.snapshot();
        assert!((lu - base * 2.0 / 3.0).abs() < 1e-6, "核心剩 2/3 Long，得 {lu}");
        assert!((su - base / 3.0).abs() < 1e-6, "次级别 1/3 Short，得 {su}");
        assert_eq!(eng.states[5], LevelState::ReducedLong);
        assert_eq!(eng.states[4], LevelState::FullShort);
        assert!((eng.n_base - base).abs() < 1e-9, "sink Σ 守恒（n_base 不变）");
        assert_eq!(eng.core_ladder(), Some(5), "核心仍在最高层 5");
    }

    // ──────────────── 步骤 4–5：平空回补 ────────────────

    #[test]
    fn 步骤4_5_次级别买点recover回补归还满仓() {
        // FullLong@5 → sink → 次级别买点@4（同向）→ recover：平子空头，归还核心 FullLong@5。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&sell_view(4), 10, 110.0); // sink → ladder4 Short 333, 核心 667
        eng.step(&buy_view(4), 20, 105.0); // 次级别买点 → recover
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(su, 0.0, "次级别空头全平");
        assert!((lu - base).abs() < 1e-6, "核心恢复全仓 1000（回补），得 {lu}");
        assert_eq!(eng.states[5], LevelState::FullLong, "回 FullLong");
        assert_eq!(eng.states[4], LevelState::Empty, "子腿空");
        // 空头 110→105 降价回补盈利。
        let mob: f64 = eng.result().mobile_realized_pnl_by_ladder.iter().sum();
        assert!(mob > 0.0, "次级别空头降价回补盈利，得 {mob}");
    }

    // ──────────────── 步骤 6：清仓 ────────────────

    #[test]
    fn 步骤6_同级别卖点清仓出场不翻空() {
        // FullLong@5 + 同级别卖点@5（反向）→ Empty（清仓回现金，不翻空）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        eng.step(&sell_view(5), 10, 110.0);
        let (navv, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "同级别卖点清多头");
        assert_eq!(su, 0.0, "不翻空——回现金，无空头");
        assert_eq!(eng.states[5], LevelState::Empty);
        assert!(eng.global_flat(), "清仓后 global_flat");
        assert!(navv > INITIAL_CAPITAL, "升价清仓盈利，得 {navv}");
        assert_eq!(eng.res.n_core_clears_by_ladder[5], 1);
    }

    #[test]
    fn 步骤6_reduced态同级别卖点清仓级联平子腿() {
        // ReducedLong@5 + 子 Short@4 + 同级别卖点@5 → 清仓级联（平核心 + 子腿）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        eng.step(&sell_view(4), 10, 110.0); // sink → ReducedLong@5 + FullShort@4
        assert_eq!(eng.states[4], LevelState::FullShort, "短差空头已建");
        eng.step(&sell_view(5), 20, 115.0); // 同级别卖点 → 清仓级联
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0);
        assert_eq!(su, 0.0, "级联平全塔（含 ladder4 短差）");
        assert_eq!(eng.states[5], LevelState::Empty);
        assert_eq!(eng.states[4], LevelState::Empty);
        assert!(eng.global_flat());
    }

    // ──────────────── 同向 / 更高级别 ────────────────

    #[test]
    fn 同级别买点持多no_op() {
        // FullLong@5 + 同级别买点@5（同向）→ no-op（已满仓，不重复建仓）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let n_before = eng.n_trades();
        eng.step(&buy_view(5), 10, 110.0);
        let (_, lu, su, _) = eng.snapshot();
        assert!((lu - 1000.0).abs() < 1e-6, "仍满仓 Long 1000，得 {lu}");
        assert_eq!(su, 0.0);
        assert_eq!(eng.n_trades(), n_before, "同向同级别 BSP 无 trade");
        assert_eq!(eng.states[5], LevelState::FullLong);
    }

    #[test]
    fn 更高级别卖点清仓出场() {
        // FullLong@5 + 更高级别卖点@7（大级别顶背驰）→ 清仓（缠论大级别优先）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        eng.step(&sell_view(7), 10, 110.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "更高级别卖点清多头");
        assert_eq!(su, 0.0, "不翻空");
        assert_eq!(eng.states[5], LevelState::Empty);
        assert!(eng.global_flat(), "清仓后 global_flat");
        assert_eq!(eng.res.n_core_clears_by_ladder[5], 1);
    }

    // ──────────────── ε 镜像（做空场景）────────────────

    #[test]
    fn ε镜像_持空次级别买点sink下放做多() {
        // FullShort@5 + 次级别买点@4（反向）→ ReducedShort@5 + spawn FullLong@4（ε 镜像）。
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&buy_view(4), 10, 90.0);
        let (_, lu, su, _) = eng.snapshot();
        assert!((su - base * 2.0 / 3.0).abs() < 1e-6, "核心剩 2/3 Short，得 {su}");
        assert!((lu - base / 3.0).abs() < 1e-6, "次级别 1/3 Long（ε 镜像做多），得 {lu}");
        assert_eq!(eng.states[5], LevelState::ReducedShort);
        assert_eq!(eng.states[4], LevelState::FullLong);
        assert!((eng.n_base - base).abs() < 1e-9, "sink Σ 守恒");
    }

    #[test]
    fn ε镜像_持空次级别卖点recover归还满仓() {
        // FullShort@5 → sink → 次级别卖点@4（同向）→ recover：平子多头，归还 FullShort@5。
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&buy_view(4), 10, 90.0); // sink → ladder4 Long 333, 核心 Short 667
        eng.step(&sell_view(4), 20, 95.0); // 次级别卖点 → recover
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "次级别多头全平");
        assert!((su - base).abs() < 1e-6, "核心恢复全仓 Short 1000，得 {su}");
        assert_eq!(eng.states[5], LevelState::FullShort);
        assert_eq!(eng.states[4], LevelState::Empty);
    }

    #[test]
    fn ε镜像_持空同级别买点清仓出场() {
        // FullShort@5 + 同级别买点@5（反向）→ Empty（ε 镜像：买点 = 空头出场时机）。
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0);
        eng.step(&buy_view(5), 10, 90.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0);
        assert_eq!(su, 0.0, "同级别买点清空头出场");
        assert_eq!(eng.states[5], LevelState::Empty);
        assert!(eng.global_flat(), "清仓后 global_flat");
    }

    // ──────────────── 循环 + 离散性 ────────────────

    #[test]
    fn 步骤2_5循环多次往返() {
        // FullLong ⇌ ReducedLong 往返两轮（多个次级别回调），守恒不变。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&sell_view(4), 10, 110.0); // 减仓做空（第一轮）
        eng.step(&buy_view(4), 20, 105.0); // 回补
        assert_eq!(eng.states[5], LevelState::FullLong, "第一轮回补后 FullLong");
        eng.step(&sell_view(4), 30, 112.0); // 再减仓做空（第二轮）
        eng.step(&buy_view(4), 40, 108.0); // 再回补
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(su, 0.0);
        assert!((lu - base).abs() < 1e-6, "两轮往返后核心恢复全仓 {base}，得 {lu}");
        assert_eq!(eng.states[5], LevelState::FullLong);
        assert!((eng.n_base - base).abs() < 1e-9, "往返 Σ 守恒");
    }

    #[test]
    fn 离散性_reduced态次级别卖点不二次减() {
        // ReducedLong@5（已减 1/3）+ 次级别卖点@4（反向）→ 忽略（离散 5 态：Reduced 不二次减）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&sell_view(4), 10, 110.0); // sink → ReducedLong@5 + FullShort@4
        let lu_after_first = eng.snapshot().1;
        eng.step(&sell_view(4), 20, 112.0); // 第二个次级别卖点 → 应忽略（已 Reduced）
        let (_, lu, su, _) = eng.snapshot();
        assert!((lu - lu_after_first).abs() < 1e-6, "核心不二次减（仍 2/3），得 {lu}");
        assert!((su - base / 3.0).abs() < 1e-6, "短差空头仍 1/3，得 {su}");
        assert_eq!(eng.states[5], LevelState::ReducedLong);
    }

    // ──────────────── 递归子腿 ────────────────

    #[test]
    fn 递归_子腿再sink多重赋格三级别同时持仓() {
        // FullLong@5 → sink → FullShort@4。子腿 FullShort@4 + 次级别买点@3（反向 flip(Short)=Long）
        // → 子腿 sink 出 FullLong@3（ε 镜像递归）。三级别手性交替同时持仓。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        eng.step(&sell_view(4), 10, 110.0); // sink → ReducedLong@5 + FullShort@4
        eng.step(&buy_view(3), 20, 105.0); // 子腿 sink → ReducedShort@4 + FullLong@3
        assert_eq!(eng.states[5], LevelState::ReducedLong);
        assert_eq!(eng.states[4], LevelState::ReducedShort);
        assert_eq!(eng.states[3], LevelState::FullLong);
        assert!(eng.layers[5].direction == Polarity::Long && eng.layers[5].units > 1e-6);
        assert!(eng.layers[4].direction == Polarity::Short && eng.layers[4].units > 1e-6);
        assert!(eng.layers[3].direction == Polarity::Long && eng.layers[3].units > 1e-6);
        assert!((eng.n_base - 1000.0).abs() < 1e-6, "多重赋格 Σ 守恒");
        assert_eq!(eng.core_ladder(), Some(5), "核心仍在最高层 5");
    }

    #[test]
    fn 递归_recover级联折叠整条子链() {
        // 三层塔（ReducedLong@5 / ReducedShort@4 / FullLong@3）→ 次级别买点@4 recover：
        // 级联折叠 k<5 子链（含 ladder3 + ladder4）归还核心 FullLong@5。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&sell_view(4), 10, 110.0); // ReducedLong@5 + FullShort@4
        eng.step(&buy_view(3), 20, 105.0); // 子腿 sink → ReducedShort@4 + FullLong@3
        eng.step(&buy_view(4), 30, 108.0); // ReducedLong@5 + 同向次级别买点@4 → recover 折叠
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(su, 0.0, "子链空头全平");
        assert!((lu - base).abs() < 1e-6, "核心折叠恢复全仓 {base}，得 {lu}");
        assert_eq!(eng.states[5], LevelState::FullLong);
        assert_eq!(eng.states[4], LevelState::Empty);
        assert_eq!(eng.states[3], LevelState::Empty);
        assert!((eng.n_base - base).abs() < 1e-6, "折叠 Σ 守恒");
    }

    // ──────────────── 边界 / 守恒 ────────────────

    #[test]
    fn empty态次级别bsp忽略() {
        // 空仓时次级别 BSP 不入场（入场只认最高 ladder 全仓）。
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(4), 0, 100.0);
        assert_eq!(eng.core_ladder(), Some(4), "空仓入场以最高信号 ladder 为核心");
        assert_eq!(eng.states[4], LevelState::FullShort);
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
        assert_eq!(eng.states[5], LevelState::Empty);
        assert_eq!(eng.res.n_liquidations_by_ladder[5], 1);
        eng.step(&buy_view(5), 20, 50.0); // 强平后买点重新入场
        assert_eq!(eng.states[5], LevelState::FullLong, "重新入场做多");
        assert!(eng.layers[5].units > 1e-6);
    }

    #[test]
    fn 空头价格翻倍爆仓nav归零() {
        // 1x 逐仓做空，价格涨到 2×basis ⟹ NAV→0（做空本质风险，非 bug）。
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0); // Short@5 1000@100
        eng.step(&TSignalView::empty(), 10, 200.0); // c≥2×basis 强平
        assert_eq!(eng.res.n_liquidations_by_ladder[5], 1);
        assert_eq!(eng.states[5], LevelState::Empty);
        let (navv, _, _, _) = eng.snapshot();
        assert!(navv.abs() < 1.0, "空头翻倍爆仓 NAV→0，得 {navv}");
    }

    #[test]
    fn 仓位层合法() {
        assert!(BASE_LADDER > FIRST_BSP_LADDER, "核心层 sink 下放至 FIRST_BSP_LADDER 仍合法");
        assert_eq!(PENDING_LO, FIRST_BSP_LADDER + 1, "源层下界 = FIRST_BSP+1（子腿 ≥ FIRST_BSP）");
    }
}
