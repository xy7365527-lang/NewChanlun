//! 闭环 S_Θ 装配——契约锚 **`Origin.FullDefinitionStrategy`**（task #102 A′ Phase2 step8 重锚）。
//!
//! ## 契约重锚（legacy Strict.HybridAssembly → Origin canonical）
//!
//! A′ Phase1（#97）已把 `formal/Origin/` 立为唯一 canonical base。本模块的闭环装配契约锚点指向
//! Origin canonical `FullDefinitionSystem`（FullDefinitionStrategy.lean:214-247）的单步闭环：
//!
//! - 闭环单步 `hybrid_step` → `Origin.hybridStep`（:229-230）/ `policyTheta`（:226-227）：
//!   `hybridStep S x e = S.transition x (policyTheta S x e) e`，由 `hybrid_step_complete_unique`
//!   （:232-236）证确定唯一、`transition_writes_full_state`（:244-247）证 T 写回完整态。
//! - 六段数据流 → `Origin.FullDefinitionSystem.{recStruct, classify, intent, risk, schedule, transition}`。
//! - R=Π-A-W 账本 → `Origin.LedgerState`（见 strategy/ledger.rs 契约重锚）。
//! - **TW 三阶段 / OQ-9 gate → `Origin.TotalWealth`**（#127 native port 已落地，两端均 Origin
//!   canonical；见 strategy/ledger.rs 模块头「TW 三阶段 / OQ-9 gate 契约 → Origin.TotalWealth」）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - 本模块 = **L0/L1**（结构镜像：闭环状态机与 Origin `FullDefinitionSystem`/`hybridStep` 接口
//!   语义对齐 = 验证管线正确性，零信息增量）。`cargo test` 通过 = 闭环每步保双账本不变量 +
//!   stage 单向 + OQ-9 gate + 真线程化，**不**是缠论盈利/实盘有效声明（那是 L2/L3，真实数据
//!   回测才可否证）。重锚到 Origin **不**提升等级（锚点换 canonical 来源仍是 L0/L1）。
//!
//! ## 模块拓扑（对齐 Origin FullDefinitionSystem 的字段 + 闭环算子）
//!
//! - [`state`]：完整态 `AssemblyState`（乘积态：micro × ledger × tw × risk × phase × pos × ord × mem）
//!   + `MicroState`/`micro_delta`（在线增量解析）+ RiskMode/Phase 枚举。
//! - [`transition`]：六段 adapter（对齐 Origin 六字段）+ `transition_adapter`（双账本写回，
//!   对齐 Origin `transition`）+ `hybrid_step`（对齐 Origin `hybridStep`）+ OQ-9 gate（锚 Origin.TotalWealth）。
//! - [`conformance`]：U5 bit-exact 逐态 conformance —— `hybrid_step` 实例化 `Origin.EngineBridge`
//!   `RustEngineContract` 协议（`step`/`initial`/`classify`），逐事件序列与 Origin `hybridStep`
//!   结构语义逐态比对（非占位 assert true）。
//!
//! ## 与开环单帧的对比（消除 runner.rs 旧版「account 构造一次不喂回」，gap-map 核心）
//!
//! 旧 runner：`AccountState` 在 `run_theta_v0` 里构造一次（runner.rs:107-110），全程不更新喂回——
//! 开环单帧。本模块：runner 改为 `for bar { x = hybrid_step(x, e) }`，闭环态每 bar 真更新喂回
//! （micro_state/ledger_state/tw_state/positions/orders）——闭环 S_Θ，对齐 Origin `hybridStep` 的
//! 「T 写回完整下一态」语义（`transition_writes_full_state`）。
//!
//! ## U5 bit-exact conformance（#127 落地，见 [`conformance`]）
//!
//! **U5 已实装**：闭环 `hybrid_step` 实例化 `Origin.EngineBridge.RustEngineContract`
//! 协议（State/Event/Class/Action/initial/classify/action/step），并对**同一事件序列**逐态
//! 比对——[`conformance`] 模块的 conformance 测试不是占位 `assert true`，而是把 `hybrid_step`
//! 的每步输出态与 Origin `hybridStep` 的结构语义（六段复合 + T 写回完整态 + 双账本写回 +
//! `rust_engine_step_total_unique` 确定唯一）逐字段对齐验证。
//!
//! ★诚实有效域（formalization-validity-domain）：U5 conformance 验证的是 **L1**——Rust 闭环
//! 与 Origin `EngineBridge`/`hybridStep` 的**结构契约**逐态一致（同事件→同态转移语义 +
//! `StepSpec` 确定唯一）。它**不**把 Origin `hybridStep` 在抽象 `FullDefinitionSystem` 上的
//! Lean 数值跑出来逐 bit 比（Origin `hybridStep` 是抽象多态函数，无具体数值轨迹可比）——
//! conformance 的对象是 `RustEngineContract` 协议（Origin 为 Rust 引擎显式留的对齐接口，
//! EngineBridge.lean），逐态比对 = Rust `step` 满足 `StepSpec` 全函数确定唯一 + 六段语义同构。

pub mod state;
pub mod transition;
pub mod buy;
pub mod conformance;
