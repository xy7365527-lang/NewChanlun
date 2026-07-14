//! C2 CompletedMove 的版本化消费层接线（task #71）。
//!
//! 本模块不改变 [`super::recursive_tower`]。它只接受在查询时点实际构造的 exact-three
//! 快照，把 #58 的 [`super::move_view::assemble_move_view`] 提升为唯一生产 seam，并在 seam
//! 外层施加对象宇宙、版本、actual-prefix、shadow replay 与 CompletedFreeze 门。

use super::super::types::Direction;
use super::decompose::{center_own_dir_at, MoveBlock};
use super::descend::RMove;
use super::move_view::{
    assemble_move_view, AssembledMove, ChainLink, CompletionStatus, CoordinateWindow,
    LowerComponent, MacdDivergenceMaterial, MoveView, MoveViewMaterial, TowerWindowUnit,
};
use super::recursive_tower::LeveledMove;
use std::collections::{BTreeMap, BTreeSet};

/// 物理对象宇宙。两个变体只允许显式比较，不允许在同一 C2 view 中混算。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectUniverse {
    /// #58 / C2 的旧主树 exact-three WindowUnit 基线。
    MainExactThreeV1,
    /// #54 的可变长窗口 + ownership 审计基线，仅作外部诊断标签。
    V54VariableWindowOwnership,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssemblySeamVersion {
    C2V1,
}

/// 划分策略与塔/组装 seam 独立版本化；数值允许 shadow policy 先注册后实现。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PartitionPolicyVersion(pub u32);

impl PartitionPolicyVersion {
    pub const GREEDY_V1: Self = Self(1);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuleVersion(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProviderVersion(pub u32);

/// 方向仍未裁决为唯一事实；四个确定性 provider 必须显式 pin。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirectionProviderVersion {
    /// 有本宇宙 ownership 则使用；没有则在同一宇宙内退回 sequence envelope。
    OwnershipFallbackV1,
    FirstLeafV1,
    FirstLastEnvelopeV1,
    SequenceEnvelopeV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ViewVersion {
    pub assembly_seam: AssemblySeamVersion,
    pub partition_policy: PartitionPolicyVersion,
    pub rule_version: RuleVersion,
    pub direction_provider: DirectionProviderVersion,
    /// `None` 表示 A/C hook 未安装；趋势必须保持 Pending。
    pub divergence_pair_provider: Option<ProviderVersion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelViewQuery {
    pub level: u32,
    pub coordinate_window: CoordinateWindow,
    pub as_of: usize,
    pub universe: ObjectUniverse,
    pub version: ViewVersion,
}

/// 调用方必须从 raw prefix（或同源逐 bar 增量状态）在 `observed_at` 构造本快照。
/// `observed_at != query.as_of` 会 fail closed，禁止 history-end 对象按 end 坐标倒滤，
/// 也禁止把较早快照冒充当前查询时点。
#[derive(Debug, Clone, Copy)]
pub struct ExactThreeMaterialSnapshot<'a> {
    pub universe: ObjectUniverse,
    pub observed_at: usize,
    pub tower_windows: &'a [LeveledMove],
    pub lower_ledger: &'a [LeveledMove],
    /// 与本 exact-three 窗口同一构造链的 ownership；缺失时只触发显式 fallback。
    pub ownership_blocks: Option<&'a [MoveBlock]>,
    pub divergence: Option<MacdDivergenceMaterial<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelViewError {
    ObjectUniverseMismatch {
        expected: ObjectUniverse,
        actual: ObjectUniverse,
    },
    UnsupportedPartitionPolicy(PartitionPolicyVersion),
    SnapshotTimeMismatch {
        observed_at: usize,
        as_of: usize,
    },
    InvalidLevel(u32),
    NonExactThreeWindow {
        index: usize,
        sub_count: usize,
    },
    InvalidTowerWindow {
        index: usize,
    },
    UnsortedMaterial,
    DivergenceProviderMismatch,
    IncomparableShadowReplay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelAsOfView {
    pub query: LevelViewQuery,
    pub view: MoveView,
}

fn first_leaf_direction(value: &LeveledMove) -> Direction {
    fn go(value: &RMove) -> Direction {
        match value {
            RMove::Segment { direction, .. } => *direction,
            RMove::Compose { subs, .. } => subs.first().map(go).unwrap_or(Direction::Up),
        }
    }
    go(&value.rmove)
}

fn direction_for(
    provider: DirectionProviderVersion,
    index: usize,
    windows: &[LeveledMove],
    ownership_blocks: Option<&[MoveBlock]>,
) -> Direction {
    let sequence =
        || windows[index].fold_direction(index.checked_sub(1).and_then(|i| windows.get(i)));
    match provider {
        DirectionProviderVersion::OwnershipFallbackV1 => ownership_blocks
            .and_then(|blocks| center_own_dir_at(blocks, index))
            .unwrap_or_else(sequence),
        DirectionProviderVersion::FirstLeafV1 => first_leaf_direction(&windows[index]),
        DirectionProviderVersion::FirstLastEnvelopeV1 => {
            match (
                windows[index].sub_moves.first(),
                windows[index].sub_moves.last(),
            ) {
                (Some(first), Some(last)) if last.rmove.hi() >= first.rmove.hi() => Direction::Up,
                (Some(_), Some(_)) => Direction::Down,
                _ => first_leaf_direction(&windows[index]),
            }
        }
        DirectionProviderVersion::SequenceEnvelopeV1 => sequence(),
    }
}

fn sorted_by_coordinates(values: &[LeveledMove]) -> bool {
    values.windows(2).all(|pair| {
        (pair[0].start_index, pair[0].end_index) <= (pair[1].start_index, pair[1].end_index)
    })
}

/// C2 唯一生产 seam。塔不被修改，输出只存在于消费层。
pub fn assemble_level_view(
    query: LevelViewQuery,
    material: ExactThreeMaterialSnapshot<'_>,
) -> Result<LevelAsOfView, LevelViewError> {
    if material.universe != query.universe {
        return Err(LevelViewError::ObjectUniverseMismatch {
            expected: query.universe,
            actual: material.universe,
        });
    }
    if query.universe != ObjectUniverse::MainExactThreeV1 {
        return Err(LevelViewError::ObjectUniverseMismatch {
            expected: ObjectUniverse::MainExactThreeV1,
            actual: query.universe,
        });
    }
    if query.version.partition_policy != PartitionPolicyVersion::GREEDY_V1 {
        return Err(LevelViewError::UnsupportedPartitionPolicy(
            query.version.partition_policy,
        ));
    }
    if material.observed_at != query.as_of {
        return Err(LevelViewError::SnapshotTimeMismatch {
            observed_at: material.observed_at,
            as_of: query.as_of,
        });
    }
    if query.level == 0 {
        return Err(LevelViewError::InvalidLevel(query.level));
    }
    if !sorted_by_coordinates(material.tower_windows)
        || !sorted_by_coordinates(material.lower_ledger)
    {
        return Err(LevelViewError::UnsortedMaterial);
    }
    if query.version.divergence_pair_provider.is_some() != material.divergence.is_some() {
        return Err(LevelViewError::DivergenceProviderMismatch);
    }

    let mut tower_windows = Vec::with_capacity(material.tower_windows.len());
    for (index, window) in material.tower_windows.iter().enumerate() {
        if window.sub_moves.len() != 3 {
            return Err(LevelViewError::NonExactThreeWindow {
                index,
                sub_count: window.sub_moves.len(),
            });
        }
        let direction = direction_for(
            query.version.direction_provider,
            index,
            material.tower_windows,
            material.ownership_blocks,
        );
        tower_windows.push(
            TowerWindowUnit::from_leveled(window, direction)
                .ok_or(LevelViewError::InvalidTowerWindow { index })?,
        );
    }
    let lower_ledger: Vec<LowerComponent> = material
        .lower_ledger
        .iter()
        .map(|component| LowerComponent {
            start_index: component.start_index,
            end_index: component.end_index,
            direction: first_leaf_direction(component),
            lo: component.rmove.lo(),
            hi: component.rmove.hi(),
        })
        .collect();
    let view = assemble_move_view(
        MoveViewMaterial {
            tower_windows: &tower_windows,
            lower_ledger: &lower_ledger,
            divergence: material.divergence,
        },
        query.level,
        query.coordinate_window,
        query.as_of,
    );
    Ok(LevelAsOfView { query, view })
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CompletedMoveId {
    pub level: u32,
    pub start_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenCompletedMove {
    pub id: CompletedMoveId,
    pub judge_at: usize,
    pub entry_bar: Option<usize>,
    pub end_index: usize,
    pub direction: Direction,
    pub kind: super::move_view::MoveViewKind,
    pub chain: Vec<ChainLink>,
}

impl FrozenCompletedMove {
    fn from_move(level: u32, observed_at: usize, value: &AssembledMove) -> Option<Self> {
        matches!(value.completion, CompletionStatus::Completed { .. }).then(|| Self {
            id: CompletedMoveId {
                level,
                start_index: value.start_index,
            },
            judge_at: observed_at,
            entry_bar: observed_at.checked_add(1),
            end_index: value.end_index,
            direction: value.direction,
            kind: value.kind,
            chain: value.chain.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletedFreezeViolation {
    VersionChanged {
        expected: ViewVersion,
        actual: ViewVersion,
    },
    UniverseChanged {
        expected: ObjectUniverse,
        actual: ObjectUniverse,
    },
    CompletedMoveMissing(CompletedMoveId),
    CompletionDowngraded(CompletedMoveId),
    ShapeChanged(CompletedMoveId),
    ChainOrMembershipChanged(CompletedMoveId),
}

/// consumer seam 内的追加式完成事件。持久化由外层 event store 负责；本层永不改写旧项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedFreezeEvent {
    pub sequence: u64,
    pub created_at: usize,
    pub snapshot: FrozenCompletedMove,
}

/// 单一完整 ViewVersion 的 CompletedFreeze。Pending 不入账，可随追加历史演化。
#[derive(Debug, Clone)]
pub struct CompletedFreezeGuard {
    version: ViewVersion,
    universe: ObjectUniverse,
    frozen: BTreeMap<CompletedMoveId, FrozenCompletedMove>,
    events: Vec<CompletedFreezeEvent>,
}

impl CompletedFreezeGuard {
    pub fn new(version: ViewVersion, universe: ObjectUniverse) -> Self {
        Self {
            version,
            universe,
            frozen: BTreeMap::new(),
            events: Vec::new(),
        }
    }

    pub fn frozen(&self) -> &BTreeMap<CompletedMoveId, FrozenCompletedMove> {
        &self.frozen
    }

    pub fn events(&self) -> &[CompletedFreezeEvent] {
        &self.events
    }

    /// 先验证全部旧 Completed，全部通过后才原子加入本轮新 Completed。
    pub fn observe(&mut self, current: &LevelAsOfView) -> Result<(), CompletedFreezeViolation> {
        if current.query.version != self.version {
            return Err(CompletedFreezeViolation::VersionChanged {
                expected: self.version,
                actual: current.query.version,
            });
        }
        if current.query.universe != self.universe {
            return Err(CompletedFreezeViolation::UniverseChanged {
                expected: self.universe,
                actual: current.query.universe,
            });
        }

        for (id, frozen) in &self.frozen {
            let Some(now) = current
                .view
                .moves
                .iter()
                .find(|value| value.start_index == id.start_index)
            else {
                return Err(CompletedFreezeViolation::CompletedMoveMissing(id.clone()));
            };
            if !matches!(now.completion, CompletionStatus::Completed { .. }) {
                return Err(CompletedFreezeViolation::CompletionDowngraded(id.clone()));
            }
            if now.end_index != frozen.end_index
                || now.direction != frozen.direction
                || now.kind != frozen.kind
            {
                return Err(CompletedFreezeViolation::ShapeChanged(id.clone()));
            }
            if now.chain != frozen.chain {
                return Err(CompletedFreezeViolation::ChainOrMembershipChanged(
                    id.clone(),
                ));
            }
        }

        let new_completed: Vec<_> = current
            .view
            .moves
            .iter()
            .filter_map(|value| {
                FrozenCompletedMove::from_move(current.query.level, current.query.as_of, value)
            })
            .collect();
        for value in new_completed {
            if !self.frozen.contains_key(&value.id) {
                self.events.push(CompletedFreezeEvent {
                    sequence: self.events.len() as u64,
                    created_at: current.query.as_of,
                    snapshot: value.clone(),
                });
                self.frozen.insert(value.id.clone(), value);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MembershipKey {
    pub start_index: usize,
    pub end_index: usize,
}

fn membership_keys(view: &LevelAsOfView) -> BTreeSet<MembershipKey> {
    view.view
        .moves
        .iter()
        .flat_map(|host| host.chain.iter())
        .map(|link| MembershipKey {
            start_index: link.component.start_index,
            end_index: link.component.end_index,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowReplayAssessment {
    pub regressions_to_unassigned: Vec<MembershipKey>,
    pub completed_freeze_violations: Vec<CompletedMoveId>,
}

impl ShadowReplayAssessment {
    pub fn no_rollback(&self) -> bool {
        self.regressions_to_unassigned.is_empty() && self.completed_freeze_violations.is_empty()
    }
}

/// A4 shadow gate：替代 PartitionPolicy 只能增加归属，且不得改变基线 Completed。
pub fn assess_shadow_replay(
    baseline: &LevelAsOfView,
    candidate: &LevelAsOfView,
) -> Result<ShadowReplayAssessment, LevelViewError> {
    if baseline.query.universe != candidate.query.universe {
        return Err(LevelViewError::ObjectUniverseMismatch {
            expected: baseline.query.universe,
            actual: candidate.query.universe,
        });
    }
    if baseline.query.as_of != candidate.query.as_of
        || baseline.query.level != candidate.query.level
        || baseline.query.version.rule_version != candidate.query.version.rule_version
    {
        return Err(LevelViewError::IncomparableShadowReplay);
    }
    let baseline_fixed = (
        baseline.query.version.assembly_seam,
        baseline.query.version.rule_version,
        baseline.query.version.direction_provider,
        baseline.query.version.divergence_pair_provider,
    );
    let candidate_fixed = (
        candidate.query.version.assembly_seam,
        candidate.query.version.rule_version,
        candidate.query.version.direction_provider,
        candidate.query.version.divergence_pair_provider,
    );
    if baseline_fixed != candidate_fixed {
        return Err(LevelViewError::IncomparableShadowReplay);
    }
    let before = membership_keys(baseline);
    let after = membership_keys(candidate);
    let regressions_to_unassigned = before.difference(&after).cloned().collect();
    let completed_freeze_violations = baseline
        .view
        .moves
        .iter()
        .filter(|value| matches!(value.completion, CompletionStatus::Completed { .. }))
        .filter_map(|value| {
            let same = candidate.view.moves.iter().find(|other| {
                other.start_index == value.start_index
                    && other.end_index == value.end_index
                    && other.kind == value.kind
                    && other.direction == value.direction
                    && other.chain == value.chain
                    && matches!(other.completion, CompletionStatus::Completed { .. })
            });
            same.is_none().then_some(CompletedMoveId {
                level: baseline.query.level,
                start_index: value.start_index,
            })
        })
        .collect();
    Ok(ShadowReplayAssessment {
        regressions_to_unassigned,
        completed_freeze_violations,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairNotEligible {
    LeaveNotACompletedMove,
    RetestNotACompletedMove,
    SameCompletedMove,
    NonAdjacentCompletedMoves,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EligibleCompletedPair {
    pub leave: CompletedMoveId,
    pub retest: CompletedMoveId,
}

/// strict C2 pair：两个目标区间必须各自精确等于不同、相邻、Completed 的 Move。
pub fn completed_pair(
    view: &LevelAsOfView,
    leave: (usize, usize),
    retest: (usize, usize),
) -> Result<EligibleCompletedPair, PairNotEligible> {
    let find = |span: (usize, usize)| {
        view.view
            .moves
            .iter()
            .enumerate()
            .find(|(_, value)| value.start_index == span.0 && value.end_index == span.1)
    };
    let Some((leave_index, leave_move)) = find(leave) else {
        return Err(PairNotEligible::LeaveNotACompletedMove);
    };
    if !matches!(leave_move.completion, CompletionStatus::Completed { .. }) {
        return Err(PairNotEligible::LeaveNotACompletedMove);
    }
    let Some((retest_index, retest_move)) = find(retest) else {
        return Err(PairNotEligible::RetestNotACompletedMove);
    };
    if !matches!(retest_move.completion, CompletionStatus::Completed { .. }) {
        return Err(PairNotEligible::RetestNotACompletedMove);
    }
    if leave_index == retest_index {
        return Err(PairNotEligible::SameCompletedMove);
    }
    if leave_index + 1 != retest_index {
        return Err(PairNotEligible::NonAdjacentCompletedMoves);
    }
    Ok(EligibleCompletedPair {
        leave: CompletedMoveId {
            level: view.query.level,
            start_index: leave_move.start_index,
        },
        retest: CompletedMoveId {
            level: view.query.level,
            start_index: retest_move.start_index,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::types::{Center, Tick};
    use super::super::center::UnitRange;
    use super::super::move_view::{ChainOrigin, CompletionEvidence, MoveViewKind, PendingReason};
    use super::super::recursive_tower::ElementId;
    use super::*;

    fn center(start: usize, end: usize, zd: Tick, zg: Tick, dd: Tick, gg: Tick) -> Center {
        Center {
            start_index: start,
            end_index: end,
            zd,
            zg,
            dd,
            gg,
        }
    }

    fn lower(
        start: usize,
        end: usize,
        direction: Direction,
        lo: Tick,
        hi: Tick,
        ordinal: u64,
    ) -> LeveledMove {
        LeveledMove::from_unit(
            &UnitRange {
                start_index: start,
                end_index: end,
                direction,
                lo,
                hi,
            },
            ElementId { level: 0, ordinal },
        )
    }

    fn window(subs: &[LeveledMove], c: Center, ordinal: u64) -> LeveledMove {
        LeveledMove::compose(subs, c, 1, ElementId { level: 1, ordinal })
    }

    fn fixture() -> (Vec<LeveledMove>, Vec<LeveledMove>) {
        use Direction::{Down, Up};
        let lower = vec![
            lower(0, 9, Up, 90, 110, 0),
            lower(10, 19, Down, 95, 115, 1),
            lower(20, 29, Up, 98, 112, 2),
            lower(30, 39, Down, 96, 116, 3),
            lower(40, 49, Up, 130, 145, 4),
            lower(50, 59, Down, 132, 148, 5),
            lower(60, 69, Up, 135, 150, 6),
            lower(70, 79, Up, 151, 160, 7),
            lower(80, 89, Down, 120, 140, 8),
            lower(90, 99, Up, 125, 145, 9),
            lower(100, 109, Down, 118, 138, 10),
        ];
        let windows = vec![
            window(&lower[0..3], center(0, 29, 98, 110, 90, 115), 0),
            window(&lower[4..7], center(40, 69, 135, 145, 130, 150), 1),
            window(&lower[8..11], center(80, 109, 125, 138, 118, 145), 2),
        ];
        (windows, lower)
    }

    fn version(direction_provider: DirectionProviderVersion) -> ViewVersion {
        ViewVersion {
            assembly_seam: AssemblySeamVersion::C2V1,
            partition_policy: PartitionPolicyVersion::GREEDY_V1,
            rule_version: RuleVersion(1),
            direction_provider,
            divergence_pair_provider: None,
        }
    }

    fn query(as_of: usize, direction_provider: DirectionProviderVersion) -> LevelViewQuery {
        LevelViewQuery {
            level: 1,
            coordinate_window: CoordinateWindow { start: 0, end: 109 },
            as_of,
            universe: ObjectUniverse::MainExactThreeV1,
            version: version(direction_provider),
        }
    }

    fn run(windows: &[LeveledMove], lower: &[LeveledMove], as_of: usize) -> LevelAsOfView {
        assemble_level_view(
            query(as_of, DirectionProviderVersion::SequenceEnvelopeV1),
            ExactThreeMaterialSnapshot {
                universe: ObjectUniverse::MainExactThreeV1,
                observed_at: as_of,
                tower_windows: windows,
                lower_ledger: lower,
                ownership_blocks: None,
                divergence: None,
            },
        )
        .unwrap()
    }

    /// 冻结门测试只关心已完成对象的跨快照不变量；完成证据由生产组装器同型枚举显式注入，
    /// 避免把 A/C hook 的独立测试前提混进冻结门测试。
    fn completed_baseline() -> LevelAsOfView {
        let (windows, lower) = fixture();
        let mut result = run(&windows, &lower, 109);
        let first = result.view.moves.first_mut().unwrap();
        first.completion = CompletionStatus::Completed {
            as_of: 109,
            evidence: CompletionEvidence::SubsequentMove { at: 80 },
        };
        result
    }

    #[test]
    fn production_seam_recovers_orphan_and_keeps_missing_hook_pending() {
        let (windows, lower) = fixture();
        let result = run(&windows[..2], &lower[..8], 79);
        assert_eq!(result.view.moves[0].kind, MoveViewKind::Trend);
        assert!(matches!(
            result.view.moves[0].completion,
            CompletionStatus::Pending {
                reason: PendingReason::MissingDivergencePair,
                ..
            }
        ));
        assert_eq!(
            result.view.moves[0].chain[3].origin,
            ChainOrigin::OrphanFromLowerLedger
        );
    }

    #[test]
    fn actual_prefix_is_required_instead_of_history_end_filtering() {
        let (windows, lower) = fixture();
        let error = assemble_level_view(
            query(79, DirectionProviderVersion::SequenceEnvelopeV1),
            ExactThreeMaterialSnapshot {
                universe: ObjectUniverse::MainExactThreeV1,
                observed_at: 109,
                tower_windows: &windows,
                lower_ledger: &lower,
                ownership_blocks: None,
                divergence: None,
            },
        )
        .unwrap_err();
        assert_eq!(
            error,
            LevelViewError::SnapshotTimeMismatch {
                observed_at: 109,
                as_of: 79
            }
        );
    }

    #[test]
    fn object_universe_and_exact_three_are_fail_closed() {
        let (windows, lower) = fixture();
        let mut foreign_query = query(109, DirectionProviderVersion::FirstLeafV1);
        foreign_query.universe = ObjectUniverse::V54VariableWindowOwnership;
        assert!(matches!(
            assemble_level_view(
                foreign_query,
                ExactThreeMaterialSnapshot {
                    universe: ObjectUniverse::MainExactThreeV1,
                    observed_at: 109,
                    tower_windows: &windows,
                    lower_ledger: &lower,
                    ownership_blocks: None,
                    divergence: None,
                }
            ),
            Err(LevelViewError::ObjectUniverseMismatch { .. })
        ));

        let variable = window(&lower[0..4], center(0, 39, 98, 110, 90, 116), 7);
        assert!(matches!(
            assemble_level_view(
                query(109, DirectionProviderVersion::FirstLeafV1),
                ExactThreeMaterialSnapshot {
                    universe: ObjectUniverse::MainExactThreeV1,
                    observed_at: 109,
                    tower_windows: &[variable],
                    lower_ledger: &lower,
                    ownership_blocks: None,
                    divergence: None,
                }
            ),
            Err(LevelViewError::NonExactThreeWindow { sub_count: 4, .. })
        ));
    }

    #[test]
    fn completed_freeze_rejects_chain_mutation_and_preserves_first_judge_at() {
        let baseline = completed_baseline();
        let mut guard = CompletedFreezeGuard::new(baseline.query.version, baseline.query.universe);
        guard.observe(&baseline).unwrap();
        let frozen_before = guard.frozen().clone();
        let events_before = guard.events().to_vec();
        assert!(!frozen_before.is_empty());
        assert_eq!(events_before.len(), frozen_before.len());
        assert!(frozen_before
            .values()
            .all(|value| value.entry_bar == Some(110)));

        let mut later = baseline.clone();
        later.query.as_of = 120;
        later.view.as_of = 120;
        for value in &mut later.view.moves {
            if let CompletionStatus::Completed { as_of, .. } = &mut value.completion {
                *as_of = 120;
            }
        }
        guard.observe(&later).unwrap();
        assert_eq!(guard.frozen(), &frozen_before);
        assert_eq!(guard.events(), events_before.as_slice());

        let mut changed = later;
        let target = changed
            .view
            .moves
            .iter_mut()
            .find(|value| matches!(value.completion, CompletionStatus::Completed { .. }))
            .unwrap();
        target.chain.pop();
        let error = guard.observe(&changed).unwrap_err();
        assert!(matches!(
            error,
            CompletedFreezeViolation::ChainOrMembershipChanged(_)
        ));
        assert_eq!(guard.frozen(), &frozen_before);
        assert_eq!(guard.events(), events_before.as_slice());
    }

    #[test]
    fn shadow_replay_rejects_released_membership_and_completed_change() {
        let baseline = completed_baseline();
        let mut candidate = baseline.clone();
        candidate.query.version.partition_policy = PartitionPolicyVersion(2);
        candidate.view.moves.remove(0);
        let assessment = assess_shadow_replay(&baseline, &candidate).unwrap();
        assert!(!assessment.no_rollback());
        assert!(!assessment.regressions_to_unassigned.is_empty());
        assert!(!assessment.completed_freeze_violations.is_empty());
    }

    #[test]
    fn strict_pair_requires_distinct_adjacent_completed_moves() {
        let result = completed_baseline();
        let completed: Vec<_> = result
            .view
            .moves
            .iter()
            .filter(|value| matches!(value.completion, CompletionStatus::Completed { .. }))
            .collect();
        assert!(!completed.is_empty());
        let first = completed[0];
        assert!(matches!(
            completed_pair(&result, (first.start_index, first.end_index), (80, 109)),
            Err(PairNotEligible::RetestNotACompletedMove)
        ));
    }

    #[test]
    fn four_direction_providers_are_deterministic_and_versioned() {
        let (windows, lower) = fixture();
        for provider in [
            DirectionProviderVersion::OwnershipFallbackV1,
            DirectionProviderVersion::FirstLeafV1,
            DirectionProviderVersion::FirstLastEnvelopeV1,
            DirectionProviderVersion::SequenceEnvelopeV1,
        ] {
            let make = || {
                assemble_level_view(
                    query(109, provider),
                    ExactThreeMaterialSnapshot {
                        universe: ObjectUniverse::MainExactThreeV1,
                        observed_at: 109,
                        tower_windows: &windows,
                        lower_ledger: &lower,
                        ownership_blocks: None,
                        divergence: None,
                    },
                )
                .unwrap()
            };
            assert_eq!(make(), make());
        }
    }
}
