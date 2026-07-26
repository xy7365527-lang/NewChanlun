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
use super::super::strategy::ledger::{RiskPolicy, TwState};
use super::super::config::ThetaConfig;
use super::super::strategy::exit::{exit_decision_for, record_held_voice, HeldVoice};
use super::super::strategy::{AccountState, VoiceDecision};
use super::super::types::{Bar, Order, StrictAction};
use super::super::{classifier, parser, strategy};
use super::data::Dataset;
use super::dual_ledger;
use super::signal::{entry_structural_stop, newly_confirmed_step};
use super::metrics::{self, Metrics};
pub use super::opsem_dump::{StrictNestCertificateRecord, StrictNestSidecarSummary};
pub use super::ledger::{TypedTrade, TYPED_TRADE_SCHEMA_VERSION, VoiceVerdictRec};
pub(super) use super::ledger::{track_position_transition, LedgerOpen, OpsemEntrySnapshot};
pub(super) use super::ledger::TwLedgerThread;
pub(crate) use super::fill::{apply_order, FillOutcome};
use super::fill::{bar_returns, simulate_fills};
// ★B-M3b-contract（#91）fill loop seam：fill loop 本体 + plan_and_fill_mtm + FillOutput(Family)
// 自 runner.rs 纯移动；对外经 `pub use` 门面保持原路径。
pub(super) use super::fill::{
    FillOutput, FillOutputDual, LegFillRec,
    pi_theta_fill_loop, pi_theta_fill_loop_voice, pi_theta_fill_loop_overlay,
    plan_and_fill_mtm, plan_and_fill_mtm_dual,
    apply_voice_fill, apply_voice_fill_dual,
};
use super::opsem_dump::{
    eta_bucket_str, force_state_str, operation_role_str, risk_mode_str,
    strict_nest_sidecar_enabled, summarize_strict_nest_certificates, t_stage_str, voice_side_str,
    OpsemDump, StrictNestSidecarCollector, OPSEM_DUMP_DIR_OVERRIDE,
};
// ★B-M2（#89）准入门 seam：χ/nest/k_Θ 三门 + κ 解析自 runner.rs 纯移动。
use super::admission::{
    voice_exec_gate, nest_cert_gate_enabled,
    NestGateStats, nest_gate_admit,
    NestGateObs, NestChainGate, LevelFingerprint,
    ExitNestGateStats, ExitNestGateCtx,
    k_theta_risk_gate, kappa_policy_resolved, kappa_priority_resolve,
};
// T3 (#172) 链类型（测试与并门派生消费）。
use super::admission::{ChainGapKind, ChainLevelStatus, ChainVerdict};
pub use super::admission::ChiFilterCtx;
// 测试经原路径访问 thread_local override（admission 内 pub(super) 可见）。
#[cfg(test)]
use super::admission::{VOICE_EXEC_OVERRIDE, NEST_CERT_GATE_OVERRIDE};
// ★三方合并 ours 独有面（#196-#200/#209/#237 探针/判据/override，随 fill loop 同迁
// fill.rs；pub(super) 升格后本行恢复 runner 原路径，既有测试零改动消费）。
#[cfg(test)]
use super::fill::{
    residual_correction_probe_count, residual_correction_probe_reset,
    reverse_open_isolation_probe_count, reverse_open_isolation_probe_reset,
    silent_drop_exit_type, t1_core_residual_probe_count, t1_core_zero_probe_count,
    t1_core_zero_probe_reset, type2_sell_guard_probe_count, type2_sell_guard_probe_reset,
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
    /// 路径（[`run_theta_v0`]）为 `None`（诚实——R 分解只接生产 π fill loop）。
    pub r_decomp: Option<super::super::strategy::risk::RDecomposition>,
    /// 严格区间套证书 sidecar 汇总。默认 `None`；仅 `THETA_STRICT_NEST_SIDECAR=1/true/yes/on`
    /// 时在生产 π 重放同帧旁路产出，不参与订单、候选、风控、账本。
    pub strict_nest_sidecar: Option<StrictNestSidecarSummary>,
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
///
/// # ⚠️ Deprecated：结构确认前视（bughunt F-01，2026-07-10）
///
/// 本入口对**全窗** bars 一次性 `parse_layer` + `classify` 后把 BSP 决策放回其历史
/// `source_index` 执行——买卖点可能在更晚 bar 才获得确认（见 :330-334 说明），
/// 故该路径会在当时尚不可知的位置交易。**其产出禁止用于 L2/L3 认识论声明**，
/// 只可用作诊断/管线冒烟。因果口径请用 [`run_theta_v0_pi`]（逐 bar
/// `IncrementalClassifier`，无前视）。既有基于本入口的 L2/L3 结论一律作废重跑。
#[deprecated(
    note = "结构确认前视（bughunt F-01）：全窗分类决策回放历史，产出禁用于 L2/L3 声明；因果口径用 run_theta_v0_pi"
)]
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
        r_decomp: fill.r_decomp, // v1 路径为 None（plan_and_fill_mtm 不产 R 分解）
        strict_nest_sidecar: None,
    }
}

/// ★关⑤：[`run_theta_v0`] 的**双账本嵌套并列入口**（纯新增；`run_theta_v0` 签名/行为零改
/// ⟹ 全部现有 bin/测试 bit-exact）。
///
/// 管线与 [`run_theta_v0`] 同骨架，两处替换：
/// 1. `classify` → [`classifier::classify_with_tower`]（Classification **bit-identical**，
///    classifier/mod.rs:482 契约；第二返回值 = 逐级塔快照，供真 ReverseOpen 角色）。
/// 2. `recognize + plan_and_fill_mtm` → [`plan_and_fill_mtm_dual`]（per-bar
///    [`strategy::recognize_nested`] + DualLedger 分腿成交 + cascade 退出 + 毛闸门）。
///
/// # ⚠️ Deprecated：与 [`run_theta_v0`] 同一前视有效域（bughunt F-01 口径）
///
/// 本入口同样对**全窗** bars 一次性 `parse_layer` + `classify_with_tower` 后把 BSP 决策放回
/// 其历史 `source_index` 执行（结构确认前视）——**产出禁止用于 L2/L3 认识论声明**，只可用作
/// 诊断/管线冒烟与合成夹具对拍。嵌套产量的因果口径验收由后续重放承担（施工图 §2 依赖层）；
/// 生产因果接线在 `nautilus::strategy::ThetaCore::recognize_current`（per-bar 窗口）。
///
/// ★#68③ 处置登记：本节经 C1 票审认为**正式边界声明终态**——deprecated 保留（不解除、不扩大
/// 语义、不在本入口接线因果验收）；前视有效域即上述三行，因果路径归 `recognize_current` /
/// LEE 合并波次的 per-bar 因果重放。
#[deprecated(
    note = "结构确认前视（同 run_theta_v0 F-01 口径）：全窗分类决策回放历史，产出禁用于 L2/L3 声明；因果验收由后续重放承担"
)]
pub fn run_theta_v0_dual(
    dataset: &Dataset,
    config: &ThetaConfig,
    years: f64,
    initial_nav: f64,
) -> RunResult {
    let bars = &dataset.bars;
    let l0 = parser::parse_layer(bars, config);
    let (classification, tower) = classifier::classify_with_tower(&l0, config);
    let closed_loop_final = run_closed_loop(bars, initial_nav);
    let dual = plan_and_fill_mtm_dual(&classification, &tower, bars, initial_nav, config);
    let fill = dual.fill;

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
        r_decomp: fill.r_decomp,
        strict_nest_sidecar: None,
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
                strict_nest_sidecar.observe_frame(&l0, &cls, &tower, &config, classifier_incr.tower_cache());
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
    // ★W1 env gate：VOICE_EXEC=1 ⟹ 声部独立执行臂；未设/非"1" ⟹ 净额臂（bit-exact）。
    let voice_exec_on = voice_exec_gate();
    let mut voice_book = if voice_exec_on {
        Some(super::super::strategy::overlay_state::VoiceExecBook::new(initial_nav))
    } else {
        None
    };

    // ★T3 (#172 并门，#168 裁定 3)：层载由链路径单一驱动——链活（nest 证书门开）⟹ 投影层
    // 必载（含三元锚索引，恰好存在物化基座）；链死不载（零开销红线不死）。生产唯一派生点
    // （层门配置面退役；派生必须先于分类器构建——层 stamping 读派生后机制位）。
    let config = super::admission::chain_driven_level_projection(config);
    let mut classifier_incr = super::incremental::IncrementalClassifier::new(bars, &config);
    let fill = pi_theta_fill_loop_overlay(
        |i| {
            let (cls, tower) = classifier_incr.classify_at(i);
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
        voice_book.as_mut(),
    );

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
    };

    let tw_final = fill.tw_final;
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
        net_result,
        tw_final,
        voice_exec,
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
    let i0 = if initial_nav > 0.0 { initial_nav as i64 } else { 1 };
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

    fn strict_nest_test_event(level: u32, side: super::super::super::types::Side, src: usize, lo: usize, hi: usize, cand: bool) -> classifier::recursive_tower::CandDeltaEvent {
        classifier::recursive_tower::CandDeltaEvent {
            level,
            side,
            divergence_confirm_src: src,
            confirm_src: src,
            interval: (lo, hi),
            a_interval: (lo, hi),
            c_episode_start: lo,
            c_episode_interval: (lo, hi),
            c_interval_full: Some((lo, hi)),
            b_parent: None,
            c_structure: None,
            third_class_in_c: None,
            cp_certificate_confirm_src: None,
            full_trend_c_qualified: None,
            full_trend_evidence: None,
            cp_ownership: None,
            enter_src: lo,
            cand_delta: cand,
            pan_div_diag: false,
        }
    }

    #[test]
    fn strict_nest_sidecar_summary_matches_p2_assembly() {
        let side = super::super::super::types::Side::Long;
        let base = strict_nest_test_event(0, side, 60, 20, 60, true);
        let mut parent = strict_nest_test_event(1, side, 50, 10, 80, true);
        let b_id = classifier::recursive_tower::ElementId { level: 2, ordinal: 1 };
        let departure_id = classifier::recursive_tower::ElementId { level: 1, ordinal: 10 };
        let terminal_id = classifier::recursive_tower::ElementId { level: 1, ordinal: 11 };
        parent.cp_ownership = Some(classifier::recursive_tower::CandDeltaCpEdge {
            b_center_id: b_id,
            cp_departure_move_id: departure_id,
            cp_source_start: 10,
        });
        let cand = vec![vec![base.clone()], vec![parent]];
        let third = classifier::recursive_tower::ThirdClassInCp {
            b_center_id: b_id,
            cp_departure_move_id: departure_id,
            departure_move_id: departure_id,
            retest_move_id: terminal_id,
            departure_interval: (10, 40),
            retest_interval: (40, 80),
            point_source_index: 80,
            side,
        };
        let object = classifier::recursive_tower::CpScanOwnership {
            b_center_index: 1,
            b_center_id: b_id,
            b_center: super::super::super::types::Center {
                zd: 0,
                zg: 1,
                dd: 0,
                gg: 1,
                start_index: 0,
                end_index: 9,
            },
            departure_move_id: Some(departure_id),
            departure_interval: Some((10, 40)),
            lifecycle: classifier::recursive_tower::CpLifecycleStatus::Closed,
            cp_certificate_confirm_src: Some(80),
            c_structure: Some(classifier::recursive_tower::CpStructureIdentity {
                level: 1,
                b_center_id: b_id,
                departure_move_id: departure_id,
                terminal_move_id: Some(terminal_id),
                source_start: 10,
                source_end: Some(80),
            }),
            third_class_in_c: Some(third),
            full_trend_evidence: None,
            full_trend_c_qualified: None,
        };
        let empty: &[classifier::recursive_tower::CpScanOwnership] = &[];
        let level1_objects = [object];
        let objects_by_level: Vec<&[classifier::recursive_tower::CpScanOwnership]> =
            vec![empty, &level1_objects];
        let terminal = super::super::super::types::BspBits { buy1: true, ..Default::default() };
        let mut terminal_by_key = std::collections::HashMap::new();
        terminal_by_key.insert((60usize, 1i8), terminal);

        let summary =
            summarize_strict_nest_certificates(&cand, &objects_by_level, &terminal_by_key);
        let expected = classifier::nest::assemble_certificates_terminal(
            &cand,
            &objects_by_level,
            0,
            1,
            |_| Some(terminal),
        );

        assert_eq!(summary.base_count, 1);
        assert_eq!(summary.terminal_missing, 0);
        assert_eq!(summary.cert_per_top, vec![(1, expected.len())]);
        assert_eq!(summary.cert_total, expected.len());
        assert_eq!(summary.certificates.len(), expected.len());
        assert_eq!(summary.certificates[0].top_level, 1);
        assert_eq!(summary.certificates[0].certificate, expected[0]);
    }

    /// F-06：同一缺失 terminal 的 L0 基例不得按可用 top 数重复计入。
    #[test]
    fn strict_nest_sidecar_counts_missing_terminal_once_across_tops() {
        let side = super::super::super::types::Side::Long;
        let base = strict_nest_test_event(0, side, 60, 20, 60, true);
        let mid = strict_nest_test_event(1, side, 50, 10, 80, true);
        let top = strict_nest_test_event(2, side, 40, 0, 100, true);
        let cand = vec![vec![base], vec![mid], vec![top]];
        let terminal_by_key = std::collections::HashMap::new();

        let empty: &[classifier::recursive_tower::CpScanOwnership] = &[];
        let objects_by_level = vec![empty, empty, empty];
        let summary =
            summarize_strict_nest_certificates(&cand, &objects_by_level, &terminal_by_key);

        // 旧实现：top=1 与 top=2 各查一次同一基例 ⟹ 膨胀为 2；唯一基例口径应为 1。
        assert_eq!(summary.terminal_missing, 1);
        assert_eq!(summary.base_count, 1);
        assert_eq!(summary.cert_total, 0);
    }

    /// 无结构数据 → 空订单流（**诚实结果，非阻塞态**）。recognize 已接通（2026-06-26）；
    /// 单调上涨数据无顶底分型交替 ⟹ 无笔 ⟹ 无中枢 ⟹ 无第三类买卖点 ⟹ 空 decisions ⟹
    /// 空订单。这验证管线 L1 串通 + "无缠论结构 ⇒ 无 Θ 决策"的正确退化（不是引擎缺陷）。
    /// bughunt F-06 回归：部分减仓（同向未到 0）逐 fill 产 TradeRecord（qty=本次减掉手数，
    /// entry_bar 延续首次入场），终平配对剩余手数——verify 反例 100买10→110减5→120平5
    /// 应产两条记录（qty=5/5），修复前仅终平一条（漏减仓敞口）。
    #[test]
    fn partial_reduce_emits_per_fill_trade_record() {
        let mut trades: Vec<metrics::TradeRecord> = Vec::new();
        let mut entry: Option<usize> = None;
        track_position_transition(&mut trades, &mut entry, 0.0, 10.0, 3, false); // bar3 开 10
        track_position_transition(&mut trades, &mut entry, 10.0, 5.0, 7, false); // bar7 减 5
        track_position_transition(&mut trades, &mut entry, 5.0, 0.0, 9, false); // bar9 平 5
        assert_eq!(trades.len(), 2, "减仓 fill + 终平各产一条 TradeRecord");
        assert_eq!((trades[0].entry_bar, trades[0].exit_bar), (3, 7));
        assert!((trades[0].qty - 5.0).abs() < 1e-12, "减仓记录 qty=减掉手数");
        assert!(trades[0].long && !trades[0].forced_close);
        assert_eq!((trades[1].entry_bar, trades[1].exit_bar), (3, 9));
        assert!((trades[1].qty - 5.0).abs() < 1e-12, "终平记录 qty=剩余手数");
        assert!(entry.is_none(), "归零后 entry_bar 清空");
    }

    #[test]
    #[allow(deprecated)] // F-01：既有管线退化测试，保留旧入口调用（A 列基线面不动）
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
            level_origin: 0,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 9_000_000_000, // px 90 < 入场 100 ⟹ 止损在下方不触及
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 9_500_000_000, zg: 10_500_000_000, dd: 9_000_000_000, gg: 11_000_000_000, start_index: 0, end_index: si })),
            struct_break_dir: None,
            force: None,
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
                    (classification.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
                } else {
                    (Classification::default(), Vec::new(), Vec::new(), i as u64, i as u64)
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

    /// ★#82 DC-E：默认关闭时，即使分类轨携原始 PanDivCert，生产订单轨逐字段冻结。
    #[test]
    fn pan_div_dc_e_default_inactive_order_track_bitexact() {
        use super::super::super::classifier::signal::PanDivCert;
        use super::super::super::types::Side;

        let config = ThetaConfig::default();
        assert!(!config.center_oscillation.enabled);
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let baseline = pi_theta_fill_loop(
            |i| (Classification::default(), Vec::new(), Vec::new(), i as u64, i as u64),
            &bars,
            1.0e6,
            &config,
            None,
        );
        let classification = Classification {
            levels: vec![LevelState {
                pan_div: Rc::new(vec![PanDivCert {
                    source_index: 6,
                    side: Side::Short,
                    center: Center {
                        zd: 9_500_000_000,
                        zg: 10_500_000_000,
                        dd: 9_000_000_000,
                        gg: 11_000_000_000,
                        start_index: 1,
                        end_index: 5,
                    },
                    seg_a: (1, 2),
                    seg_c: (5, 6),
                }]),
                ..Default::default()
            }],
        };
        let observed = pi_theta_fill_loop(
            |i| {
                if i >= 7 {
                    (classification.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
                } else {
                    (Classification::default(), Vec::new(), Vec::new(), i as u64, i as u64)
                }
            },
            &bars,
            1.0e6,
            &config,
            None,
        );

        assert_eq!(observed.n_orders, baseline.n_orders);
        assert_eq!(observed.equity_curve, baseline.equity_curve);
        assert_eq!(observed.daily_returns, baseline.daily_returns);
        assert_eq!(observed.trade_pnls_realized, baseline.trade_pnls_realized);
        assert_eq!(observed.trade_pnls_with_forced, baseline.trade_pnls_with_forced);
        assert_eq!(observed.trades, baseline.trades);
        assert_eq!(observed.typed_ledger, baseline.typed_ledger);
        assert_eq!(observed.tw_final, baseline.tw_final);
        assert_eq!(observed.r_decomp, baseline.r_decomp);
    }

    /// #282（#280 裁定）：#195 档A 生产路径见证（开启臂 OscillationBook↔SplitLegLedger
    /// 双写 + 权益分离）随 S6 开空腿账面形态删除——被测机已不存在，测试不遗留。
    /// 对照臂纪律由上方 `pan_div_dc_e_default_inactive_order_track_bitexact` 继续看守。

    /// ★#197 生产路径见证：并行记账视图（AccountIdentity 三身份 + AccountOrder）在 π fill
    /// loop 真实过账——余额按（账户, 级别, 仓位节点）分实例可读，并与既有账（G4 typed
    /// ledger）逐笔对账。expand 模式：视图只读旁路，不改净额路径（三把 bit-exact 锁为界）。
    ///
    /// 场景一（E 组夹具）：父 L1 Long 根（Core{1}）→ 子 L0 Short 短差（ReverseOpen）→
    /// 子腿一类反向平（reason=ReverseType1，账户=ReverseOpen——正交见证）→ 父仓窗口终点
    /// censored（reason=WindowEnd）。场景二（sell-first）：ambient 空根（Short=无父反向
    /// 声部）→ 一类反向平。
    #[test]
    fn account_view_witness_three_identities_in_pi_loop() {
        use super::super::super::strategy::account::{AccountIdentity, ActionReason};

        // ── 场景一：Core{1} + ReverseOpen 两身份（E 组夹具，同
        //    typed_ledger_reverse_open_close_overrides_trigger_class 的腿生命周期）。 ──
        let mut config = ThetaConfig::default();
        config.tick.tick_size = 1.0;
        let bars = e1_bars();
        let cls_parent = {
            let mut c = e_classification(false);
            c.levels[0] = LevelState::default(); // 仅父 L1 buy@12（子卖点未现）
            c
        };
        let cls_open = e_classification(false); // + 子 L0 sell@16
        let cls_all = e_classification(true); // + 子平触发 L0 buy1@18
        let tower = e_tower();
        let classify = move |i: usize| {
            let cls = if i >= 19 {
                cls_all.clone()
            } else if i >= 17 {
                cls_open.clone()
            } else if i >= 13 {
                cls_parent.clone()
            } else {
                Classification::default()
            };
            (cls, tower.clone(), tower.iter().map(|lv| lv.len()).collect::<Vec<usize>>(), i as u64, i as u64) // 合成静态塔：全塔跨 bar 位稳定 ⟹ confirmed_lens=全长（三方合并 schema 适配）
        };
        let fill = pi_theta_fill_loop(classify, &bars, 1.0e6, &config, None);
        let view = &fill.account_view;

        // 既有账锚点：父（δ=+1）/子（δ=−1）各一条 typed 交易（G4 账，独立路径产出）。
        let parent_row = fill
            .typed_ledger
            .iter()
            .find(|t| t.entry_z.delta == 1)
            .expect("父 L1 Long typed 交易存在");
        let child_row = fill
            .typed_ledger
            .iter()
            .find(|t| t.entry_z.delta == -1)
            .expect("子 L0 ReverseOpen typed 交易存在");
        assert_eq!(fill.typed_ledger.len(), 2, "父 + 子恰两条 typed 交易");

        // 对账 1（逐笔）：每条 typed 交易 ↔ 视图恰一分实例（position_node_id 严格身份），
        // 且实例已全平（qty=0）——「一类点/窗口终点后该实例=0」成为可强断言的账本事实。
        for row in &fill.typed_ledger {
            let matches: Vec<_> = view
                .instances()
                .values()
                .filter(|i| i.key.position == row.position_node_id)
                .collect();
            assert_eq!(
                matches.len(),
                1,
                "typed 交易 {:?} 在视图恰有一分实例（严格身份对账）",
                row.position_node_id
            );
            assert_eq!(matches[0].qty, 0.0, "实例已全平");
            assert!(!matches[0].open);
        }

        // 对账 2（身份归属跨账一致）：typed exit_type 分桶 ↔ 视图账户分桶。
        let close_fills: Vec<_> = view.fills().iter().filter(|f| f.order.reason != ActionReason::Open).collect();
        assert_eq!(
            close_fills.len(),
            fill.typed_ledger.len(),
            "每开一腿恰平一次 ⟹ 平仓成交数 = typed 交易数"
        );
        assert_eq!(
            close_fills.iter().filter(|f| f.order.account() == AccountIdentity::ReverseOpen { level: 0 }).count(),
            1,
            "子腿平仓归 ReverseOpen 账（exit_type=CloseReverseOpen 的账户侧）"
        );
        assert_eq!(
            close_fills.iter().filter(|f| f.order.account() == AccountIdentity::Core { level: 1 }).count(),
            1,
            "父腿平仓归 Core{{1}} 账（censored Hold 的账户侧）"
        );
        assert_eq!(
            close_fills.iter().filter(|f| f.order.account() == AccountIdentity::Short).count(),
            0,
            "本场景无 ambient 空根 ⟹ Short 账零平仓（与 typed 账无 ambient 空腿一致）"
        );

        // 正交见证：子腿平仓 账户=ReverseOpen、理由=ReverseType1（一类触发）——
        // ExitType::CloseReverseOpen 同时表达两者，AccountOrder 拆开。
        let child_close = close_fills
            .iter()
            .find(|f| f.order.account() == AccountIdentity::ReverseOpen { level: 0 })
            .expect("短差平仓成交存在");
        assert_eq!(child_close.order.reason, ActionReason::ReverseType1, "一类反向触发（正交理由轴）");
        // 父仓窗口终点 censored：理由=WindowEnd（账户=Core{1}）。
        let parent_close = close_fills
            .iter()
            .find(|f| f.order.account() == AccountIdentity::Core { level: 1 })
            .expect("父仓平仓成交存在");
        assert_eq!(parent_close.order.reason, ActionReason::WindowEnd);

        // 三身份余额读出（派生视图）+ 分实例时点对账：
        // 子腿在飞期间（开后、平前），Core{1} 余额 = 父仓手数、ReverseOpen 余额 = −子仓手数。
        let mid_bar = child_row.exit_bar - 1;
        assert!(child_row.exit_bar > child_row.entry_bar, "子腿开早于平");
        assert_eq!(
            view.balance_as_of(AccountIdentity::Core { level: 1 }, mid_bar),
            parent_row.units,
            "子在飞时父仓全额在册（G4 账 units 对账）"
        );
        assert_eq!(
            view.balance_as_of(AccountIdentity::ReverseOpen { level: 0 }, mid_bar),
            -child_row.units,
            "短差空向在册（父仓不动，毛暴露 ≠ 净额——分腿可见）"
        );
        assert_eq!(
            view.balance_as_of(AccountIdentity::ReverseOpen { level: 0 }, bars.len() - 1),
            0.0,
            "子腿一类点后 ReverseOpen 余额归零"
        );
        // 窗口终点后：全部实例全平 ⟹ 三身份余额皆 0（聚合仅派生，实例仍留档）。
        assert_eq!(view.balance(AccountIdentity::Core { level: 1 }), 0.0);
        assert_eq!(view.balance(AccountIdentity::ReverseOpen { level: 0 }), 0.0);
        assert_eq!(view.balance(AccountIdentity::Short), 0.0);
        // 聚合=派生：balance 现算 = 分实例 qty 手工求和（非 canonical 存储）。
        let manual_core: f64 = view
            .instances()
            .values()
            .filter(|i| i.key.account == AccountIdentity::Core { level: 1 })
            .map(|i| i.qty)
            .sum();
        assert_eq!(view.balance(AccountIdentity::Core { level: 1 }), manual_core);
        // 链纪律：窗口终点后 pending 全出队；每提交恰一成交。
        assert!(view.pending().is_empty(), "窗口终点后无悬挂 pending fill");
        assert_eq!(view.orders().len(), view.fills().len(), "每订单恰一成交（链闭合）");
        // 短差盈亏方向性见证（价格独立证据，非 sizing 同源）：子空 172→140 区间 ⟹ 已实现 > 0。
        assert!(child_close.realized_pnl > 0.0, "子空高卖低买 ⟹ 已实现为正");

        // ── 场景二：Short 身份（sell-first：ambient 空根 → 一类反向平）。 ──
        let cls_sell = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![sell_at(3, 1)]), ..Default::default() }],
        };
        let cls_both = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![sell_at(3, 1), buy1_at(12)]), ..Default::default() }],
        };
        let classify2 = move |i: usize| {
            if i >= 14 {
                (cls_both.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
            } else if i >= 7 {
                (cls_sell.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
            } else {
                (Classification::default(), Vec::new(), Vec::new(), i as u64, i as u64)
            }
        };
        let config2 = ThetaConfig::default();
        let bars2: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill2 = pi_theta_fill_loop(classify2, &bars2, 1.0e6, &config2, None);
        let view2 = &fill2.account_view;
        // 既有账：恰一条空根 typed 交易（CloseRoot）。
        assert_eq!(fill2.typed_ledger.len(), 1, "恰一条 ambient 空根 typed 交易");
        let short_row = &fill2.typed_ledger[0];
        assert_eq!(short_row.entry_z.delta, -1, "空根 δ=−1");
        // Short 身份读出：在飞期间余额 = −手数；一类点后归零。
        let mid2 = short_row.exit_bar - 1;
        assert_eq!(
            view2.balance_as_of(AccountIdentity::Short, mid2),
            -short_row.units,
            "ambient 空根在飞 ⟹ Short 账空向在册（无父反向声部，CONTEXT.md 反向根）"
        );
        assert_eq!(
            view2.balance_as_of(AccountIdentity::Short, short_row.exit_bar),
            0.0,
            "一类点后 Short 余额归零"
        );
        assert_eq!(view2.balance(AccountIdentity::Short), 0.0);
        assert_eq!(view2.balance(AccountIdentity::ReverseOpen { level: 0 }), 0.0, "无父场景不得记短差（互斥完备：无父即反向根）");
        // 平仓理由 = ReverseType1（正交：账户=Short、理由=一类反向）。
        let short_close = view2
            .fills()
            .iter()
            .find(|f| f.order.account() == AccountIdentity::Short && f.order.reason != ActionReason::Open)
            .expect("Short 平仓成交存在");
        assert_eq!(short_close.order.reason, ActionReason::ReverseType1);
        // 对账：Short 实例与 typed 交易同一 position_node_id。
        let short_inst = view2
            .instances()
            .values()
            .find(|i| i.key.account == AccountIdentity::Short)
            .expect("Short 分实例存在");
        assert_eq!(short_inst.key.position, short_row.position_node_id, "跨账严格身份一致");
        // 级别键诚实：本场景空根在 L0 ⟹ Short 实例键 level=0；Core 账零实例。
        assert_eq!(short_inst.key.level, 0);
        assert!(
            view2.instances().values().all(|i| !matches!(i.key.account, AccountIdentity::Core { .. })),
            "sell-first 场景无本仓腿 ⟹ Core 账零实例"
        );
    }

    /// ★#199 生产路径见证（红→绿，spec WP-2 修复 b）：二类反向关核心腿 =
    /// 「仅残余才纠错」——镜像理由 `CoreResidualCorrection`（非 `ReverseType2`）；
    /// typed 层五枚举不动（二类仍归 `ExitType::CloseRoot`，账户/理由正交）。
    ///
    /// 场景 = `buy_then_sell(2)`：buy1@3 开 L0 Long 根（bar 7）→ sell2@12 二类关（bar 14）。
    /// 红（修复前）：理由轴把二类核心关闭记 `ReverseType2`——「无条件 CloseRoot」
    /// 在账户侧的 expression（缺「仅残余才纠错」谓词，#185 审计发现 b）。
    #[test]
    fn type2_core_close_mirrors_as_core_residual_correction() {
        use super::super::super::strategy::account::{AccountIdentity, ActionReason};
        use super::super::super::strategy::interp::ExitType;
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy_then_sell(2), &bars, 1.0e6, &config, None);
        // typed 层不动（编排者裁定 2026-07-23）：根腿二类反向仍归 CloseRoot——五枚举
        // 单源，「仅残余才纠错」只长在理由轴（ActionReason），不进 ExitType。
        // ★#200 翻动核对（OpenShort 通道新订单，#179 程序）：typed 1→2——sell2@12 先平
        // （Long CloseRoot @14）后开（空根 @14，窗口终点 censored Hold）；第二笔即 OpenShort
        // 通道产物（见证在 `type2_open_short_channel_no_parent_lands_short_account`）。
        assert_eq!(fill.typed_ledger.len(), 2, "#200 新基线：二类关 + OpenShort 空根 censored 恰两条 typed");
        assert_eq!(
            fill.typed_ledger[0].exit_type,
            ExitType::CloseRoot,
            "typed 五枚举不动：二类 typed 归因仍归 CloseRoot"
        );
        let view = &fill.account_view;
        // 理由轴新口径：二类关核心腿 = CoreResidualCorrection（残余实测非零——腿在飞即残余）。
        let core_close = view
            .fills()
            .iter()
            .find(|f| {
                f.order.account() == AccountIdentity::Core { level: 0 }
                    && f.order.reason != ActionReason::Open
            })
            .expect("Core{0} 平仓成交存在");
        assert_eq!(
            core_close.order.reason,
            ActionReason::CoreResidualCorrection,
            "二类反向关核心腿 ⟹ 残余纠错理由（仅残余才纠错，非 ReverseType2）"
        );
        // 断言③：ReverseType2 永不落 Core 账（二类合法卖出仅 ReverseOpen/Short 两身份）。
        assert!(
            view.fills().iter().all(|f| {
                !(matches!(f.order.account(), AccountIdentity::Core { .. })
                    && f.order.reason == ActionReason::ReverseType2)
            }),
            "断言③：无二类 Core 卖单（ReverseType2 不得落 Core 账）"
        );
        // 纠错后 Core{0} 余额归零（残余清净）。
        assert_eq!(view.balance(AccountIdentity::Core { level: 0 }), 0.0);
    }

    /// ★#199 场景 B（回归锁，spec WP-2 修复 b 后半句）：一类点全平后二类点到来——
    /// **二类不得生成本仓卖单**（无 `ReverseType2`/`CoreResidualCorrection` 落 Core 账）；
    /// 二类 = 第二入场/加空位（ID-3）：候选走规则3 开 ambient 空根（Short 账，#200 起
    /// 理由 = `OpenShort` 通道标注）。
    ///
    /// 诚实声明：本场景修复前后均绿——规则2 要求同级别在飞反向腿，一类已平 ⟹ 二类
    /// 候选无核心腿可关，本就不生 Core 卖单；本锁看守的是未来改动（如 #200 OpenShort
    /// 通道接线）不在此路径引入 Core 卖单。「残余为零 ⟹ CoreResidualCorrection 不触发」
    /// 的红→绿证据在分流单测（`reason_of_reverse_close(Core,2,false)==None`）——生产
    /// 路径该分支结构不可达（规则2 命中=腿在飞=残余非零），为防御性硬门。
    #[test]
    fn type2_after_type1_full_close_emits_no_core_sell() {
        use super::super::super::strategy::account::{AccountIdentity, ActionReason};
        use super::super::super::strategy::interp::ExitType;
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let cls_buy = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3)]), ..Default::default() }],
        };
        let cls_sell1 = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3), sell_at(10, 1)]), ..Default::default() }],
        };
        let cls_sell2 = Classification {
            levels: vec![LevelState {
                bsp: Rc::new(vec![buy1_at(3), sell_at(10, 1), sell_at(16, 2)]),
                ..Default::default()
            }],
        };
        let classify = move |i: usize| {
            if i >= 18 {
                (cls_sell2.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
            } else if i >= 12 {
                (cls_sell1.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
            } else if i >= 7 {
                (cls_buy.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
            } else {
                (Classification::default(), Vec::new(), Vec::new(), i as u64, i as u64)
            }
        };
        let fill = pi_theta_fill_loop(classify, &bars, 1.0e6, &config, None);
        // typed 锚点：Long 一类平（CloseRoot @12）+ 空根窗口终点（Hold @19）。
        assert_eq!(fill.typed_ledger.len(), 2, "Long 一平 + 空根 censored 恰两条 typed");
        assert_eq!(fill.typed_ledger[0].exit_type, ExitType::CloseRoot);
        assert_eq!(fill.typed_ledger[1].exit_type, ExitType::Hold);
        let view = &fill.account_view;
        // ★核心断言：一类全平后，二类（bar≥18）不生任何 Core 账卖单
        // （ReverseType2/CoreResidualCorrection 均不落 Core——本仓对二类封闭）。
        assert!(
            view.fills().iter().all(|f| {
                !(matches!(f.order.account(), AccountIdentity::Core { .. })
                    && matches!(
                        f.order.reason,
                        ActionReason::ReverseType2 | ActionReason::CoreResidualCorrection
                    ))
            }),
            "一类全平后二类不得生本仓卖单（无 Core 二类单）"
        );
        // 一类平后 Core{0} 余额归零且不再变动（含二类部署之后）。
        assert_eq!(view.balance(AccountIdentity::Core { level: 0 }), 0.0);
        // 二类 = 第二入场/加空位（ID-3）：Short 账出现开空单（ambient 空根开仓，合法）。
        // ★#200 翻动核对（理由轴收紧，非行为变化）：规则3 开空根的理由 Open →
        // **OpenShort**（二类 × Short 账户的通道标注，`reason_of_open` 单源）——本测试
        // 场景正是「无父 ⟹ 空仓账」的规则3 形态（先平后开的 close 侧缺席变体）。
        assert!(
            view.fills().iter().any(|f| {
                f.order.account() == AccountIdentity::Short && f.order.reason == ActionReason::OpenShort
            }),
            "二类候选规则3 开空根（Short 账 OpenShort，第二入场位/OpenShort 通道）"
        );
        // 一类 Core 平仓理由保持 ReverseType1（一类口径不受 #199 影响）。
        let t1_close = view
            .fills()
            .iter()
            .find(|f| {
                f.order.account() == AccountIdentity::Core { level: 0 }
                    && f.order.reason != ActionReason::Open
            })
            .expect("Core{0} 一类平仓成交存在");
        assert_eq!(t1_close.order.reason, ActionReason::ReverseType1);
    }

    /// ★#199 断言②（T1 实际成交）生产路径触发见证（合成单腿场景）：一类 Core 平仓
    /// 评估探针真实触发，且单腿场景级余额归零（违例探针=0——单腿即「一类=全平」成立的
    /// 平凡域）。**多核心腿场景经 #209 终态化**：fold 一类全平 + 断言②批次硬门（见
    /// `account_mirror_post` 断言②注释与 π loop 反向关闭循环结束点硬门段）+ BTC 见证
    /// `btc_type2_residual_correction_witness` 全窗违例=0。
    #[test]
    fn t1_core_close_zero_assertion_fires_in_pi_loop() {
        t1_core_zero_probe_reset();
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy_then_sell(1), &bars, 1.0e6, &config, None);
        assert_eq!(fill.typed_ledger.len(), 1, "场景锚点：一类平一腿");
        assert_eq!(
            t1_core_zero_probe_count(),
            1,
            "一类 Core 平仓一笔 ⟹ 断言②评估在生产路径真实触发（探针为凭）"
        );
        assert_eq!(
            t1_core_residual_probe_count(),
            0,
            "单腿场景：一类平仓后 balance(Core{{0}})==0（级残余为零的平凡域）"
        );
    }

    /// ★#237 塔夹具：L2 空父 B（外缘 Down ⟹ Short）由两条 L1 子段 compose——供 L1 顺父
    /// 级联 Short 候选的 638 真父附着（sell@8 命中 b0.ρ=8 ⟹ host=b0、父 B=Short）。
    /// L1/L0 vec 空：子段由 Compose 带出（two_parent_tower/e_tower 同款形态）。
    fn l2_short_parent_tower() -> Vec<Rc<Vec<LeveledMove>>> {
        let unit = |si: usize, ei: usize, dir: Direction, lo: i64, hi: i64, ord: u64| {
            LeveledMove::from_unit(
                &UnitRange { start_index: si, end_index: ei, direction: dir, lo, hi },
                ElementId { level: 0, ordinal: ord },
            )
        };
        // b0：L1 (0,8) Short（外缘 15→12 Down）；b1：L1 (8,16) Short（外缘 13→10 Down）。
        let b0 = LeveledMove::compose(
            &[unit(0, 4, Direction::Down, 8, 15, 0), unit(4, 8, Direction::Up, 5, 12, 1)],
            Center { zd: 6, zg: 13, dd: 4, gg: 16, start_index: 0, end_index: 8 },
            1,
            ElementId { level: 1, ordinal: 0 },
        );
        let b1 = LeveledMove::compose(
            &[unit(8, 12, Direction::Down, 6, 13, 2), unit(12, 16, Direction::Up, 4, 10, 3)],
            Center { zd: 5, zg: 12, dd: 3, gg: 14, start_index: 8, end_index: 16 },
            1,
            ElementId { level: 1, ordinal: 1 },
        );
        let big = LeveledMove::compose(
            &[b0, b1],
            Center { zd: 5, zg: 13, dd: 3, gg: 16, start_index: 0, end_index: 16 },
            2,
            ElementId { level: 2, ordinal: 0 },
        ); // 外缘 15→10 Down ⟹ Short
        vec![Rc::new(Vec::new()), Rc::new(Vec::new()), Rc::new(vec![big])]
    }

    /// ★#237 断言②**按方向拆查**生产路径见证（与断言①同源同口径，用户裁 2026-07-24）：
    /// **一类买批末 `balance_side(Core{{L}}, Short)==0`**（空侧查零过）——被批收集的
    /// 方向键 = 被关腿方向（级联 Short，与断言① `l.dir` 同源）；同 bar 规则3 新开的
    /// 多侧根镜像在批末检查**之后**（#200 先平后开次序）——「买批⇒多侧不查」由
    /// 方向键 + 次序双重保证。
    ///
    /// 场景（l2_short_parent_tower）：L2 sell1@4 开 L2 根 Short（Short 账）→ L1 sell1@8
    /// 顺父级联开 Core{{1}} 空侧（FollowParent×Short，638 附着 b0）→ L1 buy1@10 一类
    /// 买批全平空侧（ReverseType1，批次收集 (1, Short)）＋ 规则3 同 bar 开 L1 根 Long。
    ///
    /// **「卖批⇒空侧存活不查」的异侧共存形态 090 如实标注**：同级「多侧＋级联 Short」
    /// 共存只能经 restore/跨 bar 结构路径形成（任何同级反向候选开仓必先关对方——
    /// 规则2 方向锁定），runner 合成塔内不可伪造；m3 win9 bar=232810 (1,470) 正是该
    /// restore 形态的真实生产实例（#233 §5 另案面），其新口径零违例由 m3 全窗承担
    /// （本票验收 2）。本测试锁的是 π loop 内方向收集/拆查/`balance_side` 消费链
    /// 真实工作 + 平凡域零违例防回归。
    #[test]
    fn t1_core_close_zero_assertion_buy_batch_checks_short_side_only() {
        use super::super::super::strategy::account::AccountIdentity;
        use super::super::super::strategy::interp::ExitType;
        use super::super::super::strategy::voice::VoiceSide;
        t1_core_zero_probe_reset();
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let cls = |l1: Vec<BspPoint>, l2: Vec<BspPoint>| Classification {
            levels: vec![
                LevelState { bsp: Rc::new(vec![]), ..Default::default() },
                LevelState { bsp: Rc::new(l1), ..Default::default() },
                LevelState { bsp: Rc::new(l2), ..Default::default() },
            ],
        };
        let c1 = cls(vec![], vec![sell_at(4, 1)]);
        let c2 = cls(vec![sell_at(8, 1)], vec![sell_at(4, 1)]);
        let c3 = cls(vec![sell_at(8, 1), buy1_at(10)], vec![sell_at(4, 1)]);
        let tower = l2_short_parent_tower();
        let classify = move |i: usize| {
            let c = if i >= 9 {
                c3.clone()
            } else if i >= 7 {
                c2.clone()
            } else if i >= 5 {
                c1.clone()
            } else {
                Classification::default()
            };
            (c, tower.clone(), tower.iter().map(|lv| lv.len()).collect::<Vec<usize>>(), i as u64, i as u64) // 合成静态塔：全塔跨 bar 位稳定 ⟹ confirmed_lens=全长（三方合并 schema 适配）
        };
        let fill = pi_theta_fill_loop(classify, &bars, 1.0e6, &config, None);
        // 订单轨锚点：L2 根 Short（censored）+ 级联 Short（一类买批平）。一类候选
        // 「消费即止」（规则2 消费后不落规则3，#200 例外仅二类）⟹ 买批不同 bar 开新根。
        assert_eq!(fill.typed_ledger.len(), 2, "L2 根 Short + 级联 Short 恰两条 typed");
        let cascade = fill
            .typed_ledger
            .iter()
            .find(|t| t.exit_type == ExitType::CloseRoot)
            .expect("级联 Short 一类买批平 typed 存在");
        assert_eq!(cascade.entry_z.delta, -1, "被一类买批全平的 = 级联 Short（空侧）");
        assert_eq!(cascade.exit_bar, 9, "一类买批 bar 9 确认 ⟹ 规则2 关空侧腿");
        let view = &fill.account_view;
        // 新口径账面事实：买批平空后空侧分量=0（多侧本场景无腿——一类消费即止）。
        assert_eq!(
            view.balance_side(AccountIdentity::Core { level: 1 }, VoiceSide::Short),
            0.0,
            "一类买批 ⇒ 该级空侧分量=0（S2「该级该方向」良构形式）"
        );
        // L2 根 Short 落 Short 账（Ambient×Short 互斥完备：无父反向声部），不入 Core。
        assert!(
            view.fills().iter().any(|f| f.order.account() == AccountIdentity::Short),
            "L2 根 Short ⟹ Short 账开仓成交存在（#185 映射不动）"
        );
        assert_eq!(
            t1_core_zero_probe_count(),
            1,
            "一类 Core 平仓一批 ⟹ 断言②评估在生产路径真实触发（探针为凭）"
        );
        assert_eq!(
            t1_core_residual_probe_count(),
            0,
            "一类买批只查空侧分量（#237 拆查口径，违例=0）"
        );
    }

    /// ★#199 断言③后半（CoreResidualCorrection 残余硬门）生产路径触发见证：
    /// 每笔 CoreResidualCorrection 过账前核心残余实测非零（`balance(Core{level})≠0`）——
    /// 「仅残余才纠错」的执行层硬门。debug 构建逐笔核对；探针恒在计数。
    #[test]
    fn residual_correction_assertion_fires_in_pi_loop() {
        residual_correction_probe_reset();
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy_then_sell(2), &bars, 1.0e6, &config, None);
        // ★#200 翻动核对：typed 1→2（OpenShort 空根 censored 为第二笔，见
        // `type2_core_close_mirrors_as_core_residual_correction` 核对记录）；残余纠错笔数不变。
        assert_eq!(fill.typed_ledger.len(), 2, "#200 新基线：二类关核心腿一笔 + OpenShort 空根一笔");
        assert_eq!(
            residual_correction_probe_count(),
            1,
            "CoreResidualCorrection 一笔 ⟹ 残余硬门在生产路径真实触发（探针为凭）"
        );
        // 跑通无 `#199 断言③` panic ⟹ 该笔过账前 balance(Core{0})≠0（残余实测非零）。
    }

    /// ★#199 断言③前半（二类卖身份）生产路径见证 + 回归锁：短差腿**二类**平 ⟹
    /// 账户=ReverseOpen、理由保留 `ReverseType2`（合法二类卖——分流不伤合法身份）；
    /// typed 归 `CloseReverseOpen`（五枚举不动）。探针=1：每笔 ReverseType2 过账均经
    /// 「不落 Core 账」约束核对（断言③前半真实触发为凭）。
    ///
    /// 场景 = E 组夹具（父 L1 Long 根 + 子 L0 Short 短差），子平触发换 buy2@18。
    #[test]
    fn type2_reverse_open_close_keeps_reverse_type2_and_fires_guard() {
        use super::super::super::strategy::account::{AccountIdentity, ActionReason};
        use super::super::super::strategy::interp::ExitType;
        type2_sell_guard_probe_reset();
        let mut config = ThetaConfig::default();
        config.tick.tick_size = 1.0;
        let bars = e1_bars();
        let cls_parent = {
            let mut c = e_classification(false);
            c.levels[0] = LevelState::default(); // 仅父 L1 buy@12（子卖点未现）
            c
        };
        let cls_open = e_classification(false); // + 子 L0 sell@16
        let cls_all = e_classification_type2_child_close(); // + 子平触发 L0 buy2@18
        let tower = e_tower();
        let classify = move |i: usize| {
            let cls = if i >= 19 {
                cls_all.clone()
            } else if i >= 17 {
                cls_open.clone()
            } else if i >= 13 {
                cls_parent.clone()
            } else {
                Classification::default()
            };
            (cls, tower.clone(), tower.iter().map(|lv| lv.len()).collect::<Vec<usize>>(), i as u64, i as u64) // 合成静态塔：全塔跨 bar 位稳定 ⟹ confirmed_lens=全长（三方合并 schema 适配）
        };
        let fill = pi_theta_fill_loop(classify, &bars, 1.0e6, &config, None);
        // ★#200 翻动核对（OpenShort 通道新订单，#179 程序）：typed 2→3——buy2@18 先平
        // （子 ReverseOpen CloseReverseOpen）后开（顺父级联 Long @19，窗口终点 censored Hold，
        // 落 Core{0} 账 = 二类「加仓」侧；账户非 Short ⟹ 理由保留 Open，非 OpenShort）。
        assert_eq!(fill.typed_ledger.len(), 3, "#200 新基线：父 + 子 + 级联加仓腿恰三条 typed");
        let child_row = fill
            .typed_ledger
            .iter()
            .find(|t| t.entry_z.delta == -1)
            .expect("子 L0 ReverseOpen typed 交易存在");
        assert_eq!(child_row.exit_type, ExitType::CloseReverseOpen);
        // 新开级联腿（先平后开加仓侧）：δ=+1、censored；账户 = Core{0}、理由 = Open。
        let cascade_row = fill
            .typed_ledger
            .iter()
            .find(|t| t.exit_type == ExitType::Hold && t.entry_z.delta == 1 && t.entry_bar >= 19)
            .expect("buy2@18 先平后开的级联加仓腿 typed 存在（censored）");
        assert!(
            fill.account_view.fills().iter().any(|f| {
                f.order.account() == AccountIdentity::Core { level: 0 }
                    && f.order.reason == ActionReason::Open
                    && f.order.key.position == cascade_row.position_node_id
            }),
            "级联加仓腿落 Core{{0}} 账 × Open（二类加仓侧，非 OpenShort）"
        );
        // 子腿平仓：账户=ReverseOpen、理由=ReverseType2（二类触发的合法二类卖）。
        let child_close = fill
            .account_view
            .fills()
            .iter()
            .find(|f| {
                f.order.account() == AccountIdentity::ReverseOpen { level: 0 }
                    && f.order.reason != ActionReason::Open
            })
            .expect("短差平仓成交存在");
        assert_eq!(
            child_close.order.reason,
            ActionReason::ReverseType2,
            "短差腿二类平 ⟹ 合法二类卖身份保留（断言③前半：ReverseOpen ∈ 合法集）"
        );
        // 探针 = 1：该笔 ReverseType2 过账经「不落 Core 账」约束核对（生产路径真实触发）。
        assert_eq!(
            type2_sell_guard_probe_count(),
            1,
            "断言③前半在生产路径真实触发 1 笔（探针为凭）"
        );
        // 父仓不受子腿二类平影响（断言4 隔离：父 Core{1} 实例仅窗口终点动）。
        let parent_close = fill
            .account_view
            .fills()
            .iter()
            .find(|f| {
                f.order.account() == AccountIdentity::Core { level: 1 }
                    && f.order.reason != ActionReason::Open
            })
            .expect("父仓平仓成交存在");
        assert_eq!(parent_close.order.reason, ActionReason::WindowEnd);
    }

    /// ★#183 runner 级端到端复验（#148 验收1 在新路径上复验，code-review Spec 轴 (a)3 采纳）：
    /// **父声部一类卖（CloseRoot 命中）⟹ 同刻全部后代声部终结，无孤儿声部存活**——
    /// 生产 π loop（`pi_theta_fill_loop` → `coverage_step_from_buckets` 归一路径）端到端。
    ///
    /// 场景 = E 组塔夹具：父 L1 Long 根（buy1@12 开 @13）+ 子 L0 Short 短差（sell1@16 开 @17）
    /// 持仓中，L1 sell1@18（父级别一类卖，bar≥20 确认）⟹ interpret 规则2 关父（𝒟_x={父}）⟹
    /// **子树清仓**：短差子腿同刻（exit=20）清除（短差腿无豁免，ADR 0001 条目4）。
    ///
    /// 见证（2026-07-24 本票实跑）：typed 恰 2 条——父 CloseRoot（interpret close 桶，
    /// prune=false）+ 子 CloseReverseOpen（via_structural_prune=true，子树清仓经结构剪枝外化
    /// 路径落 typed，与旧 AncOK 被动剪同一路径 ⟹ typed 层新旧同构）；账户流 父 Core{1}×
    /// ReverseType1（一类口径）、子 ReverseOpen×StructuralPrune（exit_type==CloseReverseOpen ⟺
    /// account==ReverseOpen，#198 跨账一致）；终态 Core{1}=ReverseOpen=0（无孤儿）。
    #[test]
    fn t4_pi_loop_parent_type1_close_liquidates_reverse_open_child_subtree() {
        use super::super::super::strategy::account::{AccountIdentity, ActionReason};
        use super::super::super::strategy::interp::ExitType;
        let mut config = ThetaConfig::default();
        config.tick.tick_size = 1.0;
        let bars = e1_bars();
        // 时序：i≥13 仅父 L1 buy1@12；i≥17 + 子 L0 sell1@16；i≥20 + 父 L1 sell1@18（一类卖）。
        let cls_parent = {
            let mut c = e_classification(false);
            c.levels[0] = LevelState::default();
            c
        };
        let cls_open = e_classification(false);
        let cls_t1 = {
            let mut c = e_classification(false);
            let mut l1 = c.levels[1].bsp.as_ref().clone();
            l1.push(BspPoint {
                source_index: 18,
                level_origin: 0, // 三方合并 schema 适配（#110 级别身份）
                bits: BspBits { sell1: true, ..Default::default() },
                pivot_low: 0,
                pivot_high: 210,
                center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 150, dd: 85, gg: 160, start_index: 8, end_index: 18 })),
                struct_break_dir: None,
                force: None,
            });
            c.levels[1] = LevelState { bsp: Rc::new(l1), ..Default::default() };
            c
        };
        let tower = e_tower();
        let classify = move |i: usize| {
            let cls = if i >= 20 {
                cls_t1.clone()
            } else if i >= 17 {
                cls_open.clone()
            } else if i >= 13 {
                cls_parent.clone()
            } else {
                Classification::default()
            };
            (cls, tower.clone(), tower.iter().map(|lv| lv.len()).collect::<Vec<usize>>(), i as u64, i as u64) // 合成静态塔：全塔跨 bar 位稳定 ⟹ confirmed_lens=全长（三方合并 schema 适配）
        };
        let fill = pi_theta_fill_loop(classify, &bars, 1.0e6, &config, None);
        // 父终结 ⟹ 子树全清：typed 恰 2 条，同一 exit bar（同刻清仓）。
        assert_eq!(fill.typed_ledger.len(), 2, "父 CloseRoot + 子树清仓短差腿恰两条 typed");
        let parent_row = &fill.typed_ledger[0];
        assert_eq!(parent_row.entry_z.level, 1, "首条 = 父 L1 腿");
        assert_eq!(parent_row.exit_type, ExitType::CloseRoot, "父一类卖 ⟹ P5 CloseRoot");
        assert!(!parent_row.via_structural_prune, "父 = interpret close 桶（非剪枝外化）");
        let child_row = fill
            .typed_ledger
            .iter()
            .find(|t| t.entry_z.delta == -1)
            .expect("子 L0 ReverseOpen typed 交易存在");
        assert_eq!(child_row.exit_type, ExitType::CloseReverseOpen, "短差子腿关闭归 CloseReverseOpen");
        assert!(child_row.via_structural_prune, "子 = 子树清仓（结构剪枝外化路径落 typed）");
        assert_eq!(
            child_row.exit_bar, parent_row.exit_bar,
            "父终结⟹子树全清同一 bar（同刻清仓，无次刻孤儿窗口）"
        );
        // 账户流：父 Core{1}×ReverseType1（一类口径）；子 ReverseOpen×StructuralPrune（跨账一致 #198）。
        let view = &fill.account_view;
        assert!(
            view.fills().iter().any(|f| {
                f.order.account() == AccountIdentity::Core { level: 1 }
                    && f.order.reason == ActionReason::ReverseType1
            }),
            "父仓一类平 ⟹ Core{{1}}×ReverseType1"
        );
        assert!(
            view.fills().iter().any(|f| {
                f.order.account() == AccountIdentity::ReverseOpen { level: 0 }
                    && f.order.reason == ActionReason::StructuralPrune
            }),
            "子树清仓短差腿 ⟹ ReverseOpen×StructuralPrune（exit_type==CloseReverseOpen ⟺ account==ReverseOpen）"
        );
        // 无孤儿声部存活：终态父子余额皆零。
        assert_eq!(view.balance(AccountIdentity::Core { level: 1 }), 0.0, "父 Core{{1}} 余额归零");
        assert_eq!(view.balance(AccountIdentity::ReverseOpen { level: 0 }), 0.0, "子 ReverseOpen 余额归零（无孤儿）");
    }

    /// ★#200 生产路径见证（红→绿，spec WP-2 修复 c / issue #200 验收一）：二类开空通道——
    /// **无父（Ambient）二类反向候选「先平后开」**：残余核心纠错（#199 CoreResidualCorrection）
    /// ＋ 开空仓账（`Short` 账户，理由 `OpenShort`；票面 `OpenShort{level, certificate}` 的
    /// level/certificate 由 `AccountKey` 携带）。ambient 守卫读法：无 active 父 ⟹ 空仓账
    /// （CONTEXT.md:93 反向根：有父即短差、无父即反向根，互斥且完备）。
    ///
    /// 场景 = `buy_then_sell(2)`：buy1@3 开 L0 Long 根（bar 7）→ sell2@12 二类反向（bar 14）。
    /// 红（修复前）：候选在 close 分支被消费、同一候选不再进 open 分支（#185 审计发现 2：
    /// 二类开空 C 通道完全缺失）⟹ 无 Short 开仓成交、typed 仅 1 条。
    #[test]
    fn type2_open_short_channel_no_parent_lands_short_account() {
        use super::super::super::strategy::account::{AccountIdentity, ActionReason};
        use super::super::super::strategy::interp::ExitType;
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy_then_sell(2), &bars, 1.0e6, &config, None);
        // 订单轨新基线锚点（#200 新通道引入新订单，#179 程序核对）：Long 二类关 + 新开空根
        // 窗口终点 censored 恰两条 typed（修复前 = 1 条，空腿从未建仓）。
        assert_eq!(fill.typed_ledger.len(), 2, "Long 二类关 + 新开空根 censored 恰两条 typed");
        assert_eq!(
            fill.typed_ledger[0].exit_type,
            ExitType::CloseRoot,
            "typed 五枚举不动：二类反向关核心腿仍归 CloseRoot（#199 裁定）"
        );
        assert_eq!(fill.typed_ledger[1].exit_type, ExitType::Hold, "新开空根持有到窗口终点（censored）");
        assert_eq!(fill.typed_ledger[1].entry_z.delta, -1, "新腿 = 空向（OpenShort 通道产物）");
        let view = &fill.account_view;
        // ★通道核心见证：Short 账出现 OpenShort 开仓成交（无父 ⟹ 空仓账，理由=开空）。
        let short_open = view
            .fills()
            .iter()
            .find(|f| {
                f.order.account() == AccountIdentity::Short
                    && f.order.reason == ActionReason::OpenShort
            })
            .expect("OpenShort 通道成交存在（二类开空，spec WP-2 修复 c）");
        assert!(short_open.order.qty_delta < 0.0, "开空 = 空向数量（多正空负）");
        // 票面 OpenShort{level, certificate} 形状：level 与入场证书由 AccountKey 携带。
        assert_eq!(short_open.order.key.level, 0, "OpenShort level = L0（候选级别）");
        let cert = short_open
            .order
            .key
            .position
            .entry_certificate
            .expect("OpenShort 仓位节点携入场证书");
        assert_eq!(
            (cert.level, cert.source_index),
            (0, 12),
            "OpenShort certificate = sell2@12 二类候选证书"
        );
        // ambient 守卫互斥：无父 ⟹ 空仓账——短差账零成交（不存在同时双记，CONTEXT.md:93）。
        assert!(
            view.fills().iter().all(|f| !matches!(f.order.account(), AccountIdentity::ReverseOpen { .. })),
            "无父场景不得记短差账（互斥完备：无父即反向根）"
        );
        // spec 新增断言 3（二类合法卖出只剩短差、开空两身份）：本场景二类产物 =
        // Core×CoreResidualCorrection（残余纠错，#199）+ Short×OpenShort（开空，本票）；
        // 无 Core×ReverseType2（本仓对二类封闭）。
        assert!(
            view.fills().iter().all(|f| {
                !(matches!(f.order.account(), AccountIdentity::Core { .. })
                    && f.order.reason == ActionReason::ReverseType2)
            }),
            "断言③：无二类 Core 卖单（ReverseType2 不得落 Core 账）"
        );
        // 开空生命周期：在飞期间 Short 余额 = −手数；窗口终点 WindowEnd 平仓归零。
        let short_row = &fill.typed_ledger[1];
        assert!(short_row.exit_bar > short_row.entry_bar, "空根开早于平");
        assert_eq!(
            view.balance_as_of(AccountIdentity::Short, short_row.exit_bar - 1),
            -short_row.units,
            "空根在飞 ⟹ Short 账空向在册（派生视图重放）"
        );
        let short_close = view
            .fills()
            .iter()
            .find(|f| {
                f.order.account() == AccountIdentity::Short
                    && !matches!(f.order.reason, ActionReason::Open | ActionReason::OpenShort)
            })
            .expect("Short 窗口终点平仓成交存在");
        assert_eq!(short_close.order.reason, ActionReason::WindowEnd);
        assert_eq!(view.balance(AccountIdentity::Short), 0.0, "窗口终点后空仓账归零");
        // #199 口径不动：Core{0} 二类关 = CoreResidualCorrection（残余实测非零才触发），纠错后归零。
        let core_close = view
            .fills()
            .iter()
            .find(|f| {
                f.order.account() == AccountIdentity::Core { level: 0 }
                    && !matches!(f.order.reason, ActionReason::Open | ActionReason::OpenShort)
            })
            .expect("Core{0} 平仓成交存在");
        assert_eq!(core_close.order.reason, ActionReason::CoreResidualCorrection);
        assert_eq!(view.balance(AccountIdentity::Core { level: 0 }), 0.0);
    }

    /// ★#200 ambient 守卫读法（红→绿，issue #200 验收一括号项）：二类反向候选归属——
    /// **父声部 active ⟹ 短差账（ReverseOpen）**；与「无父 ⟹ 空仓账」（见
    /// [`type2_open_short_channel_no_parent_lands_short_account`]）互斥且完备
    /// （CONTEXT.md:93 反向根条款；action-taxonomy §5.2：有父⇒短差 P4、无父⇒开空 P6，
    /// first-match P4≻P6）。
    ///
    /// 场景 = E 组塔变体：父 L1 buy1@12 开 Core{{1}} Long 根（bar 13）→ L0 buy1@13 顺父
    /// 级联 Core{{0}} Long（bar 15）→ L0 sell2@16（ReverseOpen 角色，同 e_classification
    /// sell_child@16 的 host=sub(12,16)）二类反向（bar 17）：级联残余纠错
    /// （Core{{0}}×CoreResidualCorrection）＋ **先平后开**短差空腿（父 L1 active ⟹
    /// ReverseOpen 账，非 Short）。红（修复前）：候选被 close 分支消费，短差空腿从未建仓。
    #[test]
    fn type2_open_short_channel_active_parent_lands_reverse_open_account() {
        use super::super::super::strategy::account::{AccountIdentity, ActionReason};
        use super::super::super::strategy::interp::ExitType;
        let mut config = ThetaConfig::default();
        config.tick.tick_size = 1.0;
        let bars = e1_bars();
        // L1 父根买点（同 e_classification buy_parent@12）。
        let buy_parent = BspPoint {
            source_index: 12,
            level_origin: 0, // 三方合并 schema 适配（#110 级别身份）
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90,
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 150, dd: 85, gg: 160, start_index: 4, end_index: 12 })),
            struct_break_dir: None,
            force: None,
        };
        // L0 顺父级联买点（FollowParent Long 级联核心仓，Core{0}）。
        let buy_cascade = BspPoint {
            source_index: 13,
            level_origin: 0, // 三方合并 schema 适配（#110 级别身份）
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 120,
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 150, dd: 85, gg: 160, start_index: 8, end_index: 13 })),
            struct_break_dir: None,
            force: None,
        };
        // L0 二类卖点（ReverseOpen 角色——同 e_classification sell_child@16 的附着坐标，class 换 2）。
        let sell2_child = BspPoint {
            source_index: 16,
            level_origin: 0, // 三方合并 schema 适配（#110 级别身份）
            bits: BspBits { sell2: true, ..Default::default() },
            pivot_low: 0,
            pivot_high: 210,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 150, dd: 85, gg: 160, start_index: 8, end_index: 16 })),
            struct_break_dir: None,
            force: None,
        };
        let cls = |l0: Vec<BspPoint>| Classification {
            levels: vec![
                LevelState { bsp: Rc::new(l0), ..Default::default() },
                LevelState { bsp: Rc::new(vec![buy_parent]), ..Default::default() },
            ],
        };
        let cls_parent = cls(vec![]); // 仅父 L1 buy@12
        let cls_cascade = cls(vec![buy_cascade]); // + L0 buy@13 级联
        let cls_full = cls(vec![buy_cascade, sell2_child]); // + L0 sell2@16
        let tower = e_tower();
        let classify = move |i: usize| {
            let c = if i >= 17 {
                cls_full.clone()
            } else if i >= 14 {
                cls_cascade.clone()
            } else if i >= 13 {
                cls_parent.clone()
            } else {
                Classification::default()
            };
            (c, tower.clone(), tower.iter().map(|lv| lv.len()).collect::<Vec<usize>>(), i as u64, i as u64) // 合成静态塔：全塔跨 bar 位稳定 ⟹ confirmed_lens=全长（三方合并 schema 适配）
        };
        let fill = pi_theta_fill_loop(classify, &bars, 1.0e6, &config, None);
        // 订单轨新基线锚点：父根 + 级联（二类关）+ 新开短差空腿（censored）恰三条 typed
        // （修复前 = 2 条，短差空腿从未建仓）。
        assert_eq!(fill.typed_ledger.len(), 3, "父 + 级联 + 短差空腿恰三条 typed");
        let cascade_row = fill
            .typed_ledger
            .iter()
            .find(|t| t.exit_type == ExitType::CloseRoot)
            .expect("级联 Core{0} 二类关 typed 存在");
        assert_eq!(cascade_row.entry_z.delta, 1, "被关 = L0 Long 级联");
        let sd_row = fill
            .typed_ledger
            .iter()
            .find(|t| t.exit_type == ExitType::Hold && t.entry_z.delta == -1)
            .expect("新开短差空腿 typed 存在（censored）");
        let view = &fill.account_view;
        // ★ambient 守卫核心见证：父 active ⟹ 新开反向腿落**短差账**（ReverseOpen），
        // **不**落空仓账——Short 账零成交（互斥完备：有父即短差，不存在同时双记）。
        let sd_open = view
            .fills()
            .iter()
            .find(|f| f.order.account() == AccountIdentity::ReverseOpen { level: 0 } && f.order.reason == ActionReason::Open)
            .expect("短差空腿开仓成交存在（父 active ⟹ ReverseOpen 账）");
        assert!(sd_open.order.qty_delta < 0.0, "反父方向 = 空向短差（父多⟹短差做空）");
        assert_eq!(sd_open.order.key.level, 0, "短差腿 = L0 次级别声部");
        assert!(
            view.fills().iter().all(|f| f.order.account() != AccountIdentity::Short),
            "父 active 场景不得记空仓账（互斥完备：有父即短差）"
        );
        // 级联残余纠错（#199 口径不动）：Core{0} 二类关 = CoreResidualCorrection，纠错后归零。
        let cascade_close = view
            .fills()
            .iter()
            .find(|f| {
                f.order.account() == AccountIdentity::Core { level: 0 }
                    && !matches!(f.order.reason, ActionReason::Open | ActionReason::OpenShort)
            })
            .expect("级联 Core{0} 平仓成交存在");
        assert_eq!(cascade_close.order.reason, ActionReason::CoreResidualCorrection);
        assert_eq!(view.balance(AccountIdentity::Core { level: 0 }), 0.0);
        // 断言③：无二类 Core 卖单（ReverseType2 不得落 Core 账）。
        assert!(
            view.fills().iter().all(|f| {
                !(matches!(f.order.account(), AccountIdentity::Core { .. })
                    && f.order.reason == ActionReason::ReverseType2)
            }),
            "断言③：无二类 Core 卖单"
        );
        // 父仓 Core{1} 不动（短差隔离）：仅窗口终点 WindowEnd 动父仓。
        let parent_close = view
            .fills()
            .iter()
            .find(|f| {
                f.order.account() == AccountIdentity::Core { level: 1 }
                    && f.order.reason != ActionReason::Open
            })
            .expect("父仓平仓成交存在");
        assert_eq!(parent_close.order.reason, ActionReason::WindowEnd);
        // 短差生命周期：在飞期间 ReverseOpen 余额 = −手数；窗口终点归零。
        assert_eq!(
            view.balance_as_of(AccountIdentity::ReverseOpen { level: 0 }, sd_row.exit_bar - 1),
            -sd_row.units,
            "短差空腿在飞（父仓不动，毛暴露分腿可见）"
        );
        assert_eq!(view.balance(AccountIdentity::ReverseOpen { level: 0 }), 0.0, "窗口终点后短差账归零");
    }

    /// ★#200 风险强平展开（spec WP-2 修复 d / issue #200 验收二 / #185 断言5）：
    /// force_flat 多腿场景**展开为多条单账户 AccountOrder**（每条恰一账户——单值 enum
    /// 类型约束的生产形态）；**venue 侧只留最外层净额投影**（单一净额平仓订单），内部
    /// 三账户先原子过账；aggregation **可逆**（`balance_as_of` 事件重放对账：强平单恰
    /// 抵消在飞余额）。
    ///
    /// 场景 = E 组（父 Core{{1}} Long + 子 ReverseOpen Short 在飞）+ 分段保证金簿：
    /// bar<18 温和（MM=1%×净名义）⟹ 两腿正常开；bar≥18 punitive（MM=10⁹×净名义）⟹
    /// Liquidation ⟹ force_flat，两腿在飞被强平（bar 18）。
    ///
    /// 诚实声明：本测试是**见证/回归锁**——逐腿展开机制由 #197 视图层落地
    /// （runner 强平消费循环逐腿 `account_mirror_close`，「视图层天然形态」），
    /// 本票补生产路径断言与 venue 净额对账（无红阶段，机制非本票新建）。
    #[test]
    fn risk_exit_expands_to_single_account_orders() {
        use super::super::super::strategy::account::{AccountIdentity, ActionReason};
        use super::super::super::strategy::interp::ExitType;
        use super::super::super::strategy::risk::{
            MarginModel, MarginSchedule, MarginScheduleBook, RiskCushions,
        };
        let mut config = ThetaConfig::default();
        // 分段保证金簿：bar≥18 punitive（retail=10⁹ ⟹ 任意非零净名义 MM≫权益 ⟹ M1 强平）。
        let mild = MarginSchedule::cme_simple(0.01, 1.0).expect("温和段");
        let punitive = MarginSchedule::cme_simple(1.0, 1.0e9).expect("punitive 段");
        let book = MarginScheduleBook::new(vec![
            (i64::MIN, 18, mild),
            (18, i64::MAX, punitive),
        ])
        .expect("分段簿合法（from<to 升序不重叠）");
        config.margin = Some(MarginModel { book, cushions: RiskCushions::new(1.0, 2.0).unwrap() });
        // 费率清零 + 严格恒定价（tick 默认 1e-8 ⟹ px=100）：权益/基量零漂移 ⟹ sizing 精确
        // （无费拖/价漂诱发的手数取整微单），venue 净额订单逐笔可数（父开+子开+一笔净额强平 = 3）。
        config.exec.commission_bps = 0.0;
        config.exec.slippage_bps = 0.0;
        config.exec.tax_bps = 0.0;
        let bars: Vec<Bar> = (0..22).map(|i| mk_bar(i, 10_000_000_000, false)).collect();
        let cls_parent = {
            let mut c = e_classification(false);
            c.levels[0] = LevelState::default(); // 仅父 L1 buy@12（子卖点未现）
            c
        };
        let cls_open = e_classification(false); // + 子 L0 sell@16（无子平触发）
        let tower = e_tower();
        let classify = move |i: usize| {
            let cls = if i >= 17 {
                cls_open.clone()
            } else if i >= 13 {
                cls_parent.clone()
            } else {
                Classification::default()
            };
            (cls, tower.clone(), tower.iter().map(|lv| lv.len()).collect::<Vec<usize>>(), i as u64, i as u64) // 合成静态塔：全塔跨 bar 位稳定 ⟹ confirmed_lens=全长（三方合并 schema 适配）
        };
        let fill = pi_theta_fill_loop(classify, &bars, 1.0e6, &config, None);
        // 锚点：父 + 子恰两条 typed，双双 RiskExit @18（punitive 段起点强平，无窗口终点 censored）。
        assert_eq!(fill.typed_ledger.len(), 2, "父 + 子恰两条 typed（强平后无持仓）");
        assert!(
            fill.typed_ledger.iter().all(|t| t.exit_type == ExitType::RiskExit && t.exit_bar == 18),
            "两腿均于 bar 18 RiskExit（force_flat）"
        );
        let parent_row = fill
            .typed_ledger
            .iter()
            .find(|t| t.entry_z.delta == 1)
            .expect("父 Long typed 存在");
        let child_row = fill
            .typed_ledger
            .iter()
            .find(|t| t.entry_z.delta == -1)
            .expect("子 Short typed 存在");
        let view = &fill.account_view;
        // ★展开核心断言：RiskExit 理由成交**恰两笔**（两腿各一，非一个无身份净额动作）；
        // 每笔恰一账户（单值 enum——match 无通配臂即类型层证明，同 account.rs 断言5）。
        let risk_fills: Vec<_> = view
            .fills()
            .iter()
            .filter(|f| f.order.reason == ActionReason::RiskExit)
            .collect();
        assert_eq!(risk_fills.len(), 2, "强平展开为多条单账户单（两腿 ⟹ 两单，非一笔净额）");
        for f in &risk_fills {
            let _tag: &str = match f.order.account() {
                AccountIdentity::Core { .. } => "core",
                AccountIdentity::ReverseOpen { .. } => "reverse_open",
                AccountIdentity::Short => "short",
            };
        }
        // 每条单账户单落在**该腿自己的账户**：父 ⟹ Core{1}、子 ⟹ ReverseOpen；
        // 数量 = 反向等量（平多 −units / 平空 +units）。
        let parent_liq = risk_fills
            .iter()
            .find(|f| f.order.account() == AccountIdentity::Core { level: 1 })
            .expect("父腿强平单落 Core{1}（恰一账户）");
        assert_eq!(parent_liq.order.qty_delta, -parent_row.units, "平多 = 反向等量");
        let child_liq = risk_fills
            .iter()
            .find(|f| f.order.account() == AccountIdentity::ReverseOpen { level: 0 })
            .expect("子腿强平单落 ReverseOpen（恰一账户）");
        assert_eq!(child_liq.order.qty_delta, child_row.units, "平空 = 反向等量");
        // 原子过账 + 可逆 reconciliation：强平单恰好抵消在飞余额（事件重放对账）——
        // 强平前（bar 17）两账在飞；强平后（bar 18 起）两账归零；终态余额归零。
        assert_eq!(view.balance_as_of(AccountIdentity::Core { level: 1 }, 17), parent_row.units);
        assert_eq!(view.balance_as_of(AccountIdentity::ReverseOpen { level: 0 }, 17), -child_row.units);
        assert_eq!(view.balance_as_of(AccountIdentity::Core { level: 1 }, 18), 0.0);
        assert_eq!(view.balance_as_of(AccountIdentity::ReverseOpen { level: 0 }, 18), 0.0);
        assert_eq!(view.balance(AccountIdentity::Core { level: 1 }), 0.0);
        assert_eq!(view.balance(AccountIdentity::ReverseOpen { level: 0 }), 0.0);
        // venue 侧只允许最外层净额投影：全窗净额执行订单恰 3 笔（父开 + 子开 + **一笔**
        // 净额平仓）——强平在 venue 侧不展开（展开只发生在内部账户过账层）。
        assert_eq!(fill.n_orders, 3, "venue 净额投影：父开+子开+一笔净额强平（最外层才净额化）");
        // 净额对账：venue 平仓量 = 两腿在飞净额 = Σ 单账户强平量（内部展开与最外层投影一致）。
        let expanded_sum: f64 = risk_fills.iter().map(|f| f.order.qty_delta).sum();
        assert_eq!(
            expanded_sum,
            -(parent_row.units - child_row.units),
            "Σ 单账户强平量 = −(父多−子空) = venue 净额平仓量（aggregation 可逆）"
        );
    }

    /// ★#200 BTC 真实数据见证（生产路径，16000 bars，issue #200 验收三）：OpenShort 通道
    /// 订单轨在真实数据上的翻动核对 + 全窗身份约束 + 通道标注逐笔对账。
    ///
    /// 见证口径：
    /// - 订单轨翻动（#179 程序核对）：新通道引入新订单 ⟹ 同窗 typed 总数自 #198 基线
    ///   25 笔翻为 **26 笔**（五枚举 9+8+7+0+2；2026-07-24 本票实跑，同窗
    ///   `btc_type2_residual_correction_witness`/`typed_ledger_btc_smoke` 复核一致）；
    /// - 断言③全窗扫描：无 `{Core, ReverseType2}` 单（二类卖永不落本仓账）；
    /// - 二类触发开仓（`entry_z.i_class==2`，实测 11 笔）的通道标注逐笔对账：
    ///   `OpenShort ⟺ Short 账`（`reason_of_open` 映射真实数据逐笔成立，零误标）；
    /// - 开空单标注约束形态：`Short×OpenShort` 逐笔核对（qty<0、入场证书在键、level 一致）。
    ///
    /// 诚实声明：本窗 `Short×OpenShort` 笔数 = **0**（2026-07-24 实跑）——二类 × 无父空根
    /// 形态在本窗**样本缺席**（11 笔二类开仓全落 Core/ReverseOpen/Ambient-Long），非机制缺席：
    /// 「开空单真实产生（含账户/理由标注）」由合成驱动见证
    /// `type2_open_short_channel_no_parent_lands_short_account`（红→绿，生产 π loop）承担。
    /// 不伪造（DATA BLOCKER 纪律）。
    #[test]
    #[ignore = "#200 BTC 见证；需 BTC 数据（DATA BLOCKER 不伪造）"]
    fn btc_type2_open_short_channel_witness() {
        use super::super::data;
        use super::super::incremental::IncrementalClassifier;
        use super::super::prereg_windows::PREREG_WINDOWS;
        use crate::theta_v0::strategy::account::{AccountIdentity, ActionReason};
        let config = ThetaConfig::default();
        let w = PREREG_WINDOWS.iter().find(|w| w.symbol == "BTC").expect("BTC prereg 窗");
        let ds = data::load_by_symbol(w.symbol, &config).expect("BTC 数据");
        let oos = ds.slice_date_window(w.oos.0, w.oos.1);
        let cut = 32_000_usize.min(oos.bars.len());
        let train_bars = &oos.bars[0..cut / 2];
        let first_px = train_bars
            .iter()
            .find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * config.tick.tick_size)
            .unwrap_or(1.0);
        let nav = (first_px * 1000.0).max(1.0e6);
        let mut classifier_incr = IncrementalClassifier::new(train_bars, &config);
        let fill = pi_theta_fill_loop(
            |i| {
                let (cls, tower) = classifier_incr.classify_at(i);
                let gen = classifier_incr.tower_generation();
                let fe = classifier_incr.forest_epoch();
                let cl = classifier_incr.tower_confirmed_lens(tower.len());
                (cls, tower, cl, gen, fe)
            },
            train_bars,
            nav,
            &config,
            None,
        );
        let view = &fill.account_view;
        let open_shorts: Vec<_> = view
            .fills()
            .iter()
            .filter(|f| f.order.reason == ActionReason::OpenShort)
            .collect();
        let n_open_short = open_shorts.len();
        eprintln!("=== BTC #200 OpenShort 通道见证（{} 笔 typed）===", fill.typed_ledger.len());
        eprintln!("  开空单(Short×OpenShort)={n_open_short}");
        // 全量开/关理由×账户分布（翻动归因记录：新增 typed 的账户身份）。
        let mut open_dist: std::collections::BTreeMap<String, usize> = Default::default();
        for f in view.fills().iter().filter(|f| matches!(f.order.reason, ActionReason::Open | ActionReason::OpenShort)) {
            *open_dist.entry(format!("{:?}×{:?}", f.order.account(), f.order.reason)).or_default() += 1;
        }
        eprintln!("  开仓分布(账户×理由)={open_dist:?}");
        let mut et_hist: std::collections::BTreeMap<String, usize> = Default::default();
        for t in &fill.typed_ledger {
            *et_hist.entry(format!("{:?}", t.exit_type)).or_default() += 1;
        }
        eprintln!("  typed 五枚举分布={et_hist:?}");
        // 通道标注逐笔核对：账户=Short（恰一账户）、空向数量、入场证书在键且 level 一致。
        for f in &open_shorts {
            assert_eq!(f.order.account(), AccountIdentity::Short, "OpenShort ⟹ 空仓账（恰一账户）");
            assert!(f.order.qty_delta < 0.0, "开空 = 空向数量");
            let cert = f
                .order
                .key
                .position
                .entry_certificate
                .expect("OpenShort 仓位节点携入场证书（票面 {{level, certificate}} 形状）");
            assert_eq!(cert.level, f.order.key.level, "证书 level 与键 level 一致");
        }
        // 断言③全窗扫描：无 {Core, ReverseType2} 单（二类卖永不落本仓账）。
        assert!(
            view.fills().iter().all(|f| {
                !(matches!(f.order.account(), AccountIdentity::Core { .. })
                    && f.order.reason == ActionReason::ReverseType2)
            }),
            "断言③：BTC 全窗无二类 Core 卖单"
        );
        // 订单轨基线锁（#179 程序核对）：#198 基线 25 笔 → #200 新基线 26 笔 → ★#233 新基线
        // **50 笔**（五枚举实名分布 CloseRoot=32 / CloseReverseOpen=17 / ReduceCore=1 / Hold=0 /
        // RiskExit=0；2026-07-25 本票实跑，debug-assertions on/off 双口径逐位一致，同窗
        // `btc_type2_residual_correction_witness`/`typed_ledger_btc_smoke` 复核一致）。
        // ★#233 翻动逐条对账（方向翻转显式化为声部终结事件，#227 裁决蓝图两步形）：
        // - typed 26→50（+24）：翻向终结的旧世代腿经 silent_drops 入轨（via_structural_prune）
        //   ——非 ReverseOpen 归 CloseRoot（8→32，+24 含连清）、ReverseOpen 归 CloseReverseOpen
        //   （9→17，+8）。train 窗内「开仓后被树静默翻向续命」的持仓腿（大量 1-bar 持有，
        //   与 (2,98) gen 0..9 同形态）自此在翻向 bar 终结入轨，不再方向突变续命至后续信号。
        // - prune 腿 6→47（+41）：基线 6 笔 prune 腿全部同 entry/同 exit_type/同 account、
        //   exit 一致提前（(0,52) 6306→6175、(0,61) 8366→7011、(0,86) 10365→9941、
        //   (0,92) 10422→10368、(0,114) 14653→14286、(0,128) 15625→15356）——旧轨里方向
        //   被树翻转后续命至 AncOK/Stale 剪，新轨在翻向 bar 即终结；新增 41 笔 = 翻向父
        //   终结 + 子树连清（𝒟_x^† 现成机制，exit.rs subtree_close 数济判据）。
        // - ReduceCore 7→1（−6）：二类减仓对象核心腿在二类信号出现前已翻向终结，减仓触发面
        //   消失；Hold 2→0（−2）：窗尾 censored 持仓在窗尾前已翻向终结。
        // - 开空单 Short×OpenShort 0→2 + 开仓分布 Core{0}×Open 10→22 / ReverseOpen×Open 10→17 /
        //   Short×Open 3→6：翻向终结释放 carrier 后**新世代重登记**（蓝图两步形②）——旧轨
        //   候选被旧世代腿「持仓身份优先」（#216 规则①）让位，新轨旧世代已终结 ⟹ 候选
        //   准入开仓（A9 generation+1 新 posId/generation）。
        // - 断言①②③ 全程违例=0（同窗 `btc_type2_residual_correction_witness`：断言①评估=1
        //   级残余违例=0、断言②评估=0 级余额违例=0、断言③前半=1（同级 Core 残余=0））；
        //   prune 腿跨账一致性（exit_type⟺account）47 笔逐笔成立
        //   （`btc_prune_leg_exit_type_matches_account_identity`）。
        // ★#270（SPEC #268 T2）基线诚实重算（GOLDEN 先例）：#269 翻向守卫事件化（出生对立
        //   无事件不再触发）使本窗基线 **50 → 51 笔**（五枚举 CloseRoot=27 / CloseReverseOpen=14
        //   / ReduceCore=9 / Hold=1 / RiskExit=0，prune 腿 47→33；2026-07-26 本票实跑，
        //   debug/release 双口径逐位一致；基线对拍 = `git checkout ff7b52026e --
        //   strategy/{coverage,persistent}.rs` 复跑，50 笔/47 腿/旧分布与 #233 在案逐位一致）。
        // ★#270 翻动逐条对账（误杀消失、真杀照杀——修复预期的直接物证）：
        // - prune 腿 47→33（−14）：基线独有 31 笔绝多为 hold=1 的「出生对立无事件」t+1
        //   误杀（(0,101) 11679→11680、(0,119) 14366→14367、(0,5) 948→949 等 24 笔 1-bar，
        //   余为 (0,37) 4401→4424 类短持），事件口径下不再触发；新独有 17 笔 = 同 carrier
        //   腿存活到真实结构事件/子树连清的合法 prune（(0,128) 15355→15625 持 270 bar、
        //   (0,51) 5869→6306 持 437 bar、(1,6) 4424→6306、(1,20) 10422→11070）+ 新世代
        //   再入场（(1,2)/(1,26) 系）；两轨同在 16 笔逐位一致（同 id/entry/exit/型/账户）。
        // - CloseRoot 32→27（−5）/ CloseReverseOpen 17→14（−3）：1-bar 误杀入轨消失。
        // - ReduceCore 1→9（+8）：核心腿不再 t+1 被剪 ⟹ 存活至二类减仓触发面恢复（#233
        //   「减仓对象在信号前已翻向终结」面收窄）；Hold 0→1（+1）：窗尾 censored 恢复。
        // - 开仓分布 Core{0}×Open 22→20 / Core{1}×Open 3→7 / ReverseOpen×Open 17→14 /
        //   Short×Open 6→7 / Short×OpenShort 2→3：误杀释放 carrier 的时点迁移 + 世代重
        //   登记形态随事件口径迁移；typed 净 +1（存活腿自有信号出场入轨 > 误杀入轨消失）。
        // - 断言①②③ 全程违例=0 保持（`btc_type2_residual_correction_witness`：断言①
        //   评估=1 不变、断言②评估=0 不变、断言③前半 1→3、残余硬门 1→4——理由轴分桶随
        //   轨迹演化，#209 先例同款，零违例硬门不变）；prune 腿跨账一致性 33 笔逐笔成立
        //   （`btc_prune_leg_exit_type_matches_account_identity`）。
        assert_eq!(fill.typed_ledger.len(), 51, "#270 订单轨新基线（50→51，#269 事件化：误杀入轨消失 + 存活腿自有出场入轨）");
        // 二类触发开仓的通道标注约束（真实数据逐笔）：`reason_of_open`
        // 映射在真实数据上逐笔成立——OpenShort ⟺ Short 账户（零误标）。
        let mut n_t2_entry = 0usize;
        for t in fill.typed_ledger.iter().filter(|t| t.entry_z.i_class == 2) {
            n_t2_entry += 1;
            let open_fill = view
                .fills()
                .iter()
                .find(|f| f.order.key.position == t.position_node_id)
                .expect("二类入场腿的开仓成交存在");
            assert_eq!(
                open_fill.order.reason == ActionReason::OpenShort,
                open_fill.order.account() == AccountIdentity::Short,
                "二类触发开仓通道标注：OpenShort ⟺ Short 账（{:?} × {:?}）",
                open_fill.order.account(),
                open_fill.order.reason
            );
        }
        assert!(n_t2_entry > 0, "BTC 窗二类触发开仓实测非空（通道判据真实经受真实数据）");
        eprintln!("  二类触发开仓(i_class=2)={n_t2_entry}");
        // ★诚实记录（2026-07-25 #233 实跑）：本窗 `n_open_short`（Short×OpenShort）= **2**——
        // #200 基线记录为 0（二类 × 无父空根形态样本缺席，非机制缺席）；#233 翻向终结
        // 释放 carrier + 新世代重登记（蓝图两步形②）后该形态在真实数据显现 2 笔，
        // 上方逐笔标注断言（OpenShort ⟹ Short 账）对这两笔真实成立。
        // ★#270 诚实记录（2026-07-26 实跑）：n_open_short 2→**3**（误杀消失后世代重登记
        // 形态迁移，逐笔标注断言对三笔真实成立；n_t2_entry 随轨迹演化，下方打印在案）。
    }

    /// ★#237「死给你看」见证（票面条款，BTC 真实数据，生产 π 路径）：(1,470)
    /// （FollowParent×Short、parent=(2,107)，m3 win9 bar=232810 旧口径误报腿）的**终结
    /// 事件**在 m3 轨迹中逐 bar 核实——「不误报 ≠ 不清理」：拆查只是不再把合法存活
    /// 误报为违例；其合法出场路径（一类买点/父关连清/元素终结/翻向终结，#234 Q4.3）
    /// 必须在轨迹中真实发生。观测轨 = `voice_verdicts`（#201：每 bar 每持仓声部恰一枚
    /// 显式裁决，restore 腿同覆盖——该腿不经 open_trades 登记、typed_ledger 可能缺席，
    /// 裁决轨是其在飞/离场的完整可观测面）+ typed_ledger（若曾登记）。win10 (2,50)
    /// （FollowParent×Short、parent=(3,11)，#233 §5 同族）一并核对。若全窗扫尽目标腿
    /// 从未终结（新僵尸），测试失败 = 停手上浮，非掩盖（票面条款）。
    #[test]
    #[ignore = "#237 (1,470)/(2,50) 死给你看见证；需 BTC 数据（DATA BLOCKER 不伪造）"]
    fn m3_follow_parent_short_leg_termination_witness() {
        use super::super::data;
        use super::super::incremental::IncrementalClassifier;
        use super::super::prereg_windows::PREREG_WINDOWS;
        use super::super::super::strategy::interp::ExitType;
        let config = ThetaConfig::default();
        let w = PREREG_WINDOWS.iter().find(|w| w.symbol == "BTC").expect("BTC prereg 窗");
        let ds = data::load_by_symbol(w.symbol, &config).expect("BTC 数据");
        // 目标：(1,470)（本票票面腿）与 (2,50)（win10 同族）；自 win9 起逐窗追踪至终结。
        for target in [ElementId { level: 1, ordinal: 470 }, ElementId { level: 2, ordinal: 50 }] {
            let mut terminated = false;
            for win in w.wf_anchored.iter().filter(|win| win.i >= 9) {
                let test_ds = ds.slice_date_window(win.test_start, win.test_end);
                if test_ds.bars.is_empty() {
                    continue;
                }
                let bars = &test_ds.bars;
                let first_px = bars
                    .iter()
                    .find(|b| !b.untradable && b.close > 0)
                    .map(|b| b.close as f64 * config.tick.tick_size)
                    .unwrap_or(1.0);
                let nav = (first_px * 1000.0).max(1.0e6);
                let mut classifier_incr = IncrementalClassifier::new(bars, &config);
                let fill = pi_theta_fill_loop(
                    |i| {
                        let (cls, tower) = classifier_incr.classify_at(i);
                        let gen = classifier_incr.tower_generation();
                        let fe = classifier_incr.forest_epoch();
                        let cl = classifier_incr.tower_confirmed_lens(tower.len());
                        (cls, tower, cl, gen, fe)
                    },
                    bars,
                    nav,
                    &config,
                    None,
                );
                let verdicts: Vec<_> =
                    fill.voice_verdicts.iter().filter(|v| v.leg.id == target).collect();
                let ledger_hits: Vec<_> =
                    fill.typed_ledger.iter().filter(|t| t.voice_id == target).collect();
                if verdicts.is_empty() && ledger_hits.is_empty() {
                    eprintln!("[win{}] {:?} 本窗无轨迹", win.i, target);
                    continue;
                }
                if let (Some(first), Some(last)) = (verdicts.first(), verdicts.last()) {
                    eprintln!(
                        "[win{}] {:?} 裁决轨：首现 bar={} 末现 bar={} 末裁决={:?} 记录数={}（窗长 {}）",
                        win.i, target, first.bar, last.bar, last.exit_type, verdicts.len(), bars.len()
                    );
                    for v in verdicts.iter().filter(|v| v.exit_type != ExitType::Hold) {
                        eprintln!("  非Hold裁决：bar={} exit={:?} dir={:?} parent={:?}", v.bar, v.exit_type, v.leg.dir, v.leg.parent_id);
                    }
                }
                for t in &ledger_hits {
                    eprintln!(
                        "[win{}] {:?} typed：entry={} exit={} exit_type={:?} via_prune={} δ={}",
                        win.i, target, t.entry_bar, t.exit_bar, t.exit_type, t.via_structural_prune, t.entry_z.delta
                    );
                }
                // 终结判定（窗内三路径）：①信号关闭（裁决轨非 Hold——一类买点 CloseRoot 等）；
                // ②typed 轨非 censored 关条目（含 via_structural_prune 父关连清/翻向终结）；
                // ③裁决轨中断（末现 bar 远离窗尾——silent_drops 结构离场：父关连清/元素终结/
                // 翻向终结，§13 剪除腿不进裁决轨）。
                let n_bars = bars.len();
                let close_by_signal = verdicts.iter().any(|v| v.exit_type != ExitType::Hold);
                let close_by_ledger = ledger_hits
                    .iter()
                    .any(|t| t.exit_type != ExitType::Hold || t.via_structural_prune);
                let vanished = verdicts.last().is_some_and(|v| v.bar + 100 < n_bars);
                if close_by_signal || close_by_ledger || vanished {
                    eprintln!(
                        "[win{}] {:?} ★终结确认：信号关闭={} 结构/typed关闭={} 裁决轨消失={}",
                        win.i, target, close_by_signal, close_by_ledger, vanished
                    );
                    terminated = true;
                    break;
                }
                eprintln!("[win{}] {:?} 窗尾仍在飞（censored），续扫下一窗", win.i, target);
            }
            assert!(
                terminated,
                "{:?} 全窗扫尽未见终结事件——新僵尸，停手上浮（090 不掩盖，票面「死给你看」条款）",
                target
            );
        }
    }


    /// ★#198 跨账一致性见证（BTC 真实数据，生产路径）：§13 结构剪枝腿的 typed 归属
    /// 必须与 #197 账户身份一致——`exit_type == CloseReverseOpen` ⟺ `account == ReverseOpen`。
    ///
    /// 见证记录（修复前红，2026-07-23 本测试实跑）：train 16000 bars 共 25 笔 typed、6 笔
    /// 结构剪枝腿，其中 2 笔 `account=Core{0}`（FollowParent 级联核心仓）被旧判据
    /// （`!= Ambient`）错标 `CloseReverseOpen`——(0,61) entry=7010 exit=8366 与
    /// (0,114) entry=13549 exit=14692；修复后归 `CloseRoot`（core structural exit），
    /// 全 6 笔跨账一致（CloseRoot 6→8、CloseReverseOpen 10→8，五枚举总数 25 不变）。
    ///
    /// 相关断言按新口径逐条核对记录（#198 验收③；结论 = 零翻动、全部与新口径一致）：
    /// 1. `interp.rs` `reverse_exit_type` 冻结测试（1662-1666）：closed 桶判据
    ///    （ReverseOpen 腿任何触发类 ⟹ CloseReverseOpen），不涉 silent_drops 判据 ⟹ 不动。
    /// 2. `coverage.rs:5248-5283` ReverseOpen 一类派 P7（双链同构见证）：同 1，closed 桶。
    /// 3. `runner.rs` `typed_ledger_reverse_open_close_overrides_trigger_class`（5506+）：
    ///    ReverseOpen 腿一类反向 ⟹ CloseReverseOpen（closed 桶）⟹ 不动。
    /// 4. `runner.rs` `typed_ledger_reverse_close_root`：根腿一类 ⟹ CloseRoot（closed 桶）⟹ 不动。
    /// 5. `l3_delta_r_alpha.rs:2016-2021` 五枚举全分类守恒：总数守恒（修复后 8+7+8+0+2=25），
    ///    无硬编码分桶计数 ⟹ 不翻（本测试同窗实跑复核通过）。
    /// 6. `shadow.rs:358/370/522/537` 双链比对映射：消费 ExitType 枚举本身（未变）⟹ 不动。
    /// 7. `wverify_run.rs` exit_type_code/decode/label：枚举编码与标签不变；W-VERIFY 5 变体
    ///    拆解为纯描述性诊断（裁定甲：不进桶键/门控/裁决基），桶语义按新口径阅读。
    /// 8. `mu_estimator.rs:326-330` ExitType 诊断切片：#180 冻结（不进 MuClass 桶键）⟹
    ///    代码零改动；μ 分桶统计口径更新 = 桶的生产语义收紧（CloseReverseOpen 只含真短差腿），
    ///    已记录于 `silent_drop_exit_type` 文档。
    #[test]
    #[ignore = "G4 typed ledger BTC 见证；需 BTC 数据（DATA BLOCKER 不伪造）"]
    fn btc_prune_leg_exit_type_matches_account_identity() {
        use super::super::data;
        use super::super::incremental::IncrementalClassifier;
        use super::super::prereg_windows::PREREG_WINDOWS;
        use crate::theta_v0::strategy::account::AccountIdentity;
        use crate::theta_v0::strategy::interp::ExitType;
        let config = ThetaConfig::default();
        let w = PREREG_WINDOWS.iter().find(|w| w.symbol == "BTC").expect("BTC prereg 窗");
        let ds = data::load_by_symbol(w.symbol, &config).expect("BTC 数据");
        let oos = ds.slice_date_window(w.oos.0, w.oos.1);
        let cut = 32_000_usize.min(oos.bars.len());
        let train_bars = &oos.bars[0..cut / 2];
        let first_px = train_bars
            .iter()
            .find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * config.tick.tick_size)
            .unwrap_or(1.0);
        let nav = (first_px * 1000.0).max(1.0e6);
        let mut classifier_incr = IncrementalClassifier::new(train_bars, &config);
        let fill = pi_theta_fill_loop(
            |i| {
                let (cls, tower) = classifier_incr.classify_at(i);
                let gen = classifier_incr.tower_generation();
                let fe = classifier_incr.forest_epoch();
                let cl = classifier_incr.tower_confirmed_lens(tower.len());
                (cls, tower, cl, gen, fe)
            },
            train_bars,
            nav,
            &config,
            None,
        );
        eprintln!("=== BTC prune 腿跨账对照（{} 笔 typed）===", fill.typed_ledger.len());
        let mut n_prune = 0usize;
        for t in &fill.typed_ledger {
            if !t.via_structural_prune {
                continue;
            }
            n_prune += 1;
            let inst = fill
                .account_view
                .instances()
                .values()
                .find(|i| i.key.position == t.position_node_id);
            let account = inst.map(|i| i.key.account);
            eprintln!(
                "  id={:?} entry={} exit={} exit_type={:?} account={:?}",
                t.voice_id, t.entry_bar, t.exit_bar, t.exit_type, account
            );
            // ★跨账一致性（#198 新口径）：剪枝腿标 CloseReverseOpen ⟺ 账户身份为 ReverseOpen。
            // FollowParent 级联核心仓（account=Core{level}）归 CloseRoot（core structural exit）。
            assert_eq!(
                t.exit_type == ExitType::CloseReverseOpen,
                matches!(account, Some(AccountIdentity::ReverseOpen { .. })),
                "§13 剪枝腿 {:?} 跨账不一致：exit_type={:?} vs account={:?}（#185 修复 a）",
                t.voice_id, t.exit_type, account
            );
        }
        // ★#233：prune 腿基线 6 笔 → 47 笔（翻向终结 + 子树连清入轨，逐条对账见
        // `btc_type2_open_short_channel_witness` 基线锁注释）；47 笔逐笔跨账一致（上方断言）。
        // ★#270：47 笔 → 33 笔（#269 事件化使「出生对立无事件」t+1 误杀不再触发——基线独有
        // 31 笔绝多 1-bar、新独有 17 笔为存活到真实事件/连清的合法杀 + 新世代，两轨同在
        // 16 笔；逐腿对账见 `btc_type2_open_short_channel_witness` 基线锁注释）；33 笔逐笔
        // 跨账一致（上方断言，2026-07-26 实跑）。
        assert!(n_prune > 0, "BTC train 窗必产结构剪枝腿（见证非空转，#270 基线 33 笔）");
    }

    /// ★#199/#209 BTC 真实数据见证（生产路径，16000 bars）：二类反向「仅残余才纠错」
    /// 全窗实测 + 断言①②③评估/违例笔数（探针为凭）。
    ///
    /// 见证口径：
    /// - 一类关核心腿笔数 = fills 中 `ReverseType1 × Core{*}`；断言②评估探针对账；
    /// - **#209 终态（2026-07-24 本票实跑）**：断言①级残余违例=**0**、断言②级余额
    ///   违例=**0**（#199 测量态实测违例=1 作对照——fold 一类全平修复后全窗归零，
    ///   S7 级别内全平在真实数据坐实）；
    /// - 二类关核心腿笔数 = fills 中 `CoreResidualCorrection`（残余硬门逐笔构造恒真）；
    /// - 二类卖身份合法集：`ReverseType2` 仅落 ReverseOpen/Short（断言③前半逐笔构造恒真）；
    /// - 全窗扫描：无 `{Core, ReverseType2}` 单（断言③账户约束）。
    ///
    /// ★#209 翻动核对（#179 程序，2026-07-24 实跑）：typed 订单轨 **26 笔零翻动**
    /// （五枚举 9+8+7+0+2 与 #200 新基线逐桶一致——一类全平多关的腿全为未登记
    /// restore 祖先腿，不入 typed ledger）；理由轴分桶随轨迹演化：断言①评估 2→3
    /// （一类关 Core 腿总数，全平多关 1 条级联/祖先腿）、CoreResidualCorrection 4→6、
    /// ReverseType2=2 不变、断言③前半=2（同级 Core 残余=0）——逐条对账断言锁死。
    ///
    /// 诚实声明：`n_residual`（二类残余纠错笔数）是数据事实——若为零是样本缺席而非
    /// 机制缺席（合成场景 A 已见证触发）；不伪造（DATA BLOCKER 纪律）。
    #[test]
    #[ignore = "#199 BTC 见证；需 BTC 数据（DATA BLOCKER 不伪造）"]
    fn btc_type2_residual_correction_witness() {
        use super::super::data;
        use super::super::incremental::IncrementalClassifier;
        use super::super::prereg_windows::PREREG_WINDOWS;
        use crate::theta_v0::strategy::account::{AccountIdentity, ActionReason};
        t1_core_zero_probe_reset();
        type2_sell_guard_probe_reset();
        residual_correction_probe_reset();
        crate::theta_v0::strategy::coverage::t1_target_zero_probe_reset();
        let config = ThetaConfig::default();
        let w = PREREG_WINDOWS.iter().find(|w| w.symbol == "BTC").expect("BTC prereg 窗");
        let ds = data::load_by_symbol(w.symbol, &config).expect("BTC 数据");
        let oos = ds.slice_date_window(w.oos.0, w.oos.1);
        let cut = 32_000_usize.min(oos.bars.len());
        let train_bars = &oos.bars[0..cut / 2];
        let first_px = train_bars
            .iter()
            .find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * config.tick.tick_size)
            .unwrap_or(1.0);
        let nav = (first_px * 1000.0).max(1.0e6);
        let mut classifier_incr = IncrementalClassifier::new(train_bars, &config);
        let fill = pi_theta_fill_loop(
            |i| {
                let (cls, tower) = classifier_incr.classify_at(i);
                let gen = classifier_incr.tower_generation();
                let fe = classifier_incr.forest_epoch();
                let cl = classifier_incr.tower_confirmed_lens(tower.len());
                (cls, tower, cl, gen, fe)
            },
            train_bars,
            nav,
            &config,
            None,
        );
        let view = &fill.account_view;
        let n_t1_core = view
            .fills()
            .iter()
            .filter(|f| {
                matches!(f.order.account(), AccountIdentity::Core { .. })
                    && f.order.reason == ActionReason::ReverseType1
            })
            .count();
        let n_residual = view
            .fills()
            .iter()
            .filter(|f| f.order.reason == ActionReason::CoreResidualCorrection)
            .count();
        let n_t2_legal = view
            .fills()
            .iter()
            .filter(|f| f.order.reason == ActionReason::ReverseType2)
            .count();
        eprintln!("=== BTC #199 二类残余纠错见证（{} 笔 typed）===", fill.typed_ledger.len());
        eprintln!(
            "  一类关核心腿(ReverseType1×Core)={n_t1_core}；二类残余纠错(CoreResidualCorrection)={n_residual}；二类卖(ReverseType2→ReverseOpen/Short)={n_t2_legal}"
        );
        eprintln!(
            "  探针：断言②评估={} 级余额违例={}；断言③前半={}（其中同级Core残余={}）；残余硬门={}",
            t1_core_zero_probe_count(),
            t1_core_residual_probe_count(),
            type2_sell_guard_probe_count(),
            type2_with_core_residual_probe_count(),
            residual_correction_probe_count()
        );
        eprintln!(
            "  断言①（coverage 目标态）：评估={} 级残余违例={}",
            crate::theta_v0::strategy::coverage::t1_target_zero_probe_count(),
            crate::theta_v0::strategy::coverage::t1_target_residual_probe_count()
        );
        // 探针计数 = fills 分类计数（生产路径真实触发笔数，逐笔对账）。
        assert_eq!(t1_core_zero_probe_count() as usize, n_t1_core, "断言②评估笔数对账");
        assert_eq!(residual_correction_probe_count() as usize, n_residual, "残余硬门笔数对账");
        assert_eq!(type2_sell_guard_probe_count() as usize, n_t2_legal, "断言③前半笔数对账");
        // ★#209 终态硬断言（票面验收①，2026-07-24 实跑）：断言①② 全窗违例=0——
        // 一类卖后级残余=0（#199 测量态实测违例=1 作对照：fold 只关首条致 L=1 残留
        // FollowParent 延续腿 277.9 单位；一类全平修复后归零）。debug 构建下硬门先行
        // （违例即 panic），本断言为探针口径的显式验收锁。
        assert_eq!(
            crate::theta_v0::strategy::coverage::t1_target_residual_probe_count(),
            0,
            "#209 断言①终态：一类卖后级残余全窗违例=0（#199 实测违例=1 对照）"
        );
        assert_eq!(
            t1_core_residual_probe_count(),
            0,
            "#209 断言②终态：一类批末级余额全窗违例=0"
        );
        // 断言③全窗扫描：无 {Core, ReverseType2} 单（二类卖永不落本仓账）；
        // CoreResidualCorrection 全落 Core 账（残余纠错语义恰一账户）。
        assert!(
            view.fills().iter().all(|f| {
                !(matches!(f.order.account(), AccountIdentity::Core { .. })
                    && f.order.reason == ActionReason::ReverseType2)
            }),
            "断言③：BTC 全窗无二类 Core 卖单"
        );
        assert!(
            view
                .fills()
                .iter()
                .filter(|f| f.order.reason == ActionReason::CoreResidualCorrection)
                .all(|f| matches!(f.order.account(), AccountIdentity::Core { .. })),
            "CoreResidualCorrection 全落 Core 账（恰一账户）"
        );
        // 非空转：断言① coverage 评估探针 >0（#209 实跑评估=3——一类关 Core 腿总数，
        // 含全平多关的级联/restore 祖先腿；#199 测量态时为 2）。
        // 诚实记录（2026-07-24 #209 实跑复核）：本窗 `n_t1_core`（镜像 ReverseType1×Core）= 0——
        // 一类关的 Core 腿全部来自**未登记 restore 祖先腿**（非本窗信号入场，
        // runner.rs「表中无登记 ⟹ 不入 ledger」），断言②评估探针=0 是数据事实而非机制
        // 缺席（断言②非空转由合成见证 `t1_core_close_zero_assertion_fires_in_pi_loop` 承担）。
        assert!(
            crate::theta_v0::strategy::coverage::t1_target_zero_probe_count() > 0,
            "BTC train 窗必产一类卖 Core（断言①评估非空转）"
        );
    }

    /// ★#198 断言4 生产路径见证（#185「短差隔离」落生产执行层）：ReverseOpen fill 逐笔
    /// 触发隔离断言（`fill.account == ReverseOpen ⟹ Δqty(Core{*})==0 且 Δqty(Short{*})==0`），
    /// 探针计数为凭。
    ///
    /// 场景 = `account_view_witness_three_identities_in_pi_loop` 场景一（E 组：父
    /// Core{1} + 子 ReverseOpen 一类平）——子腿开/平各一笔 ReverseOpen fill ⟹ 探针 = 2；
    /// 本测试跑通无 `#198 断言4` panic ⟹ 两笔均满足隔离（debug 构建逐笔核对，
    /// 断言非空转——若 ReverseOpen 过账碰了父 Core{1} 分实例，断言以 #198 panic 炸掉）。
    #[test]
    fn reverse_open_isolation_assertion_fires_in_pi_loop() {
        reverse_open_isolation_probe_reset();
        let mut config = ThetaConfig::default();
        config.tick.tick_size = 1.0;
        let bars = e1_bars();
        let cls_parent = {
            let mut c = e_classification(false);
            c.levels[0] = LevelState::default(); // 仅父 L1 buy@12（子卖点未现）
            c
        };
        let cls_open = e_classification(false); // + 子 L0 sell@16
        let cls_all = e_classification(true); // + 子平触发 L0 buy1@18
        let tower = e_tower();
        let classify = move |i: usize| {
            let cls = if i >= 19 {
                cls_all.clone()
            } else if i >= 17 {
                cls_open.clone()
            } else if i >= 13 {
                cls_parent.clone()
            } else {
                Classification::default()
            };
            (cls, tower.clone(), tower.iter().map(|lv| lv.len()).collect::<Vec<usize>>(), i as u64, i as u64) // 合成静态塔：全塔跨 bar 位稳定 ⟹ confirmed_lens=全长（三方合并 schema 适配）
        };
        let fill = pi_theta_fill_loop(classify, &bars, 1.0e6, &config, None);
        // 既有账锚点：父 + 子各一条 typed（子 = ReverseOpen 一类平），场景真实含短差生命周期。
        assert_eq!(fill.typed_ledger.len(), 2, "父 + 子恰两条 typed 交易");
        // ★生产路径真实触发：子 ReverseOpen 开（Open）+ 平（ReverseType1）各一笔 fill ⟹ 探针 = 2。
        assert_eq!(
            reverse_open_isolation_probe_count(),
            2,
            "子 ReverseOpen 开/平各一笔 fill ⟹ 断言4 在生产路径真实触发 2 笔（探针为凭）"
        );
        // 跑通无 `#198 断言4` panic ⟹ 两笔 fill 均未改 Core{*}/Short{*} 分实例（隔离成立）。
    }

    /// ★#198 判据回归（红→绿）：silent drop 的 typed 归属判据——**仅** `entry_v == ReverseOpen`
    /// 的短差腿记 `CloseReverseOpen`；FollowParent 级联核心仓与 Ambient 根归 `CloseRoot`
    /// （core structural exit）。
    ///
    /// 修复前红：旧判据 `!= Ambient` 把 FollowParent 错标 `CloseReverseOpen`（#185 发现 a：
    /// 核心腿退出污染短差桶与 μ 分桶）。账户身份与退出理由正交：账户由 #197
    /// `identity_of(entry_v × 方向)` 映射（FollowParent ⟹ `Core{level}`），剪枝理由由
    /// `via_structural_prune` / `ActionReason::StructuralPrune` 轴表达。
    #[test]
    fn silent_drop_exit_type_attributes_non_reverse_open_to_core_structural_exit() {
        use super::super::super::strategy::coverage::Vertical;
        use super::super::super::strategy::interp::ExitType;
        assert_eq!(
            silent_drop_exit_type(Vertical::ReverseOpen),
            ExitType::CloseReverseOpen,
            "短差腿结构剪枝 ⟹ CloseReverseOpen（唯一合法 CloseReverseOpen 身份）"
        );
        assert_eq!(
            silent_drop_exit_type(Vertical::FollowParent),
            ExitType::CloseRoot,
            "FollowParent 级联核心仓剪枝 ⟹ CloseRoot（core structural exit，非短差桶）"
        );
        assert_eq!(
            silent_drop_exit_type(Vertical::Ambient),
            ExitType::CloseRoot,
            "Ambient 根结构失效 ⟹ CloseRoot（旧判据本臂即对，防回归）"
        );
    }

    /// ★#198 身份见证（生产 π loop）：FollowParent 核心级联子腿开仓 ⟹ `account ==
    /// Core{level}`（**非** ReverseOpen）——#198 验收①「错标不再发生」的账户侧防回归锁
    /// （场景口径：FollowParent 核心级联场景，断言 account==Core{level} 而非 ReverseOpen）。
    ///
    /// 场景（E 组夹具变体）：父 L1 buy1@12（host 查 (1,12) 落空 ⟹ Ambient 根）→ 子 L0
    /// buy1@16（638 附着 s3(0,3)，顺父方向 Long ⟹ **FollowParent** 级联核心仓）→ 候选
    /// 消失后两腿在飞至窗口终点（censored）。
    ///
    /// 诚实声明（090）：合成 pi loop 下 silent drop 不可达——persistent overlay §10
    /// `LiveDetached` 不 prune 且 registry 无生产 invalidated 写点，腿仅经 close/risk/
    /// censored 三通道离场（探索记录：候选消失/塔切空/父被 close 三变体均不产 silent
    /// drop；塔切空另触发既有 next_active 唯一性 debug_assert，非合法输入）。剪枝归属
    /// 的红→绿回归由判据单测（上）+ BTC 真实数据跨账见证（`btc_prune_leg_exit_type_
    /// matches_account_identity`：6 笔剪枝腿 2 笔 FollowParent 错标）承担，本测试锁
    /// 「FollowParent 腿 ⟹ Core 账户」的身份映射防回归。
    #[test]
    fn followparent_child_in_pi_loop_belongs_to_core_account_not_reverse_open() {
        use super::super::super::strategy::account::AccountIdentity;
        // 父 L1 buy1@12（Ambient）；子 L0 buy1@16（顺父 Long ⟹ FollowParent）。
        let fp_buy_child = BspPoint {
            source_index: 16,
            level_origin: 0, // 三方合并 schema 适配（#110 级别身份）
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 120,
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 150, dd: 85, gg: 160, start_index: 8, end_index: 16 })),
            struct_break_dir: None,
            force: None,
        };
        let fp_buy_parent = BspPoint {
            source_index: 12,
            level_origin: 0, // 三方合并 schema 适配（#110 级别身份）
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90,
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 150, dd: 85, gg: 160, start_index: 4, end_index: 12 })),
            struct_break_dir: None,
            force: None,
        };
        let cls_parent_only = Classification {
            levels: vec![
                LevelState::default(),
                LevelState { bsp: Rc::new(vec![fp_buy_parent.clone()]), ..Default::default() },
            ],
        };
        let cls_both = Classification {
            levels: vec![
                LevelState { bsp: Rc::new(vec![fp_buy_child.clone()]), ..Default::default() },
                LevelState { bsp: Rc::new(vec![fp_buy_parent.clone()]), ..Default::default() },
            ],
        };
        let tower = e_tower();
        let classify = move |i: usize| {
            if i >= 19 {
                (Classification::default(), tower.clone(), tower.iter().map(|lv| lv.len()).collect::<Vec<usize>>(), i as u64, i as u64) // 合成静态塔：全塔跨 bar 位稳定 ⟹ confirmed_lens=全长（三方合并 schema 适配）
            } else if i >= 17 {
                (cls_both.clone(), tower.clone(), tower.iter().map(|lv| lv.len()).collect::<Vec<usize>>(), i as u64, i as u64) // 合成静态塔：全塔跨 bar 位稳定 ⟹ confirmed_lens=全长（三方合并 schema 适配）
            } else if i >= 13 {
                (cls_parent_only.clone(), tower.clone(), tower.iter().map(|lv| lv.len()).collect::<Vec<usize>>(), i as u64, i as u64) // 合成静态塔：全塔跨 bar 位稳定 ⟹ confirmed_lens=全长（三方合并 schema 适配）
            } else {
                (Classification::default(), tower.clone(), tower.iter().map(|lv| lv.len()).collect::<Vec<usize>>(), i as u64, i as u64) // 合成静态塔：全塔跨 bar 位稳定 ⟹ confirmed_lens=全长（三方合并 schema 适配）
            }
        };
        let mut config = ThetaConfig::default();
        config.tick.tick_size = 1.0;
        let bars = e1_bars();
        let fill = pi_theta_fill_loop(classify, &bars, 1.0e6, &config, None);

        // 父子各一条 typed 交易（窗口终点 censored）。
        assert_eq!(fill.typed_ledger.len(), 2, "父 + 子恰两条 typed 交易");
        let child_row = fill
            .typed_ledger
            .iter()
            .find(|t| t.voice_id == ElementId { level: 0, ordinal: 3 })
            .expect("子腿（carrier=s3(0,3)）typed 交易存在");
        // 子腿顺父方向（Long，δ=+1）：638 附着 s3（真父 A(1,0) 外缘 Long）⟹ FollowParent。
        assert_eq!(child_row.entry_z.delta, 1, "子腿顺父方向（级联核心仓）");
        // ★票面断言：account == Core{level}（子腿级别 0），**非** ReverseOpen。
        let child_inst = fill
            .account_view
            .instances()
            .values()
            .find(|i| i.key.position == child_row.position_node_id)
            .expect("子腿分实例存在");
        assert_eq!(
            child_inst.key.account,
            AccountIdentity::Core { level: 0 },
            "FollowParent 级联核心仓 ⟹ Core{{0}} 账（#197 identity_of 映射）"
        );
        assert!(
            !matches!(child_inst.key.account, AccountIdentity::ReverseOpen { .. }),
            "FollowParent 不得落首开反向账（#185 修复 a 的账户侧）"
        );
        // 父腿对照：Ambient 多根 ⟹ Core{1}。
        let parent_row = fill
            .typed_ledger
            .iter()
            .find(|t| t.voice_id.level == 1)
            .expect("父腿 typed 交易存在");
        let parent_inst = fill
            .account_view
            .instances()
            .values()
            .find(|i| i.key.position == parent_row.position_node_id)
            .expect("父腿分实例存在");
        assert_eq!(parent_inst.key.account, AccountIdentity::Core { level: 1 }, "Ambient 多根 ⟹ Core{{1}} 账");
        // 本场景无短差腿 ⟹ ReverseOpen 账零实例（隔离性反面印证）。
        assert!(
            fill.account_view
                .instances()
                .values()
                .all(|i| !matches!(i.key.account, AccountIdentity::ReverseOpen { .. })),
            "无短差腿场景 ⟹ ReverseOpen 账零实例"
        );
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★★M6：Funding/Borrow/LiquidationLoss 进 PnL + R 分解 + 账目守恒
    //  （TARGET_STRATEGY_MAXFULL.md M6 / 路线.pdf p16 第十一关）
    // ──────────────────────────────────────────────────────────────────────

    /// ★M6 bit-exact：cost_model=None ⟹ R 分解三项成本恒 0，且 equity/trades 与现状逐位相同。
    /// 守恒残差 ≈0（价格 PnL − 成交费 = 账本净变动，funding/borrow/liq 全 0）。
    #[test]
    fn m6_cost_model_none_bit_exact_and_conserves() {
        let config = ThetaConfig::default(); // cost_model=None
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
        let r = fill.r_decomp.expect("π 路径产 R 分解");
        // None ⟹ 三项持仓/强平成本恒 0。
        assert_eq!(r.funding, 0.0, "cost_model=None ⟹ funding=0");
        assert_eq!(r.borrow, 0.0, "cost_model=None ⟹ borrow=0");
        assert_eq!(r.liquidation_loss, 0.0, "cost_model=None ⟹ liq=0");
        // 守恒：net_r = price_pnl − fee（无三项），残差 ≈0（账本独立测得 = 累计器组装）。
        let tol = 1e-6_f64.max(1e-9 * (1.0e6 + r.price_pnl_gross.abs()));
        assert!(r.conservation_residual.abs() <= tol, "守恒残差 {} 超容差 {}", r.conservation_residual, tol);
        // 成交费非负（有开仓 ⟹ commission+slippage 实付）。
        assert!(r.commission_slippage >= 0.0, "成交费≥0");
    }

    /// ★M6 强平罚金端到端证人：punitive margin（高 MM）使持仓 bar 权益跌破维持保证金 ⟹ M1
    /// Liquidation ⟹ (a) force_flat 强平平仓（RiskExit 优先）+ (b) LiquidationLoss 罚金进 R 分解。
    /// 这闭合真实 BTC 窗未触发的 liq 边沿路径（真实窗 sizing 保守 ⟹ 权益≫MM ⟹ liq 从不触发）。
    #[test]
    fn m6_liquidation_penalty_triggered_on_reachable_m1() {
        use super::super::super::strategy::risk::{
            CostModel, MarginModel, MarginSchedule, MarginScheduleBook, RiskCushions,
        };
        let mut config = ThetaConfig::default();
        // punitive margin：CME-simple pct_maint 极高（MM ≈ 3× 名义）⟹ 任意持仓即 E<MM ⟹ M1 可达。
        // 全域单段快照覆盖合成 bar 时间戳 [0,n)。
        let sched = MarginSchedule::cme_simple(1.0, 3.0).expect("punitive CME");
        let book = MarginScheduleBook::new(vec![(i64::MIN, i64::MAX, sched)]).unwrap();
        let cushions = RiskCushions::new(1.0, 2.0).unwrap();
        config.margin = Some(MarginModel { book, cushions });
        // liq 罚金 1%（funding/borrow 设 0，隔离 liq 项）。
        config.cost_model = Some(CostModel::new(0.0, 480, 0.0, 0.01).unwrap());
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
        let r = fill.r_decomp.expect("π 路径产 R 分解");
        // M1 可达 + 持仓 ⟹ 强平罚金 >0（边沿触发至少一次）。
        assert!(r.liquidation_loss > 0.0, "punitive margin ⟹ M1 触发 ⟹ liq 罚金>0，实得 {}", r.liquidation_loss);
        // 守恒仍成立（罚金从 cash 扣 ⟹ 账本净变动同步）。
        let tol = 1e-6_f64.max(1e-9 * (1.0e6 + r.price_pnl_gross.abs()));
        assert!(r.conservation_residual.abs() <= tol, "含 liq 罚金守恒残差 {} 超容差 {}", r.conservation_residual, tol);
    }

    /// ★M6 机制真装：cost_model=Some ⟹ 持仓跨 funding 周期 ⟹ funding/borrow 非零进 R 分解，
    /// 且守恒残差仍 ≈0（三项从 cash 扣 ⟹ 账本净变动同步反映）。费率参数化（有效域 L1）。
    #[test]
    fn m6_cost_model_some_funding_borrow_accrue_and_conserve() {
        use super::super::super::strategy::risk::CostModel;
        let mut config = ThetaConfig::default();
        // funding 每 4 bar 收（窗内 bar 4/8/12/16 触发），borrow 每 bar 微收；liq 罚金率设 0（本测试
        // 无强平态，隔离 funding/borrow）。费率放大到可观测量级（L1 机制验证，非 L2 标定值）。
        config.cost_model = Some(CostModel::new(0.001, 4, 0.0001, 0.0).unwrap());
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
        let r = fill.r_decomp.expect("π 路径产 R 分解");
        // 有持仓跨多个 funding 周期 ⟹ funding 累计 >0（买点 bar7 确认 → bar8 成交后持仓到窗末）。
        assert!(r.funding > 0.0, "持仓跨 funding 周期 ⟹ funding>0，实得 {}", r.funding);
        // borrow：base_units=NAV/px 建仓 ⟹ 名义 ≈ NAV，杠杆超权益部分（含费拖累后）产生借贷；
        // 至少非负且有限（无杠杆时可为 0，此处名义≈权益边界——只断言有限非负，不强求 >0）。
        assert!(r.borrow >= 0.0 && r.borrow.is_finite(), "borrow≥0 有限，实得 {}", r.borrow);
        // 守恒：三项从 cash 扣 ⟹ 账本净变动 = price_pnl − fee − funding − borrow − liq（残差≈0）。
        let tol = 1e-6_f64.max(1e-9 * (1.0e6 + r.price_pnl_gross.abs()));
        assert!(
            r.conservation_residual.abs() <= tol,
            "M6 全成本守恒残差 {} 超容差 {}（net_r={} ledger_delta={}）",
            r.conservation_residual, tol, r.net_r, r.ledger_delta
        );
        // net_r 相对 gross 被成本拖累（净 ≤ 毛价格贡献 − 成交费）。
        assert!(r.net_r <= r.price_pnl_gross - r.commission_slippage + tol, "成本拖累 ⟹ net_r 被扣减");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★★A10 C5（裁定 (b) TW 桥 G1）+ 附则A（κ 注入优先序）
    // ──────────────────────────────────────────────────────────────────────

    /// ★T-N4（裁定 C5 验收线）端到端对账行：cost_model=Some ⟹ R 分解携带 **TW 桥对账行**
    /// （= ⌊funding+borrow+liq⌋ 累计量化，与循环内 cum_holding_cost shadow 同一口径）且 >0；
    /// 桥是对账行不改现金流 ⟹ 守恒残差仍 ≈0；TW 账本代数不动（零账本侵入，GAP3 A' 冻结）。
    #[test]
    fn a10_c5_tw_bridge_recon_line_with_costs() {
        use super::super::super::strategy::risk::CostModel;
        let mut config = ThetaConfig::default();
        config.cost_model = Some(CostModel::new(0.001, 4, 0.0001, 0.0).unwrap());
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
        let r = fill.r_decomp.expect("π 路径产 R 分解");
        // 对账行 == ⌊funding+borrow+liq⌋（取整界内；三项恒≥0 ⟹ 截断=⌊⌋）。
        let cost_sum = r.funding + r.borrow + r.liquidation_loss;
        assert_eq!(r.tw_holding_cost_bridge, cost_sum as i64, "TW 桥对账行 == ⌊funding+borrow+liq⌋");
        assert!(cost_sum > 0.0, "前置：持盾成本实计（funding={} borrow={}）", r.funding, r.borrow);
        assert!(r.tw_holding_cost_bridge > 0, "桥>0 ⟹ 未修正 η=tw() 高估量 >0（G1 见证）");
        // 量化界：|f64 真值 − shadow| < 1（桥接对账断言同一界）。
        assert!((cost_sum - r.tw_holding_cost_bridge as f64).abs() < 1.0, "取整界内");
        // 守恒：桥是对账行（不进现金流）⟹ R 域守恒不破。
        let tol = 1e-6_f64.max(1e-9 * (1.0e6 + r.price_pnl_gross.abs()));
        assert!(r.conservation_residual.abs() <= tol, "含桥守恒残差 {} 超容差 {}", r.conservation_residual, tol);
    }

    /// ★T-N4 回归锁（裁定 C5：κ=0 ∧ cost=None ⟹ 与现 enter_ready 判据同值 bit-exact）：
    /// cost_model=None ⟹ 桥恒 0（修正量 0 ⟹ η_bucket/enter_ready 左操作数与历史同值）。
    #[test]
    fn a10_c5_bridge_zero_when_cost_none() {
        let config = ThetaConfig::default(); // cost_model=None
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
        let r = fill.r_decomp.expect("π 路径产 R 分解");
        assert_eq!(r.tw_holding_cost_bridge, 0, "cost=None ⟹ 桥=0（修正量 0，bit-exact 回归锁）");
    }

    /// ★T-N6（附则A 验收线）：κ 优先序 **env>config>baseline** 写死（纯函数层锁定，不碰进程
    /// env——env 臂由 `m7_kappa_sensitivity_grid_real_btc`（#[ignore]）经 set_var 端到端覆盖）；
    /// `risk_policy=Some(try_new_ratio(1,2))` ⟹ `eta_star` 用 ⌈L^wc+Q/2⌉；None ⟹ baseline 逐字节一致。
    #[test]
    fn a10_kappa_priority_env_over_config_over_baseline() {
        use super::super::super::strategy::ledger::{RiskPolicy, TwState};
        let half = RiskPolicy::try_new_ratio(1, 2).unwrap();
        // baseline 臂：env=None ∧ config=None ⟹ κ=0。
        assert_eq!(kappa_priority_resolve(None, None), RiskPolicy::baseline(), "双臂空 ⟹ baseline κ=0");
        // config 臂：env=None ⟹ 取 config.risk_policy（构造闸值原样穿透）。
        assert_eq!(kappa_priority_resolve(None, Some(half)), half, "config.risk_policy 注入生效");
        // env 臂：env>config（诊断覆写优先，单源纪律防双源静默漂移）。
        assert_eq!(
            kappa_priority_resolve(Some((2, 1)), Some(half)),
            RiskPolicy::try_new_ratio(2, 1).unwrap(),
            "env>config（优先序写死）"
        );
        assert_eq!(kappa_priority_resolve(Some((1, 2)), None), half, "env 独立臂（config=None）");
        // T-N6 语义锚：κ=1/2 ⟹ η⋆=⌈L^wc+Q/2⌉——L^wc=70, Q=100 ⟹ ⌈70+50⌉=120。
        let s = TwState { notional_in: 100, withdrawn: 30, ..TwState::initial() };
        assert_eq!(kappa_priority_resolve(None, Some(half)).eta_star(&s), 120, "κ=1/2 ⟹ η⋆=⌈L^wc+Q/2⌉=120");
        // None ⟹ 与现路径逐字节一致：baseline η⋆=L^wc=70。
        assert_eq!(kappa_priority_resolve(None, None).eta_star(&s), 70, "None ⟹ baseline κ=0（bit-exact）");
    }

    /// ★T-N6（附则A）：非法 env 值 **panic 语义不变**（诊断 knob 快失败，不静默降级掩盖网格错配）。
    #[test]
    #[should_panic(expected = "非法 κ barrier grid 值")]
    fn a10_kappa_priority_invalid_env_panics() {
        // 负分子（κ<0 非法，构造闸拒 ⟹ panic）；config 在场也不兜底（env 臂优先且快失败）。
        let _ = kappa_priority_resolve(Some((-1, 1)), RiskPolicy::try_new_ratio(1, 2));
    }

    /// ★T-N6（附则A）：非正分母 panic（有理病态非法）。
    #[test]
    #[should_panic(expected = "非法 κ barrier grid 值")]
    fn a10_kappa_priority_zero_den_panics() {
        let _ = kappa_priority_resolve(Some((1, 0)), None);
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★★χ_t 阈值过滤对比（task #41 acc-chi-theta-filter 可证伪核心）：
    //  χ≡1 全覆盖 vs χ=1[μ>θ] 阈值过滤——证明过滤确实改变交易集（非 no-op）。
    // ──────────────────────────────────────────────────────────────────────

    /// 合成 classify 闭包工厂：买点 buy1@3 在 bar≥7 确认（同 `..produces_trades_nonempty`），空塔。
    fn buy1_at3_confirmed_at7() -> impl Fn(usize) -> (Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>, Vec<usize>, u64, u64) {
        let classification = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3)]), ..Default::default() }],
        };
        move |i| {
            if i >= 7 {
                (classification.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
            } else {
                (Classification::default(), Vec::new(), Vec::new(), i as u64, i as u64)
            }
        }
    }

    /// ★R5-1 bit-exact 不变量（基因 073a/274号）：OPSEM_DUMP_DIR 未设 ⟹ 生产路径逐字节不变。
    /// 验证链：(1) [`OpsemDump::from_env`] env-gating（未设/空 ⟹ None）；(2) set vs unset 跑
    /// [`pi_theta_fill_loop`] ⟹ typed_ledger/n_orders/trade_pnls_with_forced bit-exact（opsem 只活
    /// [`LedgerOpen`]，[`TypedTrade`] 不含 opsem 字段 ⟹ 结构性免疫）；(3) dump JSON 含 R5-a/b/c 三字段
    /// （lex_argmin_top3/divergence_input/nest_depth），nest_depth 非 null（R5-c 接入
    /// [`structural_nest_depth`]，旧 entry_z.nest_depth π 路径恒 None）。
    ///
    /// 并行安全：env 只用于 (1)(2) 的 None 断言（unset/"" 对并行读者同样是 None，无副作用）；
    /// (3) 经线程局部 [`OPSEM_DUMP_DIR_OVERRIDE`] 注入——进程级 env 会被并行测试的 fill loop
    /// 读到并 truncate 同一 trades.jsonl（2026-07-13 全量回归竞态实录），线程局部不可见。
    #[test]
    fn opsem_dump_env_gated_bit_exact() {
        use std::io::Read;
        std::env::remove_var("OPSEM_DUMP_DIR");
        assert!(OpsemDump::from_env().is_none(), "未设 ⟹ None");
        std::env::set_var("OPSEM_DUMP_DIR", "");
        assert!(OpsemDump::from_env().is_none(), "空串 ⟹ None（filter |s|!s.is_empty()）");
        std::env::remove_var("OPSEM_DUMP_DIR");

        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill_unset = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
        assert!(!fill_unset.typed_ledger.is_empty(), "前置：买点确认 ⟹ 有 typed 交易");

        // 固定目录会与并行/残留测试进程互相 truncate；每次测试使用唯一目录。
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("系统时间晚于 epoch")
            .as_nanos();
        let dump_dir = std::env::temp_dir().join(format!(
            "opsem_r5_bitexact_test_{}_{}",
            std::process::id(),
            nonce
        ));
        OPSEM_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = Some(dump_dir.clone()));
        let fill_set = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
        OPSEM_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = None);

        // R5-1 核心：set vs unset ⟹ 输出 bit-exact。
        assert_eq!(fill_unset.typed_ledger, fill_set.typed_ledger, "typed_ledger bit-exact");
        assert_eq!(fill_unset.n_orders, fill_set.n_orders, "n_orders bit-exact");
        assert_eq!(
            fill_unset.trade_pnls_with_forced, fill_set.trade_pnls_with_forced,
            "trade_pnls_with_forced bit-exact"
        );

        // e1/e2/e3：dump JSON 含三字段 + R5-c nest_depth 非 null（旧 entry_z.nest_depth π 路径恒 null）。
        let mut content = String::new();
        std::fs::File::open(dump_dir.join("trades.jsonl"))
            .expect("dump 启用 ⟹ trades.jsonl 生成")
            .read_to_string(&mut content)
            .unwrap();
        let first = content.lines().next().expect("trades.jsonl 非空");
        assert!(
            first.contains("\"lex_argmin_top3\":[{\"control\""),
            "e1: lex_argmin_top3 非空数组（首名=p_star 选址，含 control+J_Θ key）"
        );
        assert!(first.contains("\"divergence_input\""), "e2: divergence_input 字段存在");
        assert!(
            !first.contains("\"nest_depth\":null"),
            "e3: nest_depth 非 null（R5-c 接入 structural_nest_depth）"
        );
        let _ = std::fs::remove_dir_all(&dump_dir);
    }

    /// ★#196 阶段 A 生产路径见证：shadow 双链比对在 `run_theta_v0_pi` 生产路径**真实执行**——
    /// 同一 fill loop（buy1@3 确认 ⟹ 开 Long 持仓多 bar）内 channel 适配层每 bar 并行裁决，
    /// 分歧报告经线程局部 override 落盘（进程 env 会被并行测试覆写，opsem 竞态同款规避）。
    ///
    /// 见证链：
    /// 1. 落盘报告含汇总头且 voice_steps>0 ⟹ shadow 非空转（每 bar 真实跑 channel 裁决）；
    /// 2. 开仓 bar 空仓 slot 裁 Open ⟷ 生产 Opened、持仓 bar 裁 Hold ⟷ Held——单腿单候选
    ///    场景两链同构全 Match（coverage tests T2 跨级同构断言思路的全量化兑现）；
    /// 3. 订单轨冻结不由本测试断言——三把既有 bit-exact 锁 + pan_div DC-E 锁在 shadow
    ///    接线后全绿背书（本测试只证 shadow 真实执行并产出分歧记录）。
    #[test]
    fn shadow_dual_chain_witness_in_pi_loop() {
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("系统时间晚于 epoch")
            .as_nanos();
        let report_path = std::env::temp_dir().join(format!(
            "shadow_divergence_test_{}_{}.txt",
            std::process::id(),
            nonce
        ));
        SHADOW_DIVERGENCE_PATH_OVERRIDE.with(|c| *c.borrow_mut() = Some(report_path.clone()));
        let fill = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
        SHADOW_DIVERGENCE_PATH_OVERRIDE.with(|c| *c.borrow_mut() = None);

        assert!(!fill.typed_ledger.is_empty(), "前置：买点确认 ⟹ 有 typed 交易");
        let report = std::fs::read_to_string(&report_path).expect("shadow 分歧报告落盘");
        let _ = std::fs::remove_file(&report_path);
        assert!(report.contains("# shadow 双链比对分歧报告"), "报告含汇总头：\n{report}");
        let steps_line =
            report.lines().find(|l| l.starts_with("voice_steps=")).expect("汇总行存在");
        assert!(!steps_line.starts_with("voice_steps=0 "), "shadow 非空转：{steps_line}");
        assert!(
            report.contains("divergences=0"),
            "单腿单候选无反向场景两链同构（全 Match）：\n{report}"
        );
    }

    /// ★#201 阶段 B 生产路径见证：显式 Hold 落 typed 裁决记录——驱动 [`pi_theta_fill_loop`]
    /// （buy1@3 确认@7 ⟹ 开 Long 持仓多 bar），裁决账本轨每 bar 每持仓声部恰一枚显式裁决
    /// （spec 两道防线①：接进 run_theta_v0_pi + 生产路径测试见证）。
    ///
    /// 见证链：
    /// 1. 本场景无反向/强平信号 ⟹ 开仓后逐 bar 全为 `ExitType::Hold`（Hold 由隐式转显式，
    ///    本票核心）——注意即便 TW P3/P4 账本事件分支成立，持仓声部裁决仍是 Hold（该分支
    ///    每声部逐枚裁 Hold，见 coverage verdicts_p3 测试），故本断言对 TW 状态不敏感；
    /// 2. 裁决序列逐 bar 连续、（bar, 声部）不重复 ⟹ 「每 bar 每声部恰一枚」；
    /// 3. 裁决声部 = typed ledger 窗口终点 censored 的声部（同一腿）——censored 强平属
    ///    typed ledger 轨既有语义，不在裁决轨（裁决轨只载 bar 内解释器裁决域的声部裁决）。
    #[test]
    fn voice_verdict_ledger_witness_explicit_hold_in_pi_loop() {
        use super::super::super::strategy::interp::ExitType;
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
        assert!(!fill.typed_ledger.is_empty(), "前置：买点确认 ⟹ 有 typed 交易");
        assert!(!fill.voice_verdicts.is_empty(), "裁决账本轨非空（每 bar 每声部一枚）");
        let voice = fill.voice_verdicts[0].leg.id;
        assert!(
            fill.voice_verdicts.iter().all(|v| v.leg.id == voice),
            "单腿场景：全部裁决同一声部"
        );
        assert!(
            fill.voice_verdicts.iter().all(|v| v.exit_type == ExitType::Hold),
            "无反向/强平信号 ⟹ 逐 bar 显式 Hold（Hold 由隐式转显式）：{:?}",
            fill.voice_verdicts
        );
        // 每 bar 每声部恰一枚：(bar, 声部) 不重复且逐 bar 连续覆盖持仓期至窗口末 bar。
        let mut prev_bar = None;
        for v in &fill.voice_verdicts {
            if let Some(p) = prev_bar {
                assert_eq!(v.bar, p + 1, "裁决序列逐 bar 连续（每 bar 恰一枚）：{v:?}");
            }
            prev_bar = Some(v.bar);
        }
        assert_eq!(
            fill.voice_verdicts.last().map(|v| v.bar),
            Some(bars.len() - 1),
            "持仓声部裁决覆盖至窗口末 bar"
        );
        let censored = fill
            .typed_ledger
            .iter()
            .find(|t| t.exit_type == ExitType::Hold)
            .expect("窗口终点 censored Hold 在 typed ledger");
        assert_eq!(censored.voice_id, voice, "裁决声部 = censored 声部（同一腿）");
        assert!(
            fill.voice_verdicts.len() >= 2,
            "持仓多 bar ⟹ 显式 Hold 裁决记录逐 bar 复现：{} 枚",
            fill.voice_verdicts.len()
        );
    }

    /// ★#202 阶段 C 验收①见证（生产 π loop，spec 两道防线①）：P2/P3 域（本级证书平仓，
    /// channel 解释器承担）的 typed 裁决与 AccountIdentity 身份键兼容——反向关闭的 typed
    /// （组合层 channel 判据 [`strategy::channel::cert_close_trigger`] 单源）与账户平仓成交
    /// （#197 `identity_of` / #199 `reason_of_reverse_close` 单源）逐笔对账：
    /// - P2 域：ambient 空根一类反向 ⟹ `CloseRoot` × `Short` × `ReverseType1`；
    /// - P3 域：Core 根腿三类反向 ⟹ `ReduceCore` × `Core{{0}}` × `ReverseType3`。
    /// 且 P2/P3 域平仓笔落裁决账本轨（#201 冻结 verdicts schema：每声部一枚 typed）。
    #[test]
    fn p23_channel_close_typed_matches_account_order_witness() {
        use super::super::super::strategy::account::{AccountIdentity, ActionReason};
        use super::super::super::strategy::interp::ExitType;

        // ── P2 域：sell-first（ambient 空根 → 一类反向平）。 ──
        let cls_sell = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![sell_at(3, 1)]), ..Default::default() }],
        };
        let cls_both = Classification {
            levels: vec![LevelState {
                bsp: Rc::new(vec![sell_at(3, 1), buy1_at(12)]),
                ..Default::default()
            }],
        };
        let classify = move |i: usize| {
            if i >= 14 {
                (cls_both.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
            } else if i >= 7 {
                (cls_sell.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
            } else {
                (Classification::default(), Vec::new(), Vec::new(), i as u64, i as u64)
            }
        };
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(classify, &bars, 1.0e6, &config, None);
        assert_eq!(fill.typed_ledger.len(), 1, "恰一条 ambient 空根 typed 交易（P2 域）");
        let row = &fill.typed_ledger[0];
        assert_eq!(row.exit_type, ExitType::CloseRoot, "P2 域 typed = CloseRoot（channel 判据单源）");
        // typed × AccountOrder 逐笔对账（账户身份/理由/数量/严格身份四轴）。
        let close_fill = fill
            .account_view
            .fills()
            .iter()
            .find(|f| !matches!(f.order.reason, ActionReason::Open | ActionReason::OpenShort))
            .expect("P2 域平仓成交存在");
        assert_eq!(
            close_fill.order.account(),
            AccountIdentity::Short,
            "ambient 空根 ⟹ 空仓账（identity_of 单源，CONTEXT.md 反向根）"
        );
        assert_eq!(
            close_fill.order.reason,
            ActionReason::ReverseType1,
            "一类反向（reason_of_reverse_close 单源）"
        );
        assert_eq!(close_fill.order.key.position, row.position_node_id, "严格身份对账");
        assert_eq!(close_fill.order.qty_delta, row.units, "平空数量 = +typed units（符号开侧反向）");
        // 裁决账本轨：P2 域平仓笔落 #201 冻结 schema（每声部一枚 typed，bar 对齐）。
        let close_verdict = fill
            .voice_verdicts
            .iter()
            .find(|v| v.leg.id == row.voice_id && v.exit_type == ExitType::CloseRoot)
            .expect("P2 域平仓裁决落 verdicts 轨（schema 冻结）");
        assert_eq!(close_verdict.bar, row.exit_bar, "裁决 bar = 平仓 bar");

        // ── P3 域：buy-first（Core 根腿 → 三类反向平）。 ──
        let cls_buy = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3)]), ..Default::default() }],
        };
        let cls_rev = Classification {
            levels: vec![LevelState {
                bsp: Rc::new(vec![buy1_at(3), sell_at(12, 3)]),
                ..Default::default()
            }],
        };
        let classify3 = move |i: usize| {
            if i >= 14 {
                (cls_rev.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
            } else if i >= 7 {
                (cls_buy.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
            } else {
                (Classification::default(), Vec::new(), Vec::new(), i as u64, i as u64)
            }
        };
        let bars3: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill3 = pi_theta_fill_loop(classify3, &bars3, 1.0e6, &ThetaConfig::default(), None);
        assert_eq!(fill3.typed_ledger.len(), 1, "恰一条 Core 根腿 typed 交易（P3 域）");
        let row3 = &fill3.typed_ledger[0];
        assert_eq!(row3.exit_type, ExitType::ReduceCore, "P3 域 typed = ReduceCore（S2 二分单源）");
        let close3 = fill3
            .account_view
            .fills()
            .iter()
            .find(|f| !matches!(f.order.reason, ActionReason::Open | ActionReason::OpenShort))
            .expect("P3 域平仓成交存在");
        assert_eq!(
            close3.order.account(),
            AccountIdentity::Core { level: 0 },
            "ambient 多根 ⟹ 本仓账（identity_of 单源）"
        );
        assert_eq!(close3.order.reason, ActionReason::ReverseType3, "三类反向 = 减仓语义");
        assert_eq!(close3.order.key.position, row3.position_node_id, "严格身份对账");
        assert_eq!(close3.order.qty_delta, -row3.units, "平多数量 = −typed units");
        assert!(
            fill3
                .voice_verdicts
                .iter()
                .any(|v| v.leg.id == row3.voice_id && v.exit_type == ExitType::ReduceCore),
            "P3 域平仓裁决落 verdicts 轨（schema 冻结）"
        );
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
        // G2 σ_higher 第 9 维：合成闭包塔恒空 ⟹ 查询侧 sigma_higher_at(&[],..)=0 ⟹ Some(0)，手建 z 同口径。
        // G3 第 10-13 维口径（#138）：fill loop 查询键 origin_level=Some(c.level)（无链默认）、
        // risk_mode=Some(Normal)（equity>0 无 margin 注入 ⟹ 每 bar mode 恒 Normal 真值）；
        // cand_channel/nest_depth 两侧 None（π 路径无准入门）。手建 est 的 z 须同口径。
        // #149 第 14 维：平价无已实现利润/未退本金 ⟹ tw.stage 恒 CostReduction（stage 不推进）。
        // #175 第 15 维：η=tw()=nav0 恒等于 η⋆=L^wc=notional_in（κ=0、零已实现 PnL、未退本金）
        // ⟹ 恒 PositiveSafe（PDF η≥η⋆ 含等号）。
        let z_buy = MuClass {
            horizontal: Some(crate::theta_v0::strategy::coverage::Horizontal::First),
            sigma_higher: Some(0),
            origin_level: Some(0),
            risk_mode: Some(crate::theta_v0::strategy::risk::RiskMode::Normal),
            t_stage: Some(crate::theta_v0::strategy::ledger::TStage::CostReduction),
            eta_bucket: Some(crate::theta_v0::strategy::ledger::EtaBucket::PositiveSafe),
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
        // G2 σ_higher 第 9 维：合成闭包塔恒空 ⟹ 查询侧 sigma_higher_at(&[],..)=0 ⟹ Some(0)，手建 z 同口径。
        // G3 第 10-13 维口径（#138）：fill loop 查询键 origin_level=Some(c.level)（无链默认）、
        // risk_mode=Some(Normal)（equity>0 无 margin 注入 ⟹ 每 bar mode 恒 Normal 真值）；
        // cand_channel/nest_depth 两侧 None（π 路径无准入门）。手建 est 的 z 须同口径。
        // #149 第 14 维：平价无已实现利润/未退本金 ⟹ tw.stage 恒 CostReduction（stage 不推进）。
        // #175 第 15 维：η=tw()=nav0 恒等于 η⋆=L^wc=notional_in（κ=0、零已实现 PnL、未退本金）
        // ⟹ 恒 PositiveSafe（PDF η≥η⋆ 含等号）。
        let z_buy = MuClass {
            horizontal: Some(crate::theta_v0::strategy::coverage::Horizontal::First),
            sigma_higher: Some(0),
            origin_level: Some(0),
            risk_mode: Some(crate::theta_v0::strategy::risk::RiskMode::Normal),
            t_stage: Some(crate::theta_v0::strategy::ledger::TStage::CostReduction),
            eta_bucket: Some(crate::theta_v0::strategy::ledger::EtaBucket::PositiveSafe),
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

    // ──────────────────────────────────────────────────────────────────────
    //  ★★G4 typed ledger（#134）：生产 π fill loop 输出腿级 TypedTrade——
    //  PDF §9 typed exit 替换 τ^reverse（codex-q1-spec G4 终裁）。
    // ──────────────────────────────────────────────────────────────────────

    /// 卖点（class=1 一类 / 3 三类；pivot_high 远高于价 ⟹ 不触及 Short 止损——本测试只用它反向关 Long）。
    fn sell_at(si: usize, class: u8) -> BspPoint {
        let bits = match class {
            1 => BspBits { sell1: true, ..Default::default() },
            2 => BspBits { sell2: true, ..Default::default() },
            _ => BspBits { sell3: true, ..Default::default() },
        };
        BspPoint {
            source_index: si,
            level_origin: 0,
            bits,
            pivot_low: 0,
            pivot_high: 20_000_000_000, // px 200 ≫ 100 ⟹ 不触及
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 9_500_000_000, zg: 10_500_000_000, dd: 9_000_000_000, gg: 11_000_000_000, start_index: 0, end_index: si })),
            struct_break_dir: None,
            force: None,
        }
    }

    /// 两阶段合成闭包：bar≥7 出 buy1@3；bar≥14 追加 sell@12（class 可选）。
    fn buy_then_sell(sell_class: u8) -> impl Fn(usize) -> (Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>, Vec<usize>, u64, u64) {
        let cls_buy = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3)]), ..Default::default() }],
        };
        let cls_both = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3), sell_at(12, sell_class)]), ..Default::default() }],
        };
        move |i| {
            if i >= 14 {
                (cls_both.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
            } else if i >= 7 {
                (cls_buy.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
            } else {
                (Classification::default(), Vec::new(), Vec::new(), i as u64, i as u64)
            }
        }
    }

    /// ★G4：Long 腿被一类反向卖候选关闭 ⟹ TypedTrade{entry_bar=7, exit_bar=14, CloseRoot}。
    /// τ^typed vs τ^reverse 的可观测差异载体：出场由生产 interpret 规则2 决定，非"任意反向信号"。
    #[test]
    fn typed_ledger_reverse_close_root() {
        use super::super::super::strategy::interp::ExitType;
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy_then_sell(1), &bars, 1.0e6, &config, None);
        assert_eq!(fill.typed_ledger.len(), 1, "恰一条腿级 typed 交易");
        let t = &fill.typed_ledger[0];
        assert_eq!(t.entry_bar, 7, "买点 bar 7 确认部署 ⟹ 腿进 active");
        assert_eq!(t.exit_bar, 14, "一类卖 bar 14 确认 ⟹ interpret 规则2 关腿");
        assert_eq!(t.exit_type, ExitType::CloseRoot, "根腿 + 一类反向 ⟹ P5 CloseRoot");
        assert_eq!(t.entry_z.delta, 1, "Long 腿 δ=+1");
        assert!(t.entry_px > 0.0 && t.exit_px > t.entry_px, "微涨数据 ⟹ exit_px>entry_px");
        // G3（#138）：训练侧 entry_z 携账本态真值（equity>0 无 margin ⟹ Normal）+ π 路径
        // 无准入门（cand_channel/nest_depth None）+ origin_level=Some(level)（无链默认）。
        use super::super::super::strategy::risk::RiskMode;
        assert_eq!(t.entry_z.risk_mode, Some(RiskMode::Normal), "fill loop entry_z 携 bar 级账本态");
        assert_eq!(t.entry_z.cand_channel, None, "π 路径不经 Nest/Xzd 准入门");
        assert_eq!(t.entry_z.origin_level, Some(t.entry_z.level), "无链口径 ℓ=e");
        // #149 第 14 维：入场 bar 无已实现利润/未退本金 ⟹ 账本相位真值 = CostReduction。
        assert_eq!(
            t.entry_z.t_stage,
            Some(crate::theta_v0::strategy::ledger::TStage::CostReduction),
            "fill loop entry_z 携 bar 级 TW 相位真值（#149）"
        );
        // #175 第 15 维：入场 bar 零已实现 PnL/未退本金 ⟹ η=tw()=nav0=η⋆=L^wc ⟹ PositiveSafe。
        assert_eq!(
            t.entry_z.eta_bucket,
            Some(crate::theta_v0::strategy::ledger::EtaBucket::PositiveSafe),
            "fill loop entry_z 携 bar 级 γ_t 四桶真值（#175）"
        );
    }

    /// ★A7（Task #165，《完整的策略.pdf》§6 z「两次快照」+ §9 typed exit）：出场侧 `exit_z` 透传。
    ///
    /// 验收两条（生产路径断言 + 结构不变量）：
    /// 1. **每笔出场携 `exit_z`**（非 None——`MuClass` 非 Option，字段恒在；这里验它是**同一结构实体
    ///    的第二次快照**）：结构身份维（level/δ/i_class/parent_dir/horizontal/force_state/σ_higher/
    ///    门链维）与 `entry_z` **逐字段相等**（入场冻结不重采样，PDF §6）；账本态三元
    ///    `{t_stage, eta_bucket, risk_mode}` 取出场 bar 决策点真值。
    /// 2. **L0 同价窗**：账本相位恒 CostReduction（无已实现利润 ⟹ stage 不推进）⟹ 出场账本态三元
    ///    与入场同值（同一 L0 平价窗内 tw 不漂移）。这是 §6 z「两次快照」在 L0 有效域的可观测落点：
    ///    快照机制正确（entry/exit 各取当 bar 真值），值恒等是 L0 平价的**诚实结论**非机制缺陷。
    #[test]
    fn typed_trade_carries_exit_z_snapshot() {
        use super::super::super::strategy::ledger::{EtaBucket, TStage};
        use super::super::super::strategy::risk::RiskMode;
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy_then_sell(1), &bars, 1.0e6, &config, None);
        assert_eq!(fill.typed_ledger.len(), 1, "恰一条腿级 typed 交易");
        let t = &fill.typed_ledger[0];
        // ── 条1：exit_z 是 entry_z 的结构-同构第二次快照（结构维逐字段相等，账本态维刷新）。──
        assert_eq!(t.exit_z.level, t.entry_z.level, "level 入场冻结");
        assert_eq!(t.exit_z.delta, t.entry_z.delta, "δ 入场冻结");
        assert_eq!(t.exit_z.i_class, t.entry_z.i_class, "I_γ 入场冻结");
        assert_eq!(t.exit_z.parent_dir, t.entry_z.parent_dir, "σ_p 入场冻结");
        assert_eq!(t.exit_z.short_swing, t.entry_z.short_swing, "短差态入场冻结");
        assert_eq!(t.exit_z.position, t.entry_z.position, "仓位态入场冻结");
        assert_eq!(t.exit_z.horizontal, t.entry_z.horizontal, "H 入场冻结");
        assert_eq!(t.exit_z.force_state, t.entry_z.force_state, "力度态入场冻结");
        assert_eq!(t.exit_z.sigma_higher, t.entry_z.sigma_higher, "σ_higher 入场冻结（PDF §6 入场值）");
        assert_eq!(t.exit_z.cand_channel, t.entry_z.cand_channel, "门通道入场冻结");
        assert_eq!(t.exit_z.nest_depth, t.entry_z.nest_depth, "下沉深度入场冻结");
        assert_eq!(t.exit_z.origin_level, t.entry_z.origin_level, "起始级入场冻结");
        // 账本态三元维在出场处取真值（本 L0 同价窗恒 Normal/CostReduction/PositiveSafe，见条2）。
        assert_eq!(t.exit_z.risk_mode, Some(RiskMode::Normal), "出场 bar 账本态真值 risk_mode");
        assert_eq!(t.exit_z.t_stage, Some(TStage::CostReduction), "出场 bar TW 相位真值");
        assert_eq!(t.exit_z.eta_bucket, Some(EtaBucket::PositiveSafe), "出场 bar γ_t 四桶真值");
        // ── 条2：L0 同价 ⟹ 出场账本态三元 = 入场（tw 不漂移；快照机制正确，值恒等是 L0 诚实结论）。──
        assert_eq!(t.exit_z.risk_mode, t.entry_z.risk_mode, "L0 同价：账本相位不漂移（risk_mode）");
        assert_eq!(t.exit_z.t_stage, t.entry_z.t_stage, "L0 同价：账本相位不漂移（t_stage）");
        assert_eq!(t.exit_z.eta_bucket, t.entry_z.eta_bucket, "L0 同价：账本相位不漂移（eta_bucket）");
    }

    /// ★A7（Task #165）：censored Hold 出场（窗口终点未离场腿）也携 `exit_z`——末决策点 `last_ext`
    /// 账本态真值，非静默 None。买点开腿后无反向信号 ⟹ 兑现到末可交易 bar（ExitType::Hold）。
    #[test]
    fn typed_trade_censored_hold_carries_exit_z() {
        use super::super::super::strategy::interp::ExitType;
        use super::super::super::strategy::ledger::TStage;
        use super::super::super::strategy::risk::RiskMode;
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        // buy_then_sell 的 sell 在 bar14 出——用只出 buy 的闭包 ⟹ 腿无反向信号 ⟹ censored Hold。
        let cls_buy = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3)]), ..Default::default() }],
        };
        let fill = pi_theta_fill_loop(
            move |i| {
                if i >= 7 { (cls_buy.clone(), Vec::new(), Vec::new(), i as u64, i as u64) }
                else { (Classification::default(), Vec::new(), Vec::new(), i as u64, i as u64) }
            },
            &bars, 1.0e6, &config, None,
        );
        assert_eq!(fill.typed_ledger.len(), 1, "恰一条 censored 腿");
        let t = &fill.typed_ledger[0];
        assert_eq!(t.exit_type, ExitType::Hold, "无反向信号 ⟹ censored Hold");
        // censored 出场 z 结构维仍 = 入场（同一实体）。
        assert_eq!(t.exit_z.level, t.entry_z.level);
        assert_eq!(t.exit_z.i_class, t.entry_z.i_class);
        // 账本态三元维取末决策点 last_ext 真值（非 ZExt::NONE 静默 None——末可交易 bar 有决策点）。
        assert_eq!(t.exit_z.risk_mode, Some(RiskMode::Normal), "censored 出场携末决策点账本态（非静默 None）");
        assert_eq!(t.exit_z.t_stage, Some(TStage::CostReduction), "censored 出场 TW 相位真值");
    }

    /// ★A9（Task #166，级别容器.pdf p14/§13）：typed ledger 的 position_node_id 账本层真填充。
    ///
    /// 验收：TypedTrade 携完整四元组身份——carrier=voice_id（腿 id）、entry_certificate=Some（开仓
    /// Candidate 坐标，非 None 死值）、side=开仓方向、generation=0（首次入场 carrier）。
    /// 这证明 A9 身份不是无消费者死机制（前任把恒 0/None 塞进 ActiveLeg 的声明膨胀已修复为账本层真填充）。
    #[test]
    fn typed_trade_carries_position_node_id() {
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy_then_sell(1), &bars, 1.0e6, &config, None);
        assert_eq!(fill.typed_ledger.len(), 1);
        let t = &fill.typed_ledger[0];
        let pid = t.position_node_id;
        // carrier = voice_id（腿 id，§14 carrier-id 层与四元组 carrier 分量一致）。
        assert_eq!(pid.carrier, t.voice_id, "posId.carrier = voice_id（carrier-id 层一致）");
        // entry_certificate 真填充（非死 None）——buy1@3 ⟹ 证书 source_index=3、level=0。
        let cert = pid.entry_certificate.expect("入场证书非 None（真填充，非声明膨胀）");
        assert_eq!(cert.source_index, 3, "证书坐标 = 开仓 buy1 的 source_index=3");
        assert_eq!(cert.level, 0, "证书级别 = 开仓候选级别 0");
        // side = Long（buy 开多）；generation=0（该 carrier 首次入场）。
        assert_eq!(pid.side, VoiceSide::Long, "开多 ⟹ side=Long");
        assert_eq!(pid.generation, 0, "首次入场 carrier ⟹ generation=0");
        // hash64 确定性：同一笔的 posId 两次编码一致。
        assert_eq!(pid.hash64(), pid.hash64(), "posId hash 确定性");
    }

    /// ★A9 验收核心（Task #166，级别容器.pdf p14/§13）：**同一 carrier 两次 campaign 不混淆 +
    /// generation 单调**——时序再入场（codex a9-posnode 裁定 C「可达」）的端到端实证。
    ///
    /// `pi_loop_realized_profit_reaches_earning_shares` 同场景：buy1@3（bar7 开、bar14 反向关）与
    /// buy1@16（bar17 再开）**附着到同一 L0 carrier hostOf=ElementId(0,0)**（同一走势元素上先后两个
    /// 买卖点触发）。两笔 TypedTrade 的 `voice_id` **碰撞**（都是 (0,0)——carrier-id 层无法区分两次
    /// campaign，正是前任「单 carrier 单 instance」YAGNI 裁定的反例），但 `position_node_id` 因
    /// **generation 0→1 单调递增** + entry_certificate（source_index 3→16）双重区分 ⟹ posId 严格不碰撞。
    /// 这证明 generation 不是死值：时序再入场在生产 fill loop 真实发生并被四元组正确区分。
    #[test]
    fn same_carrier_reentry_distinguished_by_generation() {
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20)
            .map(|i| mk_bar(i, if i < 10 { 10_000_000_000 } else { 100_000_000_000 }, false))
            .collect();
        let cls_buy = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3)]), ..Default::default() }],
        };
        let cls_sell = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3), sell_at(12, 1)]), ..Default::default() }],
        };
        let cls_rebuy = Classification {
            levels: vec![LevelState {
                bsp: Rc::new(vec![buy1_at(3), sell_at(12, 1), buy1_at(16)]),
                ..Default::default()
            }],
        };
        let fill = pi_theta_fill_loop(
            move |i| {
                if i >= 17 {
                    (cls_rebuy.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
                } else if i >= 14 {
                    (cls_sell.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
                } else if i >= 7 {
                    (cls_buy.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
                } else {
                    (Classification::default(), Vec::new(), Vec::new(), i as u64, i as u64)
                }
            },
            &bars,
            1.0e6,
            &config,
            None,
        );
        assert_eq!(fill.typed_ledger.len(), 2, "buy1@3 关 + buy1@16 再开 ⟹ 恰 2 笔");
        let (t0, t1) = (&fill.typed_ledger[0], &fill.typed_ledger[1]);
        // carrier-id 层碰撞：两笔同 carrier（voice_id 无法区分两次 campaign）。
        assert_eq!(t0.voice_id, t1.voice_id, "两次 campaign 附着同一 carrier ⟹ voice_id 碰撞");
        assert_eq!(t0.position_node_id.carrier, t1.position_node_id.carrier, "posId.carrier 同");
        // generation 单调：首 campaign gen=0，再入场 gen=1（高水位 +1）。
        assert_eq!(t0.position_node_id.generation, 0, "首 campaign generation=0");
        assert_eq!(t1.position_node_id.generation, 1, "同 carrier 再入场 generation=1（单调）");
        // entry_certificate 区分入场来源（source_index 3 vs 16）。
        assert_eq!(t0.position_node_id.entry_certificate.unwrap().source_index, 3);
        assert_eq!(t1.position_node_id.entry_certificate.unwrap().source_index, 16);
        // ★不混淆铁律：carrier-id 碰撞（voice_id 相等）但 posId 四元组不碰撞（generation 区分）。
        assert_ne!(
            t0.position_node_id.hash64(),
            t1.position_node_id.hash64(),
            "同 carrier 两次 campaign posId 不碰撞（generation 区分 ⟹ 不混淆）"
        );
    }

    /// ★#124 裁定4 + codex GAP3 裁定 A' 清单⑥（重写自 `tw_ledger_producer_in_place_and_conserved`）：
    /// TW 账本生产真值源端到端物证——**TW 增量 = 已实现 PnL 量化和**。
    ///
    /// 旧断言「TW 经真实交易流守恒 = 1_000_000」在 A' 前成立的原因是**利润在账本里凭空消失**
    /// （正 PnL 平仓后 TW 不变——codex 裁定判为具名确定要重写的测试）。A' 后的正确不变量：
    /// `tw() = ⌊nav0⌋ + ⌊Σ 费后已实现 PnL⌋`（②'' shadow 对累计值量化再派差分 ⟹ 望远镜求和
    /// 使终态漂移**恰**等于累计 PnL 的截断量化，误差有界不累积）。
    ///
    /// 结算时点硬边界同时被见证：`trade_pnls_realized`（实际平仓 fill）是唯一资金源——
    /// 窗口终点 `forced_pnl`（假设强平，不改 units/cash）**不**入 TW（推导链第 11 条）。
    #[test]
    fn tw_ledger_producer_drift_equals_quantized_realized_pnl() {
        use super::super::super::strategy::ledger::TStage;
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy_then_sell(1), &bars, 1.0e6, &config, None);
        let tw = fill.tw_final.expect("π 路径 TW 生产者就位（tw_final=Some）");
        // ★清单⑥核心不变量：TW 漂移 = 已实现 PnL 量化和（f64 累加序与生产循环一致 ⟹ bit-exact）。
        let realized_sum: f64 = fill.trade_pnls_realized.iter().sum();
        assert_eq!(
            tw.tw(),
            1_000_000 + realized_sum as i64,
            "TW 增量 = ⌊Σ费后已实现PnL⌋（利润不再凭空消失；A' 清单⑥）"
        );
        // 本场景微涨平仓 ⟹ 正 PnL 真进账本（旧版此值恒 1_000_000 = 利润消失）。
        assert!(realized_sum > 0.0, "微涨买卖场景产正已实现 PnL");
        assert!(tw.tw() > 1_000_000, "正 PnL ⟹ TW 真增长（利润入账见证）");
        // forced_pnl 隔离见证：本场景腿在 bar 14 已信号平仓（无窗口终点持仓）⟹ with_forced 与
        // realized 同长；即便有 forced 尾巴，②'' 只消费 apply_order 返回值，forced 不经该路径。
        assert!(
            fill.trade_pnls_with_forced.len() >= fill.trade_pnls_realized.len(),
            "forced 口径仅报告用（不入 TW）"
        );
        // 小额 PnL ≪ notional_in=1e6 ⟹ 退本金门（holding≥notional ∧ free≥target）仍不满足 ⟹
        // stage 不推进（可达性需足额利润，见 pi_loop_realized_profit_reaches_earning_shares）。
        assert_eq!(tw.stage, TStage::CostReduction, "小额利润不足退本金门（照实）");
        assert_eq!(tw.withdrawn, 0, "无退本金事件");
        assert_eq!(tw.open_legacy_legs, 0, "本场景无 ReverseOpen 腿 ⟹ legacy 计数 0");
        assert!(tw.free > 0, "成本基回流 + 利润入账后 free>0");
    }

    /// ★★GAP3 可达性生产见证（codex 裁定 A' 落地的 L1 证明——「结构不可达」→「现实可达」）：
    /// **带真实盈亏的序列上，生产 π fill loop 的 stage 真推进到 EarningShares**。
    ///
    /// 资金源全程只有实际平仓 fill 的费后已实现 PnL（②'' Realize），无任何未实现浮盈/棘轮参与：
    /// - bar 0-9 px≈100：buy1@3 于 bar 7 确认部署 → bar 8 开多（投 w₀=0.60·NAV=6e5）。
    /// - bar 10-19 px≈1000：sell1@12 于 bar 14 确认 → bar 15 平仓——realized ≈ 6e3 units ×
    ///   (1000−100) ≈ +5.4e6 经 Realize 入 free ⟹ free ≈ 6.4e6 ≫ notional_in=1e6。
    /// - buy1@16 于 bar 17 确认 → bar 18 再建仓（投 0.6·E ≈ 3.8e6 ⟹ holding≥notional_in ∧
    ///   free≈2.5e6≥recover_target=1e6 同时满足——旧 TW 守恒代数下的互斥被已实现利润打破）⟹
    ///   P3 于 bar 18 派 RecoverCapital(1e6)（stage II，withdrawn=notional_in）⟹ P4 于 bar 19
    ///   EnterReady 五合取全真（S=II ∧ W≥I0 ∧ legs=0 ∧ Normal ∧ tw≥η⋆，κ=0 ⟹ η⋆=L^wc=0）⟹
    ///   EnterEarning ⟹ **TStage=EarningShares**。
    ///
    /// 认识论 L1（合成价格序列验证机制可达性——生产判据/账本/事件链全走生产路径，无 fixture
    /// 直捅 stage）；真实数据上的触发频率/盈利性归 L2/L3（#135 重跑）。
    #[test]
    fn pi_loop_realized_profit_reaches_earning_shares() {
        use super::super::super::strategy::ledger::TStage;
        let config = ThetaConfig::default();
        // px 100（bar 0-9）→ px 1000（bar 10-19）：tick_size=1e-8 ⟹ close_tick=1e10/1e11。
        let bars: Vec<Bar> = (0..20)
            .map(|i| mk_bar(i, if i < 10 { 10_000_000_000 } else { 100_000_000_000 }, false))
            .collect();
        // 三段合成闭包：buy1@3（bar7 确认）→ +sell1@12（bar14 确认）→ +buy1@16（bar17 确认）。
        let cls_buy = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3)]), ..Default::default() }],
        };
        let cls_sell = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3), sell_at(12, 1)]), ..Default::default() }],
        };
        let cls_rebuy = Classification {
            levels: vec![LevelState {
                bsp: Rc::new(vec![buy1_at(3), sell_at(12, 1), buy1_at(16)]),
                ..Default::default()
            }],
        };
        let fill = pi_theta_fill_loop(
            move |i| {
                if i >= 17 {
                    (cls_rebuy.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
                } else if i >= 14 {
                    (cls_sell.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
                } else if i >= 7 {
                    (cls_buy.clone(), Vec::new(), Vec::new(), i as u64, i as u64)
                } else {
                    (Classification::default(), Vec::new(), Vec::new(), i as u64, i as u64)
                }
            },
            &bars,
            1.0e6,
            &config,
            None,
        );
        let tw = fill.tw_final.expect("π 路径 TW 生产者就位");
        // ★核心：stage 真推进到 EarningShares（GAP3「∃t TStage=III」从 FALSIFIED → 生产可达）。
        assert_eq!(
            tw.stage,
            TStage::EarningShares,
            "已实现利润入账后三阶段走完：终 stage={:?}（P3 bar18 退本金 → P4 bar19 增股数）",
            tw.stage
        );
        // 本金全退（P3 足额一次性）：withdrawn = notional_in = ⌊nav0⌋。
        assert_eq!(tw.withdrawn, 1_000_000, "RecoverCapital 足额退本金（withdrawn=notional_in）");
        // 资金源审计：TW 漂移不变量在阶段推进后仍成立（RecoverCapital/EnterEarning 保 TW ⟹
        // 漂移仍恰 = 已实现 PnL 量化和——资金源唯一性的端到端见证）。
        let realized_sum: f64 = fill.trade_pnls_realized.iter().sum();
        assert_eq!(
            tw.tw(),
            1_000_000 + realized_sum as i64,
            "阶段推进不产生/不消灭财富：TW 漂移仍 = ⌊Σ已实现PnL⌋"
        );
        assert!(realized_sum > 1_500_000.0, "资金源 = 足额已实现利润（>150% 本金）");
        // 结构不变量保持：free 非负、无 legacy 腿（OQ-9 EnterEarning 入口证书曾满足）。
        assert!(tw.free >= 0, "全程 cash-sound");
        assert_eq!(tw.open_legacy_legs, 0, "EnterEarning 入口证书 legs=0");
    }

    /// ★G4：三类反向卖候选 ⟹ ReduceCore（P6，reverse_exit_type 判据经 fill loop 端到端兑现）。
    #[test]
    fn typed_ledger_reverse_reduce_core() {
        use super::super::super::strategy::interp::ExitType;
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy_then_sell(3), &bars, 1.0e6, &config, None);
        assert_eq!(fill.typed_ledger.len(), 1);
        assert_eq!(fill.typed_ledger[0].exit_type, ExitType::ReduceCore, "根腿 + 三类反向 ⟹ P6 ReduceCore");
    }

    /// ★#145 T1：entry_v=ReverseOpen 腿被**一类**反向候选关闭 ⟹ CloseReverseOpen（P7 子声部
    /// 关闭语义**压过触发类**——同触发类下根腿会派 CloseRoot）。端到端：父根 L1 Long 持仓 →
    /// 子 L0 Short（真父容器 Long ⟹ ReverseOpen，AncOK 持父准入）→ L0 buy1@18（class=1）平子腿。
    /// typed 裁决由组合层 `StepTrace.closed` 携带（#145 前移），runner 只消费不补算。
    #[test]
    fn typed_ledger_reverse_open_close_overrides_trigger_class() {
        use super::super::super::strategy::interp::ExitType;
        let mut config = ThetaConfig::default();
        config.tick.tick_size = 1.0;
        let bars = e1_bars();
        let cls_parent = {
            let mut c = e_classification(false);
            c.levels[0] = LevelState::default(); // 仅父 L1 buy@12（子卖点未现）
            c
        };
        let cls_open = e_classification(false); // + 子 L0 sell@16
        let cls_all = e_classification(true); // + 子平触发 L0 buy1@18（class=1）
        let tower = e_tower();
        let classify = move |i: usize| {
            let cls = if i >= 19 {
                cls_all.clone()
            } else if i >= 17 {
                cls_open.clone()
            } else if i >= 13 {
                cls_parent.clone()
            } else {
                Classification::default()
            };
            (cls, tower.clone(), tower.iter().map(|lv| lv.len()).collect::<Vec<usize>>(), i as u64, i as u64) // 合成静态塔：全塔跨 bar 位稳定 ⟹ confirmed_lens=全长（三方合并 schema 适配）
        };
        let fill = pi_theta_fill_loop(classify, &bars, 1.0e6, &config, None);
        let child: Vec<_> = fill
            .typed_ledger
            .iter()
            .filter(|t| t.entry_z.delta == -1)
            .collect();
        assert_eq!(child.len(), 1, "恰一条子 Short 腿 typed 交易，ledger={:?}", fill.typed_ledger);
        assert_eq!(
            child[0].exit_type,
            ExitType::CloseReverseOpen,
            "entry_v=ReverseOpen + 一类反向触发 ⟹ P7 CloseReverseOpen（压过触发类）"
        );
        assert!(!child[0].via_structural_prune, "真信号平仓（反向候选触发），非结构剪枝");
    }

    /// ★G4：无反向信号 ⟹ 窗口终点 censored（P0 Hold，兑现到末可交易 bar——旧 censored 语义保留）。
    #[test]
    fn typed_ledger_censored_hold_at_window_end() {
        use super::super::super::strategy::interp::ExitType;
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let fill = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
        assert_eq!(fill.typed_ledger.len(), 1);
        let t = &fill.typed_ledger[0];
        assert_eq!(t.exit_type, ExitType::Hold, "未离场腿 ⟹ censored Hold");
        assert_eq!(t.exit_bar, 19, "censored 到末可交易 bar");
        assert_eq!(t.entry_bar, 7);
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
    /// （有向 L1 Long 父走势）→ `assemble_gamma_with_tower` ⟹ V=ReverseOpen（来自**父容器方向**，
    /// **与持仓无关**——`assemble_gamma_with_tower` 不接受 active 参数）。取代旧"活动父腿"错口径 loop 见证。
    #[test]
    fn run_theta_v0_pi_loop_reverse_open_from_parent_container() {
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
        // ★639：未持仓（assemble_gamma_with_tower 不接受 active）仍 ReverseOpen（σ_p=父容器方向）。
        let gamma = assemble_gamma_with_tower(&sliced, &tower);
        assert_eq!(gamma[0].dir, VoiceSide::Short);
        assert_eq!(
            gamma[0].role.v,
            Vertical::ReverseOpen,
            "L0 卖 under L1 Long 父容器 ⟹ ReverseOpen（639：来自父容器方向，非持仓父腿）"
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

    /// ★M5 overlay arm 端到端（多空对冲.pdf p16 关卡10）：`run_theta_v0_pi_overlay` 在锯齿数据上
    /// 跑通——① 净额路径 bit-exact（net_result == run_theta_v0_pi）；② Σpnl_v 对账（reconcile_residual
    /// < eps，PDF §11 线性恒等）；③ ΔN 守恒（fill loop 内 debug_assert 逐 bar，无 panic 即零违例）。
    #[test]
    fn run_theta_v0_pi_overlay_reconciles_and_bit_exact_net() {
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
            symbol: "ZZ60OV".to_string(),
            bars,
            dates: (0..60).map(|i| format!("2024-02-{:02} 00:00:00", (i % 28) + 1)).collect(),
            bar_seconds: 60,
        };
        // 净额路径 bit-exact 对照：overlay arm 的 net_result 与纯净额 run_theta_v0_pi 逐字段一致。
        let baseline = run_theta_v0_pi(&ds, &config, 1.0, 1.0e6);
        let ov = run_theta_v0_pi_overlay(&ds, &config, 1.0, 1.0e6);
        assert_eq!(ov.net_result.n_orders, baseline.n_orders, "overlay 只读旁路 ⟹ 净额订单数 bit-exact");
        assert_eq!(ov.net_result.trades.len(), baseline.trades.len(), "净额 trades bit-exact");
        assert_eq!(
            ov.net_result.metrics.strat_return, baseline.metrics.strat_return,
            "净额 strat_return bit-exact（overlay 不改 cash/units/equity）"
        );
        // 验收断言2：Σpnl_v 对账净额价格 PnL（PDF §11 `Σσ_v q_v ΔP=N ΔP`，f64 结合律残差 < eps）。
        assert!(
            ov.reconcile_residual < 1e-6,
            "M5 对账残差 {} 应 < eps（Σpnl_v={} vs account={}）",
            ov.reconcile_residual, ov.total_voice_pnl, ov.account_price_pnl
        );
        // 逐声部归因表存在性（PDF §M5 entry_v/exit_v/pnl_v）：离场声部归因有限。
        assert!(
            ov.overlay.closed_voices().iter().all(|c| c.pnl_v.is_finite()),
            "逐声部 pnl_v 有限"
        );
        assert!(ov.account_price_pnl.is_finite(), "账户净额价格 PnL 有限");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★★W1：声部独立执行臂（churn 修复，netting-vs-voice-execution-audit-20260719 §7）
    // ──────────────────────────────────────────────────────────────────────

    /// ★W1 决策层 bit-exact + 事件驱动 fill（审计 §7.4 验收）：同一合成信号（buy@3 确认7 →
    /// sell@12 确认14）下，声部臂 typed_ledger/tw_final 与净额臂逐字段一致（决策单源 = 净额
    /// 影子账本，禁第二裁决源）；fill 只在声部开/合产生——20 bar 窗恰 2 个 fill 事件（开+平），
    /// 事件驱动上界 n_fills ≤ 2×n_voices；平仓按声部一次性结算（1 行），非逐 fill 切碎。
    #[test]
    fn voice_exec_event_driven_fills_decision_bitexact() {
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let baseline = pi_theta_fill_loop(buy_then_sell(1), &bars, 1.0e6, &config, None);
        let mut book =
            super::super::super::strategy::overlay_state::VoiceExecBook::new(1.0e6);
        let fill = pi_theta_fill_loop_voice(buy_then_sell(1), &bars, 1.0e6, &config, None, &mut book);
        // ③ 决策层 bit-exact：typed ledger / TW 终态逐字段一致。
        assert_eq!(fill.typed_ledger, baseline.typed_ledger, "决策单源 ⟹ typed_ledger bit-exact");
        assert_eq!(fill.tw_final, baseline.tw_final, "TW 由净额影子驱动 ⟹ tw_final bit-exact");
        // ② 事件驱动 fill：开+平 = 恰 2 个 fill 事件（声部事件率 ≪ bar 率；无事件 bar 零订单）。
        assert_eq!(fill.n_orders, 2, "恰 开+平 2 fill，实得 {}", fill.n_orders);
        assert_eq!(book.n_fills(), 2);
        assert_eq!(book.total_voices(), 1, "恰 1 个声部 campaign");
        assert!(book.n_fills() <= 2 * book.total_voices(), "事件驱动上界（§7.4 断言）");
        // ②' sizing 冻结：毛周转 = q开+q平（存续期零重定——churn 修复直接见证；净额臂同信号
        //    下 trade_pnls 可有多段减仓，声部臂恒 = 1 round-trip 1 结算）。
        let closed = &book.closed_voices()[0];
        assert_eq!(book.gross_turnover_lots() % 2, 0, "G = q开+q平 = 2q（开平对称）");
        assert!(!closed.forced);
        assert!(closed.fee_paid > 0.0, "开+平费实付");
        // 按声部结算：平仓费后已实现恰 1 行 + 1 条 TradeRecord（非逐 fill 切碎）。
        assert_eq!(fill.trade_pnls_realized.len(), 1);
        assert_eq!(fill.trades.len(), 1);
        assert!(!fill.trades[0].forced_close);
        assert_eq!(fill.trade_pnls_realized[0], closed.pnl_net, "输出 PnL = 簿内声部结算行（同源）");
        // 声部账户守恒（R 分解，loop 内 debug_assert 同锚；release 复核）。
        let r = fill.r_decomp.expect("声部臂产 R 分解");
        let tol = 1e-6_f64.max(1e-9 * (1.0e6 + r.price_pnl_gross.abs()));
        assert!(r.conservation_residual.abs() <= tol, "声部账户守恒残差 {} 超容差", r.conservation_residual);
        // 价格 PnL 对账（PDF §11）：Σpnl_price == account_price_pnl。
        assert!((book.total_voice_price_pnl() - book.account_price_pnl()).abs() < 1e-6);
    }

    /// ★W1 sizing 冻结（churn 机制修复直接见证）：锯齿价格（NAV/px 每 bar 剧烈漂移 ⟹ 净额臂
    /// base_units=equity_nav/px 每 bar 重算）下，声部臂仍只有 1 个 fill 事件（开仓）——q_v 开仓
    /// 时刻冻结，存续期零重定（无事件 bar 零订单）；窗口终点虚拟强平产 1 行 forced PnL。
    #[test]
    fn voice_exec_frozen_sizing_under_nav_drift() {
        let config = ThetaConfig::default();
        // 锯齿价（与 run_theta_v0_pi_overlay_reconciles_and_bit_exact_net 同款）：px 大幅摆动
        // ⟹ base_units 每 bar 漂移（churn 机制根源，审计 §5.1），净额臂 target 随之漂移。
        let bars: Vec<Bar> = (0..60)
            .map(|i| {
                let up = ((i / 4) % 2) == 0;
                let base = 10_000_000_000i64;
                let step = 250_000_000i64 * ((i % 4) as i64);
                mk_bar(i, if up { base + step } else { base + 1_000_000_000 - step }, false)
            })
            .collect();
        let baseline = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
        let mut book =
            super::super::super::strategy::overlay_state::VoiceExecBook::new(1.0e6);
        let fill = pi_theta_fill_loop_voice(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None, &mut book);
        // 决策层 bit-exact（锯齿 + 漂移下仍成立——影子账本承载全部决策真值）。
        assert_eq!(fill.typed_ledger, baseline.typed_ledger, "typed_ledger bit-exact");
        assert_eq!(fill.tw_final, baseline.tw_final, "tw_final bit-exact");
        // sizing 冻结核心断言：价格/ NAV 全程漂移，fill 仍只有开仓 1 次（无 churn fill）。
        assert_eq!(fill.n_orders, 1, "冻结 sizing ⟹ 仅开仓 1 fill（存续期零重定），实得 {}", fill.n_orders);
        assert_eq!(book.total_voices(), 1);
        // 窗口终点在飞 1 声部 ⟹ censored 强平（虚拟兑现）：1 行 forced PnL + 1 条 forced TradeRecord。
        assert_eq!(fill.trade_pnls_realized.len(), 0, "无平仓 fill ⟹ 无已实现行");
        assert_eq!(fill.trade_pnls_with_forced.len(), 1, "强平含浮盈 1 行");
        assert_eq!(fill.trades.len(), 1);
        assert!(fill.trades[0].forced_close);
        assert!(book.closed_voices()[0].forced);
        // 守恒（含强平在飞持仓 MtM 的 ledger_delta 口径）。
        let r = fill.r_decomp.expect("R 分解");
        let tol = 1e-6_f64.max(1e-9 * (1.0e6 + r.price_pnl_gross.abs()));
        assert!(r.conservation_residual.abs() <= tol, "守恒残差 {} 超容差", r.conservation_residual);
        assert!((book.total_voice_price_pnl() - book.account_price_pnl()).abs() < 1e-6, "Σpnl_price 对账");
    }

    /// ★W1 无信号零订单（事件驱动的空集情形）：全程无分类事件 ⟹ opened/closed 全空 ⟹
    /// 声部簿零 fill、零持仓、零费用（不伪造执行事实）。
    #[test]
    fn voice_exec_no_signal_zero_fills() {
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let mut book =
            super::super::super::strategy::overlay_state::VoiceExecBook::new(1.0e6);
        let fill = pi_theta_fill_loop_voice(
            |i| (Classification::default(), Vec::new(), Vec::new(), i as u64, i as u64),
            &bars,
            1.0e6,
            &config,
            None,
            &mut book,
        );
        assert_eq!(fill.n_orders, 0);
        assert_eq!(book.n_fills(), 0);
        assert_eq!(book.gross_turnover_lots(), 0);
        assert!(fill.trade_pnls_with_forced.is_empty());
        assert!(fill.equity_curve.iter().all(|&e| e == 1.0), "无持仓零费用 ⟹ 权益恒 1");
    }

    /// ★W1 env gate（④⑤）：VOICE_EXEC 关闭 ⟹ 生产路径逐字节不变（bit-exact 回归锁）；
    /// 开启 ⟹ 声部执行读数装配 + 决策层 TW 与净额 bit-exact + 验收量（上界/守恒/对账）成立。
    /// 线程局部注入（并行安全；进程级 env 不动——OPSEM_DUMP_DIR_OVERRIDE 同惯例）。
    #[test]
    fn voice_exec_env_gate_off_bitexact_on_voice_readings() {
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
            symbol: "ZZ60W1".to_string(),
            bars,
            dates: (0..60).map(|i| format!("2024-02-{:02} 00:00:00", (i % 28) + 1)).collect(),
            bar_seconds: 60,
        };
        let baseline = run_theta_v0_pi(&ds, &config, 1.0, 1.0e6);
        // (1) gate 关 ⟹ 逐字节不变（env 未设语义，线程局部注入 false 显式锚定）。
        VOICE_EXEC_OVERRIDE.with(|c| c.set(Some(false)));
        let off = run_theta_v0_pi_overlay(&ds, &config, 1.0, 1.0e6);
        assert!(off.voice_exec.is_none(), "gate 关 ⟹ 无声部读数（None，不伪造）");
        assert_eq!(off.net_result.n_orders, baseline.n_orders, "gate 关 ⟹ n_orders bit-exact");
        assert_eq!(off.net_result.trades, baseline.trades, "trades bit-exact");
        assert_eq!(
            off.net_result.metrics.strat_return, baseline.metrics.strat_return,
            "strat_return bit-exact"
        );
        assert_eq!(off.net_result.equity_curve, baseline.equity_curve, "equity bit-exact");
        // (2) gate 开 ⟹ 声部执行读数；决策层 TW 与净额 bit-exact。
        VOICE_EXEC_OVERRIDE.with(|c| c.set(Some(true)));
        let on = run_theta_v0_pi_overlay(&ds, &config, 1.0, 1.0e6);
        VOICE_EXEC_OVERRIDE.with(|c| c.set(None));
        let vs = on.voice_exec.expect("VOICE_EXEC=1 ⟹ 声部执行读数 Some");
        assert_eq!(on.tw_final, off.tw_final, "TW 由净额影子驱动 ⟹ gate 开/关 bit-exact");
        assert_eq!(vs.n_voice_fills, on.net_result.n_orders, "n_orders = 声部 fill 事件数（同源）");
        assert!(vs.event_bound_holds, "n_voice_fills({}) ≤ 2×n_voices({})", vs.n_voice_fills, vs.n_voices_total);
        assert_eq!(vs.n_voice_fills, vs.book.n_fills(), "读数与簿同源");
        let tol = 1e-6_f64.max(1e-9 * 1.0e6);
        assert!(vs.conservation_residual.abs() <= tol, "声部账户守恒残差 {} 超容差", vs.conservation_residual);
        assert!(vs.price_pnl_reconcile_residual < 1e-6, "价格 PnL 对账残差 {}", vs.price_pnl_reconcile_residual);
        // 事件驱动上界（构造性）：fill 数 ≤ 2×声部数（resize 事件默认空集）。
        assert!(vs.n_voice_fills <= 2 * vs.n_voices_total, "事件驱动上界");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★nest-gate（进出场区间套证书门，实装报告 nest-exit-gate-impl-20260719）
    // ──────────────────────────────────────────────────────────────────────

    /// nest-gate 夹具：lvl0 单段塔 [15,19]（执行段 end_index=19）。
    fn ng_tower() -> Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>> {
        use super::super::super::classifier::descend::RMove;
        use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove};
        use super::super::super::types::Direction;
        vec![std::rc::Rc::new(vec![LeveledMove {
            rmove: RMove::Segment { direction: Direction::Down, lo: 30, hi: 85 },
            start_index: 15,
            end_index: 19,
            sub_moves: std::rc::Rc::new(vec![]),
            id: ElementId { level: 0, ordinal: 0 },
        }])]
    }

    /// nest-gate 夹具：levels[0].bsp 携单个 source_index=19 候选点的前缀分类。
    fn ng_classification(bits: super::super::super::types::BspBits) -> classifier::Classification {
        use super::super::super::classifier::bsp::BspPoint;
        use super::super::super::classifier::LevelState;
        classifier::Classification {
            levels: vec![LevelState {
                moves: Vec::new(),
                // #218 面 B（机械改写归因）：账本层补 B 中枢（带 (0,0)，start=20 查找键）——
                // 各 nest-gate 测试的 owner 夹具点带判等（Center 载体带 (0,0) == B 带）。
                centers: std::rc::Rc::new(vec![
                    super::super::super::types::Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 20, end_index: 0 },
                ]),
                cp_ownership: std::rc::Rc::new(Vec::new()),
                bsp: std::rc::Rc::new(vec![BspPoint {
                    source_index: 19,
                    level_origin: 0,
                    bits,
                    pivot_low: 0,
                    pivot_high: 0,
                    center: None,
                    struct_break_dir: None,
                    force: None,
                }]),
                pan_div: std::rc::Rc::new(Vec::new()),
                level_projection: None,
            }],
            ..Default::default()
        }
    }

    /// nest-gate 夹具：候选（level=0，显式 dir/bits/source_index；role/nest_confirmed 不参门）。
    fn ng_candidate(
        dir: super::super::super::strategy::voice::VoiceSide,
        bits: super::super::super::types::BspBits,
        source_index: usize,
    ) -> super::super::super::strategy::interp::Candidate {
        use super::super::super::strategy::coverage::{Dir, GradeRel, Horizontal, OperationRole, Vertical};
        super::super::super::strategy::interp::Candidate {
            level: 0,
            source_index,
            bits,
            dir,
            bsp_class: 3,
            role: OperationRole { h: Horizontal::First, v: Vertical::Ambient, delta: Dir::Plus, grade: GradeRel::SameLevel },
            nest_confirmed: false, // 贴标字段不参门（门消费 build_gate_certificate，单一裁决源）
            gamma_index: 0,
            force: None,
        }
    }

    /// NG-E①（合法证书放行）：lvl==0 真三类 buy3 候选——Type3 base gate lvl0 存在性免门
    /// （econ_positive.rs:2081-2086 同夹具先例）⟹ Nest 通道证书 Some，rungs 空基例
    /// n_delta = Conf⁺_e = buy3 置位 ⟹ 放行（通道 nest_pass）。
    /// NG-E②（非法拒绝）：(a) 无执行段（source_index=25 不在塔中）⟹ 证书 None ⟹ 拒；
    /// (b) 零 bit StructBreak 候选 ⟹ 恒 None 门拒（codex 终局裁决A，econ_positive.rs:2050
    /// 同夹具先例）；(c) Flat 方向 ⟹ 拒（与 nest_confirm interp.rs:382 同语义）；
    /// (d) 两级塔上级 rung cand=false（空 sub_moves，Type1 div_cand 假）⟹ n_delta=false
    /// ⟹ 跨级递归真拒绝（可修复分量）。
    /// NG-E③（内禀分量照实）：单级塔 rungs 空 ⟹ 基例退化放行（见断言处注释）。
    #[test]
    fn nest_gate_admit_semantics() {
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::BspBits;
        let tower = ng_tower();
        let hist: Vec<f64> = (0..20).map(|_| 1.0).collect();
        // E① 放行：真三类 buy3，lvl==0。
        let buy3 = BspBits { buy3: true, ..Default::default() };
        let cls = ng_classification(buy3);
        let c = ng_candidate(VoiceSide::Long, buy3, 19);
        assert_eq!(
            nest_gate_admit(&tower, &c, &hist, 19, &cls),
            (true, "nest_pass"),
            "合法 typed N^δ 证书（lvl0 Type3 基例）⟹ 放行"
        );
        // E②a 拒：无执行段定位（塔中无 end_index==25 的段）。
        let c_no_seg = ng_candidate(VoiceSide::Long, buy3, 25);
        assert_eq!(
            nest_gate_admit(&tower, &c_no_seg, &hist, 19, &cls),
            (false, "cert_none"),
            "无执行段 ⟹ 证书 None ⟹ 拒"
        );
        // E②b 拒：零 bit StructBreak（不复用 Type3 免门通道）。
        let zero = BspBits::default();
        let cls_zero = ng_classification(zero);
        let c_zero = ng_candidate(VoiceSide::Long, zero, 19);
        assert_eq!(
            nest_gate_admit(&tower, &c_zero, &hist, 19, &cls_zero),
            (false, "cert_none"),
            "零 bit StructBreak ⟹ 恒门拒"
        );
        // E②c 拒：Flat 方向（无方向无确认）。
        let c_flat = ng_candidate(VoiceSide::Flat, buy3, 19);
        assert_eq!(
            nest_gate_admit(&tower, &c_flat, &hist, 19, &cls),
            (false, "flat_dir"),
            "Flat 方向候选 ⟹ 拒"
        );
        // E③（内禀分量照实登记）：Type1 buy1 单级塔（rungs 空）⟹ n_delta 退化为基例
        // Conf⁺_e=buy1 ⟹ **放行**——单级读不出的伪候选只能由跨级 rungs/base gate 构造性
        // 拒绝（econ 实测 95.36% 基例退化同型，econ_positive.rs:349-352），鉴别力稀薄
        // 内禀于塔现状，不是门实装缺陷。
        let buy1 = BspBits { buy1: true, ..Default::default() };
        let cls_t1 = ng_classification(buy1);
        let c_t1 = ng_candidate(VoiceSide::Long, buy1, 19);
        assert_eq!(
            nest_gate_admit(&tower, &c_t1, &hist, 19, &cls_t1),
            (true, "nest_pass"),
            "单级塔 rungs 空 ⟹ 基例退化放行（内禀分量，照实登记）"
        );
        // E②d 拒（可修复分量，跨级递归真拒绝）：两级塔——上级 rung 存在但
        // cand=false（Type1 div_cand 对空 sub_moves 假）⟹ n_delta=false ⟹ 拒。
        use super::super::super::classifier::descend::RMove;
        use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove};
        use super::super::super::types::Direction;
        let upper = LeveledMove {
            rmove: RMove::Segment { direction: Direction::Up, lo: 20, hi: 90 },
            start_index: 10,
            end_index: 30,
            sub_moves: std::rc::Rc::new(vec![]), // 空 ⟹ rung cand=false
            id: ElementId { level: 1, ordinal: 0 },
        };
        let mut tower2 = ng_tower();
        tower2.push(std::rc::Rc::new(vec![upper]));
        assert_eq!(
            nest_gate_admit(&tower2, &c_t1, &hist, 19, &cls_t1),
            (false, "nest_n_delta_false"),
            "跨级 rung cand=false ⟹ N^δ=0 ⟹ 真拒绝（区间套递归分量）"
        );
    }

    /// NG-E③（同源对拍，实装卡 §3.4-2）：π 新门的通过读出（Nest→n_delta / Xzd→gate_pass /
    /// None→拒）与 econ 门读出（econ_positive.rs:367-371）逐候选一致——对同一批夹具候选
    /// 独立构造两条读出路径比对，不一致 = 第二裁决源实锤。
    #[test]
    fn nest_gate_readout_matches_econ_gate() {
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::{BspBits, Side};
        let tower = ng_tower();
        let hist: Vec<f64> = (0..20).map(|_| 1.0).collect();
        for bits in [
            BspBits { buy3: true, ..Default::default() },
            BspBits { buy1: true, ..Default::default() },
            BspBits::default(),
        ] {
            let cls = ng_classification(bits);
            let c = ng_candidate(VoiceSide::Long, bits, 19);
            // π 门读出（生产消费点）。
            let (pi_pass, _) = nest_gate_admit(&tower, &c, &hist, 19, &cls);
            // econ 门读出（econ_positive.rs:364-371 同判据路径，独立构造）。
            let econ_pass = match super::super::econ_positive::build_gate_certificate(
                &tower, 0, 19, Side::Long, &bits, &hist, 19, &cls.levels[0].bsp, &[], &[],
            ) {
                Some(super::super::econ_positive::GateCertificate::Nest(cert)) => cert.n_delta(),
                Some(super::super::econ_positive::GateCertificate::Xzd(ev)) => ev.gate_pass(),
                None => false,
            };
            assert_eq!(pi_pass, econ_pass, "bits={bits:?}：π/econ 门读出发散 = 第二裁决源");
        }
    }

    /// NG-E④（默认路径回归锁）：门关闭（env 未设语义，线程局部注入 false 显式锚定）⟹
    /// π 路径逐字节不变；门开启 ⟹ 候选集只缩不增（订单数 ≤ 基线，门的设计语义，非回归）。
    #[test]
    fn nest_gate_env_gate_off_bitexact_on_shrinks_only() {
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
            symbol: "ZZ60NG".to_string(),
            bars,
            dates: (0..60).map(|i| format!("2024-02-{:02} 00:00:00", (i % 28) + 1)).collect(),
            bar_seconds: 60,
        };
        let baseline = run_theta_v0_pi(&ds, &config, 1.0, 1.0e6);
        // (1) 门关 ⟹ 逐字节不变（bit-exact 回归锁，VOICE_EXEC 同惯例）。
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(Some(false)));
        let off = run_theta_v0_pi(&ds, &config, 1.0, 1.0e6);
        assert_eq!(off.n_orders, baseline.n_orders, "门关 ⟹ n_orders bit-exact");
        assert_eq!(off.trades, baseline.trades, "门关 ⟹ trades bit-exact");
        assert_eq!(
            off.metrics.strat_return, baseline.metrics.strat_return,
            "门关 ⟹ strat_return bit-exact"
        );
        assert_eq!(off.equity_curve, baseline.equity_curve, "门关 ⟹ equity bit-exact");
        // (2) 门开 ⟹ 准入只缩不增（剔除 ⟹ 不开仓；μ 桶键/R5-1 不触碰）。
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(Some(true)));
        let on = run_theta_v0_pi(&ds, &config, 1.0, 1.0e6);
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(None));
        assert!(
            on.n_orders <= baseline.n_orders,
            "门开 ⟹ 订单只缩不增（on={} baseline={}）",
            on.n_orders, baseline.n_orders
        );
        assert!(
            on.trades.len() <= baseline.trades.len(),
            "门开 ⟹ trades 只缩不增"
        );
    }

    // ───────────── #75（N3-T2 进场门真链切换）─────────────

    /// #75 夹具：合成 typed nest 事件（nest_index.rs 测试同型；`b_center_start`=20 配合
    /// 终端背书点 owner 中枢 start_index=20）。
    fn nc_event(
        level: u32,
        side: super::super::super::types::Side,
        interval_b: (usize, usize),
        turn_source: usize,
        judge_at: usize,
        confirmed: bool,
    ) -> classifier::level_view::NestCandidateEvent {
        classifier::level_view::NestCandidateEvent {
            level,
            side,
            kind: classifier::level_view::NestDivergenceKind::Trend,
            seg_a: (interval_b.0.saturating_sub(5), interval_b.0),
            interval_b,
            interval_a: (interval_b.0.saturating_sub(10), interval_b.1 + 10),
            divergence_confirmed: confirmed,
            turn_source,
            judge_at,
            provider_window: (0, usize::MAX),
            intake_fallback: false,
            b_center_start: 20,
        }
    }

    fn nc_ext(
        event: classifier::level_view::NestCandidateEvent,
    ) -> classifier::level_view::NestCandidateEventExt {
        // T1 (#170)：无锚夹具（None = 缺锚路径）——旧用例语义不变；需锚用 `nc_ext_anchored`。
        // T5b (#208)：`seg_c_full` 值桥载体已删（出场旧桥四件删除账）。
        classifier::level_view::NestCandidateEventExt {
            event,
            extreme_price: None,
            group_anchor: None,
        }
    }

    /// T1 (#170) 夹具：携两元锚 sidecar 的 ext（方向留 event.side 交易层，T5a 不进身份键）。
    fn nc_ext_anchored(
        event: classifier::level_view::NestCandidateEvent,
        price: super::super::super::types::Tick,
        anchor: usize,
    ) -> classifier::level_view::NestCandidateEventExt {
        classifier::level_view::NestCandidateEventExt {
            event,
            extreme_price: Some(price),
            group_anchor: Some(anchor),
        }
    }

    /// T1 (#170) 键域重锚登记 → T5a (#207 去方向位)：新单键域 (ℓ, 极值价, 组锚@ℓ) →
    /// 身份集——无门纯增写；极值价精确等值无容差（1 tick 差即分键，v3 硬禁令）；同价
    /// 碰撞（W 底双脚）由合并组区分；缺锚事件跳过新键域并照实计数（诚实缺锚）；**方向位
    /// 自身份键退役**（ADR 20260723 裁定 1——跨向事件同键合流）。T4 (#173)：`by_end_multi`
    /// 登记已删；T5b (#208)：旧出场侧 `by_end` 登记已删（#206 Q3 判删）。
    #[test]
    fn absorb_registers_triple_anchor_domain_additively() {
        use super::super::super::types::Side;
        let mut gate = NestChainGate::for_test(Vec::new(), Vec::new(), Vec::new());
        let e1 = nc_event(1, Side::Long, (30, 50), 50, 100, true);
        let e2 = nc_event(1, Side::Long, (60, 90), 90, 110, true);
        gate.absorb_exts(vec![
            nc_ext_anchored(e1, 1000, 55),
            nc_ext_anchored(e2, 1001, 95), // 极值价仅差 1 tick
        ]);
        let id1 = classifier::nest::NestEventIdentity::of(&e1);
        let id2 = classifier::nest::NestEventIdentity::of(&e2);
        // 新键域：(ℓ, 极值价, 组锚) → 身份集；1 tick 价差分键（精确等值、无容差）。
        assert_eq!(
            gate.by_triple_anchor.get(&(1, 1000, 55)).map(Vec::as_slice),
            Some(&[id1][..])
        );
        assert_eq!(
            gate.by_triple_anchor.get(&(1, 1001, 95)).map(Vec::as_slice),
            Some(&[id2][..]),
            "极值价 1 tick 差即分键（无容差，v3 硬禁令）"
        );
        assert_eq!(gate.by_triple_anchor.len(), 2);
        assert_eq!(gate.n_anchor_misses, 0);
        // 缺锚事件：新键域跳过 + 计数（诚实缺锚，不冒充不降级）；事件账本照登。
        let e3 = nc_event(2, Side::Short, (10, 20), 20, 120, true);
        gate.absorb_exts(vec![nc_ext(e3)]);
        assert_eq!(gate.events_by_level[2].len(), 1, "缺锚不影响事件账本登记");
        assert_eq!(gate.by_triple_anchor.len(), 2, "缺锚事件不入新键域");
        assert_eq!(gate.n_anchor_misses, 1, "缺锚计数照实落账");
        // W 底双脚：同价不同组锚 ⟹ 两个键（教义裁定 4：碰撞由合并组区分，方向退役后同）。
        let f1 = nc_event(3, Side::Long, (100, 110), 110, 130, true);
        let f2 = nc_event(3, Side::Long, (200, 210), 210, 140, true);
        gate.absorb_exts(vec![
            nc_ext_anchored(f1, 888, 10),
            nc_ext_anchored(f2, 888, 20),
        ]);
        assert!(gate.by_triple_anchor.contains_key(&(3, 888, 10)));
        assert!(gate.by_triple_anchor.contains_key(&(3, 888, 20)), "同价碰撞由合并组区分");
    }

    /// #75-T1（增量喂法账本纪律）：只收确认事件（未确认过不了基例门/N2 门，对索引零贡献）；
    /// 同身份 first-wins（judge_at 保首次观察，不后移）。T5b (#208)：旧 `by_end` 值桥键
    /// 登记断言随出场桥删除（#206 Q3 判删）。
    #[test]
    fn nest_chain_absorb_confirmed_only_first_wins() {
        use super::super::super::types::Side;
        let mut gate = NestChainGate::for_test(Vec::new(), Vec::new(), Vec::new());
        let confirmed = nc_event(1, Side::Long, (30, 50), 50, 100, true);
        let unconfirmed = nc_event(1, Side::Long, (60, 90), 90, 110, false);
        gate.absorb_exts(vec![nc_ext(confirmed), nc_ext(unconfirmed)]);
        assert_eq!(gate.events_by_level.len(), 2, "事件账本按级落槽");
        assert_eq!(gate.events_by_level[1].len(), 1, "未确认事件不入账本");
        assert_eq!(gate.events_by_level[1][0].judge_at, 100);
        // 同身份再观察（judge_at=200 更晚）⟹ first-wins，不后移。
        let later = nc_event(1, Side::Long, (30, 50), 50, 200, true);
        gate.absorb_exts(vec![nc_ext(later)]);
        assert_eq!(gate.events_by_level[1].len(), 1, "同身份去重");
        assert_eq!(gate.events_by_level[1][0].judge_at, 100, "首次观察钟不后移");
    }

    /// #112-T1 → T3 (#172) 改写（判定源迁移）：候选 origin=0、证书仅在 ℓ=2（origin+2）时，
    /// 严格链要求 [L0, 链顶] 逐级闭合——L0/L1 缺证 ⟹ 断/缺拒（nest_n_delta_false），
    /// **不再**放行。（T3 前本测试断言「multi ℓ=2 命中成为进场唯一判定源 nest_pass」——
    /// 该形态即 #163 裁定 1 判退役的并集跳查：高级有证而中间断不算。T4 (#173)：并集
    /// shadow 列已随旧桥退役删除，本测试只锁链判定与谱系。）
    #[test]
    fn nest_chain_gate_admit_consumes_deeper_multi_level_hit() {
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::BspBits;
        let tower = ng_tower();
        let hist: Vec<f64> = (0..20).map(|_| 1.0).collect();
        // T3 链夹具：存在性 L0/L1/L2 全置，证书仅 ℓ=3（账本级 L2）——L0/L1 缺环。
        let classification = t3_chain_classification(&[(0, true), (1, true), (2, true)]);
        let gate = t3_gate_with_certs(&[3], 100, &classification);
        let buy1 = BspBits { buy1: true, ..Default::default() };
        let candidate = ng_candidate(VoiceSide::Long, buy1, 55);
        let (admit, channel, obs) =
            gate.admit(&tower, &candidate, &hist, 100, &classification);
        assert_eq!(
            (admit, channel),
            (false, "nest_n_delta_false"),
            "T3 链判定：L2 独证而 L0/L1 缺 ⟹ 缺/断拒"
        );
        assert_eq!(obs.chain_top, Some(2), "链顶 = L2（层间联动给出）");
        assert_eq!(obs.chain_verdict, ChainVerdict::Reject);
        assert_eq!(
            obs.chain_first_gap,
            Some((1, ChainGapKind::Broken)),
            "L1 上方有 L2 闭合 ⟹ 断（高级有证而中间断）"
        );
        let mut stats = NestGateStats::default();
        stats.observe(admit, channel, obs);
        assert_eq!(stats.chain_reject_broken, 1, "链断环拒计数");
    }

    /// #112-T2 → T3 (#172)（进场因果守卫）：唯一证书 judge_at=100 越过 anchor=19；旧 fixed
    /// 桥读出虽命中（fixed 桥无因果守卫——该桥四件已随 T5b (#208) 删除），严格链整证剔除
    /// （MissingCausal）⟹ NoChain，进入既有 Xzd fallback 三分支，而不是消费未来证书。
    /// T4 (#173)：multi 路径与对照列已删，本测试锁链守卫 + 回退。
    #[test]
    fn nest_chain_gate_multi_causal_guard_falls_back_to_xzd() {
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::{BspBits, Side};
        let tower = ng_tower();
        let hist: Vec<f64> = (0..20).map(|_| 1.0).collect();
        let mut gate = NestChainGate::for_test(Vec::new(), Vec::new(), Vec::new());
        let event = nc_event(1, Side::Long, (30, 50), 50, 100, true);
        gate.absorb_exts(vec![nc_ext(event)]);

        // 同一 levels[0] 同时承载：旧臂 Type3 候选点（src=19）与 typed 证书终端背书点。
        let buy3 = BspBits { buy3: true, ..Default::default() };
        let buy1 = BspBits { buy1: true, ..Default::default() };
        let endorsement_pt = {
            use super::super::super::classifier::bsp::BspPoint;
            use super::super::super::types::Center;
            BspPoint {
                source_index: 40, // owner 中枢 start=20（nc_event b_center_start=20 配套）
                level_origin: 0,
                bits: buy1,
                pivot_low: 0,
                pivot_high: 0,
                center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 20, end_index: 0 })),
                struct_break_dir: None,
                force: None,
            }
        };
        let mut classification = ng_classification(buy3);
        std::rc::Rc::make_mut(&mut classification.levels[0].bsp).push(endorsement_pt);
        gate.sync_index(&classification);
        let id = classifier::nest::NestEventIdentity::of(&event);
        assert!(
            gate.index.get(&id).is_some_and(|cert| cert.certificate().n_delta()),
            "夹具前提：证书已装配且 n_delta 过（无守卫读法（旧 fixed 桥）会读到这张未来证书——链守卫整证剔除的对象）"
        );

        let candidate = ng_candidate(VoiceSide::Long, buy3, 19);
        let (admit, channel, obs) =
            gate.admit(&tower, &candidate, &hist, 19, &classification);
        assert!(matches!(channel, "xzd_pass" | "xzd_gate_fail"), "越界证书剔除后走 Xzd，实际={channel}");
        assert!(obs.xzd_fallback, "old-arm 为 Nest 时重走 build_xzd_fallback 既有分支");
        assert_eq!(admit, channel == "xzd_pass", "判定来自既有 Xzd gate_pass 读出");
    }

    /// #75-T3 → T3 (#172) 改写（判定源迁移）：(a) 链全闭合（L0 单级链）⟹ nest_pass；
    /// (b) 链 NoChain + Type1 ⟹ cert_none；(c) 链 NoChain ∧ L2 有证 ⟹ Xzd 回退（判定不消费
    /// L2 nest 读出）；L2 旧臂对照差落账（cross_old_rej_new_pass 等）。T4 (#173)：并集
    /// shadow 列已删，真链命中/链深读数不再入账。
    #[test]
    fn nest_chain_gate_typed_decides_l2_only_cross() {
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::{BspBits, Side};
        let tower = ng_tower();
        let hist: Vec<f64> = (0..20).map(|_| 1.0).collect();
        let mut stats = NestGateStats::default();
        // (a) 链全闭合准入：L0 存在 + 证书 ℓ=1（账本级 L0）⟹ 链 Pass ⟹ nest_pass；
        //     旧臂在 src=55 无执行段 ⟹ None ⟹ old_admit=false ⟹ cross 旧拒新准。
        let cls_chain = t3_chain_classification(&[(0, true)]);
        let gate = t3_gate_with_certs(&[1], 100, &cls_chain);
        let buy1 = BspBits { buy1: true, ..Default::default() };
        let c_hit = ng_candidate(VoiceSide::Long, buy1, 55);
        let (admit, channel, obs) = gate.admit(&tower, &c_hit, &hist, 100, &cls_chain);
        assert_eq!((admit, channel), (true, "nest_pass"), "链全闭合 ⟹ 唯一判定源准入");
        assert_eq!(obs.chain_verdict, ChainVerdict::Pass);
        assert_eq!(obs.chain_closed_down_to, Some(0), "单级链闭合到 L0");
        assert!(!obs.old_admit && !obs.xzd_fallback, "旧臂对照：无执行段 ⟹ None");
        stats.observe(admit, channel, obs);
        // (b) 链 NoChain 案例：x=56 无分型（price 不可解）+ Type1 + 旧臂亦无执行段 ⟹ cert_none。
        let c_miss = ng_candidate(VoiceSide::Long, buy1, 56);
        let (admit, channel, obs) = gate.admit(&tower, &c_miss, &hist, 100, &cls_chain);
        assert_eq!((admit, channel), (false, "cert_none"), "链 NoChain + Type1 ⟹ 拒");
        assert_eq!(obs.chain_verdict, ChainVerdict::NoChain);
        stats.observe(admit, channel, obs);
        // (c) typed 无证 ∧ L2 有证（ng E①：lvl0 Type3 旧臂 nest_pass）⟹ 判定走 Xzd 回退，
        //     **不**消费 L2 nest 读出；回退判定与 build_xzd_fallback 独立调用逐值一致。
        let gate_empty = NestChainGate::for_test(Vec::new(), Vec::new(), Vec::new());
        let buy3 = BspBits { buy3: true, ..Default::default() };
        let cls_b3 = ng_classification(buy3);
        let c_b3 = ng_candidate(VoiceSide::Long, buy3, 19);
        let (admit, channel, obs) = gate_empty.admit(&tower, &c_b3, &hist, 19, &cls_b3);
        assert!(matches!(channel, "xzd_pass" | "xzd_gate_fail"), "Xzd 回退通道，实际={channel}");
        assert!(obs.xzd_fallback, "Xzd 回退计数标记");
        assert!(obs.old_admit, "旧臂对照：lvl0 Type3 免门 ⟹ nest_pass");
        let expected = super::super::econ_positive::build_xzd_fallback(
            &tower, 0, 19, Side::Long, &buy3, 19, &cls_b3.levels[0].bsp, &[], &[],
        )
        .map(|ev| ev.gate_pass())
        .unwrap_or(false);
        assert_eq!(admit, expected, "回退判定 = build_xzd_fallback 单一来源");
        stats.observe(admit, channel, obs);
        // (d) 对照读出一致性落账：(a) 旧拒新准 ×1、(c) 旧准（新走 Xzd 回退，结果由 gate_pass 定）。
        assert_eq!(stats.cross_old_rej_new_pass, 1, "(a) cross：旧拒新准");
        assert_eq!(stats.xzd_fallback, 1, "(c) Xzd 回退计数");
        let expected_cross_old_pass_new_rej = usize::from(!expected);
        assert_eq!(
            stats.cross_old_pass_new_rej, expected_cross_old_pass_new_rej,
            "(c) cross：旧准新拒 ⟺ Xzd 回退未通过"
        );
    }

    /// #75-T4（Xzd 复用路径）：typed 无证 ∧ 旧臂已落 Xzd ⟹ 直接复用旧臂 Xzd 判定
    ///（不重复算 xiaozhuanda_confirm，不打 xzd_fallback 标记）。
    /// T4 (#173) 改写注：「typed 无证」的现机制 = 链 NoChain（脚不可解）；
    /// 本测试只锁复用语义（旧臂 Xzd 读出 = `build_xzd_fallback` 单一来源 ⟹ 直接复用）。
    #[test]
    fn nest_chain_gate_typed_none_reuses_old_xzd() {
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::BspBits;
        let tower = ng_tower();
        let hist: Vec<f64> = (0..20).map(|_| 1.0).collect();
        // lvl=1 Type3 候选：旧臂 nest base gate（定律一下沉锚）在空次级账本下失败 ⟹ Xzd 通道。
        use super::super::super::classifier::descend::RMove;
        use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove};
        use super::super::super::classifier::LevelState;
        use super::super::super::types::Direction;
        let upper = LeveledMove {
            rmove: RMove::Segment { direction: Direction::Up, lo: 20, hi: 90 },
            start_index: 10,
            end_index: 30,
            sub_moves: std::rc::Rc::new(vec![]),
            id: ElementId { level: 1, ordinal: 0 },
        };
        let mut tower2 = tower.clone();
        tower2.push(std::rc::Rc::new(vec![upper]));
        let buy3 = BspBits { buy3: true, ..Default::default() };
        let mk_pt = |src: usize| super::super::super::classifier::bsp::BspPoint {
            source_index: src,
            level_origin: 0,
            bits: buy3,
            pivot_low: 0,
            pivot_high: 0,
            center: None,
            struct_break_dir: None,
            force: None,
        };
        let cls2 = classifier::Classification {
            levels: vec![
                LevelState { bsp: std::rc::Rc::new(vec![]), ..Default::default() },
                LevelState { bsp: std::rc::Rc::new(vec![mk_pt(30)]), ..Default::default() },
            ],
            ..Default::default()
        };
        let c = super::super::super::strategy::interp::Candidate { level: 1, ..ng_candidate(VoiceSide::Long, buy3, 30) };
        let (old_admit, old_channel) = nest_gate_admit(&tower2, &c, &hist, 19, &cls2);
        assert!(
            matches!(old_channel, "xzd_pass" | "xzd_gate_fail"),
            "夹具前提：旧臂落 Xzd 通道，实际={old_channel}"
        );
        let gate = NestChainGate::for_test(Vec::new(), Vec::new(), Vec::new());
        let (admit, channel, obs) = gate.admit(&tower2, &c, &hist, 19, &cls2);
        assert_eq!((admit, channel), (old_admit, old_channel), "旧臂 Xzd 判定直接复用");
        assert!(!obs.xzd_fallback, "复用路径不打回退标记");
    }

    /// #94（评审修复2）：cross 对照差剔除自证成分——typed 无证 ∧ 旧臂 Xzd 复用案例
    /// admit≡old_admit by construction（判定复用旧臂读出），计入 cross_reuse 单列，
    /// **不计入** cross_agree/old_pass_new_rej/old_rej_new_pass（复用通道非两路独立判定，
    /// 其「一致」是自证，掺入 agree 会灌水对照差）。夹具同 T4。
    #[test]
    fn nest_chain_gate_cross_reuse_excluded_from_agree() {
        use super::super::super::classifier::descend::RMove;
        use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove};
        use super::super::super::classifier::LevelState;
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::{BspBits, Direction};
        let tower = ng_tower();
        let hist: Vec<f64> = (0..20).map(|_| 1.0).collect();
        // lvl=1 Type3 候选：旧臂 nest base gate（定律一下沉锚）在空次级账本下失败 ⟹ Xzd 通道。
        let upper = LeveledMove {
            rmove: RMove::Segment { direction: Direction::Up, lo: 20, hi: 90 },
            start_index: 10,
            end_index: 30,
            sub_moves: std::rc::Rc::new(vec![]),
            id: ElementId { level: 1, ordinal: 0 },
        };
        let mut tower2 = tower.clone();
        tower2.push(std::rc::Rc::new(vec![upper]));
        let buy3 = BspBits { buy3: true, ..Default::default() };
        let mk_pt = |src: usize| super::super::super::classifier::bsp::BspPoint {
            source_index: src,
            level_origin: 0,
            bits: buy3,
            pivot_low: 0,
            pivot_high: 0,
            center: None,
            struct_break_dir: None,
            force: None,
        };
        let cls2 = classifier::Classification {
            levels: vec![
                LevelState { bsp: std::rc::Rc::new(vec![]), ..Default::default() },
                LevelState { bsp: std::rc::Rc::new(vec![mk_pt(30)]), ..Default::default() },
            ],
            ..Default::default()
        };
        let c = super::super::super::strategy::interp::Candidate { level: 1, ..ng_candidate(VoiceSide::Long, buy3, 30) };
        // 夹具前提：typed 无证（现机制 = 链 NoChain，同 T4）∧ 旧臂落 Xzd 通道。
        let (_old_admit, old_channel) = nest_gate_admit(&tower2, &c, &hist, 19, &cls2);
        assert!(
            matches!(old_channel, "xzd_pass" | "xzd_gate_fail"),
            "夹具前提：旧臂落 Xzd 通道，实际={old_channel}"
        );
        let gate = NestChainGate::for_test(Vec::new(), Vec::new(), Vec::new());
        let (admit, channel, obs) = gate.admit(&tower2, &c, &hist, 19, &cls2);
        assert!(obs.reused_old_xzd, "复用通道打 reused_old_xzd 标记");
        assert_eq!(admit, obs.old_admit, "复用通道 admit≡old_admit by construction");
        let mut stats = NestGateStats::default();
        stats.observe(admit, channel, obs);
        assert_eq!(stats.cross_reuse, 1, "复用计入 cross_reuse 单列");
        assert_eq!(stats.cross_agree, 0, "复用不计入 agree（剔除自证成分）");
        assert_eq!(
            stats.cross_old_pass_new_rej + stats.cross_old_rej_new_pass,
            0,
            "复用不入对照差两列"
        );
    }

    // ───────────── T3（#172 严格链判定：链谱系；T4 (#173)：typed_lookup_multi/并集 shadow 已删；
    // ───────────── T5a (#207)：方向退役——链查询/键域/层索引不再携带方向，方向见证已删）─────────────
    //
    // 语义 pin（#163 裁定 1 字面 + ADR 六术语 + ADR 20260723 裁定 1，实装理由见本测试块各断言注释）：
    // - **身份判据 = 同点递归，方向退役**（T5a）：同一 x 可在不同级别分别为顶/底（各级自为真）；
    //   脚身份 = (极值价, 组锚 a*) 两元；买卖标签留交易层（event.side/admit 裁决/Xzd 回退不动）。
    // - **级别范围**：链级 = BSP 账本级（book level）；链区间 = [L0, 链顶] 对全候选一致
    //   （「闭合到 L0」字面，候选本级是普通链级——链即身份，不靠候选自报 lvl 猜方向）；
    //   证书查询键级 = 账本级 + 1（`event_bsp_book_level` T1 移位，与旧固定桥 ℓ=lvl+1 同移）。
    // - **链顶** = x 为拐点的最高账本级（**不问分型类型**），由恰好存在经 T2 层索引层间联动
    //   给出（非塔顶移动窗口、非固定 ℓ+1）；a* 自本级层 `source_index == c.source_index`
    //   条目解析（禁第二查法：gate 不自行调 merged_group_anchor）。
    // - **缺/断极性**（词汇表：缺环即拒=lvl+1 单级过证不算；断环即拒=高级有证而中间断不算）：
    //   非闭合级 g 上方区间 (g, 链顶] 内有闭合级 ⟹ 断，否则缺；候选首位归因 = 最高非闭合级。
    //   T5a 复核：判据不涉及方向分量，逐字保留。
    // - **三态**：全链闭合 = Pass（nest_pass）；≥1 闭合级但有缺/断 = Reject（nest_n_delta_false，
    //   对标旧 Some(pass=false) 语义位）；零闭合级 / 存在性全无 / 锚不可解 = NoChain（Xzd 回退
    //   逐字不动，含因果守卫全剔 ⟹ NoChain——#112-T2 现语义保留）。

    /// T3 夹具供给：x=55 底分型 @100（极值价）；合并组 [50,60) ⟹ x 的组锚 a*=50。
    fn t3_supplies() -> (
        Vec<super::super::super::types::Fractal>,
        Vec<super::super::super::types::Bar>,
    ) {
        use super::super::super::types::{Bar, Fractal, FractalKind};
        let fractals = vec![Fractal { kind: FractalKind::Bottom, source_index: 55, timestamp: 55, price: 100 }];
        let merged = [50usize, 60]
            .into_iter()
            .map(|i| Bar { source_index: i, timestamp: i as i64, open: 0, high: 0, low: 0, close: 0, volume: 1, untradable: false })
            .collect();
        (fractals, merged)
    }

    /// T3 夹具：携两元锚供给的 gate（`for_test` 的供给版——链查询的极值价/组锚单一来源）。
    fn t3_gate_with_certs(
        cert_event_levels: &[u32],
        judge_at: usize,
        cls: &classifier::Classification,
    ) -> NestChainGate {
        use super::super::super::types::Side;
        let (fractals, merged) = t3_supplies();
        let mut gate = NestChainGate::for_test_with_supplies(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            std::rc::Rc::new(fractals),
            std::rc::Rc::new(merged),
        );
        for &el in cert_event_levels {
            let ev = nc_event(el, Side::Long, (30, 50), 50, judge_at, true);
            gate.absorb_exts(vec![nc_ext_anchored(ev, 100, 50)]);
        }
        gate.sync_index(cls);
        gate
    }

    /// T3 夹具：带投影层的多级分类。`specs` = (book_level, 是否置 x=55 存在点)。每级 book
    /// 恒置 src=40 buy1 背书点（owner 中枢 start=20，供证书装配 CWindow 窗口 [30,50] 内）；
    /// 存在位另置 src=55 buy1 点（供层登记锚 (@100) → (55, 组锚 50)，T5a 方向不进键）；投影层经
    /// `LevelProjectionLayer::from_level` 以 T3 供给 stamping（与生产 stamping 同函数）。
    fn t3_chain_classification(specs: &[(usize, bool)]) -> classifier::Classification {
        use super::super::super::classifier::bsp::BspPoint;
        use super::super::super::classifier::LevelState;
        use super::super::super::types::{BspBits, Center};
        let (fractals, merged) = t3_supplies();
        let buy1 = BspBits { buy1: true, ..Default::default() };
        let mk_pt = |src: usize| BspPoint {
            source_index: src,
            level_origin: 0,
            bits: buy1,
            pivot_low: 0,
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 20, end_index: 0 })),
            struct_break_dir: None,
            force: None,
        };
        let n_levels = specs.iter().map(|&(l, _)| l + 1).max().unwrap_or(1);
        let levels: Vec<LevelState> = (0..n_levels)
            .map(|book| {
                let existence = specs.iter().any(|&(l, e)| l == book && e);
                let mut pts = vec![mk_pt(40)];
                if existence {
                    pts.push(mk_pt(55));
                }
                let bsp = std::rc::Rc::new(pts);
                let layer = classifier::projection::LevelProjectionLayer::from_level(
                    book as u32, &bsp, &fractals, &merged,
                );
                // #218 面 B（机械改写归因：判同机制换带判同）：账本层补 B 中枢（带 (0,0)，
                // start=20 = 事件 b_center_start 查找键）——点 center 带 == B 带 ⟹ 判等。
                LevelState {
                    bsp,
                    centers: std::rc::Rc::new(vec![Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 20, end_index: 0 }]),
                    level_projection: Some(layer),
                    ..Default::default()
                }
            })
            .collect();
        classifier::Classification { levels, ..Default::default() }
    }

    /// T3-1（链顶经层间联动给出，非固定 ℓ+1）：x 存在于 L0/L2（L1 层无该脚）⟹ 链顶 = L2
    /// （固定 ℓ+1 只会给 L1）；L1 = MissingExistence（恰好存在断裂，缺环底质）；L2 闭合
    /// ⟹ L1 之 gap 位置极性 = 断（高级有证而中间断）。
    #[test]
    fn t3_chain_top_from_layer_linkage_not_fixed_plus_one() {
        let cls = t3_chain_classification(&[(0, true), (1, false), (2, true)]);
        let gate = t3_gate_with_certs(&[1, 3], 100, &cls);
        let c = ng_candidate(super::super::super::strategy::voice::VoiceSide::Long, Default::default(), 55);
        let probe = gate.chain_lookup(&c, 100, &cls);
        assert_eq!(probe.chain_top, Some(2), "链顶 = 层间联动给出的最高存在级 L2（非固定 lvl+1=1）");
        assert_eq!(probe.levels.len(), 3, "链区间 = [L0, 链顶]（闭合到 L0 字面）");
        assert_eq!(probe.levels[0].event_level, 1, "证书键级 = 账本级 + 1（T1 移位）");
        assert_eq!(probe.levels[2].status, ChainLevelStatus::Closed, "L2 闭合（证书键 ℓ=3 命中）");
        assert_eq!(
            probe.levels[1].status,
            ChainLevelStatus::MissingExistence,
            "L1 无该脚存在 ⟹ 缺环底质 MissingExistence"
        );
        assert_eq!(probe.levels[1].gap, Some(ChainGapKind::Broken), "上方 L2 有证 ⟹ 断（高级有证而中间断）");
        assert_eq!(probe.verdict, ChainVerdict::Reject, "缺/断即拒");
        assert_eq!(probe.first_gap, Some((1, ChainGapKind::Broken)), "首位归因 = 最高非闭合级 L1 断");
        assert_eq!(probe.closed_down_to, Some(2), "自链顶向下连续闭合到 L2");
    }

    /// T3-2（缺环拒精确区分）：存在性全级但仅 L0 有证 ⟹ 链顶 L2 自身查无证书（缺）——
    /// 「lvl+1 单级过证不算」：L0 独过而上方缺环 ⟹ Reject，首位归因 = 链顶缺。
    #[test]
    fn t3_chain_missing_link_reject_is_missing() {
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::{BspBits};
        let cls = t3_chain_classification(&[(0, true), (1, true), (2, true)]);
        let gate = t3_gate_with_certs(&[1], 100, &cls); // 仅事件级 ℓ=1（账本级 L0）有证
        let tower = ng_tower();
        let hist: Vec<f64> = (0..20).map(|_| 1.0).collect();
        let buy1 = BspBits { buy1: true, ..Default::default() };
        let c = ng_candidate(VoiceSide::Long, buy1, 55);
        let probe = gate.chain_lookup(&c, 100, &cls);
        assert_eq!(probe.levels[0].status, ChainLevelStatus::Closed);
        assert_eq!(probe.levels[2].status, ChainLevelStatus::MissingCert, "链顶键域查无证书 ⟹ 缺");
        assert_eq!(probe.levels[2].gap, Some(ChainGapKind::Missing), "上方无闭合级 ⟹ 缺（非断）");
        assert_eq!(probe.verdict, ChainVerdict::Reject);
        assert_eq!(probe.first_gap, Some((2, ChainGapKind::Missing)), "缺环拒：首位 = 链顶 L2 缺");
        assert_eq!(probe.closed_down_to, None, "链顶未闭合 ⟹ 无连续闭合前缀");
        let (admit, channel, obs) = gate.admit(&tower, &c, &hist, 100, &cls);
        assert_eq!((admit, channel), (false, "nest_n_delta_false"), "缺环即拒 = nest 拒（语义位不变）");
        assert_eq!(obs.chain_top, Some(2));
        assert_eq!(obs.chain_verdict, ChainVerdict::Reject);
        assert_eq!(obs.chain_first_gap, Some((2, ChainGapKind::Missing)));
        let mut stats = NestGateStats::default();
        stats.observe(admit, channel, obs);
        assert_eq!(stats.chain_reject_missing, 1, "缺环拒计数");
        assert_eq!(stats.chain_reject_broken, 0);
    }

    /// T3-3（断环拒精确区分）：L0/L2 闭合、L1 键域查无 ⟹ L1 上方有闭合级 ⟹ 断——
    /// 「高级有证而中间断不算」：Reject，首位归因 = L1 断。
    #[test]
    fn t3_chain_broken_link_reject_is_broken() {
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::{BspBits};
        let cls = t3_chain_classification(&[(0, true), (1, true), (2, true)]);
        let gate = t3_gate_with_certs(&[1, 3], 100, &cls); // ℓ=2（账本级 L1）无证
        let tower = ng_tower();
        let hist: Vec<f64> = (0..20).map(|_| 1.0).collect();
        let buy1 = BspBits { buy1: true, ..Default::default() };
        let c = ng_candidate(VoiceSide::Long, buy1, 55);
        let probe = gate.chain_lookup(&c, 100, &cls);
        assert_eq!(probe.levels[1].status, ChainLevelStatus::MissingCert);
        assert_eq!(probe.levels[1].gap, Some(ChainGapKind::Broken), "上方 L2 闭合 ⟹ 断");
        assert_eq!(probe.first_gap, Some((1, ChainGapKind::Broken)), "断环拒：首位 = L1 断");
        assert_eq!(probe.closed_down_to, Some(2), "链顶向下连续闭合前缀 = [L2]");
        let (admit, channel, obs) = gate.admit(&tower, &c, &hist, 100, &cls);
        assert_eq!((admit, channel), (false, "nest_n_delta_false"), "断环即拒");
        let mut stats = NestGateStats::default();
        stats.observe(admit, channel, obs);
        assert_eq!(stats.chain_reject_broken, 1, "断环拒计数");
        assert_eq!(stats.chain_reject_missing, 0);
    }

    /// T3-4（全链闭合通过 + 闭合到 L0）：L0/L1/L2 逐级有证且过 ⟹ Pass，closed_down_to=L0，
    /// admit 消费链结果 = nest_pass；谱系逐级别落账（链即身份）。
    #[test]
    fn t3_chain_full_closure_pass_consumed_by_admit() {
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::{BspBits};
        let cls = t3_chain_classification(&[(0, true), (1, true), (2, true)]);
        let gate = t3_gate_with_certs(&[1, 2, 3], 100, &cls);
        let tower = ng_tower();
        let hist: Vec<f64> = (0..20).map(|_| 1.0).collect();
        let buy1 = BspBits { buy1: true, ..Default::default() };
        let c = ng_candidate(VoiceSide::Long, buy1, 55);
        let (admit, channel, obs) = gate.admit(&tower, &c, &hist, 100, &cls);
        assert_eq!((admit, channel), (true, "nest_pass"), "全链闭合 ⟹ nest_pass");
        assert_eq!(obs.chain_verdict, ChainVerdict::Pass);
        assert_eq!(obs.chain_top, Some(2));
        assert_eq!(obs.chain_closed_down_to, Some(0), "连续闭合到 L0（链区间下限）");
        assert_eq!(obs.chain_first_gap, None, "全闭合无缺断");
        assert_eq!(obs.chain_genealogy.len(), 3, "完整谱系（逐级落账）");
        for (i, g) in obs.chain_genealogy.iter().enumerate() {
            assert_eq!(g.level, i as u32);
            assert_eq!(g.event_level, i as u32 + 1);
            assert_eq!(g.status, ChainLevelStatus::Closed);
            assert_eq!(g.n_certs, 1);
            assert_eq!(g.n_causal_clean, 1);
        }
        assert_eq!(obs.chain_price, Some(100), "极值价落账（T1 供给线）");
        assert_eq!(obs.chain_anchor, Some(50), "组锚 a* 落账（T2 本级层解析）");
        let mut stats = NestGateStats::default();
        stats.observe(admit, channel, obs);
        assert_eq!(stats.chain_pass, 1, "链确认计数（方向守卫读数）");
        assert_eq!(stats.chain_top_dist.get(&2), Some(&1), "链顶级别分布落账");
    }

    /// T3-5（闭合到 L0 字面）：L0 缺证而 L1/L2 全闭 ⟹ 仍拒（上方闭合不补 L0 之缺）；
    /// L0 上方有闭合 ⟹ 其 gap 极性 = 断。
    #[test]
    fn t3_chain_l0_gap_rejects_despite_upper_closure() {
        let cls = t3_chain_classification(&[(0, true), (1, true), (2, true)]);
        let gate = t3_gate_with_certs(&[2, 3], 100, &cls); // ℓ=1（账本级 L0）无证
        let c = ng_candidate(super::super::super::strategy::voice::VoiceSide::Long, Default::default(), 55);
        let probe = gate.chain_lookup(&c, 100, &cls);
        assert_eq!(probe.levels[0].status, ChainLevelStatus::MissingCert, "L0 查无证书");
        assert_eq!(probe.verdict, ChainVerdict::Reject, "L0 未闭合 ⟹ 拒（闭合到 L0 字面）");
        assert_eq!(probe.first_gap, Some((0, ChainGapKind::Broken)), "L0 上方有闭合 ⟹ 断");
    }

    /// T3-6（NoChain → Xzd 回退逐字不动）：(a) x 处无分型（price 不可解）⟹ NoChain；
    /// (b) 存在性全级但全区间零证书 ⟹ NoChain；两路均走既有 Xzd 分支（旧臂 None ⟹
    /// cert_none；旧臂 nest_pass ⟹ 重走 build_xzd_fallback，xzd_fallback 标记）。
    #[test]
    fn t3_chain_no_chain_falls_back_to_xzd_verbatim() {
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::{BspBits, Side};
        let tower = ng_tower();
        let hist: Vec<f64> = (0..20).map(|_| 1.0).collect();
        // (a) 存在性全级但零证书 ⟹ NoChain；旧臂在 src=55 无执行段 ⟹ cert_none（回退逐字）。
        let cls = t3_chain_classification(&[(0, true), (1, true), (2, true)]);
        let gate = t3_gate_with_certs(&[], 100, &cls);
        let buy1 = BspBits { buy1: true, ..Default::default() };
        let c = ng_candidate(VoiceSide::Long, buy1, 55);
        let probe = gate.chain_lookup(&c, 100, &cls);
        assert_eq!(probe.verdict, ChainVerdict::NoChain, "零闭合级 ⟹ NoChain");
        let (admit, channel, obs) = gate.admit(&tower, &c, &hist, 100, &cls);
        assert_eq!((admit, channel), (false, "cert_none"), "NoChain → 既有回退（旧臂 None ⟹ cert_none）");
        assert!(!obs.xzd_fallback && !obs.reused_old_xzd);
        // (b) x=19 无分型（price 不可解）⟹ NoChain；旧臂 lvl0 Type3 免门 = nest_pass ⟹
        // 重走 build_xzd_fallback 三分支（#94 择 (b) 逐字保留）。
        let buy3 = BspBits { buy3: true, ..Default::default() };
        let cls_b3 = ng_classification(buy3);
        let c_b3 = ng_candidate(VoiceSide::Long, buy3, 19);
        let (admit, channel, obs) = gate.admit(&tower, &c_b3, &hist, 19, &cls_b3);
        assert!(matches!(channel, "xzd_pass" | "xzd_gate_fail"), "NoChain → Xzd 回退通道，实际={channel}");
        assert!(obs.xzd_fallback, "旧臂 nest 通道 ⟹ 重走单一来源补评");
        let expected = super::super::econ_positive::build_xzd_fallback(
            &tower, 0, 19, Side::Long, &buy3, 19, &cls_b3.levels[0].bsp, &[], &[],
        )
        .map(|ev| ev.gate_pass())
        .unwrap_or(false);
        assert_eq!(admit, expected, "回退判定 = build_xzd_fallback 单一来源（现语义逐字）");
        let mut stats = NestGateStats::default();
        stats.observe(admit, channel, obs);
        assert_eq!(stats.chain_none, 1, "NoChain 计数");
    }

    /// T3-7（因果守卫语义沿用）：唯一证书 judge_at=100 越过 anchor=19 ⟹ 整证剔除
    /// （MissingCausal，缺环底质）；全区间零因果干净证书 ⟹ NoChain → Xzd（#112-T2 现语义保留）。
    #[test]
    fn t3_chain_causal_guard_missing_causal_then_no_chain() {
        let cls = t3_chain_classification(&[(0, true)]);
        let gate = t3_gate_with_certs(&[1], 100, &cls); // judge_at=100
        let c = ng_candidate(super::super::super::strategy::voice::VoiceSide::Long, Default::default(), 55);
        let probe = gate.chain_lookup(&c, 19, &cls);
        assert_eq!(
            probe.levels[0].status,
            ChainLevelStatus::MissingCausal,
            "唯一证书越界 ⟹ 整证剔除（缺环底质 MissingCausal）"
        );
        assert_eq!(probe.levels[0].n_certs, 1, "键域有证（剔除前）");
        assert_eq!(probe.levels[0].n_causal_clean, 0, "因果守卫全剔");
        assert_eq!(probe.verdict, ChainVerdict::NoChain, "零闭合级 ⟹ NoChain → Xzd（现语义不动）");
        // anchor=100 ⟹ 前缀内，正常闭合。
        let probe = gate.chain_lookup(&c, 100, &cls);
        assert_eq!(probe.levels[0].status, ChainLevelStatus::Closed);
        assert_eq!(probe.verdict, ChainVerdict::Pass);
    }

    /// T3-8（方向一致性见证）已随 T5a (#207) 退役删除——「方向分歧」在 ADR 20260723
    /// 裁定 1 下是非概念：同脚同价的层异型登记 = 同点跨型（各级自为真，A 类 235 例
    /// 语义真相回归，见 `t5a_same_point_cross_type_chain_pass`）；异侧键域有证 = 同键
    /// 合流（见 `t5a_key_domain_merges_directions`）。DirWitness 装置已死透（类型/
    /// 统计字段/dump 列/NEST_GATE_T3 列全删，编译期保证 + grep 无残留）。

    // ───────────── T5a（#207 方向退役，ADR 20260723 裁定 1：身份判据 = 同点递归）─────────────

    /// T5a 夹具：同点跨型多级分类——`specs` = (book_level, x=55 以何种 bits 登记；None = 该级无此脚)。
    /// 各级分型类型是各级自己的结构事实（ADR：同一 x 可在 L0 为底、L1 为顶，各级自为真）。
    fn t5a_cross_type_classification(
        specs: &[Option<super::super::super::types::BspBits>],
    ) -> classifier::Classification {
        use super::super::super::classifier::bsp::BspPoint;
        use super::super::super::classifier::LevelState;
        use super::super::super::types::{BspBits, Center};
        let (fractals, merged) = t3_supplies();
        let buy1 = BspBits { buy1: true, ..Default::default() };
        let mk_pt = |src: usize, bits: BspBits| BspPoint {
            source_index: src,
            level_origin: 0,
            bits,
            pivot_low: 0,
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 20, end_index: 0 })),
            struct_break_dir: None,
            force: None,
        };
        let levels: Vec<LevelState> = specs
            .iter()
            .enumerate()
            .map(|(book, foot)| {
                let mut pts = vec![mk_pt(40, buy1)];
                if let Some(bits) = foot {
                    pts.push(mk_pt(55, *bits));
                }
                let bsp = std::rc::Rc::new(pts);
                let layer = classifier::projection::LevelProjectionLayer::from_level(
                    book as u32, &bsp, &fractals, &merged,
                );
                // #218 面 B（机械改写归因：判同机制换带判同）：账本层补 B 中枢（带 (0,0)，
                // start=20 = 事件 b_center_start 查找键）——点 center 带 == B 带 ⟹ 判等。
                LevelState {
                    bsp,
                    centers: std::rc::Rc::new(vec![Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 20, end_index: 0 }]),
                    level_projection: Some(layer),
                    ..Default::default()
                }
            })
            .collect();
        classifier::Classification { levels, ..Default::default() }
    }

    /// T5a-1（同点跨型链确认成立，#206 A 类 235 例语义真相回归）：同一 x=55 在 L0 层以
    /// buy1（底）登记、L1 层以 sell1（顶）登记——方向退役后身份只剩同点递归：两级
    /// 恰好存在均成立（不问分型类型），证书逐级闭合 ⟹ **Pass**（旧同向过滤下 L1
    /// missing_existence ⟹ Reject，本测试在旧语义下为红）。
    #[test]
    fn t5a_same_point_cross_type_chain_pass() {
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::{BspBits};
        let buy1 = BspBits { buy1: true, ..Default::default() };
        let sell1 = BspBits { sell1: true, ..Default::default() };
        // L0：x=55 底（buy1）；L1：x=55 顶（sell1）——同点跨型，各级自为真。
        let cls = t5a_cross_type_classification(&[Some(buy1), Some(sell1)]);
        // 证书：事件级 ℓ=1（账本级 L0）与 ℓ=2（账本级 L1），同锚 (极值价 100, 组锚 50)。
        let gate = t3_gate_with_certs(&[1, 2], 100, &cls);
        let c = ng_candidate(VoiceSide::Long, buy1, 55);
        let probe = gate.chain_lookup(&c, 100, &cls);
        assert_eq!(probe.chain_top, Some(1), "链顶 = 以 x 为拐点的最高账本级（不问分型类型）");
        assert_eq!(probe.levels[0].status, ChainLevelStatus::Closed, "L0 闭合");
        assert_eq!(
            probe.levels[1].status,
            ChainLevelStatus::Closed,
            "L1 跨型存在 + 证书闭合——同向过滤退役后不再 missing_existence"
        );
        assert_eq!(probe.verdict, ChainVerdict::Pass, "同点跨型全链闭合 ⟹ Pass（A 类语义真相）");
        assert_eq!(probe.closed_down_to, Some(0), "闭合到 L0");
        let tower = ng_tower();
        let hist: Vec<f64> = (0..20).map(|_| 1.0).collect();
        let (admit, channel, obs) = gate.admit(&tower, &c, &hist, 100, &cls);
        assert_eq!((admit, channel), (true, "nest_pass"), "同点跨型链确认被 admit 消费");
        assert_eq!(obs.chain_verdict, ChainVerdict::Pass);
    }

    /// T5a-2（键域去方向位：登记合流 + 查询不携带方向）：同价同组锚的 Long/Short 事件
    /// 登记进**同一键** (ℓ, 极值价, 组锚)；异侧登记的证书对本向候选的链闭合同样有效
    /// （方向不参与身份）；`chain_key_hint` 对层异型登记的脚可解析（旧语义 false ⟹ 红）。
    #[test]
    fn t5a_key_domain_merges_directions() {
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::{BspBits, Side};
        let buy1 = BspBits { buy1: true, ..Default::default() };
        let sell1 = BspBits { sell1: true, ..Default::default() };
        let cls = t5a_cross_type_classification(&[Some(buy1)]);
        let (fractals, merged) = t3_supplies();
        let mut gate = NestChainGate::for_test_with_supplies(
            Vec::new(), Vec::new(), Vec::new(),
            std::rc::Rc::new(fractals), std::rc::Rc::new(merged),
        );
        // 同价同锚的 Long/Short 两事件（不同 interval_b ⟹ 不同身份）。
        let ev_long = nc_event(1, Side::Long, (30, 50), 50, 100, true);
        let ev_short = nc_event(1, Side::Short, (32, 50), 50, 100, true);
        gate.absorb_exts(vec![
            nc_ext_anchored(ev_long, 100, 50),
            nc_ext_anchored(ev_short, 100, 50),
        ]);
        assert_eq!(
            gate.by_triple_anchor.keys().count(),
            1,
            "方向退役：同价同组锚跨向事件登记进同一键 (ℓ, 极值价, 组锚)"
        );
        assert_eq!(
            gate.by_triple_anchor.values().next().map(Vec::len),
            Some(2),
            "同键身份集 = 两方向事件合流"
        );
        gate.sync_index(&cls);
        let c = ng_candidate(VoiceSide::Long, buy1, 55);
        // 查询不携带方向：Long 候选读到 Short 侧登记的证书同样闭合（身份层无方向）。
        let probe = gate.chain_lookup(&c, 100, &cls);
        assert_eq!(probe.levels[0].n_certs, 2, "键域身份集 = 双向合流（不滤方向）");
        assert_eq!(probe.verdict, ChainVerdict::Pass);
        // 异型脚可解析：候选 Long 而层仅 sell1 登记 x=55 —— 锚解析/hint 不再被方向阻断。
        let cls_opp = t5a_cross_type_classification(&[Some(sell1)]);
        assert!(
            gate.chain_key_hint(&c, &cls_opp),
            "层异型登记下脚仍可解析 + 键域有证 ⟹ hint=true（旧同向口径 false ⟹ 红）"
        );
    }

    /// T3-9（并门，#168 裁定 3）：层载由链路径单一驱动——链活 ⟹ 层必载（含索引），
    /// 链死不载（零开销红线不死）；stamping 跟随派生位（机制位仍 config，生产只经派生写入）。
    #[test]
    fn t3_chain_gate_merged_drives_layer_load() {
        let config = ThetaConfig::default();
        // 链死 ⟹ 派生位 = 原配置（层不载）。
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(Some(false)));
        let derived = super::super::admission::chain_driven_level_projection(&config);
        assert!(!derived.level_projection.enabled, "链死 ⟹ 层不载（零开销红线不死）");
        assert!(matches!(derived, std::borrow::Cow::Borrowed(_)), "链死 ⟹ 零拷贝借用");
        // 链活 ⟹ 层必载。
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(Some(true)));
        let derived = super::super::admission::chain_driven_level_projection(&config);
        assert!(derived.level_projection.enabled, "链活 ⟹ 层必载（索引成本即链判成本）");
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(None));
        // stamping 跟随机制位：开 ⟹ 每级 Some(layer)；关（默认）⟹ None（八处 None 构造点不动）。
        let bars: Vec<super::super::super::types::Bar> = (0..40)
            .map(|i| super::super::super::types::Bar {
                source_index: i,
                timestamp: i as i64,
                open: 100,
                high: if i % 2 == 0 { 112 } else { 100 },
                low: if i % 2 == 0 { 100 } else { 88 },
                close: if i % 2 == 0 { 110 } else { 90 },
                volume: 1,
                untradable: false,
            })
            .collect();
        let l0 = parser::parse_layer(&bars, &config);
        let cls_off = classifier::classify_with_tower(&l0, &config).0;
        assert!(
            cls_off.levels.iter().all(|ls| ls.level_projection.is_none()),
            "机制位关（默认）⟹ stamping 不构造层（零开销 bit-exact 锁）"
        );
        let mut config_on = config.clone();
        config_on.level_projection = classifier::projection::LevelProjectionConfig::for_chain(true);
        let cls_on = classifier::classify_with_tower(&l0, &config_on).0;
        assert!(
            cls_on.levels.iter().all(|ls| ls.level_projection.is_some()),
            "机制位开 ⟹ 每级 stamping 必载层（链活 ⟹ 层必载形态）"
        );
    }

    /// #75-T5（增量喂法跳过）：tower Rc 未变 ⟹ 不重派生（derivations 不增）；下级 Rc 变化
    /// ⟹ 该级重派生。真实 provider 夹具（nest_index.rs real_provider 同型）。
    #[test]
    fn nest_chain_sync_events_incremental_skip() {
        use super::super::super::classifier::center::UnitRange;
        use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove};
        use super::super::super::types::{Center, Direction};
        use Direction::{Down, Up};
        fn unit(start: usize, dir: Direction, lo: i64, hi: i64, ordinal: u64) -> LeveledMove {
            LeveledMove::from_unit(
                &UnitRange { start_index: start, end_index: start + 9, direction: dir, lo, hi },
                ElementId { level: 0, ordinal },
            )
        }
        let lower = vec![
            unit(0, Up, 90, 110, 0),
            unit(10, Down, 95, 115, 1),
            unit(20, Up, 98, 112, 2),
            unit(30, Down, 96, 116, 3),
            unit(40, Up, 130, 145, 4),
            unit(50, Down, 132, 148, 5),
            unit(60, Up, 135, 150, 6),
            unit(70, Down, 125, 140, 7),
            unit(80, Up, 155, 170, 8),
            unit(90, Down, 150, 165, 9),
            unit(100, Up, 160, 175, 10),
            unit(110, Down, 140, 155, 11),
            unit(120, Up, 180, 190, 12),
            unit(130, Down, 170, 195, 13),
        ];
        let windows = vec![
            LeveledMove::compose(
                &lower[0..4],
                Center { zd: 98, zg: 110, dd: 90, gg: 116, start_index: 0, end_index: 39 },
                1,
                ElementId { level: 1, ordinal: 0 },
            ),
            LeveledMove::compose(
                &lower[4..8],
                Center { zd: 135, zg: 140, dd: 125, gg: 150, start_index: 40, end_index: 79 },
                1,
                ElementId { level: 1, ordinal: 1 },
            ),
            LeveledMove::compose(
                &lower[8..12],
                Center { zd: 160, zg: 165, dd: 140, gg: 175, start_index: 80, end_index: 119 },
                1,
                ElementId { level: 1, ordinal: 2 },
            ),
        ];
        let mut hist = vec![0.0; 140];
        hist[80..110].fill(2.0);
        hist[120..140].fill(0.1);
        let mut dif = vec![0.0; 140];
        dif[80..=100].fill(-5.0);
        dif[101..140].fill(1.0);
        let close_src: Vec<usize> = (0..140).collect();
        let mut gate = NestChainGate::for_test(hist, dif, close_src);
        let tower = vec![std::rc::Rc::new(lower.clone()), std::rc::Rc::new(windows)];
        // #93 步骤 0：confirmed_lens = [len0, len1]（合成塔全确认 = 模拟无 frontier 重写的稳定塔）。
        let cl = vec![tower[0].len(), tower[1].len()];
        gate.sync_events(&tower, &cl, 139);
        assert_eq!(gate.n_derivations, 1, "首见塔 ⟹ 派生一次");
        assert_eq!(gate.n_provider_errors, 0, "夹具应零 provider 错误（管道端到端跑通）");
        // 注：本合成 3 中枢夹具不经 decompose 保证产出确认事件（nest_index 同型夹具的确认
        // 事件依赖手工 MoveBlock 序列；本路径块序列由 decompose 自治）——事件产出的端到端
        // 实证由门开冒烟（真实 BTC 截断）承担；本测试锁定增量跳过与幂等纪律。
        let absorbed: usize = gate.events_by_level.iter().map(Vec::len).sum();
        assert!(
            gate.events_by_level.iter().flatten().all(|e| e.divergence_confirmed),
            "账本只收确认事件"
        );
        // #93：值指纹不变 ⟹ 跳过（增量喂法核心——禁每 bar 从零重建；旧 Rc ptr_eq 自溃已消除）。
        gate.sync_events(&tower, &cl, 139);
        assert_eq!(gate.n_derivations, 1, "tower 值不变 ⟹ 不重派生");
        assert_eq!(
            gate.events_by_level.iter().map(Vec::len).sum::<usize>(),
            absorbed,
            "重派生跳过 ⟹ 账本零追加"
        );
        // #93 步骤 0 核心验证：新 Rc 但同内容（旧 ptr_eq 必报变；值指纹正确判不变 ⟹ 跳过）。
        let tower2 = vec![std::rc::Rc::new(lower), std::rc::Rc::clone(&tower[1])];
        gate.sync_events(&tower2, &cl, 139);
        assert_eq!(gate.n_derivations, 1, "tower[0] 新 Rc 同内容 ⟹ 值指纹跳过（自溃消除）");
        assert_eq!(
            gate.events_by_level.iter().map(Vec::len).sum::<usize>(),
            absorbed,
            "跳过 ⟹ 账本零追加"
        );
    }

    /// #93-T4：值指纹单测——内容不变跳过；tail 单元素变 ⟹ 重派生；水线回退 ⟹ 保守全量。
    #[test]
    fn nest_chain_fingerprint_value_based() {
        use super::super::super::classifier::center::UnitRange;
        use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove};
        use super::super::super::types::{Center, Direction};
        use Direction::{Down, Up};
        fn unit(start: usize, dir: Direction, lo: i64, hi: i64, ordinal: u64) -> LeveledMove {
            LeveledMove::from_unit(
                &UnitRange { start_index: start, end_index: start + 9, direction: dir, lo, hi },
                ElementId { level: 0, ordinal },
            )
        }
        let lower = vec![
            unit(0, Up, 90, 110, 0),
            unit(10, Down, 95, 115, 1),
            unit(20, Up, 98, 112, 2),
            unit(30, Down, 96, 116, 3),
        ];
        let windows = vec![LeveledMove::compose(
            &lower[0..4],
            Center { zd: 98, zg: 110, dd: 90, gg: 116, start_index: 0, end_index: 39 },
            1,
            ElementId { level: 1, ordinal: 0 },
        )];
        let hist = vec![0.0; 50];
        let dif = vec![0.0; 50];
        let close_src: Vec<usize> = (0..50).collect();
        let mut gate = NestChainGate::for_test(hist, dif, close_src);
        let tower = vec![std::rc::Rc::new(lower.clone()), std::rc::Rc::new(windows.clone())];
        // 首次：w_lower=3（前3段证书保稳定，index=3 是未确认尾段），w_self=1。
        let cl = vec![3, 1];
        gate.sync_events(&tower, &cl, 49);
        assert_eq!(gate.n_derivations, 1, "首见塔 ⟹ 派生一次");
        // 同塔同水线再喂 ⟹ 跳过。
        gate.sync_events(&tower, &cl, 49);
        assert_eq!(gate.n_derivations, 1, "值指纹不变 ⟹ 跳过");
        // 尾段单元素变（lower 尾段 lo 改）⟹ 重派生。
        let lower2 = {
            let mut l = lower.clone();
            let mut u = UnitRange { start_index: 30, end_index: 39, direction: Down, lo: 96, hi: 116 };
            u.lo = 95; // 改尾段值（index=3 在 [w=3..] 尾段内）
            l[3] = LeveledMove::from_unit(&u, ElementId { level: 0, ordinal: 3 });
            l
        };
        let tower2 = vec![std::rc::Rc::new(lower2), std::rc::Rc::new(windows.clone())];
        gate.sync_events(&tower2, &cl, 49);
        assert_eq!(gate.n_derivations, 2, "tail 单元素变 ⟹ 重派生");
        // 水线回退 ⟹ 保守全量重派生（恒正确退化）。
        let cl_regress = vec![2, 1]; // w_lower 从 3 退到 2
        gate.sync_events(&tower2, &cl_regress, 49);
        assert_eq!(gate.n_derivations, 3, "水线回退 ⟹ 保守全量重派生");
    }

    // ───────────── #76（SPEC #73 A 线第三票：出场门真链切换）─────────────

    /// #76 夹具：反向开仓决策（出场门消费对象），显式 root_side/bits/level/signal_index。
    fn xd_reverse_decision(
        root_side: VoiceSide,
        bits: super::super::super::types::BspBits,
        level: u32,
        signal_index: usize,
    ) -> VoiceDecision {
        use super::super::super::strategy::risk::StopInput;
        use super::super::super::types::Center;
        VoiceDecision {
            depth: 0,
            root_side,
            exit: false,
            enter_ok: true,
            bsp: bits,
            signal_index,
            stop_in: StopInput {
                pivot_low: 900,
                pivot_high: 1100,
                center: Center { zd: 950, zg: 1050, dd: 940, gg: 1090, start_index: 0, end_index: 5 },
            },
            entry: 1000,
            cost_per_unit: 0.0,
            level,
        }
    }

    /// T5b (#208) 出场夹具：sell1 终端背书版链分类（`t3_chain_classification` 的 Short
    /// 对偶——sell1 点既作 src=40 终端背书（owner 中枢 start=20）又作 src=55 存在位；
    /// 旧桥同向键（ℓ=1, 离开段终点=55, is_long=false）可命中 ⟹ 新旧语义同夹具对照）。
    fn xd_chain_classification(specs: &[(usize, bool)]) -> classifier::Classification {
        use super::super::super::classifier::bsp::BspPoint;
        use super::super::super::classifier::LevelState;
        use super::super::super::types::{BspBits, Center};
        let (fractals, merged) = t3_supplies();
        let sell1 = BspBits { sell1: true, ..Default::default() };
        let mk_pt = |src: usize| BspPoint {
            source_index: src,
            level_origin: 0,
            bits: sell1,
            pivot_low: 0,
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 20, end_index: 0 })),
            struct_break_dir: None,
            force: None,
        };
        let n_levels = specs.iter().map(|&(l, _)| l + 1).max().unwrap_or(1);
        let levels: Vec<LevelState> = (0..n_levels)
            .map(|book| {
                let existence = specs.iter().any(|&(l, e)| l == book && e);
                let mut pts = vec![mk_pt(40)];
                if existence {
                    pts.push(mk_pt(55));
                }
                let bsp = std::rc::Rc::new(pts);
                let layer = classifier::projection::LevelProjectionLayer::from_level(
                    book as u32, &bsp, &fractals, &merged,
                );
                // #218 面 B（机械改写归因：判同机制换带判同）：账本层补 B 中枢（带 (0,0)，
                // start=20 = 事件 b_center_start 查找键）——点 center 带 == B 带 ⟹ 判等。
                LevelState {
                    bsp,
                    centers: std::rc::Rc::new(vec![Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 20, end_index: 0 }]),
                    level_projection: Some(layer),
                    ..Default::default()
                }
            })
            .collect();
        classifier::Classification { levels, ..Default::default() }
    }

    /// T5b (#208) 出场夹具：Short 证书 gate（`t3_gate_with_certs` 的 Short 对偶——
    /// 锚 sidecar 同为 (100, 50)，事件方向 = Short；sell1 背书 ⟹ 基例门产证）。
    fn xd_gate_with_short_certs(
        cert_event_levels: &[u32],
        judge_at: usize,
        cls: &classifier::Classification,
    ) -> NestChainGate {
        use super::super::super::types::Side;
        let (fractals, merged) = t3_supplies();
        let mut gate = NestChainGate::for_test_with_supplies(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            std::rc::Rc::new(fractals),
            std::rc::Rc::new(merged),
        );
        for &el in cert_event_levels {
            let ev = nc_event(el, Side::Short, (30, 50), 50, judge_at, true);
            gate.absorb_exts(vec![nc_ext_anchored(ev, 100, 50)]);
        }
        gate.sync_index(cls);
        gate
    }

    /// #76-T1 → **T5b (#208) 出场迁链改写**（ADR 20260723 裁定 3：买点买卖点卖同一套链，
    /// 出场不独立设计）：反向点 x′（`d.signal_index`）的同点递归链确认 ⟹ 准出；无链
    /// （NoChain）/ 断链（Reject）⟹ 不准出（诚实口径，**禁 v0 fallback**）；Flat 拒；
    /// depth>0 不入统；链断以 miss（NoChain）呈现。**因果守卫（链上 judge_at ≤ 决策
    /// bar）是迁链新增严格项**（旧固定 ℓ+1 桥无守卫）；链不问方向（T5a）——同一套链
    /// 回答「x′ 是不是确认的拐点」，出场方向由交易层映射。
    ///
    /// 夹具复用 T3 供给线（`t3_supplies`/`t3_chain_classification`/`t3_gate_with_certs`）：
    /// x′=55 处 L0 分型 price=100、组锚 50；事件 ℓ=1 ↔ 账本级 L0。`xd_*` 变体 =
    /// sell1 终端背书 + Short 事件（旧桥同向键可命中 ⟹ 新旧语义差异可在同一夹具对照）。
    ///
    /// 红绿史（TDD）：(a2) 方向解放 / (c) 断链拒 / (g) 因果守卫在旧 `typed_lookup`
    /// 实装下为红（旧：同向固定 ℓ+1 hit 且 n_delta 即准、无守卫）；迁链后全绿。
    #[test]
    fn exit_gate_reverse_admit_chain_pass_reject_flat() {
        use super::super::super::strategy::exit::reverse_nest_cert_base;
        use super::super::super::types::{BspBits, Side};
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..10).map(|i| mk_bar(i, 1000 + i as i64, false)).collect();
        let sell1 = BspBits { sell1: true, ..Default::default() };
        let d_hit = xd_reverse_decision(VoiceSide::Short, sell1, 0, 55);
        let mut ctx = ExitNestGateCtx::new(&bars, &config);

        // (a) 同侧全链闭合 ⟹ 准出（新旧连续性锚：旧桥同向 hit+n_delta 亦准）。
        let cls_a = xd_chain_classification(&[(0, true)]);
        ctx.gate = xd_gate_with_short_certs(&[1], 100, &cls_a);
        ctx.classification = Some(cls_a);
        ctx.cur_bar = 100;
        assert!(ctx.reverse_admit(&d_hit, 0), "(a) 全链闭合（链顶=L0 单级链）⟹ 准出");

        // (a2) 方向退役解放（T5a 同点递归不问方向）：Long 证书同键闭合 Short 反向候选
        // 的链 ⟹ 准出；旧桥同向键（is_long=false）必 miss——迁移预期差之一（红→绿）。
        let cls_a2 = t3_chain_classification(&[(0, true)]);
        ctx.gate = t3_gate_with_certs(&[1], 100, &cls_a2); // Long 事件 + buy1 背书
        ctx.classification = Some(cls_a2);
        assert!(
            ctx.reverse_admit(&d_hit, 0),
            "(a2) 异侧证书同点闭合 ⟹ 准出（方向退役：链问「是不是拐点」，不问买卖）"
        );

        // (b) 锚不可解（src=56 无分型）⟹ NoChain ⟹ 不准出；v0 基例放行 ⟹ 对照差落账。
        let d_miss = xd_reverse_decision(VoiceSide::Short, sell1, 0, 56);
        assert!(reverse_nest_cert_base(&d_miss), "夹具前提：v0 基例对该候选放行");
        assert!(!ctx.reverse_admit(&d_miss, 0), "(b) NoChain ⟹ 不准出（禁 v0 fallback）");

        // (c) 断链拒：存在性 L0/L1 俱在而证书仅 L0 闭合（ℓ=1），链顶 L1 缺证 ⟹ Reject
        // ⟹ 不准出——「lvl+1 单级过证不算」同型；旧桥固定 ℓ+1 单级 hit 即准（红→绿）。
        let cls_c = xd_chain_classification(&[(0, true), (1, true)]);
        ctx.gate = xd_gate_with_short_certs(&[1], 100, &cls_c); // 仅 ℓ=1（账本级 L0）有证
        ctx.classification = Some(cls_c);
        assert!(!ctx.reverse_admit(&d_hit, 0), "(c) 链顶缺环 ⟹ 断链拒（链闭合 vs 单级 hit）");

        // (d) Flat 候选 ⟹ 拒（无方向 ⟹ 无证书，与 v0 基例同语义）。
        let d_flat = xd_reverse_decision(
            VoiceSide::Flat,
            BspBits { sell1: true, buy1: true, ..Default::default() },
            0,
            55,
        );
        assert!(!ctx.reverse_admit(&d_flat, 0), "(d) Flat ⟹ 无证书拒");

        // (e) depth>0：查询读出弃用，不进统计（F1 锚替代域）。
        let before = ctx.stats.total;
        let _ = ctx.reverse_admit(&d_hit, 1);
        assert_eq!(ctx.stats.total, before, "(e) depth>0 查询不进统计");

        // (g) 因果守卫（迁链新增严格项，旧桥无守卫）：唯一证书 judge_at=200 越过决策
        // bar=100 ⟹ 整证剔除 ⟹ 零闭合 ⟹ NoChain ⟹ 不准出（红→绿）；同夹具决策
        // bar=200 守卫内 ⟹ Pass 准出。
        let cls_g = xd_chain_classification(&[(0, true)]);
        ctx.gate = xd_gate_with_short_certs(&[1], 200, &cls_g);
        ctx.classification = Some(cls_g);
        ctx.cur_bar = 100;
        assert!(
            !ctx.reverse_admit(&d_hit, 0),
            "(g) judge_at 越决策 bar ⟹ 守卫剔除 ⟹ NoChain（新增严格项）"
        );
        ctx.cur_bar = 200;
        assert!(ctx.reverse_admit(&d_hit, 0), "(g′) judge_at ≤ 决策 bar ⟹ Pass 准出");

        // 统计核验：depth-0 查询 = a/a2/b/c/d/g/g′ 共 7；准入 a/a2/g′=3；链在
        // a/a2/c/g′=4；NoChain b/g=2；flat=1。cross：v0(hit)=true、v0(flat)=false——
        // agree=a/a2/d/g′=4…（见下逐项）；v0_pass_gate_rej=b/c/g=3。
        let s = &ctx.stats;
        assert_eq!(
            (s.total, s.admitted, s.chain_found, s.chain_none, s.flat_dir),
            (7, 3, 4, 2, 1),
            "出场门统计分账（迁链语义）"
        );
        assert_eq!(s.admit_rungs[0], 3, "准入侧链深全 0-rung");
        assert_eq!(s.rej_rungs[0], 1, "(c) 拒绝侧 0-rung");
        assert_eq!(s.cross_agree, 4, "a/a2/d/g′ 两路一致");
        assert_eq!(s.cross_v0_pass_gate_rej, 3, "b/c/g = v0 准链拒");
        assert_eq!(s.cross_v0_rej_gate_pass, 0);

        // (f) 链断以 miss（NoChain）呈现：Short 事件锚齐但终端背书 buy1 ⟹ 装配层基例门
        // （nest.rs:693 confirm_side）不产证 ⟹ 键域有身份而索引无证 ⟹ 零闭合 ⟹ NoChain
        // ⟹ 不准出（装配层基例门使 n_delta=false 的 0-rung 证书不存在，链断只能以
        // miss 呈现——旧测试同构形态迁移）。
        let mut ctx2 = ExitNestGateCtx::new(&bars, &config);
        let (fractals, merged) = t3_supplies();
        let mut g2 = NestChainGate::for_test_with_supplies(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            std::rc::Rc::new(fractals),
            std::rc::Rc::new(merged),
        );
        g2.absorb_exts(vec![nc_ext_anchored(
            nc_event(1, Side::Short, (30, 50), 50, 100, true),
            100,
            50,
        )]);
        // buy1 背书（t3 夹具）⟹ Short 事件过不了基例门 ⟹ 索引无证；src=55 存在位仍在。
        let cls_f = t3_chain_classification(&[(0, true)]);
        g2.sync_index(&cls_f);
        ctx2.gate = g2;
        ctx2.classification = Some(cls_f);
        ctx2.cur_bar = 100;
        assert!(!ctx2.reverse_admit(&d_hit, 0), "(f) 基例门不产证 ⟹ NoChain ⟹ 不准出");
        assert_eq!(ctx2.stats.chain_none, 1, "链断以 miss（NoChain）落账");
        assert_eq!(ctx2.stats.admitted, 0);
        assert_eq!(ctx2.stats.cross_v0_pass_gate_rej, 1, "v0 放行而链（断）拒");
    }

    /// #76-T2（v1 退出循环集成：门开真链压掉反向退出 / 门关逐字节回归锁）。
    ///
    /// 夹具：持多根（depth 0，60% 权重）+ depth-1 卖侧决策（30% 权重 ⟹ 部分减仓不覆盖
    /// held[0] 多声部快照）⟹ 门关时退出生成器反向项触发（v1 groups 不过滤根域，既有
    /// 语义）；门开时单调 bars 无分型 ⟹ 链锚不可解（NoChain）⟹ **诚实不准出**（T5b
    /// (#208) 迁链终态口径），多仓延至窗口终点强平。
    #[test]
    fn exit_gate_v1_reverse_exit_suppressed_without_cert() {
        use super::super::super::strategy::risk::StopInput;
        use super::super::super::types::{BspBits, Center};
        let mut config = ThetaConfig::default();
        config.tick.tick_size = 1.0; // 价格=美元
        // T5b (#208)：层载派生（门开 ⟹ 投影层必载——生产 π 路径 runner.rs:493 同款；
        // 层仅链查询读取，门关臂不建 ctx、fill/recognize 零消费 ⟹ 逐字节不变）。
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(Some(true)));
        let config = super::super::admission::chain_driven_level_projection(&config);
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(None));
        // 单调上行（无分型 ⟹ 无中枢 ⟹ 无 nest 事件）：全程 low>90 不触多止损。
        let bars = vec![
            ohlc_bar(0, 100, 110, 95, 105),
            ohlc_bar(1, 100, 110, 99, 108), // 开多成交
            ohlc_bar(2, 105, 115, 100, 110),
            ohlc_bar(3, 108, 118, 103, 113), // depth-1 卖侧决策成交（部分减仓）
            ohlc_bar(4, 110, 120, 105, 115), // 门关：反向退出 Close 成交 bar
            ohlc_bar(5, 112, 122, 107, 117), // 门开：持仓延至本 bar 终点强平
        ];
        // 根多：buy1 @ src 0，stop=pivot_low=90（全程不触及）。
        let entry = buy1_long_decision(0, 90);
        // depth-1 卖侧：sell1 @ src 2（Short 止损 pivot_high=200 全程不触及）——30% 权重
        // ⟹ 成交量 < 根多持仓 ⟹ 部分减仓，held[0] 多声部快照保留（退出生成器于 bar3 见
        // groups[3] 含卖侧决策 ⟹ 反向项成立）。
        let mut child_sell = buy1_long_decision(2, 90);
        child_sell.depth = 1;
        child_sell.bsp = BspBits { sell1: true, ..Default::default() };
        child_sell.stop_in = StopInput {
            pivot_low: 0,
            pivot_high: 200,
            center: Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 0, end_index: 0 },
        };
        let decisions = vec![entry, child_sell];

        // (1) 基线（无注入）== 显式门关（线程局部注入 false）——门关逐字节回归锁。
        let baseline = plan_and_fill_mtm(&decisions, &bars, 1_000_000.0, &config);
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(Some(false)));
        let off = plan_and_fill_mtm(&decisions, &bars, 1_000_000.0, &config);
        assert_eq!(off.n_orders, baseline.n_orders, "门关 ⟹ n_orders bit-exact");
        assert_eq!(off.equity_curve, baseline.equity_curve, "门关 ⟹ equity bit-exact");
        assert_eq!(
            off.trade_pnls_realized, baseline.trade_pnls_realized,
            "门关 ⟹ 已实现盈亏 bit-exact"
        );
        // 基线前提坐实：门关下反向退出触发（部分减仓记录 @3 + 反向退出 Close @4，非强平）。
        assert_eq!(baseline.trades.len(), 2, "门关：部分减仓记录 + 反向退出 Close 记录");
        assert!(!baseline.trades[1].forced_close, "门关：反向退出非强平");
        assert_eq!(baseline.trades[1].exit_bar, 4, "门关：反向退出 Close 于 bar4 成交");

        // (2) 门开：链恒 NoChain（单调 bars 无分型 ⟹ 锚不可解）⟹ 反向退出被压掉——
        // 少一笔退出 Close 单；多仓延至窗口终点强平（forced_close=true，exit_bar=5）。
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(Some(true)));
        let on = plan_and_fill_mtm(&decisions, &bars, 1_000_000.0, &config);
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(None));
        assert_eq!(
            on.n_orders + 1,
            off.n_orders,
            "门开 ⟹ 反向退出 Close 被压掉（少一单）：on={} off={}",
            on.n_orders, off.n_orders
        );
        assert_eq!(on.trades.len(), 2, "门开：部分减仓记录 + 终点强平记录");
        assert!(on.trades[1].forced_close, "门开：反向退出被压 ⟹ 终点强平");
        assert_eq!(on.trades[1].exit_bar, 5, "门开：持仓延至窗口末 bar");
        assert_eq!(
            on.trade_pnls_realized.len() + 1,
            off.trade_pnls_realized.len(),
            "门开 ⟹ 少一笔反向退出已实现盈亏"
        );
    }

    /// #76-T3（dual 退出循环：门关逐字节回归锁 + 门开机器烟）。
    ///
    /// 照实登记（探查实锚）：dual 的 depth-0 反向项在当前 recognize_nested 流中
    /// **结构性难达**——退出生成器（步 3）在开仓循环（步 2）之后运行，任何**成交**的
    /// 根域反向决策先在步 2 改写 held[0] 快照（record_held_voice），步 3 所见已是新
    /// 方向声部 ⟹ 反向项恒假（零成交边缘案例除外）。故 dual 的门开行为差在当前流程
    /// 恒为空集——本测试锁定：(1) 门关 == 基线逐字节（回归命门）；(2) 门开 ==
    /// 门关逐字节（E1 夹具无根域反向候选触发，门开机器全程空转、NEST_GATE_EXIT
    /// total=0——门开零操作回归）。反向项语义由 #76-T1/T2 + exit.rs 单测锚定
    /// （两循环共享同一 `exit_decision_for_nested_cert` 与同一闭包形态）。
    #[test]
    fn exit_gate_dual_off_bitexact_on_noop_when_no_reverse_fire() {
        let mut cfg = ThetaConfig::default();
        cfg.tick.tick_size = 1.0;
        let classification = e_classification(true);
        let tower = e_tower();
        let bars = e1_bars();
        let baseline = plan_and_fill_mtm_dual(&classification, &tower, &bars, 1_000_000.0, &cfg);
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(Some(false)));
        let off = plan_and_fill_mtm_dual(&classification, &tower, &bars, 1_000_000.0, &cfg);
        assert_eq!(off.fill.n_orders, baseline.fill.n_orders, "dual 门关 ⟹ n_orders bit-exact");
        assert_eq!(off.fill.equity_curve, baseline.fill.equity_curve, "dual 门关 ⟹ equity bit-exact");
        assert_eq!(off.leg_log.len(), baseline.leg_log.len(), "dual 门关 ⟹ 腿日志 bit-exact");
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(Some(true)));
        let on = plan_and_fill_mtm_dual(&classification, &tower, &bars, 1_000_000.0, &cfg);
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(None));
        assert_eq!(
            on.fill.n_orders, off.fill.n_orders,
            "门开机器烟：无根域反向触发 ⟹ 门开零操作（n_orders bit-exact）"
        );
        assert_eq!(on.fill.equity_curve, off.fill.equity_curve, "门开零操作 ⟹ equity bit-exact");
        assert_eq!(on.leg_log.len(), off.leg_log.len(), "门开零操作 ⟹ 腿日志 bit-exact");
        assert_eq!(on.leg_log.len(), 4, "夹具前提：E1 四步形态不变（2 开 + 1 子平 + 1 强平）");
    }

    /// #76 门开冒烟（截断窗，真实 BTC）：v1/dual 两退出循环在门开/门下各跑一遍，
    /// 出场侧读数（NEST_GATE_EXIT：反向准出率 + 链深构成 + v0 对照差）如实输出。
    /// 截断窗 = 前 30_000 bar（E5 同款截断先例）；门开臂跑逐 bar 因果前缀喂法。
    ///
    /// T5b (#208)：本 smoke 兼作**出场迁链行为对照面**（出场侧不经 m8，NEST_GATE_EXIT
    /// 三窗零行）——env `T5B_EXIT_DUMP_PATH` 设置时门开臂逐查询落 dump（装置
    /// `t5b_exit_dump`，迁移前旧桥读出 / 迁移后链读出同键对齐）。config 经
    /// `chain_driven_level_projection` 派生（门开 ⟹ 投影层载——生产 π 路径 runner.rs:493
    /// 同款；层仅链查询读取，fill/recognize 零消费，门关臂行为逐字节不变）。
    #[test]
    #[ignore = "真实数据冒烟，需 analysis/data_cache/btc_1m_full.json；显式 --ignored --nocapture"]
    fn nest_exit_gate_smoke_real_btc() {
        use super::super::data;
        let config = ThetaConfig::default();
        let ds = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[#76 smoke] BTC 加载失败：{e}（DATA BLOCKER，如实跳过）");
                return;
            }
        };
        // T5b (#208)：层载派生（门开 ⟹ 层必载，生产同款；门关臂借原 config 零开销）。
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(Some(true)));
        let config = super::super::admission::chain_driven_level_projection(&config);
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(None));
        let n = 30_000.min(ds.bars.len());
        let bars = &ds.bars[..n];
        let l0 = parser::parse_layer(bars, &config);
        let (classification, tower) = classifier::classify_with_tower(&l0, &config);
        let decisions = strategy::recognize(&classification, bars, &config);
        eprintln!(
            "[#76 smoke] bars={} levels={} decisions={}",
            n,
            classification.levels.len(),
            decisions.len()
        );
        for on in [false, true] {
            NEST_CERT_GATE_OVERRIDE.with(|c| c.set(Some(on)));
            let fill = plan_and_fill_mtm(&decisions, bars, 1.0e6, &config);
            eprintln!(
                "[#76 smoke v1 gate={}] n_orders={} trades={} realized_pnls={}",
                on,
                fill.n_orders,
                fill.trades.len(),
                fill.trade_pnls_realized.len()
            );
            let dual = plan_and_fill_mtm_dual(&classification, &tower, bars, 1.0e6, &config);
            eprintln!(
                "[#76 smoke dual gate={}] n_orders={} close_legs={} trades={}",
                on,
                dual.fill.n_orders,
                dual.leg_log.iter().filter(|l| l.close).count(),
                dual.fill.trades.len()
            );
        }
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(None));
    }

    /// #75 诊断（不进 CI）：真实 BTC 截断的终端塔单发派生——逐级打印
    /// windows/合法 run/视图/事件计数，定位「prefix 零事件」环节（对照 p92 终端通过程）。
    /// `NEST_GATE_SMOKE_BARS` 截尾窗（默认 50_000）。
    #[test]
    #[ignore = "#75 诊断：终端塔单发派生逐环节计数（手工触发，需数据）"]
    fn nest_gate_diag_terminal_events() {
        use super::super::data;
        use classifier::level_view::{
            assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
            provide_nest_candidate_events_ext, C2LevelViewConfig, C2VersionTuple,
            CoordinateWindow, LevelViewMaterial, LevelViewQuery, ProjectionMaterial,
        };
        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("BTC 加载失败：{e}（DATA BLOCKER）");
                return;
            }
        };
        let n_full = ds_full.bars.len();
        let max_bars: usize = std::env::var("NEST_GATE_SMOKE_BARS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50_000);
        // NEST_GATE_SMOKE_START：窗口起点（缺省 = 尾部 n_full-max_bars）。
        let start: usize = std::env::var("NEST_GATE_SMOKE_START")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| n_full.saturating_sub(max_bars));
        let end = (start + max_bars).min(n_full);
        // 切片必须经契约函数（source_index 重置为局部下标，data.rs:113-128）——裸切片保留
        // 全集偏移会让结构坐标（source_index 域）与窗口 as_of 错位，派生全灭（实证坐实）。
        let ds_win = ds_full.slice_bar_range(start, end);
        let bars = &ds_win.bars;
        let as_of = bars.len() - 1;
        eprintln!("[diag] window=[{start}..{end})");
        let l0 = parser::parse_layer(bars, &config);
        let closes: Vec<f64> = l0.merged_bars.iter().map(|b| b.close as f64).collect();
        let close_src: Vec<usize> = l0.merged_bars.iter().map(|b| b.source_index).collect();
        let series = classifier::divergence::compute_macd(&closes, &config.macd);
        let (_cls, tower) = classifier::classify_with_tower(&l0, &config);
        eprintln!("[diag] bars={} tower_levels={}", bars.len(), tower.len());
        for level in 1..tower.len() {
            let lower = match lower_legs_from(&tower[level - 1]) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[diag] L{level} lower_legs_from ERR {e:?}");
                    continue;
                }
            };
            // pair 失败阶段计数（provide_divergence_pairs 逐字复制，定位零 pair 根因）。
            let segments: Vec<super::super::super::types::Segment> = lower
                .iter()
                .map(|value| {
                    let (start_price, end_price) = match value.direction {
                        super::super::super::types::Direction::Up => (value.lo, value.hi),
                        super::super::super::types::Direction::Down => (value.hi, value.lo),
                    };
                    super::super::super::types::Segment {
                        direction: value.direction,
                        start_index: value.start_index,
                        end_index: value.end_index,
                        start_price,
                        end_price,
                    }
                })
                .collect();
            let windows = &tower[level];
            let (mut n_valid, mut n_runs, mut n_views, mut n_events, mut n_confirmed) =
                (0usize, 0usize, 0usize, 0usize, 0usize);
            let (mut n_tail50k, mut n_tail1m) = (0usize, 0usize);
            let mut hist10 = [0usize; 10];
            let mut run_start = None;
            for index in 0..=windows.len() {
                let valid = index < windows.len()
                    && project_extended_windows_carried_only(std::slice::from_ref(&windows[index]))
                        .is_ok();
                if valid {
                    n_valid += 1;
                }
                match (run_start, valid) {
                    (None, true) => run_start = Some(index),
                    (Some(s), false) => {
                        n_runs += 1;
                        let projection =
                            project_extended_windows_carried_only(&windows[s..index]).expect("run 投影");
                        let centers: Vec<_> =
                            projection.seeds.iter().map(|seed| seed.center).collect();
                        let blocks = classifier::decompose::decompose(&centers);
                        let view = assemble_level_view(
                            C2LevelViewConfig { enabled: true },
                            LevelViewQuery {
                                level: level as u32,
                                coordinate_window: CoordinateWindow {
                                    start: projection.seeds.first().expect("非空").start_index,
                                    end: projection.seeds.last().expect("非空").end_index,
                                },
                                as_of,
                                version: C2VersionTuple::auto_pairing(),
                            },
                            LevelViewMaterial {
                                projection: ProjectionMaterial::ExactThree(&projection),
                                move_blocks: &blocks,
                                lower_legs: &lower,
                                hist: &series.hist,
                                dif: &series.dif,
                                close_src: &close_src,
                            },
                        );
                        match view {
                            Ok(view) => {
                                n_views += 1;
                                eprintln!(
                                    "[diag] L{level} run[{s}..{index}) seeds={} blocks={} trend_completed={} pairs={}",
                                    projection.seeds.len(),
                                    blocks.len(),
                                    blocks
                                        .iter()
                                        .filter(|b| matches!(
                                            b.kind,
                                            super::super::super::types::MoveKind::Trend
                                        ) && matches!(
                                            b.status,
                                            classifier::decompose::MoveStatus::Completed
                                        ))
                                        .count(),
                                    view.pairs.len(),
                                );
                                // pair 失败阶段逐字复制计数（provide_divergence_pairs 同序短路）。
                                {
                                    let anchors =
                                        classifier::divergence::self_anchors(&segments);
                                    let mut drops = [0usize; 6];
                                    for block in &blocks {
                                        let Some(direction) = block.dir else {
                                            drops[0] += 1;
                                            continue;
                                        };
                                        if block.end_center <= block.start_center
                                            || block.end_center >= projection.seeds.len()
                                        {
                                            drops[1] += 1;
                                            continue;
                                        }
                                        let prev =
                                            projection.seeds[block.end_center - 1].center;
                                        let last = projection.seeds[block.end_center].center;
                                        if classifier::divergence::locate_departure_move_a(
                                            &segments, &anchors, &prev, &last, direction,
                                        )
                                        .is_none()
                                        {
                                            drops[2] += 1;
                                            continue;
                                        }
                                        let lo = segments.partition_point(|sg| {
                                            sg.start_index < last.end_index
                                        });
                                        let hi = segments
                                            .partition_point(|sg| sg.end_index <= as_of);
                                        if lo > hi {
                                            drops[3] += 1;
                                            continue;
                                        }
                                        let Some(c_terminal) = segments[lo..hi]
                                            .iter()
                                            .find(|sg| sg.direction == direction)
                                        else {
                                            drops[3] += 1;
                                            continue;
                                        };
                                        if classifier::divergence::departure_move_c_start(
                                            &segments,
                                            &anchors,
                                            &last,
                                            direction,
                                            c_terminal.start_index,
                                        )
                                        .is_none()
                                        {
                                            drops[4] += 1;
                                            continue;
                                        }
                                        drops[5] += 1;
                                    }
                                    eprintln!(
                                        "[diag] L{level} pair_drops dir_none={} bounds={} locate_a={} c_terminal={} c_start={} reach_end={} legs={} seg_last_end={:?}",
                                        drops[0],
                                        drops[1],
                                        drops[2],
                                        drops[3],
                                        drops[4],
                                        drops[5],
                                        segments.len(),
                                        segments.last().map(|sg| sg.end_index),
                                    );
                                }
                                let exts = provide_nest_candidate_events_ext(
                                    level as u32,
                                    &projection,
                                    &blocks,
                                    &lower,
                                    &view,
                                    &series.hist,
                                    &series.dif,
                                    &close_src,
                                    // T1 (#170)：三元锚供给（诊断臂同用真实包含层/分型管）。
                                    &l0.fractals,
                                    &l0.merged_bars,
                                );
                                n_events += exts.len();
                                n_confirmed +=
                                    exts.iter().filter(|e| e.event.divergence_confirmed).count();
                                for ext in &exts {
                                    let ts = ext.event.turn_source;
                                    if ts >= bars.len().saturating_sub(50_000) {
                                        n_tail50k += 1;
                                    }
                                    if ts >= bars.len().saturating_sub(1_000_000) {
                                        n_tail1m += 1;
                                    }
                                    let bucket = (ts * 10 / end.max(1)).min(9);
                                    hist10[bucket] += 1;
                                }
                            }
                            Err(e) => eprintln!("[diag] L{level} run[{s}..{index}) view ERR {e:?}"),
                        }
                        run_start = None;
                    }
                    _ => {}
                }
            }
            eprintln!(
                "[diag] L{level} windows={} valid={} runs={} views={} events={} confirmed={} tail50k={} tail1m={}",
                windows.len(),
                n_valid,
                n_runs,
                n_views,
                n_events,
                n_confirmed,
                n_tail50k,
                n_tail1m
            );
            eprintln!("[diag] L{level} ts_deciles={hist10:?}");
        }
    }

    /// T1 (#170) 锚供给真实数据探针（手工触发，需数据；不进 CI、不进任何生产输出）：
    /// 终端塔单发逐级 `derive_level_events`（生产 provider 路径 + 真实包含层/分型管供给），
    /// 统计 confirmed 事件的锚缺失率（`n_anchor_misses` 真实语料实测）与新键域装填规模。
    /// `T1_PROBE_BARS` 截尾窗（默认 50_000），`T1_PROBE_START` 窗起点（缺省 = 尾部）。
    #[test]
    #[ignore = "T1 锚供给真实数据探针（手工触发，需数据）"]
    fn t1_anchor_supply_real_data_probe() {
        use super::super::data;
        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("BTC 加载失败：{e}（DATA BLOCKER）");
                return;
            }
        };
        let n_full = ds_full.bars.len();
        let max_bars: usize = std::env::var("T1_PROBE_BARS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50_000);
        let start: usize = std::env::var("T1_PROBE_START")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| n_full.saturating_sub(max_bars));
        let end = (start + max_bars).min(n_full);
        let ds_win = ds_full.slice_bar_range(start, end);
        let bars = &ds_win.bars;
        let as_of = bars.len() - 1;
        let l0 = parser::parse_layer(bars, &config);
        let (_cls, tower) = classifier::classify_with_tower(&l0, &config);
        let mut gate = NestChainGate::new(bars, &config);
        eprintln!("[t1probe] window=[{start}..{end}) tower_levels={}", tower.len());
        for level in 1..tower.len() {
            let exts = gate.derive_level_events(&tower, level, as_of);
            let n = exts.len();
            let n_conf = exts.iter().filter(|e| e.event.divergence_confirmed).count();
            let n_conf_miss = exts
                .iter()
                .filter(|e| {
                    e.event.divergence_confirmed
                        && (e.extreme_price.is_none() || e.group_anchor.is_none())
                })
                .count();
            gate.absorb_exts(exts);
            eprintln!(
                "[t1probe] L{level} exts={n} confirmed={n_conf} confirmed_anchor_miss={n_conf_miss}"
            );
        }
        eprintln!(
            "[t1probe] TOTAL absorbed={} by_triple_anchor_keys={} n_anchor_misses={}（0 = 真实语料锚供给全命中）",
            gate.events_by_level.iter().map(Vec::len).sum::<usize>(),
            gate.by_triple_anchor.len(),
            gate.n_anchor_misses,
        );
    }

    /// #75 门开冒烟（小截断实测，不进 CI）：NEST_GATE_STATS/CHAIN/INDEX 三行真链读数
    ///（拒绝率 + 链深构成 vs 72.7% 单级基线 + L2 对照差）+ 增量喂法成本计数；先跑门关
    /// 基线分离门的边际成本。`NEST_GATE_SMOKE_BARS` 截尾窗（默认 20_000；>4.6M=全量）。
    #[test]
    #[ignore = "门开冒烟：BTC 小截断实测 NEST_GATE 真链读数（手工触发，需数据）"]
    fn nest_gate_smoke_stats() {
        use super::super::data;
        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("BTC 加载失败：{e}（DATA BLOCKER，不伪造合成）");
                return;
            }
        };
        let n_full = ds_full.bars.len();
        let max_bars: usize = std::env::var("NEST_GATE_SMOKE_BARS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(20_000);
        let start = n_full.saturating_sub(max_bars);
        // 切片必须经契约函数（source_index 重置为局部下标，data.rs:113-128）。
        let mut ds = ds_full.slice_bar_range(start, n_full);
        ds.symbol = format!("BTC-NGSMOKE-{max_bars}");
        // 门关基线（bit-exact 路径，分离门的边际成本）。
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(Some(false)));
        let t0 = std::time::Instant::now();
        let _ = run_theta_v0_pi(&ds, &config, 1.0, 1.0e6);
        let wall_off = t0.elapsed();
        // 门开（真链判定）。
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(Some(true)));
        let t0 = std::time::Instant::now();
        let _ = run_theta_v0_pi(&ds, &config, 1.0, 1.0e6);
        let wall_on = t0.elapsed();
        NEST_CERT_GATE_OVERRIDE.with(|c| c.set(None));
        eprintln!(
            "[nest-gate-smoke] bars={} wall_off={:?} wall_on={:?} gate_marginal={:?}",
            ds.bars.len(),
            wall_off,
            wall_on,
            wall_on.saturating_sub(wall_off),
        );
    }

    /// ★on2w3 merge skip 神谕深覆盖（debug 构建）：真实 CL 全引擎 run_theta_v0_pi。生产路径 forest_epoch
    /// 稳定（bump 率 2.17%）⟹ merge cand 段大量走 skip；`merge_in_place_split` 内嵌 debug_assert 逐 bar
    /// 对拍 skip vs 强制全量末态——任一 bar 发散立即 panic（O(n²) 修复的 bit-exact 铁律守卫）。
    /// **须 debug 构建**（release 不编译 oracle）。`cargo test --lib`（非 --release）自动跑。
    #[test]
    #[ignore = "on2w3 merge skip 神谕深覆盖；需 CL；debug 构建（oracle 仅 debug_assertions）"]
    fn merge_skip_oracle_real_cl_debug() {
        use super::super::data;
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("需 CL 数据");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let n = 4_000.min(oos.bars.len());
        let prefix = Dataset {
            symbol: oos.symbol.clone(),
            bars: oos.bars[..n].to_vec(),
            dates: oos.dates[..n].to_vec(),
            bar_seconds: 60,
        };
        let res = run_theta_v0_pi(&prefix, &config, (n as f64) / (252.0 * 390.0), 1.0e6);
        assert!(res.metrics.strat_return.is_finite(), "strat 有限");
        eprintln!("on2w3 merge skip 神谕：{n} bar 全 merge 调用逐 bar skip==全量（debug_assert 未 panic）");
    }

    /// ★697 号 ceiling L2 验证（AncOK parent 稳定性收口，a5 工位）——真实 BTC 全引擎跑，
    /// 统计 Stale 四态分派频次 + restore 祖先链恢复率 + **暴露面**（restore 因 registry 丢失祖先
    /// 提前中断的次数 = 本应有 parent 但 registry 已失去）。
    ///
    /// 判据（`restore_break_registry_lost`）：
    /// - =0 ⟹ persistent carrier 在真实数据上**总能**恢复祖先链（问题1.pdf 建议6「证明 parent
    ///   可恢复」经验成立）⟹ ceiling 休眠，声部树严格性在 BTC 上未被触发。
    /// - >0 ⟹ 暴露面转正 ⟹ 存在被 admit 子声部腿祖先链未完整 ⟹ 须按建议6 三选一实装严格修复。
    ///
    /// **须真实 BTC 数据**（329MB，全历史 ~百万级 bar）；`#[ignore]` 默认不跑，L2 收口时
    /// `cargo test --release --lib -- --ignored --nocapture ancok_l2_ceiling_exposure_real_btc` 手动运行。
    #[test]
    #[ignore = "697 ceiling L2 验证；需 BTC 全历史（329MB）；重，手动 --ignored 跑"]
    fn ancok_l2_ceiling_exposure_real_btc() {
        use super::super::data;
        use crate::theta_v0::strategy::coverage::{ancok_probe_reset, ancok_probe_snapshot};
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("BTC", &config).expect("需 BTC 数据（analysis/data_cache/btc_1m_full.json）");
        let n = ds.bars.len();
        let years = (n as f64) / super::super::data::bars_per_year(ds.bar_seconds);

        ancok_probe_reset();
        let res = run_theta_v0_pi(&ds, &config, years, 1.0e6);
        let p = ancok_probe_snapshot();

        // 探针记账封闭性（自检）：每次 restore 调用恰好以四种方式之一终止（#226 增种子中断项）。
        assert_eq!(
            p.restore_complete
                + p.restore_break_already_in_raw
                + p.restore_break_registry_lost
                + p.restore_break_closed_seed,
            p.restore_calls,
            "restore 终止方式记账不封闭（探针 bug）"
        );
        // Stale 四态分派封闭性（自检）：四态之和 == Stale arm 命中总数。
        assert_eq!(
            p.state_live_present + p.state_live_detached + p.closed_inval_boundary_kept + p.closed_inval_pruned,
            p.stale_arm,
            "Stale 四态分派记账不封闭（探针 bug）"
        );

        let restore_success_rate = if p.restore_calls == 0 {
            f64::NAN
        } else {
            (p.restore_complete + p.restore_break_already_in_raw) as f64 / p.restore_calls as f64
        };
        eprintln!("═══ 697 ceiling L2 验证（BTC {n} bar, is_l2={}） ═══", res.is_l2);
        eprintln!("Stale arm 命中总数        : {}", p.stale_arm);
        eprintln!("  ├ LivePresent          : {}", p.state_live_present);
        eprintln!("  ├ LiveDetached         : {}", p.state_live_detached);
        eprintln!("  ├ Closed/Inval 作根保留 : {}", p.closed_inval_boundary_kept);
        eprintln!("  └ Closed/Inval 诚实剪   : {}", p.closed_inval_pruned);
        eprintln!("restore 调用总数          : {}", p.restore_calls);
        eprintln!("  ├ 自然收敛（抵达真根）  : {}", p.restore_complete);
        eprintln!("  ├ 提前收敛（已在 raw）  : {}", p.restore_break_already_in_raw);
        eprintln!("  ├ ★暴露面（registry 丢失）: {}", p.restore_break_registry_lost);
        eprintln!("  └ #226 种子中断（当 bar 被关父）: {}", p.restore_break_closed_seed);
        eprintln!("restore 恢复成功率        : {restore_success_rate:.6}");
        eprintln!("═══════════════════════════════════════════════");
    }

    /// ★★M7 三阶段资金 **L2 witness**（c3 工位，真实 BTC 数据可达性检验）——把合成 witness
    /// `pi_loop_realized_profit_reaches_earning_shares`（L1）升级到 L2：喂**真实 BTC 全历史**给
    /// **同一生产 π 路径**（`pi_theta_fill_loop`，与 `run_theta_v0_pi_inner` 逐字节同款调用），
    /// 读生产真值源 `tw_final.stage` 测 `Reach(Stage=II)>0` 与 `Reach(Stage=III)>0`。
    ///
    /// ## 为什么终态 stage = Reach（可达性由终态单读，无需逐 bar 快照）
    /// TStage 单向不可逆（`advance_to` rank 单调 + `TwEvent::Realize` 不触 stage，transition.rs
    /// 推导链第 8 条）⟹ stage 一旦推进永不回退 ⟹ **终态 stage.rank = 全程历史最高 rank**
    /// ⟹ `Reach(Stage≥s) ⟺ tw_final.stage.rank ≥ s.rank`。故读终态一个量即证可达性。
    ///
    /// ## 认识论 L2（formalization-validity-domain 231号）
    /// 真实 BTC bar 流 + frozen Θ（κ=0 baseline）+ 生产判据/账本/事件链全走生产路径（无 fixture
    /// 直捅 stage）⟹ **可否证**。可达（rank≥1/≥2）⟹ 输出证人（终 stage + 账本三量 Q_T/W_T/η_T
    /// + 是否满足终态不等式）；不可达（stage 恒 CostReduction）⟹ **照实否定 + 精确归因**
    /// （差多少、卡在哪个 barrier）——161号否定性照实同等合格，不改实现凑 PASS（no-workaround）。
    ///
    /// ## 验收（TARGET_STRATEGY_MAXFULL.md M7 / PDF p17）
    /// `Reach(Stage=III)>0 ∧ Q_T>Q_0 ∧ W_T≥I_0 ∧ η_T≥η_*`。本 witness 报告四量真值，
    /// PASS/否定均照实 eprintln（可否证结果是本测试的产出，非「必须通过」的断言）。
    ///
    /// **须真实 BTC 数据**（329MB）；`#[ignore]` 默认不跑：
    /// `cargo test --release --lib -- --ignored --nocapture m7_l2_witness_treasury_reach_real_btc`
    #[test]
    #[ignore = "M7 L2 witness；需 BTC 全历史（329MB）；重，手动 --ignored 跑"]
    fn m7_l2_witness_treasury_reach_real_btc() {
        use super::super::super::strategy::ledger::{RiskPolicy, TStage};
        use super::super::data;
        let mut config = ThetaConfig::default();
        let mut ds = data::load_by_symbol("BTC", &config)
            .expect("需 BTC 数据（analysis/data_cache/btc_1m_full.json）");
        // ★窗口截断（M7_WITNESS_BARS，可选）：O(n²) 前缀重分类在 461万 bar 全量上极重（小时级）。
        // 足量窗口（如 10 万 bar）即 L2 合规（任务明示「461万 bar 或足量窗口」）——env 未设 ⟹ 全量。
        if let Some(k) = std::env::var("M7_WITNESS_BARS").ok().and_then(|s| s.parse::<usize>().ok()) {
            ds.bars.truncate(k);
        }
        let n = ds.bars.len();
        let initial_nav = 1.0e6;
        // ★A10 成本注入（M7_WITNESS_A10=1，p126 runbook §2.1 阶段 3a 新口径）：margin=CME-simple +
        // cost=三常费率，与 m8_e2e 同函数同源（wverify_run::q4_margin_model/m6_cost_model，禁第二查法）；
        // env 未设 ⟹ 零成本旧路径 bit-exact 不动（A10 附则A 优先序 env>config>baseline；C5 不回滚条款：
        // cost_model=None ⟹ funding/borrow/liq 三项恒 0 ⟹ cum_holding_cost=0，与历史判据同值）。
        if std::env::var("M7_WITNESS_A10").ok().as_deref() == Some("1") {
            config.margin = Some(super::super::wverify_run::q4_margin_model(initial_nav));
            config.cost_model = Some(super::super::wverify_run::m6_cost_model());
        }

        // ★与 run_theta_v0_pi_inner 逐字节同款：真增量分类器 + 生产 π fill loop（χ≡1 全覆盖）。
        // 不经 run_theta_v0_pi（其 RunResult 不透传 π 路径 tw_final）——直接消费生产真值源。
        let mut classifier_incr = super::super::incremental::IncrementalClassifier::new(&ds.bars, &config);
        let fill = pi_theta_fill_loop(
            |i| {
                let (cls, tower) = classifier_incr.classify_at(i);
                let cl = classifier_incr.tower_confirmed_lens(tower.len());
                (cls, tower, cl, classifier_incr.tower_generation(), classifier_incr.forest_epoch())
            },
            &ds.bars,
            initial_nav,
            &config,
            None,
        );
        let tw = fill.tw_final.expect("π 路径 TW 生产者就位（tw_final=Some）");

        // 生产 κ=0 baseline（与 runner.rs:702 同）⟹ η_* = L^wc（本金全退后 = 0）。
        let policy = RiskPolicy::baseline();
        let q0 = initial_nav as i64; // Q_0 = 注资 notional_in（生产 tw 初始化 notional_in=⌊nav0⌋）
        let i0 = initial_nav as i64; // I_0 = 本金基线
        let reach_ii = tw.stage.rank() >= TStage::CapitalRecovered.rank();
        let reach_iii = tw.stage.rank() >= TStage::EarningShares.rank();
        let realized_sum: f64 = fill.trade_pnls_realized.iter().sum();

        eprintln!("═══ M7 L2 witness（BTC {n} bar, is_l2={}, n_orders={}） ═══", fill.n_orders > 0, fill.n_orders);
        eprintln!("终 TStage             : {:?}（rank={}）", tw.stage, tw.stage.rank());
        eprintln!("Reach(Stage=II)  >0   : {reach_ii}");
        eprintln!("Reach(Stage=III) >0   : {reach_iii}");
        eprintln!("Q_T (notional_in)     : {}（Q_0={q0}）", tw.notional_in);
        eprintln!("W_T (withdrawn)       : {}（I_0={i0}，W_T≥I_0={}）", tw.withdrawn, tw.withdrawn >= i0);
        eprintln!("η_T (tw)              : {}（η_*={}, η_T≥η_*={}）", tw.tw(), policy.eta_star(&tw), tw.tw() >= policy.eta_star(&tw));
        // ★A10 C5 增打两行（p126 runbook §2.1，additive 不动既有行）：cum_holding_cost = r_decomp
        // .tw_holding_cost_bridge（⌊funding+borrow+liq⌋ 累计量化 shadow，与 loop 内 eta_correction 同源）；
        // η_corrected = tw() − cum_holding_cost（C5 后唯一合法判读口径，修正只降不升）。
        let cum_holding_cost = fill.r_decomp.as_ref().map(|d| d.tw_holding_cost_bridge).unwrap_or(0);
        let eta_corrected = tw.tw() - cum_holding_cost;
        let holding_cost_truth = fill
            .r_decomp
            .as_ref()
            .map(|d| d.funding + d.borrow + d.liquidation_loss)
            .expect("π 路径 R 分解生产者就位（r_decomp=Some）");
        assert!(
            holding_cost_truth >= 0.0
                && (holding_cost_truth - cum_holding_cost as f64).abs() < 1.0,
            "A10 C5 witness 桥接对账取整界破：f64真值={} shadow={}",
            holding_cost_truth,
            cum_holding_cost,
        );
        eprintln!("cum_holding_cost      : {cum_holding_cost}（A10 C5 shadow；cost_model=None ⟹ 0 回归锁）");
        eprintln!("η_corrected           : {eta_corrected}（=tw()−cum_holding_cost，判读改用值；η_corrected≥η_*={}）", eta_corrected >= policy.eta_star(&tw));
        eprintln!("free / holding        : {} / {}", tw.free, tw.holding);
        eprintln!("open_legacy_legs      : {}", tw.open_legacy_legs);
        eprintln!("Σ已实现PnL            : {realized_sum:.2}（TW 漂移={}）", tw.tw() - q0);
        if reach_iii {
            let accept = tw.notional_in > q0 && tw.withdrawn >= i0 && tw.tw() >= policy.eta_star(&tw);
            eprintln!("★验收 Reach(III)∧Q_T>Q_0∧W_T≥I_0∧η_T≥η_* : {accept}");
        } else if reach_ii {
            eprintln!("★卡在 Stage II（退本金已完成，未进增股数）：EnterReady 五合取未全真");
            eprintln!("  归因：W_T≥I_0={}, legs=0={}, η_T≥η_*={}",
                tw.withdrawn >= i0, tw.open_legacy_legs == 0, tw.tw() >= policy.eta_star(&tw));
        } else {
            let recover_target = (tw.notional_in - tw.withdrawn).max(0);
            eprintln!("★否定：stage 恒 CostReduction（未退本金）");
            eprintln!("  P3 RecoverCapital 门：holding≥notional_in={}（holding={} vs {}）, free≥recover_target={}（free={} vs {}）",
                tw.holding >= tw.notional_in, tw.holding, tw.notional_in,
                tw.free >= recover_target, tw.free, recover_target);
            eprintln!("  归因：需 holding 累积过名义基线 + free（成本基回流+已实现利润）足额退回本金");
        }
        eprintln!("═══════════════════════════════════════════════");

        // ★账本恒等（无论可达与否恒成立，坐实 witness 走的是真生产账本）：TW 漂移 = ⌊Σ已实现PnL⌋。
        assert_eq!(tw.tw(), q0 + realized_sum as i64, "TW 漂移 = ⌊Σ费后已实现PnL⌋（A' 清单⑥不变量）");
        // ★openLegacyLegs=0 守卫（任务点3）：若达 EarningShares，EnterReady 入口证书要求 legs=0。
        if reach_iii {
            assert_eq!(tw.open_legacy_legs, 0, "EnterEarning 入口证书 openLegacyLegs=0（OQ-9 gate）");
        }
    }

    /// ★★M8 treasury 多窗 Reach 分布（d2 增量，编排者问题驱动）——回答「真实 BTC 上是否存在任何
    /// 窗口进 Stage II/III」。扫 BTC 全部 12 个 anchored walk-forward 窗口（2019-2025，**含 2020-2021
    /// 牛市段** wf1/wf2/wf3——策略正 PnL 概率最高段），各窗独立跑生产 π 路径读 tw_final。
    ///
    /// 复用 [`m7_l2_witness_treasury_reach_real_btc`] 的 tw_final 单读法（stage 单向不可逆 ⟹
    /// 终态 stage = Reach）。**不装配 RunResult/significance**（treasury 层只需 TW 终态，省掉昂贵
    /// 的 metrics/bootstrap——ponytail：四层里只有 treasury 层要扫多窗）。nav=窗首价×1000（牛市段
    /// 价格高，固定 1e6 会 qty=0）。
    ///
    /// 认识论 L2：任一窗进 Stage II+ ⟹ 单列（正向发现）；全 CostReduction ⟹ 每窗门距离表照实。
    ///
    /// `#[ignore]`: `cargo test --release --lib -- --ignored --nocapture m8_treasury_reach_distribution_real_btc`
    #[test]
    #[ignore = "M8 treasury 多窗 Reach；需 BTC 全历史（329MB）；重（12 窗 O(n²)），手动 --ignored 跑"]
    fn m8_treasury_reach_distribution_real_btc() {
        use super::super::super::strategy::ledger::TStage;
        use super::super::data;
        use super::super::prereg_windows::PREREG_WINDOWS;
        let mut config = ThetaConfig::default();
        // ★A10 成本注入（M7_WITNESS_A10=1，p126 runbook §2.1）：cost_model 循环前注入（与 nav 无关）；
        // margin 循环内按各窗 nav 注入（q4_margin_model cushions 依赖 nav0）——与 m8_e2e 同函数同源
        // （禁第二查法）；env 未设 ⟹ 零成本旧路径 bit-exact 不动（C5 回归锁：三项恒 0）。
        let a10 = std::env::var("M7_WITNESS_A10").ok().as_deref() == Some("1");
        if a10 {
            config.cost_model = Some(super::super::wverify_run::m6_cost_model());
        }
        let ds = data::load_by_symbol("BTC", &config).expect("需 BTC 数据");
        let sw = PREREG_WINDOWS.iter().find(|w| w.symbol == "BTC").expect("BTC 在 PREREG_WINDOWS");

        let mut report = String::from(
            "# M8 treasury 多窗 Reach 分布（BTC anchored walk-forward，含 2020-2021 牛市段）\n\n\
             口径：κ=0 baseline；nav=窗首价×1000；生产 π 路径 tw_final 单读（stage 单向不可逆 ⟹ 终态=Reach）。\n\n",
        );
        if a10 {
            report.push_str(&format!(
                "**成本口径：A10 注入（M7_WITNESS_A10=1）——margin=CME-simple + cost=三常费率（{}）；TW桥列=r_decomp.tw_holding_cost_bridge（⌊funding+borrow+liq⌋）。**\n\n",
                super::super::super::strategy::risk::RATE_UNCALIBRATED_LABEL,
            ));
        }
        report.push_str(
            "| 窗 | 期间 | bar | n_orders | 终Stage | Σ已实现PnL | holding | Q(notional_in) | holding−Q(门距离) | free | 退本金target | Reach≥II |\n\
             |---|---|---|---|---|---|---|---|---|---|---|---|\n",
        );
        let mut any_reach_ii = false;
        for w in sw.wf_anchored.iter() {
            let test = ds.slice_date_window(w.test_start, w.test_end);
            if test.bars.is_empty() {
                report.push_str(&format!("| wf{} | {}..{} | 空 | | | | | | | | | |\n", w.i, w.test_start, w.test_end));
                continue;
            }
            let n = test.bars.len();
            let first_px = test.bars.iter().find(|b| !b.untradable && b.close > 0)
                .map(|b| b.close as f64 * config.tick.tick_size).unwrap_or(1.0);
            let nav = (first_px * 1000.0).max(1.0e6);
            // ★A10 margin 按各窗 nav 注入（q4_margin_model cushions = 0.02/0.05·nav0；M7_WITNESS_A10=1 时）。
            if a10 {
                config.margin = Some(super::super::wverify_run::q4_margin_model(nav));
            }
            let mut ci = super::super::incremental::IncrementalClassifier::new(&test.bars, &config);
            let fill = pi_theta_fill_loop(
                |i| { let (c, t) = ci.classify_at(i); let cl = ci.tower_confirmed_lens(t.len()); (c, t, cl, ci.tower_generation(), ci.forest_epoch()) },
                &test.bars, nav, &config, None,
            );
            let tw = fill.tw_final.expect("π 路径 tw_final=Some");
            let realized: f64 = fill.trade_pnls_realized.iter().sum();
            let reach_ii = tw.stage.rank() >= TStage::CapitalRecovered.rank();
            any_reach_ii |= reach_ii;
            let recover_target = (tw.notional_in - tw.withdrawn).max(0);
            let stage_s = match tw.stage {
                TStage::CostReduction => "I", TStage::CapitalRecovered => "II", TStage::EarningShares => "III",
            };
            report.push_str(&format!(
                "| wf{} | {}..{} | {} | {} | {} | {:+.0} | {} | {} | {} | {} | {} | {} |\n",
                w.i, w.test_start, w.test_end, n, fill.n_orders, stage_s, realized,
                tw.holding, tw.notional_in, tw.holding - tw.notional_in, tw.free, recover_target, reach_ii,
            ));
            eprintln!("[m8-treasury] wf{} {}..{}: stage={} realizedPnL={:+.0} holding={} vs Q={} (门距离={})",
                w.i, w.test_start, w.test_end, stage_s, realized, tw.holding, tw.notional_in, tw.holding - tw.notional_in);
        }
        report.push_str(&format!(
            "\n**结论**：任一窗 Reach≥Stage II = **{}**。{}\n",
            any_reach_ii,
            if any_reach_ii { "★正向发现——见上表 Reach≥II=true 行。" }
            else { "全窗终 Stage I（CostReduction）——退本金门（holding≥Q ∧ free≥target）无窗满足。门距离列（holding−Q<0）= 持仓市值从未累积过名义基线，与 signal 层无方向 alpha ⟹ 已实现 PnL 无正累积一致。" },
        ));
        std::fs::write("/tmp/m8_treasury_reach_distribution.md", &report).ok();
        eprintln!("[m8-treasury] 多窗 Reach 分布落盘 /tmp/m8_treasury_reach_distribution.md（任一 Reach≥II={any_reach_ii}）");
    }

    /// ★★M7 κ barrier **L2 敏感性诊断网格**（d1 工位，codex `.kappa-ruling-20260704` 裁定2）——
    /// κ∈{0, 0.5, 1, 2}（有理定点 `0/1,1/2,1/1,2/1`）在同一 BTC 窗口跑生产 π 路径，报告各 κ 下
    /// **终 TStage / n_orders / EnterReady 五合取分项 / η_T−η_* barrier 距离**。
    ///
    /// ★认识论等级 = **L2 敏感性诊断，非选择**（formalization-validity-domain 231号 + codex 裁定3）：
    /// 本测试**不裁定生产 κ**（正 κ 生产选择推迟 M8 L3——收益/回撤/跨标的），只暴露 barrier 对
    /// 三阶段可达性的响应曲线。κ 单调 ⟹ κ 越大 η_* 越高 ⟹ EnterReady 越难过（monotone barrier）。
    /// κ=0 是最小可达性见证；若 κ=0 都不达 III，正 κ 无必要（codex：κ=0 不达 ⟹ 正 κ 不扫）。
    ///
    /// **须真实 BTC 数据**（329MB）；`#[ignore]` 默认不跑：
    /// `M7_WITNESS_BARS=100000 cargo test --release --lib -- --ignored --nocapture m7_kappa_sensitivity_grid_real_btc`
    #[test]
    #[ignore = "M7 κ L2 敏感性网格；需 BTC 全历史（329MB）；重，手动 --ignored 跑"]
    fn m7_kappa_sensitivity_grid_real_btc() {
        use super::super::super::strategy::ledger::{RiskPolicy, TStage};
        use super::super::data;
        let mut config = ThetaConfig::default();
        let mut ds = data::load_by_symbol("BTC", &config)
            .expect("需 BTC 数据（analysis/data_cache/btc_1m_full.json）");
        if let Some(k) = std::env::var("M7_WITNESS_BARS").ok().and_then(|s| s.parse::<usize>().ok()) {
            ds.bars.truncate(k);
        }
        let n = ds.bars.len();
        let initial_nav = 1.0e6;
        // ★A10 成本注入（M7_WITNESS_A10=1，p126 runbook §2.1）：margin=CME-simple + cost=三常费率，
        // 与 m8_e2e 同函数同源（禁第二查法）；env 未设 ⟹ 零成本旧路径 bit-exact 不动（C5 回归锁）。
        if std::env::var("M7_WITNESS_A10").ok().as_deref() == Some("1") {
            config.margin = Some(super::super::wverify_run::q4_margin_model(initial_nav));
            config.cost_model = Some(super::super::wverify_run::m6_cost_model());
        }
        let q0 = initial_nav as i64;
        let i0 = initial_nav as i64;

        // M7 网格（codex 裁定2）：κ=0/0.5/1/2 有理定点。κ=0 首行必与 witness/生产 bit-exact。
        let grid: [(i64, i64); 4] = [(0, 1), (1, 2), (1, 1), (2, 1)];

        eprintln!("═══ M7 κ barrier L2 敏感性网格（BTC {n} bar，Q_0={q0}） ═══");
        eprintln!("  κ       终TStage         n_orders  Reach_II  Reach_III  W_T≥I_0  legs=0  η_T-η_*(barrier距离)");
        for (num, den) in grid {
            // κ 经生产单点 knob 注入（kappa_policy_resolved 的 env 优先臂，A10 附则A 优先序
            // env>config>baseline）——re-run 整条生产 π 路径（忠实：κ 门控
            // stage_progression，post-hoc 改 policy 会算错轨迹）。
            std::env::set_var("KAPPA_BARRIER_NUM", num.to_string());
            std::env::set_var("KAPPA_BARRIER_DEN", den.to_string());
            let mut classifier_incr =
                super::super::incremental::IncrementalClassifier::new(&ds.bars, &config);
            let fill = pi_theta_fill_loop(
                |i| {
                    let (cls, tower) = classifier_incr.classify_at(i);
                    let cl = classifier_incr.tower_confirmed_lens(tower.len());
                    (cls, tower, cl, classifier_incr.tower_generation(), classifier_incr.forest_epoch())
                },
                &ds.bars,
                initial_nav,
                &config,
                None,
            );
            let tw = fill.tw_final.expect("π 路径 TW 生产者就位");
            let policy = RiskPolicy::try_new_ratio(num, den).unwrap();
            let eta_star = policy.eta_star(&tw);
            let reach_ii = tw.stage.rank() >= TStage::CapitalRecovered.rank();
            let reach_iii = tw.stage.rank() >= TStage::EarningShares.rank();
            let kappa_str = if den == 1 { format!("{num}") } else { format!("{num}/{den}") };
            eprintln!(
                "  {:<7} {:<15?} {:<9} {:<9} {:<10} {:<8} {:<7} {}",
                kappa_str, tw.stage, fill.n_orders, reach_ii, reach_iii,
                tw.withdrawn >= i0, tw.open_legacy_legs == 0, tw.tw() - eta_star,
            );
            // ★账本恒等每 κ 恒成立（坐实走真生产账本，非 fixture）：TW 漂移 = ⌊Σ已实现PnL⌋。
            let realized_sum: f64 = fill.trade_pnls_realized.iter().sum();
            assert_eq!(tw.tw(), q0 + realized_sum as i64, "κ={kappa_str}：TW 漂移 = ⌊Σ已实现PnL⌋");
        }
        std::env::remove_var("KAPPA_BARRIER_NUM");
        std::env::remove_var("KAPPA_BARRIER_DEN");
        eprintln!("═══════════════════════════════════════════════");
        eprintln!("★纯 L2 敏感性诊断（不裁定生产 κ）——生产冻结 κ=0，正 κ 选择推迟 M8 L3（codex 裁定3）");
    }

    /// ★Q2 close_pred 折 𝒦_Θ（loop 内见证）：风控门把退出折进可行集（非第二出口）——
    /// k_theta_risk_gate 产门 + pi_theta_position 收窄 𝒦_Θ。此处坐实 force_flat（Insolvent equity≤0）门。
    #[test]
    fn run_theta_v0_pi_risk_gate_force_flat_on_insolvent() {
        let bar = px100_bar(0);
        // equity≤0 ⟹ Insolvent ⟹ GlobalRiskClose ⟹ force_flat（𝒦_Θ={0}）。
        let (gate_insolvent, mode_insolvent) = k_theta_risk_gate(&[], &std::collections::HashMap::new(), &bar, -1.0, 0.0, 100.0, None);
        assert!(gate_insolvent.force_flat, "equity≤0 ⟹ Insolvent ⟹ force_flat（𝒦_Θ={{0}}）");
        // G3：透出的 mode 与门语义一致（z 第 13 维数据源同一真值）。
        assert_eq!(mode_insolvent, super::super::super::strategy::risk::RiskMode::Insolvent);
        // equity>0 + 无活动腿 ⟹ 门全开（无风控触发）。
        let (gate_open, mode_open) = k_theta_risk_gate(&[], &std::collections::HashMap::new(), &bar, 1.0e6, 0.0, 100.0, None);
        assert!(!gate_open.force_flat && !gate_open.stop_long && !gate_open.stop_short, "正常态 ⟹ 门全开");
        assert_eq!(mode_open, super::super::super::strategy::risk::RiskMode::Normal);
    }

    /// ★族A 回归守卫：campaign 持仓腿的 source_index 漂移后（carrier 走势 ρ 延伸），逐 bar 风控门
    /// 仍从入场冻结的 [`LedgerOpen::entry_stop`] 读出 stop 并判触及。旧路径用 drifted source_index
    /// 查 bsp_index 必返 None（无 bsp 在漂移后的 ρ）⟹ `None => continue` 静默跳过 ⟹ stop 永不触发
    /// （族A 根因，exitfix-research §族A）。本测试构造 drifted 腿 + 冻结 entry_stop 断言 stop 触发，
    /// 并对照 entry_stop=None（非交易点）诚实无 stop。
    #[test]
    fn k_theta_risk_gate_reads_frozen_entry_stop_for_drifted_leg() {
        use super::super::super::types::BspBits;
        use super::super::super::classifier::recursive_tower::ElementId;
        use super::super::mu_estimator::{MuClass, PositionState};
        use super::super::super::strategy::coverage::Vertical;
        use super::super::super::strategy::interp::{ActiveLeg, EntryCertificate, PositionNodeId};
        use super::super::super::strategy::voice::VoiceSide;
        use std::collections::HashMap;

        // Short campaign 腿：source_index=999（drifted ρ——carrier 延伸后，远超 entry bsp 坐标）。
        // 旧路径在此坐标查 bsp_index 必 None（无 bsp 在 999）⟹ 静默跳过。
        let leg = ActiveLeg {
            level: 0,
            dir: VoiceSide::Short,
            source_index: 999,
            lambda: 999,
            id: ElementId { level: 0, ordinal: 0 },
            parent_id: None,
            is_boundary_root: true,
            op_parent: None,
        };
        // 冻结 entry_stop=200（Tick，空头止损在上方）；entry_certificate 源坐标=5（非 drifted）。
        let mut open_trades: HashMap<ElementId, LedgerOpen> = HashMap::new();
        open_trades.insert(
            ElementId { level: 0, ordinal: 0 },
            LedgerOpen {
                entry_bar: 0,
                entry_px: 100.0,
                entry_z: MuClass::from_certificate(0, -1, BspBits::default(), 0, PositionState::Root),
                entry_stop_dist: Some(100.0),
                entry_stop: Some(200),
                entry_v: Vertical::ReverseOpen,
                position_node_id: PositionNodeId {
                    carrier: ElementId { level: 0, ordinal: 0 },
                    entry_certificate: Some(EntryCertificate { level: 0, source_index: 5 }),
                    side: VoiceSide::Short,
                    generation: 0,
                },
                opsem: OpsemEntrySnapshot::default(),
                units: 1.0,
            },
        );
        // bar high=250 ≥ stop=200 ⟹ 空头止损触及（stop_hit 空头镜像：high≥stop）。
        let bar = Bar {
            source_index: 999,
            timestamp: 999,
            open: 100,
            high: 250,
            low: 100,
            close: 100,
            volume: 1,
            untradable: false,
        };
        let (gate, _mode) = k_theta_risk_gate(&[leg], &open_trades, &bar, 1.0e6, -1.0, 100.0, None);
        assert!(
            gate.stop_short,
            "族A：drifted campaign 腿从冻结 entry_stop 读出 stop ⟹ high≥stop 触发 stop_short"
        );
        assert!(!gate.stop_long, "仅空腿止损，多腿无触发");

        // 对照：entry_stop=None（非该方向交易点）⟹ 诚实无 stop，不触发。
        open_trades.get_mut(&ElementId { level: 0, ordinal: 0 }).unwrap().entry_stop = None;
        let (gate2, _mode2) = k_theta_risk_gate(&[leg], &open_trades, &bar, 1.0e6, -1.0, 100.0, None);
        assert!(!gate2.stop_short, "entry_stop=None ⟹ 诚实无 stop（非静默吞掉真实 stop）");
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
    /// **结论（照实；A' 落地后口径更新）**：本闭环三阶段不推进的根因有二——①未实现浮盈只作诊断
    /// 不入 free（C'）；②生产闭环 PhaseI intent 恒 Buy（intent_adapter）⟹ 无平仓 fill ⟹ 已实现
    /// 利润通道（A' 的 `Realize`，schedule 减仓分支已实装）**无事发生**。故 stage 恒 CostReduction
    /// 照实。达 EarningShares 的生产路径在 π fill loop（反向候选真平仓产生已实现 PnL），见
    /// `pi_loop_realized_profit_reaches_earning_shares` 可达性见证。
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

    /// ★★L0 同价无盈亏定理（codex GAP3 裁定 A' 清单⑦，改名/改断言自
    /// `earning_shares_structurally_unreachable_from_campaign_tw_conserved`）：从 funded_campaign
    /// 出发，**L0 同价（price = avg_cost 恒等）下 sound closed-loop 路径无法到达 CapitalRecovered**
    /// ——EarningShares 在 L0 同价下不可达。
    ///
    /// ★★有效域收窄（A' 落地后的关键变化）：本定理**不再**覆盖 L2 realized-PnL 路径——
    /// `TwEvent::Realize` 是合法 TW 漂移构造子，L2 变价下平仓 PnL≠0 ⟹ TW 可增长 ⟹ 三约束
    /// 互斥被打破 ⟹ EarningShares **现实可达**（生产见证 `pi_loop_realized_profit_reaches_
    /// earning_shares`）。本定理的有效域 = **L0 同价**：卖价 ≡ 成本价 ⟹ 每笔平仓 PnL ≡ 0 ⟹
    /// `Realize` 分量恒 0 ⟹ TW 守恒代数照旧成立（无盈亏引理的可执行形式见
    /// `transition.rs::schedule_reduce_separates_cost_basis_and_realized_pnl` 的同价分支）。
    ///
    /// ★口径收窄（codex 二轮，沿袭）：这是 **sound 路径**（cash-tight w≤free 退本金）的不可达，
    /// 非「任何 raw 事件序列」不可达（raw 越权事件被 transition_adapter 现金-sound gate 拒）。
    ///
    /// **根因（三约束在 L0 同价下联合不可满足）**：
    /// - **L0 同价 TW 守恒**：funded_campaign 起 TW=Q；非 Realize 事件全守恒，Realize 分量在
    ///   同价下恒 0 ⟹ TW 恒 =Q（利润在 L0 同价不产生）。
    /// - **退本金前提**：stage_progression 要求 `holding ≥ notional_in = Q`。
    /// - **cash-tight**（codex#3 修复）：sound 退本金要求 `free > 0`（w=min(·,free)>0）。
    ///
    /// holding≥Q ∧ TW=Q ⟹ free ≤ 0 ⟹ 与 free>0 互斥。真达 EarningShares 需 **L2 价格变动让
    /// 已实现利润经 Realize 进 TW**（A' 已实装该通道——本定理界定的是它的 L0 退化边界）。
    ///
    /// 机制正确性（enter_ready/tw_step 各分支）由 `strategy::ledger` 单元测试在显式假设态上验证
    /// （synthetic-state legality tests，机制正确 ≠ 前提可达）；本测试是 L0 reachability 侧。
    #[test]
    fn earning_shares_unreachable_l0_same_price_zero_pnl() {
        use super::super::super::strategy::ledger::{TStage, TwState, TwEvent, tw_step};

        let q: i64 = 3;
        let funded = AssemblyState::funded_campaign(1_000_000, q);
        let tw0 = funded.tw_state.tw();
        assert_eq!(tw0, q, "funded_campaign 起 TW=Q（free=Q, holding=0）");

        // ★L0 同价守恒：非 Realize 事件全守恒 + Realize 在同价下分量恒 0（无盈亏引理）⟹ TW 恒 =Q。
        let s = funded.tw_state;
        let events = [
            TwEvent::ShortDiff(2), TwEvent::ShortDiff(-2),
            TwEvent::OpenShareLeg, TwEvent::CloseShareLeg(5), TwEvent::CloseShareLeg(-5),
            TwEvent::RecoverCapital(1), TwEvent::EnterEarning,
            TwEvent::Realize(0), // L0 同价 ⟹ 平仓 PnL≡0 ⟹ Realize 分量恒 0（漂移 0 = 守恒）
        ];
        for e in events {
            assert_eq!(tw_step(&s, e).tw(), tw0, "L0 同价事件 {:?} 保 TW=Q（无利润注入）", e);
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
    /// **不同**的诊断 hwm_gain——但因 hwm_gain 零承重（不入 free），且本闭环 PhaseI 恒 Buy 无平仓
    /// fill（A' 的已实现利润通道无事发生），两组 TW 均守恒（=notional_in）且 stage 均恒
    /// CostReduction。**未实现浮盈无论多大都不驱动 stage**（A' 推导链第 6 条黑名单的行为见证）；
    /// 已实现利润驱动 stage 的生产见证在 π 路径（`pi_loop_realized_profit_reaches_earning_shares`）。
    ///
    /// 对照设计：两组 rising 布尔序列相同（隔 bar 涨），仅幅度不同：
    /// - gentle（涨到 1001）：诊断浮盈峰值 1 ⟹ hwm_gain=1。
    /// - violent（涨到 100000）：诊断浮盈峰值 99000 ⟹ hwm_gain=99000。
    /// 两组 hwm_gain 不同（价格幅度驱动诊断），但 stage 均 CostReduction、TW 均 =notional_in（承重移除）。
    #[test]
    #[allow(deprecated)] // F-01：诊断口径测试，保留旧入口调用
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
        assert_eq!(b.tw_state.stage, TStage::CostReduction, "violent：未实现浮盈无论多大不驱动 stage（A' 黑名单行为见证）");
        assert_eq!(a.tw_state.tw(), a.tw_state.notional_in, "gentle TW 守恒 =notional_in（诊断不进 TW）");
        assert_eq!(b.tw_state.tw(), b.tw_state.notional_in, "violent TW 守恒 =notional_in（承重移除，浮盈不进 TW）");
        assert_eq!(a.tw_state.withdrawn, 0, "gentle 未退本金（无 sound 资金源）");
        assert_eq!(b.tw_state.withdrawn, 0, "violent 未退本金（诊断浮盈不构成退本金资金源）");
        assert!(b.tw_state.free == 0 && a.tw_state.free == 0, "两组 free 恒 0（诊断浮盈不入账 free）");
    }

    /// ★★L2 closed_loop 不触达实证（#[ignore]，真实 BTC 变价数据）：closed_loop 三阶段闭环接真实
    /// BTC 变价数据流，EarningShares count=0。复算：`cargo test --lib
    /// l2_btc_earning_shares_unreachable_hwm_debearing -- --ignored --nocapture`（默认全历史；
    /// `ECON_L2_MAX_BARS=N` 可截尾）。run_closed_loop 是 O(n)。
    /// 发生史：C' 前 count=1（hwm_gain 棘轮，被裁 PDF p8③ 语义回补）；C' 后浮盈只作诊断 ⟹ count=0。
    /// ★A' 落地后本测试语义收窄为「**closed_loop 生产策略**不触达」：已实现利润通道已实装
    /// （schedule 减仓分支 realized_pnl→Realize），但本闭环 PhaseI intent 恒 Buy ⟹ 无平仓 fill ⟹
    /// 无已实现 PnL 产生 ⟹ stage 恒 CostReduction（策略性不触达，非账本结构不可达——后者已被
    /// `pi_loop_realized_profit_reaches_earning_shares` 在 π 生产路径上否证）。
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
        assert_eq!(count, 0, "closed_loop 生产策略（PhaseI 恒 Buy 无平仓）L2 BTC 不触达 EarningShares（count=0）——实测 final_stage={:?}", s.stage);
        assert_eq!(s.stage, TStage::CostReduction, "无平仓 fill ⟹ 无已实现 PnL ⟹ stage 恒 CostReduction（策略性不触达）");
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

    /// B/F-05：翻转单先平掉旧声部、再因现金不足拒绝余量时，只有真实平仓段可入账。
    #[test]
    fn f05_partial_fill_rejected_remainder_does_not_pollute_voice_ledger() {
        let (mut cash, mut units, mut ec, mut pnls) = (100.0, -1.0, 90.0, Vec::new());
        let order = Order { action: StrictAction::Buy, qty: 2, exec_index: 0 };

        let fill = apply_order(
            &order,
            100.0,
            0.0,
            &mut cash,
            &mut units,
            &mut ec,
            &mut pnls,
        );

        assert_eq!(fill.requested_qty, 2.0);
        assert_eq!(fill.executed_qty, 1.0, "只成交平空的 1 手");
        assert_eq!(fill.closed_qty, 1.0);
        assert_eq!(fill.opened_qty, 0.0, "开多余量因现金不足全部拒绝");
        assert_eq!(fill.rejected_qty, 1.0);
        assert_eq!(units, 0.0);

        let mut voice_qty = vec![1];
        apply_voice_fill(&mut voice_qty, 0, fill);
        assert_eq!(voice_qty, vec![0], "声部账只扣真实平仓量，不得把拒绝余量记成新仓");

        let mut n_orders_executed = 0;
        if fill.executed_qty > 0.0 {
            n_orders_executed += 1;
        }
        assert_eq!(n_orders_executed, 1, "部分成交只计一个真实执行订单");
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
    #[allow(deprecated)] // F-01：真实数据冒烟（诊断用途，允许旧入口）
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
    #[allow(deprecated)] // F-01：既有测试保留旧入口；其 L2/L3 结论按 F-01 作废待重跑
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
    #[allow(deprecated)] // F-01：既有测试保留旧入口；其 L2 结论按 F-01 作废待重跑
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
    #[allow(deprecated)] // F-01：既有测试保留旧入口；其 L2 结论按 F-01 作废待重跑
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
    #[allow(deprecated)] // F-01：既有多品种显著性测试，保留旧入口调用
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

    // ──────────────────────────────────────────────────────────────────────
    //  关⑤ E 组：双账本嵌套集成（plan_and_fill_mtm_dual，4 个）
    //  施工图：chanlun/review-results/p120-nested-voice-dual-ledger-design-20260718.md §6
    // ──────────────────────────────────────────────────────────────────────

    use super::super::super::classifier::center::UnitRange;
    use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove};
    use super::super::super::types::Direction;

    fn obar(idx: usize, o: i64, h: i64, l: i64, c: i64) -> Bar {
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

    /// E 组塔夹具：L1 走势 A（Compose 五段 L0 子，外缘 Long，id=(1,0)，λ=0，ρ=20——
    /// 相对 A1 夹具的 ρ 已**延伸**（12→20，同 ElementId 同 λ，coverage.rs:4323 漂移形态），
    /// D1 零字段案的 span 包含身份重建据此验收）。
    fn e_tower() -> Vec<Rc<Vec<LeveledMove>>> {
        let unit = |si: usize, ei: usize, dir: Direction, lo: i64, hi: i64, ord: u64| {
            LeveledMove::from_unit(
                &UnitRange { start_index: si, end_index: ei, direction: dir, lo, hi },
                ElementId { level: 0, ordinal: ord },
            )
        };
        let s0 = unit(0, 4, Direction::Up, 0, 10, 0);
        let s1 = unit(4, 8, Direction::Down, 3, 12, 1);
        let s2 = unit(8, 12, Direction::Up, 5, 15, 2);
        let s3 = unit(12, 16, Direction::Down, 8, 18, 3);
        let s4 = unit(16, 20, Direction::Up, 10, 25, 4);
        let c = Center { zd: 5, zg: 15, dd: 0, gg: 25, start_index: 0, end_index: 20 };
        let a = LeveledMove::compose(&[s0, s1, s2, s3, s4], c, 1, ElementId { level: 1, ordinal: 0 });
        vec![Rc::new(Vec::new()), Rc::new(vec![a])]
    }

    /// E 组分类夹具：L1 buy1@12（父根开仓：host 查 (1,12) 落空——A 已延伸 ρ=20 ⟹ Ambient 根）；
    /// L0 sell1@16（子 ReverseOpen：host=sub(12,16) ⟹ 真父 A (level=1, ρ=20)；附着一致经
    /// span 包含重建 ⟺ A.id）；可选 L0 buy1@18（子腿反向关闭触发，interpret 规则2 同级）。
    fn e_classification(with_child_close_trigger: bool) -> Classification {
        let buy_parent = BspPoint {
            source_index: 12,
            level_origin: 0,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90,
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 150, dd: 85, gg: 160, start_index: 4, end_index: 12 })),
            struct_break_dir: None,
            force: None,
        };
        let sell_child = BspPoint {
            source_index: 16,
            level_origin: 0,
            bits: BspBits { sell1: true, ..Default::default() },
            pivot_low: 0,
            pivot_high: 210,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 150, dd: 85, gg: 160, start_index: 8, end_index: 16 })),
            struct_break_dir: None,
            force: None,
        };
        let mut l0 = vec![sell_child];
        if with_child_close_trigger {
            l0.push(BspPoint {
                source_index: 18,
                level_origin: 0,
                bits: BspBits { buy1: true, ..Default::default() },
                pivot_low: 120,
                pivot_high: 0,
                center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 150, dd: 85, gg: 160, start_index: 12, end_index: 18 })),
                struct_break_dir: None,
                force: None,
            });
        }
        Classification {
            levels: vec![
                LevelState { bsp: Rc::new(l0), ..Default::default() },
                LevelState { bsp: Rc::new(vec![buy_parent]), ..Default::default() },
            ],
        }
    }

    /// E 组分类夹具变体（#199 切片5）：子腿关闭触发改为**二类买**（buy2@18）——
    /// 短差腿二类平保留 ReverseType2（合法二类卖，断言③前半生产形态的载体）。
    fn e_classification_type2_child_close() -> Classification {
        let mut c = e_classification(false);
        let buy2_child_close = BspPoint {
            source_index: 18,
            level_origin: 0, // 三方合并 schema 适配（#110 级别身份）
            bits: BspBits { buy2: true, ..Default::default() },
            pivot_low: 120,
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 150, dd: 85, gg: 160, start_index: 12, end_index: 18 })),
            struct_break_dir: None,
            force: None,
        };
        let mut l0 = c.levels[0].bsp.as_ref().clone();
        l0.push(buy2_child_close);
        c.levels[0] = LevelState { bsp: Rc::new(l0), ..Default::default() };
        c
    }

    /// E1 价格路径（22 根）：上行 → 顶（bar 16-17）→ 回调（18-19）→ 续（20-21）。
    /// 止损不触及（父 stop=90 / 子 stop=210 全程安全）；tick_size=1.0（tick=美元）。
    fn e1_bars() -> Vec<Bar> {
        let closes = [
            100, 105, 110, 115, 120, 125, 130, 135, 140, 145, 150, 155, 160, // 0..12 上行
            165, 168, 170, 172, 171, // 13..17 见顶（17=子空成交）
            150, 140, // 18..19 回调（19=子平成交）
            145, 150, // 20..21 续（21=末根强平）
        ];
        closes
            .iter()
            .enumerate()
            .map(|(i, &c)| obar(i, c - 1, c + 3, c - 3, c))
            .collect()
    }

    /// E3 价格路径：上行见顶后**崩落**（bar 18 low=85 < 父 stop=90 ⟹ 父止损触发）。
    fn e3_bars() -> Vec<Bar> {
        let mut bars = e1_bars();
        bars[18] = obar(18, 168, 169, 85, 130); // low 85 ≤ 90 ⟹ 父多头止损触及
        bars[19] = obar(19, 128, 129, 122, 125); // cascade 成交 bar
        bars[20] = obar(20, 126, 129, 124, 128);
        bars[21] = obar(21, 129, 132, 127, 130);
        bars
    }

    /// E1（四步 1-cycle 双账本形态，M13 父仓保持）：根 Long 开 → 子 Short 开（父腿不动）
    /// → 子买侧信号平（realized>0）→ 父腿仍在 → 终点全平 ⟹ 现金守恒。
    #[test]
    fn e2e_four_step_cycle_dual_ledger() {
        let mut cfg = ThetaConfig::default();
        cfg.tick.tick_size = 1.0;
        let classification = e_classification(true);
        let tower = e_tower();
        let bars = e1_bars();
        let dual = plan_and_fill_mtm_dual(&classification, &tower, &bars, 1_000_000.0, &cfg);

        // 逐腿轨迹：①根 Long 开@13；②子 Short 开@17（父腿不动）；③子平@19（realized>0，
        // 父腿仍在）；④父终点强平@21。
        assert_eq!(dual.leg_log.len(), 4, "四笔成交（2 开 + 1 子平 + 1 强平）");
        let l0 = &dual.leg_log[0];
        assert!(l0.leg == VoiceSide::Long && !l0.close && l0.depth == 0 && l0.bar == 13);
        assert!(l0.q_long_after > 0.0 && l0.q_short_after == 0.0);
        let l1 = &dual.leg_log[1];
        assert!(l1.leg == VoiceSide::Short && !l1.close && l1.depth == 1 && l1.bar == 17);
        assert_eq!(
            l1.q_long_after, l0.q_long_after,
            "开空不触多腿（M13 父仓保持，不先净额）"
        );
        assert!(l1.q_short_after > 0.0);
        let l2 = &dual.leg_log[2];
        assert!(l2.leg == VoiceSide::Short && l2.close && l2.depth == 1 && l2.bar == 19);
        assert!(l2.realized > 0.0, "子空腿 realized>0（价格下跌，G^sep>0 腿级语义）");
        assert_eq!(l2.q_long_after, l0.q_long_after, "子平仓后父腿仍在");
        assert_eq!(l2.q_short_after, 0.0);
        let l3 = &dual.leg_log[3];
        assert!(l3.leg == VoiceSide::Long && l3.close && l3.bar == 21);
        assert_eq!(l3.q_long_after, 0.0, "终点全平");
        assert_eq!(dual.ledger.q_long, 0.0);
        assert_eq!(dual.ledger.q_short, 0.0);

        // 交易轨迹：子平先（@19 非强平），父强平后（@21）。
        assert_eq!(dual.fill.trades.len(), 2);
        assert!(!dual.fill.trades[0].long);
        assert_eq!(dual.fill.trades[0].exit_bar, 19);
        assert!(!dual.fill.trades[0].forced_close);
        assert!(dual.fill.trades[1].long);
        assert!(dual.fill.trades[1].forced_close);

        // realized 口径不含强平：只有子平仓一笔（>0）。
        assert_eq!(dual.fill.trade_pnls_realized.len(), 1);
        assert!(dual.fill.trade_pnls_realized[0] > 0.0);

        // 现金守恒：cash_final == nav0 + Σ(trade_pnls_with_forced)（费用已内含 realized）。
        let sum: f64 = dual.fill.trade_pnls_with_forced.iter().sum();
        assert!(
            (dual.ledger.cash - (1_000_000.0 + sum)).abs() < 1e-6,
            "现金守恒：cash={} nav+Σpnl={}",
            dual.ledger.cash,
            1_000_000.0 + sum
        );
    }

    /// E2（G5 双层记账同数锁）：E1 中子空腿 realized == TW ShortDiff 事件的父降成本金额。
    /// #68② 后 TW 物证已接线（tw_final=Some：ReverseOpen **分腿成本基划转** + Realize 平仓入账
    /// + 腿计数）——但 G5 断言的「ShortDiff(d_cash) 金额 ≡ 子腿 realized」是**另一会计身份**
    /// （T37 child.P&L≡parent.cost_reduction 的双层记账同数），本接线的 ShortDiff 承载成本基
    /// 划转、子腿 realized 走 `Realize` 通道入账，二者不经同一事件 ⟹ G5 同数锁是**独立会计
    /// 裁定**（决策层语义，非 #68② 接线范围），保持 ignore 待裁定后启用。
    #[test]
    #[ignore = "G5 同数锁（TW ShortDiff 事件金额≡子腿 realized 会计身份）需独立裁定；#68② 已接 TW 物证（tw_final=Some），但 ShortDiff 承载成本基划转非父降成本同数——语义差即未接线项"]
    fn e2e_g5_double_entry_same_number() {
        let mut cfg = ThetaConfig::default();
        cfg.tick.tick_size = 1.0;
        let classification = e_classification(true);
        let tower = e_tower();
        let bars = e1_bars();
        let dual = plan_and_fill_mtm_dual(&classification, &tower, &bars, 1_000_000.0, &cfg);
        // 子空腿 realized（G^sep 腿级收益，一个会计身份）。
        let child_short_realized: f64 = dual
            .leg_log
            .iter()
            .filter(|r| r.close && r.leg == VoiceSide::Short && r.depth == 1)
            .map(|r| r.realized)
            .sum();
        assert!(child_short_realized > 0.0);
        // G5 同数（T37 child.P&L≡parent.cost_reduction）：TW ShortDiff(d_cash) 事件的
        // 父降成本金额 == child_short_realized——另一个会计身份。接线后启用断言。
        let tw = dual
            .fill
            .tw_final
            .expect("TW ShortDiff 通道接线后启用：TwState 产 ShortDiff 划转事件");
        let _ = (tw, child_short_realized);
    }

    /// E3（级联关闭 e2e）：父止损触发时子活 ⟹ 交易列表子平先于父平（最深优先）、零残留持仓。
    #[test]
    fn e2e_cascade_parent_stop_closes_child_first() {
        let mut cfg = ThetaConfig::default();
        cfg.tick.tick_size = 1.0;
        let classification = e_classification(false); // 无买侧触发——父止损驱动级联
        let tower = e_tower();
        let bars = e3_bars();
        let dual = plan_and_fill_mtm_dual(&classification, &tower, &bars, 1_000_000.0, &cfg);

        // 逐腿轨迹：根开@13 → 子开@17 → bar18 父止损（low=85≤90）⟹ cascade @19 子先平、父后平。
        assert_eq!(dual.leg_log.len(), 4, "2 开 + cascade 2 平（无强平笔）");
        let c0 = &dual.leg_log[2];
        let c1 = &dual.leg_log[3];
        assert!(c0.close && c0.depth == 1 && c0.leg == VoiceSide::Short && c0.bar == 19,
            "子平先（最深优先 cascade）");
        assert!(c1.close && c1.depth == 0 && c1.leg == VoiceSide::Long && c1.bar == 19,
            "父平后");
        // 交易列表同序：子平先于父平。
        assert_eq!(dual.fill.trades.len(), 2);
        assert!(!dual.fill.trades[0].long, "首笔=子空腿平");
        assert!(dual.fill.trades[1].long, "次笔=父多腿平");
        assert_eq!(dual.fill.trades[0].exit_bar, 19);
        assert_eq!(dual.fill.trades[1].exit_bar, 19);
        // 零残留持仓（cascade 全清；无强平 ⟹ realized 口径含两笔平仓）。
        assert_eq!(dual.ledger.q_long, 0.0);
        assert_eq!(dual.ledger.q_short, 0.0);
        assert_eq!(dual.fill.trade_pnls_realized.len(), 2);
        assert!(dual.fill.trade_pnls_realized[0] > 0.0, "子空腿崩落平仓 realized>0");
        // 现金守恒（终态零持仓：cash == nav0 + Σrealized（无强平））。
        let sum: f64 = dual.fill.trade_pnls_realized.iter().sum();
        assert!((dual.ledger.cash - (1_000_000.0 + sum)).abs() < 1e-6, "现金守恒");
    }

    /// E4（嵌套关闭 bit-exact 锁）：无 ReverseOpen 触发数据（缺塔 ⟹ 全 Ambient）⟹
    /// `plan_and_fill_mtm_dual` 输出 == `plan_and_fill_mtm`（§4.4 兼容嵌入的链路级对拍）。
    #[test]
    fn e2e_nested_disabled_bitexact() {
        let mut cfg = ThetaConfig::default();
        cfg.tick.tick_size = 1.0;
        // 单级 L0 buy1@0（pivot_low=90；无 3 类 bit ⟹ center 占位即可）；全程无卖侧 ⟹ 无反向。
        let classification = Classification {
            levels: vec![LevelState {
                bsp: Rc::new(vec![BspPoint {
                    source_index: 0,
                    level_origin: 0,
                    bits: BspBits { buy1: true, ..Default::default() },
                    pivot_low: 90,
                    pivot_high: 0,
                    center: None,
                    struct_break_dir: None,
                    force: None,
                }]),
                ..Default::default()
            }],
        };
        let bars = vec![obar(0, 100, 103, 97, 100), obar(1, 104, 107, 101, 105), obar(2, 109, 112, 106, 110)];
        let nav = 1_000_000.0;

        let decisions_old = strategy::recognize(&classification, &bars, &cfg);
        let old = plan_and_fill_mtm(&decisions_old, &bars, nav, &cfg);
        let dual = plan_and_fill_mtm_dual(&classification, &[], &bars, nav, &cfg).fill;

        assert_eq!(dual.equity_curve, old.equity_curve, "equity 逐字节");
        assert_eq!(dual.daily_returns, old.daily_returns, "returns 逐字节");
        assert_eq!(dual.trade_pnls_realized, old.trade_pnls_realized, "realized 逐字节");
        assert_eq!(
            dual.trade_pnls_with_forced, old.trade_pnls_with_forced,
            "含强平口径逐字节"
        );
        assert_eq!(dual.trades, old.trades, "交易轨迹逐字节（per-leg 轨迹=净额轨迹）");
        assert_eq!(dual.n_orders, old.n_orders, "订单数一致");
        assert!(old.n_orders > 0, "夹具有效（确有交易，非空对拍）");
        assert_eq!(dual.typed_ledger.len(), old.typed_ledger.len());
        assert!(old.tw_final.is_none(), "净额 v1 路径无 TW 接线（诚实 None，不变）");
        // #68②：dual 路径 TW 已接线（物证账本，不进 equity/pnls/trades 对拍字段——上方逐字节
        // 断言已锁决策层零漂移；TW 守恒由 dual_tw_wiring_conservation 专测）。
        assert!(dual.tw_final.is_some(), "dual 路径 TW 账本已接线（#68②）");
        assert!(dual.r_decomp.is_none() && old.r_decomp.is_none());
    }

    /// #68②/④-b TW 接线守恒（E1 夹具：父多腿 + 子 ReverseOpen 空腿共存 → 子平 → 父终点强平）：
    /// 1. tw_final=Some；2. TW 漂移不变量 `tw()−注资 == ⌊Σ平仓腿 realized⌋`（Realize 唯一漂移
    ///    构造子；强平 PnL 不入——父腿强平盈亏若入账此式即破，本断言同锁「强平排除」口径）；
    /// 3. holding>0（快照取强平前，父腿在飞）；4. legacy 腿计数开合平衡归 0；5. stage 恒
    ///    CostReduction（stage 推进机构未接的诚实镜像）。
    #[test]
    fn dual_tw_wiring_conservation() {
        use super::super::super::strategy::ledger::TStage;
        let mut cfg = ThetaConfig::default();
        cfg.tick.tick_size = 1.0;
        let classification = e_classification(true);
        let tower = e_tower();
        let bars = e1_bars();
        let dual = plan_and_fill_mtm_dual(&classification, &tower, &bars, 1_000_000.0, &cfg);
        let tw = dual.fill.tw_final.expect("#68②：dual 路径 TW 已接线");
        let realized_sum: f64 = dual.fill.trade_pnls_realized.iter().sum();
        assert!(realized_sum > 0.0, "夹具有效：子空腿平仓 realized>0");
        assert_eq!(
            tw.tw() - 1_000_000,
            realized_sum as i64,
            "TW 漂移 == ⌊Σ平仓腿 realized⌋（Realize 唯一漂移构造子；强平 PnL 不入）"
        );
        assert!(tw.holding > 0, "强平前快照：父腿成本基在 holding");
        assert_eq!(tw.withdrawn, 0, "无退本金事件（stage 推进机构未接）");
        assert_eq!(tw.open_legacy_legs, 0, "ReverseOpen 子腿开合平衡（腿计数通道已接）");
        assert_eq!(tw.stage, TStage::CostReduction, "无推进机构 ⟹ stage 恒 CostReduction（诚实镜像）");
    }

    /// #68②/④-b TW 腿计数在飞态（截窗到子 ReverseOpen 开仓 bar：子腿终点仍在飞）：
    /// open_legacy_legs==1；无平仓 ⟹ 无 Realize 事件 ⟹ ShortDiff 保 TW 守恒（tw()==注资）；
    /// holding 承载双腿成本基（q⁺·cost⁺+q⁻·cost⁻ > 0）。
    #[test]
    fn dual_tw_legacy_leg_count_open_at_end() {
        let mut cfg = ThetaConfig::default();
        cfg.tick.tick_size = 1.0;
        let classification = e_classification(false);
        let tower = e_tower();
        // 截窗到 bar 17（子 ReverseOpen 开仓 bar）——子腿尚无退出触发，强平前快照在飞。
        // （E1 全窗下子腿 @19 另有退出通道平仓——引擎既有行为，非本测试目标。）
        let bars: Vec<Bar> = e1_bars().into_iter().take(18).collect();
        let dual = plan_and_fill_mtm_dual(&classification, &tower, &bars, 1_000_000.0, &cfg);
        let tw = dual.fill.tw_final.expect("#68②：dual 路径 TW 已接线");
        assert_eq!(tw.open_legacy_legs, 1, "子 ReverseOpen 腿在飞 ⟹ legacy 腿计数=1");
        assert!(dual.fill.trade_pnls_realized.is_empty(), "无平仓（强平不计 realized 口径）");
        assert_eq!(tw.tw(), 1_000_000, "无 Realize 事件 ⟹ ShortDiff 保 TW 守恒");
        assert!(tw.holding > 0, "双腿成本基在 holding（q⁺·cost⁺+q⁻·cost⁻）");
    }
}
