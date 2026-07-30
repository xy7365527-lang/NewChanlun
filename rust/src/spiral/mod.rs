//! 螺旋引擎 v2（Spiral Engine v2）—— 以 D∞ 群结构为第一性原理的交易引擎。
//!
//! 设计文档：`docs/spiral_engine_v2_architecture.md`（785 行架构 + 实装路线）。
//! 理论基础：`docs/dialectical_exhaustion.md` · `docs/necessity_derivation.md`（58 不变量 +
//! T56–T59 群关系）· `docs/orbit_enumeration_completeness.md`（H¹）· `docs/concept_movement_chain.md`（23 环）。
//!
//! ## 存在论位置（架构 §0.1）
//! v2 把 D∞ 群结构（`G = ⟨h,τ | τ²=e, τhτ⁻¹=h⁻¹⟩`，`σ:=h²³`）提升为引擎第一性原理：
//! - 状态空间 = 群作用空间坐标 `(φ, r, ε)`（`state.rs::SpiralState`）。
//! - 交易操作 = 群元素作用（`state.rs::GroupAction`），非独立 F/C/D/E 函数。
//! - prove = 群关系运行时 panic（`prove.rs`），violation = 不合法 = panic。
//! - 闭合 = H¹ 生成元 `Δr=−1`（Step 4 `closure.rs`）。
//!
//! ## 与现有 unn 引擎的关系（架构 §2）
//! v2 **不删除** `trading::unified_necessity`（保留为 bit-exact 对照基线，直到
//! v2 验收通过）。v2 复用信号层契约（tape/bsp_events）+ 会计语义（VoiceForest）。
//!
//! ## 实装路线（架构 §11，Step 0–7 全实装）
//! - `state`：`SpiralState(φ,r,ε)` + helix + `GroupElement`/`GroupAction`（Step 0，L0）。
//! - `prove`：群关系 L0 + N1–N8/theta_sigma/cross_level/t14/a5/t1/T50/T56–T59（Step 5）。
//! - `params`：常量 + 认识论标注 + `ProveLevel`（架构 §10）。
//! - `voice`：Voice 森林（携带 SpiralState）。
//! - `result`：运行结果 + 守恒/观测计数器。
//! - `accounting`：nav/settle/close_voice/σ-不变配额 spawn + DepthGate（Step 1/4 会计）。
//! - `signal`：BarSig → 群事件（Pending/cascade/向心 confirm，Step 2）。
//! - `closure`：H¹ 三类闭合 + P-close（Δr=−1，Step 3）。
//! - `operation`：五操作群作用类型化（A∉M 编译期显形，Step 3）。
//! - `engine`：`SpiralEngineCore::step/finish`（A→C→D→E→F 群作用驱动，Step 5）。
//! - `ffi`：PyO3 `SpiralStream`（流式）+ `run_spiral`（批量，Step 6）。
//!
//! ## gap 跟踪（Step 7，保留不强行闭合，no-patch-mentality）
//! G1（整数股数 f64 掩盖）/ G2（A 强平非群——`operation::Operation::Liquidate.group_action()`
//! 返 None 显形）/ G3（confirm 向心 vs 前向延异，已 escalate）/ G4（第三类）/ G5（θ 配额节点）。

// GUARD-ROLE: legacy-generation-loadbearing-for-fugue-v3
//
// ## 名分（`docs/agents/generation-constitution.md` §1 名分五态，#762 C7-E3 执行票核定）
//
// - **名分**：**现役**（机械判据：有非测试调用者 ∧ 无 `#[deprecated]` 标记 ∧ 在唯一 git 线
//   main 上）。名分表（`.chanlun/review-results/legacy-generation-census-20260729.md` §1.1/§2）
//   原判本族"deprecated 待退役"——本票扩面复核（全仓 python 含 `analysis/`，非仅
//   `trading_system/`）新增证据推翻该判：`trading_system/backtest_spiral_stream.py:148`
//   `nr.SpiralStream(...)` 是名分表未列出的第二处真实 python 调用方（原表只列
//   `compare_spiral_unn.py:60` 一处，同批冷调用，`git log -1` 均 = 2026-06-21）。此外本族
//   `signal.rs` 被 `fugue_v3::{axis,engine,observe,morphology,operate}.rs`（本仓下一族，同
//   处置票 #762 处置范围）**生产代码**（非 `#[cfg(test)]`）直接 `use`（`PendingLocate`/
//   `SignalState`/`SpiralResult`/`GroupEventFrame`/`prove_chain`/`prove_t52_gauge_fix`），是
//   fugue_v3 的编译期硬依赖——删除会立即打断 fugue_v3 编译。
// - **对照什么**：两条独立证据链均指向"现役、不可删"——(1) 两处 python 直接调用（虽冷，
//   2026-06-21，早于 ADR-0004 2026-07-28 五周，但文件仍在主线树未删）；(2) fugue_v3 信号层
//   的编译期硬依赖。theta_v0（π，唯一现役引擎）对本族生产引用 = 0（§0 全局核查复验一致）。
// - **与现役差在哪**：π 完全自包含未复用本族一行；本族继续挂起等待"四族/散件"批次统一处置
//   （名分表 §5 执行票切票建议 N+3~N+5），非独立可判死代码。
// - **禁回灌**：本次仅加标记，未删除/未移动任何代码。
pub mod accounting;
pub mod closure;
pub mod engine;
pub mod ffi;
pub mod operation;
pub mod params;
pub mod prove;
pub mod result;
pub mod signal;
pub mod state;
pub mod voice;

// 公开 API 再导出（架构 §3 公开 API：SpiralEngine, SpiralStream）。
pub use closure::ClosureKind;
pub use engine::SpiralEngineCore;
pub use ffi::{run_spiral, PySpiralStream};
pub use operation::Operation;
pub use result::SpiralResult;
pub use state::{GroupAction, GroupElement, GroupViolation, SpiralState};
pub use voice::{SpiralForest, SpiralVoice, VoiceStatus};
