//! `ThetaPiStrategy` —— θ 信号（[`ThetaPiStream`]）的 Nautilus Rust-native `Strategy` 桥。
//!
//! ## 存在论位置（#1283 Q2/Q4 裁定：nautilus 链，θ 信号接 Strategy 接口）
//!
//! [`super::strategy::ThetaCore`]（4-adapter 路径）是**旧骨架期核心**；生产 π 回路（#951）的
//! 决策核心是 [`ThetaPiStream`]（`stream.rs`，无条件编译）——它同时是 PyO3/cdylib 出口
//! [`crate::theta_v0::ffi::PyThetaStream`] 的内核。本文件把**同一个** `ThetaPiStream` 包进真实
//! nautilus `StrategyCore + DataActor + Strategy`，让 θ 信号（`push_bar -> p_star`，目标净仓手数）
//! 直接驱动 nautilus 下单（回测 `BacktestEngine` / 实盘 `LiveNode` 同一适配器零代码切换）。
//!
//! 数据流（`on_bar` 每根 bar）：
//! 1. 真实 `&nautilus_model::data::Bar` →[bar_adapter]→ S_Θ `types::Bar`（整数 tick 域）。
//! 2. 从 `self.portfolio()` 读真实净仓 `p_t`（手数，正多/负空/0 平——持仓**数量**真相源 = Nautilus）。
//! 3. `ThetaPiStream::push_bar(bar, p_t, nav) -> p_star`（θ 信号：目标净仓手数）。
//! 4. `delta = round(p_star − p_t)`；`delta ≠ 0` ⟹ 提交再平衡 market 单（`side = sign(delta)`，
//!    `qty = |delta|`）——与 Python 侧 `rec_t_strategy.py:_rebalance` 的
//!    `delta = target − portfolio.net_position` 同口径。
//!
//! ## 与 [`super::theta_strategy::ThetaStrategy`] 的分工
//!
//! - `ThetaStrategy`（`ThetaCore`）：旧 4-adapter 路径，逐 bar 产 `OrderIntent` 列表（含退出生成器）。
//! - `ThetaPiStrategy`（`ThetaPiStream`）：生产 π 回路，逐 bar 产**单一净仓目标** `p_star`，再平衡
//!   delta 提交。两者是**不同决策核**，并存不互兜底——本桥是 #1283 paper 链（Q2/Q4）选定的 θ 信号面。
//!
//! ## ★诚实有效域（no-workaround 标注）
//!
//! - **nav 占位**：`PortfolioApi` 未公开 balance facade（v0.60/0.62 同），`nav` 暂用构造期
//!   初始 NAV 占位（回测起始余额量级）。sizing 基数足够单标的；真实 NAV 读取待 PortfolioApi
//!   公开 balance（与 `theta_strategy.rs` 同一有效域边界，非缺陷）。
//! - **θ 信号非零依赖真实结构**：`p_star ≠ 0` 需要真实 BSP 结构（买卖点）在分类器内形成——合成
//!   单调/锯齿数据产 `p_star = 0`（诚实退化，对齐 `theta_pi_stream.rs` 测试口径，该测试同样
//!   不锁非零订单）。
//!
//! ## 认识论等级（formalization-validity-domain 231号）
//!
//! - 桥结构（trait impl + 数据流接线 + 再平衡映射）= **L0/L1**（编译器类型匹配 + self-check）。
//! - 真实回测产非空订单流 = **L2**（须真实 BTC 数据 + 真实 BacktestEngine，本票未跑，数据卡点）。

use std::fmt::Debug;

use nautilus_common::actor::DataActor;
use nautilus_model::data::Bar as NautilusBar;
use nautilus_model::data::BarType;
use nautilus_model::enums::OrderSide;
use nautilus_model::identifiers::InstrumentId;
use nautilus_model::types::Quantity;
use nautilus_trading::strategy::{Strategy, StrategyConfig, StrategyCore};

use crate::theta_v0::config::ThetaConfig;
use crate::theta_v0::stream::ThetaPiStream;
use crate::theta_v0::types::Bar;

use super::bar_adapter;

/// θ 信号（`p_star`）→ 再平衡 delta（有符号手数；0 = 无再平衡）。
///
/// `p_t` = 调用方真实净仓（手数，正多/负空/0 平）。与 Python 侧
/// `rec_t_strategy.py:_rebalance` 的 `delta = target − portfolio.net_position` 同口径，
/// 且与 [`ThetaPiStream::push_bar`] 的 Schedule_Θ 契约一致（`theta_pi_stream.rs` 测试锁
/// `last_order.qty = |round(p_star − p_t)|`）。
pub(crate) fn rebalance_delta(p_star: f64, p_t: f64) -> i64 {
    (p_star - p_t).round() as i64
}

/// 再平衡 delta → (side, qty)。`delta == 0` ⟹ `None`（不交易）；
/// `delta > 0` ⟹ `Buy |delta|`；`delta < 0` ⟹ `Sell |delta|`。
pub(crate) fn rebalance_order(delta: i64) -> Option<(OrderSide, i64)> {
    if delta == 0 {
        return None;
    }
    let side = if delta > 0 {
        OrderSide::Buy
    } else {
        OrderSide::Sell
    };
    Some((side, delta.unsigned_abs() as i64))
}

/// θ 信号（`p_star`，目标净仓手数）→ 再平衡意图（合成入口，供测试锁映射口径）。
pub(crate) fn rebalance_from_signal(p_star: f64, p_t: f64) -> Option<(OrderSide, i64)> {
    rebalance_order(rebalance_delta(p_star, p_t))
}

/// θ 信号的 Nautilus Rust-native 策略桥（#1283 Q2/Q4）。
///
/// `core`：Nautilus `StrategyCore`（order/portfolio/submit_order 集成入口，宏要求字段名 `core`）。
/// `inner`：`ThetaPiStream`（θ 信号核心，`push_bar -> p_star`）。
pub struct ThetaPiStrategy {
    core: StrategyCore,
    inner: ThetaPiStream,
    instrument_id: InstrumentId,
    bar_type: BarType,
    /// S_Θ 整数 tick 域分母（quantize）；来自 `ThetaConfig.tick.tick_size`。
    tick_size: f64,
    /// 下单数量精度（来自 instrument size_precision，构造 `Quantity` 用）。
    size_precision: u8,
}

impl Debug for ThetaPiStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThetaPiStrategy")
            .field("instrument_id", &self.instrument_id)
            .field("bar_type", &self.bar_type)
            .field("n_bars", &self.inner.bar_count())
            .finish()
    }
}

impl ThetaPiStrategy {
    /// 构造。`config`：Nautilus StrategyConfig（strategy_id/oms_type 等）；`theta`：S_Θ Θ 参数；
    /// `bar_type`：订阅的 bar 类型（须 `AggregationSource::External`，与 add_data 喂的 bar 一致）；
    /// `size_precision`：instrument 数量精度（构造下单 `Quantity` 用）。
    #[must_use]
    pub fn new(
        config: StrategyConfig,
        theta: ThetaConfig,
        bar_type: BarType,
        size_precision: u8,
    ) -> Self {
        let tick_size = theta.tick.tick_size;
        let instrument_id = bar_type.instrument_id();
        Self {
            core: StrategyCore::new(config),
            inner: ThetaPiStream::new(theta),
            instrument_id,
            bar_type,
            tick_size,
            size_precision,
        }
    }

    /// 决策步（θ 信号核心，可脱离引擎测试）：S_Θ bar + 当前净仓 + NAV → (side, qty) 或 None。
    ///
    /// 与 `on_bar` 同口径：`ThetaPiStream::push_bar` 产 `p_star` → `rebalance_from_signal`。
    /// 本方法只触 `self.inner`（θ 核心），不碰 portfolio/order ⟹ 无需引擎注册即可测。
    fn decide(&mut self, s_bar: Bar, p_t: f64, nav: f64) -> Option<(OrderSide, i64)> {
        let p_star = self.inner.push_bar(s_bar, p_t, nav);
        rebalance_from_signal(p_star, p_t)
    }

    /// 从 Nautilus portfolio 读真实净仓（手数，正多/负空/0 平）——持仓**数量**真相源 = Nautilus。
    fn read_net_position(&self) -> f64 {
        decimal_to_f64(self.portfolio().net_position(&self.instrument_id))
    }

    /// 提交再平衡 market 单（`qty = |delta|`，`side = sign(delta)`；非 reduce-only——再平衡可翻仓）。
    fn submit_rebalance(&mut self, side: OrderSide, qty: i64) -> anyhow::Result<()> {
        let quantity = Quantity::new(qty as f64, self.size_precision);
        let order = self.order().market(
            self.instrument_id,
            side,
            quantity,
            None, // time_in_force（默认 GTC）
            None, // reduce_only：再平衡允许翻仓（多↔空净仓调整），非 reduce-only
            None, // quote_quantity
            None, // exec_algorithm_id
            None, // exec_algorithm_params
            None, // tags
            None, // client_order_id（自动生成）
        );
        self.submit_order(order, None, None, None)
    }
}

// 宏 impl DataActorNative + StrategyNative + Strategy。θ 信号桥无持仓台账（p_t 每 bar 从
// portfolio 读，持仓真相源唯一 = Nautilus）⟹ 无需 on_position_closed 钩子。
nautilus_trading::nautilus_strategy!(ThetaPiStrategy);

impl DataActor for ThetaPiStrategy {
    fn on_start(&mut self) -> anyhow::Result<()> {
        self.subscribe_bars(self.bar_type, None, None);
        Ok(())
    }

    fn on_bar(&mut self, bar: &NautilusBar) -> anyhow::Result<()> {
        // 1. 真实 Bar → S_Θ types::Bar（source_index = 已消费 bar 数，单调递增）。
        let source_index = self.inner.bar_count();
        let s_bar = bar_adapter::from_real_bar(bar, source_index, self.tick_size);

        // 2. 真实净仓 p_t（持仓数量真相源 = Nautilus）。
        let p_t = self.read_net_position();

        // 3. θ 信号：push_bar → p_star；再平衡 delta → (side, qty)。
        if let Some((side, qty)) = self.decide(s_bar, p_t, INITIAL_NAV_PLACEHOLDER) {
            // 4. 提交再平衡 market 单。
            self.submit_rebalance(side, qty)?;
        }
        Ok(())
    }
}

/// 初始 NAV 占位（`decide`/`on_bar` 的 nav 有效域边界，见模块头）。
/// `PortfolioApi` 未公开 balance facade ⟹ 用构造期初始 NAV 量级近似；sizing 基数恒定足够单标的。
const INITIAL_NAV_PLACEHOLDER: f64 = 1_000_000.0;

/// `rust_decimal::Decimal` → f64（net_position 转换）。
fn decimal_to_f64(d: rust_decimal::Decimal) -> f64 {
    use std::str::FromStr;
    // Decimal 无 lossless f64，用 to_string→parse（精度足够手数判定）。
    f64::from_str(&d.to_string()).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nautilus_model::data::BarSpecification;
    use nautilus_model::enums::{AggregationSource, BarAggregation, PriceType};

    /// 构造 BTC External 1-Last bar_type（与 backtest_engine.rs 同口径，供构造策略）。
    fn btc_bar_type() -> BarType {
        let spec = BarSpecification::new(1, BarAggregation::Minute, PriceType::Last);
        BarType::new(
            InstrumentId::from("BTCUSDT.BINANCE"),
            spec,
            AggregationSource::External,
        )
    }

    /// 合成锯齿 bar（OHLC 完整、量 > 0、可交易）。无缠论结构 ⟹ θ 信号 p_star=0（诚实退化）。
    fn synthetic_bars() -> Vec<Bar> {
        let mut bars = Vec::with_capacity(80);
        let mut close: i64 = 100_000_000;
        let mut idx = 0usize;
        for swing in 0..16usize {
            let dir: i64 = if swing % 2 == 0 { 1 } else { -1 };
            for _step in 0..5usize {
                let open = close;
                close += dir * 500;
                let high = open.max(close) + 100;
                let low = open.min(close) - 100;
                bars.push(Bar {
                    source_index: idx,
                    timestamp: idx as i64,
                    open,
                    high,
                    low,
                    close,
                    volume: 1000.0,
                    untradable: false,
                });
                idx += 1;
            }
        }
        bars
    }

    /// θ 信号 → 再平衡映射（L0/L1 接线锁）：`p_star` 与 `p_t` 的差直接决定 side/qty——
    /// 这是「Strategy 收到 θ 信号」后唯一的下单语义。
    #[test]
    fn theta_signal_maps_to_rebalance_order() {
        // p_star=5（目标净多 5），p_t=0（空仓）⟹ Buy 5。
        assert_eq!(rebalance_from_signal(5.0, 0.0), Some((OrderSide::Buy, 5)));
        // p_star=-3（目标净空 3），p_t=0 ⟹ Sell 3。
        assert_eq!(rebalance_from_signal(-3.0, 0.0), Some((OrderSide::Sell, 3)));
        // p_star=2，p_t=5（现持多 5）⟹ 减 3（Sell 3，翻仓向目标靠）。
        assert_eq!(rebalance_from_signal(2.0, 5.0), Some((OrderSide::Sell, 3)));
        // p_star=p_t ⟹ 无再平衡。
        assert_eq!(rebalance_from_signal(4.0, 4.0), None);
        // 小数对齐：round(2.6)=3。
        assert_eq!(rebalance_from_signal(2.6, 0.0), Some((OrderSide::Buy, 3)));
    }

    /// 策略决策步（`decide`）收到 θ 信号：每根 bar 都推进 θ 核心（`bar_count` 递增），
    /// 且 `decide` 输出恒等于 θ 信号映射 `rebalance_from_signal(push_bar(...))`。
    ///
    /// ★诚实：合成锯齿数据无缠论结构 ⟹ `p_star` 恒 0 ⟹ 每根 bar 返回 None（诚实退化）——
    /// 非零信号的映射口径已由 [`theta_signal_maps_to_rebalance_order`] 独立锁死。
    #[test]
    fn strategy_decision_receives_theta_signal() {
        let mut strat = ThetaPiStrategy::new(
            StrategyConfig::default(),
            ThetaConfig::default(),
            btc_bar_type(),
            6,
        );
        let bars = synthetic_bars();
        for (i, bar) in bars.iter().enumerate() {
            let decision = strat.decide(*bar, 0.0, INITIAL_NAV_PLACEHOLDER);
            // 决策步推进 θ 核心：bar_count == i+1（每根 bar 都到达 ThetaPiStream）。
            assert_eq!(strat.inner.bar_count(), i + 1, "bar {i} 未推进 θ 核心");
            // 输出恒等于 θ 信号映射（p_star 由 push_bar 内部算出；合成数据 p_star=0 ⟹ None）。
            assert_eq!(
                decision, None,
                "合成无结构数据 ⟹ p_star=0 ⟹ 无再平衡（诚实退化）"
            );
        }
        // 全部 80 根 bar 到达 θ 核心。
        assert_eq!(strat.inner.bar_count(), bars.len());
    }
}
