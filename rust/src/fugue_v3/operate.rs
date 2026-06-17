//! **H¹ 操作轴实装**（OperateAxis）：**D∞ word 处理器**——引擎不预设循环模式。
//!
//! 用户裁决（2026-06-17）：操作不是不变量，操作是**路径（D∞ 的 word）**。引擎只有 h/τ 两个原子，
//! 每 bar 每级别读信号决定施加哪个：nf_sell[k]→τ 下沉（σ⁻¹∘τ，子仓落 k−1）/
//! nf_buy[k−1]→τ 升回（σ∘τ，k−1 子仓回 k；级别锚定修复见 D 步骤）/ 无信号→h（仓位不变）。
//! 「四步循环」是 sink 后接 recover 的**涌现序列**，不是状态机。
//!
//! ## 核心仓 H⁰ 涌现（非硬编码字段）
//! 每次 τ 只下沉 f=1/λ（1/3），顶层级别保留多数 ⟹ σ-塔自然分布：核心仓 = 顶层稀疏下沉的多数
//! （f∝λ⁻ᴷ 罕见）；机动仓 = 内层频繁循环（f∝λ⁻ᵏ，T50）。无 core_units/mobile_units 硬拆分。
//!
//! ## 优先序 A→C→D→E→F（信号驱动，每个是 Σ 原子的合法组合，operation_route_exhaustion §7.4）
//! A 强平（边界算子）→ σ-ascend（OP_ASCEND relabel）→ C（OP_FLIP/clear，τ@顶）→
//! D recover（OP_REBUY，σ∘τ）→ E sink（OP_SHORT/REDUCE，σ⁻¹∘τ）→ F（OP_ENTER，β⁺ 建仓）。
//!
//! ## 双投影（reinterp §1/§5）
//! τ 同时改 direction（操作投影，ε 翻转）+ units（会计投影，1/3 相邻转移）。M ⊥ ε。
//! 会计在 `accounting::{reduce_at, add_at}`，按 direction 镜像（多/空双重性）。

use crate::buysellpoint::Side;
use crate::spiral::signal::{prove_chain, prove_t52_gauge_fix};
use crate::stroke::Direction;
use crate::trading::types::{Polarity, MAX_LADDER};

use super::accounting::{
    accrue_held_bars, active_voice_count, add_at, exposure, nav, reduce_at,
};
use super::axis::{MorphologyAxis, ObserveAxis, OperateAxis, StepOutcome};
use super::cycle::{liquidate_long, liquidate_short, recover_chunk, sink_chunk};
use super::layer::{FugueResult, Layer};
use super::prove::{
    count_chiral_violations, prove_conservation, prove_epsilon_symmetry, prove_nav_neutral,
    prove_no_double_act, prove_recursive_consistency,
};
use super::{EQUITY_SAMPLE_BARS, INITIAL_CAPITAL, PENDING_LO, SUB_COST_K, SUB_FRICTION_RT, SUB_LIQ_FACTOR};

/// 核心仓涌现归属级别 E*（T5/A5 第20环）：核心仓操作级别随走势向更高级别发展向上单调生长——
/// 会计重组（relabel），不触发物理交易。通过 MorphologyAxis 读方向/锚/上界。
/// `core_polarity`：根极性（Long→向上涌现检查 Up 方向；Short→向下涌现检查 Down 方向）。
fn core_emergent_ladder(core_ladder: usize, core_entry_bar: i64, h0: &dyn MorphologyAxis, core_polarity: Polarity) -> usize {
    let ceiling = h0.emergent_ceiling();
    let expected_dir = match core_polarity {
        Polarity::Long => Direction::Up,
        Polarity::Short => Direction::Down,
    };
    let mut lad = core_ladder;
    while lad + 1 < ceiling
        && h0.direction(lad + 1) == Some(expected_dir)
        && h0.anchor(lad + 1) >= core_entry_bar
    {
        lad += 1;
    }
    assert!(lad >= core_ladder, "A5(T30) 违反：核心仓涌现层 {lad} < 入场层 {core_ladder}");
    lad
}

/// H¹ 操作引擎（层结构状态机 = D∞ word 处理器）。
pub struct OperateEngine {
    res: FugueResult,
    /// 层数组（每 ladder 一个净仓位 Layer{direction, units, basis}）。
    layers: Vec<Layer>,
    /// 总单位 Σ|units|（守恒 Casimir，T48）。
    n_base: f64,
    /// 自由现金。
    free: f64,
    /// 核心仓所在层（顶层 Long/Short；σ-ascend 单调上移）。
    core_ladder: usize,
    /// 核心仓入场 bar（σ-ascend 涌现读数锚）。
    core_entry_bar: i64,
    /// 根极性 ε（Long=+1 / Short=−1）：D∞ 的 ε 维度——决定 E/D/C 的信号分派方向。
    /// 仅在 has_position() 时有意义；F 建仓时写入，C 清仓后无效（n_base=0 保证不读）。
    core_polarity: Polarity,
    last_close: f64,
    max_concurrent_seen: usize,
}

impl OperateEngine {
    pub fn new() -> Self {
        let layers = (0..MAX_LADDER).map(Layer::idle).collect();
        OperateEngine {
            res: FugueResult::default(),
            layers,
            n_base: 0.0,
            free: INITIAL_CAPITAL,
            core_ladder: 0,
            core_entry_bar: -1,
            core_polarity: Polarity::Long, // 无仓位时无意义，F 建仓时覆写
            last_close: f64::NAN,
            max_concurrent_seen: 0,
        }
    }

    fn has_position(&self) -> bool {
        self.n_base > 1e-12
    }

    /// 把整个仓位清到现金（C 顶背驰清 / eod）：reduce_at 每个占用级别（按 direction 卖/cover）。
    fn clear_to_cash(&mut self, bar: i64, c: f64, reason: &'static str, acted: &mut Vec<usize>) {
        for k in 0..self.layers.len() {
            if self.layers[k].units > 0.0 {
                let u = self.layers[k].units;
                reduce_at(&mut self.layers, k, u, &mut self.free, c, bar, &mut self.res, reason);
                if !acted.contains(&k) {
                    acted.push(k);
                }
            }
        }
        self.n_base = 0.0;
        self.core_ladder = 0;
        self.core_entry_bar = -1;
    }

    /// 收尾（末 bar equity 补采样 + 清仓到现金）。`last_bar=None`（零 bar）⇒ 仅设 final_nav=free。
    pub fn finish(&mut self, last_bar: Option<i64>) {
        if let Some(lb) = last_bar {
            let c_last = self.last_close;
            if lb % EQUITY_SAMPLE_BARS != 0 {
                self.res.equity.push((lb, nav(&self.layers, self.free, c_last)));
            }
            if self.has_position() {
                let mut acted = Vec::new();
                self.clear_to_cash(lb, c_last, "eod", &mut acted);
            }
        }
        self.res.final_nav = self.free;
        self.res.max_concurrent_voices = self.max_concurrent_seen as u64;
    }

    pub fn result(&self) -> &FugueResult {
        &self.res
    }
    pub fn result_mut(&mut self) -> &mut FugueResult {
        &mut self.res
    }

    /// 状态快照: (nav, long_units, short_units, n_active_voices)。
    pub fn snapshot(&self) -> (f64, f64, f64, usize) {
        let c = if self.last_close.is_finite() { self.last_close } else { 0.0 };
        let navv = nav(&self.layers, self.free, c);
        let (lu, su) = exposure(&self.layers);
        (navv, lu, su, active_voice_count(&self.layers))
    }

    /// 测试只读访问器。
    #[cfg(test)]
    pub(crate) fn total_long_at_entry(&self) -> f64 {
        self.layers.iter().filter(|l| l.direction == Polarity::Long).map(|l| l.units).sum()
    }
}

impl Default for OperateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl OperateAxis for OperateEngine {
    fn step(
        &mut self,
        h0: &dyn MorphologyAxis,
        obs: &dyn ObserveAxis,
        bar: i64,
        price: f64,
    ) -> StepOutcome {
        let c = price;
        self.last_close = c;
        let nav_pre = nav(&self.layers, self.free, c);
        let mut acted: Vec<usize> = Vec::new();
        let mut touched = [false; MAX_LADDER];
        let mut outcome = StepOutcome::default();

        // ── A. 边界算子：子仓强平（1x 逐仓；ε 对称：空头子仓 c≥2×basis / 多头子仓 c≤basis/2）──
        // 只强平非根层（核心仓由 C 步骤处理），子仓按极性对称判断水下深度。
        for k in 0..self.layers.len() {
            let l = &self.layers[k];
            if l.units <= 0.0 || k == self.core_ladder {
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
                touched[k] = true;
                acted.push(k);
            }
        }

        // ── σ-ascend（OP_ASCEND relabel）：核心仓涌现升级（units/nav 不变）──
        if self.has_position() {
            let re = core_emergent_ladder(self.core_ladder, self.core_entry_bar, h0, self.core_polarity);
            if re > self.core_ladder && self.layers[self.core_ladder].units > 0.0 && self.layers[re].units == 0.0 {
                let core = self.layers[self.core_ladder];
                self.layers[re] = Layer { ladder: re, ..core };
                self.layers[self.core_ladder] = Layer::idle(self.core_ladder);
                self.core_ladder = re;
            }
        }

        // ── C. 顶背驰整体清（ε 对称：Long root 检查 sell_source；Short root 检查 buy_source）──
        // Long root：sell_source≥core_ladder ∧ sell1 ⇒ 清仓。
        // Short root：buy_source≥core_ladder ∧ buy1 ⇒ 清仓（缠论卖点平空同时做多的平空端）。
        let mut cleared = false;
        if self.has_position() {
            match self.core_polarity {
                Polarity::Long => {
                    if let Some(s) = obs.sell_source() {
                        if s >= self.core_ladder && h0.sell1(s) {
                            prove_chain(obs.located_sell_chain(), Side::Sell, s, bar, "C-clear-long");
                            prove_t52_gauge_fix(obs.located_sell_chain(), s, bar, "C-clear-long");
                            self.clear_to_cash(bar, c, "core_clear", &mut acted);
                            self.res.n_core_clears_by_ladder[s] += 1;
                            outcome.cleared_sell_source = Some(s);
                            cleared = true;
                        }
                    }
                }
                Polarity::Short => {
                    if let Some(s) = obs.buy_source() {
                        if s >= self.core_ladder && h0.buy1(s) {
                            prove_chain(obs.located_buy_chain(), Side::Buy, s, bar, "C-clear-short");
                            prove_t52_gauge_fix(obs.located_buy_chain(), s, bar, "C-clear-short");
                            self.clear_to_cash(bar, c, "core_clear_short", &mut acted);
                            self.res.n_core_clears_by_ladder[s] += 1;
                            outcome.cleared_buy_source = Some(s);
                            cleared = true;
                        }
                    }
                }
            }
        }

        // ── D. recover（OP_REBUY，σ∘τ）：ε 对称信号分派 + **级别锚定修复** ──
        //
        // 级别锚定修复（fugue_v3_sink_recover_level_anchoring.md + 用户裁决 2026-06-17）：
        // recover 平的是 sub=k−1 层的子仓（sink 把它放在这里），触发应用**次级别 k−1 的买卖点**，
        // 非本级别 k。子仓押注的是 k−1 级别一段走势，该走势结束 = k−1 级别 BSP。
        // 旧 nf_buy[k]（高一级）在强趋势中稀疏一个数量级 ⟹ recover 死锁 ⟹ 子仓僵尸化
        // （ES/GC/QQQ recover/sink=0，持有 100 万+ bar，牛市浮亏 100%+，吃尽反向亏损）。
        // 缠论次级别降成本（17/26 课）：回补端用次级别买卖点，与减仓端同为次级别。
        //
        // 不对称（sink 仍用 nf_sell[k]，见 E 步骤）：sink 与 recover 是**两个不同主体**的操作——
        // sink = 核心仓@k 高抛（看本级别顶背驰，缠师"大级别卖点附近高抛"，合理）；
        // recover = 子仓@k−1 回补（看次级别 k−1 买卖点）。主体级别不同，故锚定级别不同。
        //
        // Long root：nf_buy[k−1] fire ⇒ 从 Short@k−1 升回 Long@k（期望子层=Short）。
        // Short root：nf_sell[k−1] fire ⇒ 从 Long@k−1 升回 Short@k（期望子层=Long）。
        if !cleared {
            let parent_dir = self.core_polarity; // recover 期望子层 = flip(parent_dir)
            for k in PENDING_LO..MAX_LADDER {
                let sub = k - 1;
                if self.layers[sub].units <= 0.0 || touched[k] || touched[sub] {
                    continue;
                }
                // 子仓物理在 sub=k−1 层 ⟹ 用**次级别 k−1** 买卖点触发回补（非本级别 k）。
                let signal = match parent_dir {
                    Polarity::Long => obs.nf_buy(sub),
                    Polarity::Short => obs.nf_sell(sub),
                };
                if signal.is_some() && recover_chunk(&mut self.layers, k, parent_dir, &mut self.free, c, bar, &mut self.res) {
                    touched[k] = true;
                    touched[sub] = true;
                    acted.push(k);
                }
            }
        }

        // ── E. sink（OP_SHORT/REDUCE，σ⁻¹∘τ）：ε 对称信号分派 + 成本门——
        // Long root：nf_sell[k] fire ⇒ Long@k 下沉 ⟶ Short@k−1（ε 翻转）。
        // Short root：nf_buy[k] fire ⇒ Short@k 下沉 ⟶ Long@k−1（ε 翻转）。
        if !cleared && self.has_position() {
            for k in PENDING_LO..MAX_LADDER {
                let sub = k - 1;
                if self.layers[k].units <= 0.0 || touched[k] || touched[sub] {
                    continue;
                }
                let signal = match self.core_polarity {
                    Polarity::Long => obs.nf_sell(k),
                    Polarity::Short => obs.nf_buy(k),
                };
                if signal.is_none() {
                    continue;
                }
                // 成本门（N4）：次级别 k−1 势幅度 θ。None=势不可测 / θ<K×friction=势<成本 ⇒ 不下沉。
                match h0.theta(sub) {
                    None => {
                        self.res.n_noref_rejects_by_ladder[sub] += 1;
                    }
                    Some(tq) if tq < SUB_COST_K * SUB_FRICTION_RT => {
                        self.res.n_cost_rejects_by_ladder[sub] += 1;
                    }
                    Some(_) => {
                        if sink_chunk(&mut self.layers, k, &mut self.free, c, bar, &mut self.res) {
                            touched[k] = true;
                            touched[sub] = true;
                            acted.push(k);
                        }
                    }
                }
            }
        }

        // ── F. 建仓（OP_ENTER，β±）：F 入场方向 = f(root_direction)，BSP 信号决定时机 ──
        //
        // 修改（emergent_level_direction.md §5.4 + 用户裁决 2026-06-17）：
        // 根方向是 H⁰ 涌现属性——从已涌现最高级别走势方向读取，不从 BSP 信号类型推断。
        // 之前 sell_source→Short 在强 Up regime 中频繁踏空（ES −213% / GC −122%）——
        // sell_source 在 Up regime 中是次级别顶背驰（走势结束=平多窗口），不是翻空信号。
        //
        // 缠论操盘核心：操作必须在 root 方向上（顺势）。BSP 信号只决定**时机**，方向由 root 决定。
        //
        // 入场矩阵（root_direction, 触发信号）：
        // (Up,   buy_source)  → Long（顺势底入场：高级别上涨+次级别底买点）
        // (Down, sell_source) → Short（顺势顶入场：高级别下跌+次级别顶卖点）
        // (Up,   sell_source) → 不入场（强 Up regime 中的次级别顶=操作窗口而非翻空，避免踏空）
        // (Down, buy_source)  → 不入场（强 Down regime 中的次级别底=操作窗口而非翻多，避免抄底）
        // (None, _)           → 不入场（root 方向未涌现，行情启动前）
        //
        // BSP 端点方向断言（缠论 type1 BSP 必然性）：
        // - Up root → 等 buy_source → s 级别方向必 Down（type1 buy 在 s 级别下跌末端）
        // - Down root → 等 sell_source → s 级别方向必 Up（type1 sell 在 s 级别上涨末端）
        if !cleared && !self.has_position() {
            if let Some(root_dir) = h0.root_direction() {
                // root 方向决定入场极性 + 时机信号侧（source 只定时机/层级，不定方向）。
                let (polarity, source_opt, source_side, reason) = match root_dir {
                    Direction::Up => (Polarity::Long, obs.buy_source(), Side::Buy, "F-entry-long"),
                    Direction::Down => (Polarity::Short, obs.sell_source(), Side::Sell, "F-entry-short"),
                };
                if let Some(s) = source_opt {
                    let bsp_fire = match source_side {
                        Side::Buy => h0.buy1(s),
                        Side::Sell => h0.sell1(s),
                    };
                    // 顺势确认门（nf 语义 prove_f_source_direction）：入场层 s 涌现方向必 == root。
                    // source@s ⟹ s 曾向心 confirm（nf_buy→Up / nf_sell→Down），通常同向 root；
                    // 陈旧 located（s 买点后又出卖点、价未破极值）使 emergent_dir[s] 暂逆 ⟹ 不入场
                    // （等次级别真信号），非 panic——nf 语义下 source 级别方向非无条件必然（§5.4）。
                    let aligned = h0.direction(s) == Some(root_dir);
                    if bsp_fire && aligned && self.free > 0.0 {
                        let total = self.free / c;
                        if total > 0.0 && total.is_finite() {
                            let chain = match source_side {
                                Side::Buy => obs.located_buy_chain(),
                                Side::Sell => obs.located_sell_chain(),
                            };
                            prove_chain(chain, source_side, s, bar, reason);
                            prove_t52_gauge_fix(chain, s, bar, reason);
                            add_at(&mut self.layers, s, total, polarity, &mut self.free, c, bar, &mut self.res);
                            self.core_ladder = s;
                            self.core_entry_bar = bar;
                            self.core_polarity = polarity;
                            self.n_base = total;
                            self.res.n_entries_by_ladder[s] += 1;
                            match polarity {
                                Polarity::Long => outcome.entered_buy_source = Some(s),
                                Polarity::Short => outcome.entered_sell_source = Some(s),
                            }
                            prove_epsilon_symmetry(self.core_polarity, root_dir, bar);
                        }
                    }
                }
                // BSP 时机不匹配 root 方向（如 root=Up 但只有 sell_source）⇒ 不入场（避免逆势踏空）。
            }
            // root_direction=None ⇒ 不入场（行情未启动）。
        }

        // ── 必然性运行时证明（每 bar；violation = panic）──
        prove_no_double_act(&acted, bar);
        prove_recursive_consistency(&self.layers, bar);
        // T24 手性交替：相邻同向（建仓阶段未出卖点）合法，非 panic 不变量 ⟹ 观测计数（137号 make-decision-observable）。
        let chiral_same_dir = count_chiral_violations(&self.layers) as u64;
        self.res.max_chiral_same_dir = self.res.max_chiral_same_dir.max(chiral_same_dir);
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
        outcome
    }

    fn layers(&self) -> &[Layer] {
        &self.layers
    }
}
