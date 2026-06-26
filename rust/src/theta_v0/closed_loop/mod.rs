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
//! - **TW 三阶段 / OQ-9 gate → 锚点缺位**（Origin 无 TW 对应，仍锚 legacy `TotalWealth.lean`；
//!   见 strategy/ledger.rs 模块头「TW 三阶段 / OQ-9 gate 的 Origin 锚点缺位」诚实声明）。
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
//!   对齐 Origin `transition`）+ `hybrid_step`（对齐 Origin `hybridStep`）+ OQ-9 gate（legacy 锚）。
//!
//! ## 与开环单帧的对比（消除 runner.rs 旧版「account 构造一次不喂回」，gap-map 核心）
//!
//! 旧 runner：`AccountState` 在 `run_theta_v0` 里构造一次（runner.rs:107-110），全程不更新喂回——
//! 开环单帧。本模块：runner 改为 `for bar { x = hybrid_step(x, e) }`，闭环态每 bar 真更新喂回
//! （micro_state/ledger_state/tw_state/positions/orders）——闭环 S_Θ，对齐 Origin `hybridStep` 的
//! 「T 写回完整下一态」语义（`transition_writes_full_state`）。
//!
//! ## U5 bit-exact conformance 诚实延后（非声明膨胀，no-workaround）
//!
//! **U5 待做**：闭环 `hybrid_step` 与 Origin `hybridStep`（及最终 EngineBridge/RustEngineContract，
//! 由并行 #101 重锚）的 **bit-exact 对齐证**（逐字段同构 fixture / 同一事件序列 Rust/Lean 逐态
//! 相等）**诚实延后**。当前各 `hybrid_step` 测试验证的是**结构不变量对齐**（保 R=Π-A-W / TW 守恒 /
//! stage 单向 / OQ-9 gate / 真线程化），与 Origin/legacy 定理结构一一对应——这是 **L0/L1**
//! （验管线非验缠论假设）。bit-exact 数值对齐（U5 内容）尚未做，本模块**不冒充已 conformant**：
//! 「重锚契约语义」≠「已通过 bit-exact conformance」，二者分级清晰，非半成品 workaround。

pub mod state;
pub mod transition;
pub mod sell;
