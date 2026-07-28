//! D7 `firstRetrace` 只读复核原语。
//!
//! 本模块不生成交易信号，也不改变 C2 seam 的默认关闭状态。它只完成两件事：
//! 1. 把 D1 seed 身份映射到“不同、相邻、已完成”的 C2 Move pair；
//! 2. 在严格 pair 已成立后复演 `RETEST_REENTERS / SUPERSEDE / RESTART` 身份纪律。

use super::level_view::{CompletionStatus, CoordinateWindow, ExactThreeProjection, LevelAsOfView};
use super::recursive_tower::ElementId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrictCompletedPair {
    pub leave_move_index: usize,
    pub retest_move_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrictPairError {
    MissingSource(ElementId),
    NoCompletedMove(ElementId),
    AmbiguousCompletedMove {
        source_id: ElementId,
        move_indices: Vec<usize>,
    },
    SameMove(usize),
    NotAdjacent {
        leave_move_index: usize,
        retest_move_index: usize,
    },
}

/// D1 seed → C2 CompletedMove 的严格映射。
///
/// seed 必须在 view 中唯一属于一个 CompletedMove；leave/retest 必须是不同且正向相邻的
/// CompletedMove。Pending、缺 seed、边界处同时属于两个 CompletedMove 均 fail closed。
pub fn strict_completed_pair(
    projection: &ExactThreeProjection,
    view: &LevelAsOfView,
    leave_source: ElementId,
    retest_source: ElementId,
) -> Result<StrictCompletedPair, StrictPairError> {
    let leave = unique_completed_move(projection, view, leave_source)?;
    let retest = unique_completed_move(projection, view, retest_source)?;
    if leave == retest {
        return Err(StrictPairError::SameMove(leave));
    }
    if retest != leave + 1 {
        return Err(StrictPairError::NotAdjacent {
            leave_move_index: leave,
            retest_move_index: retest,
        });
    }
    Ok(StrictCompletedPair {
        leave_move_index: leave,
        retest_move_index: retest,
    })
}

fn unique_completed_move(
    projection: &ExactThreeProjection,
    view: &LevelAsOfView,
    source_id: ElementId,
) -> Result<usize, StrictPairError> {
    let seed_index = projection
        .seeds
        .iter()
        .position(|seed| seed.source_id == source_id)
        .ok_or(StrictPairError::MissingSource(source_id))?;
    let move_indices: Vec<_> = view
        .moves
        .iter()
        .enumerate()
        .filter(|(_, value)| {
            value.center_indices.contains(&seed_index)
                && matches!(value.completion, CompletionStatus::Completed { .. })
        })
        .map(|(index, _)| index)
        .collect();
    match move_indices.as_slice() {
        [] => Err(StrictPairError::NoCompletedMove(source_id)),
        [index] => Ok(*index),
        _ => Err(StrictPairError::AmbiguousCompletedMove {
            source_id,
            move_indices,
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetraceIdentity {
    pub center: CoordinateWindow,
    pub departure_move_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetraceOutcome {
    RetestReenters,
    Success,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetraceObservation {
    pub pair: StrictCompletedPair,
    pub outcome: RetraceOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetraceLifecycleEvent {
    RetestReenters {
        identity: RetraceIdentity,
        retest_move_index: usize,
    },
    Supersede {
        identity: RetraceIdentity,
    },
    Restart {
        identity: RetraceIdentity,
    },
    Success {
        identity: RetraceIdentity,
        retest_move_index: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetraceLifecycleError {
    PairDepartureMismatch {
        identity: RetraceIdentity,
        pair: StrictCompletedPair,
    },
    FirstRetraceConsumed(RetraceIdentity),
}

/// 同一 `(center, departure CompletedMove)` 身份只消费第一次严格回试。
///
/// `RETEST_REENTERS` 会消费旧身份的 firstRetrace 资格；后续成功必须先由不同 departure
/// 触发 `SUPERSEDE(old) → RESTART(new)`。同 departure 静默晚成功明确拒绝。
pub fn replay_first_retrace(
    initial: RetraceIdentity,
    observations: &[RetraceObservation],
) -> Result<Vec<RetraceLifecycleEvent>, RetraceLifecycleError> {
    let mut active = initial;
    let mut consumed = false;
    let mut restart_allowed = false;
    let mut events = Vec::new();
    for observation in observations {
        if observation.pair.leave_move_index != active.departure_move_index {
            if !restart_allowed {
                return Err(RetraceLifecycleError::PairDepartureMismatch {
                    identity: active,
                    pair: observation.pair,
                });
            }
            events.push(RetraceLifecycleEvent::Supersede { identity: active });
            active = RetraceIdentity {
                center: active.center,
                departure_move_index: observation.pair.leave_move_index,
            };
            events.push(RetraceLifecycleEvent::Restart { identity: active });
            consumed = false;
        }
        if consumed {
            return Err(RetraceLifecycleError::FirstRetraceConsumed(active));
        }
        consumed = true;
        match observation.outcome {
            RetraceOutcome::RetestReenters => {
                restart_allowed = true;
                events.push(RetraceLifecycleEvent::RetestReenters {
                    identity: active,
                    retest_move_index: observation.pair.retest_move_index,
                });
            }
            RetraceOutcome::Success => {
                restart_allowed = false;
                events.push(RetraceLifecycleEvent::Success {
                    identity: active,
                    retest_move_index: observation.pair.retest_move_index,
                });
            }
        }
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::super::super::types::{Center, Direction, MoveKind};
    use super::super::center::UnitRange;
    use super::super::decompose::{MoveBlock, MoveStatus};
    use super::super::level_view::{
        project_extended_windows, AssembledMove, C2CacheKey, C2VersionTuple, CompletionEvidence,
        DivergencePairId, ExactThreeSeed, LevelViewQuery, ProviderVersion, SeedCoreProvenance,
    };
    use super::super::recursive_tower::LeveledMove;
    use super::*;

    fn id(ordinal: u64) -> ElementId {
        ElementId { level: 3, ordinal }
    }

    fn center(start_index: usize, end_index: usize) -> Center {
        Center {
            zd: 10,
            zg: 20,
            dd: 5,
            gg: 25,
            start_index,
            end_index,
        }
    }

    fn seed(ordinal: u64, sub_count: usize, start: usize, end: usize) -> ExactThreeSeed {
        ExactThreeSeed {
            source_id: id(ordinal),
            source_sub_count: sub_count,
            start_index: start,
            end_index: end,
            center: center(start, end),
            core_provenance: SeedCoreProvenance::SelfConsistent,
        }
    }

    fn query(as_of: usize) -> LevelViewQuery {
        LevelViewQuery {
            level: 3,
            coordinate_window: CoordinateWindow {
                start: 2_884_261,
                end: 2_943_378,
            },
            as_of,
            version: C2VersionTuple::auto_pairing(),
        }
    }

    fn assembled(
        start_index: usize,
        end_index: usize,
        kind: MoveKind,
        direction: Option<Direction>,
        completion: CompletionStatus,
        center_indices: Vec<usize>,
    ) -> AssembledMove {
        AssembledMove {
            start_index,
            end_index,
            direction,
            kind,
            completion,
            center_indices,
        }
    }

    fn view(as_of: usize, moves: Vec<AssembledMove>) -> LevelAsOfView {
        let query = query(as_of);
        LevelAsOfView {
            query,
            cache_key: C2CacheKey::from_query(&query).unwrap(),
            moves,
            pairs: Vec::new(),
            pair_confirmations: Vec::new(),
        }
    }

    fn completed(as_of: usize) -> CompletionStatus {
        CompletionStatus::Completed {
            as_of,
            evidence: CompletionEvidence::SubsequentMove,
        }
    }

    fn lower(ordinal: u64, start_index: usize, end_index: usize) -> LeveledMove {
        LeveledMove::from_unit(
            &UnitRange {
                start_index,
                end_index,
                direction: if ordinal % 2 == 0 {
                    Direction::Up
                } else {
                    Direction::Down
                },
                lo: 10,
                hi: 20,
            },
            ElementId { level: 2, ordinal },
        )
    }

    fn pending(as_of: usize) -> CompletionStatus {
        CompletionStatus::Pending {
            as_of,
            reason: super::super::level_view::PendingReason::TerminalLegNotDivergent,
        }
    }

    #[test]
    fn lv_case2_auto_pairing_tuple_keeps_all_three_versions_pinned() {
        let version = C2VersionTuple::auto_pairing();
        assert_eq!(
            version.direction_provider_version,
            Some(ProviderVersion::CENTRAL_GGDD_V1)
        );
        assert_eq!(
            version.divergence_pair_provider_version,
            Some(ProviderVersion::MOVE_BLOCK_AC_V1)
        );
        assert_eq!(
            version.projection_provider_version,
            Some(ProviderVersion::EXTENDED_TO_EXACT_THREE_V3)
        );
        assert!(version.validate().is_ok());
    }

    #[test]
    fn lv_case2_projection_uses_exact_three_not_mutable_window_tail_anchors() {
        let subs_273 = vec![
            lower(0, 2_884_261, 2_885_616),
            lower(1, 2_885_681, 2_886_792),
            lower(2, 2_886_939, 2_888_441),
            lower(3, 2_888_500, 2_896_661),
        ];
        let subs_276 = vec![
            lower(4, 2_927_357, 2_928_484),
            lower(5, 2_928_911, 2_930_933),
            lower(6, 2_931_076, 2_934_387),
            lower(7, 2_934_500, 2_942_773),
        ];
        let windows = vec![
            LeveledMove::compose(&subs_273, center(2_884_261, 2_896_661), 3, id(273)),
            LeveledMove::compose(&subs_276, center(2_927_357, 2_942_773), 3, id(276)),
        ];
        let projection = project_extended_windows(&windows).unwrap();
        assert_eq!(
            projection
                .seeds
                .iter()
                .map(|seed| (
                    seed.source_id,
                    seed.source_sub_count,
                    seed.start_index,
                    seed.end_index
                ))
                .collect::<Vec<_>>(),
            vec![
                (id(273), 4, 2_884_261, 2_888_441),
                (id(276), 4, 2_927_357, 2_934_387),
            ]
        );
    }

    #[test]
    fn lv_case2_first_pair_has_no_completed_pair_at_first_judge_time() {
        let as_of = 2_906_047;
        let projection = ExactThreeProjection {
            version: ProviderVersion::EXTENDED_TO_EXACT_THREE_V1,
            seeds: vec![
                seed(273, 6, 2_884_261, 2_888_441),
                seed(274, 5, 2_896_743, 2_901_685),
            ],
        };
        let view = view(
            as_of,
            vec![assembled(
                2_884_261,
                2_901_685,
                MoveKind::Consolidation,
                None,
                CompletionStatus::Pending {
                    as_of,
                    reason: super::super::level_view::PendingReason::AwaitingSubsequentMove,
                },
                vec![0, 1],
            )],
        );
        assert_eq!(
            strict_completed_pair(&projection, &view, id(273), id(274)),
            Err(StrictPairError::NoCompletedMove(id(273)))
        );
    }

    #[test]
    fn lv_case2_first_pair_collapses_into_same_completed_c2_move_later() {
        let as_of = 2_943_378;
        let projection = ExactThreeProjection {
            version: ProviderVersion::EXTENDED_TO_EXACT_THREE_V1,
            seeds: vec![
                seed(273, 6, 2_884_261, 2_888_441),
                seed(274, 6, 2_896_743, 2_901_685),
                seed(275, 5, 2_906_049, 2_909_293),
                seed(276, 6, 2_927_357, 2_934_387),
            ],
        };
        let view = view(
            as_of,
            vec![
                assembled(
                    2_884_261,
                    2_901_685,
                    MoveKind::Consolidation,
                    None,
                    completed(as_of),
                    vec![0, 1],
                ),
                assembled(
                    2_896_743,
                    2_909_293,
                    MoveKind::Trend,
                    Some(Direction::Down),
                    pending(as_of),
                    vec![1, 2],
                ),
                assembled(
                    2_906_049,
                    2_934_387,
                    MoveKind::Trend,
                    Some(Direction::Up),
                    CompletionStatus::Completed {
                        as_of,
                        evidence: CompletionEvidence::TerminalDivergence {
                            pair_id: DivergencePairId {
                                level: 3,
                                block_start_center: 9,
                                block_end_center: 10,
                                direction: Direction::Up,
                            },
                        },
                    },
                    vec![2, 3],
                ),
            ],
        );
        assert_eq!(
            strict_completed_pair(&projection, &view, id(273), id(274)),
            Err(StrictPairError::SameMove(0))
        );
    }

    #[test]
    fn lv_case2_late_pair_has_completed_leave_but_missing_retest_seed() {
        let as_of = 2_943_378;
        let projection = ExactThreeProjection {
            version: ProviderVersion::EXTENDED_TO_EXACT_THREE_V1,
            seeds: vec![
                seed(275, 5, 2_906_049, 2_909_293),
                seed(276, 6, 2_927_357, 2_934_387),
            ],
        };
        let view = view(
            as_of,
            vec![assembled(
                2_906_049,
                2_934_387,
                MoveKind::Trend,
                Some(Direction::Up),
                completed(as_of),
                vec![0, 1],
            )],
        );
        assert_eq!(
            strict_completed_pair(&projection, &view, id(276), id(277)),
            Err(StrictPairError::MissingSource(id(277)))
        );
    }

    #[test]
    fn lv_case2_same_identity_late_success_is_rejected_after_reentry() {
        let identity = RetraceIdentity {
            center: CoordinateWindow { start: 10, end: 20 },
            departure_move_index: 3,
        };
        let observations = [
            RetraceObservation {
                pair: StrictCompletedPair {
                    leave_move_index: 3,
                    retest_move_index: 4,
                },
                outcome: RetraceOutcome::RetestReenters,
            },
            RetraceObservation {
                pair: StrictCompletedPair {
                    leave_move_index: 3,
                    retest_move_index: 4,
                },
                outcome: RetraceOutcome::Success,
            },
        ];
        assert_eq!(
            replay_first_retrace(identity, &observations),
            Err(RetraceLifecycleError::FirstRetraceConsumed(identity))
        );
    }

    #[test]
    fn lv_case2_new_departure_supersedes_then_restarts_with_new_identity() {
        let old = RetraceIdentity {
            center: CoordinateWindow { start: 10, end: 20 },
            departure_move_index: 3,
        };
        let new = RetraceIdentity {
            center: old.center,
            departure_move_index: 5,
        };
        let events = replay_first_retrace(
            old,
            &[
                RetraceObservation {
                    pair: StrictCompletedPair {
                        leave_move_index: 3,
                        retest_move_index: 4,
                    },
                    outcome: RetraceOutcome::RetestReenters,
                },
                RetraceObservation {
                    pair: StrictCompletedPair {
                        leave_move_index: 5,
                        retest_move_index: 6,
                    },
                    outcome: RetraceOutcome::Success,
                },
            ],
        )
        .unwrap();
        assert_eq!(
            events,
            vec![
                RetraceLifecycleEvent::RetestReenters {
                    identity: old,
                    retest_move_index: 4,
                },
                RetraceLifecycleEvent::Supersede { identity: old },
                RetraceLifecycleEvent::Restart { identity: new },
                RetraceLifecycleEvent::Success {
                    identity: new,
                    retest_move_index: 6,
                },
            ]
        );
    }

    #[test]
    fn lv_case2_without_strict_pairs_emits_no_lifecycle_events() {
        let identity = RetraceIdentity {
            center: CoordinateWindow {
                start: 2_846_372,
                end: 2_880_443,
            },
            departure_move_index: 0,
        };
        assert_eq!(replay_first_retrace(identity, &[]), Ok(Vec::new()));
    }

    #[test]
    fn lv_case2_fixture_move_blocks_keep_d3_direction_tristate() {
        let blocks = [
            MoveBlock {
                start_center: 0,
                end_center: 1,
                kind: MoveKind::Consolidation,
                dir: None,
                status: MoveStatus::Completed,
            },
            MoveBlock {
                start_center: 1,
                end_center: 2,
                kind: MoveKind::Trend,
                dir: Some(Direction::Down),
                status: MoveStatus::Completed,
            },
        ];
        assert_eq!(blocks[0].dir, None);
        assert_eq!(blocks[1].dir, Some(Direction::Down));
    }
}
