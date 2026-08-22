//! 审计/统计载体（#1188 B01 职责块自 `nest_lifecycle.rs` 迁出，零行为）。

use super::*;

/// 倒退 prefix 显式拒绝注记（#78 修复 2：禁静默吸收）。
///
/// **本类型即内核注记**（票 #573 T1 重基）：三字段逐位同构——`key`、`last_as_of`
/// （该身份已见最大 as_of）、`rejected_as_of`（被拒绝的倒退 as_of）。
pub type RetrogradeRejection = LedgerRetrogradeRejection<LifecycleKey>;

/// 完成信号已到但力度不可验的独立审计事实（#428）。
///
/// 该事实只进入审计面，不改变 #78 的结算语义：对应 entry 仍为 Provisional，
/// 待力度材料补齐并重发完成信号后再按原协议结算。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionForceUnavailableAudit {
    pub key: LifecycleKey,
    pub as_of: usize,
    pub reason: UnavailReason,
}

/// 数据源首次给出结构完成信号的独立事实；按桥身份唯一。
///
/// 它与终态分开留档：身份可先由 Provisional→ForceOvertake 终局，之后到达的首完成仍须
/// 进入真实分母，但终态吸收禁止据此回填 `StructureCompleted`。
///
/// **完成钟 provenance 两分**（票 #527；#523 遗留问题 3；票 #559 条件 C3 订正）：两个时点
/// 分列，禁互相冒充——`completed_at`（lower unit **物理完成** bar，= 该单元 `end_index`）
/// ≤ `as_of`（**账本收到** bar，= `advance` 的时钟）。二者当前并不相等，故不得合并记账。
///
/// **C3 订正（为什么是两钟而不是三钟）**：#527 曾分列第三钟 `observed_completion_at`
/// （完成 Event 首次可见 bar）。但现行接线下 provider 相的构造与账本喂入在**同一 bar、
/// 同一调用链**内完成（`p123_fast_replay::lifecycle_bar_phases` → `feed_lifecycle_bar`），
/// 该钟被硬写为 `as_of`，BTC 100k 全量 248/248 恒等 ⟹ 它不是独立可测时点，只是 `as_of`
/// 的别名。保留一个恒等于另一字段的钟 = 声明代码不具备的分辨力（090 声明 = 能力）。
/// 故删除，只留两钟；`as_of − completed_at` 即完整的完成可见性滞后读数（口径不变）。
/// 若将来 provider 相构造与账本喂入解耦（批量/跨 bar 缓冲），第三钟才成为真实时点，
/// 届时按真实产出重新引入——**不预留空字段**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionSignal {
    pub key: LifecycleKey,
    /// 账本收到 bar（`advance` 时钟）。
    pub as_of: usize,
    /// 完成的 lower unit 身份（provider 查证给出，消费方不按 `kind` 猜 provenance）。
    pub completed_lower_id: ElementId,
    /// lower unit 物理完成 bar。
    pub completed_at: usize,
}

/// 一组寿命读数（单位为 `as_of` 的 bar-index 差，不冒充 trigger 次数）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LifetimeDistribution {
    pub count: usize,
    pub min: Option<usize>,
    pub median: Option<f64>,
    pub max: Option<usize>,
}

impl LifetimeDistribution {
    pub(super) fn from_values(mut values: Vec<usize>) -> Self {
        values.sort_unstable();
        let count = values.len();
        if count == 0 {
            return Self::default();
        }
        let median = if count % 2 == 0 {
            (values[count / 2 - 1] as f64 + values[count / 2] as f64) / 2.0
        } else {
            values[count / 2] as f64
        };
        Self {
            count,
            min: values.first().copied(),
            median: Some(median),
            max: values.last().copied(),
        }
    }
}

/// 生命周期终局与寿命的只读统计面；闪现（寿命 0）与非闪现分开。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LifecycleSettlementStats {
    pub entry_count: usize,
    pub first_provable_count: usize,
    pub provisional_count: usize,
    pub confirmed_count: usize,
    pub force_overtake_count: usize,
    /// 其中被**中枢升级认领**的前身数（票 #603 档 1）：该反超身份随后被一个同锚、B 更晚的
    /// 身份以 `CenterUpgraded` 认领 ⟹ 那条反超是「更精确的 B 出现之前的暂时状态」，
    /// 按 #599 §5-1 的口径正确性修复应从反超分母中剔除。
    ///
    /// **账本一个 bit 不改**：前身条目仍是 `Invalidated{ForceOvertake}` 原样留档（终态吸收/
    /// 禁复活/终态钟只写一次三条不变量不动）——纠误只发生在**口径层**，
    /// `force_overtake_count - force_overtake_claimed_count` 即纠误后的反超数。
    pub force_overtake_claimed_count: usize,
    pub never_constituted_count: usize,
    /// 身份消失 (a) 假设被推翻（真终局）——票 #559 裁定两类分列，禁合并计数。
    pub identity_vanished_refuted_count: usize,
    /// 身份消失 (b) 观测接缝伪影（provider 换轨丢下）——**不得**进入寿命/反超率统计
    /// （票 #559 裁定：混记会污染统计）。
    pub identity_vanished_seam_count: usize,
    pub flash_terminal_count: usize,
    pub nonflash_lifetime: LifetimeDistribution,
    pub force_overtake_lifetime: LifetimeDistribution,
}
