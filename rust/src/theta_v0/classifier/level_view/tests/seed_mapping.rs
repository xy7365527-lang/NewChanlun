use super::super::super::retrace_ledger::StrictCompletedPair;
use super::*;

/// 四类 fail-closed 码 + 唯一/Completed/相邻证明的测试（#640 provider 侧重落地）。
///
/// 直接构造 [`ExactThreeProjection`] 与 [`LevelAsOfView`]（两者均为公开字段纯数据），不经过
/// `assemble_level_view`——本模块只证「seed → 唯一 CompletedMove + 严格相邻」这条证明本身，
/// 与投影/确认管线解耦。

fn id(ordinal: u64) -> ElementId {
    ElementId { level: 1, ordinal }
}

fn seed(source_id: ElementId) -> ExactThreeSeed {
    ExactThreeSeed {
        source_id,
        source_sub_count: 4,
        start_index: 0,
        end_index: 9,
        center: Center {
            zd: 0,
            zg: 10,
            dd: 0,
            gg: 10,
            start_index: 0,
            end_index: 9,
        },
        core_provenance: SeedCoreProvenance::SelfConsistent,
    }
}

fn projection(source_ids: &[ElementId]) -> ExactThreeProjection {
    ExactThreeProjection {
        version: ProviderVersion::EXTENDED_TO_EXACT_THREE_V3,
        seeds: source_ids
            .iter()
            .map(|&source_id| seed(source_id))
            .collect(),
    }
}

fn completed_move(center_indices: Vec<usize>) -> AssembledMove {
    AssembledMove {
        start_index: 0,
        end_index: 9,
        direction: Some(Direction::Up),
        kind: MoveKind::Trend,
        completion: CompletionStatus::Completed {
            as_of: 0,
            evidence: CompletionEvidence::SubsequentMove,
        },
        center_indices,
    }
}

fn pending_move(center_indices: Vec<usize>) -> AssembledMove {
    AssembledMove {
        start_index: 0,
        end_index: 9,
        direction: Some(Direction::Up),
        kind: MoveKind::Trend,
        completion: CompletionStatus::Pending {
            as_of: 0,
            reason: PendingReason::AwaitingSubsequentMove,
        },
        center_indices,
    }
}

fn view(moves: Vec<AssembledMove>) -> LevelAsOfView {
    let query = LevelViewQuery {
        level: 1,
        coordinate_window: CoordinateWindow { start: 0, end: 9 },
        as_of: 0,
        version: C2VersionTuple::auto_pairing(),
    };
    let cache_key = C2CacheKey::from_query(&query).unwrap();
    LevelAsOfView {
        query,
        cache_key,
        moves,
        pairs: Vec::new(),
        pair_confirmations: Vec::new(),
    }
}

/// 码一 `MissingSource`：seed 不在投影里。
#[test]
fn missing_source_fails_closed() {
    let projection = projection(&[id(0), id(1)]);
    let view = view(vec![completed_move(vec![0]), completed_move(vec![1])]);
    assert_eq!(
        unique_completed_move(&projection, &view, id(99)),
        Err(StrictPairError::MissingSource(id(99)))
    );
    assert_eq!(
        strict_completed_pair(&projection, &view, id(99), id(0)),
        Err(StrictPairError::MissingSource(id(99)))
    );
}

/// 码二 `NoCompletedMove`：seed 存在但唯一覆盖它的 move 是 Pending（0 根 Completed）。
#[test]
fn no_completed_move_fails_closed() {
    let projection = projection(&[id(0)]);
    let view = view(vec![pending_move(vec![0])]);
    assert_eq!(
        unique_completed_move(&projection, &view, id(0)),
        Err(StrictPairError::NoCompletedMove(id(0)))
    );
}

/// 码三 `AmbiguousCompletedMove`：seed 同时被多根 CompletedMove 覆盖，归属不唯一。
#[test]
fn ambiguous_completed_move_fails_closed() {
    let projection = projection(&[id(0)]);
    let view = view(vec![completed_move(vec![0]), completed_move(vec![0])]);
    assert_eq!(
        unique_completed_move(&projection, &view, id(0)),
        Err(StrictPairError::AmbiguousCompletedMove {
            source_id: id(0),
            move_indices: vec![0, 1],
        })
    );
}

/// 码四 `NotAdjacent`：leave/retest 非「不同且严格相邻」——同根与隔根都落同一码
/// （与账本适配器不紧邻桶两码合一对齐，见 #624 对拍报告 B-4）。
#[test]
fn not_adjacent_fails_closed() {
    let projection = projection(&[id(0), id(1), id(2)]);
    let view = view(vec![
        completed_move(vec![0]),
        completed_move(vec![1]),
        completed_move(vec![2]),
    ]);
    // 隔根：leave=0、retest=2。
    assert_eq!(
        strict_completed_pair(&projection, &view, id(0), id(2)),
        Err(StrictPairError::NotAdjacent {
            leave_move_index: 0,
            retest_move_index: 2,
        })
    );
    // 同根（旧 SameMove 面）：leave=retest=1。
    assert_eq!(
        strict_completed_pair(&projection, &view, id(1), id(1)),
        Err(StrictPairError::NotAdjacent {
            leave_move_index: 1,
            retest_move_index: 1,
        })
    );
}

/// 正向：两个 seed 各自唯一归属一根 CompletedMove 且严格相邻 ⟹ 产出账本入口要的 pair。
#[test]
fn adjacent_completed_pair_maps_to_strict_pair() {
    let projection = projection(&[id(0), id(1)]);
    let view = view(vec![completed_move(vec![0]), completed_move(vec![1])]);
    assert_eq!(
        strict_completed_pair(&projection, &view, id(0), id(1)),
        Ok(StrictCompletedPair {
            leave_move_index: 0,
            retest_move_index: 1,
        })
    );
}
