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

use super::prove_guards::{
    prove_relabel_invariant, prove_sigma_quota, prove_sink_descends, OpTrigger, ProveGuards,
};
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

// ════════════════════════════ 引擎配置（变体闸门，受控实验单进程多变体）════════════════════════════

/// 引擎变体配置（OFF / ANCHOR / NEST 受控对照，单进程内可显式构造多变体——避免 env 串扰）。
///
/// - **OFF** = `from_env()` 默认（trend_done_clear ON、anchor OFF、nest OFF）= 当前 main 独立腿基线。
/// - **ANCHOR** = OFF + `enable_hold_anchor`（552号趋势底仓）。
/// - **NEST** = `enable_nest`（命题4 读法乙）：旁路 route_bsp/cc 锚，大级别背驰段闸门 a0 区间套定位翻转。
///
/// bit-exact 契约：`enable_nest=false ∧ enable_hold_anchor=false` ⇒ on_bar/route_bsp/sink/drain/enter
/// 行为与 OFF 逐字一致（新逻辑全部 flag 门控、anchor 恒 0）。
#[derive(Debug, Clone, Copy)]
pub struct EngineConfig {
    pub enable_earning: bool,
    pub enable_three_stage: bool,
    pub enable_trend_done_clear: bool,
    pub enable_hold_anchor: bool,
    pub enable_nest: bool,
    /// **consume平空（任务22）**：次级别反核心向背驰段定位 ⇒ 平核心仓 1/3（缩短整仓长持死扣，减强牛穿仓）。
    pub enable_nest_consume: bool,
    /// **严格逐级区间套（任务22 开放轴C）**：主翻转定位点须 top..loc 逐级背驰段一致（非仅 top 武装+a0 定位）。
    pub enable_nest_strict: bool,
    /// **读法B/读法乙递归（任务18 编排者修正）**：每级别独立腿多重赋格，消费 d_top 链 switch。旁路 instances 路径。
    pub enable_reading_b: bool,
    /// 读法B 触发器：false=走势完成链（556 读法B 基线）/ true=背驰段链（读法乙递归，触发更频繁可能解冻顶层）。
    pub reading_b_diverge: bool,
}

impl EngineConfig {
    /// 从 env 读（= 当前 main 行为，保 OFF 基线）。
    pub fn from_env() -> Self {
        EngineConfig {
            enable_earning: std::env::var("T_NO_EARNING").is_err(),
            enable_three_stage: std::env::var("T_NO_THREESTAGE").is_err(),
            enable_trend_done_clear: std::env::var("T_NO_TREND_DONE_CLEAR").is_err(),
            enable_hold_anchor: std::env::var("HOLD_ANCHOR").is_ok(),
            enable_nest: std::env::var("T_NEST_READING_B").is_ok(),
            enable_nest_consume: std::env::var("T_NEST_CONSUME").is_ok(),
            enable_nest_strict: std::env::var("T_NEST_STRICT").is_ok(),
            enable_reading_b: std::env::var("T_READING_B").is_ok(),
            reading_b_diverge: std::env::var("T_READING_B_DIVERGE").is_ok(),
        }
    }
    /// OFF 基线（trend_done_clear ON，anchor/nest OFF）。
    pub fn off() -> Self {
        let mut c = Self::from_env();
        c.enable_hold_anchor = false;
        c.enable_nest = false;
        c.enable_nest_consume = false;
        c.enable_nest_strict = false;
        c.enable_reading_b = false;
        c.reading_b_diverge = false;
        c
    }
    /// 读法B 基线（556：每级别独立腿 + 走势完成链触发）。
    pub fn reading_b_dtop() -> Self {
        let mut c = Self::off();
        c.enable_reading_b = true;
        c.reading_b_diverge = false;
        c
    }
    /// 读法乙递归（编排者修正：每级别独立腿 + 背驰段链触发，隔离换触发器效果）。
    pub fn reading_b_diverge() -> Self {
        let mut c = Self::off();
        c.enable_reading_b = true;
        c.reading_b_diverge = true;
        c
    }
    /// ANCHOR（OFF + 趋势底仓 552号）。
    pub fn anchor() -> Self {
        let mut c = Self::off();
        c.enable_hold_anchor = true;
        c
    }
    /// NEST（命题4 读法乙，旁路 cc 锚）。
    pub fn nest() -> Self {
        let mut c = Self::off();
        c.enable_nest = true;
        c
    }
    /// NEST + 严格逐级区间套（任务22 开放轴C，无 consume）——隔离 strict 贡献。
    pub fn nest_strict() -> Self {
        let mut c = Self::nest();
        c.enable_nest_strict = true;
        c
    }
    /// NEST + consume平空（任务22：次级别买点平空缩短持仓）。
    pub fn nest_consume() -> Self {
        let mut c = Self::nest();
        c.enable_nest_consume = true;
        c
    }
    /// NEST + consume + 严格逐级区间套（任务22 完整形态）。
    pub fn nest_consume_strict() -> Self {
        let mut c = Self::nest_consume();
        c.enable_nest_strict = true;
        c
    }
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

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
    /// **趋势底仓下限**（552号 HOLD_ANCHOR）：`sink`/`drain` 配额不作用于此份额——趋势底仓死扣吃大
    /// 趋势，只让机动仓（units−anchor）做次级别短差。`enter` 设 = m×(1−MOBILE_FRAC)，flag off 时恒 0
    /// ⇒ 机动仓=全仓 ⇒ 配额=units/3 与 OFF 逐字一致（bit-exact）。
    pub anchor: f64,
}

impl TInstance {
    fn idle(level: usize) -> Self {
        TInstance {
            node: TrendNode::new(0, 0, 0.0, 0.0, Direction::Up),
            level,
            direction: Polarity::Long,
            units: 0.0,
            basis: f64::NAN,
            anchor: 0.0,
        }
    }
    pub fn is_active(&self) -> bool {
        self.units > EPS
    }
}

// ════════════════════════════ 读法B/读法乙递归：每级别独立腿（多重赋格，任务18 编排者修正）════════════════════════════

/// **每级别独立腿**（读法B/读法乙递归，移植自 4d6856dc75）：级别 k 的腿骑 `levels[k]` 走势，
/// 消费该级别 `d_top[k]`（走势完成链 OR 背驰段链）switch。每级别独立 = leg_3 骑 L3，leg_4 骑 L4，
/// 互不干预方向（547 隔离：每级别翻自己，删 `nearest_active_parent` 跨级路由）。
#[derive(Debug, Clone, Copy)]
pub struct Leg {
    pub level: usize,
    pub direction: Polarity,
    pub units: f64,
    pub basis: f64,
    pub riding_node: TrendNode,
    pub active: bool,
}

impl Leg {
    fn idle(level: usize) -> Self {
        Leg {
            level,
            direction: Polarity::Long,
            units: 0.0,
            basis: f64::NAN,
            riding_node: TrendNode::new(0, 0, 0.0, 0.0, Direction::Up),
            active: false,
        }
    }
    fn fingerprint(&self) -> (u64, bool, bool) {
        (self.units.to_bits(), self.direction == Polarity::Long, self.active)
    }
}

/// **prove_leg_isolation（547 隔离，L0 结构 panic 守卫）**：`g(k)` 只写 `legs[k]`，断言 `legs[j≠k]`
/// 在 g(k) 前后逐位不变（547 病灶「低级别信号越级翻动高级别主力」的结构否定）。
fn prove_leg_isolation(
    fp_pre: &[(u64, bool, bool); MAX_LEVEL],
    fp_post: &[(u64, bool, bool); MAX_LEVEL],
    k: usize,
) {
    for j in 0..MAX_LEVEL {
        if j == k {
            continue;
        }
        assert_eq!(
            fp_pre[j], fp_post[j],
            "547 隔离违反：g({k}) 改动了别级腿 legs[{j}]（每级别独立腿只许 g(k) 写 legs[k]）"
        );
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
    /// 该 level 是否新增 **type1 买点**（底背驰 = 下跌走势终完美，fresh）。
    /// 走势完成信号（campaign 边界重定义，546号死锁解锁）：与 `buy` 同源去重的**子集**（仅 type1）
    /// ⇒ 与 flat `TSignalView::t1buy` **bit-exact 对称**（同一结构完成事件，仅 level/ladder 索引偏移）。
    pub t1buy: [bool; MAX_LEVEL],
    /// 该 level 是否新增 **type1 卖点**（顶背驰 = 上涨走势终完美，fresh）。
    pub t1sell: [bool; MAX_LEVEL],
    /// 各 level 当前走势节点（enter/ascend/sink 骑节点用；None=该级无走势）。
    pub nodes: [Option<TrendNode>; MAX_LEVEL],
    /// T 迭代涌现上界 (level, 操作极性)——自下而上仓位涌现（= flat emergent_top）。None=本 bar 不升级。
    pub emergent_top: Option<(usize, Polarity)>,
    /// **命题4 读法乙——大级别背驰段闸门（self-top-down 区间套前提）**。
    /// 最高级别走势进入**背驰段**（`divergence::trend_candidate`：结构∧MACD 双确认，**未创新高**=
    /// 未走势完成，严格⊊type1）时置位为**操作极性**：上涨顶背驰段→Short（顶部，卖点 close+做空），
    /// 下跌底背驰段→Long（底部，买点 cover+做多）。None=最高级别未进入背驰段。
    /// 源头审计 src-prop13（第27课区间套）：区间套前提 = 大级别**背驰段**（非走势完成）。
    pub top_diverge: Option<Polarity>,
    /// 最高级别**当前走势几何方向**（NEST 反转去武装：armed 与当前 top 方向不一致 ⇒ top 已反转 ⇒ 清 armed）。
    pub top_trend_dir: Option<Direction>,
    /// **每级别背驰段操作极性**（多重赋格 + consume平空 + 严格逐级区间套，任务22）：
    /// `level_diverge[k]` = 级别 k 当前走势进入背驰段时的操作极性（顶背驰段→Short / 底背驰段→Long），None=该级未进背驰段。
    /// consume平空用：次级别（k<core）反核心向背驰段 ⇒ 平核心仓（缩短持仓）。严格逐级用：定位点须 top..loc 逐级背驰段一致。
    pub level_diverge: [Option<Polarity>; MAX_LEVEL],
    /// **每级别 d_top 区间套链贯通真顶/真底**（任务18 编排者修正：每级别独立腿多重赋格触发器）。
    /// `divergence::d_top(k, ..., use_diverge)`——`use_diverge=false`=走势完成链（556 读法B）/`true`=背驰段链（读法乙）。
    /// 读法B 路径（`enable_reading_b`）每级别独立腿 `legs[k]` 消费 `d_top[k]` switch（close+反向 open）。
    pub d_top: [bool; MAX_LEVEL],
}

impl LevelView {
    pub fn empty() -> Self {
        LevelView {
            buy: [false; MAX_LEVEL],
            sell: [false; MAX_LEVEL],
            t1buy: [false; MAX_LEVEL],
            t1sell: [false; MAX_LEVEL],
            nodes: [None; MAX_LEVEL],
            emergent_top: None,
            top_diverge: None,
            top_trend_dir: None,
            level_diverge: [None; MAX_LEVEL],
            d_top: [false; MAX_LEVEL],
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
    /// 核心走势完成清仓开关（546号死锁解锁的 A/B 消融门）：env `T_NO_TREND_DONE_CLEAR` 置位 ⇒ false
    /// （走势完成不清仓 = 死锁基线），默认 true。与 `T_NO_EMERGENCE` 等同类 eval 工具，flat/rec 对称。
    enable_trend_done_clear: bool,
    /// **ANCHOR**（552号 HOLD_ANCHOR）：enter 划趋势底仓 anchor=m×2/3，sink/drain 配额只作机动仓。OFF=false。
    enable_hold_anchor: bool,
    /// **NEST**（命题4 读法乙）：旁路 route_bsp/cc 锚，大级别背驰段闸门 a0 区间套定位翻转。OFF=false。
    enable_nest: bool,
    /// **consume平空（任务22）**：次级别反核心向背驰段 ⇒ 平核心仓 1/3。
    enable_nest_consume: bool,
    /// **严格逐级区间套（任务22 开放轴C）**：主翻转定位点须 top..loc 逐级背驰段一致。
    enable_nest_strict: bool,
    /// NEST armed 操作极性（大级别背驰段武装，跨重跑持续至 top 反转 / 翻转消费）。None=未武装。
    nest_armed_op: Option<Polarity>,
    /// NEST 当前背驰段窗口是否已翻转（**每窗口仅翻一次** = 读法乙区间套定位一个转折点，非读法甲全 a0 穷尽）。
    /// 新窗口（top_diverge 极性变 / top 反转去武装）⇒ false；翻转 ⇒ true。
    nest_consumed: bool,
    /// **读法B/读法乙递归（任务18）**：每级别独立腿 + 触发器。enable_reading_b=true ⇒ on_bar 走 consume_legs。
    enable_reading_b: bool,
    reading_b_diverge: bool,
    /// 每级别独立腿（legs[k] 骑 levels[k] 走势消费 d_top[k]）。
    legs: Vec<Leg>,
    /// 读法B 每级别 per-level realized pnl（验收哪级别腿赚/亏）。
    pub per_level_long_pnl: [f64; MAX_LEVEL],
    pub per_level_short_pnl: [f64; MAX_LEVEL],
    /// 读法B 每级别腿切换次数（d_top[k] 驱动 close+reopen，纯观测——解 556 顶层腿是否冻结）。
    pub leg_switches_by_level: [u64; MAX_LEVEL],
    pub leg_opens_by_level: [u64; MAX_LEVEL],

    // ── 观测计数（纯诊断）──
    pub n_enters: u64,
    pub n_sinks: u64,
    pub n_recovers: u64,
    pub n_drains: u64,
    pub n_flips: u64,
    /// 核心走势完成清仓次数（546号死锁解锁路径触发计数，纯观测）。
    pub n_trend_done_clears: u64,
    pub n_ascends: u64,
    /// **NEST 翻转次数**（命题4 读法乙：大级别背驰段闸门 a0 定位翻转）。诊断 556 顶层是否解冻。
    pub n_nest_flips: u64,
    /// **NEST consume平空次数**（任务22：次级别反核心向背驰段平核心仓 1/3）。诊断是否缩短长持死扣。
    pub n_nest_consumes: u64,
    /// consume平空累计 realized（缩短持仓的平仓 pnl，验收减穿仓）。
    pub nest_consume_pnl: f64,
    /// NEST 逐笔（is_short, entry_bar, entry_px, exit_bar, exit_px, realized_pnl）——539 做空腿逐笔验收。
    pub nest_trades: Vec<(bool, i64, f64, i64, f64, f64)>,
    pub n_emergence_upgrades: u64,
    /// 编排者排查 2026-06-21：emergence_upgrade 统计——核心低于涌现级别本可升级的次数 / 方向不匹配跳过。
    pub n_emergence_attempts: u64,
    pub n_emergence_skipped_dir: u64,
    /// 核心 flip 序列（bar, from_dir, to_dir, j_level, is_buy）：查核心被低级别 BSP ping-pong。
    pub flip_log: Vec<(i64, Polarity, Polarity, usize, bool)>,
    /// 当前 bar（on_bar 每 bar 设，flip_log 用）。
    pub cur_bar: i64,
    /// 编排者排查 2026-06-21：sink/recover 路由对称性——per-level sink/recover + 买点路由分类
    /// （查为什么 recover(1348)<sink(2671)：哪些买点没被消费为 recover）。
    pub sink_by_level: [u64; MAX_LEVEL],
    pub recover_by_level: [u64; MAX_LEVEL],
    pub buy_recover: u64, // 父多+买点+j active Short → recover
    pub buy_noop: u64,    // 父多+买点+j 无短差 → no-op（浪费的买点）
    pub buy_sink: u64,    // 父空+买点 → sink（核心 Short 时减仓）
    pub buy_core: u64,    // 核心级买点 → enter/ascend/flip
    pub n_liquidations: u64,
    pub n_capital_recovered: u64,
    pub n_earning_deploys: u64,
    pub short_leg_pnl: f64,
    /// 短差腿 per-level realized（验收：哪个级别短差赚/亏，编排者 per-level P&L 追踪）。
    pub short_pnl_by_level: [f64; MAX_LEVEL],
    /// 强平 episode 诊断（编排者：查回补失败=空头没匹配买点被强平）：
    /// (level, 开空节点起点 bar, 开空价 basis, 强平 bar, 强平价, 是否空头)。
    pub liq_log: Vec<(usize, i64, f64, i64, f64, bool)>,
    /// 诊断（编排者 2026-06-21）：强平时三阶段快照 (stage_id 0=CostRed/1=CapRec/2=Earn,
    /// core_cost_basis, withdrawn, notional_in, nav)。查降成本/退本金保护是否生效 + 全仓 vs 逐仓。
    pub liq_snapshot: Vec<(u8, f64, f64, f64, f64)>,
    pub earning_units_added: f64,
    pub max_core_gain_x1000: u64,

    /// prove 守卫族（编排者裁决 2026-06-21）：BSP 触发归因（panic）+ sink/recover 平衡 / per-level
    /// 短差 pnl / 核心方向匹配（观测计数）。见 `prove_guards.rs`。
    guards: ProveGuards,
}

impl TRoot {
    /// 默认从 env 构造（= 当前 main 行为）。
    pub fn new(initial_capital: f64) -> Self {
        Self::new_with_config(initial_capital, EngineConfig::from_env())
    }

    /// 显式配置构造（受控实验：OFF / ANCHOR / NEST 单进程多变体）。
    pub fn new_with_config(initial_capital: f64, cfg: EngineConfig) -> Self {
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
            enable_earning: cfg.enable_earning,
            enable_three_stage: cfg.enable_three_stage,
            enable_trend_done_clear: cfg.enable_trend_done_clear,
            enable_hold_anchor: cfg.enable_hold_anchor,
            enable_nest: cfg.enable_nest,
            enable_nest_consume: cfg.enable_nest_consume,
            enable_nest_strict: cfg.enable_nest_strict,
            nest_armed_op: None,
            nest_consumed: false,
            enable_reading_b: cfg.enable_reading_b,
            reading_b_diverge: cfg.reading_b_diverge,
            legs: (0..MAX_LEVEL).map(Leg::idle).collect(),
            per_level_long_pnl: [0.0; MAX_LEVEL],
            per_level_short_pnl: [0.0; MAX_LEVEL],
            leg_switches_by_level: [0; MAX_LEVEL],
            leg_opens_by_level: [0; MAX_LEVEL],
            n_enters: 0,
            n_sinks: 0,
            n_recovers: 0,
            n_drains: 0,
            n_flips: 0,
            n_trend_done_clears: 0,
            n_ascends: 0,
            n_nest_flips: 0,
            n_nest_consumes: 0,
            nest_consume_pnl: 0.0,
            nest_trades: Vec::new(),
            n_emergence_upgrades: 0,
            n_emergence_attempts: 0,
            n_emergence_skipped_dir: 0,
            flip_log: Vec::new(),
            cur_bar: 0,
            sink_by_level: [0; MAX_LEVEL],
            recover_by_level: [0; MAX_LEVEL],
            buy_recover: 0,
            buy_noop: 0,
            buy_sink: 0,
            buy_core: 0,
            n_liquidations: 0,
            n_capital_recovered: 0,
            n_earning_deploys: 0,
            short_leg_pnl: 0.0,
            short_pnl_by_level: [0.0; MAX_LEVEL],
            liq_log: Vec::new(),
            liq_snapshot: Vec::new(),
            earning_units_added: 0.0,
            max_core_gain_x1000: 0,
            guards: ProveGuards::new(MAX_LEVEL),
        }
    }

    /// prove 守卫只读访问（验收报告/测试）。
    pub fn guards(&self) -> &ProveGuards {
        &self.guards
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
        // 读法B/读法乙递归：每级别独立腿（instances 与 legs 按模式互斥，无双计）。
        for leg in &self.legs {
            if leg.units > 0.0 {
                v += match leg.direction {
                    Polarity::Long => leg.units * c,
                    Polarity::Short => -leg.units * c,
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
        // campaign 终点（核心走势完成）：prove_sink_recover_balance + prove_per_level_pnl（观测）。
        self.guards.campaign_end();
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
        // ANCHOR（552号 HOLD_ANCHOR）：核心建仓即划趋势底仓 = m×(1−MOBILE_FRAC)=m×2/3，机动仓 = m/3。
        // OFF（flag off）时 anchor=0 ⇒ 机动仓=全仓 ⇒ 配额=units/3 与 OFF 逐字一致（bit-exact）。
        self.instances[level].anchor = if self.enable_hold_anchor { m * (1.0 - MOBILE_FRAC) } else { 0.0 };
        self.notional_in = spent;
        self.core_cost_basis = c;
        self.campaign_entry_cost = c;
        self.stage = RecStage::CostReduction;
        self.earning_cash = 0.0;
        self.n_enters += 1;
        // prove_bsp_triggers_operation（panic）+ campaign 起点基线。
        self.guards.note_op("enter");
        self.guards.campaign_start();
    }

    /// **clear_all**（= flat clear_all）：全塔平仓到现金 + reset_campaign。
    fn clear_all(&mut self, c: f64) {
        self.guards.note_op("clear_all"); // prove_bsp_triggers_operation（panic）
        let mut free = self.free;
        for k in 0..MAX_LEVEL {
            let u = self.instances[k].units;
            if u > 1e-12 {
                rec_reduce(&mut self.instances[k], u, &mut free, c);
            }
            // ANCHOR：全塔平仓 ⇒ 无趋势底仓存活，清 anchor 防僵尸残值。OFF 时 anchor 恒 0，置 0 = no-op（bit-exact）。
            self.instances[k].anchor = 0.0;
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
        let (lu_pre, su_pre) = self.exposure(); // 移植守卫（A5）：relabel 前敞口快照。
        let mut moved = self.instances[from];
        moved.level = to;
        moved.node = node;
        self.instances[to] = moved;
        self.instances[from] = TInstance::idle(from);
        let (lu_post, su_post) = self.exposure();
        prove_relabel_invariant(lu_pre, su_pre, lu_post, su_post); // ascend 是级别重标定非加仓，敞口必不变。
        self.n_ascends += 1;
        self.guards.note_op("ascend"); // prove_bsp_triggers_operation（panic；BSP 或 emergence 触发）
    }

    /// **emergence_upgrade**（= flat emergence_upgrade）：自下而上涌现，核心同向 → ascend 升级到 target。
    /// ascend 逻辑 = flat 原版（同向 + cc<target_level 才 ascend）。计数为纯观测（编排者排查 2026-06-21）。
    fn emergence_upgrade(&mut self, target_level: usize, target_dir: Polarity, node: TrendNode) {
        self.guards.set_trigger(OpTrigger::Emergence); // 涌现是 ascend 的合法非 BSP 触发源
        if target_level >= MAX_LEVEL {
            return;
        }
        if let Some(cc) = self.highest_active() {
            if cc < target_level {
                self.n_emergence_attempts += 1; // 核心低于涌现级别，本可升级
                if self.instances[cc].direction == target_dir {
                    self.ascend(cc, target_level, node);
                    self.n_emergence_upgrades += 1;
                } else {
                    self.n_emergence_skipped_dir += 1; // 方向不匹配跳过（核心 dir != 涌现 dir）
                }
            }
        }
    }

    /// **sink @ (parent→sub)**（= flat sink）：父减仓 m=u_P/3 + 次级别开 flip(d_P) 短差 m。
    fn sink(&mut self, parent: usize, sub: usize, sub_node: TrendNode, c: f64) {
        let u_p = self.instances[parent].units;
        let pdir = self.instances[parent].direction;
        // ANCHOR（552号）：配额作用于机动仓 u_P−anchor（趋势底仓死扣不下放），OFF 时 anchor=0 ⇒ mob_base=u_P（bit-exact）。
        let mob_base = if self.enable_hold_anchor {
            (u_p - self.instances[parent].anchor).max(0.0)
        } else {
            u_p
        };
        let m = quota(mob_base);
        if !(m > 1e-12 && m.is_finite()) || m > u_p + 1e-9 {
            return;
        }
        let mob = flip_pol(pdir);
        if self.instances[sub].is_active() && self.instances[sub].direction != mob {
            return;
        }
        // 同资本 sizing（编排者裁决 2026-06-21）：开空 units = reduce 释放的资本金额（m×parent.basis）
        // 在当前价 c 开空 = m×pb/c。价越高→同资本开的空头越少→牛市空头累积减轻、全仓 NAV 不易被拖到 0。
        // pb 必须在 reduce 前捕获（reduce 把 units 减到 ≤EPS 时会污染 basis=NaN）；表达式分组 (m*pb)/c
        // 与 flat t_engine 逐字一致保 bit-exact。
        let pb = self.instances[parent].basis;
        let short_u = if pb.is_finite() && pb > 1e-12 { m * pb / c } else { m };
        if !(short_u > 1e-12 && short_u.is_finite()) {
            return;
        }
        // 移植守卫（L0，从 spiral/fugue_v3）：区间套向心下沉 sub<parent + σ-不变配额 m=u_P×MOBILE_FRAC。
        prove_sink_descends(parent, sub, self.cur_bar);
        prove_sigma_quota(m, mob_base, sub, self.cur_bar); // anchor OFF ⇒ mob_base=u_p（bit-exact）
        let tw_pre = self.total_wealth(c);
        let mut free = self.free;
        let realized = rec_reduce(&mut self.instances[parent], m, &mut free, c);
        rec_add(&mut self.instances[sub], short_u, mob, &mut free, c);
        self.free = free;
        self.instances[sub].node = sub_node;
        self.instances[sub].level = sub;
        self.account_reduce(pdir, realized, c);
        self.n_sinks += 1;
        self.guards.note_op("sink"); // prove_bsp_triggers_operation（panic）
        self.guards.on_sink(); // prove_sink_recover_balance（campaign 内累计）
        if sub < MAX_LEVEL {
            self.sink_by_level[sub] += 1; // 诊断：per-level sink
        }
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
        // 同资本反算（编排者裁决 2026-06-21）：平空释放名义资本 = m×sub.basis（= sink 时下放的核心资本）；
        // 归还核心 units = 该资本 / parent.basis ⟹ give = m×sb/pb。parent.basis 不变时 give=m（核心恢复原
        // 股数），被 deploy_earning 降本后按资本恢复（give>m）。sb 须在 reduce(sub) 前捕获（reduce 零化
        // sub→basis=NaN）；表达式分组 (m*sb)/pb 与 flat t_engine 逐字一致保 bit-exact。
        let sb = self.instances[sub].basis;
        let pb = self.instances[parent].basis;
        let give = if pb.is_finite() && pb > 1e-12 { m * sb / pb } else { m };
        if !(give > 1e-12 && give.is_finite()) {
            return;
        }
        // 移植守卫（L0）：recover 升回与 sink 向心配对，同守 sub<parent（次级别走势完成升回父级）。
        prove_sink_descends(parent, sub, self.cur_bar);
        let tw_pre = self.total_wealth(c);
        let mut free = self.free;
        let realized = rec_reduce(&mut self.instances[sub], m, &mut free, c);
        rec_add(&mut self.instances[parent], give, pdir, &mut free, c);
        self.free = free;
        self.account_reduce(mob, realized, c);
        if sub < MAX_LEVEL {
            self.short_pnl_by_level[sub] += realized; // 短差腿 per-level（验收哪级别亏）
        }
        self.n_recovers += 1;
        self.guards.note_op("recover"); // prove_bsp_triggers_operation（panic）
        self.guards.on_recover(sub, realized); // prove_sink_recover_balance + prove_per_level_pnl
        if sub < MAX_LEVEL {
            self.recover_by_level[sub] += 1; // 诊断：per-level recover
        }
        if pdir == Polarity::Long {
            self.deploy_earning(parent, c);
        }
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **drain @ j**（= flat drain）：子级持同父向遗留仓 → 反父向 BSP 减暴露 1/3（不翻转）。
    fn drain(&mut self, j: usize, c: f64) {
        let u = self.instances[j].units;
        // ANCHOR（552号）：drain 目标 j 有活跃祖先（route_bsp Some(p) 分支），非核心 ⇒ enter 未给 j 划 anchor
        // ⇒ anchor 结构性恒 0 ⇒ mob_base=u（flag ON/OFF 皆与 OFF 逐字一致）。用机动仓基准是 σ-不变配额一致形式。
        let mob_base = if self.enable_hold_anchor {
            (u - self.instances[j].anchor).max(0.0)
        } else {
            u
        };
        let m = quota(mob_base);
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        let jdir = self.instances[j].direction;
        // 移植守卫（L0）：drain 减暴露配额 σ-不变（m=mob_base×MOBILE_FRAC，级别无关；mob_base=u 当 anchor=0）。
        prove_sigma_quota(m, mob_base, j, self.cur_bar);
        let tw_pre = self.total_wealth(c);
        let mut free = self.free;
        let realized = rec_reduce(&mut self.instances[j], m, &mut free, c);
        self.free = free;
        self.account_reduce(jdir, realized, c);
        self.n_drains += 1;
        self.guards.note_op("drain"); // prove_bsp_triggers_operation（panic）
        if jdir == Polarity::Short {
            self.guards.on_short_pnl(j, realized); // prove_per_level_pnl（同向遗留短差腿平仓）
        }
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **route_bsp**（= flat route_bsp）：level j 的 BSP 分派。
    fn route_bsp(&mut self, j: usize, is_buy: bool, node: TrendNode, c: f64) {
        self.guards.set_trigger(OpTrigger::Bsp); // 本路由触发的所有原子操作归因 BSP
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
                    if is_buy {
                        self.buy_sink += 1; // 诊断：父空+买点 → sink（减核心 Short）
                    }
                    if !self.instances[j].is_active() || self.instances[j].direction == mob {
                        self.sink(p, j, node, c);
                    } else {
                        self.drain(j, c);
                    }
                } else {
                    // 同父向 BSP：recover（j 持短差则平整条升回）；否则 no-op（不 pyramid）。
                    if self.instances[j].is_active() && self.instances[j].direction == mob {
                        if is_buy {
                            self.buy_recover += 1; // 诊断：父多+买点 → recover
                        }
                        self.recover(p, j, c);
                    } else if is_buy {
                        self.buy_noop += 1; // 诊断：父多+买点+j 无短差 → no-op（浪费的买点）
                    }
                }
            }
            // ── 核心级（无活跃祖先）：唯一独立翻转点 ──
            None => {
                if is_buy {
                    self.buy_core += 1; // 诊断：核心级买点 → enter/ascend/flip（不 recover）
                }
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
                            // 诊断（编排者排查 2026-06-21）：核心 flip 序列(bar/from→to/触发 BSP 级别+买卖)。
                            self.flip_log.push((self.cur_bar, cdir, dir, j, is_buy));
                            self.clear_all(c);
                            self.enter(j, dir, node, c);
                            self.n_flips += 1;
                        }
                    }
                }
            }
        }
    }

    // ──────────────── 读法B/读法乙递归：每级别独立腿骑走势消费 d_top（任务18 编排者修正）────────────────

    /// 几何塔配额（编排者裁决）：base=free×2/3，级别 k = base/λ^(top−k)，收敛 base×3/2≤free 恒仓不加杠杆。
    fn geom_tower_quota(&self, k: usize, top: usize, free_pool: f64, c: f64) -> f64 {
        if c <= 0.0 || free_pool <= 0.0 || k > top {
            return 0.0;
        }
        let base = free_pool * (2.0 / 3.0);
        let depth = top - k;
        let notional = base * MOBILE_FRAC.powi(depth as i32);
        notional / c
    }

    fn snapshot_fingerprints(&self) -> [(u64, bool, bool); MAX_LEVEL] {
        let mut fp = [(0u64, false, false); MAX_LEVEL];
        for k in 0..MAX_LEVEL {
            fp[k] = self.legs[k].fingerprint();
        }
        fp
    }

    /// **open_leg**（ride 分支）：级别 k 空腿按走势方向从单一 free 池领几何塔配额建仓。只写 legs[k]（547 隔离）。
    fn open_leg(&mut self, k: usize, dir: Polarity, node: TrendNode, top: usize, c: f64) {
        let fp_pre = self.snapshot_fingerprints();
        let m = self.geom_tower_quota(k, top, self.free.max(0.0), c);
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        let tw_pre = self.total_wealth(c);
        match dir {
            Polarity::Long => self.free -= m * c,
            Polarity::Short => self.free += m * c,
        }
        self.legs[k] = Leg { level: k, direction: dir, units: m, basis: c, riding_node: node, active: true };
        self.leg_opens_by_level[k] += 1;
        self.guards.note_op("open_leg");
        prove_leg_isolation(&fp_pre, &self.snapshot_fingerprints(), k);
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **close_leg**（switch 前半）：平 legs[k] 全量到现金，realized 记 per_level pnl。只写 legs[k]（547 隔离）。
    fn close_leg(&mut self, k: usize, c: f64) -> f64 {
        if !self.legs[k].active {
            return 0.0;
        }
        let fp_pre = self.snapshot_fingerprints();
        let leg = self.legs[k];
        let pnl = match leg.direction {
            Polarity::Long => leg.units * (c - leg.basis),
            Polarity::Short => leg.units * (leg.basis - c),
        };
        let tw_pre = self.total_wealth(c);
        match leg.direction {
            Polarity::Long => {
                self.free += leg.units * c;
                self.per_level_long_pnl[k] += pnl;
            }
            Polarity::Short => {
                self.free -= leg.units * c;
                self.per_level_short_pnl[k] += pnl;
            }
        }
        self.legs[k] = Leg::idle(k);
        self.guards.note_op("close_leg");
        prove_leg_isolation(&fp_pre, &self.snapshot_fingerprints(), k);
        self.prove_tw_neutral(tw_pre, c);
        pnl
    }

    /// **g(k)**（单算子三分支）：级别 k 腿消费 `levels[k]` 走势 + `d_top[k]`（触发器=走势完成链 OR 背驰段链）。
    /// switch：d_top[k] ∧ 腿活跃 → close+反向 open；ride：腿空 ∧ 有走势 → 按方向 open；hold：维持。
    /// **clear 只清触发级别**（547 隔离：绝不复用 clear_all，每级别独立 campaign）。
    fn g(&mut self, k: usize, view: &LevelView, top: usize, c: f64) {
        let d_top = view.d_top[k];
        if d_top && self.legs[k].active {
            let old_dir = self.legs[k].direction;
            self.guards.set_trigger(OpTrigger::Bsp);
            self.close_leg(k, c);
            let new_dir = flip_pol(old_dir);
            let node = view.nodes[k].unwrap_or_else(|| {
                TrendNode::new(self.cur_bar, self.cur_bar, c, c, match new_dir {
                    Polarity::Long => Direction::Up,
                    Polarity::Short => Direction::Down,
                })
            });
            self.open_leg(k, new_dir, node, top, c);
            self.leg_switches_by_level[k] += 1;
            return;
        }
        if !self.legs[k].active {
            if let Some(node) = view.nodes[k] {
                let dir = dir_to_polarity(node.direction);
                self.guards.set_trigger(OpTrigger::Bsp);
                self.open_leg(k, dir, node, top, c);
            }
            return;
        }
    }

    /// **consume_legs**（单算子递归 = construct iterate 的对偶）：a0→涌现上界逐级 g(k)。
    fn consume_legs(&mut self, view: &LevelView, c: f64) {
        let top = match (0..MAX_LEVEL).rev().find(|&k| view.nodes[k].is_some()) {
            Some(t) => t,
            None => return,
        };
        for k in 0..=top {
            self.g(k, view, top, c);
        }
    }

    /// 读法B 收尾（全平所有腿到现金）。
    fn finish_legs(&mut self, c: f64) {
        if !(c.is_finite() && c > 0.0) {
            return;
        }
        self.guards.set_trigger(OpTrigger::Eod);
        for k in 0..MAX_LEVEL {
            if self.legs[k].active {
                self.close_leg(k, c);
            }
        }
    }

    // ──────────────── NEST：命题4 读法乙（大级别背驰段闸门 + a0 区间套定位翻转）────────────────

    /// **nest_step**（命题4 读法乙，源头审计 src-prop13 / 第27课区间套）：
    /// 1. **大级别背驰段武装**（自顶向下前提）：`view.top_diverge` = 最高级别进入背驰段时的操作极性
    ///    （上涨顶背驰段→Short / 下跌底背驰段→Long）。背驰段 ⊊ 走势完成（区间套前提是背驰段非完成）。
    ///    武装跨重跑持续；top 反转（`top_trend_dir` 与 armed 不一致）⇒ 清武装（去陈旧武装）。
    /// 2. **a0 区间套定位**：在武装下，找**最低级别**与武装极性一致的 type1（顶背驰段找 type1_sell /
    ///    底背驰段找 type1_buy）= 区间套向下定位到的精确买卖点（最低级别 = 最接近 a0）。
    /// 3. **整仓翻转操作**：定位点 ∧ 当前持仓 ≠ 武装极性 ⇒ clear_all（close/cover）+ enter（做多/做空）
    ///    全仓。翻转后 cur==op ⇒ 同武装不再翻（幂等）；持仓骑走势直到反向武装定位。
    ///
    /// **不是读法甲**（全 a0 穷尽机械翻转）：唯有大级别背驰段武装下的 a0 定位点才翻转，背驰段是筛选闸门。
    /// **有效域边界**（不声明膨胀）：区间套中间级别逐级背驰段未逐层校验（仅 top 武装 + a0 最低定位）；
    /// 严格逐级嵌套（每级别背驰段）未实装，记为有效域边界（与 divergence.rs 条件3/序数简化同类）。
    fn nest_step(&mut self, view: &LevelView, bar: i64, c: f64) {
        // ── 1. 去陈旧武装：top 反转（当前 top 走势方向与 armed 期望方向不符）⇒ 清武装 + 开新窗口 ──
        if let (Some(armed), Some(dir)) = (self.nest_armed_op, view.top_trend_dir) {
            // armed Short 期望 top=Up；armed Long 期望 top=Down。不符 ⇒ top 已反转 ⇒ 清。
            let consistent = match dir {
                Direction::Up => Polarity::Short,
                Direction::Down => Polarity::Long,
            };
            if armed != consistent {
                self.nest_armed_op = None;
                self.nest_consumed = false; // 窗口结束，下个背驰段窗口可再翻
            }
        }
        // ── 1'. 武装/更新：最高级别进入背驰段 ⇒ armed = 操作极性。极性变 = 新窗口 ⇒ 重置 consumed ──
        if let Some(op) = view.top_diverge {
            if self.nest_armed_op != Some(op) {
                self.nest_consumed = false; // 新背驰段窗口（或方向反转）⇒ 允许一次新定位翻转
            }
            self.nest_armed_op = Some(op);
        }
        // ── 2. 主翻转尝试（top 背驰段闸门 + a0 区间套定位）──
        let flipped = self.nest_try_flip(view, bar, c);
        // ── 3. consume平空（任务22）：未翻转时，次级别反核心向背驰段 ⇒ 平核心仓 1/3（缩短长持死扣）──
        if !flipped && self.enable_nest_consume {
            self.nest_consume_step(view, bar, c);
        }
    }

    /// **主翻转**（top 背驰段武装 + a0 区间套定位 + 可选严格逐级）。返回是否翻转（供 consume 判断）。
    fn nest_try_flip(&mut self, view: &LevelView, bar: i64, c: f64) -> bool {
        let armed = match self.nest_armed_op {
            Some(op) => op,
            None => return false, // 未武装 = 大级别未进背驰段 ⇒ 不操作（556 可能冻结根源，L3 测）
        };
        if self.nest_consumed {
            return false; // 本背驰段窗口已翻过 ⇒ 持仓骑走势（读法乙：一个转折点）
        }
        let want_buy = armed == Polarity::Long; // 底背驰段武装 → 找 type1_buy；顶背驰段 → type1_sell
        let located = (0..MAX_LEVEL).find(|&k| if want_buy { view.t1buy[k] } else { view.t1sell[k] });
        let loc = match located {
            Some(k) => k,
            None => return false, // 武装但本重跑无 a0 定位点 ⇒ 等下一定位点
        };
        // ── 严格逐级区间套（任务22 开放轴C）：定位点须 loc..=top_armed_level 逐级背驰段一致 ──
        //   非仅 top 武装 + a0 定位——区间套要求每级别都处一致背驰段（嵌套校验）。top_armed_level =
        //   最高有 level_diverge 的级别（= top_diverge 来源级别）。loc..top 间任一级别非一致背驰段 ⇒ 不翻。
        if self.enable_nest_strict {
            let top_lvl = (0..MAX_LEVEL).rev().find(|&k| view.level_diverge[k].is_some());
            if let Some(tl) = top_lvl {
                let cascade_ok = (loc..=tl).all(|k| view.level_diverge[k] == Some(armed));
                if !cascade_ok {
                    return false; // 逐级嵌套不贯通 ⇒ 非区间套精确定位点，不翻
                }
            } else {
                return false; // 无任何级别背驰段（不应到此，top_diverge 已 Some）
            }
        }
        // ── 整仓翻转（cover/close + 反向 enter），每窗口一次（consumed 守卫）──
        let cur = self.highest_active().map(|cc| self.instances[cc].direction);
        if cur == Some(armed) {
            self.nest_consumed = true; // 已在目标方向 ⇒ 本窗口视为已消费
            return false;
        }
        // 记录被平仓位逐笔（539 验收）。
        if let Some(cc) = self.highest_active() {
            let inst = self.instances[cc];
            if inst.units > EPS && inst.basis.is_finite() {
                let is_short = inst.direction == Polarity::Short;
                let pnl = match inst.direction {
                    Polarity::Long => inst.units * (c - inst.basis),
                    Polarity::Short => inst.units * (inst.basis - c),
                };
                self.nest_trades.push((is_short, inst.node.start_bar, inst.basis, bar, c, pnl));
            }
        }
        self.guards.set_trigger(OpTrigger::Bsp); // a0 定位 type1 = 合法 BSP 触发源
        self.clear_all(c);
        let node = view.nodes.get(loc).copied().flatten().unwrap_or_else(|| {
            TrendNode::new(bar, bar, c, c, match armed {
                Polarity::Long => Direction::Up,
                Polarity::Short => Direction::Down,
            })
        });
        self.enter(loc, armed, node, c);
        self.n_nest_flips += 1;
        self.nest_consumed = true; // 翻转后骑走势到窗口结束（top 反转）/反向窗口再翻。
        true
    }

    /// **consume平空步（任务22 双向多重赋格 consume 侧）**：核心仓持有期间，**次级别（k<核心级别）反核心向
    /// 背驰段定位** ⇒ 平核心仓配额（1/3，缩短整仓长持死扣，减强牛穿仓）。
    ///
    /// 缠论依据：T 算子 construct（建仓）的对偶 = consume（平仓）。读法乙原始（无 consume）= 整仓骑到 top
    /// 反转才平（1-2 年死扣）。命题2 平空欠触发根 = 走势完成触发稀疏（short-cover-diag）⇒ 换**次级别背驰段**
    /// 触发（频繁）解。底背驰段（次级别底）⇒ 平空（核心 Short 减仓）；顶背驰段（次级别顶）⇒ 平多（核心 Long 减仓）。
    /// **平空不开反向腿**（区分 sink：sink 父减+子开短差；consume 仅平核心向中性，缩短暴露）。
    fn nest_consume_step(&mut self, view: &LevelView, _bar: i64, c: f64) {
        let cc = match self.highest_active() {
            Some(k) => k,
            None => return, // 无核心仓 ⇒ 无可平
        };
        let cdir = self.instances[cc].direction;
        // 次级别（严格低于核心级别）反核心向背驰段：核心 Long → 找顶背驰段(Short极性)平多；核心 Short → 找底背驰段(Long极性)平空。
        let want_op = flip_pol(cdir); // 反核心向操作极性
        let sub_diverge = (0..cc).any(|k| view.level_diverge[k] == Some(want_op));
        if !sub_diverge {
            return; // 无次级别反核心向背驰段 ⇒ 不平
        }
        // a0 区间套定位：次级别反核心向 type1（核心 Short 找 type1_buy 平空 / 核心 Long 找 type1_sell 平多）。
        let want_buy = want_op == Polarity::Long;
        let located = (0..cc).any(|k| if want_buy { view.t1buy[k] } else { view.t1sell[k] });
        if !located {
            return; // 无次级别定位点 ⇒ 等下一定位点
        }
        // 平核心仓配额 1/3（机动仓基准 = anchor 不参与；nest 下 anchor=0 ⇒ 全仓）。
        let u = self.instances[cc].units;
        let mob_base = (u - self.instances[cc].anchor).max(0.0);
        let m = quota(mob_base);
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        self.guards.set_trigger(OpTrigger::Bsp); // 次级别背驰段定位 = 合法 BSP 触发源
        let tw_pre = self.total_wealth(c);
        let mut free = self.free;
        let realized = rec_reduce(&mut self.instances[cc], m, &mut free, c);
        self.free = free;
        self.account_reduce(cdir, realized, c);
        self.n_nest_consumes += 1;
        self.nest_consume_pnl += realized;
        self.prove_tw_neutral(tw_pre, c);
    }

    // ──────────────── 单 bar 步进（= flat step）────────────────

    /// 单 bar 操作步进（= flat step）：强平 → emergence_upgrade → route_bsp top-down。
    pub fn on_bar(&mut self, view: &LevelView, bar: i64, c: f64) {
        self.cur_bar = bar;
        self.last_close = c;
        self.guards.set_trigger(OpTrigger::None); // 本 bar 起始无触发源（强平不经原子函数）
        let tw_pre = self.total_wealth(c);

        // ── 读法B/读法乙递归分叉（任务18 编排者修正）：ON ⟹ 每级别独立腿骑走势消费 d_top（删 sink/recover/
        //   ascend/clear_all/三阶段——每级别独立骑乘取代跨级短差）。OFF ⟹ instances 路径逐字不动（bit-exact）。
        if self.enable_reading_b {
            // 强平边界（账户级 NAV≤0 ⇒ 连锁全平腿）。
            if self.nav(c) <= 0.0 {
                for k in 0..MAX_LEVEL {
                    if self.legs[k].active {
                        let entry_bar = self.legs[k].riding_node.start_bar;
                        let entry_basis = self.legs[k].basis;
                        let is_short = self.legs[k].direction == Polarity::Short;
                        self.guards.set_trigger(OpTrigger::Eod);
                        self.close_leg(k, c);
                        self.n_liquidations += 1;
                        self.liq_log.push((k, entry_bar, entry_basis, bar, c, is_short));
                    }
                }
            }
            self.consume_legs(view, c);
            self.prove_tw_neutral(tw_pre, c);
            return;
        }

        // ── A. 边界算子：保证金强平，按三阶段切换（= flat）──
        match self.stage {
            RecStage::EarningShares => {
                // 全仓：账户级，in-system NAV≤0 ⇒ 连锁全平。这是**账户级抵消**（核心多头利润 −
                // 短差空头亏 = net），降成本立于不败的实现——逐仓 layer 独立会破坏抵消致短差翻倍亏 6×退化。
                if self.nav(c) <= 0.0 {
                    let nav_now = self.nav(c);
                    self.liq_snapshot.push((2, self.core_cost_basis, self.withdrawn, self.notional_in, nav_now));
                    let mut free = self.free;
                    for k in 0..MAX_LEVEL {
                        let kdir = self.instances[k].direction;
                        let u = self.instances[k].units;
                        if u > 1e-12 {
                            let entry_bar = self.instances[k].node.start_bar;
                            let entry_basis = self.instances[k].basis;
                            let is_short = kdir == Polarity::Short;
                            let realized = rec_reduce(&mut self.instances[k], u, &mut free, c);
                            self.free = free;
                            self.n_liquidations += 1;
                            self.account_reduce(kdir, realized, c);
                            if is_short {
                                self.guards.on_short_pnl(k, realized); // prove_per_level_pnl（强平空头腿）
                            }
                            free = self.free;
                            self.liq_log.push((k, entry_bar, entry_basis, bar, c, is_short));
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
                            let sid = match self.stage {
                                RecStage::CostReduction => 0,
                                RecStage::CapitalRecovered => 1,
                                RecStage::EarningShares => 2,
                            };
                            let nav_now = self.nav(c);
                            self.liq_snapshot.push((sid, self.core_cost_basis, self.withdrawn, self.notional_in, nav_now));
                            let entry_bar = l.node.start_bar;
                            let entry_basis = l.basis;
                            let is_short = l.direction == Polarity::Short;
                            let mut free = self.free;
                            let realized = rec_reduce(&mut self.instances[k], l.units, &mut free, c);
                            self.free = free;
                            self.n_liquidations += 1;
                            self.account_reduce(l.direction, realized, c);
                            if is_short {
                                self.guards.on_short_pnl(k, realized); // prove_per_level_pnl（强平空头腿）
                            }
                            self.liq_log.push((k, entry_bar, entry_basis, bar, c, is_short));
                        }
                    }
                }
            }
        }

        // ── A^NEST. 命题4 读法乙（旁路 route_bsp/cc 锚，大级别背驰段闸门 a0 区间套定位翻转）──
        //   enable_nest 时**取代** A''/A'/B（OFF 路径）：单一全仓位，在大级别背驰段武装下、由 a0 区间套
        //   定位到的 type1 点整仓翻转（买点 cover+做多 / 卖点 close+做空）。强平（块 A）仍保留（诚实会计）。
        if self.enable_nest {
            self.nest_step(view, bar, c);
            self.prove_tw_neutral(tw_pre, c);
            return;
        }

        // ── A''. 核心走势完成 → 主动清仓（campaign 边界重定义，546号死锁解锁；与 flat 对称）──
        //   缠论依据（fengkong/chanlun-trading-system 退出条件 = 买入程序判断条件被否定 / 走势终完美）：
        //   核心仓骑的走势在**该 level 顶/底背驰（type1）**完成 ⇒ 买入逻辑被否定 ⇒ 清仓到现金。
        //   enter 重建的**第三条路**，独立于 flip：clear_all → reset_campaign → highest_active None
        //   ⇒ 死锁（核心 units 几何衰减永不归零 → highest_active 恒 Some → enter 永不触发，546号）解除，
        //   **下一个买点**经核心级 enter 重建（不在本 bar 反向 enter ⇒ 避免 545 做空陷阱）。
        //   触发源 = type1 BSP（走势完成的可观测形式）⇒ guards 归因 Bsp。**不动 EPS、不动几何衰减**。
        if let (true, Some(cc)) = (self.enable_trend_done_clear, self.highest_active()) {
            let core_trend_done = match self.instances[cc].direction {
                Polarity::Long => cc < MAX_LEVEL && view.t1sell[cc], // 顶背驰：上涨核心走势终完美
                Polarity::Short => cc < MAX_LEVEL && view.t1buy[cc], // 底背驰：下跌核心走势终完美
            };
            if core_trend_done {
                self.guards.set_trigger(OpTrigger::Bsp); // 走势完成 = type1 BSP 驱动（合法触发源）
                self.clear_all(c);
                self.n_trend_done_clears += 1;
                // 全平到现金 ⇒ TW 中性（同价 c）。本 bar 不再 route_bsp（等下一买点 enter 重建）。
                self.prove_tw_neutral(tw_pre, c);
                return;
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

        // ── prove_direction_matches_trend（观测）：核心方向应 = 最高走势类型方向 ──
        // emergent_top 是本 bar 最高 completed 走势方向；highest_active 是核心仓方向。emergence_upgrade
        // 的同向 gate 应使二者匹配——失配 = 核心被低级别 BSP flip / 逆涌现未升级（已知强牛违反）。
        if let Some((_, edir)) = view.emergent_top {
            let core_dir = self.highest_active().map(|cc| self.instances[cc].direction);
            self.guards.check_direction(core_dir, edir);
        }
    }

    /// 收尾（全平到现金，归还 withdrawn）。返回 final_nav (= free)。
    pub fn finish(&mut self, c: f64) -> f64 {
        if self.enable_reading_b {
            self.finish_legs(c);
            return self.free;
        }
        if c.is_finite() && c > 0.0 {
            self.guards.set_trigger(OpTrigger::Eod); // eod 是 clear_all 的合法非 BSP 触发源
            self.clear_all(c);
        }
        self.free
    }

    /// 读法B 每级别独立腿只读访问（诊断/L3）。
    pub fn leg(&self, k: usize) -> &Leg {
        &self.legs[k]
    }
}
