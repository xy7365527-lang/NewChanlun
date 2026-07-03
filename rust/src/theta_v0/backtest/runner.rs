//! 回测管线串联——data → 引擎(parse→classify→plan_orders) → fill → 权益曲线 → metrics。
//!
//! ## 认识论等级（强制标注，formalization-validity-domain 231号）
//!
//! - 管线串联 + fill 模拟本身 = **L1**（验证管线串通，零信息增量）。
//! - 喂**真实历史数据**跑 frozen Θ 产出的指标 = **L2**（可否证）——**当且仅当**引擎管线
//!   产出非空订单流（阻塞点 A+B 解除）。
//!
//! ## 等引擎阻塞点（精确定位，不伪造已通）
//!
//! 完整管线（strategy/mod.rs piTheta 链）：
//! ```text
//! parse_layer → classify → recognize(Classification, bars) → VoiceDecision[]
//!             → plan_orders(decisions, bars, account, config) → Order[]
//! ```
//!
//! 当前断点**精确位于** `Classification → VoiceDecision[]`：
//!
//! - **阻塞点 A**：`classifier::classify` 返回空 [`Classification`]（task #79
//!   cc-classifier 实装中）——无 BSP / 走势 / 中枢标签。
//! - **阻塞点 B（recognize 桥接层缺失）**：strategy/mod.rs 注释声明了 `recog`
//!   （`Classification + bars → VoiceDecision[]`，对齐 StrategyFamily.lean `recog`），
//!   但 **`recognize` 函数尚未实装**（task #80 cc-strategy 实装中，依赖 #79 冻结的
//!   `bsp` 索引语义做价格桥接）。已实装的 `plan_orders(decisions, bars, account, config)`
//!   消费 `VoiceDecision[]`，但当前**无人产出非空 decisions**。
//!
//! 因此 [`run_theta_v0`] 串通到 `classify`（已存在），然后以**空 `decisions`** 调用
//! 已实装的 `plan_orders`——这真实反映阻塞态（recognize 缺失 ⇒ decisions 空 ⇒ 订单空 ⇒
//! 无交易 ⇒ 权益持平）。验证 **data→parse→classify→plan_orders→fill→metrics 全链路 L1
//! 跑通**，但**不是 L2 回测**（无 Θ 决策被检验）。
//!
//! **接通点（引擎就绪后一处修改）**：`recognize` 实装后，把 [`run_theta_v0`] 中的
//! `let decisions = Vec::new();` 替换为 `let decisions = strategy::recognize(&classification,
//! bars, config);`。已用 `RECOGNIZE_NOT_IMPLEMENTED` 常量 + 注释标记该接线点。
//!
//! ## fill 模拟（Θ_exec，reference-theta-v0.md:49-54）——可独立于引擎管线实装
//!
//! 订单 [`Order`] 携带 `exec_index`（执行延迟后的成交 bar）、`action`、`qty`。fill 语义
//! （延迟 1 根 K 下一根 open 成交、费用 commission/slippage/tax、不可交易过滤、止损成交）
//! 是 Θ_exec 的执行层，**不依赖 classify/recognize 的内部逻辑**——只要拿到 `Vec<Order>`
//! 就能模拟。本骨架实装 fill 的**基础形态**（开仓/平仓 + 费用），止损成交/冲突顺序待
//! 与 strategy 的订单语义对齐后补全（标注于代码）。
//!
//! 注：strategy 已有独立的 `exec` 子模块（延迟/费用/止损成交/冲突排序，对齐 Θ_exec）。
//! 本 harness 的 [`simulate_fills`] 是**回测账本侧**的权益曲线推进（消费 `Order[]` 算 NAV
//! 轨迹），与 strategy::exec（产出 `Order` 的执行语义）分工不同——后者决定"成交价/方向"，
//! 前者决定"权益如何随成交演化"。引擎稳定后二者口径对齐（fill 价取 Order 已定的成交价）。

use std::rc::Rc;
use super::super::closed_loop::state::{AssemblyState, MicroEvent};
use super::super::closed_loop::transition::{hybrid_step, AssemblyEvent};
use super::super::strategy::ledger::RiskPolicy;
use super::super::config::ThetaConfig;
use super::super::strategy::exit::{exit_decision_for, record_held_voice, HeldVoice};
use super::super::strategy::{AccountState, VoiceDecision};
use super::super::types::{Bar, Order, StrictAction};
use super::super::{classifier, parser, strategy};
use super::data::Dataset;
use super::metrics::{self, Metrics};

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
}

/// 执行回测：把 [`Dataset`] 喂 frozen Θ v0 引擎，模拟 fill，算指标。
///
/// **管线**（对接 theta_v0 冻结接口，piTheta 链）：
/// 1. `Dataset.bars`（已量化整数 tick）→ `parser::parse_layer(bars, config)` → `ParseLayer`
/// 2. `classifier::classify(&l0, config)` → `Classification` ⟸ **阻塞点 A（当前返回空）**
/// 3. `recognize(&classification, bars, config)` → `VoiceDecision[]` ⟸ **阻塞点 B（recognize
///    未实装，当前用空 decisions）**
/// 4. `strategy::plan_orders(&decisions, bars, &account, config)` → `Vec<Order>`（已实装，
///    消费 VoiceDecision；空 decisions ⇒ 空订单）
/// 5. fill 模拟（[`simulate_fills`]）→ 权益曲线 → `metrics::compute`
///
/// `years`：窗的年化基数（runner 调用方按数据日历传入，§3.1）。
/// `initial_nav`：回测初始资金（绝对额）。**口径关键**：Θ_risk sizing 用 `ρ*NAV/(...)`
/// 算 lot qty——NAV 太小（如 1.0）会令所有品种 qty=0（买不起 1 lot），sizing 不产单。
/// 调用方应传入与品种价格量级匹配的 NAV（如品种首价 × 容量倍数）。权益曲线归一化输出
/// （收益率口径，与 NAV 绝对值无关；NAV 只影响 sizing 的 qty 取整）。
pub fn run_theta_v0(
    dataset: &Dataset,
    config: &ThetaConfig,
    years: f64,
    initial_nav: f64,
) -> RunResult {
    let bars = &dataset.bars;

    // ── 步骤 1：解析（L0=1分钟线段账本）。bit-exact 整数 tick 域。 ──
    let l0 = parser::parse_layer(bars, config);

    // ── 步骤 2：分类（递归级别 + BSP）。classify 实装 (#79)，产 Classification（含 bsp）。──
    let classification = classifier::classify(&l0, config);

    // ── 步骤 3：recog（Classification → VoiceDecision[]）。recognize 实装 (#80)，已接通。 ──
    // 阻塞点 B 解除（2026-06-26 cc-strategy）：recognize 遍历各级别 bsp 产声部决策。
    // 无 bsp（数据无「中枢+离开+回试」结构）⟹ 空 decisions ⟹ 空订单（如实反映，非伪造）。
    let decisions: Vec<VoiceDecision> = strategy::recognize(&classification, bars, config);

    // ── 步骤 4+5：★闭环 NAV mark-to-market（Task A，消除开环单帧）。 ──
    // 旧引擎：account 构造一次（初始 NAV=1e6），plan_orders 全部订单用固定 NAV sizing
    // → qty 过大 → strat_return=3464% 不可信（Origin.TotalWealth 定义未满足）。
    // 新引擎：decisions 按 exec_index 分组，逐 bar 推进：每个 exec_index 到达时，用
    // 当前账本 TW（cash + units×px）动态计算 NAV，重新 plan_orders + fill + 更新账本。
    // 对齐 Origin.TotalWealth：TW = free（cash）+ holding（units × current_price）。
    // ★认识论：账本更新 = L0（Origin.TotalWealth 结构恒等）；指标结果 = L2（真实数据回测）。
    let closed_loop_final = run_closed_loop(bars, initial_nav);
    let fill = plan_and_fill_mtm(&decisions, bars, initial_nav, config);

    // buy&hold 对照（首尾 close，整数 tick → f64 比率，无 tick_size 依赖）。
    let bh_return = buy_and_hold_return(bars);

    // metrics.compute 用含浮盈口径的 trade_pnls（终点强平，编排者铁律「浮盈算上」）——
    // strat_return（MtM 权益曲线）本就含浮盈，win_rate/profit_factor/top5 用含浮盈口径对齐。
    let m = metrics::compute(
        &fill.equity_curve,
        &fill.daily_returns,
        &fill.trade_pnls_with_forced,
        years,
        bh_return,
    );

    let n_orders = fill.n_orders;
    let untradable_ratio = dataset.untradable_ratio();
    // L2 判据：产出非空订单流（阻塞点解除）+ 真实数据。订单空 ⇒ 仅 L1（管线串通）。
    let is_l2 = n_orders > 0;

    // 原始价格序列（close 口径，与账本侧成交价一致）——操作语义随机对照在其上重执行。
    let prices: Vec<f64> = bars
        .iter()
        .map(|b| b.close as f64 * config.tick.tick_size)
        .collect();
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;
    // Θ MtM 复利口径 total_return（权益曲线）——仅 significance 报告参考，非比较基准
    // （比较用 significance 内部同口径值；先取，m 随后 move）。
    let theta_return_mtm = m.strat_return;

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
    }
}

/// ★**七链 π_Θ 生产 runner（塔导出桥 (iii) 布线段，接 Nautilus 核心实装）**——与 [`run_theta_v0`]
/// （买卖点 v1 `recognize`）**并行**的新路径（编排者 Q1：最小侵入并行路径，v1 保留作对照基线）。
///
/// ## 与 v1 的正交（Q1 裁定）
///
/// `run_theta_v0` 走 `strategy::recognize`（每 bsp 一 [`VoiceDecision`]，离散择时，全窗 L3 8/8 否证）。
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
/// ShortDiff 子腿**——故本 runner **不开 naked 逆势仓**（639(c) 兑现）。两机制正交：σ_p 用因果塔
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
/// >   与 run_theta_v0 同）。② 前缀塔仅 L0（tower.len()<2，host 是根）⟹ 候选父=∂ ⟹ Ambient（早 bar/
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
/// >   `metrics::compute`/`coverage::pi_theta_step`；**不改** `run_theta_v0`/`recognize`/`plan_and_fill_mtm`。
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

    // 闭环终态证据（与 run_theta_v0 同——bar 闭环驱动）。
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
    let mut classifier_incr = super::incremental::IncrementalClassifier::new(bars, config);
    let fill = pi_theta_fill_loop(
        // ★工位 4g：返回塔代次（TreeCache O(1) 命中判据，跳过 per-bar O(tree) TreeKey::of）。
        |i| {
            let (cls, tower) = classifier_incr.classify_at(i);
            let gen = classifier_incr.tower_generation();
            (cls, tower, gen)
        },
        bars,
        initial_nav,
        config,
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
    }
}

/// 买卖点身份判别 u8（seen-set append-only diff 键；6 类 bit 打包）。
///
/// 同一 `(level, source_index)` 上不同类买卖点（如 2买/3买 V 型可共存）是不同身份 ⟹ 入 bits 判别。
fn bsp_bits_disc(b: &super::super::types::BspBits) -> u8 {
    (b.buy1 as u8)
        | (b.buy2 as u8) << 1
        | (b.buy3 as u8) << 2
        | (b.sell1 as u8) << 3
        | (b.sell2 as u8) << 4
        | (b.sell3 as u8) << 5
}

/// ★[A] per-bar **确认-bar 部署**（修 bsp→订单转化；`recursive_t/stream.rs:273` 同构）。
///
/// 输入是**前缀因果分类** `classify_with_tower(l0[0..=i])`（639；非全窗）。返回本 bar **新确认**的
/// 买卖点（append-only diff vs `seen`，stream.rs 的 `seen_bsps`/"只增不改"语义）：`seen.insert(key)`
/// 为真（首次出现于前缀塔）⟹ 本 bar i 确认 ⟹ 保留 + 部署；已 seen ⟹ 跳过。
///
/// ## 为什么不是 `source_index==i`（旧错口径，零订单根因）
/// 买卖点**回溯确认**——其 `source_index`（触发 K 序）的端点常在**比 source_index 晚的 bar** 才被
/// 结构（中枢突破/线段确认/背驰）确认入前缀塔。按 `source_index==i` 切，当前 bar i 的前缀塔里
/// `source_index==i` 位置的买卖点**往往尚未确认**（host 仅 L0 / 候选父=∂ ⟹ Ambient 空切）⟹ 候选恒空
/// ⟹ **零订单**。改为"本 bar 新确认买卖点 diff"：买卖点在其被确认的那根 bar（其 source_index≤i）部署
/// （确认时点 = 因果，无 look-ahead；与 stream.rs「本 bar 新增 BSP 在当前 bar 投放」同构）。
///
/// 保留 `levels` 级别结构（与因果塔 `tower_i` 级别对齐）；`moves`/`centers` 空（σ_p 父容器方向由
/// `tower_i` 经 `assemble_gamma_with_tower` 的 `attach_bsp_to_tree` 查得，不读 classification moves/centers）。
fn newly_confirmed_step(
    classification: &classifier::Classification,
    seen: &mut std::collections::HashSet<(usize, usize, u8)>,
) -> classifier::Classification {
    use super::super::classifier::LevelState;
    classifier::Classification {
        levels: classification
            .levels
            .iter()
            .enumerate()
            .map(|(lvl, ls)| LevelState {
                moves: Vec::new(),
                centers: Rc::new(Vec::new()),
                // append-only：seen.insert 为真=本 bar 首次确认 ⟹ 保留；副作用把所有 bsp 标记 seen。
                bsp: ls
                    .bsp
                    .iter()
                    .filter(|p| seen.insert((lvl, p.source_index, bsp_bits_disc(&p.bits))))
                    .cloned()
                    .collect::<Vec<_>>()
                    .into(),
            })
            .collect(),
    }
}

/// [close_pred 折 𝒦_Θ] 计算 [`KThetaRiskGate`]（Q2：风控 stop/risk 作可行集约束门，非第二出口）。
///
/// 复用 `exec::close_pred` 契约锚（no-patch-keep-primitive）：
/// - **risk（GlobalRiskClose）**：`risk_mode(equity)` ∈ {Insolvent,Liquidation} ⟹ `force_flat`
///   （v0 可计算 Insolvent `E_t≤0`；账户层 MM/buffer/liq 未建模，诚实有效域 L0）。
/// - **stop（结构止损触及）**：per 活动腿从**全窗** `classification` 查 `BspPoint` 算
///   `structural_stop`，`stop_hit(bar,…)` 判触及——多腿止损 ⟹ `stop_long`、空腿止损 ⟹ `stop_short`。
/// - **reverse_signal 不入本门**：反向信号关活动腿走 `interpret` 𝒟_x（腿级单出口）；
///   **parent_invalid** v0 root 恒 false（无父）。
fn k_theta_risk_gate(
    prev_active: &[super::super::strategy::interp::ActiveLeg],
    classification: &classifier::Classification,
    bar: &Bar,
    equity: f64,
    p_t: f64,
    px: f64,
    margin: Option<&super::super::strategy::risk::MarginModel>,
) -> super::super::strategy::coverage::KThetaRiskGate {
    use super::super::strategy::coverage::KThetaRiskGate;
    use super::super::strategy::exec::{close_pred, stop_hit, CloseTriggers, FillSide};
    use super::super::strategy::risk::{
        global_risk_close, margin_inputs, risk_mode, structural_stop, RiskMode, RiskModeInput,
        StopInput, StopSide,
    };
    use super::super::strategy::voice::VoiceSide;
    use super::super::types::Center;

    // risk mode：有 margin 注入且 bar 时间落某快照段 ⟹ 真实 MM/liq/buffer（as_of 零前视，
    // margin-design §2.7）；否则退化 MM=0（bit-exact 现状，M1/M2/M3 不可达）。
    let mode = match margin.and_then(|m| m.book.as_of(bar.timestamp).map(|s| (m, s))) {
        Some((m, sched)) => {
            // p_t 净 lot × mark = 净名义（美元，margin-design §2.2：net_notional 已折算勿再乘价）。
            let net_notional_usd = p_t.abs() * px;
            risk_mode(&margin_inputs(net_notional_usd, equity, sched, &m.cushions))
        }
        None => risk_mode(&RiskModeInput {
            equity,
            maint_margin: 0.0,
            buffer1: 0.0,
            buffer2: 0.0,
            liq_flag: false,
        }),
    };
    let risk_close = global_risk_close(mode);
    // M2/M3（Deleverage/CloseOnly）：净幅上限=当前 |p_t|（margin-design §2.8，禁增仓 → 真改订单流）。
    let no_increase_cap = match mode {
        RiskMode::Deleverage | RiskMode::CloseOnly => Some(p_t.abs()),
        _ => None,
    };

    // stop：per 活动腿结构止损触及（从全窗 classification 查 BspPoint，与 v1 同一 structural_stop）。
    // ponytail: 循环前预建 HashMap<(level,source_index),&BspPoint>，bsp.iter().find O(n) → map.get O(1)
    let mut bsp_index: std::collections::HashMap<(usize, usize), &classifier::bsp::BspPoint> =
        std::collections::HashMap::new();
    for (lvl_idx, lvl) in classification.levels.iter().enumerate() {
        for p in lvl.bsp.iter() {
            bsp_index.insert((lvl_idx, p.source_index), p);
        }
    }
    let mut long_stop = false;
    let mut short_stop = false;
    for leg in prev_active {
        let (stop_side, exit_side) = match leg.dir {
            VoiceSide::Long => (StopSide::Long, FillSide::Sell),
            VoiceSide::Short => (StopSide::Short, FillSide::Buy),
            VoiceSide::Flat => continue, // Flat 不入活动集（防御性）
        };
        let bsp = match bsp_index.get(&(leg.level as usize, leg.source_index)) {
            Some(b) => *b,
            None => continue, // 找不到对应买卖点（不应发生）⟹ 无止损读出
        };
        let stop_in = StopInput {
            pivot_low: bsp.pivot_low,
            pivot_high: bsp.pivot_high,
            // center=None（1/2 类不用 center）⟹ 零 center 占位（3 类必有 center，不到达）。
            center: bsp.center.unwrap_or(Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 0, end_index: 0 }),
        };
        if let Some(stop) = structural_stop(stop_side, &bsp.bits, &stop_in) {
            if !bar.untradable && stop_hit(bar, stop, exit_side) {
                match leg.dir {
                    VoiceSide::Long => long_stop = true,
                    VoiceSide::Short => short_stop = true,
                    VoiceSide::Flat => {}
                }
            }
        }
    }

    // close_pred 折 𝒦_Θ（契约锚保留）：风控项（stop ∨ risk）→ 方向约束门。
    KThetaRiskGate {
        force_flat: risk_close, // GlobalRiskClose ⟹ 𝒦_Θ={0}
        stop_long: close_pred(&CloseTriggers {
            parent_invalid: false,
            reverse_signal: false,
            stop: long_stop,
            risk_close,
        }),
        stop_short: close_pred(&CloseTriggers {
            parent_invalid: false,
            reverse_signal: false,
            stop: short_stop,
            risk_close,
        }),
        no_increase_cap, // M2/M3 净幅上限（margin-design §2.8）
    }
}

/// ★七链 π_Θ per-bar fill 循环（三适配器 [A][B][C] + 执行层父容器 σ_p + close_pred 折 𝒦_Θ）。
///
/// 镜像 [`plan_and_fill_mtm`] 的账本/双口径/强平结构，但**入场决策走 π_Θ 七链**（非 recognize）：
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
pub struct ChiFilterCtx<'a> {
    /// μ(z) 表（z 六维全互斥分类的样本边际收益）。
    pub est: &'a super::mu_estimator::MuEstimator,
    /// θ 阈值（成本/风险门槛，Θ_risk 参数，常数）。
    pub theta: f64,
    /// z_alpha 单边置信分位（如 1.645=95%）：准入用 LCB(μ)=mean−z_alpha·std/√n（严格alpha.pdf p25
    /// §12 防高维 z 过拟合）。z_alpha=0 ⟹ LCB=mean ⟹ 退化回裸 μ 门（向后兼容）。
    pub z_alpha: f64,
    /// 无 LCB 证据（空类 μ=None 或 n<2 单样本）χ 取值：false=不交易（最诚实，codex Q3）；
    /// true=全覆盖默认交易。
    pub treat_empty_as_pass: bool,
    /// 准入量切换（acc-three-way-l2 #83）：`None` ⟹ 准入量 = LCB(μ)（默认，z_alpha 驱动，frozen
    /// bit-exact）；`Some(τ²)` ⟹ 准入量 = mu_shrink(z,τ²) 层级收缩（z_alpha 忽略）。
    pub shrink_tau_sq: Option<f64>,
}

fn pi_theta_fill_loop<F>(
    mut classify_at: F,
    bars: &[Bar],
    initial_nav: f64,
    config: &ThetaConfig,
    chi: Option<ChiFilterCtx>,
) -> FillOutput
where
    // ★工位 4g：闭包返回三元组——第三个 u64 = 塔代次（TreeCache O(1) 命中判据）。
    F: FnMut(usize) -> (classifier::Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>, u64),
{
    use super::super::strategy::coverage::{self, PiThetaWeights};
    use super::super::strategy::exec::fill_bar_index;
    use super::super::strategy::interp::{self, ActiveLeg};

    let n = bars.len();
    let nav0 = if initial_nav > 0.0 { initial_nav } else { 1.0 };
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;
    let weights = PiThetaWeights::from_risk(&config.risk);

    let mut cash: f64 = nav0;
    let mut units: f64 = 0.0; // p_t = 净 lot（apply_order 维护，有符号：正多/负空/0空仓）
    let mut entry_cost: f64 = 0.0;
    // [C] 活动集台账（thread 跨 bar；interp::interpret 闭环递归）。
    let mut prev_active: Vec<ActiveLeg> = Vec::new();
    // ★persistent overlay（anc.pdf §4-§9）：跨 bar 持久元素注册表 Pi。
    // 修复 Q4 "LiveDetached 误处理成 Stale" → depth>0 腿被 AncOK 系统性剪掉 → #5α=0。
    // Pi+1 = merge(Pi, Ei+1, held legs)（§9）：snapshot 匹配刷新 + held 找不到标记 LiveDetached。
    let mut registry = super::super::strategy::persistent::PersistentRegistry::new();
    // ★确认-bar 部署 seen-set（append-only "只增不改"；已部署买卖点身份键，stream.rs:162 同构）。
    let mut seen_bsps: std::collections::HashSet<(usize, usize, u8)> = std::collections::HashSet::new();
    // 延迟成交队列（spec:50：订单在 exec_index bar 成交，与 plan_and_fill_mtm 同语义）。
    let mut pending: Vec<Vec<Order>> = vec![Vec::new(); n];
    // 工位 K 性能：tree-prefix 缓存（§16 confirmed prefix immutable；跨 bar 复用 extract_elements）。
    let mut tree_cache = interp::TreeCache::new();
    // ★工位 4h：candidate 段前缀缓存（源(b) O(n²) 真修；caller B merge 路径 gamma-free + 前缀复用）。
    let mut cand_cache = interp::CandidateCache::new();
    // ★工位 4f：上一 bar merge 用的 tree Rc——`Rc::ptr_eq` 命中（同一 Rc::clone）⟹ tree 逐字节不变
    // ⟹ merge tree 段跳过 step 1'/2'（confirmed prefix 增量维护，消 O(tree)/bar）。
    let mut prev_merge_tree: Option<std::rc::Rc<Vec<coverage::CoverageElement>>> = None;

    let mut equity_curve = Vec::with_capacity(n);
    let mut trade_pnls: Vec<f64> = Vec::new();
    let mut trades: Vec<metrics::TradeRecord> = Vec::new();
    let mut pos_entry_bar: Option<usize> = None;
    let mut n_orders_executed: usize = 0;

    for i in 0..n {
        let bar = &bars[i];
        let px = bar.close as f64 * config.tick.tick_size;

        // ── ① 延迟成交：本 bar 到达 exec_index 的挂单 fill（apply_order，先平后开）。 ──
        if !pending[i].is_empty() && !bar.untradable && px > 0.0 {
            let orders = std::mem::take(&mut pending[i]);
            for o in &orders {
                if o.qty > 0 {
                    let units_before = units;
                    apply_order(o, px, fee_rate, &mut cash, &mut units, &mut entry_cost, &mut trade_pnls);
                    // 轨迹配对（v1 方向中性）：units 跨 0 / 回 0 ⟹ 完整交易闭合。
                    track_position_transition(&mut trades, &mut pos_entry_bar, units_before, units, i, false);
                    n_orders_executed += 1;
                }
            }
        }

        // ── ② p_t = 净 lot（成交后真实持仓）。 ──
        let p_t = units;
        let current_nav = cash + units * px;
        let equity_nav = if current_nav > 0.0 { current_nav } else { nav0 };

        if !bar.untradable && px > 0.0 {
            // ── ③ [A] 前缀因果重分类（classify_at(i)=classify_with_tower(l0[0..=i]) → 因果塔 + 因果
            //      分类，只用 ≤i 数据 → 因果）+ 切当步候选 + [B] base_units U_ℓ + [C] thread + 风控门。 ──
            let (classification_i, tower_i, tower_gen) = classify_at(i);
            // ★当步候选 = 前缀因果塔里**本 bar 新确认**的买卖点（append-only diff vs seen，确认-bar
            // 部署）——非 source_index==i 切片（买卖点回溯确认，其触发点常在更晚 bar 才入前缀塔 ⟹
            // source_index==i 切恒空 ⟹ 零订单）。买卖点在被确认那根 bar（source_index≤i）部署=因果。
            let classification_step = newly_confirmed_step(&classification_i, &mut seen_bsps);
            let base_units = equity_nav / px; // U_ℓ：NAV/价 = 可建名义手数（方案A协变）
            // 风控门也用**前缀因果分类**（leg 止损 bsp 因果查得，非全窗非因果——与 σ_p 同因果口径）。
            let gate = k_theta_risk_gate(&prev_active, &classification_i, bar, equity_nav, p_t, px, config.margin.as_ref());
            // exec_index：延迟成交 bar（spec:50；尾部无可成交 bar ⟹ 不挂单）。
            let exec_index = fill_bar_index(i, bars, &config.exec);
            // 环5+6+7：pi_theta_step（父容器 σ_p=attach_bsp_to_tree(因果塔) + 风控门）→ (A_{t+1}, p*, O)。
            // 工位 K 性能：tree-prefix 缓存（§16，命中省 extract_elements 重建）。pi_theta_step 用
            // newly-confirmed step 候选；merge 用全 classification 候选——两者共享同一 tower tree-prefix 缓存。
            // ★热点② O(n²) 消除：tree=Rc::clone O(1)，candidate 段单独 Vec，ElementView 双段零拷贝。
            let (step_tree, step_candidates, step_gamma) =
                interp::coverage_elements_and_gamma_with_tower_cached_gen(
                    &classification_step, &tower_i, &mut Some(&mut tree_cache), Some(tower_gen),
                );
            // ★工位 4d 热点①②：注入缓存的 base（tree 前缀）兄弟/ID 索引（命中 Rc::clone O(1)），消除
            // coverage_step_from_buckets 内每 bar build_prev_sibling_index/build_tree_id_index O(tree)/bar。
            let mut step_work = coverage::ElementView::from_parts(&step_tree, step_candidates);
            if let Some((sib, id)) = tree_cache.tree_sibling_and_id() {
                step_work = step_work.with_base_indices(sib, id);
            }
            // ── ③' χ_t 阈值过滤（task #41）：Γ_t → Γ_t^trade={γ:μ(z_γ)>θ}（§13）。 ──
            // χ 仅滤候选集（喂 interpret 的 gamma）；step_work(tree/candidates) 不受影响——
            // coverage_step_prebuilt 内 gamma→interpret 三桶 与 work→AncOK 准入解耦（gamma 滤掉
            // μ≤θ 候选 ⟹ interpret 不归 open ⟹ 不开仓 = χ_t 语义）。None ⟹ χ≡1 全覆盖（不滤）。
            let step_gamma_trade = match &chi {
                Some(ctx) => super::selector::filter_gamma_with_admission(
                    &step_gamma, ctx.est, ctx.theta, ctx.z_alpha, ctx.shrink_tau_sq,
                    ctx.treat_empty_as_pass,
                ),
                None => step_gamma.clone(), // χ≡1：原候选集（bit-exact 不变）
            };
            let (next_active, _p_star, order) = coverage::pi_theta_step_prebuilt(
                step_work,
                &step_gamma_trade,
                &prev_active,
                p_t,
                exec_index.unwrap_or(i),
                base_units,
                &config.risk,
                weights,
                gate,
                &config.voice,
                &registry,
            );
            // ── ④ 挂单到 exec_index（延迟成交；qty>0 才挂）。 ──
            if order.qty > 0 {
                if let Some(ei) = exec_index {
                    if ei < n {
                        pending[ei].push(order);
                    }
                }
            }
            // ── ⑤ thread 活动集台账（喂下一 bar interpret 闭环）+ persistent registry 合并。 ──
            // ★persistent overlay（anc.pdf §9）：Pi+1 = merge(Pi, Ei+1, held legs)。
            // 用本 bar snapshot（elements）+ held legs（next_active）刷新 registry。
            // 关闭的腿（buckets.close）在 registry 中标记 invalidated（§9 rule 5）。
            {
                // ★工位 4h：caller B gamma-free + candidate 前缀缓存路径（源(b) O(n²) 真修）。merge 只
                // 消费 candidates（不读 gamma/role，codex Q1）⟹ 跳遍历2 + candidate 前缀复用（codex Q2/Q3）。
                let (tree_ref, candidates_ref) =
                    interp::coverage_elements_with_tower_cached_gen(
                        &classification_i, &tower_i, &mut tree_cache, &mut cand_cache, Some(tower_gen),
                    );
                // ★工位 4f：双段 merge（消 as_contiguous materialize O(tree) + step 1'/2' tree 全量 O(tree)）。
                // tree_dirty=false（Rc::ptr_eq 命中，tree 同上 bar）⟹ 跳过 tree 段（断言1-3 bit-exact）。
                let tree_dirty = prev_merge_tree
                    .as_ref()
                    .map(|p| !std::rc::Rc::ptr_eq(p, &tree_ref))
                    .unwrap_or(true);
                registry.merge_in_place_split(&tree_ref, tree_dirty, &candidates_ref, &next_active);
                prev_merge_tree = Some(tree_ref);
            }
            prev_active = next_active;
        }

        // ── ⑥ 权益曲线（mark-to-market，归一化 ÷nav0）。 ──
        equity_curve.push((cash + units * px) / nav0);
    }

    // ── 窗口终点强平（含浮盈口径，编排者铁律「不把浮盈算上不合理」；与 plan_and_fill_mtm 同）──
    let mut trade_pnls_with_forced = trade_pnls.clone();
    if units != 0.0 {
        if let Some(last_bar) = bars.last() {
            let last_px = last_bar.close as f64 * config.tick.tick_size;
            if last_px > 0.0 {
                let pos_sign = units.signum();
                let px_exit_net = last_px * (1.0 - pos_sign * fee_rate);
                let forced_pnl = pos_sign * (px_exit_net - entry_cost) * units.abs();
                trade_pnls_with_forced.push(forced_pnl);
                if let Some(entry_bar) = pos_entry_bar.take() {
                    let exit_bar = n.saturating_sub(1);
                    let hold_bars = exit_bar.saturating_sub(entry_bar).max(1);
                    trades.push(metrics::TradeRecord {
                        entry_bar,
                        exit_bar,
                        hold_bars,
                        qty: units.abs(),
                        long: units > 0.0,
                        forced_close: true,
                    });
                }
            }
        }
    }

    let daily_returns = bar_returns(&equity_curve);
    FillOutput {
        equity_curve,
        daily_returns,
        trade_pnls_realized: trade_pnls,
        trade_pnls_with_forced,
        trades,
        n_orders: n_orders_executed,
    }
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
/// - `tw_state`（TW 守恒 + stage 单向）每 bar 经 tw_step 更新（ShortDiff / RecoverCapital）。
/// - `orders` 每 bar +1（订单计数推进）。
/// 闭环每步保持双账本不变量（见 closed_loop::transition 的 hybrid_step_preserves_* 测试）。
///
/// 返回闭环终态（`None` 仅当 bars 为空）。`i0` 进 ledger.i0 基线（NAV 绝对额由 fill 侧另算）。
///
/// ★认识论等级：闭环驱动 = **L1**（bit-exact 管线：Rust hybrid_step 逐 bar 推进与 Lean
/// assemblyStep 结构对齐 = 验证管线正确性，零信息增量）。真实数据回测的指标才是 L2。
pub fn run_closed_loop(bars: &[Bar], initial_nav: f64) -> Option<AssemblyState> {
    if bars.is_empty() {
        return None;
    }
    // 初始闭环态（i0 = NAV 取整作账本基线；账本是结构分量，NAV 绝对额 fill 侧另算）。
    let i0 = if initial_nav > 0.0 { initial_nav as i64 } else { 1 };
    // ★GAP3（codex 复审后修正诚实声明）：**已注资 campaign** 开局（非零 TW，Q=campaign_notional 名义
    // 敞口）。**EarningShares 在此 L0 同价闭环结构上不触达**（stage 恒 CostReduction）——TW 守恒
    // （=Q）⟹ holding↑则free↓；退本金前提 holding≥Q 时 free≤0 ⟹ cash-tight 退本金（w≤free）不可能
    // （见 `earning_shares_structurally_unreachable_from_campaign_tw_conserved`）。真达需 L2 价格升值让
    // 已实现利润进 TW（超出本 L0 模型事件集，照实 161——非 bug 非 placeholder）。注资仍必要：使 ledger/
    // 仓位在真实建仓（首 bar 现金充足 Δ=1）时真线程化（非 inert 空转），验证闭环双账本每 bar 真更新。
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

/// [`plan_and_fill_mtm`] 的完整产出（双口径 trade_pnls + 操作语义随机对照的输入）。
struct FillOutput {
    /// 逐 bar 归一化权益曲线（MtM 含浮盈）。
    equity_curve: Vec<f64>,
    /// 逐 bar returns（Sharpe/显著性输入）。
    daily_returns: Vec<f64>,
    /// 已实现成交盈亏（**不含**窗口终点强平浮盈，§3.4 bootstrap 输入）。
    trade_pnls_realized: Vec<f64>,
    /// 含浮盈成交盈亏（窗口终点强平未平仓持仓，编排者铁律「浮盈算上」）。
    trade_pnls_with_forced: Vec<f64>,
    /// 交易执行轨迹（[`metrics::TradeRecord`]，含强平笔；随机对照输入）。
    trades: Vec<metrics::TradeRecord>,
    /// 执行订单数（n_orders_executed > 0 ⟺ is_l2）。
    n_orders: usize,
}

/// ★ mark-to-market 闭环 plan+fill（Task A，消除开环单帧）。
///
/// 接收 `recognize` 批产的 `Vec<VoiceDecision>`，按 `exec_index` 分组，逐 bar 推进：
/// - 每到 `exec_index == i` 时，用当前账本 TW（`cash + units × px`）构造 `AccountState`，
///   以当前账本 NAV 重新 `plan_orders`（sizing 用真实账本，非固定初始 NAV）。
/// - `apply_order` 执行 fill（cash/units/entry_cost/voice_qty 四态同步更新，成本对称）。
/// - 推进权益曲线（bar 级，归一化为 ÷initial_nav）。
///
/// **对齐 Origin.TotalWealth**：`TW = free（cash）+ holding（units × current_price）`。
/// 同价操作（buy@px → holding=qty×px）NAV 中性（L0 结构恒等）；账本 NAV 随价格走势真变化。
///
/// **voice_qty 同步**：`plan_orders` 的 `act_state` 依赖 `voice_qty[depth]` 判开/平仓。
/// fill 后按 action 更新：Buy/Add → `voice_qty[depth] += qty`；Close/Reduce → 减。
/// 当前 recognize 产单声部 depth=0，voice_qty 维护精确（多声部时仍正确按 depth 分组）。
///
/// **★含浮盈口径（2026-06-28，编排者铁律「不把浮盈算上不合理」）**：窗口终点（末 bar）对所有
/// 未平仓持仓**强制平仓**（close 成交 + 卖出费），实现持仓期浮盈。`trade_pnls_realized`（不含
/// 强平）与 `trade_pnls_with_forced`（含强平）双口径并列产出。强平笔在 [`metrics::TradeRecord`]
/// 打 `forced_close=true`（统计功效门槛 n≥30 不计强平笔，避免偷过——见 runner L2/L3 测试）。
///
/// **★交易轨迹**：每笔完整开平记 [`metrics::TradeRecord`]（entry_bar/exit_bar/hold_bars/qty），
/// 作操作语义随机入场对照（§4 重写）的输入。v0 单声部多头全开全平，开-平配对唯一。
///
/// 返回 [`FillOutput`]（L2 等级，真实数据时 n_orders > 0 ⟺ is_l2 = true）。
fn plan_and_fill_mtm(
    decisions: &[VoiceDecision],
    bars: &[Bar],
    initial_nav: f64,
    config: &ThetaConfig,
) -> FillOutput {
    use super::super::strategy::exec::fill_bar_index;
    use super::super::strategy::voice;
    use super::super::strategy::voice::VoiceSide;

    let n = bars.len();
    let nav0 = if initial_nav > 0.0 { initial_nav } else { 1.0 };
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;

    // decisions 按 exec_index 分组（开仓侧延迟成交，spec:50）。
    // exec_index = fill_bar_index(signal_index, bars, config)，
    // 与 build_open/exit_order 内计算方式完全对齐（同一函数，零重算偏差）。
    // VoiceDecision 只有 signal_index，exec_index 需要预计算。
    let mut groups: Vec<Vec<&VoiceDecision>> = vec![Vec::new(); n];
    for d in decisions {
        if let Some(ei) = fill_bar_index(d.signal_index, bars, &config.exec) {
            if ei < n {
                groups[ei].push(d);
            }
        }
    }

    let mut cash: f64 = nav0;
    let mut units: f64 = 0.0;
    // entry_cost：含买入费的每单位加权平均成本基（成本对称，见 apply_order）。
    let mut entry_cost: f64 = 0.0;
    // voice_qty：各深度声部持仓手数，对齐 AccountState.voice_qty 语义。
    let mut voice_qty: Vec<u32> = vec![0u32; config.voice.max_depth as usize];
    // ★持仓声部台账（退出决策生成器的状态）：按 depth 记录入场快照（方向 + 止损价 + 决策快照），
    // 退出判定（§9 closePred）逐 bar 读它。None = 该 depth 空仓。
    let mut held: Vec<Option<HeldVoice>> = vec![None; config.voice.max_depth as usize];
    // ★退出 Close 订单的延迟成交队列（spec:50：退出触发在 bar i 检测，Close 订单延迟到
    // fill_bar_index(i) 成交——与开仓延迟语义一致，不在触发 bar 即时成交）。
    let mut exit_orders_at: Vec<Vec<VoiceDecision>> = vec![Vec::new(); n];

    let mut equity_curve = Vec::with_capacity(n);
    let mut trade_pnls: Vec<f64> = Vec::new();
    let mut n_orders_executed: usize = 0;
    // ★交易轨迹追踪（操作语义随机对照输入）：当前持仓的开仓 bar（None=空仓）+ 已完成轨迹。
    // v0 单声部多头全开全平 ⟹ 开仓记 entry_bar，平仓到 units=0 时配对产 TradeRecord。
    let mut trades: Vec<metrics::TradeRecord> = Vec::new();
    let mut pos_entry_bar: Option<usize> = None;

    for i in 0..n {
        let bar = &bars[i];
        let px = bar.close as f64 * config.tick.tick_size;

        // ── 退出 Close 订单成交（延迟队列，spec:50）：bar i 是某退出触发的 fill bar。 ──
        // 先于开仓处理（spec:54：退出/止损先于开仓）。
        if !exit_orders_at[i].is_empty() && !bar.untradable && px > 0.0 {
            let mut exit_decisions = std::mem::take(&mut exit_orders_at[i]);
            let current_nav = cash + units * px;
            let account = AccountState {
                nav: if current_nav > 0.0 { current_nav } else { nav0 },
                voice_qty: voice_qty.clone(),
            };
            let orders = strategy::plan_orders(&exit_decisions, bars, &account, config);
            // 退出决策全是 Close（exit=true）⟹ plan_orders 按 ConflictKey 终局键 depth 升序排
            // （exec::ConflictKey 第六键 depth；退出决策同信号 bar/level/class，仅 depth 区分）。
            // 故 Close 订单按 depth 升序与按 depth 升序的退出决策一一对应（depth 配对零歧义，
            // 不再用「恒取首决策」的近似——多声部退出也精确归 depth）。
            exit_decisions.sort_by_key(|d| d.depth);
            for (o, d) in orders.iter().zip(exit_decisions.iter()) {
                if o.qty > 0 && matches!(o.action, StrictAction::Close) {
                    let depth = d.depth as usize;
                    let units_before = units;
                    apply_order(o, px, fee_rate, &mut cash, &mut units, &mut entry_cost, &mut trade_pnls);
                    // ★轨迹配对（v1 方向中性）：平仓到 units=0 / 翻转 ⟹ 完整交易闭合（非强平，正常退出）。
                    track_position_transition(
                        &mut trades, &mut pos_entry_bar, units_before, units, i, false,
                    );
                    apply_voice_qty(&mut voice_qty, depth, o);
                    // 平仓后台账状态机：全平 ⟹ 清台账（held=None）；未全平（退化边界，build_exit_order
                    // 是全平 q，正常不发生）⟹ 保留台账但重置 exit_pending，允许下一 bar 重新评估退出
                    // （避免 exit_pending 永久阻塞该声部退出）。
                    if voice_qty.get(depth).copied().unwrap_or(0) == 0 {
                        if let Some(slot) = held.get_mut(depth) {
                            *slot = None;
                        }
                    } else if let Some(Some(h)) = held.get_mut(depth) {
                        h.exit_pending = false; // 未全平：解除 pending，下一 bar 可再触发
                    }
                    n_orders_executed += 1;
                }
            }
        }

        // 该 bar 到达 exec_index 的开仓 decisions：用当前账本 NAV（MtM）重新 plan_orders。
        let bar_decisions = &groups[i];
        if !bar_decisions.is_empty() && !bar.untradable && px > 0.0 {
            // 当前账本 NAV（mark-to-market）= free（cash）+ holding（units×px）。
            // 对齐 Origin.TotalWealth：TW = free + holding。
            let current_nav = cash + units * px;
            let account = AccountState {
                nav: if current_nav > 0.0 { current_nav } else { nav0 },
                voice_qty: voice_qty.clone(),
            };
            // 对该 exec_index 的全部 decisions 批量 plan（保持冲突排序语义）。
            let bar_decision_slice: Vec<VoiceDecision> =
                bar_decisions.iter().copied().cloned().collect();
            let orders = strategy::plan_orders(&bar_decision_slice, bars, &account, config);
            for o in &orders {
                if o.qty > 0 {
                    // voice_qty 同步：找首个方向匹配的 decision，取其 depth。
                    // decisions 与 orders 不一一对应（冲突排序可合并/删订单），
                    // 但同一 exec_index 内 depth=0 单声部时精确；多声部 depth 推断是近似。
                    let matched = bar_decisions.iter().find(|d| {
                        let side = voice::voice_side(d.root_side, d.depth);
                        matches!(
                            (o.action, side),
                            (StrictAction::Buy | StrictAction::Add, VoiceSide::Long)
                                | (StrictAction::Sell, VoiceSide::Short)
                                | (StrictAction::Close | StrictAction::Reduce, _)
                        )
                    });
                    let depth = matched.map(|d| d.depth as usize).unwrap_or(0);
                    let units_before = units;
                    apply_order(o, px, fee_rate, &mut cash, &mut units, &mut entry_cost, &mut trade_pnls);
                    // ★轨迹追踪（v1 方向中性）：units 有符号（正=多/负=空）。任一订单经
                    // apply_fill「先平后开」可能同时闭合反向旧仓 + 开新仓（翻转）⟹ 用 units 跨 0
                    // 行为统一追踪：(a) |units| 由非零回 0 / 跨 0 翻转 ⟹ 闭合旧方向 trade（配对
                    // pos_entry_bar，方向 = units_before.signum()）；(b) units 由 0 进入非零 /
                    // 翻转后新方向 ⟹ 记新 entry_bar。
                    track_position_transition(
                        &mut trades, &mut pos_entry_bar, units_before, units, i, false,
                    );
                    apply_voice_qty(&mut voice_qty, depth, o);
                    // ★开仓订单 ⟹ 记入持仓台账（退出生成器读它）。止损价由入场决策的
                    // stop_in + 方向算出（与 build_open_order 内 structural_stop 同一函数）。
                    if matches!(o.action, StrictAction::Buy | StrictAction::Sell | StrictAction::Add) {
                        if let Some(d) = matched {
                            record_held_voice(&mut held, d);
                        }
                    }
                    n_orders_executed += 1;
                }
            }
        }

        // ── ★退出决策生成器（§9 closePred，本轮真根因修复核心）：逐持仓声部检查关闭谓词。 ──
        // 持仓后逐 bar 检查 [止损触及 / 反向 BSP / RiskClose]，触发则构造 exit=true 决策入
        // 延迟队列（spec:50：Close 订单延迟到 fill_bar_index(i) 成交）。对齐
        // Origin.SubVoiceOpenClose.closePred（X = ¬ParentValid ∨ χ^{σ_p} ∨ Stop ∨ RiskClose）。
        if !bar.untradable && px > 0.0 {
            // 当前账本权益（RiskClose 的 GlobalRiskClose 判据需要，Origin.RiskProj）。
            let equity_now = cash + units * px;
            for depth in 0..held.len() {
                let hv = match held[depth] {
                    Some(hv) => hv,
                    None => continue, // 空仓声部，无退出可言
                };
                if hv.exit_pending {
                    continue; // 退出已触发，Close 在延迟队列待成交——不重复入队（关闭一次触发一次平仓）
                }
                if voice_qty.get(depth).copied().unwrap_or(0) == 0 {
                    continue; // 台账方向有但手数 0（已平），跳过
                }
                if let Some(exit_d) =
                    exit_decision_for(&hv, depth, bar, i, &groups, equity_now)
                {
                    // Close 订单延迟成交（spec:50）：触发 bar i → fill bar = fill_bar_index(i)。
                    if let Some(fi) = fill_bar_index(i, bars, &config.exec) {
                        if fi < n {
                            exit_orders_at[fi].push(exit_d);
                            // 标记退出 pending（避免 fill 前重复触发同一止损/反向）。
                            if let Some(slot) = held.get_mut(depth) {
                                if let Some(ref mut h) = slot {
                                    h.exit_pending = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        // 权益曲线（mark-to-market，归一化 ÷nav0）。
        let equity = (cash + units * px) / nav0;
        equity_curve.push(equity);
    }

    // ── ★窗口终点强制平仓（含浮盈口径，编排者铁律「不把浮盈算上不合理」；v1 方向中性）──
    // 已实现口径 trade_pnls（循环内 apply_fill push）**不含**最后一段未平仓持仓的浮盈；
    // 含浮盈口径在此对剩余 units 按末 bar close 强平（含费），实现持仓期浮盈。剩余持仓可多可空
    // （units≠0），强平 PnL 方向中性：多头 = proceeds(扣卖出费)−成本基；空头 = 成本基(开空净收)−支出(含买入费)。
    // 双口径并列：trade_pnls_realized（不含强平）+ trade_pnls_with_forced（含强平）。
    let mut trade_pnls_with_forced = trade_pnls.clone();
    if units != 0.0 {
        // 末 bar 的 close 作强平价（含浮盈口径——窗口边界 mark-to-market 实现）。
        if let Some(last_bar) = bars.last() {
            let last_px = last_bar.close as f64 * config.tick.tick_size;
            if last_px > 0.0 {
                // 强平 PnL（成本对称，多空统一）：pos_sign=units.signum()。
                // 平仓净价 px_exit_net：多 px(1−fee)，空 px(1+fee)。PnL = sign×(px_exit_net−entry_cost)×|units|。
                let pos_sign = units.signum();
                let px_exit_net = last_px * (1.0 - pos_sign * fee_rate);
                let forced_pnl = pos_sign * (px_exit_net - entry_cost) * units.abs();
                trade_pnls_with_forced.push(forced_pnl);
                // 强平笔轨迹（forced_close=true，统计功效门槛不计——见 L2/L3 测试）。方向 = units 符号。
                if let Some(entry_bar) = pos_entry_bar.take() {
                    let exit_bar = n.saturating_sub(1);
                    let hold_bars = exit_bar.saturating_sub(entry_bar).max(1);
                    trades.push(metrics::TradeRecord {
                        entry_bar,
                        exit_bar,
                        hold_bars,
                        qty: units.abs(),
                        long: units > 0.0,
                        forced_close: true,
                    });
                }
            }
        }
    }

    let daily_returns = bar_returns(&equity_curve);
    FillOutput {
        equity_curve,
        daily_returns,
        trade_pnls_realized: trade_pnls,
        trade_pnls_with_forced,
        trades,
        n_orders: n_orders_executed,
    }
}

/// ★交易轨迹配对（平仓事件 → [`metrics::TradeRecord`]）。
///
/// ★持仓状态转移轨迹追踪（v1 方向中性，替代 long-only `track_close_to_trade`）。
///
/// `units` 有符号（正=多/负=空/0=空仓）。一次成交（`units_before → units_after`）可能：
/// - **纯开仓**（before=0, after≠0）：记新 `pos_entry_bar = Some(exit_bar)`（此 bar 为入场 bar）。
/// - **纯平仓**（before≠0, after=0）：配对 `pos_entry_bar` 产 TradeRecord（方向 = before.signum()：
///   before>0 ⟹ long=true 平多；before<0 ⟹ long=false 平空），清 entry_bar。
/// - **翻转**（before·after<0，先平后开同一笔）：先配对旧方向 trade（方向 = before.signum()，
///   qty = |before|），再记新 entry_bar（新方向持仓从此 bar 入场）。
/// - **同向加/减仓未到 0**（before·after>0）：entry_bar 不变（延续首次入场，v0 单标量近似）。
///
/// `forced` 标记窗口终点强平（不计 n_trades≥30 统计功效门槛）。qty = 平掉的绝对手数 = |before|
/// （翻转/全平时平掉全部旧仓；v0 build_exit_order 全平 ⟹ 配对唯一）。
fn track_position_transition(
    trades: &mut Vec<metrics::TradeRecord>,
    pos_entry_bar: &mut Option<usize>,
    units_before: f64,
    units_after: f64,
    exit_bar: usize,
    forced: bool,
) {
    // ★显式三态符号（−1/0/+1）：f64::signum 对 0.0 返回 +1.0（不返回 0），不能用于判持仓有无。
    let sign = |x: f64| -> i8 {
        if x > 0.0 {
            1
        } else if x < 0.0 {
            -1
        } else {
            0
        }
    };
    // 闭合旧仓：before≠0 且符号改变（到 0 / 翻转）⟹ 配对产 trade。
    let closed = sign(units_before) != 0 && sign(units_before) != sign(units_after);
    if closed {
        if let Some(entry_bar) = pos_entry_bar.take() {
            let hold_bars = exit_bar.saturating_sub(entry_bar).max(1);
            trades.push(metrics::TradeRecord {
                entry_bar,
                exit_bar,
                hold_bars,
                qty: units_before.abs(), // 平掉的绝对手数 = |平仓前持仓|
                long: units_before > 0.0, // 方向 = 平仓前持仓方向（多/空）
                forced_close: forced,
            });
        }
    }
    // 记新入场：after≠0 且 (纯开仓 before=0 / 翻转后新方向) ⟹ 此 bar 为新仓入场 bar。
    if units_after != 0.0 && (units_before == 0.0 || closed) {
        *pos_entry_bar = Some(exit_bar);
    }
}

/// voice_qty 同步（fill 后更新；depth 超界跳过，诚实边界不应发生）。
///
/// ★v1 方向中性：`voice_qty` 是该 depth 声部的**绝对持仓手数**（`act_state` 判 open/hold/close
/// 用，与持仓方向解耦——方向由 `HeldVoice.side` 记）。在 `plan_and_fill_mtm` 数据流中 plan_orders
/// 的 build_open_order 产 **Buy（开多）/ Sell（开空）**、build_exit_order 产 **Close（平仓）**——
/// 故 **Buy/Add/Sell = 开仓侧（绝对手数增）**，**Close/Reduce = 平仓侧（绝对手数减）**。
/// 修复前 long-only 把 Sell 当减仓 ⟹ Short 根开空后 voice_qty 恒 0 ⟹ act_state 恒 Open ⟹ 重复
/// 开空不止（v1 缺陷根因之一）。
fn apply_voice_qty(voice_qty: &mut [u32], depth: usize, o: &Order) {
    if let Some(slot) = voice_qty.get_mut(depth) {
        match o.action {
            // 开仓侧（开多 Buy / 开空 Sell / 加仓 Add）⟹ 绝对手数增。
            StrictAction::Buy | StrictAction::Add | StrictAction::Sell => {
                *slot = slot.saturating_add(o.qty as u32);
            }
            // 平仓侧（平多/平空 Close / 减仓 Reduce）⟹ 绝对手数减。
            StrictAction::Close | StrictAction::Reduce => {
                *slot = slot.saturating_sub(o.qty as u32);
            }
            StrictAction::Hold | StrictAction::Wait => {}
        }
    }
}

/// fill 模拟（Θ_exec 基础形态，reference-theta-v0.md:49-54）。
///
/// **可独立于 classify/plan_orders 内部逻辑实装**——只消费 `Vec<Order>`（携带 `exec_index`/
/// `action`/`qty`）。返回 `(权益曲线, 日 returns, 每笔平仓盈亏)`。
///
/// ## 当前实装范围（诚实声明）
///
/// - 开仓（Buy/Sell）/ 平仓（Close）/ 加减仓（Add/Reduce）按 `exec_index` bar 的 close
///   成交，扣 commission+slippage 费用（`ExecConfig`，bp/side）。
/// - **待补全**（与 strategy 订单语义对齐后，标注非 workaround 而是增量）：
///   - 止损成交（reference:52，long open<止损按 open 出否则 low 触及按 stop）——需 Order
///     携带止损价，当前 [`Order`] 无该字段（待 strategy 定订单是否含止损腿）。
///   - 冲突顺序（reference:54，止损先于开仓 / 高 level 先 / 1/2/3 类序）——需多 level
///     订单流，当前 classify 返回空无 level 信息。
///   - 延迟语义：`Order.exec_index` 已是"执行延迟后的成交 bar"（types.rs:218），fill 直接
///     用之，延迟在 strategy 层计算（此处不重复延迟）。
///
/// ## 资金口径（绝对 NAV，对齐 strategy `AccountState.nav`）
///
/// `Order.qty` 是 strategy sizing 算出的**整数 lot**（绝对手数，reference:47），lot×price =
/// 绝对名义额。fill 用**绝对 NAV**（`initial_nav`）记账——现金/市值都是绝对额，权益曲线
/// 归一化（÷initial_nav）后返回（收益率口径，metrics 用率不用绝对额）。这保留 lot sizing
/// 的绝对名义语义（区别于"归一化 cash=1 + units=qty"的错误口径——后者在 price≫NAV 时永远
/// 买不起 1 lot）。
///
/// **边界条件**：`orders` 为空（当前阻塞态）⇒ 现金不动，权益曲线 = 持平（全 1.0），
/// trade_pnls 为空。这正确反映"无 Θ 决策"，**不是** bug。
fn simulate_fills(
    bars: &[Bar],
    orders: &[Order],
    initial_nav: f64,
    config: &ThetaConfig,
) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let n = bars.len();
    let mut equity_curve = Vec::with_capacity(n);
    let mut trade_pnls = Vec::new();

    // 费用率（bp/side → 比率）。commission + slippage（tax=0 默认）。
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;

    // 按 exec_index 建索引（同 bar 多订单按出现顺序）。
    // 简单线性扫描：订单按 exec_index 排序后与 bar 主循环对齐。
    let mut sorted_orders: Vec<&Order> = orders.iter().collect();
    sorted_orders.sort_by_key(|o| o.exec_index);

    let nav = if initial_nav > 0.0 { initial_nav } else { 1.0 };
    let mut cash: f64 = nav; // 绝对现金（初始 = NAV）
    let mut units: f64 = 0.0; // 持仓 lot 数（正=多；做空腿待 strategy 订单语义定）
    let mut entry_cost: f64 = 0.0; // 含买入费的每单位成本基（成本对称）
    let mut ord_idx = 0usize;

    for i in 0..n {
        let bar = &bars[i];
        // 成交价 = 该 bar 的 close（f64 域；tick→f64 用 tick_size 还原，比率口径无关）。
        let px = bar.close as f64 * config.tick.tick_size;

        // 处理 exec_index == i 的订单（不可交易 bar 上不成交，reference:53）。
        while ord_idx < sorted_orders.len() && sorted_orders[ord_idx].exec_index == i {
            let o = sorted_orders[ord_idx];
            ord_idx += 1;
            if bar.untradable || px <= 0.0 || o.qty <= 0 {
                continue; // 不可交易 / 非法 qty ⇒ 跳过（reference:47 qty<=0 不交易）。
            }
            apply_order(o, px, fee_rate, &mut cash, &mut units, &mut entry_cost, &mut trade_pnls);
        }

        // MtM 权益 = 现金 + 持仓市值，归一化（÷NAV → 收益率口径，初始=1.0）。
        let equity = (cash + units * px) / nav;
        equity_curve.push(equity);
    }

    // 日 returns：1min bar 权益曲线聚合到日。当前骨架用 bar-级 returns 作占位
    // （runner 调用方按 dates 聚合到日时替换为日聚合——见 §3.1"按 1min→日聚合"）。
    // 诚实标注：bar-级 returns 的年化基数 ≠ 日 returns，Sharpe 数值在日聚合接通前不是
    // 协议口径。日聚合需要 dates（Dataset.dates），属指标精确化（非阻塞引擎，待接）。
    let daily_returns = bar_returns(&equity_curve);

    (equity_curve, daily_returns, trade_pnls)
}

/// 应用单个订单到仓位（**方向中性账本**，开/平/加/减仓 + 费用，多空对称）。
///
/// ★v1 做空腿（操作语义补全，对齐 canonical FULL §10「根声部双向全定义状态机」+ §13「有符号
/// 名义头寸 n_v=σ_v·M·P·q」+ Origin.TotalWealth `TW=cash+units×price`）：`units` **有符号**
/// （正=多头，负=空头，0=空仓，对齐 canonical σ_r∈{-1,0,+1}×绝对手数）。订单方向（Buy/Add 增
/// units，Sell/Reduce/Close 减 units）统一映射到有符号 units 增减，**多空完全对称**（删 v0 退化
/// 的 long-only `close_long`，no-patch 重写）。
///
/// ★先平后开（canonical FULL §20「撤单→先平后开」执行序的 fill 层兑现）：任一订单先**平掉反向
/// 持仓部分**（实现 PnL 入 trade_pnls），再用剩余 qty **开新方向仓**（更新成本基）。这使翻转
/// （平多→开空 / 平空→开多）成本基语义无歧义——不存在"多空混合成本基"。
///
/// ★成本对称（缺陷③修复保留，多空两侧统一）：`entry_cost` 是当前持仓**每单位含费成本基**——
/// 多头 = 买入均价含买入费（开仓 cash−含费 cost）；空头 = 卖出均价**扣卖出费后**净收（开空
/// cash+净 proceeds）。平仓 PnL 两侧对称：平多 PnL=平仓proceeds(扣卖出费)−成本基(含买入费)；
/// 平空 PnL=开空成本基(扣卖出费净收)−平空支出(含买入费)。
///
/// ★现金约束（多空对称）：开多需 `cash ≥ 含费 cost`（现金买入）；开空收到卖出 proceeds（cash+），
/// 无需预付现金（保证金约束属 canonical §11/§14 K_Θ 风险可行集，v0 未建模——诚实有效域 L0：
/// 做空保证金/借券成本未建模，标注非全 canonical §11，是 σ 双向 + TW 账本的最小兑现）。
fn apply_order(
    o: &Order,
    px: f64,
    fee_rate: f64,
    cash: &mut f64,
    units: &mut f64,
    entry_cost: &mut f64,
    trade_pnls: &mut Vec<f64>,
) {
    let qty = o.qty as f64;
    // 订单 → (有符号成交方向 δ, 是否纯平仓 close_only)。
    // ★信号成交（Buy/Add/Sell）：可「先平后开」翻转（开多 δ+1 / 开空 δ−1）。
    // ★纯平仓（Close/Reduce，v1 做空腿方向感知）：平掉当前持仓——平多=卖（δ−1），平空=买（δ+1），
    //   **只平不反向开**（close_only=true，剩余 qty 超持仓时不借机开反向仓）。build_exit_order 对
    //   多空两腿都产 `StrictAction::Close`（不带方向），故成交方向由**当前持仓符号**决定；空仓 ⟹ 无操作。
    let (delta, close_only): (f64, bool) = match o.action {
        StrictAction::Buy | StrictAction::Add => (1.0, false),
        StrictAction::Sell => (-1.0, false),
        StrictAction::Reduce | StrictAction::Close => {
            if *units > 0.0 {
                (-1.0, true) // 持多 ⟹ 卖出平多
            } else if *units < 0.0 {
                (1.0, true) // 持空 ⟹ 买回平空
            } else {
                return; // 空仓无仓可平
            }
        }
        StrictAction::Hold | StrictAction::Wait => return, // 不动
    };
    apply_fill(delta, qty, close_only, px, fee_rate, cash, units, entry_cost, trade_pnls);
}

/// **方向中性成交**（v1 做空腿核心，先平后开）：有符号方向 `delta`（+1 买/−1 卖）× 绝对手数 `qty`。
///
/// 分两段（canonical §20 先平后开）：
/// 1. **平反向**：若新成交方向与当前持仓反向（`units·delta < 0`），先平掉 `min(qty, |units|)` 手，
///    实现 PnL 入 trade_pnls（多空 PnL 公式对称，见下），cash 反向于 units 变化。
/// 2. **开新仓**：剩余 `qty − 已平` 手按 `delta` 方向开仓，加权平均更新含费成本基。
///
/// PnL 口径（成本对称，多空统一）：
/// - 平多（delta=−1，units>0）：proceeds=平仓 qty×px×(1−fee)，cost_basis=qty×entry_cost（开仓含买入费）⟹ PnL=proceeds−cost_basis。
/// - 平空（delta=+1，units<0）：开空成本基=qty×entry_cost（开空扣卖出费净收），平空支出=qty×px×(1+fee)（买回含买入费）⟹ PnL=成本基−支出。
///   两式统一为 `PnL = sign·(entry_cost − px·(1+sign·fee_factor))`，由下方有符号代数自动覆盖。
fn apply_fill(
    delta: f64,
    qty: f64,
    close_only: bool,
    px: f64,
    fee_rate: f64,
    cash: &mut f64,
    units: &mut f64,
    entry_cost: &mut f64,
    trade_pnls: &mut Vec<f64>,
) {
    if qty <= 0.0 || px <= 0.0 {
        return;
    }
    let mut remaining = qty;

    // ── 段 1：平反向持仓（units 与 delta 反向 ⟹ 本次成交先减仓）。 ──
    if *units * delta < 0.0 {
        let close_qty = remaining.min(units.abs());
        if close_qty > 0.0 {
            // 平仓现金流：delta=+1（买回平空）cash 减 qty×px×(1+fee)；delta=−1（卖出平多）cash 加 qty×px×(1−fee)。
            // 统一：cash += −delta × qty × px × (1 + delta×fee_rate)。
            let cash_flow = -delta * close_qty * px * (1.0 + delta * fee_rate);
            // PnL = 持仓方向收益。持仓方向 sign = units.signum()（多=+1，空=−1）。
            // 平多（持多，sign+1）：PnL = proceeds − cost = (qty×px×(1−fee)) − (qty×entry_cost)。
            // 平空（持空，sign−1）：PnL = 开空净收 − 平空支出 = (qty×entry_cost) − (qty×px×(1+fee))。
            // 统一：PnL = sign × (px_exit_net − entry_cost) × qty，其中 px_exit_net 对多=px(1−fee)，对空=px(1+fee)。
            let pos_sign = units.signum();
            let px_exit_net = px * (1.0 - pos_sign * fee_rate);
            let pnl = pos_sign * (px_exit_net - *entry_cost) * close_qty;
            *cash += cash_flow;
            // 平反向：delta 与持仓异号，units += delta×close_qty 使 |units| 减小（向 0 收敛）。
            // 平多（units=+N, delta=−1）⟹ N+(−1)·N=0；平空（units=−N, delta=+1）⟹ −N+1·N=0。
            *units += delta * close_qty;
            trade_pnls.push(pnl);
            remaining -= close_qty;
            // 全平 ⟹ 成本基归零（无持仓）；未全平 ⟹ 同方向剩余成本基不变（同价同费基）。
            if *units == 0.0 {
                *entry_cost = 0.0;
            }
        }
    }

    // ── 段 2：开新方向仓 / 同向加仓（剩余 qty 按 delta 开仓）。纯平仓 ⟹ 不开新仓。 ──
    if remaining > 0.0 && !close_only {
        // 开仓现金流：开多（delta+1）cash 减 qty×px×(1+fee)（含买入费）；
        // 开空（delta−1）cash 加 qty×px×(1−fee)（卖出净收，扣卖出费）。
        // 统一：cash += −delta × qty × px × (1 + delta×fee_rate)。
        let cash_flow = -delta * remaining * px * (1.0 + delta * fee_rate);
        // 现金约束：开多需现金充足（cash ≥ 买入含费成本）；开空收现金（无预付，保证金 v0 未建模）。
        let need_cash = delta > 0.0; // 仅开多需现金
        let cost = remaining * px * (1.0 + fee_rate);
        if need_cash && *cash < cost {
            return; // 现金不足，不开多（对齐 v0 现金约束；开空无此约束）
        }
        // 每单位含费成本基（多=买入均价含买入费；空=卖出均价扣卖出费净收）。
        // 单笔成交成本基 = px × (1 + delta×fee_rate)：多 px(1+fee)，空 px(1−fee)。
        let unit_cost = px * (1.0 + delta * fee_rate);
        let old_units_abs = units.abs();
        let new_units = *units + delta * remaining;
        let new_units_abs = new_units.abs();
        if new_units_abs > 0.0 {
            // 加权平均含费成本基（同方向加仓：旧成本基×旧手数 + 本次成本基×本次手数 ÷ 新手数）。
            *entry_cost = (*entry_cost * old_units_abs + unit_cost * remaining) / new_units_abs;
        }
        *units = new_units;
        *cash += cash_flow;
    }
}

/// bar-级 returns（占位；日聚合接通前的 L1 口径）。
fn bar_returns(equity: &[f64]) -> Vec<f64> {
    if equity.len() < 2 {
        return Vec::new();
    }
    equity
        .windows(2)
        .map(|w| if w[0] > 0.0 { w[1] / w[0] - 1.0 } else { 0.0 })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::types::Bar;

    fn mk_bar(idx: usize, close_tick: i64, untradable: bool) -> Bar {
        Bar {
            source_index: idx,
            timestamp: idx as i64,
            open: close_tick,
            high: close_tick,
            low: close_tick,
            close: close_tick,
            volume: 1,
            untradable,
        }
    }

    /// 无结构数据 → 空订单流（**诚实结果，非阻塞态**）。recognize 已接通（2026-06-26）；
    /// 单调上涨数据无顶底分型交替 ⟹ 无笔 ⟹ 无中枢 ⟹ 无第三类买卖点 ⟹ 空 decisions ⟹
    /// 空订单。这验证管线 L1 串通 + "无缠论结构 ⇒ 无 Θ 决策"的正确退化（不是引擎缺陷）。
    #[test]
    fn structureless_data_yields_empty_orders() {
        let config = ThetaConfig::default();
        // 100 根单调上涨 bar（无顶底交替 ⟹ 无缠论结构）。
        let bars: Vec<Bar> = (0..100).map(|i| mk_bar(i, 1000 + i as i64, false)).collect();
        let ds = Dataset {
            symbol: "TEST".to_string(),
            bars,
            dates: (0..100).map(|i| format!("2024-01-{:02} 00:00:00", (i % 28) + 1)).collect(),
            bar_seconds: 60,
        };
        let res = run_theta_v0(&ds, &config, 1.0, 1.0e6);
        // 无结构 ⟹ 订单空 ⟹ 非 L2 ⟹ 无交易（正确退化，非阻塞）。
        assert_eq!(res.n_orders, 0, "单调数据无缠论结构 ⇒ recognize 产空 decisions ⇒ 空订单");
        assert!(!res.is_l2, "无订单流 ⇒ 非 L2（无 Θ 决策被检验）");
        assert_eq!(res.metrics.n_trades, 0, "无订单 ⇒ 无交易");
        // L1 管线跑通：buy&hold 对照算出（首 1000 末 1099 → ~9.9%）。
        assert!(res.metrics.bh_return > 0.0, "L1：buy&hold 对照正确计算（上涨数据）");
        // is_l2 与订单一致性（诚实标注不变量）。
        assert_eq!(res.is_l2, res.n_orders > 0, "is_l2 ⟺ 订单非空");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★★塔导出桥 (iii) 布线段：run_theta_v0_pi 七链 π_Θ 生产 runner（接 Nautilus 实装）
    // ──────────────────────────────────────────────────────────────────────

    use super::super::super::classifier::{Classification, LevelState};
    use super::super::super::classifier::bsp::BspPoint;
    // BspBits/Center 由本测试模块下方 `use ...types::{BspBits, Center, Tick}` 模块级导入提供。

    /// 价格 ~$100（tick_size=1e-8 ⟹ close_tick=1e10）的可交易 bar，逐 bar 微涨（产 PnL）。
    fn px100_bar(i: usize) -> Bar {
        let c = 10_000_000_000i64 + (i as i64) * 10_000_000; // px ≈ 100 → 100.x
        mk_bar(i, c, false)
    }

    /// L0 一类买点（source_index=si；pivot_low 远低于入场价 ⟹ 止损不触及，持仓延续）。
    fn buy1_at(si: usize) -> BspPoint {
        BspPoint {
            source_index: si,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 9_000_000_000, // px 90 < 入场 100 ⟹ 止损在下方不触及
            pivot_high: 0,
            center: Some(Center { zd: 9_500_000_000, zg: 10_500_000_000, dd: 9_000_000_000, gg: 11_000_000_000, start_index: 0, end_index: si }),
            struct_break_dir: None,
        }
    }

    /// ★关键测试：`pi_theta_fill_loop` 端到端产 trades（非空）——注入合成 classify 闭包（隔离 fill
    /// 机制，绕过 classify→bsp，由 run_theta_v0 真实数据另测）。买点候选 → 开仓 → 窗口终点强平 ⟹ ≥1 笔。
    #[test]
    fn run_theta_v0_pi_loop_produces_trades_nonempty() {
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        // ★确认-bar 部署语义：L0 一类买点 source_index=3，但**回溯确认**——直到 bar 7 才被结构确认
        // 入前缀塔（合成闭包 i<7 返回空分类，i≥7 返回含 buy@3 的分类）。旧 source_index==i 切口径
        // 会在 bar 3 切（彼时前缀塔尚无该点）⟹ 永远切不到 ⟹ 零订单；确认-bar 部署在 bar 7（确认时点，
        // source_index=3≤7=因果）首次 diff 出该新确认买卖点 → 部署。+ 空塔（候选父=∂ ⟹ Ambient，足以
        // 验 fill 机制；σ_p 由 tower 专测）。
        let classification = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3)]), ..Default::default() }],
        };
        let fill = pi_theta_fill_loop(
            // ★工位 4g：第三元素 = 塔代次。合成闭包每 bar 用单调 `i as u64`（保守——每 bar 视作塔变 ⟹
            // 走 TreeKey fallback，bit-exact；空塔下 TreeKey 亦平凡）。
            |i| {
                if i >= 7 {
                    (classification.clone(), Vec::new(), i as u64)
                } else {
                    (Classification::default(), Vec::new(), i as u64)
                }
            },
            &bars,
            1.0e6,
            &config,
            None, // χ≡1（无过滤，本测试验 fill 机制）
        );
        // 七链：买点 bar 7 确认 → 确认-bar diff 部署 → 开 Long → 订单 → fill → 窗口终点强平 ⟹ trades≥1。
        assert!(fill.n_orders > 0, "π_Θ 确认-bar 部署买点 ⟹ 产订单（n_orders>0），实得 {}", fill.n_orders);
        assert!(!fill.trades.is_empty(), "开仓 + 窗口终点强平 ⟹ ≥1 笔交易轨迹，实得 {}", fill.trades.len());
        assert!(fill.trade_pnls_with_forced.iter().all(|p| p.is_finite()), "PnL 有限");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★★χ_t 阈值过滤对比（task #41 acc-chi-theta-filter 可证伪核心）：
    //  χ≡1 全覆盖 vs χ=1[μ>θ] 阈值过滤——证明过滤确实改变交易集（非 no-op）。
    // ──────────────────────────────────────────────────────────────────────

    /// 合成 classify 闭包工厂：买点 buy1@3 在 bar≥7 确认（同 `..produces_trades_nonempty`），空塔。
    fn buy1_at3_confirmed_at7() -> impl Fn(usize) -> (Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>, u64) {
        let classification = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3)]), ..Default::default() }],
        };
        move |i| {
            if i >= 7 {
                (classification.clone(), Vec::new(), i as u64)
            } else {
                (Classification::default(), Vec::new(), i as u64)
            }
        }
    }

    /// ★★χ≡1 vs χ=1[μ>θ] 对比（acc-chi-theta-filter 可证伪核心）：买点 z 的 μ≤θ ⟹ χ 滤掉它 ⟹
    /// 交易集**收缩**（χ≡1 有交易，χ=1[μ>θ] 无）。这证明 χ 过滤**确实改变交易集**（非 no-op）。
    ///
    /// 构造：买点 buy1@3（z=level0,Long,buy1,Ambient/Root）的 μ=−50<θ=0 ⟹ χ_t(γ)=0 ⟹ 滤掉。
    /// 认识论 L1（注入合成 μ 表验证选择器逻辑生效——过滤改变交易集；非 L2 alpha，μ 是合成的）。
    #[test]
    fn chi_filter_shrinks_trade_set_vs_full_coverage() {
        use super::super::mu_estimator::{MuClass, MuEstimator, MuObservation, PositionState};
        use super::super::super::types::BspBits;

        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();

        // χ≡1 全覆盖（chi=None）：买点开仓 ⟹ 有交易。
        let fill_full = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
        assert!(fill_full.n_orders > 0, "χ≡1：买点开仓 ⟹ 有订单（基线），实得 {}", fill_full.n_orders);

        // χ=1[LCB(μ)>θ]：买点 z 的 LCB(μ)=−50≤θ=0 ⟹ χ=0 ⟹ 滤掉该候选 ⟹ 无开仓订单。
        // z = 买点 buy1@3 的全互斥分类（level0,Long,buy1,父向0/Ambient,Root；空塔 ⟹ Ambient/Root）。
        // LCB 升级：n≥2 让 mu_lcb 有定义（否则 n=1 因"无 LCB 证据"被滤，机制不同——见任务 §3）。
        // b2（task #83）：fill loop 的 χ 门经 z_of_candidate 查 μ ⟹ 查询 z 携带 H=Some(role.h)。
        // 本例 buy1@3 无同级前兄弟 ⟹ H=First。手建 est 的 z 须同口径（否则 None vs Some(First) 桶不命中）。
        let z_buy = MuClass {
            horizontal: Some(crate::theta_v0::strategy::coverage::Horizontal::First),
            ..MuClass::from_certificate(0, 1, BspBits { buy1: true, ..Default::default() }, 0, PositionState::Root)
        };
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z_buy, x_gamma: -50.0 });
        est.observe(MuObservation { class: z_buy, x_gamma: -50.0 }); // n=2,μ=−50,std=0 ⟹ LCB=−50≤θ=0
        let chi = ChiFilterCtx { est: &est, theta: 0.0, z_alpha: 0.0, treat_empty_as_pass: false, shrink_tau_sq: None };
        let fill_chi = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, Some(chi));

        // ★可证伪核心：χ 过滤改变交易集——μ≤θ 的买点被滤 ⟹ 订单数收缩（严格 <）。
        assert!(
            fill_chi.n_orders < fill_full.n_orders,
            "χ=1[μ>θ] 滤掉 μ≤θ 买点 ⟹ 订单数收缩：χ过滤 {} < χ≡1 {}（若相等=过滤无效=fail）",
            fill_chi.n_orders, fill_full.n_orders
        );
        // 本例 z 唯一买点被滤 ⟹ χ 路径无开仓（只剩窗口终点无持仓 ⟹ 无强平笔）。
        assert_eq!(fill_chi.n_orders, 0, "唯一买点被 χ 滤掉 ⟹ 无订单");
        assert!(fill_chi.trades.is_empty(), "无开仓 ⟹ 无交易轨迹");
    }


    /// ★χ μ>θ 放行（对偶）：买点 z 的 μ=+50>θ=0 ⟹ χ=1 ⟹ 保留 ⟹ 与 χ≡1 同交易集（bit-exact）。
    /// 证明 χ 过滤**只滤 μ≤θ**——μ>θ 的候选不受影响（过滤是边际门，非全杀）。
    #[test]
    fn chi_filter_passes_positive_mu_same_as_full() {
        use super::super::mu_estimator::{MuClass, MuEstimator, MuObservation, PositionState};
        use super::super::super::types::BspBits;

        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();

        let fill_full = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);

        // 买点 z 的 LCB(μ)=+50>θ=0 ⟹ χ=1 ⟹ 放行（与 χ≡1 同）。
        // LCB 升级：需 n≥2 让 mu_lcb 有定义（n=1 方差未定义 ⟹ mu_lcb=None ⟹ 走 empty 分支）。
        // 喂 [50,50] ⟹ mean=50,std=0 ⟹ LCB=50−z_alpha·0=50>θ（z_alpha=0 时退化裸 μ=50）。
        // b2（task #83）：fill loop 的 χ 门经 z_of_candidate 查 μ ⟹ 查询 z 携带 H=Some(role.h)。
        // 本例 buy1@3 无同级前兄弟 ⟹ H=First。手建 est 的 z 须同口径（否则 None vs Some(First) 桶不命中）。
        let z_buy = MuClass {
            horizontal: Some(crate::theta_v0::strategy::coverage::Horizontal::First),
            ..MuClass::from_certificate(0, 1, BspBits { buy1: true, ..Default::default() }, 0, PositionState::Root)
        };
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z_buy, x_gamma: 50.0 });
        est.observe(MuObservation { class: z_buy, x_gamma: 50.0 }); // n=2 ⟹ LCB 有定义；μ=50>θ=0
        let chi = ChiFilterCtx { est: &est, theta: 0.0, z_alpha: 0.0, treat_empty_as_pass: false, shrink_tau_sq: None };
        let fill_chi = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, Some(chi));

        // μ>θ 放行 ⟹ 与 χ≡1 同订单数/交易数（过滤只滤 μ≤θ，不动 μ>θ）。
        assert_eq!(fill_chi.n_orders, fill_full.n_orders, "μ>θ 放行 ⟹ 订单数同 χ≡1");
        assert_eq!(fill_chi.trades.len(), fill_full.trades.len(), "交易轨迹同 χ≡1");
    }

    /// ★run_theta_v0_pi 全链端到端（bars → parse → classify → π_Θ → fill）不破——结构性数据
    /// 无缠论结构 ⟹ classify 产空 bsp ⟹ 无订单（诚实退化，非 bug；与 run_theta_v0 同口径）。
    #[test]
    fn run_theta_v0_pi_structureless_runs_clean() {
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..100).map(|i| mk_bar(i, 1000 + i as i64, false)).collect();
        let ds = Dataset {
            symbol: "TEST".to_string(),
            bars,
            dates: (0..100).map(|i| format!("2024-01-{:02} 00:00:00", (i % 28) + 1)).collect(),
            bar_seconds: 60,
        };
        let res = run_theta_v0_pi(&ds, &config, 1.0, 1.0e6);
        // 单调数据无缠论结构 ⟹ 空 bsp ⟹ 无订单（诚实退化）。管线全链跑通（不 panic）。
        assert_eq!(res.n_orders, 0, "无结构 ⟹ 空候选 ⟹ 无订单（诚实退化）");
        assert_eq!(res.is_l2, res.n_orders > 0, "is_l2 ⟺ 订单非空（诚实标注不变量）");
        assert!(res.metrics.bh_return > 0.0, "L1：buy&hold 对照正确（上涨数据）");
        assert!(res.closed_loop_final.is_some(), "非空 bars ⟹ 闭环终态");
    }

    /// ★[A] 因果坐实（639）：runner 前缀因果分类**只用 ≤i 数据**——`parse_layer(bars[0..=i])` 的
    /// merged_bars/segments 无任何 index > i（无 look-ahead）；前缀分类 bsp source_index ≤ i。这是执行层
    /// σ_p 因果合法的结构保证（父容器方向从 ≤i 前缀塔查得，不引用未来 bar）。
    #[test]
    fn run_theta_v0_pi_prefix_classify_is_causal_no_lookahead() {
        let config = ThetaConfig::default();
        // 锯齿 bars（顶底交替，前缀分类非平凡）。
        let bars: Vec<Bar> = (0..40)
            .map(|i| {
                let up = ((i / 3) % 2) == 0;
                let base = 10_000_000_000i64;
                let step = 300_000_000i64 * ((i % 3) as i64);
                mk_bar(i, if up { base + step } else { base + 900_000_000 - step }, false)
            })
            .collect();
        for &i in &[5usize, 12, 25, 39] {
            let l0_prefix = parser::parse_layer(&bars[..=i], &config);
            // ① merged_bars 不读 >i（无 look-ahead）。
            assert!(
                l0_prefix.merged_bars.iter().all(|b| b.source_index <= i),
                "前缀 merged_bars 无 source_index>{i}（无 look-ahead）"
            );
            // ② segments 不读 >i。
            assert!(
                l0_prefix.segments.iter().all(|s| s.end_index <= i),
                "前缀 segments 无 end_index>{i}（无 look-ahead）"
            );
            // ③ 前缀分类 bsp source_index ≤ i（候选不引用未来 bar）。
            let (c_i, _t_i) = classifier::classify_with_tower(&l0_prefix, &config);
            assert!(
                c_i.levels.iter().all(|lv| lv.bsp.iter().all(|p| p.source_index <= i)),
                "前缀分类 bsp source_index ≤ {i}（因果）"
            );
        }
    }

    /// ★执行层 σ_p = 父容器方向（639，runner [A] 适配器 wiring）：切片当步卖候选 + per-bar 因果塔
    /// （有向 L1 Long 父走势）→ `assemble_gamma_with_tower` ⟹ V=ShortDiff（来自**父容器方向**，
    /// **与持仓无关**——`assemble_gamma_with_tower` 不接受 active 参数）。取代旧"活动父腿"错口径 loop 见证。
    #[test]
    fn run_theta_v0_pi_loop_shortdiff_from_parent_container() {
        use super::super::super::strategy::interp::assemble_gamma_with_tower;
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::strategy::coverage::Vertical;
        use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove as LM};
        use super::super::super::classifier::center::UnitRange;
        use super::super::super::types::Direction;
        // per-bar 因果塔：L1 Long 父走势（3 个 L0 子，sub(8,12) 右端点 ρ=12；外缘 10→15 ⟹ Long）。
        let u = |si, ei, d, lo, hi| UnitRange { start_index: si, end_index: ei, direction: d, lo, hi };
        let s0 = LM::from_unit(&u(0, 4, Direction::Up, 0, 10), ElementId { level: 0, ordinal: 0 });
        let s1 = LM::from_unit(&u(4, 8, Direction::Down, 3, 12), ElementId { level: 0, ordinal: 1 });
        let s2 = LM::from_unit(&u(8, 12, Direction::Up, 5, 15), ElementId { level: 0, ordinal: 2 });
        let l1 = LM::compose(
            &[s0, s1, s2],
            Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: 0, end_index: 12 },
            1,
            ElementId { level: 1, ordinal: 0 },
        );
        // ★O(n) 重构：生产塔现为 Vec<Rc<Vec<LeveledMove>>>，测试字面量逐级包 Rc（仅类型适配）。
        let tower: Vec<std::rc::Rc<Vec<_>>> = vec![Vec::new(), vec![l1]].into_iter().map(std::rc::Rc::new).collect();
        // 切片当步 L0 卖候选 source_index=12（host=sub(8,12) ⟹ 真父 L1 Long ⟹ σ_p=Long）。
        let classification = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![{
                let mut p = buy1_at(12);
                p.bits = BspBits { sell1: true, ..Default::default() };
                p.pivot_low = 0; p.pivot_high = 11_000_000_000;
                p
            }]), ..Default::default() }],
        };
        // 单候选分类（bsp@12，moves/centers 空）——直接作当步候选喂 σ_p 派生（与确认-bar 部署产物同形）。
        let sliced = classification.clone();
        // ★639：未持仓（assemble_gamma_with_tower 不接受 active）仍 ShortDiff（σ_p=父容器方向）。
        let gamma = assemble_gamma_with_tower(&sliced, &tower);
        assert_eq!(gamma[0].dir, VoiceSide::Short);
        assert_eq!(
            gamma[0].role.v,
            Vertical::ShortDiff,
            "L0 卖 under L1 Long 父容器 ⟹ ShortDiff（639：来自父容器方向，非持仓父腿）"
        );
    }

    /// ★前缀重分类路径确定性（639）：`run_theta_v0_pi`（per-bar 前缀因果重分类闭包）无隐藏状态/RNG ⟹
    /// 同输入两次运行结果 bit-identical。坐实 O(n²) 前缀重分类路径确定（接 L2/L3 否证的前提）。
    #[test]
    fn run_theta_v0_pi_deterministic_prefix_path() {
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..30)
            .map(|i| {
                let up = ((i / 3) % 2) == 0;
                let base = 10_000_000_000i64;
                let step = 300_000_000i64 * ((i % 3) as i64);
                mk_bar(i, if up { base + step } else { base + 900_000_000 - step }, false)
            })
            .collect();
        let ds = Dataset {
            symbol: "ZZ".to_string(),
            bars,
            dates: (0..30).map(|i| format!("2024-01-{:02} 00:00:00", (i % 28) + 1)).collect(),
            bar_seconds: 60,
        };
        let a = run_theta_v0_pi(&ds, &config, 1.0, 1.0e6);
        let b = run_theta_v0_pi(&ds, &config, 1.0, 1.0e6);
        assert_eq!(a.n_orders, b.n_orders, "前缀重分类路径确定 ⟹ n_orders 两次相同");
        assert_eq!(a.trades.len(), b.trades.len(), "trades 两次相同");
        assert_eq!(a.metrics.n_trades, b.metrics.n_trades, "n_trades 两次相同");
    }

    /// ★`run_theta_v0_pi` 全链在**结构化锯齿数据**上前缀因果重分类跑通（不 panic，不变量保持）——
    /// 区别于 `..structureless`（单调无结构）：锯齿产笔/段，前缀塔逐 bar 演化，坐实因果路径在真结构上稳健。
    #[test]
    fn run_theta_v0_pi_zigzag_prefix_path_runs_clean() {
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..60)
            .map(|i| {
                let up = ((i / 4) % 2) == 0;
                let base = 10_000_000_000i64;
                let step = 250_000_000i64 * ((i % 4) as i64);
                mk_bar(i, if up { base + step } else { base + 1_000_000_000 - step }, false)
            })
            .collect();
        let ds = Dataset {
            symbol: "ZZ60".to_string(),
            bars,
            dates: (0..60).map(|i| format!("2024-02-{:02} 00:00:00", (i % 28) + 1)).collect(),
            bar_seconds: 60,
        };
        let res = run_theta_v0_pi(&ds, &config, 1.0, 1.0e6);
        assert_eq!(res.is_l2, res.n_orders > 0, "is_l2 ⟺ 订单非空（诚实标注不变量）");
        assert!(res.closed_loop_final.is_some(), "非空 bars ⟹ 闭环终态");
        assert!(res.metrics.strat_return.is_finite(), "strat 有限（前缀因果路径不产 NaN/Inf）");
        assert!(res.trade_pnls_with_forced.iter().all(|p| p.is_finite()), "PnL 有限");
    }

    /// ★Q2 close_pred 折 𝒦_Θ（loop 内见证）：风控门把退出折进可行集（非第二出口）——
    /// k_theta_risk_gate 产门 + pi_theta_position 收窄 𝒦_Θ。此处坐实 force_flat（Insolvent equity≤0）门。
    #[test]
    fn run_theta_v0_pi_risk_gate_force_flat_on_insolvent() {
        let classification = Classification { levels: vec![LevelState::default()] };
        let bar = px100_bar(0);
        // equity≤0 ⟹ Insolvent ⟹ GlobalRiskClose ⟹ force_flat（𝒦_Θ={0}）。
        let gate_insolvent = k_theta_risk_gate(&[], &classification, &bar, -1.0, 0.0, 100.0, None);
        assert!(gate_insolvent.force_flat, "equity≤0 ⟹ Insolvent ⟹ force_flat（𝒦_Θ={{0}}）");
        // equity>0 + 无活动腿 ⟹ 门全开（无风控触发）。
        let gate_open = k_theta_risk_gate(&[], &classification, &bar, 1.0e6, 0.0, 100.0, None);
        assert!(!gate_open.force_flat && !gate_open.stop_long && !gate_open.stop_short, "正常态 ⟹ 门全开");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★★督导见证（task #94）：多 bar 闭环证明 account+twState 每 bar 真更新喂回
    //  （对比旧 runner「account 构造一次不喂回」的开环单帧）
    // ──────────────────────────────────────────────────────────────────────

    /// ★督导见证①：多 bar 闭环——micro_state/ledger/tw_state/orders 每 bar 真更新喂回。
    ///
    /// 旧 runner（开环单帧）：`account` 构造一次（runner.rs:107-110），全程不更新——闭环态
    /// 不存在。本测试用 [`run_closed_loop`] 跑 N bar 闭环，断言闭环终态的各分量推进到 = bar 数
    /// （证明每 bar `x = hybrid_step(x, e)` 真喂回，非构造一次）。
    #[test]
    fn closed_loop_threads_every_bar() {
        // 10 根波动 bar（涨跌交替，触发不同闭环动作）。
        let closes = [1000, 1010, 1005, 1020, 1015, 1030, 1025, 1040, 1035, 1050];
        let bars: Vec<Bar> = closes.iter().enumerate().map(|(i, &c)| mk_bar(i, c, false)).collect();

        let final_state = run_closed_loop(&bars, 1.0e6).expect("非空 bars ⟹ 有闭环终态");

        // ★每 bar 真喂回的物证：micro_state 推进到 = bar 数（开环单帧 bar_count 恒 0）。
        assert_eq!(final_state.micro_state.bar_count, 10, "10 bar 闭环 ⟹ bar_count=10（每 bar 喂回）");
        assert_eq!(final_state.micro_state.bars_seen, 10, "bars_seen 推进到 10");
        // orders 计数每 bar +1（开环单帧 orders 恒 0）。
        assert_eq!(final_state.orders, 10, "10 bar ⟹ orders=10（订单计数闭环推进）");

        // ★双账本不变量在闭环每步保持（终态仍满足）。
        assert!(final_state.ledger_state.inv_holds(), "闭环终态保 R=Π-A-W");
        // ★codex R3 C' 终局裁定后：run_closed_loop 注资额 = 首个可交易市价（closes[0]=1000）=notional_in。
        // hwm_gain 降为纯诊断（零承重，不入 free）⟹ **TW 守恒** tw()=notional_in（诊断浮盈不进 TW 三量）。
        // hwm_gain 仍如实追踪浮盈峰值（上行 1050 ⟹ 峰值 50），但只作可观测诊断，不入账 free。
        assert_eq!(final_state.tw_state.notional_in, 1000, "注资额 = 首个可交易市价 closes[0]");
        assert_eq!(
            final_state.tw_state.tw(),
            final_state.tw_state.notional_in,
            "TW 守恒：tw()=notional_in（C' 后 Revalue 诊断-only，浮盈不入 TW）"
        );
        assert_eq!(final_state.tw_state.hwm_gain, 50, "诊断高水位如实追踪上行峰值 1050 ⟹ 1·(1050−1000)=50");
        assert_eq!(final_state.tw_state.free, 0, "free 恒 0（诊断浮盈不入账 free，承重已移除）");
        // OQ-9 gate：schedule_adapter 不开 legacy 腿 ⟹ open_legacy_legs 恒 0。
        assert_eq!(final_state.tw_state.open_legacy_legs, 0, "闭环终态 OQ-9 gate 保持");

        // ★对比开环单帧：旧 account 构造一次后字段不随 bar 变化（此处用初始态对比）。
        let initial = AssemblyState::initial(1_000_000);
        assert_eq!(initial.micro_state.bar_count, 0, "开环单帧基线：bar_count=0（不喂回）");
        assert_ne!(
            final_state.micro_state.bar_count, initial.micro_state.bar_count,
            "闭环终态 ≠ 初始态 ⟹ 每 bar 真喂回（非构造一次）"
        );
    }

    /// ★督导见证②：闭环 ledger 在波动数据上真演化（开仓侧 Allocate / 平仓侧 Realize 交替）。
    ///
    /// ledger_state 随**真实仓位增量**演化（codex #1 修复后：非恒等挂件，也非幽灵累积）。
    ///
    /// ★codex #1 根因修复后：初始 Normal/PhaseI ⟹ intent 恒 Buy，但 `risk_adapter` 首 bar 后恒返
    /// `positions.max(1)=1`（仓位 0→1 后不再增）⟹ **只有首 bar 有真实仓位增量 Δ=1**，其余 bar Δ=0 ⟹ Noop。
    /// 这证 ledger 由真实仓位增量驱动（仓位门控），非时间门控。
    ///
    /// ★GAP3 补桥后：建仓额 = `Δ·price`（值模型，非单位数）。首 bar close=1000 ⟹ 建 1 单位 Allocate(1000)
    /// （成本基），A=1000（**不是** A=1 的单位数口径，也不是 A=5000 的幽灵累积）。price=1 时退化为 A=1。
    #[test]
    fn closed_loop_ledger_evolves() {
        let bars: Vec<Bar> = (0..5).map(|i| mk_bar(i, 1000 + i as i64 * 10, false)).collect();
        let final_state = run_closed_loop(&bars, 1.0e6).expect("有终态");
        // 首 bar 仓位 0→1（真实建仓 Δ=1 @ price=1000 ⟹ Allocate(1000)）；bar 2-5 仓位不变（Δ=0 ⟹ Noop）。
        assert_eq!(final_state.positions, 1, "仓位 0→1（risk_adapter max(1) 饱和 ⟹ 只增一次）");
        assert_eq!(final_state.ledger_state.a, 1000, "一次真实建仓 @ price=1000 ⟹ A=1000（成本基值，非单位数）");
        assert_eq!(final_state.ledger_state.r, -1000, "R=Π-A-W=-1000（一次 Allocate(1000)）");
        assert!(final_state.ledger_state.inv_holds(), "终态保恒等");
    }

    /// ★★codex R3 C' 终局裁定后：`run_closed_loop` 在**有界振荡**价格流上 EarningShares **不触达**——照实
    /// （161/no-workaround）。C' 移除 hwm_gain 棘轮承重后，机制恢复为「TW 守恒 + 浮盈只作诊断不入 free」。
    ///
    /// **机制（C' 后）**：引擎消费真实市价，但重估浮盈**只推进诊断高水位，不入账 free**：
    /// - **建仓值模型**：首 bar close=1000 建 1 单位，holding=1000（成本基）=notional_in（注资额=首市价）。
    /// - **诊断重估**：每 bar 派 `Revalue(delta)` 只推进 hwm_gain。有界振荡（closes∈[1000,1060]）⟹
    ///   浮盈峰值 = 1·(1060−1000)=60 ⟹ hwm_gain 封顶 60（**纯诊断，不入 free**）。
    /// - **足额退本金门**：CostReduction→CapitalRecovered 需 `free ≥ recover_target = notional_in = 1000`。
    ///   而 free 恒 0（诊断浮盈不入 free，无真实卖出现金回流）⟹ 永不足额退本金 ⟹ stage 恒 CostReduction。
    ///
    /// **结论（照实，FALSIFIED）**：合法账本语义下无非回补资金源，三阶段不推进——EarningShares 结构不可达
    /// （GAP3「∃t TStage=III」FALSIFIED，codex R3 C'）。达 EarningShares 需重装「已实现利润」入账/可正可负
    /// MTM 账本并重新提交裁决（见 `price_magnitude_drives_diagnostic_hwm_not_tw_closed_loop` 与 L2 BTC 不可达回测）。
    #[test]
    fn closed_loop_earning_shares_not_reached_l0_honest_gap3() {
        use super::super::super::strategy::ledger::TStage;
        let bars: Vec<Bar> = (0..600).map(|i| mk_bar(i, 1000 + (i as i64 % 7) * 10, false)).collect();
        let final_state = run_closed_loop(&bars, 1.0e6).expect("非空 bars ⟹ 有闭环终态");

        // ★照实①：stage 恒 CostReduction（有界振荡浮盈不足足额退本金 ⟹ 三阶段不推进）。
        assert_eq!(
            final_state.tw_state.stage,
            TStage::CostReduction,
            "有界振荡闭环 stage 应恒 CostReduction（浮盈 60 ≪ 本金 1000，退本金门不开）——终 stage={:?}",
            final_state.tw_state.stage
        );

        // ★照实②：仓位门控 + 值模型——真实仓位只增一次（0→1），holding=成本基=首市价 1000（非单位数 1）。
        assert_eq!(final_state.positions, 1, "真实仓位 0→1 一次（risk_adapter max(1) 饱和）");
        assert_eq!(final_state.tw_state.holding, 1000, "holding=成本基=1 单位×首市价 1000（值模型）");
        assert_eq!(final_state.tw_state.notional_in, 1000, "注资额=首市价 1000");

        // ★照实③：诊断高水位封顶 60（有界振荡峰值），但 hwm_gain 零承重（不入 free）⟹ free 恒 0 ⟹ 未退本金。
        assert_eq!(final_state.tw_state.hwm_gain, 60, "诊断浮盈峰值 1·(1060−1000)=60（纯诊断，不入 free）");
        assert_eq!(final_state.tw_state.free, 0, "free 恒 0（C' 后诊断浮盈不入账 free，承重已移除）");
        assert_eq!(final_state.tw_state.withdrawn, 0, "无 sound 资金源 ⟹ 未退本金（withdrawn=0）");

        // ★结构不变量仍保持：TW 守恒 tw()=notional_in（诊断浮盈不进 TW）+ R=Π-A-W + OQ-9 gate（legacy 腿=0）。
        assert_eq!(
            final_state.tw_state.tw(),
            final_state.tw_state.notional_in,
            "TW 守恒：tw()=notional_in=1000（C' 后 Revalue 诊断-only，浮盈不进 TW）"
        );
        assert!(final_state.ledger_state.inv_holds(), "保 R=Π-A-W");
        assert_eq!(final_state.tw_state.open_legacy_legs, 0, "OQ-9 gate 保持（legacy 腿=0）");
    }

    /// ★★GAP3 结构不可达定理（codex 复审#2 修正：删除硬凑的「已获利」见证态）：从 funded_campaign
    /// 出发，**在 sound closed-loop 路径上无法到达 CapitalRecovered**——EarningShares 结构不可达。
    ///
    /// ★口径收窄（codex 二轮）：这是 **sound 路径**（cash-tight w≤free 退本金）的不可达，**不是**
    /// 「任何 raw 事件序列」不可达——raw `RecoverCapital(1)` 从 `free=Q, holding=0` 可推 CapitalRecovered
    /// 但 **unsound**（会破坏现金-sound，被 transition_adapter 出口断言拒；见 transition.rs 现金-sound
    /// gate）。故不可达域 = 引擎生产的 sound 转移路径（stage_progression 派生 + transition_adapter 出口
    /// 断言钉死 free≥0），非模型全 raw 事件代数。
    ///
    /// **根因（codex 复审#2 逼出的更强结论）**：三个约束联合不可满足——
    /// - **TW 守恒**：funded_campaign 起 TW=Q（free=Q, holding=0）；所有 TW 事件保 `tw()=free+holding+
    ///   withdrawn` 不变 ⟹ TW 恒 =Q（ShortDiff 中性 / CloseShareLeg 落 cum_net_cash 在 TW 外 /
    ///   RecoverCapital 是 free→withdrawn 内部转移）。**无任何事件让 TW>Q**（利润在 L0 同价不产生）。
    /// - **退本金前提**：stage_progression 的 CostReduction→退本金要求 `holding ≥ notional_in = Q`。
    /// - **cash-tight**（codex#3 修复）：sound 退本金要求 `free > 0`（w=min(·,free)>0）。
    ///
    /// holding≥Q ∧ TW=Q ⟹ free+withdrawn = Q−holding ≤ 0 ⟹ free ≤ 0（withdrawn≥0）⟹ **sound 退本金
    /// 的 free>0 与退本金前提 holding≥Q 在 TW=Q 下互斥**。故从 funded_campaign 永远到不了 sound 的
    /// CapitalRecovered ⟹ EarningShares 不可达。这是 **model-level 结构结论（照实 161/no-workaround）**：
    /// 旧「free=Q∧holding=Q」见证态 TW=2Q，**从 campaign 起点 TW 守恒下不可达**，是硬凑 harness（codex
    /// 复审#2 判致命，已删）。EarningShares 真达需 **L2 价格升值让已实现利润进 TW**（超出本 L0 模型事件集）。
    ///
    /// 机制本身正确性（enter_ready/tw_step 各分支）由 `strategy::ledger` 单元测试（`enter_ready_strict_
    /// conjunction`/`stage_rank_monotone` 等）在**显式假设态**上验证——那是「给定前提则机制正确」的单元
    /// 断言，**不**声明该前提从 campaign 起点可达（有效域区分：机制正确 ≠ 前提可达）。
    ///
    /// ★★codex §9.2（诚实标注路线）：those `strategy::ledger` unit tests are **synthetic-state
    /// legality tests, not reachability tests** — they验 mechanism legality on assumed states, 不构造
    /// (nor claim) a real trade path x0→…→xT reaching η_T≥I0−W_T+η⋆. THIS test (below) is the
    /// **reachability** side: it proves EarningShares is structurally unreachable on sound closed-loop
    /// paths from `funded_campaign` under L0 same-price（TW 守恒）. 二者互补，无 fabricated witness。
    #[test]
    fn earning_shares_structurally_unreachable_from_campaign_tw_conserved() {
        use super::super::super::strategy::ledger::{TStage, TwState, TwEvent, tw_step};

        let q: i64 = 3;
        let funded = AssemblyState::funded_campaign(1_000_000, q);
        let tw0 = funded.tw_state.tw();
        assert_eq!(tw0, q, "funded_campaign 起 TW=Q（free=Q, holding=0）");

        // ★TW 守恒：枚举所有 TW 事件，无一让 TW≠Q（利润在 L0 同价不产生 ⟹ TW 恒 =Q）。
        let s = funded.tw_state;
        let events = [
            TwEvent::ShortDiff(2), TwEvent::ShortDiff(-2),
            TwEvent::OpenShareLeg, TwEvent::CloseShareLeg(5), TwEvent::CloseShareLeg(-5),
            TwEvent::RecoverCapital(1), TwEvent::EnterEarning,
        ];
        for e in events {
            assert_eq!(tw_step(&s, e).tw(), tw0, "TW 事件 {:?} 保 TW=Q（无利润注入）", e);
        }

        // ★互斥证明：任何 TW=Q 且 holding≥notional_in=Q 的态，free≤0 ⟹ sound 退本金（free>0）不可能。
        // 遍历 holding∈[Q, 2Q] 在 TW=Q 约束下（free=Q−holding−withdrawn，withdrawn≥0），free 恒 ≤0。
        for holding in q..=(2 * q) {
            for withdrawn in 0..=q {
                let free = tw0 - holding - withdrawn; // TW=Q 约束下的 free
                if holding >= q {
                    assert!(
                        free <= 0,
                        "TW=Q 下 holding={}≥Q ∧ withdrawn={} ⟹ free={}≤0（退本金前提与 cash-tight 互斥）",
                        holding, withdrawn, free
                    );
                }
            }
        }

        // ★直接见证：构造「holding=Q（退本金前提满足）」但 TW=Q 的态 ⟹ free=0 ⟹ 退本金 w=min(Q,0)=0
        // ⟹ stage_progression 不推进（stage 恒 CostReduction）——机制正确地拒绝 unsound 退本金。
        let holding_ready = TwState { free: 0, holding: q, withdrawn: 0, notional_in: q, ..TwState::initial() };
        assert_eq!(holding_ready.tw(), q, "holding=Q 态 TW 仍=Q（free=0）");
        assert!(holding_ready.holding >= holding_ready.notional_in, "退本金前提 holding≥notional_in 满足");
        // free=0 ⟹ 无 sound 退本金源 ⟹ stage 不推进（经闭环一步验证 stage 保持 CostReduction）。
        let x = AssemblyState { tw_state: holding_ready, ..AssemblyState::funded_campaign(1_000_000, q) };
        // price=1：L0 单位价（值模型退化为守恒单位模型，重估 credit=0）——见证 L0 同价下 TW 守恒不可达。
        let e = AssemblyEvent { parse_event: MicroEvent::NewBar(true), price: 1 };
        // codex R3 §9.3：生产路径恒 Ok（此处 holding=Q/free=0，schedule 派 ShortDiff + stage_progression w=0 不推进）。
        let x1 = hybrid_step(&x, &e, &RiskPolicy::baseline())
            .expect("生产恒 Ok（schedule 只派 ShortDiff + stage_progression w≤free）");
        assert_eq!(
            x1.tw_state.stage, TStage::CostReduction,
            "holding≥Q 但 free=0 ⟹ 退本金 w=0 ⟹ stage 不推进（机制拒 unsound 退本金，非负 free 借款）"
        );
        assert!(x1.tw_state.free >= 0, "全程 free≥0（codex 复审#1：买入侧亦受 free 约束）");
    }

    /// ★★codex R3 C' 终局裁定后——价格幅度**驱动诊断高水位 hwm_gain，但不驱动 TW/stage**（决定性，秒级）：
    /// `run_closed_loop` 消费真实价格幅度（价格管线保留 §5.4），**同 rising 布尔序列**下平缓涨与暴涨产出
    /// **不同**的诊断 hwm_gain——但因 hwm_gain 零承重（不入 free），两组 TW 均守恒（=notional_in）且 stage
    /// 均恒 CostReduction（EarningShares 不可达=FALSIFIED，acc-GAP3「∃t TStage=III」恢复未证成）。
    ///
    /// 对照设计：两组 rising 布尔序列相同（隔 bar 涨），仅幅度不同：
    /// - gentle（涨到 1001）：诊断浮盈峰值 1 ⟹ hwm_gain=1。
    /// - violent（涨到 100000）：诊断浮盈峰值 99000 ⟹ hwm_gain=99000。
    /// 两组 hwm_gain 不同（价格幅度驱动诊断），但 stage 均 CostReduction、TW 均 =notional_in（承重移除）。
    #[test]
    fn price_magnitude_drives_diagnostic_hwm_not_tw_closed_loop() {
        use super::super::super::strategy::ledger::TStage;
        let n = 64usize;
        let gentle: Vec<Bar> = (0..n)
            .map(|i| mk_bar(i, if i % 2 == 0 { 1000 } else { 1001 }, false))
            .collect();
        let violent: Vec<Bar> = (0..n)
            .map(|i| mk_bar(i, if i % 2 == 0 { 1000 } else { 100_000 }, false))
            .collect();
        let rising_of = |bars: &[Bar]| -> Vec<bool> {
            let mut prev = bars[0].close;
            bars.iter().map(|b| { let r = b.close >= prev; prev = b.close; r }).collect()
        };
        assert_eq!(rising_of(&gentle), rising_of(&violent), "对照前提：两组 rising 布尔序列相同");
        let a = run_closed_loop(&gentle, 1.0e6).expect("非空");
        let b = run_closed_loop(&violent, 1.0e6).expect("非空");
        // ★核心确证：同 rising 序列下价格幅度产出**不同**诊断 hwm_gain ⟹ 引擎消费价格幅度（管线保留 §5.4）。
        assert_ne!(a.tw_state, b.tw_state, "价格幅度驱动诊断 hwm_gain（tw_state 因 hwm_gain 不同而异）");
        assert!(b.tw_state.hwm_gain > a.tw_state.hwm_gain, "暴涨诊断高水位 > 平缓涨（价格幅度驱动诊断）");
        // ★C' 承重移除：两组 stage 均 CostReduction（浮盈不入 free ⟹ 退本金门不开），TW 均守恒 =notional_in。
        assert_eq!(a.tw_state.stage, TStage::CostReduction, "gentle：诊断浮盈零承重 ⟹ 停 CostReduction");
        assert_eq!(b.tw_state.stage, TStage::CostReduction, "violent：诊断浮盈零承重 ⟹ 停 CostReduction（EarningShares 不可达=FALSIFIED）");
        assert_eq!(a.tw_state.tw(), a.tw_state.notional_in, "gentle TW 守恒 =notional_in（诊断不进 TW）");
        assert_eq!(b.tw_state.tw(), b.tw_state.notional_in, "violent TW 守恒 =notional_in（承重移除，浮盈不进 TW）");
        assert_eq!(a.tw_state.withdrawn, 0, "gentle 未退本金（无 sound 资金源）");
        assert_eq!(b.tw_state.withdrawn, 0, "violent 未退本金（诊断浮盈不构成退本金资金源）");
        assert!(b.tw_state.free == 0 && a.tw_state.free == 0, "两组 free 恒 0（诊断浮盈不入账 free）");
    }

    /// ★★L2 不可达实证（#[ignore]，真实 BTC 变价数据——codex R3 C' 终局裁定后）：closed_loop 三阶段闭环
    /// 接真实 BTC 变价数据流，acc-GAP3 判据「∃t TStage=III」在合法账本语义下**恢复 FALSIFIED**
    /// （EarningShares count=0）。复算：`cargo test --lib l2_btc_earning_shares_unreachable_hwm_debearing
    /// -- --ignored --nocapture`（默认全历史；`ECON_L2_MAX_BARS=N` 可截尾）。run_closed_loop 是 O(n)。
    /// C' 前 count=1（hwm_gain 棘轮把浮盈入 free 驱动退本金，被裁定为 PDF p8③ 语义回补）；C' 后 hwm_gain
    /// 降为纯诊断（不入 free），即便 BTC 长期升值使诊断 hwm_gain≫本金，free 恒不受浮盈影响 ⟹ 无 sound
    /// 退本金资金源 ⟹ stage 恒 CostReduction ⟹ **count=0**（FALSIFIED，直到「已实现利润」/MTM 账本重装）。
    #[test]
    #[ignore]
    fn l2_btc_earning_shares_unreachable_hwm_debearing() {
        use super::super::super::strategy::ledger::TStage;
        let config = ThetaConfig::default();
        let ds_full = match super::super::data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("BTC 加载失败：{e}（DATA BLOCKER，不伪造合成）");
                return;
            }
        };
        let n_full = ds_full.bars.len();
        // 默认全历史（BTC 长期升值使浮盈≫本金，可达确定）；ECON_L2_MAX_BARS 可截尾提速（截尾窗口内
        // 若 BTC 未达 2×建仓价则可能 count=0——那是有效域边界，非 bug）。
        let max_bars: usize = std::env::var("ECON_L2_MAX_BARS")
            .ok().and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
        let bars: &[Bar] = if n_full > max_bars { &ds_full.bars[n_full - max_bars..] } else { &ds_full.bars };
        let x = run_closed_loop(bars, 1.0e6).expect("非空 BTC bars ⟹ 闭环终态");
        let s = x.tw_state;
        let count = if s.stage == TStage::EarningShares { 1 } else { 0 };
        eprintln!(
            "[L2-BTC] bars={} (全量{}) final_stage={:?} EarningShares_count={} | TW={} free={} holding={} withdrawn={} notional_in={} cum_net_cash={} hwm_gain={}",
            bars.len(), n_full, s.stage, count, s.tw(), s.free, s.holding, s.withdrawn, s.notional_in, s.cum_net_cash, s.hwm_gain,
        );
        // ★acc-GAP3 恢复 FALSIFIED（codex R3 C'）：L2 真实 BTC 变价数据上 EarningShares 不可达（count=0）。
        assert_eq!(count, 0, "C' 后 L2 BTC EarningShares 不可达（count=0，FALSIFIED）——实测 final_stage={:?}", s.stage);
        assert_eq!(s.stage, TStage::CostReduction, "hwm_gain 诊断零承重 ⟹ stage 恒 CostReduction（无 sound 资金源）");
        // ★TW 守恒（诊断浮盈不进 TW 三量）；诊断 hwm_gain 可远超本金但不入 free ⟹ 未退本金。
        assert_eq!(s.tw(), s.notional_in, "TW 守恒 tw()=notional_in（C' 后 Revalue 诊断-only）");
        assert_eq!(s.withdrawn, 0, "无 sound 退本金资金源 ⟹ 未退本金（withdrawn=0）");
    }

    /// run_theta_v0 携带闭环终态证据（closed_loop_final 非 None ⟺ bars 非空）。
    #[test]
    fn run_theta_v0_carries_closed_loop_evidence() {
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(|i| mk_bar(i, 1000 + (i as i64 % 3) * 10, false)).collect();
        let ds = Dataset {
            symbol: "TEST".to_string(),
            bars,
            dates: (0..20).map(|i| format!("2024-01-{:02} 00:00:00", (i % 28) + 1)).collect(),
            bar_seconds: 60,
        };
        let res = run_theta_v0(&ds, &config, 1.0, 1.0e6);
        let cl = res.closed_loop_final.expect("非空 bars ⟹ 闭环终态");
        // 闭环态每 bar 喂回：bar_count = 输入 bar 数。
        assert_eq!(cl.micro_state.bar_count, 20, "run_theta_v0 内闭环驱动 20 bar");
        assert!(cl.ledger_state.inv_holds(), "回测内闭环保 R=Π-A-W");
        // ★codex R3 C' 终局裁定后：注资额=首市价 closes[0]=1000=notional_in；有界振荡（1000-1020）诊断
        // 浮盈峰值 1·(1020−1000)=20（纯诊断，不入 free）⟹ TW 守恒 tw()=notional_in=1000（承重已移除）。
        assert_eq!(cl.tw_state.notional_in, 1000, "注资额=首市价 1000");
        assert_eq!(cl.tw_state.hwm_gain, 20, "诊断浮盈峰值 20（纯诊断，不入 free）");
        assert_eq!(
            cl.tw_state.tw(),
            cl.tw_state.notional_in,
            "TW 守恒 tw()=notional_in=1000（C' 后 Revalue 诊断-only，浮盈不进 TW）"
        );
    }

    /// 空 bars ⟹ run_closed_loop 返回 None（边界条件）。
    #[test]
    fn closed_loop_empty_bars_none() {
        assert!(run_closed_loop(&[], 1.0e6).is_none(), "空 bars ⟹ 无闭环终态");
    }

    /// fill 模拟基础形态（L1）：手工构造非空订单流，验证开/平仓 + 费用 + 盈亏。
    /// 这是 fill 层的**独立 L1 验证**（不依赖 classify/plan_orders 是否实装）。
    #[test]
    fn simulate_fills_open_close_with_fees_l1() {
        let config = ThetaConfig::default(); // commission 1bp + slippage 2bp = 3bp/side
        // 价格 tick=1e-8 ⇒ close_tick=1e10 → px=100.0；下一根 tick=1.1e10 → px=110.0。
        let p0 = (100.0 / 1e-8) as i64;
        let p1 = (110.0 / 1e-8) as i64;
        let bars = vec![mk_bar(0, p0, false), mk_bar(1, p1, false)];
        // Buy 1 lot @ bar0(px=100)，Close 1 lot @ bar1(px=110)。NAV=1000（资金充足买 1 lot@100）。
        let orders = vec![
            Order { action: StrictAction::Buy, qty: 1, exec_index: 0 },
            Order { action: StrictAction::Close, qty: 1, exec_index: 1 },
        ];
        let nav = 1000.0;
        let (equity, _rets, pnls) = simulate_fills(&bars, &orders, nav, &config);
        assert_eq!(equity.len(), 2);
        assert_eq!(pnls.len(), 1, "一次完整开平仓");
        // 毛利 = 110-100 = 10；扣双边费用（建仓 100×3bp + 平仓 110×3bp ≈ 0.03+0.033）。
        // pnl = 110×(1-0.0003) - 100 ≈ 9.967（close_long 的 proceeds - cost_basis，绝对额）。
        assert!(pnls[0] > 9.5 && pnls[0] < 10.0, "盈利约 9.97（扣费），实得 {}", pnls[0]);
        // 权益曲线归一化：末权益 = (1000-100.03+110×(1-3bp))/1000 ≈ 1.00997（收益率口径）。
        assert!(equity[1] > 1.0, "盈利交易 ⇒ 末权益 > 初始 1.0，实得 {}", equity[1]);
    }

    /// ★ plan_and_fill_mtm：空 decisions ⟹ 权益曲线全 1.0，n_orders=0（L1 退化验证）。
    ///
    /// 认识论：L1（合成 bars + 空 decisions，验证管线正确退化，零信息增量）。
    /// 边界条件：decisions 非空时才进 per-bar plan 路径（此测试仅验证退化）。
    #[test]
    fn plan_and_fill_mtm_empty_decisions_flat_equity() {
        let config = ThetaConfig::default();
        let bars: Vec<Bar> =
            (0..5).map(|i| mk_bar(i, 1000 + i as i64 * 10, false)).collect();
        let fill = plan_and_fill_mtm(&[], &bars, 1.0e6, &config);
        assert_eq!(fill.n_orders, 0, "空 decisions ⟹ n_orders=0");
        assert_eq!(fill.trade_pnls_realized.len(), 0, "无交易 ⟹ 无 pnl");
        assert_eq!(fill.trade_pnls_with_forced.len(), 0, "无持仓 ⟹ 无终点强平 pnl");
        assert_eq!(fill.trades.len(), 0, "无交易 ⟹ 无轨迹");
        assert_eq!(fill.equity_curve.len(), 5, "权益曲线与 bars 等长");
        // 所有权益值 = 1.0（无交易，cash=nav0，units=0）。
        for (i, e) in fill.equity_curve.iter().enumerate() {
            assert!(
                (e - 1.0).abs() < 1e-12,
                "bar {} 权益应 =1.0（无交易），实得 {}",
                i, e
            );
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★退出决策生成器（§9 closePred）：trades=0 真根因修复的 L1 验证
    //  （合成数据验证管线正确产 Close 订单 → trade_pnls 非空 → n_trades>0）
    // ──────────────────────────────────────────────────────────────────────

    use super::super::super::strategy::VoiceDecision;
    use super::super::super::strategy::risk::StopInput;
    use super::super::super::strategy::voice::VoiceSide;
    use super::super::super::types::{BspBits, Center, Tick};

    /// OHLC 可控的 bar（退出测试需 low/high 触发止损，mk_bar 的 o=h=l=c 不够）。
    fn ohlc_bar(idx: usize, o: Tick, h: Tick, l: Tick, c: Tick) -> Bar {
        Bar {
            source_index: idx,
            timestamp: idx as i64,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 100,
            untradable: false,
        }
    }

    /// 单根 1 买 Long 入场决策（depth 0，stop = pivot_low；tick_size=1 ⟹ 价格=美元）。
    fn buy1_long_decision(signal_index: usize, pivot_low: Tick) -> VoiceDecision {
        VoiceDecision {
            depth: 0,
            root_side: VoiceSide::Long,
            exit: false,
            enter_ok: true,
            bsp: BspBits { buy1: true, ..Default::default() },
            signal_index,
            stop_in: StopInput {
                pivot_low,
                pivot_high: 0,
                center: Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 0, end_index: 0 },
            },
            entry: 100,
            cost_per_unit: 0.0,
            level: 5,
        }
    }

    /// ★真根因修复 L1：止损触及 ⟹ 退出生成器产 Close ⟹ trade_pnls 非空 ⟹ n_trades>0。
    ///
    /// 修复前（exit 恒 false，无退出生成器）：无 Close ⟹ trade_pnls 恒空 ⟹ n_trades=0（cyan 坐实）。
    /// 修复后：开多 @ bar1 → 后续 bar 的 low 跌破 stop(=pivot_low=90) ⟹ §9 closePred.Stop=true
    /// ⟹ 退出生成器产 exit=true 决策 ⟹ plan_orders 产 Close ⟹ apply_order close_long ⟹ pnl 入账。
    /// 认识论 L1（合成数据验证退出管线串通 + Close 真产出，非伪造）。
    #[test]
    fn exit_generator_stop_hit_produces_close_trade() {
        let mut config = ThetaConfig::default();
        config.tick.tick_size = 1.0; // 价格=美元（entry=100/stop=90 是美元值）

        // bar0：信号确认前置；bar1：信号 @ source_index=0 的延迟成交 bar（开多 @ open=100）；
        // bar2：价格平稳（low=95>stop=90，不触发）；bar3：low=85<stop=90 ⟹ 止损触及。
        // bar4：退出 Close 的延迟成交 bar（触发 bar3 → fill bar4）。
        let bars = vec![
            ohlc_bar(0, 100, 110, 90, 105),  // 信号 bar（source_index=0）
            ohlc_bar(1, 100, 110, 99, 108),  // 开多成交 bar（entry delay 1，open=100）
            ohlc_bar(2, 105, 112, 100, 109), // 持仓中（low=100>stop=90，不触发）
            ohlc_bar(3, 95, 100, 85, 88),    // ★low=85<stop=90 ⟹ Stop 触发
            ohlc_bar(4, 88, 92, 85, 90),     // 退出 Close 延迟成交 bar
        ];
        let decisions = vec![buy1_long_decision(0, 90)]; // stop = pivot_low = 90

        let fill = plan_and_fill_mtm(&decisions, &bars, 1_000_000.0, &config);

        // 修复核心断言：退出生成器产 Close ⟹ 已实现 trade_pnls 非空 ⟹ n_trades > 0。
        assert!(!fill.trade_pnls_realized.is_empty(), "止损触及 ⟹ 退出生成器产 Close ⟹ 已实现盈亏非空（修复前恒空）");
        assert_eq!(fill.trade_pnls_realized.len(), 1, "一次完整开平（开多 + 止损平多）");
        // 止损全平 ⟹ 末尾无未平仓 ⟹ 含浮盈口径 = 已实现口径（无强平笔）。
        assert_eq!(fill.trade_pnls_with_forced.len(), 1, "止损全平 ⟹ 无终点强平 ⟹ 双口径同长");
        // 轨迹记录：1 笔正常退出（forced_close=false）。
        assert_eq!(fill.trades.len(), 1, "1 笔交易轨迹");
        assert!(!fill.trades[0].forced_close, "止损退出非强平");
        // n_orders ≥ 2（开仓 1 + 平仓 1）。
        assert!(fill.n_orders >= 2, "至少 2 单（开多 + 止损 Close），实得 {}", fill.n_orders);
    }

    /// ★退出生成器：无关闭触发（价格平稳，无反向/止损/破产）⟹ 持仓延续，无 Close（不误平）。
    /// **+ 含浮盈口径见证（2026-06-28）**：持仓到末尾未平 ⟹ 已实现口径空，但含浮盈口径**非空**
    /// （终点强平实现浮盈），且强平笔 forced_close=true。
    ///
    /// closePred 全 false ⟹ 退出生成器返 None ⟹ 持仓延续 ⟹ 已实现 trade_pnls 空（持仓不平=正确，
    /// 关闭优先于开启不误触发）。但**含浮盈口径**对末尾持仓强平——编排者铁律「不把浮盈算上
    /// 不合理」：上涨持仓的浮盈是择时载体，剔除它=系统性抹掉。这是退出生成器**不过度触发**
    /// 与**含浮盈口径**的双重见证。
    #[test]
    fn exit_generator_no_trigger_holds_position() {
        let mut config = ThetaConfig::default();
        config.tick.tick_size = 1.0;

        // 开多 @ bar1，后续价格全在 stop=90 之上（low 恒 >90），无反向 BSP，equity>0 ⟹ closePred 全 false。
        let bars = vec![
            ohlc_bar(0, 100, 110, 95, 105),
            ohlc_bar(1, 100, 110, 99, 108), // 开多成交
            ohlc_bar(2, 105, 115, 100, 110),
            ohlc_bar(3, 110, 120, 105, 115),
            ohlc_bar(4, 115, 125, 110, 120), // 持仓全程 low>90，无止损；末 close=120 > 入场 ⟹ 浮盈
        ];
        let decisions = vec![buy1_long_decision(0, 90)];

        let fill = plan_and_fill_mtm(&decisions, &bars, 1_000_000.0, &config);
        // 无关闭触发 ⟹ 持仓延续到末尾未平 ⟹ 已实现口径无平仓 pnl（退出生成器不误触发）。
        assert!(
            fill.trade_pnls_realized.is_empty(),
            "价格平稳无关闭触发 ⟹ 持仓延续 ⟹ 已实现口径无 Close（不过度触发）"
        );
        // ★含浮盈口径：末尾持仓被终点强平 ⟹ 含浮盈口径非空（实现浮盈）。
        assert_eq!(
            fill.trade_pnls_with_forced.len(), 1,
            "持仓到末尾 ⟹ 含浮盈口径终点强平 1 笔（编排者铁律：浮盈算上）"
        );
        // 末 close=120 > 入场（开多成交 @ bar1 close=108）⟹ 强平浮盈为正。
        assert!(
            fill.trade_pnls_with_forced[0] > 0.0,
            "上涨持仓强平 ⟹ 浮盈为正，实得 {}",
            fill.trade_pnls_with_forced[0]
        );
        // 强平笔轨迹打 forced_close=true（统计功效门槛不计此笔）。
        assert_eq!(fill.trades.len(), 1, "1 笔强平轨迹");
        assert!(fill.trades[0].forced_close, "终点强平笔 forced_close=true");
    }

    /// ★退出生成器：反向 BSP（持多遇卖侧根决策）⟹ §9 closePred.reverse_signal ⟹ Close。
    ///
    /// 持多仓后，后续 bar 出现卖侧（Short 根）开仓决策 ⟹ 当前 bar 的 groups 含反向方向决策
    /// ⟹ reverse_signal=true ⟹ closePred 触发先平（对齐 risk.rs root_dir_next case2「反向先平」）。
    /// 认识论 L1（验证反向信号触发退出管线串通）。
    #[test]
    fn exit_generator_reverse_signal_produces_close() {
        let mut config = ThetaConfig::default();
        config.tick.tick_size = 1.0;

        // bar 序列：开多 @ bar1（buy1 信号 @ source_index=0），后续出现卖侧信号 @ source_index=2
        // ⟹ 延迟成交 bar3 出现 Short 根决策 ⟹ bar3 的 groups 含反向 ⟹ 持多遇反向先平。
        let bars = vec![
            ohlc_bar(0, 100, 110, 95, 105),
            ohlc_bar(1, 100, 110, 99, 108), // 开多成交
            ohlc_bar(2, 105, 115, 100, 110), // 卖侧信号 bar（source_index=2）
            ohlc_bar(3, 108, 118, 103, 113), // 反向 Short 决策延迟成交 bar（low>stop=90，止损不触发）
            ohlc_bar(4, 110, 120, 105, 115), // 退出 Close 延迟成交 bar
        ];
        // 入场：buy1 Long @ src 0；反向：sell1 Short @ src 2（stop=pivot_high）。
        let entry = buy1_long_decision(0, 90);
        let mut reverse = buy1_long_decision(2, 90);
        reverse.root_side = VoiceSide::Short;
        reverse.bsp = BspBits { sell1: true, ..Default::default() };
        reverse.stop_in = StopInput {
            pivot_low: 0,
            pivot_high: 200, // Short 止损在上方
            center: Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 0, end_index: 0 },
        };
        let decisions = vec![entry, reverse];

        let fill = plan_and_fill_mtm(&decisions, &bars, 1_000_000.0, &config);
        // 反向信号触发先平 ⟹ 多仓被 Close ⟹ 已实现 trade_pnls 非空。
        assert!(
            !fill.trade_pnls_realized.is_empty(),
            "持多遇反向卖侧根决策 ⟹ §9 closePred.reverse_signal ⟹ Close（先平后建）"
        );
    }

    /// 不可交易 bar 上不成交（reference:53）。
    #[test]
    fn untradable_bar_skips_order() {
        let config = ThetaConfig::default();
        let p = (100.0 / 1e-8) as i64;
        let bars = vec![mk_bar(0, p, true)]; // untradable
        let orders = vec![Order { action: StrictAction::Buy, qty: 1, exec_index: 0 }];
        let (_eq, _r, pnls) = simulate_fills(&bars, &orders, 1000.0, &config);
        assert_eq!(pnls.len(), 0, "不可交易 bar 跳过订单，无交易");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★v1 做空腿 fill 账本（方向中性，apply_order/apply_fill golden）
    //  canonical FULL §10「根声部双向全定义状态机」+ §13 有符号名义头寸 + Origin.TotalWealth
    // ──────────────────────────────────────────────────────────────────────

    /// 开空：units 变负，cash 增（收到卖出净收 = qty×px×(1−fee)）。对齐 canonical σ_r=−1（空头）
    /// + Origin.TotalWealth TW=cash+units×px 守恒（开空瞬间 TW 仅扣费用，本金不变）。
    #[test]
    fn short_open_units_negative_cash_increases() {
        let fee = 0.001;
        let (mut cash, mut units, mut ec, mut pnls) = (1_000_000.0, 0.0, 0.0, Vec::new());
        let o = Order { action: StrictAction::Sell, qty: 100, exec_index: 0 };
        apply_order(&o, 50.0, fee, &mut cash, &mut units, &mut ec, &mut pnls);
        assert_eq!(units, -100.0, "开空 ⟹ units 负（canonical σ_r=−1 空头）");
        assert!(
            (cash - (1_000_000.0 + 100.0 * 50.0 * (1.0 - fee))).abs() < 1e-6,
            "开空 cash+ 卖出净收"
        );
        assert!((ec - 50.0 * (1.0 - fee)).abs() < 1e-9, "空头成本基 = 卖出均价扣费");
        assert!(pnls.is_empty(), "开仓无已实现 PnL");
        let tw = cash + units * 50.0;
        assert!(
            (tw - (1_000_000.0 - 100.0 * 50.0 * fee)).abs() < 1e-6,
            "开空 TW 仅扣费（Origin.TotalWealth 守恒）"
        );
    }

    /// 平空（盈利）：买回价 < 开空价 ⟹ 空头盈利。units 回 0，cash 减买回支出，PnL>0。
    /// Close 方向感知：持空 ⟹ 买回（δ+1）。
    #[test]
    fn short_close_profit_when_price_drops() {
        let fee = 0.001;
        let (mut cash, mut units, mut ec, mut pnls) = (1_000_000.0, 0.0, 0.0, Vec::new());
        apply_order(&Order { action: StrictAction::Sell, qty: 100, exec_index: 0 }, 50.0, fee, &mut cash, &mut units, &mut ec, &mut pnls);
        let cash_after_open = cash;
        apply_order(&Order { action: StrictAction::Close, qty: 100, exec_index: 1 }, 40.0, fee, &mut cash, &mut units, &mut ec, &mut pnls);
        assert_eq!(units, 0.0, "平空 ⟹ units 回 0");
        assert!(
            (cash - (cash_after_open - 100.0 * 40.0 * (1.0 + fee))).abs() < 1e-6,
            "平空 cash− 买回支出"
        );
        assert_eq!(pnls.len(), 1, "平空产 1 笔 PnL");
        let expect = 100.0 * 50.0 * (1.0 - fee) - 100.0 * 40.0 * (1.0 + fee);
        assert!((pnls[0] - expect).abs() < 1e-6, "空头 PnL = 开空净收−平空支出");
        assert!(pnls[0] > 0.0, "价跌 ⟹ 空头盈利");
        assert_eq!(ec, 0.0, "全平 ⟹ 成本基归零");
    }

    /// 平空（亏损）：买回价 > 开空价 ⟹ 空头亏损（PnL<0）。
    #[test]
    fn short_close_loss_when_price_rises() {
        let fee = 0.001;
        let (mut cash, mut units, mut ec, mut pnls) = (1_000_000.0, 0.0, 0.0, Vec::new());
        apply_order(&Order { action: StrictAction::Sell, qty: 100, exec_index: 0 }, 50.0, fee, &mut cash, &mut units, &mut ec, &mut pnls);
        apply_order(&Order { action: StrictAction::Close, qty: 100, exec_index: 1 }, 60.0, fee, &mut cash, &mut units, &mut ec, &mut pnls);
        assert_eq!(units, 0.0, "平空 ⟹ units 回 0");
        assert!(pnls[0] < 0.0, "价涨 ⟹ 空头亏损（PnL<0）");
    }

    /// 多空 PnL 镜像对称（canonical §7 镜像等变兑现）：同幅度有利价变，多头（价涨）与空头
    /// （价跌）PnL 在零费下严格相等。
    #[test]
    fn long_short_pnl_mirror_symmetric_zero_fee() {
        let fee = 0.0;
        let (mut c1, mut u1, mut e1, mut p1) = (1_000_000.0, 0.0, 0.0, Vec::new());
        apply_order(&Order { action: StrictAction::Buy, qty: 100, exec_index: 0 }, 50.0, fee, &mut c1, &mut u1, &mut e1, &mut p1);
        apply_order(&Order { action: StrictAction::Close, qty: 100, exec_index: 1 }, 60.0, fee, &mut c1, &mut u1, &mut e1, &mut p1);
        let (mut c2, mut u2, mut e2, mut p2) = (1_000_000.0, 0.0, 0.0, Vec::new());
        apply_order(&Order { action: StrictAction::Sell, qty: 100, exec_index: 0 }, 50.0, fee, &mut c2, &mut u2, &mut e2, &mut p2);
        apply_order(&Order { action: StrictAction::Close, qty: 100, exec_index: 1 }, 40.0, fee, &mut c2, &mut u2, &mut e2, &mut p2);
        assert!((p1[0] - 1000.0).abs() < 1e-6, "多头 PnL=1000");
        assert!((p2[0] - 1000.0).abs() < 1e-6, "空头 PnL=1000");
        assert!((p1[0] - p2[0]).abs() < 1e-9, "多空 PnL 镜像对称（canonical §7 镜像等变）");
    }

    /// 翻转（先平后开，canonical §20）：持多 @50 遇 Sell qty>持仓 ⟹ 先平多（记 PnL）再开空。
    #[test]
    fn flip_long_to_short_sells_then_opens_short() {
        let fee = 0.0;
        let (mut cash, mut units, mut ec, mut pnls) = (1_000_000.0, 0.0, 0.0, Vec::new());
        apply_order(&Order { action: StrictAction::Buy, qty: 100, exec_index: 0 }, 50.0, fee, &mut cash, &mut units, &mut ec, &mut pnls);
        apply_order(&Order { action: StrictAction::Sell, qty: 150, exec_index: 1 }, 60.0, fee, &mut cash, &mut units, &mut ec, &mut pnls);
        assert_eq!(units, -50.0, "翻转后净持空 50（150 − 平多 100）");
        assert_eq!(pnls.len(), 1, "翻转产 1 笔平多 PnL");
        assert!((pnls[0] - 1000.0).abs() < 1e-6, "平多 PnL=100×(60−50)=1000");
        assert!((ec - 60.0).abs() < 1e-9, "翻转后空头成本基 = 开空价（零费）");
    }

    /// Close 方向感知：空仓遇 Close ⟹ 无操作（不借 Close 开新仓）。
    #[test]
    fn close_on_flat_is_noop() {
        let fee = 0.001;
        let (mut cash, mut units, mut ec, mut pnls) = (1_000_000.0, 0.0, 0.0, Vec::new());
        apply_order(&Order { action: StrictAction::Close, qty: 100, exec_index: 0 }, 50.0, fee, &mut cash, &mut units, &mut ec, &mut pnls);
        assert_eq!(units, 0.0, "空仓 Close 无操作");
        assert_eq!(cash, 1_000_000.0, "空仓 Close 现金不动");
        assert!(pnls.is_empty(), "空仓 Close 无 PnL");
    }

    /// 端到端做空腿（plan_and_fill_mtm）：单 Short 根决策 → 开空 → 止损平空 → trade 标 long=false。
    #[test]
    fn short_leg_end_to_end_trade_marked_short() {
        let mut config = ThetaConfig::default();
        config.tick.tick_size = 1.0;
        // Short 根：卖点 @ bar0，开空 @ bar1 open；止损在上方（pivot_high）。
        // bar3 high 突破 stop ⟹ 止损平空（买回）。
        let bars = vec![
            ohlc_bar(0, 100, 110, 90, 95),
            ohlc_bar(1, 100, 105, 95, 98),
            ohlc_bar(2, 98, 102, 94, 96),
            ohlc_bar(3, 105, 130, 104, 125), // high=130>stop ⟹ 止损触发
            ohlc_bar(4, 125, 128, 120, 122),
        ];
        let mut d = buy1_long_decision(0, 0);
        d.root_side = VoiceSide::Short;
        d.bsp = BspBits { sell1: true, ..Default::default() };
        d.stop_in = StopInput {
            pivot_low: 0,
            pivot_high: 120, // Short 止损在上方
            center: Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 0, end_index: 0 },
        };
        let fill = plan_and_fill_mtm(&[d], &bars, 1_000_000.0, &config);
        assert!(!fill.trades.is_empty(), "Short 根 ⟹ 开空 + 平空产交易轨迹");
        assert!(fill.trades.iter().all(|t| !t.long), "做空腿交易标 long=false（Short）");
        assert!(fill.n_orders >= 2, "至少 2 单（开空 Sell + 止损 Close）");
    }

    /// **真实数据分层诊断 + L2 探测**（`#[ignore]`，需 `analysis/data_cache/*.json`）。
    ///
    /// recognize 接通后（2026-06-26），此测试不再断言"阻塞态订单空"——而是**分层探测**真实
    /// OKLO 数据在 parse→classify→recognize→plan_orders 各层的产出量，定位订单流是否非空
    /// 及断点位置。**认识论**：管线串通 = L1；订单非空时的指标 = L2（可否证）。订单空时
    /// **如实标注非 L2 + 报告断点层**（不伪造 L2）。用 OKLO（最小品种，~343K bar）。
    ///
    /// 跑法：`cargo test --lib theta_v0::backtest::runner::tests::real_data_smoke_oklo -- --ignored --nocapture`
    #[test]
    #[ignore = "真实数据诊断，需 analysis/data_cache/oklo_1m_databento.json；显式 --ignored"]
    fn real_data_smoke_oklo() {
        use super::super::super::{classifier, parser, strategy};
        use super::super::data;
        use super::super::prereg_windows::PREREG_WINDOWS;

        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("OKLO", &config).expect("加载 OKLO 真实数据");
        assert!(ds.bars.len() > 300_000, "OKLO 应有 ~343K bar（协议 §1.1）");

        // 切 OOS 窗：从 PREREG_WINDOWS 消费 OKLO 的预注册 OOS 边界（§2.4 特例）。
        // OKLO OOS = ("2025-01-01", "2026-06-24")，非核心池统一 OOS("2023-01-01"/"2025-06-30")。
        // 使用 PREREG_WINDOWS 而非字面量，确保数据挖掘防护的唯一真相源（Task B）。
        let oklo_reg = PREREG_WINDOWS
            .iter()
            .find(|w| w.symbol == "OKLO")
            .expect("PREREG_WINDOWS 含 OKLO（prereg_consistency_pool_counts 已断言）");
        let oos = ds.slice_date_window(oklo_reg.oos.0, oklo_reg.oos.1);
        assert!(!oos.bars.is_empty(), "OOS 窗应非空");
        eprintln!("[OKLO] OOS 窗 bars={}", oos.bars.len());

        // ── 分层诊断：定位订单流断点（systematic-debugging）──
        let l0 = parser::parse_layer(&oos.bars, &config);
        eprintln!(
            "[OKLO] L0 解析: merged={} fractals={} strokes={} segments={}",
            l0.merged_bars.len(),
            l0.fractals.len(),
            l0.strokes.len(),
            l0.segments.len(),
        );
        let classification = classifier::classify(&l0, &config);
        let n_levels = classification.levels.len();
        let total_bsp: usize = classification.levels.iter().map(|lv| lv.bsp.len()).sum();
        let total_centers: usize = classification.levels.iter().map(|lv| lv.centers.len()).sum();
        eprintln!(
            "[OKLO] classify: levels={} total_centers={} total_bsp={}",
            n_levels, total_centers, total_bsp
        );
        let decisions = strategy::recognize(&classification, &oos.bars, &config);
        eprintln!("[OKLO] recognize: decisions={}", decisions.len());

        // ── 端到端回测 ──
        let res = run_theta_v0(&oos, &config, 2.0, 1.0e6);
        eprintln!(
            "[OKLO] 回测: n_orders={} is_l2={} bh_return={:.2}% strat_return={:.2}% \
             sharpe={:.3} maxDD={:.2}% n_trades={}",
            res.n_orders,
            res.is_l2,
            res.metrics.bh_return * 100.0,
            res.metrics.strat_return * 100.0,
            res.metrics.sharpe,
            res.metrics.max_drawdown * 100.0,
            res.metrics.n_trades,
        );

        // 断点诊断（如实报告，不伪造 L2）：
        if l0.segments.len() < 3 {
            eprintln!("[OKLO] ⟹ 断点 L0：线段<3，无法构造中枢（缠论中枢需前三连续走势）");
        } else if total_centers == 0 {
            eprintln!("[OKLO] ⟹ 断点 classify：有线段但无中枢（重叠判据未满足）");
        } else if total_bsp == 0 {
            eprintln!("[OKLO] ⟹ 断点 signal：有中枢但无第三类买卖点（离开+回试判据未满足）");
        } else if decisions.is_empty() {
            eprintln!("[OKLO] ⟹ 断点 recognize：有 bsp 但无声部决策");
        } else if res.n_orders == 0 {
            eprintln!("[OKLO] ⟹ 断点 plan_orders/sizing：有决策但无订单（qty<=0 或不可交易）");
        } else {
            eprintln!("[OKLO] ⟹ 端到端产订单流 ⇒ L2 回测有效");
        }

        // 不变量（无论订单是否非空）：管线在真实数据上不崩 + L2 标注与订单一致。
        assert!(res.metrics.bh_return.is_finite(), "buy&hold 对照有限值");
        assert_eq!(res.is_l2, res.n_orders > 0, "is_l2 ⟺ 订单非空（诚实标注）");
    }

    /// **★E5 trades=0 真根因修复 L2 验证（截断窗口，退出生成器真实数据 trades>0）**
    /// （`#[ignore]`，需 `analysis/data_cache/oklo_1m_databento.json`）。
    ///
    /// 全 OOS 窗（268K bar × 430K 决策）的退出生成器逐 bar 检查在 590s timeout 内跑不完
    /// （性能，非正确性）——本测试取 OKLO OOS 窗的**前 30000 bar** 截断窗口，在时限内验证
    /// **退出决策生成器修复后真实数据产 Close 订单 ⟹ trade_pnls 非空 ⟹ n_trades>0**。
    ///
    /// ## 分层诊断（formalization-validity-domain 231号 + 625 铁律）
    ///
    /// - parse→classify→recognize→plan_orders→退出生成器各层产出量逐层报告。
    /// - **修复前**（exit 恒 false，无退出生成器）：n_trades=0（cyan 编译期机器证据坐实）——
    ///   不是「Θ 不盈利」（L2 经验否证），是**接线层缺退出生成器**（工程层）。
    /// - **修复后**：退出生成器逐 bar 检查 §9 closePred（止损/反向/RiskClose）产 Close ⟹
    ///   n_trades>0 ⟹ **接线层打通**，回测进入可产生平仓盈亏的 L2 状态。
    /// - **诚实分层**：n_trades>0 证「退出闭环接线层打通」（L1 工程层），**不证** Θ 策略盈利
    ///   （strat% 的符号/大小是 L2 经验结果，本测试只验证 trades 计数非零，不对 strat 符号下结论）。
    ///
    /// 跑法：`cargo test --lib theta_v0::backtest::runner::tests::e5_exit_loop_trades_nonzero_oklo -- --ignored --nocapture`
    #[test]
    #[ignore = "E5 trades>0 真实数据验证，需 oklo cache；截断窗口；显式 --ignored"]
    fn e5_exit_loop_trades_nonzero_oklo() {
        use super::super::super::{classifier, parser, strategy};
        use super::super::data;
        use super::super::prereg_windows::PREREG_WINDOWS;

        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("OKLO", &config).expect("加载 OKLO");
        let oklo_reg = PREREG_WINDOWS
            .iter()
            .find(|w| w.symbol == "OKLO")
            .expect("PREREG_WINDOWS 含 OKLO");
        let oos_full = ds.slice_date_window(oklo_reg.oos.0, oklo_reg.oos.1);
        assert!(!oos_full.bars.is_empty(), "OOS 窗非空");

        // 截断前 30000 bar（性能：全窗 268K bar 退出生成器逐 bar 超时；30K 在时限内）。
        let cut = 30_000.min(oos_full.bars.len());
        let oos = Dataset {
            symbol: oos_full.symbol.clone(),
            bars: oos_full.bars[..cut].to_vec(),
            dates: oos_full.dates[..cut.min(oos_full.dates.len())].to_vec(),
            bar_seconds: 60,
        };
        eprintln!("[E5] 截断 OOS 窗 bars={}（全窗 {}）", oos.bars.len(), oos_full.bars.len());

        // ── 分层诊断（systematic-debugging）──
        let l0 = parser::parse_layer(&oos.bars, &config);
        let classification = classifier::classify(&l0, &config);
        let total_bsp: usize = classification.levels.iter().map(|lv| lv.bsp.len()).sum();
        let total_centers: usize = classification.levels.iter().map(|lv| lv.centers.len()).sum();
        let decisions = strategy::recognize(&classification, &oos.bars, &config);
        eprintln!(
            "[E5] 分层：segments={} centers={} bsp={} decisions={}",
            l0.segments.len(),
            total_centers,
            total_bsp,
            decisions.len(),
        );

        // ── 端到端回测（退出生成器接入）──
        let first_px = oos
            .bars
            .iter()
            .find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * config.tick.tick_size)
            .unwrap_or(1.0);
        let nav = (first_px * 1000.0).max(1.0e6);
        let res = run_theta_v0(&oos, &config, 0.5, nav);
        eprintln!(
            "[E5] 回测：n_orders={} n_trades={} is_l2={} bh={:.2}% strat(MtM)={:.2}%",
            res.n_orders,
            res.metrics.n_trades,
            res.is_l2,
            res.metrics.bh_return * 100.0,
            res.metrics.strat_return * 100.0,
        );

        // ── 分层诊断断点（如实报告，不伪造）──
        if decisions.is_empty() {
            eprintln!("[E5] ⟹ 断点 recognize：无决策（不应发生——OKLO 全窗 430K 决策）");
        } else if res.n_orders == 0 {
            eprintln!("[E5] ⟹ 断点 plan_orders/sizing：有决策无订单");
        } else if res.metrics.n_trades == 0 {
            eprintln!("[E5] ⟹ 断点 退出生成器：有订单但无平仓（退出闭环未产 Close——修复回退）");
        } else {
            eprintln!("[E5] ⟹ 退出闭环打通：trades>0（接线层 L1 修复，n_trades 非零）");
        }

        // ★E5 真根因修复核心断言：退出生成器修复后 n_trades>0（修复前 cyan 坐实 0）。
        // 诚实分层：这证「退出闭环接线层打通」（L1 工程层），不证 Θ 盈利（strat 符号是 L2，不断言）。
        assert!(
            res.metrics.n_trades > 0,
            "E5 真根因修复：退出生成器接入后真实数据产平仓 ⟹ n_trades>0（修复前恒 0），实得 {}",
            res.metrics.n_trades
        );
        assert!(res.n_orders > 0, "有平仓必有订单");
        assert!(res.metrics.strat_return.is_finite(), "strat 有限");
    }

    /// **L2 真实数据回测：8 品种 OOS 窗 + 分层诊断**（`#[ignore]`，需 `analysis/data_cache/*.json`）。
    ///
    /// canonical Next Gate 3 / acceptance #5 的执行体。遍历 PREREG_WINDOWS 全 8 品种，对每品种
    /// 的预注册 OOS 窗（核心/扩展统一 2023-01-01→2025-06-30；OKLO §2.4 特例）跑 frozen Θ v0，
    /// 产出每品种：strat_return / sharpe / max_dd / n_trades / n_orders + **分层诊断计数**
    /// （信号在 parse→classify→signal→recognize→plan_orders 哪一层断流）。
    ///
    /// ## 认识论（formalization-validity-domain 231号，强制标注）
    ///
    /// - 管线驱动本身 = **L1**（遍历 + 收集，零信息增量）。
    /// - 每品种产出的扣成本指标 = **L2 当且仅当 n_orders>0**（真实数据 + 非空订单流，可否证）。
    /// - **诚实约束**（[[l2-engine-incompleteness-vs-theta-falsification]] 625 铁律）：若某品种
    ///   n_orders=0，**报「L1 工程层断流在 X 层」非「L2 策略不盈利」**——分层诊断精确定位断点
    ///   （segments<3 / centers=0 / bsp=0 / decisions=0 / orders=0），不把引擎断流当经验否证。
    ///   recognize 断点进一步细分到判据级（empty_bits / fill_oob / center_inv），并校验
    ///   source_index 坐标系一致性（max_bsp_src < bars.len）。
    ///
    /// ## 发生史：source_index 坐标系 bug（acceptance #5 实证坐实并修复，留记保留生成史 012号）
    ///
    /// 本测试首跑（commit 12f8e4b8）实测 8 品种全部 `decisions=0`：`slice_date_window` 切 OOS 时
    /// 保留 `Bar.source_index` 的全集绝对偏移（未重置为局部下标），parse 链信任该字段 ⟹
    /// `BspPoint.source_index ≥ bars.len` ⟹ `fill_bar_index` 用全集偏移索引 OOS 局部数组而越界、
    /// 吞掉**全部**决策。反事实对照（测试内只读把 source_index←局部下标，管线其余不变）使 7 品种
    /// decisions 由 0 全转非零（BTC 9,323,059 等）⟹ **坐实 (b) L1 实现 bug，非 (a) 真实拒绝、非
    /// 定义冲突**：source_index 同时是「局部数组下标」与 reference:16「平局裁决键」，重置为局部
    /// 下标 0..len 满足二者、不改任何缠论定义 ⟹ 据 testing-override 正常修复。**修复已落
    /// `data.rs::slice_date_window`（切片重置 source_index 为局部下标）**，本测试现校验修复后
    /// 坐标系不再断裂（`n_coordsys_mismatch==0`）且 ≥1 品种产干净 L2 订单流。
    ///
    /// 跑法：`cargo test --lib theta_v0::backtest::runner::tests::l2_oos_eight_symbols -- --ignored --nocapture`
    #[test]
    #[ignore = "L2 真实数据回测，需 analysis/data_cache/*.json 全 8 品种；显式 --ignored"]
    fn l2_oos_eight_symbols() {
        use super::super::super::{classifier, parser, strategy};
        use super::super::data;
        use super::super::prereg_windows::PREREG_WINDOWS;

        let config = ThetaConfig::default();

        eprintln!("\n===== L2 OOS 回测：8 品种分层诊断 =====");
        eprintln!(
            "{:<6} {:>9} {:>8} {:>8} {:>7} {:>4} {:>4} {:>4} {:>5} {:>5} {:>6}  {:>4} {}",
            "symbol", "oos_bars", "bh%", "strat%", "sharpe", "seg", "ctr", "bsp",
            "decis", "ord", "trades", "L?", "断点层",
        );

        // 分层断点诊断聚合计数（L1 工程层 vs L2 经验否证的精确归因）。
        let mut n_block_l0_segments = 0usize; // 断点 L0：线段<3（无法构造中枢）
        let mut n_block_classify = 0usize; // 断点 classify：有线段无中枢
        let mut n_block_signal = 0usize; // 断点 signal：有中枢无 BSP
        let mut n_block_recognize = 0usize; // 断点 recognize：有 BSP 无声部决策
        let mut n_block_sizing = 0usize; // 断点 plan_orders/sizing：有决策无订单
        let mut n_l2_orders = 0usize; // 端到端产订单流（L2 有效）

        // recognize 断点的子原因细分（625 铁律：精确归因，非粗归到「recognize:无决策」）。
        // 当某品种 total_bsp>0 却 decisions=0 时，逐 BspPoint 复现 strategy::recognize_point 的
        // 三道拒绝判据（与 strategy/mod.rs 同一逻辑，单一真相），统计哪道判据吞掉了全部 bsp：
        //   - empty_bits：bits 无任何买卖位（bsp_root_side→None，非交易点）
        //   - fill_oob：信号后 entry_delay 落点越过序列末尾或全 untradable（fill_bar_index→None）
        //   - center_inv：含 3 类 bit 但 center=None（classifier 不变量违反，显式拒绝）
        let mut recog_reject_empty_bits = 0usize;
        let mut recog_reject_fill_oob = 0usize;
        let mut recog_reject_center_inv = 0usize;
        let mut recog_accept_point = 0usize; // 通过三道判据本应产决策的 BspPoint

        // 坐标系不一致计数（品种级）：fill_oob 全失败且该品种 max(source_index) ≥ bars.len，
        // 即 BspPoint.source_index 落在 [0, bars.len) 之外 ⟹ source_index 与回测 bars 坐标系
        // 不在同一基准（非数据稀缺：untradable_ratio≈0）。
        let mut n_coordsys_mismatch = 0usize;

        for w in PREREG_WINDOWS {
            let ds = match data::load_by_symbol(w.symbol, &config) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("{:<6} 加载失败：{e}", w.symbol);
                    continue;
                }
            };
            // 预注册 OOS 窗（唯一真相源 = PREREG_WINDOWS，防数据挖掘 Task B）。
            let oos = ds.slice_date_window(w.oos.0, w.oos.1);
            if oos.bars.is_empty() {
                eprintln!("{:<6} OOS 窗空（{}→{}）", w.symbol, w.oos.0, w.oos.1);
                continue;
            }

            // ── 分层诊断：定位订单流断点（systematic-debugging）──
            let l0 = parser::parse_layer(&oos.bars, &config);
            let classification = classifier::classify(&l0, &config);
            let total_bsp: usize = classification.levels.iter().map(|lv| lv.bsp.len()).sum();
            let total_centers: usize =
                classification.levels.iter().map(|lv| lv.centers.len()).sum();
            let decisions = strategy::recognize(&classification, &oos.bars, &config);

            // 坐标系断裂的**全品种直接证据**（含 L2 成功的 OKLO，便于对照为何它成功）：
            // oos_first_src = OOS 首 bar 的全集 source_index（slice 保留 load 赋值，不重置）；
            // max_bsp_src = 全部 BspPoint.source_index 最大值（=segment.end_index，携全集偏移）。
            // 判据：max_bsp_src ≥ bars.len ⟹ source_index 与 OOS 局部数组坐标系断裂。
            let oos_first_src = oos.bars.first().map(|b| b.source_index).unwrap_or(0);
            let max_bsp_src = classification
                .levels
                .iter()
                .flat_map(|lv| lv.bsp.iter())
                .map(|p| p.source_index)
                .max()
                .unwrap_or(0);
            eprintln!(
                "  [{:<5} 坐标] oos_first_src={oos_first_src} max_bsp_src={max_bsp_src} \
                 bars.len={} 断裂={}",
                w.symbol,
                oos.bars.len(),
                max_bsp_src >= oos.bars.len(),
            );

            // recognize 断点子原因细分（625 铁律精确归因）：仅当「有 BSP 无决策」时，逐 point
            // 复现 strategy::recognize_point 的三道拒绝判据（同序：bits→fill→center），定位是
            // 哪道判据吞掉了全部 bsp。bsp_root_side 私有，用同一 bits 判据（公开字段）复现。
            if total_bsp > 0 && decisions.is_empty() {
                let mut max_src_idx = 0usize;
                let mut first_fail_dump: Option<(usize, usize, bool)> = None;
                for level in &classification.levels {
                    for point in level.bsp.iter() {
                        max_src_idx = max_src_idx.max(point.source_index);
                        let b = &point.bits;
                        let has_buy = b.buy1 || b.buy2 || b.buy3;
                        let has_sell = b.sell1 || b.sell2 || b.sell3;
                        if !has_buy && !has_sell {
                            recog_reject_empty_bits += 1; // bsp_root_side→None
                            continue;
                        }
                        // fill_bar_index：与 strategy::recognize_point 第二道判据同一函数。
                        if strategy::exec::fill_bar_index(
                            point.source_index,
                            &oos.bars,
                            &config.exec,
                        )
                        .is_none()
                        {
                            if first_fail_dump.is_none() {
                                // 捕获首个 fill 失败的坐标证据：source_index vs bars.len，
                                // 及该 source_index 处 bar 是否越界（区分越界 vs 全 untradable）。
                                let oob = point.source_index >= oos.bars.len();
                                first_fail_dump =
                                    Some((point.source_index, oos.bars.len(), oob));
                            }
                            recog_reject_fill_oob += 1; // 信号后落点越界/全 untradable
                            continue;
                        }
                        // center 不变量：含 3 类 bit 但 center=None ⟹ recognize_point 显式拒绝。
                        let has_third = b.buy3 || b.sell3;
                        if has_third && point.center.is_none() {
                            recog_reject_center_inv += 1;
                            continue;
                        }
                        recog_accept_point += 1; // 本应产决策（与 decisions=0 矛盾→候选）
                    }
                }
                // 坐标证据（首个 fill 失败的 source_index/bars.len + max_src_idx + untradable 率）：
                // 区分根因——max_src_idx ≥ bars.len ⟹ source_index 坐标系越界（坐标系不一致）；
                // max_src_idx < bars.len 但全失败 ⟹ OOS 窗可交易 bar 稀缺（数据层）。
                let coordsys_mismatch =
                    recog_reject_fill_oob > 0 && max_src_idx >= oos.bars.len();
                if coordsys_mismatch {
                    n_coordsys_mismatch += 1;
                }
                if let Some((src, len, oob)) = first_fail_dump {
                    // OOS 首 bar 的全集偏移：slice_date_window 保留 load_symbol 赋的全集绝对
                    // source_index（不重置为 0）。若 oos_first_src > 0 ⟹ OOS 是全集中段切片，
                    // segment.end_index（=bar.source_index）携带全集偏移 ⟹ 用它索引 OOS 局部
                    // 数组必越界。这是坐标系断裂的**直接机器证据**（非推理）。
                    let oos_first_src = oos.bars.first().map(|b| b.source_index).unwrap_or(0);
                    eprintln!(
                        "  [{} 坐标证据] oos_first_src={oos_first_src} first_fail src_idx={src} \
                         bars.len={len} oob={oob} max_src_idx={max_src_idx} \
                         untradable_ratio={:.4} 坐标系不一致={coordsys_mismatch}",
                        w.symbol,
                        oos.untradable_ratio(),
                    );
                }
            }

            // 端到端回测（OOS 窗 2.5 年，年化基数 §3.1；NAV 与品种价量级匹配——用首价×容量）。
            let first_px = oos
                .bars
                .iter()
                .find(|b| !b.untradable && b.close > 0)
                .map(|b| b.close as f64 * config.tick.tick_size)
                .unwrap_or(1.0);
            let nav = (first_px * 1000.0).max(1.0e6);
            let res = run_theta_v0(&oos, &config, 2.5, nav);

            // 断点归因（如实报告，不伪造 L2；与 real_data_smoke_oklo 同一诊断阶梯）。
            let blocked_layer = if l0.segments.len() < 3 {
                n_block_l0_segments += 1;
                "L0:线段<3"
            } else if total_centers == 0 {
                n_block_classify += 1;
                "classify:无中枢"
            } else if total_bsp == 0 {
                n_block_signal += 1;
                "signal:无BSP"
            } else if decisions.is_empty() {
                n_block_recognize += 1;
                "recognize:无决策"
            } else if res.n_orders == 0 {
                n_block_sizing += 1;
                "sizing:无订单"
            } else {
                n_l2_orders += 1;
                "L2有效"
            };

            eprintln!(
                "{:<6} {:>9} {:>8.2} {:>8.2} {:>7.3} {:>4} {:>4} {:>4} {:>5} {:>5} {:>6}  {:>4} {}",
                w.symbol,
                oos.bars.len(),
                res.metrics.bh_return * 100.0,
                res.metrics.strat_return * 100.0,
                res.metrics.sharpe,
                l0.segments.len(),
                total_centers,
                total_bsp,
                decisions.len(),
                res.n_orders,
                res.metrics.n_trades,
                if res.is_l2 { "L2" } else { "L1" },
                blocked_layer,
            );

            // 不变量（每品种）：管线在真实数据上不崩 + L2 标注与订单一致。
            assert!(res.metrics.bh_return.is_finite(), "{} buy&hold 有限", w.symbol);
            assert!(res.metrics.strat_return.is_finite(), "{} strat 有限", w.symbol);
            assert_eq!(
                res.is_l2,
                res.n_orders > 0,
                "{} is_l2 ⟺ 订单非空（诚实标注）",
                w.symbol
            );
        }

        // ── 诚实结论：L2 经验否证 vs L1 工程缺口的精确归因（分层计数）──
        let total = PREREG_WINDOWS.len();
        let l1_blocked = n_block_l0_segments
            + n_block_classify
            + n_block_signal
            + n_block_recognize
            + n_block_sizing;
        eprintln!("\n===== 分层诊断聚合（8 品种 OOS）=====");
        eprintln!("断点 L0(线段<3)      : {n_block_l0_segments}");
        eprintln!("断点 classify(无中枢): {n_block_classify}");
        eprintln!("断点 signal(无BSP)   : {n_block_signal}");
        eprintln!("断点 recognize(无决策): {n_block_recognize}");
        eprintln!("断点 sizing(无订单)  : {n_block_sizing}");
        eprintln!("L2 端到端(产订单流)  : {n_l2_orders}");

        // recognize 断点子原因细分（625 精确归因）：把粗归因「recognize:无决策」拆到判据级。
        if n_block_recognize > 0 {
            eprintln!("\n--- recognize 断点子原因（逐 BspPoint 三道判据，单位=point）---");
            eprintln!("  reject empty_bits(非交易点): {recog_reject_empty_bits}");
            eprintln!("  reject fill_oob(落点越界)  : {recog_reject_fill_oob}");
            eprintln!("  reject center_inv(3类无中枢): {recog_reject_center_inv}");
            eprintln!("  accept(三道判据通过本应产决策): {recog_accept_point}");
            eprintln!("  其中坐标系不一致品种数(max_src_idx≥bars.len): {n_coordsys_mismatch}");
        }
        eprintln!(
            "\n===== source_index 坐标系回归校验（acceptance #5 bug 已修，data.rs::slice_date_window）====="
        );
        eprintln!("坐标系断裂品种数(max_bsp_src≥bars.len): {n_coordsys_mismatch}（修复后应为 0）");

        // ★ source_index 坐标系 bug 的发生史与修复（acceptance #5，反事实对照坐实）：
        //
        // 曾经的根因链（已修，留记保留生成史 012号）：
        //   1. load_symbol(data.rs) 赋 Bar.source_index = 全数据集绝对下标 i；
        //   2. slice_date_window 切 OOS 时 `push(*b)` **保留**全集 source_index，未重置为局部下标
        //      ⟹ oos.bars[0].source_index = 全集偏移（实测 BTC=2,817,999 > bars.len=1,313,200）；
        //   3. parse 链（inclusion/fractal/stroke/segment）信任 bar.source_index 字段（非数组下标），
        //      故 BspPoint.source_index(=segment.end_index) 携全集偏移；
        //   4. recognize_point → fill_bar_index(source_index, oos.bars) 用全集偏移索引 OOS 局部数组
        //      ⟹ source_index ≥ bars.len ⟹ 越界返 None ⟹ 全决策被吞 ⟹ 零订单（8 品种 decisions=0）。
        //
        // a/b 坐实（反事实对照，机器证据非推理）：测试内只读把 oos.bars 的 source_index 改为局部
        // 下标 0..len（管线其余不变）重跑 ⟹ 7 品种 decisions 由 0 全部转非零（BTC 9,323,059 等）。
        // 唯一改变的变量 = source_index ⟹ 坐实 **(b) L1 实现 bug，非 (a) 真实拒绝、非定义冲突**。
        //
        // 判据（testing-override）：source_index 在本系统同时是「局部数组下标」（fill_bar_index 用
        // bars[source_index] 直接索引当前 Dataset）与「平局裁决键」（reference:16 `(timestamp,
        // source_index)`）。修复 = slice_date_window 重置 source_index 为局部下标 0..len——既复原
        // 「= 数组下标」语义（下游索引合法），又仍是合法平局键（0..len 严格单调唯一）⟹ **不改任何
        // 缠论定义的含义/边界** ⟹ 实现 bug，正常修复（已修），不上浮。
        //
        // OKLO 原 L2 标注（strat=-0.12%）同受该 bug 污染——其 oos_first_src=74,871<bars.len 致前
        // ~54% 信号 fill 成功、后段越界被吞，是部分信号污染回测，非干净 L2。修复后重跑方为干净 L2。

        // 不变量：分层计数完备（每品种恰好归一类断点或 L2）。
        assert_eq!(
            l1_blocked + n_l2_orders,
            total,
            "分层诊断完备：每品种恰归一类（L1 断点 ∪ L2 有效 = 全集）",
        );

        // 不变量（bug 已修，坐标系回归）：slice_date_window 重置 source_index 为局部下标后，
        // 所有 BspPoint.source_index 落在 [0, bars.len) 内 ⟹ 无品种坐标系断裂。修复前实测 7 品种
        // 断裂；修复后必须为 0（若 >0 ⟹ 修复回退或 slice 路径有新的全集偏移泄漏，回归失败）。
        assert_eq!(
            n_coordsys_mismatch, 0,
            "source_index 坐标系回归：修复后无品种应断裂（max_bsp_src<bars.len），实测断裂 {n_coordsys_mismatch} 品种 \
             ⟹ slice_date_window 重置 source_index 的修复回退/泄漏",
        );

        // 不变量（acceptance #5 核心）：修复后真实数据端到端产订单流（L2 可证伪）。bug 修复前
        // 8 品种全部 decisions=0（无任何 L2）；修复后 fill 不再越界，≥1 品种产订单流 ⟹ strat/sharpe
        // 是干净 L2 否定性/确认性结果（625：此时方可判 Θ 策略经验有效性，非工程层断流）。
        assert!(
            n_l2_orders >= 1,
            "acceptance #5：source_index bug 修复后 ≥1 品种端到端产订单流（干净 L2 可证伪），实测 {n_l2_orders}",
        );
    }

    /// **★L2 否证验证：成交盈亏分布 + §3.4/§4 统计检验（OKLO 截断窗，退出生成器真实成交）**
    /// （`#[ignore]`，需 `analysis/data_cache/oklo_1m_databento.json`）。
    ///
    /// ## 认识论等级与诚实边界（formalization-validity-domain 231号）
    ///
    /// - **L2（可否证）**：真实 OKLO 数据驱动 frozen Θ → **成交盈亏**（trade_pnls，非 MtM 浮动）
    ///   → §3.4 block bootstrap p 值 + §4 随机入场对照 → Θ 是否显著优于零收益/随机择时。
    /// - **单标的 = L2（非 L3）**：仅 OKLO 单品种单时段——L3 鲁棒性需多标的/多时段（待
    ///   classifier extract_signals O(n²) 优化后全 8 品种全窗，本测试不冒充 L3）。
    /// - **截断窗 ≠ 全窗**：classifier `signal::extract_signals` O(C×S²)（profile 坐实：全窗
    ///   268K bar classify 30.96s/96.7% 总耗时；plan_and_fill_mtm 仅 7.5ms 非瓶颈）。本测试取
    ///   **前 60K bar**（classify ~0.4s 在时限内）验证 L2 链路 + 统计检验**可工作并产真实数字**。
    ///   全窗 L2 数字待 classifier 优化后重跑（本测试的截断窗 = 全窗的下采样，统计量含义口径
    ///   一致，但样本量较小 ⟹ 检验功效受限，诚实标注非全窗结论）。
    ///
    /// ## 分层诊断 + 否证性结论（625 铁律：区分工程层断流 vs L2 经验否证）
    ///
    /// (a) 引擎产不出信号（n_orders=0，工程层）；(b) 产信号但成交盈亏 p>0.05 / 不优于随机
    /// （L2 否证，Θ 无效，有价值）；(c) 成交盈亏 p≤0.05 且优于随机（L2 确认）。带计数。
    /// **MtM 浮动 ≠ 成交盈亏**：strat_return 是 mark-to-market 口径（含未平仓浮盈），统计检验
    /// 只用 trade_pnls（已实现成交盈亏）——严格区分（Lead 铁律）。
    ///
    /// 跑法：`cargo test --release --lib theta_v0::backtest::runner::tests::l2_falsify_oklo_traded_pnl -- --ignored --nocapture`
    #[test]
    #[ignore = "L2 否证验证（成交盈亏+统计检验），需 oklo cache；截断窗；--release --ignored --nocapture"]
    fn l2_falsify_oklo_traded_pnl() {
        use super::super::data;
        use super::super::metrics;
        use super::super::prereg_windows::PREREG_WINDOWS;

        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("OKLO", &config).expect("加载 OKLO");
        let oklo_reg = PREREG_WINDOWS.iter().find(|w| w.symbol == "OKLO").unwrap();
        let oos_full = ds.slice_date_window(oklo_reg.oos.0, oklo_reg.oos.1);
        assert!(!oos_full.bars.is_empty(), "OOS 窗非空");

        // 截断 60K bar（classify ~0.4s；全窗 268K classify 30.96s 待 classifier 优化）。
        let cut = 60_000.min(oos_full.bars.len());
        let oos = Dataset {
            symbol: oos_full.symbol.clone(),
            bars: oos_full.bars[..cut].to_vec(),
            dates: oos_full.dates[..cut.min(oos_full.dates.len())].to_vec(),
            bar_seconds: 60,
        };
        eprintln!(
            "\n===== L2 否证验证：OKLO 截断窗 bars={}（全窗 {}，截断因 classify O(n²) 待优化）=====",
            oos.bars.len(),
            oos_full.bars.len()
        );

        // NAV 与品种价量级匹配（首价×容量）。
        let first_px = oos
            .bars
            .iter()
            .find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * config.tick.tick_size)
            .unwrap_or(1.0);
        let nav = (first_px * 1000.0).max(1.0e6);
        let res = run_theta_v0(&oos, &config, 0.5, nav);

        // ── 成交盈亏分布（双口径并列：已实现 vs 含浮盈终点强平）──
        let pnls = &res.trade_pnls; // 已实现口径（§3.4 bootstrap 输入）
        let pnls_forced = &res.trade_pnls_with_forced; // 含浮盈口径（终点强平）
        let n_trades = pnls.len();
        // ★统计功效门槛计数 = **非强平**笔数（codex：29 笔真实退出 + 1 笔强平不偷过 n≥30）。
        let n_real_trades = res.trades.iter().filter(|t| !t.forced_close).count();
        let total_pnl: f64 = pnls.iter().sum();
        let total_pnl_forced: f64 = pnls_forced.iter().sum();
        let n_win = pnls.iter().filter(|&&p| p > 0.0).count();
        let win_rate = if n_trades > 0 { n_win as f64 / n_trades as f64 } else { 0.0 };

        eprintln!(
            "[L2] 引擎产出：n_orders={} n_trades(已实现)={} n_real_trades(非强平)={} is_l2={}",
            res.n_orders, n_trades, n_real_trades, res.is_l2
        );
        eprintln!(
            "[L2] ★双口径成交盈亏：已实现 total={:.4}（{}笔）｜含浮盈(终点强平) total={:.4}（{}笔）\
             win_rate={:.4} maxDD(MtM)={:.4}%",
            total_pnl, n_trades, total_pnl_forced, pnls_forced.len(), win_rate,
            res.metrics.max_drawdown * 100.0
        );
        eprintln!(
            "[L2] 三口径并列：strat_return(MtM 含浮盈)={:.2}% bh_return={:.2}% \
             ★随机对照用含浮盈 total_return，§3.4 bootstrap 用已实现 trade_pnls",
            res.metrics.strat_return * 100.0,
            res.metrics.bh_return * 100.0
        );

        // ── §3.4 block bootstrap（已实现口径）+ §4 操作语义随机对照（含浮盈口径，seed 冻结）──
        let sig = metrics::significance(
            pnls,
            &res.daily_returns,
            &res.trades,
            &res.prices,
            res.fee_rate,
            res.theta_return_mtm,
        );
        eprintln!(
            "[L2] §3.4 bootstrap（块长5笔 n=1000 H0:已实现收益≤0）：mean_total={:.4} \
             p(收益≤0)={:.4} CI95=[{:.4}, {:.4}]",
            sig.boot_mean_total_pnl, sig.boot_pvalue_pnl_le_0, sig.boot_ci95_lo, sig.boot_ci95_hi
        );
        eprintln!(
            "[L2] §3.4 Sharpe(Lo2002)：sharpe={:.4} SE={:.4} CI95=[{:.4}, {:.4}]",
            sig.sharpe, sig.sharpe_se, sig.sharpe_ci95_lo, sig.sharpe_ci95_hi
        );
        eprintln!(
            "[L2] §4 操作语义随机入场对照（n=1000，同口径逐笔无复利；Θ_同口径={:.4} vs Θ_MtM复利={:.4}仅参考）：\n     \
             主 schedule-shift: rand_mean={:.4} p_upper={:.4}｜副 independent: rand_mean={:.4} p_upper={:.4}\n     \
             ★Θ 优于随机（两对照 p≤0.05）={}",
            sig.theta_return_same_caliber, sig.theta_return_mtm,
            sig.shift_mean_return, sig.shift_pvalue,
            sig.indep_mean_return, sig.indep_pvalue,
            sig.theta_beats_random
        );

        // ── 分层诊断 + 否证性结论（625 铁律 + §5.5 inconclusive 严格区分）──
        // ★关键有效域区分（formalization-validity-domain）：n_real_trades<30 时统计功效不足，
        // **不构成可否证的统计检验**——样本量不足（§5.5 inconclusive，协议 §5.5 阈值 <30），
        // **不是** L2 否证（把样本不足伪装成否证 = 声明膨胀）。门槛用**非强平**笔数（强平笔是
        // 窗口边界口径，不携带 Θ 退出信号信息，不计入统计功效）。
        const MIN_TRADES_FOR_L2: usize = 30; // 协议 §5.5（backtest-protocol-v0.md:171）：n_trades<30 inconclusive
        let (layer, verdict) = if res.n_orders == 0 {
            ("(a)工程层", "引擎产不出订单——非 L2 经验否证，是工程断流")
        } else if n_trades == 0 {
            ("(a)工程层", "产订单但无平仓——退出闭环未产 Close（接线缺口）")
        } else if n_real_trades < MIN_TRADES_FOR_L2 {
            (
                "(d)inconclusive",
                "非强平成交样本不足（n_real_trades<30，协议 §5.5）⟹ 统计功效不足 ⟹ inconclusive，\
                 非 L2 否证也非确认（截断窗破坏样本量，需全窗）",
            )
        } else if sig.controls_degenerate {
            (
                "(d)inconclusive",
                "随机对照退化（无真随机平移/合法区间）⟹ p_upper 无统计含义 ⟹ inconclusive（codex 缺陷⑤）",
            )
        } else if sig.boot_pvalue_pnl_le_0 > 0.05 {
            ("(b)L2否证", "已实现盈亏 p>0.05：无法拒绝『收益≤0』⟹ Θ 收益不显著（§5.1 失败，有价值）")
        } else if !sig.theta_beats_random {
            ("(b)L2否证", "收益显著但 Θ 不优于随机择时（§5.3 失败：缠论内在语法不贡献择时 alpha，有价值）")
        } else {
            ("(c)L2确认", "已实现 p≤0.05 且含浮盈优于两随机对照 ⟹ Θ 在此窗未被证伪（截断窗，非全窗/非 L3）")
        };
        eprintln!("[L2] ★分层归因：{layer}｜{verdict}");
        eprintln!(
            "[L2] ★诚实边界：单标的(OKLO)单时段截断窗={}bar = L2（非 L3 多标的鲁棒性）；\
             全窗待 classifier extract_signals O(n²) 优化后重跑。",
            oos.bars.len()
        );
        // ★★有效域分层归因（Θ v0 = 退化实装，grammar-audit 0c71a0f771；231号强制 caveat）：
        eprintln!(
            "[L2] ★★有效域 = 「Θ v0 退化实装」非「完整缠论语法」：四维度退化叠加无法分离——\n     \
             #1 long-only（决策 mod.rs:81 + fill runner.rs:718 砍半方向）+ #2 假背驰（divergence.rs:102 \
             无 A/B/C 框架）+ #5 无声部对冲 + #6 无 τ∈{{P,U,D}} 走势分解。\n     \
             ⟹ 若否证，严禁归因「缠论/Θ 走势识别无 alpha」（退化叠加 vs 走势识别本身无 alpha 在 v0 不可分）。"
        );

        // 不变量（管线 + 标注一致性，不对 Θ 盈利符号下断言——那是 L2 经验结果，照实报告不强求方向）：
        assert!(res.metrics.bh_return.is_finite(), "buy&hold 有限");
        assert!(res.metrics.strat_return.is_finite(), "strat(MtM) 有限");
        assert!(sig.boot_pvalue_pnl_le_0 >= 0.0 && sig.boot_pvalue_pnl_le_0 <= 1.0, "p 值合法 [0,1]");
        assert!((0.0..=1.0).contains(&sig.shift_pvalue), "schedule-shift p_upper 合法 [0,1]");
        assert!((0.0..=1.0).contains(&sig.indep_pvalue), "independent p_upper 合法 [0,1]");
        assert_eq!(res.is_l2, res.n_orders > 0, "is_l2 ⟺ 订单非空（诚实标注）");
        // 可复现见证：同输入同 seed ⟹ bit-exact 同检验结果（§4 硬约束）。
        let sig2 = metrics::significance(
            pnls,
            &res.daily_returns,
            &res.trades,
            &res.prices,
            res.fee_rate,
            res.theta_return_mtm,
        );
        assert_eq!(sig, sig2, "significance 可复现（seed=20260625 冻结，含操作语义随机对照）");
    }

    /// OOS 窗时间跨度（年）——从预注册 ISO 日期 `"YYYY-MM-DD"` 端点算（§3.1 年化基数）。
    ///
    /// 纯技术性工具（日期算术），不涉及缠论概念。按 365.25 日/年近似（闰年平均），
    /// 各品种实际交易日历精确化是 L3 精化项（本处用于年化基数，量级匹配即可）。
    fn oos_years(oos_start: &str, oos_end: &str) -> f64 {
        fn ymd(s: &str) -> (i64, i64, i64) {
            let p: Vec<i64> = s.split('-').map(|x| x.parse().unwrap_or(0)).collect();
            (p[0], *p.get(1).unwrap_or(&1), *p.get(2).unwrap_or(&1))
        }
        // 朴素天数（自 0 年的近似序数日，仅用于求差，绝对值无意义）。
        fn ord(y: i64, m: i64, d: i64) -> i64 {
            // 各月累计天数（平年，闰年误差 ≤1 日，对年化基数量级无影响）。
            const CUM: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
            y * 365 + y / 4 + CUM[(m as usize - 1).min(11)] + d
        }
        let (y0, m0, d0) = ymd(oos_start);
        let (y1, m1, d1) = ymd(oos_end);
        ((ord(y1, m1, d1) - ord(y0, m0, d0)) as f64 / 365.25).max(0.1)
    }

    /// **★L3 否证/确认：全 8 品种全窗 OOS + §3.4/§4 统计检验（成交盈亏口径）**
    /// （`#[ignore]`，需全 8 品种 `analysis/data_cache/*.json`；慢测，`--release` 必须）。
    ///
    /// ## 认识论等级与诚实边界（formalization-validity-domain 231号）
    ///
    /// - **单标的 = L2，多标的复现 = L3**：本测试遍历全 8 品种 OOS **前 [`MAX_BARS`] 截断窗**
    ///   （全窗 CPU-bound 不可行，见下），每品种跑 frozen Θ → 成交盈亏 trade_pnls → §3.4 block
    ///   bootstrap p 值 + §4 随机入场对照。单品种结论是 L2（截断窗）；跨品种"是否复现同一方向
    ///   （否证/确认）"才是 L3 鲁棒性结论。截断窗 ⟹ 检验功效受限，全窗结论待优化后重跑。
    /// - **★MtM ≠ 成交盈亏**：`strat_return` 是 mark-to-market 浮动口径（含未平仓浮盈，
    ///   OKLO 60K 截断窗实测 578%），统计检验**只用 trade_pnls**（已实现成交盈亏）+ §4 随机
    ///   择时对照——这是严格的 L2 否证口径（Lead 铁律）。OKLO 60K 截断窗 §4 实测 Θ分位≈0.50、
    ///   **Θ 不优于随机择时 ⟹ (b)L2否证**（MtM 的 578% 是浮盈幻觉，非择时信息）。本测试核验
    ///   该否证是否**跨标的复现**（L3）。
    ///
    /// ## 否证性结论分层（625 铁律：工程断流 vs L2 经验否证 vs 样本不足 inconclusive）
    ///
    /// 每品种归一类：
    /// - **(a) 工程层**：n_orders=0 或 n_trades=0 —— 非 L2 否证，是接线断流（不计入否证）。
    /// - **(d) inconclusive**：n_real_trades<30（协议 §5.5；**非强平**笔数门槛）—— 样本不足，
    ///   统计功效不足，§5.5 不冒充否证也不冒充确认（625：样本不足 ≠ 否证）。
    /// - **(b) L2否证**：n_real_trades≥30 且（已实现 boot p>0.05 收益不显著 或 含浮盈不优于两随机
    ///   对照）—— §5.1/§5.3 失败：缠论内在语法不贡献择时 alpha，缩小有效域，**比确认更有价值**（231号）。
    /// - **(c) L2确认**：n_real_trades≥30 且 boot p≤0.05 且含浮盈优于 schedule-shift+independent
    ///   两随机对照 —— 在此窗未被证伪。
    ///
    /// L3 结论：统计 (b)/(c)/(d) 的跨品种分布。若否证跨标的复现（多数品种 (b)）⟹ Θ v0 择时
    /// 无信息是鲁棒否证（L3）；若各品种结论分散 ⟹ Θ 有效性品种依赖（有效域 < 定义域）。
    ///
    /// ## 全窗不可行性（诚实标注，no-patch：不假装跑了全窗）
    ///
    /// **全窗实测不可行**（2026-06-27 机器证据坐实）：BTC 全窗 OOS（≈1.31M bar）单品种端到端
    /// （parse→classify→recognize→plan_and_fill_mtm 逐 bar 退出生成器）CPU 全速跑 >5min 仍未
    /// 产出首行——CPU-bound（非死锁），8 品种全窗远超任何时限。故本测试取**前 [`MAX_BARS`]
    /// 截断窗**（与 [`l2_falsify_oklo_traded_pnl`] 60K 同口径，OKLO 实测 218 trades 样本充足）。
    /// 截断窗 = 全窗的前缀下采样：统计量含义口径一致，但样本量 < 全窗 ⟹ **检验功效受限**，
    /// **诚实标注为截断窗 L2/L3，非全窗结论**。全窗待 plan_and_fill_mtm + classify 的逐 bar
    /// 复杂度进一步优化后重跑（本测试不在测试内静默假装全窗，也不留 fallback——截断是显式
    /// 声明的有效域边界，非补丁）。
    ///
    /// 跑法：`cargo test --release --lib theta_v0::backtest::runner::tests::l3_falsify_multi_symbol_significance -- --ignored --nocapture`
    #[test]
    #[ignore = "L3 多标的截断窗否证（全 8 品种 + significance），需全 data_cache；慢测；--release --ignored --nocapture"]
    fn l3_falsify_multi_symbol_significance() {
        use super::super::data;
        use super::super::metrics;
        use super::super::prereg_windows::PREREG_WINDOWS;

        let config = ThetaConfig::default();
        const MIN_TRADES_FOR_L2: usize = 30; // 协议 §5.5（backtest-protocol-v0.md:171）：n_trades<30 inconclusive
        // 截断窗 bar 数（全窗 CPU-bound 不可行——BTC 全窗 >5min，见 doc）。60K 与
        // l2_falsify_oklo_traded_pnl 同口径（OKLO 60K 实测 218 trades 样本充足）。
        const MAX_BARS: usize = 60_000;

        eprintln!("\n===== L3 多标的【截断窗 {MAX_BARS}bar】OOS 否证：8 品种 + §3.4/§4 操作语义随机对照 =====");
        eprintln!("★全窗不可行（BTC 全窗 >5min CPU-bound，机器坐实）⟹ 截断窗 = 显式有效域边界（非全窗结论）");
        eprintln!("★口径：boot_p 用已实现 trade_pnls；随机对照（shift/indep）用含浮盈 total_return；beats?=两对照 p≤0.05");
        eprintln!(
            "{:<6} {:>10} {:>6} {:>6} {:>8} {:>9} {:>7} {:>7} {:>7} {:>6} {}",
            "symbol", "oos_bars/T", "real", "trd", "MtM%", "bh%",
            "boot_p", "shift_p", "indep_p", "beats?", "归因",
        );
        eprintln!("（real=非强平笔数(门槛)；trd=已实现笔数；MtM%=含浮盈≠成交盈亏；shift_p/indep_p=随机对照 p_upper）");

        // L3 聚合：跨品种否证/确认/inconclusive/工程断流计数。
        let mut n_falsify = 0usize; // (b) L2 否证
        let mut n_confirm = 0usize; // (c) L2 确认
        let mut n_inconclusive = 0usize; // (d) 样本不足
        let mut n_engine_block = 0usize; // (a) 工程断流（无订单/无平仓）
        let mut n_beats_random = 0usize; // Θ 优于随机择时的品种数（择时信息含量）

        for w in PREREG_WINDOWS {
            let t0 = std::time::Instant::now();
            let ds = match data::load_by_symbol(w.symbol, &config) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("{:<6} 加载失败：{e}", w.symbol);
                    continue;
                }
            };
            // 预注册 OOS 窗（唯一真相源 = PREREG_WINDOWS，防数据挖掘）。
            let oos_full = ds.slice_date_window(w.oos.0, w.oos.1);
            if oos_full.bars.is_empty() {
                eprintln!("{:<6} OOS 窗空（{}→{}）", w.symbol, w.oos.0, w.oos.1);
                continue;
            }
            // 截断前 MAX_BARS（全窗 CPU-bound 不可行，见 doc）——显式有效域边界，非静默截断。
            let cut = MAX_BARS.min(oos_full.bars.len());
            let truncated = cut < oos_full.bars.len();
            let oos = Dataset {
                symbol: oos_full.symbol.clone(),
                bars: oos_full.bars[..cut].to_vec(),
                dates: oos_full.dates[..cut.min(oos_full.dates.len())].to_vec(),
                bar_seconds: 60,
            };

            // years：截断窗按截断 bar 占全窗比例缩放全窗年跨（年化基数 §3.1，量级匹配）。
            let full_years = oos_years(w.oos.0, w.oos.1);
            let years = if truncated {
                (full_years * cut as f64 / oos_full.bars.len() as f64).max(0.1)
            } else {
                full_years
            };
            // NAV 与品种价量级匹配（首价×容量；与 l2_falsify_oklo / l2_oos_eight_symbols 同口径）。
            let first_px = oos
                .bars
                .iter()
                .find(|b| !b.untradable && b.close > 0)
                .map(|b| b.close as f64 * config.tick.tick_size)
                .unwrap_or(1.0);
            let nav = (first_px * 1000.0).max(1.0e6);
            let res = run_theta_v0(&oos, &config, years, nav);

            let pnls = &res.trade_pnls; // 已实现口径（§3.4 bootstrap）
            let n_trades = pnls.len();
            // ★门槛计数 = 非强平笔数（codex：强平笔不偷过 n≥30 统计功效门槛）。
            let n_real_trades = res.trades.iter().filter(|t| !t.forced_close).count();
            let total_pnl: f64 = pnls.iter().sum();

            // §3.4 block bootstrap（已实现）+ §4 操作语义随机对照（含浮盈，seed=20260625 冻结）。
            let sig = metrics::significance(
                pnls,
                &res.daily_returns,
                &res.trades,
                &res.prices,
                res.fee_rate,
                res.theta_return_mtm,
            );
            if sig.theta_beats_random {
                n_beats_random += 1;
            }

            // 分层归因（625 + §5.5 inconclusive 严格区分；门槛用非强平笔数）。
            let verdict = if res.n_orders == 0 || n_trades == 0 {
                n_engine_block += 1;
                "(a)工程断流"
            } else if n_real_trades < MIN_TRADES_FOR_L2 {
                n_inconclusive += 1;
                "(d)inconcl real<30"
            } else if sig.controls_degenerate {
                n_inconclusive += 1;
                "(d)inconcl 对照退化"
            } else if sig.boot_pvalue_pnl_le_0 > 0.05 {
                n_falsify += 1;
                "(b)否证:收益不显著"
            } else if !sig.theta_beats_random {
                n_falsify += 1;
                "(b)否证:不优于随机"
            } else {
                n_confirm += 1;
                "(c)确认:p≤.05且优随机"
            };

            let elapsed = t0.elapsed().as_secs_f64();
            let trunc_mark = if truncated { "T" } else { "F" }; // T=截断窗 F=全窗
            eprintln!(
                "{:<6} {:>9}{} {:>6} {:>6} {:>8.2} {:>9.2} {:>7.4} {:>7.4} {:>7.4} {:>6} {}  [{:.1}s]",
                w.symbol,
                oos.bars.len(),
                trunc_mark,
                n_real_trades,
                n_trades,
                res.metrics.strat_return * 100.0,
                res.metrics.bh_return * 100.0,
                sig.boot_pvalue_pnl_le_0,
                sig.shift_pvalue,
                sig.indep_pvalue,
                sig.theta_beats_random,
                verdict,
                elapsed,
            );
            eprintln!(
                "       └ full_oos_bars={} 已实现total={:.2} Θ_同口径={:.4} (MtM复利={:.4}仅参考) shift_mean={:.4} indep_mean={:.4} Sharpe={:.3} years={:.2}",
                oos_full.bars.len(),
                total_pnl,
                sig.theta_return_same_caliber,
                sig.theta_return_mtm,
                sig.shift_mean_return,
                sig.indep_mean_return,
                sig.sharpe,
                years,
            );

            // 不变量（每品种）：管线不崩 + 检验值合法 + 标注一致。
            assert!(res.metrics.strat_return.is_finite(), "{} strat(MtM) 有限", w.symbol);
            assert!(res.metrics.bh_return.is_finite(), "{} bh 有限", w.symbol);
            assert!(
                (0.0..=1.0).contains(&sig.boot_pvalue_pnl_le_0),
                "{} boot p∈[0,1]",
                w.symbol
            );
            assert!((0.0..=1.0).contains(&sig.shift_pvalue), "{} shift p∈[0,1]", w.symbol);
            assert!((0.0..=1.0).contains(&sig.indep_pvalue), "{} indep p∈[0,1]", w.symbol);
            assert_eq!(res.is_l2, res.n_orders > 0, "{} is_l2 ⟺ 订单非空", w.symbol);
        }

        // ── L3 跨标的结论（231号：否证跨标的复现 = 鲁棒否证，比确认更有价值）──
        let total = PREREG_WINDOWS.len();
        eprintln!("\n===== L3 跨标的否证聚合（8 品种截断窗 OOS 前 {MAX_BARS}bar，操作语义随机对照）=====");
        eprintln!("(a) 工程断流(无订单/无平仓)       : {n_engine_block}");
        eprintln!("(d) inconclusive(real_trades<30)  : {n_inconclusive}");
        eprintln!("(b) L2 否证(收益不显著/不优随机)  : {n_falsify}");
        eprintln!("(c) L2 确认(p≤.05 且优于两随机对照): {n_confirm}");
        eprintln!("    其中 Θ 含浮盈打败两随机对照的品种数 : {n_beats_random}/{total}");
        eprintln!(
            "\n★L3 诚实结论（formalization-validity-domain 231号）：\n  \
             - 跨标的多数 (b) ⟹ **「Θ v0 退化实装的择时不贡献 alpha」** 是鲁棒否证（L3，有效域缩小）。\n  \
             - 各品种结论分散 ⟹ Θ v0 有效性**品种依赖**（有效域 < 定义域，非全域有效）。\n  \
             - n_beats_random 是择时信息含量的直接计数：=0 ⟹ Θ v0 退化实装全标的无择时信息。\n  \
             - ★含浮盈口径 + 操作语义随机对照（schedule-shift 主 + independent 副）= 严格否证口径\n    \
               （旧 self-resampling 自举恒真已删；随机对照保操作外形随机化入场点，破坏缠论内在语法）。"
        );
        // ★★有效域分层归因（grammar-audit commit 0c71a0f771 坐实，强制 caveat）：
        // 本否证/确认的有效域 = **「Θ v0 退化实装」**，**不是「完整缠论语法」**。Θ v0 四维度退化：
        eprintln!(
            "\n★★有效域分层归因（Θ v0 = 退化实装，grammar-audit 0c71a0f771 坐实；231号强制 caveat）：\n  \
             否证/确认的有效域 = 「Θ v0 退化实装」，**非「完整缠论语法」**。四维度退化叠加，在 v0 上无法分离：\n  \
             - #1 long-only（双层坐实）：决策层 pi_strict (Flat,SellSide)→Wait（mod.rs:81 空仓不开空）\n    \
               + fill 层 units≥0 做空腿待定（runner.rs:718）⟹ 系统性砍掉一半方向（做空 alpha 整段缺失）。\n  \
             - #2 背驰退化：is_divergence 只判相邻同向段面积递减（divergence.rs:102），无 A/B/C 趋势背驰\n    \
               框架 ⟹ 假背驰=假买卖点=信号噪声混入。\n  \
             - #5 多声部 depth=0：结构是 canonical §5 契约（正确），但放弃次级别对冲 alpha。\n  \
             - #6 走势分解：signal 路径跳过 τ∈{{P,U,D}} 四分类（与 #2 同根）。\n  \
             ⟹ 若 (b) 否证：**严禁直接归因「缠论/Θ 走势识别无 alpha」**——可能是 long-only 砍半 + 假背驰\n    \
               噪声 + 无声部对冲的退化叠加，这几层在 v0 上**无法分离**。否证只否定退化 v0，不否定完整缠论。\n  \
             - 弱反向先验（不迁移）：谱系 553/557 做空 L3 测得 alpha 有效域≈空集，但跑在 recursive_t 引擎\n    \
               **非 theta_v0**，强牛 regime 单一，不能假设迁移——仅作弱先验，不影响本 v0 口径结论。"
        );

        // 不变量：分层完备（每产出订单的品种恰归一类 b/c/d；无订单归 a）。
        assert_eq!(
            n_falsify + n_confirm + n_inconclusive + n_engine_block,
            total,
            "L3 分层完备：每品种恰归一类（a 工程断流 ∪ b 否证 ∪ c 确认 ∪ d inconclusive = 全集）",
        );
        // ★acceptance：≥1 品种端到端产订单流（管线在多标的真实数据上跑通，否则 L2/L3 无从谈起）。
        assert!(
            n_falsify + n_confirm + n_inconclusive >= 1,
            "≥1 品种产订单流（多标的真实数据 L2 检验可执行），实测全部工程断流 ⟹ 接线回退",
        );
    }
}
