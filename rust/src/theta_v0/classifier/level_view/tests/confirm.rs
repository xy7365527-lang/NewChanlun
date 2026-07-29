use super::*;
use super::fixtures::{extended_windows, trend_block};

/// #69 5a / T0：先锁住冷核的首证钟与未决投影；resident 化不得改写现行
/// `Option<usize>` 可观察结果。
#[test]
fn confirm_state_cold_projection_preserves_first_proof_and_scanning() {
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows_carried_only(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    let segments: Vec<_> = legs.iter().map(leg_as_segment).collect();
    let last = &projection.seeds[2].center;
    let mut hist = vec![0.0; 140];
    hist[80..110].fill(2.0);
    hist[120..140].fill(0.1);
    let mut dif = vec![0.0; 140];
    dif[80..=100].fill(-5.0);
    dif[101..140].fill(1.0);
    let close_src: Vec<_> = (0..140).collect();

    let confirmed = trend_confirm_state(
        &segments,
        last,
        Direction::Up,
        Side::Short,
        (80, 109),
        120,
        139,
        &hist,
        &dif,
        &close_src,
    );
    assert_eq!(confirmed, ConfirmState::Confirmed(139));
    assert_eq!(
        confirmed.as_option(),
        trend_confirm_time(
            &segments,
            last,
            Direction::Up,
            Side::Short,
            (80, 109),
            120,
            139,
            &hist,
            &dif,
            &close_src,
        )
    );

    let scanning = trend_confirm_state(
        &segments,
        last,
        Direction::Up,
        Side::Short,
        (80, 109),
        120,
        129,
        &hist,
        &dif,
        &close_src,
    );
    assert_eq!(scanning, ConfirmState::Scanning);
    assert_eq!(scanning.as_option(), None);
}

/// #69 5a / T1：只有已进入 T5 判定且 `force_ok` 单调转假才是终假；
/// 坐标缺失等旧 `None` 必须仍是未决。
#[test]
fn confirm_state_distinguishes_terminal_false_from_unresolved_none() {
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows_carried_only(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    let segments: Vec<_> = legs.iter().map(leg_as_segment).collect();
    let last = &projection.seeds[2].center;
    let mut hist = vec![0.0; 140];
    hist[80..110].fill(2.0);
    hist[120..140].fill(5.0);
    let mut dif = vec![0.0; 140];
    dif[80..=100].fill(-5.0);
    dif[101..140].fill(1.0);
    let close_src: Vec<_> = (0..140).collect();

    let terminal = trend_confirm_state(
        &segments,
        last,
        Direction::Up,
        Side::Short,
        (80, 109),
        120,
        139,
        &hist,
        &dif,
        &close_src,
    );
    assert_eq!(terminal, ConfirmState::TerminalFalse);
    assert_eq!(terminal.as_option(), None);

    let missing_coordinates = trend_confirm_state(
        &segments,
        last,
        Direction::Up,
        Side::Short,
        (80, 109),
        120,
        139,
        &hist,
        &dif,
        &[],
    );
    assert_eq!(missing_coordinates, ConfirmState::Scanning);
}

/// #69 5a / T2：同一 pair 按 as_of 增长时，resident cursor 每一步必须与空 cursor
/// 冷算同果；已封 lower-leg 下标只前进、不重加。
#[test]
fn confirm_cursor_incremental_matches_cold_at_every_boundary() {
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows_carried_only(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    let segments: Vec<_> = legs.iter().map(leg_as_segment).collect();
    let last = &projection.seeds[2].center;
    let mut hist = vec![0.0; 140];
    hist[80..110].fill(2.0);
    hist[120..140].fill(0.1);
    let mut dif = vec![0.0; 140];
    dif[80..=100].fill(-5.0);
    dif[101..140].fill(1.0);
    let close_src: Vec<_> = (0..140).collect();
    let mut cursor = ConfirmCursor::default();
    let mut previous_k0 = 0;

    for as_of in [119, 129, 139] {
        let confirmed_len = segments.partition_point(|segment| segment.end_index <= as_of);
        let resident = trend_confirm_state_core(
            &segments,
            last,
            Direction::Up,
            Side::Short,
            (80, 109),
            120,
            as_of,
            &hist,
            &dif,
            &close_src,
            Some((&mut cursor, confirmed_len)),
        );
        let cold = trend_confirm_state(
            &segments,
            last,
            Direction::Up,
            Side::Short,
            (80, 109),
            120,
            as_of,
            &hist,
            &dif,
            &close_src,
        );
        assert_eq!(resident, cold, "as_of={as_of} 热路必须等于空 cursor 冷路");
        assert!(cursor.k0 >= previous_k0, "已封 lower-leg 游标不得倒退");
        assert!(cursor.k0 <= confirmed_len, "cursor 只能落在已封水线内");
        previous_k0 = cursor.k0;
    }
    assert_eq!(cursor.state, ConfirmState::Confirmed(139));
    let sealed_k0 = cursor.k0;
    let repeated = trend_confirm_state_core(
        &segments,
        last,
        Direction::Up,
        Side::Short,
        (80, 109),
        120,
        139,
        &hist,
        &dif,
        &close_src,
        Some((&mut cursor, segments.len())),
    );
    assert_eq!(repeated, ConfirmState::Confirmed(139));
    assert_eq!(cursor.k0, sealed_k0, "终态重复查询不得重扫或推进 cursor");
}

/// #69 5a / T3：store 只持久化已封前缀；证书回退必须清 cursor 冷重算，
/// 结构代次变化必须 key miss 且同 run 旧 key 被剪枝。
#[test]
fn confirm_store_resets_on_watermark_rollback_and_structure_change() {
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows_carried_only(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    let blocks = [trend_block(Some(Direction::Up))];
    let mut hist = vec![0.0; 140];
    hist[80..110].fill(2.0);
    hist[120..140].fill(0.1);
    let mut dif = vec![0.0; 140];
    dif[80..=100].fill(-5.0);
    dif[101..140].fill(1.0);
    let close_src: Vec<_> = (0..140).collect();
    let resident_query = LevelViewQuery {
        level: 1,
        coordinate_window: CoordinateWindow { start: 0, end: 139 },
        as_of: 139,
        version: C2VersionTuple::auto_pairing(),
    };
    let material = || LevelViewMaterial {
        projection: ProjectionMaterial::ExactThree(&projection),
        move_blocks: &blocks,
        lower_legs: &legs,
        hist: &hist,
        dif: &dif,
        close_src: &close_src,
    };
    let mut store = ConfirmCursorStore::default();

    let sealed = assemble_level_view_resident(
        C2LevelViewConfig { enabled: true },
        resident_query,
        material(),
        Some(ConfirmResidence {
            store: &mut store,
            stable_lower_len: legs.len(),
            structure_generation: 7,
        }),
    )
    .unwrap();
    assert!(matches!(
        sealed.moves[0].completion,
        CompletionStatus::Completed { .. }
    ));
    assert_eq!(store.cursors.len(), 1);
    assert_eq!(
        store.cursors.values().next().unwrap().state,
        ConfirmState::Confirmed(139)
    );

    let rolled = assemble_level_view_resident(
        C2LevelViewConfig { enabled: true },
        resident_query,
        material(),
        Some(ConfirmResidence {
            store: &mut store,
            stable_lower_len: 12,
            structure_generation: 7,
        }),
    )
    .unwrap();
    assert_eq!(
        rolled,
        assemble_level_view(
            C2LevelViewConfig { enabled: true },
            resident_query,
            material()
        )
        .unwrap()
    );
    let cursor = store.cursors.values().next().unwrap();
    assert_eq!(cursor.k0, 12, "水线回退后只可重建到新已封边界");
    assert_eq!(cursor.state, ConfirmState::Scanning, "可变尾结果不得回写");

    let regenerated = assemble_level_view_resident(
        C2LevelViewConfig { enabled: true },
        resident_query,
        material(),
        Some(ConfirmResidence {
            store: &mut store,
            stable_lower_len: legs.len(),
            structure_generation: 8,
        }),
    )
    .unwrap();
    assert_eq!(regenerated, sealed);
    assert_eq!(store.cursors.len(), 1, "同 run 的旧结构 key 必须被剪枝");
    assert!(store
        .cursors
        .keys()
        .all(|key| key.structure_generation == 8));
}

/// #69 5a / T4：assemble 与 provider 必须消费 view 内同一份 pair 状态；
/// provider 不得二次进入确认核，且 sidecar 查找必须校验 `DivergencePairId`。
#[test]
fn assemble_and_provider_share_one_pair_confirmation() {
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows_carried_only(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    let blocks = [
        trend_block(Some(Direction::Up)),
        MoveBlock {
            start_center: 1,
            end_center: 2,
            kind: MoveKind::Consolidation,
            dir: None,
            status: MoveStatus::Completed,
        },
    ];
    let mut hist = vec![0.0; 140];
    hist[80..110].fill(2.0);
    hist[120..140].fill(0.1);
    let mut dif = vec![0.0; 140];
    dif[80..=100].fill(-5.0);
    dif[101..140].fill(1.0);
    let close_src: Vec<_> = (0..140).collect();
    let query = LevelViewQuery {
        level: 1,
        coordinate_window: CoordinateWindow { start: 0, end: 139 },
        as_of: 139,
        version: C2VersionTuple::auto_pairing(),
    };
    reset_confirm_core_calls();
    let view = assemble_level_view(
        C2LevelViewConfig { enabled: true },
        query,
        LevelViewMaterial {
            projection: ProjectionMaterial::ExactThree(&projection),
            move_blocks: &blocks,
            lower_legs: &legs,
            hist: &hist,
            dif: &dif,
            close_src: &close_src,
        },
    )
    .unwrap();
    assert_eq!(view.pair_confirmations.len(), 1);
    assert_eq!(
        view.pair_confirmations[0],
        PairConfirmState {
            pair_id: view.pairs[0].id,
            state: ConfirmState::Confirmed(139),
        }
    );
    assert_eq!(confirm_core_calls(), 1, "assemble 每 pair 只进核一次");

    let events = provide_nest_candidate_events(
        1,
        &projection,
        &blocks,
        &legs,
        &view,
        &hist,
        &dif,
        &close_src,
    );
    assert!(events
        .iter()
        .any(|event| event.kind == NestDivergenceKind::Trend && event.divergence_confirmed));
    assert_eq!(confirm_core_calls(), 1, "provider 必须只读 view sidecar");

    let mut mismatched = view.clone();
    mismatched.pair_confirmations[0].pair_id.level += 1;
    let mismatched_events = provide_nest_candidate_events(
        1,
        &projection,
        &blocks,
        &legs,
        &mismatched,
        &hist,
        &dif,
        &close_src,
    );
    assert!(mismatched_events
        .iter()
        .filter(|event| event.kind == NestDivergenceKind::Trend)
        .all(|event| !event.divergence_confirmed));
    assert_eq!(
        confirm_core_calls(),
        1,
        "身份错配必须 fail-closed，禁止回退重算"
    );
}

/// #69 5a / T5：即使 resident store 被污染，显式 `None` 仍必须走真冷核，
/// 既不读取也不改写该 store。
#[test]
fn cold_none_oracle_is_isolated_from_poisoned_resident_store() {
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows_carried_only(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    let blocks = [trend_block(Some(Direction::Up))];
    let mut hist = vec![0.0; 140];
    hist[80..110].fill(2.0);
    hist[120..140].fill(0.1);
    let mut dif = vec![0.0; 140];
    dif[80..=100].fill(-5.0);
    dif[101..140].fill(1.0);
    let close_src: Vec<_> = (0..140).collect();
    let query = LevelViewQuery {
        level: 1,
        coordinate_window: CoordinateWindow { start: 0, end: 139 },
        as_of: 139,
        version: C2VersionTuple::auto_pairing(),
    };
    let material = || LevelViewMaterial {
        projection: ProjectionMaterial::ExactThree(&projection),
        move_blocks: &blocks,
        lower_legs: &legs,
        hist: &hist,
        dif: &dif,
        close_src: &close_src,
    };
    let mut store = ConfirmCursorStore::default();
    assemble_level_view_resident(
        C2LevelViewConfig { enabled: true },
        query,
        material(),
        Some(ConfirmResidence {
            store: &mut store,
            stable_lower_len: legs.len(),
            structure_generation: 1,
        }),
    )
    .unwrap();
    store.poison_for_test(ConfirmState::TerminalFalse);

    let forced_cold = assemble_level_view_resident(
        C2LevelViewConfig { enabled: true },
        query,
        material(),
        None,
    )
    .unwrap();
    assert_eq!(
        forced_cold.pair_confirmations[0].state,
        ConfirmState::Confirmed(139)
    );
    assert!(matches!(
        forced_cold.moves[0].completion,
        CompletionStatus::Completed { .. }
    ));
    assert!(store
        .cursors
        .values()
        .all(|cursor| cursor.state == ConfirmState::TerminalFalse));
}

/// 票 #427 左端恒等（裁定 #402 题二「维持工程桥」的等价性前提）：trend 确认分支把
/// `interval_b` 收束到 `[c_start, t*]`——**只截右端**。本用例跨两个独立公开面交叉核对：
/// `provide_divergence_pairs` 给出的 `pair.seg_c` 与 `provide_nest_candidate_events`
/// 给出的 `event.interval_b` 左端必须逐一相等。
///
/// 非空转证明：同一夹具下**右端确实改变**，故「左端相等」不是「两端都没动」的顺带结论。
#[test]
fn trend_confirm_truncation_keeps_seg_c_left_anchor() {
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows_carried_only(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    // 与 nest_lifecycle T5 同款两块布局：interval_a 需要 leave→retest 块对
    // （`structural_pair_span` 要求两块均 Completed），单块下 trend 事件不产出。
    let blocks = [
        MoveBlock {
            start_center: 0,
            end_center: 2,
            kind: MoveKind::Trend,
            dir: Some(Direction::Up),
            status: MoveStatus::Completed,
        },
        MoveBlock {
            start_center: 1,
            end_center: 2,
            kind: MoveKind::Consolidation,
            dir: None,
            status: MoveStatus::Completed,
        },
    ];
    let mut hist = vec![0.0; 140];
    hist[80..110].fill(2.0);
    hist[120..140].fill(0.1);
    let mut dif = vec![0.0; 140];
    dif[80..=100].fill(-5.0);
    dif[101..140].fill(1.0);
    let close_src: Vec<_> = (0..140).collect();
    let query = LevelViewQuery {
        level: 1,
        coordinate_window: CoordinateWindow { start: 0, end: 139 },
        as_of: 139,
        version: C2VersionTuple::auto_pairing(),
    };
    let view = assemble_level_view(
        C2LevelViewConfig { enabled: true },
        query,
        LevelViewMaterial {
            projection: ProjectionMaterial::ExactThree(&projection),
            move_blocks: &blocks,
            lower_legs: &legs,
            hist: &hist,
            dif: &dif,
            close_src: &close_src,
        },
    )
    .unwrap();
    let pairs = provide_divergence_pairs(1, &projection, &blocks, &legs, 139);
    let events = provide_nest_candidate_events(
        1,
        &projection,
        &blocks,
        &legs,
        &view,
        &hist,
        &dif,
        &close_src,
    );
    let trend: Vec<_> = events
        .iter()
        .filter(|event| event.kind == NestDivergenceKind::Trend)
        .collect();
    assert_eq!(trend.len(), 1, "夹具产单只 trend 事件");
    assert_eq!(pairs.len(), 1, "同夹具产单只 pair");
    assert!(trend[0].divergence_confirmed, "前提：走的是确认收束分支");
    assert_eq!(
        trend[0].interval_b.0, pairs[0].seg_c.0,
        "左端恒等（票 #427 加锁的前提）"
    );
    // 非空转：同夹具下右端**确实改变**（实测 129 → 139，t* 晚于 seg_c 右端 ⟹ 本例是
    // 外扩不是截短——裁定 #402 题二的「收束」措辞只在 t* ≤ seg_c.1 时成立；桥判同不看
    // 右端方向（`bridge_identity` 三形态口径），故等价性结论不受影响）。
    assert_ne!(
        trend[0].interval_b.1, pairs[0].seg_c.1,
        "右端确实改变 ⟹ 「左端相等」不是「两端都没动」的顺带结论"
    );

    // 第二臂：**未确认分支**（as_of=129 截断回试腿 ⟹ c 内无三买 ⟹ 全合取不成立）。
    // 两个分支各写一次 `interval_b`，左端恒等须两边都成立——只测确认分支会留下
    // 未确认分支的静默失效面（变异实测 M10 曾无干净测试捕获）。
    let query_pending = LevelViewQuery {
        level: 1,
        coordinate_window: CoordinateWindow { start: 0, end: 139 },
        as_of: 129,
        version: C2VersionTuple::auto_pairing(),
    };
    let view_pending = assemble_level_view(
        C2LevelViewConfig { enabled: true },
        query_pending,
        LevelViewMaterial {
            projection: ProjectionMaterial::ExactThree(&projection),
            move_blocks: &blocks,
            lower_legs: &legs,
            hist: &hist,
            dif: &dif,
            close_src: &close_src,
        },
    )
    .unwrap();
    let pairs_pending = provide_divergence_pairs(1, &projection, &blocks, &legs, 129);
    let events_pending = provide_nest_candidate_events(
        1,
        &projection,
        &blocks,
        &legs,
        &view_pending,
        &hist,
        &dif,
        &close_src,
    );
    let trend_pending: Vec<_> = events_pending
        .iter()
        .filter(|event| event.kind == NestDivergenceKind::Trend)
        .collect();
    assert_eq!(trend_pending.len(), 1, "未确认分支同样产单只 trend 事件");
    assert!(
        !trend_pending[0].divergence_confirmed,
        "前提：走的是未确认分支（保持全离开段结构坐标）"
    );
    assert_eq!(pairs_pending.len(), 1);
    assert_eq!(
        trend_pending[0].interval_b.0, pairs_pending[0].seg_c.0,
        "未确认分支左端同样恒等"
    );
}
