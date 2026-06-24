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
///
/// **做空腿开仓触发器（做空腿对称化，#108 L2 根因修复，编排者推动 FINAL GOAL）**：
/// #108 坐实做空腿亏 = 多空开仓触发**不对称**——多头开仓 = `view.buy[k]`（任意买点 type1/2/3，含
/// 及时的 type1 底背驰 ⇒ 骑涨，每级别 long_pnl 全正）；做空开仓 = **仅 `view.t3sell[k]`**（type3
/// 最滞后破位）⇒ 系统性晚建，错过 type1 顶背驰 → 整段 |跌幅| → type3 破位（强牛中 = 回调底 → 涨回
/// → zg 止损）。逐笔实证：做空 L0 捕获率 34-43% < 50%（BTC 36%），多头每级别全正。
///
/// 对称化（缠论「开@顶 平@底」对称性）= 把开空触发提到与多头同等及时度：
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortEntry {
    /// 默认基线（#117 已 L3）：`view.t3sell[k]`（type3 破位，最滞后）。保 #117 逐位复现。
    T3,
    /// 候选A：`view.t1sell[k]`（type1 顶背驰，与多头 type1 底背驰对称——开空于走势终完美）。
    T1,
    /// 候选B：`view.sell[k]`（任意卖点 type1/2/3，与多头 `view.buy` 完全对称）。
    Any,
}

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
    /// **读法B 一对多空腿（任务57=53.1 编排者重写）**：每级别一对 LegPair（多腿+空腿同时在场）。
    /// 开=该级别买卖点（第17课）/ 平=反向买卖点 / 止损=否定线（进场中枢 ZG/ZD 破坏）/ 链破坏 churn 门控
    /// （第27课区间套：链完整=回调不动核心多腿，链破坏=转折动核心）。删 d_top 几何方向单腿模型。
    /// OFF=false ⇒ on_bar/单腿 reading_b/instances 路径逐字不动（bit-exact）。env `T_READING_B_PAIR`。
    pub enable_reading_b_pair: bool,
    /// **牛熊对称核心翻空（任务 bear-validate）**：放开 #69 的 `k<核心` 禁令——最高活跃级别走势完成
    /// （d_top）时核心多腿不止平到现金，而是**翻空镜像**（大额吃熊）。有效域 ⊂ 真 bear regime
    /// （231号 formalization-validity-domain：net-up 8标的均亏=灾难，须 bear 数据 L3 验证）。
    /// false ⇒ g_pair 行为与 committed #69 逐字一致（bit-exact）。env `T_PAIR_CORE_SHORT`。
    pub enable_pair_core_short: bool,
    /// **放开 k<核心 t3sell 开空（任务 bear-validate 变体2）**：移除 #69 开空门控的 `below_core_long`
    /// 限制——t3sell 亦可在核心及其上开空（=变体1 机制，max_gross>1× 大额做空）。直接测「放开 k<核心
    /// ⇒ 大额做空是否吃熊」。net-up 灾难（CL L4 单笔 −40928，539）；有效域 ⊂ 真 bear。env `T_PAIR_CORE_SHORT_T3`。
    pub enable_pair_core_short_open: bool,
    /// **Face B 均匀基准单元定仓（做空腿赚 #110，shortleg-profit-spec §5.3/§6.3）**：true ⇒ LegPair
    /// open_long_leg/open_short_leg 用 `uniform_base_units`（= INITIAL_CAPITAL×MOBILE_FRAC，级别无关，
    /// 无 depth 衰减）取代 `geom_tower_quota`（恒仓归一化 Σ=free≤1× = 压制副作用，msb §14.1）。多级别独立
    /// 叠加 → 杠杆来源A 涌现（579），否定线 [ZD,ZG] 封顶每条腿（liq=0）。**同时**令信号层（rec_stream）
    /// 填充 `view.zd/zg`（进场中枢边界）⇒ pair_stop_loss_step 真生效（RB_PAIR 下 zd/zg 恒 None=死代码，
    /// liq=0 仅靠 geom_tower 恒仓；删 geom_tower 必须同步接真否定线否则穿仓）。false ⇒ RB_PAIR/OFF 逐字
    /// 不变（bit-exact，geom_tower + zd/zg 恒 None）。env `T_FACEB`。
    pub enable_uniform_sizing: bool,
    /// **做空腿开仓触发器（做空腿对称化，#108 根因）**：默认 `T3`（= #117 基线，逐位复现）；
    /// `T1`/`Any` 把开空对称到多头 `view.buy`（候选A/B）。`below_core_long` 门 + zg 否定线封顶不变
    /// （防核心假顶翻空灾难，leverage-accept −106256）。env：`T_PAIR_SHORT_T1` / `T_PAIR_SHORT_ANY`。
    /// 只在 LegPair 路径（`enable_reading_b_pair`）经 g_pair 生效；OFF/instances 路径不跑 g_pair ⇒ bit-exact。
    pub pair_short_entry: ShortEntry,
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
            enable_reading_b_pair: std::env::var("T_READING_B_PAIR").is_ok(),
            enable_pair_core_short: std::env::var("T_PAIR_CORE_SHORT").is_ok(),
            enable_pair_core_short_open: std::env::var("T_PAIR_CORE_SHORT_T3").is_ok(),
            enable_uniform_sizing: std::env::var("T_FACEB").is_ok(),
            // 做空腿对称化触发器（#108 根因）：Any 优先于 T1，二者皆无 ⇒ T3（#117 基线，逐位复现）。
            // 注：不计入 any_variant_enabled()——pair_short_entry 是 face_a 默认引擎的**修饰**（开空触发口径），
            // 非独立实验变体；单独 set T_PAIR_SHORT_T1 ⇒ production() 仍走 face_a() 分支（off() 不 reset 此字段，
            // 经 from_env 基底贯穿到 face_a），= face_a + 对称开空。
            pair_short_entry: if std::env::var("T_PAIR_SHORT_ANY").is_ok() {
                ShortEntry::Any
            } else if std::env::var("T_PAIR_SHORT_T1").is_ok() {
                ShortEntry::T1
            } else {
                ShortEntry::T3
            },
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
        c.enable_reading_b_pair = false;
        c.enable_pair_core_short = false;
        c.enable_pair_core_short_open = false;
        c.enable_uniform_sizing = false;
        c
    }
    /// **Face B：核心不僵死（做空腿赚 #110，shortleg-profit-spec §5.3）**：RB_PAIR + 均匀基准单元定仓
    /// （删 geom_tower 恒仓归一化压制）+ 真否定线 [ZD,ZG] 封顶（rec_stream 填充 view.zd/zg）。
    /// 三机制（纯级别×买卖点，删 sink/recover/anchor 配额异物）：① 删 sink ⟹ 核心不被衰减（LegPair
    /// 路径本无 sink，instances sink 仅作 OFF 回归守卫保留）；② 核心否定线 = cc 级别中枢 ZD（每腿进场
    /// 中枢边界，次级别回调不触发核心否定线）；③ 核心多腿 churn 门控 d_top（次级别卖点不平核心，已在
    /// g_pair）。唯一变量 = `enable_uniform_sizing`（其余与 reading_b_pair 逐字一致）⇒ 收益差全归因
    /// 删 geom_tower + 接真否定线。L3 有效域：做空腿赚（pair_short_pnl 符号）+ liq=0 + 中间级别解压制。
    pub fn face_b() -> Self {
        let mut c = Self::reading_b_pair();
        c.enable_uniform_sizing = true;
        c
    }
    /// **Face A：核心能动 + 接受杠杆涌现（做空腿赚 #113，shortleg-profit-spec §5.2/§六/§七）**：
    /// Face B 基座（删 geom_tower 均匀定仓 + 真否定线 [ZD,ZG]）**叠加核心翻转吃熊**——核心多腿
    /// （cc=highest_active_long）在**自己级别走势完成**（`d_top[cc]`=全深度区间套链贯通真顶=第一类卖点，
    /// 区间套级联减滞后，第27课）翻空镜像（`enable_pair_core_short`，g_pair 核心 churn 段），次级别卖点
    /// 绝不翻核心（547：sub 卖点走 below_core_long 独立空腿）。两 regime 由「哪级别走势完成」自动整合
    /// （零 if regime）：net-up cc 走势未完成 ⇒ `d_top[cc]=false` ⇒ 闸门不开 ⇒ 核心不翻空（无假顶翻空
    /// 灾难，§5.2.4）；bear cc 走势向下完成 ⇒ 闸门开 ⇒ 核心翻空吃熊。接受杠杆=多级别独立腿叠加
    /// （来源A，579），每腿否定线封顶（liq=0）。**不开 `enable_pair_core_short_open`**（=移除
    /// below_core_long 门 = net-up 假顶翻空打主升浪灾难 short_pnl +9623→−106256，leverage-accept L3
    /// 坐实；§5.2.3：type2/3 仅作走势完成级联加速器，不作核心翻转独立触发）。**生产/默认引擎**
    /// （§8.1，见 `production()`）。= reading_b_pair + uniform_sizing + pair_core_short。
    pub fn face_a() -> Self {
        let mut c = Self::face_b();
        c.enable_pair_core_short = true;
        c
    }
    /// **生产默认引擎配置（做空腿赚 #113，shortleg-profit-spec §8.1「默认开启」）**：纯级别×买卖点
    /// 统一引擎（Face A）为**生产/默认回测配置**（LegPair 路径无 sink/geom_tower 异物 ⇒ 生产路径不残留
    /// 无保护 sink，mid-scale #106 硬约束）。三档（互斥，env 显式优先）：
    /// - `T_OFF_BASELINE` 置位 ⇒ `off()`（instances OFF 基线 = bit-exact 回归守卫，R4）。
    /// - 任一显式实验变体 env 已选（`from_env` 读到 reading_b/nest/anchor/legpair/coreshort/faceb 任一）⇒
    ///   尊重该显式变体（受控实验覆盖默认，保 env 实验工具不失效）。
    /// - 无任何变体 env ⇒ `face_a()`（**默认开启**）。
    ///
    /// **why 不改 from_env 默认**：`from_env()`/`TRoot::new()` 默认 OFF 基线是 instances-path 单测
    /// （`核心级买点_enter_long` 等）的契约——改 from_env 默认会破这些单测 + OFF 回归守卫 base。故「默认
    /// 开启」落在**生产入口**（`RecStream::new_with_a0`，FFI/python 回测路径），不动 from_env。
    pub fn production() -> Self {
        if std::env::var("T_OFF_BASELINE").is_ok() {
            return Self::off(); // 显式回归守卫：instances bit-exact 基线
        }
        let env_cfg = Self::from_env();
        if env_cfg.any_variant_enabled() {
            env_cfg // 显式实验变体 env 已选 ⇒ 尊重（受控实验）
        } else {
            Self::face_a() // 无变体 env ⇒ 默认开启 Face A（§8.1）
        }
    }
    /// 是否已显式启用任一**实验变体**（instances 基线默认 earning/three_stage/trend_done_clear 不计）。
    /// `production()` 用：有显式变体 ⇒ 尊重；无 ⇒ 默认 Face A。
    fn any_variant_enabled(&self) -> bool {
        self.enable_hold_anchor
            || self.enable_nest
            || self.enable_nest_consume
            || self.enable_nest_strict
            || self.enable_reading_b
            || self.reading_b_diverge
            || self.enable_reading_b_pair
            || self.enable_pair_core_short
            || self.enable_pair_core_short_open
            || self.enable_uniform_sizing
    }
    /// **读法B 一对多空腿（任务57=53.1）**：每级别 LegPair（买卖点开平 + 否定线止损 + 链破坏 churn）。
    /// OFF + 仅 `enable_reading_b_pair`（旁路单腿 reading_b / instances 路径）。
    pub fn reading_b_pair() -> Self {
        let mut c = Self::off();
        c.enable_reading_b_pair = true;
        c
    }
    /// **牛熊对称核心翻空（任务 bear-validate）**：RB_PAIR + 放开 `k<核心`（核心走势完成→翻空镜像）。
    /// 唯一变量 = `enable_pair_core_short`（其余与 reading_b_pair 逐字一致 ⇒ 收益差全归因核心翻空）。
    /// L3 有效域：须真 bear regime 数据验证（net-up 8标的灾难，239号有效域受限）。
    pub fn reading_b_pair_coreshort() -> Self {
        let mut c = Self::reading_b_pair();
        c.enable_pair_core_short = true;
        c
    }
    /// **放开 k<核心 t3sell 大额做空（任务 bear-validate 变体2）**：RB_PAIR + 移除 below_core_long 门控
    /// （t3sell 在核心及其上开空 = 变体1 机制，max_gross>1×）。直接测「大额做空吃熊」。
    pub fn reading_b_pair_coreshort_t3() -> Self {
        let mut c = Self::reading_b_pair();
        c.enable_pair_core_short_open = true;
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

// ════════════════════════════ LegPair：每级别一对多空腿（任务57=53.1 编排者重写）════════════════════════════

/// **每级别一对多空腿（LegPair，自相似）**：级别 k 同时持有一条多腿（吃涨）+ 一条空腿（吃跌）。
/// 牛市 = 大级别多腿（核心）+ 次级别空腿（吃回调）同时在场 = 多空双开 = 吃所有级别涨跌幅。
///
/// 三个缠论结构点（每腿）：
/// 1. **开** = 该级别买卖点（第17课买卖点定律一 + 第27课区间套定位）：买点 fire → 多腿开 / 卖点 fire → 空腿开。
///    开仓方向由缠论买卖点涌现，**不由 node.direction 几何方向**（删单腿模型的 dir_to_polarity(node.direction)）。
/// 2. **平** = 该级别反向买卖点（与开对称）：多腿持仓中卖点 fire → 多腿平 / 空腿持仓中买点 fire → 空腿平。
/// 3. **止损 = 否定线**（进场中枢边界 ZG/ZD）：多腿 stop = 进场 ZD（跌破=结构破坏）/ 空腿 stop = 进场 ZG（涨破=结构破坏）。
///    「回试回中枢=假突破」⟹ 止损。stop_line 在开仓时锁定（进场中枢边界，骑节点不变）。
///
/// **零 `if level==top` 硬编码**：方向/开/平/止损全由缠论结构（买卖点/中枢/否定线）涌现，自相似跨级别同构。
#[derive(Debug, Clone, Copy)]
pub struct LegPair {
    pub level: usize,
    /// 多腿股数 ≥0（idle=0）。
    pub long_units: f64,
    /// 多腿 basis（开仓价）。
    pub long_basis: f64,
    /// 多腿否定线 = 进场中枢 ZD（跌破止损）。NaN=未锁定/无中枢（无 ZD 否定线 ⇒ 仅靠反向买卖点平）。
    pub long_stop: f64,
    /// 空腿股数 ≥0（idle=0）。
    pub short_units: f64,
    /// 空腿 basis（开仓价）。
    pub short_basis: f64,
    /// 空腿否定线 = 进场中枢 ZG（涨破止损）。NaN=未锁定/无中枢。
    pub short_stop: f64,
    /// 多腿建仓 raw bar（per-element 会计 instrumentation #149，capture-ratio 验证工具）。
    /// observation-only：不参与任何仓位/方向/止损决策 ⇒ bit-exact 不变。idle=-1。
    pub long_entry_bar: i64,
    /// 空腿建仓 raw bar（同上）。idle=-1。
    pub short_entry_bar: i64,
}

impl LegPair {
    fn idle(level: usize) -> Self {
        LegPair {
            level,
            long_units: 0.0,
            long_basis: f64::NAN,
            long_stop: f64::NAN,
            short_units: 0.0,
            short_basis: f64::NAN,
            short_stop: f64::NAN,
            long_entry_bar: -1,
            short_entry_bar: -1,
        }
    }
    pub fn long_active(&self) -> bool {
        self.long_units > EPS
    }
    pub fn short_active(&self) -> bool {
        self.short_units > EPS
    }
    /// 是否最高活跃多腿级别（结构涌现「核心」= 当前最高活跃 up-trend 多腿，无 `if level==top` 硬编码）。
    fn pair_fingerprint(&self) -> (u64, u64) {
        (self.long_units.to_bits(), self.short_units.to_bits())
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
    /// 该 level 是否新增 **type3 卖点**（第三类卖点 = 向下突破中枢下沿 ZD + 回试高点不回中枢 = 真顶转折，fresh）。
    /// 真顶/假顶判别：突破中枢+回试不回（`detect_type3`：`leave.high<ZD ∧ pull.high<ZD`）=真转折开空腿；
    /// 假突破（回试回中枢 `pull.high≥ZD`）⇒ 无 type3 ⇒ 不开空（滤震荡假突破累积止损）。
    pub t3sell: [bool; MAX_LEVEL],
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
    /// **任务69**：LegPair 路径（`enable_reading_b_pair`）**核心多腿 churn 门控**消费 `d_top`（最深区间套确认=走势完成
    /// 真顶，稀疏⇒保护主力骑牛不踏空）；次级别开空腿改用 `t3sell`（单层区间套转折，响应回调）⇒ 区间套确认深度按持仓尺度分级。
    pub d_top: [bool; MAX_LEVEL],
    /// **每级别当前走势末中枢核心区间 ZG（核心上沿，否定线原料）**（任务57=53.1 LegPair 止损）。
    /// 空腿止损线：价格涨破进场中枢 ZG ⇒ 向上突破=「回试回中枢=假突破」反面=结构破坏 ⇒ 空腿止损平。None=该级无中枢。
    pub zg: [Option<f64>; MAX_LEVEL],
    /// **每级别当前走势末中枢核心区间 ZD（核心下沿，否定线原料）**（任务57=53.1 LegPair 止损）。
    /// 多腿止损线：价格跌破进场中枢 ZD ⇒ 结构破坏（多头买入逻辑被否定，第17课区间套否定线）⇒ 多腿止损平。None=该级无中枢。
    pub zd: [Option<f64>; MAX_LEVEL],
}

impl LevelView {
    pub fn empty() -> Self {
        LevelView {
            buy: [false; MAX_LEVEL],
            sell: [false; MAX_LEVEL],
            t1buy: [false; MAX_LEVEL],
            t1sell: [false; MAX_LEVEL],
            t3sell: [false; MAX_LEVEL],
            nodes: [None; MAX_LEVEL],
            emergent_top: None,
            top_diverge: None,
            top_trend_dir: None,
            level_diverge: [None; MAX_LEVEL],
            d_top: [false; MAX_LEVEL],
            zg: [None; MAX_LEVEL],
            zd: [None; MAX_LEVEL],
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
    /// **读法B 一对多空腿（任务57=53.1）**：enable_reading_b_pair=true ⇒ on_bar 走 consume_leg_pairs。
    enable_reading_b_pair: bool,
    /// **牛熊对称核心翻空（任务 bear-validate）**：true ⇒ g_pair 放开 `k<核心`，核心走势完成翻空镜像。
    enable_pair_core_short: bool,
    /// **放开 k<核心 t3sell 开空（任务 bear-validate 变体2）**：true ⇒ 移除 below_core_long 门控（t3sell 核心及其上开空）。
    enable_pair_core_short_open: bool,
    /// **Face B 均匀基准单元定仓（做空腿赚 #110）**：true ⇒ open_*_leg 用 uniform_base_units 取代
    /// geom_tower_quota（删恒仓归一化压制 → 来源A 杠杆涌现 + 真否定线封顶）。OFF=false（bit-exact）。
    enable_uniform_sizing: bool,
    /// **做空腿开仓触发器（做空腿对称化，#108 根因）**：T3=#117 基线（view.t3sell）/ T1=候选A（view.t1sell）
    /// / Any=候选B（view.sell，对称多头 view.buy）。g_pair 开空门控用，below_core_long 门不变。
    pair_short_entry: ShortEntry,
    /// 初始本金（= free 初值）——Face B 均匀基准单元定仓的级别无关基准（initial_capital×MOBILE_FRAC）。
    initial_capital: f64,
    /// 每级别独立腿（legs[k] 骑 levels[k] 走势消费 d_top[k]）。
    legs: Vec<Leg>,
    /// **每级别一对多空腿（LegPair[k]，任务57=53.1）**：买卖点开平 + 否定线止损 + 链破坏 churn 门控。
    leg_pairs: Vec<LegPair>,
    /// LegPair per-level realized pnl（验收哪级别多/空腿赚/亏）。
    pub pair_long_pnl: [f64; MAX_LEVEL],
    pub pair_short_pnl: [f64; MAX_LEVEL],
    /// LegPair 观测计数（纯诊断）。
    pub pair_long_opens: [u64; MAX_LEVEL],
    pub pair_short_opens: [u64; MAX_LEVEL],
    pub pair_long_closes: [u64; MAX_LEVEL],
    pub pair_short_closes: [u64; MAX_LEVEL],
    /// 否定线止损次数（多/空腿，纯观测——验收止损先于 NAV≤0）。
    pub pair_long_stops: u64,
    pub pair_short_stops: u64,
    /// 链破坏 churn 触发核心多腿翻转次数（纯观测——解 churn 门控是否解冻核心）。
    pub pair_core_churns: u64,
    /// 读法B 每级别 per-level realized pnl（验收哪级别腿赚/亏）。
    pub per_level_long_pnl: [f64; MAX_LEVEL],
    pub per_level_short_pnl: [f64; MAX_LEVEL],
    /// 读法B 每级别腿切换次数（d_top[k] 驱动 close+reopen，纯观测——解 556 顶层腿是否冻结）。
    pub leg_switches_by_level: [u64; MAX_LEVEL],
    pub leg_opens_by_level: [u64; MAX_LEVEL],
    /// 读法B 杠杆验收（裂隙2 异质质询）：max 毛敞口 / max 净敞口（相对 NAV，×100 整数存）。
    /// 恒仓声明 = max_gross ≤ ~100（≤1×）。>100 = 杠杆（ES+681%可能伪影）。
    pub max_gross_exp_x100: u64,
    pub max_net_exp_x100: u64,

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
    /// #149 per-element 会计 instrumentation（capture-ratio 验证工具，observation-only）：
    /// LegPair 每笔多/空腿平仓的完整逐笔记录，供 per-(级别,走势段,方向) 捕获率归因。
    /// (level, entry_bar_raw, exit_bar_raw, entry_price, exit_price, units, is_short, realized_pnl)。
    /// entry/exit_bar = raw bar（确认时点 cur_bar）；Σ(此 log 多头 pnl)==Σ pair_long_pnl，空头同（完整性自检）。
    pub leg_trades: Vec<(usize, i64, i64, f64, f64, f64, bool, f64)>,
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
            enable_reading_b_pair: cfg.enable_reading_b_pair,
            enable_pair_core_short: cfg.enable_pair_core_short,
            enable_pair_core_short_open: cfg.enable_pair_core_short_open,
            pair_short_entry: cfg.pair_short_entry,
            enable_uniform_sizing: cfg.enable_uniform_sizing,
            initial_capital,
            legs: (0..MAX_LEVEL).map(Leg::idle).collect(),
            leg_pairs: (0..MAX_LEVEL).map(LegPair::idle).collect(),
            pair_long_pnl: [0.0; MAX_LEVEL],
            pair_short_pnl: [0.0; MAX_LEVEL],
            pair_long_opens: [0; MAX_LEVEL],
            pair_short_opens: [0; MAX_LEVEL],
            pair_long_closes: [0; MAX_LEVEL],
            pair_short_closes: [0; MAX_LEVEL],
            pair_long_stops: 0,
            pair_short_stops: 0,
            pair_core_churns: 0,
            per_level_long_pnl: [0.0; MAX_LEVEL],
            per_level_short_pnl: [0.0; MAX_LEVEL],
            leg_switches_by_level: [0; MAX_LEVEL],
            leg_opens_by_level: [0; MAX_LEVEL],
            max_gross_exp_x100: 0,
            max_net_exp_x100: 0,
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
            leg_trades: Vec::new(),
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
        // 读法B 一对多空腿（LegPair[k]，任务57=53.1）：多腿 +u·c / 空腿 −u·c。OFF 时全 idle ⇒ 0（bit-exact）。
        for p in &self.leg_pairs {
            if p.long_units > 0.0 {
                v += p.long_units * c;
            }
            if p.short_units > 0.0 {
                v -= p.short_units * c;
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

    /// **均匀基准单元定仓（Face B，shortleg-profit-spec §6.3，删 geom_tower 恒仓归一化）**：
    /// 每级别基准单元 = `initial_capital × MOBILE_FRAC`（级别无关，**无 depth 衰减、无 if level、无 free
    /// 归一化**）。geom_tower 的 Σnotional=free 恒仓归一化把 max_gross 钳到 ≤1×（压制副作用，msb §14.1
    /// = 中间级别敞口塌缩不吃自身|涨跌幅|）；均匀定仓让多级别独立腿叠加 → max_gross 自然 >1×（杠杆来源A
    /// 涌现，579），由每腿否定线 [ZD,ZG] 分散封顶（liq=0）。
    /// **no-hardcode**：唯一常数 = MOBILE_FRAC（= 1/λ 中枢三段 σ-不变，非新倍数参数）；杠杆从级别叠加
    /// 涌现非倍数（编排者「删配额非加参数，杠杆涌现非倍数」）。固定绝对基准（非 free×frac）⇒ 不复现
    /// geom_tower 的 Σ≤free 压制（free×frac 几何收敛回 ≤1×）；账户增长时 gross/nav 自动去杠杆（安全）。
    fn uniform_base_units(&self, c: f64) -> f64 {
        if !(c > 0.0) {
            return 0.0;
        }
        self.initial_capital * MOBILE_FRAC / c
    }

    /// **腿定仓分派**（Face B 均匀基准单元 / 否则 geom_tower 恒仓配额）。单点切换保 bit-exact：
    /// `enable_uniform_sizing=false` ⇒ 逐字 geom_tower_quota（RB_PAIR/OFF 不变）。
    fn leg_open_units(&self, k: usize, top: usize, c: f64) -> f64 {
        if self.enable_uniform_sizing {
            self.uniform_base_units(c)
        } else {
            self.geom_tower_quota(k, top, self.free.max(0.0), c)
        }
    }

    /// **否定线消费门（Face B，bit-exact 防御）**：进场中枢边界 [ZD,ZG] 仅在 Face B
    /// （`enable_uniform_sizing`）作为否定线锁入腿；否则 None（RB_PAIR/OFF 下 long_stop/short_stop=NaN
    /// ⇒ pair_stop_loss_step 死代码不动 = 逐字不变）。在引擎层而非仅信号层把关，使 bit-exact 不依赖
    /// rec_stream 是否填充 view.zd/zg（防御 #69 否定线=死代码的隐性依赖）。
    fn faceb_stop(&self, raw: Option<f64>) -> Option<f64> {
        if self.enable_uniform_sizing {
            raw
        } else {
            None
        }
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

    // ──────────────── 读法B 一对多空腿（LegPair，任务57=53.1 编排者重写）────────────────

    /// LegPair 指纹快照（547 隔离守卫：g_pair(k) 只许写 leg_pairs[k]）。
    fn snapshot_pair_fingerprints(&self) -> [(u64, u64); MAX_LEVEL] {
        let mut fp = [(0u64, 0u64); MAX_LEVEL];
        for k in 0..MAX_LEVEL {
            fp[k] = self.leg_pairs[k].pair_fingerprint();
        }
        fp
    }

    /// **prove_pair_isolation（547 隔离，L0 结构 panic 守卫）**：g_pair(k) 只写 leg_pairs[k]，断言
    /// leg_pairs[j≠k] 在前后逐位不变（547 病灶「低级别信号越级翻动高级别主力」的结构否定，对偶单腿守卫）。
    fn prove_pair_isolation(
        fp_pre: &[(u64, u64); MAX_LEVEL],
        fp_post: &[(u64, u64); MAX_LEVEL],
        k: usize,
    ) {
        for j in 0..MAX_LEVEL {
            if j == k {
                continue;
            }
            assert_eq!(
                fp_pre[j], fp_post[j],
                "547 隔离违反：g_pair({k}) 改动了别级 leg_pairs[{j}]（每级别独立对腿只许 g_pair(k) 写 leg_pairs[k]）"
            );
        }
    }

    /// **open_long_leg**（开多腿）：级别 k 买点 fire ⇒ 从单一 free 池领几何塔配额建多仓，否定线 = 进场 ZD。
    /// 只写 leg_pairs[k]（547 隔离）。`stop_zd`=进场中枢核心下沿（跌破止损）；None ⇒ NaN（无否定线，仅靠反向卖点平）。
    fn open_long_leg(&mut self, k: usize, top: usize, stop_zd: Option<f64>, c: f64) {
        if self.leg_pairs[k].long_active() {
            return; // 已持多腿 ⇒ 不重复开（hold，单一方向单腿）
        }
        let fp_pre = self.snapshot_pair_fingerprints();
        let m = self.leg_open_units(k, top, c);
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        let tw_pre = self.total_wealth(c);
        self.free -= m * c; // 多腿建仓 = 现金换多头（NAV 中性）
        self.leg_pairs[k].long_units = m;
        self.leg_pairs[k].long_basis = c;
        self.leg_pairs[k].long_stop = stop_zd.unwrap_or(f64::NAN);
        self.leg_pairs[k].long_entry_bar = self.cur_bar; // #149 capture-ratio 逐笔账本（observation-only）
        self.pair_long_opens[k] += 1;
        self.guards.note_op("open_long_leg");
        Self::prove_pair_isolation(&fp_pre, &self.snapshot_pair_fingerprints(), k);
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **open_short_leg**（开空腿）：级别 k 卖点 fire ⇒ 建空仓，否定线 = 进场 ZG（涨破止损）。只写 leg_pairs[k]。
    fn open_short_leg(&mut self, k: usize, top: usize, stop_zg: Option<f64>, c: f64) {
        if self.leg_pairs[k].short_active() {
            return; // 已持空腿 ⇒ 不重复开
        }
        let fp_pre = self.snapshot_pair_fingerprints();
        let m = self.leg_open_units(k, top, c);
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        let tw_pre = self.total_wealth(c);
        self.free += m * c; // 空腿建仓 = 收空头保证金现金（NAV 中性，−u·c 抵 +m·c）
        self.leg_pairs[k].short_units = m;
        self.leg_pairs[k].short_basis = c;
        self.leg_pairs[k].short_stop = stop_zg.unwrap_or(f64::NAN);
        self.leg_pairs[k].short_entry_bar = self.cur_bar; // #149 capture-ratio 逐笔账本（observation-only）
        self.pair_short_opens[k] += 1;
        self.guards.note_op("open_short_leg");
        Self::prove_pair_isolation(&fp_pre, &self.snapshot_pair_fingerprints(), k);
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **close_long_leg**（平多腿）：反向卖点 fire 或否定线止损 ⇒ 全量平多到现金，realized 记 pair_long_pnl。
    fn close_long_leg(&mut self, k: usize, c: f64) -> f64 {
        if !self.leg_pairs[k].long_active() {
            return 0.0;
        }
        let fp_pre = self.snapshot_pair_fingerprints();
        let u = self.leg_pairs[k].long_units;
        let basis = self.leg_pairs[k].long_basis;
        let pnl = u * (c - basis);
        let tw_pre = self.total_wealth(c);
        self.free += u * c;
        self.pair_long_pnl[k] += pnl;
        // #149 capture-ratio 逐笔账本（observation-only，不改决策）：(level,entry_bar,exit_bar,entry_px,exit_px,units,is_short,pnl)
        self.leg_trades.push((k, self.leg_pairs[k].long_entry_bar, self.cur_bar, basis, c, u, false, pnl));
        self.leg_pairs[k].long_units = 0.0;
        self.leg_pairs[k].long_basis = f64::NAN;
        self.leg_pairs[k].long_stop = f64::NAN;
        self.leg_pairs[k].long_entry_bar = -1;
        self.pair_long_closes[k] += 1;
        self.guards.note_op("close_long_leg");
        Self::prove_pair_isolation(&fp_pre, &self.snapshot_pair_fingerprints(), k);
        self.prove_tw_neutral(tw_pre, c);
        pnl
    }

    /// **close_short_leg**（平空腿）：反向买点 fire 或否定线止损 ⇒ 全量平空到现金，realized 记 pair_short_pnl。
    fn close_short_leg(&mut self, k: usize, c: f64) -> f64 {
        if !self.leg_pairs[k].short_active() {
            return 0.0;
        }
        let fp_pre = self.snapshot_pair_fingerprints();
        let u = self.leg_pairs[k].short_units;
        let basis = self.leg_pairs[k].short_basis;
        let pnl = u * (basis - c);
        let tw_pre = self.total_wealth(c);
        self.free -= u * c;
        self.pair_short_pnl[k] += pnl;
        // #149 capture-ratio 逐笔账本（observation-only，不改决策）：(level,entry_bar,exit_bar,entry_px,exit_px,units,is_short,pnl)
        self.leg_trades.push((k, self.leg_pairs[k].short_entry_bar, self.cur_bar, basis, c, u, true, pnl));
        self.leg_pairs[k].short_units = 0.0;
        self.leg_pairs[k].short_basis = f64::NAN;
        self.leg_pairs[k].short_stop = f64::NAN;
        self.leg_pairs[k].short_entry_bar = -1;
        self.pair_short_closes[k] += 1;
        self.guards.note_op("close_short_leg");
        Self::prove_pair_isolation(&fp_pre, &self.snapshot_pair_fingerprints(), k);
        self.prove_tw_neutral(tw_pre, c);
        pnl
    }

    /// **当前最高活跃多腿级别**（结构涌现「核心」= 当前最高活跃 up-trend 多腿，无 `if level==top` 硬编码）。
    /// LegPair 路径核心 = 最高有多腿在场的级别（自相似结构涌现，对照 instances 路径 highest_active）。
    fn highest_active_long(&self) -> Option<usize> {
        (0..MAX_LEVEL).rev().find(|&k| self.leg_pairs[k].long_active())
    }

    /// **否定线止损（任务57=53.1 第三结构点）**：每级别多/空腿价格否定结构破坏 ⇒ 止损平。
    /// 多腿：c < long_stop（进场 ZD，跌破=结构破坏）⇒ 平。空腿：c > short_stop（进场 ZG，涨破=结构破坏）⇒ 平。
    /// 「回试回中枢=假突破」⟹ 止损。止损先于 NAV≤0（账户级强平）生效 ⇒ 零强平判据。
    fn pair_stop_loss_step(&mut self, c: f64) {
        for k in 0..MAX_LEVEL {
            // 多腿否定线：跌破进场 ZD ⇒ 结构破坏止损。
            if self.leg_pairs[k].long_active() {
                let stop = self.leg_pairs[k].long_stop;
                if stop.is_finite() && c < stop {
                    self.guards.set_trigger(OpTrigger::Bsp); // 否定线 = 结构破坏（type3 形态对偶）= 合法触发源
                    self.close_long_leg(k, c);
                    self.pair_long_stops += 1;
                }
            }
            // 空腿否定线：涨破进场 ZG ⇒ 结构破坏止损。
            if self.leg_pairs[k].short_active() {
                let stop = self.leg_pairs[k].short_stop;
                if stop.is_finite() && c > stop {
                    self.guards.set_trigger(OpTrigger::Bsp);
                    self.close_short_leg(k, c);
                    self.pair_short_stops += 1;
                }
            }
        }
    }

    /// **g_pair(k)**（每级别一对腿单算子）：级别 k 消费 `view.buy[k]`/`view.sell[k]`（买卖点）开平 + 链破坏 churn 门控。
    ///
    /// 开平规则（对称）：
    /// - **买点 fire**：① 空腿持仓 ⇒ 平空（反向买卖点平，与开对称）；② 多腿空 ⇒ 开多（买点开多腿）。
    /// - **卖点 fire**：① 多腿持仓 ⇒ 平多（反向卖点平）；② 空腿空 ⇒ 开空（卖点开空腿）。
    ///
    /// **区间套-confirmed 买卖点门控（任务69）——确认深度按持仓尺度自相似分级（567洞察①「腿开平绑买卖点+区间套」）**：
    /// - **核心多腿 churn**（= highest_active_long）：只在 `d_top`（走势完成真顶 = type1买卖点 + 区间套 nesting 全深度=
    ///   最深确认，稀疏）才平/翻转；走势未完成（回调/假突破）⇒ 不平核心多腿（保护主力骑牛不踏空）。次级别 long 无门控。
    /// - **开空腿**：只在 `t3sell`（第三类卖点=突破中枢下沿+回试不回=单层区间套转折，第27课）∧ **严格次级别（k<核心）** 才开；
    ///   假突破（回试回中枢）不开（滤震荡累积止损）；核心及其上绝不翻空（net-up 假顶翻空打主浪=灾难，L3 坐实）。
    /// 自相似同构：皆该级别区间套-confirmed 买卖点驱动、确认深度∝持仓尺度（主力 d_top 全深度/短差 t3sell 单层）、
    /// 角色（core vs sub）由 `highest_active_long` 结构涌现（零 if level==N/regime）。
    ///
    /// **零方向几何**：开仓方向由买卖点（buy/sell）涌现，不由 node.direction。**只写 leg_pairs[k]**（547 隔离）。
    fn g_pair(&mut self, k: usize, view: &LevelView, top: usize, c: f64) {
        let b = view.buy[k];
        let s = view.sell[k];
        if b && s {
            return; // 同 bar 同级别买卖冲突 ⇒ 跳过（对照 on_bar route_bsp 同 level 买卖冲突）
        }
        // 「核心」= 当前最高活跃多腿级别（结构涌现，无 if level==top 硬编码）。
        let is_core_long_level = self.highest_active_long() == Some(k);
        // **区间套-confirmed 买卖点驱动门控（任务69）——区间套确认深度按持仓尺度自相似分级（编排者「快且准」+ 567洞察①）**：
        //   - **核心多腿 churn**（平主力 long）← `d_top`（区间套链贯通到 a0 = 走势完成真顶 = type1买卖点 + 区间套nesting 全深度，
        //     最深确认 ⇒ 稀疏，保护主力骑牛不踏空）。注：567「d_top 错误代理」指的是把**空腿**绑 d_top（稀疏⇒空腿冻结），
        //     核心主力 churn 恰需稀疏（骑牛），故沿用 d_top。L3 坐实：核心 churn 若改频繁信号(t1sell)⇒ 趋势踏空
        //     （GC long +63522→+16227 / QQQ +36595→+10751）。
        //   - **次级别开空腿**（吃回调短差）← `t3sell`（第三类卖点=突破中枢下沿+回试不回=单层区间套转折，第27课，响应回调）。
        // 自相似原则：区间套确认深度 ∝ 持仓尺度（主力=全深度 d_top / 短差=单层 t3sell），角色由 `highest_active_long` 结构涌现
        // 决定（零 if level==N/regime）。这正是 567洞察①「腿开平绑买卖点+区间套」——主力绑走势完成、短差绑第三类转折。
        let core_done = view.d_top[k];
        // **开空触发器（做空腿对称化，#108 L2 根因修复）**：默认 T3（=view.t3sell，#117 基线，逐位复现）。
        // 候选A（T1=view.t1sell 顶背驰）/ 候选B（Any=view.sell 任意卖点 = `s`，与多头 view.buy 完全对称）
        // 把开空提到与多头 view.buy 同等及时度——消解 #108「做空仅 t3sell 晚建」不对称。`below_core_long` 门
        // + zg 否定线封顶（下方 short_level_ok / open_short_leg）**不变** ⇒ net-up 假顶仍封死核心翻空（防灾难）。
        let sub_break = match self.pair_short_entry {
            ShortEntry::T3 => view.t3sell[k],
            ShortEntry::T1 => view.t1sell[k],
            ShortEntry::Any => s, // = view.sell[k]，已在 `if s` 块内恒真 ⇒ 任意卖点（below_core_long 内）开空
        };

        if b {
            // 买点：先平空腿（反向买卖点平），再开多腿（若多腿空）。
            if self.leg_pairs[k].short_active() {
                self.guards.set_trigger(OpTrigger::Bsp);
                self.close_short_leg(k, c);
            }
            if !self.leg_pairs[k].long_active() {
                self.guards.set_trigger(OpTrigger::Bsp);
                self.open_long_leg(k, top, self.faceb_stop(view.zd[k]), c);
            }
            return;
        }
        if s {
            // 卖点：先平多腿（反向买卖点平），但核心多腿受 churn 门控（链破坏才平=转折，链完整=回调不动核心）。
            if self.leg_pairs[k].long_active() {
                // 核心多腿只在走势完成（d_top=最深区间套真顶）才平（保护主力骑牛）；次级别 long 无门控（任意卖点平）。
                let may_close_core = !is_core_long_level || core_done;
                if may_close_core {
                    self.guards.set_trigger(OpTrigger::Bsp);
                    self.close_long_leg(k, c);
                    if is_core_long_level && core_done {
                        self.pair_core_churns += 1; // 走势完成 churn 动核心多腿（真顶转折）
                        // **牛转熊核心翻空镜像（任务 bear-validate，编排者：最高活跃级别走势完成→核心翻空）**：
                        // 放开 #69 `k<核心` 禁令——核心走势完成（d_top=最深区间套真顶=type1+全深度背驰链）不止
                        // 平多到现金，而是**翻空**（大额吃熊，max_gross>1×=核心仓尺度）。结构涌现：is_core_long_level
                        // = highest_active_long==k（零 if regime/level，编排者 no-hardcode）。zg[k]=进场中枢上沿
                        // =否定线止损（涨破⇒牛市恢复⇒止损出，27课区间套否定）。有效域 ⊂ 真 bear（231号
                        // formalization-validity-domain）：net-up 假顶翻空打主升浪=灾难（539），须 bear 数据 L3。
                        if self.enable_pair_core_short && !self.leg_pairs[k].short_active() {
                            self.guards.set_trigger(OpTrigger::Bsp);
                            self.open_short_leg(k, top, self.faceb_stop(view.zg[k]), c);
                        }
                    }
                }
                // 走势未完成 ∧ 核心多腿 ⇒ 不平核心多腿（回调），落到下方开空腿吃回调。
            }
            // **开空腿门控（任务69，编排者「次级别做空」+ per-level L3 诊断 + geom_tower_quota 结构）**：
            //   ① `sub_break`（=t3sell=突破中枢下沿+回试不回=真顶转折）才开空（假突破不开，滤假突破累积止损）；
            //   ② **仅在核心多腿级别之下开空（`k < 核心级别`）——绝不在核心或其上开空**。
            // 依据：编排者纲领「本级别卖点→平多 + 次级别做空」（核心只平多到现金，做空在严格次级别）。L3 per-level 坐实：
            //   高级别空腿 = geom_tower_quota 最大配额（depth=top−k 小⇒notional 大）× 在 net-up regime「假顶翻空打主升浪」
            //   = 灾难（变体1 CL L4 单笔−40928 / BTC L3 −20629 / GC L3 −7335，皆 1-3 笔巨亏）；严格次级别短差小配额吃回调
            //   正域（变体4 CL L0/L1/L2 +3145 / BTC L1/L2/L3 +10670 / GC/ES/OKLO 正）。`!is_core` 不够（杀手空腿开在核心
            //   之上非核心本身）⇒ 须 k<核心。
            // 牛熊切换（③）：核心走势完成（d_top）⇒ churn 平核心多腿到现金（不翻空，21:40 升跌完备性=上涨趋势无真顶卖点）；
            //   无核心多腿（熊市镜像）⇒ 无 long 框架 ⇒ 当前不开空（保守；熊市核心做空镜像吃熊=开放轴，8 标的均 net-up
            //   无法 L3 证伪 ⇒ 不擅自实装=避有效域膨胀）。
            // 自相似 no-hardcode：`highest_active_long()` 是结构涌现（无 if level==N/regime），k<core 跨级别同构。
            let below_core_long = self.highest_active_long().map_or(false, |core| k < core);
            // 变体2（放开 k<核心 t3sell 大额做空）：移除 below_core_long 限制 ⇒ t3sell 在核心及其上开空（max_gross>1×）。
            let short_level_ok = below_core_long || self.enable_pair_core_short_open;
            if !self.leg_pairs[k].short_active() && sub_break && short_level_ok {
                self.guards.set_trigger(OpTrigger::Bsp);
                self.open_short_leg(k, top, self.faceb_stop(view.zg[k]), c);
            }
            return;
        }
    }

    /// **consume_leg_pairs**（单算子递归 = consume_legs 的对偶）：账户强平 → 止损 → a0→涌现上界逐级 g_pair(k)。
    fn consume_leg_pairs(&mut self, view: &LevelView, c: f64) {
        // ⓪ 账户级 NAV≤0 强平（诚实会计安全网）：否定线止损应先于此生效 ⇒ n_liquidations==0 = 止损有效判据。
        //    保留此块是诚实会计（不声明「永不强平」而无强平路径）；零强平由否定线止损保证，非删除强平路径。
        if self.nav(c) <= 0.0 {
            for k in 0..MAX_LEVEL {
                if self.leg_pairs[k].long_active() {
                    let entry_bar = self.cur_bar;
                    let entry_basis = self.leg_pairs[k].long_basis;
                    self.guards.set_trigger(OpTrigger::Eod);
                    self.close_long_leg(k, c);
                    self.n_liquidations += 1;
                    self.liq_log.push((k, entry_bar, entry_basis, self.cur_bar, c, false));
                }
                if self.leg_pairs[k].short_active() {
                    let entry_bar = self.cur_bar;
                    let entry_basis = self.leg_pairs[k].short_basis;
                    self.guards.set_trigger(OpTrigger::Eod);
                    self.close_short_leg(k, c);
                    self.n_liquidations += 1;
                    self.liq_log.push((k, entry_bar, entry_basis, self.cur_bar, c, true));
                }
            }
        }
        // ① 否定线止损（第三结构点，先于买卖点开平 ⇒ 否定线优先平失血腿）。
        self.pair_stop_loss_step(c);
        // ② 逐级买卖点开平 + churn 门控。
        let top = match (0..MAX_LEVEL).rev().find(|&k| view.nodes[k].is_some()) {
            Some(t) => t,
            None => return,
        };
        for k in 0..=top {
            self.g_pair(k, view, top, c);
        }
    }

    /// 读法B LegPair 收尾（全平所有对腿到现金）。
    fn finish_leg_pairs(&mut self, c: f64) {
        if !(c.is_finite() && c > 0.0) {
            return;
        }
        self.guards.set_trigger(OpTrigger::Eod);
        for k in 0..MAX_LEVEL {
            if self.leg_pairs[k].long_active() {
                self.close_long_leg(k, c);
            }
            if self.leg_pairs[k].short_active() {
                self.close_short_leg(k, c);
            }
        }
    }

    /// LegPair 只读访问（诊断/L3）。
    pub fn leg_pair(&self, k: usize) -> &LegPair {
        &self.leg_pairs[k]
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

        // ── 读法B 一对多空腿分叉（任务57=53.1 编排者重写）：ON ⟹ 每级别 LegPair 买卖点开平 + 否定线止损 +
        //   链破坏 churn 门控（多空双开吃所有级别涨跌幅）。OFF ⟹ 逐字不动（bit-exact）。最先检查（旁路单腿/instances）。
        if self.enable_reading_b_pair {
            self.consume_leg_pairs(view, c);
            // 杠杆验收（裂隙2）：毛/净敞口相对 NAV（多空双开 ⇒ gross 含多+空）。
            let (mut gross, mut net) = (0.0f64, 0.0f64);
            for p in &self.leg_pairs {
                if p.long_units > EPS {
                    gross += p.long_units * c;
                    net += p.long_units * c;
                }
                if p.short_units > EPS {
                    gross += p.short_units * c;
                    net -= p.short_units * c;
                }
            }
            let nav = self.nav(c).max(1.0);
            self.max_gross_exp_x100 = self.max_gross_exp_x100.max((gross / nav * 100.0).max(0.0) as u64);
            self.max_net_exp_x100 = self.max_net_exp_x100.max((net.abs() / nav * 100.0).max(0.0) as u64);
            self.prove_tw_neutral(tw_pre, c);
            return;
        }

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
            // 杠杆验收（裂隙2）：毛/净敞口相对 NAV（恒仓 ⇒ ≤1×）。
            let (mut gross, mut net) = (0.0f64, 0.0f64);
            for leg in &self.legs {
                if leg.units > EPS {
                    let notional = leg.units * c;
                    gross += notional;
                    net += match leg.direction {
                        Polarity::Long => notional,
                        Polarity::Short => -notional,
                    };
                }
            }
            let nav = self.nav(c).max(1.0);
            self.max_gross_exp_x100 = self.max_gross_exp_x100.max((gross / nav * 100.0).max(0.0) as u64);
            self.max_net_exp_x100 = self.max_net_exp_x100.max((net.abs() / nav * 100.0).max(0.0) as u64);
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
        if self.enable_reading_b_pair {
            self.finish_leg_pairs(c);
            return self.free;
        }
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
