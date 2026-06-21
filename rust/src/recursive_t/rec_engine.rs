//! **递归 T 操作引擎**（flat 逻辑递归化，编排者裁决 2026-06-21）。
//!
//! 本模块是 flat `t_engine.rs`（battle-tested CL +120%）的**递归架构表达**——用 `TInstance`/`TRoot`
//! 结构承载 flat 的已验证操作逻辑，**不加不减**。编排者裁决：删除所有递归引擎自创的约束
//! （C1 永不翻空 / candidate gate / 连续 level=父-1 / 方向 gate），逐行对照 flat 重新实装。
//!
//! ## 与 flat 的映射（递归化 = 同行为换载体）
//! - flat `layers: Vec<Layer>`（绝对 ladder 数组）→ `instances: Vec<TInstance>`（按 level 索引）。
//! - flat `nearest_active_parent(j)` = `(j+1..MAX).find(active)` → 遍历 instances 找 level>j 最低 active。
//! - flat `highest_active()` → 遍历 instances 找最高 level active。
//! - flat 单核心 campaign 三阶段（`stage`/`core_cost_basis`/`withdrawn`/`earning_cash`）→ 在 `TRoot`
//!   （非 per-instance，per-instance 三阶段是递归自创，已删）。
//! - flat `route_bsp`/`sink`/`recover`/`drain`/`enter`/`ascend`/`emergence_upgrade`/`clear_all`/`step`
//!   → `TRoot` 同名方法，逐行对照 flat。
//!
//! ## flat route_bsp 分派（无 C1/candidate/方向 gate，编排者裁决）
//! - **有 nearest_active_parent P**（子级）：反父向 BSP → sink（P 减仓 1/3 + j 开反向短差）或 drain
//!   （j 持遗留同父向仓 → 减暴露）；同父向 BSP → recover（j 持短差则平清升回 P）或 no-op。
//! - **无父**（核心级）：空仓 → enter（全 free，方向 = BSP 方向）；同向更高 ladder → ascend；
//!   **反向 → flip（clear 全塔 + 反向 enter）**——核心**可翻空**（flat 无 C1）。
//!
//! ## 会计（单一 free 池 + 单 campaign 三阶段，复用 rec_add/rec_reduce）
//! TW = free + Σ_inst sign(d)·u·c + withdrawn。每 rec_add/rec_reduce 同价 c NAV 中性 ⟹ TW 逐 bar 守恒。
//! 三阶段（第31课）：CostReduction（降成本）→ CapitalRecovered（退本金 free→withdrawn）→ EarningShares
//! （增股数）。account_reduce 按被减层方向：核心多头 reduce → 降成本；短差腿 → short_leg_pnl。
//!
//! ## 认识论等级
//! 数据结构/会计/守恒 = L0；route_bsp 行为对照 flat = L0（flat 已 L3 验证 CL+120%）；递归化后回测
//! 收益复现 = L3（验收：与 flat 行为等价）。

use super::types::Direction;
use crate::fugue_v3::SUB_LIQ_FACTOR;
use crate::trading::types::Polarity;

/// σ-不变配额 f = 1/λ（λ=3，中枢三段）= flat MOBILE_FRAC，保持对照可比。
const MOBILE_FRAC: f64 = 1.0 / 3.0;
/// 活跃/零化阈值 = flat（`Layer::is_active` / `reduce_at` 零化 / `add_at` 占用 = `1e-12`）。
/// **对照 flat，不自创**：rec 此前用 `1e-9`（比 flat 大 1000×），在 BTC 几何塔深层（核心 units
/// 衰减到 (1e-12, 1e-9) 带）误把仍活跃的核心多头零化 → highest_active 跌到次级别空头 → 核心翻空
/// 发散（BTC/Structural rec −14.9% vs flat +38.9%，首个分歧 bar=1842082：lv4 核心 L1.180e-9 经
/// sink 减到 7.867e-10，flat 仍活跃骑牛，rec ≤1e-9 零化丢核心翻空）。
const EPS: f64 = 1e-12;
/// 级别上界 = flat 有效操作级别数 = `MAX_LADDER(11) - BASE_LADDER(LADDER_MOVE=3) = 8`。
/// flat 的 ladder = bsp.level + BASE_LADDER，有效 ladder 3..10 ⟺ bsp.level/t_level 0..7；ladder≥11
/// （t_level≥8）被 flat ceiling 丢弃。rec 无 BASE_LADDER 偏移，故 ceiling 直接 = 8（对照 flat：
/// emergent t_level≥8 不升级、BSP level≥8 不消费）。BTC emergent t_level 触达 8+ 时此 ceiling 关键
/// （rec 此前 MAX_LEVEL=16 让 rec 升级而 flat 不升 → 核心方向错位 → BTC/Structural 空头主导发散）。
pub const MAX_LEVEL: usize = 8;

// ════════════════════════════ 走势节点身份（仓位骑节点）════════════════════════════

/// 走势节点身份（重锚键 = `start_bar`）。仓位骑节点——递归结构保留；操作逻辑用绝对 `level`（对照 flat ladder）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrendNode {
    pub start_bar: i64,
    pub end_bar: i64,
    pub price_lo: f64,
    pub price_hi: f64,
    pub direction: Direction,
}

impl TrendNode {
    pub fn new(start_bar: i64, end_bar: i64, price_lo: f64, price_hi: f64, direction: Direction) -> Self {
        TrendNode { start_bar, end_bar, price_lo, price_hi, direction }
    }
    pub fn id_key(&self) -> i64 {
        self.start_bar
    }
}

/// 走势几何方向 → 操作极性（Up=Long / Down=Short）。
pub fn dir_to_polarity(d: Direction) -> Polarity {
    match d {
        Direction::Up => Polarity::Long,
        Direction::Down => Polarity::Short,
    }
}

/// 极性反转（ε：短差腿反父向 / flip）。
pub fn flip_pol(p: Polarity) -> Polarity {
    match p {
        Polarity::Long => Polarity::Short,
        Polarity::Short => Polarity::Long,
    }
}

// ════════════════════════════ 持仓三阶段（第31课，单 campaign）════════════════════════════

/// 持仓成本三阶段（= flat TStage）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecStage {
    CostReduction,
    CapitalRecovered,
    EarningShares,
}

impl RecStage {
    pub fn as_u8(self) -> u8 {
        match self {
            RecStage::CostReduction => 0,
            RecStage::CapitalRecovered => 1,
            RecStage::EarningShares => 2,
        }
    }
}

// ════════════════════════════ NAV/TW 中性会计原语（= flat add_at/reduce_at）════════════════════════════

/// 减仓 m：按 direction 双重会计，现金回 free，返回 realized pnl。NAV 中性。
fn rec_reduce(inst: &mut TInstance, m: f64, free: &mut f64, c: f64) -> f64 {
    let pnl = match inst.direction {
        Polarity::Long => m * (c - inst.basis),
        Polarity::Short => m * (inst.basis - c),
    };
    match inst.direction {
        Polarity::Long => *free += m * c,
        Polarity::Short => *free -= m * c,
    }
    inst.units -= m;
    if inst.units <= EPS {
        inst.units = 0.0;
        inst.basis = f64::NAN;
    }
    pnl
}

/// 加仓 m（方向 dir）：现金从 free，basis 加权。NAV 中性。
fn rec_add(inst: &mut TInstance, m: f64, dir: Polarity, free: &mut f64, c: f64) {
    match dir {
        Polarity::Long => *free -= m * c,
        Polarity::Short => *free += m * c,
    }
    if inst.units > EPS {
        assert_eq!(
            inst.direction, dir,
            "rec_add 层内单一方向违反：向 {:?} 实例加 {:?}",
            inst.direction, dir
        );
        inst.basis = (inst.basis * inst.units + c * m) / (inst.units + m);
    } else {
        inst.direction = dir;
        inst.basis = c;
    }
    inst.units += m;
}

/// 配额 = units/3（= flat mobile_quota）。
fn quota(units: f64) -> f64 {
    units * MOBILE_FRAC
}

// ════════════════════════════ T 实例（按 level 索引，= flat Layer + 骑走势节点）════════════════════════════

/// 单级别 T 实例（= flat `Layer` + `node`）。`instances[level]` 是该绝对级别的净仓位（idle 时 units=0）。
/// 单一 direction（相邻级别 sink 短差方向相反）。三阶段在 `TRoot`（单 campaign，非 per-instance）。
#[derive(Debug, Clone, Copy)]
pub struct TInstance {
    /// 骑的走势节点（递归结构：仓位骑走势而非纯 ladder index）。enter/sink/ascend 设。
    pub node: TrendNode,
    /// 绝对级别（= flat ladder index）。
    pub level: usize,
    pub direction: Polarity,
    /// 持有股数 ≥0（符号在 direction）。
    pub units: f64,
    /// per-实例 basis（强平 + reduce pnl 用）。
    pub basis: f64,
}

impl TInstance {
    fn idle(level: usize) -> Self {
        TInstance {
            node: TrendNode::new(0, 0, 0.0, 0.0, Direction::Up),
            level,
            direction: Polarity::Long,
            units: 0.0,
            basis: f64::NAN,
        }
    }
    pub fn is_active(&self) -> bool {
        self.units > EPS
    }
}

// ════════════════════════════ 信号视图（= flat TSignalView，按 level）════════════════════════════

/// 本次重跑信号视图（= flat `TSignalView`，索引 = level）。纯 BSP 驱动（不分 type1/2/3）。
#[derive(Debug, Clone)]
pub struct LevelView {
    /// 该 level 是否新增任意买点（fresh）。
    pub buy: [bool; MAX_LEVEL],
    /// 该 level 是否新增任意卖点（fresh）。
    pub sell: [bool; MAX_LEVEL],
    /// 各 level 当前走势节点（enter/ascend/sink 骑节点用；None=该级无走势）。
    pub nodes: [Option<TrendNode>; MAX_LEVEL],
    /// T 迭代涌现上界 (level, 操作极性)——自下而上仓位涌现（= flat emergent_top）。None=本 bar 不升级。
    pub emergent_top: Option<(usize, Polarity)>,
}

impl LevelView {
    pub fn empty() -> Self {
        LevelView {
            buy: [false; MAX_LEVEL],
            sell: [false; MAX_LEVEL],
            nodes: [None; MAX_LEVEL],
            emergent_top: None,
        }
    }
}

impl Default for LevelView {
    fn default() -> Self {
        Self::empty()
    }
}

// ════════════════════════════ 递归 T 根（= flat TPositionEngine，单 free 池 + 单 campaign）════════════════════════════

/// 递归 T 引擎根（= flat `TPositionEngine`）：`instances[level]` 净仓位 + 单 free 池 + 单 campaign 三阶段。
pub struct TRoot {
    /// 各 level 净仓位（索引 = level）。
    instances: Vec<TInstance>,
    /// 单一共享现金池（NAV = free + Σ sign(d)·u·c）。
    free: f64,
    last_close: f64,

    // ── 持仓三阶段（单 campaign，= flat）──
    stage: RecStage,
    notional_in: f64,
    core_cost_basis: f64,
    campaign_entry_cost: f64,
    withdrawn: f64,
    earning_cash: f64,
    enable_earning: bool,
    enable_three_stage: bool,

    // ── 观测计数（纯诊断）──
    pub n_enters: u64,
    pub n_sinks: u64,
    pub n_recovers: u64,
    pub n_drains: u64,
    pub n_flips: u64,
    pub n_ascends: u64,
    pub n_emergence_upgrades: u64,
    pub n_liquidations: u64,
    pub n_capital_recovered: u64,
    pub n_earning_deploys: u64,
    pub short_leg_pnl: f64,
    pub earning_units_added: f64,
    pub max_core_gain_x1000: u64,
}

impl TRoot {
    pub fn new(initial_capital: f64) -> Self {
        let instances = (0..MAX_LEVEL).map(TInstance::idle).collect();
        TRoot {
            instances,
            free: initial_capital,
            last_close: f64::NAN,
            stage: RecStage::CostReduction,
            notional_in: 0.0,
            core_cost_basis: f64::NAN,
            campaign_entry_cost: f64::NAN,
            withdrawn: 0.0,
            earning_cash: 0.0,
            enable_earning: std::env::var("T_NO_EARNING").is_err(),
            enable_three_stage: std::env::var("T_NO_THREESTAGE").is_err(),
            n_enters: 0,
            n_sinks: 0,
            n_recovers: 0,
            n_drains: 0,
            n_flips: 0,
            n_ascends: 0,
            n_emergence_upgrades: 0,
            n_liquidations: 0,
            n_capital_recovered: 0,
            n_earning_deploys: 0,
            short_leg_pnl: 0.0,
            earning_units_added: 0.0,
            max_core_gain_x1000: 0,
        }
    }

    // ──────────────── 守恒（根级别）────────────────

    /// 总 NAV = free + Σ sign(d)·u·c（不含 withdrawn）。
    pub fn nav(&self, c: f64) -> f64 {
        let mut v = self.free;
        for inst in &self.instances {
            if inst.units > 0.0 {
                // = flat `nav`（`l.units > 0.0`，非 1e-12）。
                v += match inst.direction {
                    Polarity::Long => inst.units * c,
                    Polarity::Short => -inst.units * c,
                };
            }
        }
        v
    }

    /// 总财富 TW = NAV + withdrawn（逐 bar 唯一守恒量）。
    pub fn total_wealth(&self, c: f64) -> f64 {
        self.nav(c) + self.withdrawn
    }

    fn prove_tw_neutral(&self, tw_pre: f64, c: f64) {
        let tw_post = self.total_wealth(c);
        let tol = 1e-4 * tw_pre.abs().max(1.0);
        assert!(
            (tw_post - tw_pre).abs() <= tol,
            "TW 中性违反：pre={tw_pre} post={tw_post} (c={c})"
        );
    }

    pub fn free(&self) -> f64 {
        self.free
    }
    pub fn withdrawn_total(&self) -> f64 {
        self.withdrawn
    }
    pub fn stage(&self) -> RecStage {
        self.stage
    }
    pub fn last_close(&self) -> f64 {
        self.last_close
    }

    /// 实例只读访问（诊断/测试）。
    pub fn instance(&self, level: usize) -> &TInstance {
        &self.instances[level]
    }
    /// 活跃实例数（诊断/测试）。
    pub fn n_active(&self) -> usize {
        self.instances.iter().filter(|i| i.is_active()).count()
    }
    /// 暴露 (long_units, short_units)。
    pub fn exposure(&self) -> (f64, f64) {
        let mut lu = 0.0;
        let mut su = 0.0;
        for inst in &self.instances {
            if inst.units > 0.0 {
                // = flat `exposure`（`l.units > 0.0`，非 1e-12）。
                match inst.direction {
                    Polarity::Long => lu += inst.units,
                    Polarity::Short => su += inst.units,
                }
            }
        }
        (lu, su)
    }

    // ──────────────── nearest_active_parent / highest_active（= flat）────────────────

    /// 最高活跃级别（= flat highest_active）。无 → None。
    pub fn highest_active(&self) -> Option<usize> {
        (0..MAX_LEVEL).rev().find(|&k| self.instances[k].is_active())
    }

    /// level j 的最近活跃祖先（严格更高的第一个 active）= flat nearest_active_parent。无 → None（核心级）。
    fn nearest_active_parent(&self, j: usize) -> Option<usize> {
        (j + 1..MAX_LEVEL).find(|&k| self.instances[k].is_active())
    }

    /// 核心多头 units（Σ 多头）= 降成本分母。
    fn core_long_units(&self) -> f64 {
        self.instances.iter().filter(|l| l.units > EPS && l.direction == Polarity::Long).map(|l| l.units).sum()
    }

    // ──────────────── 三阶段会计（= flat account_reduce）────────────────

    /// 核算一次 reduce 的 realized（= flat account_reduce）。`dir` = 被减层方向。
    /// Long reduce（核心高位卖出）= 降成本；Short reduce（短差腿）= short_leg_pnl 单独算。
    fn account_reduce(&mut self, dir: Polarity, realized: f64, c: f64) {
        match dir {
            Polarity::Long => match self.stage {
                RecStage::CostReduction => {
                    let rem = self.core_long_units();
                    if rem > 1e-9 && self.core_cost_basis.is_finite() {
                        self.core_cost_basis -= realized / rem;
                        if self.campaign_entry_cost > 1e-9 {
                            let drop = ((self.campaign_entry_cost - self.core_cost_basis)
                                / self.campaign_entry_cost
                                * 1000.0)
                                .max(0.0) as u64;
                            self.max_core_gain_x1000 = self.max_core_gain_x1000.max(drop);
                        }
                        if self.core_cost_basis <= 0.0 && self.enable_three_stage {
                            self.stage = RecStage::CapitalRecovered;
                            self.n_capital_recovered += 1;
                            self.try_withdraw_capital(c);
                        }
                    }
                }
                RecStage::CapitalRecovered => self.try_withdraw_capital(c),
                RecStage::EarningShares => {
                    if realized > 0.0 {
                        self.earning_cash = (self.earning_cash + realized).min(self.free.max(0.0));
                    }
                }
            },
            Polarity::Short => {
                self.short_leg_pnl += realized;
                if self.stage == RecStage::EarningShares && realized > 0.0 {
                    self.earning_cash = (self.earning_cash + realized).min(self.free.max(0.0));
                }
            }
        }
    }

    /// 退本金（= flat try_withdraw_capital）：free→withdrawn 移出 = 初始本金 K。
    fn try_withdraw_capital(&mut self, _c: f64) {
        let need = self.notional_in - self.withdrawn;
        if need <= 1e-9 {
            self.enter_earning();
            return;
        }
        let w = need.min(self.free.max(0.0));
        if w > 1e-12 {
            self.withdrawn += w;
            self.free -= w;
        }
        if self.withdrawn >= self.notional_in - 1e-9 {
            self.enter_earning();
        }
    }

    fn enter_earning(&mut self) {
        if self.stage != RecStage::EarningShares {
            self.stage = RecStage::EarningShares;
        }
    }

    /// 增股数部署（= flat deploy_earning）：EarningShares 买点把 earning_cash 全额买成核心 units。
    fn deploy_earning(&mut self, core: usize, c: f64) {
        if !self.enable_earning || self.stage != RecStage::EarningShares || c <= 0.0 {
            return;
        }
        let cash = self.earning_cash.min(self.free.max(0.0));
        let q = cash / c;
        if !(q > 1e-12 && q.is_finite()) {
            return;
        }
        if self.instances[core].is_active() && self.instances[core].direction != Polarity::Long {
            return;
        }
        let mut free = self.free;
        rec_add(&mut self.instances[core], q, Polarity::Long, &mut free, c);
        self.free = free;
        self.earning_cash -= q * c;
        self.n_earning_deploys += 1;
        self.earning_units_added += q;
    }

    /// campaign 结束（flip/clear/eod）：归还 withdrawn、重置三阶段（= flat reset_campaign）。
    fn reset_campaign(&mut self) {
        self.free += self.withdrawn;
        self.withdrawn = 0.0;
        self.stage = RecStage::CostReduction;
        self.notional_in = 0.0;
        self.core_cost_basis = f64::NAN;
        self.campaign_entry_cost = f64::NAN;
        self.earning_cash = 0.0;
    }

    // ──────────────── τ 原子（= flat enter/clear_all/ascend/sink/recover/drain）────────────────

    /// **enter**（= flat enter）：核心级空仓首次建仓，全 free 建 `dir` 方向核心仓 @ level。
    fn enter(&mut self, level: usize, dir: Polarity, node: TrendNode, c: f64) {
        if c <= 0.0 {
            return;
        }
        self.reset_campaign();
        let m = self.free / c;
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        let spent = m * c;
        let mut free = self.free;
        rec_add(&mut self.instances[level], m, dir, &mut free, c);
        self.free = free;
        self.instances[level].node = node;
        self.instances[level].level = level;
        self.notional_in = spent;
        self.core_cost_basis = c;
        self.campaign_entry_cost = c;
        self.stage = RecStage::CostReduction;
        self.earning_cash = 0.0;
        self.n_enters += 1;
    }

    /// **clear_all**（= flat clear_all）：全塔平仓到现金 + reset_campaign。
    fn clear_all(&mut self, c: f64) {
        let mut free = self.free;
        for k in 0..MAX_LEVEL {
            let u = self.instances[k].units;
            if u > 1e-12 {
                rec_reduce(&mut self.instances[k], u, &mut free, c);
            }
        }
        self.free = free;
        self.reset_campaign();
    }

    /// **ascend**（= flat ascend）：核心仓 relabel 上移 from→to（to 须 idle），无新资金 NAV 中性。
    fn ascend(&mut self, from: usize, to: usize, node: TrendNode) {
        assert!(
            !self.instances[to].is_active(),
            "ascend 目标 level {to} 非 idle（units={}）：核心上移不可覆盖活跃层",
            self.instances[to].units
        );
        let mut moved = self.instances[from];
        moved.level = to;
        moved.node = node;
        self.instances[to] = moved;
        self.instances[from] = TInstance::idle(from);
        self.n_ascends += 1;
    }

    /// **emergence_upgrade**（= flat emergence_upgrade）：自下而上涌现，核心同向 → ascend 升级到 target。
    fn emergence_upgrade(&mut self, target_level: usize, target_dir: Polarity, node: TrendNode) {
        if target_level >= MAX_LEVEL {
            return;
        }
        if let Some(cc) = self.highest_active() {
            if self.instances[cc].direction == target_dir && cc < target_level {
                self.ascend(cc, target_level, node);
                self.n_emergence_upgrades += 1;
            }
        }
    }

    /// **sink @ (parent→sub)**（= flat sink）：父减仓 m=u_P/3 + 次级别开 flip(d_P) 短差 m。
    fn sink(&mut self, parent: usize, sub: usize, sub_node: TrendNode, c: f64) {
        let u_p = self.instances[parent].units;
        let pdir = self.instances[parent].direction;
        let m = quota(u_p);
        if !(m > 1e-12 && m.is_finite()) || m > u_p + 1e-9 {
            return;
        }
        let mob = flip_pol(pdir);
        if self.instances[sub].is_active() && self.instances[sub].direction != mob {
            return;
        }
        let tw_pre = self.total_wealth(c);
        let mut free = self.free;
        let realized = rec_reduce(&mut self.instances[parent], m, &mut free, c);
        rec_add(&mut self.instances[sub], m, mob, &mut free, c);
        self.free = free;
        self.instances[sub].node = sub_node;
        self.instances[sub].level = sub;
        self.account_reduce(pdir, realized, c);
        self.n_sinks += 1;
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **recover @ (sub→parent)**（= flat recover）：次级别走势完成 ⇒ 整条短差平清 m=u_sub 升回父 d_P。
    /// EarningShares 阶段父多头 → deploy_earning（增股数）。
    fn recover(&mut self, parent: usize, sub: usize, c: f64) {
        let pdir = self.instances[parent].direction;
        let mob = flip_pol(pdir);
        if !self.instances[sub].is_active() || self.instances[sub].direction != mob {
            return;
        }
        let m = self.instances[sub].units;
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        let tw_pre = self.total_wealth(c);
        let mut free = self.free;
        let realized = rec_reduce(&mut self.instances[sub], m, &mut free, c);
        rec_add(&mut self.instances[parent], m, pdir, &mut free, c);
        self.free = free;
        self.account_reduce(mob, realized, c);
        if pdir == Polarity::Long {
            self.deploy_earning(parent, c);
        }
        self.n_recovers += 1;
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **drain @ j**（= flat drain）：子级持同父向遗留仓 → 反父向 BSP 减暴露 1/3（不翻转）。
    fn drain(&mut self, j: usize, c: f64) {
        let u = self.instances[j].units;
        let m = quota(u);
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        let jdir = self.instances[j].direction;
        let tw_pre = self.total_wealth(c);
        let mut free = self.free;
        let realized = rec_reduce(&mut self.instances[j], m, &mut free, c);
        self.free = free;
        self.account_reduce(jdir, realized, c);
        self.n_drains += 1;
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **route_bsp**（= flat route_bsp）：level j 的 BSP 分派。无 C1/candidate/方向 gate。
    fn route_bsp(&mut self, j: usize, is_buy: bool, node: TrendNode, c: f64) {
        match self.nearest_active_parent(j) {
            // ── 子级（有活跃祖先 P）：区间套约束，永不独立翻转 ──
            Some(p) => {
                let pdir = self.instances[p].direction;
                let mob = flip_pol(pdir);
                let is_reduce = match pdir {
                    Polarity::Long => !is_buy,  // 父多：卖点=减仓信号
                    Polarity::Short => is_buy,  // 父空：买点=减仓信号
                };
                if is_reduce {
                    // 反父向 BSP：sink（空/已是短差）或 drain（遗留同父向仓）。
                    if !self.instances[j].is_active() || self.instances[j].direction == mob {
                        self.sink(p, j, node, c);
                    } else {
                        self.drain(j, c);
                    }
                } else {
                    // 同父向 BSP：recover（j 持短差则平整条升回）；否则 no-op（不 pyramid）。
                    if self.instances[j].is_active() && self.instances[j].direction == mob {
                        self.recover(p, j, c);
                    }
                }
            }
            // ── 核心级（无活跃祖先）：唯一独立翻转点 ──
            None => {
                let dir = if is_buy { Polarity::Long } else { Polarity::Short };
                match self.highest_active() {
                    None => self.enter(j, dir, node, c), // 首次建仓
                    Some(cc) => {
                        let cdir = self.instances[cc].direction;
                        if dir == cdir {
                            // 同向：更高 level ⇒ ascend 骑乘；同/低 level ⇒ no-op。
                            if j > cc && !self.instances[j].is_active() {
                                self.ascend(cc, j, node);
                            }
                        } else {
                            // 反向：核心翻转（走势终完美→新走势）——清全塔 + 同 level 反向 enter。
                            self.clear_all(c);
                            self.enter(j, dir, node, c);
                            self.n_flips += 1;
                        }
                    }
                }
            }
        }
    }

    // ──────────────── 单 bar 步进（= flat step）────────────────

    /// 单 bar 操作步进（= flat step）：强平 → emergence_upgrade → route_bsp top-down。
    pub fn on_bar(&mut self, view: &LevelView, bar: i64, c: f64) {
        let _ = bar;
        self.last_close = c;
        let tw_pre = self.total_wealth(c);

        // ── A. 边界算子：保证金强平，按三阶段切换（= flat）──
        match self.stage {
            RecStage::EarningShares => {
                // 全仓：账户级，in-system NAV≤0 ⇒ 连锁全平。
                if self.nav(c) <= 0.0 {
                    let mut free = self.free;
                    for k in 0..MAX_LEVEL {
                        let kdir = self.instances[k].direction;
                        let u = self.instances[k].units;
                        if u > 1e-12 {
                            let realized = rec_reduce(&mut self.instances[k], u, &mut free, c);
                            self.free = free;
                            self.n_liquidations += 1;
                            self.account_reduce(kdir, realized, c);
                            free = self.free;
                        }
                    }
                    self.free = free;
                }
            }
            _ => {
                // 逐仓：每层独立 basis 判（CostReduction / CapitalRecovered）。
                for k in 0..MAX_LEVEL {
                    let l = self.instances[k];
                    if l.units > 1e-12 {
                        let liq = match l.direction {
                            Polarity::Long => c > 0.0 && c <= l.basis / SUB_LIQ_FACTOR,
                            Polarity::Short => c >= SUB_LIQ_FACTOR * l.basis,
                        };
                        if liq {
                            let mut free = self.free;
                            let realized = rec_reduce(&mut self.instances[k], l.units, &mut free, c);
                            self.free = free;
                            self.n_liquidations += 1;
                            self.account_reduce(l.direction, realized, c);
                        }
                    }
                }
            }
        }

        // ── A'. 自下而上仓位涌现升级（route_bsp 前）──
        if let Some((target_level, target_dir)) = view.emergent_top {
            let node = view.nodes.get(target_level).copied().flatten().unwrap_or_else(|| {
                TrendNode::new(bar, bar, c, c, match target_dir {
                    Polarity::Long => Direction::Up,
                    Polarity::Short => Direction::Down,
                })
            });
            self.emergence_upgrade(target_level, target_dir, node);
        }

        // ── B. BSP 路由：top-down（高 level 先，区间套自上而下）──
        for j in (0..MAX_LEVEL).rev() {
            let b = view.buy[j];
            let s = view.sell[j];
            if b && s {
                continue; // 同 bar 同 level 买卖冲突 → 跳过
            }
            let node = view.nodes.get(j).copied().flatten().unwrap_or_else(|| {
                TrendNode::new(bar, bar, c, c, if b { Direction::Up } else { Direction::Down })
            });
            if b {
                self.route_bsp(j, true, node, c);
            } else if s {
                self.route_bsp(j, false, node, c);
            }
        }

        self.prove_tw_neutral(tw_pre, c);
    }

    /// 收尾（全平到现金，归还 withdrawn）。返回 final_nav (= free)。
    pub fn finish(&mut self, c: f64) -> f64 {
        if c.is_finite() && c > 0.0 {
            self.clear_all(c);
        }
        self.free
    }
}
