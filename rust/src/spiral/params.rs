//! 螺旋引擎参数 + 认识论标注（架构 §10 零经验参数审计）。
//!
//! ## 核心诚实（formalization-validity-domain，架构 §10.2）
//! v2 **不是"零经验参数"**，而是"**结构参数零自由度（从群推导）+ 不可消除的 L2
//! 经验量（带认识论标注）**"。声称全零参数 = 声明膨胀（090号）。每个常量标注其
//! `ProveLevel` 等级与来源。
//!
//! | 层 | 参数 | 等级 |
//! |----|------|------|
//! | 结构（零自由度，从群推导） | 23 / FIRST_BSP_LADDER / PENDING_LO / `f=1/λ` 形式 | L0 |
//! | 不可消除（势存在性判定本质需经验） | 成本门 θ 的 4 常数、λ 的值、a0 | L2 |
//! | ambient 市场（与螺旋几何正交） | 强平阈值、摩擦率 | L2 |
//!
//! ## 强平阈值缺口（架构 §10.1，已升命名常量 + 待 escalate）
//! `SUB_LIQ_FACTOR=2.0`（1x 逐仓保证金率倒数）**不可从 23/11/2/Burnside/λ 任一群
//! 结构推导**——这是 ambient 市场微结构（`L_max=1/(D_struct+mm)`）。它藏在 A 强平
//! 块，正因 A 是非群操作（市场作用），其参数也是非群参数（与架构 §2.3 "A 进不了群"
//! 一致）。本文件升为命名常量 + L2 标注（重命名+标注，非新增；231号零新参数）。

pub use crate::trading::positional::EQUITY_SAMPLE_BARS;
pub use crate::trading::types::{FIRST_BSP_LADDER, INITIAL_CAPITAL, LADDER_MOVE, MAX_LADDER};

/// pending 势源下界（N5/N6）：move(L1)。segment(=FIRST_BSP_LADDER) 非势源。
/// **L0**（T28 move(L1)=势源下界，segment 非势源；零自由度）。
pub const PENDING_LO: usize = FIRST_BSP_LADDER + 1;

/// 尺度比 λ（A₅ 二分递归建模默认）。
/// **L2 待测**（待 T50 涌现 λ 测量精化；leverage_triad 唯一自由度占位）。
pub const LAMBDA: f64 = 2.0;

/// 降成本 spawn 配额比例 `f = 1/λ`（σ-不变，级别无关，T18×T48×T59，542号）。
/// **形式 L0**（势∝r 公理 ⟹ f=子势/父势=r_{k−1}/r_k=1/λ 几何强制，零自由度）；
/// **值 L2**（λ=2 ⟹ 0.5，A₅ 默认）。
pub const SUB_SPAWN_FRAC: f64 = 1.0 / LAMBDA;

/// 成本门倍率（N4：θ < K×friction ⇒ 势幅度<成本，递归终止）。
/// **L2 不可消除**（势存在性判定本质需经验量，R7/R6）。
pub const SUB_COST_K: f64 = 2.0;

/// 单边往返摩擦率（N4 成本门基准）。**L2 不可消除**（ambient 市场摩擦）。
pub const SUB_FRICTION_RT: f64 = 0.001;

/// θ 分位（nearest-rank）。**L2**。
pub const SUB_COST_Q: f64 = 0.5;

/// θ 分位最小观测数（warm-up 保守拒绝下界）。**L2**。
pub const SUB_COST_MIN_OBS: usize = 10;

/// 中枢振幅参照滚动窗口。**L2**。
pub const DEPTH_REF_WINDOW: usize = 50;

/// 强平阈值倍率（1x 逐仓：c≥SUB_LIQ_FACTOR×basis ⇒ capital 耗尽清算）。
/// **L2 设计缺口（架构 §10.1，待 escalate）**：ambient 市场微结构，不可从群结构推导。
/// 实际清算判据用 `capital + units×(basis − c) ≤ 0`（与 unn 一致）；此倍率仅作
/// 强平**记账价**（trade 行 exit_price = SUB_LIQ_FACTOR×basis），不影响现金守恒。
pub const SUB_LIQ_FACTOR: f64 = 2.0;

/// prove 守卫认识论等级（架构 §7.4，修复 unn 散落注释的结构债——集中声明）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProveLevel {
    /// 群关系/定义层，panic（`h²³=σ`, `τ²=e`, `Δr=−1`）。零信息增量。
    L0Structural,
    /// 会计守恒，每 bar panic（N1–N8）。8 标的真实数据 L2 验收。
    L2Conservation,
    /// regime 依赖，观测计数非 panic（T50/T57/T59）。
    L2Observation,
}
