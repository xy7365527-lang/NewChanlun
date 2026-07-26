//! 因果选择器 χ_t(γ) 阈值过滤（买卖点alpha2.pdf §13 + §17，task #41 chi-theta-filter）。
//!
//! ## 命题（alpha2 §13 line 2239）
//!
//! 把目标从「∀e 吃到所有元素」改成「只交易具有正边际条件期望的买卖点证书」。形式上定义
//! **全定义**选择函数 χ_t(γ) ∈ {0,1}：
//!
//! ```text
//!   χ_t(γ) = 1 ⟺ μ(γ) > θ ∧ RiskOK(γ, x_t) ∧ ConflictOK(γ, A_t)     (§13 line 2239)
//! ```
//!
//! θ ≥ 0 是成本和风险门槛（§13 line 2241）。解释器只处理 Γ_t^trade = {γ∈Γ_t : χ_t(γ)=1}。
//! 这仍**全互斥、全定义**——χ_t 是全定义函数，解释器仍有限、确定（§13 line 2262）。
//!
//! ## 动作选择 π̄(z)（alpha2 §17 line 3524）
//!
//! ```text
//!   π̄(z) = argmax_{a∈A(z)} μ(z,a)    if max_a μ(z,a) > θ
//!         = Hold/Flat                  否则                       (§17)
//! ```
//!
//! 仍**全定义**——每个 z 都有动作；无正边际收益时动作为 Hold/Flat（§17 line 3531）。本模块
//! 实装 §13 的二元过滤 [`chi_t`]（开/不开）与 §17 的多动作 argmax [`pi_bar`]（在一个 z 上多个
//! 候选动作中选 μ 最大者）。二者同一阈值 θ：§13 是 §17 在 |A(z)|=1（单候选 = 开/Hold）的特例。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! | 组件 | 等级 | 理由 |
//! |------|------|------|
//! | 选择器逻辑（[`chi_t`]/[`pi_bar`] 的合取/argmax/阈值比较） | **L1** | 给定 μ/θ/谓词求 χ 是确定性布尔/argmax，验证逻辑正确，零信息增量 |
//! | 阈值 θ 取值 | **Θ_risk 参数（非缠论可导）** | θ 是成本+风险门槛（§13 line 2241），由成本模型/风险偏好给定，**不能从缠论语法推出**（诚实标注） |
//! | 真实 μ 驱动的过滤结果（哪些 γ 被滤掉、ΔN 如何变） | **L2** | 喂真实历史 μ 才能否证「过滤提升 alpha」（正信息增量） |
//!
//! **关键诚实声明**（no-patch / formalization-validity-domain）：χ_t 用的 μ 表若来自**同一遍
//! in-sample 全程交易**（用全程未来交易估的 μ 过滤当前开仓），是**未来函数泄漏**（§5 因果性硬
//! 约束 / project_zero_lookahead_backtest）——此时过滤结果只证明**选择器逻辑生效**（χ≡1 与
//! χ=1[μ>θ] 的 ΔN 序列非全等，**L1**），**不**证明 alpha 提升（**那需要 walk-forward μ，是后续
//! 工位**）。本模块只提供选择器纯函数；μ 表的因果获取由调用方负责并诚实标注其泄漏状态。

use std::rc::Rc;

use super::econ_positive::NestTrigger;
use super::mu_estimator::{MuClass, MuEstimator, PositionState};
use crate::theta_v0::classifier::recursive_tower::LeveledMove;
use crate::theta_v0::strategy::coverage::Vertical;
use crate::theta_v0::strategy::interp::Candidate;
use crate::theta_v0::strategy::ledger::{EtaBucket, TStage};
use crate::theta_v0::strategy::risk::RiskMode;
use crate::theta_v0::strategy::voice::VoiceSide;
use crate::theta_v0::types::Bar;

/// G3（#138）z 第 10-13 维装配参数（§6 CandType/Ndepth/ℓ/RiskMode）。
///
/// 新维数据源在**调用方语境**而非 `Candidate` 上：准入门证书（NestTrigger/深度/链顶）只在
/// econ 统计层 collect_signals 的门判定现场，账本态（RiskMode）只在 runner π fill loop 的
/// 风控门现场——经本结构显式携带进 z 构造。
///
/// **护航点（G2 tower/bars 同款）**：[`z_of_candidate`]/
/// [`filter_gamma`]* 签名强制携带本参数。无对应数据源的路径显式传 [`ZExt::NONE`]——这是
/// 可 grep 审计的诚实声明（「本路径未经准入门/无账本」），静默遗漏在类型层不可构造；
/// 训练表与 χ 查询在同一现场共用同一 `ZExt` 值 ⟹ 同口径（防训练 Some/查询 None 全表 miss）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZExt {
    /// Cand 门通道（§6 CandType）：econ 门路径 `Some(nest_trigger(..))`；无门路径 None。
    pub cand_channel: Option<NestTrigger>,
    /// 区间套下沉深度（§6 Ndepth）：Nest 通道 `Some(rungs.len())`（0=基例真值）；
    /// Xzd/无门路径 None（下沉概念未定义，非 0）。
    pub nest_depth: Option<u8>,
    /// 起始级 ℓ 覆盖（§6 ℓ）：`Some(ℓ)`=区间套链顶（Nest 通道 `lvl + rungs.len()`）；
    /// `None` ⟹ z 填 `Some(c.level)`（起始=执行的无下沉**真值**——级别事实对任何真候选
    /// 有定义，Xzd/runner 路径皆此口径；裸 `from_certificate` 才是 None）。
    pub origin_level: Option<u32>,
    /// 账户风险模式（§6 RiskMode+MarginState）：runner π fill loop `Some(当 bar mode)`；
    /// 无账本路径（econ 统计层）None。
    pub risk_mode: Option<RiskMode>,
    /// 取本金三阶段（§6 TStage，#149 zdims 第 14 维）：runner π fill loop `Some(tw.stage)`
    /// ——当 bar 决策点 TW 账本相位真值（与 `TwStepCtx.state` 同一 `tw` 变量，P2/P3/P4 谓词
    /// 同口径）；无 TW 账本路径（econ 统计层）None（同 risk_mode 诚实口径）。
    pub t_stage: Option<TStage>,
    /// γ_t 四桶 ηBucket（§6 ηBucket，#175 zdims 第 15 维）：runner π fill loop
    /// `Some(tw_policy.eta_bucket(&tw))`——η_t=`TwState::tw()` 与 η_*=`RiskPolicy::eta_star()`
    /// 的 PDF §10 分段式离散化（终裁 a5-etabucket-stance-ruling-20260704.md，与 `t_stage` 同一
    /// `tw` 变量同一装配点）；无 TW 账本路径（econ 统计层）None（同 t_stage 诚实口径）。
    pub eta_bucket: Option<EtaBucket>,
}

impl ZExt {
    /// 无扩展数据源口径（诚实声明载体）：扩展维全 None——z 仍填 `origin_level=Some(c.level)`
    /// （见字段文档，级别事实非门产物）。
    pub const NONE: ZExt = ZExt {
        cand_channel: None,
        nest_depth: None,
        origin_level: None,
        risk_mode: None,
        t_stage: None,
        eta_bucket: None,
    };
}

/// χ_t(γ) = 1 ⟺ μ(γ) > θ ∧ RiskOK ∧ ConflictOK（§13 line 2239）。
///
/// - `mu`：μ(γ) = μ(z) 的样本估计（[`MuEstimator::mu`]）。`None` = 该 z 类**无样本**（空类无
///   估计，[`MuEstimator::mu`] 文档：空类 ≠ μ=0）。空类的 χ 取值由 `treat_empty_as_pass`
///   决定（见下）——本函数不替空类伪造 μ=0。
/// - `theta`：θ ≥ 0 成本/风险门槛（**Θ_risk 参数，非缠论可导**，诚实标注）。
/// - `risk_ok`：RiskOK(γ, x_t)——风险约束满足（K_Θ 投影/止损/杠杆上限）。
/// - `conflict_ok`：ConflictOK(γ, A_t)——全互斥解释器的 C_j 唯一裁决（同一时刻冲突唯一处理）。
/// - `treat_empty_as_pass`：空类（μ=None）时 χ 取值。**全覆盖语义**（χ≡1 退化）应传 `true`
///   （未见过的 z 默认交易，等价 θ→−∞ 对空类）；**严格过滤语义**应传 `false`（无 μ 证据 ⟹
///   不交易，保守）。这是 §13 未明确的边界——空类是「未观测」非「μ≤θ」，二语义都合法，
///   由调用方按 in-sample/walk-forward 阶段选。
///
/// # 全定义性（§13 line 2262）
/// 对任意输入恰返回一个 bool——χ_t 是全定义函数，无 panic 无未定义分支。
#[inline]
pub fn chi_t(
    mu: Option<f64>,
    theta: f64,
    risk_ok: bool,
    conflict_ok: bool,
    treat_empty_as_pass: bool,
) -> bool {
    let mu_pass = match mu {
        Some(m) => m > theta,            // 有样本：μ(γ) > θ（§13 严格大于）
        None => treat_empty_as_pass,      // 空类：未观测，由语义参数定（非伪造 μ=0）
    };
    mu_pass && risk_ok && conflict_ok
}

/// π̄(z) = argmax_{a∈A(z)} μ(z,a) if max>θ else Hold/Flat（§17 line 3524）。
///
/// 在一个状态 z 的候选动作集 `A(z)` 上选 μ 最大者；若最大 μ 仍 ≤ θ ⟹ `None`（Hold/Flat，§17）。
/// `actions` 是 (动作标识 a, μ(z,a)) 列表——μ 由 [`MuEstimator`] 在 (z,a) 复合类上估出（调用方
/// 把动作维度编码进 [`MuClass`] 或单独传）。返回 `Some(最优 a)` 或 `None`(Hold/Flat)。
///
/// # 全定义性（§17 line 3531）
/// 空 `actions` 或全部 μ≤θ ⟹ `None`(Hold/Flat)——每个 z 都有动作（最坏 Hold），无 panic。
pub fn pi_bar<A: Copy>(actions: &[(A, f64)], theta: f64) -> Option<A> {
    actions
        .iter()
        .filter(|(_, mu)| *mu > theta) // 只保留 μ>θ 的候选（§17：max>θ 才动作）
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(a, _)| *a)
}

/// 便利包装：用 [`MuEstimator`] 查 z 的 μ 后做 χ_t 过滤（开仓门，§13）。
///
/// `est` 是 **Pass 1（χ≡1）估出的 μ 表**——调用方负责保证其因果性（in-sample 全程 μ 过滤
/// 当前开仓 = 泄漏，结果是 L1 选择器逻辑生效证明，非 L2 alpha；见模块头诚实声明）。
#[inline]
pub fn chi_open_gate(
    est: &MuEstimator,
    z: &MuClass,
    theta: f64,
    risk_ok: bool,
    conflict_ok: bool,
    treat_empty_as_pass: bool,
) -> bool {
    chi_t(est.mu(z), theta, risk_ok, conflict_ok, treat_empty_as_pass)
}

/// LCB 便利包装：准入量用 **LCB(μ)** 而非裸 μ（严格alpha.pdf p25 §12——置信下界防高维 z 过拟合）。
///
/// 与 [`chi_open_gate`] 同构，唯一区别：查 [`MuEstimator::mu_lcb`]（`mean − z_alpha·std/√n`）而非
/// [`MuEstimator::mu`]。`z_alpha` 单边正态分位（如 1.645=95%）由调用方传。
///
/// **None 语义统一**（n<2 ⟹ mu_lcb=None；空类 ⟹ None）：二者皆「无 LCB 证据」，由
/// `treat_empty_as_pass` 裁决——`false` = 无证据不交易（与 p25「无正边际收益证据时不交易」一致），
/// `true` = 全覆盖默认放行。selector 不替无证据类伪造 LCB（mu_lcb 已诚实返 None）。
///
/// **退化**：θ=−∞ ⟹ LCB>−∞ 对任何有 LCB 的类恒真（全覆盖不滤已观测 n≥2 类）；None 类仍按
/// `treat_empty_as_pass`。n→∞ ⟹ std/√n→0 ⟹ LCB→mean ⟹ 与裸 μ 门决策收敛。
///
/// 认识论 **L1**（给定 μ/std/θ/z_alpha 求 χ 是确定性布尔，验证选择器逻辑，零信息增量）。
#[inline]
pub fn chi_open_gate_lcb(
    est: &MuEstimator,
    z: &MuClass,
    theta: f64,
    z_alpha: f64,
    risk_ok: bool,
    conflict_ok: bool,
    treat_empty_as_pass: bool,
) -> bool {
    chi_t(est.mu_lcb(z, z_alpha), theta, risk_ok, conflict_ok, treat_empty_as_pass)
}

/// σ_higher：信号所在 level 的上级层（tower[level+1]）末走势端点价净差符号（666 号，见
/// SignalDecomp.sigma_higher；667 号实证级别依赖调制器）。+1 净涨 / −1 净跌 / 0 持平；
/// level+1 越界或上级层空 → 0。端点越界/非正 close → 0（诚实，不兜底）。
///
/// **单一来源**（codex-q1 G2）：z 构造（[`z_of_candidate`]）与 econ_positive 信号分解
/// （SignalDecomp.sigma_higher）共用本函数——训练表与生产 χ 查询同口径，防"训练填真值/
/// 查询填 None"的静默桶不命中（H 轴接入时修过的同类陷阱，z-bucket-impl 边界条件(a)）。
pub(super) fn sigma_higher_at(tower: &[Rc<Vec<LeveledMove>>], bars: &[Bar], level: usize) -> i8 {
    let Some(upper) = tower.get(level + 1) else { return 0 };
    let Some(m) = upper.last() else { return 0 };
    let (s, e) = (bars.get(m.start_index), bars.get(m.end_index));
    match (s, e) {
        (Some(sb), Some(eb)) if sb.close > 0 && eb.close > 0 => {
            (eb.close - sb.close).signum() as i8
        }
        _ => 0,
    }
}

/// 候选 γ ([`Candidate`]) → 全互斥分类 z ([`MuClass`])（§16 z 基六维 + H/force/σ_higher +
/// G3 门/账本四维 cand_channel/nest_depth/origin_level/risk_mode）的桥接。
///
/// `Candidate` 已带 `level`/`dir`(δ_g)/`bits`(I_γ)/`role`(R(g)=(H,V,δ))。z 的 `parent_dir` σ_p 与
/// `position` 仓位态从 `role.v`（[`Vertical`]）推（与 [`MuClass::from_certificate`] 的 `short_swing`
/// 推导口径一致）：
/// - `Ambient`（σ_p=0，去根化无父）⟹ `position=Root`，`parent_dir=0`。
/// - `FollowParent`（δ_g=σ_p，顺父）⟹ `position=Child`，`parent_dir=δ_g`（同向）。
/// - `ReverseOpen`（δ_g=−σ_p，首开反向）⟹ `position=Child`，`parent_dir=−δ_g`（反向）。
///
/// `tower`/`bars`（codex-q1 G2 护航点）：σ_higher 第 9 维从塔真值取（[`sigma_higher_at`]，按
/// `c.level`）——**签名强制**携塔，使"训练表填真值/生产查询无塔填 None"的静默退化在类型层不可
/// 构造（不存在无塔重载）。所有真候选路径（collect_signals/build_mu_from_bars/fill loop χ 门）
/// 均有 `tower_i`/`bars` 在手。
///
/// `force_state` 第 8 维（A6 #159）：从 `c.force`（[`Candidate`] 透传 `BspPoint.force`，A/C 段
/// 5-proxy 单一来源）经 divergence.rs 唯一支配序原语 `ForceProxies::force_state()` 填；`c.force=None`
/// （二/三类等无 A/C 对）⟹ `force_state=None`（诚实，同 horizontal None）。**本函数是 force_state
/// 的唯一装配点**——旧 `z_of_candidate_with_force` 旁路已删（Candidate 携 force 后，「平装版静默
/// None」与「旁路传错源」两类退化在类型层同时不可构造）。
///
/// `Flat` 方向候选（不可交易，归 𝒦_x 记录）δ 占位 +1——其 z 不被 χ 用于开仓（interpret 已归 record）；
/// 此桥接只服务**方向候选**的 χ 过滤，Flat 候选由 [`filter_gamma`] 在构 z 前按 `c.dir==Flat` 跳过。
///
/// `ext`（G3 #138 护航点）：第 10-13 维数据源显式携带（见 [`ZExt`]）——训练表与 χ 查询共用
/// 同一 `ZExt` 现场值 ⟹ 同口径在类型层保证。
pub fn z_of_candidate(
    c: &Candidate,
    tower: &[Rc<Vec<LeveledMove>>],
    bars: &[Bar],
    ext: &ZExt,
) -> MuClass {
    let delta: i8 = match c.dir {
        VoiceSide::Long => 1,
        VoiceSide::Short => -1,
        VoiceSide::Flat => 1, // Flat 不可交易候选，δ 占位（调用方不对 Flat 构 z 做开仓门）
    };
    let (parent_dir, position) = match c.role.v {
        Vertical::Ambient => (0, PositionState::Root),
        Vertical::FollowParent => (delta, PositionState::Child), // σ_p = δ_g
        // GPT 命名冲突裁决：ReverseOpen = δ_g=−σ_p（商映射 AgainstParent，含同级别+次级别反父；原 ShortDiff，#281 更名）。
        // σ_p=−δ_g（反向腿）；同级别/次级别区分在独立 G 轴 c.role.grade，不进 MuClass 此投影（商映射同桶）。
        Vertical::ReverseOpen => (-delta, PositionState::Child), // σ_p = −δ_g（反父方向，GPT AgainstParent）
    };
    // G3 恒等式护栏（§6 ℓ=e+Ndepth，Nest 通道）：链顶 ℓ 与深度同时给出时必须自洽。
    if let (Some(ol), Some(d)) = (ext.origin_level, ext.nest_depth) {
        debug_assert_eq!(ol, c.level + d as u32, "origin_level ≠ level + nest_depth（区间套链恒等式破）");
    }
    // H 轴（codex #81 `h_axis_in_canonical_z: accept`）：真候选带 role.h ⟹ 填 Some(h)，升 canonical z
    // 到完整 R(g)=(H,V,δ)。from_certificate 只填 V 投影 + horizontal=None，此处 struct-update 覆盖 H。
    // force_state 第 8 维（A6 #159）：从 c.force（Candidate 透传 BspPoint.force）经 divergence.rs
    // 唯一支配序原语 ForceProxies::force_state() 填——旧 z_of_candidate_with_force 分叉已删：
    // Candidate 携 force 后「平装版静默 None」在类型层不可构造（G2 tower 签名强制同款纪律）。
    // σ_higher 第 9 维（codex-q1 G2）：塔真值 Some(v)——v=0 是计算结果（上级持平/无上级），非未知。
    // G3 第 10-13 维：ext 显式装配；origin_level 无链覆盖 ⟹ Some(c.level)（起始=执行真值）。
    // #149 第 14 维 t_stage：ext 显式装配（runner π 路径 Some(tw.stage)，统计层 None）。
    // #175 第 15 维 eta_bucket：ext 显式装配（runner π 路径 Some(γ_t 四桶)，统计层 None）。
    MuClass {
        horizontal: Some(c.role.h),
        force_state: c.force.map(|f| f.force_state()),
        sigma_higher: Some(sigma_higher_at(tower, bars, c.level as usize)),
        cand_channel: ext.cand_channel,
        nest_depth: ext.nest_depth,
        origin_level: Some(ext.origin_level.unwrap_or(c.level)),
        risk_mode: ext.risk_mode,
        t_stage: ext.t_stage,
        eta_bucket: ext.eta_bucket,
        ..MuClass::from_certificate(c.level, delta, c.bits, parent_dir, position)
    }
}

/// 出场时刻 z 快照 `exit_z`（A7 #165，《完整的策略.pdf》§6 z「两次快照」+ §9 typed exit）。
///
/// **命题（PDF §6/§9，构造被迫非裁量）**：一笔持仓在其生命期内是**同一结构实体**——入场时
/// 冻结的结构身份维（`level`/`delta`/`i_class`/`parent_dir`/`horizontal`/`force_state`/
/// `sigma_higher`/`cand_channel`/`nest_depth`/`origin_level`）**不随 bar 演化重采样**（PDF §6
/// 明文 `σ_higher: 入场时上级方向`——按定义是入场值；力度/H 由开仓 `Candidate` 派生，入场固定）。
/// [`MuClass`] 中**唯一逐 bar 变化**的维是账本态三元 `{t_stage, eta_bucket, risk_mode}`（[`ZExt`]
/// 承载的账本相位）。故「同一 z 在出场时刻的快照」= 入场 z 的结构维 **+** 出场 bar 的账本态维。
///
/// 在出场处**重新分类结构维**需要一个不存在的出场候选（多数 typed exit——CloseRoot/RiskExit/
/// censored Hold——无触发候选），伪造它 = 声明膨胀（231号）。故本构造**被迫唯一**（定理，非裁量）：
/// 精确刷新账本三元维，其余 struct-update 自 `entry_z` 承继。
///
/// `exit_ext` = 出场 bar 决策点的 [`ZExt`]（runner π 路径 = 该 bar 的 `ext_i`，与 entry_z 在同 bar
/// 入场时的账本装配点同口径）。裸/统计层路径传 [`ZExt::NONE`] ⟹ 账本三元维诚实 None（同 entry_z）。
///
/// ★消费侧边界（A7 裁量分离，team-lead 令）：本函数只**生产** `exit_z` 入 [`TypedTrade`](
/// super::runner::TypedTrade) 账本列。μ 估计器**按 `(z_entry, z_exit, exit_type)` 分桶的语义是设计
/// 裁量**（多合理方案，需价值判断）——**未在此实装**，待 codex 裁决（no-unnecessary-escalation
/// 选择类）。当前 μ 层仍按 `entry_z` 逐笔独立观测（[`build_mu_from_bars`](super::l3_delta_r_alpha)
/// 不消费 `exit_z`），本字段是出场侧完备性的账本层载体（同 `position_node_id` A9 先例）。
pub fn exit_z_of(entry_z: MuClass, exit_ext: &ZExt) -> MuClass {
    MuClass {
        risk_mode: exit_ext.risk_mode,
        t_stage: exit_ext.t_stage,
        eta_bucket: exit_ext.eta_bucket,
        ..entry_z
    }
}

/// χ_t 候选集过滤（§13 line 2256）：`Γ_t → Γ_t^trade = {γ∈Γ_t : χ_t(γ)=1}`。
///
/// **诚实有效域声明（no-patch / no-claim-inflation）**：本函数只施加 χ_t 的 **μ>θ** 项——
/// RiskOK/ConflictOK 在下游**已存在**（不重造，复用既有机制）：
/// - **RiskOK**：`k_theta_risk_gate` 产 `KThetaRiskGate`（force_flat/stop）收窄 𝒦_Θ——风险否决在
///   `pi_theta_position` 兑现，**非**本过滤层。
/// - **ConflictOK**：[`interpret`](super::super::strategy::interp::interpret)（规则2/3/4）把冲突/重复
///   候选归 𝒦_x 记录桶（不开仓）——冲突唯一裁决在解释器兑现，**非**本过滤层。
///
/// 故 χ_t(γ)=1[μ>θ ∧ RiskOK ∧ ConflictOK] = **本过滤(μ>θ)** ∘ **gate(RiskOK)** ∘ **interpret(ConflictOK)**
/// 三机制合取兑现完整 χ_t；本函数兑现其中 μ 门（§13 唯一**新增**项，task #41 增量）。
///
/// **非方向候选（`dir==Flat` / `bsp_class==u8::MAX`）保留**——由 interpret 归 𝒦_x 记录（不执行），
/// 在此不滤（χ 是开仓边际门，方向裁决是 interpret 职责，no-patch 不越界）。
///
/// `z_alpha`：单边置信分位（如 1.645=95%）——准入量用 **LCB(μ)=mean−z_alpha·std/√n**（严格alpha.pdf
/// p25 §12 防高维 z 过拟合），非裸 μ。`z_alpha=0` ⟹ LCB=mean ⟹ **n≥2 类**退化回裸 μ 门；
/// **n=1 类例外**（mu_lcb 返 None ⟹ 走 treat_empty_as_pass，非裸 μ——n=1 无方差=无 LCB 证据，拒绝
/// 是 p25 正确语义，no-patch §5 防声明膨胀）。
///
/// `treat_empty_as_pass`：无 LCB 证据（μ=None 空类 **或** n<2 单样本，mu_lcb 皆 None）χ 取值
/// （codex Q3）：`false` ⟹ χ=0 不交易（**最诚实**——无正边际证据不开，p25 一致）；`true` ⟹ 默认
/// 交易（探索/全覆盖）。LCB 升级后 None 多了「n<2 方差未定义」一源——与空类合流为「无证据」。
///
/// 认识论 **L1**（给定 μ/std 表/θ/z_alpha 过滤候选是确定性变换，验证选择器逻辑生效，零信息增量）——
/// 过滤是否提升 alpha 是 **L2/L3**（需 walk-forward μ + 真实数据否证，下游 delta-r-alpha 工位）。
pub fn filter_gamma(
    gamma: &[Candidate],
    est: &MuEstimator,
    theta: f64,
    z_alpha: f64,
    treat_empty_as_pass: bool,
    tower: &[Rc<Vec<LeveledMove>>],
    bars: &[Bar],
    ext: &ZExt,
) -> Vec<Candidate> {
    filter_gamma_with_admission(
        gamma, est, theta, z_alpha, None, treat_empty_as_pass, tower, bars, ext,
    )
}

/// 准入量泛化版（acc-three-way-l2 #83）：`shrink_tau_sq=None` ⟹ 准入量 = LCB(μ)（与 [`filter_gamma`]
/// bit-exact，z_alpha=0 退化裸 μ）；`shrink_tau_sq=Some(τ²)` ⟹ 准入量 = [`MuEstimator::mu_shrink`]
/// 层级收缩（z_alpha 此时忽略——shrink 不用置信下界，用 pooled 收缩抗稀疏过拟合；§8 收缩 vs §12
/// LCB 是两套机制）。三路对比唯一切换点：裸μ(None,z_alpha=0)/LCB(None,z_alpha>0)/shrink(Some)。
///
/// None 语义统一不变：mu_shrink 对无 pooled 样本的 (level,delta) 返 None ⟹ 走 treat_empty_as_pass。
/// 与 mu_lcb 的 n<2→None 关键区别：mu_shrink 对 n<2 类**完全收缩到 pooled**（保功效，非拒绝）⟹
/// 高级别稀疏类不被压退化空仓（#83 卖点判据：n_L3 不下降）。
pub fn filter_gamma_with_admission(
    gamma: &[Candidate],
    est: &MuEstimator,
    theta: f64,
    z_alpha: f64,
    shrink_tau_sq: Option<f64>,
    treat_empty_as_pass: bool,
    tower: &[Rc<Vec<LeveledMove>>],
    bars: &[Bar],
    ext: &ZExt,
) -> Vec<Candidate> {
    gamma
        .iter()
        .filter(|c| {
            // 非方向候选不滤（interpret 归 𝒦_x；χ 不越界做方向裁决）。
            if c.dir == VoiceSide::Flat || c.bsp_class == u8::MAX {
                return true;
            }
            // 方向候选：准入量>θ 门（RiskOK/ConflictOK 下游已施，此处仅 μ 项 risk_ok=conflict_ok=true）。
            // σ_higher 真值查询（codex-q1 G2 护航点）：与训练表同经 z_of_candidate 取塔真值——
            // 训练 Some(v)/查询 Some(v) 同口径，无静默"未见类别"退化。
            // G3 ext（#138）：调用方传入与训练侧同现场的 ZExt（runner fill loop bar 级账本态）。
            let z = z_of_candidate(c, tower, bars, ext);
            let admission = match shrink_tau_sq {
                Some(tau_sq) => est.mu_shrink(&z, tau_sq),
                None => est.mu_lcb(&z, z_alpha),
            };
            chi_t(admission, theta, true, true, treat_empty_as_pass)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::backtest::mu_estimator::{MuObservation, PositionState};
    use crate::theta_v0::types::BspBits;

    fn buy_z() -> MuClass {
        MuClass::from_certificate(
            3,
            1,
            BspBits { buy1: true, ..Default::default() },
            0,
            PositionState::Root,
        )
    }

    /// χ_t = μ>θ ∧ RiskOK ∧ ConflictOK（§13 line 2239）——三合取，任一假 ⟹ χ=0。
    #[test]
    fn chi_is_conjunction_of_three() {
        // μ=10>θ=5, RiskOK, ConflictOK ⟹ χ=1
        assert!(chi_t(Some(10.0), 5.0, true, true, false));
        // μ=10>θ 但 RiskOK=false ⟹ χ=0（风险否决）
        assert!(!chi_t(Some(10.0), 5.0, false, true, false));
        // μ=10>θ 但 ConflictOK=false ⟹ χ=0（冲突否决）
        assert!(!chi_t(Some(10.0), 5.0, true, false, false));
        // μ=3 ≤ θ=5 ⟹ χ=0（边际收益不足）
        assert!(!chi_t(Some(3.0), 5.0, true, true, false));
    }

    /// θ=0 退化：μ>0 ⟹ 交易，μ≤0 ⟹ 不交易（§13 θ≥0 下界，θ=0 = 只滤负期望）。
    /// **注意**：θ=0 **不**等于全覆盖（χ≡1）——全覆盖需对空类 pass 且对负 μ 也 pass。
    /// θ=0 仍滤掉 μ≤0 的已观测类。全覆盖退化见 `theta_neg_inf_is_full_coverage`。
    #[test]
    fn theta_zero_filters_nonpositive_mu() {
        assert!(chi_t(Some(0.001), 0.0, true, true, false)); // μ>0 ⟹ 交易
        assert!(!chi_t(Some(0.0), 0.0, true, true, false)); // μ=0 不 >0 ⟹ 不交易
        assert!(!chi_t(Some(-5.0), 0.0, true, true, false)); // μ<0 ⟹ 不交易
    }

    /// θ→−∞（用 f64::NEG_INFINITY）+ 空类 pass ⟹ χ≡1 全覆盖退化（§13 边界：所有 γ 都交易）。
    /// 这是验收 acc-chi-theta-filter 的「θ=0 退化为全覆盖」边界的精确形式：全覆盖 = θ 低到任何
    /// 已观测 μ 都过 + 空类也过。证明选择器在极限下还原 χ≡1（与 pi_bsp_timing Pass 1 一致）。
    #[test]
    fn theta_neg_inf_is_full_coverage() {
        let est = MuEstimator::new();
        let z = buy_z();
        // 空类 + treat_empty_as_pass=true ⟹ 即使无 μ 也开（全覆盖）。
        assert!(chi_open_gate(&est, &z, f64::NEG_INFINITY, true, true, true));
        // 已观测负 μ + θ=−∞ ⟹ μ > −∞ 恒真 ⟹ 开（全覆盖不滤任何已观测类）。
        let mut est2 = MuEstimator::new();
        est2.observe(MuObservation { class: z, x_gamma: -999.0 });
        assert!(chi_open_gate(&est2, &z, f64::NEG_INFINITY, true, true, true));
    }

    /// 空类语义二分：treat_empty_as_pass 控制未观测 z 的 χ（§13 未明确的边界，二者皆合法）。
    #[test]
    fn empty_class_semantics_split() {
        let est = MuEstimator::new(); // z 无样本
        let z = buy_z();
        // 全覆盖语义：未观测默认交易。
        assert!(chi_open_gate(&est, &z, 0.0, true, true, true));
        // 严格语义：无 μ 证据 ⟹ 不交易（保守）。
        assert!(!chi_open_gate(&est, &z, 0.0, true, true, false));
    }

    /// 空类**不**伪造 μ=0：chi_t 收到 None 走 treat_empty 分支，不当作 μ=0 比较（认识状态区分）。
    #[test]
    fn empty_class_not_treated_as_mu_zero() {
        // 若空类被错误当 μ=0：θ=−1 时 0>−1 ⟹ 会 pass。我们用 treat_empty_as_pass=false 验证
        // 空类走的是 pass 分支（false）而非 μ=0 比较（那样会 true）⟹ 证明无伪造。
        assert!(!chi_t(None, -1.0, true, true, false));
        // 对照：真 μ=0 在 θ=−1 时确实 pass（0>−1）——证明区别确实存在。
        assert!(chi_t(Some(0.0), -1.0, true, true, false));
    }

    /// π̄ argmax：多候选选 μ 最大者（§17 line 3524）。
    #[test]
    fn pi_bar_picks_max_mu_above_theta() {
        let actions = [("buy", 3.0), ("short", 8.0), ("hold", 0.0)];
        assert_eq!(pi_bar(&actions, 1.0), Some("short")); // μ=8 最大且>θ=1
    }

    /// π̄ 全 μ≤θ ⟹ Hold/Flat（None，§17 line 3531 全定义）。
    #[test]
    fn pi_bar_returns_hold_when_all_below_theta() {
        let actions = [("buy", 1.0), ("short", 2.0)];
        assert_eq!(pi_bar(&actions, 5.0), None); // 无候选 μ>θ=5 ⟹ Hold/Flat
        // 空候选集 ⟹ Hold/Flat（§17 每个 z 都有动作，最坏 Hold）。
        assert_eq!(pi_bar::<&str>(&[], 0.0), None);
    }

    /// LCB 门控分离（严格alpha.pdf p25 §12 核心可证伪）：高方差低 n 类，裸 μ>θ 但 LCB<θ ⟹
    /// LCB 选择器**拒绝**、裸 μ 选择器**准入**。二者决策分离 = LCB 升级有信息增量（非同义反复）。
    #[test]
    fn lcb_gate_rejects_high_variance_that_naive_mu_admits() {
        let z = buy_z();
        let theta = 5.0;
        let z_alpha = 1.645; // 95% 单边
        let mut est = MuEstimator::new();
        // 两样本 [110, -80]：mean=15>θ=5（裸 μ 准入），但 std≈134 极大 ⟹ LCB=15−1.645·134/√2≈−141<θ。
        est.observe(MuObservation { class: z, x_gamma: 110.0 });
        est.observe(MuObservation { class: z, x_gamma: -80.0 });
        let mu = est.mu(&z).unwrap();
        let lcb = est.mu_lcb(&z, z_alpha).unwrap();
        assert!(mu > theta, "裸 μ={mu} 应 >θ={theta}（裸门准入）");
        assert!(lcb < theta, "LCB={lcb} 应 <θ={theta}（高方差收缩拒绝）");
        // 裸 μ 门：准入。LCB 门：拒绝。决策分离（可证伪）。
        assert!(chi_open_gate(&est, &z, theta, true, true, false));
        assert!(!chi_open_gate_lcb(&est, &z, theta, z_alpha, true, true, false));
        // filter_gamma 同路：z_alpha=0（裸 μ）准入 vs z_alpha=1.645（LCB）拒绝——但 filter_gamma 走
        // Candidate，此处直接验便利包装已足（z_of_candidate 桥接由 chi_is_conjunction 等覆盖）。
    }

    /// n→∞ 收敛（mu_estimator 文档 std/√n→0）：大样本低方差 z，LCB 选择器与裸 μ 选择器决策一致。
    /// LCB→mean ⟹ LCB 门退化回裸 μ 门（高样本时置信下界不再收缩）。
    #[test]
    fn lcb_converges_to_naive_mu_at_large_n() {
        let z = buy_z();
        let theta = 5.0;
        let z_alpha = 1.645;
        let mut est = MuEstimator::new();
        // 大样本（n=2000）窄分布（围绕 10±0.5）⟹ std 小、√n 大 ⟹ std/√n→0 ⟹ LCB→mean≈10>θ。
        for i in 0..2000 {
            let x = if i % 2 == 0 { 10.5 } else { 9.5 }; // mean=10, 小方差
            est.observe(MuObservation { class: z, x_gamma: x });
        }
        let mu = est.mu(&z).unwrap();
        let lcb = est.mu_lcb(&z, z_alpha).unwrap();
        assert!((mu - lcb).abs() < 0.1, "大样本 LCB({lcb}) 应 ≈ mean({mu})");
        // 两门决策一致（都准入，因 LCB≈mean≈10>θ=5）。
        assert_eq!(
            chi_open_gate(&est, &z, theta, true, true, false),
            chi_open_gate_lcb(&est, &z, theta, z_alpha, true, true, false),
            "大样本下 LCB 门与裸 μ 门决策一致（收敛）"
        );
    }

    /// θ=−∞ 退化全覆盖在 LCB 门下不变：LCB>−∞ 对任何有 LCB（n≥2）的类恒真；None（n<2/空类）
    /// 仍按 treat_empty_as_pass。证明 LCB 升级保留 §13 全覆盖退化边界。
    #[test]
    fn lcb_theta_neg_inf_is_full_coverage() {
        let z = buy_z();
        let z_alpha = 1.645;
        // n≥2 高方差类：θ=−∞ ⟹ 即使 LCB 极负也 >−∞ ⟹ 准入（全覆盖不滤已观测 n≥2 类）。
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z, x_gamma: 110.0 });
        est.observe(MuObservation { class: z, x_gamma: -80.0 });
        assert!(chi_open_gate_lcb(&est, &z, f64::NEG_INFINITY, z_alpha, true, true, true));
        // None 类（空类）：θ=−∞ 不改 treat_empty_as_pass 裁决——true 放行，false 拒绝。
        let empty = MuEstimator::new();
        assert!(chi_open_gate_lcb(&empty, &z, f64::NEG_INFINITY, z_alpha, true, true, true));
        assert!(!chi_open_gate_lcb(&empty, &z, f64::NEG_INFINITY, z_alpha, true, true, false));
    }

    /// None 语义统一（任务 §3）：n<2 单样本（mu_lcb=None）与空类同走 treat_empty_as_pass，
    /// 不冒充 LCB=mean。无 LCB 证据时 false=不交易（p25「无正边际收益证据时不交易」）。
    #[test]
    fn lcb_single_sample_none_follows_empty_semantics() {
        let z = buy_z();
        let z_alpha = 1.645;
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z, x_gamma: 100.0 }); // n=1 ⟹ 方差未定义 ⟹ mu_lcb=None
        assert_eq!(est.mu_lcb(&z, z_alpha), None, "n=1 ⟹ mu_lcb None");
        assert_eq!(est.mu(&z), Some(100.0), "但裸 μ 有值（n=1 均值已定义）");
        // 无 LCB 证据：false ⟹ 不交易（诚实），true ⟹ 全覆盖放行——与空类同。
        assert!(!chi_open_gate_lcb(&est, &z, 0.0, z_alpha, true, true, false));
        assert!(chi_open_gate_lcb(&est, &z, 0.0, z_alpha, true, true, true));
    }

    /// G3（#138）ext 装配：四维经 ZExt 透传进 z；origin_level 无链覆盖 ⟹ Some(c.level)
    /// （起始=执行真值）；ZExt::NONE ⟹ 门/账本维 None（诚实口径）。
    #[test]
    fn g3_ext_dims_assembled_into_z() {
        use crate::theta_v0::backtest::econ_positive::NestTrigger;
        use crate::theta_v0::strategy::coverage::{Dir, GradeRel, Horizontal, OperationRole, Vertical};
        use crate::theta_v0::strategy::risk::RiskMode;

        let c = Candidate {
            level: 2,
            source_index: 5,
            bits: BspBits { buy2: true, ..Default::default() },
            dir: VoiceSide::Long,
            bsp_class: 2,
            role: OperationRole { h: Horizontal::First, v: Vertical::Ambient, delta: Dir::Plus, grade: GradeRel::SameLevel },
            nest_confirmed: false,
            gamma_index: 0,
            force: None,
        };
        // 无扩展源口径：门/账本维 None，origin_level=Some(level)（ℓ=e 真值非 None）。
        let z0 = z_of_candidate(&c, &[], &[], &ZExt::NONE);
        assert_eq!(z0.cand_channel, None);
        assert_eq!(z0.nest_depth, None);
        assert_eq!(z0.origin_level, Some(2), "无链覆盖 ⟹ 起始=执行级（真值）");
        assert_eq!(z0.risk_mode, None);
        assert_eq!(z0.t_stage, None, "#149：无 TW 账本口径 ⟹ t_stage 诚实 None");
        assert_eq!(z0.eta_bucket, None, "#175：无 TW 账本口径 ⟹ eta_bucket 诚实 None");
        // Nest 门口径：扩展维真值透传 + ℓ=e+depth 恒等式（debug_assert 同款自洽输入）。
        let ext = ZExt {
            cand_channel: Some(NestTrigger::Type23SublevelType1),
            nest_depth: Some(1),
            origin_level: Some(3), // = level 2 + depth 1
            risk_mode: Some(RiskMode::Normal),
            t_stage: Some(crate::theta_v0::strategy::ledger::TStage::CapitalRecovered),
            eta_bucket: Some(EtaBucket::PositiveUnsafe),
        };
        let z1 = z_of_candidate(&c, &[], &[], &ext);
        assert_eq!(z1.cand_channel, Some(NestTrigger::Type23SublevelType1));
        assert_eq!(z1.nest_depth, Some(1));
        assert_eq!(z1.origin_level, Some(3));
        assert_eq!(z1.risk_mode, Some(RiskMode::Normal));
        assert_eq!(
            z1.t_stage,
            Some(crate::theta_v0::strategy::ledger::TStage::CapitalRecovered),
            "#149：t_stage 经 ZExt 透传进 z"
        );
        assert_eq!(
            z1.eta_bucket,
            Some(EtaBucket::PositiveUnsafe),
            "#175：eta_bucket 经 ZExt 透传进 z"
        );
        // 形态维（1-9）不受 ext 影响（新维正交于形态维）。
        assert_eq!((z0.level, z0.delta, z0.i_class, z0.horizontal), (z1.level, z1.delta, z1.i_class, z1.horizontal));
    }

    /// A6（#159）force 透传：z_of_candidate 从 c.force 经唯一支配序原语 ForceProxies::force_state()
    /// 填第 8 维；c.force=None ⟹ force_state=None（诚实）。「透传断裂 ⟹ 恒 None」在此被单测封死。
    #[test]
    fn a6_force_state_assembled_from_candidate_force() {
        use crate::theta_v0::classifier::divergence::{ForceFeatures, ForceProxies, ForceStateA5};
        use crate::theta_v0::strategy::coverage::{Dir, GradeRel, Horizontal, OperationRole, Vertical};

        let ff = |s: f64| ForceFeatures {
            macd_area: 10.0 * s,
            dif_peak: 2.0 * s,
            price_amplitude: (100.0 * s) as i64,
            price_speed: 5.0 * s,
            tv: (150.0 * s) as i64,
        };
        let mut c = Candidate {
            level: 0,
            source_index: 3,
            bits: BspBits { buy1: true, ..Default::default() },
            dir: VoiceSide::Long,
            bsp_class: 1,
            role: OperationRole { h: Horizontal::First, v: Vertical::Ambient, delta: Dir::Plus, grade: GradeRel::SameLevel },
            nest_confirmed: false,
            gamma_index: 0,
            force: Some(ForceProxies { seg_a: ff(1.0), seg_c: ff(0.5) }), // C 全 5 proxy < A ⟹ Dominated（背驰）
        };
        assert_eq!(
            z_of_candidate(&c, &[], &[], &ZExt::NONE).force_state,
            Some(ForceStateA5::Dominated),
            "A6：c.force 经唯一支配序原语装配进 force_state 第 8 维"
        );
        c.force = None;
        assert_eq!(
            z_of_candidate(&c, &[], &[], &ZExt::NONE).force_state,
            None,
            "无 A/C 力度对 ⟹ force_state 诚实 None"
        );
    }

    /// G3（#138）UClass 降维不读新维（约束②：新维只进 canonical 不进 selection 桶键——
    /// 抗碎裂 + i_class×δ 共线教训）：仅新维不同的两个 z 映同一 u。
    #[test]
    fn g3_project_to_u_ignores_new_dims() {
        use crate::theta_v0::backtest::econ_positive::NestTrigger;
        use crate::theta_v0::backtest::mu_estimator::UClass;
        use crate::theta_v0::strategy::risk::RiskMode;

        let base = buy_z();
        let decorated = MuClass {
            cand_channel: Some(NestTrigger::XiaoZhuanDa),
            nest_depth: Some(2),
            origin_level: Some(5),
            risk_mode: Some(RiskMode::Deleverage),
            t_stage: Some(crate::theta_v0::strategy::ledger::TStage::EarningShares), // #149 同约束②
            eta_bucket: Some(EtaBucket::Deficit), // #175 同约束②
            ..base
        };
        assert_eq!(UClass::project_to_u(&base), UClass::project_to_u(&decorated), "ϕ:Z→U 折叠 G3/#149/#175 新维");
    }

    /// z_alpha=0 退化（向后兼容）：LCB=mean−0=mean ⟹ LCB 门 ≡ 裸 μ 门（filter_gamma/runner
    /// 默认 z_alpha=0 保 frozen bit-exact）。
    #[test]
    fn lcb_z_alpha_zero_equals_naive_mu() {
        let z = buy_z();
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z, x_gamma: 10.0 });
        est.observe(MuObservation { class: z, x_gamma: 8.0 }); // n=2，LCB 有定义
        // z_alpha=0 ⟹ LCB=mean=9，与裸 μ 门同决策。
        assert_eq!(est.mu_lcb(&z, 0.0), est.mu(&z));
        for theta in [-1.0, 8.5, 9.0, 100.0] {
            assert_eq!(
                chi_open_gate(&est, &z, theta, true, true, false),
                chi_open_gate_lcb(&est, &z, theta, 0.0, true, true, false),
                "z_alpha=0 时 LCB 门 ≡ 裸 μ 门（θ={theta}）"
            );
        }
    }
}
