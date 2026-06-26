//! # Reference Θ v0 — bit-exact 缠论可执行系统引擎（Phase 2，task #38）
//!
//! 本模块是 `docs/reference-theta-v0.md`（codex gpt-5.5 high 编排者全权代理裁决，
//! 2026-06-25）冻结的 **Θ v0 规格的 bit-exact Rust 实装**。对齐 `formal/Strict/*.lean`
//! 的 L0 spec（Classification / Trend / Center / BSP / Recursive / Decomp / Causal /
//! OpenTail / Op / StrategyFamily / ClassificationFamily / Parse / LevelState / Nest /
//! Fugue / RiskProj / Chain）。
//!
//! ## 与现有 standalone 引擎（`crate::recursive_t` / `crate::spiral` / `crate::fugue_v3`）的关系
//!
//! 现有引擎是与 Python 逐位等价的旧 ladder，**不是**对 Lean Θ v0 的 bit-exact 实装。
//! 本模块是**独立的新引擎**——唯一权威是 Θ v0 规格 + Lean spec，不复用旧 ladder 的
//! 中枢/走势/背驰逻辑（避免 Python-等价语义漂入 Lean-等价语义）。
//!
//! ## 纲领（codex 总纲，reference-theta-v0.md:4）
//!
//! **L0 证明"给定 Θ 后系统怎样无歧义地分类和行动"；不证明 Θ 是好 Θ，也不证明市场会
//! 奖励这个语法。** 链：缠论结构公理 + Θ ⟹ C_Θ（615/617）⟹ π_Θ（616）⟹ Rust 实装
//! ⟹ L2/L3 检验（可证伪）。
//!
//! ## 认识论等级（formalization-validity-domain 强制）
//!
//! - 实装本身 = **L1**（bit-exact 一致性：Rust 与 Lean spec 输出对齐 = 验证管线正确，
//!   不验证 Θ 在市场上有效）。
//! - 对齐 Lean fixture 的 conformance test = L1（合成/golden 一致性）。
//! - **L2/L3 有效域检验**是 Phase 3（回测协议）+ Phase 4-7 的事，本模块不声称。
//!
//! ## 子模块拓扑（parser → classifier → strategy 三子单元 + 横切支撑）
//!
//! - [`config`]：所有 Θ 参数显式化（`[设计选择]`/`[L3经验待标定]` 均为 config 字段，
//!   **不硬编码**——Phase 6 Θ 空间扫描的入口）。
//! - [`types`]：整数 tick 价格 / OHLC bar / 方向 / 结构对象（K线/分型/笔/线段/中枢/Move）
//!   / 信号 / 声部 / 仓位 / 订单等共享数据类型。
//! - [`parser`]：Θ_parse 实装（包含/分型/新笔/线段67课/中枢/canonical 分解/未完成尾部）。
//!   bit-exact 对齐 `Strict/Parse.lean`。**[子任务，TaskCreate]**
//! - [`classifier`]：Θ_level + Θ_signal 实装（递归级别 / R6 态 / 买卖点 bit-vector /
//!   背驰度量 / 区间套）。bit-exact 对齐 `Strict/LevelState.lean` / `Center.lean` /
//!   `BSP.lean` / `Trend.lean` / `Nest.lean`。**[子任务，TaskCreate]**
//! - [`strategy`]：Θ_voice + Θ_risk + Θ_exec 实装（声部树 / 风险投影 / sizing /
//!   执行）。bit-exact 对齐 `Strict/Fugue.lean` / `RiskProj.lean` / `Op.lean` /
//!   `StrategyFamily.lean`。**[子任务，TaskCreate]**
//!
//! ## 铁律（编排者硬指令）
//!
//! - 只实装**已冻结 Θ v0**；遇 spec 漏洞/与 Lean 冲突 → 开 change request（SendMessage
//!   Lead），**不静默改语义**。
//! - 不改 Lean spec（只追加 conformance fixture）。
//! - Rust 编译通过 + 测试绿是完成标准。
//!
//! ## 平局裁决全局约定（reference-theta-v0.md:15-16）
//!
//! - 所有价格量化为整数 tick；无 `tick_size` 默认 `1e-8`。
//! - 所有平局按 `(timestamp, source_index)` 升序裁决；已确认结构不可回写重分解。

pub mod config;
pub mod types;

pub mod classifier;
pub mod parser;
pub mod strategy;

/// 闭环 S_Θ 装配（Phase 4 引擎实装，task #94）。镜像 `formal/Strict/HybridAssembly.lean` 的
/// 单一闭环状态机（AssemblyState + transition_adapter + hybrid_step）+ 双账本（R=Π-A-W + TW
/// 取本金三阶段）+ OQ-9 gate。把开环单帧引擎升级为闭环——闭环态每 bar 真更新喂回。
pub mod closed_loop;

/// 回测 harness（Phase 4，task #81）。`#[cfg(test)]` 门控——[`backtest::data`] 依赖
/// serde_json（dev-dependency），且 backtest-protocol-v0.md §8 流程本就在 test 环境跑
/// （`cargo test --release ... -- --ignored`）。门控避免污染 cdylib（Python 扩展）构建，
/// 与 `recursive_t/backtest_run.rs` 的 serde 门控先例一致。
#[cfg(test)]
pub mod backtest;
