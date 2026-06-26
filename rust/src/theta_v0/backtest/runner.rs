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

use super::super::config::ThetaConfig;
use super::super::strategy::{AccountState, VoiceDecision};
use super::super::types::{Bar, Order, StrictAction};
use super::super::{classifier, parser, strategy};
use super::data::Dataset;
use super::metrics::{self, Metrics};

/// 单次回测的完整输出（指标 + 诊断量）。
#[derive(Debug, Clone)]
pub struct RunResult {
    pub symbol: String,
    pub metrics: Metrics,
    /// 输入 bar 数（窗内）。
    pub n_bars: usize,
    /// 引擎产出的订单数（**当前恒为 0**——阻塞点 A+B；非 0 时才是 L2 回测）。
    pub n_orders: usize,
    /// 不可交易 bar 占比（§5.5：>20% 判 inconclusive）。
    pub untradable_ratio: f64,
    /// 是否 L2（true ⟺ 产出非空订单流且真实数据）。当前恒 false（阻塞点未解）。
    pub is_l2: bool,
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

    // 初始账户（绝对 NAV，各声部空仓）。声部树深度 = voice.max_depth。
    let account = AccountState {
        nav: if initial_nav > 0.0 { initial_nav } else { 1.0 },
        voice_qty: vec![0; config.voice.max_depth as usize],
    };

    // ── 步骤 4：策略（声部决策 → 订单流）。plan_orders 已实装；空 decisions ⇒ 空订单。 ──
    let orders = strategy::plan_orders(&decisions, bars, &account, config);

    // ── 步骤 5：fill 模拟 + 权益曲线（绝对 NAV 记账，归一化输出）。 ──
    let (equity_curve, daily_returns, trade_pnls) =
        simulate_fills(bars, &orders, account.nav, config);

    // buy&hold 对照（首尾 close，整数 tick → f64 比率，无 tick_size 依赖）。
    let bh_return = buy_and_hold_return(bars);

    let m = metrics::compute(&equity_curve, &daily_returns, &trade_pnls, years, bh_return);

    let n_orders = orders.len();
    let untradable_ratio = dataset.untradable_ratio();
    // L2 判据：产出非空订单流（阻塞点解除）+ 真实数据。订单空 ⇒ 仅 L1（管线串通）。
    let is_l2 = n_orders > 0;

    RunResult {
        symbol: dataset.symbol.clone(),
        metrics: m,
        n_bars: bars.len(),
        n_orders,
        untradable_ratio,
        is_l2,
    }
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
    let mut entry_price: f64 = 0.0;
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
            apply_order(o, px, fee_rate, &mut cash, &mut units, &mut entry_price, &mut trade_pnls);
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

/// 应用单个订单到仓位（开/平/加/减仓 + 费用）。
fn apply_order(
    o: &Order,
    px: f64,
    fee_rate: f64,
    cash: &mut f64,
    units: &mut f64,
    entry_price: &mut f64,
    trade_pnls: &mut Vec<f64>,
) {
    let qty = o.qty as f64;
    match o.action {
        StrictAction::Buy | StrictAction::Add => {
            // 建多 / 加多：现金买入，扣费。
            let cost = qty * px * (1.0 + fee_rate);
            if *cash >= cost {
                // 加权平均建仓价。
                let new_units = *units + qty;
                if new_units > 0.0 {
                    *entry_price = (*entry_price * *units + px * qty) / new_units;
                }
                *units = new_units;
                *cash -= cost;
            }
        }
        StrictAction::Sell => {
            // 翻空 / 建空（简化：当前只处理平多→空仓，做空腿待 strategy 订单语义定）。
            close_long(qty, px, fee_rate, cash, units, entry_price, trade_pnls);
        }
        StrictAction::Reduce | StrictAction::Close => {
            // 减多 / 平多：卖出，扣费，记盈亏。
            close_long(qty, px, fee_rate, cash, units, entry_price, trade_pnls);
        }
        StrictAction::Hold | StrictAction::Wait => {
            // 不动。
        }
    }
}

/// 平多（减仓/清仓）：卖 min(qty, 持仓) 股，扣费，记盈亏。
fn close_long(
    qty: f64,
    px: f64,
    fee_rate: f64,
    cash: &mut f64,
    units: &mut f64,
    entry_price: &f64,
    trade_pnls: &mut Vec<f64>,
) {
    let sell = qty.min(*units);
    if sell <= 0.0 {
        return;
    }
    let proceeds = sell * px * (1.0 - fee_rate);
    let cost_basis = sell * *entry_price;
    let pnl = proceeds - cost_basis;
    *cash += proceeds;
    *units -= sell;
    trade_pnls.push(pnl);
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

        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("OKLO", &config).expect("加载 OKLO 真实数据");
        assert!(ds.bars.len() > 300_000, "OKLO 应有 ~343K bar（协议 §1.1）");

        // 切 OOS 窗（协议 §2.1：2023-01-01 → 2025-06-30）。
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
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
}
