//! 有机赋格 v2 交易层 — 引擎事件流的独立消费层（非嵌入 process_bar，v1R §3.1 裁决）。
//!
//! 设计：`analysis/organic_fugue_v2_design.md`（矛盾修正 C1-C7 + 缺失补全 D1-D3 +
//! 保留 K1-K13）；类型系统参考：`analysis/organic_fugue_rust_design.md`。
//!
//! 模块依赖（单向，无环）：
//! ```text
//!   types ← config
//!     ↑       ↑
//!   ledger  tape  center_book
//!     ↑       ↑    ↑
//!   fatigue_gate  allocator
//!     ↑       ↑    ↑
//!   level_operating_unit
//!     ↑
//!   runner
//! ```
//!
//! 现有引擎模块（orchestrator/bi_zhongshu_bsp/…）零改动——交易层只 use 引擎枚举。
//!
//! **图外挂件（票 #638）**：[`third_point_book`]（三类点成立登记账）**零入边**——上图任何模块都
//! 不 use 它，它也不进 `runner` 主循环（V0≡P5 parity 承重面不受影响）。它只依赖 `theta_v0`
//! 买卖点账本的公开产出面，是该账「成立档」的交易层消费入口（观测登记侧，非入场判据侧；理由与
//! 裁定登记见该模块头）。

pub mod allocator;
pub mod center_book;
pub mod config;
pub mod depth_ref;
pub mod fatigue_gate;
pub mod ledger;
pub mod level_operating_unit;
pub mod isolated_fugue;
pub mod master;
pub mod nested_fugue;
pub mod nested_interval_fugue;
pub mod positioning_chain_fugue;
pub mod recursive_nested_fugue;
pub mod unified_necessity;
pub mod unified_recursive;
pub mod positional;
pub mod positional_fusion;
pub mod recursive_position;
pub mod unified_osc;
pub mod unified_voice;
pub mod axiom_voice;
pub mod dual_voice;
pub mod runner;
pub mod tape;
pub mod third_point_book;
#[cfg(test)]
mod consolidation_ablation;
#[cfg(test)]
mod sublevel_confirmation_ablation;
#[cfg(test)]
mod trade_behavior;
pub mod trend_exhaustion;
pub mod types;
