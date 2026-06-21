//! **递归 T 操作引擎**（recursive self-similar，docs/recursive_t_architecture_v2.md 落码）。
//!
//! 与 flat `t_engine.rs`（绝对 ladder 数组 + 中央 route_bsp）并存——本模块是 v2 设计的递归实装：
//! 每级别一个 `TInstance` 骑一条**走势节点**（`TrendNode`，不骑绝对 ladder），父子局部通信
//! （sink/recover/spawn），级别涌现 spawn 实例，子 T 骑「回调下跌走势」持空头。会计全局态集中于
//! 根账本（单一 `free`，编排者裁决 §8.7），仓位隔离（每实例独立 units/basis/cost_basis/phase）。
//!
//! ## 自相似（第65课 aₙ=f(aₙ₋₁)）
//! 核心仓与短差仓是**同一个 T 在不同递归深度的实例**：父 T 骑本级别上涨走势持 Long 核心；其回调
//! 下跌走势上 spawn 子 T 持 Short（= 次级别短差），子 T 自相似地在自己的回调上再 spawn。无"核心 vs
//! 短差"本体区别——只有递归深度不同。
//!
//! ## 三 τ 操作（编排者三裁决，已结算 §8）
//! - **sink**（父级核心走势中确认一条回调下跌 node）：`rec_reduce(parent, m)` 父减仓到现金 +
//!   spawn 子 T `rec_add(child, m, Short)` 开空（同股数 m）。回调走势的结构性确认本身即门——**无门控
//!   参数**（零操作参数原则，§8.2）。
//! - **recover**（子 T 回调走势完成，底背驰）：`rec_reduce(child, m_short)` 平空（realized=降成本
//!   alpha）+ 父级按 phase 升回（CostReduction 同股数 m / EarningShares 同金额 earning/c）。
//! - **spawn / flip**（最高级别走势完成，无父）：spawn=找到更大容器升回新父（零现金）；flip=反向
//!   （全树塌缩：子 T 先按旧方向平空升回，再清根反向，§3.6）。
//!
//! ## 会计不变量（§4.5）
//! - **TW 中性**（逐 bar 唯一守恒，跨阶段）：`TW = free + Σ_inst sign(d)·u·c + withdrawn`。退本金
//!   free→withdrawn 使 NAV 掉 K 但 TW 不变。守卫在**根**用 total_wealth（逐实例守卫漏跨实例配对错）。
//! - **同股数**（CostReduction）/ **同金额**（EarningShares）by phase。
//! - **free 不足回补 = 结构检测 bug → fail-loud panic**（§8.1：回调终点必低于起点，否则非回调）。
//!
//! ## 认识论等级
//! 数据结构/会计/守恒/三操作 = **L0**（从 v2 设计 + accounting 代数）；on_bar/走势树接入 + 回测
//! 收益 = **待实装/L3**。本模块当前是引擎核心（结构 + 操作 + 守恒），on_bar 与 stream 接入是下一步。

use crate::trading::types::Polarity;
use super::types::Direction;

/// σ-不变配额 f = 1/λ（λ=3，中枢三段）。MOBILE_FRAC 是工程参数（§8.3 开放：无原文依据，
/// 与零操作参数原则有张力，待 L2 裁决——此处沿用 flat 引擎口径保持对照可比）。
const MOBILE_FRAC: f64 = 1.0 / 3.0;

/// NAV/TW 中性容差（同 accounting/prove 口径）。
const EPS: f64 = 1e-9;

// ════════════════════════════ 走势节点身份（A1：仓位骑节点，非绝对 ladder）════════════════════════════

/// 走势节点身份。重锚键 = `start_bar`（依赖 iterate 的 append-only 前缀冻结使其稳定——§8.6 该性质
/// 未证，是 N4 测试的验收对象）。`relative_level` 仅诊断，绝不作存储/匹配键。
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
    /// 节点身份键（A1：用 start_bar，跨重定级稳定；非绝对 ladder）。
    pub fn id_key(&self) -> i64 {
        self.start_bar
    }
}

/// 走势几何方向 → 操作极性（Up 走势=Long 归属 / Down 走势=Short 归属）。
pub fn dir_to_polarity(d: Direction) -> Polarity {
    match d {
        Direction::Up => Polarity::Long,
        Direction::Down => Polarity::Short,
    }
}

/// 极性反转（ε，仅用于 flip 与子 T 短头方向 = 反父向）。
pub fn flip_pol(p: Polarity) -> Polarity {
    match p {
        Polarity::Long => Polarity::Short,
        Polarity::Short => Polarity::Long,
    }
}

// ════════════════════════════ 三阶段（per 实例，第31课）════════════════════════════

/// 持仓成本三阶段（与 t_engine::TStage 同义，递归版独立定义避免跨模块耦合）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecStage {
    /// ① 降成本：realized < notional，恒仓短差。
    CostReduction,
    /// ② 退本金：cost_basis 穿 0，移出 = 初始本金的现金到安全池。
    CapitalRecovered,
    /// ③ 增股数：本金已全退，纯利润买更多 units，单向不可逆。
    EarningShares,
}

// ════════════════════════════ 实例引用（代际句柄，防 ABA §1.3）════════════════════════════

/// 实例引用 = (槽位, 代际)。dormant 槽位复用时 generation++；派发前校验 generation 一致，
/// 不匹配 = 引用已失效（ABA），走孤儿兜底（pending 由根回收平账）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TRef {
    pub slot: usize,
    pub generation: u64,
}

/// 实例生命周期（§3.7 状态图）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TLifecycle {
    /// 空槽（保留待复用，generation 已++）。
    Dormant,
    /// 持仓活跃。
    Active,
    /// 最高级别走势完成、容器未确认的待定态（§3.5 修复 base case 时序 gap）。
    PendingContainer,
}

// ════════════════════════════ T 实例（骑一条走势节点 + 仓位 + campaign 份额）════════════════════════════

/// 单个级别的 T 实例。骑一条走势节点，持单一方向仓位（§1.3）。
///
/// 仓位隔离（§8.7）：每实例独立 units/basis/cost_basis/phase；现金不在实例，在根账本。
/// `withdrawn`/`earning` 是「归属本实例的根池份额标签」，物理现金在 `TRoot`。
#[derive(Debug, Clone)]
pub struct TInstance {
    pub node: TrendNode,
    /// 操作极性（Up 走势=Long 核心 / Down 回调走势=Short 子 T；A2 由所骑走势方向定，非 BSP 推断）。
    pub direction: Polarity,
    /// 持有股数 ≥0（符号在 direction，禁 sign 位）。子 T 的短头 units 即「待回补量」（同股数 N）。
    pub units: f64,
    /// per-实例 basis（强平 + 逐笔 pnl 用）。
    pub basis: f64,
    /// 三阶段有效持仓成本（可<0；穿0→退本金）。NaN=无 campaign。
    pub cost_basis: f64,
    pub phase: RecStage,
    /// 本 campaign 投入本金 K_k。
    pub notional_in: f64,
    /// 本实例已退本金份额（物理在 root.withdrawn_total）。
    pub withdrawn: f64,
    /// 本实例③阶段弹药份额（物理在 root.free）。
    pub earning: f64,
    pub parent: Option<TRef>,
    pub child: Option<TRef>,
    pub lifecycle: TLifecycle,
    /// 代际（dormant 复活时++）。
    pub generation: u64,
}

impl TInstance {
    fn dormant(generation: u64) -> Self {
        TInstance {
            node: TrendNode::new(0, 0, 0.0, 0.0, Direction::Up),
            direction: Polarity::Long,
            units: 0.0,
            basis: f64::NAN,
            cost_basis: f64::NAN,
            phase: RecStage::CostReduction,
            notional_in: 0.0,
            withdrawn: 0.0,
            earning: 0.0,
            parent: None,
            child: None,
            lifecycle: TLifecycle::Dormant,
            generation,
        }
    }
    pub fn is_active(&self) -> bool {
        self.lifecycle != TLifecycle::Dormant && self.units > EPS
    }
    fn self_ref(&self, slot: usize) -> TRef {
        TRef { slot, generation: self.generation }
    }
}

// ════════════════════════════ NAV/TW 中性会计原语（mirror accounting.rs，§4.3）════════════════════════════

/// 减仓 m：按 direction 双重会计，现金回 free，返回 realized pnl。NAV 中性。
/// reduce(Long)=卖出 free+=m·c；reduce(Short)=平空 cover free−=m·c。
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
/// add(Long)=买入 free−=m·c；add(Short)=开空 free+=m·c。
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

fn quota(units: f64) -> f64 {
    units * MOBILE_FRAC
}

// ════════════════════════════ 递归 T 根（单一账本 + 实例树）════════════════════════════

/// 递归 T 引擎根：持唯一现金池 `free` + 三阶段安全池 `withdrawn_total` + 实例槽表。
///
/// 守恒律在根级别（§4.5 I1/I3）：`TW = free + Σ_inst sign(d)·u·c + withdrawn_total`。
pub struct TRoot {
    /// 单一现金池（fungible 交换媒介，§8.7）。
    free: f64,
    /// 全树安全池（退本金移出在险，§4.5 I3）。
    withdrawn_total: f64,
    /// 槽位表（dormant 可复用，generation 防 ABA）。
    instances: Vec<TInstance>,
    /// 当前最高涌现级别实例（无父，唯一独立 flip）。None=全空。
    root_slot: Option<usize>,
    last_close: f64,
    // ── 观测计数（纯诊断）──
    pub n_enters: u64,
    pub n_sinks: u64,
    pub n_recovers: u64,
    pub n_spawns: u64,
    pub n_capital_recovered: u64,
    pub short_leg_pnl: f64,
    /// 诊断：recover 时短差腿亏损（realized<0，c2>c1 卖点失败）/ 盈利（realized>0）次数。
    pub n_recover_loss: u64,
    pub n_recover_win: u64,
    /// 诊断哨兵：§8.1 free 不足（恒仓亏损短差同股数回补不可能）panic 触发次数（生产恒 0；>0 即 C3 显形）。
    pub n_freeshort: u64,
}

impl TRoot {
    pub fn new(initial_capital: f64) -> Self {
        TRoot {
            free: initial_capital,
            withdrawn_total: 0.0,
            instances: Vec::new(),
            root_slot: None,
            last_close: f64::NAN,
            n_enters: 0,
            n_sinks: 0,
            n_recovers: 0,
            n_spawns: 0,
            n_capital_recovered: 0,
            short_leg_pnl: 0.0,
            n_recover_loss: 0,
            n_recover_win: 0,
            n_freeshort: 0,
        }
    }

    // ──────────────── 守恒（根级别，§4.5）────────────────

    /// 总 NAV = free + Σ sign(d)·u·c（不含 withdrawn）。
    pub fn nav(&self, c: f64) -> f64 {
        let mut v = self.free;
        for inst in &self.instances {
            if inst.units > EPS {
                v += match inst.direction {
                    Polarity::Long => inst.units * c,
                    Polarity::Short => -inst.units * c,
                };
            }
        }
        v
    }

    /// 总财富 TW = NAV + withdrawn_total（逐 bar 唯一守恒量，§4.5 I3）。
    pub fn total_wealth(&self, c: f64) -> f64 {
        self.nav(c) + self.withdrawn_total
    }

    /// 守卫：同价 c 下操作前后 TW 中性（违反 = panic，捕跨实例配对错误）。
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
        self.withdrawn_total
    }
    pub fn root_slot(&self) -> Option<usize> {
        self.root_slot
    }
    pub fn instance(&self, slot: usize) -> &TInstance {
        &self.instances[slot]
    }
    /// 活跃实例数（含子 T）。
    pub fn n_active(&self) -> usize {
        self.instances.iter().filter(|i| i.is_active()).count()
    }
    /// 当前多/空 units 总敞口。
    pub fn exposure(&self) -> (f64, f64) {
        let mut lu = 0.0;
        let mut su = 0.0;
        for i in &self.instances {
            if i.units > EPS {
                match i.direction {
                    Polarity::Long => lu += i.units,
                    Polarity::Short => su += i.units,
                }
            }
        }
        (lu, su)
    }

    /// 校验 TRef 有效并返回槽（driver 遍历链用，§1.3 防 ABA）。失效返回 None。
    pub fn resolve_ref(&self, r: TRef) -> Option<usize> {
        self.resolve(r)
    }

    /// 取槽位的 TRef（带当前 generation，driver 调 recover 用）。
    pub fn ref_of(&self, slot: usize) -> TRef {
        self.instances[slot].self_ref(slot)
    }

    /// 校验 TRef 有效（generation 一致，§1.3 防 ABA）。失效返回 None。
    fn resolve(&self, r: TRef) -> Option<usize> {
        self.instances
            .get(r.slot)
            .filter(|i| i.generation == r.generation && i.lifecycle != TLifecycle::Dormant)
            .map(|_| r.slot)
    }

    /// 分配一个实例槽（复用 dormant，generation++；否则新建）。
    fn alloc_slot(&mut self) -> usize {
        if let Some(slot) = self.instances.iter().position(|i| i.lifecycle == TLifecycle::Dormant) {
            self.instances[slot].generation += 1;
            slot
        } else {
            let gen = 0;
            self.instances.push(TInstance::dormant(gen));
            self.instances.len() - 1
        }
    }

    // ──────────────── 三阶段会计（per 实例，§4.1）────────────────

    /// 核算一次**核心 spine** reduce 的 realized（三阶段，**方向对称**，编排者 2026-06-20 零 workaround）：
    /// realized 由 `rec_reduce` 按 direction 已算正确（Long=m(c−basis) 卖高 / Short=m(basis−c) 平低），
    /// 多空核心**同一降成本公式**——删除 §12 的 Long 三阶段/Short 平铺不对称分支。降成本→退本金→增股数对多空一致。
    /// 短差腿（recover 子 T）走 `account_leg_pnl`，不入此（点5：核心 vs 腿是拓扑角色区分，非方向）。
    fn account_core_reduce(&mut self, slot: usize, realized: f64, _c: f64) {
        let rem = self.instances[slot].units; // reduce 后剩余核心 units（降成本分母）
        let inst = &mut self.instances[slot];
        if rem > EPS && inst.cost_basis.is_finite() {
            inst.cost_basis -= realized / rem;
            if inst.cost_basis <= 0.0 && inst.phase == RecStage::CostReduction {
                inst.phase = RecStage::CapitalRecovered;
                self.n_capital_recovered += 1;
                self.try_withdraw(slot);
            }
        } else if inst.phase == RecStage::CapitalRecovered {
            self.try_withdraw(slot);
        } else if inst.phase == RecStage::EarningShares && realized > 0.0 {
            inst.earning = (inst.earning + realized).min(self.free.max(0.0));
        }
    }

    /// 短差腿 pnl（recover 子 T 平差）单独核算（点5），不入核心降成本。**方向对称**（拓扑角色，非方向）：
    /// 多头核心的空头短差腿 / 空头核心的多头短差腿，统一记 `short_leg_pnl`。
    fn account_leg_pnl(&mut self, realized: f64) {
        self.short_leg_pnl += realized;
    }

    /// 退本金：把 = 本金的现金移出在险池（free→withdrawn）。free 不足抽可得部分，停留②待补；
    /// 全额抽出后切③（单向不可逆 OQ-9）。TW 守恒（free→withdrawn）。
    fn try_withdraw(&mut self, slot: usize) {
        let need = self.instances[slot].notional_in - self.instances[slot].withdrawn;
        if need <= EPS {
            self.instances[slot].phase = RecStage::EarningShares;
            return;
        }
        let w = need.min(self.free.max(0.0));
        if w > EPS {
            self.instances[slot].withdrawn += w;
            self.free -= w;
            self.withdrawn_total += w;
        }
        if self.instances[slot].withdrawn >= self.instances[slot].notional_in - EPS {
            self.instances[slot].phase = RecStage::EarningShares;
        }
    }

    // ──────────────── τ 操作 ────────────────

    /// **enter**（根首仓）：空树时在最高涌现走势 node 上用全部 free 建核心仓（方向 = node 方向）。
    pub fn enter(&mut self, node: TrendNode, c: f64, bar: i64) -> Option<usize> {
        let _ = bar;
        if self.root_slot.is_some() || c <= 0.0 {
            return None;
        }
        let m = self.free / c;
        if !(m > EPS && m.is_finite()) {
            return None;
        }
        let dir = dir_to_polarity(node.direction);
        let tw_pre = self.total_wealth(c);
        let slot = self.alloc_slot();
        {
            let inst = &mut self.instances[slot];
            inst.node = node;
            inst.direction = dir;
            inst.lifecycle = TLifecycle::Active;
            inst.parent = None;
            inst.child = None;
            inst.notional_in = m * c;
            inst.cost_basis = c;
            inst.phase = RecStage::CostReduction;
            inst.withdrawn = 0.0;
            inst.earning = 0.0;
        }
        let mut free = self.free;
        rec_add(&mut self.instances[slot], m, dir, &mut free, c);
        self.free = free;
        self.root_slot = Some(slot);
        self.n_enters += 1;
        self.prove_tw_neutral(tw_pre, c);
        Some(slot)
    }

    /// **sink**（父级核心走势中确认一条回调下跌 node）：父减仓 m=quota 到现金 + spawn 子 T 持空 m
    /// （同股数）。回调走势的结构性确认本身即门，无门控参数（§8.2）。返回子 T 槽。
    ///
    /// `callback_node` = 回调走势节点（方向必反父向：父 Long→回调 Down；父 Short→回调 Up）。
    pub fn sink(&mut self, parent_slot: usize, callback_node: TrendNode, c: f64, bar: i64) -> Option<usize> {
        let _ = bar;
        if c <= 0.0 || !self.instances[parent_slot].is_active() {
            return None;
        }
        let pdir = self.instances[parent_slot].direction;
        let mob = flip_pol(pdir); // 子 T 短头方向 = 反父向
        // 回调方向校验（结构即门）：回调 node 方向必反父向。
        if dir_to_polarity(callback_node.direction) != mob {
            return None;
        }
        let u_p = self.instances[parent_slot].units;
        let m = quota(u_p);
        if !(m > EPS && m.is_finite()) || m > u_p + EPS {
            return None;
        }
        let tw_pre = self.total_wealth(c);
        // 父级减仓到现金，realized 降成本（父 Long）。
        let mut free = self.free;
        let realized = rec_reduce(&mut self.instances[parent_slot], m, &mut free, c);
        self.free = free;
        self.account_core_reduce(parent_slot, realized, c);
        // spawn 子 T 骑回调 node，开空 m（同股数）。
        let child_slot = self.alloc_slot();
        let pref = self.instances[parent_slot].self_ref(parent_slot);
        {
            let child = &mut self.instances[child_slot];
            child.node = callback_node;
            child.lifecycle = TLifecycle::Active;
            child.parent = Some(pref);
            child.child = None;
            child.cost_basis = f64::NAN; // 短差腿不走核心降成本
            child.phase = RecStage::CostReduction;
            child.notional_in = 0.0;
            child.withdrawn = 0.0;
            child.earning = 0.0;
        }
        let mut free = self.free;
        rec_add(&mut self.instances[child_slot], m, mob, &mut free, c);
        self.free = free;
        let cref = self.instances[child_slot].self_ref(child_slot);
        self.instances[parent_slot].child = Some(cref);
        self.n_sinks += 1;
        self.prove_tw_neutral(tw_pre, c);
        Some(child_slot)
    }

    /// **recover**（子 T 回调走势完成，底背驰）：子 T 平空（realized=降成本 alpha 入 short_leg_pnl）
    /// + 父级按 phase 升回（CostReduction 同股数 m / EarningShares 同金额 earning/c = 增股数）。
    ///
    /// §8.1：父级回补**全量同股数**（CostReduction），free 不足 = 结构 bug → fail-loud panic。
    pub fn recover(&mut self, parent_ref: TRef, child_ref: TRef, c: f64, bar: i64) -> bool {
        let _ = bar;
        let (Some(parent_slot), Some(child_slot)) = (self.resolve(parent_ref), self.resolve(child_ref)) else {
            return false; // ABA：引用失效，孤儿兜底（不动账，调用方处理）
        };
        if c <= 0.0 || !self.instances[child_slot].is_active() {
            return false;
        }
        let m_short = self.instances[child_slot].units; // 子 T 短头 units = 待回补同股数 N
        let c1_sink = self.instances[child_slot].basis; // 诊断：子 T 开空价（sink 价 c1），reduce 后变 NaN 故先抓
        let pdir = self.instances[parent_slot].direction;
        let tw_pre = self.total_wealth(c);
        // 子 T 平空（cover）：realized = 高开低平降成本 alpha。
        let mut free = self.free;
        let realized = rec_reduce(&mut self.instances[child_slot], m_short, &mut free, c);
        self.free = free;
        self.account_leg_pnl(realized); // 短差腿（方向对称：多核心→空腿 / 空核心→多腿）单独算，不入降成本
        // 诊断（编排者 Q4）：短差盈亏符号。realized<0 = c2>c1（"回调"反而涨了）= 卖点失败 = 亏损短差。
        if realized < -EPS {
            self.n_recover_loss += 1;
        } else if realized > EPS {
            self.n_recover_win += 1;
        }
        // 父级升回，按 phase 分流。
        let phase = self.instances[parent_slot].phase;
        let q = match phase {
            RecStage::CostReduction | RecStage::CapitalRecovered => m_short, // 同股数（恒仓回复）
            RecStage::EarningShares => {
                let cash = self.instances[parent_slot].earning.min(self.free.max(0.0));
                cash / c // 同金额（增股数）
            }
        };
        if q > EPS {
            // §8.1 fail-loud（方向感知，从定义推导，非硬编码不对称）：多头回补=买入需现金充足
            // （回调终点<起点⟹卖价>买价）；空头回补=再做空收现金（free 增），无现金约束。
            let pdir2 = self.instances[parent_slot].direction;
            if matches!(phase, RecStage::CostReduction | RecStage::CapitalRecovered)
                && pdir2 == Polarity::Long
                && self.free + EPS < q * c
            {
                // free 不足以同股数多头回补：c2>c1 亏损短差（恒仓 free≈0）= C3 显形（P3b 实测未触发，sink=0）。
                self.n_freeshort += 1;
                panic!(
                    "free 不足以同股数多头回补（结构检测 bug，§8.1）：free={} need={} c={} c1_sink={}",
                    self.free,
                    q * c,
                    c,
                    c1_sink
                );
            }
            let mut free = self.free;
            rec_add(&mut self.instances[parent_slot], q, pdir, &mut free, c);
            self.free = free;
            if phase == RecStage::EarningShares {
                self.instances[parent_slot].earning -= q * c;
            }
        }
        // 子 T 注销 → dormant（generation 保留，复活时++）。
        self.dormant_instance(child_slot);
        self.instances[parent_slot].child = None;
        self.n_recovers += 1;
        self.prove_tw_neutral(tw_pre, c);
        true
    }

    /// **spawn = relabel 升格**（更高同向走势涌现，§4.3 + flat emergence_upgrade 已验证语义）：核心持仓
    /// **整体迁到**更高走势 node 的新实例（units/basis/cost_basis/campaign/回调链全继承），老 root → dormant。
    /// 核心骑上更高级别走势（相对级别升一层），**零现金流**（仓位身份迁移，同价 TW 中性）。返回新 root 槽。
    /// 方向不一致（涌现反向 = 29课情况三）→ 返回 None（调用方应走 flip，§3.5 第三态）。
    pub fn spawn(&mut self, higher_node: TrendNode, c: f64, bar: i64) -> Option<usize> {
        let _ = bar;
        let root = self.root_slot?;
        let new_dir = dir_to_polarity(higher_node.direction);
        // §3.5 第三态：涌现方向 ≠ 核心方向 ⟹ 不 spawn（应 flip）。
        if new_dir != self.instances[root].direction {
            return None;
        }
        let tw_pre = self.total_wealth(c);
        let old = self.instances[root].clone();
        let new_slot = self.alloc_slot();
        {
            let np = &mut self.instances[new_slot];
            np.node = higher_node;
            np.direction = old.direction;
            np.units = old.units; // 持仓整体继承（relabel，非新建）
            np.basis = old.basis;
            np.cost_basis = old.cost_basis;
            np.phase = old.phase;
            np.notional_in = old.notional_in;
            np.withdrawn = old.withdrawn;
            np.earning = old.earning;
            np.lifecycle = TLifecycle::Active;
            np.parent = None;
            np.child = old.child; // 回调链继承
        }
        // 老 root 的回调子 T（若有）parent 重指向新实例。
        if let Some(cref) = old.child {
            if let Some(cslot) = self.resolve(cref) {
                let nref = self.instances[new_slot].self_ref(new_slot);
                self.instances[cslot].parent = Some(nref);
            }
        }
        self.dormant_instance(root);
        self.root_slot = Some(new_slot);
        self.n_spawns += 1;
        self.prove_tw_neutral(tw_pre, c); // 持仓同价迁移，TW 中性
        Some(new_slot)
    }

    // promote / flip 已删除（编排者裁决 C1，2026-06-20）：核心永不整仓翻空/翻转。
    // 所有卖点 = 短差（sink），全量清仓只在最高级别超大卖点触发（第31课，留待实装）。
    // 删 flip/reverse/Z₂——它们是"不消费 BSP 的走势跟随 workaround"。

    /// 把实例置 dormant（保留槽位 + generation，复活时由 alloc_slot ++）。
    fn dormant_instance(&mut self, slot: usize) {
        let gen = self.instances[slot].generation;
        self.instances[slot] = TInstance::dormant(gen);
    }

    /// 收尾：全平到现金 + 归还 withdrawn。final NAV = free。
    pub fn finish(&mut self, c: f64) -> f64 {
        if let Some(_root) = self.root_slot {
            let mut free = self.free;
            for inst in self.instances.iter_mut() {
                if inst.lifecycle != TLifecycle::Dormant && inst.units > EPS {
                    rec_reduce(inst, inst.units, &mut free, c);
                }
            }
            self.free = free;
            self.free += self.withdrawn_total;
            self.withdrawn_total = 0.0;
            for slot in 0..self.instances.len() {
                if self.instances[slot].lifecycle != TLifecycle::Dormant {
                    self.dormant_instance(slot);
                }
            }
            self.root_slot = None;
        }
        self.last_close = c;
        self.free
    }
}

impl Default for TRoot {
    fn default() -> Self {
        Self::new(crate::trading::types::INITIAL_CAPITAL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn up_node(s: i64, e: i64) -> TrendNode {
        TrendNode::new(s, e, 90.0, 110.0, Direction::Up)
    }
    fn down_node(s: i64, e: i64) -> TrendNode {
        TrendNode::new(s, e, 90.0, 110.0, Direction::Down)
    }

    // ──────────────── enter / 守恒 ────────────────

    #[test]
    fn enter_首仓全仓建核心_tw中性() {
        let mut r = TRoot::new(100_000.0);
        let slot = r.enter(up_node(0, 10), 100.0, 0).unwrap();
        assert_eq!(r.instances[slot].direction, Polarity::Long);
        assert!((r.instances[slot].units - 1000.0).abs() < 1e-6, "全仓 1000 units");
        assert!((r.total_wealth(100.0) - 100_000.0).abs() < 1e-4, "建仓 TW 中性");
        assert_eq!(r.root_slot, Some(slot));
    }

    #[test]
    fn enter_方向由走势node定_down走势建空() {
        let mut r = TRoot::new(100_000.0);
        let slot = r.enter(down_node(0, 10), 100.0, 0).unwrap();
        assert_eq!(r.instances[slot].direction, Polarity::Short, "Down 走势 → Short 核心（N2 方向由节点定）");
    }

    // ──────────────── sink：子 T 骑回调持空（同股数）────────────────

    #[test]
    fn sink_父持多_回调node上spawn子T持空_同股数() {
        let mut r = TRoot::new(100_000.0);
        let p = r.enter(up_node(0, 10), 100.0, 0).unwrap();
        let u_p = r.instances[p].units;
        // 父核心走势中确认一条回调下跌 node → sink。
        let child = r.sink(p, down_node(10, 15), 100.0, 10).unwrap();
        // 父减仓到 2/3。
        assert!((r.instances[p].units - u_p * 2.0 / 3.0).abs() < 1e-6, "父减到 2/3");
        // 子 T 骑回调持空 1/3（同股数 = 父减出的 m）。
        assert_eq!(r.instances[child].direction, Polarity::Short, "子 T 持空");
        assert!((r.instances[child].units - u_p / 3.0).abs() < 1e-6, "同股数：子 T 短 = 父减出的 1/3");
        // TW 中性。
        assert!((r.total_wealth(100.0) - 100_000.0).abs() < 1e-4, "sink TW 中性");
        // 拓扑：父子互指。
        assert!(r.instances[p].child.is_some() && r.instances[child].parent.is_some());
    }

    #[test]
    fn sink_回调方向必反父向_同向node拒绝() {
        let mut r = TRoot::new(100_000.0);
        let p = r.enter(up_node(0, 10), 100.0, 0).unwrap();
        // 父 Long，给一个 Up node（非回调）→ 结构门拒绝（无门控参数，方向即结构）。
        assert!(r.sink(p, up_node(10, 15), 100.0, 10).is_none(), "同向 node 非回调，拒绝 sink");
    }

    // ──────────────── recover：子 T 平空升回（降成本）────────────────

    #[test]
    fn recover_回调完成_子T平空升回_降成本同股数() {
        let mut r = TRoot::new(100_000.0);
        let p = r.enter(up_node(0, 10), 100.0, 0).unwrap();
        let u0 = r.instances[p].units;
        let child = r.sink(p, down_node(10, 15), 110.0, 10).unwrap(); // 父高位(110)减仓+子开空@110
        let pref = r.instances[p].self_ref(p);
        let cref = r.instances[child].self_ref(child);
        // 回调走势完成（底背驰）@90 < 起点110 → 子 T 平空(@90 降成本)+父升回。
        assert!(r.recover(pref, cref, 90.0, 20));
        // 子 T 注销。
        assert!(!r.instances[child].is_active(), "子 T 平空后 dormant");
        // 父核心恒仓恢复（同股数：升回 = sink 减出的 m）。
        assert!((r.instances[p].units - u0).abs() < 1e-6, "核心恒仓恢复原股数");
        // 降成本 alpha：卖110买90，TW > 纯持有（现金价差留存）。@90 收尾 free 有结余。
        let nav90 = r.nav(90.0);
        assert!(nav90 > u0 * 90.0 + 1.0, "短差价差使 NAV 超纯持有（降成本）");
        // 短差腿 pnl 记账（子 T Short 平空）。
        assert!(r.short_leg_pnl > 0.0, "高开低平短差腿盈利");
    }

    #[test]
    #[should_panic(expected = "结构检测 bug")]
    fn recover_free不足_fail_loud() {
        // §8.1：回调"终点高于起点"（卖90买110，非真回调）→ 同股数回补 free 不足 → panic。
        let mut r = TRoot::new(100_000.0);
        let p = r.enter(up_node(0, 10), 100.0, 0).unwrap();
        let child = r.sink(p, down_node(10, 15), 90.0, 10).unwrap(); // 低位(90)减仓+子开空@90
        let pref = r.instances[p].self_ref(p);
        let cref = r.instances[child].self_ref(child);
        // 大幅拉走 free 制造不足：先把父再 sink 抽干 free 不现实；直接用高价回补触发 assert。
        // 回补@200（终点>起点，非回调）：同股数回补需 m·200，远超 sink@90 回笼的现金 → fail-loud。
        r.recover(pref, cref, 200.0, 20);
    }

    // ──────────────── spawn / flip（base case）────────────────

    #[test]
    fn spawn_涌现更高走势_relabel继承持仓_方向一致() {
        let mut r = TRoot::new(100_000.0);
        let p = r.enter(up_node(5, 10), 100.0, 0).unwrap();
        let u0 = r.instances[p].units;
        let tw = r.total_wealth(100.0);
        // 涌现更高 Up 走势（start_bar 更早=包含 root，方向一致）→ spawn relabel 升格。
        let np = r.spawn(up_node(0, 20), 100.0, 20).unwrap();
        assert_eq!(r.root_slot, Some(np), "新实例成为 root");
        assert_ne!(np, p, "relabel 到新槽");
        assert!((r.instances[np].units - u0).abs() < 1e-9, "持仓整体继承（relabel 非新建零仓）");
        assert_eq!(r.instances[np].direction, Polarity::Long);
        assert!(!r.instances[p].is_active(), "老 root → dormant");
        assert!((r.total_wealth(100.0) - tw).abs() < 1e-9, "spawn relabel 零现金 TW 中性");
    }

    #[test]
    fn spawn_继承回调链_子T_parent重指向() {
        let mut r = TRoot::new(100_000.0);
        let p = r.enter(up_node(5, 10), 100.0, 0).unwrap();
        let child = r.sink(p, down_node(10, 15), 100.0, 10).unwrap(); // root 有回调子 T
        let np = r.spawn(up_node(0, 20), 100.0, 20).unwrap();
        // 回调链继承：新 root.child 指向原子 T，子 T.parent 重指向新 root。
        assert!(r.instances[np].child.is_some(), "新 root 继承回调链");
        assert_eq!(r.instances[child].parent.unwrap().slot, np, "子 T.parent 重指向新 root");
        assert!(r.instances[child].is_active(), "子 T 短头存活");
    }

    #[test]
    fn spawn_涌现反向_返回None应走flip() {
        let mut r = TRoot::new(100_000.0);
        let _p = r.enter(up_node(0, 10), 100.0, 0).unwrap();
        // 涌现 Down 走势（反核心向）= 29课情况三 → spawn 拒绝（返回 None），调用方走 flip。
        assert!(r.spawn(down_node(0, 20), 100.0, 20).is_none(), "反向涌现不 spawn（§3.5 第三态）");
    }

    // flip/promote 测试已删（编排者裁决 C1：核心永不翻转，删 flip/promote）。

    #[test]
    fn 三阶段方向对称_空头核心低位平空降cost_basis() {
        // 编排者终裁 2026-06-20（零 workaround，方向对称）：空头核心也降成本，镜像多头。
        let mut r = TRoot::new(100_000.0);
        let p = r.enter(down_node(0, 10), 100.0, 0).unwrap(); // Down 走势 → Short 核心
        assert_eq!(r.instances[p].direction, Polarity::Short);
        let cb0 = r.instances[p].cost_basis;
        // 反弹（Up）回调 → sink：空头核心低位(50)平掉 1/3 → realized=m(basis−c)>0 降 cost_basis（镜像多头高卖）。
        r.sink(p, up_node(10, 15), 50.0, 10).unwrap();
        let cb1 = r.instances[p].cost_basis;
        assert!(cb1 < cb0 - 1.0, "空头核心低位平空降 cost_basis：{cb1} < {cb0}（方向对称，非 short_leg_pnl 平铺）");
        assert_eq!(r.instances[p].phase, RecStage::CostReduction);
    }

    // ──────────────── 三阶段 + 收尾守恒 ────────────────

    #[test]
    fn 三阶段_父核心高位卖出降cost_basis() {
        let mut r = TRoot::new(100_000.0);
        let p = r.enter(up_node(0, 10), 100.0, 0).unwrap();
        let cb0 = r.instances[p].cost_basis;
        r.sink(p, down_node(10, 15), 150.0, 10).unwrap(); // 高位(150)卖核心 → realized 降 cost_basis
        let cb1 = r.instances[p].cost_basis;
        assert!(cb1 < cb0 - 1.0, "高位卖出降 cost_basis：{cb1} < {cb0}");
        assert_eq!(r.instances[p].phase, RecStage::CostReduction, "单次远未退本金");
    }

    #[test]
    fn 收尾全平_final_nav守恒() {
        let mut r = TRoot::new(100_000.0);
        let p = r.enter(up_node(0, 10), 100.0, 0).unwrap();
        r.sink(p, down_node(10, 15), 100.0, 10).unwrap();
        let fin = r.finish(100.0);
        assert!((fin - 100_000.0).abs() < 1e-4, "同价收尾 final_nav 守恒");
        assert_eq!(r.n_active(), 0, "收尾全平");
    }

    #[test]
    fn 空树finish不panic() {
        let mut r = TRoot::new(100_000.0);
        assert!((r.finish(100.0) - 100_000.0).abs() < 1e-9);
    }
}
