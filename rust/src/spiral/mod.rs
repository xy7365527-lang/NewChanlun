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
