//! GUARD-ROLE: organic-fugue-v2-trading-layer——名分：现役（详见 `trading/mod.rs` 头部 GUARD-ROLE 块，#763 C7-E4 核定；零删除/零移入 legacy/）。
//!
//! OrganicLedger — 共享仓位 + 三类腿（main/osc/rev）并发短差账本。
//!
//! 守恒律算术与 Python `organic_fugue.OrganicLedger`（← `_SharedFugue` ←
//! `ShortDiffCycle`）**逐字一致**——这是 V0≡P5 逐位等价的账本前提：
//!   - 股数守恒（ShareConserving）：profit=(sell−buy)×shares 落袋 →
//!     cost_basis −= profit/total_shares → ≤0 单向相变 EarningShares。
//!   - 金额守恒（AmountConserving）：total_shares += shares×sell/buy。
//! 浮点运算顺序逐句对应（T1 陷阱：腿按插入序 Vec 存储，_close 清腿顺序 =
//! Python dict 插入序——cost_basis 串行递推对腿序敏感）。
//!
//! ## T5 语义疑点的处置（v1R §2.6 设计注记 + 实证补注）
//! Python 在 earning=True 后闭合 ShareConserving 旧腿且 profit<0 时 cost_basis
//! 会回正而 earning 不回退；`LedgerPhase` 单向相变不可表示此状态。在册实证：
//! 全部已观测数据 earning 触发率=0 ⇒ 该路径空有效域。本实现取严格形式
//! （EarningShares 下 ShareConserving 亏损闭合不改写成本），并以
//! `n_t5_shareconserving_after_earning` 计数器使该路径**可观测**——若计数 >0
//! 且与 Python 对账分歧，这是定义冲突（守恒律相变可逆性），走矛盾上浮，
//! 不得为 bit-exact 复刻模糊语义（no-workaround）。

use super::types::*;

/// 短差循环方向（递归赋格符号交替塔，2026-06-11 任务）。
/// Short = 先卖后买（P5/REV 在册全部腿）；Long = 先买后卖（奇数深度子腿：
/// 反向走势窗口内的反弹段操作）。profit 公式两侧同为 (sell−buy)×shares。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSide {
    Short,
    Long,
}

/// 开放短差循环（`ShortDiffCycle` is_open=True 的类型化）。
/// open_price：Short 腿 = 卖出价（旧 sell_price 语义不变）；Long 腿 = 买入价。
#[derive(Debug, Clone, Copy)]
pub struct OpenCycle {
    pub shares: f64,
    pub open_price: f64,
}

/// 已闭合短差循环。profit() 只在此类型上存在——
/// Python "if self.is_open: return 0.0" 分支不可表示。
#[derive(Debug, Clone, Copy)]
pub struct ClosedCycle {
    pub shares: f64,
    pub sell_price: f64,
    pub buy_price: f64,
}

impl ClosedCycle {
    /// 与 Python `ShortDiffCycle.profit`：(sell − buy) × shares。
    pub fn profit(self) -> f64 {
        (self.sell_price - self.buy_price) * self.shares
    }
}

/// 开放腿记录：循环 + open 时刻冻结的守恒律 + 锚点 + 方向。
#[derive(Debug, Clone, Copy)]
pub struct OpenLeg {
    pub cycle: OpenCycle,
    /// open 时刻的账本阶段（Python `was_earning`），close 时 match 穷举。
    pub law: ConservationLaw,
    pub anchor: LegAnchor,
    /// 循环方向（在册路径恒 Short；Long 仅 rev_sub_depth ≥ 1 子腿）。
    pub side: DiffSide,
}

/// 腿 trace 记录（diag 模式；Python trace dict 12 字段逐一对应）。
#[derive(Debug, Clone)]
pub struct LegTrace {
    pub slot: SlotKey,
    pub sell_bar: i64,
    pub sell_price: f64,
    pub buy_bar: i64,
    pub buy_price: f64,
    pub shares: f64,
    pub diff: f64,
    pub profit: f64,
    pub was_earning: bool,
    pub shares_delta: f64,
    pub cost_basis_before: f64,
    pub cost_basis_after: f64,
    /// H3 级别上移腿（AnchorKind::OscUp）——diag 导出时 leg_kind 投影为
    /// "osc_up"（回测按腿分解上移腿盈亏的数据基础）。非 H3 变体恒 false
    /// ⇒ 导出逐位不变（O0≡P5 零接触面）。
    pub upshift: bool,
}

/// 共享仓位账本。字段对应 Python `OrganicLedger`（cost_basis+earning 折叠为
/// phase；Python 的 entry_price 字段无读取方，不复制——runner 自持 entry_price）。
#[derive(Debug)]
pub struct OrganicLedger {
    pub total_shares: f64,
    pub phase: LedgerPhase,
    pub level_frac: f64,
    pub cumulative_recovered: f64,
    /// 未部署现金（master 入场侧递归建仓，entry_mode=Recursive）。
    /// Full 模式恒 0.0——close 的 total_value 加 0.0 位等价（O0≡P5 零接触）。
    /// 资金守恒：部署现金 + 本值 = INITIAL_CAPITAL（add_entry_tranche 单向流出）。
    pub undeployed_cash: f64,
    /// 开放腿，插入序存储（bit-exact 硬前提，模块 docstring）。
    legs: Vec<(SlotKey, OpenLeg)>,
    /// 闭合腿（槽键 + 循环），报告归因用。
    pub completed: Vec<(SlotKey, ClosedCycle)>,
    pub n_open_rejects_zero: u32,
    /// T5 疑点可观测面（模块 docstring）。
    pub n_t5_shareconserving_after_earning: u32,
    /// diag 模式腿 trace（None = 不记录）。
    pub trace: Option<Vec<LegTrace>>,
    /// 开腿 bar 记录（Python `_diff_open`：key → (bar, sell_price)）。
    diff_open: Vec<(SlotKey, i64)>,
}

impl OrganicLedger {
    pub fn new(entry_price: f64, total_shares: f64, level_frac: f64, with_trace: bool) -> Self {
        OrganicLedger {
            total_shares,
            phase: LedgerPhase::CostReduction {
                cost_basis: entry_price,
            },
            level_frac,
            cumulative_recovered: 0.0,
            undeployed_cash: 0.0,
            legs: Vec::new(),
            completed: Vec::new(),
            n_open_rejects_zero: 0,
            n_t5_shareconserving_after_earning: 0,
            trace: if with_trace { Some(Vec::new()) } else { None },
            diff_open: Vec::new(),
        }
    }

    /// 部分入场账本（master 入场侧递归建仓）：deployed_cash 按入场价买入，
    /// reserve_cash 保留为未部署现金（计入 close 的 total_value）。
    pub fn with_reserve(
        entry_price: f64,
        deployed_cash: f64,
        reserve_cash: f64,
        level_frac: f64,
        with_trace: bool,
    ) -> Self {
        let mut led = OrganicLedger::new(
            entry_price,
            deployed_cash / entry_price,
            level_frac,
            with_trace,
        );
        led.undeployed_cash = reserve_cash;
        led
    }

    /// 追加入场 tranche（master 递归建仓）：cash 按 price 买入，cost_basis
    /// 更新为加权均价 (cb×S_old + cash)/S_new。EarningShares 拒绝——成本
    /// 概念已不存在，加权均价算术未定义（调用方计数，open_sub 同先例）。
    /// cash 超出未部署现金拒绝（资金守恒：调用方负责 min 钳制）。
    pub fn add_entry_tranche(&mut self, cash: f64, price: f64) -> bool {
        if cash <= 0.0 || price <= 0.0 || cash > self.undeployed_cash {
            return false;
        }
        match &mut self.phase {
            LedgerPhase::EarningShares => false,
            LedgerPhase::CostReduction { cost_basis } => {
                let shares_add = cash / price;
                let s_new = self.total_shares + shares_add;
                *cost_basis = (*cost_basis * self.total_shares + cash) / s_new;
                self.total_shares = s_new;
                self.undeployed_cash -= cash;
                true
            }
        }
    }

    /// 槽是否开放（Python `pos.active.get(key)`）。
    pub fn open_slot(&self, key: SlotKey) -> Option<&OpenLeg> {
        self.legs
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, leg)| leg)
    }

    /// 当前开放腿槽键（插入序快照；`_close` 强制清腿用）。
    pub fn open_keys(&self) -> Vec<SlotKey> {
        self.legs.iter().map(|(k, _)| *k).collect()
    }

    /// 开腿（高抛/REV 先卖）。返回是否实际开启（LOU 状态转换依赖）。
    /// 与 Python `open_diff` 逐句对应；stopped 集在 organic_fugue.py 中无写入方
    /// （恒空），故不建模——声明=能力。
    pub fn open_diff(
        &mut self,
        key: SlotKey,
        frac: f64,
        sell_price: f64,
        bar: i64,
        anchor: LegAnchor,
    ) -> bool {
        if self.open_slot(key).is_some() {
            return false;
        }
        let shares = self.total_shares * frac;
        if shares <= 0.0 {
            self.n_open_rejects_zero += 1;
            return false;
        }
        let law = if self.phase.is_earning() {
            ConservationLaw::AmountConserving
        } else {
            ConservationLaw::ShareConserving
        };
        self.legs.push((
            key,
            OpenLeg {
                cycle: OpenCycle {
                    shares,
                    open_price: sell_price,
                },
                law,
                anchor,
                side: DiffSide::Short,
            },
        ));
        if self.phase.is_earning() {
            self.total_shares -= shares;
        }
        if self.trace.is_some() {
            self.diff_open.push((key, bar));
        }
        true
    }

    /// 子腿开腿（递归赋格，绝对股数预算——预算基 = 父腿敞口，非 total_shares
    /// 比例；设计报告 §3.3 "budget(node) = 父层在本节点域内释放的敞口"）。
    /// side=Long：先买后卖（奇数深度反弹腿）；side=Short：先卖后买（偶数深度）。
    ///
    /// EarningShares 阶段拒绝：金额守恒（卖 V 买 V）对 Long 循环（买 V 卖 V′）
    /// 的 total_shares 算术未定义——声明=能力，显式拒绝（调用方计数），
    /// 不静默降级。仅 ShareConserving（名义短差，profit 降 cost_basis）开腿。
    pub fn open_sub(
        &mut self,
        key: SlotKey,
        shares: f64,
        price: f64,
        bar: i64,
        anchor: LegAnchor,
        side: DiffSide,
    ) -> bool {
        if self.open_slot(key).is_some() {
            return false;
        }
        if shares <= 0.0 {
            self.n_open_rejects_zero += 1;
            return false;
        }
        if self.phase.is_earning() {
            return false; // 调用方计 n_sub_earning_rejects
        }
        self.legs.push((
            key,
            OpenLeg {
                cycle: OpenCycle {
                    shares,
                    open_price: price,
                },
                law: ConservationLaw::ShareConserving,
                anchor,
                side,
            },
        ));
        if self.trace.is_some() {
            self.diff_open.push((key, bar));
        }
        true
    }

    /// 闭腿（Short 腿低吸买回 / Long 腿反弹顶卖出——close_price 按 side 解读）。
    /// 守恒律 match 穷举——第三种守恒律不可静默引入。
    pub fn close_diff(&mut self, key: SlotKey, close_price: f64, bar: i64) {
        let Some(pos_idx) = self.legs.iter().position(|(k, _)| *k == key) else {
            return;
        };
        if close_price <= 0.0 {
            return;
        }
        let (_, leg) = self.legs.remove(pos_idx);
        let cb_before = self.phase.cost_basis();
        let shares_before = self.total_shares;
        let closed = match leg.side {
            DiffSide::Short => ClosedCycle {
                shares: leg.cycle.shares,
                sell_price: leg.cycle.open_price,
                buy_price: close_price,
            },
            DiffSide::Long => ClosedCycle {
                shares: leg.cycle.shares,
                sell_price: close_price,
                buy_price: leg.cycle.open_price,
            },
        };
        match leg.law {
            ConservationLaw::AmountConserving => {
                // INV-2 金额守恒：卖 V 买 V，total_shares 净增（phase 已是 Earning）。
                // Long 腿不可达：open_sub 拒绝 earning 开腿（构造保证）。
                assert!(
                    leg.side == DiffSide::Short,
                    "AmountConserving × Long 不可表示（open_sub 构造保证），到达即 bug"
                );
                self.total_shares += leg.cycle.shares * leg.cycle.open_price / close_price;
            }
            ConservationLaw::ShareConserving => {
                // 股数守恒：profit 落袋 + 降共享 cost_basis；≤0 → 单向相变。
                let profit = closed.profit();
                self.cumulative_recovered += profit;
                match &mut self.phase {
                    LedgerPhase::CostReduction { cost_basis } => {
                        if self.total_shares > 0.0 {
                            *cost_basis -= profit / self.total_shares;
                        }
                        if *cost_basis <= 0.0 {
                            self.phase = LedgerPhase::EarningShares;
                        }
                    }
                    LedgerPhase::EarningShares => {
                        // T5 疑点路径（模块 docstring）：严格形式 = 成本概念已不存在，
                        // 不改写；计数可观测。
                        self.n_t5_shareconserving_after_earning += 1;
                    }
                }
            }
        }
        let was_earning = matches!(leg.law, ConservationLaw::AmountConserving);
        self.completed.push((key, closed));
        if self.trace.is_some() {
            let open_bar = self
                .diff_open
                .iter()
                .position(|(k, _)| *k == key)
                .map(|i| self.diff_open.remove(i).1)
                .unwrap_or(-1);
            // Short：开=卖、闭=买（Python 逐字）；Long：开=买、闭=卖。
            let (sell_bar, buy_bar) = match leg.side {
                DiffSide::Short => (open_bar, bar),
                DiffSide::Long => (bar, open_bar),
            };
            let cb_after = self.phase.cost_basis();
            let shares_delta = self.total_shares - shares_before;
            self.trace
                .as_mut()
                .expect("trace 已判 Some")
                .push(LegTrace {
                    slot: key,
                    sell_bar,
                    sell_price: closed.sell_price,
                    buy_bar,
                    buy_price: closed.buy_price,
                    shares: closed.shares,
                    diff: closed.sell_price - closed.buy_price,
                    profit: closed.profit(),
                    was_earning,
                    shares_delta,
                    cost_basis_before: cb_before,
                    cost_basis_after: cb_after,
                    upshift: matches!(
                        leg.anchor,
                        LegAnchor::Center {
                            kind: AnchorKind::OscUp,
                            ..
                        }
                    ),
                });
        }
    }
}

/// ShortBook — 空头书 [镜像推导]（双向会计体系 §2.2，
/// `analysis/bidirectional_nested_accounting.md`）。
///
/// **单相单律**（§2.4 非对称定理，L0）：
/// - 无相变变体：空头损失无界（c→∞）⇒ "负成本免费持仓"不动点不存在——
///   多头 `LedgerPhase::EarningShares` 在空头侧 L0 不可构造，类型不可表示。
/// - 无金额守恒变体：空头开仓是保证金担保的负债创设，"卖 V 买 V 增单位"
///   算术在空头侧符号反转且与负债机制不符（与在册 `open_sub` 对
///   EarningShares×Long 的拒绝同根源，ledger.rs `open_sub` docstring）。
/// - 唯一守恒律 = 张数守恒：腿利润 π ⇒ proceeds_basis += π/units
///   （方向协变公式 basis ← basis − d·π/units 在 d = −1 的面，§2.3）。
///
/// 空头书内的降成本腿恒为 **DiffSide::Long**（先买后卖——反弹段低买高卖），
/// `open_sub` 不带 side 参数：「空头书内只有 Long 循环」类型层表达。
///
/// `cumulative_recovered` = 纯观测量（载体条件相 cumulative_recovered ≥
/// posted_margin 的读数位；该相的会计后果被 31 课禁加仓否决 ⇒ 不进类型系统，
/// §2.4 载体条件相条款）。
#[derive(Debug)]
pub struct ShortBook {
    /// 空头单位数 U（合约张数/借出股数），恒为正。
    pub units: f64,
    /// 开空均价 B_s（多头 cost_basis 的极性镜像）：单位收入基，越高越有利。
    pub proceeds_basis: f64,
    /// 已实现短差利润累计（观测量，不驱动任何状态转移）。
    pub cumulative_recovered: f64,
    /// 开放腿，插入序存储（OrganicLedger 同纪律）。
    legs: Vec<(SlotKey, OpenLeg)>,
    /// 闭合腿（报告归因）。
    pub completed: Vec<(SlotKey, ClosedCycle)>,
    pub n_open_rejects_zero: u32,
}

impl ShortBook {
    pub fn new(open_price: f64, units: f64) -> Self {
        ShortBook {
            units,
            proceeds_basis: open_price,
            cumulative_recovered: 0.0,
            legs: Vec::new(),
            completed: Vec::new(),
            n_open_rejects_zero: 0,
        }
    }

    pub fn open_slot(&self, key: SlotKey) -> Option<&OpenLeg> {
        self.legs
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, leg)| leg)
    }

    pub fn open_keys(&self) -> Vec<SlotKey> {
        self.legs.iter().map(|(k, _)| *k).collect()
    }

    /// 开降成本腿（反弹起点先买）。空头书内腿恒 DiffSide::Long——无 side
    /// 参数（类型层穷尽，§2.2）。预算上界 = 在册 open_sub 同语义（调用方
    /// 保证 shares ≤ units——子腿名义 ≤ 父腿名义，嵌套守恒极性无关 §4.2）。
    pub fn open_sub(
        &mut self,
        key: SlotKey,
        shares: f64,
        buy_price: f64,
        anchor: LegAnchor,
    ) -> bool {
        if self.open_slot(key).is_some() {
            return false;
        }
        if shares <= 0.0 {
            self.n_open_rejects_zero += 1;
            return false;
        }
        self.legs.push((
            key,
            OpenLeg {
                cycle: OpenCycle {
                    shares,
                    open_price: buy_price,
                },
                law: ConservationLaw::ShareConserving,
                anchor,
                side: DiffSide::Long,
            },
        ));
        true
    }

    /// 闭腿（反弹顶卖出/再开空）。协变公式：π ⇒ B_s += π/U
    /// （basis ← basis − d·π/units，d = −1）。张数守恒：units 不动。
    pub fn close_diff(&mut self, key: SlotKey, sell_price: f64) {
        let Some(pos_idx) = self.legs.iter().position(|(k, _)| *k == key) else {
            return;
        };
        if sell_price <= 0.0 {
            return;
        }
        let (_, leg) = self.legs.remove(pos_idx);
        debug_assert!(
            leg.side == DiffSide::Long && leg.law == ConservationLaw::ShareConserving,
            "ShortBook 腿恒 Long×ShareConserving（open_sub 构造保证），到达即 bug"
        );
        let closed = ClosedCycle {
            shares: leg.cycle.shares,
            sell_price,
            buy_price: leg.cycle.open_price,
        };
        let profit = closed.profit();
        self.cumulative_recovered += profit;
        if self.units > 0.0 {
            // d = −1：basis −= (−1)·π/U = basis + π/U（风险解除方向 = ∞，
            // 无不动点——B_s 单调上移不触发任何相变，§2.4）。
            self.proceeds_basis += profit / self.units;
        }
        self.completed.push((key, closed));
    }
}

/// 带极性的账本（双向会计体系 §2.1 裁决：双账本极性分离，物理净额执行）。
///
/// 在 `OrganicLedger` 上加符号位被禁止——`total_shares` 允许负值 + 下游
/// `max(0,·)` 截断 = 降成本提款机 bug 的精确同型（符号污染）。类型层分离
/// 使污染编译不可达；多头书 = 在册类型逐字零改动（O0≡P5 零接触）。
#[derive(Debug)]
pub enum DirectionalBook {
    Long(OrganicLedger),
    Short(ShortBook),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor() -> LegAnchor {
        LegAnchor::SegmentScale
    }

    #[test]
    fn share_conserving_reduces_cost_basis() {
        // 100 股 @100，开腿 frac=0.5 卖 110 买回 100 → profit=500 →
        // cost_basis 100 − 500/100 = 95
        let mut led = OrganicLedger::new(100.0, 100.0, 0.5, true);
        assert!(led.open_diff(SlotKey::osc(2), 0.5, 110.0, 1, anchor()));
        led.close_diff(SlotKey::osc(2), 100.0, 2);
        assert_eq!(led.phase.cost_basis(), 95.0);
        assert_eq!(led.cumulative_recovered, 500.0);
        assert_eq!(led.total_shares, 100.0); // 股数守恒不动 total_shares
        let tr = &led.trace.as_ref().unwrap()[0];
        assert_eq!(tr.profit, 500.0);
        assert!(!tr.was_earning);
        assert_eq!(tr.shares_delta, 0.0);
    }

    #[test]
    fn phase_transition_is_one_way() {
        // cost_basis 1.0、大 profit → 相变 EarningShares；后续开腿走金额守恒
        let mut led = OrganicLedger::new(1.0, 100.0, 1.0, false);
        assert!(led.open_diff(SlotKey::osc(2), 1.0, 10.0, 1, anchor()));
        led.close_diff(SlotKey::osc(2), 5.0, 2); // profit=500 → cb=1−5=−4 → earning
        assert!(led.phase.is_earning());
        assert_eq!(led.phase.cost_basis(), 0.0);
        // earning 开腿：即时扣减 total_shares（INV-1 构造保证）
        assert!(led.open_diff(SlotKey::rev(2, 2), 0.5, 10.0, 3, anchor()));
        assert_eq!(led.total_shares, 50.0);
        // 金额守恒回补：卖 50×10=500 → 买 @5 得 100 股 → total = 50+100 = 150
        led.close_diff(SlotKey::rev(2, 2), 5.0, 4);
        assert_eq!(led.total_shares, 150.0);
        assert!(led.phase.is_earning()); // 不可逆
    }

    #[test]
    fn open_rejects_zero_shares() {
        let mut led = OrganicLedger::new(100.0, 0.0, 1.0, false);
        assert!(!led.open_diff(SlotKey::main(2), 1.0, 100.0, 1, anchor()));
        assert_eq!(led.n_open_rejects_zero, 1);
    }

    #[test]
    fn occupied_slot_rejected_without_counter() {
        let mut led = OrganicLedger::new(100.0, 100.0, 0.5, false);
        assert!(led.open_diff(SlotKey::main(2), 0.5, 110.0, 1, anchor()));
        assert!(!led.open_diff(SlotKey::main(2), 0.5, 111.0, 2, anchor()));
        assert_eq!(led.n_open_rejects_zero, 0); // 槽占用拒绝不入零股计数（Python 同序）
    }

    #[test]
    fn close_unknown_or_nonpositive_price_is_noop() {
        let mut led = OrganicLedger::new(100.0, 100.0, 0.5, false);
        led.close_diff(SlotKey::main(2), 100.0, 1); // 无腿
        assert!(led.open_diff(SlotKey::main(2), 0.5, 110.0, 1, anchor()));
        led.close_diff(SlotKey::main(2), 0.0, 2); // 非正价
        assert!(led.open_slot(SlotKey::main(2)).is_some());
    }

    #[test]
    fn sub_long_cycle_profit_reduces_cost_basis() {
        // Long 子腿（先买后卖）：买 100 卖 110，50 股 → profit=+500 →
        // cost_basis 100 − 500/100 = 95（与 Short 同一守恒算术）
        let key = SlotKey::rev_path(3, RevPath::single(3).child(2));
        let mut led = OrganicLedger::new(100.0, 100.0, 0.5, true);
        assert!(led.open_sub(key, 50.0, 100.0, 1, anchor(), DiffSide::Long));
        led.close_diff(key, 110.0, 2);
        assert_eq!(led.phase.cost_basis(), 95.0);
        assert_eq!(led.cumulative_recovered, 500.0);
        assert_eq!(led.total_shares, 100.0);
        let tr = &led.trace.as_ref().unwrap()[0];
        assert_eq!(tr.profit, 500.0);
        assert_eq!((tr.buy_price, tr.sell_price), (100.0, 110.0));
        assert_eq!((tr.buy_bar, tr.sell_bar), (1, 2)); // 开=买 bar、闭=卖 bar
        let (_, cyc) = led.completed.last().unwrap();
        assert_eq!(cyc.profit(), 500.0);
    }

    #[test]
    fn sub_open_rejected_in_earning_phase() {
        // EarningShares 下 open_sub 拒绝（金额守恒对 Long 循环未定义）
        let key = SlotKey::rev_path(3, RevPath::single(3).child(2));
        let mut led = OrganicLedger::new(1.0, 100.0, 1.0, false);
        assert!(led.open_diff(SlotKey::osc(2), 1.0, 10.0, 1, anchor()));
        led.close_diff(SlotKey::osc(2), 5.0, 2); // 相变 earning
        assert!(led.phase.is_earning());
        assert!(!led.open_sub(key, 50.0, 5.0, 3, anchor(), DiffSide::Long));
        assert!(led.open_slot(key).is_none());
    }

    #[test]
    fn with_reserve_and_tranche_weighted_avg() {
        // 入场 20% @100（200 股），追加 30% @120 → 均价 = 56000/450 = 124.44…
        // 资金守恒：undeployed 100000−20000−30000 = 50000
        let mut led = OrganicLedger::with_reserve(100.0, 20_000.0, 80_000.0, 1.0, false);
        assert_eq!(led.total_shares, 200.0);
        assert_eq!(led.phase.cost_basis(), 100.0);
        assert_eq!(led.undeployed_cash, 80_000.0);
        assert!(led.add_entry_tranche(30_000.0, 120.0));
        assert_eq!(led.total_shares, 450.0);
        assert_eq!(led.undeployed_cash, 50_000.0);
        // cb = (100×200 + 30000)/450 = 50000/450
        assert!((led.phase.cost_basis() - 50_000.0 / 450.0).abs() < 1e-12);
        // 超额追加拒绝（资金守恒）
        assert!(!led.add_entry_tranche(50_001.0, 100.0));
        // 补满到 0
        assert!(led.add_entry_tranche(50_000.0, 100.0));
        assert_eq!(led.undeployed_cash, 0.0);
        assert_eq!(led.total_shares, 950.0);
    }

    #[test]
    fn tranche_rejected_in_earning_phase() {
        let mut led = OrganicLedger::with_reserve(1.0, 50_000.0, 50_000.0, 1.0, false);
        assert!(led.open_diff(SlotKey::osc(2), 1.0, 10.0, 1, anchor()));
        led.close_diff(SlotKey::osc(2), 5.0, 2); // 相变 earning
        assert!(led.phase.is_earning());
        assert!(!led.add_entry_tranche(10_000.0, 5.0));
        assert_eq!(led.undeployed_cash, 50_000.0); // 拒绝不动现金
    }

    // ── ShortBook（双向会计 §8.2 测试 1/3：协变公式两极性 + 镜像对称）──

    #[test]
    fn covariant_formula_both_polarities() {
        // 同一笔短差利润 π=500、units=100：多头 basis 下移 5，空头 basis
        // 上移 5——basis ← basis − d·π/units 的两个极性面（§2.3）。
        let mut long = OrganicLedger::new(100.0, 100.0, 0.5, false);
        assert!(long.open_diff(SlotKey::osc(2), 0.5, 110.0, 1, anchor()));
        long.close_diff(SlotKey::osc(2), 100.0, 2); // Short 腿：卖110买100
        assert_eq!(long.phase.cost_basis(), 95.0); // 100 − (+1)·500/100

        let mut short = ShortBook::new(100.0, 100.0);
        assert!(short.open_sub(SlotKey::osc(2), 50.0, 100.0, anchor()));
        short.close_diff(SlotKey::osc(2), 110.0); // Long 腿：买100卖110，π=500
        assert_eq!(short.proceeds_basis, 105.0); // 100 − (−1)·500/100
        assert_eq!(short.units, 100.0); // 张数守恒
        assert_eq!(short.cumulative_recovered, 500.0);
    }

    #[test]
    fn short_book_mirror_symmetry() {
        // 镜像对称（§8.2 测试3）：价格序列镜像（p ↦ 200−p）下，空头书的
        // basis 移动量 = 多头书移动量取反、realized 轨迹相等。
        let mut long = OrganicLedger::new(100.0, 100.0, 1.0, false);
        assert!(long.open_diff(SlotKey::osc(3), 0.3, 112.0, 1, anchor()));
        long.close_diff(SlotKey::osc(3), 104.0, 2); // π = 0.3×100×8 = 240
        let d_long = 100.0 - long.phase.cost_basis();

        let mut short = ShortBook::new(100.0, 100.0);
        // 镜像磁带：卖112↦买88、买回104↦卖回96（200−p）
        assert!(short.open_sub(SlotKey::osc(3), 30.0, 88.0, anchor()));
        short.close_diff(SlotKey::osc(3), 96.0); // π = 30×8 = 240
        let d_short = short.proceeds_basis - 100.0;
        assert_eq!(
            d_long, d_short,
            "basis 移动量镜像相等（方向相反由字段语义承载）"
        );
        assert_eq!(long.cumulative_recovered, short.cumulative_recovered);
    }

    #[test]
    fn short_book_has_no_phase_transition() {
        // 非对称定理（§2.4）：任意大利润不触发任何相变——B_s 单调上移
        // 无不动点；显式豁免清单：空头侧相变事件数恒 0（类型上无相可变）。
        let mut short = ShortBook::new(1.0, 100.0);
        assert!(short.open_sub(SlotKey::osc(2), 100.0, 1.0, anchor()));
        short.close_diff(SlotKey::osc(2), 10.0); // π = 900 ≫ basis×units
        assert_eq!(short.proceeds_basis, 10.0); // 1 + 900/100，继续单调上移
        assert!(short.open_sub(SlotKey::osc(2), 100.0, 10.0, anchor()));
        short.close_diff(SlotKey::osc(2), 20.0); // 再开再闭——单相永续
        assert_eq!(short.proceeds_basis, 20.0);
        assert_eq!(short.units, 100.0);
    }

    #[test]
    fn short_book_rejects_zero_and_occupied() {
        let mut short = ShortBook::new(100.0, 100.0);
        assert!(!short.open_sub(SlotKey::osc(2), 0.0, 100.0, anchor()));
        assert_eq!(short.n_open_rejects_zero, 1);
        assert!(short.open_sub(SlotKey::osc(2), 10.0, 100.0, anchor()));
        assert!(!short.open_sub(SlotKey::osc(2), 10.0, 99.0, anchor()));
        short.close_diff(SlotKey::osc(2), 0.0); // 非正价 no-op
        assert!(short.open_slot(SlotKey::osc(2)).is_some());
    }

    #[test]
    fn t5_path_is_counted_not_rewritten() {
        // earning 转换后闭合 ShareConserving 旧腿（亏损）→ 计数，不改写成本
        let mut led = OrganicLedger::new(1.0, 100.0, 1.0, false);
        assert!(led.open_diff(SlotKey::osc(2), 0.3, 10.0, 1, anchor())); // 旧腿（cost>0 开）
        assert!(led.open_diff(SlotKey::osc(3), 0.5, 10.0, 1, anchor()));
        led.close_diff(SlotKey::osc(3), 5.0, 2); // 相变
        assert!(led.phase.is_earning());
        led.close_diff(SlotKey::osc(2), 20.0, 3); // 旧腿亏损闭合
        assert_eq!(led.n_t5_shareconserving_after_earning, 1);
        assert_eq!(led.phase.cost_basis(), 0.0); // 严格形式：不回正
    }
}
