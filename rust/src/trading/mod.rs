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

// GUARD-ROLE: organic-fugue-v2-trading-layer
//
// ## 名分（`docs/agents/generation-constitution.md` §1 名分五态，#763 C7-E4 执行票核定）
//
// - **名分**：**现役**（机械判据：有非测试调用者 ∧ 无 `#[deprecated]` 标记 ∧ 在唯一 git 线
//   main 上）。名分表（`.chanlun/review-results/legacy-generation-census-20260729.md` §1.4/§2）
//   原判本目录 28 个 v2 文件"生产 0，PyO3 导出 + 2 处冷 python 调用（`trading_system/` 三份
//   脚本）"⟹ deprecated 待退役、拟集中 `legacy/`——本票扩面复核（全仓 python 含 `analysis/`
//   166 个脚本，非仅原表核查的 `trading_system/`）发现该判断**不成立**：`run_organic_rust`/
//   `run_positional_rust`（经 `PolarityMode` 字符串 mode 分派，覆盖 `unn`/`urs`/`nrf`/`iso`/
//   `pcf`/`nif*`/`rnf`/`fusion_v*`/`fusion_va`/`fusion_vd*` 全部变体）/`run_recursive_rust`/
//   `UnnStream` 四个 pymodule 导出面，在 `analysis/` 下有 **60+ 处**真实非测试调用（`git log -1`
//   多数落在 2026-06-11~2026-07-26 区间，含 2026-07-26 一处近期改动），逐一映射回本目录 28
//   文件的 mode 分派链（`positional.rs` match 表 + `axiom_voice`/`dual_voice`/`isolated_fugue`/
//   `nested_fugue`/`nested_interval_fugue`/`positioning_chain_fugue`/`recursive_nested_fugue`/
//   `unified_necessity`/`unified_osc`/`unified_recursive`/`unified_voice`/`positional_fusion`
//   互相 `use super::X` 组成的单向依赖网，见本文件头部依赖图）——**28 文件无一属于"零活引用"**。
//   此外 `types`/`tape`/`positional`/`center_book`/`depth_ref` 五文件是 `spiral::signal`/
//   `fugue_v3::{accounting,layer,ffi,...}`/`recursive_t::{stream,t_engine,rec_stream,...}`
//   （#762 已改判**现役**）的编译期硬依赖（`use crate::trading::{types,tape,positional,
//   center_book,depth_ref}` 生产级引用，见 #762 报告 §2.2），三条现役族继续挂着不放，进一步
//   排除这五文件的 deprecated 资格。`consolidation_ablation`/`sublevel_confirmation_ablation`/
//   `trade_behavior` 三个 `#[cfg(test)]` 文件是带真实 `#[test]` 断言的消融守卫套件（非死代码，
//   `--ignored` 长测试，产出对照 JSON 硬校验），非 python/生产可达但本身是有效测试基础设施，
//   不适用"零活引用⟹删除"判据（判据前提是生产/python 可达性，非测试基础设施存废）。
// - **对照什么**：两条独立证据链均指向"现役、不可删/不可移"——(1) `analysis/` 扩面 python
//   调用面（60+ 处，mode 分派覆盖全部 28 文件对应实现）；(2) `types`/`tape`/`positional`/
//   `center_book`/`depth_ref` 是 #762 三族现役判定的编译期硬依赖。theta_v0（π，唯一现役引擎）
//   对本目录生产引用 = 0（名分表 §0 全局核查复验一致，π 完全自包含）。
// - **与现役差在哪**：π 完全自包含未复用本目录一行；本目录继续挂起等待"四族+散件"统一批次
//   复核（名分表 §5 N+5），非独立可判死代码；与 #762 三族同构——扩面核查系统性推翻名分表原判。
// - **禁回灌**：本次仅加标记，未删除/未移动任何代码；两个活物（`center_book::
//   consume_death_certificate` 方法体 / `third_point_book.rs` 整文件）零触碰。
// - **处置**：**GUARD-ROLE 留档，零删除，零移入 `legacy/`**（原计划"集中 legacy/"因扩面核查
//   发现无一文件满足"零活引用者"删除门槛而不成立，处置改为标记留档，沿 #762 先例）。

pub mod allocator;
pub mod axiom_voice;
pub mod center_book;
pub mod config;
#[cfg(test)]
mod consolidation_ablation;
pub mod depth_ref;
pub mod dual_voice;
pub mod fatigue_gate;
pub mod isolated_fugue;
pub mod ledger;
pub mod level_operating_unit;
pub mod master;
pub mod nested_fugue;
pub mod nested_interval_fugue;
pub mod positional;
pub mod positional_fusion;
pub mod positioning_chain_fugue;
pub mod recursive_nested_fugue;
pub mod recursive_position;
pub mod runner;
#[cfg(test)]
mod sublevel_confirmation_ablation;
pub mod tape;
pub mod third_point_book;
#[cfg(test)]
mod trade_behavior;
pub mod trend_exhaustion;
pub mod types;
pub mod unified_necessity;
pub mod unified_osc;
pub mod unified_recursive;
pub mod unified_voice;
