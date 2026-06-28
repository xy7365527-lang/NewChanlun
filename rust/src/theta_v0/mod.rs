//! # Reference Θ v0 — bit-exact 缠论可执行系统引擎（Phase 2，task #38）
//!
//! 本模块是 `docs/reference-theta-v0.md`（codex gpt-5.5 high 编排者全权代理裁决，
//! 2026-06-25）冻结的 **Θ v0 规格的 bit-exact Rust 实装**。契约锚点重锚到 `formal/Origin/*.lean`
//! 唯一 canonical base（A′ Phase2 #127）：`Origin.ChanlunElements`（ElementPipeline 构造层）/
//! `Origin.CenterComplete`+`CenterConstruction`+`CenterStates`（中枢完整判据/构造/三态）/
//! `Origin.TrendCompleteClassification`（走势裁决）/ `Origin.BspClassification`+`BspConstruction`
//! （买卖点）/ `Origin.RecursiveLevelSystem`（递归级别）/ `Origin.SubLevelDescent`（下钻）/
//! `Origin.StrategyFamily`+`VoiceTree`+`ThetaInstantiation`（策略族）/ `Origin.RiskProj`（风险）/
//! `Origin.FullDefinitionStrategy`（闭环 hybridStep）/ `Origin.TotalWealth`（TW 三阶段）/
//! `Origin.ForceInterface`（力度）/ `Origin.EngineBridge`（U5 conformance 协议）。
//!
//! ## 与现有 standalone 引擎（`crate::recursive_t` / `crate::spiral` / `crate::fugue_v3`）的关系
//!
//! 现有引擎是与 Python 逐位等价的旧 ladder，**不是**对 Origin Θ v0 的 bit-exact 实装。
//! 本模块是**独立的新引擎**——唯一权威是 Θ v0 规格 + `Origin.*` canonical spec，不复用旧 ladder 的
//! 中枢/走势/背驰逻辑（避免 Python-等价语义漂入 Origin-等价语义）。
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
//!   契约锚 `Origin.ChanlunElements.ElementPipeline`（mergeBars/fractalsOf/strokesOf/segmentsOf）。
//! - [`classifier`]：Θ_level + Θ_signal 实装（递归级别 / R6 态 / 买卖点 bit-vector /
//!   背驰度量 / 区间套）。契约锚 `Origin.RecursiveLevelSystem` / `Origin.CenterStates` /
//!   `Origin.CenterComplete` / `Origin.BspClassification` / `Origin.TrendCompleteClassification` /
//!   `Origin.SubLevelDescent`。
//! - [`strategy`]：Θ_voice + Θ_risk + Θ_exec 实装（声部树 / 风险投影 / sizing /
//!   执行 / Param 索引策略族）。契约锚 `Origin.StrategyFamily.{Theta,piTheta,StrategyFamily,
//!   familyOfTheta}` / `Origin.VoiceTree` / `Origin.ThetaInstantiation` / `Origin.RiskProj`。
//!
//! ## 铁律（编排者硬指令）
//!
//! - 只实装**已冻结 Θ v0**；遇 spec 漏洞/与 `Origin.*` canonical 冲突 → 开 change request
//!   （SendMessage Lead），**不静默改语义**。
//! - 不改 `formal/Origin/*` Lean spec（只追加 conformance fixture）。
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

/// 分账本头寸空间 P^sep（工作单元 R2，C25/C26/C29）。契约锚 **`formal/Origin/SeparateLedger.lean`**
/// （C25/C26：`Leg`/`SepPosition`/`Net`）+ spec §四 C29 分账本吃到 `Eat^sep`。与净额账本
/// （`strategy::ledger` R=Π-A-W、`nautilus::account_adapter` net_position）**正交并置**——P^sep 保留
/// 每声部多/空两独立坐标（双开 (Q,Q) 不抵消），净额映射 Net 是有损投影（230号直积退化防火墙）。
/// 全模块 L0/L1（结构镜像，非实盘盈利声明）。
pub mod ledger;

/// 闭环 S_Θ 装配（Phase 4 引擎实装，task #94；A′ Phase2 step8 契约重锚 Origin，task #102/#127）。
/// 契约锚 **`formal/Origin/FullDefinitionStrategy.lean`** 的单一闭环状态机（`FullDefinitionSystem` +
/// `hybridStep` + `transition` + `LedgerState` R=Π-A-W）+ `formal/Origin/ChanlunElements.lean`
/// （`ElementPipeline.parse`）+ `formal/Origin/EngineBridge.lean`（`RustEngineContract` U5 conformance）。
/// 双账本两端均锚 Origin canonical：R=Π-A-W 锚 `Origin.FullDefinitionStrategy.LedgerState`；TW 取本金
/// 三阶段 + OQ-9 gate 锚 `Origin.TotalWealth`（#127 native port 落地，见 closed_loop 与 strategy/ledger.rs
/// 模块头）。把开环单帧引擎升级为闭环——闭环态每 bar 真更新喂回。
pub mod closed_loop;

/// 完整状态/事件 schema（FULL 结果包 §3 17 分量 `x_t` + §20 8 元组 `e_{t+1}` 的逐分量显式实装，
/// 组C 补全工位，task #43）。契约锚 **`formal/Origin/CompleteStateEvent.lean`**
/// （`NewChanlun.Origin.CompleteStateEvent`）。补全 cov-rust-impl 报告的 E1 状态~50%（17 分量逐分量，
/// 非 `closed_loop::AssemblyState` 6 分量摘要）+ D7 事件仅价格笔（8 元组，补 7 类经纪/会计/公司行为事件，
/// 非 `closed_loop::MicroEvent` 仅 NewBar/NewStroke）。与闭环摘要态/微事件并置——完整态→摘要态、外部
/// 事件→微事件均为遗忘投影（见 [`complete`] 模块头）。全模块 L0（纯结构 schema）。
pub mod complete;

/// 回测 harness（Phase 4，task #81）。`#[cfg(test)]` 门控——[`backtest::data`] 依赖
/// serde_json（dev-dependency），且 backtest-protocol-v0.md §8 流程本就在 test 环境跑
/// （`cargo test --release ... -- --ignored`）。门控避免污染 cdylib（Python 扩展）构建，
/// 与 `recursive_t/backtest_run.rs` 的 serde 门控先例一致。
#[cfg(test)]
pub mod backtest;
