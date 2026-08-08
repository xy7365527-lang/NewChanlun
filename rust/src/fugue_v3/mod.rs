//! 赋格引擎 v3（Fugue Engine v3）—— **D∞ word 处理器**。
//!
//! 设计文档：`docs/recursive_fugue_necessity_proof.md`（定理 RF）+
//! `docs/spiral_physical_reinterpretation.md` v2（H⁰ 核心仓 / H¹ 机动仓 / D∞⋉ℝ）+
//! `docs/orbit_enumeration_completeness.md`（D∞=⟨h,τ|τ²=e,τhτ⁻¹=h⁻¹⟩, σ=h²³）+
//! `docs/operation_route_exhaustion.md`（Σ 字母表 + 双投影 + §7.4 操作→Σ 原子映射）。
//!
//! ## 核心：引擎不预设循环模式（用户裁决 2026-06-17）
//! 四步循环（平多→开空→平空→做多）**只是多方视角的一个例子**——还有空方版本、会计双重性。
//! 引擎的核心是 **D∞ word 处理器**：每 bar 每级别读信号 → 决定 **h**（走势推进）还是 **τ**
//! （翻转）→ 更新仓位。任何操作序列都是 h 和 τ 的组合（D∞ 的 word），引擎**不硬编码**特定循环。
//!
//! ```text
//! 每 bar 每级别 k：
//!   nf_sell[k]  → τ（手性翻转）+ 减仓 1/3 到次级别 k−1（σ⁻¹∘τ 下沉，穿手性缝）
//!   nf_buy[k]   → τ（翻转回来）+ 从次级别 k−1 回收 1/3（σ∘τ 升回）
//!   无信号       → h（走势推进，仓位不变）
//! ```
//!
//! ## 两投影（reinterp §1/§5：操作层 ⊥ 会计层）
//! 每个操作同时有**两个投影**：
//! - **操作投影**（D∞ 群作用于坐标 (φ,r,ε)）：τ 改 **direction**（ε 手性翻转）。
//! - **会计投影**（系数模 M=units）：τ 改 **units**（1/3 在相邻级别间转移）。
//! direction 与 units 正交（M ⊥ ε，reinterp §1.2）。
//!
//! ## 会计双重性（多方 / 空方）
//! 同一个 τ 操作的会计随 direction 镜像（`accounting::reduce_at`/`add_at`）：
//! - 多头层减仓 = 卖出（free += m·c）；空头层减仓 = 平空 cover（free −= m·c）。
//! - 加多头 = 买入（free −= m·c）；加空头 = 开空（free += m·c）。
//! 四者皆 NAV 中性（同价 c）。引擎不区分多/空方"视角"，只按 direction 施加双重会计。
//!
//! ## Layer = 五字段（无 CyclePhase——不硬编码四步相位）
//! `Layer{ladder, direction, units, basis, entry_bar}`。无 Holding/ShortActive 相位机。核心仓 H⁰
//! （2/3 恒持）**不是字段而是涌现**：每次 τ 只下沉 1/3（f=1/λ），顶层级别保留多数 ⟹ σ-塔自然
//! 分布（核心仓 = 顶层稀疏下沉的多数 f∝λ⁻ᴷ；机动仓 = 内层频繁循环 f∝λ⁻ᵏ，T50）。
//!
//! ## Σ|units| 守恒（T48 Casimir）+ 手性交替观测（T24，regime 依赖非不变量）
//! 每个 τ = reduce_at(源,|m|) + add_at(目标,|m|) 按**绝对值**转移 ⟹ **Σ|units| = n_base 严格守恒**
//! （T48 σ-不变 Casimir，与方向无关）——`prove_conservation` 守此。相邻级别方向交替**不是不变量**：
//! 建仓阶段（未出卖点触发 sink）各级别可同向做多（合法涌现），仅子级别承载下沉短差时才交替。
//! `count_chiral_violations` 观测之（非 panic，137号 make-decision-observable）。
//!
//! ## 信号层复用（DRY，bit-exact 兼容）
//! 复用 `crate::spiral::signal::{SignalState, GroupEventFrame}`（向心 confirm，T49）产
//! `nf_sell[k]/nf_buy[k]`（enable_macd_divergence=True ⟹ type1 背驰按 MACD 面积判定）。
//!
//! ## 三轴分离（operation_route_exhaustion §6B；用户裁决 2026-06-17）
//! H⁰（morphology）/ groupoid（observe）/ H¹（operate）通过 `axis.rs` trait 接口耦合，operate
//! 不直接 reach into 信号层内部。分离边界 = operate ⊥ (morphology, observe)。
//!
//! ## 认识论等级（formalization-validity-domain）
//! - D∞ word 处理器结构 / h·τ 群作用 / Σ|units| 守恒 / NAV 中性：**L0**（群论 + 守恒）。
//! - 哪条 word 此刻发声（激活）/ 相邻级别是否交替（手性）：**L2 regime 依赖**（reinterp §2.2 RF-NR2）。
//! - 回测 alpha：**L3**（真实数据，可否证；正/负域诚实报告）。
//!
//! ## gap（no-patch-mentality，保留不闭合）
//! - G1（整数股数 f64 掩盖，NR-7：H²=(ℤ/2)² 整数量子化残余）。
//! - G2（穿 ε=−1 做空载体：BTC 永续=真做空；现货退化平凡环路，§3.2）。
//! - G3（confirm 向心 vs 前向延异，已 escalate：2026-06-15-confirm-arming-differance.md）。

// GUARD-ROLE: t-engine-accounting-basis
//
// ## 名分（`docs/agents/generation-constitution.md` §1 名分五态，#762 C7-E3 执行票核定）
//
// - **名分**：**现役**（机械判据：有非测试调用者 ∧ 无 `#[deprecated]` 标记 ∧ 在唯一 git 线
//   main 上）。名分表（`.chanlun/review-results/legacy-generation-census-20260729.md` §1.2/§2）
//   原判本族"生产 0，PyO3 导出但零 python 调用方"——本票扩面复核（全仓 python 含
//   `analysis/`，非仅原表核查的 `trading_system/` 三份脚本）发现该"零调用"判断不成立，
//   存在两处真实调用方：`trading_system/backtest_fugue_v3.py:181,225`
//   `nr.FugueV3Stream(...)` / `nr.run_fugue_v3(...)`（`git log -1` = 2026-06-21）+
//   `analysis/t_vs_v3_comparison.py:81` `R.FugueV3Stream(floor_ladder=2)`（`git log -1` =
//   2026-06-18）。此外本族是 `recursive_t/`（下一族，同处置票 #762 处置范围）**T 引擎现役
//   支**（`stream.rs`/`t_engine.rs`/`prove_guards.rs`）的会计基座——`accounting`/`layer::
//   {FugueResult,Layer}`/`prove::prove_nav_neutral` 在 `t_engine.rs:47-52`、`stream.rs:27`、
//   `ffi.rs:18` 等生产路径（非 `#[cfg(test)]`）被直接 `use`，是编译期硬依赖。
// - **对照什么**：三条独立证据链均指向"现役、不可删"——(1)(2) 两处 python 直接调用；
//   (3) recursive_t T 引擎现役支的编译期硬依赖。theta_v0（π，唯一现役引擎）对本族生产
//   引用 = 0（§0 全局核查复验一致）。
// - **与现役差在哪**：π 完全自包含未复用本族一行；本族继续挂起等待"四族/散件"批次统一
//   处置（名分表 §5 执行票切票建议 N+3~N+5），非独立可判死代码。
// - **禁回灌**：本次仅加标记，未删除/未移动任何代码。
pub mod accounting;
pub mod axis;
pub mod cycle;
pub mod engine;
pub mod ffi;
pub mod layer;
pub mod morphology;
pub mod observe;
pub mod operate;
pub mod prove;

// 公开 API 再导出。
pub use axis::{MorphologyAxis, ObserveAxis, OperateAxis, StepOutcome};
pub use engine::FugueEngineCore;
pub use ffi::{run_fugue_v3, PyFugueV3Stream};
pub use layer::{FugueResult, Layer};
pub use morphology::MorphologyBridge;
pub use observe::ObserveBridge;
pub use operate::OperateEngine;

// ════════════════════════════ 常量 + 认识论标注 ════════════════════════════

pub use crate::trading::positional::EQUITY_SAMPLE_BARS;
pub use crate::trading::types::{FIRST_BSP_LADDER, INITIAL_CAPITAL, LADDER_MOVE, MAX_LADDER};

/// pending 势源下界（N5/N6）：move(L1)。segment(=FIRST_BSP_LADDER) 非势源。**L0**（T28）。
pub const PENDING_LO: usize = FIRST_BSP_LADDER + 1;

/// 尺度比 λ。**值 L2**（reinterp §5.2：026:80「用其中的 1/3」⟹ λ=3）。
///
/// **名分订正（#925 裁定，2026-08-07；不许再写成「原文规定」）**：`026:80` 是**原文举例、
/// 且原文同句明写可调**——「……但仓位可以控制，**例如**用其中的1/3，慢慢养成好习惯以后，
/// **就可以更随心所欲一点**」（「例如」与「可以更随心所欲一点」在同一句内）；同课 `026:447`
/// 【答疑】更明确「该以什么比例运用也是一个关键，**不熟练的情况下**，如果仓位不太大，
/// 1/3或1/4是比较合适的」⇒ **训练轮档位，非校准值**（`analysis/bc_architecture_research.md:325`
/// 逐字「原文数字不可当作校准值」）。另注：`026:80` 的「1/3」是**仓位比例**，与 542 定义的
/// 「级别时间尺度比」（T50 `Δt(k)∝λᵏ`）**不同量纲**——三个 λ 数字是否同一个量，#925 未合并。
/// 正本：`.chanlun/genealogy/settled/542-spawn-allocation-sigma-invariant.md` `## ★§925 订正`。
pub const LAMBDA: f64 = 3.0;

/// 单次 τ 转移配额 `f = 1/λ`（H¹ 系数，σ-不变唯一标量，T18×T48，542号）。
/// **形式 L0 / 待判**（势∝r ⟹ f=r_{k−1}/r_k=1/λ 几何强制，零自由度；但**否不掉也证不实**——
/// 否掉「级别无关」需证各档 `w` 不同，ADR 0017 裁定八已判上档测不出 ⇒ #925 标「形式层待判」）；
/// **值 L2**（λ=3 ⟹ 1/3；依据 `026:80` 是原文举例且明写可调，见 `LAMBDA` 头注的名分订正）。
///
/// **唯一源**：`recursive_t::rec_engine`（#943 前为私有 `1.0 / 3.0` 遮蔽本常数）与
/// `recursive_t::prove_guards::prove_sigma_quota` 现均引本常数——同源，结构上不可劈叉。
pub const MOBILE_FRAC: f64 = 1.0 / LAMBDA;

/// 成本门倍率（N4：θ < K×friction ⇒ 势幅度<成本，τ 不下沉）。**L2 不可消除**（势存在性需经验量）。
pub const SUB_COST_K: f64 = 2.0;

/// 单边往返摩擦率（N4 成本门基准）。**L2 不可消除**（ambient 市场摩擦）。
pub const SUB_FRICTION_RT: f64 = 0.001;

/// θ 分位（nearest-rank）。**L2**。
pub const SUB_COST_Q: f64 = 0.5;

/// θ 分位最小观测数（warm-up 保守拒绝下界）。**L2**。
pub const SUB_COST_MIN_OBS: usize = 10;

/// 强平阈值倍率（1x 逐仓：c ≥ SUB_LIQ_FACTOR×basis ⇒ 空头层清算）。**L2 设计缺口**
/// （ambient 市场微结构，不可从群结构推导；与 spiral 同口径）。
pub const SUB_LIQ_FACTOR: f64 = 2.0;
