//! #1719 I4 组件：严格 Broken 原件、精确 B 绑定及第二层清账审计。
//! 尚未接正式 S 解码、持久事务或 launcher。

use super::super::center_lifecycle::{CenterId, CenterLifecycleEvent};
use super::entry::LifecycleRevision;
use super::key::LifecycleKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionAnchor {
    ChannelSwitch,
    CenterBreak,
}

/// 调用方提供已签具名版本；组件不代填版本或切换政策。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionPolicy {
    pub version: String,
    pub anchor: CompletionAnchor,
}

/// 只接受真实事件机输出中的 Broken。三类点确认源坐标保存在原件中；
/// physical_bar 由调用方按原输入映射提供，不能用消费钟冒充。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CenterBreakFact {
    event: CenterLifecycleEvent,
    physical_bar: usize,
    known_at: usize,
}

impl CenterBreakFact {
    pub fn from_event(
        event: &CenterLifecycleEvent,
        physical_bar: usize,
        known_at: usize,
    ) -> Option<Self> {
        match event {
            CenterLifecycleEvent::Broken { .. } => Some(Self {
                event: *event,
                physical_bar,
                known_at,
            }),
            _ => None,
        }
    }

    pub fn event(&self) -> &CenterLifecycleEvent {
        &self.event
    }

    pub fn physical_bar(&self) -> usize {
        self.physical_bar
    }

    pub fn known_at(&self) -> usize {
        self.known_at
    }

    pub(super) fn identity(&self) -> BreakIdentity {
        match self.event {
            CenterLifecycleEvent::Broken {
                level,
                center,
                chain_index,
                breaker_source_index,
                breaker_side,
            } => BreakIdentity {
                level,
                center_id: CenterId::of(&center),
                chain_index,
                breaker_source_index,
                side: super::key::side_tag(breaker_side),
            },
            _ => unreachable!("构造口只接受 Broken"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct BreakIdentity {
    pub level: u32,
    pub center_id: CenterId,
    pub chain_index: usize,
    pub breaker_source_index: usize,
    pub side: u8,
}

/// map 键是当前账本键；原对象/原注册钟/原政策永不覆盖。
/// binding_key 与 center_id 是最近一次由 provider 精确选中的 B，升级期间
/// 没有新的精确绑定时暂不清账，禁止按新 start 反查。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HypothesisBridge {
    pub lifecycle_key: LifecycleKey,
    pub original_center_id: CenterId,
    pub center_id: CenterId,
    pub binding_key: LifecycleKey,
    pub policy: CompletionPolicy,
    pub registered_at: usize,
}

impl HypothesisBridge {
    pub(super) fn selected(
        key: LifecycleKey,
        center_id: CenterId,
        policy: CompletionPolicy,
        registered_at: usize,
    ) -> Self {
        Self {
            lifecycle_key: key,
            original_center_id: center_id,
            center_id,
            binding_key: key,
            policy,
            registered_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct BreakApplicationKey {
    pub fact: BreakIdentity,
    pub original_key: LifecycleKey,
    pub policy_version: String,
}

/// 独立于原生命周期修订的清账凭证。保留原件、三钟、原对象及原政策，
/// 不伪造 StructureCompleted、交易否证或成交。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CenterBreakAudit {
    pub fact: CenterBreakFact,
    pub bridge: HypothesisBridge,
    pub settled_key: LifecycleKey,
    pub consumed_at: usize,
    pub physical_before_registration: bool,
}

/// #1719 重送绑定须显式报告冻结政策，不能把新政策报为已接纳。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeRegistration {
    Accepted,
    PolicyFrozen,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterBreakOutcome {
    Applied,
    NoTarget,
    NotReady,
    LatePendingRecovery,
    FactConflict,
    PolicyDisallowed,
    Preserved,
    AlreadyApplied,
    InvalidClock,
}

/// 每次消费的目标快照；迟到时保留拒绝所依据的门卫钟，供恢复按原钟重放。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CenterBreakTarget {
    pub key: LifecycleKey,
    pub bridge: HypothesisBridge,
    pub last_as_of: Option<usize>,
    pub physical_before_registration: bool,
}

/// 与实际清账凭证分开的追加审计，包含无目标、冲突和迟到待恢复。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CenterBreakAttempt {
    pub fact: CenterBreakFact,
    pub consumed_at: usize,
    pub target: Option<CenterBreakTarget>,
    pub outcome: CenterBreakOutcome,
}

/// 多目标可有不同结果；调用方须逐项消费 attempts，不能只看修订数。
#[derive(Debug, Clone, PartialEq)]
pub struct CenterBreakResult {
    pub revisions: Vec<LifecycleRevision>,
    pub attempts: Vec<CenterBreakAttempt>,
}
