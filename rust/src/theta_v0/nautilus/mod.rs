//! # canonical S_Θ ↔ Nautilus Trader 适配层（goal `g-l2-nautilus-production` 第 2 段；订正 #524：已过骨架期）
//!
//! 把 `theta_v0` 的 canonical S_Θ（`Origin.StrategyFamily.piTheta` bit-exact 实装）接入
//! Nautilus Trader 的 Rust-native Strategy，使 S_Θ 成生产引擎（回测 `BacktestNode` + 实盘
//! `LiveNode` + IB adapter，同一适配器零代码切换）。
//!
//! ## 设计文档
//!
//! 完整调研 + 架构 + 接口映射 + 依赖方案见 `docs/nautilus-integration-design.md`。
//!
//! ## ★实装状态（订正 #524：原「骨架状态」段过期作废——三项自称均已为假）
//!
//! 本模块**已过骨架期、是实装适配层**：
//! - **本模块编译进 crate**：`theta_v0/mod.rs` 无条件 `pub mod nautilus;` 注册。
//! - **nautilus 依赖已入 Cargo.toml**：`nautilus-model/common/trading/backtest/core` v0.60.0
//!   （optional，`nautilus` feature 门控；`backtest_bin` 含之）。路径已定 Rust-native
//!   （`IntegrationPath` 占位 enum 仅作设计文档锚点，非运行时分支）。
//! - **真实 `use nautilus_*` 已在产**：`theta_strategy.rs`（feature `nautilus` 门控，真实
//!   `StrategyCore + DataActor + Strategy`）与 `backtest_engine.rs`（`all(nautilus, backtest_bin)`
//!   门控，真实 BacktestEngine 驱动，task#8 acceptance[5]）。
//! - **不声明盈利性**：声明管线贯通与增量等价，**不声明** S_Θ 接 Nautilus 后回测有效/盈利
//!   （编排者纲领「不证明 Θ 是好 Θ」）。
//!
//! ## 认识论等级（formalization-validity-domain 231号）
//!
//! - 适配架构 + 接口映射设计 = **L0**（从双侧真实接口推导的结构，零数据）。
//! - quantize 往返一致性 / 订单映射正确性 = **L1**（骨架期 13 个 self-check 在跑）。
//! - 真实集成跑通 = **L2**（已落地：task#8 真实 BacktestEngine 非空订单流；#333 60k 窗口
//!   两口径逐位一致；#345 增量分类接入，100k bar 0.47s）。
//!   本模块**不声称** S_Θ 接 Nautilus 后回测有效/盈利（编排者纲领「不证明 Θ 是好 Θ」）。
//!
//! ## 适配层子模块拓扑（数据流见设计文档 §4.2）
//!
//! ```text
//! Nautilus Bar ──[bar_adapter]──► theta_v0::types::Bar
//!   ──► (累积窗口) parse_layer → classify → recognize → plan_orders ──► theta_v0::Order
//!   ──[order_adapter]──► Nautilus order_factory().limit/market/stop ──► submit_order
//! Nautilus portfolio ──[account_adapter]──► theta_v0::strategy::AccountState
//! ```
//!
//! - [`bar_adapter`]：Nautilus `Bar`（f64 Price）↔ S_Θ `types::Bar`（i64 Tick，整数 tick 域）。
//! - [`order_adapter`]：S_Θ `Order`（`StrictAction`）↔ Nautilus `OrderSide` + `order_factory`。
//! - [`account_adapter`]：Nautilus `portfolio`（net_position/PnL）↔ S_Θ `AccountState`（NAV+voice_qty）。
//! - [`strategy`]：`ThetaStrategy` 适配器骨架（`on_bar` 串 4 个 adapter + 退出生成器接入点）。
//!
//! ## Rust-native Strategy 真实接口锚（context7 `write_rust_strategy.md`；已在 `theta_strategy.rs` 兑现，订正 #524）
//!
//! ```ignore
//! use nautilus_common::actor::DataActor;
//! use nautilus_trading::{nautilus_strategy, strategy::{Strategy, StrategyConfig, StrategyCore}};
//!
//! pub struct ThetaStrategy { core: StrategyCore, /* + S_Θ 状态 */ }
//! nautilus_strategy!(ThetaStrategy);     // 宏生成 Deref + Strategy trait
//! impl DataActor for ThetaStrategy {
//!     fn on_start(&mut self) -> anyhow::Result<()> { self.subscribe_bars(self.bar_type); Ok(()) }
//!     fn on_bar(&mut self, bar: &Bar) -> anyhow::Result<()> { /* 串 4 adapter */ Ok(()) }
//! }
//! ```

pub mod account_adapter;
pub mod bar_adapter;
pub mod order_adapter;
pub mod strategy;

/// ③ `ThetaStrategy` —— S_Θ 的 Nautilus Rust-native 策略包壳（feature `nautilus` 门控）。
/// 真实 `StrategyCore + DataActor + Strategy`，包 [`strategy::ThetaCore`]（in-crate S_Θ 核心）。
#[cfg(feature = "nautilus")]
pub mod theta_strategy;

/// ⑥ 真实 `BacktestEngine` 驱动 S_Θ 跑回测（L2 验收：非空订单流）。
/// 门控 `backtest_bin`：依赖 `theta_v0::backtest::data::Dataset`（同 `backtest_bin` 门控），
/// 且只被 CLI（`[[bin]] theta_backtest`）消费——故与 CLI feature 同生命周期。
#[cfg(all(feature = "nautilus", feature = "backtest_bin"))]
pub mod backtest_engine;

/// 适配层路径选择（设计文档 §5，待编排者裁定）。
///
/// `RustNative` = 推荐（编排者「纯 Rust S_Θ + 性能最大化」定调 ⟹ Rust-native Strategy，零 FFI）。
/// `Pyo3Bridge` = 备选（旧调研 §2.1 路径 A，Python 胶水 + v1 legacy IB，更保守稳定）。
///
/// ★这是 enum 占位，**不是运行时分支**——路径选择是构建期的依赖/crate 拓扑决策（方案 A 独立
/// binary crate vs 方案 B cdylib + Python），不在单个代码分支内切换。保留此 enum 仅为设计文档
/// 锚点（让骨架显式记录待裁定的选择点，对齐四分法「选择」类）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationPath {
    /// Rust-native Strategy（`StrategyCore` + `DataActor`），独立 bridge crate。推荐。
    RustNative,
    /// PyO3 Python `Strategy` 胶水，复用现有 cdylib。备选。
    Pyo3Bridge,
}
