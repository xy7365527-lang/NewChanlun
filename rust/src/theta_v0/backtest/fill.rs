//! B-M3a（#87）：净额执行原语 seam——自 `runner.rs` **纯移动**（设计文档
//! `chanlun/plans/runner-rs-seam-designs-20260721.md` §M3 节拍 3）：
//! `simulate_fills` / `apply_order` / `apply_fill` / `FillOutcome` / `bar_returns`。
//! 零语义改动；`dual_ledger.rs:472` D8 对拍经 `runner::apply_order` 门面 `pub(crate) use` 继续命中。

use super::super::config::ThetaConfig;
use super::super::types::{Bar, Order, StrictAction};

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
pub(super) fn simulate_fills(
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
pub(super) fn bar_returns(equity: &[f64]) -> Vec<f64> {
    if equity.len() < 2 {
        return Vec::new();
    }
    equity
        .windows(2)
        .map(|w| if w[0] > 0.0 { w[1] / w[0] - 1.0 } else { 0.0 })
        .collect()
}

// ══════════════════════════════════════════════════════════════════════════════
// B-M3b-contract（#91）：fill loop 本体 + plan_and_fill_mtm + FillOutput(Family)
// 自 runner.rs 纯移动（设计文档 §M3 节拍 6 contract 步）。零语义改动；外部消费方经
// `runner::` 门面 `pub use` 不受影响。
// ══════════════════════════════════════════════════════════════════════════════
// ★三方合并（kimi-nest-mainline-20260717）：本文件 = theirs seam 骨架 ∧ ours 教义线语义
// 并集——overlay/plan_and_fill_mtm(_dual)/FillOutput 经符号级 3-way 合并（#145 出场
// 裁决三元化 ∧ #76/#82/#93/T5b 出场门、#183 T4 归一、#195-#201 账户镜像+shadow+裁决轨、
// #200 先平后开、#209/#237 断言硬门、#90 TwLedgerThread、#75 nest 真链门、W1 声部臂、
// L2-P5/P6 opsem 富化）；ours 独有探针/镜像/shadow 辅助区随 loop 同迁（见下方合并节）。

use std::rc::Rc;
use super::super::classifier;
use super::super::closed_loop::state::AssemblyState;
use super::super::closed_loop::transition::{hybrid_step, AssemblyEvent};
use super::super::strategy::ledger::{RiskPolicy, TwEvent, TwState};
use super::super::strategy::exit::{exit_decision_for, record_held_voice, HeldVoice};
use super::super::strategy::{AccountState, VoiceDecision};
use super::super::strategy;
use super::super::parser;
use super::data::Dataset;
use super::dual_ledger;
use super::metrics::{self, Metrics};
use super::signal::{entry_structural_stop, newly_confirmed_step};
use super::opsem_dump::{
    eta_bucket_str, force_state_str, operation_role_str, risk_mode_str,
    strict_nest_sidecar_enabled, summarize_strict_nest_certificates, t_stage_str, voice_side_str,
    OpsemDump, StrictNestSidecarCollector, OPSEM_DUMP_DIR_OVERRIDE,
};
use super::admission::{
    voice_exec_gate, nest_cert_gate_enabled,
    NestGateStats,
    NestChainGate,
    ExitNestGateStats, ExitNestGateCtx,
    k_theta_risk_gate, kappa_policy_resolved,
};
use super::admission::ChiFilterCtx;
use super::ledger::{track_position_transition, LedgerOpen, OpsemEntrySnapshot, TwLedgerThread, TypedTrade, TYPED_TRADE_SCHEMA_VERSION, VoiceVerdictRec};
#[cfg(test)]
use super::admission::{VOICE_EXEC_OVERRIDE, NEST_CERT_GATE_OVERRIDE};

// ══════════════════════════════════════════════════════════════════════════════
// 三方合并 ours 独有面（教义线 #196-#200/#209/#237）：断言探针三件套、账户镜像、
// silent_drop 判据、shadow 分歧报告路径——随 fill loop seam 同迁（原 runner.rs 私件，
// reset/count 与 silent_drop_exit_type 升格 pub(super) 经 runner 门面供既有测试消费）。
// ══════════════════════════════════════════════════════════════════════════════

// ── ★#198 断言4「首开反向隔离」探针（coverage.rs ancok_probe 同款 thread_local 模式）──
// （原「短差隔离」/shortdiff_isolation 系列，#281 更名 #283 实装。）
thread_local! {
    static REVERSE_OPEN_ISOLATION_PROBE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

fn reverse_open_isolation_probe_bump() {
    REVERSE_OPEN_ISOLATION_PROBE.with(|c| c.set(c.get() + 1));
}

/// 归零首开反向隔离探针（见证测试 run 前调用；仅本模块测试消费，比照 coverage.rs
/// ancok_probe 三件套但无跨模块消费者 ⟹ 不 pub）。
pub(super) fn reverse_open_isolation_probe_reset() {
    REVERSE_OPEN_ISOLATION_PROBE.with(|c| c.set(0));
}

/// 读取首开反向隔离探针快照（ReverseOpen fill 隔离断言在生产路径的触发笔数）。
pub(super) fn reverse_open_isolation_probe_count() -> u64 {
    REVERSE_OPEN_ISOLATION_PROBE.with(std::cell::Cell::get)
}

/// ★#198 断言4 前置快照：全部非 ReverseOpen（Core{*}/Short{*}）分实例的 (key, qty)。
#[cfg(debug_assertions)]
fn non_reverse_open_qty_snapshot(
    view: &strategy::account::ParallelAccountLedger,
) -> std::collections::HashMap<strategy::account::AccountKey, f64> {
    view.instances()
        .iter()
        .filter(|(k, _)| !matches!(k.account, strategy::account::AccountIdentity::ReverseOpen { .. }))
        .map(|(k, i)| (*k, i.qty))
        .collect()
}

/// ★#198 断言4（#185「建议断言」4 的生产执行层同款）：`fill.account == ReverseOpen ⟹
/// Δqty(Core{*})==0 且 Δqty(Short{*})==0`——首开反向过账不得碰本仓/空仓任何分实例
/// （模型层「父仓不动」曾有 SplitLegLedger 等价保证——该机已随 #282 删除，本断言
/// 守生产执行层同款）。
///
/// 度量方式 = 账本状态**差分**（post 前快照 vs post 后逐实例比对，独立度量，非重算
/// `post` 的过账逻辑）；定位 = **回归锁**（与 #145 T1 同款纪律）：`post` 当前按单 key
/// 过账本满足隔离，本断言看守的是未来改动（执行延迟净额队列的逐账户投影属
/// #199/#200 接线面）把跨账户写引入过账路径时先炸而非静默漂移。挂载点说明：实仓
/// 净额 `units` 无账户键，并行记账视图（#197 expand）是当前唯一账户投影层——断言
/// 只能落此；净额出口行为不受本断言影响（debug 构建逐笔核对，release 编译消除）。
#[cfg(debug_assertions)]
fn debug_assert_reverse_open_isolation(
    view: &strategy::account::ParallelAccountLedger,
    pre: &std::collections::HashMap<strategy::account::AccountKey, f64>,
) {
    for (k, inst) in view
        .instances()
        .iter()
        .filter(|(k, _)| !matches!(k.account, strategy::account::AccountIdentity::ReverseOpen { .. }))
    {
        let before = pre.get(k).copied().unwrap_or(0.0);
        debug_assert_eq!(
            inst.qty, before,
            "#198 断言4 首开反向隔离违例：ReverseOpen fill 改动 {:?} 分实例 qty（{} → {}）",
            k.account, before, inst.qty
        );
    }
    debug_assert_eq!(
        view.instances()
            .keys()
            .filter(|k| !matches!(k.account, strategy::account::AccountIdentity::ReverseOpen { .. }))
            .count(),
        pre.len(),
        "#198 断言4 首开反向隔离违例：ReverseOpen fill 新增 Core/Short 分实例"
    );
}

// ── ★#199 断言②③探针（#198 断言4 同款 thread_local 模式；恒在计数，
//    「断言在生产路径真实触发」以探针 >0 为凭；断言本体 = debug 构建逐笔核对）──
thread_local! {
    static T1_CORE_ZERO_PROBE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static T1_CORE_RESIDUAL_PROBE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static TYPE2_SELL_GUARD_PROBE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static TYPE2_WITH_CORE_RESIDUAL_PROBE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static RESIDUAL_CORRECTION_PROBE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// 断言②评估探针计数（一类 Core 平仓笔数）。
fn t1_core_zero_probe_bump() {
    T1_CORE_ZERO_PROBE.with(|c| c.set(c.get() + 1));
}

/// 归零断言②两枚探针（评估 zero + 违例 residual；见证测试 run 前调用；
/// 仅本模块测试消费 ⟹ 不 pub）。
pub(super) fn t1_core_zero_probe_reset() {
    T1_CORE_ZERO_PROBE.with(|c| c.set(0));
    T1_CORE_RESIDUAL_PROBE.with(|c| c.set(0));
}

/// 读取断言②探针快照（一类 Core 平仓评估笔数）。
pub(super) fn t1_core_zero_probe_count() -> u64 {
    T1_CORE_ZERO_PROBE.with(std::cell::Cell::get)
}

/// ★#209 终态违例探针：一类全平批末**级余额非零**笔数（release 构建可观测面；debug
/// 构建由批次边界 debug_assert 先行拦截）。★#237 起口径 = **批次方向侧**分量余额
/// （一类卖批查多侧/一类买批查空侧，`balance_side` 按方向拆查）。历史对照：#199 测量态
/// BTC 实测违例=1。
fn t1_core_residual_probe_bump() {
    T1_CORE_RESIDUAL_PROBE.with(|c| c.set(c.get() + 1));
}

/// 读取级余额违例探针快照（#209 终态见证材料：一类批末批次方向侧 balance(Core{L})≠0
/// 笔数——#237 拆查口径，验收口径 = 全窗 0）。
pub(super) fn t1_core_residual_probe_count() -> u64 {
    T1_CORE_RESIDUAL_PROBE.with(std::cell::Cell::get)
}

/// 断言③前半探针计数（ReverseType2 过账笔数——每笔经「不落 Core 账」约束核对）。
fn type2_sell_guard_probe_bump() {
    TYPE2_SELL_GUARD_PROBE.with(|c| c.set(c.get() + 1));
}

/// 归零断言③前半两枚探针（身份 guard + 歧义测量 with_core_residual；
/// 见证测试 run 前调用；仅本模块测试消费 ⟹ 不 pub）。
pub(super) fn type2_sell_guard_probe_reset() {
    TYPE2_SELL_GUARD_PROBE.with(|c| c.set(0));
    TYPE2_WITH_CORE_RESIDUAL_PROBE.with(|c| c.set(0));
}

/// 读取断言③前半探针快照（ReverseType2 账户约束断言在生产路径的触发笔数）。
pub(super) fn type2_sell_guard_probe_count() -> u64 {
    TYPE2_SELL_GUARD_PROBE.with(std::cell::Cell::get)
}

/// ★#199 断言③后半宽读法歧义测量（Spec 轴评审发现，待裁决）：二类卖
/// （ReverseType2，ReverseOpen/Short 身份）过账时**同级 Core{level} 残余非零**笔数。
/// 票面「qty(Core{L})>0 时必须另发 CoreResidualCorrection{L}」若按宽读法（二类卖
/// 发生时 Core 有残余即须另发纠错单），本探针即该场景的实测发生率：0 笔 = 该场景
/// 生产不存在（歧义消解）；>0 笔 = 待裁决是否另发（#185 存疑区 1：B/C 撞车裁决
/// 规则蓝图有「父仓 active」读法但 #184 未给优先级/双动作规则）。
fn type2_with_core_residual_probe_bump() {
    TYPE2_WITH_CORE_RESIDUAL_PROBE.with(|c| c.set(c.get() + 1));
}

/// 读取歧义测量探针快照（二类卖时同级 Core 残余非零笔数）。
pub(super) fn type2_with_core_residual_probe_count() -> u64 {
    TYPE2_WITH_CORE_RESIDUAL_PROBE.with(std::cell::Cell::get)
}

/// 残余纠错硬门探针计数（CoreResidualCorrection 过账笔数——每笔经残余实测非零核对）。
fn residual_correction_probe_bump() {
    RESIDUAL_CORRECTION_PROBE.with(|c| c.set(c.get() + 1));
}

/// 归零残余纠错硬门探针（见证测试 run 前调用；仅本模块测试消费 ⟹ 不 pub）。
pub(super) fn residual_correction_probe_reset() {
    RESIDUAL_CORRECTION_PROBE.with(|c| c.set(0));
}

/// 读取残余纠错硬门探针快照（CoreResidualCorrection 残余实测断言在生产路径的触发笔数）。
pub(super) fn residual_correction_probe_count() -> u64 {
    RESIDUAL_CORRECTION_PROBE.with(std::cell::Cell::get)
}

/// ★#197 并行记账视图镜像辅助（expand 只读旁路）：共享过账形状——身份解析 + 建单 + 提交成交。
///
/// 账户身份由入场冻结的 `entry_v` × 声部方向映射（[`strategy::account::identity_of`]，与
/// `reverse_exit_type` 同取入场冻结值）；数量 = 腿级 sizing 目标（与 `TypedTrade::units`
/// 同源 `SepLeg.q_units`），符号由调用侧（开/关）给出。腿级账本以决策 bar 为成交时点（与
/// `TypedTrade` 价格口径一致）；执行延迟净额队列的逐账户投影属 #198–#200。
///
/// `identity_of` 返回 `None`（Ambient×Flat）不写账：Flat 方向候选归 𝒦 记录不开腿
/// （interp.rs `assemble_gamma` 文档），不伪造归属。开/关两侧同判据 ⟹ 不会悬挂实例。
#[allow(clippy::too_many_arguments)]
fn account_mirror_post(
    view: &mut strategy::account::ParallelAccountLedger,
    entry_v: strategy::coverage::Vertical,
    side: strategy::voice::VoiceSide,
    level: u32,
    position_node_id: strategy::interp::PositionNodeId,
    signed_delta: f64,
    reason: strategy::account::ActionReason,
    px: f64,
    bar: usize,
) {
    use strategy::account::{AccountKey, AccountOrder};
    let Some(account_id) = strategy::account::identity_of(entry_v, side, level) else {
        return;
    };
    // ★#198 断言4 探针（恒在计数）：ReverseOpen fill 笔数（生产路径见证——
    // 「首开反向隔离断言真实触发」以探针 >0 为凭，断言本体见下方 debug 构建逐笔核对）。
    if matches!(account_id, strategy::account::AccountIdentity::ReverseOpen { .. }) {
        reverse_open_isolation_probe_bump();
    }
    // 断言4 前置快照（仅 ReverseOpen 过账时；debug 构建，release 编译消除）。
    #[cfg(debug_assertions)]
    let pre_non_reverse_open = matches!(account_id, strategy::account::AccountIdentity::ReverseOpen { .. })
        .then(|| non_reverse_open_qty_snapshot(view));
    // ★#199 断言③探针（恒在计数）+ 硬门（debug 逐笔核对，release 编译消除）：
    // - 前半「二类卖身份」：ReverseType2 仅落 ReverseOpen/Short 两身份，永不落 Core 账；
    // - 后半「仅残余才纠错」：CoreResidualCorrection 仅在核心残余实测非零时触发
    //   （post 前余额含被关腿在册量；与 runner 分流点 `has_residual` 同口径）。
    if reason == strategy::account::ActionReason::ReverseType2 {
        type2_sell_guard_probe_bump();
        debug_assert!(
            !matches!(account_id, strategy::account::AccountIdentity::Core { .. }),
            "#199 断言③违例：二类卖（ReverseType2）落 {:?}——合法二类卖出仅 ReverseOpen/Short 两身份",
            account_id
        );
        // ★#199 断言③后半宽读法歧义测量（不置断言，待裁决）：二类卖过账时同级
        // Core{level} 残余非零则计数（post 前余额含在飞腿；宽读法场景实测发生率）。
        if view.has_residual(strategy::account::AccountIdentity::Core { level }) {
            type2_with_core_residual_probe_bump();
        }
    }
    if reason == strategy::account::ActionReason::CoreResidualCorrection {
        residual_correction_probe_bump();
        debug_assert_ne!(
            view.balance(account_id),
            0.0,
            "#199 断言③违例：CoreResidualCorrection 在核心残余为零时触发（{:?}）——仅残余才纠错",
            account_id
        );
    }
    view.post(
        AccountOrder {
            key: AccountKey::new(account_id, level, position_node_id),
            reason,
            qty_delta: signed_delta,
            decision_bar: bar,
        },
        px,
        bar,
    );
    // ★#198 断言4 逐笔核对：ReverseOpen fill 后 Core{*}/Short{*} 分实例数量零变动。
    #[cfg(debug_assertions)]
    if let Some(pre) = pre_non_reverse_open {
        debug_assert_reverse_open_isolation(view, &pre);
    }
    // ★#199 断言②评估探针（恒在计数）：一类 Core 平仓评估笔数（#185 断言2：
    // filled(T1CoreClose{L}) ⟹ execution_ledger.qty(Core{L})==0）。
    //
    // ★#209 终态硬门**挂在批次边界**（π fill loop 反向关闭循环结束点，见该处注释）：
    // 多核心腿场景一类全平产生多条 AccountOrder（每腿恰一），逐笔 post 后账户余额
    // 仍含未平同级腿——单笔粒度误报，教义口径是「一类批末该级清仓」。本点只保留
    // 评估探针（按腿计数，与 fills 分类笔数对账口径不变）。历史对照（勿删）：#199
    // 测量态 BTC train 窗实测违例=1（一类只关首条，L=1 残留 277.9 单位）。
    if reason == strategy::account::ActionReason::ReverseType1
        && matches!(account_id, strategy::account::AccountIdentity::Core { .. })
    {
        t1_core_zero_probe_bump();
    }
}

/// ★#197：开腿镜像（reason=Open，符号 = 多正空负）。开腿方向恒非 Flat（见 `account_mirror_post`）。
///
/// ★#200 理由轴（账户正交，OpenShort 通道）：二类反向候选开**空仓账**（`identity_of` 解析
/// = `Short`）⟹ 理由 `OpenShort`（spec WP-2 修复 c）；其余开仓保留 `Open`（一类首开/加仓、
/// 短差开、级联开）。判据单源 = [`strategy::account::reason_of_open`]，账户身份经
/// [`strategy::account::identity_of`]（与关闭侧 `reason_of_reverse_close` 同判据，不镜像）。
fn account_mirror_open(
    view: &mut strategy::account::ParallelAccountLedger,
    c: &strategy::interp::Candidate,
    level: u32,
    position_node_id: strategy::interp::PositionNodeId,
    units: f64,
    px: f64,
    bar: usize,
) {
    use strategy::voice::VoiceSide;
    let signed = match c.dir {
        VoiceSide::Long => units,
        VoiceSide::Short => -units,
        // 结构不可达（opened 腿恒有方向，𝒦 记录不进 opened）；即便到达，
        // identity_of(*, Flat)=None ⟹ post 跳过，0.0 永不写账。
        VoiceSide::Flat => 0.0,
    };
    // #200：开仓理由 = 通道归属（二类 × Short 账户 ⟹ OpenShort）；identity None（Flat）时
    // 理由不被消费（post 跳过），给 Open 与全域口径一致（不伪造通道标注）。
    let reason = strategy::account::identity_of(c.role.v, c.dir, level).map_or(
        strategy::account::ActionReason::Open,
        |a| strategy::account::reason_of_open(a, c.bsp_class),
    );
    account_mirror_post(
        view,
        c.role.v,
        c.dir,
        level,
        position_node_id,
        signed,
        reason,
        px,
        bar,
    );
}

/// ★#197：关腿镜像（账户身份与退出理由正交——理由由调用通道给出，符号 = 开侧反向）。
fn account_mirror_close(
    view: &mut strategy::account::ParallelAccountLedger,
    open: &LedgerOpen,
    level: u32,
    reason: strategy::account::ActionReason,
    px: f64,
    bar: usize,
) {
    use strategy::voice::VoiceSide;
    let signed = match open.position_node_id.side {
        VoiceSide::Long => -open.units,
        VoiceSide::Short => open.units,
        // 结构不可达（同开侧）；identity_of(*, Flat)=None ⟹ 0.0 永不写账。
        VoiceSide::Flat => 0.0,
    };
    account_mirror_post(
        view,
        open.entry_v,
        open.position_node_id.side,
        level,
        open.position_node_id,
        signed,
        reason,
        px,
        bar,
    );
}

/// ★#198：silent drop（§13 AncOK 连带剪/Stale prune，无触发信号）的 typed 归属判据。
///
/// 账户身份与退出理由正交（#185 修复 a）：账户由 #197 [`strategy::account::identity_of`]
/// （entry_v × 方向）映射（FollowParent 级联核心仓 ⟹ `Core{level}`），剪枝理由由
/// `TypedTrade::via_structural_prune` / `ActionReason::StructuralPrune` 表达——不再用
/// `ExitType` 同时表达两者（同类错桶的根源）。
///
/// 判据：仅 `entry_v == ReverseOpen` 的首开反向腿记 `CloseReverseOpen`；其余（Ambient 根 /
/// FollowParent 级联核心仓）归 `CloseRoot`（core structural exit，account=`Core{level}`、
/// reason=`StructuralPrune`，经 #197 account 层映射）。
///
/// μ 分桶统计口径（随本判据更新，2026-07-23）：`ExitType` 诊断切片维持 #180 冻结铁律
/// （不进 `MuClass` 桶键、不进 χ_t 门控、不进 δ-free 主裁决聚合基），代码零改动；变化的是
/// 桶的**生产语义**——`CloseReverseOpen` 桶自此只含真首开反向腿（FollowParent 级联核心仓的
/// 结构剪枝归 `CloseRoot`），W-VERIFY 5 变体拆解与 `typed_ledger_btc_smoke` 分桶计数按
/// 新口径阅读（BTC train 窗实证翻动：CloseRoot 6→8、CloseReverseOpen 10→8，五枚举总数
/// 25 不变，见 `btc_prune_leg_exit_type_matches_account_identity` 见证注释）。
pub(super) fn silent_drop_exit_type(entry_v: strategy::coverage::Vertical) -> strategy::interp::ExitType {
    // ★#198 新口径（#185 修复 a）：仅首开反向腿记 CloseReverseOpen；Ambient/FollowParent 归
    // CloseRoot（core structural exit——账户侧 Core{level}+StructuralPrune，经 #197 映射）。
    if entry_v == strategy::coverage::Vertical::ReverseOpen {
        strategy::interp::ExitType::CloseReverseOpen
    } else {
        strategy::interp::ExitType::CloseRoot
    }
}

#[cfg(test)]
thread_local! {
    /// 测试注入点：Some(path) ⟹ **本线程**的 fill loop 在 run 结束落 shadow 分歧报告。
    /// 进程级 env 会被并行测试同时读到并覆写同一路径（opsem 2026-07-13 竞态同款教训）；
    /// 线程局部对并行测试不可见。
    pub(super) static SHADOW_DIVERGENCE_PATH_OVERRIDE: std::cell::RefCell<Option<std::path::PathBuf>> =
        std::cell::RefCell::new(None);
}

/// #196 shadow 分歧报告落盘路径：线程局部 override（测试）优先于进程 env
/// `THETA_V0_SHADOW_DIVERGENCE_PATH`；未设/空串 ⟹ None（零 IO，bit-exact 原路径）。
pub(super) fn shadow_divergence_path() -> Option<std::path::PathBuf> {
    #[cfg(test)]
    {
        if let Some(p) = SHADOW_DIVERGENCE_PATH_OVERRIDE.with(|c| c.borrow().clone()) {
            return Some(p);
        }
    }
    std::env::var_os("THETA_V0_SHADOW_DIVERGENCE_PATH")
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
}
pub(super) fn pi_theta_fill_loop<F>(
    classify_at: F,
    bars: &[Bar],
    initial_nav: f64,
    config: &ThetaConfig,
    chi: Option<ChiFilterCtx>,
) -> FillOutput
where
    F: FnMut(usize) -> (classifier::Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>, Vec<usize>, u64, u64),
{
    // ★M5 wrapper：overlay=None ⟹ 现有净额路径逐字节不变（bit-exact）。overlay 簿接线走
    // [`pi_theta_fill_loop_overlay`]（run_theta_v0_pi_overlay arm 传 Some）。
    pi_theta_fill_loop_overlay(classify_at, bars, initial_nav, config, chi, None, None)
}

/// ★W1 声部独立执行臂（churn 修复，netting-vs-voice-execution-audit-20260719 §7）：与
/// [`pi_theta_fill_loop`] **同一决策路径**——净额影子账本（cash/units/entry_cost 全部原有
/// 逻辑原样跑）驱动决策层 ⟹ typed_ledger/TW/sep_legs 与净额臂逐字节一致；执行投影换为
/// [`VoiceExecBook`](super::super::strategy::overlay_state::VoiceExecBook)——
/// ① 声部独立持仓（hedge-mode (Q⁺,Q⁻)，PDF §10.2）；② 事件驱动 fill（消费同一
/// `pi_theta_step_traced` 的 step_trace 五类生命周期事件，禁第二裁决源；无事件 bar 零订单）；
/// ③ sizing 冻结（q_v 取开仓 bar 决策层 `SepLeg.q_units` 取整落簿，存续期不随 NAV/价重定）。
/// 生产接入 = `run_theta_v0_pi_overlay` 的 env `VOICE_EXEC=1` gate（默认关闭 = 净额臂 bit-exact）。
#[cfg(test)]
pub(super) fn pi_theta_fill_loop_voice<F>(
    classify_at: F,
    bars: &[Bar],
    initial_nav: f64,
    config: &ThetaConfig,
    chi: Option<ChiFilterCtx>,
    voice: &mut super::super::strategy::overlay_state::VoiceExecBook,
) -> FillOutput
where
    F: FnMut(usize) -> (classifier::Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>, Vec<usize>, u64, u64),
{
    pi_theta_fill_loop_overlay(classify_at, bars, initial_nav, config, chi, None, Some(voice))
}

/// #292（T2 落点门控接线；★★触发源改码，用户裁定 2026-07-26）：单 bar 内驱动中枢生命周期
/// 事件（born/broken/reset/superseded，与 `opsem_dump::feed_center_lifecycle` 同款
/// `consume_chain`/`push_point` 驱动方式，状态独立不交叉——同一张塔链表的两个只读消费者）
/// + **次级别买卖点** → `CenterOscillationTrigger` 构造与 `on_trigger` 消费（A 裁定：中枢身份取
/// 本级 `alive_center()`；B 裁定：触发主信号源 = 次级别已确认买卖点，盘背非必要条件——
/// `PanDivTrigger` 降格为可选辅助，本函数不再消费它，见 `oscillation` 模块头）。纯函数化——
/// 不依赖 `tower_i`/pan_div 门内部，可脱离完整 fill 循环直接单测。顺序：先喂本 bar 生命周期
/// 事件（使 `alive_center()` 反映本 bar 内已发生的死亡），再消费本 bar 次级别买卖点触发。
///
/// ★#292 续修（二轮评审浮出的挂起悬空泄漏）：`ChainConsumed::Rebased` 分支接入
/// `osc_books[lvl].on_chain_rebase`——重基后挂起按身份核对新链，迁移/终结二分，禁悬空。
///
/// ★issue #357：新增 `witness` 观测参数（[`oscillation_campaign::CampaignWiringWitness`]）——
/// 逐挂起终结来源（挂起归宿）+ 逐触发构造结果（`CenterNotAlive` 丢弃率分母/分子）落账，
/// 纯观测不改变本函数既有决策（动作产出逻辑逐字节不变，同 `CenterOscillationActionRecord`
/// 既有先例）。
fn step_center_oscillation(
    bar: usize,
    classification_i: &classifier::Classification,
    classification_step: &classifier::Classification,
    cl_machines: &mut Vec<classifier::center_lifecycle::CenterEventMachine>,
    osc_books: &mut Vec<super::super::strategy::center_oscillation_trade::CenterOscillationBook>,
    witness: &mut super::super::strategy::oscillation_campaign::CampaignWiringWitness,
) -> Vec<super::super::strategy::center_oscillation_trade::CenterOscillationActionRecord> {
    use classifier::center_lifecycle::{CenterEventMachine, CenterId, ChainConsumed, PointOutcome};
    use super::super::strategy::center_oscillation_trade::{
        CenterOscillationActionRecord, CenterOscillationBook, CenterOscillationTrigger,
    };
    use super::super::strategy::voice::VoiceSide;

    let mut actions = Vec::new();
    let n_levels = classification_i.levels.len();
    while cl_machines.len() < n_levels {
        let lvl = cl_machines.len() as u32;
        cl_machines.push(CenterEventMachine::new(lvl));
        osc_books.push(CenterOscillationBook::new(lvl));
    }
    for lvl in 0..n_levels {
        let chain: &[super::super::types::Center] = &classification_i.levels[lvl].centers;
        // #292 续修：重基（`ChainConsumed::Rebased`）到达时按身份核对新链——挂起对该级的
        // 处置见 `CenterOscillationBook::on_chain_rebase`（F 裁定：迁移/终结二分，禁悬空）。
        match cl_machines[lvl].consume_chain(chain) {
            ChainConsumed::Advanced { events, .. } => {
                for ev in events.iter() {
                    for outcome in osc_books[lvl].on_lifecycle_event(ev) {
                        witness.record_suspension_source(outcome.side, outcome.source);
                        if let Some(action) = outcome.cover_action {
                            actions.push(CenterOscillationActionRecord {
                                bar,
                                level: lvl as u32,
                                center: outcome.center,
                                action,
                                side: outcome.side, // ★#381：收手回补归属该终结所在的持仓侧
                            });
                        }
                    }
                }
            }
            ChainConsumed::Rebased { .. } => {
                for outcome in osc_books[lvl].on_chain_rebase(chain) {
                    witness.record_suspension_source(outcome.side, outcome.source);
                    if let Some(action) = outcome.cover_action {
                        actions.push(CenterOscillationActionRecord {
                            bar,
                            level: lvl as u32,
                            center: outcome.center,
                            action,
                            side: outcome.side, // ★#381：收手回补归属该终结所在的持仓侧
                        });
                    }
                }
            }
            ChainConsumed::Adopted { .. } => {}
        }
        if let Some(step_level) = classification_step.levels.get(lvl) {
            for p in step_level.bsp.iter() {
                let target = match p.center {
                    Some(classifier::bsp::OwnerRef::Center(c)) => Some(CenterId::of(&c)),
                    _ => None,
                };
                if let Ok(PointOutcome::Event(ev)) = cl_machines[lvl].push_point(p.bits, p.source_index, target) {
                    for outcome in osc_books[lvl].on_lifecycle_event(&ev) {
                        witness.record_suspension_source(outcome.side, outcome.source);
                        if let Some(action) = outcome.cover_action {
                            actions.push(CenterOscillationActionRecord {
                                bar,
                                level: lvl as u32,
                                center: outcome.center,
                                action,
                                side: outcome.side, // ★#381：收手回补归属该终结所在的持仓侧
                            });
                        }
                    }
                }
            }
        }
    }
    // #292 触发源改码（B 裁定）：开启臂生产触发的唯一驱动源 = 次级别（lvl-1）本 bar 已确认
    // 买卖点——本级中枢在场（`alive_center()`，A 裁定）+ 次级别信号方向 + 该点价格 ⟹ 边界侧
    // 独立价格判据（`CenterOscillationTrigger::new` 同口径：卖点价=`pivot_high`、买点价=
    // `pivot_low`，与 `signal.rs::entry_structural_stop` 的 Long→pivot_low/Short→pivot_high
    // 同一读法）。level 0 无次级别（L0 是递归底），故从 lvl=1 起。盘背证据不参与（B 裁定：
    // 非必要条件）。
    for lvl in 1..n_levels {
        let Some(sub_level) = classification_step.levels.get(lvl - 1) else { continue };
        let alive = cl_machines[lvl].alive_center().map(|(c, _)| CenterId::of(&c));
        for p in sub_level.bsp.iter() {
            let (signal_side, price) = if p.bits.conf_plus() {
                (VoiceSide::Long, p.pivot_low)
            } else if p.bits.conf_minus() {
                (VoiceSide::Short, p.pivot_high)
            } else {
                continue; // 无买/卖侧确认 bit（不应出现在 bsp 列表中）——防御性跳过，不构造。
            };
            let trigger_result =
                CenterOscillationTrigger::new(lvl as u32, alive, signal_side, price, p.source_index);
            witness.record_trigger_result(&trigger_result);
            if let Ok(trigger) = trigger_result {
                // ★#381：同一次触发对**两侧**各产一条镜像动作（上沿：多头减/空头补；下沿：
                // 多头补/空头减，见 `CenterOscillationBook::on_trigger_side`）——两侧挂起表
                // 独立，各自的幽灵门各自把关（未挂起的收口腿仍返回 `None` 不构造）。侧序固定
                // Long→Short（确定性，与挂起表迭代序同锚）。
                for side in [VoiceSide::Long, VoiceSide::Short] {
                    if let Some(action) = osc_books[lvl].on_trigger_side(side, trigger) {
                        actions.push(CenterOscillationActionRecord {
                            bar,
                            level: lvl as u32,
                            center: trigger.center(),
                            action,
                            side,
                        });
                    }
                }
            }
        }
    }
    actions
}

/// issue #357（T4/#294 生产实例化）：`step_center_oscillation` 产出的减/补动作 → 每仓
/// campaign 生死驱动 + 对偶事件记账（ShortDiff 成本基 + Realize 盈亏，经 closed_loop 带门
/// 通道）的接线点。纯函数化——可脱离完整 fill 循环直接单测（同 `step_center_oscillation`
/// 先例），不读取 `tower_i`/pan_div 门内部。
///
/// 顺序：先按本仓口径（编排者裁定 A：`Core{level}` 身份账户）逐级 `sync_position`（本级持仓
/// 空⟺持仓生死判据），再逐条消费本 bar 新产出的减/补动作（`apply_action`）——与生产调用点
/// `pi_theta_fill_loop_overlay` 的既有顺序一致。
///
/// ★#381（本票）：**两侧对称支持**——`(level, side)` 键的 campaign 各自独立生死/记账，空头侧
/// 短差为多头镜像（减=回补空头、补=加回空头，判据在 `CenterOscillationBook::on_trigger_side`，
/// 记账镜像在 `ShortDiffAccount`）。下段 #357 关票条件 C 的「只支持多头侧」处置**已被本票取代**，
/// 保留为谱系注记：`unsupported_short_position_count` 桶随之退役。
///
/// ★issue #357 关票条件 C（★已被 #381 取代，下段为历史口径）：**本接线只支持多头侧 campaign**（方案2）——`account_view.balance(account)`
/// 对同身份实例有符号净额求和（account.rs:341-347，多正空负），而 `identity_of` 把
/// `(FollowParent,Short)` 也映到 `Core{level}`（顺父级联空腿）；若改用净额，本级持有空头仓位时
/// 净额可能非正，会被 `sync_position` 的 `units()>0` 判据误判为空仓——campaign 永不开局，空头
/// 侧的震荡短差整支被静默丢弃。改用 `balance_side`/`cost_basis_side` 只读**多头侧**分量
/// （`VoiceSide::Long`），空头侧完全不参与本仓 campaign 的生死判据/sizing 现算——本级若持有
/// 空头仓位，多头侧读数如实为空仓，结构信号触发时的拒绝不算「预期经济场景」，而是「本接线未
/// 支持空头 campaign」，由下方 `is_short_side_held` 传给 witness 单独分桶
/// （`unsupported_short_position_count`，不与真空仓的 `no_active_campaign_count` 混计）。
/// `money` 字段口径 `as i64` 截断：**不再**援引 `TwLedgerThread::sync_basis_raw`
/// 先例——该先例是 `(units.abs()*entry_cost.abs())`（空头取绝对额=在险市值），与本接线改分侧
/// 后的多头侧口径同为正数但成因不同，两者截断写法只是形似，不构成同一惯例，不再混引。
fn drive_campaign_wiring(
    bar: usize,
    n_levels: u32,
    account_view: &strategy::account::ParallelAccountLedger,
    new_osc_actions: &[super::super::strategy::center_oscillation_trade::CenterOscillationActionRecord],
    price: i64,
    risk_mode: super::super::closed_loop::state::RiskMode,
    campaign_book: &mut super::super::strategy::oscillation_campaign::CampaignBook,
    witness: &mut super::super::strategy::oscillation_campaign::CampaignWiringWitness,
) {
    use super::super::strategy::voice::VoiceSide;

    for lvl in 0..n_levels {
        let account = strategy::account::AccountIdentity::Core { level: lvl };
        // ★#381：逐（级别, 侧）各自 sync——空头侧余额是负数（多正空负，account.rs:341-347），
        // 按侧**翻符号**（非取绝对值）折成「持有份数」：`units>0 ⟺ 该侧有仓`，两侧口径由此
        // 对称。取绝对值会把「多头侧余额意外为负」这类异常态伪装成有仓；翻符号则让两侧各自
        // 保持「符号不对即视同空仓」的既有多头侧行为（多头侧读数逐字节不变）。
        // `cost_basis_side` 恒为正的在险市值口径（account.rs:292 `delta.abs()*fill_px`），不翻。
        // ★#380 项二：本读数逐 bar 同时喂两处——开局时冻结为 sizing 基准（`CoreCostBasisSnapshot`），
        // 存续期刷新 campaign 的 `current_units`（防线读当前的现值来源）。
        for side in [VoiceSide::Long, VoiceSide::Short] {
            let signed = account_view.balance_side(account, side);
            let held = if matches!(side, VoiceSide::Short) { -signed } else { signed };
            let snapshot = super::super::strategy::short_diff_bucket::CoreCostBasisSnapshot::new(
                held as i64,
                account_view.cost_basis_side(account, side) as i64,
            );
            if let Some(event) = campaign_book.sync_position(lvl, side, snapshot, bar) {
                witness.record_lifecycle(event);
            }
        }
    }
    for rec in new_osc_actions {
        witness.record_action(rec.level, rec.side, rec.action);
        match campaign_book.apply_action(rec.level, rec.side, rec.action, price, risk_mode, rec.center) {
            // ★#380 项三/项一/项四：旧版 `Ok(_outcome) => {}` 把成功产出整个丢弃——阶段推进事件
            // （`RecoverCapital`/`EnterEarning`，#368 切换开关的唯一可读证据）、亏损入账、挂起
            // 冲抵归属全部不可见。现逐项落 witness（纯观测，不改变任何决策）。
            Ok(outcome) => {
                if outcome.loss_accounted {
                    witness.record_loss_accounted(rec.side);
                }
                witness.record_cover(rec.side, &outcome.cover);
                if let Some(ev) = outcome.stage_event {
                    let bars_since_open = campaign_book
                        .campaign(rec.level, rec.side)
                        .map(|c| c.bars_since_open(bar))
                        .unwrap_or(0);
                    witness.record_stage_event(rec.level, rec.side, bar, bars_since_open, ev);
                }
            }
            Err(violation) => {
                // ★#381：`is_short_side_held` 随 `unsupported_short_position_count` 桶退役——
                // 空头侧已有自己的 `(level, Short)` campaign，拒绝按侧自解释。
                witness.record_violation(rec.side, violation);
            }
        }
    }
}

#[cfg(test)]
mod campaign_wiring_tests {
    //! issue #357 单测：本仓取数（`Core{level}` 身份账户）→ campaign 生死 + 减/补动作真实
    //! 记账。覆盖：① 本仓口径落地（account_view 取数正确）；② sizing=1/3 落地（经
    //! `oscillation_campaign::OscillationCampaign::reduce_units` 间接见证）；③ campaign
    //! sync_position 随真实持仓事件流生死；④ witness 计数正确。
    use super::*;
    use super::super::super::strategy::account::{AccountIdentity, AccountKey, AccountOrder, ActionReason};
    use super::super::super::strategy::center_oscillation_trade::{
        CenterOscillationAction, CenterOscillationActionRecord,
    };
    use super::super::super::strategy::interp::{EntryCertificate, PositionNodeId};
    use super::super::super::strategy::ledger::TStage;
    use super::super::super::strategy::oscillation_campaign::{CampaignBook, CampaignWiringWitness};
    use super::super::super::strategy::voice::VoiceSide;
    use super::super::super::classifier::recursive_tower::ElementId;
    use super::super::super::closed_loop::state::RiskMode;

    fn core_key(level: u32) -> AccountKey {
        AccountKey::new(
            AccountIdentity::Core { level },
            level,
            PositionNodeId {
                carrier: ElementId { level, ordinal: 0 },
                entry_certificate: Some(EntryCertificate { level, source_index: 0 }),
                side: VoiceSide::Long,
                generation: 0,
            },
        )
    }

    fn record(level: u32, action: CenterOscillationAction) -> CenterOscillationActionRecord {
        record_side(level, VoiceSide::Long, action)
    }

    /// ★#381：分侧动作记录构造（`record` = 多头侧特化，既有用例不动）。
    fn record_side(
        level: u32,
        side: VoiceSide,
        action: CenterOscillationAction,
    ) -> CenterOscillationActionRecord {
        CenterOscillationActionRecord {
            side,
            bar: 0,
            level,
            center: classifier::center_lifecycle::CenterId::of(&super::super::super::types::Center {
                zd: 100,
                zg: 200,
                dd: 98,
                gg: 202,
                start_index: 0,
                end_index: 10,
            }),
            action,
        }
    }

    /// ★本仓取数正确：`account_view` 开仓 300 股 @10（cost_basis=3000）后，`drive_campaign_wiring`
    /// 应据此对 level 0 开局 campaign（notional_in/holding=3000，与 #197 account 层 `Core{0}`
    /// 取数一致），非编造/写死值。
    #[test]
    fn drive_campaign_wiring_reads_core_account_and_opens_campaign() {
        let mut account_view = strategy::account::ParallelAccountLedger::new();
        let open = AccountOrder {
            key: core_key(0),
            reason: ActionReason::Open,
            qty_delta: 300.0,
            decision_bar: 0,
        };
        account_view.post(open, 10.0, 0); // 300 股 @10 ⟹ Core{0} balance=300, cost_basis=3000

        let mut book = CampaignBook::new();
        let mut witness = CampaignWiringWitness::new();
        drive_campaign_wiring(0, 1, &account_view, &[], 12, RiskMode::Normal, &mut book, &mut witness);

        let campaign = book.campaign(0, VoiceSide::Long).expect("Core{0} 持仓非空 ⟹ 应已开局 campaign");
        assert_eq!(campaign.tw().notional_in, 3_000, "notional_in=本仓口径取数 cost_basis（非编造）");
        assert_eq!(campaign.tw().holding, 3_000);
        assert_eq!(campaign.tw().stage, TStage::CostReduction);
        assert_eq!(witness.lifecycle_opened.get("long"), Some(&1), "witness 分侧记录一次开仓生事件（多头侧）");
    }

    /// ★sizing=1/3 落地 + 动作按 (级别, 动作) 归属正确：Core{0} 持仓 300 股，`Reduce` 动作应
    /// 卖出 100（300/3，#348 裁定），witness 归属分桶恰记一次 (0, reduce)。
    #[test]
    fn drive_campaign_wiring_sizes_reduce_to_one_third_and_tallies_action_by_level() {
        let mut account_view = strategy::account::ParallelAccountLedger::new();
        let open = AccountOrder {
            key: core_key(0),
            reason: ActionReason::Open,
            qty_delta: 300.0,
            decision_bar: 0,
        };
        account_view.post(open, 10.0, 0);

        let mut book = CampaignBook::new();
        let mut witness = CampaignWiringWitness::new();
        // 先开局（无动作），再喂一条 Reduce。
        drive_campaign_wiring(0, 1, &account_view, &[], 12, RiskMode::Normal, &mut book, &mut witness);
        let actions = vec![record(0, CenterOscillationAction::Reduce)];
        drive_campaign_wiring(0, 1, &account_view, &actions, 12, RiskMode::Normal, &mut book, &mut witness);

        let campaign = book.campaign(0, VoiceSide::Long).unwrap();
        assert_eq!(
            campaign.short_diff().bucket().open_units(),
            100,
            "sizing=当时持仓(300)/3=100（#348 裁定，整数除法）落地"
        );
        assert_eq!(witness.action_by_level.get(&(0, "long", "reduce")), Some(&1), "归属分桶：level 0 Reduce 恰一次");
        assert!(witness.no_active_campaign_count.is_empty(), "campaign 已开局 ⟹ 无 NoActiveCampaign 拒绝");
        assert_eq!(witness.other_violation_count, 0, "接线正常路径下其余通道拒绝恒 0");
    }

    /// ★campaign 随真实事件流生死：Core{0} 从空仓→持仓→全平，`drive_campaign_wiring` 逐 bar
    /// 消费 `account_view` 真实状态（非模拟快照），campaign 应恰好随之开局/终结。
    #[test]
    fn drive_campaign_wiring_lifecycle_follows_real_account_event_stream() {
        let mut account_view = strategy::account::ParallelAccountLedger::new();
        let mut book = CampaignBook::new();
        let mut witness = CampaignWiringWitness::new();

        // bar0：仍空仓——no-op。
        drive_campaign_wiring(0, 1, &account_view, &[], 10, RiskMode::Normal, &mut book, &mut witness);
        assert!(book.campaign(0, VoiceSide::Long).is_none());

        // bar1：真实开仓 fill（200 股 @15）——campaign 应开局。
        let open = AccountOrder {
            key: core_key(0),
            reason: ActionReason::Open,
            qty_delta: 200.0,
            decision_bar: 1,
        };
        account_view.post(open, 15.0, 1);
        drive_campaign_wiring(0, 1, &account_view, &[], 15, RiskMode::Normal, &mut book, &mut witness);
        assert!(book.campaign(0, VoiceSide::Long).is_some(), "真实持仓事件流 ⟹ campaign 开局");
        assert_eq!(witness.lifecycle_opened.get("long"), Some(&1));

        // bar2：真实全平 fill——campaign 应终结。
        let close = AccountOrder {
            key: core_key(0),
            reason: ActionReason::ReverseType1,
            qty_delta: -200.0,
            decision_bar: 2,
        };
        account_view.post(close, 18.0, 2);
        drive_campaign_wiring(0, 1, &account_view, &[], 18, RiskMode::Normal, &mut book, &mut witness);
        assert!(book.campaign(0, VoiceSide::Long).is_none(), "真实全平事件流 ⟹ campaign 终结");
        assert_eq!(witness.lifecycle_died.get("long"), Some(&1));
    }

    /// ★`NoActiveCampaign` 分桶：向未开局的级别喂动作，`apply_action` 返回 `NoActiveCampaign`——
    /// 本仓无持仓时结构信号仍可能触发（#292 CenterOscillationBook「无门」设计，独立于本级
    /// Core 持仓状态），这是**预期经济场景**而非接线错误，故计入 `no_active_campaign_count`
    /// 而非 `other_violation_count`（真正的接线/记账错误警报，本用例应恒 0）。
    #[test]
    fn drive_campaign_wiring_tallies_no_active_campaign_separately_from_other_violations() {
        let account_view = strategy::account::ParallelAccountLedger::new(); // 空仓——无 campaign
        let mut book = CampaignBook::new();
        let mut witness = CampaignWiringWitness::new();
        let actions = vec![record(0, CenterOscillationAction::Reduce)];
        drive_campaign_wiring(0, 1, &account_view, &actions, 12, RiskMode::Normal, &mut book, &mut witness);
        assert_eq!(witness.no_active_campaign_count.get("long"), Some(&1), "空仓级别喂动作 ⟹ NoActiveCampaign 按侧分桶计数（预期读数）");
        assert_eq!(witness.other_violation_count, 0, "非 NoActiveCampaign 的其余违规恒 0（本用例不触发）");
    }

    fn short_key(level: u32) -> AccountKey {
        AccountKey::new(
            AccountIdentity::Core { level },
            level,
            PositionNodeId {
                carrier: ElementId { level, ordinal: 0 },
                entry_certificate: Some(EntryCertificate { level, source_index: 0 }),
                side: VoiceSide::Short,
                generation: 0,
            },
        )
    }

    /// ★#381 验收①：本级只持空头仓位（FollowParent×Short 顺父级联核心仓，净额为负）时，
    /// `drive_campaign_wiring` 现在为**空头侧**开局 campaign——`units` 由分侧余额**翻符号**
    /// 折得（−50 → 50，非取绝对值，见 `drive_campaign_wiring` 内注释），`notional_in` 取分侧
    /// 成本基（在险市值 1000，恒为正）。原 #357 的
    /// `unsupported_short_position_count` 桶随之退役（本用例即其退役的正面对照）。
    #[test]
    fn drive_campaign_wiring_opens_short_side_campaign_from_short_only_core() {
        let mut account_view = strategy::account::ParallelAccountLedger::new();
        let open_short = AccountOrder {
            key: short_key(0),
            reason: ActionReason::Open,
            qty_delta: -50.0,
            decision_bar: 0,
        };
        account_view.post(open_short, 20.0, 0); // 空头 50 股 @20（净额=-50，多头侧=0）

        let mut book = CampaignBook::new();
        let mut witness = CampaignWiringWitness::new();
        drive_campaign_wiring(0, 1, &account_view, &[], 20, RiskMode::Normal, &mut book, &mut witness);
        assert!(book.campaign(0, VoiceSide::Long).is_none(), "多头侧真空仓 ⟹ 多头 campaign 不开局");
        let short_campaign =
            book.campaign(0, VoiceSide::Short).expect("★#381：空头侧持仓 ⟹ 空头 campaign 开局");
        assert_eq!(short_campaign.side(), VoiceSide::Short);
        assert_eq!(short_campaign.current_units(), 50, "units=分侧余额翻符号（多正空负，−(−50)=50）");
        assert_eq!(short_campaign.tw().notional_in, 1_000, "notional_in=空头侧成本基（在险市值 50×20）");
        assert_eq!(
            witness.lifecycle_opened.get("short"),
            Some(&1),
            "恰一次开仓生（空头侧）"
        );
        assert_eq!(witness.lifecycle_opened.get("long"), None, "多头侧无仓 ⟹ 该侧桶不出现");

        // 空头侧「减」=回补空头：sizing=开局冻结 50/3=16，落空头 campaign 而非多头。
        let actions = vec![record_side(0, VoiceSide::Short, CenterOscillationAction::Reduce)];
        drive_campaign_wiring(0, 1, &account_view, &actions, 18, RiskMode::Normal, &mut book, &mut witness);
        assert_eq!(
            book.campaign(0, VoiceSide::Short).unwrap().short_diff().bucket().open_units(),
            16,
            "空头侧 sizing=当时持仓(50)/3=16（整数除法），记入空头 campaign"
        );
        assert_eq!(
            witness.action_by_level.get(&(0, "short", "reduce")),
            Some(&1),
            "动作分桶带持仓侧（#381 键增侧维）"
        );
        assert!(witness.no_active_campaign_count.is_empty(), "空头侧已开局 ⟹ 无 NoActiveCampaign 拒绝");
        assert_eq!(witness.other_violation_count, 0, "正常记账路径无警报");
    }

    /// ★#381 验收③：多空并存互不污染——两侧各自独立 campaign，`notional_in` 各按各侧成本基
    /// （多头 3000 / 空头 1000），生死互不牵连。
    #[test]
    fn drive_campaign_wiring_long_and_short_campaigns_coexist_without_contamination() {
        let mut account_view = strategy::account::ParallelAccountLedger::new();
        account_view.post(
            AccountOrder { key: core_key(0), reason: ActionReason::Open, qty_delta: 300.0, decision_bar: 0 },
            10.0,
            0,
        );
        account_view.post(
            AccountOrder { key: short_key(0), reason: ActionReason::Open, qty_delta: -50.0, decision_bar: 0 },
            20.0,
            0,
        );

        let mut book = CampaignBook::new();
        let mut witness = CampaignWiringWitness::new();
        drive_campaign_wiring(0, 1, &account_view, &[], 15, RiskMode::Normal, &mut book, &mut witness);

        let long = book.campaign(0, VoiceSide::Long).expect("多头侧 campaign");
        let short = book.campaign(0, VoiceSide::Short).expect("空头侧 campaign");
        assert_eq!(long.tw().notional_in, 3_000, "多头侧 notional_in 不含空头侧的 1000");
        assert_eq!(short.tw().notional_in, 1_000, "空头侧 notional_in 不含多头侧的 3000");
        assert_eq!(long.current_units(), 300);
        assert_eq!(short.current_units(), 50);
        assert_eq!(
            (witness.lifecycle_opened.get("long"), witness.lifecycle_opened.get("short")),
            (Some(&1), Some(&1)),
            "两侧各一次开仓生——★#381 关票修复：分侧计数，不得相加成 2"
        );

        // 同一 bar 两侧各一条镜像动作 ⟹ 各记各账，互不冲抵。
        let actions = vec![
            record_side(0, VoiceSide::Long, CenterOscillationAction::Reduce),
            record_side(0, VoiceSide::Short, CenterOscillationAction::Reduce),
        ];
        drive_campaign_wiring(0, 1, &account_view, &actions, 15, RiskMode::Normal, &mut book, &mut witness);
        assert_eq!(
            book.campaign(0, VoiceSide::Long).unwrap().short_diff().bucket().open_units(),
            100,
            "多头侧挂起=300/3"
        );
        assert_eq!(
            book.campaign(0, VoiceSide::Short).unwrap().short_diff().bucket().open_units(),
            16,
            "空头侧挂起=50/3，与多头侧互不污染"
        );

        // 多头侧全平：只杀多头 campaign，空头侧存活。
        account_view.post(
            AccountOrder {
                key: core_key(0),
                reason: ActionReason::ReverseType1,
                qty_delta: -300.0,
                decision_bar: 1,
            },
            16.0,
            1,
        );
        drive_campaign_wiring(1, 1, &account_view, &[], 16, RiskMode::Normal, &mut book, &mut witness);
        assert!(book.campaign(0, VoiceSide::Long).is_none(), "多头侧全平 ⟹ 多头 campaign 死");
        assert!(book.campaign(0, VoiceSide::Short).is_some(), "空头侧仓位未动 ⟹ 空头 campaign 存活");
        assert_eq!(
            (witness.lifecycle_died.get("long"), witness.lifecycle_died.get("short")),
            (Some(&1), None),
            "恰一次死亡事件且归属多头侧（空头侧存活 ⟹ 该侧无死亡计数）"
        );
    }

    /// ★issue #357 关票条件 C（★#381 后语义扩展）：本级多空并存时，**多头侧** campaign 的取数
    /// 不被空头侧腿的净额/成本基污染（净额求和会让 300−50=250、成本基净额求和会把两侧成本基
    /// 相加，均非「多头侧真实持仓 300 股 @10=3000」）。#381 后空头侧另有自己的 campaign，
    /// 故开仓生事件为两次（分侧独立生命周期见
    /// `drive_campaign_wiring_long_and_short_campaigns_coexist_without_contamination`）。
    #[test]
    fn drive_campaign_wiring_opens_campaign_from_long_side_only_when_mixed_with_short() {
        let mut account_view = strategy::account::ParallelAccountLedger::new();
        let open_long = AccountOrder {
            key: core_key(0),
            reason: ActionReason::Open,
            qty_delta: 300.0,
            decision_bar: 0,
        };
        account_view.post(open_long, 10.0, 0); // 多头 300 股 @10 ⟹ 成本基 3000
        let open_short = AccountOrder {
            key: short_key(0),
            reason: ActionReason::Open,
            qty_delta: -50.0,
            decision_bar: 0,
        };
        account_view.post(open_short, 20.0, 0); // 空头 50 股 @20 ⟹ 成本基 1000（另一实例，不与多头共享）

        let mut book = CampaignBook::new();
        let mut witness = CampaignWiringWitness::new();
        drive_campaign_wiring(0, 1, &account_view, &[], 15, RiskMode::Normal, &mut book, &mut witness);

        let campaign = book.campaign(0, VoiceSide::Long).expect("多头侧持仓非空 ⟹ 应已开局 campaign");
        assert_eq!(campaign.tw().notional_in, 3_000, "notional_in=多头侧成本基（3000），不含空头侧的1000");
        assert_eq!(campaign.tw().holding, 3_000);
        assert_eq!(
            (witness.lifecycle_opened.get("long"), witness.lifecycle_opened.get("short")),
            (Some(&1), Some(&1)),
            "★#381：多头侧 + 空头侧各开局一次（分侧可判读）"
        );
    }
}

#[cfg(test)]
mod center_oscillation_wiring_tests {
    //! #292 T2 落点门控接线单测（★★触发源改码，用户裁定 2026-07-26）：`step_center_oscillation`
    //! 纯函数化后可脱离完整 fill 循环直接单测（不依赖 tower_i/gate/pan_div/真实市场数据）。
    //! 覆盖验收各项：
    //! - 门控开启臂 · 触发通路：次级别（lvl-1）已确认买卖点驱动 `CenterOscillationTrigger`，
    //!   两边界侧（次级别卖点=上沿高抛 `Reduce`、次级别买点=下沿回补 `Replenish`）均可见；
    //!   **无盘背证据**（本测试模块自 #292 触发源改码后已不再构造任何 `PanDivTrigger`）仍可
    //!   触发——盘背非必要条件（B 裁定）的最强证据即本模块编译期已不存在该依赖。
    //! - 门控开启臂 · 终结通路：中枢链推进+确认三类点 ⟹ 生命周期终结动作可见。
    //! - 门控关闭臂：fill.rs 主循环里本函数整段不被调用（`pan_div_hist=None` 分支跳过），
    //!   `FillOutput.center_oscillation_actions` 恒空——由 v1/dual 路径的诚实空 Vec 字面量
    //!   保证（编译期可见，见 `FillOutput` 三处构造点），本测试补运行期证据。
    //! - #292 续修（二轮评审）：链重基（`ChainConsumed::Rebased`）到达时挂起按身份核对新链——
    //!   仍在链上⟹跟随迁移，从新链消失⟹终结，端到端经 `step_center_oscillation` 可见（见
    //!   `gate_on_chain_rebase_migrates_survivor_and_terminates_vanished_suspension`）。
    use super::*;
    use classifier::center_lifecycle::CenterId;
    use classifier::{bsp::BspPoint, bsp::OwnerRef, Classification, LevelState};
    use super::super::super::strategy::center_oscillation_trade::{CenterOscillationAction, CenterOscillationBook};
    use super::super::super::strategy::voice::VoiceSide;
    use super::super::super::types::{BspBits, Center, Tick};
    use std::rc::Rc;

    fn center(start_index: usize, end_index: usize, zd: i64, zg: i64) -> Center {
        Center { zd, zg, dd: zd - 2, gg: zg + 2, start_index, end_index }
    }

    fn level_with_centers(centers: Vec<Center>) -> LevelState {
        LevelState { centers: Rc::new(centers), ..LevelState::default() }
    }

    fn level_with_bsp(points: Vec<BspPoint>) -> LevelState {
        LevelState { bsp: Rc::new(points), ..LevelState::default() }
    }

    fn third_class_buy_break(source_index: usize, owner: Center) -> BspPoint {
        BspPoint {
            source_index,
            level_origin: 0,
            bits: BspBits { buy3: true, ..BspBits::default() },
            pivot_low: 0,
            pivot_high: 0,
            center: Some(OwnerRef::Center(owner)),
            struct_break_dir: None,
            force: None,
        }
    }

    /// 次级别（lvl-1）已确认买卖点：`side=Long` ⟹ buy1 bit（次级别买点，价格落 `pivot_low`），
    /// `side=Short` ⟹ sell1 bit（次级别卖点，价格落 `pivot_high`）——与生产读法
    /// （`step_center_oscillation`：Long→pivot_low/Short→pivot_high）同口径，供 #292 B 裁定
    /// 独立价格判据消费。无载体（次级别买卖点触发不读次级别中枢，A/B 裁定只认本级
    /// `alive_center()` + 次级别信号方向，`center: None` 即证无门读取次级别账户/结构状态）。
    fn sub_level_bsp_point(source_index: usize, side: VoiceSide, price: Tick) -> BspPoint {
        let (bits, pivot_low, pivot_high) = match side {
            VoiceSide::Long => (BspBits { buy1: true, ..BspBits::default() }, price, 0),
            VoiceSide::Short => (BspBits { sell1: true, ..BspBits::default() }, 0, price),
            VoiceSide::Flat => (BspBits::default(), 0, 0),
        };
        BspPoint {
            source_index,
            level_origin: 0,
            bits,
            pivot_low,
            pivot_high,
            center: None,
            struct_break_dir: None,
            force: None,
        }
    }

    /// 两级 Classification：level 0（次级别，仅本 bar bsp）+ level 1（本级中枢在场）。
    fn two_level_step(sub_bsp: Vec<BspPoint>) -> Classification {
        Classification { levels: vec![level_with_bsp(sub_bsp), LevelState::default()] }
    }

    /// 门控开启臂 · 触发通路（B 裁定）：本级（level 1）中枢在场（born 由 consume_chain 推出）
    /// + 次级别（level 0）已确认卖点（`Short`=上沿高抛试探）⟹ `Reduce` 动作可见，身份=本级
    /// `alive_center()`。本测试**不构造任何 `PanDivTrigger`**——`step_center_oscillation` 签名
    /// 已不接受盘背触发参数，编译期即证盘背非必要（B 裁定最强形式）。
    #[test]
    fn gate_on_sub_level_sell_point_produces_visible_reduce_action() {
        let c0 = center(5, 10, 100, 200);
        let classification =
            Classification { levels: vec![LevelState::default(), level_with_centers(vec![c0])] };
        let step = two_level_step(vec![sub_level_bsp_point(11, VoiceSide::Short, 200)]); // 中轴150，200≥中轴=上半区
        let mut cl_machines = Vec::new();
        let mut osc_books = Vec::new();
        let mut witness = super::super::super::strategy::oscillation_campaign::CampaignWiringWitness::new();
        let actions = step_center_oscillation(7, &classification, &step, &mut cl_machines, &mut osc_books, &mut witness);
        assert_eq!(actions.len(), 1, "门开+在场+次级别卖点 ⟹ 恰一条动作可见");
        assert_eq!(actions[0].bar, 7);
        assert_eq!(actions[0].level, 1, "触发落在本级（level 1），非次级别（level 0）");
        assert_eq!(actions[0].center, CenterId::of(&c0), "身份=alive_center()");
        assert_eq!(actions[0].action, CenterOscillationAction::Reduce, "次级别卖点=上沿高抛");
        assert!(osc_books[1].is_suspended(CenterId::of(&c0)));
        assert_eq!(witness.trigger_attempts, 1, "★issue #357：触发构造尝试计入 witness");
        assert_eq!(witness.dropped_center_not_alive, 0, "本级在场 ⟹ 不丢弃");
    }

    /// 门控开启臂 · 触发通路（B 裁定，另一边界侧）：次级别（level 0）已确认买点（`Long`=下沿
    /// 回补试探）——先高抛挂起，再喂次级别买点 ⟹ `Replenish` 动作可见，挂起清空。
    #[test]
    fn gate_on_sub_level_buy_point_produces_visible_replenish_action() {
        let c0 = center(5, 10, 100, 200);
        let classification =
            Classification { levels: vec![LevelState::default(), level_with_centers(vec![c0])] };
        let mut cl_machines = Vec::new();
        let mut osc_books = Vec::new();
        let mut witness = super::super::super::strategy::oscillation_campaign::CampaignWiringWitness::new();
        let empty_step = two_level_step(Vec::new());
        let reduce_step = two_level_step(vec![sub_level_bsp_point(10, VoiceSide::Short, 200)]);
        let _ = step_center_oscillation(0, &classification, &empty_step, &mut cl_machines, &mut osc_books, &mut witness);
        let reduce_actions =
            step_center_oscillation(1, &classification, &reduce_step, &mut cl_machines, &mut osc_books, &mut witness);
        assert_eq!(reduce_actions[0].action, CenterOscillationAction::Reduce);
        assert!(osc_books[1].is_suspended(CenterId::of(&c0)));

        let cover_step = two_level_step(vec![sub_level_bsp_point(20, VoiceSide::Long, 100)]); // 100≤中轴150=下半区
        let actions = step_center_oscillation(2, &classification, &cover_step, &mut cl_machines, &mut osc_books, &mut witness);
        // ★#381：同一次下沿触发对两侧各产一条镜像动作——多头侧收口（`Replenish`，回补买入），
        // 空头侧开局（`Reduce`，回补空头）。侧序固定 Long→Short。
        assert_eq!(actions.len(), 2, "门开+挂起+次级别买点 ⟹ 多头回补 + 空头开局各一条");
        assert_eq!(actions[0].side, VoiceSide::Long);
        assert_eq!(actions[0].action, CenterOscillationAction::Replenish, "次级别买点=多头侧下沿回补");
        assert_eq!(actions[0].center, CenterId::of(&c0));
        assert_eq!(actions[1].side, VoiceSide::Short);
        assert_eq!(actions[1].action, CenterOscillationAction::Reduce, "空头侧镜像：下沿=减（回补空头）");
        assert!(!osc_books[1].is_suspended(CenterId::of(&c0)), "多头侧回补出口=该侧挂起清空");
        assert!(
            osc_books[1].is_suspended_side(VoiceSide::Short, CenterId::of(&c0)),
            "空头侧此刻转为挂起（镜像开局腿）"
        );
    }

    /// 门控开启臂 · 终结通路：先触发挂起，再喂本级三类买点破坏（`buy3`，载体=在场中枢）⟹
    /// 生命周期终结事件产出 `Replenish` 收手回补动作，经 `step_center_oscillation` 可见。
    #[test]
    fn gate_on_lifecycle_broken_event_produces_visible_replenish_action() {
        let c0 = center(5, 10, 100, 200);
        let classification = Classification { levels: vec![level_with_centers(vec![c0])] };
        let mut cl_machines = Vec::new();
        let mut osc_books = Vec::new();
        let mut witness = super::super::super::strategy::oscillation_campaign::CampaignWiringWitness::new();
        // 先驱动一次空 step 让链同步（Adopted）+ 触发一次高抛挂起（不经本函数：直接摆状态）。
        let empty_step = Classification { levels: vec![LevelState::default()] };
        let _ = step_center_oscillation(0, &classification, &empty_step, &mut cl_machines, &mut osc_books, &mut witness);
        osc_books[0].on_trigger(
            super::super::super::strategy::center_oscillation_trade::CenterOscillationTrigger::new(
                0,
                Some(CenterId::of(&c0)),
                VoiceSide::Short,
                200, // 中轴150，200≥中轴=上半区
                5,
            )
            .unwrap(),
        );
        assert!(osc_books[0].is_suspended(CenterId::of(&c0)));

        // 本 bar 喂一枚三类买点破坏（buy3=Long 破坏），载体=在场实例 c0。
        let step_with_bsp =
            Classification { levels: vec![level_with_bsp(vec![third_class_buy_break(20, c0)])] };
        let actions =
            step_center_oscillation(1, &classification, &step_with_bsp, &mut cl_machines, &mut osc_books, &mut witness);
        assert_eq!(actions.len(), 1, "三类买点破坏终结 ⟹ 收手回补动作可见");
        assert_eq!(actions[0].action, CenterOscillationAction::Replenish);
        assert_eq!(actions[0].center, CenterId::of(&c0));
        assert_eq!(
            witness.suspension_by_source.get(&("long", "broken_by_third_class_buy")),
            Some(&1),
            "★issue #357：挂起归宿分桶记录三类买点破坏终结来源"
        );
        assert!(!osc_books[0].is_suspended(CenterId::of(&c0)), "终结=挂起清空");
    }

    /// #292 续修（issue #292 二轮评审：挂起悬空泄漏）端到端接线证据：`step_center_oscillation`
    /// 在遇到 `ChainConsumed::Rebased` 时正确接入 `on_chain_rebase`——重基后仍在新链上的挂起
    /// 身份跟随迁移（原样保留），从新链消失的挂起身份终结（不回补，故本 bar 无 cover_action
    /// 可见动作，只能从挂起表状态验证）。
    #[test]
    fn gate_on_chain_rebase_migrates_survivor_and_terminates_vanished_suspension() {
        let c0 = center(5, 10, 100, 200);
        let c1 = center(20, 25, 300, 400);
        let c2 = center(22, 27, 500, 600);
        let mut cl_machines = Vec::new();
        let mut osc_books = Vec::new();
        let mut witness = super::super::super::strategy::oscillation_campaign::CampaignWiringWitness::new();

        // bar0：链=[c0]，首次消费（Adopted），不产生任何生命周期事件。
        let bar0_classification = Classification { levels: vec![level_with_centers(vec![c0])] };
        let empty_step = Classification { levels: vec![LevelState::default()] };
        let _ = step_center_oscillation(0, &bar0_classification, &empty_step, &mut cl_machines, &mut osc_books, &mut witness);

        // 直接摆两笔挂起（c0 已在场；c1 尚未在链上——挂起按身份匹配，不要求当前在场，D 裁定）。
        osc_books[0].on_trigger(
            super::super::super::strategy::center_oscillation_trade::CenterOscillationTrigger::new(
                0,
                Some(CenterId::of(&c0)),
                VoiceSide::Short,
                200, // c0 中轴150，200≥中轴=上半区
                1,
            )
            .unwrap(),
        );
        osc_books[0].on_trigger(
            super::super::super::strategy::center_oscillation_trade::CenterOscillationTrigger::new(
                0,
                Some(CenterId::of(&c1)),
                VoiceSide::Short,
                400, // c1 中轴350，400≥中轴=上半区
                2,
            )
            .unwrap(),
        );
        assert!(osc_books[0].is_suspended(CenterId::of(&c0)));
        assert!(osc_books[0].is_suspended(CenterId::of(&c1)));

        // bar1：链前缀分叉为 [c1, c2]（已消费的第 0 格身份从 c0 改写为 c1）⟹ Rebased。
        // 新链含 c1、不含 c0。
        let bar1_classification = Classification { levels: vec![level_with_centers(vec![c1, c2])] };
        let actions = step_center_oscillation(1, &bar1_classification, &empty_step, &mut cl_machines, &mut osc_books, &mut witness);
        assert!(actions.is_empty(), "RebaseVanished 终结不回补，本 bar 无 cover_action 可见动作");
        assert!(!osc_books[0].is_suspended(CenterId::of(&c0)), "c0 已从新链消失⟹终结，不再悬空");
        assert!(osc_books[0].is_suspended(CenterId::of(&c1)), "c1 仍在新链上⟹跟随迁移，挂起原样保留");
    }

    /// 门控双轨（#292 项目二）：原测试用空 bars——`bars.is_empty()` 本身已让 `pan_div_hist=None`，
    /// 恒真无判别力（门控是否真生效根本没被触达）。改为非空 bars + 真实触发条件（本级中枢
    /// 在场 + 次级别已确认卖点，价格落中枢上半区）上的 `enabled=false`/`enabled=true` 对照：
    /// 门关时零动作（真实门控生效，非"从未触达"的假阴性）、门开时动作可见（真判别力）。
    #[test]
    fn gate_dual_track_nonempty_bars_off_zero_on_visible() {
        fn mk_bar(idx: usize) -> super::super::super::types::Bar {
            super::super::super::types::Bar {
                source_index: idx,
                timestamp: idx as i64,
                open: 10_000_000_000,
                high: 10_000_000_000,
                low: 10_000_000_000,
                close: 10_000_000_000,
                volume: 1,
                untradable: false,
            }
        }
        let bars: Vec<super::super::super::types::Bar> = (0..3).map(mk_bar).collect();

        let c0 = center(5, 10, 100, 200); // 中轴 = 150
        let sell_point = sub_level_bsp_point(3, VoiceSide::Short, 200); // 200≥中轴=上半区
        let classification = Classification {
            levels: vec![level_with_bsp(vec![sell_point]), level_with_centers(vec![c0])],
        };

        let config_off = super::super::super::config::ThetaConfig::default();
        assert!(!config_off.center_oscillation.enabled, "默认门关（回归锁前提）");
        let cls_off = classification.clone();
        let out_off = pi_theta_fill_loop(
            move |i| {
                let cls = if i == 0 { cls_off.clone() } else { Classification::default() };
                (cls, Vec::new(), Vec::new(), i as u64, i as u64)
            },
            &bars,
            1.0,
            &config_off,
            None,
        );
        assert!(
            out_off.center_oscillation_actions.is_empty(),
            "门关：非空 bars + 真实触发条件下仍零动作（真实门控生效，非从未触达的假阴性）"
        );

        let mut config_on = config_off.clone();
        config_on.center_oscillation.enabled = true;
        let cls_on = classification;
        let out_on = pi_theta_fill_loop(
            move |i| {
                let cls = if i == 0 { cls_on.clone() } else { Classification::default() };
                (cls, Vec::new(), Vec::new(), i as u64, i as u64)
            },
            &bars,
            1.0,
            &config_on,
            None,
        );
        assert_eq!(
            out_on.center_oscillation_actions.len(),
            1,
            "门开：同一非空 bars + 同一触发条件 ⟹ 恰一条可见动作（真判别力）"
        );
        assert_eq!(out_on.center_oscillation_actions[0].action, CenterOscillationAction::Reduce);
    }
}

pub(super) fn pi_theta_fill_loop_overlay<F>(
    mut classify_at: F,
    bars: &[Bar],
    initial_nav: f64,
    config: &ThetaConfig,
    chi: Option<ChiFilterCtx>,
    mut overlay: Option<&mut super::super::strategy::overlay_state::OverlayState>,
    mut voice_exec: Option<&mut super::super::strategy::overlay_state::VoiceExecBook>,
) -> FillOutput
where
    // ★工位 4g/on2w2：闭包返回四元组——第三个 u64 = 塔代次（candidate 段判据）；第四个 u64 =
    // forest_epoch（K_i 森林段 O(1) 命中判据，on2w2 O(n²) 修复）。
    F: FnMut(usize) -> (classifier::Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>, Vec<usize>, u64, u64),
{
    use super::super::strategy::coverage::{self, PiThetaWeights};
    use super::super::strategy::exec::fill_bar_index;
    use super::super::strategy::interp::{self, ActiveLeg};
    use super::super::strategy::overlay_state::{VoiceOrder, VoiceOrderKind};

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
    // #196 阶段 A：shadow 双链比对簿（零行为变更——只读生产状态，channel 适配层在组合层
    // 裁决点之后并行跑，分歧仅入本簿内存 + env 门控落盘；不改裁决与订单流）。
    let mut shadow_book = super::super::strategy::shadow::ShadowVoiceBook::default();
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
    // #82 DC-E：默认关闭时整段惰性（不算 MACD、不评生产门、不上触发轨），订单轨 frozen。
    // 开启时首见证书只经 econ_positive 的 Nest/XZD 单一门产触发事件（#282：账面形态已删，
    // 门内不产订单、不写子腿簿——触发链保留为 #274 原料）。
    let mut pan_div_state = super::pan_div::PanDivProductionState::default();
    // #292（T2 落点门控接线）：中枢生命周期事件机（每级别一台，T1 单一真相源，与
    // opsem_dump::feed_center_lifecycle 同款 consume_chain/push_point 驱动方式，状态独立不
    // 交叉——同一张塔链表的两个只读消费者）+ 中枢震荡挂起账（每级别一台）。惰性按级增长
    // （新级涌现补建）；门关（`config.center_oscillation.enabled=false`）⟹ 本段整体不构造
    // 不驱动，零开销，订单轨逐字节不变（bit-exact 不破）。
    let mut cl_machines: Vec<classifier::center_lifecycle::CenterEventMachine> = Vec::new();
    let mut osc_books: Vec<super::super::strategy::center_oscillation_trade::CenterOscillationBook> =
        Vec::new();
    let mut osc_actions: Vec<super::super::strategy::center_oscillation_trade::CenterOscillationActionRecord> =
        Vec::new();
    // ★issue #357（#294/#278 生产接线最后一公里）：每仓 campaign 账簿（每级别一台，惰性按级
    // 增长，同 osc_books 粒度）+ 生产接线观测统计。门关 ⟹ 本段整体不驱动（下方消费点全在
    // `pan_div_hist` 门内），二者恒 default/空，零开销，订单轨逐字节不变（bit-exact 不破）。
    let mut campaign_book = super::super::strategy::oscillation_campaign::CampaignBook::new();
    let mut campaign_witness = super::super::strategy::oscillation_campaign::CampaignWiringWitness::new();
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
    // ── ★nest-gate（进场准入升格路径 b，nest-gate-elevation-design-20260719 §3 实装卡）：
    //    开仓准入门消费 typed nest 证书（build_gate_certificate 单一裁决源）。门源依赖 MACD
    //    hist——复用 pan_div_hist（已算则零成本），否则门开启时无条件算一次 compute_macd
    //    （同 config.macd，确定性 O(n) 一次性；不构成 bit-exact 风险，只构成成本）。门关闭
    //    ⟹ None ⟹ 门段整体跳过，逐字节不变（bit-exact 回归锁）。──
    let nest_gate_hist: Option<Vec<f64>> = if nest_cert_gate_enabled() && !bars.is_empty() {
        match &pan_div_hist {
            Some(h) => Some(h.clone()),
            None => {
                let closes: Vec<f64> = bars
                    .iter()
                    .map(|bar| bar.close as f64 / config.tick.tick_size)
                    .collect();
                Some(super::super::classifier::divergence::compute_macd(
                    &closes,
                    &config.macd,
                ).hist)
            }
        }
    } else {
        None
    };
    // ── ★#75（N3-T2 真链切换）：门开 ⟹ typed 真链门状态（增量事件账本 + 证书索引 +
    //    provider 域序列，一次性 O(n) 构建）；门关 ⟹ None ⟹ 门段整体跳过，逐字节不变。──
    let mut nest_chain_gate: Option<NestChainGate> = if nest_gate_hist.is_some() {
        Some(NestChainGate::new(bars, config))
    } else {
        None
    };
    // nest-gate 观测统计（门开启时逐候选落账；门关闭恒零、不输出）。
    let mut nest_gate_stats = NestGateStats::default();

    let mut equity_curve = Vec::with_capacity(n);
    let mut trade_pnls: Vec<f64> = Vec::new();
    let mut trades: Vec<metrics::TradeRecord> = Vec::new();
    let mut pos_entry_bar: Option<usize> = None;
    let mut n_orders_executed: usize = 0;
    // ── ★W1 声部独立执行层（churn 修复臂，netting-vs-voice-execution-audit-20260719 §7）：
    //    Some ⟹ 决策层由净额影子账本（上方 cash/units/entry_cost 全部原有逻辑原样跑）驱动——
    //    typed_ledger/TW/sep_legs 与净额臂逐字节一致；真实执行投影走 VoiceExecBook（声部独立
    //    持仓 + 事件驱动 fill + 开仓冻结 sizing），输出层（equity/trade_pnls/trades/n_orders/
    //    r_decomp）换声部账户口径。None ⟹ 本块全部变量闲置、声部分支全跳过（bit-exact 回归锁）。──
    let voice_enabled = voice_exec.is_some();
    let mut pending_voice: Vec<Vec<VoiceOrder>> = if voice_enabled { vec![Vec::new(); n] } else { Vec::new() };
    let mut n_voice_fills: usize = 0;
    let mut voice_trade_pnls: Vec<f64> = Vec::new(); // 声部平仓 fill 费后已实现（不含强平）
    let mut voice_trades: Vec<metrics::TradeRecord> = Vec::new();
    let mut cum_funding_v: f64 = 0.0; // 声部账户持仓成本（cost_model=None ⟹ 恒 0）
    let mut cum_borrow_v: f64 = 0.0;
    let mut cum_liq_v: f64 = 0.0;
    // ── G4 typed ledger（#134）：腿级在飞表（voice_id → 入场登记）+ 已结算 typed 交易。 ──
    let mut open_trades: std::collections::HashMap<
        classifier::recursive_tower::ElementId,
        LedgerOpen,
    > = std::collections::HashMap::new();
    let mut typed_ledger: Vec<TypedTrade> = Vec::new();
    // #201 阶段 B 裁决账本轨（加轨不减轨）：逐 bar 消费 StepTrace.verdicts 的显式
    // per-voice 裁决序列（含显式 Hold）；与 typed_ledger/账户镜像/opsem/TW 全正交。
    let mut verdict_ledger: Vec<VoiceVerdictRec> = Vec::new();
    // #197 并行记账视图（expand 只读旁路）：(account, level) 键全链携带的分实例账本。
    // 镜像开/关腿生命周期事件过账，不改净额路径任何语义（三把 bit-exact 锁为界）；
    // 消费这些类型修复归属现病属修复票 #198/#199/#200。
    let mut account_view = strategy::account::ParallelAccountLedger::new();
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
    // ★B-M3b-expand（#90）：TW 账本线程收敛为 TwLedgerThread（~10 处 tw_step 散点聚出）。
    // κ=0 最小基线（PDF §10 canonical 默认，runner.rs:702 生产口径）。
    // ★A10 附则A（裁定接口冻结）：优先序写死 env `KAPPA_BARRIER_*`（L2 敏感性诊断覆写，codex
    // `.kappa-ruling-20260704`，M7 网格 {0,0.5,1,2}=`0/1,1/2,1/1,2/1`）> `config.risk_policy` >
    // baseline κ=0——**env 未设 ∧ config=None ⟹ baseline κ=0**，所有生产/测试路径逐字节不变
    // （bit-exact）。κ 只在此单点注入，经 stage_progression 门控三阶段推进；env 纯诊断，不改生产
    // 冻结值（正 κ 生产选择是 M8 L3 的事，codex 已裁，A10 附则A 留编排者选择类）。与
    // M7_WITNESS_BARS 同为 L2 诊断 env 惯例。
    let mut tw_thread = TwLedgerThread::new(nav0, kappa_policy_resolved(config.risk_policy));

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
        // ★W1：声部簿价格 PnL 累计（同一 Δpx 口径——簿内持仓是前 bar 收盘持仓，在本 bar
        //    声部 fill 之前累计，与上方净额影子累计同一时点）。
        if voice_enabled {
            voice_exec.as_deref_mut().expect("voice_enabled ⟹ Some").mark_to_market(px);
        }

        // ── ① 延迟成交：本 bar 到达 exec_index 的挂单 fill（apply_order，先平后开）。 ──
        if !pending[i].is_empty() && !bar.untradable && px > 0.0 {
            let orders = std::mem::take(&mut pending[i]);
            for o in &orders {
                if o.qty > 0 {
                    let units_before = units;
                    // A'（清单③）：累加本 fill 的费后已实现 PnL（多 fill 同 bar 聚合——推导链第 9 条）。
                    let fill = apply_order(o, px, fee_rate, &mut cash, &mut units, &mut entry_cost, &mut trade_pnls);
                    tw_thread.add_realized(fill.realized);
                    cum_fee += fill.fee;
                    if fill.executed_qty > 0.0 {
                        // 轨迹与执行计数只消费真实成交；全拒单不再伪造 L2 执行事实。
                        track_position_transition(&mut trades, &mut pos_entry_bar, units_before, units, i, false);
                        n_orders_executed += 1;
                    }
                }
            }
        }
        // ── ★W1 ①-voice 声部 fill（执行投影）：本 bar 到达的声部订单逐声部成交（hedge-mode
        //    簿，每声部独立 units_v/entry_cost_v）；平仓结算产 trade_pnls/TradeRecord 行。──
        if voice_enabled && !pending_voice[i].is_empty() && !bar.untradable && px > 0.0 {
            let vos = std::mem::take(&mut pending_voice[i]);
            let vb = voice_exec.as_deref_mut().expect("voice_enabled ⟹ Some");
            for vo in &vos {
                let out = match vo.kind {
                    VoiceOrderKind::Open => vb.apply_open(vo.voice, vo.side, vo.qty, px, i, fee_rate),
                    VoiceOrderKind::Close => vb.apply_close(vo.voice, px, i, fee_rate),
                };
                if out.executed_qty > 0.0 {
                    n_voice_fills += 1;
                }
                if let Some(cinfo) = out.closed {
                    voice_trade_pnls.push(cinfo.pnl);
                    voice_trades.push(metrics::TradeRecord {
                        entry_bar: cinfo.entry_bar,
                        exit_bar: i,
                        hold_bars: i.saturating_sub(cinfo.entry_bar).max(1),
                        qty: cinfo.qty as f64,
                        long: cinfo.long,
                        forced_close: false,
                    });
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
        // ★W1 ①⁺-voice：声部账户持仓成本镜像（对 N_derived=Σσ_v q_v 名义计费，现金真扣 ⟹
        //    守恒断言覆盖；cost_model=None ⟹ 恒 0 不调用）。触发时点与净额影子同（本 bar 成交后）。
        if voice_enabled {
            if let Some(cost) = config.cost_model.as_ref() {
                let vb = voice_exec.as_deref_mut().expect("voice_enabled ⟹ Some");
                let nv = vb.net_signed() as f64;
                if px > 0.0 && nv != 0.0 {
                    let net_notional_usd = nv.abs() * px;
                    let equity_pre = vb.cash() + nv * px;
                    let funding = cost.funding_accrual(i, net_notional_usd);
                    let borrow = cost.borrow_accrual(net_notional_usd, equity_pre);
                    vb.debit_cash(funding + borrow);
                    cum_funding_v += funding;
                    cum_borrow_v += borrow;
                }
            }
        }

        // ── ② p_t = 净 lot（成交后真实持仓）。 ──
        let p_t = units;
        let current_nav = cash + units * px;
        let equity_nav = if current_nav > 0.0 { current_nav } else { nav0 };

        // ── ②' TW 成本划转（#124）：真实成本基（|units|·均价，空头取绝对额=在险市值）方向差分
        //    ⟹ ShortDiff 划转（free⇄holding，TW 守恒构造子）。shadow 追真实、入账钳制（cash-sound）。──
        tw_thread.sync_basis(units, entry_cost);

        // ── ②'' TW 已实现利润入账（codex GAP3 裁定 A' 清单③）：本 bar 平仓 fill 的费后 PnL
        //    聚合（推导链第 9 条：同 bar 多 fill 聚合）经 `TwEvent::Realize(d_pi)` 入 free。
        //    ★硬边界2：置于 ②' 成本基 ShortDiff **之后**、③ TwStepCtx（stage 判据）之前——
        //    同一成交先成本基后利润。★量化口径：对累计值取整再派差分（tw_seen_basis 同款
        //    shadow 模式）⟹ 截断误差有界不累积，TW 漂移恒 = ⌊realized_cum⌋（清单⑥不变量）。
        //    ★可正可负（推导链第 5 条）：亏损如实入账——负 d_pi 使 free 下降；若累计亏损超
        //    free（做空爆亏），free 为负是真实现金透支的诚实镜像（f64 账本 cash 同步为负），
        //    非静默账目错误——stage 门（free≥recover_target>0）在负 free 下恒不过 = 安全侧。
        tw_thread.sync_realized();

        if !bar.untradable && px > 0.0 {
            // ── ③ [A] 前缀因果重分类（classify_at(i)=classify_with_tower(l0[0..=i]) → 因果塔 + 因果
            //      分类，只用 ≤i 数据 → 因果）+ 切当步候选 + [B] base_units U_ℓ + [C] thread + 风控门。 ──
            let (classification_i, tower_i, confirmed_lens, tower_gen, forest_epoch) = classify_at(i);
            // ★opsem-dump：diff tower_i vs prev_tower → 写塔事件（仅交易活跃区间，env-gated）。
            if let Some(dump) = opsem.as_mut() {
                dump.diff_tower(i, &tower_i);
            }
            // ★当步候选 = 前缀因果塔里**本 bar 新确认**的买卖点（append-only diff vs seen，确认-bar
            // 部署）——非 source_index==i 切片（买卖点回溯确认，其触发点常在更晚 bar 才入前缀塔 ⟹
            // source_index==i 切恒空 ⟹ 零订单）。买卖点在被确认那根 bar（source_index≤i）部署=因果。
            let classification_step = newly_confirmed_step(&classification_i, &mut seen_bsps);
            // ★#291（SPEC #274 T1）：中枢生命周期事件机只读旁路（born/broken/reset →
            // center_lifecycle.jsonl）。opsem env 未设 ⟹ None ⟹ 零开销，生产路径 bit-exact 不变；
            // 事件只外化落盘，不回馈任何决策（票面边界：结构地基，动作 = #292）。
            if let Some(dump) = opsem.as_mut() {
                dump.feed_center_lifecycle(i, &classification_i, &classification_step);
            }
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
                // ★W1 ③⁻-voice：声部账户强平罚金镜像（触发边沿与净额影子同一 liq_active 去抖；
                //    罚金名义 = |N_derived|·px——声部账户自身真实持仓，非影子 p_t）。
                if voice_enabled {
                    if let Some(cost) = config.cost_model.as_ref() {
                        let vb = voice_exec.as_deref_mut().expect("voice_enabled ⟹ Some");
                        let nv = vb.net_signed() as f64;
                        if in_liq && !liq_active && nv != 0.0 && px > 0.0 {
                            let penalty = cost.liquidation_penalty(nv.abs() * px);
                            vb.debit_cash(penalty);
                            cum_liq_v += penalty;
                        }
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
                t_stage: Some(tw_thread.tw.stage),
                eta_bucket: Some(tw_thread.policy.eta_bucket_eta_corrected(&tw_thread.tw, cum_holding_cost)),
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
            // ── ★nest-gate（升格路径 b，实装卡 §3.2；#75 真链切换）：Γ_t^trade → Γ_t^cert={γ:持
            //    typed N^δ/Xzd 证书}。#75 起 **typed 真链为唯一 nest 判定源**（N^δ 跨级递归链
            //    判定，nest.rs n_delta 递归核；增量喂法 + 索引消费见 NestChainGate 文档）；typed
            //    无证时回退旧臂 Xzd 通道（Xzd 无 typed 对应物，经 build_xzd_fallback 单一来源）。
            //    L2 旧臂 build_gate_certificate 双读落账供对照差（进 NEST_GATE_CHAIN cross
            //    计数；复用通道 admit≡old_admit by construction 自证，单独计 reuse 不入对照差）。
            //    证书 None 或谓词 false ⟹ 剔除 ⟹ interpret 不归 open ⟹ 不开仓（与 χ 门
            //    同一语义层，:1556-1559 注释同款「gamma 滤掉 ⟹ 不开仓」）。**拒绝不经 μ 桶键**：
            //    ext_i/entry_z 三维不动（R5-1 铁律不触碰）。nest_gate_hist=None（门关闭）⟹ 直通，
            //    逐字节不变。──
            let step_gamma_trade = match &nest_gate_hist {
                Some(hist) => {
                    let gate = nest_chain_gate
                        .as_mut()
                        .expect("THETA_NEST_CERT_GATE=1 ⟹ 真链门状态已构建");
                    gate.sync_events(&tower_i, &confirmed_lens, i);
                    // 索引惰性重建（精确，决策逐字节不变）：T4 (#173) 起唯一前置 =
                    // `chain_key_hint`（链键域 `by_triple_anchor` 保守超集——脚可解析 ∧
                    // 任一链事件级键有身份集）。T5a (#207) 去方向位后 hint 不再携方向
                    //（Flat 候选不进链求值 ⟹ 恒 false，前置精确性逐字保留；等价性论证
                    // 见 `chain_key_hint` 注释：旧 hint=true 候选逐点保持 true，多出的
                    // 触发 = 方向退役解放人口，`index_builds` 差异按足迹列账）。索引唯一
                    // 读者 = `chain_lookup`（仅经 `by_triple_anchor` 键取身份后 `index.get`）：
                    // hint=false ⟹ 链读出必不触索引（脚不可解 ⟹ NoChain；键全空 ⟹ 逐级
                    // 键域查无）⟹ 无需重建；hint=true ⟹ 本前置触发重建 ⟹ 查询点索引内容
                    // = 当前（重建是 (classification, events_by_level) 的确定函数 ⟹ 与基线
                    // 逐字节一致；诊断计数 index_builds 与末次重建时点变化属预期足迹，
                    // 不进任何判定）。事件账本仍逐 bar 维护首次观察纪律。
                    let needs_index = step_gamma_trade.iter().any(|c| {
                        use super::super::strategy::voice::VoiceSide;
                        if matches!(c.dir, VoiceSide::Flat) {
                            return false; // Flat：不进链求值（admit_inner 早退）⟹ 前置恒 false
                        }
                        classification_i.levels.get(c.level as usize).is_some()
                            && gate.chain_key_hint(c, &classification_i)
                    });
                    if needs_index {
                        gate.sync_index(&classification_i);
                    }
                    step_gamma_trade
                        .into_iter()
                        .filter(|c| {
                            let (admit, channel, obs) =
                                gate.admit(&tower_i, c, hist, i, &classification_i);
                            // T5a (#207) shadow dump（#[cfg(test)]，env 未设 = no-op）：
                            // 逐候选落新链结果（三态+谱系+缺断）+ admit/channel。
                            #[cfg(test)]
                            super::admission::t5a_chain_dump::record(i, c, &obs, admit, channel);
                            nest_gate_stats.observe(admit, channel, obs);
                            admit
                        })
                        .collect()
                }
                None => step_gamma_trade, // 门关闭 ⟹ 逐字节不变（bit-exact 回归锁）
            };
            // ── #196 shadow 输入适配：parent_projections 生产构造（现成单源
            //    `interp::parent_certificate_projection` 逐候选构造；cand_elems = step_work
            //    candidate 段只读切片、tree = step_tree，与生产同一份数据；χ 后口径——
            //    step_gamma_trade 之外的候选不产投影）。──
            let step_parent_projections: Vec<super::super::strategy::interp::ParentCertificateProjection> =
                step_gamma_trade
                    .iter()
                    .filter_map(|c| {
                        interp::parent_certificate_projection(c, step_work.overlay(), &step_tree)
                    })
                    .collect();
            // ── #124 裁定4 TW 谓词 ctx（P2/P3/P4 进 fold）：在飞腿 entry_v 映射从 typed ledger
            //    在飞表取（entry_v 入场固定，与 TW open_legacy_legs 计数同源）。#145 T1：由原
            //    ShortDiff id 半镜像升级为全量 entry_v 映射——P2 过滤在组合层按
            //    `== ReverseOpen` 判（语义 bit-exact），且兼作反向关闭 typed 裁决的
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
                state: &tw_thread.tw,
                policy: &tw_thread.policy,
                risk_mode: tw_risk_mode,
                entry_v: &entry_v_map,
                // A10 C5（裁定 (b)）：enter_ready 的 η 左操作数同源修正（与上方 η_bucket 同一
                // cum_holding_cost 变量，F4）；0 ⟹ 历史判据 bit-exact（cost_model=None 回归锁）。
                eta_correction: cum_holding_cost,
            };
            // #82 DC-E（#282 收缩）：门内段 = 触发链——首见 PanDivCert 经 Nest/XZD 单一门
            // 产触发事件上协议轨（#274 原料）。父腿快照/sync、候选路由、P10 Record、
            // 账面 apply 与净额叠加出口全部随 S6 账面形态删除。
            let mut protocol_events = super::super::strategy::protocol::ProtocolEventSet::hold(0);
            // #292（B 裁定，用户 2026-07-26）：PanDivTrigger 降格为可选辅助——不再驱动开启臂
            // 生产触发（驱动源已改为次级别买卖点，见下方 `step_center_oscillation` 调用）。本段
            // 仍保留盘背证据上协议轨（#274 原料，与 #292 触发源解耦，不喂 `step_center_oscillation`）。
            if let Some(hist) = pan_div_hist.as_deref() {
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
                        // DA-Q2：PanDiv 触发证据留在协议轨（#274 消费点；本层不产订单）。
                        let trigger = pan_div_state.prepare(gated);
                        protocol_events = protocol_events.with_center_oscillation(trigger);
                    }
                }
                // #292（T2 接线点二，触发源改码）：级数对齐 + 逐级驱动本 bar 中枢生命周期事件
                // （born/broken/reset/superseded）+ 次级别买卖点 → `CenterOscillationTrigger` 构造
                // 消费（A 裁定：中枢身份取本级 `alive_center()`；B 裁定：触发源=次级别买卖点，
                // 不依赖上面的 pan_div_hist/gate）→ 挂起账减补动作（接线可见性证据）。
                let new_osc_actions = step_center_oscillation(
                    i,
                    &classification_i,
                    &classification_step,
                    &mut cl_machines,
                    &mut osc_books,
                    &mut campaign_witness,
                );
                // ★issue #357（T4/#294 生产实例化，编排者裁定 A：本仓=`Core{level}` 身份账户）：
                // 每仓 campaign 生死驱动 + 减/补动作落成对偶事件（ShortDiff 成本基 + Realize
                // 盈亏）经 closed_loop 带门通道（OQ-9 + CashUnsound）入 TW——独立抽为
                // `drive_campaign_wiring`（可脱离完整 fill 循环直接单测，同 `step_center_oscillation`
                // 先例）。
                //
                // 取数时点（可复检）：`account_view` 反映的是**本 bar 尚未过账前**（前 bar 收盘）
                // 的 Core{level} 持仓——本段落在本 bar 主策略核心仓开/平仓镜像（下方「关闭消费
                // 循环」/「开腿：准入信号腿登记」段）之前执行，与 `step_center_oscillation`
                // 本身的调用时点一致（次级别信号驱动的震荡短差决策，读的是「进入本 bar 时」的
                // 本仓持仓，非本 bar 内主策略自身信号造成的仓位变化——两者是同 bar 内正交的
                // 两条独立决策源，不应互相以未定义的执行顺序静默耦合）。
                drive_campaign_wiring(
                    i,
                    classification_i.levels.len() as u32,
                    &account_view,
                    &new_osc_actions,
                    px as i64,
                    tw_risk_mode,
                    &mut campaign_book,
                    &mut campaign_witness,
                );
                osc_actions.extend(new_osc_actions);
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
            // ── #196 阶段 A shadow：组合层裁决点之后并行跑 channel 适配层——只记录分歧，
            //    不改裁决与订单流（零行为变更；分歧报告 env 门控落盘，见主循环后）。
            //    #201：生产事实由 step_trace.verdicts 显式裁决序列单源推导（trace/裁决层统一）。──
            shadow_book.observe_and_compare(
                i,
                &prev_active,
                &step_gamma_trade,
                &entry_v_map,
                gate.force_flat,
                &step_parent_projections,
                &step_trace,
            );
            // #282（#280 裁定）：门内执行段（select_for_bar 优先级、KΘ 容量/P9 经济减仓
            // 门、pan_div_state.apply 账面写入、signed_live_child_units 净额叠加出口）已随
            // S6 开空腿账面形态删除——门内只余触发链（上方 protocol_events 上轨），
            // 订单轨在门开启臂也不再被子腿叠加改写。
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
            // ★#200 次序（先平后开，`A_raw=(A_t∖D_t)∪O_t` 买卖点2 p.6/14 §9）：**关闭消费
            // 循环先于开腿登记循环**——OpenShort 通道的二类「先平后开」在同 bar 同 carrier
            // 上平旧 campaign + 开新 campaign（位置节点身份 = carrier id，A9 generation
            // 单调 +1）；若先 insert 后 remove，新 entry 覆盖旧 entry、关闭循环误删新
            // campaign（typed/账户双轨同错）。故开腿登记移至四个关闭循环之后（下方
            // 「开腿：准入信号腿登记」段）。
            // 反向关闭：typed 裁决消费 trace 第三分量（#145 T1——组合层决策点已经
            // reverse_exit_type 单源判定，本处不补算；登记腿必在 entry_v 映射 ⟹ 裁决非回退值）。
            // ★#209 断言②批次收集：本步一类 × Core 镜像过的 level（批末硬门检查集，
            // 见本循环结束点「断言②终态硬门」段）。★#237：检查集扩为 (level, 批次方向)
            // ——批次方向由被关腿方向单源表达（Exit_v δ(γ)=−σ_v：卖批关 Long、买批关
            // Short），与断言① l.dir 同源同口径。
            let mut t1_core_levels: Vec<(u32, strategy::voice::VoiceSide)> = Vec::new();
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
                    if open.entry_v == super::super::strategy::coverage::Vertical::ReverseOpen {
                        tw_thread.close_share_leg(); // TW 腿计数（#124）
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
                    // #197/#199：关闭镜像——账户=entry_v×方向（与理由正交）；理由按账户分流
                    // （#199「仅残余才纠错」：核心腿二类 ⟹ 核心残余实测非零（并行视图
                    // balance(Core{level})≠0）才发 CoreResidualCorrection，否则二类不生
                    // 本仓卖单（None 不镜像）；ReverseOpen/Short 腿二类 ⟹ ReverseType2
                    // 合法二类卖；一/三类委托 reason_of_reverse 单源）。
                    // 触发类 1/2/3 外不镜像，与 open 侧 None 口径对称，不伪造理由。
                    let account_id = strategy::account::identity_of(
                        open.entry_v,
                        open.position_node_id.side,
                        leg.id.level,
                    );
                    let core_residual =
                        account_id.is_some_and(|a| account_view.has_residual(a));
                    if let Some(reason) = account_id.and_then(|a| {
                        strategy::account::reason_of_reverse_close(a, trig.bsp_class, core_residual)
                    }) {
                        // ★#209：一类 × Core 镜像 ⟹ 记录 level（批末断言②硬门检查集）。
                        // ★#237：连方向一并记录——批次方向=被关腿方向（与断言① l.dir 同源，
                        // 卖批关 Long 查多侧、买批关 Short 查空侧）。
                        if reason == strategy::account::ActionReason::ReverseType1
                            && matches!(
                                account_id,
                                Some(strategy::account::AccountIdentity::Core { .. })
                            )
                        {
                            t1_core_levels.push((leg.id.level, leg.dir));
                        }
                        account_mirror_close(&mut account_view, &open, leg.id.level, reason, px, i);
                    }
                }
                // 表中无登记（本窗开跑前已持/restore 祖先腿）⟹ 非本窗信号入场，不入 ledger。
            }
            // ★#209 断言②终态硬门（批次边界，用户裁 A 2026-07-23：S7 级别内全平是必须
            // 非应当）：一类批末（该级全部核心腿平仓完）balance(Core{level})==0。检查点 =
            // 反向关闭循环结束、其余关闭/开腿循环之前——一类全平批的多条 AccountOrder
            // 已逐腿 post（每腿恰一，reason=ReverseType1 经 reason_of_reverse_close 单源），
            // 同 bar 开腿镜像尚未发生（#200 次序），此时该级本仓必清零。多腿场景逐笔
            // post 后余额仍含未平同级腿，故硬门不挂单笔粒度（见 account_mirror_post 断言②
            // 注释）。形态 = debug 构建 panic + 违例探针恒在计数（release 可观测），与
            // 断言③同款。历史对照（勿删）：#199 测量态 BTC train 窗实测违例=1。
            // ★#237 按方向拆查（与断言①同源同口径，用户裁 2026-07-24）：一类卖批 ⇒ 查
            // 该级**多侧**分量=0（`balance_side(Core{lv}, Long)`）；一类买批 ⇒ 查**空侧**
            // 分量=0——分侧账禁净额（P_sep：净额 balance 同时掩盖双侧残余，ker N 不可
            // 识别）。顺父级联 Short 腿归空侧（实例键 position.side，#185 映射不动），
            // 卖批下合法存活不计入（#234 Q4.3；m3 (1,470) 误报面的账侧同口径收口）。
            t1_core_levels.sort_unstable_by(|a, b| (a.0, a.1 as u8).cmp(&(b.0, b.1 as u8)));
            t1_core_levels.dedup();
            for (lv, side) in t1_core_levels {
                let bal = account_view
                    .balance_side(strategy::account::AccountIdentity::Core { level: lv }, side);
                if bal != 0.0 {
                    t1_core_residual_probe_bump();
                }
                debug_assert!(
                    bal == 0.0,
                    "#237 断言②终态违例：一类全平批末 balance(Core{{{lv}}} {side:?}侧) = {bal} ≠ 0——S7 级别内全平是必须（按方向拆查）"
                );
            }
            // 静默离场（§13 AncOK 连带剪/Stale prune，无触发信号）：归属判据抽为
            // [`silent_drop_exit_type`]（#198：仅首开反向腿记 CloseReverseOpen，其余归 core structural
            // exit；账户=entry_v×方向经 #197 account 层映射，剪枝理由=via_structural_prune 轴）。
            for leg in &step_trace.silent_drops {
                if let Some(open) = open_trades.remove(&leg.id) {
                    use super::super::strategy::coverage::Vertical;
                    if open.entry_v == Vertical::ReverseOpen {
                        tw_thread.close_share_leg(); // TW 腿计数（#124）
                    }
                    let exit_type = silent_drop_exit_type(open.entry_v);
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
                    // #197：关闭镜像——§13 结构剪枝（account=entry_v×方向，reason=StructuralPrune，
                    // 正交拆分即 #185 修复 a 的视图层前身：FollowParent 归 Core 账 + 剪枝理由）。
                    account_mirror_close(
                        &mut account_view,
                        &open,
                        leg.id.level,
                        strategy::account::ActionReason::StructuralPrune,
                        px,
                        i,
                    );
                }
            }
            // 强平清空（#124 P1，PDF §7 C_1 屏蔽 P2..P10）：force_flat ⟹ prev_active 全部 RiskExit
            // （无触发候选；pi_theta_step_traced 上游短路清空 next_active，见 StepTrace.risk_exits）。
            for leg in &step_trace.risk_exits {
                if let Some(open) = open_trades.remove(&leg.id) {
                    if open.entry_v == super::super::strategy::coverage::Vertical::ReverseOpen {
                        tw_thread.close_share_leg();
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
                    // #197：关闭镜像——P1 风险强平逐腿展开为单账户 AccountOrder
                    // （#185 断言5「风险全平展开为多条单账户」的视图层天然形态）。
                    account_mirror_close(
                        &mut account_view,
                        &open,
                        leg.id.level,
                        strategy::account::ActionReason::RiskExit,
                        px,
                        i,
                    );
                }
            }
            // P2 CloseOverlay（#124 裁定4，PDF §7 C_2）：TW StageII 重叠腿关闭——真实订单已经
            // 同一 schedule/fill（组合层合成 close 桶复用 𝒟_x 通道）；typed 归 CloseReverseOpen
            // （关的正是 legacy ReverseOpen 重叠腿，PDF §9 五枚举内最近语义）。
            for leg in &step_trace.overlay_closes {
                if let Some(open) = open_trades.remove(&leg.id) {
                    if open.entry_v == super::super::strategy::coverage::Vertical::ReverseOpen {
                        tw_thread.close_share_leg();
                    }
                    let pushed = TypedTrade {
                        entry_z: open.entry_z,
                        voice_id: leg.id,
                        entry_bar: open.entry_bar,
                        exit_bar: i,
                        exit_type: super::super::strategy::interp::ExitType::CloseReverseOpen,
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
                    // #197：关闭镜像——TW StageII 重叠腿关闭（reason=OverlayClose）。
                    account_mirror_close(
                        &mut account_view,
                        &open,
                        leg.id.level,
                        strategy::account::ActionReason::OverlayClose,
                        px,
                        i,
                    );
                }
            }
            // ── #201 阶段 B 裁决账本轨（加轨不减轨）：消费 StepTrace.verdicts 显式 per-voice
            //    裁决序列（含显式 Hold）——每 bar 每解释器裁决域持仓声部恰一枚，schema 冻结
            //    （阶段 C 换内核在本契约上对齐）。只入本轨；不改 typed_ledger/account_view/
            //    opsem/TW 任何既有轨（Hold 无成交，不入 TypedTrade）。──
            for v in &step_trace.verdicts {
                verdict_ledger.push(VoiceVerdictRec { bar: i, leg: v.leg, exit_type: v.exit });
            }
            // 开腿：准入信号腿登记（z 塔真值，与生产 χ 查询同经 z_of_candidate——训练/查询同口径）。
            // A6（#159）：z_of_candidate 内读 c.force（Candidate 透传 BspPoint.force）填 force_state
            // 第 8 维——entry_z（训练）与上方 filter_gamma（查询）同函数同候选 ⟹ 同口径自动成立，
            // fullz 置换 records 的 force_state 自此携真值（一类 A/C 对候选 Some）。
            // ★#200：本循环位于四个关闭消费循环**之后**（先平后开，次序依据见 ③'' 段首注释）。
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
                // ★B1（步骤4，codex review conditional 修复）：入场 sizing target 快照（开腿当步
                // SepLeg.q_units，含 dir_weight）。sep_legs 经 coverage.rs:2342-2353 filter_map(work.get) 构造 ⟹
                // opened 腿 id 通常在其中，但 filter_map 可跳过 work 不含的 e_idx，理论非 100% 保证。
                // debug_assert 抓测试期不变量违例；release 防御性 0.0（μ 不读 units ⟹ 不破坏 μ；
                // execution 诊断见 0.0 = sizing 信息缺失信号，**非真实 sizing=0**——
                // opsem 的 b1_sizing_available 字段把两种情况在 dump 层分开）。
                let b1_sep = step_trace.sep_legs.iter().find(|s| s.id == leg.id);
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
                    // ★L2-P5/P6：真嵌套深度（根=0，沿 parent_id 链；coverage.rs:1559-1567
                    // `element_depth` 同口径——铁律：来自 parent 链，非级别差）。
                    // 链源 = 本步 sep_legs 的 id→parent_id 映射（A_{t+1} 全集，含 registry
                    // restore 注入的祖先）；新开腿自身缺席 sep_legs（D-V-2 情形）时
                    // 仍可从 leg.parent_id 起跳计数，祖先在集合内即得真深度。
                    let voice_tree_depth = {
                        let sep_parent: std::collections::HashMap<_, _> = step_trace
                            .sep_legs
                            .iter()
                            .map(|s| (s.id, s.parent_id))
                            .collect();
                        let mut d: u8 = 0;
                        let mut cur = leg.parent_id;
                        while let Some(pid) = cur {
                            d = d.saturating_add(1);
                            if d >= 16 {
                                break; // 防腐化死循环（常态 depth ≤ max_depth=3）
                            }
                            cur = sep_parent.get(&pid).copied().flatten();
                        }
                        d
                    };
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
                        // ★L1-P1：χ 门是否参与本步过滤（π overlay 生产路径 chi=None ⟹ false）。
                        chi_filter_active: chi.is_some(),
                        // ★L2-P4：B1 sizing 是否成功从 sep_legs 取得（false ⟹ units=0.0 是兜底）。
                        b1_sizing_available: b1_sep.is_some(),
                        voice_tree_depth,
                        // ★L2-P6：该深度实际消费的 w_depth（[0.60,0.30,0.10] 按深度索引，越界 0.0）。
                        w_depth_at_entry: super::super::strategy::voice::depth_weight(
                            voice_tree_depth as u32,
                            &config.voice,
                        ),
                        prev_active_count: prev_active.len(),
                        // R5-a：本步 LexArgmin top-3（step 级，opened 内共享；traced 正常路径填充）。
                        lex_top3: step_trace.lex_top3.clone(),
                        parent_id: pid,
                        t_stage: t_stage_str(tw_thread.tw.stage),
                        eta_bucket: ext_i.eta_bucket.map(eta_bucket_str).unwrap_or("null"),
                        risk_mode: ext_i.risk_mode.map(risk_mode_str).unwrap_or("null"),
                    }
                } else {
                    OpsemEntrySnapshot::default()
                };
                debug_assert!(
                    b1_sep.is_some(),
                    "B1: opened leg {:?} not in step_trace.sep_legs (coverage work.get 跳过？)",
                    leg.id
                );
                let leg_units = b1_sep.map(|s| s.q_units).unwrap_or(0.0);
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
                    units: leg_units,
                });
                // #197 并行记账视图：开腿镜像（只读旁路，不改净额路径；账户身份 = entry_v × 方向）。
                account_mirror_open(&mut account_view, c, leg.id.level, position_node_id, leg_units, px, i);
            }
            // TW 腿事件（#124）：legacy ReverseOpen 腿开仓驱动 open_legacy_legs 计数（P2 的 H
            // 判据与生产腿同源同步；关侧在上方四个消费循环内经 open.entry_v 判定派
            // CloseShareLeg）。CloseShareLeg(0) 口径声明：净额架构无腿级损益分账 ⟹ profit
            // 口径量 0 承载（cum_net_cash 非承重分量——P2/P3/P4 谓词不消费它；唯一承重 =
            // open_legacy_legs 计数），非簿记伪造。
            // OQ-9 守卫：EarningShares 阶段开 legacy 腿 PDF 定义为非法（is_legal_from）——
            // A' 后该 stage 生产可达（已实现利润入账，见 TW 初始化注释）；达 earning 后此腿
            // 不计 legacy 计数，关侧 legs>=1 守卫对称跳过（合法性语义，非掩盖）。
            for (c, _leg) in &step_trace.opened {
                if c.role.v == super::super::strategy::coverage::Vertical::ReverseOpen {
                    tw_thread.open_share_leg();
                }
            }
            // P3/P4 TWEvent_t（#124 裁定4）：账本推进单点（组合层只读产出事件分量，此处是
            // 生产 π 内唯一的 stage 推进写点——stage_progression 派生事件生产恒合法）。
            if let Some(ev) = step_trace.tw_event {
                tw_thread.on_stage_event(ev);
            }
            // ── ④ 挂单到 exec_index（延迟成交；qty>0 才挂）。 ──
            if order.qty > 0 {
                if let Some(ei) = exec_index {
                    if ei < n {
                        pending[ei].push(order);
                    }
                }
            }
            // ── ★W1 ④-voice 声部挂单（事件驱动，churn 修复）：决策单源 = 本步 step_trace 五类
            //    生命周期事件（与 typed_ledger 消费同源，禁第二裁决源）；无事件 bar 零订单。
            //    sizing 冻结：q_v 取开仓 bar 决策层 SepLeg.q_units（base_units×w_depth×w_dir 已在
            //    coverage 算好，单源复用）取整为手数，落簿后存续期不再随 NAV/价每 bar 重定
            //    （治 runner base_units=equity_nav/px 每 bar 重算导致的声部目标漂移，审计 §4/§5）。
            //    平单先挂、开单后挂：同 bar close→reopen 同 carrier 时 fill 序 = 先平后开
            //    （与 apply_fill 段序同义）；跨 bar 天然按挂单方面时序排列。──
            if voice_enabled {
                if let Some(ei) = exec_index {
                    if ei < n {
                        let lot = config.risk.default_lot.max(1) as i64;
                        for leg in step_trace
                            .closed
                            .iter()
                            .map(|(l, _, _)| l) // 三方合并适配：merged StepTrace.closed 三元化（#145 T1），W1 声部挂单只读腿
                            .chain(step_trace.silent_drops.iter())
                            .chain(step_trace.risk_exits.iter())
                            .chain(step_trace.overlay_closes.iter())
                        {
                            pending_voice[ei].push(VoiceOrder {
                                voice: leg.id,
                                side: leg.dir,
                                qty: 0, // Close = fill 时刻 flatten 簿内该声部全部持仓（执行真值）
                                kind: VoiceOrderKind::Close,
                                exec_index: ei,
                            });
                        }
                        for (c, leg) in &step_trace.opened {
                            // B1 同源快照（typed ledger units 字段同值）：opened 腿在 sep_legs 缺席
                            // （coverage work.get filter_map 跳过）⟹ 0.0 ⟹ q<lot 不开（诚实退化，
                            // 与 B1 release 防御同口径——不伪造 sizing）。
                            let q_units = step_trace
                                .sep_legs
                                .iter()
                                .find(|s| s.id == leg.id)
                                .map(|s| s.q_units)
                                .unwrap_or(0.0);
                            let q = (q_units / lot as f64).round() as i64 * lot;
                            if q >= lot {
                                pending_voice[ei].push(VoiceOrder {
                                    voice: leg.id,
                                    side: c.dir,
                                    qty: q, // ★冻结：开仓时刻快照，存续期不重定
                                    kind: VoiceOrderKind::Open,
                                    exec_index: ei,
                                });
                            }
                        }
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
        // ★W1：声部臂输出声部账户权益（cash_v + N_derived·px；设计性改变——费用路径不同）；
        //    净额影子权益不进输出（决策层真值源，非本臂账户）。
        let eq_mtm = if voice_enabled {
            let vb = voice_exec.as_deref().expect("voice_enabled ⟹ Some");
            vb.cash() + vb.net_signed() as f64 * px
        } else {
            cash + units * px
        };
        equity_curve.push(eq_mtm / nav0);
        // M6：记录本 bar 有效价供下 bar 价格 PnL 差分（px>0 才更新——untradable/零价 bar 不刷，
        // 避免 Δpx 跨越无效价产生伪价格贡献）。
        if px > 0.0 {
            prev_px = Some(px);
        }
    }

    // ── #196 shadow 分歧报告：门控落盘（默认 None 零 IO，bit-exact 原路径；sidecar 文本，
    //    不进 opsem dump、不进订单轨——run 结束一次性写出）。失败显式 eprintln 而非静默
    //    `.ok()?`（OpsemDump 先例）：分歧报告是本票交付物，落盘失败必须可见——但不中断
    //    回测（shadow 是观测旁路，报告可重跑再生）。──
    if let Some(path) = shadow_divergence_path() {
        if let Err(e) = std::fs::write(&path, shadow_book.render_report()) {
            eprintln!("#196 shadow 分歧报告落盘失败（{path:?}）：{e}");
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
    let mut trade_pnls_with_forced = if voice_enabled { voice_trade_pnls.clone() } else { trade_pnls.clone() };
    // ★W1：声部臂 final equity 的真值 = cash_v + N_derived·final_px——在下方虚拟兑现（清空
    //    positions）之前捕获 N_derived（cash_v 不被虚拟兑现改动，r_decomp 装配处直读）。
    let voice_end_net: i64 = if voice_enabled {
        voice_exec.as_deref().expect("voice_enabled ⟹ Some").net_signed()
    } else {
        0
    };
    if voice_enabled {
        // 声部臂：逐声部强平（虚拟兑现，与净额臂 forced_pnl 同口径——不改现金、不计 fill）；
        // forced pnl 行 + TradeRecord（forced_close=true）按声部产（条数 = 在飞声部数）。
        if let Some(last_bar) = bars.last() {
            let last_px = last_bar.close as f64 * config.tick.tick_size;
            if last_px > 0.0 {
                let exit_bar = n.saturating_sub(1);
                let vb = voice_exec.as_deref_mut().expect("voice_enabled ⟹ Some");
                for cinfo in vb.settle_forced_virtual(last_px, exit_bar, fee_rate) {
                    trade_pnls_with_forced.push(cinfo.pnl);
                    voice_trades.push(metrics::TradeRecord {
                        entry_bar: cinfo.entry_bar,
                        exit_bar,
                        hold_bars: exit_bar.saturating_sub(cinfo.entry_bar).max(1),
                        qty: cinfo.qty as f64,
                        long: cinfo.long,
                        forced_close: true,
                    });
                }
            }
        }
    } else if units != 0.0 {
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
            // #197：关闭镜像——窗口终点 censored（reason=WindowEnd；末可交易 bar 兑现价）。
            // 口径声明：视图按腿级账本兑现归零；净额路径窗口终点仅追加 PnL、未真实成交归零
            // （#185 存疑区 5）——两口径分叉为 expand 阶段已知差异，修复票消费时须正视。
            account_mirror_close(
                &mut account_view,
                &open,
                id.level,
                strategy::account::ActionReason::WindowEnd,
                last_px,
                last_i,
            );
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
    // ★W1：声部臂 R 分解换声部账户口径（价格 PnL/费/持仓成本/ledger_delta 全部来自声部簿
    //    与其镜像累计器；守恒断言同一公式同样成立——审计 §7.4 要求）。tw_holding_cost_bridge
    //    行换声部账户自身持仓成本量化（声部账户的 TW 桥对账行；净额影子 TW 账本不受影响）。
    let (r_price, r_fee, r_fund, r_borr, r_liq, r_ledger_delta, r_bridge) = if voice_enabled {
        let vb = voice_exec.as_deref().expect("voice_enabled ⟹ Some");
        let final_equity_v = vb.cash() + voice_end_net as f64 * final_px;
        (
            vb.account_price_pnl(),
            vb.cum_fee(),
            cum_funding_v,
            cum_borrow_v,
            cum_liq_v,
            final_equity_v - nav0,
            (cum_funding_v + cum_borrow_v + cum_liq_v) as i64,
        )
    } else {
        (cum_price_pnl, cum_fee, cum_funding, cum_borrow, cum_liq_loss, ledger_delta, tw_holding_cost_bridge)
    };
    let r_decomp = super::super::strategy::risk::RDecomposition::assemble(
        r_price, r_fee, r_fund, r_borr, r_liq, r_ledger_delta, r_bridge,
    );
    // 守恒断言（no-patch-mentality：残差超容差 = 真实资金泄漏 bug，不静默）。容差按名义规模缩放
    // （f64 累加 O(n) 舍入；nav0 量级 + 累计项量级）——绝对容差 max(1e-6, 1e-9·(|nav0|+|price_pnl|)）。
    let cons_tol = 1e-6_f64.max(1e-9 * (nav0.abs() + r_price.abs()));
    debug_assert!(
        r_decomp.conservation_residual.abs() <= cons_tol,
        "M6 R 分解守恒残差 {} 超容差 {}（net_r={} vs ledger_delta={}）——资金泄漏",
        r_decomp.conservation_residual, cons_tol, r_decomp.net_r, r_ledger_delta
    );

    let daily_returns = bar_returns(&equity_curve);
    // ★nest-gate 拒绝率实测落账（实装卡 §3.4-3）：门开启时输出分通道统计（env 纯诊断惯例，
    // 门关闭恒零不输出）。分账口径：cert_none/nest_n_delta_false/xzd_gate_fail = 门实装生效
    // （可修复分量已修）；其中 rungs 空基例放行占比高 = 跨级链稀薄（内禀于塔现状，挂路径 c）。
    // ★#75：增打真链命中/链深构成（准拒分账）+ L2 旧臂对照差 + Xzd 回退；NEST_GATE_INDEX 行
    // 落索引侧链深构成（对照 72.7% 单级基线）与增量喂法成本计数（派生次数/索引重建次数）。
    // ★#94 两行分工（Spec A1 字面错位消除）：NEST_GATE_STATS = 准入判定分账（total/admitted/
    // 分通道准拒——「门放不放行」的账面）；NEST_GATE_CHAIN = 对照读数行。T4 (#173)：CHAIN
    // 行的并集/single comparison shadow 列（typed_found/typed_none/rungs 准拒分账/
    // level_hits/single_multi_divergence）随旧桥退役删除，仅剩 Xzd 回退计数与 L2 旧臂
    // cross 对照（链深构成读数由 NEST_GATE_INDEX 行承担）。
    // ★#94 cross 口径：复用通道（typed 无证 ∧ 旧臂 Xzd 复用，admit≡old_admit by construction）
    // **不计入对照差**（agree/old_pass_new_rej/old_rej_new_pass 仅含两路独立判定），
    // 单独计 reuse 列——剔除自证成分，对照差只反映真链/回退与旧臂的真实分歧。
    if nest_gate_hist.is_some() {
        let s = &nest_gate_stats;
        eprintln!(
            "NEST_GATE_STATS total={} admitted={} rejected={} | nest_pass={} xzd_pass={} | rej: flat_dir={} no_level={} cert_none={} nest_n_delta_false={} xzd_gate_fail={}",
            s.total,
            s.admitted,
            s.total - s.admitted,
            s.nest_pass,
            s.xzd_pass,
            s.rej_flat_dir,
            s.rej_no_level,
            s.rej_cert_none,
            s.rej_nest_n_delta,
            s.rej_xzd_gate,
        );
        eprintln!(
            "NEST_GATE_CHAIN xzd_fallback={} | cross agree={} old_pass_new_rej={} old_rej_new_pass={} reuse={}",
            s.xzd_fallback,
            s.cross_agree,
            s.cross_old_pass_new_rej,
            s.cross_old_rej_new_pass,
            s.cross_reuse,
        );
        // ★T3 (#172) → T5a (#207)：链判定读数行（纯增量新行——NEST_GATE_STATS/CHAIN 行
        // schema 逐字节不动，红线对照直接可比；chain_pass = 链确认读数，wf7 结算面对
        // T3 时代 39 / 并集 17 基线）。T5a：dir_witness_divergence 列随方向见证退役删除
        // （「方向分歧」是非概念，ADR 20260723 裁定 1）。
        eprintln!(
            "NEST_GATE_T3 chain_pass={} chain_reject={}(missing={} broken={}) chain_none={} | top_dist={:?}",
            s.chain_pass,
            s.chain_reject,
            s.chain_reject_missing,
            s.chain_reject_broken,
            s.chain_none,
            s.chain_top_dist,
        );
        if let Some(gate) = &nest_chain_gate {
            let st = gate.index.stats();
            let share = st
                .single_level_share()
                .map(|v| format!("{v:.4}"))
                .unwrap_or_else(|| "na".to_string());
            let events: usize = gate.events_by_level.iter().map(Vec::len).sum();
            eprintln!(
                "NEST_GATE_INDEX events={} events_seen={} base_events={} assembled={} indexed={} rungs_0={} rungs_1={} rungs_2p={} single_level_share={} | derivations={} index_builds={} provider_errors={}",
                events,
                gate.n_events_seen,
                st.base_events,
                st.assembled,
                st.indexed,
                st.rungs_0,
                st.rungs_1,
                st.rungs_2_plus,
                share,
                gate.n_derivations,
                gate.n_index_builds,
                gate.n_provider_errors,
            );
            // ★#214（spec endorsement-failure-instrument-20260724 ID-5）：背书失败原因
            // 测量——纯增量新行，NEST_GATE_STATS/CHAIN/T3/INDEX 行 schema 逐字节冻结
            // （T3 先例）；读数 = 全量计数（v3：无抽样、无概率推断）。两行分工：
            // NEST_GATE_FAIL = Trend 四桶 + owner 子计数 + 点级（类 × owner 判同）二维 +
            // 遍历计数 + Pan 成功/失败总数；NEST_GATE_LEVEL = base/assembled/indexed
            // 按级 × kind 两维分解（行 = 级别槽，列 [Trend, Consolidation]，{:?} 沿用
            // T3 top_dist 先例）。
            // ★#218 面 B（spec owner-attribution-fix-20260724 ID-4，行族 schema 演进随票
            // 登记——#214 新增行族、非冻结红线面）：桶名 owner_start_neq → owner_anchor_neq
            // （判同机制换两族锚）；owner_pts_id_missing → owner_pts_anchor_missing（语义
            // 重定 = 锚不可解）；新增 band_eq_start_neq（带碰撞观察，US-08）与
            // scan_pts 三计数（遍历计数，ID-5 不变量断言集读数）。
            let inst = gate.index.instrument();
            let tp = inst.trend_pts;
            eprintln!(
                "NEST_GATE_FAIL trend={} success={} owner_anchor_neq={} out_of_window={} opposite_side={} no_valid_point={}(book_missing={} book_empty={}) owner_pts_anchor_missing={} owner_pts_real_neq={} band_eq_start_neq={} | pts_eq/total c1={}/{} c2={}/{} c3={}/{} | scan_pts book_total={} in_window={} same_side={} | pan_success={} pan_fail={}",
                inst.trend_total(),
                inst.trend_success,
                inst.trend_owner_anchor_neq,
                inst.trend_out_of_window,
                inst.trend_opposite_side,
                inst.trend_no_valid_point,
                inst.nvp_book_missing,
                inst.nvp_book_empty,
                inst.owner_anchor_missing_pts,
                inst.owner_real_neq_pts,
                inst.band_eq_start_neq_pts,
                tp[0][0], tp[0][0] + tp[0][1],
                tp[1][0], tp[1][0] + tp[1][1],
                tp[2][0], tp[2][0] + tp[2][1],
                inst.scan_book_total,
                inst.scan_in_window,
                inst.scan_in_window_same_side,
                inst.pan_success,
                inst.pan_fail,
            );
            eprintln!(
                "NEST_GATE_LEVEL base_lk={:?} assembled_lk={:?} indexed_lk={:?} | base_trend={} base_pan={}",
                inst.base_by_level_kind,
                inst.assembled_by_level_kind,
                inst.indexed_by_level_kind,
                inst.base_trend(),
                inst.base_consolidation(),
            );
        }
    }
    FillOutput {
        equity_curve,
        daily_returns,
        // ★W1：声部臂输出声部账户口径（平仓 fill 费后已实现/逐声部 TradeRecord/声部 fill
        //    事件数）——设计性改变（审计 §7.4 如实登记），非回归；净额影子对应量不进输出。
        trade_pnls_realized: if voice_enabled { voice_trade_pnls } else { trade_pnls },
        trade_pnls_with_forced,
        trades: if voice_enabled { voice_trades } else { trades },
        n_orders: if voice_enabled { n_voice_fills } else { n_orders_executed },
        typed_ledger,
        voice_verdicts: verdict_ledger,
        tw_final: Some(tw_thread.finish()),
        r_decomp: Some(r_decomp),
        account_view,
        center_oscillation_actions: osc_actions,
        campaign_book,
        campaign_witness,
    }
}

/// [`plan_and_fill_mtm`] 的完整产出（双口径 trade_pnls + 操作语义随机对照的输入）。
pub(super) struct FillOutput {
    /// 逐 bar 归一化权益曲线（MtM 含浮盈）。
    pub(super) equity_curve: Vec<f64>,
    /// 逐 bar returns（Sharpe/显著性输入）。
    pub(super) daily_returns: Vec<f64>,
    /// 已实现成交盈亏（**不含**窗口终点强平浮盈，§3.4 bootstrap 输入）。
    pub(super) trade_pnls_realized: Vec<f64>,
    /// 含浮盈成交盈亏（窗口终点强平未平仓持仓，编排者铁律「浮盈算上」）。
    pub(super) trade_pnls_with_forced: Vec<f64>,
    /// 交易执行轨迹（[`metrics::TradeRecord`]，含强平笔；随机对照输入）。
    pub(super) trades: Vec<metrics::TradeRecord>,
    /// 执行订单数（n_orders_executed > 0 ⟺ is_l2）。
    pub(super) n_orders: usize,
    /// 腿级 typed 交易 ledger（G4；π 路径 [`pi_theta_fill_loop`] 产出，v1 路径
    /// [`plan_and_fill_mtm`] 无腿级台账 ⟹ 恒空，诚实不伪造）。
    pub(super) typed_ledger: Vec<TypedTrade>,
    /// #201 阶段 B 裁决账本轨：π 路径逐 bar 消费的显式 per-voice 裁决序列（含显式 Hold，
    /// schema 冻结见 [`VoiceVerdictRec`]）；v1/dual 路径无腿级裁决 ⟹ 恒空（诚实不伪造，
    /// 同 `typed_ledger` 先例）。加轨不减轨——不进 μ estimand、不改任何既有轨。
    pub(super) voice_verdicts: Vec<VoiceVerdictRec>,
    /// TW 账本终态（#124 裁定4：TwState 生产真值源在 π fill loop；TStage/ηBucket「生产者已
    /// 就位」的可观测物证——G3 ZExt 桥/诊断消费）。v1 路径无 TW 接线 ⟹ `None`（诚实不伪造）。
    pub(super) tw_final: Option<TwState>,
    /// M6 R 分解表（路线.pdf p16 第十一关：R=ΣN_tΔP_t−Commission−Slippage−Funding−Borrow−
    /// LiquidationLoss + 账目守恒残差）。π 路径 [`pi_theta_fill_loop`] 产出；v1 路径
    /// [`plan_and_fill_mtm`] 未接 R 分解 ⟹ `None`（诚实不伪造，v1 是 recognize 旧路径非生产 π）。
    pub(super) r_decomp: Option<super::super::strategy::risk::RDecomposition>,
    /// #197 并行记账视图（expand 只读旁路）：π 路径逐腿镜像开/关生命周期事件过账的
    /// （账户, 级别, 仓位节点）分实例账本；v1/dual 路径无腿级生命周期事件 ⟹ 恒空
    /// （诚实不伪造，同 `typed_ledger` 先例）。不改净额路径任何语义。
    pub(super) account_view: strategy::account::ParallelAccountLedger,
    /// #292（T2 门控接线可见性证据）：逐条「触发→减补动作」记录。π overlay 路径
    /// [`pi_theta_fill_loop_overlay`] 在 `config.center_oscillation.enabled` 时产出；v1/dual
    /// 路径未接线 ⟹ 恒空（诚实不伪造，同 `typed_ledger` 先例）。真实记账是 T3（#293）职责，
    /// 本轨不改任何既有订单/账本语义。
    pub(super) center_oscillation_actions:
        Vec<super::super::strategy::center_oscillation_trade::CenterOscillationActionRecord>,
    /// issue #357（T4/#294 生产实例化）：每仓 campaign 账簿终态——π overlay 路径逐 bar
    /// `sync_position`/`apply_action` 驱动；v1/dual 路径未接线 ⟹ 恒空账簿（诚实不伪造，
    /// 同 `center_oscillation_actions` 先例）。
    pub(super) campaign_book: super::super::strategy::oscillation_campaign::CampaignBook,
    /// issue #357 验收③：enabled=true wf8 产物级见证读数（减补动作归属分桶/`CenterNotAlive`
    /// 丢弃率/挂起归宿/campaign 生死事件）。v1/dual 路径未接线 ⟹ 恒 `default()`。
    pub(super) campaign_witness: super::super::strategy::oscillation_campaign::CampaignWiringWitness,
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
pub(super) fn plan_and_fill_mtm(
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
    // ★#76 出场侧真链门（门开才构建，门关 ⟹ None 零开销、全路径逐字节不变）：
    // 逐 bar 前缀因果喂法 + T5b (#208) 迁链终态 = 与进场门同一套同点递归链反查
    //（ExitNestGateCtx 文档，方案 (b) 预注入）；判定唯一源 = 链三态（Pass 准出 /
    // Reject·NoChain 诚实不准出）；v0 基例双读对照落账。
    let mut exit_gate: Option<ExitNestGateCtx> = if nest_cert_gate_enabled() && n > 0 {
        // T5b (#208) 出场 dump 路径标注（#[cfg(test)]，装置 env 未设 = no-op）。
        #[cfg(test)]
        super::admission::t5b_exit_dump::set_path("v1");
        Some(ExitNestGateCtx::new(bars, config))
    } else {
        None
    };

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
        // ★nest-gate（实装卡 §2.4，#76 真链升格）：THETA_NEST_CERT_GATE=1 ⟹ 反向项 χ^{σ_p}
        // 消费对象从裸 BspBits 升格为**同点递归链确认**（ExitNestGateCtx 逐 bar 前缀喂法 +
        // T5b (#208) 迁链终态 = 与进场门同一 chain_lookup，exit_decision_for_nested_cert
        // 注入查询闭包，四析取结构不动）；未设 ⟹ 裸 bits 路径逐字节不变（bit-exact 回归锁）。
        if !bar.untradable && px > 0.0 {
            // 当前账本权益（RiskClose 的 GlobalRiskClose 判据需要，Origin.RiskProj）。
            let equity_now = cash + units * px;
            // ★#76：门开 ⟹ 本 bar 前缀喂法（事件账本 + 索引惰性重建）——逐 bar 因果纪律
            // 与 π 进场门同款；门关 ⟹ 零工作。
            if let Some(ctx) = exit_gate.as_mut() {
                ctx.sync_bar(i, &groups[i]);
            }
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
                let exit_d = match exit_gate.as_mut() {
                    Some(ctx) => {
                        let mut q = |d: &VoiceDecision| ctx.reverse_admit(d, depth);
                        super::super::strategy::exit::exit_decision_for_nested_cert(
                            &hv, depth, bar, i, &groups, equity_now, false, &mut q,
                        )
                    }
                    None => exit_decision_for(&hv, depth, bar, i, &groups, equity_now),
                };
                if let Some(exit_d) = exit_d
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

    // ★#76：门开 ⟹ 出场侧真链门读数落账（NEST_GATE_EXIT；门关恒不输出）。
    if let Some(ctx) = &exit_gate {
        ctx.stats.report("v1");
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
        voice_verdicts: Vec::new(), // v1 无腿级裁决（诚实空，同 typed_ledger 先例）
        tw_final: None,           // v1 无 TW 接线（#124 只接 π 路径，诚实 None）
        r_decomp: None,           // v1 无 R 分解（M6 只接生产 π 路径，诚实 None）
        account_view: strategy::account::ParallelAccountLedger::new(), // v1 无腿级生命周期（诚实空，#197）
        center_oscillation_actions: Vec::new(), // v1 无 #292 门控接线（诚实空，同 typed_ledger 先例）
        campaign_book: super::super::strategy::oscillation_campaign::CampaignBook::new(), // v1 无 #357 接线（诚实空）
        campaign_witness: super::super::strategy::oscillation_campaign::CampaignWiringWitness::new(),
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
pub(super) struct FillOutputDual {
    /// 与 [`plan_and_fill_mtm`] 同构的产出（equity/pnls/trades/n_orders——E4 对拍对象）。
    pub(super) fill: FillOutput,
    /// 双账本终态（窗口终点双腿强平**已应用**于账本 ⟹ 终态零持仓；强平 PnL 只入
    /// `trade_pnls_with_forced`，与净额路径 :2883-2894 同口径）。
    pub(super) ledger: dual_ledger::DualLedger,
    /// 逐腿成交日志（含强平笔）：bar/depth/leg/close/成交手数/realized/双腿后态。
    pub(super) leg_log: Vec<LegFillRec>,
}

/// 逐腿成交记录（[`plan_and_fill_mtm_dual`] 的可观测轨迹）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct LegFillRec {
    pub(super) bar: usize,
    pub(super) depth: u32,
    pub(super) leg: super::super::strategy::voice::VoiceSide,
    pub(super) close: bool,
    pub(super) qty: f64,
    pub(super) realized: f64,
    pub(super) q_long_after: f64,
    pub(super) q_short_after: f64,
}

/// voice_qty 同步（双账版，[`apply_voice_fill`] 同语义：两段真实成交分别扣/加，
/// 绝不按原始请求量或 action 推断）。`FillOutcomeDual` 单腿单向 ⟹ closed/opened 至多一侧非零。
pub(super) fn apply_voice_fill_dual(
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
///   活动集 + 真 ReverseOpen 角色门 ⟹ depth>0 子声部**运行时产出**（非纸面声部）。
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
///   ReverseOpen 子决策的反父 bits 不喂反向项（M13：父仓穿越次级反向信号持有，短差由子腿
///   承担，非父平仓触发）；同级别反向平仓由 interpret 规则2 承载（close 决策携入场快照
///   bsp，非当 bar 信号，不入池）。exit_decision_for_nested 的 stop/risk/parent_invalid
///   与级联覆盖其余关闭通道。
/// - **交易轨迹**：per-leg `track_position_transition`（多腿 +q_long / 空腿 −q_short 有符号
///   喂入）——兼容腿标记（无真双开）下与净额 `units` 轨迹**同一交易列表**（E4 对拍锁）。
///
/// `apply_voice_fill` 语义不变（voice_qty[depth]=手数——单脊柱下 depth↔腿 1:1，
/// side 由 held 台账定）。R 分解/typed_ledger 无接线（诚实 None/空，同 v1 净额路径）。
///
/// ★#68② TW 账本已接线（tw_final=Some，镜像 π loop #124 口径 runner.rs:1416-1437）：
/// - **注资**：整窗=一个 campaign，`free=notional_in=⌊nav0⌋`（同 :1416）。
/// - **②' ShortDiff 成本划转**：`basis = q⁺·cost⁺ + q⁻·cost⁻`（分腿在险成本基——
///   净额口径 `|units|·entry_cost` 的双腿推广；空腿 `cost⁻` 为卖出净收/单位，其绝对额=
///   在险市值，同净额空头 `entry_cost.abs()` 口径）方向差分派 `TwEvent::ShortDiff`，
///   shadow 追真实+入账 cash-sound 钳制（同 :1555-1573）。
/// - **②'' Realize 平仓腿入账**：平仓腿费后 PnL 累计（`FillOutcomeDual.realized`，A'
///   结算源）量化差分派 `TwEvent::Realize(d_pi)`（同 :1576-1591）——**强平 PnL 不入**
///   （trade_pnls_with_forced 口径同净额路径，TW 快照取强平前）。
/// - **legacy 腿计数**（#124 同语义）：ReverseOpen 角色（depth>0 子声部）开仓派
///   `OpenShareLeg`、全平派 `CloseShareLeg(0)`（profit 口径量 0 承载，同 :2191-2194 声明）。
/// - **未接（诚实边界，非缺陷）**：stage 推进机构（`coverage::TwStepCtx` + κ policy +
///   `stage_progression`，π loop :1750/:2205 消费侧机构）——dual 路径无 P2/P3/P4 消费方，
///   `stage` 恒 `CostReduction` 是「无推进机构」的诚实镜像；接入点 = 同 π loop 的
///   TwStepCtx 装配处。E2 G5 同数锁（TW ShortDiff 事件金额≡子腿 realized 的另一会计身份）
///   是**独立会计裁定**，不属于本接线（E2 测试保持 ignore 并更新理由）。
pub(super) fn plan_and_fill_mtm_dual(
    classification: &super::super::classifier::Classification,
    tower: &[Rc<Vec<super::super::classifier::recursive_tower::LeveledMove>>],
    bars: &[Bar],
    initial_nav: f64,
    config: &ThetaConfig,
) -> FillOutputDual {
    use super::super::strategy::exec::fill_bar_index;
    use super::super::strategy::exit::{
        exit_decision_for_nested, parent_invalid_at, subtree_close_exit_decisions,
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

    // ── #68② TW 账本（镜像 π loop #124 口径，runner.rs:1416-1437 同源语义；tw_final 物证）。
    //    注资口径同 funded_campaign：整窗=一个 campaign，free=notional_in=⌊nav0⌋。
    // ★B-M3b-expand（#90）：TW 账本线程收敛为 TwLedgerThread。──
    let mut tw_thread = TwLedgerThread::new(nav0, RiskPolicy::baseline());
    // ★#76 出场侧真链门（门开才构建，门关 ⟹ None 零开销、全路径逐字节不变）：
    // 逐 bar 前缀因果喂法 + T5b (#208) 迁链终态 = 与进场门同一套链反查
    //（ExitNestGateCtx 文档，方案 (b) 预注入）。
    // dual 的 decisions 仍由全窗 classification/tower 经 recognize_nested 产出（deprecated
    // F-01 口径不动），门喂法走 IncrementalClassifier 逐 bar 因果前缀——判定不新增前视分量。
    let mut exit_gate: Option<ExitNestGateCtx> = if nest_cert_gate_enabled() && n > 0 {
        // T5b (#208) 出场 dump 路径标注（#[cfg(test)]，装置 env 未设 = no-op）。
        #[cfg(test)]
        super::admission::t5b_exit_dump::set_path("dual");
        Some(ExitNestGateCtx::new(bars, config))
    } else {
        None
    };

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
                    tw_thread.add_realized(fill.realized); // #68② ②''：平仓腿费后 PnL 累计（开仓腿=0 无效应）
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
                        // #68② TW 腿计数（#124 同语义）：ReverseOpen 子声部全平 ⟹ CloseShareLeg(0)。
                        if depth > 0 {
                            tw_thread.close_share_leg();
                        }
                    } else if let Some(Some(h)) = held.get_mut(depth) {
                        h.exit_pending = false;
                    }
                    n_orders_executed += 1;
                }
            }
        }

        // ── 2. 开仓循环：per-bar recognize_nested（held 活动投影 + 真 ReverseOpen 角色门）。 ──
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
                    tw_thread.add_realized(fill.realized); // #68② ②''：平仓腿费后 PnL 累计（开仓腿=0 无效应）
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
                        // #68② TW 腿计数（#124 同语义，:2196-2204 镜像）：ReverseOpen 角色
                        // （depth>0 子声部）开仓 ⟹ OpenShareLeg（OQ-9 守卫：EarningShares
                        // 阶段开 legacy 腿非法——本路径无 stage 推进机构，恒 CostReduction，
                        // is_legal_from 恒真；保留守卫调用与 π loop 同形）。
                        if d.depth > 0 {
                            tw_thread.open_share_leg();
                        }
                    } else if voice_qty.get(depth).copied().unwrap_or(0) == 0 {
                        if let Some(slot) = held.get_mut(depth) {
                            *slot = None;
                        }
                        // #68② TW 腿计数：ReverseOpen 子声部全平 ⟹ CloseShareLeg(0)。
                        if depth > 0 {
                            tw_thread.close_share_leg();
                        }
                    }
                    n_orders_executed += 1;
                }
            }
        }

        // ── 3. 退出决策生成器（§9 closePred 四析取皆实：parent_invalid 实义化 + cascade）。 ──
        // ★nest-gate（实装卡 §2.4，#76 真链升格）：THETA_NEST_CERT_GATE=1 ⟹ depth=0 反向项
        // 升格为**同点递归链确认**（ExitNestGateCtx 逐 bar 前缀喂法 + T5b (#208) 迁链
        // 终态 = 与进场门同一 chain_lookup，exit_decision_for_nested_cert 注入查询闭包）；
        // 未设 ⟹ 逐字节不变（bit-exact 回归锁）。
        if !bar.untradable && px > 0.0 {
            let equity_now = ledger.equity(px);
            let groups_view: Vec<Vec<&VoiceDecision>> =
                groups.iter().map(|g| g.iter().collect()).collect();
            // ★#76：门开 ⟹ 本 bar 前缀喂法（事件账本 + 索引惰性重建）；门关 ⟹ 零工作。
            if let Some(ctx) = exit_gate.as_mut() {
                ctx.sync_bar(i, &groups_view[i]);
            }
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
                let exit_d = match exit_gate.as_mut() {
                    Some(ctx) => {
                        let mut q = |d: &VoiceDecision| ctx.reverse_admit(d, depth);
                        super::super::strategy::exit::exit_decision_for_nested_cert(
                            &hv, depth, bar, i, &groups_view, equity_now, parent_invalid, &mut q,
                        )
                    }
                    None => exit_decision_for_nested(&hv, depth, bar, i, &groups_view, equity_now, parent_invalid),
                };
                if let Some(exit_d) = exit_d
                {
                    if let Some(fi) = fill_bar_index(i, bars, &config.exec) {
                        if fi < n {
                            // 级联发射（M16 AncOK 父关则子关，最深优先）+ pending 标记。
                            // ★#183 T4 归一：生产级联归一到镜像函数（held 槽压缩链投影 →
                            // subtree_close；散装 cascade_exit_decisions 已下线）。
                            for d in subtree_close_exit_decisions(&held, depth, exit_d, i) {
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

        // ── #68② TW ②' 成本划转 + ②'' 已实现入账（π loop :1555-1591 同式镜像，置于 bar 末、
        //    权益推进前；dual 路径无 ③ TwStepCtx stage 判据消费方——物证账本，决策层零消费）。──
        // ②' 分腿在险成本基（q⁺·cost⁺ + q⁻·cost⁻，|units|·entry_cost 的双腿推广）方向差分。
        {
            let basis_now =
                (ledger.q_long * ledger.cost_long + ledger.q_short * ledger.cost_short) as i64;
            tw_thread.sync_basis_raw(basis_now);
        }
        // ②'' 平仓腿费后 PnL（A' 结算源）量化差分 ⟹ Realize(d_pi) 入 free（唯一 TW 漂移
        // 构造子，可正可负；TW 漂移恒 = ⌊realized_cum⌋）。强平 PnL 不入 realized_cum
        // （同净额路径 forced_pnl 排除口径，:2884-2894）。
        tw_thread.sync_realized();

        // 权益曲线（净投影估值，M30；归一化 ÷nav0）。
        let equity = ledger.equity(px) / nav0;
        equity_curve.push(equity);
    }

    // ★#76：门开 ⟹ 出场侧真链门读数落账（NEST_GATE_EXIT；门关恒不输出）。
    if let Some(ctx) = &exit_gate {
        ctx.stats.report("dual");
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
            voice_verdicts: Vec::new(), // 无腿级裁决（诚实空，同 typed_ledger 先例）
            tw_final: Some(tw_thread.finish()),       // #68② TW 已接线（ShortDiff 成本划转 + Realize 平仓入账 + ReverseOpen 腿计数；快照取强平前，强平 PnL 不入 TW）
            r_decomp: None,
            account_view: strategy::account::ParallelAccountLedger::new(), // 无腿级生命周期（诚实空，#197）
            center_oscillation_actions: Vec::new(), // dual 路径无 #292 门控接线（诚实空，同 typed_ledger 先例）
            campaign_book: super::super::strategy::oscillation_campaign::CampaignBook::new(), // dual 路径无 #357 接线（诚实空）
            campaign_witness: super::super::strategy::oscillation_campaign::CampaignWiringWitness::new(),
        },
        ledger,
        leg_log,
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
pub(super) fn apply_voice_fill(voice_qty: &mut [u32], depth: usize, fill: FillOutcome) {
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
