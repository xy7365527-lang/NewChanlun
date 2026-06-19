//! **T 算子操作层引擎**（每级别独立运转同一套逻辑——T 在操作层的自我复制）。
//!
//! ## 核心命题：操作层自我复制（aₙ=f(aₙ₋₁) 的操作投影）
//! T 算子在信号层是 `aₙ=f(aₙ₋₁)`（第65课）——所有级别共用同一个 `f`，唯一不同是 `a₀`。
//! 本引擎把这个不变性延伸到**操作层**：**每个级别独立运转完全相同的操作逻辑**，没有
//! root 方向、没有 core 核心仓、没有"最高级别给方向"。同一个函数 [`TPositionEngine::act`]
//! 对每个级别调用——这就是 T 的自我复制。
//!
//! ## 每个级别的操作逻辑（[`act`]，每级别逐字相同）
//! ```text
//! 该级别 k 卖点(Sell1/2/3) → 平本级别多头 + 次级别(k−1)做空
//! 该级别 k 买点(Buy1/2/3)  → 平本级别空头 + 次级别(k−1)做多
//! ```
//! 买卖点不分 type（type1/2/3 都是"该级别买/卖点"），多空完全对称（`act(k, want)`
//! 的 `want∈{Long,Short}` 是唯一参数，Buy→Long / Sell→Short）。
//!
//! ## 级别之间 = 递归嵌套（275号谱系：局部依赖原则）
//! - 本级别的一次操作（减仓做空）是**上级别仓位的一个组成部分**：ladder k 的多头是
//!   ladder k+1 买点通过"次级别做多"建立的；ladder k 卖点把它下沉到 k−1 做空。
//! - 上级别**不指挥**本级别——每个级别只看自己的 BSP，调用同一套 [`act`]。
//! - 全局仓位是局部操作**自然涌现**的结果（Σ_k layers[k]），不是预设的 core。
//! - 「附庸的附庸不是我的附庸」（275号）：资金沿级别接力下沉，每层只管直接次级别。
//!
//! ## 资金模型（用户伪代码留白的 units 语义——本引擎的设计决断）
//! 统一现金池 `free` + 每级别独立 [`Layer`]（复用 fugue_v3 会计原语，DRY）+ 三种动作：
//! 1. **入场**（现金→仓位）：当本级别无反向仓可平（无下沉量）**且全局空仓**（n_base≈0），
//!    次级别用 `free/c` 全压建仓。这是现金↔仓位的边界（free = 级别 ∞ 的现金态）。
//! 2. **守恒下沉**（级别间接力）：本级别平掉 m 后，把 m 在次级别反向开仓（穿手性缝）；
//!    Σ|units| 在这一对 reduce+add 中守恒（n_base 不变）。这是递归嵌套的资金形式。
//! 3. **全仓翻转**：次级别已持反向仓 → 先平再开（方向翻转）。
//! 杠杆防护：入场（free 注入）**只在全局空仓**，做空令 free 虚增也不会触发二次建仓。
//!
//! ### 入场层下界（有效域边界）
//! 次级别承接（建仓）仅当 `k > BASE_LADDER`（T level ≥ 1）触发——T level 0（ladder
//! `BASE_LADDER`）是最低操作级别，其 BSP 只平本级别仓、不下沉（伪代码 `if k>0`）。
//! ⟹ **行情至少涌现 T level 1（r*≥2）才会建仓**；纯 T level 0 行情零交易（声明=能力）。
//!
//! ## 复用 fugue_v3 会计原语 vs 不复用（no-patch-mentality：声明=能力）
//! **复用**（信号无关的通用会计/守卫）：`Layer` / `add_at` / `reduce_at`（NAV 中性双重
//! 会计）/ `nav` / `exposure` / `liquidate_*`（1x 逐仓边界 A）/ `record_trade`（trade11
//! 契约，ffi 依赖）/ `prove_nav_neutral` / `prove_recursive_consistency` / `prove_conservation`
//! / `count_chiral_violations`。
//! **不复用**（深绑 root/core/σ-塔，新设计无此概念，强塞 = 补丁）：`sink_chunk`/`recover_chunk`
//! （σ-不变 1/3 配额——新设计全额翻转，无 λ 配额）/ `prove_epsilon_symmetry`（依赖
//! root_direction）/ `prove_sigma_quota`（依赖 1/3 配额）/ `prove_no_double_act`（新设计
//! 允许同级别同 bar 翻转 = reduce+add 两次触及，互斥假设不成立）。
//!
//! ## 守卫语义诚实标注（formalization-validity-domain）
//! - `prove_nav_neutral`：**L0 强守恒**（add/reduce 同价 c 中性，真约束）。每 bar panic。
//! - `prove_recursive_consistency`：仓位层 ≥ FIRST_BSP_LADDER。T 持仓 ≥ BASE_LADDER ≥ 2，
//!   恒满足（防御性保留）。
//! - `prove_conservation`：**会计自洽**（n_base ≡ Σ|units|）。注意：新设计 n_base **不是**
//!   常量（入场/平仓改它）——这不是 Σ|units|=const 物理守恒（那是 fugue_v3 单核心仓性质），
//!   是"每次 add/reduce 同步更新 n_base"的会计跟踪一致性（漏更新必 panic）。
//! - `count_chiral_violations`：相邻级别同向计数（观测，非 panic）。新设计相邻级别可同向
//!   （接力链中段），合法。
//!
//! ## 认识论等级
//! - 操作层自我复制结构 / NAV 中性 / 会计自洽：**L0**。
//! - 哪条 word 此刻发声（BSP 是否 fire）：**L2 regime 依赖**。
//! - 回测 alpha：**L3**（真实数据，可否证）。

use crate::trading::types::{Polarity, LADDER_MOVE, MAX_LADDER};

use crate::fugue_v3::accounting::{
    accrue_held_bars, active_voice_count, add_at, exposure, flip, nav, reduce_at,
};
use crate::fugue_v3::cycle::{liquidate_long, liquidate_short};
use crate::fugue_v3::layer::{FugueResult, Layer};
use crate::fugue_v3::prove::{
    count_chiral_violations, prove_conservation, prove_nav_neutral, prove_recursive_consistency,
};
use crate::fugue_v3::{EQUITY_SAMPLE_BARS, INITIAL_CAPITAL, SUB_LIQ_FACTOR};

/// T level 0 在 ladder 空间的基准（= 走势级，move(L1)）。T level k ↦ ladder k + BASE_LADDER。
/// 也是**最低操作级别**：ladder == BASE_LADDER 的 BSP 不下沉（伪代码 `if k>0`）。
pub const BASE_LADDER: usize = LADDER_MOVE;

/// 本 bar 信号视图（从 T 全塔 BSP diff 构造，stream 层填充）。
///
/// 索引空间 = **ladder**（= T level + BASE_LADDER）。新设计**不区分** type1/2/3，也**不需要**
/// root_direction / 走势方向 / 锚 / θ 成本门——每个级别只看"本 bar 有无买/卖点"。
#[derive(Debug, Clone)]
pub struct TSignalView {
    /// 本 bar 该 ladder 是否新增**任意**买点（type1/2/3 不分）。
    pub buy: [bool; MAX_LADDER],
    /// 本 bar 该 ladder 是否新增**任意**卖点。
    pub sell: [bool; MAX_LADDER],
}

impl TSignalView {
    /// 空信号（无 BSP）——warm-up / 不重跑的 bar。
    pub fn empty() -> Self {
        TSignalView { buy: [false; MAX_LADDER], sell: [false; MAX_LADDER] }
    }
}

impl Default for TSignalView {
    fn default() -> Self {
        Self::empty()
    }
}

/// T 操作层引擎（每级别独立 Layer，统一现金池，无 root/core）。
pub struct TPositionEngine {
    res: FugueResult,
    /// 每级别独立净仓位（ladder 索引；全局仓位 = Σ layers，涌现非预设）。
    layers: Vec<Layer>,
    /// 统一现金池。
    free: f64,
    /// 当前总持仓基数 Σ|units|（会计自洽锚；入场/平仓改它，非物理守恒常量）。
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

    /// 把级别 k 的仓位**导向** `want` 方向（次级别承接 / 入场 / 全仓翻转的统一原语）。
    ///
    /// - 已是 `want`：对齐，不动。
    /// - 持反向：先平（reduce，"flip"），再按下方规则开仓。
    /// - 开仓量 `m`：`m_down>0` ⟹ 守恒下沉量（级别接力）；否则全局空仓 ⟹ `free/c` 入场；
    ///   否则 0（不凭空加仓，杠杆防护）。
    fn direct_to(&mut self, target: usize, want: Polarity, m_down: f64, bar: i64, c: f64) {
        let cur = self.layers[target];
        if cur.units > 1e-12 && cur.direction == want {
            return; // 已对齐
        }
        if cur.units > 1e-12 {
            // 持反向 → 先平（全仓翻转的平腿）。
            let u = cur.units;
            reduce_at(&mut self.layers, target, u, &mut self.free, c, bar, &mut self.res, "flip");
            self.n_base -= u;
        }
        // 开仓量：守恒下沉量优先；否则仅全局空仓时用 free 入场。
        let m = if m_down > 1e-12 {
            m_down
        } else if self.global_flat() && self.free > 0.0 {
            self.free / c
        } else {
            0.0
        };
        if m > 1e-12 && c > 0.0 && m.is_finite() {
            add_at(&mut self.layers, target, m, want, &mut self.free, c, bar, &mut self.res);
            self.n_base += m;
        }
    }

    /// **一个级别的操作**（每级别逐字相同——T 操作层自我复制的代码体现）。
    ///
    /// `want`：该级别买卖点指向的方向（Buy→Long / Sell→Short）。
    /// 1. 平本级别**反向**仓（Buy 平空 / Sell 平多）；
    /// 2. 次级别（k−1）导向 `want`（仅 k>BASE_LADDER；T level 0 无次级别，只平不下沉）。
    fn act(&mut self, k: usize, want: Polarity, bar: i64, c: f64) {
        let opp = flip(want);
        // 1. 平本级别反向仓 → 得下沉量 m_down。
        let mut m_down = 0.0;
        if self.layers[k].units > 1e-12 && self.layers[k].direction == opp {
            m_down = self.layers[k].units;
            reduce_at(&mut self.layers, k, m_down, &mut self.free, c, bar, &mut self.res, "close");
            self.n_base -= m_down;
        }
        // 2. 次级别承接（仅有次级别时；T level 0 = BASE_LADDER 无次级别，伪代码 `if k>0`）。
        if k > BASE_LADDER {
            self.direct_to(k - 1, want, m_down, bar, c);
        }
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
    }

    /// 单 bar 操作步进：A 边界强平 → 逐级别（ladder 降序）独立 [`act`]。
    pub fn step(&mut self, view: &TSignalView, bar: i64, price: f64) {
        let c = price;
        self.last_close = c;
        let nav_pre = nav(&self.layers, self.free, c);

        // ── A. 边界算子：1x 逐仓强平（多空对称；无 core 豁免——新设计无 core）──
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
            }
        }

        // ── 逐级别操作（ladder 降序：高级别先承接下沉，同 bar 内确定性顺序）──
        // 每个级别完全相同的 act——T 操作层自我复制。同 ladder 先 sell 后 buy（确定性）。
        for k in (0..MAX_LADDER).rev() {
            if view.sell[k] {
                self.act(k, Polarity::Short, bar, c);
            }
            if view.buy[k] {
                self.act(k, Polarity::Long, bar, c);
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
        }
    }

    /// 收尾（末 bar equity 补采样 + 清仓到现金）。`last_bar=None`（零 bar）⇒ 仅设 final_nav=free。
    pub fn finish(&mut self, last_bar: Option<i64>) {
        if let Some(lb) = last_bar {
            let c_last = self.last_close;
            if lb % EQUITY_SAMPLE_BARS != 0 {
                self.res.equity.push((lb, nav(&self.layers, self.free, c_last)));
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

    /// 单 ladder 买点视图。
    fn buy_view(k: usize) -> TSignalView {
        let mut v = TSignalView::empty();
        v.buy[k] = true;
        v
    }

    /// 单 ladder 卖点视图。
    fn sell_view(k: usize) -> TSignalView {
        let mut v = TSignalView::empty();
        v.sell[k] = true;
        v
    }

    #[test]
    fn 次级别承接入场全仓多头nav中性() {
        let mut eng = TPositionEngine::new();
        // ladder4(T level1) 买点 → 次级别 ladder3 做多（入场全压）。
        eng.step(&buy_view(4), 0, 100.0);
        let (navv, lu, su, _) = eng.snapshot();
        // 满仓 total = 100000/100 = 1000 units 全 Long，落在次级别 ladder3。
        assert!((lu - 1000.0).abs() < 1e-6, "满仓 1000 units，得 {lu}");
        assert_eq!(su, 0.0, "建仓后无空头");
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "入场 NAV 中性");
        assert!((eng.layers[3].units - 1000.0).abs() < 1e-6, "仓位在 ladder3（次级别）");
    }

    #[test]
    fn t_level0信号不建仓() {
        let mut eng = TPositionEngine::new();
        // ladder3(T level0) 买点 → 无次级别（k==BASE_LADDER），不下沉，不建仓。
        eng.step(&buy_view(3), 0, 100.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "T level0 不建仓");
        assert_eq!(su, 0.0);
        assert!(eng.global_flat(), "全局空仓");
    }

    #[test]
    fn 次级别卖点平多盈利() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(4), 0, 100.0); // ladder3 Long 1000@100
        // ladder3(=次级别本身) 卖点 → 平本级别多（不下沉，ladder3==BASE_LADDER）。
        eng.step(&sell_view(3), 10, 120.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "平多后空仓");
        assert_eq!(su, 0.0, "ladder3 卖点不开空（T level0 不下沉）");
        eng.finish(Some(10));
        // 1000 units 100→120 ⇒ free = 1000×120 = 120000。
        assert!((eng.result().final_nav - 120_000.0).abs() < 1e-3,
                "平多 final_nav=120000，得 {}", eng.result().final_nav);
    }

    #[test]
    fn 空输入零操作守恒() {
        let mut eng = TPositionEngine::new();
        for i in 0..5 {
            eng.step(&TSignalView::empty(), i, 100.0);
        }
        eng.finish(Some(4));
        assert!(eng.result().trades.is_empty(), "无信号无 trade");
        assert!((eng.result().final_nav - INITIAL_CAPITAL).abs() < 1e-6,
                "无操作 final_nav=初始资本");
    }

    #[test]
    fn 高级别卖点把次级别多翻空全仓翻转() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(4), 0, 100.0); // ladder3 Long 1000@100
        // ladder4 卖点 → 平本级别(ladder4 无仓) + 次级别 ladder3 做空。
        // ladder3 是 Long(反向) → 先平多(120) 再开空 → 全仓翻转。
        eng.step(&sell_view(4), 10, 120.0);
        let (navv, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "翻转后无多头");
        // 平多 1000@120 free=120000，全压做空 120000/120=1000 股@120。
        assert!((su - 1000.0).abs() < 1e-6, "次级别 1000 Short，得 {su}");
        assert!((navv - 120_000.0).abs() < 1e-4, "翻转 NAV 中性（=平多后净值）");
        assert!((eng.layers[3].units - 1000.0).abs() < 1e-6);
        assert_eq!(eng.layers[3].direction, Polarity::Short);
    }

    #[test]
    fn 守恒下沉级别接力() {
        let mut eng = TPositionEngine::new();
        // ladder5 买点 → 次级别 ladder4 做多 1000@100。
        eng.step(&buy_view(5), 0, 100.0);
        assert!((eng.layers[4].units - 1000.0).abs() < 1e-6, "ladder4 Long 1000");
        // ladder5 卖点 → 平本级别(ladder5 无仓) + 次级别 ladder4 翻空（全仓翻转，入场分支）。
        // 但要测守恒下沉：用 ladder4 卖点 → 平本级别 ladder4 多(m_down=1000) + 下沉 ladder3 做空(守恒)。
        eng.step(&sell_view(4), 10, 110.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "ladder4 多被平");
        // 守恒下沉：m_down=1000 从 ladder4 平掉，在 ladder3 开空 1000。
        assert!((su - 1000.0).abs() < 1e-6, "ladder3 守恒下沉 1000 Short，得 {su}");
        assert!((eng.layers[3].units - 1000.0).abs() < 1e-6, "下沉到 ladder3");
        assert_eq!(eng.layers[3].direction, Polarity::Short);
        // Σ|units| 守恒（下沉不改 n_base）。
        assert!((eng.n_base - 1000.0).abs() < 1e-6, "守恒下沉 n_base 不变");
    }

    #[test]
    fn 次级别买点回补空头降价盈利() {
        let mut eng = TPositionEngine::new();
        eng.step(&buy_view(4), 0, 100.0); // ladder3 Long 1000@100
        eng.step(&sell_view(4), 10, 110.0); // 翻转 → ladder3 Short 1000@110
        assert_eq!(eng.layers[3].direction, Polarity::Short);
        // ladder4 买点 → 次级别 ladder3 做多：ladder3 Short(反向) → 平空@105 + 重开多。
        eng.step(&buy_view(4), 20, 105.0);
        let (_, lu, su, _) = eng.snapshot();
        assert_eq!(su, 0.0, "空头被回补");
        assert!(lu > 0.0, "回补后转多头");
        // 空头 110→105 cover 盈利：(110−105)×1000 = 5000 进 free 再投入。
        let mob: f64 = eng.result().mobile_realized_pnl_by_ladder.iter().sum();
        assert!(mob > 0.0, "做空降价回补盈利，得 {mob}");
    }

    #[test]
    fn 向下入场做空降价平仓盈利() {
        let mut eng = TPositionEngine::new();
        // ladder4 卖点（全局空仓）→ 次级别 ladder3 入场做空 1000@100。
        eng.step(&sell_view(4), 0, 100.0);
        let (navv, lu, su, _) = eng.snapshot();
        assert_eq!(lu, 0.0, "做空无多头");
        assert!((su - 1000.0).abs() < 1e-6, "入场满仓 1000 Short，得 {su}");
        assert!((navv - INITIAL_CAPITAL).abs() < 1e-4, "开空 NAV 中性");
        // ladder4 买点 → 次级别 ladder3 回补（平空@80）。
        eng.step(&buy_view(4), 10, 80.0);
        eng.finish(Some(10));
        // 开空@100 平空@80 ⇒ 赚 1000×20 = 20000 ⇒ final_nav = 120000。
        assert!((eng.result().final_nav - 120_000.0).abs() < 1e-3,
                "做空降价平仓 final_nav=120000，得 {}", eng.result().final_nav);
    }

    #[test]
    fn 空头强平边界() {
        let mut eng = TPositionEngine::new();
        eng.step(&sell_view(4), 0, 100.0); // ladder3 Short 1000@100，basis=100
        // 价格涨到 2×basis=200 ⇒ 1x 逐仓空头强平。
        eng.step(&TSignalView::empty(), 10, 200.0);
        let (_, _, su, _) = eng.snapshot();
        assert_eq!(su, 0.0, "空头被强平");
        assert_eq!(eng.res.n_liquidations_by_ladder[3], 1, "记一次强平");
        assert!(eng.global_flat(), "强平后全局空仓");
    }

    #[test]
    fn 仓位层合法() {
        use crate::trading::types::FIRST_BSP_LADDER;
        // prove_recursive_consistency：仓位层 ≥ FIRST_BSP_LADDER。
        // T 入场层 = BASE_LADDER（次级别承接最低落点），BASE_LADDER ≥ FIRST_BSP_LADDER。
        assert!(BASE_LADDER >= FIRST_BSP_LADDER);
    }
}
