//! B-M3a（#87）：净额执行原语 seam——自 `runner.rs` **纯移动**（设计文档
//! `chanlun/plans/runner-rs-seam-designs-20260721.md` §M3 节拍 3）：
//! `simulate_fills` / `apply_order` / `apply_fill` / `FillOutcome` / `bar_returns`。
//! 零语义改动；`dual_ledger.rs:472` D8 对拍经 `runner::apply_order` 门面 `pub(crate) use` 继续命中。

use super::super::config::ThetaConfig;
use super::super::types::{Bar, Order, StrictAction};
use super::super::strategy::exec::FillSide;
// ★#423 第二阶段 C 线：本文件生产区段**不** import `LiquidityRole`——成交报价走
// `FeeQuoter::production_rate_or_fallback`（无角色参数，角色单一来源 =
// `venue_fee::PRODUCTION_LIQUIDITY_ROLE`）。测试区段按需自行 import（见 `mod tests`）。
use super::super::venue_fee::FeeQuoter;

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

    // 费率解析器（#360 单源门面）：未标定档 = 三常数合成率（逐位现状），标定档按 datum 逐笔解析。
    let fees = super::treasury::fee_quoter(&config.exec);

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
            let fee_rate = order_fee_rate(&fees, o, px, units);
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
/// **订单 → (有符号成交方向 δ, 是否纯平仓)** 的**唯一**判定表。
///
/// `None` ⟹ 本订单不产生任何成交（Hold/Wait，或空仓收到纯平仓单）。抽出的理由（#360）：
/// [`apply_order`] 的账本算术与 [`order_fee_rate`] 的费率解析**必须同源**——两份手抄的 match
/// 会在任一侧改动时静默分叉（费率走买入档、现金流走卖出档，没有断言拦得住）。
pub(crate) fn order_delta_close_only(action: StrictAction, units: f64) -> Option<(f64, bool)> {
    match action {
        StrictAction::Buy | StrictAction::Add => Some((1.0, false)),
        StrictAction::Sell => Some((-1.0, false)),
        StrictAction::Reduce | StrictAction::Close => {
            if units > 0.0 {
                Some((-1.0, true)) // 持多 ⟹ 卖出平多
            } else if units < 0.0 {
                Some((1.0, true)) // 持空 ⟹ 买回平空
            } else {
                None // 空仓无仓可平
            }
        }
        StrictAction::Hold | StrictAction::Wait => None, // 不动
    }
}

/// ★#360 venue 费率解析（生产成交回路的唯一费率取值方式）。
///
/// 未标定档（`ExecConfig::fee_schedule = None`）⟹ 恒为三常数合成率，与改动前**逐位相同**；
/// 标定档 ⟹ 按本笔 (qty, px, 成交方向) 解析 datum + 未被 datum 覆盖的滑点（`FeeQuoter`）。
///
/// **流动性角色恒 [`PRODUCTION_LIQUIDITY_ROLE`](super::super::venue_fee::PRODUCTION_LIQUIDITY_ROLE)
/// （= Taker）**：本引擎全部成交按 bar close 市价撮合，无挂单语义 ⟹ 取 maker 档是声明膨胀
/// （090）。datum 里的 maker 档留给真限价执行模型（报告 §3.1）。★#423 第二阶段 C 线起本文件
/// 三个成交点**不传角色**——走 [`FeeQuoter::production_rate_or_fallback`]，角色在 quoter 内部
/// 取那个常量（单一来源；在此写角色字面量已是编译错误，无参可传）。
///
/// **询价量 = 本笔可成交量的上界，且已扣掉可预知的裁量**：纯平仓单按
/// `min(qty, |units|)` 询价（[`apply_fill`] 段 1 的同一 clamp）。剩余唯一偏差源是**开仓段
/// 现金不足拒单**——该分支拒的是整段余量（非部分成交），故仅当"平反向段成交 + 开新仓段被拒"
/// 同时发生**且**用 per-share 档时，最低佣金按偏大的量摊薄 ⟹ **低估成本**。彻底消除需两趟
/// 成交（先定量再询价），属订单层改造，不在 #360；per-notional 档与未标定档无此偏差。
fn order_fee_rate(fees: &FeeQuoter, o: &Order, px: f64, units: f64) -> f64 {
    let Some((delta, close_only)) = order_delta_close_only(o.action, units) else {
        return fees.fallback_rate(); // 不成交 ⟹ 费率不进算术
    };
    let qty = o.qty as f64;
    let qty = if close_only { qty.min(units.abs()) } else { qty };
    let side = if delta > 0.0 { FillSide::Buy } else { FillSide::Sell };
    fees.production_rate_or_fallback(qty, px, side)
}

/// 双腿账本 [`LegOrder`](super::super::strategy::LegOrder) 的成交费率：(腿方向, 开/平) ⟹ 买卖
/// 方向；平仓腿按 `min(qty, 该腿持仓)` 询价（`dual_ledger::apply_fill_dual` 的同一 clamp）。
fn leg_fee_rate(
    fees: &FeeQuoter,
    lo: &super::super::strategy::LegOrder,
    px: f64,
    ledger: &dual_ledger::DualLedger,
) -> f64 {
    use super::super::strategy::voice::VoiceSide;
    let (side, qty) = match (lo.leg, lo.close) {
        (VoiceSide::Long, false) => (FillSide::Buy, lo.order.qty as f64),
        (VoiceSide::Short, false) => (FillSide::Sell, lo.order.qty as f64),
        (VoiceSide::Long, true) => (FillSide::Sell, (lo.order.qty as f64).min(ledger.q_long)),
        (VoiceSide::Short, true) => (FillSide::Buy, (lo.order.qty as f64).min(ledger.q_short)),
        (VoiceSide::Flat, _) => return fees.fallback_rate(), // Flat 腿不成交
    };
    fees.production_rate_or_fallback(qty, px, side)
}

/// 账户净持仓在窗口终点强平（含浮盈口径）的成交费率：持多 ⟹ 卖出，持空 ⟹ 买回。
/// 询价量 = `|units|`（强平量确定，无裁量偏差）。
fn forced_flatten_fee_rate(fees: &FeeQuoter, units: f64, px: f64) -> f64 {
    let side = if units > 0.0 { FillSide::Sell } else { FillSide::Buy };
    fees.production_rate_or_fallback(units.abs(), px, side)
}

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
    // ★#360：判定表已抽成 [`order_delta_close_only`]（与 `order_fee_rate` 的费率解析同源，
    //   杜绝两份手抄 match 静默分叉）。语义逐字不变。
    let Some((delta, close_only)) = order_delta_close_only(o.action, *units) else {
        return FillOutcome::noop();
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
    OpsemDump, StrictNestSidecarCollector,
};
use super::admission::{
    voice_exec_gate, nest_cert_gate_enabled,
    NestGateStats,
    NestChainGate,
    ExitNestGateStats, ExitNestGateCtx,
    k_theta_risk_gate, kappa_policy_resolved,
};
use super::admission::ChiFilterCtx;
use super::ledger::{track_position_transition, LedgerOpen, OpsemEntrySnapshot, TwLedgerThread, TypedTrade, TYPED_TRADE_SCHEMA_VERSION};
#[cfg(test)]
use super::admission::{VOICE_EXEC_OVERRIDE, NEST_CERT_GATE_OVERRIDE};

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
    // ★LEE M1：level_ledger=None ⟹ 级别账本镜像整段跳过（bit-exact 回归锁）。
    pi_theta_fill_loop_overlay(classify_at, bars, initial_nav, config, chi, None, None, None)
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
    pi_theta_fill_loop_overlay(classify_at, bars, initial_nav, config, chi, None, None, Some(voice))
}

/// ★M5 声部执行层 fill loop（多空对冲.pdf p16 关卡10）：与 [`pi_theta_fill_loop`] **同一决策路径**
/// （同一 `pi_theta_step_traced` → 净额订单 → apply_order fill），额外用 `StepTrace.sep_legs` 喂
/// `overlay: OverlayState` hedge-mode 簿——建持久逐声部 P^sep 账本 + ΔN 订单流 + 逐声部 pnl_v 归因。
///
/// `overlay=None` ⟹ 净额路径 bit-exact（所有现有臂）；`Some(&mut ov)` ⟹ 逐 bar 决策点把 sep_legs
/// 步进 overlay（**只读旁路**，不改净额 fill 的 cash/units/trade_pnls ⟹ 现有数字不动）。
///
/// ★LEE M1 `level_ledger`（multi-level-native-execution-design-20260719 §D M1）：与 overlay 并列的
/// 第二重只读旁路——同一决策点同一 `sep_legs` 按 `id.level`≡formation_level 分桶步进
/// [`LevelLedgerMirror`](super::super::strategy::level_ledger::LevelLedgerMirror)，逐 bar
/// debug_assert **LEE-Net 恒等** `Σ_ℓ net_ℓ == overlay.net()`（加性细化，设计文档 §C.2）。
/// `None` ⟹ 整段跳过，逐字节不变（bit-exact 回归锁）。
///
/// ★W1 `voice_exec=Some`（声部独立执行臂，审计 §7）：净额影子账本（cash/units/entry_cost）
/// **原样跑**——决策层（typed_ledger/TW/sep_legs/gate/base_units）全部读影子真值 ⟹ 与净额臂
/// 逐字节一致；真实执行投影走声部簿（pending_voice 队列 + 事件驱动 fill + 开仓冻结 sizing），
/// 输出层（equity/trade_pnls/trades/n_orders/r_decomp）换为声部账户口径（设计性改变，非回归）。
/// `voice_exec=None` ⟹ 下列声部分支全部跳过，净额路径逐字节不变（bit-exact 回归锁）。
/// ★★LEE M3 单决策点级别归因 + **事件门控**（`pi_theta_fill_loop_overlay` 的订单出口，§D M3）。
///
/// M2 版每 bar 用当前 `sep_legs` 重估各级目标；M3 起**只在 `clock_ℓ` 事件时点重估，无事件 bar
/// 目标=前值**（设计文档 §D M3 逐字）。事件集定义与 `LevelState` 逐字段对齐见
/// [`level_clock`](super::super::strategy::level_clock) 模块头。
///
/// 三段，域分明（§F③ 风控/门控优先级形式化的实装落点）：
///
/// 1. **门控重估**（结构域）：`basis^gated = regate(net_ℓ, ticked)`——有 tick 的级别取本 bar
///    `net_ℓ`（[`level_nets`](super::super::strategy::level_ledger::level_nets)，与 M1 镜像
///    `nets()` 构造性同源，禁第二裁决源），无 tick 的级别取 `q_ℓ^plan` 前值。
/// 2. **账户层/风控投影**（风控域，**每 bar 无条件**）：`p̃_lee = Σ_ℓ basis^gated_ℓ` 经
///    [`pi_theta_position`](super::super::strategy::coverage::pi_theta_position) 投影到 `𝒦_Θ`
///    ——`gate.force_flat ⟹ {0}`、`stop_long ⟹ 禁净多`、`stop_short ⟹ 禁净空`，加上
///    `cap = γ̄·U_ℓ` 帽。事件门控**不**参与这一步：无事件 bar 上风控照样把目标收窄，平仓单照出。
/// 3. **`order_raw` → `K_Θ_gate` → `Schedule_Θ` 单出口**（§C.2 伪码末两行逐字：
///    `order = K_Θ_gate(order_raw, account, margin, γ̄)` 然后 `schedule(order, exec_index)`）：
///
///    - `order_raw = Σ_ℓ Δq_ℓ = T_lee − T_prev`（**计划态**增量）决定**是否**发单——`==0` ⟹
///      本 bar 无结构意图 ⟹ Hold/Wait。稀疏性由这一步保证：无 tick 且风控/帽未动 ⟹
///      `T_lee == T_prev` ⟹ 恒不发单。
///    - `K_Θ_gate`（账户层可行性）把结构意图投影到**实际持仓**上：发单时物理量
///      `= |T_lee − p_t|`，即 `schedule_order(T_lee, p_t)`。
///
///    **三个锚各司其职，不可合并**（这是本函数最容易写错的地方；★#309 MED-3 补课新增第③行——
///    旧文档只讲了①②，第②步 `pi_theta_position` 的投影**输入**锚同样被换过且未登记，见下）：
///
///    | 问题 | 锚 | 理由 |
///    |---|---|---|
///    | 发不发单 | 计划态 `T_prev` | 稀疏性——拒单后的无事件 bar 不得自动重试（§C.2 `continue`） |
///    | 发多少手 | 实际持仓 `p_t` | 物理可行性——订单作用于真实仓位，不是作用于计划 |
///    | ③ `pi_theta_position` 第二参（投影目标定位） | 计划态 `T_prev`（**非** `p_t`） | 稀疏性——`pi_theta_position` 的 `p_t` 参数同时喂 [`j_theta_key`] 的 `trade_cost=λ\|p−p_t\|`（次键）与 [`feasible_candidates`] 的 `anchor_pt` 候选（「保持现仓」候选点）。若锚**真实**持仓 `p_t`，则 `T_prev≠p_t`（拒单/部分成交后）时无 tick 的 bar 上 `p̃_lee` 虽然=`T_prev`（门控保前值），换手成本却按 `\|p−p_t\|` 而非 `\|p−T_prev\|` 计——若 `p_t≠T_prev` 恰使某个偏离 `T_prev` 的候选点换手成本反而更低，`LexArgmin` 可能选中它，产生 `p*≠T_prev` 从而 `order_raw≠0`，**破坏稀疏性构造**（②③ 步都必须锚 `T_prev` 才能保证「无 tick 且风控/帽未动 ⟹ `T_lee==T_prev` ⟹ 恒不发单」）。 |
///
///    ③ 的直接后果（照实登记，不冒充无副作用）：`trade_cost` 与 `anchor_pt` 候选点在计划态与
///    实际持仓分歧期间是按**虚拟**（计划）位置估的，不是按真实持仓——「保持当前真实持仓不动」
///    这一候选可能不在 [`feasible_candidates`] 代表集内。该分歧的幅度由
///    [`super::super::strategy::level_order::LevelOrderStats::max_abs_plan_fill_gap`]
///    （`Σ_ℓ q_ℓ^plan − p_t`）承载，但旧 3000-bar 验收 fixture 上该缺口恒为 0 ⟹ ③ 这条分歧
///    路径在旧验收上**完全不可观测**——`runner.rs` 的
///    `lee_m3_attribution_dimension_is_readable_and_not_residual_only`（#309 MED-3 补课）换
///    9000-bar fixture 后实测非零，使其从「理论上存在」变为「实测可读」（**不**断言具体数值，
///    只坐实非零，同该字段既有「只登记不判优劣」纪律）。
///
///    若①②都锚计划态（早期实装如此），在 `T_prev ≠ p_t`（拒单/部分成交）**且** `T_lee` 与
///    `T_prev` 反号时，`schedule_order` 的「反号穿零」分支产 `Sell/Buy`（`close_only=false`，
///    见 [`apply_order`]）⟹ 按计划差发量会**超开反向仓**（planned=100/held=37/T_lee=−50 ⟹
///    应开空 50，误发 150 手 ⟹ 净空 −113）。锚 `p_t` 后该路径构造上不可达。
///
/// 返回该订单的级别增量 `Δq_ℓ`（成交侧按此比例落 `held_ℓ`，L1 恒等 `Σ_ℓ held_ℓ ≡ units` 不变）。
///
/// **能力边界（090 反声明膨胀）**：`Σ_ℓ Δq_ℓ ≡ T_lee − T_prev` 是构造性恒等（L0）；订单**物理
/// 量**因 `K_Θ_gate` 锚 `p_t` 而**不**恒等于它——差额 = 计划/成交缺口，由
/// `max_abs_plan_fill_gap` 如实登记，**不**声称订单量由级别增量之和逐手承载。M3 的信息增量在
/// **门控本身**——可证伪的是稀疏性（无结构钟点的 bar 不产结构订单，认识论 **L1**：读数产自
/// 合成 fixture）与门控前后的订单流分叉幅度（**L1**，只登记不判优劣）。**M3 起订单流与 M0
/// 分叉，不得借 M2 的 bit-exact 蒙混**（设计文档 §D M3 逐字）。
/// [`plan_level_gated_order`] 的**账户层投影上下文**（§F③ 风控域的全部输入）。
///
/// 这五项恒同进同出（`pi_theta_position` 的完整签名尾部 + lot 网格），单独传是 Data Clumps；
/// 打包后「风控域输入」在类型层可见——门控（结构域）不得读它们之外的东西，反之亦然。
#[derive(Clone, Copy)]
struct AccountProjectionCtx<'a> {
    /// 当前净持仓（订单物理量的锚，见 `plan_level_gated_order` ③）。
    p_t: f64,
    /// `U_ℓ = NAV/px` 协变资本单位。
    base_units: f64,
    /// `𝒦_Θ` 帽与 lot 网格参数。
    risk: &'a crate::theta_v0::config::RiskConfig,
    /// `J_Θ` 三键权重。
    weights: super::super::strategy::coverage::PiThetaWeights,
    /// 当 bar 风控门真值（`k_theta_risk_gate` 输出，**每 bar 无条件**施加）。
    gate: super::super::strategy::coverage::KThetaRiskGate,
}

/// ★#363（#355 MED-C）M4 级别帽的**逐级** binding 判据：clamp 前后逐位比较，取值被真正改过的
/// 级别（升序，继承输入的 level 升序）。
///
/// [`super::super::strategy::coverage::clamp_levels_to_weighted_cap`] 保序等长（逐级 `map`，零项
/// 保留、残差桶透传），故逐位比较即逐级比较——不需要按 level 归并。
fn levels_narrowed_by_cap(before: &[(u32, i64)], after: &[(u32, i64)]) -> Vec<u32> {
    // ★#369 LOW-1（与下方位序断言同源，一并升为真 `assert!`）：「等长」与「保序」是同一条
    // clamp 契约的两半，失效形状也同形——release 下 `zip` 对长度不等**静默截断**，尾部被裁级别
    // 直接漏出本表 ⟹ 逐级判据反把它计为未解释违例。只升一半 = 半成品（090）。
    //
    // ★#376 LOW-2（措辞自洽）：本条虽是 clamp 的**构造性**契约，仍取真 `assert!` 而非
    // `debug_assert`——判据是「同函数内构造 vs 跨接线可破」（论证见 `plan_level_gated_order` 内
    // #362 MED-B 段），本函数与 clamp 的施加点隔着接线，属后者。
    assert_eq!(before.len(), after.len(), "clamp 保序等长契约破：{before:?} vs {after:?}");
    before
        .iter()
        .zip(after.iter())
        .filter(|(b, a)| {
            // ★#369 LOW-1：级别对齐是 clamp 的**构造性契约**（保序等长），不是需要过滤的数据
            // 情形。写在 `filter` 里 ⟹ 契约一旦破，该级被静默跳过（漏报「被帽裁」⟹ 逐级判据
            // 反而把它计为未解释违例，方向还是错的）。契约破 = 缺陷，应响亮。
            assert_eq!(b.0, a.0, "clamp 保序契约破：位序 level {} vs {}", b.0, a.0);
            b.1 != a.1
        })
        .map(|(b, _)| b.0)
        .collect()
}

/// 两张级别表的并（**去重**，顺序不承诺）。任一侧为空 ⟹ 零分配返回另一侧（default 路径与帽未
/// binding 的常态都走这条）。
///
/// ★#369 LOW-2（照实收窄）：原 doc 承诺「两张**升序**表」——该前置既无校验也无消费者
/// （非空路径统一 `sort_unstable` + `dedup`，与输入是否有序无关；唯一下游
/// [`super::super::strategy::level_order::LevelOrderPlan::cap_narrowed_levels`] 只用
/// `contains`）。按 090「声明与实际一一对应」**删承诺**而非加校验（票体 #365 条 8 二选一）。
///
/// 同理不承诺**输出**升序：两条零分配早退路径原样返回输入，输入无序则输出无序——「输出升序」
/// 恰恰依赖刚被删掉的输入前置，写进 doc 就是把删掉的膨胀换个位置再声明一遍。函数名保留
/// `sorted` 仅指非空路径的实现手段（`sort_unstable` 是 `dedup` 的前置），不是对调用方的承诺。
fn union_sorted_levels(a: Vec<u32>, b: Vec<u32>) -> Vec<u32> {
    if b.is_empty() {
        return a;
    }
    if a.is_empty() {
        return b;
    }
    let mut out = a;
    out.extend(b);
    out.sort_unstable();
    out.dedup();
    out
}

fn plan_level_gated_order(
    level_order: &mut super::super::strategy::level_order::LevelOrderLedger,
    ticks: &super::super::strategy::level_clock::LevelClockTicks,
    sep_legs: &[super::super::strategy::coverage::SepLeg],
    pan_div_child_units: i64,
    qty_m0: i64,
    acct: AccountProjectionCtx<'_>,
    exec_index: usize,
    bar: usize,
    order: &mut Order,
) -> super::super::strategy::level_order::LevelUnits {
    let AccountProjectionCtx { p_t, base_units, risk, weights, gate } = acct;
    let lot = risk.default_lot.max(1) as i64;
    use super::super::strategy::coverage::{pi_theta_position, schedule_order};
    // ① 门控重估（结构域）：clock_ℓ 有 tick 的级别才读本 bar net_ℓ。
    let basis = super::super::strategy::level_ledger::level_nets(sep_legs, lot);
    let ticked_levels = ticks.ticked_levels();
    let gated = level_order.regate(&basis, &ticked_levels);
    // ★M4 级别级风险帽（design doc §D M4；`risk.enforce_level_cap` 默认关，M0–M3 bit-exact
    // 不变）：裁剪后的 `gated` 同时喂下面的 `p_tilde_lee` 与后续 `plan_gated` 的归因基准。
    //
    // ★#351 MED-2 补课（照实订正）：帽在此处**只**裁了进入账户层前的结构基准，不是「不留
    // 一条未裁剪的旁路」——`attribute_total`（`plan_gated` 内部）在 `Σgated ≠ t_lee` 时按比例
    // 缩放**全部**级别（含账户层投影/`pan_div_child_units` 加项造成的放大），缩放后的
    // `targets_ℓ` 可重新突破 `cap_ℓ`（#349 影子评审 MED-2；见
    // `coverage::attribute_total_scaling_can_exceed_cap_and_reclamp_restores_it`）。真正堵住
    // 旁路的是下面 `plan_gated` 之后对 `plan.targets` 的第二次 `clamp_levels_to_weighted_cap`
    // ——两次裁剪合起来才是「不留旁路」：本次防止未裁剪 `net_ℓ` 进account层，下次防止归因缩放
    // 把已裁剪的目标重新推出 cap。`pan_div_child_units` 本身无级别身份、不可能单独裁剪，但它
    // 对各级 `targets_ℓ` 的影响已经过缩放传导，随第二次裁剪一并收回，不再是不受限的旁路。
    // ★#351 MED-3 补课：帽是否真的裁掉了什么，在**此处**（帽施加点本身）直接判定——不能靠
    // 下游 `p_star_lee` 与 `p_tilde_lee` 的差值反推（那个差值只反映账户层投影/风控的效应，
    // 帽已经把 `net_ℓ` 改写进 `p_tilde_lee` 的输入，帽驱动的偏离对那个差值恒不可见，见
    // `DecisionObs::risk_or_cap_active` 施加点）。`cap_narrowed` 是「帽驱动偏离」的单列判据。
    let gated_pre_cap = gated.clone();
    let gated = if risk.enforce_level_cap {
        super::super::strategy::coverage::clamp_levels_to_weighted_cap(&gated, base_units, risk)
    } else {
        gated
    };
    // ★#363（#355 MED-C）帽 binding 的**逐级**判据：clamp 前后逐位比较取被真正裁过的级别集。
    // bar 级 `cap_narrowed`（下方 `risk_or_cap_active` 用）由它派生 ⟹ 两个判据同源，不再是
    // 「bar 级一个口径、逐级另一个口径」的孪生半修（MED-C 的根因形状）。
    let cap_narrowed_levels = levels_narrowed_by_cap(&gated_pre_cap, &gated);
    // 结构净目标 = 各级门控计划之和 + **账户层 pan_div 在飞子腿**。后者是 P7/P9 中枢震荡的
    // 净目标分量，载体是 `pan_div_state` 的 lot 账本而**不是** `sep_legs` ⟹ 不经 `net_ℓ`；
    // M2 时它经 `p_star_final` 改写进目标（`schedule_order(target, p_t)` 两分支），M3 必须
    // 显式接回，否则该子系统在门控后**再也到不了订单出口**（目标来源被删 ≠ 目标重估被门控，
    // 后者才是 M3 的范围）。其生灭时点已是 clock 事件 [`LevelEventKind::PanDivCert`]，
    // 故此加项不破坏稀疏性（默认 config 下 pan_div 惰性 ⟹ 恒 0，bit-exact 无影响）。
    let p_tilde_lee = (gated.iter().map(|&(_, q)| q).sum::<i64>() + pan_div_child_units) as f64;
    // ② 账户层/风控投影（风控域，每 bar 无条件；gate 直接是 k_theta_risk_gate 的当 bar 真值）。
    let t_prev = level_order.planned_total();
    let p_star_lee = pi_theta_position(p_tilde_lee, t_prev as f64, base_units, risk, weights, gate);
    // ★#356 命名订正：这是二次裁剪**前**的账户层投影目标（票体 `t_lee`），不是最终发单量——
    // 二次裁剪后的 `t_lee'` 在下方独立绑定为 `t_lee`（真正喂给 `schedule_order` 的值）。两阶段
    // 值分开命名，避免同名 `t_lee` 被误读为「同一个值」（090 严格性：声明与实际必须一一对应）。
    let t_lee_raw = p_star_lee.round() as i64;
    // ③ order_raw（计划态增量，决定**是否**发单）→ K_Θ_gate（账户层可行性，锚**实际持仓**决定
    //    **发多少手**）→ Schedule_Θ 单出口。两锚分工与「合并即穿仓」的构造见函数文档表。
    let plan = level_order.plan_gated(&gated, t_lee_raw);
    // ★#351 MED-2 补课：归因缩放后二次裁剪（见①处「照实订正」段）——`attribute_total` 可能
    // 把 `plan.targets` 放大回 cap 之上，用同一 `clamp_levels_to_weighted_cap` 再裁一次
    // （该函数对 `LEVEL_ACCOUNT_RESIDUAL` 残差桶安全透传，见其 doc）。未越界时 `reclamped ==
    // plan.targets`，零开销分支不改变任何现有行为（含 `enforce_level_cap=false` 的默认路径）。
    //
    // ★#363（#355 MED-C）判据同步：本段同时把「帽裁到了**哪几级**」写进计划
    // （`cap_narrowed_levels` = ①②两处施加点逐级差集之并），供逐级稀疏性判据
    // （`observe_decision` 的 `n_levels_off_clock_delta_unexplained`）读。#351 只把帽并进了
    // bar 级 `risk_or_cap_active`（下方），逐级解释项仍只有 `plan.rescaled`——而帽不经
    // `attribute_total` 缩放（它直接改写 `gated`/`targets`），`rescaled` 对帽驱动的逐级偏离
    // **结构性不可见**（与 #351 MED-3 对 bar 级判据的诊断同形）。孪生判据必须同源同粒度：
    // 缩放按定义改写全部级别 ⟹ bar 级 bool 足够；帽逐级施加 ⟹ 必须逐级记，否则「某级被裁」
    // 会赦免同决策点全部 off-clock 级别，把逐级判据削回 bar 级鉴别力。
    //
    // `enforce_level_cap=false`（default）⟹ 本 if 整体跳过 ⟹ `plan` 保持 `plan_gated` 的产出
    // （`cap_narrowed_levels` 空表，零分配），判据逐字节退化为修改前的 `!plan.rescaled`。
    let plan = if risk.enforce_level_cap {
        let reclamped = super::super::strategy::coverage::clamp_levels_to_weighted_cap(
            &plan.targets,
            base_units,
            risk,
        );
        let cap_narrowed_levels = union_sorted_levels(
            cap_narrowed_levels,
            levels_narrowed_by_cap(&plan.targets, &reclamped),
        );
        if reclamped == plan.targets {
            super::super::strategy::level_order::LevelOrderPlan { cap_narrowed_levels, ..plan }
        } else {
            let deltas =
                super::super::strategy::level_attrib::sub_levels(&reclamped, level_order.planned());
            let order_units = deltas.iter().map(|&(_, q)| q).sum();
            super::super::strategy::level_order::LevelOrderPlan {
                targets: reclamped,
                deltas,
                order_units,
                cap_narrowed_levels, // MED-3：二次裁剪同样是帽驱动偏离，并入同一判据
                ..plan
            }
        }
    } else {
        plan
    };
    // bar 级判据由逐级判据派生（#363：单一来源）。
    let cap_narrowed = !plan.cap_narrowed_levels.is_empty();
    // ★#356 修复（编排者裁定：下单量走二次裁剪后的 t_lee'，裁剪点统一）：上面①②两次
    // `clamp_levels_to_weighted_cap` 已经把 `plan.targets`（归因账本）裁到位，但发单时若仍用
    // 裁剪**前**的 `t_lee_raw`——账本说「已裁」，`schedule_order` 收到的却是未裁的目标，物理
    // 下单量因此可重新突破 Σcap_ℓ（#356 issue 原句「账本已裁物理未裁」）。`plan.order_units`
    // 是（可能经二次裁剪的）`plan.targets` 相对 `planned()`(=t_prev) 的增量之和，故
    // `t_prev + plan.order_units ≡ Σ_ℓ plan.targets_ℓ` 恒成立（构造性，下方断言坐实）——下单量
    // 与归因账本因此同源同一份裁剪值，不再有第二套口径。`enforce_level_cap=false` 或帽未
    // binding 时 `plan` 未被重写，该式退化为 `t_lee_raw` 本身，M0–M3 bit-exact 不受影响
    // （纯整数运算，无浮点重排）。
    //
    // ★#362 影子评审 MED-2（声明域订正，照实收窄）：由此可得的上界是「Σ_ℓ 落在**真实级别桶**
    // 的目标不越 Σ_ℓ cap_ℓ」，**不是**「物理下单量恒不越 Σcap_ℓ」。反例是残差桶旁路：
    // `attribute_total` 在 `Σgated == 0 ∧ t_lee_raw ≠ 0` 时把全额记入
    // [`super::super::strategy::level_attrib::LEVEL_ACCOUNT_RESIDUAL`]，而
    // `clamp_levels_to_weighted_cap` 对该桶按 doc 原样透传（它无级别身份 ⟹ 无 `cap_ℓ` 可套），
    // 上面的后置护栏也据此豁免它。此路径下物理下单量的唯一上界回落到账户层 `γ̄·U`
    // （`pi_theta_position` 施加），与 Σcap_ℓ 无关。这不是 #356 的回归（#356 修的是「账本已裁
    // 物理未裁」的两套口径，残差桶两边同样透传 ⟹ 仍同源同值），而是原声明的域比事实宽——
    // 此处照实登记为旁路，**不**在本批引入残差桶裁剪语义（那是独立裁定，见 #365 条 2 原文）。
    let t_lee = t_prev + plan.order_units;
    debug_assert_eq!(
        plan.targets.iter().map(|&(_, q)| q).sum::<i64>(),
        t_lee,
        "#356 一致性断言 Σ_ℓ q_ℓ ≡ t_lee' 违例 @bar{bar}: targets={:?} t_lee'={}",
        plan.targets,
        t_lee
    );
    // ★#362 影子评审 MED-B（真 assert 升级）：本护栏与上面的 `Σ_ℓ q_ℓ ≡ t_lee'` 不同——后者
    // 的两端在**同一函数内**由同一表达式构造（`order_units` 按定义就是 `targets − planned` 之
    // 和，紧邻上方几行可逐字核对），无外部接线能破，`debug_assert` 足够；本条「二次裁剪后无级别
    // 越 cap_ℓ」依赖 `clamp_levels_to_weighted_cap` 是最后一个改写 `targets` 的算子这一
    // **接线事实**，任何后续插入的改写都能静默破坏它。按
    // `level_order.rs` 的「恒等证据须 release 非平凡可读」纪律（#289 MED 同源），只在 debug
    // 求值 = release 跑批零执行 = 没有证据 ⟹ 升为真 `assert!`。代价 O(级别数)/bar，可忽略。
    //
    // ★#376 LOW-2（分界措辞订正，照实）：本批实际起作用的分界是「**同函数内构造** vs **跨接线
    // 可破**」，不是「构造性 vs 非构造性」。区别有实效：`levels_narrowed_by_cap` 的等长/保序两条
    // （`fill.rs:543-554`）就其被调用处而言确是 clamp 的构造性契约，但其施加点与消费点隔着接线
    // ⟹ 按本分界归「跨接线可破」，同样升真 `assert!`（#369 LOW-1 的处置据此自洽）。若按旧措辞
    // （构造 ⟹ `debug_assert` 足够）推导，会得出与本批相反的结论。
    assert!(
        !risk.enforce_level_cap
            || plan
                .targets
                .iter()
                .all(|&(lvl, q)| lvl == super::super::strategy::level_attrib::LEVEL_ACCOUNT_RESIDUAL
                    || q.abs()
                        <= super::super::strategy::coverage::level_cap(lvl, base_units, risk)
                            .floor()
                            .max(0.0) as i64),
        "MED-2 二次裁剪后仍有级别突破 cap_ℓ @bar{bar}: {:?}",
        plan.targets
    );
    let p_t_i = p_t.round() as i64;
    let scheduled = if plan.order_units == 0 {
        // 无结构意图 ⟹ 不发单（稀疏性）。持仓判据取**实际**持仓（订单语义面向真实仓位）。
        Order {
            action: if p_t_i != 0 { StrictAction::Hold } else { StrictAction::Wait },
            qty: 0,
            exec_index,
        }
    } else {
        schedule_order(t_lee as f64, p_t, exec_index)
    };
    debug_assert_eq!(
        level_order.held_total(),
        p_t_i,
        "M2/M3 归因完备违例（L1）：Σ_ℓ held_ℓ={} ≠ p_t={} @bar{}",
        level_order.held_total(),
        p_t_i,
        bar
    );
    // 稀疏性的构造性护栏：无结构意图 ⟹ 必不发单（qty==0）。这是「订单时点 ⊆ 事件时点并集」
    // 在单决策点上的局部形式，debug 立即失败 + release 由 `n_orders_off_structural_clock` 累计。
    debug_assert!(
        plan.order_units != 0 || scheduled.qty == 0,
        "M3 稀疏性违例：Σ_ℓ Δq_ℓ==0 却发出 qty={} @bar{}",
        scheduled.qty,
        bar
    );
    // 稀疏性反例的解释项：风控门非全开、或帽/量化使投影目标偏离结构目标（§F③ 合法例外）。
    // 两项分开算：`risk_gate_active` 是**纯风控**求值面证据，`risk_or_cap_active` 额外含帽 binding。
    //
    // ★#351 MED-3 补课：`cap_narrowed`（帽施加前后 `gated`/`targets` 是否真的被裁过，①②两处
    // 施加点各判一次）必须单独并入——`|p_star_lee−p_tilde_lee|` 只反映账户层投影/风控的效应，
    // 帽已经把 `net_ℓ` 改写进 `p_tilde_lee` 的输入，帽驱动的偏离对这个差值恒不可见（#349 MED-3：
    // 「解释项对帽结构性不可见」）。不并入会把帽收紧逼出的 off-clock 订单误计为未解释违例。
    let risk_gate_active = gate.force_flat || gate.stop_long || gate.stop_short;
    let risk_or_cap_active =
        risk_gate_active || cap_narrowed || (p_star_lee - p_tilde_lee).abs() >= 0.5;
    level_order.observe_decision(
        &plan,
        super::super::strategy::level_order::DecisionObs {
            p_t: p_t_i,
            qty_m0,
            has_structural_tick: ticks.has_structural(),
            risk_or_cap_active,
            risk_gate_active,
            ticked: &ticked_levels,
        },
    );
    // 订单 = K_Θ_gate 的物理量（锚 p_t），动作分类沿用 Schedule_Θ 单出口。
    *order = scheduled;
    level_order.commit_planned(&plan.targets);
    plan.deltas
}

pub(super) fn pi_theta_fill_loop_overlay<F>(
    mut classify_at: F,
    bars: &[Bar],
    initial_nav: f64,
    config: &ThetaConfig,
    chi: Option<ChiFilterCtx>,
    mut overlay: Option<&mut super::super::strategy::overlay_state::OverlayState>,
    mut level_ledger: Option<&mut super::super::strategy::level_ledger::LevelLedgerMirror>,
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
    // 费率解析器（#360 单源门面）：未标定档 = 三常数合成率（逐位现状），标定档按 datum 逐笔解析。
    let fees = super::treasury::fee_quoter(&config.exec);
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
    // ★LEE M2（multi-level-native-execution-design-20260719 §D M2）：物理订单的**量**由
    //   `Σ_ℓ Δq_ℓ` 生成的级别归因台账。与 `pending` 严格配对的级别增量队列——同一个 push 分支
    //   同时入两队 ⟹ 下标恒对齐；成交时按该订单的 Δq_ℓ 比例把**实际**成交量落 held_ℓ，
    //   维持 `Σ_ℓ held_ℓ ≡ units`（拒单/部分成交同样不失配）。
    //   M1/M5 的 overlay/镜像是**只读旁路**，本台账不是——它是订单量的生产来源（无 env gate、
    //   无 Option：M2 的契约就是「物理订单改由 Σ_ℓ Δq_ℓ 生成」，旁挂式接法不兑现该契约）。
    let mut level_order = super::super::strategy::level_order::LevelOrderLedger::new();
    let mut pending_attrib: Vec<Vec<super::super::strategy::level_order::LevelUnits>> =
        vec![Vec::new(); n];
    // ★LEE M3（§D M3）：clock_ℓ 钟点的逐决策点累计读数（**release 可见**——「稀疏」若只有定义
    //   没有读数等于没有证据）。稀疏度 = `n_bars_with_structural_tick / n_decisions`。
    let mut level_clock_stats = super::super::strategy::level_clock::LevelClockStats::default();
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
            let attribs = std::mem::take(&mut pending_attrib[i]);
            debug_assert_eq!(
                orders.len(), attribs.len(),
                "M2 归因队列与订单队列同 push 分支入队 ⟹ 长度必相等 @bar{i}"
            );
            for (oi, o) in orders.iter().enumerate() {
                if o.qty > 0 {
                    let units_before = units;
                    // A'（清单③）：累加本 fill 的费后已实现 PnL（多 fill 同 bar 聚合——推导链第 9 条）。
                    let fee_rate = order_fee_rate(&fees, o, px, units);
                    let fill = apply_order(o, px, fee_rate, &mut cash, &mut units, &mut entry_cost, &mut trade_pnls);
                    tw_thread.add_realized(fill.realized);
                    cum_fee += fill.fee;
                    // ★LEE M2 归因落账：**实际**成交有符号手数（拒单/close_only 上限 ⟹ 可小于
                    //   请求量）按该订单的 Δq_ℓ 比例回缩落 held_ℓ ⟹ `Σ_ℓ held_ℓ ≡ units` 逐 fill 保持。
                    //   units 恒为整数手（apply_fill 只按整数 qty/close_qty 增减）⟹ round 无信息损失。
                    let executed_signed = (units - units_before).round() as i64;
                    level_order.on_fill(&attribs[oi], executed_signed);
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
                    VoiceOrderKind::Open => vb.apply_open(vo.voice, vo.side, vo.qty, px, i, &fees),
                    VoiceOrderKind::Close => vb.apply_close(vo.voice, px, i, &fees),
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

        // ── M6 ①⁺ Funding + Borrow 持仓期成本计提（本 bar 成交后持仓的持有成本；venue 口径
        //    #303=spot，两通道语义与基数重叠声明见 `strategy::risk::CostModel` 节头；px>0 才计——
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
            // ── #124 裁定4 TW 谓词 ctx（P2/P3/P4 进 fold）：ShortDiff 腿 id 从 typed ledger
            //    在飞表取（entry_v 入场固定，与 TW open_legacy_legs 计数同源）；risk_mode 从
            //    strategy::risk 五态投影到 closed_loop::state 五态（两枚举同锚 Origin 五构造子，
            //    此处只读逐变体映射，非第二权威源——判定仍单源 k_theta_risk_gate）。 ──
            let shortdiff_ids: std::collections::HashSet<classifier::recursive_tower::ElementId> =
                open_trades
                    .iter()
                    .filter(|(_, o)| o.entry_v == super::super::strategy::coverage::Vertical::ShortDiff)
                    .map(|(id, _)| *id)
                    .collect();
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
                shortdiff_leg_ids: &shortdiff_ids,
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
            // ★LEE M2：物理净目标 p\* 的单一读点。无 pan_div 覆盖 ⟹ 恒 = `standard_p_star`
            //   （`pi_theta_step_traced` 的 `LexArgmin_{p∈𝒦_Θ}J_x(p)`）；pan_div 覆盖订单时同步
            //   改写（下方两处 `schedule_order(target, …)` 分支）——保证 M2 归因读到的目标与
            //   净额 `Schedule_Θ` 用的目标是同一个（禁第二裁决源）。
            let mut p_star_final = standard_p_star;
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
                        p_star_final = target; // ★LEE M2：目标改写同步（禁第二裁决源）
                        order = coverage::schedule_order(target, p_t, exec_index.unwrap_or(i));
                    }
                } else if !higher_priority && pan_div_state.signed_live_child_units() != 0 {
                    // 无新 cert 的持有 bar：把 live ShortDiff 投影叠回标准目标，避免下一 bar 被基础 π
                    // 当作偏差自动回补；仍经同一 KΘ 区间 clamp 和 ScheduleΘ 单一订单出口。
                    let target = gate.clamp_position(
                        cap,
                        standard_p_star + pan_div_state.signed_live_child_units() as f64,
                    );
                    p_star_final = target; // ★LEE M2：目标改写同步（禁第二裁决源）
                    order = coverage::schedule_order(target, p_t, exec_index.unwrap_or(i));
                }
            }
            // ── ★★LEE M3 clock_ℓ 事件钟（§C.2 变化部分① / §D M3）：本 bar 各级事件集。 ──
            //    七通道全部取自**已有**产出（禁第二查法、禁重算结构）：新确认 BSP 走
            //    `classification_step`（`newly_confirmed_step` 的 append-only diff），腿生命周期
            //    五通道走 `step_trace`，盘整背驰走本 bar 首见并过门的 `pan_candidates`。
            //    事件集的最小完备定义、与 `LevelState` 的逐字段对齐、以及照实登记的三项缺口
            //    （段完成/中枢生灭不可观测、BSP「灭」无载体、五钟只实装 1/5）见
            //    `strategy::level_clock` 模块头——**不在此重复**（单源）。
            let clock_ticks = {
                use super::super::strategy::level_clock::collect_ticks;
                let bsp_levels: Vec<u32> = classification_step
                    .levels
                    .iter()
                    .enumerate()
                    .filter(|(_, ls)| !ls.bsp.is_empty())
                    .map(|(lvl, _)| lvl as u32)
                    .collect();
                let closed_levels: Vec<u32> =
                    step_trace.closed.iter().map(|(leg, _)| leg.level).collect();
                let opened_levels: Vec<u32> =
                    step_trace.opened.iter().map(|(_, leg)| leg.level).collect();
                let silent_levels: Vec<u32> =
                    step_trace.silent_drops.iter().map(|leg| leg.level).collect();
                let overlay_levels: Vec<u32> =
                    step_trace.overlay_closes.iter().map(|leg| leg.level).collect();
                let risk_levels: Vec<u32> =
                    step_trace.risk_exits.iter().map(|leg| leg.level).collect();
                let pan_levels: Vec<u32> = pan_candidates.iter().map(|c| c.level()).collect();
                collect_ticks(
                    &bsp_levels,
                    &closed_levels,
                    &opened_levels,
                    &silent_levels,
                    &overlay_levels,
                    &risk_levels,
                    &pan_levels,
                )
            };
            level_clock_stats.observe(&clock_ticks);
            // ★★LEE M3 事件门控订单（§D M3）：目标只在 clock_ℓ 时点重估，风控每 bar 无条件生效。
            //    三段（门控重估 / 账户层-风控投影 / Schedule_Θ 单出口）与能力边界见
            //    [`plan_level_gated_order`] 与 `strategy::level_order` 模块头——**不在此重复**。
            //    `p_star_final`（M0/M2 的每 bar 净额目标）在 M3 **不再**决定订单量，只作分叉幅度
            //    读数的对照量（`qty_m0`）——M3 起订单流与 M0 分叉是契约本身。M4（级别 sizing
            //    w_ℓ）本步不做。
            //    `p_star_final`（M0/M2 的每 bar 净额目标）在 M3 **不再**决定订单量，只作分叉幅度
            //    读数的对照量 `qty_m0`——其 pan_div 改写分量则经 `pan_div_child_units` 显式接回
            //    结构目标（见 `plan_level_gated_order` ①，目标来源不得被门控删掉）。
            let qty_m0 = coverage::schedule_order(p_star_final, p_t, exec_index.unwrap_or(i)).qty;
            let pan_div_child_units = if pan_div_hist.is_some() {
                pan_div_state.signed_live_child_units() as i64
            } else {
                0 // pan_div 未启用 ⟹ 无在飞子腿（bit-exact：加项恒 0）
            };
            let order_attrib = plan_level_gated_order(
                &mut level_order,
                &clock_ticks,
                &step_trace.sep_legs,
                pan_div_child_units,
                qty_m0,
                AccountProjectionCtx {
                    p_t,
                    base_units,
                    risk: &config.risk,
                    weights,
                    gate,
                },
                exec_index.unwrap_or(i),
                i,
                &mut order,
            );
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
            // ── ★LEE M1 级别账本镜像步进（multi-level-native-execution-design-20260719 §D M1）：
            //    同一决策点同一 sep_legs 按 id.level≡formation_level 分桶重放（与 overlay 并列的
            //    只读旁路，不改净额 fill 任何状态）。level_ledger=None ⟹ 整段跳过（bit-exact 回归锁）。 ──
            if let Some(ll) = level_ledger.as_deref_mut() {
                let lstep = ll.step(&step_trace.sep_legs, px, i, config.risk.default_lot.max(1) as i64);
                // ★LEE-Net 恒等（M1 验收断言，设计文档 §C.2）：Σ_ℓ net_ℓ 恒 = overlay 净敞口 N——
                // 逐级分解是 Net 的加性细化（整数手数求和，精确成立非 eps 容差）。
                if let Some(ov) = overlay.as_deref() {
                    debug_assert_eq!(
                        lstep.total_net,
                        ov.net(),
                        "LEE-Net 恒等违例（M1 加性细化）：Σ_ℓ net_ℓ={} ≠ N={} @bar{}",
                        lstep.total_net, ov.net(), i
                    );
                    // ★#289 影子评审 MED ①：恒等的 **release 非平凡证据**——上面的
                    //   debug_assert 在 release 跑批里零执行，integration 断言又取 force_flat 之后
                    //   （两账均归零 ⟹ 平凡通过）。本读数逐决策点累计 max|Σ_ℓ net_ℓ − N| 与
                    //   max|N|，release 同样执行；「残差 0 且 max|N|>0」才算见证成立。
                    let overlay_net = ov.net();
                    ll.observe_lee_net(overlay_net);
                }
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
            // 反向关闭：typed 判据单源 reverse_exit_type（入场角色 + 触发候选类）。
            for (leg, trig) in &step_trace.closed {
                if let Some(open) = open_trades.remove(&leg.id) {
                    if open.entry_v == super::super::strategy::coverage::Vertical::ShortDiff {
                        tw_thread.close_share_leg(); // TW 腿计数（#124）
                    }
                    let pushed = TypedTrade {
                        entry_z: open.entry_z,
                        voice_id: leg.id,
                        entry_bar: open.entry_bar,
                        exit_bar: i,
                        exit_type: interp::reverse_exit_type(open.entry_v, trig.bsp_class),
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
                    if open.entry_v == Vertical::ShortDiff {
                        tw_thread.close_share_leg(); // TW 腿计数（#124）
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
                    if open.entry_v == super::super::strategy::coverage::Vertical::ShortDiff {
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
                }
            }
            // P2 CloseOverlay（#124 裁定4，PDF §7 C_2）：TW StageII 重叠腿关闭——真实订单已经
            // 同一 schedule/fill（组合层合成 close 桶复用 𝒟_x 通道）；typed 归 CloseShortDiff
            // （关的正是 legacy ShortDiff 重叠腿，PDF §9 五枚举内最近语义）。
            for leg in &step_trace.overlay_closes {
                if let Some(open) = open_trades.remove(&leg.id) {
                    if open.entry_v == super::super::strategy::coverage::Vertical::ShortDiff {
                        tw_thread.close_share_leg();
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
                if c.role.v == super::super::strategy::coverage::Vertical::ShortDiff {
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
                        // ★LEE M2：订单与其级别增量 Δq_ℓ 同分支入队 ⟹ 成交侧下标恒对齐。
                        pending_attrib[ei].push(order_attrib);
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
                            .map(|(l, _)| l)
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

    // ── 窗口终点强平锚：末可交易 bar 与其收盘价（#289 LOW-1 单源化——overlay 与 LEE M1 镜像
    //    两处曾逐字重复同一 `last_i`/`last_px` 推导，任一侧口径改动会静默漂移。单源后
    //    「同价同 bar」由构造保证，不再靠两段代码碰巧一致）。 ──
    let forced_flat_anchor = (0..n)
        .rev()
        .find(|&j| !bars[j].untradable && bars[j].close > 0)
        .map(|last_i| (last_i, bars[last_i].close as f64 * config.tick.tick_size));
    // ── ★M5 overlay 窗口终点强平：全部活动声部按末可交易 bar close 离场（记 exit_v/冻结 pnl_v）。
    //    价格 PnL 已在末决策点 step 累计到末价 ⟹ 此处只搬账本行（含浮盈口径，与净额侧同理）。 ──
    if let Some(ov) = overlay.as_deref_mut() {
        if let Some((last_i, last_px)) = forced_flat_anchor {
            ov.force_flat(last_px, last_i);
        }
    }
    // ── ★LEE M1 镜像窗口终点强平：与 overlay 同价同 bar 按级重排（各级簿清空、closed 分区落账）。 ──
    if let Some(ll) = level_ledger.as_deref_mut() {
        if let Some((last_i, last_px)) = forced_flat_anchor {
            ll.force_flat(last_px, last_i);
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
                for cinfo in vb.settle_forced_virtual(last_px, exit_bar, &fees) {
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
                let fee_rate = forced_flatten_fee_rate(&fees, units, last_px);
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
        tw_final: Some(tw_thread.finish()),
        r_decomp: Some(r_decomp),
        level_order: level_order.stats(),
        level_clock: level_clock_stats,
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
    /// TW 账本终态（#124 裁定4：TwState 生产真值源在 π fill loop；TStage/ηBucket「生产者已
    /// 就位」的可观测物证——G3 ZExt 桥/诊断消费）。v1 路径无 TW 接线 ⟹ `None`（诚实不伪造）。
    pub(super) tw_final: Option<TwState>,
    /// M6 R 分解表（路线.pdf p16 第十一关：R=ΣN_tΔP_t−Commission−Slippage−Funding−Borrow−
    /// LiquidationLoss + 账目守恒残差）。π 路径 [`pi_theta_fill_loop`] 产出；v1 路径
    /// [`plan_and_fill_mtm`] 未接 R 分解 ⟹ `None`（诚实不伪造，v1 是 recognize 旧路径非生产 π）。
    pub(super) r_decomp: Option<super::super::strategy::risk::RDecomposition>,
    /// ★LEE M2 级别归因见证读数（`Σ_ℓ Δq_ℓ ≡ ΔN` 的 **release 可见**证据）。π 路径
    /// [`pi_theta_fill_loop_overlay`] 产真值；v1 路径 [`plan_and_fill_mtm`] 无级别归因台账
    /// ⟹ 全零默认（`identity_witnessed()==false`，诚实不伪造——零决策不是恒等成立的证据）。
    pub(super) level_order: super::super::strategy::level_order::LevelOrderStats,
    /// ★LEE M3 clock_ℓ 钟点见证读数（稀疏度的 **release 可见**证据）。π 路径
    /// [`pi_theta_fill_loop_overlay`] 产真值；v1 / 关⑤双账本路径无事件钟 ⟹ 全零默认
    /// （`sparsity_witnessed()==false`，诚实不伪造——零决策不是稀疏性成立的证据）。
    pub(super) level_clock: super::super::strategy::level_clock::LevelClockStats,
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
    // 费率解析器（#360 单源门面）：未标定档 = 三常数合成率（逐位现状），标定档按 datum 逐笔解析。
    let fees = super::treasury::fee_quoter(&config.exec);

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
                    let fee_rate = order_fee_rate(&fees, o, px, units);
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
                    let fee_rate = order_fee_rate(&fees, o, px, units);
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
                let fee_rate = forced_flatten_fee_rate(&fees, units, last_px);
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
        // ★LEE M2：v1 recognize 路径无级别归因台账 ⟹ 全零默认（`identity_witnessed()==false`，
        // 诚实不伪造——零决策不构成恒等成立的证据，同 typed_ledger/tw_final 的诚实空口径）。
        level_order: Default::default(),
        // ★LEE M3：同理无事件钟 ⟹ 全零（`sparsity_witnessed()==false`）。
        level_clock: Default::default(),
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
/// - **legacy 腿计数**（#124 同语义）：ShortDiff 角色（depth>0 子声部）开仓派
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
        cascade_exit_decisions, exit_decision_for_nested, parent_invalid_at,
    };
    use super::super::strategy::risk;
    use super::super::strategy::voice::VoiceSide;
    use super::super::strategy::LegOrder;

    let n = bars.len();
    let nav0 = if initial_nav > 0.0 { initial_nav } else { 1.0 };
    // 费率解析器（#360 单源门面）：未标定档 = 三常数合成率（逐位现状），标定档按 datum 逐笔解析。
    let fees = super::treasury::fee_quoter(&config.exec);

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
                    let fee_rate = leg_fee_rate(&fees, lo, px, &ledger);
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
                        // #68② TW 腿计数（#124 同语义）：ShortDiff 子声部全平 ⟹ CloseShareLeg(0)。
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
                    let fee_rate = leg_fee_rate(&fees, lo, px, &ledger);
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
                        // #68② TW 腿计数（#124 同语义，:2196-2204 镜像）：ShortDiff 角色
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
                        // #68② TW 腿计数：ShortDiff 子声部全平 ⟹ CloseShareLeg(0)。
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
                let fee_rate = leg_fee_rate(&fees, &lo, last_px, &ledger);
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
                let fee_rate = leg_fee_rate(&fees, &lo, last_px, &ledger);
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
            tw_final: Some(tw_thread.finish()),       // #68② TW 已接线（ShortDiff 成本划转 + Realize 平仓入账 + ShortDiff 腿计数；快照取强平前，强平 PnL 不入 TW）
            r_decomp: None,
            // ★LEE M2：关⑤双账本路径无级别归因台账 ⟹ 全零默认（诚实不伪造）。
            level_order: Default::default(),
            // ★LEE M3：同理无事件钟 ⟹ 全零默认（诚实不伪造）。
            level_clock: Default::default(),
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

#[cfg(test)]
mod level_cap_reclamp_tests {
    use super::*;
    use super::super::super::classifier::recursive_tower::ElementId;
    use super::super::super::config::RiskConfig;
    use super::super::super::strategy::coverage::{KThetaRiskGate, PiThetaWeights, SepLeg, Vertical};
    use super::super::super::strategy::level_clock::{LevelClockTicks, LevelEventKind};
    use super::super::super::strategy::level_order::LevelOrderLedger;
    use super::super::super::strategy::voice::VoiceSide;

    /// ★#356 端到端红绿（`plan_level_gated_order` 是生产订单出口，非纯函数——本测直接调用它，
    /// 校验它的副作用：归因台账 + 输出 `Order`）：`t_lee` 二次裁剪前，下单量仍走裁剪**前**的值
    /// ⟹ 物理下单量可重越 Σcap_ℓ（账本已裁、物理未裁，issue #356 原句）；接线后下单量与账本
    /// 同源，二者恒相等；且在本场景（**全额落真实级别桶**）下不越 Σcap_ℓ。
    ///
    /// ★#362 MED-2 声明域：「不越 Σcap_ℓ」只对真实级别桶成立——残差桶路径
    /// （`Σgated=0 ∧ t_lee_raw≠0`）下全额原样透传，上界回落到账户层 `γ̄·U`。本测不覆盖那条
    /// 旁路（照实登记在 `plan_level_gated_order` 的 `t_lee` 绑定处注释）。
    ///
    /// 场景复刻 `coverage.rs::attribute_total_scaling_can_exceed_cap_and_reclamp_restores_it`
    /// 的「60→100」缩放突破：`level_weights=[0.5,0.1]` ⟹ `cap_0=50/cap_1=10`（`base_units=100`），
    /// 两级 `sep_legs` 净额恰好压在 cap 上（Σ=60），账户层 `pan_div_child_units=40` 把 `p̃_lee`
    /// 推到 100（模拟 pan_div 加项/lex 重估使 `T_lee≠Σgated`，同 fill.rs ①处注释场景）——
    /// `pi_theta_position` 在 `gamma=1.0·base_units=100` 的账户层 cap 上恰好把 `t_lee` 顶到
    /// 100，`attribute_total` 按比例把两级放大到 Σ=100，level0 被推到 >50 突破 `cap_0`。
    #[test]
    fn physical_order_shares_same_reclamped_target_as_ledger() {
        let risk = RiskConfig {
            level_weights: vec![0.5, 0.1],
            enforce_level_cap: true,
            ..RiskConfig::default()
        };
        let base_units = 100.0; // cap_0=50, cap_1=10 ⟹ Σcap_ℓ=60
        let weights = PiThetaWeights::from_risk(&risk);
        let gate = KThetaRiskGate::open();

        let sep_legs = vec![
            SepLeg {
                id: ElementId { level: 0, ordinal: 0 },
                side: VoiceSide::Long,
                q_units: 50.0,
                role_v: Vertical::FollowParent,
                parent_id: None,
            },
            SepLeg {
                id: ElementId { level: 1, ordinal: 0 },
                side: VoiceSide::Long,
                q_units: 10.0,
                role_v: Vertical::FollowParent,
                parent_id: None,
            },
        ];
        let mut ticks = LevelClockTicks::empty();
        ticks.insert(0, LevelEventKind::BspConfirmed);
        ticks.insert(1, LevelEventKind::BspConfirmed);

        let mut level_order = LevelOrderLedger::new();
        let mut order = Order { action: StrictAction::Wait, qty: 0, exec_index: 0 };
        let acct = AccountProjectionCtx { p_t: 0.0, base_units, risk: &risk, weights, gate };
        plan_level_gated_order(
            &mut level_order,
            &ticks,
            &sep_legs,
            40, // pan_div_child_units：把账户层目标从 Σgated=60 推到 100（重越场景构造）
            0,  // qty_m0（仅进归因统计 max_abs_order_residual，不影响本测断言）
            acct,
            0,
            0,
            &mut order,
        );

        let cap_0 = 50i64;
        let cap_1 = 10i64;
        let targets_sum: i64 = level_order.planned().iter().map(|&(_, q)| q).sum();
        // ★#362 LOW-2：per-level cap 用**生产** `level_cap()` 取，不手写 `if lvl==0 {..} else {..}`
        // ——后者把 lvl≥2 与残差桶（`LEVEL_ACCOUNT_RESIDUAL`，u32::MAX 量级）一并罩进 cap_1=10，
        // 既是错的映射又会在场景漂移出两级时给出假绿/假红。残差桶无级别身份 ⟹ 无 cap 可套，
        // 显式跳过（与生产后置护栏同一豁免口径）。
        for &(lvl, q) in level_order.planned() {
            if lvl == crate::theta_v0::strategy::level_attrib::LEVEL_ACCOUNT_RESIDUAL {
                continue;
            }
            let cap = crate::theta_v0::strategy::coverage::level_cap(lvl, base_units, &risk)
                .floor()
                .max(0.0) as i64;
            assert!(q.abs() <= cap, "前置：归因账本已裁到位 level{lvl}={q} 应 ≤ cap={cap}");
        }
        // ★#362 MED-4：钉死当前真值而非 `< 100` 的松代理——本场景两级都被裁到各自 cap 上
        // （50+10），账本合计恒 = Σcap_ℓ = 60。写成 `< 100` 时，上游任何把合计压到 0 的漂移
        // （例如门控/归因回归使 `gated` 全零）都会平凡满足，测试静默失效；钉死真值 ⟹ 漂移即红。
        assert_eq!(
            targets_sum,
            cap_0 + cap_1,
            "前置：本场景须真实触发重越缩放且二次裁剪把两级都压回各自 cap（50+10=60，从未裁的 \
             100 收窄），targets_sum={targets_sum}"
        );

        let signed_qty = match order.action {
            StrictAction::Buy | StrictAction::Add => order.qty as i64,
            StrictAction::Sell | StrictAction::Reduce => -(order.qty as i64),
            _ => 0,
        };
        let physical_position_after = signed_qty; // p_t=0 起点，Δ即终态净仓
        // 绿（#356 修复核心）：物理下单量与归因账本同源——不再是两套裁剪值。
        assert_eq!(
            physical_position_after, targets_sum,
            "物理下单量（p_t+Δ={physical_position_after}）应与账本合计 targets_sum={targets_sum} \
             同源同值；修复前二者分叉（物理端仍是未裁的 100）"
        );
        // 绿：物理下单量不越 Σcap_ℓ（票体验收字面）。
        assert!(
            physical_position_after <= cap_0 + cap_1,
            "物理下单量 {physical_position_after} 突破 Σcap_ℓ={}",
            cap_0 + cap_1
        );
    }

    /// 建两级 `sep_legs`（level0/level1 各一条多头腿，深度角色中性）。
    fn two_level_legs(q0: f64, q1: f64) -> Vec<SepLeg> {
        vec![
            SepLeg {
                id: ElementId { level: 0, ordinal: 0 },
                side: VoiceSide::Long,
                q_units: q0,
                role_v: Vertical::FollowParent,
                parent_id: None,
            },
            SepLeg {
                id: ElementId { level: 1, ordinal: 0 },
                side: VoiceSide::Long,
                q_units: q1,
                role_v: Vertical::FollowParent,
                parent_id: None,
            },
        ]
    }

    /// ★★#363（#355 MED-C）端到端红绿：**逐级**稀疏性判据的帽驱动解释项。
    ///
    /// #351 把 `cap_narrowed` 只并入了 **bar 级** `risk_or_cap_active`；**逐级**
    /// [`LevelOrderStats::per_level_sparsity_has_no_unexplained_violation`] 的解释项仍只有
    /// `plan.rescaled`（`level_order.rs::observe_decision`）。孪生判据只修了一半 ⟹ 帽驱动的
    /// 逐级 `Δq_ℓ≠0` 被记为**未解释违例**。
    ///
    /// 场景（两决策点，直接调生产出口 `plan_level_gated_order`，非纯函数）：
    /// - t0：两级 tick，`base_units=100` ⟹ `cap_0=50/cap_1=10`，`Σgated=60` 恰压在帽上（帽不
    ///   binding）⟹ 建仓 60，`planned=[(0,50),(1,10)]`；
    /// - t1：**无任何 tick**（`gated` 取前值），`base_units` 腰斩到 50 ⟹ `cap_0=25/cap_1=5`
    ///   ⟹ 帽在 `regate` 后的施加点真实裁剪（`cap_narrowed=true`），`Σgated=30`；账户层投影
    ///   `t_lee_raw==Σgated` ⟹ `attribute_total` 走**恒等分支** ⟹ `plan.rescaled=false`。
    ///   两级都无 tick 而 `Δq_ℓ≠0`（50→25、10→5）——修前唯一解释项 `rescaled` 为假 ⟹
    ///   `n_levels_off_clock_delta_unexplained=2`，逐级判据破（红）；修后 `plan.cap_narrowed`
    ///   同为解释项 ⟹ 判据成立（绿）。
    ///
    /// `base_units = NAV/px` 逐 bar 漂移是生产常态（见 `level_clock` 模块头对齐表末行），故
    /// 「无结构事件而帽随资本单位收紧」不是构造出来的人工场景，而是 cap-on 下的常规路径。
    #[test]
    fn cap_narrowed_explains_per_level_off_clock_delta() {
        let risk = RiskConfig {
            level_weights: vec![0.5, 0.1],
            enforce_level_cap: true,
            ..RiskConfig::default()
        };
        let weights = PiThetaWeights::from_risk(&risk);
        let gate = KThetaRiskGate::open();
        let sep_legs = two_level_legs(50.0, 10.0);

        let mut ticks = LevelClockTicks::empty();
        ticks.insert(0, LevelEventKind::BspConfirmed);
        ticks.insert(1, LevelEventKind::BspConfirmed);

        let mut level_order = LevelOrderLedger::new();
        let mut order = Order { action: StrictAction::Wait, qty: 0, exec_index: 0 };
        // t0：帽不 binding 的建仓（base_units=100 ⟹ cap_0=50/cap_1=10，Σgated=60 恰在帽内）。
        let d0 = plan_level_gated_order(
            &mut level_order,
            &ticks,
            &sep_legs,
            0,
            0,
            AccountProjectionCtx { p_t: 0.0, base_units: 100.0, risk: &risk, weights, gate },
            0,
            0,
            &mut order,
        );
        let filled0: i64 = d0.iter().map(|&(_, q)| q).sum();
        level_order.on_fill(&d0, filled0); // 全额成交 ⟹ held 与 p_t 同步（归因完备护栏前提）
        assert_eq!(
            level_order.planned(),
            &[(0u32, 50i64), (1, 10)],
            "t0 前置：帽未 binding ⟹ 两级目标 = 结构净额"
        );
        let s0 = level_order.stats();
        assert_eq!(
            s0.n_levels_off_clock_delta, 0,
            "t0 前置：两级均有 tick ⟹ 无 off-clock 增量"
        );

        // t1：无任何 tick + base_units 腰斩 ⟹ 帽在 regate 后施加点真实裁剪。
        let no_ticks = LevelClockTicks::empty();
        plan_level_gated_order(
            &mut level_order,
            &no_ticks,
            &sep_legs,
            0,
            0,
            AccountProjectionCtx {
                p_t: filled0 as f64,
                base_units: 50.0,
                risk: &risk,
                weights,
                gate,
            },
            1,
            1,
            &mut order,
        );
        let s = level_order.stats();
        eprintln!(
            "MED_C off_clock={} unexplained={} n_rescaled={} planned={:?}",
            s.n_levels_off_clock_delta,
            s.n_levels_off_clock_delta_unexplained,
            s.n_rescaled,
            level_order.planned()
        );
        // 非平凡前置①：本决策点确有「无 tick 却 Δq_ℓ≠0」的级别（否则判据平凡通过）。
        assert!(
            s.n_levels_off_clock_delta > 0,
            "非平凡前置：t1 须产生 off-clock 逐级增量（帽收紧把两级目标从 50/10 压到 25/5）"
        );
        // 非平凡前置②：这些增量**不**来自 `attribute_total` 比例缩放——`rescaled` 恒假 ⟹ 修前
        // 它们全部落进 unexplained（若此处 rescaled 为真，本测将退化为平凡绿，红点被绕开）。
        assert_eq!(
            s.n_rescaled, 0,
            "非平凡前置：t1 走 attribute_total 恒等分支（rescaled=false），红点才在帽解释项上"
        );
        // 非平凡前置③：帽真的 binding 过（否则「无未解释违例」可能只是帽从未裁剪的平凡通过）。
        assert_eq!(s.n_cap_narrowed, 1, "非平凡前置：恰 t1 一个决策点的帽 binding");
        // 绿（#363 修复核心）：帽驱动的逐级偏离进解释项 ⟹ 逐级稀疏性无未解释违例。
        assert!(
            s.per_level_sparsity_has_no_unexplained_violation(),
            "逐级稀疏性硬约束破：off_clock={} 中有 {} 个未被帽解释的违例（#355 MED-C：`cap_narrowed` \
             只并入 bar 级 `risk_or_cap_active`，逐级解释项仍只有 `plan.rescaled`）",
            s.n_levels_off_clock_delta,
            s.n_levels_off_clock_delta_unexplained
        );
    }

    /// ★★#363 解释项的**粒度**（两轴评审 MED：票体原文是「**该级**被二次裁剪」）：帽解释项按
    /// 级别逐个判，同决策点内某级被帽裁**不**赦免另一级的 off-clock 增量。
    ///
    /// 判据层直测（喂手造 `LevelOrderPlan` 给 `observe_decision`）：两级都无 tick 且 `Δq_ℓ≠0`，
    /// 只有 level0 在 `cap_narrowed_levels` 内 ⟹ 未解释违例恰 1（level1），逐级谓词为假。
    /// 若解释项退回 plan 级 bool（本票初版形状），此处会是 0 ⟹ 判据被削回 bar 级鉴别力，
    /// 真实违例被帽的存在顺带赦免。
    ///
    /// 为什么必须直测判据层：在当前生产路径上二者对计数**恰好等价**——`rescaled=false` ⟺
    /// `attribute_total` 恒等分支 ⟺ `targets == gated`，此时无 tick 级别的
    /// `Δq_ℓ = gated_ℓ − planned_ℓ ≠ 0` ⟺ 该级被帽裁过，故「非零 off-clock 增量」与「该级在
    /// 差集内」一一对应。等价性依赖那条构造性事实，判据本身不该依赖它（一旦上游新增改写
    /// `targets` 的路径，plan 级 bool 就开始误赦免）。本测把逐级语义钉死在判据层。
    #[test]
    fn per_level_explanation_is_level_scoped_not_plan_scoped() {
        use super::super::super::strategy::level_order::{DecisionObs, LevelOrderPlan};
        use std::collections::BTreeSet;

        let mut led = LevelOrderLedger::new();
        led.commit_planned(&[(0u32, 50i64), (1, 10)]); // 前一决策点已提交的目标
        let plan = LevelOrderPlan {
            targets: vec![(0, 25), (1, 5)],
            deltas: vec![(0, -25), (1, -5)],
            order_units: -30,
            used_residual_bucket: false,
            rescaled: false,                 // 恒等分支：缩放不构成解释
            cap_narrowed_levels: vec![0],    // 帽只裁了 level0
            struct_gap: 0,
        };
        led.observe_decision(
            &plan,
            DecisionObs {
                p_t: 60,
                qty_m0: 0,
                has_structural_tick: false,
                risk_or_cap_active: true,
                risk_gate_active: false,
                ticked: &BTreeSet::new(), // 无任何 tick ⟹ 两级都是 off-clock
            },
        );
        let s = led.stats();
        assert_eq!(s.n_levels_off_clock_delta, 2, "两级都无 tick 且 Δq_ℓ≠0");
        assert_eq!(
            s.n_levels_off_clock_delta_unexplained, 1,
            "只有被帽裁过的 level0 被解释；level1 是真实违例（plan 级 bool 会误记为 0）"
        );
        assert!(
            !s.per_level_sparsity_has_no_unexplained_violation(),
            "存在未被任何解释项覆盖的逐级违例时，谓词必须为假"
        );
    }

    /// ★#363 default 路径零行为变化（票体任务③）：`enforce_level_cap=false`（`RiskConfig`
    /// default）下，帽的两处施加点都不执行 ⟹ `plan.cap_narrowed` 恒假 ⟹ 逐级判据
    /// `!rescaled && !cap_narrowed` **逐字节**退化为修改前的 `!rescaled`，`n_cap_narrowed` 恒 0。
    ///
    /// 场景与 [`cap_narrowed_explains_per_level_off_clock_delta`] 逐字相同（同样的两决策点、
    /// 同样的 `base_units` 腰斩），只改帽开关。**照实记录**：t1 的目标在 default 下同样收窄
    /// （50/10 → 42/8），但那是**账户层总 cap**（`feasible_net_cap·base_units=50 < Σgated=60`）
    /// 经 `attribute_total` 比例缩放的结果——`rescaled=true`，与级别帽无关。两条路径因此干净
    /// 分离：cap-on 走 `cap_narrowed`（上一测，`rescaled=0`），cap-off 走 `rescaled`（本测，
    /// `n_cap_narrowed=0`），逐级判据在 default 下的取值**只由 `rescaled` 决定**，与本票修改
    /// 前逐字同值。
    #[test]
    fn default_no_cap_path_is_unchanged() {
        let risk = RiskConfig::default(); // enforce_level_cap=false
        assert!(!risk.enforce_level_cap, "前置：default 关帽");
        let weights = PiThetaWeights::from_risk(&risk);
        let gate = KThetaRiskGate::open();
        let sep_legs = two_level_legs(50.0, 10.0);
        let mut ticks = LevelClockTicks::empty();
        ticks.insert(0, LevelEventKind::BspConfirmed);
        ticks.insert(1, LevelEventKind::BspConfirmed);

        let mut level_order = LevelOrderLedger::new();
        let mut order = Order { action: StrictAction::Wait, qty: 0, exec_index: 0 };
        let d0 = plan_level_gated_order(
            &mut level_order,
            &ticks,
            &sep_legs,
            0,
            0,
            AccountProjectionCtx { p_t: 0.0, base_units: 100.0, risk: &risk, weights, gate },
            0,
            0,
            &mut order,
        );
        let filled0: i64 = d0.iter().map(|&(_, q)| q).sum();
        level_order.on_fill(&d0, filled0);
        let no_ticks = LevelClockTicks::empty();
        plan_level_gated_order(
            &mut level_order,
            &no_ticks,
            &sep_legs,
            0,
            0,
            AccountProjectionCtx {
                p_t: filled0 as f64,
                base_units: 50.0, // 同上一测的资本单位腰斩：关帽 ⟹ 对目标零影响
                risk: &risk,
                weights,
                gate,
            },
            1,
            1,
            &mut order,
        );
        let s = level_order.stats();
        assert_eq!(s.n_cap_narrowed, 0, "default 路径不得触达任何级别帽施加点");
        assert_eq!(
            level_order.planned(),
            &[(0u32, 42i64), (1, 8)],
            "default 路径：收窄来自账户层总 cap（50<Σgated=60）+ 最大余数法归因，非级别帽"
        );
        assert_eq!(s.n_rescaled, 1, "default 路径：该收窄经 attribute_total 比例缩放登记");
        assert!(
            s.per_level_sparsity_has_no_unexplained_violation(),
            "default 路径的逐级判据只由 rescaled 解释（与本票修改前逐字同值）"
        );
    }

    /// ★#363 第二施加点见证（票体任务①字面「二次裁剪路径」）：归因缩放后的二次裁剪同样把
    /// 「帽 binding」写进计划（`plan.cap_narrowed` ⟹ `n_cap_narrowed`），而不是随 `..plan`
    /// 静默继承 `false`。场景复用 #356 的「60→100 缩放突破」（见
    /// [`physical_order_shares_same_reclamped_target_as_ledger`]）。
    ///
    /// **照实记录**（本票边界条件）：该路径下 `rescaled` 恒为真——二次裁剪的触发前提是
    /// `attribute_total` 把 `targets` 放大出 cap，而第一施加点已保证 `gated_ℓ ≤ cap_ℓ`，故
    /// `targets ≠ gated` ⟹ `Σgated ≠ t_lee_raw` ⟹ 走非恒等分支（`attribute_total` 的
    /// `rescaled` 语义即 `Σbasis ≠ total`）。**逐级判据在这条路径上修前即成立**，MED-C 的
    /// 可触达红点在第一施加点（见上一测）。本测钉住的是两处施加点的判据**同形同源**：
    /// 若日后有别的路径让 `rescaled=false` 与二次裁剪共存，解释项已就位而不是重新缺一半。
    #[test]
    fn reclamp_path_records_cap_narrowed_in_plan() {
        let risk = RiskConfig {
            level_weights: vec![0.5, 0.1],
            enforce_level_cap: true,
            ..RiskConfig::default()
        };
        let weights = PiThetaWeights::from_risk(&risk);
        let gate = KThetaRiskGate::open();
        let sep_legs = two_level_legs(50.0, 10.0);
        let mut ticks = LevelClockTicks::empty();
        ticks.insert(0, LevelEventKind::BspConfirmed);
        ticks.insert(1, LevelEventKind::BspConfirmed);

        let mut level_order = LevelOrderLedger::new();
        let mut order = Order { action: StrictAction::Wait, qty: 0, exec_index: 0 };
        plan_level_gated_order(
            &mut level_order,
            &ticks,
            &sep_legs,
            40, // pan_div_child_units：Σgated=60 → 账户层目标 100（触发缩放后重越 cap_0）
            0,
            AccountProjectionCtx { p_t: 0.0, base_units: 100.0, risk: &risk, weights, gate },
            0,
            0,
            &mut order,
        );
        let s = level_order.stats();
        assert_eq!(s.n_rescaled, 1, "前置：本场景经 attribute_total 比例缩放（60→100）");
        assert_eq!(
            s.n_cap_narrowed, 1,
            "二次裁剪路径未把帽 binding 写进计划（`..plan` 静默继承 cap_narrowed=false）"
        );
        assert!(
            s.per_level_sparsity_has_no_unexplained_violation(),
            "逐级稀疏性无未解释违例（本路径两级均有 tick，off_clock={}）",
            s.n_levels_off_clock_delta
        );
    }
}

/// ★#360 venue 费率标定接线测试：`ExecConfig::fee_schedule` 的 None/Some 两支在**生产成交
/// 回路**上的可观测差别（模块内单测只验算术，本组验的是"账本确实用了 datum 费率"）。
///
/// **认识论等级（231号强制）**：除 `real_window_datum_reconciliation`（真实价格序列，仍是
/// **L1** 一致性）外，本组全部跑合成 bar ⟹ **L1**（管线/算术正确性，零市场信息增量）。
/// datum 本身的 **L2**（venue 官方费率表快照）由 `venue_fee` 模块的对账测试承担；本组
/// **不主张**任何 alpha 或有效域结论。
#[cfg(test)]
mod venue_fee_wiring_tests {
    use super::*;
    use super::super::super::config::{ExecConfig, ThetaConfig};
    use super::super::super::venue_fee::{
        datum_dir, load_datum, FeeUnit, VenueFeeSchedule,
    };

    const NAV: f64 = 100_000.0;
    const PX: f64 = 20.0;

    /// tick_size=1 ⟹ close tick 与美元价逐位相等（量化不引入误差，费率差别可手算对账）。
    fn cfg() -> ThetaConfig {
        let mut c = ThetaConfig::default();
        c.tick.tick_size = 1.0;
        c
    }

    fn bars(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| Bar {
                source_index: i,
                timestamp: i as i64,
                open: PX as i64,
                high: PX as i64,
                low: PX as i64,
                close: PX as i64,
                volume: 1_000,
                untradable: false,
            })
            .collect()
    }

    /// 单笔买单（qty 手 @ bar0）。
    fn buy(qty: i64) -> Vec<Order> {
        vec![Order { action: StrictAction::Buy, qty, exec_index: 0 }]
    }

    fn oklo() -> VenueFeeSchedule {
        load_datum(&datum_dir().join("venue_fee_ibkr_pro_20260726.json"))
            .expect("IBKR datum 可加载")
            .resolve("OKLO", "PRO_TIERED_LE_300K_SHARES")
            .expect("OKLO 在册")
    }

    /// 末点权益 = (cash + units·px)/NAV；买入后同价 ⟹ 权益缺口恰为**实付费用/NAV**。
    fn fee_paid_via_equity(config: &ThetaConfig, orders: &[Order]) -> f64 {
        let (eq, _, _) = simulate_fills(&bars(3), orders, NAV, config);
        (1.0 - *eq.last().unwrap()) * NAV
    }

    fn approx(a: f64, b: f64, ctx: &str) {
        let tol = 1e-9 * b.abs().max(1.0);
        assert!((a - b).abs() <= tol, "{ctx}: 实得 {a:.12} ≠ 预期 {b:.12}");
    }

    /// **None ⟹ 现状**：三常数合成率 3bp/side，实付 = 名义 × 3e-4（逐值对账）。
    #[test]
    fn none_schedule_charges_flat_three_bps() {
        let c = cfg();
        assert!(c.exec.fee_schedule.is_none(), "default 未标定");
        approx(fee_paid_via_equity(&c, &buy(100)), 2000.0 * 3e-4, "100 手 @ $20 未标定费用");
        approx(fee_paid_via_equity(&c, &buy(10)), 200.0 * 3e-4, "10 手 @ $20 未标定费用");
    }

    /// **同率 per-notional 标定档 ⟹ 与 None 逐位相同**：证明 Some 支不引入额外浮点重排。
    ///
    /// 两侧都把 `slippage_bps` 置 0（datum 不覆盖滑点 ⟹ 标定档要加它；置 0 才谈得上"同率"），
    /// 于是 base 侧 `(3+0+0)/1e4` 与 cal 侧 `3.0/1e4 + 0.0` **逐位同值** ⟹ 权益曲线/日
    /// returns/trade_pnls 三者 `assert_eq!` 位相等。
    #[test]
    fn notional_schedule_equal_to_fallback_is_bit_exact_to_none() {
        let mut base = cfg();
        base.exec.commission_bps = 3.0;
        base.exec.slippage_bps = 0.0;
        let mut cal = base.clone();
        cal.exec.commission_bps = 1.0; // 标定档下 commission 常数被 datum 取代（取值无关）
        cal.exec.fee_schedule = Some(VenueFeeSchedule {
            venue: "SYNTH".into(),
            symbol: "X".into(),
            tier: "T".into(),
            unit: FeeUnit::Notional { maker_bps: 3.0, taker_bps: 3.0 },
            datum_sha256: "0".repeat(64),
        });
        for qty in [1i64, 10, 100, 7_777] {
            let (eq_a, dr_a, pnl_a) = simulate_fills(&bars(3), &buy(qty), NAV, &base);
            let (eq_b, dr_b, pnl_b) = simulate_fills(&bars(3), &buy(qty), NAV, &cal);
            assert_eq!(eq_a, eq_b, "qty={qty}: 权益曲线须逐位相同");
            assert_eq!(dr_a, dr_b, "qty={qty}: 日 returns 须逐位相同");
            assert_eq!(pnl_a, pnl_b, "qty={qty}: trade_pnls 须逐位相同");
        }
    }

    /// **Some(datum) ⟹ 账本按 venue 口径扣费**：OKLO/IBKR Pro Tiered，100 股 @ $20 买入
    /// = $0.370559（报告 §2.3 逐项：佣金 0.35 + pass-through + 清算 0.02 + CAT）。
    #[test]
    fn per_share_schedule_charges_datum_fee() {
        let mut c = cfg();
        c.exec.fee_schedule = Some(oklo());
        // datum 费率 $0.370559 + **未被 datum 覆盖的滑点 2bp**（2000×2e-4 = $0.40）。
        approx(fee_paid_via_equity(&c, &buy(100)), 0.370559 + 0.4, "OKLO 100 股买入实付");
        // 未标定档同单收 $0.60（3bp）。标定后 $0.7706 > $0.60——佣金科目被真实费率替换、
        // 滑点科目原样保留，总摩擦上升；若标定反而降低总摩擦，即是把滑点悄悄抹掉了。
        approx(fee_paid_via_equity(&cfg(), &buy(100)), 0.6, "同单未标定实付");
        assert!(
            fee_paid_via_equity(&c, &buy(100)) > fee_paid_via_equity(&cfg(), &buy(100)),
            "本价位下标定实付须高于未标定实付"
        );
        // ★不是普适命题（090 照实）：per-share 档折算成 bp 随价位反比——$20 上 1.85bp+2bp 滑点
        //   > 未标定 3bp，但高价位上 $0.0035/股 会低于 1bp，总摩擦可低于未标定档（真实 OKLO 窗
        //   实测即如此，见 `real_window_datum_reconciliation` 输出）。本断言只约束本价位。
    }

    /// **最低佣金在小单上主导**（压成 per-notional 会丢掉的效应，报告 §3.1）：
    /// 10 股 @ $20 名义仅 $200，实付 $0.392289（≈19.6bp，含 2bp 滑点），远高于未标定 3bp。
    #[test]
    fn per_share_min_commission_dominates_small_order() {
        let mut c = cfg();
        c.exec.fee_schedule = Some(oklo());
        let paid = fee_paid_via_equity(&c, &buy(10));
        // datum $0.352289 + 滑点 200×2e-4 = $0.04。
        approx(paid, 0.352289 + 0.04, "OKLO 10 股买入实付（最低佣金托底 + 滑点）");
        assert!(
            paid > fee_paid_via_equity(&cfg(), &buy(10)) * 5.0,
            "最低佣金档实付须远高于未标定常率档（{paid}）"
        );
    }

    /// **卖出监管费只在卖出侧计**（SEC Section 31 + FINRA TAF）：开空 100 股 ⟹ 实付
    /// $0.431259 > 买入 $0.370559，差额 = 0.0607（报告 §2.3/§2.5 一手数字）。
    #[test]
    fn per_share_sell_side_adds_regulatory_fees() {
        let mut c = cfg();
        c.exec.fee_schedule = Some(oklo());
        let sell = vec![Order { action: StrictAction::Sell, qty: 100, exec_index: 0 }];
        let (eq, _, _) = simulate_fills(&bars(3), &sell, NAV, &c);
        // 开空后同价：权益 = (cash + units·px)/NAV，units=−100 ⟹ 缺口仍为实付费用。
        let paid = (1.0 - *eq.last().unwrap()) * NAV;
        approx(paid, 0.431259 + 0.4, "OKLO 100 股开空实付（含 SEC+TAF + 滑点）");
        // 滑点两侧同额 ⟹ 买卖差额恰是监管费增量（SEC 0.0412 + TAF 0.0195）。
        approx(paid - (0.370559 + 0.4), 0.0607, "卖出监管费增量");
    }

    /// ★#374 LOW-E：**开仓段现金拒单 ⟹ per-share 档低估成本**——把 `order_fee_rate` 文档里
    /// 登记的偏差**方向与量级**锁进测试（无测试的登记会被后续重构悄悄放大）。
    ///
    /// 构造：NAV 小 ⟹ bar0 开空 10 股（询价量 10，正确），bar1 下 Buy 1000 触发翻转——段 1
    /// 平掉 10 股空头成交，段 2 开多 990 股因现金不足**整段拒单**。但费率在两段之前按整单
    /// `qty=1000` 询过一次价 ⟹ 最低佣金 $0.35 摊到 1000 股（而非实际成交的 10 股）⟹ 平仓那
    /// 10 股按**偏低**费率扣费。断言方向（低估，非高估）+ 量级（总费率 ≈5×；datum 科目本身
    /// ≈9.5×，被与量无关的滑点分量摊薄）。
    ///
    /// **认识论 L1**（合成 bar 上的费用算术；不主张任何 alpha）。
    #[test]
    fn per_share_cash_rejected_open_underestimates_fee() {
        const SMALL_NAV: f64 = 500.0; // 开空收 ~$200 ⟹ cash ~$700 ≪ 990 股 @ $20 = $19,800
        let mut c = cfg();
        c.exec.fee_schedule = Some(oklo());
        let orders = vec![
            Order { action: StrictAction::Sell, qty: 10, exec_index: 0 },
            Order { action: StrictAction::Buy, qty: 1_000, exec_index: 1 },
        ];
        let (eq, _, pnls) = simulate_fills(&bars(3), &orders, SMALL_NAV, &c);
        assert_eq!(pnls.len(), 1, "只应有 1 笔平仓（开多段被现金拒单）");

        // 末点持仓归零 ⟹ 权益缺口 = 两次成交的实付费用总额（价格恒定，无 PnL 分量）。
        let paid = (1.0 - *eq.last().unwrap()) * SMALL_NAV;

        let q = super::super::treasury::fee_quoter(&c.exec);
        let rate = |qty: f64, side| q.production_rate_or_fallback(qty, PX, side);
        let fee_open = 10.0 * PX * rate(10.0, FillSide::Sell); // 开空段：询价量 = 实际成交量
        let fee_close_quoted = 10.0 * PX * rate(1_000.0, FillSide::Buy); // 实扣：按整单量询价
        let fee_close_correct = 10.0 * PX * rate(10.0, FillSide::Buy); // 应扣：按实际成交量

        approx(paid, fee_open + fee_close_quoted, "实付 = 开空段 + 平仓段（按整单量询价）");
        assert!(
            fee_close_quoted < fee_close_correct,
            "偏差方向须为低估（实扣 {fee_close_quoted:.6} 应 < 应扣 {fee_close_correct:.6}）"
        );
        // 量级：总费率实测 ≈5.1× 低估。**不是** 9.5×——datum 科目本身确实差 9.5 倍
        // （$0.352289 vs $0.037056，最低佣金摊到 100 倍的量），但费率里还含**与量无关**的
        // 滑点 2bp（两侧同为 $0.04），它把比值压回 5 倍档。照实标（090）。
        assert!(
            fee_close_correct > fee_close_quoted * 5.0,
            "本构造下低估约 5 倍（datum 科目 9.5×，被与量无关的滑点分量摊薄）：\
             实扣 {fee_close_quoted:.6} vs 应扣 {fee_close_correct:.6}"
        );
        // ★有效域（090 照实）：低估只在 **per-share 档 + 开仓段整段拒单** 同时发生时出现。
        // per-notional 与未标定档费率与 qty 无关 ⟹ 无此偏差，下面用同一 orders 反证。
        let paid_flat = {
            let (eq, _, _) = simulate_fills(&bars(3), &orders, SMALL_NAV, &cfg());
            (1.0 - *eq.last().unwrap()) * SMALL_NAV
        };
        approx(paid_flat, 2.0 * 10.0 * PX * 3e-4, "未标定档：两段各 10 股 × 3bp，与询价量无关");
    }

    /// ★票体 Acceptance② 「Some(datum) 时费用计算对拍（**BTC/OKLO 窗口**，与报告 §2 一手数字
    /// 对账）」：在**真实数据窗**上跑两遍（None / Some），把账本实付差与**独立重算**的 Σ 费用差
    /// 对上——重算侧的费率**不调 datum 对象**，而是把报告 §2 的一手数字逐项手写成公式
    /// （Binance 10bp/side；IBKR $0.0035/股 + min $0.35 + 1% 上限 + 清算 0.0002 + CAT 0.000003
    /// + pass-through 0.00074×佣金 + 卖出 SEC 0.0000206 + TAF 0.000195/股 cap 9.79），两条路径
    /// 独立 ⟹ 对上才是对账，不是同义反复。
    ///
    /// **认识论 L1**（费用算术在真实价格序列上的一致性；不主张任何 alpha）。
    ///
    /// `#[ignore]`：需真实数据（`analysis/data_cache/*.json`，BTC 329MB）。
    /// 跑法：`cargo test --release --lib real_window_datum_reconciliation -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_window_datum_reconciliation() {
        use super::super::data;
        const BIG_NAV: f64 = 1e9; // 足够大 ⟹ 无现金拒单 ⟹ 两遍 units 轨迹完全相同

        // (品种, datum 文件, 档位, 每单手数)
        let cases: [(&str, &str, &str, i64); 2] = [
            ("BTC", "venue_fee_binance_spot_20260726.json", "VIP0", 1),
            ("OKLO", "venue_fee_ibkr_pro_20260726.json", "PRO_TIERED_LE_300K_SHARES", 100),
        ];

        for (sym, file, tier, lots) in cases {
            let base = ThetaConfig::default();
            let ds = data::load_by_symbol(sym, &base)
                .unwrap_or_else(|e| panic!("{sym} 数据加载失败：{e}"));
            // 窗口 = 末 2000 根可交易 bar（真实价格序列，非合成）。
            let win: Vec<Bar> = ds
                .bars
                .iter()
                .rev()
                .filter(|b| !b.untradable && b.close > 0)
                .take(2000)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            assert!(win.len() >= 500, "{sym}: 窗口 bar 数不足（{}）", win.len());

            // 确定性订单流：每 100 根 bar 一次 Buy，其后第 50 根 Close（多轮往返）。
            let mut orders: Vec<Order> = Vec::new();
            let mut i = 0usize;
            while i + 50 < win.len() {
                orders.push(Order { action: StrictAction::Buy, qty: lots, exec_index: i });
                orders.push(Order { action: StrictAction::Close, qty: lots, exec_index: i + 50 });
                i += 100;
            }
            assert!(orders.len() >= 10, "{sym}: 订单数不足");

            let mut cal = base.clone();
            cal.exec.fee_schedule = Some(
                load_datum(&datum_dir().join(file))
                    .expect("datum 可加载")
                    .resolve(sym, tier)
                    .expect("档位在册"),
            );

            let (eq_none, _, _) = simulate_fills(&win, &orders, BIG_NAV, &base);
            let (eq_cal, _, _) = simulate_fills(&win, &orders, BIG_NAV, &cal);

            // ── 独立重算（不调 datum 对象；报告 §2 数字手写）──────────────────────
            let slip = base.exec.slippage_bps / 10_000.0; // datum 不覆盖，两档都收
            let none_rate = (base.exec.commission_bps + base.exec.slippage_bps + base.exec.tax_bps)
                / 10_000.0;
            let venue_rate = |qty: f64, px: f64, sell: bool| -> f64 {
                match sym {
                    "BTC" => 10.0 / 10_000.0, // Binance 现货 VIP0 taker（报告 §2.1）
                    _ => {
                        // IBKR Pro Tiered 首档（报告 §2.3/§2.5）
                        let notional = qty * px;
                        let commission = (0.0035 * qty).max(0.35).min(notional * 0.01);
                        let passthru = commission * (0.000175 + 0.000565);
                        let per_share = (0.0002 + 0.000003) * qty;
                        let sell_reg = if sell {
                            notional * 0.0000206 + (0.000195 * qty).min(9.79)
                        } else {
                            0.0
                        };
                        (commission + passthru + per_share + sell_reg) / notional
                    }
                }
            };

            let mut sum_none = 0.0;
            let mut sum_cal = 0.0;
            let mut units = 0.0f64;
            let mut sorted = orders.clone();
            sorted.sort_by_key(|o| o.exec_index);
            for o in &sorted {
                let px = win[o.exec_index].close as f64 * base.tick.tick_size;
                let (delta, _) = order_delta_close_only(o.action, units).expect("本流无 noop 单");
                let qty = o.qty as f64;
                let notional = qty * px;
                sum_none += notional * none_rate;
                sum_cal += notional * (venue_rate(qty, px, delta < 0.0) + slip);
                units += delta * qty;
            }
            assert_eq!(units, 0.0, "{sym}: 订单流应收平");

            // 两遍 units 轨迹相同 ⟹ 末点权益差 = −Σ费用差 / NAV（逐笔现金流的唯一差别）。
            let ledger_diff = (eq_none.last().unwrap() - eq_cal.last().unwrap()) * BIG_NAV;
            let recomputed_diff = sum_cal - sum_none;
            let tol = 1e-6 * recomputed_diff.abs().max(1.0);
            assert!(
                (ledger_diff - recomputed_diff).abs() <= tol,
                "{sym}: 账本实付差 {ledger_diff:.6} ≠ §2 独立重算差 {recomputed_diff:.6}"
            );
            eprintln!(
                "[#360 对拍] {sym} 窗 {}bar / {}单：未标定 Σfee={:.4}，标定 Σfee={:.4}（datum+滑点），\
                 账本差={:.6} 重算差={:.6}",
                win.len(),
                orders.len(),
                sum_none,
                sum_cal,
                ledger_diff,
                recomputed_diff
            );
        }
    }

    /// ★#388 T2「OKLO 真实窗单跑」：per-share 档（IBKR Pro Tiered）在**真实 OKLO 价格序列**上的
    /// **触达读数**——最低佣金 $0.35 托底、1% 名义额封顶、卖出监管费（SEC Section 31 / FINRA TAF
    /// 及其 $9.79 上限）各自在多少笔上生效。
    ///
    /// 与 [`real_window_datum_reconciliation`] 的分工：那一测证明「datum 路径与手写公式对得上」
    /// （一个 Σ 对拍）；本测答的是票体 #388 的问题「per-share 档的**非线性拐点**在真实数据上到底
    /// 触没触达」——这是单标量费率口径在**按股档（per-share）**失效的**经验证据**（#374 有效域
    /// 收窄的实证面）：若最低佣金/上限在真实单量上频繁生效，逐笔有效费率就不是常数，任何单标量
    /// 近似都失真。
    ///
    /// **有效域：本证据只管按股档，不牵连按金额档**（★#423 措辞收窄，#447 尾扫 MED-1 第 3 处）。
    /// 原措辞写作「在**标定档**失效」，把 per-share 特有的非线性当成了标定档全体的性质。按金额档
    /// （[`FeeUnit::Notional`](super::super::venue_fee::FeeUnit::Notional)）的有效费率 = `bps/1e4`，
    /// `VenueFeeSchedule::effective_rate` 的该分支**函数体不读 `qty`/`px`**，也无最低佣金/名义额
    /// 上限/仅卖出科目 ⟹ 没有拐点可触达，本测的证据对它**无话可说**（既不支持也不反对）。
    /// 按金额档能否取单标量另有判据（maker/taker 是否逐位对称），本体 =
    /// `VenueFeeSchedule::constant_effective_rate`，分叉见 `treasury::scalar_cost_rate_opt`（★#423）。
    ///
    /// **单量分组**（同款构造、只改每单股数）：50 / 100 / 500 股 —— 50 股 raw 佣金 $0.175 < $0.35
    /// ⟹ 必触底；100 股恰 $0.35（边界，不触底）；500 股 $1.75 ⟹ 不触底。三组一起跑才能看出
    /// 有效费率**不是单量的常数**：触底组被最低佣金抬高，未触底组落在同一线性档上（100/500 股
    /// 有效费率相同 —— per-share 佣金与按股监管费都与名义额同比，只有托底/封顶两个拐点破线性）。
    ///
    /// **独立重算**：分项（佣金/pass-through/清算+CAT/卖出 SEC/卖出 TAF）由报告 §2 的一手数字
    /// **手写公式**算，与 datum 对象 `fee_usd` 的总额逐笔对拍 ⟹ 两条独立路径，不是同义反复。
    ///
    /// **认识论 L1**（费用算术在真实价格序列上的一致性 + 触达计数；不主张任何 alpha）。
    ///
    /// `#[ignore]`：需真实数据（`analysis/data_cache/oklo_1m.json` 等）。跑法：
    /// `cargo test --release --lib oklo_real_window_per_share_readings -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn oklo_real_window_per_share_readings() {
        use super::super::super::venue_fee::PRODUCTION_LIQUIDITY_ROLE;
        use super::super::data;
        const BIG_NAV: f64 = 1e9; // 足够大 ⟹ 无现金拒单 ⟹ 各组 units 轨迹相同
        // 读数落盘路径（与 m8 跑批 `/tmp/m8_e2e_all_systems_oos.md` 同惯例：`#[ignore]` 手动跑批
        // 的产物落 /tmp，由跑批记录引用；不进仓，避免验收产物与源码同轴漂移）。
        const OKLO_READINGS_PATH: &str = "/tmp/388_oklo_per_share_readings.md";

        let base = ThetaConfig::default();
        let sched = oklo();
        let f = match &sched.unit {
            super::super::super::venue_fee::FeeUnit::PerShare(f) => *f,
            _ => panic!("OKLO 档须是 per_share 单位"),
        };
        let ds = data::load_by_symbol("OKLO", &base).expect("OKLO 数据加载");
        let win: Vec<Bar> = ds
            .bars
            .iter()
            .rev()
            .filter(|b| !b.untradable && b.close > 0)
            .take(2000)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        assert!(win.len() >= 500, "OKLO 窗口 bar 数不足（{}）", win.len());

        // 抬头口径标签的 datum 前缀由 `&sched.datum_sha256[..12]` **运行时插值**（与本测 footer
        // 同源，见下方 `md.push_str`），不写死字面量：产物的认识论等级须机器可读（#385 US4，
        // #418 LOW-2）。落盘后由本测末尾的回读断言校验。
        let mut md = format!(
            "# OKLO 真实窗 per-share 档读数（#388 T2；IBKR Pro Tiered ≤300K shares）\n\n\
             口径标签：`[L2费率标定: datum {}]`（成交费率科目）。**认识论 L1**\
             （费用算术 + 触达计数；不作 alpha 论据）。\n\n\
             | 每单股数 | 成交腿数 | Σ佣金 | Σpass-thru | Σ清算+CAT | Σ卖出SEC | Σ卖出TAF | Σ总费用 | \
             最低佣金触达 | 1%上限触达 | TAF上限触达 | 有效费率(Σ费用/Σ名义) |\n\
             |---|---|---|---|---|---|---|---|---|---|---|---|\n",
            &sched.datum_sha256[..12],
        );
        for lots in [50i64, 100, 500] {
            let mut orders: Vec<Order> = Vec::new();
            let mut i = 0usize;
            while i + 50 < win.len() {
                orders.push(Order { action: StrictAction::Buy, qty: lots, exec_index: i });
                orders.push(Order { action: StrictAction::Close, qty: lots, exec_index: i + 50 });
                i += 100;
            }
            assert!(orders.len() >= 10, "订单数不足");

            let mut cal = base.clone();
            cal.exec.fee_schedule = Some(sched.clone());
            let (eq_none, _, _) = simulate_fills(&win, &orders, BIG_NAV, &base);
            let (eq_cal, _, _) = simulate_fills(&win, &orders, BIG_NAV, &cal);

            let slip = base.exec.slippage_bps / 10_000.0;
            let none_rate =
                (base.exec.commission_bps + base.exec.slippage_bps + base.exec.tax_bps) / 10_000.0;
            let (mut s_comm, mut s_pass, mut s_ps, mut s_sec, mut s_taf) = (0.0, 0.0, 0.0, 0.0, 0.0);
            let (mut n_min, mut n_cap, mut n_taf_cap, mut n_legs) = (0usize, 0usize, 0usize, 0usize);
            let (mut s_notional, mut sum_none, mut sum_cal_total) = (0.0, 0.0, 0.0);
            let mut units = 0.0f64;
            let mut sorted = orders.clone();
            sorted.sort_by_key(|o| o.exec_index);
            for o in &sorted {
                let px = win[o.exec_index].close as f64 * base.tick.tick_size;
                let (delta, _) = order_delta_close_only(o.action, units).expect("本流无 noop 单");
                let qty = o.qty as f64;
                let notional = qty * px;
                let sell = delta < 0.0;
                // ── 独立重算（报告 §2.3/§2.5 一手数字手写，不调 datum 对象）──
                let raw = 0.0035 * qty;
                let cap = notional * 0.01;
                let commission = raw.max(0.35).min(cap);
                if raw < 0.35 && 0.35 <= cap {
                    n_min += 1;
                }
                if cap < raw.max(0.35) {
                    n_cap += 1;
                }
                let passthru = commission * (0.000175 + 0.000565);
                let per_share = (0.0002 + 0.000003) * qty;
                let (sec, taf) = if sell {
                    let taf_raw = 0.000195 * qty;
                    if taf_raw > 9.79 {
                        n_taf_cap += 1;
                    }
                    (notional * 0.0000206, taf_raw.min(9.79))
                } else {
                    (0.0, 0.0)
                };
                let recomputed = commission + passthru + per_share + sec + taf;
                // datum 对象路径（第二条独立路径）——逐笔对拍，容差 = f64 求和自由度。
                let side = if sell {
                    super::super::super::strategy::exec::FillSide::Sell
                } else {
                    super::super::super::strategy::exec::FillSide::Buy
                };
                // 角色取生产常量（★#423 C 线：测试也不写字面量——本测对拍的是生产口径）。
                let via_datum = sched.fee_usd(qty, px, side, PRODUCTION_LIQUIDITY_ROLE);
                assert!(
                    (via_datum - recomputed).abs() <= 1e-9 * recomputed.abs().max(1.0),
                    "OKLO lots={lots} 逐笔对拍失败：datum {via_datum:.12} ≠ 手写 {recomputed:.12}"
                );
                s_comm += commission;
                s_pass += passthru;
                s_ps += per_share;
                s_sec += sec;
                s_taf += taf;
                s_notional += notional;
                sum_none += notional * none_rate;
                sum_cal_total += recomputed + notional * slip; // 账本口径含未标定滑点 addon
                n_legs += 1;
                units += delta * qty;
            }
            assert_eq!(units, 0.0, "订单流应收平");

            // 账本实付差 = 独立重算差（两遍 units 轨迹相同 ⟹ 唯一差别是费用现金流）。
            let ledger_diff = (eq_none.last().unwrap() - eq_cal.last().unwrap()) * BIG_NAV;
            let recomputed_diff = sum_cal_total - sum_none;
            assert!(
                (ledger_diff - recomputed_diff).abs() <= 1e-6 * recomputed_diff.abs().max(1.0),
                "OKLO lots={lots}: 账本实付差 {ledger_diff:.6} ≠ 独立重算差 {recomputed_diff:.6}"
            );
            // 分项之和 = 总额（分解无遗漏科目）。
            let parts = s_comm + s_pass + s_ps + s_sec + s_taf;
            let total_datum = sum_cal_total - s_notional * slip;
            assert!(
                (parts - total_datum).abs() <= 1e-9 * total_datum.abs().max(1.0),
                "OKLO lots={lots}: 科目分项和 {parts:.12} ≠ datum 总额 {total_datum:.12}"
            );

            md.push_str(&format!(
                "| {lots} | {n_legs} | {s_comm:.4} | {s_pass:.6} | {s_ps:.4} | {s_sec:.4} | \
                 {s_taf:.4} | {total_datum:.4} | {n_min}/{n_legs} | {n_cap}/{n_legs} | \
                 {n_taf_cap}/{n_legs} | {:.6e} |\n",
                total_datum / s_notional,
            ));
            eprintln!(
                "[#388 OKLO] lots={lots} legs={n_legs} Σfee={total_datum:.4} \
                 有效费率={:.6e} 最低佣金触达={n_min} 1%上限触达={n_cap} TAF上限触达={n_taf_cap}",
                total_datum / s_notional,
            );
        }
        md.push_str(&format!(
            "\n- 窗口 = OKLO 末 {} 根可交易 bar（真实价格序列，非合成）；订单流 = 每 100 bar 买入、\
             其后第 50 bar 平仓（确定性，与 `real_window_datum_reconciliation` 同款构造）。\n\
             - datum：`venue_fee_ibkr_pro_20260726.json` / OKLO / `PRO_TIERED_LE_300K_SHARES`，\
               sha256 前 12 位 = `{}`（sidecar 已校验）。\n\
             - 「有效费率」= Σ总费用 / Σ名义额，**不含**未标定滑点 addon（{:.1} bp/腿，datum 不覆盖）。\n\
             - 最低佣金判定：`0.0035×股数 < $0.35 ≤ 1%×名义额`；1% 上限判定：`1%×名义额 < max(raw, $0.35)`。\n\
             - per-share 参数：commission {}/股、min {}、cap {}、clearing {}/股、CAT {}/股、\
               SEC {} × 卖出名义、TAF {}/股 cap {}。\n",
            win.len(),
            &sched.datum_sha256[..12],
            base.exec.slippage_bps,
            f.commission_per_share_usd,
            f.min_commission_usd,
            f.max_commission_frac_of_notional,
            f.clearing_per_share_usd,
            f.cat_per_share_usd,
            f.sell_sec_fee_frac,
            f.sell_taf_per_share_usd,
            f.sell_taf_cap_usd,
        ));
        // 落盘失败即测试失败（不 `.ok()` 吞错）：本测的产出**就是**这份读数，写不出去 = 没有交付物。
        std::fs::write(OKLO_READINGS_PATH, &md)
            .unwrap_or_else(|e| panic!("OKLO 读数落盘失败 {OKLO_READINGS_PATH}：{e}"));

        // ── #418 LOW-2 机器断言：产物抬头的口径标签必须落**真哈希**，不得留字面占位符 ──
        // 母 SPEC #385 US4 的目的是「读数的认识论等级**机器可读**」。本组断言守的是一条
        // **机器可读格式契约**：产物抬头承诺「口径标签」行可被正则
        // `\[L2费率标定: datum ([0-9a-f]{12})\]` 解析出档位与 datum 身份。
        // ★#423 措辞订正（#447 尾扫 LOW-2）：原注释写作「**消费方**以该正则提取……」——
        // 仓内**不存在**这样的消费方（`grep -rn "L2费率标定" rust/src scripts chanlun` 的命中
        // 全是**生产端**写标签：`fill.rs`/`wverify_run.rs`/`venue_fee.rs`/`risk.rs`，加评审报告
        // 引用；没有任何代码/脚本读回并解析它）。故契约是**给未来/外部消费方的格式承诺**，
        // 不是对既有消费方的描述——虚构一个不存在的消费方即 090 声明膨胀。
        // 抬头若留 `<前12位>` 这类字面占位符，上述正则在产物上匹配失败 ⟹ 契约破、等级退化为人读。
        // 断言对象是**回读的落盘文件**而非内存 `md`：校验的必须是交付物本身。
        let written = std::fs::read_to_string(OKLO_READINGS_PATH)
            .unwrap_or_else(|e| panic!("OKLO 读数回读失败 {OKLO_READINGS_PATH}：{e}"));
        let label_line = written
            .lines()
            .find(|l| l.starts_with("口径标签："))
            .unwrap_or_else(|| panic!("产物缺「口径标签：」行：{OKLO_READINGS_PATH}"));
        assert!(
            !label_line.contains('<') && !label_line.contains('>'),
            "口径标签行残留字面占位符（#418 LOW-2）：{label_line}"
        );
        let digest12 = &sched.datum_sha256[..12];
        assert!(
            digest12.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "datum sha256 前 12 位非小写 hex：{digest12}"
        );
        // ★#423（#447 尾扫 LOW-3）：本条相对前两条**冗余**——`digest12` 与生成抬头用的是同一
        // 变量，故它对「哈希本身是否正确」是重言的，只能确认标签格式串未被改坏、且回读文件里
        // 的那一行确实由本次生成写出。**真防线是第一条**（占位符检查：抬头是否落了运行时插值
        // 而非字面 `<前12位>`）。保留本条的理由是格式串完整性（`[L2费率标定: datum …]` 的前后缀
        // 被改坏时它变红），不是哈希正确性——后者由 sidecar `shasum -a 256 -c` 独立复核。
        let expected_label = format!("[L2费率标定: datum {digest12}]");
        assert!(
            label_line.contains(&expected_label),
            "口径标签行未落真哈希（#418 LOW-2）：期望含 `{expected_label}`，实为 {label_line}"
        );

        eprintln!("[#388 OKLO] 读数落盘 {OKLO_READINGS_PATH}");
    }

    /// 口径标签契约（risk.rs:709）：None ⟹ L1 未标定；Some ⟹ `[L2费率标定: datum <前12位>]`。
    #[test]
    fn calibration_label_follows_schedule() {
        use super::super::super::strategy::risk::{rate_calibration_label, RATE_UNCALIBRATED_LABEL};
        let plain = ExecConfig::default();
        assert_eq!(rate_calibration_label(&plain), RATE_UNCALIBRATED_LABEL);
        let mut cal = ExecConfig::default();
        let s = oklo();
        let sha = s.datum_sha256.clone();
        cal.fee_schedule = Some(s);
        assert_eq!(
            rate_calibration_label(&cal),
            format!("[L2费率标定: datum {}]", &sha[..12])
        );
    }
}
