//! `ThetaStrategy` Nautilus Rust-native 适配器骨架（串 4 个 adapter + 退出生成器接入点）。
//!
//! ## 职责（设计文档 §4.1-4.2 数据流）
//!
//! Nautilus Rust-native `Strategy`（`StrategyCore` + `DataActor`）的 `on_bar` 中：
//! 1. [`super::bar_adapter`]：Nautilus Bar → S_Θ `types::Bar`，追加到累积窗口。
//! 2. S_Θ 管线：`parse_layer → classify → recognize → plan_orders`（`StrategyFamily::pi`）。
//! 3. [`super::account_adapter`]：Nautilus portfolio → S_Θ `AccountState`（sizing 用真实账户）。
//! 4. [`super::order_adapter`]：S_Θ `Order` → Nautilus 下单意图 → `order_factory` → `submit_order`。
//! 5. 退出生成器接入点：§9 closePred（止损/反向 BSP/RiskClose）触发 → Close 订单走 Nautilus。
//!
//! ## ★骨架（nautilus 依赖未加，不编译）：真实 Strategy trait 以注释 + TODO 锚定。
//!
//! 真实结构（context7 `write_rust_strategy.md`，待依赖后兑现）：
//! ```ignore
//! use nautilus_common::actor::DataActor;
//! use nautilus_model::data::Bar;
//! use nautilus_trading::{nautilus_strategy, strategy::StrategyCore};
//!
//! pub struct ThetaStrategy {
//!     core: StrategyCore,              // order_factory + portfolio 集成
//!     instrument_id: InstrumentId,
//!     bar_type: BarType,
//!     config: ThetaConfig,             // S_Θ 参数（Param 索引族成员）
//!     bars: Vec<theta_v0::types::Bar>, // 累积窗口
//! }
//! nautilus_strategy!(ThetaStrategy);
//! impl DataActor for ThetaStrategy {
//!     fn on_start(&mut self) -> anyhow::Result<()> { self.subscribe_bars(self.bar_type); Ok(()) }
//!     fn on_bar(&mut self, bar: &Bar) -> anyhow::Result<()> { self.on_bar_inner(bar)?; Ok(()) }
//!     fn on_order_filled(&mut self, e: &OrderFilled) -> anyhow::Result<()> { /* 对账 */ Ok(()) }
//!     fn on_position_closed(&mut self, e: &PositionClosed) -> anyhow::Result<()> { /* 清台账 */ Ok(()) }
//! }
//! ```

use crate::theta_v0::classifier;
use crate::theta_v0::config::ThetaConfig;
use crate::theta_v0::parser;
use crate::theta_v0::strategy::{self, AccountState};
use crate::theta_v0::types::{Bar, Order};

use super::account_adapter::{self, PortfolioSnapshot};
use super::order_adapter::{self, OrderIntent};

/// `ThetaStrategy` 适配器的 S_Θ 侧核心状态（与 Nautilus `StrategyCore` 组合）。
///
/// ★骨架：依赖加入后，把此结构嵌入真实 `ThetaStrategy { core: StrategyCore, inner: ThetaCore }`，
/// `on_bar` 调 [`ThetaCore::plan_for_bar`]，再用返回的 `OrderIntent` 调 `core.order_factory()`（TODO）。
#[derive(Debug, Clone)]
pub struct ThetaCore {
    /// S_Θ 参数（Param 索引策略族成员，`StrategyFamily::pi` 的 param）。
    pub config: ThetaConfig,
    /// 累积 bar 窗口（source_index = 窗口序号；S_Θ 管线吃完整窗口，非单 bar）。
    pub bars: Vec<Bar>,
}

impl ThetaCore {
    /// 构造（给定 Θ config，空窗口）。
    pub fn new(config: ThetaConfig) -> Self {
        ThetaCore { config, bars: Vec::new() }
    }

    /// **每 bar 决策（适配层核心，设计文档 §4.2 数据流）**。
    ///
    /// 1. 追加新 bar（已由 [`super::bar_adapter::from_nautilus_bar`] 转换为 S_Θ `types::Bar`，
    ///    `source_index = self.bars.len()`）。
    /// 2. S_Θ 管线：`parse_layer → classify → StrategyFamily::pi(config, cls, bars, account)`
    ///    （= recognize + plan_orders，产唯一订单流）。
    /// 3. 每个 S_Θ `Order` → [`OrderIntent`]（order_adapter，持仓方向从 portfolio 快照推）。
    ///
    /// 返回本 bar 的下单意图列表（调用方 strategy.on_bar 用 `core.order_factory()` 提交，TODO）。
    ///
    /// ## ★诚实有效域（设计文档 §4.4）
    ///
    /// - 持仓真相源 = Nautilus portfolio（`snap`）——sizing 用真实账户 NAV+净仓，**不用** S_Θ
    ///   `plan_and_fill_mtm` 的内部模拟台账（生产路径只用 recognize+plan_orders 产订单，fill/equity
    ///   由 Nautilus venue 撮合，见设计文档 §4.4）。
    /// - **退出决策生成器**（runner.rs `exit_decision_for`，§9 closePred）的缠论触发逻辑在生产路径
    ///   需移植到此处（持仓 + 当前 bar → 止损/反向 BSP/RiskClose → Close 决策）——本骨架**标接入点**
    ///   （TODO），不内联实现（避免与 runner 双源；移植时复用 runner 的 `exit_decision_for` 逻辑，
    ///   持仓从 `snap` 读而非内部台账）。这是 no-patch 的「声明边界」：骨架不假装退出逻辑已就位。
    ///
    /// 边界条件：bars 不足以产生结构（< min_parts_per_level）⟹ classify 产空 ⟹ 无决策 ⟹ 空意图
    /// （诚实退化，对齐 runner.rs `structureless_data_yields_empty_orders`）。
    pub fn plan_for_bar(&mut self, new_bar: Bar, snap: &PortfolioSnapshot) -> Vec<OrderIntent> {
        self.bars.push(new_bar);

        // S_Θ 管线（开仓侧订单）。account 从 Nautilus portfolio（真实持仓真相源）。
        let account: AccountState =
            account_adapter::to_account_state(snap, self.config.voice.max_depth);
        let orders = self.plan_orders(&account);

        // 退出决策生成器接入点（TODO，设计文档 §4.4）：
        //   let exit_orders = self.generate_exits(snap, &new_bar);  // §9 closePred，持仓从 snap 读
        //   orders.extend(exit_orders);
        // 本骨架不内联（避免与 runner::exit_decision_for 双源；移植时单源化）。

        // S_Θ Order → Nautilus 下单意图。
        let pos_dir = account_adapter::position_dir(snap);
        orders
            .iter()
            .filter_map(|o| order_adapter::to_order_intent(o, pos_dir, self.entry_tick_for(o)))
            .collect()
    }

    /// S_Θ 开仓侧订单流（`StrategyFamily::pi` = recognize + plan_orders）。
    ///
    /// ★诚实：`StrategyFamily::pi` 吃 `Classification`——本函数串 `parse_layer → classify → pi`
    /// （`pi` 内部再 recognize）。等价于 `run_theta_v0` 的步骤 1-3（不含 plan_and_fill_mtm 的 fill）。
    fn plan_orders(&self, account: &AccountState) -> Vec<Order> {
        let l0 = parser::parse_layer(&self.bars, &self.config);
        let classification = classifier::classify(&l0, &self.config);
        strategy::StrategyFamily::family().pi(&self.config, &classification, &self.bars, account)
    }

    /// 取订单对应的限价 tick（开仓限价腿）。
    ///
    /// ★骨架占位：S_Θ `Order` 不直接携带 entry 价（entry 在 `VoiceDecision` 中，plan_orders 已消费）。
    /// 生产路径需让 `plan_orders` 或本适配层保留 decision→order 的 entry 映射（TODO）。当前骨架返回
    /// `None`（市价腿）——限价腿的 entry 价回填待 plan_orders 接口扩展（报 theta_v0 owner，非本工位改）。
    fn entry_tick_for(&self, _order: &Order) -> Option<crate::theta_v0::types::Tick> {
        None // TODO: 限价腿 entry 价回填（依赖 plan_orders 暴露 decision→order entry 映射）
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::types::Bar;

    fn mk_bar(idx: usize, close: i64) -> Bar {
        Bar {
            source_index: idx,
            timestamp: idx as i64,
            open: close,
            high: close,
            low: close,
            close,
            volume: 100,
            untradable: false,
        }
    }

    fn flat_snap() -> PortfolioSnapshot {
        PortfolioSnapshot { nav: 1_000_000.0, net_position: 0.0, realized_pnl: 0.0, unrealized_pnl: 0.0 }
    }

    /// L0 骨架退化验证：无结构 bar 序列 ⟹ 空下单意图（对齐 runner 诚实退化）。
    ///
    /// ★认识论 L0（合成单调 bar，验证适配管线串通 + 正确退化，零信息增量）。这**不是** L2
    /// （L2 需真实数据 + 真实 Nautilus 引擎，依赖未加）。
    #[test]
    fn structureless_bars_yield_no_intents() {
        let mut core = ThetaCore::new(ThetaConfig::default());
        let snap = flat_snap();
        // 单调上涨（无顶底交替 ⟹ 无缠论结构 ⟹ 无买卖点 ⟹ 无订单）。
        let mut intents = Vec::new();
        for i in 0..50 {
            intents = core.plan_for_bar(mk_bar(i, 1000 + i as i64), &snap);
        }
        assert!(intents.is_empty(), "单调数据无结构 ⟹ 空下单意图（诚实退化，非缺陷）");
        assert_eq!(core.bars.len(), 50, "bar 窗口累积到 50");
    }

    /// 窗口累积：每 bar 追加，source_index 单调。
    #[test]
    fn bars_accumulate_with_monotone_index() {
        let mut core = ThetaCore::new(ThetaConfig::default());
        let snap = flat_snap();
        core.plan_for_bar(mk_bar(0, 1000), &snap);
        core.plan_for_bar(mk_bar(1, 1010), &snap);
        assert_eq!(core.bars.len(), 2);
        assert_eq!(core.bars[0].source_index, 0);
        assert_eq!(core.bars[1].source_index, 1);
    }
}
