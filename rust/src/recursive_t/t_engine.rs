//! **T 算子操作层引擎**（4 状态翻转版：永远在市场，同级别反向 BSP = 整仓翻转）。
//!
//! ## 本次重写（编排者裁决 2026-06-19）：翻转版替代清仓版
//! 上一版（清仓版）同级别反向 BSP → Empty（清仓回现金）。本版改为**翻转**：同级别反向 BSP →
//! 整仓翻转到反方向（FullLong↔FullShort），**永远在市场有方向**，没有「空仓」状态（除最初
//! 入场前 / 强平爆仓后）。翻转是 M=N 等量翻转（清旧向全塔 + 建等量反向核心），非清仓。
//!
//! ## 4 个活跃状态（+ Empty 边界态）
//! `FullLong`（满多）/ `ReducedLong`（减仓多 2/3）/ `FullShort`（满空）/ `ReducedShort`（减仓空 2/3）。
//! `Empty` 仅出现在：最初入场前、强平爆仓后（等重新入场）。正常运转永不回 Empty。
//!
//! ## 状态机转移表（做多侧；做空侧 ε 镜像）
//! ```text
//! Empty         --[首个BSP]----->  FullLong / FullShort（入场，方向字面）
//! FullLong      --[同级Sell]----->  FullShort（整仓翻转：清多+建等量空，M=N）
//! FullShort     --[同级Buy]------>  FullLong（整仓翻转：清空+建等量多，ε 镜像）
//! FullLong      --[次级Sell]----->  ReducedLong + spawn Short@(k−1)（减仓做空，不变）
//! ReducedLong   --[次级Buy]------>  FullLong（回补恢复满仓，守恒归还，不变）
//! ReducedLong   --[同级Sell]----->  FullShort（翻转：平子空头+回补+整仓翻转到等量空）
//! ReducedShort  --[同级Buy]------>  FullLong（翻转：平子多头+回补+整仓翻转，ε 镜像）
//! ```
//! 关键约束：
//! - 同级别反向 BSP（持多遇卖点 / 持空遇买点）= **翻转**（不是清仓）。永远在市场。
//! - 次级别短差逻辑**不变**（Full⇌Reduced，次级别 sink/recover）。
//! - ReducedLong 遇同级别卖点 → flip_core 一次性平 core(2/3 多)+子(1/3 空) 再建 U 空（M=N）。
//! - 次级别 Sell 只在 Full 触发（Reduced 不二次减）；次级别 Buy 只在 Reduced 触发（回补）。
//!
//! ## 翻转会计（M=N 等量翻转，Σ|units| 守恒）
//! `flip_core(k, new_dir)`：reduce_at 平掉**全塔**（core + 所有子腿，累计 total = n_base）+
//! add_at(k, total, new_dir) 建等量反向核心。Σ|units| 不变（reduce total + add total），
//! **n_base 不变**（翻转非边界算子，不像清仓/强平改 n_base）。NAV 中性（reduce/add 同价 c）。
//!
//! ## 分层职责消歧（275号局部依赖：链顶判定）
//! 信号 `buy@(k−1)` 既是 level k 的「次级别买点」又是 level k−1 的「同级别买点」。消歧：core
//! （最高占用层 = 链顶）处理同级别翻转；子腿的同级别出场由父级 recover（次级别买点）统一处理。
//!
//! ## 第一次入场 + 强平重入（Empty 的唯一来源）
//! 全局空仓（n_base≈0）收到本 bar 最高 ladder 的 BSP：用全部资金建核心，方向字面。之后**永远
//! 翻转**（不回 Empty）。仅 1x 逐仓强平爆仓（NAV 风险）使该层回 Empty；若 core 全爆 ⟹
//! global_flat ⟹ 下个 BSP 重新入场。
//!
//! ## 递归多重赋格（子腿也是状态机）
//! spawn 出的子腿 `FullShort@(k−1)` 本身遵守同一转移表，收到反向次级别 BSP@(k−2) 时 sink 出
//! `Long@(k−2)`（ε 镜像）。翻转时 flip_core 级联平掉整条子链。
//!
//! ## 认识论等级
//! - 状态机转移结构 / M=N 翻转 / sink-recover ε 对称 / Σ 守恒 / NAV 中性：**L0**；
//! - BSP fire 时机 / 翻转 vs 减仓的判定：**L2**；回测 alpha（翻转版机械 1:1 暴露风险）：**L3**。

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

/// 单个级别 k 的离散状态机状态（4 活跃态 + Empty 边界态）。units 真值在 `Layer`（单一真相源），
/// 本枚举只持**离散语义标签**——`FullLong` ⟺ layer 满仓，`ReducedLong` ⟺ layer 减为 2/3。
/// `Empty` 仅入场前 / 强平爆仓后出现（翻转版永远在市场，正常运转不回 Empty）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LevelState {
    /// 空仓（仅入场前 / 强平后）。
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

/// T 操作层引擎（4 状态翻转版：永远在市场，同级别反向 BSP = 整仓翻转；递归多重赋格）。
pub struct TPositionEngine {
    res: FugueResult,
    /// 每级别会计真值（ladder 索引；units/basis/direction）。
    layers: Vec<Layer>,
    /// 每级别离散状态（操作投影，与 layers 一一对应）。
    states: [LevelState; MAX_LADDER],
    /// 统一现金池（NAV 中性双重会计的现金腿）。
    free: f64,
    /// 当前总持仓基数 Σ|units|（会计自洽锚；仅入场/强平/eod 改，翻转/sink/recover 守恒不改）。
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

    /// 全局是否空仓（n_base≈0 ⟹ 资金全在 free；仅最初 / 强平爆仓清空后为真 ⟹ 下个 BSP 重新入场）。
    fn global_flat(&self) -> bool {
        self.n_base <= 1e-9
    }

    /// 当前**核心持仓级别**（= 最高占用 ladder = 链顶；翻转/sink 都以核心为锚）。
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

    // ──────────────── 入场（现金 → 核心仓；Empty 的唯一退出）────────────────

    /// **入场**：全局空仓时（最初 / 强平后）在本 bar 最高 ladder 的 BSP 级别用全部资金建核心，
    /// 方向由该 BSP 字面决定（buy→FullLong / sell→FullShort）。之后永远翻转，不回 Empty。
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

    // ──────────────── 状态机驱动 ────────────────

    /// 分层 BSP 驱动状态机（已非 global_flat）。
    ///
    /// (1) **翻转**：core 及更高级别反向 BSP（持多遇卖点 / 持空遇买点）→ 整仓翻转到反方向
    ///     （M=N，永远在市场，**非清仓**）。
    /// (2) **次级别 sink/recover**：遍历占用层（高到低），match 状态 × 次级别 BSP（不变）。
    fn drive(&mut self, view: &TSignalView, bar: i64, c: f64) {
        // ── (1) 翻转（同级别 / 更高级别反向 BSP 优先于次级别短差）──
        if let Some(core) = self.core_ladder() {
            let d = self.states[core].direction().expect("core 占用必有方向");
            let reverse = (core..MAX_LADDER).any(|j| has_bsp(view, j, flip(d)));
            if reverse {
                self.flip_core(core, flip(d), bar, c);
                return; // 翻转后本 bar 结束（仍在市场，新方向）
            }
        }

        // ── (2) 次级别 sink/recover（per-bar 互斥：每层每 bar 最多一次转移）──
        let mut acted = [false; MAX_LADDER];
        for k in (PENDING_LO..MAX_LADDER).rev() {
            let st = self.states[k];
            let sub = k - 1;
            if st == LevelState::Empty || acted[k] || acted[sub] {
                continue;
            }
            let d = st.direction().expect("非 Empty 必有方向");
            match st {
                // Full + 反向次级别 BSP → sink（减 1/3 下放做短差）。
                LevelState::FullLong | LevelState::FullShort if has_bsp(view, sub, flip(d)) => {
                    if self.sink(k, bar, c) {
                        acted[k] = true;
                        acted[sub] = true;
                    }
                }
                // Reduced + 同向次级别 BSP → recover（折叠子链归还核心）。
                LevelState::ReducedLong | LevelState::ReducedShort if has_bsp(view, sub, d) => {
                    if self.recover(k, bar, c) {
                        acted[k] = true;
                        for slot in acted.iter_mut().take(k).skip(FIRST_BSP_LADDER) {
                            *slot = true;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// **翻转 @ k**（同级别 / 更高级别反向 BSP）：整仓翻转到 `new_dir`（清旧向**全塔** + 建等量
    /// 反向核心），永远在市场（M=N 等量翻转，**非清仓回现金**）。
    ///
    /// 平掉 core + 所有子腿（累计 total = 当前 Σ|units| = n_base），在 k 建 total 的 new_dir 核心。
    /// Σ|units| 守恒（reduce total + add total），**n_base 不变**（翻转非边界算子）。NAV 中性
    /// （reduce/add 同价 c）。ReducedLong 翻转 ⟹ 一次平 core(2/3)+子(1/3) 再建 U 反向（用户第6点）。
    fn flip_core(&mut self, k: usize, new_dir: Polarity, bar: i64, c: f64) {
        let mut total = 0.0;
        for j in 0..self.layers.len() {
            let u = self.layers[j].units;
            if u > 1e-12 {
                reduce_at(&mut self.layers, j, u, &mut self.free, c, bar, &mut self.res, "flip");
                self.states[j] = LevelState::Empty;
                total += u;
            }
        }
        if total > 1e-12 {
            add_at(&mut self.layers, k, total, new_dir, &mut self.free, c, bar, &mut self.res);
            self.states[k] = LevelState::full(new_dir);
            self.res.n_core_clears_by_ladder[k] += 1; // 复用为翻转计数（观测）
        }
        // n_base 不变（reduce total + add total 守恒翻转）。
    }

    /// **sink @ k**（Full + 反向次级别 BSP）：减 k 的 1/3 下放 k−1 做短差（穿手性缝，方向
    /// flip(d)），k → Reduced。复用 `sink_chunk`（含 σ-quota / cross-level 守卫）。返回成功否。
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

    /// **recover @ k**（Reduced + 同向次级别 BSP）：级联折叠 k 以下整条子链，units 归还 k，
    /// k → Full。Σ 守恒（reduce 子链 + add 归还同量，n_base 不变）。返回成功否。
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
            return false; // 子腿已被强平清空，无可回补
        }
        add_at(&mut self.layers, k, total, d, &mut self.free, c, bar, &mut self.res);
        self.states[k] = LevelState::full(d);
        self.res.n_cycle_closes_by_ladder[k] += 1;
        self.res.cross_level_closures += 1;
        true
    }

    /// 整链清到现金（仅 eod 收尾）。改 n_base（边界算子）。翻转版正常运转不清仓——本方法只在
    /// `finish` 调用（末 bar 平仓结算）。
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
    /// core 全爆 ⟹ global_flat ⟹ 重新入场（Empty 的唯一非入场来源）。
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

    /// 单 bar 操作步进：A 边界强平 → B BSP 状态机（入场 / 翻转 / 次级别 sink/recover）
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

    // ──────────────── 入场 ────────────────

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

    // ──────────────── 翻转（核心新语义）────────────────

    #[test]
    fn 翻转_同级别卖点fulllong翻fullshort不清仓() {
        // FullLong@5 + 同级别卖点@5（反向）→ FullShort@5（M=N 翻转，永远在市场，非清仓）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // FullLong@5 = 1000
        let base = eng.n_base;
        eng.step(&sell_view(5), 10, 110.0);
        let (navv, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "多头清掉");
        assert!((su - base).abs() < 1e-6, "翻转建等量空头 1000（M=N），得 {su}");
        assert_eq!(eng.states[5], LevelState::FullShort, "翻转到 FullShort");
        assert!(!eng.global_flat(), "翻转后仍在市场（非清仓）");
        assert!((eng.n_base - base).abs() < 1e-9, "翻转 n_base 守恒不变");
        // 翻转本身 NAV 中性由 step 内 prove_nav_neutral 守（nav_pre=nav_post）；多头 100→110
        // 升值在翻转时已实现 +1万入 free ⟹ 翻转后 NAV=110000（非 INITIAL）。
        assert!((navv - 110_000.0).abs() < 1.0, "翻转后 NAV=110000（多头已实现 +1万），得 {navv}");
    }

    #[test]
    fn 翻转_往返多空多永远在市场() {
        // FullLong → 卖点翻 FullShort → 买点翻 FullLong，全程在市场。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&sell_view(5), 10, 110.0); // → FullShort
        assert_eq!(eng.states[5], LevelState::FullShort);
        assert!(!eng.global_flat());
        eng.step(&buy_view(5), 20, 105.0); // → FullLong（ε 镜像翻回）
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(su, 0.0);
        assert!((lu - base).abs() < 1e-6, "翻回满仓多头 1000，得 {lu}");
        assert_eq!(eng.states[5], LevelState::FullLong);
        assert!(!eng.global_flat(), "往返全程在市场");
        assert!((eng.n_base - base).abs() < 1e-9, "往返 n_base 守恒");
    }

    #[test]
    fn 翻转_fullshort同级别买点翻fulllong() {
        // ε 镜像：FullShort@5 + 同级别买点@5 → FullLong@5。
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&buy_view(5), 10, 90.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(su, 0.0);
        assert!((lu - base).abs() < 1e-6, "翻转建等量多头，得 {lu}");
        assert_eq!(eng.states[5], LevelState::FullLong);
        assert!(!eng.global_flat());
    }

    #[test]
    fn 翻转_reduced态同级别卖点平子加翻转() {
        // ReducedLong@5(2/3) + 子 Short@4(1/3) + 同级别卖点@5 → FullShort@5(满仓空)，子腿平掉。
        // 用户第6点：先平子空头+回补，再整仓翻转（flip_core 一次完成，total=2/3+1/3=U）。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0); // FullLong@5 = 1000
        let base = eng.n_base;
        eng.step(&sell_view(4), 10, 110.0); // sink → ReducedLong@5(667) + FullShort@4(333)
        assert_eq!(eng.states[5], LevelState::ReducedLong);
        assert_eq!(eng.states[4], LevelState::FullShort);
        eng.step(&sell_view(5), 20, 115.0); // 同级别卖点 → 翻转
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "多头（含 core 2/3）全清");
        assert!((su - base).abs() < 1e-6, "翻转建满仓空 1000（core 2/3 + 子 1/3 = U），得 {su}");
        assert_eq!(eng.states[5], LevelState::FullShort, "core 翻 FullShort");
        assert_eq!(eng.states[4], LevelState::Empty, "子腿平掉");
        assert!(!eng.global_flat());
        assert!((eng.n_base - base).abs() < 1e-9, "翻转 n_base 守恒");
    }

    #[test]
    fn 翻转_更高级别卖点也翻转() {
        // FullLong@5 + 更高级别卖点@7（大级别顶背驰）→ 翻转 FullShort@5。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&sell_view(7), 10, 110.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0);
        assert!((su - base).abs() < 1e-6, "更高级别卖点翻转建空");
        assert_eq!(eng.states[5], LevelState::FullShort);
        assert!(!eng.global_flat());
    }

    // ──────────────── 次级别短差（不变）────────────────

    #[test]
    fn 次级别卖点sink减仓做空() {
        // FullLong@5 + 次级别卖点@4 → ReducedLong@5 + spawn FullShort@4。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&sell_view(4), 10, 110.0);
        let (_, lu, su, _) = eng.snapshot();
        assert!((lu - base * 2.0 / 3.0).abs() < 1e-6, "核心剩 2/3 Long，得 {lu}");
        assert!((su - base / 3.0).abs() < 1e-6, "次级别 1/3 Short，得 {su}");
        assert_eq!(eng.states[5], LevelState::ReducedLong);
        assert_eq!(eng.states[4], LevelState::FullShort);
        assert!((eng.n_base - base).abs() < 1e-9, "sink Σ 守恒");
        assert_eq!(eng.core_ladder(), Some(5));
    }

    #[test]
    fn 次级别买点recover回补归还满仓() {
        // FullLong@5 → sink → 次级别买点@4 → recover 回补 FullLong@5。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&sell_view(4), 10, 110.0);
        eng.step(&buy_view(4), 20, 105.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(su, 0.0, "次级别空头全平");
        assert!((lu - base).abs() < 1e-6, "核心恢复全仓 1000，得 {lu}");
        assert_eq!(eng.states[5], LevelState::FullLong);
        assert_eq!(eng.states[4], LevelState::Empty);
        let mob: f64 = eng.result().mobile_realized_pnl_by_ladder.iter().sum();
        assert!(mob > 0.0, "次级别空头降价回补盈利，得 {mob}");
    }

    #[test]
    fn ε镜像_持空次级别买点sink下放做多() {
        // FullShort@5 + 次级别买点@4 → ReducedShort@5 + spawn FullLong@4。
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&buy_view(4), 10, 90.0);
        let (_, lu, su, _) = eng.snapshot();
        assert!((su - base * 2.0 / 3.0).abs() < 1e-6, "核心剩 2/3 Short，得 {su}");
        assert!((lu - base / 3.0).abs() < 1e-6, "次级别 1/3 Long（ε 镜像），得 {lu}");
        assert_eq!(eng.states[5], LevelState::ReducedShort);
        assert_eq!(eng.states[4], LevelState::FullLong);
        assert!((eng.n_base - base).abs() < 1e-9);
    }

    #[test]
    fn 离散性_reduced态次级别卖点不二次减() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&sell_view(4), 10, 110.0);
        let lu_after_first = eng.snapshot().1;
        eng.step(&sell_view(4), 20, 112.0); // 第二个次级别卖点 → 忽略（已 Reduced）
        let (_, lu, su, _) = eng.snapshot();
        assert!((lu - lu_after_first).abs() < 1e-6, "核心不二次减（仍 2/3），得 {lu}");
        assert!((su - base / 3.0).abs() < 1e-6, "短差空头仍 1/3");
        assert_eq!(eng.states[5], LevelState::ReducedLong);
    }

    // ──────────────── 同向 no-op ────────────────

    #[test]
    fn 同级别买点持多no_op() {
        // FullLong@5 + 同级别买点@5（同向）→ no-op（不翻转，不重复建仓）。
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

    // ──────────────── 递归子腿 ────────────────

    #[test]
    fn 递归_子腿再sink多重赋格三级别同时持仓() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        eng.step(&sell_view(4), 10, 110.0); // ReducedLong@5 + FullShort@4
        eng.step(&buy_view(3), 20, 105.0); // 子腿 sink → ReducedShort@4 + FullLong@3
        assert_eq!(eng.states[5], LevelState::ReducedLong);
        assert_eq!(eng.states[4], LevelState::ReducedShort);
        assert_eq!(eng.states[3], LevelState::FullLong);
        assert!((eng.n_base - 1000.0).abs() < 1e-6, "多重赋格 Σ 守恒");
        assert_eq!(eng.core_ladder(), Some(5));
    }

    #[test]
    fn 递归_翻转级联平整条子链() {
        // 三层塔 → core 同级别卖点 → flip_core 级联平 ladder3+4+5 多头部分 + 建满仓空。
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(5), 0, 100.0);
        let base = eng.n_base;
        eng.step(&sell_view(4), 10, 110.0); // ReducedLong@5 + FullShort@4
        eng.step(&buy_view(3), 20, 105.0); // ReducedShort@4 + FullLong@3
        eng.step(&sell_view(5), 30, 108.0); // core 同级别卖点 → 翻转
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "全塔多头清掉");
        assert!((su - base).abs() < 1e-6, "翻转建满仓空 1000（全塔 total），得 {su}");
        assert_eq!(eng.states[5], LevelState::FullShort);
        assert_eq!(eng.states[4], LevelState::Empty);
        assert_eq!(eng.states[3], LevelState::Empty);
        assert!((eng.n_base - base).abs() < 1e-6, "翻转 Σ 守恒");
    }

    // ──────────────── 边界 / 守恒 ────────────────

    #[test]
    fn empty态次级别bsp忽略() {
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
        // 强平爆仓 → Empty → 重新入场（Empty 的唯一非入场来源）。
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
