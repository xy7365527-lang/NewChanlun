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

use crate::funding_record::{FundingError, FundingRecord};
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
    /// sizing 基数：ADR 0040 funding-record 的 `Q_total`（#1601 C-02，取代 1e6 占位）。
    nav: f64,
    /// 所读 funding-record 的 id 与版本（`label()`）。
    funding_record: String,
}

impl Debug for ThetaStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThetaStrategy")
            .field("instrument_id", &self.instrument_id)
            .field("bar_type", &self.bar_type)
            .field("funding_record", &self.funding_record)
            .field("n_bars", &self.inner.bars.len())
            .finish()
    }
}

impl ThetaStrategy {
    /// 构造。`config`：Nautilus StrategyConfig（strategy_id/oms_type 等）；`theta`：S_Θ Θ 参数；
    /// `bar_type`：订阅的 bar 类型（须 `AggregationSource::External`，与 add_data 喂的 bar 一致）；
    /// `size_precision`：instrument 数量精度（构造下单 Quantity 用）。
    ///
    /// `funding`：ADR 0040 funding-record；sizing 基数读其 `Q_total`。缺记录 ⟹
    /// `AwaitingPolicy{funding_record}`，`evidence_only` ⟹ `Invalid{scope}`——都不构造策略、不下单（#1601）。
    ///
    /// ★`inner = ThetaCore::new(theta)` 继承 #7 的生产 `entry_delay_bars`（CLI 须设 0，见 task#8）。
    pub fn new(
        config: StrategyConfig,
        theta: ThetaConfig,
        bar_type: nautilus_model::data::BarType,
        size_precision: u8,
        funding: Option<FundingRecord>,
    ) -> Result<Self, FundingError> {
        let funding = FundingRecord::admit_production(funding)?;
        let tick_size = theta.tick.tick_size;
        let instrument_id = bar_type.instrument_id();
        Ok(Self {
            core: StrategyCore::new(config),
            inner: ThetaCore::new(theta),
            instrument_id,
            bar_type,
            tick_size,
            size_precision,
            nav: funding.q_total_f64(),
            funding_record: funding.label(),
        })
    }

    /// 本策略读的 funding-record id 与版本。
    pub fn funding_record(&self) -> &str {
        &self.funding_record
    }

    /// 从 Nautilus portfolio 读真实持仓快照（持仓**数量**真相源 = Nautilus，消双源）。
    ///
    /// `nav`：账户净值（sizing 基数）；`net_position`：净仓手数（正多/负空/0平）；
    /// `unrealized_pnl`：mark-to-market 浮盈（equity = nav + unrealized，对齐 Origin.TotalWealth）。
    ///
    /// ★有效域（no-workaround）：v0.60.0 PortfolioApi 暴露 `net_position(&id) -> Decimal`。
    /// nav/PnL 的 Money 读取在 PortfolioApi 当前为 `pub(crate)`（balances facade 未公开），故 nav
    /// 暂用构造期 funding-record 的 `Q_total`（回测起始余额同源）——这是**有效域边界**（单标的回测起始 NAV 恒定，
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
            None,                     // time_in_force（默认 GTC）
            Some(intent.reduce_only), // reduce_only：退出/减仓 = true
            None,                     // quote_quantity
            None,                     // exec_algorithm_id
            None,                     // exec_algorithm_params
            None,                     // tags
            None,                     // client_order_id（自动生成）
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

        // 2. portfolio 快照（持仓数量真相源 = Nautilus）。nav 近似见 read_portfolio_snapshot。
        let snap = self.read_portfolio_snapshot(self.nav);

        // 3. ThetaCore 退出先于开仓（#7）→ OrderIntent 列表。
        let intents = self.inner.plan_for_bar(s_bar, &snap);

        // 4. 逐意图提交真实 market 单（⑤ 裁决）。
        for intent in &intents {
            self.submit_intent(intent)?;
        }
        Ok(())
    }
}

/// `rust_decimal::Decimal` → f64（net_position 转换）。
fn decimal_to_f64(d: rust_decimal::Decimal) -> f64 {
    use std::str::FromStr;
    // Decimal 无 lossless f64，用 to_string→parse（精度足够手数判定）。
    f64::from_str(&d.to_string()).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nautilus_model::data::{BarSpecification, BarType};
    use nautilus_model::enums::{AggregationSource, BarAggregation, PriceType};
    use nautilus_model::identifiers::InstrumentId;

    fn btc_bar_type() -> BarType {
        let spec = BarSpecification::new(1, BarAggregation::Minute, PriceType::Last);
        BarType::new(
            InstrumentId::from("BTCUSDT.BINANCE"),
            spec,
            AggregationSource::External,
        )
    }

    fn build(funding: Option<FundingRecord>) -> Result<ThetaStrategy, FundingError> {
        ThetaStrategy::new(
            StrategyConfig::default(),
            ThetaConfig::default(),
            btc_bar_type(),
            6,
            funding,
        )
    }

    /// #1601 C-02：sizing 基数 = 记录的 `Q_total`；换 profile 即换基数（改回常数的变异体在此变红）。
    #[test]
    fn nav_reads_funding_record_q_total() {
        for (profile, q) in [("P0", 100_000.0), ("P1-hl-testnet", 500.0)] {
            let strat = build(Some(FundingRecord::active(profile).unwrap())).unwrap();
            assert_eq!(strat.nav, q, "{profile}");
            assert_eq!(
                strat.funding_record(),
                format!("#1557/funding-authorization/v1@{profile}/v1")
            );
        }
    }

    /// 负控：缺记录 ⟹ AwaitingPolicy{funding_record}；evidence_only ⟹ 拒入生产。都不构造策略。
    #[test]
    fn missing_or_evidence_only_record_refuses_to_build() {
        match build(None) {
            Err(FundingError::AwaitingPolicy { slot, .. }) => assert_eq!(slot, "funding_record"),
            other => panic!("缺记录须 AwaitingPolicy，得 {other:?}"),
        }
        let acceptance = FundingRecord::active("P0-acceptance-5m").unwrap();
        assert!(matches!(
            build(Some(acceptance)),
            Err(FundingError::Invalid { ref field, .. }) if field == "scope"
        ));
    }
}
