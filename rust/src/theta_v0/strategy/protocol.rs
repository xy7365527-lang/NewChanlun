//! 盘整/趋势在线协议状态机（组合 R：DA-C / DA-E3 / DA-Q2）。
//!
//! 本模块与订单 P1..P10 正交：它只裁协议事件与 [`ProtocolMode`]，不读取、改写订单谓词。
//! 三类点只能构造 [`ProtocolMaturity::Type3Candidate`]，不能构造 `Trend`；`Trend` 的唯一
//! 激活入口是两个 canonical Completed [`Center`] 经 `central-ggdd-v1`
//! ([`classify_relation`]) 得到的 [`ProtocolEvent::settled_center_relation`]。

use super::super::classifier::center::{classify_relation, CenterRelation};
use super::super::types::{Center, Direction};

/// 稳定证据引用。`source_index` 是因果确认坐标，`generation` 区分同坐标重启/重放实例。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceRef {
    source_index: usize,
    generation: u32,
}

impl EvidenceRef {
    pub const fn new(source_index: usize, generation: u32) -> Self {
        Self {
            source_index,
            generation,
        }
    }

    pub const fn source_index(self) -> usize {
        self.source_index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

/// 已完成中枢的稳定身份。只能从 canonical `Center` 投影。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompletedCenterRef {
    start_index: usize,
    end_index: usize,
}

impl CompletedCenterRef {
    pub const fn from_center(center: &Center) -> Self {
        Self {
            start_index: center.start_index,
            end_index: center.end_index,
        }
    }

    pub const fn start_index(self) -> usize {
        self.start_index
    }

    pub const fn end_index(self) -> usize {
        self.end_index
    }
}

/// `Pending(PreTrend)` 的不可省略身份：级别 + 所属中枢 + 离开段 + 三类触发证据。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PreTrendKey {
    level: usize,
    center: CompletedCenterRef,
    departure: EvidenceRef,
    trigger: EvidenceRef,
}

impl PreTrendKey {
    pub const fn new(
        level: usize,
        center: CompletedCenterRef,
        departure: EvidenceRef,
        trigger: EvidenceRef,
    ) -> Self {
        Self {
            level,
            center,
            departure,
            trigger,
        }
    }

    pub const fn level(self) -> usize {
        self.level
    }

    pub const fn center(self) -> CompletedCenterRef {
        self.center
    }

    pub const fn departure(self) -> EvidenceRef {
        self.departure
    }

    pub const fn trigger(self) -> EvidenceRef {
        self.trigger
    }
}

/// `Pending(PostTrend)` 的不可省略身份：级别 + 前趋势锚中枢 + 前趋势走势 + 触发证据。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PostTrendKey {
    level: usize,
    prior_center: CompletedCenterRef,
    trend_move: EvidenceRef,
    trigger: EvidenceRef,
}

impl PostTrendKey {
    pub const fn new(
        level: usize,
        prior_center: CompletedCenterRef,
        trend_move: EvidenceRef,
        trigger: EvidenceRef,
    ) -> Self {
        Self {
            level,
            prior_center,
            trend_move,
            trigger,
        }
    }

    pub const fn level(self) -> usize {
        self.level
    }

    pub const fn prior_center(self) -> CompletedCenterRef {
        self.prior_center
    }

    pub const fn trend_move(self) -> EvidenceRef {
        self.trend_move
    }

    pub const fn trigger(self) -> EvidenceRef {
        self.trigger
    }
}

/// Pending 原因位集。并发同目的原因按位并集，天然交换、结合、幂等。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReasonSet(u8);

impl ReasonSet {
    pub const TYPE3: Self = Self(1 << 0);
    pub const TREND_DIVERGENCE: Self = Self(1 << 1);
    pub const COMPLETED_MOVE: Self = Self(1 << 2);
    pub const COMPLETED_RETRACE: Self = Self(1 << 3);
    pub const COMPLETED_REENTRY: Self = Self(1 << 4);
    pub const SETTLED_RELATION: Self = Self(1 << 5);

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains(self, reason: Self) -> bool {
        self.0 & reason.0 == reason.0
    }
}

/// Pending 类型本身携身份和原因；不存在可裸构造的 `Pending` 单元枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PendingIdentity {
    PreTrend(PreTrendKey),
    PostTrend(PostTrendKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PendingProtocol {
    identity: PendingIdentity,
    reasons: ReasonSet,
}

impl PendingProtocol {
    pub const fn identity(self) -> PendingIdentity {
        self.identity
    }

    pub const fn reasons(self) -> ReasonSet {
        self.reasons
    }

    const fn pre(key: PreTrendKey, reasons: ReasonSet) -> Self {
        Self {
            identity: PendingIdentity::PreTrend(key),
            reasons,
        }
    }

    const fn post(key: PostTrendKey, reasons: ReasonSet) -> Self {
        Self {
            identity: PendingIdentity::PostTrend(key),
            reasons,
        }
    }
}

/// 任一时刻恰一协议态。Rust 和类型保证 `Pending` 必带 [`PendingProtocol`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolMode {
    Consolidation,
    Trend(Direction),
    Pending(PendingProtocol),
}

/// 协议事件证据成熟度全序（数值越大越成熟）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProtocolMaturity {
    Hold,
    Type3Candidate,
    CompletedRetraceReentry,
    CompletedMoveEvidence,
    SettledCenterRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SettledRelationEvidence {
    level: usize,
    previous: CompletedCenterRef,
    successor: CompletedCenterRef,
    relation: CenterRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum EventPayload {
    Hold { level: usize },
    Type3Candidate(PreTrendKey),
    CompletedRetraceReentry(PostTrendKey),
    CompletedMoveEvidence(PostTrendKey),
    SettledCenterRelation(SettledRelationEvidence),
}

/// 协议轨的显式全域输出。`Hold` 是一等构造子，不使用 `Option`/缺省表达。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProtocolEvent {
    payload: EventPayload,
    reasons: ReasonSet,
}

impl ProtocolEvent {
    pub const fn hold(level: usize) -> Self {
        Self {
            payload: EventPayload::Hold { level },
            reasons: ReasonSet(0),
        }
    }

    pub const fn type3_candidate(key: PreTrendKey) -> Self {
        Self {
            payload: EventPayload::Type3Candidate(key),
            reasons: ReasonSet::TYPE3,
        }
    }

    /// 趋势背驰/完成走势证据：只产 `Pending(PostTrend)`，不直接产盘整。
    pub const fn completed_move_evidence(key: PostTrendKey) -> Self {
        Self {
            payload: EventPayload::CompletedMoveEvidence(key),
            reasons: ReasonSet::TREND_DIVERGENCE.union(ReasonSet::COMPLETED_MOVE),
        }
    }

    pub const fn completed_retrace(key: PostTrendKey) -> Self {
        Self {
            payload: EventPayload::CompletedRetraceReentry(key),
            reasons: ReasonSet::COMPLETED_RETRACE,
        }
    }

    pub const fn completed_reentry(key: PostTrendKey) -> Self {
        Self {
            payload: EventPayload::CompletedRetraceReentry(key),
            reasons: ReasonSet::COMPLETED_REENTRY,
        }
    }

    /// `Trend` 的唯一激活证据构造器：两个 canonical Completed 中枢，同一个显式 `level`，
    /// 关系值只能由现有 central-ggdd-v1 `classify_relation` 算出。
    pub fn settled_center_relation(level: usize, previous: &Center, successor: &Center) -> Self {
        Self {
            payload: EventPayload::SettledCenterRelation(SettledRelationEvidence {
                level,
                previous: CompletedCenterRef::from_center(previous),
                successor: CompletedCenterRef::from_center(successor),
                relation: classify_relation(previous, successor),
            }),
            reasons: ReasonSet::SETTLED_RELATION,
        }
    }

    pub const fn maturity(self) -> ProtocolMaturity {
        match self.payload {
            EventPayload::Hold { .. } => ProtocolMaturity::Hold,
            EventPayload::Type3Candidate(_) => ProtocolMaturity::Type3Candidate,
            EventPayload::CompletedRetraceReentry(_) => ProtocolMaturity::CompletedRetraceReentry,
            EventPayload::CompletedMoveEvidence(_) => ProtocolMaturity::CompletedMoveEvidence,
            EventPayload::SettledCenterRelation(_) => ProtocolMaturity::SettledCenterRelation,
        }
    }

    pub const fn reasons(self) -> ReasonSet {
        self.reasons
    }

    pub const fn level(self) -> usize {
        match self.payload {
            EventPayload::Hold { level } => level,
            EventPayload::Type3Candidate(key) => key.level(),
            EventPayload::CompletedRetraceReentry(key)
            | EventPayload::CompletedMoveEvidence(key) => key.level(),
            EventPayload::SettledCenterRelation(evidence) => evidence.level,
        }
    }

    /// 固定全序合并：成熟度优先，同成熟度按完整 payload 稳定裁决；原因总是取并集。
    /// `max` 与位并集均交换、结合、幂等，因此本运算也满足三律。
    pub fn merge(self, other: Self) -> Self {
        let lhs_key = (self.maturity(), self.payload);
        let rhs_key = (other.maturity(), other.payload);
        let payload = if lhs_key >= rhs_key {
            self.payload
        } else {
            other.payload
        };
        Self {
            payload,
            reasons: self.reasons.union(other.reasons),
        }
    }
}

/// 非空协议候选集合的折叠器。构造时即含显式 Hold，因此空业务证据也有像。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolEventSet {
    selected: ProtocolEvent,
}

impl ProtocolEventSet {
    pub const fn hold(level: usize) -> Self {
        Self {
            selected: ProtocolEvent::hold(level),
        }
    }

    pub fn with(self, event: ProtocolEvent) -> Self {
        Self {
            selected: self.selected.merge(event),
        }
    }

    pub const fn selected(self) -> ProtocolEvent {
        self.selected
    }
}

/// 单级别协议状态。字段私有，Pending 只能经带身份的协议事件转移产生。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProtocolState {
    level: usize,
    mode: ProtocolMode,
}

impl ProtocolState {
    pub const fn initial(level: usize) -> Self {
        Self {
            level,
            mode: ProtocolMode::Consolidation,
        }
    }

    pub const fn level(self) -> usize {
        self.level
    }

    pub const fn mode(self) -> ProtocolMode {
        self.mode
    }

    /// 总转移函数。错级别或错 identity 的事件不消费当前 Pending，而是显式保持原态。
    pub fn transition(self, event: ProtocolEvent) -> Self {
        if event.level() != self.level {
            return self;
        }
        let mode = match event.payload {
            EventPayload::Hold { .. } => self.mode,
            EventPayload::Type3Candidate(key) => match self.mode {
                ProtocolMode::Consolidation => {
                    ProtocolMode::Pending(PendingProtocol::pre(key, event.reasons))
                }
                ProtocolMode::Pending(p) if p.identity == PendingIdentity::PreTrend(key) => {
                    ProtocolMode::Pending(PendingProtocol::pre(key, p.reasons.union(event.reasons)))
                }
                // Type3 在既有 Trend/PostTrend 中只是候选，不得改写 settled truth。
                _ => self.mode,
            },
            EventPayload::CompletedRetraceReentry(key)
            | EventPayload::CompletedMoveEvidence(key) => match self.mode {
                ProtocolMode::Trend(_) => {
                    ProtocolMode::Pending(PendingProtocol::post(key, event.reasons))
                }
                ProtocolMode::Pending(p) if p.identity == PendingIdentity::PostTrend(key) => {
                    ProtocolMode::Pending(PendingProtocol::post(
                        key,
                        p.reasons.union(event.reasons),
                    ))
                }
                _ => self.mode,
            },
            EventPayload::SettledCenterRelation(evidence) => {
                let identity_matches = match self.mode {
                    ProtocolMode::Pending(p) => match p.identity {
                        PendingIdentity::PreTrend(key) => key.center == evidence.previous,
                        PendingIdentity::PostTrend(key) => key.prior_center == evidence.previous,
                    },
                    ProtocolMode::Consolidation | ProtocolMode::Trend(_) => true,
                };
                if !identity_matches {
                    self.mode
                } else {
                    match evidence.relation {
                        CenterRelation::UpContinuation => ProtocolMode::Trend(Direction::Up),
                        CenterRelation::DownContinuation => ProtocolMode::Trend(Direction::Down),
                        CenterRelation::LevelExpansion => ProtocolMode::Consolidation,
                    }
                }
            }
        };
        Self {
            level: self.level,
            mode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn center(start: usize, end: usize, dd: i64, gg: i64) -> Center {
        Center {
            zd: dd,
            zg: gg,
            dd,
            gg,
            start_index: start,
            end_index: end,
        }
    }

    fn pre(level: usize, c: &Center, seed: usize) -> PreTrendKey {
        PreTrendKey::new(
            level,
            CompletedCenterRef::from_center(c),
            EvidenceRef::new(seed, 0),
            EvidenceRef::new(seed + 1, 0),
        )
    }

    fn post(level: usize, c: &Center, seed: usize) -> PostTrendKey {
        PostTrendKey::new(
            level,
            CompletedCenterRef::from_center(c),
            EvidenceRef::new(seed, 0),
            EvidenceRef::new(seed + 1, 0),
        )
    }

    fn trend_state(level: usize, direction: Direction) -> ProtocolState {
        let previous = center(0, 10, 90, 120);
        let successor = match direction {
            Direction::Up => center(20, 30, 125, 145),
            Direction::Down => center(20, 30, 60, 85),
        };
        ProtocolState::initial(level).transition(ProtocolEvent::settled_center_relation(
            level, &previous, &successor,
        ))
    }

    fn event_suite() -> Vec<ProtocolEvent> {
        let c0 = center(0, 10, 90, 120);
        let c1 = center(20, 30, 130, 150);
        let pk = post(0, &c0, 40);
        vec![
            ProtocolEvent::hold(0),
            ProtocolEvent::type3_candidate(pre(0, &c0, 11)),
            ProtocolEvent::completed_retrace(pk),
            ProtocolEvent::completed_reentry(pk),
            ProtocolEvent::completed_move_evidence(pk),
            ProtocolEvent::settled_center_relation(0, &c0, &c1),
        ]
    }

    #[test]
    fn protocol_mode_total_exclusive_reachable_partition() {
        let mut reachable = std::collections::HashSet::from([
            ProtocolState::initial(0),
            trend_state(0, Direction::Up),
            trend_state(0, Direction::Down),
        ]);
        for _ in 0..4 {
            let snapshot: Vec<_> = reachable.iter().copied().collect();
            for state in snapshot {
                for event in event_suite() {
                    reachable.insert(state.transition(event));
                }
            }
        }
        assert!(!reachable.is_empty());
        for state in reachable {
            let partition_count = match state.mode() {
                ProtocolMode::Consolidation | ProtocolMode::Trend(_) | ProtocolMode::Pending(_) => {
                    1
                }
            };
            assert_eq!(partition_count, 1, "任意可达状态恰一 ProtocolMode");
        }
    }

    #[test]
    fn pending_identity_bound_by_type_and_constructive() {
        let c0 = center(0, 10, 90, 120);
        let key = pre(2, &c0, 11);
        let state = ProtocolState::initial(2).transition(ProtocolEvent::type3_candidate(key));
        let ProtocolMode::Pending(pending) = state.mode() else {
            panic!("Type3 必须进入带身份 Pending")
        };
        assert_eq!(pending.identity(), PendingIdentity::PreTrend(key));
        assert_eq!(key.level(), 2);
        assert_eq!(key.center(), CompletedCenterRef::from_center(&c0));
        assert_eq!(key.departure(), EvidenceRef::new(11, 0));
        assert_eq!(key.trigger(), EvidenceRef::new(12, 0));
        // 类型上不存在 `ProtocolMode::Pending` 的零字段构造；必须提供 PendingProtocol。
    }

    #[test]
    fn protocol_event_priority_total_and_merge_laws() {
        let events = event_suite();
        for &a in &events {
            assert_eq!(a.merge(a), a, "幂等律");
            for &b in &events {
                assert_eq!(a.merge(b), b.merge(a), "交换律");
                for &c in &events {
                    assert_eq!(a.merge(b).merge(c), a.merge(b.merge(c)), "结合律");
                }
            }
        }
        let maturities: Vec<_> = events.iter().map(|e| e.maturity()).collect();
        assert!(maturities.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn protocol_settled_relation_beats_candidate() {
        let c0 = center(0, 10, 90, 120);
        let c1 = center(20, 30, 130, 150);
        let candidate = ProtocolEvent::type3_candidate(pre(0, &c0, 11));
        let settled = ProtocolEvent::settled_center_relation(0, &c0, &c1);
        let selected = ProtocolEventSet::hold(0)
            .with(candidate)
            .with(settled)
            .selected();
        assert_eq!(selected.maturity(), ProtocolMaturity::SettledCenterRelation);
        assert!(
            selected.reasons().contains(ReasonSet::TYPE3),
            "低成熟度候选 provenance 不丢"
        );
        assert!(selected.reasons().contains(ReasonSet::SETTLED_RELATION));
    }

    #[test]
    fn same_bar_pending_reasons_commute() {
        let c0 = center(0, 10, 90, 120);
        let key = post(0, &c0, 20);
        let divergence = ProtocolEvent::completed_move_evidence(key);
        let reentry = ProtocolEvent::completed_reentry(key);
        let lhs = divergence.merge(reentry);
        let rhs = reentry.merge(divergence);
        assert_eq!(lhs, rhs);
        assert!(lhs.reasons().contains(ReasonSet::TREND_DIVERGENCE));
        assert!(lhs.reasons().contains(ReasonSet::COMPLETED_REENTRY));
    }

    #[test]
    fn type3_enters_pretrend_not_trend() {
        let c0 = center(0, 10, 90, 120);
        let state =
            ProtocolState::initial(0).transition(ProtocolEvent::type3_candidate(pre(0, &c0, 11)));
        assert!(matches!(
            state.mode(),
            ProtocolMode::Pending(PendingProtocol {
                identity: PendingIdentity::PreTrend(_),
                ..
            })
        ));
        assert!(!matches!(state.mode(), ProtocolMode::Trend(_)));
    }

    #[test]
    fn pretrend_newborn_settles_direction_only_by_completed_ggdd() {
        let c0 = center(0, 10, 90, 120);
        let c1 = center(20, 30, 125, 145); // next.dd > prev.gg
        let pending =
            ProtocolState::initial(0).transition(ProtocolEvent::type3_candidate(pre(0, &c0, 11)));
        let settled = pending.transition(ProtocolEvent::settled_center_relation(0, &c0, &c1));
        assert_eq!(settled.mode(), ProtocolMode::Trend(Direction::Up));
    }

    #[test]
    fn pretrend_expansion_rolls_to_consolidation() {
        let c0 = center(0, 10, 90, 120);
        let c1 = center(20, 30, 110, 140); // 外缘重叠
        let pending =
            ProtocolState::initial(0).transition(ProtocolEvent::type3_candidate(pre(0, &c0, 11)));
        let settled = pending.transition(ProtocolEvent::settled_center_relation(0, &c0, &c1));
        assert_eq!(settled.mode(), ProtocolMode::Consolidation);
    }

    #[test]
    fn trend_divergence_and_reentry_merge_posttrend_reasons() {
        let c0 = center(0, 10, 90, 120);
        let key = post(0, &c0, 20);
        let merged = ProtocolEvent::completed_move_evidence(key)
            .merge(ProtocolEvent::completed_reentry(key));
        assert_eq!(merged.maturity(), ProtocolMaturity::CompletedMoveEvidence);
        assert!(merged.reasons().contains(ReasonSet::TREND_DIVERGENCE));
        assert!(merged.reasons().contains(ReasonSet::COMPLETED_REENTRY));
        let state = trend_state(0, Direction::Up).transition(merged);
        let ProtocolMode::Pending(pending) = state.mode() else {
            panic!("趋势完成证据必须进入 PostTrend Pending")
        };
        assert_eq!(pending.identity(), PendingIdentity::PostTrend(key));
        assert!(pending.reasons().contains(ReasonSet::COMPLETED_REENTRY));
    }

    #[test]
    fn posttrend_settles_continuation_reversal_or_expansion() {
        let previous = center(0, 10, 90, 120);
        let key = post(0, &previous, 20);
        let pending =
            trend_state(0, Direction::Up).transition(ProtocolEvent::completed_move_evidence(key));
        assert!(matches!(
            pending.mode(),
            ProtocolMode::Pending(PendingProtocol {
                identity: PendingIdentity::PostTrend(_),
                ..
            })
        ));

        let up = center(20, 30, 125, 145);
        let down = center(20, 30, 60, 85);
        let overlap = center(20, 30, 110, 140);
        assert_eq!(
            pending
                .transition(ProtocolEvent::settled_center_relation(0, &previous, &up))
                .mode(),
            ProtocolMode::Trend(Direction::Up),
            "与原 Up 同向，归类为延续",
        );
        assert_eq!(
            pending
                .transition(ProtocolEvent::settled_center_relation(0, &previous, &down))
                .mode(),
            ProtocolMode::Trend(Direction::Down),
            "与原 Up 反向，归类为转折",
        );
        assert_eq!(
            pending
                .transition(ProtocolEvent::settled_center_relation(
                    0, &previous, &overlap
                ))
                .mode(),
            ProtocolMode::Consolidation,
            "外缘重叠，归类为扩展/盘整",
        );
    }

    #[test]
    fn pending_identity_never_crosses_departure() {
        let c0 = center(0, 10, 90, 120);
        let unrelated = center(40, 50, 130, 150);
        let successor = center(60, 70, 160, 180);
        let pending =
            ProtocolState::initial(0).transition(ProtocolEvent::type3_candidate(pre(0, &c0, 11)));
        let after = pending.transition(ProtocolEvent::settled_center_relation(
            0, &unrelated, &successor,
        ));
        assert_eq!(
            after, pending,
            "无关中枢关系不得消费另一 departure 的 Pending"
        );
    }

    #[test]
    fn hold_is_explicit_and_preserves_mode() {
        let state = trend_state(3, Direction::Down);
        let hold = ProtocolEventSet::hold(3).selected();
        assert_eq!(hold.maturity(), ProtocolMaturity::Hold);
        assert_eq!(state.transition(hold), state);
    }
}
