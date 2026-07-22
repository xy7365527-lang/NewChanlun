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
use super::super::strategy::ledger::{tw_step, RiskPolicy, TwEvent, TwState};
use super::super::config::ThetaConfig;
use super::super::strategy::exit::{exit_decision_for, record_held_voice, HeldVoice};
use super::super::strategy::{AccountState, VoiceDecision};
use super::super::types::{Bar, Order, StrictAction};
use super::super::{classifier, parser, strategy};
use super::data::Dataset;
use super::dual_ledger;
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
    /// ★M6 R 分解表（路线.pdf p16 第十一关：R=ΣN_tΔP_t−Commission−Slippage−Funding−Borrow−
    /// LiquidationLoss + 守恒残差）。生产 π 路径（[`run_theta_v0_pi`] 系列）产出；旧 recognize
    /// 路径（[`run_theta_v0`]）为 `None`（诚实——R 分解只接生产 π fill loop）。
    pub r_decomp: Option<super::super::strategy::risk::RDecomposition>,
    /// 严格区间套证书 sidecar 汇总。默认 `None`；仅 `THETA_STRICT_NEST_SIDECAR=1/true/yes/on`
    /// 时在生产 π 重放同帧旁路产出，不参与订单、候选、风控、账本。
    pub strict_nest_sidecar: Option<StrictNestSidecarSummary>,
}

/// 严格区间套证书记录（P3 sidecar）：目标级 `top_level` 的一条 `N^δ_{ℓ↓0}` 证书。
#[derive(Debug, Clone, PartialEq)]
pub struct StrictNestCertificateRecord {
    pub top_level: usize,
    pub certificate: classifier::nest::NestCertificate,
}

/// 严格区间套 sidecar 的末帧汇总；口径复用 P2 `nest.rs::assemble_certificates`。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StrictNestSidecarSummary {
    /// 已观察生产 replay 帧数。
    pub frames: usize,
    /// 末帧 ℓ0 `cand_delta=true` 基例数。
    pub base_count: usize,
    /// 末帧 terminal 查无次数（P1 一致性推论下应为 0）。
    pub terminal_missing: usize,
    /// 末帧每个目标级的证书数。
    pub cert_per_top: Vec<(usize, usize)>,
    /// 末帧证书总数。
    pub cert_total: usize,
    /// 末帧证书流内容（预期极稀；P2 当前 BTC 全量为 0）。
    pub certificates: Vec<StrictNestCertificateRecord>,
}

struct StrictNestSidecarCollector {
    enabled: bool,
    summary: StrictNestSidecarSummary,
}

impl StrictNestSidecarCollector {
    fn new(enabled: bool) -> Self {
        Self { enabled, summary: StrictNestSidecarSummary::default() }
    }

    fn observe_frame(
        &mut self,
        l0: &parser::ParseLayer,
        classification: &classifier::Classification,
        tower: &[std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>],
        config: &ThetaConfig,
        cache: &classifier::TowerCache,
    ) {
        if !self.enabled {
            return;
        }

        let cand = classifier::cand_delta_tower_cached(l0, classification, tower, config, cache);
        let mut terminal_by_key = std::collections::HashMap::new();
        if let Some(l0_level) = classification.levels.first() {
            for p in l0_level.bsp.iter() {
                if p.bits.buy1 {
                    terminal_by_key.entry((p.source_index, 1i8)).or_insert(p.bits);
                }
                if p.bits.sell1 {
                    terminal_by_key.entry((p.source_index, -1i8)).or_insert(p.bits);
                }
            }
        }
        let frames = self.summary.frames + 1;
        let objects_by_level: Vec<&[classifier::recursive_tower::CpScanOwnership]> = classification
            .levels
            .iter()
            .map(|state| state.cp_ownership.as_slice())
            .collect();
        self.summary =
            summarize_strict_nest_certificates(&cand, &objects_by_level, &terminal_by_key);
        self.summary.frames = frames;
    }

    fn finish(self) -> Option<StrictNestSidecarSummary> {
        self.enabled.then_some(self.summary)
    }
}

fn strict_nest_side_i8(side: super::super::types::Side) -> i8 {
    match side {
        super::super::types::Side::Long => 1,
        super::super::types::Side::Short => -1,
    }
}

fn summarize_strict_nest_certificates(
    cand: &[Vec<classifier::recursive_tower::CandDeltaEvent>],
    objects_by_level: &[&[classifier::recursive_tower::CpScanOwnership]],
    terminal_by_key: &std::collections::HashMap<(usize, i8), super::super::types::BspBits>,
) -> StrictNestSidecarSummary {
    let mut summary = StrictNestSidecarSummary {
        base_count: cand.first().map(|evs| evs.iter().filter(|e| e.cand_delta).count()).unwrap_or(0),
        ..StrictNestSidecarSummary::default()
    };
    // F-06：terminal_missing 以“唯一 L0 基例”计数——此前在每个 top 的装配回调里累加，
    // 同一缺失基例会按可用 top 数重复计入。
    summary.terminal_missing = cand
        .first()
        .map(|evs| {
            evs.iter()
                .filter(|e| e.cand_delta)
                .filter(|e| {
                    !terminal_by_key.contains_key(&(e.confirm_src, strict_nest_side_i8(e.side)))
                })
                .count()
        })
        .unwrap_or(0);
    for top in 1..cand.len() {
        let certs = classifier::nest::assemble_certificates_terminal(
            cand,
            objects_by_level,
            0,
            top,
            |base| {
                terminal_by_key
                    .get(&(base.confirm_src, strict_nest_side_i8(base.side)))
                    .copied()
            },
        );
        summary.cert_per_top.push((top, certs.len()));
        summary.certificates.extend(certs.into_iter().map(|certificate| StrictNestCertificateRecord {
            top_level: top,
            certificate,
        }));
    }
    summary.cert_total = summary.certificates.len();
    summary
}

fn strict_nest_sidecar_enabled() -> bool {
    std::env::var("THETA_STRICT_NEST_SIDECAR")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"))
        .unwrap_or(false)
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
///    classifier/mod.rs:482 契约；第二返回值 = 逐级塔快照，供真 ShortDiff 角色）。
/// 2. `recognize + plan_and_fill_mtm` → [`plan_and_fill_mtm_dual`]（per-bar
///    [`strategy::recognize_nested`] + DualLedger 分腿成交 + cascade 退出 + 毛闸门）。
///
/// # ⚠️ Deprecated：与 [`run_theta_v0`] 同一前视有效域（bughunt F-01 口径）
///
/// 本入口同样对**全窗** bars 一次性 `parse_layer` + `classify_with_tower` 后把 BSP 决策放回
/// 其历史 `source_index` 执行（结构确认前视）——**产出禁止用于 L2/L3 认识论声明**，只可用作
/// 诊断/管线冒烟与合成夹具对拍。嵌套产量的因果口径验收由后续重放承担（施工图 §2 依赖层）；
/// 生产因果接线在 `nautilus::strategy::ThetaCore::recognize_current`（per-bar 窗口）。
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
    let mut strict_nest_sidecar = StrictNestSidecarCollector::new(strict_nest_sidecar_enabled());
    let fill = pi_theta_fill_loop(
        // ★工位 4g：返回塔代次（TreeCache O(1) 命中判据，跳过 per-bar O(tree) TreeKey::of）。
        |i| {
            let (cls, tower) = if strict_nest_sidecar.enabled {
                let (l0, cls, tower) = classifier_incr.classify_at_with_l0(i);
                strict_nest_sidecar.observe_frame(&l0, &cls, &tower, config, classifier_incr.tower_cache());
                (cls, tower)
            } else {
                classifier_incr.classify_at(i)
            };
            let gen = classifier_incr.tower_generation();
            let fe = classifier_incr.forest_epoch(); // ★on2w2：K_i O(1) 命中判据。
            (cls, tower, gen, fe)
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
    /// 净额执行层订单数（overlay ΔN 非零步数 = 真实下单次数）。
    pub n_overlay_orders: usize,
    /// 账户级累计净额价格 PnL（`Σ_t N_t·ΔP_t`，PDF §11 对账账户侧）。
    pub account_price_pnl: f64,
    /// Σ_v pnl_v（分账本侧，PDF §11 应 ≈ `account_price_pnl`）。
    pub total_voice_pnl: f64,
    /// 对账残差 `|account_price_pnl − total_voice_pnl|`（验收断言2：< eps）。
    pub reconcile_residual: f64,
    /// 终态 overlay 账本（活动 + 已离场声部归因，逐声部 entry_v/exit_v/parent(v)/role(v)/pnl_v）。
    pub overlay: super::super::strategy::overlay_state::OverlayState,
    /// 净额执行层 RunResult（同 `run_theta_v0_pi`，净额订单/权益——overlay 是其只读旁路，数字不变）。
    pub net_result: RunResult,
    /// ★M8 treasury 层终态（TARGET_STRATEGY_MAXFULL.md M7:156-159）：三阶段资金账本 `TwState`
    /// 终态（stage/free/holding/withdrawn/notional_in/open_legacy_legs）。overlay 臂驱动的同一
    /// 主 loop 内建 TW 账本（`pi_theta_fill_loop_overlay` 的 `fill.tw_final`），此前被
    /// `net_result: RunResult` 装配丢弃（RunResult 无 tw_final 字段）——M8 端到端四层报告的
    /// treasury 层（第三层）需读它算 `Reach(Stage)`/`Q_T`/`W_T`/`η_T`。`None` 仅当 bars 为空。
    pub tw_final: Option<super::super::strategy::ledger::TwState>,
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
pub fn run_theta_v0_pi_overlay(
    dataset: &Dataset,
    config: &ThetaConfig,
    years: f64,
    initial_nav: f64,
) -> OverlayRunResult {
    let bars = &dataset.bars;
    let mut overlay = super::super::strategy::overlay_state::OverlayState::new();

    let mut classifier_incr = super::incremental::IncrementalClassifier::new(bars, config);
    let fill = pi_theta_fill_loop_overlay(
        |i| {
            let (cls, tower) = classifier_incr.classify_at(i);
            let gen = classifier_incr.tower_generation();
            let fe = classifier_incr.forest_epoch();
            (cls, tower, gen, fe)
        },
        bars,
        initial_nav,
        config,
        None, // χ≡1 全覆盖（与 run_theta_v0_pi 同信号路径）
        Some(&mut overlay),
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
    // 净额执行订单数 = 活动 + 已离场声部（每声部至少一次 open/close 产 ΔN；诚实计数用 closed 表大小
    // + 活动声部数——每条离场声部完整经历过 open+close 两次 ΔN 端点，活动声部至少一次 open）。
    let n_overlay_orders = overlay.closed_voices().len() + overlay.active_voices().count();

    OverlayRunResult {
        symbol: dataset.symbol.clone(),
        n_bars: bars.len(),
        n_overlay_orders,
        account_price_pnl,
        total_voice_pnl,
        reconcile_residual,
        overlay,
        net_result,
        tw_final,
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
                cp_ownership: Rc::new(Vec::new()),
                pan_div: Rc::new(Vec::new()), // Q4：新确认投影只携 bsp（盘整背驰承接在 econ 层，此处无消费者）
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
/// - **stop（结构止损触及）**：per 活动腿从**入场冻结的** `structural_stop`（[`LedgerOpen::entry_stop`]，
///   开仓 bar 一次性算）判 `stop_hit(bar,…)` 触及——多腿止损 ⟹ `stop_long`、空腿止损 ⟹ `stop_short`。
///   族A 修复：旧路径逐 bar 用 `leg.source_index`（carrier 走势 ρ，随父延伸漂移）回查 `classification`
///   ⟹ drifted ρ 不命中 ⟹ 静默跳过 ⟹ 跨趋势持仓 stop 永不触发；改入场冻结对齐 nautilus
///   `record_held_voice`（exitfix-research §族A）。
/// - **reverse_signal 不入本门**：反向信号关活动腿走 `interpret` 𝒟_x（腿级单出口）；
///   **parent_invalid** v0 root 恒 false（无父）。
fn k_theta_risk_gate(
    prev_active: &[super::super::strategy::interp::ActiveLeg],
    open_trades: &std::collections::HashMap<classifier::recursive_tower::ElementId, LedgerOpen>,
    bar: &Bar,
    equity: f64,
    p_t: f64,
    px: f64,
    margin: Option<&super::super::strategy::risk::MarginModel>,
) -> (super::super::strategy::coverage::KThetaRiskGate, super::super::strategy::risk::RiskMode) {
    use super::super::strategy::coverage::KThetaRiskGate;
    use super::super::strategy::exec::{close_pred, stop_hit, CloseTriggers, FillSide};
    use super::super::strategy::risk::{
        global_risk_close, margin_inputs, risk_mode, RiskMode, RiskModeInput,
    };
    use super::super::strategy::voice::VoiceSide;

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

    // ★族A 修复（formal-chain §9 closePred Stop 覆盖度）：stop 从**入场冻结的 structural_stop**
    // （[`LedgerOpen::entry_stop`]）读出，非逐 bar 用 `leg.source_index` 回查 classification。
    // 旧路径 `leg.source_index` 是 carrier 走势的 ρ（右端点），随父延伸漂移（coverage.rs 父延伸
    // 不变量 ρ≥旧 source_index）⟹ 按 drifted ρ 查 bsp_index 不命中 ⟹ `None => continue` 静默
    // 跳过该腿 stop 判定 ⟹ 持仓跨大级别趋势时 stop 永不触发（族 A 根因，exitfix-research §族A）。
    // 入场路径（[`candidate_stop_dist`]）用候选 `c.source_index`（bsp 确认点）查得对——两路径同腿
    // 不同坐标是 bug。本修复对齐两路径 + nautilus `record_held_voice`（入场一次性算 stop 冻结到
    // HeldVoice.stop，exitfix-research line 49）：stop 值固定在开仓结构 = formal-chain §9 语义
    // （结构失效价触及，非 trailing）。L0 静态根因；L2 dump（3765 等笔 stop 读出实际值）待 OOS。
    let mut long_stop = false;
    let mut short_stop = false;
    for leg in prev_active {
        let exit_side = match leg.dir {
            VoiceSide::Long => FillSide::Sell,
            VoiceSide::Short => FillSide::Buy,
            VoiceSide::Flat => continue, // Flat 不入活动集（防御性）
        };
        let stop = match open_trades.get(&leg.id).and_then(|o| o.entry_stop) {
            Some(s) => s,
            None => {
                // 腿不在 open_trades = **结构走势载体**（非 campaign 持仓）。`next_active` 含走势元素
                // （AncOK 祖先闭包 + [`coverage::restore_ancestor_chain_from_registry`] 注入的父 carrier
                // —— empirically：活动集里 level-1 走势载体与 open_trades 里 level-0 campaign 持仓并存，
                // open_trades 仅记 campaign）。这类腿的 `dir`=走势 eps（非持仓方向）、`source_index`=
                // 走势 ρ（漂移）—— 非开仓结构，无 stop 可读。旧路径用 drifted ρ 查 bsp_index 也返
                // None ⟹ 同样跳过，但旧路径把载体**误当持仓**算 stop（无意义计算）。本修复按
                // open_trades 成员区分 campaign 持仓 vs 结构载体，仅前者判 stop（族A 正域）。
                continue;
            }
        };
        if !bar.untradable && stop_hit(bar, stop, exit_side) {
            match leg.dir {
                VoiceSide::Long => long_stop = true,
                VoiceSide::Short => short_stop = true,
                VoiceSide::Flat => {}
            }
        }
    }

    // close_pred 折 𝒦_Θ（契约锚保留）：风控项（stop ∨ risk）→ 方向约束门。
    // G3（#138）：mode 一并透出——z 第 13 维 risk_mode 的账本态真值源（每 bar 已算，零重算）。
    (
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
        },
        mode,
    )
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

/// barrier 缓冲系数 κ 政策注入（A10 附则A 裁定——**优先序写死**：env `KAPPA_BARRIER_*`
/// （L2 敏感性诊断覆写，codex `.kappa-ruling-20260704`）> `config.risk_policy` > `baseline()` κ=0）。
///
/// env `KAPPA_BARRIER_NUM` / `KAPPA_BARRIER_DEN`（默认 den=1）⟹ `RiskPolicy::try_new_ratio(num, den)`；
/// **两者都未设（或 num 不可解析，现状语义）⟹ 落 `config.risk_policy`；config=None ⟹ `baseline()`
/// κ=0**——生产/测试逐字节不变（bit-exact）。非法 env 值（负分子/非正分母）⟹ panic（诊断 knob
/// 快失败，不静默降级掩盖网格错配——语义不变）。单源纪律防双源静默漂移（χ G2 先例）。
fn kappa_policy_resolved(config_policy: Option<RiskPolicy>) -> RiskPolicy {
    let env = std::env::var("KAPPA_BARRIER_NUM")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .map(|num| {
            let den = std::env::var("KAPPA_BARRIER_DEN")
                .ok()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(1);
            (num, den)
        });
    kappa_priority_resolve(env, config_policy)
}

/// κ 优先序纯函数（A10 附则A 写死：env>config>baseline；可测——不碰进程 env）。
/// env 非法值 panic（与 env knob 语义一致）；env=None ⟹ config 或 baseline。
fn kappa_priority_resolve(env: Option<(i64, i64)>, config_policy: Option<RiskPolicy>) -> RiskPolicy {
    match env {
        Some((num, den)) => RiskPolicy::try_new_ratio(num, den)
            .unwrap_or_else(|| panic!("非法 κ barrier grid 值 num={num} den={den}（要求 num≥0 ∧ den>0）")),
        None => config_policy.unwrap_or_else(RiskPolicy::baseline),
    }
}

fn pi_theta_fill_loop<F>(
    classify_at: F,
    bars: &[Bar],
    initial_nav: f64,
    config: &ThetaConfig,
    chi: Option<ChiFilterCtx>,
) -> FillOutput
where
    F: FnMut(usize) -> (classifier::Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>, u64, u64),
{
    // ★M5 wrapper：overlay=None ⟹ 现有净额路径逐字节不变（bit-exact）。overlay 簿接线走
    // [`pi_theta_fill_loop_overlay`]（run_theta_v0_pi_overlay arm 传 Some）。
    pi_theta_fill_loop_overlay(classify_at, bars, initial_nav, config, chi, None)
}

/// ★M5 声部执行层 fill loop（多空对冲.pdf p16 关卡10）：与 [`pi_theta_fill_loop`] **同一决策路径**
/// （同一 `pi_theta_step_traced` → 净额订单 → apply_order fill），额外用 `StepTrace.sep_legs` 喂
/// `overlay: OverlayState` hedge-mode 簿——建持久逐声部 P^sep 账本 + ΔN 订单流 + 逐声部 pnl_v 归因。
///
/// `overlay=None` ⟹ 净额路径 bit-exact（所有现有臂）；`Some(&mut ov)` ⟹ 逐 bar 决策点把 sep_legs
/// 步进 overlay（**只读旁路**，不改净额 fill 的 cash/units/trade_pnls ⟹ 现有数字不动）。
fn pi_theta_fill_loop_overlay<F>(
    mut classify_at: F,
    bars: &[Bar],
    initial_nav: f64,
    config: &ThetaConfig,
    chi: Option<ChiFilterCtx>,
    mut overlay: Option<&mut super::super::strategy::overlay_state::OverlayState>,
) -> FillOutput
where
    // ★工位 4g/on2w2：闭包返回四元组——第三个 u64 = 塔代次（candidate 段判据）；第四个 u64 =
    // forest_epoch（K_i 森林段 O(1) 命中判据，on2w2 O(n²) 修复）。
    F: FnMut(usize) -> (classifier::Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>, u64, u64),
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
    // #82 DC-E：默认关闭时整段惰性（不算 MACD、不评生产门、不写子腿簿），订单轨 frozen。
    // 开启时首见证书只经 econ_positive 的 Nest/XZD 单一门，再写 #81 OscillationBook。
    let mut pan_div_state = super::pan_div::PanDivProductionState::default();
    let pan_div_hist = if config.center_oscillation.enabled && !bars.is_empty() {
        let closes: Vec<f64> = bars
            .iter()
            .map(|bar| bar.close as f64 / config.tick.tick_size)
            .collect();
        Some(super::super::classifier::divergence::compute_macd(
            &closes,
            &config.macd,
        ).hist)
    } else {
        None
    };

    let mut equity_curve = Vec::with_capacity(n);
    let mut trade_pnls: Vec<f64> = Vec::new();
    let mut trades: Vec<metrics::TradeRecord> = Vec::new();
    let mut pos_entry_bar: Option<usize> = None;
    let mut n_orders_executed: usize = 0;
    // ── G4 typed ledger（#134）：腿级在飞表（voice_id → 入场登记）+ 已结算 typed 交易。 ──
    let mut open_trades: std::collections::HashMap<
        classifier::recursive_tower::ElementId,
        LedgerOpen,
    > = std::collections::HashMap::new();
    let mut typed_ledger: Vec<TypedTrade> = Vec::new();
    // ★opsem-dump（基因 073a/274号）：env `OPSEM_DUMP_DIR` 启用时开两个 JSONL 写入器。
    // 未启用 ⟹ None，所有 write_trade/diff_tower 调用 no-op ⟹ 生产路径 bit-exact 不变。
    let mut opsem = OpsemDump::from_env();
    // ★A7（Task #165）：出场 z 账本态维（t_stage/eta_bucket/risk_mode）取自出场 bar 决策点的
    //   `ext_i`。窗口终点 censored Hold 在主循环外结算 ⟹ 需保留**最后一个决策点** ext_i（末可交易
    //   bar 的账本态真值，与 censored 兑现价同 bar）。主循环内每决策点刷新；无决策点（全窗不可交易）
    //   ⟹ ZExt::NONE（账本态维诚实 None，无 bar 决策点可取——同 entry_z 裸口径）。
    let mut last_ext: super::selector::ZExt = super::selector::ZExt::NONE;
    // ★A9（Task #166，级别容器.pdf p14/§13）：campaign generation 高水位表——carrier(ElementId) →
    //   该 carrier 已见最高 generation。同一 carrier close→reopen 时新 campaign 的 generation =
    //   高水位 +1（首次入场 = 0）。`ActiveLeg::id`/`voice_id` 会跨 campaign 复用（close 后同 carrier
    //   可再 open，见 interpret 无历史 tombstone + open_trades.insert 二次覆盖），高水位表使
    //   position_node_id 四元组严格不碰撞（时序再入场可达，codex a9-posnode 裁定 C）。
    let mut gen_hiwater: std::collections::HashMap<
        classifier::recursive_tower::ElementId,
        u32,
    > = std::collections::HashMap::new();
    // ── #124 裁定4：TW 账本单一生产真值源（TwState 接入 π 路径；run_closed_loop 降级纯结构
    //    验证工具）。注资口径 = funded_campaign 同款（state.rs:219）：整窗 = 一个 campaign，
    //    投入 = 初始 NAV 取整（free = notional_in = ⌊nav0⌋）。i64 取整粒度诚实声明：TW 账本是
    //    **结构谓词源**（P2/P3/P4 判据消费 stage/open_legacy_legs/holding/free/withdrawn），
    //    非逐分对账账本——其在生产 π 的唯一消费者是 I_Θ 组合层 TW 谓词 + tw_final 物证。
    //    ★可达性（codex GAP3 终局裁定 A' 落地，2026-07-03）：**已实现利润有入 free 通道**——
    //    每笔实际平仓 fill 的费后 PnL 经 ②'' `TwEvent::Realize` 入账（可正可负），TW 漂移 =
    //    量化已实现 PnL 累计 ⟹ holding≥notional_in ∧ free≥recover_target 在 L2 变价 + 足额
    //    已实现利润下可满足 ⟹ P2/P3/P4 生产触发**现实可达**（可达性见证 `pi_loop_realized_
    //    profit_reaches_earning_shares`）。L0 同价下平仓 PnL≡0 ⟹ 仍不可达（L0 同价无盈亏
    //    定理，见 earning_shares_unreachable_l0_same_price_zero_pnl）。
    let mut tw = TwState {
        free: nav0 as i64,
        notional_in: (nav0 as i64).max(1),
        ..TwState::initial()
    };
    // κ=0 最小基线（PDF §10 canonical 默认，runner.rs:702 生产口径）。
    // ★A10 附则A（裁定接口冻结）：优先序写死 env `KAPPA_BARRIER_*`（L2 敏感性诊断覆写，codex
    // `.kappa-ruling-20260704`，M7 网格 {0,0.5,1,2}=`0/1,1/2,1/1,2/1`）> `config.risk_policy` >
    // baseline κ=0——**env 未设 ∧ config=None ⟹ baseline κ=0**，所有生产/测试路径逐字节不变
    // （bit-exact）。κ 只在此单点注入，经 stage_progression 门控三阶段推进；env 纯诊断，不改生产
    // 冻结值（正 κ 生产选择是 M8 L3 的事，codex 已裁，A10 附则A 留编排者选择类）。与
    // M7_WITNESS_BARS 同为 L2 诊断 env 惯例。
    let tw_policy = kappa_policy_resolved(config.risk_policy);
    // TW 侧已见的真实成本基（i64 shadow；方向差分派 ShortDiff 划转，入账量 cash-sound 钳制
    // ——真实划转超出 free/holding 时部分承载 ⟹ holding 低估 ⟹ 退本金门更难过 = 安全侧）。
    let mut tw_seen_basis: i64 = 0;
    // ★A'（裁定清单③）：费后已实现 PnL 累计（f64 真值，仅实际平仓 fill 累加——apply_order
    // 返回值；forced_pnl 是窗口终点报告用假设强平，不改 units/cash，**不得**入此累计，推导链
    // 第 11 条）+ TW 侧已入账的量化累计 shadow（同 tw_seen_basis 模式：对**累计值**量化再派
    // 差分 ⟹ 截断误差有界不累积，TW 漂移恒 = ⌊Σ已实现PnL⌋）。
    let mut realized_cum: f64 = 0.0;
    let mut tw_seen_realized: i64 = 0;

    // ── M6 R 分解成本累计（路线.pdf p16 第十一关：R = ΣN_tΔP_t − Commission − Slippage −
    //    Funding − Borrow − LiquidationLoss）。cost_model=None ⟹ funding/borrow/liq 恒 0
    //    （bit-exact——不改 units/cash，只多算三个恒 0 累计），commission_slippage 仍如实累计
    //    （守恒断言用，不改现金流）。──
    let mut cum_fee: f64 = 0.0; // Commission+Slippage+Tax（apply_fill 独立测得，非从 net 反推）
    let mut cum_funding: f64 = 0.0;
    let mut cum_borrow: f64 = 0.0;
    let mut cum_liq_loss: f64 = 0.0;
    // 价格 PnL 毛额 Σ_t N_t·ΔP_t（逐 bar：p_t（前 bar 收盘的净持仓）×（px_t − px_{t−1}）×
    //   |合约乘数=1|，units 已是名义手数）。首 bar 无前价 ⟹ 不计。
    let mut cum_price_pnl: f64 = 0.0;
    let mut prev_px: Option<f64> = None;
    // 强平罚金边沿触发（一次一集）：进入 {Insolvent,Liquidation} 且持仓 ⟹ 收一次罚金，
    // 置 true；离开强平态 ⟹ 复位。避免强平态跨 bar 持续（exec 延迟平仓期间）重复罚。
    let mut liq_active: bool = false;

    for i in 0..n {
        let bar = &bars[i];
        let px = bar.close as f64 * config.tick.tick_size;

        // ── M6 ①⁻ 价格 PnL 毛额累计（ΣN_tΔP_t）：本 bar 开头 units 是**前 bar 收盘持仓**
        //    （尚未经本 bar ① 成交），px−prev_px 是本 bar 价格变动 ⟹ 贡献 = units·Δpx。
        //    这是含浮盈的 MtM 价格贡献，与 equity_curve 的价格重估同源（守恒断言校验）。──
        if let Some(pp) = prev_px {
            if px > 0.0 {
                cum_price_pnl += units * (px - pp);
            }
        }

        // ── ① 延迟成交：本 bar 到达 exec_index 的挂单 fill（apply_order，先平后开）。 ──
        if !pending[i].is_empty() && !bar.untradable && px > 0.0 {
            let orders = std::mem::take(&mut pending[i]);
            for o in &orders {
                if o.qty > 0 {
                    let units_before = units;
                    // A'（清单③）：累加本 fill 的费后已实现 PnL（多 fill 同 bar 聚合——推导链第 9 条）。
                    let fill = apply_order(o, px, fee_rate, &mut cash, &mut units, &mut entry_cost, &mut trade_pnls);
                    realized_cum += fill.realized;
                    cum_fee += fill.fee;
                    if fill.executed_qty > 0.0 {
                        // 轨迹与执行计数只消费真实成交；全拒单不再伪造 L2 执行事实。
                        track_position_transition(&mut trades, &mut pos_entry_bar, units_before, units, i, false);
                        n_orders_executed += 1;
                    }
                }
            }
        }

        // ── M6 ①⁺ Funding + Borrow 持仓期成本计提（本 bar 成交后持仓的持有成本；px>0 才计——
        //    untradable bar（px=0）无有效 mark ⟹ 跳过计提，与 cum_price_pnl 的 px>0 累计口径
        //    一致，守恒才成立）。从 cash 扣除 ⟹ equity_curve 反映拖累。cost_model=None ⟹ 恒 0
        //    （bit-exact）。──
        if let Some(cost) = config.cost_model.as_ref() {
            if px > 0.0 && units != 0.0 {
                let net_notional_usd = units.abs() * px;
                let equity_pre = cash + units * px; // 计提前权益（借入名义 = max(0,|N|−E)）
                let funding = cost.funding_accrual(i, net_notional_usd);
                let borrow = cost.borrow_accrual(net_notional_usd, equity_pre);
                cash -= funding + borrow;
                cum_funding += funding;
                cum_borrow += borrow;
            }
        }

        // ── ② p_t = 净 lot（成交后真实持仓）。 ──
        let p_t = units;
        let current_nav = cash + units * px;
        let equity_nav = if current_nav > 0.0 { current_nav } else { nav0 };

        // ── ②' TW 成本划转（#124）：真实成本基（|units|·均价，空头取绝对额=在险市值）方向差分
        //    ⟹ ShortDiff 划转（free⇄holding，TW 守恒构造子）。shadow 追真实、入账钳制（cash-sound）。──
        {
            let basis_now = (units.abs() * entry_cost.abs()) as i64;
            let d = basis_now - tw_seen_basis;
            tw_seen_basis = basis_now;
            if d > 0 {
                let inflow = d.min(tw.free); // 买入 free→holding，不透支 free
                if inflow > 0 {
                    tw = tw_step(&tw, TwEvent::ShortDiff(-inflow));
                }
            } else if d < 0 {
                let outflow = (-d).min(tw.holding); // 卖出 holding→free，成本基回流
                if outflow > 0 {
                    tw = tw_step(&tw, TwEvent::ShortDiff(outflow));
                }
            }
        }

        // ── ②'' TW 已实现利润入账（codex GAP3 裁定 A' 清单③）：本 bar 平仓 fill 的费后 PnL
        //    聚合（推导链第 9 条：同 bar 多 fill 聚合）经 `TwEvent::Realize(d_pi)` 入 free。
        //    ★硬边界2：置于 ②' 成本基 ShortDiff **之后**、③ TwStepCtx（stage 判据）之前——
        //    同一成交先成本基后利润。★量化口径：对累计值取整再派差分（tw_seen_basis 同款
        //    shadow 模式）⟹ 截断误差有界不累积，TW 漂移恒 = ⌊realized_cum⌋（清单⑥不变量）。
        //    ★可正可负（推导链第 5 条）：亏损如实入账——负 d_pi 使 free 下降；若累计亏损超
        //    free（做空爆亏），free 为负是真实现金透支的诚实镜像（f64 账本 cash 同步为负），
        //    非静默账目错误——stage 门（free≥recover_target>0）在负 free 下恒不过 = 安全侧。
        {
            let realized_now = realized_cum as i64;
            let d_pi = realized_now - tw_seen_realized;
            if d_pi != 0 {
                tw_seen_realized = realized_now;
                tw = tw_step(&tw, TwEvent::Realize(d_pi));
            }
        }

        if !bar.untradable && px > 0.0 {
            // ── ③ [A] 前缀因果重分类（classify_at(i)=classify_with_tower(l0[0..=i]) → 因果塔 + 因果
            //      分类，只用 ≤i 数据 → 因果）+ 切当步候选 + [B] base_units U_ℓ + [C] thread + 风控门。 ──
            let (classification_i, tower_i, tower_gen, forest_epoch) = classify_at(i);
            // ★opsem-dump：diff tower_i vs prev_tower → 写塔事件（仅交易活跃区间，env-gated）。
            if let Some(dump) = opsem.as_mut() {
                dump.diff_tower(i, &tower_i);
            }
            // ★当步候选 = 前缀因果塔里**本 bar 新确认**的买卖点（append-only diff vs seen，确认-bar
            // 部署）——非 source_index==i 切片（买卖点回溯确认，其触发点常在更晚 bar 才入前缀塔 ⟹
            // source_index==i 切恒空 ⟹ 零订单）。买卖点在被确认那根 bar（source_index≤i）部署=因果。
            let classification_step = newly_confirmed_step(&classification_i, &mut seen_bsps);
            let base_units = equity_nav / px; // U_ℓ：NAV/价 = 可建名义手数（方案A协变）
            // 风控门也用**前缀因果分类**（leg 止损 bsp 因果查得，非全窗非因果——与 σ_p 同因果口径）。
            let (gate, risk_mode_i) = k_theta_risk_gate(&prev_active, &open_trades, bar, equity_nav, p_t, px, config.margin.as_ref());
            // ── M6 ③⁻ LiquidationLoss 强平罚金（边沿触发，一次一集）：本 bar 进入
            //    {Insolvent,Liquidation} 且持仓 ⟹ 收一次罚金（强平清算费/滑点），从 cash 扣。
            //    liq_active 边沿去抖：强平态跨 bar 持续（exec 延迟平仓期间）不重复罚；离开强平态复位。
            //    cost_model=None ⟹ 恒 0（bit-exact）。RiskExit 优先级由 gate.force_flat 上游承担
            //    （coverage P1 屏蔽 P2..P10），本罚金是该强平事件的**成本记账**，不改平仓触发逻辑。──
            {
                use super::super::strategy::risk::global_risk_close;
                let in_liq = global_risk_close(risk_mode_i);
                if let Some(cost) = config.cost_model.as_ref() {
                    if in_liq && !liq_active && p_t != 0.0 && px > 0.0 {
                        let penalty = cost.liquidation_penalty(p_t.abs() * px);
                        cash -= penalty;
                        cum_liq_loss += penalty;
                    }
                }
                liq_active = in_liq;
            }
            // ── A10 C5（裁定 (b) TW 桥 G1，零账本侵入）：`cum_holding_cost` i64 shadow =
            //    funding+borrow+liq **累计量化**（`tw_seen_basis`/`tw_seen_realized` 同款「对累计值
            //    量化」模式；f64→定点口径与 treasury `Realize(⌊realized_cum⌋)` 一致——向零截断，
            //    三项恒 ≥0 ⟹ 截断=⌊⌋）。G1 缺口：持盾成本只扣 f64 `cash`（①⁺/③⁻），TW 账本不经
            //    任何构造子见到它（GAP3 A' 构造子冻结不动）⟹ 未修正 η=tw() **高估**真实在险权益
            //    ⟹ enter_ready 易过=激进侧（不安全）。修正全部在**策略层消费侧**：η_bucket
            //    （ZExt 第 15 维）与 TwStepCtx.eta_correction 读**同一本变量**（F4 同源强制——
            //    裁定 C5 第 4 条：两处不同步则 z 维与判据裂口）。cost_model=None ⟹ 三项恒 0
            //    ⟹ shadow=0 ⟹ 与历史判据同值 bit-exact（回归锁）。──
            let cum_holding_cost: i64 = (cum_funding + cum_borrow + cum_liq_loss) as i64;
            // 桥接对账断言（裁定 C5 裁决1，取整界内）：η_corrected := tw() − cum_holding_cost
            // ⟹ tw() − η_corrected == ⌊cum_holding_cost⌋——此处断言 shadow 与 f64 真值的量化
            // 界 <1 个量化单位（三项恒 ≥0 保号；断言即断言，不降格为告警）。
            debug_assert!(
                cum_funding + cum_borrow + cum_liq_loss >= 0.0
                    && ((cum_funding + cum_borrow + cum_liq_loss) - cum_holding_cost as f64).abs() < 1.0,
                "A10 C5 桥接对账：tw()−η_corrected == ⌊cum_holding_cost⌋ 取整界破——shadow={} f64真值={}",
                cum_holding_cost,
                cum_funding + cum_borrow + cum_liq_loss
            );
            // G3（#138）z 第 10-13 维装配：π 路径候选不经 Nest/Xzd 准入门（cand_channel/nest_depth
            // None 诚实口径，origin_level 由 z_of_candidate 填 Some(c.level) 起始=执行真值）；
            // risk_mode = 当 bar 账本态真值。**同一 ext_i 同时喂 χ 查询（filter）与训练登记
            // （entry_z）** ⟹ 训练/查询同口径在共享变量层保证（G2 护航点同款）。
            // #149 第 14 维 t_stage = tw.stage 当 bar 决策点相位真值：②'/②'' 账本更新之后、
            // 本 bar stage_progression（step_trace.tw_event 写点）之前读取——与 TwStepCtx.state
            // 喂给 P2/P3/P4 谓词的是同一 tw 值，账本相位单一真值源（#124 裁定4）无第二口径。
            // #175 第 15 维 eta_bucket = γ_t 四桶（PDF §10 分段式；终裁 a5-etabucket-stance-
            // ruling-20260704.md：η_t=tw.tw() 即 enter_ready 判据左操作数、η_*=tw_policy.eta_star）
            // ——与 t_stage 读同一 tw 变量同一时点（非另开账本查询，零时序错位）。
            // ★A10 C5（F4 同源修正）：η 左操作数经 cum_holding_cost 修正（η_corrected =
            // tw() − cum_holding_cost）——与下方 TwStepCtx.eta_correction **同一变量同一时点**，
            // 裁定 C5 第 4 条「η_bucket 与 enter_ready 左操作数同源修正为一笔改动」的落点。
            let ext_i = super::selector::ZExt {
                risk_mode: Some(risk_mode_i),
                t_stage: Some(tw.stage),
                eta_bucket: Some(tw_policy.eta_bucket_eta_corrected(&tw, cum_holding_cost)),
                ..super::selector::ZExt::NONE
            };
            // A7（#165）：记录当前决策点账本态——窗口终点 censored Hold（主循环外）取此末值作 exit_z
            // 账本态维（末可交易 bar 决策点真值，与 censored 兑现价同 bar 口径）。
            last_ext = ext_i;
            // exec_index：延迟成交 bar（spec:50；尾部无可成交 bar ⟹ 不挂单）。
            let exec_index = fill_bar_index(i, bars, &config.exec);
            // 环5+6+7：pi_theta_step（父容器 σ_p=attach_bsp_to_tree(因果塔) + 风控门）→ (A_{t+1}, p*, O)。
            // 工位 K 性能：tree-prefix 缓存（§16，命中省 extract_elements 重建）。pi_theta_step 用
            // newly-confirmed step 候选；merge 用全 classification 候选——两者共享同一 tower tree-prefix 缓存。
            // ★热点② O(n²) 消除：tree=Rc::clone O(1)，candidate 段单独 Vec，ElementView 双段零拷贝。
            let (step_tree, step_candidates, step_gamma) = {
                let _g = classifier::stage_profile::stage("cand_build_step");
                interp::coverage_elements_and_gamma_with_tower_cached_gen(
                    &classification_step, &tower_i, &mut Some(&mut tree_cache), Some(tower_gen), Some(forest_epoch),
                )
            };
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
                // σ_higher 真值穿透（codex-q1 G2 护航点）：生产 χ 查询与训练表同经塔真值构 z——
                // 训练 Some/查询 None 的静默"未见类别"退化在此被接口封死。
                Some(ctx) => super::selector::filter_gamma_with_admission(
                    &step_gamma, ctx.est, ctx.theta, ctx.z_alpha, ctx.shrink_tau_sq,
                    ctx.treat_empty_as_pass, &tower_i, bars, &ext_i,
                ),
                None => step_gamma.clone(), // χ≡1：原候选集（bit-exact 不变）
            };
            // ── #124 裁定4 TW 谓词 ctx（P2/P3/P4 进 fold）：在飞腿 entry_v 映射从 typed ledger
            //    在飞表取（entry_v 入场固定，与 TW open_legacy_legs 计数同源）。#145 T1：由原
            //    ShortDiff id 半镜像升级为全量 entry_v 映射——P2 过滤在组合层按
            //    `== ShortDiff` 判（语义 bit-exact），且兼作反向关闭 typed 裁决的
            //    reverse_exit_type 原料（组合层单点，本处不再结算补算）；risk_mode 从
            //    strategy::risk 五态投影到 closed_loop::state 五态（两枚举同锚 Origin 五构造子，
            //    此处只读逐变体映射，非第二权威源——判定仍单源 k_theta_risk_gate）。 ──
            let entry_v_map: std::collections::HashMap<
                classifier::recursive_tower::ElementId,
                super::super::strategy::coverage::Vertical,
            > = open_trades.iter().map(|(id, o)| (*id, o.entry_v)).collect();
            let tw_risk_mode = {
                use super::super::closed_loop::state::RiskMode as ClRiskMode;
                use super::super::strategy::risk::RiskMode as StRiskMode;
                match risk_mode_i {
                    StRiskMode::Insolvent => ClRiskMode::Insolvent,
                    StRiskMode::Liquidation => ClRiskMode::Liquidation,
                    StRiskMode::Deleverage => ClRiskMode::Deleverage,
                    StRiskMode::CloseOnly => ClRiskMode::CloseOnly,
                    StRiskMode::Normal => ClRiskMode::Normal,
                }
            };
            let twc = coverage::TwStepCtx {
                state: &tw,
                policy: &tw_policy,
                risk_mode: tw_risk_mode,
                entry_v: &entry_v_map,
                // A10 C5（裁定 (b)）：enter_ready 的 η 左操作数同源修正（与上方 η_bucket 同一
                // cum_holding_cost 变量，F4）；0 ⟹ 历史判据 bit-exact（cost_model=None 回归锁）。
                eta_correction: cum_holding_cost,
            };
            // #82 DC-E：只从真实在飞 campaign 建父腿快照；结构 carrier、身份不明或零单位不冒充父腿。
            // sync 只替换 OscillationBook 的只读 parents 表，lots 原位保留（无平行账本）。
            let mut pan_candidates = Vec::new();
            let mut protocol_events = super::super::strategy::protocol::ProtocolEventSet::hold(0);
            if let Some(hist) = pan_div_hist.as_deref() {
                use super::super::strategy::oscillation::OscillationParentLeg;
                let parents = prev_active
                    .iter()
                    .filter_map(|leg| {
                        let open = open_trades.get(&leg.id)?;
                        if open.entry_v == super::super::strategy::coverage::Vertical::ShortDiff {
                            return None;
                        }
                        let units = open.units.round().max(0.0) as u64;
                        OscillationParentLeg::new(leg.id, leg.level, leg.dir, units).ok()
                    })
                    .collect();
                pan_div_state.sync_live_parents(parents);
                for (lvl, ls) in classification_i.levels.iter().enumerate() {
                    for cert in ls.pan_div.iter() {
                        // 首见先落 seen；门闭也终局，后续 bar 不重试（与统计通道 τin 同时序）。
                        if !pan_div_state.observe_raw(lvl as u32, cert) {
                            continue;
                        }
                        let sub_centers: &[super::super::types::Center] = if lvl > 0 {
                            &classification_i.levels[lvl - 1].centers
                        } else {
                            &[]
                        };
                        let sub_bsp: &[super::super::classifier::bsp::BspPoint] = if lvl > 0 {
                            &classification_i.levels[lvl - 1].bsp
                        } else {
                            &[]
                        };
                        let Some(gated) = super::econ_positive::gate_pan_div_for_production(
                            &tower_i,
                            lvl,
                            cert,
                            hist,
                            i,
                            &ls.bsp,
                            sub_centers,
                            sub_bsp,
                        ) else {
                            pan_div_state.note_gate_rejected();
                            continue;
                        };
                        match pan_div_state.prepare(gated) {
                            super::pan_div::PreparedPanDiv::Candidate(candidate) => {
                                // DA-Q2：即使订单槽稍后被 BSP/P1-P4 占用，PanDiv 证据仍留在协议轨。
                                protocol_events = protocol_events.with_center_oscillation(candidate);
                                pan_candidates.push(candidate);
                            }
                            super::pan_div::PreparedPanDiv::Record(_reason) => {
                                // P10 已在生产状态统计；无合法 parent/lot identity 时不伪造候选。
                            }
                        }
                    }
                }
            }
            let (next_active, standard_p_star, (mut order, _protocol_event), step_trace) = coverage::pi_theta_step_traced(
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
                Some(&twc),
                &protocol_events,
            );
            if pan_div_hist.is_some() {
                use super::super::strategy::oscillation::{
                    OscillationAction, OscillationApplyResult, OscillationIntent,
                    SizeKThetaProjection,
                };
                use super::super::strategy::voice::VoiceSide;

                // 真 BSP（非 struct_break 零 bits）与 P1-P4/标准腿生命周期优先；PanDiv 仅留上方
                // protocol confirmation，不重复占订单槽。
                let standard_bsp = classification_step.levels.iter().any(|level| {
                    level.bsp.iter().any(|point| point.bits.class_index() != 0)
                });
                let standard_lifecycle = !step_trace.opened.is_empty()
                    || !step_trace.closed.is_empty()
                    || !step_trace.silent_drops.is_empty()
                    || !step_trace.risk_exits.is_empty()
                    || !step_trace.overlay_closes.is_empty()
                    || step_trace.tw_event.is_some();
                let higher_priority = standard_bsp || standard_lifecycle;
                let mut actionable_pan = pan_div_state.pending_candidates().to_vec();
                actionable_pan.extend_from_slice(&pan_candidates);
                let selected = pan_div_state.select_for_bar(&actionable_pan, higher_priority);
                let cap = base_units.abs() * config.risk.gamma.abs();
                if let Some(candidate) = selected {
                    // KΘ 从“标准父腿目标 + 当前 live 子腿”这一实际组合目标算剩余空间；若从
                    // standard_p_star 单独算，父腿已在上限时会错误阻塞 P7 回补。
                    let pan_anchor = standard_p_star
                        + pan_div_state.signed_live_child_units() as f64;
                    let mut k_theta_units = gate.delta_capacity_units(
                        cap,
                        pan_anchor,
                        candidate.action_side(),
                    );
                    if candidate.intent() == OscillationIntent::Open {
                        // P9 必须是经济减仓：只允许把标准目标向 0 移动，不穿零反向净加仓。
                        let reduces = matches!(
                            (pan_anchor.is_sign_positive(), candidate.action_side()),
                            (true, VoiceSide::Short) | (false, VoiceSide::Long)
                        ) && pan_anchor != 0.0;
                        k_theta_units = if reduces {
                            k_theta_units.min(pan_anchor.abs().floor() as u64)
                        } else {
                            0
                        };
                    }
                    let applied = pan_div_state.apply(
                        candidate,
                        super::super::strategy::oscillation::CenterOscillationConfig {
                            enabled: true,
                        },
                        SizeKThetaProjection {
                            size_theta_units: candidate.target_units(),
                            k_theta_units,
                        },
                    );
                    if matches!(
                        applied,
                        OscillationApplyResult::Applied {
                            action: OscillationAction::OpenShortDiff | OscillationAction::CloseShortDiff,
                            ..
                        }
                    ) {
                        let target = gate.clamp_position(
                            cap,
                            standard_p_star + pan_div_state.signed_live_child_units() as f64,
                        );
                        order = coverage::schedule_order(target, p_t, exec_index.unwrap_or(i));
                    }
                } else if !higher_priority && pan_div_state.signed_live_child_units() != 0 {
                    // 无新 cert 的持有 bar：把 live ShortDiff 投影叠回标准目标，避免下一 bar 被基础 π
                    // 当作偏差自动回补；仍经同一 KΘ 区间 clamp 和 ScheduleΘ 单一订单出口。
                    let target = gate.clamp_position(
                        cap,
                        standard_p_star + pan_div_state.signed_live_child_units() as f64,
                    );
                    order = coverage::schedule_order(target, p_t, exec_index.unwrap_or(i));
                }
            }
            // ── ★M5 overlay 簿步进（多空对冲.pdf p16 关卡10）：sep_legs=P^sep_{t+1} 目标 → hedge-mode
            //    逐声部账本 → ΔN 订单 + 逐声部 pnl_v 累计。只读旁路（不改净额 fill 的 cash/units/
            //    trade_pnls ⟹ 现有臂 bit-exact）。overlay=None（现有臂）⟹ 整段跳过。 ──
            if let Some(ov) = overlay.as_deref_mut() {
                let ostep = ov.step(&step_trace.sep_legs, px, i, config.risk.default_lot.max(1) as i64);
                // ★ΔN 守恒（验收断言1，PDF p16 `Order_t=N_t−N_{t−1}`）：overlay 订单恒 = 净敞口增量。
                debug_assert_eq!(
                    ostep.order,
                    ostep.net_after - ostep.net_before,
                    "M5 ΔN 守恒违例：order={} ≠ N_t−N_{{t−1}}={}−{}",
                    ostep.order, ostep.net_after, ostep.net_before
                );
            }
            // ── ③'' G4 typed ledger（#134）：消费 StepTrace 腿级生命周期事件。 ──
            // 开腿：准入信号腿登记（z 塔真值，与生产 χ 查询同经 z_of_candidate——训练/查询同口径）。
            // A6（#159）：z_of_candidate 内读 c.force（Candidate 透传 BspPoint.force）填 force_state
            // 第 8 维——entry_z（训练）与上方 filter_gamma（查询）同函数同候选 ⟹ 同口径自动成立，
            // fullz 置换 records 的 force_state 自此携真值（一类 A/C 对候选 Some）。
            for (c, leg) in &step_trace.opened {
                use super::super::strategy::interp::{EntryCertificate, PositionNodeId};
                // ★A9 generation：carrier 首次入场 = 0；close→reopen（表中已有）= 高水位 +1（单调）。
                let generation = match gen_hiwater.get(&leg.id) {
                    Some(&hi) => hi + 1,
                    None => 0,
                };
                gen_hiwater.insert(leg.id, generation);
                let position_node_id = PositionNodeId {
                    carrier: leg.id,
                    entry_certificate: Some(EntryCertificate {
                        level: c.level,
                        source_index: c.source_index,
                    }),
                    side: c.dir,
                    generation,
                };
                // ★A6（prereg-rev2-20260704）：入场结构止损距离 d=|entry_px−stop|（美元，ex-ante）。
                // 决策 bar 因果分类 classification_i 查开腿候选 BspPoint（(level,source_index) 键，与
                // k_theta_risk_gate 止损回查同源 structural_stop），stop_side 由候选方向定。None =
                // structural_stop 返 None（非该方向交易点）/ BspPoint 缺失 ⟹ μ_R 剔除（诚实缺口）。
                // ★族A：入场一次性冻结 structural_stop（Tick）到 LedgerOpen.entry_stop，逐 bar 风控门
                // 读此冻结值（消除旧路径 drifted leg.source_index 回查 ⟹ 静默跳过 stop）。dist 同源导出。
                let entry_stop = entry_structural_stop(c, &classification_i);
                let entry_stop_dist =
                    entry_stop.map(|stop| (px - stop as f64 * config.tick.tick_size).abs());
                // ★opsem-dump：入场时刻操作语义快照（env-gated，未启用零字段零开销）。
                let opsem_snap = if opsem.is_some() {
                    let (sa_macd, sc_macd, sa_dif, sc_dif, fstate) = match c.force {
                        Some(fp) => (
                            Some(fp.seg_a.macd_area),
                            Some(fp.seg_c.macd_area),
                            Some(fp.seg_a.dif_peak),
                            Some(fp.seg_c.dif_peak),
                            c.force.as_ref().map(|fp| force_state_str(fp.force_state())),
                        ),
                        None => (None, None, None, None, None),
                    };
                    let pid = leg.parent_id.map(|p| (p.level, p.ordinal));
                    if let Some(dump) = opsem.as_mut() {
                        dump.mark_entry(i);
                    }
                    OpsemEntrySnapshot {
                        cand_level: c.level,
                        cand_source_index: c.source_index,
                        cand_bits: c.bits.class_index(),
                        cand_dir: voice_side_str(c.dir),
                        cand_bsp_class: c.bsp_class,
                        cand_nest_confirmed: c.nest_confirmed,
                        // R5-c：区间套深度（纯结构读数，与生产门 rungs.len() 同口径，不依赖 hist）。
                        nest_depth: super::econ_positive::structural_nest_depth(
                            &tower_i, c.level as usize, c.source_index,
                        ),
                        cand_role: Box::leak(operation_role_str(c.role).into_boxed_str()),
                        seg_a_macd_area: sa_macd,
                        seg_c_macd_area: sc_macd,
                        seg_a_dif_peak: sa_dif,
                        seg_c_dif_peak: sc_dif,
                        force_state: fstate,
                        gamma_count: step_gamma_trade.len(),
                        prev_active_count: prev_active.len(),
                        // R5-a：本步 LexArgmin top-3（step 级，opened 内共享；traced 正常路径填充）。
                        lex_top3: step_trace.lex_top3.clone(),
                        parent_id: pid,
                        t_stage: t_stage_str(tw.stage),
                        eta_bucket: ext_i.eta_bucket.map(eta_bucket_str).unwrap_or("null"),
                        risk_mode: ext_i.risk_mode.map(risk_mode_str).unwrap_or("null"),
                    }
                } else {
                    OpsemEntrySnapshot::default()
                };
                // ★B1（步骤4，codex review conditional 修复）：入场 sizing target 快照（开腿当步
                // SepLeg.q_units，含 dir_weight）。sep_legs 经 coverage.rs:2317 filter_map(work.get) 构造 ⟹
                // opened 腿 id 通常在其中，但 filter_map 可跳过 work 不含的 e_idx，理论非 100% 保证。
                // debug_assert 抓测试期不变量违例；release 防御性 0.0（μ 不读 units ⟹ 不破坏 μ；
                // execution 诊断见 0.0 = sizing 信息缺失信号，**非真实 sizing=0**）。
                let b1_sep = step_trace.sep_legs.iter().find(|s| s.id == leg.id);
                debug_assert!(
                    b1_sep.is_some(),
                    "B1: opened leg {:?} not in step_trace.sep_legs (coverage work.get 跳过？)",
                    leg.id
                );
                open_trades.insert(leg.id, LedgerOpen {
                    entry_bar: i,
                    entry_px: px,
                    entry_stop,
                    entry_stop_dist,
                    // G3：与本 bar χ 查询共用同一 ext_i（训练/查询同口径，共享变量层保证）。
                    entry_z: super::selector::z_of_candidate(c, &tower_i, bars, &ext_i),
                    entry_v: c.role.v,
                    position_node_id,
                    opsem: opsem_snap,
                    units: b1_sep.map(|s| s.q_units).unwrap_or(0.0),
                });
            }
            // 反向关闭：typed 裁决消费 trace 第三分量（#145 T1——组合层决策点已经
            // reverse_exit_type 单源判定，本处不补算；登记腿必在 entry_v 映射 ⟹ 裁决非回退值）。
            for (leg, trig, exit_type) in &step_trace.closed {
                if let Some(open) = open_trades.remove(&leg.id) {
                    // #145 T1 不变量：登记腿必在本步 entry_v 映射（closed ⊆ prev_active ⊆ 本 bar
                    // 快照 open_trades），组合层裁决非 Ambient 回退值。映射构建与消费之间若未来
                    // 插入 open_trades 突变，此断言先炸而非静默错型。
                    debug_assert!(
                        entry_v_map.contains_key(&leg.id),
                        "#145 T1：trace.closed 腿 {:?} 不在本步 entry_v 映射（裁决为回退值）",
                        leg.id
                    );
                    if open.entry_v == super::super::strategy::coverage::Vertical::ShortDiff
                        && tw.open_legacy_legs >= 1
                    {
                        tw = tw_step(&tw, TwEvent::CloseShareLeg(0)); // TW 腿计数（#124）
                    }
                    let pushed = TypedTrade {
                        entry_z: open.entry_z,
                        voice_id: leg.id,
                        entry_bar: open.entry_bar,
                        exit_bar: i,
                        exit_type: *exit_type,
                        entry_px: open.entry_px,
                        exit_px: px,
                        via_structural_prune: false, // 真信号平仓（反向候选触发）
                        position_node_id: open.position_node_id,
                        entry_stop_dist: open.entry_stop_dist, // A6：入场止损距离 d（μ_R 分母）
                        exit_z: super::selector::exit_z_of(open.entry_z, &ext_i), // A7 #165：出场时刻 z 快照
                        units: open.units, // B1 步骤4：腿级 sizing 透传（不进 μ estimand）
                    };
                    // ★opsem-dump：反向关闭外化（trig.bsp_class = 触发候选类）。
                    if let Some(dump) = opsem.as_mut() {
                        dump.mark_exit(i);
                        let _ = dump.write_trade(&pushed, &open, Some(trig.bsp_class));
                    }
                    typed_ledger.push(pushed);
                }
                // 表中无登记（本窗开跑前已持/restore 祖先腿）⟹ 非本窗信号入场，不入 ledger。
            }
            // 静默离场（§13 AncOK 连带剪/Stale prune，无触发信号）：子声部随父失效 ⟹
            // CloseShortDiff；根腿结构失效 ⟹ CloseRoot（PDF §9 五枚举全集下的最近语义归置，
            // 判据声明见 g4-impl 结果包边界条件）。
            for leg in &step_trace.silent_drops {
                if let Some(open) = open_trades.remove(&leg.id) {
                    use super::super::strategy::coverage::Vertical;
                    use super::super::strategy::interp::ExitType;
                    if open.entry_v == Vertical::ShortDiff && tw.open_legacy_legs >= 1 {
                        tw = tw_step(&tw, TwEvent::CloseShareLeg(0)); // TW 腿计数（#124）
                    }
                    let exit_type = if open.entry_v != Vertical::Ambient {
                        ExitType::CloseShortDiff
                    } else {
                        ExitType::CloseRoot
                    };
                    let pushed = TypedTrade {
                        entry_z: open.entry_z,
                        voice_id: leg.id,
                        entry_bar: open.entry_bar,
                        exit_bar: i,
                        exit_type,
                        entry_px: open.entry_px,
                        exit_px: px,
                        via_structural_prune: true, // §13 父驱动连带剪枝，非独立信号（μ 侧可分离）
                        position_node_id: open.position_node_id,
                        entry_stop_dist: open.entry_stop_dist, // A6：入场止损距离 d（μ_R 分母）
                        exit_z: super::selector::exit_z_of(open.entry_z, &ext_i), // A7 #165：出场时刻 z 快照
                        units: open.units, // B1 步骤4：腿级 sizing 透传（不进 μ estimand）
                    };
                    // ★opsem-dump：静默离场外化（无触发候选，trigger_bsp_class=null）。
                    if let Some(dump) = opsem.as_mut() {
                        dump.mark_exit(i);
                        let _ = dump.write_trade(&pushed, &open, None);
                    }
                    typed_ledger.push(pushed);
                }
            }
            // 强平清空（#124 P1，PDF §7 C_1 屏蔽 P2..P10）：force_flat ⟹ prev_active 全部 RiskExit
            // （无触发候选；pi_theta_step_traced 上游短路清空 next_active，见 StepTrace.risk_exits）。
            for leg in &step_trace.risk_exits {
                if let Some(open) = open_trades.remove(&leg.id) {
                    if open.entry_v == super::super::strategy::coverage::Vertical::ShortDiff
                        && tw.open_legacy_legs >= 1
                    {
                        tw = tw_step(&tw, TwEvent::CloseShareLeg(0));
                    }
                    let pushed = TypedTrade {
                        entry_z: open.entry_z,
                        voice_id: leg.id,
                        entry_bar: open.entry_bar,
                        exit_bar: i,
                        exit_type: super::super::strategy::interp::ExitType::RiskExit,
                        entry_px: open.entry_px,
                        exit_px: px,
                        via_structural_prune: false, // 强平=风险信号平仓，非结构剪枝
                        position_node_id: open.position_node_id,
                        entry_stop_dist: open.entry_stop_dist, // A6：入场止损距离 d（μ_R 分母）
                        exit_z: super::selector::exit_z_of(open.entry_z, &ext_i), // A7 #165：出场时刻 z 快照
                        units: open.units, // B1 步骤4：腿级 sizing 透传（不进 μ estimand）
                    };
                    // ★opsem-dump：强平外化（无触发候选，trigger_bsp_class=null）。
                    if let Some(dump) = opsem.as_mut() {
                        dump.mark_exit(i);
                        let _ = dump.write_trade(&pushed, &open, None);
                    }
                    typed_ledger.push(pushed);
                }
            }
            // P2 CloseOverlay（#124 裁定4，PDF §7 C_2）：TW StageII 重叠腿关闭——真实订单已经
            // 同一 schedule/fill（组合层合成 close 桶复用 𝒟_x 通道）；typed 归 CloseShortDiff
            // （关的正是 legacy ShortDiff 重叠腿，PDF §9 五枚举内最近语义）。
            for leg in &step_trace.overlay_closes {
                if let Some(open) = open_trades.remove(&leg.id) {
                    if open.entry_v == super::super::strategy::coverage::Vertical::ShortDiff
                        && tw.open_legacy_legs >= 1
                    {
                        tw = tw_step(&tw, TwEvent::CloseShareLeg(0));
                    }
                    let pushed = TypedTrade {
                        entry_z: open.entry_z,
                        voice_id: leg.id,
                        entry_bar: open.entry_bar,
                        exit_bar: i,
                        exit_type: super::super::strategy::interp::ExitType::CloseShortDiff,
                        entry_px: open.entry_px,
                        exit_px: px,
                        via_structural_prune: false, // TW 账本谓词驱动的真实平仓，非结构剪枝
                        position_node_id: open.position_node_id,
                        entry_stop_dist: open.entry_stop_dist, // A6：入场止损距离 d（μ_R 分母）
                        exit_z: super::selector::exit_z_of(open.entry_z, &ext_i), // A7 #165：出场时刻 z 快照
                        units: open.units, // B1 步骤4：腿级 sizing 透传（不进 μ estimand）
                    };
                    // ★opsem-dump：P2 overlay 关闭外化（无触发候选，trigger_bsp_class=null）。
                    if let Some(dump) = opsem.as_mut() {
                        dump.mark_exit(i);
                        let _ = dump.write_trade(&pushed, &open, None);
                    }
                    typed_ledger.push(pushed);
                }
            }
            // TW 腿事件（#124）：legacy ShortDiff 腿开仓驱动 open_legacy_legs 计数（P2 的 H
            // 判据与生产腿同源同步；关侧在上方四个消费循环内经 open.entry_v 判定派
            // CloseShareLeg）。CloseShareLeg(0) 口径声明：净额架构无腿级损益分账 ⟹ profit
            // 口径量 0 承载（cum_net_cash 非承重分量——P2/P3/P4 谓词不消费它；唯一承重 =
            // open_legacy_legs 计数），非簿记伪造。
            // OQ-9 守卫：EarningShares 阶段开 legacy 腿 PDF 定义为非法（is_legal_from）——
            // A' 后该 stage 生产可达（已实现利润入账，见 TW 初始化注释）；达 earning 后此腿
            // 不计 legacy 计数，关侧 legs>=1 守卫对称跳过（合法性语义，非掩盖）。
            for (c, _leg) in &step_trace.opened {
                if c.role.v == super::super::strategy::coverage::Vertical::ShortDiff
                    && TwEvent::OpenShareLeg.is_legal_from(&tw)
                {
                    tw = tw_step(&tw, TwEvent::OpenShareLeg);
                }
            }
            // P3/P4 TWEvent_t（#124 裁定4）：账本推进单点（组合层只读产出事件分量，此处是
            // 生产 π 内唯一的 stage 推进写点——stage_progression 派生事件生产恒合法）。
            if let Some(ev) = step_trace.tw_event {
                debug_assert!(
                    ev.is_legal_from(&tw),
                    "stage_progression 派生事件恒合法（OQ-9 生产不变量）"
                );
                tw = tw_step(&tw, ev);
            }
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
                let (tree_ref, candidates_ref) = {
                    let _g = classifier::stage_profile::stage("cand_build_merge");
                    interp::coverage_elements_with_tower_cached_gen(
                        &classification_i, &tower_i, &mut tree_cache, &mut cand_cache, Some(tower_gen), Some(forest_epoch),
                    )
                };
                classifier::stage_profile::record_span("cand_count", candidates_ref.len() as u64);
                let cand_dirty = cand_cache.dirty; // ★on2w3：candidates 逐字节变？否 ⟹ merge cand 段跳过。
                // ★工位 4f：双段 merge（消 as_contiguous materialize O(tree) + step 1'/2' tree 全量 O(tree)）。
                // tree_dirty=false（Rc::ptr_eq 命中，tree 同上 bar）⟹ 跳过 tree 段（断言1-3 bit-exact）。
                let tree_dirty = prev_merge_tree
                    .as_ref()
                    .map(|p| !std::rc::Rc::ptr_eq(p, &tree_ref))
                    .unwrap_or(true);
                {
                    let _g = classifier::stage_profile::stage("cand_merge_consume");
                    registry.merge_in_place_split(&tree_ref, tree_dirty, &candidates_ref, cand_dirty, &next_active);
                }
                prev_merge_tree = Some(tree_ref);
            }
            prev_active = next_active;
        }

        // ── ⑥ 权益曲线（mark-to-market，归一化 ÷nav0）。 ──
        equity_curve.push((cash + units * px) / nav0);
        // M6：记录本 bar 有效价供下 bar 价格 PnL 差分（px>0 才更新——untradable/零价 bar 不刷，
        // 避免 Δpx 跨越无效价产生伪价格贡献）。
        if px > 0.0 {
            prev_px = Some(px);
        }
    }

    // ── ★M5 overlay 窗口终点强平：全部活动声部按末可交易 bar close 离场（记 exit_v/冻结 pnl_v）。
    //    价格 PnL 已在末决策点 step 累计到末价 ⟹ 此处只搬账本行（含浮盈口径，与净额侧同理）。 ──
    if let Some(ov) = overlay.as_deref_mut() {
        if let Some(last_i) = (0..n).rev().find(|&j| !bars[j].untradable && bars[j].close > 0) {
            let last_px = bars[last_i].close as f64 * config.tick.tick_size;
            ov.force_flat(last_px, last_i);
        }
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

    // ── G4 typed ledger 窗口终点 censored（PDF §9 P0 Hold）：未离场腿兑现到末可交易 bar
    //    close（与旧 build_mu_from_bars censored 语义/窗口终点强平含浮盈同理，不偷看窗外）。──
    if let Some(last_i) = (0..n).rev().find(|&j| !bars[j].untradable && bars[j].close > 0) {
        let last_px = bars[last_i].close as f64 * config.tick.tick_size;
        // 确定序输出（HashMap 迭代序不定 ⟹ 按 (entry_bar, voice_id) 排序，bit-exact 可复现）。
        let mut censored: Vec<(classifier::recursive_tower::ElementId, LedgerOpen)> =
            open_trades.drain().collect();
        censored.sort_by_key(|(id, o)| (o.entry_bar, id.level, id.ordinal));
        for (id, open) in censored {
            let pushed = TypedTrade {
                entry_z: open.entry_z,
                voice_id: id,
                entry_bar: open.entry_bar,
                exit_bar: last_i,
                exit_type: super::super::strategy::interp::ExitType::Hold,
                entry_px: open.entry_px,
                exit_px: last_px,
                via_structural_prune: false, // censored 窗口边界，非结构剪枝
                position_node_id: open.position_node_id,
                entry_stop_dist: open.entry_stop_dist, // A6：入场止损距离 d（μ_R 分母）
                // A7 #165：censored 出场账本态取末决策点 last_ext（末可交易 bar 决策点真值，
                // 与 last_px 兑现价同 bar 口径）；全窗无决策点 ⟹ ZExt::NONE 账本态维诚实 None。
                exit_z: super::selector::exit_z_of(open.entry_z, &last_ext),
                units: open.units, // B1 步骤4：腿级 sizing 透传（不进 μ estimand）
            };
            // ★opsem-dump：censored Hold 外化（无触发候选，trigger_bsp_class=null）。
            if let Some(dump) = opsem.as_mut() {
                dump.mark_exit(last_i);
                let _ = dump.write_trade(&pushed, &open, None);
            }
            typed_ledger.push(pushed);
        }
    }

    // ── M6 R 分解组装（路线.pdf p16 第十一关）+ 账目守恒断言 ──
    // ledger_delta 从**实际账本**（cash/units 经 apply_fill + 成本扣减独立演化）测得的净变动，
    // net_r 从**独立累计器**（cum_price_pnl 在循环顶部按 units·Δpx 累加、cum_fee 由 apply_fill
    // 独立返回、funding/borrow/liq 独立累计）组装——两条独立路径应恒等（守恒），残差 ≈0 是
    // 「资金无泄漏」的物证。final MtM 用 prev_px（末有效价）避免末 bar untradable（px=0）伪归零。
    let final_px = prev_px.unwrap_or(0.0);
    let final_equity_abs = cash + units * final_px;
    let ledger_delta = final_equity_abs - nav0;
    // A10 C5 TW 桥对账行：= ⌊cum_funding+cum_borrow+cum_liq_loss⌋（与循环内 shadow 同一量化
    // 口径——对累计值截断，三项恒 ≥0 ⟹ 截断=⌊⌋）；cost_model=None ⟹ 0（bit-exact）。
    let tw_holding_cost_bridge: i64 = (cum_funding + cum_borrow + cum_liq_loss) as i64;
    let r_decomp = super::super::strategy::risk::RDecomposition::assemble(
        cum_price_pnl, cum_fee, cum_funding, cum_borrow, cum_liq_loss, ledger_delta,
        tw_holding_cost_bridge,
    );
    // 守恒断言（no-patch-mentality：残差超容差 = 真实资金泄漏 bug，不静默）。容差按名义规模缩放
    // （f64 累加 O(n) 舍入；nav0 量级 + 累计项量级）——绝对容差 max(1e-6, 1e-9·(|nav0|+|price_pnl|)）。
    let cons_tol = 1e-6_f64.max(1e-9 * (nav0.abs() + cum_price_pnl.abs()));
    debug_assert!(
        r_decomp.conservation_residual.abs() <= cons_tol,
        "M6 R 分解守恒残差 {} 超容差 {}（net_r={} vs ledger_delta={}）——资金泄漏",
        r_decomp.conservation_residual, cons_tol, r_decomp.net_r, ledger_delta
    );

    let daily_returns = bar_returns(&equity_curve);
    FillOutput {
        equity_curve,
        daily_returns,
        trade_pnls_realized: trade_pnls,
        trade_pnls_with_forced,
        trades,
        n_orders: n_orders_executed,
        typed_ledger,
        tw_final: Some(tw),
        r_decomp: Some(r_decomp),
    }
}

/// ★族A 修复：入场结构止损值（[`super::super::types::Tick`]，开仓 bar 一次性算 + 冻结到
/// [`LedgerOpen::entry_stop`]）。
///
/// 在决策 bar 因果分类 `classification` 上按候选 `(c.level, c.source_index)` 查 [`BspPoint`]——这是
/// **bsp 确认点坐标**（稳定，不漂移），方向由候选 `dir` 定（Long→pivot_low / Short→pivot_high，
/// 3 类→center）。`None` = structural_stop 返 None（非该方向交易点）/ BspPoint 缺失。
///
/// 与逐 bar 止损门 [`k_theta_risk_gate`] 共享同一 `structural_stop` 真值——本函数在**入场时**用候选
/// 坐标查得，冻结后供逐 bar 门读出（消除旧路径用 drifted `leg.source_index` 回查的覆盖度缺陷）。
fn entry_structural_stop(
    c: &super::super::strategy::interp::Candidate,
    classification: &super::super::classifier::Classification,
) -> Option<super::super::types::Tick> {
    use super::super::strategy::risk::{structural_stop, StopInput, StopSide};
    use super::super::strategy::voice::VoiceSide;
    use super::super::types::Center;
    let stop_side = match c.dir {
        VoiceSide::Long => StopSide::Long,
        VoiceSide::Short => StopSide::Short,
        VoiceSide::Flat => return None, // Flat 候选不开仓，无止损可言
    };
    let lvl = classification.levels.get(c.level as usize)?;
    let bsp = lvl.bsp.iter().find(|p| p.source_index == c.source_index)?;
    let stop_in = StopInput {
        pivot_low: bsp.pivot_low,
        pivot_high: bsp.pivot_high,
        center: bsp
            .center
            .unwrap_or(Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 0, end_index: 0 }),
    };
    structural_stop(stop_side, &bsp.bits, &stop_in)
}

/// ★A6（prereg-rev2-20260704）：开腿候选的入场结构止损距离 `d=|entry_px−stop|`（美元，ex-ante）。
/// μ_R 分母，non-Some ⟹ 下游剔除（231号）。族A 修复：薄包 [`entry_structural_stop`]（同源
/// `structural_stop`，单次算），距离从冻结 stop 值导出（与 [`LedgerOpen::entry_stop`] 同源）。
fn candidate_stop_dist(
    c: &super::super::strategy::interp::Candidate,
    classification: &super::super::classifier::Classification,
    entry_px: f64,
    tick_size: f64,
) -> Option<f64> {
    let stop = entry_structural_stop(c, classification)?;
    Some((entry_px - stop as f64 * tick_size).abs())
}

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
            let gen = classifier_incr.tower_generation();
            let fe = classifier_incr.forest_epoch(); // ★on2w2：K_i O(1) 命中判据。
            (cls, tower, gen, fe)
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
/// - `tw_state`（TW 守恒 + stage 单向）每 bar 经 tw_step 更新（ShortDiff / RecoverCapital）。
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

/// 腿级 typed 交易记录（G4 #134：《完整的策略.pdf》§9 typed exit 的统计层载体）。
///
/// 由**生产 π fill loop**（[`pi_theta_fill_loop`]，唯一状态机）逐腿输出——腿进 `next_active`
/// 开条目、腿离场（interpret 规则2 反向关闭 / §13 AncOK 剪 / 窗口终点 censored）关条目，
/// `exit_type` 经 [`super::super::strategy::interp::reverse_exit_type`] 单源判据产出。
/// 下游 `build_mu_from_bars` 从本记录构造 `MuObservation`/`ResidualTrade`（替换 PDF §9
/// 点名废弃的 τ^reverse 平行简化状态机，codex-q1-spec G4 终裁）。
///
/// **价格口径**：`entry_px`/`exit_px` = 腿进/出 active 的**决策 bar close**（F_t 可测，名义
/// 单位口径，与旧训练路径同价格语义——G4 只改出场时点规则，不改价格口径）；非账户 fill 价
/// （腿级无独立成交，净持仓聚合后账户级 P&L 归 equity_curve 路径）。
///
/// **RiskExit 通道**（#124 P1 已落地）：`force_flat` ⟹ 组合层上游短路清活动腿，经
/// `StepTrace.risk_exits` 产 `RiskExit`（幽灵腿堵口）。**CloseOverlay 通道**（#124 裁定4）：
/// TW StageII 重叠腿经 `StepTrace.overlay_closes` 产 `CloseShortDiff`（生产触发可达性受
/// 账本语义约束，见 fill loop TW 初始化注释）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypedTrade {
    /// 开腿候选的全互斥分类 z（`z_of_candidate` 塔真值，与生产 χ 查询同口径）。
    pub entry_z: super::mu_estimator::MuClass,
    /// 腿声部身份（`ActiveLeg.id`，跨 bar 稳定 ElementId）。
    pub voice_id: classifier::recursive_tower::ElementId,
    /// 腿进 active 的决策 bar。
    pub entry_bar: usize,
    /// 腿离场决策 bar（`Hold` censored = 窗口末可交易 bar）。
    pub exit_bar: usize,
    /// PDF §9 typed exit（interp.rs 单源五枚举）。
    pub exit_type: super::super::strategy::interp::ExitType,
    /// 入场决策 bar close（×tick，名义单位口径）。
    pub entry_px: f64,
    /// 离场决策 bar close（×tick）。
    pub exit_px: f64,
    /// 离场是否来自 §13 结构剪枝（AncOK 连带剪/Stale prune，父驱动同 bar 连带、非独立反向
    /// 信号触发）——ws-g5interp flag：结构剪枝的 P&L 分布与真信号平仓不同，混桶偏 μ。本标记
    /// 使 μ 侧**可分离**；`build_mu_from_bars` 现口径仍全部入 μ（排除/分桶是新统计决策，
    /// 归 #135 prereg，不在此静默改口径）。反向关闭/RiskExit/censored Hold 恒 false。
    pub via_structural_prune: bool,
    /// ★A9（Task #166，级别容器.pdf p14/§13）：position instance 严格身份四元组
    /// `hash(carrier, entry_certificate, side, generation)`。同一 carrier（`voice_id`）先后多次
    /// campaign 由 `generation` 单调区分——`voice_id` 会碰撞（close→reopen 复用同 ElementId），
    /// `position_node_id` 不碰撞（generation +1）。供跨笔同 carrier campaign 归因/去重（当前 μ 层
    /// 按 `entry_z` 逐笔独立观测，不消费本字段——它是身份完备性的账本层载体，非 μ 统计输入）。
    pub position_node_id: super::super::strategy::interp::PositionNodeId,
    /// ★A6（prereg-rev2-20260704，codex-ruling-696）：入场结构止损距离 d=|entry_px−stop|（美元）。
    /// μ_R=E[Y/d] co-primary 门的分母，ex-ante 可得（risk.rs structural_stop 在决策 bar 因果算）。
    /// `None` = 不可得（μ_R 剔除，raw μ 保留）。透传自 `LedgerOpen::entry_stop_dist`，
    /// `build_mu_from_bars` 塞进 [`ResidualTrade::d`](super::mu_estimator::ResidualTrade)。
    pub entry_stop_dist: Option<f64>,
    /// ★A7（Task #165，《完整的策略.pdf》§6 z「两次快照」+ §9 typed exit）：出场时刻的 z 快照。
    ///
    /// 与 `entry_z` 是**同一 [`MuClass`](super::mu_estimator::MuClass) 类型的两次快照**（时刻不同、
    /// 账本态不同）：结构身份维（level/δ/i_class/parent_dir/horizontal/force_state/σ_higher/门链维）
    /// 入场冻结不重采样（PDF §6 `σ_higher: 入场时上级方向`——按定义入场值），唯一逐 bar 变化的账本态
    /// 三元 `{t_stage, eta_bucket, risk_mode}` 刷新到出场 bar 决策点真值。由 [`super::selector::exit_z_of`]
    /// 单源构造（`entry_z` + 出场 bar `ext_i`），构造被迫唯一（在无触发候选的出场点重分类结构维 =
    /// 伪造不存在的候选 = 声明膨胀，231号）。
    ///
    /// **消费侧未定（A7 裁量分离）**：μ 估计器按 `(entry_z, exit_z, exit_type)` 分桶的语义是设计裁量，
    /// 待 codex 裁决——当前 μ 层不消费本字段（`build_mu_from_bars` 仍按 `entry_z` 逐笔独立观测），
    /// 本字段是出场侧完备性的账本层载体（同 `position_node_id` A9 先例）。
    pub exit_z: super::mu_estimator::MuClass,
    /// ★B1（步骤4，dw-sizing-diag-20260705，codex review conditional 修复）：**入场时刻 sizing
    /// target 快照**——开腿当步 `SepLeg.q_units`（=`base_units×w_depth×w_dir`，含 dir_weight；post
    /// gross-cap；coverage.rs:2321 从 `LegTarget.units` 透传）。**非逐 bar fill 后实际腿级持仓**——是
    /// 入场决策点的目标单位，不是执行期 fill 累计。**不进 μ estimand**（μ 保持单位边际 qty=1.0，696 域，
    /// `build_mu_from_bars` 不读本字段）。
    ///
    /// **诊断边界（codex review）**：本字段供"入场 sizing target 逐笔分布"诊断——对比 μ 单位边际
    /// qty=1.0，看 dir_weight 改变了哪些腿的入场规模。**账户级 execution R 分解由 `r_decomp` 负责**
    /// （runner.rs:1489 `RDecomposition::assemble`，`cum_price_pnl=Σ units·Δpx` 真实账户 sizing 加权），
    /// 本字段**不用于逐笔 execution P&L**——真要逐笔 execution P&L 需 per-bar exposure / fill ledger，
    /// 非本字段（本字段仅入场 target 快照）。
    pub units: f64,
}

/// ★`TypedTrade` 账本行 schema 版本（每次账本层增列 +1；显式版本标记）。
///
/// **版本史**：v3 增 `exit_z`（出场 z 快照，A7 #165）；v4 增 `units`（腿级 sizing 目标，B1 步骤4）。
///
/// **B30 prereg 前置清单联动声明（不静默，team-lead 令）**：μ **样本** schema
/// （[`MuObservation`](super::mu_estimator::MuObservation) = `{class, x_gamma}`）**未变**——
/// A7（`exit_z`）/B1（`units`）只在账本层增列，μ 消费侧仍按 `entry_z` 单位边际（qty=1.0，696 域）。
/// 任何后续把 `exit_z`/`units` 接入 μ 分桶键或 sizing 加权的工位（codex 裁决后）须新开 prereg
/// 冻结口径，不得静默改 estimand（formalization-validity-domain / B30 前置清单 / 696）。
pub const TYPED_TRADE_SCHEMA_VERSION: u32 = 4;

/// ledger 在飞条目（开腿登记，关腿时结算为 [`TypedTrade`]）。
struct LedgerOpen {
    entry_bar: usize,
    entry_px: f64,
    entry_z: super::mu_estimator::MuClass,
    /// ★A6（prereg-rev2-20260704）：入场结构止损距离 d=|entry_px−stop|（美元，ex-ante）。
    /// `Some(d)` = 决策 bar 因果 BspPoint + structural_stop 可算；`None` = structural_stop 返 None
    /// （非该方向交易点）/ BspPoint 缺失 ⟹ 下游 μ_R 剔除（诚实缺口，231号）。
    entry_stop_dist: Option<f64>,
    /// ★族A 修复：入场冻结的结构止损值（[`super::super::types::Tick`]，开仓 bar 由
    /// [`entry_structural_stop`] 一次性算）。逐 bar 风控门 [`k_theta_risk_gate`] 读此冻结值判
    /// `stop_hit`——消除旧路径用 drifted `leg.source_index`（carrier 走势 ρ）回查 `classification`
    /// 的覆盖度缺陷（exitfix-research §族A）。对齐 nautilus `record_held_voice`（入场一次性写
    /// HeldVoice.stop）。`None` = 该方向无结构止损（非交易点，与 `entry_stop_dist` 同口径）。
    entry_stop: Option<super::super::types::Tick>,
    /// 入场角色垂直轴（腿声部身份入场时固定）——`reverse_exit_type`/silent drop 判据输入。
    entry_v: super::super::strategy::coverage::Vertical,
    /// ★A9（Task #166，级别容器.pdf p14/§13）：position instance 严格身份四元组
    /// `hash(carrier, entry_certificate, side, generation)`。入场时刻冻结（carrier=腿 id、
    /// 证书=开仓 Candidate 坐标、side=Candidate 方向、generation=同 carrier campaign 高水位）。
    /// codex a9-posnode 裁定 C：身份归**账本生命周期层**（本结构 + `TypedTrade`），不进 `ActiveLeg`
    /// 结构层——`ActiveLeg` 每 bar 从树重建拿不到 campaign 状态。
    position_node_id: super::super::strategy::interp::PositionNodeId,
    /// ★opsem-dump（基因 073a）：入场时刻操作语义快照——env-gated `OPSEM_DUMP_DIR` 启用时由
    /// [`OpsemDump::write_trade`] 消费；未启用路径 `Default::default()` 零字段零开销（bit-exact）。
    /// 不进生产语义/μ 桶键/J_Θ 排序——纯只读外化（同 `dump_deltafree_pertrade` 先例）。
    opsem: OpsemEntrySnapshot,
    /// ★B1（步骤4）：腿级 sizing 目标（透传 `SepLeg::q_units`，含 dir_weight）。开腿登记时从
    /// `step_trace.sep_legs` 冻结，关腿结算透传 `TypedTrade::units`。不进 μ estimand（696 域）。
    units: f64,
}

/// 入场时刻操作语义快照（仅 env-gated dump 消费，零生产影响）。
#[derive(Default, Clone)]
struct OpsemEntrySnapshot {
    /// 入场候选 level（Candidate.level，ℓ_g）。
    cand_level: u32,
    /// 入场候选 source_index（bsp 触发点 L0 K 序）。
    cand_source_index: usize,
    /// 入场候选 bsp bits（6 bit 非互斥）。
    cand_bits: u8,
    /// 入场候选方向 σ_g（VoiceSide 编码：Long/Short/Flat）。
    cand_dir: &'static str,
    /// 入场候选最小成立类号（1/2/3，u8::MAX=无）。
    cand_bsp_class: u8,
    /// 入场候选 N^δ 区间套确认（nest_confirmed）。
    cand_nest_confirmed: bool,
    /// ★R5-c：入场候选区间套深度 Ndepth（structural_nest_depth 读数，= 从执行级向上连续包含
    /// source_index 的塔层数，与生产门 build_nest_certificate rungs.len() 同口径）。dump 专用——
    /// **不进 entry_z/MuClass/μ 桶键**（R5-1 铁律），替代旧 `entry_z.nest_depth`（π 路径恒 None）。
    nest_depth: u8,
    /// 入场候选角色 R(g)=(H,V,δ) 的 18 类索引字符串。
    cand_role: &'static str,
    /// 入场候选 A 段 MACD 面积（ForceProxies.seg_a.macd_area；None=无力度源，非一类背驰候选）。
    seg_a_macd_area: Option<f64>,
    /// 入场候选 C 段 MACD 面积（ForceProxies.seg_c.macd_area；None=同上）。
    seg_c_macd_area: Option<f64>,
    /// 入场候选 A 段 DIF 峰绝对值。
    seg_a_dif_peak: Option<f64>,
    /// 入场候选 C 段 DIF 峰绝对值。
    seg_c_dif_peak: Option<f64>,
    /// 入场候选 β^div 力度支配态（Weak=Dominated=背驰；None=无力度源）。
    force_state: Option<&'static str>,
    /// 入场时刻解释器喂入候选集大小（χ 过滤后 step_gamma_trade.len()）。
    gamma_count: usize,
    /// 入场时刻活动腿数（prev_active.len()，含即将开仓腿的兄弟）。
    prev_active_count: usize,
    /// ★R5-a：入场步 LexArgmin 的 top-3 J_Θ 候选键（字典序升序，`(JThetaKey, control)`）。首名 = p_star
    /// 选址（被选），余两名 = 被拒的次优。来自 `StepTrace.lex_top3`（pi_theta_step_traced 正常路径
    /// 填充）。dump 专用——不进 p_star/J_Θ/χ（R5-1 铁律）。空 vec = 该步无开仓（不应进 opsem_snap，
    /// 因 opsem_snap 仅对 opened 构造）。
    lex_top3: Vec<(strategy::intent::JThetaKey, f64)>,
    /// 入场腿父容器 ElementId（σ_p 来源；None=真边界胚元 ∂，σ_p=0=Ambient）。
    parent_id: Option<(u32, u64)>,
    /// 入场时刻 TW 阶段（tw.stage，入场决策点相位）。
    t_stage: &'static str,
    /// 入场时刻 η_t bucket（tw_policy.eta_bucket(&tw)）。
    eta_bucket: &'static str,
    /// 入场时刻 risk_mode（margin 五态）。
    risk_mode: &'static str,
}

// ─────────────────────────────────────────────────────────────────────────────
//  ★opsem-dump（基因 073a/274号 谱系）：只读语义快照 dump——env-gated，零生产语义改动。
//
//  触发：env `OPSEM_DUMP_DIR=<dir>`（如 /tmp/opsem）。**未设 ⟹ 全部方法 no-op**，
//  生产路径与既有测试逐字节不变（bit-exact，同 `dump_deltafree_pertrade`/`kappa_policy_resolved`
//  先例）。dump 不进 μ 桶键/J_Θ 排序/χ 门控——纯只读外化（no-patch-mentality：诊断切片
//  不冒充裁决层，铁律 exit-μ-BUCKETING-FROZEN #180 同款约束）。
//
//  产物（落 `<dir>/trades.jsonl` + `<dir>/tower_events.jsonl`）：
//  - trades.jsonl：每笔 TypedTrade 一行 JSON（entry/exit 字段 + 触发证书 + 解释器状态 +
//    声部树快照 + 背驰判定输入 + TW 阶段）。缺席字段标 null（不许编造）。
//  - tower_events.jsonl：bar 级塔事件（中枢新建/延伸/升级/破坏 + 级别 + bar 号），仅交易
//    活跃区间（首入场 bar .. 末离场 bar）。
//
//  认识论等级（formalization-validity-domain 231号）：L1（纯只读外化，零信息增量）。
//  字段缺口标注原则（no-patch-mentality + result-package 六要素）——R5（2026-07-05）后状态：
//  - 「LexArgmin 选中的与被拒的前 3 名 J_Θ 排序键」（R5-a 已实装）：`intent::lex_argmin_top_k`
//    在 `coverage::feasible_lex_candidates`（与 `pi_theta_position` 同源候选集）上稳定排序取前 3，
//    经 `StepTrace.lex_top3` → `OpsemEntrySnapshot.lex_top3` 透传至 `lex_argmin_top3` 字段
//    （`[{control,key:{5维}},...]`，首名 = p_star 选址）。不进 p_star/J_Θ/χ（R5-1 铁律）。
//  - 「区间套深度 nest_depth」（R5-c 已实装）：`econ_positive::structural_nest_depth`（与生产门
//    `build_nest_certificate` rungs 构造同款 partition_point，不依赖 hist）读 rungs.len()，
//    写入 `OpsemEntrySnapshot.nest_depth`（**不进 entry_z/MuClass/μ 桶键**——R5-1 铁律，避免
//    MuClass derive Hash 的 nest_depth 字段破坏 μ 分桶 bit-exact）。
//  - 「中枢破坏事件」：前缀因果塔单调增长（prefix classification 不删结构），**无破坏概念**
//    ⟹ tower_events.jsonl 不输出 destroy 事件（诚实缺席，不伪造）。
// ─────────────────────────────────────────────────────────────────────────────
struct OpsemDump {
    trades_buf: std::io::BufWriter<std::fs::File>,
    tower_buf: std::io::BufWriter<std::fs::File>,
    trade_id_counter: u64,
    /// 交易活跃区间（首入场 bar .. 末离场 bar）；None=尚未见入场。
    active_start: Option<usize>,
    active_end: Option<usize>,
    /// 上一 bar 的塔（仅交易活跃区间内 diff，O(n) per bar）。
    prev_tower: Option<Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>>,
}

#[cfg(test)]
thread_local! {
    /// 测试注入点：Some(dir) ⟹ **本线程**的 fill loop 启用 opsem dump。进程级 env 会被并行
    /// 测试的 [`OpsemDump::from_env`] 同时读到并 truncate 同一 trades.jsonl（2026-07-13 全量
    /// 回归竞态实录：0 笔交易的合成测试把 R5 测试的 dump 清空）；线程局部对并行测试不可见。
    static OPSEM_DUMP_DIR_OVERRIDE: std::cell::RefCell<Option<std::path::PathBuf>> =
        std::cell::RefCell::new(None);
}

impl OpsemDump {
    /// 从 env `OPSEM_DUMP_DIR` 构造；未设 ⟹ None（零开销）。测试经线程局部
    /// [`OPSEM_DUMP_DIR_OVERRIDE`] 注入（优先于 env；生产构建不含该分支）。
    fn from_env() -> Option<Self> {
        #[cfg(test)]
        {
            if let Some(dir) = OPSEM_DUMP_DIR_OVERRIDE.with(|c| c.borrow().clone()) {
                return Self::at_dir(&dir);
            }
        }
        let dir = std::env::var("OPSEM_DUMP_DIR").ok().filter(|s| !s.is_empty())?;
        Self::at_dir(std::path::Path::new(&dir))
    }

    /// 在指定目录开两个 JSONL 写入器。
    /// ponytail: 截断打开（每次回测重写；同 dump_deltafree_pertrade 落盘语义）。path 局部化——
    /// struct 只持 BufWriter（path 仅 create 时用，后续不读，YAGNI 不存字段）。
    fn at_dir(dir_path: &std::path::Path) -> Option<Self> {
        std::fs::create_dir_all(dir_path).ok()?;
        let trades_file = std::fs::File::create(dir_path.join("trades.jsonl")).ok()?;
        let tower_file = std::fs::File::create(dir_path.join("tower_events.jsonl")).ok()?;
        Some(Self {
            trades_buf: std::io::BufWriter::new(trades_file),
            tower_buf: std::io::BufWriter::new(tower_file),
            trade_id_counter: 0,
            active_start: None,
            active_end: None,
            prev_tower: None,
        })
    }

    /// 在入场 bar 标记交易活跃区间起点。
    fn mark_entry(&mut self, bar: usize) {
        if self.active_start.is_none() {
            self.active_start = Some(bar);
        }
        self.active_end = Some(bar);
    }

    /// 在离场 bar 更新活跃区间末点。
    fn mark_exit(&mut self, bar: usize) {
        self.active_end = Some(bar);
    }

    /// 写一笔 trade JSONL 行。`t` 是 TypedTrade，`open.opsem` 是入场快照，`pnl` 是费前方向盈亏
    /// （(exit_px-entry_px)×delta，未扣费——生产 fee 已在 fill loop 扣，TypedTrade 不携 fee）。
    fn write_trade(
        &mut self,
        t: &TypedTrade,
        open: &LedgerOpen,
        trigger_bsp_class: Option<u8>,
    ) -> std::io::Result<()> {
        use std::io::Write;
        self.trade_id_counter += 1;
        // 费前方向盈亏（delta=+1 Long / -1 Short）。
        let delta_sign: f64 = t.entry_z.delta as f64;
        let pnl_raw = delta_sign * (t.exit_px - t.entry_px);
        let o = &open.opsem;
        // ponytail: 显式 push_str 拼装 JSON——避免 format! 的 `{{`/`}}` 转义混乱（曾出引号 bug）。
        // 缺席字段写 null（JSON 标准缺席标注，不编造）。
        let mut s = String::with_capacity(1024);
        s.push('{');
        // 基本字段。
        s.push_str(&format!(
            "\"trade_id\":{},\"voice_id\":{{\"level\":{},\"ordinal\":{}}},",
            self.trade_id_counter, t.voice_id.level, t.voice_id.ordinal,
        ));
        s.push_str(&format!(
            "\"position_node_id\":{},\"entry_bar\":{},\"exit_bar\":{},\"entry_px\":{},\"exit_px\":{},\"pnl_raw_unlevered\":{},\"exit_type\":\"{}\",\"via_structural_prune\":{},\"units\":{},",
            t.position_node_id.hash64(), t.entry_bar, t.exit_bar,
            t.entry_px, t.exit_px, pnl_raw,
            exit_type_str(t.exit_type), t.via_structural_prune, t.units,
        ));
        // 触发证书。
        s.push_str("\"certificate\":{");
        s.push_str(&format!(
            "\"bsp_bits_class_index\":{},\"bsp_class_min\":{},\"level\":{},\"source_index\":{},\"dir\":\"{}\",\"delta\":{},\"nest_confirmed\":{},\"nest_depth\":{},\"parent_dir_sigma_p\":{},\"role\":\"{}\"}},",
            o.cand_bits,
            if o.cand_bsp_class == u8::MAX { -1 } else { o.cand_bsp_class as i64 },
            o.cand_level, o.cand_source_index, o.cand_dir, t.entry_z.delta,
            // R5-c：nest_depth 改读 opsem_snap（structural_nest_depth 纯结构读数），不读 entry_z
            // （π 路径 entry_z.nest_depth 恒 None——MuClass 进 μ 桶键，填 Some 破坏 bit-exact）。
            o.cand_nest_confirmed, o.nest_depth,
            t.entry_z.parent_dir, o.cand_role,
        ));
        // 入场时刻解释器状态。
        s.push_str("\"interpreter_at_entry\":{");
        s.push_str(&format!(
            "\"gamma_count_chi_filtered\":{},\"prev_active_count\":{},\"lex_argmin_top3\":{}",
            o.gamma_count, o.prev_active_count, lex_top3_json(&o.lex_top3),
        ));
        s.push_str("},");
        // 声部树快照。
        s.push_str("\"voice_tree_at_entry\":{");
        s.push_str(&format!(
            "\"parent_id\":{},\"is_boundary_root_absent\":{},\"active_count_inclusive\":{}}},",
            opt_pair_str(o.parent_id), o.parent_id.is_none(),
            o.prev_active_count.saturating_add(1),
        ));
        // 背驰判定输入。
        s.push_str("\"divergence_input\":{");
        s.push_str(&format!(
            "\"seg_a_macd_area\":{},\"seg_c_macd_area\":{},\"seg_a_dif_peak\":{},\"seg_c_dif_peak\":{},\"force_state_weak_judgment\":{}}},",
            opt_f64_str(o.seg_a_macd_area), opt_f64_str(o.seg_c_macd_area),
            opt_f64_str(o.seg_a_dif_peak), opt_f64_str(o.seg_c_dif_peak),
            opt_str_quoted(o.force_state),
        ));
        // 入场时刻 TW 阶段。
        s.push_str("\"tw_at_entry\":{");
        s.push_str(&format!(
            "\"stage\":\"{}\",\"eta_bucket\":\"{}\",\"risk_mode\":\"{}\"}},",
            o.t_stage, o.eta_bucket, o.risk_mode,
        ));
        // 出场侧 + 收尾。
        s.push_str(&format!(
            "\"entry_stop_dist\":{},\"trigger_bsp_class_at_exit\":{},\"exit_z_t_stage\":{}}}\n",
            opt_f64_str(t.entry_stop_dist),
            opt_u8_str(trigger_bsp_class),
            opt_str_quoted(t.exit_z.t_stage.map(t_stage_str)),
        ));
        self.trades_buf.write_all(s.as_bytes())?;
        Ok(())
    }

    /// 写一个塔事件 JSONL 行（仅交易活跃区间）。
    fn write_tower_event(
        &mut self,
        bar: usize,
        level: u32,
        kind: &str,
        detail: &str,
    ) -> std::io::Result<()> {
        use std::io::Write;
        let active = match (self.active_start, self.active_end) {
            (Some(s), _) if bar < s => false,
            (_, Some(_e)) => true,
            _ => false,
        };
        if !active {
            return Ok(());
        }
        let json = format!(
            "{{\"bar\":{bar},\"level\":{lvl},\"kind\":\"{kind}\",\"detail\":\"{detail}\"}}\n",
            bar = bar,
            lvl = level,
            kind = kind,
            detail = detail.replace('\\', "\\\\").replace('"', "\\\""),
        );
        self.tower_buf.write_all(json.as_bytes())?;
        Ok(())
    }

    /// 在每 bar 调用：diff tower_i vs self.prev_tower，输出新建/延伸/升级事件（仅活跃区间）。
    /// `destroy` 事件缺席——前缀因果塔单调增长（prefix classification 不删结构）。
    fn diff_tower(
        &mut self,
        bar: usize,
        tower_i: &[std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>],
    ) {
        use classifier::descend::RMove;
        use classifier::recursive_tower::LeveledMove;
        // 仅在交易活跃区间内 diff（避免 O(n) per-bar 全窗扫描）。
        let in_active = match (self.active_start, self.active_end) {
            (Some(s), _) if bar >= s => true,
            _ => false,
        };
        if !in_active {
            self.prev_tower = Some(tower_i.to_vec());
            return;
        }
        let prev = self.prev_tower.take();
        match prev {
            None => {
                // 首个活跃 bar：所有 Compose 都作 "new_center"。
                for (lvl, moves) in tower_i.iter().enumerate() {
                    for m in moves.iter() {
                        if let RMove::Compose { centers, .. } = &m.rmove {
                            if let Some(c) = centers.first() {
                                let _ = self.write_tower_event(
                                    bar,
                                    lvl as u32,
                                    "new_center",
                                    &format!(
                                        "L{} #{} zd={} zg={} si={} ei={}",
                                        lvl, m.id.ordinal, c.zd, c.zg, m.start_index, m.end_index
                                    ),
                                );
                            }
                        }
                    }
                }
            }
            Some(prev_vec) => {
                for (lvl, moves) in tower_i.iter().enumerate() {
                    let prev_moves: &[LeveledMove] = prev_vec
                        .get(lvl)
                        .map(|rc| rc.as_slice())
                        .unwrap_or(&[]);
                    // 升级：该级别在 prev 不存在（或为空）且现非空 ⟹ 新级别涌现。
                    if prev_moves.is_empty() && !moves.is_empty() {
                        let _ = self.write_tower_event(
                            bar,
                            lvl as u32,
                            "level_upgrade",
                            &format!("L{lvl} first compose count={}", moves.len()),
                        );
                    }
                    // 新建 Compose / 延伸末段 end_index。
                    let prev_len = prev_moves.len();
                    for (i, m) in moves.iter().enumerate() {
                        if let RMove::Compose { centers, .. } = &m.rmove {
                            if i >= prev_len {
                                // 新 Compose 涌现。
                                if let Some(c) = centers.first() {
                                    let _ = self.write_tower_event(
                                        bar,
                                        lvl as u32,
                                        "new_center",
                                        &format!(
                                            "L{} #{} zd={} zg={} si={} ei={}",
                                            lvl, m.id.ordinal, c.zd, c.zg, m.start_index, m.end_index
                                        ),
                                    );
                                }
                            } else if let Some(pm) = prev_moves.get(i) {
                                // 已存在 Compose，比较 end_index —— 延伸事件。
                                if m.end_index != pm.end_index {
                                    if let Some(c) = centers.first() {
                                        let _ = self.write_tower_event(
                                            bar,
                                            lvl as u32,
                                            "extend",
                                            &format!(
                                                "L{} #{} zd={} zg={} ei {}->{}",
                                                lvl, m.id.ordinal, c.zd, c.zg, pm.end_index, m.end_index
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        self.prev_tower = Some(tower_i.to_vec());
    }

    /// flush 缓冲（drop 前）。
    fn flush(&mut self) {
        use std::io::Write;
        let _ = self.trades_buf.flush();
        let _ = self.tower_buf.flush();
    }
}

impl Drop for OpsemDump {
    fn drop(&mut self) {
        self.flush();
    }
}

/// 辅助：Optional<u8> → JSON 字符串（number 或 null）。
fn opt_u8_str(o: Option<u8>) -> String {
    match o {
        Some(v) => v.to_string(),
        None => "null".into(),
    }
}

/// ★R5-a 辅助：LexArgmin top-3 候选 → JSON 数组字符串。每项 `{control, key:{5 维}}`，字典序升序
/// （首名 = p_star 选址/被选，余 = 被拒次优）。空 ⟹ `[]`（无开仓步，不应进 opsem_snap）。
/// `control` 是净持仓格点（f64），`key` 是 [`JThetaKey`] 5 维 i64 字典序键。
fn lex_top3_json(items: &[(strategy::intent::JThetaKey, f64)]) -> String {
    if items.is_empty() {
        return "[]".into();
    }
    let parts: Vec<String> = items
        .iter()
        .map(|(key, control)| {
            format!(
                "{{\"control\":{},\"key\":{{\"tracking_err\":{},\"trade_cost\":{},\"risk_penalty\":{},\"turnover\":{},\"grid_index\":{}}}}}",
                control, key.tracking_err, key.trade_cost, key.risk_penalty, key.turnover, key.grid_index,
            )
        })
        .collect();
    format!("[{}]", parts.join(","))
}

/// 辅助：Optional<f64> → JSON 字符串（number 或 null）。NaN/Inf 一律 null（JSON 无 NaN）。
fn opt_f64_str(o: Option<f64>) -> String {
    match o {
        Some(v) if v.is_finite() => format!("{v}"),
        _ => "null".into(),
    }
}

/// 辅助：Optional<&str> → JSON 字符串（带引号 或 null）。
fn opt_str_quoted(o: Option<&str>) -> String {
    match o {
        Some(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        None => "null".into(),
    }
}

/// 辅助：Optional<(u32,u64)> → JSON 对象 或 null（parent_id 序列化）。
fn opt_pair_str(o: Option<(u32, u64)>) -> String {
    match o {
        Some((lvl, ord)) => format!("{{\"level\":{lvl},\"ordinal\":{ord}}}"),
        None => "null".into(),
    }
}

/// 辅助：ExitType → 字符串。
fn exit_type_str(e: super::super::strategy::interp::ExitType) -> &'static str {
    use super::super::strategy::interp::ExitType;
    match e {
        ExitType::CloseRoot => "CloseRoot",
        ExitType::ReduceCore => "ReduceCore",
        ExitType::CloseShortDiff => "CloseShortDiff",
        ExitType::RiskExit => "RiskExit",
        ExitType::Hold => "Hold",
    }
}

/// 辅助：TStage → 字符串。
fn t_stage_str(s: super::super::strategy::ledger::TStage) -> &'static str {
    use super::super::strategy::ledger::TStage;
    match s {
        TStage::CostReduction => "CostReduction",
        TStage::CapitalRecovered => "CapitalRecovered",
        TStage::EarningShares => "EarningShares",
    }
}

/// 辅助：VoiceSide → 字符串。
fn voice_side_str(v: super::super::strategy::voice::VoiceSide) -> &'static str {
    use super::super::strategy::voice::VoiceSide;
    match v {
        VoiceSide::Long => "Long",
        VoiceSide::Short => "Short",
        VoiceSide::Flat => "Flat",
    }
}

/// 辅助：Vertical → 字符串（声部角色垂直轴）。
fn vertical_str(v: super::super::strategy::coverage::Vertical) -> &'static str {
    use super::super::strategy::coverage::Vertical;
    match v {
        Vertical::Ambient => "Ambient",
        Vertical::FollowParent => "FollowParent",
        Vertical::ShortDiff => "ShortDiff",
    }
}

/// 辅助：OperationRole → 字符串（H,V,δ 三分量）。
fn operation_role_str(r: super::super::strategy::coverage::OperationRole) -> String {
    use super::super::strategy::coverage::{Dir, Horizontal};
    let h = match r.h {
        Horizontal::First => "First",
        Horizontal::SameFollow => "SameFollow",
        Horizontal::SameReverse => "SameReverse",
    };
    let d = match r.delta {
        Dir::Plus => "Plus",
        Dir::Minus => "Minus",
    };
    format!("{h}|{}|{d}", vertical_str(r.v))
}

/// 辅助：RiskMode → 字符串。
fn risk_mode_str(m: super::super::strategy::risk::RiskMode) -> &'static str {
    use super::super::strategy::risk::RiskMode;
    match m {
        RiskMode::Normal => "Normal",
        RiskMode::Deleverage => "Deleverage",
        RiskMode::CloseOnly => "CloseOnly",
        RiskMode::Insolvent => "Insolvent",
        RiskMode::Liquidation => "Liquidation",
    }
}

/// 辅助：EtaBucket → 字符串。
fn eta_bucket_str(e: super::super::strategy::ledger::EtaBucket) -> &'static str {
    use super::super::strategy::ledger::EtaBucket;
    match e {
        EtaBucket::Deficit => "Deficit",
        EtaBucket::Zero => "Zero",
        EtaBucket::PositiveUnsafe => "PositiveUnsafe",
        EtaBucket::PositiveSafe => "PositiveSafe",
    }
}

/// 辅助：ForceStateA5 → 字符串。
fn force_state_str(s: super::super::classifier::divergence::ForceStateA5) -> &'static str {
    use super::super::classifier::divergence::ForceStateA5;
    match s {
        ForceStateA5::Dominated => "Dominated(Weak=背驰)",
        ForceStateA5::Dominates => "Dominates(力度延续)",
        ForceStateA5::Tie => "Tie",
        ForceStateA5::Incomparable => "Incomparable(口径冲突)",
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
    /// 腿级 typed 交易 ledger（G4；π 路径 [`pi_theta_fill_loop`] 产出，v1 路径
    /// [`plan_and_fill_mtm`] 无腿级台账 ⟹ 恒空，诚实不伪造）。
    typed_ledger: Vec<TypedTrade>,
    /// TW 账本终态（#124 裁定4：TwState 生产真值源在 π fill loop；TStage/ηBucket「生产者已
    /// 就位」的可观测物证——G3 ZExt 桥/诊断消费）。v1 路径无 TW 接线 ⟹ `None`（诚实不伪造）。
    tw_final: Option<TwState>,
    /// M6 R 分解表（路线.pdf p16 第十一关：R=ΣN_tΔP_t−Commission−Slippage−Funding−Borrow−
    /// LiquidationLoss + 账目守恒残差）。π 路径 [`pi_theta_fill_loop`] 产出；v1 路径
    /// [`plan_and_fill_mtm`] 未接 R 分解 ⟹ `None`（诚实不伪造，v1 是 recognize 旧路径非生产 π）。
    r_decomp: Option<super::super::strategy::risk::RDecomposition>,
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
/// **★交易轨迹**：平仓事件逐 fill 记 [`metrics::TradeRecord`]（entry_bar/exit_bar/hold_bars/qty）：
/// 符号归零/翻转配对整段，**部分减仓也逐 fill 产记录**（qty=减掉手数，bughunt F-06——与
/// `trade_pnls` 逐 fill 已实现口径对齐），作操作语义随机入场对照（§4 重写）的输入。
/// v0 单声部多头全开全平时退化为开-平配对唯一。
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
                    let fill = apply_order(o, px, fee_rate, &mut cash, &mut units, &mut entry_cost, &mut trade_pnls);
                    if fill.executed_qty <= 0.0 {
                        continue;
                    }
                    // ★轨迹配对（v1 方向中性）：平仓到 units=0 / 翻转 ⟹ 完整交易闭合（非强平，正常退出）。
                    track_position_transition(
                        &mut trades, &mut pos_entry_bar, units_before, units, i, false,
                    );
                    apply_voice_fill(&mut voice_qty, depth, fill);
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
                    let fill = apply_order(o, px, fee_rate, &mut cash, &mut units, &mut entry_cost, &mut trade_pnls);
                    if fill.executed_qty <= 0.0 {
                        continue;
                    }
                    // ★轨迹追踪（v1 方向中性）：units 有符号（正=多/负=空）。任一订单经
                    // apply_fill「先平后开」可能同时闭合反向旧仓 + 开新仓（翻转）⟹ 用 units 跨 0
                    // 行为统一追踪：(a) |units| 由非零回 0 / 跨 0 翻转 ⟹ 闭合旧方向 trade（配对
                    // pos_entry_bar，方向 = units_before.signum()）；(b) units 由 0 进入非零 /
                    // 翻转后新方向 ⟹ 记新 entry_bar。
                    track_position_transition(
                        &mut trades, &mut pos_entry_bar, units_before, units, i, false,
                    );
                    apply_voice_fill(&mut voice_qty, depth, fill);
                    // ★开仓订单 ⟹ 记入持仓台账（退出生成器读它）。止损价由入场决策的
                    // stop_in + 方向算出（与 build_open_order 内 structural_stop 同一函数）。
                    if fill.opened_qty > 0.0
                        && matches!(o.action, StrictAction::Buy | StrictAction::Sell | StrictAction::Add)
                    {
                        if let Some(d) = matched {
                            record_held_voice(&mut held, d);
                        }
                    } else if voice_qty.get(depth).copied().unwrap_or(0) == 0 {
                        if let Some(slot) = held.get_mut(depth) {
                            *slot = None;
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
        typed_ledger: Vec::new(), // v1 无腿级台账（诚实空，非 π 路径）
        tw_final: None,           // v1 无 TW 接线（#124 只接 π 路径，诚实 None）
        r_decomp: None,           // v1 无 R 分解（M6 只接生产 π 路径，诚实 None）
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  关⑤：双账本嵌套并列路径（纯新增，旧路径零改）
//  施工图：chanlun/review-results/p120-nested-voice-dual-ledger-design-20260718.md §4.6
// ──────────────────────────────────────────────────────────────────────────

/// [`plan_and_fill_mtm_dual`] 的完整产出（[`FillOutput`] + 双账本终态 + 逐腿成交日志）。
///
/// `leg_log` 是嵌套/双账语义的**可观测物证**（090 纸面定义=运行时产出）：E 组测试据此断言
/// 「开空父腿不动（M13）」「最深优先 cascade」「终点零残留」——不进 `FillOutput`（其字段
/// 口径与净额路径逐字节对拍用，E4）。
struct FillOutputDual {
    /// 与 [`plan_and_fill_mtm`] 同构的产出（equity/pnls/trades/n_orders——E4 对拍对象）。
    fill: FillOutput,
    /// 双账本终态（窗口终点双腿强平**已应用**于账本 ⟹ 终态零持仓；强平 PnL 只入
    /// `trade_pnls_with_forced`，与净额路径 :2883-2894 同口径）。
    ledger: dual_ledger::DualLedger,
    /// 逐腿成交日志（含强平笔）：bar/depth/leg/close/成交手数/realized/双腿后态。
    leg_log: Vec<LegFillRec>,
}

/// 逐腿成交记录（[`plan_and_fill_mtm_dual`] 的可观测轨迹）。
#[derive(Debug, Clone, Copy, PartialEq)]
struct LegFillRec {
    bar: usize,
    depth: u32,
    leg: super::super::strategy::voice::VoiceSide,
    close: bool,
    qty: f64,
    realized: f64,
    q_long_after: f64,
    q_short_after: f64,
}

/// voice_qty 同步（双账版，[`apply_voice_fill`] 同语义：两段真实成交分别扣/加，
/// 绝不按原始请求量或 action 推断）。`FillOutcomeDual` 单腿单向 ⟹ closed/opened 至多一侧非零。
fn apply_voice_fill_dual(
    voice_qty: &mut [u32],
    depth: usize,
    fill: &dual_ledger::FillOutcomeDual,
) {
    if let Some(slot) = voice_qty.get_mut(depth) {
        let closed = fill.closed_qty().round().clamp(0.0, u32::MAX as f64) as u32;
        let opened = fill.opened_qty().round().clamp(0.0, u32::MAX as f64) as u32;
        debug_assert!((fill.closed_qty() - closed as f64).abs() < 1e-9);
        debug_assert!((fill.opened_qty() - opened as f64).abs() < 1e-9);
        *slot = slot.saturating_sub(closed).saturating_add(opened);
    }
}

/// ★关⑤：[`plan_and_fill_mtm`] 的**双账本嵌套并列版**（纯新增，旧路径零改）。
///
/// 与净额路径的差异（施工图 §4.6）：
/// - **账本态**：`DualLedger{cash,q⁺,q⁻,cost⁺,cost⁻}`（M14 `P^sep` 账户层兑现）——分腿
///   不先净额（M13 父仓保持：持多腿时开空腿，多腿不动）；realized 只出平仓腿（A' 保留）。
/// - **识别**：开仓循环 per-bar 喂 [`strategy::recognize_nested`]（§3.2）——held 台账投影
///   活动集 + 真 ShortDiff 角色门 ⟹ depth>0 子声部**运行时产出**（非纸面声部）。
///   每 bar 以**当 bar 活动集**重识别全窗候选、只执行 `exec_index==i` 的决策（候选的
///   根/子归属由其 exec bar 的活动集定——与净额路径的静态预分组语义差异如实声明；
///   v1 全窗口径的前视有效域与 `run_theta_v0` 同源标注）。
/// - **成交**：`apply_order` 调用点换 [`dual_ledger::apply_fill_dual`]（订单经
///   [`strategy::plan_orders_dual_traced`] 携腿标记 + decision 精确配对，不用方向近似匹配）。
/// - **退出循环**：`exit_decision_for_nested`（parent_invalid 实义化）+ cascade 发射
///   （父退出 ⟹ 全部更深 held 强制退出）；批内执行**最深优先**（§3.5/§20：子腿现金先到位）。
/// - **毛闸门**（§3.6，[设计选择,Θ_risk]）：depth>0 边际子开仓校验 `Σq_v ≤ γ̄·base_units`
///   （`risk::gross_units_ok` 单源，coverage.rs:1687 同一 predicate）；超限 ⟹ 拒当步边际
///   子开仓（fail-closed；不缩放既有腿——缩放规则属另一裁定）。
/// - **§9 反向项信号池**：`groups[i]` 只收**根域开仓决策**（exit=false ∧ depth==0）——
///   ShortDiff 子决策的反父 bits 不喂反向项（M13：父仓穿越次级反向信号持有，短差由子腿
///   承担，非父平仓触发）；同级别反向平仓由 interpret 规则2 承载（close 决策携入场快照
///   bsp，非当 bar 信号，不入池）。exit_decision_for_nested 的 stop/risk/parent_invalid
///   与级联覆盖其余关闭通道。
/// - **交易轨迹**：per-leg `track_position_transition`（多腿 +q_long / 空腿 −q_short 有符号
///   喂入）——兼容腿标记（无真双开）下与净额 `units` 轨迹**同一交易列表**（E4 对拍锁）。
///
/// `apply_voice_fill` 语义不变（voice_qty[depth]=手数——单脊柱下 depth↔腿 1:1，
/// side 由 held 台账定）。TW/R 分解/typed_ledger 无接线（诚实 None/空，同 v1 净额路径）。
fn plan_and_fill_mtm_dual(
    classification: &super::super::classifier::Classification,
    tower: &[Rc<Vec<super::super::classifier::recursive_tower::LeveledMove>>],
    bars: &[Bar],
    initial_nav: f64,
    config: &ThetaConfig,
) -> FillOutputDual {
    use super::super::strategy::exec::fill_bar_index;
    use super::super::strategy::exit::{
        cascade_exit_decisions, exit_decision_for_nested, parent_invalid_at,
    };
    use super::super::strategy::risk;
    use super::super::strategy::voice::VoiceSide;
    use super::super::strategy::LegOrder;

    let n = bars.len();
    let nav0 = if initial_nav > 0.0 { initial_nav } else { 1.0 };
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;

    // 账本态（M14 P^sep）：DualLedger 分腿；voice_qty/held 与净额路径同构（depth 索引单槽）。
    let mut ledger = dual_ledger::DualLedger::new(nav0);
    let mut voice_qty: Vec<u32> = vec![0u32; config.voice.max_depth as usize];
    let mut held: Vec<Option<HeldVoice>> = vec![None; config.voice.max_depth as usize];
    let mut exit_orders_at: Vec<Vec<VoiceDecision>> = vec![Vec::new(); n];
    // groups[i] = 根域开仓决策（§9 反向项信号池，见函数头注释）。
    let mut groups: Vec<Vec<VoiceDecision>> = vec![Vec::new(); n];

    let mut equity_curve = Vec::with_capacity(n);
    let mut trade_pnls: Vec<f64> = Vec::new();
    let mut n_orders_executed: usize = 0;
    let mut trades: Vec<metrics::TradeRecord> = Vec::new();
    // per-leg 入场 bar（交易轨迹配对；多腿 +q / 空腿 −q 有符号喂 track_position_transition）。
    let mut entry_bar_long: Option<usize> = None;
    let mut entry_bar_short: Option<usize> = None;
    let mut leg_log: Vec<LegFillRec> = Vec::new();

    for i in 0..n {
        let bar = &bars[i];
        let px = bar.close as f64 * config.tick.tick_size;

        // ── 1. 退出 Close 延迟队列成交（spec:54 退出先于开仓；cascade 批内最深优先执行）。 ──
        if !exit_orders_at[i].is_empty() && !bar.untradable && px > 0.0 {
            let exit_decisions = std::mem::take(&mut exit_orders_at[i]);
            let current_nav = ledger.equity(px);
            let account = AccountState {
                nav: if current_nav > 0.0 { current_nav } else { nav0 },
                voice_qty: voice_qty.clone(),
            };
            let mut traced =
                strategy::plan_orders_dual_traced(&exit_decisions, bars, &account, config);
            // cascade 执行序：最深优先（§3.5；单脊柱 depth 唯一 ⟹ depth 降序即全序，
            // stable sort 保持同 depth 内 ConflictKey 序——单脊柱下无同 depth 两决策）。
            traced.sort_by(|a, b| b.0.depth.cmp(&a.0.depth));
            for (d, lo) in traced.iter() {
                if lo.order.qty > 0 && matches!(lo.order.action, StrictAction::Close) {
                    let depth = d.depth as usize;
                    let (ql_b, qs_b) = (ledger.q_long, ledger.q_short);
                    let fill =
                        dual_ledger::apply_fill_dual(lo, px, fee_rate, &mut ledger, &mut trade_pnls);
                    if fill.executed_qty <= 0.0 {
                        continue;
                    }
                    track_position_transition(
                        &mut trades, &mut entry_bar_long, ql_b, ledger.q_long, i, false,
                    );
                    track_position_transition(
                        &mut trades, &mut entry_bar_short, -qs_b, -ledger.q_short, i, false,
                    );
                    apply_voice_fill_dual(&mut voice_qty, depth, &fill);
                    leg_log.push(LegFillRec {
                        bar: i,
                        depth: d.depth,
                        leg: lo.leg,
                        close: true,
                        qty: fill.executed_qty,
                        realized: fill.realized,
                        q_long_after: ledger.q_long,
                        q_short_after: ledger.q_short,
                    });
                    // 平仓后台账状态机：全平 ⟹ 清台账；未全平 ⟹ 重置 exit_pending（同净额路径）。
                    if voice_qty.get(depth).copied().unwrap_or(0) == 0 {
                        if let Some(slot) = held.get_mut(depth) {
                            *slot = None;
                        }
                    } else if let Some(Some(h)) = held.get_mut(depth) {
                        h.exit_pending = false;
                    }
                    n_orders_executed += 1;
                }
            }
        }

        // ── 2. 开仓循环：per-bar recognize_nested（held 活动投影 + 真 ShortDiff 角色门）。 ──
        if !bar.untradable && px > 0.0 {
            let active = strategy::held_voice_projection(&held);
            let recog = strategy::recognize_nested(classification, tower, &active, bars, config);
            let bar_decisions: Vec<VoiceDecision> = recog
                .into_iter()
                .filter(|d| fill_bar_index(d.signal_index, bars, &config.exec) == Some(i))
                .collect();
            // §9 反向项信号池：只收根域开仓决策（M13：子决策的反父 bits 不触发父平仓）。
            groups[i] = bar_decisions
                .iter()
                .filter(|d| !d.exit && d.depth == 0)
                .copied()
                .collect();

            if !bar_decisions.is_empty() {
                let current_nav = ledger.equity(px);
                let account = AccountState {
                    nav: if current_nav > 0.0 { current_nav } else { nav0 },
                    voice_qty: voice_qty.clone(),
                };
                let traced =
                    strategy::plan_orders_dual_traced(&bar_decisions, bars, &account, config);
                // base_units（U_ℓ = NAV/px，runner [B] 同口径）——毛闸门分母。
                let base_units = account.nav / px;
                for (d, lo) in traced.iter() {
                    if lo.order.qty <= 0 {
                        continue;
                    }
                    // ★毛闸门（§3.6）：depth>0 边际子开仓 Σq_v ≤ γ̄·base_units（risk::gross_units_ok
                    // 单源判定）；超限 ⟹ 拒当步边际子开仓（fail-closed，不缩放既有腿）。
                    if d.depth > 0 && !lo.close {
                        let tentative = ledger.gross_units() + lo.order.qty as f64;
                        if !risk::gross_units_ok(tentative, base_units, config.risk.gamma) {
                            continue;
                        }
                    }
                    let depth = d.depth as usize;
                    let (ql_b, qs_b) = (ledger.q_long, ledger.q_short);
                    let fill =
                        dual_ledger::apply_fill_dual(lo, px, fee_rate, &mut ledger, &mut trade_pnls);
                    if fill.executed_qty <= 0.0 {
                        continue;
                    }
                    track_position_transition(
                        &mut trades, &mut entry_bar_long, ql_b, ledger.q_long, i, false,
                    );
                    track_position_transition(
                        &mut trades, &mut entry_bar_short, -qs_b, -ledger.q_short, i, false,
                    );
                    apply_voice_fill_dual(&mut voice_qty, depth, &fill);
                    leg_log.push(LegFillRec {
                        bar: i,
                        depth: d.depth,
                        leg: lo.leg,
                        close: lo.close,
                        qty: fill.executed_qty,
                        realized: fill.realized,
                        q_long_after: ledger.q_long,
                        q_short_after: ledger.q_short,
                    });
                    // 开仓成交 ⟹ 记入持仓台账（decision 精确配对，traced 单源）；
                    // 平仓成交到手数 0 ⟹ 清台账（recognize 产 close 决策的成交后处理）。
                    if fill.opened_qty() > 0.0 && !lo.close {
                        record_held_voice(&mut held, d);
                    } else if voice_qty.get(depth).copied().unwrap_or(0) == 0 {
                        if let Some(slot) = held.get_mut(depth) {
                            *slot = None;
                        }
                    }
                    n_orders_executed += 1;
                }
            }
        }

        // ── 3. 退出决策生成器（§9 closePred 四析取皆实：parent_invalid 实义化 + cascade）。 ──
        if !bar.untradable && px > 0.0 {
            let equity_now = ledger.equity(px);
            let groups_view: Vec<Vec<&VoiceDecision>> =
                groups.iter().map(|g| g.iter().collect()).collect();
            for depth in 0..held.len() {
                let hv = match held[depth] {
                    Some(hv) => hv,
                    None => continue,
                };
                if hv.exit_pending {
                    continue; // Close 在延迟队列待成交——不重复入队
                }
                if voice_qty.get(depth).copied().unwrap_or(0) == 0 {
                    continue;
                }
                let parent_invalid = parent_invalid_at(&held, depth);
                if let Some(exit_d) =
                    exit_decision_for_nested(&hv, depth, bar, i, &groups_view, equity_now, parent_invalid)
                {
                    if let Some(fi) = fill_bar_index(i, bars, &config.exec) {
                        if fi < n {
                            // cascade 发射（M16 AncOK 父关则子关，最深优先）+ pending 标记。
                            for d in cascade_exit_decisions(&held, depth, exit_d, i) {
                                if let Some(Some(h)) = held.get_mut(d.depth as usize) {
                                    h.exit_pending = true;
                                }
                                exit_orders_at[fi].push(d);
                            }
                        }
                    }
                }
            }
        }

        // 权益曲线（净投影估值，M30；归一化 ÷nav0）。
        let equity = ledger.equity(px) / nav0;
        equity_curve.push(equity);
    }

    // ── 窗口终点强制平仓（双账本：双腿各自强平，分腿口径与净额 :2884-2894 同形；
    //    强平**应用于账本** ⟹ 终态零持仓（E1 现金守恒/E3 零残留的断言对象）；
    //    PnL 只入 trade_pnls_with_forced（realized 口径不含强平，与净额路径一致））。 ──
    let mut trade_pnls_with_forced = trade_pnls.clone();
    if let Some(last_bar) = bars.last() {
        let last_px = last_bar.close as f64 * config.tick.tick_size;
        if last_px > 0.0 {
            let exit_bar = n.saturating_sub(1);
            if ledger.q_long > 0.0 {
                let lo = LegOrder {
                    order: Order {
                        action: StrictAction::Close,
                        qty: ledger.q_long as i64,
                        exec_index: exit_bar,
                    },
                    leg: VoiceSide::Long,
                    close: true,
                };
                let fill = dual_ledger::apply_fill_dual(
                    &lo, last_px, fee_rate, &mut ledger, &mut trade_pnls_with_forced,
                );
                if let Some(entry_bar) = entry_bar_long.take() {
                    trades.push(metrics::TradeRecord {
                        entry_bar,
                        exit_bar,
                        hold_bars: exit_bar.saturating_sub(entry_bar).max(1),
                        qty: fill.closed_long,
                        long: true,
                        forced_close: true,
                    });
                }
                leg_log.push(LegFillRec {
                    bar: exit_bar,
                    depth: 0, // 强平是账户层双腿清空，depth 栏无单槽语义（记 0 占位）
                    leg: VoiceSide::Long,
                    close: true,
                    qty: fill.closed_long,
                    realized: fill.realized,
                    q_long_after: ledger.q_long,
                    q_short_after: ledger.q_short,
                });
            }
            if ledger.q_short > 0.0 {
                let lo = LegOrder {
                    order: Order {
                        action: StrictAction::Close,
                        qty: ledger.q_short as i64,
                        exec_index: exit_bar,
                    },
                    leg: VoiceSide::Short,
                    close: true,
                };
                let fill = dual_ledger::apply_fill_dual(
                    &lo, last_px, fee_rate, &mut ledger, &mut trade_pnls_with_forced,
                );
                if let Some(entry_bar) = entry_bar_short.take() {
                    trades.push(metrics::TradeRecord {
                        entry_bar,
                        exit_bar,
                        hold_bars: exit_bar.saturating_sub(entry_bar).max(1),
                        qty: fill.closed_short,
                        long: false,
                        forced_close: true,
                    });
                }
                leg_log.push(LegFillRec {
                    bar: exit_bar,
                    depth: 0,
                    leg: VoiceSide::Short,
                    close: true,
                    qty: fill.closed_short,
                    realized: fill.realized,
                    q_long_after: ledger.q_long,
                    q_short_after: ledger.q_short,
                });
            }
        }
    }

    let daily_returns = bar_returns(&equity_curve);
    FillOutputDual {
        fill: FillOutput {
            equity_curve,
            daily_returns,
            trade_pnls_realized: trade_pnls,
            trade_pnls_with_forced,
            trades,
            n_orders: n_orders_executed,
            typed_ledger: Vec::new(), // 无腿级台账（诚实空，同 v1 净额路径）
            tw_final: None,           // 无 TW 接线（E2 G5 同数锁待接线后启用，诚实 None）
            r_decomp: None,
        },
        ledger,
        leg_log,
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
/// - **同向减仓未到 0**（before·after>0 且 |after|<|before|）：**逐 fill 产 TradeRecord**
///   （qty = 减掉的手数，entry_bar 延续首次入场，v0 单标量近似）——bughunt F-06：否则部分
///   减仓的已实现 PnL/敞口在 trades 轨迹无对应记录，§4 随机对照 same-caliber 重算与敞口
///   归一化漏计（反例 100买10→110减5→120平5 漏 49.685）。
/// - **同向加仓**（before·after>0 且 |after|>|before|）：entry_bar 不变（延续首次入场，
///   v0 单标量近似），不产记录。
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
    // F-06：部分减仓（同向未到 0，|after|<|before|）⟹ 逐 fill 产 TradeRecord（qty=本次减掉
    // 的手数，entry_bar 延续首次入场）。与 `apply_order` 逐 fill push trade_pnls 口径对齐，
    // 消除"减仓 PnL 有账无迹"的口径错配（消费方：§4 随机对照 same-caliber/敞口归一化）。
    let same_side = sign(units_before) != 0 && sign(units_before) == sign(units_after);
    if same_side && units_after.abs() < units_before.abs() - 1e-12 {
        if let Some(entry_bar) = *pos_entry_bar {
            let hold_bars = exit_bar.saturating_sub(entry_bar).max(1);
            trades.push(metrics::TradeRecord {
                entry_bar,
                exit_bar,
                hold_bars,
                qty: units_before.abs() - units_after.abs(), // 本次减掉的绝对手数
                long: units_before > 0.0,
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
fn apply_voice_fill(voice_qty: &mut [u32], depth: usize, fill: FillOutcome) {
    if let Some(slot) = voice_qty.get_mut(depth) {
        // 订单可“先平后开”，且开仓余量可能因现金不足被拒；声部账必须按两个真实成交段
        // 分别扣/加，绝不能按原始请求量或 action 推断。
        let closed = fill.closed_qty.round().clamp(0.0, u32::MAX as f64) as u32;
        let opened = fill.opened_qty.round().clamp(0.0, u32::MAX as f64) as u32;
        debug_assert!((fill.closed_qty - closed as f64).abs() < 1e-9);
        debug_assert!((fill.opened_qty - opened as f64).abs() < 1e-9);
        *slot = slot.saturating_sub(closed).saturating_add(opened);
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
pub(crate) fn apply_order(
    o: &Order,
    px: f64,
    fee_rate: f64,
    cash: &mut f64,
    units: &mut f64,
    entry_cost: &mut f64,
    trade_pnls: &mut Vec<f64>,
) -> FillOutcome {
    let qty = o.qty as f64;
    // 订单 → (有符号成交方向 δ, 是否纯平仓 close_only)。
    // ★信号成交（Buy/Add/Sell）：可「先平后开」翻转（开多 δ+1 / 开空 δ−1）。
    // ★纯平仓（Close/Reduce，v1 做空腿方向感知）：平掉当前持仓——平多=卖（δ−1），平空=买（δ+1），
    //   **只平不反向开**（close_only=true，剩余 qty 超持仓时不借机开反向仓）。build_exit_order 对
    //   多空两腿都产 `StrictAction::Close`（不带方向），故成交方向由**当前持仓符号**决定；空仓 ⟹ 无操作。
    // ★A'（codex GAP3 裁定清单③）：返回本次 fill 的费后已实现 PnL（透传 [`apply_fill`]，
    //   无平仓分量 = 0.0）——TW 账本 Realize 的唯一合法资金源。
    let (delta, close_only): (f64, bool) = match o.action {
        StrictAction::Buy | StrictAction::Add => (1.0, false),
        StrictAction::Sell => (-1.0, false),
        StrictAction::Reduce | StrictAction::Close => {
            if *units > 0.0 {
                (-1.0, true) // 持多 ⟹ 卖出平多
            } else if *units < 0.0 {
                (1.0, true) // 持空 ⟹ 买回平空
            } else {
                return FillOutcome::noop(); // 空仓无仓可平
            }
        }
        StrictAction::Hold | StrictAction::Wait => return FillOutcome::noop(), // 不动
    };
    apply_fill(delta, qty, close_only, px, fee_rate, cash, units, entry_cost, trade_pnls)
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
pub(crate) fn apply_fill(
    delta: f64,
    qty: f64,
    close_only: bool,
    px: f64,
    fee_rate: f64,
    cash: &mut f64,
    units: &mut f64,
    entry_cost: &mut f64,
    trade_pnls: &mut Vec<f64>,
) -> FillOutcome {
    // ★A'（codex GAP3 裁定清单③）：返回 `(费后已实现 PnL, 成交费用)`。
    // - realized：本次 fill 的费后已实现 PnL（仅段 1 平仓分量产生；无平仓 = 0.0）。结算时点硬
    //   边界（推导链第 9 条）：只锚实际平仓 fill 的 close 分支——partial close 按平掉数量结算；
    //   flip（先平后开）只对先平的反向段结算（段 2 开新仓不产 PnL）。
    // - fee_paid（M6 R 分解 Commission+Slippage 独立累计）：本次成交实付费用 = 成交名义 ×
    //   fee_rate（平仓段 close_qty·px + 开仓段 remaining·px）。已内蕴在 cash 现金流的 (1±fee)
    //   因子里，此处**独立**测得供 RDecomposition 守恒断言（校验和 = 账本净变动，非从 net_r 反推）。
    let mut realized = 0.0;
    let mut fee_paid = 0.0;
    let mut closed_qty = 0.0;
    let mut opened_qty = 0.0;
    if qty <= 0.0 || px <= 0.0 {
        return FillOutcome {
            requested_qty: qty.max(0.0),
            executed_qty: 0.0,
            closed_qty,
            opened_qty,
            rejected_qty: qty.max(0.0),
            realized,
            fee: fee_paid,
        };
    }
    let mut remaining = qty;

    // ── 段 1：平反向持仓（units 与 delta 反向 ⟹ 本次成交先减仓）。 ──
    if *units * delta < 0.0 {
        let close_qty = remaining.min(units.abs());
        if close_qty > 0.0 {
            // 平仓现金流：delta=+1（买回平空）cash 减 qty×px×(1+fee)；delta=−1（卖出平多）cash 加 qty×px×(1−fee)。
            // 统一：cash += −delta × qty × px × (1 + delta×fee_rate)。
            let cash_flow = -delta * close_qty * px * (1.0 + delta * fee_rate);
            fee_paid += close_qty * px * fee_rate; // 平仓成交费（commission+slippage+tax）
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
            realized = pnl; // A'：平仓结算事实（与 trade_pnls 同一值，返回给 TW Realize 生产者）
            closed_qty += close_qty;
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
            return FillOutcome {
                requested_qty: qty,
                executed_qty: closed_qty,
                closed_qty,
                opened_qty,
                rejected_qty: remaining,
                realized,
                fee: fee_paid,
            };
        }
        fee_paid += remaining * px * fee_rate; // 开仓成交费（commission+slippage+tax）
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
        opened_qty += remaining;
        remaining = 0.0;
    }
    FillOutcome {
        requested_qty: qty,
        executed_qty: closed_qty + opened_qty,
        closed_qty,
        opened_qty,
        rejected_qty: remaining,
        realized,
        fee: fee_paid,
    }
}

/// 单次订单的真实成交分解。`closed_qty + opened_qty = executed_qty`；未成交余量只进入
/// `rejected_qty`，不得进入执行计数、交易轨迹或声部持仓账。
/// （pub(crate)：关⑤ D8 嵌入恒等对拍（dual_ledger.rs）只读复用，行为零改。）
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FillOutcome {
    pub(crate) requested_qty: f64,
    pub(crate) executed_qty: f64,
    pub(crate) closed_qty: f64,
    pub(crate) opened_qty: f64,
    pub(crate) rejected_qty: f64,
    pub(crate) realized: f64,
    pub(crate) fee: f64,
}

impl FillOutcome {
    pub(crate) fn noop() -> Self {
        Self {
            requested_qty: 0.0,
            executed_qty: 0.0,
            closed_qty: 0.0,
            opened_qty: 0.0,
            rejected_qty: 0.0,
            realized: 0.0,
            fee: 0.0,
        }
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
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 9_000_000_000, // px 90 < 入场 100 ⟹ 止损在下方不触及
            pivot_high: 0,
            center: Some(Center { zd: 9_500_000_000, zg: 10_500_000_000, dd: 9_000_000_000, gg: 11_000_000_000, start_index: 0, end_index: si }),
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
                    (classification.clone(), Vec::new(), i as u64, i as u64)
                } else {
                    (Classification::default(), Vec::new(), i as u64, i as u64)
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
            |i| (Classification::default(), Vec::new(), i as u64, i as u64),
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
                    (classification.clone(), Vec::new(), i as u64, i as u64)
                } else {
                    (Classification::default(), Vec::new(), i as u64, i as u64)
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
    fn buy1_at3_confirmed_at7() -> impl Fn(usize) -> (Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>, u64, u64) {
        let classification = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3)]), ..Default::default() }],
        };
        move |i| {
            if i >= 7 {
                (classification.clone(), Vec::new(), i as u64, i as u64)
            } else {
                (Classification::default(), Vec::new(), i as u64, i as u64)
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
            _ => BspBits { sell3: true, ..Default::default() },
        };
        BspPoint {
            source_index: si,
            bits,
            pivot_low: 0,
            pivot_high: 20_000_000_000, // px 200 ≫ 100 ⟹ 不触及
            center: Some(Center { zd: 9_500_000_000, zg: 10_500_000_000, dd: 9_000_000_000, gg: 11_000_000_000, start_index: 0, end_index: si }),
            struct_break_dir: None,
            force: None,
        }
    }

    /// 两阶段合成闭包：bar≥7 出 buy1@3；bar≥14 追加 sell@12（class 可选）。
    fn buy_then_sell(sell_class: u8) -> impl Fn(usize) -> (Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>, u64, u64) {
        let cls_buy = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3)]), ..Default::default() }],
        };
        let cls_both = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy1_at(3), sell_at(12, sell_class)]), ..Default::default() }],
        };
        move |i| {
            if i >= 14 {
                (cls_both.clone(), Vec::new(), i as u64, i as u64)
            } else if i >= 7 {
                (cls_buy.clone(), Vec::new(), i as u64, i as u64)
            } else {
                (Classification::default(), Vec::new(), i as u64, i as u64)
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
                if i >= 7 { (cls_buy.clone(), Vec::new(), i as u64, i as u64) }
                else { (Classification::default(), Vec::new(), i as u64, i as u64) }
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
                    (cls_rebuy.clone(), Vec::new(), i as u64, i as u64)
                } else if i >= 14 {
                    (cls_sell.clone(), Vec::new(), i as u64, i as u64)
                } else if i >= 7 {
                    (cls_buy.clone(), Vec::new(), i as u64, i as u64)
                } else {
                    (Classification::default(), Vec::new(), i as u64, i as u64)
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
        assert_eq!(tw.open_legacy_legs, 0, "本场景无 ShortDiff 腿 ⟹ legacy 计数 0");
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
                    (cls_rebuy.clone(), Vec::new(), i as u64, i as u64)
                } else if i >= 14 {
                    (cls_sell.clone(), Vec::new(), i as u64, i as u64)
                } else if i >= 7 {
                    (cls_buy.clone(), Vec::new(), i as u64, i as u64)
                } else {
                    (Classification::default(), Vec::new(), i as u64, i as u64)
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

    /// ★#145 T1：entry_v=ShortDiff 腿被**一类**反向候选关闭 ⟹ CloseShortDiff（P7 子声部
    /// 关闭语义**压过触发类**——同触发类下根腿会派 CloseRoot）。端到端：父根 L1 Long 持仓 →
    /// 子 L0 Short（真父容器 Long ⟹ ShortDiff，AncOK 持父准入）→ L0 buy1@18（class=1）平子腿。
    /// typed 裁决由组合层 `StepTrace.closed` 携带（#145 前移），runner 只消费不补算。
    #[test]
    fn typed_ledger_shortdiff_close_overrides_trigger_class() {
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
            (cls, tower.clone(), i as u64, i as u64)
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
            ExitType::CloseShortDiff,
            "entry_v=ShortDiff + 一类反向触发 ⟹ P7 CloseShortDiff（压过触发类）"
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

        // 探针记账封闭性（自检）：每次 restore 调用恰好以三种方式之一终止。
        assert_eq!(
            p.restore_complete + p.restore_break_already_in_raw + p.restore_break_registry_lost,
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
        eprintln!("  └ ★暴露面（registry 丢失）: {}", p.restore_break_registry_lost);
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
                (cls, tower, classifier_incr.tower_generation(), classifier_incr.forest_epoch())
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
                |i| { let (c, t) = ci.classify_at(i); (c, t, ci.tower_generation(), ci.forest_epoch()) },
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
                    (cls, tower, classifier_incr.tower_generation(), classifier_incr.forest_epoch())
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
                entry_v: Vertical::ShortDiff,
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
    /// L0 sell1@16（子 ShortDiff：host=sub(12,16) ⟹ 真父 A (level=1, ρ=20)；附着一致经
    /// span 包含重建 ⟺ A.id）；可选 L0 buy1@18（子腿反向关闭触发，interpret 规则2 同级）。
    fn e_classification(with_child_close_trigger: bool) -> Classification {
        let buy_parent = BspPoint {
            source_index: 12,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90,
            pivot_high: 0,
            center: Some(Center { zd: 100, zg: 150, dd: 85, gg: 160, start_index: 4, end_index: 12 }),
            struct_break_dir: None,
            force: None,
        };
        let sell_child = BspPoint {
            source_index: 16,
            bits: BspBits { sell1: true, ..Default::default() },
            pivot_low: 0,
            pivot_high: 210,
            center: Some(Center { zd: 100, zg: 150, dd: 85, gg: 160, start_index: 8, end_index: 16 }),
            struct_break_dir: None,
            force: None,
        };
        let mut l0 = vec![sell_child];
        if with_child_close_trigger {
            l0.push(BspPoint {
                source_index: 18,
                bits: BspBits { buy1: true, ..Default::default() },
                pivot_low: 120,
                pivot_high: 0,
                center: Some(Center { zd: 100, zg: 150, dd: 85, gg: 160, start_index: 12, end_index: 18 }),
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
    /// TW 接线到位后启用（施工图 §6 E2 原文）；当前 v1 dual 路径 tw_final=None（诚实空）。
    #[test]
    #[ignore = "TW ShortDiff 通道未接线 v1 dual 路径（tw_final=None 诚实空）；G5 同数锁待 TW 接线后启用（施工图 §6 E2）"]
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

    /// E4（嵌套关闭 bit-exact 锁）：无 ShortDiff 触发数据（缺塔 ⟹ 全 Ambient）⟹
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
        assert!(dual.tw_final.is_none() && old.tw_final.is_none());
        assert!(dual.r_decomp.is_none() && old.r_decomp.is_none());
    }
}
