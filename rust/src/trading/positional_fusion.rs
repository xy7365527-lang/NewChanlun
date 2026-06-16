//! positional_fusion — hold26 × CounterSeg 合流（B+C 合体，2026-06-12 任务）。
//!
//! 谱系位置：534号（嵌套递归会计语义概念分离）的实装——矛盾消解点 =
//! 49课行52 同段二相：趋势相（中枢向上移动满仓 = 独立暴露）/震荡相
//! （围绕中枢差价上减下增 = 降成本短差）。两相 regime 正交，无全域赢家。
//! 原文架构判决：`analysis/bc_architecture_research.md`——B 管层间
//! （量按级别、k 级信号最多清 k 级净配额），C 管层内（同股数借出还回、
//! 目的=降成本），26课恒仓段（026:34）是合体的原文原型。
//!
//! ## 三机制（每机制课号锚定）
//!
//! 1. **基座 = hold26 恒仓极性**（026:34"仓位是一直不变的……卖点的时候变少，
//!    买点的时候又回复原来的数量，但绝对不加仓"）——per-layer slice 默认持有，
//!    本层任意 confirmed 卖点削减、任意买点回复（positional.rs 在册，
//!    BTC +1247% L2）。
//! 2. **趋势相停削**（049:52"中枢向上移动时，就应该满仓，这才是最正确的
//!    仓位"）——层 k 趋势相判据 = 尾 move kind==Trend ∧ dir==Up（17课趋势
//!    定义"≥2 同向中枢"的引擎直读：trend_flips × dir_flips 磁带行；引擎
//!    映射近似声明：49课"中枢向上移动"的对象是层 k 的走势，本实现以该层
//!    尾 move kind 与方向行的合取读出）。趋势相 ⇒ 本层卖点不削减（差价
//!    不能做）；震荡相 ⇒ hold26 在册削减。熊市（dir≠Up）行为与 hold26
//!    逐字相同——P2 熊市 α 的承载面零接触。
//! 3. **层内 C 短差（CounterSeg 词汇）**（053:34"在第二、三买卖点之间，
//!    都是中枢震荡……如果参与其中的买卖，用的都是低级别的买卖点" +
//!    049:64"在中枢上方全部抛出筹码，在下方如数接回"）——震荡相中
//!    k−1 级卖证据（confirmed Sell1 ∨ 盘背卖；`SubMode::CounterSeg`
//!    开腿谓词的 Short 侧逐字镜像）全抛本层 slice，k−1 级买证据
//!    （confirmed Buy1/Buy3 ∨ 盘背买；counterseg_close_trigger 镜像）
//!    如数接回。35课成本门（θ_q(k−1) 因果滚动中位数 ≥ k×friction）守卫
//!    级别可操作性；笔=a0（77/78课）⇒ k−1 < FIRST_BSP_LADDER 不开。
//!
//! ## 44课铰链（044:44，两线均未实现的新轴）
//!
//! > "就必须先出一部分，然后在出现上一段所说的情况时在出清。当然，如果
//! > 没有出现上一段所说的情况，就可以回补，权当弄了一个短差。"
//!
//! 层内 C 卖出**不预声明身份**——同一笔卖出的两种事后路径：
//! - k−1 买回证据先到 ⇒ 回补如数接回（事后归类为**短差**，C 路径）；
//! - 本层 k 级卖点先到（恶化升级条件的引擎映射：该买点起始的走势被本级别
//!   卖点宣告结束）⇒ **升级为减仓**（出清，B 路径）——trade 以实际变现价
//!   （短差卖出 bar 的 close）记账，exit_reason = "hinge_escalate"。
//! - 趋势相开始 ⇒ 强制回补（049:52 满仓义务——对象域消失）。
//!
//! ## 会计不变量
//!
//! 短差卖出 = 股数→现金的等价转换（卖出 bar close 单一成交价，现金入
//! pool）；接回 = 现金→同股数。trade 行的 exit_bar/exit_price 恒为实际
//! 变现点 ⇒ 逐 trade 行重建 NAV 与 final_nav 逐位一致（回测脚本 assert）。
//! 回补义务因资金不足推迟时逐 bar 重试（D7 推迟同构），且回补在阶段 A
//! 处理（先于一切新入场）——义务优先于配额（osc 僵尸腿判决：闭腿是义务）。
//!
//! ## C 轴资金解耦（capital_decoupled，fusion_e/fusion_se）
//!
//! 在册 fusion 判决的负交互机械根因 = 资金池耦合：T 轴满仓占资 ⇒ C 轴回补
//! 义务推迟暴涨（BTC 133K → 874K bar，6.6×），推迟期被迫空仓追价
//! （`hold26_counterseg_fusion_results.md` §3.3）。解耦 = **earmark**：
//! 短差卖出所得不入共享池，锁定为该层回补专款——049:64"如数接回"义务语义
//! 的资金面物理化（义务资金不可被其它层新入场挪用；优先权从"bar 内阶段 A
//! 先于阶段 C"升级为"跨 bar 不可挪用"）。53课:34"该级别能容纳的资金量……
//! 以后再说"留白区的资金语义之一，与耦合臂同为原文合法读法，回测裁决。
//! 资金守恒：escrow 计入 NAV（nav_fusion）；回补先专款后池（追价 deficit
//! 由池补差，topup 观测）；义务消灭点（铰链升级/eod）专款释放入池。
//! 统一代数 `可用 = escrow + pool`：耦合臂 escrow≡0 ⇒ 行为逐位同旧
//! （在册判决零漂移由构造保证，非守卫分支）。

use super::center_book::CenterBook;
use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
use crate::buysellpoint::Side;
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::positional::{
    enter_or_defer, theta_weights, LayerState, LayerTrade, OscRouting,
    PositionalResult, TrendAxisOpts, TrendScope, EQUITY_SAMPLE_BARS,
};
use super::tape::SignalTape;
use super::unified_osc::{OscLayer, OscOut};
use super::types::{
    BspClass, BspEvent, DivEvent, Polarity, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER,
};
use crate::divergence::DivKind;
use crate::stroke::Direction;

/// 35课成本门参数——逐字复用 `OrganicConfig` 默认（sub_cost_k=2.0,
/// sub_friction_rt=0.001；231号零新参数纪律；单测
/// `cost_gate_constants_match_organic_defaults` 守护漂移）。
/// pub(crate)：统一配置 U 的②振幅门（unified_osc.rs）逐字消费同常数。
pub(crate) const SUB_COST_K: f64 = 2.0;
pub(crate) const SUB_FRICTION_RT: f64 = 0.001;

/// 仓位分配 spawn 比例 f = m/p_units = 1/λ（σ-不变常数；编排者裁决 2026-06-16）。
/// **裁决依据（读法A：势∝r 是公理）**：r 标记级别=递归深度=势能，「势∝r」是径向坐标 r
/// 的**定义本身**（L0，信息增量为零，不可经验否定）⟹ 配额 = 子势/父势 = r_{k−1}/r_k =
/// λ^{k−1}/λ^k = **1/λ**，几何强制 σ-不变（T48 units=σ-不变 Casimir + T59 自相似 σWσ⁻¹=W）。
/// **替代**旧全局 θ_sub/θ_total 归一化（`theta_weights` 固定窗口 [FIRST_BSP,MAX) 不随 σ:k↦k+1
/// 平移 ⇒ f 随级别变 ⇒ 破 T59；环15「必然性争议」+ escalation theta-allocation 裁决，未改前
/// 第53课配额「留白」被「势∝r 公理」填补为 1/λ 形式）。
/// **值 = 1/λ**：λ（级别尺度比，A₅）是涌现量（T50 Δt(k)∝λ^k），未独立测 ⇒ SUB_SPAWN_FRAC
/// 作 leverage_triad 结算的**唯一自由度**（非违 231号零新参数——成本门 SUB_COST_K/FRICTION
/// 仍复用 OrganicConfig 经验 θ；本常数是 leverage_triad「唯一自由度=顶层配额」的显式化）回测
/// 扫描。初始 λ=2（二分递归默认）⟹ f=0.5；待全8标的回测扫描 + T50 涌现 λ 测量精化为
/// f=1/λ_measured 的零自由度 R1 形式（扫得最优 f≈1/λ_emergent 则势∝r 公理获 L2 经验旁证）。
/// pub(crate)：unn `try_spawn_cost_gated` 消费（角色B 配额比例；角色A 成本门仍用经验 θ）。
pub(crate) const SUB_SPAWN_FRAC: f64 = 0.5;

/// osc 层消费的相位三值视图（`unified_osc` 接口；P6 任务，
/// `analysis/p6_phase_machine_research.md`）。
///
/// 三个消费点：路由①门拒 `≠Osc`（049:52 前提"中枢震荡依旧" + 049:40
/// 不参与下跌——MOVE↓ 也上扫）、在外腿满仓义务 `==MoveUp`（049:52）、
/// 44课铰链抑制 `==MoveUp`（停削语义对在外腿成立）。
///
/// KindDir 时钟（在册 fusion_t/fusion_u）映射：trend×dir==Up → MoveUp，
/// 否则 Osc（无 MoveDown）——该映射下 `≠Osc ⟺ ==MoveUp`，三个消费点与
/// 在册 phase_up 布尔行为逐位等值（零漂移由构造保证，非守卫分支）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PhaseView {
    Osc,
    MoveUp,
    MoveDown,
}

/// P6 相位机的锁定相位（confirmed 转移）。candidate 离开窗口由 CenterBook
/// `pending_departure` 承载（049:68 当下读法 + 价格回中枢否定 = "校正"），
/// 锁定相位 ∪ candidate 窗口 = 有效相位（`run_fusion` 的 phase_view 闭包）。
/// 初始 Osc：中枢尚未存在时无 MOVE 区间可言——停削不成立、osc 开腿由对象
/// 存在性检查把关（trend_state 初始 false 同构）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayerPhase {
    Osc,
    /// confirmed Buy3 锁定（锚 = 被三买终结的中枢 cs，049:60/62）。出口 =
    /// 新中枢结算（alive.seg_start > anchor，049:54）/ 向上背驰事件
    /// （049:54/42——EXIT 全抛强读法一期不启用）/ dir 翻 Down（结构兜底，
    /// b3_start 在册同构）。
    MoveUp { anchor_cs: i64 },
    /// confirmed Sell3 锁定——镜像（038:36；049:52"不能回补"+049:40）。
    MoveDown { anchor_cs: i64 },
}

/// 层内 C 短差的在外态（44课铰链：身份未决——回补=短差 / k 级卖点=减仓）。
#[derive(Debug, Clone, Copy)]
struct SubOut {
    /// 卖出的股数（= 卖出 bar 本层全部股数，049:64"全部抛出筹码"）。
    shares: f64,
    sell_bar: i64,
    sell_price: f64,
    /// 回补触发已发生但资金不足 ⇒ 逐 bar 重试（D7 推迟同构）。
    restore_due: bool,
    /// 回补义务专款（capital_decoupled：卖出所得 earmark 不入共享池，
    /// 049:64"如数接回"义务语义的资金面物理化——义务资金不可被其它层
    /// 新入场挪用）。耦合模式恒 0（所得直接入 pool）⇒ 下游全部资金公式
    /// 以 escrow + pool 统一书写，escrow=0 时逐位退化为耦合行为。
    escrow: f64,
}

/// 区间套正向定位窗口（nest_forward；027课精确大转折点寻找程序定理）。
///
/// 武装 = 本级别 candidate Type1/Type3 事件（= "本级别进入背驰段后"/
/// 38:258"第三类买卖对盘整结束的确认，最终也要看其内部结构的背驰"）；
/// 触发 = 次级别（k−1）第一个同侧证据（"到次级别去寻找背驰点"——chan99/
/// 0027:19）；否定 = 价格越过 candidate 极值（027:25"只要没有打破背驰段，
/// 就要密切注意"的逆否：打破即作废）。
#[derive(Debug, Clone, Copy)]
struct NestWin {
    /// candidate 事件端点价（背驰段极值；卖窗取 max 刷新 / 买窗取 min）。
    extreme: f64,
    /// candidate 锚中枢（confirmed 配对回填 lead 统计用；type1 可无锚）。
    cs: Option<i64>,
}

/// 次级别证据（区间套正向定位的触发词汇）。`sub = k−1`：
/// BSP 承载层（≥ FIRST_BSP_LADDER）= 任意同侧 BSP 事件 ∨ 同侧背驰事件
/// （candidate 亦可——区间套递归"将该过程反复进行下去"的一层截断读法：
/// 次级别 candidate = 次级别已进入背驰段）；bi 层（无事件流）= 本 bar
/// 方向翻转沿（SC 先例：dir 行是 bi 层唯一结构通道；用沿不用态——态在
/// 下跌语境恒真，会使窗口武装即触发退化为纯 candidate 消费）。
fn nest_sub_evidence(
    sub: usize,
    side: Side,
    evs: &[BspEvent],
    devs: &[DivEvent],
    flip_edge: Option<Direction>,
) -> bool {
    if sub >= FIRST_BSP_LADDER {
        evs.iter().any(|e| e.class.side() == side)
            || devs.iter().any(|d| d.side() == side)
    } else {
        let want = match side {
            Side::Sell => Direction::Down,
            Side::Buy => Direction::Up,
        };
        flip_edge == Some(want)
    }
}

/// 层内短差开（先卖）谓词——`SubMode::CounterSeg` 开腿三岔镜像的 Short 侧
/// 逐字复刻（confirmed Sell1 ∨ 盘背卖；type2/3 不在开腿词汇——震荡型）。
fn sub_open_trigger(evs: &[BspEvent], devs: &[DivEvent]) -> bool {
    evs.iter().any(|e| e.confirmed && e.class == BspClass::Sell1)
        || devs
            .iter()
            .any(|d| d.kind == DivKind::Consolidation && d.direction == Direction::Up)
}

/// 层内短差回补（后买）谓词——`counterseg_close_trigger` 的 Short 侧逐字
/// 镜像（confirmed Buy1 ∨ confirmed Buy3 ∨ 盘背买；Buy3 = 中枢向上离开 =
/// 向下反向段终结的更强结构陈述）。
fn sub_close_trigger(evs: &[BspEvent], devs: &[DivEvent]) -> bool {
    evs.iter()
        .any(|e| e.confirmed && (e.class == BspClass::Buy1 || e.class == BspClass::Buy3))
        || devs
            .iter()
            .any(|d| d.kind == DivKind::Consolidation && d.direction == Direction::Down)
}

/// NAV：pool + 专款 + 各层在市股数（短差在外的层其价值是现金——耦合模式
/// 在 pool、解耦模式在该层 escrow；两种表示 NAV 恒等。U 的 osc 在外腿
/// 同理：所得已在 pool——耦合资金语义 escrow≡0，跳过股数计值）。
fn nav_fusion(
    layers: &[LayerState; MAX_LADDER],
    subs: &[Option<SubOut>; MAX_LADDER],
    oscs: &[Option<OscOut>; MAX_LADDER],
    cash: f64,
    c: f64,
    floor: usize,
) -> f64 {
    let mut v = cash;
    for k in floor..MAX_LADDER {
        if let Some(sub) = subs[k] {
            v += sub.escrow;
            continue;
        }
        if oscs[k].is_some() {
            continue;
        }
        match layers[k] {
            LayerState::Long { shares, .. } => v += shares * c,
            // 空头层权益 = margin + units×(B_s − c)（虚拟逐仓，会计文档
            // §4.3 NAV 恒等式的逐仓口径；浮亏可为负——强平在阶段 C 收口）。
            LayerState::Short { entry_price, units, margin, .. } => {
                v += margin + units * (entry_price - c);
            }
            _ => {}
        }
    }
    v
}

/// 回补尝试（如数接回）。成功 ⇒ 推入在外窗口 trade 行（"sub_diff"）、
/// 本层 Long 周期重锚到当下（entry_bar/entry_price = 接回点）、清 sub 态；
/// pool 不足 ⇒ 置 restore_due 逐 bar 重试并计数。
#[allow(clippy::too_many_arguments)]
fn try_restore(
    k: usize,
    c: f64,
    bar: i64,
    reason: &'static str,
    pool: &mut f64,
    layers: &mut [LayerState; MAX_LADDER],
    subs: &mut [Option<SubOut>; MAX_LADDER],
    res: &mut PositionalResult,
) -> bool {
    let sub = subs[k].expect("调用前提：subs[k] 为 Some");
    let cost = sub.shares * c;
    // 统一资金代数：可用 = 专款 + 池（耦合 escrow=0 ⇒ 判据/扣减逐位同旧）。
    if cost > sub.escrow + *pool {
        subs[k] = Some(SubOut { restore_due: true, ..sub });
        res.n_sub_restore_defer_bars += 1;
        return false;
    }
    let LayerState::Long { entry_bar, entry_price, shares, weight, deferred_bars, partial } =
        layers[k]
    else {
        unreachable!("sub 在外 ⇒ 本层恒 Long（sub 生命周期内层不变迁）")
    };
    debug_assert!((shares - sub.shares).abs() < 1e-12, "全抛/如数接回 ⇒ 股数恒等");
    // 先专款后池：escrow > cost ⇒ 盈余入池（差价利润释放为自由资金）；
    // escrow < cost ⇒ 池补差（追价 regime，topup 观测）。
    if sub.escrow > 0.0 && cost > sub.escrow {
        res.n_sub_pool_topup_by_ladder[k] += 1;
    }
    *pool += sub.escrow - cost;
    res.sub_net_cash_by_ladder[k] += sub.shares * sub.sell_price - cost;
    res.trades.push(LayerTrade {
        ladder: k as u8,
        entry_bar,
        entry_price,
        exit_bar: sub.sell_bar,
        exit_price: sub.sell_price,
        shares: sub.shares,
        weight_at_entry: weight,
        deferred_bars,
        partial,
        exit_reason: reason,
        polarity: Polarity::Long,
    });
    layers[k] = LayerState::Long {
        entry_bar: bar,
        entry_price: c,
        shares,
        weight,
        deferred_bars: 0,
        partial,
    };
    subs[k] = None;
    true
}

/// 主入口（`PolarityMode::Fusion` 经 `run_positional` 分派至此）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_fusion(
    tape: &SignalTape,
    floor_ladder: usize,
    trend_hold: bool,
    counter_sub: bool,
    capital_decoupled: bool,
    opts: TrendAxisOpts,
    osc_routing: OscRouting,
    phase_clock: bool,
    r2_gate: bool,
    trend_scope: TrendScope,
    nest_forward: bool,
    short_mask: u16,
    short_anc_gate: bool,
    short_ghost: bool,
    seq38_sub: bool,
) -> Result<PositionalResult, String> {
    if !(FIRST_BSP_LADDER..MAX_LADDER).contains(&floor_ladder) {
        return Err(format!(
            "fusion 要求 floor_ladder ∈ [{FIRST_BSP_LADDER}, {MAX_LADDER})\
             （BSP 承载层）；floor_ladder={floor_ladder}"
        ));
    }
    if !tape.has_bsp_events() {
        return Err("fusion 要求事件磁带（bsp_events 全空）".to_string());
    }
    if !(trend_hold || counter_sub) {
        return Err(
            "Fusion{trend_hold:false, counter_sub:false} 是 Hold26 的冗余表示\
             ——声明=能力，显式拒绝（Recursive base_frac=1 先例）"
                .to_string(),
        );
    }
    if capital_decoupled && !counter_sub {
        return Err(
            "capital_decoupled（C 轴回补义务专款）要求 counter_sub——专款的\
             唯一对象是短差回补义务（049:64），无 C 轴即无对象，显式拒绝"
                .to_string(),
        );
    }
    if opts.any() && !trend_hold {
        return Err(
            "TrendAxisOpts（41课衰竭门/49课:54 背驰出场/49课:60 三买起点）\
             是 T 轴的修饰子——无 trend_hold 即无对象，显式拒绝"
                .to_string(),
        );
    }
    if opts.gate41 && !tape.has_div_events() {
        return Err(
            "gate41（41课:22 大级别未衰竭门）要求背驰磁带（div_events 全空）\
             ——衰竭证据 = 父层向下背驰事件（41课:28\"没有进入背驰段，就\
             不能操作\"），缺行即判据无数据基础（不静默降级为纯方向门）"
                .to_string(),
        );
    }
    if trend_hold && !phase_clock && !(tape.has_trend_rows() && tape.has_dir_rows()) {
        return Err(
            "trend_hold（49课:52 二相停削）要求磁带 trend_flips + dir_flips 行\
             ——趋势相判据 = kind==Trend ∧ dir==Up（17课趋势定义），无行即\
             判据无数据基础（不提供方向行单独代理降级：方向 ≠ 趋势）"
                .to_string(),
        );
    }
    if phase_clock {
        // P6 相位机守卫（声明=能力；p6_phase_machine_research.md §5）。
        if !trend_hold {
            return Err(
                "phase_clock（P6 相位机时钟）的对象是停削窗口——无 trend_hold \
                 即无对象，显式拒绝"
                    .to_string(),
            );
        }
        if counter_sub {
            return Err(
                "phase_clock × counter_sub 未预注册（P6 配对臂'其余全同'条款），\
                 显式拒绝"
                    .to_string(),
            );
        }
        if opts.any() {
            return Err(
                "phase_clock × TrendAxisOpts 未预注册——b3_start 三买窗口已被\
                 相位机收编（MOVE↑ 锁定转移），gate41/div_exit 组合未预注册，\
                 显式拒绝"
                    .to_string(),
            );
        }
        if !(tape.has_dir_rows() && tape.has_div_events()) {
            return Err(
                "phase_clock（P6 相位机）要求磁带 dir_flips 行 + 背驰磁带——\
                 MOVE 区间的方向兜底出口与背驰出口缺行即判据无数据基础；\
                 kind 行零消费故不要求（错位时钟退役，research v2 §2.2）"
                    .to_string(),
            );
        }
    }
    if r2_gate {
        // P7 R2 位置门守卫（535 号裁决实验；research §2.3/§6 P7 预注册）。
        if !trend_hold {
            return Err(
                "r2_gate（P7 R2 位置门）定义在二相基座的震荡相削减词汇上——\
                 无 trend_hold 即无对象，显式拒绝"
                    .to_string(),
            );
        }
        if counter_sub {
            return Err("r2_gate × counter_sub 未预注册，显式拒绝".to_string());
        }
        if opts.any() {
            return Err("r2_gate × TrendAxisOpts 未预注册，显式拒绝".to_string());
        }
    }
    if nest_forward {
        // 区间套正向定位守卫（027课程序定理；预注册臂：fusion_tn/fusion_trn
        // ——t 基座 ± r2 位置门；fusion_btran_s{digits}——btra 双向基座，
        // 全量普适组合 2026-06-12，nest × short 形态由 short_mask 守卫管）。
        if !trend_hold {
            return Err(
                "nest_forward（区间套正向定位）替换的是 hold26 削减/回复词汇\
                 的触发时机，预注册基座 = fusion_t（trend_hold）——其余基座\
                 组合未预注册，显式拒绝"
                    .to_string(),
            );
        }
        if counter_sub || opts.any() || phase_clock {
            return Err(
                "nest_forward 仅预注册 fusion_tn/fusion_trn/fusion_btra{mods} \
                 臂——与 counter_sub/TrendAxisOpts/phase_clock 的合取未预注册，\
                 显式拒绝"
                    .to_string(),
            );
        }
        if osc_routing != OscRouting::Off && !(short_mask != 0 && short_anc_gate) {
            return Err(
                "nest_forward × osc 仅预注册 fusion_btra{mods} 双向基座\
                 （全量普适组合 2026-06-12 消融矩阵）——fusion_tn/trn × osc \
                 未预注册，显式拒绝"
                    .to_string(),
            );
        }
        if !(tape.has_div_events() && tape.has_dir_rows()) {
            return Err(
                "nest_forward 要求背驰磁带 + dir_flips 行——次级别证据词汇 = \
                 k−1 BSP 事件 ∨ k−1 背驰事件 ∨ bi 层方向翻转沿（SC 先例：bi \
                 层无事件流时方向行是唯一通道），缺行即词汇残缺"
                    .to_string(),
            );
        }
    }
    if trend_scope != TrendScope::SelfLayer {
        // anc 祖先趋势豁免守卫（26:80 下沉；slow_bull 调研 §6 仅预注册
        // hold26_anc/fusion_ta 两臂——其余轴合取未预注册，显式拒绝）。
        if !trend_hold {
            return Err(
                "trend_scope（anc 祖先趋势豁免，26:80）是停削时钟的作用域\
                 ——无 trend_hold 即无对象，显式拒绝"
                    .to_string(),
            );
        }
        if phase_clock {
            return Err(
                "trend_scope × phase_clock 未预注册——anc 判据定义在 KindDir \
                 行（17课 ≥2 同向中枢），相位机时钟 kind 行零消费，两时钟\
                 合取无预注册语义，显式拒绝"
                    .to_string(),
            );
        }
        if counter_sub || opts.any() || r2_gate || osc_routing != OscRouting::Off || nest_forward
        {
            return Err(
                "trend_scope（anc）仅预注册 hold26_anc/fusion_ta 两臂\
                 （slow_bull §6）——与 counter_sub/TrendAxisOpts/r2_gate/osc/\
                 nest_forward 的合取未预注册，显式拒绝"
                    .to_string(),
            );
        }
    }
    if short_mask != 0 {
        // 双向条件轴 S1-S4 守卫 [镜像推导]（声明=能力；会计文档 §8 +
        // slow_bull §7.6）。仅预注册 fusion_tr 基座合取（探针2 对照臂）。
        if !(trend_hold && r2_gate) {
            return Err(
                "short_mask（双向条件轴）仅预注册 fusion_tr 基座（trend_hold \
                 ∧ r2_gate——探针2 的对照臂口径）；其它基座合取未预注册，\
                 显式拒绝"
                    .to_string(),
            );
        }
        if counter_sub || opts.any() || phase_clock
            || trend_scope != TrendScope::SelfLayer || capital_decoupled
        {
            return Err(
                "short_mask × {counter_sub/TrendAxisOpts/phase_clock/\
                 trend_scope/decoupled} 合取未预注册，显式拒绝"
                    .to_string(),
            );
        }
        if nest_forward && !(short_anc_gate && !short_ghost) {
            return Err(
                "short_mask × nest_forward 仅预注册 anc 门形态\
                 （fusion_btra{mods}_s{digits}，全量普适组合 2026-06-12）——\
                 btr 无 anc / btrg ghost 与 nest 的合取未预注册，显式拒绝"
                    .to_string(),
            );
        }
        if osc_routing != OscRouting::Off && !(short_anc_gate && !short_ghost) {
            return Err(
                "short_mask × osc（统一 osc 层 = H4 承载臂）仅预注册 anc 门\
                 形态（fusion_btra{mods}_s{digits}，全量普适组合 2026-06-12 \
                 消融矩阵）——slice custody 分区：翻空白名单层由双向词汇\
                 独占，osc 仅非白名单层激活；btr/btrg 合取未预注册，显式拒绝"
                    .to_string(),
            );
        }
        if seq38_sub && !(short_anc_gate && !short_ghost) {
            return Err(
                "short_mask × seq38_sub（Sequence38 子腿声部）仅预注册 anc 门\
                 形态（fusion_btra{mods}_s{digits}，全量普适组合 2026-06-12 \
                 消融矩阵）——btr/btrg 合取未预注册，显式拒绝"
                    .to_string(),
            );
        }
        if short_ghost && short_anc_gate {
            return Err(
                "short_ghost（纯回复门消融臂）的对照臂 = fusion_btr——与 \
                 short_anc_gate 合取未预注册（消融不混 anc 门），显式拒绝"
                    .to_string(),
            );
        }
        if short_mask & !0b11100u16 != 0 {
            return Err(
                "short_mask 位必须 ⊆ {2,3,4}（segment/move/recL2）——S3 尾部\
                 风险界：高层（≥recL3）空头默认禁用（GC recL4 因果空头单窗口 \
                 −0.806 nats 反例，slow_bull §7.6）；非 BSP 承载层无卖点词汇"
                    .to_string(),
            );
        }
    }
    if counter_sub && !tape.has_div_events() {
        return Err(
            "counter_sub（层内 C 短差）要求背驰磁带（div_events 全空）——\
             盘背是开/回补词汇的结构分量（CounterSeg 三岔镜像），缺行即词汇\
             残缺（不静默降级为纯事件词汇）"
                .to_string(),
        );
    }
    if seq38_sub {
        // Sequence38 子腿守卫（38课:36 程式下沉；全量普适组合 2026-06-12
        // 仅预注册 fusion_btra{mods} 双向基座的消融臂）。
        if short_mask == 0 {
            return Err(
                "seq38_sub 仅预注册 fusion_btra{mods}_s{digits} 双向基座\
                 ——其余基座合取未预注册，显式拒绝"
                    .to_string(),
            );
        }
        if counter_sub {
            return Err(
                "seq38_sub 与 counter_sub 互斥——两词汇驱动同一 SubOut 载具\
                 （同一笔筹码不能同时按两种节奏在外），组合未定义，显式拒绝"
                    .to_string(),
            );
        }
        if !(tape.has_div_events() && tape.has_dir_rows()) {
            return Err(
                "seq38_sub 要求背驰磁带 + dir_flips 行——盘背事件是开/闭腿\
                 词汇本体（38课:36），不破第一段低点岔的次级别确认含方向行\
                 证据（答疑:296），缺行即词汇残缺"
                    .to_string(),
            );
        }
    }
    if osc_routing != OscRouting::Off {
        // 统一配置 U 的定义域守卫（fusion_u/fusion_uw）：U := fusion_t 基座
        // + 相位递归路由 osc 层（research §3.2）。基座之外的组合未预注册，
        // 显式拒绝（声明=能力）。
        if !trend_hold {
            return Err(
                "OscRouting::Unified 定义在 fusion_t 基座上（049:52 二相停削\
                 ——①相位门与基座共用同一趋势相时钟）；trend_hold=false 组合\
                 未预注册，显式拒绝"
                    .to_string(),
            );
        }
        if counter_sub {
            return Err(
                "OscRouting::Unified 与 counter_sub 互斥——两机制都全抛本层\
                 slice（同一笔筹码不能同时按两种节奏在外），组合未定义，\
                 显式拒绝"
                    .to_string(),
            );
        }
        if opts.any() {
            return Err(
                "OscRouting::Unified × TrendAxisOpts 组合未预注册（U 的基座 = \
                 fusion_t 逐字，T 轴严格化臂在册判决均为否证/近 no-op），\
                 显式拒绝"
                    .to_string(),
            );
        }
        // ①相位门与③参照所需磁带行由时钟对应守卫强制：KindDir ⇒ trend+dir
        // 行（trend_hold 守卫）；phase_clock ⇒ dir+div 行（相位机守卫）。
    }

    let n = tape.bars.len();
    let mut res = PositionalResult::default();
    let mut layers: [LayerState; MAX_LADDER] = [LayerState::Flat; MAX_LADDER];
    let mut subs: [Option<SubOut>; MAX_LADDER] = [None; MAX_LADDER];
    let mut pool = INITIAL_CAPITAL;
    let mut book = CenterBook::new();
    let mut depth_ref = DepthRef::new(DEPTH_REF_WINDOW);
    // 统一配置 U 的 osc 层（Off 时恒空——fusion_t 在册路径零接触退化）。
    let mut osc_layer = OscLayer::new();
    let (osc_on, osc_strong, osc_h1) = match osc_routing {
        OscRouting::Off => (false, false, false),
        OscRouting::Unified { strong_gate, h1_freeze } => (true, strong_gate, h1_freeze),
    };

    // D3/趋势滚动状态（runner.rs 同构：稀疏翻转行 → 逐 bar 视图）。
    let flips: &[(i64, u8, Direction)] = tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;
    let mut dir_state: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
    let tflips: &[(i64, u8, bool)] = tape.trend_flips.as_deref().unwrap_or(&[]);
    let mut tflip_ptr = 0usize;
    let mut trend_state: [bool; MAX_LADDER] = [false; MAX_LADDER];
    // gate41：层 k 当前方向 run 内是否已现向下背驰（衰竭证据）。dir 翻转清零。
    let mut down_exhaust: [bool; MAX_LADDER] = [false; MAX_LADDER];
    // b3_start：三买窗口锚（confirmed Buy3 的中枢 cs）。None = 窗口关闭。
    let mut b3_anchor: [Option<i64>; MAX_LADDER] = [None; MAX_LADDER];
    // P6 相位机锁定相位（phase_clock；candidate 窗口在 CenterBook）。
    let mut phi: [LayerPhase; MAX_LADDER] = [LayerPhase::Osc; MAX_LADDER];
    // 区间套正向定位窗口（nest_forward；其余模式恒 None = 在册零接触）。
    let mut nest_sell: [Option<NestWin>; MAX_LADDER] = [None; MAX_LADDER];
    let mut nest_buy: [Option<NestWin>; MAX_LADDER] = [None; MAX_LADDER];
    // 正向触发 → 事后 confirmed 配对（lead 统计）：(ladder, 卖侧?, cs) → fire_bar。
    let mut nest_fired: std::collections::HashMap<(usize, bool, i64), i64> =
        std::collections::HashMap::new();
    // Sequence38 子腿状态（seq38_sub；38课:36 程式——其余模式恒初值零接触）：
    // seg1_low = 开腿时冻结的第一段低点（run_low 快照）；run_low = 节点激活/
    // 上次买回以来 close 运行最低（每 bar 更新，含本 bar）；low_since_open =
    // 在外期间 close 最低（不破第一段低点岔的判据量）。
    let mut seq_seg1_low: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
    let mut seq_run_low: [f64; MAX_LADDER] = [f64::INFINITY; MAX_LADDER];
    let mut seq_low_since_open: [f64; MAX_LADDER] = [f64::INFINITY; MAX_LADDER];

    let empty_evs: [Vec<BspEvent>; MAX_LADDER] = Default::default();
    let empty_devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();

    for i in 0..n {
        let sig = &tape.bars[i];
        let c = sig.close;
        // bi 层方向翻转沿（本 bar；nest_forward 次级别证据的 bi 通道）。
        let mut flip_edge: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];

        while flip_ptr < flips.len() && flips[flip_ptr].0 == i as i64 {
            let (_, lad, dir) = flips[flip_ptr];
            dir_state[lad as usize] = Some(dir);
            flip_edge[lad as usize] = Some(dir);
            // 方向 run 重置 ⇒ 上一 run 的衰竭证据失效（gate41 状态）。
            down_exhaust[lad as usize] = false;
            // 049:60 三买窗口：dir 翻 Down = 向上离开失败的结构证据 ⇒ 关窗。
            if dir == Direction::Down {
                b3_anchor[lad as usize] = None;
            }
            // P6 相位机：方向翻转 = MOVE 锁定区间的结构兜底出口（无新中枢、
            // 无背驰事件而方向已翻 ⇒ 区间不得悬置；b3_start 同构）。
            if phase_clock {
                let l = lad as usize;
                match (phi[l], dir) {
                    (LayerPhase::MoveUp { .. }, Direction::Down) => {
                        phi[l] = LayerPhase::Osc;
                        res.n_phase_up_dir_closes_by_ladder[l] += 1;
                    }
                    (LayerPhase::MoveDown { .. }, Direction::Up) => {
                        phi[l] = LayerPhase::Osc;
                        res.n_phase_dn_closes_by_ladder[l] += 1;
                    }
                    _ => {}
                }
            }
            flip_ptr += 1;
        }
        while tflip_ptr < tflips.len() && tflips[tflip_ptr].0 == i as i64 {
            let (_, lad, is_trend) = tflips[tflip_ptr];
            trend_state[lad as usize] = is_trend;
            tflip_ptr += 1;
        }

        // 市场性质：中枢账本 + 振幅参照（与 run_positional 逐字同构）。
        if let Some(evrows) = sig.bsp_events.as_deref() {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                book.ingest(lad, &evrows[lad], true, None);
            }
            depth_ref.observe(&book, c);
        }
        let evrows: &[Vec<BspEvent>; MAX_LADDER] =
            sig.bsp_events.as_deref().unwrap_or(&empty_evs);
        let devrows: &[Vec<DivEvent>; MAX_LADDER] =
            sig.div_events.as_deref().unwrap_or(&empty_devs);

        // ③门参照刷新（093:26"上涨的最后一个中枢"——dir==Up 期间跟随存活
        // 中枢，翻落即冻结快照；市场性质，与持仓无关）。
        if osc_on {
            osc_layer.observe_refs(&book, &dir_state);
        }
        // H1 candidate 冻结窗口的价格否定（h1_freeze；049:68 当下读法——
        // 回试跌回中枢边界内 ⇒ 窗口关闭。phase_clock 路径已有同款调用且
        // 两者守卫互斥不可同现；其余模式零接触）。
        if osc_h1 {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                book.negate_pending_departure(lad, c);
            }
        }
        // Sequence38 第一段低点参照推进（seq38_sub；含本 bar——LOU run_low
        // 同语义。市场性质，与持仓无关；其余模式恒 INFINITY 零消费）。
        if seq38_sub {
            for k in FIRST_BSP_LADDER..MAX_LADDER {
                seq_run_low[k] = seq_run_low[k].min(c);
            }
        }

        // ── P6 相位机（phase_clock；市场性质，事件 bar 即时生效——与 dir/
        //    trend 行翻转语义一致）。更新序：candidate 窗口价格否定（049:68
        //    当下读法的"校正"——回试跌回中枢 = 仍是中枢震荡）→ confirmed
        //    Buy3/Sell3 锁定转移 → settle(C′) 关 → 背驰关 ──
        if phase_clock {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                book.negate_pending_departure(lad, c);
                if sig.bsp_events.is_some() {
                    for e in &evrows[lad] {
                        if !e.confirmed {
                            continue;
                        }
                        let Some(cs) = e.cs else { continue };
                        match e.class {
                            BspClass::Buy3 => {
                                phi[lad] = LayerPhase::MoveUp { anchor_cs: cs };
                                res.n_phase_up_opens_by_ladder[lad] += 1;
                            }
                            BspClass::Sell3 => {
                                phi[lad] = LayerPhase::MoveDown { anchor_cs: cs };
                                res.n_phase_dn_opens_by_ladder[lad] += 1;
                            }
                            _ => {}
                        }
                    }
                }
                // settle(C′)：锚中枢已被 confirmed type3 终结，存活中枢
                // seg_start 单调 ⇒ alive > anchor 即新中枢结算（049:54/60
                // "直到新中枢出现"）。陈旧确认（锁定时 C′ 已在）同 bar 即关
                // ——自校正，不产生悬置区间。
                match phi[lad] {
                    LayerPhase::MoveUp { anchor_cs } => {
                        if book.alive(lad).is_some_and(|lc| lc.seg_start > anchor_cs) {
                            phi[lad] = LayerPhase::Osc;
                            res.n_phase_up_settle_closes_by_ladder[lad] += 1;
                        } else if sig.div_events.is_some()
                            && devrows[lad].iter().any(|d| d.direction == Direction::Up)
                        {
                            // 049:54 移动背驰 / 049:42 盘背 ⇒ MOVE↑ 终结
                            //（EXIT 全抛强读法一期不启用——相位翻 OSC 后由
                            // 卖点词汇近似承载出场，research §5.1 在册声明）。
                            phi[lad] = LayerPhase::Osc;
                            res.n_phase_up_div_closes_by_ladder[lad] += 1;
                        }
                    }
                    LayerPhase::MoveDown { anchor_cs } => {
                        if book.alive(lad).is_some_and(|lc| lc.seg_start > anchor_cs) {
                            phi[lad] = LayerPhase::Osc;
                            res.n_phase_dn_closes_by_ladder[lad] += 1;
                        } else if sig.div_events.is_some()
                            && devrows[lad]
                                .iter()
                                .any(|d| d.direction == Direction::Down)
                        {
                            phi[lad] = LayerPhase::Osc;
                            res.n_phase_dn_closes_by_ladder[lad] += 1;
                        }
                    }
                    LayerPhase::Osc => {}
                }
            }
        }

        // ── 区间套正向定位（nest_forward；市场性质，与持仓无关）。更新序：
        //    ① 打破否定（027:25"只要没有打破背驰段"的逆否——价格越过
        //    candidate 极值 ⇒ 窗口作废）→ ② 本级别 Type1/Type3 事件
        //    （candidate 武装/刷新；confirmed 到达 ⇒ 窗口让位基线掩码路径
        //    + lead 配对回填）→ ③ 次级别第一个同侧证据 ⇒ 正向触发
        //    （chan99/0027:19"本级别进入背驰段后，到次级别去寻找背驰点"；
        //    038:258"涨的时候一旦进入背驰的区间套里，就要陆续走"）──
        let mut nf_sell = [false; MAX_LADDER];
        let mut nf_buy = [false; MAX_LADDER];
        if nest_forward {
            for k in FIRST_BSP_LADDER..MAX_LADDER {
                // ① 打破背驰段 ⇒ 作废（卖窗：创新高；买窗：创新低）。
                if nest_sell[k].is_some_and(|w| c > w.extreme) {
                    nest_sell[k] = None;
                    res.n_nest_breaks_by_ladder[k] += 1;
                }
                if nest_buy[k].is_some_and(|w| c < w.extreme) {
                    nest_buy[k] = None;
                    res.n_nest_breaks_by_ladder[k] += 1;
                }
                // ② Type1（背驰段 proper）/Type3（38:258 内部结构确认）事件。
                //    Type2 不在对象域：其 confirmed 是同 bar 价格比较，无
                //    时间等待 ⇒ 无滞后可消。
                if sig.bsp_events.is_some() {
                    for e in &evrows[k] {
                        let sellside = match e.class {
                            BspClass::Sell1 | BspClass::Sell3 => true,
                            BspClass::Buy1 | BspClass::Buy3 => false,
                            BspClass::Sell2 | BspClass::Buy2 => continue,
                        };
                        let win = if sellside { &mut nest_sell[k] } else { &mut nest_buy[k] };
                        if e.confirmed {
                            *win = None;
                            if let Some(cs) = e.cs {
                                if let Some(fb) = nest_fired.remove(&(k, sellside, cs)) {
                                    res.nest_lead_bars_sum += (i as i64 - fb) as u64;
                                    res.nest_lead_n += 1;
                                }
                            }
                        } else {
                            let ext = win.map_or(e.price, |w| {
                                if sellside {
                                    w.extreme.max(e.price)
                                } else {
                                    w.extreme.min(e.price)
                                }
                            });
                            let cs = e.cs.or(win.and_then(|w| w.cs));
                            *win = Some(NestWin { extreme: ext, cs });
                            res.n_nest_arms_by_ladder[k] += 1;
                        }
                    }
                }
                // ③ 次级别证据触发（一次性：触发即清窗，等下一个 candidate）。
                let sub = k - 1;
                if let Some(w) = nest_sell[k] {
                    if nest_sub_evidence(
                        sub, Side::Sell, &evrows[sub], &devrows[sub], flip_edge[sub],
                    ) {
                        nf_sell[k] = true;
                        nest_sell[k] = None;
                        res.n_nest_fire_sell_by_ladder[k] += 1;
                        if let Some(cs) = w.cs {
                            nest_fired.insert((k, true, cs), i as i64);
                        }
                    }
                }
                if let Some(w) = nest_buy[k] {
                    if nest_sub_evidence(
                        sub, Side::Buy, &evrows[sub], &devrows[sub], flip_edge[sub],
                    ) {
                        nf_buy[k] = true;
                        nest_buy[k] = None;
                        res.n_nest_fire_buy_by_ladder[k] += 1;
                        if let Some(cs) = w.cs {
                            nest_fired.insert((k, false, cs), i as i64);
                        }
                    }
                }
            }
        }

        // gate41 衰竭证据 / b3 窗口的事件流更新（在相位判定之前——同 bar
        // 事件即时生效，与 dir/trend 行的翻转语义一致）。
        if opts.gate41 && sig.div_events.is_some() {
            for lad in 0..MAX_LADDER {
                if devrows[lad].iter().any(|d| d.direction == Direction::Down) {
                    down_exhaust[lad] = true;
                }
            }
        }
        if opts.b3_start {
            if sig.bsp_events.is_some() {
                for lad in 0..MAX_LADDER {
                    let mut new_b3: Option<i64> = None;
                    let mut max_cs: Option<i64> = None;
                    for e in &evrows[lad] {
                        if let Some(cs) = e.cs {
                            max_cs = Some(max_cs.map_or(cs, |m| m.max(cs)));
                        }
                        if e.confirmed && e.class == BspClass::Buy3 {
                            if let Some(cs) = e.cs {
                                new_b3 = Some(new_b3.map_or(cs, |m| m.max(cs)));
                            }
                        }
                    }
                    if let Some(b) = new_b3 {
                        b3_anchor[lad] = Some(b); // 049:60 三买 ⇒ 开窗
                    }
                    if let (Some(anchor), Some(m)) = (b3_anchor[lad], max_cs) {
                        if m > anchor {
                            b3_anchor[lad] = None; // 新中枢出现 ⇒ 回到震荡相
                        }
                    }
                }
            }
            // 049:42/46：三买后向上走势出现背驰/盘背 ⇒ 离开段衰竭，关窗
            //（独立于 bsp 行——背驰事件可单独到达）。
            if sig.div_events.is_some() {
                for lad in 0..MAX_LADDER {
                    if b3_anchor[lad].is_some()
                        && devrows[lad].iter().any(|d| d.direction == Direction::Up)
                    {
                        b3_anchor[lad] = None;
                    }
                }
            }
        }

        // 相位三值视图（osc 层 + phase_clock 停削的共用时钟）：
        //   KindDir 时钟 = trend_state（17课 ≥2 同向中枢）∧ dir==Up → MoveUp，
        //     否则 Osc（在册 fusion_t/fusion_u 行为，无 MoveDown）；
        //   相位机时钟 = 锁定相位 phi ∪ candidate 离开窗口（049:68 当下读法，
        //     pending_departure Buy 侧 = MOVE↑ / Sell 侧 = MOVE↓）。
        let phase_view = |k: usize| -> PhaseView {
            if phase_clock {
                match phi[k] {
                    LayerPhase::MoveUp { .. } => PhaseView::MoveUp,
                    LayerPhase::MoveDown { .. } => PhaseView::MoveDown,
                    LayerPhase::Osc => match book.pending_departure_side(k) {
                        Some(Side::Buy) => PhaseView::MoveUp,
                        Some(Side::Sell) => PhaseView::MoveDown,
                        None => PhaseView::Osc,
                    },
                }
            } else if trend_state[k] && dir_state[k] == Some(Direction::Up) {
                PhaseView::MoveUp
            } else {
                PhaseView::Osc
            }
        };
        // 相位驻留观测（每 bar，有效相位口径——含 candidate 窗口；P6 机制
        // 可观测性：停削窗口大小与 kind×dir 窗口直接可比）。
        if phase_clock {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                match phase_view(lad) {
                    PhaseView::MoveUp => res.phase_up_bars_by_ladder[lad] += 1,
                    PhaseView::MoveDown => res.phase_dn_bars_by_ladder[lad] += 1,
                    PhaseView::Osc => {}
                }
            }
        }
        // 层 k 趋势相（49课:52"中枢向上移动"）。KindDir 时钟两分量：
        //   kind 直读 = trend_state（17课 ≥2 同向中枢）∧ dir==Up（在册 fusion_t）；
        //   b3 窗口 = 049:60 三买后新中枢前（kind 判据的结构滞后区，b3_start）。
        // 相位机时钟：停削窗口 = 有效 MOVE↑（与 osc ①门同一时钟——双侧同步，
        // P6 任务的核心条款）。raw = 未过 41课门的相位；gate41 在 raw 之上
        // 叠加大级别未衰竭否决（phase_clock 下 opts 已拒，gate41 恒放行）。
        // anc 豁免窗口（26:80"更大级别的单边上扬"）：∃ j > k 祖先层
        // kind==Trend ∧ dir==Up（17课 ≥2 同向中枢 = 中枢序列单调上移的引擎
        // 直读）。单调性打破（祖先 trend 翻落或 dir 翻 Down）⇒ 窗口即关，
        // 削减恢复（35:16"第N个中枢不再高于第N-1个才可说上涨结束"）。
        let anc_up = |k: usize| {
            ((k + 1)..MAX_LADDER)
                .any(|j| trend_state[j] && dir_state[j] == Some(Direction::Up))
        };
        // 自层 KindDir 窗口（在册 fusion_t；b3 窗口是其结构滞后区补丁）。
        let self_up = |k: usize| {
            (trend_state[k] && dir_state[k] == Some(Direction::Up))
                || (opts.b3_start && b3_anchor[k].is_some())
        };
        let in_trend_raw = |k: usize| {
            trend_hold
                && if phase_clock {
                    phase_view(k) == PhaseView::MoveUp
                } else {
                    match trend_scope {
                        TrendScope::SelfLayer => self_up(k),
                        TrendScope::Ancestor => anc_up(k),
                        TrendScope::SelfOrAncestor => self_up(k) || anc_up(k),
                    }
                }
        };
        // anc 豁免窗口驻留观测（市场性质逐 bar；P7 occupancy 对齐——
        // 与调研 §1.4 E 桶占比直接可比；SelfLayer 恒零 = 在册零接触）。
        if trend_scope != TrendScope::SelfLayer {
            for lad in floor_ladder..MAX_LADDER {
                if anc_up(lad) {
                    res.anc_up_bars_by_ladder[lad] += 1;
                }
            }
        }
        let gate41_pass = |k: usize| {
            if !opts.gate41 {
                return true;
            }
            let p = k + 1;
            // 41课:22：父层向下且未衰竭 ⇒ 层 k 向上参与 = 刀口舔血，否决。
            !(p < MAX_LADDER && dir_state[p] == Some(Direction::Down) && !down_exhaust[p])
        };
        let in_trend = |k: usize| in_trend_raw(k) && gate41_pass(k);
        // P7 R2 位置门（049:52"在中枢上方仓位减少"——震荡相削减的位置分量）：
        // 域 = 震荡相精确（phase_clock ⇒ Φ(k)=OSC；KindDir ⇒ dir≠Down——
        // 熊市削减保留，位置门对象是中枢震荡非下行段，049:40 映射声明；
        // ¬tp 由调用位置保证）。无存活中枢 ⇒ 位置词汇无对象，卖点回退出场
        // 语义不拦截（049:54）。NaN ZG 比较恒 false ⇒ 拦截（保守方向，
        // CenterBook NaN 纪律同构）。回复侧位置门不在本轴（P7 最小差分）。
        let r2_blocked = |k: usize| {
            if !r2_gate {
                return false;
            }
            let in_domain = if phase_clock {
                phase_view(k) == PhaseView::Osc
            } else {
                dir_state[k] != Some(Direction::Down)
            };
            in_domain && book.alive(k).is_some_and(|lc| !(c >= lc.zg))
        };
        // ── 双向条件轴 S1-S4 谓词 [镜像推导]（short_mask=0 不可达）──
        // 镜像 anc 窗口（fusion_btra；S2 条件化形式）：∃ j > k 祖先层
        // kind==Trend ∧ dir==Down（26:80 豁免下沉的空头镜像）。
        let anc_down = |k: usize| {
            ((k + 1)..MAX_LADDER)
                .any(|j| trend_state[j] && dir_state[j] == Some(Direction::Down))
        };
        // MoveDown 相（KindDir 时钟的 38:36 对称延拓：trend ∧ dir==Down）。
        // 49:52 镜像"中枢向下移动应满空仓、停止回补"——P6 MoveDown 相位的
        // 空头侧消费者（会计文档 §2.5）。
        let in_movedown = |k: usize| {
            trend_state[k] && dir_state[k] == Some(Direction::Down)
        };
        // 空侧 R2 回补位置门（049:64"在下方如数接回"镜像——P7 回复侧
        // 位置门获得对象）：震荡相中买点回补要求 c ≤ ZD（中枢下方）；
        // 上行段（dir==Up）回补不拦截（逃命语义，多头侧 dir≠Down 镜像）。
        // NaN ZD 比较恒 false ⇒ 拦截（保守方向同构）。
        let short_r2_blocked = |k: usize| {
            if !r2_gate {
                return false;
            }
            let in_domain = dir_state[k] != Some(Direction::Up);
            in_domain && book.alive(k).is_some_and(|lc| !(c <= lc.zd))
        };

        // ── 阶段 A：层级出场 / 44课铰链 / 回补（资金释放先于一切入场；
        //    回补义务在本阶段处理 ⇒ 对 pool 的优先权高于阶段 C 新入场）──
        for k in floor_ladder..MAX_LADDER {
            let LayerState::Long { entry_bar, entry_price, shares, weight, deferred_bars, partial } =
                layers[k]
            else {
                continue;
            };
            res.held_bars_by_ladder[k] += 1;
            let tp = in_trend(k);
            if osc_layer.outs[k].is_some() {
                // ── 统一配置 U：osc 在外腿出口集（满仓义务/铰链/回补，
                //    unified_osc::step_exit；与 subs 互斥由 custody 检查
                //    保证）。nest 触发与 confirmed 掩码同词汇地位 ⇒ nf
                //    双侧透传（nest 关时恒 false 零接触）──
                osc_layer.step_exit(
                    k, c, i as i64, sig, nf_sell[k], nf_buy[k], &phase_view, &book,
                    &mut layers, &mut pool, &mut res,
                );
            } else if let Some(sub) = subs[k] {
                if seq38_sub {
                    seq_low_since_open[k] = seq_low_since_open[k].min(c);
                }
                if tp {
                    // 049:52 满仓义务：趋势相开始 ⇒ 强制回补（对象域消失）。
                    // 载具级规则——seq38 子腿同受（38课程式运行在 fusion
                    // 基座的已结算义务之内，差分声明见 seq38_sub 字段文档）。
                    if try_restore(
                        k, c, i as i64, "sub_diff", &mut pool, &mut layers, &mut subs,
                        &mut res,
                    ) {
                        res.n_sub_phase_closes_by_ladder[k] += 1;
                        if seq38_sub {
                            seq_seg1_low[k] = None;
                            seq_run_low[k] = c;
                            seq_low_since_open[k] = f64::INFINITY;
                        }
                    }
                } else if sig.sell_any.get(k) || nf_sell[k] {
                    // 044:44 铰链恶化升级：本层卖点先到 ⇒ 短差卖出升级为
                    // 减仓（出清）。回补义务消灭 ⇒ 专款释放入池（耦合模式
                    // escrow=0，现金已在卖出 bar 入 pool）——只改记账身份。
                    // nest 正向触发同词汇地位（nest 关时 nf 恒 false）。
                    pool += sub.escrow;
                    res.trades.push(LayerTrade {
                        ladder: k as u8,
                        entry_bar,
                        entry_price,
                        exit_bar: sub.sell_bar,
                        exit_price: sub.sell_price,
                        shares: sub.shares,
                        weight_at_entry: weight,
                        deferred_bars,
                        partial,
                        exit_reason: "hinge_escalate",
                        polarity: Polarity::Long,
                    });
                    res.n_sub_escalates_by_ladder[k] += 1;
                    res.n_exits_by_ladder[k] += 1;
                    layers[k] = LayerState::Flat;
                    subs[k] = None;
                    if seq38_sub {
                        seq_seg1_low[k] = None;
                        seq_run_low[k] = c;
                        seq_low_since_open[k] = f64::INFINITY;
                    }
                } else if seq38_sub {
                    // ── Sequence38 闭腿三岔（38课:36 + 答疑:296；任一为真
                    //    即买回，归因取最强证据）──
                    // (1) 段间盘整背驰买点（Consolidation × Buy @ k）
                    let cons_buy = devrows[k]
                        .iter()
                        .any(|d| d.kind == DivKind::Consolidation && d.side() == Side::Buy);
                    // (2) 不跌破第一段低点 ∧ 次级别结构确认（答疑:296——
                    //     事件证据 ∨ D3 方向行证据，LOU sub_confirm 逐字）
                    let nobreak = seq_seg1_low[k]
                        .is_some_and(|s1| seq_low_since_open[k] > s1)
                        && k >= 1
                        && (sig.buy_any.get(k - 1)
                            || devrows[k - 1].iter().any(|d| d.side() == Side::Buy)
                            || dir_state[k - 1] == Some(Direction::Up));
                    // (3) 新的下跌背驰（观望出口）
                    let new_div = devrows[k]
                        .iter()
                        .any(|d| d.kind == DivKind::Trend && d.side() == Side::Buy)
                        || evrows[k]
                            .iter()
                            .any(|e| e.class == BspClass::Buy1 && e.confirmed);
                    if sub.restore_due || cons_buy || nobreak || new_div {
                        let ok = try_restore(
                            k, c, i as i64, "seq38_diff", &mut pool, &mut layers,
                            &mut subs, &mut res,
                        );
                        if ok {
                            if cons_buy {
                                res.n_seq38_consbuy_closes_by_ladder[k] += 1;
                            } else if nobreak {
                                res.n_seq38_nobreak_closes_by_ladder[k] += 1;
                            } else if new_div {
                                res.n_seq38_newdiv_closes_by_ladder[k] += 1;
                            } else {
                                res.n_sub_restores_by_ladder[k] += 1; // 义务遗留
                            }
                            // 中间循环复位：新一轮第一段低点从当下重新累计
                            seq_seg1_low[k] = None;
                            seq_run_low[k] = c;
                            seq_low_since_open[k] = f64::INFINITY;
                        }
                    }
                } else {
                    let sc = k >= 1 && sub_close_trigger(&evrows[k - 1], &devrows[k - 1]);
                    let kb = sig.buy_any.get(k);
                    if sub.restore_due || sc || kb {
                        let ok = try_restore(
                            k, c, i as i64, "sub_diff", &mut pool, &mut layers, &mut subs,
                            &mut res,
                        );
                        if ok {
                            if sc || sub.restore_due {
                                res.n_sub_restores_by_ladder[k] += 1;
                            } else {
                                res.n_sub_kbuy_restores_by_ladder[k] += 1;
                            }
                        }
                    }
                }
            } else if sig.sell_any.get(k) || nf_sell[k] {
                // 049:54 背驰出场：趋势相内 type1（趋势顶背驰词汇）⇒ 全抛
                // （"这个级别的走势类型完成"）；其余卖点停削（049:60"中途
                // 不参与短差"）。div_exit 关闭时趋势相一律停削（在册行为）。
                // nest_forward：正向触发与 confirmed 掩码同词汇地位（区间套
                // 正向定位 = 同一卖点的更早时间坐标，非新卖点类别）。
                let div_exit_hit = tp && opts.div_exit && sig.sell1.get(k);
                if tp && !div_exit_hit {
                    // 049:52 趋势相停削（"那种中枢完成后的向上移动时的差价
                    // 是不能做的，中枢向上移动时，就应该满仓"）。
                    res.n_trend_holds_by_ladder[k] += 1;
                    // anc 豁免拦截归因（P7）：自层判据为假 ⇒ 本次拦截纯由
                    // 祖先窗口触发（= 调研 §1.4 E0 桶的实装对应物）。
                    if trend_scope != TrendScope::SelfLayer && !self_up(k) {
                        res.n_anc_exempt_blocks_by_ladder[k] += 1;
                    }
                } else if r2_blocked(k) {
                    // P7 R2 位置门：震荡相非高位（c < ZG(k)）卖点不削减
                    // （049:52 位置分量——"在中枢上方仓位减少"）。
                    res.n_r2_pos_blocks_by_ladder[k] += 1;
                } else {
                    if div_exit_hit {
                        res.n_trend_div_exits_by_ladder[k] += 1;
                    } else if in_trend_raw(k) {
                        // tp 为假而 raw 为真 ⇒ 唯一否决者是 41课门（观测）。
                        res.n_gate41_blocks_by_ladder[k] += 1;
                    }
                    // hold26 在册削减（震荡相：上减）∨ 049:54 趋势顶背驰全抛。
                    pool += shares * c;
                    res.trades.push(LayerTrade {
                        ladder: k as u8,
                        entry_bar,
                        entry_price,
                        exit_bar: i as i64,
                        exit_price: c,
                        shares,
                        weight_at_entry: weight,
                        deferred_bars,
                        partial,
                        exit_reason: if div_exit_hit {
                            "trend_div"
                        } else if !sig.sell_any.get(k) {
                            // 纯正向触发（confirmed 掩码未置位）——区间套定位
                            // 归因。nest_forward=false 时 nf_sell 恒 false ⇒
                            // 本分支不可达，在册 "sellpt" 零漂移。
                            "nest_sell"
                        } else {
                            "sellpt"
                        },
                        polarity: Polarity::Long,
                    });
                    res.n_exits_by_ladder[k] += 1;
                    layers[k] = LayerState::Flat;
                    // ── 双向条件轴 [镜像推导]：白名单层翻转断面（会计文档
                    //    §5.2 四步序的 ②平多→③开空；①子腿 cascade settle
                    //    无对象——counter_sub 被守卫拒绝，subs[k] 恒 None）。
                    //    M = N 同股数定理（26:34 单位数量纲）：units = 平多
                    //    股数；1x 虚拟逐仓 margin = units×c = 平多所得 ⇒
                    //    翻转 bar pool 净流转 0（断面无渗漏）。同 bar 同价、
                    //    会计分两行 trade（段归属核算硬要求，不可合并）。──
                    if short_mask & (1 << k) != 0 {
                        if short_ghost {
                            // fusion_btrg 消融臂：持币 + 回复门驻留——把
                            // 空头暴露分量摘除，保留回复时点门分量。
                            layers[k] = LayerState::Gated {
                                entry_bar: i as i64,
                                weight,
                            };
                            res.n_gate_enters_by_ladder[k] += 1;
                        } else if short_anc_gate && !anc_down(k) {
                            // fusion_btra：镜像 anc 窗口外削减照常、不开空。
                            res.n_short_anc_rejects_by_ladder[k] += 1;
                        } else {
                            let margin = shares * c;
                            pool -= margin;
                            layers[k] = LayerState::Short {
                                entry_bar: i as i64,
                                entry_price: c,
                                units: shares,
                                weight,
                                margin,
                            };
                            res.n_flip_shorts_by_ladder[k] += 1;
                        }
                    }
                }
            }
        }

        // ── 阶段 B：层内短差开（先卖；53课次级别词汇 + 49课:64 全抛）。
        //    股数→现金等价转换 ⇒ NAV 不变，阶段 C 快照不受先后影响 ──
        if counter_sub {
            for k in (floor_ladder.max(FIRST_BSP_LADDER + 1))..MAX_LADDER {
                if subs[k].is_some() || in_trend(k) {
                    continue;
                }
                let LayerState::Long { shares, .. } = layers[k] else { continue };
                let sk = k - 1; // 笔=a0（77/78课）：循环下界已保证 sk ≥ FIRST_BSP_LADDER
                if !sub_open_trigger(&evrows[sk], &devrows[sk]) {
                    continue;
                }
                // 35课成本门：次级别典型中枢振幅 ≥ k×friction 才可操作；
                // 参照不可定义 ⇒ 保守拒绝（不静默放行）。
                match depth_ref.theta(sk, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
                    None => {
                        res.n_sub_noref_rejects_by_ladder[k] += 1;
                        continue;
                    }
                    Some(tq) if tq < SUB_COST_K * SUB_FRICTION_RT => {
                        res.n_sub_cost_rejects_by_ladder[k] += 1;
                        continue;
                    }
                    Some(_) => {}
                }
                // 卖出所得：耦合 ⇒ 入共享池（自由资金）；解耦 ⇒ 锁定为
                // 本层回补专款（earmark，不可被其它层新入场挪用）。
                let proceeds = shares * c;
                let escrow = if capital_decoupled { proceeds } else { 0.0 };
                if !capital_decoupled {
                    pool += proceeds;
                }
                subs[k] = Some(SubOut {
                    shares,
                    sell_bar: i as i64,
                    sell_price: c,
                    restore_due: false,
                    escrow,
                });
                res.n_sub_opens_by_ladder[k] += 1;
            }
        }

        // ── 阶段 B''：Sequence38 子腿开（先卖；38课:36 程式——本级别∨
        //    次级别盘整背驰卖点开腿，seg1_low 冻结 = run_low 快照。无
        //    成本门/无中枢前置 = LOU 逐字；in_trend 拒 = 基座 049:52
        //    满仓义务（载具级规则，counter_sub 同款）。custody：翻空
        //    白名单层 slice 由双向词汇独占；osc 在外层不可重入 ──
        if seq38_sub {
            for k in floor_ladder.max(FIRST_BSP_LADDER)..MAX_LADDER {
                if subs[k].is_some()
                    || osc_layer.outs[k].is_some()
                    || short_mask & (1 << k) != 0
                    || in_trend(k)
                {
                    continue;
                }
                let LayerState::Long { shares, .. } = layers[k] else { continue };
                let cons_sell = |ds: &[DivEvent]| {
                    ds.iter()
                        .any(|d| d.kind == DivKind::Consolidation && d.side() == Side::Sell)
                };
                if !(cons_sell(&devrows[k]) || (k >= 1 && cons_sell(&devrows[k - 1]))) {
                    continue;
                }
                pool += shares * c;
                subs[k] = Some(SubOut {
                    shares,
                    sell_bar: i as i64,
                    sell_price: c,
                    restore_due: false,
                    escrow: 0.0,
                });
                res.n_seq38_opens_by_ladder[k] += 1;
                seq_seg1_low[k] = Some(seq_run_low[k]);
                seq_low_since_open[k] = c;
            }
        }

        // ── 阶段 B'：统一配置 U 的 osc 开腿（相位递归路由——三门合取找
        //    第一个合法级别 j，触发判据 c≥ZG(j)∧sub_sell(j−1)；先卖后买，
        //    股数→现金等价转换 ⇒ NAV 不变，阶段 C 快照不受先后影响）。
        //    sell_any≠0 预滤：无任何次级别卖证据的 bar 任何层都不可能触发
        //    开腿（纯性能预滤，路由计数共享此条件——n_route_* 字段声明）。
        //    custody：翻空白名单层 + subs 在外层跳过（slice 单一在外原则——
        //    全量普适组合 2026-06-12；在册 fusion_u 两集恒空零接触）──
        if osc_on && sig.sell_any.0 != 0 {
            for k in floor_ladder..MAX_LADDER {
                if osc_layer.outs[k].is_some()
                    || subs[k].is_some()
                    || short_mask & (1 << k) != 0
                {
                    continue;
                }
                osc_layer.try_open(
                    k, c, i as i64, sig, osc_strong, osc_h1, &phase_view, &book,
                    &depth_ref, &layers, &mut pool, &mut res,
                );
            }
        }

        // ── 阶段 C：入场/回复（hold26 同构：confirmed 买点直接消费，无
        //    ARMED；ladder 降序——大级别优先拿配额）──
        let bar_nav = nav_fusion(&layers, &subs, &osc_layer.outs, pool, c, floor_ladder);
        let (thetas, theta_total) = theta_weights(&depth_ref, floor_ladder);
        for k in (floor_ladder..MAX_LADDER).rev() {
            match layers[k] {
                LayerState::Flat => {
                    // nest_forward：正向买触发与 confirmed 买掩码同词汇地位
                    // （回复侧镜像——次级别第一个买证据 = 买点精确坐标）。
                    if sig.buy_any.get(k) || nf_buy[k] {
                        layers[k] = enter_or_defer(
                            k, i as i64, i as i64, c, bar_nav, &thetas, theta_total,
                            &mut pool, &mut res,
                        );
                    }
                }
                LayerState::Pending { confirm_bar } => {
                    if sig.sell_any.get(k) || nf_sell[k] {
                        res.n_pending_cancels_by_ladder[k] += 1;
                        layers[k] = LayerState::Flat;
                    } else {
                        layers[k] = enter_or_defer(
                            k, confirm_bar, i as i64, c, bar_nav, &thetas, theta_total,
                            &mut pool, &mut res,
                        );
                    }
                }
                LayerState::Armed { .. } => {
                    unreachable!("Fusion 无 ARMED 相位——confirmed 事件直接消费")
                }
                LayerState::Long { .. } => {}
                // ── 双向条件轴 [镜像推导]：空头层出口集（short_mask 层
                //    专属——其余模式不可达）。优先序：逐仓强平（物理事件）
                //    → MoveUp 强制平空（49:52 满仓义务镜像：空头前提消失）
                //    → 买点回补（MoveDown 停回补 / 空侧 R2 位置门拦截）。
                //    平空 = 层权益→现金等价转换（NAV 不变 ⇒ bar_nav 快照
                //    严格）；翻多走在册 enter_or_defer 配额流程（断面对偶）──
                LayerState::Short { entry_bar, entry_price, units, weight, margin } => {
                    res.short_held_bars_by_ladder[k] += 1;
                    let equity_k = margin + units * (entry_price - c);
                    let mut cover = |exit_price: f64,
                                     exit_reason: &'static str,
                                     pool: &mut f64,
                                     res: &mut PositionalResult| {
                        // 1x 逐仓：损失上界 = margin（强平价记账 ⇒ 现金
                        // 流出恰为全部 margin，逐 trade 重建零渗漏）。
                        *pool += margin + units * (entry_price - exit_price);
                        res.short_net_cash_by_ladder[k] +=
                            units * (entry_price - exit_price);
                        res.trades.push(LayerTrade {
                            ladder: k as u8,
                            entry_bar,
                            entry_price,
                            exit_bar: i as i64,
                            exit_price,
                            shares: units,
                            weight_at_entry: weight,
                            deferred_bars: 0,
                            partial: false,
                            exit_reason,
                            polarity: Polarity::Short,
                        });
                        res.n_exits_by_ladder[k] += 1;
                    };
                    if equity_k <= 0.0 {
                        // 虚拟逐仓强平：1x 解析强平价 = 2×entry_price
                        // （margin = units×entry_price 的 equity=0 解）。
                        cover(2.0 * entry_price, "short_liquidated", &mut pool, &mut res);
                        res.n_short_liquidations_by_ladder[k] += 1;
                        layers[k] = LayerState::Flat;
                    } else if in_trend(k) {
                        cover(c, "cover_moveup", &mut pool, &mut res);
                        res.n_short_moveup_covers_by_ladder[k] += 1;
                        layers[k] = enter_or_defer(
                            k, i as i64, i as i64, c, bar_nav, &thetas, theta_total,
                            &mut pool, &mut res,
                        );
                    } else if sig.buy_any.get(k) || nf_buy[k] {
                        // nest_forward：正向买触发与 confirmed 买掩码同词汇
                        // 地位（区间套定位 = 同一买点的更早时间坐标）——
                        // 平空出口与 Flat 回复侧对称消费（fusion_btran_s*；
                        // nest_forward=false 时 nf_buy 恒 false ⇒ 在册
                        // "cover_buypt" 零漂移）。MoveDown 停回补 / 空侧
                        // R2 门对两种触发一视同仁（门在词汇之后）。
                        if in_movedown(k) {
                            // 49:52 镜像：中枢向下移动时应满空仓、停回补。
                            res.n_short_trend_holds_by_ladder[k] += 1;
                        } else if short_r2_blocked(k) {
                            res.n_short_r2_blocks_by_ladder[k] += 1;
                        } else {
                            let reason = if sig.buy_any.get(k) {
                                "cover_buypt"
                            } else {
                                // 纯正向触发（confirmed 掩码未置位）——归因。
                                "cover_nest"
                            };
                            cover(c, reason, &mut pool, &mut res);
                            res.n_short_covers_by_ladder[k] += 1;
                            layers[k] = enter_or_defer(
                                k, i as i64, i as i64, c, bar_nav, &thetas, theta_total,
                                &mut pool, &mut res,
                            );
                        }
                    }
                }
                // ── 纯回复门消融臂（fusion_btrg）：Gated 出口集与 Short
                //    逐句同款（in_trend 强制回复 / 买点 + MoveDown 停回复 +
                //    空侧 R2 门）——唯一差分 = 无空头暴露（持币零市场风险，
                //    无强平路径）。拦截计数复用 n_short_trend_holds/
                //    n_short_r2_blocks（GH2 可比性守卫的同名读数）──
                LayerState::Gated { weight: _, .. } => {
                    res.gate_held_bars_by_ladder[k] += 1;
                    if in_trend(k) {
                        res.n_gate_moveup_restores_by_ladder[k] += 1;
                        layers[k] = enter_or_defer(
                            k, i as i64, i as i64, c, bar_nav, &thetas, theta_total,
                            &mut pool, &mut res,
                        );
                    } else if sig.buy_any.get(k) {
                        if in_movedown(k) {
                            res.n_short_trend_holds_by_ladder[k] += 1;
                        } else if short_r2_blocked(k) {
                            res.n_short_r2_blocks_by_ladder[k] += 1;
                        } else {
                            res.n_gate_restores_by_ladder[k] += 1;
                            layers[k] = enter_or_defer(
                                k, i as i64, i as i64, c, bar_nav, &thetas, theta_total,
                                &mut pool, &mut res,
                            );
                        }
                    }
                }
            }
        }

        if i as i64 % EQUITY_SAMPLE_BARS == 0 || i == n - 1 {
            res.equity.push((i as i64, bar_nav));
        }
    }

    // ── eod：全层收口。短差在外的层其股数已变现（现金在 pool）——trade 行
    //    以实际变现点记账（身份悬置为 eod，铰链未及裁决）──
    let last_close = tape.bars[n - 1].close;
    for k in floor_ladder..MAX_LADDER {
        // 双向条件轴：空头层 eod 收口（按市价平空；末 bar 跳穿逐仓界则按
        // 强平价记账——与阶段 C 强平同一会计口径）。
        if let LayerState::Short { entry_bar, entry_price, units, weight, margin } =
            layers[k]
        {
            let equity_k = margin + units * (entry_price - last_close);
            let (exit_price, exit_reason) = if equity_k <= 0.0 {
                (2.0 * entry_price, "short_liquidated")
            } else {
                (last_close, "eod")
            };
            pool += margin + units * (entry_price - exit_price);
            res.short_net_cash_by_ladder[k] += units * (entry_price - exit_price);
            if exit_reason == "short_liquidated" {
                res.n_short_liquidations_by_ladder[k] += 1;
            }
            res.trades.push(LayerTrade {
                ladder: k as u8,
                entry_bar,
                entry_price,
                exit_bar: n as i64 - 1,
                exit_price,
                shares: units,
                weight_at_entry: weight,
                deferred_bars: 0,
                partial: false,
                exit_reason,
                polarity: Polarity::Short,
            });
            res.n_exits_by_ladder[k] += 1;
            layers[k] = LayerState::Flat;
            continue;
        }
        let LayerState::Long { entry_bar, entry_price, shares, weight, deferred_bars, partial } =
            layers[k]
        else {
            continue;
        };
        let (exit_bar, exit_price, sh) = match (subs[k], osc_layer.outs[k]) {
            (Some(sub), _) => {
                // 义务在 eod 悬置收口 ⇒ 专款释放（耦合模式 escrow=0）。
                pool += sub.escrow;
                (sub.sell_bar, sub.sell_price, sub.shares)
            }
            // U 的 osc 在外腿悬置收口（所得已在 pool，铰链未及裁决）。
            (None, Some(osc)) => (osc.sell_bar, osc.sell_price, osc.shares),
            (None, None) => {
                pool += shares * last_close;
                (n as i64 - 1, last_close, shares)
            }
        };
        res.trades.push(LayerTrade {
            ladder: k as u8,
            entry_bar,
            entry_price,
            exit_bar,
            exit_price,
            shares: sh,
            weight_at_entry: weight,
            deferred_bars,
            partial,
            exit_reason: "eod",
            polarity: Polarity::Long,
        });
        res.n_exits_by_ladder[k] += 1;
        layers[k] = LayerState::Flat;
        subs[k] = None;
        osc_layer.outs[k] = None;
    }
    res.final_nav = pool;
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::super::positional::{run_positional, PolarityMode};
    use super::*;
    use crate::trading::tape::BarSig;
    use crate::trading::types::LadderMask;

    fn bar(close: f64) -> BarSig {
        BarSig { close, max_ladder: 5, ..Default::default() }
    }

    fn anchor_ev(cs: i64, zd: f64, zg: f64) -> BspEvent {
        BspEvent {
            class: BspClass::Sell1,
            seg_idx: 0,
            confirmed: false,
            cs: Some(cs),
            zd: Some(zd),
            zg: Some(zg),
            price: 0.0,
        }
    }

    fn ev(class: BspClass) -> BspEvent {
        BspEvent {
            class,
            seg_idx: 0,
            confirmed: true,
            cs: None,
            zd: None,
            zg: None,
            price: 0.0,
        }
    }

    fn with_ev(mut b: BarSig, lad: usize, e: BspEvent) -> BarSig {
        let rows = b
            .bsp_events
            .get_or_insert_with(|| Box::new(<[Vec<BspEvent>; MAX_LADDER]>::default()));
        rows[lad].push(e);
        b
    }

    fn with_anchor(b: BarSig, lad: usize, cs: i64, zd: f64, zg: f64) -> BarSig {
        with_ev(b, lad, anchor_ev(cs, zd, zg))
    }

    fn div_ev(dir: Direction) -> DivEvent {
        DivEvent {
            kind: DivKind::Consolidation,
            direction: dir,
            seg_idx: 0,
            force_a: 0.0,
            force_c: 0.0,
            price: 0.0,
        }
    }

    fn with_div(mut b: BarSig, lad: usize, d: DivEvent) -> BarSig {
        let rows = b
            .div_events
            .get_or_insert_with(|| Box::new(<[Vec<DivEvent>; MAX_LADDER]>::default()));
        rows[lad].push(d);
        b
    }

    /// 空 div 行占位（counter_sub capability guard 的最小满足）。
    fn with_empty_div(mut b: BarSig) -> BarSig {
        b.div_events.get_or_insert_with(Box::default);
        b
    }

    fn buypt(mut b: BarSig, lad: usize) -> BarSig {
        b.buy_any = LadderMask(b.buy_any.0 | (1 << lad));
        b
    }

    fn sellpt(mut b: BarSig, lad: usize) -> BarSig {
        b.sell_any = LadderMask(b.sell_any.0 | (1 << lad));
        b
    }

    /// 头部参照：θ₃=1%、θ₄=3%（c=100 口径）⇒ w₄=0.75；θ₃ 过 35课成本门
    /// （1% ≥ 2×0.1%）。首 bar 带空 div 行满足 counter_sub guard。
    fn warmup34() -> Vec<BarSig> {
        let mut bars = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let b = with_anchor(bar(100.0), 3, 10 + j, 50.0, 51.0);
            let b = with_anchor(b, 4, 100 + j, 50.0, 53.0);
            bars.push(if j == 0 { with_empty_div(b) } else { b });
        }
        bars
    }

    fn run(bars: Vec<BarSig>, mode: &str) -> PositionalResult {
        run_with_rows(bars, mode, None, None)
    }

    fn run_with_rows(
        bars: Vec<BarSig>,
        mode: &str,
        dir_flips: Option<Vec<(i64, u8, Direction)>>,
        trend_flips: Option<Vec<(i64, u8, bool)>>,
    ) -> PositionalResult {
        let t = SignalTape { bars, dir_flips, trend_flips, ..Default::default() };
        run_positional(&t, 2, PolarityMode::parse(mode).unwrap()).unwrap()
    }

    #[test]
    fn cost_gate_constants_match_organic_defaults() {
        let d = crate::trading::config::OrganicConfig::default();
        assert_eq!(SUB_COST_K, d.sub_cost_k, "成本门倍数与 OrganicConfig 默认漂移");
        assert_eq!(SUB_FRICTION_RT, d.sub_friction_rt, "摩擦参数与 OrganicConfig 默认漂移");
    }

    /// 测试构造捷径：trend_opts 全默认、无 osc 层的 Fusion。
    fn fz(trend_hold: bool, counter_sub: bool, decoupled: bool) -> PolarityMode {
        PolarityMode::Fusion {
            trend_hold,
            counter_sub,
            decoupled,
            trend_opts: TrendAxisOpts::default(),
            osc: OscRouting::Off,
            phase_clock: false,
            r2_gate: false,
            trend_scope: TrendScope::SelfLayer,
            nest_forward: false,
            short_mask: 0,
            short_anc_gate: false,
            short_ghost: false,
            seq38_sub: false,
        }
    }

    /// 测试构造捷径：统一配置 U（fusion_u/fusion_uw）。
    fn fu(strong_gate: bool) -> PolarityMode {
        PolarityMode::Fusion {
            trend_hold: true,
            counter_sub: false,
            decoupled: false,
            trend_opts: TrendAxisOpts::default(),
            osc: OscRouting::Unified { strong_gate, h1_freeze: false },
            phase_clock: false,
            r2_gate: false,
            trend_scope: TrendScope::SelfLayer,
            nest_forward: false,
            short_mask: 0,
            short_anc_gate: false,
            short_ghost: false,
            seq38_sub: false,
        }
    }

    #[test]
    fn parse_and_guards() {
        assert_eq!(PolarityMode::parse("fusion"), Some(fz(true, true, false)));
        assert_eq!(PolarityMode::parse("fusion_t"), Some(fz(true, false, false)));
        assert_eq!(PolarityMode::parse("fusion_s"), Some(fz(false, true, false)));
        assert_eq!(PolarityMode::parse("fusion_e"), Some(fz(true, true, true)));
        assert_eq!(PolarityMode::parse("fusion_se"), Some(fz(false, true, true)));
        let bars = warmup34();
        // Fusion{false,false} = Hold26 冗余表示 ⇒ 拒绝
        let t = SignalTape { bars: warmup34(), ..Default::default() };
        assert!(run_positional(&t, 2, fz(false, false, false)).is_err());
        // decoupled 无 counter_sub = 专款无对象 ⇒ 拒绝
        let td = SignalTape { bars: warmup34(), ..Default::default() };
        assert!(run_positional(&td, 2, fz(true, false, true)).is_err());
        // trend_hold 无 trend/dir 行 ⇒ 拒绝
        let t2 = SignalTape { bars: warmup34(), ..Default::default() };
        assert!(run_positional(&t2, 2, fz(true, false, false)).is_err());
        // counter_sub 无 div 磁带 ⇒ 拒绝（warmup34 不带 div 行时）
        let no_div: Vec<BarSig> = bars
            .into_iter()
            .map(|mut b| {
                b.div_events = None;
                b
            })
            .collect();
        let t3 = SignalTape { bars: no_div, ..Default::default() };
        assert!(run_positional(&t3, 2, fz(false, true, false)).is_err());
    }

    #[test]
    fn parse_trend_axis_modes_and_guards() {
        let opt = |g: bool, d: bool, b: bool| PolarityMode::Fusion {
            trend_hold: true,
            counter_sub: false,
            decoupled: false,
            trend_opts: TrendAxisOpts { gate41: g, div_exit: d, b3_start: b },
            osc: OscRouting::Off,
            phase_clock: false,
            r2_gate: false,
            trend_scope: TrendScope::SelfLayer,
            nest_forward: false,
            short_mask: 0,
            short_anc_gate: false,
            short_ghost: false,
            seq38_sub: false,
        };
        assert_eq!(PolarityMode::parse("fusion_tg"), Some(opt(true, false, false)));
        assert_eq!(PolarityMode::parse("fusion_td"), Some(opt(false, true, false)));
        assert_eq!(PolarityMode::parse("fusion_tb"), Some(opt(false, false, true)));
        assert_eq!(PolarityMode::parse("fusion_tgb"), Some(opt(true, false, true)));
        assert_eq!(PolarityMode::parse("fusion_tgdb"), Some(opt(true, true, true)));
        // 乱序/重复/非法字符 ⇒ None
        assert_eq!(PolarityMode::parse("fusion_tdg"), None);
        assert_eq!(PolarityMode::parse("fusion_tgg"), None);
        assert_eq!(PolarityMode::parse("fusion_tx"), None);
        // opts 无 trend_hold = 修饰子无对象 ⇒ 拒绝
        let t = SignalTape { bars: warmup34(), ..Default::default() };
        assert!(run_positional(
            &t,
            2,
            PolarityMode::Fusion {
                trend_hold: false,
                counter_sub: true,
                decoupled: false,
                trend_opts: TrendAxisOpts { gate41: true, ..Default::default() },
                osc: OscRouting::Off,
                phase_clock: false,
                r2_gate: false,
                trend_scope: TrendScope::SelfLayer,
                nest_forward: false,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            }
        )
        .is_err());
        // gate41 无 div 磁带 ⇒ 拒绝（衰竭证据无数据基础）
        let no_div: Vec<BarSig> = warmup34()
            .into_iter()
            .map(|mut b| {
                b.div_events = None;
                b
            })
            .collect();
        let t2 = SignalTape {
            bars: no_div,
            dir_flips: Some(vec![]),
            trend_flips: Some(vec![]),
            ..Default::default()
        };
        assert!(run_positional(
            &t2,
            2,
            PolarityMode::Fusion {
                trend_hold: true,
                counter_sub: false,
                decoupled: false,
                trend_opts: TrendAxisOpts { gate41: true, ..Default::default() },
                osc: OscRouting::Off,
                phase_clock: false,
                r2_gate: false,
                trend_scope: TrendScope::SelfLayer,
                nest_forward: false,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            }
        )
        .is_err());
    }

    #[test]
    fn trend_phase_suppresses_trim_oscillation_resumes() {
        // θ₂ 参照 → 层2 全仓入场 → 趋势相卖点停削 → 震荡相卖点削减。
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0))
            .collect();
        bars.push(buypt(bar(100.0), 2)); // 入场 @100
        bars.push(sellpt(bar(110.0), 2)); // 趋势相（行在下方注入）→ 停削
        bars.push(sellpt(bar(108.0), 2)); // 趋势翻落 → 削减 @108
        bars.push(bar(108.0));
        let entry_bar = SUB_COST_MIN_OBS as i64;
        let r = run_with_rows(
            bars,
            "fusion_t",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![(0, 2, true), (entry_bar + 2, 2, false)]),
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 1, "趋势相卖点停削");
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2.len(), 1);
        assert_eq!(t2[0].exit_reason, "sellpt");
        assert_eq!(t2[0].exit_price, 108.0, "震荡相恢复削减");
        assert!((r.final_nav - 108_000.0).abs() < 1e-6);
    }

    #[test]
    fn bear_direction_keeps_hold26_trim() {
        // 趋势相判据要求 dir==Up：kind==Trend ∧ dir==Down（下行趋势）不停削
        // ——P2 熊市 α 承载面零接触。
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0))
            .collect();
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(90.0), 2)); // 下行趋势中卖点 → 正常削减
        bars.push(bar(80.0));
        let r = run_with_rows(
            bars,
            "fusion_t",
            Some(vec![(0, 2, Direction::Down)]),
            Some(vec![(0, 2, true)]),
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 0);
        assert_eq!(r.trades[0].exit_reason, "sellpt");
        assert_eq!(r.trades[0].exit_price, 90.0);
    }

    // ════════════════════════════════════════════════════════
    // 区间套正向定位（nest_forward；fusion_tn/fusion_trn）
    // ════════════════════════════════════════════════════════

    /// candidate 事件构造（价格可控——背驰段极值）。
    fn cand(class: BspClass, cs: i64, price: f64) -> BspEvent {
        BspEvent {
            class,
            seg_idx: 0,
            confirmed: false,
            cs: Some(cs),
            zd: Some(50.0),
            zg: Some(51.0),
            price,
        }
    }

    /// 层3 θ 参照 warmup + 首 bar 空 div 行（nest 守卫的磁带能力满足）。
    fn nest_warmup() -> Vec<BarSig> {
        (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let b = with_anchor(bar(100.0), 3, 10 + j, 50.0, 51.0);
                if j == 0 {
                    with_empty_div(b)
                } else {
                    b
                }
            })
            .collect()
    }

    #[test]
    fn nest_parse_and_guards() {
        let nm = |r2: bool| PolarityMode::Fusion {
            trend_hold: true,
            counter_sub: false,
            decoupled: false,
            trend_opts: TrendAxisOpts::default(),
            osc: OscRouting::Off,
            phase_clock: false,
            r2_gate: r2,
            trend_scope: TrendScope::SelfLayer,
            nest_forward: true,
            short_mask: 0,
            short_anc_gate: false,
            short_ghost: false,
            seq38_sub: false,
        };
        assert_eq!(PolarityMode::parse("fusion_tn"), Some(nm(false)));
        assert_eq!(PolarityMode::parse("fusion_trn"), Some(nm(true)));
        // 无 div 行 ⇒ 拒绝（次级别证据词汇残缺）
        let no_div: Vec<BarSig> = nest_warmup()
            .into_iter()
            .map(|mut b| {
                b.div_events = None;
                b
            })
            .collect();
        let t = SignalTape {
            bars: no_div,
            dir_flips: Some(vec![]),
            trend_flips: Some(vec![]),
            ..Default::default()
        };
        assert!(run_positional(&t, 2, nm(false)).is_err());
        // × counter_sub 未预注册 ⇒ 拒绝
        let t2 = SignalTape {
            bars: nest_warmup(),
            dir_flips: Some(vec![]),
            trend_flips: Some(vec![]),
            ..Default::default()
        };
        assert!(run_positional(
            &t2,
            2,
            PolarityMode::Fusion {
                trend_hold: true,
                counter_sub: true,
                decoupled: false,
                trend_opts: TrendAxisOpts::default(),
                osc: OscRouting::Off,
                phase_clock: false,
                r2_gate: false,
                trend_scope: TrendScope::SelfLayer,
                nest_forward: true,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            }
        )
        .is_err());
    }

    #[test]
    fn nest_fires_on_sub_evidence_before_confirmed() {
        // 层3 candidate Sell1 武装 → 次级别(2)第一个卖事件触发削减——
        // confirmed 永不到达（区间套正向定位 vs 等右侧确认的核心差分）。
        let mut bars = nest_warmup();
        bars.push(buypt(bar(100.0), 3)); // 入场 @100
        bars.push(with_ev(bar(100.0), 3, cand(BspClass::Sell1, 99, 200.0))); // 武装
        bars.push(with_ev(bar(98.0), 2, ev(BspClass::Sell1))); // 次级别证据 → 触发
        bars.push(bar(98.0));
        let r = run_with_rows(bars, "fusion_tn", Some(vec![]), Some(vec![]));
        assert_eq!(r.n_nest_fire_sell_by_ladder[3], 1, "次级别证据正向触发");
        let t3: Vec<_> =
            r.trades.iter().filter(|t| t.ladder == 3 && t.exit_reason != "eod").collect();
        assert_eq!(t3.len(), 1);
        assert_eq!(t3[0].exit_reason, "nest_sell", "归因为区间套触发");
        assert_eq!(t3[0].exit_price, 98.0, "在证据 bar 削减，不等 confirmed");
    }

    #[test]
    fn nest_break_negates_window() {
        // 027:25"只要没有打破背驰段"的逆否：价格越过 candidate 极值 ⇒ 作废，
        // 其后次级别证据不触发。
        let mut bars = nest_warmup();
        bars.push(buypt(bar(100.0), 3));
        bars.push(with_ev(bar(100.0), 3, cand(BspClass::Sell1, 99, 105.0))); // extreme=105
        bars.push(bar(110.0)); // 打破 → 作废
        bars.push(with_ev(bar(110.0), 2, ev(BspClass::Sell1))); // 证据迟到，窗口已亡
        bars.push(bar(110.0));
        let r = run_with_rows(bars, "fusion_tn", Some(vec![]), Some(vec![]));
        assert_eq!(r.n_nest_fire_sell_by_ladder[3], 0, "打破后不触发");
        assert!(r.n_nest_breaks_by_ladder[3] >= 1);
        assert_eq!(
            r.trades.iter().filter(|t| t.ladder == 3 && t.exit_reason != "eod").count(),
            0,
            "持仓不被已作废窗口削减"
        );
    }

    #[test]
    fn nest_buy_fires_entry() {
        // 回复侧镜像：candidate Buy1 武装 → 次级别买证据 ⇒ 回复入场
        // （不等 confirmed 买掩码）。
        let mut bars = nest_warmup();
        bars.push(buypt(bar(100.0), 3)); // 入场
        bars.push(sellpt(bar(100.0), 3)); // confirmed 卖掩码削减 → Flat
        bars.push(with_ev(bar(95.0), 3, cand(BspClass::Buy1, 99, 90.0))); // 买窗 extreme=90
        bars.push(with_ev(bar(95.0), 2, ev(BspClass::Buy1))); // 次级别买证据 → 回复
        bars.push(bar(95.0));
        let r = run_with_rows(bars, "fusion_tn", Some(vec![]), Some(vec![]));
        assert_eq!(r.n_nest_fire_buy_by_ladder[3], 1);
        assert_eq!(r.n_entries_by_ladder[3], 2, "削减后由正向定位回复");
    }

    #[test]
    fn nest_lead_pairing_on_late_confirmed() {
        // 正向触发领先量测量：confirmed 同 (class,cs) 事后到达 ⇒ 配对回填。
        let mut bars = nest_warmup();
        bars.push(buypt(bar(100.0), 3));
        bars.push(with_ev(bar(100.0), 3, cand(BspClass::Sell1, 99, 200.0)));
        bars.push(with_ev(bar(98.0), 2, ev(BspClass::Sell1))); // fire bar = b
        bars.push(bar(98.0));
        bars.push(with_ev(
            bar(97.0),
            3,
            BspEvent { confirmed: true, ..cand(BspClass::Sell1, 99, 97.0) },
        )); // confirmed @ b+2
        bars.push(bar(97.0));
        let r = run_with_rows(bars, "fusion_tn", Some(vec![]), Some(vec![]));
        assert_eq!(r.nest_lead_n, 1);
        assert_eq!(r.nest_lead_bars_sum, 2, "正向触发领先 confirmed 2 bar");
    }

    #[test]
    fn nest_off_zero_intrusion() {
        // fusion_t 在同一磁带上 nest 计数恒零、行为与在册一致（候选事件
        // 存在但 nest_forward=false ⇒ 全部路径不可达）。
        let mut bars = nest_warmup();
        bars.push(buypt(bar(100.0), 3));
        bars.push(with_ev(bar(100.0), 3, cand(BspClass::Sell1, 99, 200.0)));
        bars.push(with_ev(bar(98.0), 2, ev(BspClass::Sell1)));
        bars.push(bar(98.0));
        let r = run_with_rows(bars, "fusion_t", Some(vec![]), Some(vec![]));
        assert_eq!(r.n_nest_fire_sell_by_ladder[3], 0);
        assert_eq!(r.n_nest_arms_by_ladder[3], 0);
        assert_eq!(
            r.trades.iter().filter(|t| t.ladder == 3 && t.exit_reason != "eod").count(),
            0,
            "无 confirmed 掩码 ⇒ 在册行为不削减"
        );
    }

    /// anc 测试构造捷径：hold26_anc / fusion_ta（其余轴全关）+ 单轴变体。
    fn anc_mode_with(
        scope: TrendScope,
        trend_hold: bool,
        counter_sub: bool,
        r2_gate: bool,
    ) -> PolarityMode {
        PolarityMode::Fusion {
            trend_hold,
            counter_sub,
            decoupled: false,
            trend_opts: TrendAxisOpts::default(),
            osc: OscRouting::Off,
            phase_clock: false,
            r2_gate,
            trend_scope: scope,
            nest_forward: false,
            short_mask: 0,
            short_anc_gate: false,
            short_ghost: false,
            seq38_sub: false,
        }
    }

    fn anc_mode(scope: TrendScope) -> PolarityMode {
        anc_mode_with(scope, true, false, false)
    }

    #[test]
    fn parse_anc_modes_and_guards() {
        assert_eq!(
            PolarityMode::parse("hold26_anc"),
            Some(anc_mode(TrendScope::Ancestor))
        );
        assert_eq!(
            PolarityMode::parse("fusion_ta"),
            Some(anc_mode(TrendScope::SelfOrAncestor))
        );
        // 在册模式 trend_scope 恒 SelfLayer（零接触守卫）
        assert_eq!(
            PolarityMode::parse("fusion_t"),
            Some(anc_mode_with(TrendScope::SelfLayer, true, false, false))
        );
        // anc × 未预注册轴 ⇒ 拒绝
        let reject = |m: PolarityMode| {
            let t = SignalTape {
                bars: warmup34(),
                dir_flips: Some(vec![]),
                trend_flips: Some(vec![]),
                ..Default::default()
            };
            assert!(run_positional(&t, 2, m).is_err());
        };
        // 无 trend_hold = 时钟无对象
        reject(anc_mode_with(TrendScope::Ancestor, false, true, false));
        // × counter_sub 未预注册
        reject(anc_mode_with(TrendScope::Ancestor, true, true, false));
        // × r2_gate 未预注册
        reject(anc_mode_with(TrendScope::SelfOrAncestor, true, false, true));
        // anc 要求 trend/dir 行（trend_hold 既有守卫覆盖）
        let t = SignalTape { bars: warmup34(), ..Default::default() };
        assert!(run_positional(&t, 2, anc_mode(TrendScope::Ancestor)).is_err());
    }

    #[test]
    fn anc_ancestor_trend_suppresses_self_layer_trim() {
        // 层2 自层震荡（无 trend 行置位），祖先层3 kind==Trend ∧ dir==Up
        // ⇒ hold26_anc 豁免层2 卖点削减（26:80 下沉）；fusion_t 不豁免。
        let mk_bars = || -> Vec<BarSig> {
            let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
                .map(|j| with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0))
                .collect();
            bars.push(buypt(bar(100.0), 2)); // 入场 @100
            bars.push(sellpt(bar(110.0), 2)); // 祖先趋势中 → 豁免
            bars.push(bar(120.0));
            bars
        };
        let rows_dir = Some(vec![(0i64, 3u8, Direction::Up)]);
        let rows_trend = Some(vec![(0i64, 3u8, true)]);
        let r =
            run_with_rows(mk_bars(), "hold26_anc", rows_dir.clone(), rows_trend.clone());
        assert_eq!(r.n_trend_holds_by_ladder[2], 1, "祖先趋势 ⇒ 层2 停削");
        assert_eq!(
            r.n_anc_exempt_blocks_by_ladder[2], 1,
            "自层判据为假 ⇒ 纯 anc 豁免拦截（P7 归因）"
        );
        assert!(r.anc_up_bars_by_ladder[2] > 0, "豁免窗口驻留可观测");
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2.len(), 1);
        assert_eq!(t2[0].exit_reason, "eod", "豁免后持有到尾（无削减 trade）");
        // 对照：fusion_t 自层时钟看不到祖先 ⇒ 正常削减
        let rf = run_with_rows(mk_bars(), "fusion_t", rows_dir, rows_trend);
        assert_eq!(rf.n_trend_holds_by_ladder[2], 0);
        assert_eq!(rf.n_anc_exempt_blocks_by_ladder[2], 0, "在册模式 anc 计数恒零");
        assert_eq!(rf.trades[0].exit_reason, "sellpt");
    }

    #[test]
    fn anc_monotonicity_break_resumes_trim() {
        // 祖先单调性打破（层3 dir 翻 Down）⇒ 豁免窗口关，削减恢复
        // （35:16"第N个中枢不再高于第N-1个才可说上涨结束"的引擎读出）。
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0))
            .collect();
        bars.push(buypt(bar(100.0), 2)); // 入场 @100
        bars.push(sellpt(bar(110.0), 2)); // 窗口内 → 豁免
        bars.push(sellpt(bar(108.0), 2)); // 窗口已关 → 削减 @108
        bars.push(bar(108.0));
        let entry_bar = SUB_COST_MIN_OBS as i64;
        let r = run_with_rows(
            bars,
            "hold26_anc",
            Some(vec![(0, 3, Direction::Up), (entry_bar + 2, 3, Direction::Down)]),
            Some(vec![(0, 3, true)]),
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 1);
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2.len(), 1);
        assert_eq!(t2[0].exit_reason, "sellpt");
        assert_eq!(t2[0].exit_price, 108.0, "单调性打破 ⇒ 恢复削减");
        assert!((r.final_nav - 108_000.0).abs() < 1e-6);
    }

    #[test]
    fn anc_bear_ancestor_keeps_trim_and_ta_unions_self() {
        // 熊市零接触（P4 承载面）：祖先 kind==Trend ∧ dir==Down 不豁免。
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0))
            .collect();
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(90.0), 2));
        bars.push(bar(80.0));
        let r = run_with_rows(
            bars,
            "hold26_anc",
            Some(vec![(0, 3, Direction::Down)]),
            Some(vec![(0, 3, true)]),
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 0, "下行祖先趋势不豁免");
        assert_eq!(r.trades[0].exit_reason, "sellpt");
        // fusion_ta 并集：自层趋势（祖先无）⇒ 仍停削（49:52 字面保留）；
        // 且自层为真时 anc 归因计数不增（P7 口径：纯祖先拦截）。
        let mut bars2: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0))
            .collect();
        bars2.push(buypt(bar(100.0), 2));
        bars2.push(sellpt(bar(110.0), 2));
        bars2.push(bar(120.0));
        let r2 = run_with_rows(
            bars2,
            "fusion_ta",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![(0, 2, true)]),
        );
        assert_eq!(r2.n_trend_holds_by_ladder[2], 1, "自层窗口在并集中保留");
        assert_eq!(r2.n_anc_exempt_blocks_by_ladder[2], 0, "自层真 ⇒ 非 anc 归因");
    }

    #[test]
    fn sub_cycle_sell_high_buy_back_same_shares() {
        // 层4 @100 入场（w=0.75，750股）→ Sell1@3 @110 全抛 → Buy1@3 @105
        // 如数接回（净 +3750）→ eod @105。NAV = 100000 + 750×(110−105) +
        // 750×(105−100) = 107500。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars.push(with_ev(bar(105.0), 3, ev(BspClass::Buy1)));
        bars.push(bar(105.0));
        let r = run(bars, "fusion_s");
        assert_eq!(r.n_sub_opens_by_ladder[4], 1);
        assert_eq!(r.n_sub_restores_by_ladder[4], 1);
        assert!((r.sub_net_cash_by_ladder[4] - 3750.0).abs() < 1e-6);
        let t4: Vec<_> = r.trades.iter().filter(|t| t.ladder == 4).collect();
        assert_eq!(t4.len(), 2);
        assert_eq!(t4[0].exit_reason, "sub_diff");
        assert_eq!(t4[0].exit_price, 110.0);
        assert_eq!(t4[1].exit_reason, "eod");
        assert_eq!(t4[1].entry_price, 105.0, "接回点重锚");
        assert!((t4[0].shares - t4[1].shares).abs() < 1e-12, "如数接回股数守恒");
        assert!((r.final_nav - 107_500.0).abs() < 1e-6);
    }

    #[test]
    fn sub_opens_on_consolidation_div_and_closes_on_buy3() {
        // 盘背卖（div Consolidation×Up @3）开 → confirmed Buy3@3 回补。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_div(bar(110.0), 3, div_ev(Direction::Up)));
        bars.push(with_ev(bar(104.0), 3, ev(BspClass::Buy3)));
        bars.push(bar(104.0));
        let r = run(bars, "fusion_s");
        assert_eq!(r.n_sub_opens_by_ladder[4], 1, "盘背卖词汇分量");
        assert_eq!(r.n_sub_restores_by_ladder[4], 1, "Buy3 回补词汇分量");
        assert!((r.sub_net_cash_by_ladder[4] - 750.0 * 6.0).abs() < 1e-6);
    }

    #[test]
    fn hinge_escalates_to_layer_exit_on_k_level_sellpt() {
        // 044:44 铰链：短差在外 + 本层卖点 ⇒ 升级为减仓。变现价 = 短差卖出
        // 价 110（非升级 bar 的 120）——身份事后授予，记账以实际现金流。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars.push(sellpt(bar(120.0), 4));
        bars.push(bar(130.0));
        let r = run(bars, "fusion_s");
        assert_eq!(r.n_sub_escalates_by_ladder[4], 1);
        assert_eq!(r.n_sub_restores_by_ladder[4], 0);
        let t4: Vec<_> = r.trades.iter().filter(|t| t.ladder == 4).collect();
        assert_eq!(t4.len(), 1);
        assert_eq!(t4[0].exit_reason, "hinge_escalate");
        assert_eq!(t4[0].exit_price, 110.0);
        // 750×(110−100) = +7500
        assert!((r.final_nav - 107_500.0).abs() < 1e-6);
    }

    #[test]
    fn trend_phase_forces_buy_back() {
        // 短差在外 + 趋势相开始 ⇒ 强制回补（049:52 满仓义务）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars.push(bar(106.0)); // 趋势相起点（行注入）→ 强制回补 @106
        bars.push(bar(115.0));
        let entry_bar = SUB_COST_MIN_OBS as i64;
        let r = run_with_rows(
            bars,
            "fusion",
            Some(vec![(entry_bar + 2, 4, Direction::Up)]),
            Some(vec![(entry_bar + 2, 4, true)]),
        );
        assert_eq!(r.n_sub_phase_closes_by_ladder[4], 1);
        let t4: Vec<_> = r.trades.iter().filter(|t| t.ladder == 4).collect();
        assert_eq!(t4[0].exit_reason, "sub_diff");
        assert_eq!(t4[1].entry_price, 106.0);
        assert_eq!(t4[1].exit_reason, "eod");
        // +750×(110−106) 短差 + 750×(115−100−4) 持有
        assert!((r.sub_net_cash_by_ladder[4] - 3000.0).abs() < 1e-6);
    }

    #[test]
    fn k_level_buypt_closes_sub() {
        // 26课"任何的买点都是买点"：本层 k 级买点也回补在外短差。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars.push(buypt(bar(103.0), 4));
        bars.push(bar(103.0));
        let r = run(bars, "fusion_s");
        assert_eq!(r.n_sub_kbuy_restores_by_ladder[4], 1);
        assert_eq!(r.n_sub_restores_by_ladder[4], 0);
    }

    #[test]
    fn trend_phase_blocks_sub_open() {
        // 趋势相中 k−1 卖证据不开短差（049:52 差价不能做）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars.push(bar(112.0));
        let r = run_with_rows(
            bars,
            "fusion",
            Some(vec![(0, 4, Direction::Up)]),
            Some(vec![(0, 4, true)]),
        );
        assert_eq!(r.n_sub_opens_by_ladder[4], 0);
    }

    #[test]
    fn cost_gate_noref_and_thin_amp_reject() {
        // 组1：θ₃ 无参照（仅 θ₄ warm-up）→ noref 保守拒。
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let b = with_anchor(bar(100.0), 4, 100 + j, 50.0, 53.0);
                if j == 0 { with_empty_div(b) } else { b }
            })
            .collect();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars.push(bar(110.0));
        let r = run(bars, "fusion_s");
        assert_eq!(r.n_sub_noref_rejects_by_ladder[4], 1);
        assert_eq!(r.n_sub_opens_by_ladder[4], 0);

        // 组2：θ₃ = 0.01% < 2×0.1% → 成本拒。
        let mut bars2: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let b = with_anchor(bar(100.0), 3, 10 + j, 50.0, 50.01);
                let b = with_anchor(b, 4, 100 + j, 50.0, 53.0);
                if j == 0 { with_empty_div(b) } else { b }
            })
            .collect();
        bars2.push(buypt(bar(100.0), 4));
        bars2.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars2.push(bar(110.0));
        let r2 = run(bars2, "fusion_s");
        assert_eq!(r2.n_sub_cost_rejects_by_ladder[4], 1);
        assert_eq!(r2.n_sub_opens_by_ladder[4], 0);
    }

    #[test]
    fn bi_floor_blocks_segment_layer_sub() {
        // 层2 的 k−1 = bi（a0，77/78课）⇒ 结构上不开短差。
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let b = with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0);
                if j == 0 { with_empty_div(b) } else { b }
            })
            .collect();
        bars.push(buypt(bar(100.0), 2));
        bars.push(with_ev(bar(110.0), 1, ev(BspClass::Sell1))); // 笔级"事件"（合成）
        bars.push(bar(110.0));
        let r = run(bars, "fusion_s");
        assert_eq!(r.n_sub_opens_by_ladder[2], 0);
        assert_eq!(r.n_sub_noref_rejects_by_ladder[2], 0, "未进入成本门——存在论拒绝在先");
    }

    #[test]
    fn restore_deferred_when_pool_drained_then_retries() {
        // 层4 全仓（仅 θ₄）→ 短差在外 → 层2 入场抽走部分 pool → 回补触发时
        // 价格上行 pool 不足 ⇒ 推迟；价格回落 ⇒ 补完（义务恒在）。
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let b = with_anchor(bar(100.0), 3, 10 + j, 50.0, 51.0);
                let b = with_anchor(b, 4, 100 + j, 50.0, 53.0);
                if j == 0 { with_empty_div(b) } else { b }
            })
            .collect();
        bars.push(buypt(bar(100.0), 4)); // w₄=0.75 → 750 股，pool=25000
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1))); // pool=107500
        bars.push(buypt(bar(100.0), 2)); // 层2 Flat→入场？θ₂ 无参照 → noref skip
        // 用层3 抽 pool：θ₃=1% 有参照，w₃=0.25 → 抽 ~26875
        bars.push(buypt(bar(100.0), 3));
        bars.push(with_ev(bar(120.0), 3, ev(BspClass::Buy1))); // cost=90000 > pool ⇒ 推迟
        bars.push(bar(95.0)); // cost=71250 ≤ pool ⇒ 补完
        bars.push(bar(95.0));
        let r = run(bars, "fusion_s");
        assert!(r.n_sub_restore_defer_bars >= 1, "资金不足推迟计数");
        assert_eq!(r.n_sub_restores_by_ladder[4], 1, "义务最终补完");
    }

    /// 防挪用 warmup：θ₃=1%、θ₄=3% 双参照（解耦/耦合对照测试共用）。
    fn warmup_dual() -> Vec<BarSig> {
        (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let b = with_anchor(bar(100.0), 3, 10 + j, 50.0, 51.0);
                let b = with_anchor(b, 4, 100 + j, 50.0, 53.0);
                if j == 0 { with_empty_div(b) } else { b }
            })
            .collect()
    }

    /// 防挪用核心对照：sub 在外期间其它层入场吃池 → 回补触发。
    /// 耦合臂：卖出所得已被 L3 入场挪用 ⇒ 推迟（在册负交互的机械形态）；
    /// 解耦臂：专款不可挪用 ⇒ 立即回补、零推迟。同一磁带，唯一变量=资金语义。
    #[test]
    fn decoupled_escrow_prevents_misappropriation() {
        let mk = || {
            let mut bars = warmup_dual();
            bars.push(buypt(bar(100.0), 4)); // L4 750股，pool=25000
            bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1))); // sub 开（所得 82500）
            bars.push(buypt(bar(100.0), 3)); // L3 入场吃池
            bars.push(with_ev(bar(108.0), 3, ev(BspClass::Buy1))); // 回补触发 @108
            bars.push(bar(108.0)); // 重试 bar
            bars.push(bar(95.0)); // 耦合臂在此才补完
            bars.push(bar(95.0));
            bars
        };
        let coupled = run(mk(), "fusion_s");
        let dec = run(mk(), "fusion_se");
        // 耦合：cost 81000 > pool 80625（L3 拿走 26875）⇒ 推迟两 bar，@95 补完
        assert!(coupled.n_sub_restore_defer_bars >= 2, "耦合臂应推迟（在册形态）");
        assert_eq!(coupled.n_sub_restores_by_ladder[4], 1);
        // 解耦：escrow 82500 ≥ cost 81000 ⇒ @108 立即回补，零推迟零补差
        assert_eq!(dec.n_sub_restore_defer_bars, 0, "专款兜底 ⇒ 义务零推迟");
        assert_eq!(dec.n_sub_restores_by_ladder[4], 1);
        assert_eq!(dec.n_sub_pool_topup_by_ladder[4], 0);
        let t4: Vec<_> = dec.trades.iter().filter(|t| t.ladder == 4).collect();
        assert_eq!(t4[1].entry_price, 108.0, "解耦臂在触发 bar 接回（重锚 @108）");
        // 短差净现金：解耦 +1500（110→108）> 耦合 +11250（110→95）？否——
        // 耦合臂推迟反而以更低价接回。解耦消除的是义务风险（追价敞口），
        // 不保证单笔更优——此处只验证机制，优劣由回测裁决。
        assert!((dec.sub_net_cash_by_ladder[4] - 1500.0).abs() < 1e-6);
    }

    /// 追价补差：回补价 > 卖出价 ⇒ 专款不足，池补差 + topup 观测。
    #[test]
    fn decoupled_topup_on_price_chase() {
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 750股，pool=25000
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1))); // escrow=82500
        bars.push(with_ev(bar(112.0), 3, ev(BspClass::Buy1))); // cost=84000 追价
        bars.push(bar(112.0));
        let r = run(bars, "fusion_se");
        assert_eq!(r.n_sub_restores_by_ladder[4], 1);
        assert_eq!(r.n_sub_pool_topup_by_ladder[4], 1, "deficit 1500 由池补差");
        assert!((r.sub_net_cash_by_ladder[4] + 1500.0).abs() < 1e-6);
        // NAV = 100000 − 750×(112−110) + 750×(112−100) 持仓增值
        assert!((r.final_nav - 107_500.0).abs() < 1e-6);
    }

    /// 池不约束时解耦与耦合 NAV 恒等（escrow 只改资金归属不改现金流）。
    #[test]
    fn decoupled_equals_coupled_when_pool_unbinding() {
        let mk = || {
            let mut bars = warmup34();
            bars.push(buypt(bar(100.0), 4));
            bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
            bars.push(with_ev(bar(105.0), 3, ev(BspClass::Buy1)));
            bars.push(bar(105.0));
            bars
        };
        let coupled = run(mk(), "fusion_s");
        let dec = run(mk(), "fusion_se");
        assert!((coupled.final_nav - dec.final_nav).abs() < 1e-9);
        assert!((dec.final_nav - 107_500.0).abs() < 1e-6);
        assert_eq!(dec.n_sub_pool_topup_by_ladder[4], 0, "105 < 110 ⇒ 专款盈余入池");
    }

    /// 义务消灭点释放专款：铰链升级（本层卖点）⇒ escrow 入池，NAV 同耦合臂。
    #[test]
    fn decoupled_escrow_released_on_hinge_escalate() {
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars.push(sellpt(bar(120.0), 4));
        bars.push(bar(130.0));
        let r = run(bars, "fusion_se");
        assert_eq!(r.n_sub_escalates_by_ladder[4], 1);
        assert!((r.final_nav - 107_500.0).abs() < 1e-6, "变现价恒为短差卖出价 110");
    }

    /// eod 悬置收口释放专款 + 价格不变往返 NAV 守恒（解耦版会计自检）。
    #[test]
    fn decoupled_nav_conservation_and_eod_release() {
        // 组1：sub 在外至 eod ⇒ 专款释放，final_nav 含 escrow。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(110.0), 3, ev(BspClass::Sell1)));
        bars.push(bar(115.0)); // 无回补词汇 ⇒ 悬置
        let r = run(bars, "fusion_se");
        // pool 25000 + escrow 82500 = 107500（750×10 已变现）
        assert!((r.final_nav - 107_500.0).abs() < 1e-6);
        // 组2：价格不变完整往返 ⇒ NAV 守恒 + trade 行重建一致。
        let mut bars2 = warmup34();
        bars2.push(buypt(bar(100.0), 4));
        bars2.push(with_ev(bar(100.0), 3, ev(BspClass::Sell1)));
        bars2.push(with_ev(bar(100.0), 3, ev(BspClass::Buy1)));
        bars2.push(sellpt(bar(100.0), 4));
        bars2.push(bar(100.0));
        let r2 = run(bars2, "fusion_se");
        assert!((r2.final_nav - INITIAL_CAPITAL).abs() < 1e-9);
        let mut pool = INITIAL_CAPITAL;
        for t in &r2.trades {
            pool += t.shares * (t.exit_price - t.entry_price);
        }
        assert!((pool - r2.final_nav).abs() < 1e-9);
    }

    #[test]
    fn nav_reconstruction_invariant_round_trip() {
        // 价格不变的完整短差往返 ⇒ NAV 守恒（会计自检）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(100.0), 3, ev(BspClass::Sell1)));
        bars.push(with_ev(bar(100.0), 3, ev(BspClass::Buy1)));
        bars.push(sellpt(bar(100.0), 4));
        bars.push(bar(100.0));
        let r = run(bars, "fusion_s");
        assert!((r.final_nav - INITIAL_CAPITAL).abs() < 1e-9);
        // trade 行现金流重建 = final_nav（脚本 assert 的 Rust 侧前哨）
        let mut pool = INITIAL_CAPITAL;
        for t in &r.trades {
            pool += t.shares * (t.exit_price - t.entry_price);
        }
        assert!((pool - r.final_nav).abs() < 1e-9);
    }

    // ───────────────── T 轴严格化（TrendAxisOpts）─────────────────

    /// θ₂ 参照 warm-up（首 bar 带空 div 行——gate41/b3 测试的磁带能力前提）。
    fn warmup2() -> Vec<BarSig> {
        (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let b = with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0);
                if j == 0 { with_empty_div(b) } else { b }
            })
            .collect()
    }

    /// type1 卖点：sell1 + sell_any 双 mask（真实磁带中 type1 必同置 any 行）。
    fn s1pt(mut b: BarSig, lad: usize) -> BarSig {
        b.sell1 = LadderMask(b.sell1.0 | (1 << lad));
        sellpt(b, lad)
    }

    /// 带中枢锚的 confirmed 事件（b3 窗口测试：Buy3 开窗 / 大 cs 事件关窗）。
    fn ev_cs(class: BspClass, cs: i64) -> BspEvent {
        BspEvent {
            class,
            seg_idx: 0,
            confirmed: true,
            cs: Some(cs),
            zd: Some(50.0),
            zg: Some(51.0),
            price: 0.0,
        }
    }

    #[test]
    fn gate41_blocks_bear_rally_hold_until_parent_exhausts() {
        // 41课:22：层2 趋势相∧Up，父层3 dir==Down 未衰竭 ⇒ 停削被否决
        // （卖点照常削减）；父层向下背驰出现后 ⇒ 停削恢复。
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2)); // 入场 @100
        bars.push(sellpt(bar(110.0), 2)); // bar w+1：门否决 → 削减 @110
        bars.push(buypt(bar(100.0), 2)); // bar w+2：回复 @100
        bars.push(with_div(bar(95.0), 3, div_ev(Direction::Down))); // 父层衰竭
        bars.push(sellpt(bar(120.0), 2)); // bar w+4：停削恢复
        bars.push(bar(120.0));
        let r = run_with_rows(
            bars,
            "fusion_tg",
            Some(vec![(0, 2, Direction::Up), (0, 3, Direction::Down)]),
            Some(vec![(0, 2, true)]),
        );
        assert_eq!(r.n_gate41_blocks_by_ladder[2], 1, "未衰竭期卖点被门放行削减");
        assert_eq!(r.n_trend_holds_by_ladder[2], 1, "衰竭后停削恢复");
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "sellpt");
        assert_eq!(t2[0].exit_price, 110.0);
        assert_eq!(t2[1].exit_reason, "eod", "第二周期持有穿越（@120 收口）");
        // 100000×1.10 ×（120/100）= 132000
        assert!((r.final_nav - 132_000.0).abs() < 1e-6, "nav={}", r.final_nav);
    }

    #[test]
    fn gate41_passes_when_parent_dir_up_or_undefined() {
        // 父层无方向数据 / 父层向上 ⇒ 门放行（41课约束只在"方向相反"时定义）。
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(110.0), 2)); // 父层 dir 未定义 → 停削成立
        bars.push(bar(110.0));
        let r = run_with_rows(
            bars,
            "fusion_tg",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![(0, 2, true)]),
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 1);
        assert_eq!(r.n_gate41_blocks_by_ladder[2], 0);
    }

    #[test]
    fn div_exit_full_exit_on_type1_in_trend() {
        // 049:54：趋势相内非 t1 卖点停削；t1（趋势顶背驰词汇）⇒ 全抛
        // （exit_reason="trend_div"），"这个级别的走势类型完成"。
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(110.0), 2)); // 非 t1 → 停削
        bars.push(s1pt(bar(120.0), 2)); // t1 → 全抛 @120
        bars.push(bar(130.0));
        let r = run_with_rows(
            bars,
            "fusion_td",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![(0, 2, true)]),
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 1);
        assert_eq!(r.n_trend_div_exits_by_ladder[2], 1);
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2.len(), 1);
        assert_eq!(t2[0].exit_reason, "trend_div");
        assert_eq!(t2[0].exit_price, 120.0);
        assert!((r.final_nav - 120_000.0).abs() < 1e-6);
    }

    #[test]
    fn b3_window_holds_first_departure_until_new_center() {
        // 049:60：confirmed Buy3 开窗（kind 仍盘整 ⇒ trend_state=false 的
        // 首次离开段也停削）；新中枢事件（cs > 锚）⇒ 关窗回到震荡相削减。
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(with_ev(bar(102.0), 2, ev_cs(BspClass::Buy3, 1000)));
        bars.push(sellpt(bar(110.0), 2)); // 窗口内 → 停削
        bars.push(with_anchor(bar(112.0), 2, 1001, 108.0, 112.0)); // 新中枢
        bars.push(sellpt(bar(111.0), 2)); // 关窗 → 削减 @111
        bars.push(bar(111.0));
        let r = run_with_rows(
            bars,
            "fusion_tb",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![]), // trend_state 恒 false——窗口独立于 kind 直读
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 1, "三买窗口停削");
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "sellpt");
        assert_eq!(t2[0].exit_price, 111.0, "新中枢后恢复削减");
        assert!((r.final_nav - 111_000.0).abs() < 1e-6);
    }

    #[test]
    fn b3_window_closes_on_up_divergence() {
        // 049:42/46：三买后向上走势出现背驰/盘背 ⇒ 离开段衰竭，关窗。
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(with_ev(bar(102.0), 2, ev_cs(BspClass::Buy3, 1000)));
        bars.push(with_div(bar(115.0), 2, div_ev(Direction::Up))); // 盘背顶
        bars.push(sellpt(bar(113.0), 2)); // 关窗 → 削减
        bars.push(bar(113.0));
        let r = run_with_rows(
            bars,
            "fusion_tb",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![]),
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 0);
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "sellpt");
        assert_eq!(t2[0].exit_price, 113.0);
    }

    #[test]
    fn b3_window_closes_on_dir_flip_down() {
        // dir 翻 Down = 向上离开失败的结构证据 ⇒ 关窗。
        let w = SUB_COST_MIN_OBS as i64;
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(with_ev(bar(102.0), 2, ev_cs(BspClass::Buy3, 1000)));
        bars.push(bar(101.0)); // bar w+2：dir 翻 Down（行注入）
        bars.push(sellpt(bar(99.0), 2)); // 关窗 → 削减
        bars.push(bar(99.0));
        let r = run_with_rows(
            bars,
            "fusion_tb",
            Some(vec![(0, 2, Direction::Up), (w + 2, 2, Direction::Down)]),
            Some(vec![]),
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 0);
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "sellpt");
        assert_eq!(t2[0].exit_price, 99.0);
    }

    // ───────────────── 统一配置 U（fusion_u：相位递归路由）─────────────────

    #[test]
    fn parse_fusion_u_modes_and_guards() {
        assert_eq!(PolarityMode::parse("fusion_u"), Some(fu(true)));
        assert_eq!(PolarityMode::parse("fusion_uw"), Some(fu(false)));
        // !trend_hold × osc 层（counter_sub=true 绕过 Hold26 冗余守卫）⇒ 拒
        let t = SignalTape { bars: warmup34(), ..Default::default() };
        assert!(run_positional(
            &t,
            2,
            PolarityMode::Fusion {
                trend_hold: false,
                counter_sub: true,
                decoupled: false,
                trend_opts: TrendAxisOpts::default(),
                osc: OscRouting::Unified { strong_gate: true, h1_freeze: false },
                phase_clock: false,
                r2_gate: false,
                trend_scope: TrendScope::SelfLayer,
                nest_forward: false,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            }
        )
        .is_err());
        // counter_sub × osc 层（同层 slice 双在外）⇒ 拒
        let t2 = SignalTape {
            bars: warmup34(),
            dir_flips: Some(vec![]),
            trend_flips: Some(vec![]),
            ..Default::default()
        };
        assert!(run_positional(
            &t2,
            2,
            PolarityMode::Fusion {
                trend_hold: true,
                counter_sub: true,
                decoupled: false,
                trend_opts: TrendAxisOpts::default(),
                osc: OscRouting::Unified { strong_gate: true, h1_freeze: false },
                phase_clock: false,
                r2_gate: false,
                trend_scope: TrendScope::SelfLayer,
                nest_forward: false,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            }
        )
        .is_err());
        // trend_opts × osc 层（未预注册组合）⇒ 拒
        let t3 = SignalTape {
            bars: warmup34(),
            dir_flips: Some(vec![]),
            trend_flips: Some(vec![]),
            ..Default::default()
        };
        assert!(run_positional(
            &t3,
            2,
            PolarityMode::Fusion {
                trend_hold: true,
                counter_sub: false,
                decoupled: false,
                trend_opts: TrendAxisOpts { gate41: true, ..Default::default() },
                osc: OscRouting::Unified { strong_gate: true, h1_freeze: false },
                phase_clock: false,
                r2_gate: false,
                trend_scope: TrendScope::SelfLayer,
                nest_forward: false,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            }
        )
        .is_err());
    }

    /// ③参照不可定义（该层从未 dir==Up）⇒ 全塔拒绝 ⇒ 逐位退化为 fusion_t
    /// （P2"最坏应退化为不开"的构造性验证）。
    #[test]
    fn fusion_u_degenerates_to_fusion_t_when_routing_rejects() {
        let mk = || {
            let mut bars = warmup2();
            bars.push(buypt(bar(100.0), 2));
            bars.push(sellpt(bar(100.0), 1)); // 次级别卖证据（触发面非空）
            bars.push(bar(110.0));
            bars
        };
        let u = run_with_rows(mk(), "fusion_u", Some(vec![]), Some(vec![]));
        let ft = run_with_rows(mk(), "fusion_t", Some(vec![]), Some(vec![]));
        assert_eq!(u.n_osc_opens_by_ladder[2], 0);
        assert!(u.n_route_weak_noref[2] >= 1, "③参照不可定义保守拒");
        assert!(u.n_route_exhausted >= 1, "全塔拒绝可观测");
        assert!((u.final_nav - ft.final_nav).abs() < 1e-9, "退化面 NAV 恒等");
    }

    /// 本层震荡相三门全过 ⇒ 在宿主层开腿；ZD 触线兑现（L0 定理）。
    #[test]
    fn fusion_u_opens_home_level_and_zd_restores() {
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2)); // 1000 股 @100（w₂=1.0）
        bars.push(sellpt(bar(100.0), 1)); // 开腿：c=100 ≥ ZG=51 ∧ sub_sell
        bars.push(bar(50.0)); // ZD=50 触线 → 回补
        bars.push(bar(60.0));
        let r = run_with_rows(
            bars,
            "fusion_u",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![]),
        );
        assert_eq!(r.n_osc_opens_by_ladder[2], 1);
        assert_eq!(r.n_osc_open_at_level[2], 1, "宿主层路由（j==k）");
        assert_eq!(r.n_osc_upshift_opens_by_ladder[2], 0);
        assert_eq!(r.n_osc_zd_restores_by_ladder[2], 1);
        assert!((r.osc_net_cash_by_ladder[2] - 50_000.0).abs() < 1e-6);
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "osc_diff");
        assert_eq!(t2[0].exit_price, 100.0);
        assert_eq!(t2[1].entry_price, 50.0, "接回点重锚");
        // 100000 + 1000×(100−50) 短差 + 1000×(60−100) 持有 = 110000
        assert!((r.final_nav - 110_000.0).abs() < 1e-6, "nav={}", r.final_nav);
    }

    /// 宿主层趋势相 ⇒ ①门上移，j=k+1 三门过 ⇒ 上移开腿（H3 重路由的
    /// 递归形式；同 bar 本层卖点停削与上移开腿并行）。
    #[test]
    fn fusion_u_routes_upward_when_home_in_trend() {
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let b = with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0);
                with_anchor(b, 3, 100 + j, 50.0, 53.0)
            })
            .collect();
        bars.push(buypt(bar(100.0), 2)); // 250 股 @100（w₂=0.25）
        bars.push(sellpt(bar(100.0), 2)); // k=2 卖点：停削 + j=3 的 sub 证据
        bars.push(bar(50.0)); // ZD(3)=50 触线 → 回补
        bars.push(bar(50.0));
        let r = run_with_rows(
            bars,
            "fusion_u",
            Some(vec![(0, 2, Direction::Up), (0, 3, Direction::Up)]),
            Some(vec![(0, 2, true)]), // 层2 趋势相
        );
        assert_eq!(r.n_trend_holds_by_ladder[2], 1, "基座停削零接触");
        assert!(r.n_route_phase_skips[2] >= 1, "①门上移可观测");
        assert_eq!(r.n_osc_opens_by_ladder[2], 1);
        assert_eq!(r.n_osc_upshift_opens_by_ladder[2], 1);
        assert_eq!(r.n_osc_open_at_level[3], 1, "路由层 j=3");
        assert!(r.n_osc_up_out_bars_by_ladder[2] >= 1, "上移腿槽占用可观测");
        assert!((r.osc_net_cash_at_level[3] - 12_500.0).abs() < 1e-6);
        // 250×(100−50) 短差对冲了 250×(50−100) 持有损失 ⇒ NAV 守恒
        assert!((r.final_nav - 100_000.0).abs() < 1e-6, "nav={}", r.final_nav);
    }

    /// 全塔趋势 ⇒ 不开 osc，恒仓吃趋势（053:34）——与 fusion_t 逐位恒等。
    #[test]
    fn fusion_u_full_tower_trend_holds_like_fusion_t() {
        let mk = || {
            let mut bars = warmup2();
            bars.push(buypt(bar(100.0), 2));
            bars.push(sellpt(bar(110.0), 1));
            bars.push(bar(120.0));
            bars
        };
        let dirs: Vec<(i64, u8, Direction)> =
            (2..MAX_LADDER as u8).map(|l| (0, l, Direction::Up)).collect();
        let trends: Vec<(i64, u8, bool)> =
            (2..MAX_LADDER as u8).map(|l| (0, l, true)).collect();
        let u = run_with_rows(mk(), "fusion_u", Some(dirs.clone()), Some(trends.clone()));
        let ft = run_with_rows(mk(), "fusion_t", Some(dirs), Some(trends));
        assert_eq!(u.n_osc_opens_by_ladder[2], 0);
        assert!(u.n_route_exhausted >= 1);
        assert!((u.final_nav - ft.final_nav).abs() < 1e-9);
        assert!((u.final_nav - 120_000.0).abs() < 1e-6, "恒仓吃趋势");
    }

    /// ③强震荡门：当前中枢整体跌落前上涨最后中枢区间之下 = 弱震荡拒开
    /// （093:26）；fusion_uw（U−③ 消融臂）同磁带放行——P5 的机制隔离。
    #[test]
    fn fusion_u_weak_oscillation_rejected_uw_admits() {
        let w = SUB_COST_MIN_OBS as i64;
        let mk = || {
            let mut bars = warmup2(); // ref 随 dir==Up 冻结于 (50,51)
            bars.push(with_anchor(bar(100.0), 2, 2000, 30.0, 40.0)); // 跌落中枢
            bars.push(buypt(bar(100.0), 2));
            bars.push(sellpt(bar(100.0), 1));
            bars.push(bar(100.0));
            bars
        };
        let rows = || {
            (
                Some(vec![(0, 2, Direction::Up), (w, 2, Direction::Down)]),
                Some(vec![]),
            )
        };
        let (d, t) = rows();
        let u = run_with_rows(mk(), "fusion_u", d, t);
        assert_eq!(u.n_osc_opens_by_ladder[2], 0);
        assert!(u.n_route_weak_rejects[2] >= 1, "弱震荡拒可观测");
        let (d2, t2) = rows();
        let uw = run_with_rows(mk(), "fusion_uw", d2, t2);
        assert_eq!(uw.n_osc_opens_by_ladder[2], 1, "③关闭后同磁带放行");
    }

    /// 三卖否决（049:52"一旦出现第三类卖点，就不能回补了"）：锚中枢被
    /// confirmed Sell3 终结后 k 级买点不再回补，仅 ZD 触线（更低处）兑现。
    #[test]
    fn fusion_u_sell3_veto_blocks_kbuy_until_zd() {
        let w = SUB_COST_MIN_OBS as i64;
        let anchor_cs = 10 + w - 1; // warmup2 最后一个中枢
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(100.0), 1)); // 开腿
        bars.push(with_ev(bar(80.0), 2, ev_cs(BspClass::Sell3, anchor_cs)));
        bars.push(buypt(bar(70.0), 2)); // k 买点——否决期不回补
        bars.push(bar(50.0)); // ZD 触线 → 兑现
        bars.push(bar(50.0));
        let r = run_with_rows(
            bars,
            "fusion_u",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![]),
        );
        assert_eq!(r.n_osc_sell3_vetos_by_ladder[2], 1);
        assert_eq!(r.n_osc_kbuy_restores_by_ladder[2], 0, "否决期 k 买点不回补");
        assert_eq!(r.n_osc_death_restores_by_ladder[2], 0, "三卖死亡不触发死亡回补");
        assert_eq!(r.n_osc_zd_restores_by_ladder[2], 1, "更低处兑现");
        assert!((r.osc_net_cash_by_ladder[2] - 50_000.0).abs() < 1e-6);
    }

    /// sc 中枢上移出口（049:54）：新中枢 ZD > 锚 ZG ⇒ 立即回补。
    #[test]
    fn fusion_u_shift_close_on_center_upmove() {
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(100.0), 1)); // 开腿（锚 zg=51）
        bars.push(with_anchor(bar(100.0), 2, 3000, 60.0, 70.0)); // 新 ZD=60 > 51
        bars.push(bar(100.0));
        let r = run_with_rows(
            bars,
            "fusion_u",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![]),
        );
        assert_eq!(r.n_osc_shift_restores_by_ladder[2], 1);
        assert!((r.final_nav - 100_000.0).abs() < 1e-6, "同价往返 NAV 守恒");
    }

    /// 趋势相满仓义务（049:52）：路由层翻入趋势相 ⇒ 强制回补。
    #[test]
    fn fusion_u_phase_restore_on_trend_start() {
        let w = SUB_COST_MIN_OBS as i64;
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(100.0), 1)); // 开腿（j=2）
        bars.push(bar(95.0)); // bar w+2：层2 趋势相起点（行注入）→ 强制回补
        bars.push(bar(110.0));
        let r = run_with_rows(
            bars,
            "fusion_u",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![(w + 2, 2, true)]),
        );
        assert_eq!(r.n_osc_phase_restores_by_ladder[2], 1);
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "osc_diff");
        assert_eq!(t2[1].entry_price, 95.0, "满仓义务在触发 bar 接回");
        // 1000×(100−95) 短差 + 1000×(110−100) 持有 = +15000
        assert!((r.final_nav - 115_000.0).abs() < 1e-6, "nav={}", r.final_nav);
    }

    /// 44课铰链：osc 在外 + 本层卖点 ⇒ 升级为减仓出清（变现价 = 短差卖出价，
    /// 身份事后授予——hinge_escalate 同构）。
    #[test]
    fn fusion_u_hinge_escalates_on_k_sellpt() {
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(110.0), 1)); // 开腿 @110
        bars.push(sellpt(bar(120.0), 2)); // 本层卖点 → 升级出清
        bars.push(bar(130.0));
        let r = run_with_rows(
            bars,
            "fusion_u",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![]),
        );
        assert_eq!(r.n_osc_escalates_by_ladder[2], 1);
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2.len(), 1);
        assert_eq!(t2[0].exit_reason, "osc_escalate");
        assert_eq!(t2[0].exit_price, 110.0);
        assert!((r.final_nav - 110_000.0).abs() < 1e-6);
    }

    /// 铰链可达性 = ¬in_trend(k)（counter_sub 在册同构）：上移腿在 k 趋势
    /// 相内不被 k 卖点升级出清——049:52 停削语义对在外腿同样成立（升级
    /// 旁路被堵，事件计入 n_osc_trend_hold_sells）。
    #[test]
    fn fusion_u_hinge_blocked_in_k_trend_phase() {
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| {
                let b = with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0);
                with_anchor(b, 3, 100 + j, 50.0, 53.0)
            })
            .collect();
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(100.0), 2)); // 停削 + 上移开腿（j=3）
        bars.push(sellpt(bar(110.0), 2)); // k 趋势相内再现 k 卖点 → 抑制
        bars.push(bar(110.0));
        let r = run_with_rows(
            bars,
            "fusion_u",
            Some(vec![(0, 2, Direction::Up), (0, 3, Direction::Up)]),
            Some(vec![(0, 2, true)]),
        );
        assert_eq!(r.n_osc_upshift_opens_by_ladder[2], 1);
        assert_eq!(r.n_osc_escalates_by_ladder[2], 0, "趋势相内升级旁路被堵");
        assert!(r.n_osc_trend_hold_sells_by_ladder[2] >= 1, "抑制事件可观测");
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2.len(), 1);
        assert_eq!(t2[0].exit_reason, "eod", "腿悬置至 eod（以卖出点记账）");
        assert_eq!(t2[0].exit_price, 100.0);
    }

    /// ②振幅门：θ_q < k×friction ⇒ 拒开（H4 逐字；035:30）。
    #[test]
    fn fusion_u_amp_gate_rejects_thin_level() {
        // θ₂ = 0.01% < 2×0.1% ⇒ ②拒；全塔无其它候选 ⇒ 不开。
        let mut bars: Vec<BarSig> = (0..SUB_COST_MIN_OBS as i64)
            .map(|j| with_anchor(bar(100.0), 2, 10 + j, 50.0, 50.01))
            .collect();
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(100.0), 1));
        bars.push(bar(100.0));
        let r = run_with_rows(
            bars,
            "fusion_u",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![]),
        );
        assert_eq!(r.n_osc_opens_by_ladder[2], 0);
        assert!(r.n_route_amp_rejects[2] >= 1, "②拒可观测");
    }

    // ───────────────── P6 相位机（fusion_p/fusion_pu）─────────────────

    /// 带中枢锚的 candidate 事件（P6：candidate 离开窗口 = 049:68 当下读法）。
    fn cand_cs(class: BspClass, cs: i64) -> BspEvent {
        BspEvent {
            class,
            seg_idx: 0,
            confirmed: false,
            cs: Some(cs),
            zd: Some(50.0),
            zg: Some(51.0),
            price: 0.0,
        }
    }

    /// P6 测试捷径：phase_clock 臂（trend_flips 不需要——kind 行零消费）。
    fn run_phase(bars: Vec<BarSig>, mode: &str) -> PositionalResult {
        run_with_rows(bars, mode, Some(vec![(0, 2, Direction::Up)]), None)
    }

    #[test]
    fn parse_fusion_p_modes_and_guards() {
        let fp = |osc: OscRouting| PolarityMode::Fusion {
            trend_hold: true,
            counter_sub: false,
            decoupled: false,
            trend_opts: TrendAxisOpts::default(),
            osc,
            phase_clock: true,
            r2_gate: false,
            trend_scope: TrendScope::SelfLayer,
            nest_forward: false,
            short_mask: 0,
            short_anc_gate: false,
            short_ghost: false,
            seq38_sub: false,
        };
        assert_eq!(PolarityMode::parse("fusion_p"), Some(fp(OscRouting::Off)));
        assert_eq!(
            PolarityMode::parse("fusion_pu"),
            Some(fp(OscRouting::Unified { strong_gate: true, h1_freeze: false }))
        );
        // phase_clock × counter_sub 未预注册 ⇒ 拒
        let t = SignalTape {
            bars: warmup34(),
            dir_flips: Some(vec![]),
            ..Default::default()
        };
        assert!(run_positional(
            &t,
            2,
            PolarityMode::Fusion {
                trend_hold: true,
                counter_sub: true,
                decoupled: false,
                trend_opts: TrendAxisOpts::default(),
                osc: OscRouting::Off,
                phase_clock: true,
                r2_gate: false,
                trend_scope: TrendScope::SelfLayer,
                nest_forward: false,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            }
        )
        .is_err());
        // phase_clock × trend_opts（b3 已被收编）⇒ 拒
        let t2 = SignalTape {
            bars: warmup34(),
            dir_flips: Some(vec![]),
            ..Default::default()
        };
        assert!(run_positional(
            &t2,
            2,
            PolarityMode::Fusion {
                trend_hold: true,
                counter_sub: false,
                decoupled: false,
                trend_opts: TrendAxisOpts { b3_start: true, ..Default::default() },
                osc: OscRouting::Off,
                phase_clock: true,
                r2_gate: false,
                trend_scope: TrendScope::SelfLayer,
                nest_forward: false,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            }
        )
        .is_err());
        // 缺 div 磁带（背驰出口无数据基础）⇒ 拒
        let no_div: Vec<BarSig> = warmup34()
            .into_iter()
            .map(|mut b| {
                b.div_events = None;
                b
            })
            .collect();
        let t3 = SignalTape {
            bars: no_div,
            dir_flips: Some(vec![]),
            ..Default::default()
        };
        assert!(run_positional(&t3, 2, PolarityMode::parse("fusion_p").unwrap()).is_err());
        // 缺 dir 行（方向兜底出口无数据基础）⇒ 拒
        let t4 = SignalTape { bars: warmup34(), ..Default::default() };
        assert!(run_positional(&t4, 2, PolarityMode::parse("fusion_p").unwrap()).is_err());
        // kind 行不要求：trend_flips=None 合法运行（错位时钟退役的能力面）
        let t5 = SignalTape {
            bars: warmup34(),
            dir_flips: Some(vec![]),
            ..Default::default()
        };
        assert!(run_positional(&t5, 2, PolarityMode::parse("fusion_p").unwrap()).is_ok());
    }

    /// candidate 离开窗口停削（049:68 当下读法）；价格回中枢否定 = "校正"
    /// ⇒ 削减恢复（38课答疑"能回到中枢就不是第三类买点"）。
    #[test]
    fn fusion_p_candidate_departure_stops_trim_until_negated() {
        let w = SUB_COST_MIN_OBS as i64;
        let anchor_cs = 10 + w - 1;
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2)); // 入场 @100（1000 股）
        bars.push(with_ev(bar(100.0), 2, cand_cs(BspClass::Buy3, anchor_cs))); // 离开窗口
        bars.push(sellpt(bar(110.0), 2)); // 窗口内 → 停削
        bars.push(bar(45.0)); // c < ZG=51 ⇒ 价格否定关窗
        bars.push(sellpt(bar(108.0), 2)); // 削减恢复 @108
        bars.push(bar(108.0));
        let r = run_phase(bars, "fusion_p");
        assert_eq!(r.n_trend_holds_by_ladder[2], 1, "candidate 窗口停削");
        assert!(r.phase_up_bars_by_ladder[2] >= 2, "MOVE↑ 有效驻留可观测");
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "sellpt");
        assert_eq!(t2[0].exit_price, 108.0, "否定后削减恢复");
        assert!((r.final_nav - 108_000.0).abs() < 1e-6, "nav={}", r.final_nav);
    }

    /// confirmed Buy3 锁定 MOVE↑（价格否定不再适用），直到新中枢结算
    /// （049:60"在中枢第三类买点后持股直到新中枢出现"逐字）。
    #[test]
    fn fusion_p_confirmed_buy3_locks_until_new_center_settles() {
        let w = SUB_COST_MIN_OBS as i64;
        let anchor_cs = 10 + w - 1;
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(with_ev(bar(102.0), 2, ev_cs(BspClass::Buy3, anchor_cs))); // 锁定
        bars.push(bar(45.0)); // 价格回落不解锁（锁定区间无价格否定词汇）
        bars.push(sellpt(bar(110.0), 2)); // 仍 MOVE↑ → 停削
        bars.push(with_anchor(bar(112.0), 2, anchor_cs + 1000, 108.0, 112.0)); // C′ 结算
        bars.push(sellpt(bar(111.0), 2)); // OSC 恢复 → 削减 @111
        bars.push(bar(111.0));
        let r = run_phase(bars, "fusion_p");
        assert_eq!(r.n_phase_up_opens_by_ladder[2], 1, "锁定转移可观测");
        assert_eq!(r.n_phase_up_settle_closes_by_ladder[2], 1, "settle(C′) 关");
        assert_eq!(r.n_trend_holds_by_ladder[2], 1);
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "sellpt");
        assert_eq!(t2[0].exit_price, 111.0);
        assert!((r.final_nav - 111_000.0).abs() < 1e-6, "nav={}", r.final_nav);
    }

    /// MOVE↑ 锁定区间的背驰出口（049:54——相位翻 OSC，卖点词汇承载出场）。
    #[test]
    fn fusion_p_locked_moveup_closes_on_up_divergence() {
        let w = SUB_COST_MIN_OBS as i64;
        let anchor_cs = 10 + w - 1;
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(with_ev(bar(102.0), 2, ev_cs(BspClass::Buy3, anchor_cs)));
        bars.push(with_div(bar(115.0), 2, div_ev(Direction::Up))); // 移动背驰
        bars.push(sellpt(bar(113.0), 2)); // OSC → 削减 @113
        bars.push(bar(113.0));
        let r = run_phase(bars, "fusion_p");
        assert_eq!(r.n_phase_up_div_closes_by_ladder[2], 1, "背驰关可观测");
        assert_eq!(r.n_trend_holds_by_ladder[2], 0);
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_price, 113.0);
    }

    /// MOVE↓（confirmed Sell3）：削减照常（hold26 逐字——熊市 α 承载面
    /// 零接触；"不回补"强读法不在 P6，研究文档 §2 在册声明）。
    #[test]
    fn fusion_p_movedown_keeps_trim() {
        let w = SUB_COST_MIN_OBS as i64;
        let anchor_cs = 10 + w - 1;
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(with_ev(bar(95.0), 2, ev_cs(BspClass::Sell3, anchor_cs))); // MOVE↓
        bars.push(sellpt(bar(90.0), 2)); // 削减照常 @90
        bars.push(bar(90.0));
        let r = run_phase(bars, "fusion_p");
        assert_eq!(r.n_phase_dn_opens_by_ladder[2], 1);
        assert!(r.phase_dn_bars_by_ladder[2] >= 1);
        assert_eq!(r.n_trend_holds_by_ladder[2], 0, "MOVE↓ 不停削");
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "sellpt");
        assert_eq!(t2[0].exit_price, 90.0);
    }

    /// 双侧同步核心：candidate 离开窗口内 osc ①门拒开（第一段 MOVE↑ 被
    /// 逐出 osc 窗口——U 否证根因的修复面）；窗口否定后同词汇放行。
    #[test]
    fn fusion_pu_osc_blocked_in_candidate_window_admits_after_negation() {
        let w = SUB_COST_MIN_OBS as i64;
        let anchor_cs = 10 + w - 1;
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2)); // 1000 股 @100
        bars.push(with_ev(bar(100.0), 2, cand_cs(BspClass::Buy3, anchor_cs)));
        bars.push(sellpt(bar(100.0), 1)); // 窗口内：①拒 → 全塔无候选 → 不开
        bars.push(bar(45.0)); // 价格否定关窗
        bars.push(sellpt(bar(100.0), 1)); // OSC：c≥ZG ∧ sub_sell → 开腿
        bars.push(bar(50.0)); // ZD 触线 → 回补
        bars.push(bar(60.0));
        let r = run_phase(bars, "fusion_pu");
        assert!(r.n_route_phase_skips[2] >= 1, "①门拒可观测（MOVE↑ 窗口）");
        assert_eq!(r.n_osc_opens_by_ladder[2], 1, "仅窗口外的尝试开腿");
        assert_eq!(r.n_osc_zd_restores_by_ladder[2], 1);
        assert!((r.osc_net_cash_by_ladder[2] - 50_000.0).abs() < 1e-6);
        assert!((r.final_nav - 110_000.0).abs() < 1e-6, "nav={}", r.final_nav);
    }

    /// 双侧同步核心：在外腿遇 candidate 离开（移动启动）⇒ 满仓义务立即
    /// 回补——"osc 腿在移动段卖切片失血"的修复面（kind 行做不到：kind
    /// 在第一段移动尚未翻 Trend）。
    #[test]
    fn fusion_pu_phase_restore_on_candidate_departure() {
        let w = SUB_COST_MIN_OBS as i64;
        let anchor_cs = 10 + w - 1;
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(100.0), 1)); // OSC 相开腿 @100
        bars.push(with_ev(bar(95.0), 2, cand_cs(BspClass::Buy3, anchor_cs))); // 移动启动
        bars.push(bar(110.0));
        let r = run_phase(bars, "fusion_pu");
        assert_eq!(r.n_osc_opens_by_ladder[2], 1);
        assert_eq!(r.n_osc_phase_restores_by_ladder[2], 1, "满仓义务在移动启动 bar 接回");
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "osc_diff");
        assert_eq!(t2[1].entry_price, 95.0);
        // 1000×(100−95) 短差 + 1000×(110−100) 持有 = +15000
        assert!((r.final_nav - 115_000.0).abs() < 1e-6, "nav={}", r.final_nav);
    }

    // ───────────────── P7 R2 位置门（fusion_tr/fusion_pr/fusion_pur）─────────────────

    #[test]
    fn parse_r2_modes_and_guards() {
        let mk = |phase: bool, osc: OscRouting| PolarityMode::Fusion {
            trend_hold: true,
            counter_sub: false,
            decoupled: false,
            trend_opts: TrendAxisOpts::default(),
            osc,
            phase_clock: phase,
            r2_gate: true,
            trend_scope: TrendScope::SelfLayer,
            nest_forward: false,
            short_mask: 0,
            short_anc_gate: false,
            short_ghost: false,
            seq38_sub: false,
        };
        assert_eq!(PolarityMode::parse("fusion_tr"), Some(mk(false, OscRouting::Off)));
        assert_eq!(PolarityMode::parse("fusion_pr"), Some(mk(true, OscRouting::Off)));
        assert_eq!(
            PolarityMode::parse("fusion_pur"),
            Some(mk(true, OscRouting::Unified { strong_gate: true, h1_freeze: false }))
        );
        // r2 × counter_sub 未预注册 ⇒ 拒
        let t = SignalTape {
            bars: warmup34(),
            dir_flips: Some(vec![]),
            trend_flips: Some(vec![]),
            ..Default::default()
        };
        assert!(run_positional(
            &t,
            2,
            PolarityMode::Fusion {
                trend_hold: true,
                counter_sub: true,
                decoupled: false,
                trend_opts: TrendAxisOpts::default(),
                osc: OscRouting::Off,
                phase_clock: false,
                r2_gate: true,
                trend_scope: TrendScope::SelfLayer,
                nest_forward: false,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            }
        )
        .is_err());
        // r2 × trend_opts 未预注册 ⇒ 拒
        let t2 = SignalTape {
            bars: warmup34(),
            dir_flips: Some(vec![]),
            trend_flips: Some(vec![]),
            ..Default::default()
        };
        assert!(run_positional(
            &t2,
            2,
            PolarityMode::Fusion {
                trend_hold: true,
                counter_sub: false,
                decoupled: false,
                trend_opts: TrendAxisOpts { gate41: true, ..Default::default() },
                osc: OscRouting::Off,
                phase_clock: false,
                r2_gate: true,
                trend_scope: TrendScope::SelfLayer,
                nest_forward: false,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            }
        )
        .is_err());
    }

    /// R2 位置门（kind 时钟）：震荡相非高位（c < ZG）卖点不削减；高位恢复
    /// 削减（049:52"在中枢上方仓位减少"位置分量）。
    #[test]
    fn fusion_tr_blocks_low_position_trim_allows_high() {
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2)); // 入场（中枢 50/51）
        bars.push(sellpt(bar(50.5), 2)); // c < ZG=51 ⇒ 拦截
        bars.push(sellpt(bar(55.0), 2)); // c ≥ 51 ⇒ 削减 @55
        bars.push(bar(55.0));
        let r = run_with_rows(
            bars,
            "fusion_tr",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![]), // kind 恒 false ⇒ 全程震荡相
        );
        assert_eq!(r.n_r2_pos_blocks_by_ladder[2], 1, "低位卖点被位置门拦截");
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "sellpt");
        assert_eq!(t2[0].exit_price, 55.0);
    }

    /// R2 位置门域排除熊市腿（kind 时钟 dir==Down）：低位卖点照常削减
    /// （位置门对象 = 中枢震荡非下行段，049:40 映射声明——熊市 α 零接触）。
    #[test]
    fn fusion_tr_bear_leg_trims_freely_below_zg() {
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(45.0), 2)); // dir==Down ⇒ 域外，削减 @45
        bars.push(bar(45.0));
        let r = run_with_rows(
            bars,
            "fusion_tr",
            Some(vec![(0, 2, Direction::Down)]),
            Some(vec![]),
        );
        assert_eq!(r.n_r2_pos_blocks_by_ladder[2], 0);
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_price, 45.0);
    }

    /// P6×P7 合取（fusion_pr）：OSC 相低位拦截/高位放行；MOVE↓（Sell3 杀锚
    /// ⇒ 无存活中枢 = 位置词汇无对象）低位卖点回退出场语义照常削减。
    #[test]
    fn fusion_pr_osc_gate_and_movedown_fallback() {
        let w = SUB_COST_MIN_OBS as i64;
        let anchor_cs = 10 + w - 1;
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(sellpt(bar(50.5), 2)); // Φ=OSC ∧ c<ZG ⇒ 拦截
        bars.push(with_ev(bar(48.0), 2, ev_cs(BspClass::Sell3, anchor_cs))); // MOVE↓
        bars.push(sellpt(bar(45.0), 2)); // 锚已死无存活中枢 ⇒ 不拦截，削减 @45
        bars.push(bar(45.0));
        let r = run_phase(bars, "fusion_pr");
        assert_eq!(r.n_r2_pos_blocks_by_ladder[2], 1);
        assert_eq!(r.n_phase_dn_opens_by_ladder[2], 1);
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "sellpt");
        assert_eq!(t2[0].exit_price, 45.0);
    }

    // ════════ 双向条件轴 S1-S4（fusion_btr/fusion_btra）[镜像推导] ════════
    use crate::trading::types::Polarity;

    #[test]
    fn parse_btr_modes_and_guards() {
        // fusion_btr_s34 = fusion_tr 基座 + 层{3,4} 翻空白名单（S1/S3 BTC 臂）
        let Some(PolarityMode::Fusion {
            trend_hold, r2_gate, short_mask, short_anc_gate, counter_sub, ..
        }) = PolarityMode::parse("fusion_btr_s34")
        else {
            panic!("fusion_btr_s34 必须可解析")
        };
        assert!(trend_hold && r2_gate && !counter_sub);
        assert_eq!(short_mask, 0b11000);
        assert!(!short_anc_gate);
        // fusion_btra_s24 = 加镜像 anc 窗口门（S2/S4 CL 臂的条件化变体）
        let Some(PolarityMode::Fusion { short_mask: m2, short_anc_gate: g2, .. }) =
            PolarityMode::parse("fusion_btra_s24")
        else {
            panic!("fusion_btra_s24 必须可解析")
        };
        assert_eq!(m2, 0b10100);
        assert!(g2);
        // S3 尾部风险界：≥recL3（ladder 5）禁用；非法串全 None
        assert_eq!(PolarityMode::parse("fusion_btr_s5"), None);
        assert_eq!(PolarityMode::parse("fusion_btr_s45"), None);
        assert_eq!(PolarityMode::parse("fusion_btr_s1"), None);
        assert_eq!(PolarityMode::parse("fusion_btr_s"), None);
        assert_eq!(PolarityMode::parse("fusion_btr_s43"), None); // 乱序
        assert_eq!(PolarityMode::parse("fusion_btr_s33"), None); // 重复
        // 守卫：short_mask 仅 fusion_tr 基座合取预注册
        let t = SignalTape { bars: warmup34(), ..Default::default() };
        let bad = PolarityMode::Fusion {
            trend_hold: true,
            counter_sub: false,
            decoupled: false,
            trend_opts: TrendAxisOpts::default(),
            osc: OscRouting::Off,
            phase_clock: false,
            r2_gate: false, // 缺 r2 ⇒ 非 fusion_tr 基座
            trend_scope: TrendScope::SelfLayer,
            nest_forward: false,
            short_mask: 0b10000,
            short_anc_gate: false,
            short_ghost: false,
            seq38_sub: false,
        };
        assert!(run_positional(&t, 2, bad).is_err());
    }

    /// 翻转断面往返（§5.2 四步序 + 数字推演同构）：卖点平多+开空两行 trade、
    /// 买点平空+翻多；1x 逐仓 margin = 平多所得 ⇒ 断面 pool 净流转 0。
    #[test]
    fn flip_short_on_sellpt_then_cover_on_buypt() {
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 层4 入场 w=0.75 → 750 股
        bars.push(sellpt(bar(110.0), 4)); // 翻空断面：平多@110 + 开空 750u@110
        bars.push(buypt(bar(45.0), 4)); // c=45 ≤ ZD(50) ⇒ 回补放行 + 翻多
        bars.push(bar(45.0));
        let r = run_with_rows(bars, "fusion_btr_s4", Some(vec![]), Some(vec![]));
        assert_eq!(r.n_flip_shorts_by_ladder[4], 1);
        assert_eq!(r.n_short_covers_by_ladder[4], 1);
        let t4: Vec<_> = r.trades.iter().filter(|t| t.ladder == 4).collect();
        assert_eq!(t4.len(), 3, "平多 + 空头腿 + 翻多eod 三行（段归属不可合并）");
        assert_eq!(t4[0].polarity, Polarity::Long);
        assert_eq!((t4[0].entry_price, t4[0].exit_price), (100.0, 110.0));
        assert_eq!(t4[1].polarity, Polarity::Short);
        assert_eq!((t4[1].entry_price, t4[1].exit_price), (110.0, 45.0));
        assert_eq!(t4[1].exit_reason, "cover_buypt");
        assert_eq!(t4[1].shares, t4[0].shares, "M = N 同股数定理（26:34）");
        assert_eq!(t4[2].polarity, Polarity::Long);
        // 会计核验：750×(110−100) 多头段 + 750×(110−45) 空头段
        let expect = INITIAL_CAPITAL + 750.0 * 10.0 + 750.0 * 65.0;
        assert!((r.final_nav - expect).abs() < 1e-6, "{} ≠ {expect}", r.final_nav);
        assert!((r.short_net_cash_by_ladder[4] - 750.0 * 65.0).abs() < 1e-9);
    }

    /// 非白名单层零接触：层3 卖点照旧削减驻 Flat（在册 {+Q,0}），仅层4 翻空。
    #[test]
    fn non_whitelist_layer_keeps_inbook_flat() {
        let mut bars = warmup34();
        bars.push(buypt(buypt(bar(100.0), 3), 4));
        bars.push(sellpt(sellpt(bar(110.0), 3), 4));
        bars.push(bar(110.0));
        let r = run_with_rows(bars, "fusion_btr_s4", Some(vec![]), Some(vec![]));
        assert_eq!(r.n_flip_shorts_by_ladder[4], 1);
        assert_eq!(r.n_flip_shorts_by_ladder[3], 0, "层3 不在白名单——零接触");
        let t3: Vec<_> = r.trades.iter().filter(|t| t.ladder == 3).collect();
        assert!(t3.iter().all(|t| t.polarity == Polarity::Long));
    }

    /// MoveDown 相停回补（49:52 镜像满空仓义务）+ MoveUp 强制平空翻多
    /// （满仓义务——空头前提消失）。
    #[test]
    fn movedown_blocks_cover_moveup_forces_cover() {
        let w = SUB_COST_MIN_OBS as i64;
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // bar w
        bars.push(sellpt(bar(110.0), 4)); // bar w+1：翻空
        bars.push(buypt(bar(45.0), 4)); // bar w+2：MoveDown ⇒ 停回补
        bars.push(bar(60.0)); // bar w+3：MoveUp ⇒ 强制平空 + 翻多
        bars.push(bar(60.0));
        let dirs = vec![
            (w + 2, 4u8, Direction::Down),
            (w + 3, 4u8, Direction::Up),
        ];
        let trends = vec![(w + 2, 4u8, true)];
        let r = run_with_rows(bars, "fusion_btr_s4", Some(dirs), Some(trends));
        assert_eq!(r.n_short_trend_holds_by_ladder[4], 1, "MoveDown 停回补");
        assert_eq!(r.n_short_moveup_covers_by_ladder[4], 1, "MoveUp 强制平空");
        let s4: Vec<_> = r
            .trades
            .iter()
            .filter(|t| t.ladder == 4 && t.polarity == Polarity::Short)
            .collect();
        assert_eq!(s4[0].exit_reason, "cover_moveup");
        assert_eq!(s4[0].exit_price, 60.0);
    }

    /// 空侧 R2 回补位置门（049:64 镜像：c > ZD ⇒ 买点不回补——P7 回复侧
    /// 位置门获得对象）。
    #[test]
    fn short_r2_gate_blocks_cover_above_zd() {
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(sellpt(bar(110.0), 4)); // 翻空
        bars.push(buypt(bar(60.0), 4)); // c=60 > ZD(50) ⇒ 拦截
        bars.push(buypt(bar(45.0), 4)); // c=45 ≤ ZD ⇒ 放行
        bars.push(bar(45.0));
        let r = run_with_rows(bars, "fusion_btr_s4", Some(vec![]), Some(vec![]));
        assert_eq!(r.n_short_r2_blocks_by_ladder[4], 1);
        assert_eq!(r.n_short_covers_by_ladder[4], 1);
        let s4 = r
            .trades
            .iter()
            .find(|t| t.ladder == 4 && t.polarity == Polarity::Short)
            .unwrap();
        assert_eq!(s4.exit_price, 45.0, "回补成交在位置门放行的低位 bar");
    }

    /// 虚拟逐仓强平：1x 解析强平价 = 2×B_s；损失上界 = margin（pool 现金
    /// 流出恰为全部 margin，逐 trade 重建零渗漏）。
    #[test]
    fn short_liquidation_caps_loss_at_margin() {
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 750 股
        bars.push(sellpt(bar(110.0), 4)); // 翻空 @110，margin=82500
        bars.push(bar(250.0)); // 击穿 2×110=220 ⇒ 强平
        bars.push(bar(250.0));
        let r = run_with_rows(bars, "fusion_btr_s4", Some(vec![]), Some(vec![]));
        assert_eq!(r.n_short_liquidations_by_ladder[4], 1);
        let s4 = r
            .trades
            .iter()
            .find(|t| t.ladder == 4 && t.polarity == Polarity::Short)
            .unwrap();
        assert_eq!(s4.exit_reason, "short_liquidated");
        assert_eq!(s4.exit_price, 220.0, "强平价 = 2×B_s（1x 解析解）");
        // NAV：25000 现金 + 多头段利得 7500 已含在 margin 中被空头亏光
        // = 100000 + 750×10 − 82500 = 25000
        assert!((r.final_nav - 25_000.0).abs() < 1e-6, "{}", r.final_nav);
        assert!((r.short_net_cash_by_ladder[4] + 82_500.0).abs() < 1e-9);
    }

    /// fusion_btra：镜像 anc 窗口门——∄祖先 Trend∧Down ⇒ 削减照常不开空；
    /// ∃ ⇒ 开空（26:80 豁免下沉的空头镜像，S2 条件化形式）。
    #[test]
    fn btra_anc_gate_conditions_flip() {
        let w = SUB_COST_MIN_OBS as i64;
        // 臂一：无祖先 Down 趋势 ⇒ 拒开空，层留 Flat（在册削减驻留）
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(sellpt(bar(110.0), 4));
        bars.push(bar(110.0));
        let r = run_with_rows(bars, "fusion_btra_s4", Some(vec![]), Some(vec![]));
        assert_eq!(r.n_short_anc_rejects_by_ladder[4], 1);
        assert_eq!(r.n_flip_shorts_by_ladder[4], 0);
        // 臂二：祖先层5 Trend∧Down ⇒ 开空
        let mut bars2 = warmup34();
        bars2.push(buypt(bar(100.0), 4));
        bars2.push(sellpt(bar(110.0), 4));
        bars2.push(bar(110.0));
        let dirs = vec![(w, 5u8, Direction::Down)];
        let trends = vec![(w, 5u8, true)];
        let r2 = run_with_rows(bars2, "fusion_btra_s4", Some(dirs), Some(trends));
        assert_eq!(r2.n_flip_shorts_by_ladder[4], 1);
        assert_eq!(r2.n_short_anc_rejects_by_ladder[4], 0);
    }

    /// 纯回复门消融臂（fusion_btrg）：卖点削减进 Gated（持币不开空），
    /// 出口判据与 Short 同款——MoveDown 停回复 + c≤ZD 放行回复。
    #[test]
    fn btrg_gates_restore_without_short_exposure() {
        let w = SUB_COST_MIN_OBS as i64;
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // bar w：入场 750 股
        bars.push(sellpt(bar(110.0), 4)); // bar w+1：削减 → Gated（不开空）
        bars.push(buypt(bar(45.0), 4)); // bar w+2：MoveDown ⇒ 停回复
        bars.push(buypt(bar(45.0), 4)); // bar w+3：门放行（c≤ZD）⇒ 回复
        bars.push(bar(45.0));
        let dirs = vec![(w + 2, 4u8, Direction::Down), (w + 3, 4u8, Direction::Up)];
        // bar w+2 trend∧Down = MoveDown 拦截；w+3 dir 翻 Up（非 trend ⇒
        // 非 MoveUp 强制，r2 in_domain=dir==Up 不拦 ⇒ 买点正常回复）
        let trends = vec![(w + 2, 4u8, true), (w + 3, 4u8, false)];
        let r = run_with_rows(bars, "fusion_btrg_s4", Some(dirs), Some(trends));
        assert_eq!(r.n_gate_enters_by_ladder[4], 1);
        assert_eq!(r.n_flip_shorts_by_ladder[4], 0, "消融臂零空头暴露");
        assert_eq!(r.n_short_trend_holds_by_ladder[4], 1, "MoveDown 拦截同款计数");
        assert_eq!(r.n_gate_restores_by_ladder[4], 1);
        assert!(r.trades.iter().all(|t| t.polarity == Polarity::Long));
        // 会计：多头段 750×10 + 持币穿越下跌 + @45 回复（无空头收割）
        let t4: Vec<_> = r.trades.iter().filter(|t| t.ladder == 4).collect();
        assert_eq!(t4.len(), 2, "削减行 + 回复eod 行（无空头腿）");
        assert!((r.final_nav - (INITIAL_CAPITAL + 7_500.0)).abs() < 1e-6);
    }

    #[test]
    fn btrg_parse_and_guards() {
        let Some(PolarityMode::Fusion { short_mask, short_ghost, short_anc_gate, .. }) =
            PolarityMode::parse("fusion_btrg_s34")
        else {
            panic!("fusion_btrg_s34 必须可解析")
        };
        assert_eq!(short_mask, 0b11000);
        assert!(short_ghost && !short_anc_gate);
        assert_eq!(PolarityMode::parse("fusion_btrg_s5"), None);
        // ghost × anc_gate 合取拒绝（守卫）
        let t = SignalTape { bars: warmup34(), ..Default::default() };
        let bad = PolarityMode::Fusion {
            trend_hold: true,
            counter_sub: false,
            decoupled: false,
            trend_opts: TrendAxisOpts::default(),
            osc: OscRouting::Off,
            phase_clock: false,
            r2_gate: true,
            trend_scope: TrendScope::SelfLayer,
            nest_forward: false,
            short_mask: 0b10000,
            short_anc_gate: true,
            short_ghost: true,
            seq38_sub: false,
        };
        assert!(run_positional(&t, 2, bad).is_err());
    }

    /// 翻转断面会计无渗漏：同价开平往返 ⇒ NAV 守恒（零摩擦语法前提）。
    #[test]
    fn flip_round_trip_nav_conservation() {
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(sellpt(bar(100.0), 4)); // 同价翻空
        bars.push(buypt(bar(100.0), 4)); // 同价翻多（c=100 > ZD…r2 拦截？）
        bars.push(bar(100.0));
        // c=100 > ZD(50) ⇒ 空侧 R2 拦截回补——本测试验证的是断面本身的
        // 会计守恒，eod 收口同价平空：NAV 必须回到初始。
        let r = run_with_rows(bars, "fusion_btr_s4", Some(vec![]), Some(vec![]));
        assert!((r.final_nav - INITIAL_CAPITAL).abs() < 1e-9, "{}", r.final_nav);
    }

    // ════════ 全量普适组合（fusion_btran_s{digits}；2026-06-12 预注册）════════

    #[test]
    fn parse_btran_and_guards() {
        // fusion_btran_s34 = btra 双向基座 + 区间套正向定位（nest×short
        // 唯一预注册形态：anc 门 ∧ ¬ghost）。
        let Some(PolarityMode::Fusion {
            trend_hold, r2_gate, short_mask, short_anc_gate, short_ghost,
            nest_forward, ..
        }) = PolarityMode::parse("fusion_btran_s34")
        else {
            panic!("fusion_btran_s34 必须可解析")
        };
        assert!(trend_hold && r2_gate && short_anc_gate && nest_forward);
        assert!(!short_ghost);
        assert_eq!(short_mask, 0b11000);
        // 非法串：空白名单 / S3 界 / 乱序
        assert_eq!(PolarityMode::parse("fusion_btran_s"), None);
        assert_eq!(PolarityMode::parse("fusion_btran_s5"), None);
        assert_eq!(PolarityMode::parse("fusion_btran_s43"), None);
        // 守卫：nest × short 仅 anc 门形态预注册——btr（无 anc）+ nest 拒
        let mk = |anc: bool, ghost: bool| PolarityMode::Fusion {
            trend_hold: true,
            counter_sub: false,
            decoupled: false,
            trend_opts: TrendAxisOpts::default(),
            osc: OscRouting::Off,
            phase_clock: false,
            r2_gate: true,
            trend_scope: TrendScope::SelfLayer,
            nest_forward: true,
            short_mask: 0b10000,
            short_anc_gate: anc,
            short_ghost: ghost,
            seq38_sub: false,
        };
        let t = SignalTape {
            bars: warmup34(),
            dir_flips: Some(vec![]),
            trend_flips: Some(vec![]),
            ..Default::default()
        };
        assert!(run_positional(&t, 2, mk(false, false)).is_err(), "btr+nest 未预注册");
        assert!(run_positional(&t, 2, mk(false, true)).is_err(), "btrg+nest 未预注册");
        assert!(run_positional(&t, 2, mk(true, false)).is_ok(), "btran 形态放行");
    }

    /// 全量组合行为：nest 卖触发沿翻转断面开空（nest_sell 行 + 翻空），
    /// nest 买触发沿平空出口回补翻多（cover_nest 行）——nest 与翻空/平空
    /// 动作正交合取（nest = 同一买卖点的更早时间坐标，词汇地位对称）。
    #[test]
    fn btran_nest_sell_flips_short_and_nest_buy_covers() {
        let w = SUB_COST_MIN_OBS as i64;
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 3)); // 层3 入场 @100
        bars.push(with_ev(bar(100.0), 3, cand(BspClass::Sell1, 99, 200.0))); // 武装卖窗
        bars.push(with_ev(bar(98.0), 2, ev(BspClass::Sell1))); // 次级别证据 → nest 削减 + 翻空
        bars.push(with_ev(bar(60.0), 3, cand(BspClass::Buy1, 88, 40.0))); // 武装买窗
        bars.push(with_ev(bar(45.0), 2, ev(BspClass::Buy1))); // 次级别证据 → 平空（c≤ZD 放行）+ 翻多
        bars.push(bar(45.0));
        // 祖先层5 Trend∧Down 全程 ⇒ 镜像 anc 开空窗口开
        let dirs = vec![(w, 5u8, Direction::Down)];
        let trends = vec![(w, 5u8, true)];
        let r = run_with_rows(bars.clone(), "fusion_btran_s34", Some(dirs), Some(trends));
        assert_eq!(r.n_nest_fire_sell_by_ladder[3], 1, "nest 卖窗正向触发");
        assert_eq!(r.n_flip_shorts_by_ladder[3], 1, "nest 削减沿翻转断面开空");
        assert_eq!(r.n_nest_fire_buy_by_ladder[3], 1, "nest 买窗正向触发");
        assert_eq!(r.n_short_covers_by_ladder[3], 1, "nest 买触发平空");
        let t3: Vec<_> = r.trades.iter().filter(|t| t.ladder == 3).collect();
        assert_eq!(t3.len(), 3, "平多 + 空头腿 + 翻多eod 三行");
        assert_eq!(t3[0].polarity, Polarity::Long);
        assert_eq!(t3[0].exit_reason, "nest_sell");
        assert_eq!((t3[0].entry_price, t3[0].exit_price), (100.0, 98.0));
        assert_eq!(t3[1].polarity, Polarity::Short);
        assert_eq!(t3[1].exit_reason, "cover_nest", "纯正向触发平空归因");
        assert_eq!((t3[1].entry_price, t3[1].exit_price), (98.0, 45.0));
        assert_eq!(t3[1].shares, t3[0].shares, "M = N 同股数定理（26:34）");
        assert_eq!(t3[2].polarity, Polarity::Long, "平空翻多回复");
        // 零接触差分：同磁带 fusion_btra_s34（无 nest）——confirmed 掩码
        // 从未置位 ⇒ 不削减不翻空，nest 计数恒零。
        let r2 = run_with_rows(
            bars,
            "fusion_btra_s34",
            Some(vec![(w, 5u8, Direction::Down)]),
            Some(vec![(w, 5u8, true)]),
        );
        assert_eq!(r2.n_nest_fire_sell_by_ladder[3], 0);
        assert_eq!(r2.n_flip_shorts_by_ladder[3], 0);
    }

    // ════════ 全量普适组合消融矩阵（fusion_btra{mods}；2026-06-12 v2）════════

    #[test]
    fn parse_btra_mods_grammar() {
        // mods ⊆ {n,u,f,q} 规范序；u = osc 层（H4 承载臂）；f = H1 冻结
        // （要求 u）；q = Sequence38 子腿。
        let Some(PolarityMode::Fusion { osc, seq38_sub, nest_forward, .. }) =
            PolarityMode::parse("fusion_btrau_s34")
        else {
            panic!("fusion_btrau_s34 必须可解析")
        };
        assert_eq!(osc, OscRouting::Unified { strong_gate: true, h1_freeze: false });
        assert!(!seq38_sub && !nest_forward);
        let Some(PolarityMode::Fusion { osc: o2, .. }) =
            PolarityMode::parse("fusion_btrauf_s34")
        else {
            panic!("fusion_btrauf_s34 必须可解析")
        };
        assert_eq!(o2, OscRouting::Unified { strong_gate: true, h1_freeze: true });
        let Some(PolarityMode::Fusion { seq38_sub: q3, osc: o3, .. }) =
            PolarityMode::parse("fusion_btraq_s34")
        else {
            panic!("fusion_btraq_s34 必须可解析")
        };
        assert!(q3);
        assert_eq!(o3, OscRouting::Off);
        let Some(PolarityMode::Fusion {
            nest_forward: n4, osc: o4, seq38_sub: q4, short_mask: m4,
            short_anc_gate: a4, ..
        }) = PolarityMode::parse("fusion_btranufq_s34")
        else {
            panic!("fusion_btranufq_s34 必须可解析")
        };
        assert!(n4 && q4 && a4);
        assert_eq!(o4, OscRouting::Unified { strong_gate: true, h1_freeze: true });
        assert_eq!(m4, 0b11000);
        // 非法串：f 无 u 载体 / 乱序 / 重复 / 未知字母
        assert_eq!(PolarityMode::parse("fusion_btraf_s34"), None);
        assert_eq!(PolarityMode::parse("fusion_btraun_s34"), None);
        assert_eq!(PolarityMode::parse("fusion_btrann_s34"), None);
        assert_eq!(PolarityMode::parse("fusion_btrax_s34"), None);
        // 守卫：q × btr（无 anc）直构拒绝（仅 btra 基座预注册）
        let t = SignalTape {
            bars: warmup34(),
            dir_flips: Some(vec![]),
            trend_flips: Some(vec![]),
            ..Default::default()
        };
        let bad = PolarityMode::Fusion {
            trend_hold: true,
            counter_sub: false,
            decoupled: false,
            trend_opts: TrendAxisOpts::default(),
            osc: OscRouting::Off,
            phase_clock: false,
            r2_gate: true,
            trend_scope: TrendScope::SelfLayer,
            nest_forward: false,
            short_mask: 0b10000,
            short_anc_gate: false,
            short_ghost: false,
            seq38_sub: true,
        };
        assert!(run_positional(&t, 2, bad).is_err(), "q×btr 未预注册");
    }

    /// H1 candidate 冻结：未决 candidate type3 离开段 ⇒ osc 不开（btrauf）；
    /// 同磁带 H1 关（btrau）⇒ 照常开——单门差分可观测。
    #[test]
    fn btrauf_h1_freezes_osc_open() {
        let w = SUB_COST_MIN_OBS as i64;
        let mk = || {
            let mut bars = warmup2();
            bars.push(buypt(bar(100.0), 2)); // 层2 入场
            // candidate Buy3（向上离开段未决）⇒ H1 窗口置位；c=100 ≥ ZG=51
            // ⇒ 价格不否定窗口（38课答疑"能回到中枢就不是三买"的逆面）。
            bars.push(with_ev(
                bar(100.0),
                2,
                BspEvent {
                    class: BspClass::Buy3,
                    seg_idx: 0,
                    confirmed: false,
                    cs: Some(10 + w - 1),
                    zd: Some(50.0),
                    zg: Some(51.0),
                    price: 100.0,
                },
            ));
            bars.push(sellpt(bar(100.0), 1)); // 开腿触发面（c≥ZG ∧ sub_sell）
            bars.push(bar(100.0));
            bars
        };
        let frozen = run_with_rows(
            mk(),
            "fusion_btrauf_s34",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![]),
        );
        assert_eq!(frozen.n_osc_opens_by_ladder[2], 0, "H1 窗口内不开");
        assert!(frozen.n_route_h1_freezes[2] >= 1, "H1 拦截可观测");
        let open = run_with_rows(
            mk(),
            "fusion_btrau_s34",
            Some(vec![(0, 2, Direction::Up)]),
            Some(vec![]),
        );
        assert_eq!(open.n_osc_opens_by_ladder[2], 1, "H1 关 ⇒ 同磁带照常开");
        assert_eq!(open.n_route_h1_freezes[2], 0);
    }

    /// custody 分区：翻空白名单层（s34 = {3,4}）的 slice 由双向词汇独占
    /// ——osc 开腿条件齐备仍跳过（无路由尝试无开腿）。
    #[test]
    fn btrau_osc_skips_short_whitelist_layers() {
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 3)); // 层3 ∈ short_mask 入场
        bars.push(sellpt(bar(100.0), 2)); // sub 证据（j=3 的 j−1=2）
        bars.push(bar(100.0)); // c=100 ≥ ZG(3)=51
        let r = run_with_rows(
            bars,
            "fusion_btrau_s34",
            Some(vec![(0, 3, Direction::Up)]),
            Some(vec![]),
        );
        assert_eq!(r.n_osc_opens_by_ladder[3], 0, "白名单层 custody 跳过");
        assert_eq!(r.n_osc_opens_by_ladder[4], 0);
    }

    /// Sequence38 子腿（38课:36 程式）：盘背卖开腿（先卖）→ 盘背买买回
    /// （如数接回，归因 consbuy）——非白名单层激活，会计走 SubOut 载具。
    #[test]
    fn btraq_seq38_opens_and_closes_on_consolidation_div() {
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2)); // 层2 入场 1000股 @100（w₂=1.0）
        bars.push(with_div(bar(100.0), 2, div_ev(Direction::Up))); // 盘背卖 → 开
        bars.push(with_div(bar(80.0), 2, div_ev(Direction::Down))); // 盘背买 → 回
        bars.push(bar(80.0));
        let r = run_with_rows(bars, "fusion_btraq_s34", Some(vec![]), Some(vec![]));
        assert_eq!(r.n_seq38_opens_by_ladder[2], 1, "盘背卖开腿");
        assert_eq!(r.n_seq38_consbuy_closes_by_ladder[2], 1, "盘背买买回归因");
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "seq38_diff");
        assert_eq!((t2[0].entry_price, t2[0].exit_price), (100.0, 100.0));
        assert_eq!(t2[1].entry_price, 80.0, "接回点重锚");
        assert!((r.sub_net_cash_by_ladder[2] - 20_000.0).abs() < 1e-6);
        // 短差 +1000×(100−80) 恰对冲持仓跌幅 −1000×(100−80) ⇒ NAV 守恒
        // （seq38 子腿的存在论：同一段下跌被先卖后买兑现为现金）
        assert!((r.final_nav - 100_000.0).abs() < 1e-6, "nav={}", r.final_nav);
        // custody：白名单层（3,4）零 seq38 激活（同磁带无事件本就不开——
        // 此处断言的是计数面恒零）
        assert_eq!(r.n_seq38_opens_by_ladder[3], 0);
        assert_eq!(r.n_seq38_opens_by_ladder[4], 0);
    }

    /// Seq38 不破第一段低点岔（答疑:296）：低点未破 ∧ 次级别方向行翻多
    /// ⇒ 买回（nobreak 归因）——纯几何触线不充分，结构确认是判据本体。
    #[test]
    fn btraq_seq38_nobreak_close_requires_sub_confirm() {
        let w = SUB_COST_MIN_OBS as i64;
        let mut bars = warmup2();
        bars.push(buypt(bar(100.0), 2));
        bars.push(bar(99.0)); // run_low 打低至 99（第一段低点参照）
        bars.push(with_div(bar(100.0), 2, div_ev(Direction::Up))); // 开（seg1_low=99）
        bars.push(bar(99.5)); // 在外低点 99.5 > seg1_low=99（不破）；无确认 ⇒ 不回
        bars.push(bar(99.5)); // dir 行在本 bar 翻多（次级别结构确认）⇒ nobreak 买回
        bars.push(bar(99.5));
        let dirs = vec![(w + 4, 1u8, Direction::Up)];
        let r = run_with_rows(bars, "fusion_btraq_s34", Some(dirs), Some(vec![]));
        assert_eq!(r.n_seq38_opens_by_ladder[2], 1);
        assert_eq!(r.n_seq38_nobreak_closes_by_ladder[2], 1, "确认到达才买回");
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "seq38_diff");
    }
}
