//! Θ v0 回测 harness（Phase 4，task #81）——对接 theta_v0 **冻结接口**的可证伪检验执行层。
//!
//! ## 纲领（docs/backtest-protocol-v0.md）
//!
//! 用 frozen Reference Θ v0（`ThetaConfig::default()`）在 8 个真实 1min 品种上执行
//! **预注册**回测协议。协议在看任何结果前冻结（§0），harness 照跑、不事后改判据。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! | 组件 | 等级 | 理由 |
//! |------|------|------|
//! | 数据加载/清洗（[`data`]） | **L1** | 管线正确性（读 JSON→量化→bar），零信息增量 |
//! | 指标计算（[`metrics`]） | **L1** | 给定权益曲线算 CAGR/Sharpe/... 是确定性变换，验证算法正确，不验证 Θ |
//! | 管线串联（[`runner`]） | **L1**（管线就绪后） | parse→classify→plan_orders→fill 串通 = 验证管线，不验证 Θ 有效 |
//! | **真实历史数据回测结论** | **L2** | 用真实 OHLC 跑 frozen Θ，扣成本后风险调整收益可被否证（正信息增量） |
//!
//! **关键诚实声明**：harness 框架本身（data+metrics+runner 管线串通）是 **L1**——
//! 合成数据"验证"管线只能确认无 bug，不能确认 Θ 在市场有效（合成数据的独立性验证是
//! 同义反复，231号）。**只有喂真实历史数据跑出的扣成本指标才是 L2**。严禁用合成数据
//! 跑通管线后声称"回测已验证 Θ v0"。
//!
//! ## 等引擎阻塞点（管线尾部当前是骨架占位，必须显式标注）
//!
//! theta_v0 管线数据流当前**断裂**（这是冻结的设计中间态，非 harness 缺陷）：
//!
//! - `parser::parse_layer(bars, config) -> ParseLayer`：签名冻结，前 4 步已实装
//!   （inclusion/fractal/stroke/segment）；中枢/canonical/tail 待 #78 收尾。
//! - `classifier::classify(l0, config) -> Classification`：签名冻结，**当前返回空**
//!   （骨架占位，task #79 cc-classifier 实装中）。⟸ **阻塞点 A**
//! - `strategy::plan_orders(config) -> Vec<Order>`：签名冻结，**当前返回空**，且
//!   **尚未接入 `Classification` 输入**（strategy.rs 注释："为避免骨架阶段引入未实装
//!   的数据流耦合，此处先不接线"）。task #80 cc-strategy 实装时会改签名接入
//!   classification。⟸ **阻塞点 B（接口签名会变）**
//!
//! 因此 [`runner`] 的管线串联函数 [`runner::run_theta_v0`] 当前对接**冻结的当前接口
//! 形态**，但在 classify/plan_orders 返回空时，订单流为空 ⇒ 无交易 ⇒ 指标退化为
//! buy&hold 对照基线。**这不是 L2 回测**——真正的 L2 需要阻塞点 A+B 解除（classifier
//! 产出非空 BSP + strategy 接入 classification 产出订单）。harness 不伪造管线已通。
//!
//! ## 子模块拓扑
//!
//! - [`data`]：读 `analysis/data_cache/*.json` → 清洗 → 量化为整数 tick [`Bar`]。
//!   日期窗切片（OOS/Holdout/walk-forward，协议 §2）。**独立于断裂的管线，当前 L1 可验证。**
//! - [`metrics`]：权益曲线 → 协议 §3 全套指标（CAGR/Sharpe/Sortino/MaxDD/Calmar/CVaR/
//!   换手/胜率/盈亏比/费用敏感性）。**独立于引擎，当前 L1 可验证。**
//! - [`runner`]：串联 data→引擎→订单→fill→权益曲线→metrics。管线尾部阻塞于引擎。
//!
//! ## 协议锚点（docs/backtest-protocol-v0.md，冻结）
//!
//! - 数据快照只读（§1.2）；OOS=2023-01-01→2025-06-30，Holdout=2025-07-01→末尾（§2.1）。
//! - 费用基线：commission 1bp/side、slippage 2bp/side、tax 0bp（`ExecConfig::default()`）。
//! - 随机对照 seed=20260625（§4）；失败判据 §5 看结果前冻结。
//!
//! [`Bar`]: super::types::Bar

pub mod data;
pub mod incremental;
pub mod metrics;
pub mod mu_estimator;
pub mod prereg_windows;
pub mod runner;

/// 全窗 L3 定论测试模块（task #75，owner=l3-fullwindow 工位，Lead 登记）。
/// 复现 [`runner`] 的 `l3_falsify_multi_symbol_significance` 但 `cut=全窗`（非 60K 截断），
/// 定论缠论择时 alpha。整个 `backtest` 已被 `#[cfg(test)]` 门控（theta_v0/mod.rs:88），本子模块
/// 继承门控——只在 `cargo test --lib` 编译，不污染 cdylib；integration test（独立 crate）看不到
/// （故全窗测试只能挂此处，见文件头机器坐实 E0433）。
mod l3_fullwindow;

/// Phase-1 可行性探针（pi 七链生产 runner 的 O(n²) substrate 时标 + 信号/交易计数）。
/// 决定 L2/L3 用全窗还是可行子集。继承 backtest cfg(test) 门控。
mod l3_pi_probe;

/// Phase-2 L2/L3 pi 否证（驱动 run_theta_v0_pi 七链生产 runner；双口径门 + 分层诊断）。
/// 镜像 [`l3_fullwindow`] 口径但驱动 pi（非 v1 recognize）；O(n²) 致用可行子集截断窗。
mod l3_pi_falsify;

/// #5 多声部对冲深度贡献根因诊断（231 诊断非 alpha）——instrument 计数区分 (a) ρ漂移剪枝
/// vs (b) 结构不产。O(n²) CL/BTC 32K，继承 backtest cfg(test) 门控。
mod l3_pi_depth_diag;
