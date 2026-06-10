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

pub mod allocator;
pub mod center_book;
pub mod config;
pub mod fatigue_gate;
pub mod ledger;
pub mod level_operating_unit;
pub mod master;
pub mod runner;
pub mod tape;
pub mod types;
