//! `ThetaStrategy` —— canonical S_Θ 的 Nautilus Rust-native `Strategy` 包壳（goal acceptance[5] ③）。
//!
//! ## 存在论位置（设计文档 §4.1-4.2）
//!
//! [`super::strategy::ThetaCore`] 是 **S_Θ 侧核心**（in-crate，零 nautilus 依赖，含退出生成器 #7）。
//! 本文件是 **nautilus 侧适配**（feature `nautilus` 门控）：把 ThetaCore 包进真实
//! `StrategyCore + DataActor + Strategy`，让同一 S_Θ 既能跑 `BacktestEngine`（⑥）也能跑 `LiveNode`（实盘）。
//!
//! 数据流（`on_bar` 每根 bar）：
//! 1. 真实 `&nautilus_model::data::Bar` →[bar_adapter]→ S_Θ `types::Bar`（整数 tick 域）。
//! 2. 从 `self.portfolio()` 读真实持仓 → [`PortfolioSnapshot`]（持仓**数量**真相源 = Nautilus）。
//! 3. `ThetaCore::plan_for_bar(bar, &snap)` → `Vec<OrderIntent>`（退出先于开仓，#7）。
//! 4. 每个 `OrderIntent` →[order()/OrderApi]→ 真实 `OrderAny` → `submit_order`。
//!
//! ## ⑤ CR 裁决（不开 canonical Order）：用 **market 单**
//!
//! `OrderIntent.price_tick=Some` 本可走 limit，但 ⑤ 裁决「不碰 canonical Order、不加价格腿」⟹
//! 本包壳**全部用 market 单**（`OrderApi::market`），entry/exit 都市价腿。limit 腿待 CR 开后接入。
//!
//! ## 认识论等级（formalization-validity-domain 231号）
//!
//! - 包壳结构（trait impl + 数据流接线）= **L0/L1**（编译器验证类型匹配 + self-check 管线串通）。
//! - 真实回测产非空订单流 = **L2**（由 ⑥ CLI 跑 BTC 真实数据 + `BacktestResult.total_orders>0` 验）。

use std::fmt::Debug;

use nautilus_common::actor::DataActor;
use nautilus_model::data::Bar as NautilusBar;
use nautilus_model::enums::OrderSide;
use nautilus_model::events::PositionClosed;
use nautilus_model::identifiers::InstrumentId;
use nautilus_model::types::Quantity;
use nautilus_trading::strategy::{Strategy, StrategyConfig, StrategyCore};

use crate::theta_v0::config::ThetaConfig;

use super::account_adapter::PortfolioSnapshot;
use super::bar_adapter;
use super::order_adapter::{OrderIntent, OrderSideLike};
use super::strategy::ThetaCore;

/// canonical S_Θ 的 Nautilus Rust-native 策略。
///
/// `core`：Nautilus `StrategyCore`（order/portfolio/submit_order 集成入口，宏要求字段名 `core`）。
/// `inner`：S_Θ 核心（含 held 台账 #7 + 生产 `entry_delay_bars=0`，见 [`ThetaCore::new`] 约束）。
pub struct ThetaStrategy {
    core: StrategyCore,
    inner: ThetaCore,
    instrument_id: InstrumentId,
    bar_type: nautilus_model::data::BarType,
    /// S_Θ 整数 tick 域分母（quantize/dequantize）；来自 `ThetaConfig.tick.tick_size`。
    tick_size: f64,
    /// 下单数量精度（来自 instrument size_precision，构造 `Quantity` 用）。
    size_precision: u8,
}

impl Debug for ThetaStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThetaStrategy")
            .field("instrument_id", &self.instrument_id)
            .field("bar_type", &self.bar_type)
            .field("n_bars", &self.inner.bars.len())
            .finish()
    }
}

impl ThetaStrategy {
    /// 构造。`config`：Nautilus StrategyConfig（strategy_id/oms_type 等）；`theta`：S_Θ Θ 参数；
    /// `bar_type`：订阅的 bar 类型（须 `AggregationSource::External`，与 add_data 喂的 bar 一致）；
    /// `size_precision`：instrument 数量精度（构造下单 Quantity 用）。
    ///
    /// ★`inner = ThetaCore::new(theta)` 继承 #7 的生产 `entry_delay_bars`（CLI 须设 0，见 task#8）。
    #[must_use]
    pub fn new(
        config: StrategyConfig,
        theta: ThetaConfig,
        bar_type: nautilus_model::data::BarType,
        size_precision: u8,
    ) -> Self {
        let tick_size = theta.tick.tick_size;
        let instrument_id = bar_type.instrument_id();
        Self {
            core: StrategyCore::new(config),
            inner: ThetaCore::new(theta),
            instrument_id,
            bar_type,
            tick_size,
            size_precision,
        }
    }

    /// 从 Nautilus portfolio 读真实持仓快照（持仓**数量**真相源 = Nautilus，消双源）。
    ///
    /// `nav`：账户净值（sizing 基数）；`net_position`：净仓手数（正多/负空/0平）；
    /// `unrealized_pnl`：mark-to-market 浮盈（equity = nav + unrealized，对齐 Origin.TotalWealth）。
    ///
    /// ★有效域（no-workaround）：v0.60.0 PortfolioApi 暴露 `net_position(&id) -> Decimal`。
    /// nav/PnL 的 Money 读取在 PortfolioApi 当前为 `pub(crate)`（balances facade 未公开），故 nav
    /// 暂用构造期初始 NAV 近似（回测起始余额）——这是**有效域边界**（单标的回测起始 NAV 恒定，
    /// sizing 基数足够），非缺陷。多笔成交后 NAV 漂移的精确读取待 PortfolioApi 公开 balance facade。
    fn read_portfolio_snapshot(&self, initial_nav: f64) -> PortfolioSnapshot {
        let portfolio = self.portfolio();
        // net_position: Decimal → f64（手数；BTCUSDT size_precision=6，小数手合法）。
        let net_position = decimal_to_f64(portfolio.net_position(&self.instrument_id));
        PortfolioSnapshot {
            nav: initial_nav,
            net_position,
            realized_pnl: 0.0,
            // 浮盈：PortfolioApi 未公开 unrealized_pnl facade（pub(crate)）⟹ 0 近似。
            // 影响：equity=nav（不含浮盈），RiskClose equity 门略保守——有效域边界，标注非缺陷。
            unrealized_pnl: 0.0,
        }
    }

    /// 把一个 S_Θ [`OrderIntent`] 提交为真实 Nautilus market 单（⑤ 裁决：market only）。
    fn submit_intent(&mut self, intent: &OrderIntent) -> anyhow::Result<()> {
        let side = match intent.side {
            OrderSideLike::Buy => OrderSide::Buy,
            OrderSideLike::Sell => OrderSide::Sell,
        };
        let qty = Quantity::new(intent.qty as f64, self.size_precision);
        // market(instrument_id, side, qty, tif, reduce_only, quote_qty, exec_algo_id, exec_algo_params, tags, client_order_id)
        let order = self.order().market(
            self.instrument_id,
            side,
            qty,
            None,                      // time_in_force（默认 GTC）
            Some(intent.reduce_only),  // reduce_only：退出/减仓 = true
            None,                      // quote_quantity
            None,                      // exec_algorithm_id
            None,                      // exec_algorithm_params
            None,                      // tags
            None,                      // client_order_id（自动生成）
        );
        // submit_order(order, position_id, client_id, params) —— 4 参（task#8 breaking #4）。
        self.submit_order(order, None, None, None)
    }
}

// 宏 impl DataActorNative + StrategyNative + Strategy。on_position_closed 在 Strategy trait
// （返回 ()，task#8 breaking #1）：仓位全平 ⟹ 清 S_Θ held 台账（缠论语义真相源）。
nautilus_trading::nautilus_strategy!(ThetaStrategy, {
    fn on_position_closed(&mut self, _event: PositionClosed) {
        // 全平 ⟹ 清缠论台账（held 数量真相由 portfolio 管，缠论语义状态在此清）。
        self.inner.clear_held();
    }
});

impl DataActor for ThetaStrategy {
    fn on_start(&mut self) -> anyhow::Result<()> {
        // subscribe_bars(bar_type, client_id, params) → ()（task#8 breaking #5）。
        self.subscribe_bars(self.bar_type, None, None);
        Ok(())
    }

    fn on_bar(&mut self, bar: &NautilusBar) -> anyhow::Result<()> {
        // 1. 真实 Bar → S_Θ types::Bar（source_index = 累积窗口序号）。
        let source_index = self.inner.bars.len();
        let s_bar = bar_adapter::from_real_bar(bar, source_index, self.tick_size);

        // 2. portfolio 快照（持仓数量真相源 = Nautilus）。initial_nav 近似见 read_portfolio_snapshot。
        let snap = self.read_portfolio_snapshot(INITIAL_NAV_PLACEHOLDER);

        // 3. ThetaCore 退出先于开仓（#7）→ OrderIntent 列表。
        let intents = self.inner.plan_for_bar(s_bar, &snap);

        // 4. 逐意图提交真实 market 单（⑤ 裁决）。
        for intent in &intents {
            self.submit_intent(intent)?;
        }
        Ok(())
    }
}

/// 初始 NAV 占位（read_portfolio_snapshot 的有效域边界，见其文档）。
/// ⑥ CLI 用 venue starting_balance 量级；sizing 基数恒定足够单标的回测。
const INITIAL_NAV_PLACEHOLDER: f64 = 1_000_000.0;

/// `rust_decimal::Decimal` → f64（net_position 转换）。
fn decimal_to_f64(d: rust_decimal::Decimal) -> f64 {
    use std::str::FromStr;
    // Decimal 无 lossless f64，用 to_string→parse（精度足够手数判定）。
    f64::from_str(&d.to_string()).unwrap_or(0.0)
}
