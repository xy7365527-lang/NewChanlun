//! 闭环 S_Θ 装配——镜像 Lean `Strict/HybridAssembly.lean`（单一闭环状态机实例）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - 本模块 = **L0/L1**（结构镜像：闭环状态机与 Lean `assembly`/`assemblyStep` 对齐 = 验证
//!   管线正确性，零信息增量）。`cargo test` 通过 = 闭环每步保双账本不变量 + stage 单向 +
//!   OQ-9 gate + 真线程化，**不**是缠论盈利/实盘有效声明（那是 L2/L3，真实数据回测才可否证）。
//!
//! ## 模块拓扑（对齐 Lean HybridAssembly 的 §2/§3/§4）
//!
//! - [`state`]：完整态 `AssemblyState`（乘积态：micro × ledger × tw × risk × phase × pos × ord × mem）
//!   + `MicroState`/`micro_delta`（镜像 Dynamics.State/delta）+ RiskMode/Phase 枚举。
//! - [`transition`]：六段 adapter + `transition_adapter`（双账本写回）+ `hybrid_step`（x_t ─e→ x_{t+1}
//!   单一闭环）+ OQ-9 gate。
//!
//! ## 与开环单帧的对比（消除 runner.rs 旧版「account 构造一次不喂回」，gap-map 核心）
//!
//! 旧 runner：`AccountState` 在 `run_theta_v0` 里构造一次（runner.rs:107-110），全程不更新喂回——
//! 开环单帧。本模块：runner 改为 `for bar { x = hybrid_step(x, e) }`，闭环态每 bar 真更新喂回
//! （micro_state/ledger_state/tw_state/positions/orders）——闭环 S_Θ。
//!
//! ## U5 conformance 延后（诚实标注，非声明膨胀）
//!
//! TODO（U5 待 Lean #93/#88 封印）：闭环 `hybrid_step` 与 Lean `assemblyStep` 的 **bit-exact 对齐
//! 证**（逐字段同构 fixture）延后到 Lean 侧封印后做。当前各 `hybrid_step` 测试验证**结构不变量
//! 对齐**（保 R=Π-A-W / TW 守恒 / stage 单向 / OQ-9 gate / 真线程化），与 Lean 定理一一对应；
//! bit-exact 数值对齐（同一事件序列 Rust/Lean 逐态相等）是 U5 的内容。这是诚实延后（声明当前
//! 覆盖结构不变量对齐，不冒充已做 bit-exact 对齐），非半成品 workaround。

pub mod state;
pub mod transition;
