//! 回测管线串联——data → 引擎(parse→classify→π_Θ 七链/plan_orders) → fill → 权益曲线 → metrics。
//!
//! ## 现状订正（#744：此前描述的「阻塞点 A/B」已解除，不再是当前架构）
//!
//! **已退役的是 v1 *runner***（`run_theta_v0`/`plan_and_fill_mtm`，每 bsp 一
//! `VoiceDecision`，离散择时；#499 退役，全窗 L3 8/8 已否证）——`classifier::classify`
//! 与 `strategy::recognize` 本身**均存活**：`classify` 是 π 生产路径的分类前端
//! （`IncrementalClassifier::classify_at` 逐 bar 调）；`recognize` 仍是存活公开 API
//! （`StrategyFamily::pi` strategy/mod.rs:1070 在调）。**生产路径**是
//! [`run_theta_v0_pi`]/`run_theta_v0_pi_chi`/`run_theta_v0_pi_chi_shrink`——七链 π_Θ
//! 引擎（`coverage::pi_theta_step`），只是生产 π 路径不经 `recognize`，per-bar 三适配器
//! 驱动订单产出，详见该函数组的 doc。
//!
//! ## 认识论等级（formalization-validity-domain 231号，对生产路径仍成立）
//!
//! - 管线串联 + fill 模拟本身 = **L1**（验证管线串通，零信息增量）。
//! - 喂真实历史数据跑 frozen Θ 产出的指标 = **L2**（可否证）——产出的 trades 须再经
//!   L2/L3 否证检验（下个工位，见 [`run_theta_v0_pi`] doc 的"下游推论"）。
//!
//! ## fill 模拟（Θ_exec，reference-theta-v0.md:49-54）
//!
//! 订单 [`Order`] 携带 `exec_index`（执行延迟后的成交 bar）、`action`、`qty`。
//! **生产 fill** = [`pi_theta_fill_loop`]（fill.rs:718）：per-bar 驱动 `apply_order` 推进
//! NAV 轨迹。[`simulate_fills`] 生产零调用，现仅测试消费（迁出后唯二调用点在
//! runner_tests.rs）；strategy 另有独立的 `exec` 子模块（延迟/费用/止损成交/冲突排序，
//! 对齐 Θ_exec，决定"成交价/方向"）。

use super::super::closed_loop::state::{AssemblyState, MicroEvent};
use super::super::closed_loop::transition::{hybrid_step, AssemblyEvent};
use super::super::config::ThetaConfig;
use super::super::strategy::ledger::{RiskPolicy, TwState};
use super::super::types::{Bar, Order, StrictAction};
use super::super::{classifier, parser};
use super::data::Dataset;
pub(crate) use super::fill::{apply_order, FillOutcome};
use super::fill::{bar_returns, simulate_fills};
pub(super) use super::ledger::TwLedgerThread;
pub(super) use super::ledger::{track_position_transition, LedgerOpen, OpsemEntrySnapshot};
pub use super::ledger::{TypedTrade, VoiceVerdictRec, TYPED_TRADE_SCHEMA_VERSION};
use super::metrics::{self, Metrics};
pub use super::opsem_dump::{StrictNestCertificateRecord, StrictNestSidecarSummary};
use super::signal::{entry_structural_stop, newly_confirmed_step};
use std::rc::Rc;
// ★B-M3b-contract（#91）fill loop seam：fill loop 本体 + FillOutput(Family)
// 自 runner.rs 纯移动；对外经 `pub use` 门面保持原路径。
pub(super) use super::fill::{apply_voice_fill, pi_theta_fill_loop, pi_theta_fill_loop_overlay};
// ★#295：声部独立执行臂 wrapper 本体在 fill.rs 即 #[cfg(test)]（生产接入走
// run_theta_v0_pi_overlay 的 VOICE_EXEC=1 gate 直调 pi_theta_fill_loop_overlay）——
// re-export 同门控，bin（非 test）构建不引 test-only 符号。
#[cfg(test)]
pub(super) use super::fill::pi_theta_fill_loop_voice;
pub(crate) use super::opsem_dump::OtherwiseDomainSidecarSummary;
use super::opsem_dump::{
    eta_bucket_str, force_state_str, operation_role_str, risk_mode_str,
    strict_nest_sidecar_enabled, summarize_strict_nest_certificates, t_stage_str, voice_side_str,
    OpsemDump, StrictNestSidecarCollector,
};
use super::opsem_dump::{otherwise_domain_sidecar_enabled, OtherwiseDomainSidecarCollector};
// ★#295：OPSEM_DUMP_DIR_OVERRIDE 为 #[cfg(test)] thread_local 注入点（opsem_dump.rs:180，
// 消费面仅 tests::opsem_dump_env_gated_bit_exact）——import 同门控，同 VOICE_EXEC_OVERRIDE 惯例。
#[cfg(test)]
use super::opsem_dump::OPSEM_DUMP_DIR_OVERRIDE;
// ★B-M2（#89）准入门 seam：χ/nest/k_Θ 三门 + κ 解析自 runner.rs 纯移动。
use super::admission::{
    k_theta_risk_gate, kappa_policy_resolved, kappa_priority_resolve, nest_cert_gate_enabled,
    nest_gate_admit, voice_exec_gate, LevelFingerprint, NestChainGate, NestGateObs, NestGateStats,
};
// T3 (#172) 链类型（测试与并门派生消费）。
pub use super::admission::ChiFilterCtx;
use super::admission::{ChainGapKind, ChainLevelStatus, ChainVerdict};
// 测试经原路径访问 thread_local override（admission 内 pub(super) 可见）。
#[cfg(test)]
use super::admission::{NEST_CERT_GATE_OVERRIDE, VOICE_EXEC_OVERRIDE};
// ★三方合并 ours 独有面（#196-#200/#209/#237 探针/判据/override，随 fill loop 同迁
// fill.rs；pub(super) 升格后本行恢复 runner 原路径，既有测试零改动消费）。
#[cfg(test)]
use super::fill::{
    residual_correction_probe_count, residual_correction_probe_reset,
    reverse_open_isolation_probe_count, reverse_open_isolation_probe_reset, silent_drop_exit_type,
    t1_core_residual_probe_count, t1_core_zero_probe_count, t1_core_zero_probe_reset,
    type2_sell_guard_probe_count, type2_sell_guard_probe_reset,
    type2_with_core_residual_probe_count, SHADOW_DIVERGENCE_PATH_OVERRIDE,
};

/// 单次回测的完整输出（指标 + 诊断量 + 闭环证据）。
#[derive(Debug, Clone)]
pub struct RunResult {
    pub symbol: String,
    pub metrics: Metrics,
    /// 输入 bar 数（窗内）。
    pub n_bars: usize,
    /// 引擎产出的订单数（非 0 时才是 L2 回测；recognize 接通后取决于真实结构）。
    pub n_orders: usize,
    /// 不可交易 bar 占比（§5.5：>20% 判 inconclusive）。
    pub untradable_ratio: f64,
    /// 是否 L2（true ⟺ 产出非空订单流且真实数据）。
    pub is_l2: bool,
    /// ★闭环终态证据（task #94 引擎实装）：bar 闭环驱动 [`run_closed_loop`] 的终态。
    /// 证明 account+twState 每 bar 真更新喂回（非开环单帧构造一次）——见
    /// `closed_loop_threads_every_bar` 见证。`None` 仅当 bars 为空。
    pub closed_loop_final: Option<AssemblyState>,
    /// ★每笔完整平仓交易的**已实现成交盈亏**（**不含**窗口终点强平的浮盈）——§3.4 block
    /// bootstrap（H0:收益≤0）的输入，**已实现口径**。区别于含浮盈口径（[`trade_pnls_with_forced`]）。
    /// 真实数据回测时这是 L2 否证/确认的数据基础（区别于 `metrics.strat_return` 的 MtM 口径）。
    pub trade_pnls: Vec<f64>,
    /// ★每笔交易盈亏的**含浮盈口径**（窗口终点对未平仓持仓强制平仓，实现所有浮盈）。
    /// 编排者铁律「不把浮盈算上不合理」——对「少平仓长持有」策略，持仓期 MtM 浮盈是择时载体，
    /// 剔除它=系统性抹掉。已实现口径（[`trade_pnls`]）与含浮盈口径**并列诚实报告**（231号）。
    pub trade_pnls_with_forced: Vec<f64>,
    /// ★交易执行轨迹（[`metrics::TradeRecord`]）——操作语义随机入场对照（§4 重写）的输入。
    /// 含 entry_bar/hold_bars/qty/forced_close，随机对照保操作外形随机化入场点检验择时 alpha。
    pub trades: Vec<metrics::TradeRecord>,
    /// 原始价格序列（close 口径，与账本侧 `apply_order` 成交价一致）——随机对照在其上重执行。
    pub prices: Vec<f64>,
    /// 单边费用率（commission+slippage+tax 比率）——随机对照含同等成本。
    pub fee_rate: f64,
    /// Θ MtM 复利口径 total_return（= `metrics.strat_return`）——**仅作 significance 报告参考**
    /// （`theta_return_mtm`），**不是**随机对照比较基准。实际比较用 significance 内部算的
    /// `theta_return_same_caliber`（逐笔无复利同口径）——消除复利偏置（codex 实现审查缺陷①②）。
    pub theta_return_mtm: f64,
    /// 逐 bar（聚合前）**百分比** returns 序列（`w[1]/w[0]−1`）——Sharpe 标准误（Lo 2002）与
    /// 显著性检验的输入。**口径警告**：除数是前一 bar 权益 E_{t−1}（非 nav0），两条 NAV 路径发散
    /// 时百分比配对差 ≠ ΔR/nav0（§10 线性可加要求绝对增量 ÷nav0）——ΔR 净额增量检验须用
    /// [`equity_curve`]（归一化绝对权益）算逐 bar 绝对增量，不用本字段（delta-r-audit P0 修复）。
    pub daily_returns: Vec<f64>,
    /// 逐 bar **归一化绝对权益**序列 `E_t=(cash_t+units_t·P_t)/nav0`（已÷nav0）——§10 ΔR 净额增量
    /// 检验的口径正确输入。两条独立回测的绝对增量配对差 `(E^χ_t−E^χ_{t−1})−(E^0_t−E^0_{t−1})`
    /// = ΔR_t/nav0（成本已扣进 equity，∴ ΔC 自动含入），**与 NAV 路径发散无关**（除数恒为 nav0，
    /// 非 path-dependent 的 E_{t−1}）。区别于 [`daily_returns`]（百分比收益，喂 metrics 算 Sharpe）。
    pub equity_curve: Vec<f64>,
    /// ★M6 R 分解表（路线.pdf p16 第十一关：R=ΣN_tΔP_t−Commission−Slippage−Funding−Borrow−
    /// LiquidationLoss + 守恒残差）。生产 π 路径（[`run_theta_v0_pi`] 系列）产出；旧 recognize
    /// 路径（已退役 v1 runner）为 `None`（诚实——R 分解只接生产 π fill loop）。
    pub r_decomp: Option<super::super::strategy::risk::RDecomposition>,
    /// 严格区间套证书 sidecar 汇总。默认 `None`；仅 `THETA_STRICT_NEST_SIDECAR=1/true/yes/on`
    /// 时在生产 π 重放同帧旁路产出，不参与订单、候选、风控、账本。
    pub strict_nest_sidecar: Option<StrictNestSidecarSummary>,
    /// D4（#606 S1）：37:18 否则域亚型记录 sidecar 汇总。默认 `None`；仅
    /// `THETA_OTHERWISE_DOMAIN_SIDECAR=1/true/yes/on` 时在生产 π 重放同帧旁路产出（同
    /// `strict_nest_sidecar` 先例，观测面，不参与订单、候选、风控、账本、任何 bit 判据）。
    pub(crate) otherwise_domain_sidecar: Option<OtherwiseDomainSidecarSummary>,
}

// run_theta_v0 已按 #499 裁定退役删除（deprecated F-01 前视；删除 commit 见票）。

// run_theta_v0_dual 已按 #499 裁定退役删除（deprecated F-01 前视；删除 commit 见票）。

/// ★**七链 π_Θ 生产 runner（塔导出桥 (iii) 布线段，接 Nautilus 核心实装）**——
/// 原与买卖点 v1 `recognize` 并行；v1 家族已由 #499 退役。
///
/// ## 与 v1 的正交（Q1 裁定）
///
/// 已退役 v1 runner 走 `strategy::recognize`（每 bsp 一 [`VoiceDecision`]，离散择时，全窗 L3 8/8 否证）。
/// 本 runner 走**全定义策略 π_Θ 七链**（`coverage::pi_theta_step`）：每 bar 解释器三桶 → 活动集
/// 递归 → 目标头寸 p̃ → LexArgmin 𝒦_Θ → 唯一订单（§16 单一决策出口）。**不改** `recognize`。
///
/// ## 三适配器（诊断 task adcc804058774e334 缺口解除）
///
/// - **[A] per-bar 前缀因果重分类 + 确认-bar 部署**（`classify_with_tower` + [`newly_confirmed_step`]）：
///   每 bar i 喂 **前缀** `classify_with_tower(parse_layer(bars[0..=i]))` 得当步**因果塔 + 因果分类**
///   （只用 ≤i 数据 → 因果，无 look-ahead）；再 [`newly_confirmed_step`] 取**本 bar 新确认买卖点**
///   （append-only diff vs seen，确认时点部署——非 `source_index==i` 切片，后者因买卖点回溯确认而恒空
///   ⟹ 零订单，见 [`newly_confirmed_step`]）。全窗 `classify` 非因果（>i 数据确认），执行层禁用（639）。
/// - **[B] base_units（U_ℓ）注入**：从当前 NAV 协变派生 `U_ℓ = NAV/px`（方案A，`cap=U_ℓ·γ̄`，
///   `coverage::feasible_net_cap` 已无量纲化）——净 lot 资本单位（可建名义手数）。
/// - **[C] per-bar 驱动 + 活动集台账**：thread `(prev_active: Vec<ActiveLeg>, p_t)`；`p_t` 取
///   [`apply_order`] fill 后的 `units`（净 lot）；订单经 [`apply_order`]（全 [`StrictAction`] + 先平
///   后开）成交。
///
/// ## 执行层 σ_p = 父容器方向（★639 settle，per-bar 因果塔）
///
/// 候选垂直关系 V 的父向 σ_{p(g)} = **父容器方向**（结构对象），从 per-bar 前缀因果塔经
/// [`coverage::attach_bsp_to_tree`]（host→真 Compose 父→父 `rmove_side`，封于
/// `interp::assemble_gamma_with_tower`）查得，**与是否持仓无关**。父=胚元∂（缺塔/host 是根）⟹
/// σ_p=0 ⟹ Ambient（去根化，非"未持仓"）。错口径"活动持仓父腿"（`parent_dir_from_active` /
/// `assemble_gamma_active_parent`）已删（639，no-patch 删不留 fallback）。Lean 同构落地
/// `Origin/ParentDirContainer.lean`（parentDirOfContainer 无持仓门控）。
///
/// ## close_pred 折 𝒦_Θ（Q2 裁定，非第二决策出口）
///
/// 风控（stop/risk）经 [`k_theta_risk_gate`]（复用 `exec::close_pred` 契约锚）→ [`KThetaRiskGate`]
/// → 收窄 `pi_theta_position` 的 𝒦_Θ（force_flat→{0}/止损→禁向）——退出走**唯一决策出口** p*，非
/// 独立 exit 订单。reverse_signal 走 interpret 𝒟_x（腿级单出口）。
///
/// ## 认识论 L0/L1（formalization-validity-domain 231号）
///
/// 管线串联 + fill = **L1**（管线正确性）。本 runner 产 trades 后须 L2/L3 否证（下个工位）——
/// 本工位**不声明 alpha**（L0/L1 结构非 alpha；真 Fugue + 231：σ_p 父只来自真 Compose 父容器，
/// 禁级别差/走势几何伪造）。
///
/// ★σ_p 来源 + §13 AncOK 持仓准入双机制（639，两正交机制均就位）：① σ_p **来源** = 父容器方向
/// （`assemble_gamma_with_tower` 从因果塔查，与持仓无关）；② **§13 AncOK 持仓准入**
/// （`coverage_step_from_buckets` 经 `prev_active` 对位真树元素 + `ancestor_close`）**未持父则剔除
/// ReverseOpen 子腿**——故本 runner **不开 naked 逆势仓**（639(c) 兑现）。两机制正交：σ_p 用因果塔
/// （结构对象），准入用持仓台账（A_t 父容器腿在场判据）。
///
/// > **结果包六要素**
/// > - **结论**：新增 `run_theta_v0_pi`——七链 π_Θ 引擎接入生产 runner，per-bar 三适配器
/// >   ([A]前缀因果重分类/[B]base_units/[C]thread) 驱动 `pi_theta_step`（σ_p=父容器方向，639）产订单
/// >   → `apply_order` fill → metrics。
/// > - **定义依据**：reference §13-§16（活动集递归 + π_Θ 单一决策出口）；MEMORY
/// >   coverage-engine-needs-tower-export-bridge（互斥全定义策略=买卖点入场+多级角色/嵌套对冲）。
/// >   输入特征：`classification.levels[ℓ].bsp` 满足 `source_index` 坐标 ⟹ [A] 切片每 bar 候选。
/// > - **边界条件**：① classify 产空 bsp（无缠论结构）⟹ 候选空 ⟹ 无订单（诚实退化，非 bug，
/// >   与退役 v1 退化契约同）。② 前缀塔仅 L0（tower.len()<2，host 是根）⟹ 候选父=∂ ⟹ Ambient（早 bar/
/// >   冷启动态，与持仓无关）。③ base_units 随 NAV 协变 ⟹ 净持仓目标随 NAV 标度（vol/equity 目标化
/// >   重平衡）；NAV≤0 ⟹ force_flat 门 ⟹ 𝒦_Θ={0}。④ 若 σ_p 改回全窗塔（>i 数据）或持仓父腿则违因果
/// >   或混淆 §7.2 与 §13——本 runner 严禁（639）。
/// > - **下游推论**：`run_theta_v0_pi` 产非空 trades ⟹ L2/L3 净额回测否证检验有输入（下个工位）；
/// >   接 Nautilus 时净额账本 `units` 直接对接 `apply_order`（StrictAction + 先平后开已处理）。
/// > - **谱系引用**：Q1（最小侵入并行路径，v1 保留）；Q2（close_pred 折 𝒦_Θ，非第二出口）；639（σ_p
/// >   = 父容器方向，删"活动父腿"错口径，per-bar 因果塔）；638（hostOf 附着=σ_p 来源）；547（主力锚
/// >   删除）；newchanlun-v1-fullwindow-l3-falsified（v1 否证，本 runner 是 element-coverage 兑现，alpha
/// >   待验）；trades-vs-closedloop-disjoint-paths。
/// > - **影响声明**：新增 runner.rs `run_theta_v0_pi`/[`pi_theta_fill_loop`]/[`newly_confirmed_step`]/
/// >   [`k_theta_risk_gate`]；复用 [`apply_order`]/[`track_position_transition`]/[`run_closed_loop`]/
/// >   `metrics::compute`/`coverage::pi_theta_step`；不改存活的 `recognize`。
pub fn run_theta_v0_pi(
    dataset: &Dataset,
    config: &ThetaConfig,
    years: f64,
    initial_nav: f64,
) -> RunResult {
    // χ≡1 全覆盖（默认入口，frozen Θ v0 bit-exact 不变）。χ 阈值过滤端到端入口见 [`run_theta_v0_pi_chi`]。
    run_theta_v0_pi_inner(dataset, config, years, initial_nav, None)
}

/// ★χ_t 阈值过滤端到端入口（task #41 + delta-r-alpha #42）——`run_theta_v0_pi` 接 χ 过滤的版本。
///
/// θ 取 `config.risk.chi_theta`（`None` ⟹ 退化为 [`run_theta_v0_pi`] χ≡1）；μ 表 `est` 由调用方提供
/// （其因果性由调用方负责并诚实标注：全窗 in-sample μ=泄漏 L1；walk-forward 增量 μ=L2/L3，#42）。
/// `treat_empty_as_pass`：空类（μ=None）χ 取值（codex Q3：false=不交易最诚实）。
pub fn run_theta_v0_pi_chi(
    dataset: &Dataset,
    config: &ThetaConfig,
    years: f64,
    initial_nav: f64,
    est: &super::mu_estimator::MuEstimator,
    treat_empty_as_pass: bool,
) -> RunResult {
    let chi = config.risk.chi_theta.map(|theta| ChiFilterCtx {
        est,
        theta,
        z_alpha: config.risk.chi_z_alpha,
        treat_empty_as_pass,
        shrink_tau_sq: None,
    });
    run_theta_v0_pi_inner(dataset, config, years, initial_nav, chi)
}

/// shrinkage 准入入口（acc-three-way-l2 #83）：与 [`run_theta_v0_pi_chi`] 同 harness，唯一区别
/// 准入量 = mu_shrink(z,τ²)（层级收缩抗稀疏）而非 LCB(μ)。`chi_theta=None` 时退化 χ≡1（同基线）。
pub fn run_theta_v0_pi_chi_shrink(
    dataset: &Dataset,
    config: &ThetaConfig,
    years: f64,
    initial_nav: f64,
    est: &super::mu_estimator::MuEstimator,
    tau_sq: f64,
    treat_empty_as_pass: bool,
) -> RunResult {
    let chi = config.risk.chi_theta.map(|theta| ChiFilterCtx {
        est,
        theta,
        z_alpha: config.risk.chi_z_alpha,
        treat_empty_as_pass,
        shrink_tau_sq: Some(tau_sq),
    });
    run_theta_v0_pi_inner(dataset, config, years, initial_nav, chi)
}

fn run_theta_v0_pi_inner(
    dataset: &Dataset,
    config: &ThetaConfig,
    years: f64,
    initial_nav: f64,
    chi: Option<ChiFilterCtx>,
) -> RunResult {
    let bars = &dataset.bars;

    // 闭环终态证据（与退役 v1 契约同——bar 闭环驱动）。
    let closed_loop_final = run_closed_loop(bars, initial_nav);

    // 步骤 3+4+5：七链 π_Θ per-bar 驱动 + fill（三适配器 [A][B][C] + 执行层父容器 σ_p + 风控门）。
    // ★[A] 执行层 σ_p = 父容器方向（639）：fill loop 每 bar i 经 `IncrementalClassifier::classify_at(i)`
    // 得**因果塔 + 因果分类**——只用 ≤i 数据（无 look-ahead）。全窗 classify（非因果，bsp/父走势
    // 可能用 >i 数据确认）执行层禁用，故此处不预算全窗分类（增量器逐 bar 前缀重分类）。
    //
    // ★增量塔接入（ad7319b9 + 本工位）：IncrementalClassifier 内部走真增量链——
    // ParseLayerIncr::append（inclusion O(1)/bar）+ classify_with_tower_incremental（TowerCache 跨 bar
    // 复用：LevelCache.upper_moves/centers/scan_cursor 持久 + MACD 增量递推）。
    //
    // ponytail: 增量塔身份稳定路径已被 L2 证伪（ab5f5a29d ΔSharpe 重测 0.000，Stale 90%+ 未降）。
    // 增量塔 TowerCache 跨 bar 复用 O(n)（bit-exact，见 incremental.rs 文档），但旧 held_leg_tree_index
    // 值字段比较（level/ρ/eps/λ）非对象身份——bit-exact 不变 ⟹ 值比较结果相同 ⟹ Stale 不降。
    // ★codex Q4 发现 B 归因修正：旧注释"父走势演化后 λ/eps 漂移"归因错误——λ=start_index 在
    // confirmed 前缀不回写时不变。真因 = extract_elements 每 bar 重建 Vec + 更高级新出现时根结构
    // 重构索引重映射 + 值比较非 spec §13 结构映射 p(g)。Q4 修复：确定性 ElementId 跨 bar 稳定
    //（全量/增量产同 ID），held_leg_tree_index 按 ID 匹配非值比较；Stale 不伪造 parent:None（发现 A），
    // 非边界根父未解析 = prune（AncOK 严格 §13）⟹ depth>0 腿可准入 ⟹ ΔSharpe 可非零（待 L2 重测）。
    //
    // **bit-exact 不变**：增量链 == 全量 classify_with_tower(parse_layer(..=i))（parser +
    // classifier 各自 bit-exact 已证，见 incremental.rs 文档）。逐 bar 断言见 `incremental::bit_exact_*`。
    // 注意：bit-exact 仅证明塔构造 O(n) 达成，不证明身份稳定→Stale 降根（后者被 L2 否证）。
    // ★T3 (#172 并门，#168 裁定 3)：层载由链路径单一驱动——链活（nest 证书门开）⟹ 投影层
    // 必载（含三元锚索引，恰好存在物化基座）；链死不载（零开销红线不死）。生产唯一派生点
    // （层门配置面退役；派生借/克隆零拷贝于门关路径）。
    let config = super::admission::chain_driven_level_projection(config);
    let mut classifier_incr = super::incremental::IncrementalClassifier::new(bars, &config);
    let mut strict_nest_sidecar = StrictNestSidecarCollector::new(strict_nest_sidecar_enabled());
    let fill = pi_theta_fill_loop(
        // ★工位 4g：返回塔代次（TreeCache O(1) 命中判据，跳过 per-bar O(tree) TreeKey::of）。
        |i| {
            let (cls, tower) = if strict_nest_sidecar.enabled {
                let (l0, cls, tower) = classifier_incr.classify_at_with_l0(i);
                strict_nest_sidecar.observe_frame(
                    &l0,
                    &cls,
                    &tower,
                    &config,
                    classifier_incr.tower_cache(),
                );
                (cls, tower)
            } else {
                classifier_incr.classify_at(i)
            };
            let cl = classifier_incr.tower_confirmed_lens(tower.len());
            let gen = classifier_incr.tower_generation();
            let fe = classifier_incr.forest_epoch(); // ★on2w2：K_i O(1) 命中判据。
            (cls, tower, cl, gen, fe)
        },
        bars,
        initial_nav,
        &config,
        // χ 过滤上下文（None ⟹ χ≡1 全覆盖；Some ⟹ Γ_t^trade={γ:μ>θ}，来自 run_theta_v0_pi_chi）。
        chi,
    );

    let bh_return = buy_and_hold_return(bars);
    let m = metrics::compute(
        &fill.equity_curve,
        &fill.daily_returns,
        &fill.trade_pnls_with_forced,
        years,
        bh_return,
    );

    let n_orders = fill.n_orders;
    let untradable_ratio = dataset.untradable_ratio();
    let is_l2 = n_orders > 0;
    let prices: Vec<f64> = bars
        .iter()
        .map(|b| b.close as f64 * config.tick.tick_size)
        .collect();
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;
    let theta_return_mtm = m.strat_return;
    let strict_nest_sidecar = strict_nest_sidecar.finish();

    RunResult {
        symbol: dataset.symbol.clone(),
        metrics: m,
        n_bars: bars.len(),
        n_orders,
        untradable_ratio,
        is_l2,
        closed_loop_final,
        trade_pnls: fill.trade_pnls_realized,
        trade_pnls_with_forced: fill.trade_pnls_with_forced,
        trades: fill.trades,
        prices,
        fee_rate,
        theta_return_mtm,
        daily_returns: fill.daily_returns,
        equity_curve: fill.equity_curve,
        r_decomp: fill.r_decomp, // 生产 π 路径 R 分解（cost_model=None ⟹ 三项 0，仍产分解表）
        strict_nest_sidecar,
        otherwise_domain_sidecar: None, // 本路径（run_theta_v0_pi/_chi/_chi_shrink）未接线（S1 仅接 overlay 臂）
    }
}

/// ★M5 声部执行层跑批产出（多空对冲.pdf p16 关卡10 / TARGET_STRATEGY_MAXFULL.md §M5）。
///
/// [`run_theta_v0_pi_overlay`] 的产出：终态 [`OverlayState`](super::super::strategy::overlay_state)
/// hedge-mode 逐声部账本（活动 + 已离场声部归因表）+ 净额执行层证据。
pub struct OverlayRunResult {
    pub symbol: String,
    pub n_bars: usize,
    /// 净额执行层 fill 事件数（overlay ΔN 非零步数 = 真实下单次数）。
    /// ★090 三态一致（L1-P3，D-Ovr-1）：同源 `net_result.n_orders`（= `fill.n_orders`，
    /// 每 fill 事件 `executed_qty>0` 才计数，runner.rs:1191-1195），与 ΔN 守恒断言
    /// （`pi_theta_fill_loop_overlay` 内逐 bar `order==ΔN` debug_assert）口径一致。
    /// ★W1 例外：env `VOICE_EXEC=1` 时 = **声部 fill 事件数**（执行投影口径，见 `voice_exec`
    /// 字段——两读数分列，禁冒充净额口径）。
    pub n_overlay_fill_events: usize,
    /// overlay 层声部总数（已离场 + 活动声部），诊断读数——**声部 ≠ 订单**
    /// （一声部生命周期内可产多次 ΔN fill 事件：开仓/减仓/平仓/再开）。
    /// 本字段承载原 `n_overlay_orders` 的赋值语义（该旧字段名声部数冒充订单数，090 违规已修）。
    pub n_overlay_voices: usize,
    /// 账户级累计净额价格 PnL（`Σ_t N_t·ΔP_t`，PDF §11 对账账户侧）。
    pub account_price_pnl: f64,
    /// Σ_v pnl_v（分账本侧，PDF §11 应 ≈ `account_price_pnl`）。
    pub total_voice_pnl: f64,
    /// 对账残差 `|account_price_pnl − total_voice_pnl|`（验收断言2：< eps）。
    pub reconcile_residual: f64,
    /// 终态 overlay 账本（活动 + 已离场声部归因，逐声部 entry_v/exit_v/parent(v)/role(v)/pnl_v）。
    pub overlay: super::super::strategy::overlay_state::OverlayState,
    /// ★LEE M1（multi-level-native-execution-design-20260719 §D M1，#644 语义重放）：终态级别
    /// 账本镜像——与 `overlay` 并列的只读旁路，同一 `sep_legs` 按 `id.level`≡formation_level
    /// 分桶重放；LEE-Net 恒等 `Σ_ℓ net_ℓ ≡ N` 见 `level_ledger.lee_net_witness()`
    /// （`identity_witnessed()` = 残差恒 0 且曾见非零净敞口）。**只读**：本字段不影响
    /// `net_result` 任何数值（level_order/level_risk sizing 接线不在 #644，归 #755）。
    pub level_ledger: super::super::strategy::level_ledger::LevelLedgerMirror,
    /// ★LEE M3 `clock_ℓ`（multi-level-native-execution-design-20260719 §D M3，#644 语义重放）：
    /// 逐 bar 事件钟点**只读**累计读数——**不**用于门控订单（本仓不接 `level_order.regate`，
    /// 见 `strategy::level_clock` 模块头「只读不改决策」纪律）。`sparsity_witnessed()` 判据同源。
    pub level_clock: super::super::strategy::level_clock::LevelClockStats,
    /// ★LEE 归因算子层（#644 语义重放，`strategy::level_attrib::attribute_total`）：逐 bar 把
    /// 当步 M0 净额目标 `standard_p_star`（**已经生产 M0 路径算出，本读数只读消费，不回写**）
    /// 按结构基准 `level_nets(sep_legs)` 归因到各级的**只读诊断**计数——决策点总数 /
    /// 落入账户层残差桶（`LEVEL_ACCOUNT_RESIDUAL`，无结构级别可归因）的决策点数 /
    /// 经比例缩放（`Σbasis≠total`）的决策点数。
    pub level_attrib_n_bars: u64,
    pub level_attrib_n_residual_bars: u64,
    pub level_attrib_n_rescaled_bars: u64,
    /// 净额执行层 RunResult（同 `run_theta_v0_pi`，净额订单/权益——overlay 是其只读旁路，数字不变）。
    /// ★W1 例外：env `VOICE_EXEC=1` 时本字段承载**声部执行投影**口径（见 `voice_exec` 字段
    /// 注释——fill.n_orders=声部 fill 事件数、equity/trade_pnls/r_decomp=声部账户），
    /// 决策层（typed_ledger/TW/sep_legs）仍与净额臂逐字节一致。
    pub net_result: RunResult,
    /// ★M8 treasury 层终态（TARGET_STRATEGY_MAXFULL.md M7:156-159）：三阶段资金账本 `TwState`
    /// 终态（stage/free/holding/withdrawn/notional_in/open_legacy_legs）。overlay 臂驱动的同一
    /// 主 loop 内建 TW 账本（`pi_theta_fill_loop_overlay` 的 `fill.tw_final`），此前被
    /// `net_result: RunResult` 装配丢弃（RunResult 无 tw_final 字段）——M8 端到端四层报告的
    /// treasury 层（第三层）需读它算 `Reach(Stage)`/`Q_T`/`W_T`/`η_T`。`None` 仅当 bars 为空。
    /// ★W1：TW 账本由净额影子账本驱动 ⟹ VOICE_EXEC=1 时本字段仍与净额臂逐字节一致。
    pub tw_final: Option<super::super::strategy::ledger::TwState>,
    /// ★W1 声部独立执行读数（churn 修复臂，netting-vs-voice-execution-audit-20260719 §7）。
    /// env `VOICE_EXEC=1` 时 `Some`：此时 `net_result`/`n_overlay_fill_events` 承载**声部执行投影**
    /// 口径（fill.n_orders = 声部 fill 事件数，equity/trade_pnls/r_decomp = 声部账户）——与净额臂
    /// 语义不同，两读数经本字段分列，禁互相冒充（090）。未设 env ⟹ `None`，全部净额读数逐字节不变。
    pub voice_exec: Option<VoiceExecRunSummary>,
    /// issue #357（T4/#294 生产实例化）：每仓 campaign 账簿终态——同 `tw_final`，`net_result:
    /// RunResult` 无此字段，`pi_theta_fill_loop_overlay` 内建的 `fill.campaign_book` 装配丢弃，
    /// 由本字段单独转发。
    pub campaign_book: super::super::strategy::oscillation_campaign::CampaignBook,
    /// issue #357 验收③：enabled=true wf8 产物级见证读数（减补动作归属分桶/`CenterNotAlive`
    /// 丢弃率/挂起归宿/campaign 生死事件）。
    pub campaign_witness: super::super::strategy::oscillation_campaign::CampaignWiringWitness,
}

/// ★W1 声部独立执行跑批读数（VOICE_EXEC=1 时装配；验收量见审计 §7.4）。
pub struct VoiceExecRunSummary {
    /// 声部 fill 事件数（executed>0 才计；== 本臂 `net_result.n_orders`）。
    pub n_voice_fills: usize,
    /// 开过的声部 campaign 总数（含在飞）。
    pub n_voices_total: usize,
    /// 窗口终点仍在飞的声部数（censored，经虚拟兑现入归因表）。
    pub n_voices_open_end: usize,
    /// 声部毛周转（手数）G = Σ(q开+q平)——事件驱动下 == 实际成交周转 T（审计 §7.4 验收量）。
    pub gross_turnover_lots: i64,
    /// 声部账户累计成交费（Commission+Slippage+Tax）。
    pub fee_paid: f64,
    /// 声部账户 R 分解守恒残差（应 ≈0；loop 内 debug_assert 同锚）。
    pub conservation_residual: f64,
    /// 事件驱动上界不变量见证：`n_voice_fills ≤ 2×n_voices_total`（resize 事件默认空集，
    /// 构造性可断——审计 §7.4 断言）。
    pub event_bound_holds: bool,
    /// 声部账户价格 PnL 对账残差 |account_price_pnl − Σ_v pnl_price|（PDF §11，< eps）。
    pub price_pnl_reconcile_residual: f64,
    /// 终态声部执行簿（逐声部归因表；含 forced 强平行）。
    pub book: super::super::strategy::overlay_state::VoiceExecBook,
}

/// ★M5 声部执行层 arm（多空对冲.pdf p16 关卡10）：与 [`run_theta_v0_pi`] **同一信号决策路径**
/// （χ≡1 全覆盖），额外驱动 [`OverlayState`](super::super::strategy::overlay_state) hedge-mode 逐声部
/// 账本 P^sep → N=Net(P^sep) → Order_t=ΔN → 逐声部 pnl_v 归因。
///
/// **净额路径 bit-exact**：overlay 是 `pi_theta_fill_loop_overlay` 内 sep_legs 的只读旁路——不改
/// cash/units/trade_pnls/equity ⟹ `net_result` 与 `run_theta_v0_pi` 逐字节一致（现有臂不污染）。
///
/// **验收**（结果包）：① ΔN 守恒（`pi_theta_fill_loop_overlay` 内 debug_assert 逐 bar `order==ΔN`，
/// 零违例）；② Σpnl_v 对账（`reconcile_residual < eps`，PDF §11 线性恒等 `Σσ_v q_v ΔP=N ΔP`）。
///
/// **认识论 L1**：ΔN 守恒 + 对账是结构恒等（构造性 + 线性代数），**不声明 alpha**——首轮数字
/// （亏损/空转）照实（执行层首次真实化，PDF §9 声部生成层 + 净额可见层验收，非经济有效层）。
///
/// ★W1（churn 修复臂，netting-vs-voice-execution-audit-20260719 §7）：env `VOICE_EXEC=1` ⟹
/// 接入声部独立执行（`VoiceExecBook` 驱动真实 fill：声部独立持仓 + 事件驱动开合 + 开仓冻结
/// sizing），决策层（typed_ledger/TW/sep_legs）由净额影子账本驱动 ⟹ 与净额臂逐字节一致；
/// **env 未设 ⟹ 净额执行 + overlay 只读旁路，生产路径逐字节不变**（bit-exact 回归锁，
/// `voice_exec_env_unset_bit_exact` 测试锚）。
pub fn run_theta_v0_pi_overlay(
    dataset: &Dataset,
    config: &ThetaConfig,
    years: f64,
    initial_nav: f64,
) -> OverlayRunResult {
    let bars = &dataset.bars;
    let mut overlay = super::super::strategy::overlay_state::OverlayState::new();
    // ★LEE M1（#644 语义重放）：与 overlay 并列的级别账本只读旁路镜像（同一 sep_legs 按
    // id.level 分桶）。只读——不改本函数下方任何决策/账本变量。
    let mut level_ledger = super::super::strategy::level_ledger::LevelLedgerMirror::new();
    // ★W1 env gate：VOICE_EXEC=1 ⟹ 声部独立执行臂；未设/非"1" ⟹ 净额臂（bit-exact）。
    let voice_exec_on = voice_exec_gate();
    let mut voice_book = if voice_exec_on {
        Some(super::super::strategy::overlay_state::VoiceExecBook::new(
            initial_nav,
        ))
    } else {
        None
    };

    // ★T3 (#172 并门，#168 裁定 3)：层载由链路径单一驱动——链活（nest 证书门开）⟹ 投影层
    // 必载（含三元锚索引，恰好存在物化基座）；链死不载（零开销红线不死）。生产唯一派生点
    // （层门配置面退役；派生必须先于分类器构建——层 stamping 读派生后机制位）。
    let config = super::admission::chain_driven_level_projection(config);
    let mut classifier_incr = super::incremental::IncrementalClassifier::new(bars, &config);
    // D4（#606 S1 返工）：同 `run_theta_v0_pi_inner` 的 `strict_nest_sidecar` 先例——
    // `enabled=false`（env 未设，默认态）⟹ `observe_frame` 立即 no-op；纯观测面，不参与 fill
    // loop 任何决策输入、不改变 `classify_at` 调用本体（挂点在 signal.rs 生产路径内部，见
    // `OtherwiseDomainSidecarCollector` 文档）。
    let mut otherwise_domain_sidecar =
        OtherwiseDomainSidecarCollector::new(otherwise_domain_sidecar_enabled());
    let fill = pi_theta_fill_loop_overlay(
        |i| {
            let (cls, tower) = classifier_incr.classify_at(i);
            otherwise_domain_sidecar.observe_frame();
            let cl = classifier_incr.tower_confirmed_lens(tower.len());
            let gen = classifier_incr.tower_generation();
            let fe = classifier_incr.forest_epoch();
            (cls, tower, cl, gen, fe)
        },
        bars,
        initial_nav,
        &config,
        None, // χ≡1 全覆盖（与 run_theta_v0_pi 同信号路径）
        Some(&mut overlay),
        Some(&mut level_ledger),
        voice_book.as_mut(),
    );
    let otherwise_domain_sidecar = otherwise_domain_sidecar.finish();

    // 净额执行层 RunResult（与 run_theta_v0_pi 同装配，bit-exact——overlay 是只读旁路）。
    let bh_return = buy_and_hold_return(bars);
    let m = metrics::compute(
        &fill.equity_curve,
        &fill.daily_returns,
        &fill.trade_pnls_with_forced,
        years,
        bh_return,
    );
    let n_orders = fill.n_orders;
    let untradable_ratio = dataset.untradable_ratio();
    let is_l2 = n_orders > 0;
    let prices: Vec<f64> = bars
        .iter()
        .map(|b| b.close as f64 * config.tick.tick_size)
        .collect();
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;
    let theta_return_mtm = m.strat_return;
    let closed_loop_final = run_closed_loop(bars, initial_nav);
    let net_result = RunResult {
        symbol: dataset.symbol.clone(),
        metrics: m,
        n_bars: bars.len(),
        n_orders,
        untradable_ratio,
        is_l2,
        closed_loop_final,
        trade_pnls: fill.trade_pnls_realized,
        trade_pnls_with_forced: fill.trade_pnls_with_forced,
        trades: fill.trades,
        prices,
        fee_rate,
        theta_return_mtm,
        daily_returns: fill.daily_returns,
        equity_curve: fill.equity_curve,
        r_decomp: fill.r_decomp,
        strict_nest_sidecar: None,
        otherwise_domain_sidecar,
    };

    let tw_final = fill.tw_final;
    let campaign_book = fill.campaign_book;
    let campaign_witness = fill.campaign_witness;
    let account_price_pnl = overlay.account_price_pnl();
    let total_voice_pnl = overlay.total_voice_pnl();
    let reconcile_residual = (account_price_pnl - total_voice_pnl).abs();
    // ★090 三态一致修正（L1-P3，D-Ovr-1）：fill 事件数同源 fill.n_orders（真 ΔN 非零步数，
    // 与 ΔN 守恒断言对齐）；原赋值（closed+active 声部数）保留为 n_overlay_voices 诊断读数——
    // 声部 ≠ 订单（一声部至少 open+close 两次 ΔN 端点，还可减仓/再开）。
    let n_overlay_fill_events = fill.n_orders;
    let n_overlay_voices = overlay.closed_voices().len() + overlay.active_voices().count();

    // ★W1：VOICE_EXEC=1 时装配声部执行读数（验收量 §7.4：fill 上界/毛周转/守恒残差/价格 PnL
    //    对账）；未设 ⟹ None（净额读数逐字节不变）。
    let voice_exec = voice_book.map(|book| {
        let conservation_residual = net_result
            .r_decomp
            .map(|r| r.conservation_residual)
            .unwrap_or(0.0);
        let price_pnl_reconcile_residual =
            (book.account_price_pnl() - book.total_voice_price_pnl()).abs();
        VoiceExecRunSummary {
            n_voice_fills: book.n_fills(),
            n_voices_total: book.total_voices(),
            n_voices_open_end: book.n_open_end(),
            gross_turnover_lots: book.gross_turnover_lots(),
            fee_paid: book.cum_fee(),
            conservation_residual,
            event_bound_holds: book.n_fills() <= 2 * book.total_voices(),
            price_pnl_reconcile_residual,
            book,
        }
    });

    OverlayRunResult {
        symbol: dataset.symbol.clone(),
        n_bars: bars.len(),
        n_overlay_fill_events,
        n_overlay_voices,
        account_price_pnl,
        total_voice_pnl,
        reconcile_residual,
        overlay,
        level_ledger,
        level_clock: fill.level_clock,
        level_attrib_n_bars: fill.level_attrib_n_bars,
        level_attrib_n_residual_bars: fill.level_attrib_n_residual_bars,
        level_attrib_n_rescaled_bars: fill.level_attrib_n_rescaled_bars,
        net_result,
        tw_final,
        voice_exec,
        campaign_book,
        campaign_witness,
    }
}

/// ★七链 π_Θ per-bar fill 循环（三适配器 [A][B][C] + 执行层父容器 σ_p + close_pred 折 𝒦_Θ）。
///
/// 沿用退役 v1 的账本/双口径/强平契约，但**入场决策走 π_Θ 七链**（非 recognize）：
/// 每 bar — ① 延迟成交挂单（[`apply_order`]）→ ② `p_t=units`（净 lot）→ ③ [A] **前缀因果重分类**
/// （`classify_at(i)`=`classify_with_tower(l0[0..=i])` → 因果塔 + 因果分类，再 [`newly_confirmed_step`]
/// 取本 bar 新确认买卖点=确认-bar 部署）+ [B] `base_units=NAV/px` + [C] thread `prev_active` → `pi_theta_step`（父容器 σ_p +
/// 风控门）→ ④ 挂单到 `exec_index`（延迟）→ ⑤ thread 活动集 → ⑥ MtM 权益。窗口终点强平（含浮盈口径）。
///
/// **★[A] 因果（639）**：`classify_at` 闭包**只用 ≤i 数据**（`bars[0..=i]` 前缀）——父容器方向 σ_p
/// 与风控止损 bsp 均从前缀因果分类查得（无 look-ahead）。runner 注入
/// `|i| classify_with_tower(parse_layer(bars[0..=i]))`；测试可注入合成闭包（隔离 fill 机制）。
/// **复杂度**：逐 bar 前缀重分类 = O(n²)（正确性优先；性能/采样是 L2/L3 下个工位，本工位不优化）。
///
/// **认识论 L1**（管线正确性，非 L2 alpha）：fill 模拟 + 账本推进确定，产 trades 是引擎管线串通
/// 的物证；是否盈利由 L2/L3 否证（下个工位）。
/// χ_t 阈值过滤上下文（task #41 chi-theta-filter）——`pi_theta_fill_loop` 的可选候选集过滤器。
///
/// `None`（不传）⟹ χ≡1 全覆盖（解释器处理全部 Γ_t，frozen Θ v0 bit-exact 不变）。`Some(ctx)` ⟹
/// 每 bar 把 `step_gamma`（确认-bar 部署候选）过滤为 `Γ_t^trade={γ:μ(z_γ)>θ}`（[`selector::filter_gamma`]）。
///
/// **μ 表因果性由调用方负责**（selector.rs 模块头诚实声明）：`est` 若来自全窗 in-sample 交易 = 泄漏
/// （L1 选择器逻辑生效证明，非 L2 alpha）；walk-forward 增量 μ 是下游 delta-r-alpha 工位（L2/L3）。
/// θ 为**常数**（不从样本 μ 分布选，codex Q1 审查确认无前视）。

/// G4（#134）：训练管线的 typed ledger 入口——**生产 π fill loop**（χ≡1 全覆盖）在 `bars` 上
/// 跑一遍，返回不可变 [`TypedTrade`] ledger（`build_mu_from_bars` 消费，替换 τ^reverse）。
///
/// nav 口径与 L3 harness 同（首可交易价×1000，下限 1e6）——腿级生命周期事件（interpret/
/// AncOK）不依赖 nav 绝对值；nav 只进 base_units/风控门（equity>0 常态下门全开）。
pub(super) fn typed_ledger_from_bars(bars: &[Bar], config: &ThetaConfig) -> Vec<TypedTrade> {
    let first_px = bars
        .iter()
        .find(|b| !b.untradable && b.close > 0)
        .map(|b| b.close as f64 * config.tick.tick_size)
        .unwrap_or(1.0);
    let nav = (first_px * 1000.0).max(1.0e6);
    let mut classifier_incr = super::incremental::IncrementalClassifier::new(bars, config);
    let fill = pi_theta_fill_loop(
        |i| {
            let (cls, tower) = classifier_incr.classify_at(i);
            let cl = classifier_incr.tower_confirmed_lens(tower.len());
            let gen = classifier_incr.tower_generation();
            let fe = classifier_incr.forest_epoch(); // ★on2w2：K_i O(1) 命中判据。
            (cls, tower, cl, gen, fe)
        },
        bars,
        nav,
        config,
        None, // χ≡1 全覆盖：训练对全候选集估 μ（μ 表尚不存在，无 χ 可查）
    );
    fill.typed_ledger
}

/// ★闭环 S_Θ 驱动（task #94 引擎实装核心）：把 bar 序列逐 bar 喂入闭环 [`hybrid_step`]。
///
/// 镜像 Lean `for bar { x = assemblyStep(x, e) }`——从初始闭环态出发，每根 bar 构造一个
/// [`AssemblyEvent`]（`NewBar(rising)`，rising = 该 bar 相对前 bar 收涨）跑一步 [`hybrid_step`]，
/// 闭环态喂回作为下一步输入。这消除旧 runner「account 构造一次不喂回」的开环单帧。
///
/// **每 bar 真更新喂回的物证**（与开环单帧对比）：
/// - `micro_state.bar_count` / `bars_seen` 每 bar +1（推进到 = 可交易 bar 数）。
/// - `ledger_state`（R=Π-A-W）每 bar 经 ledger_step 更新（开仓侧 Allocate / 平仓侧 Realize）。
/// - `tw_state`（TW 守恒 + stage 单向）每 bar 经 tw_step 更新（ReverseOpen / RecoverCapital）。
/// - `orders` 每 bar +1（订单计数推进）。
/// 闭环每步保持双账本不变量（见 closed_loop::transition 的 hybrid_step_preserves_* 测试）。
///
/// 返回闭环终态（`None` 仅当 bars 为空）。`i0` 进 ledger.i0 基线（NAV 绝对额由 fill 侧另算）。
///
/// ★认识论等级：闭环驱动 = **L1**（bit-exact 管线：Rust hybrid_step 逐 bar 推进与 Lean
/// assemblyStep 结构对齐 = 验证管线正确性，零信息增量）。真实数据回测的指标才是 L2。
///
/// ★★身份降级（#124 裁定4 明文）：本函数是**纯结构验证工具**（Lean 契约锚 bit-exact 对齐
/// 见证），**不再是 TW 机制的候选实装路径**——TW 账本的生产真值源已移到 [`pi_theta_fill_loop`]
/// 内的 `TwState`（真实交易事件流驱动，I_Θ 组合层 P2/P3/P4 谓词消费）。本函数的极简摘要事件流
/// （`NewBar(rising)` 布尔）与生产 π 订单流 disjoint，其输出仅作 `closed_loop_final` 结构证据，
/// 不喂任何 alpha 判定。
pub fn run_closed_loop(bars: &[Bar], initial_nav: f64) -> Option<AssemblyState> {
    if bars.is_empty() {
        return None;
    }
    // 初始闭环态（i0 = NAV 取整作账本基线；账本是结构分量，NAV 绝对额 fill 侧另算）。
    let i0 = if initial_nav > 0.0 {
        initial_nav as i64
    } else {
        1
    };
    // ★GAP3（codex 复审 + 裁定 A' 后口径）：**已注资 campaign** 开局（非零 TW，Q=campaign_notional
    // 名义敞口）。**本闭环在生产策略下不触达 EarningShares**（stage 恒 CostReduction）——A' 后
    // 已实现利润通道（schedule 减仓分支 realized_pnl→Realize）已存在，但 PhaseI intent 恒 Buy
    // （intent_adapter：Normal×PhaseI→Buy）⟹ 生产闭环无平仓 fill ⟹ 无已实现 PnL 产生 ⟹ 退本金门
    // （holding≥Q ∧ free≥target）在 TW=Q 守恒下互斥（L0 同价无盈亏定理同型，见 runner
    // `earning_shares_unreachable_l0_same_price_zero_pnl`）。生产 π 路径（pi_theta_fill_loop）有
    // 反向候选平仓 ⟹ 已实现利润真入 TW ⟹ 可达（见 `pi_loop_realized_profit_reaches_earning_shares`）。
    // 注资仍必要：使 ledger/仓位在真实建仓（首 bar 现金充足 Δ=1）时真线程化（非 inert 空转）。
    // ★GAP3 补桥（丢弃点1/2/3 修复后）注资口径：`campaign_notional = 首个可交易 bar 的市价`——使首 bar
    // 建 1 单位仓位现金恰足（affordable = free/price = notional/c₀ = 1），且 `holding(成本基)=c₀=notional_in`
    // ⟹ 退本金前提 `holding≥notional_in` 于建仓即满足。旧口径 `clamp(len/4,1,128)`（单位数尺度）在
    // 值模型 + 维度修复 affordable=free/price 下会因 `notional≪市价` 使 `free/price=0`（买不起一单位），
    // 是单位模型残留（现已消除）。注资使 ledger/仓位真线程化，且价格重估浮盈可累积至足额退本金。
    let campaign_notional = bars
        .iter()
        .find(|b| !b.untradable && b.close > 0)
        .map(|b| b.close)
        .unwrap_or(1)
        .max(1);
    let mut x = AssemblyState::funded_campaign(i0, campaign_notional);
    // κ=0 最小基线政策（PDF §10 canonical 默认；η⋆=L^wc）。
    let policy = RiskPolicy::baseline();

    // bar 闭环：每 bar 推进一步（rising = 相对前 bar 收涨；price = 外生市价，成交/重估侧消费）。第 0 根
    // 无前 bar，取 rising=true 起点。price 打通丢弃点1（bar.close 不再仅投影为 rising 布尔）。
    let mut prev_close = bars[0].close;
    for bar in bars {
        let rising = bar.close >= prev_close;
        // ★丢弃点2 修复：事件承载真实市价幅度（不可交易 bar close 可能异常，取 max(0) 由 schedule 判不成交）。
        let e = AssemblyEvent {
            parse_event: MicroEvent::NewBar(rising),
            price: if bar.untradable { 0 } else { bar.close.max(0) },
        };
        // ★闭环喂回：x_{t+1} = hybrid_step(x_t, e, policy)——闭环态每 bar 真更新（非构造一次）。
        // policy 携 barrier（stage_progression 在有 sound 资金源时才推进；L0 同价下无源 ⟹ 恒 CostReduction）。
        // ★codex R3 §9.3：hybrid_step 返 Result；生产路径（schedule 只派 ShortDiff + cash 约束）恒 Ok，
        // 此处 `.expect()` 在生产边界把「闭环恒合法」显式化为契约（run_closed_loop 对上游仍返 Option，
        // Result 的 Err 语义只在外部注入非法 OrderOut 时触发，不经此生产闭环）。
        x = hybrid_step(&x, &e, &policy)
            .expect("run_closed_loop 生产闭环恒 Ok（schedule 只派 ShortDiff + cash 约束 + stage_progression w≤free）");
        prev_close = bar.close;
    }
    Some(x)
}

/// buy&hold 收益率（首尾可交易 bar 的 close 比率）。整数 tick 比率无 tick_size 依赖。
fn buy_and_hold_return(bars: &[Bar]) -> f64 {
    let first = bars.iter().find(|b| !b.untradable && b.close > 0);
    let last = bars.iter().rev().find(|b| !b.untradable && b.close > 0);
    match (first, last) {
        (Some(f), Some(l)) if f.close > 0 => l.close as f64 / f.close as f64 - 1.0,
        _ => 0.0,
    }
}

#[cfg(test)]
#[path = "runner_tests.rs"]
mod tests;
