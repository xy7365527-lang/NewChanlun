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

/// C2：46 个直读 env 键的单一登记处（map #743，spec #756）——键名/语义/行为门|观测门/默认臂/
/// 所属层五列在案，运行期可枚举/按类过滤。模块头详述形状裁定与完备性单测。
pub mod env_registry;

/// venue 真实费率标定 datum（#360；规格 = `venue-fee-source-research-20260726.md` §3）——
/// per-notional（Binance 现货 maker/taker）与 per-share + 最低佣金 + 卖出监管费（IBKR Pro 美股）
/// 两种计费单位的原生表达 + datum 文件 sha256 版本哈希。[`config::ExecConfig::fee_schedule`]
/// `None` ⟹ 三常数未标定 fallback（**逐位现状**），`Some` ⟹ L2 标定档。
pub mod venue_fee;

pub mod classifier;
pub mod parser;
pub mod strategy;

/// #951：生产 π 回路的流式单 bar 驱动（`ThetaPiStream`）——PyO3 出口的决策内核。无条件编译
/// （不随 `backtest` 门控），默认 cdylib 构建可达。seam = `push_bar(bar, p_t, nav) -> p_star`。
pub mod stream;

/// #951 D1 出口：`theta_v0` 的 PyO3 出口（`ThetaStream`）。无条件编译（默认 cdylib 构建可达）。
pub mod ffi;

/// 重基构造谱系簿（#679 D1b）：消费 D1a（#543）的 `RebaseTransformTxnV1` 构造证书，在进程内建
/// 「旧中枢身份 → 新中枢身份」的 1→1 映射，供接线层把挂起随谱系迁移（`RebaseVanished` 从
/// 「工程丢身份」压回真实的构造终止）。教义口径、fail-closed 规则与两种读法见模块头。
pub mod lineage_book;

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

/// 回测 harness（Phase 4，task #81）。门控为 `any(test, feature = "backtest_bin")`——
/// [`backtest::data`] 依赖 serde_json（test 走 dev-dependency；CLI 走 backtest_bin feature 的
/// optional dep）。默认 cdylib（Python 扩展）构建**两者都不开** ⟹ 零依赖膨胀（acceptance[5]
/// CLI 生产入口解门控：let theta_backtest bin 链接 run_theta_v0_pi/load_by_symbol）。
#[cfg(any(test, feature = "backtest_bin"))]
pub mod backtest;

/// Nautilus Trader ↔ canonical S_Θ 适配层（goal acceptance[5]，设计 `docs/nautilus-integration-design.md`）。
///
/// 实装在役（订正 #524：原「骨架阶段/不 `use nautilus_*`」声明过期作废——依赖已入 Cargo.toml
/// 5 件 v0.60.0 optional + `nautilus` feature 门控；`theta_strategy.rs` / `backtest_engine.rs`
/// 真实 `use nautilus_*`，分别由 `nautilus` / `all(nautilus, backtest_bin)` feature 门控）。
pub mod nautilus;
